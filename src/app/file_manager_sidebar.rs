//! Prepared, bounded navigation model for the native file-manager sidebar.
//!
//! Filesystem discovery is confined to [`FileManagerSidebarModel::from_home_and_pins`].
//! View computation, render, and input consume only these pure data types.

use std::path::{Path, PathBuf};

use ratatui::layout::Rect;

/// Hard ceiling for the complete prepared Files-sidebar model. Configuration
/// and mount discovery are external inputs; keeping them bounded prevents a
/// malformed source from creating unbounded frame or hit-test work.
pub const FILE_MANAGER_SIDEBAR_MAX_ITEMS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerSidebarSectionKind {
    Favorites,
    Pinned,
    Locations,
}

impl FileManagerSidebarSectionKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Favorites => "FAVORITES",
            Self::Pinned => "PINNED",
            Self::Locations => "LOCATIONS",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerSidebarIcon {
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

impl FileManagerSidebarIcon {
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
pub struct FileManagerSidebarItem {
    pub label: String,
    pub path: PathBuf,
    pub icon: FileManagerSidebarIcon,
    pub accessible: bool,
    pub ejectable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerSidebarSection {
    pub kind: FileManagerSidebarSectionKind,
    pub items: Vec<FileManagerSidebarItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileManagerSidebarModel {
    revision: u64,
    pub sections: Vec<FileManagerSidebarSection>,
}

impl FileManagerSidebarModel {
    pub fn from_sources(
        favorites: Vec<FileManagerSidebarItem>,
        pinned: Vec<FileManagerSidebarItem>,
        locations: Vec<FileManagerSidebarItem>,
    ) -> Self {
        let mut seen = std::collections::HashSet::new();
        let mut remaining = FILE_MANAGER_SIDEBAR_MAX_ITEMS;
        let mut sections = Vec::with_capacity(3);

        for (kind, source) in [
            (FileManagerSidebarSectionKind::Favorites, favorites),
            (FileManagerSidebarSectionKind::Pinned, pinned),
            (FileManagerSidebarSectionKind::Locations, locations),
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
                sections.push(FileManagerSidebarSection { kind, items });
            }
        }

        Self {
            revision: 1,
            sections,
        }
    }

    /// Prepare the startup Files-sidebar projection. This is an explicit
    /// refresh boundary: it may inspect directory metadata, while render and
    /// mouse input consume only the returned data.
    pub fn from_home_and_pins(home: &Path, pinned: &[PathBuf]) -> Self {
        fn directory_is_accessible(path: &Path) -> bool {
            std::fs::metadata(path).is_ok_and(|metadata| metadata.is_dir())
        }

        fn item(
            label: impl Into<String>,
            path: PathBuf,
            icon: FileManagerSidebarIcon,
            accessible: bool,
        ) -> FileManagerSidebarItem {
            FileManagerSidebarItem {
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
            FileManagerSidebarIcon::Home,
            directory_is_accessible(home),
        )];
        for (label, child, icon) in [
            ("Desktop", "Desktop", FileManagerSidebarIcon::Desktop),
            ("Downloads", "Downloads", FileManagerSidebarIcon::Downloads),
            ("Documents", "Documents", FileManagerSidebarIcon::Documents),
            ("Pictures", "Pictures", FileManagerSidebarIcon::Pictures),
            ("Videos", "Videos", FileManagerSidebarIcon::Videos),
            ("Music", "Music", FileManagerSidebarIcon::Music),
            ("Trash", ".local/share/Trash", FileManagerSidebarIcon::Trash),
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
                    FileManagerSidebarIcon::Pin,
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
                    FileManagerSidebarIcon::Disk,
                    directory_is_accessible(root),
                )]
            })
            .unwrap_or_default();

        Self::from_sources(favorites, pinned, locations)
    }

    #[cfg(test)]
    pub fn section(
        &self,
        kind: FileManagerSidebarSectionKind,
    ) -> Option<&FileManagerSidebarSection> {
        self.sections.iter().find(|section| section.kind == kind)
    }

    #[cfg(test)]
    pub fn item_count(&self) -> usize {
        self.sections
            .iter()
            .map(|section| section.items.len())
            .sum()
    }

    pub fn item_for_path(&self, path: &Path) -> Option<&FileManagerSidebarItem> {
        self.sections
            .iter()
            .flat_map(|section| &section.items)
            .find(|item| item.path == path)
    }

    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }

    /// Replace a published test projection and advance its identity so stale
    /// asynchronous completions can be exercised without filesystem timing.
    #[cfg(test)]
    pub(crate) fn replace_with(&mut self, mut replacement: Self) {
        replacement.revision = self.revision.wrapping_add(1).max(1);
        *self = replacement;
    }
}

/// One exact, authority-bearing item row in the prepared Files sidebar.
/// Headers and spacing are deliberately absent from this geometry model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileManagerSidebarRowArea {
    pub rect: Rect,
    pub path: PathBuf,
}
