//! Waiting, in tests, on a machine that may be busy.
//!
//! A test that waits can run out for two reasons that look identical from the
//! outside: the thing it awaited will never happen, or it will happen and the
//! machine has not got round to it yet. One is a product defect. The other is
//! three builds sharing a laptop.
//!
//! For a long time the message did not distinguish them. `assert!(Instant::now()
//! < deadline, "worker completion timed out")` names neither how long it waited
//! nor what it waited against, so under load it reads exactly like a worker that
//! stopped working — and a reader who believes it goes looking for a defect that
//! is not there. That cost a day once.
//!
//! Both halves are needed. Enlarging a budget without fixing the message only
//! delays the same wrong sentence; fixing the message without enlarging the
//! budget leaves the wait failing on a machine that was merely busy.
//!
//! This module exists because a second module needed it. Putting it here before
//! that would have been designing an API with one user, which is how a shared
//! helper ends up shaped like whichever caller happened to be first.

use std::time::{Duration, Instant};

/// How much longer a wait may take on a machine that is busy.
///
/// Twelve, measured rather than chosen: #72 clocked a cold start at 17.6 seconds
/// against the second and a half the same wait takes on an idle machine. A
/// budget costs nothing until a wait fails to finish, so the number buys room on
/// a loaded machine and charges nothing for it on a quiet one.
pub(crate) const LOAD_SLACK: u64 = 12;

/// Checked where it cannot be skipped. Lowering the slack below what a cold
/// start was measured to need is a decision, and a decision is better made
/// against a build that refuses than against a test somebody can rerun.
const _: () = assert!(LOAD_SLACK >= 12);

/// A wait's budget, and what it is waiting for.
///
/// An object rather than a closure, deliberately. The waits that use this are
/// not one shape — some are `loop` with a `break`, some are `while` on a
/// condition — and folding them into one closure would mean rewriting every one
/// of those loops in a change whose entire purpose is that they keep meaning
/// what they meant. With an object, two lines move at each site and no loop
/// changes.
///
/// It also leaves the checks that belong *inside* a loop where they are. A
/// worker that disconnected is not "not ready yet", and a closure returning a
/// bare condition would have flattened that distinction into the budget.
pub(crate) struct LoadAwareDeadline {
    started: Instant,
    budget: Duration,
    what: &'static str,
}

impl LoadAwareDeadline {
    /// A budget of `budget_secs` as it would be on an idle machine, stretched to
    /// what a busy one needs.
    pub(crate) fn new(budget_secs: u64, what: &'static str) -> Self {
        Self {
            started: Instant::now(),
            budget: Duration::from_secs(budget_secs * LOAD_SLACK),
            what,
        }
    }

    /// The budget this wait is actually held to.
    #[cfg(test)]
    pub(crate) fn budget(&self) -> Duration {
        self.budget
    }

    /// Fail, saying what was awaited, for how long, and against what budget.
    ///
    /// `track_caller` is part of the behaviour rather than a nicety. Every wait
    /// in a file shares this one function, so without it they would all report
    /// this line and send a reader to the helper instead of to the wait that ran
    /// out — the same "the message points at the wrong place" fault this type
    /// exists to remove.
    #[track_caller]
    pub(crate) fn check(&self) {
        self.check_with(format_args!(""));
    }

    /// The same, for a wait whose diagnosis needs more than its name.
    ///
    /// Some waits already carried a message built at the moment they failed —
    /// the entries a directory actually held, the generation a worker was on.
    /// Flattening those into a fixed name would have thrown away the one part
    /// of the message that says which run this was, which is the opposite of
    /// what this type is for. So the caller's detail is appended rather than
    /// replaced, and the three standard facts arrive with it.
    #[track_caller]
    pub(crate) fn check_with(&self, detail: std::fmt::Arguments<'_>) {
        let elapsed = self.started.elapsed();
        assert!(
            elapsed < self.budget,
            "timed out waiting for {what} after {elapsed:?} of a {budget:?} budget{separator}{detail}",
            what = self.what,
            budget = self.budget,
            separator = if detail.as_str() == Some("") { "" } else { "; " },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TP-WAIT-01: a budget is the idle wait stretched by the slack.
    #[test]
    fn a_load_aware_budget_is_the_idle_wait_stretched_by_the_slack() {
        let wait = LoadAwareDeadline::new(5, "something");
        assert_eq!(wait.budget(), Duration::from_secs(5 * LOAD_SLACK));
    }

    // TP-WAIT-02: a wait with time left says nothing, so the happy path stays
    // free. A forgiving budget that cost anything would have made every test
    // using one slower — a regression wearing a fix's clothes.
    #[test]
    fn a_wait_with_time_left_does_not_complain() {
        let wait = LoadAwareDeadline::new(5, "something");
        for _ in 0..1000 {
            wait.check();
        }
    }

    // TP-WAIT-03: a wait that runs out says what, how long, and against what.
    // All three, because with any one missing the message still cannot tell a
    // busy machine from a broken product.
    #[test]
    fn a_wait_that_runs_out_says_what_it_awaited_and_for_how_long() {
        // Zero seconds times any slack is still zero, so this runs out on its
        // first check and the test does not spend a real budget to prove what
        // the message says.
        let wait = LoadAwareDeadline {
            started: Instant::now(),
            budget: Duration::ZERO,
            what: "a thing that never happens",
        };
        std::thread::sleep(Duration::from_millis(1));

        let panic = std::panic::catch_unwind(|| wait.check()).expect_err("an expired wait fails");
        let message = panic
            .downcast_ref::<String>()
            .map(String::as_str)
            .unwrap_or_default();

        assert!(
            message.contains("a thing that never happens"),
            "the message does not say what was awaited: {message}"
        );
        assert!(
            message.contains("budget"),
            "the message does not say what the budget was: {message}"
        );
        assert!(
            message.contains("after"),
            "the message does not say how long it waited: {message}"
        );
    }
}
