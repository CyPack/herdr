//! `pane.audio.stream`: one dedicated socket feeds one pane's audio.
//!
//! Sibling of [`super::pane_graphics_stream`] and shaped by it on purpose: the
//! request line opens the stream, the app answers before the ack, the owner is
//! registered before the ack so a cancel can always reach it, and the same
//! idle/absolute read deadlines apply. Two things differ, and both come from
//! the codec rather than from taste:
//!
//! * There is no per-frame header. The body is a sequence of whole frames,
//!   every one exactly [`PANE_AUDIO_FRAME_BYTES`] long — 960 samples × 2
//!   channels × 4 bytes of little-endian f32. A header would only repeat what
//!   the shape already fixes.
//! * A partial frame is a failure, never padded or truncated. Twenty
//!   milliseconds of audio that is quietly shortened drifts the stream by that
//!   much on every frame: inaudible once, obvious after a minute, impossible to
//!   trace back afterwards.
//!
//! Nothing here names an audio platform. The producer decides where the
//! samples come from; the protocol only ever sees frames.

use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

use crate::api::schema::{
    ErrorBody, ErrorResponse, Method, PaneAudioChunkParams, PaneAudioStreamCloseParams,
    PaneAudioStreamParams, Request, ResponseResult, SuccessResponse, PANE_AUDIO_CHANNELS,
    PANE_AUDIO_FORMAT, PANE_AUDIO_FRAME_BYTES, PANE_AUDIO_SAMPLE_RATE_HZ,
};
use crate::api::ApiRequestSender;
use crate::ipc::{is_connection_closed_error, LocalStream};

use super::pane_graphics_stream::{read_exact, stream_is_running, ReadTimeouts, READ_TIMEOUTS};
use super::{
    api_response_outcome, dispatch_stream_open, dispatch_to_app_with_timeout, write_json_line,
    write_json_line_allow_disconnect, write_text_line_allow_disconnect, APP_RESPONSE_TIMEOUT,
};

static NEXT_PANE_AUDIO_STREAM_OWNER: AtomicU64 = AtomicU64::new(1);
static REGISTERED_STREAM_COUNT: AtomicUsize = AtomicUsize::new(0);
static REGISTERED_STREAMS: OnceLock<Mutex<HashMap<String, Weak<AtomicBool>>>> = OnceLock::new();

fn stream_registry() -> &'static Mutex<HashMap<String, Weak<AtomicBool>>> {
    REGISTERED_STREAMS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn register_stream(owner: &str, active: &Arc<AtomicBool>) {
    let Ok(mut streams) = stream_registry().lock() else {
        active.store(false, Ordering::Release);
        return;
    };
    if streams
        .insert(owner.to_string(), Arc::downgrade(active))
        .is_none()
    {
        REGISTERED_STREAM_COUNT.fetch_add(1, Ordering::Release);
    }
}

fn unregister_stream(owner: &str) {
    let Ok(mut streams) = stream_registry().lock() else {
        return;
    };
    if streams.remove(owner).is_some() {
        REGISTERED_STREAM_COUNT.fetch_sub(1, Ordering::Release);
    }
}

/// Stops every registered audio stream whose owner the app no longer knows.
///
/// The app is the single source of truth: a session that vanished (pane
/// closed, stream superseded) leaves a reader blocked on its socket, and this
/// sweep is what makes that reader return. Costs nothing while no stream is
/// registered.
pub(crate) fn cancel_inactive_streams(mut is_active: impl FnMut(&str) -> bool) {
    if REGISTERED_STREAM_COUNT.load(Ordering::Acquire) == 0 {
        return;
    }
    let Ok(mut streams) = stream_registry().lock() else {
        return;
    };
    let before = streams.len();
    streams.retain(|owner, active| {
        let keep = is_active(owner);
        if !keep {
            if let Some(active) = active.upgrade() {
                active.store(false, Ordering::Release);
            }
        }
        keep
    });
    REGISTERED_STREAM_COUNT.fetch_sub(before.saturating_sub(streams.len()), Ordering::Release);
}

/// Why a stream's frame loop stopped.
#[derive(Debug, PartialEq, Eq)]
enum StreamEnd {
    /// The producer closed its socket on a frame boundary.
    Ended,
    /// Something refused a frame, or the socket ended inside one.
    Failed(String),
}

/// The identity a frame loop carries to every dispatch and error line.
struct StreamIdentity<'a> {
    request_id: &'a str,
    owner: &'a str,
    pane_id: &'a str,
}

/// Names the parameter the encoder cannot take, or `None` when the shape is
/// exactly the one every audio stream uses.
///
/// Checked before the app is asked, so a producer with the wrong shape is told
/// before it has sent a single frame — after the ack it would already be
/// streaming into a stream that will never open.
pub(crate) fn shape_error(params: &PaneAudioStreamParams) -> Option<String> {
    if params.sample_rate_hz != PANE_AUDIO_SAMPLE_RATE_HZ {
        return Some(format!(
            "sample_rate_hz must be {PANE_AUDIO_SAMPLE_RATE_HZ}, got {}",
            params.sample_rate_hz
        ));
    }
    if params.channels != PANE_AUDIO_CHANNELS {
        return Some(format!(
            "channels must be {PANE_AUDIO_CHANNELS}, got {}",
            params.channels
        ));
    }
    if params.format != PANE_AUDIO_FORMAT {
        return Some(format!(
            "format must be {PANE_AUDIO_FORMAT:?}, got {:?}",
            params.format
        ));
    }
    None
}

pub(super) fn serve(
    stream: LocalStream,
    request_id: String,
    params: PaneAudioStreamParams,
    api_tx: &ApiRequestSender,
    running: &Arc<AtomicBool>,
) -> io::Result<()> {
    serve_with_timeouts(
        stream,
        request_id,
        params,
        api_tx,
        running,
        APP_RESPONSE_TIMEOUT,
        READ_TIMEOUTS,
    )
}

fn serve_with_timeouts(
    mut stream: LocalStream,
    request_id: String,
    mut params: PaneAudioStreamParams,
    api_tx: &ApiRequestSender,
    running: &Arc<AtomicBool>,
    open_timeout: Duration,
    read_timeouts: ReadTimeouts,
) -> io::Result<()> {
    if let Some(message) = shape_error(&params) {
        write_json_line_allow_disconnect(
            &mut stream,
            &ErrorResponse {
                id: request_id,
                error: ErrorBody {
                    code: "invalid_params".into(),
                    message,
                },
            },
        )?;
        return Ok(());
    }

    let pane_id = params.pane_id.clone();
    let owner = next_owner();
    params.owner = owner.clone();
    let open_active = Arc::new(AtomicBool::new(true));
    let open_response = dispatch_stream_open(
        Request {
            id: request_id.clone(),
            method: Method::PaneAudioStreamOpen(params),
        },
        api_tx,
        open_timeout,
        Arc::clone(&open_active),
    );
    if api_response_outcome(&open_response) != "ok" {
        open_active.store(false, Ordering::Release);
        let write_result = write_text_line_allow_disconnect(&mut stream, &open_response);
        close_stream(&pane_id, &owner, true, "open refused", api_tx);
        write_result?;
        return Ok(());
    }

    // Register before acknowledging, for the same reason the graphics stream
    // does: once the producer has seen the ack, a cancel must be able to reach
    // this stream, otherwise it runs on until an unrelated timeout.
    let stream_active = Arc::new(AtomicBool::new(true));
    register_stream(&owner, &stream_active);

    if let Err(err) = write_json_line(
        &mut stream,
        &SuccessResponse {
            id: request_id.clone(),
            result: ResponseResult::Ok {},
        },
    ) {
        unregister_stream(&owner);
        stream_active.store(false, Ordering::Release);
        close_stream(
            &pane_id,
            &owner,
            true,
            "producer left before the ack",
            api_tx,
        );
        if is_connection_closed_error(&err) {
            return Ok(());
        }
        return Err(err);
    }

    let identity = StreamIdentity {
        request_id: &request_id,
        owner: &owner,
        pane_id: &pane_id,
    };
    let result = serve_frames(
        &mut stream,
        &identity,
        api_tx,
        running,
        &stream_active,
        read_timeouts,
    );
    stream_active.store(false, Ordering::Release);
    unregister_stream(&owner);
    match result {
        Ok(StreamEnd::Ended) => {
            close_stream(&pane_id, &owner, false, "", api_tx);
            Ok(())
        }
        Ok(StreamEnd::Failed(detail)) => {
            close_stream(&pane_id, &owner, true, &detail, api_tx);
            Ok(())
        }
        Err(err) => {
            close_stream(&pane_id, &owner, true, &err.to_string(), api_tx);
            Err(err)
        }
    }
}

/// Reads whole frames until the producer stops, refusing the first frame that
/// is not whole.
///
/// The body carries no headers, so the only framing is the frame length
/// itself. A socket that ends on a frame boundary is a producer that finished;
/// a socket that ends inside a frame is a producer that broke, and the stream
/// is closed as failed so the listener learns the difference.
fn serve_frames(
    stream: &mut LocalStream,
    identity: &StreamIdentity<'_>,
    api_tx: &ApiRequestSender,
    running: &Arc<AtomicBool>,
    stream_active: &Arc<AtomicBool>,
    timeouts: ReadTimeouts,
) -> io::Result<StreamEnd> {
    let mut frame_seq = 0_u64;
    while stream_is_running(running, stream_active) {
        let pcm = match read_exact(
            stream,
            PANE_AUDIO_FRAME_BYTES,
            running,
            stream_active,
            timeouts.body_idle,
            timeouts.body_total,
        ) {
            Ok(Some(pcm)) => pcm,
            Ok(None) => return Ok(StreamEnd::Ended),
            Err(err)
                if err.kind() == io::ErrorKind::UnexpectedEof
                    || is_connection_closed_error(&err) =>
            {
                let detail = format!(
                    "frame {} ended short; every frame is {PANE_AUDIO_FRAME_BYTES} bytes of f32le samples",
                    frame_seq + 1
                );
                write_json_line_allow_disconnect(
                    stream,
                    &ErrorResponse {
                        id: identity.request_id.to_string(),
                        error: ErrorBody {
                            code: "invalid_frame".into(),
                            message: detail.clone(),
                        },
                    },
                )?;
                return Ok(StreamEnd::Failed(detail));
            }
            Err(err) => return Err(err),
        };

        frame_seq = frame_seq.saturating_add(1);
        let response = dispatch_to_app_with_timeout(
            Request {
                id: format!("{}:frame:{frame_seq}", identity.request_id),
                method: Method::PaneAudioStreamChunk(PaneAudioChunkParams {
                    pane_id: identity.pane_id.to_owned(),
                    owner: identity.owner.to_owned(),
                    pcm,
                }),
            },
            api_tx,
            Some(APP_RESPONSE_TIMEOUT),
        );
        if api_response_outcome(&response) != "ok" {
            write_text_line_allow_disconnect(stream, &response)?;
            return Ok(StreamEnd::Failed(format!("frame {frame_seq} refused")));
        }
    }

    Ok(StreamEnd::Ended)
}

fn next_owner() -> String {
    let id = NEXT_PANE_AUDIO_STREAM_OWNER.fetch_add(1, Ordering::Relaxed);
    format!("pane.audio.stream:{}:{id}", std::process::id())
}

fn close_stream(pane_id: &str, owner: &str, failed: bool, detail: &str, api_tx: &ApiRequestSender) {
    let _response = dispatch_to_app_with_timeout(
        Request {
            id: format!("pane.audio.stream.close:{pane_id}"),
            method: Method::PaneAudioStreamClose(PaneAudioStreamCloseParams {
                pane_id: pane_id.to_string(),
                owner: owner.to_string(),
                failed,
                detail: detail.to_string(),
            }),
        },
        api_tx,
        Some(APP_RESPONSE_TIMEOUT),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::{ErrorResponse, Method, ResponseResult, SuccessResponse};
    use crate::api::ApiRequestMessage;
    #[cfg(unix)]
    use crate::api::EventHub;
    use crate::ipc::LocalStream;
    use interprocess::local_socket::traits::Listener as _;
    use std::io::{BufRead, BufReader, Write};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};
    use tokio::sync::mpsc;

    /// Hang guard for a dispatch that normally arrives well inside one
    /// `CONNECTION_POLL_INTERVAL`; deliberately far larger than the latency
    /// so an oversubscribed suite reports a hang, not a slow scheduler.
    const DISPATCH_HANG_GUARD: Duration = Duration::from_secs(10);

    static NEXT_LOCAL_STREAM_ID: AtomicU64 = AtomicU64::new(1);

    fn local_stream_pair() -> (LocalStream, LocalStream, PathBuf) {
        let unique = format!(
            "hpa-{}-{}.sock",
            std::process::id(),
            NEXT_LOCAL_STREAM_ID.fetch_add(1, Ordering::Relaxed)
        );
        let path = std::env::temp_dir().join(unique);
        let listener = crate::ipc::bind_local_listener(&path).unwrap();
        let client = crate::ipc::connect_local_stream(&path).unwrap();
        let server = listener.accept().unwrap();
        (client, server, path)
    }

    fn read_response_line(stream: &mut LocalStream) -> String {
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        line
    }

    fn respond_ok(message: ApiRequestMessage) {
        let response = serde_json::to_string(&SuccessResponse {
            id: message.request.id,
            result: ResponseResult::Ok {},
        })
        .unwrap();
        message.respond_to.send(response).unwrap();
    }

    fn params(pane_id: &str) -> PaneAudioStreamParams {
        PaneAudioStreamParams {
            pane_id: pane_id.into(),
            sample_rate_hz: PANE_AUDIO_SAMPLE_RATE_HZ,
            channels: PANE_AUDIO_CHANNELS,
            format: PANE_AUDIO_FORMAT.into(),
            owner: String::new(),
        }
    }

    /// Pulls the open request off the app channel and answers it, returning
    /// the server-assigned owner.
    fn accept_open(api_rx: &mut mpsc::UnboundedReceiver<ApiRequestMessage>) -> String {
        let open = api_rx.blocking_recv().unwrap();
        let owner = match &open.request.method {
            Method::PaneAudioStreamOpen(params) => {
                assert_eq!(params.pane_id, "pane_1");
                assert!(params.owner.starts_with("pane.audio.stream:"));
                assert_eq!(params.sample_rate_hz, PANE_AUDIO_SAMPLE_RATE_HZ);
                assert_eq!(params.channels, PANE_AUDIO_CHANNELS);
                assert_eq!(params.format, PANE_AUDIO_FORMAT);
                params.owner.clone()
            }
            other => panic!("unexpected open request: {other:?}"),
        };
        respond_ok(open);
        owner
    }

    fn assert_close(message: ApiRequestMessage, owner: &str, failed: bool) -> String {
        let detail = match &message.request.method {
            Method::PaneAudioStreamClose(params) => {
                assert_eq!(params.pane_id, "pane_1");
                assert_eq!(params.owner, owner);
                assert_eq!(params.failed, failed, "close failed flag");
                params.detail.clone()
            }
            other => panic!("unexpected close request: {other:?}"),
        };
        respond_ok(message);
        detail
    }

    fn frame_filled_with(byte: u8) -> Vec<u8> {
        vec![byte; PANE_AUDIO_FRAME_BYTES]
    }

    // TP-MEDIA-API-01
    #[cfg(unix)]
    #[test]
    fn pane_audio_stream_dispatches_whole_frames_and_closes_as_ended() {
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        let (mut client, server, _path) = local_stream_pair();
        client
            .write_all(
                br#"{"id":"audio_1","method":"pane.audio.stream","params":{"pane_id":"pane_1"}}"#,
            )
            .unwrap();
        client.write_all(b"\n").unwrap();
        client.flush().unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);
        let event_hub = EventHub::default();
        let server_thread = std::thread::spawn(move || {
            super::super::handle_connection(server, &api_tx, &event_hub, &server_running, None)
        });

        let owner = accept_open(&mut api_rx);
        let ack: SuccessResponse = serde_json::from_str(&read_response_line(&mut client)).unwrap();
        assert_eq!(ack.id, "audio_1");
        assert_eq!(ack.result, ResponseResult::Ok {});

        for byte in 1..=3_u8 {
            client.write_all(&frame_filled_with(byte)).unwrap();
            client.flush().unwrap();
            let msg = api_rx.blocking_recv().unwrap();
            match &msg.request.method {
                Method::PaneAudioStreamChunk(params) => {
                    assert_eq!(params.pane_id, "pane_1");
                    assert_eq!(params.owner, owner);
                    assert_eq!(params.pcm.len(), PANE_AUDIO_FRAME_BYTES);
                    assert!(params.pcm.iter().all(|value| *value == byte));
                }
                other => panic!("unexpected request: {other:?}"),
            }
            assert_eq!(msg.request.id, format!("audio_1:frame:{byte}"));
            respond_ok(msg);
        }

        drop(client);
        running.store(false, Ordering::Relaxed);
        let detail = assert_close(api_rx.blocking_recv().unwrap(), &owner, false);
        assert!(detail.is_empty());
        assert!(server_thread.join().unwrap().is_ok());
    }

    // TP-MEDIA-API-01
    #[cfg(unix)]
    #[test]
    fn a_partial_first_frame_never_reaches_the_app_and_fails_the_stream() {
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        let (mut client, server, _path) = local_stream_pair();
        client
            .write_all(br#"{"id":"audio_short","method":"pane.audio.stream","params":{"pane_id":"pane_1"}}"#)
            .unwrap();
        client.write_all(b"\n").unwrap();
        client.flush().unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);
        let event_hub = EventHub::default();
        let server_thread = std::thread::spawn(move || {
            super::super::handle_connection(server, &api_tx, &event_hub, &server_running, None)
        });

        let owner = accept_open(&mut api_rx);
        let _ack = read_response_line(&mut client);

        // One byte short of a frame, then gone.
        client
            .write_all(&vec![7_u8; PANE_AUDIO_FRAME_BYTES - 1])
            .unwrap();
        client.flush().unwrap();
        drop(client);

        let detail = assert_close(api_rx.blocking_recv().unwrap(), &owner, true);
        assert!(detail.contains("short"), "detail: {detail}");
        running.store(false, Ordering::Relaxed);
        assert!(server_thread.join().unwrap().is_ok());
    }

    // TP-MEDIA-API-01
    #[cfg(unix)]
    #[test]
    fn the_whole_frames_before_a_short_one_are_delivered_and_the_short_one_fails_the_stream() {
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        let (mut client, server, _path) = local_stream_pair();
        client
            .write_all(br#"{"id":"audio_tail","method":"pane.audio.stream","params":{"pane_id":"pane_1"}}"#)
            .unwrap();
        client.write_all(b"\n").unwrap();
        client.flush().unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);
        let event_hub = EventHub::default();
        let server_thread = std::thread::spawn(move || {
            super::super::handle_connection(server, &api_tx, &event_hub, &server_running, None)
        });

        let owner = accept_open(&mut api_rx);
        let _ack = read_response_line(&mut client);

        // One whole frame plus a single stray byte.
        let mut body = frame_filled_with(9);
        body.push(1);
        client.write_all(&body).unwrap();
        client.flush().unwrap();

        let chunk = api_rx.blocking_recv().unwrap();
        assert!(matches!(
            &chunk.request.method,
            Method::PaneAudioStreamChunk(params) if params.pcm.len() == PANE_AUDIO_FRAME_BYTES
        ));
        respond_ok(chunk);
        drop(client);

        let detail = assert_close(api_rx.blocking_recv().unwrap(), &owner, true);
        assert!(detail.contains("frame 2"), "detail: {detail}");
        running.store(false, Ordering::Relaxed);
        assert!(server_thread.join().unwrap().is_ok());
    }

    // TP-MEDIA-API-01
    #[cfg(unix)]
    #[test]
    fn a_stream_with_the_wrong_shape_is_refused_before_the_app_is_asked() {
        let cases = [
            r#"{"id":"rate","method":"pane.audio.stream","params":{"pane_id":"pane_1","sample_rate_hz":44100}}"#,
            r#"{"id":"channels","method":"pane.audio.stream","params":{"pane_id":"pane_1","channels":1}}"#,
            r#"{"id":"format","method":"pane.audio.stream","params":{"pane_id":"pane_1","format":"s16le"}}"#,
        ];
        for request in cases {
            let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
            let (mut client, server, _path) = local_stream_pair();
            client.write_all(request.as_bytes()).unwrap();
            client.write_all(b"\n").unwrap();
            client.flush().unwrap();

            let running = Arc::new(AtomicBool::new(true));
            let server_running = Arc::clone(&running);
            let event_hub = EventHub::default();
            let server_thread = std::thread::spawn(move || {
                super::super::handle_connection(server, &api_tx, &event_hub, &server_running, None)
            });

            let response: ErrorResponse =
                serde_json::from_str(&read_response_line(&mut client)).unwrap();
            assert_eq!(response.error.code, "invalid_params", "{request}");
            drop(client);
            running.store(false, Ordering::Relaxed);
            assert!(server_thread.join().unwrap().is_ok());
            assert!(
                api_rx.try_recv().is_err(),
                "the app must not be asked for a stream it cannot encode: {request}"
            );
        }
    }

    // TP-MEDIA-API-01
    #[test]
    fn shape_error_names_the_field_that_is_wrong() {
        assert_eq!(shape_error(&params("pane_1")), None);
        let mut wrong = params("pane_1");
        wrong.sample_rate_hz = 44_100;
        assert!(shape_error(&wrong).unwrap().contains("sample_rate_hz"));
        let mut wrong = params("pane_1");
        wrong.channels = 1;
        assert!(shape_error(&wrong).unwrap().contains("channels"));
        let mut wrong = params("pane_1");
        wrong.format = "s16le".into();
        assert!(shape_error(&wrong).unwrap().contains("format"));
    }

    // TP-MEDIA-API-01
    #[cfg(unix)]
    #[test]
    fn pane_audio_stream_reports_open_errors_before_ack() {
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        let (mut client, server, _path) = local_stream_pair();
        client
            .write_all(
                br#"{"id":"audio_2","method":"pane.audio.stream","params":{"pane_id":"pane_1"}}"#,
            )
            .unwrap();
        client.write_all(b"\n").unwrap();
        client.flush().unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);
        let event_hub = EventHub::default();
        let server_thread = std::thread::spawn(move || {
            super::super::handle_connection(server, &api_tx, &event_hub, &server_running, None)
        });

        let open = api_rx.blocking_recv().unwrap();
        let owner = match &open.request.method {
            Method::PaneAudioStreamOpen(params) => params.owner.clone(),
            other => panic!("unexpected open request: {other:?}"),
        };
        open.respond_to
            .send(super::super::error_response_json(
                open.request.id,
                "pane_not_found",
                "pane pane_1 not found".into(),
            ))
            .unwrap();

        let response: ErrorResponse =
            serde_json::from_str(&read_response_line(&mut client)).unwrap();
        assert_eq!(response.id, "audio_2");
        assert_eq!(response.error.code, "pane_not_found");

        assert_close(api_rx.blocking_recv().unwrap(), &owner, true);
        drop(client);
        running.store(false, Ordering::Relaxed);
        assert!(server_thread.join().unwrap().is_ok());
    }

    // TP-MEDIA-API-01
    #[cfg(unix)]
    #[test]
    fn pane_audio_stream_closes_claim_after_open_timeout() {
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        let (mut client, server, _path) = local_stream_pair();
        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);
        let server_thread = std::thread::spawn(move || {
            serve_with_timeouts(
                server,
                "audio_timeout".into(),
                params("pane_1"),
                &api_tx,
                &server_running,
                Duration::from_millis(10),
                READ_TIMEOUTS,
            )
        });

        let open = api_rx.blocking_recv().unwrap();
        let owner = match &open.request.method {
            Method::PaneAudioStreamOpen(params) => params.owner.clone(),
            other => panic!("unexpected open request: {other:?}"),
        };

        let response: ErrorResponse =
            serde_json::from_str(&read_response_line(&mut client)).unwrap();
        assert_eq!(response.id, "audio_timeout");
        assert_eq!(response.error.code, "server_unavailable");
        assert!(response.error.message.contains("timed out"));

        assert_close(api_rx.blocking_recv().unwrap(), &owner, true);
        drop(open);
        drop(client);
        running.store(false, Ordering::Relaxed);
        assert!(server_thread.join().unwrap().is_ok());
    }

    // TP-MEDIA-API-01
    #[cfg(unix)]
    #[test]
    fn pane_audio_stream_closes_claim_when_client_disconnects_before_ack() {
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        let (mut client, server, _path) = local_stream_pair();
        client
            .write_all(
                br#"{"id":"audio_3","method":"pane.audio.stream","params":{"pane_id":"pane_1"}}"#,
            )
            .unwrap();
        client.write_all(b"\n").unwrap();
        client.flush().unwrap();

        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);
        let event_hub = EventHub::default();
        let server_thread = std::thread::spawn(move || {
            super::super::handle_connection(server, &api_tx, &event_hub, &server_running, None)
        });

        let open = api_rx.blocking_recv().unwrap();
        let owner = match &open.request.method {
            Method::PaneAudioStreamOpen(params) => params.owner.clone(),
            other => panic!("unexpected open request: {other:?}"),
        };
        drop(client);
        respond_ok(open);

        let (close_tx, close_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            close_tx.send(api_rx.blocking_recv()).unwrap();
        });
        let close = close_rx.recv_timeout(DISPATCH_HANG_GUARD).unwrap().unwrap();
        assert_close(close, &owner, true);

        running.store(false, Ordering::Relaxed);
        assert!(server_thread.join().unwrap().is_ok());
    }

    // TP-MEDIA-API-01
    #[test]
    fn inactive_owner_cancels_idle_audio_stream_and_dispatches_close() {
        let (mut client, server, _path) = local_stream_pair();
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        let running = Arc::new(AtomicBool::new(true));
        let server_running = Arc::clone(&running);
        let server_thread = std::thread::spawn(move || {
            serve_with_timeouts(
                server,
                "audio-cancel".into(),
                params("pane_1"),
                &api_tx,
                &server_running,
                Duration::from_secs(1),
                READ_TIMEOUTS,
            )
        });

        let owner = accept_open(&mut api_rx);
        let ack: SuccessResponse = serde_json::from_str(&read_response_line(&mut client)).unwrap();
        assert_eq!(ack.id, "audio-cancel");

        cancel_inactive_streams(|registered| registered != owner);

        let (close_tx, close_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || close_tx.send(api_rx.blocking_recv()).unwrap());
        let close = close_rx
            .recv_timeout(DISPATCH_HANG_GUARD)
            .expect("cancelled idle audio stream should dispatch a close")
            .expect("API request channel should remain open");
        // Cancelled by the sweep: the app already forgot the session, so the
        // reader reports a normal end rather than a failure of its own.
        assert_close(close, &owner, false);

        drop(client);
        running.store(false, Ordering::Relaxed);
        assert!(server_thread.join().unwrap().is_ok());
    }

    // TP-MEDIA-API-01
    #[test]
    fn trickled_audio_frame_obeys_absolute_deadline() {
        let (mut client, mut server, _path) = local_stream_pair();
        let (api_tx, mut api_rx) = mpsc::unbounded_channel::<ApiRequestMessage>();
        client.write_all(&[1_u8]).unwrap();
        client.flush().unwrap();
        let running = Arc::new(AtomicBool::new(true));
        let active = Arc::new(AtomicBool::new(true));
        let writer_running = Arc::clone(&running);
        let writer = std::thread::spawn(move || {
            while writer_running.load(Ordering::Relaxed) {
                if client.write_all(&[1_u8]).is_err() {
                    break;
                }
                let _ = client.flush();
                std::thread::sleep(Duration::from_millis(5));
            }
        });

        let identity = StreamIdentity {
            request_id: "audio-trickle",
            owner: "owner-1",
            pane_id: "pane_1",
        };
        let started = Instant::now();
        let error = serve_frames(
            &mut server,
            &identity,
            &api_tx,
            &running,
            &active,
            ReadTimeouts {
                header_idle: Duration::from_millis(20),
                header_total: Duration::from_millis(60),
                body_idle: Duration::from_millis(20),
                body_total: Duration::from_millis(60),
            },
        )
        .unwrap_err();

        running.store(false, Ordering::Relaxed);
        writer.join().unwrap();
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(started.elapsed() >= Duration::from_millis(50));
        assert!(started.elapsed() < Duration::from_millis(500));
        assert!(
            api_rx.try_recv().is_err(),
            "a frame that never completed must not reach the app"
        );
    }

    // TP-MEDIA-API-02
    #[test]
    fn the_audio_stream_surface_names_no_audio_platform() {
        // The words are assembled at run time so this file does not itself
        // fail the check it performs.
        let forbidden = [
            ["pipe", "wire"].concat(),
            ["al", "sa"].concat(),
            ["core", "audio"].concat(),
            ["cp", "al"].concat(),
            ["pulse", "audio"].concat(),
            ["was", "api"].concat(),
        ];
        let surfaces = [
            (
                "src/protocol/wire.rs",
                include_str!("../../protocol/wire.rs"),
            ),
            (
                "src/api/schema/panes.rs",
                include_str!("../schema/panes.rs"),
            ),
            ("src/api/schema.rs", include_str!("../schema.rs")),
            (
                "src/api/server/pane_audio_stream.rs",
                include_str!("pane_audio_stream.rs"),
            ),
        ];
        for (name, text) in surfaces {
            let code = text
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .map(str::to_ascii_lowercase)
                .collect::<Vec<_>>()
                .join("\n");
            for word in &forbidden {
                assert!(
                    !code.contains(word.as_str()),
                    "{name} names an audio platform ({word}); the protocol must stay platform-blind"
                );
            }
        }
    }
}
