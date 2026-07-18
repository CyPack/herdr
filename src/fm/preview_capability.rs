//! Pure file-preview provider selection.
//!
//! Capability selection is client-local prepared state. It never reads the
//! filesystem, checks `PATH`, loads configuration, spawns a process, or
//! mutates file-manager navigation.

use std::path::Path;

use super::entry_kind::FileEntryKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewFallback {
    NativeText,
    MetadataOnly(PreviewReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreviewReason {
    DirectoryUsesTrail,
    DocumentMetadata,
    ArchiveMetadata,
    MediaMetadata,
    BinaryMetadata,
    BrokenSymlink,
    SpecialFile,
    UnsafePath,
}

impl PreviewReason {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::DirectoryUsesTrail => "directory contents are shown in the trail",
            Self::DocumentMetadata => "optional document viewer",
            Self::ArchiveMetadata => "optional archive viewer",
            Self::MediaMetadata => "optional media viewer",
            Self::BinaryMetadata => "binary file",
            Self::BrokenSymlink => "broken symlink",
            Self::SpecialFile => "special file",
            Self::UnsafePath => "path cannot be previewed safely",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreviewCapability {
    NativeText,
    NativeImage,
    MetadataOnly {
        reason: PreviewReason,
    },
    OptionalPlugin {
        action_id: String,
        fallback: PreviewFallback,
    },
    Unsupported {
        reason: PreviewReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewPluginProvider {
    pub action_id: String,
    pub platform_supported: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PreviewProviderSet {
    pub markdown: Option<PreviewPluginProvider>,
    pub documents: Option<PreviewPluginProvider>,
    pub archives: Option<PreviewPluginProvider>,
    pub media: Option<PreviewPluginProvider>,
}

pub(crate) fn preview_capability(
    path: &Path,
    kind: FileEntryKind,
    providers: &PreviewProviderSet,
) -> PreviewCapability {
    if matches!(
        kind,
        FileEntryKind::Directory | FileEntryKind::SymlinkDirectory
    ) {
        return PreviewCapability::Unsupported {
            reason: PreviewReason::DirectoryUsesTrail,
        };
    }
    if kind == FileEntryKind::BrokenSymlink {
        return PreviewCapability::Unsupported {
            reason: PreviewReason::BrokenSymlink,
        };
    }
    if kind == FileEntryKind::UnsupportedSpecial {
        return PreviewCapability::Unsupported {
            reason: PreviewReason::SpecialFile,
        };
    }

    let Some(path_text) = path.as_os_str().to_str() else {
        return PreviewCapability::Unsupported {
            reason: PreviewReason::UnsafePath,
        };
    };
    if path_text.chars().any(char::is_control) {
        return PreviewCapability::Unsupported {
            reason: PreviewReason::UnsafePath,
        };
    }

    if super::is_image_preview_path(path) {
        return PreviewCapability::NativeImage;
    }

    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path_text)
        .to_ascii_lowercase();
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);

    if matches_extension(extension.as_deref(), &["md", "markdown", "mdown"]) {
        return plugin_or_fallback(providers.markdown.as_ref(), PreviewFallback::NativeText);
    }
    if matches_extension(
        extension.as_deref(),
        &[
            "pdf", "doc", "docx", "odt", "rtf", "xls", "xlsx", "ods", "ppt", "pptx", "odp",
        ],
    ) {
        return plugin_or_fallback(
            providers.documents.as_ref(),
            PreviewFallback::MetadataOnly(PreviewReason::DocumentMetadata),
        );
    }
    if name.ends_with(".tar.gz")
        || name.ends_with(".tar.bz2")
        || name.ends_with(".tar.xz")
        || matches_extension(
            extension.as_deref(),
            &["zip", "tar", "gz", "bz2", "xz", "7z", "rar", "zst"],
        )
    {
        return plugin_or_fallback(
            providers.archives.as_ref(),
            PreviewFallback::MetadataOnly(PreviewReason::ArchiveMetadata),
        );
    }
    if matches_extension(
        extension.as_deref(),
        &[
            "mp3", "flac", "wav", "ogg", "m4a", "aac", "mp4", "mkv", "mov", "avi", "webm", "mpeg",
            "mpg",
        ],
    ) {
        return plugin_or_fallback(
            providers.media.as_ref(),
            PreviewFallback::MetadataOnly(PreviewReason::MediaMetadata),
        );
    }
    if matches_extension(
        extension.as_deref(),
        &[
            "bin", "exe", "dll", "so", "dylib", "class", "wasm", "o", "a", "pyc",
        ],
    ) {
        return PreviewCapability::MetadataOnly {
            reason: PreviewReason::BinaryMetadata,
        };
    }

    PreviewCapability::NativeText
}

fn matches_extension(extension: Option<&str>, candidates: &[&str]) -> bool {
    extension.is_some_and(|extension| candidates.contains(&extension))
}

fn plugin_or_fallback(
    provider: Option<&PreviewPluginProvider>,
    fallback: PreviewFallback,
) -> PreviewCapability {
    if let Some(provider) = provider
        .filter(|provider| provider.platform_supported && !provider.action_id.trim().is_empty())
    {
        return PreviewCapability::OptionalPlugin {
            action_id: provider.action_id.clone(),
            fallback,
        };
    }
    match fallback {
        PreviewFallback::NativeText => PreviewCapability::NativeText,
        PreviewFallback::MetadataOnly(reason) => PreviewCapability::MetadataOnly { reason },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn provider(action_id: &str) -> PreviewPluginProvider {
        PreviewPluginProvider {
            action_id: action_id.to_owned(),
            platform_supported: true,
        }
    }

    #[test]
    fn preview_capability_classifies_native_metadata_and_unsupported_cases() {
        let providers = PreviewProviderSet::default();
        let cases = [
            (
                "folder",
                FileEntryKind::Directory,
                PreviewCapability::Unsupported {
                    reason: PreviewReason::DirectoryUsesTrail,
                },
            ),
            (
                "notes.txt",
                FileEntryKind::RegularFile,
                PreviewCapability::NativeText,
            ),
            (
                "src/main.rs",
                FileEntryKind::RegularFile,
                PreviewCapability::NativeText,
            ),
            (
                "config.toml",
                FileEntryKind::RegularFile,
                PreviewCapability::NativeText,
            ),
            (
                "README.md",
                FileEntryKind::RegularFile,
                PreviewCapability::NativeText,
            ),
            (
                "photo.PNG",
                FileEntryKind::RegularFile,
                PreviewCapability::NativeImage,
            ),
            (
                "manual.pdf",
                FileEntryKind::RegularFile,
                PreviewCapability::MetadataOnly {
                    reason: PreviewReason::DocumentMetadata,
                },
            ),
            (
                "report.docx",
                FileEntryKind::RegularFile,
                PreviewCapability::MetadataOnly {
                    reason: PreviewReason::DocumentMetadata,
                },
            ),
            (
                "source.tar.gz",
                FileEntryKind::RegularFile,
                PreviewCapability::MetadataOnly {
                    reason: PreviewReason::ArchiveMetadata,
                },
            ),
            (
                "voice.flac",
                FileEntryKind::RegularFile,
                PreviewCapability::MetadataOnly {
                    reason: PreviewReason::MediaMetadata,
                },
            ),
            (
                "clip.mp4",
                FileEntryKind::RegularFile,
                PreviewCapability::MetadataOnly {
                    reason: PreviewReason::MediaMetadata,
                },
            ),
            (
                "payload.bin",
                FileEntryKind::RegularFile,
                PreviewCapability::MetadataOnly {
                    reason: PreviewReason::BinaryMetadata,
                },
            ),
            (
                "missing",
                FileEntryKind::BrokenSymlink,
                PreviewCapability::Unsupported {
                    reason: PreviewReason::BrokenSymlink,
                },
            ),
            (
                "socket",
                FileEntryKind::UnsupportedSpecial,
                PreviewCapability::Unsupported {
                    reason: PreviewReason::SpecialFile,
                },
            ),
            (
                "bad\nname.txt",
                FileEntryKind::RegularFile,
                PreviewCapability::Unsupported {
                    reason: PreviewReason::UnsafePath,
                },
            ),
        ];

        for (path, kind, expected) in cases {
            assert_eq!(
                preview_capability(Path::new(path), kind, &providers),
                expected,
                "capability mismatch for {path}"
            );
        }
    }

    #[test]
    fn preview_capability_uses_only_explicit_supported_plugin_providers() {
        let providers = PreviewProviderSet {
            markdown: Some(provider("preview.markdown")),
            documents: Some(PreviewPluginProvider {
                action_id: "preview.document".to_owned(),
                platform_supported: false,
            }),
            archives: Some(provider("preview.archive")),
            media: Some(provider("preview.media")),
        };

        assert_eq!(
            preview_capability(
                Path::new("README.md"),
                FileEntryKind::RegularFile,
                &providers
            ),
            PreviewCapability::OptionalPlugin {
                action_id: "preview.markdown".to_owned(),
                fallback: PreviewFallback::NativeText,
            }
        );
        assert_eq!(
            preview_capability(
                Path::new("manual.pdf"),
                FileEntryKind::RegularFile,
                &providers
            ),
            PreviewCapability::MetadataOnly {
                reason: PreviewReason::DocumentMetadata,
            },
            "unsupported-platform providers must fall back"
        );
        assert_eq!(
            preview_capability(
                Path::new("source.zip"),
                FileEntryKind::RegularFile,
                &providers
            ),
            PreviewCapability::OptionalPlugin {
                action_id: "preview.archive".to_owned(),
                fallback: PreviewFallback::MetadataOnly(PreviewReason::ArchiveMetadata),
            }
        );
        assert_eq!(
            preview_capability(
                Path::new("clip.mkv"),
                FileEntryKind::RegularFile,
                &providers
            ),
            PreviewCapability::OptionalPlugin {
                action_id: "preview.media".to_owned(),
                fallback: PreviewFallback::MetadataOnly(PreviewReason::MediaMetadata),
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn preview_capability_rejects_non_utf8_paths_without_lossy_classification() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let path = Path::new(OsStr::from_bytes(b"bad-\xff.txt"));
        assert_eq!(
            preview_capability(
                path,
                FileEntryKind::RegularFile,
                &PreviewProviderSet::default()
            ),
            PreviewCapability::Unsupported {
                reason: PreviewReason::UnsafePath,
            }
        );
    }
}
