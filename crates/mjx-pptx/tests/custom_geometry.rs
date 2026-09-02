//! Integration tests for the unified shape-geometry surface (`shape_geometry` / `set_shape_geometry`)
//! carrying custom geometry (`a:custGeom`): author a freeform shape, read it back typed, convert
//! between preset / custom / inherited, resolve the guide formulas a coordinate may name, and
//! confirm fidelity (only the edited slide changes).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    AdjustCoordinate, CustomGeometrySpec, DrawCommand, Emu, GuideContext, GuideSpec, Path2DSpec,
    PathFillMode, Point, ShapeGeometry, Size,
};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_opc::Package;
use mjx_pptx::{Geometry, Presentation, ShapeBounds};

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

fn bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0)
}

/// A triangle drawn as a custom geometry: move to the top, line to two corners, close.
fn triangle() -> CustomGeometrySpec {
    CustomGeometrySpec {
        paths: vec![Path2DSpec {
            width: Some(Emu::from_emu(1_828_800)),
            height: Some(Emu::from_emu(1_828_800)),
            fill: Some(PathFillMode::Normal),
            commands: vec![
                DrawCommand::MoveTo(Point::from_emu(914_400, 0)),
                DrawCommand::LineTo(Point::from_emu(1_828_800, 1_828_800)),
                DrawCommand::LineTo(Point::from_emu(0, 1_828_800)),
                DrawCommand::Close,
            ],
            ..Path2DSpec::default()
        }],
        ..CustomGeometrySpec::default()
    }
}

#[test]
fn authoring_custom_geometry_reads_back_and_survives_save() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres.add_text_box(0, "hi", bounds()).expect("add text box");

    // A freshly added text box is a preset rectangle.
    assert!(matches!(
        pres.shape_geometry(0, idx).expect("geometry"),
        Geometry::Preset(_)
    ));

    // Replace it with a custom triangle; it reads back as the same spec.
    pres.set_shape_geometry(0, idx, Geometry::Custom(triangle()))
        .expect("set custom geometry");
    assert_eq!(
        pres.shape_geometry(0, idx).expect("geometry"),
        Geometry::Custom(triangle())
    );

    // And it survives a save / reopen.
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_geometry(0, idx).expect("geometry"),
        Geometry::Custom(triangle())
    );
}

#[test]
fn setting_custom_geometry_keeps_other_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let idx = pres.add_text_box(0, "hi", bounds()).expect("add text box");
    pres.set_shape_geometry(0, idx, Geometry::Custom(triangle()))
        .expect("set custom geometry");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    for (name, original) in &snapshot {
        if name.ends_with("slide1.xml") {
            continue; // the one part we edited
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "part {name} changed but should be byte-identical"
        );
    }
}

#[test]
fn reading_does_not_dirty_the_part() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Reading geometry on the (preset) shapes must not mark anything dirty.
    for idx in 0..pres.shape_count(0).expect("count") {
        let _ = pres.shape_geometry(0, idx);
    }
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    assert_eq!(reopened, snapshot, "a read-only pass changed the package");
}

#[test]
fn geometry_converts_between_preset_custom_and_inherited() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape");

    // preset → custom
    pres.set_shape_geometry(0, idx, Geometry::Custom(triangle()))
        .expect("to custom");
    assert_eq!(
        pres.shape_geometry(0, idx).expect("geometry"),
        Geometry::Custom(triangle())
    );

    // custom → preset (the custom element is dropped, a prstGeom takes its place)
    pres.set_shape_geometry(
        0,
        idx,
        Geometry::Preset(ShapeGeometry::Unmodeled(PresetShapeType::Diamond)),
    )
    .expect("to preset");
    assert_eq!(
        pres.shape_geometry(0, idx).expect("geometry"),
        Geometry::Preset(ShapeGeometry::Unmodeled(PresetShapeType::Diamond))
    );

    // preset → inherited (the geometry element is removed entirely)
    pres.set_shape_geometry(0, idx, Geometry::Inherited)
        .expect("to inherited");
    assert_eq!(
        pres.shape_geometry(0, idx).expect("geometry"),
        Geometry::Inherited
    );

    // The whole round of edits still saves and reopens cleanly.
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_geometry(0, idx).expect("geometry"),
        Geometry::Inherited
    );
}

// ---------------------------------------------------------------------------------------------
// Resolving the guide formulas a geometry is expressed in
// ---------------------------------------------------------------------------------------------

/// A two-inch square, the size the fixture shapes are authored at.
fn geometry_size() -> GuideContext {
    GuideContext::from_size(Size::from_emu(1_828_800, 1_828_800))
}

/// The same triangle, but with its apex placed by a guide instead of a number: `apex = */ w 1 2`.
fn guide_driven_triangle() -> CustomGeometrySpec {
    CustomGeometrySpec {
        guides: vec![GuideSpec {
            name: "apex".to_owned(),
            formula: "*/ w 1 2".to_owned(),
        }],
        paths: vec![Path2DSpec {
            commands: vec![
                DrawCommand::MoveTo(Point {
                    x: AdjustCoordinate::Guide("apex".to_owned()),
                    y: AdjustCoordinate::Emu(Emu::from_emu(0)),
                }),
                DrawCommand::LineTo(Point {
                    x: AdjustCoordinate::Guide("r".to_owned()),
                    y: AdjustCoordinate::Guide("b".to_owned()),
                }),
                DrawCommand::LineTo(Point {
                    x: AdjustCoordinate::Guide("l".to_owned()),
                    y: AdjustCoordinate::Guide("b".to_owned()),
                }),
                DrawCommand::Close,
            ],
            ..Path2DSpec::default()
        }],
        ..CustomGeometrySpec::default()
    }
}

#[test]
fn a_guide_driven_custom_geometry_reads_back_as_numbers() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres.add_text_box(0, "hi", bounds()).expect("add text box");
    pres.set_shape_geometry(0, idx, Geometry::Custom(guide_driven_triangle()))
        .expect("set custom geometry");

    let Geometry::Custom(spec) = pres.shape_geometry(0, idx).expect("geometry") else {
        panic!("expected a custom geometry");
    };
    // The formula is still a formula on the wire — reading it back gives what was written.
    assert_eq!(spec.guides[0].formula, "*/ w 1 2");

    let resolved = spec.resolve(geometry_size()).expect("resolves");
    let commands = &resolved.paths[0].commands;
    let mjx_dml::ResolvedDrawCommand::MoveTo(apex) = commands[0] else {
        panic!("expected a moveTo");
    };
    assert_eq!(apex.x, Emu::from_emu(914_400), "half the shape width");
    let mjx_dml::ResolvedDrawCommand::LineTo(corner) = commands[1] else {
        panic!("expected a lnTo");
    };
    assert_eq!(corner.x, Emu::from_emu(1_828_800), "the right edge");
    assert_eq!(corner.y, Emu::from_emu(1_828_800), "the bottom edge");
}

#[test]
fn a_preset_shapes_adjustments_resolve_to_numbers_and_a_domain() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres.add_text_box(0, "hi", bounds()).expect("add text box");
    pres.set_shape_geometry(
        0,
        idx,
        Geometry::Preset(ShapeGeometry::Unmodeled(PresetShapeType::Chevron)),
    )
    .expect("set preset geometry");

    // A square shape: `ss` is the width, so chevron's `maxAdj = */ 100000 w ss` is 100000.
    let adjustments = pres
        .shape_adjustments(0, idx, geometry_size())
        .expect("adjustments resolve");
    assert_eq!(adjustments.len(), 1);
    assert_eq!(adjustments[0].spec.wire_name, "adj");
    assert!((adjustments[0].value - 50_000.0).abs() < 1e-9);
    assert!((adjustments[0].minimum - 0.0).abs() < 1e-9);
    assert!((adjustments[0].maximum - 100_000.0).abs() < 1e-9);

    // Twice as wide as it is tall, `ss` is the height and the domain doubles.
    let wide = GuideContext::from_size(Size::from_emu(3_657_600, 1_828_800));
    let adjustments = pres
        .shape_adjustments(0, idx, wide)
        .expect("adjustments resolve");
    assert!((adjustments[0].maximum - 200_000.0).abs() < 1e-9);
}

#[test]
fn a_shape_with_no_preset_geometry_has_no_adjustments() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres.add_text_box(0, "hi", bounds()).expect("add text box");
    pres.set_shape_geometry(0, idx, Geometry::Custom(triangle()))
        .expect("set custom geometry");
    assert!(pres
        .shape_adjustments(0, idx, geometry_size())
        .expect("resolves")
        .is_empty());
}

#[test]
fn resolving_a_shapes_geometry_keeps_every_part_byte_identical() {
    // Resolution is a read: the whole package must come back unchanged, the edited slide included.
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    for idx in 0..pres.shape_count(0).expect("count") {
        let _ = pres.shape_adjustments(0, idx, geometry_size());
        if let Ok(Geometry::Custom(spec)) = pres.shape_geometry(0, idx) {
            let _ = spec.resolve(geometry_size());
        }
    }
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    assert_eq!(reopened, snapshot, "resolving changed the package");
}
