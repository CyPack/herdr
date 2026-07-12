//! click-bridge session-binding reader for preview↔tab sync.
//!
//! click-bridge (the local Alt+Click → terminal-agent bridge) records browser-tab
//! pairing state in `<home>/.click-bridge/bindings.jsonl` as append-only JSONL:
//!
//! ```text
//! pending: {"token":"…","state":"pending","claude_pid":N|null,"url":"…","ts":EPOCH}
//! bound:   {"token":"…","state":"bound","session_id":"<12 chars>","claude_pid":N|"N"|"","ts":EPOCH}
//! ```
//!
//! Field quirks verified against the live producers (dev-browser.sh, pair-url.sh,
//! click-bridge-inject.sh hook v4):
//! - `url` exists only in pending records written since 2026-07-12; older records
//!   lack it entirely.
//! - `session_id` in bound records is the Claude Code session UUID TRUNCATED to
//!   its first 12 characters (the hook applies `cut -c1-12`), so consumers must
//!   prefix-match, never equality-match, against a full session UUID.
//! - `claude_pid` may be a JSON number (pending), a numeric string (bound via the
//!   hook's Python `str()`), an empty string, `null`, or absent (lazy binds).
//! - Records honor a 48h TTL and are last-wins per token.
//!
//! This module merges pending + bound records into one view per token so the
//! runtime can map a binding to a tab either by claude pid (pane child pid) or
//! by session-id prefix (`Tab::resumed_session_id`). Pure data (CLAUDE.md
//! boundary): no PTYs, no runtime state; never panics on malformed input.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Bindings older than this are ignored (mirrors the hook's 48h TTL).
const BINDING_TTL_SECS: u64 = 48 * 60 * 60;

/// Merged (pending ∪ bound) view of one click-bridge tab binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewBinding {
    /// Pairing token carried by the browser tab (`#cb=TOKEN`).
    pub token: String,
    /// PID of the claude CLI process that launched the preview, when known.
    pub claude_pid: Option<u32>,
    /// First 12 characters of the bound Claude Code session id (bound records).
    pub session_prefix: Option<String>,
    /// Base preview URL without the `#cb` fragment (newer pending records).
    pub url: Option<String>,
    /// Epoch seconds of the newest record contributing to this view.
    pub ts: u64,
}

/// Resolve `<home>/.click-bridge/bindings.jsonl` via an injectable env lookup so
/// the reader is testable against a fake HOME without touching the live bridge.
pub(crate) fn bindings_path(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let home = env("HOME").map(PathBuf::from)?;
    if home.as_os_str().is_empty() {
        return None;
    }
    Some(home.join(".click-bridge").join("bindings.jsonl"))
}

/// Convenience wrapper over [`bindings_path`] using the real process env.
pub fn default_bindings_path() -> Option<PathBuf> {
    bindings_path(|k| std::env::var_os(k))
}

/// Read and merge every live binding, newest first.
///
/// Never panics: a missing/unreadable file yields an empty list, malformed lines
/// are skipped individually, and expired records (48h TTL against `now_epoch`)
/// are dropped before merging.
pub fn read_preview_bindings(path: &Path, now_epoch: u64) -> Vec<PreviewBinding> {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return Vec::new();
    };

    let mut merged: std::collections::HashMap<String, PreviewBinding> =
        std::collections::HashMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // malformed line: skip, never fail the whole file
        };
        let Some(obj) = value.as_object() else {
            continue;
        };
        let Some(token) = nonempty_str(obj.get("token")) else {
            continue;
        };
        let Some(ts) = obj.get("ts").and_then(epoch_secs) else {
            continue;
        };
        if now_epoch.saturating_sub(ts) > BINDING_TTL_SECS {
            continue; // expired (future timestamps saturate to age 0 and survive)
        }
        let state = obj.get("state").and_then(|v| v.as_str()).unwrap_or("");

        let entry = merged
            .entry(token.clone())
            .or_insert_with(|| PreviewBinding {
                token,
                claude_pid: None,
                session_prefix: None,
                url: None,
                ts,
            });
        entry.ts = entry.ts.max(ts);
        match state {
            "pending" => {
                if let Some(pid) = parse_pid(obj.get("claude_pid")) {
                    entry.claude_pid = Some(pid);
                }
                if let Some(url) = nonempty_str(obj.get("url")) {
                    entry.url = Some(url); // last-wins: newest pending owns the URL
                }
            }
            "bound" => {
                if let Some(sid) = nonempty_str(obj.get("session_id")) {
                    entry.session_prefix = Some(sid); // last-wins
                }
                if let Some(pid) = parse_pid(obj.get("claude_pid")) {
                    entry.claude_pid = Some(pid);
                }
            }
            _ => {} // unknown state: token seen (ts advanced) but no fields taken
        }
    }

    let mut bindings: Vec<PreviewBinding> = merged.into_values().collect();
    // Newest first; token as a deterministic tiebreaker for equal timestamps.
    bindings.sort_by(|a, b| b.ts.cmp(&a.ts).then_with(|| a.token.cmp(&b.token)));
    bindings
}

/// Accept a pid as a JSON number or a numeric string; reject 0/negative/junk.
fn parse_pid(value: Option<&serde_json::Value>) -> Option<u32> {
    match value? {
        serde_json::Value::Number(n) => n.as_u64().and_then(|p| u32::try_from(p).ok()),
        serde_json::Value::String(s) => s.trim().parse::<u32>().ok(),
        _ => None,
    }
    .filter(|pid| *pid > 0)
}

/// Epoch seconds from a JSON number (integer or float — producers write ints,
/// but a float from a future producer must not drop the record).
fn epoch_secs(value: &serde_json::Value) -> Option<u64> {
    if let Some(n) = value.as_u64() {
        return Some(n);
    }
    let f = value.as_f64()?;
    if f.is_finite() && f >= 0.0 {
        Some(f as u64)
    } else {
        None
    }
}

fn nonempty_str(value: Option<&serde_json::Value>) -> Option<String> {
    let s = value?.as_str()?.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};

    const NOW: u64 = 1_783_850_000;

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Isolated fake bindings file; never touches the live `~/.click-bridge`.
    struct TempBindings {
        root: PathBuf,
        path: PathBuf,
    }

    impl TempBindings {
        fn new(tag: &str, lines: &[&str]) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-pb-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            std::fs::create_dir_all(&root).expect("create temp bindings root");
            let path = root.join("bindings.jsonl");
            let mut file = std::fs::File::create(&path).expect("create bindings file");
            for line in lines {
                writeln!(file, "{line}").expect("write bindings line");
            }
            Self { root, path }
        }
    }

    impl Drop for TempBindings {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn line_pending(token: &str, pid: u32, url: Option<&str>, ts: u64) -> String {
        match url {
            Some(url) => format!(
                r#"{{"token":"{token}","state":"pending","claude_pid":{pid},"url":"{url}","ts":{ts}}}"#
            ),
            None => {
                format!(r#"{{"token":"{token}","state":"pending","claude_pid":{pid},"ts":{ts}}}"#)
            }
        }
    }

    #[test]
    fn merges_pending_and_bound_records_for_one_token() {
        let tb = TempBindings::new(
            "merge",
            &[
                &line_pending("tok1", 4242, Some("http://127.0.0.1:8770/"), NOW - 100),
                // hook v4 writes claude_pid as a STRING in bound records
                r#"{"token":"tok1","state":"bound","session_id":"930e93d6-20c","claude_pid":"4242","ts":1783849950}"#,
            ],
        );
        let got = read_preview_bindings(&tb.path, NOW);
        assert_eq!(
            got,
            vec![PreviewBinding {
                token: "tok1".into(),
                claude_pid: Some(4242),
                session_prefix: Some("930e93d6-20c".into()),
                url: Some("http://127.0.0.1:8770/".into()),
                ts: NOW - 50,
            }]
        );
    }

    #[test]
    fn skips_malformed_lines_without_dropping_valid_ones() {
        let tb = TempBindings::new(
            "corrupt",
            &[
                "{not json at all",
                r#"["an","array","not","an","object"]"#,
                r#"{"state":"pending","claude_pid":1,"ts":1783849999}"#, // token missing
                r#"{"token":"ok","state":"pending","claude_pid":7,"ts":1783849999}"#,
                r#"{"token":"nots"}"#, // ts missing
            ],
        );
        let got = read_preview_bindings(&tb.path, NOW);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].token, "ok");
        assert_eq!(got[0].claude_pid, Some(7));
    }

    #[test]
    fn drops_records_older_than_the_48h_ttl() {
        let expired = NOW - BINDING_TTL_SECS - 1;
        let fresh = NOW - BINDING_TTL_SECS + 60;
        let tb = TempBindings::new(
            "ttl",
            &[
                &line_pending("old", 1, Some("http://127.0.0.1:1111/"), expired),
                &line_pending("new", 2, Some("http://127.0.0.1:2222/"), fresh),
            ],
        );
        let got = read_preview_bindings(&tb.path, NOW);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].token, "new");
    }

    #[test]
    fn last_pending_record_wins_per_token() {
        let tb = TempBindings::new(
            "lastwins",
            &[
                &line_pending("tok", 10, Some("http://127.0.0.1:3000/"), NOW - 500),
                &line_pending("tok", 11, Some("http://127.0.0.1:8770/"), NOW - 10),
            ],
        );
        let got = read_preview_bindings(&tb.path, NOW);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].claude_pid, Some(11));
        assert_eq!(got[0].url.as_deref(), Some("http://127.0.0.1:8770/"));
        assert_eq!(got[0].ts, NOW - 10);
    }

    #[test]
    fn tolerates_all_live_claude_pid_shapes() {
        let tb = TempBindings::new(
            "pids",
            &[
                // lazy bind: no claude_pid at all
                r#"{"token":"lazy","state":"bound","session_id":"aaaabbbbcccc","via":"lazy","ts":1783849000}"#,
                // empty-string pid (hook writes str(None or "") on dead-owner paths)
                r#"{"token":"empty","state":"bound","session_id":"ddddeeeeffff","claude_pid":"","ts":1783849000}"#,
                // null pid (dev-browser launched outside any claude session)
                r#"{"token":"null","state":"pending","claude_pid":null,"url":"http://127.0.0.1:9/","ts":1783849000}"#,
            ],
        );
        let got = read_preview_bindings(&tb.path, NOW);
        assert_eq!(got.len(), 3);
        for b in &got {
            assert_eq!(b.claude_pid, None, "token {} must have no pid", b.token);
        }
        let lazy = got.iter().find(|b| b.token == "lazy").expect("lazy");
        assert_eq!(lazy.session_prefix.as_deref(), Some("aaaabbbbcccc"));
        let null = got.iter().find(|b| b.token == "null").expect("null");
        assert_eq!(null.url.as_deref(), Some("http://127.0.0.1:9/"));
    }

    #[test]
    fn pre_url_era_pending_records_yield_no_url() {
        let tb = TempBindings::new("nourl", &[&line_pending("old-era", 99, None, NOW - 5)]);
        let got = read_preview_bindings(&tb.path, NOW);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].url, None);
        assert_eq!(got[0].claude_pid, Some(99));
    }

    #[test]
    fn missing_file_yields_empty_list() {
        let root = std::env::temp_dir().join(format!(
            "herdr-pb-test-missing-{}-{}",
            std::process::id(),
            unique()
        ));
        let got = read_preview_bindings(&root.join("bindings.jsonl"), NOW);
        assert!(got.is_empty());
    }

    #[test]
    fn newest_binding_sorts_first_with_deterministic_ties() {
        let tb = TempBindings::new(
            "order",
            &[
                &line_pending("older", 1, None, NOW - 300),
                &line_pending("newest", 2, None, NOW - 10),
                &line_pending("tie-b", 3, None, NOW - 100),
                &line_pending("tie-a", 4, None, NOW - 100),
            ],
        );
        let got: Vec<String> = read_preview_bindings(&tb.path, NOW)
            .into_iter()
            .map(|b| b.token)
            .collect();
        assert_eq!(got, vec!["newest", "tie-a", "tie-b", "older"]);
    }

    #[test]
    fn bindings_path_requires_a_nonempty_home() {
        assert_eq!(bindings_path(|_| None), None);
        assert_eq!(bindings_path(|_| Some(OsString::new())), None);
        let p = bindings_path(|_| Some(OsString::from("/home/tester"))).expect("path");
        assert_eq!(
            p,
            PathBuf::from("/home/tester/.click-bridge/bindings.jsonl")
        );
    }
}
