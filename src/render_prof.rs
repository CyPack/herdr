//! Lightweight opt-in render profiling for local performance investigations.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
#[cfg(test)]
use std::{cell::RefCell, thread::LocalKey};

const ENV_VAR: &str = "HERDR_RENDER_PROF";
const MAX_METRIC_LABELS_PER_KIND: usize = 128;

static ENABLED: OnceLock<bool> = OnceLock::new();
static PROFILER: OnceLock<Mutex<RenderProfiler>> = OnceLock::new();
#[cfg(test)]
thread_local! {
    static TEST_PROFILER: RefCell<Option<RenderProfiler>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct DurationStats {
    count: u64,
    total_ns: u128,
    max_ns: u128,
}

struct RenderProfiler {
    window_started: Instant,
    counters: BTreeMap<&'static str, u64>,
    durations: BTreeMap<&'static str, DurationStats>,
    dropped_counter_labels: u64,
    dropped_duration_labels: u64,
}

impl RenderProfiler {
    fn new() -> Self {
        Self {
            window_started: Instant::now(),
            counters: BTreeMap::new(),
            durations: BTreeMap::new(),
            dropped_counter_labels: 0,
            dropped_duration_labels: 0,
        }
    }

    fn increment(&mut self, name: &'static str, value: u64) {
        if let Some(counter) = self.counters.get_mut(name) {
            *counter = counter.saturating_add(value);
        } else if self.counters.len() < MAX_METRIC_LABELS_PER_KIND {
            self.counters.insert(name, value);
        } else {
            self.dropped_counter_labels = self.dropped_counter_labels.saturating_add(1);
        }
    }

    fn duration(&mut self, name: &'static str, duration: Duration) {
        if !self.durations.contains_key(name) && self.durations.len() >= MAX_METRIC_LABELS_PER_KIND
        {
            self.dropped_duration_labels = self.dropped_duration_labels.saturating_add(1);
            return;
        }
        let stats = self.durations.entry(name).or_default();
        let ns = duration.as_nanos();
        stats.count = stats.count.saturating_add(1);
        stats.total_ns = stats.total_ns.saturating_add(ns);
        stats.max_ns = stats.max_ns.max(ns);
    }

    fn flush_if_due(&mut self) {
        let elapsed = self.window_started.elapsed();
        if elapsed < Duration::from_secs(1) {
            return;
        }

        let counters = self
            .counters
            .iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        let durations = self
            .durations
            .iter()
            .map(|(name, stats)| {
                let avg_us = if stats.count == 0 {
                    0
                } else {
                    stats.total_ns / u128::from(stats.count) / 1_000
                };
                let max_us = stats.max_ns / 1_000;
                format!(
                    "{name}=count:{} avg_us:{} max_us:{}",
                    stats.count, avg_us, max_us
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        tracing::info!(
            event = "render.prof",
            window_ms = elapsed.as_millis() as u64,
            counters = %counters,
            durations = %durations,
            dropped_counter_labels = self.dropped_counter_labels,
            dropped_duration_labels = self.dropped_duration_labels,
            "render profiler window"
        );

        self.window_started = Instant::now();
        self.counters.clear();
        self.durations.clear();
        self.dropped_counter_labels = 0;
        self.dropped_duration_labels = 0;
    }
}

pub(crate) fn enabled() -> bool {
    *ENABLED.get_or_init(|| {
        std::env::var(ENV_VAR)
            .map(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
    })
}

fn with_profiler(update: impl FnOnce(&mut RenderProfiler)) {
    if !enabled() {
        return;
    }
    let profiler = PROFILER.get_or_init(|| Mutex::new(RenderProfiler::new()));
    if let Ok(mut profiler) = profiler.lock() {
        update(&mut profiler);
    }
}

pub(crate) fn counter(name: &'static str, value: u64) {
    if value == 0 {
        return;
    }
    #[cfg(test)]
    with_test_profiler(|profiler| profiler.increment(name, value));
    with_profiler(|profiler| profiler.increment(name, value));
}

pub(crate) fn event(name: &'static str) {
    counter(name, 1);
}

pub(crate) fn duration(name: &'static str, duration: Duration) {
    #[cfg(test)]
    with_test_profiler(|profiler| profiler.duration(name, duration));
    with_profiler(|profiler| profiler.duration(name, duration));
}

pub(crate) fn timer() -> Option<Instant> {
    enabled().then(Instant::now)
}

pub(crate) fn duration_since(name: &'static str, started: Option<Instant>) {
    if let Some(started) = started {
        duration(name, started.elapsed());
    }
}

pub(crate) struct DurationGuard {
    name: &'static str,
    started: Option<Instant>,
}

impl Drop for DurationGuard {
    fn drop(&mut self) {
        duration_since(self.name, self.started);
    }
}

pub(crate) fn duration_guard(name: &'static str) -> DurationGuard {
    let active = enabled();
    #[cfg(test)]
    let active = active || test_profiler_active();
    DurationGuard {
        name,
        started: active.then(Instant::now),
    }
}

pub(crate) fn flush_if_due() {
    with_profiler(RenderProfiler::flush_if_due);
}

#[cfg(test)]
fn test_profiler_active() -> bool {
    TEST_PROFILER.with(|profiler| profiler.borrow().is_some())
}

#[cfg(test)]
fn with_test_profiler(update: impl FnOnce(&mut RenderProfiler)) {
    TEST_PROFILER.with(|profiler| {
        if let Some(profiler) = profiler.borrow_mut().as_mut() {
            update(profiler);
        }
    });
}

#[cfg(test)]
pub(crate) struct TestRenderProfile {
    counters: BTreeMap<&'static str, u64>,
    durations: BTreeMap<&'static str, DurationStats>,
}

#[cfg(test)]
impl TestRenderProfile {
    pub(crate) fn counter(&self, name: &'static str) -> u64 {
        self.counters.get(name).copied().unwrap_or(0)
    }

    pub(crate) fn duration_count(&self, name: &'static str) -> u64 {
        self.durations
            .get(name)
            .map(|stats| stats.count)
            .unwrap_or(0)
    }
}

#[cfg(test)]
struct TestProfilerGuard(&'static LocalKey<RefCell<Option<RenderProfiler>>>);

#[cfg(test)]
impl Drop for TestProfilerGuard {
    fn drop(&mut self) {
        self.0.with(|profiler| {
            profiler.borrow_mut().take();
        });
    }
}

#[cfg(test)]
pub(crate) fn observe_for_test<T>(work: impl FnOnce() -> T) -> (T, TestRenderProfile) {
    TEST_PROFILER.with(|profiler| {
        assert!(
            profiler
                .borrow_mut()
                .replace(RenderProfiler::new())
                .is_none(),
            "render profile observers cannot nest on one test thread"
        );
    });
    let guard = TestProfilerGuard(&TEST_PROFILER);
    let value = work();
    let profiler = TEST_PROFILER.with(|profiler| {
        profiler
            .borrow_mut()
            .take()
            .expect("scoped render profiler remains installed")
    });
    drop(guard);
    (
        value,
        TestRenderProfile {
            counters: profiler.counters,
            durations: profiler.durations,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::Arc;

    #[derive(Clone, Default)]
    struct SharedLogWriter(Arc<Mutex<Vec<u8>>>);

    struct LockedLogWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for LockedLogWriter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0
                .lock()
                .expect("profile log buffer lock")
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::writer::MakeWriter<'a> for SharedLogWriter {
        type Writer = LockedLogWriter;

        fn make_writer(&'a self) -> Self::Writer {
            LockedLogWriter(self.0.clone())
        }
    }

    #[test]
    fn render_profiler_is_bounded_and_resettable() {
        const EXPECTED_MAX_LABELS_PER_KIND: usize = 128;
        let mut profiler = RenderProfiler::new();

        for index in 0..(EXPECTED_MAX_LABELS_PER_KIND * 2) {
            let counter_name: &'static str =
                Box::leak(format!("test.counter.{index}").into_boxed_str());
            let duration_name: &'static str =
                Box::leak(format!("test.duration.{index}").into_boxed_str());
            profiler.increment(counter_name, 1);
            profiler.duration(duration_name, Duration::from_micros(index as u64));
        }

        assert!(
            profiler.counters.len() <= EXPECTED_MAX_LABELS_PER_KIND,
            "counter labels must stay bounded, got {}",
            profiler.counters.len()
        );
        assert!(
            profiler.durations.len() <= EXPECTED_MAX_LABELS_PER_KIND,
            "duration labels must stay bounded, got {}",
            profiler.durations.len()
        );
        assert_eq!(profiler.dropped_counter_labels, 128);
        assert_eq!(profiler.dropped_duration_labels, 128);

        profiler.window_started = Instant::now() - Duration::from_secs(2);
        profiler.flush_if_due();
        assert!(profiler.counters.is_empty());
        assert!(profiler.durations.is_empty());
        assert_eq!(profiler.dropped_counter_labels, 0);
        assert_eq!(profiler.dropped_duration_labels, 0);
    }

    #[test]
    fn render_profiler_flush_reports_duration_p95() {
        let writer = SharedLogWriter::default();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(writer.clone())
            .finish();
        let mut profiler = RenderProfiler::new();
        for duration_us in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512] {
            profiler.duration("test.duration", Duration::from_micros(duration_us));
        }
        profiler.window_started = Instant::now() - Duration::from_secs(2);

        tracing::subscriber::with_default(subscriber, || profiler.flush_if_due());

        let output = String::from_utf8(writer.0.lock().expect("profile log buffer lock").clone())
            .expect("profile output is UTF-8");
        assert!(
            output.contains("p95_us:525"),
            "duration telemetry must expose the bounded log2-bucket p95 estimate: {output}"
        );
    }

    #[test]
    fn test_render_profile_observer_is_scoped_and_thread_local() {
        let (_, first) = observe_for_test(|| {
            event("test.outer");
            counter("test.outer", 2);
            duration("test.duration", Duration::from_micros(7));
            std::thread::spawn(|| event("test.other_thread"))
                .join()
                .expect("test observer thread");
        });
        assert_eq!(first.counter("test.outer"), 3);
        assert_eq!(first.counter("test.other_thread"), 0);
        assert_eq!(first.duration_count("test.duration"), 1);

        let (_, second) = observe_for_test(|| {});
        assert_eq!(second.counter("test.outer"), 0);
        assert_eq!(second.duration_count("test.duration"), 0);
    }
}
