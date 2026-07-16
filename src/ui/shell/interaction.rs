use ratatui::layout::Position;

use super::{RegionId, ShellDirection};

/// Stable identity for one divider between adjacent shell regions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct DividerId {
    leading: RegionId,
    trailing: RegionId,
    axis: ShellDirection,
}

impl DividerId {
    pub(crate) fn new(leading: RegionId, trailing: RegionId, axis: ShellDirection) -> Option<Self> {
        (leading != trailing).then_some(Self {
            leading,
            trailing,
            axis,
        })
    }
}

/// Valid min/max constraints for the two tracks adjacent to a divider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResizeBounds {
    leading_min: u16,
    leading_max: u16,
    trailing_min: u16,
    trailing_max: u16,
}

impl ResizeBounds {
    pub(crate) fn new(
        leading_min: u16,
        leading_max: u16,
        trailing_min: u16,
        trailing_max: u16,
    ) -> Option<Self> {
        (leading_min <= leading_max && trailing_min <= trailing_max).then_some(Self {
            leading_min,
            leading_max,
            trailing_min,
            trailing_max,
        })
    }
}

/// Pure transient state for one bounded divider gesture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ResizeTransaction {
    divider: DividerId,
    view_generation: u64,
    pointer_origin: Position,
    original_tracks: [u16; 2],
    preview_tracks: [u16; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResizeDecision {
    Inert,
    Committed([u16; 2]),
    Cancelled([u16; 2]),
}

/// Pure effect request returned to the application adapter at transaction
/// boundaries. Preview never creates one of these requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResizeUpdate {
    decision: ResizeDecision,
    mark_persistence_dirty: bool,
    request_pty_resize: bool,
}

/// Client-local committed collapse state for one named optional region.
/// Transient resize preview never writes this state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RegionCollapseState {
    region: RegionId,
    restore_width: u16,
    collapsed: bool,
    revision: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CollapseDecision {
    Inert,
    Collapsed { restore_width: u16 },
    Expanded { width: u16 },
}

/// Pure persistence effect request for one collapse/expand boundary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CollapseUpdate {
    decision: CollapseDecision,
    mark_persistence_dirty: bool,
}

/// Axis owned by one bounded scroll viewport.
// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollAxis {
    Horizontal,
    Vertical,
}

/// Stable client-local identity for a viewport inside a named region. `slot`
/// permits more than one bounded component viewport without coupling identity
/// to its current rectangle or tree position.
// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScrollViewportId {
    region: RegionId,
    slot: u8,
}

/// Two-dimensional content offset, independent from derived terminal geometry.
// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ScrollOffset {
    horizontal: usize,
    vertical: usize,
}

/// Derived viewport/content extents for one compute generation. Zero viewport
/// extent deliberately yields a zero maximum offset.
// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScrollViewportMetrics {
    viewport_width: u16,
    viewport_height: u16,
    content_width: usize,
    content_height: usize,
}

/// Committed client-local offset for one viewport. It owns no geometry, input
/// handle, runtime resource, or render loop.
// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScrollViewportState {
    id: ScrollViewportId,
    offset: ScrollOffset,
}

/// One already-hit-tested scroll owner. The router receives these bottom to
/// top and never falls through the final owner.
// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ScrollOwner {
    id: ScrollViewportId,
    metrics: ScrollViewportMetrics,
}

// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ScrollDecision {
    Unhandled,
    Consumed { changed: bool },
}

/// Aggregate committed shell presentation preferences. AppState owns one of
/// these rather than accumulating region-specific fields at its top level.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShellPresentationState {
    left_panel: RegionCollapseState,
}

impl ResizeUpdate {
    fn inert() -> Self {
        Self {
            decision: ResizeDecision::Inert,
            mark_persistence_dirty: false,
            request_pty_resize: false,
        }
    }

    fn committed(tracks: [u16; 2]) -> Self {
        Self {
            decision: ResizeDecision::Committed(tracks),
            mark_persistence_dirty: true,
            request_pty_resize: true,
        }
    }

    fn cancelled(tracks: [u16; 2]) -> Self {
        Self {
            decision: ResizeDecision::Cancelled(tracks),
            mark_persistence_dirty: false,
            request_pty_resize: false,
        }
    }

    pub(crate) const fn decision(self) -> ResizeDecision {
        self.decision
    }

    pub(crate) const fn marks_persistence_dirty(self) -> bool {
        self.mark_persistence_dirty
    }

    pub(crate) const fn requests_pty_resize(self) -> bool {
        self.request_pty_resize
    }
}

impl CollapseUpdate {
    const fn inert() -> Self {
        Self {
            decision: CollapseDecision::Inert,
            mark_persistence_dirty: false,
        }
    }

    const fn collapsed(restore_width: u16) -> Self {
        Self {
            decision: CollapseDecision::Collapsed { restore_width },
            mark_persistence_dirty: true,
        }
    }

    const fn expanded(width: u16) -> Self {
        Self {
            decision: CollapseDecision::Expanded { width },
            mark_persistence_dirty: true,
        }
    }

    pub(crate) const fn decision(self) -> CollapseDecision {
        self.decision
    }

    pub(crate) const fn marks_persistence_dirty(self) -> bool {
        self.mark_persistence_dirty
    }
}

// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
impl ScrollViewportId {
    pub(crate) const fn new(region: RegionId, slot: u8) -> Self {
        Self { region, slot }
    }
}

// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
impl ScrollOffset {
    pub(crate) const fn new(horizontal: usize, vertical: usize) -> Self {
        Self {
            horizontal,
            vertical,
        }
    }
}

// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
impl ScrollViewportMetrics {
    pub(crate) const fn new(
        viewport_width: u16,
        viewport_height: u16,
        content_width: usize,
        content_height: usize,
    ) -> Self {
        Self {
            viewport_width,
            viewport_height,
            content_width,
            content_height,
        }
    }

    fn max_offset(self, axis: ScrollAxis) -> usize {
        let (viewport, content) = match axis {
            ScrollAxis::Horizontal => (usize::from(self.viewport_width), self.content_width),
            ScrollAxis::Vertical => (usize::from(self.viewport_height), self.content_height),
        };
        if viewport == 0 {
            0
        } else {
            content.saturating_sub(viewport)
        }
    }
}

// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
impl ScrollViewportState {
    pub(crate) const fn new(id: ScrollViewportId) -> Self {
        Self::with_offset(id, ScrollOffset::new(0, 0))
    }

    const fn with_offset(id: ScrollViewportId, offset: ScrollOffset) -> Self {
        Self { id, offset }
    }

    pub(crate) fn reconcile(&mut self, metrics: ScrollViewportMetrics) -> bool {
        let before = self.offset;
        self.offset.horizontal = self
            .offset
            .horizontal
            .min(metrics.max_offset(ScrollAxis::Horizontal));
        self.offset.vertical = self
            .offset
            .vertical
            .min(metrics.max_offset(ScrollAxis::Vertical));
        self.offset != before
    }

    pub(crate) fn scroll_by(
        &mut self,
        axis: ScrollAxis,
        delta: i32,
        metrics: ScrollViewportMetrics,
    ) -> ScrollDecision {
        let before = self.offset;
        self.reconcile(metrics);
        let max_offset = metrics.max_offset(axis);
        let offset = match axis {
            ScrollAxis::Horizontal => &mut self.offset.horizontal,
            ScrollAxis::Vertical => &mut self.offset.vertical,
        };
        *offset = if delta < 0 {
            offset.saturating_sub(delta.unsigned_abs() as usize)
        } else {
            offset.saturating_add(delta as usize).min(max_offset)
        };
        ScrollDecision::Consumed {
            changed: self.offset != before,
        }
    }
}

// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
impl ScrollOwner {
    pub(crate) const fn new(id: ScrollViewportId, metrics: ScrollViewportMetrics) -> Self {
        Self { id, metrics }
    }
}

/// Route to exactly the final (topmost) owner. A stale top owner is consumed
/// inert so background surfaces can never inherit its input.
// SF3.2 establishes the pure primitive before SF4 attaches real input-router consumers.
#[allow(dead_code)]
pub(crate) fn route_scroll_to_topmost(
    states: &mut [ScrollViewportState],
    owners: &[ScrollOwner],
    axis: ScrollAxis,
    delta: i32,
) -> ScrollDecision {
    let Some(owner) = owners.last() else {
        return ScrollDecision::Unhandled;
    };
    let Some(state) = states.iter_mut().find(|state| state.id == owner.id) else {
        return ScrollDecision::Consumed { changed: false };
    };
    state.scroll_by(axis, delta, owner.metrics)
}

impl RegionCollapseState {
    pub(crate) const fn expanded(region: RegionId, width: u16) -> Self {
        Self {
            region,
            restore_width: width,
            collapsed: false,
            revision: 0,
        }
    }

    pub(crate) const fn collapsed(region: RegionId, restore_width: u16) -> Self {
        Self {
            region,
            restore_width,
            collapsed: true,
            revision: 0,
        }
    }

    pub(crate) const fn visible_width(self) -> u16 {
        if self.collapsed {
            0
        } else {
            self.restore_width
        }
    }

    pub(crate) fn collapse(&mut self, committed_width: u16) -> CollapseUpdate {
        if self.collapsed || is_mandatory_stage(self.region) {
            return CollapseUpdate::inert();
        }
        let Some(revision) = self.revision.checked_add(1) else {
            return CollapseUpdate::inert();
        };

        self.restore_width = committed_width;
        self.collapsed = true;
        self.revision = revision;
        CollapseUpdate::collapsed(committed_width)
    }

    pub(crate) fn expand(&mut self, total: u16, bounds: ResizeBounds) -> CollapseUpdate {
        if !self.collapsed {
            return CollapseUpdate::inert();
        }
        let Some([width, _]) = normalized_tracks(total, i32::from(self.restore_width), bounds)
        else {
            return CollapseUpdate::inert();
        };
        let Some(revision) = self.revision.checked_add(1) else {
            return CollapseUpdate::inert();
        };

        self.restore_width = width;
        self.collapsed = false;
        self.revision = revision;
        CollapseUpdate::expanded(width)
    }
}

impl ShellPresentationState {
    pub(crate) const fn new(left_panel_width: u16) -> Self {
        Self::from_restored_left_panel(left_panel_width, false)
    }

    pub(crate) const fn from_restored_left_panel(left_panel_width: u16, collapsed: bool) -> Self {
        Self {
            left_panel: if collapsed {
                RegionCollapseState::collapsed(RegionId::LeftPanel, left_panel_width)
            } else {
                RegionCollapseState::expanded(RegionId::LeftPanel, left_panel_width)
            },
        }
    }

    pub(crate) fn collapse_left_panel(&mut self, committed_width: u16) -> CollapseUpdate {
        self.left_panel.collapse(committed_width)
    }

    pub(crate) fn expand_left_panel(&mut self, total: u16, bounds: ResizeBounds) -> CollapseUpdate {
        self.left_panel.expand(total, bounds)
    }

    pub(crate) const fn left_panel_restore_width(&self) -> u16 {
        self.left_panel.restore_width
    }

    pub(crate) const fn left_panel_collapsed(&self) -> bool {
        self.left_panel.collapsed
    }

    pub(crate) const fn left_panel_collapse_revision(&self) -> u64 {
        self.left_panel.revision
    }
}

const fn is_mandatory_stage(region: RegionId) -> bool {
    matches!(region, RegionId::CenterContent | RegionId::WorkspaceStage)
}

/// Aggregate transient interaction state. It is intentionally absent from
/// snapshots and owns no runtime or I/O handle.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ShellInteractionState {
    resize: Option<ResizeTransaction>,
}

impl ShellInteractionState {
    pub(crate) fn begin_resize(&mut self, transaction: ResizeTransaction) {
        self.resize = Some(transaction);
    }

    pub(crate) fn resize_active(&self) -> bool {
        self.resize.is_some()
    }

    pub(crate) fn resize_generation(&self) -> Option<u64> {
        self.resize
            .as_ref()
            .map(|transaction| transaction.view_generation)
    }

    pub(crate) fn resize_preview_tracks(&self) -> Option<[u16; 2]> {
        self.resize
            .as_ref()
            .map(|transaction| transaction.preview_tracks)
    }

    pub(crate) fn resize_original_total(&self) -> Option<u16> {
        let transaction = self.resize.as_ref()?;
        transaction.original_tracks[0].checked_add(transaction.original_tracks[1])
    }

    pub(crate) fn preview_resize(&mut self, pointer: Position, bounds: ResizeBounds) -> bool {
        self.resize
            .as_mut()
            .is_some_and(|transaction| transaction.preview(pointer, bounds))
    }

    pub(crate) fn preview_keyboard_resize_step(&mut self, step: i16, bounds: ResizeBounds) -> bool {
        self.resize
            .as_mut()
            .is_some_and(|transaction| transaction.preview_keyboard_step(step, bounds))
    }

    pub(crate) fn rebase_resize_generation(&mut self, generation: u64) {
        if let Some(transaction) = self.resize.as_mut() {
            transaction.view_generation = generation;
        }
    }

    pub(crate) fn commit_resize(&mut self, generation: u64) -> ResizeUpdate {
        ResizeTransaction::commit(&mut self.resize, generation)
    }

    pub(crate) fn cancel_resize(&mut self) -> ResizeUpdate {
        ResizeTransaction::cancel(&mut self.resize)
    }

    pub(crate) fn terminal_resize(&mut self, new_total: u16, bounds: ResizeBounds) -> ResizeUpdate {
        ResizeTransaction::terminal_resize(&mut self.resize, new_total, bounds)
    }
}

impl ResizeTransaction {
    pub(crate) fn begin(
        divider: DividerId,
        view_generation: u64,
        pointer_origin: Position,
        original_tracks: [u16; 2],
    ) -> Option<Self> {
        let _total = original_tracks[0].checked_add(original_tracks[1])?;
        if original_tracks.contains(&0) {
            return None;
        }
        Some(Self {
            divider,
            view_generation,
            pointer_origin,
            original_tracks,
            preview_tracks: original_tracks,
        })
    }

    /// Update only the transient tracks. No effect adapter is available here,
    /// so preview cannot dirty persistence or resize a terminal runtime.
    pub(crate) fn preview(&mut self, pointer: Position, bounds: ResizeBounds) -> bool {
        let (pointer_now, pointer_origin) = match self.divider.axis {
            ShellDirection::Horizontal => (pointer.x, self.pointer_origin.x),
            ShellDirection::Vertical => (pointer.y, self.pointer_origin.y),
        };
        self.preview_delta(i32::from(pointer_now) - i32::from(pointer_origin), bounds)
    }

    /// Keyboard input supplies the same original-relative delta as pointer
    /// preview, so both paths share identical bounds and normalization.
    pub(crate) fn preview_keyboard_delta(&mut self, delta: i16, bounds: ResizeBounds) -> bool {
        self.preview_delta(i32::from(delta), bounds)
    }

    /// Apply one keyboard step relative to the current preview. Repeated key
    /// presses therefore remain in the same bounded transaction while pointer
    /// preview can keep using its original-relative coordinate delta.
    fn preview_keyboard_step(&mut self, step: i16, bounds: ResizeBounds) -> bool {
        let Some(total) = self.original_tracks[0].checked_add(self.original_tracks[1]) else {
            return false;
        };
        let desired = i32::from(self.preview_tracks[0]) + i32::from(step);
        let Some(tracks) = normalized_tracks(total, desired, bounds) else {
            return false;
        };
        self.preview_tracks = tracks;
        true
    }

    fn preview_delta(&mut self, delta: i32, bounds: ResizeBounds) -> bool {
        let Some(total) = self.original_tracks[0].checked_add(self.original_tracks[1]) else {
            return false;
        };
        let desired = i32::from(self.original_tracks[0]) + delta;
        let Some(tracks) = normalized_tracks(total, desired, bounds) else {
            return false;
        };
        self.preview_tracks = tracks;
        true
    }

    pub(crate) fn commit(capture: &mut Option<Self>, current_generation: u64) -> ResizeUpdate {
        let Some(transaction) = capture.as_ref() else {
            return ResizeUpdate::inert();
        };
        if transaction.view_generation != current_generation {
            return ResizeUpdate::inert();
        }

        let Some(transaction) = capture.take() else {
            return ResizeUpdate::inert();
        };
        if transaction.preview_tracks == transaction.original_tracks {
            return ResizeUpdate::inert();
        }
        ResizeUpdate::committed(transaction.preview_tracks)
    }

    pub(crate) fn cancel(capture: &mut Option<Self>) -> ResizeUpdate {
        capture
            .take()
            .map_or_else(ResizeUpdate::inert, |transaction| {
                ResizeUpdate::cancelled(transaction.original_tracks)
            })
    }

    pub(crate) fn terminal_resize(
        capture: &mut Option<Self>,
        new_total: u16,
        bounds: ResizeBounds,
    ) -> ResizeUpdate {
        let Some(transaction) = capture.take() else {
            return ResizeUpdate::inert();
        };
        let tracks =
            normalized_tracks(new_total, i32::from(transaction.original_tracks[0]), bounds)
                .unwrap_or(transaction.original_tracks);
        ResizeUpdate::cancelled(tracks)
    }

    pub(crate) fn reset_preferred(
        current: [u16; 2],
        preferred: u16,
        bounds: ResizeBounds,
    ) -> ResizeUpdate {
        let Some(total) = current[0].checked_add(current[1]) else {
            return ResizeUpdate::inert();
        };
        let Some(tracks) = normalized_tracks(total, i32::from(preferred), bounds) else {
            return ResizeUpdate::inert();
        };
        if tracks == current {
            ResizeUpdate::inert()
        } else {
            ResizeUpdate::committed(tracks)
        }
    }
}

fn normalized_tracks(total: u16, desired_leading: i32, bounds: ResizeBounds) -> Option<[u16; 2]> {
    let leading_lower = bounds
        .leading_min
        .max(total.saturating_sub(bounds.trailing_max));
    let leading_upper = bounds
        .leading_max
        .min(total.saturating_sub(bounds.trailing_min));
    if leading_lower > leading_upper {
        return None;
    }

    let leading = desired_leading.clamp(i32::from(leading_lower), i32::from(leading_upper)) as u16;
    Some([leading, total.saturating_sub(leading)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn divider_down_captures_original_constraints() {
        let divider = DividerId::new(
            RegionId::LeftPanel,
            RegionId::WorkspaceStage,
            ShellDirection::Horizontal,
        )
        .expect("distinct adjacent regions form a divider");

        let transaction = ResizeTransaction::begin(divider, 7, Position::new(25, 5), [26, 54])
            .expect("non-zero normalized tracks start a transaction");

        assert_eq!(
            (
                transaction.divider,
                transaction.view_generation,
                transaction.pointer_origin,
                transaction.original_tracks,
                transaction.preview_tracks,
            ),
            (divider, 7, Position::new(25, 5), [26, 54], [26, 54],)
        );
    }

    #[test]
    fn drag_preview_clamps_without_dirty_or_pty_resize() {
        let divider = DividerId::new(
            RegionId::LeftPanel,
            RegionId::WorkspaceStage,
            ShellDirection::Horizontal,
        )
        .expect("distinct adjacent regions form a divider");
        let mut transaction = ResizeTransaction::begin(divider, 7, Position::new(25, 5), [26, 54])
            .expect("non-zero normalized tracks start a transaction");
        let bounds = ResizeBounds::new(4, 40, 1, 80).expect("ordered track bounds");
        let effects = TestResizeEffects::default();

        assert!(transaction.preview(Position::new(99, 5), bounds));

        assert_eq!(
            (
                transaction.preview_tracks,
                effects.persistence_dirty,
                effects.pty_resize_requests,
            ),
            ([40, 40], 0, 0)
        );
    }

    #[test]
    fn drag_commit_marks_dirty_once_and_requests_at_most_one_resize() {
        let mut capture = Some(transaction());
        capture
            .as_mut()
            .expect("fixture owns capture")
            .preview(Position::new(29, 5), bounds());
        let mut effects = TestResizeEffects::default();

        let decision = commit_resize_for_test(&mut capture, 7, &mut effects);

        assert_eq!(
            (decision, capture, effects),
            (
                ResizeDecision::Committed([30, 50]),
                None,
                TestResizeEffects {
                    persistence_dirty: 1,
                    pty_resize_requests: 1,
                },
            )
        );
    }

    #[test]
    fn divider_double_click_resets_to_preferred_once() {
        let mut effects = TestResizeEffects::default();

        let decision = reset_preferred_for_test([30, 50], 26, bounds(), &mut effects);

        assert_eq!(
            (decision, effects),
            (
                ResizeDecision::Committed([26, 54]),
                TestResizeEffects {
                    persistence_dirty: 1,
                    pty_resize_requests: 1,
                },
            )
        );
    }

    #[test]
    fn keyboard_resize_uses_same_clamp_preview_and_commit_path() {
        let mut mouse = transaction();
        let mut keyboard = transaction();

        assert!(mouse.preview(Position::new(99, 5), bounds()));
        keyboard_preview_for_test(&mut keyboard, 74, bounds());

        assert_eq!(keyboard.preview_tracks, mouse.preview_tracks);
    }

    #[test]
    fn escape_restores_original_constraints() {
        let mut capture = Some(transaction());
        capture
            .as_mut()
            .expect("fixture owns capture")
            .preview(Position::new(99, 5), bounds());
        let mut effects = TestResizeEffects::default();

        let decision = cancel_resize_for_test(&mut capture, &mut effects);

        assert_eq!(
            (decision, capture, effects),
            (
                ResizeDecision::Cancelled([26, 54]),
                None,
                TestResizeEffects::default(),
            )
        );
    }

    #[test]
    fn terminal_resize_cancels_and_recomputes_from_original() {
        let mut capture = Some(transaction());
        capture
            .as_mut()
            .expect("fixture owns capture")
            .preview(Position::new(99, 5), bounds());
        let mut effects = TestResizeEffects::default();

        let decision = terminal_resize_for_test(&mut capture, 60, bounds(), &mut effects);

        assert_eq!(
            (decision, capture, effects),
            (
                ResizeDecision::Cancelled([26, 34]),
                None,
                TestResizeEffects::default(),
            )
        );
    }

    #[test]
    fn stale_divider_generation_is_consumed_inert() {
        let mut capture = Some(transaction());
        let before = capture.clone();
        let mut effects = TestResizeEffects::default();

        let decision = commit_resize_for_test(&mut capture, 8, &mut effects);

        assert_eq!(
            (decision, capture, effects),
            (ResizeDecision::Inert, before, TestResizeEffects::default(),)
        );
    }

    #[test]
    fn mouse_up_without_capture_is_inert() {
        let mut capture = None;
        let mut effects = TestResizeEffects::default();

        let decision = commit_resize_for_test(&mut capture, 7, &mut effects);

        assert_eq!(
            (decision, capture, effects),
            (ResizeDecision::Inert, None, TestResizeEffects::default(),)
        );
    }

    #[test]
    fn mouse_up_without_preview_clears_capture_without_effect() {
        let mut capture = Some(transaction());
        let mut effects = TestResizeEffects::default();

        let decision = commit_resize_for_test(&mut capture, 7, &mut effects);

        assert_eq!(
            (decision, capture, effects),
            (ResizeDecision::Inert, None, TestResizeEffects::default(),)
        );
    }

    #[test]
    fn collapse_remembers_last_committed_width() {
        let mut state = TestCollapseState::expanded(RegionId::LeftPanel, 32);
        let mut effects = TestCollapseEffects::default();

        let decision = collapse_for_test(&mut state, 32, &mut effects);

        assert_eq!(
            (
                decision,
                state.visible_width(),
                state.restore_width,
                state.collapsed,
                state.revision,
                effects,
            ),
            (
                TestCollapseDecision::Collapsed { restore_width: 32 },
                0,
                32,
                true,
                1,
                TestCollapseEffects {
                    persistence_dirty: 1,
                },
            )
        );
    }

    #[test]
    fn repeated_collapse_is_inert() {
        let mut state = TestCollapseState::expanded(RegionId::LeftPanel, 32);
        let mut effects = TestCollapseEffects::default();
        collapse_for_test(&mut state, 32, &mut effects);

        let decision = collapse_for_test(&mut state, 0, &mut effects);

        assert_eq!(decision, TestCollapseDecision::Inert);
        assert_eq!((state.revision, effects.persistence_dirty), (1, 1));
        assert_eq!((state.visible_width(), state.restore_width), (0, 32));
    }

    #[test]
    fn expand_clamps_restore_width_to_current_bounds() {
        let mut state = TestCollapseState::collapsed(RegionId::LeftPanel, 32);
        let bounds = ResizeBounds::new(18, 36, 10, 40).expect("ordered track bounds");
        let mut effects = TestCollapseEffects::default();

        let decision = expand_for_test(&mut state, 36, bounds, &mut effects);

        assert_eq!(
            (
                decision,
                state.visible_width(),
                state.restore_width,
                state.collapsed,
                state.revision,
                effects,
            ),
            (
                TestCollapseDecision::Expanded { width: 26 },
                26,
                26,
                false,
                1,
                TestCollapseEffects {
                    persistence_dirty: 1,
                },
            )
        );
    }

    #[test]
    fn stage_collapse_is_rejected() {
        for region in [RegionId::WorkspaceStage, RegionId::CenterContent] {
            let mut state = TestCollapseState::expanded(region, 80);
            let mut effects = TestCollapseEffects::default();

            let decision = collapse_for_test(&mut state, 80, &mut effects);

            assert_eq!(
                (
                    decision,
                    state.visible_width(),
                    state.restore_width,
                    state.collapsed,
                    state.revision,
                    effects,
                ),
                (
                    TestCollapseDecision::Inert,
                    80,
                    80,
                    false,
                    0,
                    TestCollapseEffects::default(),
                )
            );
        }
    }

    #[test]
    fn empty_optional_region_collapses_to_zero() {
        let mut state = TestCollapseState::expanded(RegionId::RightPanel, 0);
        let mut effects = TestCollapseEffects::default();

        let decision = collapse_for_test(&mut state, 0, &mut effects);

        assert_eq!(
            (
                decision,
                state.visible_width(),
                state.restore_width,
                state.collapsed,
                state.revision,
                effects,
            ),
            (
                TestCollapseDecision::Collapsed { restore_width: 0 },
                0,
                0,
                true,
                1,
                TestCollapseEffects {
                    persistence_dirty: 1,
                },
            )
        );
    }

    #[test]
    fn collapse_revision_exhaustion_is_inert() {
        let mut state = TestCollapseState::expanded(RegionId::LeftPanel, 32);
        state.revision = u64::MAX;
        let mut effects = TestCollapseEffects::default();

        let decision = collapse_for_test(&mut state, 32, &mut effects);

        assert_eq!(decision, TestCollapseDecision::Inert);
        assert_eq!(
            (
                state.visible_width(),
                state.restore_width,
                state.collapsed,
                state.revision,
                effects,
            ),
            (32, 32, false, u64::MAX, TestCollapseEffects::default(),)
        );
    }

    #[test]
    fn expand_without_feasible_tracks_is_inert() {
        let mut state = TestCollapseState::collapsed(RegionId::LeftPanel, 32);
        let bounds = ResizeBounds::new(30, 40, 20, 30).expect("ordered track bounds");
        let mut effects = TestCollapseEffects::default();

        let decision = expand_for_test(&mut state, 40, bounds, &mut effects);

        assert_eq!(decision, TestCollapseDecision::Inert);
        assert_eq!(
            (
                state.visible_width(),
                state.restore_width,
                state.collapsed,
                state.revision,
                effects,
            ),
            (0, 32, true, 0, TestCollapseEffects::default(),)
        );
    }

    #[test]
    fn horizontal_and_vertical_offsets_clamp_independently() {
        let mut state =
            TestScrollViewportState::new(scroll_viewport_id(RegionId::WorkspaceStage, 0));
        let metrics = TestScrollViewportMetrics::new(10, 4, 25, 9);

        assert_eq!(
            scroll_for_test(&mut state, TestScrollAxis::Horizontal, i32::MAX, metrics,),
            TestScrollDecision::Consumed { changed: true }
        );
        assert_eq!(state.offset, TestScrollOffset::new(15, 0));

        assert_eq!(
            scroll_for_test(&mut state, TestScrollAxis::Vertical, i32::MAX, metrics,),
            TestScrollDecision::Consumed { changed: true }
        );
        assert_eq!(state.offset, TestScrollOffset::new(15, 5));

        assert_eq!(
            scroll_for_test(&mut state, TestScrollAxis::Horizontal, i32::MIN, metrics,),
            TestScrollDecision::Consumed { changed: true }
        );
        assert_eq!(state.offset, TestScrollOffset::new(0, 5));
    }

    #[test]
    fn content_shrink_clamps_stale_scroll_offset() {
        let mut state = TestScrollViewportState::with_offset(
            scroll_viewport_id(RegionId::LeftPanel, 0),
            TestScrollOffset::new(15, 5),
        );
        let smaller = TestScrollViewportMetrics::new(10, 4, 12, 3);

        assert!(reconcile_scroll_for_test(&mut state, smaller));
        assert_eq!(state.offset, TestScrollOffset::new(2, 0));
        assert!(!reconcile_scroll_for_test(&mut state, smaller));
    }

    #[test]
    fn zero_area_viewport_consumes_without_mutation() {
        let mut state = TestScrollViewportState::new(scroll_viewport_id(RegionId::RightPanel, 0));
        let zero_area = TestScrollViewportMetrics::new(0, 0, 100, 100);

        let decision = scroll_for_test(&mut state, TestScrollAxis::Vertical, 3, zero_area);

        assert_eq!(decision, TestScrollDecision::Consumed { changed: false });
        assert_eq!(state.offset, TestScrollOffset::new(0, 0));
    }

    #[test]
    fn scroll_changes_only_topmost_owning_viewport() {
        let bottom_id = scroll_viewport_id(RegionId::WorkspaceStage, 0);
        let top_id = scroll_viewport_id(RegionId::RightPanel, 0);
        let mut states = [
            TestScrollViewportState::new(bottom_id),
            TestScrollViewportState::new(top_id),
        ];
        let metrics = TestScrollViewportMetrics::new(10, 4, 10, 12);
        let owners = [
            TestScrollOwner::new(bottom_id, metrics),
            TestScrollOwner::new(top_id, metrics),
        ];

        let decision = route_scroll_for_test(&mut states, &owners, TestScrollAxis::Vertical, 3);

        assert_eq!(decision, TestScrollDecision::Consumed { changed: true });
        assert_eq!(states[0].offset, TestScrollOffset::new(0, 0));
        assert_eq!(states[1].offset, TestScrollOffset::new(0, 3));
    }

    #[test]
    fn topmost_scroll_boundary_does_not_fall_through() {
        let bottom_id = scroll_viewport_id(RegionId::WorkspaceStage, 0);
        let top_id = scroll_viewport_id(RegionId::RightPanel, 0);
        let mut states = [
            TestScrollViewportState::new(bottom_id),
            TestScrollViewportState::with_offset(top_id, TestScrollOffset::new(0, 8)),
        ];
        let metrics = TestScrollViewportMetrics::new(10, 4, 10, 12);
        let owners = [
            TestScrollOwner::new(bottom_id, metrics),
            TestScrollOwner::new(top_id, metrics),
        ];

        let decision = route_scroll_for_test(&mut states, &owners, TestScrollAxis::Vertical, 3);

        assert_eq!(decision, TestScrollDecision::Consumed { changed: false });
        assert_eq!(states[0].offset, TestScrollOffset::new(0, 0));
        assert_eq!(states[1].offset, TestScrollOffset::new(0, 8));
    }

    #[test]
    fn stale_topmost_scroll_owner_is_consumed_inert() {
        let bottom_id = scroll_viewport_id(RegionId::WorkspaceStage, 0);
        let stale_top_id = scroll_viewport_id(RegionId::RightPanel, 0);
        let mut states = [TestScrollViewportState::new(bottom_id)];
        let metrics = TestScrollViewportMetrics::new(10, 4, 10, 12);
        let owners = [
            TestScrollOwner::new(bottom_id, metrics),
            TestScrollOwner::new(stale_top_id, metrics),
        ];

        let decision = route_scroll_for_test(&mut states, &owners, TestScrollAxis::Vertical, 3);

        assert_eq!(decision, TestScrollDecision::Consumed { changed: false });
        assert_eq!(states[0].offset, TestScrollOffset::new(0, 0));
    }

    fn divider() -> DividerId {
        DividerId::new(
            RegionId::LeftPanel,
            RegionId::WorkspaceStage,
            ShellDirection::Horizontal,
        )
        .expect("distinct adjacent regions form a divider")
    }

    fn transaction() -> ResizeTransaction {
        ResizeTransaction::begin(divider(), 7, Position::new(25, 5), [26, 54])
            .expect("non-zero normalized tracks start a transaction")
    }

    fn bounds() -> ResizeBounds {
        ResizeBounds::new(4, 40, 1, 80).expect("ordered track bounds")
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct TestResizeEffects {
        persistence_dirty: usize,
        pty_resize_requests: usize,
    }

    type TestCollapseState = RegionCollapseState;
    type TestCollapseDecision = CollapseDecision;

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct TestCollapseEffects {
        persistence_dirty: usize,
    }

    type TestScrollAxis = ScrollAxis;
    type TestScrollOffset = ScrollOffset;
    type TestScrollViewportId = ScrollViewportId;
    type TestScrollViewportMetrics = ScrollViewportMetrics;
    type TestScrollViewportState = ScrollViewportState;
    type TestScrollOwner = ScrollOwner;
    type TestScrollDecision = ScrollDecision;

    const fn scroll_viewport_id(region: RegionId, slot: u8) -> TestScrollViewportId {
        ScrollViewportId::new(region, slot)
    }

    fn scroll_for_test(
        state: &mut TestScrollViewportState,
        axis: TestScrollAxis,
        delta: i32,
        metrics: TestScrollViewportMetrics,
    ) -> TestScrollDecision {
        state.scroll_by(axis, delta, metrics)
    }

    fn reconcile_scroll_for_test(
        state: &mut TestScrollViewportState,
        metrics: TestScrollViewportMetrics,
    ) -> bool {
        state.reconcile(metrics)
    }

    fn route_scroll_for_test(
        states: &mut [TestScrollViewportState],
        owners: &[TestScrollOwner],
        axis: TestScrollAxis,
        delta: i32,
    ) -> TestScrollDecision {
        route_scroll_to_topmost(states, owners, axis, delta)
    }

    fn collapse_for_test(
        state: &mut TestCollapseState,
        committed_width: u16,
        effects: &mut TestCollapseEffects,
    ) -> TestCollapseDecision {
        let update = state.collapse(committed_width);
        effects.persistence_dirty += usize::from(update.marks_persistence_dirty());
        update.decision()
    }

    fn expand_for_test(
        state: &mut TestCollapseState,
        total: u16,
        bounds: ResizeBounds,
        effects: &mut TestCollapseEffects,
    ) -> TestCollapseDecision {
        let update = state.expand(total, bounds);
        effects.persistence_dirty += usize::from(update.marks_persistence_dirty());
        update.decision()
    }

    fn commit_resize_for_test(
        capture: &mut Option<ResizeTransaction>,
        current_generation: u64,
        effects: &mut TestResizeEffects,
    ) -> ResizeDecision {
        apply_update_for_test(
            ResizeTransaction::commit(capture, current_generation),
            effects,
        )
    }

    fn reset_preferred_for_test(
        current: [u16; 2],
        preferred: u16,
        bounds: ResizeBounds,
        effects: &mut TestResizeEffects,
    ) -> ResizeDecision {
        apply_update_for_test(
            ResizeTransaction::reset_preferred(current, preferred, bounds),
            effects,
        )
    }

    fn keyboard_preview_for_test(
        transaction: &mut ResizeTransaction,
        delta: i16,
        bounds: ResizeBounds,
    ) {
        transaction.preview_keyboard_delta(delta, bounds);
    }

    fn cancel_resize_for_test(
        capture: &mut Option<ResizeTransaction>,
        effects: &mut TestResizeEffects,
    ) -> ResizeDecision {
        apply_update_for_test(ResizeTransaction::cancel(capture), effects)
    }

    fn terminal_resize_for_test(
        capture: &mut Option<ResizeTransaction>,
        new_total: u16,
        bounds: ResizeBounds,
        effects: &mut TestResizeEffects,
    ) -> ResizeDecision {
        apply_update_for_test(
            ResizeTransaction::terminal_resize(capture, new_total, bounds),
            effects,
        )
    }

    fn apply_update_for_test(
        update: ResizeUpdate,
        effects: &mut TestResizeEffects,
    ) -> ResizeDecision {
        effects.persistence_dirty += usize::from(update.mark_persistence_dirty);
        effects.pty_resize_requests += usize::from(update.request_pty_resize);
        update.decision
    }
}
