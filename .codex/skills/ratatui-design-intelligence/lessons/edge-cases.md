# Edge Cases

| situation | solution | date |
|---|---|---|
| Terminal is smaller than a component's bordered minimum | Degrade or decline visibly; use saturating geometry and never index assumed rows/columns | 2026-07-15 |
| Rounded or braille glyphs are missing/misaligned | Switch the whole component symbol set to plain/ASCII; never mix corner families inside one border | 2026-07-15 |
| Truecolor is unavailable or output is not a TTY | Map semantic tokens to 256/16/no-color profiles and preserve meaning with glyphs/modifiers | 2026-07-15 |
| Overlay opens above another overlay | Push a new focus scope, route input only to the top blocker, and restore the prior valid owner when it closes | 2026-07-15 |
| Tiling drag leaves the terminal or receives resize mid-gesture | Clamp preview to current area; cancel or recompute from the original tree and commit only a valid normalized operation | 2026-07-15 |
| Loading animation is not visible | Use idle tick rate and do not keep the high-frequency animation clock active | 2026-07-15 |
| Reference mutates state during render | Extract state transition into update and geometry/hit projection into `compute_view`; retain render behavior only | 2026-07-15 |
| Strong TUI reference uses OpenTUI or another non-Ratatui stack and has no detected license | Index and classify it as application/behavior intelligence, do not copy code, and re-express patterns through Herdr's pure Ratatui architecture | 2026-07-15 |
| Herdr target worktree is dirty while integration intelligence is generated | Record commit plus relevant dirty-path boundary, treat existing edits as user-owned, map current symbols read-only, and require P14 product-isolation status to match the frozen boundary | 2026-07-15 |
| An approved TDD task sequence names a RED commit but omits the preceding GREEN commit before the next RED slice | Add an atomic GREEN commit after fresh focused and regression evidence so the branch tip never carries an earlier failing contract into the next slice | 2026-07-15 |
