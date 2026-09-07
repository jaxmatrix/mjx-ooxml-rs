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

mod common;

use common::{box_on_the_page, extents_of_the_box, StepTally};
use mjx_dml::geometry::{GeometryGuide, GeometryGuideList, PresetGeometry};
use mjx_geometry::{
    outline_of_definition, preset_outline, Derivation, PresetCoordinate, PresetPath,
    PresetPathStep, PresetPoint, PresetShapeDefinition, PresetShapeType, ShapeOutline,
};
use mjx_ooxml_core::Interner;
use mjx_scene::PathCommand;

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
    guides: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
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
    guides: &[],
    paths: &[PresetPath {
        width: None,
        height: None,
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
