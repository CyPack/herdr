# Machine-readable surfaces — what to ask instead of reading the source

Audience: agents, and people driving them.
Companions: `behaviors/README.md` (what is guaranteed and why),
`docs/patterns/bar-surface-platform.md` (one surface in depth),
`docs/references/README.md` (external sources, with confidence).

This document exists because every surface below **already exists and is gated**, and an agent
that does not know they exist reads five thousand lines of Rust to answer a question the binary
answers in one command. That has happened more than once, measured, in this repository.

Nothing here needs a running server or a live pane unless the table says so.

---

## MR0 · The surfaces, measured 2026-08-17

Each row was run, and its exit code read **without a pipe** — `cmd | head && echo ok` reports
`head`'s status and will tell you a failing command succeeded.

| ask | command / path | answers |
|---|---|---|
| What can a bar config say? | `herdr shell spec` · `--json` | Section kinds, widget kinds, action kinds, secondary presentations, the bar-section menu's rows, colour tokens, bundled pictures with their sizes, and the switches outside `[shell.bars]` — each with the keys it reads, the keys it refuses, and a working example |
| What can the API be asked? | `herdr api schema --json` | JSON Schema for every request and response, plus the wire `protocol` number. Also committed at `docs/next/api/herdr-api.schema.json` |
| What can `config.toml` contain? | `docs/next/website/src/data/config-reference.json` | Every key, its type and its default, generated rather than written |
| Is *this* config valid? | `herdr config check` | The real parser's answer on the real file, with the edge, index and reason for anything it refuses |
| What does this fork guarantee, and why? | `behaviors/*.md` | 26 files. Each row is an id, a behaviour, **the reason it exists**, and the tests that own it. `behavior_registry_check` reported 941 documented rows on 2026-08-17 |
| What is a pane actually showing? | `herdr agent read <pane> --source detection --format text` · `--format ansi` | The buffer a detector sees, which is not the viewport a person scrolls |
| Why did detection decide that? | `herdr agent explain <pane> --json` | Which manifest rules matched, and which did not |

### The one that is easy to miss

`herdr shell spec` is not documentation *about* the grammar; it is generated **from** the
grammar, and every list in it is gated against the parser in both directions. A name the spec
prints is a name this build accepts, and a name this build accepts is a name the spec prints.
Neither statement is true of any prose in this repository, including this file.

---

## MR1 · The gates behind them, and why you can trust the output

A published list is only worth reading if it cannot drift from the thing it describes. Three
gate shapes are used here, and it is worth knowing which one is protecting a given answer:

| gate | what it catches | example |
|---|---|---|
| **Set equality, both directions** | A name in the code and not the document (undiscoverable), and a name in the document and not the code (a refused line somebody was told to write) | the guide's colour table vs the resolver's tokens (TP-CHROME-107) |
| **Executable example** | A documented snippet that no longer parses | every `example` in `shell spec` is fed through the real parser (D70-5) |
| **Function-pointer table** | "The name exists but nothing builds it" | `SectionKind`, `BUILTIN_ART`, `BAR_COLOR_TOKENS` (D70-7) |

### What a gate cannot do, and the trap that follows

⚠ **A gate built on substring matching is not a gate.** `guide.contains(name)` looked like it
protected the picture catalogue for weeks. Measured on the configuration guide: `bar` occurs 81
times, `star` 18, `ring` 14, `line` 11 — nine of fifteen plausible picture names would have
passed without one line of documentation being written. Parse the **table**: find the `|---|`
rule row and read what follows it.

⚠ **A spec generated from a table, compared to that table, measures nothing** and stays green
forever (D-L2-3). A staleness gate must be anchored on something hand-written — the size column,
the prose example, the refusal sentence.

---

## MR2 · The order to ask in

```
1. codebase-mcp        map first — this repo's folders are not its modules (Leiden clustering)
2. herdr shell spec    the vocabulary, before guessing a key name
3. behaviors/*.md      whether a behaviour is guaranteed, and what breaks if it goes
4. herdr config check  the parser's answer on a real file
5. the source          last, and now with a symbol name rather than a search term
```

Reversing this order is not merely slower. `grep` on a 5,000-line module finds the word and not
the decision, and the decision is usually recorded — in a doc comment, a behaviour row, or a
refusal message — somewhere a search for the word will not look.

---

## MR3 · Anti-patterns

| Don't | Do |
|---|---|
| Read the source to learn what a config key accepts | `herdr shell spec` — it is generated from the parser |
| Trust a hand-written capability table | Open the struct. Two rows in `bar-surface-platform.md` were wrong exactly this way |
| Cite a recorded rejection as a finding | Re-measure it. One cost estimate was off by four files and four fixtures |
| Copy a list of names into a document | It ages silently. Name the command that prints it |
| Report `config: ok` as evidence | An empty config says `ok`. Pair every positive with a negative control that is refused |
| Read the exit code through a pipe | `cmd > log 2>&1; echo $?`. `cmd \| tail && echo ok` reads `tail`'s status — it has lied twice here |
| Add a fork behaviour without a registry row | The marker and the row land in the **same commit**; a row without a marker fails the build, on purpose |
| Take a green test as proof a gate works | Mutate the thing it guards and watch it turn red. A test that cannot fail is not a gate |
