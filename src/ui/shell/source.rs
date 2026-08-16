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
use crate::popup_size::PopupSize;

use super::layout::allocate_section_lengths;
use super::model::{
    RegionId, RegionSize, ShellChild, ShellDirection, ShellLayout, ShellNode, ShellValidationError,
    TrackPolicy, ValidatedShellLayout,
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

/// The most parts one bar may ever be divided into, whatever a config asks for.
///
/// This used to be `MAX_SPLIT_CHILDREN`, on the reasoning that a bar's division
/// and a pane split are "the same question with the same answer". They are not,
/// and the assumption cost the toolbar its eleventh icon: a screen cut into
/// twelve panes is unusable, while a strip carrying twelve icons is ordinary.
/// One number was answering two questions.
///
/// Still a bound rather than a `Vec`, and still finite. CLA7's objection is to
/// an *unbounded* visible chain, not to a larger finite one — and the array is
/// what keeps `BarSections` `Copy` and comparable by value, which is what makes
/// CL3's rule ("the key contains every input that decided the geometry") true by
/// construction. Measured cost of the raise: `BarSections` and `BarSectionRects`
/// go from 66 to 130 bytes each, so all four bars together grow by 512 bytes.
///
/// The number a person meets is `shell.bars.<edge>.max_sections`, which defaults
/// to 8 and may not exceed this.
pub(crate) const MAX_BAR_SECTIONS: usize = 16;

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
        track.with_sections(sections_from_config(config, edge))
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
    PopupSizeWithoutPopup {
        edge: &'static str,
        index: usize,
    },
    UnknownSecondaryPresentation {
        edge: &'static str,
        index: usize,
        presentation: String,
    },
    SecondaryWithoutAction {
        edge: &'static str,
        index: usize,
    },
    PluginActionWithoutId {
        edge: &'static str,
        index: usize,
    },
    PluginActionWithPopupField {
        edge: &'static str,
        index: usize,
        /// Which popup-only field was left behind, so the complaint sends
        /// somebody to the line that is actually wrong.
        field: &'static str,
    },
    PluginActionWithSecondary {
        edge: &'static str,
        index: usize,
    },
    PopupActionWithPluginCommand {
        edge: &'static str,
        index: usize,
    },
    PluginCommandWithoutAction {
        edge: &'static str,
        index: usize,
    },
    SectionBudgetOutOfRange {
        edge: &'static str,
        requested: usize,
        max: usize,
    },
    UnknownSectionWidgetKind {
        edge: &'static str,
        index: usize,
        kind: String,
    },
    UnknownSectionWidgetMetric {
        edge: &'static str,
        index: usize,
        metric: String,
    },
    IconWithoutPicture {
        edge: &'static str,
        index: usize,
    },
    IconWithTwoPictures {
        edge: &'static str,
        index: usize,
    },
    /// `shell.glyph_icons` is off and this section's glyph has nothing to fall
    /// back to.
    ///
    /// Drawing nothing would be worse than saying so: an empty section is what
    /// a section with no widget looks like, so a person reading the bar could
    /// not tell a disabled glyph from one they never wrote. The switch is the
    /// only setting that can invalidate a section that was valid before, and
    /// that is deliberate — it is a visible cost, and the alternative is a
    /// silent gap.
    IconGlyphOffWithoutText {
        edge: &'static str,
        index: usize,
    },
    UnknownIconArt {
        edge: &'static str,
        index: usize,
        name: String,
    },
    UnreadableIconArt {
        edge: &'static str,
        index: usize,
        problem: crate::icon::IconProblem,
    },
    IconDoesNotFit {
        edge: &'static str,
        index: usize,
        needs: u16,
        has: u16,
    },
    WidgetTextWithoutWidget {
        edge: &'static str,
        index: usize,
    },
}

/// The accepted names of a closed set, phrased the way the refusals below already
/// phrase them: quoted, separated by commas, and joined by `or` before the last.
///
/// Six refusals carry such a list and each one is written out by hand beside the
/// match that decides. Writing the phrase once is the first half of holding the two
/// together; the second half is that the list itself comes from the same place the
/// match does, which is what the tables are for.
///
/// The wording is not a matter of taste here. It is what the guide quotes, what the
/// tests read and what people have learned to expect, so this reproduces today's
/// sentence exactly rather than improving on it.
fn accepted_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [only] => format!("{only:?}"),
        [rest @ .., last] => {
            let listed = rest
                .iter()
                .map(|name| format!("{name:?}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{listed} or {last:?}")
        }
    }
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
                "shell.bars.{edge}.sections[{index}].kind is \"{kind}\"; expected {offered}, \
                 so this bar is drawn undivided",
                offered = accepted_names(&SectionKind::offered(SIZING_KINDS))
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
            // The accepted list is the discoverable half of a closed enum. Left
            // out, somebody reads "expected popup" and concludes this build has
            // no plugin support at all — a wrong conclusion drawn from a
            // correct message.
            Self::UnknownSectionActionKind { edge, index, kind } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action.kind is \"{kind}\"; expected \
                 {offered}, so this section does nothing when clicked",
                offered = accepted_names(&SectionKind::offered(ACTION_KINDS))
            ),
            Self::PopupActionWithoutCommand { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action opens a popup but names no command \
                 to run, so this section does nothing when clicked"
            ),
            Self::PopupSizeWithoutPopup { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action sets a popup size but opens no \
                 popup, so the size is never used"
            ),
            Self::UnknownSecondaryPresentation {
                edge,
                index,
                presentation,
            } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action.secondary is \"{presentation}\"; \
                 expected {offered}, so a right press on this section does nothing",
                offered = accepted_names(
                    &SECONDARY_PRESENTATIONS
                        .iter()
                        .map(|(name, _)| *name)
                        .collect::<Vec<_>>()
                )
            ),
            Self::SecondaryWithoutAction { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action names a secondary presentation but \
                 no command to present, so a right press on this section does nothing"
            ),
            Self::PluginActionWithoutId { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action invokes a plugin but names no \
                 action.command to invoke, so this section does nothing when clicked"
            ),
            Self::PluginActionWithPopupField { edge, index, field } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action.{field} belongs to a popup, but \
                 this action invokes a plugin and a plugin's own manifest decides what it \
                 runs and where, so the setting is never used"
            ),
            Self::PluginActionWithSecondary { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action.secondary asks to re-present a \
                 plugin action, but a plugin's own manifest decides where it opens, so a \
                 right press on this section cannot honour it"
            ),
            Self::PopupActionWithPluginCommand { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action.command names a plugin action but \
                 this action opens a popup, so the plugin action is never invoked"
            ),
            Self::PluginCommandWithoutAction { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].action.command names a plugin action but \
                 the section opens nothing, so the plugin action is never invoked"
            ),
            Self::SectionBudgetOutOfRange {
                edge,
                requested,
                max,
            } => write!(
                formatter,
                "shell.bars.{edge}.max_sections is {requested}; this build allows 1 to {max}, \
                 so this bar is drawn undivided"
            ),
            Self::UnknownSectionWidgetKind { edge, index, kind } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget.kind is \"{kind}\"; expected \
                 {offered}, so this section shows nothing",
                offered = accepted_names(&SectionKind::offered(WIDGET_KINDS))
            ),
            Self::UnknownSectionWidgetMetric {
                edge,
                index,
                metric,
            } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget.metric is \"{metric}\"; expected \
                 {offered}, so this section shows nothing",
                offered = accepted_names(crate::resource::ResourceMetric::ACCEPTED)
            ),
            Self::IconWithoutPicture { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget is an icon but names no picture; \
                 set one of glyph, art or pixels"
            ),
            Self::IconGlyphOffWithoutText { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget draws a glyph, but \
                 shell.glyph_icons is off and the section names no text to draw instead; \
                 give it a text, or turn the switch back on"
            ),
            Self::IconWithTwoPictures { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget names more than one picture; \
                 glyph, art and pixels are alternatives, and which one wins would be invisible"
            ),
            Self::UnknownIconArt { edge, index, name } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget.art is \"{name}\", which this build \
                 does not bundle; expected {offered}, so this section shows nothing",
                offered = accepted_names(&crate::icon::builtin_names())
            ),
            Self::UnreadableIconArt {
                edge,
                index,
                problem,
            } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget {problem}"
            ),
            Self::IconDoesNotFit {
                edge,
                index,
                needs,
                has,
            } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget needs {needs} cells but the section \
                 declares {has}; a clipped picture is the wrong picture"
            ),
            Self::WidgetTextWithoutWidget { edge, index } => write!(
                formatter,
                "shell.bars.{edge}.sections[{index}].widget sets text but names no widget to \
                 show it, so the text never appears"
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

/// The budget this bar was given, or what is wrong with the number it asked for.
///
/// Refused rather than clamped: a file saying 40 next to a build doing 16 is a
/// file the next reader will believe. And zero is refused because a bar allowed
/// no parts is a bar that cannot divide, which `enabled = false` already says —
/// two spellings for one state is one too many.
fn section_budget(config: &ShellBarConfig, edge: &'static str) -> Result<usize, BarConfigProblem> {
    let requested = usize::from(config.max_sections);
    if requested == 0 || requested > MAX_BAR_SECTIONS {
        return Err(BarConfigProblem::SectionBudgetOutOfRange {
            edge,
            requested,
            max: MAX_BAR_SECTIONS,
        });
    }
    Ok(requested)
}

fn section_count_problem(
    sections: usize,
    edge: &'static str,
    budget: usize,
) -> Option<BarConfigProblem> {
    (sections > budget).then_some(BarConfigProblem::TooManySections {
        edge,
        sections,
        max: budget,
    })
}

// TP-CHROME-35/36: the checker and the deriver read one predicate, so a
// setting that will not be drawn is also a setting that gets said out loud.
/// Everything under `[shell.bars]` that this build will refuse to draw.
///
/// Only enabled edges are examined: a disabled bar is not drawn either, but
/// that is what the person asked for, and reporting it would bury the real
/// complaints under noise.
pub(crate) fn shell_bar_config_problems(
    config: &ShellBarsConfig,
    glyph_icons: bool,
) -> Vec<BarConfigProblem> {
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
        // The budget is asked first: a bar whose ceiling this build cannot
        // honour is undivided for that reason, and reporting the section count
        // against a number that was itself refused would name the wrong line.
        let budget = match section_budget(bar, edge) {
            Ok(budget) => budget,
            Err(problem) => {
                problems.push(problem);
                continue;
            }
        };
        if let Some(problem) = section_count_problem(bar.sections.len(), edge, budget) {
            problems.push(problem);
            continue;
        }
        for (index, section) in bar.sections.iter().enumerate() {
            if let Err(problem) = section_policy(section, edge, index) {
                problems.push(problem);
            }
            // Asked separately from the sizing rule because the two have
            // different blast radii and a person fixing one should not have to
            // guess that the other was also refused. The widget is asked before
            // the action so a section's complaints read in the order the table
            // is written.
            if let Err(problem) = section_widget(section, edge, index, glyph_icons) {
                problems.push(problem);
            }
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
fn sections_from_config(config: &ShellBarConfig, edge: &'static str) -> BarSections {
    if config.sections.is_empty() {
        return BarSections::NONE;
    }
    // The budget is read before the sections are, because a budget this build
    // cannot honour is a reason the division as a whole does not stand — the
    // same all-or-nothing rule the section policies already follow.
    let budget = match section_budget(config, edge) {
        Ok(budget) => budget,
        Err(problem) => {
            tracing::warn!(%problem, "the bar is drawn undivided");
            return BarSections::NONE;
        }
    };
    match section_policies(&config.sections, edge, budget) {
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
    budget: usize,
) -> Result<Vec<TrackPolicy>, BarConfigProblem> {
    if let Some(problem) = section_count_problem(configs.len(), edge, budget) {
        return Err(problem);
    }
    configs
        .iter()
        .enumerate()
        .map(|(index, config)| section_policy(config, edge, index))
        .collect()
}

/// One accepted name and the thing it builds.
///
/// `build` is a function pointer rather than data, and that is the whole point:
/// adding a name to a table forces somebody to write the code that builds it, and
/// deleting the code breaks the table at compile time. The state this file kept
/// drifting into — a list that offers a name nothing builds, or a match arm no list
/// mentions — stops being expressible.
struct SectionKind<T> {
    name: &'static str,
    /// The keys this kind reads.
    ///
    /// Written here rather than beside the builder so that adding a key nobody
    /// lists is a diff somebody has to look at.
    keys: &'static [&'static str],
    /// The keys this kind refuses to be handed.
    ///
    /// A refusal is as much a part of the grammar as an acceptance, and it is
    /// the half a reader has no way to discover except by being turned down.
    refuses: &'static [&'static str],
    /// A whole config this build accepts, showing the kind in use.
    ///
    /// Standalone on purpose: it is published verbatim by `herdr shell spec`, so
    /// the gate that checks it can parse exactly the bytes a reader receives
    /// rather than a reconstruction assembled in a test.
    example: &'static str,
    build: fn(SectionAt<'_>) -> Result<T, BarConfigProblem>,
}

/// A section, where it sits, and the switches that reach it.
///
/// The three surfaces do not all want the same inputs — sizing never asks about
/// glyphs and widgets always do — and giving every builder the union of them as
/// loose arguments would put a parameter in front of `fixed` that means nothing
/// there. One shape carries them instead, so the tables stay one type and the
/// functions that surround them keep the signatures their callers already use.
#[derive(Clone, Copy)]
struct SectionAt<'a> {
    config: &'a ShellBarSectionConfig,
    edge: &'static str,
    index: usize,
    glyph_icons: bool,
}

impl<'a> SectionAt<'a> {
    /// For the surfaces that never ask about glyphs.
    ///
    /// Sizing and actions do not read the switch, and spelling that out once here is
    /// better than each of them choosing a value and the next reader wondering which
    /// choice mattered.
    fn plain(config: &'a ShellBarSectionConfig, edge: &'static str, index: usize) -> Self {
        Self {
            config,
            edge,
            index,
            glyph_icons: true,
        }
    }
}

impl<T> SectionKind<T> {
    /// The names, in the order the refusal offers them.
    ///
    /// Order is the table's, not the dispatch's: lookup is by name, so the sequence
    /// exists only to be read aloud in a message somebody has already learned.
    fn offered(table: &'static [Self]) -> Vec<&'static str> {
        table.iter().map(|entry| entry.name).collect()
    }

    fn find(table: &'static [Self], name: &str) -> Option<&'static Self> {
        table.iter().find(|entry| entry.name == name)
    }

    /// The table with its builders left behind.
    fn facts(table: &'static [Self]) -> Vec<KindFacts> {
        table
            .iter()
            .map(|entry| KindFacts {
                name: entry.name,
                keys: entry.keys,
                refuses: entry.refuses,
                example: entry.example,
            })
            .collect()
    }
}

/// One accepted name, described rather than built.
///
/// The tables themselves cannot cross a module boundary: each is typed by what
/// its builders make, and no two of them make the same thing. What a reader
/// needs is the half that is the same everywhere, so that is what leaves.
pub(crate) struct KindFacts {
    pub name: &'static str,
    pub keys: &'static [&'static str],
    pub refuses: &'static [&'static str],
    pub example: &'static str,
}

/// The ways a section can ask for space.
pub(crate) fn sizing_kind_facts() -> Vec<KindFacts> {
    SectionKind::facts(SIZING_KINDS)
}

/// The things a section can show.
pub(crate) fn widget_kind_facts() -> Vec<KindFacts> {
    SectionKind::facts(WIDGET_KINDS)
}

/// The things a press can do.
pub(crate) fn action_kind_facts() -> Vec<KindFacts> {
    SectionKind::facts(ACTION_KINDS)
}

/// The presentations a second gesture can ask for.
pub(crate) fn secondary_presentation_names() -> Vec<&'static str> {
    SECONDARY_PRESENTATIONS
        .iter()
        .map(|(name, _)| *name)
        .collect()
}

/// The three ways a section can ask for space.
const SIZING_KINDS: &[SectionKind<TrackPolicy>] = &[
    SectionKind {
        name: "fixed",
        keys: &["cells"],
        refuses: &[],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"fixed\"\ncells = 12\n",
        build: fixed_policy,
    },
    SectionKind {
        name: "fill",
        keys: &["weight"],
        refuses: &[],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"fill\"\nweight = 2\n",
        build: fill_policy,
    },
    SectionKind {
        name: "content",
        keys: &["min", "max"],
        refuses: &[],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"content\"\nmin = 4\nmax = 24\n",
        build: content_policy,
    },
];

fn fixed_policy(at: SectionAt<'_>) -> Result<TrackPolicy, BarConfigProblem> {
    if at.config.cells == 0 {
        return Err(BarConfigProblem::FixedSectionWithoutCells {
            edge: at.edge,
            index: at.index,
        });
    }
    Ok(TrackPolicy::Fixed {
        cells: at.config.cells,
    })
}

/// A fill with no weight is the common shape of "just take the rest", and refusing
/// it would make the simplest section the one that needs the most typing.
fn fill_policy(at: SectionAt<'_>) -> Result<TrackPolicy, BarConfigProblem> {
    Ok(TrackPolicy::Fill {
        weight: at.config.weight.max(1),
    })
}

fn content_policy(at: SectionAt<'_>) -> Result<TrackPolicy, BarConfigProblem> {
    if at.config.max < at.config.min {
        return Err(BarConfigProblem::ContentSectionMaxBelowMin {
            edge: at.edge,
            index: at.index,
            min: at.config.min,
            max: at.config.max,
        });
    }
    Ok(TrackPolicy::ContentBounded {
        min: at.config.min,
        max: at.config.max,
    })
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
    match SectionKind::find(SIZING_KINDS, config.kind.as_str()) {
        Some(entry) => (entry.build)(SectionAt::plain(config, edge, index)),
        None => Err(BarConfigProblem::UnknownSectionKind {
            edge,
            index,
            kind: config.kind.clone(),
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
        /// Outer popup size, or `None` for the popup's own default.
        ///
        /// Carried beside the command rather than resolved here: this layer
        /// reads config, and how many cells a percentage becomes depends on a
        /// terminal area this layer has never seen (CLA4 — no I/O, and no
        /// geometry, in a derivation).
        width: Option<PopupSize>,
        height: Option<PopupSize>,
        /// How a secondary press shows the same command, or `None` when the
        /// section answers only one gesture.
        ///
        /// Deliberately a presentation of the command above rather than a
        /// command of its own. Two commands in one section could drift into
        /// running different programs from the same picture, and the person
        /// pressing has no way to know which one they got. One command, two
        /// presentations, is also what makes the gesture rule true rather than
        /// decorative: the right press chooses how, never what.
        secondary: Option<SecondaryPresentation>,
    },
    /// Invoke an action an installed plugin declared, by the id its manifest
    /// gives it.
    ///
    /// Carries an id rather than a command line, and that is the whole security
    /// argument for this arm: what runs is chosen by a manifest herdr has
    /// already read and validated, not by a string in a bar's config. A section
    /// that could name an argv here would turn "place this app's icon on the
    /// bar" into "run this command line", which is a different thing to accept
    /// from a plugin registry.
    ///
    /// Nothing is checked about the id here. This layer reads config and has
    /// never seen the installed-plugin list (CLA4), and it should not: a plugin
    /// can be installed after the config naming it was written, and refusing
    /// the line now would forbid the icon of an app that has not been
    /// downloaded yet — the exact opposite of what putting one there is for.
    InvokePlugin {
        /// Verbatim, as written. The resolver on the other end matches this
        /// against every installed manifest and trims for itself; reshaping it
        /// here could land on a different plugin's action than the file names.
        action: String,
    },
}

/// How a secondary press presents a section's command.
///
/// One variant today, and closed like its neighbours. A "split the current
/// pane" presentation was considered and left out: it needs a target pane, and
/// a bar has no idea which pane the person meant. The enum being closed is what
/// makes adding one later a cost the compiler counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SecondaryPresentation {
    /// A new tab of the current workspace, running the command at full size.
    ///
    /// Full size needs no zoom: a tab's root pane already occupies the whole
    /// tab, so asking for one is asking for the other.
    Tab,
}

/// What one section of a bar shows.
///
/// Closed for the same reason its neighbour is, and deliberately smaller than
/// it looks: a divider and a "blank" kind were both considered and left out.
/// Blank is what a section already is with no widget at all, and a divider is a
/// variant nobody has asked for — the enum being closed is what makes adding
/// one later a cost the compiler counts rather than a guess made now.
///
/// A widget never decides how wide its section is. Letting text size a section
/// would put that text in the geometry key, and editing a label would then
/// re-lay-out the whole bar for a change that moves nothing. A label that does
/// not fit is clipped, by display width — the person asked for icons, and an
/// emoji is two cells wide, so clipping by character count would overrun the
/// rectangle the section was promised.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) enum SectionWidget {
    /// Draws nothing, which is what every section did before this existed.
    #[default]
    None,
    Label {
        text: String,
    },
    /// A number the machine keeps changing under the section.
    ///
    /// The widget holds only *which* number it wants. The number itself lives
    /// in state, sampled on the loop's own clock, because a widget that could
    /// read a counter would be read once per frame by the thing that draws it.
    Resource {
        metric: crate::resource::ResourceMetric,
    },
    /// A filled bar showing how full one metric is.
    ///
    /// Like `Resource` it names only the metric; the number arrives already
    /// sampled. Unlike it, the value is drawn rather than written, which is the
    /// difference between reading a bar and reading a figure.
    Meter {
        metric: crate::resource::ResourceMetric,
    },
    /// One grapheme the font already knows how to draw.
    Icon {
        glyph: String,
    },
    /// A picture in cells, colours still unresolved.
    ///
    /// The specs stay as written until draw time so a theme change recolours
    /// the picture without re-deriving any geometry — the same split every
    /// other bar colour already follows.
    Art {
        art: crate::icon::IconArt,
    },
}

/// What one section shows and what a click on it does.
///
/// The two are held together rather than in two parallel lists on purpose.
/// Both are addressed by the section's index and both have to disappear when a
/// division is refused; two structures would mean two alignment invariants and
/// two copies of the refusal rule, which is the divergence this file has
/// already been bitten by three times.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct SectionChrome {
    pub(crate) widget: SectionWidget,
    pub(crate) action: SectionAction,
}

/// One edge's sections' chrome, in the same order and addressed by the same
/// indices as that edge's sections.
///
/// Index-aligned with [`BarSections`] by construction: both are derived from
/// the same config list through the same refusal predicate, so a division that
/// was refused cannot leave behind a chrome list whose indices address the
/// sections of some other, imagined bar.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct BarSectionChrome {
    entries: Vec<SectionChrome>,
}

impl BarSectionChrome {
    pub(crate) const EMPTY: Self = Self {
        entries: Vec::new(),
    };

    fn get(&self, index: u8) -> Option<&SectionChrome> {
        self.entries.get(usize::from(index))
    }
}

/// What clicking each part of each edge does.
///
/// Held apart from [`ShellBars`] on purpose. `ShellBars` travels inside the
/// geometry cache key, and actions do not decide geometry: folding a command
/// line into the key would make editing that command invalidate every cached
/// rectangle on screen, for a change that moves nothing.
// TP-CHROME-47: the separation is what keeps a popup's size — and the command
// beside it — out of the value the geometry cache compares.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ShellBarChrome {
    top: BarSectionChrome,
    bottom: BarSectionChrome,
    left: BarSectionChrome,
    right: BarSectionChrome,
}

impl ShellBarChrome {
    pub(crate) fn from_config(config: &ShellBarsConfig, glyph_icons: bool) -> Self {
        Self {
            top: bar_section_chrome(&config.top, "top", glyph_icons),
            bottom: bar_section_chrome(&config.bottom, "bottom", glyph_icons),
            left: bar_section_chrome(&config.left, "left", glyph_icons),
            right: bar_section_chrome(&config.right, "right", glyph_icons),
        }
    }

    /// What the numbered section of the named region shows and does, if that
    /// region is an edge bar and it has such a section.
    ///
    /// Resolves the region through the same [`bar_edge_for`] the geometry side
    /// uses, so the bar a click is attributed to, the bar a label is drawn in,
    /// and the bar the rectangle came from can never be three different bars.
    pub(crate) fn for_section(&self, region: RegionId, index: u8) -> Option<&SectionChrome> {
        let edge = match bar_edge_for(region)? {
            BarEdge::Top => &self.top,
            BarEdge::Bottom => &self.bottom,
            BarEdge::Left => &self.left,
            BarEdge::Right => &self.right,
        };
        edge.get(index)
    }

    pub(crate) fn action_for(&self, region: RegionId, index: u8) -> Option<&SectionAction> {
        self.for_section(region, index).map(|chrome| &chrome.action)
    }

    pub(crate) fn widget_for(&self, region: RegionId, index: u8) -> Option<&SectionWidget> {
        self.for_section(region, index).map(|chrome| &chrome.widget)
    }

    /// Whether anything on screen is waiting on a machine counter.
    ///
    /// This is what keeps the feature free for the people not using it. No
    /// resource section means no deadline, which means the loop never wakes to
    /// sample and never opens `/proc` at all — rather than sampling always and
    /// throwing the answer away, which is the shape this kind of widget
    /// usually arrives in.
    // TP-RES-07: sampling is demand-driven; an unused feature costs nothing.
    pub(crate) fn wants_resources(&self) -> bool {
        [&self.top, &self.bottom, &self.left, &self.right]
            .into_iter()
            .flat_map(|bar| bar.entries.iter())
            .any(|chrome| {
                matches!(
                    chrome.widget,
                    SectionWidget::Resource { .. } | SectionWidget::Meter { .. }
                )
            })
    }
}

/// Read one edge's sections' chrome, aligned with the sections that edge
/// actually has.
///
/// A refused division yields no chrome at all: the indices a chrome list is
/// addressed by are the section indices, and there are none.
///
/// A single unreadable widget or action costs only itself. That asymmetry with
/// the sizing rules is deliberate — a misspelled command name should not take
/// the whole bar's layout down with it, and leaving the section in place with
/// nothing on it keeps every other index pointing where it pointed.
// TP-CHROME-37/39/40/45/46/52: chrome answers at the index it was written at, a
// refused division leaves none, a refused entry costs only its section, a popup
// carries the size it was written with, and a widget is read the same way.
fn bar_section_chrome(
    config: &ShellBarConfig,
    edge: &'static str,
    glyph_icons: bool,
) -> BarSectionChrome {
    if !config.enabled || config.sections.is_empty() {
        return BarSectionChrome::EMPTY;
    }
    if bar_size_problem(config, edge).is_some() {
        return BarSectionChrome::EMPTY;
    }
    let Ok(budget) = section_budget(config, edge) else {
        return BarSectionChrome::EMPTY;
    };
    if section_policies(&config.sections, edge, budget).is_err() {
        return BarSectionChrome::EMPTY;
    }
    let entries = config
        .sections
        .iter()
        .enumerate()
        .map(|(index, section)| SectionChrome {
            widget: match section_widget(section, edge, index, glyph_icons) {
                Ok(widget) => widget,
                Err(problem) => {
                    tracing::warn!(%problem, "the section is drawn with nothing in it");
                    SectionWidget::None
                }
            },
            action: match section_action(section, edge, index) {
                Ok(action) => action,
                Err(problem) => {
                    tracing::warn!(%problem, "the section is drawn without a click action");
                    SectionAction::None
                }
            },
        })
        .collect();
    BarSectionChrome { entries }
}

/// One section's widget table as a widget, or what is wrong with it.
// TP-CHROME-54: a widget this build cannot show, and text with nothing to show
// it, are both refused here and reported by the checker reading this same
// function.
fn section_widget(
    config: &ShellBarSectionConfig,
    edge: &'static str,
    index: usize,
    glyph_icons: bool,
) -> Result<SectionWidget, BarConfigProblem> {
    // Naming nothing is not one of the choices below; it is the absence of a
    // choice, and the only question left is whether anything was written for a
    // widget that is not there. Text with nothing to put it in is the same
    // half-finished shape a leftover popup size is: it will never appear, and a
    // person reading the file back would believe it does.
    if config.widget.kind.is_empty() {
        return if config.widget.text.is_empty() {
            Ok(SectionWidget::None)
        } else {
            Err(BarConfigProblem::WidgetTextWithoutWidget { edge, index })
        };
    }

    let at = SectionAt {
        config,
        edge,
        index,
        glyph_icons,
    };
    match SectionKind::find(WIDGET_KINDS, config.widget.kind.as_str()) {
        Some(entry) => (entry.build)(at),
        None => Err(BarConfigProblem::UnknownSectionWidgetKind {
            edge,
            index,
            kind: config.widget.kind.clone(),
        }),
    }
}

/// The four things a section can show, in the order a refusal offers them.
const WIDGET_KINDS: &[SectionKind<SectionWidget>] = &[
    SectionKind {
        name: "label",
        keys: &["text"],
        refuses: &[],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"content\"\n\
                  widget = { kind = \"label\", text = \"herdr\" }\n",
        build: label_widget,
    },
    SectionKind {
        name: "resource",
        keys: &["metric"],
        refuses: &[],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"content\"\n\
                  widget = { kind = \"resource\", metric = \"cpu\" }\n",
        build: resource_widget,
    },
    SectionKind {
        name: "icon",
        // Exactly one picture, and `text` only so a switched-off glyph still has
        // something to say.
        keys: &["glyph", "art", "pixels", "text"],
        refuses: &[],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"content\"\n\
                  widget = { kind = \"icon\", art = \"herd\" }\n",
        build: section_icon,
    },
    SectionKind {
        name: "meter",
        keys: &["metric"],
        refuses: &[],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"content\"\n\
                  widget = { kind = \"meter\", metric = \"mem\" }\n",
        build: meter_widget,
    },
];

fn label_widget(at: SectionAt<'_>) -> Result<SectionWidget, BarConfigProblem> {
    Ok(SectionWidget::Label {
        text: at.config.widget.text.clone(),
    })
}

fn resource_widget(at: SectionAt<'_>) -> Result<SectionWidget, BarConfigProblem> {
    named_metric(at).map(|metric| SectionWidget::Resource { metric })
}

fn meter_widget(at: SectionAt<'_>) -> Result<SectionWidget, BarConfigProblem> {
    named_metric(at).map(|metric| SectionWidget::Meter { metric })
}

/// A section that shows a number names its metric, and a metric this build does not
/// know is refused here rather than drawn as an empty section. The same reasoning as
/// an unknown widget kind: a typo that renders as blank is indistinguishable from one
/// that renders as nothing on purpose.
fn named_metric(at: SectionAt<'_>) -> Result<crate::resource::ResourceMetric, BarConfigProblem> {
    crate::resource::ResourceMetric::parse(&at.config.widget.metric).ok_or(
        BarConfigProblem::UnknownSectionWidgetMetric {
            edge: at.edge,
            index: at.index,
            metric: at.config.widget.metric.clone(),
        },
    )
}

/// One section's picture, or what is wrong with it.
///
/// Three ways to name a picture and exactly one may be used. They are not
/// ranked and never will be: a precedence rule between `glyph` and `pixels`
/// would be a decision the config file cannot show, so writing two is an error
/// rather than a silent choice.
///
/// The width check only fires for a section that declares its own cells. A
/// `fill` section's width is not known until the terminal has a size, and
/// refusing at that point would mean a config that loads on one screen and not
/// on another. What a runtime squeeze does instead is clip, which is what a
/// label already does when the window narrows.
// TP-ART-03/05: one picture per icon; a declared width that cannot hold it is
// refused where it is written, and a section sized by the terminal is not.
fn section_icon(at: SectionAt<'_>) -> Result<SectionWidget, BarConfigProblem> {
    let SectionAt {
        config,
        edge,
        index,
        glyph_icons,
    } = at;
    let widget = &config.widget;
    let named = usize::from(!widget.glyph.trim().is_empty())
        + usize::from(!widget.art.trim().is_empty())
        + usize::from(!widget.pixels.is_empty());
    match named {
        0 => return Err(BarConfigProblem::IconWithoutPicture { edge, index }),
        1 => {}
        _ => return Err(BarConfigProblem::IconWithTwoPictures { edge, index }),
    }

    // A section only knows its own width when it declared one.
    let declared = (config.kind.trim().eq_ignore_ascii_case("fixed")).then_some(config.cells);
    let refuse_unless_fits = |needs: u16| match declared {
        Some(has) if has < needs => Err(BarConfigProblem::IconDoesNotFit {
            edge,
            index,
            needs,
            has,
        }),
        _ => Ok(()),
    };

    if !widget.glyph.trim().is_empty() {
        // With the switch off the section keeps its place and its meaning by
        // saying the same thing in letters. That is a label, not a third kind
        // of widget: reusing it means the clipping rule, the geometry rule and
        // the unchanged-buffer property all come from the one place that
        // already owns them, instead of being written a second time and
        // drifting.
        if !glyph_icons {
            let text = widget.text.trim();
            if text.is_empty() {
                return Err(BarConfigProblem::IconGlyphOffWithoutText { edge, index });
            }
            return Ok(SectionWidget::Label {
                text: widget.text.clone(),
            });
        }
        let glyph = widget.glyph.clone();
        let needs = u16::try_from(unicode_width::UnicodeWidthStr::width(glyph.as_str()))
            .unwrap_or(u16::MAX);
        refuse_unless_fits(needs)?;
        return Ok(SectionWidget::Icon { glyph });
    }

    let (pixels, palette) = if widget.art.trim().is_empty() {
        (widget.pixels.clone(), widget.palette.clone())
    } else {
        crate::icon::builtin(widget.art.trim()).ok_or_else(|| BarConfigProblem::UnknownIconArt {
            edge,
            index,
            name: widget.art.trim().to_string(),
        })?
    };

    let art = crate::icon::art_from_pixels(&pixels, &palette).map_err(|problem| {
        BarConfigProblem::UnreadableIconArt {
            edge,
            index,
            problem,
        }
    })?;
    refuse_unless_fits(art.width())?;
    Ok(SectionWidget::Art { art })
}

/// One section's action table as an action, or what is wrong with it.
fn section_action(
    config: &ShellBarSectionConfig,
    edge: &'static str,
    index: usize,
) -> Result<SectionAction, BarConfigProblem> {
    // Naming no action is not one of the choices below; it is the absence of a
    // choice, and what is left to ask is whether a deleted action left anything
    // behind. That question belongs before the table rather than in it: it has no
    // action to build and no name a person could write.
    if config.action.kind.is_empty() {
        return no_action(config, edge, index);
    }

    match SectionKind::find(ACTION_KINDS, config.action.kind.as_str()) {
        Some(entry) => (entry.build)(SectionAt::plain(config, edge, index)),
        None => Err(BarConfigProblem::UnknownSectionActionKind {
            edge,
            index,
            kind: config.action.kind.clone(),
        }),
    }
}

/// A section that opens nothing, and the three ways a removed action leaves
/// something behind.
///
/// Each is the same half-finished edit seen from a different field: the command was
/// deleted and the geometry, the second gesture or the plugin id stayed. Saying so is
/// cheap; leaving it silent means the next person reads a setting that has never once
/// been used and believes it.
///
/// Asked only here. An unreadable action kind already reports its own cause, and a
/// second complaint about a field it also carries would send somebody to fix the
/// wrong line.
fn no_action(
    config: &ShellBarSectionConfig,
    edge: &'static str,
    index: usize,
) -> Result<SectionAction, BarConfigProblem> {
    if config.action.width.is_some() || config.action.height.is_some() {
        return Err(BarConfigProblem::PopupSizeWithoutPopup { edge, index });
    }
    if !config.action.secondary.is_empty() {
        return Err(BarConfigProblem::SecondaryWithoutAction { edge, index });
    }
    if !config.action.command.trim().is_empty() {
        return Err(BarConfigProblem::PluginCommandWithoutAction { edge, index });
    }
    Ok(SectionAction::None)
}

/// The two things a press can do, in the order a refusal offers them.
const ACTION_KINDS: &[SectionKind<SectionAction>] = &[
    SectionKind {
        name: "popup",
        keys: &["argv", "width", "height", "secondary"],
        // A popup runs what the file says; a plugin id here would be read by
        // nothing.
        refuses: &["command"],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"content\"\n\
                  widget = { kind = \"label\", text = \"status\" }\n\
                  action = { kind = \"popup\", argv = [\"herdr\", \"status\"] }\n",
        build: popup_action,
    },
    SectionKind {
        name: "plugin",
        keys: &["command"],
        // Command line, geometry and the second gesture all come from the
        // plugin's manifest, so every one of these is a setting nothing reads.
        refuses: &["argv", "width", "height", "secondary"],
        example: "[shell.bars.top]\nenabled = true\n\n\
                  [[shell.bars.top.sections]]\nkind = \"content\"\n\
                  widget = { kind = \"label\", text = \"files\" }\n\
                  action = { kind = \"plugin\", command = \"files.open\" }\n",
        build: plugin_action,
    },
];

fn popup_action(at: SectionAt<'_>) -> Result<SectionAction, BarConfigProblem> {
    let SectionAt {
        config,
        edge,
        index,
        ..
    } = at;
    // The mirror of the builder below. Without both directions, one of the two ways
    // to leave a plugin id somewhere it can never be read stays silent — and a silent
    // leftover is precisely the kind the next reader trusts.
    if !config.action.command.trim().is_empty() {
        return Err(BarConfigProblem::PopupActionWithPluginCommand { edge, index });
    }
    // An empty argv, or one made only of blanks, would ask the runtime to execute
    // nothing. Disk is untrusted input (CL1): refuse it here rather than discovering
    // it at the moment somebody clicks.
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
        width: config.action.width,
        height: config.action.height,
        secondary: secondary_presentation(&config.action.secondary, edge, index)?,
    })
}

fn plugin_action(at: SectionAt<'_>) -> Result<SectionAction, BarConfigProblem> {
    let SectionAt {
        config,
        edge,
        index,
        ..
    } = at;
    // The leftover popup fields are asked about first. When both a leftover and a
    // missing id are present the leftover is the better complaint: it names a line
    // that exists and is wrong, while the missing id names a line that is not there
    // to look at.
    if let Some(field) = plugin_action_popup_field(config) {
        return Err(BarConfigProblem::PluginActionWithPopupField { edge, index, field });
    }
    if !config.action.secondary.is_empty() {
        return Err(BarConfigProblem::PluginActionWithSecondary { edge, index });
    }
    // Trimmed only to decide whether anything was named. The stored id stays verbatim:
    // the resolver trims for itself, and reshaping a value here to make a check
    // convenient is how a config comes to mean something other than what it says.
    if config.action.command.trim().is_empty() {
        return Err(BarConfigProblem::PluginActionWithoutId { edge, index });
    }
    Ok(SectionAction::InvokePlugin {
        action: config.action.command.clone(),
    })
}

/// The first popup-only field a plugin action left behind, if it left one.
///
/// A plugin action's command line and its geometry both come from the plugin's
/// manifest, so every one of these is a setting nothing will ever read. Named
/// rather than counted: a complaint that says "something does not belong" sends
/// somebody to read the whole table, which is the cost this function exists to
/// avoid.
fn plugin_action_popup_field(config: &ShellBarSectionConfig) -> Option<&'static str> {
    if !config
        .action
        .argv
        .iter()
        .all(|argument| argument.trim().is_empty())
    {
        return Some("argv");
    }
    if config.action.width.is_some() {
        return Some("width");
    }
    if config.action.height.is_some() {
        return Some("height");
    }
    None
}

/// One section's `action.secondary` as a presentation, or what is wrong with it.
///
/// Matched exactly, with no trimming and no case folding. A near-miss like
/// `"TAB"` is far more likely to be a typo than a considered spelling, and
/// accepting it would mean the file no longer says what the build does — the
/// same reason every other kind in this module is matched exactly.
fn secondary_presentation(
    raw: &str,
    edge: &'static str,
    index: usize,
) -> Result<Option<SecondaryPresentation>, BarConfigProblem> {
    // Naming nothing is the absence of a second gesture rather than one of the
    // choices below, and unlike the other surfaces there is no leftover to ask
    // about: whatever a deleted presentation left behind belongs to the action
    // that carried it.
    if raw.is_empty() {
        return Ok(None);
    }
    SECONDARY_PRESENTATIONS
        .iter()
        .find(|(name, _)| *name == raw)
        .map(|(_, presentation)| Some(*presentation))
        .ok_or_else(|| BarConfigProblem::UnknownSecondaryPresentation {
            edge,
            index,
            presentation: raw.to_string(),
        })
}

/// The presentations a second gesture can ask for, in the order a refusal offers
/// them.
///
/// A pair rather than a builder: nothing is constructed here beyond the variant
/// itself, and a function pointer that only returns a constant would be ceremony
/// around a lookup.
const SECONDARY_PRESENTATIONS: &[(&str, SecondaryPresentation)] =
    &[("tab", SecondaryPresentation::Tab)];

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
pub(crate) fn bar_color(spec: &str, palette: &Palette) -> Color {
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
            ..Default::default()
        }
    }

    // A type in a bar config lives in six closed sets, and each one lives in two
    // places: the match that builds it, and the message that lists what it accepts.
    //
    //   surface      built by                       listed by                       refusal costs
    //   sizing       section_policy                 UnknownSectionKind              the whole bar
    //   widget       section_widget                 UnknownSectionWidgetKind        one blank section
    //   action       section_action                 UnknownSectionActionKind        one inert section
    //   secondary    secondary_presentation         UnknownSecondaryPresentation    one inert right press
    //   metric       resource::ResourceMetric       UnknownSectionWidgetMetric      one blank section
    //   icon art     icon::builtin                  UnknownIconArt                  one blank section
    //
    // Nothing holds the two halves together, so they drift in either direction and
    // both are silent: a name the parser takes but the message never mentions is a
    // feature nobody can find, and a name the message offers but the parser refuses
    // is a config somebody writes on the message's word.
    //
    // These pin today's answers before a type table moves them. The shape is the one
    // the documentation gate already uses: harvest the list out of the text, then ask
    // the product about every name in it. What differs is the source — a message the
    // product writes about itself rather than a guide somebody else maintains.

    /// The quoted names on the "expected" side of a refusal.
    ///
    /// The message carries the offending name in quotes too (`… is "gauge"; expected
    /// …`), so the harvest starts after `expected`. Losing that split would let the
    /// refused name count as an accepted one, and the check would agree with itself.
    fn expected_names(message: &str) -> Vec<String> {
        let tail = match message.split_once("expected ") {
            Some((_, tail)) => tail,
            None => return Vec::new(),
        };
        tail.split('"')
            .skip(1)
            .step_by(2)
            .map(|name| name.to_string())
            .collect()
    }

    /// One section in one bar, and whatever it is reported for.
    fn problems_for(section: ShellBarSectionConfig) -> Vec<String> {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section]),
            ..Default::default()
        };
        shell_bar_config_problems(&config, true)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect()
    }

    /// The refusal a surface writes when it is handed a name it does not know.
    fn message_naming(section: ShellBarSectionConfig, needle: &str) -> String {
        problems_for(section)
            .into_iter()
            .find(|text| text.contains(needle))
            .unwrap_or_else(|| panic!("no problem mentioned {needle:?}"))
    }

    /// Every name a message lists is a name its surface takes.
    ///
    /// The claim is not that nothing is reported — a resource still wants a metric, an
    /// icon still wants a picture, a fixed section still wants cells. It is that the
    /// name itself was recognised, which is the half a type table is about to own.
    fn assert_every_listed_name_is_accepted(
        message: &str,
        at_least: usize,
        refusal_shape: impl Fn(&str) -> String,
        apply: impl Fn(&str) -> ShellBarSectionConfig,
    ) {
        let listed = expected_names(message);

        // The harvest is pinned as well. Reword the message and it would find nothing,
        // the loop below would never run, and the check would pass in silence — which
        // is the failure mode this whole shape exists to prevent.
        assert!(
            listed.len() >= at_least,
            "the message no longer lists what it accepts: {message}"
        );

        for name in &listed {
            let refused = problems_for(apply(name))
                .into_iter()
                .any(|problem| problem.contains(&refusal_shape(name)));
            assert!(
                !refused,
                "the message lists {name:?} but the parser refuses it"
            );
        }
    }

    // TP-CHROME-99: the sizing surface, whose refusal costs the whole bar rather than
    // one section, so a message that offers a kind the parser will not take is the
    // most expensive of the six to get wrong.
    #[test]
    fn the_section_kinds_the_message_lists_are_exactly_the_ones_accepted() {
        let message = message_naming(plain_section("stretch"), "sections[0].kind is \"stretch\"");

        // The anchor carries `sections[0].` because a bare `kind is "…"` is a substring
        // of `widget.kind is "…"` and `action.kind is "…"` as well. Three surfaces share
        // one sentence and only the prefix tells them apart.
        assert_every_listed_name_is_accepted(
            &message,
            3,
            |name| format!("sections[0].kind is \"{name}\""),
            plain_section,
        );

        assert!(
            !expected_names(&message)
                .iter()
                .any(|name| name == "stretch"),
            "an unknown sizing kind leaked into the accepted list"
        );
    }

    // TP-CHROME-96: the widget surface.
    #[test]
    fn the_widget_kinds_the_message_lists_are_exactly_the_ones_accepted() {
        let mut unknown = plain_section("fill");
        unknown.widget.kind = "gauge".to_string();
        let message = message_naming(unknown, "widget.kind is \"gauge\"");

        assert_every_listed_name_is_accepted(
            &message,
            4,
            |name| format!("widget.kind is \"{name}\""),
            |name| {
                let mut section = plain_section("fill");
                section.widget.kind = name.to_string();
                section
            },
        );
    }

    // TP-CHROME-97: the action surface. A refused action costs only its own section,
    // which makes this the quietest drift of the four: the bar still divides and the
    // part simply stops answering.
    #[test]
    fn the_action_kinds_the_message_lists_are_exactly_the_ones_accepted() {
        let mut unknown = plain_section("fill");
        unknown.action.kind = "teleport".to_string();
        let message = message_naming(unknown, "action.kind is \"teleport\"");

        assert_every_listed_name_is_accepted(
            &message,
            2,
            |name| format!("action.kind is \"{name}\""),
            |name| {
                let mut section = plain_section("fill");
                section.action.kind = name.to_string();
                section
            },
        );
    }

    // TP-CHROME-98: this pair crosses a module boundary. The message is written here
    // and the parser lives in `crate::resource`, so a table gathered inside this file
    // cannot close the gap — it can only keep it visible.
    #[test]
    fn the_metrics_the_message_lists_are_exactly_the_ones_resource_parses() {
        let mut section = plain_section("fill");
        section.widget.kind = "resource".to_string();
        section.widget.metric = "flux".to_string();
        let message = message_naming(section, "widget.metric is \"flux\"");

        let listed = expected_names(&message);
        assert!(
            listed.len() >= 3,
            "the message no longer lists the accepted metrics: {message}"
        );
        for metric in &listed {
            assert!(
                crate::resource::ResourceMetric::parse(metric).is_some(),
                "the message lists {metric:?} but ResourceMetric does not parse it"
            );
        }
        assert!(
            crate::resource::ResourceMetric::parse("flux").is_none(),
            "an unknown metric parsed"
        );
    }

    // TP-CHROME-102: `ram` is taken and never advertised.
    //
    // This is characterisation, not endorsement. The alias works and the guide says so,
    // but the refusal names only cpu, mem and swap, so somebody who mistypes it is told
    // nothing about the spelling that would have worked. A type table has to carry
    // aliases or it will drop this one without a word; adding it to the message instead
    // is a decision with its own docs, and either way this test should be the thing
    // that makes the change deliberate.
    #[test]
    fn the_mem_metric_answers_to_ram_without_advertising_it() {
        assert_eq!(
            crate::resource::ResourceMetric::parse("ram"),
            crate::resource::ResourceMetric::parse("mem"),
            "ram stopped being an alias for mem"
        );

        let mut section = plain_section("fill");
        section.widget.kind = "resource".to_string();
        section.widget.metric = "flux".to_string();
        let message = message_naming(section, "widget.metric is \"flux\"");
        assert!(
            !expected_names(&message).iter().any(|name| name == "ram"),
            "ram started being advertised, which is a behaviour change rather than a \
             refactor: {message}"
        );
    }

    // TP-CHROME-103: the secondary surface, and the only list with a single member.
    //
    // `expected "tab"` has no separator in it. A generated list pinned only against the
    // longer forms could produce a broken sentence here and nothing else would notice.
    //
    // The presentation is only read on the popup arm, and an empty argv is refused
    // before it, so reaching this refusal needs a popup that is otherwise valid.
    #[test]
    fn the_secondary_presentations_the_message_lists_are_exactly_the_ones_accepted() {
        let mut unknown = section_with_action("fill", "popup", &["true"]);
        unknown.action.secondary = "tabs".to_string();
        let message = message_naming(unknown, "action.secondary is \"tabs\"");

        assert_every_listed_name_is_accepted(
            &message,
            1,
            |name| format!("action.secondary is \"{name}\""),
            |name| {
                let mut section = section_with_action("fill", "popup", &["true"]);
                section.action.secondary = name.to_string();
                section
            },
        );
    }

    // TP-CHROME-104: the icon catalogue says what it bundles, like the other five.
    //
    // It was once the exception. A `match` can be asked whether it knows a name but
    // never asked what names it knows, so this refusal turned a config down and left
    // the reader with nowhere to look — the one closed set in the grammar that kept
    // its contents to itself. The earlier version of this test pinned that gap and
    // said closing it would turn the test red on purpose; the table in `crate::icon`
    // closed it, and this is the same row now pinning the other direction.
    #[test]
    fn the_icon_catalogue_names_what_it_bundles() {
        let art_section = |name: &str| {
            let mut section = plain_section("fill");
            section.widget.kind = "icon".to_string();
            section.widget.art = name.to_string();
            section
        };
        let message = message_naming(art_section("hrd"), "widget.art is \"hrd\"");

        assert!(
            message.contains("does not bundle"),
            "the icon refusal changed shape: {message}"
        );

        // The same contract the other five surfaces carry: every name a message
        // offers is a name its surface takes. A picture still has to fit the
        // section it is put in, so what is checked is that the name itself was
        // recognised rather than that nothing at all is reported.
        assert_every_listed_name_is_accepted(
            &message,
            1,
            |name| format!("widget.art is \"{name}\""),
            art_section,
        );

        assert!(
            !expected_names(&message).iter().any(|name| name == "hrd"),
            "an unknown picture leaked into the accepted list: {message}"
        );
    }

    // TP-CHROME-101: the empty kind is never offered as something to write.
    //
    // It is not a kind. It is where the leftover checks live, and they ask a different
    // question — whether a width, a presentation or a plugin id outlived the action
    // that gave them meaning. Listing `""` would invite people to write it, and a
    // refactor that folded it into the kind table would lose those checks entirely.
    #[test]
    fn the_empty_kind_is_never_offered_as_something_to_write() {
        let mut widget = plain_section("fill");
        widget.widget.kind = "gauge".to_string();
        let mut action = plain_section("fill");
        action.action.kind = "teleport".to_string();

        for message in [
            message_naming(plain_section("stretch"), "sections[0].kind is \"stretch\""),
            message_naming(widget, "widget.kind is \"gauge\""),
            message_naming(action, "action.kind is \"teleport\""),
        ] {
            assert!(
                !expected_names(&message).iter().any(|name| name.is_empty()),
                "the empty kind was offered as something to write: {message}"
            );
        }
    }

    // The harvest reads only what follows `expected`, including the one-item form that
    // has no separator to lean on.
    #[test]
    fn the_harvest_reads_only_what_comes_after_expected() {
        let message = "shell.bars.top.sections[0].kind is \"stretch\"; expected \"fixed\", \
                       \"fill\" or \"content\", so this bar is drawn undivided";
        assert_eq!(expected_names(message), vec!["fixed", "fill", "content"]);
        assert_eq!(expected_names("no such word here"), Vec::<String>::new());
        assert_eq!(
            expected_names("… action.secondary is \"tabs\"; expected \"tab\", so a right press …"),
            vec!["tab"]
        );
    }

    // TP-CHROME-105: the phrase a generated list has to reproduce, at every length the
    // six refusals actually use. Pinning one arity would not pin the generator: the single-name
    // form has no separator to get wrong, and the longer ones differ only in where
    // the `or` falls. All four are taken from the messages as they read today.
    #[test]
    fn a_generated_list_reads_the_way_the_refusals_already_read() {
        assert_eq!(accepted_names(&["tab"]), r#""tab""#);
        assert_eq!(
            accepted_names(&["popup", "plugin"]),
            r#""popup" or "plugin""#
        );
        assert_eq!(
            accepted_names(&["fixed", "fill", "content"]),
            r#""fixed", "fill" or "content""#
        );
        assert_eq!(
            accepted_names(&["label", "resource", "icon", "meter"]),
            r#""label", "resource", "icon" or "meter""#
        );

        // An empty closed set cannot be written and would read as a missing sentence
        // rather than a refusal, so it is named here rather than discovered later.
        assert_eq!(accepted_names(&[]), "");
    }

    fn plain_section(kind: &str) -> ShellBarSectionConfig {
        ShellBarSectionConfig {
            kind: kind.to_string(),
            cells: 4,
            max: 4,
            ..Default::default()
        }
    }

    /// The `toml` blocks of the guide's "Edge bars" section, in the order they
    /// are written.
    /// The guide, as one string.
    fn configuration_guide() -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/next/website/src/content/docs/configuration.mdx");
        std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("the configuration guide at {path:?} is readable: {err}"))
    }

    /// Just the part of the guide that documents bars.
    ///
    /// Scoped rather than whole, for the checks that read the guide's own words
    /// back: `kind` is a key several unrelated things could use, and a test that
    /// held the whole file to the bar grammar would turn red the day somebody
    /// documented one of them.
    fn edge_bars_section() -> String {
        let guide = configuration_guide();
        let after_heading = guide
            .split_once("\n## Edge bars\n")
            .unwrap_or_else(|| panic!("the guide still has an \"Edge bars\" section"))
            .1;
        after_heading
            .split_once("\n## ")
            .map_or(after_heading, |(before, _)| before)
            .to_string()
    }

    fn documented_bar_examples() -> Vec<String> {
        edge_bars_section()
            .split("```toml\n")
            .skip(1)
            .filter_map(|rest| rest.split_once("```"))
            .map(|(block, _)| block.to_string())
            .collect()
    }

    /// Every example in the guide is a config this build accepts.
    ///
    /// A documented example is the first thing anyone copies, an agent most of
    /// all, so an example that quietly produces an empty section is worse than
    /// no example at all. The parser here is the real one rather than the
    /// installed binary's: measured 2026-08-15, running the guide's examples
    /// through `~/.local/bin/herdr` reported an "unknown config key" for a key
    /// this branch had just added — the check was one commit behind the thing it
    /// was checking. In-process, that skew cannot exist.
    #[test]
    fn every_bar_example_in_the_guide_is_a_config_this_build_accepts() {
        // TP-CHROME-94: every example the guide prints is a config this build
        // accepts. A documented example is the first thing anyone copies, an
        // agent most of all, so one that quietly produces an empty section is
        // worse than no example at all.
        let examples = documented_bar_examples();

        // Guard the harvest itself: a renamed heading or a changed fence would
        // leave this test passing over nothing at all, which is the failure mode
        // a documentation gate is least likely to notice about itself.
        assert!(
            examples.len() >= 5,
            "expected the Edge bars section to still carry its examples, found {}",
            examples.len()
        );

        for (index, block) in examples.iter().enumerate() {
            let config: crate::config::Config = toml::from_str(block).unwrap_or_else(|err| {
                panic!(
                    "guide example {} is not valid TOML: {err}\n{block}",
                    index + 1
                )
            });
            let problems = shell_bar_config_problems(&config.shell.bars, config.shell.glyph_icons)
                .into_iter()
                .map(|problem| problem.to_string())
                .collect::<Vec<_>>();
            assert!(
                problems.is_empty(),
                "guide example {} is refused by this build: {problems:?}\n{block}",
                index + 1
            );
        }
    }

    /// The guide shows every kind there is, and shows nothing that is not one.
    ///
    /// Both directions matter and they fail differently: a kind the guide never
    /// mentions is a feature nobody can find, and a kind the guide invents is a
    /// config an agent will write and Herdr will refuse.
    ///
    /// Every list below is read from the table that also carries the code
    /// building each name. It used to be typed out here, which made this test
    /// the second place the kinds were written down and left it able to agree
    /// with a guide that both had fallen behind. Adding a kind now fails this
    /// test until the guide learns it, without anybody having to remember that
    /// this file exists.
    #[test]
    fn the_guide_shows_every_widget_and_action_kind() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/next/website/src/content/docs/configuration.mdx");
        let raw = std::fs::read_to_string(&path).expect("the configuration guide is readable");
        // The guide aligns its assignments, so a section kind is written
        // `kind  = "fixed"` with the spaces padded out. Runs of blanks are
        // collapsed here rather than matched exactly: a gate that reports a
        // documented kind as missing because of the width of a space sends
        // somebody to edit a guide that was already right, and a gate nobody
        // believes is worse than no gate. Newlines are left alone so nothing
        // matches across two lines that never sat together.
        let guide = raw
            .split('\n')
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n");

        // TP-CHROME-95: the guide shows every kind there is and invents none.
        // A kind it never mentions is a feature nobody can find; a kind it
        // invents is a config an agent writes and Herdr refuses.
        for (surface, kinds) in [
            ("sizing", SectionKind::offered(SIZING_KINDS)),
            ("widget", SectionKind::offered(WIDGET_KINDS)),
            ("action", SectionKind::offered(ACTION_KINDS)),
        ] {
            assert!(!kinds.is_empty(), "the {surface} table emptied out");
            for kind in kinds {
                assert!(
                    guide.contains(&format!("kind = \"{kind}\"")),
                    "the guide never shows {surface} kind {kind:?}"
                );
            }
        }
        for art in crate::icon::builtin_names() {
            assert!(
                guide.contains(art),
                "the guide never names the bundled art {art:?}"
            );
        }
        // Aliases as well as names: `ram` works, and a guide that omits it
        // leaves the one spelling a refusal will never teach undocumented
        // anywhere at all.
        for metric in crate::resource::ResourceMetric::ACCEPTED
            .iter()
            .copied()
            .chain(
                crate::resource::ResourceMetric::ALIASES
                    .iter()
                    .map(|(alias, _)| *alias),
            )
        {
            assert!(
                guide.contains(metric),
                "the guide never names the metric {metric:?}"
            );
        }

        // The other direction, which this row has always claimed and never
        // actually checked. Everything above walks the tables and looks each
        // name up in the guide, so a name the guide invents — or one it keeps
        // showing after the build stopped accepting it — passes silently. That
        // was measured: deleting a bundled picture from its table broke nothing
        // at all, because a loop over the table simply stopped looking for it.
        //
        // So the guide is read the other way round: every kind and every
        // picture it writes out has to be one this build takes. Only the bar
        // section, because `kind` is a key other things could use one day.
        let bars = edge_bars_section()
            .split('\n')
            .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join("\n");
        let written = |assignment: &str| {
            bars.match_indices(&format!("{assignment} = \""))
                .filter_map(|(at, matched)| {
                    let rest = &bars[at + matched.len()..];
                    rest.find('"').map(|end| rest[..end].to_string())
                })
                .collect::<Vec<_>>()
        };

        let known_kinds = SectionKind::offered(SIZING_KINDS)
            .into_iter()
            .chain(SectionKind::offered(WIDGET_KINDS))
            .chain(SectionKind::offered(ACTION_KINDS))
            .collect::<Vec<_>>();
        let shown_kinds = written("kind");
        assert!(
            shown_kinds.len() >= known_kinds.len(),
            "the guide stopped showing kinds at all, so the check below reads nothing: \
             {shown_kinds:?}"
        );
        for kind in &shown_kinds {
            assert!(
                known_kinds.contains(&kind.as_str()),
                "the guide writes kind {kind:?}, which this build does not accept"
            );
        }

        let bundled = crate::icon::builtin_names();
        for art in written("art") {
            assert!(
                bundled.contains(&art.as_str()),
                "the guide writes art {art:?}, which this build does not bundle"
            );
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
            ..Default::default()
        }
    }

    /// A glyph is only a picture where the font has it, and herdr cannot ask
    /// whether it does. These four tests are the whole of what the switch is
    /// for: what it changes, what it refuses, what it must not touch, and what
    /// does not count as something to fall back to.
    #[test]
    fn a_switched_off_glyph_draws_its_text_as_a_label() {
        let mut section = plain_section("fixed");
        section.widget.kind = "icon".to_string();
        section.widget.glyph = "★".to_string();
        section.widget.text = "cpu".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section]),
            ..Default::default()
        };

        // TP-CHROME-89: the switch defaults on and changes nothing there, because
        // shipping it must not alter a bar anybody already has.
        assert_eq!(
            ShellBarChrome::from_config(&config, true).widget_for(RegionId::TopBar, 0),
            Some(&SectionWidget::Icon {
                glyph: "★".to_string()
            }),
        );

        // TP-CHROME-90: off, the section keeps its meaning in letters — and what
        // it becomes is the label that already owns clipping and geometry, not a
        // third kind of widget that would own them a second time.
        assert_eq!(
            ShellBarChrome::from_config(&config, false).widget_for(RegionId::TopBar, 0),
            Some(&SectionWidget::Label {
                text: "cpu".to_string()
            }),
        );
    }

    #[test]
    fn a_switched_off_glyph_with_nothing_to_say_is_reported() {
        let mut section = plain_section("fixed");
        section.widget.kind = "icon".to_string();
        section.widget.glyph = "★".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section]),
            ..Default::default()
        };

        assert!(
            shell_bar_config_problems(&config, true).is_empty(),
            "a glyph with no text is a complete section while glyphs are on"
        );

        // TP-CHROME-91: turning the switch off can invalidate a section that was
        // valid before, and it says so. Drawing nothing instead would be
        // indistinguishable from a section somebody left empty on purpose.
        let reported = shell_bar_config_problems(&config, false)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();
        assert_eq!(reported.len(), 1, "{reported:?}");
        assert!(
            reported[0].contains("names no text to draw instead"),
            "{reported:?}"
        );
    }

    #[test]
    fn the_glyph_switch_leaves_pictures_made_of_cells_alone() {
        let mut section = plain_section("fill");
        section.widget.kind = "icon".to_string();
        section.widget.art = "herd".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section]),
            ..Default::default()
        };

        // TP-CHROME-92: the switch is a statement about fonts. A half block is
        // not a font question, so turning glyphs off must not erase a picture
        // that was never at risk — that would destroy information for nothing.
        assert_eq!(
            ShellBarChrome::from_config(&config, true).widget_for(RegionId::TopBar, 0),
            ShellBarChrome::from_config(&config, false).widget_for(RegionId::TopBar, 0),
        );
        assert!(shell_bar_config_problems(&config, false).is_empty());
    }

    #[test]
    fn whitespace_is_not_something_to_fall_back_to() {
        let mut section = plain_section("fixed");
        section.widget.kind = "icon".to_string();
        section.widget.glyph = "★".to_string();
        section.widget.text = "   ".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section]),
            ..Default::default()
        };

        // TP-CHROME-93: a text of spaces draws the same nothing an absent one
        // does, and the person who wrote it believes they have a fallback. The
        // blank is caught where it is written rather than on screen.
        let reported = shell_bar_config_problems(&config, false)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();
        assert_eq!(reported.len(), 1, "{reported:?}");
        assert!(
            reported[0].contains("names no text to draw instead"),
            "{reported:?}"
        );
    }

    fn popup_argv(actions: &ShellBarChrome, region: RegionId, index: u8) -> Option<Vec<String>> {
        match actions.action_for(region, index) {
            Some(SectionAction::OpenPopup { argv, .. }) => Some(argv.clone()),
            _ => None,
        }
    }

    fn popup_size(
        actions: &ShellBarChrome,
        region: RegionId,
        index: u8,
    ) -> Option<(Option<PopupSize>, Option<PopupSize>)> {
        match actions.action_for(region, index) {
            Some(SectionAction::OpenPopup { width, height, .. }) => Some((*width, *height)),
            _ => None,
        }
    }

    fn sized_popup_section(width: &str, height: &str) -> ShellBarSectionConfig {
        let mut section = section_with_action("fill", "popup", &["btop"]);
        if !width.is_empty() {
            section.action.width =
                Some(crate::popup_size::PopupSize::parse_cli(width).expect("width fixture"));
        }
        if !height.is_empty() {
            section.action.height =
                Some(crate::popup_size::PopupSize::parse_cli(height).expect("height fixture"));
        }
        section
    }

    // TC-B1/TC-B2/TC-B3 · the size the person wrote survives the derivation, in
    // both spellings, and its absence stays absent rather than becoming a
    // number this layer invented.
    #[test]
    fn a_popup_action_carries_the_size_it_was_written_with() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                sized_popup_section("80%", "60%"),
                sized_popup_section("100", "40"),
                section_with_action("fill", "popup", &["btop"]),
            ]),
            ..Default::default()
        };

        let actions = ShellBarChrome::from_config(&config, true);

        assert_eq!(
            popup_size(&actions, RegionId::TopBar, 0),
            Some((
                Some(crate::popup_size::PopupSize::Percent(80)),
                Some(crate::popup_size::PopupSize::Percent(60))
            )),
            "a percentage must arrive as a percentage, unresolved"
        );
        assert_eq!(
            popup_size(&actions, RegionId::TopBar, 1),
            Some((
                Some(crate::popup_size::PopupSize::Cells(100)),
                Some(crate::popup_size::PopupSize::Cells(40))
            )),
            "cells and percentages must read through one parser"
        );
        assert_eq!(
            popup_size(&actions, RegionId::TopBar, 2),
            Some((None, None)),
            "a size nobody wrote must stay absent so the popup keeps its default"
        );
    }

    // TC-C4/TC-C6 · a widget this build cannot show is reported by the same
    // predicate that refuses it, and text with no widget to show it is the same
    // half-finished shape as a leftover popup size. And a label never reaches
    // the value the geometry cache compares: editing text must not re-lay-out
    // the bar.
    // TC-I6/TC-I8/TC-I10 · every way of writing a picture wrong is refused
    // where it is written, each with its own cause.
    //
    // A picture fails silently in a way text does not. Wrong text is still
    // text on screen; a wrong picture is an empty rectangle, which is exactly
    // what a section with no widget looks like. So the checker has to be the
    // thing that speaks, and it has to name which of the three mistakes it is:
    // no picture, two pictures, or one that cannot fit.
    // TP-ART-03: an icon that cannot be drawn is reported, and carries no widget.
    #[test]
    fn every_way_of_writing_a_picture_wrong_is_reported_with_its_own_cause() {
        let mut nothing = plain_section("fill");
        nothing.widget.kind = "icon".to_string();

        let mut both = plain_section("fill");
        both.widget.kind = "icon".to_string();
        both.widget.glyph = "*".to_string();
        both.widget.art = "herd".to_string();

        let mut unknown = plain_section("fill");
        unknown.widget.kind = "icon".to_string();
        unknown.widget.art = "no-such-mark".to_string();

        // `herd` is ten cells wide and this section declares four.
        let mut cramped = plain_section("fixed");
        cramped.cells = 4;
        cramped.widget.kind = "icon".to_string();
        cramped.widget.art = "herd".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![nothing, both, unknown, cramped]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();

        assert_eq!(reported.len(), 4, "{reported:?}");
        assert!(reported[0].contains("names no picture"), "{reported:?}");
        assert!(
            reported[1].contains("more than one picture"),
            "{reported:?}"
        );
        assert!(reported[2].contains("\"no-such-mark\""), "{reported:?}");
        assert!(
            reported[3].contains("needs 10 cells but the section declares 4"),
            "a refusal has to name both numbers or it cannot be acted on: {reported:?}"
        );

        let chrome = ShellBarChrome::from_config(&config, true);
        for index in 0..4 {
            assert_eq!(
                chrome.widget_for(RegionId::TopBar, index),
                Some(&SectionWidget::None),
                "section {index}: a picture the checker refused must not be carried either"
            );
        }
    }

    // A `fill` section has no width until the terminal has a size, so refusing
    // one at config time would mean a file that loads on one screen and not on
    // another. What a runtime squeeze does instead is clip, exactly as a label
    // already does — the refusal is for a width somebody wrote down, not for a
    // window somebody dragged.
    // TP-ART-05: only a declared width can refuse a picture.
    #[test]
    fn a_picture_in_a_section_with_no_declared_width_is_accepted_and_left_to_the_renderer() {
        let mut section = plain_section("fill");
        section.widget.kind = "icon".to_string();
        section.widget.art = "herd".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section]),
            ..Default::default()
        };

        assert!(shell_bar_config_problems(&config, true).is_empty());
        let chrome = ShellBarChrome::from_config(&config, true);
        let widget = chrome
            .widget_for(RegionId::TopBar, 0)
            .expect("the section has chrome");
        assert!(
            matches!(widget, SectionWidget::Art { art } if art.width() == 10),
            "a fill section keeps the picture: {widget:?}"
        );
    }

    // A metric is a second name inside a widget that already named itself, and
    // a wrong one fails in the same silent way a wrong kind does: the section
    // draws nothing and looks exactly like a section meant to draw nothing.
    // It gets its own message rather than sharing the unknown-kind one, because
    // the two send a person to different lines of the same file.
    // TP-CHROME-56: an unknown metric is refused, and carries no widget.
    #[test]
    fn a_resource_widget_with_an_unknown_metric_is_reported_and_never_reaches_the_geometry() {
        let mut typo = plain_section("fill");
        typo.widget.kind = "resource".to_string();
        typo.widget.metric = "cpu%".to_string();
        let mut good = plain_section("fill");
        good.widget.kind = "resource".to_string();
        good.widget.metric = "swap".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![typo, good]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();

        assert_eq!(reported.len(), 1, "{reported:?}");
        assert!(
            reported[0].contains("sections[0].widget.metric is \"cpu%\""),
            "the message has to name the metric that was wrong: {reported:?}"
        );

        let chrome = ShellBarChrome::from_config(&config, true);
        assert_eq!(
            chrome.widget_for(RegionId::TopBar, 0),
            Some(&SectionWidget::None),
            "a metric the checker refused must not be carried either"
        );
        assert_eq!(
            chrome.widget_for(RegionId::TopBar, 1),
            Some(&SectionWidget::Resource {
                metric: crate::resource::ResourceMetric::Swap
            })
        );
        assert!(
            chrome.wants_resources(),
            "one good live section is enough to make the loop sample"
        );
    }

    #[test]
    fn a_widget_this_build_cannot_show_is_reported_and_never_reaches_the_geometry() {
        let mut wrong_kind = plain_section("fill");
        wrong_kind.widget.kind = "sparkline".to_string();
        let mut orphan_text = plain_section("fill");
        orphan_text.widget.text = "CPU".to_string();
        let mut label = plain_section("fill");
        label.widget.kind = "label".to_string();
        label.widget.text = "CPU".to_string();

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![wrong_kind, orphan_text, label]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();

        assert_eq!(reported.len(), 2, "{reported:?}");
        assert!(
            reported[0].contains("sections[0].widget.kind is \"sparkline\""),
            "{reported:?}"
        );
        assert!(
            reported[1].contains("sections[1].widget sets text but names no widget"),
            "{reported:?}"
        );

        let chrome = ShellBarChrome::from_config(&config, true);
        assert_eq!(
            chrome.widget_for(RegionId::TopBar, 0),
            Some(&SectionWidget::None),
            "a widget the checker complained about must not be carried either"
        );
        assert_eq!(
            chrome.widget_for(RegionId::TopBar, 2),
            Some(&SectionWidget::Label {
                text: "CPU".to_string()
            })
        );

        // TC-C6: only the text differs, and the bars the geometry key compares
        // must not notice.
        let mut renamed = plain_section("fill");
        renamed.widget.kind = "label".to_string();
        renamed.widget.text = "MEM".to_string();
        let other = ShellBarsConfig {
            top: bar_with_sections(vec![renamed]),
            ..Default::default()
        };
        let one = ShellBarsConfig {
            top: bar_with_sections(vec![label_section("CPU")]),
            ..Default::default()
        };
        assert_eq!(
            ShellBars::from_config(&one),
            ShellBars::from_config(&other),
            "a label must never decide how wide its section is"
        );
        assert_ne!(
            ShellBarChrome::from_config(&one, true),
            ShellBarChrome::from_config(&other, true),
            "control: the chrome itself must notice, or the assertion above is empty"
        );
    }

    fn label_section(text: &str) -> ShellBarSectionConfig {
        let mut section = plain_section("fill");
        section.widget.kind = "label".to_string();
        section.widget.text = text.to_string();
        section
    }

    // TC-B5 · a size on an action that will never open a popup can never take
    // effect, so it is said out loud rather than dropped where nobody sees it.
    #[test]
    fn a_popup_size_on_something_that_is_not_a_popup_is_reported() {
        let mut leftover_width = plain_section("fill");
        leftover_width.action.width = Some(crate::popup_size::PopupSize::Percent(80));
        let mut leftover_height = plain_section("fill");
        leftover_height.action.height = Some(crate::popup_size::PopupSize::Cells(40));

        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                leftover_width,
                leftover_height,
                sized_popup_section("80%", "60%"),
            ]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();

        assert_eq!(
            reported.len(),
            2,
            "each leftover size is reported once, and the popup that uses its \
             own size is not a complaint: {reported:?}"
        );
        assert!(
            reported[0].contains("sections[0].action sets a popup size but opens no popup"),
            "{reported:?}"
        );
        assert!(
            reported[1].contains("sections[1].action sets a popup size but opens no popup"),
            "a leftover height counts the same as a leftover width: {reported:?}"
        );

        // An unreadable action kind already reports its own cause. Complaining
        // about the size it also carries would send somebody to fix the line
        // that is not the reason their section does nothing.
        let mut wrong_kind = section_with_action("fill", "teleport", &["nowhere"]);
        wrong_kind.action.height = Some(crate::popup_size::PopupSize::Cells(40));
        let single_cause = ShellBarsConfig {
            top: bar_with_sections(vec![wrong_kind]),
            ..Default::default()
        };
        let reported = shell_bar_config_problems(&single_cause, true)
            .into_iter()
            .map(|problem| problem.to_string())
            .collect::<Vec<_>>();
        assert_eq!(reported.len(), 1, "one cause, one complaint: {reported:?}");
        assert!(
            reported[0].contains("action.kind is \"teleport\""),
            "{reported:?}"
        );
    }

    // The checker and the deriver stay joined across this rule too: a size the
    // checker complains about is a size the derived action does not carry, and
    // there is no third state where it is quietly kept.
    #[test]
    fn a_reported_leftover_size_is_a_size_the_action_does_not_carry() {
        let mut leftover = plain_section("fill");
        leftover.action.width = Some(crate::popup_size::PopupSize::Percent(80));
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![leftover]),
            ..Default::default()
        };

        assert_eq!(shell_bar_config_problems(&config, true).len(), 1);
        assert_eq!(
            ShellBarChrome::from_config(&config, true).action_for(RegionId::TopBar, 0),
            Some(&SectionAction::None),
            "the section the checker complained about must carry no action at all"
        );
    }

    // TC-B6 · a popup's size decides nothing about where a rectangle is, so it
    // must not reach the geometry key: editing a command's size would otherwise
    // invalidate every cached rectangle on screen for a change that moves
    // nothing.
    #[test]
    fn changing_only_a_popup_size_leaves_the_bars_identity_alone() {
        let small = ShellBarsConfig {
            top: bar_with_sections(vec![sized_popup_section("40%", "40%")]),
            ..Default::default()
        };
        let large = ShellBarsConfig {
            top: bar_with_sections(vec![sized_popup_section("90%", "90%")]),
            ..Default::default()
        };

        assert_eq!(
            ShellBars::from_config(&small),
            ShellBars::from_config(&large),
            "the bars value the geometry key compares must not notice a popup size"
        );
        assert_ne!(
            ShellBarChrome::from_config(&small, true),
            ShellBarChrome::from_config(&large, true),
            "control: the actions themselves must notice, or the test above proves nothing"
        );
    }

    fn popup_secondary(
        actions: &ShellBarChrome,
        region: RegionId,
        index: u8,
    ) -> Option<Option<SecondaryPresentation>> {
        match actions.action_for(region, index) {
            Some(SectionAction::OpenPopup { secondary, .. }) => Some(*secondary),
            _ => None,
        }
    }

    fn secondary_section(action_kind: &str, secondary: &str) -> ShellBarSectionConfig {
        let mut section = section_with_action("fill", action_kind, &["btop"]);
        if action_kind.is_empty() {
            section.action.argv.clear();
        }
        section.action.secondary = secondary.to_string();
        section
    }

    fn plugin_section(command: &str) -> ShellBarSectionConfig {
        let mut section = plain_section("fill");
        section.action.kind = "plugin".to_string();
        section.action.command = command.to_string();
        section
    }

    fn plugin_action(actions: &ShellBarChrome, region: RegionId, index: u8) -> Option<String> {
        match actions.action_for(region, index) {
            Some(SectionAction::InvokePlugin { action }) => Some(action.clone()),
            _ => None,
        }
    }

    /// How many sections of this bar can actually be addressed by index.
    ///
    /// Counted through the public accessor rather than a length field, because
    /// "addressable" is the property everything downstream depends on: a
    /// section that exists in a vector but answers `None` at its own index is
    /// a section nothing can click.
    fn addressable_sections(config: &ShellBarsConfig, region: RegionId) -> usize {
        let chrome = ShellBarChrome::from_config(config, true);
        (0..u8::MAX)
            .take_while(|index| chrome.for_section(region, *index).is_some())
            .count()
    }

    fn bar_with_budget(budget: u16, count: usize) -> ShellBarConfig {
        let mut bar = bar_with_sections((0..count).map(|_| plain_section("fill")).collect());
        bar.max_sections = budget;
        bar
    }

    // TC-69-1 · the ceiling a file meets when it never mentions one is still 8.
    // This is the regression gate for every config written before the key
    // existed: those files must keep refusing exactly the section they refused
    // yesterday, with the same message.
    // TP-CHROME-70: a bar that names no budget is bounded at eight, as it
    // always was.
    #[test]
    fn a_bar_that_names_no_budget_is_still_bounded_at_eight() {
        let eight = ShellBarsConfig {
            top: bar_with_sections((0..8).map(|_| plain_section("fill")).collect()),
            ..Default::default()
        };
        assert!(
            shell_bar_config_problems(&eight, true).is_empty(),
            "eight was always allowed"
        );
        assert_eq!(addressable_sections(&eight, RegionId::TopBar), 8);

        let nine = ShellBarsConfig {
            top: bar_with_sections((0..9).map(|_| plain_section("fill")).collect()),
            ..Default::default()
        };
        let reported = shell_bar_config_problems(&nine, true)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(reported.len(), 1, "one cause: {reported:?}");
        assert!(
            reported[0].contains("9") && reported[0].contains("8"),
            "the message must name what was written and what is allowed: {reported:?}"
        );
    }

    // TC-69-2/TC-69-3 · the budget the person chose is the number enforced —
    // not the capacity above it, and not the default below it. Both halves
    // matter: accepting twelve proves the raise reaches the geometry, and
    // refusing the thirteenth proves the chosen number is what bounds it.
    // TP-CHROME-71: a raised budget is honoured up to itself and refused past
    // itself.
    #[test]
    fn a_raised_budget_is_honoured_up_to_itself_and_no_further() {
        let twelve = ShellBarsConfig {
            top: bar_with_budget(12, 12),
            ..Default::default()
        };
        assert!(
            shell_bar_config_problems(&twelve, true).is_empty(),
            "twelve sections under a budget of twelve is not a problem"
        );
        assert_eq!(
            addressable_sections(&twelve, RegionId::TopBar),
            12,
            "the sections must reach the derived chrome, not merely pass the checker"
        );

        let thirteen = ShellBarsConfig {
            top: bar_with_budget(12, 13),
            ..Default::default()
        };
        let reported = shell_bar_config_problems(&thirteen, true)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        assert_eq!(reported.len(), 1, "one cause: {reported:?}");
        assert!(
            reported[0].contains("13") && reported[0].contains("12"),
            "the refusal must name the budget the person chose: {reported:?}"
        );
    }

    // TC-69-4/TC-69-5 · a budget this build cannot honour is refused by name,
    // never clamped. A file saying forty beside a build doing sixteen is a file
    // its next reader will believe. Zero is refused for a different reason: a
    // bar allowed no parts is what `enabled = false` already means.
    // TP-CHROME-72: a budget outside the build's range is refused with its own
    // message rather than quietly clamped.
    #[test]
    fn a_budget_this_build_cannot_honour_is_refused_rather_than_clamped() {
        for requested in [0_u16, 17, 40] {
            let config = ShellBarsConfig {
                top: bar_with_budget(requested, 2),
                ..Default::default()
            };

            let reported = shell_bar_config_problems(&config, true)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            assert_eq!(reported.len(), 1, "one cause for {requested}: {reported:?}");
            assert!(
                reported[0].contains("max_sections")
                    && reported[0].contains(&requested.to_string())
                    && reported[0].contains("16"),
                "the message must name the field, the value and the bound: {reported:?}"
            );
            assert_eq!(
                addressable_sections(&config, RegionId::TopBar),
                0,
                "a refused budget leaves the bar undivided rather than silently clamped"
            );
        }
    }

    // TC-69-7 · one edge's budget is that edge's. A shared number would make
    // raising the toolbar's ceiling silently raise the status strip's too.
    // TP-CHROME-73: the section budget is per edge.
    #[test]
    fn each_edge_carries_its_own_budget() {
        let config = ShellBarsConfig {
            top: bar_with_budget(12, 12),
            bottom: bar_with_sections((0..9).map(|_| plain_section("fill")).collect()),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        assert_eq!(
            reported.len(),
            1,
            "only the bottom bar is over its own budget: {reported:?}"
        );
        assert!(
            reported[0].contains("bottom"),
            "the raised top budget must not leak downward: {reported:?}"
        );
        assert_eq!(
            addressable_sections(&config, RegionId::TopBar),
            12,
            "the top bar keeps all twelve"
        );
    }

    // TC-67-1/TC-67-4 · the presentation the person wrote survives the
    // derivation, and its absence stays absent. The second half is the
    // regression gate for every config file written before this field existed:
    // those sections must keep meaning exactly what they meant.
    // TP-CHROME-57: a section carries the secondary presentation it was written
    // with, and carries none when none was written.
    #[test]
    fn a_section_carries_the_secondary_presentation_it_was_written_with() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                secondary_section("popup", "tab"),
                section_with_action("fill", "popup", &["btop"]),
            ]),
            ..Default::default()
        };

        let actions = ShellBarChrome::from_config(&config, true);

        assert_eq!(
            popup_secondary(&actions, RegionId::TopBar, 0),
            Some(Some(SecondaryPresentation::Tab)),
            "the presentation must arrive as itself, not be re-derived downstream"
        );
        assert_eq!(
            popup_secondary(&actions, RegionId::TopBar, 1),
            Some(None),
            "a section written before this field existed must still mean what it meant"
        );
        assert!(
            shell_bar_config_problems(&config, true).is_empty(),
            "neither spelling is a problem worth reporting"
        );
    }

    // TC-66-1 · the id arrives exactly as it was written. Neither trimmed nor
    // case-folded: `find_plugin_action` resolves this string against every
    // installed manifest, so quietly reshaping it here could land on a
    // different plugin's action than the one the file names. Verbatim is a
    // behaviour, not an accident of the current implementation.
    // TP-CHROME-75: a bar section can name a plugin action, and the id it was
    // written with is the id that arrives.
    #[test]
    fn a_section_carries_the_plugin_action_id_it_was_written_with() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                plugin_section("jt.command-palette.open"),
                plugin_section("toggle"),
                section_with_action("fill", "popup", &["btop"]),
            ]),
            ..Default::default()
        };

        let actions = ShellBarChrome::from_config(&config, true);

        assert_eq!(
            plugin_action(&actions, RegionId::TopBar, 0).as_deref(),
            Some("jt.command-palette.open"),
            "the qualified spelling must survive derivation untouched"
        );
        assert_eq!(
            plugin_action(&actions, RegionId::TopBar, 1).as_deref(),
            Some("toggle"),
            "the short spelling resolves downstream, where ambiguity is reported \
             by name, so this layer has no business forbidding it"
        );
        assert_eq!(
            popup_argv(&actions, RegionId::TopBar, 2).as_deref(),
            Some(["btop".to_string()].as_slice()),
            "a popup section beside a plugin section keeps its own answer"
        );
        assert!(
            shell_bar_config_problems(&config, true).is_empty(),
            "neither spelling is a problem worth reporting"
        );
    }

    // TC-66-2 / TC-66-3 · an action that names nothing to invoke can never do
    // anything, and an icon that dies silently under the finger is the worst
    // outcome this surface has. Whitespace counts as nothing for the same
    // reason the popup arm already refuses an all-blank argv: disk is untrusted
    // input (CL1) and the cheapest place to say so is while reading the file.
    // TP-CHROME-76: a plugin action that names no action id is refused by name.
    #[test]
    fn a_plugin_action_without_an_id_is_refused() {
        for spelling in ["", "   ", "\t"] {
            let config = ShellBarsConfig {
                top: bar_with_sections(vec![
                    plugin_section(spelling),
                    section_with_action("fill", "popup", &["htop"]),
                ]),
                ..Default::default()
            };

            let reported = shell_bar_config_problems(&config, true)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            assert_eq!(
                reported.len(),
                1,
                "exactly one complaint, naming the field that is wrong: {reported:?}"
            );
            assert!(
                reported[0].contains("action.command") && reported[0].contains("sections[0]"),
                "the message must name the field and the index: {reported:?}"
            );

            let actions = ShellBarChrome::from_config(&config, true);
            assert_eq!(
                actions.action_for(RegionId::TopBar, 0),
                Some(&SectionAction::None),
                "the refused section stops answering rather than answering wrongly"
            );
            assert_eq!(
                popup_argv(&actions, RegionId::TopBar, 1).as_deref(),
                Some(["htop".to_string()].as_slice()),
                "its neighbour keeps both its index and its command"
            );
        }
    }

    // TC-66-4 / TC-66-5 · a field that can never be read is a lie the next
    // person believes. This is the same shape `PopupSizeWithoutPopup` already
    // names, one action kind along: a popup command or a popup size left behind
    // by a half-finished edit. The message carries WHICH field, because a
    // complaint that names the wrong line sends somebody to fix the wrong line.
    // TP-CHROME-77: a plugin action carrying popup-only fields is refused, and
    // the refusal names the field that does not belong.
    #[test]
    fn a_plugin_action_carrying_popup_only_fields_is_refused_by_field() {
        // One leftover popup field, written onto a section that is about to be
        // refused for carrying it. Named rather than spelled inline because
        // clippy counts the tuple-of-function-pointer as a complex type, and it
        // is right that the reader should meet a name instead.
        type LeaveBehind = fn(&mut ShellBarSectionConfig);

        let cases: [(&str, LeaveBehind); 3] = [
            ("action.argv", |section| {
                section.action.argv = vec!["btop".to_string()];
            }),
            ("action.width", |section| {
                section.action.width =
                    Some(crate::popup_size::PopupSize::parse_cli("80%").expect("width fixture"));
            }),
            ("action.height", |section| {
                section.action.height =
                    Some(crate::popup_size::PopupSize::parse_cli("60%").expect("height fixture"));
            }),
        ];

        for (field, apply) in cases {
            let mut section = plugin_section("jt.command-palette.open");
            apply(&mut section);
            let config = ShellBarsConfig {
                top: bar_with_sections(vec![section]),
                ..Default::default()
            };

            let reported = shell_bar_config_problems(&config, true)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            assert_eq!(
                reported.len(),
                1,
                "one field is wrong, so one complaint: {reported:?}"
            );
            assert!(
                reported[0].contains(field) && reported[0].contains("sections[0]"),
                "the message must name {field} specifically: {reported:?}"
            );
        }
    }

    // TC-66-6 · the bar does not open what a plugin action opens — the
    // manifest's own pane placement does. Offering to re-present it in a tab is
    // a promise this surface cannot keep, and a promise nobody can keep is
    // worse than a missing feature: the person presses, nothing happens, and
    // the gesture looks broken rather than unimplemented.
    // TP-CHROME-78: a plugin action asking for a second presentation is refused.
    #[test]
    fn a_plugin_action_that_asks_for_a_second_presentation_is_refused() {
        let mut section = plugin_section("jt.command-palette.open");
        section.action.secondary = "tab".to_string();
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        assert_eq!(reported.len(), 1, "one complaint: {reported:?}");
        assert!(
            reported[0].contains("action.secondary") && reported[0].contains("sections[0]"),
            "the message must name the field that cannot be honoured: {reported:?}"
        );
    }

    // TC-66-7 / TC-66-8 · the two mirrors of the same half-finished edit: a
    // plugin command left on a popup action, and a plugin command left with no
    // action at all. Without both directions one of them stays silent, and a
    // silent leftover is exactly what the reader trusts.
    // TP-CHROME-79: a plugin command on an action that is not a plugin action
    // is refused, in both directions.
    #[test]
    fn a_plugin_command_on_the_wrong_action_is_refused_in_both_directions() {
        let mut on_popup = section_with_action("fill", "popup", &["btop"]);
        on_popup.action.command = "jt.command-palette.open".to_string();

        let mut on_nothing = plain_section("fill");
        on_nothing.action.command = "jt.command-palette.open".to_string();

        for (label, section) in [("popup", on_popup), ("no action", on_nothing)] {
            let config = ShellBarsConfig {
                top: bar_with_sections(vec![section]),
                ..Default::default()
            };

            let reported = shell_bar_config_problems(&config, true)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            assert_eq!(
                reported.len(),
                1,
                "{label}: one leftover field, one complaint: {reported:?}"
            );
            assert!(
                reported[0].contains("action.command") && reported[0].contains("sections[0]"),
                "{label}: the message must name the leftover field: {reported:?}"
            );
        }
    }

    // TC-66-9 · the blast radius of a refused ACTION is one section, while a
    // refused SIZING costs the whole division — measured at `sections_from_config`
    // (all-or-nothing, so a dropped section cannot renumber its neighbours) and
    // at `bar_section_chrome` (a refused action becomes `None`). This is not
    // free: one `?` moved a line up and the whole bar dies quietly, with every
    // other test still green.
    // TP-CHROME-80: a refused plugin action leaves the division and its
    // neighbours intact.
    #[test]
    fn a_refused_plugin_action_costs_only_its_own_section() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![
                plugin_section("jt.command-palette.open"),
                plugin_section(""),
                section_with_action("fill", "popup", &["htop"]),
            ]),
            ..Default::default()
        };

        let actions = ShellBarChrome::from_config(&config, true);

        assert_eq!(
            addressable_sections(&config, RegionId::TopBar),
            3,
            "the bar still divides — a refused action is not a refused division"
        );
        assert_eq!(
            plugin_action(&actions, RegionId::TopBar, 0).as_deref(),
            Some("jt.command-palette.open"),
            "the section before the bad one keeps its command"
        );
        assert_eq!(
            actions.action_for(RegionId::TopBar, 1),
            Some(&SectionAction::None),
            "only the refused section goes inert"
        );
        assert_eq!(
            popup_argv(&actions, RegionId::TopBar, 2).as_deref(),
            Some(["htop".to_string()].as_slice()),
            "the section after it keeps its index, which is what everything \
             downstream addresses it by"
        );
    }

    // TC-66-10 · an unreadable action kind must say what this build accepts.
    // The list is the discoverable half of a closed enum: without it the person
    // reads "expected popup" and concludes plugins are not supported at all.
    // TP-CHROME-81: the unknown-action-kind refusal names every kind this build
    // accepts.
    #[test]
    fn an_unknown_action_kind_names_the_kinds_this_build_accepts() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![section_with_action("fill", "plug-in", &["btop"])]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        assert_eq!(reported.len(), 1, "one complaint: {reported:?}");
        assert!(
            reported[0].contains("\"popup\"") && reported[0].contains("\"plugin\""),
            "the refusal must list both kinds, or the reader concludes the one \
             it omits does not exist: {reported:?}"
        );
    }

    // TC-67-2 · a near-miss is refused rather than quietly ignored. Disk is
    // untrusted input (CL1), and a section that silently stopped answering the
    // right press would look like the gesture is broken rather than like the
    // file has a typo. Case sensitivity is a decision, so it is pinned here.
    // TP-CHROME-58: a secondary presentation this build does not know is
    // refused by name, and costs only its own section.
    #[test]
    fn a_secondary_presentation_this_build_does_not_know_is_refused() {
        for spelling in ["TAB", " tab ", "window", "split"] {
            let config = ShellBarsConfig {
                top: bar_with_sections(vec![
                    secondary_section("popup", spelling),
                    section_with_action("fill", "popup", &["htop"]),
                ]),
                ..Default::default()
            };

            let reported = shell_bar_config_problems(&config, true)
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>();

            assert_eq!(
                reported.len(),
                1,
                "exactly one complaint, naming the field that is wrong: {reported:?}"
            );
            assert!(
                reported[0].contains("action.secondary")
                    && reported[0].contains(spelling)
                    && reported[0].contains("sections[0]"),
                "the message must name the field, the value and the index: {reported:?}"
            );

            // TC-67-5 · a refused action costs only its own section. Measured at
            // source.rs `bar_section_chrome`: sizing and policy problems leave a
            // bar undivided, a refused action does not.
            let actions = ShellBarChrome::from_config(&config, true);
            assert_eq!(
                actions.action_for(RegionId::TopBar, 0),
                Some(&SectionAction::None),
                "the refused section stops answering, rather than answering wrongly"
            );
            assert_eq!(
                popup_argv(&actions, RegionId::TopBar, 1).as_deref(),
                Some(["htop".to_string()].as_slice()),
                "its neighbour keeps both its index and its command"
            );
        }
    }

    // TC-67-3 · a presentation with nothing to present is the shape a
    // half-finished edit leaves behind — the command was removed and the
    // gesture stayed. The same reasoning, and the same treatment, as a popup
    // size with no popup.
    // TP-CHROME-59: a secondary presentation on a section with no command is
    // refused where it can be fixed, not discovered at the moment of a press.
    #[test]
    fn a_secondary_presentation_without_a_command_is_refused() {
        let config = ShellBarsConfig {
            top: bar_with_sections(vec![secondary_section("", "tab")]),
            ..Default::default()
        };

        let reported = shell_bar_config_problems(&config, true)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        assert_eq!(reported.len(), 1, "one cause, one complaint: {reported:?}");
        assert!(
            reported[0].contains("secondary presentation")
                && reported[0].contains("no command")
                && reported[0].contains("sections[0]"),
            "the message must say what is missing, not merely that something is: {reported:?}"
        );
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

        let actions = ShellBarChrome::from_config(&config, true);

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

        let actions = ShellBarChrome::from_config(&config, true);
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
        let actions = ShellBarChrome::from_config(&config, true);

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
        let actions = ShellBarChrome::from_config(&config, true);

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

        let reported = shell_bar_config_problems(&config, true)
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
        assert!(shell_bar_config_problems(&quiet, true).is_empty());
    }

    fn bordered_config_bar(size: u16) -> ShellBarConfig {
        ShellBarConfig {
            enabled: true,
            size,
            border: true,
            color: String::new(),
            gradient: Vec::new(),
            sections: Vec::new(),
            ..Default::default()
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
            ..Default::default()
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
        let bar = |sections: Vec<ShellBarSectionConfig>| ShellBarConfig {
            enabled: true,
            size: 1,
            border: false,
            color: String::new(),
            gradient: Vec::new(),
            sections,
            ..Default::default()
        };
        assert!(sections_from_config(&bar(configs), "top").is_empty());

        let good = vec![fixed_section(4), fixed_section(6)];
        assert_eq!(sections_from_config(&bar(good), "top").len(), 2);
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
            ..Default::default()
        }
    }

    // T-CFG-1 · a size nothing can draw is said out loud, with its edge named
    #[test]
    fn an_out_of_range_bar_size_is_reported_with_its_edge() {
        // Before this, the value parsed perfectly, the checker said "ok", and
        // the bar simply never appeared — sending the person to look at their
        // terminal rather than at the line they had just written.
        let problems = shell_bar_config_problems(&bars_with_top(enabled_bar(999, false)), true);
        assert_eq!(problems.len(), 1);
        let text = problems[0].to_string();
        assert!(text.contains("shell.bars.top.size"), "{text}");
        assert!(text.contains("999"), "{text}");
    }

    // T-CFG-2 · two different mistakes must not read as the same mistake
    #[test]
    fn a_bordered_bar_too_thin_reads_differently_from_one_out_of_range() {
        let thin = shell_bar_config_problems(&bars_with_top(enabled_bar(1, true)), true);
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
        assert!(shell_bar_config_problems(&bars_with_top(enabled_bar(1, false)), true).is_empty());
    }

    // T-CFG-3 · a section's complaint names which section it is
    #[test]
    fn an_unknown_section_kind_is_reported_with_its_index() {
        let mut bar = enabled_bar(3, true);
        bar.sections = vec![fixed_section(4), section_config("nonsense")];
        let problems = shell_bar_config_problems(&bars_with_top(bar), true);

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
        assert_eq!(
            shell_bar_config_problems(&bars_with_top(narrow), true).len(),
            1
        );

        let mut crowded = enabled_bar(3, true);
        crowded.sections = (0..=MAX_BAR_SECTIONS)
            .map(|_| section_config("fill"))
            .collect();
        let problems = shell_bar_config_problems(&bars_with_top(crowded), true);
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
        assert!(shell_bar_config_problems(&bars_with_top(bar), true).is_empty());

        // A disabled bar is not drawn either, but that is what was asked for.
        // Reporting it would bury the real complaints under noise.
        let mut disabled = enabled_bar(999, false);
        disabled.enabled = false;
        assert!(shell_bar_config_problems(&bars_with_top(disabled), true).is_empty());

        // And a bar nobody configured at all.
        assert!(shell_bar_config_problems(&ShellBarsConfig::default(), true).is_empty());
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
            let reported = shell_bar_config_problems(
                &bars_with_top(ShellBarConfig {
                    enabled: true,
                    size,
                    border,
                    color: String::new(),
                    gradient: Vec::new(),
                    sections: Vec::new(),
                    ..Default::default()
                }),
                true,
            )
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
