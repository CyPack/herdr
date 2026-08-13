---
doc: herdr-references-registry
domain: tui-composition
created: 2026-07-13
status: canonical — çıplak iddia yok; her giriş tier + confidence taşır (evidence-propagation uyumlu)
git_note: >
  /docs/* herdr .gitignore'da IGNORED (yalnız /docs/next/ istisna) → bu dizin LOKAL yaşar,
  upstream'e/PR'a SIZMAZ (external-contributor guardrail'e bilinçli uyum). Makine-kopyası:
  .cartography/tui-composition-SYSTEM-MAP.json
agentic_triggers:
  - "herdr composable shell · herdr panel sistemi · herdr popup mimarisi · herdr sayfa sistemi"
  - "compositor pattern · component trait · dockable panel · plugin ui composition"
  - "zellij layout · helix compositor · k9s pagestack · gitui component"
related:
  - docs/patterns/tui-composition.md              # pattern kataloğu (bu registry'nin damıtılmış hâli)
  - .cartography/tui-composition-SYSTEM-MAP.json  # evidence graph
  - src/layout.rs                                  # herdr'ın mevcut BSP pane tree'si (baseline)
  - src/ui.rs + src/ui/*.rs                        # herdr'ın mevcut ad-hoc overlay render fonksiyonları (baseline)
---

# herdr Referans Registry — DOMAIN: tui-composition

> "Advanced terminal apps" composition-mimarisi araştırması (2026-07-13, 6 paralel agent, hepsi
> resmî docs veya kaynak-kod okumasıyla doğrulandı). Amaç: herdr'ı "desktop-app shell in a terminal"
> hâline getirecek proven pattern'leri tespit etmek (named regions, swappable component slots,
> addable pages, popups, dockable panels).

## Tier sözlüğü
`official` (proje resmî docs'u) · `source_code` (doğrudan okunmuş kaynak kod — en güçlü kanıt) ·
`official-corroborating` (resmî docs, kaynak-kod bulgusunu bağımsız doğruluyor) · `community` (blog/wiki,
zayıf tekil kanıt) · `local_source` (bu makinede zaten klonlu repo, doğrudan okundu).

## zellij (Rust multiplexer — aynı kategori: multiplexer)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[zellij-layout]` | `zellij-utils/src/input/layout.rs` (raw.githubusercontent.com/zellij-org/zellij/main/...) | source_code | 0.95 | `TiledPaneLayout` recursive split tree, `SplitSize::Percent\|Fixed`, `Run` enum (Plugin\|Command\|EditFile\|Cwd) leaf |
| `[zellij-kdl-docs]` | https://zellij.dev/documentation/creating-a-layout.html | official | 0.9 | KDL layout syntax, `stacked=true`, pane_template/tab_template |
| `[zellij-resize-docs]` | https://zellij.dev/documentation/keybindings-possible-actions.html | official | 0.95 | `Resize` action args (Left\|Right\|Up\|Down\|Increase\|Decrease) |
| `[zellij-stacked-resize]` | https://zellij.dev/news/stacked-resize-pinned-panes/ + /tutorials/stacked-resize/ | official | 0.9 | Alt+/- grow/shrink, undo-chain, pin (always-on-top) |
| `[zellij-floating]` | `zellij-server/src/panes/floating_panes/mod.rs` | source_code | 0.9 | `z_indices: Vec<PaneId>`, `get_pane_z_index`, `focus_pane*`, `make_sure_pinned_panes_are_on_top` |
| `[zellij-plugin-trait]` | `zellij-tile/src/lib.rs` | source_code | 0.9 | `ZellijPlugin` trait: `render(&mut self, rows, cols)` |
| `[zellij-plugin-pane]` | `zellij-server/src/panes/plugin_pane.rs` | source_code | 0.9 | Plugin = `PluginPane` implementing same `Pane` trait as `TerminalPane`; per-client `vte::Parser`+`Grid` |
| `[zellij-wasm-runtime]` | — | — | — | ⚠️ unverified — wasmtime vs wasmi conflicting secondary signal, not resolved |

## helix (Rust editor — Compositor+Component, HIGHEST relevance)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[helix-compositor]` | `helix-term/src/compositor.rs` (github.com/helix-editor/helix/blob/master/...) | source_code | 0.95 | `Compositor{layers: Vec<Box<dyn Component>>}`, event routing (`.rev()`, Consumed short-circuit), render (full-Rect back-to-front) |
| `[helix-architecture-doc]` | https://github.com/helix-editor/helix/blob/master/docs/architecture.md | official | 0.95 | Independent corroboration: "Components... Popup and Overlay can take other components as children" |
| `[helix-overlay]` | `helix-term/src/ui/overlay.rs` | source_code | 0.95 | `Overlay<T>` generic centering/size-constraint decorator (full 85-line file) |
| `[helix-popup]` | `helix-term/src/ui/popup.rs` | source_code | 0.95 | `Popup<T: Component>` generic bordered/anchored/scrollable/esc-close decorator |
| `[helix-menu]` `[helix-prompt]` `[helix-picker]` | `helix-term/src/ui/{menu,prompt,picker}.rs` | source_code | 0.95 | Concrete `Component` impls (completion menu, `:` command line, fuzzy picker) |
| `[helix-editorview]` | `helix-term/src/ui/editor.rs` | source_code | 0.95 | Base/bottom layer; split-tree rendering happens INSIDE this one Component, not in the Compositor |
| `[helix-tree]` | `helix-view/src/tree.rs` | source_code | 0.95 | n-ary, same-direction-flattening split tree (SlotMap-backed), iterative resize |
| `[helix-application]` | `helix-term/src/application.rs` | source_code | 0.95 | `Compositor::new` + `push(EditorView)`, draw loop, `layer_count()==1` used as "no popups open" check |

## lazygit (Go, gocui) + gitui (Rust/ratatui)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[lazygit-boxlayout]` | `pkg/gui/layout.go`, `controllers/helpers/window_arrangement_helper.go` | source_code | 0.95 | Declarative `boxlayout.Box` tree rebuilt every frame from live state (responsive) |
| `[lazygit-context]` | `pkg/gui/types/context.go`, `pkg/gui/context.go` | source_code | 0.95 | `ContextKind` enum + `ContextMgr.ContextStack` push/pop/replace rules |
| `[lazygit-popup]` | `pkg/gui/controllers/helpers/confirmation_helper.go`, `pkg/gui/popup/popup_handler.go` | source_code | 0.95 | Separate content-sized/centered popup positioning pass, reapplied every layout() |
| `[gitui-component]` | `src/components/mod.rs` (github.com/gitui-org/gitui) | source_code | 0.95 | `DrawableComponent`+`Component` traits; explicit doc-comment: "composition by CODE not by DATA" |
| `[gitui-app]` | `src/app.rs` | source_code | 0.95 | Flat struct: 5 tabs (match) + ~30 named popup fields, first-responder `event_pump` |
| `[gitui-popupstack]` | `src/popup_stack.rs` | source_code | 0.9 | Separate small `Vec<StackablePopupOpen>` — ESC-dismissal nav order only |

## k9s (Go, tview — registry+page-stack, HIGH relevance for "addable pages")

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[k9s-model-component]` | `internal/model/types.go` | source_code | 0.95 | Universal `model.Component` interface every screen implements |
| `[k9s-stack]` | `internal/model/stack.go`, `internal/ui/pages.go`, `internal/view/page_stack.go` | source_code | 0.95 | Generic LIFO `model.Stack` → `ui.Pages` (tview.Pages wrapper) → `view.PageStack` (Start/Stop+focus lifecycle) |
| `[k9s-registrar]` | `internal/view/registrar.go` | source_code | 0.95 | `MetaViewers map[*client.GVR]MetaViewer{viewerFn,enterFn}` — registry, not hardcoded switch |
| `[k9s-command]` | `internal/view/command.go` | source_code | 0.9 | `:command` → alias→GVR→registry lookup→construct→`inject`→`Content.Push` (same path as drill-down) |
| `[k9s-app-layout]` | `internal/view/app.go` (`layout()`, `buildHeader()`) | source_code | 0.95 | Persistent `tview.Flex` chrome (header/status→**Content**→crumbs→flash); only Content swaps |
| `[k9s-styles]` | `internal/config/styles.go` | source_code | 0.9 | Skins = pure color layer via `StyleListener`, verified NOT to affect layout |
| `[k9s-alias-resolve]` | `internal/config/alias.go` | community (unread) | 0.7 | ⚠️ exact `Resolve()` mechanics not source-read, only search-corroborated |

## neovim / emacs (window/buffer/float — popup primitive)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[nvim-windows-doc]` | https://neovim.io/doc/user/windows.html (`runtime/doc/windows.txt`) | official | 0.95 | buffer(content) vs window(viewport) decoupling, split-tree by bisection |
| `[nvim-api-doc]` | https://neovim.io/doc/user/api.html (`runtime/doc/api.txt`) | official | 0.9 | `nvim_open_win()` full param surface (relative/anchor/zindex/style=minimal/border) |
| `[nvim-zindex-issue]` | https://github.com/neovim/neovim/issues/18486 | official (issue tracker) | 0.85 | Default float zindex=50; builtin popup=100, msg=200, cmdline-completion=250 |
| `[telescope-pickers]` | `nvim-telescope/telescope.nvim` `lua/telescope/pickers.lua` | source_code | 0.9 | 3 floating windows (prompt/results/preview), buffer-scoped autocmds |
| `[which-key-win]` | `folke/which-key.nvim` `lua/which-key/win.lua` | source_code | 0.9 | scratch buffer + `nvim_open_win(zindex=1000)` |
| `[alpha-nvim]` | `goolord/alpha-nvim` `lua/alpha.lua` | source_code | 0.9 | scratch buffer swapped into CURRENT normal window (no float at all) — proves placement is decoupled from content-construction |
| `[emacs-window-internals]` | https://www.gnu.org/software/emacs/manual/html_node/elisp/Window-Internals.html | official | 0.9 | buffer→window(tree)→frame 3-tier model |
| `[which-key-el]` | https://github.com/justbur/emacs-which-key `README.org` | official | 0.85 | 3 popup backends: minibuffer / side-window / child-frame (no single unified float API) |

## Rust TUI ecosystem + local refpool (yazi, superfile, tui-realm, cursive, ratatui/templates)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[yazi-ui-layout]` | `~/.cartography/refpool/yazi-src/yazi-binding/src/elements/layout.rs:9-80` | local_source | 0.95 | `ui.Layout` Lua binding wraps `ratatui_core::layout::Layout` directly |
| `[yazi-lua-components]` | `.../yazi-plugin/preset/components/{root,tab}.lua` | local_source | 0.95 | Unenforced Lua duck-type convention: `new/layout/build/reflow/redraw` |
| `[yazi-renderable]` | `.../yazi-widgets/src/renderable.rs:10-21,38-56,98-116` | local_source | 0.95 | Closed set of Rust UserData renderable primitives, TypeId-matched into `Renderable` enum |
| `[yazi-renderer]` | `.../yazi-fm/src/renderer.rs:29-51` | local_source | 0.95 | Host calls named Lua global's `.new(area)`/`.redraw()`, flattens, renders |
| `[yazi-fixed-popups]` | `.../yazi-fm/src/root.rs:24-65` | local_source | 0.95 | Hardcoded fixed-order ~9-popup if-chain (anti-pattern — avoid) |
| `[yazi-modal-registry]` | `.../yazi-fm/src/mgr/modal.rs:1-19`, `.../yazi-plugin/preset/components/modal.lua:1-44` | local_source | 0.9 | Exception: `children_add(component,order)`/`children_remove(id)` — real dynamic z-ordered popup registry, unused internally |
| `[superfile-model]` | `~/.cartography/refpool/superfile-src/src/internal/type.go:53-75` | local_source | 0.9 | Flat Elm-model (bubbletea), one field per panel/modal — negative/confirming example |
| `[superfile-rendering-readme]` | `.../src/internal/ui/rendering/README.md:1-12` | local_source | 0.95 | Own README admits hand-rolled compositor is ad-hoc |
| `[tui-realm-component]` | `veeso/tui-realm` `crates/tuirealm/src/core/component.rs:29-70` | source_code | 0.95 | `Component`/`AppComponent` traits |
| `[tui-realm-application]` | `.../core/application.rs:26-297` | source_code | 0.95 | `Application<ComponentId,Msg,UserEvent>` mount/umount/view/query/attr/active/tick registry |
| `[cursive-view]` | `gyscos/cursive` `cursive-core/src/view/view_trait.rs:32-140` | source_code | 0.95 | `View` trait (draw/layout/on_event/required_size/take_focus) |
| `[cursive-stackview]` | `.../cursive-core/src/views/stack_view.rs:18-37,342-830` | source_code | 0.95 | `StackView{layers}` — modal-aware, front-to-back dispatch, always-laid-out background |
| `[ratatui-app-patterns]` | https://ratatui.rs/concepts/application-patterns/ | official | 0.85 | No official pattern endorsed; points to tui-realm for TEA |
| `[ratatui-templates-component]` | `ratatui/templates` `component-generated/src/{components,app}.rs` | source_code | 0.95 | Closest "official" reference: `Vec<Box<dyn Component>>` + tokio mpsc `Action` bus |

## herdr baseline (this repo, local)

| Etiket | Kaynak | Tier | Conf | Konu |
|---|---|---|---|---|
| `[herdr-bsp]` | `src/layout.rs` | source_code | 0.9 | Existing BSP pane tree (`PaneId`, `Direction`, f32 ratio) — structurally analogous to zellij/helix trees, but leaves are terminal-only (no `Run`-like enum), no separate floating layer |
| `[herdr-ui-adhoc]` | `src/ui.rs` + `src/ui/{dialogs,menus,sidebar,tabs,status,...}.rs` | source_code | 0.9 | Hand-written `render_X()` free functions dispatched from central `render()` — closest to gitui's "composition by code" (c5) / superfile's flat-field pattern (c10), but without even gitui's shared trait |

## Kayıt kuralı (yeni kaynak eklerken)

1. Etiket ver (`[kebab-case]`), tabloya satır ekle — tier + confidence ZORUNLU.
2. URL ise canlılık zaten bu araştırmada WebFetch ile doğrulandı (2026-07-13); yeni URL eklerken tekrar doğrula.
3. Kaynak bir pattern'i destekliyorsa `docs/patterns/tui-composition.md`'deki pattern ID'sini yaz.
4. Harita bağlantısı: `.cartography/tui-composition-SYSTEM-MAP.json`'a claim/evidence olarak işle.

---
*v1.0.0 — 2026-07-13 · 6 paralel araştırma agent'ı (general-purpose), tamamı source-code veya official-docs doğrulamalı, 0 uydurma/ölü URL.*
