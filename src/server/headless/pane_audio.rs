//! Fan-out of pane audio streams to connected clients.
//!
//! The app decides what happens to a stream and records it as [`Outbound`]
//! items; this side owns the writers and the capability sets, so it is the
//! only place that can turn those items into frames on the wire. Open and
//! close travel on the control lane — reliable, and always ahead of media —
//! so a chunk can never overtake the open that announced its stream. Chunks
//! go on the media lane, last in priority and bounded, and a lane that is
//! full is a chunk dropped at the source, not a delay.

use std::sync::mpsc::TrySendError;

use super::{HeadlessServer, RenderImpact};
use crate::api;
use crate::app::pane_audio::{Delivery, Outbound, TARGET_LATENCY_US};
use crate::media::{CHANNELS, SAMPLE_RATE_HZ};
use crate::protocol::{capability, codec, MediaCloseReason, MediaParams, ServerMessage};

impl HeadlessServer {
    /// Stops audio readers whose session the app no longer holds.
    pub(super) fn cancel_inactive_pane_audio_streams(&self) {
        api::cancel_inactive_pane_audio_streams(|owner| self.app.pane_audio.owner_is_active(owner));
    }

    /// Routes one audio-stream request through the app and sends what it
    /// produced. Audio never repaints the terminal.
    pub(super) fn handle_pane_audio_stream_request(
        &mut self,
        msg: api::ApiRequestMessage,
    ) -> RenderImpact {
        let changed = self.handle_api_request_with_shutdown_check_inner(msg, false);
        self.flush_pane_audio_outbound();
        if changed {
            RenderImpact::Full
        } else {
            RenderImpact::None
        }
    }

    /// Per-tick upkeep: release readers of dead panes and tell their clients.
    /// Costs nothing while no stream is open.
    pub(super) fn tick_pane_audio(&mut self) {
        self.cancel_inactive_pane_audio_streams();
        if self.app.pane_audio.retain_live_panes(&self.app.state) {
            self.flush_pane_audio_outbound();
        }
    }

    pub(super) fn flush_pane_audio_outbound(&mut self) {
        for item in self.app.pane_audio.take_outbound() {
            match item {
                Outbound::Open { stream_id, pane_id } => {
                    self.fan_out_media_open(stream_id, pane_id)
                }
                Outbound::Chunk {
                    stream_id,
                    seq,
                    pts_us,
                    data,
                } => self.fan_out_media_chunk(stream_id, seq, pts_us, data),
                Outbound::Close {
                    stream_id,
                    reason,
                    detail,
                    clients,
                } => self.fan_out_media_close(stream_id, reason, detail, clients),
            }
        }
    }

    /// App clients that negotiated an Opus sink, and how many app clients did
    /// not — the latter are never offered a stream they could not play.
    /// TP-MEDIA-CAP-07
    fn audio_capable_clients(&self) -> (Vec<u64>, u64) {
        let mut capable = Vec::new();
        let mut declined = 0_u64;
        for (client_id, client) in &self.clients {
            if client.writer.is_none() || !client.is_full_app_client() {
                continue;
            }
            if client.capabilities.negotiated_value(capability::AUDIO_SINK) == Some(codec::OPUS) {
                capable.push(*client_id);
            } else {
                declined += 1;
            }
        }
        capable.sort_unstable();
        (capable, declined)
    }

    fn fan_out_media_open(&mut self, stream_id: u32, pane_id: String) {
        let message = ServerMessage::MediaOpen {
            stream_id,
            pane_id,
            codec: codec::OPUS.to_owned(),
            params: MediaParams::Audio {
                sample_rate_hz: SAMPLE_RATE_HZ,
                channels: CHANNELS,
            },
            target_latency_us: TARGET_LATENCY_US,
        };
        let Ok(framed) = Self::frame_server_message(&message) else {
            return;
        };
        let (capable, declined) = self.audio_capable_clients();
        tracing::debug!(
            stream_id,
            clients = self.clients.len(),
            capable = capable.len(),
            declined,
            "audio stream fan-out"
        );
        for client_id in capable {
            let sent = self
                .clients
                .get(&client_id)
                .and_then(|client| client.writer.as_ref())
                .is_some_and(|writer| writer.control.send(framed.clone()).is_ok());
            if sent {
                self.app.pane_audio.subscribe(stream_id, client_id);
            }
        }
        self.app.pane_audio.note_declined(stream_id, declined);
    }

    /// One encoded chunk to every subscribed client with room, on the media
    /// lane. A full lane counts as a drop; nothing waits. TP-MEDIA-LANE-02
    fn fan_out_media_chunk(&mut self, stream_id: u32, seq: u64, pts_us: u64, data: Vec<u8>) {
        let Some(session) = self.app.pane_audio.session_for_stream(stream_id) else {
            return;
        };
        let targets = session.clients_with_room();
        if targets.is_empty() {
            return;
        }
        let message = ServerMessage::MediaChunk {
            stream_id,
            seq,
            pts_us,
            data,
        };
        let Ok(framed) = Self::frame_server_message(&message) else {
            return;
        };
        for client_id in targets {
            let Some(writer) = self
                .clients
                .get(&client_id)
                .and_then(|client| client.writer.as_ref())
            else {
                self.app.pane_audio.forget_client(client_id);
                continue;
            };
            let delivery = match writer.media.try_send(framed.clone()) {
                Ok(()) => Delivery::Sent,
                Err(TrySendError::Full(_)) => Delivery::Full,
                Err(TrySendError::Disconnected(_)) => {
                    self.app.pane_audio.forget_client(client_id);
                    continue;
                }
            };
            self.app
                .pane_audio
                .record_delivery(stream_id, client_id, delivery);
        }
    }

    fn fan_out_media_close(
        &mut self,
        stream_id: u32,
        reason: MediaCloseReason,
        detail: String,
        clients: Vec<u64>,
    ) {
        let message = ServerMessage::MediaClose {
            stream_id,
            reason,
            detail,
        };
        let Ok(framed) = Self::frame_server_message(&message) else {
            return;
        };
        for client_id in clients {
            if let Some(writer) = self
                .clients
                .get(&client_id)
                .and_then(|client| client.writer.as_ref())
            {
                let _ = writer.control.send(framed.clone());
            }
        }
    }
}
