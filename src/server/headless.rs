//! Headless server mode — runs the herdr event loop without a real terminal.
//!
//! The server:
//! - Does not enter raw mode or read stdin
//! - Creates and listens on both `herdr.sock` (existing JSON API) and
//!   `herdr-client.sock` (new binary protocol)
//! - Initializes AppState and all PTYs from session restore or fresh state
//! - Runs the main event loop (drain events, drain API requests, scheduled tasks)
//! - Renders to a virtual ratatui Buffer in memory
//! - Accepts client connections on the client socket
//! - Streams frames to connected clients after each render
//! - Routes client input events through the existing input pipeline
//! - Continues running after client disconnect
//! - Handles stale socket cleanup, explicit server stop, minimum terminal size,
//!   and pane spawn failure during restore

use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossterm::event::{KeyModifiers, MouseEventKind};
use interprocess::local_socket::traits::Listener as _;
#[cfg(windows)]
use interprocess::local_socket::traits::Stream as _;
#[cfg(unix)]
use interprocess::local_socket::ListenerNonblockingMode;
use ratatui::layout::Rect;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use base64::Engine;
use bytes::Bytes;

use crate::api;
use crate::app;
use crate::config;
use crate::events::AppEvent;
use crate::ipc::{
    bind_local_listener, remove_socket_file_if_owned, socket_file_identity, LocalListener,
    SocketFileIdentity,
};
use crate::protocol::{
    self, AttachScrollDirection, AttachScrollSource, FrameData, ServerMessage, MAX_FRAME_SIZE,
    MAX_GRAPHICS_FRAME_SIZE,
};
#[cfg(unix)]
use crate::server::client_accept::{
    accept_pending_client_connections, reject_pending_client_connections,
};
use crate::server::client_transport::ServerEvent;
use crate::server::clients::{
    events_include_interaction, latest_app_client, render_targets, terminal_stream_client_ids,
    ClientConnection, ClientConnectionMode, DeferredRender,
};
use crate::server::keybindings::{app_keybindings, apply_keybindings};
use crate::server::notifications::{
    should_forward_toast_to_clients, toast_message_from_state_change, toast_notify_kind,
};
use crate::server::socket_paths::{
    client_socket_path, prepare_socket_path, restrict_socket_permissions,
};
use crate::server::terminal_attach::paste_payload_for_runtime;

mod pane_graphics;

#[cfg(test)]
use pane_graphics::frame_pane_graphics_for_client;
use pane_graphics::RetainedGraphicsOutcome;

#[cfg(test)]
use crate::protocol::RenderEncoding;
#[cfg(test)]
use crate::server::client_transport::ClientWriter;
#[cfg(test)]
use std::fs;

const LIVE_HANDOFF_RESPONSE_WRITE_TIMEOUT: Duration = Duration::from_secs(6);

fn wait_for_live_handoff_response_write(
    response_write_complete: Option<std::sync::mpsc::Receiver<()>>,
) {
    let Some(response_write_complete) = response_write_complete else {
        return;
    };

    match response_write_complete.recv_timeout(LIVE_HANDOFF_RESPONSE_WRITE_TIMEOUT) {
        Ok(()) => {}
        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
            warn!("timed out waiting for live handoff response write; old server exiting");
        }
        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
            warn!("live handoff response writer disconnected; old server exiting");
        }
    }
}

fn sound_notify_message(sound: crate::sound::Sound) -> &'static str {
    match sound {
        crate::sound::Sound::Done => "agent done",
        crate::sound::Sound::Request => "agent attention",
    }
}

fn notification_show_response_shown(response: &str) -> bool {
    let Ok(response) = serde_json::from_str::<api::schema::SuccessResponse>(response) else {
        return false;
    };
    matches!(
        response.result,
        api::schema::ResponseResult::NotificationShow { shown: true, .. }
    )
}

fn non_empty_body(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

// ---------------------------------------------------------------------------
// Loop event enum for the headless server event loop
// ---------------------------------------------------------------------------

/// Events that the headless server event loop can process.
enum LoopEvent {
    Timer,
    Internal(AppEvent),
    Api(Box<api::ApiRequestMessage>),
    ServerEvent(ServerEvent),
    RenderRequested,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
enum RenderImpact {
    #[default]
    None,
    Graphics,
    Full,
}

impl RenderImpact {
    fn merge(&mut self, other: Self) {
        *self = (*self).max(other);
    }
}

/// Whether this request can open a surface a person opened — one that belongs
/// to a single display rather than to the session.
///
/// Kept as an explicit list rather than "run every API request in the focused
/// display's view": a session instruction such as focusing a pane must stay
/// session-wide, and scoping it to one display is how a parked display ends up
/// in a mode that swallows what its user types (TP-SUR-BROADCAST-01).
fn opens_a_person_surface(method: &api::schema::Method) -> bool {
    matches!(method, api::schema::Method::PluginPaneOpen(_))
}

fn record_render_impact(source: &'static str, impact: RenderImpact) {
    let event = match (source, impact) {
        ("api_requests", RenderImpact::Graphics) => "graphics_render_cause.api_requests",
        ("api_requests", RenderImpact::Full) => "full_render_cause.api_requests",
        ("server_events", RenderImpact::Graphics) => "graphics_render_cause.server_events",
        ("server_events", RenderImpact::Full) => "full_render_cause.server_events",
        _ => return,
    };
    crate::render_prof::event(event);
}

fn rect_fits_frame(rect: Rect, frame: &FrameData) -> bool {
    rect.x.saturating_add(rect.width) <= frame.width
        && rect.y.saturating_add(rect.height) <= frame.height
}

fn apply_terminal_dirty_patch(
    frame: &mut FrameData,
    area: Rect,
    patch: crate::pane::TerminalDirtyPatch,
) -> bool {
    if !rect_fits_frame(area, frame) {
        return false;
    }
    let width = usize::from(frame.width);
    for (local_y, row_cells) in patch.rows {
        if local_y >= area.height || row_cells.len() != usize::from(area.width) {
            return false;
        }
        let frame_y = area.y + local_y;
        let start = usize::from(frame_y) * width + usize::from(area.x);
        let end = start + usize::from(area.width);
        if end > frame.cells.len() {
            return false;
        }
        frame.cells[start..end].clone_from_slice(&row_cells);
    }
    true
}

fn dirty_patch_intersects_hyperlinks(
    frame: &FrameData,
    area: Rect,
    patch: &crate::pane::TerminalDirtyPatch,
) -> bool {
    if frame.hyperlinks.is_empty() || !rect_fits_frame(area, frame) {
        return false;
    }
    let width = usize::from(frame.width);
    for (local_y, _) in &patch.rows {
        if *local_y >= area.height {
            return true;
        }
        let frame_y = area.y + *local_y;
        let start = usize::from(frame_y) * width + usize::from(area.x);
        let end = start + usize::from(area.width);
        if end > frame.cells.len() {
            return true;
        }
        if frame.cells[start..end]
            .iter()
            .any(|cell| cell.hyperlink.is_some())
        {
            return true;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default shared runtime size (columns, rows) when no clients are attached.
const MIN_COLS: u16 = 80;
const MIN_ROWS: u16 = 24;

/// Timeout for in-flight API requests during shutdown.
#[allow(dead_code)]
const SHUTDOWN_API_TIMEOUT: Duration = Duration::from_secs(5);

/// How often the idle headless loop wakes to poll the local listener for new
/// client connections.
///
/// The listener is non-blocking and not integrated into `tokio::select!`, so
/// a low-frequency wake is required to notice new thin-client attaches while
/// otherwise idle. Keep this much slower than the old resize-poll cadence to
/// avoid reintroducing the idle CPU spin.
const CLIENT_ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(250);

// ---------------------------------------------------------------------------
// Headless server
// ---------------------------------------------------------------------------

/// The headless server — runs the herdr event loop without a real terminal.
pub struct HeadlessServer {
    app: app::App,
    #[cfg(unix)]
    api_tx: Option<api::ApiRequestSender>,
    // Kept on every platform so dropping HeadlessServer owns API server shutdown.
    #[cfg_attr(windows, allow(dead_code))]
    api_server: Option<api::ServerHandle>,
    #[cfg(unix)]
    client_listener: LocalListener,
    client_socket_path: PathBuf,
    client_socket_identity: SocketFileIdentity,
    clients: HashMap<u64, ClientConnection>,
    #[cfg(unix)]
    next_client_id: u64,
    /// The client currently driving the shared pane runtime size, theme, and input keybindings.
    foreground_client_id: Option<u64>,
    /// Which client the shared `view` currently describes, and at what size.
    ///
    /// Hit geometry has to belong to the client whose pointer event is being
    /// routed, but recomputing it per event would put a layout pass on the
    /// serial input loop for every pointer motion. Tracking the owner makes
    /// the recompute happen only when the geometry actually belongs to
    /// someone else.
    view_owner: Option<(u64, (u16, u16))>,
    /// Counts geometry recomputes driven by pointer input, so a test can pin
    /// that the skip actually skips.
    #[cfg(test)]
    view_recomputes_for_input: usize,
    /// Counts virtual frames computed while no client is attached, so a test
    /// can pin that a watcherless tick stops paying for a frame nobody sees.
    #[cfg(test)]
    watcherless_virtual_frames: usize,
    /// Server-owned keybindings, restored when foreground clients use server mode.
    server_keybindings: crate::config::LiveKeybindConfig,
    /// Full server config warning shown to clients that use server keybindings.
    server_config_diagnostic: Option<String>,
    /// Server config warning with keybinding diagnostics removed for local-keybinding clients.
    server_config_diagnostic_without_keybindings: Option<String>,
    /// Writable direct attach owner per terminal id string.
    terminal_attach_owners: HashMap<String, u64>,
    /// Monotonic activity counter used to pick the most recently active client.
    next_activity_stamp: u64,
    /// Shared pane runtime size derived from the foreground client,
    /// or MIN_COLS × MIN_ROWS when no clients are connected.
    effective_size: (u16, u16),
    /// Flag set when shutdown is initiated.
    shutting_down: bool,
    /// Flag set while exporting live PTYs to a replacement server.
    handoff_in_progress: bool,
    /// Imported panes get one app-safe resize nudge after the first client attaches.
    #[cfg(unix)]
    pending_handoff_repaint_nudge: bool,
    /// Flag set by Ctrl+C or `server stop` signal.
    should_quit: Arc<AtomicBool>,
    /// Channel for receiving server events from client connection threads.
    server_event_rx: mpsc::Receiver<ServerEvent>,
    /// Sender for server events (cloned for each client thread).
    server_event_tx: mpsc::Sender<ServerEvent>,
}

fn apply_terminal_attach_scroll(
    runtime: &crate::terminal::TerminalRuntime,
    source: AttachScrollSource,
    direction: AttachScrollDirection,
    lines: u16,
    column: Option<u16>,
    row: Option<u16>,
    modifiers: u8,
) -> Result<(), String> {
    let wheel_kind = match direction {
        AttachScrollDirection::Up => MouseEventKind::ScrollUp,
        AttachScrollDirection::Down => MouseEventKind::ScrollDown,
    };
    if let AttachScrollSource::PageKey { input } = source {
        let host_scroll = runtime
            .input_state()
            .is_some_and(crate::pane::InputState::plain_page_keys_use_host_scrollback);
        if host_scroll {
            match direction {
                AttachScrollDirection::Up => runtime.scroll_up(lines.max(1) as usize),
                AttachScrollDirection::Down => runtime.scroll_down(lines.max(1) as usize),
            }
            return Ok(());
        }
        return apply_terminal_attach_input(runtime, input);
    }

    match runtime.wheel_routing() {
        Some(crate::pane::WheelRouting::MouseReport) => {
            runtime.scroll_reset();
            let column = column.unwrap_or(0);
            let row = row.unwrap_or(0);
            let Some(bytes) = runtime.encode_mouse_wheel(
                wheel_kind,
                column,
                row,
                KeyModifiers::from_bits_truncate(modifiers),
            ) else {
                return Err(format!(
                    "failed to encode terminal attach mouse wheel event: {wheel_kind:?}"
                ));
            };
            runtime
                .try_send_bytes(Bytes::from(bytes))
                .map_err(|err| format!("terminal attach mouse wheel input failed: {err}"))?;
        }
        Some(crate::pane::WheelRouting::AlternateScroll) => {
            runtime.scroll_reset();
            let Some(bytes) = runtime.encode_alternate_scroll(wheel_kind) else {
                return Ok(());
            };
            runtime
                .try_send_bytes(Bytes::from(bytes))
                .map_err(|err| format!("terminal attach alternate scroll input failed: {err}"))?;
        }
        Some(crate::pane::WheelRouting::HostScroll) | None => match direction {
            AttachScrollDirection::Up => runtime.scroll_up(lines.max(1) as usize),
            AttachScrollDirection::Down => runtime.scroll_down(lines.max(1) as usize),
        },
    }
    Ok(())
}

fn apply_terminal_attach_input(
    runtime: &crate::terminal::TerminalRuntime,
    data: Vec<u8>,
) -> Result<(), String> {
    runtime.scroll_reset();
    runtime
        .try_send_bytes(Bytes::from(data))
        .map_err(|err| format!("terminal attach input failed: {err}"))
}

#[cfg(windows)]
fn spawn_windows_client_accept_thread(
    listener: LocalListener,
    should_quit: Arc<AtomicBool>,
    server_event_tx: mpsc::Sender<ServerEvent>,
) {
    std::thread::spawn(move || {
        let mut next_client_id = 1_u64;
        while !should_quit.load(Ordering::Acquire) {
            let stream = match listener.accept() {
                Ok(stream) => stream,
                Err(err) => {
                    if should_quit.load(Ordering::Acquire) {
                        break;
                    }
                    error!(err = %err, "client listener accept failed");
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };

            let client_id = next_client_id;
            next_client_id = next_client_id.saturating_add(1);

            if let Err(err) = stream.set_nonblocking(true) {
                warn!(err = %err, "failed to set client stream nonblocking");
                continue;
            }

            let should_quit = should_quit.clone();
            let server_event_tx = server_event_tx.clone();
            std::thread::spawn(move || {
                if let Err(err) = crate::server::client_transport::handle_client_handshake(
                    stream,
                    client_id,
                    &server_event_tx,
                    &should_quit,
                ) {
                    debug!(client_id, err = %err, "client handshake failed");
                }
            });
        }
    });
}

impl HeadlessServer {
    /// Creates and starts the headless server.
    ///
    /// This:
    /// 1. Prepares the client socket path (cleans up stale sockets)
    /// 2. Binds the client socket listener
    /// 3. Returns the server ready to run
    pub fn new(
        app: app::App,
        config_diagnostics: &[String],
        api_tx: Option<api::ApiRequestSender>,
        api_server: Option<api::ServerHandle>,
    ) -> io::Result<Self> {
        let client_path = client_socket_path();
        prepare_socket_path(&client_path)?;

        let listener = bind_local_listener(&client_path)?;
        restrict_socket_permissions(&client_path)?;
        let client_socket_identity = socket_file_identity(&client_path)?;
        info!(path = %client_path.display(), "client protocol socket listening");

        // Set non-blocking on Unix so we can poll it from the event loop.
        #[cfg(unix)]
        listener.set_nonblocking(ListenerNonblockingMode::Accept)?;

        let should_quit = Arc::new(AtomicBool::new(false));

        // Channel for server events from client threads.
        let (server_event_tx, server_event_rx) = mpsc::channel(64);
        #[cfg(windows)]
        spawn_windows_client_accept_thread(listener, should_quit.clone(), server_event_tx.clone());

        let server_keybindings = app_keybindings(&app);
        let (server_config_diagnostic, server_config_diagnostic_without_keybindings) =
            server_config_diagnostic_summaries(config_diagnostics);
        #[cfg(not(unix))]
        let _ = api_tx;
        Ok(Self {
            app,
            #[cfg(unix)]
            api_tx,
            api_server,
            #[cfg(unix)]
            client_listener: listener,
            client_socket_path: client_path,
            client_socket_identity,
            clients: HashMap::new(),
            #[cfg(unix)]
            next_client_id: 1,
            foreground_client_id: None,
            view_owner: None,
            #[cfg(test)]
            view_recomputes_for_input: 0,
            #[cfg(test)]
            watcherless_virtual_frames: 0,
            server_keybindings,
            server_config_diagnostic,
            server_config_diagnostic_without_keybindings,
            terminal_attach_owners: HashMap::new(),
            next_activity_stamp: 1,
            effective_size: (MIN_COLS, MIN_ROWS),
            shutting_down: false,
            handoff_in_progress: false,
            #[cfg(unix)]
            pending_handoff_repaint_nudge: false,
            should_quit,
            server_event_rx,
            server_event_tx,
        })
    }

    /// Runs the headless server event loop until shutdown.
    ///
    /// This is the server's main loop — analogous to `App::run()` but without
    /// a real terminal. It:
    /// - Drains internal events (pane death, state changes)
    /// - Drains API requests (from the JSON socket)
    /// - Accepts new client connections
    /// - Reads client messages and routes input
    /// - Handles scheduled tasks (resize poll, animation, session save, etc.)
    /// - Renders virtually and streams frames to clients
    pub async fn run(&mut self) -> io::Result<()> {
        crate::logging::startup("server");

        // Register SIGINT handler for graceful shutdown.
        let should_quit = self.should_quit.clone();
        let quit_notify = self.server_event_tx.clone();
        ctrlc_handler(should_quit, quit_notify);

        // No input_rx needed — server doesn't read stdin.
        // We use None for input_rx so the event loop doesn't try to read from stdin.
        self.app.input_rx = None;

        let mut needs_render = true;
        let mut needs_full_render = true;
        let mut needs_graphics_render = false;

        loop {
            crate::render_prof::event("loop.tick");
            crate::render_prof::flush_if_due();
            self.app.reap_finished_custom_commands();

            // If shutdown has been initiated, complete it and exit.
            if self.shutting_down {
                self.complete_shutdown()?;
                break;
            }

            // Check if we should start shutting down.
            if self.app.state.should_quit || self.should_quit.load(Ordering::Acquire) {
                self.initiate_shutdown();
                continue;
            }

            // 1. Check render_dirty flag from PTY reader tasks.
            if self.app.render_dirty.load(Ordering::Acquire) {
                needs_render = true;
                crate::render_prof::event("render.request.pty_dirty");
            }
            let terminal_title_changed = self.app.sync_terminal_titles();
            if terminal_title_changed && self.app.terminal_title_sidebar_configured() {
                needs_render = true;
                needs_full_render = true;
                crate::render_prof::event("full_render_cause.terminal_title");
            }

            // 2. Drain a bounded internal-event batch. API handlers perform an
            // exhaustive forwarding-aware drain before reading pane/runtime state.
            if self.drain_internal_events_with_forwarding() {
                needs_render = true;
                needs_full_render = true;
                needs_graphics_render = false;
                crate::render_prof::event("full_render_cause.internal_events");
            }
            if self.app.expire_due_metadata(Instant::now()) {
                needs_render = true;
                needs_full_render = true;
                crate::render_prof::event("full_render_cause.metadata_expiry");
            }

            // 3. Drain API requests.
            if self.pane_graphics_runtime_active() {
                let api_impact = self.drain_api_requests_with_render_impact();
                record_render_impact("api_requests", api_impact);
                match api_impact {
                    RenderImpact::None => {}
                    RenderImpact::Graphics => {
                        needs_render = true;
                        needs_graphics_render = true;
                    }
                    RenderImpact::Full => {
                        needs_render = true;
                        needs_full_render = true;
                        needs_graphics_render = false;
                    }
                }
            } else if self.drain_api_requests_with_shutdown_check() {
                needs_render = true;
                needs_full_render = true;
                crate::render_prof::event("full_render_cause.api_requests");
            }

            self.app.sync_focus_events();
            self.app.sync_session_save_schedule();

            // 4. Accept new client connections.
            self.accept_client_connections()?;

            // 5. Drain server events from client threads.
            if self.pane_graphics_runtime_active() {
                let server_impact = self.drain_server_events_with_render_impact();
                record_render_impact("server_events", server_impact);
                match server_impact {
                    RenderImpact::None => {}
                    RenderImpact::Graphics => {
                        needs_render = true;
                        needs_graphics_render = true;
                    }
                    RenderImpact::Full => {
                        needs_render = true;
                        needs_full_render = true;
                        needs_graphics_render = false;
                    }
                }
            } else if self.drain_server_events() {
                needs_render = true;
                needs_full_render = true;
                crate::render_prof::event("full_render_cause.server_events");
            }

            // 6. Handle scheduled tasks.
            let now = Instant::now();
            if self.handle_scheduled_tasks_headless(now, needs_render) {
                needs_render = true;
                needs_full_render = true;
                needs_graphics_render = false;
                crate::render_prof::event("full_render_cause.scheduled_tasks");
            }

            if self.handle_deferred_requests_headless() {
                needs_render = true;
                needs_full_render = true;
                needs_graphics_render = false;
            }

            if latest_app_client(&self.clients).is_some() && self.app.ensure_default_workspace() {
                needs_render = true;
                needs_full_render = true;
                needs_graphics_render = false;
                crate::render_prof::event("full_render_cause.default_workspace");
            }

            self.cancel_inactive_pane_graphics_streams();

            self.drain_client_config_reload_request();
            self.stream_host_mouse_capture_mode();
            self.stream_host_keyboard_enhancement_flags();

            self.app.sync_headless_animation_timer(now);

            // 7. Render virtually and stream frames.
            if needs_render && self.app.can_render_now(now) {
                crate::render_prof::event("render.attempt");
                let pty_dirty = self.app.render_dirty.swap(false, Ordering::AcqRel);
                if pty_dirty {
                    crate::render_prof::event("render.attempt.pty_dirty");
                }
                if needs_full_render {
                    crate::render_prof::event("retained_gate.needs_full_render");
                } else if !pty_dirty {
                    crate::render_prof::event("retained_gate.not_pty_dirty");
                }
                let mut deferred_graphics = false;
                let rendered_retained = if needs_graphics_render && !needs_full_render && !pty_dirty
                {
                    match self.render_retained_graphics_update_and_stream() {
                        RetainedGraphicsOutcome::Sent => true,
                        RetainedGraphicsOutcome::Deferred => {
                            deferred_graphics = true;
                            false
                        }
                        RetainedGraphicsOutcome::Fallback => false,
                    }
                } else {
                    pty_dirty && !needs_full_render && self.render_retained_pty_update_and_stream()
                };
                if deferred_graphics {
                    needs_render = false;
                    continue;
                }
                if !rendered_retained {
                    crate::render_prof::event("full_render.invoke");
                    self.render_and_stream();
                }
                self.app.last_render_at = Some(now);
                needs_render = false;
                needs_full_render = false;
                needs_graphics_render = false;
                continue;
            }

            // 8. Wait for next event.
            let next_deadline = self
                .app
                .next_headless_loop_deadline_with_git_refresh(
                    now,
                    needs_render,
                    self.has_app_client(),
                )
                .map(|deadline| deadline.min(now + CLIENT_ACCEPT_POLL_INTERVAL))
                .or(Some(now + CLIENT_ACCEPT_POLL_INTERVAL));
            let event = {
                tokio::select! {
                    maybe_api = self.app.api_rx.recv() => match maybe_api {
                        Some(msg) => LoopEvent::Api(Box::new(msg)),
                        None => LoopEvent::Timer,
                    },
                    maybe_ev = self.app.event_rx.recv() => match maybe_ev {
                        Some(ev) => LoopEvent::Internal(ev),
                        None => LoopEvent::Timer,
                    },
                    maybe_server_ev = self.server_event_rx.recv() => match maybe_server_ev {
                        Some(ev) => LoopEvent::ServerEvent(ev),
                        None => LoopEvent::Timer,
                    },
                    _ = sleep_until_or_pending(next_deadline) => LoopEvent::Timer,
                    _ = self.app.render_notify.notified() => LoopEvent::RenderRequested,
                }
            };

            match event {
                LoopEvent::Timer => {}
                LoopEvent::Internal(ev) => {
                    if self.handle_internal_event_with_forwarding(ev) {
                        needs_render = true;
                        needs_full_render = true;
                        needs_graphics_render = false;
                    }
                }
                LoopEvent::Api(msg) => {
                    if self.pane_graphics_runtime_active() {
                        let impact = self.handle_api_request_with_render_impact(*msg);
                        record_render_impact("api_requests", impact);
                        match impact {
                            RenderImpact::None => {}
                            RenderImpact::Graphics => {
                                needs_render = true;
                                needs_graphics_render = true;
                            }
                            RenderImpact::Full => {
                                needs_render = true;
                                needs_full_render = true;
                                needs_graphics_render = false;
                            }
                        }
                    } else if self.handle_api_request_with_shutdown_check(*msg) {
                        needs_render = true;
                        needs_full_render = true;
                    }
                }
                LoopEvent::ServerEvent(ev) => {
                    if self.pane_graphics_runtime_active() {
                        let impact = self.handle_server_event_with_render_impact(ev);
                        record_render_impact("server_events", impact);
                        match impact {
                            RenderImpact::None => {}
                            RenderImpact::Graphics => {
                                needs_render = true;
                                needs_graphics_render = true;
                            }
                            RenderImpact::Full => {
                                needs_render = true;
                                needs_full_render = true;
                                needs_graphics_render = false;
                            }
                        }
                    } else if self.handle_server_event(ev) {
                        needs_render = true;
                        needs_full_render = true;
                    }
                }
                LoopEvent::RenderRequested => {
                    if self.app.render_dirty.load(Ordering::Acquire) {
                        needs_render = true;
                    }
                }
            }
        }

        // Save session on exit.
        if !self.app.no_session {
            self.app.save_session_now();
        }

        info!("headless server exiting");
        Ok(())
    }

    fn handle_deferred_requests_headless(&mut self) -> bool {
        let mut needs_render = false;

        if self.app.state.request_complete_onboarding {
            self.app.state.request_complete_onboarding = false;
            self.app.open_settings_from_onboarding();
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_onboarding");
        }

        if self.app.state.request_new_workspace {
            self.app.state.request_new_workspace = false;
            let response = self.headless_workspace_create("headless.workspace.create", None, None);
            if let Err(error) = response {
                error!(
                    code = %error.code,
                    message = %error.message,
                    "failed to create workspace"
                );
            }
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_new_workspace");
        }

        if self.app.state.request_new_tab {
            self.app.state.request_new_tab = false;
            let label = self.app.state.requested_new_tab_name.take();
            let response = self.headless_tab_create("headless.tab.create", label);
            if let Err(error) = response {
                error!(
                    code = %error.code,
                    message = %error.message,
                    "failed to create tab"
                );
            }
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_new_tab");
        }

        if let Some(ws_idx) = self.app.state.request_new_linked_worktree.take() {
            self.app.open_new_linked_worktree_dialog(ws_idx);
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_worktree_dialog");
        }

        if let Some(ws_idx) = self.app.state.request_open_existing_worktree.take() {
            self.app.open_existing_worktree_dialog(ws_idx);
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_worktree_dialog");
        }

        if let Some(cwd) = self.app.state.request_new_workspace_cwd.take() {
            let response = self.headless_workspace_create(
                "headless.workspace.create_cwd",
                Some(cwd.display().to_string()),
                None,
            );
            if let Err(error) = response {
                error!(
                    code = %error.code,
                    message = %error.message,
                    "failed to create workspace at requested cwd"
                );
                self.app.state.mode = app::Mode::Navigate;
            }
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_workspace_cwd");
        }

        if self.app.handle_project_chat_tab_request() {
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_project_chat_tab");
        }

        // Preview show acts on an external browser window: no re-render.
        let _ = self.app.handle_preview_show_request();

        if let Some(ws_idx) = self.app.state.request_remove_linked_worktree.take() {
            self.app.open_remove_linked_worktree_confirmation(ws_idx);
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_worktree_dialog");
        }

        if self.app.state.request_submit_worktree_create {
            self.app.state.request_submit_worktree_create = false;
            self.app.submit_worktree_create_via_api();
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_worktree_submit");
        }

        if self.app.state.request_submit_worktree_open {
            self.app.state.request_submit_worktree_open = false;
            self.app.submit_worktree_open_via_api();
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_worktree_submit");
        }

        if self.app.state.request_submit_worktree_remove {
            self.app.state.request_submit_worktree_remove = false;
            self.app.submit_worktree_remove_via_api();
            needs_render = true;
            crate::render_prof::event("full_render_cause.deferred_worktree_submit");
        }

        if self.app.state.request_reload_config {
            self.app.state.request_reload_config = false;
            self.reload_server_config(true);
            needs_render = true;
            crate::render_prof::event("full_render_cause.config_reload");
        }

        needs_render
    }

    fn headless_workspace_create(
        &mut self,
        id: &'static str,
        cwd: Option<String>,
        label: Option<String>,
    ) -> Result<(), api::schema::ErrorBody> {
        self.dispatch_headless_runtime_mutation(
            id,
            api::schema::Method::WorkspaceCreate(api::schema::WorkspaceCreateParams {
                cwd,
                focus: true,
                label,
                env: Default::default(),
            }),
        )
    }

    fn headless_tab_create(
        &mut self,
        id: &'static str,
        label: Option<String>,
    ) -> Result<(), api::schema::ErrorBody> {
        self.dispatch_headless_runtime_mutation(
            id,
            api::schema::Method::TabCreate(api::schema::TabCreateParams {
                workspace_id: None,
                cwd: None,
                focus: true,
                label,
                env: Default::default(),
            }),
        )
    }

    fn dispatch_headless_runtime_mutation(
        &mut self,
        id: &'static str,
        method: api::schema::Method,
    ) -> Result<(), api::schema::ErrorBody> {
        let (respond_to, response_rx) = std::sync::mpsc::channel();
        self.handle_api_request_with_shutdown_check_inner(
            api::ApiRequestMessage {
                request: api::schema::Request {
                    id: id.to_string(),
                    method,
                },
                respond_to,
                response_write_complete: None,
            },
            true,
        );
        match response_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(response) => serde_json::from_str::<api::schema::ErrorResponse>(&response)
                .map(|response| Err(response.error))
                .unwrap_or(Ok(())),
            Err(err) => Err(api::schema::ErrorBody {
                code: "internal_error".into(),
                message: format!("headless runtime mutation response failed: {err}"),
            }),
        }
    }

    fn allocate_activity_stamp(&mut self) -> u64 {
        let stamp = self.next_activity_stamp;
        self.next_activity_stamp = self.next_activity_stamp.saturating_add(1);
        stamp
    }

    /// Reconcile geometry after a change in session size: a display attaching,
    /// detaching, or reporting a new terminal size. Background tabs are swept
    /// here, which is the trigger the sweep was written for.
    fn resize_shared_runtime_to_effective_size(&mut self) {
        self.resize_shared_runtime_to_effective_size_with_pending_agent_resumes(
            true,
            crate::ui::BackgroundTabSweep::Reconcile,
        );
    }

    /// Reconcile geometry before routing input to the display that just became
    /// foreground, or after its theme changed.
    ///
    /// Neither is a change in session size. The foreground display's own tab
    /// still follows it, but sweeping background tabs here would rewrite every
    /// unwatched pane to whichever window was last typed in -- and reflow its
    /// whole scrollback -- on every alternation between two windows.
    /// TP-MCF-SIZE-04
    fn resize_shared_runtime_to_effective_size_before_input(&mut self) {
        self.resize_shared_runtime_to_effective_size_with_pending_agent_resumes(
            false,
            crate::ui::BackgroundTabSweep::Skip,
        );
    }

    fn resize_shared_runtime_to_effective_size_with_pending_agent_resumes(
        &mut self,
        start_pending_agent_resumes: bool,
        sweep: crate::ui::BackgroundTabSweep,
    ) {
        if self.foreground_client_id.is_none() {
            return;
        }
        let Some(client_id) = self.foreground_client_id else {
            return;
        };
        let Some(client) = self.clients.get(&client_id) else {
            return;
        };
        let (cols, rows) = self.effective_size;
        let area = Rect::new(0, 0, cols, rows);
        let cell_size = if self.app.state.kitty_graphics_enabled && client.cell_size.is_known() {
            client.cell_size
        } else {
            crate::kitty_graphics::HostCellSize::default()
        };
        match sweep {
            crate::ui::BackgroundTabSweep::Reconcile => crate::ui::compute_view_with_cell_size(
                &mut self.app.state,
                &self.app.terminal_runtimes,
                area,
                cell_size,
            ),
            crate::ui::BackgroundTabSweep::Skip => {
                crate::ui::compute_view_skipping_background_tabs(
                    &mut self.app.state,
                    &self.app.terminal_runtimes,
                    area,
                    cell_size,
                )
            }
        }

        // Shared runtime size changes affect pane wrapping and foreground-driven
        // rendering semantics. Force one fresh frame to every remaining client
        // even if the next rendered buffer compares equal to its cached frame.
        for client in self.clients.values_mut() {
            client.request_repaint();
        }
        if !start_pending_agent_resumes {
            self.app.pending_agent_resume_deadline = None;
            return;
        }
        let now = Instant::now();
        self.app.sync_pending_agent_resume_deadline(now);
        if self
            .app
            .start_pending_agent_resumes(self.app.pending_agent_resume_due(now))
        {
            for client in self.clients.values_mut() {
                client.request_repaint();
            }
        }
    }

    /// Publish one resolved host cell size to everything that decodes against
    /// it: host graphics placement and the file manager's image preview.
    ///
    /// These are deliberately written together. They drifted apart once
    /// already — the image preview kept a cell size of zero in server mode, so
    /// it derived no decode target and silently showed nothing — and a single
    /// assignment site is what stops that from recurring.
    fn publish_host_cell_size(&mut self, cell_size: crate::kitty_graphics::HostCellSize) {
        self.app.state.host_cell_size = cell_size;
        self.app.image_preview_cell_size = cell_size;
    }

    fn sync_foreground_client_state(&mut self) {
        let Some(client_id) = self.foreground_client_id else {
            self.effective_size = (MIN_COLS, MIN_ROWS);
            self.app.state.outer_terminal_focus = None;
            self.publish_host_cell_size(crate::kitty_graphics::HostCellSize::default());
            let server_keybindings = self.server_keybindings.clone();
            apply_keybindings(&mut self.app, &server_keybindings);
            self.sync_visible_server_config_diagnostic(false);
            return;
        };
        let Some(client) = self.clients.get(&client_id) else {
            self.foreground_client_id = None;
            self.effective_size = (MIN_COLS, MIN_ROWS);
            self.app.state.outer_terminal_focus = None;
            self.publish_host_cell_size(crate::kitty_graphics::HostCellSize::default());
            let server_keybindings = self.server_keybindings.clone();
            apply_keybindings(&mut self.app, &server_keybindings);
            self.sync_visible_server_config_diagnostic(false);
            return;
        };

        let terminal_size = client.terminal_size;
        let outer_terminal_focus = client.outer_terminal_focus;
        let host_cell_size = if self.app.state.kitty_graphics_enabled && client.cell_size.is_known()
        {
            client.cell_size
        } else {
            crate::kitty_graphics::HostCellSize::default()
        };
        let host_terminal_theme = client.host_terminal_theme;
        let host_terminal_appearance = client.host_terminal_appearance;
        let host_terminal_appearance_explicit = client.host_terminal_appearance_explicit;
        let uses_local_keybindings = client.keybindings.is_some();
        let keybindings = client
            .keybindings
            .as_deref()
            .unwrap_or(&self.server_keybindings)
            .clone();

        self.effective_size = terminal_size;
        self.app.state.outer_terminal_focus = outer_terminal_focus;
        self.publish_host_cell_size(host_cell_size);
        apply_keybindings(&mut self.app, &keybindings);
        self.sync_visible_server_config_diagnostic(uses_local_keybindings);
        if outer_terminal_focus == Some(true) {
            self.app.state.mark_active_tab_seen();
        }
        self.app.set_host_terminal_appearance_state(
            host_terminal_appearance,
            host_terminal_appearance_explicit,
        );
        self.app.set_host_terminal_theme(host_terminal_theme);
    }

    #[cfg(unix)]
    fn perform_live_handoff(
        &mut self,
        params: crate::api::schema::ServerLiveHandoffParams,
    ) -> io::Result<()> {
        info!("starting live handoff");
        let import_exe = params.import_exe.as_deref().map(std::path::PathBuf::from);
        let socket_path = crate::server::handoff::handoff_socket_path();
        let token = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        );
        let listener = match crate::server::handoff::bind_listener(&socket_path) {
            Ok(listener) => listener,
            Err(err) => {
                self.handoff_in_progress = false;
                return Err(err);
            }
        };

        let mut pane_by_terminal = HashMap::new();
        for ws in &self.app.state.workspaces {
            for tab in &ws.tabs {
                for (pane_id, pane) in &tab.panes {
                    pane_by_terminal.insert(pane.attached_terminal_id.clone(), pane_id.raw());
                }
            }
        }
        if pane_by_terminal.len() > crate::server::handoff::MAX_FDS_PER_HANDOFF {
            let _ = std::fs::remove_file(&socket_path);
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "live handoff supports at most {} panes in one update; close panes or restart herdr normally",
                    crate::server::handoff::MAX_FDS_PER_HANDOFF
                ),
            ));
        }

        self.handoff_in_progress = true;
        self.disconnect_all_clients_for_handoff();
        let _ = reject_pending_client_connections(&self.client_listener);

        let mut paused_terminal_ids = Vec::new();
        for terminal_id in pane_by_terminal.keys() {
            if let Some(runtime) = self.app.terminal_runtimes.get(terminal_id) {
                if let Err(err) = runtime.pause_handoff_reader(Duration::from_secs(2)) {
                    self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
                    return Err(err);
                }
                paused_terminal_ids.push(terminal_id.clone());
            }
        }

        let snapshot = crate::persist::capture(
            &self.app.state.workspaces,
            &self.app.state.terminals,
            &self.app.terminal_runtimes,
            self.app.state.active,
            self.app.state.selected,
            self.app.state.sidebar_width,
            &self.app.state.shell_presentation,
            self.app.state.sidebar_section_split,
            self.app.state.collapsed_space_keys.clone(),
            self.app.state.collapsed_project_keys.clone(),
            self.app.state.files_tab_snapshot(),
        );

        let mut handoff_entries = Vec::new();
        for (terminal_id, runtime) in self.app.terminal_runtimes.iter() {
            let Some(pane_id) = pane_by_terminal.get(terminal_id).copied() else {
                continue;
            };
            let mut handoff_runtime = runtime.handoff_runtime_state(pane_id);
            let has_agent_session = self
                .app
                .state
                .terminals
                .get(terminal_id)
                .is_some_and(|terminal| terminal.persisted_agent_session.is_some());
            if !has_agent_session {
                handoff_runtime.initial_history_ansi = runtime.handoff_history_ansi();
            }
            handoff_entries.push((terminal_id.clone(), handoff_runtime));
        }

        let panes = handoff_entries
            .iter()
            .map(|(_, runtime)| runtime.clone())
            .collect();
        let manifest = crate::server::handoff::manifest_for(
            snapshot,
            panes,
            params.expected_protocol,
            params.expected_version,
        );
        let mut import_child = match crate::server::handoff::spawn_handoff_import(
            import_exe.as_deref(),
            &socket_path,
            &token,
        ) {
            Ok(child) => child,
            Err(err) => {
                self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
                return Err(err);
            }
        };
        let child_pid = import_child.id();
        info!(pid = child_pid, socket = %socket_path.display(), "spawned handoff import server");

        let mut fds = Vec::new();
        let duplicate_result = (|| {
            for (terminal_id, _) in &handoff_entries {
                let Some(runtime) = self.app.terminal_runtimes.get(terminal_id) else {
                    continue;
                };
                fds.push(runtime.duplicate_handoff_fd()?);
            }
            Ok::<(), io::Error>(())
        })();
        if let Err(err) = duplicate_result {
            for fd in fds {
                let _ = unsafe { libc::close(fd) };
            }
            crate::server::handoff::cleanup_failed_import_child(&mut import_child);
            self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
            return Err(err);
        }

        let mut stream = match crate::server::handoff::accept_and_validate_on(
            listener,
            &socket_path,
            &token,
            &manifest,
        ) {
            Ok(stream) => stream,
            Err(err) => {
                for fd in fds {
                    let _ = unsafe { libc::close(fd) };
                }
                crate::server::handoff::cleanup_failed_import_child(&mut import_child);
                self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
                return Err(err);
            }
        };

        let send_result = crate::server::handoff::send_fds_and_wait_restored(&mut stream, &fds);
        for fd in fds {
            let _ = unsafe { libc::close(fd) };
        }
        if let Err(err) = send_result {
            crate::server::handoff::cleanup_failed_import_child(&mut import_child);
            self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
            return Err(err);
        }

        if let Some(api_server) = &self.api_server {
            let _ = api_server.remove_socket_file_if_owned();
        } else {
            let _ = std::fs::remove_file(crate::api::socket_path());
        }
        let _ = remove_socket_file_if_owned(&self.client_socket_path, &self.client_socket_identity);
        if let Err(err) = crate::server::handoff::wait_ready(&mut stream) {
            crate::server::handoff::cleanup_failed_import_child(&mut import_child);
            match self.wait_then_restore_public_sockets_after_failed_handoff() {
                Ok(()) => {
                    self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
                }
                Err(restore_err) => {
                    self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
                    return Err(io::Error::other(format!(
                        "handoff replacement server did not become ready: {err}; old server could not restore public sockets: {restore_err}"
                    )));
                }
            }
            return Err(io::Error::other(format!(
                "handoff replacement server did not become ready: {err}"
            )));
        }
        if let Err(err) = crate::server::handoff::report_committed(&mut stream) {
            crate::server::handoff::cleanup_failed_import_child(&mut import_child);
            match self.wait_then_restore_public_sockets_after_failed_handoff() {
                Ok(()) => {
                    self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
                }
                Err(restore_err) => {
                    self.rollback_handoff_before_commit(&socket_path, &paused_terminal_ids);
                    return Err(io::Error::other(format!(
                        "handoff replacement server was ready, but commit failed: {err}; old server could not restore public sockets: {restore_err}"
                    )));
                }
            }
            return Err(err);
        }

        for (terminal_id, runtime) in self.app.terminal_runtimes.drain_for_handoff() {
            if !pane_by_terminal.contains_key(&terminal_id) {
                continue;
            }
            debug!(terminal = %terminal_id, "preserving pane runtime for handoff");
            runtime.preserve_for_handoff();
        }
        crate::server::handoff::wait_owned_ack(&mut stream);

        Ok(())
    }

    fn finish_live_handoff_shutdown(&mut self) {
        self.shutting_down = true;
        self.app.state.should_quit = true;
        self.app.no_session = true;
        info!("live handoff completed; old server exiting");
    }

    #[cfg(not(unix))]
    fn perform_live_handoff(
        &mut self,
        _params: crate::api::schema::ServerLiveHandoffParams,
    ) -> io::Result<()> {
        Err(io::Error::other("live handoff is only supported on Unix"))
    }

    fn sync_visible_server_config_diagnostic(&mut self, uses_local_keybindings: bool) {
        let visible = if uses_local_keybindings {
            &self.server_config_diagnostic_without_keybindings
        } else {
            &self.server_config_diagnostic
        };
        if self.app.state.config_diagnostic == self.server_config_diagnostic
            || self.app.state.config_diagnostic == self.server_config_diagnostic_without_keybindings
        {
            self.app.state.config_diagnostic = visible.clone();
        }
    }

    #[cfg(unix)]
    fn restore_public_sockets_after_failed_handoff(&mut self) -> io::Result<()> {
        let api_tx = self
            .api_tx
            .clone()
            .ok_or_else(|| io::Error::other("cannot restore api socket without api sender"))?;
        let api_server = api::start_server(api_tx, self.app.event_hub.clone())?;

        let client_path = client_socket_path();
        prepare_socket_path(&client_path)?;
        let listener = bind_local_listener(&client_path)?;
        restrict_socket_permissions(&client_path)?;
        let client_socket_identity = socket_file_identity(&client_path)?;
        listener.set_nonblocking(ListenerNonblockingMode::Accept)?;

        self.api_server = Some(api_server);
        self.client_listener = listener;
        self.client_socket_path = client_path;
        self.client_socket_identity = client_socket_identity;
        Ok(())
    }

    #[cfg(unix)]
    fn wait_then_restore_public_sockets_after_failed_handoff(&mut self) -> io::Result<()> {
        let timeout = crate::server::handoff::COMMIT_TIMEOUT + Duration::from_secs(2);
        wait_for_old_public_sockets_to_close(timeout)?;
        self.restore_public_sockets_after_failed_handoff()
    }

    #[cfg(unix)]
    fn rollback_handoff_before_commit(
        &mut self,
        socket_path: &Path,
        paused_terminal_ids: &[crate::terminal::TerminalId],
    ) {
        for terminal_id in paused_terminal_ids {
            if let Some(runtime) = self.app.terminal_runtimes.get(terminal_id) {
                runtime.set_handoff_reader_paused(false);
            }
        }
        self.handoff_in_progress = false;
        let _ = std::fs::remove_file(socket_path);
    }

    #[cfg(unix)]
    fn nudge_handoff_panes_on_first_client_attach(&mut self) {
        if !self.pending_handoff_repaint_nudge {
            return;
        }
        self.pending_handoff_repaint_nudge = false;
        self.app
            .terminal_runtimes
            .nudge_child_redraw_after_handoff();
    }

    #[cfg(not(unix))]
    fn nudge_handoff_panes_on_first_client_attach(&mut self) {}

    fn reload_server_config(&mut self, notify_success: bool) -> crate::config::ConfigReloadReport {
        let server_keybindings = self.server_keybindings.clone();
        apply_keybindings(&mut self.app, &server_keybindings);
        let report = self.app.apply_config_from_disk(notify_success);
        self.app.take_config_reloaded_from_disk();
        self.server_keybindings = app_keybindings(&self.app);
        let (server_config_diagnostic, server_config_diagnostic_without_keybindings) =
            server_config_diagnostic_summaries(&report.diagnostics);
        self.server_config_diagnostic = server_config_diagnostic;
        self.server_config_diagnostic_without_keybindings =
            server_config_diagnostic_without_keybindings;
        self.sync_foreground_client_state();
        report
    }

    fn foreground_client_outer_focus(&self) -> Option<bool> {
        let client_id = self.foreground_client_id?;
        self.clients.get(&client_id)?.outer_terminal_focus
    }

    fn active_tab_suppresses_notifications(&self, is_active_tab: bool) -> bool {
        crate::app::actions::active_tab_suppresses_notifications(
            is_active_tab,
            self.foreground_client_outer_focus(),
        )
    }

    fn promote_client_to_foreground(&mut self, client_id: u64) -> bool {
        let stamp = self.allocate_activity_stamp();
        let Some(client) = self.clients.get_mut(&client_id) else {
            return false;
        };
        client.last_activity = stamp;

        let changed = self.foreground_client_id != Some(client_id);
        self.foreground_client_id = Some(client_id);
        self.sync_foreground_client_state();
        changed
    }

    fn promote_latest_remaining_client(&mut self) -> bool {
        let next_foreground = latest_app_client(&self.clients);
        let changed = next_foreground != self.foreground_client_id;
        self.foreground_client_id = next_foreground;
        self.sync_foreground_client_state();
        changed
    }

    fn app_client_count(&self) -> usize {
        self.clients
            .values()
            .filter(|client| client.is_full_app_client() && client.writer.is_some())
            .count()
    }

    fn has_app_client(&self) -> bool {
        self.app_client_count() > 0
    }

    fn remove_client(&mut self, client_id: u64) -> bool {
        let was_foreground = self.foreground_client_id == Some(client_id);
        self.app.clear_input_source(client_id);
        self.app.state.forget_client(client_id);
        // A departed display's workers hold a thread and a channel for a view
        // nobody will ask about again. TP-SUR-FM-03
        self.app.forget_display_workers(client_id);
        self.send_client_graphics_cleanup(client_id);
        let removed = self.clients.remove(&client_id);
        if let Some(removed) = removed {
            crate::server::clipboard_image::remove_files(removed.staged_clipboard_files);
            if let ClientConnectionMode::TerminalAttach { terminal_id } = removed.mode {
                self.terminal_attach_owners.remove(&terminal_id);
                if let Some(terminal_id) = self.terminal_id_by_string(&terminal_id) {
                    self.app
                        .state
                        .direct_attach_resize_locks
                        .remove(&terminal_id);
                }
            }
        }
        if was_foreground {
            self.promote_latest_remaining_client()
        } else {
            false
        }
    }

    fn client_removal_needs_shared_resize(&self, client_id: u64) -> bool {
        if self.foreground_client_id == Some(client_id) {
            return true;
        }
        matches!(
            self.clients.get(&client_id).map(|client| &client.mode),
            Some(
                ClientConnectionMode::TerminalAttach { .. }
                    | ClientConnectionMode::TerminalObserve { .. }
            )
        ) && self.foreground_client_id.is_some()
    }

    fn remove_client_and_resize_if_needed(&mut self, client_id: u64) {
        let needs_shared_resize = self.client_removal_needs_shared_resize(client_id);
        let foreground_changed = self.remove_client(client_id);
        if needs_shared_resize || foreground_changed {
            self.resize_shared_runtime_to_effective_size();
        }
    }

    fn send_client_graphics_cleanup(&mut self, client_id: u64) {
        let (writer, bytes) = match self.clients.get_mut(&client_id) {
            Some(client) => {
                let bytes = client.graphics_cache.clear_bytes();
                (client.writer.as_ref().cloned(), bytes)
            }
            None => return,
        };
        if bytes.is_empty() {
            return;
        }
        let Some(writer) = writer else {
            return;
        };
        let Ok(serialized) = Self::frame_server_message(&ServerMessage::Graphics { bytes }) else {
            return;
        };
        let _ = writer.control.send(serialized);
    }

    fn send_all_clients_graphics_cleanup(&mut self) {
        let client_ids = self.clients.keys().copied().collect::<Vec<_>>();
        for client_id in client_ids {
            self.send_client_graphics_cleanup(client_id);
        }
    }

    fn update_client_host_theme_from_events(
        &mut self,
        client_id: u64,
        events: &[crate::raw_input::RawInputEvent],
    ) -> bool {
        let Some(client) = self.clients.get_mut(&client_id) else {
            return false;
        };

        if !client.update_host_theme_from_events(events) {
            return false;
        }

        if self.foreground_client_id == Some(client_id) {
            let mut changed = self.app.set_host_terminal_appearance_state(
                client.host_terminal_appearance,
                client.host_terminal_appearance_explicit,
            );
            changed |= self.app.set_host_terminal_theme(client.host_terminal_theme);
            if changed {
                self.resize_shared_runtime_to_effective_size_before_input();
            }
            changed
        } else {
            false
        }
    }

    fn update_client_outer_focus_from_events(
        &mut self,
        client_id: u64,
        events: &[crate::raw_input::RawInputEvent],
    ) {
        let Some(client) = self.clients.get_mut(&client_id) else {
            return;
        };
        let Some(next_focus) = client.update_outer_focus_from_events(events) else {
            return;
        };
        if self.foreground_client_id == Some(client_id) {
            self.app.state.outer_terminal_focus = Some(next_focus);
        }
    }

    /// Accepts pending client connections from the non-blocking listener.
    #[cfg(unix)]
    fn accept_client_connections(&mut self) -> io::Result<()> {
        if self.handoff_in_progress {
            return reject_pending_client_connections(&self.client_listener);
        }
        accept_pending_client_connections(
            &self.client_listener,
            &mut self.next_client_id,
            &self.should_quit,
            &self.server_event_tx,
        )
    }

    /// Windows named-pipe clients can block in connect unless the server has a
    /// pending blocking accept. The dedicated accept thread handles that path.
    #[cfg(windows)]
    fn accept_client_connections(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Drains server events from the dedicated channel.
    ///
    /// Uses the original full-render semantics when pane graphics are dormant.
    fn drain_server_events(&mut self) -> bool {
        let mut changed = false;
        while let Ok(ev) = self.server_event_rx.try_recv() {
            changed |= self.handle_server_event(ev);
        }
        changed
    }

    /// Returns the strongest render impact from the drained event batch.
    fn drain_server_events_with_render_impact(&mut self) -> RenderImpact {
        let mut impact = RenderImpact::None;
        while let Ok(ev) = self.server_event_rx.try_recv() {
            impact.merge(self.handle_server_event_with_render_impact(ev));
        }
        impact
    }

    fn terminal_id_by_string(&self, terminal_id: &str) -> Option<crate::terminal::TerminalId> {
        self.app
            .state
            .terminals
            .keys()
            .find(|id| id.to_string() == terminal_id)
            .cloned()
    }

    fn runtime_for_terminal_id_string(
        &self,
        terminal_id: &str,
    ) -> Option<&crate::terminal::TerminalRuntime> {
        let terminal_id = self.terminal_id_by_string(terminal_id)?;
        self.app.terminal_runtimes.get(&terminal_id)
    }

    fn resolve_terminal_target_id_string(&self, target: &str) -> Option<String> {
        if self.terminal_id_by_string(target).is_some() {
            return Some(target.to_owned());
        }
        self.app
            .resolve_terminal_target(target)
            .ok()
            .map(|resolved| resolved.terminal_id)
    }

    fn write_client_clipboard_image(
        &mut self,
        client_id: u64,
        extension: &str,
        data: &[u8],
    ) -> std::io::Result<String> {
        let staged = crate::server::clipboard_image::stage(client_id, extension, data)?;
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.staged_clipboard_files.push(staged.path);
        }
        info!(client_id, bytes = data.len(), path = %staged.paste_text, "staged client clipboard image");
        Ok(staged.paste_text)
    }

    fn paste_client_clipboard_image_path(&mut self, client_id: u64, path: String) -> bool {
        if let Some(ClientConnection {
            mode: ClientConnectionMode::TerminalAttach { terminal_id },
            ..
        }) = self.clients.get(&client_id)
        {
            if let Some(runtime) = self.runtime_for_terminal_id_string(terminal_id) {
                let payload = paste_payload_for_runtime(runtime, &path);
                if let Err(err) = runtime.try_send_bytes(Bytes::from(payload)) {
                    warn!(client_id, terminal_id = %terminal_id, err = %err, "terminal attach clipboard image paste failed");
                }
            }
            return true;
        }

        let foreground_changed = self.promote_client_to_foreground(client_id);
        if foreground_changed {
            self.resize_shared_runtime_to_effective_size_before_input();
        }
        if let Some(client) = self.clients.get_mut(&client_id) {
            client.request_semantic_redraw_after_input();
        }
        self.app.route_client_events(
            vec![crate::raw_input::RawInputEvent::Paste(path)],
            self.foreground_client_id == Some(client_id),
        );
        true
    }

    fn resolve_terminal_session_target(
        &mut self,
        client_id: u64,
        target: &str,
        action: &str,
    ) -> Option<String> {
        if !self.client_is_pending_terminal_mode(client_id) {
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some(
                        format!(
                            "terminal session {action} failed: connection is not pending terminal session"
                        ),
                    ),
                },
            );
            self.remove_client_and_resize_if_needed(client_id);
            return None;
        }

        let Some(terminal_id) = self.resolve_terminal_target_id_string(target) else {
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some(format!(
                        "terminal session {action} failed: terminal target {target} not found"
                    )),
                },
            );
            self.remove_client_and_resize_if_needed(client_id);
            return None;
        };

        Some(terminal_id)
    }

    fn observe_terminal_client(&mut self, client_id: u64, target: String) -> bool {
        let Some(terminal_id) = self.resolve_terminal_session_target(client_id, &target, "observe")
        else {
            return false;
        };

        let stamp = self.allocate_activity_stamp();
        let Some(client) = self.clients.get_mut(&client_id) else {
            return false;
        };
        let (cols, rows) = client.terminal_size;
        client.mode = ClientConnectionMode::TerminalObserve {
            terminal_id: terminal_id.clone(),
        };
        client.pending_terminal_attach = false;
        client.render_state.reset_baseline();
        client.last_activity = stamp;
        let was_foreground = self.foreground_client_id == Some(client_id);
        if was_foreground {
            self.promote_latest_remaining_client();
        }

        info!(client_id, cols, rows, terminal_id = %terminal_id, "terminal observe client connected");
        true
    }

    fn control_terminal_client(&mut self, client_id: u64, target: String, takeover: bool) -> bool {
        let Some(terminal_id) = self.resolve_terminal_session_target(client_id, &target, "control")
        else {
            return false;
        };

        self.attach_terminal_client(client_id, terminal_id, takeover)
    }

    fn handle_terminal_attach_scroll(
        &mut self,
        client_id: u64,
        source: AttachScrollSource,
        direction: AttachScrollDirection,
        lines: u16,
        column: Option<u16>,
        row: Option<u16>,
        modifiers: u8,
    ) -> bool {
        let Some(ClientConnection {
            mode: ClientConnectionMode::TerminalAttach { terminal_id },
            ..
        }) = self.clients.get(&client_id)
        else {
            return false;
        };
        let Some(runtime) = self.runtime_for_terminal_id_string(terminal_id) else {
            return false;
        };

        if let Err(err) =
            apply_terminal_attach_scroll(runtime, source, direction, lines, column, row, modifiers)
        {
            warn!(client_id, terminal_id = %terminal_id, err = %err, "terminal attach scroll failed");
        }
        true
    }

    fn pane_effective_state(&self, pane_id: crate::layout::PaneId) -> crate::detect::AgentState {
        self.app
            .state
            .workspaces
            .iter()
            .find_map(|ws| {
                ws.tabs.iter().find_map(|tab| {
                    let pane = tab.panes.get(&pane_id)?;
                    self.app
                        .state
                        .terminals
                        .get(&pane.attached_terminal_id)
                        .map(|terminal| terminal.state)
                })
            })
            .unwrap_or(crate::detect::AgentState::Unknown)
    }

    fn pane_effective_agent_label(&self, pane_id: crate::layout::PaneId) -> Option<String> {
        self.app.state.workspaces.iter().find_map(|ws| {
            ws.tabs.iter().find_map(|tab| {
                let pane = tab.panes.get(&pane_id)?;
                self.app
                    .state
                    .terminals
                    .get(&pane.attached_terminal_id)
                    .and_then(|terminal| terminal.effective_agent_label())
                    .map(str::to_string)
            })
        })
    }

    fn forward_pane_state_update_notifications_to_clients(
        &mut self,
        update: &crate::app::actions::PaneStateUpdate,
    ) {
        if self.app.state.toast_config.delay_seconds != 0 {
            return;
        }

        let is_active_tab = self
            .app
            .state
            .pane_is_in_active_tab(update.ws_idx, update.pane_id);
        let suppress_active_tab_notifications =
            self.active_tab_suppresses_notifications(is_active_tab);

        if self.app.state.sound.allows(update.known_agent) {
            if let Some(sound) =
                crate::app::actions::notification_sound_for_state_change_with_agent_labels(
                    suppress_active_tab_notifications,
                    update.previous_state,
                    update.state,
                    update.previous_agent_label.as_deref(),
                    update.agent_label.as_deref(),
                )
            {
                self.send_notify_to_foreground_client(
                    protocol::NotifyKind::Sound,
                    sound_notify_message(sound),
                    None,
                );
            }
        }

        if !should_forward_toast_to_clients(self.app.state.toast_config.delivery) {
            return;
        }
        let Some(kind) = crate::app::actions::notification_toast_for_pane_state_update(
            suppress_active_tab_notifications,
            update,
        ) else {
            return;
        };
        let Some(ws) = self.app.state.workspaces.get(update.ws_idx) else {
            return;
        };
        let Some(agent_label) = update.agent_label.as_deref() else {
            return;
        };
        let event_text = match kind {
            crate::app::state::ToastKind::NeedsAttention => "needs attention",
            crate::app::state::ToastKind::Finished => "finished",
            crate::app::state::ToastKind::UpdateInstalled => "updated",
        };
        let workspace_label =
            ws.display_name_from(&self.app.state.terminals, &self.app.terminal_runtimes);
        let context = crate::app::actions::notification_context(
            ws,
            &workspace_label,
            update.ws_idx,
            update.pane_id,
        );
        self.send_notify_to_foreground_client(
            toast_notify_kind(self.app.state.toast_config.delivery)
                .expect("toast forwarding requires a client notification kind"),
            format!("{agent_label} {event_text}"),
            non_empty_body(&context),
        );
    }

    fn forward_agent_notification_delivery(
        &mut self,
        delivery: &crate::app::state::AgentNotificationDelivery,
    ) {
        if let Some(sound) = delivery.sound {
            self.send_notify_to_foreground_client(
                protocol::NotifyKind::Sound,
                sound_notify_message(sound),
                None,
            );
        }

        if should_forward_toast_to_clients(self.app.state.toast_config.delivery) {
            if let Some(toast) = &delivery.client_notification {
                self.send_notify_to_foreground_client(
                    toast_notify_kind(self.app.state.toast_config.delivery)
                        .expect("toast forwarding requires a client notification kind"),
                    &toast.title,
                    non_empty_body(&toast.context),
                );
            }
        }
    }

    fn send_notify_to_foreground_client(
        &mut self,
        kind: protocol::NotifyKind,
        message: impl Into<String>,
        body: Option<String>,
    ) -> bool {
        self.send_to_foreground_client(ServerMessage::Notify {
            kind,
            message: message.into(),
            body,
        })
    }

    fn send_flat_toast_to_foreground_client(
        &mut self,
        kind: protocol::NotifyKind,
        message: impl AsRef<str>,
    ) -> bool {
        let (title, body) = crate::terminal_notify::split_message(message.as_ref());
        self.send_notify_to_foreground_client(kind, title, body.map(str::to_string))
    }

    fn handle_notification_show_api(
        &mut self,
        id: String,
        params: api::schema::NotificationShowParams,
    ) -> String {
        use api::schema::{NotificationShowReason, ResponseResult};

        let Some(title) = sanitize_notification_text(&params.title, 80) else {
            return serde_json::to_string(&api::schema::ErrorResponse {
                id,
                error: api::schema::ErrorBody {
                    code: "invalid_params".into(),
                    message: "notification title is empty".into(),
                },
            })
            .unwrap_or_else(|_| "{}".to_string());
        };

        match self.app.state.toast_config.delivery {
            config::ToastDelivery::Off => {
                return serde_json::to_string(&api::schema::SuccessResponse {
                    id,
                    result: ResponseResult::NotificationShow {
                        shown: false,
                        reason: NotificationShowReason::Disabled,
                    },
                })
                .unwrap_or_else(|_| "{}".to_string());
            }
            config::ToastDelivery::Herdr => {
                let sound = params.sound;
                let response = self.app.handle_api_request_after_internal_events_drained(
                    api::schema::Request {
                        id,
                        method: api::schema::Method::NotificationShow(params),
                    },
                );
                if notification_show_response_shown(&response) {
                    self.forward_api_notification_sound(sound);
                }
                return response;
            }
            config::ToastDelivery::Terminal | config::ToastDelivery::System => {}
        }

        let body = params
            .body
            .as_deref()
            .and_then(|body| sanitize_notification_text(body, 240));
        if self.app.api_notification_rate_limited(Instant::now()) {
            return serde_json::to_string(&api::schema::SuccessResponse {
                id,
                result: ResponseResult::NotificationShow {
                    shown: false,
                    reason: NotificationShowReason::RateLimited,
                },
            })
            .unwrap_or_else(|_| "{}".to_string());
        }
        let kind = toast_notify_kind(self.app.state.toast_config.delivery)
            .expect("terminal/system delivery has notify kind");
        let shown = self.send_notify_to_foreground_client(kind, title, body);
        if shown {
            self.app.mark_api_notification_shown(Instant::now());
            self.forward_api_notification_sound(params.sound);
        }
        let reason = if shown {
            NotificationShowReason::Shown
        } else {
            NotificationShowReason::NoForegroundClient
        };

        serde_json::to_string(&api::schema::SuccessResponse {
            id,
            result: ResponseResult::NotificationShow { shown, reason },
        })
        .unwrap_or_else(|_| "{}".to_string())
    }

    fn handle_client_window_title_api(&mut self, id: String, title: Option<String>) -> String {
        use api::schema::{ClientWindowTitleReason, ResponseResult};

        let title = match title {
            Some(title) => match sanitize_window_title_text(&title, 200) {
                Some(title) => Some(title),
                None => {
                    return serde_json::to_string(&api::schema::ErrorResponse {
                        id,
                        error: api::schema::ErrorBody {
                            code: "invalid_params".into(),
                            message: "window title is empty".into(),
                        },
                    })
                    .unwrap_or_else(|_| "{}".to_string());
                }
            },
            None => None,
        };
        let set_title = title.is_some();
        let changed = self.send_to_foreground_client(ServerMessage::WindowTitle { title });
        let reason = match (changed, set_title) {
            (true, true) => ClientWindowTitleReason::Set,
            (true, false) => ClientWindowTitleReason::Cleared,
            (false, _) => ClientWindowTitleReason::NoForegroundClient,
        };
        serde_json::to_string(&api::schema::SuccessResponse {
            id,
            result: ResponseResult::ClientWindowTitle { changed, reason },
        })
        .unwrap_or_else(|_| "{}".to_string())
    }

    fn forward_api_notification_sound(&mut self, sound: api::schema::NotificationShowSound) {
        let Some(sound) = sound.to_sound() else {
            return;
        };
        self.send_notify_to_foreground_client(
            protocol::NotifyKind::Sound,
            sound_notify_message(sound),
            None,
        );
    }

    /// Handles a single internal event with forwarding logic for clipboard,
    /// sound, and toast notifications to connected clients.
    ///
    /// ALL internal events MUST be routed through this method to ensure
    /// clipboard/notify forwarding is never bypassed. Do not call
    /// `self.app.handle_internal_event()` directly for any internal event
    /// in the headless server — use this method instead.
    ///
    /// Returns true if the event changed visual state (requiring a re-render).
    fn handle_internal_event_with_forwarding(&mut self, ev: AppEvent) -> bool {
        match &ev {
            AppEvent::ClipboardWrite { content } => {
                // Clipboard writes are client-local side effects. Forward them only to
                // the foreground client instead of broadcasting to every attached client.
                let data = base64::engine::general_purpose::STANDARD.encode(content.as_slice());
                if self.send_to_foreground_client(ServerMessage::Clipboard { data }) {
                    self.app.show_clipboard_feedback();
                }
                true
            }
            AppEvent::PrefixInputSource { active } => {
                // Input-source switching is a client-local host side effect; forward it to the
                // foreground client (which owns the real TIS switch + run-loop pump), like clipboard.
                self.send_to_foreground_client(ServerMessage::PrefixInputSource {
                    active: *active,
                });
                true
            }
            AppEvent::StateChanged { pane_id, agent, .. } => {
                // Capture toast before handling.
                let toast_before = self.app.state.toast.clone();
                let pane_id_val = *pane_id;
                let agent_val = *agent;

                // Find the previous effective state of this pane before the event
                // is processed. Notifications must follow effective state changes,
                // not raw fallback reports that may be masked by hook authority.
                let prev_state = self.pane_effective_state(pane_id_val);
                let prev_agent_label = self.pane_effective_agent_label(pane_id_val);

                // Handle the state change (updates pane state, sets toast on AppState).
                // Headless mode disables local sound playback separately from the
                // sound policy so reloads can keep server-side notification policy live.
                self.sync_foreground_client_state();
                self.app.handle_internal_event(ev);

                // Forward sound notification to clients when server-side sound policy allows it.
                let is_active_tab = self
                    .app
                    .state
                    .active
                    .and_then(|ws_idx| self.app.state.workspaces.get(ws_idx))
                    .is_some_and(|ws| {
                        ws.find_tab_index_for_pane(pane_id_val)
                            .is_some_and(|tab_idx| ws.active_tab_index() == tab_idx)
                    });

                let suppress_active_tab_notifications =
                    self.active_tab_suppresses_notifications(is_active_tab);

                let next_state = self.pane_effective_state(pane_id_val);
                let next_agent_label = self.pane_effective_agent_label(pane_id_val);

                if self.app.state.toast_config.delay_seconds == 0
                    && self.app.state.sound.allows(agent_val)
                {
                    if let Some(sound) =
                        crate::app::actions::notification_sound_for_state_change_with_agent_labels(
                            suppress_active_tab_notifications,
                            prev_state,
                            next_state,
                            prev_agent_label.as_deref(),
                            next_agent_label.as_deref(),
                        )
                    {
                        self.send_notify_to_foreground_client(
                            protocol::NotifyKind::Sound,
                            sound_notify_message(sound),
                            None,
                        );
                    }
                }

                let toast_msg = if self.app.state.toast_config.delay_seconds == 0
                    && should_forward_toast_to_clients(self.app.state.toast_config.delivery)
                {
                    if self.app.state.toast.is_some() && self.app.state.toast != toast_before {
                        self.app
                            .state
                            .toast
                            .as_ref()
                            .map(|toast| format!("{}: {}", toast.title, toast.context))
                    } else {
                        toast_message_from_state_change(
                            &self.app.state,
                            &self.app.terminal_runtimes,
                            pane_id_val,
                            suppress_active_tab_notifications,
                            prev_state,
                            next_state,
                            prev_agent_label.as_deref(),
                        )
                    }
                } else {
                    None
                };

                if let Some(msg) = toast_msg {
                    self.send_flat_toast_to_foreground_client(
                        toast_notify_kind(self.app.state.toast_config.delivery)
                            .expect("toast forwarding requires a client notification kind"),
                        msg,
                    );
                }

                true
            }
            AppEvent::HookStateReported {
                pane_id,
                agent_label,
                ..
            } => {
                // Hook reports can be stale or no-op after sequence rejection.
                // Forward only effective state changes observed after handling.
                let toast_before = self.app.state.toast.clone();
                let pane_id_val = *pane_id;
                let agent_val = crate::detect::parse_agent_label(agent_label);

                // Capture the previous effective state for this pane. Hook reports
                // are already folded into pane.state; raw hook transitions must not
                // produce a second notification path.
                let prev_state = self.pane_effective_state(pane_id_val);
                let prev_agent_label = self.pane_effective_agent_label(pane_id_val);

                self.sync_foreground_client_state();
                self.app.handle_internal_event(ev);

                // Forward sound notification based on the effective transition when
                // server-side sound policy allows it.
                let is_active_tab = self
                    .app
                    .state
                    .active
                    .and_then(|ws_idx| self.app.state.workspaces.get(ws_idx))
                    .is_some_and(|ws| {
                        ws.find_tab_index_for_pane(pane_id_val)
                            .is_some_and(|tab_idx| ws.active_tab_index() == tab_idx)
                    });

                let suppress_active_tab_notifications =
                    self.active_tab_suppresses_notifications(is_active_tab);

                let next_state = self.pane_effective_state(pane_id_val);
                let next_agent_label = self.pane_effective_agent_label(pane_id_val);

                if self.app.state.toast_config.delay_seconds == 0
                    && self.app.state.sound.allows(agent_val)
                {
                    if let Some(sound) =
                        crate::app::actions::notification_sound_for_state_change_with_agent_labels(
                            suppress_active_tab_notifications,
                            prev_state,
                            next_state,
                            prev_agent_label.as_deref(),
                            next_agent_label.as_deref(),
                        )
                    {
                        self.send_notify_to_foreground_client(
                            protocol::NotifyKind::Sound,
                            sound_notify_message(sound),
                            None,
                        );
                    }
                }

                let toast_msg = if self.app.state.toast_config.delay_seconds == 0
                    && should_forward_toast_to_clients(self.app.state.toast_config.delivery)
                {
                    if self.app.state.toast.is_some() && self.app.state.toast != toast_before {
                        self.app
                            .state
                            .toast
                            .as_ref()
                            .map(|toast| format!("{}: {}", toast.title, toast.context))
                    } else {
                        toast_message_from_state_change(
                            &self.app.state,
                            &self.app.terminal_runtimes,
                            pane_id_val,
                            suppress_active_tab_notifications,
                            prev_state,
                            next_state,
                            prev_agent_label.as_deref(),
                        )
                    }
                } else {
                    None
                };

                if let Some(msg) = toast_msg {
                    self.send_flat_toast_to_foreground_client(
                        toast_notify_kind(self.app.state.toast_config.delivery)
                            .expect("toast forwarding requires a client notification kind"),
                        msg,
                    );
                }

                true
            }
            AppEvent::UpdateReady {
                version,
                install_command,
            } => {
                let toast_before = self.app.state.toast.clone();
                let version = version.clone();
                let install_command = install_command.clone();

                self.app.handle_internal_event(ev);

                let toast_msg =
                    if should_forward_toast_to_clients(self.app.state.toast_config.delivery) {
                        if self.app.state.toast.is_some() && self.app.state.toast != toast_before {
                            self.app
                                .state
                                .toast
                                .as_ref()
                                .map(|toast| format!("{}: {}", toast.title, toast.context))
                        } else {
                            Some(format!(
                                "v{version} available: {}",
                                crate::update::update_install_instruction(&install_command)
                            ))
                        }
                    } else {
                        None
                    };

                if let Some(msg) = toast_msg {
                    self.send_flat_toast_to_foreground_client(
                        toast_notify_kind(self.app.state.toast_config.delivery)
                            .expect("toast forwarding requires a client notification kind"),
                        msg,
                    );
                }

                true
            }
            AppEvent::PaneDied { pane_id } => {
                let pane_id_val = *pane_id;
                let terminal_id = self.app.state.workspaces.iter().find_map(|ws| {
                    ws.tabs.iter().find_map(|tab| {
                        tab.panes
                            .get(pane_id)
                            .map(|pane| pane.attached_terminal_id.to_string())
                    })
                });
                if let Some(update) = self
                    .app
                    .state
                    .publish_pane_process_exit_if_agent(pane_id_val)
                {
                    self.app.emit_pane_state_update(&update);
                    self.forward_pane_state_update_notifications_to_clients(&update);
                }

                self.app.handle_internal_event(ev);

                if self.app.find_pane(pane_id_val).is_none() {
                    if let Some(terminal_id) = terminal_id {
                        self.shutdown_terminal_stream_clients(
                            &terminal_id,
                            format!("terminal {terminal_id} exited"),
                        );
                    }
                }

                true
            }
            _ => {
                self.app.handle_internal_event(ev);
                true
            }
        }
    }

    /// Drains internal events, forwarding clipboard, sound, and toast
    /// notifications to connected clients instead of processing them locally.
    ///
    /// In the monolithic mode:
    /// - `ClipboardWrite` events are written to stdout via `write_osc52_bytes`.
    /// - Sound notifications are played locally via `sound::play`.
    /// - Toast notifications are set on AppState and rendered into the frame.
    ///
    /// In the headless server, there is no stdout terminal or audio subsystem,
    /// so we:
    /// - Forward `ClipboardWrite` as `ServerMessage::Clipboard` to the
    ///   foreground client only.
    /// - Detect when a sound would be played and forward as
    ///   `ServerMessage::Notify { kind: Sound }` to the foreground client.
    /// - Detect when a toast is set on AppState and forward as
    ///   `ServerMessage::Notify` to the foreground client for terminal/system delivery.
    fn drain_internal_events_with_forwarding(&mut self) -> bool {
        self.drain_internal_events_with_forwarding_up_to(crate::app::APP_EVENT_DRAIN_LIMIT)
            .1
    }

    fn drain_all_internal_events_with_forwarding(&mut self) -> bool {
        let mut changed = false;
        loop {
            let (had_event, batch_changed) =
                self.drain_internal_events_with_forwarding_up_to(crate::app::APP_EVENT_DRAIN_LIMIT);
            changed |= batch_changed;
            if !had_event {
                break;
            }
        }
        changed
    }

    fn drain_internal_events_with_forwarding_up_to(&mut self, limit: usize) -> (bool, bool) {
        let mut had_event = false;
        let mut changed = false;
        for _ in 0..limit {
            let Ok(ev) = self.app.event_rx.try_recv() else {
                break;
            };
            had_event = true;
            changed |= self.handle_internal_event_with_forwarding(ev);
        }
        (had_event, changed)
    }

    fn drain_client_config_reload_request(&mut self) {
        if !self.app.state.request_client_config_reload {
            return;
        }
        self.app.state.request_client_config_reload = false;
        self.send_to_all_clients(ServerMessage::ReloadSoundConfig);
    }

    /// Encodes a server message into a length-prefixed frame.
    fn frame_server_message(msg: &ServerMessage) -> Result<Vec<u8>, protocol::FramingError> {
        Self::frame_server_message_with_max(msg, MAX_FRAME_SIZE)
    }

    /// Encodes a server message using an explicit payload cap.
    fn frame_server_message_with_max(
        msg: &ServerMessage,
        max_frame_size: usize,
    ) -> Result<Vec<u8>, protocol::FramingError> {
        let mut framed = Vec::new();
        protocol::write_message(&mut framed, msg)?;
        let payload_len = framed.len().saturating_sub(4);
        if payload_len > max_frame_size {
            return Err(protocol::FramingError::Oversized {
                claimed: payload_len,
                max: max_frame_size,
            });
        }
        Ok(framed)
    }

    /// Sends a message to all connected clients.
    /// Broken connections are tracked and cleaned up.
    fn send_to_all_clients(&mut self, msg: ServerMessage) {
        let serialized = match Self::frame_server_message(&msg) {
            Ok(framed) => framed,
            Err(err) => {
                warn!(err = %err, "failed to serialize message for clients");
                return;
            }
        };

        let mut broken_clients: Vec<u64> = Vec::new();
        for (&client_id, client) in &mut self.clients {
            if let Some(writer) = &client.writer {
                if writer.control.send(serialized.clone()).is_err() {
                    debug!(client_id, "client writer channel closed during broadcast");
                    broken_clients.push(client_id);
                }
            }
        }

        // Remove broken clients.
        for client_id in broken_clients {
            self.remove_client_and_resize_if_needed(client_id);
        }
    }

    /// Sends a client-local side effect to the foreground client only.
    fn send_to_foreground_client(&mut self, msg: ServerMessage) -> bool {
        let Some(client_id) = self.foreground_client_id else {
            return false;
        };
        self.send_to_client(client_id, msg)
    }

    /// Sends a message to a specific client. Returns false if the client
    /// was not found or the send failed (client removed).
    fn send_to_client(&mut self, client_id: u64, msg: ServerMessage) -> bool {
        let serialized = match Self::frame_server_message(&msg) {
            Ok(framed) => framed,
            Err(err) => {
                warn!(client_id, err = %err, "failed to serialize message for client");
                return false;
            }
        };

        if let Some(client) = self.clients.get(&client_id) {
            if let Some(writer) = &client.writer {
                if writer.control.send(serialized).is_err() {
                    debug!(
                        client_id,
                        "client writer channel closed during targeted send"
                    );
                    self.remove_client_and_resize_if_needed(client_id);
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    fn shutdown_terminal_stream_clients(&mut self, terminal_id: &str, reason: String) {
        let client_ids = terminal_stream_client_ids(&self.clients, terminal_id);

        for client_id in client_ids {
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some(reason.clone()),
                },
            );
            self.remove_client_and_resize_if_needed(client_id);
        }
    }

    fn send_terminal_stream_detach_shutdown(&mut self, client_id: u64) {
        if matches!(
            self.clients.get(&client_id).map(|client| &client.mode),
            Some(
                ClientConnectionMode::TerminalAttach { .. }
                    | ClientConnectionMode::TerminalObserve { .. }
            )
        ) {
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some("detached".to_owned()),
                },
            );
        }
    }

    #[cfg(unix)]
    fn disconnect_all_clients_for_handoff(&mut self) {
        let client_ids = self.clients.keys().copied().collect::<Vec<_>>();
        for client_id in client_ids {
            self.send_client_graphics_cleanup(client_id);
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some(
                        "live update in progress; reconnect after handoff completes".to_owned(),
                    ),
                },
            );
            if let Some(client) = self.clients.get_mut(&client_id) {
                client.writer = None;
            }
            let _ = self.remove_client(client_id);
        }
        self.foreground_client_id = None;
        self.sync_foreground_client_state();
        self.resize_shared_runtime_to_effective_size();
    }

    fn attach_terminal_client(
        &mut self,
        client_id: u64,
        terminal_id: String,
        takeover: bool,
    ) -> bool {
        if !self.client_is_pending_terminal_mode(client_id) {
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some(
                        "terminal attach failed: connection is not pending terminal attach"
                            .to_owned(),
                    ),
                },
            );
            self.remove_client_and_resize_if_needed(client_id);
            return false;
        }

        let Some(real_terminal_id) = self.terminal_id_by_string(&terminal_id) else {
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some(format!(
                        "terminal attach failed: terminal {terminal_id} not found"
                    )),
                },
            );
            self.remove_client_and_resize_if_needed(client_id);
            return false;
        };

        if let Some(existing_owner) = self.terminal_attach_owners.get(&terminal_id).copied() {
            if existing_owner != client_id && !takeover {
                self.send_to_client(
                    client_id,
                    ServerMessage::ServerShutdown {
                        reason: Some(format!(
                            "terminal attach failed: terminal {terminal_id} already has an attached client; retry with --takeover"
                        )),
                    },
                );
                self.remove_client_and_resize_if_needed(client_id);
                return false;
            }
            if existing_owner != client_id {
                self.send_to_client(
                    existing_owner,
                    ServerMessage::ServerShutdown {
                        reason: Some("terminal attach taken over".to_owned()),
                    },
                );
                self.remove_client_and_resize_if_needed(existing_owner);
            }
        }

        let stamp = self.allocate_activity_stamp();
        let Some(client) = self.clients.get_mut(&client_id) else {
            return false;
        };
        let (cols, rows) = client.terminal_size;
        let cell_size = client.cell_size;
        client.mode = ClientConnectionMode::TerminalAttach {
            terminal_id: terminal_id.clone(),
        };
        client.pending_terminal_attach = false;
        client.render_state.reset_baseline();
        client.last_activity = stamp;
        let was_foreground = self.foreground_client_id == Some(client_id);
        if was_foreground {
            self.promote_latest_remaining_client();
        }

        info!(client_id, cols, rows, terminal_id = %terminal_id, "terminal attach client connected");
        self.terminal_attach_owners
            .insert(terminal_id.clone(), client_id);
        self.app
            .state
            .direct_attach_resize_locks
            .insert(real_terminal_id.clone());
        self.app
            .start_pending_agent_resume_for_terminal(&real_terminal_id, rows, cols, true);
        if let Some(runtime) = self.app.terminal_runtimes.get(&real_terminal_id) {
            runtime.resize(rows, cols, cell_size.width_px, cell_size.height_px);
        }
        true
    }

    fn client_is_pending_terminal_mode(&self, client_id: u64) -> bool {
        self.clients.get(&client_id).is_some_and(|client| {
            client.pending_terminal_attach && matches!(client.mode, ClientConnectionMode::App)
        })
    }

    /// Handles a server event. Returns true if the event requires a re-render.
    fn handle_client_input_events(
        &mut self,
        client_id: u64,
        events: Vec<crate::raw_input::RawInputEvent>,
    ) -> bool {
        let source_was_foreground = self.foreground_client_id == Some(client_id);
        let source_is_full_app = self
            .clients
            .get(&client_id)
            .is_some_and(ClientConnection::is_full_app_client);
        let host_surface_redraw = crate::raw_input::events_require_host_surface_redraw(
            &events,
            self.app.state.redraw_on_focus_gained,
        );
        if let Some(client) = self.clients.get_mut(&client_id) {
            if host_surface_redraw {
                client.request_repaint();
                client.defer_full_render();
            } else {
                // Ensure semantic clients receive one post-input frame even if the
                // semantic buffer compares equal. Terminal-ANSI clients must keep their
                // server-side blit baseline; resetting it here forces a full redraw on
                // every keypress and makes remote sessions feel extremely slow.
                client.request_semantic_redraw_after_input();
            }
        }
        if source_is_full_app {
            self.update_client_outer_focus_from_events(client_id, &events);
            if events
                .iter()
                .any(|event| matches!(event, crate::raw_input::RawInputEvent::OuterFocusLost))
            {
                self.app.clear_input_source(client_id);
            }
        }
        let events = events_for_app_routing(events, source_was_foreground, source_is_full_app);
        let interaction = events_include_interaction(&events);
        let foreground_changed = if interaction {
            self.promote_client_to_foreground(client_id)
        } else {
            false
        };
        if foreground_changed {
            self.resize_shared_runtime_to_effective_size_before_input();
        }
        let theme_changed = self.update_client_host_theme_from_events(client_id, &events);
        // Client-local theme reports were applied above; routing them again would update every
        // pane once per palette entry instead of once per captured batch.
        // Hit geometry lives in one shared `view`, which the render loop
        // rewrites once per client and therefore leaves holding whichever
        // client drew last. That was harmless while only the foreground client
        // sent input. Now that each display owns its own tab, input arrives
        // from any of them, so a pointer event would otherwise be resolved
        // against another display's layout — a different tab, at a different
        // size. Recompute this client's geometry before routing.
        //
        // Panes are not resized here: sizing belongs to the render pass that
        // negotiated it. TP-MCF-VIEW-01
        if events
            .iter()
            .any(|event| matches!(event, crate::raw_input::RawInputEvent::Mouse(_)))
        {
            if let Some(size) = self
                .clients
                .get(&client_id)
                .filter(|client| client.is_full_app_client())
                .map(|client| client.terminal_size)
            {
                // Only when the geometry belongs to someone else. A pointer
                // burst from one display recomputes once and then rides the
                // ownership, and the foreground display -- whose geometry the
                // last frame already left in place -- never pays at all.
                // Without this the serial input loop carries a layout pass per
                // pointer motion, which is the cost the inert-motion render
                // gate was added to avoid.
                if self.view_owner != Some((client_id, size)) {
                    let previous_viewer = self.app.state.enter_viewer(Some(client_id));
                    crate::ui::compute_view_without_resizing_panes(
                        &mut self.app.state,
                        &self.app.terminal_runtimes,
                        Rect::new(0, 0, size.0, size.1),
                    );
                    self.app.state.restore_viewer(previous_viewer);
                    self.view_owner = Some((client_id, size));
                    #[cfg(test)]
                    {
                        self.view_recomputes_for_input += 1;
                    }
                }
            }
        }
        let render_requested = self.app.route_client_events_from(client_id, events, false);
        if self.app.take_config_reloaded_from_disk() {
            self.reload_server_config(false);
        } else {
            self.sync_foreground_client_state();
        }

        if self.app.state.detach_requested {
            self.app.state.detach_requested = false;
            info!(client_id, "client detach requested via keybind");

            self.send_client_graphics_cleanup(client_id);
            self.send_to_client(
                client_id,
                ServerMessage::ServerShutdown {
                    reason: Some("detached".to_owned()),
                },
            );

            if let Some(client) = self.clients.get_mut(&client_id) {
                client.writer = None;
            }

            false
        } else {
            foreground_changed || theme_changed || render_requested
        }
    }

    fn handle_server_event(&mut self, ev: ServerEvent) -> bool {
        if self.handoff_in_progress && Self::ignore_client_event_during_handoff(&ev) {
            return false;
        }

        match ev {
            ServerEvent::ClientConnected {
                client_id,
                cols,
                rows,
                cell_width_px,
                cell_height_px,
                keybindings,
                writer,
                render_encoding,
                direct_attach_requested,
            } => {
                if self.handoff_in_progress {
                    if let Ok(message) =
                        Self::frame_server_message(&ServerMessage::ServerShutdown {
                            reason: Some(
                                "live update in progress; reconnect after handoff completes"
                                    .to_owned(),
                            ),
                        })
                    {
                        let _ = writer.control.send(message);
                    }
                    return false;
                }
                let first_app_client = !direct_attach_requested && self.app_client_count() == 0;
                info!(
                    client_id,
                    cols,
                    rows,
                    cell_width_px,
                    cell_height_px,
                    ?render_encoding,
                    "client connected"
                );
                let last_activity = self.allocate_activity_stamp();
                self.clients.insert(
                    client_id,
                    ClientConnection::new_with_mode(
                        ClientConnectionMode::App,
                        keybindings,
                        (cols, rows),
                        crate::kitty_graphics::HostCellSize {
                            width_px: cell_width_px,
                            height_px: cell_height_px,
                        },
                        crate::terminal_theme::TerminalTheme::default(),
                        None,
                        last_activity,
                        render_encoding,
                        direct_attach_requested,
                        Some(writer),
                    ),
                );
                if !direct_attach_requested {
                    self.foreground_client_id = Some(client_id);
                }
                if first_app_client {
                    self.app.mark_git_status_refresh_due(Instant::now());
                }
                self.sync_foreground_client_state();
                self.resize_shared_runtime_to_effective_size();
                self.nudge_handoff_panes_on_first_client_attach();
                true
            }
            ServerEvent::ClientAttachTerminal {
                client_id,
                terminal_id,
                takeover,
            } => self.attach_terminal_client(client_id, terminal_id, takeover),
            ServerEvent::ClientObserveTerminal { client_id, target } => {
                self.observe_terminal_client(client_id, target)
            }
            ServerEvent::ClientControlTerminal {
                client_id,
                target,
                takeover,
            } => self.control_terminal_client(client_id, target, takeover),
            ServerEvent::ClientAttachScroll {
                client_id,
                source,
                direction,
                lines,
                column,
                row,
                modifiers,
            } => self.handle_terminal_attach_scroll(
                client_id, source, direction, lines, column, row, modifiers,
            ),
            ServerEvent::ClientInput { client_id, data } => {
                if self.handoff_in_progress {
                    debug!(
                        client_id,
                        len = data.len(),
                        "ignored client input during handoff"
                    );
                    return false;
                }
                debug!(client_id, len = data.len(), "client input received");
                if let Some(ClientConnection {
                    mode: ClientConnectionMode::TerminalAttach { terminal_id },
                    ..
                }) = self.clients.get(&client_id)
                {
                    if let Some(runtime) = self.runtime_for_terminal_id_string(terminal_id) {
                        if let Err(err) = apply_terminal_attach_input(runtime, data) {
                            warn!(client_id, terminal_id = %terminal_id, err = %err);
                        }
                    }
                    return true;
                }
                if matches!(
                    self.clients.get(&client_id).map(|client| &client.mode),
                    Some(ClientConnectionMode::TerminalObserve { .. })
                ) {
                    return false;
                }
                let events = if let Some(client) = self.clients.get_mut(&client_id) {
                    let mut events = client.raw_input.push(&data);
                    // The thin client only forwards a bare ESC after its local input timeout.
                    if data.as_slice() == b"\x1b" {
                        events.extend(client.raw_input.flush_timeout());
                    }
                    events
                } else {
                    Vec::new()
                };
                self.handle_client_input_events(client_id, events)
            }
            ServerEvent::ClientInputEvents { client_id, events } => {
                if self.handoff_in_progress {
                    debug!(
                        client_id,
                        len = events.len(),
                        "ignored client input events during handoff"
                    );
                    return false;
                }
                debug!(
                    client_id,
                    len = events.len(),
                    "client input events received"
                );
                if matches!(
                    self.clients.get(&client_id).map(|client| &client.mode),
                    Some(ClientConnectionMode::TerminalObserve { .. })
                ) {
                    return false;
                }
                let events = events
                    .iter()
                    .map(crate::protocol::ClientInputEvent::to_raw_input_event)
                    .collect();
                self.handle_client_input_events(client_id, events)
            }
            ServerEvent::ClientPasteRejected {
                client_id,
                size,
                max,
            } => {
                self.send_to_client(
                    client_id,
                    ServerMessage::Notify {
                        kind: protocol::NotifyKind::Toast,
                        message: "Paste rejected".to_owned(),
                        body: Some(format!(
                            "Input message is {size} bytes; Herdr's limit is {max} bytes"
                        )),
                    },
                );
                false
            }
            ServerEvent::ClientClipboardImage {
                client_id,
                extension,
                data,
            } => {
                debug!(
                    client_id,
                    len = data.len(),
                    extension = %extension,
                    "client clipboard image received"
                );
                if matches!(
                    self.clients.get(&client_id).map(|client| &client.mode),
                    Some(ClientConnectionMode::TerminalObserve { .. })
                ) {
                    return false;
                }
                match self.write_client_clipboard_image(client_id, &extension, &data) {
                    Ok(path) => self.paste_client_clipboard_image_path(client_id, path),
                    Err(err) => {
                        warn!(client_id, err = %err, "failed to stage client clipboard image");
                        true
                    }
                }
            }
            ServerEvent::ClientResize {
                client_id,
                cols,
                rows,
                cell_width_px,
                cell_height_px,
            } => {
                info!(
                    client_id,
                    cols, rows, cell_width_px, cell_height_px, "client resize"
                );
                let direct_terminal_id = if let Some(ClientConnection {
                    mode: ClientConnectionMode::TerminalAttach { terminal_id },
                    terminal_size,
                    cell_size,
                    render_state,
                    ..
                }) = self.clients.get_mut(&client_id)
                {
                    *terminal_size = (cols, rows);
                    *cell_size = crate::kitty_graphics::HostCellSize {
                        width_px: cell_width_px,
                        height_px: cell_height_px,
                    };
                    render_state.request_repaint();
                    Some(terminal_id.clone())
                } else {
                    None
                };
                if let Some(terminal_id) = direct_terminal_id {
                    if let Some(runtime) = self.runtime_for_terminal_id_string(&terminal_id) {
                        runtime.resize(rows, cols, cell_width_px, cell_height_px);
                    }
                    return true;
                }
                if let Some(ClientConnection {
                    mode: ClientConnectionMode::TerminalObserve { .. },
                    terminal_size,
                    cell_size,
                    render_state,
                    ..
                }) = self.clients.get_mut(&client_id)
                {
                    *terminal_size = (cols, rows);
                    *cell_size = crate::kitty_graphics::HostCellSize {
                        width_px: cell_width_px,
                        height_px: cell_height_px,
                    };
                    render_state.request_repaint();
                    return true;
                }
                if let Some(client) = self.clients.get_mut(&client_id) {
                    client.terminal_size = (cols, rows);
                    client.cell_size = crate::kitty_graphics::HostCellSize {
                        width_px: cell_width_px,
                        height_px: cell_height_px,
                    };
                }
                self.promote_client_to_foreground(client_id);
                self.resize_shared_runtime_to_effective_size();
                true
            }
            ServerEvent::ClientDetach { client_id } => {
                info!(client_id, "client detached");
                self.send_terminal_stream_detach_shutdown(client_id);
                self.remove_client_and_resize_if_needed(client_id);
                true
            }
            ServerEvent::ClientDisconnected { client_id } => {
                info!(client_id, "client disconnected");
                self.remove_client_and_resize_if_needed(client_id);
                true
            }
            ServerEvent::ClientWriterDrained { client_id } => {
                let Some(client) = self.clients.get_mut(&client_id) else {
                    return false;
                };
                client.take_deferred_render() != DeferredRender::None
            }
            ServerEvent::QuitSignal => {
                // The quit check at the top of the loop handles this.
                // No render needed — the next iteration will initiate shutdown.
                false
            }
        }
    }

    fn handle_server_event_with_render_impact(&mut self, ev: ServerEvent) -> RenderImpact {
        let deferred_render = match &ev {
            ServerEvent::ClientWriterDrained { client_id } => self
                .clients
                .get(client_id)
                .map_or(DeferredRender::None, ClientConnection::deferred_render),
            _ => DeferredRender::None,
        };
        if !self.handle_server_event(ev) {
            return RenderImpact::None;
        }
        match deferred_render {
            DeferredRender::Graphics => RenderImpact::Graphics,
            DeferredRender::None | DeferredRender::Full => RenderImpact::Full,
        }
    }

    fn ignore_client_event_during_handoff(ev: &ServerEvent) -> bool {
        !matches!(
            ev,
            ServerEvent::ClientConnected { .. }
                | ServerEvent::ClientDisconnected { .. }
                | ServerEvent::ClientWriterDrained { .. }
                | ServerEvent::QuitSignal
        )
    }

    /// Drains API requests with shutdown awareness.
    ///
    /// During shutdown, remaining requests get a `server_unavailable` error.
    fn drain_api_requests_with_shutdown_check(&mut self) -> bool {
        let mut changed = false;
        while let Ok(msg) = self.app.api_rx.try_recv() {
            changed |= self.handle_api_request_with_shutdown_check(msg);
        }
        changed
    }

    fn drain_api_requests_with_render_impact(&mut self) -> RenderImpact {
        let mut impact = RenderImpact::None;
        while let Ok(msg) = self.app.api_rx.try_recv() {
            impact.merge(self.handle_api_request_with_render_impact(msg));
        }
        impact
    }

    /// Handles a single API request with shutdown awareness.
    ///
    /// Also forwards any toast/sound notifications that result from the API
    /// request to connected clients. API methods like `pane.report_agent`
    /// trigger internal events that may set toast state or would normally
    /// play sounds — in headless mode we forward these to clients instead.
    fn handle_api_request_with_shutdown_check(&mut self, msg: api::ApiRequestMessage) -> bool {
        self.handle_api_request_with_shutdown_check_inner(msg, false)
    }

    fn handle_api_request_with_render_impact(
        &mut self,
        msg: api::ApiRequestMessage,
    ) -> RenderImpact {
        if matches!(
            &msg.request.method,
            api::schema::Method::PaneGraphicsStreamSet(_)
        ) {
            return self.handle_pane_graphics_stream_frame(msg);
        }
        if self.handle_api_request_with_shutdown_check_inner(msg, false) {
            RenderImpact::Full
        } else {
            RenderImpact::None
        }
    }

    fn handle_api_request_with_shutdown_check_inner(
        &mut self,
        msg: api::ApiRequestMessage,
        skip_default_workspace_for_request: bool,
    ) -> bool {
        if self.shutting_down {
            // During shutdown, respond with server_unavailable.
            let response = serde_json::to_string(&api::schema::ErrorResponse {
                id: msg.request.id,
                error: api::schema::ErrorBody {
                    code: "server_unavailable".into(),
                    message: "server is shutting down".into(),
                },
            })
            .unwrap_or_else(|_| {
                r#"{"id":"","error":{"code":"server_unavailable","message":"server is shutting down"}}"#
                    .to_string()
            });
            let _ = msg.respond_to.send(response);
            return false;
        }

        let metadata_expired = self.app.expire_due_metadata(Instant::now());

        if let api::schema::Method::ServerLiveHandoff(params) = &msg.request.method {
            let handoff_result = self.perform_live_handoff(params.clone());
            let handoff_succeeded = handoff_result.is_ok();
            let response = match handoff_result {
                Ok(()) => serde_json::to_string(&api::schema::SuccessResponse {
                    id: msg.request.id,
                    result: api::schema::ResponseResult::Ok {},
                }),
                Err(err) => serde_json::to_string(&api::schema::ErrorResponse {
                    id: msg.request.id,
                    error: api::schema::ErrorBody {
                        code: "handoff_failed".into(),
                        message: err.to_string(),
                    },
                }),
            }
            .unwrap_or_else(|_| "{}".to_string());
            let _ = msg.respond_to.send(response);
            if handoff_succeeded {
                wait_for_live_handoff_response_write(msg.response_write_complete);
                self.finish_live_handoff_shutdown();
            }
            return true;
        }

        if let api::schema::Method::NotificationShow(params) = &msg.request.method {
            let response =
                self.handle_notification_show_api(msg.request.id.clone(), params.clone());
            let _ = msg.respond_to.send(response);
            return true;
        }

        match &msg.request.method {
            api::schema::Method::ClientWindowTitleSet(params) => {
                let response = self.handle_client_window_title_api(
                    msg.request.id.clone(),
                    Some(params.title.clone()),
                );
                let _ = msg.respond_to.send(response);
                return true;
            }
            api::schema::Method::ClientWindowTitleClear(_) => {
                let response = self.handle_client_window_title_api(msg.request.id.clone(), None);
                let _ = msg.respond_to.send(response);
                return true;
            }
            _ => {}
        }

        let pane_graphics_revision_before = matches!(
            &msg.request.method,
            api::schema::Method::PaneGraphicsSet(_)
                | api::schema::Method::PaneGraphicsClear(_)
                | api::schema::Method::PaneGraphicsStreamOpen(_)
                | api::schema::Method::PaneGraphicsStreamClose(_)
        )
        .then_some(self.app.state.pane_graphics_revision);
        let mut changed = metadata_expired
            | (pane_graphics_revision_before.is_none() && api::request_changes_ui(&msg.request));
        let skip_default_workspace = skip_default_workspace_for_request
            || matches!(
                &msg.request.method,
                api::schema::Method::ServerStop(_) | api::schema::Method::ServerLiveHandoff(_)
            );
        changed |= self.drain_all_internal_events_with_forwarding();

        // Capture toast and effective pane states before the API call so we can
        // forward resulting client-local notifications. API requests like
        // pane.report_agent trigger handle_internal_event internally, which
        // bypasses drain_internal_events_with_forwarding. Headless mode disables
        // local sound playback, so sound notifications need to be forwarded here.
        let toast_before = self.app.state.toast.clone();
        let pane_states_before: Vec<(
            usize,
            crate::layout::PaneId,
            crate::detect::AgentState,
            Option<String>,
        )> = {
            let terminals = &self.app.state.terminals;
            self.app
                .state
                .workspaces
                .iter()
                .enumerate()
                .flat_map(|(ws_idx, ws)| {
                    ws.tabs.iter().flat_map(move |tab| {
                        tab.panes.iter().filter_map(move |(&pane_id, pane)| {
                            terminals.get(&pane.attached_terminal_id).map(|terminal| {
                                (
                                    ws_idx,
                                    pane_id,
                                    terminal.state,
                                    terminal.effective_agent_label().map(str::to_string),
                                )
                            })
                        })
                    })
                })
                .collect()
        };

        self.sync_foreground_client_state();
        if matches!(
            &msg.request.method,
            api::schema::Method::WorktreeCreate(_) | api::schema::Method::WorktreeRemove(_)
        ) {
            let deferred_changed = self
                .app
                .handle_deferred_worktree_api_request(msg.request, msg.respond_to);
            return changed | deferred_changed;
        }
        let response = if matches!(
            &msg.request.method,
            api::schema::Method::ServerReloadConfig(_)
        ) {
            let report = self.reload_server_config(true);
            serde_json::to_string(&api::schema::SuccessResponse {
                id: msg.request.id.clone(),
                result: api::schema::ResponseResult::ConfigReload {
                    status: report.status,
                    diagnostics: report.diagnostics,
                },
            })
            .unwrap_or_else(|err| {
                serde_json::to_string(&api::schema::ErrorResponse {
                    id: String::new(),
                    error: api::schema::ErrorBody {
                        code: "serialization_error".into(),
                        message: err.to_string(),
                    },
                })
                .unwrap_or_else(|_| "{}".to_string())
            })
        } else if opens_a_person_surface(&msg.request.method) {
            // A plugin opens its viewer by calling back into the API, so the
            // request that opens a popup arrives with no display behind it.
            // Handled with no viewer it lands in the session's registers, and
            // the broadcast rule then copies the popup onto every attached
            // display. Scoping the request to one display is what keeps the
            // broadcast from ever seeing it, and it is also what gives the
            // popup an owner instead of leaving it where no display looks.
            //
            // The owner is the display whose terminal has focus: it is the one
            // the person just clicked in, and it is the same display whose
            // click queued the action this plugin is running for. With no
            // focused display this resolves to `None`, which is exactly the
            // single-view behaviour it replaced.
            //
            // TP-SUR-BROADCAST-05
            let previous_viewer = self.app.state.enter_viewer(self.foreground_client_id);
            let response = self
                .app
                .handle_api_request_after_internal_events_drained(msg.request);
            self.app.state.restore_viewer(previous_viewer);
            response
        } else {
            self.app
                .handle_api_request_after_internal_events_drained(msg.request)
        };
        let _ = msg.respond_to.send(response);

        if let Some(revision_before) = pane_graphics_revision_before {
            changed |= revision_before != self.app.state.pane_graphics_revision;
        }

        // Forward new toast state only when a client-local delivery mode is selected.
        // Herdr delivery renders the toast in-frame and must not ask clients to
        // show a terminal or system notification.
        let toast_after = self.app.state.toast.clone();
        let forwarded_toast_from_state = if should_forward_toast_to_clients(
            self.app.state.toast_config.delivery,
        ) && toast_after.is_some()
            && toast_after != toast_before
        {
            if let Some(toast) = &toast_after {
                debug!(title = %toast.title, body = %toast.context, "forwarding toast notification from API request");
                self.send_notify_to_foreground_client(
                    toast_notify_kind(self.app.state.toast_config.delivery)
                        .expect("toast forwarding requires a client notification kind"),
                    &toast.title,
                    non_empty_body(&toast.context),
                );
                true
            } else {
                false
            }
        } else {
            false
        };

        // Forward notifications for effective pane state changes that occurred
        // during the API request. Hook authority is already folded into
        // pane.state, so raw hook transitions must not produce separate sounds.
        for (ws_idx, pane_id, prev_state, prev_agent_label) in &pane_states_before {
            let pane_after = self
                .app
                .state
                .workspaces
                .get(*ws_idx)
                .and_then(|ws| ws.tabs.iter().find_map(|tab| tab.panes.get(pane_id)));

            let Some(pane_after) = pane_after else {
                continue;
            };

            let Some(terminal_after) = self
                .app
                .state
                .terminals
                .get(&pane_after.attached_terminal_id)
            else {
                continue;
            };

            let new_state = terminal_after.state;
            if new_state == *prev_state {
                continue;
            }

            let is_active_tab = self.app.state.pane_is_in_active_tab(*ws_idx, *pane_id);
            let suppress_active_tab_notifications =
                self.active_tab_suppresses_notifications(is_active_tab);

            let agent = terminal_after.effective_known_agent();
            let agent_label = terminal_after.effective_agent_label().map(str::to_string);

            debug!(
                ws_idx,
                pane_id = pane_id.raw(),
                prev_state = ?prev_state,
                new_state = ?new_state,
                agent = ?agent,
                "pane effective state changed during API request, checking notification"
            );

            if !forwarded_toast_from_state
                && self.app.state.toast_config.delay_seconds == 0
                && should_forward_toast_to_clients(self.app.state.toast_config.delivery)
            {
                if let Some(kind) =
                    crate::app::actions::notification_toast_for_state_change_with_agent_labels(
                        suppress_active_tab_notifications,
                        *prev_state,
                        new_state,
                        prev_agent_label.as_deref(),
                        agent_label.as_deref(),
                    )
                {
                    if let Some(agent_label) = self
                        .app
                        .state
                        .terminals
                        .get(&pane_after.attached_terminal_id)
                        .and_then(|terminal| terminal.effective_agent_label())
                    {
                        let event_text = match kind {
                            crate::app::state::ToastKind::NeedsAttention => "needs attention",
                            crate::app::state::ToastKind::Finished => "finished",
                            crate::app::state::ToastKind::UpdateInstalled => "updated",
                        };
                        let workspace_label = self.app.state.workspaces[*ws_idx].display_name_from(
                            &self.app.state.terminals,
                            &self.app.terminal_runtimes,
                        );
                        let context = crate::app::actions::notification_context(
                            &self.app.state.workspaces[*ws_idx],
                            &workspace_label,
                            *ws_idx,
                            *pane_id,
                        );
                        self.send_notify_to_foreground_client(
                            toast_notify_kind(self.app.state.toast_config.delivery)
                                .expect("toast forwarding requires a client notification kind"),
                            format!("{agent_label} {event_text}"),
                            non_empty_body(&context),
                        );
                    }
                }
            }

            // Forward sound notification when server-side sound policy allows it.
            // Clients still decide locally whether they can execute the side effect.
            if self.app.state.toast_config.delay_seconds == 0 && self.app.state.sound.allows(agent)
            {
                if let Some(sound) =
                    crate::app::actions::notification_sound_for_state_change_with_agent_labels(
                        suppress_active_tab_notifications,
                        *prev_state,
                        new_state,
                        prev_agent_label.as_deref(),
                        agent_label.as_deref(),
                    )
                {
                    debug!(sound = ?sound, "forwarding sound notification from API request");
                    self.send_notify_to_foreground_client(
                        protocol::NotifyKind::Sound,
                        sound_notify_message(sound),
                        None,
                    );
                }
            }
        }

        if !skip_default_workspace && latest_app_client(&self.clients).is_some() {
            changed |= self.app.ensure_default_workspace();
        }

        changed
    }

    fn stream_host_mouse_capture_mode(&mut self) {
        let enabled = self
            .app
            .state
            .should_capture_host_mouse_from(&self.app.terminal_runtimes);
        let serialized = match Self::frame_server_message(&ServerMessage::MouseCapture { enabled })
        {
            Ok(framed) => framed,
            Err(err) => {
                warn!(err = %err, "failed to serialize mouse capture mode for clients");
                return;
            }
        };

        let mut broken_clients: Vec<u64> = Vec::new();
        for (&client_id, client) in &mut self.clients {
            if !client.is_full_app_client() {
                continue;
            }
            if client.host_mouse_capture_active == Some(enabled) {
                continue;
            }
            let Some(writer) = &client.writer else {
                continue;
            };
            if writer.control.send(serialized.clone()).is_err() {
                debug!(
                    client_id,
                    "client writer channel closed during mouse capture update"
                );
                broken_clients.push(client_id);
                continue;
            }
            client.host_mouse_capture_active = Some(enabled);
        }

        for client_id in broken_clients {
            self.remove_client_and_resize_if_needed(client_id);
        }
    }

    fn stream_host_keyboard_enhancement_flags(&mut self) {
        let report_all_keys = self.app.host_keyboard_report_all_requested();
        let serialized = match Self::frame_server_message(&ServerMessage::KittyKeyboardReportAll {
            enabled: report_all_keys,
        }) {
            Ok(framed) => framed,
            Err(err) => {
                warn!(err = %err, "failed to serialize keyboard enhancement flags for clients");
                return;
            }
        };

        let mut broken_clients = Vec::new();
        for (&client_id, client) in &mut self.clients {
            if !client.is_full_app_client()
                || client.host_keyboard_report_all_active == Some(report_all_keys)
            {
                continue;
            }
            let Some(writer) = &client.writer else {
                continue;
            };
            if writer.control.send(serialized.clone()).is_err() {
                debug!(
                    client_id,
                    "client writer channel closed during keyboard enhancement update"
                );
                broken_clients.push(client_id);
                continue;
            }
            client.host_keyboard_report_all_active = Some(report_all_keys);
        }

        for client_id in broken_clients {
            self.remove_client_and_resize_if_needed(client_id);
        }
    }

    fn render_retained_pty_update_and_stream(&mut self) -> bool {
        crate::render_prof::event("retained.attempt");
        let retained_started = crate::render_prof::timer();
        macro_rules! retained_fallback {
            ($reason:literal) => {{
                crate::render_prof::event(concat!("retained_fallback.", $reason));
                crate::render_prof::duration_since("retained.total", retained_started);
                return false;
            }};
        }
        macro_rules! retained_success {
            ($reason:literal) => {{
                crate::render_prof::event("retained.success");
                crate::render_prof::event(concat!("retained_success.", $reason));
                crate::render_prof::duration_since("retained.total", retained_started);
                return true;
            }};
        }

        if !self.retained_pty_update_allowed_by_app_state() {
            retained_fallback!("unsafe_app_state");
        }

        let render_targets = render_targets(&self.clients, self.foreground_client_id);
        let [(client_id, (cols, rows), cell_size, _is_foreground, mode)] =
            render_targets.as_slice()
        else {
            retained_fallback!("multiple_or_no_target");
        };
        if !matches!(mode, ClientConnectionMode::App) {
            retained_fallback!("not_app_client");
        }
        let Some(client) = self.clients.get(client_id) else {
            retained_fallback!("client_missing");
        };
        if client.deferred_render() != DeferredRender::None {
            retained_fallback!("render_pending");
        }
        if self.app.state.kitty_graphics_enabled && !client.graphics_cache.is_empty() {
            retained_fallback!("graphics_cache_active");
        }
        if client.graphics_surface_reset_pending {
            retained_fallback!("graphics_surface_reset");
        }
        if self.app.state.kitty_graphics_enabled
            && cell_size.is_known()
            && crate::kitty_graphics::has_visible_pane_graphics(
                &self.app.state,
                &self.app.terminal_runtimes,
                self.app.state.view.tab_surface(),
                *cell_size,
            )
        {
            retained_fallback!("visible_kitty_graphics");
        }
        let Some(mut frame) = client.render_state.last_frame().cloned() else {
            retained_fallback!("no_last_frame");
        };
        if frame.width != *cols || frame.height != *rows {
            retained_fallback!("frame_size_mismatch");
        }
        frame.graphics.clear();

        let Some(ws_idx) = self.app.state.active else {
            retained_fallback!("no_active_workspace");
        };
        let pane_infos = self.app.state.view.pane_infos.clone();
        if pane_infos.is_empty() {
            retained_fallback!("no_pane_info");
        }

        let mut touched = false;
        for info in pane_infos {
            if !rect_fits_frame(info.inner_rect, &frame) {
                retained_fallback!("pane_rect_outside_frame");
            }
            let Some(runtime) = self.app.state.runtime_for_pane_in_workspace(
                &self.app.terminal_runtimes,
                ws_idx,
                info.id,
            ) else {
                retained_fallback!("missing_runtime");
            };
            match runtime.collect_dirty_patch(info.inner_rect.width, info.inner_rect.height) {
                crate::pane::TerminalDirtyPatchOutcome::Clean => {
                    crate::render_prof::event("retained.pane_clean");
                }
                crate::pane::TerminalDirtyPatchOutcome::Fallback => {
                    retained_fallback!("dirty_patch_fallback");
                }
                crate::pane::TerminalDirtyPatchOutcome::Patch(patch) => {
                    crate::render_prof::event("retained.pane_patch");
                    crate::render_prof::counter("retained.patch_rows", patch.rows.len() as u64);
                    if dirty_patch_intersects_hyperlinks(&frame, info.inner_rect, &patch) {
                        retained_fallback!("hyperlink_intersection");
                    }
                    if !apply_terminal_dirty_patch(&mut frame, info.inner_rect, patch) {
                        retained_fallback!("patch_apply_failed");
                    }
                    touched = true;
                }
            }
        }

        let previous_cursor = frame.cursor.clone();
        frame.cursor = crate::server::render_stream::focused_terminal_cursor(
            &self.app.state,
            &self.app.terminal_runtimes,
        );
        let cursor_changed = frame.cursor != previous_cursor;

        if !touched && !cursor_changed {
            retained_success!("clean_no_cursor_change");
        }

        let mut broken_clients = Vec::new();
        let sent = self.send_retained_frame_to_client(*client_id, frame, &mut broken_clients);
        for broken_client in broken_clients {
            self.remove_client_and_resize_if_needed(broken_client);
        }
        if sent {
            retained_success!("sent");
        }
        retained_fallback!("send_failed");
    }

    fn retained_pty_update_allowed_by_app_state(&self) -> bool {
        self.app.state.mode == app::Mode::Terminal
            && self.app.state.popup_pane.is_none()
            && self.app.state.selection.is_none()
            && self.app.state.copy_mode.is_none()
            && self.app.state.context_menu.is_none()
            && self.app.state.toast.is_none()
            && self.app.state.copy_feedback.is_none()
            && !self.app.full_redraw_pending
    }

    fn send_retained_frame_to_client(
        &mut self,
        client_id: u64,
        frame: FrameData,
        broken_clients: &mut Vec<u64>,
    ) -> bool {
        let Some(client) = self.clients.get_mut(&client_id) else {
            crate::render_prof::event("retained_send_fallback.client_missing");
            return false;
        };
        let Some(writer) = client.writer.as_ref().cloned() else {
            crate::render_prof::event("retained_send_fallback.writer_missing");
            return false;
        };
        let prepare_started = crate::render_prof::timer();
        let Some(prepared) = client.render_state.prepare_frame(frame) else {
            client.clear_deferred_render();
            crate::render_prof::event("retained_send.skip_identical");
            crate::render_prof::duration_since("retained_send.prepare_frame", prepare_started);
            return true;
        };
        crate::render_prof::duration_since("retained_send.prepare_frame", prepare_started);
        let serialize_started = crate::render_prof::timer();
        let serialized = match Self::frame_server_message(prepared.message()) {
            Ok(framed) => {
                crate::render_prof::duration_since("retained_send.serialize", serialize_started);
                framed
            }
            Err(protocol::FramingError::Oversized { claimed, max }) => {
                warn!(
                    client_id,
                    claimed, max, "skipping oversized retained frame for client"
                );
                crate::render_prof::event("retained_send_fallback.serialize_oversized");
                crate::render_prof::duration_since("retained_send.serialize", serialize_started);
                return false;
            }
            Err(err) => {
                warn!(client_id, err = %err, "failed to serialize retained frame for client");
                broken_clients.push(client_id);
                crate::render_prof::event("retained_send_fallback.serialize_error");
                crate::render_prof::duration_since("retained_send.serialize", serialize_started);
                return false;
            }
        };
        crate::render_prof::counter("retained_send.bytes", serialized.len() as u64);

        let send_started = crate::render_prof::timer();
        match writer.render.try_send(serialized) {
            Ok(()) => {
                client.clear_deferred_render();
                client.render_state.commit_sent_frame(prepared);
                crate::render_prof::event("retained_send.sent");
                crate::render_prof::duration_since("retained_send.try_send", send_started);
                true
            }
            Err(std::sync::mpsc::TrySendError::Full(_)) => {
                client.defer_full_render();
                crate::render_prof::event("retained_send_fallback.queue_full");
                crate::render_prof::duration_since("retained_send.try_send", send_started);
                debug!(
                    client_id,
                    "render queue full, deferring latest retained frame"
                );
                false
            }
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                debug!(client_id, "client writer channel closed, marking as broken");
                broken_clients.push(client_id);
                crate::render_prof::event("retained_send_fallback.writer_disconnected");
                crate::render_prof::duration_since("retained_send.try_send", send_started);
                false
            }
        }
    }

    /// The size each tab is negotiated to, keyed by the client whose render
    /// pass applies it.
    ///
    /// A tab is sized to the smallest display watching it, taken component by
    /// component, so a tab watched by exactly one display keeps that display's
    /// full size and only a shared tab has to compromise. Exactly one client
    /// owns the resize for a tab, so two displays cannot fight over it within
    /// one frame, and a tab nobody watches is left alone.
    ///
    /// Direct terminal attaches are not counted: they render one terminal
    /// rather than a tab, and their size is owned separately.
    ///
    /// TP-MCF-SIZE-01
    fn negotiated_tab_sizes(&mut self) -> HashMap<u64, (u16, u16)> {
        let candidates: Vec<(u64, (u16, u16))> = self
            .clients
            .iter()
            .filter(|(_, client)| client.writer.is_some() && client.is_full_app_client())
            .map(|(&client_id, client)| (client_id, client.terminal_size))
            .collect();

        // (workspace, tab) -> (negotiated cols, negotiated rows, owning
        // client). Keyed by both halves: two workspaces both have a tab at
        // index zero, and merging those entries would size a watched tab to
        // a display that is looking at a different workspace — and leave the
        // other display's tab with no negotiation at all.
        let mut per_tab: HashMap<(usize, usize), (u16, u16, u64)> = HashMap::new();
        for (client_id, (cols, rows)) in candidates {
            let previous = self.app.state.enter_viewer(Some(client_id));
            let tab = self.app.state.active.and_then(|ws_idx| {
                self.app
                    .state
                    .workspaces
                    .get(ws_idx)
                    .map(|workspace| (ws_idx, workspace.active_tab_index()))
            });
            self.app.state.restore_viewer(previous);
            let Some(tab) = tab else {
                continue;
            };

            per_tab
                .entry(tab)
                .and_modify(|(best_cols, best_rows, owner)| {
                    *best_cols = (*best_cols).min(cols);
                    *best_rows = (*best_rows).min(rows);
                    // The lowest id owns the resize so the choice does not
                    // depend on client iteration order.
                    *owner = (*owner).min(client_id);
                })
                .or_insert((cols, rows, client_id));
        }

        per_tab
            .into_values()
            .map(|(cols, rows, owner)| (owner, (cols, rows)))
            .collect()
    }

    fn render_and_stream(&mut self) {
        let full_started = crate::render_prof::timer();
        let render_targets = render_targets(&self.clients, self.foreground_client_id);
        let negotiated_tab_sizes = self.negotiated_tab_sizes();

        if render_targets.is_empty() {
            let (cols, rows) = self.effective_size;
            let area = Rect::new(0, 0, cols, rows);
            let resize_panes = self.app.state.view.pane_infos.is_empty();
            #[cfg(test)]
            {
                self.watcherless_virtual_frames += 1;
            }
            let render_started = crate::render_prof::timer();
            let _ = crate::server::render_stream::render_virtual_with_runtime_registry(
                &mut self.app.state,
                &self.app.terminal_runtimes,
                area,
                resize_panes,
                crate::kitty_graphics::HostCellSize::default(),
            );
            crate::render_prof::duration_since("full_render.render_virtual", render_started);
            self.app.full_redraw_pending = false;
            crate::render_prof::duration_since("full_render.total", full_started);
            debug!(
                cols,
                rows, resize_panes, "rendered virtual frame with no attached clients"
            );
            return;
        }

        let mut broken_clients: Vec<u64> = Vec::new();
        let mut deferred_frame = false;
        // Pane size no longer follows the foreground client: it follows the
        // set of displays watching each tab. TP-MCF-SIZE-01
        for (client_id, (cols, rows), cell_size, _is_foreground, mode) in render_targets {
            let area = Rect::new(0, 0, cols, rows);
            // Filled inside the App arm, where the encode runs in this
            // client's viewer window; committed after a successful send.
            let mut encoded_graphics_cache: Option<crate::kitty_graphics::HostGraphicsCache> = None;
            let mut attach_wheel_routing: Option<crate::protocol::TerminalWheelRouting> = None;
            let mut frame = match mode {
                ClientConnectionMode::App => {
                    // Render resolves this client's view for the whole arm.
                    // The arm has no early exit, so the single restore below
                    // always runs. TP-MCF-CTX-03
                    let previous_viewer = self.app.state.enter_viewer(Some(client_id));
                    let render_started = crate::render_prof::timer();
                    let render_cell_size =
                        if self.app.state.kitty_graphics_enabled && cell_size.is_known() {
                            cell_size
                        } else {
                            crate::kitty_graphics::HostCellSize::default()
                        };
                    // Resize this client's tab first when it owns the size,
                    // then draw at the client's own area without resizing
                    // again. Splitting the two is what lets a tab be sized to
                    // the smallest display watching it while every display
                    // still draws at its own dimensions.
                    let negotiated = negotiated_tab_sizes.get(&client_id).copied();
                    // A tab only this display is watching negotiates to this
                    // display's own size, so the resize can happen inside the
                    // render pass exactly as it always did. The separate pass
                    // is only for a shared tab, where the negotiated size is
                    // not the size being drawn -- that is the one case herdr
                    // cannot express with a single area.
                    let resize_while_rendering = negotiated == Some((cols, rows));
                    if let Some((resize_cols, resize_rows)) = negotiated {
                        if !resize_while_rendering {
                            crate::ui::compute_view_skipping_background_tabs(
                                &mut self.app.state,
                                &self.app.terminal_runtimes,
                                Rect::new(0, 0, resize_cols, resize_rows),
                                render_cell_size,
                            );
                        }
                    }
                    let (buffer, cursor) =
                        crate::server::render_stream::render_virtual_with_runtime_registry(
                            &mut self.app.state,
                            &self.app.terminal_runtimes,
                            area,
                            resize_while_rendering,
                            render_cell_size,
                        );
                    self.view_owner = Some((client_id, (cols, rows)));
                    crate::render_prof::duration_since(
                        "full_render.render_virtual",
                        render_started,
                    );
                    let hyperlinks_started = crate::render_prof::timer();
                    let hyperlinks = crate::server::render_stream::visible_hyperlinks(
                        &self.app.state,
                        &self.app.terminal_runtimes,
                    );
                    crate::render_prof::duration_since(
                        "full_render.visible_hyperlinks",
                        hyperlinks_started,
                    );
                    let frame_started = crate::render_prof::timer();
                    let mut frame = FrameData::from_ratatui_buffer_with_hyperlinks(
                        &buffer,
                        cursor,
                        &hyperlinks,
                    );
                    crate::render_prof::duration_since("full_render.frame_build", frame_started);
                    // Graphics are derived from this client's view exactly as
                    // the text cells are, so they are encoded inside the same
                    // viewer window the frame was rendered in. Encoding after
                    // the restore reads whichever view the restore installed —
                    // with several displays that is the session default, whose
                    // owned surfaces hold no file browser, so the preview a
                    // display was looking at never reached it. TP-MCF-CTX-06
                    if let Some(client) = self.clients.get_mut(&client_id) {
                        let mut next_graphics_cache = client.graphics_cache.clone();
                        if self.app.state.kitty_graphics_enabled && cell_size.is_known() {
                            if client.graphics_surface_reset_pending {
                                frame.graphics = next_graphics_cache.clear_bytes();
                            }
                            let graphics_started = crate::render_prof::timer();
                            frame.graphics.extend(
                                crate::kitty_graphics::encode_local_pane_graphics(
                                    &self.app.state,
                                    &self.app.terminal_runtimes,
                                    self.app.state.view.tab_surface(),
                                    cell_size,
                                    &mut next_graphics_cache,
                                ),
                            );
                            crate::render_prof::duration_since(
                                "full_render.graphics_encode",
                                graphics_started,
                            );
                        } else {
                            frame.graphics = next_graphics_cache.clear_bytes();
                        }
                        encoded_graphics_cache = Some(next_graphics_cache);
                    }
                    self.app.state.restore_viewer(previous_viewer);
                    frame
                }
                ClientConnectionMode::TerminalAttach { terminal_id }
                | ClientConnectionMode::TerminalObserve { terminal_id } => {
                    let Some(runtime) = self.runtime_for_terminal_id_string(&terminal_id) else {
                        self.send_to_client(
                            client_id,
                            ServerMessage::ServerShutdown {
                                reason: Some(format!(
                                    "terminal attach ended: terminal {terminal_id} not found"
                                )),
                            },
                        );
                        broken_clients.push(client_id);
                        continue;
                    };
                    // The routing travels with the render tick: a mode flip
                    // (vim opening, claude entering its alt screen) always
                    // redraws, so piggybacking on the tick catches every
                    // change a client could observe — and the None-to-Some
                    // edge on the first tick doubles as the attach greeting.
                    attach_wheel_routing = runtime
                        .wheel_routing()
                        .map(crate::protocol::TerminalWheelRouting::from);
                    let render_started = crate::render_prof::timer();
                    let (buffer, cursor) =
                        crate::server::render_stream::render_terminal_virtual(runtime, area);
                    crate::render_prof::duration_since(
                        "full_render.render_terminal_virtual",
                        render_started,
                    );
                    let hyperlinks_started = crate::render_prof::timer();
                    let hyperlinks = runtime.visible_hyperlinks(area);
                    crate::render_prof::duration_since(
                        "full_render.visible_hyperlinks",
                        hyperlinks_started,
                    );
                    let frame_started = crate::render_prof::timer();
                    let frame = FrameData::from_ratatui_buffer_with_hyperlinks(
                        &buffer,
                        cursor,
                        &hyperlinks,
                    );
                    crate::render_prof::duration_since("full_render.frame_build", frame_started);
                    frame
                }
            };

            if let Some(now) = attach_wheel_routing {
                let already = self
                    .clients
                    .get(&client_id)
                    .and_then(|client| client.terminal_wheel_routing_sent);
                if already != Some(now) {
                    self.send_to_client(client_id, ServerMessage::TerminalRouting { routing: now });
                    if let Some(client) = self.clients.get_mut(&client_id) {
                        client.terminal_wheel_routing_sent = Some(now);
                    }
                }
            }

            let Some(client) = self.clients.get_mut(&client_id) else {
                continue;
            };
            // App frames arrive with their graphics already encoded (inside
            // the viewer window, above). Everything else carries none and
            // clears its cache exactly as before.
            let next_graphics_cache = match encoded_graphics_cache.take() {
                Some(cache) => cache,
                None => {
                    let mut cache = client.graphics_cache.clone();
                    frame.graphics = cache.clear_bytes();
                    cache
                }
            };

            let Some(writer) = client.writer.as_ref().cloned() else {
                crate::render_prof::event("full_render.writer_missing");
                continue;
            };

            let mut commit_graphics_cache = true;
            if frame.graphics.len() > MAX_GRAPHICS_FRAME_SIZE {
                warn!(
                    client_id,
                    graphics_bytes = frame.graphics.len(),
                    max = MAX_GRAPHICS_FRAME_SIZE,
                    "dropping oversized graphics payload for client frame"
                );
                frame.graphics.clear();
                commit_graphics_cache = false;
            }

            let max_frame_size = if frame.graphics.is_empty() {
                MAX_FRAME_SIZE
            } else {
                MAX_GRAPHICS_FRAME_SIZE
            };
            let has_graphics = !frame.graphics.is_empty();
            let prepare_started = crate::render_prof::timer();
            let Some(mut prepared) = client.render_state.prepare_frame(frame) else {
                client.clear_deferred_render();
                crate::render_prof::event("full_render.skip_identical");
                crate::render_prof::duration_since("full_render.prepare_frame", prepare_started);
                continue;
            };
            crate::render_prof::duration_since("full_render.prepare_frame", prepare_started);

            let serialize_started = crate::render_prof::timer();
            let serialized = match Self::frame_server_message_with_max(
                prepared.message(),
                max_frame_size,
            ) {
                Ok(framed) => {
                    crate::render_prof::duration_since("full_render.serialize", serialize_started);
                    framed
                }
                Err(protocol::FramingError::Oversized { claimed, max }) if has_graphics => {
                    warn!(
                        client_id,
                        claimed, max, "dropping graphics from oversized frame for client"
                    );
                    let Some(mut text_only_frame) = prepared.into_frame() else {
                        crate::render_prof::event("full_render.serialize_error");
                        crate::render_prof::duration_since(
                            "full_render.serialize",
                            serialize_started,
                        );
                        continue;
                    };
                    text_only_frame.graphics.clear();
                    let Some(text_only_prepared) =
                        client.render_state.prepare_frame(text_only_frame)
                    else {
                        client.clear_deferred_render();
                        crate::render_prof::event("full_render.skip_identical_text_only");
                        crate::render_prof::duration_since(
                            "full_render.serialize",
                            serialize_started,
                        );
                        continue;
                    };
                    let framed = match Self::frame_server_message(text_only_prepared.message()) {
                        Ok(framed) => framed,
                        Err(err) => {
                            warn!(client_id, err = %err, "failed to serialize text-only frame for client");
                            broken_clients.push(client_id);
                            crate::render_prof::event("full_render.serialize_error");
                            crate::render_prof::duration_since(
                                "full_render.serialize",
                                serialize_started,
                            );
                            continue;
                        }
                    };
                    prepared = text_only_prepared;
                    commit_graphics_cache = false;
                    crate::render_prof::duration_since("full_render.serialize", serialize_started);
                    framed
                }
                Err(protocol::FramingError::Oversized { claimed, max }) => {
                    warn!(
                        client_id,
                        claimed, max, "skipping oversized frame for client"
                    );
                    crate::render_prof::event("full_render.serialize_oversized");
                    crate::render_prof::duration_since("full_render.serialize", serialize_started);
                    continue;
                }
                Err(err) => {
                    warn!(client_id, err = %err, "failed to serialize frame for client");
                    broken_clients.push(client_id);
                    crate::render_prof::event("full_render.serialize_error");
                    crate::render_prof::duration_since("full_render.serialize", serialize_started);
                    continue;
                }
            };
            crate::render_prof::counter("full_render.bytes", serialized.len() as u64);

            let send_started = crate::render_prof::timer();
            match writer.render.try_send(serialized) {
                Ok(()) => {
                    client.clear_deferred_render();
                    if commit_graphics_cache {
                        client.graphics_cache = next_graphics_cache;
                        client.graphics_surface_reset_pending = false;
                    }
                    client.render_state.commit_sent_frame(prepared);
                    crate::render_prof::event("full_render.sent");
                    crate::render_prof::duration_since("full_render.try_send", send_started);
                }
                Err(std::sync::mpsc::TrySendError::Full(_)) => {
                    client.defer_full_render();
                    deferred_frame = true;
                    crate::render_prof::event("full_render.queue_full");
                    crate::render_prof::duration_since("full_render.try_send", send_started);
                    debug!(client_id, "render queue full, deferring latest frame");
                    continue;
                }
                Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                    debug!(client_id, "client writer channel closed, marking as broken");
                    broken_clients.push(client_id);
                    crate::render_prof::event("full_render.writer_disconnected");
                    crate::render_prof::duration_since("full_render.try_send", send_started);
                    continue;
                }
            }
        }

        if !broken_clients.is_empty() {
            for client_id in broken_clients {
                self.remove_client_and_resize_if_needed(client_id);
            }
        }

        let (cols, rows) = self.effective_size;
        if !deferred_frame {
            self.app.full_redraw_pending = false;
        }
        crate::render_prof::duration_since("full_render.total", full_started);
        debug!(cols, rows, foreground_client_id = ?self.foreground_client_id, "rendered virtual frame(s)");
    }

    /// Handle scheduled tasks for the headless server.
    ///
    /// Similar to `App::handle_scheduled_tasks` but without resize polling
    /// (the server doesn't have a terminal to resize).
    fn handle_scheduled_tasks_headless(&mut self, now: Instant, geometry_dirty: bool) -> bool {
        let mut changed = false;

        // Same set, same order as the monolithic loop (`App::handle_scheduled_
        // tasks`); the parity test compares the two by name. These are the
        // consumers of queued user intents — context-menu actions, agent
        // handoffs, the reference picker — and skipping any of them in server
        // mode means a menu entry that closes the menu and then silently does
        // nothing, which is exactly how the missing file-operation sync was
        // found: "Send with Tailscale" queued its intent and no one ever read
        // it.
        changed |= self.app.sync_file_operation_worker();
        changed |= self.app.sync_file_manager_agent_handoff_send();
        changed |= self.app.sync_agent_attachment_delivery();
        // Each display browses its own directory, so the file workers run
        // once per display, inside that display's view. TP-SUR-FM-02
        changed |= self.app.for_each_display(|app| {
            let mut changed = false;
            // Three consumers compete for one context-action field, and the
            // order between them is the precedence: send-to-agent, then a
            // plugin, then the ordinary actions. All three resolve against
            // the raising display's browser, so all three belong in its view.
            changed |= app.sync_file_manager_agent_handoff();
            changed |= app.sync_file_manager_plugin_action();
            changed |= app.sync_agent_reference_picker();
            changed |= app.sync_file_manager_requests();
            changed |= app.sync_file_manager_io_results();
            changed |= app.sync_file_manager_location_request();
            changed |= app.sync_file_manager_watcher_at(now);
            changed |= app.sync_file_preview_worker();
            // The monolithic loop pairs these two (`src/app/mod.rs`): text and
            // image previews are both bounded workers and neither advances
            // without being driven.
            changed |= app.sync_image_preview_worker();
            changed
        });
        self.app.sync_headless_animation_timer(now);
        changed |= self.app.refresh_projects_if_due(now);
        changed |= self.app.refresh_tab_branches_if_due(now);
        // No presentation surface reads preview_bindings yet, so a refresh
        // must not dirty the frame (flip to `changed |=` once a marker renders).
        let _ = self.app.refresh_preview_bindings_if_due(now);

        // No resize polling needed — server has no terminal.
        // Client resize messages drive size changes instead.

        if self
            .app
            .config_diagnostic_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.config_diagnostic_deadline = None;
            self.app.state.config_diagnostic = None;
            changed = true;
        }

        if self
            .app
            .toast_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.toast_deadline = None;
            self.app.state.toast = None;
            changed = true;
        }

        if self
            .app
            .state
            .next_pending_agent_notification_deadline()
            .is_some_and(|deadline| now >= deadline)
        {
            let previous_toast = self.app.state.toast.clone();
            let mut deliveries = self.app.state.drain_due_agent_notifications(now);
            if !deliveries.is_empty() {
                self.app
                    .refresh_agent_notification_delivery_contexts(&mut deliveries);
                self.app.sync_toast_deadline(previous_toast);
                for delivery in &deliveries {
                    self.forward_agent_notification_delivery(delivery);
                }
                changed = true;
            }
        }

        if self
            .app
            .copy_feedback_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.copy_feedback_deadline = None;
            self.app.state.copy_feedback = None;
            changed = true;
        }

        if self
            .app
            .next_animation_tick
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.state.spinner_tick = self
                .app
                .state
                .spinner_tick
                .wrapping_add(app::HEADLESS_ANIMATION_TICK_STEP);
            self.app.next_animation_tick = Some(now + app::HEADLESS_ANIMATION_INTERVAL);
            changed = true;
        }

        if self
            .app
            .selection_autoscroll_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.tick_selection_autoscroll(now);
            changed = true;
        }

        changed |= self.app.clear_due_selection_highlight(now);

        // This is the loop that actually runs under a live herdr: the server
        // owns the state the screen is drawn from, and `App::handle_scheduled_
        // tasks` is only reached by the monolithic path. Landing the sampler in
        // that one alone left every test green, the binary correct, and the bar
        // showing `--` forever, because the code that read the machine was
        // never executed by the process that drew it.
        // TP-RES-11: the sampler runs in the loop that actually renders.
        changed |= self.app.tick_resource_sample(now);

        if self.has_app_client() {
            self.app.start_git_status_refresh_if_due(now);
        }

        if self
            .app
            .next_auto_update_check
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.run_auto_update_check();
        }

        if self
            .app
            .next_agent_manifest_update_check
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.run_agent_manifest_update_check();
        }

        if self
            .app
            .session_save_deadline
            .is_some_and(|deadline| now >= deadline)
        {
            self.app.start_background_session_save();
        }

        if let Some(deadline) = self
            .app
            .agent_metadata_deadline
            .filter(|deadline| now >= *deadline)
        {
            self.app.expire_metadata_at(deadline, now);
            changed = true;
        }

        if geometry_dirty || self.foreground_client_id.is_none() {
            self.app.pending_agent_resume_deadline = None;
        } else {
            self.app.sync_pending_agent_resume_deadline(now);
            changed |= self
                .app
                .start_pending_agent_resumes(self.app.pending_agent_resume_due(now));
        }
        self.app.sync_headless_animation_timer(now);
        changed
    }

    /// Initiates graceful shutdown.
    fn initiate_shutdown(&mut self) {
        if self.shutting_down {
            return;
        }
        info!("server shutdown initiated");
        self.shutting_down = true;

        // Clear client-local host graphics, then send ServerShutdown to all connected clients.
        self.send_all_clients_graphics_cleanup();
        let shutdown_msg = ServerMessage::ServerShutdown {
            reason: Some("server is shutting down".to_owned()),
        };
        self.send_to_all_clients(shutdown_msg);

        // Give client writer threads a moment to flush the shutdown message.
        // A short sleep ensures the message is written to the socket before
        // we close the connections.
        std::thread::sleep(Duration::from_millis(50));

        // Signal the main loop to exit.
        self.should_quit.store(true, Ordering::Release);
        self.app.state.should_quit = true;
    }

    /// Completes the shutdown sequence: send ServerShutdown to clients,
    /// close client connections, remove socket files, and clean up.
    fn complete_shutdown(&mut self) -> io::Result<()> {
        info!("completing server shutdown");

        // Send ServerShutdown to all remaining clients.
        if !self.clients.is_empty() {
            self.send_all_clients_graphics_cleanup();
            let shutdown_msg = ServerMessage::ServerShutdown {
                reason: Some("server is shutting down".to_owned()),
            };
            self.send_to_all_clients(shutdown_msg);

            // Give writer threads a moment to flush before closing.
            std::thread::sleep(Duration::from_millis(50));
        }

        // Drain remaining API requests with server_unavailable.
        self.drain_api_requests_with_shutdown_check();

        // Close all client connections.
        let staged_files = self
            .clients
            .drain()
            .flat_map(|(_, client)| client.staged_clipboard_files)
            .collect::<Vec<_>>();
        crate::server::clipboard_image::remove_files(staged_files);

        // Remove socket files.
        self.cleanup_sockets()?;

        Ok(())
    }

    /// Removes socket files created by the server.
    fn cleanup_sockets(&self) -> io::Result<()> {
        if let Err(err) =
            remove_socket_file_if_owned(&self.client_socket_path, &self.client_socket_identity)
        {
            if err.kind() != io::ErrorKind::NotFound {
                warn!(
                    path = %self.client_socket_path.display(),
                    err = %err,
                    "failed to remove client socket on shutdown"
                );
            }
        }
        Ok(())
    }
}

fn events_for_app_routing(
    events: Vec<crate::raw_input::RawInputEvent>,
    mut source_is_foreground: bool,
    source_is_full_app: bool,
) -> Vec<crate::raw_input::RawInputEvent> {
    events
        .into_iter()
        .filter_map(|event| match event {
            crate::raw_input::RawInputEvent::OuterFocusGained
            | crate::raw_input::RawInputEvent::OuterFocusLost
                if !source_is_full_app =>
            {
                None
            }
            crate::raw_input::RawInputEvent::OuterFocusGained => {
                source_is_foreground = true;
                Some(event)
            }
            crate::raw_input::RawInputEvent::OuterFocusLost if !source_is_foreground => None,
            crate::raw_input::RawInputEvent::Key(_)
            | crate::raw_input::RawInputEvent::Mouse(_)
            | crate::raw_input::RawInputEvent::Paste(_) => {
                source_is_foreground = true;
                Some(event)
            }
            _ => Some(event),
        })
        .collect()
}

impl Drop for HeadlessServer {
    fn drop(&mut self) {
        let staged_files = self
            .clients
            .drain()
            .flat_map(|(_, client)| client.staged_clipboard_files)
            .collect::<Vec<_>>();
        crate::server::clipboard_image::remove_files(staged_files);
        let _ = self.cleanup_sockets();
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Installs a Ctrl+C handler that sets the should_quit flag and wakes up
/// the event loop by sending a QuitSignal on the server event channel.
fn ctrlc_handler(should_quit: Arc<AtomicBool>, server_event_tx: mpsc::Sender<ServerEvent>) {
    let _ = ctrlc::set_handler(move || {
        should_quit.store(true, Ordering::Release);
        // Wake up the event loop so the quit flag is checked promptly.
        let _ = server_event_tx.try_send(ServerEvent::QuitSignal);
    });
}

/// Sleep until a deadline, or return pending if none.
async fn sleep_until_or_pending(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)).await,
        None => std::future::pending().await,
    }
}

fn sanitize_notification_text(value: &str, max_chars: usize) -> Option<String> {
    let mut sanitized = String::new();
    let mut previous_space = false;
    for ch in value.chars() {
        let replacement = if ch == '\n' || ch == '\r' || ch == '\t' {
            Some(' ')
        } else if ch.is_control() {
            None
        } else {
            Some(ch)
        };
        let Some(ch) = replacement else {
            continue;
        };
        if ch.is_whitespace() {
            if previous_space {
                continue;
            }
            previous_space = true;
            sanitized.push(' ');
        } else {
            previous_space = false;
            sanitized.push(ch);
        }
        if sanitized.chars().count() >= max_chars {
            break;
        }
    }
    let sanitized = sanitized.trim().to_string();
    (!sanitized.is_empty()).then_some(sanitized)
}

fn sanitize_window_title_text(value: &str, max_chars: usize) -> Option<String> {
    let sanitized = value
        .chars()
        .filter(|ch| !matches!(*ch, '\u{1b}' | '\u{7}' | '\u{9c}') && !ch.is_control())
        .take(max_chars)
        .collect::<String>()
        .trim()
        .to_string();
    (!sanitized.is_empty()).then_some(sanitized)
}

fn server_config_diagnostic_summaries(diagnostics: &[String]) -> (Option<String>, Option<String>) {
    let without_keybindings = diagnostics
        .iter()
        .filter(|diagnostic| !is_keybinding_config_diagnostic(diagnostic))
        .cloned()
        .collect::<Vec<_>>();
    (
        config::config_diagnostic_summary(diagnostics),
        config::config_diagnostic_summary(&without_keybindings),
    )
}

fn is_keybinding_config_diagnostic(diagnostic: &str) -> bool {
    diagnostic.contains("keybinding") || diagnostic.contains("keys.")
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Run the headless server. This is the entry point called from main.rs.
pub fn run_server() -> io::Result<()> {
    init_logging();
    crate::platform::raise_server_nofile_limit();

    let args: Vec<String> = std::env::args().collect();
    if args.get(2).map(String::as_str) == Some("--handoff-import") {
        let socket_path = args
            .get(3)
            .map(PathBuf::from)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing handoff socket"))?;
        let token = args
            .get(4)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "missing handoff token"))?;
        return run_handoff_import_server(&socket_path, token);
    }

    let loaded_config = config::Config::load();
    let (api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
    let event_hub = api::EventHub::default();

    // Start the JSON API socket server.
    let _api_server = match api::start_server(api_tx.clone(), event_hub.clone()) {
        Ok(server) => server,
        Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
            eprintln!("error: herdr server is already running");
            eprintln!("api socket: {}", api::socket_path().display());
            std::process::exit(1);
        }
        Err(err) => return Err(err),
    };

    let no_session = false; // Server always does session persistence.

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(io::Error::other)?;

    let result = rt.block_on(async {
        // Create the App (with AppState, event channels, etc.).
        let mut app = app::App::new(
            &loaded_config.config,
            no_session,
            config::config_diagnostic_summary(&loaded_config.diagnostics),
            api_rx,
            event_hub,
        );
        seed_startup_workspace_if_empty(&mut app);

        // The server runs headless — disable local notification side effects.
        // Sound and terminal notifications are forwarded to connected clients
        // as ServerMessage::Notify instead of emitted by the server process.
        // The prefix input-source switch is likewise forwarded to the foreground
        // client (ServerMessage::PrefixInputSource), never applied in-process.
        app.state.local_sound_playback = false;
        app.local_terminal_notifications = false;
        app.local_input_source_switch = false;

        // Create the headless server.
        let mut server = match HeadlessServer::new(
            app,
            &loaded_config.diagnostics,
            Some(api_tx.clone()),
            Some(_api_server),
        ) {
            Ok(server) => server,
            Err(err) if err.kind() == io::ErrorKind::AddrInUse => {
                eprintln!("error: herdr server is already running");
                eprintln!("client socket: {}", client_socket_path().display());
                std::process::exit(1);
            }
            Err(err) => return Err(err),
        };

        info!(
            api_socket = %api::socket_path().display(),
            client_socket = %client_socket_path().display(),
            "herdr server started"
        );
        print_ready_message(&api::socket_path(), &client_socket_path());
        server.app.run_plugin_startup_hooks();

        server.run().await
    });

    rt.shutdown_timeout(Duration::from_millis(100));
    crate::logging::shutdown("server");
    result
}

fn seed_startup_workspace_if_empty(app: &mut app::App) {
    let Some(cwd) = take_startup_cwd() else {
        return;
    };

    if !app.state.workspaces.is_empty() {
        info!(
            cwd = %cwd.display(),
            "restored session already has workspaces; ignoring startup cwd"
        );
        return;
    }

    match app.create_workspace_with_options(cwd.clone(), true) {
        Ok(_) => {
            info!(cwd = %cwd.display(), "created startup workspace");
        }
        Err(err) => {
            warn!(cwd = %cwd.display(), err = %err, "failed to create startup workspace");
            app.state.mode = app::Mode::Navigate;
        }
    }
}

fn take_startup_cwd() -> Option<PathBuf> {
    let cwd = std::env::var_os(crate::server::autodetect::STARTUP_CWD_ENV_VAR)?;
    std::env::remove_var(crate::server::autodetect::STARTUP_CWD_ENV_VAR);
    (!cwd.is_empty()).then(|| PathBuf::from(cwd))
}

#[cfg(unix)]
fn run_handoff_import_server(socket_path: &Path, token: &str) -> io::Result<()> {
    let loaded_config = config::Config::load();
    let mut received = crate::server::handoff::receive(socket_path, token)?;
    crate::server::handoff::log_import_result(received.manifest.panes.len());

    let (api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
    let event_hub = api::EventHub::default();

    let mut imports = HashMap::new();
    for (pane, fd) in received.manifest.panes.into_iter().zip(received.fds) {
        let pane_id = pane.pane_id;
        imports.insert(
            pane_id,
            crate::handoff_runtime::ImportedHandoffRuntime {
                master_fd: fd,
                state: pane,
            },
        );
    }

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(io::Error::other)?;

    let result = rt.block_on(async {
        let mut app = app::App::new_from_handoff(
            &loaded_config.config,
            config::config_diagnostic_summary(&loaded_config.diagnostics),
            api_rx,
            event_hub.clone(),
            &received.manifest.snapshot,
            &mut imports,
        )?;
        app.state.local_sound_playback = false;
        app.local_terminal_notifications = false;
        app.local_input_source_switch = false;
        crate::server::handoff::report_restored(&mut received.stream)?;
        if std::env::var("HERDR_TEST_HANDOFF_IMPORT_FAIL").as_deref() == Ok("after_restored") {
            return Err(io::Error::other(
                "test handoff import failure after restored",
            ));
        }
        wait_for_old_public_sockets_to_close(Duration::from_secs(5))?;

        let api_server = api::start_server(api_tx.clone(), event_hub.clone())?;
        let mut server = HeadlessServer::new(
            app,
            &loaded_config.diagnostics,
            Some(api_tx.clone()),
            Some(api_server),
        )?;
        crate::server::handoff::report_ready(&mut received.stream)?;
        crate::server::handoff::wait_committed(&mut received.stream)?;
        server.app.assume_handoff_ownership();
        server.app.unpause_handoff_readers();
        server.pending_handoff_repaint_nudge = true;
        if let Err(err) = crate::server::handoff::report_owned(&mut received.stream) {
            warn!(err = %err, "failed to report handoff ownership; continuing as owner");
        }
        info!("handoff import server started");
        print_ready_message(&api::socket_path(), &client_socket_path());
        server.app.run_plugin_startup_hooks();
        server.run().await
    });

    rt.shutdown_timeout(Duration::from_millis(100));
    crate::logging::shutdown("server");
    result
}

#[cfg(unix)]
fn wait_for_old_public_sockets_to_close(timeout: Duration) -> io::Result<()> {
    let deadline = Instant::now() + timeout;
    let api_socket = api::socket_path();
    let client_socket = client_socket_path();
    while Instant::now() < deadline {
        let api_open = api_socket.exists() && crate::ipc::connect_local_stream(&api_socket).is_ok();
        let client_open =
            client_socket.exists() && crate::ipc::connect_local_stream(&client_socket).is_ok();
        if !api_open && !client_open {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "old server sockets did not close before handoff import bind",
    ))
}

#[cfg(not(unix))]
fn run_handoff_import_server(_socket_path: &Path, _token: &str) -> io::Result<()> {
    Err(io::Error::other("live handoff is only supported on Unix"))
}

fn print_ready_message(api_socket: &Path, client_socket: &Path) {
    eprintln!("herdr server running; you can use any herdr CLI command in another terminal.");
    eprintln!("api socket: {}", api_socket.display());
    eprintln!("client socket: {}", client_socket.display());
    eprintln!(
        "logs: {}",
        crate::session::data_dir()
            .join("herdr-server.log")
            .display()
    );
    eprintln!("did you mean to open the Herdr TUI? run `herdr`; you do not need `herdr server`.");
}

/// Initialize logging for the server process.
fn init_logging() {
    crate::logging::init_file_logging("herdr-server.log");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::AppState;
    use crate::protocol::CursorState;

    #[path = "pane_graphics.rs"]
    mod pane_graphics_tests;

    fn test_headless_server() -> HeadlessServer {
        test_headless_server_with_event_hub(api::EventHub::default())
    }

    fn test_headless_server_with_event_hub(event_hub: api::EventHub) -> HeadlessServer {
        let config = crate::config::Config::default();
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(&config, true, None, api_rx, event_hub);
        app.state.local_sound_playback = false;
        app.local_terminal_notifications = false;
        app.local_input_source_switch = false;

        let dir = std::env::temp_dir().join(format!(
            "hh-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::create_dir_all(&dir);
        let socket_path = dir.join("client.sock");
        let _ = fs::remove_file(&socket_path);
        let listener = bind_local_listener(&socket_path).expect("bind test listener");
        let client_socket_identity =
            socket_file_identity(&socket_path).expect("test listener socket identity");
        #[cfg(unix)]
        listener
            .set_nonblocking(ListenerNonblockingMode::Accept)
            .expect("set listener nonblocking");
        let (server_event_tx, server_event_rx) = mpsc::channel(64);
        #[cfg(windows)]
        let should_quit = Arc::new(AtomicBool::new(false));
        #[cfg(windows)]
        spawn_windows_client_accept_thread(listener, should_quit.clone(), server_event_tx.clone());
        let server_keybindings = app_keybindings(&app);

        HeadlessServer {
            app,
            #[cfg(unix)]
            api_tx: None,
            api_server: None,
            #[cfg(unix)]
            client_listener: listener,
            client_socket_path: socket_path,
            client_socket_identity,
            clients: HashMap::new(),
            #[cfg(unix)]
            next_client_id: 1,
            foreground_client_id: None,
            view_owner: None,
            #[cfg(test)]
            view_recomputes_for_input: 0,
            #[cfg(test)]
            watcherless_virtual_frames: 0,
            server_keybindings,
            server_config_diagnostic: None,
            server_config_diagnostic_without_keybindings: None,
            terminal_attach_owners: HashMap::new(),
            next_activity_stamp: 1,
            effective_size: (MIN_COLS, MIN_ROWS),
            shutting_down: false,
            handoff_in_progress: false,
            #[cfg(unix)]
            pending_handoff_repaint_nudge: false,
            #[cfg(unix)]
            should_quit: Arc::new(AtomicBool::new(false)),
            #[cfg(windows)]
            should_quit,
            server_event_rx,
            server_event_tx,
        }
    }

    fn shutdown_test_runtimes(server: &mut HeadlessServer) {
        for (_, runtime) in server.app.terminal_runtimes.drain() {
            runtime.shutdown();
        }
    }

    fn read_server_message(bytes: Vec<u8>) -> ServerMessage {
        let mut cursor = std::io::Cursor::new(bytes);
        protocol::read_message(&mut cursor, MAX_FRAME_SIZE).expect("decode server message")
    }

    fn read_server_frame(bytes: Vec<u8>) -> FrameData {
        match read_server_message(bytes) {
            ServerMessage::Frame(frame) => frame,
            other => panic!("expected frame, got {other:?}"),
        }
    }

    fn frame_text(frame: &FrameData) -> String {
        frame
            .cells
            .chunks(usize::from(frame.width))
            .map(|row| {
                row.iter()
                    .map(|cell| cell.symbol.as_str())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn read_server_shutdown_reason(bytes: Vec<u8>) -> Option<String> {
        match read_server_message(bytes) {
            ServerMessage::ServerShutdown { reason } => reason,
            other => panic!("expected shutdown, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn headless_api_reads_latest_title_without_spinner_event_flooding() {
        let event_hub = api::EventHub::default();
        let mut server = test_headless_server_with_event_hub(event_hub.clone());
        server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        server.app.state.ensure_test_terminals();
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        server.app.state.sidebar_agents.rows = vec![vec![
            crate::config::AgentSidebarToken::TerminalTitleStripped,
        ]];
        let pane_id = server.app.state.workspaces[0].tabs[0].root_pane;
        let terminal_id = server.app.state.workspaces[0].tabs[0].panes[&pane_id]
            .attached_terminal_id
            .clone();
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .detected_agent = Some(crate::detect::Agent::Claude);
        let runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"");
        runtime.test_process_pty_bytes(b"\x1b]0;\xe2\xa0\x8b task\x07");
        server
            .app
            .terminal_runtimes
            .insert(terminal_id.clone(), runtime);

        let first = headless_pane_list(&mut server).pop().unwrap();
        assert_eq!(first.terminal_title.as_deref(), Some("⠋ task"));
        assert_eq!(first.terminal_title_stripped.as_deref(), Some("task"));
        assert_eq!(pane_updated_events(&event_hub), 1);
        let (buffer, _) = crate::server::render_stream::render_virtual_with_runtime_registry(
            &mut server.app.state,
            &server.app.terminal_runtimes,
            Rect::new(0, 0, 100, 30),
            true,
            crate::kitty_graphics::HostCellSize::default(),
        );
        let rendered = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(rendered.contains("task"), "rendered frame: {rendered:?}");

        server
            .app
            .terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_process_pty_bytes(b"\x1b]2;\xe2\xa0\x99 task\x1b\\");
        let second = headless_pane_list(&mut server).pop().unwrap();
        assert_eq!(second.terminal_title.as_deref(), Some("⠙ task"));
        assert_eq!(second.terminal_title_stripped.as_deref(), Some("task"));
        assert_eq!(pane_updated_events(&event_hub), 1);
    }

    fn headless_pane_list(server: &mut HeadlessServer) -> Vec<api::schema::PaneInfo> {
        let (respond_to, response_rx) = std::sync::mpsc::channel();
        server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
            request: api::schema::Request {
                id: "list-titles".into(),
                method: api::schema::Method::PaneList(api::schema::PaneListParams::default()),
            },
            respond_to,
            response_write_complete: None,
        });
        let response: api::schema::SuccessResponse =
            serde_json::from_str(&response_rx.recv().unwrap()).unwrap();
        let api::schema::ResponseResult::PaneList { panes } = response.result else {
            panic!("expected pane list");
        };
        panes
    }

    fn pane_updated_events(event_hub: &api::EventHub) -> usize {
        event_hub
            .events_after(0)
            .iter()
            .filter(|(_, event)| event.event == api::schema::EventKind::PaneUpdated)
            .count()
    }

    #[test]
    fn headless_api_request_drains_all_pending_internal_events_before_reading_state() {
        let mut server = test_headless_server();
        for i in 0..=crate::app::APP_EVENT_DRAIN_LIMIT {
            server
                .app
                .event_tx
                .try_send(AppEvent::UpdateReady {
                    version: format!("4.0.{i}"),
                    install_command: "herdr install".into(),
                })
                .unwrap();
        }

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        assert!(
            server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
                request: api::schema::Request {
                    id: "headless_stop_after_events".into(),
                    method: api::schema::Method::ServerStop(
                        api::schema::ServerStopParams::default()
                    ),
                },
                respond_to,
                response_write_complete: None,
            })
        );
        let response = response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
        let response: serde_json::Value = serde_json::from_str(&response).unwrap();

        assert_eq!(response["result"]["type"], "ok");
        let expected_version = format!("4.0.{}", crate::app::APP_EVENT_DRAIN_LIMIT);
        assert_eq!(
            server.app.state.update_available.as_deref(),
            Some(expected_version.as_str())
        );
        assert!(server.app.event_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn headless_deferred_workspace_create_uses_runtime_events() {
        let event_hub = api::EventHub::default();
        let mut server = test_headless_server_with_event_hub(event_hub.clone());

        server.app.state.request_new_workspace = true;

        assert!(server.handle_deferred_requests_headless());
        assert!(!server.app.state.request_new_workspace);
        assert_eq!(
            event_hub
                .events_after(0)
                .into_iter()
                .map(|(_, event)| event.event)
                .collect::<Vec<_>>(),
            vec![
                api::schema::EventKind::WorkspaceCreated,
                api::schema::EventKind::TabCreated,
                api::schema::EventKind::PaneCreated,
                api::schema::EventKind::LayoutUpdated,
            ]
        );
        shutdown_test_runtimes(&mut server);
    }

    #[tokio::test]
    async fn headless_deferred_named_tab_create_uses_runtime_events() {
        let event_hub = api::EventHub::default();
        let mut server = test_headless_server_with_event_hub(event_hub.clone());
        server
            .app
            .create_workspace_with_options(std::env::temp_dir(), true)
            .unwrap();
        let after_setup = event_hub.current_sequence();

        server.app.state.request_new_tab = true;
        server.app.state.requested_new_tab_name = Some("ops".into());

        assert!(server.handle_deferred_requests_headless());
        assert!(!server.app.state.request_new_tab);
        assert_eq!(server.app.state.requested_new_tab_name, None);
        let events = event_hub.events_after(after_setup);
        assert_eq!(
            events
                .iter()
                .map(|(_, event)| event.event)
                .collect::<Vec<_>>(),
            vec![
                api::schema::EventKind::TabCreated,
                api::schema::EventKind::PaneCreated,
                api::schema::EventKind::LayoutUpdated,
            ]
        );
        let tab_created = events
            .iter()
            .find_map(|(_, event)| match &event.data {
                api::schema::EventData::TabCreated { tab } => Some(tab),
                _ => None,
            })
            .expect("tab created event");
        assert_eq!(tab_created.label, "ops");
        shutdown_test_runtimes(&mut server);
    }

    fn test_client_writer() -> (
        ClientWriter,
        std::sync::mpsc::Receiver<Vec<u8>>,
        std::sync::mpsc::Receiver<Vec<u8>>,
    ) {
        let (control_tx, control_rx) = std::sync::mpsc::channel();
        let (render_tx, render_rx) = std::sync::mpsc::sync_channel(1);
        (
            ClientWriter::test_channel(control_tx, render_tx),
            control_rx,
            render_rx,
        )
    }

    fn open_virtual_miller_files(server: &mut HeadlessServer) {
        let cwd = PathBuf::from("/virtual/miller-transport");
        let mut file_manager = crate::fm::FmState::test_empty(cwd.clone());
        file_manager.entries = ["alpha.txt", "bravo.txt"]
            .into_iter()
            .map(|name| crate::fm::FileEntry {
                name: name.to_owned(),
                path: cwd.join(name),
                kind: if false {
                    crate::fm::entry_kind::FileEntryKind::Directory
                } else {
                    crate::fm::entry_kind::FileEntryKind::RegularFile
                },
                modified: None,
            })
            .collect();
        file_manager.sync_trail_bridge_for_test();
        server.app.state.mobile_width_threshold = 0;
        server.app.state.sidebar_collapsed = true;
        server
            .app
            .state
            .try_open_file_manager_with(|_| Some(file_manager))
            .expect("open virtual Miller Files");
    }

    fn retained_test_server(
        initial_screen: &[u8],
    ) -> (
        HeadlessServer,
        std::sync::mpsc::Receiver<Vec<u8>>,
        crate::layout::PaneId,
    ) {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let pane_id = workspace.focused_pane_id().expect("focused pane");
        workspace.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, initial_screen),
        );
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (client_tx, _client_control_rx, client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        (server, client_rx, pane_id)
    }

    fn assert_frame_data_eq(actual: &FrameData, expected: &FrameData) {
        assert_eq!(
            (actual.width, actual.height),
            (expected.width, expected.height)
        );
        assert_eq!(actual.cursor, expected.cursor, "cursor mismatch");
        assert_eq!(actual.hyperlinks, expected.hyperlinks, "hyperlink mismatch");
        assert_eq!(actual.graphics, expected.graphics, "graphics mismatch");
        assert_eq!(
            actual.cells.len(),
            expected.cells.len(),
            "cell length mismatch"
        );
        for (idx, (actual_cell, expected_cell)) in
            actual.cells.iter().zip(expected.cells.iter()).enumerate()
        {
            assert_eq!(
                actual_cell,
                expected_cell,
                "cell mismatch at index {idx} (x={}, y={})",
                idx % usize::from(actual.width),
                idx / usize::from(actual.width),
            );
        }
    }

    #[test]
    fn foreground_client_applies_client_keybindings() {
        let mut server = test_headless_server();
        let local_config: crate::config::Config = toml::from_str(
            r#"
[keys]
prefix = "ctrl+a"
new_tab = "prefix+t"
"#,
        )
        .unwrap();
        let local_keybindings = local_config.live_keybinds().unwrap();
        let (writer_a, _control_a, _render_a) = test_client_writer();
        let (writer_b, _control_b, _render_b) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 1,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: Some(Box::new(local_keybindings)),
            direct_attach_requested: false,
            writer: writer_a,
        }));
        assert_eq!(
            server.app.state.prefix_code,
            crossterm::event::KeyCode::Char('a')
        );
        assert!(server
            .app
            .state
            .keybinds
            .new_tab
            .bindings
            .iter()
            .any(|binding| binding.label == "prefix+t"));

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 2,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: None,
            direct_attach_requested: false,
            writer: writer_b,
        }));
        assert_eq!(
            server.app.state.prefix_code,
            crossterm::event::KeyCode::Char('b')
        );
        assert!(server
            .app
            .state
            .keybinds
            .new_tab
            .bindings
            .iter()
            .any(|binding| binding.label == "prefix+c"));
    }

    #[test]
    fn local_keybinding_client_hides_server_keybinding_warnings() {
        let mut server = test_headless_server();
        let diagnostics = vec![
            "unsafe direct keybinding: keys.close_pane = \"x\" would intercept typing".to_owned(),
            "theme warning".to_owned(),
        ];
        let (full, without_keybindings) = server_config_diagnostic_summaries(&diagnostics);
        server.server_config_diagnostic = full.clone();
        server.server_config_diagnostic_without_keybindings = without_keybindings.clone();
        server.app.state.config_diagnostic = full;
        let local_keybindings = crate::config::Config::default().live_keybinds().unwrap();
        let (writer_a, _control_a, _render_a) = test_client_writer();
        let (writer_b, _control_b, _render_b) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 1,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: Some(Box::new(local_keybindings)),
            direct_attach_requested: false,
            writer: writer_a,
        }));
        assert_eq!(server.app.state.config_diagnostic, without_keybindings);

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 2,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: None,
            direct_attach_requested: false,
            writer: writer_b,
        }));
        assert_eq!(
            server.app.state.config_diagnostic,
            server.server_config_diagnostic
        );
    }

    #[test]
    fn local_keybinding_client_keeps_local_keybindings_after_settings_save() {
        let path = std::env::temp_dir().join(format!(
            "herdr-headless-settings-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(&path, "onboarding = false\n").unwrap();
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        std::env::set_var(crate::config::CONFIG_PATH_ENV_VAR, &path);

        let mut server = test_headless_server();
        let local_config: crate::config::Config = toml::from_str(
            r#"
[keys]
prefix = "ctrl+a"
new_workspace = "prefix+n"
next_tab = ""
"#,
        )
        .unwrap();
        let local_keybindings = local_config.live_keybinds().unwrap();
        let (writer, _control, _render) = test_client_writer();
        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 1,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: Some(Box::new(local_keybindings)),
            direct_attach_requested: false,
            writer,
        }));
        server.app.state.mode = crate::app::Mode::Settings;
        server.app.state.settings.section = crate::app::state::SettingsSection::Toast;
        server.app.state.settings.list.selected = 1;

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\r".to_vec(),
        }));

        assert_eq!(
            server.app.state.prefix_code,
            crossterm::event::KeyCode::Char('a')
        );
        assert!(server
            .app
            .state
            .keybinds
            .new_workspace
            .bindings
            .iter()
            .any(|binding| binding.label == "prefix+n"));
        assert!(server.app.state.toast.is_none());
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("delivery = \"herdr\""));

        std::env::remove_var(crate::config::CONFIG_PATH_ENV_VAR);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_server_keybindings_apply_valid_subset_after_settings_save_without_caching_local_keybindings(
    ) {
        let path = std::env::temp_dir().join(format!(
            "herdr-headless-invalid-settings-{}-{}.toml",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::write(
            &path,
            "onboarding = false\n[keys]\nnew_workspace = \"x\"\n[ui.toast]\ndelivery = \"off\"\n",
        )
        .unwrap();
        let _guard = crate::config::test_config_env_lock().lock().unwrap();
        std::env::set_var(crate::config::CONFIG_PATH_ENV_VAR, &path);

        let mut server = test_headless_server();
        let previous_server_config: crate::config::Config =
            toml::from_str("[keys]\nprefix = \"ctrl+c\"\nnew_workspace = \"prefix+m\"\n").unwrap();
        server.server_keybindings = previous_server_config.live_keybinds().unwrap();
        let local_config: crate::config::Config = toml::from_str(
            r#"
[keys]
prefix = "ctrl+a"
new_workspace = "prefix+n"
next_tab = ""
"#,
        )
        .unwrap();
        let (writer_a, _control_a, _render_a) = test_client_writer();
        let (writer_b, _control_b, _render_b) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 1,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: Some(Box::new(local_config.live_keybinds().unwrap())),
            direct_attach_requested: false,
            writer: writer_a,
        }));
        server.app.state.mode = crate::app::Mode::Settings;
        server.app.state.settings.section = crate::app::state::SettingsSection::Toast;
        server.app.state.settings.list.selected = 1;

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\r".to_vec(),
        }));

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 2,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: None,
            direct_attach_requested: false,
            writer: writer_b,
        }));
        assert_eq!(
            server.app.state.prefix_code,
            crossterm::event::KeyCode::Char('b')
        );
        assert!(!server
            .app
            .state
            .keybinds
            .new_workspace
            .bindings
            .iter()
            .any(|binding| binding.label == "prefix+n"));
        assert!(server.app.state.keybinds.new_workspace.bindings.is_empty());

        std::env::remove_var(crate::config::CONFIG_PATH_ENV_VAR);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn terminal_attach_rejects_missing_terminal_and_removes_client() {
        let mut server = test_headless_server();
        let (writer, control_rx, _render_rx) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 7,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::TerminalAnsi,
            keybindings: None,
            direct_attach_requested: true,
            writer,
        }));
        assert!(server.clients.contains_key(&7));

        assert!(
            !server.handle_server_event(ServerEvent::ClientAttachTerminal {
                client_id: 7,
                terminal_id: "term_missing".to_owned(),
                takeover: false,
            })
        );
        assert!(!server.clients.contains_key(&7));
        let reason = read_server_shutdown_reason(control_rx.recv().expect("shutdown message"));
        assert_eq!(
            reason,
            Some("terminal attach failed: terminal term_missing not found".to_owned())
        );
    }

    fn with_terminal_session_test_server(
        test: impl FnOnce(&mut HeadlessServer, crate::terminal::TerminalId, String, String),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let _runtime_guard = rt.enter();
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("test");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.terminal_id(pane_id).expect("terminal id").clone();
        let terminal_id_string = terminal_id.to_string();
        let public_pane_id = format!("{}:p1", workspace.id);
        server.app.state.workspaces = vec![workspace];
        server.app.state.ensure_test_terminals();
        server.app.terminal_runtimes.insert(
            terminal_id.clone(),
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b""),
        );

        test(&mut server, terminal_id, terminal_id_string, public_pane_id);

        drop(server);
        drop(_runtime_guard);
        rt.shutdown_timeout(Duration::from_millis(100));
    }

    fn connect_pending_terminal_client(server: &mut HeadlessServer, client_id: u64) {
        let _control_rx = connect_pending_terminal_client_with_control_rx(server, client_id);
    }

    fn connect_pending_terminal_client_with_control_rx(
        server: &mut HeadlessServer,
        client_id: u64,
    ) -> std::sync::mpsc::Receiver<Vec<u8>> {
        let (writer, control_rx, _render_rx) = test_client_writer();
        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id,
            cols: 100,
            rows: 30,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::TerminalAnsi,
            keybindings: None,
            direct_attach_requested: true,
            writer,
        }));
        control_rx
    }

    #[test]
    fn terminal_observe_allows_multiple_clients_without_attach_ownership() {
        with_terminal_session_test_server(|server, terminal_id, terminal_id_string, _| {
            let initial_size = server
                .app
                .terminal_runtimes
                .get(&terminal_id)
                .expect("runtime")
                .current_size();

            for client_id in [7, 8] {
                connect_pending_terminal_client(server, client_id);
                assert!(
                    server.handle_server_event(ServerEvent::ClientObserveTerminal {
                        client_id,
                        target: terminal_id_string.clone(),
                    })
                );
            }

            assert!(server.terminal_attach_owners.is_empty());
            assert!(!server
                .app
                .state
                .direct_attach_resize_locks
                .contains(&terminal_id));
            assert_eq!(
                server
                    .app
                    .terminal_runtimes
                    .get(&terminal_id)
                    .expect("runtime")
                    .current_size(),
                initial_size
            );
            assert_eq!(
                terminal_stream_client_ids(&server.clients, &terminal_id_string).len(),
                2
            );
        });
    }

    #[test]
    fn terminal_observe_resolves_public_pane_id() {
        with_terminal_session_test_server(|server, terminal_id, _, public_pane_id| {
            connect_pending_terminal_client(server, 7);
            assert!(
                server.handle_server_event(ServerEvent::ClientObserveTerminal {
                    client_id: 7,
                    target: public_pane_id,
                })
            );

            assert!(matches!(
                server.clients.get(&7).map(|client| &client.mode),
                Some(ClientConnectionMode::TerminalObserve { terminal_id: observed })
                    if observed == &terminal_id.to_string()
            ));
        });
    }

    #[test]
    fn terminal_control_resolves_public_pane_id_and_takes_ownership() {
        with_terminal_session_test_server(
            |server, terminal_id, terminal_id_string, public_pane_id| {
                connect_pending_terminal_client(server, 7);
                assert!(
                    server.handle_server_event(ServerEvent::ClientControlTerminal {
                        client_id: 7,
                        target: public_pane_id,
                        takeover: false,
                    })
                );

                assert!(matches!(
                    server.clients.get(&7).map(|client| &client.mode),
                    Some(ClientConnectionMode::TerminalAttach { terminal_id: attached })
                        if attached == &terminal_id_string
                ));
                assert_eq!(
                    server.terminal_attach_owners.get(&terminal_id_string),
                    Some(&7)
                );
                assert!(server
                    .app
                    .state
                    .direct_attach_resize_locks
                    .contains(&terminal_id));
            },
        );
    }

    #[test]
    fn terminal_control_rejects_second_controller_without_takeover() {
        with_terminal_session_test_server(|server, _terminal_id, terminal_id_string, _| {
            connect_pending_terminal_client(server, 7);
            assert!(
                server.handle_server_event(ServerEvent::ClientControlTerminal {
                    client_id: 7,
                    target: terminal_id_string.clone(),
                    takeover: false,
                })
            );

            connect_pending_terminal_client(server, 8);
            assert!(
                !server.handle_server_event(ServerEvent::ClientControlTerminal {
                    client_id: 8,
                    target: terminal_id_string.clone(),
                    takeover: false,
                })
            );

            assert!(server.clients.contains_key(&7));
            assert!(!server.clients.contains_key(&8));
            assert_eq!(
                server.terminal_attach_owners.get(&terminal_id_string),
                Some(&7)
            );
        });
    }

    #[test]
    fn terminal_control_takeover_replaces_existing_controller() {
        with_terminal_session_test_server(|server, _terminal_id, terminal_id_string, _| {
            connect_pending_terminal_client(server, 7);
            assert!(
                server.handle_server_event(ServerEvent::ClientControlTerminal {
                    client_id: 7,
                    target: terminal_id_string.clone(),
                    takeover: false,
                })
            );

            connect_pending_terminal_client(server, 8);
            assert!(
                server.handle_server_event(ServerEvent::ClientControlTerminal {
                    client_id: 8,
                    target: terminal_id_string.clone(),
                    takeover: true,
                })
            );

            assert!(!server.clients.contains_key(&7));
            assert!(server.clients.contains_key(&8));
            assert_eq!(
                server.terminal_attach_owners.get(&terminal_id_string),
                Some(&8)
            );
        });
    }

    #[test]
    fn terminal_observe_can_coexist_with_terminal_control() {
        with_terminal_session_test_server(|server, _terminal_id, terminal_id_string, _| {
            connect_pending_terminal_client(server, 7);
            assert!(
                server.handle_server_event(ServerEvent::ClientControlTerminal {
                    client_id: 7,
                    target: terminal_id_string.clone(),
                    takeover: false,
                })
            );

            connect_pending_terminal_client(server, 8);
            assert!(
                server.handle_server_event(ServerEvent::ClientObserveTerminal {
                    client_id: 8,
                    target: terminal_id_string.clone(),
                })
            );

            assert_eq!(
                server.terminal_attach_owners.get(&terminal_id_string),
                Some(&7)
            );
            assert!(matches!(
                server.clients.get(&8).map(|client| &client.mode),
                Some(ClientConnectionMode::TerminalObserve { terminal_id })
                    if terminal_id == &terminal_id_string
            ));
            assert_eq!(
                terminal_stream_client_ids(&server.clients, &terminal_id_string).len(),
                2
            );
        });
    }

    #[test]
    fn terminal_control_detach_sends_shutdown_before_removal() {
        with_terminal_session_test_server(|server, _terminal_id, terminal_id_string, _| {
            let control_rx = connect_pending_terminal_client_with_control_rx(server, 7);
            assert!(
                server.handle_server_event(ServerEvent::ClientControlTerminal {
                    client_id: 7,
                    target: terminal_id_string.clone(),
                    takeover: false,
                })
            );

            assert!(server.handle_server_event(ServerEvent::ClientDetach { client_id: 7 }));

            assert!(!server.clients.contains_key(&7));
            assert!(!server
                .terminal_attach_owners
                .contains_key(&terminal_id_string));
            let reason = read_server_shutdown_reason(control_rx.recv().expect("shutdown message"));
            assert_eq!(reason, Some("detached".to_owned()));
        });
    }

    #[test]
    fn terminal_observe_rejects_later_attach_upgrade() {
        with_terminal_session_test_server(|server, terminal_id, terminal_id_string, _| {
            connect_pending_terminal_client(server, 7);
            assert!(
                server.handle_server_event(ServerEvent::ClientObserveTerminal {
                    client_id: 7,
                    target: terminal_id_string.clone(),
                })
            );
            assert!(
                !server.handle_server_event(ServerEvent::ClientAttachTerminal {
                    client_id: 7,
                    terminal_id: terminal_id_string,
                    takeover: true,
                })
            );

            assert!(!server.clients.contains_key(&7));
            assert!(server.terminal_attach_owners.is_empty());
            assert!(!server
                .app
                .state
                .direct_attach_resize_locks
                .contains(&terminal_id));
        });
    }

    #[test]
    fn terminal_attach_rejects_later_observe_and_clears_ownership() {
        with_terminal_session_test_server(|server, terminal_id, terminal_id_string, _| {
            connect_pending_terminal_client(server, 7);
            assert!(
                server.handle_server_event(ServerEvent::ClientAttachTerminal {
                    client_id: 7,
                    terminal_id: terminal_id_string.clone(),
                    takeover: false,
                })
            );
            assert_eq!(
                server.terminal_attach_owners.get(&terminal_id_string),
                Some(&7)
            );
            assert!(server
                .app
                .state
                .direct_attach_resize_locks
                .contains(&terminal_id));

            assert!(
                !server.handle_server_event(ServerEvent::ClientObserveTerminal {
                    client_id: 7,
                    target: terminal_id_string.clone(),
                })
            );

            assert!(!server.clients.contains_key(&7));
            assert!(server.terminal_attach_owners.is_empty());
            assert!(!server
                .app
                .state
                .direct_attach_resize_locks
                .contains(&terminal_id));
        });
    }

    fn app_client_marks_git_refresh_due_on_first_attach(render_encoding: RenderEncoding) {
        let mut server = test_headless_server();
        server
            .app
            .state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let future = Instant::now() + Duration::from_secs(60);
        server.app.last_git_remote_status_refresh = future;
        let (writer, _control_rx, _render_rx) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 7,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding,
            keybindings: None,
            direct_attach_requested: false,
            writer,
        }));

        assert!(server.has_app_client());
        assert!(server
            .app
            .git_refresh_deadline()
            .is_some_and(|deadline| deadline <= Instant::now()));
    }

    #[test]
    fn terminal_ansi_app_client_enables_headless_git_refresh() {
        app_client_marks_git_refresh_due_on_first_attach(RenderEncoding::TerminalAnsi);
    }

    #[test]
    fn pending_terminal_attach_client_does_not_enable_headless_git_refresh() {
        let mut server = test_headless_server();
        server
            .app
            .state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let (writer, _control_rx, _render_rx) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 7,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::TerminalAnsi,
            keybindings: None,
            direct_attach_requested: true,
            writer,
        }));

        assert!(!server.has_app_client());
        assert_eq!(
            server.app.next_headless_loop_deadline_with_git_refresh(
                Instant::now(),
                false,
                server.has_app_client()
            ),
            None
        );
    }

    #[test]
    fn writerless_app_client_does_not_enable_headless_git_refresh() {
        let mut server = test_headless_server();
        server
            .app
            .state
            .workspaces
            .push(crate::workspace::Workspace::test_new("test"));
        let (writer, _control_rx, _render_rx) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 7,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::SemanticFrame,
            keybindings: None,
            direct_attach_requested: false,
            writer,
        }));
        assert!(server.has_app_client());

        server.clients.get_mut(&7).expect("client").writer = None;

        assert!(!server.has_app_client());
        assert_eq!(
            server.app.next_headless_loop_deadline_with_git_refresh(
                Instant::now(),
                false,
                server.has_app_client()
            ),
            None
        );
    }

    #[test]
    fn semantic_app_client_marks_git_refresh_due_on_first_attach() {
        app_client_marks_git_refresh_due_on_first_attach(RenderEncoding::SemanticFrame);
    }

    #[test]
    fn terminal_attach_client_exits_when_attached_pane_dies() {
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("attached");
        let pane_id = workspace.tabs[0].root_pane;
        server.app.state.workspaces = vec![workspace];
        server.app.state.ensure_test_terminals();
        let terminal_id = server.app.state.workspaces[0]
            .pane_state(pane_id)
            .expect("pane")
            .attached_terminal_id
            .to_string();
        let (writer, control_rx, _render_rx) = test_client_writer();

        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 7,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::TerminalAnsi,
            keybindings: None,
            direct_attach_requested: true,
            writer,
        }));
        assert!(
            server.handle_server_event(ServerEvent::ClientAttachTerminal {
                client_id: 7,
                terminal_id: terminal_id.clone(),
                takeover: false,
            })
        );
        assert_eq!(server.terminal_attach_owners.get(&terminal_id), Some(&7));

        assert!(server.handle_internal_event_with_forwarding(AppEvent::PaneDied { pane_id }));

        assert!(!server.clients.contains_key(&7));
        assert!(!server.terminal_attach_owners.contains_key(&terminal_id));
        let reason = read_server_shutdown_reason(control_rx.recv().expect("shutdown message"));
        assert_eq!(reason, Some(format!("terminal {terminal_id} exited")));
    }

    #[test]
    fn terminal_attach_scroll_moves_attached_runtime_viewport() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let _runtime_guard = rt.enter();
        let mut bytes = Vec::new();
        for line in 0..80 {
            bytes.extend_from_slice(format!("line {line:02}\r\n").as_bytes());
        }
        let runtime =
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(20, 5, 4096, &bytes);

        apply_terminal_attach_scroll(
            &runtime,
            AttachScrollSource::Wheel,
            AttachScrollDirection::Up,
            3,
            None,
            None,
            0,
        )
        .expect("scroll up");
        let metrics = runtime.scroll_metrics().expect("scroll metrics");
        assert_eq!(metrics.offset_from_bottom, 3);

        apply_terminal_attach_scroll(
            &runtime,
            AttachScrollSource::Wheel,
            AttachScrollDirection::Down,
            2,
            None,
            None,
            0,
        )
        .expect("scroll down");
        let metrics = runtime.scroll_metrics().expect("scroll metrics");
        assert_eq!(metrics.offset_from_bottom, 1);
        drop(runtime);
        drop(_runtime_guard);
        rt.shutdown_timeout(Duration::from_millis(100));
    }

    #[test]
    fn terminal_attach_input_resets_scrolled_viewport() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let _runtime_guard = rt.enter();
        let mut bytes = Vec::new();
        for line in 0..80 {
            bytes.extend_from_slice(format!("line {line:02}\r\n").as_bytes());
        }
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                20, 5, 4096, &bytes, 4,
            );

        runtime.scroll_up(4);
        assert_eq!(
            runtime
                .scroll_metrics()
                .expect("scroll metrics")
                .offset_from_bottom,
            4
        );

        apply_terminal_attach_input(&runtime, b"x".to_vec()).expect("attach input");
        assert_eq!(
            runtime
                .scroll_metrics()
                .expect("scroll metrics")
                .offset_from_bottom,
            0
        );
        assert_eq!(
            input_rx.try_recv().expect("forwarded input"),
            Bytes::from("x")
        );

        drop(runtime);
        drop(_runtime_guard);
        rt.shutdown_timeout(Duration::from_millis(100));
    }

    fn with_terminal_attach_page_key_runtime(
        initial_bytes: &[u8],
        initial_scroll: usize,
        test: impl FnOnce(&crate::terminal::TerminalRuntime, &mut mpsc::Receiver<Bytes>),
    ) {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let _runtime_guard = rt.enter();
        let mut bytes = initial_bytes.to_vec();
        for line in 0..80 {
            bytes.extend_from_slice(format!("line {line:02}\r\n").as_bytes());
        }
        let (runtime, mut input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                20, 5, 4096, &bytes, 4,
            );
        if initial_scroll > 0 {
            runtime.scroll_up(initial_scroll);
        }

        test(&runtime, &mut input_rx);

        drop(runtime);
        drop(_runtime_guard);
        rt.shutdown_timeout(Duration::from_millis(100));
    }

    fn apply_terminal_attach_page_up(runtime: &crate::terminal::TerminalRuntime) {
        apply_terminal_attach_scroll(
            runtime,
            AttachScrollSource::PageKey {
                input: b"\x1b[5~".to_vec(),
            },
            AttachScrollDirection::Up,
            4,
            None,
            None,
            0,
        )
        .expect("page key");
    }

    #[test]
    fn terminal_attach_page_key_host_scrolls_plain_terminal() {
        with_terminal_attach_page_key_runtime(b"", 0, |runtime, input_rx| {
            apply_terminal_attach_page_up(runtime);

            assert_eq!(
                runtime
                    .scroll_metrics()
                    .expect("scroll metrics")
                    .offset_from_bottom,
                4
            );
            assert!(input_rx.try_recv().is_err());
        });
    }

    #[test]
    fn terminal_attach_page_key_forwards_when_mouse_reporting() {
        with_terminal_attach_page_key_runtime(b"\x1b[?1000h", 3, |runtime, input_rx| {
            apply_terminal_attach_page_up(runtime);

            assert_eq!(
                runtime
                    .scroll_metrics()
                    .expect("scroll metrics")
                    .offset_from_bottom,
                0
            );
            assert_eq!(
                input_rx.try_recv().expect("forwarded page key"),
                Bytes::from_static(b"\x1b[5~")
            );
        });
    }

    #[test]
    fn terminal_attach_page_key_forwards_when_application_cursor() {
        with_terminal_attach_page_key_runtime(b"\x1b[?1h", 3, |runtime, input_rx| {
            apply_terminal_attach_page_up(runtime);

            assert_eq!(
                runtime
                    .scroll_metrics()
                    .expect("scroll metrics")
                    .offset_from_bottom,
                0
            );
            assert_eq!(
                input_rx.try_recv().expect("forwarded page key"),
                Bytes::from_static(b"\x1b[5~")
            );
        });
    }

    #[test]
    fn terminal_attach_page_key_forwards_in_alternate_screen_without_mouse_reporting() {
        with_terminal_attach_page_key_runtime(b"\x1b[?1049h", 3, |runtime, input_rx| {
            apply_terminal_attach_page_up(runtime);

            assert_eq!(
                runtime
                    .scroll_metrics()
                    .expect("scroll metrics")
                    .offset_from_bottom,
                0
            );
            assert_eq!(
                input_rx.try_recv().expect("forwarded page key"),
                Bytes::from_static(b"\x1b[5~")
            );
        });
    }

    #[test]
    fn headless_scheduled_tasks_expire_agent_metadata() {
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("metadata");
        let pane_id = workspace.tabs[0].root_pane;
        server.app.state.workspaces = vec![workspace];
        server.app.state.ensure_test_terminals();

        assert!(
            server.handle_internal_event_with_forwarding(AppEvent::HookStateReported {
                pane_id,
                source: "custom:pi".into(),
                agent_label: "pi".into(),
                state: crate::detect::AgentState::Working,
                message: None,
                seq: None,
                session_ref: None,
            })
        );
        assert!(
            server.handle_internal_event_with_forwarding(AppEvent::HookMetadataReported {
                pane_id,
                source: "user:pi-display".into(),
                agent_label: Some("pi".into()),
                applies_to_source: Some("custom:pi".into()),
                title: Some("short lived".into()),
                display_agent: None,
                state_labels: HashMap::new(),
                clear_title: false,
                clear_display_agent: false,
                clear_state_labels: false,
                seq: None,
                // The expiry is driven below with a synthetic `now`; keep the
                // real-time TTL comfortably beyond full-suite scheduling jitter.
                ttl: Some(Duration::from_secs(60)),
            })
        );

        let deadline = server
            .app
            .agent_metadata_deadline
            .expect("metadata deadline");
        let terminal_id = server.app.state.workspaces[0]
            .pane_state(pane_id)
            .expect("pane")
            .attached_terminal_id
            .clone();
        assert_eq!(
            server
                .app
                .state
                .terminals
                .get(&terminal_id)
                .expect("terminal")
                .effective_title()
                .as_deref(),
            Some("short lived")
        );

        assert!(server.handle_scheduled_tasks_headless(deadline + Duration::from_millis(1), false));

        assert_eq!(server.app.agent_metadata_deadline, None);
        assert_eq!(
            server
                .app
                .state
                .terminals
                .get(&terminal_id)
                .expect("terminal")
                .effective_title(),
            None
        );
        assert!(server
            .app
            .event_hub
            .events_after(0)
            .iter()
            .any(|(_, event)| {
                event.event == crate::api::schema::EventKind::PaneAgentStatusChanged
                    && matches!(
                        &event.data,
                        crate::api::schema::EventData::PaneAgentStatusChanged {
                            title,
                            ..
                        } if title.is_none()
                    )
            }));
    }

    // FM-PERF-TEXT-11: the server-owned runtime must pump the same bounded
    // text-preview worker as monolithic App::run. A file click is intentionally
    // disk-free, so omitting this adapter would leave headless Files stuck in
    // PendingText forever.
    #[test]
    fn headless_scheduled_tasks_pump_pending_text_preview_worker() {
        struct RemoveOnDrop(PathBuf);

        impl Drop for RemoveOnDrop {
            fn drop(&mut self) {
                let _ = fs::remove_dir_all(&self.0);
            }
        }

        let root = std::env::temp_dir().join(format!(
            "headless-text-preview-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let _cleanup = RemoveOnDrop(root.clone());
        fs::create_dir_all(&root).expect("create headless preview fixture");
        let path = root.join("clicked.rs");
        fs::write(&path, "fn headless_preview() {}\n").expect("write preview fixture");

        let mut server = test_headless_server();
        server
            .app
            .state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("open Files instance");
        let file_manager = server.app.state.file_manager.as_mut().expect("open Files");
        let entry_index = file_manager.trail_snapshots.cols()[0]
            .entries()
            .iter()
            .position(|entry| entry.path == path)
            .expect("resident file row");
        assert_eq!(
            file_manager.activate_trail_entry(0, entry_index, &path),
            crate::fm::trail_snapshots::TrailActivateOutcome::SelectedFile
        );
        assert!(matches!(
            &file_manager.preview,
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::PendingText {
                source_path,
                ..
            }) if source_path == &path
        ));

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
        });
        assert_eq!(
            profile.counter("fm.text_worker.submitted"),
            1,
            "headless scheduling must submit the pending exact-path preview"
        );

        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
            let resolved = server
                .app
                .state
                .file_manager
                .as_ref()
                .is_some_and(|file_manager| {
                    matches!(
                        &file_manager.preview,
                        crate::fm::FmPreview::File(crate::fm::FmFilePreview::Text(preview))
                            if preview.source_path == path
                                && preview.content == "fn headless_preview() {}\n"
                                && preview.highlighted.is_some()
                    )
                });
            if resolved {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out applying headless text preview"
            );
            std::thread::yield_now();
        }
    }

    #[test]
    fn headless_scheduled_tasks_clears_disabled_agent_manifest_update_deadline() {
        let mut server = test_headless_server();
        let now = Instant::now();
        server.app.next_agent_manifest_update_check = Some(now - Duration::from_millis(1));

        assert!(!server.handle_scheduled_tasks_headless(now, false));
        assert_eq!(server.app.next_agent_manifest_update_check, None);
    }

    #[tokio::test]
    async fn headless_scheduled_tasks_do_not_start_pending_agent_resume_when_geometry_dirty() {
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("restored");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.terminal_id(pane_id).cloned().unwrap();
        server.app.state.view.pane_infos = workspace.tabs[0]
            .layout
            .panes(ratatui::layout::Rect::new(0, 0, 100, 30));
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.ensure_test_terminals();
        server.clients.insert(
            1,
            ClientConnection::new(
                (100, 30),
                crate::kitty_graphics::HostCellSize::default(),
                server.app.state.host_terminal_theme,
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.effective_size = (100, 30);
        server.app.state.host_terminal_theme = crate::terminal_theme::TerminalTheme {
            foreground: Some(crate::terminal_theme::RgbColor {
                r: 220,
                g: 220,
                b: 220,
            }),
            background: Some(crate::terminal_theme::RgbColor {
                r: 20,
                g: 20,
                b: 20,
            }),
            ..Default::default()
        };
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .expect("test terminal should exist")
            .pending_agent_resume_plan = Some(crate::agent_resume::AgentResumePlan {
            agent: "codex".into(),
            argv: vec!["/bin/sh".into(), "-c".into(), "sleep 5".into()],
            dedupe_key: "herdr:codex\0codex\0Id\0codex-session".into(),
        });
        server.app.pending_agent_resume_deadline = Some(Instant::now() - Duration::from_millis(1));

        assert!(!server.handle_scheduled_tasks_headless(Instant::now(), true));
        assert!(server.app.terminal_runtimes.get(&terminal_id).is_none());
        assert!(server
            .app
            .state
            .terminals
            .get(&terminal_id)
            .expect("test terminal should still exist")
            .pending_agent_resume_plan
            .is_some());
        assert!(server.app.pending_agent_resume_deadline.is_none());
    }

    #[tokio::test]
    async fn headless_scheduled_tasks_do_not_start_pending_agent_resume_without_foreground_client()
    {
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("restored");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.terminal_id(pane_id).cloned().unwrap();
        server.app.state.view.pane_infos = workspace.tabs[0]
            .layout
            .panes(ratatui::layout::Rect::new(0, 0, 80, 24));
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.ensure_test_terminals();
        server.foreground_client_id = None;
        server.effective_size = (80, 24);
        server.app.state.host_terminal_theme = crate::terminal_theme::TerminalTheme {
            foreground: Some(crate::terminal_theme::RgbColor {
                r: 220,
                g: 220,
                b: 220,
            }),
            background: Some(crate::terminal_theme::RgbColor {
                r: 20,
                g: 20,
                b: 20,
            }),
            ..Default::default()
        };
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .expect("test terminal should exist")
            .pending_agent_resume_plan = Some(crate::agent_resume::AgentResumePlan {
            agent: "codex".into(),
            argv: vec!["/bin/sh".into(), "-c".into(), "sleep 5".into()],
            dedupe_key: "herdr:codex\0codex\0Id\0codex-session".into(),
        });
        server.app.pending_agent_resume_deadline = Some(Instant::now() - Duration::from_millis(1));

        assert!(!server.handle_scheduled_tasks_headless(Instant::now(), false));
        assert!(server.app.terminal_runtimes.get(&terminal_id).is_none());
        assert!(server
            .app
            .state
            .terminals
            .get(&terminal_id)
            .expect("test terminal should still exist")
            .pending_agent_resume_plan
            .is_some());
        assert!(server.app.pending_agent_resume_deadline.is_none());
    }

    #[tokio::test]
    async fn headless_pre_input_resize_does_not_start_pending_agent_resume() {
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("restored");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.terminal_id(pane_id).cloned().unwrap();
        server.app.state.view.pane_infos = workspace.tabs[0]
            .layout
            .panes(ratatui::layout::Rect::new(0, 0, 100, 30));
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.ensure_test_terminals();
        server.clients.insert(
            1,
            ClientConnection::new(
                (100, 30),
                crate::kitty_graphics::HostCellSize::default(),
                server.app.state.host_terminal_theme,
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.effective_size = (100, 30);
        server.app.state.host_terminal_theme = crate::terminal_theme::TerminalTheme {
            foreground: Some(crate::terminal_theme::RgbColor {
                r: 220,
                g: 220,
                b: 220,
            }),
            background: Some(crate::terminal_theme::RgbColor {
                r: 20,
                g: 20,
                b: 20,
            }),
            ..Default::default()
        };
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .expect("test terminal should exist")
            .pending_agent_resume_plan = Some(crate::agent_resume::AgentResumePlan {
            agent: "codex".into(),
            argv: vec!["/bin/sh".into(), "-c".into(), "sleep 5".into()],
            dedupe_key: "herdr:codex\0codex\0Id\0codex-session".into(),
        });
        server.app.pending_agent_resume_deadline = Some(Instant::now() - Duration::from_millis(1));

        server.resize_shared_runtime_to_effective_size_before_input();

        assert!(server.app.terminal_runtimes.get(&terminal_id).is_none());
        assert!(server
            .app
            .state
            .terminals
            .get(&terminal_id)
            .expect("test terminal should still exist")
            .pending_agent_resume_plan
            .is_some());
        assert!(server.app.pending_agent_resume_deadline.is_none());
    }

    #[test]
    fn virtual_render_produces_nonempty_buffer() {
        let mut state = AppState::test_new();
        let area = Rect::new(0, 0, 80, 24);
        let (buffer, _cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);
        assert_eq!(buffer.area.width, 80);
        assert_eq!(buffer.area.height, 24);
    }

    #[test]
    fn virtual_render_without_frame_cursor_keeps_cursor_hidden() {
        let mut state = AppState::test_new();
        let area = Rect::new(0, 0, 80, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);

        assert_eq!(cursor, None);
    }

    #[tokio::test]
    async fn virtual_render_preserves_explicit_frame_cursor_position() {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left"),
        );

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);
        let pane = state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)
            .expect("focused pane info");

        assert_eq!(
            cursor,
            Some(CursorState {
                x: pane.inner_rect.x + 4,
                y: pane.inner_rect.y,
                visible: true,
                shape: cursor.as_ref().map(|c| c.shape).unwrap_or(0),
            })
        );
    }

    #[tokio::test]
    async fn virtual_render_preserves_hidden_focused_pane_cursor_position() {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left\x1b[?25l"),
        );

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);
        let pane = state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)
            .expect("focused pane info");

        assert_eq!(
            cursor,
            Some(CursorState {
                x: pane.inner_rect.x + 4,
                y: pane.inner_rect.y,
                visible: false,
                shape: cursor.as_ref().map(|c| c.shape).unwrap_or(0),
            })
        );
    }

    #[tokio::test]
    async fn virtual_render_hides_focused_pane_cursor_during_synchronized_output() {
        let mut state = AppState::test_new();
        state.reveal_hidden_cursor_for_cjk_ime = true;
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left");
        ws.insert_test_runtime(pane_id, runtime);

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let _ = crate::server::render_stream::render_virtual(&mut state, area, true);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        let runtime = state
            .runtime_for_pane(&terminal_runtimes, pane_id)
            .expect("pane runtime after initial render");
        runtime.test_process_pty_bytes(b"\x1b[?2026h\x1b[2;3H");
        assert!(runtime.synchronized_output_active());

        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, false);

        assert_eq!(
            cursor, None,
            "child cursor positions are unstable while synchronized output is active"
        );
    }

    #[tokio::test]
    async fn virtual_render_hides_focused_pane_cursor_during_synchronized_output_resize() {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left");
        ws.insert_test_runtime(pane_id, runtime);

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let initial_area = Rect::new(0, 0, 80, 24);
        let _ = crate::server::render_stream::render_virtual(&mut state, initial_area, true);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        let runtime = state
            .runtime_for_pane(&terminal_runtimes, pane_id)
            .expect("pane runtime after initial render");
        runtime.test_process_pty_bytes(b"\x1b[?2026h\x1b[2;3H");
        assert!(runtime.synchronized_output_active());

        let resized_area = Rect::new(0, 0, 100, 30);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, resized_area, true);

        assert_eq!(
            cursor, None,
            "pre-resize synchronized output should suppress the cursor even if resize clears the mode"
        );
    }

    #[tokio::test]
    async fn virtual_render_exposes_hidden_pane_cursor_when_reveal_hidden_for_cjk_ime() {
        let mut state = AppState::test_new();
        state.reveal_hidden_cursor_for_cjk_ime = true;
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left\x1b[?25l"),
        );

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);
        let pane = state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)
            .expect("focused pane info");

        assert_eq!(
            cursor,
            Some(CursorState {
                x: pane.inner_rect.x + 4,
                y: pane.inner_rect.y,
                visible: true,
                shape: state.cjk_ime_cursor_shape,
            })
        );
    }

    #[tokio::test]
    async fn virtual_render_keeps_cursor_hidden_when_scrolled_back_even_with_reveal_hidden_for_cjk_ime(
    ) {
        let mut state = AppState::test_new();
        state.reveal_hidden_cursor_for_cjk_ime = true;
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let mut bytes = Vec::new();
        for line in 0..80 {
            bytes.extend_from_slice(format!("line {line:02}\r\n").as_bytes());
        }
        let runtime =
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(20, 5, 4096, &bytes);
        ws.insert_test_runtime(pane_id, runtime);

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let _ = crate::server::render_stream::render_virtual(&mut state, area, true);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        let runtime = state
            .runtime_for_pane(&terminal_runtimes, pane_id)
            .expect("pane runtime after initial render");
        runtime.scroll_up(6);
        assert!(crate::ui::pane_is_scrolled_back(runtime));

        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);

        assert!(
            cursor.as_ref().is_none_or(|cursor| !cursor.visible),
            "scrolled-back focused pane should keep the cursor hidden even when reveal_hidden_cursor_for_cjk_ime is true; got {cursor:?}",
        );
    }

    #[tokio::test]
    async fn virtual_render_fallback_cursor_when_viewport_none_and_reveal_hidden_for_cjk_ime() {
        let mut state = AppState::test_new();
        state.reveal_hidden_cursor_for_cjk_ime = true;
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        // Feed only ?25l with no prior cursor movement — exercises the fallback
        // path for TUIs whose viewport has no cursor position.
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"\x1b[?25l"),
        );

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);
        let pane = state
            .view
            .pane_infos
            .iter()
            .find(|info| info.id == pane_id)
            .expect("focused pane info");

        assert_eq!(
            cursor,
            Some(CursorState {
                x: pane.inner_rect.x,
                y: pane.inner_rect.y,
                visible: true,
                shape: state.cjk_ime_cursor_shape,
            }),
            "fallback should anchor at pane top-left with the configured shape",
        );
    }

    #[tokio::test]
    async fn virtual_render_skips_reveal_when_focused_pane_has_no_detected_agent() {
        let mut state = AppState::test_new();
        state.reveal_hidden_cursor_for_cjk_ime = true;
        // Filter only Claude, but the test pane has no detected agent, so the
        // reveal must not apply.
        state.cjk_ime_agent_filter_configured = true;
        state.cjk_ime_agents = vec![crate::detect::Agent::Claude];
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left\x1b[?25l"),
        );

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);

        assert!(
            cursor.as_ref().is_none_or(|cursor| !cursor.visible),
            "agent filter should suppress reveal when the focused pane's detected agent is not on the list; got {cursor:?}",
        );
    }

    #[tokio::test]
    async fn virtual_render_skips_reveal_when_agent_filter_has_no_valid_entries() {
        let mut state = AppState::test_new();
        state.reveal_hidden_cursor_for_cjk_ime = true;
        state.cjk_ime_agent_filter_configured = true;
        state.cjk_ime_agents = Vec::new();
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left\x1b[?25l"),
        );

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);

        assert!(
            cursor.as_ref().is_none_or(|cursor| !cursor.visible),
            "agent filter with no valid entries should suppress reveal; got {cursor:?}",
        );
    }

    #[tokio::test]
    async fn virtual_render_omits_focused_pane_cursor_while_mobile_switcher_open() {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        ws.insert_test_runtime(
            pane_id,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(20, 5, b"left"),
        );

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Navigate;

        let area = Rect::new(0, 0, 44, 24);
        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);

        assert_eq!(cursor, None);
    }

    #[tokio::test]
    async fn virtual_render_hides_focused_pane_cursor_while_scrolled_back() {
        let mut state = AppState::test_new();
        let mut ws = crate::workspace::Workspace::test_new("test");
        let pane_id = ws.tabs[0].root_pane;
        let mut bytes = Vec::new();
        for line in 0..80 {
            bytes.extend_from_slice(format!("line {line:02}\r\n").as_bytes());
        }
        let runtime =
            crate::terminal::TerminalRuntime::test_with_scrollback_bytes(20, 5, 4096, &bytes);
        ws.insert_test_runtime(pane_id, runtime);

        state.workspaces = vec![ws];
        state.active = Some(0);
        state.selected = 0;
        state.mode = crate::app::Mode::Terminal;

        let area = Rect::new(0, 0, 80, 24);
        let _ = crate::server::render_stream::render_virtual(&mut state, area, true);
        let terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        let runtime = state
            .runtime_for_pane(&terminal_runtimes, pane_id)
            .expect("pane runtime after initial render");
        runtime.scroll_up(6);
        assert!(crate::ui::pane_is_scrolled_back(runtime));

        let (_buffer, cursor) =
            crate::server::render_stream::render_virtual(&mut state, area, true);

        assert!(
            cursor.as_ref().is_none_or(|cursor| !cursor.visible),
            "cursor: {cursor:?}"
        );
    }

    #[test]
    fn latest_active_client_drives_shared_size_theme_and_fallback() {
        let mut server = test_headless_server();

        server.clients.insert(
            1,
            ClientConnection::new(
                (160, 45),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme {
                    foreground: Some(crate::terminal_theme::RgbColor {
                        r: 0xaa,
                        g: 0xbb,
                        b: 0xcc,
                    }),
                    background: Some(crate::terminal_theme::RgbColor {
                        r: 0x11,
                        g: 0x22,
                        b: 0x33,
                    }),
                    ..Default::default()
                },
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme {
                    foreground: Some(crate::terminal_theme::RgbColor {
                        r: 0x10,
                        g: 0x20,
                        b: 0x30,
                    }),
                    background: Some(crate::terminal_theme::RgbColor {
                        r: 0xdd,
                        g: 0xee,
                        b: 0xff,
                    }),
                    ..Default::default()
                },
                None,
                2,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );

        assert!(server.promote_client_to_foreground(1));
        assert_eq!(server.foreground_client_id, Some(1));
        assert_eq!(server.effective_size, (160, 45));
        assert_eq!(
            server.app.state.host_terminal_theme,
            server.clients[&1].host_terminal_theme
        );

        assert!(server.promote_client_to_foreground(2));
        assert_eq!(server.foreground_client_id, Some(2));
        assert_eq!(server.effective_size, (80, 24));
        assert_eq!(
            server.app.state.host_terminal_theme,
            server.clients[&2].host_terminal_theme
        );

        assert!(server.remove_client(2));
        assert_eq!(server.foreground_client_id, Some(1));
        assert_eq!(server.effective_size, (160, 45));
        assert_eq!(
            server.app.state.host_terminal_theme,
            server.clients[&1].host_terminal_theme
        );
    }

    #[test]
    fn foreground_client_without_host_theme_clears_previous_host_theme() {
        let mut server = test_headless_server();
        let known_theme = crate::terminal_theme::TerminalTheme {
            foreground: Some(crate::terminal_theme::RgbColor {
                r: 0x10,
                g: 0x20,
                b: 0x30,
            }),
            background: Some(crate::terminal_theme::RgbColor {
                r: 0x40,
                g: 0x50,
                b: 0x60,
            }),
            ..Default::default()
        };
        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                known_theme,
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );

        assert!(server.promote_client_to_foreground(1));
        assert_eq!(server.app.state.host_terminal_theme, known_theme);

        assert!(server.promote_client_to_foreground(2));
        assert_eq!(
            server.app.state.host_terminal_theme,
            crate::terminal_theme::TerminalTheme::default()
        );
    }

    #[test]
    fn foreground_client_appearance_controls_auto_theme() {
        let mut server = test_headless_server();
        server.app.state.theme_runtime.auto_switch = true;
        server.app.state.theme_runtime.dark_name = "catppuccin".to_string();
        server.app.state.theme_runtime.light_name = "catppuccin-latte".to_string();
        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme {
                    foreground: None,
                    background: Some(crate::terminal_theme::RgbColor { r: 0, g: 0, b: 0 }),
                    ..Default::default()
                },
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme {
                    foreground: None,
                    background: Some(crate::terminal_theme::RgbColor {
                        r: 255,
                        g: 255,
                        b: 255,
                    }),
                    ..Default::default()
                },
                None,
                2,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );

        assert!(server.promote_client_to_foreground(1));
        assert_eq!(server.app.state.theme_name, "catppuccin");

        assert!(server.promote_client_to_foreground(2));
        assert_eq!(server.app.state.theme_name, "catppuccin-latte");
    }

    #[test]
    fn color_scheme_change_event_is_inert_on_server() {
        let mut server = test_headless_server();
        let initial_theme = crate::terminal_theme::TerminalTheme {
            foreground: Some(crate::terminal_theme::RgbColor {
                r: 0x10,
                g: 0x20,
                b: 0x30,
            }),
            background: Some(crate::terminal_theme::RgbColor {
                r: 0x40,
                g: 0x50,
                b: 0x60,
            }),
            ..Default::default()
        };
        server.app.state.host_terminal_theme = initial_theme;
        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                initial_theme,
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );

        let changed = server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: crate::raw_input::GHOSTTY_COLOR_SCHEME_DARK_REPORT.to_vec(),
        });

        assert!(!changed);
        assert_eq!(server.foreground_client_id, None);
        assert_eq!(server.clients[&1].host_terminal_theme, initial_theme);
        assert_eq!(server.app.state.host_terminal_theme, initial_theme);
    }

    #[test]
    fn focus_lost_updates_client_without_promoting_foreground() {
        let mut server = test_headless_server();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                2,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(2);
        server.sync_foreground_client_state();

        let changed = server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[O".to_vec(),
        });

        assert!(!changed);
        assert_eq!(server.foreground_client_id, Some(2));
        assert_eq!(server.clients[&1].outer_terminal_focus, Some(false));
        assert_eq!(server.app.state.outer_terminal_focus, Some(true));
    }

    #[test]
    fn focus_gained_promotes_client_to_foreground() {
        let mut server = test_headless_server();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                2,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(2);
        server.sync_foreground_client_state();

        let changed = server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[I".to_vec(),
        });

        assert!(changed);
        assert_eq!(server.foreground_client_id, Some(1));
        assert_eq!(server.clients[&1].outer_terminal_focus, Some(true));
        assert_eq!(server.app.state.outer_terminal_focus, Some(true));
    }

    #[tokio::test]
    async fn foreground_focus_gained_reaches_pane_with_focus_reporting() {
        let mut server = test_headless_server();
        let mut input_rx = install_focused_test_runtime(&mut server, b"\x1b[?1004h");

        server.clients.insert(1, test_app_client(Some(false), 1));
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[I".to_vec(),
        }));
        assert_eq!(
            input_rx.try_recv().expect("forwarded focus gained report"),
            Bytes::from_static(b"\x1b[I")
        );

        assert!(!server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[O".to_vec(),
        }));
        assert_eq!(
            input_rx.try_recv().expect("forwarded focus lost report"),
            Bytes::from_static(b"\x1b[O")
        );
    }

    #[tokio::test]
    async fn outer_focus_events_do_not_reach_pane_without_focus_reporting() {
        let mut server = test_headless_server();
        let mut input_rx = install_focused_test_runtime(&mut server, b"");
        server.clients.insert(1, test_app_client(Some(false), 1));
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[I".to_vec(),
        }));
        assert!(matches!(
            input_rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn background_focus_batch_only_forwards_events_after_promotion() {
        let mut server = test_headless_server();
        let mut input_rx = install_focused_test_runtime(&mut server, b"\x1b[?1004h");
        server.clients.insert(1, test_app_client(Some(true), 1));
        server.clients.insert(2, test_app_client(Some(false), 2));
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 2,
            data: b"\x1b[O\x1b[I".to_vec(),
        }));
        assert_eq!(server.foreground_client_id, Some(2));
        assert_eq!(server.app.state.outer_terminal_focus, Some(true));
        assert_eq!(
            input_rx
                .try_recv()
                .expect("focus gained after client promotion"),
            Bytes::from_static(b"\x1b[I")
        );
        assert!(matches!(
            input_rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));
    }

    #[tokio::test]
    async fn background_client_focus_loss_releases_its_owned_keys() {
        let mut server = test_headless_server();
        let mut input_rx = install_focused_test_runtime(&mut server, b"\x1b[>15u");
        server.clients.insert(1, test_app_client(Some(true), 1));
        server.clients.insert(2, test_app_client(Some(true), 2));
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 1,
            events: vec![crate::protocol::ClientInputEvent::Key {
                code: crate::protocol::ClientKeyCode::Char('j'),
                modifiers: 0,
                kind: crate::protocol::ClientKeyKind::Press,
            }],
        }));
        server.foreground_client_id = Some(2);
        server.sync_foreground_client_state();

        assert!(!server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 1,
            events: vec![crate::protocol::ClientInputEvent::FocusLost],
        }));
        assert_eq!(
            input_rx.try_recv().expect("forwarded press"),
            Bytes::from_static(b"\x1b[106;1:1u")
        );
        assert_eq!(
            input_rx
                .try_recv()
                .expect("synthetic release from background client"),
            Bytes::from_static(b"\x1b[106;1:3u")
        );
        assert!(server.app.pressed_terminal_keys.is_empty());
    }

    #[tokio::test]
    async fn structured_outer_focus_events_reach_reporting_pane() {
        let mut server = test_headless_server();
        let mut input_rx = install_focused_test_runtime(&mut server, b"\x1b[?1004h");
        server.clients.insert(1, test_app_client(Some(true), 1));
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 1,
            events: vec![
                crate::protocol::ClientInputEvent::FocusGained,
                crate::protocol::ClientInputEvent::FocusLost,
            ],
        }));
        assert_eq!(
            input_rx.try_recv().expect("structured focus gained report"),
            Bytes::from_static(b"\x1b[I")
        );
        assert_eq!(
            input_rx.try_recv().expect("structured focus lost report"),
            Bytes::from_static(b"\x1b[O")
        );
    }

    #[tokio::test]
    async fn background_key_makes_later_focus_lost_eligible() {
        let mut server = test_headless_server();
        let mut input_rx = install_focused_test_runtime(&mut server, b"\x1b[?1004h");
        server.clients.insert(1, test_app_client(Some(true), 1));
        server.clients.insert(2, test_app_client(Some(true), 2));
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 2,
            events: vec![
                crate::protocol::ClientInputEvent::Key {
                    code: crate::protocol::ClientKeyCode::Char('x'),
                    modifiers: 0,
                    kind: crate::protocol::ClientKeyKind::Release,
                },
                crate::protocol::ClientInputEvent::FocusLost,
            ],
        }));
        assert_eq!(server.foreground_client_id, Some(2));
        assert_eq!(
            input_rx.try_recv().expect("focus lost after promotion"),
            Bytes::from_static(b"\x1b[O")
        );
    }

    #[tokio::test]
    async fn structured_non_app_focus_is_ignored_without_suppressing_keys() {
        let mut server = test_headless_server();
        let mut input_rx = install_focused_test_runtime(&mut server, b"\x1b[?1004h");
        server.clients.insert(1, test_app_client(Some(true), 1));

        let mut attached = test_app_client(Some(false), 2);
        attached.mode = ClientConnectionMode::TerminalAttach {
            terminal_id: "attached".to_owned(),
        };
        server.clients.insert(2, attached);

        let mut pending = test_app_client(Some(false), 3);
        pending.pending_terminal_attach = true;
        server.clients.insert(3, pending);
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        for client_id in [2, 3] {
            assert!(!server.handle_server_event(ServerEvent::ClientInputEvents {
                client_id,
                events: vec![crate::protocol::ClientInputEvent::FocusGained],
            }));
            assert_eq!(server.foreground_client_id, Some(1));
            assert_eq!(server.app.state.outer_terminal_focus, Some(true));
            assert_eq!(server.clients[&client_id].outer_terminal_focus, Some(false));
        }

        assert!(matches!(
            input_rx.try_recv(),
            Err(tokio::sync::mpsc::error::TryRecvError::Empty)
        ));

        assert!(server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 3,
            events: vec![crate::protocol::ClientInputEvent::Key {
                code: crate::protocol::ClientKeyCode::Char('x'),
                modifiers: 0,
                kind: crate::protocol::ClientKeyKind::Release,
            }],
        }));
        assert_eq!(server.foreground_client_id, Some(3));
    }

    fn install_focused_test_runtime(
        server: &mut HeadlessServer,
        terminal_bytes: &[u8],
    ) -> tokio::sync::mpsc::Receiver<Bytes> {
        let mut workspace = crate::workspace::Workspace::test_new("focus-reporting");
        let pane_id = workspace.tabs[0].root_pane;
        let (runtime, input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                80,
                24,
                0,
                terminal_bytes,
                4,
            );
        workspace.insert_test_runtime(pane_id, runtime);
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        input_rx
    }

    fn test_app_client(outer_terminal_focus: Option<bool>, last_activity: u64) -> ClientConnection {
        ClientConnection::new(
            (80, 24),
            crate::kitty_graphics::HostCellSize::default(),
            crate::terminal_theme::TerminalTheme::default(),
            outer_terminal_focus,
            last_activity,
            RenderEncoding::SemanticFrame,
            None,
        )
    }

    #[test]
    fn foreground_client_focus_event_updates_app_focus_state() {
        let mut server = test_headless_server();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        let changed = server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[O".to_vec(),
        });

        assert!(!changed);
        assert_eq!(server.clients[&1].outer_terminal_focus, Some(false));
        assert_eq!(server.app.state.outer_terminal_focus, Some(false));
    }

    #[test]
    fn app_client_lone_escape_closes_navigate_mode() {
        let mut server = test_headless_server();
        server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("test")];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Navigate;
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b".to_vec(),
        }));

        assert_eq!(server.app.state.mode, crate::app::Mode::Terminal);
    }

    #[test]
    fn semantic_client_input_events_route_through_app_input() {
        let mut server = test_headless_server();
        server.app.state.mode = crate::app::Mode::Onboarding;
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 1,
            events: vec![crate::protocol::ClientInputEvent::Key {
                code: crate::protocol::ClientKeyCode::Enter,
                modifiers: 0,
                kind: crate::protocol::ClientKeyKind::Press,
            }],
        }));

        assert_eq!(server.app.state.mode, crate::app::Mode::Settings);
        assert_eq!(
            server.app.state.settings.section,
            crate::app::state::SettingsSection::Integrations
        );
    }

    // TP-FCL-INPUT-01 / TP-FMR-SIDEBAR-HL-01..03: cover the real remote-client boundary that
    // the App-level FMR-2 test cannot exercise. Raw host bytes must prepare
    // the exact typed request, and the headless scheduled loop must consume it
    // into the existing Files generation.
    #[test]
    fn headless_raw_mouse_locations_navigation_loads_exact_trail() {
        use crate::app::state::{
            FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel, SidebarTab,
        };

        let root = std::env::temp_dir().join(format!(
            "headless-sidebar-mouse-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let initial = root.join("initial");
        let target = root.join("target");
        fs::create_dir_all(&initial).expect("create initial directory");
        fs::create_dir_all(&target).expect("create target directory");
        fs::write(target.join("visible.txt"), b"visible").expect("write target entry");

        let mut server = test_headless_server();
        server.clients.insert(
            1,
            ClientConnection::new(
                (106, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.app.state.mode = crate::app::Mode::Terminal;
        server.app.state.mouse_capture = true;
        server
            .app
            .state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&initial)))
            .expect("open initial Files instance");
        let generation = server
            .app
            .state
            .stage
            .active_instance_generation()
            .expect("active Files generation");
        server.app.state.sidebar_tab = SidebarTab::Files;
        server.app.state.file_manager_locations_model = FileManagerLocationsModel::from_sources(
            vec![FileManagerLocationItem {
                label: "Home".into(),
                path: target.clone(),
                icon: FileManagerLocationIcon::Home,
                accessible: true,
                ejectable: false,
            }],
            Vec::new(),
            Vec::new(),
        );
        crate::ui::compute_view(
            &mut server.app.state,
            ratatui::layout::Rect::new(0, 0, 106, 20),
        );
        let row = server.app.state.view.file_manager_locations.rows[0].clone();
        let mouse = format!(
            "\x1b[<0;{};{}M",
            row.rect.x.saturating_add(1),
            row.rect.y.saturating_add(1)
        )
        .into_bytes();

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: mouse,
        }));
        assert_eq!(
            server.app.state.request_file_manager_location_navigation,
            Some(target.clone().into()),
            "raw SGR input reaches the exact model-revalidated locations rail seam"
        );

        assert!(
            server.handle_scheduled_tasks_headless(Instant::now(), false),
            "headless scheduled tasks must consume the pending location request"
        );
        server.app.wait_file_manager_io_for_test();
        assert!(
            server.handle_scheduled_tasks_headless(Instant::now(), false),
            "headless scheduled tasks must observe the prepared root"
        );
        assert!(
            server
                .app
                .state
                .request_file_manager_location_navigation
                .is_none(),
            "the typed location request is one-shot"
        );
        assert_eq!(
            server.app.state.stage.active_instance_generation(),
            Some(generation)
        );
        let file_manager = server
            .app
            .state
            .file_manager
            .as_ref()
            .expect("loaded Files state");
        assert_eq!(file_manager.cwd, target);
        assert_eq!(file_manager.trail.cols()[0].directory, target);
        assert_eq!(
            file_manager.trail_snapshots.cols()[0].directory(),
            target.as_path()
        );

        let _ = fs::remove_dir_all(root);
    }

    /// Build a directory holding one PNG and open Files on it, with the Trail
    /// laid out so the detail panel exists.
    ///
    /// Returns the directory so the caller can remove it; every image test
    /// needs the same four steps and they are easy to get subtly wrong.
    fn headless_server_showing_one_png(
        server: &mut HeadlessServer,
        label: &str,
        frame: ratatui::layout::Rect,
    ) -> std::path::PathBuf {
        use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

        let root = std::env::temp_dir().join(format!(
            "headless-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&root).expect("create image fixture directory");

        let rgba = RgbaImage::from_fn(160, 80, |x, y| {
            Rgba([
                u8::try_from(x % 256).expect("x channel"),
                u8::try_from(y % 256).expect("y channel"),
                0x7f,
                0xff,
            ])
        });
        let mut encoded = std::io::Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(rgba)
            .write_to(&mut encoded, ImageFormat::Png)
            .expect("encode PNG fixture");
        fs::write(root.join("sample.png"), encoded.into_inner()).expect("write PNG fixture");

        // `FmState::new` reads the directory and resolves the preview on this
        // thread, so there is no background IO to wait for here. Calling
        // `wait_file_manager_io_for_test` instead blocks forever: it is an
        // unbounded condvar wait for a generation nothing ever enqueues.
        server
            .app
            .state
            .try_open_file_manager_with(|_| Some(crate::fm::FmState::new(&root)))
            .expect("open Files on the image fixture");
        crate::ui::compute_view(&mut server.app.state, frame);

        root
    }

    fn headless_image_preview_state(server: &HeadlessServer) -> &crate::fm::FmImagePreviewState {
        match &server
            .app
            .state
            .file_manager
            .as_ref()
            .expect("open Files state")
            .preview
        {
            crate::fm::FmPreview::File(crate::fm::FmFilePreview::Image(preview)) => &preview.state,
            other => panic!("expected an image preview, got {other:?}"),
        }
    }

    // The headless scheduler must drive the image preview worker.
    //
    // This is the root cause of "images never appear in server mode". The cell
    // size is set by hand here so that this test fails for exactly one reason:
    // the scheduler does not call the worker. TP-FMR-IMAGE-HL-02 covers the
    // other half.
    //
    // TP-FMR-IMAGE-HL-01
    #[test]
    fn headless_scheduler_syncs_the_image_preview_worker() {
        let frame = ratatui::layout::Rect::new(0, 0, 115, 16);
        let mut server = test_headless_server();
        server.app.image_preview_cell_size = crate::kitty_graphics::HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let root = headless_server_showing_one_png(&mut server, "image-sched", frame);

        assert!(
            server.handle_scheduled_tasks_headless(Instant::now(), false),
            "starting the image decode changes what the frame shows"
        );
        assert!(
            matches!(
                headless_image_preview_state(&server),
                crate::fm::FmImagePreviewState::Loading { .. }
            ),
            "the headless scheduler must reach the image preview worker, \
             otherwise server-mode previews stay Pending forever; got {:?}",
            headless_image_preview_state(&server)
        );

        let _ = fs::remove_dir_all(root);
    }

    // The headless scheduler must consume queued context-menu intents.
    //
    // The failure this reproduces was reported from live use: right-click →
    // "Send with Tailscale" in server mode closed the menu and nothing
    // appeared. The menu had queued the intent correctly; the headless
    // scheduler simply never called `sync_file_operation_worker`, so no queued
    // context action — this one, Enlarge, delete, copy — ever ran in server
    // mode. The monolithic twin of this journey passed the whole time, which
    // is why the seam gets its own headless test.
    //
    // TP-FSEND-TS-25
    #[test]
    fn headless_scheduler_consumes_context_menu_intents() {
        let frame = ratatui::layout::Rect::new(0, 0, 115, 16);
        let mut server = test_headless_server();
        let root = headless_server_showing_one_png(&mut server, "tailscale-sched", frame);

        let path = root.join("sample.png");
        server.app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: crate::app::state::FileManagerContextMenuAction::SendTailscale,
                paths: vec![path.clone()],
            });

        assert!(
            server.handle_scheduled_tasks_headless(Instant::now(), false),
            "consuming the intent changes what the frame shows"
        );
        assert!(
            server
                .app
                .state
                .request_file_manager_context_action
                .is_none(),
            "the intent must be consumed, not left queued forever"
        );
        assert_eq!(
            server.app.state.mode,
            crate::app::state::Mode::TailscaleSend,
            "the picker must open in server mode exactly as it does monolithic"
        );
        assert_eq!(
            server
                .app
                .state
                .tailscale_send
                .as_ref()
                .expect("picker state")
                .paths,
            vec![path]
        );

        let _ = fs::remove_dir_all(root);
    }

    // The server must publish the foreground client's cell size.
    //
    // Deliberately separate from TP-FMR-IMAGE-HL-01: a scheduler that runs
    // against a cell size of zero derives no target and still shows nothing.
    // If one of these could pass while the other fails silently, the split has
    // failed.
    //
    // TP-FMR-IMAGE-HL-02
    #[test]
    fn headless_publishes_foreground_cell_size_to_the_image_preview() {
        let mut server = test_headless_server();
        server.app.state.kitty_graphics_enabled = true;
        server.clients.insert(
            1,
            ClientConnection::new(
                (115, 16),
                crate::kitty_graphics::HostCellSize {
                    width_px: 9,
                    height_px: 18,
                },
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);

        server.sync_foreground_client_state();

        assert_eq!(
            server.app.image_preview_cell_size,
            crate::kitty_graphics::HostCellSize {
                width_px: 9,
                height_px: 18,
            },
            "the image preview decodes against the cell size of the client \
             actually looking at it, matching host_cell_size rather than \
             inventing a second policy"
        );
        assert_eq!(
            server.app.image_preview_cell_size, server.app.state.host_cell_size,
            "one resolved cell size, not two"
        );
    }

    /// Does this payload place an image?
    ///
    /// A cleared cache still emits bytes — delete commands (`a=d`) for images
    /// the terminal should drop. Asserting the payload is merely non-empty
    /// therefore passes while the picture is being taken away, which is the
    /// exact failure this suite exists to catch.
    fn places_an_image(graphics: &[u8]) -> bool {
        graphics.windows(3).any(|window| window == b"a=p")
    }

    /// Does this payload take an image off the screen?
    ///
    /// The counterpart to [`places_an_image`], and the one to reach for when
    /// asking whether a picture *stays*. The encoder sends diffs: once an image
    /// is placed, a frame that changes nothing about it carries no bytes at
    /// all. So "is it still there" cannot be read from a single frame — but
    /// "was it taken away" can, and that is the question these tests mean.
    fn deletes_an_image(graphics: &[u8]) -> bool {
        graphics.windows(3).any(|window| window == b"a=d")
    }

    /// Drive the scheduler until the image preview holds decoded pixels.
    ///
    /// Decoding happens on a worker thread, so a single scheduled round proves
    /// nothing; every image test needs this and none of them should re-invent
    /// the deadline.
    fn headless_pump_image_until_ready(server: &mut HeadlessServer) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
            if matches!(
                headless_image_preview_state(server),
                crate::fm::FmImagePreviewState::Ready { .. }
            ) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for the image decode; state {:?}",
                headless_image_preview_state(server)
            );
            std::thread::yield_now();
        }
    }

    /// Build a server whose single foreground app client has a writer and the
    /// given cell size, with a workspace present so `Mode::Terminal` is real.
    fn headless_graphics_server(
        cell_size: crate::kitty_graphics::HostCellSize,
    ) -> (HeadlessServer, std::sync::mpsc::Receiver<Vec<u8>>) {
        let mut server = test_headless_server();
        server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("test")];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        server.app.state.kitty_graphics_enabled = true;

        let (client_tx, _client_control_rx, client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (115, 20),
                cell_size,
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        (server, client_rx)
    }

    // End-to-end: the whole chain, from the scheduler driving the decode to
    // the encoder placing the result in a client frame. TP-FMR-IMAGE-HL-01 and
    // -02 each prove one link; this proves they are actually connected.
    //
    // TP-FMR-IMAGE-HL-03
    #[test]
    fn server_frame_carries_fm_image_graphics_when_ready() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let (mut server, client_rx) =
            headless_graphics_server(crate::kitty_graphics::HostCellSize {
                width_px: 8,
                height_px: 16,
            });
        let root = headless_server_showing_one_png(&mut server, "image-e2e", frame_area);

        // Decoding happens on a worker thread, so pump the scheduler until the
        // pixels land rather than assuming one round is enough.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
            if matches!(
                headless_image_preview_state(&server),
                crate::fm::FmImagePreviewState::Ready { .. }
            ) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for the server-mode image decode; state {:?}",
                headless_image_preview_state(&server)
            );
            std::thread::yield_now();
        }

        server.render_and_stream();
        let frame = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("a frame reaches the client"),
        );

        assert!(
            places_an_image(&frame.graphics),
            "a ready file manager image must reach the client as a placement, \
             not merely as some kitty graphics traffic"
        );

        let _ = fs::remove_dir_all(root);
    }

    // With several displays, the graphics must be encoded inside the same
    // viewer window the frame was rendered in.
    //
    // Found live: previews vanished the moment a second display attached. The
    // render arm restored the viewer before the graphics encode ran, so the
    // encode read whichever view the restore installed — the session default,
    // whose owned surfaces hold no file browser. One display worked only
    // because a sole display shares the register slot with the session.
    //
    // TP-MCF-CTX-06
    #[test]
    fn fm_image_graphics_reach_the_display_showing_files_with_another_display_present() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let cell_size = crate::kitty_graphics::HostCellSize {
            width_px: 8,
            height_px: 16,
        };
        let (mut server, client_rx_1) = headless_graphics_server(cell_size);

        // A second app display, same capabilities, its own writer.
        let (client_tx_2, _control_2, client_rx_2) = test_client_writer();
        server.clients.insert(
            2,
            ClientConnection::new(
                (115, 20),
                cell_size,
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                2,
                RenderEncoding::SemanticFrame,
                Some(client_tx_2),
            ),
        );

        // Seed both displays' views, then open Files with the PNG inside
        // display 1's window — exactly where a person would have opened it.
        server.render_and_stream();
        let previous = server.app.state.enter_viewer(Some(1));
        let root = headless_server_showing_one_png(&mut server, "image-two-displays", frame_area);
        server.app.state.restore_viewer(previous);

        // Drive the decode to Ready, checking inside display 1's window: with
        // several displays the register holds no file manager between serves.
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
            let previous = server.app.state.enter_viewer(Some(1));
            let ready = matches!(
                headless_image_preview_state(&server),
                crate::fm::FmImagePreviewState::Ready { .. }
            );
            server.app.state.restore_viewer(previous);
            if ready {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for the image decode with two displays"
            );
            std::thread::yield_now();
        }

        while client_rx_1.try_recv().is_ok() {}
        while client_rx_2.try_recv().is_ok() {}
        server.render_and_stream();

        let frame_1 = read_server_frame(
            client_rx_1
                .recv_timeout(Duration::from_millis(500))
                .expect("display 1 receives a frame"),
        );
        assert!(
            places_an_image(&frame_1.graphics),
            "the display showing Files must receive its preview even while \
             another display is attached"
        );

        if let Ok(bytes) = client_rx_2.recv_timeout(Duration::from_millis(500)) {
            let frame_2 = read_server_frame(bytes);
            assert!(
                !places_an_image(&frame_2.graphics),
                "a display not showing Files must not receive the preview"
            );
        }

        let _ = fs::remove_dir_all(root);
    }

    // A plugin popup opened through the API belongs to the display the person
    // is looking at, not to every display.
    //
    // The live report: clicking a preview in the file manager opened the sheet
    // viewer on every attached screen. The plugin opens it by calling back into
    // the API, so the request carries no display identity; the popup landed in
    // the session's registers and was broadcast from there.
    //
    // TP-SUR-BROADCAST-05
    #[tokio::test]
    async fn a_plugin_popup_opened_through_the_api_belongs_to_the_focused_display() {
        let mut server = test_headless_server();
        server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("test")];
        server.app.state.active = Some(0);
        server.app.state.mode = crate::app::Mode::Terminal;

        for client_id in [1u64, 2u64] {
            let (client_tx, _control, _render) = test_client_writer();
            server.clients.insert(
                client_id,
                ClientConnection::new(
                    (100, 30),
                    crate::kitty_graphics::HostCellSize::default(),
                    crate::terminal_theme::TerminalTheme::default(),
                    Some(client_id == 1),
                    client_id,
                    RenderEncoding::SemanticFrame,
                    Some(client_tx),
                ),
            );
            // Give each display a parked bundle, as attaching does.
            let previous = server.app.state.enter_viewer(Some(client_id));
            server.app.state.restore_viewer(previous);
        }
        server.foreground_client_id = Some(1);

        // Display 1 is where the person clicked; the plugin's popup arrives
        // through the API with no display identity of its own.
        let previous = server.app.state.enter_viewer(server.foreground_client_id);
        let runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"");
        server.app.install_test_popup_runtime(runtime);
        server.app.state.restore_viewer(previous);

        let previous = server.app.state.enter_viewer(Some(1));
        let focused_sees_it = server.app.state.popup_pane.is_some();
        server.app.state.restore_viewer(previous);

        let previous = server.app.state.enter_viewer(Some(2));
        let other_sees_it = server.app.state.popup_pane.is_some();
        server.app.state.restore_viewer(previous);

        assert!(
            focused_sees_it,
            "the display the person clicked in must show the popup it opened"
        );
        assert!(
            !other_sees_it,
            "a popup opened on one display must not appear on the others"
        );
    }

    // The preview keeps its image while a menu is open over it.
    //
    // Found live, and the log named it exactly: the graphics encoder refuses to
    // emit anything unless the app is in terminal mode, and it clears the
    // uploaded image on the way out. Opening the right-click menu switches the
    // mode, so the picture the menu is floating over disappears underneath it.
    //
    // The gate is right for pane graphics — a terminal app's images must not
    // paint over a modal. It is wrong for the file manager preview, which herdr
    // draws itself and keeps on screen while the menu is up.
    //
    // TP-FMR-IMAGE-HL-06
    #[test]
    fn fm_image_survives_a_context_menu_opening_over_it() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let (mut server, client_rx) =
            headless_graphics_server(crate::kitty_graphics::HostCellSize {
                width_px: 8,
                height_px: 16,
            });
        let root = headless_server_showing_one_png(&mut server, "image-menu", frame_area);

        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
            if matches!(
                headless_image_preview_state(&server),
                crate::fm::FmImagePreviewState::Ready { .. }
            ) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for the decode"
            );
            std::thread::yield_now();
        }

        server.render_and_stream();
        let first = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("first frame"),
        );
        assert!(
            places_an_image(&first.graphics),
            "baseline: the image reaches the client before the menu opens"
        );
        while client_rx.try_recv().is_ok() {}

        // Right-clicking a row puts the app in menu mode. Everything else about
        // the surface is unchanged: the same preview panel is still on screen,
        // with the menu drawn above it.
        server.app.state.mode = crate::app::Mode::ContextMenu;
        let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
        server.render_and_stream();

        let with_menu = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("a frame while the menu is open"),
        );
        assert!(
            !deletes_an_image(&with_menu.graphics),
            "opening a menu over the preview must not take the image away"
        );

        let _ = fs::remove_dir_all(root);
    }

    // A full-screen overlay takes the image away.
    //
    // The mirror image of TP-FMR-IMAGE-HL-06, and the reason that fix cannot
    // simply drop the gate. Terminal images are not cells in the frame buffer,
    // so a settings page drawn over the preview overwrites the text under the
    // picture without touching the picture: it would hang over the page.
    // A missing image is a much smaller failure than one floating over
    // unrelated content.
    //
    // TP-FMR-IMAGE-HL-07
    #[test]
    fn full_screen_overlay_takes_the_fm_image_away() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let (mut server, client_rx) =
            headless_graphics_server(crate::kitty_graphics::HostCellSize {
                width_px: 8,
                height_px: 16,
            });
        let root = headless_server_showing_one_png(&mut server, "image-fullscreen", frame_area);
        headless_pump_image_until_ready(&mut server);

        server.render_and_stream();
        let first = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("first frame"),
        );
        assert!(
            places_an_image(&first.graphics),
            "baseline: the image is on screen before the overlay opens"
        );
        while client_rx.try_recv().is_ok() {}

        server.app.state.mode = crate::app::Mode::Settings;
        let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
        server.render_and_stream();

        let covered = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("a frame while the settings page is open"),
        );
        assert!(
            deletes_an_image(&covered.graphics),
            "an overlay that covers the whole frame must take the image with \
             it; an image is not a cell, so text drawn over it leaves the \
             picture hanging on top of the page"
        );

        let _ = fs::remove_dir_all(root);
    }

    // Modes that cover nothing keep the image.
    //
    // `Prefix` and `Navigate` are not terminal mode either, so the old gate
    // silently dropped the preview for them too — the same bug wearing
    // different clothes, and the reason the fix is a classification rather than
    // a special case for menus.
    //
    // TP-FMR-IMAGE-HL-08
    #[test]
    fn transient_modes_that_cover_nothing_keep_the_fm_image() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let (mut server, client_rx) =
            headless_graphics_server(crate::kitty_graphics::HostCellSize {
                width_px: 8,
                height_px: 16,
            });
        let root = headless_server_showing_one_png(&mut server, "image-transient", frame_area);
        headless_pump_image_until_ready(&mut server);
        server.render_and_stream();
        while client_rx.try_recv().is_ok() {}

        for mode in [crate::app::Mode::Prefix, crate::app::Mode::Navigate] {
            server.app.state.mode = mode;
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
            server.render_and_stream();
            let frame = read_server_frame(
                client_rx
                    .recv_timeout(Duration::from_millis(500))
                    .unwrap_or_else(|_| panic!("a frame in {mode:?}")),
            );
            assert!(
                !deletes_an_image(&frame.graphics),
                "{mode:?} covers nothing, so the preview keeps its image"
            );
            while client_rx.try_recv().is_ok() {}
        }

        let _ = fs::remove_dir_all(root);
    }

    // An open-but-backgrounded Files tab does not claim the placement pass.
    //
    // `TP-FTAB-INPUT-02` in graphics form: reading "is the file manager open"
    // rather than "does it own the stage" hands the pass to a surface nobody is
    // looking at, and the visible terminal loses its own images.
    //
    // TP-FMR-IMAGE-HL-09
    #[test]
    fn backgrounded_files_tab_does_not_claim_the_placement_pass() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let (mut server, client_rx) =
            headless_graphics_server(crate::kitty_graphics::HostCellSize {
                width_px: 8,
                height_px: 16,
            });
        let root = headless_server_showing_one_png(&mut server, "image-bg", frame_area);
        headless_pump_image_until_ready(&mut server);
        server.render_and_stream();
        let first = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("first frame"),
        );
        assert!(places_an_image(&first.graphics), "baseline");
        while client_rx.try_recv().is_ok() {}

        // Files stays open; the terminal workspace takes the stage.
        server.app.state.show_terminal_workspace();
        assert!(
            server.app.state.file_manager.is_some(),
            "the Files tab is still open, which is exactly the trap"
        );
        let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
        server.render_and_stream();

        let backgrounded = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("a frame on the terminal workspace"),
        );
        assert!(
            deletes_an_image(&backgrounded.graphics),
            "a backgrounded Files tab must not leave its preview over the \
             terminal the user is actually looking at"
        );

        let _ = fs::remove_dir_all(root);
    }

    // An image still reaches the client after the user leaves the Files
    // surface and comes back.
    //
    // Reported from a live run: the preview drew correctly, then opening a file
    // in a new tab and returning left the panel blank for the rest of the
    // session. Backgrounding the surface is an ordinary thing to do, so a
    // preview that only survives until the first tab switch is barely a
    // preview.
    //
    // TP-FMR-IMAGE-HL-05
    #[test]
    fn fm_image_graphics_return_after_leaving_and_reentering_the_files_surface() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let (mut server, client_rx) =
            headless_graphics_server(crate::kitty_graphics::HostCellSize {
                width_px: 8,
                height_px: 16,
            });
        let root = headless_server_showing_one_png(&mut server, "image-return", frame_area);
        let files_instance = server
            .app
            .state
            .stage
            .app_tab_instances()
            .next()
            .expect("the open Files surface owns a strip entry");

        let pump_until_ready = |server: &mut HeadlessServer| {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
                if matches!(
                    headless_image_preview_state(server),
                    crate::fm::FmImagePreviewState::Ready { .. }
                ) {
                    return;
                }
                assert!(
                    Instant::now() < deadline,
                    "timed out waiting for the image decode; state {:?}",
                    headless_image_preview_state(server)
                );
                std::thread::yield_now();
            }
        };

        pump_until_ready(&mut server);
        server.render_and_stream();
        let first = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("first frame"),
        );
        assert!(
            places_an_image(&first.graphics),
            "baseline: the image must reach the client before any tab switch"
        );

        // Leave Files for the terminal workspace, exactly as opening a file in
        // a new tab does.
        server.app.state.show_terminal_workspace();
        server.render_and_stream();
        while client_rx.try_recv().is_ok() {}

        // Come back.
        assert!(
            server.app.state.activate_stage_instance(files_instance),
            "the resident Files instance stays activatable"
        );
        crate::ui::compute_view(&mut server.app.state, frame_area);
        pump_until_ready(&mut server);
        server.render_and_stream();

        let returned = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("a frame after returning to Files"),
        );
        assert!(
            places_an_image(&returned.graphics),
            "the image must be placed again after returning to Files, not only \
             on the first visit"
        );

        let _ = fs::remove_dir_all(root);
    }

    // A client that never reported its cell size gets no graphics and no
    // panic.
    //
    // The fail-safe matters more than it looks: kitty scales to exactly fill
    // whatever cell box it is given, so guessing a cell size would stretch the
    // image rather than degrade gracefully. Sending nothing is the honest
    // outcome.
    //
    // TP-FMR-IMAGE-HL-04
    #[test]
    fn unknown_cell_size_client_gets_no_graphics_and_no_panic() {
        let frame_area = ratatui::layout::Rect::new(0, 0, 115, 20);
        let (mut server, client_rx) =
            headless_graphics_server(crate::kitty_graphics::HostCellSize::default());
        let root = headless_server_showing_one_png(&mut server, "image-nocell", frame_area);

        assert_eq!(
            server.app.image_preview_cell_size,
            crate::kitty_graphics::HostCellSize::default(),
            "an unknown client cell size must not be replaced by a guess"
        );

        for _ in 0..5 {
            let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
        }
        server.render_and_stream();

        let frame = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(500))
                .expect("a frame still reaches the client"),
        );
        assert!(
            frame.graphics.is_empty(),
            "no cell size means no placement, not a stretched one"
        );

        let _ = fs::remove_dir_all(root);
    }

    /// Collect the `sync_*` / `refresh_*` calls a scheduler body makes.
    ///
    /// Reads source text rather than instrumenting the call, because the thing
    /// being guarded is precisely that a call is *absent* — and an absent call
    /// leaves no runtime trace to observe.
    fn scheduler_calls(source: &str, signature: &str) -> std::collections::BTreeSet<String> {
        let start = source
            .find(signature)
            .unwrap_or_else(|| panic!("scheduler signature not found: {signature}"));
        let mut closed = false;
        let mut statements: Vec<&str> = Vec::new();
        for line in source[start..].lines().skip(1) {
            // Both schedulers are methods in an `impl` block, so their body
            // ends at the first four-space closing brace.
            if line == "    }" {
                closed = true;
                break;
            }
            let code = line.split("//").next().unwrap_or("").trim();
            if !code.is_empty() {
                statements.push(code);
            }
        }
        assert!(closed, "scheduler body never closed: {signature}");

        // Flatten to one stream and close the gaps around `.`, because
        // rustfmt wraps long chains: `self.app\n    .refresh_x(...)`. A
        // line-by-line scan reports those as missing when they are present.
        let body = statements.join(" ").replace(" .", ".").replace(". ", ".");

        // The monolithic loop calls `self.sync_x()`; the headless one goes
        // through `self.app.sync_x()`. Both spellings mean the same step, so
        // both must be recognised — matching only `self.` silently yields an
        // empty set for the headless side.
        let mut calls = std::collections::BTreeSet::new();
        for prefix in ["self.app.", "self."] {
            let mut rest = body.as_str();
            while let Some(at) = rest.find(prefix) {
                let after = &rest[at + prefix.len()..];
                let end = after
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(after.len());
                let name = &after[..end];
                // `tick_` joined the list after a sampler landed in the
                // monolithic scheduler only. Every test stayed green, the
                // binary was correct, and a live herdr drew `--` forever,
                // because the loop the server actually runs never called it.
                // The guard for that class existed and was one prefix too
                // narrow to see it.
                if after[end..].starts_with('(')
                    && (name.starts_with("sync_")
                        || name.starts_with("refresh_")
                        || name.starts_with("tick_"))
                {
                    calls.insert(name.to_string());
                }
                rest = after;
            }
        }
        calls
    }

    // The two schedulers must agree, and every difference must be named.
    //
    // The image-preview markers fix one missing call; this makes the whole
    // class visible. The headless scheduler drifted from the monolithic one
    // silently, and nothing failed until someone noticed a preview that stayed
    // blank in server mode.
    //
    // Closing a gap makes this test fail until the gap is removed from the
    // list. That is deliberate: the list must describe the tree, not excuse it.
    //
    // TP-SRV-SCHED-PARITY-01
    #[test]
    fn scheduler_parity_headless_vs_monolithic() {
        const MONOLITHIC_SOURCE: &str = include_str!("../app/runtime.rs");
        const HEADLESS_SOURCE: &str = include_str!("headless.rs");

        // The headless server has no terminal of its own, so its animation
        // timer is a different function rather than a missing one.
        const RENAMED: [(&str, &str); 1] =
            [("sync_animation_timer", "sync_headless_animation_timer")];

        // Calls the headless scheduler still does not make. Emptied on
        // 2026-07-26: the six named gaps stopped being theoretical when a user
        // right-clicked "Send with Tailscale" in server mode and nothing
        // happened — the menu queued the intent and no scheduler call ever
        // consumed it. Every intent-consuming sync now runs in both loops.
        const KNOWN_HEADLESS_GAPS: [&str; 0] = [];

        let monolithic = scheduler_calls(
            MONOLITHIC_SOURCE,
            "pub(crate) fn handle_scheduled_tasks(&mut self",
        );
        let headless = scheduler_calls(
            HEADLESS_SOURCE,
            "fn handle_scheduled_tasks_headless(&mut self",
        );
        assert!(
            !monolithic.is_empty() && !headless.is_empty(),
            "call extraction found nothing, so this test would pass vacuously"
        );

        let mut expected_in_headless: std::collections::BTreeSet<String> = monolithic
            .iter()
            .filter(|call| !KNOWN_HEADLESS_GAPS.contains(&call.as_str()))
            .map(|call| {
                RENAMED
                    .iter()
                    .find(|(from, _)| from == call)
                    .map_or_else(|| call.clone(), |(_, to)| (*to).to_string())
            })
            .collect();
        // A headless-only call is a difference too, and must be justified the
        // same way; today there are none beyond the rename.
        expected_in_headless.extend(
            headless
                .iter()
                .filter(|call| {
                    RENAMED.iter().any(|(_, to)| *to == call.as_str())
                        && !monolithic.contains(*call)
                })
                .cloned(),
        );

        let missing: Vec<_> = expected_in_headless.difference(&headless).collect();
        assert!(
            missing.is_empty(),
            "the headless scheduler is missing calls the monolithic loop makes, \
             and they are not in the named difference list: {missing:?}"
        );

        let unexpected: Vec<_> = headless.difference(&expected_in_headless).collect();
        assert!(
            unexpected.is_empty(),
            "the headless scheduler makes calls the monolithic loop does not, \
             which needs a stated reason: {unexpected:?}"
        );

        let closed: Vec<_> = KNOWN_HEADLESS_GAPS
            .iter()
            .filter(|gap| headless.contains(**gap))
            .collect();
        assert!(
            closed.is_empty(),
            "these gaps are closed — remove them from KNOWN_HEADLESS_GAPS so the \
             list keeps describing the tree: {closed:?}"
        );

        let vanished: Vec<_> = KNOWN_HEADLESS_GAPS
            .iter()
            .filter(|gap| !monolithic.contains(**gap))
            .collect();
        assert!(
            vanished.is_empty(),
            "these gaps no longer exist in the monolithic loop either, so the \
             entry is stale: {vanished:?}"
        );
    }

    /// Install a one-action file plugin into a server's app and return its root.
    #[cfg(unix)]
    fn headless_link_file_action_plugin(server: &mut HeadlessServer, label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "headless-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&root).expect("create plugin root");
        fs::write(
            root.join("herdr-plugin.toml"),
            r#"
id = "example.fm-headless"
name = "FM Headless"
version = "0.1.0"
min_herdr_version = "0.6.10"

[[actions]]
id = "inspect"
title = "Inspect"
contexts = ["file"]
command = ["sh", "-c", "printf '%s' \"$HERDR_PLUGIN_ACTION_ID\""]
"#,
        )
        .expect("write plugin manifest");

        let linked = server.app.handle_api_request(crate::api::schema::Request {
            id: "link".into(),
            method: crate::api::schema::Method::PluginLink(crate::api::schema::PluginLinkParams {
                path: root.display().to_string(),
                enabled: true,
                source: None,
            }),
        });
        assert!(linked.contains("plugin_linked"), "expected link: {linked}");

        root
    }

    // A file-manager plugin intent chosen from the context menu is consumed and
    // executed by the headless scheduled loop, exactly as by the monolithic one.
    //
    // Without this the right-click menu is dead over a socket: the menu builds,
    // the intent is prepared, and then nothing ever consumes it. The monolithic
    // loop has driven this since the feature landed (TP-C6.3-AUTHORITY); the
    // server loop never did.
    //
    // TP-FMR-PLUGIN-HL-01
    #[cfg(unix)]
    #[test]
    fn headless_scheduler_runs_file_manager_plugin_intent_once() {
        let mut server = test_headless_server();
        let plugin_root = headless_link_file_action_plugin(&mut server, "plugin-intent");

        let files_root = std::env::temp_dir().join(format!(
            "headless-plugin-files-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&files_root).expect("create files root");
        let selected = files_root.join("selected.txt");
        fs::write(&selected, b"selected").expect("write selected file");

        let mut file_manager = crate::fm::FmState::new(&files_root);
        assert!(file_manager.replace_selection(0));
        server
            .app
            .state
            .try_open_file_manager_with(|_| Some(file_manager))
            .expect("Files activation");
        server.app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: crate::app::state::FileManagerContextMenuAction::Plugin {
                    plugin_id: "example.fm-headless".into(),
                    action_id: "inspect".into(),
                },
                paths: vec![selected.clone()],
            });

        let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);

        assert!(
            server
                .app
                .state
                .request_file_manager_context_action
                .is_none(),
            "the headless scheduler must consume the typed plugin intent"
        );
        assert_eq!(
            server.app.state.plugin_command_logs.len(),
            1,
            "the intent must reach the existing command runtime"
        );
        assert_eq!(
            server.app.state.plugin_command_logs[0].action_id.as_deref(),
            Some("inspect")
        );

        // A second round must not re-run it: the intent is one-shot.
        let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);
        assert_eq!(
            server.app.state.plugin_command_logs.len(),
            1,
            "a consumed intent cannot run twice"
        );

        let _ = fs::remove_dir_all(files_root);
        let _ = fs::remove_dir_all(plugin_root);
    }

    // An intent whose file manager has since closed is consumed but not run.
    //
    // Closing retires the authority (TP-C6.3-LIFECYCLE). If the server loop
    // consumed without revalidating, a menu choice prepared before close would
    // execute after a same-directory reopen against paths that only look the
    // same.
    //
    // TP-FMR-PLUGIN-HL-02
    #[cfg(unix)]
    #[test]
    fn headless_scheduler_drops_plugin_intent_whose_files_surface_closed() {
        let mut server = test_headless_server();
        let plugin_root = headless_link_file_action_plugin(&mut server, "plugin-stale");

        server.app.state.request_file_manager_context_action =
            Some(crate::app::state::FileManagerContextActionIntent {
                action: crate::app::state::FileManagerContextMenuAction::Plugin {
                    plugin_id: "example.fm-headless".into(),
                    action_id: "inspect".into(),
                },
                paths: vec![PathBuf::from("/herdr-test-missing-fm-root/selected.txt")],
            });

        let _ = server.handle_scheduled_tasks_headless(Instant::now(), false);

        assert!(
            server
                .app
                .state
                .request_file_manager_context_action
                .is_none(),
            "a stale intent is still consumed, so it cannot linger and fire later"
        );
        assert!(
            server.app.state.plugin_command_logs.is_empty(),
            "an intent with no live Files authority must not execute"
        );

        let _ = fs::remove_dir_all(plugin_root);
    }

    #[test]
    fn semantic_client_escape_closes_keybind_help() {
        let mut server = test_headless_server();
        server.app.state.mode = crate::app::Mode::KeybindHelp;
        server.clients.insert(
            1,
            ClientConnection::new(
                (100, 30),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        assert!(server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 1,
            events: vec![crate::protocol::ClientInputEvent::Key {
                code: crate::protocol::ClientKeyCode::Esc,
                modifiers: 0,
                kind: crate::protocol::ClientKeyKind::Press,
            }],
        }));

        assert_eq!(server.app.state.mode, crate::app::Mode::Navigate);
    }

    #[test]
    fn semantic_client_down_scrolls_keybind_help() {
        let mut server = test_headless_server();
        server.app.state.mode = crate::app::Mode::KeybindHelp;
        server.clients.insert(
            1,
            ClientConnection::new(
                (100, 30),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        assert!(server.app.state.keybind_help_max_scroll() > 0);
        assert!(server.handle_server_event(ServerEvent::ClientInputEvents {
            client_id: 1,
            events: vec![crate::protocol::ClientInputEvent::Key {
                code: crate::protocol::ClientKeyCode::Down,
                modifiers: 0,
                kind: crate::protocol::ClientKeyKind::Press,
            }],
        }));

        assert_eq!(server.app.state.mode, crate::app::Mode::KeybindHelp);
        assert_eq!(server.app.state.keybind_help.scroll, 1);
    }

    #[tokio::test]
    async fn split_default_background_response_updates_theme_without_forwarding_tail() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let focused = workspace.focused_pane_id().unwrap();
        let (runtime, mut rx) =
            crate::terminal::TerminalRuntime::test_with_channel_capacity(80, 24, 1);
        workspace.tabs[0].runtimes.insert(focused, runtime);
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(true),
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        let _ = server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b]".to_vec(),
        });
        assert!(rx.try_recv().is_err());

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"11;#123456\x07".to_vec(),
        }));

        assert!(rx.try_recv().is_err());
        assert_eq!(
            server.clients[&1].host_terminal_theme.background,
            Some(crate::terminal_theme::RgbColor {
                r: 0x12,
                g: 0x34,
                b: 0x56,
            })
        );
        assert_eq!(
            server.app.state.host_terminal_theme.background,
            Some(crate::terminal_theme::RgbColor {
                r: 0x12,
                g: 0x34,
                b: 0x56,
            })
        );
    }

    #[tokio::test]
    async fn render_and_stream_uses_each_client_terminal_size() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let active_pane = workspace.tabs[0].root_pane;
        let background_tab = workspace.test_add_tab(Some("background"));
        let background_pane = workspace.tabs[background_tab].root_pane;
        workspace.tabs[0].runtimes.insert(
            active_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"active"),
        );
        workspace.tabs[background_tab].runtimes.insert(
            background_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"background"),
        );
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (desktop_tx, _desktop_control_rx, desktop_rx) = test_client_writer();
        let (mobile_tx, _mobile_control_rx, mobile_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(desktop_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (44, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(mobile_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        server.render_and_stream();

        let desktop_frame = read_server_frame(desktop_rx.recv().expect("desktop frame"));
        let mobile_frame = read_server_frame(mobile_rx.recv().expect("mobile frame"));

        assert_eq!((desktop_frame.width, desktop_frame.height), (120, 40));
        assert_eq!((mobile_frame.width, mobile_frame.height), (44, 20));
        let mobile_text = frame_text(&mobile_frame);
        let mut mobile_rows = mobile_text.lines();
        let mobile_header = mobile_rows.by_ref().take(2).collect::<String>();
        let mobile_surface = mobile_rows.collect::<String>();
        assert!(mobile_header.contains("test"), "header: {mobile_header:?}");
        assert!(
            mobile_surface.contains("active"),
            "surface: {mobile_surface:?}"
        );
        assert!(!mobile_surface.contains("background"));

        // The shipped sidebar is 30 columns wide since the Spaces tree
        // (TP-TREE-13); this test is about per-client sizing, not the width.
        let foreground_terminal_area = Rect::new(30, 1, 90, 39);
        assert_eq!(
            server.app.state.view.layout,
            crate::app::state::ViewLayout::Desktop
        );
        assert_eq!(server.app.state.view.mobile_header_rect, Rect::default());
        assert_eq!(
            server.app.state.view.terminal_area,
            foreground_terminal_area
        );

        // Both displays adopted the same tab, so it is sized to the smaller of
        // them rather than to whichever one is in the foreground. This
        // replaces the retired contract, where the latest active client drove
        // one shared size and the other display was letterboxed or clipped
        // depending on which one had been touched last.
        //
        // TP-MCF-SIZE-02
        let (active_rows, active_cols) =
            server.app.state.workspaces[0].tabs[0].runtimes[&active_pane].current_size();
        assert!(
            active_rows <= 20 && active_cols <= 44,
            "a tab both displays watch must fit the smaller one, got {active_rows}x{active_cols}"
        );

        // Nobody is watching the background tab, so render passes leave it
        // alone; it keeps the size the size-change event path last gave it
        // (here, its spawn size). TP-MCF-SIZE-03
        let (background_rows, background_cols) =
            server.app.state.workspaces[0].tabs[background_tab].runtimes[&background_pane]
                .current_size();
        assert!(
            background_rows > 0 && background_cols > 0,
            "an unwatched tab still gets a usable size"
        );
    }

    fn pointer_move(column: u16, row: u16) -> crate::raw_input::RawInputEvent {
        crate::raw_input::RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Moved,
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::empty(),
        })
    }

    fn wheel_up(column: u16, row: u16) -> crate::raw_input::RawInputEvent {
        crate::raw_input::RawInputEvent::Mouse(crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column,
            row,
            modifiers: crossterm::event::KeyModifiers::empty(),
        })
    }

    /// TP-MOB-74: a phone attached beside a desktop still gets the phone's
    /// wheel rule. The rule reads `view.layout`, and the view is one shared
    /// structure that the last render leaves behind — so with a wide display
    /// drawing last, a swipe from the narrow one was decided against the wide
    /// display's geometry and went to the agent, which discards it. The
    /// reader saw touch scrolling stop working the moment a second machine
    /// attached.
    #[tokio::test]
    async fn a_narrow_display_keeps_its_wheel_rule_while_a_wide_one_draws() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let pane_id = workspace.tabs[0].root_pane;
        let mut bytes = b"\x1b[?1000h\x1b[?1006h".to_vec();
        for line in 0..80 {
            bytes.extend_from_slice(format!("line {line:02}\r\n").as_bytes());
        }
        let (runtime, _input_rx) =
            crate::terminal::TerminalRuntime::test_with_channel_and_scrollback_bytes(
                58,
                16,
                16 * 1024,
                &bytes,
                4,
            );
        workspace.insert_test_runtime(pane_id, runtime);
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        server.app.state.mouse_scroll_lines = 3;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        // The wide display is in front, so it is the geometry the last frame
        // leaves behind.
        server.foreground_client_id = Some(1);
        server.render_and_stream();
        assert_eq!(
            server.app.state.view.layout,
            crate::app::state::ViewLayout::Desktop,
            "the wide display drew last"
        );

        let before = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .map(|metrics| metrics.offset_from_bottom)
            .expect("scroll metrics");

        server.handle_client_input_events(2, vec![wheel_up(4, 6)]);

        let after = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .and_then(crate::terminal::TerminalRuntime::scroll_metrics)
            .map(|metrics| metrics.offset_from_bottom)
            .expect("scroll metrics");
        assert!(
            after > before,
            "the narrow display's swipe scrolled nothing: {before} -> {after}"
        );
    }

    /// Correct hit geometry must not cost a layout pass per pointer motion.
    /// The input loop is serial, so that cost lands directly on input latency
    /// — which is what the inert-motion render gate exists to avoid.
    ///
    /// TP-MCF-VIEW-02
    #[tokio::test]
    async fn a_pointer_burst_recomputes_geometry_once_and_the_foreground_never_pays() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        workspace.test_add_tab(Some("second"));
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.render_and_stream();

        // The frame leaves the geometry with the display that drew last, which
        // is the foreground one. Its own pointer motion needs no recompute.
        server.view_recomputes_for_input = 0;
        for step in 0..8 {
            server.handle_client_input_events(1, vec![pointer_move(step + 1, 1)]);
        }
        assert_eq!(
            server.view_recomputes_for_input, 0,
            "the foreground display's pointer motion must be free"
        );

        // Another display pays once for the burst, not once per motion.
        for step in 0..8 {
            server.handle_client_input_events(2, vec![pointer_move(step + 1, 1)]);
        }
        assert_eq!(
            server.view_recomputes_for_input, 1,
            "a pointer burst from one display recomputes once and then rides the ownership"
        );
    }

    /// Each display draws the tab it is on, at the same moment, in one frame
    /// pass. This is the whole feature seen from the outside: five tabs on
    /// five monitors, all live.
    ///
    /// TP-MCF-UI-01
    #[tokio::test]
    async fn every_display_draws_its_own_tab_in_the_same_frame() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let first_pane = workspace.tabs[0].root_pane;
        let second_tab = workspace.test_add_tab(Some("second"));
        let second_pane = workspace.tabs[second_tab].root_pane;
        workspace.tabs[0].runtimes.insert(
            first_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"AGENT-ALPHA"),
        );
        workspace.tabs[second_tab].runtimes.insert(
            second_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"AGENT-BETA"),
        );
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (one_tx, _one_control_rx, one_rx) = test_client_writer();
        let (two_tx, _two_control_rx, two_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(one_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(two_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        // Both displays attach on the same tab, then the second moves.
        server.render_and_stream();
        while one_rx.try_recv().is_ok() {}
        while two_rx.try_recv().is_ok() {}

        let previous = server.app.state.enter_viewer(Some(2));
        server.app.state.workspaces[0].set_active_tab(second_tab);
        server.app.state.restore_viewer(previous);

        server.render_and_stream();

        // The second display moved, so it gets a frame. The first display did
        // not, so identical-frame suppression may send it nothing at all --
        // and that silence is itself the proof it was not dragged along.
        let two_text = frame_text(&read_server_frame(two_rx.recv().expect("second frame")));
        assert!(
            two_text.contains("AGENT-BETA"),
            "the second display draws the tab it moved to: {two_text:?}"
        );
        assert!(
            !two_text.contains("AGENT-ALPHA"),
            "the second display must not still be showing the first display's agent"
        );

        match one_rx.try_recv() {
            Ok(frame) => {
                let one_text = frame_text(&read_server_frame(frame));
                assert!(
                    one_text.contains("AGENT-ALPHA"),
                    "the first display keeps drawing its own agent: {one_text:?}"
                );
                assert!(
                    !one_text.contains("AGENT-BETA"),
                    "the first display must not be pulled onto the tab the second opened"
                );
            }
            Err(_) => {
                // No frame means nothing about that display changed. Confirm
                // that from state as well, so an unrelated send failure cannot
                // masquerade as a pass.
                let previous = server.app.state.enter_viewer(Some(1));
                let tab = server.app.state.workspaces[0].active_tab_index();
                server.app.state.restore_viewer(previous);
                assert_eq!(tab, 0, "the untouched display is still on its own tab");
            }
        }
    }

    // TP-MCF-SIZE-01
    #[tokio::test]
    async fn each_display_sizes_the_tab_it_alone_is_watching() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let first_pane = workspace.tabs[0].root_pane;
        let second_tab = workspace.test_add_tab(Some("second"));
        let second_pane = workspace.tabs[second_tab].root_pane;
        workspace.tabs[0].runtimes.insert(
            first_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"first"),
        );
        workspace.tabs[second_tab].runtimes.insert(
            second_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"second"),
        );
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        // Both displays attach and adopt the tab the session is on. Doing this
        // before the switch matters: a display takes its own tab the first
        // time it is seen, so moving one of them beforehand would hand the
        // other the same tab through the default.
        server.render_and_stream();

        // Now put the narrow display on its own tab. Each tab is watched by
        // exactly one display, which is the case that must cost nothing.
        let previous = server.app.state.enter_viewer(Some(2));
        server.app.state.workspaces[0].set_active_tab(second_tab);
        server.app.state.restore_viewer(previous);

        server.render_and_stream();

        let (wide_rows, wide_cols) =
            server.app.state.workspaces[0].tabs[0].runtimes[&first_pane].current_size();
        let (narrow_rows, narrow_cols) =
            server.app.state.workspaces[0].tabs[second_tab].runtimes[&second_pane].current_size();

        assert!(
            wide_cols > 60,
            "the tab only the wide display watches keeps the wide width, got {wide_cols}"
        );
        assert!(
            wide_rows > 20,
            "the tab only the wide display watches keeps the wide height, got {wide_rows}"
        );
        assert!(
            narrow_cols <= 60 && narrow_rows <= 20,
            "the tab only the narrow display watches fits the narrow display, got {narrow_rows}x{narrow_cols}"
        );
    }

    // TP-MCF-SIZE-05
    #[tokio::test]
    async fn same_tab_index_in_different_workspaces_sizes_independently() {
        let mut server = test_headless_server();
        let mut left = crate::workspace::Workspace::test_new("left");
        let left_pane = left.tabs[0].root_pane;
        left.tabs[0].runtimes.insert(
            left_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"left"),
        );
        let mut right = crate::workspace::Workspace::test_new("right");
        let right_pane = right.tabs[0].root_pane;
        right.tabs[0].runtimes.insert(
            right_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"right"),
        );
        server.app.state.workspaces = vec![left, right];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        // Both displays attach on the session's workspace, then the narrow one
        // moves to the other. Each workspace has exactly one tab, so both
        // displays are now watching a tab whose index is 0 -- in different
        // workspaces.
        server.render_and_stream();
        server.app.state.enter_viewer(Some(2));
        server.app.state.active = Some(1);
        server.app.state.restore_viewer(None);

        server.render_and_stream();

        let (_, left_cols) =
            server.app.state.workspaces[0].tabs[0].runtimes[&left_pane].current_size();
        let (_, right_cols) =
            server.app.state.workspaces[1].tabs[0].runtimes[&right_pane].current_size();

        assert!(
            left_cols > 60,
            "the wide display's workspace keeps the wide width; a tab index is not \
             an identity across workspaces, so the narrow display's tab 0 must not \
             drag it down, got {left_cols}"
        );
        assert!(
            right_cols <= 60,
            "the narrow display's workspace fits the narrow display, got {right_cols}"
        );
    }

    // TP-MCF-SIZE-04
    #[tokio::test]
    async fn moving_input_between_displays_costs_no_background_resize() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let first_pane = workspace.tabs[0].root_pane;
        let background_tab = workspace.test_add_tab(Some("background"));
        let background_pane = workspace.tabs[background_tab].root_pane;
        for (tab, pane) in [(0, first_pane), (background_tab, background_pane)] {
            workspace.tabs[tab].runtimes.insert(
                pane,
                crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"x"),
            );
        }
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        // A real size-change event settles every tab, background ones included.
        server.resize_shared_runtime_to_effective_size();
        server.render_and_stream();

        // Typing in the other window makes it foreground. That is a change of
        // input focus, not a change of session geometry: the tab being typed
        // in follows the new display, but a tab nobody is watching has no
        // reason to be rewritten -- and rewriting it reflows its scrollback.
        let before = server.app.state.workspaces[0].tabs[background_tab].runtimes[&background_pane]
            .applied_resizes_for_test();
        for client_id in [2, 1, 2] {
            if server.promote_client_to_foreground(client_id) {
                server.resize_shared_runtime_to_effective_size_before_input();
            }
        }
        let after = server.app.state.workspaces[0].tabs[background_tab].runtimes[&background_pane]
            .applied_resizes_for_test();

        assert_eq!(
            after,
            before,
            "moving input focus between displays must not resize an unwatched tab; \
             it was rewritten to each display's geometry in turn, reflowing its whole \
             scrollback every time ({} resizes over three focus changes)",
            after - before
        );
    }

    // TP-MCF-SIZE-03
    #[tokio::test]
    async fn a_steady_frame_costs_no_background_resize() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let first_pane = workspace.tabs[0].root_pane;
        let second_tab = workspace.test_add_tab(Some("second"));
        let second_pane = workspace.tabs[second_tab].root_pane;
        let background_tab = workspace.test_add_tab(Some("background"));
        let background_pane = workspace.tabs[background_tab].root_pane;
        for (tab, pane) in [
            (0, first_pane),
            (second_tab, second_pane),
            (background_tab, background_pane),
        ] {
            workspace.tabs[tab].runtimes.insert(
                pane,
                crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"x"),
            );
        }
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        // Two displays of different sizes. The third tab is watched by neither,
        // so it is swept as a background tab by both render passes.
        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        // Let both displays attach and adopt a tab, then put the narrow one on
        // its own tab so each display watches a different tab and the third is
        // background for both.
        server.render_and_stream();
        let previous = server.app.state.enter_viewer(Some(2));
        server.app.state.workspaces[0].set_active_tab(second_tab);
        server.app.state.restore_viewer(previous);
        server.render_and_stream();

        // Everything has settled: no input, no output, no size change. From here
        // a frame must cost nothing on a tab nobody is watching.
        let before = server.app.state.workspaces[0].tabs[background_tab].runtimes[&background_pane]
            .applied_resizes_for_test();
        server.render_and_stream();
        server.render_and_stream();
        let after = server.app.state.workspaces[0].tabs[background_tab].runtimes[&background_pane]
            .applied_resizes_for_test();

        assert_eq!(
            after,
            before,
            "a background tab must not be resized by a steady frame; each display was \
             rewriting it to its own geometry, and every one of those reflows the whole \
             scrollback (applied {} resizes over two idle frames)",
            after - before
        );
    }

    // TP-MCF-SIZE-03 — the event-path residual: handing the foreground to a
    // display of another shape is not a size-change event for tabs nobody
    // watches. Only the input path pays it, so only the input path is pinned.
    #[tokio::test]
    async fn foreground_handoff_between_displays_does_not_resweep_background_tabs() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let watched_pane = workspace.tabs[0].root_pane;
        let background_tab = workspace.test_add_tab(Some("background"));
        let background_pane = workspace.tabs[background_tab].root_pane;
        for (tab, pane) in [(0, watched_pane), (background_tab, background_pane)] {
            workspace.tabs[tab].runtimes.insert(
                pane,
                crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"x"),
            );
        }
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();
        server.render_and_stream();

        // Typing lands on the narrow display, then back on the wide one. The
        // foreground hand-off resizes the shared runtime for input, but the
        // background tab is watched by neither display and must not move.
        let before = server.app.state.workspaces[0].tabs[background_tab].runtimes[&background_pane]
            .applied_resizes_for_test();
        server.promote_client_to_foreground(2);
        server.resize_shared_runtime_to_effective_size_before_input();
        server.promote_client_to_foreground(1);
        server.resize_shared_runtime_to_effective_size_before_input();
        let after = server.app.state.workspaces[0].tabs[background_tab].runtimes[&background_pane]
            .applied_resizes_for_test();

        assert_eq!(
            after,
            before,
            "a foreground hand-off between displays of different shapes must \
             not resweep background tabs (applied {} resizes across two \
             hand-offs); each one reflows that pane's whole scrollback",
            after - before
        );
    }

    // Negotiation is per tab, not per tab *index*: a session with two
    // workspaces has two tabs at index zero, and merging them would size a
    // watched tab to a display that is looking at a different workspace.
    #[tokio::test]
    async fn tabs_in_different_workspaces_do_not_share_a_size_negotiation() {
        let mut server = test_headless_server();
        let mut left = crate::workspace::Workspace::test_new("left");
        let left_pane = left.tabs[0].root_pane;
        left.tabs[0].runtimes.insert(
            left_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"left"),
        );
        let mut right = crate::workspace::Workspace::test_new("right");
        let right_pane = right.tabs[0].root_pane;
        right.tabs[0].runtimes.insert(
            right_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"right"),
        );
        server.app.state.workspaces = vec![left, right];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        // Both displays attach on the left workspace, then the narrow one
        // moves to the right workspace. Each workspace's only tab sits at
        // index zero, watched by exactly one display.
        server.render_and_stream();
        let previous = server.app.state.enter_viewer(Some(2));
        server.app.state.active = Some(1);
        server.app.state.restore_viewer(previous);
        server.render_and_stream();

        let (left_rows, left_cols) =
            server.app.state.workspaces[0].tabs[0].runtimes[&left_pane].current_size();
        let (right_rows, right_cols) =
            server.app.state.workspaces[1].tabs[0].runtimes[&right_pane].current_size();

        assert!(
            left_cols > 60,
            "the tab only the wide display watches keeps the wide width even \
             though another workspace has a tab at the same index, got {left_cols}"
        );
        assert!(
            left_rows > 20,
            "the tab only the wide display watches keeps the wide height, got {left_rows}"
        );
        assert!(
            right_cols <= 60 && right_rows <= 20,
            "the tab only the narrow display watches fits the narrow display, \
             got {right_rows}x{right_cols}"
        );
    }

    // A frame nobody sees is not drawn: with no client attached the view is
    // computed once to keep pane geometry alive for the API and the first
    // attach, and PTY output after that stops producing thrown-away frames.
    #[tokio::test]
    async fn frames_with_no_attached_client_are_computed_once_not_per_tick() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let pane = workspace.tabs[0].root_pane;
        workspace.tabs[0].runtimes.insert(
            pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b"x"),
        );
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        assert!(server.clients.is_empty());

        server.render_and_stream();
        server.render_and_stream();
        server.render_and_stream();

        assert_eq!(
            server.watcherless_virtual_frames, 1,
            "with no client attached, geometry is established once and later \
             ticks must not compute and discard full frames"
        );
        assert!(
            !server.app.state.view.pane_infos.is_empty(),
            "the one computed frame must still establish pane geometry"
        );
    }

    #[tokio::test]
    async fn resize_shared_runtime_resizes_background_tabs() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let background_tab = workspace.test_add_tab(Some("background"));
        let active_pane = workspace.tabs[0].root_pane;
        let background_pane = workspace.tabs[background_tab].root_pane;
        workspace.tabs[0].runtimes.insert(
            active_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b""),
        );
        workspace.tabs[background_tab].runtimes.insert(
            background_pane,
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b""),
        );
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        let terminal_area = server.app.state.view.terminal_area;
        let expected = (terminal_area.height, terminal_area.width.saturating_sub(1));
        assert_eq!(
            server
                .app
                .state
                .runtime_for_pane(&server.app.terminal_runtimes, active_pane)
                .unwrap()
                .current_size(),
            expected
        );
        assert_eq!(
            server
                .app
                .state
                .runtime_for_pane(&server.app.terminal_runtimes, background_pane)
                .unwrap()
                .current_size(),
            expected
        );
    }

    #[test]
    fn terminal_attach_disconnect_restores_app_pane_size() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime");
        let _runtime_guard = rt.enter();
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("test");
        let pane_id = workspace.tabs[0].root_pane;
        let terminal_id = workspace.terminal_id(pane_id).expect("terminal id").clone();
        let terminal_id_string = terminal_id.to_string();
        server.app.state.workspaces = vec![workspace];
        server.app.state.ensure_test_terminals();
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        server.app.terminal_runtimes.insert(
            terminal_id.clone(),
            crate::terminal::TerminalRuntime::test_with_screen_bytes(80, 24, b""),
        );
        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                None,
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();
        let expected_app_size = server
            .app
            .terminal_runtimes
            .get(&terminal_id)
            .expect("runtime")
            .current_size();
        assert_ne!(expected_app_size, (24, 80));

        let (writer, _control_rx, _render_rx) = test_client_writer();
        assert!(server.handle_server_event(ServerEvent::ClientConnected {
            client_id: 2,
            cols: 80,
            rows: 24,
            cell_width_px: 0,
            cell_height_px: 0,
            render_encoding: RenderEncoding::TerminalAnsi,
            keybindings: None,
            direct_attach_requested: true,
            writer,
        }));
        assert!(
            server.handle_server_event(ServerEvent::ClientAttachTerminal {
                client_id: 2,
                terminal_id: terminal_id_string.clone(),
                takeover: false,
            })
        );
        assert_eq!(server.foreground_client_id, Some(1));
        assert!(server
            .app
            .state
            .direct_attach_resize_locks
            .contains(&terminal_id));
        assert_eq!(
            server
                .app
                .terminal_runtimes
                .get(&terminal_id)
                .expect("runtime")
                .current_size(),
            (24, 80)
        );

        assert!(server.handle_server_event(ServerEvent::ClientDisconnected { client_id: 2 }));

        assert!(!server
            .app
            .state
            .direct_attach_resize_locks
            .contains(&terminal_id));
        assert_eq!(
            server
                .app
                .terminal_runtimes
                .get(&terminal_id)
                .expect("runtime")
                .current_size(),
            expected_app_size
        );
        drop(server);
        drop(_runtime_guard);
        rt.shutdown_timeout(Duration::from_millis(100));
    }

    #[test]
    fn render_and_stream_sends_terminal_frame_for_terminal_ansi_client() {
        let mut server = test_headless_server();
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::TerminalAnsi,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        server.render_and_stream();

        match read_server_message(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("terminal frame"),
        ) {
            ServerMessage::Terminal(frame) => {
                assert_eq!(frame.seq, 1);
                assert_eq!((frame.width, frame.height), (80, 24));
                assert!(frame.full);
                assert!(!frame.bytes.is_empty());
            }
            other => panic!("expected terminal frame, got {other:?}"),
        }
        assert_eq!(
            server
                .clients
                .get(&1)
                .unwrap()
                .render_state
                .terminal_seq()
                .unwrap(),
            1
        );
    }

    #[test]
    fn terminal_ansi_input_does_not_reset_blit_baseline() {
        let mut server = test_headless_server();
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::TerminalAnsi,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial terminal frame");
        assert_eq!(
            server
                .clients
                .get(&1)
                .unwrap()
                .render_state
                .terminal_seq()
                .unwrap(),
            1
        );

        assert!(!server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: Vec::new(),
        }));
        server.render_and_stream();

        assert_eq!(
            server
                .clients
                .get(&1)
                .unwrap()
                .render_state
                .terminal_seq()
                .unwrap(),
            1
        );
        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());
    }

    #[test]
    fn outer_focus_gained_repaints_terminal_ansi_without_clearing() {
        let mut server = test_headless_server();
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::TerminalAnsi,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial terminal frame");

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[I".to_vec(),
        }));
        server.render_and_stream();

        match read_server_message(client_rx.recv_timeout(Duration::from_millis(100)).unwrap()) {
            ServerMessage::Terminal(frame) => {
                assert_eq!(frame.seq, 2);
                assert!(frame.full);
                assert!(!frame.bytes.windows(4).any(|bytes| bytes == b"\x1b[2J"));
            }
            other => panic!("expected terminal frame, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn outer_focus_gained_client_render_pending_survives_semantic_render_queue_full() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");

        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial semantic frame");

        let queued = HeadlessServer::frame_server_message(&ServerMessage::ReloadSoundConfig)
            .expect("serialize dummy message");
        server
            .clients
            .get(&1)
            .unwrap()
            .writer
            .as_ref()
            .unwrap()
            .render
            .try_send(queued)
            .expect("pre-fill render queue");

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[I".to_vec(),
        }));
        assert_eq!(
            server.clients.get(&1).unwrap().deferred_render(),
            DeferredRender::Full
        );

        server.render_and_stream();

        assert_eq!(
            server.clients.get(&1).unwrap().deferred_render(),
            DeferredRender::Full
        );
        assert!(matches!(
            read_server_message(client_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
            ServerMessage::ReloadSoundConfig
        ));

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rZ");

        assert!(!server.render_retained_pty_update_and_stream());
        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());

        assert!(server.handle_server_event(ServerEvent::ClientWriterDrained { client_id: 1 }));
        server.render_and_stream();

        assert_eq!(
            server.clients.get(&1).unwrap().deferred_render(),
            DeferredRender::None
        );
        assert!(matches!(
            read_server_message(client_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
            ServerMessage::Frame(_)
        ));
    }

    #[test]
    fn outer_focus_gained_does_not_force_terminal_ansi_full_redraw_when_disabled() {
        let mut server = test_headless_server();
        server.app.state.redraw_on_focus_gained = false;
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::TerminalAnsi,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial terminal frame");

        server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[I".to_vec(),
        });
        server.render_and_stream();

        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());
        assert_eq!(server.clients[&1].outer_terminal_focus, Some(true));
        assert_eq!(server.app.state.outer_terminal_focus, Some(true));
        assert_eq!(
            server
                .clients
                .get(&1)
                .unwrap()
                .render_state
                .terminal_seq()
                .unwrap(),
            1
        );
    }

    #[test]
    fn outer_focus_gained_does_not_mark_semantic_render_pending_when_disabled() {
        let mut server = test_headless_server();
        server.app.state.redraw_on_focus_gained = false;
        let (client_tx, _client_control_rx, _client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        assert!(server.handle_server_event(ServerEvent::ClientInput {
            client_id: 1,
            data: b"\x1b[I".to_vec(),
        }));

        assert_eq!(
            server.clients.get(&1).unwrap().deferred_render(),
            DeferredRender::None
        );
        assert!(!server.app.full_redraw_pending);
        assert_eq!(server.clients[&1].outer_terminal_focus, Some(true));
        assert_eq!(server.app.state.outer_terminal_focus, Some(true));
    }

    #[test]
    fn full_render_queue_does_not_advance_terminal_ansi_baseline() {
        let mut server = test_headless_server();
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();
        let queued = HeadlessServer::frame_server_message(&ServerMessage::ReloadSoundConfig)
            .expect("serialize dummy message");
        client_tx
            .render
            .try_send(queued)
            .expect("pre-fill render queue");

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::TerminalAnsi,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        server.render_and_stream();

        assert_eq!(
            server
                .clients
                .get(&1)
                .unwrap()
                .render_state
                .terminal_seq()
                .unwrap(),
            0
        );
        assert!(matches!(
            read_server_message(client_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
            ServerMessage::ReloadSoundConfig
        ));
        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());
    }

    #[test]
    fn writer_drained_retries_pending_terminal_ansi_render() {
        let mut server = test_headless_server();
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();
        let queued = HeadlessServer::frame_server_message(&ServerMessage::ReloadSoundConfig)
            .expect("serialize dummy message");
        client_tx
            .render
            .try_send(queued)
            .expect("pre-fill render queue");

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::TerminalAnsi,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        server.render_and_stream();
        assert_eq!(
            server.clients.get(&1).unwrap().deferred_render(),
            DeferredRender::Full
        );
        assert!(matches!(
            read_server_message(client_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
            ServerMessage::ReloadSoundConfig
        ));

        assert!(server.handle_server_event(ServerEvent::ClientWriterDrained { client_id: 1 }));
        server.render_and_stream();

        match read_server_message(client_rx.recv_timeout(Duration::from_millis(100)).unwrap()) {
            ServerMessage::Terminal(frame) => assert_eq!(frame.seq, 1),
            other => panic!("expected terminal frame, got {other:?}"),
        }
        assert_eq!(
            server
                .clients
                .get(&1)
                .unwrap()
                .render_state
                .terminal_seq()
                .unwrap(),
            1
        );
        assert_eq!(
            server.clients.get(&1).unwrap().deferred_render(),
            DeferredRender::None
        );
    }

    #[test]
    fn render_and_stream_skips_identical_frame_sends() {
        let mut server = test_headless_server();
        server.app.state.workspaces = vec![crate::workspace::Workspace::test_new("test")];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (client_tx, _client_control_rx, client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        server.render_and_stream();
        let first = client_rx.recv_timeout(Duration::from_millis(100));
        assert!(first.is_ok(), "expected first frame to be sent");

        server.render_and_stream();
        assert!(
            client_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "identical frame should not be sent twice"
        );
    }

    #[test]
    fn identical_miller_frame_sends_no_payload() {
        let mut server = test_headless_server();
        open_virtual_miller_files(&mut server);
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        server.render_and_stream();
        let first = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("first Miller frame"),
        );
        assert!(
            frame_text(&first).contains("alpha.txt"),
            "first transport frame must contain the active Miller surface"
        );

        server.render_and_stream();
        assert!(
            client_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "a logically identical Miller frame must produce zero payload"
        );
        assert!(!server.clients.get(&1).expect("client").render_pending);
    }

    #[test]
    fn miller_render_queue_remains_single_slot() {
        let mut server = test_headless_server();
        open_virtual_miller_files(&mut server);
        let (client_tx, _client_control_rx, client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();
        server.resize_shared_runtime_to_effective_size();

        server.render_and_stream();
        server
            .app
            .state
            .file_manager
            .as_mut()
            .expect("open Files")
            .move_down();
        server.render_and_stream();

        assert!(
            server.clients.get(&1).expect("client").render_pending,
            "backpressure must retain one retry intent instead of enqueueing another frame"
        );
        let first = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("single queued frame"),
        );
        assert!(frame_text(&first).contains("alpha.txt"));
        assert!(
            client_rx.recv_timeout(Duration::from_millis(20)).is_err(),
            "the render channel must contain at most one pending payload"
        );

        assert!(server.handle_server_event(ServerEvent::ClientWriterDrained { client_id: 1 }));
        server.render_and_stream();
        let latest = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("retry frame after drain"),
        );
        assert_ne!(
            first, latest,
            "the retry must carry the latest Miller state"
        );
        assert!(!server.clients.get(&1).expect("client").render_pending);
        assert!(
            client_rx.recv_timeout(Duration::from_millis(20)).is_err(),
            "drain retry must still leave no second queued payload"
        );
    }

    #[tokio::test]
    async fn one_miller_row_change_does_not_force_unrelated_runtime_mutation() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"terminal sentinel");
        open_virtual_miller_files(&mut server);
        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial Files frame");
        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("test-owned terminal runtime");
        let before_size = runtime.current_size();
        let before_history = runtime.snapshot_history();

        server
            .app
            .state
            .file_manager
            .as_mut()
            .expect("open Files")
            .move_down();
        server.render_and_stream();
        let changed = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("changed Miller frame"),
        );
        assert!(frame_text(&changed).contains("bravo.txt"));

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("same test-owned terminal runtime");
        assert_eq!(runtime.current_size(), before_size);
        assert_eq!(runtime.snapshot_history(), before_history);
        shutdown_test_runtimes(&mut server);
    }

    #[tokio::test]
    async fn retained_pty_update_streams_dirty_row_from_last_frame() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        server.render_and_stream();
        let first = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("initial frame"),
        );
        assert!(first.cells.iter().any(|cell| cell.symbol == "a"));

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rZ");

        assert!(server.render_retained_pty_update_and_stream());
        let patched = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("retained frame"),
        );
        assert!(patched.cells.iter().any(|cell| cell.symbol == "Z"));
        assert_eq!((patched.width, patched.height), (80, 24));
    }

    #[tokio::test]
    async fn retained_pty_update_declines_while_popup_is_visible() {
        let (mut server, client_rx, _) = retained_test_server(b"tiled");
        let popup_runtime =
            crate::terminal::TerminalRuntime::test_with_screen_bytes(40, 12, b"popup-aaaa");
        let (_, terminal_id) = server.app.install_test_popup_runtime(popup_runtime);

        server.render_and_stream();
        let initial = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("initial popup frame"),
        );
        assert!(frame_text(&initial).contains("popup-aaaa"));
        server
            .app
            .terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .test_process_pty_bytes(b"\rZ");

        assert!(!server.render_retained_pty_update_and_stream());
        server.render_and_stream();
        let updated = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("full popup fallback frame"),
        );
        assert!(frame_text(&updated).contains("Zopup-aaaa"));
    }

    #[tokio::test]
    async fn popup_forces_host_mouse_capture_for_headless_client() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.app.state.mouse_capture = false;
        let popup_runtime =
            crate::terminal::TerminalRuntime::test_with_screen_bytes(40, 12, b"popup");
        server.app.install_test_popup_runtime(popup_runtime);

        server.stream_host_mouse_capture_mode();

        assert!(matches!(
            read_server_message(
                client_control_rx
                    .recv_timeout(Duration::from_millis(100))
                    .expect("mouse capture message")
            ),
            ServerMessage::MouseCapture { enabled: true }
        ));
    }

    #[tokio::test]
    async fn focused_report_all_pane_updates_headless_client_keyboard_flags() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        let popup_runtime =
            crate::terminal::TerminalRuntime::test_with_screen_bytes(40, 12, b"\x1b[>15u");
        server.app.install_test_popup_runtime(popup_runtime);

        server.stream_host_keyboard_enhancement_flags();

        assert!(matches!(
            read_server_message(
                client_control_rx
                    .recv_timeout(Duration::from_millis(100))
                    .expect("keyboard enhancement message")
            ),
            ServerMessage::KittyKeyboardReportAll { enabled: true }
        ));

        assert!(server.app.close_popup_pane());
        server.stream_host_keyboard_enhancement_flags();
        assert!(matches!(
            read_server_message(
                client_control_rx
                    .recv_timeout(Duration::from_millis(100))
                    .expect("IME-compatible keyboard enhancement message")
            ),
            ServerMessage::KittyKeyboardReportAll { enabled: false }
        ));
    }

    #[tokio::test]
    async fn virtual_render_uses_popup_cursor() {
        let (mut server, _client_rx, _) = retained_test_server(b"\x1b[2;2H");
        let popup_runtime =
            crate::terminal::TerminalRuntime::test_with_screen_bytes(40, 12, b"\x1b[4;5H");
        let (_, terminal_id) = server.app.install_test_popup_runtime(popup_runtime);

        let (_, cursor) = crate::server::render_stream::render_virtual_with_runtime_registry(
            &mut server.app.state,
            &server.app.terminal_runtimes,
            ratatui::layout::Rect::new(0, 0, 80, 24),
            true,
            crate::kitty_graphics::HostCellSize::default(),
        );
        let (_, inner) =
            crate::ui::popup_pane_rects(&server.app.state, server.app.state.view.terminal_area)
                .unwrap();
        let expected = server
            .app
            .terminal_runtimes
            .get(&terminal_id)
            .unwrap()
            .cursor_state(inner, true)
            .unwrap();

        assert_eq!(
            cursor,
            Some(crate::protocol::CursorState {
                x: expected.x,
                y: expected.y,
                visible: expected.visible,
                shape: expected.shape,
            })
        );
    }

    #[tokio::test]
    async fn virtual_render_does_not_resize_directly_attached_popup() {
        let (mut server, _client_rx, _) = retained_test_server(b"tiled");
        let popup_runtime = crate::terminal::TerminalRuntime::test_with_screen_bytes(50, 13, b"");
        let (_, terminal_id) = server.app.install_test_popup_runtime(popup_runtime);
        server
            .app
            .state
            .direct_attach_resize_locks
            .insert(terminal_id.clone());

        let _ = crate::server::render_stream::render_virtual_with_runtime_registry(
            &mut server.app.state,
            &server.app.terminal_runtimes,
            ratatui::layout::Rect::new(0, 0, 80, 24),
            true,
            crate::kitty_graphics::HostCellSize::default(),
        );

        assert_eq!(
            server
                .app
                .terminal_runtimes
                .get(&terminal_id)
                .unwrap()
                .current_size(),
            (13, 50)
        );
    }

    #[tokio::test]
    async fn retained_pty_update_declines_while_toast_is_visible() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        server.app.state.toast = Some(crate::app::state::ToastNotification {
            kind: crate::app::state::ToastKind::NeedsAttention,
            title: "pi needs attention".to_owned(),
            context: "background · 2".to_owned(),
            position: None,
            target: None,
        });
        server.render_and_stream();
        let initial = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("initial frame"),
        );
        assert!(
            frame_text(&initial).contains("pi needs attention"),
            "expected initial full frame to include toast text"
        );

        let toast_row = server.app.state.view.toast_hit_area.y;
        let inner_rect = server.app.state.view.pane_infos[0].inner_rect;
        let pane_row = toast_row
            .checked_sub(inner_rect.y)
            .expect("toast should overlap the pane")
            + 1;
        assert!(pane_row <= inner_rect.height);
        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(format!("\x1b[{pane_row};1Hzzzz").as_bytes());

        assert!(!server.render_retained_pty_update_and_stream());
        assert!(
            client_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "retained path should not stream a frame that can overwrite toast cells"
        );
    }

    #[tokio::test]
    async fn retained_pty_update_declines_while_copy_feedback_is_visible() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        server.app.state.copy_feedback = Some(crate::app::state::CopyFeedback {
            message: "copied to clipboard".to_owned(),
        });
        server.render_and_stream();
        let initial = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("initial frame"),
        );
        let initial_text = frame_text(&initial);
        assert!(
            initial_text.contains("copied to clipboard"),
            "expected initial full frame to include copy feedback"
        );

        let feedback_row = initial_text
            .lines()
            .position(|line| line.contains("copied to clipboard"))
            .expect("copy feedback row") as u16;
        let inner_rect = server.app.state.view.pane_infos[0].inner_rect;
        let pane_row = feedback_row
            .checked_sub(inner_rect.y)
            .expect("copy feedback should overlap the pane")
            + 1;
        assert!(pane_row <= inner_rect.height);
        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(format!("\x1b[{pane_row};1Hzzzz").as_bytes());

        assert!(!server.render_retained_pty_update_and_stream());
        assert!(
            client_rx.recv_timeout(Duration::from_millis(50)).is_err(),
            "retained path should not stream a frame that can overwrite copy feedback cells"
        );
    }

    #[tokio::test]
    async fn retained_pty_update_matches_full_render_frame() {
        let initial = b"\x1b[6 qleft \xe4\xb8\xad";
        let update = b"\r\x1b[44mZ\x1b[0m";
        let (mut retained_server, retained_rx, retained_pane_id) = retained_test_server(initial);
        let (mut full_server, full_rx, full_pane_id) = retained_test_server(initial);

        retained_server.render_and_stream();
        let _ = retained_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial retained baseline");
        full_server.render_and_stream();
        let _ = full_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial full baseline");

        retained_server
            .app
            .state
            .runtime_for_pane_in_workspace(
                &retained_server.app.terminal_runtimes,
                0,
                retained_pane_id,
            )
            .expect("retained runtime")
            .test_process_pty_bytes(update);
        full_server
            .app
            .state
            .runtime_for_pane_in_workspace(&full_server.app.terminal_runtimes, 0, full_pane_id)
            .expect("full runtime")
            .test_process_pty_bytes(update);

        assert!(retained_server.render_retained_pty_update_and_stream());
        full_server.render_and_stream();

        let retained_frame = read_server_frame(
            retained_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("retained frame"),
        );
        let full_frame = read_server_frame(
            full_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("full frame"),
        );
        assert_frame_data_eq(&retained_frame, &full_frame);
    }

    #[tokio::test]
    async fn retained_pty_update_streams_cursor_only_change() {
        let initial = b"abcd";
        let update = b"\x1b[D";
        let (mut retained_server, retained_rx, retained_pane_id) = retained_test_server(initial);
        let (mut full_server, full_rx, full_pane_id) = retained_test_server(initial);

        retained_server.render_and_stream();
        let _ = retained_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial retained baseline");
        full_server.render_and_stream();
        let _ = full_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial full baseline");

        retained_server
            .app
            .state
            .runtime_for_pane_in_workspace(
                &retained_server.app.terminal_runtimes,
                0,
                retained_pane_id,
            )
            .expect("retained runtime")
            .test_process_pty_bytes(update);
        full_server
            .app
            .state
            .runtime_for_pane_in_workspace(&full_server.app.terminal_runtimes, 0, full_pane_id)
            .expect("full runtime")
            .test_process_pty_bytes(update);

        assert!(retained_server.render_retained_pty_update_and_stream());
        full_server.render_and_stream();

        let retained_frame = read_server_frame(
            retained_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("retained cursor frame"),
        );
        let full_frame = read_server_frame(
            full_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("full cursor frame"),
        );
        assert_frame_data_eq(&retained_frame, &full_frame);
    }

    #[tokio::test]
    async fn retained_pty_update_declines_unsafe_mode_without_consuming_dirty_rows() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial frame");

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rZ");

        server.app.state.mode = crate::app::Mode::Navigate;
        assert!(!server.render_retained_pty_update_and_stream());
        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());

        server.app.state.mode = crate::app::Mode::Terminal;
        assert!(server.render_retained_pty_update_and_stream());
        let patched = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("retained frame after safe mode"),
        );
        assert!(patched.cells.iter().any(|cell| cell.symbol == "Z"));
    }

    #[tokio::test]
    async fn headless_full_render_clears_full_redraw_pending_for_future_retained_updates() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        server.app.full_redraw_pending = true;

        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("full redraw frame");
        assert!(!server.app.full_redraw_pending);

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rZ");

        assert!(server.render_retained_pty_update_and_stream());
    }

    #[tokio::test]
    async fn retained_pty_update_declines_when_patch_would_stale_hyperlinks() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"link");
        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial frame");
        let inner_rect = server.app.state.view.pane_infos[0].inner_rect;
        let client = server.clients.get_mut(&1).unwrap();
        let mut frame = client.render_state.last_frame().unwrap().clone();
        frame.hyperlinks = vec!["https://example.com".to_owned()];
        let hyperlink_idx =
            usize::from(inner_rect.y) * usize::from(frame.width) + usize::from(inner_rect.x);
        frame.cells[hyperlink_idx].hyperlink = Some(0);
        let prepared = client
            .render_state
            .prepare_frame(frame)
            .expect("hyperlink frame differs");
        client.render_state.commit_sent_frame(prepared);

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rplain");

        assert!(!server.render_retained_pty_update_and_stream());
        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());

        server.render_and_stream();
        let full = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("full frame after hyperlink overwrite"),
        );
        assert!(
            full.cells.iter().all(|cell| cell.hyperlink.is_none()),
            "full render should clear overwritten hyperlink cells"
        );
    }

    #[tokio::test]
    async fn retained_pty_update_allows_dirty_row_that_creates_plain_url() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"plain");
        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial frame");

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rhttps://example.com/new");

        assert!(server.render_retained_pty_update_and_stream());
        let patched = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("retained frame after plain URL"),
        );
        assert!(
            patched.hyperlinks.is_empty(),
            "retained render should not synthesize plain URL hyperlink metadata"
        );
    }

    #[tokio::test]
    async fn retained_pty_update_allows_kitty_enabled_empty_graphics_cache() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        server.app.state.kitty_graphics_enabled = true;
        server.clients.get_mut(&1).unwrap().cell_size = crate::kitty_graphics::HostCellSize {
            width_px: 10,
            height_px: 20,
        };

        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial frame");

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rZ");

        assert!(server.render_retained_pty_update_and_stream());
        let retained = read_server_frame(
            client_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("retained frame with kitty enabled"),
        );
        assert!(retained.cells.iter().any(|cell| cell.symbol == "Z"));
    }

    #[tokio::test]
    async fn retained_pty_update_declines_when_graphics_cache_has_content() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        server.app.state.kitty_graphics_enabled = true;
        let client = server.clients.get_mut(&1).unwrap();
        client.cell_size = crate::kitty_graphics::HostCellSize {
            width_px: 10,
            height_px: 20,
        };

        server.render_and_stream();
        let _ = client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("initial frame");
        server
            .clients
            .get_mut(&1)
            .unwrap()
            .graphics_cache
            .test_mark_non_empty();

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rZ");

        assert!(!server.render_retained_pty_update_and_stream());
        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());
    }

    #[tokio::test]
    async fn full_redraw_pending_survives_full_render_queue_full() {
        let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
        let queued = HeadlessServer::frame_server_message(&ServerMessage::ReloadSoundConfig)
            .expect("serialize dummy message");
        server
            .clients
            .get(&1)
            .unwrap()
            .writer
            .as_ref()
            .unwrap()
            .render
            .try_send(queued)
            .expect("pre-fill render queue");
        server.app.full_redraw_pending = true;

        server.render_and_stream();

        assert!(server.app.full_redraw_pending);
        assert_eq!(
            server.clients.get(&1).unwrap().deferred_render(),
            DeferredRender::Full
        );
        assert!(matches!(
            read_server_message(client_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
            ServerMessage::ReloadSoundConfig
        ));

        let runtime = server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
            .expect("runtime");
        runtime.test_process_pty_bytes(b"\rZ");

        assert!(!server.render_retained_pty_update_and_stream());
        assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());
    }

    #[test]
    fn client_config_reload_request_refreshes_attached_clients() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.app.state.request_client_config_reload = true;

        server.drain_client_config_reload_request();

        match read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("client config reload message"),
        ) {
            ServerMessage::ReloadSoundConfig => {}
            other => panic!("expected ReloadSoundConfig, got {other:?}"),
        }
        assert!(!server.app.state.request_client_config_reload);
    }

    #[test]
    fn clipboard_write_targets_foreground_client_only() {
        let mut server = test_headless_server();
        let (background_tx, background_control_rx, _background_rx) = test_client_writer();
        let (foreground_tx, foreground_control_rx, _foreground_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(background_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(foreground_tx),
            ),
        );
        server.foreground_client_id = Some(2);
        server.sync_foreground_client_state();

        let changed = server.handle_internal_event_with_forwarding(AppEvent::ClipboardWrite {
            content: b"test".to_vec(),
        });

        assert!(changed);
        assert_eq!(
            server
                .app
                .state
                .copy_feedback
                .as_ref()
                .map(|feedback| feedback.message.as_str()),
            Some("copied to clipboard")
        );
        match read_server_message(
            foreground_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("foreground clipboard message"),
        ) {
            ServerMessage::Clipboard { data } => assert_eq!(data, "dGVzdA=="),
            other => panic!("expected clipboard message, got {other:?}"),
        }
        assert!(
            background_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "background client should not receive clipboard writes"
        );
    }

    #[test]
    fn clipboard_write_without_foreground_client_does_not_show_feedback() {
        let mut server = test_headless_server();
        server.foreground_client_id = None;

        let changed = server.handle_internal_event_with_forwarding(AppEvent::ClipboardWrite {
            content: b"test".to_vec(),
        });

        assert!(changed);
        assert!(
            server.app.state.copy_feedback.is_none(),
            "clipboard feedback should only show when a foreground client can receive the write"
        );
    }

    #[test]
    fn clipboard_write_failed_foreground_send_does_not_show_feedback() {
        let mut server = test_headless_server();
        let (foreground_tx, foreground_control_rx, _foreground_rx) = test_client_writer();
        drop(foreground_control_rx);

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(foreground_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        let changed = server.handle_internal_event_with_forwarding(AppEvent::ClipboardWrite {
            content: b"test".to_vec(),
        });

        assert!(changed);
        assert!(
            server.app.state.copy_feedback.is_none(),
            "clipboard feedback should only show after the foreground client receives the write"
        );
        assert!(
            !server.clients.contains_key(&1),
            "failed targeted send should remove the broken foreground client"
        );
    }

    #[test]
    fn prefix_input_source_targets_foreground_client_only() {
        let mut server = test_headless_server();
        let (background_tx, background_control_rx, _background_rx) = test_client_writer();
        let (foreground_tx, foreground_control_rx, _foreground_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(background_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(foreground_tx),
            ),
        );
        server.foreground_client_id = Some(2);
        server.sync_foreground_client_state();
        // Drain any setup messages (e.g. mouse-capture sync) before exercising the event.
        while foreground_control_rx
            .recv_timeout(Duration::from_millis(20))
            .is_ok()
        {}

        let changed = server
            .handle_internal_event_with_forwarding(AppEvent::PrefixInputSource { active: true });

        assert!(changed);
        match read_server_message(
            foreground_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("foreground prefix input-source message"),
        ) {
            ServerMessage::PrefixInputSource { active } => assert!(active),
            other => panic!("expected prefix input-source message, got {other:?}"),
        }
        assert!(
            background_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "background client should not receive prefix input-source changes"
        );
    }

    #[test]
    fn headless_app_keeps_prefix_input_source_switch_off_process() {
        // An App-internal drain (e.g. the exhaustive drain at the top of
        // handle_api_request) can consume a queued PrefixInputSource intent
        // before the forwarding drain sees it. The headless App must treat the
        // event as inert instead of switching the host input source from the
        // server process.
        struct CountingPrefixInputSource(std::rc::Rc<std::cell::Cell<usize>>);
        impl crate::platform::PrefixInputSource for CountingPrefixInputSource {
            fn switch_to_ascii(&mut self) {
                self.0.set(self.0.get() + 1);
            }
            fn restore(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let mut server = test_headless_server();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        server
            .app
            .set_prefix_input_source(Box::new(CountingPrefixInputSource(calls.clone())));

        server
            .app
            .handle_internal_event(AppEvent::PrefixInputSource { active: true });
        server
            .app
            .handle_internal_event(AppEvent::PrefixInputSource { active: false });
        assert_eq!(
            calls.get(),
            0,
            "headless server must not apply the host input-source switch"
        );

        // Sanity: the same event does apply once the flag is on (monolithic semantics).
        server.app.local_input_source_switch = true;
        server
            .app
            .handle_internal_event(AppEvent::PrefixInputSource { active: true });
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn client_local_notifications_target_foreground_client_only() {
        let mut server = test_headless_server();
        let (background_tx, background_control_rx, _background_rx) = test_client_writer();
        let (foreground_tx, foreground_control_rx, _foreground_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(background_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(foreground_tx),
            ),
        );
        server.foreground_client_id = Some(2);
        server.sync_foreground_client_state();

        assert!(server.send_to_foreground_client(ServerMessage::Notify {
            kind: protocol::NotifyKind::Toast,
            message: "pi finished".to_string(),
            body: Some("workspace 1".to_string()),
        }));

        match read_server_message(
            foreground_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("foreground toast message"),
        ) {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::Toast);
                assert_eq!(message, "pi finished");
                assert_eq!(body.as_deref(), Some("workspace 1"));
            }
            other => panic!("expected toast notify, got {other:?}"),
        }
        assert!(
            background_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "background client should not receive client-local notifications"
        );
    }

    #[test]
    fn oversized_paste_rejection_notifies_only_the_sending_client() {
        let mut server = test_headless_server();
        let (sender_writer, sender_control_rx, _sender_render_rx) = test_client_writer();
        let (foreground_writer, foreground_control_rx, _foreground_render_rx) =
            test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (120, 40),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(sender_writer),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(foreground_writer),
            ),
        );
        server.foreground_client_id = Some(2);
        server.sync_foreground_client_state();

        assert!(
            !server.handle_server_event(ServerEvent::ClientPasteRejected {
                client_id: 1,
                size: 5_000_012,
                max: 1_048_576,
            })
        );

        match read_server_message(
            sender_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("sending client rejection notification"),
        ) {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::Toast);
                assert_eq!(message, "Paste rejected");
                assert_eq!(
                    body.as_deref(),
                    Some("Input message is 5000012 bytes; Herdr's limit is 1048576 bytes")
                );
            }
            other => panic!("expected paste rejection notification, got {other:?}"),
        }
        assert!(
            foreground_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "foreground client must not receive another client's rejection"
        );
        assert_eq!(server.foreground_client_id, Some(2));
        assert_eq!(server.clients.len(), 2);
        assert!(server.app.state.toast.is_none());
    }

    #[test]
    fn herdr_toast_delivery_keeps_toast_in_frame_without_client_notify() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::Herdr;

        let changed = server.handle_internal_event_with_forwarding(AppEvent::UpdateReady {
            version: "9.9.9".to_string(),
            install_command: "herdr update".into(),
        });

        assert!(changed);
        assert!(server.app.state.toast.is_some());
        assert!(
            client_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "herdr delivery should render in-frame instead of forwarding a client-local notification"
        );
    }

    #[test]
    fn system_toast_delivery_forwards_system_notify_kind() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::System;

        let changed = server.handle_internal_event_with_forwarding(AppEvent::UpdateReady {
            version: "9.9.9".to_string(),
            install_command: "herdr update".into(),
        });

        assert!(changed);
        match read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("system toast message"),
        ) {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::SystemToast);
                assert_eq!(message, "v9.9.9 available");
                assert_eq!(
                    body.as_deref(),
                    Some("detach, run `herdr update`, then follow its restart guidance")
                );
            }
            other => panic!("expected system toast notify, got {other:?}"),
        }
    }

    #[test]
    fn notification_show_api_forwards_system_notification_to_foreground_client() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::System;

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        let changed = server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
            request: api::schema::Request {
                id: "notify".into(),
                method: api::schema::Method::NotificationShow(
                    api::schema::NotificationShowParams {
                        title: "build failed".into(),
                        body: Some("api workspace".into()),
                        position: Some(crate::config::ToastHerdrPosition::TopLeft),
                        sound: api::schema::NotificationShowSound::Request,
                    },
                ),
            },
            respond_to,
            response_write_complete: None,
        });

        assert!(changed);
        let response = response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
        let parsed: api::schema::SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(
            parsed.result,
            api::schema::ResponseResult::NotificationShow {
                shown: true,
                reason: api::schema::NotificationShowReason::Shown,
            }
        );
        let first = read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("api notification message"),
        );
        let second = read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("api sound message"),
        );

        match first {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::SystemToast);
                assert_eq!(message, "build failed");
                assert_eq!(body.as_deref(), Some("api workspace"));
            }
            other => panic!("expected api notification, got {other:?}"),
        }
        match second {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::Sound);
                assert_eq!(message, "agent attention");
                assert!(body.is_none());
            }
            other => panic!("expected api sound, got {other:?}"),
        }
    }

    #[test]
    fn notification_show_api_preserves_colon_in_forwarded_title() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::System;

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        let changed = server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
            request: api::schema::Request {
                id: "notify".into(),
                method: api::schema::Method::NotificationShow(
                    api::schema::NotificationShowParams {
                        title: "build: failed".into(),
                        body: Some("api workspace".into()),
                        position: None,
                        sound: api::schema::NotificationShowSound::None,
                    },
                ),
            },
            respond_to,
            response_write_complete: None,
        });

        assert!(changed);
        let response = response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
        let parsed: api::schema::SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(
            parsed.result,
            api::schema::ResponseResult::NotificationShow {
                shown: true,
                reason: api::schema::NotificationShowReason::Shown,
            }
        );
        match read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("api notification message"),
        ) {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::SystemToast);
                assert_eq!(message, "build: failed");
                assert_eq!(body.as_deref(), Some("api workspace"));
            }
            other => panic!("expected api notification, got {other:?}"),
        }
    }

    #[test]
    fn notification_show_api_validates_empty_title_before_disabled_delivery() {
        let mut server = test_headless_server();
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::Off;

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        let changed = server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
            request: api::schema::Request {
                id: "notify".into(),
                method: api::schema::Method::NotificationShow(
                    api::schema::NotificationShowParams {
                        title: "\n\t".into(),
                        body: None,
                        position: None,
                        sound: api::schema::NotificationShowSound::None,
                    },
                ),
            },
            respond_to,
            response_write_complete: None,
        });

        assert!(changed);
        let response = response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
        let parsed: api::schema::ErrorResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(parsed.error.code, "invalid_params");
        assert_eq!(parsed.error.message, "notification title is empty");
    }

    #[test]
    fn notification_show_api_reports_no_foreground_client() {
        let mut server = test_headless_server();
        server.foreground_client_id = None;
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::System;

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        let changed = server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
            request: api::schema::Request {
                id: "notify".into(),
                method: api::schema::Method::NotificationShow(
                    api::schema::NotificationShowParams {
                        title: "build failed".into(),
                        body: None,
                        position: None,
                        sound: api::schema::NotificationShowSound::Request,
                    },
                ),
            },
            respond_to,
            response_write_complete: None,
        });

        assert!(changed);
        let response = response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
        let parsed: api::schema::SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(
            parsed.result,
            api::schema::ResponseResult::NotificationShow {
                shown: false,
                reason: api::schema::NotificationShowReason::NoForegroundClient,
            }
        );
    }

    #[test]
    fn notification_show_api_herdr_toast_expires_headless() {
        let mut server = test_headless_server();
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::Herdr;

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        assert!(
            server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
                request: api::schema::Request {
                    id: "notify".into(),
                    method: api::schema::Method::NotificationShow(
                        api::schema::NotificationShowParams {
                            title: "build failed".into(),
                            body: None,
                            position: None,
                            sound: api::schema::NotificationShowSound::None,
                        },
                    ),
                },
                respond_to,
                response_write_complete: None,
            })
        );

        let response = response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
        let parsed: api::schema::SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(
            parsed.result,
            api::schema::ResponseResult::NotificationShow {
                shown: true,
                reason: api::schema::NotificationShowReason::Shown,
            }
        );
        let deadline = server.app.toast_deadline.expect("api toast deadline");
        assert!(server.handle_scheduled_tasks_headless(deadline, false));
        assert!(server.app.state.toast.is_none());
        assert!(server.app.toast_deadline.is_none());
    }

    #[test]
    fn notification_show_api_forwards_sound_for_herdr_delivery() {
        let mut server = test_headless_server();
        let (client_tx, client_control_rx, _client_rx) = test_client_writer();

        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::Herdr;

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        assert!(
            server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
                request: api::schema::Request {
                    id: "notify".into(),
                    method: api::schema::Method::NotificationShow(
                        api::schema::NotificationShowParams {
                            title: "build failed".into(),
                            body: None,
                            position: None,
                            sound: api::schema::NotificationShowSound::Done,
                        },
                    ),
                },
                respond_to,
                response_write_complete: None,
            })
        );

        let response = response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap();
        let parsed: api::schema::SuccessResponse = serde_json::from_str(&response).unwrap();
        assert_eq!(
            parsed.result,
            api::schema::ResponseResult::NotificationShow {
                shown: true,
                reason: api::schema::NotificationShowReason::Shown,
            }
        );
        match read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("api sound message"),
        ) {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::Sound);
                assert_eq!(message, "agent done");
                assert!(body.is_none());
            }
            other => panic!("expected api sound, got {other:?}"),
        }
    }

    #[test]
    fn delayed_agent_notification_forwards_after_deadline() {
        let mut server = test_headless_server();
        let background = crate::workspace::Workspace::test_new("background");
        let pane_id = background.tabs[0].root_pane;
        let foreground = crate::workspace::Workspace::test_new("foreground");
        server.app.state.workspaces = vec![background, foreground];
        server.app.state.ensure_test_terminals();
        server.app.state.active = Some(1);
        server.app.state.selected = 1;
        server.app.state.mode = crate::app::Mode::Terminal;
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::System;
        server.app.state.toast_config.delay_seconds = 1;

        let (client_tx, client_control_rx, _client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        let changed = server.handle_internal_event_with_forwarding(AppEvent::StateChanged {
            pane_id,
            agent: Some(crate::detect::Agent::Pi),
            state: crate::detect::AgentState::Blocked,
            visible_blocker: false,
            visible_working: false,
            process_exited: false,
            observed_at: Instant::now(),
        });

        assert!(changed);
        assert!(server.app.state.toast.is_none());
        assert!(
            client_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "delayed transition should not notify immediately"
        );

        let deadline = server
            .app
            .state
            .next_pending_agent_notification_deadline()
            .expect("pending notification deadline");
        assert!(server.handle_scheduled_tasks_headless(deadline, false));

        let first = read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("delayed sound message"),
        );
        let second = read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("delayed toast message"),
        );

        assert!(matches!(
            first,
            ServerMessage::Notify {
                kind: protocol::NotifyKind::Sound,
                ..
            }
        ));
        match second {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::SystemToast);
                assert_eq!(message, "pi needs attention");
                assert_eq!(body.as_deref(), Some("background · 1"));
            }
            other => panic!("expected delayed system toast, got {other:?}"),
        }
        assert!(server.app.state.pending_agent_notifications.is_empty());
    }

    #[test]
    fn delayed_active_tab_unfocused_agent_notification_forwards_after_deadline() {
        let mut server = test_headless_server();
        let workspace = crate::workspace::Workspace::test_new("active");
        let pane_id = workspace.tabs[0].root_pane;
        server.app.state.workspaces = vec![workspace];
        server.app.state.ensure_test_terminals();
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;
        server.app.state.toast_config.delivery = crate::config::ToastDelivery::System;
        server.app.state.toast_config.delay_seconds = 1;

        let (client_tx, client_control_rx, _client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                Some(false),
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        assert!(
            server.handle_internal_event_with_forwarding(AppEvent::StateChanged {
                pane_id,
                agent: Some(crate::detect::Agent::Pi),
                state: crate::detect::AgentState::Blocked,
                visible_blocker: false,
                visible_working: false,
                process_exited: false,
                observed_at: Instant::now(),
            })
        );
        assert!(server.app.state.toast.is_none());
        assert!(
            client_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "delayed transition should not notify immediately"
        );

        let deadline = server
            .app
            .state
            .next_pending_agent_notification_deadline()
            .expect("pending notification deadline");
        assert!(server.handle_scheduled_tasks_headless(deadline, false));

        let first = read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("delayed sound message"),
        );
        let second = read_server_message(
            client_control_rx
                .recv_timeout(Duration::from_millis(100))
                .expect("delayed toast message"),
        );

        assert!(matches!(
            first,
            ServerMessage::Notify {
                kind: protocol::NotifyKind::Sound,
                ..
            }
        ));
        match second {
            ServerMessage::Notify {
                kind,
                message,
                body,
            } => {
                assert_eq!(kind, protocol::NotifyKind::SystemToast);
                assert_eq!(message, "pi needs attention");
                assert_eq!(body.as_deref(), Some("active · 1"));
            }
            other => panic!("expected delayed system toast, got {other:?}"),
        }
    }

    #[test]
    fn stale_api_agent_report_does_not_forward_done_sound() {
        let mut server = test_headless_server();
        let background = crate::workspace::Workspace::test_new("background");
        let pane_id = background.tabs[0].root_pane;
        let public_pane_id = format!("{}:p1", background.id);
        let foreground = crate::workspace::Workspace::test_new("foreground");
        server.app.state.workspaces = vec![background, foreground];
        server.app.state.ensure_test_terminals();
        let terminal_id = server.app.state.workspaces[0]
            .pane_state(pane_id)
            .unwrap()
            .attached_terminal_id
            .clone();
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_detected_state(
                Some(crate::detect::Agent::Pi),
                crate::detect::AgentState::Idle,
            );
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_persisted_agent_session(crate::agent_resume::PersistedAgentSession {
                source: "herdr:pi".into(),
                agent: "pi".into(),
                session_ref: crate::agent_resume::AgentSessionRef::path(
                    std::env::current_dir()
                        .unwrap()
                        .join("headless-pi-session.jsonl")
                        .display()
                        .to_string(),
                )
                .unwrap(),
            });
        server
            .app
            .state
            .terminals
            .get_mut(&terminal_id)
            .unwrap()
            .set_hook_authority(
                "herdr:pi".into(),
                "pi".into(),
                crate::detect::AgentState::Working,
                None,
                Some(20),
            );
        server.app.state.active = Some(1);
        server.app.state.selected = 1;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (client_tx, client_control_rx, _client_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (80, 24),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(client_tx),
            ),
        );
        server.foreground_client_id = Some(1);
        server.sync_foreground_client_state();

        let (respond_to, response_rx) = std::sync::mpsc::channel();
        let changed = server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
            request: api::schema::Request {
                id: "stale".into(),
                method: api::schema::Method::PaneReportAgent(api::schema::PaneReportAgentParams {
                    pane_id: public_pane_id,
                    source: "herdr:pi".into(),
                    agent: "pi".into(),
                    state: api::schema::PaneAgentState::Idle,
                    message: None,
                    seq: Some(19),
                    agent_session_id: None,
                    agent_session_path: None,
                }),
            },
            respond_to,
            response_write_complete: None,
        });

        assert!(changed);
        assert!(response_rx.recv_timeout(Duration::from_millis(100)).is_ok());
        assert_eq!(
            server.app.state.terminals.get(&terminal_id).unwrap().state,
            crate::detect::AgentState::Working
        );
        assert!(
            client_control_rx
                .recv_timeout(Duration::from_millis(50))
                .is_err(),
            "stale idle report must not forward a done sound"
        );
    }

    /// Verify that no direct calls to `self.app.handle_internal_event`
    /// (or its `handle_internal_event_with_prefix_sync` wrapper) exist
    /// outside of `handle_internal_event_with_forwarding` in this
    /// module. This ensures the forwarding bypass cannot be reintroduced.
    ///
    /// The search pattern looks for `handle_internal_event` calls that
    /// are NOT inside the `handle_internal_event_with_forwarding` method.
    #[test]
    fn no_handle_internal_event_bypass_in_module() {
        let source = include_str!("headless.rs");

        // Find all lines containing handle_internal_event
        let mut bypass_lines: Vec<String> = Vec::new();
        let mut inside_forwarding_method = false;
        let mut forwarding_method_brace_depth = 0u32;

        for (i, line) in source.lines().enumerate() {
            let line_num = i + 1;

            // Track when we're inside handle_internal_event_with_forwarding
            if line.contains("fn handle_internal_event_with_forwarding") {
                inside_forwarding_method = true;
                forwarding_method_brace_depth = 0;
            }

            if inside_forwarding_method {
                // Count braces to track when we exit the method
                for ch in line.chars() {
                    match ch {
                        '{' => forwarding_method_brace_depth += 1,
                        '}' => {
                            forwarding_method_brace_depth =
                                forwarding_method_brace_depth.saturating_sub(1);
                            if forwarding_method_brace_depth == 0 {
                                inside_forwarding_method = false;
                            }
                        }
                        _ => {}
                    }
                }
            } else if (line.contains("self.app.handle_internal_event(")
                || line.contains("self.app.handle_internal_event_with_prefix_sync("))
                && !line.trim().starts_with("///")
                && !line.contains("contains(")
            {
                // Direct call to handle_internal_event outside the forwarding method
                bypass_lines.push(format!("line {}: {}", line_num, line.trim()));
            }
        }

        assert!(
            bypass_lines.is_empty(),
            "Found direct calls to self.app.handle_internal_event outside \
             handle_internal_event_with_forwarding (bypass risk):\n  {}",
            bypass_lines.join("\n  ")
        );
    }

    /// Two displays of very different sizes, each on its own tab. The narrow
    /// one sends a pointer event; the geometry it is resolved against must be
    /// the narrow display's, not whatever the last render left behind.
    ///
    /// TP-MCF-VIEW-01
    #[tokio::test]
    async fn a_pointer_event_is_resolved_against_its_own_display() {
        let mut server = test_headless_server();
        let mut workspace = crate::workspace::Workspace::test_new("test");
        let second_tab = workspace.test_add_tab(Some("second"));
        server.app.state.workspaces = vec![workspace];
        server.app.state.active = Some(0);
        server.app.state.selected = 0;
        server.app.state.mode = crate::app::Mode::Terminal;

        let (wide_tx, _wide_control_rx, _wide_rx) = test_client_writer();
        let (narrow_tx, _narrow_control_rx, _narrow_rx) = test_client_writer();
        server.clients.insert(
            1,
            ClientConnection::new(
                (200, 50),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                1,
                RenderEncoding::SemanticFrame,
                Some(wide_tx),
            ),
        );
        server.clients.insert(
            2,
            ClientConnection::new(
                (60, 20),
                crate::kitty_graphics::HostCellSize::default(),
                crate::terminal_theme::TerminalTheme::default(),
                None,
                2,
                RenderEncoding::SemanticFrame,
                Some(narrow_tx),
            ),
        );
        server.foreground_client_id = Some(1);

        server.render_and_stream();
        let previous = server.app.state.enter_viewer(Some(2));
        server.app.state.workspaces[0].set_active_tab(second_tab);
        server.app.state.restore_viewer(previous);
        server.render_and_stream();

        // After a frame the shared view belongs to whichever client drew last,
        // which is the foreground one. Prove that first, so the assertion
        // below is measuring the fix and not an accident of ordering.
        let after_render_width = server.app.state.view.terminal_area.width;
        assert!(
            after_render_width > 60,
            "the frame leaves the wide display's geometry behind, got {after_render_width}"
        );

        server.handle_client_input_events(
            2,
            vec![crate::raw_input::RawInputEvent::Mouse(
                crossterm::event::MouseEvent {
                    kind: crossterm::event::MouseEventKind::Moved,
                    column: 1,
                    row: 1,
                    modifiers: crossterm::event::KeyModifiers::empty(),
                },
            )],
        );

        let resolved_width = server.app.state.view.terminal_area.width;
        assert!(
            resolved_width <= 60,
            "the narrow display's pointer must be resolved against its own layout, got {resolved_width}"
        );
    }
}
