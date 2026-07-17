//! Lightweight opt-in render profiling for local performance investigations.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const ENV_VAR: &str = "HERDR_RENDER_PROF";
const MAX_METRIC_LABELS_PER_KIND: usize = 128;

static ENABLED: OnceLock<bool> = OnceLock::new();
static PROFILER: OnceLock<Mutex<RenderProfiler>> = OnceLock::new();

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
    with_profiler(|profiler| profiler.increment(name, value));
}

pub(crate) fn event(name: &'static str) {
    counter(name, 1);
}

pub(crate) fn duration(name: &'static str, duration: Duration) {
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

pub(crate) fn flush_if_due() {
    with_profiler(RenderProfiler::flush_if_due);
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
