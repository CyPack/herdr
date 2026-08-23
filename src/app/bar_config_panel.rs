//! Client-local bar configuration panel model: right-press a strip and every
//! knob it answers to is one popup away — the switch, style, border, size,
//! colour, backdrop — plus the scope choice ("this bar" / "all bars") and the
//! other edges' switches, so nothing about bars has to be hunted through a
//! settings file (TP-CHROME-150). The model is pure data: opening snapshots
//! the loaded bars, every adjustment edits a draft, and the diff between the
//! two is exactly what Apply persists (TP-CHROME-151/152).

use crate::config::{ManagedBarOverride, ShellBarConfig, ShellBarsConfig};
use crate::ui::shell::BarEdge;

/// Every edge, in the order the panel lists them.
pub(crate) const BAR_PANEL_EDGES: [BarEdge; 4] =
    [BarEdge::Top, BarEdge::Bottom, BarEdge::Left, BarEdge::Right];

/// The looks the style row cycles through — the same closed set the spec
/// promises, in the order the docs teach them.
pub(crate) const BAR_STYLE_CHOICES: [&str; 4] = ["framed", "islands", "plain", "pills"];

pub(crate) const fn bar_edge_name(edge: BarEdge) -> &'static str {
    match edge {
        BarEdge::Top => "top",
        BarEdge::Bottom => "bottom",
        BarEdge::Left => "left",
        BarEdge::Right => "right",
    }
}

/// One selectable row of the panel, top to bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BarPanelRow {
    Enabled,
    Style,
    Border,
    Size,
    Color,
    Background,
    Scope,
    OtherBar(BarEdge),
    Apply,
    Cancel,
}

/// The rows the panel shows for a bar on `edge` — six knobs for that edge,
/// the scope choice, the OTHER three edges' switches, then the two verbs.
pub(crate) fn panel_rows(edge: BarEdge) -> Vec<BarPanelRow> {
    let mut rows = vec![
        BarPanelRow::Enabled,
        BarPanelRow::Style,
        BarPanelRow::Border,
        BarPanelRow::Size,
        BarPanelRow::Color,
        BarPanelRow::Background,
        BarPanelRow::Scope,
    ];
    for other in BAR_PANEL_EDGES {
        if other != edge {
            rows.push(BarPanelRow::OtherBar(other));
        }
    }
    rows.push(BarPanelRow::Apply);
    rows.push(BarPanelRow::Cancel);
    rows
}

/// The closed set of action kinds the edit form cycles through — the same
/// kinds a press on the bar resolves, plus `none` to disarm a press. Free
/// text here would let a typo write a kind nothing fires (the meter-display
/// lesson: a closed set refuses at the edge, not at draw time).
pub(crate) const EDIT_KIND_CHOICES: [&str; 5] = ["popup", "run", "workspace", "plugin", "none"];

/// Which part of the edit form holds the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BarAppEditField {
    Kind,
    Value,
    Width,
    Height,
    Save,
    Cancel,
}

impl BarAppEditField {
    pub(crate) const ALL: [Self; 6] = [
        Self::Kind,
        Self::Value,
        Self::Width,
        Self::Height,
        Self::Save,
        Self::Cancel,
    ];
}

/// The Apps face's mini editor: rebind what a press on one identified row
/// does. A client-local draft — nothing reaches the disk until Save, and
/// Save rides a read-modify-write because the overlay serializer replaces
/// an edge's whole `section_overrides` array (TP-CHROME-166).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BarAppEditForm {
    /// The section id the override will bind to, resolved when the form
    /// opened: the row's action-carrying section, or its first identified
    /// one when no action is bound yet.
    pub section_id: String,
    /// What the row shows, echoed in the form header.
    pub shows: String,
    /// Index into `EDIT_KIND_CHOICES`.
    pub kind_choice: usize,
    /// One line of text, read per kind: popup/run split it into argv on
    /// whitespace, workspace reads a name, plugin reads an action id, and
    /// none ignores it.
    pub value_text: String,
    pub width_text: String,
    pub height_text: String,
    pub field: BarAppEditField,
    /// Why the last Save was refused — cleared by the next edit.
    pub validation_error: Option<String>,
}

/// Blocking client-local panel state. Owns no watcher, worker, process, pane,
/// or server state; closing it discards only presentation data — the draft.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BarConfigPanelState {
    /// Which face is forward (TP-CHROME-153).
    pub tab: BarPanelTab,
    /// The edge whose six knobs the field rows edit.
    pub edge: BarEdge,
    /// The working copy every adjustment edits and the preview draws from.
    pub draft: ShellBarsConfig,
    /// The bars as they were when the panel opened — the Cancel target and
    /// the base every Apply diff is taken against.
    pub original: ShellBarsConfig,
    /// false = Apply writes the focused edge; true = its changes fan out to
    /// every edge.
    pub scope_all: bool,
    pub selected: usize,
    /// The Apps face's editor, when one is open (TP-CHROME-166).
    pub edit: Option<BarAppEditForm>,
}

impl BarConfigPanelState {
    pub(crate) fn open(edge: BarEdge, bars: &ShellBarsConfig) -> Self {
        Self {
            tab: BarPanelTab::Configure,
            edge,
            draft: bars.clone(),
            original: bars.clone(),
            scope_all: false,
            selected: 0,
            edit: None,
        }
    }

    /// Open the editor for the selected Apps row, seeded from what the row
    /// does today. A row whose sections carry no id cannot be re-aimed by
    /// the overlay at all, so the attempt is refused with the hint rather
    /// than opening a form that could never save.
    pub(crate) fn open_edit_form(&mut self) -> Result<(), String> {
        let rows = self.app_rows();
        let Some(row) = rows.get(self.selected) else {
            return Err("no row is selected".to_string());
        };
        let Some(section_id) = row.section_id.clone() else {
            return Err(
                "the overlay binds by id and none of this row's sections carries one; \
                 add id = \"...\" to the section in your config.toml to edit it here"
                    .to_string(),
            );
        };
        // The seed follows the run road's choice: the first action-carrying
        // section speaks for the row; a dead-end row seeds an empty action.
        let bar = edge_config(&self.original, self.edge);
        let seed = row
            .section_indices
            .iter()
            .filter_map(|&idx| bar.sections.get(idx))
            .map(|section| &section.action)
            .find(|action| action_summary(action).is_some())
            .cloned()
            .unwrap_or_default();
        let kind_choice = EDIT_KIND_CHOICES
            .iter()
            .position(|kind| *kind == seed.kind.as_str())
            .unwrap_or(EDIT_KIND_CHOICES.len() - 1); // an unset kind seeds as none
        let value_text = match seed.kind.as_str() {
            "workspace" => seed.name.clone(),
            "plugin" => seed.command.clone(),
            _ => seed.argv.join(" "),
        };
        let size_text = |size: Option<crate::popup_size::PopupSize>| match size {
            Some(crate::popup_size::PopupSize::Cells(cells)) => cells.to_string(),
            Some(crate::popup_size::PopupSize::Percent(percent)) => format!("{percent}%"),
            None => String::new(),
        };
        self.edit = Some(BarAppEditForm {
            section_id,
            shows: row.shows.clone(),
            kind_choice,
            value_text,
            width_text: size_text(seed.width),
            height_text: size_text(seed.height),
            field: BarAppEditField::Kind,
            validation_error: None,
        });
        Ok(())
    }

    /// Move the form cursor one field down (or up), stopping at the ends.
    pub(crate) fn edit_field_step(&mut self, forward: bool) {
        let Some(form) = self.edit.as_mut() else {
            return;
        };
        let fields = BarAppEditField::ALL;
        let Some(at) = fields.iter().position(|field| *field == form.field) else {
            return;
        };
        let next = if forward {
            (at + 1).min(fields.len() - 1)
        } else {
            at.saturating_sub(1)
        };
        form.field = fields[next];
    }

    /// Cycle the kind choice through the closed set, wrapping at both ends.
    pub(crate) fn edit_cycle_kind(&mut self, forward: bool) {
        let Some(form) = self.edit.as_mut() else {
            return;
        };
        let len = EDIT_KIND_CHOICES.len();
        form.kind_choice = if forward {
            (form.kind_choice + 1) % len
        } else {
            (form.kind_choice + len - 1) % len
        };
        form.validation_error = None;
    }

    /// Type into whichever text field holds the cursor.
    pub(crate) fn edit_insert_text(&mut self, text: &str) {
        let Some(form) = self.edit.as_mut() else {
            return;
        };
        match form.field {
            BarAppEditField::Value => form.value_text.push_str(text),
            BarAppEditField::Width => form.width_text.push_str(text),
            BarAppEditField::Height => form.height_text.push_str(text),
            _ => return,
        }
        form.validation_error = None;
    }

    /// Backspace in whichever text field holds the cursor.
    pub(crate) fn edit_delete_char(&mut self) {
        let Some(form) = self.edit.as_mut() else {
            return;
        };
        match form.field {
            BarAppEditField::Value => {
                form.value_text.pop();
            }
            BarAppEditField::Width => {
                form.width_text.pop();
            }
            BarAppEditField::Height => {
                form.height_text.pop();
            }
            _ => return,
        }
        form.validation_error = None;
    }

    /// Parse the form and merge it over the overlay's existing overrides —
    /// the full list the serializer will write. Pure: the disk never
    /// appears here, so the refusals (a bad size, a kind missing its
    /// value) are testable without one.
    pub(crate) fn edit_save_payload(
        &self,
        existing: Vec<crate::config::ManagedSectionOverride>,
    ) -> Result<Vec<crate::config::ManagedSectionOverride>, String> {
        let Some(form) = self.edit.as_ref() else {
            return Err("no editor is open".to_string());
        };
        let kind = EDIT_KIND_CHOICES
            .get(form.kind_choice)
            .copied()
            .unwrap_or("none");
        // Sizes are parsed for every kind so a typo is refused rather than
        // silently carried; they are only written for the kind that reads
        // them.
        let parse_size = |text: &str, name: &str| {
            let text = text.trim();
            if text.is_empty() {
                return Ok(None);
            }
            crate::popup_size::PopupSize::parse_cli(text)
                .map(Some)
                .map_err(|err| format!("{name} {err}"))
        };
        let width = parse_size(&form.width_text, "width:")?;
        let height = parse_size(&form.height_text, "height:")?;
        let value = form.value_text.trim();
        let action = match kind {
            // an empty kind is how a press is disarmed
            "none" => crate::config::ShellBarSectionActionConfig::default(),
            "popup" | "run" => {
                let argv: Vec<String> = value.split_whitespace().map(str::to_string).collect();
                if argv.is_empty() {
                    return Err(format!("a {kind} action needs a command line"));
                }
                crate::config::ShellBarSectionActionConfig {
                    kind: kind.to_string(),
                    argv,
                    width: if kind == "popup" { width } else { None },
                    height: if kind == "popup" { height } else { None },
                    ..Default::default()
                }
            }
            "workspace" => {
                if value.is_empty() {
                    return Err("a workspace action needs the workspace's name".to_string());
                }
                crate::config::ShellBarSectionActionConfig {
                    kind: kind.to_string(),
                    name: value.to_string(),
                    ..Default::default()
                }
            }
            "plugin" => {
                if value.is_empty() {
                    return Err("a plugin action needs its action id".to_string());
                }
                crate::config::ShellBarSectionActionConfig {
                    kind: kind.to_string(),
                    command: value.to_string(),
                    ..Default::default()
                }
            }
            other => return Err(format!("unknown action kind {other:?}")),
        };
        Ok(crate::config::upsert_section_override(
            existing,
            crate::config::ManagedSectionOverride {
                id: form.section_id.clone(),
                action: Some(action),
            },
        ))
    }

    pub(crate) fn rows(&self) -> Vec<BarPanelRow> {
        panel_rows(self.edge)
    }

    /// The Apps inventory reads the ORIGINAL snapshot, never the draft: it
    /// is what the disk's bar does today, and a knob turned on the other
    /// tab must not rewrite history until Apply lands it (TP-CHROME-153).
    pub(crate) fn app_rows(&self) -> Vec<BarAppRow> {
        section_app_rows(edge_config(&self.original, self.edge))
    }

    /// Bring the other face forward. Selection resets: an index carried
    /// across tabs would land on whatever row happens to share the number.
    pub(crate) fn switch_tab(&mut self, tab: BarPanelTab) -> bool {
        if self.tab == tab {
            return false;
        }
        self.tab = tab;
        self.selected = 0;
        true
    }

    /// How many selectable rows the FORWARD tab offers. While the Apps
    /// editor is open its fields are the rows — the mouse map and the
    /// Down-clamp both read this, so the form is reachable by click for
    /// free (TP-CHROME-166).
    pub(crate) fn forward_row_count(&self) -> usize {
        if self.edit.is_some() {
            return BarAppEditField::ALL.len();
        }
        match self.tab {
            BarPanelTab::Apps => self.app_rows().len(),
            BarPanelTab::Configure => self.rows().len(),
        }
    }

    /// Adjust the selected row one step. Returns true when the draft (or the
    /// scope) actually changed, which is when the preview needs a refresh.
    pub(crate) fn adjust_selected(&mut self, forward: bool) -> bool {
        let rows = self.rows();
        let Some(&row) = rows.get(self.selected) else {
            return false;
        };
        self.adjust_row(row, forward)
    }

    /// Adjust one named row. Every row edits exactly what it names: the six
    /// field rows touch only the focused edge's field, an OtherBar row
    /// touches only that edge's switch, Scope flips the fan-out flag, and
    /// the two verbs adjust nothing.
    pub(crate) fn adjust_row(&mut self, row: BarPanelRow, forward: bool) -> bool {
        match row {
            BarPanelRow::Enabled => {
                let bar = edge_config_mut(&mut self.draft, self.edge);
                bar.enabled = !bar.enabled;
                true
            }
            BarPanelRow::Style => {
                let bar = edge_config_mut(&mut self.draft, self.edge);
                bar.style = cycle_style(&bar.style, forward);
                true
            }
            BarPanelRow::Border => {
                let bar = edge_config_mut(&mut self.draft, self.edge);
                bar.border = cycle_border(bar.border, forward);
                true
            }
            BarPanelRow::Size => {
                let bar = edge_config_mut(&mut self.draft, self.edge);
                let stepped = step_size(bar.size, forward);
                let changed = stepped != bar.size;
                bar.size = stepped;
                changed
            }
            BarPanelRow::Color => {
                let original = edge_config(&self.original, self.edge).color.clone();
                let bar = edge_config_mut(&mut self.draft, self.edge);
                let choices = color_choices(&original);
                bar.color = cycle_choice(&bar.color, &choices, forward);
                true
            }
            BarPanelRow::Background => {
                let original = edge_config(&self.original, self.edge).background.clone();
                let bar = edge_config_mut(&mut self.draft, self.edge);
                let choices = background_choices(&original);
                bar.background = cycle_choice(&bar.background, &choices, forward);
                true
            }
            BarPanelRow::Scope => {
                self.scope_all = !self.scope_all;
                true
            }
            BarPanelRow::OtherBar(other) => {
                let bar = edge_config_mut(&mut self.draft, other);
                bar.enabled = !bar.enabled;
                true
            }
            BarPanelRow::Apply | BarPanelRow::Cancel => false,
        }
    }

    /// What Apply persists: per edge, the managed fields whose draft differs
    /// from the opening snapshot. With `scope_all`, the focused edge's
    /// changes fan out to every edge — but an explicit per-edge change (an
    /// OtherBar switch) wins over the fan-out for the one field both can
    /// speak, because a switch somebody pressed is a decision and a fan-out
    /// is a convenience (TP-CHROME-152).
    pub(crate) fn managed_overrides(&self) -> Vec<(BarEdge, ManagedBarOverride)> {
        let focused = diff_edge(
            edge_config(&self.original, self.edge),
            edge_config(&self.draft, self.edge),
        );
        let mut out = Vec::new();
        for edge in BAR_PANEL_EDGES {
            let mut over = diff_edge(
                edge_config(&self.original, edge),
                edge_config(&self.draft, edge),
            );
            if self.scope_all && edge != self.edge {
                over = merge_fanout(over, &focused);
            }
            if !over.is_empty() {
                out.push((edge, over));
            }
        }
        out
    }
}

pub(crate) fn edge_config(bars: &ShellBarsConfig, edge: BarEdge) -> &ShellBarConfig {
    match edge {
        BarEdge::Top => &bars.top,
        BarEdge::Bottom => &bars.bottom,
        BarEdge::Left => &bars.left,
        BarEdge::Right => &bars.right,
    }
}

pub(crate) fn edge_config_mut(bars: &mut ShellBarsConfig, edge: BarEdge) -> &mut ShellBarConfig {
    match edge {
        BarEdge::Top => &mut bars.top,
        BarEdge::Bottom => &mut bars.bottom,
        BarEdge::Left => &mut bars.left,
        BarEdge::Right => &mut bars.right,
    }
}

/// The style cycle is total over the spec's closed set; an unwritten style
/// means `framed`, so that is where the cycle stands when it starts.
pub(crate) fn cycle_style(current: &str, forward: bool) -> String {
    let normalized = if current.is_empty() {
        "framed"
    } else {
        current
    };
    let idx = BAR_STYLE_CHOICES
        .iter()
        .position(|choice| *choice == normalized)
        .unwrap_or(0);
    let len = BAR_STYLE_CHOICES.len();
    let next = if forward {
        (idx + 1) % len
    } else {
        (idx + len - 1) % len
    };
    BAR_STYLE_CHOICES[next].to_string()
}

/// auto → on → off → auto, in both directions — the three states the config
/// key can hold.
pub(crate) fn cycle_border(current: Option<bool>, forward: bool) -> Option<bool> {
    match (current, forward) {
        (None, true) => Some(true),
        (Some(true), true) => Some(false),
        (Some(false), true) => None,
        (None, false) => Some(false),
        (Some(false), false) => Some(true),
        (Some(true), false) => None,
    }
}

/// One step along the spec's 1-32 range, held at the ends rather than
/// wrapped — a size that jumped from 32 to 1 would collapse the bar the
/// person was growing.
pub(crate) fn step_size(current: u16, forward: bool) -> u16 {
    if forward {
        current.saturating_add(1).min(32)
    } else {
        current.saturating_sub(1).max(1)
    }
}

/// The colour row's choices: the default tone first, then — when the file
/// holds a literal the token table does not know — that literal, so the
/// person's own colour stays reachable after cycling away, then every token
/// this build resolves.
pub(crate) fn color_choices(original: &str) -> Vec<String> {
    let mut choices = vec![String::new()];
    push_custom_and_tokens(&mut choices, original);
    choices
}

/// The backdrop row adds `reset` — the one word with no token: the
/// terminal's own background showing through.
pub(crate) fn background_choices(original: &str) -> Vec<String> {
    let mut choices = vec![String::new(), "reset".to_string()];
    push_custom_and_tokens(&mut choices, original);
    choices
}

fn push_custom_and_tokens(choices: &mut Vec<String>, original: &str) {
    if !original.is_empty() && !choices.iter().any(|c| c == original) {
        let known = crate::ui::shell::bar_color_tokens().contains(&original);
        if !known {
            choices.push(original.to_string());
        }
    }
    choices.extend(
        crate::ui::shell::bar_color_tokens()
            .iter()
            .map(|token| (*token).to_string()),
    );
}

pub(crate) fn cycle_choice(current: &str, choices: &[String], forward: bool) -> String {
    if choices.is_empty() {
        return current.to_string();
    }
    let idx = choices.iter().position(|choice| choice == current);
    let len = choices.len();
    let next = match (idx, forward) {
        (Some(i), true) => (i + 1) % len,
        (Some(i), false) => (i + len - 1) % len,
        // A value the list does not carry starts the cycle at its head.
        (None, _) => 0,
    };
    choices[next].clone()
}

/// The word the managed file writes for a border state — TOML cannot write
/// "explicitly none", so `auto` carries it (TP-CHROME-149).
pub(crate) const fn border_word(border: Option<bool>) -> &'static str {
    match border {
        None => "auto",
        Some(true) => "on",
        Some(false) => "off",
    }
}

/// The managed fields whose draft differs from the snapshot — exactly what
/// the overlay may carry, nothing else.
fn diff_edge(original: &ShellBarConfig, draft: &ShellBarConfig) -> ManagedBarOverride {
    ManagedBarOverride {
        enabled: (original.enabled != draft.enabled).then_some(draft.enabled),
        size: (original.size != draft.size).then_some(draft.size),
        style: (original.style != draft.style).then(|| draft.style.clone()),
        border: (original.border != draft.border).then(|| border_word(draft.border).to_string()),
        color: (original.color != draft.color).then(|| draft.color.clone()),
        background: (original.background != draft.background).then(|| draft.background.clone()),
        // The Configure face edits no section actions yet; the diff carries
        // none so an Apply can never erase a hand-written override.
        section_overrides: Vec::new(),
    }
}

/// Fan the focused edge's changes onto another edge's own diff — the
/// explicit diff wins field by field.
fn merge_fanout(explicit: ManagedBarOverride, focused: &ManagedBarOverride) -> ManagedBarOverride {
    ManagedBarOverride {
        enabled: explicit.enabled.or(focused.enabled),
        size: explicit.size.or(focused.size),
        style: explicit.style.or_else(|| focused.style.clone()),
        border: explicit.border.or_else(|| focused.border.clone()),
        color: explicit.color.or_else(|| focused.color.clone()),
        background: explicit.background.or_else(|| focused.background.clone()),
        // Section rebindings never fan out: an id names ONE section on ONE
        // edge, and copying it to edges that lack the id would only produce
        // four diagnostics for one intent.
        section_overrides: explicit.section_overrides,
    }
}

/// Which face of the panel is forward (TP-CHROME-153): `Apps` is the
/// inventory — what each configured section SHOWS and which app a press
/// reaches — `Configure` is the knobs. Configure opens first: it is the
/// panel S30-2 shipped and the muscle memory the tabs must not steal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BarPanelTab {
    Apps,
    Configure,
}

impl BarPanelTab {
    pub(crate) const ALL: [Self; 2] = [Self::Apps, Self::Configure];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Apps => "Apps",
            Self::Configure => "Configure",
        }
    }
}

/// One inventory row of the Apps tab. `section_indices` are TRUE indices
/// into the bar's `sections` — a grouped run folds into one row and keeps
/// every member's address, so a press can still name a real section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BarAppRow {
    pub section_indices: Vec<usize>,
    pub shows: String,
    pub does: String,
    /// Whether Enter/click has a primary action to run — a dead-end row is
    /// offered greyed, never silent.
    pub live: bool,
    /// The id the edit form would bind an override to: the action-carrying
    /// section's, or the first identified one when no action is bound yet.
    /// `None` means the row cannot be edited from here — the overlay binds
    /// by id and none of the row's sections carries one (TP-CHROME-166).
    pub section_id: Option<String>,
}

/// One form row's text, in the same label-then-value shape the Configure
/// rows wear. Pure, so the render only draws (TP-CHROME-166).
pub(crate) fn edit_row_label(form: &BarAppEditForm, field: BarAppEditField) -> String {
    let value_name = match EDIT_KIND_CHOICES.get(form.kind_choice).copied() {
        Some("workspace") => "workspace",
        Some("plugin") => "action id",
        Some("none") => "(disarmed)",
        _ => "command",
    };
    let text_or_dash = |text: &str| {
        if text.is_empty() {
            "\u{2014}".to_string()
        } else {
            text.to_string()
        }
    };
    match field {
        BarAppEditField::Kind => format!(
            "kind       \u{2039} {} \u{203a}",
            EDIT_KIND_CHOICES
                .get(form.kind_choice)
                .copied()
                .unwrap_or("?")
        ),
        BarAppEditField::Value => {
            format!("{value_name:<10} {}", text_or_dash(&form.value_text))
        }
        BarAppEditField::Width => format!("width      {}", text_or_dash(&form.width_text)),
        BarAppEditField::Height => format!("height     {}", text_or_dash(&form.height_text)),
        BarAppEditField::Save => "[ Save ]".to_string(),
        BarAppEditField::Cancel => "[ Cancel ]".to_string(),
    }
}

/// What one section's widget puts on the strip, in the config's own words.
pub(crate) fn widget_summary(section: &crate::config::ShellBarSectionConfig) -> String {
    let widget = &section.widget;
    match widget.kind.as_str() {
        "" => match section.kind.as_str() {
            "" => "bare".to_string(),
            kind => kind.to_string(),
        },
        "label" => {
            if widget.text.is_empty() {
                "label".to_string()
            } else {
                format!("label \"{}\"", widget.text)
            }
        }
        "resource" | "meter" | "sparkline" => {
            if widget.metric.is_empty() {
                widget.kind.clone()
            } else {
                format!("{} {}", widget.kind, widget.metric)
            }
        }
        "icon" => {
            if !widget.glyph.is_empty() {
                format!("icon {}", widget.glyph)
            } else if !widget.art.is_empty() {
                format!("icon {}", widget.art)
            } else if !widget.pixels.is_empty() {
                "icon custom".to_string()
            } else {
                "icon".to_string()
            }
        }
        "clock" => {
            let format = if widget.format.is_empty() {
                "%H:%M"
            } else {
                &widget.format
            };
            format!("clock {format}")
        }
        other => other.to_string(),
    }
}

/// Which app (or road) a primary press on the section reaches — `None` for
/// an indicator that consumes clicks inertly.
pub(crate) fn action_summary(
    action: &crate::config::ShellBarSectionActionConfig,
) -> Option<String> {
    let program = || {
        action
            .argv
            .first()
            .map(|argv0| argv0.rsplit('/').next().unwrap_or(argv0).to_string())
            .unwrap_or_default()
    };
    match action.kind.as_str() {
        "" | "none" => None,
        "popup" => Some(format!("{} [popup]", program()).trim_start().to_string()),
        "run" => Some(format!("{} [run]", program()).trim_start().to_string()),
        "workspace" => Some(format!("workspace {}", action.name)),
        "plugin" => Some(format!("plugin {}", action.command)),
        "hide" => Some("hide bar".to_string()),
        other => Some(other.to_string()),
    }
}

/// The Apps tab's rows for one bar: one row per section, except that a
/// grouped run — adjacent sections naming one `group` — folds into ONE row,
/// the same folding the frame machinery draws (one rectangle, one row;
/// TP-CHROME-144's vocabulary, reused rather than re-invented). An
/// undivided strip is one honest row, never an empty list.
pub(crate) fn section_app_rows(bar: &ShellBarConfig) -> Vec<BarAppRow> {
    if bar.sections.is_empty() {
        return vec![BarAppRow {
            section_indices: Vec::new(),
            shows: "undivided strip".to_string(),
            does: "\u{2014}".to_string(),
            live: false,
            section_id: None,
        }];
    }
    let mut rows: Vec<BarAppRow> = Vec::new();
    let mut run: Vec<usize> = Vec::new();
    let mut run_group = String::new();
    let sections = &bar.sections;
    let flush = |run: &mut Vec<usize>, rows: &mut Vec<BarAppRow>| {
        if run.is_empty() {
            return;
        }
        let shows = run
            .iter()
            .map(|&idx| widget_summary(&sections[idx]))
            .collect::<Vec<_>>()
            .join(" \u{b7} ");
        let mut does_parts: Vec<String> = Vec::new();
        for &idx in run.iter() {
            if let Some(does) = action_summary(&sections[idx].action) {
                if !does_parts.contains(&does) {
                    does_parts.push(does);
                }
            }
        }
        let live = !does_parts.is_empty();
        let does = if live {
            format!("\u{2192} {}", does_parts.join(" \u{b7} "))
        } else {
            "\u{2014}".to_string()
        };
        // The id the edit form binds to follows the same choice the run road
        // makes: the first action-carrying section speaks for the row. A row
        // with no action yet falls back to its first identified section, so
        // a dead-end row can still be given a press.
        let section_id = run
            .iter()
            .find(|&&idx| action_summary(&sections[idx].action).is_some())
            .or_else(|| run.iter().find(|&&idx| !sections[idx].id.is_empty()))
            .map(|&idx| sections[idx].id.clone())
            .filter(|id| !id.is_empty());
        rows.push(BarAppRow {
            section_indices: std::mem::take(run),
            shows,
            does,
            live,
            section_id,
        });
    };
    for (idx, section) in sections.iter().enumerate() {
        let grouped_with_previous =
            !section.group.is_empty() && section.group == run_group && !run.is_empty();
        if !grouped_with_previous {
            flush(&mut run, &mut rows);
            run_group = section.group.clone();
        }
        run.push(idx);
        if section.group.is_empty() {
            flush(&mut run, &mut rows);
            run_group.clear();
        }
    }
    flush(&mut run, &mut rows);
    rows
}

/// What a row shows for its current draft value — one string, so the render
/// pass and its tests read the same words (TP-CHROME-150).
/// TP-CHROME-160: a small closed set is ENUMERATED, not merely echoed.
/// "Style: pills" told the user nothing about islands existing — the exact
/// discoverability gap they reported. The selected member wears brackets.
fn enumerate_choice<'a, I>(choices: I, selected: &str) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    choices
        .into_iter()
        .map(|choice| {
            if choice == selected {
                format!("[{choice}]")
            } else {
                choice.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" \u{b7} ")
}

/// TP-CHROME-161: each face teaches its own real keys — the Apps face has no
/// ←→ adjustment, so its hint must not claim one.
pub(crate) const fn panel_hint(tab: BarPanelTab) -> &'static str {
    match tab {
        BarPanelTab::Configure => {
            "\u{2190}\u{2192} change \u{b7} Enter select \u{b7} Tab apps \u{b7} Esc close"
        }
        BarPanelTab::Apps => "Enter run \u{b7} e edit \u{b7} Tab configure \u{b7} Esc close",
    }
}

pub(crate) fn row_value_label(state: &BarConfigPanelState, row: BarPanelRow) -> String {
    let bar = edge_config(&state.draft, state.edge);
    match row {
        BarPanelRow::Enabled => format!("Enabled: {}", if bar.enabled { "on" } else { "off" }),
        BarPanelRow::Style => {
            let current = if bar.style.is_empty() {
                "framed"
            } else {
                &bar.style
            };
            format!(
                "Style: {}",
                enumerate_choice(BAR_STYLE_CHOICES.iter().copied(), current)
            )
        }
        BarPanelRow::Border => format!(
            "Border: {}",
            enumerate_choice(["auto", "on", "off"], border_word(bar.border))
        ),
        BarPanelRow::Size => format!("Size: \u{25c2} {} \u{25b8}", bar.size),
        BarPanelRow::Color => format!(
            "Colour: \u{25c2} {} \u{25b8}",
            if bar.color.is_empty() {
                "(default)"
            } else {
                &bar.color
            }
        ),
        BarPanelRow::Background => format!(
            "Backdrop: \u{25c2} {} \u{25b8}",
            if bar.background.is_empty() {
                "(theme)"
            } else {
                &bar.background
            }
        ),
        BarPanelRow::Scope => format!(
            "Apply to: {}",
            enumerate_choice(
                ["this bar", "all bars"],
                if state.scope_all {
                    "all bars"
                } else {
                    "this bar"
                }
            )
        ),
        BarPanelRow::OtherBar(edge) => format!(
            "{} bar: {}",
            bar_edge_name(edge),
            if edge_config(&state.draft, edge).enabled {
                "on"
            } else {
                "off"
            }
        ),
        BarPanelRow::Apply => "[ Apply ]".to_string(),
        BarPanelRow::Cancel => "[ Cancel ]".to_string(),
    }
}

impl crate::app::state::AppState {
    /// Centered popup rect over the terminal area — the colleague picker's
    /// geometry, because the two are one kind of surface.
    pub(crate) fn bar_config_panel_popup_rect(&self) -> Option<ratatui::layout::Rect> {
        let panel = self.bar_config_panel.as_ref()?;
        let area = self.view.terminal_area;
        // TP-CHROME-160/161: one width for both faces — the enumerated rows
        // and the hint line need the room, and the popup no longer jumps when
        // Tab switches faces.
        let width = 56u16.min(area.width.saturating_sub(2)).max(4);
        let height = (panel.forward_row_count() as u16)
            .saturating_add(5)
            .min(area.height.saturating_sub(2))
            .max(4);
        if area.width < 8 || area.height < 6 {
            return None;
        }
        let x = area.x + (area.width.saturating_sub(width)) / 2;
        let y = area.y + (area.height.saturating_sub(height)) / 2;
        Some(ratatui::layout::Rect::new(x, y, width, height))
    }

    pub(crate) fn bar_config_panel_row_hit_areas(&self) -> Vec<ratatui::layout::Rect> {
        let Some(panel) = self.bar_config_panel.as_ref() else {
            return Vec::new();
        };
        let Some(popup) = self.bar_config_panel_popup_rect() else {
            return Vec::new();
        };
        // The last inner line belongs to the hint (TP-CHROME-161) — rows must
        // neither draw over it nor make it clickable.
        let inner = ratatui::layout::Rect::new(
            popup.x + 1,
            popup.y + 3,
            popup.width.saturating_sub(2),
            popup.height.saturating_sub(5),
        );
        (0..panel.forward_row_count())
            .take(inner.height as usize)
            .map(|idx| ratatui::layout::Rect::new(inner.x, inner.y + idx as u16, inner.width, 1))
            .collect()
    }

    /// The two tab labels' rects on the strip line (popup.y + 2), in
    /// `BarPanelTab::ALL` order.
    pub(crate) fn bar_config_panel_tab_hit_areas(&self) -> Vec<ratatui::layout::Rect> {
        let Some(popup) = self.bar_config_panel_popup_rect() else {
            return Vec::new();
        };
        let mut x = popup.x + 1;
        let y = popup.y + 2;
        BarPanelTab::ALL
            .iter()
            .map(|tab| {
                let width = tab.label().len() as u16 + 2;
                let rect = ratatui::layout::Rect::new(x, y, width, 1);
                x = x.saturating_add(width.saturating_add(1));
                rect
            })
            .collect()
    }

    pub(crate) fn bar_config_panel_tab_at(&self, column: u16, row: u16) -> Option<BarPanelTab> {
        self.bar_config_panel_tab_hit_areas()
            .iter()
            .position(|rect| {
                column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
            })
            .map(|idx| BarPanelTab::ALL[idx])
    }

    pub(crate) fn bar_config_panel_row_at(&self, column: u16, row: u16) -> Option<usize> {
        self.bar_config_panel_row_hit_areas()
            .iter()
            .position(|rect| {
                column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
            })
    }

    /// Drop the panel and restore the pre-overlay focus owner. State-level
    /// only — the preview restore lives on the App road, which is the only
    /// road that can repaint.
    pub(crate) fn close_bar_config_panel(&mut self) {
        if self.bar_config_panel.take().is_some() {
            crate::app::input::leave_modal(self);
        }
    }
}

impl crate::app::App {
    /// Rebuild everything the bars show from one set of bar configs — the
    /// same three-piece refresh a config reload performs, extracted so the
    /// panel's live preview and the reload can never drift apart
    /// (TP-CHROME-151).
    pub(crate) fn refresh_bar_presentation(&mut self, bars: &ShellBarsConfig) {
        self.state
            .shell_presentation
            .set_bars(crate::ui::shell::ShellBars::from_config(bars));
        self.state
            .shell_presentation
            .set_bar_colors(crate::ui::shell::BarColors::from_config(
                bars,
                &self.state.palette,
            ));
        self.state.shell_bar_chrome = crate::ui::shell::ShellBarChrome::from_config(
            bars,
            self.state.shell_glyph_icons,
            &self.state.palette,
        );
    }

    /// Open the panel for `edge`, seeded from the bars the last config load
    /// left behind.
    pub(crate) fn open_bar_config_panel(&mut self, edge: BarEdge) {
        self.state.bar_config_panel = Some(BarConfigPanelState::open(
            edge,
            &self.state.shell_bars_config.clone(),
        ));
        self.state
            .enter_overlay_mode(crate::app::state::Mode::BarConfigPanel);
    }

    /// Throw the draft away: repaint from the untouched snapshot and close.
    pub(crate) fn cancel_bar_config_panel(&mut self) {
        let bars = self.state.shell_bars_config.clone();
        self.refresh_bar_presentation(&bars);
        self.state.close_bar_config_panel();
    }

    /// Adjust the selected row and repaint the preview when it changed.
    pub(crate) fn adjust_bar_config_panel(&mut self, forward: bool) {
        let Some(panel) = self.state.bar_config_panel.as_mut() else {
            return;
        };
        if panel.adjust_selected(forward) {
            let draft = panel.draft.clone();
            self.refresh_bar_presentation(&draft);
        }
    }

    /// Enter / click on the selected row. On the Configure face the verbs
    /// act and everything else adjusts forward; on the Apps face the row's
    /// own app is reached (TP-CHROME-153).
    pub(crate) fn press_bar_config_panel_row(&mut self) {
        let Some(panel) = self.state.bar_config_panel.as_ref() else {
            return;
        };
        // While the editor is up its fields are the rows: a press on a text
        // field moves the cursor there, the two verbs act (TP-CHROME-166).
        if panel.edit.is_some() {
            let Some(&field) = BarAppEditField::ALL.get(panel.selected) else {
                return;
            };
            match field {
                BarAppEditField::Save => self.save_bar_app_edit(),
                BarAppEditField::Cancel => {
                    if let Some(panel) = self.state.bar_config_panel.as_mut() {
                        panel.edit = None;
                    }
                }
                other => {
                    if let Some(form) = self
                        .state
                        .bar_config_panel
                        .as_mut()
                        .and_then(|panel| panel.edit.as_mut())
                    {
                        form.field = other;
                    }
                }
            }
            return;
        }
        match panel.tab {
            BarPanelTab::Apps => self.run_bar_app_row(panel.selected),
            BarPanelTab::Configure => match panel.rows().get(panel.selected) {
                Some(BarPanelRow::Apply) => self.apply_bar_config_panel(),
                Some(BarPanelRow::Cancel) => self.cancel_bar_config_panel(),
                Some(_) => self.adjust_bar_config_panel(true),
                None => {}
            },
        }
    }

    /// Reach the app an Apps row points at — the popup/run/workspace kinds,
    /// through the SAME App roads the bar's own click takes, so there is one
    /// owner of "what running a section means". A plugin or hide row is
    /// honest about its limit rather than growing a second resolution
    /// machine: the bar itself is where those fire (TP-CHROME-153).
    pub(crate) fn run_bar_app_row(&mut self, row_idx: usize) {
        let Some(panel) = self.state.bar_config_panel.as_ref() else {
            return;
        };
        let rows = panel.app_rows();
        let Some(row) = rows.get(row_idx) else {
            return;
        };
        // No separate liveness guard: the action lookup below IS the guard —
        // a dead-end row has no summarizable action to find, and the mutant
        // that removed an explicit check here changed nothing (T5, quadruple:
        // redundant). `live` stays a render fact: the grey the eye reads.
        let bar = edge_config(&panel.original, panel.edge).clone();
        let action = row
            .section_indices
            .iter()
            .filter_map(|&idx| bar.sections.get(idx))
            .map(|section| section.action.clone())
            .find(|action| action_summary(action).is_some());
        let Some(action) = action else {
            return;
        };
        match action.kind.as_str() {
            "popup" => {
                if let Err(err) = self.spawn_popup_argv_command(
                    &action.argv,
                    None,
                    Vec::new(),
                    crate::app::popup::PopupGeometry {
                        width: action.width,
                        height: action.height,
                    },
                ) {
                    self.warn_about_bar_section_action(
                        "bar section action failed",
                        err.to_string(),
                    );
                }
            }
            "run" => {
                if let Err(err) = self.run_bar_section_command(&action.argv) {
                    self.warn_about_bar_section_action(
                        "bar section action failed",
                        err.to_string(),
                    );
                }
            }
            "workspace" => match self.state.workspace_index_named(&action.name) {
                Some(ws_idx) => self.focus_workspace_idx_via_api(ws_idx),
                None => self.warn_about_bar_section_action(
                    "no workspace by that name",
                    format!("nothing open is called {:?}", action.name),
                ),
            },
            other => {
                self.warn_about_bar_section_action(
                    "open it from the bar",
                    format!("a {other:?} action fires from its own section"),
                );
            }
        }
    }

    /// Persist the diff and converge: write `bars.managed.toml`, then take
    /// the same reload road `herdr server reload-config` takes, so the disk
    /// — not the preview — is what every surface ends up showing
    /// (TP-CHROME-151/152).
    pub(crate) fn apply_bar_config_panel(&mut self) {
        let Some(panel) = self.state.bar_config_panel.as_ref() else {
            return;
        };
        let overrides = panel.managed_overrides();
        if overrides.is_empty() {
            self.cancel_bar_config_panel();
            return;
        }
        let named: Vec<(&str, crate::config::ManagedBarOverride)> = overrides
            .into_iter()
            .map(|(edge, over)| (bar_edge_name(edge), over))
            .collect();
        match crate::config::persist_managed_bar_overrides(&named) {
            Ok(()) => {
                self.state.close_bar_config_panel();
                self.dispatch_api_request(
                    "tui.bars.configure",
                    crate::api::schema::Method::ServerReloadConfig(
                        crate::api::schema::EmptyParams::default(),
                    ),
                );
            }
            Err(err) => {
                // The draft survives a failed write — closing here would
                // throw away edits the person can still retry or cancel.
                self.warn_about_bar_section_action("could not save bar config", err);
            }
        }
    }

    pub(crate) fn handle_bar_config_panel_key(&mut self, key: crossterm::event::KeyEvent) {
        // The open editor answers first: while it is up, every key is the
        // form's — a stray 'e' must land in the text, not reopen the form.
        if self
            .state
            .bar_config_panel
            .as_ref()
            .is_some_and(|panel| panel.edit.is_some())
        {
            self.handle_bar_app_edit_key(key);
            return;
        }
        if key.code == crossterm::event::KeyCode::Char('e')
            && self
                .state
                .bar_config_panel
                .as_ref()
                .is_some_and(|panel| panel.tab == BarPanelTab::Apps)
        {
            self.open_bar_app_edit();
            return;
        }
        match key.code {
            crossterm::event::KeyCode::Esc => self.cancel_bar_config_panel(),
            crossterm::event::KeyCode::Enter => self.press_bar_config_panel_row(),
            crossterm::event::KeyCode::Tab => {
                if let Some(panel) = self.state.bar_config_panel.as_mut() {
                    let next = match panel.tab {
                        BarPanelTab::Apps => BarPanelTab::Configure,
                        BarPanelTab::Configure => BarPanelTab::Apps,
                    };
                    panel.switch_tab(next);
                }
            }
            crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Char('h') => {
                if self
                    .state
                    .bar_config_panel
                    .as_ref()
                    .is_some_and(|panel| panel.tab == BarPanelTab::Configure)
                {
                    self.adjust_bar_config_panel(false);
                }
            }
            crossterm::event::KeyCode::Right | crossterm::event::KeyCode::Char('l') => {
                if self
                    .state
                    .bar_config_panel
                    .as_ref()
                    .is_some_and(|panel| panel.tab == BarPanelTab::Configure)
                {
                    self.adjust_bar_config_panel(true);
                }
            }
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') => {
                if let Some(panel) = self.state.bar_config_panel.as_mut() {
                    panel.selected = panel.selected.saturating_sub(1);
                }
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') => {
                if let Some(panel) = self.state.bar_config_panel.as_mut() {
                    if panel.selected.saturating_add(1) < panel.forward_row_count() {
                        panel.selected += 1;
                    }
                }
            }
            _ => {}
        }
    }

    /// Open the Apps row editor, or say why it cannot open — the overlay
    /// binds by id, and a row without one has nothing to bind to.
    pub(crate) fn open_bar_app_edit(&mut self) {
        let Some(panel) = self.state.bar_config_panel.as_mut() else {
            return;
        };
        if let Err(hint) = panel.open_edit_form() {
            self.warn_about_bar_section_action("this row cannot be edited here", hint);
        }
    }

    /// Keys while the Apps row editor is open. The named keys come first
    /// and everything else that carries a character is typing — so 'e'
    /// lands in the text rather than reopening the form, and 'j' spells
    /// rather than walks.
    pub(crate) fn handle_bar_app_edit_key(&mut self, key: crossterm::event::KeyEvent) {
        if key.code == crossterm::event::KeyCode::Enter {
            let on_cancel = self
                .state
                .bar_config_panel
                .as_ref()
                .and_then(|panel| panel.edit.as_ref())
                .is_some_and(|form| form.field == BarAppEditField::Cancel);
            if on_cancel {
                if let Some(panel) = self.state.bar_config_panel.as_mut() {
                    panel.edit = None;
                }
            } else {
                self.save_bar_app_edit();
            }
            return;
        }
        let Some(panel) = self.state.bar_config_panel.as_mut() else {
            return;
        };
        match key.code {
            crossterm::event::KeyCode::Esc => {
                panel.edit = None;
            }
            crossterm::event::KeyCode::Tab | crossterm::event::KeyCode::Down => {
                panel.edit_field_step(true);
            }
            crossterm::event::KeyCode::Up => {
                panel.edit_field_step(false);
            }
            crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Right => {
                if panel
                    .edit
                    .as_ref()
                    .is_some_and(|form| form.field == BarAppEditField::Kind)
                {
                    panel.edit_cycle_kind(key.code == crossterm::event::KeyCode::Right);
                }
            }
            crossterm::event::KeyCode::Backspace => {
                panel.edit_delete_char();
            }
            crossterm::event::KeyCode::Char(ch) => {
                panel.edit_insert_text(&ch.to_string());
            }
            _ => {}
        }
    }

    /// Save the editor: read the overlay's current overrides, merge the
    /// form over them, write the whole list back, and take the same reload
    /// road Apply takes — which closes the panel, because a reload moves
    /// the world the panel's snapshot describes (TP-CHROME-151). The disk,
    /// not the form, is what every surface ends up showing (TP-CHROME-166).
    pub(crate) fn save_bar_app_edit(&mut self) {
        let Some(panel) = self.state.bar_config_panel.as_mut() else {
            return;
        };
        let edge = bar_edge_name(panel.edge);
        let existing = crate::config::read_managed_section_overrides(edge);
        let list = match panel.edit_save_payload(existing) {
            Ok(list) => list,
            Err(err) => {
                if let Some(form) = panel.edit.as_mut() {
                    form.validation_error = Some(err);
                }
                return;
            }
        };
        let over = crate::config::ManagedBarOverride {
            section_overrides: list,
            ..Default::default()
        };
        match crate::config::persist_managed_bar_overrides(&[(edge, over)]) {
            Ok(()) => {
                self.state.close_bar_config_panel();
                self.dispatch_api_request(
                    "tui.bars.section-edit",
                    crate::api::schema::Method::ServerReloadConfig(
                        crate::api::schema::EmptyParams::default(),
                    ),
                );
            }
            Err(err) => {
                // The form survives a failed write — closing here would
                // throw away edits the person can still fix or cancel.
                self.warn_about_bar_section_action("could not save the press", err);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::Mode;
    use crate::workspace::Workspace;

    fn bars() -> ShellBarsConfig {
        ShellBarsConfig::default()
    }

    // TP-CHROME-150: the panel lists every knob the strip answers to, the
    // scope choice, the OTHER edges' switches, and the two verbs — in that
    // order, so muscle memory survives which bar was pressed.
    #[test]
    fn the_panel_rows_list_every_knob_and_exclude_the_focused_edge() {
        let rows = panel_rows(BarEdge::Bottom);
        assert_eq!(
            rows,
            vec![
                BarPanelRow::Enabled,
                BarPanelRow::Style,
                BarPanelRow::Border,
                BarPanelRow::Size,
                BarPanelRow::Color,
                BarPanelRow::Background,
                BarPanelRow::Scope,
                BarPanelRow::OtherBar(BarEdge::Top),
                BarPanelRow::OtherBar(BarEdge::Left),
                BarPanelRow::OtherBar(BarEdge::Right),
                BarPanelRow::Apply,
                BarPanelRow::Cancel,
            ]
        );
    }

    // TP-CHROME-160: a small closed set is enumerated with the selection
    // bracketed — the user's exact report was "styles'ta sadece pills var,
    // islands eksik": every member must be VISIBLE in the row itself.
    #[test]
    fn the_style_row_enumerates_every_choice_and_brackets_the_selection() {
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars());
        edge_config_mut(&mut state.draft, BarEdge::Top).style = "pills".to_string();
        let label = row_value_label(&state, BarPanelRow::Style);
        assert_eq!(
            label,
            "Style: framed \u{b7} islands \u{b7} plain \u{b7} [pills]"
        );
        for choice in BAR_STYLE_CHOICES {
            assert!(
                label.contains(choice),
                "{choice} must be discoverable: {label}"
            );
        }

        // The selection marker follows the value, not a fixed slot.
        edge_config_mut(&mut state.draft, BarEdge::Top).style = "islands".to_string();
        let label = row_value_label(&state, BarPanelRow::Style);
        assert!(
            label.contains("[islands]") && !label.contains("[pills]"),
            "{label}"
        );

        // An empty style IS framed (the cycle's own normalisation).
        edge_config_mut(&mut state.draft, BarEdge::Top).style = String::new();
        let label = row_value_label(&state, BarPanelRow::Style);
        assert!(label.contains("[framed]"), "{label}");
    }

    // TP-CHROME-160: the same treatment for the other small closed sets.
    #[test]
    fn border_and_scope_rows_enumerate_their_choices() {
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars());
        edge_config_mut(&mut state.draft, BarEdge::Top).border = Some(true);
        assert_eq!(
            row_value_label(&state, BarPanelRow::Border),
            "Border: auto \u{b7} [on] \u{b7} off"
        );
        assert_eq!(
            row_value_label(&state, BarPanelRow::Scope),
            "Apply to: [this bar] \u{b7} all bars"
        );
        state.scope_all = true;
        assert_eq!(
            row_value_label(&state, BarPanelRow::Scope),
            "Apply to: this bar \u{b7} [all bars]"
        );
    }

    // TP-CHROME-160: long/open sets wear stepper arrows instead — the
    // affordance is "there are neighbours", not the whole list.
    #[test]
    fn long_value_rows_wear_stepper_arrows() {
        let state = BarConfigPanelState::open(BarEdge::Top, &bars());
        for row in [
            BarPanelRow::Size,
            BarPanelRow::Color,
            BarPanelRow::Background,
        ] {
            let label = row_value_label(&state, row);
            assert!(
                label.contains('\u{25c2}') && label.contains('\u{25b8}'),
                "steppable row shows its arrows: {label}"
            );
        }
    }

    // TP-CHROME-161: each face teaches its own real keys — Apps has no
    // \u{2190}\u{2192} adjustment, so its hint must not claim one.
    #[test]
    fn the_hint_line_matches_each_faces_real_keys() {
        let configure = panel_hint(BarPanelTab::Configure);
        assert!(configure.contains("\u{2190}\u{2192}") && configure.contains("Tab apps"));
        let apps = panel_hint(BarPanelTab::Apps);
        assert!(
            !apps.contains("\u{2190}"),
            "Apps face has no left/right verb: {apps}"
        );
        assert!(apps.contains("Tab configure"));
    }

    // TP-CHROME-150: every cycle is total — nothing the person can reach
    // steps outside the closed set the spec promises.
    #[test]
    fn style_border_and_size_cycles_are_total_and_bounded() {
        let mut style = String::new();
        for _ in 0..4 {
            style = cycle_style(&style, true);
            assert!(BAR_STYLE_CHOICES.contains(&style.as_str()));
        }
        assert_eq!(style, "framed", "four forward steps close the loop");
        assert_eq!(cycle_style("framed", false), "pills", "backwards wraps");

        assert_eq!(cycle_border(None, true), Some(true));
        assert_eq!(cycle_border(Some(true), true), Some(false));
        assert_eq!(cycle_border(Some(false), true), None);
        assert_eq!(cycle_border(None, false), Some(false));

        assert_eq!(step_size(32, true), 32, "held at the top of the range");
        assert_eq!(step_size(1, false), 1, "held at the bottom");
        assert_eq!(step_size(3, true), 4);
    }

    // TP-CHROME-150: a literal the token table does not know stays reachable
    // — cycling away from the person's own colour must not eat it.
    #[test]
    fn colour_choices_keep_a_custom_literal_reachable() {
        let choices = color_choices("#cba6f7");
        assert_eq!(choices[0], "", "the default tone leads");
        assert!(choices.iter().any(|c| c == "#cba6f7"));
        assert!(choices.iter().any(|c| c == "mauve"));

        let token_only = color_choices("mauve");
        assert_eq!(
            token_only.iter().filter(|c| *c == "mauve").count(),
            1,
            "a known token is not doubled"
        );

        let backdrop = background_choices("");
        assert_eq!(backdrop[0], "");
        assert_eq!(backdrop[1], "reset", "the backdrop speaks reset too");
    }

    // TP-CHROME-151: an untouched panel writes nothing — opening and closing
    // must leave no trace on disk.
    #[test]
    fn an_untouched_panel_writes_nothing() {
        let state = BarConfigPanelState::open(BarEdge::Top, &bars());
        assert!(state.managed_overrides().is_empty());
    }

    // TP-CHROME-151: a change writes only itself, on its own edge — the diff
    // is the persistence contract, so an unchanged field keeps following the
    // user's own file.
    #[test]
    fn a_changed_field_writes_only_itself_on_its_edge() {
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars());
        state.adjust_row(BarPanelRow::Style, true);
        let overrides = state.managed_overrides();
        assert_eq!(overrides.len(), 1);
        let (edge, over) = &overrides[0];
        assert_eq!(*edge, BarEdge::Top);
        assert_eq!(over.style.as_deref(), Some("islands"));
        assert_eq!(over.color, None, "an untouched field is not written");
        assert_eq!(over.enabled, None);
    }

    // TP-CHROME-152: "all bars" fans the focused edge's changes to every
    // edge in one Apply.
    #[test]
    fn scope_all_fans_the_focused_changes_to_every_edge() {
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars());
        state.adjust_row(BarPanelRow::Style, true);
        state.adjust_row(BarPanelRow::Scope, true);
        let overrides = state.managed_overrides();
        assert_eq!(overrides.len(), 4);
        for (_, over) in &overrides {
            assert_eq!(over.style.as_deref(), Some("islands"));
        }
    }

    // TP-CHROME-152: a switch somebody pressed is a decision; the fan-out is
    // a convenience — the explicit change wins the one field both can speak.
    #[test]
    fn an_explicit_other_bar_switch_wins_over_the_fanout() {
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars());
        state.adjust_row(BarPanelRow::Enabled, true); // top: off -> on
        state.adjust_row(BarPanelRow::Scope, true);
        state.adjust_row(BarPanelRow::OtherBar(BarEdge::Bottom), true); // on
        state.adjust_row(BarPanelRow::OtherBar(BarEdge::Bottom), true); // off again
                                                                        // bottom's explicit round-trip left it unchanged, so the fan-out's
                                                                        // enabled=true is what it receives — but flip it once more and the
                                                                        // explicit OFF must survive the fan-out saying ON.
        state.adjust_row(BarPanelRow::OtherBar(BarEdge::Bottom), true); // on
        let overrides = state.managed_overrides();
        let bottom = overrides
            .iter()
            .find(|(edge, _)| *edge == BarEdge::Bottom)
            .map(|(_, over)| over)
            .expect("bottom carries its explicit switch");
        assert_eq!(bottom.enabled, Some(true));

        // and the true conflict: focused ON fanned out, bottom explicitly
        // ends OFF after starting ON in the snapshot.
        let mut enabled_bars = bars();
        enabled_bars.bottom.enabled = true;
        let mut state = BarConfigPanelState::open(BarEdge::Top, &enabled_bars);
        state.adjust_row(BarPanelRow::Enabled, true); // top on
        state.adjust_row(BarPanelRow::Scope, true);
        state.adjust_row(BarPanelRow::OtherBar(BarEdge::Bottom), true); // bottom off
        let overrides = state.managed_overrides();
        let bottom = overrides
            .iter()
            .find(|(edge, _)| *edge == BarEdge::Bottom)
            .map(|(_, over)| over)
            .expect("bottom carries its explicit switch");
        assert_eq!(
            bottom.enabled,
            Some(false),
            "the pressed switch beats the fan-out"
        );
    }

    // TP-CHROME-149/151: the border diff travels as the overlay's word.
    #[test]
    fn a_border_change_travels_as_a_word() {
        let mut explicit = bars();
        explicit.top.border = Some(true);
        let mut state = BarConfigPanelState::open(BarEdge::Top, &explicit);
        state.adjust_row(BarPanelRow::Border, true); // Some(true) -> Some(false)
        let overrides = state.managed_overrides();
        assert_eq!(overrides[0].1.border.as_deref(), Some("off"));
        state.adjust_row(BarPanelRow::Border, true); // -> None
        let overrides = state.managed_overrides();
        assert_eq!(overrides[0].1.border.as_deref(), Some("auto"));
    }

    // TP-CHROME-150: every row edits exactly what it names — a field row the
    // focused edge, an OtherBar row that edge's switch, the verbs nothing.
    #[test]
    fn adjusting_rows_touches_only_what_the_row_names() {
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars());
        assert!(state.adjust_row(BarPanelRow::Color, true));
        assert_eq!(state.draft.top.color, "accent", "first token after default");
        assert_eq!(state.draft.bottom, bars().bottom, "other edges untouched");

        assert!(state.adjust_row(BarPanelRow::OtherBar(BarEdge::Left), true));
        assert!(state.draft.left.enabled);
        assert_eq!(state.draft.top.enabled, bars().top.enabled);

        assert!(
            !state.adjust_row(BarPanelRow::Apply, true),
            "verbs adjust nothing"
        );
        assert!(!state.adjust_row(BarPanelRow::Cancel, true));
    }
    fn section(kind: &str) -> crate::config::ShellBarSectionConfig {
        crate::config::ShellBarSectionConfig {
            kind: kind.to_string(),
            ..Default::default()
        }
    }

    // TP-CHROME-153: the inventory speaks the config's own words — what a
    // section shows and which app a press reaches, per kind, with honest
    // fallbacks for the bare and the undivided.
    #[test]
    fn the_apps_inventory_names_what_shows_and_what_a_press_reaches() {
        let mut bar = crate::config::ShellBarConfig::default();
        assert_eq!(
            section_app_rows(&bar),
            vec![BarAppRow {
                section_indices: Vec::new(),
                shows: "undivided strip".to_string(),
                does: "\u{2014}".to_string(),
                live: false,
                section_id: None,
            }],
            "an undivided strip is one honest row, never an empty list"
        );

        let mut cpu = section("content");
        cpu.widget.kind = "resource".to_string();
        cpu.widget.metric = "cpu".to_string();
        cpu.action.kind = "popup".to_string();
        cpu.action.argv = vec!["/usr/bin/btop".to_string()];
        let mut clock = section("fixed");
        clock.widget.kind = "clock".to_string();
        let fill = section("fill");
        bar.sections = vec![cpu, clock, fill];

        let rows = section_app_rows(&bar);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].shows, "resource cpu");
        assert_eq!(
            rows[0].does, "\u{2192} btop [popup]",
            "argv is shown by basename"
        );
        assert!(rows[0].live);
        assert_eq!(
            rows[1].shows, "clock %H:%M",
            "an unwritten format is the default"
        );
        assert_eq!(rows[1].does, "\u{2014}");
        assert!(
            !rows[1].live,
            "an indicator is offered greyed, never silent"
        );
        assert_eq!(rows[2].shows, "fill");
        assert_eq!(rows[0].section_indices, vec![0]);
    }

    // TP-CHROME-153: a grouped run folds into ONE row — the same folding the
    // frame machinery draws — and keeps every member's true address.
    #[test]
    fn a_grouped_run_folds_into_one_inventory_row() {
        let mut bar = crate::config::ShellBarConfig::default();
        let mut cpu = section("content");
        cpu.widget.kind = "resource".to_string();
        cpu.widget.metric = "cpu".to_string();
        cpu.group = "sys".to_string();
        let mut mem = cpu.clone();
        mem.widget.metric = "mem".to_string();
        let mut swap = cpu.clone();
        swap.widget.metric = "swap".to_string();
        swap.action.kind = "popup".to_string();
        swap.action.argv = vec!["btop".to_string()];
        let mut lone = section("fixed");
        lone.widget.kind = "icon".to_string();
        lone.widget.glyph = "\u{2699}".to_string();
        bar.sections = vec![cpu, mem, swap, lone];

        let rows = section_app_rows(&bar);
        assert_eq!(rows.len(), 2, "three grouped + one lone = two rows");
        assert_eq!(
            rows[0].shows,
            "resource cpu \u{b7} resource mem \u{b7} resource swap"
        );
        assert_eq!(rows[0].does, "\u{2192} btop [popup]");
        assert_eq!(rows[0].section_indices, vec![0, 1, 2]);
        assert!(rows[0].live);
        assert_eq!(rows[1].shows, "icon \u{2699}");
    }

    // TP-CHROME-153: the tabs — Configure opens first, switching brings the
    // other face forward and resets the selection, and the inventory reads
    // the SNAPSHOT, so a knob turned on the Configure tab cannot rewrite it.
    #[test]
    fn switching_tabs_resets_selection_and_the_inventory_reads_the_snapshot() {
        let mut bars = ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars);
        assert_eq!(state.tab, BarPanelTab::Configure, "muscle memory survives");

        state.selected = 3;
        assert!(state.switch_tab(BarPanelTab::Apps));
        assert_eq!(state.selected, 0, "an index carried across tabs is a ghost");
        assert!(!state.switch_tab(BarPanelTab::Apps), "same tab is a no-op");
        assert_eq!(
            state.forward_row_count(),
            1,
            "the undivided strip's one row"
        );

        state.adjust_row(BarPanelRow::Enabled, true); // draft flips on the other face
        assert_eq!(
            state.app_rows(),
            section_app_rows(&bars.top),
            "the inventory is the snapshot, not the draft"
        );
    }

    // TP-CHROME-153: Tab turns the face, arrows stay Configure's knobs, and
    // Enter on a live Apps row reaches its app through the same popup road
    // the bar's own click takes.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_apps_face_runs_its_row_on_the_panels_own_key_road() {
        let mut app = test_app();
        // a bar with one btop popup section, loaded as the snapshot
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            ..Default::default()
        };
        sec.widget.kind = "resource".to_string();
        sec.widget.metric = "cpu".to_string();
        sec.action.kind = "popup".to_string();
        sec.action.argv = vec!["sh".to_string(), "-c".to_string(), "sleep 5".to_string()];
        bars.top.sections = vec![sec];
        app.state.shell_bars_config = bars;

        app.open_bar_config_panel(BarEdge::Top);
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ));
        assert_eq!(
            app.state.bar_config_panel.as_ref().unwrap().tab,
            BarPanelTab::Apps
        );
        // arrows are Configure's knobs — on Apps they must not edit the draft
        let draft_before = app.state.bar_config_panel.as_ref().unwrap().draft.clone();
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Right,
        ));
        assert_eq!(
            app.state.bar_config_panel.as_ref().unwrap().draft,
            draft_before,
            "an arrow on the Apps face turned a knob"
        );
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));
        assert!(
            app.state.popup_pane.is_some(),
            "Enter on the btop row opened the popup"
        );
    }

    // TP-CHROME-153: a dead-end Apps row is honest — Enter runs nothing.
    #[test]
    fn a_dead_end_apps_row_is_inert_on_enter() {
        let mut app = test_app();
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        app.state.shell_bars_config = bars;
        app.open_bar_config_panel(BarEdge::Top);
        if let Some(panel) = app.state.bar_config_panel.as_mut() {
            panel.switch_tab(BarPanelTab::Apps);
        }
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));
        assert!(app.state.popup_pane.is_none());
        assert!(
            app.state.bar_config_panel.is_some(),
            "an inert press neither runs nor closes"
        );
    }

    // TP-CHROME-166: the editor opens seeded from what the row does today —
    // an empty form would trade an edit for a retype, and the person came to
    // change one thing, not to remember four.
    #[test]
    fn pressing_e_on_an_identified_row_opens_a_seeded_edit_form() {
        let mut app = test_app();
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            id: "cpu".to_string(),
            ..Default::default()
        };
        sec.widget.kind = "meter".to_string();
        sec.widget.metric = "cpu".to_string();
        sec.action.kind = "popup".to_string();
        sec.action.argv = vec!["btop".to_string()];
        sec.action.width = Some(crate::popup_size::PopupSize::Percent(97));
        sec.action.height = Some(crate::popup_size::PopupSize::Cells(30));
        bars.top.sections = vec![sec];
        app.state.shell_bars_config = bars;

        app.open_bar_config_panel(BarEdge::Top);
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e'),
        ));
        let panel = app.state.bar_config_panel.as_ref().expect("panel stays");
        let form = panel.edit.as_ref().expect("the editor opened");
        assert_eq!(form.section_id, "cpu");
        assert_eq!(EDIT_KIND_CHOICES[form.kind_choice], "popup");
        assert_eq!(form.value_text, "btop", "argv seeds the value line");
        assert_eq!(form.width_text, "97%", "a percent width echoes as one");
        assert_eq!(form.height_text, "30", "a cells height echoes as a number");
        assert!(form.validation_error.is_none());
    }

    // TP-CHROME-166: the overlay binds by id, so a row without one has
    // nothing to bind to — the refusal says what to add rather than opening
    // a form whose Save could never land.
    #[test]
    fn an_unidentified_row_refuses_the_edit_form_with_the_add_id_hint() {
        let mut app = test_app();
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            ..Default::default()
        };
        sec.widget.kind = "clock".to_string();
        bars.top.sections = vec![sec];
        app.state.shell_bars_config = bars;

        app.open_bar_config_panel(BarEdge::Top);
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e'),
        ));
        let panel = app.state.bar_config_panel.as_ref().expect("panel stays");
        assert!(panel.edit.is_none(), "no id, no form");
        let toast = app.state.toast.as_ref().expect("the refusal is spoken");
        assert!(
            toast.context.contains("add id"),
            "the hint names the fix: {}",
            toast.context
        );
    }

    // TP-CHROME-166: the kind is a closed set that cycles and wraps — free
    // text here would let a typo write a kind nothing fires.
    #[test]
    fn the_edit_form_kind_cycles_the_closed_set_and_wraps() {
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            id: "cpu".to_string(),
            ..Default::default()
        };
        sec.action.kind = "popup".to_string();
        sec.action.argv = vec!["btop".to_string()];
        bars.top.sections = vec![sec];
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars);
        state.switch_tab(BarPanelTab::Apps);
        state.open_edit_form().expect("an identified row opens");
        let start = state.edit.as_ref().expect("form").kind_choice;
        assert_eq!(EDIT_KIND_CHOICES[start], "popup");
        for _ in 0..EDIT_KIND_CHOICES.len() {
            state.edit_cycle_kind(true);
        }
        assert_eq!(
            state.edit.as_ref().expect("form").kind_choice,
            start,
            "a full forward lap wraps home"
        );
        state.edit_cycle_kind(false);
        assert_eq!(
            EDIT_KIND_CHOICES[state.edit.as_ref().expect("form").kind_choice],
            "none",
            "one step back from popup wraps to the far end"
        );
    }

    // TP-CHROME-166: the save payload is the read-modify-write's pure half —
    // the form merges over what the overlay already carries, so editing one
    // override can never eat its neighbours.
    #[test]
    fn the_save_payload_merges_over_the_existing_overrides() {
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            id: "cpu".to_string(),
            ..Default::default()
        };
        sec.action.kind = "popup".to_string();
        sec.action.argv = vec!["btop".to_string()];
        bars.top.sections = vec![sec];
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars);
        state.switch_tab(BarPanelTab::Apps);
        state.open_edit_form().expect("form opens");
        {
            let form = state.edit.as_mut().expect("form");
            form.kind_choice = EDIT_KIND_CHOICES
                .iter()
                .position(|k| *k == "run")
                .expect("run is in the set");
            form.value_text = "gotop --rate 2".to_string();
            form.width_text.clear();
            form.height_text.clear();
        }
        let neighbour = crate::config::ManagedSectionOverride {
            id: "clock".to_string(),
            action: Some(crate::config::ShellBarSectionActionConfig {
                kind: "popup".to_string(),
                argv: vec!["khal".to_string()],
                ..Default::default()
            }),
        };
        let list = state
            .edit_save_payload(vec![neighbour])
            .expect("a well-formed form saves");
        assert_eq!(list.len(), 2, "the neighbour rode along");
        assert!(list.iter().any(|o| o.id == "clock"));
        let cpu = list.iter().find(|o| o.id == "cpu").expect("the edit");
        let action = cpu.action.as_ref().expect("an action was written");
        assert_eq!(action.kind, "run");
        assert_eq!(
            action.argv,
            vec!["gotop".to_string(), "--rate".to_string(), "2".to_string()],
            "the value line splits into argv on whitespace"
        );
    }

    // TP-CHROME-166: a size the parser refuses refuses the Save — the form
    // stays up and says why, because a silently-dropped size and a saved one
    // look identical from the bar.
    #[test]
    fn a_bad_size_refuses_the_save_and_names_why() {
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            id: "cpu".to_string(),
            ..Default::default()
        };
        sec.action.kind = "popup".to_string();
        sec.action.argv = vec!["btop".to_string()];
        bars.top.sections = vec![sec];
        let mut state = BarConfigPanelState::open(BarEdge::Top, &bars);
        state.switch_tab(BarPanelTab::Apps);
        state.open_edit_form().expect("form opens");
        state.edit.as_mut().expect("form").width_text = "huge".to_string();
        let err = state
            .edit_save_payload(Vec::new())
            .expect_err("a bad width cannot save");
        assert!(
            err.contains("percentage"),
            "the refusal teaches the accepted shapes: {err}"
        );
    }

    // TP-CHROME-166: Esc closes the editor and only the editor — the panel
    // underneath survives, and nothing was written anywhere.
    #[test]
    fn escape_closes_the_editor_and_keeps_the_panel() {
        let mut app = test_app();
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            id: "cpu".to_string(),
            ..Default::default()
        };
        sec.action.kind = "popup".to_string();
        sec.action.argv = vec!["btop".to_string()];
        bars.top.sections = vec![sec];
        app.state.shell_bars_config = bars;

        app.open_bar_config_panel(BarEdge::Top);
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e'),
        ));
        assert!(app
            .state
            .bar_config_panel
            .as_ref()
            .is_some_and(|panel| panel.edit.is_some()));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        ));
        let panel = app.state.bar_config_panel.as_ref().expect("panel survives");
        assert!(panel.edit.is_none(), "only the editor closed");
    }

    // TP-CHROME-166: typing lands in the text field the cursor is on, and a
    // stray 'e' lands in the text rather than reopening the form.
    #[test]
    fn typing_in_the_form_edits_the_focused_text_field() {
        let mut app = test_app();
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            id: "cpu".to_string(),
            ..Default::default()
        };
        sec.action.kind = "run".to_string();
        sec.action.argv = vec!["gotop".to_string()];
        bars.top.sections = vec![sec];
        app.state.shell_bars_config = bars;

        app.open_bar_config_panel(BarEdge::Top);
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e'),
        ));
        // move the cursor to the Value field and type
        {
            let panel = app.state.bar_config_panel.as_mut().expect("panel");
            let form = panel.edit.as_mut().expect("form");
            form.field = BarAppEditField::Value;
        }
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e'),
        ));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Backspace,
        ));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('x'),
        ));
        let panel = app.state.bar_config_panel.as_ref().expect("panel");
        let form = panel.edit.as_ref().expect("the form is still up");
        assert_eq!(
            form.value_text, "gotopx",
            "'e' typed, backspace erased, 'x' typed"
        );
    }

    // TP-CHROME-166: the whole road — Save reads the overlay, keeps the
    // neighbour, writes the change, and the reload makes the disk what the
    // panel's row reads. XDG_CONFIG_HOME is pointed at a throwaway so the
    // real overlay is never touched (nextest isolates processes, so the
    // env var races nothing).
    #[cfg(unix)]
    #[tokio::test]
    async fn saving_the_form_rewrites_the_overlay_and_the_row_reads_the_new_press() {
        let dir = std::env::temp_dir().join(format!("herdr-bar-app-edit-{}", std::process::id()));
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        let config_dir = crate::config::config_dir();
        std::fs::create_dir_all(&config_dir).expect("mkdir");
        std::fs::write(
            config_dir.join("config.toml"),
            r#"
[shell.bars.top]
enabled = true
[[shell.bars.top.sections]]
kind = "content"
id = "clock"
widget = { kind = "clock" }
[[shell.bars.top.sections]]
kind = "content"
id = "cpu"
widget = { kind = "meter", metric = "cpu" }
"#,
        )
        .expect("seed config");
        // the overlay already carries a neighbour the save must not eat
        crate::config::persist_managed_bar_overrides(&[(
            "top",
            crate::config::ManagedBarOverride {
                section_overrides: vec![crate::config::ManagedSectionOverride {
                    id: "clock".to_string(),
                    action: Some(crate::config::ShellBarSectionActionConfig {
                        kind: "popup".to_string(),
                        argv: vec!["khal".to_string()],
                        ..Default::default()
                    }),
                }],
                ..Default::default()
            },
        )])
        .expect("seed overlay");

        let mut app = test_app();
        let loaded = crate::config::load_live_config().expect("the seeded config loads");
        app.state.shell_bars_config = loaded.config.shell.bars.clone();

        app.open_bar_config_panel(BarEdge::Top);
        if let Some(panel) = app.state.bar_config_panel.as_mut() {
            panel.switch_tab(BarPanelTab::Apps);
            panel.selected = 1; // the cpu row
        }
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e'),
        ));
        {
            let panel = app.state.bar_config_panel.as_mut().expect("panel");
            let form = panel.edit.as_mut().expect("form");
            form.kind_choice = EDIT_KIND_CHOICES
                .iter()
                .position(|k| *k == "run")
                .expect("run");
            form.value_text = "gotop".to_string();
            form.width_text.clear();
            form.height_text.clear();
        }
        app.save_bar_app_edit();

        // A clean save takes Apply's road: the reload closes the panel,
        // because its snapshot describes a world that just moved
        // (TP-CHROME-151).
        assert!(
            app.state.bar_config_panel.is_none(),
            "a clean save lands and closes the panel"
        );
        let written = std::fs::read_to_string(crate::config::managed_bars_path())
            .expect("the overlay was written");
        let list = crate::config::managed_section_overrides_from_str(&written, "top");
        assert_eq!(list.len(), 2, "the neighbour survived: {written}");
        assert!(list.iter().any(|o| o.id == "clock"));
        let cpu = list.iter().find(|o| o.id == "cpu").expect("the edit");
        assert_eq!(cpu.action.as_ref().expect("action").kind, "run");
        // The reload already folded the overlay back in: the world the next
        // panel opens onto reads the new press.
        let rows = section_app_rows(&app.state.shell_bars_config.top);
        assert!(
            rows[1].does.contains("gotop [run]"),
            "the reloaded bars read the new press: {}",
            rows[1].does
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    // TP-CHROME-166: the mouse reaches the form too — a press on the row a
    // field sits on moves the cursor there, and a press on Cancel closes
    // the editor without writing.
    #[test]
    fn a_press_on_a_form_row_moves_the_cursor_and_cancel_closes() {
        let mut app = test_app();
        let mut bars = crate::config::ShellBarsConfig::default();
        bars.top.enabled = true;
        let mut sec = crate::config::ShellBarSectionConfig {
            kind: "content".to_string(),
            id: "cpu".to_string(),
            ..Default::default()
        };
        sec.action.kind = "popup".to_string();
        sec.action.argv = vec!["btop".to_string()];
        bars.top.sections = vec![sec];
        app.state.shell_bars_config = bars;

        app.open_bar_config_panel(BarEdge::Top);
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Tab,
        ));
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Char('e'),
        ));
        let value_idx = BarAppEditField::ALL
            .iter()
            .position(|f| *f == BarAppEditField::Value)
            .expect("value row");
        if let Some(panel) = app.state.bar_config_panel.as_mut() {
            panel.selected = value_idx;
        }
        app.press_bar_config_panel_row();
        assert_eq!(
            app.state
                .bar_config_panel
                .as_ref()
                .and_then(|panel| panel.edit.as_ref())
                .map(|form| form.field),
            Some(BarAppEditField::Value),
            "the press moved the cursor to the field it landed on"
        );
        let cancel_idx = BarAppEditField::ALL
            .iter()
            .position(|f| *f == BarAppEditField::Cancel)
            .expect("cancel row");
        if let Some(panel) = app.state.bar_config_panel.as_mut() {
            panel.selected = cancel_idx;
        }
        app.press_bar_config_panel_row();
        let panel = app.state.bar_config_panel.as_ref().expect("panel stays");
        assert!(panel.edit.is_none(), "Cancel closed only the editor");
    }

    fn test_app() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![Workspace::test_new("main")];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app
    }

    // TP-CHROME-150: opening seeds the draft from the loaded bars and hands
    // the panel the keyboard; TP-CHROME-151: an adjustment repaints the live
    // presentation from the draft, and Esc restores the untouched snapshot.
    #[test]
    fn the_panel_previews_the_draft_and_esc_restores_the_snapshot() {
        let mut app = test_app();
        let before = app.state.shell_presentation.bars();

        app.open_bar_config_panel(BarEdge::Top);
        assert_eq!(app.state.mode, Mode::BarConfigPanel);

        // enable the top bar through the panel: the preview must repaint
        app.adjust_bar_config_panel(true); // selected=0 is Enabled
        let previewed = app.state.shell_presentation.bars();
        assert_ne!(previewed, before, "the preview reached the presentation");
        let expected = {
            let mut bars = crate::config::ShellBarsConfig::default();
            bars.top.enabled = true;
            crate::ui::shell::ShellBars::from_config(&bars)
        };
        assert_eq!(previewed, expected, "the preview is the draft, derived");

        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Esc,
        ));
        assert!(app.state.bar_config_panel.is_none());
        assert_eq!(
            app.state.shell_presentation.bars(),
            before,
            "cancel restores the pre-panel presentation"
        );
    }

    // TP-CHROME-151: Apply writes exactly the diff to bars.managed.toml and
    // takes the reload road, so the disk — not the preview — is what the
    // surfaces end up showing. Isolated under a throwaway XDG_CONFIG_HOME;
    // nextest runs each test in its own process, so the env var leaks nowhere.
    #[test]
    fn apply_writes_the_diff_to_the_managed_file_and_reloads() {
        let dir =
            std::env::temp_dir().join(format!("herdr-bar-panel-apply-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &dir);

        let mut app = test_app();
        app.open_bar_config_panel(BarEdge::Top);
        app.adjust_bar_config_panel(true); // Enabled: off -> on
                                           // move to Apply and press it on the real key road
        let rows = panel_rows(BarEdge::Top);
        let apply_idx = rows
            .iter()
            .position(|row| *row == BarPanelRow::Apply)
            .unwrap();
        if let Some(panel) = app.state.bar_config_panel.as_mut() {
            panel.selected = apply_idx;
        }
        app.handle_bar_config_panel_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Enter,
        ));

        assert!(
            app.state.bar_config_panel.is_none(),
            "apply closes the panel"
        );
        let written = std::fs::read_to_string(
            crate::config::managed_spaces_path().with_file_name("bars.managed.toml"),
        )
        .expect("the managed bars file was written");
        assert!(written.contains("[shell.bars.top]"), "{written}");
        assert!(written.contains("enabled = true"), "{written}");
        assert!(
            !written.contains("style"),
            "an untouched field is not written: {written}"
        );
        // the reload road re-derived the presentation from disk: the managed
        // file enables the top bar, so the presentation shows one.
        let expected = {
            let mut bars = crate::config::ShellBarsConfig::default();
            bars.top.enabled = true;
            crate::ui::shell::ShellBars::from_config(&bars)
        };
        assert_eq!(app.state.shell_presentation.bars(), expected);

        let _ = std::fs::remove_dir_all(&dir);
    }

    // TP-CHROME-151: Apply with nothing changed is a cancel — no file, no
    // reload, no trace.
    #[test]
    fn apply_with_no_changes_writes_nothing() {
        let dir = std::env::temp_dir().join(format!("herdr-bar-panel-noop-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("XDG_CONFIG_HOME", &dir);

        let mut app = test_app();
        app.open_bar_config_panel(BarEdge::Left);
        app.apply_bar_config_panel();
        assert!(app.state.bar_config_panel.is_none());
        assert!(
            !crate::config::managed_spaces_path()
                .with_file_name("bars.managed.toml")
                .exists(),
            "an untouched panel leaves no file behind"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
