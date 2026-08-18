# Bar surface as a platform — icons, buttons, and third-party apps

Audience: agents and people who will place, wire and ship things that live in a herdr bar.
Companions: `docs/patterns/bar-icons.md` (how a picture is drawn),
`docs/patterns/custom-layout.md` (how a bar is composed),
`behaviors/surface-chrome.md` (what is guaranteed).

This document exists so nobody starts from scratch again. It records **what already
exists, measured**, what is missing, and the shape the missing parts should take.

---

## SP0 · The measured platform (2026-08-17)

The app-store-shaped question — *"do we build this from zero?"* — is answered **no**. herdr
already carries a plugin system with an install path, a manifest, declared actions, and pane
ownership. What is missing is the **bridge** from a plugin to a bar section.

| capability | state | evidence |
|---|---|---|
| install / remove an app | **exists** | `Method::PluginLink`, `PluginUnlink` |
| enumerate, enable, disable | **exists** | `PluginList`, `PluginEnable`, `PluginDisable` |
| an app declares actions, and they can be invoked | **exists** | `PluginActionList`, `PluginActionInvoke` |
| an app opens / focuses / closes a pane | **exists** | `PluginPaneOpen`, `PluginPaneFocus`, `PluginPaneClose` |
| managed install directory | **exists** | `src/plugin_paths.rs::managed_plugins_dir()` |
| apply config changes without restart | **exists** | `server.reload_config` |
| open something in a new tab | **exists** | `Method::TabCreate` |
| bar section draws a picture | **exists** | `SectionWidget::Icon` / `Art` (`e1eaa92a`) |
| bar section opens a popup on left click | **exists** | `SectionAction::OpenPopup` (`5771dfc6`, `f8df7485`) |
| bar section answers a right press | **exists** | `SectionGesture::Secondary` (`b4d37959`) |
| a right press offers a **menu** of presentations | **exists** | `ContextMenuKind::BarSection`, `action.secondary = "menu"` — the default |
| open a command in a new tab of the current workspace | **exists** | `App::open_argv_in_new_tab`, `secondary = "tab"` |
| open a command **beside the focused pane** | **exists** | `App::open_argv_in_split`, `secondary = "split"` |
| bar section **runs a command and opens nothing** | **exists** | `SectionAction::Run`, `action = { kind = "run", argv = […] }` |
| bar section **goes to a workspace by name** | **exists** | `SectionAction::FocusWorkspace`, `action = { kind = "workspace", name = "…" }` |
| bar section shows **the time** | **exists** | `SectionWidget::Clock`, `widget = { kind = "clock", format = "%H:%M" }` |
| bar section shows **disk / battery / net / temp** | **exists** | `ResourceMetric` table; only the metrics a section shows are read |
| bar section runs a plugin action | **exists** | `SectionAction::InvokePlugin`, `action = { kind = "plugin", command = "…" }` |
| the grammar can be **read without running it** | **exists** | `herdr shell spec [--json]` — kinds, keys, refusals, colours, pictures, menu rows |
| **icon by name rather than raw codepoint** | **MISSING** | `glyph` still takes a literal grapheme; `art` takes a name, an icon-font symbol does not |
| more than one popup at a time | **MISSING** | `BarSectionClick::PopupAlreadyOpen`; the menu greys the row instead |
| more than 8 sections in one bar | **exists** | `shell.bars.<edge>.max_sections`, default 8, up to 16 (`267c8496`) |

Read that table before proposing anything. Two rows are missing; the rest is plumbing that
already works and must be reused rather than reinvented.

⭐ **Everything in this table is in the installed binary** as of 2026-08-17 19:39 — verified by
running `herdr shell spec` against it, not by reading the source. SP3.5 is about the link after
that one.

> ⚠ **This table is a claim about the code, and it has been wrong twice.** Both times the
> failure was the same: a row written from what the surrounding design implied rather than
> from the struct. Before you trust a row, `search_graph` the symbol it names.

> ⚠ One correction worth carrying: an earlier draft of this document said the right press was
> "just wiring" because `Method::TabCreate` already existed. Measured 2026-08-14 — it does not
> carry a command at all (`workspace_id`, `cwd`, `focus`, `label`, `env` only), so it opens an
> empty shell tab. The primitive that runs something is `Workspace::create_tab_argv_command`.
> A capability table is only as good as the last time somebody opened the struct.

---

## SP1 · Catalogue — what a section can BE

A section is one address. Two independent questions are answered about it: **what it shows**
and **what a press on it does**. They are deliberately separate — a picture with no action is
a status indicator, an action with no picture is a hit area, and both are legitimate.

### SP1.a · What it shows (`widget.kind`)

| kind | draws | costs | use it for |
|---|---|---|---|
| *(absent)* | nothing | 0 | a spacer that still swallows clicks |
| `label` | text, clipped by display width | one line | a word, a count, a static tag |
| `icon` + `glyph` | one icon-font grapheme | one cell | toolbar / status symbols (needs a Nerd Font) |
| `icon` + `art` | a bundled picture in half-blocks | W cells × ⌈H/2⌉ rows | a brand mark, multi-colour |
| `icon` + `pixels` | the same, written inline | same | a one-off mark, or a third-party's |
| `resource` | a live machine counter, as a figure | one line | cpu / mem / swap, when the exact number matters |
| `meter` | the same counter as a filled coloured bar | every row of the section | cpu / mem / swap, when "is this a problem" matters |

### SP1.a.i · The cost contract every live widget signs

`resource` and `meter` both show something that changes. Neither of them reads
anything. The sample arrives already taken, on the loop's clock, and the widget
only formats it — which is what makes "the renderer never samples" a property of
the code rather than a promise in a comment.

Three consequences you can rely on, and must preserve if you add a live widget:

1. **Nothing is read unless something on screen asks.** With no live section
   configured, no deadline is scheduled and `/proc` is never opened at all.
2. **The screen follows the data's clock**, not the other way round. A new
   sample is the only reason these widgets redraw, so an idle machine costs
   nothing beyond one reading every two seconds.
3. **An unchanged widget is free in the frame diff.** This is measurable, and it
   is measured: `drawing_the_same_picture_twice_produces_an_identical_buffer_so_the_diff_sends_nothing`.

A live widget that reached for a clock, a counter, or an allocation-ordered map
inside `render` would look perfectly correct on screen and would resend its
whole region on every frame. That failure has no other detector.

### SP1.a.ii · Reading a meter

A bar fills left to right in **eighths of a cell** — whole cells are `█`, the
cell after them carries the remainder as `▏▎▍▌▋▊▉`. Whole-cell steps would hide
every change under one-tenth on a ten-cell bar and then lurch; the eighth-block
glyphs cost the same single cell.

The colour is a **threshold, not a ramp**: green below 60%, yellow below 85%,
red above. Three answers are easier to read in three cells than a gradient, and
a person looks at a meter to decide whether something is a problem.

A metric with **no ratio draws nothing at all** — not an empty bar. An empty bar
claims "plenty free", and about an unreadable counter or a machine with no swap
that claim is false, in exactly the way a fabricated `0%` would be.

### SP1.b · What a press does (`action.kind`)

| kind | today | gesture |
|---|---|---|
| *(absent)* | inert — consumes the press so it cannot leak to the surface behind | left |
| `popup` | spawns `argv` in a floating pane, at `width`/`height` | left |
| `plugin` | invokes `command = "<plugin-id>.<action-id>"` in-process, through the same resolver a keybind uses | left |

And what the *right* press does, on a `popup` section, by `action.secondary`:

| `action.secondary` | today | note |
|---|---|---|
| *(absent)* | same as `"menu"` | the default changed on 2026-08-17; it used to be inert |
| `menu` | opens `ContextMenuKind::BarSection` at the pointer: **Open in popup**, **Open in new tab**, **Open in split** | the popup row is disabled while a popup is open |
| `tab` | opens **the same `argv`** in a new tab of the current workspace, full size | unchanged, and deliberately: a file written before the menu existed must keep acting, not start asking |
| `split` | opens **the same `argv`** beside the focused pane | horizontal, matching every other launcher in the product |
| `none` | nothing — but the press is still consumed | the old default, now something a file can say rather than only rely on |

A `plugin` action answers **one** gesture. It carries no `argv`, no `width`, no `height` and no
`secondary`, and each of those is refused by name rather than ignored: a plugin's command line and
where it appears both come from its own manifest, so every one of those fields would be a setting
nothing ever reads. The right press stays inert — the bar cannot re-present something it does not
open.

The spelling is not new. It is the one a keybind already uses, deliberately reused whole:

```toml
# a key, which already worked
[[keys.command]]
key = "prefix+p"
type = "plugin_action"
command = "jt.command-palette.open"

# a bar section, which now works the same way
[[shell.bars.top.sections]]
kind   = "fixed"
cells  = 3
widget = { kind = "icon", glyph = "" }
action = { kind = "plugin", command = "jt.command-palette.open" }
```

Nothing checks that the plugin exists while the config is read, and that is deliberate: a plugin
can be installed after the config naming it was written, and refusing the line at read time would
forbid the icon of an app somebody has not downloaded yet. An id that resolves to nothing reports
itself as a toast when pressed, with the resolver's own reason — "not found" and "disabled" stay
different messages, because they need different answers from the person reading them.

**The gesture rule that must hold:** left is *the* action, right is *choice about* the action.
That is the convention every desktop the user compared us against follows, and breaking it
makes a bar feel wrong for reasons people cannot name.

`secondary` is what makes that rule literally true rather than a slogan. It names a
**presentation**, not a command — so the right press *cannot* run a different program, only
show the same one differently. Three shapes were weighed and rejected before it: a second
action table (writes `argv` twice, and the two drift), a boolean (nowhere to grow), and an
array of gesture-tagged actions (verbose for the case that dominates, and still duplicates the
command).

```toml
action = { kind = "popup", argv = ["btop"], width = "97%", height = "92%", secondary = "tab" }
```

A `secondary` naming something this build does not know, or sitting on a section with no
command, is refused at config time with its own message — the same treatment a popup size with
no popup gets, and for the same reason: that is the shape a half-finished edit leaves behind.

**Why the context menu is here now** (2026-08-17). It was rejected once, on the grounds that a
new `ContextMenuKind` variant touches "eleven files including an exhaustive invariant arm and
four fixtures", and that a menu of one item is a click tax. Two of those three were wrong when
measured, and the third stopped being true:

| the estimate | what building it actually cost |
|---|---|
| eleven files | **seven source files**: `state.rs` (variant, rows, `item_enabled`, invariant arm), `input/shell.rs` (the decision), `input/mod.rs` (opening it), `input/modal.rs` (the picks), `tabs.rs` (`open_argv_in_split`), `ui/shell/source.rs` (the grammar), `ui/shell/spec.rs` (publishing it) — plus two behaviour-registry files and two documents |
| four fixtures | **none.** No fixture in this repository enumerates `ContextMenuKind` |
| an exhaustive invariant arm | **true**, and it cost four lines: a bar section menu carries a command line rather than an index, so it has no identity that can go stale and nothing to assert |
| a menu of one item is a click tax | **true, and no longer the situation.** Three presentations exist now, and the enum being closed is what turned "should we?" into a cost the compiler could count |

The lesson worth carrying is not "menus are cheap". It is that **a rejection recorded with an
estimate has to be re-measured before it is cited**, because the estimate ages exactly like the
capability table above it — and this one was cited for three days as though it were a finding.

The other rejected reason aged the same way. `SecondaryPresentation` used to say, in the code,
that a split presentation "needs a target pane, and a bar has no idea which pane the person
meant". That was true the day it was written. It stopped being true when `open_argv_in_new_tab`
started resolving the focused pane to borrow its directory: the bar's answer to *which pane* is
now the same answer the pane menu's own "Split right" gives.

---

## SP2 · Guide — placing a section and wiring it

### Placement

A bar divides along its long axis: `top`/`bottom` across the width, `left`/`right` down the
height. A section takes its place from its **position in the array** — the index is its only
stable name, which is why a refused section leaves the whole bar undivided rather than
renumbering its neighbours.

```toml
[shell.bars.top]
enabled = true
size    = 5          # border 2 + content 3; a 6-pixel-tall mark needs 3 rows
border  = true
color   = "mauve"

[[shell.bars.top.sections]]   # index 0 — leftmost
kind   = "fixed"              # fixed | fill | weighted (see custom-layout.md)
cells  = 10
widget = { kind = "icon", art = "herd" }
action = { kind = "popup", argv = ["btop"], width = "97%", height = "92%" }
```

Sizing rules worth knowing before you write:

- `fixed` declares its own width and **can be refused at config time** if the picture will
  not fit. That refusal is a gift: it happens where you can fix it.
- `fill` takes what is left and is **never refused** — its width is not known until the
  terminal has a size. It clips instead.
- `content` asks for **at least `min` and at most `max`**, taking as much of what is left as
  fits between them. Both default to zero, and an unwritten `max` used to be honoured
  literally — "at most zero cells" — so a section naming only its widget parsed cleanly,
  raised no diagnostic, and drew **nothing**. `herdr shell spec` published ten widget
  examples in exactly that shape. Since 2026-08-18 an unwritten `max` means unbounded, the
  same way an unwritten `fill` weight means one: the simplest section must not be the one
  that needs the most typing (TP-CHROME-124). Write both bounds when you want a bound;
  write neither when you want the content to size itself.
- **`max_sections` is the bar's budget**, default 8, accepted from 1 to 16. It used to be a
  hard 8 borrowed from the pane splitter — on the reasoning that dividing a bar and splitting a
  screen are the same question. They are not, and a macOS-style toolbar of 10–11 icons was the
  proof. Raising it is measured, not free: `BarSections` and `BarSectionRects` grow from 66 to
  130 bytes each, so all four bars cost 512 bytes more inside the geometry cache key.
- A budget outside `1..=16` is **refused, never clamped** — a file saying forty beside a build
  doing sixteen is a file its next reader will believe.

### Assigning behaviour

A popup's `width`/`height` are a share of the **pane area**, not of the terminal, so the
sidebar is already subtracted before the percentage applies. Measured 2026-08-18 on a
120-column screen: `width = "80%"` left btop 69 usable columns and it drew
`Terminal size too small: Width = 69 · Needed = 80` instead of a process list. The examples
here ask for `97%`/`92%` because a full-screen tool wants nearly the whole pane; a small
status popup does not. Size the popup to what you are launching, and check it by launching it.

Two shapes: `action = { kind = "popup", argv = [...] }` and
`action = { kind = "plugin", command = "…" }`. A `popup` section additionally chooses what its
right press does, through `action.secondary` — see the second table in SP1.b.

Whatever lands next must keep this property: **an action is declarative data**, never a script.
A bar that could run arbitrary shell from a config file is a bar that cannot safely accept a
third-party section. The menu does not weaken this: it offers presentations of the command the
config already named, never a command of its own.

You do not have to read this file to learn the vocabulary. `herdr shell spec` prints every
accepted name — section kinds, widget kinds, action kinds, secondary presentations, the menu's
rows, the colour tokens, the bundled pictures and their sizes — and `--json` makes it
machine-readable. Each of those lists is gated against the parser in both directions, so the
spec cannot advertise a name the build refuses or hide one it accepts.

### Adding a custom icon

Three ways, exactly one per section (writing two is refused, because which one wins would be
invisible in the file):

```toml
widget = { kind = "icon", glyph = "󰂯" }              # icon font, 1 cell
widget = { kind = "icon", art = "herd" }             # bundled picture
widget = { kind = "icon", palette = { a = "mauve" }, pixels = ["..aa..", ".aaaa."] }
```

The pixel form is the one a third party ships. It is text, so it diffs, reviews and signs;
it is inert, so accepting it is not accepting code; and its cell footprint is known before
it is drawn, so a bar budget can be computed rather than discovered.

Full rules — palette keys, transparency, odd row counts, refusal messages — are in
`docs/patterns/bar-icons.md` §BI3.

---

## SP3 · The app-store shape

The user's scenario, verbatim: *"ilerde lazım olan senaryo bir app indirilecek ikonlu veya
ikonsuz şu bölüme şunun yanına konulsun denilecek — ya agent yapacak kolayca ya da configten
ayarlanabilecek."*

That decomposes into four things, and three of them already exist.

```
  1. GET IT          PluginLink                                  ✅ exists
  2. DESCRIBE IT     plugin manifest + PluginActionList          ✅ exists
  3. PLACE IT        [[shell.bars.<edge>.sections]] + reload     ✅ exists
  4. WIRE IT         SectionAction::InvokePlugin { action }      ✅ exists  (2026-08-14)
```

All four steps exist. This block said step 4 was missing for three days after it shipped, while
the capability table twenty lines above already recorded it — the same document disagreeing with
itself, which is what a "what is missing" list does when it is written once and read many times.

What a plugin section still cannot do is offer **its own** action list from the right press: the
menu built here presents one command in three places, and a plugin action is not a command this
layer knows how to re-present (`SectionGesture::Secondary` on a plugin action is deliberately
inert). That is the next bridge, and the menu is the surface it will land on.

**Why an external store is viable.** A third-party contribution is two inert files: a plugin
manifest (already a defined format) and, optionally, an icon written as pixel rows. Neither
executes at parse time. An agent placing one writes a TOML table and calls
`server.reload_config`. Nothing about that path needs a bespoke store protocol — a store is
then just an index of manifests, which can live entirely outside this repository.

**What must be decided before it ships** (these are choices, not unknowns):

- **Trust boundary.** A plugin's action runs a command. Accepting one from a store is
  accepting code, even though the *icon* is inert. The vetting bar is
  `skill-mcp-vetting`'s, not the icon format's — do not let the icon's safety argument leak
  onto the plugin's.
- **Namespacing.** Two apps both want to be called `git`. Section ids and plugin ids need a
  collision rule before a store exists, not after.
- **The section budget.** A store implies many apps, and `max_sections` is now the number that
  decides how much of somebody's bar they may take. It is the user's, not the store's: a
  plugin that wants a section asks against a limit that already existed rather than one
  invented under pressure.

---

## SP3.5 · Getting it onto somebody's screen

Everything above is about what a bar *can* be. This section is about the gap that closed last
and cost the most: what a bar *is*, on the machine of the person who asked for one.

The chain has six links and the last one is the one that gets skipped:

```
  source merged  →  artefact built  →  artefact INSTALLED  →  process RESTARTED
                                                                    ↓
                                            the person's CONFIG names a bar
                                                                    ↓
                                                   the person SEES it
```

Measured on 2026-08-17, after the whole bar platform had landed: the first four links were
green — `~/.local/bin/herdr` carried every feature, the live process had been restarted onto
it, and `herdr shell spec` listed `clock`, `disk`, `battery`, `net`, `temp`, `run`,
`workspace`, the menu and `split`. The fifth link was `grep -c "shell.bars" ~/.config/herdr/config.toml`
→ **0**. Ten commits, thirty killed mutations and roughly five thousand green tests had changed
nothing about what anybody was looking at.

**Checking each link, in the product's own terms:**

```bash
# 3. what is installed, and when
ls -l --time-style=+%F_%H:%M "$(command -v herdr)"

# 4. what the live process is actually running from, and how old it is
pgrep -af 'herdr server' | head -1
ls -l /proc/<pid>/exe          # "(deleted)" means the file moved under a running process
ps -o pid,lstart,etime -p <pid>

# 5. which features that binary carries — the only claim that cannot be faked
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH herdr shell spec

# 6. whether the person's own config asks for any of it
grep -c "shell.bars" ~/.config/herdr/config.toml
```

**Two rules that fall out of this.**

A new binary does not reach a running herdr. The process keeps executing the file it started
with, so installing and restarting are separate steps and the restart belongs to the person
whose session it is. `herdr update` is not the way to install a fork build — it replaces it
with an upstream release.

And a config that loads is not a config that is seen. `herdr config check` answering `ok`
proves the file parses; an empty file answers `ok` too. The last link is measured by the person
looking at their own bar, and by nothing else.

### SP3.5.a · The seventh link — a reload that reaches the running client

The chain above has a link nobody drew until it cost three rounds: **the config on disk
reaching the client that is already running.** Editing the file and reloading is not the same
as starting with it.

Measured 2026-08-18, against a correct config on disk that `config check` accepted and a
reload that answered `"status":"applied"`:

| what was changed and reloaded | reached the screen |
|---|---|
| a section's `action` (a click target) | ❌ |
| a section's `max` (its width) | ❌ |
| `shell.resource_interval_ms` | ✅ |

`App::new` built three structures out of `[shell.bars]` — the bar set, its colours, and the
chrome that answers presses — and `apply_live_config` rebuilt none of them; it read one field
out of `config.shell`. The bar drew correctly the whole time, because it was drawing the config
the client had **started** with. Both reload paths behaved identically: the CLI
(`herdr server reload-config`) and the TUI shortcut (`prefix+shift+r`).

Fixed by TP-CHROME-127: a reload now rebuilds all three together, and the session facts sharing
that aggregate — panel width, fold state, presented template — are replaced in place rather
than rebuilt, because those belong to the session file rather than to `config.toml`.

**The rule this leaves.** When a config change does not show up, ask *which* of the seven links
is open before touching the setting again:

```bash
# is it on disk?
grep -A4 'shell.bars' ~/.config/herdr/config.toml
# does it parse?
env -u HERDR_SOCKET_PATH -u HERDR_CLIENT_SOCKET_PATH herdr config check ; echo "exit=$?"
# did the running client take it? — the only honest test is the product itself:
#   change something VISIBLE (a section's `max`, a label's text), reload, and look.
#   A reload reporting "applied" is the server's answer, not the screen's.
```

## SP4 · Anti-patterns

| Don't | Do |
|---|---|
| Design a new install/registry mechanism | `PluginLink` and the manifest already exist — SP0 |
| Put shell in a section's action | Declarative action data; a config file must not be executable |
| Let a section run a plugin by shelling out to a CLI | The bridge is `PluginActionInvoke`, in-process |
| Give the right press its own `argv` | `secondary` names a presentation; two commands in one section drift, and the person pressing cannot tell which they got |
| Trust a capability table without opening the struct | `Method::TabCreate` was listed as "the wiring is already there" and cannot run a command at all |
| Cite a recorded rejection without re-measuring it | The context menu was refused on an eleven-file, four-fixture estimate. Measured: seven source files, zero fixtures |
| Believe a code comment about what is impossible | `SecondaryPresentation` said a split "needs a target pane, and a bar has no idea which pane"; the bar had been resolving the focused pane for its own `cwd` the whole time |
| Guard a documented name with `guide.contains(name)` | The guide is prose: `bar` occurs 81 times in it. Parse the **table**, compare as a set, in both directions |
| Write a screen detector from what you expect the product to draw | Derive it from a dump. A popup-frame detector looking for `╭` reported "no popup" while `┌ popup` sat in the middle of the screen |
| Add a ninth section | The bar is refused entirely — it does not truncate |
| Ship an icon-font glyph with no fallback | A machine without the font shows tofu, which reads as broken |
| Prove a bar works from a state test | Read the cells, and read their **colour** (TP-CHROME-55) |
| Read a counter inside `render` | The loop samples; the widget formats. SP1.a.i |
| Draw an empty bar for a pool you cannot read | Draw nothing — an empty bar is a claim |
| Demonstrate a meter by filling **swap** | Swap thrashing can lock a desktop for minutes with no user-side undo. Move **CPU** instead: it responds in seconds and `kill` reverses it instantly |
