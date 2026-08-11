//! Where the desktop shell tree comes from, and what identifies the result.
//!
//! Until now the answer was written inline at the single call site: a hardcoded
//! `ShellLayout::default()` next to a hardcoded revision constant. That was
//! honest while there was exactly one possible tree, and it stops being honest
//! the moment there are two — because the revision is what the geometry cache
//! keys on, and a constant cannot describe a tree that changes.
//!
//! So derivation gets one home. Today it answers with the same legacy tree and
//! the same revision, which is why nothing on screen moves; tomorrow it is the
//! one place that learns about configured edge bars, and the cache key follows
//! it for free.
//!
//! Fail-closed by construction: a template that does not validate falls back to
//! the legacy tree rather than propagating an error to a renderer that has no
//! way to answer it. A shell that cannot be composed must still show a shell.

use super::model::{ShellLayout, ShellValidationError, ValidatedShellLayout};
use super::template::ShellTemplateId;

/// Identity of the tree the desktop shell has always drawn.
///
/// Kept at its historical value so that the default path's cache key is byte
/// identical to the one it had before derivation existed.
pub(crate) const LEGACY_DESKTOP_REVISION: u64 = 1;

/// Where built-in template revisions start, far enough from the legacy value
/// that the two spaces can never be confused by eye in a log line.
const TEMPLATE_REVISION_BASE: u64 = 100;

/// A shell tree together with everything the geometry cache needs to know that
/// it is looking at that tree and not another one.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DerivedShellLayout {
    pub layout: ShellLayout,
    pub revision: u64,
    /// `None` means the legacy desktop tree, which is not a built-in template.
    pub template: Option<ShellTemplateId>,
}

/// Derive the desktop shell tree from what the user asked for.
///
/// `None` is today's production request and yields exactly today's tree.
pub(crate) fn derive_desktop_shell_layout(
    requested: Option<ShellTemplateId>,
) -> DerivedShellLayout {
    let Some(template) = requested else {
        return legacy_desktop_layout();
    };
    finish(template, template.validated_layout())
}

/// The seam where a validation verdict becomes a tree.
///
/// Separated so the fail-closed branch is reachable from a test: the five
/// built-in templates all validate today, and a guard that cannot be exercised
/// is a guard nobody can trust.
fn finish(
    template: ShellTemplateId,
    validated: Result<ValidatedShellLayout, ShellValidationError>,
) -> DerivedShellLayout {
    match validated {
        Ok(valid) => DerivedShellLayout {
            layout: valid.as_layout().clone(),
            revision: revision_for(template),
            template: Some(template),
        },
        // The identity falls back with the tree. Reporting a template we did
        // not draw would poison the cache key with a lie.
        Err(_) => legacy_desktop_layout(),
    }
}

fn legacy_desktop_layout() -> DerivedShellLayout {
    DerivedShellLayout {
        layout: ShellLayout::default(),
        revision: LEGACY_DESKTOP_REVISION,
        template: None,
    }
}

const fn revision_for(template: ShellTemplateId) -> u64 {
    TEMPLATE_REVISION_BASE
        + match template {
            ShellTemplateId::StageOnly => 0,
            ShellTemplateId::DockStage => 1,
            ShellTemplateId::DockSidebarStage => 2,
            ShellTemplateId::DesktopWorkspace => 3,
            ShellTemplateId::InspectorWorkspace => 4,
        }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_TEMPLATES: [ShellTemplateId; 5] = [
        ShellTemplateId::StageOnly,
        ShellTemplateId::DockStage,
        ShellTemplateId::DockSidebarStage,
        ShellTemplateId::DesktopWorkspace,
        ShellTemplateId::InspectorWorkspace,
    ];

    // T5 · the default path is the path that already exists
    #[test]
    fn asking_for_nothing_derives_exactly_todays_tree() {
        // The whole promise of this layer: introducing a derivation must not
        // move a single cell. If this drifts, every visual baseline the layout
        // lock protects drifts with it, and nothing else would say so.
        let derived = derive_desktop_shell_layout(None);
        assert_eq!(derived.layout, ShellLayout::default());
        assert_eq!(derived.revision, LEGACY_DESKTOP_REVISION);
        assert_eq!(derived.template, None);
    }

    // T7 · whatever is derived is composable — the stage always survives
    #[test]
    fn every_derived_tree_still_validates() {
        for template in ALL_TEMPLATES {
            let derived = derive_desktop_shell_layout(Some(template));
            assert!(
                derived.layout.clone().validate().is_ok(),
                "{template:?} derived a tree that cannot be composed"
            );
        }
        assert!(derive_desktop_shell_layout(None).layout.validate().is_ok());
    }

    // T9 · a different tree is a different identity, or the cache lies
    #[test]
    fn every_template_carries_its_own_revision() {
        let mut seen = vec![LEGACY_DESKTOP_REVISION];
        for template in ALL_TEMPLATES {
            let derived = derive_desktop_shell_layout(Some(template));
            assert_eq!(derived.template, Some(template));
            assert!(
                !seen.contains(&derived.revision),
                "{template:?} reuses a revision another tree already claimed"
            );
            seen.push(derived.revision);
        }
    }

    // T6 · a tree that will not compose falls back, and says so in its identity
    #[test]
    fn a_template_that_does_not_validate_falls_back_to_the_legacy_tree() {
        // Reached through the seam because all five built-ins validate today.
        // A guard that cannot be exercised is a guard nobody can trust, and the
        // interesting half is the IDENTITY: claiming a template we did not draw
        // would key the cache on a tree that is not on screen.
        let derived = finish(
            ShellTemplateId::DesktopWorkspace,
            Err(ShellValidationError::MissingWorkspaceStage),
        );
        assert_eq!(derived.layout, ShellLayout::default());
        assert_eq!(derived.revision, LEGACY_DESKTOP_REVISION);
        assert_eq!(derived.template, None);
    }
}
