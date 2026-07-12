//! Claude Code session (chat) reader for the Projects sidebar tab.
//!
//! Claude Code stores each project's chat sessions under
//! `<home>/.claude/projects/<encoded>/*.jsonl`, where every `.jsonl` file is one
//! chat session and the directory name is the project's absolute path with every
//! non-ASCII-alphanumeric character replaced by `-`.
//!
//! The encoding was verified against real on-disk data (51/52 local projects
//! matched; the single miss was a session that `cd`-ed away from its start
//! directory, and `ssh-<uuid>` directories are remote sessions). Because the
//! encoding collapses `/`, `.`, ` `, `_`, `-`, … all to `-`, it is LOSSY and
//! therefore NEVER reversed — we only ever go project-path -> directory.
//!
//! This module is TUI/client-layer pure data (CLAUDE.md boundary): no PTYs, no
//! runtime state, no network. It never panics on malformed input.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// One Claude Code chat session belonging to a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudeSession {
    /// Session id = the `.jsonl` file stem (a UUID).
    pub id: String,
    /// Display title: `custom-title` > `ai-title` > first user message > "(untitled)".
    pub title: String,
    /// File modification time; used for chronological ordering (newest first).
    pub last_modified: SystemTime,
    /// Number of user + assistant turns (a rough activity signal).
    pub msg_count: usize,
}

/// Placeholder shown when a session has no derivable title.
pub const UNTITLED: &str = "(untitled)";

/// Maximum displayed title length (in characters) before truncation.
const MAX_TITLE_CHARS: usize = 80;

/// Encode an absolute project path into its Claude Code storage directory name.
///
/// Rule (empirically verified): every character that is not ASCII alphanumeric
/// becomes `-`. This is LOSSY and NOT reversible — callers must only go
/// path -> directory, never the reverse.
pub fn encode_project_path(path: &str) -> String {
    path.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// Resolve `<home>/.claude/projects` using an injectable env lookup so the reader
/// is testable against a fake HOME without touching the real `~/.claude`.
pub(crate) fn claude_projects_dir(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    let home = env("HOME").map(PathBuf::from)?;
    if home.as_os_str().is_empty() {
        return None;
    }
    Some(home.join(".claude").join("projects"))
}

/// Convenience wrapper over [`claude_projects_dir`] using the real process env.
pub fn default_claude_projects_dir() -> Option<PathBuf> {
    claude_projects_dir(|k| std::env::var_os(k))
}

/// Read every chat session for `project_path`, newest first.
///
/// `projects_dir` is the `.../.claude/projects` root (injected for testability).
/// Never panics: a missing/unreadable project directory yields an empty list,
/// and malformed session files are skipped individually.
// Production callers go through the cached/limited variant; this full read
// remains the reference behavior exercised by this module's tests.
#[cfg(test)]
pub fn read_sessions_for_project(projects_dir: &Path, project_path: &str) -> Vec<ClaudeSession> {
    read_recent_sessions_for_project(projects_dir, project_path, usize::MAX).0
}

#[cfg(test)]
pub fn read_recent_sessions_for_project(
    projects_dir: &Path,
    project_path: &str,
    limit: usize,
) -> (Vec<ClaudeSession>, usize) {
    read_recent_inner(projects_dir, project_path, limit, None)
}

/// Per-file parse cache keyed by (mtime, size): an unchanged session file is
/// never re-read, so a refresh costs only the DIFF — usually zero or one
/// files — instead of re-parsing the store (the incremental "cc l" pattern).
pub type SessionParseCache =
    std::collections::HashMap<std::path::PathBuf, ((std::time::SystemTime, u64), ClaudeSession)>;

/// Like [`read_recent_sessions_for_project`] with an incremental parse cache.
pub fn read_recent_sessions_for_project_cached(
    projects_dir: &Path,
    project_path: &str,
    limit: usize,
    cache: &mut SessionParseCache,
) -> (Vec<ClaudeSession>, usize) {
    read_recent_inner(projects_dir, project_path, limit, Some(cache))
}

/// Like [`read_sessions_for_project`] but parses only the `limit` newest
/// session files (ranked by mtime from directory metadata alone), returning
/// the parsed sessions plus the TOTAL session-file count. Parsing reads whole
/// files, so a busy project (hundreds of files, tens of MB) must never be
/// fully parsed just to list its newest few chats.
fn read_recent_inner(
    projects_dir: &Path,
    project_path: &str,
    limit: usize,
    mut cache: Option<&mut SessionParseCache>,
) -> (Vec<ClaudeSession>, usize) {
    let encoded = encode_project_path(project_path);
    let dir = projects_dir.join(&encoded);

    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(err) => {
            // A project with no chats yet is a normal, expected state; only log
            // at debug so it does not spam. NotFound is silent-by-design here.
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(?dir, %err, "claude_sessions: read_dir failed");
            }
            return (Vec::new(), 0);
        }
    };

    // Rank candidates by mtime using directory metadata only; files are
    // opened just for the newest `limit` of them that missed the cache.
    let mut candidates: Vec<(std::time::SystemTime, u64, std::path::PathBuf)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                return None;
            }
            let (modified, size) = entry
                .metadata()
                .map(|meta| {
                    (
                        meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                        meta.len(),
                    )
                })
                .unwrap_or((std::time::SystemTime::UNIX_EPOCH, 0));
            Some((modified, size, path))
        })
        .collect();
    let total = candidates.len();
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));

    let mut sessions: Vec<ClaudeSession> = Vec::new();
    for (modified, size, path) in candidates.into_iter().take(limit) {
        let key = (modified, size);
        if let Some(cache) = cache.as_deref_mut() {
            if let Some((cached_key, session)) = cache.get(&path) {
                if *cached_key == key {
                    sessions.push(session.clone());
                    continue;
                }
            }
        }
        match parse_session_file(&path) {
            Some(session) => {
                if let Some(cache) = cache.as_deref_mut() {
                    cache.insert(path, (key, session.clone()));
                }
                sessions.push(session);
            }
            None => tracing::debug!(?path, "claude_sessions: skipped unreadable session file"),
        }
    }

    // Chronological: newest first (mtime desc). Ties broken by id for a stable,
    // deterministic order (important for reproducible tests and rendering).
    sessions.sort_by(|a, b| {
        b.last_modified
            .cmp(&a.last_modified)
            .then_with(|| a.id.cmp(&b.id))
    });
    (sessions, total)
}

/// Parse a single `<uuid>.jsonl` session file into a [`ClaudeSession`].
///
/// Returns `None` only when the file has no usable id or cannot be read at all.
/// Malformed individual lines are skipped, never fatal.
fn parse_session_file(path: &Path) -> Option<ClaudeSession> {
    let id = path.file_stem()?.to_str()?.to_string();
    let content = fs::read_to_string(path).ok()?;
    // Metadata failure must not panic; fall back to the epoch (sorts oldest).
    let last_modified = fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut custom_title: Option<String> = None;
    let mut ai_title: Option<String> = None;
    let mut first_user: Option<String> = None;
    let mut msg_count: usize = 0;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // Malformed / partial JSON line -> skip (real jsonl files can be
        // truncated mid-write). Never crash the whole session for one bad line.
        let value: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(obj) = value.as_object() else {
            continue;
        };

        match obj.get("type").and_then(|t| t.as_str()) {
            // Later title lines override earlier ones -> the last one wins.
            Some("custom-title") => {
                if let Some(t) = nonempty_str(obj.get("customTitle")) {
                    custom_title = Some(t);
                }
            }
            Some("ai-title") => {
                if let Some(t) = nonempty_str(obj.get("aiTitle")) {
                    ai_title = Some(t);
                }
            }
            Some("user") => {
                msg_count += 1;
                if first_user.is_none() {
                    first_user = extract_user_text(obj);
                }
            }
            Some("assistant") => {
                msg_count += 1;
            }
            _ => {}
        }
    }

    let title = derive_title(custom_title, ai_title, first_user);
    Some(ClaudeSession {
        id,
        title,
        last_modified,
        msg_count,
    })
}

/// Title precedence: user-set custom-title > AI-generated ai-title >
/// first user message > `UNTITLED`.
fn derive_title(
    custom_title: Option<String>,
    ai_title: Option<String>,
    first_user: Option<String>,
) -> String {
    custom_title
        .or(ai_title)
        .or(first_user)
        .map(|t| clean_title(&t))
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| UNTITLED.to_string())
}

/// Extract the first text of a user message. `message.content` may be a plain
/// string or an array of typed blocks (`{type:"text", text:"…"}`).
fn extract_user_text(obj: &serde_json::Map<String, serde_json::Value>) -> Option<String> {
    let content = obj.get("message")?.as_object()?.get("content")?;
    let text = match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(blocks) => blocks.iter().find_map(|block| {
            let block = block.as_object()?;
            if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                block
                    .get("text")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string())
            } else {
                None
            }
        })?,
        _ => return None,
    };
    let cleaned = clean_title(&text);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Read a string field, returning `Some` only when present and non-blank.
fn nonempty_str(value: Option<&serde_json::Value>) -> Option<String> {
    let s = value?.as_str()?.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Collapse whitespace to a single line and truncate for display.
fn clean_title(raw: &str) -> String {
    let one_line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() > MAX_TITLE_CHARS {
        let truncated: String = one_line.chars().take(MAX_TITLE_CHARS - 1).collect();
        format!("{truncated}…")
    } else {
        one_line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::Duration;

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Isolated fake `.claude/projects` root. Never touches the real `~/.claude`;
    /// cleaned up on drop.
    struct TempProjects {
        root: PathBuf,
    }

    impl TempProjects {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-cs-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp projects root");
            Self { root }
        }

        /// Write a session file for `project` with the given raw jsonl `lines`.
        fn write_session(&self, project: &str, session_id: &str, lines: &[&str]) -> PathBuf {
            let dir = self.root.join(encode_project_path(project));
            fs::create_dir_all(&dir).expect("create project dir");
            let path = dir.join(format!("{session_id}.jsonl"));
            let mut file = fs::File::create(&path).expect("create session file");
            for line in lines {
                writeln!(file, "{line}").expect("write session line");
            }
            path
        }
    }

    impl Drop for TempProjects {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    // ---- T1.1a: basic encoding ------------------------------------------------
    #[test]
    fn encode_basic_path() {
        assert_eq!(
            encode_project_path("/home/ayaz/projects/x"),
            "-home-ayaz-projects-x"
        );
    }

    // ---- T1.1b: real edge cases — space and dot also become '-' --------------
    #[test]
    fn encode_space_and_dot_and_underscore() {
        assert_eq!(
            encode_project_path("/Users/a/The Planner"),
            "-Users-a-The-Planner"
        );
        assert_eq!(encode_project_path("/home/a/.config"), "-home-a--config");
        // underscore is non-alphanumeric -> '-'
        assert_eq!(encode_project_path("/home/a/my_proj"), "-home-a-my-proj");
        // digits are preserved
        assert_eq!(encode_project_path("/srv/app2"), "-srv-app2");
    }

    // ---- T1.1c: malformed / partial jsonl lines are skipped, not fatal -------
    #[test]
    fn malformed_lines_are_skipped_and_valid_session_still_reads() {
        let tp = TempProjects::new("malformed");
        tp.write_session(
            "/home/x/proj",
            "1111",
            &[
                "{ this is not valid json",
                r#"{"type":"ai-title","aiTitle":"real title"}"#,
                "",
                r#"{"type":"user","message":{"content":"hi"}}"#,
                "{truncated mid-write",
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].title, "real title");
        assert_eq!(sessions[0].msg_count, 1);
    }

    // ---- T1.1d: missing / empty project dir -> empty list, no panic ----------
    #[test]
    fn missing_project_dir_returns_empty() {
        let tp = TempProjects::new("missing");
        let sessions = read_sessions_for_project(&tp.root, "/home/x/never-opened");
        assert!(sessions.is_empty());
    }

    #[test]
    fn empty_project_dir_returns_empty() {
        let tp = TempProjects::new("empty");
        // create the encoded dir but no jsonl files
        fs::create_dir_all(tp.root.join(encode_project_path("/home/x/proj"))).unwrap();
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        assert!(sessions.is_empty());
    }

    // ---- T1.1e: title fallback chain -----------------------------------------
    #[test]
    fn title_falls_back_to_first_user_then_untitled() {
        let tp = TempProjects::new("fallback");
        // no titles -> first user message
        tp.write_session(
            "/home/x/proj",
            "user-only",
            &[r#"{"type":"user","message":{"content":"open the preview"}}"#],
        );
        // no titles, no user -> UNTITLED
        tp.write_session(
            "/home/x/proj",
            "empty-meta",
            &[r#"{"type":"assistant","message":{"content":"hello"}}"#],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let by_id = |id: &str| sessions.iter().find(|s| s.id == id).unwrap();
        assert_eq!(by_id("user-only").title, "open the preview");
        assert_eq!(by_id("empty-meta").title, UNTITLED);
    }

    // ---- T1.1g: custom-title beats ai-title; last value wins ------------------
    #[test]
    fn custom_title_beats_ai_title_and_last_wins() {
        let tp = TempProjects::new("precedence");
        tp.write_session(
            "/home/x/proj",
            "titled",
            &[
                r#"{"type":"ai-title","aiTitle":"ai first"}"#,
                r#"{"type":"custom-title","customTitle":"user pick"}"#,
                r#"{"type":"ai-title","aiTitle":"ai second"}"#,
                r#"{"type":"custom-title","customTitle":"final pick"}"#,
                r#"{"type":"user","message":{"content":"ignored for title"}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        assert_eq!(sessions[0].title, "final pick");
    }

    // ---- T1.1f: chronological ordering (newest first by mtime) ----------------
    #[test]
    fn sessions_sorted_newest_first() {
        let tp = TempProjects::new("order");
        let older = tp.write_session(
            "/home/x/proj",
            "aaa-older",
            &[r#"{"type":"custom-title","customTitle":"older"}"#],
        );
        let newer = tp.write_session(
            "/home/x/proj",
            "bbb-newer",
            &[r#"{"type":"custom-title","customTitle":"newer"}"#],
        );
        // Force a deterministic mtime ordering regardless of write speed.
        let base = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        set_mtime(&older, base);
        set_mtime(&newer, base + Duration::from_secs(60));

        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].title, "newer");
        assert_eq!(sessions[1].title, "older");
    }

    // Deterministic mtime via std (File::set_modified, stable since Rust 1.75) —
    // no external `filetime` dep, cross-platform, clippy-clean.
    fn set_mtime(path: &Path, when: SystemTime) {
        let file = fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open session file to set mtime");
        file.set_modified(when).expect("set mtime");
    }

    // ---- claude_projects_dir env resolution ----------------------------------
    #[test]
    fn projects_dir_resolves_from_home() {
        let dir = claude_projects_dir(|k| {
            if k == "HOME" {
                Some(OsString::from("/home/tester"))
            } else {
                None
            }
        });
        assert_eq!(dir, Some(PathBuf::from("/home/tester/.claude/projects")));
    }

    #[test]
    fn projects_dir_none_without_home() {
        assert_eq!(claude_projects_dir(|_| None), None);
        assert_eq!(claude_projects_dir(|_| Some(OsString::new())), None);
    }
}
