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
        let total = self.original_tracks[0] + self.original_tracks[1];
        let leading_lower = bounds
            .leading_min
            .max(total.saturating_sub(bounds.trailing_max));
        let leading_upper = bounds
            .leading_max
            .min(total.saturating_sub(bounds.trailing_min));
        if leading_lower > leading_upper {
            return false;
        }

        let (pointer_now, pointer_origin) = match self.divider.axis {
            ShellDirection::Horizontal => (pointer.x, self.pointer_origin.x),
            ShellDirection::Vertical => (pointer.y, self.pointer_origin.y),
        };
        let desired =
            i32::from(self.original_tracks[0]) + i32::from(pointer_now) - i32::from(pointer_origin);
        let leading = desired.clamp(i32::from(leading_lower), i32::from(leading_upper)) as u16;
        self.preview_tracks = [leading, total.saturating_sub(leading)];
        true
    }
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
                TestResizeDecision::Committed([30, 50]),
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
                TestResizeDecision::Committed([26, 54]),
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
                TestResizeDecision::Cancelled([26, 54]),
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
                TestResizeDecision::Cancelled([26, 34]),
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
            (
                TestResizeDecision::Inert,
                before,
                TestResizeEffects::default(),
            )
        );
    }

    #[test]
    fn mouse_up_without_capture_is_inert() {
        let mut capture = None;
        let mut effects = TestResizeEffects::default();

        let decision = commit_resize_for_test(&mut capture, 7, &mut effects);

        assert_eq!(
            (decision, capture, effects),
            (
                TestResizeDecision::Inert,
                None,
                TestResizeEffects::default(),
            )
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

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TestResizeDecision {
        Inert,
        Committed([u16; 2]),
        Cancelled([u16; 2]),
    }

    #[derive(Clone, Debug, Default, PartialEq, Eq)]
    struct TestResizeEffects {
        persistence_dirty: usize,
        pty_resize_requests: usize,
    }

    fn commit_resize_for_test(
        capture: &mut Option<ResizeTransaction>,
        _current_generation: u64,
        effects: &mut TestResizeEffects,
    ) -> TestResizeDecision {
        // RED-only seam: SF3.1 must revalidate generation, consume exactly one
        // valid capture, and emit one bounded commit request.
        if let Some(transaction) = capture.take() {
            effects.persistence_dirty += 2;
            effects.pty_resize_requests += 2;
            TestResizeDecision::Committed(transaction.preview_tracks)
        } else {
            effects.persistence_dirty += 1;
            TestResizeDecision::Committed([0, 0])
        }
    }

    fn reset_preferred_for_test(
        current: [u16; 2],
        _preferred: u16,
        _bounds: ResizeBounds,
        _effects: &mut TestResizeEffects,
    ) -> TestResizeDecision {
        // RED-only seam: SF3.1 must route reset through the same normalized
        // commit boundary instead of retaining the current tracks.
        TestResizeDecision::Committed(current)
    }

    fn keyboard_preview_for_test(
        _transaction: &mut ResizeTransaction,
        _delta: i16,
        _bounds: ResizeBounds,
    ) {
        // RED-only seam: SF3.1 must reuse pointer preview clamping.
    }

    fn cancel_resize_for_test(
        capture: &mut Option<ResizeTransaction>,
        effects: &mut TestResizeEffects,
    ) -> TestResizeDecision {
        // RED-only seam: SF3.1 must restore the committed original with no
        // persistence or PTY effect.
        let preview = capture
            .take()
            .map(|transaction| transaction.preview_tracks)
            .unwrap_or_default();
        effects.persistence_dirty += 1;
        TestResizeDecision::Committed(preview)
    }

    fn terminal_resize_for_test(
        capture: &mut Option<ResizeTransaction>,
        _new_total: u16,
        _bounds: ResizeBounds,
        effects: &mut TestResizeEffects,
    ) -> TestResizeDecision {
        // RED-only seam: SF3.1 must cancel stale preview geometry and derive
        // the safe tracks from the original committed leading track.
        let preview = capture
            .as_ref()
            .map(|transaction| transaction.preview_tracks)
            .unwrap_or_default();
        effects.pty_resize_requests += 1;
        TestResizeDecision::Committed(preview)
    }
}
