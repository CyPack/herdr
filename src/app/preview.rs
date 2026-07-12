//! Preview↔tab sync: click-bridge binding polling (runtime layer).
//!
//! The click-bridge stack wires a browser tab to the Claude Code session that
//! launched it (see [`crate::preview_bindings`]). This module keeps an
//! in-state mirror of those bindings fresh with a cheap fingerprint-gated
//! poll, so the runtime can answer "does this tab have a wired preview?"
//! without ever touching the filesystem on an interaction path.

use std::time::Instant;

/// How often the bindings file is re-checked. Each check is one `stat()`;
/// file contents are only read when the fingerprint actually changed.
const PREVIEW_BINDINGS_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

/// Cheap change fingerprint for the bindings file: (mtime, len). The file is
/// append-only, so its length grows on every write and breaks same-second
/// mtime ties. `None` means "file absent" — itself a comparable state, so
/// deletion is detected too.
fn bindings_file_fingerprint(path: &std::path::Path) -> Option<(std::time::SystemTime, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    Some((meta.modified().ok()?, meta.len()))
}

/// Repeated focus events for the same wired token inside this window trigger
/// no second action: spam-clicking a tab yields exactly one window action.
const PREVIEW_SHOW_DEBOUNCE: std::time::Duration = std::time::Duration::from_secs(1);

/// The dev-browser's DevTools endpoint (click-bridge convention).
const PREVIEW_CDP_HOST: &str = "127.0.0.1:9222";

/// One DevTools target (the subset of `/json/list` output the plan needs).
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
struct CdpTarget {
    #[serde(default)]
    id: String,
    #[serde(default, rename = "type")]
    kind: String,
    #[serde(default)]
    url: String,
}

/// What the show action should do — decided PURELY from the target list, so
/// the idempotency core is unit-testable without any browser.
#[derive(Debug, PartialEq, Eq)]
enum PreviewShowPlan {
    /// A tab on the preview's origin already exists: activate it, never spawn.
    Activate(String),
    /// No such tab: launch via dev-browser (which itself reuses the window).
    Launch,
}

/// `scheme://host:port` of a URL. Paths, queries and fragments change as the
/// user navigates, so a preview tab is identified by its origin alone. Dev
/// URLs always carry an explicit port, so no default-port normalization.
fn url_origin(url: &str) -> Option<&str> {
    let scheme_end = url.find("://")? + 3;
    let rest = url.get(scheme_end..)?;
    let authority_len = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    if authority_len == 0 {
        return None; // "http:///x" — an empty authority is malformed
    }
    url.get(..scheme_end + authority_len)
}

/// Decide the show action from the DevTools target list: a real page tab on
/// the preview's origin is activated (never spawned again); anything else
/// launches dev-browser, which itself opens a tab in the existing window
/// when CDP is alive — the second idempotency line.
fn preview_show_plan(targets: &[CdpTarget], preview_url: &str) -> PreviewShowPlan {
    let Some(origin) = url_origin(preview_url) else {
        return PreviewShowPlan::Launch;
    };
    targets
        .iter()
        .find(|t| t.kind == "page" && url_origin(&t.url) == Some(origin))
        .map(|t| PreviewShowPlan::Activate(t.id.clone()))
        .unwrap_or(PreviewShowPlan::Launch)
}

/// Blocking `curl` GET (the repo's HTTP idiom — see `update.rs`): background
/// threads only, never the event loop. `None` on any failure.
fn curl_get(url: &str) -> Option<String> {
    let output = std::process::Command::new("curl")
        .args(["-fsS", "--max-time", "2", url])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// All DevTools page targets, empty when CDP is down (→ Launch path).
fn cdp_page_targets(host: &str) -> Vec<CdpTarget> {
    let Some(body) = curl_get(&format!("http://{host}/json/list")) else {
        return Vec::new();
    };
    serde_json::from_str(&body).unwrap_or_default()
}

/// Bring one DevTools target's tab to the front of its window.
fn cdp_activate(host: &str, target_id: &str) -> bool {
    curl_get(&format!("http://{host}/json/activate/{target_id}")).is_some()
}

/// Launch the preview via click-bridge's dev-browser launcher, which records
/// a fresh session binding and reuses the existing window when CDP is alive.
fn launch_preview(url: &str) {
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        tracing::warn!("preview launch skipped: HOME is unset");
        return;
    };
    let script = home.join("projects/click-bridge/tools/dev-browser.sh");
    if !script.exists() {
        tracing::warn!(script = %script.display(), "preview launch skipped: dev-browser.sh not found");
        return;
    }
    match std::process::Command::new("bash")
        .arg(&script)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(_) => tracing::debug!(url, "preview launch dispatched via dev-browser"),
        Err(err) => tracing::warn!(?err, url, "preview launch failed to spawn"),
    }
}

/// The background show action: probe CDP, then activate-or-launch, then
/// bring the dev-browser window forward. Every step is best-effort — a dead
/// browser or missing desktop tooling must never disturb the app.
fn preview_show_worker(token: &str, url: &str) {
    let targets = cdp_page_targets(PREVIEW_CDP_HOST);
    match preview_show_plan(&targets, url) {
        PreviewShowPlan::Activate(target_id) => {
            if cdp_activate(PREVIEW_CDP_HOST, &target_id) {
                tracing::debug!(
                    token,
                    url,
                    target = target_id.as_str(),
                    "preview tab activated"
                );
            } else {
                tracing::warn!(
                    token,
                    url,
                    target = target_id.as_str(),
                    "preview tab activation failed"
                );
            }
        }
        PreviewShowPlan::Launch => launch_preview(url),
    }
    raise_preview_window();
}

// ── window raise + placement (GNOME Wayland, best-effort) ──────────────────
//
// Chromium cannot position itself under Wayland, so the raise goes through
// the window-calls-extended shell extension (List/FocusID over D-Bus) and the
// one-time half-screen placement through GNOME's native tiling shortcut.

/// dev-browser's dedicated profile directory: its main chromium process is
/// found by this marker, and shell windows carry that process's pid.
const DEVBROWSER_PROFILE_MARKER: &str = "cc-dev-browser-profile";
const WINDOWS_EXT_DEST: &str = "org.gnome.Shell";
const WINDOWS_EXT_PATH: &str = "/org/gnome/Shell/Extensions/WindowsExt";
const WINDOWS_EXT_IFACE: &str = "org.gnome.Shell.Extensions.WindowsExt";

/// One shell window from `WindowsExt.List` (only the fields we match on).
#[derive(Debug, serde::Deserialize)]
struct ShellWindow {
    #[serde(default)]
    class: String,
    #[serde(default)]
    pid: i64,
    #[serde(default)]
    id: u64,
}

/// `gdbus call` prints a GVariant tuple — `("<json with \" escapes>",)` —
/// not raw JSON. Cut the array out of the tuple and unescape it.
fn extract_gdbus_json_array(raw: &str) -> Option<String> {
    let start = raw.find('[')?;
    let end = raw.rfind(']')?;
    if end <= start {
        return None;
    }
    // GVariant string escaping inside the printed tuple: `\"` and `\\`.
    Some(raw[start..=end].replace("\\\"", "\"").replace("\\\\", "\\"))
}

/// The dev-browser's shell window id: matched by the browser MAIN pid (shell
/// windows carry the main chromium process pid) with a defensive class check
/// — this desktop runs several unrelated chromium windows whose classes even
/// differ in case ("Chromium-browser" vs "chromium-browser").
fn devbrowser_window_id(list_json: &str, browser_pid: u32) -> Option<u64> {
    let windows: Vec<ShellWindow> = serde_json::from_str(list_json).ok()?;
    windows
        .iter()
        .find(|window| {
            window.pid == i64::from(browser_pid)
                && window.class.to_ascii_lowercase().contains("chromium")
        })
        .map(|window| window.id)
}

/// The dev-browser's main chromium pid (`pgrep -o` = oldest matching process,
/// i.e. the parent that owns the windows), or None when it is not running.
fn devbrowser_main_pid() -> Option<u32> {
    let output = std::process::Command::new("pgrep")
        .args(["-of", DEVBROWSER_PROFILE_MARKER])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

/// One `WindowsExt` D-Bus call via `gdbus` (repo idiom: subprocess over new
/// dependencies); background threads only. None on any failure.
fn windows_ext_call(method: &str, args: &[String]) -> Option<String> {
    let mut command = std::process::Command::new("gdbus");
    command.args([
        "call",
        "--session",
        "--dest",
        WINDOWS_EXT_DEST,
        "--object-path",
        WINDOWS_EXT_PATH,
        "--method",
        &format!("{WINDOWS_EXT_IFACE}.{method}"),
    ]);
    for arg in args {
        command.arg(arg);
    }
    let output = command.output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Bring the dev-browser window to the front and (once per window) tile it
/// to the right half. The launch path needs the retry loop: a fresh window
/// takes a moment to appear in the shell's list.
fn raise_preview_window() {
    let Some(browser_pid) = devbrowser_main_pid() else {
        tracing::debug!("no dev-browser process; nothing to raise");
        return;
    };
    for _ in 0..10 {
        if let Some(window_id) = windows_ext_call("List", &[])
            .and_then(|raw| extract_gdbus_json_array(&raw))
            .and_then(|json| devbrowser_window_id(&json, browser_pid))
        {
            if windows_ext_call("FocusID", &[window_id.to_string()]).is_some() {
                tracing::debug!(window = window_id, "dev-browser window raised");
                place_half_screen_right_once(window_id);
            } else {
                tracing::warn!(window = window_id, "dev-browser window raise failed");
            }
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    tracing::debug!(
        browser_pid,
        "dev-browser window did not appear in the shell list"
    );
}

/// Tile the freshly raised window to the right half via GNOME's native
/// shortcut (Super+Right through ydotool) — ONCE per window id per run:
/// repeating Super+Right on an already-tiled window pushes it across
/// monitors on multi-monitor setups.
fn place_half_screen_right_once(window_id: u64) {
    static PLACED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<u64>>> =
        std::sync::OnceLock::new();
    let placed = PLACED.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    match placed.lock() {
        Ok(mut set) => {
            if !set.insert(window_id) {
                return; // already placed this run
            }
        }
        Err(_) => return, // poisoned lock: skip placement, never panic
    }
    // KEY_LEFTMETA=125, KEY_RIGHT=106 (press/release pairs).
    match std::process::Command::new("ydotool")
        .args(["key", "125:1", "106:1", "106:0", "125:0"])
        .output()
    {
        Ok(output) if output.status.success() => {
            tracing::debug!(window = window_id, "preview window tiled right");
        }
        Ok(output) => tracing::debug!(
            window = window_id,
            status = ?output.status,
            "ydotool tiling skipped/failed (daemon down?)"
        ),
        Err(err) => tracing::debug!(?err, "ydotool unavailable; placement skipped"),
    }
}

impl super::App {
    /// Keep `AppState::preview_bindings` in sync with the click-bridge binding
    /// log. Unlike the Projects poll this is NOT gated on sidebar visibility:
    /// preview wiring must resolve on tab focus even while the sidebar is
    /// collapsed or on another tab. Returns whether state changed.
    pub(crate) fn refresh_preview_bindings_if_due(&mut self, now: Instant) -> bool {
        if self.next_preview_bindings_poll.is_some_and(|due| now < due) {
            return false;
        }
        self.next_preview_bindings_poll = Some(now + PREVIEW_BINDINGS_POLL_INTERVAL);
        let Some(path) = crate::preview_bindings::default_bindings_path() else {
            return false;
        };
        self.refresh_preview_bindings_at(&path)
    }

    /// Fingerprint-gated refresh against `path` (injected for tests).
    fn refresh_preview_bindings_at(&mut self, path: &std::path::Path) -> bool {
        let fingerprint = bindings_file_fingerprint(path);
        if fingerprint == self.preview_bindings_fingerprint {
            return false;
        }
        self.preview_bindings_fingerprint = fingerprint;
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let fresh = crate::preview_bindings::read_preview_bindings(path, now_epoch);
        if fresh == self.state.preview_bindings {
            // The file changed but the merged live view didn't (e.g. an
            // expired or redundant line was appended) — no re-render needed.
            return false;
        }
        self.state.preview_bindings = fresh;
        true
    }

    /// Resolve the preview binding wired to one tab, newest binding first
    /// (a session that opened two previews gets its latest URL).
    ///
    /// Match passes per binding, cheapest first:
    /// 1. bound session prefix — the hook truncates the Claude session id to
    ///    12 chars (`cut -c1-12`), so this is a prefix match against the
    ///    tab's full `resumed_session_id`, and prefixes shorter than 12 are
    ///    malformed records that must never match;
    /// 2. the binding's claude pid IS one of the tab's pane child pids
    ///    (chats spawned by the Projects tab: claude is the pane child);
    /// 3. the binding's claude pid is a MEMBER of a pane child's process
    ///    session (a claude started by hand inside a shell tab).
    ///
    /// Pass 3 scans the process table (`platform::session_processes`), so it
    /// is computed lazily at most once per call — and callers must stay off
    /// the render/input path (consume-side or background only).
    pub(crate) fn preview_binding_for_tab(
        &self,
        ws_idx: usize,
        tab_idx: usize,
    ) -> Option<crate::preview_bindings::PreviewBinding> {
        let tab = self.state.workspaces.get(ws_idx)?.tabs.get(tab_idx)?;
        if self.state.preview_bindings.is_empty() {
            return None;
        }

        // Pane child pids via the runtime registry: memory reads only.
        let pane_pids: Vec<u32> = tab
            .panes
            .values()
            .filter_map(|pane| self.terminal_runtimes.get(&pane.attached_terminal_id))
            .filter_map(|runtime| runtime.child_pid())
            .collect();

        let mut session_members: Option<std::collections::HashSet<u32>> = None;

        for binding in &self.state.preview_bindings {
            if let (Some(prefix), Some(session_id)) =
                (&binding.session_prefix, &tab.resumed_session_id)
            {
                if prefix.len() >= 12 && session_id.starts_with(prefix.as_str()) {
                    return Some(binding.clone());
                }
            }
            let Some(pid) = binding.claude_pid else {
                continue;
            };
            if pane_pids.contains(&pid) {
                return Some(binding.clone());
            }
            if pane_pids.is_empty() {
                continue;
            }
            let members = session_members.get_or_insert_with(|| {
                pane_pids
                    .iter()
                    .flat_map(|pane_pid| crate::platform::session_processes(*pane_pid))
                    .collect()
            });
            if members.contains(&pid) {
                return Some(binding.clone());
            }
        }
        None
    }

    /// Consume a queued preview-show request (armed by every tab-focus
    /// change). The ACTIVE tab is resolved at consume time — no indices are
    /// stored, so the request can never go stale. Returns whether a preview
    /// dispatch happened; the action is an external window, so callers never
    /// need a re-render for it.
    pub(crate) fn handle_preview_show_request(&mut self) -> bool {
        self.handle_preview_show_request_with(Self::dispatch_preview_show)
    }

    /// Dispatch-injected core (tests record the dispatch instead of touching
    /// a real browser).
    fn handle_preview_show_request_with(
        &mut self,
        dispatch: impl FnOnce(&mut Self, crate::preview_bindings::PreviewBinding),
    ) -> bool {
        if !self.state.request_preview_show {
            return false;
        }
        self.state.request_preview_show = false;
        let Some(ws_idx) = self.state.active else {
            return false;
        };
        let Some(tab_idx) = self
            .state
            .workspaces
            .get(ws_idx)
            .map(|ws| ws.active_tab_index())
        else {
            return false;
        };
        let Some(binding) = self.preview_binding_for_tab(ws_idx, tab_idx) else {
            return false;
        };
        if binding.url.is_none() {
            // Pre-url-era record: nothing to open, no origin to match.
            tracing::debug!(
                token = binding.token.as_str(),
                "wired preview has no url (old record) — nothing to open"
            );
            return false;
        }
        let now = Instant::now();
        if let Some((last_token, at)) = &self.preview_last_show {
            if *last_token == binding.token && now.duration_since(*at) < PREVIEW_SHOW_DEBOUNCE {
                return false; // spam re-focus: one window action per token/window
            }
        }
        self.preview_last_show = Some((binding.token.clone(), now));
        dispatch(self, binding);
        true
    }

    /// The real show action: an idempotent activate-or-launch on a detached
    /// background thread — CDP probing and process spawning must never touch
    /// the event loop.
    fn dispatch_preview_show(&mut self, binding: crate::preview_bindings::PreviewBinding) {
        let Some(url) = binding.url.clone() else {
            return; // guarded upstream; stay defensive
        };
        let token = binding.token.clone();
        if let Err(err) = std::thread::Builder::new()
            .name("preview-show".into())
            .spawn(move || preview_show_worker(&token, &url))
        {
            tracing::warn!(?err, "preview-show worker thread failed to start");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn test_app() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        )
    }

    fn unique_dir(tag: &str) -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "herdr-preview-poll-{tag}-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir).expect("create test dir");
        dir
    }

    fn now_epoch() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn append(path: &std::path::Path, line: &str) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open bindings file");
        writeln!(f, "{line}").expect("append binding line");
    }

    // PV1: the poll is interval-gated — a due poll arms the interval, and an
    // immediate re-poll inside the interval does nothing (no stat, no read).
    #[tokio::test]
    async fn preview_poll_is_interval_gated() {
        let mut app = test_app();
        let now = Instant::now();
        let _ = app.refresh_preview_bindings_if_due(now);
        assert_eq!(
            app.next_preview_bindings_poll,
            Some(now + PREVIEW_BINDINGS_POLL_INTERVAL)
        );
        assert!(
            !app.refresh_preview_bindings_if_due(now + std::time::Duration::from_millis(100)),
            "inside the interval the poll must be a no-op"
        );
    }

    // PV2: fingerprint gating against an injected file — first look loads,
    // unchanged file no-ops, an appended bound record reloads, deletion clears.
    #[tokio::test]
    async fn preview_refresh_is_fingerprint_driven() {
        let dir = unique_dir("fp");
        let path = dir.join("bindings.jsonl");
        let ts = now_epoch();
        append(
            &path,
            &format!(
                r#"{{"token":"tokA","state":"pending","claude_pid":4242,"url":"http://127.0.0.1:8770/","ts":{ts}}}"#
            ),
        );

        let mut app = test_app();
        assert!(app.refresh_preview_bindings_at(&path), "first look loads");
        assert_eq!(app.state.preview_bindings.len(), 1);
        assert_eq!(app.state.preview_bindings[0].claude_pid, Some(4242));

        assert!(
            !app.refresh_preview_bindings_at(&path),
            "unchanged file must not re-read"
        );

        append(
            &path,
            &format!(
                r#"{{"token":"tokA","state":"bound","session_id":"930e93d6-20c","claude_pid":"4242","ts":{ts}}}"#
            ),
        );
        assert!(
            app.refresh_preview_bindings_at(&path),
            "bound record reloads"
        );
        assert_eq!(
            app.state.preview_bindings[0].session_prefix.as_deref(),
            Some("930e93d6-20c")
        );

        std::fs::remove_file(&path).expect("delete bindings file");
        assert!(app.refresh_preview_bindings_at(&path), "deletion clears");
        assert!(app.state.preview_bindings.is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    // PV3: a file change that does not change the merged live view (an
    // already-expired record appended) updates the fingerprint but reports
    // no state change — callers must not re-render for nothing.
    #[tokio::test]
    async fn expired_append_changes_fingerprint_but_not_state() {
        let dir = unique_dir("expired");
        let path = dir.join("bindings.jsonl");
        let ts = now_epoch();
        append(
            &path,
            &format!(r#"{{"token":"live","state":"pending","claude_pid":7,"ts":{ts}}}"#),
        );

        let mut app = test_app();
        assert!(app.refresh_preview_bindings_at(&path));
        assert_eq!(app.state.preview_bindings.len(), 1);

        let ancient = ts.saturating_sub(60 * 60 * 24 * 30); // far past the 48h TTL
        append(
            &path,
            &format!(r#"{{"token":"dead","state":"pending","claude_pid":8,"ts":{ancient}}}"#),
        );
        assert!(
            !app.refresh_preview_bindings_at(&path),
            "expired append must not report a state change"
        );
        assert_eq!(app.state.preview_bindings.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    // ── P1c-2: binding↔tab lookup ───────────────────────────────────────

    fn req(
        project_path: std::path::PathBuf,
        session_id: Option<&str>,
    ) -> crate::app::state::ProjectChatTabRequest {
        crate::app::state::ProjectChatTabRequest {
            project_path,
            session_id: session_id.map(str::to_string),
        }
    }

    fn binding(
        token: &str,
        pid: Option<u32>,
        session_prefix: Option<&str>,
        ts: u64,
    ) -> crate::preview_bindings::PreviewBinding {
        crate::preview_bindings::PreviewBinding {
            token: token.into(),
            claude_pid: pid,
            session_prefix: session_prefix.map(str::to_string),
            url: Some("http://127.0.0.1:8770/".into()),
            ts,
        }
    }

    fn app_with_plain_workspace(dir: &std::path::Path) -> crate::app::App {
        let mut app = test_app();
        let mut ws = crate::workspace::Workspace::test_new("proj");
        ws.identity_cwd = dir.to_path_buf();
        app.state.workspaces = vec![ws];
        app.state.active = Some(0);
        app.state.selected = 0;
        app
    }

    fn pgrep(pattern: &str) -> Vec<u32> {
        let out = std::process::Command::new("pgrep")
            .args(["-f", pattern])
            .output()
            .expect("pgrep runs");
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|line| line.trim().parse().ok())
            .collect()
    }

    // L1: a bound 12-char prefix (hook truncates with `cut -c1-12`) must
    // prefix-match the tab's FULL resumed session id; a wrong prefix and a
    // degenerate-short (malformed) prefix must not match anything.
    #[tokio::test]
    async fn lookup_matches_bound_session_by_12_char_prefix() {
        let dir = unique_dir("l1");
        let mut app = app_with_plain_workspace(&dir);
        app.state.workspaces[0].tabs[0].resumed_session_id =
            Some("db792de9-036c-4e96-9316-bef3b9b7817e".into());

        app.state.preview_bindings = vec![binding("tok-right", None, Some("db792de9-036"), 100)];
        assert_eq!(
            app.preview_binding_for_tab(0, 0).map(|b| b.token),
            Some("tok-right".into())
        );

        app.state.preview_bindings = vec![binding("tok-wrong", None, Some("930e93d6-20c"), 100)];
        assert_eq!(app.preview_binding_for_tab(0, 0), None);

        app.state.preview_bindings = vec![binding("tok-short", None, Some("db"), 100)];
        assert_eq!(
            app.preview_binding_for_tab(0, 0),
            None,
            "a malformed short prefix must never match"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // L2: two bindings wired to the same session (dev-browser ran twice) →
    // the newest one wins, because it carries the latest preview URL.
    #[tokio::test]
    async fn lookup_prefers_the_newest_matching_binding() {
        let dir = unique_dir("l2");
        let mut app = app_with_plain_workspace(&dir);
        app.state.workspaces[0].tabs[0].resumed_session_id =
            Some("db792de9-036c-4e96-9316-bef3b9b7817e".into());
        app.state.preview_bindings = vec![
            binding("tok-new", None, Some("db792de9-036"), 200),
            binding("tok-old", None, Some("db792de9-036"), 100),
        ];
        assert_eq!(
            app.preview_binding_for_tab(0, 1.min(0)).map(|b| b.token),
            Some("tok-new".into())
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // L3 (chain proof, covers P1c-1): a spawned tab's pane child pid — read
    // through registry → TerminalRuntime::child_pid — resolves a pending
    // binding even when the tab has NO resumed session id (new chats). A
    // nonexistent-pid binding (newest) must fall through, and with only a
    // nonexistent pid the lookup must return None (L4).
    #[tokio::test]
    async fn lookup_matches_pending_binding_by_pane_child_pid() {
        let dir = unique_dir("l3");
        let mut app = app_with_plain_workspace(&dir);
        let argv = vec!["/bin/sh".into(), "-c".into(), "sleep 300".into()];
        app.open_project_chat_tab_with_argv(req(dir.clone(), None), &argv, Vec::new());

        let tab = &app.state.workspaces[0].tabs[1];
        let tid = tab.panes[&tab.root_pane].attached_terminal_id.clone();
        let mut pid = None;
        for _ in 0..100 {
            pid = app
                .terminal_runtimes
                .get(&tid)
                .and_then(|rt| rt.child_pid());
            if pid.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        let pid = pid.expect("spawned pane must expose its child pid");

        app.state.preview_bindings = vec![
            binding("tok-ghost", Some(999_999_999), None, 300),
            binding("tok-pid", Some(pid), None, 200),
        ];
        assert_eq!(
            app.preview_binding_for_tab(0, 1).map(|b| b.token),
            Some("tok-pid".into()),
            "direct pane child pid must match past a nonexistent-pid binding"
        );

        app.state.preview_bindings = vec![binding("tok-ghost", Some(999_999_999), None, 300)];
        assert_eq!(
            app.preview_binding_for_tab(0, 1),
            None,
            "L4: no false positive"
        );

        app.state.close_tab();
        app.shutdown_detached_terminal_runtimes();
        let _ = std::fs::remove_dir_all(dir);
    }

    // L5 (linux): a claude started INSIDE a shell tab is not the pane child
    // itself, but a member of the pane's process session — the membership
    // pass must still wire it (T5c unique-sleep marker pattern).
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn lookup_matches_binding_by_pane_session_membership() {
        let dir = unique_dir("l5");
        let tag = format!("8{:05}", std::process::id() % 100_000);
        let script = format!("sleep {tag}1 & sleep {tag}2");
        let argv = vec!["/bin/sh".to_string(), "-c".to_string(), script];

        let mut app = app_with_plain_workspace(&dir);
        app.open_project_chat_tab_with_argv(req(dir.clone(), None), &argv, Vec::new());

        let mut background = Vec::new();
        for _ in 0..100 {
            background = pgrep(&format!("sleep {tag}1"));
            if !background.is_empty() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let member_pid = *background
            .first()
            .expect("background sleep must come up before the assertion");

        app.state.preview_bindings = vec![binding("tok-member", Some(member_pid), None, 100)];
        assert_eq!(
            app.preview_binding_for_tab(0, 1).map(|b| b.token),
            Some("tok-member".into()),
            "session membership must wire a nested process"
        );

        app.state.close_tab();
        app.shutdown_detached_terminal_runtimes();
        let _ = std::fs::remove_dir_all(dir);
    }

    // ── P1c-3: tab-focus trigger + deferred consume ─────────────────────

    // F1: every switch_workspace_tab (the focus funnel: tab-bar click,
    // sidebar chat row, spam-guard focus) must arm the preview-show check.
    #[tokio::test]
    async fn switching_tab_arms_the_preview_show_request() {
        let dir = unique_dir("f1");
        let mut app = app_with_plain_workspace(&dir);
        assert!(!app.state.request_preview_show, "starts unarmed");
        assert!(app.state.switch_workspace_tab(0, 0));
        assert!(
            app.state.request_preview_show,
            "focus change arms the check"
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    // F2: a wired active tab dispatches EXACTLY once per armed request, with
    // the right binding; the flag is consumed (no re-dispatch without a new
    // focus change).
    #[tokio::test]
    async fn consume_dispatches_once_for_a_wired_active_tab() {
        let dir = unique_dir("f2");
        let mut app = app_with_plain_workspace(&dir);
        app.state.workspaces[0].tabs[0].resumed_session_id =
            Some("db792de9-036c-4e96-9316-bef3b9b7817e".into());
        app.state.preview_bindings = vec![binding("tok-wired", None, Some("db792de9-036"), 100)];
        app.state.request_preview_show = true;

        let mut got: Vec<String> = Vec::new();
        assert!(app.handle_preview_show_request_with(|_, b| got.push(b.token)));
        assert_eq!(got, vec!["tok-wired".to_string()]);
        assert!(!app.state.request_preview_show, "request consumed");

        assert!(
            !app.handle_preview_show_request_with(|_, b| got.push(b.token)),
            "no re-dispatch without a new focus change"
        );
        assert_eq!(got.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    // F3: an armed request on an UNWIRED tab consumes the flag quietly —
    // no dispatch, no error, ready for the next focus change.
    #[tokio::test]
    async fn consume_is_quiet_for_an_unwired_tab() {
        let dir = unique_dir("f3");
        let mut app = app_with_plain_workspace(&dir);
        app.state.preview_bindings =
            vec![binding("tok-elsewhere", None, Some("930e93d6-20c0"), 100)];
        app.state.request_preview_show = true;

        let mut dispatched = false;
        assert!(!app.handle_preview_show_request_with(|_, _| dispatched = true));
        assert!(!dispatched, "unwired tab must not dispatch");
        assert!(!app.state.request_preview_show, "flag still consumed");
        let _ = std::fs::remove_dir_all(dir);
    }

    // F4: without an armed request the consume pass is a strict no-op.
    #[tokio::test]
    async fn consume_without_request_is_a_no_op() {
        let dir = unique_dir("f4");
        let mut app = app_with_plain_workspace(&dir);
        let mut dispatched = false;
        assert!(!app.handle_preview_show_request_with(|_, _| dispatched = true));
        assert!(!dispatched);
        let _ = std::fs::remove_dir_all(dir);
    }

    // ── P1c-4: idempotent show action (origin + plan + debounce) ────────

    // O1: origin extraction — the identity of a preview tab.
    #[test]
    fn url_origin_extracts_scheme_host_port() {
        assert_eq!(
            url_origin("http://127.0.0.1:8770/"),
            Some("http://127.0.0.1:8770")
        );
        assert_eq!(
            url_origin("http://127.0.0.1:8770/deep/path?q=1#frag"),
            Some("http://127.0.0.1:8770")
        );
        assert_eq!(
            url_origin("http://100.75.115.68:8770"),
            Some("http://100.75.115.68:8770")
        );
        assert_eq!(url_origin("not-a-url"), None);
        assert_eq!(
            url_origin("http:///path-only"),
            None,
            "an empty authority is malformed"
        );
    }

    fn target(id: &str, kind: &str, url: &str) -> CdpTarget {
        CdpTarget {
            id: id.into(),
            kind: kind.into(),
            url: url.into(),
        }
    }

    // P1: an existing page tab on the preview's origin must be activated —
    // never spawned again — even after the user navigated deeper; non-page
    // targets and other origins must not steal the match.
    #[test]
    fn plan_activates_the_existing_page_on_the_preview_origin() {
        let targets = vec![
            target("t-other", "page", "http://127.0.0.1:3000/"),
            target("t-bg", "background_page", "http://127.0.0.1:8770/"),
            target("t-hit", "page", "http://127.0.0.1:8770/deep/path?q=1"),
        ];
        assert_eq!(
            preview_show_plan(&targets, "http://127.0.0.1:8770/"),
            PreviewShowPlan::Activate("t-hit".into())
        );
    }

    // P2: no tab on that origin → launch (dev-browser reuses the window).
    #[test]
    fn plan_launches_when_no_tab_matches() {
        let targets = vec![target("t-other", "page", "http://127.0.0.1:3000/")];
        assert_eq!(
            preview_show_plan(&targets, "http://127.0.0.1:8770/"),
            PreviewShowPlan::Launch
        );
        assert_eq!(
            preview_show_plan(&[], "http://127.0.0.1:8770/"),
            PreviewShowPlan::Launch
        );
    }

    // D1: rapid re-focus of the same wired token dispatches ONCE (the user's
    // "10 clicks must not spawn 10 windows" requirement, at the real consume
    // path); a rewound debounce window and a different token pass again.
    #[tokio::test]
    async fn consume_debounces_rapid_refocus_of_the_same_token() {
        let dir = unique_dir("d1");
        let mut app = app_with_plain_workspace(&dir);
        app.state.workspaces[0].tabs[0].resumed_session_id =
            Some("db792de9-036c-4e96-9316-bef3b9b7817e".into());
        app.state.preview_bindings = vec![binding("tok-a", None, Some("db792de9-036"), 100)];

        let mut hits = 0;
        app.state.request_preview_show = true;
        assert!(app.handle_preview_show_request_with(|_, _| hits += 1));
        app.state.request_preview_show = true; // immediate re-focus (spam)
        assert!(
            !app.handle_preview_show_request_with(|_, _| hits += 1),
            "second dispatch inside the debounce window must be swallowed"
        );
        assert_eq!(hits, 1);

        // an elapsed window passes again (rewind the anchor instead of sleeping)
        app.preview_last_show =
            Some(("tok-a".into(), Instant::now() - (PREVIEW_SHOW_DEBOUNCE * 2)));
        app.state.request_preview_show = true;
        assert!(app.handle_preview_show_request_with(|_, _| hits += 1));
        assert_eq!(hits, 2);

        // a NEWER preview (different token) must not be blocked by tok-a
        app.state.preview_bindings = vec![binding("tok-b", None, Some("db792de9-036"), 200)];
        app.state.request_preview_show = true;
        assert!(app.handle_preview_show_request_with(|_, _| hits += 1));
        assert_eq!(hits, 3);
        let _ = std::fs::remove_dir_all(dir);
    }

    // G1: gdbus prints a GVariant tuple with escaped quotes, not raw JSON —
    // the extractor must recover the parseable array (real captured sample),
    // and must not invent output from garbage.
    #[test]
    fn gdbus_tuple_output_yields_parseable_json() {
        let raw = r#"("[{\"class\":\"chromium-browser\",\"pid\":2291913,\"id\":2091965964,\"focus\":false,\"title\":\"CC Kumanda - Chromium\"}]",)"#;
        let json = extract_gdbus_json_array(raw).expect("array extracted");
        let parsed: Vec<ShellWindow> = serde_json::from_str(&json).expect("parses");
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].pid, 2_291_913);
        assert_eq!(parsed[0].id, 2_091_965_964);

        assert_eq!(extract_gdbus_json_array("no array here"), None);
        assert_eq!(extract_gdbus_json_array(""), None);
    }

    // G2: window matching is PID-anchored with a defensive class check —
    // this desktop really runs several unrelated chromium windows whose
    // classes even differ in case; title matching would be ambiguous.
    #[test]
    fn devbrowser_window_is_matched_by_main_pid() {
        let list = r#"[
            {"class":"kitty","pid":385592,"id":11,"focus":false,"title":"~"},
            {"class":"Chromium-browser","pid":21082,"id":22,"focus":false,"title":"WhatsApp"},
            {"class":"chromium-browser","pid":2291913,"id":33,"focus":false,"title":"CC Kumanda - Chromium"},
            {"class":"code","pid":2291913,"id":44,"focus":false,"title":"impostor same-pid non-chromium"}
        ]"#;
        assert_eq!(devbrowser_window_id(list, 2_291_913), Some(33));
        assert_eq!(
            devbrowser_window_id(list, 21_082),
            Some(22),
            "capital-C class variant must still match case-insensitively"
        );
        assert_eq!(devbrowser_window_id(list, 999), None, "unknown pid");
        assert_eq!(devbrowser_window_id("{malformed", 2_291_913), None);
    }

    // U1: a wired binding WITHOUT a url (pre-url-era record) can neither be
    // origin-matched nor launched — the consume pass must skip it quietly.
    #[tokio::test]
    async fn consume_skips_a_wired_binding_without_url() {
        let dir = unique_dir("u1");
        let mut app = app_with_plain_workspace(&dir);
        app.state.workspaces[0].tabs[0].resumed_session_id =
            Some("db792de9-036c-4e96-9316-bef3b9b7817e".into());
        app.state.preview_bindings = vec![crate::preview_bindings::PreviewBinding {
            token: "tok-nourl".into(),
            claude_pid: None,
            session_prefix: Some("db792de9-036".into()),
            url: None,
            ts: 100,
        }];
        app.state.request_preview_show = true;

        let mut dispatched = false;
        assert!(!app.handle_preview_show_request_with(|_, _| dispatched = true));
        assert!(!dispatched, "nothing to open without a url");
        assert!(!app.state.request_preview_show, "flag still consumed");
        let _ = std::fs::remove_dir_all(dir);
    }
}
