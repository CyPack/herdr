//! Which capturable audio stream belongs to which pane — the pure half.
//!
//! A pane that plays sound does not own the process that produces it. A
//! browser hands its audio to a separate helper, and on the measured machine
//! that helper sits two levels below the pane's own command:
//!
//! ```text
//! audio helper  <-  browser daemon  <-  launcher (the pane's shell_pid)  <-  server
//! ```
//!
//! So the question "is this stream the pane's?" is answered by the *chain*,
//! never by the producing pid alone. Three further facts, all measured on the
//! live tree rather than assumed, shape the rules below:
//!
//! * The environment marker a linked web pane carries is set on the launcher
//!   and on nothing else — not the daemon, not the renderer, not the audio
//!   helper. It is therefore a chain fact too, and a weaker one than ancestry.
//! * A sound server that bridges another protocol can report *its own* pid for
//!   every stream it forwards, which would make one wrong owner look like the
//!   owner of everything. Such a candidate arrives here with no pid at all.
//! * A machine always has other producers. During the measurement a speech
//!   daemon was live; a rule that picks "the only stream playing" would have
//!   shipped its sound to a remote client.
//!
//! Nothing in this module reads `/proc` or names an audio platform: the
//! platform layer gathers the facts, this decides. That split is what lets the
//! decision be tested against a topology nobody has to reproduce.
//!
//! TP-MEDIA-OWNER-01.

// Unused until the supervisor that consults it lands: the rules are built and
// pinned first so the driver has something already tested to call, the same
// staging the damage differ used.
//
// REMOVAL CONDITION: delete this attribute the moment `match_pane_source` is
// called from the pane-audio supervisor — after that, a dead item here is a
// real leak, not a staged one.
#![allow(dead_code)]

use std::collections::BTreeSet;

/// One capturable output stream, already enriched by the platform layer with
/// the facts the match needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceCandidate {
    /// Platform-side handle for the stream.
    pub(crate) node_id: u32,
    /// The producing process, when the platform could honestly name one.
    /// `None` when the only pid on offer belonged to the sound server's own
    /// bridge rather than to a program the user started.
    pub(crate) pid: Option<u32>,
    /// Ancestors of `pid`, nearest first, with `init` left out.
    pub(crate) ancestors: Vec<u32>,
    /// Whether `pid` or any of its ancestors carries this pane's environment
    /// marker.
    pub(crate) carries_pane_marker: bool,
    /// The producer's own name, for the last-resort rule.
    pub(crate) app_name: Option<String>,
}

/// The pane's own processes, as the server already knows them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PaneProcesses {
    pub(crate) pids: BTreeSet<u32>,
    pub(crate) names: BTreeSet<String>,
}

/// Which rule produced a match. Kept because a match found by the weakest rule
/// deserves a different line in the log than one found by ancestry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchHow {
    /// The producer, or one of its ancestors, is a process of this pane.
    Ancestry,
    /// Someone in the chain carries the pane's environment marker.
    Marker,
    /// The producer names itself after one of the pane's own processes.
    Name,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceMatch {
    /// Nothing this pane owns is making sound. The ordinary case, not an error.
    None,
    One {
        node_id: u32,
        how: MatchHow,
    },
    /// More than one stream fits and no rule separates them. Deliberately not
    /// a guess: sending another program's sound to a remote listener is worse
    /// than sending none, and a guess leaves no trace of why it was wrong.
    Ambiguous(Vec<u32>),
}

/// Picks the pane's stream, or says honestly that it cannot.
///
/// The rules are tried strongest first — ancestry, then the environment
/// marker, then the name. A rule that finds two candidates ends the search as
/// [`SourceMatch::Ambiguous`] instead of falling through: a stronger rule that
/// cannot separate two streams has not failed, it has found a real ambiguity,
/// and letting a weaker rule break the tie would be guessing with extra steps.
pub(crate) fn match_pane_source(
    candidates: &[SourceCandidate],
    pane: &PaneProcesses,
) -> SourceMatch {
    let rules = [
        (MatchHow::Ancestry, by_ancestry(candidates, pane)),
        (MatchHow::Marker, by_marker(candidates)),
        (MatchHow::Name, by_name(candidates, pane)),
    ];
    for (how, hits) in rules {
        match hits.len() {
            0 => continue,
            1 => {
                return SourceMatch::One {
                    node_id: hits[0],
                    how,
                }
            }
            _ => return SourceMatch::Ambiguous(hits),
        }
    }
    SourceMatch::None
}

/// The producer, or something it descends from, is a process of this pane.
fn by_ancestry(candidates: &[SourceCandidate], pane: &PaneProcesses) -> Vec<u32> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate.pid.is_some_and(|pid| pane.pids.contains(&pid))
                || candidate
                    .ancestors
                    .iter()
                    .any(|ancestor| pane.pids.contains(ancestor))
        })
        .map(|candidate| candidate.node_id)
        .collect()
}

/// Someone in the chain carries the pane's environment marker. Weaker than
/// ancestry because a marker survives being inherited by a process the pane no
/// longer owns, while a parent link cannot be inherited by mistake.
fn by_marker(candidates: &[SourceCandidate]) -> Vec<u32> {
    candidates
        .iter()
        .filter(|candidate| candidate.carries_pane_marker)
        .map(|candidate| candidate.node_id)
        .collect()
}

/// The producer names itself after one of the pane's own processes. Matched
/// exactly, not case-insensitively: this is the weakest rule, so it is also
/// the one where a false positive costs the most, and no measurement yet
/// justifies widening it.
fn by_name(candidates: &[SourceCandidate], pane: &PaneProcesses) -> Vec<u32> {
    candidates
        .iter()
        .filter(|candidate| {
            candidate
                .app_name
                .as_deref()
                .is_some_and(|name| pane.names.contains(name))
        })
        .map(|candidate| candidate.node_id)
        .collect()
}

/// A pane as the supervisor sees it at one moment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaneSourceState {
    pub(crate) pane_id: String,
    pub(crate) matched: SourceMatch,
}

/// Why a capture is being stopped. Carried because "the video ended" and "the
/// last listener left" look identical from the outside and need different
/// lines in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CloseReason {
    /// No connected client can play audio any more.
    NoListener,
    /// The pane itself is gone.
    PaneGone,
    /// The pane lives, but nothing it owns is making sound.
    SourceEnded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SourceAction {
    Open {
        pane_id: String,
        node_id: u32,
    },
    Close {
        pane_id: String,
        reason: CloseReason,
    },
}

/// Decides what to start and what to stop, from the whole picture rather than
/// from the event that woke it.
///
/// Recomputing the plan means a missed event costs a late decision, never a
/// wrong one — the alternative, mutating state per event, is where a capture
/// survives the pane that owned it.
///
/// Closes come before opens so a machine that is at its limit gives up a
/// capture before asking for another.
pub(crate) fn plan(
    panes: &[PaneSourceState],
    open: &BTreeSet<String>,
    listeners: usize,
) -> Vec<SourceAction> {
    let _ = (panes, open, listeners);
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pane(pids: &[u32], names: &[&str]) -> PaneProcesses {
        PaneProcesses {
            pids: pids.iter().copied().collect(),
            names: names.iter().map(|n| (*n).to_string()).collect(),
        }
    }

    fn candidate(node_id: u32, pid: Option<u32>, ancestors: &[u32]) -> SourceCandidate {
        SourceCandidate {
            node_id,
            pid,
            ancestors: ancestors.to_vec(),
            carries_pane_marker: false,
            app_name: None,
        }
    }

    fn pane_state(pane_id: &str, matched: SourceMatch) -> PaneSourceState {
        PaneSourceState {
            pane_id: pane_id.to_string(),
            matched,
        }
    }

    fn matched_one(node_id: u32) -> SourceMatch {
        SourceMatch::One {
            node_id,
            how: MatchHow::Ancestry,
        }
    }

    fn open_set(pane_ids: &[&str]) -> BTreeSet<String> {
        pane_ids.iter().map(|id| (*id).to_string()).collect()
    }

    fn opened(pane_id: &str, node_id: u32) -> SourceAction {
        SourceAction::Open {
            pane_id: pane_id.to_string(),
            node_id,
        }
    }

    fn closed(pane_id: &str, reason: CloseReason) -> SourceAction {
        SourceAction::Close {
            pane_id: pane_id.to_string(),
            reason,
        }
    }

    /// SV-1 — nobody can hear it, so nothing is started. The whole feature
    /// costs zero while no client has an audio sink.
    #[test]
    fn without_a_listener_nothing_is_started() {
        let panes = [pane_state("w1:p1", matched_one(7))];
        assert_eq!(plan(&panes, &open_set(&[]), 0), Vec::new());
    }

    /// SV-2 — the ordinary case.
    #[test]
    fn a_matched_pane_with_a_listener_is_opened() {
        let panes = [pane_state("w1:p1", matched_one(7))];
        assert_eq!(plan(&panes, &open_set(&[]), 1), vec![opened("w1:p1", 7)]);
    }

    /// SV-3 — the last listener leaving stops the capture. A recorder left
    /// running for nobody is both wasted work and a microphone-shaped
    /// surprise on someone's machine.
    #[test]
    fn the_last_listener_leaving_closes_what_is_open() {
        let panes = [pane_state("w1:p1", matched_one(7))];
        assert_eq!(
            plan(&panes, &open_set(&["w1:p1"]), 0),
            vec![closed("w1:p1", CloseReason::NoListener)]
        );
    }

    /// SV-4 — a capture must never outlive the pane that owned it.
    #[test]
    fn a_vanished_pane_closes_its_capture() {
        assert_eq!(
            plan(&[], &open_set(&["w1:p1"]), 1),
            vec![closed("w1:p1", CloseReason::PaneGone)]
        );
    }

    /// SV-5 — the video ended. Closing here is what makes the next video
    /// openable: a channel held open for a silent pane spends bandwidth and
    /// hides the moment sound returns.
    #[test]
    fn a_pane_that_fell_silent_closes_its_capture() {
        let panes = [pane_state("w1:p1", SourceMatch::None)];
        assert_eq!(
            plan(&panes, &open_set(&["w1:p1"]), 1),
            vec![closed("w1:p1", CloseReason::SourceEnded)]
        );
    }

    /// SV-6 — an already-open pane is not opened again. The session layer
    /// would refuse the second open as a conflict; producing it at all would
    /// turn a healthy state into an error line every tick.
    #[test]
    fn an_open_pane_is_not_opened_twice() {
        let panes = [pane_state("w1:p1", matched_one(7))];
        assert_eq!(plan(&panes, &open_set(&["w1:p1"]), 1), Vec::new());
    }

    /// SV-7 — a second stream appearing under a pane that is already being
    /// captured changes nothing: the capture already names one node, and
    /// cutting the sound mid-video to re-decide would be a worse answer than
    /// keeping the one that is playing.
    #[test]
    fn an_open_pane_that_turns_ambiguous_keeps_playing() {
        let panes = [pane_state("w1:p1", SourceMatch::Ambiguous(vec![7, 8]))];
        assert_eq!(plan(&panes, &open_set(&["w1:p1"]), 1), Vec::new());
    }

    /// SV-8 — a machine at its limit gives one up before asking for another.
    #[test]
    fn closes_are_planned_before_opens() {
        let panes = [
            pane_state("w1:p1", SourceMatch::None),
            pane_state("w1:p2", matched_one(9)),
        ];
        assert_eq!(
            plan(&panes, &open_set(&["w1:p1"]), 1),
            vec![
                closed("w1:p1", CloseReason::SourceEnded),
                opened("w1:p2", 9),
            ]
        );
    }

    /// ID-1 — the live topology: helper <- daemon <- launcher, and the
    /// launcher is the pane's own process.
    #[test]
    fn a_stream_whose_chain_reaches_the_pane_is_the_panes() {
        let candidates = [candidate(112, Some(2553031), &[2539534, 2539493, 2511933])];
        assert_eq!(
            match_pane_source(&candidates, &pane(&[2539493], &[])),
            SourceMatch::One {
                node_id: 112,
                how: MatchHow::Ancestry
            }
        );
    }

    /// ID-2 — another program's sound never becomes this pane's sound.
    #[test]
    fn a_foreign_producer_is_not_matched() {
        let candidates = [candidate(9, Some(4242), &[4026, 1000])];
        assert_eq!(
            match_pane_source(&candidates, &pane(&[2539493], &["electron"])),
            SourceMatch::None
        );
    }

    /// ID-3 — a bridge that reports its own pid for everything arrives with no
    /// pid, so it can never be mistaken for the owner by ancestry.
    #[test]
    fn a_bridge_without_an_honest_pid_matches_nothing_by_ancestry() {
        let candidates = [candidate(112, None, &[])];
        assert_eq!(
            match_pane_source(&candidates, &pane(&[2539493], &[])),
            SourceMatch::None
        );
    }

    /// ID-4 — the marker sits on the launcher, never on the audio helper, so
    /// the chain is what carries it.
    #[test]
    fn the_environment_marker_is_read_along_the_chain() {
        let mut only = candidate(77, Some(2553031), &[2539534, 2539493]);
        only.carries_pane_marker = true;
        assert_eq!(
            match_pane_source(&[only], &pane(&[], &[])),
            SourceMatch::One {
                node_id: 77,
                how: MatchHow::Marker
            }
        );
    }

    /// ID-5 — two streams of the same pane are an ambiguity, not a coin toss.
    #[test]
    fn two_streams_of_one_pane_are_ambiguous() {
        let candidates = [
            candidate(1, Some(10), &[2539493]),
            candidate(2, Some(11), &[2539493]),
        ];
        assert_eq!(
            match_pane_source(&candidates, &pane(&[2539493], &[])),
            SourceMatch::Ambiguous(vec![1, 2])
        );
    }

    /// ID-6 — silence is the common case and must not read as a failure.
    #[test]
    fn no_candidates_is_no_match() {
        assert_eq!(
            match_pane_source(&[], &pane(&[2539493], &["electron"])),
            SourceMatch::None
        );
    }

    /// ID-7 — the last resort: a producer that names itself after one of the
    /// pane's processes, used only when nothing stronger applies.
    #[test]
    fn the_name_rule_is_the_last_resort() {
        let mut named = candidate(5, None, &[]);
        named.app_name = Some("electron".to_string());
        assert_eq!(
            match_pane_source(&[named], &pane(&[2539493], &["electron"])),
            SourceMatch::One {
                node_id: 5,
                how: MatchHow::Name
            }
        );
    }

    /// ID-8 — a stronger rule that finds two answers ends the search. Letting
    /// the name rule break an ancestry tie would be a guess wearing a rule's
    /// clothes.
    #[test]
    fn an_ambiguous_strong_rule_does_not_fall_through_to_a_weaker_one() {
        let mut first = candidate(1, Some(10), &[2539493]);
        first.app_name = Some("electron".to_string());
        let second = candidate(2, Some(11), &[2539493]);
        assert_eq!(
            match_pane_source(&[first, second], &pane(&[2539493], &["electron"])),
            SourceMatch::Ambiguous(vec![1, 2])
        );
    }

    /// ID-9 — the producer itself may be the pane's process, with no chain to
    /// walk at all.
    #[test]
    fn the_producer_may_be_the_pane_process_itself() {
        let candidates = [candidate(3, Some(2539493), &[])];
        assert_eq!(
            match_pane_source(&candidates, &pane(&[2539493], &[])),
            SourceMatch::One {
                node_id: 3,
                how: MatchHow::Ancestry
            }
        );
    }
}
