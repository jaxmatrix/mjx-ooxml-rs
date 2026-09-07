//! What each of the 186 presets *draws*, step for step, at six sizes and at every adjustment's own
//! extremes — the check that needs no reference and catches a transcription slip immediately.
//!
//! # The claim, and why it is stronger than a count
//!
//! MJXOFF-201 §6 asks for *"command counts matching the definition"*. A count is the weak form: a
//! `rect` that emitted four `LineTo`s in the wrong order has the right count, and so does a shape
//! whose `a:arcTo` silently became a straight line while a neighbouring `a:lnTo` became an arc.
//! What is asserted here is the **correspondence**: the resolved command list is the table's step
//! list, in order, with exactly three transformations applied and every one of them accounted for.
//!
//! 1. an `a:arcTo` becomes one or more `CubicTo`s — or none, when the arc has no extent;
//! 2. a step following an `a:close` with no intervening `a:moveTo` gains a `MoveTo` in front of it,
//!    because a [`PathCommand`](mjx_scene::PathCommand) list cannot say *"reopen the contour I just
//!    closed"* and `mjx-scene` drops a step that arrives before any `MoveTo`; and
//! 3. a second `a:close` on an already-closed contour emits nothing.
//!
//! Nothing else may differ, and the walk names the shape, the path and the step index where it does.
//! **The arc case cannot swallow a cubic the table asked for, because no path in ECMA-376's whole
//! geometry file contains both an `a:arcTo` and an `a:cubicBezTo`** — 393 arcs and 28 cubics across
//! 319 paths, and not one path with one of each. [`no_path_mixes_an_arc_with_a_cubic`] asserts that
//! premise rather than relying on it, because it is a fact about the file and the file could change.
//!
//! What the walk cannot do, said here rather than discovered later: **consecutive arcs are matched
//! as a run.** Eleven `a:arcTo`s in a row produce a run of `CubicTo`s and neither list says where
//! one arc's cubics stop, so the claim over a run of *k* is that it became between one and five·*k*
//! cubics and nothing else. `cloud` is the shape that settles it — eleven arcs, twenty-two cubics —
//! and an arc-at-a-time walk credits all twenty-two to the first arc and reports a defect that is
//! not there. The per-arc bound is still asserted, on every arc that stands alone between two other
//! kinds of step, and there are enough of those for the bound to be a measurement.
//!
//! # And the second claim: the map onto the box is affine
//!
//! [`the_same_shape_in_two_boxes_differs_by_exactly_the_affine_map_between_them`] resolves every
//! preset in two device boxes and asserts every command maps by the exact scale-and-translate
//! between them. A resolver that dropped the box's offset, applied a fixed scale, or clamped a
//! coordinate would still produce a plausible shape in each box separately, and the two would
//! disagree here.
//!
//! # Where the sizes come from
//!
//! Six, chosen so that no single confusion survives all of them: landscape and portrait with the
//! same `ss` (so `ss` cannot pass for `h`), a square (so `w` cannot pass for `h`), one an order of
//! magnitude larger and one two orders smaller (so a fixed scale cannot pass for a proportion), and
//! one whose extents and whose device box have *different* aspect ratios (so the shape's own space
//! cannot pass for the page's).

mod common;

use std::collections::{BTreeMap, BTreeSet};

use common::{box_on_the_page, extents_of_the_box, portrait_box_on_the_page, portrait_extents};
use mjx_geometry::{
    adjustment_domains, contours_of_definition, preset_contours, seeded_shapes, AdjustmentOverride,
    GeometryError, PathFillMode, PresetAngle, PresetCoordinate, PresetPath, PresetPathStep,
    PresetPoint, PresetShapeDefinition, PresetShapeType, Size,
};
use mjx_scene::{PathCommand, SceneRect};

// -------------------------------------------------------------------------------------------
// The numbers
// -------------------------------------------------------------------------------------------

/// How many `a:path` elements the whole table holds.
const PATHS_IN_THE_TABLE: usize = 319;

/// How many `a:arcTo` steps it holds, and how many `a:cubicBezTo` and `a:quadBezTo`.
///
/// The arc figure is what makes the correspondence walk worth writing: 393 of the 2 907 drawing
/// steps expand into a variable number of commands, and every other kind is one for one.
const ARCS_IN_THE_TABLE: usize = 393;
/// As [`ARCS_IN_THE_TABLE`].
const CUBICS_IN_THE_TABLE: usize = 28;
/// As [`ARCS_IN_THE_TABLE`].
const QUADRATICS_IN_THE_TABLE: usize = 33;

/// How many paths the file leaves without an `a:close`.
///
/// Seventy of the 319, and that is what a stroked, unfilled outline is: `straightConnector1` is a
/// line segment and closing it would draw it twice. Asserted so that a `a:close` appearing or
/// disappearing fails here.
const PATHS_THE_FILE_LEAVES_OPEN: usize = 70;

/// The presets with a drawing step after an `a:close` and no `a:moveTo` between them.
///
/// Six, all of them accent callouts, and every one is the case `emit_path`'s reopen rule exists for:
/// the shape draws its body as a closed contour and then its accent bar from the same corner. A
/// renderer that dropped the rule would lose the bar entirely, because `mjx-scene` drops a step that
/// arrives before any `MoveTo`.
const REOPENS_A_CLOSED_CONTOUR: &[&str] = &[
    "accentBorderCallout1",
    "accentBorderCallout2",
    "accentBorderCallout3",
    "accentCallout1",
    "accentCallout2",
    "accentCallout3",
];

/// The presets the correspondence walk cannot reach at some size and some adjustment stop, because
/// one of their own guides has no finite value there.
///
/// Four, and every one divides by a quantity that reaches zero, or takes the root of a quantity
/// that goes negative, at one of its own stops. They are **named as unverified at those points**
/// rather than quietly skipped: each is walked successfully at every other size and every other
/// stop, and every one of the four is also in `an_adjustment_moves_the_shape.rs`'s
/// `SINGULAR_SOMEWHERE`.
///
/// The list is not the same as `every_adjustment_moves_its_shape.rs`'s
/// `SINGULAR_AT_A_DOMAIN_ENDPOINT`, which has the three circular arrows and not `noSmoking` —
/// because that suite sweeps one adjustment at a time in two orientations, and this one crosses
/// every stop with six sizes. `noSmoking`'s `sqrt` of a negative is reached at a stop only at some
/// of them. **Three lists of singular shapes were already three different lists** (paths, text
/// rectangle, connection sites); this is a fourth reading of the first, and the difference is the
/// sizes rather than the consumer.
const SINGULAR_AT_SOME_SIZE_AND_STOP: &[&str] = &[
    "circularArrow",
    "leftCircularArrow",
    "leftRightCircularArrow",
    "noSmoking",
];

/// The most cubics any single `a:arcTo` of the table decomposes into, at any size and any
/// adjustment this suite visits.
///
/// **Four, and four is a whole turn** — `MAXIMUM_ARC_SEGMENT_RADIANS` is a quarter turn, so this
/// says no arc in ECMA-376's geometry file ever sweeps past 360°, at any adjustment either of its
/// stops allows. Asserted as an **equality** rather than as a ceiling, because a bound nothing
/// touches is a bound that measures nothing; and asserted at all because an arc decomposing into
/// dozens of segments is what a swing computed in the wrong unit produces, and it is otherwise
/// invisible — the shape still closes and still fills its box.
///
/// [`every_resolved_command_is_a_step_the_table_asks_for`] **prints which presets reach it**
/// rather than this comment claiming to know, because a shape named in a doc comment and a shape
/// the data actually contains are two different things and only one of them stays true.
const MOST_CUBICS_ONE_ARC_BECOMES: usize = 4;

/// How far, in device pixels, two resolutions of one shape may disagree after the affine map
/// between their boxes is applied.
///
/// The same unit and the same two error sources as every other tolerance in the crate — half an EMU
/// of coordinate rounding and one narrowing to `f32` — but **applied twice and then scaled**, which
/// is why it is not the `0.01` the rest of the crate uses. The far box is eight times the near one,
/// so the near box's own `f32` error is multiplied by eight before the two are compared. The worst
/// disagreement measured over 15 096 points is `6.1e-5` px, and this is sixteen times that — tight
/// enough that the headroom assertion below (`worst ≤ tolerance / 2`) is a real constraint rather
/// than a formality, and [`the_affine_tolerance_is_not_wide_enough_to_pass_a_dropped_offset`] shows
/// the smallest defect it still catches: an offset dropped by one device pixel measures `1.0`, a
/// thousand times this number.
const AFFINE_TOLERANCE_PIXELS: f32 = 0.001;

/// The six (extents, box) pairs every structural claim below is made at.
///
/// See the module documentation for why each is here. The labels are what a failure prints.
fn sizes() -> [(&'static str, Size, SceneRect); 6] {
    [
        ("landscape", extents_of_the_box(), box_on_the_page()),
        ("portrait", portrait_extents(), portrait_box_on_the_page()),
        (
            "square",
            Size::from_emu(120 * 12_700, 120 * 12_700),
            SceneRect::new(5.0, 7.0, 125.0, 127.0),
        ),
        (
            "a metre wide",
            Size::from_emu(4_000 * 12_700, 3_000 * 12_700),
            SceneRect::new(-100.0, -50.0, 3_900.0, 2_950.0),
        ),
        (
            "a shape two points across",
            Size::from_emu(2 * 12_700, 12_700),
            SceneRect::new(0.25, 0.5, 2.25, 1.5),
        ),
        (
            "extents and box of different aspect",
            Size::from_emu(200 * 12_700, 40 * 12_700),
            SceneRect::new(11.0, 13.0, 51.0, 213.0),
        ),
    ]
}

/// Every adjustment case a shape is walked at: its defaults, and each adjustment at each end of its
/// own domain with the rest left alone.
fn adjustment_cases(
    preset: PresetShapeType,
    extents: Size,
) -> Vec<(String, Vec<AdjustmentOverride>)> {
    let mut cases = vec![("its defaults".to_owned(), Vec::new())];
    let Ok(domains) = adjustment_domains(preset, extents, &[]) else {
        return cases;
    };
    for domain in domains {
        for (label, value) in [("minimum", domain.minimum), ("maximum", domain.maximum)] {
            cases.push((
                format!("`{}` at its {label} ({value})", domain.spec.wire_name),
                vec![AdjustmentOverride::new(domain.spec.wire_name, value)],
            ));
        }
    }
    cases
}

// -------------------------------------------------------------------------------------------
// The premise the correspondence walk rests on
// -------------------------------------------------------------------------------------------

#[test]
fn no_path_mixes_an_arc_with_a_cubic() {
    let mut paths = 0usize;
    let (mut arcs, mut cubics, mut quadratics, mut open) = (0usize, 0usize, 0usize, 0usize);
    let mut mixed: BTreeSet<&'static str> = BTreeSet::new();
    for definition in seeded_shapes() {
        for path in definition.paths {
            paths += 1;
            let counted = |wanted: fn(&PresetPathStep) -> bool| {
                path.steps.iter().filter(|step| wanted(step)).count()
            };
            let arc = counted(|step| matches!(step, PresetPathStep::ArcTo { .. }));
            let cubic = counted(|step| matches!(step, PresetPathStep::CubicBezierTo { .. }));
            arcs += arc;
            cubics += cubic;
            quadratics += counted(|step| matches!(step, PresetPathStep::QuadBezierTo { .. }));
            if arc > 0 && cubic > 0 {
                mixed.insert(definition.preset.to_wire());
            }
            if !matches!(path.steps.last(), Some(PresetPathStep::Close)) {
                open += 1;
            }
        }
    }
    assert!(
        mixed.is_empty(),
        "these presets have a path with both an `a:arcTo` and an `a:cubicBezTo`, so the \
         correspondence walk can no longer tell an arc's cubics from the table's own: {mixed:?}"
    );
    assert_eq!(
        (paths, arcs, cubics, quadratics, open),
        (
            PATHS_IN_THE_TABLE,
            ARCS_IN_THE_TABLE,
            CUBICS_IN_THE_TABLE,
            QUADRATICS_IN_THE_TABLE,
            PATHS_THE_FILE_LEAVES_OPEN
        ),
        "the table's own step census has changed"
    );
}

// -------------------------------------------------------------------------------------------
// 1 · The resolved commands are the table's steps
// -------------------------------------------------------------------------------------------

/// What one path's walk produced, or where it stopped agreeing.
struct Walk {
    /// The most cubics any arc that stands *alone* between two other kinds of step became.
    widest_single_arc: usize,
    /// How many such arcs there were.
    single_arcs: usize,
    /// The longest run of consecutive `a:arcTo`s in this path.
    widest_run: usize,
    /// How many `MoveTo`s the reopen rule inserted.
    reopens: usize,
    /// How many `a:close`es were suppressed as a second close on a closed contour.
    suppressed_closes: usize,
}

/// Walk one path's steps against the commands they resolved to, naming the first disagreement.
///
/// `Err` rather than a panic, so that [`the_correspondence_walk_is_able_to_fail`] can run the whole
/// table with one shape's arc written in the wrong unit and show that exactly one shape reports.
/// Every ordinary caller unwraps it immediately, which reads the same as an assertion.
fn walk(
    shape: &str,
    size: &str,
    case: &str,
    index: usize,
    path: &PresetPath,
    commands: &[PathCommand],
) -> Result<Walk, String> {
    let mut at = 0usize;
    let mut open = false;
    let mut ever_opened = false;
    let mut result = Walk {
        widest_single_arc: 0,
        single_arcs: 0,
        widest_run: 0,
        reopens: 0,
        suppressed_closes: 0,
    };
    // How many further steps of the current run of arcs the run has already accounted for.
    let mut consumed_by_a_run = 0usize;
    let where_ = |step: usize, at: usize| {
        format!("`{shape}` at {size} with {case}, path {index}, step {step} (command {at})")
    };

    for (step_index, step) in path.steps.iter().enumerate() {
        // The reopen rule, mirrored: a step that is not a `MoveTo` and arrives on a closed contour
        // is preceded by a restated start point, and a second `Close` draws nothing at all.
        if !open && ever_opened && !matches!(step, PresetPathStep::MoveTo(_)) {
            if matches!(step, PresetPathStep::Close) {
                result.suppressed_closes += 1;
                continue;
            }
            if !matches!(commands.get(at), Some(PathCommand::MoveTo(_))) {
                return Err(format!(
                    "{}: the reopen rule should have restated the start point and the command is \
                     {:?}",
                    where_(step_index, at),
                    commands.get(at)
                ));
            }
            at += 1;
            result.reopens += 1;
            open = true;
        }
        match step {
            PresetPathStep::Close => {
                if !matches!(commands.get(at), Some(PathCommand::Close)) {
                    return Err(format!(
                        "{}: an `a:close` resolved to {:?}",
                        where_(step_index, at),
                        commands.get(at)
                    ));
                }
                at += 1;
                open = false;
            }
            PresetPathStep::MoveTo(_) => {
                if !matches!(commands.get(at), Some(PathCommand::MoveTo(_))) {
                    return Err(format!(
                        "{}: an `a:moveTo` resolved to {:?}",
                        where_(step_index, at),
                        commands.get(at)
                    ));
                }
                at += 1;
                open = true;
                ever_opened = true;
            }
            PresetPathStep::LineTo(_) => {
                if !matches!(commands.get(at), Some(PathCommand::LineTo(_))) {
                    return Err(format!(
                        "{}: an `a:lnTo` resolved to {:?}",
                        where_(step_index, at),
                        commands.get(at)
                    ));
                }
                at += 1;
            }
            PresetPathStep::QuadBezierTo { .. } => {
                if !matches!(commands.get(at), Some(PathCommand::QuadraticTo { .. })) {
                    return Err(format!(
                        "{}: an `a:quadBezTo` resolved to {:?}",
                        where_(step_index, at),
                        commands.get(at)
                    ));
                }
                at += 1;
            }
            PresetPathStep::CubicBezierTo { .. } => {
                if !matches!(commands.get(at), Some(PathCommand::CubicTo { .. })) {
                    return Err(format!(
                        "{}: an `a:cubicBezTo` resolved to {:?}",
                        where_(step_index, at),
                        commands.get(at)
                    ));
                }
                at += 1;
            }
            PresetPathStep::ArcTo { .. } => {
                if consumed_by_a_run > 0 {
                    // Already accounted for by the run this arc is inside; see below.
                    consumed_by_a_run -= 1;
                    continue;
                }
                // **A run of consecutive arcs is matched as a run**, because consecutive `a:arcTo`s
                // produce consecutive `CubicTo`s and nothing in either list says where one arc's
                // cubics end and the next one's begin. `cloud` is the shape that settles it: eleven
                // arcs in a row and twenty-two cubics, which an arc-at-a-time walk would credit
                // entirely to the first. So the claim made here is the one that is actually
                // available — *k* arcs become between one and `MOST_CUBICS_ONE_ARC_BECOMES · k`
                // cubics and no other command — and the per-arc bound is asserted separately, on
                // the 158 runs of length one, by [`ARC_RUNS_OF_ONE`]'s own counter.
                let arcs_in_the_run = path.steps[step_index..]
                    .iter()
                    .take_while(|step| matches!(step, PresetPathStep::ArcTo { .. }))
                    .count();
                consumed_by_a_run = arcs_in_the_run - 1;
                let start = at;
                while matches!(commands.get(at), Some(PathCommand::CubicTo { .. })) {
                    at += 1;
                }
                let cubics = at - start;
                if arcs_in_the_run == 1 {
                    result.widest_single_arc = result.widest_single_arc.max(cubics);
                    result.single_arcs += 1;
                }
                result.widest_run = result.widest_run.max(arcs_in_the_run);
                if cubics > MOST_CUBICS_ONE_ARC_BECOMES * arcs_in_the_run {
                    return Err(format!(
                        "{}: {arcs_in_the_run} consecutive `a:arcTo`s became {cubics} cubics, past \
                         the {MOST_CUBICS_ONE_ARC_BECOMES} per arc a swing of a full turn needs — \
                         which is what an angle read in the wrong unit produces",
                        where_(step_index, start),
                    ));
                }
            }
        }
    }
    if at != commands.len() {
        return Err(format!(
            "`{shape}` at {size} with {case}, path {index}: the table's steps account for {at} of \
             {} resolved commands, so the resolver emitted something the table does not ask for",
            commands.len()
        ));
    }
    Ok(result)
}

#[test]
fn every_resolved_command_is_a_step_the_table_asks_for() {
    let mut walked = 0usize;
    let mut widest_single_arc = 0usize;
    let mut widest_single_arc_shapes: BTreeSet<&'static str> = BTreeSet::new();
    let mut single_arcs = 0usize;
    let mut widest_run = 0usize;
    let mut reopened: BTreeSet<&'static str> = BTreeSet::new();
    let mut suppressed = 0usize;
    let mut singular: BTreeSet<&'static str> = BTreeSet::new();

    for (size, extents, within) in sizes() {
        for definition in seeded_shapes() {
            let preset = definition.preset;
            let shape = preset.to_wire();
            for (case, adjustments) in adjustment_cases(preset, extents) {
                let contours = match preset_contours(preset, extents, &adjustments, within) {
                    Ok(contours) => contours,
                    Err(GeometryError::SingularGeometry { .. }) => {
                        singular.insert(shape);
                        continue;
                    }
                    Err(error) => panic!("`{shape}` at {size} with {case} failed with {error}"),
                };
                assert_eq!(
                    contours.len(),
                    definition.paths.len(),
                    "`{shape}` at {size} with {case} resolved {} contours for {} `a:path`s",
                    contours.len(),
                    definition.paths.len()
                );
                for (index, (path, contour)) in
                    definition.paths.iter().zip(contours.iter()).enumerate()
                {
                    let result = walk(shape, size, &case, index, path, &contour.commands)
                        .unwrap_or_else(|why| panic!("{why}"));
                    walked += 1;
                    if result.widest_single_arc >= widest_single_arc {
                        if result.widest_single_arc > widest_single_arc {
                            widest_single_arc_shapes.clear();
                        }
                        widest_single_arc = result.widest_single_arc;
                        widest_single_arc_shapes.insert(shape);
                    }
                    single_arcs += result.single_arcs;
                    widest_run = widest_run.max(result.widest_run);
                    suppressed += result.suppressed_closes;
                    if result.reopens > 0 {
                        reopened.insert(shape);
                    }
                    // The flags the table carries are the flags the contour carries, per path —
                    // which is what makes `flowChartMultidocument`'s undrawn third path a fact the
                    // seam can act on rather than a field nobody reads.
                    assert_eq!(contour.fill, path.fill);
                    assert_eq!(contour.stroke, path.stroke);
                    assert_eq!(contour.extrusion_ok, path.extrusion_ok);
                }
            }
        }
    }

    // A walk that walked nothing would satisfy every assertion inside it.
    assert!(
        walked > 7_000,
        "the correspondence walk covered only {walked} (shape, size, adjustment, path) cases"
    );
    // The per-arc bound, measured on the arcs that stand alone — where the correspondence *is*
    // one to one — so the number is a measurement rather than a ceiling nothing reaches.
    assert!(
        single_arcs > 1_000,
        "only {single_arcs} arcs stood alone, so the per-arc bound was barely exercised"
    );
    assert_eq!(
        widest_single_arc, MOST_CUBICS_ONE_ARC_BECOMES,
        "no arc standing alone reached {MOST_CUBICS_ONE_ARC_BECOMES} cubics, so the bound is above \
         anything that happens and measures nothing"
    );
    assert!(
        widest_run > 1,
        "no path has two consecutive arcs, so the run-matching branch is never taken"
    );
    assert_eq!(
        reopened,
        REOPENS_A_CLOSED_CONTOUR
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        "the presets that draw a step after an `a:close` have changed"
    );
    // The presets the walk could not reach at some size and some stop, named rather than counted:
    // each is a point ECMA-376's own arithmetic has no value at, and each is already named in
    // `every_adjustment_moves_its_shape.rs` or in `an_adjustment_moves_the_shape.rs`. Naming them
    // here too is the difference between an exclusion a reader can check and a count nobody reads.
    assert_eq!(
        singular,
        SINGULAR_AT_SOME_SIZE_AND_STOP
            .iter()
            .copied()
            .collect::<BTreeSet<_>>(),
        "the presets the structural walk cannot reach at some size and some adjustment stop have \
         changed"
    );
    assert_eq!(
        suppressed, 0,
        "the table now has a second `a:close` on an already-closed contour, which this walk \
         accounts for but no shape in ECMA-376's file has had"
    );
    println!(
        "{walked} (shape, size, adjustment, path) walks; every resolved command is a step the \
         table asks for. {single_arcs} arcs stood alone and the widest became \
         {widest_single_arc} cubics ({widest_single_arc_shapes:?}); the longest run of \
         consecutive arcs is {widest_run}. \
         {} presets reopen a closed contour, {} are singular somewhere",
        reopened.len(),
        singular.len()
    );
}

#[test]
fn a_contour_the_table_closes_is_closed_and_one_it_leaves_open_is_open() {
    // The other half of the structure: not only *which* commands, but whether the contour they
    // form is closed. Stated per path and per size rather than per shape, because 70 of the 319
    // paths are deliberately open — a `fill="none"` stroke — and a suite that asserted "every shape
    // closes" would have to except a third of the file.
    let mut open_paths = 0usize;
    for (size, extents, within) in sizes() {
        for definition in seeded_shapes() {
            let shape = definition.preset.to_wire();
            let Ok(contours) = preset_contours(definition.preset, extents, &[], within) else {
                continue;
            };
            for (index, (path, contour)) in definition.paths.iter().zip(contours.iter()).enumerate()
            {
                let closes_in_the_table = matches!(path.steps.last(), Some(PresetPathStep::Close));
                let closes_when_resolved =
                    matches!(contour.commands.last(), Some(PathCommand::Close));
                assert_eq!(
                    closes_in_the_table,
                    closes_when_resolved,
                    "`{shape}` at {size}, path {index}: the table {} and the resolved contour {}",
                    if closes_in_the_table {
                        "closes"
                    } else {
                        "does not close"
                    },
                    if closes_when_resolved {
                        "does"
                    } else {
                        "does not"
                    }
                );
                if size == "landscape" && !closes_in_the_table {
                    open_paths += 1;
                }
                // And no contour draws before its first `MoveTo`, at any size. **Not** "no contour
                // opens a subpath while one is still open": one `a:path` may hold several
                // subpaths, and `actionButtonHome`'s glyph is three of them with no `a:close`
                // between — an open stroke, which is what an unfilled path is for. What is never
                // legal is a step with no start point, because `mjx-scene` drops it and the shape
                // silently loses an edge.
                assert!(
                    matches!(
                        contour.commands.first(),
                        Some(PathCommand::MoveTo(_)) | None
                    ),
                    "`{shape}` at {size}, path {index} begins with {:?} rather than a `MoveTo`, \
                     and `mjx-scene` drops a step that arrives before any start point",
                    contour.commands.first()
                );
            }
        }
    }
    assert_eq!(open_paths, PATHS_THE_FILE_LEAVES_OPEN);
}

// -------------------------------------------------------------------------------------------
// 2 · The map onto the box is affine
// -------------------------------------------------------------------------------------------

/// Every point of a command list, in order, so two resolutions can be compared point for point.
fn coordinates(commands: &[PathCommand]) -> Vec<(f32, f32)> {
    common::points_of(commands)
        .into_iter()
        .map(|point| (point.x, point.y))
        .collect()
}

#[test]
fn the_same_shape_in_two_boxes_differs_by_exactly_the_affine_map_between_them() {
    // One shape, one set of extents, two device boxes — and the second is the first scaled by eight
    // and moved somewhere with a negative corner, so a resolver that dropped the offset, used a
    // fixed scale, took the absolute value of a coordinate or clamped one to the box would produce
    // a perfectly plausible shape in each and disagree here.
    let extents = extents_of_the_box();
    let near = SceneRect::new(37.0, 11.0, 197.0, 131.0);
    let far = SceneRect::new(-300.0, -80.0, 980.0, 880.0);
    let (scale_x, scale_y) = (far.width() / near.width(), far.height() / near.height());

    let mut worst = 0.0f32;
    let mut compared = 0usize;
    for definition in seeded_shapes() {
        let preset = definition.preset;
        let shape = preset.to_wire();
        for (case, adjustments) in adjustment_cases(preset, extents) {
            let (Ok(here), Ok(there)) = (
                preset_contours(preset, extents, &adjustments, near),
                preset_contours(preset, extents, &adjustments, far),
            ) else {
                continue;
            };
            assert_eq!(here.len(), there.len());
            for (index, (left, right)) in here.iter().zip(there.iter()).enumerate() {
                let (from, to) = (coordinates(&left.commands), coordinates(&right.commands));
                assert_eq!(
                    from.len(),
                    to.len(),
                    "`{shape}` with {case}, path {index} drew {} points in one box and {} in the \
                     other",
                    from.len(),
                    to.len()
                );
                for ((x, y), (mapped_x, mapped_y)) in from.iter().zip(to.iter()) {
                    let expected_x = far.left + (x - near.left) * scale_x;
                    let expected_y = far.top + (y - near.top) * scale_y;
                    let apart = (expected_x - mapped_x)
                        .abs()
                        .max((expected_y - mapped_y).abs());
                    assert!(
                        apart <= AFFINE_TOLERANCE_PIXELS,
                        "`{shape}` with {case}, path {index}: ({x}, {y}) in the near box maps to \
                         ({expected_x}, {expected_y}) and resolved to ({mapped_x}, {mapped_y}), \
                         {apart} px apart"
                    );
                    worst = worst.max(apart);
                    compared += 1;
                }
            }
        }
    }
    assert!(compared > 10_000, "only {compared} points were compared");
    println!("{compared} points compared across two boxes; worst disagreement {worst} px");
    // The headroom, so the tolerance is a bound and not a curtain.
    assert!(
        worst <= AFFINE_TOLERANCE_PIXELS / 2.0,
        "the worst affine disagreement is {worst} px, more than half the tolerance — the tolerance \
         is doing the geometry's work"
    );
}

#[test]
fn the_affine_tolerance_is_not_wide_enough_to_pass_a_dropped_offset() {
    // The tolerance shown against the defect it is meant to catch. A box moved by a single device
    // pixel — the smallest offset error there is — measures 1.0 px, which is twenty times
    // [`AFFINE_TOLERANCE_PIXELS`] and two thousand times the worst honest disagreement above.
    let extents = extents_of_the_box();
    let here = SceneRect::new(37.0, 11.0, 197.0, 131.0);
    let moved = SceneRect::new(38.0, 11.0, 198.0, 131.0);
    let left =
        preset_contours(PresetShapeType::Rectangle, extents, &[], here).expect("`rect` resolves");
    let right =
        preset_contours(PresetShapeType::Rectangle, extents, &[], moved).expect("`rect` resolves");
    let apart = coordinates(&left[0].commands)
        .into_iter()
        .zip(coordinates(&right[0].commands))
        .map(|((x, _), (moved_x, _))| (moved_x - x).abs())
        .fold(0.0f32, f32::max);
    assert!(
        (apart - 1.0).abs() < 1e-5,
        "a box moved one pixel moved its shape {apart} px"
    );
    assert!(
        apart > AFFINE_TOLERANCE_PIXELS * 100.0,
        "a one-pixel offset error measures {apart}, which {AFFINE_TOLERANCE_PIXELS} would pass"
    );
}

// -------------------------------------------------------------------------------------------
// 3 · The census that says the walk saw every shape
// -------------------------------------------------------------------------------------------

#[test]
fn the_walk_covers_every_preset_and_names_the_ones_it_cannot() {
    // The gate MJXOFF-205 states as *"a shape that cannot be verified is named as unverified"*.
    // Every preset must be walked at its defaults at every size, and the ones that cannot be are
    // named with the reason — which, at default adjustments, is none of them.
    let mut walked: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut unwalkable: BTreeMap<&'static str, String> = BTreeMap::new();
    for (size, extents, within) in sizes() {
        for definition in seeded_shapes() {
            let shape = definition.preset.to_wire();
            match preset_contours(definition.preset, extents, &[], within) {
                Ok(contours) => {
                    for (index, (path, contour)) in
                        definition.paths.iter().zip(contours.iter()).enumerate()
                    {
                        walk(shape, size, "its defaults", index, path, &contour.commands)
                            .unwrap_or_else(|why| panic!("{why}"));
                    }
                    *walked.entry(shape).or_default() += 1;
                }
                Err(error) => {
                    unwalkable.insert(shape, format!("{size}: {error}"));
                }
            }
        }
    }
    assert!(
        unwalkable.is_empty(),
        "these presets could not be walked at their default adjustments: {unwalkable:#?}"
    );
    assert_eq!(
        walked.len(),
        186,
        "the walk covered {} presets",
        walked.len()
    );
    assert!(
        walked.values().all(|count| *count == sizes().len()),
        "some preset was walked at fewer than all six sizes"
    );
}

// -------------------------------------------------------------------------------------------
// The check shown able to fail
// -------------------------------------------------------------------------------------------

/// One `pie` whose wedge angle is written in **degrees** rather than in the wire's 60000ths of one.
///
/// The mutation MJXOFF-205 asks the structural check to be proved against, and it is a mutation of
/// the *table* rather than of the resolver, which matters and is worth saying plainly: the
/// correspondence walk reads the table on both sides, so a table edit that is merely *different* is
/// self-consistent and cannot fail it. What can is a table edit that makes the resolver produce a
/// different **number** of commands than the step it came from allows — and an angle whose unit is
/// wrong is exactly that. `pie`'s swing is `16 200 000` sixty-thousandths of a degree, which is
/// 270°; read as `16 200 000` *degrees* it is forty-five thousand turns, and the arc decomposes
/// into the thousands of cubics `arc_to_cubics` caps itself at.
///
/// This is not a contrived defect. Every angle in `presetShapeDefinitions.xml` is in the wire scale
/// and every one of them looks like a plain integer; `mjx-dml`'s
/// [`AdjustAngle::Angle`](mjx_dml::geometry::AdjustAngle::Angle) takes degrees. The conversion is
/// one multiplication in `PresetAngle::to_adjust_angle` and nothing else in the crate would notice
/// it going missing: the shape still closes and its bounding box is unchanged, because the arc
/// simply goes round and round.
static A_PIE_WHOSE_SWING_IS_IN_THE_WRONG_UNIT: &[PresetPathStep] = &[
    PresetPathStep::MoveTo(PresetPoint::at("x1", "y1")),
    PresetPathStep::ArcTo {
        width_radius: PresetCoordinate::Guide("wd2"),
        height_radius: PresetCoordinate::Guide("hd2"),
        start_angle: PresetAngle::Guide("stAng"),
        // `swAng` in the wire scale is 60 000 times this. The mutation is the missing factor.
        swing_angle: PresetAngle::Native(16_200_000 * 60_000),
    },
    PresetPathStep::LineTo(PresetPoint::at("hc", "vc")),
    PresetPathStep::Close,
];

/// [`A_PIE_WHOSE_SWING_IS_IN_THE_WRONG_UNIT`] as the one-path list a definition holds.
static A_PIE_WITH_ITS_SWING_IN_THE_WRONG_UNIT: &[PresetPath] = &[PresetPath {
    width: None,
    height: None,
    fill: PathFillMode::Normal,
    stroke: true,
    extrusion_ok: true,
    steps: A_PIE_WHOSE_SWING_IS_IN_THE_WRONG_UNIT,
}];

#[test]
fn the_correspondence_walk_is_able_to_fail_on_one_shape_and_only_that_shape() {
    let (size, extents, within) = sizes()[0];
    let real = seeded_shapes()
        .iter()
        .find(|definition| definition.preset == PresetShapeType::Pie)
        .expect("the table has `pie`");
    let mutant = PresetShapeDefinition {
        paths: A_PIE_WITH_ITS_SWING_IN_THE_WRONG_UNIT,
        ..*real
    };

    // The whole table walked with the mutant standing in for `pie`. Exactly one shape must report,
    // and it must be that one: a check that reddens everything is as useless as one that reddens
    // nothing.
    let mut reported: BTreeMap<&'static str, String> = BTreeMap::new();
    for definition in seeded_shapes() {
        let shape = definition.preset.to_wire();
        let definition = if definition.preset == PresetShapeType::Pie {
            &mutant
        } else {
            definition
        };
        let Ok(contours) = contours_of_definition(definition, extents, &[], within) else {
            continue;
        };
        for (index, (path, contour)) in definition.paths.iter().zip(contours.iter()).enumerate() {
            if let Err(why) = walk(shape, size, "its defaults", index, path, &contour.commands) {
                reported.entry(shape).or_insert(why);
            }
        }
    }
    assert_eq!(
        reported.keys().copied().collect::<Vec<_>>(),
        vec!["pie"],
        "the mutation reddened {reported:#?}"
    );
    assert!(
        reported["pie"].contains("cubics"),
        "`pie` failed for a reason other than its arc: {}",
        reported["pie"]
    );

    // And the same walk over the unmutated table reports nothing, so the assertion above is about
    // the mutation and not about the walk.
    for definition in seeded_shapes() {
        let shape = definition.preset.to_wire();
        let Ok(contours) = contours_of_definition(definition, extents, &[], within) else {
            continue;
        };
        for (index, (path, contour)) in definition.paths.iter().zip(contours.iter()).enumerate() {
            walk(shape, size, "its defaults", index, path, &contour.commands)
                .unwrap_or_else(|why| panic!("the unmutated table failed: {why}"));
        }
    }
}
