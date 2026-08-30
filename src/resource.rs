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
/// that reports memory but not swap is a normal machine, not a broken one, and
/// a desktop with no battery is not a broken laptop.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct ResourceSample {
    pub(crate) cpu: Option<f32>,
    pub(crate) mem: Option<Usage>,
    pub(crate) swap: Option<Usage>,
    /// The filesystem herdr itself is running from, used and total.
    pub(crate) disk: Option<Usage>,
    /// Charge remaining, 0..=100.
    pub(crate) battery: Option<f32>,
    /// Bytes per second across every interface but loopback, in and out
    /// together. A rate, so like CPU it does not exist until the second
    /// reading — and unlike every other metric here it has no ceiling, so it
    /// is a figure and never a proportion.
    pub(crate) net: Option<f64>,
    /// The warmest sensor the machine reports, in degrees Celsius.
    pub(crate) temp: Option<f32>,
}

/// Which number a section shows. Closed, because config names map onto it and
/// an open set would let a typo become a silently blank section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ResourceMetric {
    Cpu,
    Mem,
    Swap,
    Disk,
    Battery,
    Net,
    Temp,
}

/// One metric, and everything a config or a message needs to say about it.
///
/// A table rather than parallel `match`es. The set has to be printable — by the
/// refusal, by `herdr shell spec`, by the guide's own gate — and a `match` can
/// be asked whether it knows a name but never what names it knows (D70-7). The
/// list of accepted spellings used to be written out beside the parser, which
/// is two places to add a metric and one place to forget.
pub(crate) struct MetricFacts {
    /// The spelling to teach.
    pub name: &'static str,
    pub metric: ResourceMetric,
    /// What a section prints in front of the number.
    pub label: &'static str,
}

pub(crate) const METRICS: &[MetricFacts] = &[
    MetricFacts {
        name: "cpu",
        metric: ResourceMetric::Cpu,
        label: "CPU",
    },
    MetricFacts {
        name: "mem",
        metric: ResourceMetric::Mem,
        label: "MEM",
    },
    MetricFacts {
        name: "swap",
        metric: ResourceMetric::Swap,
        label: "SWP",
    },
    MetricFacts {
        name: "disk",
        metric: ResourceMetric::Disk,
        label: "DSK",
    },
    MetricFacts {
        name: "battery",
        metric: ResourceMetric::Battery,
        label: "BAT",
    },
    MetricFacts {
        name: "net",
        metric: ResourceMetric::Net,
        label: "NET",
    },
    MetricFacts {
        name: "temp",
        metric: ResourceMetric::Temp,
        label: "TMP",
    },
];

impl ResourceMetric {
    /// The names a refusal offers, in the order it offers them.
    ///
    /// Derived from the table rather than written beside it, so a metric that
    /// exists is a metric the refusal names. `ram` is deliberately absent — it
    /// is an alias `parse` honours, not a spelling to teach.
    pub(crate) fn accepted() -> Vec<&'static str> {
        METRICS.iter().map(|facts| facts.name).collect()
    }

    /// The spellings `parse` honours that `accepted()` deliberately does not
    /// teach, each beside the name it is another word for.
    ///
    /// Kept apart rather than folded in. A refusal that offered both would
    /// present an alias as an equal citizen, and dropping the alias would break
    /// files that already say it — so the grammar has to be able to state
    /// "accepted, but not the spelling to learn", and this is where it states it.
    pub(crate) const ALIASES: &'static [(&'static str, &'static str)] = &[("ram", "mem")];

    pub(crate) fn parse(name: &str) -> Option<Self> {
        if let Some(facts) = METRICS.iter().find(|facts| facts.name == name) {
            return Some(facts.metric);
        }
        Self::ALIASES
            .iter()
            .find(|(alias, _)| *alias == name)
            .and_then(|(_, means)| METRICS.iter().find(|facts| facts.name == *means))
            .map(|facts| facts.metric)
    }

    pub(crate) fn label(self) -> &'static str {
        METRICS
            .iter()
            .find(|facts| facts.metric == self)
            // Unreachable while the table covers the enum, which
            // `every_metric_has_a_row_in_the_table` is what makes true.
            .map_or("???", |facts| facts.label)
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

/// Total bytes carried by every interface but loopback, from `/proc/net/dev`.
///
/// Cumulative since boot, so this is not a rate yet — the rate is the
/// difference between two of these over the time between them, exactly as CPU
/// is. Loopback is excluded because a process talking to itself is not traffic,
/// and on a machine running a local server it dwarfs everything real.
///
/// Receive and transmit are summed. Two figures need two sections, and the
/// question a bar answers is "is the network busy", which one number answers.
// TP-RES-19: network totals exclude loopback and sum both directions.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn parse_proc_net_dev(text: &str) -> Option<u64> {
    let mut total: u64 = 0;
    let mut seen = false;
    for line in text.lines() {
        let Some((interface, counters)) = line.split_once(':') else {
            continue; // the two header lines carry no colon
        };
        let interface = interface.trim();
        if interface == "lo" {
            continue;
        }
        let numbers = counters
            .split_whitespace()
            .map(str::parse::<u64>)
            .collect::<Result<Vec<_>, _>>()
            .ok()?;
        // `receive bytes` is first and `transmit bytes` is the ninth of the
        // sixteen columns. Indexed rather than summed, unlike `/proc/stat`:
        // these columns are packets and errors as well as bytes, and adding
        // them all would report a number that is not bytes at all.
        let (Some(received), Some(transmitted)) = (numbers.first(), numbers.get(8)) else {
            continue;
        };
        total = total.saturating_add(*received).saturating_add(*transmitted);
        seen = true;
    }
    seen.then_some(total)
}

/// The warmest sensor reading, in degrees Celsius, from millidegree inputs.
///
/// The warmest rather than the first or an average: a person watching a
/// temperature is watching for trouble, and trouble is whichever part of the
/// machine is hottest. An average of a hot CPU and a cool chipset is a number
/// nothing in the machine is actually at.
// TP-RES-20: the temperature shown is the warmest sensor, in Celsius.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn warmest_millidegrees(readings: impl IntoIterator<Item = i64>) -> Option<f32> {
    let warmest = readings.into_iter().max()?;
    // Sensors that report a negative or absurd value are offline rather than
    // freezing; a laptop is not at -273 °C and a bar saying so is worse than
    // one saying nothing.
    if !(0..=150_000).contains(&warmest) {
        return None;
    }
    #[allow(clippy::cast_precision_loss)] // three significant digits at most
    Some(warmest as f32 / 1000.0)
}

/// Bytes per second between two cumulative readings.
///
/// `None` when the counter went backwards — an interface that came up or a
/// counter that wrapped — because the alternative is one enormous spike that
/// looks exactly like real traffic.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn byte_rate(previous: u64, current: u64, elapsed: std::time::Duration) -> Option<f64> {
    let seconds = elapsed.as_secs_f64();
    if seconds <= 0.0 || current < previous {
        return None;
    }
    #[allow(clippy::cast_precision_loss)] // a rate drawn in three digits
    Some((current - previous) as f64 / seconds)
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

/// How full one metric is, as 0..1, or `None` when it cannot be known.
///
/// A meter needs a ratio, and the three metrics carry it differently: CPU is
/// already a percentage, memory and swap are a pair. A pool with no capacity —
/// a machine with no swap — has no ratio at all; drawing it as empty would say
/// "plenty free" about something that does not exist.
// TP-METER-02: a pool with no capacity has no ratio, and neither has an
// unreadable one.
pub(crate) fn meter_ratio(sample: &ResourceSample, metric: ResourceMetric) -> Option<f32> {
    let usage = match metric {
        ResourceMetric::Cpu => return sample.cpu.map(|pct| (pct / 100.0).clamp(0.0, 1.0)),
        ResourceMetric::Battery => return sample.battery.map(|pct| (pct / 100.0).clamp(0.0, 1.0)),
        // A ceiling somebody can argue with, and that is the point: 100 °C is
        // where a machine is in trouble, so a full bar means the same thing a
        // full memory bar means. Clamped rather than left to overflow, because
        // a sensor reporting 127 would otherwise draw past its own section.
        ResourceMetric::Temp => return sample.temp.map(|c| (c / 100.0).clamp(0.0, 1.0)),
        // No ratio, on purpose. A rate has no ceiling to be a proportion of,
        // and inventing one — the fastest seen so far, the link speed — would
        // make a full bar mean something different on every machine and at
        // every moment. A `net` meter therefore draws nothing, which is what
        // this function already says about a pool with no capacity.
        ResourceMetric::Net => return None,
        ResourceMetric::Mem => sample.mem?,
        ResourceMetric::Swap => sample.swap?,
        ResourceMetric::Disk => sample.disk?,
    };
    if usage.total == 0 {
        return None;
    }
    // Lossy above 2^24 bytes of precision, which for a ratio drawn in at most a
    // few dozen cells is far below one pixel of difference.
    #[allow(clippy::cast_precision_loss)]
    Some((usage.used as f32 / usage.total as f32).clamp(0.0, 1.0))
}

/// The eighth-blocks that draw a bar `width` cells wide filled to `ratio`.
///
/// Returns whole cells plus the eighths of the one after them. Eighths rather
/// than whole cells because a meter that can only move in cell steps jumps: on
/// a ten-cell bar every change under 10% is invisible, and then it lurches. The
/// glyphs `▏▎▍▌▋▊▉█` exist for exactly this and cost the same one cell.
// TP-METER-03: a bar moves in eighths, and never exceeds its own width.
pub(crate) fn meter_cells(ratio: f32, width: u16) -> (u16, u8) {
    if width == 0 {
        return (0, 0);
    }
    let ratio = ratio.clamp(0.0, 1.0);
    #[allow(clippy::cast_precision_loss)]
    let eighths_total = (ratio * f32::from(width) * 8.0).round();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let eighths_total = eighths_total.max(0.0) as u32;
    let full = u16::try_from(eighths_total / 8).unwrap_or(width).min(width);
    let remainder = if full >= width {
        0
    } else {
        u8::try_from(eighths_total % 8).unwrap_or(0)
    };
    (full, remainder)
}

/// How a meter draws its fill. A closed set: an unknown name is refused at
/// parse time rather than drawn blank (the section-kind rule).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum MeterDisplay {
    /// Horizontal eighth-blocks — the original look.
    #[default]
    Bar,
    /// Vertical eighth-blocks for the partial cell (▁▂▃▄▅▆▇, full █).
    Blocks,
    /// Braille density ramp (⣀ ⣤ ⣶, full ⣿).
    Braille,
    /// The bar's fill recoloured by position: green start, red end.
    Gradient,
    /// Round dots — full • over an explicit empty · track.
    Dots,
}

impl MeterDisplay {
    pub(crate) fn parse(name: &str) -> Option<Self> {
        match name {
            "bar" => Some(Self::Bar),
            "blocks" => Some(Self::Blocks),
            "braille" => Some(Self::Braille),
            "gradient" => Some(Self::Gradient),
            "dots" => Some(Self::Dots),
            _ => None,
        }
    }

    pub(crate) const fn full_symbol(self) -> &'static str {
        match self {
            Self::Bar | Self::Blocks | Self::Gradient => "\u{2588}",
            Self::Braille => "\u{28ff}",
            Self::Dots => "\u{2022}",
        }
    }

    /// The glyph for the one partial cell, or `None` when this display
    /// rounds instead (dots have no half dot worth drawing).
    pub(crate) const fn partial_symbol(self, eighths: u8) -> Option<&'static str> {
        match self {
            Self::Bar | Self::Gradient => eighth_block(eighths),
            Self::Blocks => match eighths {
                1 => Some("\u{2581}"),
                2 => Some("\u{2582}"),
                3 => Some("\u{2583}"),
                4 => Some("\u{2584}"),
                5 => Some("\u{2585}"),
                6 => Some("\u{2586}"),
                7 => Some("\u{2587}"),
                _ => None,
            },
            Self::Braille => match eighths {
                1 | 2 => Some("\u{28c0}"),
                3 | 4 => Some("\u{28e4}"),
                5 | 6 => Some("\u{28f6}"),
                7 => Some("\u{28ff}"),
                _ => None,
            },
            Self::Dots => None,
        }
    }

    /// What an empty cell shows: dots draw an explicit track, every other
    /// display leaves the cell untouched.
    pub(crate) const fn empty_symbol(self) -> Option<&'static str> {
        match self {
            Self::Dots => Some("\u{00b7}"),
            _ => None,
        }
    }

    /// The colour name for the cell at `column` of `width`. Every display
    /// but the gradient colours by VALUE (the existing meter rule); the
    /// gradient colours by POSITION, so the fill walks green → yellow → red
    /// as it grows and the palette stays in charge of the actual colours.
    pub(crate) fn cell_colour(self, ratio: f32, column: u16, width: u16) -> &'static str {
        match self {
            Self::Gradient => {
                let position = (f32::from(column) + 0.5) / f32::from(width.max(1));
                meter_colour(position)
            }
            _ => meter_colour(ratio),
        }
    }
}

/// The eighth-block glyph for a partial cell, or `None` for an empty one.
pub(crate) const fn eighth_block(eighths: u8) -> Option<&'static str> {
    match eighths {
        1 => Some("\u{258f}"),
        2 => Some("\u{258e}"),
        3 => Some("\u{258d}"),
        4 => Some("\u{258c}"),
        5 => Some("\u{258b}"),
        6 => Some("\u{258a}"),
        7 => Some("\u{2589}"),
        _ => None,
    }
}

/// The same, for a cell filled from the bottom rather than from the left.
///
/// A meter grows sideways and a sparkline grows upward, so they need different
/// halves of the same idea: `eighth_block` gives ▏▎▍▌▋▊▉ and this gives ▁▂▃▄▅▆▇.
/// Only the glyph table differs — the arithmetic that decides how many eighths
/// a value is worth is `meter_cells`, and neither of them repeats it.
pub(crate) const fn lower_eighth_block(eighths: u8) -> Option<&'static str> {
    match eighths {
        1 => Some("\u{2581}"),
        2 => Some("\u{2582}"),
        3 => Some("\u{2583}"),
        4 => Some("\u{2584}"),
        5 => Some("\u{2585}"),
        6 => Some("\u{2586}"),
        7 => Some("\u{2587}"),
        _ => None,
    }
}

/// How many readings of each metric are kept for a sparkline to draw.
///
/// Comfortably wider than any terminal, and small enough not to matter: three
/// metrics at four bytes each is about six kilobytes. A section wider than this
/// draws what there is rather than repeating the oldest reading, because a
/// repeated sample is a shape somebody would read as real.
pub(crate) const RESOURCE_HISTORY_CAPACITY: usize = 512;

/// What each metric has recently been, oldest first.
///
/// Ratios rather than raw readings. `meter_ratio` already flattens the three
/// metrics onto one 0..1 scale, so storing what it produced keeps the history
/// metric-agnostic and means drawing does no arithmetic at all.
///
/// `Option` is carried through on purpose: a reading that could not be taken is
/// not a reading of zero, and the two must not draw the same. That distinction
/// is the whole reason this is a history of options rather than of numbers.
/// One ring per metric, indexed by the metric's position in `METRICS`.
///
/// An array rather than a field per metric, and that is what stopped this
/// growing a fourth arm in three places when the table grew from three metrics
/// to seven. The index comes from the table, so a metric that exists has a ring
/// and a ring belongs to a metric — neither can be added without the other.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ResourceHistory {
    series: [std::collections::VecDeque<Option<f32>>; METRICS.len()],
}

impl ResourceHistory {
    /// Record one reading of every metric.
    pub(crate) fn push(&mut self, sample: &ResourceSample) {
        for (index, facts) in METRICS.iter().enumerate() {
            let series = &mut self.series[index];
            if series.len() == RESOURCE_HISTORY_CAPACITY {
                series.pop_front();
            }
            series.push_back(meter_ratio(sample, facts.metric));
        }
    }

    /// One metric's readings, oldest first.
    pub(crate) fn series(
        &self,
        metric: ResourceMetric,
    ) -> &std::collections::VecDeque<Option<f32>> {
        static EMPTY: std::sync::OnceLock<std::collections::VecDeque<Option<f32>>> =
            std::sync::OnceLock::new();
        METRICS
            .iter()
            .position(|facts| facts.metric == metric)
            .and_then(|index| self.series.get(index))
            // Unreachable while the table covers the enum, and pinned by
            // `every_metric_has_a_row_in_the_table`. An empty run draws as a
            // section with no history yet, which is the honest answer to a
            // metric nothing has recorded.
            .unwrap_or_else(|| EMPTY.get_or_init(Default::default))
    }
}

/// The colour a level reads as. Thresholds, not a gradient: a person reads a
/// meter to answer "is this a problem", and three answers are easier to see at
/// a glance in three cells than a continuous ramp.
// TP-METER-02: level maps to a palette token, and the boundaries are stable.
pub(crate) fn meter_colour(ratio: f32) -> &'static str {
    if ratio >= 0.85 {
        "red"
    } else if ratio >= 0.6 {
        "yellow"
    } else {
        "green"
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
        ResourceMetric::Cpu => percent_text(label, sample.cpu),
        ResourceMetric::Battery => percent_text(label, sample.battery),
        ResourceMetric::Mem => usage_text(label, sample.mem),
        ResourceMetric::Swap => usage_text(label, sample.swap),
        ResourceMetric::Disk => usage_text(label, sample.disk),
        ResourceMetric::Temp => match sample.temp {
            Some(celsius) => format!("{label} {celsius:>3.0}C"),
            None => format!("{label}  --"),
        },
        ResourceMetric::Net => match sample.net {
            Some(rate) => format!("{label} {}", rate_text(rate)),
            None => format!("{label}  --"),
        },
    }
}

fn percent_text(label: &str, percent: Option<f32>) -> String {
    match percent {
        Some(pct) => format!("{label} {pct:>3.0}%"),
        None => format!("{label}  --"),
    }
}

/// Bytes per second, in the largest unit that leaves a number worth reading.
///
/// Three characters and a unit, so the section's width does not change as the
/// rate does — a figure that grew a column when traffic picked up would push
/// whatever sits beside it, and a bar that reflows while you watch it is harder
/// to read than one that does not.
fn rate_text(rate: f64) -> String {
    const UNITS: [&str; 4] = ["B/s", "K/s", "M/s", "G/s"];
    let mut value = rate.max(0.0);
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:>3.0}{}", UNITS[unit])
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

/// One sample reduced to the precision anything on screen can show.
///
/// The sampler reads floats and byte counts; the bar renders whole percents,
/// megabytes and two-figure rates. Two samples that agree at this precision
/// look identical on every surface, so they must not cost a frame. The quantum
/// errs fine on purpose: an extra frame is cheap, a gauge frozen by a
/// too-coarse comparison is a wrong display. TP-RES-27
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct DisplaySignature {
    cpu_pct: Option<u8>,
    mem_mb: Option<(u64, u64)>,
    swap_mb: Option<(u64, u64)>,
    disk_mb: Option<(u64, u64)>,
    battery_pct: Option<u8>,
    net_rate: Option<u64>,
    temp_c: Option<i16>,
}

fn usage_mb(usage: Usage) -> (u64, u64) {
    const MB: u64 = 1024 * 1024;
    (usage.used / MB, usage.total / MB)
}

/// Round to two significant figures: a rate has no ceiling, so a fixed unit
/// would either flap at every kilobyte or hide whole megabytes.
fn two_figures(value: u64) -> u64 {
    if value < 100 {
        return value;
    }
    let mut scale = 1u64;
    let mut head = value;
    while head >= 100 {
        head /= 10;
        scale = scale.saturating_mul(10);
    }
    // Round the third figure instead of truncating it, so 194 and 196 land
    // apart rather than both on 190.
    ((value + scale / 2) / scale).saturating_mul(scale)
}

pub(crate) fn display_signature(sample: &ResourceSample) -> DisplaySignature {
    DisplaySignature {
        cpu_pct: sample.cpu.map(|cpu| cpu.round().clamp(0.0, 255.0) as u8),
        mem_mb: sample.mem.map(usage_mb),
        swap_mb: sample.swap.map(usage_mb),
        disk_mb: sample.disk.map(usage_mb),
        battery_pct: sample
            .battery
            .map(|charge| charge.round().clamp(0.0, 255.0) as u8),
        net_rate: sample.net.map(|rate| two_figures(rate.max(0.0) as u64)),
        temp_c: sample
            .temp
            .map(|temp| temp.round().clamp(-999.0, 999.0) as i16),
    }
}

/// Whether this sample deserves a frame. TP-RES-27: a reading that lands on
/// the numbers already shown is not a change; a visible sparkline makes every
/// sample a change, because its history — which grows unconditionally — is
/// itself the picture.
pub(crate) fn display_changed(
    previous: &mut Option<DisplaySignature>,
    sample: &ResourceSample,
    sparkline_visible: bool,
) -> bool {
    let signature = display_signature(sample);
    let changed = sparkline_visible || previous.is_none_or(|last| last != signature);
    *previous = Some(signature);
    changed
}

#[cfg(test)]
mod tests {
    // TP-RES-27: two readings that agree at display precision are one
    // picture — the second must not cost a frame.
    #[test]
    fn a_reading_on_the_same_shown_numbers_is_not_a_change() {
        let mut previous = None;
        let mut sample = super::ResourceSample {
            cpu: Some(36.2),
            ..Default::default()
        };
        assert!(
            super::display_changed(&mut previous, &sample, false),
            "the first reading has nothing on screen to agree with"
        );
        sample.cpu = Some(36.4); // still shows 36
        assert!(!super::display_changed(&mut previous, &sample, false));
        sample.cpu = Some(37.6); // shows 38
        assert!(super::display_changed(&mut previous, &sample, false));
    }

    // TP-RES-27: a sparkline's history is the picture itself, so for it an
    // identical reading is still a new column.
    #[test]
    fn a_visible_sparkline_makes_every_reading_a_change() {
        let mut previous = None;
        let sample = super::ResourceSample {
            cpu: Some(36.2),
            ..Default::default()
        };
        assert!(super::display_changed(&mut previous, &sample, true));
        assert!(
            super::display_changed(&mut previous, &sample, true),
            "an identical reading still scrolls a history"
        );
    }

    // TP-RES-27: a rate has no ceiling, so it compares at two significant
    // figures — fixed units would either flap at every kilobyte or hide
    // whole megabytes.
    #[test]
    fn rates_compare_at_two_significant_figures() {
        assert_eq!(super::two_figures(99), 99);
        assert_eq!(super::two_figures(194), 190);
        assert_eq!(super::two_figures(196), 200);
        assert_eq!(super::two_figures(203_000), 200_000);
        assert_eq!(super::two_figures(207_000), 210_000);
    }

    // C1: the display family's shared contract, property-style — fills are
    // monotone in value, exact at both ends, sane at width 1, and NaN or a
    // negative sample clamps to empty instead of panicking or overdrawing.
    #[test]
    fn meter_fill_is_monotone_and_exact_at_the_ends() {
        for width in [1u16, 5, 13, 80] {
            let mut last = (0u16, 0u8);
            for step in 0..=100u32 {
                #[allow(clippy::cast_precision_loss)]
                let ratio = step as f32 / 100.0;
                let (full, eighths) = meter_cells(ratio, width);
                let total = u32::from(full) * 8 + u32::from(eighths);
                let last_total = u32::from(last.0) * 8 + u32::from(last.1);
                assert!(
                    total >= last_total,
                    "fill is monotone (w={width} step={step})"
                );
                assert!(full <= width);
                last = (full, eighths);
            }
            assert_eq!(meter_cells(0.0, width), (0, 0), "empty at zero");
            assert_eq!(meter_cells(1.0, width), (width, 0), "full at one");
        }
        assert_eq!(meter_cells(f32::NAN, 10), (0, 0), "NaN clamps to empty");
        assert_eq!(meter_cells(-3.0, 10), (0, 0), "negative clamps to empty");
        assert_eq!(meter_cells(7.0, 10), (10, 0), "overdrive clamps to full");
    }

    // C1: each display names a full glyph, its own partial ramp (or none for
    // dots, which round), and only dots draw an explicit empty track.
    #[test]
    fn every_display_names_its_glyph_family() {
        use MeterDisplay as D;
        assert_eq!(D::parse("bar"), Some(D::Bar));
        assert_eq!(D::parse("blocks"), Some(D::Blocks));
        assert_eq!(D::parse("braille"), Some(D::Braille));
        assert_eq!(D::parse("gradient"), Some(D::Gradient));
        assert_eq!(D::parse("dots"), Some(D::Dots));
        assert_eq!(D::parse("wave"), None, "the set is closed");

        let blocks_ladder: Vec<_> = (1..=7).map(|e| D::Blocks.partial_symbol(e)).collect();
        assert_eq!(
            blocks_ladder,
            ["\u{2581}", "\u{2582}", "\u{2583}", "\u{2584}", "\u{2585}", "\u{2586}", "\u{2587}"]
                .map(Some),
            "the blocks ladder climbs the vertical eighths in order"
        );
        assert_eq!(D::Braille.partial_symbol(1), Some("\u{28c0}"));
        assert_eq!(D::Braille.partial_symbol(3), Some("\u{28e4}"));
        assert_eq!(D::Braille.partial_symbol(5), Some("\u{28f6}"));
        assert_eq!(D::Braille.partial_symbol(7), Some("\u{28ff}"));
        assert_eq!(D::Bar.partial_symbol(4), eighth_block(4));
        assert_eq!(D::Dots.partial_symbol(4), None, "dots round, no half dot");
        for display in [D::Bar, D::Blocks, D::Braille, D::Gradient] {
            assert_eq!(display.empty_symbol(), None);
            assert_eq!(display.partial_symbol(0), None);
            assert_eq!(display.partial_symbol(8), None);
        }
        assert_eq!(D::Dots.empty_symbol(), Some("\u{00b7}"));
        assert_eq!(D::Dots.full_symbol(), "\u{2022}");
    }

    // C1: the gradient colours by POSITION (green start, red end, whatever
    // the value); every other display keeps the value-coloured rule.
    #[test]
    fn the_gradient_walks_the_ramp_by_position() {
        use MeterDisplay as D;
        assert_eq!(D::Gradient.cell_colour(0.1, 0, 10), meter_colour(0.05));
        assert_eq!(D::Gradient.cell_colour(0.1, 9, 10), meter_colour(0.95));
        assert_ne!(
            D::Gradient.cell_colour(0.1, 0, 10),
            D::Gradient.cell_colour(0.1, 9, 10),
            "the two ends of the ramp differ"
        );
        assert_eq!(D::Bar.cell_colour(0.97, 0, 10), meter_colour(0.97));
        assert_eq!(
            D::Bar.cell_colour(0.97, 0, 10),
            D::Bar.cell_colour(0.97, 9, 10),
            "value colouring is position-blind"
        );
    }

    use super::*;

    // TP-CHROME-106: the list a refusal offers and the names `parse` takes are two halves
    // of one closed set, and they sit in different modules. Nothing but this holds them
    // together, so a metric added to the enum and forgotten here would be a feature
    // the message never mentions.
    /// The table covers the enum, and nothing in it repeats.
    ///
    /// `label()` and `series()` both fall back when a metric has no row, and a
    /// fallback nobody can reach is a fallback nobody notices going wrong. This
    /// is what makes those two unreachable, so a metric added to the enum and
    /// forgotten in the table turns this red instead of printing `???` on
    /// somebody's bar.
    // TP-RES-22: every metric has exactly one row, and every row a distinct name.
    #[test]
    fn every_metric_has_a_row_in_the_table() {
        // Written out rather than iterated: a list derived from the table would
        // agree with the table however wrong both were. This is the hand-written
        // half the gate needs.
        let all = [
            ResourceMetric::Cpu,
            ResourceMetric::Mem,
            ResourceMetric::Swap,
            ResourceMetric::Disk,
            ResourceMetric::Battery,
            ResourceMetric::Net,
            ResourceMetric::Temp,
        ];
        assert_eq!(
            all.len(),
            METRICS.len(),
            "the table and the enum are different sizes"
        );
        for metric in all {
            let rows = METRICS
                .iter()
                .filter(|facts| facts.metric == metric)
                .count();
            assert_eq!(rows, 1, "{metric:?} has {rows} rows, not one");
            assert_ne!(
                metric.label(),
                "???",
                "{metric:?} fell through to the unreachable label"
            );
        }

        let names = METRICS
            .iter()
            .map(|facts| facts.name)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(names.len(), METRICS.len(), "two rows share a name");
        let labels = METRICS
            .iter()
            .map(|facts| facts.label)
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            labels.len(),
            METRICS.len(),
            "two metrics print the same label, so a bar cannot say which is which"
        );
    }

    /// A metric that could not be read renders `--`, never a number.
    ///
    /// The rule the whole module is built on, checked across every metric at
    /// once because it is the kind of thing a new arm gets wrong on its own: a
    /// battery reading `0%` on a desktop and a temperature reading `0C` on a
    /// machine with no sensor are both false in a way nothing on screen
    /// distinguishes from true.
    // TP-RES-23: an unreadable metric renders `--`, and never zero.
    #[test]
    fn an_unreadable_metric_never_renders_as_zero() {
        let nothing = ResourceSample::default();
        for facts in METRICS {
            let text = metric_text(&nothing, facts.metric);
            assert!(
                text.contains("--"),
                "{} renders {text:?} when it could not be read",
                facts.name
            );
            assert!(
                !text.contains('0'),
                "{} renders {text:?}, which reads as a real reading of zero",
                facts.name
            );
        }
    }

    /// A rate has no ceiling, so it is never drawn as a proportion.
    // TP-RES-24: `net` has no meter ratio, whatever it is reading.
    #[test]
    fn a_network_rate_is_never_a_proportion() {
        let busy = ResourceSample {
            net: Some(125_000_000.0),
            ..Default::default()
        };
        assert_eq!(
            meter_ratio(&busy, ResourceMetric::Net),
            None,
            "a full bar would have to mean something, and there is nothing for \
             it to be full of"
        );

        // Control: the metrics that do have a ceiling still produce one, or the
        // assertion above would hold for a `meter_ratio` that gave up entirely.
        let charged = ResourceSample {
            battery: Some(50.0),
            temp: Some(50.0),
            ..Default::default()
        };
        assert_eq!(meter_ratio(&charged, ResourceMetric::Battery), Some(0.5));
        assert_eq!(meter_ratio(&charged, ResourceMetric::Temp), Some(0.5));
    }

    /// A counter that went backwards is not a spike.
    // TP-RES-25: a rate needs two readings, forward in time.
    #[test]
    fn a_rate_refuses_a_counter_that_went_backwards_or_stood_still() {
        let second = std::time::Duration::from_secs(1);
        assert_eq!(byte_rate(1_000, 2_000, second), Some(1_000.0));
        assert_eq!(
            byte_rate(2_000, 1_000, second),
            None,
            "an interface that came up would otherwise read as an enormous burst"
        );
        assert_eq!(
            byte_rate(1_000, 2_000, std::time::Duration::ZERO),
            None,
            "no time passed, so no rate exists"
        );
    }

    /// Loopback is not traffic, and both directions are one number.
    // TP-RES-19: network totals exclude loopback and sum both directions.
    #[test]
    fn network_totals_skip_loopback_and_add_both_directions() {
        // Two header lines with no colon, then three interfaces. The columns
        // are the kernel's sixteen: bytes first, transmit bytes ninth.
        let text = "Inter-|   Receive                    |  Transmit\n\
                    face |bytes packets errs drop fifo frame compressed multicast|bytes packets\n\
                    \x20   lo:  5000      10    0    0    0     0          0         0   7000      10    0    0    0     0       0          0\n\
                    \x20 eth0:  1000       5    0    0    0     0          0         0    500       3    0    0    0     0       0          0\n\
                    \x20 wlan0:   30       1    0    0    0     0          0         0     20       1    0    0    0     0       0          0\n";
        assert_eq!(
            parse_proc_net_dev(text),
            Some(1_550),
            "1000+500 on eth0 and 30+20 on wlan0, with lo's 12000 left out"
        );
        assert_eq!(
            parse_proc_net_dev("Inter-|\nface |bytes\n"),
            None,
            "a file with no interface at all is unreadable, not a machine at rest"
        );
    }

    /// The warmest sensor wins, and an implausible one is offline.
    // TP-RES-20: the temperature shown is the warmest sensor, in Celsius.
    #[test]
    fn the_temperature_is_the_warmest_plausible_sensor() {
        assert_eq!(warmest_millidegrees([42_000, 61_500, 38_000]), Some(61.5));
        assert_eq!(
            warmest_millidegrees([-5_000, -1_000]),
            None,
            "a machine below freezing is a sensor that is off, not a cold laptop"
        );
        assert_eq!(
            warmest_millidegrees([200_000]),
            None,
            "a reading above any real silicon is a sensor reporting nonsense"
        );
        assert_eq!(warmest_millidegrees([]), None);
    }

    #[test]
    fn every_offered_metric_is_one_this_build_parses() {
        assert!(!ResourceMetric::accepted().is_empty());
        for name in ResourceMetric::accepted() {
            assert!(
                ResourceMetric::parse(name).is_some(),
                "the refusal offers {name:?} but parse does not take it"
            );
        }

        // The other direction, for the one spelling that is honoured without being
        // offered. Kept as a fact rather than a rule: see the shell-side test that
        // characterises it.
        assert_eq!(ResourceMetric::parse("ram"), Some(ResourceMetric::Mem));
        assert!(!ResourceMetric::accepted().contains(&"ram"));
    }

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
            disk: Some(Usage {
                used: 858_993_459_200,
                total: 1_023_180_800_000,
            }),
            battery: Some(78.0),
            net: Some(1_536_000.0),
            temp: Some(54.0),
        };
        assert_eq!(metric_text(&sample, ResourceMetric::Cpu), "CPU  12%");
        assert_eq!(metric_text(&sample, ResourceMetric::Mem), "MEM 5.0G/31G");
        assert_eq!(metric_text(&sample, ResourceMetric::Swap), "SWP 0B/8.0G");
        assert_eq!(metric_text(&sample, ResourceMetric::Disk), "DSK 800G/953G");
        assert_eq!(metric_text(&sample, ResourceMetric::Battery), "BAT  78%");
        assert_eq!(metric_text(&sample, ResourceMetric::Temp), "TMP  54C");
        assert_eq!(
            metric_text(&sample, ResourceMetric::Net),
            "NET   1M/s",
            "a rate is written in three digits and a unit, so the section does \
             not change width as traffic does"
        );
    }

    // TC-M1 · a pool that does not exist has no ratio. Drawing a swapless
    // machine as an empty bar says "plenty free" about something absent.
    #[test]
    fn a_pool_with_no_capacity_and_an_unreadable_one_both_have_no_ratio() {
        let none = ResourceSample::default();
        assert_eq!(meter_ratio(&none, ResourceMetric::Cpu), None);
        assert_eq!(meter_ratio(&none, ResourceMetric::Mem), None);

        let swapless = ResourceSample {
            swap: Some(Usage { used: 0, total: 0 }),
            ..ResourceSample::default()
        };
        assert_eq!(meter_ratio(&swapless, ResourceMetric::Swap), None);
    }

    #[test]
    fn a_ratio_comes_from_the_pair_and_a_percentage_from_the_number() {
        let sample = ResourceSample {
            cpu: Some(50.0),
            mem: Some(Usage { used: 3, total: 4 }),
            ..ResourceSample::default()
        };
        assert_eq!(meter_ratio(&sample, ResourceMetric::Cpu), Some(0.5));
        assert_eq!(meter_ratio(&sample, ResourceMetric::Mem), Some(0.75));
    }

    // TC-M2 · the bar moves in eighths and never overruns its own width.
    // Whole-cell steps would make every change under 1/width invisible and then
    // lurch; overrunning would paint the neighbouring section.
    #[test]
    fn a_bar_fills_in_eighths_and_never_exceeds_its_width() {
        assert_eq!(meter_cells(0.0, 10), (0, 0));
        assert_eq!(meter_cells(1.0, 10), (10, 0), "full leaves no partial cell");
        assert_eq!(meter_cells(0.5, 10), (5, 0));
        // Half of one cell on a one-cell bar is four eighths.
        assert_eq!(meter_cells(0.5, 1), (0, 4));
        // A value between cells keeps the remainder rather than rounding away.
        let (full, eighths) = meter_cells(0.25, 2);
        assert_eq!(
            (full, eighths),
            (0, 4),
            "quarter of two cells is half of one"
        );

        // Nonsense in, bounded out — never a bar wider than the rectangle.
        assert_eq!(meter_cells(9.0, 4), (4, 0));
        assert_eq!(meter_cells(-1.0, 4), (0, 0));
        assert_eq!(
            meter_cells(0.5, 0),
            (0, 0),
            "a zero-width bar draws nothing"
        );
    }

    // TP-SPARK-06: the upward glyph table, at both ends and through the middle.
    //
    // A glyph table is the kind of thing one wrong line breaks silently: every
    // value still renders something, and the shape is merely wrong by one step.
    // Checked against the codepoints rather than against itself, because a table
    // compared to a copy of itself agrees however wrong both are.
    #[test]
    fn the_upward_eighths_climb_one_step_at_a_time() {
        assert_eq!(lower_eighth_block(0), None, "nothing is not a glyph");
        assert_eq!(
            lower_eighth_block(8),
            None,
            "eight eighths is a full cell, which is the caller's to draw"
        );
        assert_eq!(
            (1..8)
                .filter_map(lower_eighth_block)
                .collect::<Vec<_>>()
                .concat(),
            "▁▂▃▄▅▆▇",
            "the upward eighths no longer climb one step at a time"
        );
    }

    // TP-SPARK-01: the history is a ring — it stops growing, and it keeps order.
    #[test]
    fn the_history_drops_its_oldest_reading_rather_than_growing() {
        let mut history = ResourceHistory::default();
        for used in 0..(RESOURCE_HISTORY_CAPACITY as u64 + 10) {
            history.push(&ResourceSample {
                mem: Some(Usage { used, total: 1000 }),
                ..Default::default()
            });
        }

        let series = history.series(ResourceMetric::Mem);
        assert_eq!(
            series.len(),
            RESOURCE_HISTORY_CAPACITY,
            "the history grew past its capacity"
        );
        // The newest reading is the last one pushed, and the oldest survivor is
        // the tenth — order is the whole meaning of a sparkline.
        let newest = (RESOURCE_HISTORY_CAPACITY as u64 + 9) as f32 / 1000.0;
        assert_eq!(series.back().copied().flatten(), Some(newest));
        assert_eq!(series.front().copied().flatten(), Some(10.0 / 1000.0));
    }

    // TP-SPARK-02: a reading that could not be taken is not a reading of zero.
    //
    // The two are one pixel apart and mean opposite things: "the machine was
    // idle" and "we have no idea". A history that flattened them would let a bar
    // report an idle machine it never measured.
    #[test]
    fn an_unread_metric_is_kept_apart_from_one_that_read_zero() {
        let mut history = ResourceHistory::default();
        history.push(&ResourceSample::default());
        history.push(&ResourceSample {
            cpu: Some(0.0),
            ..Default::default()
        });

        let cpu = history.series(ResourceMetric::Cpu);
        assert_eq!(cpu.front().copied(), Some(None), "an unread metric is None");
        assert_eq!(
            cpu.back().copied(),
            Some(Some(0.0)),
            "a metric read as zero is Some(0.0), not None"
        );
    }

    #[test]
    fn every_eighth_has_a_glyph_and_zero_and_eight_have_none() {
        assert_eq!(eighth_block(0), None);
        assert_eq!(
            eighth_block(8),
            None,
            "eight eighths is a full cell, not a partial"
        );
        for eighths in 1..8 {
            assert!(
                eighth_block(eighths).is_some(),
                "no glyph for {eighths} eighths"
            );
        }
    }

    // TC-M3 · three answers, and the boundaries are pinned because moving one
    // silently changes what a person believes about their machine.
    #[test]
    fn a_level_maps_to_one_of_three_colours_at_stable_boundaries() {
        assert_eq!(meter_colour(0.0), "green");
        assert_eq!(meter_colour(0.59), "green");
        assert_eq!(meter_colour(0.6), "yellow");
        assert_eq!(meter_colour(0.84), "yellow");
        assert_eq!(meter_colour(0.85), "red");
        assert_eq!(meter_colour(1.0), "red");
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
