# Bar surface as a platform — icons, buttons, and third-party apps

Audience: agents and people who will place, wire and ship things that live in a herdr bar.
Companions: `docs/patterns/bar-icons.md` (how a picture is drawn),
`docs/patterns/custom-layout.md` (how a bar is composed),
`behaviors/surface-chrome.md` (what is guaranteed).

This document exists so nobody starts from scratch again. It records **what already
exists, measured**, what is missing, and the shape the missing parts should take.

---

## SP0 · The measured platform (2026-08-13)

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
| bar section answers a right press | **exists** | `SectionGesture::Secondary`, `action.secondary = "tab"` (`b4d37959`) |
| open a command in a new tab of the current workspace | **exists** | `App::open_argv_in_new_tab` |
| **bar section runs a plugin action** | **MISSING** | no `SectionAction::Plugin` |
| **icon by name rather than raw codepoint** | **MISSING** | config takes a literal grapheme |
| **more than 8 sections in one bar** | **CEILING** | `MAX_BAR_SECTIONS = 8` |

Read that table before proposing anything. Two rows are missing and one is a ceiling; the rest
is plumbing that already works and must be reused rather than reinvented.

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
| `secondary = "tab"` | opens **the same `argv`** in a new tab of the current workspace, full size | right |
| `plugin` | **not built** — invoke a plugin action by id | left |
| `menu` | **not built** — a context menu of the section's own actions | right, once there is more than one |

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
action = { kind = "popup", argv = ["btop"], width = "80%", height = "80%", secondary = "tab" }
```

A `secondary` naming something this build does not know, or sitting on a section with no
command, is refused at config time with its own message — the same treatment a popup size with
no popup gets, and for the same reason: that is the shape a half-finished edit leaves behind.

**Why not a context menu yet.** The fork's existing right-press idiom *is* a context menu
(`ContextMenuKind::AppDock`), and reusing patterns is the house rule. It was rejected for this
layer on measured grounds: a new `ContextMenuKind` variant touches **eleven files** including an
exhaustive invariant arm and four fixtures, and a menu of one item is a click tax. It becomes
the right answer when a section can carry a plugin's whole action list — which is what the
`plugin` row above unlocks.

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
action = { kind = "popup", argv = ["btop"], width = "80%", height = "80%" }
```

Sizing rules worth knowing before you write:

- `fixed` declares its own width and **can be refused at config time** if the picture will
  not fit. That refusal is a gift: it happens where you can fix it.
- `fill` takes what is left and is **never refused** — its width is not known until the
  terminal has a size. It clips instead.
- **At most 8 sections per bar.** A macOS-style toolbar has 10–11 icons; if every icon must
  be separately clickable, this ceiling is a real design constraint and needs a decision,
  not a workaround.

### Assigning behaviour

Today, one shape: `action = { kind = "popup", argv = [...] }`. Everything else in SP1.b is
unbuilt. When they land they must keep this property: **an action is declarative data**, never
a script. A bar that could run arbitrary shell from a config file is a bar that cannot safely
accept a third-party section.

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
  4. WIRE IT         SectionAction::Plugin { id, action }        ❌ MISSING  ← the bridge
```

Only step 4 is new, and it is small. Everything else is reuse.

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
- **The 8-section ceiling.** A store implies many apps; a bar holds eight. Either the ceiling
  moves, or a section becomes a container of several icons whose clicks are resolved by
  position within it. These are different architectures and the choice belongs to whoever
  owns the roadmap.

---

## SP4 · Anti-patterns

| Don't | Do |
|---|---|
| Design a new install/registry mechanism | `PluginLink` and the manifest already exist — SP0 |
| Put shell in a section's action | Declarative action data; a config file must not be executable |
| Let a section run a plugin by shelling out to a CLI | The bridge is `PluginActionInvoke`, in-process |
| Give the right press its own `argv` | `secondary` names a presentation; two commands in one section drift, and the person pressing cannot tell which they got |
| Trust a capability table without opening the struct | `Method::TabCreate` was listed as "the wiring is already there" and cannot run a command at all |
| Write a screen detector from what you expect the product to draw | Derive it from a dump. A popup-frame detector looking for `╭` reported "no popup" while `┌ popup` sat in the middle of the screen |
| Add a ninth section | The bar is refused entirely — it does not truncate |
| Ship an icon-font glyph with no fallback | A machine without the font shows tofu, which reads as broken |
| Prove a bar works from a state test | Read the cells, and read their **colour** (TP-CHROME-55) |
| Read a counter inside `render` | The loop samples; the widget formats. SP1.a.i |
| Draw an empty bar for a pool you cannot read | Draw nothing — an empty bar is a claim |
| Demonstrate a meter by filling **swap** | Swap thrashing can lock a desktop for minutes with no user-side undo. Move **CPU** instead: it responds in seconds and `kill` reverses it instantly |
