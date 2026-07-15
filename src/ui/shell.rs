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

use ratatui::layout::{Constraint, Layout, Rect};

mod model;

pub(crate) use model::{
    RegionId, RegionRects, RegionSize, ShellChild, ShellDirection, ShellLayout, ShellNode,
};

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
                            region: RegionId::WorkspaceStage,
                        },
                    },
                ],
            },
        }
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

    #[test]
    fn shell_layout_places_dock_sidebar_stage_without_overlap() {
        let serialized = r#"
        {
          "root": {
            "Split": {
              "direction": "Horizontal",
              "children": [
                {
                  "size": "Dynamic",
                  "node": { "Slot": { "region": "AppDock" } }
                },
                {
                  "size": "Dynamic",
                  "node": { "Slot": { "region": "LeftPanel" } }
                },
                {
                  "size": "Fill",
                  "node": { "Slot": { "region": "WorkspaceStage" } }
                }
              ]
            }
          }
        }
        "#;
        let layout = serde_json::from_str::<ShellLayout>(serialized);
        assert!(
            layout.is_ok(),
            "SF2 named shell regions should deserialize: {layout:?}"
        );
        let layout = layout.expect("checked named shell layout");
        let area = Rect::new(0, 0, 120, 40);
        let regions = layout.compute_regions(area, |region| match format!("{region:?}").as_str() {
            "AppDock" => 5,
            "LeftPanel" => 26,
            _ => 0,
        });
        let rect_for = |name: &str| {
            regions
                .rects
                .iter()
                .find_map(|(region, rect)| (format!("{region:?}") == name).then_some(*rect))
                .unwrap_or_else(|| panic!("missing shell region {name}"))
        };

        assert_eq!(regions.rects.len(), 3);
        let dock = rect_for("AppDock");
        let sidebar = rect_for("LeftPanel");
        let stage = rect_for("WorkspaceStage");
        assert_eq!(dock, Rect::new(0, 0, 5, 40));
        assert_eq!(sidebar, Rect::new(5, 0, 26, 40));
        assert_eq!(stage, Rect::new(31, 0, 89, 40));

        for (left, right) in [(dock, sidebar), (dock, stage), (sidebar, stage)] {
            assert!(left.intersection(right).is_empty());
        }
        assert_eq!(dock.union(sidebar).union(stage), area);
    }

    #[test]
    fn shell_rejects_depth_above_four() {
        let mut node = serialized_slot("WorkspaceStage");
        for _ in 0..5 {
            node = serialized_split(vec![serialized_child(node)]);
        }

        assert_serialized_shell_rejected(node, "DepthExceeded");
    }

    #[test]
    fn shell_rejects_more_than_eight_split_children() {
        let children = (0..9)
            .map(|_| serialized_child(serialized_slot("WorkspaceStage")))
            .collect();

        assert_serialized_shell_rejected(serialized_split(children), "ChildrenExceeded");
    }

    #[test]
    fn shell_rejects_more_than_sixty_four_visible_leaves() {
        let sixty_four = serialized_split(
            (0..8)
                .map(|_| {
                    serialized_child(serialized_split(
                        (0..8)
                            .map(|_| serialized_child(serialized_slot("WorkspaceStage")))
                            .collect(),
                    ))
                })
                .collect(),
        );
        let root = serialized_split(vec![
            serialized_child(sixty_four),
            serialized_child(serialized_slot("WorkspaceStage")),
        ]);

        assert_serialized_shell_rejected(root, "VisibleLeavesExceeded");
    }

    #[test]
    fn shell_rejects_more_than_one_hundred_twenty_eight_serialized_nodes() {
        let root = serialized_split(
            (0..8)
                .map(|_| {
                    serialized_child(serialized_split(
                        (0..8)
                            .map(|_| {
                                serialized_child(serialized_split(vec![serialized_child(
                                    serialized_slot("WorkspaceStage"),
                                )]))
                            })
                            .collect(),
                    ))
                })
                .collect(),
        );

        assert_serialized_shell_rejected(root, "SerializedNodesExceeded");
    }

    #[test]
    fn shell_rejects_duplicate_outer_region() {
        let root = serialized_split(vec![
            serialized_child(serialized_slot("LeftPanel")),
            serialized_child(serialized_slot("LeftPanel")),
            serialized_child(serialized_slot("WorkspaceStage")),
        ]);

        assert_serialized_shell_rejected(root, "DuplicateRegion");
    }

    #[test]
    fn shell_rejects_collapsed_or_missing_stage() {
        assert_serialized_shell_rejected(serialized_slot("LeftPanel"), "MissingWorkspaceStage");
    }

    fn serialized_slot(region: &str) -> serde_json::Value {
        serde_json::json!({ "Slot": { "region": region } })
    }

    fn serialized_child(node: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "size": "Fill", "node": node })
    }

    fn serialized_split(children: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({
            "Split": {
                "direction": "Horizontal",
                "children": children,
            }
        })
    }

    fn assert_serialized_shell_rejected(root: serde_json::Value, expected: &str) {
        let serialized = serde_json::json!({ "root": root }).to_string();
        let result = serde_json::from_str::<ShellLayout>(&serialized);
        assert!(
            result.is_err(),
            "shell should reject {expected}, but accepted {result:?}"
        );
        let error = result.expect_err("checked invalid shell").to_string();
        assert!(
            error.contains(expected),
            "shell rejected the fixture for the wrong reason: expected {expected}, got {error}"
        );
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
