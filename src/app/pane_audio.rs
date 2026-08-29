//! Server-side lifecycle of pane audio streams.
//!
//! One session per pane: it owns the encoder-side [`AudioStream`], remembers
//! which client has room for how many chunks, and turns every event into an
//! [`Outbound`] item the server loop fans out to clients. The split is
//! deliberate — this module knows nothing about clients or writers, so the
//! whole lifecycle is testable without a socket, and the fan-out that needs
//! the writers lives beside them in the server.
//!
//! Credit is kept per client rather than as one number on the stream. The
//! stream's own credit is set, before every frame, to the most room any
//! client has: the frame is encoded once when anyone can take it, and each
//! client is then offered it only against its own room. Credit is a level the
//! client reports every 100 ms, never a total the server keeps — so nothing
//! here refunds it, and a refused send simply counts as a drop.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::layout::PaneId;
use crate::media::stream::{AudioStream, ChunkFate};
use crate::media::{MediaError, CHANNELS, DEFAULT_BITRATE_BPS, FRAME_SAMPLES};
use crate::protocol::MediaCloseReason;

/// Playout delay the server suggests when it opens a stream: 100 ms, inside
/// the jitter buffer's 10..500 ms range and small enough to feel live.
pub(crate) const TARGET_LATENCY_US: u32 = 100_000;

/// Bytes in one frame of interleaved little-endian f32 samples.
pub(crate) const FRAME_BYTES: usize = FRAME_SAMPLES * CHANNELS as usize * 4;

/// Something the server loop must send to clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Outbound {
    /// A stream opened; offer it to every client that can play it.
    Open { stream_id: u32, pane_id: String },
    /// One encoded chunk, for every subscribed client with room.
    Chunk {
        stream_id: u32,
        seq: u64,
        pts_us: u64,
        data: Vec<u8>,
    },
    /// A stream ended; tell the clients that were subscribed to it.
    Close {
        stream_id: u32,
        reason: MediaCloseReason,
        detail: String,
        clients: Vec<u64>,
    },
}

/// What happened to the frames one session was offered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Counters {
    pub(crate) offered: u64,
    pub(crate) sent: u64,
    pub(crate) dropped_late: u64,
    pub(crate) dropped_no_credit: u64,
    pub(crate) dropped_full: u64,
    pub(crate) declined_no_sink: u64,
}

/// How one client took one chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Delivery {
    Sent,
    Full,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OpenError {
    /// The pane already has a live stream.
    Conflict,
    /// The encoder could not be built.
    Codec(MediaError),
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OfferError {
    NotOpen,
    OwnerMismatch,
    Frame(MediaError),
}

/// A frame body whose length is not one whole frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FrameBytesError {
    pub(crate) got: usize,
}

/// Turns one body of little-endian f32 samples into a frame, refusing any
/// length that is not exactly one frame — padding or truncating would drift
/// the audio by the difference on every frame.
pub(crate) fn pcm_from_f32le(bytes: &[u8]) -> Result<Vec<f32>, FrameBytesError> {
    if bytes.len() != FRAME_BYTES {
        return Err(FrameBytesError { got: bytes.len() });
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|sample| f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]))
        .collect())
}

pub(crate) struct Session {
    owner: String,
    stream_id: u32,
    pane_id: String,
    stream: AudioStream,
    active: Option<Arc<AtomicBool>>,
    client_credit: HashMap<u64, u16>,
    counters: Counters,
}

impl Session {
    pub(crate) fn stream_id(&self) -> u32 {
        self.stream_id
    }

    pub(crate) fn owner(&self) -> &str {
        &self.owner
    }

    pub(crate) fn pane_id(&self) -> &str {
        &self.pane_id
    }

    pub(crate) fn counters(&self) -> Counters {
        self.counters
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
            .as_ref()
            .is_some_and(|active| active.load(Ordering::Acquire))
    }

    pub(crate) fn credit_of(&self, client_id: u64) -> u16 {
        self.client_credit.get(&client_id).copied().unwrap_or(0)
    }

    pub(crate) fn subscribers(&self) -> Vec<u64> {
        let mut clients: Vec<u64> = self.client_credit.keys().copied().collect();
        clients.sort_unstable();
        clients
    }

    /// Clients that currently have room for a chunk.
    pub(crate) fn clients_with_room(&self) -> Vec<u64> {
        let mut clients: Vec<u64> = self
            .client_credit
            .iter()
            .filter(|(_, credit)| **credit > 0)
            .map(|(client, _)| *client)
            .collect();
        clients.sort_unstable();
        clients
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        if let Some(active) = &self.active {
            active.store(false, Ordering::Release);
        }
    }
}

#[derive(Default)]
pub(crate) struct Runtime {
    sessions: HashMap<PaneId, Session>,
    next_stream_id: u32,
    outbound: Vec<Outbound>,
}

impl Runtime {
    /// Opens a stream on `pane`, superseding a stale one and refusing a live one.
    pub(crate) fn open(
        &mut self,
        pane: PaneId,
        pane_id: &str,
        owner: &str,
        now_us: u64,
    ) -> Result<u32, OpenError> {
        if let Some(existing) = self.sessions.get(&pane) {
            if existing.is_active() {
                return Err(OpenError::Conflict);
            }
            if let Some(stale) = self.sessions.remove(&pane) {
                self.outbound.push(Outbound::Close {
                    stream_id: stale.stream_id,
                    reason: MediaCloseReason::Failed,
                    detail: "superseded by a new stream".into(),
                    clients: stale.subscribers(),
                });
            }
        }
        let stream_id = self.allocate_stream_id();
        let stream = AudioStream::new(stream_id, pane_id, DEFAULT_BITRATE_BPS, now_us)
            .map_err(OpenError::Codec)?;
        self.sessions.insert(
            pane,
            Session {
                owner: owner.to_owned(),
                stream_id,
                pane_id: pane_id.to_owned(),
                stream,
                active: Some(Arc::new(AtomicBool::new(true))),
                client_credit: HashMap::new(),
                counters: Counters::default(),
            },
        );
        self.outbound.push(Outbound::Open {
            stream_id,
            pane_id: pane_id.to_owned(),
        });
        Ok(stream_id)
    }

    /// Stream ids start at 1: the client reserves 0 for "no stream open".
    fn allocate_stream_id(&mut self) -> u32 {
        let id = self.next_stream_id.max(1);
        self.next_stream_id = id.wrapping_add(1).max(1);
        id
    }

    /// Binds the reader's cancellation flag to the session it opened.
    pub(crate) fn attach_active(&mut self, pane: PaneId, owner: &str, active: Arc<AtomicBool>) {
        if let Some(session) = self
            .sessions
            .get_mut(&pane)
            .filter(|session| session.owner == owner)
        {
            session.active = Some(active);
        }
    }

    /// Whether some session still owns a live stream under this owner — the
    /// cancel sweep's single source of truth.
    pub(crate) fn owner_is_active(&self, owner: &str) -> bool {
        self.sessions
            .values()
            .any(|session| session.owner == owner && session.is_active())
    }

    /// Offers one frame to the pane's stream.
    ///
    /// The stream's credit is set to the most room any client has before the
    /// frame is offered, so a frame is encoded exactly when someone can take
    /// it. A `Send` becomes an outbound chunk; every other fate is counted.
    pub(crate) fn offer(
        &mut self,
        pane: PaneId,
        owner: &str,
        pcm: &[f32],
        now_us: u64,
    ) -> Result<ChunkFate, OfferError> {
        let session = self.sessions.get_mut(&pane).ok_or(OfferError::NotOpen)?;
        if session.owner != owner {
            return Err(OfferError::OwnerMismatch);
        }
        let room = session.client_credit.values().copied().max().unwrap_or(0);
        session.stream.set_credit(room);
        session.counters.offered += 1;
        let fate = session
            .stream
            .offer(pcm, now_us)
            .map_err(OfferError::Frame)?;
        match &fate {
            ChunkFate::Send { seq, pts_us, data } => self.outbound.push(Outbound::Chunk {
                stream_id: session.stream_id,
                seq: *seq,
                pts_us: *pts_us,
                data: data.clone(),
            }),
            ChunkFate::DroppedLate { .. } => session.counters.dropped_late += 1,
            ChunkFate::DroppedNoCredit { .. } => session.counters.dropped_no_credit += 1,
        }
        Ok(fate)
    }

    /// Closes the pane's stream if `owner` owns it. Idempotent: a second close
    /// finds nothing and does nothing.
    pub(crate) fn close(
        &mut self,
        pane: PaneId,
        owner: &str,
        reason: MediaCloseReason,
        detail: String,
    ) -> bool {
        let owns = self
            .sessions
            .get(&pane)
            .is_some_and(|session| session.owner == owner);
        if !owns {
            return false;
        }
        let Some(session) = self.sessions.remove(&pane) else {
            return false;
        };
        self.outbound.push(Outbound::Close {
            stream_id: session.stream_id,
            reason,
            detail,
            clients: session.subscribers(),
        });
        true
    }

    /// Records that a client was offered the stream. Its room starts at zero
    /// until it reports otherwise, which it does the moment it accepts.
    pub(crate) fn subscribe(&mut self, stream_id: u32, client_id: u64) {
        if let Some(session) = self.session_for_stream_mut(stream_id) {
            session.client_credit.entry(client_id).or_insert(0);
        }
    }

    /// Counts clients that could not be offered the stream at all.
    pub(crate) fn note_declined(&mut self, stream_id: u32, clients: u64) {
        if let Some(session) = self.session_for_stream_mut(stream_id) {
            session.counters.declined_no_sink += clients;
        }
    }

    /// Records the room a subscribed client says it has. A report for a client
    /// that was never offered the stream is ignored: it cannot be playing it.
    pub(crate) fn set_client_credit(
        &mut self,
        stream_id: u32,
        client_id: u64,
        chunks: u16,
    ) -> bool {
        let Some(session) = self.session_for_stream_mut(stream_id) else {
            return false;
        };
        match session.client_credit.get_mut(&client_id) {
            Some(credit) => {
                *credit = chunks;
                true
            }
            None => false,
        }
    }

    /// Records how one client took one chunk. A successful send spends one of
    /// that client's room; a refusal is counted and the room is left alone —
    /// the client will report its real level again shortly.
    pub(crate) fn record_delivery(&mut self, stream_id: u32, client_id: u64, delivery: Delivery) {
        let Some(session) = self.session_for_stream_mut(stream_id) else {
            return;
        };
        match delivery {
            Delivery::Sent => {
                if let Some(credit) = session.client_credit.get_mut(&client_id) {
                    *credit = credit.saturating_sub(1);
                }
                session.counters.sent += 1;
            }
            Delivery::Full => session.counters.dropped_full += 1,
        }
    }

    /// Forgets a client that is gone, so its stale room can never send to it.
    pub(crate) fn forget_client(&mut self, client_id: u64) {
        for session in self.sessions.values_mut() {
            session.client_credit.remove(&client_id);
        }
    }

    /// Drops every session whose pane no longer exists, closing its stream
    /// as failed so the clients and the reader both learn it is gone.
    pub(crate) fn retain_live_panes(&mut self, state: &crate::app::state::AppState) -> bool {
        if self.sessions.is_empty() {
            return false;
        }
        let dead: Vec<PaneId> = self
            .sessions
            .keys()
            .copied()
            .filter(|pane| {
                !state
                    .workspaces
                    .iter()
                    .any(|workspace| workspace.pane_state(*pane).is_some())
            })
            .collect();
        for pane in &dead {
            if let Some(session) = self.sessions.remove(pane) {
                self.outbound.push(Outbound::Close {
                    stream_id: session.stream_id,
                    reason: MediaCloseReason::Failed,
                    detail: "pane closed".into(),
                    clients: session.subscribers(),
                });
            }
        }
        !dead.is_empty()
    }

    /// Hands the server loop everything that must be sent since the last call.
    pub(crate) fn take_outbound(&mut self) -> Vec<Outbound> {
        std::mem::take(&mut self.outbound)
    }

    pub(crate) fn session_for_pane(&self, pane: PaneId) -> Option<&Session> {
        self.sessions.get(&pane)
    }

    pub(crate) fn session_for_stream(&self, stream_id: u32) -> Option<&Session> {
        self.sessions
            .values()
            .find(|session| session.stream_id == stream_id)
    }

    fn session_for_stream_mut(&mut self, stream_id: u32) -> Option<&mut Session> {
        self.sessions
            .values_mut()
            .find(|session| session.stream_id == stream_id)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Workspace;

    fn frame() -> Vec<f32> {
        (0..FRAME_SAMPLES * CHANNELS as usize)
            .map(|i| ((i as f32) * 0.002).sin() * 0.3)
            .collect()
    }

    fn pane_in(state: &mut crate::app::state::AppState) -> PaneId {
        let workspace = Workspace::test_new("audio");
        let pane = workspace.tabs[0].root_pane;
        state.workspaces = vec![workspace];
        pane
    }

    // TP-MEDIA-API-03
    #[test]
    fn opening_a_stream_numbers_it_from_one_and_offers_it_to_clients() {
        let mut runtime = Runtime::default();
        let pane = PaneId::from_raw(3);
        let id = runtime.open(pane, "w1:p3", "owner-a", 5_000).expect("open");
        assert_eq!(id, 1, "the client reserves stream 0 for 'nothing open'");
        assert_eq!(
            runtime.take_outbound(),
            vec![Outbound::Open {
                stream_id: 1,
                pane_id: "w1:p3".into()
            }]
        );
        let session = runtime.session_for_pane(pane).expect("session");
        assert!(session.is_active());
        assert_eq!(session.owner(), "owner-a");
    }

    // TP-MEDIA-API-03
    #[test]
    fn a_live_stream_refuses_a_second_opener_but_a_dead_one_is_superseded() {
        let mut runtime = Runtime::default();
        let pane = PaneId::from_raw(1);
        runtime.open(pane, "w1:p1", "first", 0).expect("open");
        assert_eq!(
            runtime.open(pane, "w1:p1", "second", 0),
            Err(OpenError::Conflict)
        );

        let dead = Arc::new(AtomicBool::new(false));
        runtime.attach_active(pane, "first", dead);
        runtime.subscribe(1, 42);
        let id = runtime.open(pane, "w1:p1", "second", 0).expect("supersede");
        assert_eq!(id, 2);
        let outbound = runtime.take_outbound();
        assert!(
            matches!(
                &outbound[..],
                [Outbound::Open { stream_id: 1, .. }, Outbound::Close { stream_id: 1, reason: MediaCloseReason::Failed, clients, .. }, Outbound::Open { stream_id: 2, .. }]
                    if clients == &vec![42]
            ),
            "{outbound:?}"
        );
        assert!(!runtime.owner_is_active("first"));
        assert!(runtime.owner_is_active("second"));
    }

    // TP-MEDIA-API-03
    #[test]
    fn a_frame_is_encoded_once_the_moment_any_client_has_room() {
        let mut runtime = Runtime::default();
        let pane = PaneId::from_raw(1);
        runtime
            .open(pane, "w1:p1", "owner", 1_000_000)
            .expect("open");
        runtime.subscribe(1, 7);
        runtime.subscribe(1, 8);
        runtime.take_outbound();

        // Nobody has room yet: the frame is counted, never encoded.
        let fate = runtime
            .offer(pane, "owner", &frame(), 1_000_000)
            .expect("offer");
        assert!(matches!(fate, ChunkFate::DroppedNoCredit { seq: 0, .. }));
        assert!(runtime.take_outbound().is_empty());

        assert!(runtime.set_client_credit(1, 8, 3));
        let fate = runtime
            .offer(pane, "owner", &frame(), 1_020_000)
            .expect("offer");
        match fate {
            ChunkFate::Send { seq, pts_us, .. } => {
                assert_eq!(seq, 1, "the dropped frame still spent its slot");
                assert_eq!(pts_us, 1_020_000);
            }
            other => panic!("expected Send, got {other:?}"),
        }
        let outbound = runtime.take_outbound();
        assert_eq!(outbound.len(), 1);
        assert!(matches!(
            &outbound[0],
            Outbound::Chunk { stream_id: 1, seq: 1, pts_us: 1_020_000, data } if !data.is_empty()
        ));
        let session = runtime.session_for_pane(pane).expect("session");
        assert_eq!(session.clients_with_room(), vec![8]);
        assert_eq!(session.counters().offered, 2);
        assert_eq!(session.counters().dropped_no_credit, 1);
    }

    // TP-MEDIA-API-03
    #[test]
    fn a_delivery_spends_that_client_room_and_a_refusal_only_counts() {
        let mut runtime = Runtime::default();
        let pane = PaneId::from_raw(1);
        runtime.open(pane, "w1:p1", "owner", 0).expect("open");
        runtime.subscribe(1, 7);
        runtime.set_client_credit(1, 7, 2);

        runtime.record_delivery(1, 7, Delivery::Sent);
        runtime.record_delivery(1, 7, Delivery::Full);
        let session = runtime.session_for_stream(1).expect("session");
        assert_eq!(session.credit_of(7), 1);
        assert_eq!(session.counters().sent, 1);
        assert_eq!(session.counters().dropped_full, 1);

        // A level, not a total: the next report replaces what the server thought.
        runtime.set_client_credit(1, 7, 9);
        assert_eq!(runtime.session_for_stream(1).unwrap().credit_of(7), 9);
        // A client that was never offered the stream cannot report room for it.
        assert!(!runtime.set_client_credit(1, 99, 4));
        assert!(!runtime.set_client_credit(5, 7, 4));
    }

    // TP-MEDIA-API-03
    #[test]
    fn the_wrong_owner_cannot_feed_or_close_a_stream() {
        let mut runtime = Runtime::default();
        let pane = PaneId::from_raw(1);
        runtime.open(pane, "w1:p1", "owner", 0).expect("open");
        assert_eq!(
            runtime.offer(pane, "impostor", &frame(), 0),
            Err(OfferError::OwnerMismatch)
        );
        assert!(!runtime.close(pane, "impostor", MediaCloseReason::Ended, String::new()));
        assert!(runtime.session_for_pane(pane).is_some());
        assert_eq!(
            runtime.offer(PaneId::from_raw(9), "owner", &frame(), 0),
            Err(OfferError::NotOpen)
        );
    }

    // TP-MEDIA-API-03
    #[test]
    fn closing_tells_the_subscribed_clients_and_is_idempotent() {
        let mut runtime = Runtime::default();
        let pane = PaneId::from_raw(1);
        runtime.open(pane, "w1:p1", "owner", 0).expect("open");
        runtime.subscribe(1, 3);
        runtime.subscribe(1, 5);
        runtime.take_outbound();

        assert!(runtime.close(pane, "owner", MediaCloseReason::Ended, String::new()));
        assert_eq!(
            runtime.take_outbound(),
            vec![Outbound::Close {
                stream_id: 1,
                reason: MediaCloseReason::Ended,
                detail: String::new(),
                clients: vec![3, 5],
            }]
        );
        assert!(!runtime.close(pane, "owner", MediaCloseReason::Ended, String::new()));
        assert!(runtime.take_outbound().is_empty());
        assert!(runtime.is_empty());
    }

    // TP-MEDIA-API-03
    #[test]
    fn a_stream_whose_pane_disappears_is_closed_as_failed_and_its_reader_released() {
        let mut runtime = Runtime::default();
        let mut state = crate::app::state::AppState::test_new();
        let pane = pane_in(&mut state);
        runtime.open(pane, "w1:p1", "owner", 0).expect("open");
        let active = Arc::new(AtomicBool::new(true));
        runtime.attach_active(pane, "owner", Arc::clone(&active));
        runtime.subscribe(1, 4);
        runtime.take_outbound();

        assert!(
            !runtime.retain_live_panes(&state),
            "a live pane keeps its stream"
        );
        state.workspaces.clear();
        assert!(runtime.retain_live_panes(&state));
        assert_eq!(
            runtime.take_outbound(),
            vec![Outbound::Close {
                stream_id: 1,
                reason: MediaCloseReason::Failed,
                detail: "pane closed".into(),
                clients: vec![4],
            }]
        );
        assert!(
            !active.load(Ordering::Acquire),
            "dropping the session must release the reader blocked on its socket"
        );
        assert!(!runtime.owner_is_active("owner"));
    }

    // TP-MEDIA-API-03
    #[test]
    fn a_departed_client_is_forgotten_by_every_stream() {
        let mut runtime = Runtime::default();
        runtime
            .open(PaneId::from_raw(1), "w1:p1", "a", 0)
            .expect("open");
        runtime
            .open(PaneId::from_raw(2), "w1:p2", "b", 0)
            .expect("open");
        runtime.subscribe(1, 7);
        runtime.subscribe(2, 7);
        runtime.subscribe(2, 8);
        runtime.set_client_credit(1, 7, 5);
        runtime.set_client_credit(2, 7, 5);

        runtime.forget_client(7);
        assert!(runtime
            .session_for_stream(1)
            .unwrap()
            .subscribers()
            .is_empty());
        assert_eq!(
            runtime.session_for_stream(2).unwrap().subscribers(),
            vec![8]
        );
        assert!(runtime
            .session_for_stream(1)
            .unwrap()
            .clients_with_room()
            .is_empty());
    }

    // TP-MEDIA-API-01
    #[test]
    fn a_frame_body_must_be_exactly_one_frame_of_f32le_samples() {
        assert_eq!(
            pcm_from_f32le(&[0_u8; FRAME_BYTES - 1]),
            Err(FrameBytesError {
                got: FRAME_BYTES - 1
            })
        );
        assert_eq!(
            pcm_from_f32le(&[0_u8; FRAME_BYTES + 1]),
            Err(FrameBytesError {
                got: FRAME_BYTES + 1
            })
        );
        let mut body = vec![0_u8; FRAME_BYTES];
        body[..4].copy_from_slice(&0.5_f32.to_le_bytes());
        body[FRAME_BYTES - 4..].copy_from_slice(&(-0.25_f32).to_le_bytes());
        let pcm = pcm_from_f32le(&body).expect("whole frame");
        assert_eq!(pcm.len(), FRAME_SAMPLES * CHANNELS as usize);
        assert_eq!(pcm[0], 0.5);
        assert_eq!(pcm[pcm.len() - 1], -0.25);
    }
}
