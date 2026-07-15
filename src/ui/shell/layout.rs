use std::collections::BTreeMap;

use ratatui::layout::Rect;

use super::model::{
    RegionId, RegionRects, RegionSize, ShellChild, ShellDirection, ShellNode, TrackPolicy,
    ValidatedShellLayout,
};

const LEFT_PANEL_COMPACT_WIDTH: u16 = 4;
const STAGE_MIN_CELLS: u16 = 1;

/// Deterministic responsive state produced together with shell geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ResponsiveDegradation {
    Workspace,
    Wide,
    Standard,
    Compact,
    TooSmall,
}

pub(super) struct SolvedShellLayout {
    regions: RegionRects,
    degradation: ResponsiveDegradation,
    #[cfg(test)]
    visit_count: usize,
}

impl SolvedShellLayout {
    pub(super) fn into_parts(self) -> (RegionRects, ResponsiveDegradation) {
        (self.regions, self.degradation)
    }

    #[cfg(test)]
    pub(super) fn visit_count(&self) -> usize {
        self.visit_count
    }

    #[cfg(test)]
    pub(super) fn degradation(&self) -> ResponsiveDegradation {
        self.degradation
    }

    #[cfg(test)]
    pub(super) fn regions(&self) -> &RegionRects {
        &self.regions
    }
}

pub(super) fn solve(
    layout: &ValidatedShellLayout,
    area: Rect,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
) -> SolvedShellLayout {
    let layout = layout.as_layout();
    let mut stats = SolverStats::default();
    let measured = measure_node(&layout.root, &mut stats);
    let mut regions = RegionRects::default();
    let degradation = allocate_node(
        &measured,
        area,
        &layout.tracks,
        resolve_dynamic,
        &mut regions,
        &mut stats,
    );

    SolvedShellLayout {
        regions,
        degradation,
        #[cfg(test)]
        visit_count: stats.visits,
    }
}

#[derive(Default)]
struct SolverStats {
    #[cfg(test)]
    visits: usize,
}

impl SolverStats {
    fn visit(&mut self) {
        #[cfg(test)]
        {
            self.visits = self.visits.saturating_add(1);
        }
    }
}

enum MeasuredNode {
    Slot {
        region: RegionId,
    },
    Split {
        direction: ShellDirection,
        children: Vec<MeasuredChild>,
        contains_workspace_stage: bool,
    },
}

struct MeasuredChild {
    size: RegionSize,
    node: MeasuredNode,
    primary_region: Option<RegionId>,
    contains_workspace_stage: bool,
}

fn measure_node(node: &ShellNode, stats: &mut SolverStats) -> MeasuredNode {
    stats.visit();
    match node {
        ShellNode::Slot { region } => MeasuredNode::Slot { region: *region },
        ShellNode::Split {
            direction,
            children,
        } => {
            let children = children
                .iter()
                .map(|child| measure_child(child, stats))
                .collect::<Vec<_>>();
            let contains_workspace_stage =
                children.iter().any(|child| child.contains_workspace_stage);
            MeasuredNode::Split {
                direction: *direction,
                children,
                contains_workspace_stage,
            }
        }
    }
}

fn measure_child(child: &ShellChild, stats: &mut SolverStats) -> MeasuredChild {
    let node = measure_node(&child.node, stats);
    let primary_region = measured_primary_region(&node);
    let contains_workspace_stage = measured_contains_workspace_stage(&node);
    MeasuredChild {
        size: child.size,
        node,
        primary_region,
        contains_workspace_stage,
    }
}

fn measured_contains_workspace_stage(node: &MeasuredNode) -> bool {
    match node {
        MeasuredNode::Slot { region } => *region == RegionId::WorkspaceStage,
        MeasuredNode::Split {
            contains_workspace_stage,
            ..
        } => *contains_workspace_stage,
    }
}

fn measured_primary_region(node: &MeasuredNode) -> Option<RegionId> {
    match node {
        MeasuredNode::Slot { region } => Some(*region),
        MeasuredNode::Split { children, .. } => {
            children.first().and_then(|child| child.primary_region)
        }
    }
}

fn allocate_node(
    node: &MeasuredNode,
    area: Rect,
    tracks: &BTreeMap<RegionId, TrackPolicy>,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
    out: &mut RegionRects,
    stats: &mut SolverStats,
) -> ResponsiveDegradation {
    stats.visit();
    match node {
        MeasuredNode::Slot { region } => {
            out.insert(*region, area);
            ResponsiveDegradation::Workspace
        }
        MeasuredNode::Split {
            direction,
            children,
            ..
        } => {
            let available = match direction {
                ShellDirection::Horizontal => area.width,
                ShellDirection::Vertical => area.height,
            };
            let (lengths, mut degradation) =
                allocate_lengths(*direction, children, tracks, resolve_dynamic, available);
            if degradation == ResponsiveDegradation::TooSmall {
                return degradation;
            }

            let mut offset = 0u16;
            for (child, length) in children.iter().zip(lengths) {
                let child_area = child_rect(area, *direction, offset, length);
                offset = offset.saturating_add(length);
                degradation = degradation.max(allocate_node(
                    &child.node,
                    child_area,
                    tracks,
                    resolve_dynamic,
                    out,
                    stats,
                ));
            }
            degradation
        }
    }
}

fn child_rect(area: Rect, direction: ShellDirection, offset: u16, length: u16) -> Rect {
    match direction {
        ShellDirection::Horizontal => Rect::new(
            area.x.saturating_add(offset),
            area.y,
            length.min(area.width.saturating_sub(offset)),
            area.height,
        ),
        ShellDirection::Vertical => Rect::new(
            area.x,
            area.y.saturating_add(offset),
            area.width,
            length.min(area.height.saturating_sub(offset)),
        ),
    }
}

#[derive(Clone, Copy)]
struct TrackRequest {
    region: Option<RegionId>,
    contains_workspace_stage: bool,
    min: u16,
    desired: u16,
    max: u16,
    fill_weight: u16,
    collapsed: bool,
    resizable: bool,
}

impl TrackRequest {
    fn fixed(region: Option<RegionId>, cells: u16) -> Self {
        Self {
            region,
            contains_workspace_stage: false,
            min: cells,
            desired: cells,
            max: cells,
            fill_weight: 0,
            collapsed: false,
            resizable: false,
        }
    }

    fn fill(region: Option<RegionId>, weight: u16) -> Self {
        Self {
            region,
            contains_workspace_stage: false,
            min: 0,
            desired: 0,
            max: u16::MAX,
            fill_weight: weight,
            collapsed: false,
            resizable: false,
        }
    }

    fn collapse(&mut self) {
        self.min = 0;
        self.desired = 0;
        self.fill_weight = 0;
        self.collapsed = true;
    }

    fn is_fill(self) -> bool {
        self.fill_weight > 0 && !self.collapsed
    }
}

fn allocate_lengths(
    direction: ShellDirection,
    children: &[MeasuredChild],
    tracks: &BTreeMap<RegionId, TrackPolicy>,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
    available: u16,
) -> (Vec<u16>, ResponsiveDegradation) {
    let mut requests = children
        .iter()
        .map(|child| request_for_child(child, tracks, resolve_dynamic))
        .collect::<Vec<_>>();
    let mut degradation = ResponsiveDegradation::Workspace;

    if direction == ShellDirection::Horizontal && is_workspace_split(&requests) {
        degradation = degrade_workspace_requests(&mut requests, available);
        if degradation == ResponsiveDegradation::TooSmall {
            return (vec![0; children.len()], degradation);
        }
    } else if direction == ShellDirection::Vertical && has_stage(&requests) {
        degradation = degrade_height_requests(&mut requests, available);
        if degradation == ResponsiveDegradation::TooSmall {
            return (vec![0; children.len()], degradation);
        }
    }

    let mut lengths = vec![0u16; requests.len()];
    let mut remaining = available;
    for (index, request) in requests.iter().enumerate() {
        if request.is_fill() || request.collapsed {
            continue;
        }
        let length = request.min.min(remaining);
        lengths[index] = length;
        remaining = remaining.saturating_sub(length);
    }

    let stage_fill_index = requests
        .iter()
        .position(|request| request.contains_workspace_stage && request.is_fill());
    let reserved_stage = u16::from(stage_fill_index.is_some() && remaining > 0);
    let mut growth_budget = remaining.saturating_sub(reserved_stage);
    for (index, request) in requests.iter().enumerate() {
        if request.is_fill() || request.collapsed || request.desired <= lengths[index] {
            continue;
        }
        let growth = request
            .desired
            .saturating_sub(lengths[index])
            .min(growth_budget);
        lengths[index] = lengths[index].saturating_add(growth);
        growth_budget = growth_budget.saturating_sub(growth);
    }

    let used_non_fill = lengths
        .iter()
        .fold(0u16, |sum, length| sum.saturating_add(*length));
    remaining = available.saturating_sub(used_non_fill);
    distribute_fill(&requests, &mut lengths, remaining, stage_fill_index);

    if degradation == ResponsiveDegradation::Workspace
        && requests.iter().zip(&lengths).any(|(request, length)| {
            !request.is_fill() && !request.collapsed && *length < request.desired
        })
    {
        degradation = ResponsiveDegradation::Wide;
    }

    (lengths, degradation)
}

fn request_for_child(
    child: &MeasuredChild,
    tracks: &BTreeMap<RegionId, TrackPolicy>,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
) -> TrackRequest {
    let mut request = match &child.node {
        MeasuredNode::Slot { region } => tracks
            .get(region)
            .map(|policy| request_from_policy(*region, *policy, resolve_dynamic))
            .unwrap_or_else(|| request_from_legacy_size(child, resolve_dynamic)),
        MeasuredNode::Split { .. } => request_from_legacy_size(child, resolve_dynamic),
    };
    request.contains_workspace_stage = child.contains_workspace_stage;
    request
}

fn request_from_legacy_size(
    child: &MeasuredChild,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
) -> TrackRequest {
    match child.size {
        RegionSize::Dynamic => TrackRequest::fixed(
            child.primary_region,
            child.primary_region.map(resolve_dynamic).unwrap_or(0),
        ),
        RegionSize::Fill => TrackRequest::fill(child.primary_region, 1),
    }
}

fn request_from_policy(
    region: RegionId,
    policy: TrackPolicy,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
) -> TrackRequest {
    match policy {
        TrackPolicy::Fixed { cells } => TrackRequest::fixed(Some(region), cells),
        TrackPolicy::ContentBounded { min, max } => TrackRequest {
            region: Some(region),
            contains_workspace_stage: false,
            min,
            desired: resolve_dynamic(region).clamp(min, max),
            max,
            fill_weight: 0,
            collapsed: false,
            resizable: true,
        },
        TrackPolicy::Resizable {
            min,
            preferred,
            max,
        } => TrackRequest {
            region: Some(region),
            contains_workspace_stage: false,
            min,
            desired: preferred.clamp(min, max),
            max,
            fill_weight: 0,
            collapsed: false,
            resizable: true,
        },
        TrackPolicy::Fill { weight } => TrackRequest::fill(Some(region), weight),
        TrackPolicy::Collapsed { .. } => TrackRequest {
            region: Some(region),
            contains_workspace_stage: false,
            min: 0,
            desired: 0,
            max: 0,
            fill_weight: 0,
            collapsed: true,
            resizable: false,
        },
    }
}

fn is_workspace_split(requests: &[TrackRequest]) -> bool {
    let has_stage = has_stage(requests);
    let has_structural_optional = requests.iter().any(|request| {
        matches!(
            request.region,
            Some(RegionId::LeftPanel | RegionId::RightPanel)
        )
    });
    let has_resizable_dock = requests
        .iter()
        .any(|request| request.region == Some(RegionId::AppDock) && request.resizable);
    has_stage && (has_structural_optional || has_resizable_dock)
}

fn has_stage(requests: &[TrackRequest]) -> bool {
    requests
        .iter()
        .any(|request| request.contains_workspace_stage)
}

fn degrade_workspace_requests(
    requests: &mut [TrackRequest],
    available: u16,
) -> ResponsiveDegradation {
    if minimum_required(requests) <= u32::from(available) {
        return ResponsiveDegradation::Workspace;
    }

    let mut degradation = ResponsiveDegradation::Wide;
    if collapse_region(requests, RegionId::RightPanel) {
        degradation = ResponsiveDegradation::Standard;
    }
    if minimum_required(requests) <= u32::from(available) {
        return degradation;
    }

    if let Some(left_panel) = request_for_region_mut(requests, RegionId::LeftPanel) {
        if !left_panel.collapsed {
            let compact = LEFT_PANEL_COMPACT_WIDTH.clamp(left_panel.min, left_panel.max);
            left_panel.min = compact;
            left_panel.desired = compact;
            left_panel.fill_weight = 0;
            degradation = ResponsiveDegradation::Compact;
        }
    }
    if minimum_required(requests) <= u32::from(available) {
        return degradation;
    }

    if collapse_region(requests, RegionId::AppDock) {
        degradation = ResponsiveDegradation::Compact;
    }
    if minimum_required(requests) <= u32::from(available) {
        return degradation;
    }

    ResponsiveDegradation::TooSmall
}

fn degrade_height_requests(requests: &mut [TrackRequest], available: u16) -> ResponsiveDegradation {
    if minimum_required(requests) <= u32::from(available) {
        return ResponsiveDegradation::Workspace;
    }

    let mut degradation = ResponsiveDegradation::Wide;
    if collapse_region(requests, RegionId::BottomBar) {
        degradation = ResponsiveDegradation::Standard;
    }
    if minimum_required(requests) <= u32::from(available) {
        return degradation;
    }

    if collapse_region(requests, RegionId::TopBar) {
        degradation = ResponsiveDegradation::Compact;
    }
    if minimum_required(requests) <= u32::from(available) {
        return degradation;
    }

    ResponsiveDegradation::TooSmall
}

fn request_for_region_mut(
    requests: &mut [TrackRequest],
    region: RegionId,
) -> Option<&mut TrackRequest> {
    requests
        .iter_mut()
        .find(|request| request.region == Some(region))
}

fn collapse_region(requests: &mut [TrackRequest], region: RegionId) -> bool {
    let Some(request) = request_for_region_mut(requests, region) else {
        return false;
    };
    if request.collapsed {
        return false;
    }
    request.collapse();
    true
}

fn minimum_required(requests: &[TrackRequest]) -> u32 {
    let non_fill = requests
        .iter()
        .filter(|request| !request.is_fill() && !request.collapsed)
        .map(|request| u32::from(request.min))
        .sum::<u32>();
    let stage = requests
        .iter()
        .any(|request| request.contains_workspace_stage && request.is_fill());
    non_fill + u32::from(STAGE_MIN_CELLS) * u32::from(stage)
}

fn distribute_fill(
    requests: &[TrackRequest],
    lengths: &mut [u16],
    remaining: u16,
    stage_fill_index: Option<usize>,
) {
    let total_weight = requests
        .iter()
        .filter(|request| request.is_fill())
        .map(|request| u32::from(request.fill_weight))
        .sum::<u32>();
    if remaining == 0 || total_weight == 0 {
        return;
    }

    let mut remainders = Vec::new();
    let mut assigned = 0u16;
    for (index, request) in requests.iter().enumerate() {
        if !request.is_fill() {
            continue;
        }
        let weighted = u32::from(remaining) * u32::from(request.fill_weight);
        let share = (weighted / total_weight) as u16;
        lengths[index] = share;
        assigned = assigned.saturating_add(share);
        remainders.push((index, weighted % total_weight));
    }

    remainders.sort_by(
        |(left_index, left_remainder), (right_index, right_remainder)| {
            right_remainder
                .cmp(left_remainder)
                .then_with(|| left_index.cmp(right_index))
        },
    );
    let mut leftover = remaining.saturating_sub(assigned);
    for (index, _) in remainders.iter().cycle().take(usize::from(leftover)) {
        lengths[*index] = lengths[*index].saturating_add(1);
        leftover = leftover.saturating_sub(1);
    }
    debug_assert_eq!(leftover, 0);

    if let Some(stage_index) = stage_fill_index {
        if lengths[stage_index] == 0 {
            if let Some((donor_index, _)) = lengths
                .iter()
                .enumerate()
                .filter(|(index, length)| *index != stage_index && **length > 0)
                .max_by_key(|(_, length)| **length)
            {
                lengths[donor_index] = lengths[donor_index].saturating_sub(1);
                lengths[stage_index] = 1;
            }
        }
    }
}
