# Fork setup — getting the file manager features to actually appear

This fork adds a native file manager with picture, PDF and spreadsheet
previews, and hands files to editors and viewers. Everything here is compiled
into the binary, but three of the features stay invisible until the environment
around them is set up. All three cost one command each, and all three have
already cost someone an afternoon of thinking the build was broken.

Read this before concluding that a feature is missing.

---

## 1. Pictures and PDFs need one config flag

Preview images are drawn with the kitty graphics protocol, which herdr keeps
behind an experimental flag that is **off by default**. With it off the preview
panel draws no picture at all and says `set experimental.kitty_graphics`.

Add to `~/.config/herdr/config.toml` (create the file if it is not there):

```toml
[experimental]
kitty_graphics = true
```

Your terminal has to support the protocol. Ghostty and kitty do; most others
do not, and there the flag changes nothing.

## 2. Config changes reach the SERVER, not the client

herdr runs a server that owns the sessions and a client that draws them.
Closing the terminal window and reopening it restarts the **client**, which
reattaches to the same server that is already running — with the same config it
read when it started. This is the single most confusing part of the setup:
everything looks restarted and nothing changed.

After editing `config.toml`:

```bash
herdr server reload-config     # applies it live, keeps every session
```

`herdr server stop` also works, but it kills whatever is running inside the
panes, which is rarely what you want.

Verify the reload was accepted — it answers with a status:

```json
{"result":{"diagnostics":[],"status":"applied","type":"config_reload"}}
```

## 3. "Open in New Tab", "Edit Here" and the preview click are plugins

Opening a file from the file manager — a spreadsheet in the editor, a picture
or PDF in its own tab, text in an editor floating over the panel — is done by
**local plugins**, not by the binary. Without them the right-click menu simply
has fewer entries and clicking a preview does nothing.

The plugins live outside this repository. Link each one:

```bash
herdr plugin link /path/to/herdr-plugins/sheets
herdr plugin link /path/to/herdr-plugins/edit
```

Two things worth knowing:

- The registry stores a **snapshot** of each manifest taken at link time. Edit a
  manifest and nothing changes until you link it again.
- The registry lives beside the config, so it is scoped per profile. A debug
  build reads `herdr-dev`, a release build reads `herdr`. A plugin linked under
  one is invisible to the other.

Check what the running server actually has:

```bash
herdr plugin action list
```

Without the plugins you still get every preview in the panel and the built-in
enlarge viewer (`Enter`, or click the picture). Only "open it somewhere else"
is missing.

---

## Quick verification

```bash
herdr view --help                     # the viewer command exists in this build
herdr plugin action list              # plugin actions the server can offer
grep -A1 '\[experimental\]' ~/.config/herdr/config.toml
```

Then, in the file manager: select a `.png`. If the panel says
`set experimental.kitty_graphics`, step 1 or step 2 is still outstanding. If it
draws the picture but right-clicking offers nothing to open it with, step 3 is.

## If a feature still looks missing

Check that the binary you are running is the one you built. A stale binary
still on `PATH` is indistinguishable from a missing feature:

```bash
sha256sum "$(command -v herdr)" target/release/herdr
```

The two must match. They will not if the server was started before the install:

```bash
ps -o pid,lstart,cmd -p "$(pgrep -f 'herdr server' | head -1)"
```

A server older than the binary is running the code it started with.
