# Miller mtime dependency audit

Date: 2026-07-19

## Decision

Declare the already-locked package directly:

```toml
time = { version = "=0.3.47", features = ["local-offset"] }
```

Do not enable `formatting`, `parsing`, `macros`, or `serde`. The implementation
uses integer getters and manual bounded labels.

## Existing graph

`cargo tree -i time@0.3.47 --locked` reports:

```text
time v0.3.47
└── ratatui-widgets v0.3.0
    └── ratatui v0.30.0
        └── herdr v0.7.3
```

The feature graph already enables `std`, `alloc`, and `local-offset` through
`ratatui-widgets`. Therefore the direct declaration creates no new package and
no new feature activation. Cargo records direct root dependencies in the
lockfile, so the expected lock delta is exactly one `"time"` line in the
existing `herdr` package dependency array. Any version, checksum, or package
record delta is a failed gate.

## Local source and platform behavior

The installed `time-0.3.47` source proves:

- `OffsetDateTime: From<SystemTime>`;
- `UtcOffset::local_offset_at(datetime)` is the fallible local-offset seam;
- `OffsetDateTime::to_offset(offset)` exposes `year`, `month`, `day`,
  `weekday`, `hour`, `minute`, and `to_julian_day`;
- Unix uses the thread-safe `localtime_r` path and platform `tm_gmtoff`;
- Windows uses `SystemTimeToTzSpecificLocalTime`;
- the crate declares `rust-version = "1.88"`; the repository toolchain is
  newer;
- license is `MIT OR Apache-2.0`.

The implementation resolves the offset separately at each timestamp, rather
than applying the anchor offset to every historical entry. That preserves
daylight-saving boundaries.

## Failure policy

Filesystem mtime and calendar conversion are separate facts:

- `symlink_metadata(path).modified()` failure yields `modified = None`;
- a known `SystemTime` always retains its chronological sort authority;
- local-offset failure yields `Unknown Date` and `—`, never a fabricated local
  label;
- render receives only prepared group and label data and performs no clock,
  timezone, or filesystem calls.

## Advisory evidence

- RustSec `RUSTSEC-2026-0009` affects versions earlier than `0.3.47` and lists
  `>=0.3.47` as patched.
- RustSec `RUSTSEC-2020-0071` affects historical `0.1` and `0.2` ranges, not
  `0.3.47`.

Primary references:

- https://rustsec.org/advisories/RUSTSEC-2026-0009.html
- https://rustsec.org/advisories/RUSTSEC-2020-0071.html
- https://docs.rs/time/0.3.47/time/struct.UtcOffset.html

## Verification gates

```bash
export PATH="$HOME/.local/bin:$PATH"
cargo tree -i time@0.3.47 --locked
cargo tree -e features -i time@0.3.47 --locked
git diff -- Cargo.lock
LIBGHOSTTY_VT_SIMD=false cargo clippy --bin herdr --target x86_64-pc-windows-msvc --locked -- -D warnings
```

Expected:

- one exact `time v0.3.47`;
- no package/feature growth beyond the existing graph;
- exactly one root dependency-list lockfile line and no package/version delta;
- Windows target lint is clean.
