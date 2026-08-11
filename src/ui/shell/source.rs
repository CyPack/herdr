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

use crate::config::{ShellBarConfig, ShellBarsConfig};

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

/// How thick one edge's strip is, or that there is none.
///
/// The cache keys on this value directly rather than on a digest of it: the
/// whole point of the geometry key is that two different screens can never
/// answer to the same identity, and a hash trades that certainty for brevity
/// nobody needs at four small fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub(crate) struct BarTrack {
    cells: Option<u16>,
}

impl BarTrack {
    pub(crate) const NONE: Self = Self { cells: None };

    /// A strip of exactly this many cells. Bounds live in [`BarTrack::from_config`],
    /// where the number arrives from somebody's config file; a caller that
    /// already holds a checked number does not re-check it here.
    pub(crate) const fn of(cells: u16) -> Self {
        Self { cells: Some(cells) }
    }

    /// Read one edge, refusing rather than repairing.
    ///
    /// A disabled bar and a bar with an impossible size are the same answer —
    /// no strip — but only the second one is worth saying out loud, so the
    /// caller gets told which edge it was.
    fn from_config(config: &ShellBarConfig, edge: &'static str) -> Self {
        if !config.enabled {
            return Self::NONE;
        }
        if config.size == 0 || config.size > MAX_BAR_CELLS {
            tracing::warn!(
                edge,
                size = config.size,
                max = MAX_BAR_CELLS,
                "shell bar size is out of range; the bar is not drawn"
            );
            return Self::NONE;
        }
        Self::of(config.size)
    }

    const fn enabled(self) -> bool {
        self.cells.is_some()
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

    const fn any_enabled(self) -> bool {
        self.top.enabled() || self.bottom.enabled() || self.left.enabled() || self.right.enabled()
    }
}

/// Derive the desktop shell tree from what the user asked for.
///
/// Asking for nothing with no bars is today's production request and yields
/// exactly today's tree. A requested template owns the whole composition, so
/// bars are not composed onto it — a template already names every region it
/// wants, and adding an edge it already owns would produce a duplicate region
/// and fail the whole tree rather than the one bar.
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
        ShellBarConfig { enabled, size }
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
}
