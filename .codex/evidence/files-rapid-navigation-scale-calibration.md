# Files Rapid Navigation Scale Calibration

Date: 2026-07-19
Probe commit: `631f213f`
Branch: `feat/native-fm`

## Decision

`FMP-4D` does not authorize partial directory listing, a directory cache, or
viewport-first enrichment. The calibrated 100,000-entry fixture stays within
the predeclared final-snapshot and retained-metadata budgets in the slower
debug profile. The measured interaction defect is therefore not caused by
recursive file bodies or directory payload size.

The production architecture remains:

1. render only prepared metadata snapshots;
2. reuse resident Trail snapshots without filesystem work;
3. perform nonresident listing on the bounded one-executing /
   one-latest-pending worker;
4. reject stale completion by Files generation, source, column, index, and
   exact path;
5. prioritize client input over replaceable semantic frames while keeping
   terminal deltas and control messages lossless.

## Fixed Fixture And Budgets

- filesystem: `/tmp` on `tmpfs`, 4 KiB block size;
- entries: 100,000 regular files;
- large-file control: four sparse files at 256 GiB each, 1 TiB logical total;
- listing payload: immediate-child name, path, kind, and mtime only;
- build profile: Rust debug/test;
- warm-up: one complete snapshot;
- measured samples: five complete snapshots;
- final sorted snapshot p95 budget: 2,000 ms;
- retained metadata lower-bound budget: 64 MiB;
- cleanup: isolated `TempDir`, recursively removed on probe completion.

## Fresh Result

Command:

```bash
cargo test --bin herdr \
  fm::tests::fmp_scale_100k_directory_snapshot_meets_reference_budget \
  --locked -- --exact --ignored --nocapture
```

Output:

```text
fixture_ms=1428
samples_ms=[763, 764, 781, 788, 802]
p95_ms=802
entries=100000
logical_large_file_bytes=1099511627776
retained_metadata_lower_bound_bytes=14800000
```

The p95 used 40.1% of the time budget. The retained metadata lower bound used
22.1% of the byte budget. Because the debug build passed, release-only timing
was not needed to avoid a false GREEN.

## Host Conditions

- Linux `7.1.3-200.fc44.x86_64`;
- AMD Ryzen 9 5900HX, 8 cores / 16 threads;
- 15 GiB RAM;
- Rust `1.96.1`;
- cargo-nextest `0.9.140`;
- `/tmp` free before the probe: 7.4 GiB and 1,037,230 inodes.

## Body-Read Rejection

The four sparse entries contribute 1 TiB of logical file contents, but
`read_directory_snapshot` stores no body bytes. It enumerates immediate
children, obtains symlink metadata for kind/mtime, and sorts `FileEntry`
metadata. The probe completes within the same bounded snapshot budget, so
logical file size does not affect listing delivery.

## Reopen Condition

Reopen `FMP-4D` only when a reproducible target filesystem exceeds either
budget or a separate deterministic test proves unacceptable time-to-first-row.
At that point the next design must use typed `Start/Part/Done` tickets,
cancellation, stable final ordering, and byte/entry-bounded cache eviction.
