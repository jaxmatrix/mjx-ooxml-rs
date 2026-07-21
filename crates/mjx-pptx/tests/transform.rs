//! Integration tests for the transform surface: read a shape's bounds, move / resize / rotate /
//! flip it, give one a transform it never had — on every shape kind and every surface — with
//! fidelity (only the edited part changes, and reading changes nothing).
//!
//! `tests/fixtures/layouts.pptx` is the deck with transforms to read: its slides, layouts and master
//! all place their shapes explicitly, and slide 2 carries a `p:grpSp` and a `p:graphicFrame` so the
//! two exotic locator paths are exercised against a real file rather than only a fragment.
//! `sample.pptx`'s single `p:sp` declares **no** transform, which is what makes it the fixture for
//! creating one.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{Angle, Position, Size, Transform2D};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_opc::Package;
use mjx_pptx::{PptxError, Presentation, ShapeBounds, ShapeKind, Surface};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn layouts() -> Presentation {
    Presentation::open(&fixture("layouts.pptx")).expect("open layouts.pptx")
}

fn sample() -> Presentation {
    Presentation::open(&fixture("sample.pptx")).expect("open sample.pptx")
}

/// Asserts every part except those named survived a save byte-for-byte.
#[track_caller]
fn only_these_parts_changed(before: &BTreeMap<String, Vec<u8>>, saved: &[u8], edited: &[&str]) {
    let after = byte_map(&Package::open(saved).expect("reopen package"));
    for (name, original) in before {
        if edited.iter().any(|part| name.ends_with(part)) {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(original),
            "a transform edit dirtied unrelated part {name}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// Reading
// ---------------------------------------------------------------------------------------------

#[test]
fn reads_the_bounds_a_shape_declares() {
    let mut pres = layouts();
    let bounds = pres.shape_bounds(0, 0).expect("bounds").expect("declared");

    // Whatever the fixture says, bounds must be inside a sane slide and non-degenerate.
    let size = pres.slide_size().expect("slide size");
    assert!(bounds.width_emu > 0 && bounds.height_emu > 0);
    assert!(bounds.offset_x_emu >= 0 && bounds.offset_x_emu < size.width_emu);
    assert!(bounds.offset_y_emu >= 0 && bounds.offset_y_emu < size.height_emu);
}

#[test]
fn a_shape_that_places_itself_nowhere_reads_as_none() {
    // sample.pptx's title declares no `a:xfrm` — it inherits its position from the layout.
    let mut pres = sample();
    assert_eq!(pres.shape_bounds(0, 0).expect("bounds"), None);
    assert_eq!(pres.shape_transform(0, 0).expect("transform"), None);
}

#[test]
fn reading_a_transform_dirties_nothing() {
    let mut pres = layouts();
    let before = byte_map(&Package::open(&fixture("layouts.pptx")).expect("open package"));
    let _ = pres.shape_bounds(0, 0).expect("bounds");
    let _ = pres.shape_transform(Surface::Layout(0), 0).expect("layout");
    let _ = pres.shape_transform(Surface::Master(0), 0).expect("master");

    let saved = pres.save().expect("save");
    only_these_parts_changed(&before, &saved, &[]);
}

#[test]
fn every_surface_answers() {
    // A layout's and a master's shapes are placed too, and the same API reaches them.
    let mut pres = layouts();
    for surface in [
        Surface::Slide(0),
        Surface::Layout(0),
        Surface::Layout(1),
        Surface::Master(0),
    ] {
        let count = pres.shape_count(surface).expect("count");
        for idx in 0..count {
            // Must not error on any shape of any kind, whatever it answers.
            let _ = pres.shape_transform(surface, idx).expect("transform");
        }
    }
}

#[test]
fn out_of_range_is_a_typed_error() {
    let mut pres = sample();
    assert!(matches!(
        pres.shape_bounds(0, 99),
        Err(PptxError::ShapeIndexOutOfRange { index: 99, .. })
    ));
    assert!(matches!(
        pres.set_shape_bounds(0, 99, ShapeBounds::from_inches(0.0, 0.0, 1.0, 1.0)),
        Err(PptxError::ShapeIndexOutOfRange { index: 99, .. })
    ));
}

// ---------------------------------------------------------------------------------------------
// Writing
// ---------------------------------------------------------------------------------------------

#[test]
fn moving_and_resizing_round_trips_through_a_save() {
    let mut pres = layouts();
    let before = byte_map(&Package::open(&fixture("layouts.pptx")).expect("open package"));

    let moved = ShapeBounds::from_inches(2.5, 1.25, 3.0, 0.75);
    pres.set_shape_bounds(0, 0, moved).expect("set bounds");
    assert_eq!(pres.shape_bounds(0, 0).expect("bounds"), Some(moved));

    let saved = pres.save().expect("save");
    only_these_parts_changed(&before, &saved, &["slide1.xml"]);

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(reopened.shape_bounds(0, 0).expect("bounds"), Some(moved));
}

#[test]
fn a_shape_with_no_transform_gets_one() {
    let mut pres = sample();
    let bounds = ShapeBounds::from_inches(1.0, 2.0, 4.0, 0.5);
    pres.set_shape_bounds(0, 0, bounds).expect("set bounds");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(reopened.shape_bounds(0, 0).expect("bounds"), Some(bounds));
}

#[test]
fn rotation_and_flips_survive_a_save() {
    let mut pres = sample();
    let idx = pres
        .add_shape(
            0,
            PresetShapeType::RoundedRectangle,
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
        )
        .expect("add shape");

    pres.set_shape_transform(
        0,
        idx,
        &Transform2D {
            rotation: Some(Angle::from_degrees(45.0)),
            flip_horizontal: Some(true),
            flip_vertical: Some(false),
            ..Transform2D::default()
        },
    )
    .expect("set transform");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    let transform = reopened
        .shape_transform(0, idx)
        .expect("transform")
        .expect("declared");

    assert!((transform.rotation.expect("rotation").degrees() - 45.0).abs() < 1e-6);
    assert_eq!(transform.flip_horizontal, Some(true));
    assert_eq!(transform.flip_vertical, Some(false));
    // The bounds `add_shape` wrote are untouched: an unset field means leave it alone.
    assert_eq!(
        reopened.shape_bounds(0, idx).expect("bounds"),
        Some(ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0))
    );
}

#[test]
fn setting_bounds_leaves_a_rotation_alone() {
    // The property that makes `set_shape_bounds` safe to call on any shape.
    let mut pres = sample();
    let idx = pres
        .add_text_box(0, "spun", ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0))
        .expect("add text box");
    pres.set_shape_transform(
        0,
        idx,
        &Transform2D {
            rotation: Some(Angle::from_degrees(90.0)),
            ..Transform2D::default()
        },
    )
    .expect("rotate");

    let moved = ShapeBounds::from_inches(3.0, 3.0, 1.0, 1.0);
    pres.set_shape_bounds(0, idx, moved).expect("move");

    let transform = pres
        .shape_transform(0, idx)
        .expect("transform")
        .expect("declared");
    assert_eq!(ShapeBounds::from_transform(&transform), Some(moved));
    assert!((transform.rotation.expect("rotation").degrees() - 90.0).abs() < 1e-6);
}

#[test]
fn editing_a_layout_moves_the_shape_for_every_slide_that_inherits_it() {
    let mut pres = layouts();
    let before = byte_map(&Package::open(&fixture("layouts.pptx")).expect("open package"));

    let moved = ShapeBounds::from_inches(0.5, 0.5, 4.0, 1.0);
    pres.set_shape_bounds(Surface::Layout(0), 0, moved)
        .expect("set layout bounds");

    let saved = pres.save().expect("save");
    only_these_parts_changed(&before, &saved, &["slideLayout1.xml"]);

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened
            .shape_bounds(Surface::Layout(0), 0)
            .expect("bounds"),
        Some(moved)
    );
}

#[test]
fn a_picture_is_positioned_like_any_other_shape() {
    const TINY_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44,
        0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F,
        0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78, 0x9C, 0x63, 0x00,
        0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, 0x00, 0x00, 0x00, 0x49,
        0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];
    let mut pres = sample();
    let idx = pres
        .add_picture(0, TINY_PNG, ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0))
        .expect("add picture");
    assert_eq!(pres.shape_kind(0, idx).expect("kind"), ShapeKind::Picture);

    let moved = ShapeBounds::from_inches(4.0, 0.5, 1.5, 1.5);
    pres.set_shape_bounds(0, idx, moved).expect("move picture");
    assert_eq!(pres.shape_bounds(0, idx).expect("bounds"), Some(moved));
}

// ---------------------------------------------------------------------------------------------
// The kinds whose transform is somewhere else
// ---------------------------------------------------------------------------------------------

/// The index of the first shape of `kind` on slide `slide`, or `None`.
fn find_kind(pres: &mut Presentation, slide: usize, kind: ShapeKind) -> Option<usize> {
    let count = pres.shape_count(slide).expect("count");
    (0..count).find(|&idx| pres.shape_kind(slide, idx).expect("kind") == kind)
}

#[test]
fn a_group_is_positioned_through_its_own_properties_element() {
    let mut pres = layouts();
    let idx = find_kind(&mut pres, 1, ShapeKind::GroupShape).expect("a group on slide 2");

    let transform = pres
        .shape_transform(1, idx)
        .expect("transform")
        .expect("the group places itself");
    assert!(
        transform.child_position.is_some() && transform.child_size.is_some(),
        "a group's transform carries the child coordinate space"
    );
}

#[test]
fn moving_a_group_keeps_the_child_space_its_members_live_in() {
    // The whole reason `apply` merges: rebuilding would drop `a:chOff`/`a:chExt` and drag every
    // member of the group along with it.
    let mut pres = layouts();
    let idx = find_kind(&mut pres, 1, ShapeKind::GroupShape).expect("a group on slide 2");
    let before = pres
        .shape_transform(1, idx)
        .expect("transform")
        .expect("declared");

    let moved = ShapeBounds::from_inches(4.0, 2.0, 2.0, 2.0);
    pres.set_shape_bounds(1, idx, moved).expect("move group");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    let after = reopened
        .shape_transform(1, idx)
        .expect("transform")
        .expect("declared");

    assert_eq!(ShapeBounds::from_transform(&after), Some(moved));
    assert_eq!(after.child_position, before.child_position);
    assert_eq!(after.child_size, before.child_size);
}

#[test]
fn a_graphic_frame_is_positioned_through_its_presentationml_transform() {
    let mut pres = layouts();
    let idx = find_kind(&mut pres, 1, ShapeKind::GraphicFrame).expect("a graphic frame on slide 2");

    assert!(
        pres.shape_bounds(1, idx).expect("bounds").is_some(),
        "a graphic frame's p:xfrm is required by the schema, so it always places itself"
    );

    let moved = ShapeBounds::from_inches(1.0, 3.0, 3.0, 1.0);
    pres.set_shape_bounds(1, idx, moved).expect("move frame");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(reopened.shape_bounds(1, idx).expect("bounds"), Some(moved));
}

#[test]
fn a_transform_names_only_what_the_file_states() {
    // A shape placed with `a:off`/`a:ext` and nothing else must not report a rotation of zero —
    // "unstated" and "zero" are different, and the inheritance walk depends on the difference.
    let mut pres = sample();
    let idx = pres
        .add_text_box(0, "plain", ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0))
        .expect("add text box");
    let transform = pres
        .shape_transform(0, idx)
        .expect("transform")
        .expect("declared");

    assert_eq!(
        transform.position,
        Some(Position::from_emu(914_400, 914_400))
    );
    assert_eq!(transform.size, Some(Size::from_emu(1_828_800, 914_400)));
    assert_eq!(transform.rotation, None);
    assert_eq!(transform.flip_horizontal, None);
    assert_eq!(transform.flip_vertical, None);
    assert_eq!(transform.child_position, None);
}
