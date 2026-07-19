# Files Rapid Navigation Root-Cause Evidence

## Decision

The 2026-07-19 isolated profile identifies two defects:

1. Primary: Trail directory activation performs synchronous filesystem and
   preview work on the serial server input loop, including for an exact
   resident ancestor.
2. Amplifier: the client combines stdin, resize, and every server message in
   one FIFO and performs blocking socket/output work in the same consumer.

This is not mouse-byte loss and not transfer of file bodies or “hundreds of
GB” to the client.

## Reproducible Evidence

Archive:

```text
.local/perf/files-layout-v1/20260719-165338-run
```

Checkpoint:

```text
5378d2b09bfd6e3407c2dde7c2526b86d10ecf5b
```

The client and server logs each contain 7,411 `raw input event parsed` rows.
After stripping timestamps, all 7,411 byte/event signatures are exactly equal
and ordered.

Pair-by-index latency from the client main-loop parse to server parse:

| percentile | latency |
|---|---:|
| p50 | 9.872 ms |
| p95 | 7,587.645 ms |
| p99 | 9,303.526 ms |
| max | 9,867.817 ms |

Large server-log silences start directly after left mouse-down dispatch.
Examples are 9.945 s, 9.886 s, 9.837 s, 9.230 s, 8.024 s, and 7.024 s.

Across 312.762 seconds:

- 1,699 filesystem reads;
- 4,158 sent full frames;
- 500,967,844 serialized full-frame bytes;
- zero `full_render.queue_full`;
- one 10.910-second window contains 62 filesystem reads while full render
  averages 16.4 ms and peaks at 18.9 ms.

## Exact Blocking Chain

```text
Mouse Down(Left)
  -> App::handle_file_manager_row_mouse
  -> App::activate_trail_row
  -> FmState::activate_trail_entry
  -> TrailSnapshots::activate_entry
  -> TrailSnapshots::select_dir
  -> read_directory_snapshot(child)
  -> FmState::install_trail_operation_projection
  -> directory_is_writable(owner)
  -> read_parent_context(owner)
  -> prepare_preview(selected child)
  -> read directory child again
```

Every listing reads names and metadata only. It does not recursively read file
bodies. Text/image detail content is a separate bounded preview concern.

## Accepted Architecture

- Resident exact child: rebranch from the loaded snapshot with zero
  filesystem/body reads.
- Nonresident child: typed Trail activation on the existing bounded
  one-executing/one-latest-pending FM worker.
- Completion: exact Files generation, source state, Trail column/index/path,
  show-hidden, and latest worker generation validation.
- Client: lossless input lane, ordered control lane, latest-only semantic-frame
  mailbox, and bounded fairness.
- Terminal ANSI deltas, graphics, clipboard, shutdown, title, mouse capture,
  and input-source messages remain lossless and ordered.
- Directory streaming/cache is a separate measured scale feature, not the
  primary lag fix.

## Primary References

- Yazi performance architecture:
  https://yazi-rs.github.io/blog/why-is-yazi-fast/
- Yazi v26.5.6 source:
  https://github.com/sxyazi/yazi/tree/v26.5.6
- Ranger bounded loader:
  https://github.com/ranger/ranger/blob/master/ranger/core/loader.py
- Broot:
  https://github.com/Canop/broot

## Safety

The archived run used the cleanup-first/cleanup-last isolated helper. Stable
Herdr, its socket, existing Ghostty windows, browsers, editors, upstream, and
the user-owned `.superpowers/` tree were untouched.
