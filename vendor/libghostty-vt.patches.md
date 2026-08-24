# libghostty-vt local patches

This file tracks intentional local changes applied on top of the vendored
`libghostty-vt` source. Remove a patch only when the vendored source commit
contains the upstream behavior and the listed verification still passes.

## 0001 default lib-vt panes to grapheme clustering

status: active

patch: `vendor/patches/libghostty-vt/0001-default-grapheme-cluster-mode.patch`

herdr issue: https://github.com/herdrdev/herdr/issues/243

upstream discussion: not opened; libghostty-vt currently exposes current mode mutation but no C API for configuring terminal default modes

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/terminal/c/terminal.zig`

reason: Herdr renders terminal cells directly and requires DEC private mode
2027 to store flags, ZWJ emoji, and other multi-codepoint grapheme clusters in
one cell. This patch makes clustering active for new terminals and keeps it as
the reset default so RIS (`ESC c`) does not disable it.

remove when: libghostty-vt exposes a C API for setting default mode 2027, or
upstream makes grapheme clustering the lib-vt default, and the reset-survival
regression passes without this patch.

verification:

```sh
cargo nextest run --locked grapheme_cluster_mode_is_default_and_survives_full_reset
cargo nextest run --locked grapheme_cluster_mode_renders_flag_emoji_in_single_wide_cell
cargo nextest run --locked grapheme_cluster_mode_renders_zwj_family_in_single_wide_cell
```

## 0002 expose modifyOtherKeys mode through terminal data

status: active

patch: `vendor/patches/libghostty-vt/0002-expose-modify-other-keys-mode.patch`

herdr issue: none; fixes the performance regression exposed by
https://github.com/herdrdev/herdr/pull/2303

upstream discussion: not opened

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/include/ghostty/vt/terminal.h`
- `vendor/libghostty-vt/src/terminal/c/terminal.zig`

reason: Herdr must know whether xterm modifyOtherKeys mode 2 is active to
request printable key releases from the outer terminal. The formatter API can
recover this fact only by formatting the active screen and scrollback. A typed
terminal-data query exposes the authoritative scalar without formatting or
allocation.

remove when: the vendored source exposes an equivalent scalar query for
modifyOtherKeys mode 2 and Herdr can use it without this patch.

verification:

```sh
cargo nextest run --locked modify_other_keys_query_tracks_mode_two
cargo nextest run --locked host_report_all_supplies_printable_releases_for_event_type_only_panes
python3 -m unittest scripts.test_vendor_libghostty_vt scripts.test_ui_hot_path_architecture
```

## 0003 expose a screen-selectable formatter in the lib-vt C API

status: active

patch: `vendor/patches/libghostty-vt/0003-formatter-screen-new.patch`

herdr issue: not opened; found while closing the alt-screen handoff/dormancy
history loss (TP-HANDOFF-HIST-02, TP-DORMANT-10)

upstream discussion: not opened; the core formatter's own docs point at this
shape ("If you want to emit data for all screens, you should manually
construct a no-content terminal formatter, followed by screen formatters")
but the C API only exposes the terminal formatter, which always reads the
active screen

upstream pr: not opened

vendored base: `c5a21edfcbc2d5b46540ad91b7980aca31f5f1f3`

local files:

- `vendor/libghostty-vt/src/terminal/c/formatter.zig`
- `vendor/libghostty-vt/src/terminal/c/main.zig`
- `vendor/libghostty-vt/src/lib_vt.zig`
- `vendor/libghostty-vt/include/ghostty/vt/formatter.h`

reason: Herdr's live-handoff freight and pane dormancy capture a pane's full
primary-screen scrollback. Every existing C-API read resolves through the
active screen, so a pane caught inside a fullscreen application (alternate
screen) captured nothing and lost its primary history. The new
`ghostty_formatter_screen_new(allocator, formatter, terminal, screen,
options)` constructs a ScreenFormatter for the requested
`GhosttyTerminalScreen` regardless of which screen is active; a NULL
selection formats the whole screen. The Rust extern lives in
`src/ghostty/bindings_ext.rs` (hand-written; `bindings.rs` is generated
offline by rust-bindgen and must stay pristine).

remove when: libghostty-vt exposes a C API that can read a non-active
screen's content (a screen-keyed formatter or grid-ref resolver), and the
verification below passes without this patch after moving
`bindings_ext.rs` to the upstream symbol.

verification:

```sh
zig build test-lib-vt -Dtest-filter="screen_new"   # in vendor/libghostty-vt
cargo nextest run --locked reading_the_primary_screen_while_the_alternate_is_active
cargo nextest run --locked handoff_history_ansi_full_reads_the_primary_screen_from_the_alternate
cargo nextest run --locked an_alt_screen_retired_pane_sleeps_with_its_primary_history
cargo nextest run --locked live_handoff_preserves_primary_history_of_an_alt_screen_pane
```
