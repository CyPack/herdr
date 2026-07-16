#[cfg(test)]
mod tests {
    use ratatui::layout::Position;

    use super::super::RegionId;

    #[test]
    fn divider_down_captures_original_constraints() {
        let divider = TestDividerId {
            leading: RegionId::LeftPanel,
            trailing: RegionId::WorkspaceStage,
        };

        let transaction = begin_resize_for_test(divider, 7, Position::new(25, 5), [26, 54]);

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
        let mut transaction = TestResizeTransaction {
            divider: TestDividerId {
                leading: RegionId::LeftPanel,
                trailing: RegionId::WorkspaceStage,
            },
            view_generation: 7,
            pointer_origin: Position::new(25, 5),
            original_tracks: [26, 54],
            preview_tracks: [26, 54],
        };
        let mut effects = TestResizeEffects::default();

        preview_resize_for_test(&mut transaction, Position::new(99, 5), 4, 40, &mut effects);

        assert_eq!(
            (
                transaction.preview_tracks,
                effects.persistence_dirty,
                effects.pty_resize_requests,
            ),
            ([40, 40], 0, 0)
        );
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TestDividerId {
        leading: RegionId,
        trailing: RegionId,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct TestResizeTransaction {
        divider: TestDividerId,
        view_generation: u64,
        pointer_origin: Position,
        original_tracks: [u16; 2],
        preview_tracks: [u16; 2],
    }

    #[derive(Default)]
    struct TestResizeEffects {
        persistence_dirty: usize,
        pty_resize_requests: usize,
    }

    fn begin_resize_for_test(
        divider: TestDividerId,
        _view_generation: u64,
        _pointer_origin: Position,
        original_tracks: [u16; 2],
    ) -> TestResizeTransaction {
        // RED-only seam: SF3.1 must capture the current generation, pointer,
        // and committed normalized tracks instead of manufacturing defaults.
        let total = original_tracks[0].saturating_add(original_tracks[1]);
        TestResizeTransaction {
            divider,
            view_generation: 0,
            pointer_origin: Position::new(0, 0),
            original_tracks: [0, total],
            preview_tracks: [0, total],
        }
    }

    fn preview_resize_for_test(
        transaction: &mut TestResizeTransaction,
        pointer: Position,
        _leading_min: u16,
        _leading_max: u16,
        effects: &mut TestResizeEffects,
    ) {
        // RED-only seam: SF3.1 replaces this eager, unclamped mutation with a
        // pure bounded preview that emits no persistence or PTY effects.
        let total = transaction.original_tracks[0].saturating_add(transaction.original_tracks[1]);
        let leading = pointer.x.min(total);
        transaction.preview_tracks = [leading, total.saturating_sub(leading)];
        effects.persistence_dirty += 1;
        effects.pty_resize_requests += 1;
    }
}
