//! The ledger of recently closed agents — dead data rows, not processes.
//!
//! A closed agent is a *record* carrying an identity and a revival recipe:
//! no PTY, no runtime, no timer, no subscription. Nothing ticks for it, which
//! is what lets the panel show a graveyard of any size for free. The record
//! is written by a trigger (the agent closed), and removed by a trigger (it
//! was revived, or it aged out of the ring). Anything periodic here would be
//! a design regression, not an optimisation problem.

use std::collections::VecDeque;
use std::path::PathBuf;

/// How many closed agents the panel remembers.
///
/// The sidebar's height is finite: a separator plus eight grey rows sits under
/// the active cards without pushing them off screen. The bound itself is the
/// important part — closing agents is one of the most frequent gestures there
/// is, and an unbounded list is unbounded memory wearing a feature's name.
pub(crate) const CLOSED_AGENT_CAPACITY: usize = 8;

/// Where a ghost is in its journey back to life.
///
/// Revival is a one-way transition, not a debounce: a second click during
/// `Reviving` is inert because the state already says a spawn is in flight.
/// Timing plays no part, so the guarantee is testable on the slowest machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevivalState {
    /// A dead row waiting to be clicked.
    Dormant,
    /// A spawn is in flight; further clicks mean nothing until it lands.
    Reviving,
}

/// Everything a revival needs, frozen at the moment the agent closed.
///
/// Each field earns its place by what breaks without it: the identity keeps
/// one agent from haunting the list twice, the cwd is the whole difference
/// between reopening where the user worked and reopening in `$HOME`, and the
/// session key is what lets the chat drawer reattach to the same conversation
/// instead of presenting the revival as a stranger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClosedAgentRecord {
    /// Stable identity; the dedup key and what a revival resolves back to.
    pub agent_id: String,
    /// The name the grey row wears; live state is gone, so it travels here.
    pub label: String,
    /// Where the agent last worked. Derived-at-revival was #46's root cause.
    /// `None` when the close never knew one — the ghost still shows, and the
    /// revival is the layer that refuses to guess (never `$HOME`).
    pub cwd: Option<PathBuf>,
    /// Which workspace/branch the revival returns under.
    pub workspace_key: Option<String>,
    /// The resume recipe, frozen at close time: the agent's own reported
    /// session (source, agent, session ref) — everything the existing resume
    /// roads need to bring the conversation back. `None` when the close never
    /// knew one; such a ghost is visible history whose revival is refused
    /// with a reason rather than launching a stranger.
    pub session: Option<crate::agent_resume::PersistedAgentSession>,
    /// When it closed, in milliseconds; newest-first ordering and eviction.
    pub closed_at: u64,
    /// The revival state machine (see [`RevivalState`]).
    pub revival: RevivalState,
}

impl ClosedAgentRecord {
    /// The session value a revival resumes and wires its new tab to.
    pub fn session_value(&self) -> Option<&str> {
        self.session
            .as_ref()
            .map(|session| session.session_ref.value.as_str())
    }
}

/// Why a revival was refused. Every variant is a visible reason: the
/// alternative to refusing is guessing, and #46 measured exactly where a
/// guessed directory lands ($HOME). A refusal never flips the row's state —
/// the ghost stays clickable for the moment the answer changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RevivalRefusal {
    /// No record carries this identity — a stale click, or the row aged out.
    UnknownGhost,
    /// A spawn is already in flight; the click is inert, not queued.
    RevivalInFlight,
    /// The record closed without a directory; reviving would mean guessing.
    NoCwd,
    /// The recorded directory no longer exists (worktree removed).
    CwdGone,
    /// The record closed without a session recipe; there is nothing to resume.
    NoSessionRecipe,
    /// The spawn road produced no wired tab; the ghost returns to rest.
    SpawnFailed,
}

impl RevivalRefusal {
    /// Stable machine-readable code for the API surface.
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::UnknownGhost => "unknown_closed_agent",
            Self::RevivalInFlight => "revival_in_flight",
            Self::NoCwd => "closed_agent_cwd_unknown",
            Self::CwdGone => "closed_agent_cwd_gone",
            Self::NoSessionRecipe => "closed_agent_session_unknown",
            Self::SpawnFailed => "revival_spawn_failed",
        }
    }

    /// The visible reason behind the code.
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::UnknownGhost => "no recently closed agent carries this id",
            Self::RevivalInFlight => "a revival for this agent is already in flight",
            Self::NoCwd => "the closed agent never recorded a working directory",
            Self::CwdGone => "the recorded working directory no longer exists",
            Self::NoSessionRecipe => "the closed agent has no session to resume",
            Self::SpawnFailed => "the revival spawn produced no tab",
        }
    }
}

/// A spawnable launch: the argv and the extra environment it carries.
pub(crate) type LaunchRecipe = (Vec<String>, Vec<(String, String)>);

/// The launch a revival uses: claude rides the chat drawer's own launch
/// (identical flags, identical env), every other agent gets the plan the
/// session-restore road builds for it. A revival argv invented here would be
/// a third road for the next UI change to forget.
pub(crate) fn revival_launch(
    session: &crate::agent_resume::PersistedAgentSession,
) -> Option<LaunchRecipe> {
    if session.agent == "claude" {
        return Some(super::projects::project_chat_launch(
            "claude",
            Some(session.session_ref.value.as_str()),
        ));
    }
    crate::agent_resume::plan(&session.source, &session.agent, &session.session_ref)
        .map(|plan| (plan.argv, Vec::new()))
}

impl super::App {
    /// Revive a ghost: spawn its agent back in the recorded directory, wired
    /// to the recorded conversation, riding the same road a chat-drawer
    /// resume takes — including its focus-the-wired-tab spam guard.
    ///
    /// Validation happens before the claim so a refusal leaves the row
    /// dormant and clickable; the claim happens before the spawn so two fast
    /// clicks cannot both win (TP-AGPANEL-10).
    pub(crate) fn revive_closed_agent(&mut self, agent_id: &str) -> Result<(), RevivalRefusal> {
        let Some(record) = self
            .state
            .closed_agents
            .entries()
            .find(|record| record.agent_id == agent_id)
        else {
            return Err(RevivalRefusal::UnknownGhost);
        };
        if record.revival == RevivalState::Reviving {
            return Err(RevivalRefusal::RevivalInFlight);
        }
        let Some(cwd) = record.cwd.clone() else {
            return Err(RevivalRefusal::NoCwd);
        };
        let Some(session) = record.session.clone() else {
            return Err(RevivalRefusal::NoSessionRecipe);
        };
        if !cwd.is_dir() {
            return Err(RevivalRefusal::CwdGone);
        }
        let Some((argv, extra_env)) = revival_launch(&session) else {
            return Err(RevivalRefusal::NoSessionRecipe);
        };
        let session_value = session.session_ref.value;

        if !self.state.closed_agents.try_begin_revival(agent_id) {
            return Err(RevivalRefusal::RevivalInFlight);
        }
        self.open_project_chat_tab_with_argv(
            crate::app::state::ProjectChatTabRequest {
                project_path: cwd,
                session_id: Some(session_value.clone()),
            },
            &argv,
            extra_env,
        );
        // The road wires whichever tab it spawned (or focused) to the session;
        // that wiring is the road's own receipt that the revival landed.
        if self.state.find_resumed_chat_tab(&session_value).is_some() {
            self.state.closed_agents.resolve_revival(agent_id);
            Ok(())
        } else {
            self.state.closed_agents.abort_revival(agent_id);
            Err(RevivalRefusal::SpawnFailed)
        }
    }
}

/// The fixed-capacity, newest-first ring of closed agents.
#[derive(Debug, Default)]
pub(crate) struct ClosedAgentLedger {
    records: VecDeque<ClosedAgentRecord>,
}

impl ClosedAgentLedger {
    /// Remember a closed agent, newest first.
    ///
    /// Closing an agent that is already remembered refreshes its row rather
    /// than adding a second one: two ghosts with one identity would leave the
    /// user guessing which of them revives, and the answer would be neither
    /// reliably. The refreshed row starts `Dormant` again — whatever revival
    /// was in flight belonged to a life that has since ended. Beyond capacity
    /// the oldest row is evicted.
    pub fn record_closed(&mut self, mut record: ClosedAgentRecord) {
        // A fresh death always starts dormant, whatever the writer handed in:
        // any revival that was in flight belonged to a life that just ended.
        record.revival = RevivalState::Dormant;
        self.records.retain(|r| r.agent_id != record.agent_id);
        self.records.push_front(record);
        self.records.truncate(CLOSED_AGENT_CAPACITY);
    }

    /// The remembered ghosts, newest first.
    pub fn entries(&self) -> impl Iterator<Item = &ClosedAgentRecord> {
        self.records.iter()
    }

    /// Rebuild from what the store kept, newest first.
    ///
    /// TP-AGPANEL-34: a loaded ghost is always `Dormant`. Revival state
    /// describes *this* process — a spawn that was in flight when the server
    /// was replaced did not survive the replacement, and a row that came back
    /// claiming `Reviving` would be inert forever, because the claim it is
    /// waiting on belongs to a process that no longer exists.
    ///
    /// Records that cannot be understood are skipped rather than fatal: this
    /// runs at startup, and one malformed row must not cost the whole
    /// graveyard.
    pub fn load_stored(&mut self, stored: Vec<crate::persist::closed_agents::StoredClosedAgent>) {
        self.records = stored
            .into_iter()
            .map(|row| ClosedAgentRecord {
                agent_id: row.agent_id,
                label: row.label,
                cwd: row.cwd.map(PathBuf::from),
                workspace_key: row.workspace_key,
                session: row
                    .session
                    .map(|session| crate::agent_resume::PersistedAgentSession {
                        source: session.source,
                        agent: session.agent,
                        session_ref: crate::agent_resume::AgentSessionRef {
                            kind: session.ref_kind,
                            value: session.ref_value,
                        },
                    }),
                closed_at: row.closed_at,
                revival: RevivalState::Dormant,
            })
            .collect();
    }

    /// Project to the disk shape, newest first.
    pub fn to_stored(&self) -> Vec<crate::persist::closed_agents::StoredClosedAgent> {
        self.records
            .iter()
            .map(|row| crate::persist::closed_agents::StoredClosedAgent {
                agent_id: row.agent_id.clone(),
                label: row.label.clone(),
                cwd: row
                    .cwd
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
                workspace_key: row.workspace_key.clone(),
                session: row.session.as_ref().map(|session| {
                    crate::persist::closed_agents::StoredSession {
                        source: session.source.clone(),
                        agent: session.agent.clone(),
                        ref_kind: session.session_ref.kind,
                        ref_value: session.session_ref.value.clone(),
                    }
                }),
                closed_at: row.closed_at,
            })
            .collect()
    }

    /// Claim the right to revive a ghost — the atomic half of spam safety.
    ///
    /// Returns `true` exactly once per dormancy: the transition to `Reviving`
    /// happens here, before any spawn is attempted, so two fast clicks cannot
    /// both come back `true`. A ghost that is unknown or already reviving
    /// yields `false` and the caller does nothing — a stale target is inert,
    /// the same principle TP-AGPANEL-06 pinned for the chat road.
    pub fn try_begin_revival(&mut self, agent_id: &str) -> bool {
        match self.records.iter_mut().find(|r| r.agent_id == agent_id) {
            Some(row) if row.revival == RevivalState::Dormant => {
                row.revival = RevivalState::Reviving;
                true
            }
            _ => false,
        }
    }

    /// The spawn landed: the row leaves the graveyard.
    ///
    /// From here on the agent is an ordinary live one, and clicking it means
    /// focus, not revival — the ordinary panel behaviour, no special case.
    pub fn resolve_revival(&mut self, agent_id: &str) {
        self.records.retain(|r| r.agent_id != agent_id);
    }

    /// The spawn failed: the ghost returns to rest, clickable again.
    ///
    /// Without this, one failed revival would brick the row forever in
    /// `Reviving` — inert to every later click for no reason the user can see.
    pub fn abort_revival(&mut self, agent_id: &str) {
        if let Some(row) = self.records.iter_mut().find(|r| r.agent_id == agent_id) {
            if row.revival == RevivalState::Reviving {
                row.revival = RevivalState::Dormant;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str, closed_at: u64) -> ClosedAgentRecord {
        ClosedAgentRecord {
            agent_id: id.to_string(),
            label: format!("agent {id}"),
            cwd: Some(PathBuf::from("/tmp")),
            workspace_key: None,
            session: crate::agent_resume::AgentSessionRef::id(format!("session-{id}")).map(
                |session_ref| crate::agent_resume::PersistedAgentSession {
                    source: "herdr:claude".into(),
                    agent: "claude".into(),
                    session_ref,
                },
            ),
            closed_at,
            revival: RevivalState::Dormant,
        }
    }

    fn ids(ledger: &ClosedAgentLedger) -> Vec<String> {
        ledger.entries().map(|r| r.agent_id.clone()).collect()
    }

    // TP-AGPANEL-34: revival state describes THIS process, so it must not
    // survive one. A spawn that was in flight when the server was replaced did
    // not survive the replacement, and a row that came back claiming
    // `Reviving` would be inert forever — the claim it waits on belongs to a
    // process that no longer exists, and the row would refuse every click with
    // `RevivalInFlight`.
    #[test]
    fn a_loaded_ghost_starts_dormant_whatever_it_was_doing() {
        let mut ledger = ClosedAgentLedger::default();
        ledger.record_closed(record("a", 1));
        assert!(
            ledger.try_begin_revival("a"),
            "precondition: the ghost is mid-revival when the store is written"
        );

        let mut restored = ClosedAgentLedger::default();
        restored.load_stored(ledger.to_stored());

        assert_eq!(
            restored.entries().next().map(|r| r.revival),
            Some(RevivalState::Dormant),
            "a ghost that comes back mid-revival could never be clicked again"
        );
    }

    // TP-AGPANEL-37: everything a revival needs crosses the disk. The cwd is
    // the whole difference between reopening where the user worked and
    // reopening in `$HOME` (#46), and the session ref is what reattaches the
    // conversation instead of presenting a stranger — a ghost that survives a
    // restart without them is a row that can only refuse.
    #[test]
    fn a_round_trip_keeps_what_a_revival_needs() {
        let mut ledger = ClosedAgentLedger::default();
        ledger.record_closed(record("keep", 42));

        let mut restored = ClosedAgentLedger::default();
        restored.load_stored(ledger.to_stored());

        let before = ledger.entries().next().expect("written");
        let after = restored.entries().next().expect("read back");
        assert_eq!(after.agent_id, before.agent_id);
        assert_eq!(after.label, before.label);
        assert_eq!(
            after.cwd, before.cwd,
            "the revival directory crosses the disk"
        );
        assert_eq!(after.closed_at, before.closed_at);
        assert_eq!(
            after.session.as_ref().map(|s| s.session_ref.value.clone()),
            before.session.as_ref().map(|s| s.session_ref.value.clone()),
            "the resume key crosses the disk"
        );
        assert_eq!(
            after.session.as_ref().map(|s| s.session_ref.kind),
            before.session.as_ref().map(|s| s.session_ref.kind),
            "an id and a transcript path resume differently; the kind must survive"
        );
    }

    // TP-AGPANEL-07: the graveyard is newest first — "recently closed" is an
    // ordering claim, and without it the words on the panel lie.
    #[test]
    fn a_closed_agent_is_remembered_newest_first() {
        let mut ledger = ClosedAgentLedger::default();
        ledger.record_closed(record("a", 1));
        ledger.record_closed(record("b", 2));
        assert_eq!(ids(&ledger), vec!["b", "a"]);
    }

    // TP-AGPANEL-08: one identity, one ghost. Closing the same agent again
    // refreshes the row (newest position, dormant again) instead of adding a
    // twin the user cannot tell from the original.
    #[test]
    fn closing_the_same_agent_again_refreshes_the_row_instead_of_duplicating() {
        let mut ledger = ClosedAgentLedger::default();
        ledger.record_closed(record("a", 1));
        ledger.record_closed(record("b", 2));
        assert!(ledger.try_begin_revival("a"));
        let mut again = record("a", 3);
        again.revival = RevivalState::Reviving; // yazan taraf ne derse desin —
        ledger.record_closed(again); // taze bir ölüm Dormant başlar
        assert_eq!(ids(&ledger), vec!["a", "b"]);
        let a = ledger.entries().next().expect("a is present");
        assert_eq!(a.closed_at, 3);
        assert_eq!(a.revival, RevivalState::Dormant);
    }

    // TP-AGPANEL-09: the ring is bounded. Closing agents is a daily gesture
    // by the hundreds; without eviction the grey list is a leak with a UI.
    #[test]
    fn the_ledger_evicts_its_oldest_beyond_capacity() {
        let mut ledger = ClosedAgentLedger::default();
        for i in 0..=CLOSED_AGENT_CAPACITY {
            ledger.record_closed(record(&format!("g{i}"), i as u64));
        }
        let remembered = ids(&ledger);
        assert_eq!(remembered.len(), CLOSED_AGENT_CAPACITY);
        assert!(
            !remembered.contains(&"g0".to_string()),
            "en eski tahliye edilir"
        );
        assert_eq!(remembered.first().map(String::as_str), Some("g8"));
    }

    // TP-AGPANEL-10: revival is a one-way claim, not a debounce. The second
    // caller gets `false` from the state itself — no window, however fast the
    // clicks or slow the machine, in which two spawns can both win.
    #[test]
    fn a_second_revival_attempt_while_one_is_in_flight_is_inert() {
        let mut ledger = ClosedAgentLedger::default();
        ledger.record_closed(record("a", 1));
        assert!(
            ledger.try_begin_revival("a"),
            "ilk tıklama spawn hakkını alır"
        );
        assert!(
            !ledger.try_begin_revival("a"),
            "uçuştayken ikinci tıklama atıldır"
        );
        assert!(!ledger.try_begin_revival("yok"), "bilinmeyen hedef atıldır");
    }

    // TP-AGPANEL-11: a landed revival leaves the graveyard; a failed one
    // returns to rest. Either way the row never sticks in `Reviving` — a
    // ghost bricked by one failure would be inert to every later click.
    #[test]
    fn a_finished_revival_leaves_and_a_failed_one_rests_again() {
        let mut ledger = ClosedAgentLedger::default();
        ledger.record_closed(record("a", 1));
        ledger.record_closed(record("b", 2));

        assert!(ledger.try_begin_revival("a"));
        ledger.resolve_revival("a");
        assert_eq!(ids(&ledger), vec!["b"], "inen diriltme mezarlıktan çıkar");

        assert!(ledger.try_begin_revival("b"));
        ledger.abort_revival("b");
        assert!(
            ledger.try_begin_revival("b"),
            "başarısız diriltme yeniden tıklanabilir"
        );
    }

    fn revival_test_app() -> crate::app::App {
        let (_api_tx, api_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::app::App::new(
            &crate::config::Config::default(),
            true,
            None,
            api_rx,
            crate::api::EventHub::default(),
        );
        app.state.workspaces = vec![crate::workspace::Workspace::test_new("revive")];
        app.state.ensure_test_terminals();
        app.state.active = Some(0);
        app
    }

    // TP-AGPANEL-18: a revival that cannot answer for its directory is
    // refused with a reason, and the refusal leaves the row dormant — still
    // clickable for the moment the answer changes. Guessing instead of
    // refusing is the exact #46 failure ($HOME) this feature exists to bury.
    #[test]
    fn a_revival_without_a_livable_directory_is_refused_and_stays_clickable() {
        let mut app = revival_test_app();
        let mut no_cwd = record("no-cwd", 1);
        no_cwd.cwd = None;
        app.state.closed_agents.record_closed(no_cwd);
        let mut gone = record("gone", 2);
        gone.cwd = Some(PathBuf::from("/nonexistent/definitely-gone-herdr-test"));
        app.state.closed_agents.record_closed(gone);

        assert_eq!(
            app.revive_closed_agent("no-cwd"),
            Err(RevivalRefusal::NoCwd)
        );
        assert_eq!(
            app.revive_closed_agent("gone"),
            Err(RevivalRefusal::CwdGone)
        );
        assert_eq!(
            app.revive_closed_agent("nobody"),
            Err(RevivalRefusal::UnknownGhost)
        );
        // Every refusal left its row dormant: a later claim still succeeds…
        assert!(app.state.closed_agents.try_begin_revival("no-cwd"));
        assert!(app.state.closed_agents.try_begin_revival("gone"));
        // …and a row already claimed reports the flight instead of racing it.
        assert_eq!(
            app.revive_closed_agent("gone"),
            Err(RevivalRefusal::RevivalInFlight)
        );
    }

    // TP-AGPANEL-19: a ghost that closed without a session recipe has nothing
    // to resume. The click is refused with a reason instead of silently
    // launching a stranger and calling it a revival.
    #[test]
    fn a_revival_without_a_session_recipe_is_refused() {
        let mut app = revival_test_app();
        let mut ghost = record("recipe-less", 1);
        ghost.session = None;
        app.state.closed_agents.record_closed(ghost);
        assert_eq!(
            app.revive_closed_agent("recipe-less"),
            Err(RevivalRefusal::NoSessionRecipe)
        );
        assert!(app.state.closed_agents.try_begin_revival("recipe-less"));
    }

    // TP-AGPANEL-20: a ghost whose conversation already lives in a wired tab
    // must not spawn a twin (#45's lesson wearing grey). The revival rides
    // the chat road's own guard: it focuses the wired tab, and the ghost
    // leaves the graveyard — clicking again is no longer a revival at all.
    #[test]
    fn a_ghost_whose_session_is_already_open_focuses_it_instead_of_twinning() {
        let mut app = revival_test_app();
        app.state.workspaces[0].test_add_tab(None);
        app.state.ensure_test_terminals();
        app.state.workspaces[0].tabs[1].resumed_session_id = Some("session-open".into());
        let before = app.state.workspaces[0].tabs.len();

        app.state.closed_agents.record_closed(record("open", 1));
        assert_eq!(app.revive_closed_agent("open"), Ok(()));

        assert_eq!(app.state.workspaces[0].tabs.len(), before, "no twin tab");
        assert_eq!(
            app.state.workspaces[0].active_tab_index(),
            1,
            "the wired tab took focus"
        );
        assert!(
            app.state.closed_agents.entries().next().is_none(),
            "a revival that landed leaves the graveyard"
        );
        assert_eq!(
            app.revive_closed_agent("open"),
            Err(RevivalRefusal::UnknownGhost),
            "the next click is ordinary focus territory, not revival"
        );
    }

    // TP-AGPANEL-21: every ghost revives along its own agent's existing road —
    // claude with the chat drawer's exact launch (flags and env included),
    // codex with the restore road's plan. A revival argv invented here would
    // be a third road for the next change to forget.
    #[test]
    fn a_revival_launch_matches_the_agents_existing_road() {
        let claude = crate::agent_resume::PersistedAgentSession {
            source: "herdr:claude".into(),
            agent: "claude".into(),
            session_ref: crate::agent_resume::AgentSessionRef::id("abc").expect("valid id"),
        };
        let (argv, env) = revival_launch(&claude).expect("claude revives");
        assert_eq!(
            argv,
            vec![
                "claude".to_string(),
                "--dangerously-skip-permissions".into(),
                "--resume".into(),
                "abc".into(),
            ]
        );
        assert!(env.iter().any(|(key, _)| key == "ENABLE_BACKGROUND_TASKS"));

        let codex = crate::agent_resume::PersistedAgentSession {
            source: "herdr:codex".into(),
            agent: "codex".into(),
            session_ref: crate::agent_resume::AgentSessionRef::id("xyz").expect("valid id"),
        };
        let (argv, env) = revival_launch(&codex).expect("codex revives");
        assert_eq!(
            argv,
            vec!["codex".to_string(), "resume".into(), "xyz".into()]
        );
        assert!(env.is_empty());
    }
}
