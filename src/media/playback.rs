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

/// Chunks the client tells the server it has room for when its buffer is
/// empty. 32 frames of 20 ms is 640 ms — more than the largest playout delay,
/// so a healthy link is never throttled by credit, and small enough that a
/// stalled client cannot pull a second of audio into a buffer it will drop.
pub const CREDIT_ROOM: u16 = 32;

/// Most consecutive missing chunks replaced with silence.
///
/// One or two lost frames concealed with silence is a quiet moment; twenty
/// would be a second of nothing while the buffer already holds newer audio.
/// Past this the stream simply resumes from what arrived.
const MAX_CONCEAL: u64 = 5;

/// How far past its due time a played frame may fall behind before the gap
/// counts as an underrun. Two frames: one is the ordinary scheduling slack of
/// a 5 ms tick, the second is the actual silence.
const UNDERRUN_SLACK_US: i64 = 2 * FRAME_MS as i64 * 1000;

/// What one `tick` did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TickReport {
    pub played: u32,
    pub concealed: u32,
    pub underrun: bool,
    pub sink_closed: bool,
}

/// One open audio stream and the sink it plays into.
pub struct AudioPlayback {
    stream_id: u32,
    decoder: AudioDecoder,
    buffer: PlayoutBuffer,
    sink: Box<dyn AudioSink>,
    pcm: Vec<f32>,
    silence: Vec<f32>,
    played: u64,
    concealed: u64,
    decode_errors: u64,
    underruns: u64,
    /// Set when the buffer was found empty mid-stream; cleared by the next
    /// played frame, so one gap counts once however many ticks it spans.
    starved: bool,
    last_played_pts: Option<u64>,
    sink_closed: bool,
}

impl AudioPlayback {
    /// Opens playback for `stream_id` into `sink`, starting at the server's
    /// suggested delay.
    pub fn open(
        stream_id: u32,
        target_latency_us: u32,
        sink: Box<dyn AudioSink>,
    ) -> Result<Self, MediaError> {
        let frame = FRAME_SAMPLES * CHANNELS as usize;
        Ok(Self {
            stream_id,
            decoder: AudioDecoder::new()?,
            buffer: PlayoutBuffer::new(target_latency_us),
            sink,
            pcm: vec![0.0; frame],
            silence: vec![0.0; frame],
            played: 0,
            concealed: 0,
            decode_errors: 0,
            underruns: 0,
            starved: false,
            last_played_pts: None,
            sink_closed: false,
        })
    }

    pub fn stream_id(&self) -> u32 {
        self.stream_id
    }

    /// Accepts one chunk that arrived at `now_server_us`. Returns whether the
    /// buffer kept it.
    pub fn push(&mut self, seq: u64, pts_us: u64, data: Vec<u8>, now_server_us: i64) -> bool {
        if self.sink_closed {
            return false;
        }
        self.buffer.push(Chunk { seq, pts_us, data }, now_server_us)
    }

    /// Plays everything that is due at `now_server_us`.
    pub fn tick(&mut self, now_server_us: i64) -> TickReport {
        let mut report = TickReport::default();
        if self.sink_closed {
            report.sink_closed = true;
            return report;
        }

        loop {
            match self.buffer.take(now_server_us) {
                Playout::Play(chunk) => {
                    let frame = match self.decoder.decode(&chunk.data, &mut self.pcm) {
                        Ok(_) => &self.pcm,
                        Err(_) => {
                            // A packet that does not decode is concealed like
                            // a packet that never arrived: the time it stood
                            // for still has to pass, or everything after it
                            // plays early.
                            self.decode_errors += 1;
                            self.concealed += 1;
                            report.concealed += 1;
                            &self.silence
                        }
                    };
                    if let Err(err) = self.sink.write_frame(frame) {
                        return self.sink_failed(err, report);
                    }
                    self.played += 1;
                    report.played += 1;
                    self.starved = false;
                    self.last_played_pts = Some(chunk.pts_us);
                }
                Playout::Lost { missing } => {
                    let conceal = missing.min(MAX_CONCEAL);
                    for _ in 0..conceal {
                        if let Err(err) = self.sink.write_frame(&self.silence) {
                            return self.sink_failed(err, report);
                        }
                    }
                    self.concealed += conceal;
                    report.concealed += conceal as u32;
                }
                Playout::Waiting => break,
            }
        }

        // Nothing due. That is fine before the first frame and fine while the
        // next one is merely early; it is an underrun when a frame has played,
        // the buffer is empty, and the time its successor was due has passed.
        if !self.starved && self.buffer.held() == 0 {
            if let Some(last) = self.last_played_pts {
                let next_due =
                    last as i64 + FRAME_MS as i64 * 1000 + self.buffer.target_delay_us() as i64;
                if now_server_us > next_due + UNDERRUN_SLACK_US {
                    self.starved = true;
                    self.underruns += 1;
                    report.underrun = true;
                }
            }
        }
        report
    }

    fn sink_failed(&mut self, err: SinkError, mut report: TickReport) -> TickReport {
        tracing::warn!(stream_id = self.stream_id, %err, "audio sink failed; stream stops");
        self.sink_closed = true;
        report.sink_closed = true;
        report
    }

    /// Chunks the server may still send. Zero once the sink is gone, which is
    /// the only way this protocol has to say "stop".
    pub fn credit(&self) -> u16 {
        if self.sink_closed {
            return 0;
        }
        CREDIT_ROOM.saturating_sub(self.buffer.held().min(u16::MAX as usize) as u16)
    }

    pub fn close(&mut self) {
        let _ = self.sink.close();
        self.sink_closed = true;
    }

    pub fn played(&self) -> u64 {
        self.played
    }
    pub fn concealed(&self) -> u64 {
        self.concealed
    }
    pub fn decode_errors(&self) -> u64 {
        self.decode_errors
    }
    pub fn underruns(&self) -> u64 {
        self.underruns
    }
    pub fn lost(&self) -> u64 {
        self.buffer.lost()
    }
    pub fn dropped_late(&self) -> u64 {
        self.buffer.dropped_late()
    }
    pub fn target_delay_us(&self) -> u32 {
        self.buffer.target_delay_us()
    }
    pub fn jitter_us(&self) -> f64 {
        self.buffer.jitter_us()
    }
    pub fn sink_closed(&self) -> bool {
        self.sink_closed
    }
}

/// What the main loop can tell the playout thread.
#[derive(Debug)]
pub enum PlaybackCommand {
    Open {
        stream_id: u32,
        target_latency_us: u32,
    },
    Chunk {
        stream_id: u32,
        seq: u64,
        pts_us: u64,
        data: Vec<u8>,
    },
    Close {
        stream_id: u32,
    },
    /// Microseconds to add to the client clock to get the server's.
    ClockOffset(i64),
    Shutdown,
}

/// Counters the playout thread publishes and the main loop reads.
#[derive(Debug, Default)]
pub struct PlaybackStats {
    pub credit: AtomicU16,
    pub played: AtomicU64,
    pub concealed: AtomicU64,
    pub underruns: AtomicU64,
    pub lost: AtomicU64,
    pub target_delay_us: AtomicU32,
    pub sink_closed: AtomicBool,
    pub open_stream: AtomicU32,
    pub sink_name: std::sync::Mutex<String>,
}

/// The counters at one instant, in the shape a log line can carry.
///
/// The client writes one of these when a stream closes: every counter above
/// existed before and none of them reached a log, so "did it play" could only
/// be answered by ear.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackSnapshot {
    pub open_stream: u32,
    pub sink_name: String,
    pub played: u64,
    pub concealed: u64,
    pub underruns: u64,
    pub lost: u64,
    pub credit: u16,
    pub target_delay_us: u32,
    pub sink_closed: bool,
}

impl std::fmt::Display for PlaybackSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "stream={} sink={} played={} concealed={} underruns={} lost={} credit={} target_delay_us={} sink_closed={}",
            self.open_stream,
            if self.sink_name.is_empty() { "-" } else { &self.sink_name },
            self.played,
            self.concealed,
            self.underruns,
            self.lost,
            self.credit,
            self.target_delay_us,
            self.sink_closed,
        )
    }
}

impl PlaybackStats {
    /// Reads every counter once, for a log line or a status surface.
    pub fn snapshot(&self) -> PlaybackSnapshot {
        PlaybackSnapshot {
            open_stream: self.open_stream.load(Ordering::Relaxed),
            sink_name: self
                .sink_name
                .lock()
                .map(|name| name.clone())
                .unwrap_or_default(),
            played: self.played.load(Ordering::Relaxed),
            concealed: self.concealed.load(Ordering::Relaxed),
            underruns: self.underruns.load(Ordering::Relaxed),
            lost: self.lost.load(Ordering::Relaxed),
            credit: self.credit.load(Ordering::Relaxed),
            target_delay_us: self.target_delay_us.load(Ordering::Relaxed),
            sink_closed: self.sink_closed.load(Ordering::Relaxed),
        }
    }

    fn publish(&self, playback: Option<&AudioPlayback>) {
        match playback {
            Some(p) => {
                self.credit.store(p.credit(), Ordering::Relaxed);
                self.played.store(p.played(), Ordering::Relaxed);
                self.concealed.store(p.concealed(), Ordering::Relaxed);
                self.underruns.store(p.underruns(), Ordering::Relaxed);
                self.lost.store(p.lost(), Ordering::Relaxed);
                self.target_delay_us
                    .store(p.target_delay_us(), Ordering::Relaxed);
                self.sink_closed.store(p.sink_closed(), Ordering::Relaxed);
                self.open_stream.store(p.stream_id(), Ordering::Relaxed);
            }
            None => {
                self.credit.store(0, Ordering::Relaxed);
                self.open_stream.store(0, Ordering::Relaxed);
            }
        }
    }
}

/// A factory for the sink, run on the playout thread.
///
/// The sink is built where it is used rather than handed across threads, so a
/// native audio stream that is not `Send` — and on some platforms it is not —
/// never has to be.
pub type SinkFactory = Box<dyn FnOnce() -> Result<Box<dyn AudioSink>, SinkError> + Send>;

/// How often the thread looks for due frames while a stream is open.
///
/// A quarter of a frame: fine enough that scheduling slack stays well under
/// the underrun threshold, coarse enough that an open stream costs a few
/// hundred wakeups a second and nothing more.
const TICK: Duration = Duration::from_millis(5);

/// Handle to the playout thread.
pub struct PlaybackThread {
    tx: mpsc::Sender<PlaybackCommand>,
    stats: Arc<PlaybackStats>,
    join: Option<JoinHandle<()>>,
}

impl PlaybackThread {
    /// Starts the thread. The sink is not opened until the first `Open`, so a
    /// client that never receives a stream never touches an audio device.
    pub fn spawn(open_sink: SinkFactory) -> Self {
        let (tx, rx) = mpsc::channel();
        let stats = Arc::new(PlaybackStats::default());
        let thread_stats = Arc::clone(&stats);
        let join = std::thread::Builder::new()
            .name("herdr-audio-playout".into())
            .spawn(move || run(rx, thread_stats, open_sink))
            .ok();
        Self { tx, stats, join }
    }

    pub fn send(&self, command: PlaybackCommand) {
        let _ = self.tx.send(command);
    }

    pub fn stats(&self) -> &Arc<PlaybackStats> {
        &self.stats
    }

    pub fn shutdown(mut self) {
        let _ = self.tx.send(PlaybackCommand::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn run(rx: mpsc::Receiver<PlaybackCommand>, stats: Arc<PlaybackStats>, open_sink: SinkFactory) {
    let mut open_sink = Some(open_sink);
    let mut sink: Option<Box<dyn AudioSink>> = None;
    let mut playback: Option<AudioPlayback> = None;
    let mut offset_us: i64 = 0;
    let mut pending: VecDeque<PlaybackCommand> = VecDeque::new();

    loop {
        // Block outright when nothing is open: an idle client must not wake.
        let command = if playback.is_some() {
            match rx.recv_timeout(TICK) {
                Ok(command) => Some(command),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(command) => Some(command),
                Err(_) => break,
            }
        };

        if let Some(command) = command {
            pending.push_back(command);
            while let Ok(more) = rx.try_recv() {
                pending.push_back(more);
            }
        }

        let now_server_us = now_us() as i64 + offset_us;
        let mut shutdown = false;
        while let Some(command) = pending.pop_front() {
            match command {
                PlaybackCommand::ClockOffset(offset) => offset_us = offset,
                PlaybackCommand::Open {
                    stream_id,
                    target_latency_us,
                } => {
                    if let Some(mut old) = playback.take() {
                        // One sink, one stream. A second open replaces the
                        // first rather than mixing into it.
                        old.close();
                        sink = None;
                    }
                    if sink.is_none() {
                        if let Some(factory) = open_sink.take() {
                            match factory() {
                                Ok(built) => {
                                    if let Ok(mut name) = stats.sink_name.lock() {
                                        *name = built.describe();
                                    }
                                    sink = Some(built);
                                }
                                Err(err) => {
                                    tracing::warn!(%err, "no audio sink; stream declined");
                                    stats.sink_closed.store(true, Ordering::Relaxed);
                                }
                            }
                        }
                    }
                    if let Some(built) = sink.take() {
                        match AudioPlayback::open(stream_id, target_latency_us, built) {
                            Ok(p) => playback = Some(p),
                            Err(err) => {
                                tracing::warn!(%err, "audio decoder failed; stream declined");
                                stats.sink_closed.store(true, Ordering::Relaxed);
                            }
                        }
                    }
                }
                PlaybackCommand::Chunk {
                    stream_id,
                    seq,
                    pts_us,
                    data,
                } => {
                    if let Some(p) = playback.as_mut() {
                        if p.stream_id() == stream_id {
                            p.push(seq, pts_us, data, now_server_us);
                        }
                    }
                }
                PlaybackCommand::Close { stream_id } => {
                    if playback
                        .as_ref()
                        .is_some_and(|p| p.stream_id() == stream_id)
                    {
                        if let Some(mut p) = playback.take() {
                            p.close();
                        }
                    }
                }
                PlaybackCommand::Shutdown => shutdown = true,
            }
        }

        if let Some(p) = playback.as_mut() {
            let report = p.tick(now_server_us);
            if report.sink_closed {
                if let Some(mut dead) = playback.take() {
                    dead.close();
                }
            }
        }
        stats.publish(playback.as_ref());

        if shutdown {
            if let Some(mut p) = playback.take() {
                p.close();
            }
            break;
        }
    }
}

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
        //
        // Ticked at each frame's own moment, the way the thread does. One tick
        // at the end would find seq 0 already past its frame and discard it
        // (the expiry rule in the jitter behaviour) — that test would measure
        // expiry, not concealment.
        let (mut p, frames) = playback();
        let pts0 = 1_000_000;
        p.push(0, pts0, packet(), pts0 as i64);
        let pts2 = pts0 + 2 * FRAME_US;
        p.push(2, pts2, packet(), pts2 as i64);
        let delay = i64::from(p.target_delay_us());

        let first = p.tick(pts0 as i64 + delay);
        assert_eq!(first.played, 1);
        assert_eq!(first.concealed, 0, "nothing is missing yet");

        let report = p.tick(pts2 as i64 + delay + 1);
        assert_eq!(report.played, 1);
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
        let delay = i64::from(p.target_delay_us());

        assert_eq!(p.tick(pts0 as i64 + delay).played, 1);
        let report = p.tick(far as i64 + delay + 1);
        assert_eq!(report.played, 1);
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

    // TP-MEDIA-PLAYBACK-08
    #[test]
    fn the_stats_snapshot_is_one_log_line_with_every_counter_named() {
        let stats = PlaybackStats::default();
        stats.open_stream.store(1, Ordering::Relaxed);
        *stats.sink_name.lock().unwrap() = "ffplay".into();
        stats.played.store(2995, Ordering::Relaxed);
        stats.concealed.store(3, Ordering::Relaxed);
        stats.underruns.store(2, Ordering::Relaxed);
        stats.lost.store(4, Ordering::Relaxed);
        stats.credit.store(7, Ordering::Relaxed);
        stats.target_delay_us.store(100_000, Ordering::Relaxed);
        stats.sink_closed.store(false, Ordering::Relaxed);

        let snapshot = stats.snapshot();
        assert_eq!(
            snapshot.to_string(),
            "stream=1 sink=ffplay played=2995 concealed=3 underruns=2 lost=4 credit=7 target_delay_us=100000 sink_closed=false"
        );

        // A sink that never opened has no name; the line says so instead of
        // leaving an empty field a reader cannot tell from a missing one.
        let fresh = PlaybackStats::default().snapshot();
        assert_eq!(
            fresh.to_string(),
            "stream=0 sink=- played=0 concealed=0 underruns=0 lost=0 credit=0 target_delay_us=0 sink_closed=false"
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
