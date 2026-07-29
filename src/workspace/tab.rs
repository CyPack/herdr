use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::{Duration, Instant};

use ratatui::layout::Direction;
use tokio::sync::{mpsc, Notify};

use crate::events::AppEvent;
use crate::layout::{Node, PaneId, TileLayout};
use crate::pane::{PaneLaunchEnv, PaneState};
use crate::terminal::{TerminalId, TerminalRuntime, TerminalRuntimeRegistry, TerminalState};

pub(crate) type DetachedPane = (PaneId, TerminalId);

/// The strip flash of a newly spawned tab: total window, then the blink's
/// half-period within it.
pub(crate) const TAB_FLASH_WINDOW: Duration = Duration::from_millis(2000);
const TAB_FLASH_HALF_PERIOD_MS: u128 = 250;

/// Elapsed-based (not tick-based) on purpose: the monolithic renderer ticks
/// every 16 ms and the headless one every 128 ms, and the blink must look the
/// same under both. TP-TAB-FLASH-02
pub(crate) fn flash_phase_at(spawned_at: Option<Instant>, now: Instant) -> Option<bool> {
    let elapsed = now.checked_duration_since(spawned_at?)?;
    if elapsed >= TAB_FLASH_WINDOW {
        return None;
    }
    Some((elapsed.as_millis() / TAB_FLASH_HALF_PERIOD_MS).is_multiple_of(2))
}

pub(crate) struct MovedPane {
    pub pane_id: PaneId,
    pub pane_state: PaneState,
}

pub struct NewPane {
    pub pane_id: PaneId,
    pub terminal: TerminalState,
    pub runtime: TerminalRuntime,
}

enum SplitCommand<'a> {
    Shell {
        command: &'a str,
        launch_env: &'a PaneLaunchEnv,
    },
    Argv {
        argv: &'a [String],
        launch_env: &'a PaneLaunchEnv,
    },
}

pub struct Tab {
    pub custom_name: Option<String>,
    pub number: usize,
    /// Claude Code session id this tab was opened to resume (Projects tab).
    /// Wires the chat to exactly one tab so repeated clicks focus it instead
    /// of spawning duplicates; cleared with the tab when it closes.
    pub resumed_session_id: Option<String>,
    /// The tab was opened in the background and has never been activated.
    ///
    /// Opt-in: constructors start it `false` and only the background-opening
    /// API paths (plugin tab, `tab create focus:false`) set it, so restored
    /// and moved-pane tabs never light up. The first visit clears it for the
    /// whole session — the flag belongs to the tab, not to a display, exactly
    /// like tmux's window activity flag. TP-TAB-UNSEEN-05
    pub unseen: bool,
    /// When this tab was spawned, driving the strip's short flash.
    ///
    /// Opt-in like `unseen`: the spawn constructors set it, while restored
    /// and moved-pane tabs carry `None` — a restart must not strobe the whole
    /// strip. Not persisted. TP-TAB-FLASH-01
    pub spawned_at: Option<Instant>,
    /// Identity source for this tab's pane tree.
    pub root_pane: PaneId,
    pub layout: TileLayout,
    /// Pane viewport state — always present, testable without PTYs.
    pub panes: HashMap<PaneId, PaneState>,
    #[cfg(test)]
    pub runtimes: HashMap<PaneId, TerminalRuntime>,
    pub zoomed: bool,
    pub events: mpsc::Sender<AppEvent>,
    pub(crate) render_notify: Arc<Notify>,
    pub(crate) render_dirty: Arc<AtomicBool>,
}

impl Tab {
    pub fn new(
        number: usize,
        initial_cwd: PathBuf,
        rows: u16,
        cols: u16,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        shell_config: crate::pane::PaneShellConfig<'_>,
        launch_env: &PaneLaunchEnv,
        events: mpsc::Sender<AppEvent>,
        render_notify: Arc<Notify>,
        render_dirty: Arc<AtomicBool>,
    ) -> std::io::Result<(Self, TerminalState, TerminalRuntime)> {
        Self::new_with_runtime(
            number,
            initial_cwd,
            rows,
            cols,
            scrollback_limit_bytes,
            host_terminal_theme,
            shell_config,
            launch_env,
            events,
            render_notify,
            render_dirty,
            None,
        )
    }

    pub fn new_argv_command(
        number: usize,
        initial_cwd: PathBuf,
        rows: u16,
        cols: u16,
        argv: &[String],
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        launch_env: &PaneLaunchEnv,
        events: mpsc::Sender<AppEvent>,
        render_notify: Arc<Notify>,
        render_dirty: Arc<AtomicBool>,
    ) -> std::io::Result<(Self, TerminalState, TerminalRuntime)> {
        Self::new_with_runtime(
            number,
            initial_cwd,
            rows,
            cols,
            scrollback_limit_bytes,
            host_terminal_theme,
            crate::pane::PaneShellConfig::new("", crate::config::ShellModeConfig::NonLogin),
            launch_env,
            events,
            render_notify,
            render_dirty,
            Some(argv),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn new_with_runtime(
        number: usize,
        initial_cwd: PathBuf,
        rows: u16,
        cols: u16,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        shell_config: crate::pane::PaneShellConfig<'_>,
        launch_env: &PaneLaunchEnv,
        events: mpsc::Sender<AppEvent>,
        render_notify: Arc<Notify>,
        render_dirty: Arc<AtomicBool>,
        argv: Option<&[String]>,
    ) -> std::io::Result<(Self, TerminalState, TerminalRuntime)> {
        let (layout, root_id) = TileLayout::new();
        let runtime = if let Some(argv) = argv {
            TerminalRuntime::spawn_argv_command(
                root_id,
                rows,
                cols,
                initial_cwd.clone(),
                argv,
                launch_env,
                crate::pane::AgentDetection::Enabled,
                scrollback_limit_bytes,
                host_terminal_theme,
                events.clone(),
                render_notify.clone(),
                render_dirty.clone(),
            )?
        } else {
            TerminalRuntime::spawn(
                root_id,
                rows,
                cols,
                initial_cwd.clone(),
                scrollback_limit_bytes,
                host_terminal_theme,
                shell_config,
                launch_env,
                events.clone(),
                render_notify.clone(),
                render_dirty.clone(),
            )?
        };

        let terminal_id = TerminalId::alloc();
        let terminal = match argv {
            Some(argv) => {
                TerminalState::new(terminal_id.clone(), initial_cwd).with_launch_argv(argv.to_vec())
            }
            None => TerminalState::new(terminal_id.clone(), initial_cwd),
        };
        let mut panes = HashMap::new();
        panes.insert(root_id, PaneState::new(terminal_id));

        Ok((
            Self {
                custom_name: None,
                number,
                resumed_session_id: None,
                unseen: false,
                spawned_at: Some(Instant::now()),
                root_pane: root_id,
                layout,
                panes,
                #[cfg(test)]
                runtimes: HashMap::new(),
                zoomed: false,
                events,
                render_notify,
                render_dirty,
            },
            terminal,
            runtime,
        ))
    }

    pub fn is_auto_named(&self) -> bool {
        self.custom_name.is_none()
    }

    /// Blink phase of the spawn flash: `None` outside the window,
    /// `Some(bright)` inside it. TP-TAB-FLASH-02
    pub(crate) fn flash_phase(&self, now: Instant) -> Option<bool> {
        flash_phase_at(self.spawned_at, now)
    }

    /// True through the whole flash window regardless of blink phase — this
    /// is what keeps the animation timer alive so the flash actually gets
    /// drawn under the headless renderer. TP-TAB-FLASH-03
    pub(crate) fn flash_window_active(&self, now: Instant) -> bool {
        self.spawned_at.is_some_and(|spawned| {
            now.checked_duration_since(spawned)
                .is_some_and(|elapsed| elapsed < TAB_FLASH_WINDOW)
        })
    }

    pub fn set_custom_name(&mut self, name: String) {
        self.custom_name = Some(name);
    }

    pub fn split_focused(
        &mut self,
        direction: Direction,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        shell_config: crate::pane::PaneShellConfig<'_>,
        launch_env: &PaneLaunchEnv,
    ) -> std::io::Result<NewPane> {
        self.split_focused_with_runtime(
            direction,
            None,
            rows,
            cols,
            cwd,
            scrollback_limit_bytes,
            host_terminal_theme,
            shell_config,
            launch_env,
            None,
        )
    }

    pub fn split_focused_with_ratio(
        &mut self,
        direction: Direction,
        ratio: f32,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        shell_config: crate::pane::PaneShellConfig<'_>,
        launch_env: &PaneLaunchEnv,
    ) -> std::io::Result<NewPane> {
        self.split_focused_with_runtime(
            direction,
            Some(ratio),
            rows,
            cols,
            cwd,
            scrollback_limit_bytes,
            host_terminal_theme,
            shell_config,
            launch_env,
            None,
        )
    }

    pub fn split_focused_command(
        &mut self,
        direction: Direction,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        command: &str,
        launch_env: &PaneLaunchEnv,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
    ) -> std::io::Result<NewPane> {
        self.split_focused_with_runtime(
            direction,
            None,
            rows,
            cols,
            cwd,
            scrollback_limit_bytes,
            host_terminal_theme,
            crate::pane::PaneShellConfig::new("", crate::config::ShellModeConfig::NonLogin),
            launch_env,
            Some(SplitCommand::Shell {
                command,
                launch_env,
            }),
        )
    }

    pub fn split_focused_argv_command(
        &mut self,
        direction: Direction,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        argv: &[String],
        launch_env: &PaneLaunchEnv,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
    ) -> std::io::Result<NewPane> {
        self.split_focused_with_runtime(
            direction,
            None,
            rows,
            cols,
            cwd,
            scrollback_limit_bytes,
            host_terminal_theme,
            crate::pane::PaneShellConfig::new("", crate::config::ShellModeConfig::NonLogin),
            launch_env,
            Some(SplitCommand::Argv { argv, launch_env }),
        )
    }

    pub fn split_focused_argv_command_with_ratio(
        &mut self,
        direction: Direction,
        ratio: f32,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        argv: &[String],
        launch_env: &PaneLaunchEnv,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
    ) -> std::io::Result<NewPane> {
        self.split_focused_with_runtime(
            direction,
            Some(ratio),
            rows,
            cols,
            cwd,
            scrollback_limit_bytes,
            host_terminal_theme,
            crate::pane::PaneShellConfig::new("", crate::config::ShellModeConfig::NonLogin),
            launch_env,
            Some(SplitCommand::Argv { argv, launch_env }),
        )
    }

    fn split_focused_with_runtime(
        &mut self,
        direction: Direction,
        ratio: Option<f32>,
        rows: u16,
        cols: u16,
        cwd: Option<PathBuf>,
        scrollback_limit_bytes: usize,
        host_terminal_theme: crate::terminal_theme::TerminalTheme,
        shell_config: crate::pane::PaneShellConfig<'_>,
        launch_env: &PaneLaunchEnv,
        command: Option<SplitCommand<'_>>,
    ) -> std::io::Result<NewPane> {
        let previous_focus = self.layout.focused();
        let new_id = match ratio {
            Some(ratio) => self.layout.split_focused_with_ratio(direction, ratio),
            None => self.layout.split_focused(direction),
        };
        let actual_cwd =
            cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| "/".into()));
        let launch_argv = if let Some(SplitCommand::Argv { argv, .. }) = &command {
            Some((*argv).to_vec())
        } else {
            None
        };
        let runtime = match command {
            Some(SplitCommand::Shell {
                command,
                launch_env,
            }) => TerminalRuntime::spawn_shell_command(
                new_id,
                rows,
                cols,
                actual_cwd.clone(),
                command,
                launch_env,
                crate::pane::AgentDetection::Enabled,
                scrollback_limit_bytes,
                host_terminal_theme,
                self.events.clone(),
                self.render_notify.clone(),
                self.render_dirty.clone(),
            ),
            Some(SplitCommand::Argv { argv, launch_env }) => TerminalRuntime::spawn_argv_command(
                new_id,
                rows,
                cols,
                actual_cwd.clone(),
                argv,
                launch_env,
                crate::pane::AgentDetection::Enabled,
                scrollback_limit_bytes,
                host_terminal_theme,
                self.events.clone(),
                self.render_notify.clone(),
                self.render_dirty.clone(),
            ),
            None => TerminalRuntime::spawn(
                new_id,
                rows,
                cols,
                actual_cwd.clone(),
                scrollback_limit_bytes,
                host_terminal_theme,
                shell_config,
                launch_env,
                self.events.clone(),
                self.render_notify.clone(),
                self.render_dirty.clone(),
            ),
        };
        let runtime = match runtime {
            Ok(runtime) => runtime,
            Err(err) => {
                self.layout.close_focused();
                self.layout.focus_pane(previous_focus);
                return Err(err);
            }
        };
        let terminal_id = TerminalId::alloc();
        let terminal = match launch_argv {
            Some(argv) => {
                TerminalState::new(terminal_id.clone(), actual_cwd).with_launch_argv(argv)
            }
            None => TerminalState::new(terminal_id.clone(), actual_cwd),
        };
        self.panes.insert(new_id, PaneState::new(terminal_id));
        self.zoomed = false;
        Ok(NewPane {
            pane_id: new_id,
            terminal,
            runtime,
        })
    }

    #[cfg(test)]
    pub fn close_focused(&mut self) -> Option<DetachedPane> {
        let pane_id = self.layout.focused();
        self.detach_pane(pane_id)
    }

    pub fn close_pane(&mut self, pane_id: PaneId) -> Option<DetachedPane> {
        self.detach_pane(pane_id)
    }

    pub fn remove_pane(&mut self, pane_id: PaneId) -> Option<DetachedPane> {
        self.detach_pane(pane_id)
    }

    pub(crate) fn from_existing_pane(
        number: usize,
        custom_name: Option<String>,
        moved: MovedPane,
        events: mpsc::Sender<AppEvent>,
        render_notify: Arc<Notify>,
        render_dirty: Arc<AtomicBool>,
    ) -> Self {
        let mut panes = HashMap::new();
        let pane_id = moved.pane_id;
        panes.insert(pane_id, moved.pane_state);
        Self {
            custom_name,
            number,
            resumed_session_id: None,
            // The person moved this pane here themselves — nothing to notice,
            // nothing to flash about.
            unseen: false,
            spawned_at: None,
            root_pane: pane_id,
            layout: TileLayout::from_saved(Node::Pane(pane_id), pane_id),
            panes,
            #[cfg(test)]
            runtimes: HashMap::new(),
            zoomed: false,
            events,
            render_notify,
            render_dirty,
        }
    }

    pub(crate) fn take_pane_for_move(&mut self, pane_id: PaneId) -> Option<MovedPane> {
        if !self.panes.contains_key(&pane_id) {
            return None;
        }

        if self.layout.pane_count() > 1 {
            let next_root = self.promoted_root_if_needed(pane_id);
            if self.layout.focused() == pane_id {
                self.layout.close_focused();
            } else {
                let prev_focus = self.layout.focused();
                self.layout.focus_pane(pane_id);
                self.layout.close_focused();
                self.layout.focus_pane(prev_focus);
            }
            if let Some(next_root) = next_root {
                self.root_pane = next_root;
            }
        }

        let pane_state = self.panes.remove(&pane_id)?;
        self.zoomed = false;
        Some(MovedPane {
            pane_id,
            pane_state,
        })
    }

    pub(crate) fn insert_existing_pane(
        &mut self,
        target_pane_id: PaneId,
        moved: MovedPane,
        direction: Direction,
        ratio: f32,
    ) -> Result<PaneId, MovedPane> {
        if !self
            .layout
            .insert_pane_near(target_pane_id, moved.pane_id, direction, ratio)
        {
            return Err(moved);
        }
        let pane_id = moved.pane_id;
        self.panes.insert(pane_id, moved.pane_state);
        self.zoomed = false;
        Ok(pane_id)
    }

    fn detach_pane(&mut self, pane_id: PaneId) -> Option<DetachedPane> {
        if self.layout.pane_count() <= 1 {
            return None;
        }

        let next_root = self.promoted_root_if_needed(pane_id);

        if self.layout.focused() == pane_id {
            self.layout.close_focused();
        } else {
            let prev_focus = self.layout.focused();
            self.layout.focus_pane(pane_id);
            self.layout.close_focused();
            self.layout.focus_pane(prev_focus);
        }

        let pane = self.panes.remove(&pane_id)?;
        let terminal_id = pane.attached_terminal_id;
        self.zoomed = false;
        if let Some(next_root) = next_root {
            self.root_pane = next_root;
        }
        Some((pane_id, terminal_id))
    }

    fn promoted_root_if_needed(&self, closing: PaneId) -> Option<PaneId> {
        if self.root_pane != closing {
            return None;
        }
        self.layout.pane_ids().into_iter().find(|id| *id != closing)
    }

    pub fn terminal_id(&self, pane_id: PaneId) -> Option<&TerminalId> {
        self.panes
            .get(&pane_id)
            .map(|pane| &pane.attached_terminal_id)
    }

    pub fn cwd_for_pane(
        &self,
        pane_id: PaneId,
        terminals: &HashMap<TerminalId, TerminalState>,
        terminal_runtimes: &TerminalRuntimeRegistry,
    ) -> Option<PathBuf> {
        let terminal_id = self.terminal_id(pane_id)?;
        terminal_runtimes
            .get(terminal_id)
            .and_then(|rt| rt.cwd())
            .or_else(|| {
                terminals
                    .get(terminal_id)
                    .map(|terminal| terminal.cwd.clone())
            })
    }

    pub fn foreground_cwd_for_pane(
        &self,
        pane_id: PaneId,
        terminal_runtimes: &TerminalRuntimeRegistry,
    ) -> Option<PathBuf> {
        let terminal_id = self.terminal_id(pane_id)?;
        terminal_runtimes
            .get(terminal_id)
            .and_then(|rt| rt.foreground_cwd())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TP-TAB-FLASH-02: the phase function is the flash. It must open bright
    // (a render within the first half-period always shows the flash), blink
    // on the half-period, and close for good — a phase that never closes is
    // a strip that strobes forever.
    #[test]
    fn the_flash_phase_blinks_through_its_window_and_then_goes_dark() {
        let now = Instant::now();
        let past = |ms: u64| {
            now.checked_sub(Duration::from_millis(ms))
                .expect("test clock has history")
        };

        assert_eq!(flash_phase_at(None, now), None, "no spawn, no flash");
        assert_eq!(
            flash_phase_at(Some(now), now),
            Some(true),
            "the window opens bright"
        );
        assert_eq!(
            flash_phase_at(Some(past(300)), now),
            Some(false),
            "the second half-period is dark — that alternation IS the blink"
        );
        assert_eq!(flash_phase_at(Some(past(600)), now), Some(true));
        assert_eq!(
            flash_phase_at(Some(past(2000)), now),
            None,
            "the window closes for good"
        );
        assert_eq!(flash_phase_at(Some(past(60_000)), now), None);
    }
}
