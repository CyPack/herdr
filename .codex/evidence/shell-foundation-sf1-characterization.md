# Shell Foundation SF1 Characterization Evidence

Date: 2026-07-15

## Decision

Result: **PASS; SF1 and delivery I6 are closed. I7/SF2 is next.**

SF1 changed tests only. It did not add or modify Shell Foundation production
behavior. The current Files curtain remains explicitly characterized as a
before-state for SF6.

## Git and File Boundary

- Product/test commit: `7b9b626d` (`test: characterize shell foundation baseline`).
- Exact commit SHA: `7b9b626d3e84b4ba03856a8f54b81196985d3f48`.
- Changed path: `src/ui.rs` tests only, 79 insertions.
- At the product/test publication checkpoint, CyPack `feat/native-fm` and fork
  `master` both resolved to the exact SHA; the later continuity closure keeps
  this commit as their direct ancestor.
- Both remote refs were proven ancestors before sequential fast-forward pushes.
- `upstream` was not pushed; no force push occurred.
- The user-owned untracked `.superpowers/` tree was not staged.

## Characterization Contract

`files_curtain_currently_replaces_terminal_surface` proves all three frozen
facts inside the computed `terminal_area`:

1. a bounded Files marker is visible;
2. the terminal pane marker is absent behind the Files curtain;
3. the terminal runtime registry retains the same count and exact terminal ID.

The fixture owns a unique temp root and removes it through a `Drop` guard even
when an assertion panics. A post-test `/tmp` inventory found zero matching
fixture roots.

## RED Validity and Diagnostic Failures

SF1 is characterization, so its intended result is GREEN. Two early failures
were fixture/setup failures and are explicitly rejected as behavior RED:

- A synchronous test panicked before assertions because
  `TerminalRuntime::test_with_screen_bytes` creates a Tokio task. Existing
  Herdr runtime-render fixtures proved the correct `#[tokio::test]` contract.
- The first long filename assertion reached render but failed because the
  responsive Miller column correctly displayed `FM_CURTAIN_VISIBLE…`. The
  bounded surface output proved Files was present and terminal content absent;
  the fixture now uses the short marker `FM_VISIBLE`.

No production code was written in response to either diagnostic failure.

## Fresh Test Evidence

- Existing frozen baseline before the new test: 10/10 pass, Nextest run
  `0871044a-6b1a-465c-9a0c-81185f055525`.
- New characterization alone: 1/1 pass, Nextest run
  `968932e4-1040-4974-b338-c1ece7f8e37a`.
- Combined SF1 characterization set: 11/11 pass, Nextest run
  `fee947e1-e1ae-43b1-b130-75be68f11dc1`.
- Full repository Nextest: 3203/3203 pass and one intentional skip, run
  `ad19df87-2ba3-4d29-b423-b928b5794cea`.
- Ignored-test inventory contains exactly
  `kitty_graphics::tests::path_beta_real_host_probe`; ordinary `cargo test`
  proves `1 ignored, 0 failed` without executing the real-host probe.

## Direct `just check` Equivalent

`just` is not installed, so every applicable command in the lowercase
`justfile` `check` recipe was executed directly:

- `cargo fmt --check`: PASS.
- `cargo clippy --all-targets --locked -- -D warnings`: PASS.
- canonical Windows MSVC Clippy with `LIBGHOSTTY_VT_SIMD=false`: PASS.
- integration asset Bun tests: 5/5 PASS.
- plugin marketplace Bun tests: 12/12 PASS.
- Python maintenance modules: 64/64 PASS.
- `git diff --check`: PASS before targeted staging.

No stable Herdr process, inherited socket, installed binary, or user process
was contacted, restarted, or terminated.

## Graph Evidence

The built-in MCP transport became unavailable during fixture diagnosis. No
proxy or user process was restarted. The documented single-worker fallback ran:

```text
CBM_WORKERS=1 codebase-memory-mcp cli index_repository '{"repo_path":"/home/ayaz/projects/herdr","mode":"fast","persistence":true}'
```

It completed with zero extraction errors. The refreshed graph is 19,809 nodes
and 91,610 edges. Exact searches and snippets return both:

- `files_curtain_currently_replaces_terminal_surface` in `src/ui.rs`;
- `miller_layout` in `src/ui/file_manager.rs`.

Freshness was therefore not inferred from `ready` alone.

## Next Gate

Current delivery phase is I7/SF2. The next and only behavior test is
`shell_layout_places_dock_sidebar_stage_without_overlap`. It must compile and
fail for the intended named-region geometry/deserialize assertion. Compile,
setup, filter, fixture, or unrelated failures do not count as RED.
