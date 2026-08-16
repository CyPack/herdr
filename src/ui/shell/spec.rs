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
    /// How a section asks for space.
    pub section_kinds: Vec<KindSpec>,
    /// What a section shows.
    pub widget_kinds: Vec<KindSpec>,
    /// What a press on a section does.
    pub action_kinds: Vec<KindSpec>,
    /// What a second gesture can ask for.
    pub secondary_presentations: Vec<&'static str>,
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
        key: "border",
        value_type: "boolean",
        range: "",
        default: "true",
    },
    KeySpec {
        key: "color",
        value_type: "string",
        range: "",
        default: "",
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
];

/// Switches that live outside `[shell.bars]` and change what a bar may do.
const SWITCHES: &[KeySpec] = &[KeySpec {
    key: "shell.glyph_icons",
    value_type: "boolean",
    range: "",
    default: "true",
}];

/// The bar grammar this build actually implements.
pub(crate) fn shell_spec() -> ShellSpec {
    ShellSpec {
        version: SPEC_VERSION,
        edges: vec!["top", "bottom", "left", "right"],
        bar_keys: BAR_KEYS.iter().map(KeySpec::copied).collect(),
        section_kinds: kinds(source::sizing_kind_facts()),
        widget_kinds: kinds(source::widget_kind_facts()),
        action_kinds: kinds(source::action_kind_facts()),
        secondary_presentations: source::secondary_presentation_names(),
        metrics: MetricSpec {
            names: ResourceMetric::ACCEPTED.to_vec(),
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
            "  {name}: {cells} cells by {rows} rows\n",
            name = art.name,
            cells = art.cells,
            rows = art.rows
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
        assert!(spec.metrics.names.len() >= 3, "a metric went missing");
        assert!(!spec.icon_art.is_empty(), "the art catalogue emptied out");
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
                other => panic!("no sample value for the refused key {other:?}"),
            };
            // A section that is otherwise complete, so the only thing left to
            // complain about is the key under test.
            let text = format!(
                "[shell.bars.top]\nenabled = true\n\n\
                 [[shell.bars.top.sections]]\nkind = \"content\"\n\
                 widget = {{ kind = \"label\", text = \"x\" }}\n\
                 action = {{ kind = \"{kind}\", {base}, {key} = {value} }}\n",
                base = match kind {
                    "popup" => "argv = [\"true\"]",
                    _ => "command = \"files.open\"",
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
                "border" => "true".to_string(),
                "color" => "\"mauve\"".to_string(),
                "gradient" => "[\"mauve\", \"teal\"]".to_string(),
                "max_sections" => "8".to_string(),
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

    /// TP-SPEC-08: the spec survives the trip a consumer puts it through.
    #[test]
    fn the_spec_serialises_to_json_a_consumer_can_read() {
        let json = serde_json::to_string_pretty(&shell_spec()).expect("the spec serialises");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("and reads back");

        assert_eq!(parsed["version"], SPEC_VERSION);
        for field in [
            "edges",
            "bar_keys",
            "section_kinds",
            "widget_kinds",
            "action_kinds",
            "secondary_presentations",
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
