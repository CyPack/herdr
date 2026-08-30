//! Starting and stopping the capture of one pane's sound.
//!
//! The two sources this owns — the recorder and the graph watcher — both block
//! on purpose: the alternative is polling, and polling bills an idle desktop
//! for a listener who is not there. But the server loop cannot block for even
//! one frame, so each blocking read lives on a thread of its own and reaches
//! the loop through a channel the loop drains without waiting.
//!
//! Three decisions here are worth their reasons.
//!
//! The frame channel is **bounded**. A recorder produces 7680 bytes every
//! twenty milliseconds — 384 KB a second for one pane — so an unbounded
//! channel turns a stalled loop into growing memory. When the channel is full
//! the frame is **dropped** and counted, never waited on: a blocking send would
//! back up into the recorder's pipe, and what comes back after that is not
//! silence but sound with accumulated delay, which is worse. This is not a new
//! policy — `Runtime::offer` already drops late chunks at the source.
//!
//! A reader thread is never joined from the loop. Joining is waiting, and this
//! is the one place that cannot wait; finished threads are reaped when they are
//! noticed instead.
//!
//! Nothing here starts before a listener exists. That is the resource doctrine
//! at its plainest, and its violation is silent: a watcher nobody asked for
//! looks exactly like a watcher somebody did.
//!
//! TP-MEDIA-SUPERVISE-01.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError, TrySendError};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Instant;

use crate::app::pane_audio_source::{Debouncer, GRAPH_QUIET};
use crate::platform::{AudioSourceError, FrameSource, GraphSignals};

/// How many frames may wait for the server loop before one is dropped.
///
/// Eight frames is 160 ms. Long enough to ride out a loop that is busy drawing,
/// short enough that a listener never hears the queue instead of the sound.
const FRAME_QUEUE: usize = 8;

/// How a capture is started, behind a boxed function so a test can hand the
/// supervisor a source it wrote. The shipped one is `platform::capture_stream`;
/// asserting against the real recorder would be asserting about the machine.
type CaptureFn = Box<dyn Fn(u32) -> Option<Result<Box<dyn FrameSource>, AudioSourceError>> + Send>;

/// The same seam for the graph watcher.
type WatcherFn = Box<dyn Fn() -> Option<Result<Box<dyn GraphSignals>, AudioSourceError>> + Send>;

/// What the server loop should do about a pane, in the order it must do it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CaptureEvent {
    /// The pane's first whole frame. The stream is opened *now*, with this
    /// frame in hand, because the server counts a chunk's timestamp from the
    /// open: opening earlier and waiting for sound makes every frame late.
    Opened { pane_id: String, frame: Vec<u8> },
    /// A further frame for a pane whose stream is already open.
    Frame { pane_id: String, frame: Vec<u8> },
    /// The pane's source ended, on its own or because it was stopped.
    Ended { pane_id: String },
}

struct PaneCapture {
    frames: Receiver<Vec<u8>>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
    /// False until a whole frame has been handed to the loop. The pre-roll rule
    /// lives in this one bit.
    opened: bool,
}

struct WatcherHandle {
    signals: Receiver<()>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

pub(crate) struct CaptureSupervisor {
    capture: CaptureFn,
    watcher_factory: WatcherFn,
    watcher: Option<WatcherHandle>,
    panes: BTreeMap<String, PaneCapture>,
    debouncer: Debouncer,
    dropped: Arc<AtomicU64>,
}

impl std::fmt::Debug for CaptureSupervisor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CaptureSupervisor")
            .field("panes", &self.panes.len())
            .field("watching", &self.watcher.is_some())
            .field("dropped", &self.dropped.load(Ordering::Relaxed))
            .finish()
    }
}

impl CaptureSupervisor {
    /// The shipped supervisor, wired to this platform.
    pub(crate) fn new() -> Self {
        Self::with_sources(
            Box::new(crate::platform::capture_stream),
            Box::new(crate::platform::start_graph_watcher),
        )
    }

    /// The seam a test uses to drive sources it controls.
    pub(crate) fn with_sources(capture: CaptureFn, watcher_factory: WatcherFn) -> Self {
        Self {
            capture,
            watcher_factory,
            watcher: None,
            panes: BTreeMap::new(),
            debouncer: Debouncer::new(GRAPH_QUIET),
            dropped: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Begins capturing one pane. Starting a pane that is already captured does
    /// nothing, which is what makes it safe to call from a plan recomputed on
    /// every event.
    pub(crate) fn start(
        &mut self,
        pane_id: &str,
        object_serial: u32,
    ) -> Result<(), AudioSourceError> {
        if self.panes.contains_key(pane_id) {
            return Ok(());
        }
        let mut source = match (self.capture)(object_serial) {
            Some(source) => source?,
            None => {
                return Err(AudioSourceError::Unavailable(
                    "this platform cannot capture audio".to_owned(),
                ))
            }
        };
        let (tx, frames) = sync_channel::<Vec<u8>>(FRAME_QUEUE);
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let dropped = Arc::clone(&self.dropped);
        let join = std::thread::Builder::new()
            .name(format!("pane-audio-{pane_id}"))
            .spawn(move || {
                while !thread_stop.load(Ordering::Acquire) {
                    match source.next_frame() {
                        Ok(Some(frame)) => match tx.try_send(frame) {
                            Ok(()) => {}
                            // The loop is behind. A dropped frame costs one
                            // gap; a waited-on frame costs every frame after it.
                            Err(TrySendError::Full(_)) => {
                                dropped.fetch_add(1, Ordering::Relaxed);
                            }
                            // Nobody is listening any more.
                            Err(TrySendError::Disconnected(_)) => break,
                        },
                        // The recorder ended. Dropping the sender is what tells
                        // the loop, so there is nothing else to say.
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
                // The thread owns the source, so the thread closes it. Leaving
                // that to a drop somewhere else is how a recorder outlives the
                // pane that wanted it.
                let _ = source.close();
            })
            .map_err(|err| AudioSourceError::Unavailable(format!("capture thread: {err}")))?;
        self.panes.insert(
            pane_id.to_owned(),
            PaneCapture {
                frames,
                stop,
                join: Some(join),
                opened: false,
            },
        );
        Ok(())
    }

    /// Ends one pane's capture. The thread notices within a frame and closes
    /// the source itself; nothing here waits for it.
    pub(crate) fn stop(&mut self, pane_id: &str) {
        if let Some(pane) = self.panes.remove(pane_id) {
            pane.stop.store(true, Ordering::Release);
            drop(pane.frames);
            drop(pane.join);
        }
    }

    /// Ends everything, including the watcher. What the last listener leaving
    /// costs is nothing at all.
    pub(crate) fn stop_all(&mut self) {
        let panes: Vec<String> = self.panes.keys().cloned().collect();
        for pane_id in panes {
            self.stop(&pane_id);
        }
        self.unwatch();
    }

    /// Starts the graph watcher if it is not already running.
    pub(crate) fn watch(&mut self) -> Result<(), AudioSourceError> {
        if self.watcher.is_some() {
            return Ok(());
        }
        let mut watcher = match (self.watcher_factory)() {
            Some(watcher) => watcher?,
            None => {
                return Err(AudioSourceError::Unavailable(
                    "this platform has no graph watcher".to_owned(),
                ))
            }
        };
        let (tx, signals) = std::sync::mpsc::channel::<()>();
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let join = std::thread::Builder::new()
            .name("pane-audio-graph".to_owned())
            .spawn(move || {
                while !thread_stop.load(Ordering::Acquire) {
                    match watcher.next_signal() {
                        Ok(true) => {
                            if tx.send(()).is_err() {
                                break;
                            }
                        }
                        Ok(false) | Err(_) => break,
                    }
                }
                let _ = watcher.close();
            })
            .map_err(|err| AudioSourceError::Unavailable(format!("watch thread: {err}")))?;
        self.watcher = Some(WatcherHandle {
            signals,
            stop,
            join: Some(join),
        });
        Ok(())
    }

    /// Stops the graph watcher.
    pub(crate) fn unwatch(&mut self) {
        if let Some(watcher) = self.watcher.take() {
            watcher.stop.store(true, Ordering::Release);
            drop(watcher.signals);
            drop(watcher.join);
        }
    }

    pub(crate) fn is_watching(&self) -> bool {
        self.watcher.is_some()
    }

    /// Whether the graph has moved and then settled, which is the only moment
    /// worth re-reading it. Never blocks.
    pub(crate) fn graph_settled(&mut self, now: Instant) -> bool {
        if let Some(watcher) = self.watcher.as_ref() {
            loop {
                match watcher.signals.try_recv() {
                    Ok(()) => self.debouncer.signal(now),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => break,
                }
            }
        }
        self.debouncer.take_due(now)
    }

    /// Takes everything the reader threads have produced. Never blocks, and
    /// never waits on a thread: the server loop is the one place that cannot.
    pub(crate) fn drain(&mut self) -> Vec<CaptureEvent> {
        let mut events = Vec::new();
        let mut ended = Vec::new();
        for (pane_id, pane) in self.panes.iter_mut() {
            loop {
                match pane.frames.try_recv() {
                    Ok(frame) => {
                        if pane.opened {
                            events.push(CaptureEvent::Frame {
                                pane_id: pane_id.clone(),
                                frame,
                            });
                        } else {
                            pane.opened = true;
                            events.push(CaptureEvent::Opened {
                                pane_id: pane_id.clone(),
                                frame,
                            });
                        }
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        ended.push(pane_id.clone());
                        break;
                    }
                }
            }
        }
        for pane_id in ended {
            self.panes.remove(&pane_id);
            events.push(CaptureEvent::Ended { pane_id });
        }
        events
    }

    /// Frames the loop was too slow to take. A number that climbs is a report
    /// about the loop, not about the sound.
    pub(crate) fn dropped_frames(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub(crate) fn active_panes(&self) -> usize {
        self.panes.len()
    }

    /// The panes being captured right now, which is what the plan compares
    /// against. Owned rather than borrowed so the caller can keep planning
    /// while it starts and stops.
    pub(crate) fn captured_panes(&self) -> BTreeSet<String> {
        self.panes.keys().cloned().collect()
    }
}

impl Drop for CaptureSupervisor {
    fn drop(&mut self) {
        self.stop_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::time::Duration;

    /// A source that hands over a written list of frames and then ends.
    ///
    /// Nothing here needs a recorder, so these run on a build box with no sound
    /// server: a test that needed pw-record would be a test about the machine.
    struct ScriptedSource {
        frames: VecDeque<Vec<u8>>,
        closed: Arc<AtomicU64>,
    }

    impl FrameSource for ScriptedSource {
        fn next_frame(&mut self) -> Result<Option<Vec<u8>>, AudioSourceError> {
            Ok(self.frames.pop_front())
        }

        fn close(&mut self) -> Result<(), AudioSourceError> {
            self.closed.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    /// A watcher that reports a written number of changes and then ends.
    struct ScriptedWatcher {
        remaining: u64,
    }

    impl GraphSignals for ScriptedWatcher {
        fn next_signal(&mut self) -> Result<bool, AudioSourceError> {
            if self.remaining == 0 {
                return Ok(false);
            }
            self.remaining -= 1;
            Ok(true)
        }

        fn close(&mut self) -> Result<(), AudioSourceError> {
            Ok(())
        }
    }

    fn frame(fill: u8) -> Vec<u8> {
        vec![fill; crate::app::pane_audio::FRAME_BYTES]
    }

    /// Counts how many times a capture was asked for, so a test can prove that
    /// nothing was started rather than that nothing was heard.
    fn counting_capture(
        frames: usize,
        starts: Arc<AtomicU64>,
        closed: Arc<AtomicU64>,
    ) -> CaptureFn {
        Box::new(move |_serial| {
            starts.fetch_add(1, Ordering::Relaxed);
            let scripted = ScriptedSource {
                frames: (0..frames).map(|i| frame(i as u8)).collect(),
                closed: Arc::clone(&closed),
            };
            Some(Ok(Box::new(scripted) as Box<dyn FrameSource>))
        })
    }

    fn no_watcher() -> WatcherFn {
        Box::new(|| {
            Some(Err(AudioSourceError::Unavailable(
                "not in this test".to_owned(),
            )))
        })
    }

    /// Waits until a condition holds, with a ceiling rather than a guess.
    ///
    /// The assertions themselves are exact — this only gives the reader thread
    /// room to finish, and fails loudly instead of asserting on a race.
    fn until(mut done: impl FnMut() -> bool) {
        for _ in 0..2_000 {
            if done() {
                return;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        panic!("the reader thread never finished");
    }

    #[test]
    fn nothing_is_started_until_a_pane_is_captured() {
        // SUP-1. The doctrine's plainest rule, and the one whose violation is
        // invisible: a recorder nobody asked for looks like one somebody did.
        let starts = Arc::new(AtomicU64::new(0));
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(0, Arc::clone(&starts), Arc::new(AtomicU64::new(0))),
            no_watcher(),
        );
        for _ in 0..50 {
            assert!(supervisor.drain().is_empty());
        }
        assert_eq!(starts.load(Ordering::Relaxed), 0);
        assert_eq!(supervisor.active_panes(), 0);
        assert!(!supervisor.is_watching());
    }

    #[test]
    fn the_first_frame_opens_and_the_rest_do_not() {
        // PRE-1 and PRE-2. The open carries the first frame because the server
        // dates every chunk from the open: opening early and waiting for sound
        // makes every frame late, which was measured as played=0.
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(3, Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0))),
            no_watcher(),
        );
        supervisor.start("w1:pA", 1).expect("start");
        let mut opened = 0;
        let mut frames = 0;
        until(|| {
            for event in supervisor.drain() {
                match event {
                    CaptureEvent::Opened { frame, .. } => {
                        assert_eq!(frames, 0, "a frame arrived before the open");
                        assert_eq!(frame.len(), crate::app::pane_audio::FRAME_BYTES);
                        opened += 1;
                    }
                    CaptureEvent::Frame { frame, .. } => {
                        assert_eq!(opened, 1, "a frame arrived before the open");
                        assert_eq!(frame.len(), crate::app::pane_audio::FRAME_BYTES);
                        frames += 1;
                    }
                    CaptureEvent::Ended { .. } => return true,
                }
            }
            false
        });
        assert_eq!(opened, 1, "exactly one open");
        assert_eq!(frames, 2, "the remaining frames are not opens");
    }

    #[test]
    fn a_frame_is_a_whole_protocol_frame() {
        // SUP-6. The runtime takes samples, not bytes, and refuses any body of
        // the wrong length — so the wrong length here is silence, not an error
        // anybody sees.
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(1, Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0))),
            no_watcher(),
        );
        supervisor.start("w1:pA", 1).expect("start");
        let mut checked = false;
        until(|| {
            for event in supervisor.drain() {
                if let CaptureEvent::Opened { frame, .. } = event {
                    let pcm = crate::app::pane_audio::pcm_from_f32le(&frame)
                        .expect("the runtime accepts a captured frame");
                    assert_eq!(pcm.len(), crate::media::FRAME_SAMPLES * 2);
                    checked = true;
                }
            }
            checked
        });
    }

    #[test]
    fn a_source_that_ends_ends_its_pane_and_leaves_the_others() {
        // SUP-3. One pane's failure is not the others'.
        let mut supervisor = CaptureSupervisor::with_sources(
            Box::new(|serial| {
                let frames = if serial == 1 { 0 } else { 4 };
                Some(Ok(Box::new(ScriptedSource {
                    frames: (0..frames).map(|i| frame(i as u8)).collect(),
                    closed: Arc::new(AtomicU64::new(0)),
                }) as Box<dyn FrameSource>))
            }),
            no_watcher(),
        );
        supervisor.start("w1:silent", 1).expect("start");
        supervisor.start("w1:loud", 2).expect("start");
        let mut ended = Vec::new();
        let mut loud_frames = 0;
        until(|| {
            for event in supervisor.drain() {
                match event {
                    CaptureEvent::Ended { pane_id } => ended.push(pane_id),
                    CaptureEvent::Opened { pane_id, .. } | CaptureEvent::Frame { pane_id, .. } => {
                        assert_eq!(pane_id, "w1:loud");
                        loud_frames += 1;
                    }
                }
            }
            ended.len() == 2
        });
        assert_eq!(loud_frames, 4, "the live pane delivered everything");
        assert!(ended.contains(&"w1:silent".to_owned()));
        assert_eq!(supervisor.active_panes(), 0);
    }

    #[test]
    fn a_pane_can_be_captured_again_after_it_stops() {
        // SUP-2. A one-shot mistake in this family has been measured before.
        let starts = Arc::new(AtomicU64::new(0));
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(2, Arc::clone(&starts), Arc::new(AtomicU64::new(0))),
            no_watcher(),
        );
        supervisor.start("w1:pA", 1).expect("first start");
        supervisor.stop("w1:pA");
        assert_eq!(supervisor.active_panes(), 0);
        supervisor.start("w1:pA", 1).expect("second start");
        let mut delivered = 0;
        until(|| {
            for event in supervisor.drain() {
                if matches!(
                    event,
                    CaptureEvent::Opened { .. } | CaptureEvent::Frame { .. }
                ) {
                    delivered += 1;
                }
            }
            delivered == 2
        });
        assert_eq!(starts.load(Ordering::Relaxed), 2, "both starts happened");
    }

    #[test]
    fn starting_a_captured_pane_again_starts_nothing() {
        let starts = Arc::new(AtomicU64::new(0));
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(4, Arc::clone(&starts), Arc::new(AtomicU64::new(0))),
            no_watcher(),
        );
        supervisor.start("w1:pA", 1).expect("start");
        supervisor.start("w1:pA", 1).expect("start again");
        assert_eq!(starts.load(Ordering::Relaxed), 1);
        assert_eq!(supervisor.active_panes(), 1);
    }

    #[test]
    fn a_full_queue_drops_frames_instead_of_waiting() {
        // SUP-5. Exact rather than timed: every frame the source produced was
        // either delivered or dropped, whatever the scheduler did in between. A
        // blocking send would instead have hung this test, which is the point.
        let produced = 200;
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(
                produced,
                Arc::new(AtomicU64::new(0)),
                Arc::new(AtomicU64::new(0)),
            ),
            no_watcher(),
        );
        supervisor.start("w1:pA", 1).expect("start");
        let mut delivered = 0_u64;
        let mut ended = false;
        until(|| {
            for event in supervisor.drain() {
                match event {
                    CaptureEvent::Ended { .. } => ended = true,
                    _ => delivered += 1,
                }
            }
            ended
        });
        let dropped = supervisor.dropped_frames();
        assert!(dropped > 0, "a queue of {FRAME_QUEUE} could not hold 200");
        assert_eq!(
            delivered + dropped,
            produced as u64,
            "every frame was either delivered or dropped, none waited on"
        );
    }

    #[test]
    fn stopping_closes_the_source_the_thread_owns() {
        // The orphan rule, one layer up: the thread that owns the recorder is
        // the thread that closes it.
        let closed = Arc::new(AtomicU64::new(0));
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(1, Arc::new(AtomicU64::new(0)), Arc::clone(&closed)),
            no_watcher(),
        );
        supervisor.start("w1:pA", 1).expect("start");
        until(|| closed.load(Ordering::Relaxed) == 1);
    }

    #[test]
    fn the_graph_settles_once_after_a_burst() {
        // WCH-2 at the supervisor's level: many signals, one look.
        let base = Instant::now();
        let mut supervisor = CaptureSupervisor::with_sources(
            counting_capture(0, Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0))),
            Box::new(|| {
                Some(Ok(
                    Box::new(ScriptedWatcher { remaining: 12 }) as Box<dyn GraphSignals>
                ))
            }),
        );
        supervisor.watch().expect("watch");
        assert!(supervisor.is_watching());
        until(|| {
            supervisor.graph_settled(base);
            supervisor.graph_settled(base + GRAPH_QUIET + Duration::from_millis(1))
        });
        // The burst is spent: quiet does not keep asking.
        assert!(!supervisor.graph_settled(base + Duration::from_secs(10)));
        supervisor.unwatch();
        assert!(!supervisor.is_watching());
    }

    #[test]
    fn a_platform_without_capture_is_an_error_not_a_panic() {
        let mut supervisor = CaptureSupervisor::with_sources(Box::new(|_| None), Box::new(|| None));
        let err = supervisor.start("w1:pA", 1).expect_err("no capture here");
        assert!(matches!(err, AudioSourceError::Unavailable(_)), "{err}");
        let err = supervisor.watch().expect_err("no watcher here");
        assert!(matches!(err, AudioSourceError::Unavailable(_)), "{err}");
        assert_eq!(supervisor.active_panes(), 0);
    }
}
