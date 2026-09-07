//! The real provider, in the place a real document is rendered from — and the stand-in counted
//! from both of the two lines that count it.
//!
//! MJX-STAND-IN: this file asserts the stand-in is still reachable and still `mjx-scene`'s own, so
//! it builds one to compare the provider's fall-through against. A second implementation of the
//! placeholder would pass every count in this file and fail that comparison.
//!
//! # What this file claims that `a_preset_renders_as_itself.rs` does not
//!
//! MJXOFF-202 asked for `DrawReport::placeholders` in both directions and got it: zero for a scene
//! of the table's own shapes, one for a scene containing `upArrow`. Both of those scenes are built
//! **from the typed registry**, are drawn **as fills**, and are lowered by `plan_frame` alone. Each
//! of those three is a place a wiring defect can hide, and this file closes all three:
//!
//! 1. **The document's own `a:prstGeom` is the route.** Every shape here is registered through
//!    [`ShapeOutline::from_preset_geometry`] out of a `PresetGeometry` built the way a `.pptx` has
//!    it — token, `a:avLst` and all. That bridge existed, was documented and was exercised as far
//!    as `preset_outline`; **it had never been shown to reach a painter's report.** A bridge that
//!    resolves a shape correctly and is wired to nothing renders exactly as many placeholders as no
//!    bridge at all.
//!
//! 2. **A stand-in is counted from a stroke as well as from a fill.** `crates/mjx-paint/src/plan.rs`
//!    increments the counter in exactly two places — under `Command::FillPath` (`853`) and under
//!    `Command::StrokePath` (`892`) — and until MJXOFF-206 **every stand-in in this workspace was
//!    filled**. Replacing the stroke increment with `std::process::abort()` left `mjx-paint`,
//!    `mjx-scene` and `mjx-geometry` green: the line had never executed. Half a counter satisfies
//!    "zero placeholders" exactly as well as a whole one.
//!
//! 3. **Every painter, not only the planner.** The count reaches a caller through four painters,
//!    and a report the planner fills and a painter drops is the same defect one layer up — which is
//!    what `SceneMesh::provenance` was before MJXOFF-163. The three pure-Rust painters run here on
//!    every machine; the `wgpu` one asserts the same pair in
//!    `crates/mjx-paint/tests/a_page_becomes_pixels.rs`, where a missing adapter is a named skip.
//!
//! # And the direction that is not "it is zero"
//!
//! **"No placeholders were reported" is true of a page with no shapes on it**, so every zero here
//! is asserted beside the draw call and triangle counts that say the walk reached the shapes. Every
//! non-zero is asserted beside the *producer* that made it, because the three producers of a
//! stand-in — an unregistered handle, a preset ECMA-376 defines no geometry for, and a shape whose
//! own formulas are singular at the adjustments in force — are three different arms of
//! `PresetGeometryProvider::outline` and a gate that only ever uses one proves only that one.
//!
//! # Two orientations, not one
//!
//! Every census in this crate is taken in both, and for the reason `common::orientations` gives:
//! `ss` is `min(w, h)`, so in a landscape box it is always the height and an implementation that
//! read `h` where the formula says `ss` would be invisible. A wiring suite has no less reason to
//! ask — a page laid out in one aspect ratio is one sample.

mod common;

use common::orientations;
use mjx_dml::geometry::{GeometryGuide, GeometryGuideList, PresetGeometry};
use mjx_geometry::{
    adjustment_domains, seeded_shapes, AdjustmentOverride, GeometryError, PresetGeometryProvider,
    PresetShapeType, ShapeOutline, Size, UnknownShapePolicy, PRESETS_WITHOUT_GEOMETRY,
};
use mjx_ooxml_core::Interner;
use mjx_paint::{
    plan_frame, NoGlyphs, NoImages, OffscreenSurface, PaintError, Painter, PdfPainter, Resources,
    SoftwarePainter, SvgPainter, Viewport,
};
use mjx_scene::{
    Color, Command, DeviceScale, DisplayList, Geometry, GeometryProvider, OutlineProvenance, Paint,
    PlaceholderGeometry, SceneBuilder, SceneError, SceneRect, StrokeStyle, Tessellator,
};

// -------------------------------------------------------------------------------------------
// The deck: 186 presets, laid out on one page, each drawn twice
// -------------------------------------------------------------------------------------------

/// How many plates to a row. Fourteen, so 186 shapes make a nearly square sheet at either
/// orientation and no row is a special case.
const COLUMNS: usize = 14;

/// How wide the deck expects the table to be.
///
/// Stated rather than derived, because "every preset drew" is the claim and a count taken from the
/// same list the loop walks would agree with a list that had lost a row. `mjx-geometry`'s own
/// `the_generated_table_is_the_spec_file.rs` is where the number comes from.
const PRESETS: usize = 186;

/// A shape's box inside its plate, in device pixels, at each orientation.
///
/// Never square: a mapping that swapped the axes would land inside a square box unchanged.
fn plate(landscape: bool) -> (f32, f32) {
    if landscape {
        (100.0, 80.0)
    } else {
        (80.0, 100.0)
    }
}

/// The margin between one plate's box and the next, so two shapes are never the same rectangle.
const GUTTER: f32 = 20.0;

/// Where the `index`-th shape's box is on the sheet.
fn box_of(index: usize, landscape: bool) -> SceneRect {
    let (width, height) = plate(landscape);
    let (column, row) = (index % COLUMNS, index / COLUMNS);
    let left = column as f32 * (width + GUTTER) + GUTTER / 2.0;
    let top = row as f32 * (height + GUTTER) + GUTTER / 2.0;
    SceneRect::new(left, top, left + width, top + height)
}

/// How large the sheet has to be to hold `count` plates.
fn sheet(count: usize, landscape: bool) -> (f32, f32) {
    let (width, height) = plate(landscape);
    let rows = count.div_ceil(COLUMNS);
    (
        COLUMNS as f32 * (width + GUTTER),
        rows as f32 * (height + GUTTER),
    )
}

/// The extents a shape in [`box_of`] has in the document, at one device pixel to the point.
fn extents(landscape: bool) -> Size {
    let (width, height) = plate(landscape);
    Size::from_emu((width as i64) * 12_700, (height as i64) * 12_700)
}

/// The handle the `index`-th shape is registered under.
///
/// Deliberately not `index`: a resolver that ignored the registry and indexed the table directly
/// would answer correctly for handles that happen to be `0..n`, and that is the wiring defect this
/// file is looking for.
fn handle_of(index: usize) -> u64 {
    0x00A5_0000_0000_0000 | ((index as u64) * 7 + 3)
}

/// `<a:prstGeom prst="{token}"><a:avLst>…</a:avLst></a:prstGeom>`, built the way a document has it.
///
/// The same construction `the_hand_offs_reach_somebody.rs` uses, because the point of the route is
/// that it is the document's and there must not be a second idea of what a document's looks like.
fn a_document_shape(
    interner: &mut Interner,
    token: &str,
    overrides: &[(&str, &str)],
) -> PresetGeometry {
    let guides = overrides
        .iter()
        .map(|(name, formula)| GeometryGuide::new(interner, name, formula))
        .collect();
    let list = GeometryGuideList::new(interner, guides);
    let known = PresetShapeType::from_wire(token).unwrap_or(PresetShapeType::Rectangle);
    let mut geometry = PresetGeometry::new(interner, known, Some(list));
    geometry.set_preset_token(interner, token);
    geometry
}

/// Every preset the table draws, registered **out of a document's own `a:prstGeom`**.
///
/// Answers the provider and the handles, in table order, so a caller can lay them out.
fn a_deck_of_every_preset(
    policy: UnknownShapePolicy,
    landscape: bool,
) -> (PresetGeometryProvider, Vec<u64>) {
    let mut interner = Interner::new();
    let mut provider = provider_with(policy);
    let mut handles = Vec::new();
    for (index, definition) in seeded_shapes().iter().enumerate() {
        let token = definition.preset.to_wire();
        let document = a_document_shape(&mut interner, token, &[]);
        let outline = ShapeOutline::from_preset_geometry(&document, &interner, extents(landscape))
            .unwrap_or_else(|| {
                panic!("`{token}` is in the table but did not cross the document bridge")
            });
        assert_eq!(
            outline.preset, definition.preset,
            "`{token}` crossed the bridge as some other shape"
        );
        let handle = handle_of(index);
        assert!(
            provider.register(handle, outline).is_none(),
            "`{token}` was registered on a handle something else already held"
        );
        handles.push(handle);
    }
    (provider, handles)
}

/// A provider under `policy`, so no suite writes the `match` twice.
fn provider_with(policy: UnknownShapePolicy) -> PresetGeometryProvider {
    match policy {
        UnknownShapePolicy::Refuse => PresetGeometryProvider::new(),
        UnknownShapePolicy::StandIn => PresetGeometryProvider::standing_in_for_unknown_shapes(),
    }
}

/// Which of a shape's two draws a page carries.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Draws {
    /// One `Command::FillPath` per handle — the arm `plan.rs:853` counts.
    Filled,
    /// One `Command::StrokePath` per handle — the arm `plan.rs:892` counts.
    Stroked,
    /// Both, so one page exercises both lines and the total is twice the shape count.
    Both,
}

impl Draws {
    /// How many draw calls one handle produces.
    fn per_handle(self) -> usize {
        match self {
            Self::Filled | Self::Stroked => 1,
            Self::Both => 2,
        }
    }

    fn fills(self) -> bool {
        matches!(self, Self::Filled | Self::Both)
    }

    fn strokes(self) -> bool {
        matches!(self, Self::Stroked | Self::Both)
    }
}

/// A page with one plate per handle, nothing resolved while it is built.
///
/// Every geometry is a [`Geometry::Unresolved`] — the display list carries the handle and the box
/// and no path at all, which is the seam working — so the shapes on this page are whatever the
/// provider handed to `plan_frame` says they are.
fn a_page_of(handles: &[u64], landscape: bool, draws: Draws) -> DisplayList {
    let (width, height) = sheet(handles.len(), landscape);
    let mut builder = SceneBuilder::new(DeviceScale::UNZOOMED, width, height);
    let paint = builder
        .add_paint(Paint::Solid(Color {
            red: 0x22,
            green: 0x44,
            blue: 0x88,
            alpha: 0xff,
        }))
        .expect("one paint");
    let stroke = builder
        .add_stroke_style(&StrokeStyle::solid(
            2.0,
            Color {
                red: 0x11,
                green: 0x11,
                blue: 0x11,
                alpha: 0xff,
            },
        ))
        .expect("a stroke interns")
        .expect("a visible stroke");
    for (index, handle) in handles.iter().enumerate() {
        let geometry = builder
            .add_geometry(&Geometry::Unresolved {
                outline: *handle,
                bounds: box_of(index, landscape),
            })
            .expect("an unresolved geometry");
        if draws.fills() {
            builder
                .push(Command::FillPath { geometry, paint })
                .expect("a fill");
        }
        if draws.strokes() {
            builder
                .push(Command::StrokePath { geometry, stroke })
                .expect("a stroke");
        }
    }
    builder.finish().expect("the scene is well formed")
}

// -------------------------------------------------------------------------------------------
// Zero, over a deck that certainly contains shapes
// -------------------------------------------------------------------------------------------

#[test]
fn a_deck_of_every_preset_out_of_a_document_draws_no_stand_in_at_all() {
    for (orientation, _, _) in orientations() {
        let landscape = orientation == "landscape";
        let (provider, handles) = a_deck_of_every_preset(UnknownShapePolicy::Refuse, landscape);
        assert_eq!(
            handles.len(),
            PRESETS,
            "the table is no longer {PRESETS} shapes; the deck this file draws is derived from it \
             and the count is not"
        );

        let list = a_page_of(&handles, landscape, Draws::Both);
        let mut tessellator = Tessellator::new();
        let plan = plan_frame(&list, &provider, &mut tessellator)
            .unwrap_or_else(|error| panic!("the {orientation} deck did not lower: {error}"));
        let report = plan.report();

        assert_eq!(
            report.placeholders, 0,
            "a {orientation} deck of {PRESETS} presets, taken from their own `a:prstGeom`, drew \
             {} stand-in(s)",
            report.placeholders
        );

        // The half that makes the zero mean something. A walk that reached no command, a provider
        // that answered nothing and a tessellator that made no triangles all satisfy "no
        // placeholders were reported" perfectly.
        assert_eq!(
            report.draw_calls,
            handles.len() * Draws::Both.per_handle(),
            "the {orientation} deck's fills and strokes did not all draw: {report:?}"
        );
        assert!(
            report.triangles > handles.len() * 4,
            "{} shapes filled and stroked became {} triangle(s), which cannot be that many \
             outlines: {report:?}",
            handles.len(),
            report.triangles
        );

        // And the provider really answered with the document's own geometry rather than with
        // something that merely was not a placeholder: every handle resolves, and says so.
        for (index, handle) in handles.iter().enumerate() {
            let outline = provider
                .outline(*handle, box_of(index, landscape))
                .expect("a registered preset resolves");
            assert_eq!(
                outline.provenance,
                OutlineProvenance::Document,
                "handle {handle} answered with a stand-in's provenance"
            );
        }
    }
}

#[test]
fn each_command_kind_alone_still_draws_the_whole_deck_with_no_stand_in() {
    // The `Both` page above would pass with one of the two kinds silently skipped — the count would
    // simply be lower and the zero would be unchanged. Each kind alone pins its own number.
    for (draws, expected) in [(Draws::Filled, 1usize), (Draws::Stroked, 1)] {
        let (provider, handles) = a_deck_of_every_preset(UnknownShapePolicy::Refuse, true);
        let list = a_page_of(&handles, true, draws);
        let mut tessellator = Tessellator::new();
        let plan = plan_frame(&list, &provider, &mut tessellator)
            .unwrap_or_else(|error| panic!("a {draws:?} deck did not lower: {error}"));
        let report = plan.report();
        assert_eq!(report.placeholders, 0, "{draws:?}: {report:?}");
        assert_eq!(
            report.draw_calls,
            handles.len() * expected,
            "{draws:?} did not draw every shape once: {report:?}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// Non-zero, from three producers and from both command kinds
// -------------------------------------------------------------------------------------------

/// A shape a real document can contain that this build genuinely has no geometry for, with the
/// arm of `PresetGeometryProvider::outline` it reaches, and the handle to register it on.
///
/// **Three, and not one.** They are three different producers of the same fall-through, and the
/// list is what a gate that only ever used `upArrow` would not have:
///
/// * an **unregistered handle** — the case a document with a shape nobody minted a registry entry
///   for reaches, and the only one that needs no shape at all;
/// * **`upArrow`** — the single preset `ST_ShapeType` declares and `presetShapeDefinitions.xml`
///   defines nothing for, so [`PRESETS_WITHOUT_GEOMETRY`] is exactly `[UpArrow]`;
/// * **`circularArrow` at `adj5`'s own minimum** — a shape the table *has*, whose `swAng` has no
///   value at one end of a handle's own domain. A user dragging that handle reaches it.
fn unresolvable_shapes() -> Vec<(&'static str, Option<ShapeOutline>, ErrorKind)> {
    let unseeded = PRESETS_WITHOUT_GEOMETRY[0];
    assert!(
        mjx_geometry::definition_of(unseeded).is_none(),
        "`{}` is now seeded; pick a preset this build still cannot draw",
        unseeded.to_wire()
    );

    let singular = seeded_shapes()
        .iter()
        .find(|definition| definition.preset.to_wire() == "circularArrow")
        .map(|definition| definition.preset)
        .expect("`circularArrow` is in the table");
    let domain = adjustment_domains(singular, extents(true), &[])
        .expect("`circularArrow` has domains")
        .into_iter()
        .find(|domain| domain.spec.wire_name == "adj5")
        .expect("`circularArrow` declares `adj5`");

    vec![
        ("an unregistered handle", None, ErrorKind::Unregistered),
        (
            "a preset ECMA-376 defines no geometry for",
            Some(ShapeOutline::new(unseeded, extents(true))),
            ErrorKind::Unseeded,
        ),
        (
            "a shape singular at its own adjustment's minimum",
            Some(
                ShapeOutline::new(singular, extents(true)).with_adjustment("adj5", domain.minimum),
            ),
            ErrorKind::Singular,
        ),
    ]
}

/// Which arm of [`GeometryError`] a producer reaches.
///
/// Named, and asserted per producer, because a list of three cases that all failed the same way
/// would be one case written three times — and the fall-through it exercises is keyed on
/// `has_no_geometry_to_draw`, which spans all three and would hide the collapse.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum ErrorKind {
    Unregistered,
    Unseeded,
    Singular,
}

impl ErrorKind {
    /// Whether `error` is this kind.
    fn matches(self, error: &GeometryError) -> bool {
        match self {
            Self::Unregistered => matches!(error, GeometryError::UnregisteredOutline { .. }),
            Self::Unseeded => matches!(error, GeometryError::UnseededShape { .. }),
            Self::Singular => matches!(error, GeometryError::SingularGeometry { .. }),
        }
    }
}

#[test]
fn every_producer_of_a_stand_in_is_counted_from_a_fill_and_from_a_stroke() {
    let producers = unresolvable_shapes();
    assert_eq!(
        producers
            .iter()
            .map(|(_, _, kind)| *kind)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        3,
        "the three producers collapsed onto fewer than three arms of `GeometryError`, so this \
         gate is one case written three times"
    );

    for (what, shape, kind) in producers {
        for draws in [Draws::Filled, Draws::Stroked, Draws::Both] {
            let mut provider = provider_with(UnknownShapePolicy::StandIn);
            let handle = handle_of(0);
            if let Some(shape) = shape.clone() {
                provider.register(handle, shape);
            }

            // The producer really is the one the list names. `resolve` is the policy-free answer,
            // so this reads the reason the fall-through fired rather than inferring it.
            let error = provider
                .resolve(handle, box_of(0, true))
                .expect_err("this shape cannot be drawn");
            assert!(
                kind.matches(&error),
                "{what} failed as {error:?}, which is not {kind:?}"
            );
            assert!(
                error.has_no_geometry_to_draw(),
                "{what} failed with {error}, which the provider would refuse rather than stand in \
                 for"
            );

            let list = a_page_of(&[handle], true, draws);
            let mut tessellator = Tessellator::new();
            let plan = plan_frame(&list, &provider, &mut tessellator).unwrap_or_else(|error| {
                panic!("{what}, {draws:?}: the page did not lower: {error}")
            });
            let report = plan.report();
            assert_eq!(
                report.placeholders,
                draws.per_handle(),
                "{what}, drawn {draws:?}, was counted {} time(s) rather than {}: {report:?}",
                report.placeholders,
                draws.per_handle()
            );
            assert_eq!(
                report.draw_calls,
                draws.per_handle(),
                "{what}, drawn {draws:?}: the stand-in has to *draw*, not merely be counted — a \
                 shape that silently drew nothing is the one answer the seam forbids: {report:?}"
            );
            assert!(
                report.triangles > 0,
                "{what}, drawn {draws:?}: the stand-in produced no triangles at all: {report:?}"
            );
        }
    }
}

#[test]
fn one_unresolvable_shape_among_a_whole_deck_raises_the_count_off_zero() {
    // The number a real deck would show: 186 shapes the table draws and one it does not, on one
    // page, filled and stroked. A counter wired to the wrong branch would answer 0 or 374 here and
    // both are visibly wrong.
    let (mut provider, mut handles) = a_deck_of_every_preset(UnknownShapePolicy::StandIn, true);
    let unseeded = handle_of(handles.len());
    provider.register(
        unseeded,
        ShapeOutline::new(PRESETS_WITHOUT_GEOMETRY[0], extents(true)),
    );
    handles.push(unseeded);

    let list = a_page_of(&handles, true, Draws::Both);
    let mut tessellator = Tessellator::new();
    let plan = plan_frame(&list, &provider, &mut tessellator).expect("the page lowers");
    let report = plan.report();
    assert_eq!(
        report.placeholders, 2,
        "one unseeded shape drawn twice among {PRESETS} the table has: {report:?}"
    );
    assert_eq!(report.draw_calls, handles.len() * 2);
}

#[test]
fn refusing_is_a_refusal_and_not_a_quietly_wrong_page() {
    // The other policy, and the reason there are two. `Refuse` is the default and is what a gate
    // wants: a page containing a shape this build cannot draw fails to lower rather than lowering
    // into something a golden image could be taken against. Asserted for both command kinds,
    // because the fill and the stroke resolve through different call sites.
    for draws in [Draws::Filled, Draws::Stroked, Draws::Both] {
        let provider = provider_with(UnknownShapePolicy::Refuse);
        let list = a_page_of(&[handle_of(0)], true, draws);
        let mut tessellator = Tessellator::new();
        let error = plan_frame(&list, &provider, &mut tessellator)
            .expect_err("a refusing provider must refuse an unregistered handle");
        let handle = handle_of(0);
        assert!(
            matches!(
                &error,
                PaintError::Scene(SceneError::UnresolvedOutline { outline }) if *outline == handle
            ),
            "{draws:?}: the refusal is not `UnresolvedOutline` naming handle {handle}: {error:?}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The stand-in is still there, and is still the stand-in
// -------------------------------------------------------------------------------------------

#[test]
fn the_stand_in_is_still_reachable_and_is_still_the_scenes_own() {
    // `PlaceholderGeometry` is the honest answer for a shape the table cannot resolve, and deleting
    // it would replace a visible placeholder with a silent nothing. So this asserts two things: the
    // path is reachable, **and** what comes back is `mjx-scene`'s own stand-in rather than a second
    // implementation of it that has drifted. A provider that built its own framed rectangle would
    // pass every count in this file and fail here.
    let within = box_of(0, true);
    let expected = PlaceholderGeometry::new()
        .outline(11, within)
        .expect("the stand-in answers every handle");

    let provider = provider_with(UnknownShapePolicy::StandIn);
    let answered = provider
        .outline(11, within)
        .expect("a standing-in provider answers an unregistered handle");

    assert_eq!(answered.provenance, OutlineProvenance::Placeholder);
    assert_eq!(
        answered.commands, expected.commands,
        "not the scene's own stand-in path"
    );
    assert_eq!(answered.fill_rule, expected.fill_rule);
    assert_eq!(answered.label, PlaceholderGeometry::label_for(11));
    assert!(
        !answered.commands.is_empty(),
        "an empty path is the one answer the seam forbids"
    );
}

// -------------------------------------------------------------------------------------------
// Every painter, not only the planner
// -------------------------------------------------------------------------------------------

/// Draw `list` through `painter` with `provider`, and answer what the frame reported.
fn painted(
    painter: &mut dyn Painter,
    list: &DisplayList,
    provider: &dyn GeometryProvider,
    width: u32,
    height: u32,
) -> mjx_paint::DrawReport {
    let mut glyphs = NoGlyphs;
    let images = NoImages;
    let mut host = OffscreenSurface::new(width, height, 1.0);
    let viewport = Viewport::covering(&host);
    let frame = painter.begin(&mut host, viewport).expect("a frame opens");
    let mut resources = Resources::new(&mut glyphs, provider, &images);
    let drawn = painter
        .draw(&frame, list, &mut resources)
        .expect("the page draws");
    painter.end(frame).expect("the frame finishes");
    drawn
}

#[test]
fn every_pure_rust_painter_reports_the_count_in_both_directions() {
    // **The trap, in this file's terms.** R09 added three painters and asserted the field for none
    // of them; the fix was to assert it per painter rather than per planner. A report the planner
    // fills and a painter drops is the same defect one layer up, and the count is what R10 refuses
    // a golden image on.
    //
    // The three here need no graphics stack, so they run everywhere. The `wgpu` painter asserts the
    // same pair in `crates/mjx-paint/tests/a_page_becomes_pixels.rs`, where a missing adapter is a
    // named skip rather than a silent pass.
    let (real, handles) = a_deck_of_every_preset(UnknownShapePolicy::Refuse, true);
    let deck = a_page_of(&handles, true, Draws::Both);
    let (width, height) = sheet(handles.len(), true);

    let mut standing_in = provider_with(UnknownShapePolicy::StandIn);
    standing_in.register(
        handle_of(0),
        ShapeOutline::new(PRESETS_WITHOUT_GEOMETRY[0], extents(true)),
    );
    let unresolvable = a_page_of(&[handle_of(0)], true, Draws::Both);

    let painters: Vec<(&str, Box<dyn Painter>)> = vec![
        ("tiny-skia", Box::new(SoftwarePainter::new())),
        ("svg", Box::new(SvgPainter::new())),
        ("pdf", Box::new(PdfPainter::new())),
    ];

    for (name, mut painter) in painters {
        let drawn = painted(painter.as_mut(), &deck, &real, width as u32, height as u32);
        assert_eq!(
            drawn.placeholders, 0,
            "`{name}` reported {} stand-in(s) for a deck of {PRESETS} presets it was handed the \
             real provider for: {drawn:?}",
            drawn.placeholders
        );
        assert_eq!(
            drawn.draw_calls,
            handles.len() * 2,
            "`{name}` did not draw the deck at all, which satisfies a zero count perfectly: \
             {drawn:?}"
        );

        let drawn = painted(painter.as_mut(), &unresolvable, &standing_in, 160, 160);
        assert_eq!(
            drawn.placeholders, 2,
            "`{name}` did not count a stand-in drawn as a fill *and* as a stroke: {drawn:?}"
        );
    }
}

// -------------------------------------------------------------------------------------------
// The identity-value probe: the registry is read, not guessed
// -------------------------------------------------------------------------------------------

#[test]
fn the_handle_decides_the_shape_and_not_the_order_it_was_registered_in() {
    // A provider that ignored its registry and walked the table in order would answer every
    // assertion above unchanged, because the deck is built in table order. This is the probe that
    // separates the two: the same two handles, registered in the opposite order, draw the same two
    // shapes.
    let within = box_of(0, true);
    let size = extents(true);
    let mut forwards = provider_with(UnknownShapePolicy::Refuse);
    forwards.register(1, ShapeOutline::new(PresetShapeType::Ellipse, size));
    forwards.register(2, ShapeOutline::new(PresetShapeType::Triangle, size));

    let mut backwards = provider_with(UnknownShapePolicy::Refuse);
    backwards.register(2, ShapeOutline::new(PresetShapeType::Triangle, size));
    backwards.register(1, ShapeOutline::new(PresetShapeType::Ellipse, size));

    for handle in [1u64, 2] {
        assert_eq!(
            forwards
                .outline(handle, within)
                .expect("it resolves")
                .commands,
            backwards
                .outline(handle, within)
                .expect("it resolves")
                .commands,
            "handle {handle} drew a different shape depending on the order it was registered in"
        );
    }
    assert_ne!(
        forwards.outline(1, within).expect("it resolves").commands,
        forwards.outline(2, within).expect("it resolves").commands,
        "an ellipse and a triangle drew the same path, so the handle is not deciding anything"
    );
}

#[test]
fn a_documents_own_adjustment_reaches_the_painted_page() {
    // The document route's own identity-value probe. `from_preset_geometry` carries only the
    // *overridden* adjustments across, and a bridge that dropped them would draw 186 correct
    // default shapes and be wrong on every deck a person has actually edited — which is invisible
    // to every count in this file, because a stand-in is not what a dropped override produces.
    let mut interner = Interner::new();
    let size = extents(true);
    let within = box_of(0, true);

    let plain = ShapeOutline::from_preset_geometry(
        &a_document_shape(&mut interner, "roundRect", &[]),
        &interner,
        size,
    )
    .expect("`roundRect` crosses the bridge");
    let adjusted = ShapeOutline::from_preset_geometry(
        &a_document_shape(&mut interner, "roundRect", &[("adj", "val 40000")]),
        &interner,
        size,
    )
    .expect("`roundRect` with an override crosses the bridge");
    assert_eq!(adjusted.adjustments.len(), 1);
    assert_eq!(
        adjusted.adjustments,
        vec![AdjustmentOverride::new("adj", 40_000.0)]
    );

    let mut provider = provider_with(UnknownShapePolicy::Refuse);
    provider.register(1, plain);
    provider.register(2, adjusted);
    assert_ne!(
        provider.outline(1, within).expect("it resolves").commands,
        provider.outline(2, within).expect("it resolves").commands,
        "a document's own `a:avLst` did not reach the outline the page draws"
    );

    // And it reaches the *painted* page, which is the claim this file is for: two handles, one
    // adjustment apart, drawn through the planner rather than asked of the provider directly.
    let list = a_page_of(&[1, 2], true, Draws::Both);
    let mut tessellator = Tessellator::new();
    let plan = plan_frame(&list, &provider, &mut tessellator).expect("the page lowers");
    assert_eq!(plan.report().placeholders, 0);
    assert_eq!(plan.report().draw_calls, 4);
}
