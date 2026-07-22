# Files Locations Follow Performance Evidence

Date: 2026-07-22
Branch: `feat/native-fm`
Pre-commit base: `b3d86779`
Scope: FLF Task 6, fixed-label root lifecycle telemetry and host calibration

## Decision

Locations Follow remains on the existing bounded file-manager I/O lane. The
fresh structural and release evidence does not authorize a second worker,
debounce timer, directory cache, LRU, partial listing, or pinned-location
pre-warm:

1. vertical Rail movement and root dispatch stay synchronous and cheap;
2. root enumeration remains off the scheduled input/render thread;
3. one executing plus one latest pending request bounds burst work;
4. a 100-event blocked burst processes only its first and final targets;
5. scheduled result polling continues throughout a deterministic 500 ms worker
   hold;
6. exact lifecycle counters use seven static, path-free labels.

The 10k and 100k enumeration numbers below are host observations, not portable
CI limits. A future regression requires diagnosis against a comparable host and
filesystem; it does not authorize weakening a structural assertion.

## Fixed Counter Contract

| Label | Meaning |
|---|---|
| `fm.locations.root.submitted` | A Root request was accepted by the bounded worker lane. |
| `fm.locations.root.replaced` | That Root submission replaced the single pending work slot. |
| `fm.locations.root.processed` | The production Root processor began handling the request. |
| `fm.locations.root.enumeration` | Root preparation/enumeration was attempted off the scheduled thread. |
| `fm.locations.root.accepted` | An exact authoritative prepared root was installed and accepted. |
| `fm.locations.root.failed` | An authoritative Root error, panic, disconnect, or destination-focus failure was surfaced. |
| `fm.locations.root.stale` | A non-latest or lifecycle-invalid Root result was discarded. |

All labels are `&'static str` constants. Tests inspect only the
`fm.locations.root.*` subset and require the exact expected label set; target
paths never participate in metric identity. The existing profiler independently
caps each metric kind at 128 labels.

The test profiler is intentionally thread-local. Therefore `processed` and
`enumeration` are tested through synchronous `process_request`, while
submit/replace/accept/fail/stale are observed on their owning caller thread.
The opt-in runtime profiler remains process-global and sees worker-thread
events during live use.

## TDD Evidence

Assertion-level RED:

```text
cargo nextest run --locked -E 'test(/flf_profiler/)' \
  --no-fail-fast --status-level fail --final-status-level fail \
  --failure-output final --success-output never

Nextest run ID: be552631-32a3-4f31-86d3-17d68703349c
Result: 0 passed, 2 failed, 3667 skipped
Failure: expected processed=1, observed 0
Failure: expected submitted=3, observed 0
```

First GREEN after adding the seven counters:

```text
Nextest run ID: 5f1ca15b-c8ec-4633-b079-22ede515455c
Result: 2 passed, 0 failed, 3667 skipped
```

Final structural filter after the calibration and nanosecond reporting change:

```text
cargo nextest run --locked \
  -E 'test(/flf_blocked_|flf_root_panic|flf_worker_disconnect|flf_profiler/)' \
  --no-fail-fast --status-level fail --final-status-level fail \
  --failure-output final --success-output never

Nextest run ID: e06ddbaa-a736-41b1-9299-b4fce8c518e6
Result: 6 passed, 0 failed, 3664 skipped
```

This final filter covers exact fixed labels, accepted/error/stale application,
pending replacement, first/final burst processing, loop responsiveness,
processor-panic lane reuse, and one-shot disconnect recovery.

## Release Calibration Method

Command:

```text
cargo nextest run --release --locked --run-ignored only \
  -E 'test(flf_scale_locations_follow_navigation)' \
  --status-level all --final-status-level slow \
  --failure-output immediate-final --success-output immediate
```

Final run:

```text
Nextest run ID: 92eecb11-95d7-4767-9497-0682f21d3c9a
Result: 1 passed, 0 failed, 3669 skipped
Test duration: 6.803 s
Build profile: release (optimized)
```

The ignored probe created only test-owned paths beneath its `TempDir` in
`/tmp`:

- small flat root: 32 regular files;
- medium flat root: 10,000 regular files;
- large flat root: 100,000 regular files;
- dispatch samples: 25 per root, interleaved while one worker request was held;
- enumeration samples: one warm-up plus five measured full roots per size;
- burst proof: 100 empty synthetic roots with the first processor held;
- continuity hold: 500 ms, polling the scheduled result path every ~1 ms.

The gate-release guard opens a held worker even if the ignored test unwinds, so
the worker `Drop` path cannot remain blocked during failure cleanup.

## Host And Filesystem

```text
kernel: Linux 7.1.3-200.fc44.x86_64 x86_64
cpu: AMD Ryzen 9 5900HX, 8 cores / 16 threads
rustc: 1.96.1 (31fca3adb 2026-06-26)
cargo-nextest: 0.9.140 (a9fef2964 2026-07-05)
/tmp: tmpfs, 7.6 GiB total, 7.1 GiB available before probe
/tmp inodes: 1,048,576 total, 1,027,415 available before probe
```

No user Home, Desktop, or Downloads directory was read or used as a fixture.

## Final Measurements

Fixture creation:

| Root | Entries | Creation |
|---|---:|---:|
| small | 32 | 401 us |
| medium | 10,000 | 119,766 us |
| large | 100,000 | 1,155,242 us |

Bounded-lane dispatch, measured in nanoseconds to avoid false zeroes:

| Root | p50 | p95 | max |
|---|---:|---:|---:|
| small | 731 ns | 851 ns | 1,242 ns |
| medium | 731 ns | 811 ns | 1,443 ns |
| large | 721 ns | 802 ns | 1,072 ns |

Full prepared-root enumeration after one warm-up:

| Root | p50 | p95 | max |
|---|---:|---:|---:|
| small | 216 us | 223 us | 223 us |
| medium | 63,766 us | 66,061 us | 66,061 us |
| large | 616,271 us | 657,574 us | 657,574 us |

Blocked burst and settle:

```text
events staged: 100/100
targets actually processed: 2 (first and final only)
hold observation: 500,694 us
scheduled loop polls: 473
render-neutral/no-result polls: 473
maximum observed poll gap: 1,091 us
release-to-final-settle: 1,472 us
final target exact: true
final focus Rail-owned: true
Linux VmRSS after settle: 96,708 KiB
```

The first release run used microseconds for dispatch and rounded sub-microsecond
samples to zero. It still passed structurally (`cdc28a47-6e62-4c26-987c-6ea6203d9f83`),
but the report was corrected to nanoseconds and rerun rather than presenting
zero as useful timing evidence.

## Cleanup And Runtime Safety

After each release probe:

- no `/tmp/herdr-fcl-io-*` directory remained;
- `/tmp` returned to 7.1 GiB available and 1,027,416 free inodes;
- no Herdr server or client was launched by the automated calibration;
- stable Herdr, its socket, and `~/.config/herdr` were not targeted.

The ignored local helper `.local/herdr-files-v1-profile.sh` was updated but is
not staged. Fresh static checks verified:

```text
bash -n: pass
git check-ignore: /.local/ rule confirmed
kill/pkill/killall tokens: absent
ownership marker: present
semantic isolated server stop: present
open-socket deletion refusal: present
stable_socket_touched=false evidence field: present
normal run exit automatically calls report_latest: present
```

The live user command remains one line:

```text
cd /home/ayaz/projects/herdr && HERDR_RENDER_PROF=1 ./.local/herdr-files-v1-profile.sh run
```

Live acceptance is deliberately not claimed here; it belongs to FLF Task 7
and requires the user to exercise the interactive terminal UI.

## Static Gates

```text
cargo fmt --all -- --check: pass
cargo clippy --all-targets --all-features --locked -- -D warnings: pass
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --locked \
  --target x86_64-pc-windows-msvc -- -D warnings: pass
git diff --check: pass
```

`just` was not installed in the active shell, so `just windows-lint` returned
127 before running. The `windows-lint` recipe was read from the checked-in
lowercase `justfile`, its already-installed MSVC target was confirmed, and the
recipe's clippy command above was executed verbatim. This is a tool-runner
availability issue, not a skipped Windows gate.

## Confidence And Reopen Conditions

- Structural bounded/latest-wins verdict: **high confidence** (deterministic).
- Fixed-label and no-path-label verdict: **high confidence** (type plus tests).
- Host timing values: **high confidence for this host/run**, not portable.
- Interactive UX/runtime verdict: **pending Task 7 live acceptance**.

Reopen architecture work only with fresh evidence of one of these conditions:

1. dispatch blocks the scheduled thread;
2. more than first/final work is processed under the deterministic burst;
3. an old completion wins any lifecycle/identity axis;
4. a comparable-host enumeration regression is reproducible;
5. live isolated profiling shows loop or render storms despite bounded I/O.

Pinned Home/Desktop/Downloads pre-warm remains the separate FMN-6 measurement
track and is not implied or authorized by these results.
