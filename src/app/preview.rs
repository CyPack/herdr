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
    // Temporary: the production caller is the P1c-3 tab-focus trigger; until
    // that lands only this module's tests exercise the lookup.
    #[allow(dead_code)]
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
}
