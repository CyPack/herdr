# Files Preview Capability Test Points

Date: 2026-07-18

| Input class | Default capability | Explicit supported provider | Why |
|---|---|---|---|
| directory | unsupported: Trail owns it | never delegated | one navigation authority |
| UTF-8 text/source/config | bounded native text | unchanged | fast, deterministic fallback |
| Markdown | bounded native text | optional plugin + native-text fallback | rich render is expert workflow |
| recognized image | native image | unchanged | existing generation-bound Kitty path |
| PDF/office | metadata-only | optional plugin + metadata fallback | no parser/process in native render |
| archive | metadata-only | optional plugin + metadata fallback | no implicit extraction |
| audio/video | metadata-only | optional plugin + metadata fallback | no native decode/playback |
| generic binary | metadata-only | none in P4 | never misread NUL content as text |
| broken symlink/special | unsupported with reason | never delegated | fail closed |
| non-UTF-8/control path | unsupported with reason | never delegated | no lossy or unsafe identity |
| oversized UTF-8 text | bounded native text, `truncated=true` | unchanged | existing hard read ceiling |
| missing/unsupported provider | native/metadata fallback | no dispatch | deterministic offline behavior |

The classifier consumes only prepared kind, exact path name/extension, and an
injected provider set. It performs no filesystem/config/PATH lookup, process
spawn, socket access, or navigation mutation.

## Closure

- TDD: `4c87a18f` RED, `ea75a269` GREEN.
- Readable metadata fallback: `b61e719b`.
- VIS-14 oracle/baseline: `5d917b5d` / `b79b55f6`.
- Rust: 3,526/3,526 PASS, 2 skipped.
- Linux and Windows clippy: clean.
- Playwright Chromium: 22/22 PASS; Trail + mutation subset 9/9.
- VIS-14 was visually inspected: exact PDF selection, preserved Trail columns,
  and full `metadata only` / `optional document viewer` status are visible.
