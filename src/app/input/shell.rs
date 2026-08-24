use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Position;

use super::super::state::{AppState, DragState, DragTarget, Mode, SidebarWidthSource};
use crate::ui::shell::{
    CollapseDecision, DividerId, RegionId, ResizeBounds, ResizeDecision, ResizeTransaction,
    ResizeUpdate, ShellDirection,
};

/// The single owner the frozen shell input precedence resolves for one event.
///
/// Frozen order (design spec "Focus, Mouse, and Keyboard Routing"):
/// topmost blocking overlay -> active capture -> z-ordered topmost hit ->
/// focused component -> page/template shortcut -> global shortcuts ->
/// fail-closed consumption so hidden background surfaces never act.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ShellInputOwner {
    TopmostOverlay,
    ActiveCapture,
    TopmostHit(RegionId),
    FocusedComponent,
    PageShortcut,
    GlobalShortcut,
    FailClosed,
}

/// Ownership facts one event resolves against. The context is a pure
/// projection of current state: building it performs no mutation, and the
/// positional hit must come from the exact current `ShellView` generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShellInputRouteContext {
    pub(crate) topmost_overlay: bool,
    pub(crate) active_capture: bool,
    pub(crate) topmost_hit: Option<RegionId>,
    pub(crate) focused_component: bool,
    pub(crate) page_shortcut: bool,
    pub(crate) global_shortcut: bool,
}

/// Resolve exactly one input owner from the frozen precedence. Total by
/// construction: every context maps to one owner and the empty context fails
/// closed instead of leaking to a hidden background surface.
pub(crate) fn route_shell_input(context: ShellInputRouteContext) -> ShellInputOwner {
    if context.topmost_overlay {
        return ShellInputOwner::TopmostOverlay;
    }
    if context.active_capture {
        return ShellInputOwner::ActiveCapture;
    }
    if let Some(target) = context.topmost_hit {
        return ShellInputOwner::TopmostHit(target);
    }
    if context.focused_component {
        return ShellInputOwner::FocusedComponent;
    }
    if context.page_shortcut {
        return ShellInputOwner::PageShortcut;
    }
    if context.global_shortcut {
        return ShellInputOwner::GlobalShortcut;
    }
    ShellInputOwner::FailClosed
}

/// What a press over an edge bar resolves to, before anything is run.
///
/// A typed intent rather than a direct call so the decision stays in pure
/// state: every rule below — is this a section at all, does it carry an action,
/// is a popup already open — is then answerable in a test without a PTY, and
/// the side of the code that spawns processes is left with one match and no
/// judgement of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BarSectionClick {
    /// Not over a live section of this generation. The event belongs to
    /// whoever owns it next; the bar does not claim it.
    Elsewhere,
    /// Over a section that has nothing to do. Still consumed: a bar is chrome,
    /// and an event falling through it would act on a surface the person was
    /// not pointing at (CL12).
    Inert,
    OpenPopup {
        argv: Vec<String>,
        /// The size the person asked this popup to open at, unresolved. The
        /// resolution needs a terminal area, which belongs to the layer that
        /// actually opens the popup, not to the one that decides whether to.
        width: Option<crate::popup_size::PopupSize>,
        height: Option<crate::popup_size::PopupSize>,
    },
    /// Open the same command in a new tab of the current workspace.
    ///
    /// No size travels with it, unlike `OpenPopup`: a tab's root pane already
    /// occupies the whole tab, so "full size" is a property of the shape rather
    /// than a number somebody has to choose. A field that is always ignored is
    /// the dead component CLA9 names.
    ///
    /// No "already open" sibling either. A popup has exactly one slot and
    /// replacing it would drop somebody's open work; tabs are not scarce, and
    /// refusing a second one would refuse something harmless.
    OpenTab { argv: Vec<String> },
    /// Run a command and open nothing.
    ///
    /// No size, no presentation and no "already open" sibling: there is nothing
    /// on screen for a second one to collide with. What it cannot do is report
    /// success, because a command that opened nothing leaves nothing to look
    /// at — only failure is worth saying out loud.
    RunCommand { argv: Vec<String> },
    /// Switch the bar this section sits on off.
    ///
    /// Carries the edge rather than re-deriving it later: the layer that acts
    /// on this has no position in hand any more, and the resolution that does
    /// is the one place drawing, hitting and acting already agree on
    /// (`bar_edge_for`). A second derivation is how a click on one bar comes
    /// to hide another.
    HideBar { edge: crate::ui::shell::BarEdge },
    /// Open the bar's own configuration panel for the edge the press landed
    /// on (TP-CHROME-150). Carries the edge for the same reason `HideBar`
    /// does: the resolution that drawing, hitting and acting agree on is the
    /// one this arm was made at.
    ConfigureBar { edge: crate::ui::shell::BarEdge },
    /// Go to the workspace with this name.
    ///
    /// Resolved where the workspaces are, not here: pure state can answer which
    /// workspace is called what, but a bar chrome table holds a name from a
    /// config file and the list it has to match against can change under it.
    FocusWorkspace { name: String },
    /// Open the same command beside the focused pane.
    ///
    /// Which pane gets split is not carried here for the same reason the
    /// directory is not carried in `OpenTab`: the focused pane is resolved
    /// where the split happens, so a menu left open across a focus change
    /// splits what is focused when the person picks, not what was focused when
    /// they pressed. A pane id captured here would name a pane that may have
    /// closed in between.
    OpenSplit { argv: Vec<String> },
    /// Ask which presentation, by opening a menu at the pointer.
    ///
    /// Carries the popup's size because one of the menu's items opens a popup,
    /// and carries whether a popup is already open because that decides whether
    /// that item can be picked. Both are read here, in the same pass that
    /// resolved the section — a second lookup when the item fires would answer
    /// for whatever the bar holds by then, and a menu can outlive the frame it
    /// was opened from (the reason `AgentEntry` carries its session id).
    OpenMenu {
        argv: Vec<String>,
        width: Option<crate::popup_size::PopupSize>,
        height: Option<crate::popup_size::PopupSize>,
        popup_open: bool,
        /// The edge the menu opens on — what its "Configure bar..." row
        /// configures (TP-CHROME-150).
        edge: crate::ui::shell::BarEdge,
    },
    /// Over a popup action while a popup is already open. Named rather than
    /// folded into `Inert` because the two deserve different answers: this one
    /// is worth saying out loud, and neither may close the popup that is
    /// already there — dropping somebody's open work on a stray bar click is
    /// not undoable.
    PopupAlreadyOpen,
    /// Invoke an action an installed plugin declared.
    ///
    /// No `PopupAlreadyOpen` sibling, and that omission is a decision rather
    /// than an oversight. The popup slot holds exactly one pane, so a second
    /// request has to be refused or somebody's open work is dropped. A plugin
    /// action is not that slot: what it opens — an overlay, a split, a
    /// notification, nothing at all — is the plugin manifest's choice, and
    /// guarding it here would refuse something harmless.
    InvokePlugin {
        /// The action id, still exactly as the config spelled it. Resolution
        /// against the installed manifests happens where the plugin registry
        /// lives; this layer only decides that it was asked for.
        action: String,
    },
}

/// Which of a section's two answers a press is asking for.
///
/// Named for what the gestures mean rather than for which physical button was
/// pressed, and deliberately not `crossterm::MouseButton`: pure state has no
/// business learning an input-transport type, and "left" is already a lie on a
/// machine whose owner swapped the buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SectionGesture {
    /// The action itself.
    Primary,
    /// A choice *about* the action — today, which presentation it opens in.
    Secondary,
}

impl AppState {
    /// Which action, if any, a press at this position asks for.
    ///
    /// Resolved against the exact current geometry generation, so coordinates
    /// from a layout that no longer exists resolve to nothing rather than to
    /// whatever section happens to sit there now (CL5).
    // TP-CHROME-41/42: only the live generation is authority, and a popup
    // already on screen is neither replaced nor closed.
    // TP-CHROME-60: the two gestures resolve against the same hit, so a section
    // can never answer one of them at a place it does not answer the other.
    pub(crate) fn bar_section_click_at(
        &self,
        position: Position,
        gesture: SectionGesture,
    ) -> BarSectionClick {
        let Some((region, index)) = self
            .view
            .shell
            .bar_section_hit_at(self.view.shell.generation, position)
        else {
            return BarSectionClick::Elsewhere;
        };
        match self.shell_bar_chrome.action_for(region, index) {
            // An actionless stretch of the bar is still chrome — and on the
            // second gesture it is the bar's own door: the config panel opens
            // for the edge the press landed on (TP-CHROME-150). Sections with
            // their own actions keep their recorded answers.
            None | Some(crate::ui::shell::SectionAction::None) => match gesture {
                SectionGesture::Primary => BarSectionClick::Inert,
                SectionGesture::Secondary => match crate::ui::shell::bar_edge_for(region) {
                    Some(edge) => BarSectionClick::ConfigureBar { edge },
                    None => BarSectionClick::Inert,
                },
            },
            // Neither of these opens anything, so neither has a second
            // presentation to offer — the same reason the plugin arm below is
            // deliberately inert on a right press. A menu here would list three
            // places to put something that goes to none of them.
            Some(crate::ui::shell::SectionAction::Run { argv }) => match gesture {
                SectionGesture::Primary => BarSectionClick::RunCommand { argv: argv.clone() },
                SectionGesture::Secondary => BarSectionClick::Inert,
            },
            Some(crate::ui::shell::SectionAction::FocusWorkspace { name }) => match gesture {
                SectionGesture::Primary => BarSectionClick::FocusWorkspace { name: name.clone() },
                SectionGesture::Secondary => BarSectionClick::Inert,
            },
            Some(crate::ui::shell::SectionAction::OpenPopup {
                argv,
                width,
                height,
                secondary,
            }) => match gesture {
                SectionGesture::Primary => {
                    if self.popup_pane.is_some() {
                        BarSectionClick::PopupAlreadyOpen
                    } else {
                        BarSectionClick::OpenPopup {
                            argv: argv.clone(),
                            width: *width,
                            height: *height,
                        }
                    }
                }
                // A section whose second answer is `Inert` stays inert rather
                // than falling through: the bar is still chrome, and an event
                // that reached the surface behind would act on something the
                // person was not pointing at (CL12).
                SectionGesture::Secondary => match secondary {
                    // Fail closed on the edge, like every other arm that
                    // carries one: a menu that could not name its bar would
                    // offer a Configure row pointing at nothing.
                    crate::ui::shell::SecondaryPresentation::Menu => {
                        match crate::ui::shell::bar_edge_for(region) {
                            Some(edge) => BarSectionClick::OpenMenu {
                                argv: argv.clone(),
                                width: *width,
                                height: *height,
                                popup_open: self.popup_pane.is_some(),
                                edge,
                            },
                            None => BarSectionClick::Inert,
                        }
                    }
                    crate::ui::shell::SecondaryPresentation::Tab => {
                        BarSectionClick::OpenTab { argv: argv.clone() }
                    }
                    crate::ui::shell::SecondaryPresentation::Split => {
                        BarSectionClick::OpenSplit { argv: argv.clone() }
                    }
                    crate::ui::shell::SecondaryPresentation::Inert => BarSectionClick::Inert,
                },
            },
            Some(crate::ui::shell::SectionAction::Hide) => match gesture {
                SectionGesture::Primary => match crate::ui::shell::bar_edge_for(region) {
                    Some(edge) => BarSectionClick::HideBar { edge },
                    // A section only resolves inside an edge bar, so there is
                    // nothing for this arm to be — but fail closed rather than
                    // into whichever edge happens to be last (TP-CHROME-38's
                    // rule, applied to the press that acts on the chrome).
                    None => BarSectionClick::Inert,
                },
                // Nothing to present, and the bar stays chrome: an event that
                // fell through would act on the surface behind it (CL12).
                SectionGesture::Secondary => BarSectionClick::Inert,
            },
            Some(crate::ui::shell::SectionAction::InvokePlugin { action }) => match gesture {
                SectionGesture::Primary => BarSectionClick::InvokePlugin {
                    action: action.clone(),
                },
                // The bar does not open what a plugin action opens, so it has
                // no second presentation to offer. Consumed all the same, for
                // the same reason as the arm above: chrome that let an event
                // through would act on the surface behind it.
                SectionGesture::Secondary => BarSectionClick::Inert,
            },
        }
    }

    /// Project current keyboard ownership into the frozen router. Keyboard
    /// events carry no position, so the hit tier stays empty; v0 has no
    /// page/template shortcut owner yet, so remaining keys belong to the
    /// global application dispatch. The focused component derives from the
    /// TYPED stage authority AND live Files domain state, so a divergent
    /// legacy boolean can never grant keyboard focus to a hidden surface.
    pub(crate) fn shell_key_input_owner(&self) -> ShellInputOwner {
        // TP-SBS-FOCUS-01: Files owns the keyboard on its full stage, and in
        // the right half of a split whose focus the person clicked over —
        // both gated on live Files domain state, so a divergent boolean can
        // never grant keys to a hidden surface.
        let files_beside_focused = self.files_beside_active()
            && self
                .side_by_side
                .is_some_and(|sbs| sbs.focus == crate::app::state::SideBySideFocus::Right);
        let files_surface_focused = (self.stage.surface_view()
            == crate::ui::surface_host::StageSurfaceView::NativeFiles
            || files_beside_focused)
            && self.file_manager.is_some();
        route_shell_input(ShellInputRouteContext {
            topmost_overlay: self.blocking_overlay_active(),
            active_capture: self.shell_interaction.resize_active(),
            topmost_hit: None,
            focused_component: files_surface_focused,
            page_shortcut: false,
            global_shortcut: true,
        })
    }

    /// Project current mouse ownership into the frozen router. The overlay
    /// tier and the positional-hit tier are live: the hit resolves ONLY
    /// against the exact current `ShellView` generation, so coordinates from
    /// vanished geometry re-resolve to their current owner. Mouse captures
    /// stay routed through `DragState` (frozen by the SF4.2-04
    /// characterization), and the focused-component tier arrives with the
    /// SF4.2-08 hidden-terminal slice, so unrouted events remain with the
    /// existing mode-guarded global dispatch.
    pub(crate) fn shell_mouse_input_owner(&self, position: Position) -> ShellInputOwner {
        route_shell_input(ShellInputRouteContext {
            topmost_overlay: self.blocking_overlay_active(),
            active_capture: false,
            topmost_hit: self
                .view
                .shell
                .region_hit_at(self.view.shell.generation, position),
            focused_component: false,
            page_shortcut: false,
            global_shortcut: true,
        })
    }

    /// Enter a blocking overlay while remembering the current non-default
    /// focus owner (`Resize`/`Copy`) so `leave_modal` can restore it. An
    /// overlay-to-overlay transition preserves the original remembered owner;
    /// entering from a default owner clears any stale value by construction.
    pub(crate) fn enter_overlay_mode(&mut self, overlay: Mode) {
        if !self.blocking_overlay_active() {
            self.overlay_return_mode =
                matches!(self.mode, Mode::Resize | Mode::Copy).then_some(self.mode);
        }
        self.mode = overlay;
    }

    /// Every mode whose surface is a topmost blocking overlay for mouse and
    /// keyboard routing. The match is exhaustive so a new mode must choose a
    /// side explicitly instead of silently leaking background input.
    pub(crate) fn blocking_overlay_active(&self) -> bool {
        match self.mode {
            Mode::Terminal | Mode::Prefix | Mode::Navigate | Mode::Copy | Mode::Resize => false,
            Mode::Onboarding
            | Mode::ReleaseNotes
            | Mode::ProductAnnouncement
            | Mode::AttachFile
            | Mode::RenameWorkspace
            | Mode::RenameTab
            | Mode::RenamePane
            | Mode::RenameFile
            | Mode::NewLinkedWorktree
            | Mode::OpenExistingWorktree
            | Mode::ConfirmRemoveWorktree
            | Mode::ConfirmDeleteModule
            | Mode::ConfirmClose
            | Mode::ConfirmFileDelete
            | Mode::ContextMenu
            | Mode::Settings
            | Mode::GlobalMenu
            | Mode::KeybindHelp
            | Mode::ChatWorkLog
            | Mode::Navigator
            | Mode::PreviewViewer
            | Mode::TailscaleSend
            | Mode::AgentReferencePicker
            | Mode::AgentColleaguePicker
            | Mode::BarConfigPanel => true,
        }
    }

    /// Does the current mode's overlay hide the stage surface underneath it?
    ///
    /// A separate question from [`Self::blocking_overlay_active`], which is
    /// about input routing and therefore groups a small anchored menu with a
    /// full-screen settings page. For drawing they are opposites: the menu
    /// floats above the surface, the settings page replaces it.
    ///
    /// The distinction only matters for host graphics. Terminal images are not
    /// cells in the frame buffer, so a full-screen overlay overwrites the text
    /// under an image without touching the image itself — the picture would
    /// hang over the settings page. An anchored overlay covers a small part of
    /// the surface, and taking the picture away for it is the bug this answers.
    ///
    /// Each arm follows what `OverlayLayer` actually renders: an overlay drawn
    /// into `frame.area()` hides the surface; one drawn into `terminal_area`,
    /// or into its own anchored box, does not.
    ///
    /// Exhaustive on purpose, like its neighbour: a new mode has to pick a side
    /// rather than inherit one silently.
    pub(crate) fn overlay_hides_stage_surface(&self) -> bool {
        match self.mode {
            Mode::Terminal
            | Mode::Prefix
            | Mode::Navigate
            | Mode::Copy
            | Mode::Resize
            | Mode::ContextMenu
            | Mode::ConfirmClose
            // The viewer's whole purpose is the picture, and the picture is a
            // host image rather than cells in the frame buffer. Declaring it a
            // surface-hiding overlay would suppress the placement pass and the
            // viewer would open onto an empty frame.
            | Mode::PreviewViewer
            // Drawn as its own centred box over the file manager, like the
            // delete confirmation beside it. The rule for this match is
            // mechanical — it follows what the overlay actually paints, not
            // what would be convenient — and an anchored box does not replace
            // the surface.
            | Mode::TailscaleSend
            | Mode::ConfirmFileDelete => false,
            Mode::Onboarding
            | Mode::ReleaseNotes
            | Mode::ProductAnnouncement
            | Mode::AttachFile
            | Mode::RenameWorkspace
            | Mode::RenameTab
            | Mode::RenamePane
            | Mode::RenameFile
            | Mode::NewLinkedWorktree
            | Mode::OpenExistingWorktree
            | Mode::ConfirmRemoveWorktree
            | Mode::ConfirmDeleteModule
            | Mode::Settings
            | Mode::GlobalMenu
            | Mode::KeybindHelp
            | Mode::ChatWorkLog
            | Mode::Navigator
            | Mode::AgentReferencePicker
            | Mode::AgentColleaguePicker
            | Mode::BarConfigPanel => true,
        }
    }

    pub(crate) fn begin_sidebar_resize(&mut self, pointer: Position) -> bool {
        let Some(total) = self.current_sidebar_resize_total() else {
            return false;
        };
        let Some(original_tracks) = self.sidebar_resize_tracks(total) else {
            return false;
        };
        let Some(divider) = DividerId::new(
            RegionId::LeftPanel,
            RegionId::WorkspaceStage,
            ShellDirection::Horizontal,
        ) else {
            return false;
        };
        let Some(transaction) = ResizeTransaction::begin(
            divider,
            self.view.shell.generation,
            pointer,
            original_tracks,
        ) else {
            return false;
        };

        self.shell_interaction.begin_resize(transaction);
        self.drag = Some(DragState {
            target: DragTarget::SidebarDivider,
        });
        true
    }

    pub(crate) fn preview_sidebar_resize(&mut self, pointer: Position) -> bool {
        let Some(total) = self.shell_interaction.resize_original_total() else {
            return false;
        };
        let Some(bounds) = self.sidebar_resize_bounds(total) else {
            return false;
        };
        self.shell_interaction.preview_resize(pointer, bounds)
    }

    pub(crate) fn handle_shell_resize_key(&mut self, key: KeyEvent) -> bool {
        if !self.shell_resize_active() {
            return false;
        }

        match key.code {
            KeyCode::Right | KeyCode::Char('l') => self.preview_sidebar_resize_step(1),
            KeyCode::Left | KeyCode::Char('h') => self.preview_sidebar_resize_step(-1),
            KeyCode::Enter => {
                self.commit_sidebar_resize();
                self.clear_sidebar_resize_drag();
            }
            KeyCode::Esc => self.cancel_sidebar_resize(),
            _ => {}
        }
        true
    }

    pub(crate) fn set_sidebar_collapsed(&mut self, collapsed: bool) -> bool {
        let update = if collapsed {
            self.shell_presentation
                .collapse_left_panel(self.sidebar_width)
        } else {
            let Some(total) = self.current_sidebar_resize_total() else {
                return false;
            };
            let Some(bounds) = self.sidebar_resize_bounds(total) else {
                return false;
            };
            self.shell_presentation.expand_left_panel(total, bounds)
        };

        match update.decision() {
            CollapseDecision::Inert => return false,
            CollapseDecision::Collapsed { .. } => {
                self.sidebar_collapsed = true;
            }
            CollapseDecision::Expanded { width } => {
                self.sidebar_collapsed = false;
                // An explicit expansion outranks the short-viewport fold for
                // the rest of the session.
                self.sidebar_expanded_explicitly = true;
                self.sidebar_width = width;
                self.sidebar_width_source = SidebarWidthSource::Manual;
                self.sidebar_width_auto = false;
            }
        }
        if update.marks_persistence_dirty() {
            self.mark_session_dirty();
        }
        true
    }

    /// Switch every configured bar off, or every switched-off bar back on.
    ///
    /// A filled switch — however it was filled — empties. The global key and
    /// the per-edge hide button share one set, and the person pressing the key
    /// over a partial state is asking for their bars back, not for the rest to
    /// disappear too: filling from partial would make the restore press hide
    /// things, the opposite of what it says. An empty config is inert rather
    /// than "changed": a press that flipped an empty set to an empty set would
    /// report work nobody can see.
    ///
    /// No session-dirty mark on purpose: the switch is deliberately unsaved
    /// (see `ShellPresentationState::toggled_off`), and marking the session
    /// dirty for a value the snapshot never records would schedule a write
    /// that changes nothing.
    pub(crate) fn toggle_bars(&mut self) -> bool {
        if !self.shell_presentation.toggled_off().is_empty() {
            self.shell_presentation
                .set_toggled_off(crate::ui::shell::BarEdges::NONE);
            return true;
        }
        let enabled = self.shell_presentation.bars().enabled_edges();
        if enabled.is_empty() {
            return false;
        }
        self.shell_presentation.set_toggled_off(enabled);
        true
    }

    /// Switch one edge off — the hide button's half of the gesture.
    ///
    /// Insert rather than replace: two hide buttons pressed in turn leave two
    /// edges off, and the global key releases them together. No session-dirty
    /// mark, for the reason `toggle_bars` gives.
    pub(crate) fn hide_bar_edge(&mut self, edge: crate::ui::shell::BarEdge) -> bool {
        let mut off = self.shell_presentation.toggled_off();
        off.insert(edge);
        if off == self.shell_presentation.toggled_off() {
            return false;
        }
        self.shell_presentation.set_toggled_off(off);
        true
    }

    #[cfg(test)]
    fn sidebar_collapse_snapshot_for_test(&self) -> (u16, u64) {
        (
            self.shell_presentation.left_panel_restore_width(),
            self.shell_presentation.left_panel_collapse_revision(),
        )
    }

    pub(crate) fn commit_sidebar_resize(&mut self) {
        let generation = self
            .shell_interaction
            .resize_generation()
            .unwrap_or(self.view.shell.generation);
        let update = self.shell_interaction.commit_resize(generation);
        self.apply_sidebar_resize_update(update, SidebarWidthSource::Manual);
    }

    fn cancel_sidebar_resize(&mut self) {
        let update = self.shell_interaction.cancel_resize();
        debug_assert!(!update.marks_persistence_dirty());
        debug_assert!(!update.requests_pty_resize());
        self.clear_sidebar_resize_drag();
    }

    pub(crate) fn reset_sidebar_resize_to_preferred(&mut self) {
        let _ = self.shell_interaction.cancel_resize();
        self.clear_sidebar_resize_drag();

        let Some(total) = self.current_sidebar_resize_total() else {
            return;
        };
        let Some(current) = self.sidebar_resize_tracks(total) else {
            return;
        };
        let Some(bounds) = self.sidebar_resize_bounds(total) else {
            return;
        };
        let update =
            ResizeTransaction::reset_preferred(current, self.default_sidebar_width, bounds);
        self.apply_sidebar_resize_update(update, SidebarWidthSource::ConfigDefault);
    }

    pub(crate) fn cancel_sidebar_resize_for_terminal_area(&mut self, new_total: u16) {
        let update = if let Some(bounds) = self.sidebar_resize_bounds(new_total) {
            self.shell_interaction.terminal_resize(new_total, bounds)
        } else {
            self.shell_interaction.cancel_resize()
        };
        debug_assert!(!update.marks_persistence_dirty());
        debug_assert!(!update.requests_pty_resize());
        self.clear_sidebar_resize_drag();
    }

    pub(crate) fn cancel_miller_resize_for_terminal_area(&mut self, new_total: u16) {
        let previous_total = self.view.shell.area.width;
        if previous_total == 0
            || previous_total == new_total
            || !self.shell_interaction.miller_resize_active()
        {
            return;
        }
        let update = self.shell_interaction.cancel_resize();
        debug_assert!(!update.marks_persistence_dirty());
        debug_assert!(!update.requests_pty_resize());
    }

    pub(crate) fn rebase_sidebar_resize_generation(&mut self, generation: u64) {
        self.shell_interaction.rebase_resize_generation(generation);
    }

    pub(crate) fn shell_resize_active(&self) -> bool {
        self.shell_interaction.shell_resize_active()
    }

    pub(crate) fn shell_resize_preview_width(&self) -> Option<u16> {
        self.shell_interaction
            .shell_resize_preview_tracks()
            .map(|tracks| tracks[0])
    }

    pub(crate) fn shell_resize_original_total(&self) -> Option<u16> {
        self.shell_interaction.shell_resize_original_total()
    }

    fn current_sidebar_resize_total(&self) -> Option<u16> {
        if self.view.shell.area.width > 0 {
            Some(self.view.shell.area.width)
        } else {
            self.view
                .sidebar_rect
                .width
                .checked_add(self.view.terminal_area.width)
        }
    }

    fn sidebar_resize_tracks(&self, total: u16) -> Option<[u16; 2]> {
        let leading = self
            .sidebar_width
            .clamp(self.sidebar_min_width, self.sidebar_max_width);
        let trailing = total.checked_sub(leading)?;
        Some([leading, trailing])
    }

    fn sidebar_resize_bounds(&self, total: u16) -> Option<ResizeBounds> {
        ResizeBounds::new(self.sidebar_min_width, self.sidebar_max_width, 1, total)
    }

    fn preview_sidebar_resize_step(&mut self, step: i16) {
        let Some(total) = self.shell_interaction.resize_original_total() else {
            return;
        };
        let Some(bounds) = self.sidebar_resize_bounds(total) else {
            return;
        };
        self.shell_interaction
            .preview_keyboard_resize_step(step, bounds);
    }

    fn apply_sidebar_resize_update(&mut self, update: ResizeUpdate, source: SidebarWidthSource) {
        if let ResizeDecision::Committed([leading, _]) = update.decision() {
            self.sidebar_width = leading;
            self.sidebar_width_source = source;
            self.sidebar_width_auto = false;
        }
        if update.marks_persistence_dirty() {
            crate::render_prof::event("shell.persistence_write");
            self.mark_session_dirty();
        }
        if update.requests_pty_resize() {
            crate::render_prof::event("shell.pty_resize_request");
        }
        // Clearing preview capture makes the next committed compute_view frame
        // the single high-level resize request represented by this flag.
        debug_assert_eq!(
            update.requests_pty_resize(),
            matches!(update.decision(), ResizeDecision::Committed(_))
        );
    }

    fn clear_sidebar_resize_drag(&mut self) {
        if matches!(
            self.drag.as_ref().map(|drag| &drag.target),
            Some(DragTarget::SidebarDivider)
        ) {
            self.drag = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::{Position, Rect};

    use super::*;

    fn bars_on_two_edges() -> crate::ui::shell::ShellBars {
        let mut config = crate::config::ShellBarsConfig::default();
        config.top.enabled = true;
        config.top.size = 1;
        config.top.border = Some(false);
        config.right.enabled = true;
        config.right.size = 1;
        config.right.border = Some(false);
        crate::ui::shell::ShellBars::from_config(&config)
    }

    // TP-CHROME-140: the gesture is global and the model is per edge. One
    // press fills the switch with every edge this config put something on;
    // the next empties it, whatever it holds.
    #[test]
    fn the_bar_switch_fills_with_every_enabled_edge_and_empties_again() {
        let mut state = AppState::test_new();
        state.shell_presentation.set_bars(bars_on_two_edges());

        assert!(state.toggle_bars(), "the first press changes something");
        let off = state.shell_presentation.toggled_off();
        let drawn = state.shell_presentation.bars().visible(false, off);
        assert!(
            !drawn.top.enabled() && !drawn.right.enabled(),
            "every configured edge went quiet together"
        );

        assert!(state.toggle_bars(), "the second press changes it back");
        assert!(
            state.shell_presentation.toggled_off().is_empty(),
            "the switch is empty again"
        );
    }

    // TP-CHROME-140: a partly filled switch empties rather than filling.
    //
    // The cell the two easy transitions never visit. A per-edge gesture — the
    // hide button — can leave one edge off while others draw, and the person
    // pressing the global key there is asking for their bars back, not for
    // the rest to disappear too. Filling from a partial state would make the
    // restore press hide things, which is the opposite of what it says.
    #[test]
    fn a_partly_filled_switch_empties_rather_than_filling() {
        let mut state = AppState::test_new();
        state.shell_presentation.set_bars(bars_on_two_edges());
        // A one-edge set, built the way the hide button will build it: from a
        // composition that has exactly that edge.
        let mut top_only = crate::config::ShellBarsConfig::default();
        top_only.top.enabled = true;
        top_only.top.size = 1;
        top_only.top.border = Some(false);
        state
            .shell_presentation
            .set_toggled_off(crate::ui::shell::ShellBars::from_config(&top_only).enabled_edges());

        assert!(state.toggle_bars(), "the press changes something");
        assert!(
            state.shell_presentation.toggled_off().is_empty(),
            "a partial switch is released, never topped up"
        );
    }

    // TP-CHROME-140: with nothing configured the gesture is inert — it says
    // so, and it leaves no state behind. A press that "changed" an empty set
    // to an empty set would mark work nobody can see.
    #[test]
    fn the_switch_is_inert_with_no_bar_to_switch() {
        let mut state = AppState::test_new();

        assert!(!state.toggle_bars(), "nothing to switch, nothing changed");
        assert!(state.shell_presentation.toggled_off().is_empty());
    }

    #[test]
    fn active_sidebar_capture_right_arrow_previews_one_cell() {
        let mut state = state_with_sidebar_capture();

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.shell_resize_preview_width(),
                state.sidebar_width,
                state.session_dirty,
            ),
            (true, Some(27), 26, false)
        );
    }

    #[test]
    fn repeated_keyboard_resize_accumulates_through_same_preview_path() {
        let mut state = state_with_sidebar_capture();
        let right = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);

        handle_shell_resize_key_for_test(&mut state, right);
        handle_shell_resize_key_for_test(&mut state, right);

        assert_eq!(state.shell_resize_preview_width(), Some(28));
        assert_eq!(state.sidebar_width, 26);
        assert!(!state.session_dirty);
    }

    // SF6.3 contract: one hundred pointer preview moves inside a single
    // resize transaction produce ZERO persistence writes and leave the
    // committed width untouched; exactly the commit marks persistence once.
    // PTY resize purity is structural and separately frozen: preview returns
    // no `ResizeUpdate` by type, and `resize_panes_during_shell_preview`
    // suppresses pane resizing for the whole preview window.
    #[test]
    fn hundred_preview_moves_produce_no_persistence_or_pty_effects() {
        let mut state = state_with_sidebar_capture();

        for step in 0..100u16 {
            state.preview_sidebar_resize(Position::new(20 + (step % 10), 5));
        }
        assert!(
            !state.session_dirty,
            "one hundred preview moves must write no persistence"
        );
        assert_eq!(
            state.sidebar_width, 26,
            "the committed width is untouched during preview"
        );
        assert!(
            state.shell_resize_preview_width().is_some(),
            "the transaction is still live after one hundred moves"
        );

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert!(handled);
        assert!(
            state.session_dirty,
            "exactly the commit marks persistence dirty"
        );
        assert!(state.shell_resize_preview_width().is_none());
    }

    #[test]
    fn resize_profile_counts_only_committed_persistence_and_pty_requests() {
        let mut state = state_with_sidebar_capture();

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            for step in 0..100_u16 {
                state.preview_sidebar_resize(Position::new(20 + (step % 10), 5));
            }
            state.commit_sidebar_resize();
        });

        assert_eq!(profile.counter("shell.persistence_write"), 1);
        assert_eq!(profile.counter("shell.pty_resize_request"), 1);
        assert!(
            state.session_dirty,
            "the persistence counter corresponds to the debounced dirty request"
        );
    }

    #[test]
    fn active_sidebar_capture_enter_commits_preview() {
        let mut state = state_with_sidebar_capture();
        handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        state.session_dirty = false;

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.sidebar_width,
                state.shell_resize_active(),
                state.session_dirty,
            ),
            (true, 27, false, true)
        );
    }

    #[test]
    fn active_sidebar_capture_escape_restores_original() {
        let mut state = state_with_sidebar_capture();
        handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        );
        state.session_dirty = false;

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.sidebar_width,
                state.shell_resize_active(),
                state.session_dirty,
            ),
            (true, 26, false, false)
        );
    }

    #[test]
    fn active_sidebar_capture_consumes_non_axis_key_inert() {
        let mut state = state_with_sidebar_capture();

        let handled = handle_shell_resize_key_for_test(
            &mut state,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        );

        assert_eq!(
            (
                handled,
                state.shell_resize_preview_width(),
                state.sidebar_width,
                state.session_dirty,
            ),
            (true, Some(26), 26, false)
        );
    }

    #[test]
    fn sidebar_collapse_adapter_remembers_committed_width_and_marks_dirty_once() {
        let mut state = state_for_sidebar_collapse();
        state.mode = super::super::super::state::Mode::Navigate;

        let changed = set_sidebar_collapsed_for_test(&mut state, true);

        assert_eq!(
            (
                changed,
                state.sidebar_collapsed,
                state.sidebar_width,
                sidebar_collapse_snapshot_for_test(&state),
                state.session_dirty,
                state.mode,
            ),
            (
                true,
                true,
                32,
                (32, 1),
                true,
                super::super::super::state::Mode::Navigate,
            )
        );
    }

    #[test]
    fn repeated_sidebar_collapse_intent_is_inert() {
        let mut state = state_for_sidebar_collapse();
        assert!(set_sidebar_collapsed_for_test(&mut state, true));
        state.session_dirty = false;

        let changed = set_sidebar_collapsed_for_test(&mut state, true);

        assert_eq!(
            (
                changed,
                sidebar_collapse_snapshot_for_test(&state),
                state.session_dirty,
            ),
            (false, (32, 1), false)
        );
    }

    #[test]
    fn sidebar_expand_adapter_clamps_restore_after_terminal_shrink() {
        let mut state = state_for_sidebar_collapse();
        assert!(set_sidebar_collapsed_for_test(&mut state, true));
        state.session_dirty = false;
        state.view.shell.area.width = 27;

        let changed = set_sidebar_collapsed_for_test(&mut state, false);

        assert_eq!(
            (
                changed,
                state.sidebar_collapsed,
                state.sidebar_width,
                sidebar_collapse_snapshot_for_test(&state),
                state.session_dirty,
            ),
            (true, false, 26, (26, 2), true)
        );
    }

    fn state_with_sidebar_capture() -> AppState {
        let mut state = AppState::test_new();
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        assert!(state.begin_sidebar_resize(Position::new(25, 5)));
        state.session_dirty = false;
        state
    }

    fn state_for_sidebar_collapse() -> AppState {
        let mut state = AppState::test_new();
        state.sidebar_width = 32;
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        state.session_dirty = false;
        state
    }

    fn set_sidebar_collapsed_for_test(state: &mut AppState, collapsed: bool) -> bool {
        state.set_sidebar_collapsed(collapsed)
    }

    fn sidebar_collapse_snapshot_for_test(state: &AppState) -> (u16, u64) {
        state.sidebar_collapse_snapshot_for_test()
    }

    fn handle_shell_resize_key_for_test(state: &mut AppState, key: KeyEvent) -> bool {
        state.handle_shell_resize_key(key)
    }

    // SF4.2-05: closing an overlay restores the previous VALID focus owner,
    // not blindly the Terminal/Navigate template fallback. The launcher is
    // explicitly enabled in Resize mode, so GlobalMenu-from-Resize is a real
    // user path whose close must return to the resize session.
    #[test]
    fn focus_restores_after_overlay_close() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let esc = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);

        // Control: a Terminal-origin overlay still falls back to Terminal.
        let mut state = AppState::test_new();
        state.workspaces = vec![crate::workspace::Workspace::test_new("one")];
        state.active = Some(0);
        state.selected = 0;
        state.mode = Mode::Terminal;
        super::super::modal::open_global_menu(&mut state);
        assert_eq!(state.mode, Mode::GlobalMenu);
        super::super::modal::handle_global_menu_key(&mut state, esc);
        assert_eq!(
            state.mode,
            Mode::Terminal,
            "control: a Terminal-origin overlay close keeps the fallback"
        );

        // A Resize-origin overlay close must restore the resize session.
        state.mode = Mode::Resize;
        super::super::modal::open_global_menu(&mut state);
        assert_eq!(state.mode, Mode::GlobalMenu);
        super::super::modal::handle_global_menu_key(&mut state, esc);
        assert_eq!(
            state.mode,
            Mode::Resize,
            "closing an overlay must restore the previous valid focus owner"
        );

        // A remembered owner that is no longer valid falls back instead.
        state.mode = Mode::Resize;
        super::super::modal::open_global_menu(&mut state);
        state.active = None;
        super::super::modal::handle_global_menu_key(&mut state, esc);
        assert_eq!(
            state.mode,
            Mode::Navigate,
            "an invalid remembered owner must fall back, never restore blindly"
        );

        // A ContextMenu opened from Copy mode (the right-click arms carry no
        // mode guard) must restore the live copy session instead of stranding
        // `copy_mode: Some` under `Mode::Terminal`.
        state.active = Some(0);
        state.selected = 0;
        let pane_id = state.workspaces[0]
            .focused_pane_id()
            .expect("test workspace exposes a focused pane");
        state.copy_mode = Some(crate::app::state::CopyModeState {
            pane_id,
            cursor_row: 0,
            cursor_col: 0,
            entry_offset_from_bottom: 0,
            selection: None,
            search: crate::app::state::CopyModeSearchState::default(),
        });
        state.mode = Mode::Copy;
        state.open_project_new_chat_menu(0, 4, 4);
        assert_eq!(state.mode, Mode::ContextMenu);
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();
        super::super::modal::handle_context_menu_key(&mut state, &mut terminal_runtimes, esc);
        assert_eq!(
            state.mode,
            Mode::Copy,
            "closing a context menu must restore the live copy session"
        );
        assert!(
            state.copy_mode.is_some(),
            "the copy session itself must survive the overlay episode"
        );
    }

    // SF4.2-07: the mouse context builder must resolve the z-ordered topmost
    // hit from the exact current `ShellView` generation, so a position is
    // authority only against live geometry — old coordinates re-resolve to
    // their CURRENT owner after every geometry change and never grant a
    // vanished region's authority.
    #[test]
    fn stale_hit_generation_fails_closed() {
        let layout = crate::ui::shell::ShellLayout::default();
        let area = ratatui::layout::Rect::new(0, 0, 80, 24);
        let sidebar_position = Position::new(5, 5);

        let mut state = AppState::test_new();
        state.view.shell = crate::ui::shell::compute_shell_view(
            &layout,
            crate::ui::shell::ShellGeometryKey::new(
                area,
                0,
                26,
                0,
                None,
                crate::ui::shell::ShellBars::NONE,
            ),
            crate::ui::shell::ShellView::default(),
            &|region| u16::from(region == RegionId::LeftPanel) * 26,
        );

        // A current-generation position inside the sidebar is owned by the
        // hit tier.
        assert_eq!(
            shell_mouse_owner_at(&state, sidebar_position),
            ShellInputOwner::TopmostHit(RegionId::LeftPanel),
            "a live sidebar position must resolve through the hit tier"
        );

        // A blocking overlay outranks every positional hit.
        state.mode = Mode::GlobalMenu;
        assert_eq!(
            shell_mouse_owner_at(&state, sidebar_position),
            ShellInputOwner::TopmostOverlay,
            "the hit tier must never outrank a blocking overlay"
        );
        state.mode = Mode::Terminal;

        // After a geometry change the SAME coordinates belong to the current
        // owner; the vanished sidebar authority is gone with its generation.
        let previous = std::mem::take(&mut state.view.shell);
        state.view.shell = crate::ui::shell::compute_shell_view(
            &layout,
            crate::ui::shell::ShellGeometryKey::new(
                area,
                0,
                4,
                0,
                None,
                crate::ui::shell::ShellBars::NONE,
            ),
            previous,
            &|region| u16::from(region == RegionId::LeftPanel) * 4,
        );
        assert_eq!(
            shell_mouse_owner_at(&state, sidebar_position),
            ShellInputOwner::TopmostHit(RegionId::WorkspaceStage),
            "old coordinates must re-resolve to their current owner"
        );

        // Outside every live region the hit tier stays silent.
        assert_eq!(
            shell_mouse_owner_at(&state, Position::new(100, 100)),
            ShellInputOwner::GlobalShortcut,
            "a positionless miss must fall through, never invent a hit"
        );
    }

    fn shell_mouse_owner_at(state: &AppState, position: Position) -> ShellInputOwner {
        state.shell_mouse_input_owner(position)
    }

    /// A state whose top bar is one cell tall and divided as `sections`, with
    /// the geometry actually computed — so every assertion below is answered by
    /// the same derivation, layout and hit list the running app uses, not by a
    /// hand-built fixture that could agree with nothing.
    fn state_with_divided_top_bar(
        sections: Vec<crate::config::ShellBarSectionConfig>,
    ) -> (AppState, Rect) {
        state_with_divided_top_bar_of_size(sections, 1)
    }

    fn state_with_divided_top_bar_of_size(
        sections: Vec<crate::config::ShellBarSectionConfig>,
        size: u16,
    ) -> (AppState, Rect) {
        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size,
                border: Some(false),
                hide_when_focused: false,
                color: String::new(),
                gradient: Vec::new(),
                sections,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut state = AppState::test_new();
        state.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            26,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::themed_by_default(&config, true);
        let area = Rect::new(0, 0, 106, 40);
        crate::ui::compute_view(&mut state, area);
        (state, area)
    }

    /// The bottom-bar twin of the fixture above, and deliberately not the top
    /// bar: a resolution that hardcoded one edge would still pass every test
    /// written on that edge, which is masking, not coverage.
    fn state_with_divided_bottom_bar(
        sections: Vec<crate::config::ShellBarSectionConfig>,
    ) -> (AppState, Rect) {
        let config = crate::config::ShellBarsConfig {
            bottom: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: Some(false),
                hide_when_focused: false,
                color: String::new(),
                gradient: Vec::new(),
                sections,
                ..Default::default()
            },
            ..Default::default()
        };

        let mut state = AppState::test_new();
        state.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            26,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::themed_by_default(&config, true);
        let area = Rect::new(0, 0, 106, 40);
        crate::ui::compute_view(&mut state, area);
        (state, area)
    }

    fn hide_section(cells: u16) -> crate::config::ShellBarSectionConfig {
        let mut section = inert_section(cells);
        section.action.kind = "hide".to_string();
        section
    }

    // TP-CHROME-142: a press on a hide section switches its *own* edge off —
    // resolved on the bottom bar, so a resolution that hardcoded an edge
    // cannot pass. The second gesture stays inert: hide opens nothing, so it
    // has no second presentation to offer, and the bar is chrome — an event
    // that fell through would act on the surface behind it.
    #[test]
    fn a_press_on_a_hide_section_names_its_own_edge() {
        let (state, area) = state_with_divided_bottom_bar(vec![hide_section(10)]);
        let inside = Position::new(4, area.height - 1);

        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Primary),
            BarSectionClick::HideBar {
                edge: crate::ui::shell::BarEdge::Bottom
            },
            "the press names the edge the section actually sits on"
        );
        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Secondary),
            BarSectionClick::Inert,
            "hide has nothing to present, and the bar stays chrome"
        );
    }

    // TP-CHROME-142: the state half. One edge goes quiet, the others stay,
    // and the global key over that partial state restores rather than hides.
    #[test]
    fn hiding_one_edge_leaves_the_others_and_the_key_restores() {
        let mut state = AppState::test_new();
        state.shell_presentation.set_bars(bars_on_two_edges());

        assert!(state.hide_bar_edge(crate::ui::shell::BarEdge::Top));
        let drawn = state
            .shell_presentation
            .bars()
            .visible(false, state.shell_presentation.toggled_off());
        assert!(!drawn.top.enabled(), "the named edge is quiet");
        assert!(drawn.right.enabled(), "its neighbour still draws");

        assert!(state.toggle_bars(), "the key over a partial state acts");
        assert!(
            state.shell_presentation.toggled_off().is_empty(),
            "and it restores — the press means \"give me my bars back\""
        );
    }

    /// G8 · the frame is chrome; the address stays the section's. A press on
    /// a grouped run's middle member fires that member's own action.
    #[test]
    fn a_press_inside_a_group_lands_on_the_member_it_is_over() {
        // TP-CHROME-145.
        let mut first = popup_section(9, &["one"]);
        first.group = "sys".to_string();
        let mut second = popup_section(9, &["two"]);
        second.group = "sys".to_string();
        let mut third = popup_section(9, &["three"]);
        third.group = "sys".to_string();
        let (state, _) = state_with_divided_top_bar_of_size(vec![first, second, third], 3);

        match state.bar_section_click_at(Position::new(13, 1), SectionGesture::Primary) {
            BarSectionClick::OpenPopup { argv, .. } => assert_eq!(argv, vec!["two".to_string()]),
            other => panic!("the middle member answers with its own action, got {other:?}"),
        }
    }

    fn popup_section(cells: u16, argv: &[&str]) -> crate::config::ShellBarSectionConfig {
        let mut section = inert_section(cells);
        section.action.kind = "popup".to_string();
        section.action.argv = argv.iter().map(|argument| argument.to_string()).collect();
        section
    }

    fn plugin_section(cells: u16, command: &str) -> crate::config::ShellBarSectionConfig {
        let mut section = inert_section(cells);
        section.action.kind = "plugin".to_string();
        section.action.command = command.to_string();
        section
    }

    // TC-B1, end to end: the size travels config -> derivation -> hit -> intent
    // without any layer in between quietly dropping it. Each of those hops was
    // a place it could have been lost, and losing it looks exactly like the
    // default: a popup at half the screen, with nothing to say why.
    #[test]
    fn a_press_carries_the_popup_size_the_person_asked_for() {
        let mut sized = popup_section(10, &["btop"]);
        sized.action.width = Some(crate::popup_size::PopupSize::Percent(80));
        sized.action.height = Some(crate::popup_size::PopupSize::Cells(30));
        let (state, _) = state_with_divided_top_bar(vec![sized]);

        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::OpenPopup {
                argv: vec!["btop".to_string()],
                width: Some(crate::popup_size::PopupSize::Percent(80)),
                height: Some(crate::popup_size::PopupSize::Cells(30)),
            },
            "the size must reach the layer that opens the popup, unresolved"
        );
    }

    fn inert_section(cells: u16) -> crate::config::ShellBarSectionConfig {
        crate::config::ShellBarSectionConfig {
            kind: "fixed".to_string(),
            cells,
            ..Default::default()
        }
    }

    // TC-69-6 · THE USER'S OWN CONDITION, in their words: "hepsi ayri ayri
    // tiklanabilir olmali". Raising a ceiling proves nothing about whether the
    // twelfth icon answers — a hit test that walked a hardcoded eight would
    // leave sections nine through twelve drawn, addressable in config, and
    // silently dead under the pointer.
    //
    // Each section carries its OWN command, so the assertion is not merely
    // "something answered" but "this one answered, not its neighbour". That is
    // the difference between twelve hit areas and one wide one.
    // TP-CHROME-74: every section of a bar raised past eight answers at its own
    // index with its own command.
    #[test]
    fn every_section_of_a_raised_bar_answers_with_its_own_command() {
        const COUNT: usize = 12;
        const CELLS: u16 = 8;

        let sections = (0..COUNT)
            .map(|index| {
                let mut section = popup_section(CELLS, &[]);
                section.action.argv = vec![format!("cmd-{index}")];
                section
            })
            .collect::<Vec<_>>();
        let config = crate::config::ShellBarsConfig {
            top: crate::config::ShellBarConfig {
                enabled: true,
                size: 1,
                border: Some(false),
                color: String::new(),
                gradient: Vec::new(),
                sections,
                max_sections: COUNT as u16,
                hide_when_focused: false,
                style: String::new(),
                background: String::new(),
            },
            ..Default::default()
        };

        let mut state = AppState::test_new();
        state.shell_presentation = crate::ui::shell::ShellPresentationState::from_restored(
            26,
            false,
            None,
            crate::ui::shell::ShellBars::from_config(&config),
        );
        state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::themed_by_default(&config, true);
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));

        let mut answered = Vec::new();
        for index in 0..COUNT {
            // The middle of each section, so the assertion cannot pass by
            // landing on a boundary that happens to belong to a neighbour.
            let column = (index as u16) * CELLS + CELLS / 2;
            match state.bar_section_click_at(Position::new(column, 0), SectionGesture::Primary) {
                BarSectionClick::OpenPopup { argv, .. } => answered.push(argv.join(" ")),
                other => panic!("section {index} at column {column} answered {other:?}"),
            }
        }

        let expected = (0..COUNT)
            .map(|index| format!("cmd-{index}"))
            .collect::<Vec<_>>();
        assert_eq!(
            answered, expected,
            "each section must answer at its own index with its own command"
        );
    }

    fn secondary_section(
        cells: u16,
        argv: &[&str],
        presentation: &str,
    ) -> crate::config::ShellBarSectionConfig {
        let mut section = popup_section(cells, argv);
        section.action.secondary = presentation.to_string();
        section
    }

    fn two_gesture_section(cells: u16, argv: &[&str]) -> crate::config::ShellBarSectionConfig {
        secondary_section(cells, argv, "tab")
    }

    // TC-67-6/TC-67-7 · one section, two answers, and neither steals the other.
    // The argv is asserted on BOTH arms rather than only on the new one: a
    // secondary press that opened an empty command would look like a tab that
    // simply exited, which is indistinguishable from a program that finished.
    // TP-CHROME-62: a section answers its primary and secondary gestures
    // differently, from the same command.
    #[test]
    fn the_two_gestures_on_one_section_ask_for_different_presentations() {
        let (state, _) = state_with_divided_top_bar(vec![two_gesture_section(10, &["btop", "-t"])]);
        let inside = Position::new(4, 0);

        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Primary),
            BarSectionClick::OpenPopup {
                argv: vec!["btop".to_string(), "-t".to_string()],
                width: None,
                height: None,
            },
            "the second answer must not take the first one's place"
        );
        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Secondary),
            BarSectionClick::OpenTab {
                argv: vec!["btop".to_string(), "-t".to_string()],
            },
            "the command must survive the hop to the second presentation, whole"
        );
    }

    // TC-67-8 · a section that answers the second gesture with nothing consumes
    // the press anyway. Falling through would run whatever is under the bar,
    // which is the surface the person was demonstrably not pointing at.
    //
    // Both spellings of "nothing happens" are pinned, because they arrive here
    // by different roads and only one of them still exists. A section that
    // names no presentation now opens the menu; the section that stays silent
    // is the one whose file says `"none"`. Neither may reach `Elsewhere`.
    // TP-CHROME-63: a secondary press that opens nothing is consumed, not
    // passed through.
    #[test]
    fn a_secondary_press_on_a_section_with_one_answer_is_consumed() {
        let (unwritten, _) = state_with_divided_top_bar(vec![popup_section(10, &["btop"])]);
        let (silenced, _) =
            state_with_divided_top_bar(vec![secondary_section(10, &["btop"], "none")]);

        assert!(
            matches!(
                unwritten.bar_section_click_at(Position::new(4, 0), SectionGesture::Secondary),
                BarSectionClick::OpenMenu { .. }
            ),
            "a section that named no presentation asks rather than doing nothing"
        );
        assert_eq!(
            silenced.bar_section_click_at(Position::new(4, 0), SectionGesture::Secondary),
            BarSectionClick::Inert,
            "inert, not Elsewhere: the bar still owns the event it does not act on"
        );
    }

    // TN-2.2/TN-2.3/TN-2.4 · every presentation the grammar accepts reaches a
    // different intent, from the same press on the same kind of section.
    //
    // Written as one table rather than four tests because the property is the
    // mapping itself: a presentation that silently landed on its neighbour's
    // intent would be a section that does the wrong thing while every
    // individual test still passed. The `"tab"` row is the backward
    // compatibility gate — a file written before the menu existed must keep
    // opening a tab directly, not start asking.
    // TP-CHROME-112: each secondary presentation resolves to its own intent,
    // and `"tab"` keeps opening a tab without asking.
    #[test]
    fn each_secondary_presentation_reaches_its_own_intent() {
        let argv = vec!["btop".to_string()];
        let cases: &[(&str, BarSectionClick)] = &[
            ("tab", BarSectionClick::OpenTab { argv: argv.clone() }),
            ("split", BarSectionClick::OpenSplit { argv: argv.clone() }),
            ("none", BarSectionClick::Inert),
            (
                "menu",
                BarSectionClick::OpenMenu {
                    argv: argv.clone(),
                    width: None,
                    height: None,
                    popup_open: false,
                    edge: crate::ui::shell::BarEdge::Top,
                },
            ),
        ];

        for (presentation, expected) in cases {
            let (state, _) =
                state_with_divided_top_bar(vec![secondary_section(10, &["btop"], presentation)]);
            assert_eq!(
                &state.bar_section_click_at(Position::new(4, 0), SectionGesture::Secondary),
                expected,
                "`secondary = \"{presentation}\"` must reach its own intent and no other"
            );
        }
    }

    // TN-G5.1 · the two actions that open nothing reach their own intents, and
    // offer no second presentation.
    //
    // The secondary half is the part worth pinning. Both of these are a single
    // thing that happens, so a menu offering "in a popup / in a tab / in a
    // split" would be listing three places to put something that goes to none
    // of them — and the menu is now what a section does by default, so staying
    // inert here had to be written down rather than inherited.
    // TP-CHROME-121: an action that opens nothing answers the first gesture and
    // consumes the second.
    #[test]
    fn an_action_that_opens_nothing_offers_no_second_presentation() {
        let mut run = popup_section(10, &["true"]);
        run.action.kind = "run".to_string();
        let (run_state, _) = state_with_divided_top_bar(vec![run]);

        let mut go = popup_section(10, &["true"]);
        go.action.kind = "workspace".to_string();
        go.action.argv = Vec::new();
        go.action.name = "herdr".to_string();
        let (go_state, _) = state_with_divided_top_bar(vec![go]);

        let at = Position::new(4, 0);
        assert_eq!(
            run_state.bar_section_click_at(at, SectionGesture::Primary),
            BarSectionClick::RunCommand {
                argv: vec!["true".to_string()],
            }
        );
        assert_eq!(
            go_state.bar_section_click_at(at, SectionGesture::Primary),
            BarSectionClick::FocusWorkspace {
                name: "herdr".to_string(),
            }
        );

        for (name, state) in [("run", &run_state), ("workspace", &go_state)] {
            assert_eq!(
                state.bar_section_click_at(at, SectionGesture::Secondary),
                BarSectionClick::Inert,
                "a {name} action has nothing to re-present, and the press still \
                 belongs to the bar"
            );
        }
    }

    // TN-2.6 · the menu is told whether the popup slot was taken, at the moment
    // the press resolved.
    //
    // Carried rather than looked up later, for the reason every other menu
    // carries its own identity: a menu outlives the frame it was opened from,
    // and a popup can close while it is still on screen. Reading the slot when
    // the item fires would enable a row against a state nobody saw.
    // TP-CHROME-113: a bar section menu records whether the popup slot was
    // taken when it opened.
    #[test]
    fn a_bar_section_menu_records_whether_the_popup_slot_was_taken() {
        let (mut state, _) = state_with_divided_top_bar(vec![popup_section(10, &["btop"])]);
        let free = state.bar_section_click_at(Position::new(4, 0), SectionGesture::Secondary);
        assert_eq!(
            free,
            BarSectionClick::OpenMenu {
                argv: vec!["btop".to_string()],
                width: None,
                height: None,
                popup_open: false,
                edge: crate::ui::shell::BarEdge::Top,
            },
            "control: with the slot free, nothing in the menu is closed off"
        );

        state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id: crate::layout::PaneId::alloc(),
            terminal_id: crate::terminal::TerminalId::alloc(),
            width: None,
            height: None,
        });

        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Secondary),
            BarSectionClick::OpenMenu {
                argv: vec!["btop".to_string()],
                width: None,
                height: None,
                popup_open: true,
                edge: crate::ui::shell::BarEdge::Top,
            },
            "the menu must learn the slot is taken, or it offers a popup it cannot open"
        );
    }

    // TP-CHROME-150: an actionless stretch of the bar answers the second
    // gesture with the config door — and names the edge it was pressed on,
    // through the same `bar_edge_for` resolution every acting arm uses. The
    // first gesture stays inert: the strip is chrome, not a button.
    #[test]
    fn a_secondary_press_on_an_actionless_section_asks_for_the_panel() {
        let (state, _) = state_with_divided_top_bar(vec![crate::config::ShellBarSectionConfig {
            kind: "fill".to_string(),
            weight: 1,
            ..Default::default()
        }]);
        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Secondary),
            BarSectionClick::ConfigureBar {
                edge: crate::ui::shell::BarEdge::Top
            },
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::Inert,
            "the first gesture is not a door"
        );
    }

    // TN-2.7 · the primary gesture is untouched by all of the above.
    //
    // A control, and not a redundant one: every change in this file was made on
    // the secondary arm of the same `match`, and nothing else asserts that the
    // left press still means what it meant. A refactor that folded the two arms
    // together would pass every test above.
    // TP-CHROME-114: growing the second gesture leaves the first one alone.
    #[test]
    fn the_primary_gesture_is_unchanged_by_the_secondary_ones() {
        for presentation in ["", "tab", "split", "menu", "none"] {
            let (state, _) =
                state_with_divided_top_bar(vec![secondary_section(10, &["btop"], presentation)]);
            assert_eq!(
                state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
                BarSectionClick::OpenPopup {
                    argv: vec!["btop".to_string()],
                    width: None,
                    height: None,
                },
                "a left press opens the popup whatever the right press was told to do"
            );
        }
    }

    // TC-67-9/TC-67-10 · the guarantees the primary gesture already had must
    // hold for the second one. Both were separately capable of regressing: a
    // second code path is a second place to forget the generation gate, and a
    // secondary press that ignored it would open a tab for whatever section
    // happens to sit at those coordinates now.
    // TP-CHROME-64: the secondary gesture resolves against the live generation
    // only, and claims nothing outside a section.
    #[test]
    fn a_secondary_press_obeys_the_same_geometry_rules_as_the_first() {
        let (mut state, _) = state_with_divided_top_bar(vec![two_gesture_section(10, &["btop"])]);
        let inside = Position::new(4, 0);

        assert_eq!(
            state.bar_section_click_at(Position::new(60, 0), SectionGesture::Secondary),
            BarSectionClick::Elsewhere,
            "inside the bar but outside every section is not the section's"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(60, 20), SectionGesture::Secondary),
            BarSectionClick::Elsewhere,
            "nowhere near the bar must not reach a bar action"
        );

        assert!(
            matches!(
                state.bar_section_click_at(inside, SectionGesture::Secondary),
                BarSectionClick::OpenTab { .. }
            ),
            "control: the press must resolve against the live geometry"
        );
        state.view.shell.generation = state.view.shell.generation.wrapping_add(1);
        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Secondary),
            BarSectionClick::Elsewhere,
            "coordinates from a layout that no longer exists must open nothing"
        );
    }

    // TC-67-19 · the control the routing layer's comment leans on: whether a
    // position is over a section at all is positional, so it cannot depend on
    // which gesture asked. `handle_bar_section_mouse` probes with one gesture
    // to answer that question for events it will not run, and this is what
    // makes that sound rather than merely plausible.
    // TP-CHROME-65: whether a press is over a section does not depend on the
    // gesture.
    #[test]
    fn whether_a_press_is_over_a_section_does_not_depend_on_the_gesture() {
        let (state, _) =
            state_with_divided_top_bar(vec![two_gesture_section(10, &["btop"]), inert_section(10)]);

        for position in [
            Position::new(4, 0),
            Position::new(14, 0),
            Position::new(60, 0),
            Position::new(60, 20),
        ] {
            let primary = state.bar_section_click_at(position, SectionGesture::Primary);
            let secondary = state.bar_section_click_at(position, SectionGesture::Secondary);
            assert_eq!(
                matches!(primary, BarSectionClick::Elsewhere),
                matches!(secondary, BarSectionClick::Elsewhere),
                "the two gestures disagreed about whether {position:?} is a section \
                 at all: {primary:?} vs {secondary:?}"
            );
        }
    }

    // TC-67-11 · a tab is not scarce the way the single popup slot is, so an
    // open popup must not refuse one. Pinned so the guard is not added later by
    // symmetry with the arm above it.
    // TP-CHROME-66: an open popup does not block the secondary presentation.
    #[test]
    fn an_open_popup_does_not_block_opening_a_tab() {
        let (mut state, _) = state_with_divided_top_bar(vec![two_gesture_section(10, &["btop"])]);
        state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id: crate::layout::PaneId::alloc(),
            terminal_id: crate::terminal::TerminalId::alloc(),
            width: None,
            height: None,
        });

        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::PopupAlreadyOpen,
            "control: the popup slot is still occupied"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Secondary),
            BarSectionClick::OpenTab {
                argv: vec!["btop".to_string()],
            },
            "a tab costs a tab; refusing one would refuse something harmless"
        );
    }

    // TC-66-11/TC-66-12 · a plugin section answers the first gesture and
    // consumes the second. Consumed rather than passed through, for the same
    // reason every other bar press is: an event falling through chrome acts on
    // the surface underneath, which is demonstrably not the one being pointed
    // at (CL12). And Inert rather than a tab, because the bar does not open
    // what a plugin action opens — the manifest's own pane placement does, so
    // offering to re-present it would be a promise this layer cannot keep.
    // TP-CHROME-82: a primary press on a plugin section asks to invoke it, and
    // a secondary press is consumed without inventing a second presentation.
    #[test]
    fn a_plugin_section_answers_the_first_gesture_and_consumes_the_second() {
        let (state, _) =
            state_with_divided_top_bar(vec![plugin_section(10, "jt.command-palette.open")]);
        let inside = Position::new(4, 0);

        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Primary),
            BarSectionClick::InvokePlugin {
                action: "jt.command-palette.open".to_string(),
            },
            "the id must survive the hop from chrome to intent, whole"
        );
        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Secondary),
            BarSectionClick::Inert,
            "inert, not Elsewhere: the bar still owns the event it will not act on"
        );
    }

    // TC-66-13 · the popup guard must NOT be copied here by symmetry with the
    // arm above it. The popup slot holds exactly one pane and a second press
    // would drop somebody's open work; a plugin action is not that slot — it
    // opens its own pane, or a split, or nothing at all. Refusing something
    // harmless builds a wall the person cannot see the reason for.
    // TP-CHROME-83: an open popup does not block invoking a plugin action.
    #[test]
    fn an_open_popup_does_not_block_invoking_a_plugin() {
        let (mut state, _) = state_with_divided_top_bar(vec![
            plugin_section(10, "jt.command-palette.open"),
            popup_section(10, &["btop"]),
        ]);
        state.popup_pane = Some(crate::app::state::PopupPaneState {
            pane_id: crate::layout::PaneId::alloc(),
            terminal_id: crate::terminal::TerminalId::alloc(),
            width: None,
            height: None,
        });

        assert_eq!(
            state.bar_section_click_at(Position::new(14, 0), SectionGesture::Primary),
            BarSectionClick::PopupAlreadyOpen,
            "control: the popup slot really is occupied, so the next assertion \
             is about the plugin arm rather than about an empty slot"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::InvokePlugin {
                action: "jt.command-palette.open".to_string(),
            },
            "a plugin action is not the popup slot, so the slot cannot refuse it"
        );
    }

    // TC-66-14 · two action kinds share one index-addressed table. If the
    // chrome list and the section list ever drift apart, a press lands on the
    // right rectangle and runs the wrong neighbour's command — which looks like
    // a working button doing the wrong thing, the hardest kind of bug to
    // believe. Asserting both directions is what makes the alignment a
    // behaviour rather than a coincidence of ordering.
    // TP-CHROME-84: a plugin section and a popup section on one bar each keep
    // their own answer at their own index.
    #[test]
    fn a_plugin_section_and_a_popup_section_keep_their_own_answers() {
        let (state, _) = state_with_divided_top_bar(vec![
            popup_section(10, &["htop"]),
            plugin_section(10, "persiyanov.reviewr.toggle"),
            popup_section(10, &["btop"]),
        ]);

        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::OpenPopup {
                argv: vec!["htop".to_string()],
                width: None,
                height: None,
            },
            "index 0 keeps its own command"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(14, 0), SectionGesture::Primary),
            BarSectionClick::InvokePlugin {
                action: "persiyanov.reviewr.toggle".to_string(),
            },
            "index 1 answers with the plugin action written at index 1"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(24, 0), SectionGesture::Primary),
            BarSectionClick::OpenPopup {
                argv: vec!["btop".to_string()],
                width: None,
                height: None,
            },
            "index 2 is not shifted by the plugin section before it"
        );
    }

    // TA-1/TA-2/TA-5 · a press resolves to the action of the section it landed
    // in; a press that landed in no section resolves to nothing at all, so the
    // bar's own owner still gets it.
    #[test]
    fn a_press_resolves_the_action_of_the_section_it_landed_in() {
        // Two ten-cell sections at the left of a 106-cell bar: the rest of the
        // bar is bar, but no section, which is the case TA-5 needs to exist.
        let (state, _) =
            state_with_divided_top_bar(vec![popup_section(10, &["btop"]), inert_section(10)]);

        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::OpenPopup {
                argv: vec!["btop".to_string()],
                width: None,
                height: None,
            },
            "a press in the first section must ask for that section's command"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(14, 0), SectionGesture::Primary),
            BarSectionClick::Inert,
            "a section with no action is consumed, never fallen through"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(60, 0), SectionGesture::Primary),
            BarSectionClick::Elsewhere,
            "inside the bar but outside every section belongs to the bar, not \
             to a section that is not there"
        );
        assert_eq!(
            state.bar_section_click_at(Position::new(60, 20), SectionGesture::Primary),
            BarSectionClick::Elsewhere,
            "a press nowhere near the bar must not reach a bar action"
        );
    }

    // TA-4 · positional authority is only ever against the live generation.
    // Skipping the gate would let coordinates from a layout that no longer
    // exists run a command, which is CL5's whole point.
    #[test]
    fn a_press_from_a_vanished_generation_runs_nothing() {
        let (mut state, _) = state_with_divided_top_bar(vec![popup_section(10, &["btop"])]);
        let inside = Position::new(4, 0);

        assert!(
            matches!(
                state.bar_section_click_at(inside, SectionGesture::Primary),
                BarSectionClick::OpenPopup { .. }
            ),
            "control: the press must resolve against the live geometry"
        );

        state.view.shell.generation = state.view.shell.generation.wrapping_add(1);

        assert_eq!(
            state.bar_section_click_at(inside, SectionGesture::Primary),
            BarSectionClick::Elsewhere,
            "hit areas from an older generation must resolve to nothing"
        );
    }

    // TA-3 · a bar press may not open a second popup, and may not close the
    // one that is already there. Somebody's open work is not undoable.
    #[test]
    fn a_press_while_a_popup_is_open_neither_opens_nor_closes_one() {
        let (mut state, _) = state_with_divided_top_bar(vec![popup_section(10, &["btop"])]);
        let popup = crate::app::state::PopupPaneState {
            pane_id: crate::layout::PaneId::alloc(),
            terminal_id: crate::terminal::TerminalId::alloc(),
            width: None,
            height: None,
        };
        state.popup_pane = Some(popup.clone());

        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::PopupAlreadyOpen
        );
        assert_eq!(
            state.popup_pane,
            Some(popup),
            "resolving the press must leave the open popup exactly as it was"
        );
    }

    // A section whose action config could not be read stays a section: it is
    // drawn, it consumes its own presses, and it does nothing. The alternative
    // — falling through — would act on the surface behind the chrome.
    #[test]
    fn a_section_whose_action_was_refused_is_inert_rather_than_transparent() {
        let mut refused = popup_section(10, &[]);
        refused.action.argv = Vec::new();
        let (state, _) = state_with_divided_top_bar(vec![refused]);

        assert_eq!(
            state.bar_section_click_at(Position::new(4, 0), SectionGesture::Primary),
            BarSectionClick::Inert
        );
    }

    // SF5.2 characterization: dock resize reuses the SAME region-generic SF3
    // `ResizeTransaction` with the dock's frozen 3..=9 track bounds — no
    // dock-specific drag state exists. Valid RED was refuted by source: the
    // reducer is generic over `DividerId` region pairs by construction.
    #[test]
    fn dock_resize_and_collapse_use_shared_transaction() {
        let divider = DividerId::new(
            RegionId::AppDock,
            RegionId::WorkspaceStage,
            ShellDirection::Horizontal,
        )
        .expect("dock divider");
        let bounds = ResizeBounds::new(3, 9, 1, 80).expect("dock bounds");

        // Growing far beyond the maximum clamps to the frozen 9-cell cap.
        let mut transaction = ResizeTransaction::begin(divider, 7, Position::new(5, 3), [5, 75]);
        let tx = transaction.as_mut().expect("dock transaction");
        assert!(tx.preview(Position::new(200, 3), bounds));
        let update = ResizeTransaction::commit(&mut transaction, 7);
        assert_eq!(
            update.decision(),
            ResizeDecision::Committed([9, 71]),
            "the shared transaction clamps the dock to its maximum"
        );

        // Shrinking below the minimum clamps to the frozen 3-cell floor.
        let mut transaction = ResizeTransaction::begin(divider, 7, Position::new(5, 3), [5, 75]);
        let tx = transaction.as_mut().expect("dock transaction");
        assert!(tx.preview(Position::new(0, 3), bounds));
        let update = ResizeTransaction::commit(&mut transaction, 7);
        assert_eq!(update.decision(), ResizeDecision::Committed([3, 77]));

        // A stale view generation stays inert — the same guard every shell
        // divider already obeys.
        let mut transaction = ResizeTransaction::begin(divider, 7, Position::new(5, 3), [5, 75]);
        let tx = transaction.as_mut().expect("dock transaction");
        assert!(tx.preview(Position::new(200, 3), bounds));
        let update = ResizeTransaction::commit(&mut transaction, 8);
        assert_eq!(update.decision(), ResizeDecision::Inert);
    }

    // SF4.2-06 companion characterization: the collapsed-sidebar guard inside
    // `on_sidebar_divider` is load-bearing but was previously unpinned. The
    // adversarial fixture keeps a stale non-zero sidebar rect in the view so
    // ONLY the collapse guard stands between hidden geometry and a resize
    // capture; deleting that guard must fail this test.
    #[test]
    fn collapsed_sidebar_exposes_no_divider_capture() {
        let mut state = AppState::test_new();
        state.view.sidebar_rect = ratatui::layout::Rect::new(0, 0, 26, 24);
        let divider_col = 25;

        state.sidebar_collapsed = false;
        assert!(
            state.on_sidebar_divider(divider_col, 5),
            "control: the probe must hit the live divider column"
        );

        state.sidebar_collapsed = true;
        assert!(
            !state.on_sidebar_divider(divider_col, 5),
            "a collapsed sidebar must never expose divider capture authority"
        );
    }

    // SF4.2-04 characterization: an active divider capture already owns every
    // move/up event through `DragState`, independent of coordinates. This is
    // GREEN by intent (SF1 precedent): drag routing never re-resolves rects,
    // and a left-down clears any lingering selection before a capture can
    // begin, so no competing owner is reachable mid-gesture.
    #[test]
    fn capture_owns_move_and_up_outside_original_rect() {
        use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

        fn mouse_event(kind: MouseEventKind, column: u16, row: u16) -> MouseEvent {
            MouseEvent {
                kind,
                column,
                row,
                modifiers: crossterm::event::KeyModifiers::empty(),
            }
        }

        let mut state = AppState::test_new();
        state.workspaces = vec![
            crate::workspace::Workspace::test_new("one"),
            crate::workspace::Workspace::test_new("two"),
        ];
        state.active = Some(0);
        state.selected = 1;
        crate::ui::compute_view(&mut state, Rect::new(0, 0, 106, 40));
        state.session_dirty = false;
        let mut terminal_runtimes = crate::terminal::TerminalRuntimeRegistry::new();

        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            crate::app::LOCAL_INPUT_SOURCE,
            mouse_event(MouseEventKind::Down(MouseButton::Left), 25, 5),
        );
        assert!(
            state.shell_resize_active(),
            "a divider left-down must begin the resize capture"
        );

        // Drag over the pane area, far outside the divider rect.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            crate::app::LOCAL_INPUT_SOURCE,
            mouse_event(MouseEventKind::Drag(MouseButton::Left), 30, 15),
        );
        assert_eq!(state.shell_resize_preview_width(), Some(31));
        assert!(state.selection.is_none(), "capture drag must not select");

        // Drag over the sidebar workspace rows: no press, reorder, or
        // selection movement may start under the active capture.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            crate::app::LOCAL_INPUT_SOURCE,
            mouse_event(MouseEventKind::Drag(MouseButton::Left), 5, 3),
        );
        assert_eq!(
            (state.shell_resize_preview_width(), state.selected),
            (Some(state.sidebar_min_width), 1)
        );

        // Drag to the far corner clamps at the bound and keeps ownership.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            crate::app::LOCAL_INPUT_SOURCE,
            mouse_event(MouseEventKind::Drag(MouseButton::Left), 100, 35),
        );
        assert_eq!(
            state.shell_resize_preview_width(),
            Some(state.sidebar_max_width)
        );

        // Releasing outside the original rect commits exactly once.
        let _ = state.handle_mouse(
            &mut terminal_runtimes,
            crate::app::LOCAL_INPUT_SOURCE,
            mouse_event(MouseEventKind::Up(MouseButton::Left), 100, 35),
        );
        assert_eq!(
            (
                state.sidebar_width,
                state.shell_resize_active(),
                state.session_dirty,
                state.selected,
            ),
            (state.sidebar_max_width, false, true, 1)
        );
    }

    #[test]
    fn shell_input_router_follows_frozen_precedence() {
        struct PrecedenceRow {
            name: &'static str,
            context: ShellInputRouteContext,
            expected: ShellInputOwner,
        }

        let rows = [
            PrecedenceRow {
                name: "topmost blocking overlay owns input ahead of every lower tier",
                context: ShellInputRouteContext {
                    topmost_overlay: true,
                    active_capture: true,
                    topmost_hit: Some(RegionId::WorkspaceStage),
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::TopmostOverlay,
            },
            PrecedenceRow {
                name: "active capture owns input under an absent overlay",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: true,
                    topmost_hit: Some(RegionId::WorkspaceStage),
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::ActiveCapture,
            },
            PrecedenceRow {
                name: "resolved z-ordered topmost hit owns input under overlay and capture",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: Some(RegionId::LeftPanel),
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::TopmostHit(RegionId::LeftPanel),
            },
            PrecedenceRow {
                name: "focused component owns non-positional input without a hit",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: true,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::FocusedComponent,
            },
            PrecedenceRow {
                name: "page shortcut owner precedes global shortcuts",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: false,
                    page_shortcut: true,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::PageShortcut,
            },
            PrecedenceRow {
                name: "global application shortcuts are the last acting tier",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: false,
                    page_shortcut: false,
                    global_shortcut: true,
                },
                expected: ShellInputOwner::GlobalShortcut,
            },
            PrecedenceRow {
                name: "input with no owner fails closed instead of reaching hidden surfaces",
                context: ShellInputRouteContext {
                    topmost_overlay: false,
                    active_capture: false,
                    topmost_hit: None,
                    focused_component: false,
                    page_shortcut: false,
                    global_shortcut: false,
                },
                expected: ShellInputOwner::FailClosed,
            },
        ];

        for row in rows {
            assert_eq!(
                route_shell_input(row.context),
                row.expected,
                "frozen precedence row: {}",
                row.name
            );
        }
    }
}
