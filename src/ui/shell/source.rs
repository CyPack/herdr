//! Where the desktop shell tree comes from, and what identifies the result.
//!
//! Until now the answer was written inline at the single call site: a hardcoded
//! `ShellLayout::default()` next to a hardcoded revision constant. That was
//! honest while there was exactly one possible tree, and it stops being honest
//! the moment there are two — because the revision is what the geometry cache
//! keys on, and a constant cannot describe a tree that changes.
//!
//! So derivation gets one home. Today it answers with the same legacy tree and
//! the same revision, which is why nothing on screen moves; tomorrow it is the
//! one place that learns about configured edge bars, and the cache key follows
//! it for free.
//!
//! Fail-closed by construction: a template that does not validate falls back to
//! the legacy tree rather than propagating an error to a renderer that has no
//! way to answer it. A shell that cannot be composed must still show a shell.

use ratatui::layout::Rect;
use ratatui::style::Color;

use crate::app::state::Palette;
use crate::config::{parse_color, ShellBarConfig, ShellBarSectionConfig, ShellBarsConfig};

use super::layout::allocate_section_lengths;
use super::model::{
    RegionId, RegionSize, ShellChild, ShellDirection, ShellLayout, ShellNode, ShellValidationError,
    TrackPolicy, ValidatedShellLayout, MAX_SPLIT_CHILDREN,
};
use super::template::ShellTemplateId;

/// The widest an edge bar may be before it stops being a bar.
///
/// A bounded number rather than a clamp: an out-of-range size is somebody's
/// mistake, and quietly resizing it to something they did not write would hide
/// the mistake behind a layout they did not ask for.
pub(crate) const MAX_BAR_CELLS: u16 = 32;

/// The thinnest a bordered strip can be and still hold anything: one cell of
/// border on each side, one of content.
pub(crate) const MIN_BORDERED_BAR_CELLS: u16 = 3;

/// Identity of the tree the desktop shell has always drawn.
///
/// Kept at its historical value so that the default path's cache key is byte
/// identical to the one it had before derivation existed.
pub(crate) const LEGACY_DESKTOP_REVISION: u64 = 1;

/// Where built-in template revisions start, far enough from the legacy value
/// that the two spaces can never be confused by eye in a log line.
const TEMPLATE_REVISION_BASE: u64 = 100;

/// Where configured-edge revisions start. Far from both other spaces for the
/// same reason: a revision in a log line should say which kind of tree it was.
const BAR_REVISION_BASE: u64 = 200;

/// A shell tree together with everything the geometry cache needs to know that
/// it is looking at that tree and not another one.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DerivedShellLayout {
    pub layout: ShellLayout,
    pub revision: u64,
    /// `None` means the legacy desktop tree, which is not a built-in template.
    pub template: Option<ShellTemplateId>,
}

/// The most parts one bar may be divided into.
///
/// The same number the shell tree allows a split, because it is the same
/// question with the same answer: an unbounded number of sections is CLA7's
/// unbounded visible chain wearing an edge bar for a hat.
pub(crate) const MAX_BAR_SECTIONS: usize = MAX_SPLIT_CHILDREN;

/// How one bar is divided along its long axis.
///
/// A fixed array rather than a `Vec` because this value travels inside the
/// geometry cache key, and the key must stay `Copy` and comparable by value:
/// putting the sections here is what makes CL3's rule ("the key contains every
/// input that decided the geometry") true by construction rather than by
/// somebody remembering to bump a revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct BarSections {
    policies: [Option<TrackPolicy>; MAX_BAR_SECTIONS],
    len: u8,
}

impl BarSections {
    /// An undivided bar: one strip, no sections, exactly what every bar is today.
    pub(crate) const NONE: Self = Self {
        policies: [None; MAX_BAR_SECTIONS],
        len: 0,
    };

    // TP-CHROME-23/24/31: the ceiling is reachable, exceeding it is refused,
    // and one unreadable entry costs the division rather than the numbering.
    /// Read an ordered list of sections, refusing rather than truncating.
    ///
    /// Nine sections is somebody's mistake, and silently keeping the first
    /// eight hands them a layout they did not write — the same reasoning that
    /// makes an out-of-range bar size a refusal rather than a clamp. The bar
    /// still draws; it just draws undivided, and the warning names the edge.
    pub(super) fn from_policies(policies: &[TrackPolicy], edge: &'static str) -> Self {
        if policies.len() > MAX_BAR_SECTIONS {
            tracing::warn!(
                edge,
                sections = policies.len(),
                max = MAX_BAR_SECTIONS,
                "a shell bar may not have this many sections; the bar is drawn undivided"
            );
            return Self::NONE;
        }
        let mut stored = [None; MAX_BAR_SECTIONS];
        for (slot, policy) in stored.iter_mut().zip(policies) {
            *slot = Some(*policy);
        }
        Self {
            policies: stored,
            len: policies.len() as u8,
        }
    }

    // How many parts a bar was divided into is what a caller asks before
    // addressing one; the production caller arrives with the widget catalogue
    // (F32-L7), which is the first thing that has anything to put in a section.
    #[allow(dead_code)]
    pub(crate) const fn len(self) -> usize {
        self.len as usize
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.len == 0
    }

    /// The policies in the order they were written, which is the order they are
    /// laid out and the order their indices address them by.
    fn policies(self) -> Vec<TrackPolicy> {
        self.policies.into_iter().flatten().collect()
    }
}

/// Where each of a bar's sections ended up.
///
/// Allocation-free for the same reason the sections themselves are: this is
/// answered on the geometry path and read on the drawing and hit-testing paths,
/// and none of the three should be paying for a heap allocation per bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct BarSectionRects {
    rects: [Rect; MAX_BAR_SECTIONS],
    len: u8,
}

impl BarSectionRects {
    pub(crate) const EMPTY: Self = Self {
        rects: [Rect::ZERO; MAX_BAR_SECTIONS],
        len: 0,
    };

    pub(crate) const fn len(self) -> usize {
        self.len as usize
    }

    // Both read a resolved division by index rather than by iteration, which is
    // what drawing one section's contents needs (F32-L7). The hit path uses
    // `occupied`, so these have no production caller until then.
    #[allow(dead_code)]
    pub(crate) const fn is_empty(self) -> bool {
        self.len == 0
    }

    #[allow(dead_code)]
    pub(crate) fn get(self, index: usize) -> Option<Rect> {
        (index < self.len()).then(|| self.rects[index])
    }

    /// Every section that actually got cells, with the index that addresses it.
    ///
    /// A section resolved to nothing is skipped rather than reported empty: a
    /// zero-width rectangle that still answered clicks would take its
    /// neighbour's, which is the quiet failure CL5 exists to prevent.
    pub(crate) fn occupied(self) -> impl Iterator<Item = (usize, Rect)> {
        (0..self.len())
            .map(move |index| (index, self.rects[index]))
            .filter(|(_, rect)| rect.width > 0 && rect.height > 0)
    }
}

/// How thick one edge's strip is, whether it is divided, or that there is none.
///
/// The cache keys on this value directly rather than on a digest of it: the
/// whole point of the geometry key is that two different screens can never
/// answer to the same identity, and a hash trades that certainty for brevity
/// nobody needs at a handful of small fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct BarTrack {
    cells: Option<u16>,
    /// Whether the strip wears a panel border. It costs one cell on each side,
    /// which is why a bordered bar is refused below three.
    bordered: bool,
    // TP-CHROME-30: the division lives inside the value the geometry key
    // compares, so a differently divided bar cannot share an identity.
    sections: BarSections,
}

impl BarTrack {
    pub(crate) const NONE: Self = Self {
        cells: None,
        bordered: false,
        sections: BarSections::NONE,
    };

    /// A strip of exactly this many cells. Bounds live in [`BarTrack::from_config`],
    /// where the number arrives from somebody's config file; a caller that
    /// already holds a checked number does not re-check it here.
    pub(crate) const fn of(cells: u16) -> Self {
        Self {
            cells: Some(cells),
            bordered: false,
            sections: BarSections::NONE,
        }
    }

    /// A bordered strip of exactly this many cells, border included.
    pub(crate) const fn bordered(cells: u16) -> Self {
        Self {
            cells: Some(cells),
            bordered: true,
            sections: BarSections::NONE,
        }
    }

    /// The same strip, divided as asked.
    pub(super) fn with_sections(self, sections: BarSections) -> Self {
        Self { sections, ..self }
    }

    pub(crate) const fn sections(self) -> BarSections {
        self.sections
    }

    // TP-CHROME-25..28: the shell's own allocator, the edge's own axis, the
    // border's own inset, and no target for a section that got no cells.
    /// Divide this bar's content area among its sections.
    ///
    /// `outer` is the whole strip as the shell solver placed it; the border is
    /// taken off here through the one function that knows how thick it is, so
    /// that drawing a section and clicking a section can never disagree about
    /// where it starts. That disagreement is C79/C80, and this line is where it
    /// would be born if the inset were computed anywhere else.
    ///
    /// The axis follows the edge: a top or bottom bar is divided across its
    /// width, a left or right bar down its height. Getting this backwards
    /// collapses every section onto the same cells and still looks like it
    /// works from a distance, which is why T29 names it.
    pub(crate) fn section_rects(self, region: RegionId, outer: Rect) -> BarSectionRects {
        if self.sections.is_empty() {
            return BarSectionRects::EMPTY;
        }
        let inner = self.inner(outer);
        if inner.width == 0 || inner.height == 0 {
            return BarSectionRects::EMPTY;
        }

        let horizontal = matches!(region, RegionId::TopBar | RegionId::BottomBar);
        let available = if horizontal {
            inner.width
        } else {
            inner.height
        };
        let lengths = allocate_section_lengths(&self.sections.policies(), available);

        let mut rects = [Rect::ZERO; MAX_BAR_SECTIONS];
        let mut offset = 0u16;
        for (slot, length) in rects.iter_mut().zip(&lengths) {
            let length = (*length).min(available.saturating_sub(offset));
            *slot = if horizontal {
                Rect::new(
                    inner.x.saturating_add(offset),
                    inner.y,
                    length,
                    inner.height,
                )
            } else {
                Rect::new(inner.x, inner.y.saturating_add(offset), inner.width, length)
            };
            offset = offset.saturating_add(length);
        }

        BarSectionRects {
            rects,
            len: self.sections.len,
        }
    }

    pub(crate) const fn has_border(self) -> bool {
        self.bordered
    }

    /// The area left for content once the border has taken its cells.
    ///
    /// One function for both the drawing and the hit testing: if those two ever
    /// computed the inset separately they would drift, and a click would land
    /// one cell away from what the person aimed at — the quiet failure CL5 is
    /// written against.
    pub(crate) fn inner(self, outer: Rect) -> Rect {
        if !self.bordered {
            return outer;
        }
        if outer.width < 3 || outer.height < 3 {
            return Rect::new(outer.x, outer.y, 0, 0);
        }
        Rect::new(outer.x + 1, outer.y + 1, outer.width - 2, outer.height - 2)
    }

    /// Read one edge, refusing rather than repairing.
    ///
    /// A disabled bar and a bar with an impossible size are the same answer —
    /// no strip — but only the second one is worth saying out loud, so the
    /// caller gets told which edge it was.
    // TP-CHROME-04/05: an impossible size is refused, never repaired.
    fn from_config(config: &ShellBarConfig, edge: &'static str) -> Self {
        if !config.enabled {
            return Self::NONE;
        }
        if let Some(problem) = bar_size_problem(config, edge) {
            tracing::warn!(%problem, "shell bar size refused; the bar is not drawn");
            return Self::NONE;
        }
        let track = if config.border {
            Self::bordered(config.size)
        } else {
            Self::of(config.size)
        };
        track.with_sections(sections_from_config(&config.sections, edge))
    }

    const fn enabled(self) -> bool {
        self.cells.is_some()
    }
}

/// Something written under `[shell.bars]` that this build cannot draw.
///
/// Carried as a typed value rather than a formatted string because two callers
/// need the same verdict from it: the derivation, which decides what to draw,
/// and `herdr config check`, which decides what to say. A second copy of these
/// range rules would drift the first time one of them changed and nothing
/// would go red — the failure C80 names, in its config-facing form.
///
/// Every variant here is *unusable*: a value that can never work, whatever
/// else the person does. That distinction is deliberate and load-bearing. A
/// setting that is merely *empty for now* — legitimate, and waiting on
/// something the person has not built yet — must not be reported as an issue,
/// because a false alarm on every new setup teaches people to stop reading the
/// checker. If such a case is ever added here, it needs its own severity
/// rather than a seat in this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BarConfigProblem {
    SizeOutOfRange {
        edge: &'static str,
        size: u16,
        max: u16,
    },
    BorderedBarTooThin {
        edge: &'static str,
        size: u16,
        minimum: u16,
    },
    TooManySections {
        edge: &'static str,
        sections: usize,
        max: usize,
    },
    UnknownSectionKind {
        edge: &'static str,
        index: usize,
        kind: String,
    },
    FixedSectionWithoutCells {
        edge: &'static str,
        index: usize,
    },
    ContentSectionMaxBelowMin {
        edge: &'static str,
        index: usize,
        min: u16,
        max: u16,
    },
    UnknownSectionActionKind {
        edge: &'static str,
        index: usize,
        kind: String,
    },
    PopupActionWithoutCommand {
        edge: &'static str,
        index: usize,
    },
}

impl std::fmt::Display for BarConfigProblem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Each message names the edge, and a section's message names its index
        // too: telling somebody that "a section is wrong" when a bar may hold
        // eight of them sends them looking through all eight.
        match self {
            Self::SizeOutOfRange { edge, size, max } => write!(
                formatter,
                "shell.bars.{edge}.size is {size}; a bar must be between 1 and {max} cells, \
                 so this bar is not drawn"
            ),
            Self::BorderedBarTooThin {
                edge,
                size,
                minimum,
            } => write!(
                formatter,
                "shell.bars.{edge}.size is {size} with a border; a bordered bar needs at least \
                 {minimum} cells to hold its border and a row of content, so this bar is not \
                 drawn"
            ),
            Self::TooManySections {
                edge,
                sections,
                max,
            } => write!(
                formatter,
                "shell.bars.{edge} has {sections} sections; a bar may hold at most {max}, \
                 so this bar is drawn undivided"
            ),
            Self::UnknownSectionKind { edge, index, kind } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].kind is \"{kind}\"; expected \"fixed\", \
                 \"fill\" or \"content\", so this bar is drawn undivided"
            ),
            Self::FixedSectionWithoutCells { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}] is fixed but asks for no cells, \
                 so this bar is drawn undivided"
            ),
            Self::ContentSectionMaxBelowMin {
                edge,
                index,
                min,
                max,
            } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}] allows at most {max} cells but demands \
                 at least {min}, so this bar is drawn undivided"
            ),
            // Unlike the sizing problems above, a refused action costs only its
            // own section: the bar still divides, and the part it names simply
            // stops answering clicks. Saying which part keeps that from reading
            // as "clicks are broken".
            Self::UnknownSectionActionKind { edge, index, kind } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action.kind is \"{kind}\"; expected \
                 \"popup\", so this section does nothing when clicked"
            ),
            Self::PopupActionWithoutCommand { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action opens a popup but names no command \
                 to run, so this section does nothing when clicked"
            ),
        }
    }
}

/// Why this edge's size cannot be drawn, if it cannot.
fn bar_size_problem(config: &ShellBarConfig, edge: &'static str) -> Option<BarConfigProblem> {
    if config.size == 0 || config.size > MAX_BAR_CELLS {
        return Some(BarConfigProblem::SizeOutOfRange {
            edge,
            size: config.size,
            max: MAX_BAR_CELLS,
        });
    }
    // Refused rather than drawn borderless: someone who asked for a bordered
    // bar and got a bare band would read it as the border failing, not as
    // their size being impossible — which is why this is its own message.
    if config.border && config.size < MIN_BORDERED_BAR_CELLS {
        return Some(BarConfigProblem::BorderedBarTooThin {
            edge,
            size: config.size,
            minimum: MIN_BORDERED_BAR_CELLS,
        });
    }
    None
}

fn section_count_problem(sections: usize, edge: &'static str) -> Option<BarConfigProblem> {
    (sections > MAX_BAR_SECTIONS).then_some(BarConfigProblem::TooManySections {
        edge,
        sections,
        max: MAX_BAR_SECTIONS,
    })
}

// TP-CHROME-35/36: the checker and the deriver read one predicate, so a
// setting that will not be drawn is also a setting that gets said out loud.
/// Everything under `[shell.bars]` that this build will refuse to draw.
///
/// Only enabled edges are examined: a disabled bar is not drawn either, but
/// that is what the person asked for, and reporting it would bury the real
/// complaints under noise.
pub(crate) fn shell_bar_config_problems(config: &ShellBarsConfig) -> Vec<BarConfigProblem> {
    let mut problems = Vec::new();
    for (bar, edge) in [
        (&config.top, "top"),
        (&config.bottom, "bottom"),
        (&config.left, "left"),
        (&config.right, "right"),
    ] {
        if !bar.enabled {
            continue;
        }
        if let Some(problem) = bar_size_problem(bar, edge) {
            problems.push(problem);
            // A bar that will not be drawn has nothing to divide, so its
            // sections are not examined: reporting them would ask somebody to
            // fix a table that is not the reason their bar is missing.
            continue;
        }
        if let Some(problem) = section_count_problem(bar.sections.len(), edge) {
            problems.push(problem);
            continue;
        }
        for (index, section) in bar.sections.iter().enumerate() {
            if let Err(problem) = section_policy(section, edge, index) {
                problems.push(problem);
            }
            // Asked separately from the sizing rule because the two have
            // different blast radii and a person fixing one should not have to
            // guess that the other was also refused.
            if let Err(problem) = section_action(section, edge, index) {
                problems.push(problem);
            }
        }
    }
    problems
}

/// Read one edge's sections, refusing the whole division if any part of it is
/// not something this build can draw.
///
/// All-or-nothing on purpose. Dropping only the section that did not parse
/// would silently renumber every section after it, and the indices are what
/// everything downstream addresses a section by — a config typo would then move
/// somebody's content rather than reporting itself.
fn sections_from_config(configs: &[ShellBarSectionConfig], edge: &'static str) -> BarSections {
    if configs.is_empty() {
        return BarSections::NONE;
    }
    match section_policies(configs, edge) {
        Ok(policies) => BarSections::from_policies(&policies, edge),
        Err(problem) => {
            tracing::warn!(%problem, "the bar is drawn undivided");
            BarSections::NONE
        }
    }
}

/// The sizing policies of one edge's sections, or the first reason the division
/// as a whole cannot stand.
///
/// Quiet by design: two callers need this verdict and only one of them should
/// speak. Extracting it is what keeps "is this division drawn" a single
/// predicate — the action table below asks the same question, and a second copy
/// of the rule would let a refused division keep an addressable action list.
fn section_policies(
    configs: &[ShellBarSectionConfig],
    edge: &'static str,
) -> Result<Vec<TrackPolicy>, BarConfigProblem> {
    if let Some(problem) = section_count_problem(configs.len(), edge) {
        return Err(problem);
    }
    configs
        .iter()
        .enumerate()
        .map(|(index, config)| section_policy(config, edge, index))
        .collect()
}

/// One section's table as a sizing policy, or what is wrong with it.
///
/// Returning the problem rather than swallowing it is what lets `herdr config
/// check` and the drawing path reach the same verdict from the same predicate.
fn section_policy(
    config: &ShellBarSectionConfig,
    edge: &'static str,
    index: usize,
) -> Result<TrackPolicy, BarConfigProblem> {
    match config.kind.as_str() {
        "fixed" => {
            if config.cells == 0 {
                return Err(BarConfigProblem::FixedSectionWithoutCells { edge, index });
            }
            Ok(TrackPolicy::Fixed {
                cells: config.cells,
            })
        }
        // A fill with no weight is the common shape of "just take the rest",
        // and refusing it would make the simplest section the one that needs
        // the most typing.
        "fill" => Ok(TrackPolicy::Fill {
            weight: config.weight.max(1),
        }),
        "content" => {
            if config.max < config.min {
                return Err(BarConfigProblem::ContentSectionMaxBelowMin {
                    edge,
                    index,
                    min: config.min,
                    max: config.max,
                });
            }
            Ok(TrackPolicy::ContentBounded {
                min: config.min,
                max: config.max,
            })
        }
        other => Err(BarConfigProblem::UnknownSectionKind {
            edge,
            index,
            kind: other.to_string(),
        }),
    }
}

/// Which of the four edges a region is the bar of.
///
/// The left bar is the dock's region, which is exactly why this exists: two
/// places need to answer "which bar is this region", and the answer is not the
/// one the names suggest. A second copy of this match is how a click on the
/// left bar starts running the right bar's command — the same divergence the
/// config checker and the deriver were joined to avoid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BarEdge {
    Top,
    Bottom,
    Left,
    Right,
}

// TP-CHROME-38: one mapping answers "which bar is this region" for the track
// the geometry carries and for the action a click resolves.
/// Fail-closed: a region that is not an edge bar is not one, rather than
/// falling into whichever arm happens to be last.
pub(crate) const fn bar_edge_for(region: RegionId) -> Option<BarEdge> {
    match region {
        RegionId::TopBar => Some(BarEdge::Top),
        RegionId::BottomBar => Some(BarEdge::Bottom),
        RegionId::AppDock => Some(BarEdge::Left),
        RegionId::RightPanel => Some(BarEdge::Right),
        RegionId::LeftPanel | RegionId::CenterContent | RegionId::WorkspaceStage => None,
    }
}

/// What a click on one section of a bar does.
///
/// A closed enum, like the sizing policies beside it and for the same reason
/// (CL8): adding a new thing a bar section can do should be a cost the compiler
/// counts at every place that resolves one, not a string somebody discovers at
/// runtime.
///
/// Deliberately small. "Focus this region" was considered and left out: bars
/// hold nothing focusable until the widget catalogue arrives (F32-L7), and a
/// variant whose arm does nothing is the dead component CLA9 names. The closed
/// shape is what makes adding it later cheap and visible.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum SectionAction {
    /// The section is an indicator. Clicks over it are consumed — bars are
    /// chrome, and an event that fell through to whatever is underneath would
    /// act on a surface the person was not pointing at (CL12).
    #[default]
    None,
    OpenPopup {
        argv: Vec<String>,
    },
}

/// One edge's actions, in the same order and addressed by the same indices as
/// that edge's sections.
///
/// Index-aligned with [`BarSections`] by construction: both are derived from
/// the same config list through the same refusal predicate, so a division that
/// was refused cannot leave behind an action list whose indices address the
/// sections of some other, imagined bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BarSectionActions {
    actions: Vec<SectionAction>,
}

impl BarSectionActions {
    pub(crate) const EMPTY: Self = Self {
        actions: Vec::new(),
    };

    fn get(&self, index: u8) -> Option<&SectionAction> {
        self.actions.get(usize::from(index))
    }
}

/// What clicking each part of each edge does.
///
/// Held apart from [`ShellBars`] on purpose. `ShellBars` travels inside the
/// geometry cache key, and actions do not decide geometry: folding a command
/// line into the key would make editing that command invalidate every cached
/// rectangle on screen, for a change that moves nothing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ShellBarActions {
    top: BarSectionActions,
    bottom: BarSectionActions,
    left: BarSectionActions,
    right: BarSectionActions,
}

impl ShellBarActions {
    pub(crate) fn from_config(config: &ShellBarsConfig) -> Self {
        Self {
            top: bar_section_actions(&config.top, "top"),
            bottom: bar_section_actions(&config.bottom, "bottom"),
            left: bar_section_actions(&config.left, "left"),
            right: bar_section_actions(&config.right, "right"),
        }
    }

    /// What the numbered section of the named region does, if that region is
    /// an edge bar and that section carries an action.
    ///
    /// Resolves the region through the same [`bar_edge_for`] the geometry side
    /// uses, so the bar a click is attributed to and the bar it was drawn from
    /// can never be two different bars.
    pub(crate) fn action_for(&self, region: RegionId, index: u8) -> Option<&SectionAction> {
        let edge = match bar_edge_for(region)? {
            BarEdge::Top => &self.top,
            BarEdge::Bottom => &self.bottom,
            BarEdge::Left => &self.left,
            BarEdge::Right => &self.right,
        };
        edge.get(index)
    }
}

/// Read one edge's click actions, aligned with the sections that edge actually
/// has.
///
/// A refused division yields no actions at all: the indices an action list is
/// addressed by are the section indices, and there are none.
///
/// A single unreadable action costs only itself. That asymmetry with the sizing
/// rules is deliberate — a misspelled command name should not take the whole
/// bar's layout down with it, and leaving the section in place with no action
/// keeps every other index pointing where it pointed.
// TP-CHROME-37/39/40: actions answer at the index they were written at, a
// refused division leaves none, and a refused action costs only its section.
fn bar_section_actions(config: &ShellBarConfig, edge: &'static str) -> BarSectionActions {
    if !config.enabled || config.sections.is_empty() {
        return BarSectionActions::EMPTY;
    }
    if bar_size_problem(config, edge).is_some() {
        return BarSectionActions::EMPTY;
    }
    if section_policies(&config.sections, edge).is_err() {
        return BarSectionActions::EMPTY;
    }
    let actions = config
        .sections
        .iter()
        .enumerate()
        .map(
            |(index, section)| match section_action(section, edge, index) {
                Ok(action) => action,
                Err(problem) => {
                    tracing::warn!(%problem, "the section is drawn without a click action");
                    SectionAction::None
                }
            },
        )
        .collect();
    BarSectionActions { actions }
}

/// One section's action table as an action, or what is wrong with it.
fn section_action(
    config: &ShellBarSectionConfig,
    edge: &'static str,
    index: usize,
) -> Result<SectionAction, BarConfigProblem> {
    match config.action.kind.as_str() {
        "" => Ok(SectionAction::None),
        "popup" => {
            // An empty argv, or one made only of blanks, would ask the runtime
            // to execute nothing. Disk is untrusted input (CL1): refuse it here
            // rather than discovering it at the moment somebody clicks.
            if config
                .action
                .argv
                .iter()
                .all(|argument| argument.trim().is_empty())
            {
                return Err(BarConfigProblem::PopupActionWithoutCommand { edge, index });
            }
            Ok(SectionAction::OpenPopup {
                argv: config.action.argv.clone(),
            })
        }
        other => Err(BarConfigProblem::UnknownSectionActionKind {
            edge,
            index,
            kind: other.to_string(),
        }),
    }
}

/// The four edges, as the tree builder sees them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct ShellBars {
    pub top: BarTrack,
    pub bottom: BarTrack,
    pub left: BarTrack,
    pub right: BarTrack,
}

impl ShellBars {
    /// What everyone has today: no strip on any edge.
    pub(crate) const NONE: Self = Self {
        top: BarTrack::NONE,
        bottom: BarTrack::NONE,
        left: BarTrack::NONE,
        right: BarTrack::NONE,
    };

    pub(crate) fn from_config(config: &ShellBarsConfig) -> Self {
        Self {
            top: BarTrack::from_config(&config.top, "top"),
            bottom: BarTrack::from_config(&config.bottom, "bottom"),
            left: BarTrack::from_config(&config.left, "left"),
            right: BarTrack::from_config(&config.right, "right"),
        }
    }

    /// The track that owns one edge region.
    pub(crate) const fn track_for(self, region: RegionId) -> BarTrack {
        match bar_edge_for(region) {
            Some(BarEdge::Top) => self.top,
            Some(BarEdge::Bottom) => self.bottom,
            Some(BarEdge::Left) => self.left,
            Some(BarEdge::Right) => self.right,
            None => BarTrack::NONE,
        }
    }

    const fn any_enabled(self) -> bool {
        self.top.enabled() || self.bottom.enabled() || self.left.enabled() || self.right.enabled()
    }
}

/// What one edge's border is painted with: a tone, or a fade between two.
///
/// A fade needs real channel values at both ends. A named terminal colour has
/// none — `Color::Yellow` is whatever the terminal decides — so a gradient that
/// names one falls back to the solid tone instead of inventing numbers for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BarTint {
    start: Color,
    end: Option<Color>,
}

impl BarTint {
    pub(crate) const fn solid(color: Color) -> Self {
        Self {
            start: color,
            end: None,
        }
    }

    /// The tone at `position` of `span` cells along the bar's long axis.
    ///
    /// A solid tint ignores both, which is what makes this the only colour
    /// lookup the renderer needs.
    pub(crate) fn at(self, position: u16, span: u16) -> Color {
        let (Some(end), true) = (self.end, span > 1) else {
            return self.start;
        };
        let (Color::Rgb(r0, g0, b0), Color::Rgb(r1, g1, b1)) = (self.start, end) else {
            return self.start;
        };
        let t = f32::from(position.min(span - 1)) / f32::from(span - 1);
        let mix = |a: u8, b: u8| {
            (f32::from(a) + (f32::from(b) - f32::from(a)) * t)
                .round()
                .clamp(0.0, 255.0) as u8
        };
        Color::Rgb(mix(r0, r1), mix(g0, g1), mix(b0, b1))
    }

    #[cfg(test)]
    pub(crate) const fn fades(self) -> bool {
        self.end.is_some()
    }
}

/// What each edge's border is painted with.
///
/// Kept beside the geometry rather than inside it: a colour never moves a
/// rectangle, so putting it in the cache key would throw away a correct
/// projection every time somebody edited a theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BarColors {
    pub top: BarTint,
    pub bottom: BarTint,
    pub left: BarTint,
    pub right: BarTint,
}

impl BarColors {
    /// The warm default, spelled as a constant so a presentation can be built
    /// in a const context before any palette exists.
    pub(crate) const DEFAULT_CONST: Self = {
        let peach = BarTint::solid(Color::Rgb(250, 179, 135));
        Self {
            top: peach,
            bottom: peach,
            left: peach,
            right: peach,
        }
    };

    pub(crate) fn from_config(config: &ShellBarsConfig, palette: &Palette) -> Self {
        Self {
            top: bar_tint(&config.top, palette, "top"),
            bottom: bar_tint(&config.bottom, palette, "bottom"),
            left: bar_tint(&config.left, palette, "left"),
            right: bar_tint(&config.right, palette, "right"),
        }
    }

    pub(crate) const fn for_region(self, region: RegionId) -> BarTint {
        match region {
            RegionId::TopBar => self.top,
            RegionId::BottomBar => self.bottom,
            RegionId::AppDock => self.left,
            _ => self.right,
        }
    }
}

impl Default for BarColors {
    fn default() -> Self {
        Self::DEFAULT_CONST
    }
}

/// Read one edge's tint: a fade when both ends carry channel values, the solid
/// tone otherwise.
fn bar_tint(config: &ShellBarConfig, palette: &Palette, edge: &'static str) -> BarTint {
    tint_from_parts(&config.color, &config.gradient, palette, edge)
}

/// One tint reader for every framed surface, so a colour written for a bar and
/// a colour written for the left panel mean the same thing.
// TP-CHROME-08..10: palette token first, literal second, and a gradient that
// cannot interpolate says so instead of fading to nothing.
fn tint_from_parts(
    color: &str,
    gradient: &[String],
    palette: &Palette,
    edge: &'static str,
) -> BarTint {
    let solid = bar_color(color, palette);
    let stops: Vec<Color> = gradient
        .iter()
        .map(|spec| bar_color(spec, palette))
        .collect();
    match stops.as_slice() {
        [] => BarTint::solid(solid),
        [only] => {
            tracing::warn!(
                edge,
                "a gradient needs two ends; the single stop is used as a solid tone"
            );
            BarTint::solid(*only)
        }
        [first, .., last] => {
            if matches!(first, Color::Rgb(..)) && matches!(last, Color::Rgb(..)) {
                BarTint {
                    start: *first,
                    end: Some(*last),
                }
            } else {
                // A named terminal colour has no channel values to walk
                // between; guessing them would paint a fade the person never
                // described.
                tracing::warn!(
                    edge,
                    "a gradient end has no channel values; falling back to the solid tone"
                );
                BarTint::solid(solid)
            }
        }
    }
}

/// A palette token first, a literal colour second.
///
/// Writing `accent` should follow the theme the way every other herdr surface
/// does; writing `#fab387` should mean exactly that. An empty setting is the
/// warm default, which is what makes an unconfigured bar look deliberate
/// rather than like a rendering fault.
fn bar_color(spec: &str, palette: &Palette) -> Color {
    match spec.trim().to_lowercase().as_str() {
        "" => palette.peach,
        "accent" => palette.accent,
        "text" => palette.text,
        "mauve" => palette.mauve,
        "green" => palette.green,
        "yellow" => palette.yellow,
        "red" => palette.red,
        "blue" => palette.blue,
        "teal" => palette.teal,
        "peach" | "orange" => palette.peach,
        "surface" | "dim" => palette.surface_dim,
        other => parse_color(other),
    }
}

/// Whether each half of the left panel wears a frame, and in what tone.
///
/// Copy and tiny so the one function that projects both section rectangles can
/// take it directly. That matters more than it looks: drawing and hit testing
/// read those rectangles from the same place, so an inset applied there cannot
/// drift between what is painted and what a click resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct SidebarChrome {
    pub spaces: Option<BarTint>,
    pub agents: Option<BarTint>,
    /// When set, the panel's own controls are drawn as framed chips, and the
    /// footer grows from one row to `CHIP_ROWS` to hold them.
    pub chips: Option<BarTint>,
}

impl SidebarChrome {
    pub(crate) const NONE: Self = Self {
        spaces: None,
        agents: None,
        chips: None,
    };

    pub(crate) fn from_config(config: &crate::config::SidebarConfig, palette: &Palette) -> Self {
        Self {
            spaces: section_tint(&config.spaces.border, palette),
            agents: section_tint(&config.agents.border, palette),
            chips: section_tint(&config.chips, palette),
        }
    }

    /// How many rows the sidebar footer occupies — the single source both the
    /// list's bottom edge and the footer's own rectangle read.
    ///
    /// A chip is a frame, and a frame needs its own rows; a bare label needs
    /// one. Every place that carves the footer out of the list must ask here,
    /// because a row counted differently in two places puts the list and the
    /// buttons on top of each other without any of them being empty or
    /// out of bounds — the failure C80 describes.
    // TP-CHROME-17: one answer for how many rows the footer owns.
    pub(crate) fn footer_rows(self) -> u16 {
        if self.chips.is_some() {
            crate::ui::widgets::CHIP_ROWS
        } else {
            1
        }
    }

    /// How wide a control is: `bare_cells` as a plain label, or exactly what a
    /// chip around `label` asks for.
    ///
    /// A chip sizes itself to its label and clips to whatever it is given, so
    /// handing it this width makes the drawn frame and the clickable rectangle
    /// the same rectangle for both a short label and one too long to fit. A
    /// rectangle chosen independently would leave cells that look like nothing
    /// and click like a button.
    pub(crate) fn control_width(self, bare_cells: u16, label: &str) -> u16 {
        if self.chips.is_some() {
            crate::ui::widgets::chip_width(label)
        } else {
            bare_cells
        }
    }
}

fn section_tint(config: &crate::config::SectionBorderConfig, palette: &Palette) -> Option<BarTint> {
    if !config.enabled {
        return None;
    }
    Some(tint_from_parts(
        &config.color,
        &config.gradient,
        palette,
        "sidebar",
    ))
}

/// Derive the desktop shell tree from what the user asked for.
///
/// Asking for nothing with no bars is today's production request and yields
/// exactly today's tree. A requested template owns the whole composition, so
/// bars are not composed onto it — a template already names every region it
/// wants, and adding an edge it already owns would produce a duplicate region
/// and fail the whole tree rather than the one bar.
// TP-CHROME-01..03/06/07: the edges a person asked for, the identity that
// tells two compositions apart, and the fallback when one does not validate.
pub(crate) fn derive_desktop_shell_layout(
    requested: Option<ShellTemplateId>,
    bars: ShellBars,
) -> DerivedShellLayout {
    if let Some(template) = requested {
        return finish(template, template.validated_layout());
    }
    if !bars.any_enabled() {
        return legacy_desktop_layout();
    }
    finish_bars(bars, bar_layout(bars))
}

/// Compose the legacy tree with whichever edges were asked for.
///
/// Only enabled edges become nodes, and only enabled edges get a track policy.
/// That omission is load-bearing: a region without a policy falls back to the
/// runtime size the caller resolves, which is how the left panel keeps the
/// width the person dragged it to instead of being reset by the mere presence
/// of a top bar.
fn bar_layout(bars: ShellBars) -> ShellLayout {
    let mut tracks = Vec::new();
    let mut body = Vec::new();

    if let Some(cells) = bars.left.cells {
        body.push(dynamic_child(RegionId::AppDock));
        tracks.push((RegionId::AppDock, TrackPolicy::Fixed { cells }));
    }
    body.push(dynamic_child(RegionId::LeftPanel));
    body.push(fill_child(RegionId::WorkspaceStage));
    if let Some(cells) = bars.right.cells {
        body.push(dynamic_child(RegionId::RightPanel));
        tracks.push((RegionId::RightPanel, TrackPolicy::Fixed { cells }));
    }

    let body = ShellNode::Split {
        direction: ShellDirection::Horizontal,
        children: body,
    };

    let root = if bars.top.enabled() || bars.bottom.enabled() {
        let mut rows = Vec::new();
        if let Some(cells) = bars.top.cells {
            rows.push(dynamic_child(RegionId::TopBar));
            tracks.push((RegionId::TopBar, TrackPolicy::Fixed { cells }));
        }
        rows.push(ShellChild {
            size: RegionSize::Fill,
            node: body,
        });
        if let Some(cells) = bars.bottom.cells {
            rows.push(dynamic_child(RegionId::BottomBar));
            tracks.push((RegionId::BottomBar, TrackPolicy::Fixed { cells }));
        }
        ShellNode::Split {
            direction: ShellDirection::Vertical,
            children: rows,
        }
    } else {
        body
    };

    ShellLayout::from_parts(root, tracks.into_iter().collect(), Vec::new(), Vec::new())
}

/// Every non-filling child is `Dynamic`; what makes a bar a fixed thickness is
/// its track policy, not its node. Keeping one constructor here means the node
/// and the policy can never disagree about which is in charge.
fn dynamic_child(region: RegionId) -> ShellChild {
    ShellChild {
        size: RegionSize::Dynamic,
        node: ShellNode::Slot { region },
    }
}

fn fill_child(region: RegionId) -> ShellChild {
    ShellChild {
        size: RegionSize::Fill,
        node: ShellNode::Slot { region },
    }
}

/// The seam where a bar composition becomes a tree.
///
/// Separated for the same reason as [`finish`]: the refusal has to be reachable
/// from a test. A composition that will not validate falls back to the legacy
/// tree, so a bad `[shell.bars]` table costs the person their bars, not their
/// editor.
fn finish_bars(bars: ShellBars, layout: ShellLayout) -> DerivedShellLayout {
    match layout.clone().validate() {
        Ok(_) => DerivedShellLayout {
            layout,
            revision: revision_for_bars(bars),
            template: None,
        },
        Err(error) => {
            tracing::warn!(%error, "shell bars do not compose; drawing the default shell");
            legacy_desktop_layout()
        }
    }
}

/// A revision per distinct edge composition.
///
/// Sixteen on/off combinations times the thickness of each edge is more than a
/// small integer can name, so the identity that the cache actually compares is
/// the `ShellBars` value in the geometry key. This number only has to move when
/// the tree does, and it does.
fn revision_for_bars(bars: ShellBars) -> u64 {
    BAR_REVISION_BASE
        + u64::from(bars.top.enabled())
        + (u64::from(bars.bottom.enabled()) << 1)
        + (u64::from(bars.left.enabled()) << 2)
        + (u64::from(bars.right.enabled()) << 3)
}

/// The seam where a validation verdict becomes a tree.
///
/// Separated so the fail-closed branch is reachable from a test: the five
/// built-in templates all validate today, and a guard that cannot be exercised
/// is a guard nobody can trust.
fn finish(
    template: ShellTemplateId,
    validated: Result<ValidatedShellLayout, ShellValidationError>,
) -> DerivedShellLayout {
    match validated {
        Ok(valid) => DerivedShellLayout {
            layout: valid.as_layout().clone(),
            revision: revision_for(template),
            template: Some(template),
        },
        // The identity falls back with the tree. Reporting a template we did
        // not draw would poison the cache key with a lie.
        Err(_) => legacy_desktop_layout(),
    }
}

fn legacy_desktop_layout() -> DerivedShellLayout {
    DerivedShellLayout {
        layout: ShellLayout::default(),
        revision: LEGACY_DESKTOP_REVISION,
        template: None,
    }
}

const fn revision_for(template: ShellTemplateId) -> u64 {
    TEMPLATE_REVISION_BASE
        + match template {
            ShellTemplateId::StageOnly => 0,
            ShellTemplateId::DockStage => 1,
            ShellTemplateId::DockSidebarStage => 2,
            ShellTemplateId::DesktopWorkspace => 3,
            ShellTemplateId::InspectorWorkspace => 4,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_TEMPLATES: [ShellTemplateId; 5] = [
        ShellTemplateId::StageOnly,
        ShellTemplateId::DockStage,
        ShellTemplateId::DockSidebarStage,
        ShellTemplateId::DesktopWorkspace,
        ShellTemplateId::InspectorWorkspace,
    ];

    // T5 · the default path is the path that already exists
    #[test]
    fn asking_for_nothing_derives_exactly_todays_tree() {
        // The whole promise of this layer: introducing a derivation must not
        // move a single cell. If this drifts, every visual baseline the layout
        // lock protects drifts with it, and nothing else would say so.
        let derived = derive_desktop_shell_layout(None, ShellBars::NONE);
        assert_eq!(derived.layout, ShellLayout::default());
        assert_eq!(derived.revision, LEGACY_DESKTOP_REVISION);
        assert_eq!(derived.template, None);
    }

    // T7 · whatever is derived is composable — the stage always survives
    #[test]
    fn every_derived_tree_still_validates() {
        for template in ALL_TEMPLATES {
            let derived = derive_desktop_shell_layout(Some(template), ShellBars::NONE);
            assert!(
                derived.layout.clone().validate().is_ok(),
                "{template:?} derived a tree that cannot be composed"
            );
        }
        assert!(derive_desktop_shell_layout(None, ShellBars::NONE)
            .layout
            .validate()
            .is_ok());
    }

    // T9 · a different tree is a different identity, or the cache lies
    #[test]
    fn every_template_carries_its_own_revision() {
        let mut seen = vec![LEGACY_DESKTOP_REVISION];
        for template in ALL_TEMPLATES {
            let derived = derive_desktop_shell_layout(Some(template), ShellBars::NONE);
            assert_eq!(derived.template, Some(template));
            assert!(
                !seen.contains(&derived.revision),
                "{template:?} reuses a revision another tree already claimed"
            );
            seen.push(derived.revision);
        }
    }

    // T6 · a tree that will not compose falls back, and says so in its identity
    #[test]
    fn a_template_that_does_not_validate_falls_back_to_the_legacy_tree() {
        // Reached through the seam because all five built-ins validate today.
        // A guard that cannot be exercised is a guard nobody can trust, and the
        // interesting half is the IDENTITY: claiming a template we did not draw
        // would key the cache on a tree that is not on screen.
        let derived = finish(
            ShellTemplateId::DesktopWorkspace,
            Err(ShellValidationError::MissingWorkspaceStage),
        );
        assert_eq!(derived.layout, ShellLayout::default());
        assert_eq!(derived.revision, LEGACY_DESKTOP_REVISION);
        assert_eq!(derived.template, None);
    }

    fn bar(cells: u16) -> BarTrack {
        BarTrack::of(cells)
    }

    fn config_bar(enabled: bool, size: u16) -> ShellBarConfig {
        ShellBarConfig {
            enabled,
            size,
            border: false,
            color: String::new(),
            gradient: Vec::new(),
            sections: Vec::new(),
        }
    }

    fn plain_section(kind: &str) -> ShellBarSectionConfig {
        ShellBarSectionConfig {
            kind: kind.to_string(),
            cells: 4,
            max: 4,
            ..Default::default()
        }
    }

    fn section_with_action(kind: &str, action_kind: &str, argv: &[&str]) -> ShellBarSectionConfig {
        let mut section = plain_section(kind);
        section.action.kind = action_kind.to_string();
        section.action.argv = argv.iter().map(|argument| argument.to_string()).collect();
        section
    }

    fn bar_with_sections(sections: Vec<ShellBarSectionConfig>) -> ShellBarConfig {
        ShellBarConfig {
            enabled: true,
            size: 1,
            border: false,
            color: String::new(),
            gradient: Vec::new(),
            sections,
        }
    }

    fn popup_argv(actions: &ShellBarActions, region: RegionId, index: u8) -> Option<Vec<String>> {
        match actions.action_for(region, index) {
            Some(SectionAction::OpenPopup { argv }) => Some(argv.clone()),
            _ => None,
        }
    }

    // TA-2 · the action reaches the index that addresses it, and no other.
    #[test]
    fn a_section_action_answers_at_the_index_it_was_written_at() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                plain_section("fill"),
                section_with_action("fill", "popup", &["btop", "--utf-force"]),
                plain_section("fill"),
            ]),
            ..Default::default()
        };

        let actions = ShellBarActions::from_config(&config);

        assert_eq!(
            popup_argv(&actions, RegionId::TopBar, 1).as_deref(),
            Some(["btop".to_string(), "--utf-force".to_string()].as_slice()),
            "the argv must arrive exactly as it was written"
        );
        assert_eq!(
            actions.action_for(RegionId::TopBar, 0),
            Some(&SectionAction::None),
            "a neighbour without an action must not inherit one"
        );
        assert_eq!(
            actions.action_for(RegionId::TopBar, 2),
            Some(&SectionAction::None)
        );
        assert_eq!(
            actions.action_for(RegionId::TopBar, 3),
            None,
            "an index past the division addresses nothing"
        );
    }

    // The left bar is the DOCK's region. Two mappings would let a click on the
    // left bar run the right bar's command; this pins that there is one.
    #[test]
    fn each_edge_s_actions_answer_at_the_region_its_track_is_drawn_in() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section_with_action("fill", "popup", &["top-cmd"])]),
            bottom: bar_with_sections(vec![section_with_action("fill", "popup", &["bottom-cmd"])]),
            left: bar_with_sections(vec![section_with_action("fill", "popup", &["left-cmd"])]),
            right: bar_with_sections(vec![section_with_action("fill", "popup", &["right-cmd"])]),
        };

        let actions = ShellBarActions::from_config(&config);
        let bars = ShellBars::from_config(&config);

        for (region, expected) in [
            (RegionId::TopBar, "top-cmd"),
            (RegionId::BottomBar, "bottom-cmd"),
            (RegionId::AppDock, "left-cmd"),
            (RegionId::RightPanel, "right-cmd"),
        ] {
            assert_eq!(
                popup_argv(&actions, region, 0).as_deref(),
                Some([expected.to_string()].as_slice()),
                "{region:?} must resolve the action of the bar drawn in it"
            );
            assert!(
                !bars.track_for(region).sections().is_empty(),
                "{region:?} must be the same region that carries that bar's division"
            );
        }

        // A region that is not an edge bar answers nothing rather than falling
        // into whichever arm happens to be last.
        for region in [
            RegionId::LeftPanel,
            RegionId::CenterContent,
            RegionId::WorkspaceStage,
        ] {
            assert_eq!(actions.action_for(region, 0), None, "{region:?}");
            assert!(bars.track_for(region).sections().is_empty(), "{region:?}");
        }
    }

    // TA-10 · a division that was refused leaves nothing addressable behind.
    // Actions survive on their own indices, so an action list that outlived its
    // sections would run the command of a section that is not on screen.
    #[test]
    fn a_refused_division_leaves_no_addressable_actions() {
        let too_many = (0..=MAX_BAR_SECTIONS)
            .map(|_| section_with_action("fill", "popup", &["btop"]))
            .collect::<Vec<_>>();
        let config = ShellBarsConfig {
            top: bar_with_sections(too_many),
            ..Default::default()
        };

        let bars = ShellBars::from_config(&config);
        let actions = ShellBarActions::from_config(&config);

        assert!(
            bars.top.sections().is_empty(),
            "control: the division itself must be refused"
        );
        for index in 0..=(MAX_BAR_SECTIONS as u8) {
            assert_eq!(
                actions.action_for(RegionId::TopBar, index),
                None,
                "index {index} must address nothing once the division is refused"
            );
        }
    }

    // D53-8 · an unreadable action costs its own section, not the whole bar.
    // The asymmetry with the sizing rules is the point: a misspelled command
    // must not move somebody's layout, and the indices around it must not shift.
    #[test]
    fn an_unusable_action_costs_only_the_section_that_carries_it() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                section_with_action("fill", "popup", &["first"]),
                section_with_action("fill", "teleport", &["nowhere"]),
                section_with_action("fill", "popup", &["third"]),
            ]),
            ..Default::default()
        };

        let bars = ShellBars::from_config(&config);
        let actions = ShellBarActions::from_config(&config);

        assert_eq!(
            bars.top.sections().len(),
            3,
            "the division must survive an action it cannot run"
        );
        assert_eq!(
            popup_argv(&actions, RegionId::TopBar, 0).as_deref(),
            Some(["first".to_string()].as_slice())
        );
        assert_eq!(
            actions.action_for(RegionId::TopBar, 1),
            Some(&SectionAction::None),
            "the unreadable action is dropped, and only it"
        );
        assert_eq!(
            popup_argv(&actions, RegionId::TopBar, 2).as_deref(),
            Some(["third".to_string()].as_slice()),
            "the section after it keeps its own index and its own action"
        );
    }

    // TA-8/TA-9 · a value that cannot be run is said out loud, by the same
    // predicate that refuses it — the join #54 established, extended to actions.
    #[test]
    fn config_check_reports_an_action_this_build_cannot_run() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                section_with_action("fill", "teleport", &["nowhere"]),
                section_with_action("fill", "popup", &[]),
                section_with_action("fill", "popup", &["  "]),
            ]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            reported.len(),
            3,
            "each unusable action is reported once: {reported:?}"
        );
        assert!(
            reported[0].contains("sections[0].action.kind is \"teleport\""),
            "{reported:?}"
        );
        assert!(
            reported[1].contains("sections[1].action opens a popup but names no command"),
            "{reported:?}"
        );
        assert!(
            reported[2].contains("sections[2].action opens a popup but names no command"),
            "a command made only of blanks names no command: {reported:?}"
        );

        // A section with no action at all is not a complaint: most sections
        // will never have one, and a checker that cries on every setup stops
        // being read.
        let quiet = ShellBarsConfig {
            top: bar_with_sections(vec![plain_section("fill")]),
            ..Default::default()
        };
        assert!(shell_bar_config_problems(&quiet).is_empty());
    }

    fn bordered_config_bar(size: u16) -> ShellBarConfig {
        ShellBarConfig {
            enabled: true,
            size,
            border: true,
            color: String::new(),
            gradient: Vec::new(),
            sections: Vec::new(),
        }
    }

    // T17 · most people never want an edge bar, and must not pay for one.
    #[test]
    fn no_configured_bar_derives_exactly_todays_tree() {
        // Structural equality with `default()` includes the empty track map,
        // which is what keeps the solver on its legacy branch. A tree that
        // merely *looks* the same but carries policies would take the tracked
        // branch and re-derive every width from scratch.
        let derived = derive_desktop_shell_layout(None, ShellBars::NONE);
        assert_eq!(derived.layout, ShellLayout::default());
        assert_eq!(derived.revision, LEGACY_DESKTOP_REVISION);
        assert_eq!(derived.template, None);
    }

    // T18 · "whichever side they want" — each edge, on its own.
    #[test]
    fn each_edge_can_carry_a_bar_by_itself() {
        let cases = [
            (
                "top",
                ShellBars {
                    top: bar(1),
                    ..ShellBars::NONE
                },
                RegionId::TopBar,
            ),
            (
                "bottom",
                ShellBars {
                    bottom: bar(1),
                    ..ShellBars::NONE
                },
                RegionId::BottomBar,
            ),
            (
                "left",
                ShellBars {
                    left: bar(6),
                    ..ShellBars::NONE
                },
                RegionId::AppDock,
            ),
            (
                "right",
                ShellBars {
                    right: bar(12),
                    ..ShellBars::NONE
                },
                RegionId::RightPanel,
            ),
        ];

        for (edge, bars, region) in cases {
            let derived = derive_desktop_shell_layout(None, bars);
            assert!(
                derived.layout.clone().validate().is_ok(),
                "{edge} bar derived a tree that cannot be composed"
            );
            assert!(
                derived.layout.tracks.contains_key(&region),
                "{edge} bar did not size its own region"
            );
            assert_ne!(
                derived.revision, LEGACY_DESKTOP_REVISION,
                "{edge} bar must not share the legacy identity"
            );
            // The left panel keeps whatever width the runtime resolves, which
            // is the whole reason its policy is absent rather than defaulted.
            assert!(
                !derived.layout.tracks.contains_key(&RegionId::LeftPanel),
                "{edge} bar silently took over the sidebar width"
            );
        }
    }

    // T19 · bars may not eat the stage.
    #[test]
    fn all_four_bars_at_once_still_leave_a_stage() {
        let bars = ShellBars {
            top: bar(1),
            bottom: bar(1),
            left: bar(6),
            right: bar(12),
        };
        let derived = derive_desktop_shell_layout(None, bars);

        derived
            .layout
            .clone()
            .validate()
            .expect("four bars must still compose around the stage");
        assert!(
            !derived
                .layout
                .tracks
                .contains_key(&RegionId::WorkspaceStage),
            "the stage keeps its fill share rather than a bar-sized policy"
        );
        assert_eq!(derived.template, None);
    }

    // T18b · every edge composition has its own identity, or the cache lies.
    #[test]
    fn every_edge_composition_has_its_own_revision() {
        let mut seen = vec![LEGACY_DESKTOP_REVISION];
        for mask in 1u8..16 {
            let bars = ShellBars {
                top: if mask & 1 != 0 {
                    bar(1)
                } else {
                    BarTrack::NONE
                },
                bottom: if mask & 2 != 0 {
                    bar(1)
                } else {
                    BarTrack::NONE
                },
                left: if mask & 4 != 0 {
                    bar(6)
                } else {
                    BarTrack::NONE
                },
                right: if mask & 8 != 0 {
                    bar(12)
                } else {
                    BarTrack::NONE
                },
            };
            let revision = derive_desktop_shell_layout(None, bars).revision;
            assert!(
                !seen.contains(&revision),
                "mask {mask:04b} reuses a revision another composition already claimed"
            );
            seen.push(revision);
        }
    }

    // T20 · the config file is somebody's typing, not a promise.
    #[test]
    fn an_impossible_bar_size_is_refused_rather_than_repaired() {
        // Clamping would be friendlier and worse: the person would get a bar
        // they did not write, at a size they cannot find in their own config.
        for size in [0, MAX_BAR_CELLS + 1, u16::MAX] {
            let config = ShellBarsConfig {
                top: config_bar(true, size),
                ..ShellBarsConfig::default()
            };
            assert_eq!(
                ShellBars::from_config(&config),
                ShellBars::NONE,
                "size {size} must leave no bar behind"
            );
        }

        let usable = ShellBarsConfig {
            top: config_bar(true, MAX_BAR_CELLS),
            ..ShellBarsConfig::default()
        };
        assert_eq!(ShellBars::from_config(&usable).top, bar(MAX_BAR_CELLS));
    }

    // T34 · a border needs room, and a bar that cannot have one is refused
    // rather than quietly drawn bare.
    #[test]
    fn a_bordered_bar_thinner_than_its_border_is_refused() {
        // Drawing it borderless instead would read as the border failing, not
        // as the size being impossible — and the person would go looking in the
        // wrong place.
        for size in [1, 2] {
            let config = ShellBarsConfig {
                top: bordered_config_bar(size),
                ..ShellBarsConfig::default()
            };
            assert_eq!(
                ShellBars::from_config(&config).top,
                BarTrack::NONE,
                "size {size} leaves nothing inside the border"
            );
        }

        let usable = ShellBarsConfig {
            top: bordered_config_bar(MIN_BORDERED_BAR_CELLS),
            ..ShellBarsConfig::default()
        };
        let track = ShellBars::from_config(&usable).top;
        assert!(track.has_border());
        assert_eq!(track.cells, Some(MIN_BORDERED_BAR_CELLS));
    }

    // T35 · a bare one-cell strip stays legal; the border is a choice.
    #[test]
    fn an_unbordered_bar_may_be_a_single_cell() {
        let config = ShellBarsConfig {
            top: config_bar(true, 1),
            ..ShellBarsConfig::default()
        };
        let track = ShellBars::from_config(&config).top;
        assert_eq!(track.cells, Some(1));
        assert!(!track.has_border());
    }

    // T39 · one inset, so drawing and hit testing can never disagree.
    #[test]
    fn the_inner_area_is_the_same_answer_for_drawing_and_for_hits() {
        let outer = Rect::new(4, 2, 5, 10);
        assert_eq!(BarTrack::of(5).inner(outer), outer, "no border, no inset");
        assert_eq!(
            BarTrack::bordered(5).inner(outer),
            Rect::new(5, 3, 3, 8),
            "a border takes one cell on every side"
        );
        // Too small to hold anything: an empty rect, never a wrapped one.
        assert_eq!(
            BarTrack::bordered(2).inner(Rect::new(4, 2, 2, 2)),
            Rect::new(4, 2, 0, 0)
        );
    }

    // T37/T38 · a palette token follows the theme, a literal is taken at its
    // word, and something unreadable falls back instead of panicking.
    #[test]
    fn a_bar_colour_reads_palette_tokens_then_literals() {
        let palette = Palette::tokyo_night();
        assert_eq!(bar_color("", &palette), palette.peach, "the warm default");
        assert_eq!(bar_color("accent", &palette), palette.accent);
        assert_eq!(bar_color("  Mauve ", &palette), palette.mauve);
        assert_eq!(bar_color("orange", &palette), palette.peach);
        assert_eq!(bar_color("#fab387", &palette), parse_color("#fab387"));
        // Unreadable input must still answer with a colour.
        let _ = bar_color("not-a-colour-at-all", &palette);
    }

    fn gradient_config(stops: &[&str]) -> ShellBarConfig {
        ShellBarConfig {
            enabled: true,
            size: 3,
            border: true,
            color: String::new(),
            gradient: stops.iter().map(|s| (*s).to_string()).collect(),
            sections: Vec::new(),
        }
    }

    // T41 · a fade that ends where it began is not a fade.
    #[test]
    fn a_two_stop_gradient_changes_colour_across_the_span() {
        let palette = Palette::tokyo_night();
        let config = ShellBarsConfig {
            top: gradient_config(&["#000000", "#ffffff"]),
            ..ShellBarsConfig::default()
        };
        let tint = BarColors::from_config(&config, &palette).top;

        assert!(tint.fades());
        assert_eq!(
            tint.at(0, 11),
            Color::Rgb(0, 0, 0),
            "the first cell is the first stop"
        );
        assert_eq!(
            tint.at(10, 11),
            Color::Rgb(255, 255, 255),
            "and the last cell is the last stop"
        );
        let middle = tint.at(5, 11);
        assert_ne!(middle, tint.at(0, 11));
        assert_ne!(middle, tint.at(10, 11));
    }

    // T42 · a one-cell span has nowhere to fade, and must not divide by zero.
    #[test]
    fn a_gradient_over_a_single_cell_answers_with_its_first_stop() {
        let palette = Palette::tokyo_night();
        let config = ShellBarsConfig {
            top: gradient_config(&["#102030", "#405060"]),
            ..ShellBarsConfig::default()
        };
        let tint = BarColors::from_config(&config, &palette).top;
        assert_eq!(tint.at(0, 1), Color::Rgb(16, 32, 48));
        assert_eq!(
            tint.at(9, 1),
            Color::Rgb(16, 32, 48),
            "clamped, not wrapped"
        );
    }

    // T43 · a named terminal colour has no channels to walk between.
    #[test]
    fn a_gradient_that_names_a_channelless_colour_falls_back_to_solid() {
        let palette = Palette::tokyo_night();
        let config = ShellBarsConfig {
            // `magenta` is a terminal slot, not a value: what it looks like is
            // the terminal's business. Inventing channels for it would paint a
            // fade nobody described. (`yellow` and friends are palette tokens
            // here and DO carry channels — the distinction is the point.)
            top: ShellBarConfig {
                color: "accent".to_string(),
                ..gradient_config(&["magenta", "#ffffff"])
            },
            ..ShellBarsConfig::default()
        };
        let tint = BarColors::from_config(&config, &palette).top;
        assert!(!tint.fades());
        assert_eq!(tint.at(0, 10), palette.accent, "the solid tone stands in");
        assert_eq!(tint.at(9, 10), palette.accent);
    }

    // T44 · one stop is a colour, not a gradient.
    #[test]
    fn a_single_stop_gradient_is_read_as_a_solid_tone() {
        let palette = Palette::tokyo_night();
        let config = ShellBarsConfig {
            top: gradient_config(&["mauve"]),
            ..ShellBarsConfig::default()
        };
        let tint = BarColors::from_config(&config, &palette).top;
        assert!(!tint.fades());
        assert_eq!(tint.at(0, 10), palette.mauve);
    }

    // T20b · a disabled edge is inert no matter what size it names.
    #[test]
    fn a_disabled_edge_never_reaches_the_tree() {
        let config = ShellBarsConfig {
            top: config_bar(false, 4),
            bottom: config_bar(false, 4),
            left: config_bar(false, 20),
            right: config_bar(false, 20),
        };
        assert_eq!(ShellBars::from_config(&config), ShellBars::NONE);
        assert_eq!(
            derive_desktop_shell_layout(None, ShellBars::from_config(&config)).layout,
            ShellLayout::default()
        );
    }

    // A template names every region it wants; composing bars onto it would
    // duplicate one and fail the whole tree instead of the one bar.
    #[test]
    fn a_requested_template_owns_the_whole_tree_and_bars_do_not_double_it() {
        let bars = ShellBars {
            left: bar(6),
            ..ShellBars::NONE
        };
        for template in ALL_TEMPLATES {
            let with_bars = derive_desktop_shell_layout(Some(template), bars);
            let without = derive_desktop_shell_layout(Some(template), ShellBars::NONE);
            assert_eq!(with_bars, without, "{template:?} was altered by a bar");
        }
    }

    // A composition that will not validate costs the person their bars, not
    // their editor. Reached through the seam because every real combination
    // composes today.
    #[test]
    fn a_bar_composition_that_does_not_validate_falls_back_to_the_legacy_tree() {
        // A stage-less tree cannot be produced by the builder, so it is handed
        // to the seam directly — a guard nobody can exercise is a guard nobody
        // can trust.
        let derived = finish_bars(
            ShellBars {
                top: bar(1),
                ..ShellBars::NONE
            },
            ShellLayout::from_parts(
                ShellNode::Slot {
                    region: RegionId::LeftPanel,
                },
                Default::default(),
                Vec::new(),
                Vec::new(),
            ),
        );
        assert_eq!(derived.layout, ShellLayout::default());
        assert_eq!(derived.revision, LEGACY_DESKTOP_REVISION);
        assert_eq!(derived.template, None);
    }

    // ---- F32-L6 · dividing a bar into bounded sections ----

    fn section_config(kind: &str) -> ShellBarSectionConfig {
        ShellBarSectionConfig {
            kind: kind.to_string(),
            ..Default::default()
        }
    }

    fn fixed_section(cells: u16) -> ShellBarSectionConfig {
        ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells,
            ..Default::default()
        }
    }

    fn sections_of(policies: &[TrackPolicy]) -> BarSections {
        BarSections::from_policies(policies, "test")
    }

    // T22 · a ninth section is somebody's mistake, and mistakes are refused
    #[test]
    fn a_bar_refuses_a_ninth_section_rather_than_dropping_it() {
        // Truncating would hand the person a layout they did not write, and
        // renumber every section after the one that vanished — and the number
        // is the only name a section has.
        let too_many = vec![TrackPolicy::Fill { weight: 1 }; MAX_BAR_SECTIONS + 1];
        let sections = sections_of(&too_many);
        assert!(
            sections.is_empty(),
            "a bar with more sections than it may hold must be drawn undivided"
        );
    }

    // T22b · the ceiling has to be reachable, or it is an off-by-one
    #[test]
    fn a_bar_accepts_exactly_its_maximum_number_of_sections() {
        let full = vec![TrackPolicy::Fill { weight: 1 }; MAX_BAR_SECTIONS];
        assert_eq!(sections_of(&full).len(), MAX_BAR_SECTIONS);
    }

    // T23 · the sections are divided by the shell's own three phases
    #[test]
    fn section_widths_come_from_the_shell_solvers_own_arithmetic() {
        // Fixed takes its cells first; what is left is shared among the filling
        // sections in proportion to their weights. Hand-computed so that a
        // change in the allocator has to be argued for rather than absorbed.
        let track = BarTrack::of(3).with_sections(sections_of(&[
            TrackPolicy::Fixed { cells: 12 },
            TrackPolicy::Fill { weight: 1 },
            TrackPolicy::Fill { weight: 3 },
        ]));
        let rects = track.section_rects(RegionId::TopBar, Rect::new(0, 0, 40, 3));

        assert_eq!(rects.len(), 3);
        assert_eq!(rects.get(0).map(|r| r.width), Some(12));
        assert_eq!(rects.get(1).map(|r| r.width), Some(7));
        assert_eq!(rects.get(2).map(|r| r.width), Some(21));
    }

    // T23b · an odd remainder is broken the same way every time
    #[test]
    fn an_odd_remainder_goes_to_the_earlier_section_every_time() {
        // Two equal claims on seven cells cannot both be satisfied. The shell
        // solver breaks that tie by index, and a section allocator that broke
        // it any other way would drift from the regions by one cell without
        // anything going red.
        let track = BarTrack::of(1).with_sections(sections_of(&[
            TrackPolicy::Fill { weight: 1 },
            TrackPolicy::Fill { weight: 1 },
        ]));
        let rects = track.section_rects(RegionId::BottomBar, Rect::new(0, 0, 7, 1));

        assert_eq!(rects.get(0).map(|r| r.width), Some(4));
        assert_eq!(rects.get(1).map(|r| r.width), Some(3));
    }

    // T25 · a section that got no cells is not a target
    #[test]
    fn a_section_squeezed_to_nothing_is_not_reported_as_occupied() {
        // A zero-width rectangle that still answered clicks would answer for
        // its neighbour, which is the quiet failure CL5 exists to prevent.
        let track = BarTrack::of(1).with_sections(sections_of(&[
            TrackPolicy::Fixed { cells: 20 },
            TrackPolicy::Fixed { cells: 20 },
            TrackPolicy::Fill { weight: 1 },
        ]));
        let rects = track.section_rects(RegionId::TopBar, Rect::new(0, 0, 40, 1));

        assert_eq!(rects.len(), 3, "the third section still exists");
        assert_eq!(rects.get(2).map(|r| r.width), Some(0));
        assert_eq!(
            rects.occupied().count(),
            2,
            "a section with no cells must not appear as a target"
        );
    }

    // T26 · an undivided bar is exactly the bar everyone has today
    #[test]
    fn a_bar_without_sections_divides_into_nothing() {
        let rects = BarTrack::of(3).section_rects(RegionId::TopBar, Rect::new(0, 0, 40, 3));
        assert!(rects.is_empty());
        assert_eq!(rects.occupied().count(), 0);
    }

    // T27 · the border is not a section's to write on
    #[test]
    fn sections_of_a_bordered_bar_stay_inside_its_border() {
        let track = BarTrack::bordered(3).with_sections(sections_of(&[
            TrackPolicy::Fill { weight: 1 },
            TrackPolicy::Fill { weight: 1 },
        ]));
        let outer = Rect::new(0, 0, 40, 3);
        let inner = track.inner(outer);
        let rects = track.section_rects(RegionId::TopBar, outer);

        for (index, rect) in rects.occupied() {
            assert!(
                rect.y >= inner.y
                    && rect.x >= inner.x
                    && rect.right() <= inner.right()
                    && rect.bottom() <= inner.bottom(),
                "section {index} at {rect:?} escaped the bar's inner area {inner:?}"
            );
        }
        assert_eq!(rects.occupied().count(), 2);
    }

    // T29 · a side bar is divided down its height, not across its width
    #[test]
    fn a_side_bar_divides_its_height_and_an_edge_bar_divides_its_width() {
        // Getting the axis backwards collapses every section onto the same
        // cells and still looks plausible from a distance.
        let policies = sections_of(&[
            TrackPolicy::Fill { weight: 1 },
            TrackPolicy::Fill { weight: 1 },
        ]);

        let side = BarTrack::of(12).with_sections(policies);
        let side_rects = side.section_rects(RegionId::AppDock, Rect::new(0, 0, 12, 20));
        assert_eq!(side_rects.get(0), Some(Rect::new(0, 0, 12, 10)));
        assert_eq!(side_rects.get(1), Some(Rect::new(0, 10, 12, 10)));

        let edge = BarTrack::of(1).with_sections(policies);
        let edge_rects = edge.section_rects(RegionId::TopBar, Rect::new(0, 0, 20, 1));
        assert_eq!(edge_rects.get(0), Some(Rect::new(0, 0, 10, 1)));
        assert_eq!(edge_rects.get(1), Some(Rect::new(10, 0, 10, 1)));
    }

    // T30 · the sections account for the whole bar, and for no more than it
    #[test]
    fn sections_fill_the_bar_exactly_without_overrunning_it() {
        // A short division leaves an undrawn strip at the end of the bar; a
        // long one writes into the region next door.
        for available in [1u16, 2, 7, 13, 40] {
            let track = BarTrack::of(1).with_sections(sections_of(&[
                TrackPolicy::Fill { weight: 2 },
                TrackPolicy::Fill { weight: 1 },
                TrackPolicy::Fill { weight: 1 },
            ]));
            let rects = track.section_rects(RegionId::TopBar, Rect::new(0, 0, available, 1));
            let total: u16 = (0..rects.len())
                .filter_map(|index| rects.get(index))
                .map(|rect| rect.width)
                .sum();
            assert_eq!(
                total, available,
                "three filling sections must account for all {available} cells"
            );
        }
    }

    // T22c · one unreadable section costs the division, not the bar
    #[test]
    fn an_unreadable_section_leaves_the_bar_undivided_rather_than_renumbered() {
        // Dropping only the bad entry would shift every later section up one
        // index, silently moving whatever those indices address.
        let configs = vec![
            fixed_section(4),
            section_config("nonsense"),
            fixed_section(6),
        ];
        assert!(sections_from_config(&configs, "top").is_empty());

        let good = vec![fixed_section(4), fixed_section(6)];
        assert_eq!(sections_from_config(&good, "top").len(), 2);
    }

    // T22d · a section whose numbers cannot describe a size is refused
    #[test]
    fn a_section_with_impossible_numbers_is_refused() {
        assert_eq!(
            section_policy(&fixed_section(0), "top", 0),
            Err(BarConfigProblem::FixedSectionWithoutCells {
                edge: "top",
                index: 0
            })
        );
        assert_eq!(
            section_policy(
                &ShellBarSectionConfig {
                    kind: "content".to_string(),
                    min: 10,
                    max: 4,
                    ..Default::default()
                },
                "top",
                0
            ),
            Err(BarConfigProblem::ContentSectionMaxBelowMin {
                edge: "top",
                index: 0,
                min: 10,
                max: 4
            })
        );
        // A fill with no weight written is the commonest section there is, and
        // asking for it must not require typing a number.
        assert_eq!(
            section_policy(&section_config("fill"), "top", 0),
            Ok(TrackPolicy::Fill { weight: 1 })
        );
    }

    // ---- #54 · a refused setting is reported, not just a misspelled one ----

    fn bars_with_top(bar: ShellBarConfig) -> ShellBarsConfig {
        ShellBarsConfig {
            top: bar,
            ..Default::default()
        }
    }

    fn enabled_bar(size: u16, border: bool) -> ShellBarConfig {
        ShellBarConfig {
            enabled: true,
            size,
            border,
            color: String::new(),
            gradient: Vec::new(),
            sections: Vec::new(),
        }
    }

    // T-CFG-1 · a size nothing can draw is said out loud, with its edge named
    #[test]
    fn an_out_of_range_bar_size_is_reported_with_its_edge() {
        // Before this, the value parsed perfectly, the checker said "ok", and
        // the bar simply never appeared — sending the person to look at their
        // terminal rather than at the line they had just written.
        let problems = shell_bar_config_problems(&bars_with_top(enabled_bar(999, false)));
        assert_eq!(problems.len(), 1);
        let text = problems[0].to_string();
        assert!(text.contains("shell.bars.top.size"), "{text}");
        assert!(text.contains("999"), "{text}");
    }

    // T-CFG-2 · two different mistakes must not read as the same mistake
    #[test]
    fn a_bordered_bar_too_thin_reads_differently_from_one_out_of_range() {
        let thin = shell_bar_config_problems(&bars_with_top(enabled_bar(1, true)));
        assert_eq!(thin.len(), 1);
        assert_eq!(
            thin[0],
            BarConfigProblem::BorderedBarTooThin {
                edge: "top",
                size: 1,
                minimum: MIN_BORDERED_BAR_CELLS
            }
        );

        // The same thickness without a border is perfectly drawable, so it
        // must produce nothing at all: a person told to fix a working setting
        // learns to stop reading.
        assert!(shell_bar_config_problems(&bars_with_top(enabled_bar(1, false))).is_empty());
    }

    // T-CFG-3 · a section's complaint names which section it is
    #[test]
    fn an_unknown_section_kind_is_reported_with_its_index() {
        let mut bar = enabled_bar(3, true);
        bar.sections = vec![fixed_section(4), section_config("nonsense")];
        let problems = shell_bar_config_problems(&bars_with_top(bar));

        assert_eq!(problems.len(), 1);
        let text = problems[0].to_string();
        // A bar may hold eight sections; "a section is wrong" sends somebody
        // through all eight of them.
        assert!(text.contains("sections[1]"), "{text}");
        assert!(text.contains("nonsense"), "{text}");
    }

    // T-CFG-4/5 · the remaining refusals are reported too
    #[test]
    fn an_impossible_content_section_and_too_many_sections_are_reported() {
        let mut narrow = enabled_bar(3, true);
        narrow.sections = vec![ShellBarSectionConfig {
            kind: "content".to_string(),
            min: 10,
            max: 4,
            ..Default::default()
        }];
        assert_eq!(shell_bar_config_problems(&bars_with_top(narrow)).len(), 1);

        let mut crowded = enabled_bar(3, true);
        crowded.sections = (0..=MAX_BAR_SECTIONS)
            .map(|_| section_config("fill"))
            .collect();
        let problems = shell_bar_config_problems(&bars_with_top(crowded));
        assert_eq!(problems.len(), 1);
        assert!(
            problems[0].to_string().contains("at most"),
            "{}",
            problems[0]
        );
    }

    // T-CFG-6 · a configuration this build can draw says nothing
    #[test]
    fn a_drawable_configuration_produces_no_complaints() {
        let mut bar = enabled_bar(3, true);
        bar.sections = vec![fixed_section(12), section_config("fill")];
        assert!(shell_bar_config_problems(&bars_with_top(bar)).is_empty());

        // A disabled bar is not drawn either, but that is what was asked for.
        // Reporting it would bury the real complaints under noise.
        let mut disabled = enabled_bar(999, false);
        disabled.enabled = false;
        assert!(shell_bar_config_problems(&bars_with_top(disabled)).is_empty());

        // And a bar nobody configured at all.
        assert!(shell_bar_config_problems(&ShellBarsConfig::default()).is_empty());
    }

    // T-CFG-7 · what is reported and what is drawn come from one predicate
    #[test]
    fn every_reported_problem_is_a_bar_that_is_actually_refused() {
        // This is the guard, not the messages. If the checker ever grew its own
        // copy of these range rules, the two would agree on the day it was
        // written and drift on the first change to either — and nothing would
        // go red, because each side stays internally consistent. So the test
        // asserts the equivalence itself, across every case in both directions.
        let cases: Vec<(ShellBarConfig, bool)> = vec![
            (enabled_bar(3, true), true),
            (enabled_bar(1, false), true),
            (enabled_bar(32, false), true),
            (enabled_bar(0, false), false),
            (enabled_bar(33, false), false),
            (enabled_bar(999, true), false),
            (enabled_bar(2, true), false),
            (enabled_bar(1, true), false),
        ];

        for (bar, expected_drawn) in cases {
            let size = bar.size;
            let border = bar.border;
            let reported = shell_bar_config_problems(&bars_with_top(ShellBarConfig {
                enabled: true,
                size,
                border,
                color: String::new(),
                gradient: Vec::new(),
                sections: Vec::new(),
            }))
            .is_empty();
            let drawn = BarTrack::from_config(&bar, "top").enabled();

            assert_eq!(
                drawn, expected_drawn,
                "size={size} border={border}: drawing disagrees with the case table"
            );
            assert_eq!(
                reported, drawn,
                "size={size} border={border}: the checker says {reported} but the bar is \
                 drawn={drawn} — the two read different rules"
            );
        }
    }

    // T28a · a bar that is divided differently is a different bar
    #[test]
    fn changing_only_a_sections_policy_changes_the_bars_identity() {
        // `BarTrack` travels inside the geometry cache key. If two different
        // divisions compared equal, the cache would hand back the previous
        // geometry and every click in that bar would land in the wrong section.
        let one = BarTrack::of(3).with_sections(sections_of(&[TrackPolicy::Fixed { cells: 4 }]));
        let other = BarTrack::of(3).with_sections(sections_of(&[TrackPolicy::Fixed { cells: 5 }]));
        assert_ne!(one, other);

        let bars_one = ShellBars {
            top: one,
            ..ShellBars::NONE
        };
        let bars_other = ShellBars {
            top: other,
            ..ShellBars::NONE
        };
        assert_ne!(bars_one, bars_other);
    }
}
