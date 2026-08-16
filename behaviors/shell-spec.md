# Shell spec — the bar grammar, as something a program can read

`herdr shell spec [--json]` prints what a bar config may say: the kinds, the
keys each one reads, the keys each one refuses, and a working example of every
one. Upstream has no such command; this whole surface is ours to lose.

The point is the audience. A person learning the bar grammar can read the
configuration guide, provoke a refusal, or experiment. An agent has the same
three routes and all of them are prose, so writing a bar config has meant
rediscovering by trial what the parser already knows exactly. This command is
the fourth route, and the only one a program can consume without guessing.

Nothing in it is written down twice. Every accepted name comes from the tables
in `src/ui/shell/source.rs` that also carry the code building it, so a kind that
exists is a kind the spec lists — the two halves cannot be separated. What is
hand-written is the part no table can derive: which keys a kind reads, which it
refuses, and an example. Those three are exactly what the rows below check, and
they check them by **running them through the real parser** rather than reading
them back. A gate that compares a generated list against the table it was
generated from measures nothing; that trap is why these rows are shaped this
way.

The command touches no socket and needs no server. The question "what may I
write?" is asked before anything is running, and an answer that required a live
session would be missing exactly when it is wanted.

Design rationale and the measurements behind each decision:
`.local/prd/f70-l2-l4-shell-spec.md`.

| id | behavior | why it matters | tests |
| --- | --- | --- | --- |
| TP-SPEC-01 | Every example the spec publishes is a config this build accepts, and each one actually shows the kind it is filed under | An example is the first thing anybody copies, an agent most of all, and one that quietly produces an empty section teaches the wrong thing while reading as correct. Nothing is assembled in the test: the bytes parsed are the bytes a reader receives, so the check cannot pass over a reconstruction that differs from what was published. The second half catches the likeliest mistake when a kind is added — pasting the entry above it and forgetting to change the name | `every_example_the_spec_publishes_is_a_config_this_build_accepts`, `every_example_shows_the_kind_it_is_filed_under` |
| TP-SPEC-02 | Every key the spec says an action kind refuses is one this build really refuses | The direction nothing else covers. A refusal is the half of a grammar a reader cannot discover except by being turned down, so a spec that promises one and does not deliver sends somebody hunting for a bug in their own file. Handing `argv` to a `plugin` action is the mistake this exists for: it is the first thing anybody tries | `every_refusal_the_spec_promises_is_one_this_build_makes` |
| TP-SPEC-03 | The spec names exactly what the refusal messages offer, across all four kind surfaces and the art catalogue | Two surfaces face outward — the JSON and the message text — and a reader who finds one name in one and a different set in the other has no way to know which one this build meant | `the_spec_names_exactly_what_the_refusals_offer` |
| TP-SPEC-04 | A metric alias is offered as an alias and never as a name | `ram` works and always has, and the refusal deliberately does not teach it. Listing it among the names would make the spec contradict the message; leaving it out entirely would tell somebody that a word already in their file is not accepted. The sibling of TP-CHROME-102, on the spec side | `the_metric_aliases_are_offered_as_aliases_rather_than_as_names` |
| TP-SPEC-05 | Every alias parses to the metric the spec claims it means, and every name it lists parses at all | The alias list is an assertion about the parser, so it is put to the parser rather than read back | `every_alias_parses_to_the_metric_it_claims_to_mean` |
| TP-SPEC-06 | Every `[shell.bars.<edge>]` key the spec lists is one a config may set | There is no table behind those keys — they are struct fields — so this stands in for one: a document setting all of them at once has to be a document this build accepts. It is what catches a renamed or removed key, which nothing else here would see | `every_bar_key_is_one_this_build_accepts` |
| TP-SPEC-07 | Every list the spec publishes still has entries in it | Every other row here loops over something the spec published, so a change that emptied a list would leave them all passing over nothing at all. That is the one failure a gate is least likely to notice about itself | `the_spec_still_carries_every_surface_it_is_meant_to` |
| TP-SPEC-08 | The spec serialises to JSON a consumer can read, carries its version, and keeps `refuses` present even when empty | A consumer reads these fields by name, so a rename is a silent break on the far side of a boundary this repository cannot see. An empty list that disappears makes a consumer branch on shape rather than on content, which is the same break wearing a different hat | `the_spec_serialises_to_json_a_consumer_can_read` |
| TP-SPEC-09 | The text rendering names everything the JSON names, including the alias explanation | Two renderings of one grammar that disagree would make the human and the machine answer differently, and the human's copy is the one nothing else checks | `the_text_rendering_carries_every_name_the_json_carries` |
| TP-SPEC-10 | Every bundled picture in the catalogue is one that can actually be drawn | `builtin_catalogue` leaves out a picture it cannot read rather than panicking in front of somebody running the CLI. That is the right behaviour there and the wrong silence to keep in a test, so this is where an unreadable picture is noticed | `every_bundled_picture_is_one_that_can_be_drawn` |
| TP-SPEC-11 | `shell spec` prints text unless `--json` is asked for, and refuses an option it does not have by name | Text is the default because a person typing the command is reading it. Silently ignoring an unknown option would print the default and let somebody believe they had asked for something else — the failure that costs most to notice, because the output looks fine | `the_spec_prints_text_until_json_is_asked_for`, `an_option_the_spec_does_not_have_is_refused_rather_than_ignored` |
