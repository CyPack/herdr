use super::*;
use crate::app::pane_audio::Outbound;
use crate::protocol::{capability, codec, CapabilityEntry, CapabilitySet};
use crate::server::client_transport::WriteLane;

fn audio_test_server() -> (HeadlessServer, crate::layout::PaneId, String) {
    let mut server = test_headless_server();
    let workspace = crate::workspace::Workspace::test_new("audio");
    let pane_id = workspace.focused_pane_id().expect("focused pane");
    server.app.state.workspaces = vec![workspace];
    server.app.state.active = Some(0);
    server.app.state.selected = 0;
    server.app.state.ensure_test_terminals();
    let public = server
        .app
        .public_pane_id(0, pane_id)
        .expect("public pane id");
    (server, pane_id, public)
}

fn connect(
    server: &mut HeadlessServer,
    client_id: u64,
    sink: Option<&str>,
) -> crate::server::client_transport::TestQueueDrain {
    let (writer, drain) = ClientWriter::test_channel_through_queue();
    let mut client = ClientConnection::new(
        (80, 24),
        crate::kitty_graphics::HostCellSize::default(),
        crate::terminal_theme::TerminalTheme::default(),
        None,
        client_id,
        RenderEncoding::SemanticFrame,
        Some(writer),
    );
    client.capabilities = CapabilitySet::from_entries(match sink {
        Some(codec) => vec![
            CapabilityEntry::flag(capability::MEDIA_STREAMS),
            CapabilityEntry::with_values(capability::AUDIO_SINK, [codec]),
        ],
        None => Vec::new(),
    });
    server.clients.insert(client_id, client);
    drain
}

fn decode(bytes: &[u8]) -> ServerMessage {
    let mut cursor = std::io::Cursor::new(bytes);
    protocol::read_message(&mut cursor, crate::protocol::MAX_FRAME_SIZE)
        .expect("framed server message")
}

fn open_stream(server: &mut HeadlessServer, public: &str, owner: &str) {
    let response = server.app.handle_api_request(api::schema::Request {
        id: "open".into(),
        method: api::schema::Method::PaneAudioStreamOpen(api::schema::PaneAudioStreamParams {
            pane_id: public.to_owned(),
            sample_rate_hz: 48_000,
            channels: 2,
            format: "f32le".into(),
            owner: owner.to_owned(),
        }),
    });
    assert!(
        serde_json::from_str::<api::schema::SuccessResponse>(&response).is_ok(),
        "{response}"
    );
    server.flush_pane_audio_outbound();
}

fn feed_frame(server: &mut HeadlessServer, public: &str, owner: &str) {
    let response = server.app.handle_api_request(api::schema::Request {
        id: "chunk".into(),
        method: api::schema::Method::PaneAudioStreamChunk(api::schema::PaneAudioChunkParams {
            pane_id: public.to_owned(),
            owner: owner.to_owned(),
            pcm: 0.05_f32
                .to_le_bytes()
                .repeat(api::schema::PANE_AUDIO_FRAME_BYTES / 4),
        }),
    });
    assert!(
        serde_json::from_str::<api::schema::SuccessResponse>(&response).is_ok(),
        "{response}"
    );
    server.flush_pane_audio_outbound();
}

// TP-MEDIA-DISPATCH-01
#[test]
fn an_audio_request_off_the_graphics_path_still_reaches_the_fan_out() {
    // The main loop routes an API request through `handle_api_request_with_shutdown_check`
    // whenever no graphics runtime is active — which, for an audio-only session, is
    // always. That path used to skip the audio fan-out entirely: the stream opened,
    // the API answered ok, and no client was ever offered it. This drives the real
    // request path, not the app handler the other tests call directly.
    let (mut server, _pane, public) = audio_test_server();
    let opus = connect(&mut server, 1, Some(codec::OPUS));
    let (tx, _rx) = std::sync::mpsc::channel();
    server.handle_api_request_with_shutdown_check(api::ApiRequestMessage {
        request: api::schema::Request {
            id: "open".into(),
            method: api::schema::Method::PaneAudioStreamOpen(api::schema::PaneAudioStreamParams {
                pane_id: public.clone(),
                sample_rate_hz: 48_000,
                channels: 2,
                format: "f32le".into(),
                owner: "owner".into(),
            }),
        },
        respond_to: tx,
        response_write_complete: None,
        stream_active: None,
    });
    let items = opus.drain();
    assert_eq!(items.len(), 1, "the opus client is offered the stream");
    assert_eq!(
        items[0].0,
        WriteLane::Control,
        "open rides the reliable lane"
    );
    assert!(
        matches!(
            decode(&items[0].1),
            ServerMessage::MediaOpen { stream_id: 1, .. }
        ),
        "the offer is a MediaOpen for the first stream"
    );
}

// TP-MEDIA-CAP-07
#[tokio::test]
async fn a_stream_is_offered_only_to_clients_that_negotiated_an_opus_sink() {
    let (mut server, _pane, public) = audio_test_server();
    let opus = connect(&mut server, 1, Some(codec::OPUS));
    let silent = connect(&mut server, 2, None);
    let other = connect(&mut server, 3, Some(codec::PCM_S16LE));

    open_stream(&mut server, &public, "owner");

    let opus_items = opus.drain();
    assert_eq!(opus_items.len(), 1);
    assert_eq!(
        opus_items[0].0,
        WriteLane::Control,
        "open rides the reliable lane"
    );
    match decode(&opus_items[0].1) {
        ServerMessage::MediaOpen {
            stream_id,
            codec: name,
            target_latency_us,
            ..
        } => {
            assert_eq!(stream_id, 1);
            assert_eq!(name, codec::OPUS);
            assert_eq!(target_latency_us, crate::app::pane_audio::TARGET_LATENCY_US);
        }
        other => panic!("expected MediaOpen, got {other:?}"),
    }
    assert!(
        silent.drain().is_empty(),
        "a client without a sink hears nothing"
    );
    assert!(
        other.drain().is_empty(),
        "a client with another codec hears nothing"
    );
    let session = server
        .app
        .pane_audio
        .session_for_stream(1)
        .expect("session");
    assert_eq!(session.subscribers(), vec![1]);
    assert_eq!(session.counters().declined_no_sink, 2);
}

// TP-MEDIA-CREDIT-02
#[tokio::test]
async fn credit_reported_by_a_client_gates_what_it_is_sent() {
    let (mut server, _pane, public) = audio_test_server();
    let starved = connect(&mut server, 1, Some(codec::OPUS));
    let fed = connect(&mut server, 2, Some(codec::OPUS));
    open_stream(&mut server, &public, "owner");
    starved.drain();
    fed.drain();

    // Nobody has reported room: the frame is dropped at the source.
    feed_frame(&mut server, &public, "owner");
    assert!(starved.drain().is_empty());
    assert!(fed.drain().is_empty());
    let counters = server
        .app
        .pane_audio
        .session_for_stream(1)
        .unwrap()
        .counters();
    assert_eq!(counters.dropped_no_credit, 1);

    assert!(!server.handle_server_event(ServerEvent::ClientMediaCredit {
        client_id: 2,
        stream_id: 1,
        chunks: 2,
    }));
    assert!(!server.handle_server_event(ServerEvent::ClientMediaCredit {
        client_id: 1,
        stream_id: 1,
        chunks: 0,
    }));
    for _ in 0..3 {
        feed_frame(&mut server, &public, "owner");
    }
    assert!(starved.drain().is_empty(), "zero credit means zero chunks");
    let fed_items = fed.drain();
    assert_eq!(
        fed_items.len(),
        2,
        "two chunks of room, then the source drops"
    );
    assert!(fed_items.iter().all(|(lane, _)| *lane == WriteLane::Media));
    assert!(matches!(
        decode(&fed_items[0].1),
        ServerMessage::MediaChunk { seq: 1, .. }
    ));
    let counters = server
        .app
        .pane_audio
        .session_for_stream(1)
        .unwrap()
        .counters();
    assert_eq!(counters.sent, 2);
    assert_eq!(counters.dropped_no_credit, 2);

    // A departed client is forgotten; its stale room can never send to it.
    assert!(server.handle_server_event(ServerEvent::ClientDisconnected { client_id: 2 }));
    assert_eq!(
        server
            .app
            .pane_audio
            .session_for_stream(1)
            .unwrap()
            .subscribers(),
        vec![1],
        "only the departed client is forgotten"
    );
}

// TP-MEDIA-LANE-02
#[tokio::test]
async fn a_full_media_lane_drops_the_chunk_and_control_still_leaves_first() {
    let (mut server, _pane, public) = audio_test_server();
    let drain = connect(&mut server, 1, Some(codec::OPUS));
    open_stream(&mut server, &public, "owner");
    assert!(!server.handle_server_event(ServerEvent::ClientMediaCredit {
        client_id: 1,
        stream_id: 1,
        chunks: u16::MAX,
    }));

    // 64 fit the lane; the 65th is refused and counted, not queued.
    for _ in 0..65 {
        feed_frame(&mut server, &public, "owner");
    }
    let counters = server
        .app
        .pane_audio
        .session_for_stream(1)
        .unwrap()
        .counters();
    assert_eq!(counters.sent, 64);
    assert_eq!(counters.dropped_full, 1);

    let items = drain.drain();
    assert_eq!(
        items[0].0,
        WriteLane::Control,
        "the open left before any chunk"
    );
    assert_eq!(items.len(), 65);
    assert!(items[1..].iter().all(|(lane, _)| *lane == WriteLane::Media));

    let _ = Outbound::Open {
        stream_id: 0,
        pane_id: String::new(),
    };
}
