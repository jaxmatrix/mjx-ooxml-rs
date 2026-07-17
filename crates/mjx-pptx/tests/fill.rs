//! Integration tests for the shape-fill surface: read a shape's [`FillSpec`], set each fill kind,
//! and save — with fidelity (only the edited slide changes) and correct `p:spPr` placement.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{Angle, ColorSpec, FillSpec, Fraction, GradientStopSpec, PatternType, SchemeColor};
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
fn fresh_shape_has_no_explicit_fill() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    assert_eq!(pres.shape_fill(0, idx).expect("shape_fill"), None);
}

#[test]
fn set_solid_fill_reads_back_and_persists() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_fill(0, idx, &FillSpec::solid(ColorSpec::Srgb("FF0000".into())))
        .expect("set fill");

    assert_eq!(
        pres.shape_fill(0, idx).expect("shape_fill"),
        Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".into())))
    );

    // Survives a save/reopen, and the shape's geometry is intact (the fill didn't clobber spPr).
    let mut reread = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reread.shape_fill(0, idx).expect("shape_fill"),
        Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".into())))
    );
    assert!(matches!(
        reread.shape_geometry(0, idx).expect("geometry"),
        mjx_dml::ShapeGeometry::RoundedRectangle { .. }
    ));
}

#[test]
fn set_fill_replaces_an_existing_fill_in_place() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_fill(
        0,
        idx,
        &FillSpec::solid(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("set solid");
    // Overriding replaces the fill element rather than adding a second one.
    pres.set_shape_fill(
        0,
        idx,
        &FillSpec::pattern(
            PatternType::Percent25,
            ColorSpec::Srgb("000000".into()),
            ColorSpec::Srgb("FFFFFF".into()),
        ),
    )
    .expect("set pattern");

    assert_eq!(
        pres.shape_fill(0, idx).expect("shape_fill"),
        Some(FillSpec::Pattern {
            preset: Some(PatternType::Percent25),
            foreground: Some(ColorSpec::Srgb("000000".into())),
            background: Some(ColorSpec::Srgb("FFFFFF".into())),
        })
    );
}

#[test]
fn set_gradient_fill_reads_back_stops_and_angle() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    let spec = FillSpec::linear_gradient(
        vec![
            GradientStopSpec {
                position: Fraction::from_ratio(0.0),
                color: ColorSpec::Srgb("FF0000".into()),
            },
            GradientStopSpec {
                position: Fraction::from_ratio(1.0),
                color: ColorSpec::Scheme(SchemeColor::Accent2),
            },
        ],
        Angle::from_degrees(45.0),
    );
    pres.set_shape_fill(0, idx, &spec).expect("set gradient");
    assert_eq!(pres.shape_fill(0, idx).expect("shape_fill"), Some(spec));
}

#[test]
fn set_no_fill_writes_an_explicit_nofill() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_no_fill(0, idx).expect("set no fill");
    // An explicit a:noFill reads as Some(FillSpec::None) — distinct from an absent fill (None).
    assert_eq!(
        pres.shape_fill(0, idx).expect("shape_fill"),
        Some(FillSpec::None)
    );
}

#[test]
fn set_fill_keeps_other_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let idx = added_shape(&mut pres);
    pres.set_shape_fill(0, idx, &FillSpec::solid(ColorSpec::Srgb("00FF00".into())))
        .expect("set fill");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen package"));
    for (name, original) in &snapshot {
        if name.ends_with("slide1.xml") {
            continue; // the one part we edited
        }
        assert_eq!(
            reopened.get(name),
            Some(original),
            "fill edit dirtied unrelated part {name}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// effective_shape_fill — the shape's rendered fill, resolved to concrete RGB
// ---------------------------------------------------------------------------------------------

#[test]
fn effective_fill_resolves_a_scheme_color_against_the_theme() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    // A shape filled with a theme scheme color...
    pres.set_shape_fill(
        0,
        idx,
        &FillSpec::solid(ColorSpec::Scheme(SchemeColor::Accent1)),
    )
    .expect("set fill");
    // ...resolves to the fixture theme's concrete accent1 (4472C4).
    assert_eq!(
        pres.effective_shape_fill(0, idx).expect("effective fill"),
        Some(FillSpec::Solid(ColorSpec::Srgb("4472C4".into())))
    );
}

#[test]
fn effective_fill_keeps_explicit_srgb_and_baked_gradient() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    // An explicit sRGB fill resolves to itself.
    pres.set_shape_fill(0, idx, &FillSpec::solid(ColorSpec::Srgb("112233".into())))
        .expect("set fill");
    assert_eq!(
        pres.effective_shape_fill(0, idx).expect("effective fill"),
        Some(FillSpec::Solid(ColorSpec::Srgb("112233".into())))
    );

    // A gradient with a scheme stop resolves that stop to concrete RGB.
    let grad = added_shape(&mut pres);
    pres.set_shape_fill(
        0,
        grad,
        &FillSpec::linear_gradient(
            vec![
                GradientStopSpec {
                    position: Fraction::from_ratio(0.0),
                    color: ColorSpec::Srgb("FF0000".into()),
                },
                GradientStopSpec {
                    position: Fraction::from_ratio(1.0),
                    color: ColorSpec::Scheme(SchemeColor::Accent1),
                },
            ],
            Angle::from_degrees(0.0),
        ),
    )
    .expect("set gradient");
    let Some(FillSpec::Gradient { stops, .. }) =
        pres.effective_shape_fill(0, grad).expect("effective fill")
    else {
        panic!("expected a gradient");
    };
    assert_eq!(stops[0].color, ColorSpec::Srgb("FF0000".into()));
    assert_eq!(stops[1].color, ColorSpec::Srgb("4472C4".into()));
}

#[test]
fn effective_fill_is_none_when_the_shape_declares_no_fill() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = added_shape(&mut pres);
    // A fresh autoshape has no explicit fill and no p:style; inherited fill is a later step.
    assert_eq!(
        pres.effective_shape_fill(0, idx).expect("effective fill"),
        None
    );
}

#[test]
fn reading_effective_fill_keeps_all_parts_byte_identical() {
    let bytes = fixture("sample.pptx");
    let snapshot = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Reading the title's effective fill navigates slide->layout->master->theme without dirtying.
    let _ = pres.effective_shape_fill(0, 0).expect("effective fill");
    let saved = pres.save().expect("save");

    let reopened = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, original) in &snapshot {
        assert_eq!(
            reopened.get(name),
            Some(original),
            "reading effective fill dirtied part {name}"
        );
    }
}
