# Sparkline — a metric's recent shape, not just its current value

A `meter` answers "how full is it now". A person glancing at a bar usually has
a different question: what has it been *doing*. Did the build start. Has it
finished. Was that a spike or is it sustained. `sparkline` draws the same metric
over time — one column per reading, newest on the right, each column coloured by
its own value so the moment the load rose is readable from the shape alone.

Upstream has none of this; the whole widget is ours to lose.

Three decisions carry the behaviour, and each has a row below. Readings are kept
as ratios in a fixed ring, so a herdr left running for a week does not accumulate
a week of samples for a widget that shows sixty. A reading that could not be
taken is kept apart from a reading of zero, because "idle" and "no idea" are one
pixel apart and mean opposite things. And the arithmetic is the meter's own —
filling a cell upward is the same division as filling one sideways, so only the
glyph table differs.

The loop records the history, exactly as it records the sample. A history filled
at draw time would hold one entry per frame and none at all while nothing was
being redrawn, which would make the shape depend on how often the screen
happened to change.

Design rationale: `.local/prd/f61-sparkline.md`.

| id | behavior | why it matters | tests |
| --- | --- | --- | --- |
| TP-SPARK-01 | The history is a ring: it stops growing at its capacity, drops its oldest reading, and keeps order | Unbounded growth is a leak that goes unnoticed for weeks, and order is the entire meaning of a sparkline — a history that kept the right readings in the wrong sequence would draw a plausible shape that never happened | `the_history_drops_its_oldest_reading_rather_than_growing` |
| TP-SPARK-02 | A reading that could not be taken draws nothing; a reading of zero draws the thinnest mark there is | One pixel apart, opposite in meaning. Collapsed, a bar reports an idle machine it never measured and nothing on screen says so. Held at both layers: the history keeps `None` apart from `Some(0.0)`, and the renderer draws them differently | `an_unread_metric_is_kept_apart_from_one_that_read_zero`, `an_unread_column_is_blank_and_a_zero_column_is_the_thinnest_mark` |
| TP-SPARK-03 | With more readings than columns, the newest are the ones drawn | A sparkline answers "lately". Drawing the oldest readings would answer a question nobody asked and would look exactly as convincing | `a_history_longer_than_the_section_keeps_its_newest_readings` |
| TP-SPARK-04 | With fewer readings than columns, they sit at the right and the empty half is on the left | A herdr just opened grows its history leftward. Left alignment would shunt the newest column sideways on every reading, which reads as the whole graph sliding rather than as one sample arriving | `a_history_shorter_than_the_section_is_right_aligned` |
| TP-SPARK-05 | Every column fills upward from the bottom row of the section | Gravity, and a mistake that is invisible in the one-row bar where this would otherwise have been tested — which is why it is tested four rows tall | `a_column_fills_upward_from_the_bottom_row` |
| TP-SPARK-06 | The upward eighth glyphs climb one step at a time, and neither zero nor eight is a partial cell | A glyph table breaks silently when one line is wrong: every value still renders something and the shape is merely off by one step. Checked against the codepoints rather than against a copy of itself | `the_upward_eighths_climb_one_step_at_a_time` |
| TP-SPARK-07 | The loop records a reading in the history exactly when it takes one, and never otherwise | A widget whose data never arrives draws an empty section, which looks precisely like a section meant to be empty. The second half matters as much: a tick that read nothing must not record anything, or the history would count frames rather than readings | `sampling_the_machine_records_a_reading_in_the_history` |
