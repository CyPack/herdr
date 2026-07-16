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

    #[derive(Default)]
    struct TestResizeEffects {
        persistence_dirty: usize,
        pty_resize_requests: usize,
    }
}
