//! The two things this crate produces for somebody else, asked from **the consumer's side**.
//!
//! # Why a file of its own
//!
//! MJXOFF-155 §6's rule is not *"is this value produced?"* but *"does it reach anyone?"*, and a
//! hand-off that failed exactly that test has reached a running child twice in this loop. Two of
//! this crate's exist only for a caller that has not been written yet, so nothing else here would
//! have called them:
//!
//! * [`ShapeOutline::from_preset_geometry`] is the bridge from a document's own `a:prstGeom` into
//!   the registry. **Its consumer is the application, in the second loop.** Until this file it was
//!   written, documented, exported — and called by nothing, which is precisely the shape of the
//!   defect `SceneMesh::provenance` was.
//! * A path step that follows an `a:close` reopens the contour at its start point. **Its consumer
//!   is MJXOFF-203**, which will generate presets drawn as several closed contours. No seeded shape
//!   has one, so it was found by mutation rather than by a test: deleting the pen's return to the
//!   subpath start was a **green mutation**, and this file is what makes it red.
//! * MJXOFF-204 added two more of exactly the same kind. [`preset_text_rectangle`]'s consumer is
//!   **Phase R's text layout** and [`preset_connection_sites`]'s is **a later child's connector
//!   routing**, and neither exists. So both are asked here the way their consumer will ask:
//!   [`where_the_first_line_goes`] makes the call a text layout makes, including
//!   [`TextRectangle::or_bounding_box`](mjx_geometry::TextRectangle::or_bounding_box), the named
//!   fallback, and [`a_connector_gets_a_point_and_a_heading_and_not_just_a_point`] walks ten pixels
//!   along each site's own `@ang` and requires the stub to leave the shape. Without the second,
//!   `ConnectionPoint::angle` would be a field written once and read never.

mod common;

use common::{box_on_the_page, curve_bounds, extents_of_the_box, StepTally};
use mjx_dml::geometry::{GeometryGuide, GeometryGuideList, PresetGeometry};
use mjx_geometry::PathFillMode;
use mjx_geometry::{
    outline_of_definition, preset_connection_sites, preset_outline, preset_text_rectangle,
    AdjustmentOverride, ConnectionPoint, Derivation, PresetCoordinate, PresetPath, PresetPathStep,
    PresetPoint, PresetShapeDefinition, PresetShapeType, ShapeOutline,
};
use mjx_ooxml_core::Interner;
use mjx_scene::{PathCommand, ScenePoint, SceneRect};

// -------------------------------------------------------------------------------------------
// The document's own `a:prstGeom`, into the registry
// -------------------------------------------------------------------------------------------

/// `<a:prstGeom prst="{preset}"><a:avLst>…</a:avLst></a:prstGeom>`, built the way a document has it.
fn a_shape(interner: &mut Interner, preset: &str, overrides: &[(&str, &str)]) -> PresetGeometry {
    let guides = overrides
        .iter()
        .map(|(name, formula)| GeometryGuide::new(interner, name, formula))
        .collect();
    let list = GeometryGuideList::new(interner, guides);
    // Built through `PresetShapeType` where the token is one this build knows, and by hand
    // otherwise — which is the case the last assertion below needs.
    let known = PresetShapeType::from_wire(preset).unwrap_or(PresetShapeType::Rectangle);
    let mut geometry = PresetGeometry::new(interner, known, Some(list));
    geometry.set_preset_token(interner, preset);
    geometry
}

#[test]
fn a_documents_own_preset_geometry_becomes_a_registry_entry_that_draws() {
    let mut interner = Interner::new();
    let extents = extents_of_the_box();
    let within = box_on_the_page();

    // A rounded rectangle whose `a:avLst` overrides `adj`, exactly as PowerPoint writes it.
    let document = a_shape(&mut interner, "roundRect", &[("adj", "val 40000")]);
    let registered = ShapeOutline::from_preset_geometry(&document, &interner, extents)
        .expect("`roundRect` is a preset this build knows");

    assert_eq!(registered.preset, PresetShapeType::RoundedRectangle);
    assert_eq!(registered.extents, extents);
    // **Only the override crosses.** A registry entry that also carried the defaults would put two
    // sources of the same number in the process, and the generated table would stop being the one.
    assert_eq!(registered.adjustments.len(), 1);
    assert_eq!(registered.adjustments[0].wire_name, "adj");
    assert_eq!(registered.adjustments[0].value, 40_000.0);

    // And the consequence: the outline it draws is the overridden one, not the default one.
    let drawn = preset_outline(
        registered.preset,
        registered.extents,
        &registered.adjustments,
        within,
    )
    .expect("the registered shape resolves");
    let defaulted = preset_outline(PresetShapeType::RoundedRectangle, extents, &[], within)
        .expect("the default resolves");
    assert_ne!(
        drawn.commands, defaulted.commands,
        "the document's own adjustment did not reach the outline it draws"
    );

    // A shape with no `a:avLst` at all registers no overrides — the ordinary case, and the one that
    // would silently work either way if the filter were removed. Asserted here because it is the
    // *structure* that matters: the defaults have one source and it is the generated table.
    let plain = ShapeOutline::from_preset_geometry(
        &a_shape(&mut interner, "roundRect", &[]),
        &interner,
        extents,
    )
    .expect("a shape with no overrides is still a shape");
    assert!(
        plain.adjustments.is_empty(),
        "a shape with an empty `avLst` carried {} override(s) into the registry",
        plain.adjustments.len()
    );
    assert_eq!(
        preset_outline(plain.preset, plain.extents, &plain.adjustments, within)
            .expect("it resolves")
            .commands,
        defaulted.commands,
        "a shape with no overrides did not draw the default"
    );

    // …and a `prst` this build does not know is `None`, which is what must reach the placeholder
    // rather than a wrong outline. A future version of the format is the whole reason this returns
    // an `Option`.
    assert!(
        ShapeOutline::from_preset_geometry(
            &a_shape(&mut interner, "somethingOffice2035Invented", &[]),
            &interner,
            extents,
        )
        .is_none(),
        "an unknown `prst` produced a registry entry, so it would draw *some* shape"
    );
}

// -------------------------------------------------------------------------------------------
// A step after an `a:close`
// -------------------------------------------------------------------------------------------

/// Two closed triangles sharing a start point, the second drawn **without** a second `a:moveTo` —
/// which ECMA-376 permits, because `a:close` returns the pen to the subpath's start.
const TWO_CONTOURS_ONE_MOVE: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "two contours sharing a start point, for the hand-off to MJXOFF-203",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::MoveTo(PresetPoint::at("l", "t")),
            PresetPathStep::LineTo(PresetPoint::at("hc", "t")),
            PresetPathStep::LineTo(PresetPoint::at("l", "vc")),
            PresetPathStep::Close,
            // The pen is back at (l, t). This line draws from there, and this is the whole point.
            PresetPathStep::LineTo(PresetPoint::at("r", "t")),
            PresetPathStep::LineTo(PresetPoint::at("r", "vc")),
            PresetPathStep::Close,
        ],
    }],
};

/// A path whose first step is not an `a:moveTo` — malformed, and it must not gain an invented one.
const A_PATH_WITH_NO_START: PresetShapeDefinition = PresetShapeDefinition {
    preset: PresetShapeType::Rectangle,
    derivation: Derivation::FromFirstPrinciples,
    source: "a path that starts with a line, which is malformed and must not gain a start point",
    adjustment_values: &[],
    guides: &[],
    text_rectangle: None,
    connection_sites: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
        fill: PathFillMode::Normal,
        stroke: true,
        extrusion_ok: true,
        steps: &[
            PresetPathStep::LineTo(PresetPoint::at("r", "b")),
            PresetPathStep::Close,
            PresetPathStep::MoveTo(PresetPoint {
                x: PresetCoordinate::Emu(0),
                y: PresetCoordinate::Emu(0),
            }),
            PresetPathStep::LineTo(PresetPoint::at("r", "t")),
            PresetPathStep::Close,
        ],
    }],
};

#[test]
fn a_step_after_a_close_draws_from_where_the_contour_began() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let outline = outline_of_definition(&TWO_CONTOURS_ONE_MOVE, extents, &[], within)
        .expect("two contours resolve");

    // Two contours: the second's `MoveTo` is supplied by the resolver, because the table's step
    // list has only one.
    assert_eq!(
        StepTally::of(&outline.commands),
        StepTally {
            moves: 2,
            lines: 4,
            quadratics: 0,
            cubics: 0,
            closes: 2,
        },
        "the second contour did not get a start point of its own"
    );
    // And it starts where the first one began, which is the whole of what `a:close` means.
    match (outline.commands[0], outline.commands[4]) {
        (PathCommand::MoveTo(first), PathCommand::MoveTo(second)) => assert_eq!(
            first, second,
            "the reopened contour begins at {second:?} and the closed one began at {first:?}"
        ),
        (first, second) => panic!("commands 0 and 4 are {first:?} and {second:?}"),
    }
}

#[test]
fn a_path_that_starts_with_a_line_does_not_gain_a_start_point() {
    // The other arm, and the reason the resolver tracks *two* booleans rather than one. Inventing a
    // start for a malformed path would draw a line from the shape's own origin — a visible stray
    // edge that no document asks for — and `mjx-scene` already drops such a step for the same
    // reason. Only a contour that was genuinely opened and then closed may be reopened.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let outline = outline_of_definition(&A_PATH_WITH_NO_START, extents, &[], within)
        .expect("a malformed path still resolves");

    assert_eq!(
        StepTally::of(&outline.commands),
        StepTally {
            moves: 1,
            lines: 2,
            quadratics: 0,
            cubics: 0,
            closes: 2,
        },
        "a leading line gained a start point, or the later contour lost one"
    );
    assert!(
        matches!(outline.commands[0], PathCommand::LineTo(_)),
        "the leading line is carried through unchanged, for `mjx-scene` to drop: {:?}",
        outline.commands[0]
    );
}

// -------------------------------------------------------------------------------------------
// MJXOFF-204's two hand-offs, asked the way their consumers will ask
// -------------------------------------------------------------------------------------------

/// Where a text layout would put the first line of a shape's text, in device pixels.
///
/// **This is the consumer, written out.** Phase R's text work is what reads
/// [`preset_text_rectangle`], and it does not exist yet — so, exactly as
/// `ShapeOutline::from_preset_geometry` was before this file, `TextRectangle` and its
/// [`or_bounding_box`](TextRectangle::or_bounding_box) would otherwise be produced, exported,
/// documented and called by nobody. This function is the smallest thing that has to make the call
/// the way a real one will: ask for the rectangle, and fall back to the shape's box **by name**
/// where there is none.
fn where_the_first_line_goes(
    preset: PresetShapeType,
    adjustments: &[AdjustmentOverride],
    within: SceneRect,
) -> (ScenePoint, bool) {
    let answer = preset_text_rectangle(preset, extents_of_the_box(), adjustments, within)
        .unwrap_or_else(|error| panic!("`{}`: {error}", preset.to_wire()));
    let laid_in = answer.or_bounding_box(within);
    // A first line sits at the top-left of the text area, which is all a caller needs from this to
    // be a different point for a different shape.
    (
        ScenePoint::new(laid_in.left, laid_in.top),
        answer.declared().is_some(),
    )
}

#[test]
fn a_text_layout_gets_a_different_answer_for_a_shape_that_insets_its_text() {
    // Three shapes, three answers, through the call a text layout will make.
    let within = box_on_the_page();
    let (rounded, rounded_declared) =
        where_the_first_line_goes(PresetShapeType::RoundedRectangle, &[], within);
    let (plain, plain_declared) =
        where_the_first_line_goes(PresetShapeType::Rectangle, &[], within);
    let (bare_line, line_declared) =
        where_the_first_line_goes(PresetShapeType::StraightLine, &[], within);

    // `roundRect` insets; `rect` declares the whole box; `line` declares nothing and falls back to
    // it. The first is a different point from the other two — which is the whole hand-off — and the
    // second and third are the same *point* reached for two different *reasons*, which is why
    // `declared()` is carried beside it.
    assert!(
        rounded.x > plain.x + 1.0 && rounded.y > plain.y + 1.0,
        "a rounded rectangle lays its first line at {rounded:?} and a plain one at {plain:?}"
    );
    assert_eq!(plain, bare_line, "the box is the box");
    assert!(rounded_declared && plain_declared);
    assert!(
        !line_declared,
        "`line` reports a text rectangle it does not declare, so a caller could not tell the \
         fallback from a real answer"
    );

    // And it moves with the document: a wider corner radius moves the first line further in, which
    // is what makes this a resolution rather than a lookup.
    let (wide, _) = where_the_first_line_goes(
        PresetShapeType::RoundedRectangle,
        &[AdjustmentOverride::new("adj", 50_000.0)],
        within,
    );
    assert!(
        wide.x > rounded.x + 5.0,
        "a corner radius three times the default moved the first line from {rounded:?} to {wide:?}"
    );
}

#[test]
fn a_connector_gets_a_point_and_a_heading_and_not_just_a_point() {
    // The other hand-off, asked the way an elbow connector will ask it: leave shape A at one of its
    // sites, travelling along that site's angle, and arrive at shape B. **Its consumer is the
    // connector routing of a later child**, so without this the angle would be a field that is
    // written and never read — the exact defect `SceneMesh::provenance` was.
    let within = box_on_the_page();
    let sites = preset_connection_sites(
        PresetShapeType::RoundedRectangle,
        extents_of_the_box(),
        &[],
        within,
    )
    .expect("`roundRect` has connection sites");
    assert_eq!(sites.len(), 4, "`roundRect` has four sites");

    // One stub per site, 10 device pixels along the site's own heading. `y` grows downward, so a
    // clockwise angle turns that way too.
    let stub = |site: &ConnectionPoint| {
        let radians = site.angle.radians();
        ScenePoint::new(
            site.position.x + 10.0 * radians.cos() as f32,
            site.position.y + 10.0 * radians.sin() as f32,
        )
    };

    // The top site leaves **upward**, out of the shape — which is the whole point of `@ang`, and
    // which a router that used the straight line to the other shape would get wrong.
    let top = &sites[0];
    assert!(
        (top.position.x - (within.left + within.right) / 2.0).abs() < 0.01,
        "the first site is not the top-edge midpoint: {:?}",
        top.position
    );
    assert!(
        stub(top).y < top.position.y - 9.0,
        "the top site's stub goes to {:?} from {:?}, which is not upward",
        stub(top),
        top.position
    );

    // …and the four stubs go four different ways, so the angle is read per site rather than once.
    let headings: Vec<(i64, i64)> = sites
        .iter()
        .map(|site| {
            let end = stub(site);
            (
                (end.x - site.position.x).round() as i64,
                (end.y - site.position.y).round() as i64,
            )
        })
        .collect();
    assert_eq!(
        headings,
        vec![(0, -10), (-10, 0), (0, 10), (10, 0)],
        "the four sites do not leave up, left, down and right"
    );

    // Every stub leaves the shape: 10 pixels along the outgoing heading is outside the outline, in
    // all four directions. A site whose angle pointed inward would fail here rather than draw a
    // connector through the shape it started in.
    let outline = preset_outline(
        PresetShapeType::RoundedRectangle,
        extents_of_the_box(),
        &[],
        within,
    )
    .expect("`roundRect` resolves");
    let bounds = curve_bounds(&outline.commands);
    for site in &sites {
        let end = stub(site);
        assert!(
            end.x < bounds.left - 0.5
                || end.x > bounds.right + 0.5
                || end.y < bounds.top - 0.5
                || end.y > bounds.bottom + 0.5,
            "a stub from {:?} ended at {end:?}, still inside the shape's own bounds {bounds:?}",
            site.position
        );
    }
}
