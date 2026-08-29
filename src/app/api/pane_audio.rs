use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use super::responses::{encode_error, encode_success};
use crate::api::schema::{
    PaneAudioChunkParams, PaneAudioStreamCloseParams, PaneAudioStreamParams, ResponseResult,
    SuccessResponse, PANE_AUDIO_FRAME_BYTES,
};
use crate::app::pane_audio::{pcm_from_f32le, OfferError, OpenError};
use crate::app::App;
use crate::protocol::MediaCloseReason;

impl App {
    pub(super) fn handle_pane_audio_stream_open(
        &mut self,
        id: String,
        params: PaneAudioStreamParams,
    ) -> String {
        if params.owner.is_empty() {
            return encode_error(id, "invalid_stream", "pane audio stream owner is required");
        }
        let Some((_, pane)) = self.parse_pane_id(&params.pane_id) else {
            return pane_not_found(id, &params.pane_id);
        };
        match self
            .pane_audio
            .open(pane, &params.pane_id, &params.owner, crate::media::now_us())
        {
            Ok(_) => encode_success(id, ResponseResult::Ok {}),
            Err(OpenError::Conflict) => encode_error(
                id,
                "stream_conflict",
                "pane already has an active audio stream",
            ),
            Err(OpenError::Codec(err)) => encode_error(id, "codec_unavailable", err.to_string()),
        }
    }

    pub(super) fn handle_pane_audio_stream_chunk(
        &mut self,
        id: String,
        params: PaneAudioChunkParams,
    ) -> String {
        let Some((_, pane)) = self.parse_pane_id(&params.pane_id) else {
            return pane_not_found(id, &params.pane_id);
        };
        let pcm = match pcm_from_f32le(&params.pcm) {
            Ok(pcm) => pcm,
            Err(err) => {
                return encode_error(
                    id,
                    "invalid_frame",
                    format!(
                        "audio frame must be {PANE_AUDIO_FRAME_BYTES} bytes of f32le samples, got {}",
                        err.got
                    ),
                );
            }
        };
        match self
            .pane_audio
            .offer(pane, &params.owner, &pcm, crate::media::now_us())
        {
            Ok(_) => encode_success(id, ResponseResult::Ok {}),
            Err(OfferError::NotOpen) => {
                encode_error(id, "stream_closed", "pane audio stream is not active")
            }
            Err(OfferError::OwnerMismatch) => encode_error(
                id,
                "stream_conflict",
                "pane audio stream owner does not match active stream",
            ),
            Err(OfferError::Frame(err)) => encode_error(id, "invalid_frame", err.to_string()),
        }
    }

    pub(super) fn handle_pane_audio_stream_close(
        &mut self,
        id: String,
        params: PaneAudioStreamCloseParams,
    ) -> String {
        let Some((_, pane)) = self.parse_pane_id(&params.pane_id) else {
            return pane_not_found(id, &params.pane_id);
        };
        let reason = if params.failed {
            MediaCloseReason::Failed
        } else {
            MediaCloseReason::Ended
        };
        self.pane_audio
            .close(pane, &params.owner, reason, params.detail);
        encode_success(id, ResponseResult::Ok {})
    }

    pub(crate) fn attach_pane_audio_stream_active(
        &mut self,
        params: &PaneAudioStreamParams,
        active: Arc<AtomicBool>,
        response: &str,
    ) {
        if serde_json::from_str::<SuccessResponse>(response).is_err() {
            return;
        }
        let Some((_, pane)) = self.parse_pane_id(&params.pane_id) else {
            return;
        };
        self.pane_audio.attach_active(pane, &params.owner, active);
    }
}

fn pane_not_found(id: String, pane_id: &str) -> String {
    encode_error(id, "pane_not_found", format!("pane {pane_id} not found"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::schema::ErrorResponse;
    use crate::app::pane_audio::Outbound;
    use crate::config::Config;
    use crate::workspace::Workspace;

    fn app() -> (App, String) {
        let (_tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = App::new(
            &Config::default(),
            true,
            None,
            rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![Workspace::test_new("audio")];
        app.state.ensure_test_terminals();
        let pane = app.state.workspaces[0].tabs[0].root_pane;
        let public = app.public_pane_id(0, pane).unwrap();
        (app, public)
    }

    fn params(pane_id: &str, owner: &str) -> PaneAudioStreamParams {
        PaneAudioStreamParams {
            pane_id: pane_id.into(),
            sample_rate_hz: 48_000,
            channels: 2,
            format: "f32le".into(),
            owner: owner.into(),
        }
    }

    fn frame_bytes(value: f32) -> Vec<u8> {
        value.to_le_bytes().repeat(PANE_AUDIO_FRAME_BYTES / 4)
    }

    fn chunk(pane_id: &str, owner: &str, pcm: Vec<u8>) -> PaneAudioChunkParams {
        PaneAudioChunkParams {
            pane_id: pane_id.into(),
            owner: owner.into(),
            pcm,
        }
    }

    fn error_code(response: &str) -> String {
        serde_json::from_str::<ErrorResponse>(response)
            .unwrap()
            .error
            .code
    }

    fn is_ok(response: &str) -> bool {
        serde_json::from_str::<SuccessResponse>(response).is_ok()
    }

    // TP-MEDIA-PTS-01
    #[test]
    fn the_first_chunk_presents_at_the_moment_the_stream_opened() {
        let (mut app, pane_id) = app();
        let before = crate::media::now_us();
        assert!(is_ok(&app.handle_pane_audio_stream_open(
            "open".into(),
            params(&pane_id, "owner")
        )));
        let after = crate::media::now_us();
        assert!(matches!(
            app.pane_audio.take_outbound()[..],
            [Outbound::Open { stream_id: 1, .. }]
        ));
        // One client with room, so the frame is actually encoded.
        app.pane_audio.subscribe(1, 7);
        app.pane_audio.set_client_credit(1, 7, 4);

        let response = app.handle_pane_audio_stream_chunk(
            "f1".into(),
            chunk(&pane_id, "owner", frame_bytes(0.1)),
        );
        assert!(is_ok(&response), "{response}");
        match &app.pane_audio.take_outbound()[..] {
            [Outbound::Chunk { seq: 0, pts_us, .. }] => {
                assert!(
                    (before..=after).contains(pts_us),
                    "pts {pts_us} is not the open moment {before}..={after}"
                );
                assert_ne!(*pts_us, 0, "a zero pts is expired on arrival at the client");
            }
            other => panic!("expected one chunk, got {other:?}"),
        }
    }

    // TP-MEDIA-API-03
    #[test]
    fn a_frame_of_the_wrong_length_is_refused_and_feeds_nothing() {
        let (mut app, pane_id) = app();
        assert!(is_ok(&app.handle_pane_audio_stream_open(
            "open".into(),
            params(&pane_id, "owner")
        )));
        app.pane_audio.subscribe(1, 7);
        app.pane_audio.set_client_credit(1, 7, 4);
        app.pane_audio.take_outbound();
        for len in [PANE_AUDIO_FRAME_BYTES - 1, PANE_AUDIO_FRAME_BYTES + 1, 0] {
            let response = app
                .handle_pane_audio_stream_chunk("f".into(), chunk(&pane_id, "owner", vec![0; len]));
            assert_eq!(error_code(&response), "invalid_frame", "len {len}");
        }
        assert!(app.pane_audio.take_outbound().is_empty());
        assert_eq!(
            app.pane_audio
                .session_for_stream(1)
                .unwrap()
                .counters()
                .offered,
            0,
            "a refused body must not spend a slot"
        );
    }

    // TP-MEDIA-API-03
    #[test]
    fn owner_and_pane_are_checked_before_a_frame_is_fed() {
        let (mut app, pane_id) = app();
        assert_eq!(
            error_code(&app.handle_pane_audio_stream_chunk(
                "f".into(),
                chunk("w9:p9", "owner", frame_bytes(0.0))
            )),
            "pane_not_found"
        );
        assert_eq!(
            error_code(&app.handle_pane_audio_stream_chunk(
                "f".into(),
                chunk(&pane_id, "owner", frame_bytes(0.0))
            )),
            "stream_closed"
        );
        assert!(is_ok(&app.handle_pane_audio_stream_open(
            "open".into(),
            params(&pane_id, "owner")
        )));
        assert_eq!(
            error_code(&app.handle_pane_audio_stream_chunk(
                "f".into(),
                chunk(&pane_id, "impostor", frame_bytes(0.0))
            )),
            "stream_conflict"
        );
        assert_eq!(
            error_code(&app.handle_pane_audio_stream_open("open".into(), params(&pane_id, ""))),
            "invalid_stream"
        );
    }

    // TP-MEDIA-API-03
    #[test]
    fn a_second_open_on_a_live_stream_conflicts_and_a_close_ends_it() {
        let (mut app, pane_id) = app();
        assert!(is_ok(&app.handle_pane_audio_stream_open(
            "open".into(),
            params(&pane_id, "a")
        )));
        assert_eq!(
            error_code(&app.handle_pane_audio_stream_open("again".into(), params(&pane_id, "b"))),
            "stream_conflict"
        );
        app.pane_audio.take_outbound();
        assert!(is_ok(&app.handle_pane_audio_stream_close(
            "close".into(),
            PaneAudioStreamCloseParams {
                pane_id: pane_id.clone(),
                owner: "a".into(),
                failed: false,
                detail: String::new(),
            },
        )));
        assert!(matches!(
            app.pane_audio.take_outbound()[..],
            [Outbound::Close {
                stream_id: 1,
                reason: MediaCloseReason::Ended,
                ..
            }]
        ));
        assert!(app.pane_audio.is_empty());
        assert!(is_ok(&app.handle_pane_audio_stream_open(
            "open".into(),
            params(&pane_id, "b")
        )));
        assert!(matches!(
            app.pane_audio.take_outbound()[..],
            [Outbound::Open { stream_id: 2, .. }]
        ));
    }
}
