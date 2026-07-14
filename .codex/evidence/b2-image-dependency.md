# B2 Image Dependency Decision — 2026-07-14

## Decision

Use `image 0.25.10` with default features disabled and only the common preview
formats enabled when TP-B2.1 first requires the production decoder:

```toml
image = { version = "0.25.10", default-features = false, features = ["png", "jpeg", "gif", "webp"] }
```

Keep the existing direct `png 0.17.16` dependency unchanged. It backs the
established production `ghostty::decode_png_rgba` path. Consolidating that
decoder onto `png 0.18` is a separate behavior-migration concern and is not
required for B2.

## Alternatives Measured

| Candidate | Format scope | Exact additional lock packages | Existing package upgrades | Decision |
|-----------|--------------|--------------------------------|---------------------------|----------|
| Existing `png 0.17.16` only | PNG | 0 | 0 | Rejected: no JPEG/GIF/WebP and no shared orientation/resize API |
| `image 0.25.10`, PNG only | PNG | 5 | 0 | Rejected: five packages still leave the user-facing format gap |
| `image 0.25.10`, common formats | PNG/JPEG/GIF/WebP | 12 | 0 | Selected: bounded common-format coverage without default-format bloat |
| `image 0.25.10`, defaults | Broad defaults plus rayon/AVIF/EXR/TIFF and others | 78 | 0 | Rejected: unnecessary compile, security, and platform surface |

The selected lock delta is:

- `byteorder-lite 0.1.0`
- `color_quant 1.1.0`
- `gif 0.14.2`
- `image 0.25.10`
- `image-webp 0.2.4`
- `moxcms 0.8.1`
- `png 0.18.1`
- `pxfm 0.1.30`
- `quick-error 2.0.1`
- `weezl 0.1.12`
- `zune-core 0.5.1`
- `zune-jpeg 0.5.15`

No selected package contains a build script or proc macro. License metadata is
MIT, Apache-2.0, BSD-3-Clause, Unlicense, or Zlib compatible combinations.

## Security and Platform Evidence

- Package-registry advisory queries returned no advisory for 11 of the 12
  selected packages. The two `image` advisories affect only `<0.23.12` and
  `>=0.10.2,<0.21.3`; neither affects `0.25.10`.
- A clean `cargo +1.96.1 check --locked --target x86_64-pc-windows-msvc` for
  the selected feature set passed.
- `image 0.25.10` declares Rust `1.88.0`; Herdr pins Rust `1.96.1`.

## Compile-Cost Evidence

Three clean-target `cargo +1.96.1 check --locked` samples were taken for each
non-default `image` candidate after registry/source download:

| Candidate | Median wall time | Median max RSS | One-shot target bytes |
|-----------|------------------|----------------|-----------------------|
| PNG only | 6.46 s | 274,344 KiB | 21,372,360 |
| PNG/JPEG/GIF/WebP | 5.92 s | 274,528 KiB | 23,804,232 |

Wall-time variance is not treated as a speed claim. The reliable cost delta is
seven more lock packages and about 2.43 MB more clean check artifacts for
common-format coverage; RSS was effectively unchanged.

## Mandatory Decode Limits

`image::Limits` makes width and height strict, but documents `max_alloc` as a
best-effort limit that some decoders may ignore. TP-B2.1 therefore must enforce
all of these independently before full decode or placement allocation:

1. bounded encoded input bytes;
2. strict decoder width and height;
3. checked width × height pixel count;
4. checked decoder `total_bytes()`;
5. bounded RGBA output bytes;
6. bounded aspect-fit target dimensions and placement bytes.

Decoder work remains outside render. Unsupported, corrupt, truncated,
oversized, zero-area, and arithmetic-overflow inputs must return explicit
failures without panic.
