//! One worker per display, created the first time that display needs one.

use std::collections::HashMap;

use crate::app::state::ClientId;

/// A worker kept per display, keyed by whose view it is serving.
///
/// The file workers each hold one bounded in-flight request: a new request
/// supersedes the one before it, which is what keeps a fast scroll from
/// queueing a preview per row. With one browser that is exactly right. With
/// one browser per display it starves them: two displays each supersede the
/// other's request every tick and neither preview ever arrives.
///
/// Keying the worker by display restores the property the bounded slot was
/// relying on — one requester — without changing a line of the worker itself.
///
/// `None` is the session's own worker: the one the monolithic path uses, and
/// the only one that exists until a display is served.
pub(crate) struct PerDisplay<T> {
    workers: HashMap<Option<ClientId>, T>,
}

impl<T> Default for PerDisplay<T> {
    fn default() -> Self {
        Self {
            workers: HashMap::new(),
        }
    }
}

impl<T> PerDisplay<T> {
    /// The worker serving `viewer`, started if this is the first time that
    /// display has needed one.
    pub(crate) fn get_or_start(
        &mut self,
        viewer: Option<ClientId>,
        start: impl FnOnce() -> T,
    ) -> &mut T {
        self.workers.entry(viewer).or_insert_with(start)
    }

    /// Drops a departed display's worker.
    ///
    /// A worker left behind holds a thread and a channel for a display that
    /// will never ask it for anything again.
    pub(crate) fn forget(&mut self, client: ClientId) {
        self.workers.remove(&Some(client));
    }
}
