//! The client side of a stream: chunks in, sound out.
//!
//! Two layers. `AudioPlayback` is the pure core — it is told the time, never
//! reads it, so every rule in it can be tested against a clock the test owns.
//! `PlaybackThread` wraps one core in a thread that reads the real clock and
//! feeds the sink at the pace the jitter buffer dictates.
//!
//! The thread blocks when nothing is open. A client with no stream costs
//! nothing, which is the resource rule this fork holds every component to.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use super::opus::AudioDecoder;
use super::playout::{Chunk, Playout, PlayoutBuffer};
use super::sink::{AudioSink, SinkError};
use super::{now_us, MediaError, CHANNELS, FRAME_MS, FRAME_SAMPLES};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::media::opus::AudioEncoder;
    use crate::media::sink::SilentSink;
    use crate::media::DEFAULT_BITRATE_BPS;

    /// A sink whose behaviour the test chooses.
    struct ScriptedSink {
        frames: Arc<AtomicU64>,
        fail_after: Option<u64>,
    }

    impl AudioSink for ScriptedSink {
        fn write_frame(&mut self, pcm: &[f32]) -> Result<(), SinkError> {
            assert_eq!(pcm.len(), FRAME_SAMPLES * CHANNELS as usize);
            let n = self.frames.fetch_add(1, Ordering::Relaxed) + 1;
            if self.fail_after.is_some_and(|limit| n > limit) {
                return Err(SinkError::Closed("scripted".into()));
            }
            Ok(())
        }
        fn close(&mut self) -> Result<(), SinkError> {
            Ok(())
        }
        fn describe(&self) -> String {
            "scripted".into()
        }
    }

    fn packet() -> Vec<u8> {
        let mut encoder = AudioEncoder::new(DEFAULT_BITRATE_BPS).expect("encoder");
        let pcm: Vec<f32> = (0..FRAME_SAMPLES * CHANNELS as usize)
            .map(|i| ((i as f32) * 0.01).sin() * 0.3)
            .collect();
        let mut out = vec![0u8; 4000];
        let n = encoder.encode(&pcm, &mut out).expect("encode");
        out.truncate(n);
        out
    }

    fn playback() -> (AudioPlayback, Arc<AtomicU64>) {
        let frames = Arc::new(AtomicU64::new(0));
        let sink = ScriptedSink {
            frames: Arc::clone(&frames),
            fail_after: None,
        };
        let p = AudioPlayback::open(7, 100_000, Box::new(sink)).expect("open");
        (p, frames)
    }

    const FRAME_US: u64 = FRAME_MS as u64 * 1000;

    // TP-MEDIA-PLAYBACK-01
    #[test]
    fn a_chunk_reaches_the_sink_at_its_moment_and_not_before() {
        // The playout rule, observed from the sink's side: the frame is not
        // written when it arrives, it is written when it is due.
        let (mut p, frames) = playback();
        let pts = 1_000_000;
        assert!(p.push(0, pts, packet(), pts as i64));
        let due = pts as i64 + i64::from(p.target_delay_us());

        let early = p.tick(due - 1);
        assert_eq!(early.played, 0);
        assert_eq!(frames.load(Ordering::Relaxed), 0, "nothing written early");

        let on_time = p.tick(due);
        assert_eq!(on_time.played, 1);
        assert_eq!(frames.load(Ordering::Relaxed), 1);
        assert_eq!(p.played(), 1);
    }

    // TP-MEDIA-PLAYBACK-02
    #[test]
    fn a_missing_chunk_is_concealed_with_silence_rather_than_skipped() {
        // Silence keeps the sink's own clock honest: the time the lost frame
        // stood for still has to pass, or everything after it plays early and
        // the stream drifts ahead of the picture by one frame per loss.
        let (mut p, frames) = playback();
        let pts0 = 1_000_000;
        p.push(0, pts0, packet(), pts0 as i64);
        p.push(
            2,
            pts0 + 2 * FRAME_US,
            packet(),
            (pts0 + 2 * FRAME_US) as i64,
        );

        let late = pts0 as i64 + 2 * FRAME_US as i64 + i64::from(p.target_delay_us()) + 1;
        let report = p.tick(late);

        assert_eq!(report.played, 2);
        assert_eq!(report.concealed, 1);
        assert_eq!(
            frames.load(Ordering::Relaxed),
            3,
            "two real frames and one frame of silence"
        );
        assert_eq!(p.lost(), 1);
    }

    // TP-MEDIA-PLAYBACK-02
    #[test]
    fn concealment_is_bounded_so_a_long_gap_resumes_instead_of_playing_silence() {
        // Twenty lost frames concealed one by one is a second of nothing while
        // newer audio already waits in the buffer. Past the bound the stream
        // simply resumes from what arrived.
        let (mut p, frames) = playback();
        let pts0 = 1_000_000;
        p.push(0, pts0, packet(), pts0 as i64);
        let far = pts0 + 20 * FRAME_US;
        p.push(20, far, packet(), far as i64);

        let report = p.tick(far as i64 + i64::from(p.target_delay_us()) + 1);
        assert_eq!(report.played, 2);
        assert_eq!(report.concealed as u64, MAX_CONCEAL);
        assert_eq!(frames.load(Ordering::Relaxed), 2 + MAX_CONCEAL);
    }

    // TP-MEDIA-PLAYBACK-03
    #[test]
    fn credit_is_the_room_left_in_the_buffer() {
        // Credit is a level, and this is where the level comes from. It has to
        // fall as chunks are held and rise as they play, or the server either
        // starves a healthy client or floods a stalled one.
        let (mut p, _frames) = playback();
        assert_eq!(p.credit(), CREDIT_ROOM);
        let pts0 = 1_000_000;
        for seq in 0..3u64 {
            p.push(seq, pts0 + seq * FRAME_US, packet(), pts0 as i64);
        }
        assert_eq!(p.credit(), CREDIT_ROOM - 3);

        p.tick(pts0 as i64 + 3 * FRAME_US as i64 + i64::from(p.target_delay_us()));
        assert_eq!(
            p.credit(),
            CREDIT_ROOM,
            "played chunks give their room back"
        );
    }

    // TP-MEDIA-PLAYBACK-04
    #[test]
    fn running_dry_mid_stream_counts_one_underrun_per_gap() {
        // An underrun is the phase's first acceptance criterion, so it has to be
        // counted where it happens and counted once: not before the first frame
        // (nothing has started), not on every tick of the same gap (one silence
        // is one silence), and again for a second gap.
        let (mut p, _frames) = playback();
        let pts0 = 1_000_000;

        assert!(
            !p.tick(pts0 as i64 + 5_000_000).underrun,
            "no stream yet, no underrun"
        );

        p.push(0, pts0, packet(), pts0 as i64);
        let delay = i64::from(p.target_delay_us());
        p.tick(pts0 as i64 + delay);
        assert_eq!(p.played(), 1);

        let gap = pts0 as i64 + delay + FRAME_US as i64 + UNDERRUN_SLACK_US + 1;
        assert!(
            p.tick(gap).underrun,
            "the successor's moment has passed with nothing to play"
        );
        assert!(
            !p.tick(gap + 50_000).underrun,
            "the same gap does not count twice"
        );
        assert_eq!(p.underruns(), 1);

        // Audio resumes, then dries up again.
        let pts1 = (gap + 200_000) as u64;
        p.push(1, pts1, packet(), pts1 as i64);
        p.tick(pts1 as i64 + delay);
        assert_eq!(p.played(), 2);
        let gap2 = pts1 as i64 + delay + FRAME_US as i64 + UNDERRUN_SLACK_US + 1;
        assert!(p.tick(gap2).underrun);
        assert_eq!(p.underruns(), 2);
    }

    // TP-MEDIA-PLAYBACK-05
    #[test]
    fn a_corrupt_packet_is_concealed_and_the_stream_continues() {
        // A packet that does not decode is treated like one that never
        // arrived. Stopping on it would let one corrupted byte end the stream;
        // skipping it without silence would pull every later frame early.
        let (mut p, frames) = playback();
        let pts0 = 1_000_000;
        p.push(0, pts0, vec![0xff, 0x00, 0x13, 0x37], pts0 as i64);
        p.push(1, pts0 + FRAME_US, packet(), pts0 as i64);

        let report = p.tick(pts0 as i64 + FRAME_US as i64 + i64::from(p.target_delay_us()));
        assert_eq!(
            report.played, 2,
            "the corrupt frame still occupies its slot"
        );
        assert_eq!(report.concealed, 1);
        assert_eq!(p.decode_errors(), 1);
        assert_eq!(frames.load(Ordering::Relaxed), 2);
    }

    // TP-MEDIA-PLAYBACK-06
    #[test]
    fn a_sink_that_dies_stops_playback_and_zeroes_the_credit() {
        // Credit zero is the only "stop" this protocol has. A dead sink that
        // kept advertising room would have the server encode and send audio
        // into nothing, healthy at both ends.
        let frames = Arc::new(AtomicU64::new(0));
        let sink = ScriptedSink {
            frames: Arc::clone(&frames),
            fail_after: Some(1),
        };
        let mut p = AudioPlayback::open(7, 100_000, Box::new(sink)).expect("open");
        let pts0 = 1_000_000;
        p.push(0, pts0, packet(), pts0 as i64);
        p.push(1, pts0 + FRAME_US, packet(), pts0 as i64);

        let report = p.tick(pts0 as i64 + FRAME_US as i64 + i64::from(p.target_delay_us()));
        assert_eq!(report.played, 1);
        assert!(report.sink_closed);
        assert!(p.sink_closed());
        assert_eq!(p.credit(), 0);
        assert!(
            !p.push(2, pts0 + 2 * FRAME_US, packet(), pts0 as i64),
            "a closed playback refuses new chunks"
        );
    }

    // TP-MEDIA-PLAYBACK-07
    #[test]
    fn the_playout_thread_plays_a_due_chunk_and_publishes_what_it_did() {
        // The thread wrapper, end to end with real time: open, one chunk due
        // almost at once, and the counters the main loop reads have to move.
        use crate::app::test_wait::LoadAwareDeadline;

        let thread = PlaybackThread::spawn(Box::new(|| {
            Ok(Box::new(SilentSink::new()) as Box<dyn AudioSink>)
        }));
        thread.send(PlaybackCommand::ClockOffset(0));
        thread.send(PlaybackCommand::Open {
            stream_id: 3,
            target_latency_us: 10_000,
        });
        let pts = now_us();
        thread.send(PlaybackCommand::Chunk {
            stream_id: 3,
            seq: 0,
            pts_us: pts,
            data: packet(),
        });

        let deadline = LoadAwareDeadline::new(5, "the playout thread to play one chunk");
        loop {
            if thread.stats().played.load(Ordering::Relaxed) >= 1 {
                break;
            }
            deadline.check();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(thread.stats().open_stream.load(Ordering::Relaxed), 3);
        assert_eq!(
            thread.stats().credit.load(Ordering::Relaxed),
            CREDIT_ROOM,
            "the played chunk gave its room back"
        );
        assert_eq!(
            thread.stats().sink_name.lock().expect("name").as_str(),
            "silent"
        );
        thread.shutdown();
    }

    // TP-MEDIA-PLAYBACK-07
    #[test]
    fn a_sink_that_cannot_open_declines_the_stream_with_zero_credit() {
        use crate::app::test_wait::LoadAwareDeadline;

        let thread = PlaybackThread::spawn(Box::new(|| {
            Err(SinkError::Unavailable("scripted: no device".into()))
        }));
        thread.send(PlaybackCommand::Open {
            stream_id: 9,
            target_latency_us: 10_000,
        });
        let deadline = LoadAwareDeadline::new(5, "the playout thread to decline");
        loop {
            if thread.stats().sink_closed.load(Ordering::Relaxed) {
                break;
            }
            deadline.check();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(thread.stats().credit.load(Ordering::Relaxed), 0);
        assert_eq!(thread.stats().open_stream.load(Ordering::Relaxed), 0);
        thread.shutdown();
    }
}
