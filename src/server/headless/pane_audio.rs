//! Fan-out of pane audio streams to connected clients.
//!
//! The app decides what happens to a stream and records it as [`Outbound`]
//! items; this side owns the writers and the capability sets, so it is the
//! only place that can turn those items into frames on the wire. Open and
//! close travel on the control lane — reliable, and always ahead of media —
//! so a chunk can never overtake the open that announced its stream. Chunks
//! go on the media lane, last in priority and bounded, and a lane that is
//! full is a chunk dropped at the source, not a delay.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc::TrySendError;
use std::time::Instant;

use super::{pane_audio_capture, HeadlessServer, RenderImpact};
use pane_audio_capture::CaptureEvent;

/// The name this server's own capture holds its streams under.
///
/// Distinct from any external producer's owner string, so the cancel sweep and
/// the ownership checks can tell a capture this server started from one a
/// program on the socket started.
const CAPTURE_OWNER: &str = "pane-capture";

/// The chain of a producing pid, nearest first, `init` left out.
fn ancestry_of(pid: u32) -> Vec<u32> {
    let mut chain = Vec::new();
    let mut current = pid;
    // Bounded: a cycle in /proc would otherwise hang the server loop, and a
    // depth this large has never been a real process tree.
    for _ in 0..64 {
        let Some(parent) = crate::platform::process_parent(current) else {
            break;
        };
        chain.push(parent);
        current = parent;
    }
    chain
}

/// Whether the producer or anything above it carries this pane's marker.
fn marker_in_chain(pid: Option<u32>, ancestors: &[u32], public_id: &str) -> bool {
    let Some(pid) = pid else {
        return false;
    };
    std::iter::once(&pid)
        .chain(ancestors.iter())
        .any(|candidate| {
            crate::platform::process_environ_has(*candidate, PANE_MARKER_ENV, public_id)
        })
}

/// The variable a linked web pane sets on its launcher.
const PANE_MARKER_ENV: &str = "HERDR_WEB_LINKED_AGENT";
use crate::api;
use crate::app::pane_audio::{Delivery, Outbound, TARGET_LATENCY_US};
use crate::app::pane_audio_source::{
    match_pane_source, plan, PaneProcesses, PaneSourceState, SourceAction, SourceCandidate,
};
use crate::layout::PaneId;
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
        self.tick_pane_audio_capture();
    }

    /// Runs the capture only while somebody could hear it.
    ///
    /// The gate is first and it is absolute: with no client that negotiated an
    /// audio sink, the supervisor is torn down rather than idled. An idle
    /// supervisor still holds a watcher process and a thread, and a cost that
    /// nobody asked for is invisible until it is measured — which is exactly
    /// the failure the resource doctrine exists to prevent.
    fn tick_pane_audio_capture(&mut self) {
        let (capable, declined) = self.audio_capable_clients();
        if capable.is_empty() {
            if let Some(mut capture) = self.pane_audio_capture.take() {
                capture.stop_all();
                tracing::info!(
                    declined,
                    "pane audio capture stopped: no client can play audio"
                );
            }
            return;
        }
        // Both edges are logged at info, and that is what makes the *absence*
        // of a line evidence too: if the gate ever opened, this line exists, so
        // a log without it says the clients never announced an audio sink —
        // which is the likeliest reason for silence and the one a diagnostic
        // printed behind this gate could never report.
        let capture = match self.pane_audio_capture {
            Some(ref mut capture) => capture,
            None => {
                tracing::info!(
                    listeners = capable.len(),
                    declined,
                    "pane audio capture starting"
                );
                self.pane_audio_capture
                    .insert(pane_audio_capture::CaptureSupervisor::new())
            }
        };
        // A platform with no watcher has nothing to watch; the error says so
        // once and the next tick asks again for free.
        if capture.watch().is_err() {
            return;
        }
        // The graph is re-read only after it has settled. Reading it per event
        // would cost a process launch for every volume slider tick.
        if capture.graph_settled(Instant::now()) {
            self.resync_pane_audio_sources(capable.len());
        }
        self.deliver_captured_pane_audio();
    }

    /// Re-reads the audio graph and makes the captures match what it says.
    ///
    /// The whole picture is recomputed rather than patched per event, which is
    /// what makes a missed signal cost a late decision instead of a wrong one.
    fn resync_pane_audio_sources(&mut self, listeners: usize) {
        let Some(Ok(streams)) = crate::platform::read_output_streams() else {
            return;
        };
        // Aiming needs the serial, the rules speak in node ids, and the two are
        // different numbers for the same stream.
        let serials: BTreeMap<u32, u32> = streams
            .iter()
            .filter_map(|stream| Some((stream.node_id, stream.object_serial?)))
            .collect();
        // Each producing pid's ancestry is walked once, not once per pane.
        let mut chains: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for stream in &streams {
            if let Some(pid) = stream.pid {
                chains.entry(pid).or_insert_with(|| ancestry_of(pid));
            }
        }

        let mut states: Vec<PaneSourceState> = Vec::new();
        for ws_idx in 0..self.app.state.workspaces.len() {
            for pane in self.app.state.pane_ids_for_workspace(ws_idx) {
                let Some(public_id) = self.app.public_pane_id(ws_idx, pane) else {
                    continue;
                };
                let processes = self.pane_processes(ws_idx, pane);
                if processes.pids.is_empty() {
                    continue;
                }
                let candidates: Vec<SourceCandidate> = streams
                    .iter()
                    .map(|stream| {
                        let ancestors = stream
                            .pid
                            .and_then(|pid| chains.get(&pid))
                            .cloned()
                            .unwrap_or_default();
                        SourceCandidate {
                            node_id: stream.node_id,
                            pid: stream.pid,
                            carries_pane_marker: marker_in_chain(
                                stream.pid, &ancestors, &public_id,
                            ),
                            ancestors,
                            app_name: stream.app_name.clone(),
                        }
                    })
                    .collect();
                states.push(PaneSourceState {
                    pane_id: public_id,
                    matched: match_pane_source(&candidates, &processes),
                });
            }
        }

        let Some(capture) = self.pane_audio_capture.as_mut() else {
            return;
        };
        let open: BTreeSet<String> = capture.captured_panes();
        for action in plan(&states, &open, listeners) {
            match action {
                SourceAction::Close { pane_id, reason } => {
                    capture.stop(&pane_id);
                    tracing::debug!(pane_id, ?reason, "pane audio capture stopped");
                }
                SourceAction::Open { pane_id, node_id } => {
                    let Some(serial) = serials.get(&node_id) else {
                        // A stream the graph named but could not aim at. Saying
                        // so is better than recording someone else's sound.
                        tracing::debug!(pane_id, node_id, "stream has no capture serial");
                        continue;
                    };
                    match capture.start(&pane_id, *serial) {
                        Ok(()) => {
                            tracing::debug!(pane_id, node_id, serial, "pane audio capture started")
                        }
                        Err(err) => {
                            tracing::debug!(pane_id, %err, "pane audio capture unavailable")
                        }
                    }
                }
            }
        }
        // One line that answers "why is there no sound" without a rebuild:
        // whether anything is being watched, how many panes are captured, and
        // whether frames are being thrown away because the loop is behind.
        //
        // At info, because the server's default filter is `herdr=info` and a
        // diagnostic nobody can see is not a diagnostic — but only when the
        // state changes, because the same line every quarter second would be
        // noise and would get filtered back out by whoever reads it.
        let dropped = capture.dropped_frames();
        let report = pane_audio_capture::CaptureReport {
            watching: capture.is_watching(),
            captured: capture.active_panes(),
            dropping: dropped > 0,
            streams: streams.len(),
            listeners,
        };
        if capture.report_changed(report) {
            tracing::info!(
                watching = report.watching,
                captured = report.captured,
                dropped,
                streams = report.streams,
                listeners,
                "pane audio capture state"
            );
        }
    }

    /// Hands what the capture threads produced to the app, in the one order
    /// that works: the open carries the first frame, every frame after it is
    /// offered, and a source that ended closes its stream.
    fn deliver_captured_pane_audio(&mut self) {
        let Some(capture) = self.pane_audio_capture.as_mut() else {
            return;
        };
        let events = capture.drain();
        if events.is_empty() {
            return;
        }
        let now_us = crate::media::now_us();
        for event in events {
            match event {
                CaptureEvent::Opened { pane_id, frame } => {
                    let Some(pane) = self.pane_for_public_id(&pane_id) else {
                        continue;
                    };
                    if self
                        .app
                        .pane_audio
                        .open(pane, &pane_id, CAPTURE_OWNER, now_us)
                        .is_err()
                    {
                        continue;
                    }
                    self.offer_captured_frame(pane, &frame, now_us);
                }
                CaptureEvent::Frame { pane_id, frame } => {
                    let Some(pane) = self.pane_for_public_id(&pane_id) else {
                        continue;
                    };
                    self.offer_captured_frame(pane, &frame, now_us);
                }
                CaptureEvent::Ended { pane_id } => {
                    let Some(pane) = self.pane_for_public_id(&pane_id) else {
                        continue;
                    };
                    self.app.pane_audio.close(
                        pane,
                        CAPTURE_OWNER,
                        MediaCloseReason::Ended,
                        "capture ended".to_owned(),
                    );
                }
            }
        }
        self.flush_pane_audio_outbound();
    }

    /// Turns one captured frame into samples and offers it.
    ///
    /// A frame of the wrong length is refused here rather than padded: padding
    /// shifts the clock by the difference on every frame that follows.
    fn offer_captured_frame(&mut self, pane: PaneId, frame: &[u8], now_us: u64) {
        let Ok(pcm) = crate::app::pane_audio::pcm_from_f32le(frame) else {
            return;
        };
        let _ = self.app.pane_audio.offer(pane, CAPTURE_OWNER, &pcm, now_us);
    }

    /// The pane behind a public id, or `None` if it has since gone.
    fn pane_for_public_id(&self, public_id: &str) -> Option<PaneId> {
        for ws_idx in 0..self.app.state.workspaces.len() {
            for pane in self.app.state.pane_ids_for_workspace(ws_idx) {
                if self.app.public_pane_id(ws_idx, pane).as_deref() == Some(public_id) {
                    return Some(pane);
                }
            }
        }
        None
    }

    /// The processes a pane owns: its shell and whatever is in the foreground.
    fn pane_processes(&self, ws_idx: usize, pane: PaneId) -> PaneProcesses {
        let mut pids = BTreeSet::new();
        let mut names = BTreeSet::new();
        if let Some(runtime) =
            self.app
                .state
                .runtime_for_pane_in_workspace(&self.app.terminal_runtimes, ws_idx, pane)
        {
            if let Some(shell) = runtime.child_pid() {
                pids.insert(shell);
                if let Some(job) = crate::detect::foreground_job(shell) {
                    for process in job.processes {
                        pids.insert(process.pid);
                        names.insert(process.name);
                    }
                }
            }
        }
        PaneProcesses { pids, names }
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
            if seq == 0 {
                tracing::debug!(stream_id, client_id, "first audio chunk offered");
            }
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
        if let Some(session) = self.app.pane_audio.session_for_stream(stream_id) {
            tracing::info!(
                stream_id,
                ?reason,
                counters = %format!("{:?}", session.counters()),
                subscribers = clients.len(),
                "audio stream closing"
            );
        }

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
