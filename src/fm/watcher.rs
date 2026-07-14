//! Pure file-manager watcher event normalization.
//!
//! Backend-specific watcher events are translated into these small values
//! before they can affect [`super::FmState`]. Keeping this seam filesystem-free
//! makes event relevance and burst coalescing deterministic in unit tests.

use std::path::{Path, PathBuf};

use notify_debouncer_full::{
    notify::{event::ModifyKind, Event, EventKind},
    DebounceEventResult,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FmWatchEventKind {
    Create,
    Modify,
    Remove,
    Rename,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmWatchEvent {
    pub(crate) kind: FmWatchEventKind,
    pub(crate) paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FmWatchIntent {
    Refresh,
}

pub(crate) fn normalize_watch_events(
    watched_dir: &Path,
    events: &[FmWatchEvent],
) -> Option<FmWatchIntent> {
    events
        .iter()
        .any(|event| {
            event.kind != FmWatchEventKind::Other
                && event.paths.iter().any(|path| {
                    path == watched_dir || path.parent().is_some_and(|parent| parent == watched_dir)
                })
        })
        .then_some(FmWatchIntent::Refresh)
}

pub(crate) fn map_notify_event(event: &Event) -> Option<FmWatchEvent> {
    let kind = match event.kind {
        EventKind::Create(_) => FmWatchEventKind::Create,
        EventKind::Modify(ModifyKind::Name(_)) => FmWatchEventKind::Rename,
        EventKind::Modify(_) => FmWatchEventKind::Modify,
        EventKind::Remove(_) => FmWatchEventKind::Remove,
        EventKind::Access(_) | EventKind::Any | EventKind::Other => return None,
    };

    Some(FmWatchEvent {
        kind,
        paths: event.paths.clone(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FmWatchMessage {
    generation: u64,
    events: Vec<FmWatchEvent>,
    errors: Vec<String>,
}

impl FmWatchMessage {
    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    pub(crate) fn event_count(&self) -> usize {
        self.events.len()
    }

    #[cfg(test)]
    pub(crate) fn errors(&self) -> &[String] {
        &self.errors
    }
}

pub(crate) fn watch_message_from_result(
    generation: u64,
    result: DebounceEventResult,
) -> FmWatchMessage {
    match result {
        Ok(events) => FmWatchMessage {
            generation,
            events: events
                .iter()
                .filter_map(|event| map_notify_event(event))
                .collect(),
            errors: Vec::new(),
        },
        Err(errors) => FmWatchMessage {
            generation,
            events: Vec::new(),
            errors: errors.into_iter().map(|error| error.to_string()).collect(),
        },
    }
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct FmWatchDrain {
    pub(crate) refresh: bool,
    pub(crate) accepted_messages: usize,
    pub(crate) stale_messages: usize,
    pub(crate) errors: Vec<String>,
}

pub(crate) fn drain_watch_messages<B>(
    slot: &FmWatcherSlot<B>,
    messages: impl IntoIterator<Item = FmWatchMessage>,
) -> FmWatchDrain {
    let mut drained = FmWatchDrain::default();

    for message in messages {
        if !slot.accepts_generation(message.generation) {
            drained.stale_messages += 1;
            continue;
        }

        drained.accepted_messages += 1;
        if let Some(watched_dir) = slot.watched_dir() {
            drained.refresh |= normalize_watch_events(watched_dir, &message.events)
                == Some(FmWatchIntent::Refresh);
        }
        drained.errors.extend(message.errors);
    }

    drained
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FmWatcherSync {
    Unchanged,
    Started { generation: u64 },
    Stopped,
}

pub(crate) struct FmWatcherSlot<B> {
    watched_dir: Option<PathBuf>,
    generation: u64,
    backend: Option<B>,
}

impl<B> Default for FmWatcherSlot<B> {
    fn default() -> Self {
        Self {
            watched_dir: None,
            generation: 0,
            backend: None,
        }
    }
}

impl<B> FmWatcherSlot<B> {
    pub(crate) fn sync_with<E>(
        &mut self,
        target: Option<&Path>,
        start: impl FnOnce(&Path, u64) -> Result<B, E>,
    ) -> Result<FmWatcherSync, E> {
        if self.watched_dir.as_deref() == target {
            return Ok(FmWatcherSync::Unchanged);
        }

        self.backend = None;
        self.watched_dir = target.map(Path::to_path_buf);
        self.generation = self.generation.wrapping_add(1).max(1);

        let Some(target) = target else {
            return Ok(FmWatcherSync::Stopped);
        };
        let backend = start(target, self.generation)?;
        self.backend = Some(backend);
        Ok(FmWatcherSync::Started {
            generation: self.generation,
        })
    }

    pub(crate) fn watched_dir(&self) -> Option<&Path> {
        self.watched_dir.as_deref()
    }

    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn has_backend(&self) -> bool {
        self.backend.is_some()
    }

    pub(crate) fn accepts_generation(&self, generation: u64) -> bool {
        self.backend.is_some() && self.generation == generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify_debouncer_full::{
        notify::{
            event::{AccessKind, CreateKind, DataChange, ModifyKind, RemoveKind, RenameMode},
            Error, Event, EventKind,
        },
        DebouncedEvent,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Instant;

    fn event(kind: FmWatchEventKind, paths: &[&str]) -> FmWatchEvent {
        FmWatchEvent {
            kind,
            paths: paths.iter().map(PathBuf::from).collect(),
        }
    }

    fn debounced_event(kind: EventKind, paths: &[&str]) -> DebouncedEvent {
        let event = paths.iter().fold(Event::new(kind), |event, path| {
            event.add_path(PathBuf::from(path))
        });
        DebouncedEvent::new(event, Instant::now())
    }

    #[test]
    fn notify_event_mapping_keeps_mutations_and_ignores_noise() {
        let cases = [
            (
                EventKind::Create(CreateKind::File),
                Some(FmWatchEventKind::Create),
            ),
            (
                EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                Some(FmWatchEventKind::Modify),
            ),
            (
                EventKind::Modify(ModifyKind::Name(RenameMode::Both)),
                Some(FmWatchEventKind::Rename),
            ),
            (
                EventKind::Remove(RemoveKind::File),
                Some(FmWatchEventKind::Remove),
            ),
            (EventKind::Access(AccessKind::Read), None),
            (EventKind::Other, None),
        ];

        for (kind, expected) in cases {
            let event = Event::new(kind).add_path(PathBuf::from("/work/file.txt"));
            assert_eq!(
                map_notify_event(&event).map(|event| event.kind),
                expected,
                "unexpected mapping for {kind:?}"
            );
        }
    }

    #[test]
    fn callback_message_preserves_generation_and_reports_errors() {
        let event_message = watch_message_from_result(
            7,
            Ok(vec![debounced_event(
                EventKind::Create(CreateKind::File),
                &["/work/file.txt"],
            )]),
        );
        assert_eq!(event_message.generation(), 7);
        assert_eq!(event_message.event_count(), 1);
        assert!(event_message.errors().is_empty());

        let error_message = watch_message_from_result(8, Err(vec![Error::generic("watch failed")]));
        assert_eq!(error_message.generation(), 8);
        assert_eq!(error_message.event_count(), 0);
        assert_eq!(error_message.errors(), &["watch failed".to_string()]);
    }

    #[test]
    fn drain_rejects_stale_messages_and_coalesces_current_burst() {
        let mut slot = FmWatcherSlot::default();
        slot.sync_with(Some(Path::new("/work")), |_, _| Ok::<_, ()>(()))
            .expect("start watcher");
        let current_generation = slot.generation();

        let stale = watch_message_from_result(
            current_generation.saturating_sub(1),
            Ok(vec![debounced_event(
                EventKind::Remove(RemoveKind::File),
                &["/work/stale.txt"],
            )]),
        );
        let create = watch_message_from_result(
            current_generation,
            Ok(vec![debounced_event(
                EventKind::Create(CreateKind::File),
                &["/work/file.txt"],
            )]),
        );
        let modify = watch_message_from_result(
            current_generation,
            Ok(vec![debounced_event(
                EventKind::Modify(ModifyKind::Data(DataChange::Content)),
                &["/work/file.txt"],
            )]),
        );

        let drained = drain_watch_messages(&slot, [stale, create, modify]);
        assert!(drained.refresh);
        assert_eq!(drained.accepted_messages, 2);
        assert_eq!(drained.stale_messages, 1);
        assert!(drained.errors.is_empty());
    }

    #[test]
    fn drain_keeps_current_errors_without_requesting_refresh() {
        let mut slot = FmWatcherSlot::default();
        slot.sync_with(Some(Path::new("/work")), |_, _| Ok::<_, ()>(()))
            .expect("start watcher");
        let message = watch_message_from_result(
            slot.generation(),
            Err(vec![Error::generic("backend unavailable")]),
        );

        let drained = drain_watch_messages(&slot, [message]);
        assert!(!drained.refresh);
        assert_eq!(drained.accepted_messages, 1);
        assert_eq!(drained.stale_messages, 0);
        assert_eq!(drained.errors, vec!["backend unavailable".to_string()]);
    }

    #[test]
    fn direct_child_content_changes_request_refresh() {
        let watched = Path::new("/work");

        for kind in [
            FmWatchEventKind::Create,
            FmWatchEventKind::Modify,
            FmWatchEventKind::Remove,
        ] {
            assert_eq!(
                normalize_watch_events(watched, &[event(kind, &["/work/file.txt"])]),
                Some(FmWatchIntent::Refresh),
                "{kind:?} must refresh the watched directory"
            );
        }
    }

    #[test]
    fn rename_into_or_out_of_watched_dir_requests_refresh() {
        let watched = Path::new("/work");

        assert_eq!(
            normalize_watch_events(
                watched,
                &[event(
                    FmWatchEventKind::Rename,
                    &["/else/old.txt", "/work/new.txt"],
                )],
            ),
            Some(FmWatchIntent::Refresh),
            "move-in must refresh"
        );
        assert_eq!(
            normalize_watch_events(
                watched,
                &[event(
                    FmWatchEventKind::Rename,
                    &["/work/old.txt", "/else/new.txt"],
                )],
            ),
            Some(FmWatchIntent::Refresh),
            "move-out must refresh"
        );
    }

    #[test]
    fn duplicate_burst_coalesces_to_one_refresh_intent() {
        let events = vec![
            event(FmWatchEventKind::Create, &["/work/file.txt"]),
            event(FmWatchEventKind::Modify, &["/work/file.txt"]),
            event(FmWatchEventKind::Modify, &["/work/file.txt"]),
        ];

        assert_eq!(
            normalize_watch_events(Path::new("/work"), &events),
            Some(FmWatchIntent::Refresh)
        );
    }

    #[test]
    fn irrelevant_paths_and_non_content_events_are_ignored() {
        let events = vec![
            event(FmWatchEventKind::Create, &["/sibling/file.txt"]),
            event(FmWatchEventKind::Modify, &["/work/nested/file.txt"]),
            event(FmWatchEventKind::Rename, &["/else/a", "/else/b"]),
            event(FmWatchEventKind::Other, &["/work/file.txt"]),
        ];

        assert_eq!(normalize_watch_events(Path::new("/work"), &events), None);
    }

    #[test]
    fn watched_directory_event_requests_refresh() {
        assert_eq!(
            normalize_watch_events(
                Path::new("/work"),
                &[event(FmWatchEventKind::Remove, &["/work"])],
            ),
            Some(FmWatchIntent::Refresh)
        );
    }

    #[derive(Debug)]
    struct FakeBackend {
        drops: Arc<AtomicUsize>,
    }

    impl Drop for FakeBackend {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn watcher_slot_starts_once_for_an_unchanged_target() {
        let drops = Arc::new(AtomicUsize::new(0));
        let starts = Arc::new(AtomicUsize::new(0));
        let mut slot = FmWatcherSlot::default();

        let first_starts = starts.clone();
        let first_drops = drops.clone();
        assert_eq!(
            slot.sync_with(Some(Path::new("/work")), move |_, _| {
                first_starts.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(FakeBackend { drops: first_drops })
            }),
            Ok(FmWatcherSync::Started { generation: 1 })
        );

        let second_starts = starts.clone();
        let second_drops = drops.clone();
        assert_eq!(
            slot.sync_with(Some(Path::new("/work")), move |_, _| {
                second_starts.fetch_add(1, Ordering::SeqCst);
                Ok::<_, ()>(FakeBackend {
                    drops: second_drops,
                })
            }),
            Ok(FmWatcherSync::Unchanged)
        );
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(drops.load(Ordering::SeqCst), 0);
        assert_eq!(slot.watched_dir(), Some(Path::new("/work")));
        assert!(slot.has_backend());
    }

    #[test]
    fn watcher_slot_rebind_drops_old_backend_and_rejects_stale_generation() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut slot = FmWatcherSlot::default();
        let first_drops = drops.clone();
        slot.sync_with(Some(Path::new("/work")), move |_, _| {
            Ok::<_, ()>(FakeBackend { drops: first_drops })
        })
        .expect("start first watcher");
        let stale_generation = slot.generation();

        let second_drops = drops.clone();
        assert_eq!(
            slot.sync_with(Some(Path::new("/other")), move |_, _| {
                Ok::<_, ()>(FakeBackend {
                    drops: second_drops,
                })
            }),
            Ok(FmWatcherSync::Started { generation: 2 })
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(slot.watched_dir(), Some(Path::new("/other")));
        assert!(!slot.accepts_generation(stale_generation));
        assert!(slot.accepts_generation(2));
    }

    #[test]
    fn watcher_slot_close_drops_backend_and_rejects_prior_generation() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut slot = FmWatcherSlot::default();
        let backend_drops = drops.clone();
        slot.sync_with(Some(Path::new("/work")), move |_, _| {
            Ok::<_, ()>(FakeBackend {
                drops: backend_drops,
            })
        })
        .expect("start watcher");
        let stale_generation = slot.generation();

        assert_eq!(
            slot.sync_with(None, |_, _| -> Result<FakeBackend, ()> {
                panic!("close must not call the backend factory")
            }),
            Ok(FmWatcherSync::Stopped)
        );
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        assert_eq!(slot.watched_dir(), None);
        assert!(!slot.has_backend());
        assert!(!slot.accepts_generation(stale_generation));
    }

    #[test]
    fn watcher_slot_latches_start_failure_without_retrying_every_sync() {
        let starts = Arc::new(AtomicUsize::new(0));
        let mut slot = FmWatcherSlot::<FakeBackend>::default();

        let first_starts = starts.clone();
        assert_eq!(
            slot.sync_with(Some(Path::new("/denied")), move |_, _| {
                first_starts.fetch_add(1, Ordering::SeqCst);
                Err("permission denied")
            }),
            Err("permission denied")
        );
        let failed_generation = slot.generation();

        let second_starts = starts.clone();
        assert_eq!(
            slot.sync_with(Some(Path::new("/denied")), move |_, _| {
                second_starts.fetch_add(1, Ordering::SeqCst);
                Err("must not retry")
            }),
            Ok(FmWatcherSync::Unchanged)
        );
        assert_eq!(starts.load(Ordering::SeqCst), 1);
        assert_eq!(slot.watched_dir(), Some(Path::new("/denied")));
        assert!(!slot.has_backend());
        assert!(!slot.accepts_generation(failed_generation));
    }
}
