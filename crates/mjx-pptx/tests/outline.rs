//! Integration tests for the shape-outline surface: read a shape's [`LineSpec`], set an outline,
//! and save — with fidelity (only the edited slide changes) and correct `p:spPr` placement (after the
//! fill, before effects).

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    ColorSpec, CompoundLine, FillSpec, LineCap, LineDash, LineEnd, LineEndLength, LineEndType,
    LineEndWidth, LineJoin, LineSpec, LineWidth, PresetLineDash,
};
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

/// Adds a fresh autoshape and returns its shape index.
fn added_shape(pres: &mut Presentation) -> usize {
    pres.add_shape(0, PresetShapeType::RoundedRectangle, bounds())
        .expect("add shape")
}

#[test]
fn fresh_shape_has_no_outline() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    assert_eq!(pres.shape_outline(0, idx).expect("shape_outline"), None);
}

#[test]
fn set_solid_outline_reads_back_and_persists() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    let spec = LineSpec::solid(
        LineWidth::from_points(2.0),
        ColorSpec::Srgb("FF0000".into()),
    );
    pres.set_shape_outline(0, idx, &spec).expect("set outline");

    assert_eq!(
        pres.shape_outline(0, idx).expect("shape_outline"),
        Some(spec.clone())
    );

    // Survives a save/reopen, and the shape's geometry is intact (the outline didn't clobber spPr).
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_outline(0, idx).expect("shape_outline"),
        Some(spec)
    );
    assert!(matches!(
        reread.shape_geometry(0, idx).expect("geometry"),
        mjx_dml::ShapeGeometry::RoundedRectangle { .. }
    ));
}

#[test]
fn set_outline_replaces_an_existing_outline_in_place() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_outline(
        0,
        idx,
        &LineSpec::solid(
            LineWidth::from_points(1.0),
            ColorSpec::Srgb("000000".into()),
        ),
    )
    .expect("set first outline");
    // Overriding replaces the a:ln element rather than adding a second one.
    let second = LineSpec::solid(
        LineWidth::from_points(3.0),
        ColorSpec::Srgb("00FF00".into()),
    );
    pres.set_shape_outline(0, idx, &second)
        .expect("set second outline");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_outline(0, idx).expect("shape_outline"),
        Some(second)
    );
}

#[test]
fn set_rich_outline_round_trips_through_save() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    let spec = LineSpec {
        width: Some(LineWidth::from_points(2.5)),
        cap: Some(LineCap::Round),
        compound: Some(CompoundLine::Single),
        pen_alignment: None,
        fill: Some(FillSpec::Solid(ColorSpec::Srgb("112233".into()))),
        dash: Some(LineDash::Preset(PresetLineDash::Dash)),
        join: Some(LineJoin::Round),
        head_end: Some(LineEnd {
            kind: Some(LineEndType::Triangle),
            width: Some(LineEndWidth::Medium),
            length: Some(LineEndLength::Medium),
        }),
        tail_end: Some(LineEnd {
            kind: Some(LineEndType::Arrow),
            width: None,
            length: None,
        }),
    };
    pres.set_shape_outline(0, idx, &spec).expect("set outline");

    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_outline(0, idx).expect("shape_outline"),
        Some(spec)
    );
}

#[test]
fn set_no_outline_writes_an_ln_with_nofill() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_no_outline(0, idx).expect("set no outline");
    // An explicit <a:ln><a:noFill/></a:ln> reads back as an outline whose fill is None.
    assert_eq!(
        pres.shape_outline(0, idx).expect("shape_outline"),
        Some(LineSpec {
            fill: Some(FillSpec::None),
            ..LineSpec::new()
        })
    );
}

#[test]
fn outline_and_fill_coexist_on_one_shape() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_fill(0, idx, &FillSpec::solid(ColorSpec::Srgb("FFFF00".into())))
        .expect("set fill");
    pres.set_shape_outline(
        0,
        idx,
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("FF0000".into()),
        ),
    )
    .expect("set outline");

    // Both slots read back after a save/reopen — the two spPr children don't collide.
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_fill(0, idx).expect("shape_fill"),
        Some(FillSpec::Solid(ColorSpec::Srgb("FFFF00".into())))
    );
    assert_eq!(
        reread.shape_outline(0, idx).expect("shape_outline"),
        Some(LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("FF0000".into())
        ))
    );
}

#[test]
fn set_outline_keeps_other_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_outline(
        0,
        idx,
        &LineSpec::solid(
            LineWidth::from_points(2.0),
            ColorSpec::Srgb("00FF00".into()),
        ),
    )
    .expect("set outline");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    for (name, original) in &snapshot {
        if name.ends_with("slide1.xml") {
            continue; // the one part we edited
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "outline edit dirtied unrelated part {name}"
        );
    }
}
