use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;

use ratatui::layout::Rect;
use serde::{de, Deserialize, Deserializer, Serialize};

use super::template::ShellTemplateId;

pub(super) const MAX_NESTED_SPLIT_DEPTH: usize = 4;
pub(super) const MAX_SPLIT_CHILDREN: usize = 8;
pub(super) const MAX_VISIBLE_LEAVES: usize = 64;
pub(super) const MAX_SERIALIZED_NODES: usize = 128;
pub(super) const MAX_STACK_CHILDREN: usize = 32;
pub(super) const MAX_COMPONENT_PLACEMENTS: usize = 64;

/// Stable identities for the finite outer shell owned by the TUI client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

/// Bounded sizing policy consumed by the SF2.3 solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TrackPolicy {
    Fixed { cells: u16 },
    ContentBounded { min: u16, max: u16 },
    Resizable { min: u16, preferred: u16, max: u16 },
    Fill { weight: u16 },
    Collapsed { restore: u16 },
}

/// Closed v0 identities for components placed into shell regions or stacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum ShellComponentId {
    AppDock,
    AgentSidebar,
    WorkspaceStage,
    Inspector,
    TopBar,
    BottomBar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ComponentPlacement {
    pub component: ShellComponentId,
    pub region: RegionId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StackContainer {
    pub children: Vec<ShellComponentId>,
    pub selected: usize,
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
    pub(super) tracks: BTreeMap<RegionId, TrackPolicy>,
    pub(super) stacks: Vec<StackContainer>,
    pub(super) component_placements: Vec<ComponentPlacement>,
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
    StackChildrenExceeded,
    ComponentPlacementsExceeded,
    DuplicateComponentPlacement,
    DuplicateRegion(RegionId),
    InvalidTrackBounds,
    MissingWorkspaceStage,
    CollapsedWorkspaceStage,
    InvalidStackSelection,
}

impl fmt::Display for ShellValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ShellValidationError {}

impl ShellLayout {
    pub(super) fn from_legacy_root(root: ShellNode) -> Self {
        Self {
            root,
            tracks: BTreeMap::new(),
            stacks: Vec::new(),
            component_placements: Vec::new(),
        }
    }

    pub(super) fn from_parts(
        root: ShellNode,
        tracks: BTreeMap<RegionId, TrackPolicy>,
        stacks: Vec<StackContainer>,
        component_placements: Vec<ComponentPlacement>,
    ) -> Self {
        Self {
            root,
            tracks,
            stacks,
            component_placements,
        }
    }

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

        if self.component_placements.len() > MAX_COMPONENT_PLACEMENTS {
            return Err(ShellValidationError::ComponentPlacementsExceeded);
        }
        for stack in &self.stacks {
            if stack.children.len() > MAX_STACK_CHILDREN {
                return Err(ShellValidationError::StackChildrenExceeded);
            }
            if (stack.children.is_empty() && stack.selected != 0)
                || (!stack.children.is_empty() && stack.selected >= stack.children.len())
            {
                return Err(ShellValidationError::InvalidStackSelection);
            }
        }
        for track in self.tracks.values() {
            let valid = match *track {
                TrackPolicy::Fixed { .. } | TrackPolicy::Collapsed { .. } => true,
                TrackPolicy::ContentBounded { min, max } => min <= max,
                TrackPolicy::Resizable {
                    min,
                    preferred,
                    max,
                } => min <= preferred && preferred <= max,
                TrackPolicy::Fill { weight } => weight > 0,
            };
            if !valid {
                return Err(ShellValidationError::InvalidTrackBounds);
            }
        }
        if matches!(
            self.tracks.get(&RegionId::WorkspaceStage),
            Some(TrackPolicy::Collapsed { .. })
        ) {
            return Err(ShellValidationError::CollapsedWorkspaceStage);
        }
        let mut placed_components = HashSet::new();
        for placement in &self.component_placements {
            if !placed_components.insert(placement.component) {
                return Err(ShellValidationError::DuplicateComponentPlacement);
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
#[serde(deny_unknown_fields)]
struct SerializedShellTree {
    root: ShellNode,
    #[serde(default)]
    tracks: BTreeMap<RegionId, TrackPolicy>,
    #[serde(default)]
    stacks: Vec<StackContainer>,
    #[serde(default)]
    component_placements: Vec<ComponentPlacement>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SerializedShellTemplate {
    template: ShellTemplateId,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SerializedShellLayout {
    Tree(SerializedShellTree),
    Template(SerializedShellTemplate),
}

impl<'de> Deserialize<'de> for ShellLayout {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let serialized = SerializedShellLayout::deserialize(deserializer)?;
        let validated = match serialized {
            SerializedShellLayout::Tree(tree) => ShellLayout::from_parts(
                tree.root,
                tree.tracks,
                tree.stacks,
                tree.component_placements,
            )
            .validate(),
            SerializedShellLayout::Template(template) => template.template.validated_layout(),
        };
        validated
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
