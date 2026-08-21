//! The bar grammar, as something a program can read.
//!
//! A person learning what a bar section may say has three ways to find out: the
//! configuration guide, a refusal message, or trying it. An agent has the same
//! three and all of them are prose. This module is the fourth: the same grammar,
//! emitted as data, so that writing a bar config stops being an exercise in
//! rediscovering what the parser already knows.
//!
//! Nothing here is written twice. Every name comes from the table that also
//! carries the code building it, so a kind that exists is a kind this spec
//! lists, and a kind this spec lists is one the parser accepts — neither half
//! can be added without the other. What is written by hand is the part a table
//! cannot derive: which keys a kind reads, which it refuses, and an example.
//! Those three are exactly what the gate at the bottom of this file checks, by
//! running them through the real parser rather than reading them back.
//!
//! No session, no server, no socket. A question about grammar should not need a
//! running herdr to answer, least of all from something deciding what to write
//! before it starts anything at all.

use serde::Serialize;

use super::source;
use crate::icon;
use crate::resource::ResourceMetric;

/// The shape of what this module emits.
///
/// Version one, and versioned from the first day it existed. A consumer that
/// cannot tell an old shape from a new one has to guess, and adding a version
/// after somebody is already parsing the thing is always too late.
pub(crate) const SPEC_VERSION: u32 = 1;

/// The whole bar grammar.
#[derive(Debug, Serialize)]
pub(crate) struct ShellSpec {
    pub version: u32,
    /// The edges a bar can sit on.
    pub edges: Vec<&'static str>,
    /// The keys `[shell.bars.<edge>]` itself takes.
    pub bar_keys: Vec<KeySpec>,
    /// The keys any section takes, whatever its kind.
    ///
    /// Apart from the kind tables because they belong to none of them: every
    /// sizing kind reads these, so filing them under one would teach a reader
    /// that the others turn them down.
    pub section_keys: Vec<KeySpec>,
    /// How a section asks for space.
    pub section_kinds: Vec<KindSpec>,
    /// What a section shows.
    pub widget_kinds: Vec<KindSpec>,
    /// What a press on a section does.
    pub action_kinds: Vec<KindSpec>,
    /// What a second gesture can ask for.
    pub secondary_presentations: Vec<&'static str>,
    /// The fields a `clock` widget's `format` may name, each with what it
    /// renders as.
    ///
    /// Published for the reason the colours are: an example in the guide shows
    /// one spelling, and nothing else can teach the other ten. The examples are
    /// all of one moment, so the list reads as a single clock taken apart.
    pub clock_fields: Vec<ClockFieldSpec>,
    /// The rows the menu offers when a second gesture asks rather than acts.
    ///
    /// Published for the same reason the colours are: nothing else can teach
    /// them. A menu only exists once somebody has already pressed, so a reader
    /// deciding whether `secondary = "menu"` is worth writing would otherwise
    /// have to run the thing to find out what it offers.
    pub bar_section_menu: Vec<&'static str>,
    /// The colour names any `color`, `gradient` stop or picture palette may
    /// write. Published because nothing else can teach them: an unrecognised
    /// colour is not refused, so no refusal ever names the set, and a reader
    /// who has only seen `"mauve"` in an example has no way to learn `teal`.
    pub colors: Vec<&'static str>,
    pub metrics: MetricSpec,
    pub icon_art: Vec<ArtSpec>,
    /// Switches outside the bars that change what a bar may do.
    pub switches: Vec<KeySpec>,
}

/// One accepted name, everything it reads, and a config that uses it.
#[derive(Debug, Serialize)]
pub(crate) struct KindSpec {
    pub kind: &'static str,
    pub keys: Vec<&'static str>,
    /// Keys this kind turns down. Always present, empty where there are none:
    /// a field that appears and disappears makes a consumer branch on shape
    /// rather than on content.
    pub refuses: Vec<&'static str>,
    /// A whole config this build accepts. Copy it and it works.
    pub example: &'static str,
}

/// One key, its type, and what it does when nobody sets it.
#[derive(Debug, Serialize)]
pub(crate) struct KeySpec {
    pub key: &'static str,
    #[serde(rename = "type")]
    pub value_type: &'static str,
    /// Bounds, where there are any. Empty rather than absent, for the same
    /// reason `refuses` is.
    pub range: &'static str,
    pub default: &'static str,
}

/// The metric names, and the ones that work without being taught.
///
/// Two lists rather than one. Folding them together would present an alias as a
/// spelling to learn; leaving the aliases out entirely would tell a reader that
/// a word their file already contains is not accepted. Both would be wrong in a
/// way nothing else would catch.
#[derive(Debug, Serialize)]
pub(crate) struct MetricSpec {
    pub names: Vec<&'static str>,
    /// Alias first, then the name it is another word for.
    pub aliases: Vec<AliasSpec>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AliasSpec {
    pub alias: &'static str,
    pub means: &'static str,
}

/// One clock format field, and what it renders as.
#[derive(Debug, Serialize)]
pub(crate) struct ClockFieldSpec {
    /// As a format writes it, `%` included.
    pub field: String,
    /// What that field produces, for one fixed moment shared by every row.
    pub renders: &'static str,
}

/// One bundled picture and the room it needs.
#[derive(Debug, Serialize)]
pub(crate) struct ArtSpec {
    pub name: &'static str,
    /// Cells across.
    pub cells: u16,
    /// Rows down. Half the pixel rows, because a picture is drawn two pixels to
    /// the cell.
    pub rows: u16,
}

/// The keys `[shell.bars.<edge>]` takes.
///
/// Hand-written, because there is no table to read them from: they are struct
/// fields, and a struct cannot say what its bounds mean or what a missing value
/// stands for. `every_bar_key_is_one_this_build_accepts` is what keeps the list
/// honest.
const BAR_KEYS: &[KeySpec] = &[
    KeySpec {
        key: "enabled",
        value_type: "boolean",
        range: "",
        default: "false",
    },
    KeySpec {
        key: "size",
        value_type: "integer",
        range: "1-32",
        default: "3",
    },
    KeySpec {
        key: "style",
        value_type: "string",
        range: "framed, islands, plain, pills",
        default: "framed",
    },
    KeySpec {
        key: "border",
        value_type: "boolean",
        range: "",
        default: "the style's — framed: true",
    },
    KeySpec {
        key: "color",
        value_type: "string",
        range: "",
        default: "",
    },
    KeySpec {
        key: "background",
        value_type: "string",
        range: "",
        default: "the theme's general background",
    },
    KeySpec {
        key: "gradient",
        value_type: "array of string",
        range: "2 or more stops",
        default: "",
    },
    KeySpec {
        key: "max_sections",
        value_type: "integer",
        range: "1-16",
        default: "8",
    },
    KeySpec {
        key: "hide_when_focused",
        value_type: "boolean",
        range: "",
        default: "false",
    },
];

/// The keys `[[shell.bars.<edge>.sections]]` takes whatever its `kind` is.
///
/// Hand-written for the reason [`BAR_KEYS`] is — they are struct fields, and a
/// struct cannot say what a missing value stands for — and kept out of the kind
/// tables because they are not any one kind's.
const SECTION_KEYS: &[KeySpec] = &[
    KeySpec {
        key: "group",
        value_type: "string",
        range: "",
        default: "",
    },
    KeySpec {
        key: "border",
        value_type: "boolean",
        range: "",
        default: "false",
    },
    KeySpec {
        key: "background",
        value_type: "string",
        range: "",
        default: "the style's — pills: a dusty tone of the run's colour",
    },
    KeySpec {
        key: "color",
        value_type: "string",
        range: "",
        default: "the bar's own colour",
    },
];

/// Switches that live outside `[shell.bars]` and change what a bar may do.
const SWITCHES: &[KeySpec] = &[
    KeySpec {
        key: "shell.glyph_icons",
        value_type: "boolean",
        range: "",
        default: "true",
    },
    KeySpec {
        key: "shell.resource_interval_ms",
        value_type: "integer",
        range: "250-60000",
        default: "2000",
    },
];

/// The bar grammar this build actually implements.
pub(crate) fn shell_spec() -> ShellSpec {
    ShellSpec {
        version: SPEC_VERSION,
        edges: vec!["top", "bottom", "left", "right"],
        bar_keys: BAR_KEYS.iter().map(KeySpec::copied).collect(),
        section_keys: SECTION_KEYS.iter().map(KeySpec::copied).collect(),
        section_kinds: kinds(source::sizing_kind_facts()),
        widget_kinds: kinds(source::widget_kind_facts()),
        action_kinds: kinds(source::action_kind_facts()),
        secondary_presentations: source::secondary_presentation_names(),
        clock_fields: crate::clock::CLOCK_FIELDS
            .iter()
            .map(|field| ClockFieldSpec {
                field: format!("%{}", field.spec),
                renders: field.example,
            })
            .collect(),
        bar_section_menu: crate::app::state::BarSectionMenuItem::ALL
            .iter()
            .map(|item| item.label())
            .collect(),
        colors: source::bar_color_tokens(),
        metrics: MetricSpec {
            names: ResourceMetric::accepted(),
            aliases: ResourceMetric::ALIASES
                .iter()
                .map(|(alias, means)| AliasSpec { alias, means })
                .collect(),
        },
        icon_art: icon::builtin_catalogue()
            .into_iter()
            .map(|(name, cells, rows)| ArtSpec { name, cells, rows })
            .collect(),
        switches: SWITCHES.iter().map(KeySpec::copied).collect(),
    }
}

impl KeySpec {
    fn copied(&self) -> Self {
        Self {
            key: self.key,
            value_type: self.value_type,
            range: self.range,
            default: self.default,
        }
    }
}

fn kinds(facts: Vec<source::KindFacts>) -> Vec<KindSpec> {
    facts
        .into_iter()
        .map(|fact| KindSpec {
            kind: fact.name,
            keys: fact.keys.to_vec(),
            refuses: fact.refuses.to_vec(),
            example: fact.example,
        })
        .collect()
}

/// The spec as a person reads it.
///
/// The same facts as the JSON and in the same order, because two orderings of
/// one grammar is one more than anybody needs to hold in their head.
pub(crate) fn render_text(spec: &ShellSpec) -> String {
    let mut out = String::new();
    out.push_str(&format!("herdr shell spec v{}\n", spec.version));
    out.push_str(&format!("\nedges: {}\n", spec.edges.join(", ")));

    out.push_str("\n[shell.bars.<edge>]\n");
    for key in &spec.bar_keys {
        out.push_str(&key_line(key));
    }

    out.push_str("\n[[shell.bars.<edge>.sections]]\n");
    for key in &spec.section_keys {
        out.push_str(&key_line(key));
    }

    for (title, table) in [
        ("section kinds (kind = …)", &spec.section_kinds),
        ("widget kinds (widget.kind = …)", &spec.widget_kinds),
        ("action kinds (action.kind = …)", &spec.action_kinds),
    ] {
        out.push_str(&format!("\n{title}\n"));
        for entry in table {
            out.push_str(&format!("  {}\n", entry.kind));
            out.push_str(&format!("    reads:   {}\n", list(&entry.keys)));
            if !entry.refuses.is_empty() {
                out.push_str(&format!("    refuses: {}\n", list(&entry.refuses)));
            }
        }
    }

    out.push_str(&format!(
        "\nsecondary presentations (action.secondary = …): {}\n",
        list(&spec.secondary_presentations)
    ));
    out.push_str(&format!(
        "  the menu offers: {}\n",
        list(&spec.bar_section_menu)
    ));

    out.push_str("\nclock fields (widget.format = …), all shown at one moment:\n");
    for field in &spec.clock_fields {
        out.push_str(&format!("  {} → {}\n", field.field, field.renders));
    }
    out.push_str("  %% is a literal percent; anything else is carried through\n");

    out.push_str(&format!(
        "\ncolours (color, gradient stops, picture palettes): {}\n",
        list(&spec.colors)
    ));
    out.push_str("  or a literal: #cba6f7, #fa8, rgb(203, 166, 247)\n");

    out.push_str(&format!(
        "\nmetrics (widget.metric = …): {}\n",
        list(&spec.metrics.names)
    ));
    for alias in &spec.metrics.aliases {
        out.push_str(&format!(
            "  {} also works, and means {}\n",
            alias.alias, alias.means
        ));
    }

    out.push_str("\nbundled art (widget.art = …)\n");
    for art in &spec.icon_art {
        out.push_str(&format!(
            "  {name}: {cells} cell{cell_s} by {rows} row{row_s}\n",
            name = art.name,
            cells = art.cells,
            cell_s = if art.cells == 1 { "" } else { "s" },
            rows = art.rows,
            row_s = if art.rows == 1 { "" } else { "s" }
        ));
    }

    out.push_str("\nswitches\n");
    for switch in &spec.switches {
        out.push_str(&key_line(switch));
    }

    out.push_str("\nexamples\n");
    for entry in spec
        .section_kinds
        .iter()
        .chain(&spec.widget_kinds)
        .chain(&spec.action_kinds)
    {
        out.push_str(&format!("\n  # {}\n", entry.kind));
        for line in entry.example.lines() {
            out.push_str(&format!("  {line}\n"));
        }
    }

    out
}

fn key_line(key: &KeySpec) -> String {
    let range = if key.range.is_empty() {
        String::new()
    } else {
        format!(", {}", key.range)
    };
    let default = if key.default.is_empty() {
        String::new()
    } else {
        format!(" (default {})", key.default)
    };
    format!(
        "  {key}: {value_type}{range}{default}\n",
        key = key.key,
        value_type = key.value_type,
    )
}

fn list(names: &[&str]) -> String {
    if names.is_empty() {
        "nothing".to_string()
    } else {
        names.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whatever this build says is wrong with a config, as plain sentences.
    fn problems(toml_text: &str) -> Vec<String> {
        let config: crate::config::Config =
            toml::from_str(toml_text).unwrap_or_else(|err| panic!("{err}\n{toml_text}"));
        super::super::source::shell_bar_config_problems(
            &config.shell.bars,
            config.shell.glyph_icons,
        )
        .into_iter()
        .map(|problem| problem.to_string())
        .collect()
    }

    fn every_kind(spec: &ShellSpec) -> Vec<&KindSpec> {
        spec.section_kinds
            .iter()
            .chain(&spec.widget_kinds)
            .chain(&spec.action_kinds)
            .collect()
    }

    /// TP-SPEC-07: the lists are long enough to be lists.
    ///
    /// Every other test here loops over something this spec published, so a
    /// change that emptied a list would leave them all passing over nothing at
    /// all. That is the one failure a gate is least likely to notice about
    /// itself, and it is why this test is first.
    #[test]
    fn the_spec_still_carries_every_surface_it_is_meant_to() {
        let spec = shell_spec();

        assert_eq!(spec.version, SPEC_VERSION);
        assert_eq!(spec.edges.len(), 4, "an edge went missing");
        assert!(spec.bar_keys.len() >= 6, "the bar keys thinned out");
        assert!(spec.section_kinds.len() >= 3, "a sizing kind went missing");
        assert!(spec.widget_kinds.len() >= 4, "a widget kind went missing");
        assert!(spec.action_kinds.len() >= 2, "an action kind went missing");
        assert!(
            !spec.secondary_presentations.is_empty(),
            "the secondary presentations emptied out"
        );
        // TP-SPEC-15: the menu the spec publishes is the menu the product
        // draws. Four, not "some": the rows are a closed set — three
        // presentations and, since TP-CHROME-150, the bar's configure door —
        // and a spec that published fewer would teach that a section reaches
        // fewer places.
        assert_eq!(
            spec.bar_section_menu.len(),
            4,
            "the published menu no longer has a row per presentation"
        );
        assert!(spec.colors.len() >= 8, "the colour names thinned out");
        assert!(spec.metrics.names.len() >= 3, "a metric went missing");
        // Two, not "some". Deleting a bundled picture was measured to break
        // nothing at all: every other check walks the catalogue, so a name
        // removed from it is simply a name nothing looks for any more. A
        // catalogue may grow freely; shrinking is a config somebody already
        // wrote quietly ceasing to work, and it should arrive as a decision.
        assert!(
            spec.icon_art.len() >= 2,
            "the art catalogue shrank; a picture that ships is a config somebody may \
             already have written: {:?}",
            spec.icon_art
        );
        assert!(!spec.switches.is_empty(), "the switches emptied out");
    }

    /// TP-SPEC-01: every example this spec publishes is a config this build takes.
    ///
    /// The example is the first thing anybody copies, an agent most of all, and
    /// one that quietly produces an empty section teaches the wrong thing while
    /// reading as correct. Nothing is assembled here: the bytes parsed are the
    /// bytes a reader receives, so the check cannot pass over a reconstruction
    /// that differs from what was published.
    #[test]
    fn every_example_the_spec_publishes_is_a_config_this_build_accepts() {
        let spec = shell_spec();
        let kinds = every_kind(&spec);
        assert!(kinds.len() >= 9, "the examples thinned out");

        for entry in kinds {
            let found = problems(entry.example);
            assert!(
                found.is_empty(),
                "the example for {:?} is refused by this build: {found:?}\n{}",
                entry.kind,
                entry.example
            );
        }
    }

    /// TP-SPEC-01: and every example actually shows the kind it is filed under.
    ///
    /// Without this an example could be copied from a neighbour, parse cleanly,
    /// and teach the wrong thing — the most likely mistake when a kind is added
    /// by pasting the entry above it.
    #[test]
    fn every_example_shows_the_kind_it_is_filed_under() {
        let spec = shell_spec();

        for entry in &spec.section_kinds {
            assert!(
                entry
                    .example
                    .contains(&format!("kind = \"{}\"", entry.kind)),
                "the example for sizing kind {:?} never uses it\n{}",
                entry.kind,
                entry.example
            );
        }
        for entry in &spec.widget_kinds {
            assert!(
                entry
                    .example
                    .contains(&format!("widget = {{ kind = \"{}\"", entry.kind)),
                "the example for widget kind {:?} never uses it\n{}",
                entry.kind,
                entry.example
            );
        }
        for entry in &spec.action_kinds {
            assert!(
                entry
                    .example
                    .contains(&format!("action = {{ kind = \"{}\"", entry.kind)),
                "the example for action kind {:?} never uses it\n{}",
                entry.kind,
                entry.example
            );
        }
    }

    /// TP-SPEC-02: every key the spec says a kind refuses is one it really does.
    ///
    /// The direction nothing else covers. A refusal is the half of a grammar a
    /// reader cannot discover except by being turned down, so a spec that
    /// promises one and does not deliver sends somebody looking for a bug in
    /// their own file. `plugin` being handed `argv` is the mistake this exists
    /// for: it is the first thing anybody tries.
    #[test]
    fn every_refusal_the_spec_promises_is_one_this_build_makes() {
        let spec = shell_spec();
        let refused_keys = spec
            .action_kinds
            .iter()
            .flat_map(|entry| entry.refuses.iter().map(move |key| (entry.kind, *key)))
            .collect::<Vec<_>>();
        assert!(
            refused_keys.len() >= 5,
            "the refusals thinned out: {refused_keys:?}"
        );

        for (kind, key) in refused_keys {
            let value = match key {
                "argv" => "[\"true\"]",
                "width" | "height" => "10",
                "secondary" => "\"tab\"",
                "command" => "\"files.open\"",
                "name" => "\"herdr\"",
                other => panic!("no sample value for the refused key {other:?}"),
            };
            // A section that is otherwise complete, so the only thing left to
            // complain about is the key under test.
            let text = format!(
                "[shell.bars.top]\nenabled = true\n\n\
                 [[shell.bars.top.sections]]\nkind = \"content\"\n\
                 widget = {{ kind = \"label\", text = \"x\" }}\n\
                 action = {{ kind = \"{kind}\", {base}{key} = {value} }}\n",
                // The key that makes each kind complete, so the only thing
                // left to complain about is the one under test. A `panic!`
                // rather than a fallback: an action kind added later has to
                // name its own required key here, and a silent default would
                // hand it whichever one the neighbour happened to use.
                base = match kind {
                    "popup" | "run" => "argv = [\"true\"], ",
                    "plugin" => "command = \"files.open\", ",
                    "workspace" => "name = \"herdr\", ",
                    // Complete bare: it reads no keys, so the only thing left
                    // to complain about is already the one under test.
                    "hide" => "",
                    other => panic!("no complete section shape for action kind {other:?}"),
                },
            );
            let found = problems(&text);
            assert!(
                !found.is_empty(),
                "the spec says action kind {kind:?} refuses {key:?}, but this build accepted it\n{text}"
            );
        }
    }

    /// TP-SPEC-03: the spec and the refusal messages name the same things.
    ///
    /// Two surfaces face outward and they must not disagree. A reader who finds
    /// one name in the JSON and another in the message has no way to know which
    /// one this build meant.
    #[test]
    fn the_spec_names_exactly_what_the_refusals_offer() {
        let spec = shell_spec();

        assert_eq!(
            spec.section_kinds
                .iter()
                .map(|entry| entry.kind)
                .collect::<Vec<_>>(),
            source::sizing_kind_facts()
                .iter()
                .map(|fact| fact.name)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            spec.widget_kinds
                .iter()
                .map(|entry| entry.kind)
                .collect::<Vec<_>>(),
            source::widget_kind_facts()
                .iter()
                .map(|fact| fact.name)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            spec.action_kinds
                .iter()
                .map(|entry| entry.kind)
                .collect::<Vec<_>>(),
            source::action_kind_facts()
                .iter()
                .map(|fact| fact.name)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            spec.icon_art.iter().map(|art| art.name).collect::<Vec<_>>(),
            icon::builtin_names(),
            "a bundled picture is listed in one place and not the other"
        );
        // TP-SPEC-14: the spec publishes the colour vocabulary, and publishes
        // the same set the parser resolves. Nothing else can teach it — an
        // unknown colour is not refused, so no message ever names the set.
        assert_eq!(
            spec.colors,
            source::bar_color_tokens(),
            "a colour name is listed in one place and not the other"
        );
    }

    /// TP-SPEC-04: an alias is offered as an alias, never as a name.
    ///
    /// Characterisation, not endorsement. `ram` works and always has, and the
    /// refusal deliberately does not teach it. Listing it among the names would
    /// make the spec contradict the message; leaving it out entirely would tell
    /// somebody a word their file already contains is not accepted.
    #[test]
    fn the_metric_aliases_are_offered_as_aliases_rather_than_as_names() {
        let spec = shell_spec();

        assert!(
            !spec.metrics.names.contains(&"ram"),
            "ram started being advertised as a metric name, which is a change to \
             the grammar rather than to this spec: {:?}",
            spec.metrics.names
        );
        assert!(
            spec.metrics
                .aliases
                .iter()
                .any(|alias| alias.alias == "ram" && alias.means == "mem"),
            "ram stopped being offered as an alias"
        );
    }

    /// TP-SPEC-05: every alias means what the spec says it means.
    ///
    /// The alias list is an assertion about the parser, so it is put to the
    /// parser rather than read back.
    #[test]
    fn every_alias_parses_to_the_metric_it_claims_to_mean() {
        let spec = shell_spec();
        assert!(!spec.metrics.aliases.is_empty(), "the aliases emptied out");

        for alias in &spec.metrics.aliases {
            assert_eq!(
                ResourceMetric::parse(alias.alias),
                ResourceMetric::parse(alias.means),
                "{:?} no longer means {:?}",
                alias.alias,
                alias.means
            );
        }
        for name in &spec.metrics.names {
            assert!(
                ResourceMetric::parse(name).is_some(),
                "the spec names the metric {name:?} but the parser refuses it"
            );
        }
    }

    /// TP-SPEC-12: the bar keys the spec lists are the ones the config
    /// reference documents, in both directions.
    ///
    /// The direction the check below cannot reach. Feeding every listed key
    /// through the parser proves each one is real, but it says nothing about a
    /// key that is real and was never listed — and a spec that quietly omits a
    /// key is a feature nobody can find, which is the failure this whole
    /// surface exists to remove.
    ///
    /// The reference is a separate artefact with a gate of its own, so these
    /// two lists are drawn from different places. Two lists drawn from one
    /// place agree by construction and prove nothing.
    #[test]
    fn the_bar_keys_the_spec_lists_are_the_ones_the_config_reference_documents() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("docs/next/website/src/data/config-reference.json");
        let reference: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path).expect("the config reference is readable"),
        )
        .expect("the config reference is JSON");

        // Every documented key, out of whatever section it is filed under, then
        // narrowed to one edge: all four carry the same vocabulary, so one of
        // them is the whole of it and four would only repeat it.
        let mut documented = reference["sections"]
            .as_array()
            .expect("the reference has sections")
            .iter()
            .flat_map(|section| {
                section["keys"]
                    .as_array()
                    .map(Vec::as_slice)
                    .unwrap_or_default()
            })
            .filter_map(|entry| entry["key"].as_str())
            .filter_map(|key| key.strip_prefix("shell.bars.top."))
            .collect::<Vec<_>>();
        documented.sort_unstable();
        assert!(
            documented.len() >= 6,
            "the reference stopped documenting the top bar, so this check reads nothing \
             at all: {documented:?}"
        );

        let mut listed = shell_spec()
            .bar_keys
            .iter()
            .map(|key| key.key)
            .collect::<Vec<_>>();
        listed.sort_unstable();

        assert_eq!(
            listed, documented,
            "the spec and the config reference disagree about what `[shell.bars.<edge>]` takes"
        );
    }

    /// TP-SPEC-13: a switch that states a range states the one this build enforces.
    ///
    /// The ranges are hand-written text, which is exactly the surface a
    /// staleness gate exists for. Reading the bounds back out of the sentence
    /// and putting them to the checker means the spec cannot promise a range
    /// nobody honours — the same shape as the examples, applied to the half of
    /// the grammar that is a number rather than a name.
    #[test]
    fn every_range_a_switch_states_is_the_one_this_build_enforces() {
        let spec = shell_spec();
        let ranged = spec
            .switches
            .iter()
            .filter(|switch| !switch.range.is_empty())
            .collect::<Vec<_>>();
        assert!(
            !ranged.is_empty(),
            "no switch states a range any more, so this check reads nothing"
        );

        for switch in ranged {
            let (low, high) = switch
                .range
                .split_once('-')
                .expect("a range reads as low-high");
            let low: u64 = low.trim().parse().expect("a numeric lower bound");
            let high: u64 = high.trim().parse().expect("a numeric upper bound");

            for (millis, accepted) in [
                (low, true),
                (high, true),
                (low - 1, false),
                (high + 1, false),
            ] {
                let shell = crate::config::ShellConfig {
                    resource_interval_ms: millis,
                    ..Default::default()
                };
                let refused = super::super::source::shell_config_problems(&shell)
                    .iter()
                    .any(|problem| problem.to_string().contains(switch.key));
                assert_eq!(
                    !refused,
                    accepted,
                    "{key} says its range is {range}, but {millis} was {verdict}",
                    key = switch.key,
                    range = switch.range,
                    verdict = if refused { "refused" } else { "accepted" }
                );
            }
        }
    }

    /// TP-SPEC-06: every bar key is one this build reads.
    ///
    /// There is no table behind `[shell.bars.<edge>]` — the keys are struct
    /// fields — so this is the cross-check that stands in for one: a document
    /// setting all of them at once has to be a document this build accepts.
    #[test]
    fn every_bar_key_is_one_this_build_accepts() {
        let spec = shell_spec();
        assert!(spec.bar_keys.len() >= 6, "the bar keys thinned out");

        let mut text = String::from("[shell.bars.top]\n");
        for key in &spec.bar_keys {
            let value = match key.key {
                "enabled" => "true".to_string(),
                "size" => "3".to_string(),
                "style" => "\"islands\"".to_string(),
                "border" => "true".to_string(),
                "color" => "\"mauve\"".to_string(),
                "background" => "\"bg\"".to_string(),
                "gradient" => "[\"mauve\", \"teal\"]".to_string(),
                "max_sections" => "8".to_string(),
                "hide_when_focused" => "false".to_string(),
                other => panic!("no sample value for the bar key {other:?}"),
            };
            text.push_str(&format!("{} = {value}\n", key.key));
        }

        let found = problems(&text);
        assert!(
            found.is_empty(),
            "a document setting every bar key this spec lists is refused: {found:?}\n{text}"
        );
    }

    /// I7 · a section key nothing publishes is a feature nobody can find.
    ///
    /// `border` and `color` are read by every sizing kind, so they belong to
    /// none of the kind tables: filing them under `fixed` would teach a reader
    /// that `content` turns them down. They need a list of their own, and a
    /// list of their own needs the gate the bar keys already have — every name
    /// on it put through the real parser, so the spec cannot offer a key this
    /// build refuses.
    ///
    /// Read out of the serialised shape rather than off the struct, because
    /// that is what a consumer actually receives: a field that stops being
    /// emitted is invisible to a check that reads the value it was built from.
    #[test]
    fn every_section_key_is_one_this_build_accepts() {
        // TP-CHROME-137: the keys every section takes are published and real.
        let published: serde_json::Value =
            serde_json::to_value(shell_spec()).expect("the spec serialises");
        let listed = published["section_keys"]
            .as_array()
            .expect("the spec publishes the keys every section takes, whatever its kind");
        assert!(
            !listed.is_empty(),
            "an empty list teaches a reader that a section takes nothing but its kind's keys"
        );

        // Two documents rather than one, because the vocabulary itself has a
        // deliberate exclusion in it: a grouped section's frame comes from its
        // group, so `group` and `border` on one section is refused by name.
        // One document per shape proves every key is real without asking the
        // parser to accept a combination this build refuses on purpose.
        // Five rather than three: this bar keeps its own frame, which spends
        // two of the size, and an island needs three of what is left.
        let header = "[shell.bars.top]\nenabled = true\nsize = 5\nborder = true\n\n\
             [[shell.bars.top.sections]]\nkind = \"fixed\"\ncells = 6\n";
        let mut solo = String::from(header);
        let mut grouped = String::from(header);
        for entry in listed {
            let key = entry["key"].as_str().expect("a section key is named");
            let value = match key {
                "group" => "\"sys\"".to_string(),
                "border" => "true".to_string(),
                "color" => "\"teal\"".to_string(),
                "background" => "\"teal\"".to_string(),
                other => panic!("no sample value for the section key {other:?}"),
            };
            if key != "group" {
                solo.push_str(&format!("{key} = {value}\n"));
            }
            if key != "border" {
                grouped.push_str(&format!("{key} = {value}\n"));
            }
        }

        for (shape, text) in [("solo", &solo), ("grouped", &grouped)] {
            let found = problems(text);
            assert!(
                found.is_empty(),
                "a {shape} section setting every key it may take is refused: {found:?}\n{text}"
            );
        }

        // And they are published once. A key that also appeared under a kind
        // would read as that kind's, which is the misunderstanding this list
        // exists to prevent.
        let spec = shell_spec();
        for entry in every_kind(&spec) {
            for key in &entry.keys {
                assert!(
                    !listed
                        .iter()
                        .any(|section_key| section_key["key"].as_str() == Some(*key)),
                    "{key:?} is published both as a section key and as a key of \
                     the {:?} kind",
                    entry.kind
                );
            }
        }
    }

    /// TP-SPEC-08: the spec survives the trip a consumer puts it through.
    #[test]
    fn the_spec_serialises_to_json_a_consumer_can_read() {
        let json = serde_json::to_string_pretty(&shell_spec()).expect("the spec serialises");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("and reads back");

        assert_eq!(parsed["version"], SPEC_VERSION);
        for field in [
            "edges",
            "bar_keys",
            "section_keys",
            "section_kinds",
            "widget_kinds",
            "action_kinds",
            "secondary_presentations",
            "bar_section_menu",
            "clock_fields",
            "metrics",
            "icon_art",
            "switches",
        ] {
            assert!(
                !parsed[field].is_null(),
                "the spec stopped carrying {field:?}, which a consumer reads by name"
            );
        }
        assert!(
            parsed["metrics"]["names"].is_array() && parsed["metrics"]["aliases"].is_array(),
            "the metric shape changed under a consumer"
        );
        // `refuses` is present even when empty, so a consumer branches on what
        // it says rather than on whether it is there.
        assert!(
            parsed["section_kinds"][0]["refuses"].is_array(),
            "an empty refusal list stopped being emitted"
        );
    }

    /// TP-SPEC-09: the text rendering says the same things the JSON does.
    #[test]
    fn the_text_rendering_carries_every_name_the_json_carries() {
        let spec = shell_spec();
        let text = render_text(&spec);

        for entry in every_kind(&spec) {
            assert!(
                text.contains(entry.kind),
                "the text rendering never names {:?}",
                entry.kind
            );
        }
        for art in &spec.icon_art {
            assert!(
                text.contains(art.name),
                "the text rendering never names the bundled art {:?}",
                art.name
            );
        }
        // TP-CHROME-137: the keys every section takes reach the reader who is
        // looking at a terminal rather than parsing JSON. Read off the block
        // they are printed under rather than searched for in the whole dump:
        // `border` and `color` are also bar keys, so `contains` would answer
        // yes for a rendering that never printed the section block at all —
        // the same false pass the colour check above is written against.
        let section_block = text
            .split_once("[[shell.bars.<edge>.sections]]\n")
            .map(|(_, rest)| rest.split("\n\n").next().unwrap_or_default().to_string())
            .unwrap_or_default();
        for key in &spec.section_keys {
            assert!(
                section_block.contains(key.key),
                "the text rendering never names the section key {:?}; the block \
                 it would be under reads {section_block:?}",
                key.key
            );
        }
        for name in &spec.metrics.names {
            assert!(
                text.contains(name),
                "the text rendering never names the metric {name:?}"
            );
        }
        assert!(
            text.contains("ram also works"),
            "the text rendering stopped explaining the alias"
        );

        // TP-SPEC-14: the colour vocabulary reaches the text rendering too, so
        // the reader who is not parsing JSON learns the same set.
        //
        // Colours are read off their own line rather than searched for in the
        // whole rendering. `text` and `red` and `blue` are ordinary words in a
        // grammar dump — `text` is a widget key — so `contains` would answer
        // yes for names this rendering never printed. That is the exact way the
        // guide's picture check used to pass without checking anything.
        let colours = text
            .lines()
            .find_map(|line| line.strip_prefix("colours ("))
            .and_then(|rest| rest.split_once(": "))
            .map(|(_, names)| names.to_string())
            .expect("the text rendering still prints a colours line");
        let printed = colours.split(", ").collect::<Vec<_>>();
        for name in &spec.colors {
            assert!(
                printed.contains(name),
                "the text rendering never names the colour {name:?}; it printed {printed:?}"
            );
        }
    }

    /// Every bundled picture paints with a colour this build resolves.
    ///
    /// The bar holds a person's own colours loosely on purpose: an
    /// unrecognised one is not refused, it warns into the log and comes back
    /// cyan, because refusing would mean a config that loads today and not
    /// tomorrow. That tolerance is right for a colour somebody wrote and can
    /// see, and wrong for one this build ships: a typo in a bundled palette
    /// would draw cyan on every machine, and the person looking at it never
    /// wrote the line that produced it.
    ///
    /// A palette token rather than a literal, too. A bundled picture that
    /// baked `#cba6f7` would be the one mark on the bar that kept its colour
    /// when somebody switched theme.
    #[test]
    fn every_bundled_picture_paints_with_a_colour_this_build_knows() {
        // TP-ART-07: a bundled palette names colours the bar resolves, so no
        // shipped picture can arrive as the unknown-colour fallback.
        let known = source::bar_color_tokens();
        for name in icon::builtin_names() {
            let (_, palette) = icon::builtin(name).expect("the name came from the catalogue");
            assert!(!palette.is_empty(), "{name} paints with no colours at all");
            for (key, colour) in palette {
                assert!(
                    known.contains(&colour.as_str()),
                    "the bundled picture {name} paints key {key:?} with {colour:?}, which is \
                     not a colour this build names; it would reach the screen as cyan and \
                     nobody would have written it. Known: {known:?}"
                );
            }
        }
    }

    /// TP-SPEC-10: the whole catalogue is readable.
    ///
    /// `builtin_catalogue` leaves out a picture it cannot read rather than
    /// panicking in front of somebody running the CLI. That is the right
    /// behaviour there and the wrong silence to keep here, so this is where an
    /// unreadable picture is noticed.
    #[test]
    fn every_bundled_picture_is_one_that_can_be_drawn() {
        assert_eq!(
            icon::builtin_catalogue().len(),
            icon::builtin_names().len(),
            "a bundled picture cannot be read and was quietly left out of the catalogue"
        );
        for (name, cells, rows) in icon::builtin_catalogue() {
            assert!(cells > 0 && rows > 0, "{name} draws at no size at all");
        }
    }
}
