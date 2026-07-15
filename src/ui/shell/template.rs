use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::model::{
    RegionId, RegionSize, ShellChild, ShellDirection, ShellLayout, ShellNode, ShellValidationError,
    TrackPolicy, ValidatedShellLayout,
};

/// Closed built-in page templates; Foundation v0 exposes no arbitrary layout DSL.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ShellTemplateId {
    StageOnly,
    DockStage,
    DockSidebarStage,
    DesktopWorkspace,
    InspectorWorkspace,
}

impl ShellTemplateId {
    pub(crate) fn validated_layout(self) -> Result<ValidatedShellLayout, ShellValidationError> {
        self.build().validate()
    }

    fn build(self) -> ShellLayout {
        match self {
            Self::StageOnly => shell_layout(
                slot(RegionId::WorkspaceStage),
                [(RegionId::WorkspaceStage, TrackPolicy::Fill { weight: 1 })],
            ),
            Self::DockStage => shell_layout(
                horizontal(vec![
                    dynamic_child(slot(RegionId::AppDock)),
                    fill_child(slot(RegionId::WorkspaceStage)),
                ]),
                [
                    (RegionId::AppDock, dock_track()),
                    (RegionId::WorkspaceStage, TrackPolicy::Fill { weight: 1 }),
                ],
            ),
            Self::DockSidebarStage => shell_layout(
                horizontal(vec![
                    dynamic_child(slot(RegionId::AppDock)),
                    dynamic_child(slot(RegionId::LeftPanel)),
                    fill_child(slot(RegionId::WorkspaceStage)),
                ]),
                [
                    (RegionId::AppDock, dock_track()),
                    (RegionId::LeftPanel, sidebar_track()),
                    (RegionId::WorkspaceStage, TrackPolicy::Fill { weight: 1 }),
                ],
            ),
            Self::DesktopWorkspace => desktop_workspace(),
            Self::InspectorWorkspace => shell_layout(
                horizontal(vec![
                    dynamic_child(slot(RegionId::LeftPanel)),
                    fill_child(slot(RegionId::WorkspaceStage)),
                    dynamic_child(slot(RegionId::RightPanel)),
                ]),
                [
                    (RegionId::LeftPanel, sidebar_track()),
                    (RegionId::WorkspaceStage, TrackPolicy::Fill { weight: 1 }),
                    (
                        RegionId::RightPanel,
                        TrackPolicy::Resizable {
                            min: 20,
                            preferred: 32,
                            max: 60,
                        },
                    ),
                ],
            ),
        }
    }
}

fn desktop_workspace() -> ShellLayout {
    let body = horizontal(vec![
        dynamic_child(slot(RegionId::AppDock)),
        dynamic_child(slot(RegionId::LeftPanel)),
        fill_child(slot(RegionId::WorkspaceStage)),
        dynamic_child(slot(RegionId::RightPanel)),
    ]);
    shell_layout(
        vertical(vec![
            dynamic_child(slot(RegionId::TopBar)),
            fill_child(body),
            dynamic_child(slot(RegionId::BottomBar)),
        ]),
        [
            (RegionId::TopBar, TrackPolicy::Fixed { cells: 0 }),
            (RegionId::AppDock, dock_track()),
            (RegionId::LeftPanel, sidebar_track()),
            (RegionId::WorkspaceStage, TrackPolicy::Fill { weight: 1 }),
            (RegionId::RightPanel, TrackPolicy::Collapsed { restore: 32 }),
            (RegionId::BottomBar, TrackPolicy::Fixed { cells: 0 }),
        ],
    )
}

fn shell_layout(
    root: ShellNode,
    tracks: impl IntoIterator<Item = (RegionId, TrackPolicy)>,
) -> ShellLayout {
    ShellLayout::from_parts(root, BTreeMap::from_iter(tracks), Vec::new(), Vec::new())
}

fn slot(region: RegionId) -> ShellNode {
    ShellNode::Slot { region }
}

fn horizontal(children: Vec<ShellChild>) -> ShellNode {
    ShellNode::Split {
        direction: ShellDirection::Horizontal,
        children,
    }
}

fn vertical(children: Vec<ShellChild>) -> ShellNode {
    ShellNode::Split {
        direction: ShellDirection::Vertical,
        children,
    }
}

fn dynamic_child(node: ShellNode) -> ShellChild {
    ShellChild {
        size: RegionSize::Dynamic,
        node,
    }
}

fn fill_child(node: ShellNode) -> ShellChild {
    ShellChild {
        size: RegionSize::Fill,
        node,
    }
}

fn dock_track() -> TrackPolicy {
    TrackPolicy::Resizable {
        min: 3,
        preferred: 5,
        max: 9,
    }
}

fn sidebar_track() -> TrackPolicy {
    TrackPolicy::Resizable {
        min: 0,
        preferred: 26,
        max: 40,
    }
}
