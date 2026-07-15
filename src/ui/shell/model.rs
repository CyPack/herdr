use std::collections::{HashMap, HashSet};
use std::fmt;

use ratatui::layout::Rect;
use serde::{de, Deserialize, Deserializer, Serialize};

pub(super) const MAX_NESTED_SPLIT_DEPTH: usize = 4;
pub(super) const MAX_SPLIT_CHILDREN: usize = 8;
pub(super) const MAX_VISIBLE_LEAVES: usize = 64;
pub(super) const MAX_SERIALIZED_NODES: usize = 128;

/// Stable identities for the finite outer shell owned by the TUI client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum RegionId {
    TopBar,
    AppDock,
    LeftPanel,
    /// Temporary local compatibility identity until all current call sites and
    /// persisted shell fixtures migrate to `WorkspaceStage`.
    CenterContent,
    WorkspaceStage,
    RightPanel,
    BottomBar,
}

/// Legacy sizing policy retained until the bounded solver lands in SF2.3.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) enum RegionSize {
    Dynamic,
    Fill,
}

/// Serializable legacy shell topology used by the compatibility projection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum ShellNode {
    Slot {
        region: RegionId,
    },
    Split {
        direction: ShellDirection,
        children: Vec<ShellChild>,
    },
}

/// One child and its sizing policy inside a split node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ShellChild {
    pub size: RegionSize,
    pub node: ShellNode,
}

/// Stable serializable split axis, independent of Ratatui's representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ShellDirection {
    Horizontal,
    Vertical,
}

/// A bounded shell tree. Deserialization validates before exposing the value.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub(crate) struct ShellLayout {
    pub(super) root: ShellNode,
}

/// Proof that a shell tree satisfies the finite composition invariants.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ValidatedShellLayout(ShellLayout);

impl ValidatedShellLayout {
    fn into_inner(self) -> ShellLayout {
        self.0
    }
}

/// Fail-closed reasons produced before a shell tree reaches layout projection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ShellValidationError {
    EmptySplit,
    DepthExceeded,
    ChildrenExceeded,
    VisibleLeavesExceeded,
    SerializedNodesExceeded,
    DuplicateRegion(RegionId),
    MissingWorkspaceStage,
}

impl fmt::Display for ShellValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ShellValidationError {}

impl ShellLayout {
    /// Validate with one iterative traversal bounded by serialized node count.
    pub(crate) fn validate(self) -> Result<ValidatedShellLayout, ShellValidationError> {
        let mut stack = vec![(&self.root, 0usize)];
        let mut serialized_nodes = 0usize;
        let mut visible_leaves = 0usize;
        let mut regions = HashSet::new();
        let mut duplicate_region = None;
        let mut has_workspace_stage = false;

        while let Some((node, split_depth)) = stack.pop() {
            serialized_nodes += 1;
            if serialized_nodes > MAX_SERIALIZED_NODES {
                return Err(ShellValidationError::SerializedNodesExceeded);
            }

            match node {
                ShellNode::Slot { region } => {
                    visible_leaves += 1;
                    if visible_leaves > MAX_VISIBLE_LEAVES {
                        return Err(ShellValidationError::VisibleLeavesExceeded);
                    }

                    let canonical_region = canonical_region(*region);
                    has_workspace_stage |= canonical_region == RegionId::WorkspaceStage;
                    if !regions.insert(canonical_region) && duplicate_region.is_none() {
                        duplicate_region = Some(canonical_region);
                    }
                }
                ShellNode::Split { children, .. } => {
                    if children.is_empty() {
                        return Err(ShellValidationError::EmptySplit);
                    }
                    if children.len() > MAX_SPLIT_CHILDREN {
                        return Err(ShellValidationError::ChildrenExceeded);
                    }
                    let child_split_depth = split_depth + 1;
                    if child_split_depth > MAX_NESTED_SPLIT_DEPTH {
                        return Err(ShellValidationError::DepthExceeded);
                    }
                    stack.extend(
                        children
                            .iter()
                            .rev()
                            .map(|child| (&child.node, child_split_depth)),
                    );
                }
            }
        }

        if let Some(region) = duplicate_region {
            return Err(ShellValidationError::DuplicateRegion(region));
        }
        if !has_workspace_stage {
            return Err(ShellValidationError::MissingWorkspaceStage);
        }

        Ok(ValidatedShellLayout(self))
    }
}

#[derive(Deserialize)]
struct SerializedShellLayout {
    root: ShellNode,
}

impl<'de> Deserialize<'de> for ShellLayout {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialized = SerializedShellLayout::deserialize(deserializer)?;
        ShellLayout {
            root: serialized.root,
        }
        .validate()
        .map(ValidatedShellLayout::into_inner)
        .map_err(de::Error::custom)
    }
}

/// Resolved region rectangles with a temporary CenterContent/WorkspaceStage
/// compatibility lookup during the SF2 migration.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct RegionRects {
    pub(super) rects: HashMap<RegionId, Rect>,
}

impl RegionRects {
    pub(crate) fn get(&self, region: RegionId) -> Rect {
        self.rects
            .get(&region)
            .or_else(|| self.rects.get(&compatibility_region(region)))
            .copied()
            .unwrap_or_default()
    }

    pub(super) fn insert(&mut self, region: RegionId, rect: Rect) {
        self.rects.insert(region, rect);
    }
}

fn canonical_region(region: RegionId) -> RegionId {
    match region {
        RegionId::CenterContent => RegionId::WorkspaceStage,
        other => other,
    }
}

fn compatibility_region(region: RegionId) -> RegionId {
    match region {
        RegionId::CenterContent => RegionId::WorkspaceStage,
        RegionId::WorkspaceStage => RegionId::CenterContent,
        other => other,
    }
}
