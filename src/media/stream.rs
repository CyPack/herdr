//! The server side of one audio stream: sequencing, timestamps, credit, and
//! the decision to drop.
//!
//! The decision that shapes this module is *where* a chunk is dropped. A queue
//! that grows under load delivers every chunk eventually, each one later than
//! the last, and the delay never comes back down — the link recovers and the
//! backlog does not. Dropping at the source is the opposite: the stream thins
//! out while the link is bad and is instantly current again when it is not.
//!
//! That only works here, because only here is the chunk's deadline still known.
//! By the time bytes are in a writer queue they are anonymous.

use super::opus::AudioEncoder;
use super::{MediaError, CHANNELS, FRAME_MS, FRAME_SAMPLES};

/// How far past its presentation time a chunk may still be worth sending.
///
/// One frame. A chunk that late will arrive after the client has already
/// played past its slot, so sending it spends bandwidth to make the stream
/// briefly worse.
pub const MAX_LATENESS_US: u64 = FRAME_MS as u64 * 1000;

/// What happened to one frame offered to the stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChunkFate {
    /// Encoded and ready to queue.
    Send {
        seq: u64,
        pts_us: u64,
        data: Vec<u8>,
    },
    /// The chunk's moment had already passed when it was offered.
    DroppedLate { seq: u64, pts_us: u64 },
    /// The client has no room for it.
    ///
    /// Distinct from `DroppedLate` because it means something different about
    /// the far end: late is the network, no credit is the client's own
    /// playback falling behind, and treating them alike hides which one is
    /// happening.
    DroppedNoCredit { seq: u64, pts_us: u64 },
}

/// One outbound audio stream.
pub struct AudioStream {
    stream_id: u32,
    pane_id: String,
    encoder: AudioEncoder,
    /// Next sequence number. Monotonic and never reused, including across
    /// drops: a gap is how the client learns a chunk is missing, so a stream
    /// that renumbers to stay dense makes loss invisible.
    next_seq: u64,
    /// Presentation time of the next frame, in the server's clock.
    next_pts_us: u64,
    /// Chunks the client has room for.
    credit: u16,
    packet: Vec<u8>,
}

impl AudioStream {
    /// Opens a stream whose first frame presents at `start_pts_us`.
    pub fn new(
        stream_id: u32,
        pane_id: impl Into<String>,
        bitrate_bps: i32,
        start_pts_us: u64,
    ) -> Result<Self, MediaError> {
        Ok(Self {
            stream_id,
            pane_id: pane_id.into(),
            encoder: AudioEncoder::new(bitrate_bps)?,
            next_seq: 0,
            next_pts_us: start_pts_us,
            credit: 0,
            packet: vec![0u8; 4000],
        })
    }

    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }

    pub fn pane_id(&self) -> &str {
        &self.pane_id
    }

    pub fn credit(&self) -> u16 {
        self.credit
    }

    /// Records the room the client says it has.
    ///
    /// Replaces rather than adds. The client reports a level, not a delta, so
    /// a lost or duplicated credit message costs one stale reading instead of
    /// permanently shifting the server's idea of the far end's buffer.
    pub fn set_credit(&mut self, chunks: u16) {
        self.credit = chunks;
    }

    /// Offers one frame of interleaved samples.
    ///
    /// `now_us` is the server's clock. Time advances by one frame whatever the
    /// outcome: a dropped chunk still consumed its slot, and a stream whose
    /// timestamps pause during a drop plays the rest of the audio late by the
    /// length of the outage.
    pub fn offer(&mut self, pcm: &[f32], now_us: u64) -> Result<ChunkFate, MediaError> {
        let expected = FRAME_SAMPLES * CHANNELS as usize;
        if pcm.len() != expected {
            return Err(MediaError::FrameSize {
                expected,
                got: pcm.len(),
            });
        }

        let seq = self.next_seq;
        let pts_us = self.next_pts_us;
        self.next_seq += 1;
        self.next_pts_us += FRAME_MS as u64 * 1000;

        if now_us > pts_us + MAX_LATENESS_US {
            return Ok(ChunkFate::DroppedLate { seq, pts_us });
        }
        if self.credit == 0 {
            return Ok(ChunkFate::DroppedNoCredit { seq, pts_us });
        }

        let written = self.encoder.encode(pcm, &mut self.packet)?;
        self.credit -= 1;
        Ok(ChunkFate::Send {
            seq,
            pts_us,
            data: self.packet[..written].to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::DEFAULT_BITRATE_BPS;

    fn frame() -> Vec<f32> {
        (0..FRAME_SAMPLES * CHANNELS as usize)
            .map(|i| ((i as f32) * 0.001).sin() * 0.4)
            .collect()
    }

    fn stream() -> AudioStream {
        AudioStream::new(1, "w1:p1", DEFAULT_BITRATE_BPS, 1_000_000).expect("stream")
    }

    // TP-MEDIA-SOURCE-01
    #[test]
    fn a_chunk_past_its_moment_is_dropped_before_it_is_encoded() {
        // Dropped at the source, which is the only place the deadline is still
        // known. A queue that holds it instead delivers it eventually — and
        // every chunk after it, each one later than the last, with the delay
        // never coming back down once the link recovers.
        let mut audio = stream();
        audio.set_credit(10);

        let fate = audio.offer(&frame(), 1_100_000).expect("offer");
        assert_eq!(
            fate,
            ChunkFate::DroppedLate {
                seq: 0,
                pts_us: 1_000_000
            }
        );
        assert_eq!(
            audio.credit(),
            10,
            "a dropped chunk must not spend the client's room"
        );
    }

    // TP-MEDIA-SOURCE-01
    #[test]
    fn a_dropped_chunk_still_consumes_its_slot_in_time_and_sequence() {
        // The subtle half. If time paused during a drop, everything after the
        // outage would present late by the length of the outage — the audio
        // would be permanently behind rather than briefly thin. And if the
        // sequence numbers closed up, the client would see an unbroken stream
        // and never learn anything was lost.
        let mut audio = stream();
        audio.set_credit(10);

        let dropped = audio.offer(&frame(), 1_100_000).expect("offer");
        assert!(matches!(dropped, ChunkFate::DroppedLate { seq: 0, .. }));

        let sent = audio.offer(&frame(), 1_000_000).expect("offer");
        match sent {
            ChunkFate::Send { seq, pts_us, .. } => {
                assert_eq!(seq, 1, "the sequence must gap where a chunk was lost");
                assert_eq!(pts_us, 1_020_000, "time advances through a drop");
            }
            other => panic!("expected Send, got {other:?}"),
        }
    }

    // TP-MEDIA-SOURCE-01
    #[test]
    fn a_client_with_no_room_is_not_sent_to() {
        // Without this the server's memory grows instead of the client's
        // queue, and the first symptom of a slow client is the server rather
        // than the playback.
        let mut audio = stream();
        assert_eq!(audio.credit(), 0);

        let fate = audio.offer(&frame(), 1_000_000).expect("offer");
        assert!(matches!(fate, ChunkFate::DroppedNoCredit { seq: 0, .. }));
    }

    // TP-MEDIA-SOURCE-01
    #[test]
    fn credit_is_spent_one_chunk_at_a_time_and_replaced_not_accumulated() {
        // Replaced, because the client reports a level rather than a delta. A
        // lost or duplicated credit message then costs one stale reading, where
        // an accumulating counter would shift the server's idea of the far end
        // permanently and in a direction nothing corrects.
        let mut audio = stream();
        audio.set_credit(2);

        assert!(matches!(
            audio.offer(&frame(), 1_000_000),
            Ok(ChunkFate::Send { seq: 0, .. })
        ));
        assert_eq!(audio.credit(), 1);
        assert!(matches!(
            audio.offer(&frame(), 1_020_000),
            Ok(ChunkFate::Send { seq: 1, .. })
        ));
        assert_eq!(audio.credit(), 0);
        assert!(matches!(
            audio.offer(&frame(), 1_040_000),
            Ok(ChunkFate::DroppedNoCredit { seq: 2, .. })
        ));

        audio.set_credit(5);
        assert_eq!(audio.credit(), 5, "credit is a level, not a running total");
    }

    // TP-MEDIA-SOURCE-01
    #[test]
    fn one_frame_of_lateness_is_still_worth_sending() {
        // The boundary is a decision, not an accident: a chunk exactly one
        // frame late still lands inside the client's jitter buffer, and
        // dropping it would spend a gap to save nothing.
        let mut audio = stream();
        audio.set_credit(10);
        let fate = audio
            .offer(&frame(), 1_000_000 + MAX_LATENESS_US)
            .expect("offer");
        assert!(matches!(fate, ChunkFate::Send { seq: 0, .. }));
    }

    // TP-MEDIA-SOURCE-01
    #[test]
    fn a_wrong_sized_frame_is_refused_without_consuming_a_slot() {
        // A caller mistake must not move the stream's clock. If it did, every
        // rejected frame would shift the audio by 20 ms and the drift would
        // have no visible cause.
        let mut audio = stream();
        audio.set_credit(10);
        assert_eq!(
            audio.offer(&[0.0f32; 8], 1_000_000),
            Err(MediaError::FrameSize {
                expected: FRAME_SAMPLES * CHANNELS as usize,
                got: 8
            })
        );

        let fate = audio.offer(&frame(), 1_000_000).expect("offer");
        match fate {
            ChunkFate::Send { seq, pts_us, .. } => {
                assert_eq!(seq, 0);
                assert_eq!(pts_us, 1_000_000);
            }
            other => panic!("expected Send, got {other:?}"),
        }
    }
}
