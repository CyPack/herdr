use ratatui::layout::{Position, Rect};

use super::source::ShellBars;
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
    /// The exact edge composition, not a digest of it. Two different screens
    /// must never answer to one identity, and this is small enough to compare.
    pub bars: ShellBars,
}

impl ShellGeometryKey {
    pub(crate) const fn new(
        area: Rect,
        layout_revision: u64,
        constraints_revision: u64,
        collapse_revision: u64,
        template: Option<ShellTemplateId>,
        bars: ShellBars,
    ) -> Self {
        Self {
            area,
            layout_revision,
            constraints_revision,
            collapse_revision,
            template,
            bars,
        }
    }
}

impl Default for ShellGeometryKey {
    fn default() -> Self {
        Self::new(Rect::ZERO, 0, 0, 0, None, ShellBars::NONE)
    }
}

/// Stable semantic target carried by a flattened shell hit area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ShellHitTarget {
    Region(RegionId),
    /// One numbered part of an edge bar. The index is the position the person
    /// wrote it at, which is the only stable name a section has — a rectangle
    /// is not an identity, and CL5 resolves by identity.
    BarSection {
        region: RegionId,
        index: u8,
    },
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
                // A section is part of its bar for the purposes of "which
                // region did they click": the finer answer is a separate
                // question, asked by `bar_section_hit_at`.
                ShellHitTarget::BarSection { region, .. } => region,
            })
    }

    /// Which numbered section of which bar a position lands in, against this
    /// exact generation.
    ///
    /// Separate from [`Self::region_hit_at`] so that a caller who only wants a
    /// region is not forced to care that bars can be divided, and so that a
    /// caller who wants the section cannot get one from a stale geometry.
    // Read on the input path by `AppState::bar_section_click_at`, which is the
    // production caller this was written ahead of (F34-L9).
    pub(crate) fn bar_section_hit_at(
        &self,
        generation: u64,
        position: Position,
    ) -> Option<(RegionId, u8)> {
        match self.hit_at(generation, position)? {
            ShellHitTarget::BarSection { region, index } => Some((region, index)),
            ShellHitTarget::Region(_) => None,
        }
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
    let mut hits = flatten_region_hits(&regions, generation);
    append_bar_section_hits(&mut hits, &regions, key.bars, generation);

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

/// The edges that can carry a divided bar, and the region each one is.
const BAR_REGIONS: [RegionId; 4] = [
    RegionId::TopBar,
    RegionId::BottomBar,
    RegionId::AppDock,
    RegionId::RightPanel,
];

// TP-CHROME-29/33: sections are clicked where they are drawn, against this
// generation only, and an undivided bar contributes nothing here.
/// Put each bar's sections on top of the bar itself.
///
/// Appended after the region hits so that `hit_at`'s reverse scan finds the
/// finer target first: a click inside a divided bar belongs to the section it
/// landed in, and only to the bar when it landed in none.
///
/// The division is recomputed here from `key.bars` — the same value the drawing
/// path divides by — rather than being handed in from somewhere else. That is
/// deliberate and it is the whole guard: this line is exactly where C79/C80
/// were born three times over, each time because one side read the live
/// context and the other was handed a constant.
fn append_bar_section_hits(
    hits: &mut Vec<ShellHitArea>,
    regions: &RegionRects,
    bars: ShellBars,
    generation: u64,
) {
    for region in BAR_REGIONS {
        let track = bars.track_for(region);
        if track.sections().is_empty() {
            continue;
        }
        let outer = regions.get(region);
        if outer.is_empty() {
            continue;
        }
        for (index, rect) in track.section_rects(region, outer).occupied() {
            hits.push(ShellHitArea {
                generation,
                target: ShellHitTarget::BarSection {
                    region,
                    index: index as u8,
                },
                rect,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::shell::model::TrackPolicy;
    use crate::ui::shell::source::{derive_desktop_shell_layout, BarSections, BarTrack};

    /// The bars a divided top strip would produce, and the view they project.
    fn sectioned_top_bar_view(
        area: Rect,
        bordered: bool,
        policies: &[TrackPolicy],
    ) -> (ShellBars, ShellView) {
        let sections = BarSections::from_policies(policies, "top");
        let track = if bordered {
            BarTrack::bordered(3)
        } else {
            BarTrack::of(1)
        };
        let bars = ShellBars {
            top: track.with_sections(sections),
            ..ShellBars::NONE
        };
        let derived = derive_desktop_shell_layout(None, bars);
        let key = ShellGeometryKey::new(area, derived.revision, 0, 0, derived.template, bars);
        let view = compute_shell_view(&derived.layout, key, ShellView::default(), &|region| {
            u16::from(region == RegionId::LeftPanel) * 26
        });
        (bars, view)
    }

    // T24 · a section is positional authority only against its own generation
    #[test]
    fn a_bar_section_answers_only_for_the_generation_that_drew_it() {
        let (_, view) = sectioned_top_bar_view(
            Rect::new(0, 0, 80, 24),
            false,
            &[
                TrackPolicy::Fill { weight: 1 },
                TrackPolicy::Fill { weight: 1 },
            ],
        );

        let left = Position::new(5, 0);
        assert_eq!(
            view.bar_section_hit_at(view.generation, left),
            Some((RegionId::TopBar, 0)),
            "the left half of a divided top bar is its first section"
        );
        assert_eq!(
            view.bar_section_hit_at(view.generation, Position::new(60, 0)),
            Some((RegionId::TopBar, 1))
        );
        // A coordinate from a geometry that no longer exists resolves to
        // nothing rather than to whatever now happens to occupy those cells.
        assert_eq!(
            view.bar_section_hit_at(view.generation.wrapping_add(1), left),
            None
        );
    }

    // T24b · the rectangle a section is drawn in is the rectangle it is clicked in
    #[test]
    fn every_bar_section_is_clicked_exactly_where_it_is_drawn() {
        // This is the failure that has been produced three separate times on
        // this line (C79, then C80 in three places): drawing reads the live
        // chrome, hit-testing is handed a constant, and both rectangles are
        // plausible so nothing goes red. Deriving both from the same value is
        // the guard, and this test is what proves the derivation is shared.
        for bordered in [false, true] {
            let area = Rect::new(0, 0, 80, 24);
            let (bars, view) = sectioned_top_bar_view(
                area,
                bordered,
                &[
                    TrackPolicy::Fixed { cells: 12 },
                    TrackPolicy::Fill { weight: 1 },
                    TrackPolicy::Fill { weight: 3 },
                ],
            );

            let outer = view.regions.get(RegionId::TopBar);
            let drawn = bars.top.section_rects(RegionId::TopBar, outer);
            assert!(
                drawn.occupied().count() >= 2,
                "bordered={bordered}: the bar must actually be divided for this to mean anything"
            );

            for (index, rect) in drawn.occupied() {
                for corner in [
                    Position::new(rect.x, rect.y),
                    Position::new(rect.right() - 1, rect.bottom() - 1),
                ] {
                    assert_eq!(
                        view.bar_section_hit_at(view.generation, corner),
                        Some((RegionId::TopBar, index as u8)),
                        "bordered={bordered}: {corner:?} is drawn inside section {index} \
                         at {rect:?} but does not resolve to it"
                    );
                }
            }
        }
    }

    // T28 · a differently divided bar is a different geometry
    #[test]
    fn redividing_a_bar_misses_the_cache_and_advances_the_generation_once() {
        let area = Rect::new(0, 0, 80, 24);
        let (_, first) = sectioned_top_bar_view(
            area,
            false,
            &[
                TrackPolicy::Fixed { cells: 10 },
                TrackPolicy::Fill { weight: 1 },
            ],
        );

        let sections = BarSections::from_policies(
            &[
                TrackPolicy::Fixed { cells: 20 },
                TrackPolicy::Fill { weight: 1 },
            ],
            "top",
        );
        let bars = ShellBars {
            top: BarTrack::of(1).with_sections(sections),
            ..ShellBars::NONE
        };
        let derived = derive_desktop_shell_layout(None, bars);
        let key = ShellGeometryKey::new(area, derived.revision, 0, 0, derived.template, bars);

        // Same area, same edges, same thickness — only the division moved. If
        // that did not reach the key, the cache would return the previous
        // rectangles and every click in the bar would answer for the wrong
        // section while looking perfectly correct.
        let second = compute_shell_view(&derived.layout, key, first.clone(), &|region| {
            u16::from(region == RegionId::LeftPanel) * 26
        });
        assert_eq!(second.generation, first.generation + 1);
        assert_eq!(
            second.bar_section_hit_at(second.generation, Position::new(15, 0)),
            Some((RegionId::TopBar, 0)),
            "the first section now reaches further than it did before"
        );
    }

    // F34-L0 · M2 · a divided bar costs nothing on the steady path
    #[test]
    fn sections_cost_nothing_on_the_retained_path() {
        // The whole isolation argument for bar sections rests on this: they are
        // resolved when the geometry is resolved, and a screen that has not
        // changed resolves nothing at all. If a section ever made the retained
        // path do work, every frame would pay for every section — which is
        // exactly the load the person asked never to be put on herdr.
        let area = Rect::new(0, 0, 120, 40);
        let sections = BarSections::from_policies(
            &[
                TrackPolicy::Fixed { cells: 12 },
                TrackPolicy::Fill { weight: 1 },
                TrackPolicy::Fill { weight: 3 },
            ],
            "top",
        );
        let bars = ShellBars {
            top: BarTrack::bordered(3).with_sections(sections),
            ..ShellBars::NONE
        };
        let derived = derive_desktop_shell_layout(None, bars);
        let key = ShellGeometryKey::new(area, derived.revision, 0, 0, derived.template, bars);
        let resolver = |region: RegionId| u16::from(region == RegionId::LeftPanel) * 26;

        let (views, profile) = crate::render_prof::observe_for_test(|| {
            let first = compute_shell_view(&derived.layout, key, ShellView::default(), &resolver);
            let second = compute_shell_view(&derived.layout, key, first.clone(), &resolver);
            (first, second)
        });

        assert_eq!(profile.counter("shell.geometry_cache.miss"), 1);
        assert_eq!(profile.counter("shell.geometry_cache.hit"), 1);

        let (first, second) = views;
        assert_eq!(
            first.generation, second.generation,
            "an unchanged screen must not advance the generation, or every hit \
             target in the bar is rebuilt for nothing"
        );
        assert_eq!(
            first.hits, second.hits,
            "the retained path must return the same targets it already had"
        );
        assert!(
            second
                .hits
                .iter()
                .any(|hit| matches!(hit.target, ShellHitTarget::BarSection { .. })),
            "the bar has to actually be divided for this measurement to mean anything"
        );
    }

    // F34-L0 · M1 · what dividing a bar costs the geometry solver
    //
    // A measurement, not a gate: a test that asserts a duration is a test that
    // fails when somebody else's build is running, and a flaky gate teaches
    // people to rerun rather than to read. The numbers belong in
    // `.local/prd/f34-l0-budget-measurement.md`; a threshold can only be named
    // after there is something to name it against.
    //
    //   cargo nextest run -E 'test(l0_section_geometry_cost)' --run-ignored all --no-capture
    #[test]
    #[ignore = "measurement, not a gate — see the command above"]
    fn l0_section_geometry_cost() {
        use std::time::Instant;

        const ROUNDS: u32 = 20_000;
        let resolver = |region: RegionId| u16::from(region == RegionId::LeftPanel) * 26;

        println!("\n=== M1 · geometry solve time by section count ===");
        println!(
            "{:>9} {:>14} {:>12} {:>10}",
            "sections", "total(ms)", "solve(us)", "vs 0"
        );

        let mut baseline = 0f64;
        for count in [0usize, 1, 2, 4, 8] {
            let policies: Vec<TrackPolicy> = (0..count)
                .map(|index| {
                    if index == 0 {
                        TrackPolicy::Fixed { cells: 12 }
                    } else {
                        TrackPolicy::Fill {
                            weight: (index as u16) % 3 + 1,
                        }
                    }
                })
                .collect();
            let bars = ShellBars {
                top: BarTrack::bordered(3)
                    .with_sections(BarSections::from_policies(&policies, "top")),
                ..ShellBars::NONE
            };
            let derived = derive_desktop_shell_layout(None, bars);

            // Every round asks a different question, so what is measured is the
            // solve and not the cache: a run that hit the cache would report the
            // cost of comparing two keys and call it the cost of sections.
            let started = Instant::now();
            let mut previous = ShellView::default();
            for round in 0..ROUNDS {
                let width = 180 + u16::try_from(round % 20).unwrap_or(0);
                let key = ShellGeometryKey::new(
                    Rect::new(0, 0, width, 50),
                    derived.revision,
                    0,
                    0,
                    derived.template,
                    bars,
                );
                previous = compute_shell_view(&derived.layout, key, previous, &resolver);
            }
            let elapsed = started.elapsed();
            let per_solve = elapsed.as_nanos() as f64 / f64::from(ROUNDS);
            if count == 0 {
                baseline = per_solve;
            }
            println!(
                "{count:>9} {:>14.1} {:>12.3} {:>9.2}x",
                elapsed.as_secs_f64() * 1000.0,
                per_solve / 1000.0,
                per_solve / baseline
            );
            assert!(previous.generation > 0, "the solver has to have run");
        }
        println!(
            "\nnote: sections draw nothing yet, so this is the FLOOR. Re-measure \
             once a widget catalogue puts content in them."
        );
    }

    // T26 · an undivided bar produces no section targets at all
    #[test]
    fn an_undivided_bar_contributes_no_section_hits() {
        let area = Rect::new(0, 0, 80, 24);
        let bars = ShellBars {
            top: BarTrack::of(1),
            ..ShellBars::NONE
        };
        let derived = derive_desktop_shell_layout(None, bars);
        let key = ShellGeometryKey::new(area, derived.revision, 0, 0, derived.template, bars);
        let view = compute_shell_view(&derived.layout, key, ShellView::default(), &|region| {
            u16::from(region == RegionId::LeftPanel) * 26
        });

        assert_eq!(
            view.bar_section_hit_at(view.generation, Position::new(5, 0)),
            None
        );
        assert_eq!(
            view.region_hit_at(view.generation, Position::new(5, 0)),
            Some(RegionId::TopBar),
            "the bar itself is still a target"
        );
    }

    #[test]
    fn geometry_cache_profile_counts_desktop_and_empty_hits_and_misses() {
        let layout = ShellLayout::default();
        let desktop_key =
            ShellGeometryKey::new(Rect::new(0, 0, 120, 40), 1, 2, 3, None, ShellBars::NONE);
        let mobile_key =
            ShellGeometryKey::new(Rect::new(0, 0, 40, 20), 4, 5, 6, None, ShellBars::NONE);

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
        let legacy = ShellGeometryKey::new(area, 1, 2, 3, None, ShellBars::NONE);
        let dock = ShellGeometryKey::new(
            area,
            1,
            2,
            3,
            Some(ShellTemplateId::DockStage),
            ShellBars::NONE,
        );
        let desktop = ShellGeometryKey::new(
            area,
            1,
            2,
            3,
            Some(ShellTemplateId::DesktopWorkspace),
            ShellBars::NONE,
        );

        assert_ne!(legacy, dock);
        assert_ne!(dock, desktop);
        assert_eq!(
            dock,
            ShellGeometryKey::new(
                area,
                1,
                2,
                3,
                Some(ShellTemplateId::DockStage),
                ShellBars::NONE
            )
        );
    }

    // CLA3 again, one level down: the revision only names WHICH edges are on,
    // so two bars of different thickness share it. The key carries the exact
    // composition for precisely this case — drop that field and a one-row top
    // bar would hand back a three-row bar's rectangles.
    #[test]
    fn two_bars_of_different_thickness_are_two_different_authorities() {
        let area = Rect::new(0, 0, 120, 40);
        let thin = ShellBars {
            top: super::super::source::BarTrack::of(1),
            ..ShellBars::NONE
        };
        let thick = ShellBars {
            top: super::super::source::BarTrack::of(3),
            ..ShellBars::NONE
        };

        let thin_derived = super::super::derive_desktop_shell_layout(None, thin);
        let thick_derived = super::super::derive_desktop_shell_layout(None, thick);
        assert_eq!(
            thin_derived.revision, thick_derived.revision,
            "this test is only interesting while the revisions agree"
        );
        assert_ne!(
            thin_derived.layout, thick_derived.layout,
            "and while the trees do not"
        );

        assert_ne!(
            ShellGeometryKey::new(area, thin_derived.revision, 2, 3, None, thin),
            ShellGeometryKey::new(area, thick_derived.revision, 2, 3, None, thick),
        );
    }

    // T2 + T4 · a changed authority is a new generation, exactly once.
    #[test]
    fn a_template_change_misses_the_cache_and_advances_the_generation_once() {
        let layout = ShellLayout::default();
        let area = Rect::new(0, 0, 120, 40);
        let dock = ShellGeometryKey::new(
            area,
            1,
            2,
            3,
            Some(ShellTemplateId::DockStage),
            ShellBars::NONE,
        );
        let desktop = ShellGeometryKey::new(
            area,
            1,
            2,
            3,
            Some(ShellTemplateId::DesktopWorkspace),
            ShellBars::NONE,
        );

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
            ShellGeometryKey::new(area, 1, 2, 3, None, ShellBars::NONE),
            ShellGeometryKey::new(area, 1, 2, 3, None, ShellBars::NONE)
        );
        assert_eq!(
            ShellGeometryKey::default(),
            ShellGeometryKey::new(Rect::ZERO, 0, 0, 0, None, ShellBars::NONE)
        );
    }
}
