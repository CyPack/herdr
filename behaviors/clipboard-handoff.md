# Clipboard handoff — bounded wait, no zombies

Fork behaviour registry entry. Every rule below is owned by a named test; if a
future merge drops the behaviour, the named test fails loudly instead of the
feature disappearing silently.

Surface: `src/platform/linux.rs` (`run_clipboard_command*`, `hand_off_clipboard_child`).

## Why this exists (measured incident, 2026-07-29)

`wl-copy` must acquire clipboard ownership from the Wayland compositor *before*
it forks into the background. Normal handoff: **~104 ms** (measured twice, with
herdr's exact argv, stdout/stderr nulled).

Under CPU pressure that handshake stalls. On 2026-07-29 the desktop was
throttled on three levels at once (DisplayLink manager spiking to ~540% against
a 600% `system.slice` cap; the kitty scope holding 116 processes against its own
600% cap from `kitty-.scope.d/10-term-cpu-cap.conf`). A `wl-copy` child then hung
for **2m50s** with `ppid = herdr client`, and the client's main thread sat in
`do_wait` the whole time — **input stopped being processed entirely**. Killing the
child released it (`do_wait` → `futex_do_wait`), which confirmed the chain.

Root cause is not the compositor slowness itself but the unbounded
`child.wait()`: it converts a transient external delay into a permanent lock of
our own event loop. Helpers we do not control must never be able to do that.

## Behaviours

| ID | Behaviour | Owning test |
|---|---|---|
| CB-WAIT-1 | A clipboard helper that never exits does **not** hold the caller; the call returns within the bounded window instead of blocking. | `clipboard_command_does_not_block_on_hanging_helper` |
| CB-WAIT-2 | Past the timeout the copy is reported as **delivered** (`true`), so the caller does not fall through to the X11 helpers and spawn a second, competing clipboard writer. | `clipboard_command_does_not_block_on_hanging_helper` (asserts the return value) |
| CB-WAIT-3 | A fast helper keeps the original semantics: reaped inline, success taken from its exit status. | `clipboard_command_succeeds_for_fast_helper` |
| CB-WAIT-4 | Spawn failure stays falsy so the `wl-copy → xclip → xsel` fallback chain still works. | `clipboard_command_fails_for_missing_program` |
| CB-WAIT-5 | A non-zero exit reports failure rather than a false success. | `clipboard_command_fails_for_nonzero_exit` |
| CB-REAP-1 | A handed-off child is owned by a background reaper thread and collected when it exits — the bounded wait must not trade a UI lock for a process leak. | `handed_off_clipboard_child_is_reaped_not_zombied` |

## Invariant

```
No clipboard helper may block a herdr thread for longer than
CLIPBOARD_HANDOFF_TIMEOUT, and no handed-off helper may outlive its reaper.
```

`CLIPBOARD_HANDOFF_TIMEOUT` = 300 ms — ~3x the measured 104 ms handoff, still
below the point where a copy reads as a UI stall. `CLIPBOARD_POLL_INTERVAL` =
10 ms.

## Deliberately unchanged

- Clipboard semantics, helper selection order, and `ui.copy_on_select` behaviour
  are untouched: this is a liveness fix, not a feature change.
- macOS/Windows paths are not modified; the incident and the measurement are
  Wayland/`wl-copy` specific, and platform code stays isolated per project rules.
- The compositor-side causes (cgroup CPU caps, DisplayLink CPU render) are
  system configuration, tracked outside this repo. This entry only guarantees
  herdr degrades gracefully when they bite.
