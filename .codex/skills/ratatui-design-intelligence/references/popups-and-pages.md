# Popups, Modals, Overlays and Pages

## Overlay rendering contract

Ratatui's official popup recipe and R014 `Clear` establish the draw order:

1. compute a bounded overlay rectangle;
2. render `Clear` into that rectangle;
3. render overlay surface and border;
4. render content, footer actions and focus indicator;
5. keep input ownership outside render.

Without `Clear`, background text, modifiers and colors can leak through.

## Overlay stack model

Use an explicit client-side stack:

```text
OverlayEntry {
  id,
  kind: modal | popover | context_menu | command_palette | toast,
  anchor_or_placement,
  focus_scope,
  dismiss_policy,
  payload,
}
```

Topmost blocking entry handles input first. Escape closes only the top eligible
entry. Mouse clicks outside dismiss only when its policy allows. Closing restores
the prior focus owner if it still exists.

## Evidence-backed patterns

### R033 hypertile command/plugin palette

`extras/src/runtime/render.rs::HypertileRuntime.render_palette` clears and draws
a centered bounded list, clamps visible rows and selection, and styles selected
content. Reuse the geometry and lifecycle, replace hard-coded colors with Herdr
tokens.

### R061 tui-studio authoring catalog

The repository contains command palette, component palette, export modal, save
dialog, settings, help, about and changelog modal patterns. It is valuable for
surface inventory, authoring flow and consistent modal families. It is a web
application; CSS radius and DOM event behavior are not directly reusable in
Ratatui.

### Official Ratatui R014

`Clear`, `Block`, `Paragraph`, `Rect::centered`, `Constraint` and `Flex::Center`
are the minimal popup toolkit. They do not supply modal input isolation or a
stack manager.

### New candidates

- `ratatui-kit`: `Modal`, `ConfirmModal`, `AlertModal`,
  `ShortcutInfoModal`, router/outlet and centralized input layers.
- `ratatui-interact`: `PopupDialog`, `HotkeyDialog`, `ContextMenu`, `MenuBar`,
  `ToastStack`, dropdown select and focus/click traits.
- `rat-salsa` / `rat-widget`: dialog windows, popup/menu/focus crates and a
  mature multi-page widget set.
- `tui-popup`: useful centered/auto-sized/draggable popup history, but its repo
  moved to `tui-widgets`; prefer the maintained destination after indexing.

## Page architecture

Pages are client presentation routes, not server runtime entities. Prefer:

```text
PageId -> PageState -> compute_page_view(area, app_state) -> PageView
                                               |
                                               +-> render_page(PageView)
```

Each page declares minimum viable geometry and degradation order. Example IDE
shell pages:

- workspace: explorer + editor/terminal + inspector;
- agents: agent list + detail + activity stream;
- files: Miller columns + preview/actions;
- tasks/events: filterable list + details/timeline;
- settings/plugins: navigation + form/editor + validation summary.

On narrow terminals, secondary persistent panels become tabs, drawers or
popups. Do not squeeze every surface until content becomes unusable.

## Modal family definition

Every modal kind should declare:

- sizing: fixed, content-bounded, percentage, or anchored;
- minimum and maximum width/height;
- focus order and initial focus;
- confirm/cancel/dismiss rules;
- background input blocking;
- overflow/scroll behavior;
- loading, empty, error and disabled variants;
- mouse hit regions and keyboard parity;
- no-color/ASCII appearance.

## Required tests

- tiny area clamps safely or declines to open with a visible reason;
- background is cleared and does not leak styles;
- top overlay consumes input; background shortcuts do not fire;
- nested overlay close restores valid previous focus;
- click-outside policy and Escape policy are independent;
- long titles/body/actions truncate or scroll without overflow;
- route changes preserve only explicitly persistent page state.
# R073 Archer page and overlay family

Archer supplies four coherent full-page templates: launcher, execution
dashboard, run-history browser and configuration editor. Each retains the page
under a full-screen blocking overlay and centers a rounded permission, review,
input, chooser, confirmation, message or summary modal. Permission and review
requests are queued. In Herdr, implement the same family through one overlay
stack/focus contract, route input only to the top blocker, and compute popup and
background geometry before pure rendering.
