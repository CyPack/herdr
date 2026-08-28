//! The client side: hold chunks until their moment, then hand them over in
//! order.
//!
//! This is the jitter buffer, and its whole content is one rule — a chunk is
//! played at `pts_us + target_delay`, never on arrival. Everything else here
//! exists to make that rule survive the things a network does: chunks that
//! arrive out of order, chunks that never arrive, and chunks that arrive after
//! the moment they were for.

use std::collections::BTreeMap;

use super::clock::{JitterEstimator, PlayoutDelay};
use super::FRAME_MS;

/// One received chunk, waiting for its moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub seq: u64,
    pub pts_us: u64,
    pub data: Vec<u8>,
}

/// What the buffer has for the caller right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Playout {
    /// Play this.
    Play(Chunk),
    /// Nothing is due yet. Not an error and not silence — the audio device
    /// keeps playing what it already has.
    Waiting,
    /// A gap of `missing` chunks was skipped. The caller conceals it.
    ///
    /// Reported rather than passed over, because loss the client cannot see is
    /// loss it cannot conceal, and an unconcealed gap is a click.
    Lost { missing: u64 },
}

/// Most chunks held at once.
///
/// A ceiling on memory rather than a policy: at 20 ms a frame this is four
/// seconds, far past any delay the target will ask for, so reaching it means
/// something is wrong rather than merely slow.
const MAX_HELD: usize = 200;

/// Holds chunks until their presentation time.
pub struct PlayoutBuffer {
    delay: PlayoutDelay,
    jitter: JitterEstimator,
    /// Keyed by presentation time, which is also the order they play in.
    /// Arrival order is not that order, and using it would defeat the buffer.
    held: BTreeMap<u64, Chunk>,
    next_seq: Option<u64>,
    dropped_late: u64,
    dropped_full: u64,
    lost: u64,
}

impl PlayoutBuffer {
    /// Starts with the delay the server suggested.
    pub fn new(target_latency_us: u32) -> Self {
        Self {
            delay: PlayoutDelay::new(target_latency_us),
            jitter: JitterEstimator::new(),
            held: BTreeMap::new(),
            next_seq: None,
            dropped_late: 0,
            dropped_full: 0,
            lost: 0,
        }
    }

    /// Accepts a chunk that arrived at `now_server_us`.
    ///
    /// Returns whether it was kept. A chunk already past its moment is refused
    /// here rather than filtered later: keeping it would let it into the
    /// jitter estimate and into the memory bound, both for something that will
    /// never be played.
    pub fn push(&mut self, chunk: Chunk, now_server_us: i64) -> bool {
        self.jitter.observe(chunk.pts_us, now_server_us);
        self.delay.update(self.jitter.jitter_us());

        if self
            .delay
            .is_expired(chunk.pts_us, now_server_us, FRAME_MS * 1000)
        {
            self.dropped_late += 1;
            return false;
        }
        if self.held.len() >= MAX_HELD {
            self.dropped_full += 1;
            return false;
        }
        self.held.insert(chunk.pts_us, chunk);
        true
    }

    /// Takes whatever is due at `now_server_us`.
    pub fn take(&mut self, now_server_us: i64) -> Playout {
        // Anything whose moment passed while it sat here is discarded first.
        // Playing it would move the stream backwards; holding it would block
        // everything behind it.
        loop {
            let Some((&pts, _)) = self.held.iter().next() else {
                return Playout::Waiting;
            };
            if self.delay.is_expired(pts, now_server_us, FRAME_MS * 1000) {
                self.held.remove(&pts);
                self.dropped_late += 1;
                continue;
            }
            if !self.delay.is_due(pts, now_server_us) {
                return Playout::Waiting;
            }
            break;
        }

        let Some((&pts, _)) = self.held.iter().next() else {
            return Playout::Waiting;
        };
        let chunk = self.held.remove(&pts).expect("just observed");

        if let Some(expected) = self.next_seq {
            if chunk.seq > expected {
                let missing = chunk.seq - expected;
                self.lost += missing;
                self.next_seq = Some(chunk.seq);
                // The chunk stays held so the caller gets it on the next call,
                // after it has concealed the gap. Reporting the loss and the
                // audio in one answer would make the caller choose which to
                // honour.
                self.held.insert(pts, chunk);
                return Playout::Lost { missing };
            }
            if chunk.seq < expected {
                // A duplicate or a straggler from before a gap was reported.
                return Playout::Waiting;
            }
        }
        self.next_seq = Some(chunk.seq + 1);
        Playout::Play(chunk)
    }

    pub fn target_delay_us(&self) -> u32 {
        self.delay.target_us()
    }

    pub fn jitter_us(&self) -> f64 {
        self.jitter.jitter_us()
    }

    pub fn held(&self) -> usize {
        self.held.len()
    }

    /// Chunks discarded for arriving or waiting past their moment.
    pub fn dropped_late(&self) -> u64 {
        self.dropped_late
    }

    /// Chunks refused because the buffer was full.
    pub fn dropped_full(&self) -> u64 {
        self.dropped_full
    }

    /// Chunks that never arrived, counted from the sequence gaps.
    pub fn lost(&self) -> u64 {
        self.lost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(seq: u64, pts_us: u64) -> Chunk {
        Chunk {
            seq,
            pts_us,
            data: vec![seq as u8],
        }
    }

    /// A buffer with a settled 100 ms target and no jitter history to disturb
    /// it, so the tests below measure the rule rather than the estimator.
    fn buffer() -> PlayoutBuffer {
        PlayoutBuffer::new(100_000)
    }

    // TP-MEDIA-JITTER-03
    #[test]
    fn an_early_chunk_is_held_until_its_moment() {
        // The rule the layer exists for. Handing a chunk over on arrival passes
        // every network wobble straight to the speaker; holding it until its
        // presentation time converts a variable delay into a constant one.
        let mut buf = buffer();
        assert!(buf.push(chunk(0, 1_000_000), 1_000_000));

        assert_eq!(buf.take(1_050_000), Playout::Waiting);
        assert_eq!(buf.take(1_099_999), Playout::Waiting);
        assert_eq!(buf.take(1_100_000), Playout::Play(chunk(0, 1_000_000)));
    }

    // TP-MEDIA-JITTER-04
    #[test]
    fn a_chunk_that_arrives_after_its_moment_is_refused_at_the_door() {
        // Refused on arrival rather than filtered at playout: keeping it would
        // let it into the jitter estimate and into the memory bound, both for
        // something that can never be played.
        let mut buf = buffer();
        assert!(!buf.push(chunk(0, 1_000_000), 1_500_000));
        assert_eq!(buf.held(), 0);
        assert_eq!(buf.dropped_late(), 1);
        assert_eq!(buf.take(1_500_000), Playout::Waiting);
    }

    // TP-MEDIA-SEQ-01
    #[test]
    fn chunks_that_arrive_out_of_order_play_in_order() {
        // Arrival order is not presentation order, and a buffer that used it
        // would defeat its own purpose — reordering is one of the two things
        // it exists to absorb.
        let mut buf = buffer();
        assert!(buf.push(chunk(1, 1_020_000), 1_020_000));
        assert!(buf.push(chunk(0, 1_000_000), 1_021_000));

        assert_eq!(buf.take(1_100_000), Playout::Play(chunk(0, 1_000_000)));
        assert_eq!(buf.take(1_120_000), Playout::Play(chunk(1, 1_020_000)));
    }

    // TP-MEDIA-SEQ-01
    #[test]
    fn a_missing_chunk_is_reported_before_the_one_after_it_plays() {
        // Loss the caller cannot see is loss it cannot conceal, and an
        // unconcealed gap is an audible click rather than a quiet moment. The
        // report comes first and the audio after, so the caller never has to
        // choose which of the two to honour in one answer.
        let mut buf = buffer();
        assert!(buf.push(chunk(0, 1_000_000), 1_000_000));
        assert_eq!(buf.take(1_100_000), Playout::Play(chunk(0, 1_000_000)));

        // seq 1 never arrives.
        assert!(buf.push(chunk(2, 1_040_000), 1_040_000));
        assert_eq!(buf.take(1_140_000), Playout::Lost { missing: 1 });
        assert_eq!(buf.lost(), 1);
        assert_eq!(buf.take(1_140_000), Playout::Play(chunk(2, 1_040_000)));
    }

    // TP-MEDIA-SEQ-01
    #[test]
    fn a_duplicate_chunk_is_not_played_twice() {
        let mut buf = buffer();
        assert!(buf.push(chunk(0, 1_000_000), 1_000_000));
        assert_eq!(buf.take(1_100_000), Playout::Play(chunk(0, 1_000_000)));

        assert!(buf.push(chunk(0, 1_000_000), 1_001_000));
        assert_eq!(buf.take(1_100_000), Playout::Waiting);
    }

    // TP-MEDIA-JITTER-04
    #[test]
    fn a_chunk_that_expires_while_waiting_is_discarded_rather_than_blocking() {
        // A stalled link leaves chunks in the buffer past their moment. Playing
        // them moves the stream backwards; holding them blocks everything
        // behind, which turns a brief stall into permanent silence.
        let mut buf = buffer();
        assert!(buf.push(chunk(0, 1_000_000), 1_000_000));
        assert!(buf.push(chunk(1, 1_020_000), 1_000_500));

        // Nothing is taken until well past both moments.
        let result = buf.take(1_200_000);
        assert_eq!(result, Playout::Waiting);
        assert_eq!(buf.held(), 0);
        assert_eq!(buf.dropped_late(), 2);
    }

    // TP-MEDIA-JITTER-05
    #[test]
    fn a_steady_stream_keeps_the_target_delay_from_growing() {
        // The counterpart to the estimator's own test, at the level a caller
        // sees: regular arrivals must not inflate the buffer, or the delay
        // creeps upward for the length of the session and the phase's latency
        // target is missed with every test still green.
        let mut buf = buffer();
        let start = buf.target_delay_us();
        for seq in 0..500u64 {
            let pts = 1_000_000 + seq * 20_000;
            buf.push(chunk(seq, pts), pts as i64 + 30_000);
            let _ = buf.take(pts as i64 + 200_000);
        }
        assert!(
            buf.target_delay_us() <= start,
            "a steady stream grew the target from {start} to {}",
            buf.target_delay_us()
        );
    }
}
