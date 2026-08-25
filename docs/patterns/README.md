# docs/patterns — the contributor layer

User docs live in `docs/next/website/src/content/docs/` (published to
`/docs/preview/`, promoted on release). **This directory is the layer under
that**: how the fork's surfaces are built, measured and extended, written for
the next person changing the code. Every claim here is anchored to a landed
commit, a `TP-*` behavior id (see `behaviors/`), or a named test.

## Map

| file | owns |
|---|---|
| `bar-surface-platform.md` | Edge bars end to end (SP0 model → SP5 delivery): sections, chrome, panel, spec gates |
| `bar-icons.md` | Pixel/glyph icon pipeline for bar sections |
| `native-file-manager.md` | Files surface: trail/miller/locations, resident lifecycle, stage vs beside |
| `fm-trail-filter.md` | The `/` filter road (editor, live narrowing, cursor normalization) |
| `tui-composition.md` | compute→render split, view projections, hit-area discipline |
| `multi-client-focus.md` | Two clients, focus authority, resize interplay |
| `headless-loop-cadence.md` | The 98%-core spin incident: past-due deadline clamp, 10ms housekeeping gate, live profiling marker |
| `custom-layout.md` | Pane layout maths |
| `document-rendering.md` | Preview/detail rendering (text, image, PDF) |
| `machine-readable-surfaces.md` | `herdr shell spec` and the two-way doc gates |
| `measurement-discipline.md` | How this repo measures before it claims |
| `resource-doctrine.md` | RD0-RD9: no cost while nobody looks, bytes ∝ changed cells |
| `rust-engineering.md` | HP1-HP10 code-layer principles |
| `pty-ipc-runtime.md` | PTY/IPC runtime layer |
| `remote-media-transport.md` | Remote client audio/video design |
| `feature-change-map.md` | Where a change of each kind usually lands |
| `fm-architecture-decision.md` | Files surface model evaluated (stage vs pane vs embed vs hybrid), the DnD verdict, and the web-port references |

## The rules that bind every recipe

1. **A fork behavior is a registered behavior.** Marker `TP-<FAMILY>-<NN>`
   above the owning tests **and** a row in `behaviors/<feature>.md`, same
   commit. `python3 -m scripts.behavior_registry_check` fails the build on a
   marker without a test. Pick the number first:
   `grep -rho 'TP-<FAMILY>-[0-9]*' behaviors/ | sort -V | tail`.
2. **RED before code.** A new behavior's test goes red without the code (for a
   pure style tweak, a removal-mutant probe after the fact is the accepted
   substitute). Then green, then a mutation pass on the committed base:
   apply mutant → the owning test fails → revert → `git diff` clean.
3. **The full gate before landing.** `just check` (via the remote build box
   where mandated). Landing is `wt.sh auto <branch>` — serial, gate-tested
   against the *merge*, never the branch alone.

## Recipe 1 — a new bar widget kind

Proven by the `clock`/`sparkline` family and, for the render-side color road,
`feat(shell): a written section color reaches island text` (`60b733e7`,
TP-CHROME-163).

1. **Model**: extend the widget config in `src/config/model.rs`; refuse
   nonsense in `shell_bar_config_problems` (source.rs) so a bad key is a
   *named* problem, not an empty section.
2. **Spec**: add the kind + its keys to `herdr shell spec` output. The spec is
   parser-derived; the two-way gates (`the_guide_shows_every_widget_and_action_kind`,
   TP-SPEC family) fail until spec, parser and guide agree.
3. **Chrome/source**: resolve the widget in `src/ui/shell/source.rs`
   (`section_widget`); carry any per-section tone through `SlotChrome`
   (`Pill{bg,fg}` / `Frame{backdrop,fg}` — 60b733e7 is the template for adding
   a field and updating every match site).
4. **Render**: draw in `widgets::render_section_widget` (ui.rs picks the style
   per chrome). Product proof is a TestBackend cell assertion — glyph *and*
   fg/bg, not glyph alone.
5. **Docs**: one `### ` subsection + a ```toml example inside `## Edge bars`
   of `docs/next/.../configuration.mdx`. Every fence there is harvested by
   `every_bar_example_in_the_guide_is_a_config_this_build_accepts` — a broken
   example fails the gate, which is the point.
6. Registry row + marker, mutation pass, `wt.sh auto`.

## Recipe 2 — a new Files header verb

Proven by the copy-path verb (S30-5b) and the action-bar model tests.

1. **Verb enum**: `FileManagerHeaderAction` in `src/app/state.rs`; give it a
   label and an enable-predicate in `compute_file_manager_action_bar_model`.
2. **Paths authority**: `file_operation_worker::current_action_paths` decides
   what the verb acts on; the mouse road refuses a press whose paths vanished
   (`handle_file_manager_mouse` header-action arm).
3. **Execution**: implement in the file-operation worker, off the render
   thread; report through the operation state the bar already shows.
4. **Tests**: model test (enabled exactly when), dispatch test (click the
   projected `file_manager_header_action_areas` rect), worker test (the
   operation itself), plus a disabled-press test — a dead button must consume,
   not fall through.
5. Marker + `behaviors/file-manager.md` row, mutation pass, land.

## Recipe 3 — a new overlay / dialog

Proven by the module-delete confirmation (`0d70ff78`+`b2d872fa`, TP-MOD-43)
and the attachment-picker mirror (GP1).

1. **State**: an `Option<XState>` on `AppState` + a `Mode::` variant; opening
   fills both, closing clears both. No render-time state.
2. **Geometry**: a pure `x_popup_rect(area)` + `x_button_rects(rect)` pair in
   `src/ui/dialogs.rs` — pure functions the mouse handler shares, so hit-test
   and paint cannot drift.
3. **Render**: draw from the computed rects only; assert the *style* of the
   decisive cell in a TestBackend test (the accent-bg cancel button of
   TP-MOD-43 is the template: the safe default must be visibly the bright one).
4. **Input**: keyboard in `modal.rs`, mouse hits in `mouse.rs`, both through
   the shared rects; every button gets its own dispatch test.
5. **Seam the effect**: route the destructive action through one injectable
   seam (`delete_module_target_at(path, &target)` is the template) so tests
   pin routing without touching the live config.
6. Marker + registry row, mutation pass (kill the dialog-open, each button,
   and the seam-routing separately), land.
