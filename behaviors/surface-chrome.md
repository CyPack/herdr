# Surface chrome — edge bars, framed panels and boxed controls

This fork lets a person decide which of herdr's surfaces wear a frame, what
colour it is, and whether the panel's own controls are drawn as boxed buttons.
Upstream has none of it: every one of these behaviors is ours to lose.

All of it is **off by default**. A frame costs two cells on each axis of a
surface that is often already the narrowest one on screen, so turning one on is
a composition choice rather than a restyle — and the default path still draws
what it drew before, which is what keeps this work inside `V1.x` of
`docs/superpowers/specs/2026-07-19-herdr-files-layout-v1-lock.md`.

Design rationale and the measurements behind each decision:
`.local/prd/custom-layout-v2.md` (§9 bars, §11 chrome).

## Edge bars — `[shell.bars.<edge>]`

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-CHROME-01 | With no `[shell.bars]` table, the derived tree is structurally identical to the one herdr has always drawn | Everyone pays for a feature they did not ask for, and every visual baseline moves in a single commit | `no_configured_bar_derives_exactly_todays_tree` |
| TP-CHROME-02 | Each of the four edges can carry a bar on its own (`top`→TopBar, `bottom`→BottomBar, `left`→AppDock, `right`→RightPanel) | "A bar on whichever side you want" is the whole request; region ids already existed and none were invented | `each_edge_can_carry_a_bar_by_itself` |
| TP-CHROME-03 | All four bars at once still leave a workspace stage | Bars could eat the surface the app exists to show | `all_four_bars_at_once_still_leave_a_stage` |
| TP-CHROME-04 | A size outside 1..32 is refused with a warning rather than clamped | Silently resizing someone's number hides their mistake behind a layout they did not write | `an_impossible_bar_size_is_refused_rather_than_repaired` |
| TP-CHROME-05 | A bordered bar thinner than three cells is refused, not drawn borderless | A bare band where a frame was asked for reads as the border failing, not as the size being impossible | `a_bordered_bar_thinner_than_its_border_is_refused` |
| TP-CHROME-06 | Every enabled-edge composition carries its own revision, and the exact composition is in the geometry key | Two different screens answering to one identity is CLA3: the cache returns the old geometry and hit testing goes to the wrong region | `every_edge_composition_has_its_own_revision` |
| TP-CHROME-07 | A composition that fails validation falls back to the legacy tree instead of propagating an error | A shell that cannot be composed must still show a shell; config is untrusted input | `a_bar_composition_that_does_not_validate_falls_back_to_the_legacy_tree` |

## Bar sections — `[[shell.bars.<edge>.sections]]`

A bar can be divided into at most eight parts along its long axis. A section is
an address: what fills it and what a click on it does are answered by later
layers, and the number it was written at is the only stable name it has.

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-CHROME-23 | More sections than a bar may hold leaves it undivided with a warning, rather than keeping the first eight | Truncation hands someone a layout they did not write and renumbers every section after the one that vanished | `a_bar_refuses_a_ninth_section_rather_than_dropping_it` |
| TP-CHROME-24 | Exactly the maximum number of sections is accepted | A ceiling that cannot be reached is an off-by-one that silently destroys a section | `a_bar_accepts_exactly_its_maximum_number_of_sections` |
| TP-CHROME-25 | Section widths come from the shell solver's own three phases, including its largest-remainder tie-break | A second distribution implementation drifts by one cell the first time two remainders tie, and both answers stay internally consistent | `section_widths_come_from_the_shell_solvers_own_arithmetic`, `an_odd_remainder_goes_to_the_earlier_section_every_time` |
| TP-CHROME-26 | A top or bottom bar is divided across its width; a left or right bar down its height | The wrong axis collapses every section onto the same cells and still looks plausible | `a_side_bar_divides_its_height_and_an_edge_bar_divides_its_width` |
| TP-CHROME-27 | Sections account for the bar's content area exactly, and stay inside its border | A short division leaves an undrawn strip; a long one writes into the region next door; either escapes the frame it was promised | `sections_fill_the_bar_exactly_without_overrunning_it`, `sections_of_a_bordered_bar_stay_inside_its_border` |
| TP-CHROME-28 | A section squeezed to no cells is not a target | A zero-width rectangle that still answers clicks answers for its neighbour | `a_section_squeezed_to_nothing_is_not_reported_as_occupied` |
| TP-CHROME-29 | A section is clicked exactly where it is drawn, framed or not, and only against the generation that drew it | This is the fourth place the same defect could appear: drawing reads the live value, hit testing is handed a constant, and both rectangles stay plausible | `every_bar_section_is_clicked_exactly_where_it_is_drawn`, `a_bar_section_answers_only_for_the_generation_that_drew_it` |
| TP-CHROME-30 | A different division is a different geometry identity, and re-dividing advances the generation exactly once | Same key means the cache returns the previous rectangles and every click in that bar answers for the wrong section | `changing_only_a_sections_policy_changes_the_bars_identity`, `redividing_a_bar_misses_the_cache_and_advances_the_generation_once` |
| TP-CHROME-31 | One unreadable section leaves the whole bar undivided rather than dropping just that entry | Dropping one entry shifts every later index, silently moving whatever those indices address | `an_unreadable_section_leaves_the_bar_undivided_rather_than_renumbered`, `a_section_with_impossible_numbers_is_refused` |
| TP-CHROME-32 | A sections array survives the live config reader, not just the model | The layer that added `[shell.bars]` tested the model and never a file; a dropped array reloads as an undivided bar while the file plainly says otherwise | `a_bars_sections_array_survives_the_live_reader` |
| TP-CHROME-33 | An undivided bar projects no section targets at all, and is still a target itself | Everyone who never divides a bar must pay nothing for the feature | `an_undivided_bar_contributes_no_section_hits`, `a_bar_without_sections_divides_into_nothing` |
| TP-CHROME-34 | A component placement naming a region the tree does not contain is refused | Such a placement draws nothing and reports nothing; the gate is the only place that can notice | `shell_rejects_a_placement_whose_region_is_not_in_the_tree` |

## Frame tone — one vocabulary for every framed surface

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-CHROME-08 | A colour is read as a palette token first and a literal second, so a themed name follows the theme | `accent` freezing to one RGB is the difference between a frame that belongs to the theme and one painted over it | `a_bar_colour_reads_palette_tokens_then_literals` |
| TP-CHROME-09 | A gradient whose ends carry no channel values falls back to a solid tone and says so | A named terminal colour has no RGB to interpolate; fading silently to nothing looks like the gradient being ignored | `a_gradient_that_names_a_channelless_colour_falls_back_to_solid` |
| TP-CHROME-10 | A two-stop gradient actually changes tone across the surface's long axis | The feature reduces to an expensive solid colour without anything noticing | `a_two_stop_gradient_changes_colour_across_the_span` |
| TP-CHROME-11 | Framed shells wear rounded corners | The rounded corner is the request; no thick-and-rounded glyph exists, so weight comes from bold instead | `floating_shells_wear_rounded_corners` |

## The left panel's two halves

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-CHROME-12 | Framing one half leaves the other's rectangle untouched | The halves are independent surfaces; one decision must not move the other | `framing_one_section_leaves_the_other_untouched` |
| TP-CHROME-13 | A half too small for a frame keeps its whole rectangle for content | Losing the border is cosmetic; losing the panel is not | `a_section_too_small_for_a_frame_keeps_its_whole_rectangle` |
| TP-CHROME-14 | Each half is hit-tested through the same inset it was drawn through | Drawing and hit testing that disagree put every row one cell off the row the person can see — and nothing else in the suite objects, because both rectangles stay inside the sidebar and stay non-empty | `a_framed_half_is_clicked_through_the_inset_it_was_drawn_through` |
| TP-CHROME-15 | The collapse icon steps inside a framed half instead of overwriting its corner | An icon on the corner glyph reads as a broken border rather than as a button inside a panel | `the_collapse_icon_keeps_off_a_framed_panels_corner` |
| TP-CHROME-16 | The collapse click follows the icon inwards, and the frame's corner stops collapsing anything | The icon would still be painted and the stale cell would still be inside the sidebar, so the drift would ship unnoticed | `a_framed_sidebar_takes_the_collapse_click_where_the_icon_was_drawn` |

## Boxed controls — `[ui.sidebar.chips]`

| ID | Behavior | What breaks if it is lost | Verified by |
|---|---|---|---|
| TP-CHROME-17 | Asking for chips grows the footer to a frame's height and the list gives up exactly those rows, with the controls staying inside it and disjoint | A footer height counted differently in two places puts the list and the buttons on top of each other while both remain non-empty and in bounds | `chips_grow_the_footer_and_the_list_gives_up_exactly_those_rows` |
| TP-CHROME-18 | The footer's controls are drawn as frames whose labels survive inside them | Corner assertions alone are satisfied by a frame around nothing; the label is what makes it a button | `footer_controls_wear_their_own_frames_when_chips_are_on` |
| TP-CHROME-19 | Chips that would overlap are both dropped, and the footer keeps its plain labels | Interleaved frames read as neither control; half a border is worse than none and a vanished button is worse than both | `a_footer_too_narrow_for_chips_still_draws_its_labels` |
| TP-CHROME-20 | The agents header's name and sort control become chips in its top corners, and cost the agent list no rows | The header already reserved a frame's worth of rows; spending more would take them from the list for nothing | `the_agents_header_wears_its_labels_as_corner_chips` |
| TP-CHROME-21 | The sort control answers at the rectangle the header drew it in, framed or not | This was the third independent copy of a control whose hit test rebuilt the panel geometry from a constant chrome | `the_agents_sort_control_is_clicked_where_the_header_drew_it` |
| TP-CHROME-22 | A chip keeps its label inside its own frame, clips rather than wraps when narrow, and refuses a space too short for a frame | A chip that wraps breaks the frame it just promised; one that draws anyway ships half a border | `a_chip_keeps_its_label_inside_its_own_frame`, `a_narrow_chip_clips_its_label_rather_than_its_frame`, `a_chip_refuses_a_space_too_short_for_its_frame` |

## The rule this family keeps learning

A shared geometry function is not shared if one caller hands it a constant.
Three separate controls in this fork recomputed the panel's rectangles from
`SidebarChrome::NONE` while the renderer used the live value, and none of them
produced a red test. When a context argument is added to a geometry helper, its
call sites get grepped for constants before the change is called done.

Source markers: `src/ui/shell/source.rs` (`ShellBars`, `BarTint`,
`SidebarChrome`), `src/ui/widgets.rs` (`render_bar_shell`, `render_chip`),
`src/ui/sidebar.rs` (`expanded_sidebar_sections`, `render_sidebar_footer_buttons`,
`render_agent_detail`), `src/app/input/sidebar.rs` (the hit tests).
