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
}
