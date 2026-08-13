//! Machine resource samples, and the pure arithmetic that turns raw counters
//! into something a bar section can show.
//!
//! Everything here is a pure function of text and numbers. The reading of
//! actual files lives in `crate::platform`, because that is where this
//! codebase keeps OS behaviour, and because a parser that owns its own file
//! handle can only be tested on a machine that happens to have the file.
//! Splitting them means the arithmetic — which is where the mistakes are — is
//! tested against fixtures on every platform, including the ones that have no
//! `/proc` at all.
//!
//! The other half of the design is the word `Option`. A counter that could not
//! be read is `None`, never zero. A silent zero is indistinguishable from an
//! idle machine, and a meter that reads "0%" while it is actually broken is
//! worse than one that admits it does not know.

/// The two numbers a CPU percentage is made of.
///
/// A percentage needs two of these taken some time apart: the kernel exposes
/// cumulative time since boot, so a single reading says only what the machine
/// has averaged over its whole uptime, which is never what somebody watching a
/// bar wants to know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct CpuTimes {
    /// Every jiffy the kernel has accounted for, idle included.
    pub(crate) total: u64,
    /// The jiffies that were spent doing nothing — idle plus iowait.
    pub(crate) idle: u64,
}

/// Used and total for one pool of memory, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Usage {
    pub(crate) used: u64,
    pub(crate) total: u64,
}

/// One reading of the machine. Any field may be missing on its own: a kernel
/// that reports memory but not swap is a normal machine, not a broken one.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ResourceSample {
    pub(crate) cpu: Option<f32>,
    pub(crate) mem: Option<Usage>,
    pub(crate) swap: Option<Usage>,
}

/// Which number a section shows. Closed, because config names map onto it and
/// an open set would let a typo become a silently blank section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceMetric {
    Cpu,
    Mem,
    Swap,
}

impl ResourceMetric {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "cpu" => Some(Self::Cpu),
            "mem" | "ram" => Some(Self::Mem),
            "swap" => Some(Self::Swap),
            _ => None,
        }
    }

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Cpu => "CPU",
            Self::Mem => "MEM",
            Self::Swap => "SWP",
        }
    }
}

/// Reads the aggregate `cpu` line of `/proc/stat`.
///
/// The line is `cpu user nice system idle iowait irq softirq steal ...`, and
/// the count of fields has grown across kernel releases, so this sums whatever
/// is there rather than indexing a fixed tail. `iowait` counts as idle: a core
/// waiting on a disk is not doing work, and calling that busy makes the meter
/// jump every time something touches storage.
// TP-RES-01: the aggregate line is summed, not indexed by a fixed arity.
//
// Only the Linux reader calls this, but it is compiled and tested everywhere on
// purpose: keeping the arithmetic platform-independent is what lets a Windows or
// macOS `just check` catch a mistake in it. Deleting it from those targets would
// hide the tests that guard it behind the one platform least likely to run them.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_proc_stat(text: &str) -> Option<CpuTimes> {
    let line = text.lines().find(|line| {
        line.strip_prefix("cpu")
            .is_some_and(|rest| rest.starts_with(char::is_whitespace))
    })?;
    let mut fields = line.split_whitespace();
    fields.next()?;

    let mut total: u64 = 0;
    let mut idle: u64 = 0;
    let mut seen = 0usize;
    for (index, field) in fields.enumerate() {
        let value: u64 = field.parse().ok()?;
        total = total.checked_add(value)?;
        // Fields 3 and 4 after the label are idle and iowait.
        if index == 3 || index == 4 {
            idle = idle.checked_add(value)?;
        }
        seen += 1;
    }
    // Anything shorter than user/nice/system/idle is not the line we want.
    if seen < 4 {
        return None;
    }
    Some(CpuTimes { total, idle })
}

/// The share of the interval between two readings that was spent busy.
///
/// Returns `None` when the two readings cannot produce an answer: no time
/// passed between them, or the counters went backwards, which happens when the
/// previous reading came from before a suspend. Returning zero in those cases
/// would draw an idle machine, and returning a number computed from a negative
/// interval would draw nonsense.
// TP-RES-02: zero elapsed and backwards counters both refuse rather than divide.
pub(crate) fn cpu_percent(prev: CpuTimes, now: CpuTimes) -> Option<f32> {
    let total_delta = now.total.checked_sub(prev.total)?;
    let idle_delta = now.idle.checked_sub(prev.idle)?;
    if total_delta == 0 || idle_delta > total_delta {
        return None;
    }
    let busy = total_delta - idle_delta;
    // The cast is lossy above 2^24 jiffies of delta, which at 100 Hz is about
    // two days inside a single sampling interval. A meter is allowed to be
    // approximate; it is not allowed to panic or to wrap.
    #[allow(clippy::cast_precision_loss)]
    Some((busy as f32 / total_delta as f32) * 100.0)
}

/// Reads memory and swap out of `/proc/meminfo`.
///
/// `used` is derived from `MemAvailable`, not from `MemFree`. Free memory on a
/// healthy Linux box is nearly zero because the kernel spends it all on cache,
/// so a meter built on `MemFree` reads 97% used on an idle machine and teaches
/// its owner to ignore it. `MemAvailable` is the kernel's own estimate of what
/// a new workload could actually get, which is the number a person means.
// TP-RES-03: memory is derived from MemAvailable, and kB are converted to bytes.
//
// Compiled on every target for the same reason as `parse_proc_stat`: the tests
// that pin this arithmetic have to run wherever `just check` runs.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_proc_meminfo(text: &str) -> (Option<Usage>, Option<Usage>) {
    let mut mem_total = None;
    let mut mem_available = None;
    let mut swap_total = None;
    let mut swap_free = None;

    for line in text.lines() {
        let Some((key, rest)) = line.split_once(':') else {
            continue;
        };
        let Some(value) = rest.split_whitespace().next() else {
            continue;
        };
        let Ok(kb) = value.parse::<u64>() else {
            continue;
        };
        let bytes = kb.saturating_mul(1024);
        match key {
            "MemTotal" => mem_total = Some(bytes),
            "MemAvailable" => mem_available = Some(bytes),
            "SwapTotal" => swap_total = Some(bytes),
            "SwapFree" => swap_free = Some(bytes),
            _ => {}
        }
    }

    let mem = match (mem_total, mem_available) {
        (Some(total), Some(available)) => Some(Usage {
            used: total.saturating_sub(available),
            total,
        }),
        _ => None,
    };
    let swap = match (swap_total, swap_free) {
        (Some(total), Some(free)) => Some(Usage {
            used: total.saturating_sub(free),
            total,
        }),
        _ => None,
    };
    (mem, swap)
}

/// Bytes as a person reads them, in at most four characters plus the unit.
pub(crate) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "K", "M", "G", "T"];
    #[allow(clippy::cast_precision_loss)]
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes}{}", UNITS[0])
    } else if value < 10.0 {
        format!("{value:.1}{}", UNITS[unit])
    } else {
        format!("{value:.0}{}", UNITS[unit])
    }
}

/// What a section shows for one metric of one sample.
///
/// Three outcomes, and they read differently on purpose: a number, `off` for a
/// pool the machine genuinely does not have, and `--` for one that could not be
/// read. Collapsing the last two would tell somebody with no swap that their
/// meter is broken, and somebody with a broken meter that they have no swap.
// TP-RES-04: unreadable renders `--`, absent renders `off`, neither renders 0.
pub(crate) fn metric_text(sample: &ResourceSample, metric: ResourceMetric) -> String {
    let label = metric.label();
    match metric {
        ResourceMetric::Cpu => match sample.cpu {
            Some(pct) => format!("{label} {pct:>3.0}%"),
            None => format!("{label}  --"),
        },
        ResourceMetric::Mem => usage_text(label, sample.mem),
        ResourceMetric::Swap => usage_text(label, sample.swap),
    }
}

fn usage_text(label: &str, usage: Option<Usage>) -> String {
    match usage {
        Some(usage) if usage.total == 0 => format!("{label} off"),
        Some(usage) => format!(
            "{label} {}/{}",
            format_bytes(usage.used),
            format_bytes(usage.total)
        ),
        None => format!("{label}  --"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A real line from a running 6.x kernel, ten fields wide.
    const PROC_STAT: &str = "cpu  1000 20 300 8000 50 0 10 0 0 0\n\
                             cpu0 500 10 150 4000 25 0 5 0 0 0\n\
                             intr 12345\n";

    #[test]
    fn the_aggregate_cpu_line_is_summed_including_fields_added_by_newer_kernels() {
        let times = parse_proc_stat(PROC_STAT).expect("a well formed cpu line parses");
        assert_eq!(times.total, 1000 + 20 + 300 + 8000 + 50 + 10);
        // idle + iowait, and nothing else.
        assert_eq!(times.idle, 8000 + 50);
    }

    #[test]
    fn the_per_core_lines_are_not_mistaken_for_the_aggregate() {
        // `cpu0` starts with "cpu" too; matching on the prefix alone would read
        // one core and call it the machine.
        let only_cores = "cpu0 500 10 150 4000 25\nintr 1\n";
        assert_eq!(parse_proc_stat(only_cores), None);
    }

    #[test]
    fn a_truncated_or_unreadable_stat_refuses_rather_than_guessing() {
        assert_eq!(parse_proc_stat(""), None);
        assert_eq!(parse_proc_stat("cpu  1 2 3\n"), None, "shorter than idle");
        assert_eq!(parse_proc_stat("cpu  1 2 x 4 5\n"), None, "not a number");
        assert_eq!(parse_proc_stat("cpuinfo 1 2 3 4 5\n"), None, "wrong key");
    }

    #[test]
    fn a_percentage_is_the_busy_share_of_the_interval_between_two_readings() {
        let prev = CpuTimes {
            total: 1000,
            idle: 900,
        };
        let now = CpuTimes {
            total: 1100,
            idle: 950,
        };
        // 100 jiffies passed, 50 of them idle.
        let pct = cpu_percent(prev, now).expect("two usable readings");
        assert!((pct - 50.0).abs() < f32::EPSILON, "got {pct}");
    }

    #[test]
    fn two_readings_that_cannot_produce_an_answer_refuse_instead_of_dividing() {
        let same = CpuTimes {
            total: 1000,
            idle: 900,
        };
        assert_eq!(cpu_percent(same, same), None, "no time passed");

        let backwards = CpuTimes {
            total: 999,
            idle: 899,
        };
        assert_eq!(cpu_percent(same, backwards), None, "counters went back");

        // Idle grew faster than total, which cannot happen on a sane kernel and
        // would otherwise produce a negative busy share.
        let impossible = CpuTimes {
            total: 1010,
            idle: 1000,
        };
        assert_eq!(cpu_percent(same, impossible), None);
    }

    #[test]
    fn memory_is_derived_from_available_rather_than_free_and_kb_become_bytes() {
        let meminfo = "MemTotal:       32000000 kB\n\
                       MemFree:          400000 kB\n\
                       MemAvailable:   24000000 kB\n\
                       SwapTotal:       8000000 kB\n\
                       SwapFree:        7000000 kB\n";
        let (mem, swap) = parse_proc_meminfo(meminfo);
        let mem = mem.expect("memory is present");
        assert_eq!(mem.total, 32_000_000 * 1024);
        // Built on MemFree this would read 31.6M of 32M used on an idle box.
        assert_eq!(mem.used, (32_000_000 - 24_000_000) * 1024);
        let swap = swap.expect("swap is present");
        assert_eq!(swap.used, 1_000_000 * 1024);
    }

    #[test]
    fn a_pool_whose_lines_are_missing_is_absent_rather_than_zero() {
        let (mem, swap) = parse_proc_meminfo("MemTotal: 100 kB\n");
        assert_eq!(mem, None, "total without available cannot make a usage");
        assert_eq!(swap, None);
        let (mem, swap) = parse_proc_meminfo("");
        assert_eq!(mem, None);
        assert_eq!(swap, None);
    }

    #[test]
    fn a_malformed_meminfo_line_is_skipped_without_taking_the_rest_with_it() {
        let meminfo = "garbage without a colon\n\
                       MemTotal:       nonsense kB\n\
                       MemTotal:       1000 kB\n\
                       MemAvailable:    400 kB\n";
        let (mem, _) = parse_proc_meminfo(meminfo);
        let mem = mem.expect("the readable line still counts");
        assert_eq!(mem.total, 1000 * 1024);
        assert_eq!(mem.used, 600 * 1024);
    }

    #[test]
    fn a_reading_that_failed_shows_dashes_and_never_a_zero() {
        let broken = ResourceSample::default();
        assert_eq!(metric_text(&broken, ResourceMetric::Cpu), "CPU  --");
        assert_eq!(metric_text(&broken, ResourceMetric::Mem), "MEM  --");
        assert_eq!(metric_text(&broken, ResourceMetric::Swap), "SWP  --");
        for metric in [
            ResourceMetric::Cpu,
            ResourceMetric::Mem,
            ResourceMetric::Swap,
        ] {
            let text = metric_text(&broken, metric);
            assert!(
                !text.contains('0'),
                "a broken meter must not read as an idle one: {text:?}"
            );
        }
    }

    #[test]
    fn a_machine_with_no_swap_says_so_rather_than_looking_broken() {
        let sample = ResourceSample {
            swap: Some(Usage { used: 0, total: 0 }),
            ..ResourceSample::default()
        };
        assert_eq!(metric_text(&sample, ResourceMetric::Swap), "SWP off");
    }

    #[test]
    fn a_reading_that_worked_shows_the_numbers() {
        let sample = ResourceSample {
            cpu: Some(12.4),
            mem: Some(Usage {
                used: 5_368_709_120,
                total: 33_285_996_544,
            }),
            swap: Some(Usage {
                used: 0,
                total: 8_589_934_592,
            }),
        };
        assert_eq!(metric_text(&sample, ResourceMetric::Cpu), "CPU  12%");
        assert_eq!(metric_text(&sample, ResourceMetric::Mem), "MEM 5.0G/31G");
        assert_eq!(metric_text(&sample, ResourceMetric::Swap), "SWP 0B/8.0G");
    }

    #[test]
    fn a_metric_name_maps_to_one_metric_and_a_typo_maps_to_none() {
        assert_eq!(ResourceMetric::parse("cpu"), Some(ResourceMetric::Cpu));
        assert_eq!(ResourceMetric::parse("mem"), Some(ResourceMetric::Mem));
        assert_eq!(ResourceMetric::parse("ram"), Some(ResourceMetric::Mem));
        assert_eq!(ResourceMetric::parse("swap"), Some(ResourceMetric::Swap));
        assert_eq!(ResourceMetric::parse("cpu%"), None);
        assert_eq!(ResourceMetric::parse(""), None);
    }
}
