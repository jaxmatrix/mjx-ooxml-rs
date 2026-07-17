//! Integration tests for the shape-geometry surface: read a shape's typed `ShapeGeometry`, edit it,
//! add a preset-geometry shape, and save — with fidelity (only the edited slide changes).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{Fraction, ShapeGeometry};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_opc::Package;
use mjx_pptx::{Presentation, ShapeBounds};

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
    ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0)
}

#[test]
fn added_text_box_reports_rectangle_geometry() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres.add_text_box(0, "hi", bounds()).expect("add text box");
    // A text box is `prst="rect"` — a parameterless shape → the Unmodeled catch-all.
    assert_eq!(
        pres.shape_geometry(0, idx).expect("geometry"),
        ShapeGeometry::Unmodeled(PresetShapeType::Rectangle)
    );
}

#[test]
fn add_shape_creates_named_geometry_with_defaults() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape");

    // Default corner radius is the spec default (16667/100000).
    let ShapeGeometry::RoundedRectangle { corner_radius } =
        pres.shape_geometry(0, idx).expect("geometry")
    else {
        panic!("expected RoundedRectangle");
    };
    assert!((corner_radius.ratio() - 0.16667).abs() < 1e-9);

    // It survives a save/reopen.
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert!(matches!(
        reread.shape_geometry(0, idx).expect("geometry"),
        ShapeGeometry::RoundedRectangle { .. }
    ));
}

#[test]
fn set_shape_geometry_round_trips_and_keeps_other_parts_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let idx = pres.add_text_box(0, "hi", bounds()).expect("add text box");
    pres.set_shape_geometry(
        0,
        idx,
        ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.25),
        },
    )
    .expect("set geometry");
    let saved = pres.save().expect("save");

    // The edit landed and reads back exactly.
    let mut reread = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reread.shape_geometry(0, idx).expect("geometry"),
        ShapeGeometry::RoundedRectangle {
            corner_radius: Fraction::from_ratio(0.25)
        }
    );

    // Fidelity: only the edited slide changed; every other pre-existing part is byte-identical.
    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    for (name, original) in &snapshot {
        if name.ends_with("slide1.xml") {
            continue; // the one part we edited
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "geometry edit dirtied unrelated part {name}"
        );
    }
}
