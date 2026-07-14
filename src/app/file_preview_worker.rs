use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::fm::TextPreview;

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilePreviewHighlightKey {
    path: PathBuf,
    content_sha256: [u8; 32],
    truncated: bool,
}

impl FilePreviewHighlightKey {
    fn new(path: &Path, preview: &TextPreview) -> Self {
        let digest = Sha256::digest(preview.content.as_bytes());
        Self {
            path: path.to_path_buf(),
            content_sha256: digest.into(),
            truncated: preview.truncated,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilePreviewHighlightSync {
    Unchanged,
    Started { generation: u64 },
    Stopped,
}

#[derive(Debug, Default)]
struct FilePreviewHighlightSlot {
    generation: u64,
    active: Option<FilePreviewHighlightKey>,
}

impl FilePreviewHighlightSlot {
    fn sync(&mut self, target: Option<FilePreviewHighlightKey>) -> FilePreviewHighlightSync {
        if self.active == target {
            return FilePreviewHighlightSync::Unchanged;
        }

        self.generation = self.generation.wrapping_add(1).max(1);
        self.active = target;
        if self.active.is_some() {
            FilePreviewHighlightSync::Started {
                generation: self.generation,
            }
        } else {
            FilePreviewHighlightSync::Stopped
        }
    }

    fn accepts(&self, generation: u64, key: &FilePreviewHighlightKey) -> bool {
        self.generation == generation && self.active.as_ref() == Some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fm::TextPreview;
    use std::path::Path;

    fn preview(content: &str) -> TextPreview {
        TextPreview {
            content: content.to_owned(),
            truncated: false,
        }
    }

    // TP-B1.4-LIFECYCLE: unchanged content does not enqueue duplicate work,
    // while a content change advances generation and invalidates old results.
    #[test]
    fn highlight_slot_rebinds_and_rejects_stale_generation() {
        let first_key = FilePreviewHighlightKey::new(Path::new("sample.rs"), &preview("first"));
        let second_key = FilePreviewHighlightKey::new(Path::new("sample.rs"), &preview("second"));
        let mut slot = FilePreviewHighlightSlot::default();

        let first_generation = match slot.sync(Some(first_key.clone())) {
            FilePreviewHighlightSync::Started { generation } => generation,
            other => panic!("first target must start, got {other:?}"),
        };
        assert_eq!(
            slot.sync(Some(first_key.clone())),
            FilePreviewHighlightSync::Unchanged
        );

        let second_generation = match slot.sync(Some(second_key.clone())) {
            FilePreviewHighlightSync::Started { generation } => generation,
            other => panic!("changed content must restart, got {other:?}"),
        };

        assert_ne!(first_generation, second_generation);
        assert!(!slot.accepts(first_generation, &first_key));
        assert!(slot.accepts(second_generation, &second_key));
    }

    // TP-B1.4-LIFECYCLE: closing the FM invalidates the current authority even
    // if its background result arrives later.
    #[test]
    fn highlight_slot_close_rejects_prior_generation() {
        let key = FilePreviewHighlightKey::new(Path::new("sample.rs"), &preview("content"));
        let mut slot = FilePreviewHighlightSlot::default();
        let generation = match slot.sync(Some(key.clone())) {
            FilePreviewHighlightSync::Started { generation } => generation,
            other => panic!("target must start, got {other:?}"),
        };

        assert_eq!(slot.sync(None), FilePreviewHighlightSync::Stopped);
        assert!(!slot.accepts(generation, &key));
        assert_eq!(slot.sync(None), FilePreviewHighlightSync::Unchanged);
    }
}
