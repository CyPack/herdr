# Measurement discipline — why the tool's answer is not the answer

Audience: anyone, human or agent, about to write "this works" in this repository.
Companions: `docs/patterns/machine-readable-surfaces.md` (what to ask instead of reading),
`behaviors/README.md` (what is guaranteed, and why), `docs/patterns/rust-engineering.md`.

Every entry below is something that actually happened here, with the measurement that exposed
it. None of them is hypothetical, and none was caught by being careful — each was caught by a
*second* measurement disagreeing with the first.

> ```
> THE OUTPUT OF AN INSTRUMENT — OR OF A JUDGEMENT —
> IS NOT EVIDENCE THAT YOU ASKED THE RIGHT QUESTION.
> ```

---

## MD1 · The instrument's silence is not a finding

Four separate times in one week, silence looked like good news:

| what was run | what it reported | what was true |
|---|---|---|
| `cargo nextest … \| tail -40` | exit **0** | **two tests were red** — the pipeline reported `tail`'s status |
| `$E XDG_CONFIG_HOME=… herdr config check` | empty output | exit **127**: zsh does not word-split a variable, so `$E` was one command name |
| a mutation harness run | `SURVIVED` | the filter never selected the test that guards it — **nothing was measured** |
| `grep -A6 "action kinds"` | two action kinds | there were **four** — the window cut the answer in half |

The fix is never "be careful". It is a second, differently-shaped measurement:

```bash
cmd > /tmp/log 2>&1; echo "exit=$?"      # a status, read without a pipe
sed -n '/^section/,/^next section/p'     # a range, not a fixed window
grep -c "^        FAIL" /tmp/log         # a count, not an impression
```

## MD2 · A gate built on prose is not a gate

`guide.contains(name)` looked like it protected the configuration guide for weeks. Measured on
that file:

| name | occurrences | inside |
|---|---:|---|
| `ram` | 11 | **program** |
| `mem` | 13 | **remember**, **implement** |
| `bar` | 81 | ordinary English in a manual about bars |
| `temp` | 5 | **template** |
| `star` · `ring` · `line` | 18 · 14 · 11 | ordinary English |

Nine of fifteen plausible picture names, every metric name and the one alias passed that check
**with no line written about them** — and would have gone on passing after the thing was
deleted from the build.

**What to do instead.** Anchor on a sentence, find the table's `|---|` rule row, treat what
follows as data, and compare as a **set in both directions**:

```rust
let documented = guide_table_after("There are seven metrics")   // rule row → data
    .into_iter().map(|columns| columns[0].clone())
    .collect::<BTreeSet<_>>();
let built = ResourceMetric::accepted().into_iter().collect::<BTreeSet<_>>();
assert_eq!(documented, built);      // both directions, in one line
```

Each direction fails differently and both fail silently: a name the build accepts and the guide
omits is one nobody will ever write; a name the guide offers and the build refuses is worse,
because somebody read it here and their config now reports an error against a documented line.

⚠ Do not drop the header row by asking "does this look like data" — this repo's own picture
table has `` `art` `` as its header cell, backticks and all.

## MD3 · A decision with no guard is not a decision

Mutation testing found two of these in one session:

- A bar section splits **horizontally**, matching every other launcher in the product. Turning
  the direction round broke **nothing**: every assertion counted panes, and a pane below is
  exactly as much a pane as one beside.
- An action with nothing to act on is **refused**. Removing the refusal broke nothing, because
  the only thing checking it was a product-level probe nobody had turned into a test.

A green suite says the decisions you *did* pin are pinned. It says nothing about the ones you
only wrote in a comment.

## MD4 · The mutation probe must be proven too

`SURVIVED` has three meanings and only one of them is a finding:

```
a) the gate really does not bite          → FINDING
b) the mutation never applied             → measurement error
c) it applied, but the filter did not select the guarding test  → measurement error
```

A harness therefore has to: refuse a dirty tree (its `git checkout` revert would delete
uncommitted tests), report `PROBE DID NOT APPLY` rather than counting a silent pass, verify the
mutated text is really in the file, and — when something survives — make you question the
**filter** before the code.

## MD5 · Evidence comes from the layer of the complaint

| the complaint | sufficient evidence | NOT sufficient |
|---|---|---|
| "the bar shows nothing" | run it, look at the screen | a green unit test |
| "this config is refused" | `herdr config check`, exit code read without a pipe | the parser's unit test |
| "the feature is live" | `herdr shell spec` from the **installed** binary | the source being merged |

And the control half is load-bearing: `config: ok` is what an **empty** config says too. Pair
every positive with a negative that is refused on purpose, or "everything passed" and "nothing
was tested" produce the same output.

## MD6 · The loop that renders is the headless server

Added to `App::handle_scheduled_tasks` alone, a periodic task passes every test, ships inside
the binary, and never runs — because the server owns the state the screen is drawn from. This
has now happened **twice**: once with the resource sampler (TP-RES-11) and once with the clock
(TP-CLOCK-12). The parity guard caught both. Do not weaken
`scheduler_parity_headless_vs_monolithic`; extend it.

## MD7 · A recorded rejection ages like a capability table

Three claims in this repository's own documents were false when re-measured:

| written | measured |
|---|---|
| "a new `ContextMenuKind` touches eleven files and four fixtures" | seven source files, **zero** fixtures |
| "a split needs a target pane, and a bar has no idea which pane" | the bar had been resolving the focused pane for its own `cwd` the whole time |
| "step 4 — WIRE IT — **MISSING**" | the capability table twenty lines above already said `exists`, and had for three days |

Before citing a rejection, re-measure it. An estimate ages exactly like the table above it, and
a document can contradict itself for as long as nobody reads both halves at once.

## MD8 · "Shipped" is not "in use"

The most expensive gap is the last one. In one session: ten commits landed, thirty mutations
were killed, ~5,170 tests were green, the binary was built and installed, the live process was
restarted onto it — and the person's screen was **unchanged**, because their config had never
been told the feature existed.

```
herdr shell spec  →  clock, disk, battery, net, temp, run, workspace, menu, split
grep -c "shell.bars" ~/.config/herdr/config.toml  →  0
```

A success that is not delivered is indistinguishable from work that was never done. Walk the
whole chain and measure each link — source, artefact, **installed** artefact, running process,
and finally the thing the person actually looks at.

---

## MD9 · Anti-patterns

| Don't | Do |
|---|---|
| Read an exit code through a pipe | `cmd > log 2>&1; echo $?` |
| Trust a fixed `-A`/`-B` window to have shown you everything | Read the range: `sed -n '/^from/,/^to/p'` |
| Guard a documented name with `contains` | Parse the table, compare as a set, both ways |
| Take a green test as proof a gate works | Mutate what it guards and watch it turn red |
| Take `SURVIVED` as a finding | Check the filter and the probe first |
| Cite a recorded estimate as a fact | Re-measure it; estimates age |
| Say "cannot be done" without measuring | It was said five times in one session and was wrong five times |
| Report `config: ok` as evidence | An empty config says it too — pair it with a refusal |
| Add a periodic task to the monolithic loop only | The headless server is the loop that renders |
| Call it finished when the tests are green | Walk the delivery chain to the person's screen |
