//! Prepared, bounded navigation model for Native Files locations.
//!
//! Filesystem discovery is confined to [`FileManagerLocationsModel::from_host_sources`].
//! View computation, render, and input consume only these pure data types.

use std::path::{Path, PathBuf};

/// Hard ceiling for the complete prepared Files-locations model. Configuration
/// and mount discovery are external inputs; keeping them bounded prevents a
/// malformed source from creating unbounded frame or hit-test work.
pub const FILE_MANAGER_LOCATIONS_MAX_ITEMS: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerLocationSectionKind {
    Favorites,
    Bookmarks,
    Pinned,
    Locations,
}

impl FileManagerLocationSectionKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Favorites => "FAVORITES",
            Self::Bookmarks => "BOOKMARKS",
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
    Network,
    Bookmark,
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
            Self::Network => "󰛳",
            Self::Bookmark => "󰉋",
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

/// Everything the rail is built from, gathered by the caller so that model
/// preparation itself stays a pure projection over named inputs.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FileManagerLocationSources<'a> {
    /// The user's home directory.
    pub(crate) home: &'a Path,
    /// The well-known user directories as the host actually keeps them. Their
    /// names are localized per path element, so they arrive as measured paths
    /// rather than being assumed from English defaults.
    pub(crate) user_dirs: &'a [crate::platform::UserDirectory],
    /// Volumes the host currently has mounted, in mount-table order. These are
    /// what makes the rail worth having on a machine with no desktop at all.
    pub(crate) volumes: &'a [PathBuf],
    /// Root of the host's mounted network shares, when it has one.
    pub(crate) network_root: Option<&'a Path>,
    /// The host file manager's bookmark list, in the order the user arranged it.
    pub(crate) bookmarks: &'a [crate::platform::DesktopBookmark],
    /// Absolute `[projects] pinned` directories from this application's config.
    pub(crate) pinned: &'a [PathBuf],
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
    /// Compose a rail with no host bookmark section. Retained for the many
    /// suites that exercise sections unrelated to the host list; production
    /// composition goes through [`Self::from_ordered_sources`].
    #[cfg(test)]
    pub fn from_sources(
        favorites: Vec<FileManagerLocationItem>,
        pinned: Vec<FileManagerLocationItem>,
        locations: Vec<FileManagerLocationItem>,
    ) -> Self {
        Self::from_ordered_sources(favorites, Vec::new(), pinned, locations)
    }

    /// Compose the rail in its published section order: the host built-in
    /// block, then the list the user curated in their desktop file manager,
    /// then this application's own pins.
    ///
    /// Path identity belongs to the **first** section that claims it. That is
    /// what keeps a bookmarked `Downloads` in the fixed built-in block instead
    /// of drawing it a second time in the middle of the curated list.
    pub fn from_ordered_sources(
        favorites: Vec<FileManagerLocationItem>,
        bookmarks: Vec<FileManagerLocationItem>,
        pinned: Vec<FileManagerLocationItem>,
        locations: Vec<FileManagerLocationItem>,
    ) -> Self {
        let mut seen = std::collections::HashSet::new();
        let mut remaining = FILE_MANAGER_LOCATIONS_MAX_ITEMS;
        let mut sections = Vec::with_capacity(4);

        for (kind, source) in [
            (FileManagerLocationSectionKind::Favorites, favorites),
            (FileManagerLocationSectionKind::Bookmarks, bookmarks),
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
    ///
    /// Host discovery — reading the bookmark list, locating the network mount
    /// root — happens in [`crate::platform`] and arrives here as data, so this
    /// projection stays deterministic under test.
    pub(crate) fn from_host_sources(sources: FileManagerLocationSources<'_>) -> Self {
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

        /// A path's own name is its display text unless something authoritative
        /// renamed it.
        fn derived_label(path: &Path) -> String {
            path.file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .map_or_else(|| path.display().to_string(), ToOwned::to_owned)
        }

        let FileManagerLocationSources {
            home,
            user_dirs,
            volumes,
            network_root,
            bookmarks,
            pinned,
        } = sources;

        let mut favorites = vec![item(
            "Home",
            home.to_path_buf(),
            FileManagerLocationIcon::Home,
            directory_is_accessible(home),
        )];

        // The well-known user directories are permanent members of the built-in
        // block, with their own identity icons. Their names are localized per
        // path element on the host, so the measured path is authoritative and
        // its own name is the label.
        // with their own identity icons. Users routinely bookmark them as well;
        // section dedup then absorbs that copy here rather than scattering
        // Downloads and Documents into the middle of the curated list.
        for directory in user_dirs {
            let icon = match directory.kind {
                crate::platform::UserDirectoryKind::Desktop => FileManagerLocationIcon::Desktop,
                crate::platform::UserDirectoryKind::Downloads => FileManagerLocationIcon::Downloads,
                crate::platform::UserDirectoryKind::Documents => FileManagerLocationIcon::Documents,
                crate::platform::UserDirectoryKind::Pictures => FileManagerLocationIcon::Pictures,
                crate::platform::UserDirectoryKind::Videos => FileManagerLocationIcon::Videos,
                crate::platform::UserDirectoryKind::Music => FileManagerLocationIcon::Music,
            };
            if directory_is_accessible(&directory.path) {
                favorites.push(item(
                    derived_label(&directory.path),
                    directory.path.clone(),
                    icon,
                    true,
                ));
            }
        }

        if let Some(root) = network_root.filter(|root| directory_is_accessible(root)) {
            favorites.push(item(
                "Network",
                root.to_path_buf(),
                FileManagerLocationIcon::Network,
                true,
            ));
        }

        // Desktop file managers open the trash's `files/` directory rather than
        // the freedesktop container that also holds `info/` and `expunged/`.
        // The container is the fallback for a home that has not been used yet.
        let trash_container = home.join(".local/share/Trash");
        for candidate in [trash_container.join("files"), trash_container] {
            if directory_is_accessible(&candidate) {
                favorites.push(item(
                    "Trash",
                    candidate,
                    FileManagerLocationIcon::Trash,
                    true,
                ));
                break;
            }
        }

        // A bookmark whose target is gone stays visible and inaccessible. The
        // desktop file manager marks it too; hiding it would silently erase the
        // fact that something the user relies on has moved.
        let bookmarks = bookmarks
            .iter()
            .map(|bookmark| {
                item(
                    bookmark
                        .label
                        .clone()
                        .unwrap_or_else(|| derived_label(&bookmark.path)),
                    bookmark.path.clone(),
                    FileManagerLocationIcon::Bookmark,
                    directory_is_accessible(&bookmark.path),
                )
            })
            .collect();

        let pinned = pinned
            .iter()
            .map(|path| {
                item(
                    derived_label(path),
                    path.clone(),
                    FileManagerLocationIcon::Pin,
                    directory_is_accessible(path),
                )
            })
            .collect();

        // The mounted volumes and the filesystem root are what this section is
        // for. On a host with no desktop — a server, a container, an SSH
        // session — they are the whole of what "places" can mean, and the rail
        // is only worth drawing because they are here.
        let mut locations: Vec<_> = volumes
            .iter()
            .map(|volume| {
                item(
                    derived_label(volume),
                    volume.clone(),
                    FileManagerLocationIcon::Disk,
                    directory_is_accessible(volume),
                )
            })
            .collect();
        locations.extend(
            home.ancestors()
                .last()
                .filter(|path| !path.as_os_str().is_empty())
                .map(|root| {
                    item(
                        "Root",
                        root.to_path_buf(),
                        FileManagerLocationIcon::Disk,
                        directory_is_accessible(root),
                    )
                }),
        );

        Self::from_ordered_sources(favorites, bookmarks, pinned, locations)
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

    pub(crate) fn accessible_items(
        &self,
    ) -> impl DoubleEndedIterator<Item = &FileManagerLocationItem> {
        self.sections
            .iter()
            .flat_map(|section| section.items.iter())
            .filter(|item| item.accessible)
    }

    pub(crate) fn content_line_count(&self) -> usize {
        self.sections
            .iter()
            .map(|section| 1usize.saturating_add(section.items.len()))
            .sum::<usize>()
            .saturating_add(self.sections.len().saturating_sub(1))
    }

    pub(crate) fn line_index_for_path(&self, path: &Path) -> Option<usize> {
        let mut line_index = 0usize;
        for (section_index, section) in self.sections.iter().enumerate() {
            line_index = line_index.saturating_add(1);
            for item in &section.items {
                if item.path == path {
                    return Some(line_index);
                }
                line_index = line_index.saturating_add(1);
            }
            if section_index + 1 < self.sections.len() {
                line_index = line_index.saturating_add(1);
            }
        }
        None
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{
        FileManagerLocationIcon, FileManagerLocationItem, FileManagerLocationSectionKind,
        FileManagerLocationSources, FileManagerLocationsModel,
    };
    use crate::platform::{DesktopBookmark, UserDirectory, UserDirectoryKind};

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

    struct TempHome(PathBuf);

    impl TempHome {
        fn new(name: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            let root = std::env::temp_dir().join(format!(
                "herdr-locations-{name}-{}-{}",
                std::process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&root).expect("create temp home");
            Self(root)
        }

        fn directory(&self, child: &str) -> PathBuf {
            let path = self.0.join(child);
            std::fs::create_dir_all(&path).expect("create directory");
            path
        }
    }

    impl Drop for TempHome {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn bookmark(path: &Path, label: Option<&str>) -> DesktopBookmark {
        DesktopBookmark {
            path: path.to_path_buf(),
            label: label.map(ToOwned::to_owned),
        }
    }

    // TP-FLF-STEP-01: input auto-scroll and renderer rows share one content
    // line identity across headers, inaccessible rows, and section gaps.
    // TP-FDB-MODEL-03: the built-in block names its directories the way the host
    // does. The freedesktop layout is localized per path element, so a desktop
    // that keeps `İndirilenler` must see `İndirilenler`; assuming the English
    // name empties this block on every desktop that is not English. A recorded
    // directory pointing back at home draws no second Home row.
    // TP-FDB-VOL-02: a host with no desktop at all — no file manager, so no
    // bookmark list, and none of the well-known directories on disk — still
    // gets a rail worth drawing. herdr carries the experience; the host is not
    // required to have a graphical file manager for places to mean something.
    #[test]
    fn fdb_a_host_without_a_desktop_still_gets_a_rail_worth_drawing() {
        let home = TempHome::new("headless");
        // A mounted volume is what a server has instead of a Downloads folder.
        let volume = home.directory("arsiv");

        let model = FileManagerLocationsModel::from_host_sources(FileManagerLocationSources {
            home: &home.0,
            // The defaults are offered, but none of them exist on this host.
            user_dirs: &crate::platform::well_known_user_directories(&home.0),
            volumes: &[volume],
            network_root: None,
            bookmarks: &[],
            pinned: &[],
        });

        let labels: Vec<_> = model
            .sections
            .iter()
            .flat_map(|section| section.items.iter().map(|item| item.label.as_str()))
            .collect();

        assert!(
            labels.contains(&"Home") && labels.contains(&"arsiv") && labels.contains(&"Root"),
            "a desktop-less host still reaches home, its volumes and the filesystem root: {labels:?}"
        );
        assert!(
            labels.len() >= 3,
            "an empty rail is not an acceptable outcome in any environment: {labels:?}"
        );
    }

    #[test]
    fn fdb_built_in_block_follows_the_host_own_directory_names() {
        let home = TempHome::new("localized");
        let downloads = home.directory("İndirilenler");
        let documents = home.directory("Belgeler");

        let model = FileManagerLocationsModel::from_host_sources(FileManagerLocationSources {
            home: &home.0,
            user_dirs: &[
                UserDirectory {
                    kind: UserDirectoryKind::Desktop,
                    path: home.0.clone(),
                },
                UserDirectory {
                    kind: UserDirectoryKind::Downloads,
                    path: downloads,
                },
                UserDirectory {
                    kind: UserDirectoryKind::Documents,
                    path: documents,
                },
            ],
            volumes: &[],
            network_root: None,
            bookmarks: &[],
            pinned: &[],
        });

        let favorites = model
            .section(FileManagerLocationSectionKind::Favorites)
            .expect("favorites section");
        assert_eq!(
            favorites
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["Home", "İndirilenler", "Belgeler"],
            "the host's own names are the labels, and home is drawn once"
        );
        assert_eq!(
            favorites
                .items
                .iter()
                .find(|item| item.label == "İndirilenler")
                .map(|item| item.icon),
            Some(FileManagerLocationIcon::Downloads),
            "identity travels with the kind, not with the localized name"
        );
    }

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

    // TP-FDB-MODEL-01: the host bookmark list reaches the rail as its own
    // section, in the user's order, keeping renamed labels and keeping broken
    // targets visible-but-inaccessible instead of silently disappearing.
    #[test]
    fn fdb_bookmarks_section_preserves_host_order_labels_and_broken_targets() {
        let home = TempHome::new("bookmarks");
        let projects = home.directory("projects");
        let renamed = home.directory("Asus-Downloads");
        let screenshots = home.directory("Pictures/Screenshots");
        let missing = home.0.join("removed-since-bookmarking");

        let model = FileManagerLocationsModel::from_host_sources(FileManagerLocationSources {
            home: &home.0,
            user_dirs: &crate::platform::well_known_user_directories(&home.0),
            volumes: &[],
            network_root: None,
            bookmarks: &[
                bookmark(&projects, None),
                bookmark(&missing, Some("Git-sync")),
                bookmark(&renamed, Some("ASUS Downloads (Tailscale)")),
                bookmark(&screenshots, None),
                bookmark(&home.0, Some("Home again")),
            ],
            pinned: &[],
        });

        let bookmarks = model
            .section(FileManagerLocationSectionKind::Bookmarks)
            .expect("bookmarks section");
        assert_eq!(
            bookmarks
                .items
                .iter()
                .map(|item| (item.label.as_str(), item.accessible))
                .collect::<Vec<_>>(),
            [
                ("projects", true),
                ("Git-sync", false),
                ("ASUS Downloads (Tailscale)", true),
                ("Screenshots", true),
            ],
            "host order and labels survive, and a duplicate of Home stays with Favorites"
        );
    }

    // TP-FDB-MODEL-02: the built-in block is fixed — the XDG user directories
    // keep their identity position there whether or not the host also bookmarks
    // them. Bookmarking Downloads must not demote it into the curated list.
    #[test]
    fn fdb_well_known_directories_stay_in_the_built_in_block_when_also_bookmarked() {
        let home = TempHome::new("favorites");
        let downloads = home.directory("Downloads");
        home.directory("Documents");
        let music = home.directory("Music");
        let projects = home.directory("projects");
        let network = home.directory("gvfs");
        home.directory(".local/share/Trash/files");

        let model = FileManagerLocationsModel::from_host_sources(FileManagerLocationSources {
            home: &home.0,
            user_dirs: &crate::platform::well_known_user_directories(&home.0),
            volumes: &[],
            network_root: Some(&network),
            bookmarks: &[
                bookmark(&projects, None),
                bookmark(&downloads, None),
                bookmark(&music, Some("Müzik")),
            ],
            pinned: &[],
        });

        let favorites = model
            .section(FileManagerLocationSectionKind::Favorites)
            .expect("favorites section");
        assert_eq!(
            favorites
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            [
                "Home",
                "Downloads",
                "Documents",
                "Music",
                "Network",
                "Trash"
            ],
            "well-known directories hold their built-in position and icon"
        );
        assert_eq!(
            favorites
                .items
                .iter()
                .find(|item| item.label == "Trash")
                .map(|item| item.path.clone()),
            Some(home.0.join(".local/share/Trash/files")),
            "Trash points at the directory the desktop file manager shows"
        );

        assert_eq!(
            model
                .section(FileManagerLocationSectionKind::Bookmarks)
                .expect("bookmarks section")
                .items
                .iter()
                .map(|item| item.label.as_str())
                .collect::<Vec<_>>(),
            ["projects"],
            "the curated list carries only what the built-in block does not own"
        );

        let without_bookmarks =
            FileManagerLocationsModel::from_host_sources(FileManagerLocationSources {
                home: &home.0,
                user_dirs: &crate::platform::well_known_user_directories(&home.0),
                volumes: &[],
                network_root: Some(&network),
                bookmarks: &[],
                pinned: &[],
            });
        assert!(
            without_bookmarks
                .section(FileManagerLocationSectionKind::Bookmarks)
                .is_none(),
            "an empty host list produces no empty section header"
        );
    }
}
