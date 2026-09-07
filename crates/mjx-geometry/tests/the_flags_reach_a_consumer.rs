//! `a:path@fill` and `@stroke` arrived with somebody who reads them, and this is that somebody.
//!
//! # The defect this file exists to refuse
//!
//! MJXOFF-202 extracted neither flag, deliberately, and wrote down why: *"a flag extracted and read
//! by nobody is the exact defect this loop keeps finding"* — `SceneMesh::provenance` was written
//! once and read zero times. MJXOFF-203 could not decline them, because `arc` needs them: its only
//! stroked path is `fill="none"` and filling an open contour draws a chord nothing asked for. So
//! the condition was that they arrive **with a consumer**.
//!
//! There are two, and both are gated below.
//!
//! 1. [`contours_of_definition`] is a per-`a:path` answer carrying each contour's own treatment —
//!    the answer [`mjx_scene::ResolvedOutline`] cannot hold, since the seam takes one command list
//!    and one fill rule. [`PresetGeometryProvider::contours`] is the same answer from a registered
//!    handle.
//! 2. [`outline_of_definition`] **acts** on the pair: a contour that is neither filled nor stroked
//!    draws nothing at all, and is left out of the command list the seam receives. Exactly one path
//!    in ECMA-376's geometry file is in that state, so the rule is gated by name and in both
//!    directions — the contour is present in the contour list and absent from the outline.
//!
//! `@extrusionOk` is the one flag nothing here acts on, and it is carried rather than dropped for a
//! stated reason (MJXOFF-201 §7 puts 3-D outside this epic; MJXOFF-211 names its reader). It is
//! still asserted to be *carried faithfully* — both values occur in the file and both reach the
//! table — so that when its reader arrives it finds real data rather than a column of defaults.

mod common;

use std::collections::BTreeSet;

use common::{box_on_the_page, extents_of_the_box};
use mjx_geometry::{
    contours_of_definition, definition_of, outline_of_definition, preset_contours, seeded_shapes,
    PathFillMode, PresetGeometryProvider, PresetShapeType, ShapeOutline,
};

/// The shape whose third `a:path` is `fill="none" stroke="false"` — neither filled nor stroked.
const DRAWS_NOTHING: PresetShapeType = PresetShapeType::FlowChartMultidocument;

/// Which of the three paths of [`DRAWS_NOTHING`] that is.
const THE_CONTOUR_THAT_DRAWS_NOTHING: usize = 2;

#[test]
fn a_contour_that_draws_nothing_is_reported_and_then_left_out() {
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let definition = definition_of(DRAWS_NOTHING).expect("in the generated table");

    let contours =
        contours_of_definition(definition, extents, &[], within).expect("`flowChartMultidocument`");
    assert_eq!(
        contours.len(),
        definition.paths.len(),
        "one contour per `a:path`, and no fewer"
    );

    // Exactly one contour draws nothing, and it is the one the file marks so.
    let silent: Vec<usize> = contours
        .iter()
        .enumerate()
        .filter(|(_, contour)| contour.draws_nothing())
        .map(|(index, _)| index)
        .collect();
    assert_eq!(
        silent,
        vec![THE_CONTOUR_THAT_DRAWS_NOTHING],
        "the contour that draws nothing is not the third"
    );
    let quiet = &contours[THE_CONTOUR_THAT_DRAWS_NOTHING];
    assert_eq!(quiet.fill, PathFillMode::None);
    assert!(!quiet.stroke);
    assert!(!quiet.is_filled());
    assert!(
        !quiet.commands.is_empty(),
        "the contour is still resolved — it is left out of the outline, not thrown away, so a \
         consumer that wants every contour still gets it"
    );

    // And the outline is the rest of them, exactly.
    let outline = outline_of_definition(definition, extents, &[], within).expect("the outline");
    let kept: usize = contours
        .iter()
        .filter(|contour| !contour.draws_nothing())
        .map(|contour| contour.commands.len())
        .sum();
    let all: usize = contours.iter().map(|contour| contour.commands.len()).sum();
    assert_eq!(
        outline.commands.len(),
        kept,
        "the outline is not the contours that draw something"
    );
    assert!(
        outline.commands.len() < all,
        "the outline kept every contour, so nothing read `@fill` or `@stroke` at all"
    );
    assert_eq!(
        all - kept,
        quiet.commands.len(),
        "the outline lost something other than the silent contour"
    );
}

#[test]
fn arc_keeps_its_stroked_outline_and_its_filled_body_apart() {
    // The shape MJXOFF-202 named as the reason the flags could not be declined. Two paths: one
    // `fill="none"` that is only stroked, one `stroke="false"` that is only filled. Both are drawn
    // — neither draws nothing — and only a consumer that reads the flags can tell them apart.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let definition = definition_of(PresetShapeType::Arc).expect("in the generated table");
    let contours = contours_of_definition(definition, extents, &[], within).expect("`arc`");

    let treatments: Vec<(PathFillMode, bool)> = contours
        .iter()
        .map(|contour| (contour.fill, contour.stroke))
        .collect();
    assert!(
        treatments.contains(&(PathFillMode::None, true)),
        "`arc` has no stroked, unfilled contour; its visible curve would be filled into a chord"
    );
    assert!(
        treatments
            .iter()
            .any(|(fill, stroke)| *fill != PathFillMode::None && !*stroke),
        "`arc` has no filled, unstroked contour"
    );
    assert!(
        contours.iter().all(|contour| !contour.draws_nothing()),
        "one of `arc`'s contours draws nothing, so the outline would lose it"
    );
    assert_eq!(
        outline_of_definition(definition, extents, &[], within)
            .expect("the outline")
            .commands
            .len(),
        contours.iter().map(|c| c.commands.len()).sum::<usize>(),
        "`arc`'s outline is not both of its contours"
    );
}

#[test]
fn every_fill_mode_and_both_flags_occur_in_the_table() {
    // A field that only ever holds its default is a field nothing carries. All six `ST_PathFillMode`
    // values occur in ECMA-376's geometry file and all six must reach the table; so must both
    // values of `@stroke` and both of `@extrusionOk`.
    let mut fills: BTreeSet<&'static str> = BTreeSet::new();
    let (mut stroked, mut unstroked) = (false, false);
    let (mut extrudable, mut not_extrudable) = (false, false);
    for path in seeded_shapes()
        .iter()
        .flat_map(|definition| definition.paths)
    {
        fills.insert(path.fill.to_wire());
        stroked |= path.stroke;
        unstroked |= !path.stroke;
        extrudable |= path.extrusion_ok;
        not_extrudable |= !path.extrusion_ok;
    }
    assert_eq!(
        fills,
        BTreeSet::from([
            "none",
            "norm",
            "lighten",
            "lightenLess",
            "darken",
            "darkenLess"
        ]),
        "the table does not carry every `ST_PathFillMode` value"
    );
    assert!(stroked && unstroked, "`@stroke` only ever takes one value");
    assert!(
        extrudable && not_extrudable,
        "`@extrusionOk` only ever takes one value, so it is a column of defaults"
    );

    // `is_filled` is the predicate the outline acts through; both of its arms are taken by real
    // rows, which is what makes the branch above a decision rather than a constant.
    assert!(seeded_shapes()
        .iter()
        .flat_map(|definition| definition.paths)
        .any(|path| path.is_filled()));
    assert!(seeded_shapes()
        .iter()
        .flat_map(|definition| definition.paths)
        .any(|path| !path.is_filled()));
    assert_eq!(
        seeded_shapes()
            .iter()
            .flat_map(|definition| definition.paths)
            .filter(|path| path.draws_nothing())
            .count(),
        1,
        "the number of paths in the file that draw nothing at all has changed"
    );
}

#[test]
fn a_registered_handle_answers_with_its_contours_as_well_as_its_outline() {
    // The provider's own reader, so the richer answer is reachable from the same place the seam's
    // is, rather than only from a free function a caller has to know about.
    let (within, extents) = (box_on_the_page(), extents_of_the_box());
    let mut provider = PresetGeometryProvider::new();
    provider.register(11, ShapeOutline::new(PresetShapeType::Arc, extents));

    let contours = provider.contours(11, within).expect("a registered handle");
    let direct = preset_contours(PresetShapeType::Arc, extents, &[], within).expect("the same");
    assert_eq!(contours, direct, "the provider answered something else");
    assert!(contours.len() > 1, "`arc` is more than one contour");

    let unknown = provider.contours(12, within);
    assert!(
        unknown.is_err(),
        "an unregistered handle answered with contours rather than an error"
    );
}
