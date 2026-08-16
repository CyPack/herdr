# Test wait budgets — telling a busy machine from a broken product

A test that waits can run out for two reasons that look identical from the
outside: the thing it awaited will never happen, or it will happen and the
machine has not got round to it yet. The first is a product defect. The second
is three other builds running on the same laptop.

For a long time the message did not distinguish them. `assert!(Instant::now() <
deadline, "worker completion timed out")` names neither how long it waited nor
what it was waiting against, so a wait that ran out under load reads exactly
like a worker that stopped working — and a reader who believes it goes looking
for a defect that is not there. That confusion cost a full day once.

Two halves, and neither is enough alone. The budget is stretched by a slack
**measured** rather than chosen: `#72` clocked a cold start at 17.6 seconds
against the second and a half the same wait takes idle. And the message names
what was awaited, how long it waited, and what the budget was — because
enlarging the budget without fixing the message only delays the same wrong
sentence.

The budget costs nothing on the happy path: a wait ends when its condition
holds, and the size of the ceiling is paid only by a build that was already
failing.

Design rationale and the inventory of remaining sites:
`.local/prd/y2-test-wait-load-slack.md`.

| id | behavior | why it matters | tests |
| --- | --- | --- | --- |
| TP-WAIT-01 | A wait's budget is its idle expectation multiplied by a load slack, and the slack cannot silently drop below what a cold start was measured to need | The multiplier is the whole mechanism, so it is stated once rather than trusted at every call site. The floor is a `const` assertion rather than a test, because lowering it is a decision better made against a build that refuses than a test somebody can rerun | `a_load_aware_budget_is_the_idle_wait_stretched_by_the_slack` |
| TP-WAIT-02 | A wait with time left costs nothing | If a more forgiving budget were paid on the happy path, every test using one would have got slower and the change would be a regression wearing a fix's clothes | `a_wait_with_time_left_does_not_complain` |
| TP-WAIT-03 | A wait that runs out says what it awaited, how long it waited, and what the budget was — and reports the caller's line, not the helper's | With any one of the three missing, the message still cannot tell a busy machine from a broken product. `#[track_caller]` is part of the behavior, not decoration: every wait in a file shares one helper, so without it they would all point at the same line and send the reader to the wrong place — the exact fault this exists to remove | `a_wait_that_runs_out_says_what_it_awaited_and_for_how_long` |
