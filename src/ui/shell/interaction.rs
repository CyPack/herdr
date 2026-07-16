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
