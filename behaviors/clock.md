# Clock — the one widget that changes without anybody touching it

Every other bar widget draws something a person put there or a reading somebody
took. A clock moves on its own, and that makes it the first place in this
product where "how often does this cost anything while nobody is looking" is a
question the widget itself has to answer.

Upstream has none of this; the whole widget is ours to lose.

The answer is that **the format decides the pace**. A format naming `%S` has to
be redrawn every second; one without it changes sixty times less often, and
waking every second to render the same string would push those cells through the
frame diff for nothing. Deriving the pace from the format also removes a whole
class of disagreement: an `interval` key beside the format could say something
the format contradicts, and nothing would decide which one wins.

The reading is taken by the loop and drawn by the renderer, exactly as a
resource sample is. A clock calling `now()` inside `render` would look perfectly
correct on screen and would repaint its region on every frame — the failure
`resource` was shaped to avoid, and the one the parity guard below caught this
widget committing in a different form.

An unreadable local zone draws nothing rather than falling back to UTC. A clock
quietly showing another country's time is worse than an empty section, because
nothing about it looks wrong.

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-CLOCK-01 | Every clock field renders the example published beside it | The table carries an example so `herdr shell spec` and the guide can show one, and an example nobody checks is a sentence that ages the first time a field's padding changes. Reading it back through the real renderer makes the published table a claim this build has to keep | `every_clock_field_renders_the_example_it_publishes` |
| TP-CLOCK-02 | A twelve-hour clock shows `12` at midnight and at noon, never `00` | `hour % 12` alone is the classic way to put `00` on a twelve-hour clock, which no twelve-hour clock in the world shows. Both boundaries are checked because they fail separately: noon is the one an off-by-one at 12 gets wrong | `a_twelve_hour_clock_shows_twelve_at_midnight_and_noon` |
| TP-CLOCK-03 | Text between fields is carried through verbatim, and `%%` is one percent | A format is mostly punctuation the person chose. Dropping it leaves a clock that is technically correct and unreadable, and a `%%` that vanished would make a literal percent impossible to write | `literal_text_between_fields_is_carried_through` |
| TP-CLOCK-04 | A field this build cannot write is refused by name, case included | `%Q` drawn as itself is indistinguishable from a field that exists and rendered wrongly. Case matters for the same reason every other name in this grammar is matched exactly: a near-miss is a typo, not another spelling | `a_field_this_build_does_not_know_is_refused_by_name` |
| TP-CLOCK-05 | Only a format showing seconds refreshes every second | The whole cost argument. The control half — every other field together still leaves the clock on the slow tick — is what makes this a cost gate rather than a spelling check, and the default is checked too so the cheapest clock is the one nobody had to ask for | `only_a_format_showing_seconds_refreshes_every_second` |
| TP-CLOCK-06 | A clock without seconds renders one string for the whole minute | What makes the slow tick safe. A clock woken at :00 and again at :59 of one minute has nothing new to draw, so the frame diff sends nothing; without this a once-a-minute wakeup could still repaint. The control row — a format that does show seconds renders differently — stops the assertion holding for a renderer that ignores time altogether | `a_clock_without_seconds_renders_the_same_text_all_minute` |
| TP-CLOCK-07 | No clock section means no tick at all, and a clock never starts the sampler | A clock is the first thing here that changes untouched, so a tick scheduled unconditionally would wake every herdr on every machine once a minute for a widget almost nobody turned on. The two live clocks stay separate: a meter must not start waking on the minute, and a clock must not start opening `/proc` | `only_a_bar_with_a_clock_asks_the_loop_to_wake_for_one` |
| TP-CLOCK-08 | A clock wakes on the boundary of the unit it shows | A `%H:%M` clock woken every sixty seconds from whenever it started would show a minute that turned over up to fifty-nine seconds ago. The alignment costs one modulo and is the difference between a clock and a stopwatch | `only_a_bar_with_a_clock_asks_the_loop_to_wake_for_one` |
| TP-CLOCK-09 | The loop reads the clock, the renderer never does — and only a changed reading asks for a draw | A clock reading `now()` in `render` draws correctly and repaints every frame. The second half keeps the wakeup cheap: a tick reporting a change every time would repaint the clock's cells on every wakeup, which is exactly what the resolution rule exists to prevent | `ticking_the_clock_fills_state_and_reports_only_real_changes` |
| TP-CLOCK-10 | A bar that loses its clock forgets the time it was holding | A stale time is worse than no time. A config reload that removes the clock and later restores it would otherwise paint whatever moment the previous bar was showing, for as long as it took the next tick to arrive | `a_bar_that_loses_its_clock_forgets_the_time_it_was_holding` |
| TP-CLOCK-11 | The guide's clock table is exactly the fields this build writes, renderings included | A name alone is not a useful row: somebody chooses `%I` over `%H` by reading what it produces, and a wrong rendering sends them to a bar that says something other than what they read. The renderings are tied to the real renderer by TP-CLOCK-01, so a wrong number in the guide cannot survive by matching a wrong number in the code | `the_guide_lists_exactly_the_clock_fields_this_build_can_write` |
| TP-CLOCK-12 | The clock ticks in the loop that actually renders | Added to the monolithic loop alone, every clock test stayed green and the parity guard named `tick_clock` as a call the headless scheduler was missing — the server owns the state the screen is drawn from, so a clock ticking only in the other loop is a clock that never moves. The same class TP-RES-11 records, caught by the same guard | `scheduler_parity_headless_vs_monolithic` |
| TP-CLOCK-13 | The face comparison runs at the resolution the fastest visible clock can show | `tick_clock` compared (hour, minute, second) unconditionally, so a `%H:%M` bar reported a change every second — one whole-surface frame per second for a string that moves once a minute, which is the exact cost `resolution`'s own comment documents avoiding. Measured live as the surviving half of the once-a-second full-render pulse after TP-RES-27 silenced the sampler's half: the fix landed, the pulse stayed at 1.13 state changes a second, and the clock was the line directly below the one that had just been fixed | `a_minute_face_ignores_a_new_second`, `a_minute_face_changes_when_the_minute_does`, `a_face_appearing_or_vanishing_is_a_change` |
