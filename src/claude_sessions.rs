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
    /// File modification time. An upper bound on activity: a resume that
    /// appends a bookkeeping line refreshes it without any message being sent.
    pub last_modified: SystemTime,
    /// TP-DRAW-15: when the last user/assistant message in the transcript was
    /// written, from the line's own `timestamp` field. `None` for a transcript
    /// with no timestamped messages (older formats). This — not the file's
    /// mtime and never the ledger's sighting — is what "how old is this chat"
    /// means to a person reading the list.
    pub last_message_at: Option<SystemTime>,
    /// Number of user + assistant turns (a rough activity signal).
    pub msg_count: usize,
    /// Normalised opening: the *shape* of the first user message.
    ///
    /// Separate from `title` on purpose. A title answers "what is this chat
    /// called" and prefers whatever a human or the model named it; an opening
    /// answers "how was this chat started", which is the only thing that tells
    /// a scheduled task apart from a person typing. When a chat carries an
    /// `ai-title`, the first user message never reaches the title at all — so
    /// deriving one from the other would lose exactly the cases that matter.
    ///
    /// `None` when the chat has no readable first user message.
    pub opening: Option<String>,
}

impl ClaudeSession {
    /// The moment this chat last moved, as a reader understands "moved":
    /// the last message's own timestamp when the transcript records one,
    /// else the file mtime (TP-DRAW-15).
    pub fn activity_time(&self) -> SystemTime {
        self.last_message_at.unwrap_or(self.last_modified)
    }
}

/// Placeholder shown when a session has no derivable title.
pub const UNTITLED: &str = "(untitled)";

/// Maximum displayed title length (in characters) before truncation.
const MAX_TITLE_CHARS: usize = 80;

/// How much of the first user message is kept after normalising.
///
/// Long enough to separate two automations that share an opening word, short
/// enough that a message which merely *starts* the same way still collapses to
/// one shape — the measured scheduled tasks differ within their first line.
const MAX_OPENING_CHARS: usize = 120;

/// How much raw text is examined before normalising, so a pathological single
/// line cannot make this scan expensive.
const OPENING_SCAN_CHARS: usize = 400;

/// Reduce a chat's first user message to the *shape* of how it was started.
///
/// Two runs of the same automation open with the same words and differ only in
/// the parts that change every run: a timestamp, a uuid, a path, a counter.
/// Masking those is what lets one shape be recognised across runs — measured on
/// real data, a single scheduled task appeared as two distinct spellings (39
/// and 32 occurrences) that differed only by a date.
///
/// The result is lowercase, whitespace-collapsed and truncated. It is a
/// grouping key, never shown to anyone, and never reversed.
pub fn normalise_opening(text: &str) -> String {
    let scanned: String = text.chars().take(OPENING_SCAN_CHARS).collect();

    let mut out = String::with_capacity(scanned.len());
    let mut rest = scanned.as_str();

    // Order matters: uuids before dates (a uuid contains dash groups a date
    // pattern could bite into), paths before numbers (a path carries digits),
    // numbers last. Each replacement writes a token that no later rule can
    // match again, which is what keeps this idempotent.
    while !rest.is_empty() {
        if let Some(len) = uuid_prefix_len(rest) {
            out.push_str("<id>");
            rest = &rest[len..];
        } else if let Some(len) = date_prefix_len(rest) {
            out.push_str("<date>");
            rest = &rest[len..];
        } else if let Some(len) = path_prefix_len(rest) {
            out.push_str("<path>");
            rest = &rest[len..];
        } else if let Some(len) = digits_prefix_len(rest) {
            out.push_str("<n>");
            rest = &rest[len..];
        } else {
            let ch = match rest.chars().next() {
                Some(ch) => ch,
                None => break,
            };
            out.push(ch);
            rest = &rest[ch.len_utf8()..];
        }
    }

    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed
        .to_lowercase()
        .chars()
        .take(MAX_OPENING_CHARS)
        .collect()
}

/// Byte length of a `8-4-4-4-12` hex uuid at the start of `s`, if there is one.
fn uuid_prefix_len(s: &str) -> Option<usize> {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let bytes = s.as_bytes();
    let mut at = 0usize;
    for (index, group) in GROUPS.iter().enumerate() {
        if index > 0 {
            if bytes.get(at) != Some(&b'-') {
                return None;
            }
            at += 1;
        }
        for _ in 0..*group {
            match bytes.get(at) {
                Some(byte) if byte.is_ascii_hexdigit() => at += 1,
                _ => return None,
            }
        }
    }
    // A longer hex run means this was not a uuid but some other token.
    match bytes.get(at) {
        Some(byte) if byte.is_ascii_hexdigit() => None,
        _ => Some(at),
    }
}

/// Byte length of a `YYYY-MM-DD` date at the start of `s`, if there is one.
fn date_prefix_len(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let digits = |at: usize, count: usize| {
        (0..count).all(|offset| bytes.get(at + offset).is_some_and(u8::is_ascii_digit))
    };
    if digits(0, 4) && bytes.get(4) == Some(&b'-') && digits(5, 2) && bytes.get(7) == Some(&b'-')
    // A date is exactly ten characters; anything longer is a different token.
        && digits(8, 2)
        && !bytes.get(10).is_some_and(u8::is_ascii_digit)
    {
        Some(10)
    } else {
        None
    }
}

/// Byte length of an absolute path at the start of `s`, if there is one.
///
/// Requires at least one character after the leading slash so a bare `/` (a
/// division sign, a sentence's slash) is left alone.
fn path_prefix_len(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.first() != Some(&b'/') {
        return None;
    }
    let mut at = 1usize;
    while let Some(byte) = bytes.get(at) {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'_' | b'-') {
            at += 1;
        } else {
            break;
        }
    }
    (at > 1).then_some(at)
}

/// Byte length of a run of digits at the start of `s`, if there is one.
fn digits_prefix_len(s: &str) -> Option<usize> {
    let len = s.bytes().take_while(u8::is_ascii_digit).count();
    (len > 0).then_some(len)
}

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
        b.activity_time()
            .cmp(&a.activity_time())
            .then_with(|| a.id.cmp(&b.id))
    });
    (sessions, total)
}

/// Parse a single `<uuid>.jsonl` session file into a [`ClaudeSession`].
///
/// Returns `None` only when the file has no usable id or cannot be read at all.
/// Malformed individual lines are skipped, never fatal.
/// The normalised opening of one chat, read on its own.
///
/// TP-DAILY-27: the graveyard holds records for conversations no drawer is
/// listing right now — a chat closed under a checkout nobody has open still
/// has a headstone. Their openings are therefore never learned by the drawer
/// parse, and without an opening a rule has nothing to judge, so those rows
/// were the ones a person kept seeing.
///
/// Deliberately NOT `parse_session_file`: that reads the file whole to derive
/// a title, and this runs once per headstone at startup. This stops at the
/// first user message — the only line it needs — so a long conversation costs
/// the same as a short one.
pub fn read_opening_for_session(
    projects_dir: &Path,
    project_path: &str,
    session_id: &str,
) -> Option<String> {
    use std::io::BufRead;

    let path = projects_dir
        .join(encode_project_path(project_path))
        .join(format!("{session_id}.jsonl"));
    let file = fs::File::open(path).ok()?;
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let Some(obj) = value.as_object() else {
            continue;
        };
        if obj.get("type").and_then(|t| t.as_str()) != Some("user") {
            continue;
        }
        let opening = extract_user_text(obj).map(|text| normalise_opening(&text))?;
        return (!opening.is_empty()).then_some(opening);
    }
    None
}

/// The jsonl line's own `timestamp` field ("2026-08-19T10:05:30.250Z") as a
/// SystemTime. RFC 3339; anything absent, non-string, unparseable, or
/// pre-epoch is `None` — a bad line must never poison the session (TP-DRAW-15).
fn parse_line_timestamp(obj: &serde_json::Map<String, serde_json::Value>) -> Option<SystemTime> {
    let raw = obj.get("timestamp")?.as_str()?;
    let parsed =
        time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).ok()?;
    let ns = parsed.unix_timestamp_nanos();
    u64::try_from(ns)
        .ok()
        .map(|ns| SystemTime::UNIX_EPOCH + std::time::Duration::from_nanos(ns))
}

/// TP-DRAW-16: `max` keeps the newest timestamp forever, so one corrupt
/// future-dated line would pin the chat at "now" for good. Allow a small
/// clock-skew window; beyond it the timestamp is treated as absent.
const FUTURE_TIMESTAMP_SLACK: std::time::Duration = std::time::Duration::from_secs(5 * 60);

fn accepted_line_timestamp(
    obj: &serde_json::Map<String, serde_json::Value>,
    now: SystemTime,
) -> Option<SystemTime> {
    parse_line_timestamp(obj).filter(|ts| *ts <= now + FUTURE_TIMESTAMP_SLACK)
}

/// TP-DRAW-16, cc-l parity (`_is_noise`): a user line advances the chat clock
/// only when it is the human actually speaking — not an injected `<…>` block
/// and not a continuation/interrupt banner. Tool results never reach here:
/// they carry no text block, so extraction already returns `None` for them.
fn user_text_advances_clock(text: &str) -> bool {
    !text.starts_with('<')
        && !text.starts_with("This session is being continued")
        && !text.starts_with("[Request interrupted")
}

/// TP-DRAW-16, cc-l parity: an assistant line advances the clock only when it
/// carries visible text that is not an injected `<…>` block. While an agent
/// runs, tool_use-only lines stream in constantly; counting them shows every
/// agent chat as "1m ago" forever.
fn assistant_text_advances_clock(text: &str) -> bool {
    !text.starts_with('<')
}

fn parse_session_file(path: &Path) -> Option<ClaudeSession> {
    let id = path.file_stem()?.to_str()?.to_string();
    let content = fs::read_to_string(path).ok()?;
    // Metadata failure must not panic; fall back to the epoch (sorts oldest).
    let last_modified = fs::metadata(path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    let mut custom_title: Option<String> = None;
    let mut last_message_at: Option<SystemTime> = None;
    // Taken once per file: the reference point for the future-date reject.
    let now = SystemTime::now();
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
                // One extraction serves both the title chain and the clock
                // sieve (tool_result-only lines extract to `None`).
                let text = extract_user_text(obj);
                if first_user.is_none() {
                    first_user = text.clone();
                }
                // TP-DRAW-15: the newest message timestamp wins, whatever
                // order the lines were appended in. TP-DRAW-16: but only a
                // substantive human message may advance the clock.
                if text.as_deref().is_some_and(user_text_advances_clock) {
                    if let Some(ts) = accepted_line_timestamp(obj, now) {
                        last_message_at = Some(last_message_at.map_or(ts, |cur| cur.max(ts)));
                    }
                }
            }
            Some("assistant") => {
                msg_count += 1;
                // TP-DRAW-16: tool_use-only assistant lines are machinery,
                // not conversation — only visible text advances the clock.
                if extract_user_text(obj)
                    .as_deref()
                    .is_some_and(assistant_text_advances_clock)
                {
                    if let Some(ts) = accepted_line_timestamp(obj, now) {
                        last_message_at = Some(last_message_at.map_or(ts, |cur| cur.max(ts)));
                    }
                }
            }
            _ => {}
        }
    }

    // The opening is taken before `derive_title` consumes `first_user`: a chat
    // with an `ai-title` never lets its first message reach the title, and
    // those are precisely the chats an automation produces.
    let opening = first_user
        .as_deref()
        .map(normalise_opening)
        .filter(|opening| !opening.is_empty());
    let title = derive_title(custom_title, ai_title, first_user);
    Some(ClaudeSession {
        id,
        title,
        last_modified,
        last_message_at,
        msg_count,
        opening,
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
            encode_project_path("/home/user/projects/x"),
            "-home-user-projects-x"
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

    // ---- TP-DRAW-15: a chat's age is its last message's own time -------------

    #[test]
    fn the_last_message_timestamp_comes_from_the_last_user_or_assistant_line() {
        let tp = TempProjects::new("last-msg");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"summary","summary":"s"}"#,
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T10:05:30.250Z","message":{"content":[{"type":"text","text":"hello back"}]}}"#,
                r#"{"type":"custom-title","customTitle":"t"}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        assert_eq!(sessions.len(), 1);
        // The LAST message line wins, and non-message lines after it change
        // nothing — the reader asks "when did someone last speak here".
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_930_250);
        assert_eq!(sessions[0].last_message_at, Some(expected));
        assert_eq!(sessions[0].activity_time(), expected);
    }

    #[test]
    fn a_transcript_without_messages_carries_no_last_message_time() {
        let tp = TempProjects::new("no-msg");
        let path = tp.write_session(
            "/home/x/proj",
            "bbbb",
            &[r#"{"type":"ai-title","aiTitle":"only a title"}"#],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].last_message_at, None);
        // Honest fallback: with no timestamped message the file's own mtime
        // still answers, so the row is dated rather than blank.
        let mtime = fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(sessions[0].activity_time(), mtime);
    }

    #[test]
    fn a_malformed_timestamp_does_not_poison_the_last_good_one() {
        let tp = TempProjects::new("bad-ts");
        tp.write_session(
            "/home/x/proj",
            "cccc",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"assistant","timestamp":"not-a-date"}"#,
                r#"{"type":"user","message":{"content":"no timestamp field at all"}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        assert_eq!(sessions.len(), 1);
        // Unparseable or absent timestamps are skipped, never zeroed in.
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_600_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    #[test]
    fn the_listing_orders_by_the_message_clock_not_the_file_clock() {
        // Two files written back-to-back (mtimes practically equal, "old"
        // freshly written): the one whose LAST MESSAGE is newer must lead.
        // This is the restart scenario — a resume refreshes mtimes wholesale,
        // and only the message clock keeps the order truthful.
        let tp = TempProjects::new("order");
        // Written and named so every OTHER clock gives the wrong order:
        // z-fresh is written FIRST (older mtime) and sorts LAST by id — only
        // the message clock can put it first.
        tp.write_session(
            "/home/x/proj",
            "z-fresh",
            &[r#"{"type":"user","timestamp":"2026-08-19T00:00:00.000Z","message":{"content":"b"}}"#],
        );
        std::thread::sleep(Duration::from_millis(25));
        tp.write_session(
            "/home/x/proj",
            "a-stale",
            &[r#"{"type":"user","timestamp":"2026-08-01T00:00:00.000Z","message":{"content":"a"}}"#],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let ids: Vec<&str> = sessions.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["z-fresh", "a-stale"]);
    }

    #[test]
    fn an_out_of_order_transcript_still_reports_the_newest_message() {
        // Lines appended out of chronological order (a merge, a repair, an
        // imported chat): the NEWEST message timestamp wins, not the last line.
        let tp = TempProjects::new("ooo");
        tp.write_session(
            "/home/x/proj",
            "dddd",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:05:30.250Z","message":{"content":"late"}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":[{"type":"text","text":"earlier reply"}]}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_930_250);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    // ---- TP-DRAW-16: only substantive messages advance the chat clock --------

    #[test]
    fn tool_use_only_assistant_lines_do_not_advance_the_clock() {
        // The "always 1m ago" bug: while an agent runs, every tool call
        // appends an assistant line with no visible text. Counting those
        // pins the row at "now" forever, so they must not touch the clock.
        let tp = TempProjects::new("tool-only");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T13:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Bash","input":{}}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T13:05:00.000Z"}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_600_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    #[test]
    fn an_assistant_line_with_text_advances_the_clock() {
        // The sieve must not overreach: a real reply is real activity.
        let tp = TempProjects::new("asst-text");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T13:00:00.000Z","message":{"content":[{"type":"text","text":"done"}]}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_144_400_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    #[test]
    fn injected_angle_bracket_blocks_do_not_advance_the_clock() {
        // System reminders and command transcripts arrive as ordinary
        // user/assistant lines whose text starts with "<". cc-l ignores
        // them and so do we.
        let tp = TempProjects::new("angle");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"user","timestamp":"2026-08-19T13:00:00.000Z","message":{"content":"<system-reminder>ping</system-reminder>"}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T13:05:00.000Z","message":{"content":[{"type":"text","text":"<system>injected</system>"}]}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_600_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    #[test]
    fn continuation_and_interrupt_banners_do_not_advance_the_clock() {
        // Compact/resume machinery writes these as user lines; they are not
        // the human speaking (cc-l's noise-prefix parity).
        let tp = TempProjects::new("banner");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"user","timestamp":"2026-08-19T13:00:00.000Z","message":{"content":"This session is being continued from a previous conversation"}}"#,
                r#"{"type":"user","timestamp":"2026-08-19T13:05:00.000Z","message":{"content":"[Request interrupted by user]"}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_600_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    #[test]
    fn tool_result_user_lines_do_not_advance_the_clock() {
        // While an agent runs, tool results stream back as user-typed lines
        // with no text block; they are machinery, not conversation.
        let tp = TempProjects::new("tool-result");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"user","timestamp":"2026-08-19T13:00:00.000Z","message":{"content":[{"type":"tool_result","tool_use_id":"x","content":"ok"}]}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_600_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    #[test]
    fn a_future_dated_line_is_ignored_by_the_clock() {
        // `max` keeps the newest timestamp forever, so a single corrupt
        // future-dated line would pin the chat at "now" for good. Reject
        // anything past the present (plus a small clock-skew allowance).
        let tp = TempProjects::new("future");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":"hi"}}"#,
                r#"{"type":"user","timestamp":"2099-01-01T00:00:00.000Z","message":{"content":"from the future"}}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_600_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
    }

    #[test]
    fn the_clock_matches_the_cc_l_content_sieve_on_a_mixed_transcript() {
        // Fixture parity with `claude-sessions list` (cc-l): on a realistic
        // agent-session tail — real message, then a burst of tool traffic and
        // injected blocks — both tools must date the chat at the last REAL
        // message. cc-l's sieve: user counts when non-noise; assistant counts
        // when it has text not starting with "<".
        let tp = TempProjects::new("ccl-parity");
        tp.write_session(
            "/home/x/proj",
            "aaaa",
            &[
                r#"{"type":"user","timestamp":"2026-08-19T09:00:00.000Z","message":{"content":"start the job"}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T10:00:00.000Z","message":{"content":[{"type":"text","text":"working on it"}]}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T13:00:00.000Z","message":{"content":[{"type":"tool_use","name":"Bash","input":{}}]}}"#,
                r#"{"type":"user","timestamp":"2026-08-19T13:00:05.000Z","message":{"content":[{"type":"tool_result","tool_use_id":"x","content":"out"}]}}"#,
                r#"{"type":"user","timestamp":"2026-08-19T13:01:00.000Z","message":{"content":"<local-command-stdout>ok</local-command-stdout>"}}"#,
                r#"{"type":"assistant","timestamp":"2026-08-19T13:02:00.000Z"}"#,
            ],
        );
        let sessions = read_sessions_for_project(&tp.root, "/home/x/proj");
        // 10:00:00Z — the assistant's last real reply, exactly what cc-l shows.
        let expected = std::time::UNIX_EPOCH + Duration::from_millis(1_787_133_600_000);
        assert_eq!(sessions[0].last_message_at, Some(expected));
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

    // ---- Opening extraction and normalisation (O1-O6) -----------------------

    /// O4: the whole point of normalising. Two runs of one automation differ
    /// only in the parts that change every run; masking those is what makes
    /// them one shape. Measured on real data: a single scheduled task appeared
    /// as two spellings, 39 and 32 times, differing only by a date.
    #[test]
    fn normalising_masks_the_parts_that_change_every_run() {
        let first = normalise_opening(
            "<scheduled-task name=\"sor\" file=\"/home/user/.claude/x.md\" at=\"2026-08-17\" run=42>",
        );
        let second = normalise_opening(
            "<scheduled-task name=\"sor\" file=\"/home/user/.claude/x.md\" at=\"2026-07-03\" run=7>",
        );
        assert_eq!(first, second, "two runs of one task must share one shape");
        assert!(first.contains("<date>"), "{first}");
        assert!(first.contains("<path>"), "{first}");
        assert!(first.contains("<n>"), "{first}");
    }

    /// O4b: a uuid is masked as one token rather than being chopped into a
    /// date and several numbers, which would make two runs disagree.
    #[test]
    fn normalising_masks_a_uuid_as_one_token() {
        let opening = normalise_opening("resume 3f33db7a-797d-4bcc-b60a-fabb4ada10ae now");
        assert_eq!(opening, "resume <id> now");
    }

    /// O5: normalising is idempotent. If it were not, the same chat could
    /// produce two different shapes depending on how often it was processed,
    /// and the repeat counter would split one automation across two buckets.
    #[test]
    fn normalising_is_idempotent() {
        for raw in [
            "<scheduled-task name=\"x\" file=\"/a/b.md\" at=\"2026-08-17\">",
            "resume 3f33db7a-797d-4bcc-b60a-fabb4ada10ae",
            "plain words with 42 numbers",
            "",
        ] {
            let once = normalise_opening(raw);
            assert_eq!(normalise_opening(&once), once, "not idempotent for {raw:?}");
        }
    }

    /// O4c: masking must not eat ordinary text. A guard that flattens prose
    /// makes unrelated chats collide and hides real conversations.
    #[test]
    fn normalising_leaves_ordinary_prose_recognisable() {
        let opening = normalise_opening("Bu sohbette neler yaptık? Özetle lütfen.");
        assert_eq!(opening, "bu sohbette neler yaptık? özetle lütfen.");
    }

    /// O4d: a lone slash is a slash, not a path. Requiring a character after
    /// it keeps division signs and sentence slashes out of the mask.
    #[test]
    fn a_bare_slash_is_not_a_path() {
        assert_eq!(normalise_opening("a / b"), "a / b");
        assert_eq!(normalise_opening("see /tmp/x now"), "see <path> now");
    }

    /// TP-DAILY-27 (O1/O3/O5): a single chat's opening, read on its own.
    ///
    /// The graveyard needs this for conversations no drawer is listing: the
    /// headstone outlives the drawer, and a row with no opening is a row no
    /// rule can judge — which is exactly the set a person kept seeing. A
    /// missing directory, a missing file and a chat that never spoke all
    /// answer `None` rather than costing anything.
    #[test]
    fn one_chats_opening_can_be_read_without_parsing_the_whole_file() {
        let temp = TempProjects::new("lone-opening");
        temp.write_session(
            "/repo/thing",
            "s1",
            &[
                r#"{"type":"user","message":{"content":"Review this change for security vulnerabilities. Changed files: a.rs"}}"#,
                r#"{"type":"assistant","message":{"content":"ok"}}"#,
                r#"{"type":"user","message":{"content":"and a second message that must not win"}}"#,
            ],
        );

        assert_eq!(
            read_opening_for_session(&temp.root, "/repo/thing", "s1").as_deref(),
            Some("review this change for security vulnerabilities. changed files: a.rs"),
            "the FIRST user message is the opening"
        );
        assert_eq!(
            read_opening_for_session(&temp.root, "/repo/thing", "missing"),
            None,
            "a chat with no file costs nothing"
        );
        assert_eq!(
            read_opening_for_session(&temp.root, "/repo/elsewhere", "s1"),
            None,
            "and neither does a directory nothing was written to"
        );

        temp.write_session(
            "/repo/thing",
            "quiet",
            &[r#"{"type":"assistant","message":{"content":"nobody asked anything"}}"#],
        );
        assert_eq!(
            read_opening_for_session(&temp.root, "/repo/thing", "quiet"),
            None,
            "a chat that never spoke has no shape to remember"
        );
    }

    /// O1/O2: the opening is read from the first user message and survives an
    /// `ai-title`. This is the case that matters most: an automation's chat is
    /// titled by the model, so the first message never reaches the title, and
    /// deriving the opening from the title would lose exactly those chats.
    #[test]
    fn the_opening_is_read_even_when_an_ai_title_wins_the_title() {
        let temp = TempProjects::new("opening-ai-title");
        temp.write_session(
            "/p",
            "s1",
            &[
                r#"{"type":"user","message":{"content":"<scheduled-task name=\"nightly\">"}}"#,
                r#"{"type":"assistant","message":{"content":"ok"}}"#,
                r#"{"type":"ai-title","aiTitle":"Nightly report"}"#,
            ],
        );

        let sessions = read_sessions_for_project(&temp.root, "/p");
        assert_eq!(sessions.len(), 1);
        assert_eq!(
            sessions[0].title, "Nightly report",
            "title precedence is unchanged"
        );
        assert_eq!(
            sessions[0].opening.as_deref(),
            Some("<scheduled-task name=\"nightly\">"),
            "the opening must survive an ai-title"
        );
    }

    /// O3: adding the opening must not move the title. Titles are what people
    /// read; a silent change there would be a regression nobody asked for.
    #[test]
    fn adding_the_opening_does_not_move_the_title() {
        assert_eq!(
            derive_title(
                Some("custom".into()),
                Some("ai".into()),
                Some("first".into())
            ),
            "custom"
        );
        assert_eq!(
            derive_title(None, Some("ai".into()), Some("first".into())),
            "ai"
        );
        assert_eq!(derive_title(None, None, Some("first".into())), "first");
        assert_eq!(derive_title(None, None, None), UNTITLED);
    }

    /// O6: a chat with nothing readable gets no opening at all. Inventing a
    /// shape for it would put unrelated chats in one bucket and let the repeat
    /// rule hide them together.
    #[test]
    fn a_chat_with_no_readable_first_message_has_no_opening() {
        let temp = TempProjects::new("opening-none");
        temp.write_session(
            "/p",
            "s1",
            &[
                r#"{"type":"assistant","message":{"content":"hi"}}"#,
                r#"{"type":"user","message":{"content":"   "}}"#,
            ],
        );

        let sessions = read_sessions_for_project(&temp.root, "/p");
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].opening, None);
    }
}
