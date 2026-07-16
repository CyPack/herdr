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

// SF3.1 lands the pure reducer before the input adapter consumes it in the
// next test-first slice.
#[allow(dead_code)]
mod interaction;
mod layout;
mod model;
mod template;
mod view;

pub(crate) use layout::ResponsiveDegradation;
pub(crate) use model::{
    RegionId, RegionRects, RegionSize, ShellChild, ShellDirection, ShellLayout, ShellNode,
};
pub(crate) use view::{compute_empty_shell_view, compute_shell_view, ShellGeometryKey, ShellView};

impl Default for ShellLayout {
    /// Today's outer shell: a horizontal split of the frame into the left
    /// sidebar (dynamic width) and the center content (remaining space) — i.e.
    /// exactly `Layout::horizontal([Length(sidebar_w), Min(1)])`.
    fn default() -> Self {
        Self::from_legacy_root(ShellNode::Split {
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
        })
    }
}

impl ShellLayout {
    /// Resolve every region's rect within `area`. `resolve_dynamic` supplies the
    /// cell count for [`RegionSize::Dynamic`] children (today: the runtime
    /// sidebar width for `LeftPanel`). Pure and deterministic.
    #[cfg(test)]
    pub fn compute_regions(
        &self,
        area: Rect,
        resolve_dynamic: impl Fn(RegionId) -> u16,
    ) -> RegionRects {
        self.compute_projection(area, &resolve_dynamic).0
    }

    fn compute_projection(
        &self,
        area: Rect,
        resolve_dynamic: &impl Fn(RegionId) -> u16,
    ) -> (RegionRects, ResponsiveDegradation) {
        if !self.tracks.is_empty() {
            let Some(solved) = self.solve_tracked(area, &resolve_dynamic) else {
                return (RegionRects::default(), ResponsiveDegradation::TooSmall);
            };
            return solved.into_parts();
        }

        let mut rects = RegionRects::default();
        layout_node(&self.root, area, resolve_dynamic, &mut rects);
        (rects, ResponsiveDegradation::Workspace)
    }

    fn solve_tracked(
        &self,
        area: Rect,
        resolve_dynamic: &impl Fn(RegionId) -> u16,
    ) -> Option<layout::SolvedShellLayout> {
        let validated = self.clone().validate().ok()?;
        Some(layout::solve(&validated, area, resolve_dynamic))
    }

    #[cfg(test)]
    fn solve_tracked_for_test(
        &self,
        area: Rect,
        resolve_dynamic: &impl Fn(RegionId) -> u16,
    ) -> layout::SolvedShellLayout {
        self.solve_tracked(area, resolve_dynamic)
            .expect("test fixture is a validated tracked shell")
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

        assert_serialized_shell_document_rejected(
            serde_json::json!({
                "root": serialized_slot("WorkspaceStage"),
                "tracks": {
                    "WorkspaceStage": { "Collapsed": { "restore": 20 } }
                }
            }),
            "CollapsedWorkspaceStage",
        );
    }

    #[test]
    fn shell_rejects_more_than_thirty_two_stack_children() {
        assert_serialized_shell_document_rejected(
            serde_json::json!({
                "root": serialized_slot("WorkspaceStage"),
                "stacks": [{
                    "children": vec!["AppDock"; 33],
                    "selected": 0,
                }]
            }),
            "StackChildrenExceeded",
        );
    }

    #[test]
    fn shell_rejects_more_than_sixty_four_component_placements() {
        let placements = vec![
            serde_json::json!({
                "component": "AppDock",
                "region": "AppDock",
            });
            65
        ];
        assert_serialized_shell_document_rejected(
            serde_json::json!({
                "root": serialized_slot("WorkspaceStage"),
                "component_placements": placements,
            }),
            "ComponentPlacementsExceeded",
        );
    }

    #[test]
    fn shell_rejects_duplicate_component_placement() {
        let placement = serde_json::json!({
            "component": "AppDock",
            "region": "AppDock",
        });
        assert_serialized_shell_document_rejected(
            serde_json::json!({
                "root": serialized_slot("WorkspaceStage"),
                "component_placements": [placement.clone(), placement],
            }),
            "DuplicateComponentPlacement",
        );
    }

    #[test]
    fn shell_rejects_invalid_track_bounds() {
        for track in [
            serde_json::json!({ "ContentBounded": { "min": 8, "max": 3 } }),
            serde_json::json!({
                "Resizable": { "min": 9, "preferred": 5, "max": 12 }
            }),
            serde_json::json!({
                "Resizable": { "min": 3, "preferred": 13, "max": 12 }
            }),
            serde_json::json!({ "Fill": { "weight": 0 } }),
        ] {
            assert_serialized_shell_document_rejected(
                serde_json::json!({
                    "root": serialized_slot("WorkspaceStage"),
                    "tracks": { "WorkspaceStage": track },
                }),
                "InvalidTrackBounds",
            );
        }
    }

    #[test]
    fn shell_rejects_out_of_range_stack_selection() {
        assert_serialized_shell_document_rejected(
            serde_json::json!({
                "root": serialized_slot("WorkspaceStage"),
                "stacks": [{
                    "children": ["AppDock"],
                    "selected": 1,
                }]
            }),
            "InvalidStackSelection",
        );
    }

    #[test]
    fn typed_templates_validate_without_runtime_registry() {
        for (template, expected_regions) in [
            ("StageOnly", vec!["WorkspaceStage"]),
            ("DockStage", vec!["AppDock", "WorkspaceStage"]),
            (
                "DockSidebarStage",
                vec!["AppDock", "LeftPanel", "WorkspaceStage"],
            ),
            (
                "DesktopWorkspace",
                vec![
                    "AppDock",
                    "BottomBar",
                    "LeftPanel",
                    "RightPanel",
                    "TopBar",
                    "WorkspaceStage",
                ],
            ),
            (
                "InspectorWorkspace",
                vec!["LeftPanel", "RightPanel", "WorkspaceStage"],
            ),
        ] {
            let serialized = serde_json::json!({ "template": template }).to_string();
            let layout = serde_json::from_str::<ShellLayout>(&serialized);
            assert!(
                layout.is_ok(),
                "typed template {template} should validate without a runtime registry: {layout:?}"
            );
            let serialized_layout =
                serde_json::to_value(layout.expect("checked typed template layout"))
                    .expect("serialize typed template layout");
            let mut regions = Vec::new();
            collect_serialized_regions(&serialized_layout, &mut regions);
            regions.sort_unstable();
            regions.dedup();
            assert_eq!(
                regions,
                expected_regions
                    .into_iter()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
                "template {template}"
            );
            assert_eq!(
                serialized_layout["component_placements"],
                serde_json::json!([]),
                "template {template} should not require runtime placements"
            );
            assert_eq!(
                serialized_layout["stacks"],
                serde_json::json!([]),
                "template {template} should not require runtime stacks"
            );

            if template == "DesktopWorkspace" {
                assert_eq!(
                    serialized_layout["tracks"]["TopBar"],
                    serde_json::json!({ "Fixed": { "cells": 0 } })
                );
                assert_eq!(
                    serialized_layout["tracks"]["AppDock"],
                    serde_json::json!({
                        "Resizable": { "min": 3, "preferred": 5, "max": 9 }
                    })
                );
                assert_eq!(
                    serialized_layout["tracks"]["LeftPanel"],
                    serde_json::json!({
                        "Resizable": { "min": 4, "preferred": 26, "max": 40 }
                    })
                );
                assert_eq!(
                    serialized_layout["tracks"]["WorkspaceStage"],
                    serde_json::json!({ "Fill": { "weight": 1 } })
                );
                assert_eq!(
                    serialized_layout["tracks"]["RightPanel"],
                    serde_json::json!({ "Collapsed": { "restore": 32 } })
                );
                assert_eq!(
                    serialized_layout["tracks"]["BottomBar"],
                    serde_json::json!({ "Fixed": { "cells": 0 } })
                );
            }
        }
    }

    #[test]
    fn fixed_track_uses_exact_cells_or_available_space() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": { "Fixed": { "cells": 5 } },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
            }),
        );

        let roomy = layout.compute_regions(Rect::new(0, 0, 12, 3), |_| 9);
        assert_eq!(roomy.get(RegionId::AppDock), Rect::new(0, 0, 5, 3));
        assert_eq!(roomy.get(RegionId::WorkspaceStage), Rect::new(5, 0, 7, 3));

        let constrained = layout.compute_regions(Rect::new(0, 0, 3, 3), |_| 9);
        assert_eq!(constrained.get(RegionId::AppDock), Rect::new(0, 0, 3, 3));
        assert!(constrained.get(RegionId::WorkspaceStage).is_empty());
    }

    #[test]
    fn content_bounded_clamps_measurement() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": { "ContentBounded": { "min": 3, "max": 8 } },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
            }),
        );

        let below_min = layout.compute_regions(Rect::new(0, 0, 20, 2), |_| 1);
        assert_eq!(below_min.get(RegionId::AppDock).width, 3);

        let above_max = layout.compute_regions(Rect::new(0, 0, 20, 2), |_| 12);
        assert_eq!(above_max.get(RegionId::AppDock).width, 8);
    }

    #[test]
    fn resizable_track_clamps_preferred() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": {
                    "Resizable": { "min": 3, "preferred": 5, "max": 9 }
                },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
            }),
        );

        let regions = layout.compute_regions(Rect::new(0, 0, 20, 2), |_| 99);
        assert_eq!(regions.get(RegionId::AppDock).width, 5);
        assert_eq!(regions.get(RegionId::WorkspaceStage).width, 15);
    }

    #[test]
    fn fill_weights_split_only_remaining_cells() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_child(serialized_slot("LeftPanel")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": { "Fixed": { "cells": 10 } },
                "LeftPanel": { "Fill": { "weight": 1 } },
                "WorkspaceStage": { "Fill": { "weight": 2 } },
            }),
        );

        let regions = layout.compute_regions(Rect::new(0, 0, 100, 2), |_| 40);
        assert_eq!(regions.get(RegionId::AppDock).width, 10);
        assert_eq!(regions.get(RegionId::LeftPanel).width, 30);
        assert_eq!(regions.get(RegionId::WorkspaceStage).width, 60);
    }

    #[test]
    fn collapsed_track_is_zero_and_keeps_restore_width() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": { "Collapsed": { "restore": 7 } },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
            }),
        );

        let regions = layout.compute_regions(Rect::new(0, 0, 20, 2), |_| 7);
        assert!(regions.get(RegionId::AppDock).is_empty());
        assert_eq!(
            regions.get(RegionId::WorkspaceStage),
            Rect::new(0, 0, 20, 2)
        );
        let serialized = serde_json::to_value(layout).expect("serialize collapsed track");
        assert_eq!(
            serialized["tracks"]["AppDock"],
            serde_json::json!({ "Collapsed": { "restore": 7 } })
        );
    }

    #[test]
    fn zero_area_never_underflows() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": { "Fixed": { "cells": 5 } },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
            }),
        );

        let regions = layout.compute_regions(Rect::new(u16::MAX, u16::MAX, 0, 0), |_| 5);
        assert!(regions.get(RegionId::AppDock).is_empty());
        assert!(regions.get(RegionId::WorkspaceStage).is_empty());
    }

    #[test]
    fn allocation_remainder_is_deterministic() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_child(serialized_slot("AppDock")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": { "Fill": { "weight": 1 } },
                "WorkspaceStage": { "Fill": { "weight": 2 } },
            }),
        );

        for _ in 0..16 {
            let regions = layout.compute_regions(Rect::new(0, 0, 5, 1), |_| 0);
            assert_eq!(regions.get(RegionId::AppDock), Rect::new(0, 0, 2, 1));
            assert_eq!(regions.get(RegionId::WorkspaceStage), Rect::new(2, 0, 3, 1));
        }
    }

    #[test]
    fn all_rects_are_inside_parent_without_overlap() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_sized_child("Dynamic", serialized_slot("LeftPanel")),
                serialized_child(serialized_slot("WorkspaceStage")),
                serialized_sized_child("Dynamic", serialized_slot("RightPanel")),
            ],
            serde_json::json!({
                "AppDock": { "Fixed": { "cells": 80 } },
                "LeftPanel": {
                    "ContentBounded": { "min": 4, "max": 40 }
                },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
                "RightPanel": {
                    "Resizable": { "min": 20, "preferred": 32, "max": 60 }
                },
            }),
        );
        let area = Rect::new(u16::MAX - 10, u16::MAX - 2, 10, 2);
        let regions = layout.compute_regions(area, |_| u16::MAX);
        let rects = [
            regions.get(RegionId::AppDock),
            regions.get(RegionId::LeftPanel),
            regions.get(RegionId::WorkspaceStage),
            regions.get(RegionId::RightPanel),
        ];

        for rect in rects {
            assert_eq!(rect.intersection(area), rect);
        }
        for left in 0..rects.len() {
            for right in (left + 1)..rects.len() {
                assert!(rects[left].intersection(rects[right]).is_empty());
            }
        }
    }

    #[test]
    fn shell_degrades_in_frozen_priority_order() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_sized_child("Dynamic", serialized_slot("LeftPanel")),
                serialized_child(serialized_slot("WorkspaceStage")),
                serialized_sized_child("Dynamic", serialized_slot("RightPanel")),
            ],
            serde_json::json!({
                "AppDock": {
                    "Resizable": { "min": 3, "preferred": 5, "max": 9 }
                },
                "LeftPanel": {
                    "Resizable": { "min": 4, "preferred": 26, "max": 40 }
                },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
                "RightPanel": {
                    "Resizable": { "min": 20, "preferred": 32, "max": 60 }
                },
            }),
        );
        let preferred = |region| match region {
            RegionId::AppDock => 5,
            RegionId::LeftPanel => 26,
            RegionId::RightPanel => 32,
            _ => 0,
        };

        let workspace = layout.compute_regions(Rect::new(0, 0, 64, 2), preferred);
        assert_eq!(workspace.get(RegionId::AppDock).width, 5);
        assert_eq!(workspace.get(RegionId::LeftPanel).width, 26);
        assert_eq!(workspace.get(RegionId::WorkspaceStage).width, 1);
        assert_eq!(workspace.get(RegionId::RightPanel).width, 32);

        let minimums = layout.compute_regions(Rect::new(0, 0, 28, 2), preferred);
        assert_eq!(minimums.get(RegionId::AppDock).width, 3);
        assert_eq!(minimums.get(RegionId::LeftPanel).width, 4);
        assert_eq!(minimums.get(RegionId::WorkspaceStage).width, 1);
        assert_eq!(minimums.get(RegionId::RightPanel).width, 20);

        let without_inspector = layout.compute_regions(Rect::new(0, 0, 27, 2), preferred);
        assert!(without_inspector.get(RegionId::RightPanel).is_empty());
        assert!(!without_inspector.get(RegionId::AppDock).is_empty());
        assert!(!without_inspector.get(RegionId::LeftPanel).is_empty());
        assert!(!without_inspector.get(RegionId::WorkspaceStage).is_empty());

        let compact = layout.compute_regions(Rect::new(0, 0, 8, 2), preferred);
        assert_eq!(compact.get(RegionId::AppDock).width, 3);
        assert_eq!(compact.get(RegionId::LeftPanel).width, 4);
        assert_eq!(compact.get(RegionId::WorkspaceStage).width, 1);
        assert!(compact.get(RegionId::RightPanel).is_empty());

        let without_dock = layout.compute_regions(Rect::new(0, 0, 7, 2), preferred);
        assert!(without_dock.get(RegionId::AppDock).is_empty());
        assert_eq!(without_dock.get(RegionId::LeftPanel).width, 4);
        assert_eq!(without_dock.get(RegionId::WorkspaceStage).width, 3);

        let too_small = layout.compute_regions(Rect::new(0, 0, 4, 2), preferred);
        assert!(too_small.get(RegionId::AppDock).is_empty());
        assert!(too_small.get(RegionId::LeftPanel).is_empty());
        assert!(too_small.get(RegionId::WorkspaceStage).is_empty());
        assert!(too_small.get(RegionId::RightPanel).is_empty());

        let threshold_modes = [
            (65, "Workspace"),
            (64, "Workspace"),
            (63, "Wide"),
            (29, "Wide"),
            (28, "Wide"),
            (27, "Standard"),
            (9, "Standard"),
            (8, "Standard"),
            (7, "Compact"),
            (6, "Compact"),
            (5, "Compact"),
            (4, "TooSmall"),
        ];
        for (width, expected_mode) in threshold_modes {
            assert_eq!(
                degradation_for_test(&layout, Rect::new(0, 0, width, 2)),
                expected_mode,
                "unexpected degradation at width {width}"
            );
        }
    }

    #[test]
    fn shell_degradation_respects_left_panel_track_bounds() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_sized_child("Dynamic", serialized_slot("LeftPanel")),
                serialized_child(serialized_slot("WorkspaceStage")),
                serialized_sized_child("Dynamic", serialized_slot("RightPanel")),
            ],
            serde_json::json!({
                "AppDock": {
                    "Resizable": { "min": 3, "preferred": 5, "max": 9 }
                },
                "LeftPanel": {
                    "Resizable": { "min": 1, "preferred": 2, "max": 2 }
                },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
                "RightPanel": {
                    "Resizable": { "min": 20, "preferred": 32, "max": 60 }
                },
            }),
        );

        let compact = layout.compute_regions(Rect::new(0, 0, 4, 2), |_| 0);
        assert!(compact.get(RegionId::AppDock).is_empty());
        assert_eq!(compact.get(RegionId::LeftPanel).width, 2);
        assert_eq!(compact.get(RegionId::WorkspaceStage).width, 2);
        assert!(compact.get(RegionId::RightPanel).is_empty());
    }

    #[test]
    fn shell_degrades_height_without_starving_stage() {
        let layout = tracked_layout(
            "Vertical",
            vec![
                serialized_sized_child("Dynamic", serialized_slot("TopBar")),
                serialized_child(serialized_slot("WorkspaceStage")),
                serialized_sized_child("Dynamic", serialized_slot("BottomBar")),
            ],
            serde_json::json!({
                "TopBar": { "ContentBounded": { "min": 1, "max": 3 } },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
                "BottomBar": { "ContentBounded": { "min": 1, "max": 2 } },
            }),
        );
        let measurement = |region| match region {
            RegionId::TopBar => 3,
            RegionId::BottomBar => 2,
            _ => 0,
        };

        let roomy = layout.compute_regions(Rect::new(0, 0, 20, 6), measurement);
        assert_eq!(roomy.get(RegionId::TopBar).height, 3);
        assert_eq!(roomy.get(RegionId::WorkspaceStage).height, 1);
        assert_eq!(roomy.get(RegionId::BottomBar).height, 2);

        let minimums = layout.compute_regions(Rect::new(0, 0, 20, 3), measurement);
        assert_eq!(minimums.get(RegionId::TopBar).height, 1);
        assert_eq!(minimums.get(RegionId::WorkspaceStage).height, 1);
        assert_eq!(minimums.get(RegionId::BottomBar).height, 1);

        let without_bottom = layout.compute_regions(Rect::new(0, 0, 20, 2), measurement);
        assert_eq!(without_bottom.get(RegionId::TopBar).height, 1);
        assert_eq!(without_bottom.get(RegionId::WorkspaceStage).height, 1);
        assert!(without_bottom.get(RegionId::BottomBar).is_empty());

        let stage_only = layout.compute_regions(Rect::new(0, 0, 20, 1), measurement);
        assert!(stage_only.get(RegionId::TopBar).is_empty());
        assert_eq!(stage_only.get(RegionId::WorkspaceStage).height, 1);
        assert!(stage_only.get(RegionId::BottomBar).is_empty());

        let empty = layout.compute_regions(Rect::new(0, 0, 20, 0), measurement);
        assert!(empty.get(RegionId::TopBar).is_empty());
        assert!(empty.get(RegionId::WorkspaceStage).is_empty());
        assert!(empty.get(RegionId::BottomBar).is_empty());

        let threshold_modes = [
            (7, "Workspace"),
            (6, "Workspace"),
            (5, "Wide"),
            (4, "Wide"),
            (3, "Wide"),
            (2, "Standard"),
            (1, "Compact"),
            (0, "TooSmall"),
        ];
        for (height, expected_mode) in threshold_modes {
            assert_eq!(
                degradation_for_test_with(&layout, Rect::new(0, 0, 20, height), &measurement,),
                expected_mode,
                "unexpected degradation at height {height}"
            );
        }
    }

    #[test]
    fn nested_stage_drives_height_degradation() {
        let body = serialized_split(vec![
            serialized_sized_child("Dynamic", serialized_slot("AppDock")),
            serialized_child(serialized_slot("WorkspaceStage")),
        ]);
        let layout = tracked_layout(
            "Vertical",
            vec![
                serialized_sized_child("Dynamic", serialized_slot("TopBar")),
                serialized_child(body),
                serialized_sized_child("Dynamic", serialized_slot("BottomBar")),
            ],
            serde_json::json!({
                "TopBar": { "ContentBounded": { "min": 1, "max": 3 } },
                "AppDock": {
                    "Resizable": { "min": 3, "preferred": 5, "max": 9 }
                },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
                "BottomBar": { "ContentBounded": { "min": 1, "max": 2 } },
            }),
        );

        let one_row = layout.compute_regions(Rect::new(0, 0, 80, 1), |_| 3);
        assert!(one_row.get(RegionId::TopBar).is_empty());
        assert_eq!(
            one_row.get(RegionId::WorkspaceStage),
            Rect::new(5, 0, 75, 1)
        );
        assert!(one_row.get(RegionId::BottomBar).is_empty());
    }

    #[test]
    fn desktop_workspace_template_solves_normal_compact_and_too_small() {
        let layout: ShellLayout = serde_json::from_value(serde_json::json!({
            "template": "DesktopWorkspace",
        }))
        .expect("built-in desktop template should validate");

        let normal = layout.compute_regions(Rect::new(0, 0, 120, 40), |_| 0);
        assert!(normal.get(RegionId::TopBar).is_empty());
        assert_eq!(normal.get(RegionId::AppDock), Rect::new(0, 0, 5, 40));
        assert_eq!(normal.get(RegionId::LeftPanel), Rect::new(5, 0, 26, 40));
        assert_eq!(
            normal.get(RegionId::WorkspaceStage),
            Rect::new(31, 0, 89, 40)
        );
        assert!(normal.get(RegionId::RightPanel).is_empty());
        assert!(normal.get(RegionId::BottomBar).is_empty());

        let compact = layout.compute_regions(Rect::new(0, 0, 7, 1), |_| 0);
        assert!(compact.get(RegionId::AppDock).is_empty());
        assert_eq!(compact.get(RegionId::LeftPanel), Rect::new(0, 0, 4, 1));
        assert_eq!(compact.get(RegionId::WorkspaceStage), Rect::new(4, 0, 3, 1));
        assert!(compact.get(RegionId::RightPanel).is_empty());

        let too_small = layout.compute_regions(Rect::new(0, 0, 4, 1), |_| 0);
        assert!(too_small.get(RegionId::AppDock).is_empty());
        assert!(too_small.get(RegionId::LeftPanel).is_empty());
        assert!(too_small.get(RegionId::WorkspaceStage).is_empty());
        assert!(too_small.get(RegionId::RightPanel).is_empty());
    }

    #[test]
    fn invalid_tracked_layout_fails_closed_without_partial_regions() {
        let invalid = ShellLayout::from_parts(
            ShellNode::Slot {
                region: RegionId::AppDock,
            },
            std::collections::BTreeMap::from([(
                RegionId::AppDock,
                model::TrackPolicy::Fixed { cells: 5 },
            )]),
            Vec::new(),
            Vec::new(),
        );

        let regions = invalid.compute_regions(Rect::new(0, 0, 80, 24), |_| 5);
        assert!(regions.get(RegionId::AppDock).is_empty());
        assert!(regions.get(RegionId::WorkspaceStage).is_empty());
    }

    #[test]
    fn shell_solver_visits_each_node_at_most_twice() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_child(serialized_slot("LeftPanel")),
                serialized_child(serialized_slot("WorkspaceStage")),
            ],
            serde_json::json!({
                "AppDock": { "Fixed": { "cells": 5 } },
                "LeftPanel": { "Fill": { "weight": 1 } },
                "WorkspaceStage": { "Fill": { "weight": 2 } },
            }),
        );

        let (regions, visits) = compute_regions_with_visit_count(&layout, Rect::new(0, 0, 80, 24));
        assert_eq!(regions.get(RegionId::AppDock).width, 5);
        assert!(
            visits <= 8,
            "four serialized nodes should require at most two visits each, got {visits}"
        );
    }

    #[test]
    fn shell_reports_explicit_too_small_degradation() {
        let layout = tracked_horizontal_layout(
            vec![
                serialized_sized_child("Dynamic", serialized_slot("AppDock")),
                serialized_sized_child("Dynamic", serialized_slot("LeftPanel")),
                serialized_child(serialized_slot("WorkspaceStage")),
                serialized_sized_child("Dynamic", serialized_slot("RightPanel")),
            ],
            serde_json::json!({
                "AppDock": {
                    "Resizable": { "min": 3, "preferred": 5, "max": 9 }
                },
                "LeftPanel": {
                    "Resizable": { "min": 4, "preferred": 26, "max": 40 }
                },
                "WorkspaceStage": { "Fill": { "weight": 1 } },
                "RightPanel": {
                    "Resizable": { "min": 20, "preferred": 32, "max": 60 }
                },
            }),
        );

        assert_eq!(
            degradation_for_test(&layout, Rect::new(0, 0, 4, 2)),
            "TooSmall"
        );
    }

    #[test]
    fn unchanged_geometry_key_reuses_shell_generation() {
        let resolver_calls = std::cell::Cell::new(0);
        let layout = ShellLayout::default();
        let area = Rect::new(0, 0, 80, 24);
        let previous = ShellView {
            generation: 7,
            area,
            regions: layout.compute_regions(area, legacy_sidebar_resolver(26)),
            hits: Vec::new(),
            degradation: ResponsiveDegradation::Workspace,
            geometry_key: ShellGeometryKey::new(area, 0, 26, 0),
        };

        let current = compute_shell_view(&layout, previous.geometry_key, previous.clone(), &|_| {
            resolver_calls.set(resolver_calls.get() + 1);
            26
        });

        assert_eq!(current.generation, 7);
        assert_eq!(current.geometry_key, previous.geometry_key);
        assert_eq!(current.regions, previous.regions);
        assert_eq!(resolver_calls.get(), 0);
    }

    #[test]
    fn area_or_constraint_change_advances_shell_generation_once() {
        let layout = ShellLayout::default();
        let area = Rect::new(0, 0, 80, 24);
        let previous = ShellView {
            generation: 7,
            area,
            regions: layout.compute_regions(area, legacy_sidebar_resolver(26)),
            hits: Vec::new(),
            degradation: ResponsiveDegradation::Workspace,
            geometry_key: ShellGeometryKey::new(area, 0, 26, 0),
        };

        let area_changed =
            compute_shell_view_for_test(&layout, Rect::new(0, 0, 81, 24), 26, Some(&previous));
        let constraint_changed = compute_shell_view_for_test(&layout, area, 27, Some(&previous));

        assert_eq!(
            [area_changed.generation, constraint_changed.generation],
            [8, 8]
        );
    }

    #[test]
    fn flattened_hits_are_complete_disjoint_and_in_bounds() {
        let layout = ShellLayout::default();
        let area = Rect::new(10, 20, 80, 24);
        let view = compute_shell_view_for_test(&layout, area, 26, None);

        assert_eq!(view.hits.len(), 2);
        assert_eq!(
            view.hits[0].target,
            super::view::ShellHitTarget::Region(RegionId::LeftPanel)
        );
        assert_eq!(
            view.hits[1].target,
            super::view::ShellHitTarget::Region(RegionId::WorkspaceStage)
        );
        assert_eq!(view.hits[0].generation, view.generation);
        assert_eq!(view.hits[1].generation, view.generation);
        assert_eq!(view.hits[0].rect.intersection(area), view.hits[0].rect);
        assert_eq!(view.hits[1].rect.intersection(area), view.hits[1].rect);
        assert!(view.hits[0].rect.intersection(view.hits[1].rect).is_empty());
        assert_eq!(
            u32::from(view.hits[0].rect.width) + u32::from(view.hits[1].rect.width),
            u32::from(area.width)
        );
    }

    #[test]
    fn stale_shell_hit_generation_is_rejected() {
        let rect = Rect::new(0, 0, 5, 10);
        let view = ShellView {
            generation: 9,
            area: rect,
            regions: RegionRects::default(),
            hits: vec![super::view::ShellHitArea {
                generation: 9,
                target: super::view::ShellHitTarget::Region(RegionId::AppDock),
                rect,
            }],
            degradation: ResponsiveDegradation::Workspace,
            geometry_key: ShellGeometryKey::new(rect, 0, 5, 0),
        };

        assert_eq!(shell_hit_for_test(&view, 9, 2, 2), Some(RegionId::AppDock));
        assert_eq!(shell_hit_for_test(&view, 8, 2, 2), None);
    }

    #[test]
    fn legacy_sidebar_and_center_rects_match_compatibility_projection() {
        let layout = ShellLayout::default();
        let area = Rect::new(4, 7, 100, 30);
        let view = compute_shell_view_for_test(&layout, area, 26, None);
        let [legacy_sidebar, legacy_center] =
            Layout::horizontal([Constraint::Length(26), Constraint::Min(1)]).areas(area);

        assert_eq!(view.regions.get(RegionId::LeftPanel), legacy_sidebar);
        assert_eq!(view.regions.get(RegionId::CenterContent), legacy_center);
        assert_eq!(view.regions.get(RegionId::WorkspaceStage), legacy_center);
    }

    #[test]
    fn mobile_empty_projection_clears_hits_once_and_reuses_generation() {
        let layout = ShellLayout::default();
        let desktop_area = Rect::new(0, 0, 80, 24);
        let desktop = compute_shell_view_for_test(&layout, desktop_area, 26, None);
        assert!(!desktop.hits.is_empty());
        let desktop_generation = desktop.generation;

        let mobile_key = ShellGeometryKey::new(Rect::new(0, 0, 30, 20), 2, 0, 0);
        let mobile = compute_empty_shell_view(mobile_key, desktop);
        assert_eq!(mobile.generation, desktop_generation + 1);
        assert_eq!(mobile.area, mobile_key.area);
        assert_eq!(mobile.regions, RegionRects::default());
        assert!(mobile.hits.is_empty());

        let mobile_generation = mobile.generation;
        let repeated = compute_empty_shell_view(mobile_key, mobile);
        assert_eq!(repeated.generation, mobile_generation);
        assert!(repeated.hits.is_empty());
    }

    #[test]
    fn generation_exhaustion_keeps_geometry_but_clears_hit_authority() {
        let layout = ShellLayout::default();
        let old_area = Rect::new(0, 0, 80, 24);
        let previous = ShellView {
            generation: u64::MAX,
            area: old_area,
            regions: layout.compute_regions(old_area, legacy_sidebar_resolver(26)),
            hits: vec![super::view::ShellHitArea {
                generation: u64::MAX,
                target: super::view::ShellHitTarget::Region(RegionId::LeftPanel),
                rect: Rect::new(0, 0, 26, 24),
            }],
            degradation: ResponsiveDegradation::Workspace,
            geometry_key: ShellGeometryKey::new(old_area, 0, 26, 0),
        };
        let new_area = Rect::new(0, 0, 81, 24);

        let current = compute_shell_view_for_test(&layout, new_area, 26, Some(&previous));

        assert_eq!(current.generation, u64::MAX);
        assert_eq!(current.area, new_area);
        assert_eq!(current.regions.get(RegionId::LeftPanel).width, 26);
        assert_eq!(current.regions.get(RegionId::WorkspaceStage).width, 55);
        assert!(current.hits.is_empty());
        assert_eq!(shell_hit_for_test(&current, u64::MAX, 2, 2), None);
    }

    fn compute_shell_view_for_test(
        layout: &ShellLayout,
        area: Rect,
        sidebar_width: u16,
        previous: Option<&ShellView>,
    ) -> ShellView {
        let resolver = legacy_sidebar_resolver(sidebar_width);
        compute_shell_view(
            layout,
            ShellGeometryKey::new(area, 0, u64::from(sidebar_width), 0),
            previous.cloned().unwrap_or_default(),
            &resolver,
        )
    }

    fn shell_hit_for_test(view: &ShellView, generation: u64, x: u16, y: u16) -> Option<RegionId> {
        view.hit_at(generation, ratatui::layout::Position::new(x, y))
            .map(|target| match target {
                super::view::ShellHitTarget::Region(region) => region,
            })
    }

    fn legacy_sidebar_resolver(sidebar_width: u16) -> impl Fn(RegionId) -> u16 {
        move |region| u16::from(region == RegionId::LeftPanel) * sidebar_width
    }

    fn collect_serialized_regions(value: &serde_json::Value, out: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(entries) => {
                for (key, value) in entries {
                    if key == "region" {
                        if let Some(region) = value.as_str() {
                            out.push(region.to_owned());
                        }
                    }
                    collect_serialized_regions(value, out);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    collect_serialized_regions(value, out);
                }
            }
            _ => {}
        }
    }

    fn serialized_slot(region: &str) -> serde_json::Value {
        serde_json::json!({ "Slot": { "region": region } })
    }

    fn serialized_child(node: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "size": "Fill", "node": node })
    }

    fn serialized_sized_child(size: &str, node: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "size": size, "node": node })
    }

    fn tracked_horizontal_layout(
        children: Vec<serde_json::Value>,
        tracks: serde_json::Value,
    ) -> ShellLayout {
        tracked_layout("Horizontal", children, tracks)
    }

    fn tracked_layout(
        direction: &str,
        children: Vec<serde_json::Value>,
        tracks: serde_json::Value,
    ) -> ShellLayout {
        serde_json::from_value(serde_json::json!({
            "root": {
                "Split": {
                    "direction": direction,
                    "children": children,
                }
            },
            "tracks": tracks,
        }))
        .expect("tracked shell fixture should validate")
    }

    fn compute_regions_with_visit_count(layout: &ShellLayout, area: Rect) -> (RegionRects, usize) {
        let solved = layout.solve_tracked_for_test(area, &|_| 5);
        (solved.regions().clone(), solved.visit_count())
    }

    fn degradation_for_test(layout: &ShellLayout, area: Rect) -> &'static str {
        degradation_for_test_with(layout, area, &|_| 0)
    }

    fn degradation_for_test_with(
        layout: &ShellLayout,
        area: Rect,
        resolve_dynamic: &impl Fn(RegionId) -> u16,
    ) -> &'static str {
        match layout
            .solve_tracked_for_test(area, resolve_dynamic)
            .degradation()
        {
            layout::ResponsiveDegradation::Workspace => "Workspace",
            layout::ResponsiveDegradation::Wide => "Wide",
            layout::ResponsiveDegradation::Standard => "Standard",
            layout::ResponsiveDegradation::Compact => "Compact",
            layout::ResponsiveDegradation::TooSmall => "TooSmall",
        }
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
        assert_serialized_shell_document_rejected(serde_json::json!({ "root": root }), expected);
    }

    fn assert_serialized_shell_document_rejected(document: serde_json::Value, expected: &str) {
        let serialized = document.to_string();
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
        ShellLayout::from_legacy_root(ShellNode::Split {
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
        })
    }
}
