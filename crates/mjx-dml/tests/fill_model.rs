//! Unit tests for the exhaustive DrawingML fill model, through the public API only. Every round-trip
//! assertion is paired with a structural one so byte-identity can't pass by dumping everything into an
//! opaque bucket; typed reads and builders are checked against expected values/bytes.

use mjx_dml::{
    Angle, BlipFill, BlipFillMode, Color, ColorKind, ColorSpec, Fill, FillSpec, Fraction,
    GradientFill, GradientStopSpec, NoFill, PatternFill, PatternType, SchemeColor,
};
use mjx_dml::{GroupFill, SolidFill};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";
const R: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships";

fn parse_typed<T: FromXml>(fragment: &[u8]) -> (T, RawDocument) {
    let doc = fidelity::parse(fragment).expect("fragment parses");
    let typed = T::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (typed, doc)
}

#[track_caller]
fn assert_round_trips<T: ToXml>(typed: &T, mut doc: RawDocument, expected: &[u8]) {
    doc.root = typed.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        String::from_utf8_lossy(expected),
        "round-trip byte mismatch"
    );
}

fn serialize_built<T: ToXml>(mut interner: Interner, typed: &T) -> String {
    let root = typed.to_xml(&mut interner);
    let doc = RawDocument {
        interner,
        bom: false,
        prologue: Vec::new(),
        root,
        epilogue: Vec::new(),
    };
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

// ---------------------------------------------------------------------------------------------
// noFill / grpFill
// ---------------------------------------------------------------------------------------------

#[test]
fn no_fill_and_group_fill_round_trip() {
    let fragment = format!(r#"<a:noFill xmlns:a="{A}"/>"#);
    let (fill, doc): (NoFill, _) = parse_typed(fragment.as_bytes());
    assert_round_trips(&fill, doc, fragment.as_bytes());

    let fragment = format!(r#"<a:grpFill xmlns:a="{A}"/>"#);
    let (fill, doc): (GroupFill, _) = parse_typed(fragment.as_bytes());
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

#[test]
fn builds_no_fill_and_group_fill() {
    let mut interner = Interner::new();
    let no_fill = NoFill::new(&mut interner);
    assert_eq!(serialize_built(interner, &no_fill), r#"<a:noFill/>"#);

    let mut interner = Interner::new();
    let group_fill = GroupFill::new(&mut interner);
    assert_eq!(serialize_built(interner, &group_fill), r#"<a:grpFill/>"#);
}

// ---------------------------------------------------------------------------------------------
// gradFill
// ---------------------------------------------------------------------------------------------

#[test]
fn gradient_fill_reads_stops_angle_and_preserves_internals() {
    // Two stops + a linear shade + an opaque tileRect; gradient-level attributes.
    let fragment = format!(
        r#"<a:gradFill xmlns:a="{A}" flip="none" rotWithShape="1"><a:gsLst><a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs><a:gs pos="100000"><a:schemeClr val="accent1"/></a:gs></a:gsLst><a:lin ang="5400000" scaled="1"/><a:tileRect l="1000"/></a:gradFill>"#
    );
    let (fill, doc): (GradientFill, _) = parse_typed(fragment.as_bytes());

    let stops = fill.stops(&doc.interner);
    assert_eq!(stops.len(), 2);
    assert!((stops[0].position.ratio() - 0.0).abs() < 1e-9);
    assert_eq!(stops[0].color.kind(&doc.interner), ColorKind::Srgb);
    assert_eq!(stops[0].color.hex(&doc.interner), Some("FF0000"));
    assert!((stops[1].position.ratio() - 1.0).abs() < 1e-9);
    assert_eq!(
        stops[1].color.scheme_color(&doc.interner),
        Some(SchemeColor::Accent1)
    );

    let angle = fill.linear_angle(&doc.interner).expect("linear angle");
    assert!((angle.degrees() - 90.0).abs() < 1e-9);

    assert_eq!(fill.flip(&doc.interner), Some("none"));
    assert_eq!(fill.rot_with_shape(&doc.interner), Some(true));

    // The opaque tileRect and the `scaled` attribute survive verbatim.
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

#[test]
fn builds_linear_gradient() {
    let mut interner = Interner::new();
    let start = Color::srgb(&mut interner, "FF0000");
    let end = Color::srgb(&mut interner, "0000FF");
    let gradient = GradientFill::linear(
        &mut interner,
        &[
            (mjx_dml::Fraction::from_ratio(0.0), start),
            (mjx_dml::Fraction::from_ratio(1.0), end),
        ],
        mjx_dml::Angle::from_degrees(90.0),
    );
    assert_eq!(
        serialize_built(interner, &gradient),
        r#"<a:gradFill><a:gsLst><a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs><a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs></a:gsLst><a:lin ang="5400000"/></a:gradFill>"#
    );
}

// ---------------------------------------------------------------------------------------------
// blipFill
// ---------------------------------------------------------------------------------------------

#[test]
fn blip_fill_reads_rel_id_mode_and_preserves_effects() {
    // r:embed + an opaque blip effect + a stretch fill mode.
    let fragment = format!(
        r#"<a:blipFill xmlns:a="{A}" xmlns:r="{R}"><a:blip r:embed="rId2"><a:alphaModFix amt="50000"/></a:blip><a:stretch><a:fillRect/></a:stretch></a:blipFill>"#
    );
    let (fill, doc): (BlipFill, _) = parse_typed(fragment.as_bytes());

    assert_eq!(fill.image_rel_id(&doc.interner), Some("rId2"));
    assert_eq!(fill.image_link_id(&doc.interner), None);
    assert_eq!(fill.mode(&doc.interner), BlipFillMode::Stretch);

    // The alphaModFix effect and the stretch's fillRect survive verbatim.
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

#[test]
fn builds_blip_fill() {
    let mut interner = Interner::new();
    let blip = BlipFill::new(&mut interner, "rId5", BlipFillMode::Stretch);
    assert_eq!(
        serialize_built(interner, &blip),
        r#"<a:blipFill><a:blip r:embed="rId5"/><a:stretch/></a:blipFill>"#
    );

    let mut interner = Interner::new();
    let tiled = BlipFill::new(&mut interner, "rId6", BlipFillMode::Tile);
    assert_eq!(
        serialize_built(interner, &tiled),
        r#"<a:blipFill><a:blip r:embed="rId6"/><a:tile/></a:blipFill>"#
    );
}

// ---------------------------------------------------------------------------------------------
// pattFill
// ---------------------------------------------------------------------------------------------

#[test]
fn pattern_fill_reads_preset_and_colors() {
    let fragment = format!(
        r#"<a:pattFill xmlns:a="{A}" prst="pct25"><a:fgClr><a:srgbClr val="000000"/></a:fgClr><a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr></a:pattFill>"#
    );
    let (fill, doc): (PatternFill, _) = parse_typed(fragment.as_bytes());

    assert_eq!(fill.preset(&doc.interner), Some(PatternType::Percent25));
    assert_eq!(
        fill.foreground(&doc.interner).unwrap().hex(&doc.interner),
        Some("000000")
    );
    assert_eq!(
        fill.background(&doc.interner).unwrap().hex(&doc.interner),
        Some("FFFFFF")
    );
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

#[test]
fn builds_pattern_fill() {
    let mut interner = Interner::new();
    let fg = Color::srgb(&mut interner, "000000");
    let bg = Color::scheme(&mut interner, SchemeColor::Background1);
    let pattern = PatternFill::new(&mut interner, PatternType::DiagonalCross, fg, bg);
    assert_eq!(
        serialize_built(interner, &pattern),
        r#"<a:pattFill prst="diagCross"><a:fgClr><a:srgbClr val="000000"/></a:fgClr><a:bgClr><a:schemeClr val="bg1"/></a:bgClr></a:pattFill>"#
    );
}

// ---------------------------------------------------------------------------------------------
// Fill (the exhaustive choice)
// ---------------------------------------------------------------------------------------------

/// Parses `fragment` as a [`Fill`], asserts `is_variant` holds for the dispatched variant, and
/// asserts the wrapper round-trips byte-for-byte.
#[track_caller]
fn assert_fill_variant(fragment: &str, is_variant: impl Fn(&Fill) -> bool) {
    let (fill, doc): (Fill, _) = parse_typed(fragment.as_bytes());
    assert!(is_variant(&fill), "wrong variant for {fragment}");
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

#[test]
fn fill_dispatches_on_local_name_for_all_six_kinds() {
    assert_fill_variant(&format!(r#"<a:noFill xmlns:a="{A}"/>"#), |f| {
        matches!(f, Fill::None(_))
    });
    assert_fill_variant(
        &format!(r#"<a:solidFill xmlns:a="{A}"><a:srgbClr val="FF0000"/></a:solidFill>"#),
        |f| matches!(f, Fill::Solid(_)),
    );
    assert_fill_variant(
        &format!(r#"<a:gradFill xmlns:a="{A}"><a:gsLst/></a:gradFill>"#),
        |f| matches!(f, Fill::Gradient(_)),
    );
    assert_fill_variant(
        &format!(r#"<a:blipFill xmlns:a="{A}"><a:blip/></a:blipFill>"#),
        |f| matches!(f, Fill::Blip(_)),
    );
    assert_fill_variant(
        &format!(r#"<a:pattFill xmlns:a="{A}" prst="pct5"/>"#),
        |f| matches!(f, Fill::Pattern(_)),
    );
    assert_fill_variant(&format!(r#"<a:grpFill xmlns:a="{A}"/>"#), |f| {
        matches!(f, Fill::Group(_))
    });
}

#[test]
fn fill_recognizes_its_element_locals() {
    for local in [
        "noFill",
        "solidFill",
        "gradFill",
        "blipFill",
        "pattFill",
        "grpFill",
    ] {
        assert!(Fill::is_fill_local(local), "{local} should be a fill local");
    }
    assert!(!Fill::is_fill_local("ln"));
}

#[test]
fn fill_solid_variant_exposes_its_color() {
    let fragment = format!(r#"<a:solidFill xmlns:a="{A}"><a:srgbClr val="112233"/></a:solidFill>"#);
    let (fill, doc): (Fill, _) = parse_typed(fragment.as_bytes());
    let Fill::Solid(solid) = &fill else {
        panic!("expected a solid fill");
    };
    let _: &SolidFill = solid;
    assert_eq!(solid.color().unwrap().hex(&doc.interner), Some("112233"));
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// FillSpec / ColorSpec (the interner-free description)
// ---------------------------------------------------------------------------------------------

/// Round-trips a [`FillSpec`] through the fidelity model: `spec → to_fill → Fill → spec`.
#[track_caller]
fn assert_spec_round_trips(spec: FillSpec) {
    let mut interner = Interner::new();
    let fill = spec.to_fill(&mut interner);
    assert_eq!(fill.spec(&interner), spec, "FillSpec round-trip mismatch");
}

#[test]
fn fill_spec_round_trips_each_kind() {
    assert_spec_round_trips(FillSpec::None);
    assert_spec_round_trips(FillSpec::Group);
    assert_spec_round_trips(FillSpec::solid(ColorSpec::Srgb("FF0000".into())));
    assert_spec_round_trips(FillSpec::solid(ColorSpec::Scheme(SchemeColor::Accent1)));
    // A non-first-class color kind survives as Other with its raw val.
    assert_spec_round_trips(FillSpec::solid(ColorSpec::Other {
        kind: ColorKind::System,
        value: Some("windowText".into()),
    }));
    assert_spec_round_trips(FillSpec::linear_gradient(
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
        Angle::from_degrees(90.0),
    ));
    assert_spec_round_trips(FillSpec::Blip {
        rel_id: "rId7".into(),
        mode: BlipFillMode::Stretch,
    });
    assert_spec_round_trips(FillSpec::pattern(
        PatternType::Percent25,
        ColorSpec::Srgb("000000".into()),
        ColorSpec::Srgb("FFFFFF".into()),
    ));
}

#[test]
fn solid_fill_reads_via_spec() {
    let fragment =
        format!(r#"<a:solidFill xmlns:a="{A}"><a:schemeClr val="accent3"/></a:solidFill>"#);
    let (fill, doc): (Fill, _) = parse_typed(fragment.as_bytes());
    assert_eq!(
        fill.spec(&doc.interner),
        FillSpec::Solid(ColorSpec::Scheme(SchemeColor::Accent3))
    );
}

#[test]
fn color_less_solid_fill_reads_and_rebuilds_empty() {
    // An empty solidFill has no color: it reads as Other/Unknown and rebuilds as `<a:solidFill/>`.
    let mut interner = Interner::new();
    let spec = FillSpec::Solid(ColorSpec::Other {
        kind: ColorKind::Unknown,
        value: None,
    });
    let fill = spec.to_fill(&mut interner);
    assert_eq!(fill.spec(&interner), spec);
    let Fill::Solid(solid) = &fill else {
        panic!("expected a solid fill");
    };
    assert!(solid.color().is_none());
}

#[test]
fn fill_spec_gradient_without_linear_shade_has_no_angle() {
    // A gradient with stops but no linear angle round-trips with angle == None.
    let spec = FillSpec::Gradient {
        stops: vec![
            GradientStopSpec {
                position: Fraction::from_ratio(0.0),
                color: ColorSpec::Srgb("112233".into()),
            },
            GradientStopSpec {
                position: Fraction::from_ratio(1.0),
                color: ColorSpec::Srgb("445566".into()),
            },
        ],
        angle: None,
    };
    assert_spec_round_trips(spec);
}
