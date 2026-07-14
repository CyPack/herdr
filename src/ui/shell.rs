//! Named-region shell layout — the outer composition tree around the pane
//! tree.
//!
//! This is the additive "outer shell" of herdr's flexible-composition work: a
//! serializable tree of *named regions* (a left sidebar, the center content, and
//! reserved seams for a top toolbar / right rail / bottom bar). Today it exists
//! only to reproduce the current outer split exactly — [`ShellLayout::default`]
//! encodes today's `sidebar | main` layout, so introducing it changes nothing on
//! screen. Later steps let a region host a swappable component (S3+), resize its
//! divider (S6), and persist a customized tree (S6).
//!
//! Pure TUI presentation (AGENTS.md runtime/client boundary): none of these
//! types are shared runtime facts, and none appear in `protocol`/`api::schema`.
//! `compute_regions` is a pure geometry function — it reads a layout and an
//! area and returns rects, matching herdr's `compute_view` (geometry) / `render`
//! (pure draw) split.

use std::collections::HashMap;

use ratatui::layout::{Constraint, Layout, Rect};
use serde::{Deserialize, Serialize};

/// A named region of the outer shell (the chrome around the pane/content area).
///
/// Only `LeftPanel` + `CenterContent` are populated by [`ShellLayout::default`]
/// today — together they reproduce herdr's current `sidebar | main` outer split.
/// `TopBar`, `RightPanel`, and `BottomBar` are reserved seams for future
/// composition (a top toolbar, a right rail, a bottom bar); they are not in the
/// default tree yet, so [`RegionRects::get`] returns an empty rect for them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum RegionId {
    TopBar,
    LeftPanel,
    CenterContent,
    RightPanel,
    BottomBar,
}

/// How much space a shell child claims along its parent split's axis.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum RegionSize {
    /// A cell count resolved at compute time from live app state (today only the
    /// sidebar, whose width depends on collapse mode and clamps). Kept *out* of
    /// the tree so the existing `sidebar_width` stays the single source of truth.
    /// Maps to ratatui `Constraint::Length`.
    Dynamic,
    /// Takes the remaining space (ratatui `Constraint::Min(1)`).
    Fill,
}

/// A node in the outer-shell layout tree: either a leaf hosting one region, or
/// an axis-aligned split of children.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ShellNode {
    /// A leaf hosting exactly one named region.
    Slot { region: RegionId },
    /// An axis-aligned split; children are laid out first-to-last along `direction`.
    Split {
        direction: ShellDirection,
        children: Vec<ShellChild>,
    },
}

/// One child of a [`ShellNode::Split`]: its size constraint plus its subtree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ShellChild {
    pub size: RegionSize,
    pub node: ShellNode,
}

/// Split axis. Mirrors ratatui's `Direction` with a stable serde representation
/// (the same reason `persist::snapshot::DirectionSnapshot` exists).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ShellDirection {
    Horizontal,
    Vertical,
}

/// The outer-shell layout tree. [`ShellLayout::default`] reproduces herdr's
/// current outer split exactly, so introducing it is behavior-identical.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ShellLayout {
    root: ShellNode,
}

impl Default for ShellLayout {
    /// Today's outer shell: a horizontal split of the frame into the left
    /// sidebar (dynamic width) and the center content (remaining space) — i.e.
    /// exactly `Layout::horizontal([Length(sidebar_w), Min(1)])`.
    fn default() -> Self {
        Self {
            root: ShellNode::Split {
                direction: ShellDirection::Horizontal,
                children: vec![
                    ShellChild {
                        size: RegionSize::Dynamic,
                        node: ShellNode::Slot {
                            region: RegionId::LeftPanel,
                        },
                    },
                    ShellChild {
                        size: RegionSize::Fill,
                        node: ShellNode::Slot {
                            region: RegionId::CenterContent,
                        },
                    },
                ],
            },
        }
    }
}

/// The resolved rect of each region present in a computed [`ShellLayout`].
/// Regions absent from the layout tree return an empty rect from [`Self::get`]
/// (total — never panics), so callers never need to unwrap.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RegionRects {
    rects: HashMap<RegionId, Rect>,
}

impl RegionRects {
    /// The rect assigned to `region`, or `Rect::default()` (empty) if the region
    /// is not part of the computed layout.
    pub fn get(&self, region: RegionId) -> Rect {
        self.rects.get(&region).copied().unwrap_or_default()
    }

    fn insert(&mut self, region: RegionId, rect: Rect) {
        self.rects.insert(region, rect);
    }
}

impl ShellLayout {
    /// Resolve every region's rect within `area`. `resolve_dynamic` supplies the
    /// cell count for [`RegionSize::Dynamic`] children (today: the runtime
    /// sidebar width for `LeftPanel`). Pure and deterministic.
    pub fn compute_regions(
        &self,
        area: Rect,
        resolve_dynamic: impl Fn(RegionId) -> u16,
    ) -> RegionRects {
        let mut rects = RegionRects::default();
        layout_node(&self.root, area, &resolve_dynamic, &mut rects);
        rects
    }
}

fn layout_node(
    node: &ShellNode,
    area: Rect,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
    out: &mut RegionRects,
) {
    match node {
        ShellNode::Slot { region } => out.insert(*region, area),
        ShellNode::Split {
            direction,
            children,
        } => {
            let constraints: Vec<Constraint> = children
                .iter()
                .map(|child| child_constraint(child, resolve_dynamic))
                .collect();
            let layout = match direction {
                ShellDirection::Horizontal => Layout::horizontal(constraints),
                ShellDirection::Vertical => Layout::vertical(constraints),
            };
            let segments = layout.split(area);
            for (child, segment) in children.iter().zip(segments.iter()) {
                layout_node(&child.node, *segment, resolve_dynamic, out);
            }
        }
    }
}

fn child_constraint(child: &ShellChild, resolve_dynamic: &impl Fn(RegionId) -> u16) -> Constraint {
    match child.size {
        RegionSize::Fill => Constraint::Min(1),
        RegionSize::Dynamic => {
            // A dynamic size is driven by the child's representative region (its
            // first slot). Today that is only the sidebar (LeftPanel).
            let width = child
                .node
                .primary_region()
                .map(resolve_dynamic)
                .unwrap_or(0);
            Constraint::Length(width)
        }
    }
}

impl ShellNode {
    /// The representative region of this subtree (its first slot in tree order),
    /// used to resolve a `Dynamic` size. `None` only for an empty split.
    fn primary_region(&self) -> Option<RegionId> {
        match self {
            ShellNode::Slot { region } => Some(*region),
            ShellNode::Split { children, .. } => children
                .iter()
                .find_map(|child| child.node.primary_region()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TP-S2.3 (equivalence, the core proof): the default shell reproduces the
    /// old inline outer split `Layout::horizontal([Length(sidebar_w), Min(1)])`
    /// EXACTLY, across representative and degenerate areas and sidebar widths.
    #[test]
    fn default_matches_legacy_outer_split_exactly() {
        let areas = [
            (80u16, 24u16),
            (120, 40),
            (200, 50),
            (1, 1),
            (0, 0),
            (4, 10),
            (2, 3),
            (26, 1),
        ];
        for &(w, h) in &areas {
            // Cover collapsed (0 / COLLAPSED_WIDTH), typical, clamp bounds, and
            // the pathological "sidebar wider than frame" case.
            for &sidebar_w in &[0u16, 1, 4, 26, 40, w] {
                let area = Rect::new(0, 0, w, h);
                let [legacy_sidebar, legacy_main] =
                    Layout::horizontal([Constraint::Length(sidebar_w), Constraint::Min(1)])
                        .areas(area);
                let regions = ShellLayout::default().compute_regions(area, |region| {
                    if region == RegionId::LeftPanel {
                        sidebar_w
                    } else {
                        0
                    }
                });
                assert_eq!(
                    regions.get(RegionId::LeftPanel),
                    legacy_sidebar,
                    "LeftPanel mismatch at w={w} h={h} sidebar_w={sidebar_w}"
                );
                assert_eq!(
                    regions.get(RegionId::CenterContent),
                    legacy_main,
                    "CenterContent mismatch at w={w} h={h} sidebar_w={sidebar_w}"
                );
            }
        }
    }

    /// A region absent from the tree yields an empty rect — `get` is total and
    /// never panics (no-happy-path: callers must not need to unwrap).
    #[test]
    fn absent_region_returns_empty_rect() {
        let regions = ShellLayout::default().compute_regions(Rect::new(0, 0, 80, 24), |_| 20);
        assert_eq!(regions.get(RegionId::TopBar), Rect::default());
        assert_eq!(regions.get(RegionId::RightPanel), Rect::default());
        assert_eq!(regions.get(RegionId::BottomBar), Rect::default());
    }

    /// TP-S2.5 (serde round-trip): the default tree and a future-shaped nested
    /// tree both survive serialize → deserialize unchanged (the type is ready to
    /// be persisted in S6 without further migration work).
    #[test]
    fn serde_round_trip_default_and_nested() {
        let default = ShellLayout::default();
        let json = serde_json::to_string(&default).unwrap();
        let restored: ShellLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(default, restored);

        let nested = nested_fixture();
        let json = serde_json::to_string(&nested).unwrap();
        let restored: ShellLayout = serde_json::from_str(&json).unwrap();
        assert_eq!(nested, restored);
    }

    /// The tree walker generalizes beyond the 2-slot default: a nested tree
    /// (sidebar | (center over bottom bar)) lays out every region correctly and
    /// a degenerate zero area does not panic.
    #[test]
    fn nested_tree_lays_out_all_regions_and_survives_degenerate_area() {
        let nested = nested_fixture();
        let regions = nested.compute_regions(Rect::new(0, 0, 100, 30), |region| match region {
            RegionId::LeftPanel => 10,
            RegionId::BottomBar => 1,
            _ => 0,
        });

        let left = regions.get(RegionId::LeftPanel);
        let center = regions.get(RegionId::CenterContent);
        let bottom = regions.get(RegionId::BottomBar);

        assert_eq!(left.x, 0);
        assert_eq!(left.width, 10);
        // Center starts immediately right of the fixed-width sidebar.
        assert_eq!(center.x, 10);
        assert_eq!(center.width, 90);
        // Bottom bar is stacked directly under the center, one row tall.
        assert_eq!(bottom.x, 10);
        assert_eq!(bottom.height, 1);
        assert_eq!(bottom.y, center.y + center.height);

        // Degenerate area: must not panic, and every rect collapses to zero size.
        let empty = nested.compute_regions(Rect::new(0, 0, 0, 0), |_| 10);
        assert_eq!(empty.get(RegionId::LeftPanel).width, 0);
        assert_eq!(empty.get(RegionId::CenterContent).width, 0);
        assert_eq!(empty.get(RegionId::BottomBar).height, 0);
    }

    /// sidebar | (center / bottom bar) | right rail — exercises horizontal and
    /// vertical splits, a nested split, and a dynamic size on a Split child.
    fn nested_fixture() -> ShellLayout {
        ShellLayout {
            root: ShellNode::Split {
                direction: ShellDirection::Horizontal,
                children: vec![
                    ShellChild {
                        size: RegionSize::Dynamic,
                        node: ShellNode::Slot {
                            region: RegionId::LeftPanel,
                        },
                    },
                    ShellChild {
                        size: RegionSize::Fill,
                        node: ShellNode::Split {
                            direction: ShellDirection::Vertical,
                            children: vec![
                                ShellChild {
                                    size: RegionSize::Fill,
                                    node: ShellNode::Slot {
                                        region: RegionId::CenterContent,
                                    },
                                },
                                ShellChild {
                                    size: RegionSize::Dynamic,
                                    node: ShellNode::Slot {
                                        region: RegionId::BottomBar,
                                    },
                                },
                            ],
                        },
                    },
                ],
            },
        }
    }
}
