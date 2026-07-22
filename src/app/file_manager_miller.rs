//! Typed Trail column resize authority.
//!
//! Raw coordinates resolve against the immutable layout snapshot, then exact
//! Files generation, Trail path, and layout revision identities are
//! revalidated before a width preference may change.

impl crate::app::App {
    pub(super) fn queue_file_manager_trail_directory_preview_identity(
        &mut self,
        trail_col: usize,
        entry_index: usize,
        expected_path: &std::path::Path,
    ) -> bool {
        self.queue_file_manager_trail_directory_request(
            trail_col,
            entry_index,
            expected_path,
            true,
            crate::app::file_manager_io_worker::FileManagerTrailDestinationPolicy::PreserveMouseSelection,
        )
        .unwrap_or(false)
    }

    pub(super) fn queue_file_manager_trail_directory_activation_identity(
        &mut self,
        trail_col: usize,
        entry_index: usize,
        expected_path: &std::path::Path,
    ) -> bool {
        self.queue_file_manager_trail_directory_request(
            trail_col,
            entry_index,
            expected_path,
            false,
            crate::app::file_manager_io_worker::FileManagerTrailDestinationPolicy::FocusFirstActionable,
        )
        .unwrap_or(false)
    }

    fn queue_file_manager_trail_directory_request(
        &mut self,
        trail_col: usize,
        entry_index: usize,
        expected_path: &std::path::Path,
        preview_only: bool,
        destination_policy: crate::app::file_manager_io_worker::FileManagerTrailDestinationPolicy,
    ) -> Option<bool> {
        let files_generation = self.state.stage.active_instance_generation()?;
        let file_manager = self.state.file_manager.as_ref()?;
        match file_manager.trail_entry_is_directory(trail_col, entry_index, expected_path) {
            Some(false) => return None,
            None => return Some(false),
            Some(true) => {}
        }
        let source = crate::app::file_manager_io_worker::FileManagerIoSource::from_file_manager(
            file_manager,
        );
        let request = if preview_only {
            crate::app::file_manager_io_worker::FileManagerIoRequest::TrailPreview {
                files_generation,
                source,
                trail_col,
                entry_index,
                expected_path: expected_path.to_path_buf(),
                file_manager: Box::new(file_manager.clone()),
            }
        } else {
            crate::app::file_manager_io_worker::FileManagerIoRequest::TrailActivate {
                files_generation,
                source,
                trail_col,
                entry_index,
                expected_path: expected_path.to_path_buf(),
                destination_policy,
                file_manager: Box::new(file_manager.clone()),
            }
        };
        Some(matches!(
            self.file_manager_io_worker.submit(request),
            crate::app::file_manager_io_worker::FileManagerIoSubmit::Accepted { .. }
        ))
    }

    pub(super) fn execute_file_manager_navigation(
        &mut self,
        request: crate::fm::FmNavigationRequest,
    ) -> bool {
        let Some(files_generation) = self.state.stage.active_instance_generation() else {
            return false;
        };
        matches!(
            self.file_manager_io_worker.submit(
                crate::app::file_manager_io_worker::FileManagerIoRequest::Navigate {
                    files_generation,
                    request,
                },
            ),
            crate::app::file_manager_io_worker::FileManagerIoSubmit::Accepted { .. }
        )
    }

    pub(super) fn begin_miller_resize_capture(&mut self, column: u16, row: u16) -> bool {
        let transaction = {
            let Some(files_generation) = self.state.stage.active_instance_generation() else {
                return false;
            };
            let snapshot = &self.state.view.file_manager_miller;
            let mut hits = snapshot
                .dividers
                .iter()
                .filter(|divider| rect_contains(divider.rect, column, row));
            let Some(divider) = hits.next() else {
                return false;
            };
            if hits.next().is_some() {
                return true;
            }
            let Some(file_manager) = self.state.file_manager.as_ref() else {
                return true;
            };
            if snapshot.files_generation != Some(files_generation)
                || snapshot.model_revision != file_manager.miller.revision
            {
                return true;
            }
            let (Some(leading), Some(trailing)) = (
                snapshot.columns.get(divider.left_column),
                snapshot.columns.get(divider.right_column),
            ) else {
                return true;
            };
            if leading.projection_index.saturating_add(1) != trailing.projection_index {
                return true;
            }
            let (Some(leading_id), Some(trailing_id)) = (
                miller_resize_column_id(leading),
                miller_resize_column_id(trailing),
            ) else {
                return true;
            };
            let Some(divider_id) = crate::ui::shell::MillerDividerId::new(
                files_generation,
                snapshot.model_revision,
                leading_id,
                trailing_id,
                crate::ui::shell::ShellDirection::Horizontal,
            ) else {
                return true;
            };
            crate::ui::shell::ResizeTransaction::begin_miller(
                divider_id,
                snapshot.model_revision,
                ratatui::layout::Position::new(column, row),
                [leading.rect.width, trailing.rect.width],
            )
        };
        if let Some(transaction) = transaction {
            self.state.shell_interaction.begin_resize(transaction);
            crate::render_prof::event("fm.miller_resize.started");
        }
        true
    }

    pub(super) fn commit_miller_resize(&mut self) -> bool {
        let Some(divider) = self
            .state
            .shell_interaction
            .miller_resize_preview()
            .map(|(divider, _)| divider.clone())
        else {
            return false;
        };
        let Some(current_revision) = self
            .state
            .file_manager
            .as_ref()
            .map(|file_manager| file_manager.miller.revision)
        else {
            return false;
        };
        if divider.model_revision() != current_revision {
            let update = self.state.shell_interaction.cancel_resize();
            debug_assert!(!update.marks_persistence_dirty());
            debug_assert!(!update.requests_pty_resize());
            return false;
        }
        let update = self.state.shell_interaction.commit_resize(current_revision);
        let crate::ui::shell::ResizeDecision::Committed([leading_width, trailing_width]) =
            update.decision()
        else {
            return false;
        };
        let Some(files_generation) = self.state.stage.active_instance_generation() else {
            return false;
        };
        let Some(file_manager) = self.state.file_manager.as_mut() else {
            return false;
        };
        if divider.files_generation() != files_generation
            || divider.model_revision() != file_manager.miller.revision
            || divider.axis() != crate::ui::shell::ShellDirection::Horizontal
            || divider.leading().projection_index().saturating_add(1)
                != divider.trailing().projection_index()
            || !crate::ui::miller_resize_column_is_live(divider.leading(), file_manager)
            || !crate::ui::miller_resize_column_is_live(divider.trailing(), file_manager)
        {
            return false;
        }
        let crate::ui::shell::MillerResizeColumnId::Directory {
            directory: leading_directory,
            ..
        } = divider.leading()
        else {
            return false;
        };
        let trailing = match divider.trailing() {
            crate::ui::shell::MillerResizeColumnId::Directory { directory, .. } => {
                crate::fm::miller::MillerAdjacentWidthTarget::Directory(directory.clone())
            }
            crate::ui::shell::MillerResizeColumnId::Preview { .. } => {
                crate::fm::miller::MillerAdjacentWidthTarget::Preview
            }
        };
        let committed = file_manager.miller.commit_adjacent_column_widths(
            leading_directory,
            leading_width,
            trailing,
            trailing_width,
        );
        if committed {
            crate::render_prof::event("fm.miller_resize.committed");
        }
        committed
    }

    pub(super) fn handle_miller_resize_key(&mut self, key: crossterm::event::KeyEvent) -> bool {
        use crossterm::event::KeyCode;

        if !self.state.shell_interaction.miller_resize_active() {
            return false;
        }
        let step = match key.code {
            KeyCode::Right | KeyCode::Char('l') => Some(1),
            KeyCode::Left | KeyCode::Char('h') => Some(-1),
            _ => None,
        };
        if let Some(step) = step {
            let trailing_min = self.state.shell_interaction.miller_resize_preview().map_or(
                crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
                |(divider, _)| match divider.trailing() {
                    crate::ui::shell::MillerResizeColumnId::Directory { .. } => {
                        crate::fm::miller::MILLER_COLUMN_MIN_WIDTH
                    }
                    crate::ui::shell::MillerResizeColumnId::Preview { .. } => {
                        crate::fm::miller::MILLER_DETAIL_MIN_WIDTH
                    }
                },
            );
            if let Some(bounds) = crate::ui::shell::ResizeBounds::new(
                crate::fm::miller::MILLER_COLUMN_MIN_WIDTH,
                crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
                trailing_min,
                crate::fm::miller::MILLER_COLUMN_MAX_WIDTH,
            ) {
                let before = self.state.shell_interaction.resize_preview_tracks();
                let accepted = self
                    .state
                    .shell_interaction
                    .preview_keyboard_resize_step(step, bounds);
                if accepted && self.state.shell_interaction.resize_preview_tracks() != before {
                    crate::render_prof::event("fm.miller_resize.preview_changed");
                }
            }
            return true;
        }
        match key.code {
            KeyCode::Enter => {
                let _ = self.commit_miller_resize();
            }
            KeyCode::Esc => {
                let update = self.state.shell_interaction.cancel_resize();
                debug_assert!(!update.marks_persistence_dirty());
                debug_assert!(!update.requests_pty_resize());
            }
            _ => {}
        }
        true
    }

    pub(super) fn handle_miller_horizontal_scroll(
        &mut self,
        kind: crossterm::event::MouseEventKind,
        modifiers: crossterm::event::KeyModifiers,
    ) -> bool {
        use crossterm::event::{KeyModifiers, MouseEventKind};
        let delta = match (kind, modifiers) {
            (MouseEventKind::ScrollLeft, KeyModifiers::NONE)
            | (MouseEventKind::ScrollUp, KeyModifiers::SHIFT) => -1,
            (MouseEventKind::ScrollRight, KeyModifiers::NONE)
            | (MouseEventKind::ScrollDown, KeyModifiers::SHIFT) => 1,
            _ => return false,
        };
        let active_generation = self.state.stage.active_instance_generation();
        let snapshot = &self.state.view.file_manager_trail;
        let target = (snapshot.files_generation == active_generation)
            .then(|| {
                self.state
                    .file_manager
                    .as_ref()
                    .and_then(|file_manager| snapshot.horizontal_scroll_target(file_manager, delta))
            })
            .flatten();
        if let (Some(file_manager), Some(target)) = (self.state.file_manager.as_mut(), target) {
            file_manager.miller.horizontal.offset_cells = target;
            file_manager.miller.horizontal.follow_active = false;
        }
        true
    }
}

fn miller_resize_column_id(
    column: &crate::ui::MillerColumnView,
) -> Option<crate::ui::shell::MillerResizeColumnId> {
    match &column.kind {
        crate::ui::MillerColumnKind::Trail {
            trail_index,
            directory,
            generation,
            ..
        } => Some(crate::ui::shell::MillerResizeColumnId::Directory {
            chain_index: *trail_index,
            directory: directory.clone(),
            generation: *generation,
        }),
        crate::ui::MillerColumnKind::Detail {
            parent_trail_index,
            source_path,
            generation,
        } => Some(crate::ui::shell::MillerResizeColumnId::Preview {
            parent_chain_index: *parent_trail_index,
            source_path: Some(source_path.clone()),
            generation: *generation,
        }),
    }
}

fn rect_contains(rect: ratatui::layout::Rect, column: u16, row: u16) -> bool {
    column >= rect.x
        && column < rect.x.saturating_add(rect.width)
        && row >= rect.y
        && row < rect.y.saturating_add(rect.height)
}
