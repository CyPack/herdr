//! Prepared, bounded navigation model for Native Files locations.
//!
//! Filesystem discovery is confined to [`FileManagerLocationsModel::from_home_and_pins`].
//! View computation, render, and input consume only these pure data types.

use std::path::{Path, PathBuf};

/// Hard ceiling for the complete prepared Files-locations model. Configuration
/// and mount discovery are external inputs; keeping them bounded prevents a
/// malformed source from creating unbounded frame or hit-test work.
pub const FILE_MANAGER_LOCATIONS_MAX_ITEMS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerLocationSectionKind {
    Favorites,
    Pinned,
    Locations,
}

impl FileManagerLocationSectionKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Favorites => "FAVORITES",
            Self::Pinned => "PINNED",
            Self::Locations => "LOCATIONS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerLocationIcon {
    Home,
    Desktop,
    Downloads,
    Documents,
    Pictures,
    Videos,
    Music,
    Trash,
    Pin,
    Disk,
}

impl FileManagerLocationIcon {
    pub const fn glyph(self) -> &'static str {
        match self {
            Self::Home => "󰋜",
            Self::Desktop => "󰇄",
            Self::Downloads => "󰉍",
            Self::Documents => "󰈙",
            Self::Pictures => "󰋩",
            Self::Videos => "󰕧",
            Self::Music => "󰝚",
            Self::Trash => "󰩹",
            Self::Pin => "󰐃",
            Self::Disk => "󰋊",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerLocationItem {
    pub label: String,
    pub path: PathBuf,
    pub icon: FileManagerLocationIcon,
    pub accessible: bool,
    pub ejectable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerLocationSection {
    pub kind: FileManagerLocationSectionKind,
    pub items: Vec<FileManagerLocationItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileManagerLocationsModel {
    revision: u64,
    pub sections: Vec<FileManagerLocationSection>,
}

impl FileManagerLocationsModel {
    pub fn from_sources(
        favorites: Vec<FileManagerLocationItem>,
        pinned: Vec<FileManagerLocationItem>,
        locations: Vec<FileManagerLocationItem>,
    ) -> Self {
        let mut seen = std::collections::HashSet::new();
        let mut remaining = FILE_MANAGER_LOCATIONS_MAX_ITEMS;
        let mut sections = Vec::with_capacity(3);

        for (kind, source) in [
            (FileManagerLocationSectionKind::Favorites, favorites),
            (FileManagerLocationSectionKind::Pinned, pinned),
            (FileManagerLocationSectionKind::Locations, locations),
        ] {
            if remaining == 0 {
                break;
            }
            let items: Vec<_> = source
                .into_iter()
                .filter(|item| seen.insert(item.path.clone()))
                .take(remaining)
                .collect();
            remaining = remaining.saturating_sub(items.len());
            if !items.is_empty() {
                sections.push(FileManagerLocationSection { kind, items });
            }
        }

        Self {
            revision: 1,
            sections,
        }
    }

    /// Prepare the startup Files-locations projection. This is an explicit
    /// refresh boundary: it may inspect directory metadata, while render and
    /// mouse input consume only the returned data.
    pub fn from_home_and_pins(home: &Path, pinned: &[PathBuf]) -> Self {
        fn directory_is_accessible(path: &Path) -> bool {
            std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
        }

        fn item(
            label: impl Into<String>,
            path: PathBuf,
            icon: FileManagerLocationIcon,
            accessible: bool,
        ) -> FileManagerLocationItem {
            FileManagerLocationItem {
                label: label.into(),
                path,
                icon,
                accessible,
                ejectable: false,
            }
        }

        let mut favorites = vec![item(
            "Home",
            home.to_path_buf(),
            FileManagerLocationIcon::Home,
            directory_is_accessible(home),
        )];
        for (label, child, icon) in [
            ("Desktop", "Desktop", FileManagerLocationIcon::Desktop),
            ("Downloads", "Downloads", FileManagerLocationIcon::Downloads),
            ("Documents", "Documents", FileManagerLocationIcon::Documents),
            ("Pictures", "Pictures", FileManagerLocationIcon::Pictures),
            ("Videos", "Videos", FileManagerLocationIcon::Videos),
            ("Music", "Music", FileManagerLocationIcon::Music),
            (
                "Trash",
                ".local/share/Trash",
                FileManagerLocationIcon::Trash,
            ),
        ] {
            let path = home.join(child);
            if directory_is_accessible(&path) {
                favorites.push(item(label, path, icon, true));
            }
        }

        let pinned = pinned
            .iter()
            .map(|path| {
                let label = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .filter(|name| !name.is_empty())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| path.display().to_string());
                item(
                    label,
                    path.clone(),
                    FileManagerLocationIcon::Pin,
                    directory_is_accessible(path),
                )
            })
            .collect();

        let locations = home
            .ancestors()
            .last()
            .filter(|path| !path.as_os_str().is_empty())
            .map(|root| {
                vec![item(
                    "Root",
                    root.to_path_buf(),
                    FileManagerLocationIcon::Disk,
                    directory_is_accessible(root),
                )]
            })
            .unwrap_or_default();

        Self::from_sources(favorites, pinned, locations)
    }

    #[cfg(test)]
    pub fn section(
        &self,
        kind: FileManagerLocationSectionKind,
    ) -> Option<&FileManagerLocationSection> {
        self.sections.iter().find(|section| section.kind == kind)
    }

    #[cfg(test)]
    pub fn item_count(&self) -> usize {
        self.sections
            .iter()
            .map(|section| section.items.len())
            .sum()
    }

    pub fn item_for_path(&self, path: &Path) -> Option<&FileManagerLocationItem> {
        self.sections
            .iter()
            .flat_map(|section| &section.items)
            .find(|item| item.path == path)
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    #[cfg(test)]
    pub(crate) fn content_line_count(&self) -> usize {
        0
    }

    #[cfg(test)]
    pub(crate) fn line_index_for_path(&self, _path: &Path) -> Option<usize> {
        None
    }

    /// Replace a published test projection and advance its identity so stale
    /// asynchronous completions can be exercised without filesystem timing.
    #[cfg(test)]
    pub(crate) fn replace_with(&mut self, mut replacement: Self) {
        replacement.revision = self.revision.wrapping_add(1).max(1);
        *self = replacement;
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationsModel};

    fn item(path: &str, accessible: bool) -> FileManagerLocationItem {
        FileManagerLocationItem {
            label: Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path)
                .to_string(),
            path: PathBuf::from(path),
            icon: FileManagerLocationIcon::Pin,
            accessible,
            ejectable: false,
        }
    }

    // TP-FLF-STEP-01: input auto-scroll and renderer rows share one content
    // line identity across headers, inaccessible rows, and section gaps.
    #[test]
    fn flf_model_line_identity_matches_render_section_law() {
        let model = FileManagerLocationsModel::from_sources(
            vec![item("/workspace", true), item("/missing", false)],
            vec![item("/pinned", true)],
            vec![item("/", true)],
        );

        assert_eq!(model.content_line_count(), 9);
        assert_eq!(model.line_index_for_path(Path::new("/workspace")), Some(1));
        assert_eq!(model.line_index_for_path(Path::new("/missing")), Some(2));
        assert_eq!(model.line_index_for_path(Path::new("/pinned")), Some(5));
        assert_eq!(model.line_index_for_path(Path::new("/")), Some(8));
        assert_eq!(model.line_index_for_path(Path::new("/absent")), None);
    }
}
