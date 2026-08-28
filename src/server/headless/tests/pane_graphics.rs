use super::*;

fn receive_render(receiver: &std::sync::mpsc::Receiver<Vec<u8>>, timeout: Duration) -> Vec<u8> {
    receiver.recv_timeout(timeout).unwrap()
}

#[tokio::test]
async fn cold_redraw_advances_one_bounded_layer_after_each_send() {
    let (mut server, client_rx, pane_id) = retained_test_server(b"cold redraw");
    server.app.state.kitty_graphics_enabled = true;
    server.clients.get_mut(&1).unwrap().cell_size = crate::kitty_graphics::HostCellSize {
        width_px: 10,
        height_px: 20,
    };
    const LAYERS: usize = 8;
    for index in 0..LAYERS {
        set_named_graphics_layer(
            &mut server,
            pane_id,
            &format!("layer-{index:02}"),
            vec![index as u8; 1024 * 1024],
            index as i32,
        );
    }

    fill_render_lane(&server);
    server.render_and_stream();
    assert!(server.clients[&1].graphics_cache.is_empty());
    let _older = client_rx.recv_timeout(Duration::from_secs(1)).unwrap();

    for expected in 1..=LAYERS {
        server.render_and_stream();
        let bytes = client_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(bytes.len() <= MAX_GRAPHICS_FRAME_SIZE + 4);
        let frame = read_server_frame(bytes);
        assert_eq!(
            frame
                .graphics
                .windows(4)
                .filter(|part| *part == b"a=t,")
                .count(),
            1
        );
        assert_eq!(
            server.clients[&1].graphics_cache.test_image_count(),
            expected
        );
    }
    assert_eq!(server.clients[&1].deferred_render(), DeferredRender::None);
}

fn enable_graphics_and_render(
    server: &mut HeadlessServer,
    client_rx: &std::sync::mpsc::Receiver<Vec<u8>>,
) -> FrameData {
    server.app.state.kitty_graphics_enabled = true;
    server.clients.get_mut(&1).unwrap().cell_size = crate::kitty_graphics::HostCellSize {
        width_px: 10,
        height_px: 20,
    };
    server.render_and_stream();
    read_server_frame(receive_render(client_rx, Duration::from_millis(100)))
}

fn graphics_key(pane_id: crate::layout::PaneId) -> crate::app::pane_graphics::Key {
    (pane_id, api::schema::PANE_GRAPHICS_PRIMARY_LAYER_ID.into())
}

fn active_gate() -> std::sync::Arc<std::sync::atomic::AtomicBool> {
    std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true))
}

fn set_graphics_layer(server: &mut HeadlessServer, pane_id: crate::layout::PaneId, data: Vec<u8>) {
    set_named_graphics_layer(
        server,
        pane_id,
        api::schema::PANE_GRAPHICS_PRIMARY_LAYER_ID,
        data,
        0,
    );
}

fn set_named_graphics_layer(
    server: &mut HeadlessServer,
    pane_id: crate::layout::PaneId,
    layer_id: &str,
    data: Vec<u8>,
    z_index: i32,
) {
    let key = (pane_id, layer_id.into());
    let host_image_id = server.app.pane_graphics.reserve_image_id(&key).unwrap();
    let layer = crate::app::pane_graphics::Layer::inline(
        api::schema::PaneGraphicsFormat::Png,
        1,
        1,
        data,
        Default::default(),
        z_index,
    );
    server.app.pane_graphics.slots.insert(
        key,
        crate::app::pane_graphics::Slot::test(host_image_id, Some(layer)),
    );
}

fn set_stream_owner(server: &mut HeadlessServer, pane_id: crate::layout::PaneId, owner: &str) {
    let key = graphics_key(pane_id);
    if let Some(slot) = server.app.pane_graphics.slots.get_mut(&key) {
        slot.stream_owner = Some(owner.into());
        slot.stream_active = Some(active_gate());
    } else {
        let host_image_id = server.app.pane_graphics.reserve_image_id(&key).unwrap();
        let mut slot = crate::app::pane_graphics::Slot::test(host_image_id, None);
        slot.stream_owner = Some(owner.into());
        slot.stream_active = Some(active_gate());
        server.app.pane_graphics.slots.insert(key, slot);
    }
}

fn fill_render_lane(server: &HeadlessServer) {
    let queued = HeadlessServer::frame_server_message(&ServerMessage::ReloadSoundConfig)
        .expect("dummy frame");
    server.clients[&1]
        .writer
        .as_ref()
        .unwrap()
        .test_fill_render(queued);
}

fn stream_set_message(
    id: &str,
    pane_id: &str,
    owner: &str,
    data: Vec<u8>,
) -> (api::ApiRequestMessage, std::sync::mpsc::Receiver<String>) {
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    (
        api::ApiRequestMessage {
            request: api::schema::Request {
                id: id.into(),
                method: api::schema::Method::PaneGraphicsStreamSet(
                    api::schema::PaneGraphicsSetParams {
                        pane_id: pane_id.into(),
                        layer_id: None,
                        z_index: 0,
                        owner: owner.into(),
                        format: api::schema::PaneGraphicsFormat::Png,
                        image_width: 1,
                        image_height: 1,
                        data: Some(data),
                        data_base64: String::new(),
                        placement: api::schema::PaneGraphicsPlacementParams::default(),
                    },
                ),
            },
            respond_to,
            response_write_complete: None,
            stream_active: None,
        },
        response_rx,
    )
}

#[cfg(unix)]
fn sparse_direct_frame(
    server: &HeadlessServer,
    name: &str,
    image_width: u32,
    image_height: u32,
) -> String {
    use std::os::unix::fs::OpenOptionsExt as _;

    let path = server
        .app
        .pane_graphics_files
        .source_directory()
        .unwrap()
        .join(name);
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    file.set_len(u64::from(image_width) * u64::from(image_height) * 4)
        .unwrap();
    path.to_string_lossy().into_owned()
}

#[cfg(unix)]
fn direct_stream_message(
    id: &str,
    pane_id: &str,
    owner: &str,
    path: String,
    image_width: u32,
    image_height: u32,
) -> (api::ApiRequestMessage, std::sync::mpsc::Receiver<String>) {
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    (
        api::ApiRequestMessage {
            request: api::schema::Request {
                id: id.into(),
                method: api::schema::Method::PaneGraphicsStreamDirect(
                    api::schema::PaneGraphicsDirectParams {
                        pane_id: pane_id.into(),
                        layer_id: None,
                        z_index: 0,
                        owner: owner.into(),
                        image_width,
                        image_height,
                        format: api::schema::PaneGraphicsFormat::Rgba,
                        path,
                        sequence: 1,
                        revision: 1,
                        placement: Default::default(),
                    },
                ),
            },
            respond_to,
            response_write_complete: None,
            stream_active: None,
        },
        response_rx,
    )
}

#[tokio::test]
async fn pixel_mouse_activation_requires_graphics_demand_not_direct_transport() {
    let (mut server, _client_rx, pane_id) =
        retained_test_server(b"\x1b[?1003h\x1b[?1006h\x1b[?1016h");
    let (writer, control_rx, _render_rx) = test_client_writer();
    let client = server.clients.get_mut(&1).unwrap();
    client.writer = Some(writer);
    client.direct_graphics = false;
    client.pixel_mouse = true;
    client.host_mouse_capture_active = None;
    client.host_sgr_pixels_active = None;
    server.app.direct_graphics_available = false;

    server.stream_host_mouse_capture_mode();
    assert!(matches!(
        read_server_message(control_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
        ServerMessage::MouseCapture {
            enabled: true,
            sgr_pixels: false
        }
    ));

    set_graphics_layer(&mut server, pane_id, vec![1, 2, 3]);
    server.stream_host_mouse_capture_mode();
    assert!(matches!(
        read_server_message(control_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
        ServerMessage::MouseCapture {
            enabled: true,
            sgr_pixels: true
        }
    ));
}

/// TP-INP-MOUSE-05 — TN-4 (PIX-1): a pane that paints Kitty graphics straight through its PTY — terminal-browser
/// and every other native Kitty producer — never registers an API graphics layer. Reading only
/// the API layer left those panes on cell-quantised mouse reports, so any target shorter than one
/// cell was unreachable. Same two-path rule as `pane_is_painting_graphics` (D65).
#[tokio::test]
async fn terminal_native_kitty_graphics_demands_the_pixel_mouse() {
    // Mouse modes plus a real Kitty transmission straight through the PTY - no API layer.
    let (mut server, _client_rx, pane_id) = retained_test_server(
        b"\x1b[?1003h\x1b[?1006h\x1b[?1016h\x1b_Ga=t,f=32,t=d,i=7,s=1,v=1,q=2;/wAA/w==\x1b\\",
    );
    let (writer, control_rx, _render_rx) = test_client_writer();
    let client = server.clients.get_mut(&1).unwrap();
    client.writer = Some(writer);
    client.direct_graphics = false;
    client.pixel_mouse = true;
    client.host_mouse_capture_active = None;
    client.host_sgr_pixels_active = None;
    server.app.direct_graphics_available = false;
    assert!(
        !server.app.pane_graphics.active_for_pane(pane_id),
        "precondition: the pane must have no API graphics layer"
    );

    server.stream_host_mouse_capture_mode();
    assert!(
        matches!(
            read_server_message(control_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
            ServerMessage::MouseCapture {
                enabled: true,
                sgr_pixels: true
            }
        ),
        "a PTY-native Kitty painter must get sub-cell mouse reports"
    );
}

/// TP-INP-MOUSE-05 — TN-7 (PIX-1): a client that cannot report pixel positions must never be asked to enable
/// DECSET 1016, however much the focused pane wants sub-cell input.
#[tokio::test]
async fn a_client_without_the_pixel_mouse_capability_is_never_asked_for_it() {
    let (mut server, _client_rx, _pane_id) = retained_test_server(
        b"\x1b[?1003h\x1b[?1006h\x1b[?1016h\x1b_Ga=t,f=32,t=d,i=7,s=1,v=1,q=2;/wAA/w==\x1b\\",
    );
    let (writer, control_rx, _render_rx) = test_client_writer();
    let client = server.clients.get_mut(&1).unwrap();
    client.writer = Some(writer);
    client.pixel_mouse = false;
    client.host_mouse_capture_active = None;
    client.host_sgr_pixels_active = None;

    server.stream_host_mouse_capture_mode();
    assert!(matches!(
        read_server_message(control_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
        ServerMessage::MouseCapture {
            sgr_pixels: false,
            ..
        }
    ));
}

/// TP-INP-MOUSE-05 — TN-5 (PIX-1): the two-path rule must not fire for ordinary text panes; asking every terminal
/// for DECSET 1016 would widen reports nobody consumes.
#[tokio::test]
async fn a_plain_text_pane_does_not_demand_the_pixel_mouse() {
    let (mut server, _client_rx, _pane_id) =
        retained_test_server(b"\x1b[?1003h\x1b[?1006h\x1b[?1016h plain text, no graphics");
    let (writer, control_rx, _render_rx) = test_client_writer();
    let client = server.clients.get_mut(&1).unwrap();
    client.writer = Some(writer);
    client.pixel_mouse = true;
    client.host_mouse_capture_active = None;
    client.host_sgr_pixels_active = None;

    server.stream_host_mouse_capture_mode();
    assert!(matches!(
        read_server_message(control_rx.recv_timeout(Duration::from_millis(100)).unwrap()),
        ServerMessage::MouseCapture {
            sgr_pixels: false,
            ..
        }
    ));
}

#[tokio::test]
async fn pixel_input_metadata_cannot_resize_authoritative_client_state() {
    let (mut server, _client_rx, pane_id) =
        retained_test_server(b"\x1b[?1003h\x1b[?1006h\x1b[?1016h");
    set_graphics_layer(&mut server, pane_id, vec![1]);
    let client = server.clients.get_mut(&1).unwrap();
    client.pixel_mouse = true;
    client.host_sgr_pixels_active = Some(true);
    server.foreground_client_id = None;
    assert!(!server.handle_server_event(ServerEvent::ClientInputPixels {
        client_id: 1,
        data: b"\x1b[<0;500;300M".to_vec(),
        geometry: crate::input::mouse::HostGeometry::new(80, 24, 800, 480).unwrap(),
    }));
    server.clients.get_mut(&1).unwrap().cell_size = crate::kitty_graphics::HostCellSize {
        width_px: 10,
        height_px: 20,
    };
    for (geometry, data) in [
        (
            crate::input::mouse::HostGeometry::new(100, 30, 1_000, 600).unwrap(),
            b"\x1b[<0;500;300M".as_slice(),
        ),
        (
            crate::input::mouse::HostGeometry::new(80, 24, 960, 480).unwrap(),
            b"\x1b[<0;500;300M",
        ),
        (
            crate::input::mouse::HostGeometry::new(80, 24, 800, 480).unwrap(),
            b"\x1b[<0;0;1M",
        ),
    ] {
        assert!(!server.handle_server_event(ServerEvent::ClientInputPixels {
            client_id: 1,
            data: data.to_vec(),
            geometry,
        }));
    }
    assert_eq!(server.clients[&1].terminal_size, (80, 24));
    assert_eq!(
        (server.effective_size, server.foreground_client_id),
        ((80, 24), None)
    );
}

#[test]
fn direct_eligibility_is_installed_with_the_client_connection() {
    let mut server = test_headless_server();
    let (writer, _control_rx, _render_rx) = test_client_writer();

    assert!(server.handle_server_event(ServerEvent::ClientConnected {
        client_id: 7,
        cols: 80,
        rows: 24,
        cell_width_px: 10,
        cell_height_px: 20,
        render_encoding: RenderEncoding::SemanticFrame,
        keybindings: None,
        direct_attach_requested: false,
        direct_graphics: true,
        pixel_mouse: true,
        writer,
    }));

    let client = server.clients.get(&7).expect("connected client");
    assert!(client.direct_graphics);
    assert_eq!(server.foreground_client_id, Some(7));
    assert!(server.app.direct_graphics_available);
}

#[tokio::test]
async fn focus_repaint_preserves_uploaded_graphics() {
    let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
    let (client_2_writer, _client_2_control_rx, client_2_rx) = test_client_writer();
    server.clients.insert(
        2,
        ClientConnection::new(
            (80, 24),
            crate::kitty_graphics::HostCellSize {
                width_px: 10,
                height_px: 20,
            },
            crate::terminal_theme::TerminalTheme::default(),
            Some(false),
            0,
            RenderEncoding::SemanticFrame,
            Some(client_2_writer),
        ),
    );
    set_graphics_layer(&mut server, pane_id, vec![1, 2, 3]);
    let initial = enable_graphics_and_render(&mut server, &client_rx);
    let initial_graphics = String::from_utf8_lossy(&initial.graphics);
    assert!(initial_graphics.contains("a=t"));
    assert!(initial_graphics.contains("a=p"));
    let client_2_initial =
        read_server_frame(receive_render(&client_2_rx, Duration::from_millis(100)));
    assert!(String::from_utf8_lossy(&client_2_initial.graphics).contains("a=t"));

    assert!(server.handle_server_event(ServerEvent::ClientInput {
        client_id: 2,
        data: b"\x1b[I".to_vec(),
    }));
    assert_eq!(server.foreground_client_id, Some(2));
    server.render_and_stream();

    let focused = read_server_frame(receive_render(&client_2_rx, Duration::from_millis(100)));
    let focused_graphics = String::from_utf8_lossy(&focused.graphics);
    assert!(focused_graphics.contains("a=p"));
    assert!(!focused_graphics.contains("a=t"));
}

#[tokio::test]
async fn resize_replays_placement_without_retransmitting_or_closing_stream() {
    let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
    set_graphics_layer(&mut server, pane_id, vec![1, 2, 3]);
    set_stream_owner(&mut server, pane_id, "owner-resize");
    let initial = enable_graphics_and_render(&mut server, &client_rx);
    assert!(String::from_utf8_lossy(&initial.graphics).contains("a=t"));

    for (cols, rows, cell_width_px, cell_height_px) in
        [(100, 30, 10, 20), (100, 30, 12, 24), (100, 30, 12, 24)]
    {
        assert!(server.handle_server_event(ServerEvent::ClientResize {
            client_id: 1,
            cols,
            rows,
            cell_width_px,
            cell_height_px,
        }));
        server.render_and_stream();
        let frame = read_server_frame(receive_render(&client_rx, Duration::from_millis(100)));
        let graphics = String::from_utf8_lossy(&frame.graphics);
        assert!(!graphics.contains("a=t"));
        assert!(graphics.contains("a=p"));
    }
    assert_eq!(
        server
            .app
            .pane_graphics
            .slots
            .get(&graphics_key(pane_id))
            .and_then(|slot| slot.stream_owner.as_deref()),
        Some("owner-resize")
    );
}

#[tokio::test]
async fn graphics_pruning_preserves_live_panes_and_removes_closed_panes() {
    let (mut server, _client_rx, pane_id) = retained_test_server(b"aaaa");
    set_graphics_layer(&mut server, pane_id, vec![1, 2, 3]);

    assert!(!server
        .app
        .pane_graphics
        .retain_live_panes(&server.app.state));
    assert!(server
        .app
        .pane_graphics
        .slots
        .contains_key(&graphics_key(pane_id)));

    server.app.state.workspaces.clear();
    assert!(server
        .app
        .pane_graphics
        .retain_live_panes(&server.app.state));
    assert!(server.app.pane_graphics.slots.is_empty());
}

#[tokio::test]
async fn retained_update_sends_only_graphics_message() {
    let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
    let baseline = enable_graphics_and_render(&mut server, &client_rx);
    set_graphics_layer(&mut server, pane_id, vec![1, 2, 3]);

    assert_eq!(
        server.render_retained_graphics_update_and_stream(),
        RetainedGraphicsOutcome::Sent
    );
    match read_server_message(
        client_rx
            .recv_timeout(Duration::from_millis(100))
            .expect("graphics-only update"),
    ) {
        ServerMessage::Graphics { bytes } => {
            assert!(bytes.windows(3).any(|window| window == b"\x1b_G"));
        }
        other => panic!("expected graphics-only message, got {other:?}"),
    }
    assert_frame_data_eq(
        server
            .clients
            .get(&1)
            .unwrap()
            .render_state
            .last_frame()
            .expect("semantic baseline"),
        &baseline,
    );
}

#[tokio::test]
async fn retained_graphics_stays_ordered_after_an_older_render() {
    let (mut server, client_rx, pane_id) = retained_test_server(b"aaaa");
    let _ = enable_graphics_and_render(&mut server, &client_rx);
    fill_render_lane(&server);
    set_graphics_layer(&mut server, pane_id, vec![4, 5, 6]);
    let older = client_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert_eq!(
        server.render_retained_graphics_update_and_stream(),
        RetainedGraphicsOutcome::Sent
    );
    let graphics = client_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    assert!(matches!(
        read_server_message(older),
        ServerMessage::ReloadSoundConfig
    ));
    assert!(matches!(
        read_server_message(graphics),
        ServerMessage::Graphics { .. }
    ));
}

#[tokio::test]
async fn retained_update_falls_back_for_mixed_app_geometry() {
    let (mut server, client_rx, _pane_id) = retained_test_server(b"aaaa");
    let _ = enable_graphics_and_render(&mut server, &client_rx);

    let (writer, _control_rx, _render_rx) = test_client_writer();
    server.clients.insert(
        2,
        ClientConnection::new(
            (60, 20),
            crate::kitty_graphics::HostCellSize {
                width_px: 10,
                height_px: 20,
            },
            crate::terminal_theme::TerminalTheme::default(),
            None,
            2,
            RenderEncoding::SemanticFrame,
            Some(writer),
        ),
    );

    assert_eq!(
        server.render_retained_graphics_update_and_stream(),
        RetainedGraphicsOutcome::Fallback
    );
}

#[test]
fn stream_open_gate_is_owned_by_the_layer_and_cancels_on_removal() {
    let mut server = test_headless_server();
    server.app.state.kitty_graphics_enabled = true;
    let workspace = crate::workspace::Workspace::test_new("gated");
    let pane_id = workspace.tabs[0].root_pane;
    let public = format!("{}:p1", workspace.id);
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    let active = active_gate();
    let (respond_to, response_rx) = std::sync::mpsc::channel();

    server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
        request: api::schema::Request {
            id: "open-gated".into(),
            method: api::schema::Method::PaneGraphicsStreamOpen(
                api::schema::PaneGraphicsStreamParams {
                    pane_id: public.clone(),
                    layer_id: None,
                    z_index: 0,
                    owner: "worker-1".into(),
                },
            ),
        },
        respond_to,
        response_write_complete: None,
        stream_active: Some(active.clone()),
    });
    assert!(
        serde_json::from_str::<api::schema::SuccessResponse>(&response_rx.recv().unwrap()).is_ok()
    );
    let (frame, frame_response) =
        stream_set_message("gated-frame", &public, "worker-1", vec![1, 2, 3]);
    assert_eq!(
        server.handle_api_request_with_render_impact(frame),
        RenderImpact::Graphics
    );
    assert!(frame_response.recv().is_ok());
    assert!(active.load(std::sync::atomic::Ordering::Acquire));
    active.store(false, std::sync::atomic::Ordering::Release);
    let (delayed, delayed_response) =
        stream_set_message("delayed-frame", &public, "worker-1", vec![4, 5, 6]);
    assert_eq!(
        server.handle_api_request_with_render_impact(delayed),
        RenderImpact::None
    );
    let error: api::schema::ErrorResponse =
        serde_json::from_str(&delayed_response.recv().unwrap()).unwrap();
    assert_eq!(error.error.code, "stream_closed");
    assert!(server
        .app
        .pane_graphics
        .slots
        .remove(&graphics_key(pane_id))
        .is_some());
    assert!(!active.load(std::sync::atomic::Ordering::Acquire));
}

#[test]
fn stream_set_has_graphics_only_render_impact() {
    let mut server = test_headless_server();
    let workspace = crate::workspace::Workspace::test_new("graphics");
    let pane_id = workspace.tabs[0].root_pane;
    let public_pane_id = format!("{}:p1", workspace.id);
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.kitty_graphics_enabled = true;
    set_stream_owner(&mut server, pane_id, "owner-a");

    let (request, response_rx) =
        stream_set_message("wrong-owner", &public_pane_id, "owner-b", vec![1, 2, 3]);
    assert_eq!(
        server.handle_api_request_with_render_impact(request),
        RenderImpact::None
    );
    assert!(serde_json::from_str::<api::schema::ErrorResponse>(
        &response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap()
    )
    .is_ok());

    let (request, response_rx) =
        stream_set_message("stream-frame", &public_pane_id, "owner-a", vec![1, 2, 3]);
    assert_eq!(
        server.handle_api_request_with_render_impact(request),
        RenderImpact::Graphics
    );
    assert!(serde_json::from_str::<api::schema::SuccessResponse>(
        &response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap()
    )
    .is_ok());

    server
        .app
        .event_tx
        .try_send(AppEvent::UpdateReady {
            version: "9.9.9".into(),
            install_command: "herdr update".into(),
        })
        .unwrap();
    let (request, _response_rx) = stream_set_message(
        "stream-frame-with-internal-event",
        &public_pane_id,
        "owner-a",
        vec![4, 5, 6],
    );
    assert_eq!(
        server.handle_api_request_with_render_impact(request),
        RenderImpact::Full
    );

    server.app.pane_graphics.clear();
    let (respond_to, _response_rx) = std::sync::mpsc::channel();
    let impact = server.handle_api_request_with_render_impact(api::ApiRequestMessage {
        request: api::schema::Request {
            id: "direct-frame".into(),
            method: api::schema::Method::PaneGraphicsSet(api::schema::PaneGraphicsSetParams {
                pane_id: public_pane_id,
                layer_id: None,
                z_index: 0,
                owner: String::new(),
                format: api::schema::PaneGraphicsFormat::Png,
                image_width: 1,
                image_height: 1,
                data: Some(vec![1, 2, 3]),
                data_base64: String::new(),
                placement: api::schema::PaneGraphicsPlacementParams::default(),
            }),
        },
        respond_to,
        response_write_complete: None,
        stream_active: None,
    });
    assert_eq!(impact, RenderImpact::Full);
}

#[test]
fn rejected_or_stale_requests_do_not_schedule_rendering() {
    let mut server = test_headless_server();
    let workspace = crate::workspace::Workspace::test_new("graphics");
    let pane_id = workspace.tabs[0].root_pane;
    let public_pane_id = format!("{}:p1", workspace.id);
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    server.app.state.selected = 0;

    let (respond_to, response_rx) = std::sync::mpsc::channel();
    let changed = server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
        request: api::schema::Request {
            id: "disabled-set".into(),
            method: api::schema::Method::PaneGraphicsSet(api::schema::PaneGraphicsSetParams {
                pane_id: public_pane_id.clone(),
                layer_id: None,
                z_index: 0,
                owner: String::new(),
                format: api::schema::PaneGraphicsFormat::Png,
                image_width: 1,
                image_height: 1,
                data: Some(vec![1, 2, 3]),
                data_base64: String::new(),
                placement: api::schema::PaneGraphicsPlacementParams::default(),
            }),
        },
        respond_to,
        response_write_complete: None,
        stream_active: None,
    });
    assert!(!changed);
    let response = response_rx
        .recv_timeout(Duration::from_millis(100))
        .unwrap();
    assert_eq!(
        serde_json::from_str::<api::schema::ErrorResponse>(&response)
            .unwrap()
            .error
            .code,
        "feature_disabled"
    );

    server.app.state.kitty_graphics_enabled = true;
    set_stream_owner(&mut server, pane_id, "current-owner");
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    let impact = server.handle_api_request_with_render_impact(api::ApiRequestMessage {
        request: api::schema::Request {
            id: "stale-close".into(),
            method: api::schema::Method::PaneGraphicsStreamClose(
                api::schema::PaneGraphicsStreamParams {
                    pane_id: public_pane_id,
                    layer_id: None,
                    z_index: 0,
                    owner: "stale-owner".into(),
                },
            ),
        },
        respond_to,
        response_write_complete: None,
        stream_active: None,
    });
    assert_eq!(impact, RenderImpact::None);
    assert_eq!(
        server
            .app
            .pane_graphics
            .slots
            .get(&graphics_key(pane_id))
            .and_then(|slot| slot.stream_owner.as_deref()),
        Some("current-owner")
    );
    assert!(serde_json::from_str::<api::schema::SuccessResponse>(
        &response_rx
            .recv_timeout(Duration::from_millis(100))
            .unwrap()
    )
    .is_ok());
}

#[cfg(unix)]
#[tokio::test]
async fn hidden_large_direct_frame_uploads_then_replays_placement_without_closing_stream() {
    let (mut server, client_rx, _) = retained_test_server(b"active");
    enable_graphics_and_render(&mut server, &client_rx);
    let background_tab = server.app.state.workspaces[0].test_add_tab(Some("browser"));
    let pane_id = server.app.state.workspaces[0].tabs[background_tab].root_pane;
    let pane_number = server.app.state.workspaces[0]
        .public_pane_number(pane_id)
        .unwrap();
    let public_pane_id = crate::workspace::public_pane_id_for_number(
        &server.app.state.workspaces[0].id,
        pane_number,
    );
    server.clients.get_mut(&1).unwrap().direct_graphics = true;
    server.app.direct_graphics_available = true;
    set_stream_owner(&mut server, pane_id, "browser");

    let image_width = 2_048;
    let image_height = 2_049;
    let expected_len = u64::from(image_width) * u64::from(image_height) * 4;
    assert!(expected_len > api::schema::PANE_GRAPHICS_STREAM_MAX_BYTES as u64);
    let path = sparse_direct_frame(
        &server,
        "hidden-large-frame.rgba",
        image_width,
        image_height,
    );
    let (message, response_rx) = direct_stream_message(
        "hidden-frame",
        &public_pane_id,
        "browser",
        path,
        image_width,
        image_height,
    );

    assert_eq!(
        server.handle_pane_graphics_stream_frame(message),
        RenderImpact::None
    );
    let (transfer_id, image_id, control, leading) = match read_server_message(
        client_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("hidden direct upload"),
    ) {
        ServerMessage::GraphicsFile {
            transfer_id,
            image_id,
            control,
            leading,
            expected_len: sent_len,
            ..
        } => {
            assert_eq!(sent_len, expected_len);
            (transfer_id, image_id, control, leading)
        }
        other => panic!("expected graphics file, got {other:?}"),
    };
    assert!(leading.is_empty());
    assert!(control.starts_with("a=t,"), "{control}");
    assert!(!control.contains("p="), "{control}");
    assert!(response_rx.try_recv().is_err());

    // Rebased for the fork's per-display tabs (TP-MCF-TAB-01): switch the
    // tab inside client 1's viewer window so the client's own tab ledger
    // moves; a bare switch_tab would leave the client on its recorded tab.
    let viewer = server.app.state.enter_viewer(Some(1));
    server.app.state.workspaces[0].switch_tab(background_tab);
    server.app.state.restore_viewer(viewer);
    server.render_and_stream();
    let frame = read_server_frame(
        client_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("frame while upload is pending"),
    );
    assert!(!frame.graphics.windows(4).any(|bytes| bytes == b"a=p,"));

    let viewer = server.app.state.enter_viewer(Some(1));
    server.app.state.workspaces[0].switch_tab(0);
    server.app.state.restore_viewer(viewer);
    server.render_and_stream();
    let _hidden_again = read_server_frame(
        client_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("frame after hiding pending upload"),
    );
    server.start_direct_graphics_response(1, transfer_id, image_id);
    assert!(server.complete_direct_graphics(1, transfer_id, image_id, true));
    assert!(serde_json::from_str::<api::schema::SuccessResponse>(
        &response_rx.recv_timeout(Duration::from_secs(1)).unwrap()
    )
    .is_ok());
    let slot = &server.app.pane_graphics.slots[&graphics_key(pane_id)];
    assert!(slot.stream_is_active());
    assert!(slot.layer.as_ref().unwrap().terminal_only());

    let viewer = server.app.state.enter_viewer(Some(1));
    server.app.state.workspaces[0].switch_tab(background_tab);
    server.app.state.restore_viewer(viewer);
    server.render_and_stream();
    let frame = read_server_frame(
        client_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("placement replay after tab switch"),
    );
    let graphics = String::from_utf8_lossy(&frame.graphics);
    assert!(graphics.contains("a=p,"), "{graphics:?}");
    assert!(graphics.contains(&format!("i={image_id}")), "{graphics:?}");
    assert!(!graphics.contains("a=t,"), "{graphics:?}");

    let next_path = sparse_direct_frame(
        &server,
        "visible-next-frame.rgba",
        image_width,
        image_height,
    );
    let (message, next_response_rx) = direct_stream_message(
        "visible-frame",
        &public_pane_id,
        "browser",
        next_path,
        image_width,
        image_height,
    );
    assert_eq!(
        server.handle_pane_graphics_stream_frame(message),
        RenderImpact::None
    );
    match read_server_message(
        client_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("next visible direct frame"),
    ) {
        ServerMessage::GraphicsFile { control, .. } => {
            assert!(control.starts_with("a=T,"), "{control}");
        }
        other => panic!("expected graphics file, got {other:?}"),
    }
    assert!(next_response_rx.try_recv().is_err());
    assert!(server.app.pane_graphics.slots[&graphics_key(pane_id)].stream_is_active());
}

#[cfg(unix)]
#[tokio::test]
async fn hidden_small_direct_frame_preserves_owned_inline_fallback() {
    let (mut server, client_rx, _) = retained_test_server(b"active");
    enable_graphics_and_render(&mut server, &client_rx);
    let background_tab = server.app.state.workspaces[0].test_add_tab(Some("browser"));
    let pane_id = server.app.state.workspaces[0].tabs[background_tab].root_pane;
    let pane_number = server.app.state.workspaces[0]
        .public_pane_number(pane_id)
        .unwrap();
    let public_pane_id = crate::workspace::public_pane_id_for_number(
        &server.app.state.workspaces[0].id,
        pane_number,
    );
    server.clients.get_mut(&1).unwrap().direct_graphics = true;
    server.app.direct_graphics_available = true;
    set_stream_owner(&mut server, pane_id, "browser");

    let path = sparse_direct_frame(&server, "hidden-small-frame.rgba", 1, 1);
    let (message, response_rx) =
        direct_stream_message("hidden-small", &public_pane_id, "browser", path, 1, 1);
    assert_eq!(
        server.handle_pane_graphics_stream_frame(message),
        RenderImpact::Graphics
    );
    assert!(serde_json::from_str::<api::schema::SuccessResponse>(
        &response_rx.recv_timeout(Duration::from_secs(1)).unwrap()
    )
    .is_ok());
    assert!(client_rx.recv_timeout(Duration::from_millis(50)).is_err());
    let slot = &server.app.pane_graphics.slots[&graphics_key(pane_id)];
    assert!(slot.stream_is_active());
    assert_eq!(
        slot.layer.as_ref().unwrap().inline_data(),
        Some([0; 4].as_slice())
    );
}

#[cfg(unix)]
#[tokio::test]
async fn direct_frame_during_internal_redraw_uploads_without_placement() {
    let (mut server, client_rx, pane_id) = retained_test_server(b"active");
    enable_graphics_and_render(&mut server, &client_rx);
    let pane_number = server.app.state.workspaces[0]
        .public_pane_number(pane_id)
        .unwrap();
    let public_pane_id = crate::workspace::public_pane_id_for_number(
        &server.app.state.workspaces[0].id,
        pane_number,
    );
    server.clients.get_mut(&1).unwrap().direct_graphics = true;
    server.app.direct_graphics_available = true;
    set_stream_owner(&mut server, pane_id, "browser");
    server
        .app
        .event_tx
        .try_send(AppEvent::UpdateReady {
            version: "9.9.9".into(),
            install_command: "herdr update".into(),
        })
        .unwrap();

    let image_width = 2_048;
    let image_height = 2_049;
    let path = sparse_direct_frame(&server, "redraw-frame.rgba", image_width, image_height);
    let (message, response_rx) = direct_stream_message(
        "redraw",
        &public_pane_id,
        "browser",
        path,
        image_width,
        image_height,
    );
    assert_eq!(
        server.handle_pane_graphics_stream_frame(message),
        RenderImpact::Full
    );
    let (transfer_id, image_id) = match read_server_message(
        client_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("direct upload during redraw"),
    ) {
        ServerMessage::GraphicsFile {
            control,
            leading,
            transfer_id,
            image_id,
            ..
        } => {
            assert!(leading.is_empty());
            assert!(control.starts_with("a=t,"), "{control}");
            (transfer_id, image_id)
        }
        other => panic!("expected graphics file, got {other:?}"),
    };
    server.start_direct_graphics_response(1, transfer_id, image_id);
    assert!(server.complete_direct_graphics(1, transfer_id, image_id, true));
    assert!(response_rx.recv_timeout(Duration::from_secs(1)).is_ok());
    assert!(server.app.pane_graphics.slots[&graphics_key(pane_id)].stream_is_active());

    server.render_and_stream();
    let frame = read_server_frame(
        client_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("placement after redraw upload acknowledgement"),
    );
    let graphics = String::from_utf8_lossy(&frame.graphics);
    assert!(graphics.contains("a=p,"), "{graphics:?}");
    assert!(graphics.contains(&format!("i={image_id}")), "{graphics:?}");
    assert!(!graphics.contains("a=t,"), "{graphics:?}");
}

#[cfg(unix)]
fn direct_gate_server(
    data: &[u8],
) -> (
    HeadlessServer,
    crate::app::pane_graphics::Key,
    std::sync::mpsc::Receiver<String>,
) {
    direct_gate_server_with_file(data.len(), Some(data))
}

#[cfg(unix)]
fn direct_gate_server_with_file(
    len: usize,
    data: Option<&[u8]>,
) -> (
    HeadlessServer,
    crate::app::pane_graphics::Key,
    std::sync::mpsc::Receiver<String>,
) {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;
    let mut server = test_headless_server();
    let workspace = crate::workspace::Workspace::test_new("direct-gate");
    let pane_id = workspace.tabs[0].root_pane;
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    let key = graphics_key(pane_id);
    let path = server
        .app
        .pane_graphics_files
        .source_directory()
        .unwrap()
        .join("gate-frame");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    if let Some(data) = data {
        file.write_all(data).unwrap();
    } else {
        file.set_len(len as u64).unwrap();
    }
    drop(file);
    let lease = server.app.pane_graphics_files.lease(&path, len).unwrap();
    let (respond_to, response_rx) = std::sync::mpsc::channel();
    let layer =
        crate::app::pane_graphics::Layer::direct(1, 1, lease.clone(), Default::default(), 0);
    let mut slot = crate::app::pane_graphics::Slot::test((1 << 31) | 900, Some(layer));
    slot.stream_owner = Some("owner".into());
    slot.stream_active = Some(active_gate());
    slot.direct_gate = Some(crate::app::pane_graphics::DirectGate {
        transfer_id: lease.fingerprint(),
        client_id: 7,
        deadline: std::time::Instant::now() + Duration::from_secs(1),
        written: true,
        success_response: "ack".into(),
        respond_to,
    });
    server.app.pane_graphics.slots.insert(key.clone(), slot);
    (server, key, response_rx)
}

#[cfg(unix)]
fn direct_ids(server: &HeadlessServer, key: &crate::app::pane_graphics::Key) -> (u64, u32) {
    let slot = &server.app.pane_graphics.slots[key];
    (
        slot.direct_gate.as_ref().unwrap().transfer_id,
        slot.host_image_id,
    )
}

#[cfg(unix)]
fn add_direct_client(server: &mut HeadlessServer, client_id: u64) {
    let (writer, control_rx, render_rx) = test_client_writer();
    std::mem::forget((control_rx, render_rx));
    let mut client = ClientConnection::new(
        (80, 24),
        crate::kitty_graphics::HostCellSize {
            width_px: 10,
            height_px: 20,
        },
        crate::terminal_theme::TerminalTheme::default(),
        None,
        1,
        RenderEncoding::SemanticFrame,
        Some(writer),
    );
    client.direct_graphics = true;
    client.pixel_mouse = true;
    server.clients.insert(client_id, client);
}

#[cfg(unix)]
#[test]
fn terminal_response_deadline_starts_only_after_client_flush() {
    let (mut server, key, _response_rx) = direct_gate_server(&[1, 2, 3, 4]);
    let slot = server.app.pane_graphics.slots.get_mut(&key).unwrap();
    let gate = slot.direct_gate.as_mut().unwrap();
    gate.written = false;
    let (transfer_id, image_id) = (gate.transfer_id, slot.host_image_id);
    assert!(!server.complete_direct_graphics(7, transfer_id, image_id, true));
    assert!(!server.start_direct_graphics_response(7, transfer_id, image_id));
    let gate = server.app.pane_graphics.slots[&key]
        .direct_gate
        .as_ref()
        .unwrap();
    assert!(gate.written && gate.deadline > std::time::Instant::now());
}

#[cfg(unix)]
#[test]
fn outer_timeout_covers_both_direct_phases_and_cancellation_blocks_late_results() {
    assert!(
        crate::app::pane_graphics::DIRECT_OUTER_TIMEOUT
            > crate::app::pane_graphics::DIRECT_DELIVERY_TIMEOUT
                + crate::app::pane_graphics::DIRECT_RESPONSE_TIMEOUT
    );
    let (mut server, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
    let slot = server.app.pane_graphics.slots.get_mut(&key).unwrap();
    slot.stream_active
        .as_ref()
        .unwrap()
        .store(false, std::sync::atomic::Ordering::Release);
    let (transfer_id, image_id) = (
        slot.direct_gate.as_ref().unwrap().transfer_id,
        slot.host_image_id,
    );

    assert!(!server.complete_direct_graphics(7, transfer_id, image_id, true));
    assert!(response_rx.try_recv().is_err());
    assert!(server.app.pane_graphics.slots[&key]
        .layer
        .as_ref()
        .unwrap()
        .direct_lease()
        .is_some());
}

#[cfg(unix)]
#[test]
fn matching_terminal_ok_releases_producer_and_acknowledges() {
    let (mut server, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
    let (transfer_id, image_id) = direct_ids(&server, &key);

    assert!(server.complete_direct_graphics(7, transfer_id, image_id, true));

    assert_eq!(response_rx.recv().unwrap(), "ack");
    let layer = server.app.pane_graphics.slots[&key].layer.as_ref().unwrap();
    assert!(layer.terminal_only());
    assert!(layer.direct_lease().is_none());
}

#[cfg(unix)]
#[test]
fn explicit_terminal_error_acks_only_after_owned_inline_fallback() {
    let (mut server, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
    add_direct_client(&mut server, 7);
    let (transfer_id, image_id) = direct_ids(&server, &key);
    let layer = server.app.pane_graphics.slots[&key].layer.as_ref().unwrap();
    server
        .clients
        .get_mut(&7)
        .unwrap()
        .graphics_cache
        .trust_pane_layer(&key, image_id, layer);
    assert!(server.complete_direct_graphics(7, transfer_id, image_id, false));

    let layer = server.app.pane_graphics.slots[&key].layer.as_ref().unwrap();
    assert_eq!(
        (
            response_rx.recv().unwrap(),
            layer.inline_data(),
            server.clients[&7].direct_graphics,
            server.clients[&7].pixel_mouse,
        ),
        ("ack".into(), Some([1, 2, 3, 4].as_slice()), false, true)
    );
    assert!(server.clients[&7].graphics_cache.is_empty());
}

#[cfg(unix)]
#[test]
fn large_direct_terminal_error_closes_without_acknowledging_or_copying() {
    let len = crate::api::schema::PANE_GRAPHICS_STREAM_MAX_BYTES + 4;
    let (mut server, key, response_rx) = direct_gate_server_with_file(len, None);
    add_direct_client(&mut server, 7);
    let (transfer_id, image_id) = direct_ids(&server, &key);

    assert!(server.complete_direct_graphics(7, transfer_id, image_id, false));
    assert!(!server.app.pane_graphics.slots.contains_key(&key));
    assert!(matches!(
        response_rx.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Disconnected)
    ));
}

#[cfg(unix)]
#[test]
fn unwritten_direct_full_falls_back_without_stickiness_but_disconnect_retires() {
    for error in [
        std::sync::mpsc::TrySendError::Full(Vec::new()),
        std::sync::mpsc::TrySendError::Disconnected(Vec::new()),
    ] {
        let should_ack = matches!(error, std::sync::mpsc::TrySendError::Full(_));
        let (mut server, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
        add_direct_client(&mut server, 7);
        let gate = server
            .app
            .pane_graphics
            .slots
            .get_mut(&key)
            .and_then(|slot| slot.direct_gate.take())
            .unwrap();
        let result = server.handle_unwritten_direct_failure(
            &key,
            gate.success_response,
            gate.respond_to,
            error,
        );
        let inline = server
            .app
            .pane_graphics
            .slots
            .get(&key)
            .and_then(|slot| slot.layer.as_ref()?.inline_data())
            .is_some();
        assert_eq!(
            (
                result,
                response_rx.try_recv().ok().as_deref() == Some("ack"),
                inline,
                server.clients[&7].direct_graphics,
            ),
            (should_ack, should_ack, should_ack, true)
        );
    }
}

#[cfg(unix)]
#[test]
fn client_loss_retires_only_its_direct_stream() {
    let (mut pending, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
    pending.retire_direct_graphics_for_client(8);
    assert!(pending.app.pane_graphics.slots.contains_key(&key));
    pending.retire_direct_graphics_for_client(7);
    assert!(!pending.app.pane_graphics.slots.contains_key(&key));
    assert!(response_rx.recv().is_err());

    let (mut resident, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
    let slot = resident.app.pane_graphics.slots.get(&key).unwrap();
    assert!(resident.complete_direct_graphics(
        7,
        slot.direct_gate.as_ref().unwrap().transfer_id,
        slot.host_image_id,
        true,
    ));
    assert_eq!(response_rx.recv().unwrap(), "ack");
    resident.retire_direct_graphics_for_client(8);
    assert!(resident.app.pane_graphics.slots.contains_key(&key));
    resident.retire_direct_graphics_for_client(7);
    assert!(!resident.app.pane_graphics.slots.contains_key(&key));
}

#[cfg(unix)]
#[test]
fn pane_removal_and_shutdown_drop_direct_without_ack() {
    let setups: [fn(&mut HeadlessServer); 2] = [
        |server| server.app.state.workspaces.clear(),
        |server| server.shutting_down = true,
    ];
    for setup in setups {
        let (mut server, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
        let (transfer_id, image_id) = direct_ids(&server, &key);
        setup(&mut server);
        assert!(!server.complete_direct_graphics(7, transfer_id, image_id, true));
        assert!(response_rx.recv().is_err());
        assert!(!server.app.pane_graphics.slots.contains_key(&key));
    }
}

#[cfg(unix)]
#[test]
fn timeout_retires_stream_without_producer_ack() {
    let (mut server, key, response_rx) = direct_gate_server(&[1, 2, 3, 4]);
    add_direct_client(&mut server, 7);
    server
        .app
        .pane_graphics
        .slots
        .get_mut(&key)
        .unwrap()
        .direct_gate
        .as_mut()
        .unwrap()
        .deadline = std::time::Instant::now() - Duration::from_millis(1);

    assert!(server.expire_direct_graphics(std::time::Instant::now()));

    assert!(response_rx.recv().is_err());
    assert!(!server.app.pane_graphics.slots.contains_key(&key));
    assert!(!server.clients[&7].direct_graphics);
    assert!(server.clients[&7].pixel_mouse);
}

// ---------------------------------------------------------------------------
// K3 / BRW-2.1 repro probe. The user narrowed the browser pane and a frame of
// an already-closed video stayed painted OUTSIDE the pane. tb rewrites its
// placement grid only when it sends a NEW frame, so the reproduction is: a
// wide placement on screen, the visible area narrows, and no new frame ever
// arrives. The server must either re-clip the placement to the narrower area
// or delete it; leaving the wide placement standing is the reported defect.
fn first_control_field(graphics: &str, action: &str, field: &str) -> Option<u32> {
    let start = graphics.find(action)?;
    let tail = &graphics[start..];
    let end = tail.find('\x1b').unwrap_or(tail.len());
    let control = &tail[..end];
    let key = format!("{field}=");
    let pos = control.find(&key)? + key.len();
    let digits: String = control[pos..]
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    digits.parse().ok()
}

// TP-GFX-STREAM-02
#[tokio::test]
async fn a_live_stream_owner_is_recognised_by_the_cancel_sweep() {
    let (mut server, _rx, pane_id) = retained_test_server(b"stream owner");
    set_stream_owner(&mut server, pane_id, "owner-live");
    // A1 — the defect's unit: a slot that is alive and active MUST count as
    // active, or the sweep cancels every stream the moment it registers
    // (measured live on 2026-08-27: 898 one-per-second stream_closed retries,
    // a frozen browser frame on every tab, and the pixel mouse withdrawn).
    assert!(
        server.pane_graphics_stream_owner_is_active("owner-live"),
        "a live, active stream owner must be recognised"
    );
    // A4 — no cross-owner match.
    assert!(!server.pane_graphics_stream_owner_is_active("owner-other"));
    // A2 — a slot whose gate closed is a ghost: the sweep may cancel it.
    if let Some(active) = server
        .app
        .pane_graphics
        .slots
        .get(&graphics_key(pane_id))
        .and_then(|slot| slot.stream_active.clone())
    {
        active.store(false, std::sync::atomic::Ordering::Release);
    }
    assert!(!server.pane_graphics_stream_owner_is_active("owner-live"));
    // A3 — an empty world is safely inactive.
    server.app.pane_graphics.slots.clear();
    assert!(!server.pane_graphics_stream_owner_is_active("owner-live"));
}

fn alive_placements(feed: &mut std::collections::HashMap<(String, String), u32>, graphics: &str) {
    // A miniature kitty: apply every graphics command in order and keep the
    // set of placements a real terminal would still be showing.
    let mut rest = graphics;
    while let Some(start) = rest.find("\u{1b}_G") {
        let tail = &rest[start + 3..];
        let end = tail.find('\u{1b}').unwrap_or(tail.len());
        let head = &tail[..end];
        let head = head.split(';').next().unwrap_or(head);
        let mut a = "";
        let mut i = "";
        let mut pl = "";
        let mut d = "";
        let mut c = 0u32;
        for part in head.split(',') {
            let Some((k, v)) = part.split_once('=') else {
                continue;
            };
            match k {
                "a" => a = v,
                "i" => i = v,
                "p" => pl = v,
                "d" => d = v,
                "c" => c = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        match a {
            "T" | "p" => {
                feed.insert((i.to_owned(), pl.to_owned()), c);
            }
            "d" => match d {
                "A" | "a" => feed.clear(),
                "I" | "i" if d == "I" => {
                    feed.retain(|(image, _), _| image != i);
                }
                _ => {
                    feed.remove(&(i.to_owned(), pl.to_owned()));
                }
            },
            _ => {}
        }
        rest = &rest[start + 3..];
    }
}

fn drain_alive(
    rx: &std::sync::mpsc::Receiver<Vec<u8>>,
    feed: &mut std::collections::HashMap<(String, String), u32>,
) -> usize {
    let mut messages = 0;
    while let Ok(bytes) = rx.recv_timeout(Duration::from_millis(80)) {
        messages += 1;
        match read_server_message(bytes) {
            ServerMessage::Frame(frame) => {
                alive_placements(feed, &String::from_utf8_lossy(&frame.graphics));
            }
            ServerMessage::Graphics { bytes } => {
                alive_placements(feed, &String::from_utf8_lossy(&bytes));
            }
            _ => {}
        }
    }
    messages
}

fn push_tb_frame(server: &mut HeadlessServer, pane_id: crate::layout::PaneId, tick: u8) {
    // One terminal-native video frame, the way the fallback browser paints:
    // same kitty image id and placement id every frame, new pixel content —
    // which gives the host a new content-hashed image id per frame.
    let pixels: Vec<u8> = (0..16).map(|n| n ^ tick).collect();
    use base64::Engine as _;
    let payload = base64::engine::general_purpose::STANDARD.encode(&pixels);
    let apc = format!("\u{1b}_Ga=T,f=32,s=2,v=2,i=77,p=9,q=2;{payload}\u{1b}\\");
    if let Some(runtime) =
        server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
    {
        runtime.test_process_pty_bytes(apc.as_bytes());
    }
}

// TP-GFX-LEDGER-01
#[tokio::test]
async fn resizing_a_streaming_terminal_pane_leaves_no_orphan_placements() {
    let (mut server, rx1, pane_id) = retained_test_server(b"video pane");
    server.app.state.kitty_graphics_enabled = true;
    server.clients.get_mut(&1).unwrap().cell_size = crate::kitty_graphics::HostCellSize {
        width_px: 10,
        height_px: 20,
    };
    server.sync_foreground_client_state();
    server.resize_shared_runtime_to_effective_size();

    // A neighbour pane holding an API graphics layer keeps the slot pool
    // non-empty, which is what routes terminal-source placements through the
    // incremental encoder in the live session.
    let neighbour =
        server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    set_graphics_layer(&mut server, neighbour, vec![7, 7, 7, 7]);
    server.render_and_stream();
    let mut kitty = std::collections::HashMap::new();
    drain_alive(&rx1, &mut kitty);

    let area = server.app.state.view.terminal_area;
    let gesture: &[Option<(crate::api::schema::PaneDirection, f32)>] = &[
        None,
        None,
        None,
        Some((crate::api::schema::PaneDirection::Right, 0.20)),
        None,
        None,
        Some((crate::api::schema::PaneDirection::Left, 0.12)),
        None,
        None,
        Some((crate::api::schema::PaneDirection::Left, 0.10)),
        None,
        None,
    ];
    for (turn, step) in gesture.iter().enumerate() {
        if let Some((direction, amount)) = step {
            let moved = server.app.state.workspaces[0].tabs[0].layout.resize_pane(
                pane_id,
                (*direction).into(),
                *amount,
                area,
            );
            assert!(moved, "resize step must change the layout");
        }
        push_tb_frame(&mut server, pane_id, turn as u8 + 1);
        // The live wire alternates the two encode streams: the text frame
        // path and the graphics-only retained path.
        if turn % 2 == 0 {
            server.render_and_stream();
            println!("turn={turn} path=full");
        } else {
            let outcome = server.render_retained_graphics_update_and_stream();
            println!("turn={turn} path=retained outcome={outcome:?}");
        }
        // A slow reader: drain only every second turn so the capacity-1
        // render channel spends half the gesture full, like an ssh client.
        if turn % 2 == 1 {
            let n = drain_alive(&rx1, &mut kitty);
            println!("turn={turn} drained={n} alive={}", kitty.len());
        }
    }
    for _ in 0..4 {
        server.render_and_stream();
        let _ = server.render_retained_graphics_update_and_stream();
        drain_alive(&rx1, &mut kitty);
    }
    drain_alive(&rx1, &mut kitty);

    // Reserved ids (bit 31 set) carry the neighbour's API layer; the
    // browser's terminal-source images live in the small hashed range.
    let pane_alive: Vec<_> = kitty
        .iter()
        .filter(|((image, _), cols)| {
            **cols > 0 && image.parse::<u64>().is_ok_and(|id| id < 0x8000_0000)
        })
        .map(|((image, placement), cols)| format!("i={image} p={placement} c={cols}"))
        .collect();
    assert!(
        pane_alive.len() <= 1,
        "the terminal still shows {} placements for one pane; a resize must \
         not strand earlier frames: {pane_alive:?}",
        pane_alive.len()
    );
}

fn drain_alive_fast(
    rx: &std::sync::mpsc::Receiver<Vec<u8>>,
    feed: &mut std::collections::HashMap<(String, String), u32>,
) -> usize {
    // The render channel's sender runs on this same thread, so anything the
    // server managed to enqueue is already there: no timeout needed.
    let mut messages = 0;
    while let Ok(bytes) = rx.try_recv() {
        messages += 1;
        match read_server_message(bytes) {
            ServerMessage::Frame(frame) => {
                alive_placements(feed, &String::from_utf8_lossy(&frame.graphics));
            }
            ServerMessage::Graphics { bytes } => {
                alive_placements(feed, &String::from_utf8_lossy(&bytes));
            }
            _ => {}
        }
    }
    messages
}

// TP-GFX-LEDGER-01
#[tokio::test]
async fn alternating_render_paths_never_strand_a_terminal_placement() {
    // The live wire interleaves the full text-frame path with the graphics-only
    // retained path against a capacity-1 client channel, while the reader
    // drains at its own pace. A fixed alternation leaves the retained path
    // permanently Deferred, so the Sent/Deferred hand-over weave was never
    // exercised before. Sweep every 8-bit interleaving of the first eight
    // turns crossed with five reader cadences; after convergence the simulated
    // kitty and the server's placement ledger must agree, and at most one
    // terminal placement may survive for the pane.
    let gesture: &[Option<(crate::api::schema::PaneDirection, f32)>] = &[
        None,
        None,
        None,
        Some((crate::api::schema::PaneDirection::Right, 0.20)),
        None,
        None,
        Some((crate::api::schema::PaneDirection::Left, 0.12)),
        None,
        None,
        Some((crate::api::schema::PaneDirection::Left, 0.10)),
        None,
        None,
    ];
    for path_mask in 0..256u32 {
        for drain_mask in [0xFFFu32, 0xAAA, 0x555, 0x0F0, 0x000] {
            let (mut server, rx1, pane_id) = retained_test_server(b"video pane");
            server.app.state.kitty_graphics_enabled = true;
            server.clients.get_mut(&1).unwrap().cell_size = crate::kitty_graphics::HostCellSize {
                width_px: 10,
                height_px: 20,
            };
            server.sync_foreground_client_state();
            server.resize_shared_runtime_to_effective_size();
            let neighbour =
                server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
            set_graphics_layer(&mut server, neighbour, vec![7, 7, 7, 7]);
            server.render_and_stream();
            let mut kitty = std::collections::HashMap::new();
            drain_alive_fast(&rx1, &mut kitty);

            let area = server.app.state.view.terminal_area;
            for (turn, step) in gesture.iter().enumerate() {
                if let Some((direction, amount)) = step {
                    let moved = server.app.state.workspaces[0].tabs[0].layout.resize_pane(
                        pane_id,
                        (*direction).into(),
                        *amount,
                        area,
                    );
                    assert!(moved, "resize step must change the layout");
                }
                push_tb_frame(&mut server, pane_id, turn as u8 + 1);
                let use_retained = if turn < 8 {
                    path_mask & (1 << turn) != 0
                } else {
                    turn % 2 == 1
                };
                if use_retained {
                    let _ = server.render_retained_graphics_update_and_stream();
                } else {
                    server.render_and_stream();
                }
                if drain_mask & (1 << turn) != 0 {
                    drain_alive_fast(&rx1, &mut kitty);
                }
            }
            for _ in 0..6 {
                server.render_and_stream();
                let _ = server.render_retained_graphics_update_and_stream();
                drain_alive_fast(&rx1, &mut kitty);
            }
            drain_alive_fast(&rx1, &mut kitty);

            let ledger: std::collections::HashSet<(u32, u32)> = server
                .clients
                .get(&1)
                .expect("client 1")
                .graphics_cache
                .test_placement_keys()
                .into_iter()
                .collect();
            let pane_alive: Vec<_> = kitty
                .iter()
                .filter(|((image, _), cols)| {
                    **cols > 0 && image.parse::<u64>().is_ok_and(|id| id < 0x8000_0000)
                })
                .map(|((image, placement), cols)| {
                    (
                        image.parse::<u32>().unwrap_or(0),
                        placement.parse::<u32>().unwrap_or(0),
                        *cols,
                    )
                })
                .collect();
            for (image, placement, cols) in &pane_alive {
                assert!(
                    ledger.contains(&(*image, *placement)),
                    "path_mask={path_mask:03x} drain_mask={drain_mask:03x}: the terminal \
                     still shows i={image} p={placement} c={cols} but the server ledger no \
                     longer tracks it — that placement can never be deleted again",
                );
            }
            assert!(
                pane_alive.len() <= 1,
                "path_mask={path_mask:03x} drain_mask={drain_mask:03x}: the terminal shows \
                 {} placements for one pane after convergence: {pane_alive:?}",
                pane_alive.len()
            );
        }
    }
}

fn push_tb_frame_large(server: &mut HeadlessServer, pane_id: crate::layout::PaneId, tick: u8) {
    // A production-sized fallback-browser frame: 64x96 RGBA pixels make a
    // ~32KB base64 payload that spans many kitty chunks, exactly like the
    // 4105-byte chunk trains recorded on the live wire.
    let pixels: Vec<u8> = (0..64usize * 96 * 4).map(|n| (n as u8) ^ tick).collect();
    use base64::Engine as _;
    let payload = base64::engine::general_purpose::STANDARD.encode(&pixels);
    let mut apc = String::new();
    let chunks: Vec<&str> = payload
        .as_bytes()
        .chunks(4096)
        .map(|chunk| std::str::from_utf8(chunk).expect("base64 is ascii"))
        .collect();
    for (index, chunk) in chunks.iter().enumerate() {
        let more = if index + 1 == chunks.len() { 0 } else { 1 };
        if index == 0 {
            apc.push_str(&format!(
                "\u{1b}_Ga=T,f=32,s=64,v=96,i=77,p=9,q=2,m={more};{chunk}\u{1b}\\"
            ));
        } else {
            apc.push_str(&format!("\u{1b}_Gm={more};{chunk}\u{1b}\\"));
        }
    }
    if let Some(runtime) =
        server
            .app
            .state
            .runtime_for_pane_in_workspace(&server.app.terminal_runtimes, 0, pane_id)
    {
        runtime.test_process_pty_bytes(apc.as_bytes());
    }
}

// TP-GFX-LEDGER-01
#[tokio::test]
async fn the_live_render_plan_with_a_second_display_never_strands_a_placement() {
    // The wire capture pinned every stranded placement to the wrapped
    // ServerMessage::Graphics stream, and the absence of d=i deletes proved
    // the stranded transactions were never committed to the ledger. Drive the
    // real plan flow (retained first; Fallback runs the full path in the same
    // tick, Deferred resolves into a full render next tick) with a
    // production-sized frame, an optional second display that reads slowly,
    // and focus-style full redraws — then the simulated kitty of the first
    // display and the server ledger must agree.
    let gesture: &[Option<(crate::api::schema::PaneDirection, f32)>] = &[
        None,
        None,
        None,
        Some((crate::api::schema::PaneDirection::Right, 0.20)),
        None,
        None,
        Some((crate::api::schema::PaneDirection::Left, 0.12)),
        None,
        None,
        Some((crate::api::schema::PaneDirection::Left, 0.10)),
        None,
        None,
    ];
    for second_client in [None, Some((80u16, 24u16)), Some((60u16, 20u16))] {
        for drain_mask in [0xFFFu32, 0xAAA, 0x249, 0x000] {
            for redraw_mask in [0x000u32, 0x044, 0x420] {
                for drain_second in [false, true] {
                    let (mut server, rx1, pane_id) = retained_test_server(b"video pane");
                    server.app.state.kitty_graphics_enabled = true;
                    server.clients.get_mut(&1).unwrap().cell_size =
                        crate::kitty_graphics::HostCellSize {
                            width_px: 10,
                            height_px: 20,
                        };
                    server.sync_foreground_client_state();
                    server.resize_shared_runtime_to_effective_size();
                    let neighbour = server.app.state.workspaces[0]
                        .test_split(ratatui::layout::Direction::Horizontal);
                    set_graphics_layer(&mut server, neighbour, vec![7, 7, 7, 7]);
                    let mut rx2 = None;
                    if let Some(size) = second_client {
                        let (writer, _control_rx, render_rx) = test_client_writer();
                        server.clients.insert(
                            2,
                            ClientConnection::new(
                                size,
                                crate::kitty_graphics::HostCellSize {
                                    width_px: 10,
                                    height_px: 20,
                                },
                                crate::terminal_theme::TerminalTheme::default(),
                                None,
                                2,
                                RenderEncoding::SemanticFrame,
                                Some(writer),
                            ),
                        );
                        rx2 = Some(render_rx);
                    }
                    server.render_and_stream();
                    let mut kitty = std::collections::HashMap::new();
                    drain_alive_fast(&rx1, &mut kitty);
                    let mut kitty2 = std::collections::HashMap::new();
                    if let Some(rx2) = rx2.as_ref() {
                        drain_alive_fast(rx2, &mut kitty2);
                    }

                    let area = server.app.state.view.terminal_area;
                    for (turn, step) in gesture.iter().enumerate() {
                        if let Some((direction, amount)) = step {
                            server.app.state.workspaces[0].tabs[0].layout.resize_pane(
                                pane_id,
                                (*direction).into(),
                                *amount,
                                area,
                            );
                        }
                        push_tb_frame_large(&mut server, pane_id, turn as u8 + 1);
                        if redraw_mask & (1 << turn) != 0 {
                            server.app.full_redraw_pending = true;
                        }
                        match server.render_retained_graphics_update_and_stream() {
                            RetainedGraphicsOutcome::Sent => {}
                            RetainedGraphicsOutcome::Deferred
                            | RetainedGraphicsOutcome::Fallback => {
                                server.render_and_stream();
                            }
                        }
                        if drain_mask & (1 << turn) != 0 {
                            drain_alive_fast(&rx1, &mut kitty);
                        }
                        if drain_second {
                            if let Some(rx2) = rx2.as_ref() {
                                drain_alive_fast(rx2, &mut kitty2);
                            }
                        }
                    }
                    for _ in 0..8 {
                        match server.render_retained_graphics_update_and_stream() {
                            RetainedGraphicsOutcome::Sent => {}
                            _ => server.render_and_stream(),
                        }
                        drain_alive_fast(&rx1, &mut kitty);
                        if let Some(rx2) = rx2.as_ref() {
                            drain_alive_fast(rx2, &mut kitty2);
                        }
                    }
                    drain_alive_fast(&rx1, &mut kitty);

                    let ledger: std::collections::HashSet<(u32, u32)> = server
                        .clients
                        .get(&1)
                        .expect("client 1")
                        .graphics_cache
                        .test_placement_keys()
                        .into_iter()
                        .collect();
                    let pane_alive: Vec<_> = kitty
                        .iter()
                        .filter(|((image, _), cols)| {
                            **cols > 0 && image.parse::<u64>().is_ok_and(|id| id < 0x8000_0000)
                        })
                        .map(|((image, placement), cols)| {
                            (
                                image.parse::<u32>().unwrap_or(0),
                                placement.parse::<u32>().unwrap_or(0),
                                *cols,
                            )
                        })
                        .collect();
                    let context = format!(
                        "second_client={second_client:?} drain_mask={drain_mask:03x} \
                         redraw_mask={redraw_mask:03x} drain_second={drain_second}"
                    );
                    for (image, placement, cols) in &pane_alive {
                        assert!(
                            ledger.contains(&(*image, *placement)),
                            "{context}: display 1 still shows i={image} p={placement} \
                             c={cols} but the server ledger no longer tracks it — that \
                             placement can never be deleted again",
                        );
                    }
                    assert!(
                        pane_alive.len() <= 1,
                        "{context}: display 1 shows {} placements for one pane after \
                         convergence: {pane_alive:?}",
                        pane_alive.len()
                    );
                }
            }
        }
    }
}

// TP-GFX-CONVERGE-01
#[tokio::test]
async fn narrowing_the_view_reclips_or_deletes_a_stale_pane_graphics_placement() {
    let (mut server, client_rx, pane_id) = retained_test_server(b"stale-frame");
    set_graphics_layer(&mut server, pane_id, vec![7, 7, 7, 7]);
    if let Some(layer) = server
        .app
        .pane_graphics
        .slots
        .get_mut(&graphics_key(pane_id))
        .and_then(|slot| slot.layer.as_mut())
    {
        // What tb leaves behind: the grid of the LAST frame it rendered,
        // sized for the wide pane. No new frame arrives after the narrowing.
        layer.render.grid_cols = 200;
        layer.render.grid_rows = 100;
    }
    let initial = enable_graphics_and_render(&mut server, &client_rx);
    let g0 = String::from_utf8_lossy(&initial.graphics).into_owned();
    let cols0 = first_control_field(&g0, "a=p", "c")
        .unwrap_or_else(|| panic!("first frame must place the layer: {g0:?}"));
    let image0 = first_control_field(&g0, "a=p", "i");
    let placement0 = first_control_field(&g0, "a=p", "p");

    // The visible area narrows; the layer is untouched (no new frame).
    assert!(server.handle_server_event(ServerEvent::ClientResize {
        client_id: 1,
        cols: 40,
        rows: 24,
        cell_width_px: 10,
        cell_height_px: 20,
    }));
    server.render_and_stream();
    let frame = read_server_frame(receive_render(&client_rx, Duration::from_millis(500)));
    let g1 = String::from_utf8_lossy(&frame.graphics).into_owned();
    let deleted = g1.contains("a=d");
    let cols1 = first_control_field(&g1, "a=p", "c");
    let reclipped = cols1.is_some_and(|c| c < cols0);
    assert!(
        deleted || reclipped,
        "narrowed view left the wide placement standing: cols0={cols0} cols1={cols1:?} \
         i0={image0:?} p0={placement0:?} second graphics={g1:?}"
    );
}

// K3 / BRW-2.1 multi-client repro. The product-layer lab proved it three times
// out of three: with two differently sized displays attached, a rapid
// grow-then-shrink of the pane leaves one client holding the WIDE placement
// forever (0 bytes reach it after the resizes; the screenshot's stale strip).
// The contract under test is the user's own framing: the server knows what
// every client's screen holds (its graphics_cache), so after the dust settles
// EVERY client must have been brought to the final pane geometry.
fn drain_into(rx: &std::sync::mpsc::Receiver<Vec<u8>>, history: &mut Vec<u32>) -> String {
    let mut labels = Vec::new();
    while let Ok(bytes) = rx.recv_timeout(Duration::from_millis(120)) {
        let (kind, graphics) = match read_server_message(bytes) {
            ServerMessage::Frame(frame) => (
                "Frame",
                String::from_utf8_lossy(&frame.graphics).into_owned(),
            ),
            ServerMessage::Graphics { bytes } => {
                ("Graphics", String::from_utf8_lossy(&bytes).into_owned())
            }
            other => {
                labels.push(format!("{:?}", std::mem::discriminant(&other)));
                continue;
            }
        };
        let cols = last_placement_cols(&graphics);
        if let Some(cols) = cols {
            history.push(cols);
        }
        labels.push(format!("{kind}(g={}B,last_c={cols:?})", graphics.len()));
    }
    labels.join(",")
}

fn last_placement_cols(graphics: &str) -> Option<u32> {
    let mut cols = None;
    let mut rest = graphics;
    while let Some(start) = rest.find("a=p") {
        let tail = &rest[start..];
        let end = tail.find('\x1b').unwrap_or(tail.len());
        if let Some(pos) = tail[..end].find(",c=") {
            let digits: String = tail[pos + 3..end]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(value) = digits.parse() {
                cols = Some(value);
            }
        }
        rest = &rest[start + 3..];
    }
    cols
}

// TP-GFX-CONVERGE-01
#[tokio::test]
async fn every_client_converges_to_the_final_pane_geometry_after_rapid_resizes() {
    let (mut server, rx1, pane_id) = retained_test_server(b"stale strip");
    // Second, differently sized display — the live session always has one.
    let (tx2, _control2, rx2) = test_client_writer();
    server.clients.insert(
        2,
        ClientConnection::new(
            (120, 40),
            crate::kitty_graphics::HostCellSize {
                width_px: 10,
                height_px: 20,
            },
            crate::terminal_theme::TerminalTheme::default(),
            None,
            2,
            RenderEncoding::SemanticFrame,
            Some(tx2),
        ),
    );
    server.sync_foreground_client_state();
    server.resize_shared_runtime_to_effective_size();

    set_graphics_layer(&mut server, pane_id, vec![9, 9, 9, 9]);
    if let Some(layer) = server
        .app
        .pane_graphics
        .slots
        .get_mut(&graphics_key(pane_id))
        .and_then(|slot| slot.layer.as_mut())
    {
        // The last frame tb rendered, sized for the wide pane; no new frame
        // arrives during the gesture.
        layer.render.grid_cols = 200;
        layer.render.grid_rows = 100;
    }
    server.app.state.kitty_graphics_enabled = true;
    server.clients.get_mut(&1).unwrap().cell_size = crate::kitty_graphics::HostCellSize {
        width_px: 10,
        height_px: 20,
    };
    server.render_and_stream();
    let mut history1 = Vec::new();
    let mut history2 = Vec::new();
    println!(
        "setup c1=[{}] c2=[{}]",
        drain_into(&rx1, &mut history1),
        drain_into(&rx2, &mut history2)
    );
    assert!(
        !history1.is_empty() || !history2.is_empty(),
        "setup must place the layer somewhere"
    );

    // A second pane shares the tab (the browser never lives alone in the
    // live session), and it gives resize_pane a neighbour to trade with.
    let _neighbour =
        server.app.state.workspaces[0].test_split(ratatui::layout::Direction::Horizontal);
    server.render_and_stream();
    println!(
        "split c1=[{}] c2=[{}]",
        drain_into(&rx1, &mut history1),
        drain_into(&rx2, &mut history2)
    );

    // The user's gesture: grow, then shrink, one render turn per step —
    // exactly what dragging a divider produces.
    let area = server.app.state.view.terminal_area;
    for (turn, (direction, amount)) in [
        (crate::api::schema::PaneDirection::Right, 0.20_f32),
        (crate::api::schema::PaneDirection::Left, 0.12_f32),
    ]
    .into_iter()
    .enumerate()
    {
        let moved = server.app.state.workspaces[0].tabs[0].layout.resize_pane(
            pane_id,
            direction.into(),
            amount,
            area,
        );
        assert!(moved, "resize step must change the layout");
        server.render_and_stream();
        println!(
            "turn={turn} c1=[{}] c2=[{}]",
            drain_into(&rx1, &mut history1),
            drain_into(&rx2, &mut history2)
        );
    }
    // Give every deferred path its follow-up turn.
    for follow in 0..2 {
        server.render_and_stream();
        println!(
            "follow={follow} c1=[{}] c2=[{}]",
            drain_into(&rx1, &mut history1),
            drain_into(&rx2, &mut history2)
        );
    }

    // Two behavioural invariants, measured in each client's OWN stream so no
    // cross-viewer geometry guess can lie: the stream settles (the follow-up
    // turns repeat the final placement instead of falling silent), and the
    // settled value is narrower than the widest placement ever shown (the
    // shrink actually reached this client). The defect's signature — one
    // client parked on the grow-turn width for ever — fails both.
    for (name, history) in [("1", &history1), ("2", &history2)] {
        let n = history.len();
        assert!(
            n >= 2,
            "client {name} never saw a placement update: {history:?}"
        );
        let last = history[n - 1];
        let peak = *history.iter().max().expect("nonempty history");
        assert_eq!(
            history[n - 1],
            history[n - 2],
            "client {name}'s stream never settled: {history:?}"
        );
        assert!(
            last < peak,
            "client {name} was left on its widest placement (no shrink reached it): {history:?}"
        );
    }
}
