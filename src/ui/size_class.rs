//! Discrete size classes derived from the terminal viewport.
//!
//! Layout, overlay geometry and switcher density each used to answer "is this
//! small?" from their own raw width comparison. That held while width was the
//! only axis; it stopped holding once a phone in landscape — wide but only
//! fourteen rows tall — had to be told apart from a desktop terminal. This
//! module is the one place those thresholds live, and it stays pure: a `Rect`
//! and the configured mobile threshold go in, a class comes out. No state, no
//! IO, so every threshold decision is testable without a PTY or a frame.

use ratatui::layout::Rect;

/// Widest viewport that still counts as [`WidthClass::Tight`].
///
/// Measured against overlay geometry: at 40 columns a centred popup keeps 34
/// columns of interior, which is where a two-column (key + label) row stops
/// fitting without wrapping into the key column.
pub(crate) const TIGHT_MAX_WIDTH: u16 = 40;

/// Tallest viewport that still counts as [`HeightClass::Short`].
///
/// The mobile shell spends two rows on its header, and a toast plus a config
/// diagnostic can take two more. At sixteen rows that leaves twelve for the
/// terminal, so below this an optional row of chrome is a measurable loss.
pub(crate) const SHORT_MAX_HEIGHT: u16 = 16;

/// How much horizontal room the viewport has, in the terms layout cares about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WidthClass {
    /// No room for chrome beside content: a phone held upright. Overlays span
    /// the full width instead of floating.
    Tight,
    /// The mobile shell fits, but a desktop sidebar does not.
    Compact,
    /// Desktop: sidebar, tab bar and floating overlays all fit.
    Regular,
}

/// How much vertical room the viewport has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HeightClass {
    /// Rows are the scarce resource — a phone held sideways. Optional chrome
    /// costs more than it returns.
    Short,
    /// Enough rows that a row of chrome is not a meaningful loss.
    Regular,
}

/// The viewport's size expressed on both axes at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SizeClass {
    pub width: WidthClass,
    pub height: HeightClass,
}

impl SizeClass {
    /// Classify `area` against the user's configured mobile width threshold.
    ///
    /// A zero-width or zero-height viewport classifies as `Regular` on that
    /// axis. That is deliberate rather than incidental: `is_mobile_width` has
    /// always answered "not mobile" for a zero width, and a degenerate
    /// viewport draws nothing anyway, so shrinking chrome for it would only
    /// add a branch nobody can observe.
    pub(crate) fn of(area: Rect, mobile_threshold: u16) -> Self {
        Self {
            width: width_class(area.width, mobile_threshold),
            height: height_class(area.height),
        }
    }

    /// Classify `area` for decisions that are about the viewport's physical
    /// room rather than about which shell to draw.
    ///
    /// Overlay geometry is the case this exists for. Whether a popup can
    /// afford a two-column margin depends on how many columns there are, not
    /// on where the user drew the line between the mobile and desktop shells:
    /// someone who raises `mobile_width_threshold` to 100 to get the mobile
    /// shell on a tablet has said nothing about margins, and a 90-column
    /// popup can still spare its margin.
    pub(crate) fn of_viewport(area: Rect) -> Self {
        Self::of(area, crate::config::DEFAULT_MOBILE_WIDTH_THRESHOLD)
    }

    /// Whether this viewport takes the mobile shell (header + full-width
    /// terminal) rather than the desktop shell.
    pub(crate) fn is_mobile_shell(self) -> bool {
        matches!(self.width, WidthClass::Tight | WidthClass::Compact)
    }
}

/// The widest viewport that is still `Tight`, given the configured threshold.
///
/// Derived rather than configured: a user who sets `mobile_width_threshold`
/// below 40 would otherwise create a range where `Tight` swallows `Compact`
/// and a viewport could be "too narrow for the mobile shell" — a state with no
/// meaning. Deriving keeps the two thresholds ordered by construction.
pub(crate) fn tight_max_width(mobile_threshold: u16) -> u16 {
    TIGHT_MAX_WIDTH.min(mobile_threshold)
}

fn width_class(width: u16, mobile_threshold: u16) -> WidthClass {
    if width == 0 || width > mobile_threshold {
        return WidthClass::Regular;
    }
    if width <= tight_max_width(mobile_threshold) {
        WidthClass::Tight
    } else {
        WidthClass::Compact
    }
}

fn height_class(height: u16) -> HeightClass {
    if height == 0 || height > SHORT_MAX_HEIGHT {
        HeightClass::Regular
    } else {
        HeightClass::Short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFAULT_THRESHOLD: u16 = 64;

    fn width_of(width: u16) -> WidthClass {
        SizeClass::of(Rect::new(0, 0, width, 24), DEFAULT_THRESHOLD).width
    }

    fn height_of(height: u16) -> HeightClass {
        SizeClass::of(Rect::new(0, 0, 80, height), DEFAULT_THRESHOLD).height
    }

    // TP-MOB-01: the width boundaries are exact, so a viewport one column
    // either side of a threshold lands in the class the design assigned it.
    #[test]
    fn size_class_width_boundaries() {
        assert_eq!(width_of(1), WidthClass::Tight);
        assert_eq!(width_of(TIGHT_MAX_WIDTH), WidthClass::Tight);
        assert_eq!(width_of(TIGHT_MAX_WIDTH + 1), WidthClass::Compact);
        assert_eq!(width_of(DEFAULT_THRESHOLD), WidthClass::Compact);
        assert_eq!(width_of(DEFAULT_THRESHOLD + 1), WidthClass::Regular);
    }

    // TP-MOB-02: the height boundary is exact for the same reason.
    #[test]
    fn size_class_height_boundaries() {
        assert_eq!(height_of(1), HeightClass::Short);
        assert_eq!(height_of(SHORT_MAX_HEIGHT), HeightClass::Short);
        assert_eq!(height_of(SHORT_MAX_HEIGHT + 1), HeightClass::Regular);
    }

    // TP-MOB-03: lowering the configured mobile threshold below the tight
    // ceiling cannot produce a width that is "too narrow for mobile" — the
    // tight ceiling follows the threshold down.
    #[test]
    fn tight_never_exceeds_the_mobile_threshold() {
        let threshold = 30;
        assert_eq!(tight_max_width(threshold), threshold);
        let class = |width: u16| SizeClass::of(Rect::new(0, 0, width, 24), threshold).width;
        assert_eq!(class(threshold), WidthClass::Tight);
        assert_eq!(class(threshold + 1), WidthClass::Regular);
        assert_eq!(tight_max_width(200), TIGHT_MAX_WIDTH);
    }

    // TP-MOB-04: a degenerate viewport keeps answering the way
    // `is_mobile_width` always answered it, so no caller changes behavior on
    // an area it never had to draw.
    #[test]
    fn zero_area_is_not_mobile() {
        let zero_width = SizeClass::of(Rect::new(0, 0, 0, 24), DEFAULT_THRESHOLD);
        assert_eq!(zero_width.width, WidthClass::Regular);
        assert!(!zero_width.is_mobile_shell());

        let zero_height = SizeClass::of(Rect::new(0, 0, 80, 0), DEFAULT_THRESHOLD);
        assert_eq!(zero_height.height, HeightClass::Regular);
    }

    /// The predicate `is_mobile_shell` replaced, spelled out here so the
    /// equivalence test compares against a fixed rule rather than against a
    /// function that now delegates to the code under test.
    fn legacy_is_mobile_width(area: Rect, threshold: u16) -> bool {
        area.width > 0 && area.width <= threshold
    }

    // TP-MOB-05: the mobile shell answer stays identical to the width-only
    // predicate it replaces, for every width and every threshold in the
    // neighbourhood. This is the whole safety argument for routing the shell
    // decision through a new type.
    #[test]
    fn mobile_shell_matches_the_width_predicate_it_replaces() {
        for threshold in [1u16, 20, 30, 40, 41, DEFAULT_THRESHOLD, 100] {
            for width in 0..=120u16 {
                let area = Rect::new(0, 0, width, 24);
                assert_eq!(
                    SizeClass::of(area, threshold).is_mobile_shell(),
                    legacy_is_mobile_width(area, threshold),
                    "width {width} at threshold {threshold} must classify the same \
                     as the predicate it replaces"
                );
            }
        }
    }
}
