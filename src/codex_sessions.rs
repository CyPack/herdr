//! The Codex transcript store, read the way `claude_sessions` reads Claude's.
//!
//! Codex writes one rollout per conversation:
//! `~/.codex/sessions/<YYYY>/<MM>/<DD>/rollout-<stamp>-<uuid>.jsonl`. The first
//! line is `session_meta` (id, cwd, timestamp); a person's turns arrive as
//! `event_msg` lines whose payload is `user_message`; the model's as
//! `agent_message`. Every line carries its own RFC 3339 `timestamp`.
//!
//! TP-CODEX-STORE-01: nothing else in herdr knew this store existed. A Codex
//! chat could be sighted by a pane while it ran, and that was the whole of
//! what a drawer could ever say about it — `019cbcf7 · codex` — and only while
//! the ledger remembered the sighting. Four hundred and twenty-two rollouts
//! stood on the reporting machine, reachable from no drawer and no re-home.
//!
//! The store is large where Claude's is wide: 3.8 GB in 422 files on the
//! reporting machine, 155 of them over 5 MB (a rollout keeps the model's
//! reasoning). So NO rollout is ever read whole. The head is read up to
//! [`HEAD_BUDGET`] bytes — enough for the meta line and the first user turn —
//! and the tail up to [`TAIL_BUDGET`] bytes for the newest timestamp. Both
//! are cached by (mtime, size), so an unchanged store costs one stat per file.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Seek as _, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// One Codex conversation as the drawer needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexSession {
    /// The `session_meta` id, also the tail of the rollout's file name.
    pub id: String,
    /// Where the conversation stood — the drawer files it by this.
    pub cwd: PathBuf,
    /// The person's first turn, first line, trimmed for a row; `(untitled)`
    /// when the head holds no user turn yet.
    pub title: String,
    /// The shape of the first user turn (`claude_sessions::normalise_opening`).
    pub opening: Option<String>,
    /// File mtime — an upper bound on activity.
    pub last_modified: SystemTime,
    /// The newest timestamped line in the tail, read from the line itself.
    /// `None` when the tail holds no timestamp (a truncated or foreign file).
    pub last_message_at: Option<SystemTime>,
}

/// Bytes read from the front of a rollout: the meta line is a few hundred
/// bytes and a first user turn a few thousand; the developer preamble Codex
/// injects before it can run to tens of KB.
const HEAD_BUDGET: u64 = 96 * 1024;
/// Bytes read from the back of a rollout: one `event_msg` line with its
/// timestamp, with room for a large function-call output standing after it.
const TAIL_BUDGET: u64 = 16 * 1024;
/// Rows are one line; a first turn that runs on is cut here.
const TITLE_MAX_CHARS: usize = 80;
/// The store is three directories deep; a symlink loop is not a store.
const MAX_WALK_DEPTH: usize = 5;

/// Parse cache keyed by (mtime, size): an unchanged rollout is never re-read.
pub type CodexParseCache = HashMap<PathBuf, ((SystemTime, u64), CodexSession)>;

/// `$CODEX_HOME/sessions`, else `~/.codex/sessions`.
pub(crate) fn codex_sessions_dir(env: impl Fn(&str) -> Option<OsString>) -> Option<PathBuf> {
    if let Some(home) = env("CODEX_HOME") {
        return Some(PathBuf::from(home).join("sessions"));
    }
    env("HOME").map(|home| PathBuf::from(home).join(".codex").join("sessions"))
}

pub fn default_codex_sessions_dir() -> Option<PathBuf> {
    codex_sessions_dir(|key| std::env::var_os(key))
}

/// Every rollout under `sessions_dir` with the metadata the cache is keyed by.
/// One `read_dir` per directory and one stat per file; no file is opened.
fn rollout_files(sessions_dir: &Path) -> Vec<(SystemTime, u64, PathBuf)> {
    let mut out = Vec::new();
    let mut stack = vec![(sessions_dir.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = fs::metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                if depth < MAX_WALK_DEPTH {
                    stack.push((path, depth + 1));
                }
                continue;
            }
            if !is_rollout_name(&path) {
                continue;
            }
            out.push((meta.modified().unwrap_or(UNIX_EPOCH), meta.len(), path));
        }
    }
    out
}

fn is_rollout_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("rollout-") && name.ends_with(".jsonl"))
}

/// The uuid a rollout's file name ends with — the id's fallback when the
/// meta line is unreadable, and the by-id road's file test.
fn id_from_file_name(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let chars: Vec<char> = stem.chars().collect();
    if chars.len() < 37 || chars[chars.len() - 37] != '-' {
        return None;
    }
    let tail: String = chars[chars.len() - 36..].iter().collect();
    tail.chars()
        .all(|c| c.is_ascii_hexdigit() || c == '-')
        .then_some(tail)
}

/// Read up to `budget` bytes from the front of `path`.
fn read_head(path: &Path, budget: u64) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    file.by_ref().take(budget).read_to_end(&mut buf).ok()?;
    Some(String::from_utf8_lossy(&buf).into_owned())
}

/// Read up to `budget` bytes from the back of `path`, from a line boundary.
fn read_tail(path: &Path, size: u64, budget: u64) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let start = size.saturating_sub(budget);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    // The first line is a fragment unless the read started at 0.
    Some(if start == 0 {
        text
    } else {
        text.split_once('\n')
            .map(|(_, rest)| rest.to_string())
            .unwrap_or_default()
    })
}

fn line_timestamp(obj: &serde_json::Value, now: SystemTime) -> Option<SystemTime> {
    let raw = obj.get("timestamp")?.as_str()?;
    let parsed =
        time::OffsetDateTime::parse(raw, &time::format_description::well_known::Rfc3339).ok()?;
    let ts = SystemTime::from(parsed);
    // A clock that ran ahead when the line was written is not "the future";
    // it is rejected the way `claude_sessions` rejects it.
    (ts <= now).then_some(ts)
}

/// The person's first turn, as the drawer shows it.
fn title_from_turn(text: &str) -> String {
    let first_line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("");
    let mut title: String = first_line.chars().take(TITLE_MAX_CHARS).collect();
    if first_line.chars().count() > TITLE_MAX_CHARS {
        title.push('…');
    }
    if title.is_empty() {
        "(untitled)".to_string()
    } else {
        title
    }
}

/// Parse one rollout within the read budgets.
fn parse_rollout(path: &Path, size: u64, last_modified: SystemTime) -> Option<CodexSession> {
    let head = read_head(path, HEAD_BUDGET)?;
    let now = SystemTime::now();
    let mut id: Option<String> = None;
    let mut cwd: Option<PathBuf> = None;
    let mut first_turn: Option<String> = None;
    let mut last_message_at: Option<SystemTime> = None;

    for line in head.lines() {
        let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if let Some(ts) = line_timestamp(&obj, now) {
            last_message_at = Some(last_message_at.map_or(ts, |cur| cur.max(ts)));
        }
        let payload = obj.get("payload");
        match obj.get("type").and_then(|t| t.as_str()) {
            Some("session_meta") => {
                if let Some(payload) = payload {
                    id = payload
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(str::to_string);
                    cwd = payload
                        .get("cwd")
                        .and_then(|v| v.as_str())
                        .map(PathBuf::from);
                }
            }
            Some("event_msg") if first_turn.is_none() => {
                let is_user = payload.and_then(|p| p.get("type")).and_then(|t| t.as_str())
                    == Some("user_message");
                if is_user {
                    first_turn = payload
                        .and_then(|p| p.get("message"))
                        .and_then(|m| m.as_str())
                        .map(str::to_string);
                }
            }
            _ => {}
        }
        // No early break: the newest timestamp lives on the LAST line the head
        // holds, so the scan runs the whole (budgeted) head even after the
        // meta and the first turn are in hand — stopping at the first turn
        // was measured to freeze the clock at the person's own message.
    }

    // The tail is only worth a read when the head did not already reach it.
    if size > HEAD_BUDGET {
        if let Some(tail) = read_tail(path, size, TAIL_BUDGET) {
            for line in tail.lines() {
                let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
                    continue;
                };
                if let Some(ts) = line_timestamp(&obj, now) {
                    last_message_at = Some(last_message_at.map_or(ts, |cur| cur.max(ts)));
                }
            }
        }
    }

    let id = id.or_else(|| id_from_file_name(path))?;
    // No cwd, no drawer: a rollout the store cannot place is not a row.
    let cwd = cwd?;
    let title = first_turn
        .as_deref()
        .map(title_from_turn)
        .unwrap_or_else(|| "(untitled)".into());
    let opening = first_turn
        .as_deref()
        .map(crate::claude_sessions::normalise_opening);
    Some(CodexSession {
        id,
        cwd,
        title,
        opening,
        last_modified,
        last_message_at,
    })
}

fn parse_cached(
    cache: &mut CodexParseCache,
    modified: SystemTime,
    size: u64,
    path: PathBuf,
) -> Option<CodexSession> {
    let key = (modified, size);
    if let Some((cached_key, session)) = cache.get(&path) {
        if *cached_key == key {
            return Some(session.clone());
        }
    }
    let session = parse_rollout(&path, size, modified)?;
    cache.insert(path, (key, session.clone()));
    Some(session)
}

/// The newest `limit` conversations that stood in `cwd`, newest first.
///
/// Every rollout's head is read once (then cached), because the cwd is inside
/// the file and nowhere in the name; the walk itself opens nothing.
pub fn read_recent_sessions_for_cwd_cached(
    sessions_dir: &Path,
    cwd: &Path,
    limit: usize,
    cache: &mut CodexParseCache,
) -> Vec<CodexSession> {
    let wanted = crate::persist::workspace_chats::ledger_key(cwd);
    let mut files = rollout_files(sessions_dir);
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(&b.2)));
    let mut out = Vec::new();
    for (modified, size, path) in files {
        if out.len() >= limit {
            break;
        }
        let Some(session) = parse_cached(cache, modified, size, path) else {
            continue;
        };
        if crate::persist::workspace_chats::ledger_key(&session.cwd) == wanted {
            out.push(session);
        }
    }
    out.sort_by(|a, b| {
        b.last_message_at
            .unwrap_or(b.last_modified)
            .cmp(&a.last_message_at.unwrap_or(a.last_modified))
            .then_with(|| a.id.cmp(&b.id))
    });
    out
}

/// One conversation by id, wherever it stands in the store. The id is the
/// tail of the file name, so this opens exactly one file.
pub fn read_session_by_id_cached(
    sessions_dir: &Path,
    session_id: &str,
    cache: &mut CodexParseCache,
) -> Option<CodexSession> {
    if session_id.len() != 36 || session_id.contains(['/', '\\']) {
        return None;
    }
    let suffix = format!("-{session_id}.jsonl");
    let (modified, size, path) = rollout_files(sessions_dir)
        .into_iter()
        .find(|(_, _, path)| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(&suffix))
        })?;
    parse_cached(cache, modified, size, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn store(tag: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "herdr-codex-store-{}-{}-{}",
            std::process::id(),
            tag,
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("2026/08/29")).expect("store dirs");
        root
    }

    fn write_rollout(root: &Path, id: &str, cwd: &str, user_turn: Option<&str>) -> PathBuf {
        let path = root
            .join("2026/08/29")
            .join(format!("rollout-2026-08-29T10-00-00-{id}.jsonl"));
        let mut file = fs::File::create(&path).expect("rollout");
        writeln!(
            file,
            r#"{{"timestamp":"2026-08-29T10:00:00.000Z","type":"session_meta","payload":{{"id":"{id}","timestamp":"2026-08-29T10:00:00.000Z","cwd":"{cwd}","originator":"codex_cli_rs"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"timestamp":"2026-08-29T10:00:01.000Z","type":"response_item","payload":{{"type":"message","role":"developer","content":[{{"type":"input_text","text":"<permissions instructions>"}}]}}}}"#
        )
        .unwrap();
        if let Some(turn) = user_turn {
            let turn = turn.replace('"', "\\\"").replace('\n', "\\n");
            writeln!(
                file,
                r#"{{"timestamp":"2026-08-29T10:00:02.000Z","type":"event_msg","payload":{{"type":"user_message","message":"{turn}","images":[]}}}}"#
            )
            .unwrap();
            writeln!(
                file,
                r#"{{"timestamp":"2026-08-29T10:05:00.000Z","type":"event_msg","payload":{{"type":"agent_message","message":"done"}}}}"#
            )
            .unwrap();
        }
        path
    }

    const ID: &str = "019cbcf7-7800-7002-a7e4-562e4595cb84";

    // TP-CODEX-STORE-01: a rollout yields what a drawer row needs — id, the
    // directory it stood in, the person's first turn as its title, and a
    // clock read from the lines themselves.
    #[test]
    fn a_rollout_yields_id_cwd_title_and_clocks() {
        let root = store("parse");
        write_rollout(
            &root,
            ID,
            "/home/user/projects/sor",
            Some("SOR dosyalarını yeniden adlandır\nikinci satır"),
        );
        let mut cache = CodexParseCache::default();
        let sessions = read_recent_sessions_for_cwd_cached(
            &root,
            Path::new("/home/user/projects/sor"),
            12,
            &mut cache,
        );
        assert_eq!(sessions.len(), 1);
        let s = &sessions[0];
        assert_eq!(s.id, ID);
        assert_eq!(
            s.title, "SOR dosyalarını yeniden adlandır",
            "first line only"
        );
        assert!(s.opening.is_some());
        let expected = time::OffsetDateTime::parse(
            "2026-08-29T10:05:00.000Z",
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        assert_eq!(
            s.last_message_at,
            Some(SystemTime::from(expected)),
            "the newest line's own timestamp"
        );
        let _ = fs::remove_dir_all(&root);
    }

    // TP-CODEX-STORE-01: the cwd filter is the whole point — a rollout that
    // stood elsewhere is not this drawer's row, and one with no user turn is
    // listed but honestly untitled.
    #[test]
    fn rollouts_are_filed_by_cwd_and_an_empty_one_is_untitled() {
        let root = store("cwd");
        write_rollout(&root, ID, "/home/user/a", None);
        write_rollout(
            &root,
            "019cbcf7-7800-7002-a7e4-562e4595cb85",
            "/home/user/b",
            Some("b"),
        );
        let mut cache = CodexParseCache::default();
        let a =
            read_recent_sessions_for_cwd_cached(&root, Path::new("/home/user/a"), 12, &mut cache);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].title, "(untitled)");
        let b =
            read_recent_sessions_for_cwd_cached(&root, Path::new("/home/user/b"), 12, &mut cache);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].title, "b");
        let _ = fs::remove_dir_all(&root);
    }

    // TP-CODEX-STORE-02: the by-id road opens the one file whose name carries
    // the id, and refuses a shape that is not an id.
    #[test]
    fn a_rollout_is_found_by_its_id_and_a_non_id_is_refused() {
        let root = store("by-id");
        write_rollout(&root, ID, "/home/user/x", Some("find me"));
        let mut cache = CodexParseCache::default();
        let found = read_session_by_id_cached(&root, ID, &mut cache).expect("found by id");
        assert_eq!(found.title, "find me");
        assert!(read_session_by_id_cached(&root, "../etc/passwd", &mut cache).is_none());
        assert!(read_session_by_id_cached(&root, "nope", &mut cache).is_none());
        let _ = fs::remove_dir_all(&root);
    }

    // The cache is honoured: an unchanged rollout is answered without a read.
    #[test]
    fn an_unchanged_rollout_is_served_from_the_cache() {
        let root = store("cache");
        let path = write_rollout(&root, ID, "/home/user/c", Some("cached"));
        let mut cache = CodexParseCache::default();
        let _ = read_session_by_id_cached(&root, ID, &mut cache);
        // Poison the cached copy; a second read that hit the disk would undo it.
        if let Some((_, session)) = cache.get_mut(&path) {
            session.title = "from cache".into();
        }
        let again = read_session_by_id_cached(&root, ID, &mut cache).unwrap();
        assert_eq!(again.title, "from cache");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_id_falls_back_to_the_file_name() {
        let path =
            Path::new("/x/rollout-2026-08-29T10-00-00-019cbcf7-7800-7002-a7e4-562e4595cb84.jsonl");
        assert_eq!(id_from_file_name(path).as_deref(), Some(ID));
        assert_eq!(id_from_file_name(Path::new("/x/rollout-short.jsonl")), None);
    }
}
