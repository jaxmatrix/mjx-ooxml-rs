//! Integration tests for the unified shape-geometry surface (`shape_geometry` / `set_shape_geometry`)
//! carrying custom geometry (`a:custGeom`): author a freeform shape, read it back typed, convert
//! between preset / custom / inherited, and confirm fidelity (only the edited slide changes).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    CustomGeometrySpec, DrawCommand, Emu, Path2DSpec, PathFillMode, Point, ShapeGeometry,
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
