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

/// Blocking client-local panel state. Owns no watcher, worker, process, pane,
/// or server state; closing it discards only presentation data — the draft.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BarConfigPanelState {
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
}

impl BarConfigPanelState {
    pub(crate) fn open(edge: BarEdge, bars: &ShellBarsConfig) -> Self {
        Self {
            edge,
            draft: bars.clone(),
            original: bars.clone(),
            scope_all: false,
            selected: 0,
        }
    }

    pub(crate) fn rows(&self) -> Vec<BarPanelRow> {
        panel_rows(self.edge)
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
        let known = crate::ui::shell::bar_color_tokens()
            .iter()
            .any(|token| *token == original);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
