# Files Sidebar Mouse Runtime Matrix

Date: 2026-07-18

## Outcome

FMR-2 is closed. A real primary click on a visible Files shortcut now has
end-to-end test coverage through the production chain:

`sidebar geometry -> model-revalidated exact path -> one-shot request ->
scheduled consumer -> FmState::open_trail_to`.

The current source already completed that chain for plain primary clicks. The
reboot-only field symptom was therefore executable drift, not lost Git state:
plain `/home/ayaz/.local/bin/herdr` was the older 2026-07-12 build while the
repository debug binary carried the current 2026-07-18 source.

The characterization did uncover one separate authority defect: a modified
primary click could enter the Files shortcut path branch because that branch
did not require empty modifiers. It now accepts only a plain primary press and
still swallows every other mouse event in the shortcut region without
activating navigation.

No installed or debug process, server, socket, or user session was stopped,
restarted, signalled, or replaced during this task.

## Mouse authority matrix

| Case | Expected result | Evidence |
|---|---|---|
| plain primary press on Home | request consumed once; exact root Trail loaded | `sidebar_shortcut_mouse_click_consumes_to_loaded_trail` |
| generation | opening the shortcut preserves the current Files generation | same end-to-end test |
| target contents | loaded snapshot contains the target directory entry | same end-to-end test |
| modified primary click | no request, no stage or Trail mutation | `sidebar_shortcut_mouse_modified_click_is_inert` |
| middle button / release | inert | `sidebar_shortcut_mouse_non_primary_and_inaccessible_rows_are_inert` |
| inaccessible configured row | fail closed; no request | same non-primary/inaccessible test |
| symlink-directory pin | exact symlink path becomes Trail root and contents load | `sidebar_shortcut_mouse_symlink_directory_loads_exact_trail` |
| stale hit geometry | model refresh invalidates the old row | `stale_file_sidebar_hit_area_is_inert_after_model_refresh` |
| collapsed Files group | shortcut row is inert | `collapsed_sidebar_files_tab_is_inert` |
| blocking overlay | background mouse action is swallowed | `overlay_blocks_every_background_mouse_action` |
| typed request seam | exact path and generation are prepared | `clicking_file_sidebar_item_prepares_exact_typed_navigation_request` |
| scheduled consumer | fresh target opens; stale target is rejected | `sidebar_navigation_opens_exact_directory_and_rejects_stale_targets` |

Home is the deterministic built-in shortcut used for the real click-to-load
test. Configured shortcut rows use the same geometry and typed request path;
the symlink-directory fixture exercises that configured-row path without
depending on the host's Downloads directory.

## TDD chain

| Layer | Commit |
|---|---|
| modifier-authority RED | `0b69b557` |
| plain-primary modifier gate GREEN | `0b8ab32f` |
| full end-to-end and adversarial characterization | `918ae4df` |

The first implementation attempt used a Rust 2024-style let-chain in this
Rust 2021 crate. It was replaced with nested `if` statements before the GREEN
commit; no unsupported syntax remains.

## Fresh gates

- Focused mouse/navigation family: 8/8 PASS.
- Rust: 3,521/3,521 PASS, 2 skipped.
- Linux clippy: clean with `-D warnings`.
- Windows clippy: clean for `x86_64-pc-windows-msvc`.
- Playwright Chromium: 21/21 PASS with unchanged approved baselines.
- Formatting: clean.

## Next dependency

FMR-3 is next: define and verify the native/metadata/plugin/unsupported file
preview capability matrix while preserving bounded work, generation
cancellation, escape sanitization, pure rendering, and zero navigation
mutation.
