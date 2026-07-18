//! Canonical semantic classification for native file-manager entries
//! (TP-FIP-ICON-01..05). Prepared once during directory snapshot; render and
//! capability checks derive from this single source of truth.

/// Canonical filesystem kind of one visible entry. Client-local presentation
/// state: never persisted and never added to the server protocol.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEntryKind {
    Directory,
    RegularFile,
    SymlinkDirectory,
    SymlinkFile,
    BrokenSymlink,
    UnsupportedSpecial,
}

impl FileEntryKind {
    pub fn is_directory_target(self) -> bool {
        matches!(self, Self::Directory | Self::SymlinkDirectory)
    }

    pub fn supports_native_operation(self) -> bool {
        !matches!(self, Self::BrokenSymlink | Self::UnsupportedSpecial)
    }

    pub fn supports_agent_reference(self) -> bool {
        self.supports_native_operation()
    }
}

/// Pure visual class of one entry: kind always wins over name; exact
/// well-known names win over the lowercase final extension; every unmatched
/// regular file is `Generic` (TP-FIP-ICON-01..07, ICON-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualClass {
    Directory,
    SymlinkDirectory,
    SymlinkFile,
    Broken,
    Special,
    VersionControl,
    BuildPackage,
    SourceCode,
    WebCode,
    Script,
    ConfigData,
    Document,
    Image,
    Audio,
    Video,
    Archive,
    Generic,
}

/// Icon glyph profile. `Nerd` reuses the private-use icon language already
/// present in the sidebar/AppDock; `Ascii` is the deterministic no-font
/// fallback and the canonical cross-machine visual-fixture profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconProfile {
    Nerd,
    Ascii,
}

impl VisualClass {
    pub const ALL: [Self; 17] = [
        Self::Directory,
        Self::SymlinkDirectory,
        Self::SymlinkFile,
        Self::Broken,
        Self::Special,
        Self::VersionControl,
        Self::BuildPackage,
        Self::SourceCode,
        Self::WebCode,
        Self::Script,
        Self::ConfigData,
        Self::Document,
        Self::Image,
        Self::Audio,
        Self::Video,
        Self::Archive,
        Self::Generic,
    ];

    /// One display-cell token for this class in the given profile.
    pub fn glyph(self, profile: IconProfile) -> &'static str {
        match profile {
            IconProfile::Nerd => match self {
                Self::Directory => "\u{f07b}",
                Self::SymlinkDirectory => "\u{f482}",
                Self::SymlinkFile => "\u{f481}",
                Self::Broken => "\u{f127}",
                Self::Special => "\u{f128}",
                Self::VersionControl => "\u{f1d3}",
                Self::BuildPackage => "\u{f487}",
                Self::SourceCode => "\u{f121}",
                Self::WebCode => "\u{f13b}",
                Self::Script => "\u{f120}",
                Self::ConfigData => "\u{f013}",
                Self::Document => "\u{f15c}",
                Self::Image => "\u{f1c5}",
                Self::Audio => "\u{f001}",
                Self::Video => "\u{f008}",
                Self::Archive => "\u{f1c6}",
                Self::Generic => "\u{f15b}",
            },
            IconProfile::Ascii => match self {
                Self::Directory => "/",
                Self::SymlinkDirectory => "@",
                Self::SymlinkFile => "&",
                Self::Broken => "!",
                Self::Special => "?",
                Self::VersionControl => "+",
                Self::BuildPackage => "*",
                Self::SourceCode => "#",
                Self::WebCode => "<",
                Self::Script => "$",
                Self::ConfigData => "=",
                Self::Document => "-",
                Self::Image => "%",
                Self::Audio => "~",
                Self::Video => "^",
                Self::Archive => "[",
                Self::Generic => ".",
            },
        }
    }
}

/// Classify the visual class from the prepared kind and file name. Pure:
/// no filesystem, config, process, or socket work (TP-FIP-ICON-11).
pub fn visual_class(kind: FileEntryKind, name: &str) -> VisualClass {
    match kind {
        FileEntryKind::Directory => return VisualClass::Directory,
        FileEntryKind::SymlinkDirectory => return VisualClass::SymlinkDirectory,
        FileEntryKind::SymlinkFile => return VisualClass::SymlinkFile,
        FileEntryKind::BrokenSymlink => return VisualClass::Broken,
        FileEntryKind::UnsupportedSpecial => return VisualClass::Special,
        FileEntryKind::RegularFile => {}
    }
    match name {
        ".gitignore" | ".gitattributes" | ".gitmodules" => return VisualClass::VersionControl,
        "Cargo.toml" | "Cargo.lock" | "package.json" | "package-lock.json" | "pnpm-lock.yaml"
        | "yarn.lock" | "bun.lock" | "bun.lockb" | "Dockerfile" | "Makefile" | "Justfile"
        | "justfile" => return VisualClass::BuildPackage,
        _ => {}
    }
    let extension = name
        .rsplit_once('.')
        .map(|(stem, extension)| (stem, extension.to_ascii_lowercase()));
    let Some((stem, extension)) = extension else {
        return VisualClass::Generic;
    };
    if stem.is_empty() {
        // Dotfiles without a further extension (".bashrc") stay generic
        // unless matched by the exact-name table above.
        return VisualClass::Generic;
    }
    match extension.as_str() {
        "rs" | "c" | "h" | "cpp" | "hpp" | "go" | "py" | "rb" | "java" | "kt" | "swift" | "lua" => {
            VisualClass::SourceCode
        }
        "js" | "jsx" | "ts" | "tsx" | "html" | "htm" | "css" | "scss" | "sass" | "vue"
        | "svelte" => VisualClass::WebCode,
        "sh" | "bash" | "zsh" | "fish" | "ps1" | "bat" | "cmd" => VisualClass::Script,
        "toml" | "yaml" | "yml" | "json" | "jsonc" | "xml" | "csv" | "env" | "ini" | "conf"
        | "properties" => VisualClass::ConfigData,
        "md" | "mdx" | "rst" | "txt" | "pdf" | "doc" | "docx" | "odt" | "xls" | "xlsx" | "ods"
        | "ppt" | "pptx" => VisualClass::Document,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "ico" | "bmp" | "tif" | "tiff"
        | "avif" => VisualClass::Image,
        "mp3" | "wav" | "flac" | "m4a" | "aac" | "ogg" | "opus" => VisualClass::Audio,
        "mp4" | "mkv" | "mov" | "webm" | "avi" | "m4v" => VisualClass::Video,
        "zip" | "tar" | "gz" | "bz2" | "xz" | "zst" | "7z" | "rar" | "tgz" => VisualClass::Archive,
        _ => VisualClass::Generic,
    }
}

/// Escape every control character in a file name into a printable form so a
/// hostile name can never break row layout (TP-FIP-ICON-13). C0 controls map
/// to their single-cell Unicode Control Picture, DEL to `␡`, and every other
/// control (the C1 range) to the replacement character. Clean names borrow.
pub(crate) fn escape_control_chars(name: &str) -> std::borrow::Cow<'_, str> {
    if !name.chars().any(char::is_control) {
        return std::borrow::Cow::Borrowed(name);
    }
    std::borrow::Cow::Owned(name.chars().map(escape_control_char).collect())
}

fn escape_control_char(c: char) -> char {
    if !c.is_control() {
        return c;
    }
    match u32::from(c) {
        code @ 0x00..=0x1f => char::from_u32(0x2400 + code).unwrap_or('\u{fffd}'),
        0x7f => '\u{2421}',
        _ => '\u{fffd}',
    }
}

/// Classify one directory entry from symlink-aware metadata. A symlink keeps
/// its link identity and resolves its target kind; a broken link and every
/// special (FIFO/socket/device) or unprovable target fail closed.
pub(crate) fn classify_dir_entry(entry: &std::fs::DirEntry) -> FileEntryKind {
    match entry.file_type() {
        Ok(ft) if ft.is_symlink() => match std::fs::metadata(entry.path()) {
            Ok(target) if target.is_dir() => FileEntryKind::SymlinkDirectory,
            Ok(target) if target.is_file() => FileEntryKind::SymlinkFile,
            Ok(_) => FileEntryKind::UnsupportedSpecial,
            Err(_) => FileEntryKind::BrokenSymlink,
        },
        Ok(ft) if ft.is_dir() => FileEntryKind::Directory,
        Ok(ft) if ft.is_file() => FileEntryKind::RegularFile,
        Ok(_) | Err(_) => FileEntryKind::UnsupportedSpecial,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // TP-FIP-ICON-01..05: every filesystem observation maps to its canonical
    // kind; broken/special targets carry no operation or reference authority.
    #[test]
    fn classify_covers_all_six_entry_kinds() {
        let root = std::env::temp_dir().join(format!(
            "herdr-entry-kind-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        std::fs::create_dir_all(root.join("dir")).expect("dir");
        std::fs::write(root.join("file.txt"), b"x").expect("file");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(root.join("dir"), root.join("link-dir")).expect("link-dir");
            std::os::unix::fs::symlink(root.join("file.txt"), root.join("link-file"))
                .expect("link-file");
            std::os::unix::fs::symlink(root.join("missing"), root.join("broken")).expect("broken");
            let status = std::process::Command::new("mkfifo")
                .arg(root.join("fifo"))
                .status()
                .expect("mkfifo runs");
            assert!(status.success(), "fifo fixture must exist");
        }

        let kinds: HashMap<String, FileEntryKind> = std::fs::read_dir(&root)
            .expect("read fixture root")
            .map(|entry| {
                let entry = entry.expect("dir entry");
                (
                    entry.file_name().to_string_lossy().into_owned(),
                    classify_dir_entry(&entry),
                )
            })
            .collect();
        let _ = std::fs::remove_dir_all(&root);

        assert_eq!(kinds["dir"], FileEntryKind::Directory);
        assert_eq!(kinds["file.txt"], FileEntryKind::RegularFile);
        #[cfg(unix)]
        {
            assert_eq!(kinds["link-dir"], FileEntryKind::SymlinkDirectory);
            assert_eq!(kinds["link-file"], FileEntryKind::SymlinkFile);
            assert_eq!(kinds["broken"], FileEntryKind::BrokenSymlink);
            assert_eq!(kinds["fifo"], FileEntryKind::UnsupportedSpecial);
        }
    }

    #[test]
    fn broken_and_special_kinds_disable_operations_and_reference() {
        for kind in [
            FileEntryKind::BrokenSymlink,
            FileEntryKind::UnsupportedSpecial,
        ] {
            assert!(!kind.supports_native_operation());
            assert!(!kind.supports_agent_reference());
            assert!(!kind.is_directory_target());
        }
        for kind in [
            FileEntryKind::Directory,
            FileEntryKind::RegularFile,
            FileEntryKind::SymlinkDirectory,
            FileEntryKind::SymlinkFile,
        ] {
            assert!(kind.supports_native_operation());
            assert!(kind.supports_agent_reference());
        }
        assert!(FileEntryKind::SymlinkDirectory.is_directory_target());
        assert!(!FileEntryKind::SymlinkFile.is_directory_target());
    }

    // TP-FIP-ICON-07: exact well-known names win before extension matching.
    #[test]
    fn visual_class_uses_exact_name_override_before_extension() {
        let file = FileEntryKind::RegularFile;
        assert_eq!(visual_class(file, "Dockerfile"), VisualClass::BuildPackage);
        assert_eq!(visual_class(file, "Makefile"), VisualClass::BuildPackage);
        assert_eq!(visual_class(file, "Cargo.toml"), VisualClass::BuildPackage);
        assert_eq!(
            visual_class(file, ".gitignore"),
            VisualClass::VersionControl
        );
    }

    // TP-FIP-ICON-06: extension matching is case-insensitive.
    #[test]
    fn visual_class_extension_match_is_case_insensitive() {
        let file = FileEntryKind::RegularFile;
        assert_eq!(visual_class(file, "photo.PNG"), VisualClass::Image);
        assert_eq!(visual_class(file, "main.RS"), VisualClass::SourceCode);
        assert_eq!(visual_class(file, "notes.Md"), VisualClass::Document);
    }

    // TP-FIP-ICON-01..05: kind always wins over any apparent extension.
    #[test]
    fn visual_class_kind_wins_over_extension() {
        assert_eq!(
            visual_class(FileEntryKind::Directory, "dir.png"),
            VisualClass::Directory
        );
        assert_eq!(
            visual_class(FileEntryKind::BrokenSymlink, "a.rs"),
            VisualClass::Broken
        );
        assert_eq!(
            visual_class(FileEntryKind::UnsupportedSpecial, "b.md"),
            VisualClass::Special
        );
        assert_eq!(
            visual_class(FileEntryKind::SymlinkDirectory, "c.zip"),
            VisualClass::SymlinkDirectory
        );
    }

    // TP-FIP-ICON-02: deterministic generic fallback.
    #[test]
    fn visual_class_no_extension_maps_to_generic() {
        let file = FileEntryKind::RegularFile;
        assert_eq!(visual_class(file, "README2"), VisualClass::Generic);
        assert_eq!(visual_class(file, "data.unknownext"), VisualClass::Generic);
    }

    // TP-FIP-ICON-13: control characters escape deterministically to
    // printable single-cell forms; clean names borrow without allocation.
    #[test]
    fn escape_control_chars_maps_every_control_to_printable() {
        use unicode_width::UnicodeWidthStr;
        assert!(matches!(
            escape_control_chars("clean-name.rs"),
            std::borrow::Cow::Borrowed(_)
        ));
        assert_eq!(escape_control_chars("a\nb"), "a\u{240a}b");
        assert_eq!(escape_control_chars("t\tx"), "t\u{2409}x");
        assert_eq!(escape_control_chars("nul\0"), "nul\u{2400}");
        assert_eq!(escape_control_chars("del\u{7f}"), "del\u{2421}");
        assert_eq!(escape_control_chars("c1\u{85}"), "c1\u{fffd}");
        for code in 0u32..0x20 {
            let raw = char::from_u32(code).expect("c0 scalar").to_string();
            let escaped = escape_control_chars(&raw);
            assert!(
                !escaped.chars().any(char::is_control),
                "{code:#x} must escape to a non-control form"
            );
            assert_eq!(escaped.width(), 1, "{code:#x} must stay one display cell");
        }
    }

    // TP-FIP-ICON-10: every class maps to exactly one display cell in both
    // profiles; the ASCII fallback contains no private-use glyph.
    #[test]
    fn every_visual_class_has_one_cell_glyph_in_both_profiles() {
        use unicode_width::UnicodeWidthStr;
        let mut seen_ascii = std::collections::HashSet::new();
        for class in VisualClass::ALL {
            for profile in [IconProfile::Nerd, IconProfile::Ascii] {
                let glyph = class.glyph(profile);
                assert_eq!(glyph.width(), 1, "{class:?} {profile:?} must be one cell");
            }
            let ascii = class.glyph(IconProfile::Ascii);
            assert!(ascii.is_ascii(), "{class:?} ascii profile must stay ascii");
            assert!(
                seen_ascii.insert(ascii),
                "{class:?} ascii token must be unique"
            );
        }
    }
}
