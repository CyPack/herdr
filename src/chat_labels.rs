//! What kind of conversation a chat is.
//!
//! The daily section lists a directory's chats newest-first, and a task that
//! runs on a timer is newer than real work almost every time it runs. Labelling
//! is how those are told apart without deleting anything: the transcript stays
//! on disk, the row stays reachable, and only the busiest surface stops leading
//! with chores.
//!
//! The rules here are deliberately dull. Each one answers from the *opening* —
//! the normalised shape of a chat's first message, which
//! [`crate::claude_sessions::normalise_opening`] already produces for free
//! while the file is parsed. Nothing here reads a transcript a second time.

/// A chat's kind. Absence is a fourth state and it is not a synonym for
/// [`ChatLabel::Context`]: a chat no rule recognised is unclassified, and
/// saying otherwise would be inventing a fact about someone's work.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChatLabel {
    /// Opened to understand or research something.
    Context,
    /// The conversation a piece of work is carried out in.
    Project,
    /// An automation: a timer, a hook, a probe, a namer.
    Routine,
}

/// The spelling of [`ChatLabel::Context`] in config files and in the ledger.
pub const CONTEXT_CHAT_LABEL: &str = "context";
/// The spelling of [`ChatLabel::Project`] in config files and in the ledger.
pub const PROJECT_CHAT_LABEL: &str = "project";

/// The label picker's items, written the way a menu writes things.
///
/// Kept apart from the config spellings on purpose: a config file is read by a
/// parser and a menu is read by a person, and tying them together would mean
/// either a lowercase menu or a capitalised config key.
pub const CONTEXT_CHAT_LABEL_ITEM: &str = "Context";
/// See [`CONTEXT_CHAT_LABEL_ITEM`].
pub const PROJECT_CHAT_LABEL_ITEM: &str = "Project";
/// See [`CONTEXT_CHAT_LABEL_ITEM`].
pub const ROUTINE_CHAT_LABEL_ITEM: &str = "Routine";
/// See [`CONTEXT_CHAT_LABEL_ITEM`].
pub const CLEAR_CHAT_LABEL_ITEM: &str = "Clear label";

/// The label a picker item stands for, or `None` for the withdrawal item.
pub fn label_for_menu_item(item: &str) -> Option<ChatLabel> {
    match item {
        CONTEXT_CHAT_LABEL_ITEM => Some(ChatLabel::Context),
        PROJECT_CHAT_LABEL_ITEM => Some(ChatLabel::Project),
        ROUTINE_CHAT_LABEL_ITEM => Some(ChatLabel::Routine),
        _ => None,
    }
}

impl ChatLabel {
    /// The spelling written into config files and into the ledger.
    pub fn as_config_name(self) -> &'static str {
        match self {
            Self::Context => CONTEXT_CHAT_LABEL,
            Self::Project => PROJECT_CHAT_LABEL,
            Self::Routine => crate::config::ROUTINE_CHAT_LABEL,
        }
    }

    /// Read a label written by a person. Unknown spellings are `None` rather
    /// than an error: a typo in `hidden_chat_labels` should hide nothing,
    /// never everything.
    ///
    /// The three spellings are named as constants rather than written out
    /// here, because the config default has to say `routine` too and two
    /// literals in two files is exactly how a rule and its default drift into
    /// disagreeing.
    pub fn from_config_name(name: &str) -> Option<Self> {
        let name = name.trim().to_ascii_lowercase();
        match name.as_str() {
            CONTEXT_CHAT_LABEL => Some(Self::Context),
            PROJECT_CHAT_LABEL => Some(Self::Project),
            crate::config::ROUTINE_CHAT_LABEL => Some(Self::Routine),
            _ => None,
        }
    }
}

/// The rules that decide whether an opening is a routine, in the order they are
/// asked. Borrowed from config so the caller keeps ownership.
pub struct RoutineRules<'a> {
    /// Structural openings a chat writes about itself.
    pub markers: &'a [String],
    /// Further openings a person recognises as their own chores.
    pub patterns: &'a [String],
}

/// K1 + K2: does this opening declare itself a routine?
///
/// Both layers are prefix rules, and that is the whole of their precision. A
/// scheduled task announces itself in the first characters of its first
/// message; a person writing *about* scheduled tasks mentions them somewhere in
/// the middle of a sentence. Matching anywhere would collapse that difference
/// and hide the conversation where the feature was designed.
///
/// Returns `None` when no rule recognised the opening. That is not
/// [`ChatLabel::Context`] — see the type.
pub fn classify_opening(opening: &str, rules: &RoutineRules<'_>) -> Option<ChatLabel> {
    // The two lists differ in who writes them, not in how they match: the
    // markers ship with Herdr because a scheduled task declares itself the
    // same way on every machine, and the patterns are added by the person
    // whose chores they are. Both are put through the same normalisation the
    // opening already went through, or a rule typed with capitals would be
    // correct and match nothing.
    rules
        .markers
        .iter()
        .chain(rules.patterns.iter())
        .filter_map(|rule| {
            let rule = crate::claude_sessions::normalise_opening(rule);
            (!rule.is_empty()).then_some(rule)
        })
        .any(|rule| opening.starts_with(&rule))
        .then_some(ChatLabel::Routine)
}

/// Read a list of label names written by a person, dropping the ones nothing
/// recognises.
///
/// Dropping rather than keeping is the whole safety of the hidden-labels
/// setting: an entry that survives as an unmatchable string is harmless, but an
/// entry that survives as *something* could hide a surface. A misspelling here
/// must cost the reader nothing more than the line not working.
pub fn resolve_labels(names: &[String]) -> Vec<ChatLabel> {
    names
        .iter()
        .filter_map(|name| ChatLabel::from_config_name(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules<'a>(markers: &'a [String], patterns: &'a [String]) -> RoutineRules<'a> {
        RoutineRules { markers, patterns }
    }

    fn owned(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    /// TP-DAILY-21 (K1a): the largest certain group measured on the reported
    /// machine — 170 chats — opens with a scheduled task announcing its own
    /// name and file. This is the one rule that needs no guessing at all.
    #[test]
    fn an_opening_that_announces_a_scheduled_task_is_routine() {
        let markers = owned(&["<scheduled-task", "<command-name>"]);
        let patterns = Vec::new();
        assert_eq!(
            classify_opening(
                "<scheduled-task name=\"bam-t<n>f-daily-tasks\" file=\"<path>\">",
                &rules(&markers, &patterns)
            ),
            Some(ChatLabel::Routine)
        );
    }

    /// TP-DAILY-21 (K1b): a marker in the middle of a sentence is a person
    /// talking about automations, which is exactly the conversation this
    /// feature was designed in. Matching anywhere would hide it.
    #[test]
    fn a_marker_mentioned_mid_sentence_is_not_a_declaration() {
        let markers = owned(&["<scheduled-task", "<command-name>"]);
        let patterns = Vec::new();
        assert_eq!(
            classify_opening(
                "why does <scheduled-task never fire on sundays?",
                &rules(&markers, &patterns)
            ),
            None
        );
    }

    /// TP-DAILY-21 (K1c): an install that configures nothing must classify
    /// nothing, not fall over.
    #[test]
    fn without_markers_the_structural_rule_matches_nothing() {
        let markers: Vec<String> = Vec::new();
        let patterns = Vec::new();
        assert_eq!(
            classify_opening("<scheduled-task name=\"x\">", &rules(&markers, &patterns)),
            None
        );
    }

    /// TP-DAILY-21 (K2a): the escape hatch for the chores only their owner
    /// knows about — 185 chats here repeat an opening no marker describes.
    #[test]
    fn an_opening_that_starts_with_a_known_pattern_is_routine() {
        let markers = Vec::new();
        let patterns = owned(&["claude code update"]);
        assert_eq!(
            classify_opening("claude code update", &rules(&markers, &patterns)),
            Some(ChatLabel::Routine)
        );
    }

    /// TP-DAILY-21 (K2b): patterns are written by hand, so they arrive with
    /// whatever capitals and spacing the writer used. They are compared in the
    /// same normalised shape the opening is already in, or a correct pattern
    /// would silently match nothing.
    #[test]
    fn a_pattern_matches_through_case_and_spacing() {
        let markers = Vec::new();
        let patterns = owned(&["  Claude   Code   Update  "]);
        assert_eq!(
            classify_opening(
                "claude code update to the latest version",
                &rules(&markers, &patterns)
            ),
            Some(ChatLabel::Routine)
        );
    }

    /// TP-DAILY-21 (K2c): an empty string is a prefix of everything. One blank
    /// line in either list would mark every chat routine and empty the daily
    /// section — the exact outcome this feature exists to prevent, arrived at
    /// by a typo.
    #[test]
    fn a_blank_rule_marks_nothing_rather_than_everything() {
        let markers = owned(&["", "   "]);
        let patterns = owned(&[""]);
        assert_eq!(
            classify_opening(
                "a real conversation about a real thing",
                &rules(&markers, &patterns)
            ),
            None
        );
    }

    /// TP-DAILY-21 (K5): no rule recognised it, so nothing is claimed. An
    /// unlabelled chat is drawn; guessing `context` here would be inventing a
    /// fact about the work.
    #[test]
    fn an_unrecognised_opening_is_left_unlabelled() {
        let markers = owned(&["<scheduled-task"]);
        let patterns = owned(&["claude code update"]);
        assert_eq!(
            classify_opening(
                "can you look at why the sidebar redraws twice?",
                &rules(&markers, &patterns)
            ),
            None
        );
    }

    /// TP-DAILY-21: the label spellings are a two-way contract with the config
    /// file and the ledger, so both directions are pinned. An unknown spelling
    /// reads as nothing rather than as a default, because a typo in the hidden
    /// list must hide nothing rather than everything.
    #[test]
    fn label_names_are_read_and_unknown_names_are_nothing() {
        assert_eq!(
            ChatLabel::from_config_name(CONTEXT_CHAT_LABEL),
            Some(ChatLabel::Context)
        );
        assert_eq!(
            ChatLabel::from_config_name(PROJECT_CHAT_LABEL),
            Some(ChatLabel::Project)
        );
        assert_eq!(
            ChatLabel::from_config_name(crate::config::ROUTINE_CHAT_LABEL),
            Some(ChatLabel::Routine)
        );
        assert_eq!(
            ChatLabel::from_config_name(" Routine "),
            Some(ChatLabel::Routine)
        );
        assert_eq!(ChatLabel::from_config_name("routin"), None);
        assert_eq!(ChatLabel::from_config_name(""), None);
    }

    /// TP-DAILY-21: a hidden-labels list is read entry by entry, and an entry
    /// nothing recognises is dropped rather than carried. Kept as an unmatched
    /// value it would be a rule nobody wrote, sitting in a list whose whole
    /// job is to remove things from a screen.
    #[test]
    fn unreadable_hidden_labels_are_dropped_rather_than_kept() {
        let names = vec![
            "routine".to_string(),
            "routin".to_string(),
            "  PROJECT ".to_string(),
            String::new(),
        ];
        assert_eq!(
            resolve_labels(&names),
            vec![ChatLabel::Routine, ChatLabel::Project]
        );
    }
}
