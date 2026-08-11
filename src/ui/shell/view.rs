use ratatui::layout::{Position, Rect};

use super::template::ShellTemplateId;
use super::{RegionId, RegionRects, ResponsiveDegradation, ShellLayout};

const FLATTENED_REGION_ORDER: [RegionId; 6] = [
    RegionId::TopBar,
    RegionId::AppDock,
    RegionId::LeftPanel,
    RegionId::WorkspaceStage,
    RegionId::RightPanel,
    RegionId::BottomBar,
];

/// Complete authority key for one cached shell geometry projection.
///
/// "Complete" is the load-bearing word: a projection may only be reused when
/// every input that decided it is the same, and the template is one of those
/// inputs — a different template is a different set of regions, at different
/// rectangles, answering different hits. It is carried as an identity rather
/// than folded into `layout_revision`, because a number nobody bumps looks
/// exactly like a number that did not need bumping.
///
/// `None` is today's production path: the desktop shell is still derived from
/// `ShellLayout::default()`, which is not one of the built-in templates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellGeometryKey {
    pub area: Rect,
    pub layout_revision: u64,
    pub constraints_revision: u64,
    pub collapse_revision: u64,
    pub template: Option<ShellTemplateId>,
}

impl ShellGeometryKey {
    pub(crate) const fn new(
        area: Rect,
        layout_revision: u64,
        constraints_revision: u64,
        collapse_revision: u64,
        template: Option<ShellTemplateId>,
    ) -> Self {
        Self {
            area,
            layout_revision,
            constraints_revision,
            collapse_revision,
            template,
        }
    }
}

impl Default for ShellGeometryKey {
    fn default() -> Self {
        Self::new(Rect::ZERO, 0, 0, 0, None)
    }
}

/// Stable semantic target carried by a flattened shell hit area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellHitTarget {
    Region(RegionId),
}

/// One complete non-zero hit rectangle from a specific shell generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ShellHitArea {
    pub generation: u64,
    pub target: ShellHitTarget,
    pub rect: Rect,
}

/// Cached, client-local presentation projection of the bounded outer shell.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShellView {
    pub generation: u64,
    pub area: Rect,
    pub regions: RegionRects,
    pub(super) hits: Vec<ShellHitArea>,
    pub degradation: ResponsiveDegradation,
    pub(super) geometry_key: ShellGeometryKey,
}

impl Default for ShellView {
    fn default() -> Self {
        Self {
            generation: 0,
            area: Rect::ZERO,
            regions: RegionRects::default(),
            hits: Vec::new(),
            degradation: ResponsiveDegradation::Workspace,
            geometry_key: ShellGeometryKey::default(),
        }
    }
}

impl ShellView {
    /// Resolve only a hit from this exact geometry generation. SF4 wires this
    /// pure seam into the topmost input router.
    pub(super) fn hit_at(&self, generation: u64, position: Position) -> Option<ShellHitTarget> {
        if generation != self.generation {
            return None;
        }
        self.hits
            .iter()
            .rev()
            .find(|hit| hit.generation == generation && hit.rect.contains(position))
            .map(|hit| hit.target)
    }

    /// Crate-visible region projection of `hit_at` for the shell input
    /// router: a region is positional authority only against the exact
    /// current generation, so stale coordinates resolve to nothing.
    pub(crate) fn region_hit_at(&self, generation: u64, position: Position) -> Option<RegionId> {
        self.hit_at(generation, position)
            .map(|target| match target {
                ShellHitTarget::Region(region) => region,
            })
    }
}

pub(crate) fn compute_shell_view(
    layout: &ShellLayout,
    key: ShellGeometryKey,
    previous: ShellView,
    resolve_dynamic: &impl Fn(RegionId) -> u16,
) -> ShellView {
    if previous.geometry_key == key {
        crate::render_prof::event("shell.geometry_cache.hit");
        return previous;
    }
    crate::render_prof::event("shell.geometry_cache.miss");

    let (regions, degradation) = layout.compute_projection(key.area, resolve_dynamic);
    project_changed_geometry(key, previous.generation, regions, degradation)
}

pub(crate) fn compute_empty_shell_view(key: ShellGeometryKey, previous: ShellView) -> ShellView {
    if previous.geometry_key == key {
        crate::render_prof::event("shell.geometry_cache.hit");
        return previous;
    }
    crate::render_prof::event("shell.geometry_cache.miss");

    project_changed_geometry(
        key,
        previous.generation,
        RegionRects::default(),
        ResponsiveDegradation::Workspace,
    )
}

fn project_changed_geometry(
    key: ShellGeometryKey,
    previous_generation: u64,
    regions: RegionRects,
    degradation: ResponsiveDegradation,
) -> ShellView {
    let Some(generation) = previous_generation.checked_add(1) else {
        // Exhaustion must never alias an older hit generation. Keep the new
        // geometry visible but fail closed with no interactive shell targets.
        return ShellView {
            generation: previous_generation,
            area: key.area,
            regions,
            hits: Vec::new(),
            degradation,
            geometry_key: key,
        };
    };
    let hits = flatten_region_hits(&regions, generation);

    ShellView {
        generation,
        area: key.area,
        regions,
        hits,
        degradation,
        geometry_key: key,
    }
}

fn flatten_region_hits(regions: &RegionRects, generation: u64) -> Vec<ShellHitArea> {
    FLATTENED_REGION_ORDER
        .into_iter()
        .filter_map(|region| {
            let rect = regions.get(region);
            (!rect.is_empty()).then_some(ShellHitArea {
                generation,
                target: ShellHitTarget::Region(region),
                rect,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_cache_profile_counts_desktop_and_empty_hits_and_misses() {
        let layout = ShellLayout::default();
        let desktop_key = ShellGeometryKey::new(Rect::new(0, 0, 120, 40), 1, 2, 3, None);
        let mobile_key = ShellGeometryKey::new(Rect::new(0, 0, 40, 20), 4, 5, 6, None);

        let (_, profile) = crate::render_prof::observe_for_test(|| {
            let desktop =
                compute_shell_view(&layout, desktop_key, ShellView::default(), &|_region| 0);
            let _desktop_hit = compute_shell_view(&layout, desktop_key, desktop, &|_region| 0);

            let mobile = compute_empty_shell_view(mobile_key, ShellView::default());
            let _mobile_hit = compute_empty_shell_view(mobile_key, mobile);
        });

        assert_eq!(profile.counter("shell.geometry_cache.miss"), 2);
        assert_eq!(profile.counter("shell.geometry_cache.hit"), 2);
    }

    // T1 · the key must carry EVERY input that decides geometry (CLA3).
    #[test]
    fn a_template_is_part_of_the_geometry_authority() {
        // Before this field existed, two different templates with the same area
        // and the same revisions produced the SAME key. The cache would then
        // hand back the previous template's rectangles and hit-testing would
        // answer for regions that are no longer on screen. Nothing goes red
        // when that happens — the wrong region simply replies, which is why the
        // catalogue files it as a silent-loss anti-pattern.
        let area = Rect::new(0, 0, 120, 40);
        let legacy = ShellGeometryKey::new(area, 1, 2, 3, None);
        let dock = ShellGeometryKey::new(area, 1, 2, 3, Some(ShellTemplateId::DockStage));
        let desktop = ShellGeometryKey::new(area, 1, 2, 3, Some(ShellTemplateId::DesktopWorkspace));

        assert_ne!(legacy, dock);
        assert_ne!(dock, desktop);
        assert_eq!(
            dock,
            ShellGeometryKey::new(area, 1, 2, 3, Some(ShellTemplateId::DockStage))
        );
    }

    // T2 + T4 · a changed authority is a new generation, exactly once.
    #[test]
    fn a_template_change_misses_the_cache_and_advances_the_generation_once() {
        let layout = ShellLayout::default();
        let area = Rect::new(0, 0, 120, 40);
        let dock = ShellGeometryKey::new(area, 1, 2, 3, Some(ShellTemplateId::DockStage));
        let desktop = ShellGeometryKey::new(area, 1, 2, 3, Some(ShellTemplateId::DesktopWorkspace));

        let (generations, profile) = crate::render_prof::observe_for_test(|| {
            let first = compute_shell_view(&layout, dock, ShellView::default(), &|_region| 0);
            let changed = compute_shell_view(&layout, desktop, first.clone(), &|_region| 0);
            let repeat = compute_shell_view(&layout, desktop, changed.clone(), &|_region| 0);
            (first.generation, changed.generation, repeat.generation)
        });

        let (first, changed, repeat) = generations;
        assert_eq!(profile.counter("shell.geometry_cache.miss"), 2);
        assert_eq!(profile.counter("shell.geometry_cache.hit"), 1);
        assert_eq!(changed, first + 1, "a new authority is a new generation");
        assert_eq!(
            repeat, changed,
            "an unchanged key must not renumber the world"
        );
    }

    // T3 · today's path keeps its identity: no template is its own answer, not
    // an accident, and two legacy keys with the same numbers still match.
    #[test]
    fn the_legacy_path_keeps_its_identity() {
        let area = Rect::new(0, 0, 120, 40);
        assert_eq!(
            ShellGeometryKey::new(area, 1, 2, 3, None),
            ShellGeometryKey::new(area, 1, 2, 3, None)
        );
        assert_eq!(
            ShellGeometryKey::default(),
            ShellGeometryKey::new(Rect::ZERO, 0, 0, 0, None)
        );
    }
}
