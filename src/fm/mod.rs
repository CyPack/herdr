//! Native file manager — client-side directory model.
//!
//! This is the L4 "domain" of the native file manager: reading a directory,
//! ordering its entries, and holding the cursor/visibility state for one browsed
//! folder. Following the runtime/client boundary (AGENTS.md), it is pure,
//! PTY-free presentation state that lives entirely on the TUI/client side — it
//! spawns no processes, holds no runtime state, touches no network, and never
//! panics on unreadable directories (mirrors `claude_sessions`). Rendering (A2)
//! and navigation input (A3) consume this model; they do not live here.
//!
//! Design docs: `.local/prd/native-fm/` (A1-fs-reader.md, 00-MODULE-TREE.md).

mod natsort;

use std::path::{Path, PathBuf};

/// One entry in a browsed directory. Pure, cloneable data for rendering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// File name (the final path component), as shown to the user.
    pub name: String,
    /// Absolute (or `dir`-relative) path to the entry.
    pub path: PathBuf,
    /// Whether this entry resolves to a directory (symlinks are followed).
    pub is_dir: bool,
}

/// Read the immediate children of `dir`: directories first, then files, each
/// group in natural (human) order. When `show_hidden` is false, dot-prefixed
/// names are dropped.
///
/// Never panics. A missing or unreadable directory yields an empty `Vec`, and
/// individually unreadable entries (or non-UTF-8 names) are skipped.
pub fn read_dir_entries(dir: &Path, show_hidden: bool) -> Vec<FileEntry> {
    let read = match std::fs::read_dir(dir) {
        Ok(read) => read,
        Err(err) => {
            // A directory that does not exist yet is a normal state; only log the
            // genuinely unexpected failures, and never spam.
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(?dir, %err, "fm: read_dir failed");
            }
            return Vec::new();
        }
    };

    let mut entries: Vec<FileEntry> = read
        .flatten()
        .filter_map(|entry| {
            // Non-UTF-8 names cannot be rendered as a `str`; skip them in v1.
            let name = entry.file_name().to_str()?.to_string();
            if !show_hidden && name.starts_with('.') {
                return None;
            }
            Some(FileEntry {
                is_dir: entry_is_dir(&entry),
                path: entry.path(),
                name,
            })
        })
        .collect();

    sort_entries(&mut entries);
    entries
}

/// Decide whether a directory entry counts as a directory for the file list.
///
/// `file_type()` comes straight from the `readdir` result (no extra syscall for
/// real files/dirs). Only symlinks need a follow-up `stat` to resolve their
/// target; a broken symlink resolves to `false` (listed as a file).
fn entry_is_dir(entry: &std::fs::DirEntry) -> bool {
    match entry.file_type() {
        Ok(ft) if ft.is_symlink() => entry.path().is_dir(),
        Ok(ft) => ft.is_dir(),
        Err(_) => entry.path().is_dir(),
    }
}

/// Order entries directories-first, then by natural (case-insensitive) name,
/// with the raw name as a stable tiebreaker for deterministic rendering/tests.
fn sort_entries(entries: &mut [FileEntry]) {
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| natsort::natsort(a.name.as_bytes(), b.name.as_bytes(), true))
            .then_with(|| a.name.cmp(&b.name))
    });
}

/// Pure, TUI-only browsing state for one directory pane: the current directory,
/// its ordered entries, the cursor row, and hidden-file visibility. No PTY, no
/// runtime, no async — constructible and assertable without a terminal.
#[derive(Debug, Clone)]
pub struct FmState {
    /// The directory currently being browsed.
    pub cwd: PathBuf,
    /// Ordered entries of `cwd` (see [`read_dir_entries`]).
    pub entries: Vec<FileEntry>,
    /// Index of the highlighted row; always within `[0, entries.len())`, or 0
    /// when the directory is empty.
    pub cursor: usize,
    /// Whether dot-prefixed entries are shown.
    pub show_hidden: bool,
}

impl FmState {
    /// Open `cwd` (hidden files off) and read its entries, cursor at the top.
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self::with_hidden(cwd, false)
    }

    /// Open `cwd` with an explicit hidden-file setting.
    pub fn with_hidden(cwd: impl Into<PathBuf>, show_hidden: bool) -> Self {
        let cwd = cwd.into();
        let entries = read_dir_entries(&cwd, show_hidden);
        Self {
            cwd,
            entries,
            cursor: 0,
            show_hidden,
        }
    }

    /// Re-read the current directory, keeping `show_hidden` and clamping the
    /// cursor into the (possibly shrunk) entry range.
    pub fn reload(&mut self) {
        self.entries = read_dir_entries(&self.cwd, self.show_hidden);
        self.clamp_cursor();
    }

    /// Toggle hidden-file visibility and re-read the directory.
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
        self.reload();
    }

    /// The currently highlighted entry, if any.
    pub fn selected(&self) -> Option<&FileEntry> {
        self.entries.get(self.cursor)
    }

    /// Move the cursor down one row, stopping at the last entry.
    pub fn move_down(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
        }
    }

    /// Move the cursor up one row, stopping at the top.
    pub fn move_up(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// Descend into the selected entry when it is a directory, re-reading its
    /// contents with the cursor back at the top. A no-op when the selection is a
    /// file (or the directory is empty).
    pub fn enter(&mut self) {
        let target = self
            .selected()
            .filter(|entry| entry.is_dir)
            .map(|entry| entry.path.clone());
        if let Some(path) = target {
            self.cwd = path;
            self.cursor = 0;
            self.reload();
        }
    }

    /// Go to the parent directory, re-reading its contents with the cursor at the
    /// top. A no-op at the filesystem root (no parent).
    pub fn leave(&mut self) {
        if let Some(parent) = self.cwd.parent() {
            self.cwd = parent.to_path_buf();
            self.cursor = 0;
            self.reload();
        }
    }

    /// Force the cursor back into `[0, entries.len())` (0 when empty).
    fn clamp_cursor(&mut self) {
        if self.entries.is_empty() {
            self.cursor = 0;
        } else if self.cursor >= self.entries.len() {
            self.cursor = self.entries.len() - 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

    fn unique() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, AtomicOrdering::Relaxed)
    }

    /// Isolated temp directory, recursively removed on drop. Never touches any
    /// real user directory.
    struct TempDir {
        root: PathBuf,
    }

    impl TempDir {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "herdr-fm-test-{}-{}-{}",
                std::process::id(),
                tag,
                unique()
            ));
            fs::create_dir_all(&root).expect("create temp root");
            Self { root }
        }

        fn file(&self, name: &str) {
            fs::write(self.root.join(name), b"x").expect("write temp file");
        }

        fn dir(&self, name: &str) {
            fs::create_dir_all(self.root.join(name)).expect("create temp dir");
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn names(entries: &[FileEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.name.as_str()).collect()
    }

    // T-A1.2a: directories first, each group in natural order. Cross-check
    // reference: `ls --group-directories-first -v` yields the same order.
    #[test]
    fn dirs_first_then_natural_order() {
        let td = TempDir::new("order");
        td.file("file10.txt");
        td.file("file2.txt");
        td.dir("beta");
        td.dir("alpha10");
        td.dir("alpha2");
        let entries = read_dir_entries(&td.root, false);
        assert_eq!(
            names(&entries),
            vec!["alpha2", "alpha10", "beta", "file2.txt", "file10.txt"]
        );
    }

    // T-A1.2b: hidden entries are dropped unless requested.
    #[test]
    fn hidden_entries_toggle() {
        let td = TempDir::new("hidden");
        td.file("visible.txt");
        td.file(".secret");
        td.dir(".git");
        td.dir("src");
        assert_eq!(
            names(&read_dir_entries(&td.root, false)),
            vec!["src", "visible.txt"]
        );
        assert_eq!(
            names(&read_dir_entries(&td.root, true)),
            vec![".git", "src", ".secret", "visible.txt"]
        );
    }

    // T-A1.2c: a missing directory yields an empty list, no panic.
    #[test]
    fn missing_directory_is_empty_and_panic_free() {
        let td = TempDir::new("missing");
        let entries = read_dir_entries(&td.root.join("does-not-exist"), false);
        assert!(entries.is_empty());
    }

    // T-A1.2d: a symlink to a directory is listed and sorted as a directory.
    #[cfg(unix)]
    #[test]
    fn symlink_to_directory_counts_as_directory() {
        let td = TempDir::new("symlink");
        td.dir("target");
        td.file("zzz.txt");
        std::os::unix::fs::symlink(td.root.join("target"), td.root.join("link"))
            .expect("create symlink");
        let entries = read_dir_entries(&td.root, false);
        assert_eq!(names(&entries), vec!["link", "target", "zzz.txt"]);
        let link = entries
            .iter()
            .find(|e| e.name == "link")
            .expect("link entry");
        assert!(link.is_dir);
    }

    // T-A1.2d: unicode / emoji names survive intact.
    #[test]
    fn unicode_and_emoji_names_survive() {
        let td = TempDir::new("unicode");
        td.file("café.txt");
        td.file("naïve.md");
        td.dir("smile-dir");
        let entries = read_dir_entries(&td.root, false);
        assert!(entries.iter().any(|e| e.name == "café.txt"));
        assert!(entries.iter().any(|e| e.name == "naïve.md"));
        assert!(entries.iter().any(|e| e.name == "smile-dir" && e.is_dir));
    }

    // T-A1.3a: opening a directory puts the cursor at the top, dir first.
    #[test]
    fn fmstate_opens_with_cursor_at_top() {
        let td = TempDir::new("state-open");
        td.file("a.txt");
        td.dir("d");
        let st = FmState::new(&td.root);
        assert_eq!(st.cursor, 0);
        assert_eq!(st.entries.len(), 2);
        assert_eq!(st.selected().map(|e| e.name.as_str()), Some("d"));
    }

    // T-A1.3b: the cursor is always clamped into range (empty, end, shrink).
    #[test]
    fn cursor_stays_in_range() {
        let td = TempDir::new("state-clamp");

        // Empty directory: cursor pinned at 0, moving down is a no-op.
        let mut st = FmState::new(&td.root);
        assert_eq!(st.cursor, 0);
        st.move_down();
        assert_eq!(st.cursor, 0);
        assert!(st.selected().is_none());

        // Populate and drive the cursor to the last entry.
        td.file("a");
        td.file("b");
        td.file("c");
        st.reload();
        assert_eq!(st.entries.len(), 3);
        st.move_down();
        st.move_down();
        st.move_down();
        assert_eq!(st.cursor, 2);

        // Shrinking the directory re-clamps the cursor on reload.
        fs::remove_file(td.root.join("b")).expect("remove b");
        fs::remove_file(td.root.join("c")).expect("remove c");
        st.reload();
        assert_eq!(st.entries.len(), 1);
        assert_eq!(st.cursor, 0);
    }

    // T-A1.3c: toggling hidden re-reads and changes what is visible.
    #[test]
    fn toggle_hidden_reveals_and_hides_dotfiles() {
        let td = TempDir::new("state-hidden");
        td.file("shown");
        td.file(".hidden");
        let mut st = FmState::new(&td.root);
        assert_eq!(st.entries.len(), 1);
        st.toggle_hidden();
        assert!(st.show_hidden);
        assert_eq!(st.entries.len(), 2);
        st.toggle_hidden();
        assert!(!st.show_hidden);
        assert_eq!(st.entries.len(), 1);
    }

    // TP-A3.1: entering a selected directory reads its contents, cursor at top.
    #[test]
    fn enter_descends_into_selected_directory() {
        let td = TempDir::new("enter");
        td.dir("sub");
        td.file("top.txt");
        fs::write(td.root.join("sub").join("child.txt"), b"x").expect("write child");
        let mut st = FmState::new(&td.root);
        // Directories sort first, so "sub" is selected at the top.
        assert_eq!(st.selected().map(|e| e.name.as_str()), Some("sub"));

        st.enter();

        assert_eq!(st.cwd, td.root.join("sub"));
        assert_eq!(st.cursor, 0);
        assert!(st.entries.iter().any(|e| e.name == "child.txt"));
    }

    // TP-A3.2: entering a file selection is a no-op.
    #[test]
    fn enter_on_file_is_noop() {
        let td = TempDir::new("enterfile");
        td.file("only.txt");
        let mut st = FmState::new(&td.root);
        assert_eq!(st.selected().map(|e| e.name.as_str()), Some("only.txt"));

        let before = st.cwd.clone();
        st.enter();

        assert_eq!(st.cwd, before, "entering a file does not change directory");
    }

    // TP-A3.3: leaving goes to the parent directory, cursor at top.
    #[test]
    fn leave_ascends_to_parent() {
        let td = TempDir::new("leave");
        td.dir("sub");
        let mut st = FmState::new(td.root.join("sub"));

        st.leave();

        assert_eq!(st.cwd, td.root);
        assert_eq!(st.cursor, 0);
    }

    // TP-A3.4: leaving at the filesystem root is a no-op (no panic).
    #[test]
    fn leave_at_root_is_noop() {
        let mut st = FmState::new("/");
        st.leave();
        assert_eq!(st.cwd, PathBuf::from("/"));
    }
}
