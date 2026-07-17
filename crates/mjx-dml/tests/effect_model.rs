//! Unit tests for the DrawingML effect-list model, through the public API only. Every round-trip
//! assertion is paired with a structural/typed one so byte-identity can't pass by dumping everything
//! into an opaque bucket; typed reads and the `EffectListSpec` builder are checked against expected
//! values/bytes.

use mjx_dml::{
    Angle, BlendMode, BlurEffect, ColorSpec, EffectList, EffectListSpec, Emu, FillOverlayEffect,
    FillSpec, Fraction, GlowEffect, InnerShadowEffect, OuterShadowEffect, PresetShadow,
    PresetShadowEffect, RectangleAlignment, ReflectionEffect, SoftEdgeEffect,
};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, ToXml};
use mjx_xml::fidelity;

const A: &str = "http://schemas.openxmlformats.org/drawingml/2006/main";

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

/// A maximal effect-list spec exercising all eight effects — shared by the value-tier and
/// builder-byte tests. Its `to_effect_list` output is a fixed point of `spec()`.
fn full_spec() -> EffectListSpec {
    EffectListSpec {
        blur: Some(BlurEffect {
            radius: Some(Emu::from_emu(50_800)),
            grow: Some(false),
        }),
        fill_overlay: Some(FillOverlayEffect {
            fill: FillSpec::Solid(ColorSpec::Srgb("FFFF00".to_owned())),
            blend: BlendMode::Over,
        }),
        glow: Some(GlowEffect {
            color: ColorSpec::Srgb("FF0000".to_owned()),
            radius: Some(Emu::from_emu(63_500)),
        }),
        inner_shadow: Some(InnerShadowEffect {
            color: ColorSpec::Srgb("000000".to_owned()),
            blur_radius: Some(Emu::from_emu(40_000)),
            distance: Some(Emu::from_emu(20_000)),
            direction: Some(Angle::from_degrees(45.0)),
        }),
        outer_shadow: Some(OuterShadowEffect {
            color: ColorSpec::Srgb("808080".to_owned()),
            blur_radius: Some(Emu::from_emu(50_000)),
            distance: Some(Emu::from_emu(38_100)),
            direction: Some(Angle::from_degrees(45.0)),
            scale_x: Some(Fraction::from_ratio(0.9)),
            scale_y: Some(Fraction::from_ratio(0.9)),
            skew_x: Some(Angle::from_degrees(10.0)),
            skew_y: None,
            alignment: Some(RectangleAlignment::TopLeft),
            rotate_with_shape: Some(false),
        }),
        preset_shadow: Some(PresetShadowEffect {
            preset: PresetShadow::Shadow13,
            color: ColorSpec::Srgb("C0C0C0".to_owned()),
            distance: Some(Emu::from_emu(45_000)),
            direction: Some(Angle::from_degrees(45.0)),
        }),
        reflection: Some(ReflectionEffect {
            blur_radius: Some(Emu::from_emu(6_350)),
            start_alpha: Some(Fraction::from_ratio(0.5)),
            start_position: Some(Fraction::from_ratio(0.0)),
            end_alpha: Some(Fraction::from_ratio(0.3)),
            end_position: Some(Fraction::from_ratio(0.9)),
            distance: Some(Emu::from_emu(60_000)),
            direction: Some(Angle::from_degrees(90.0)),
            fade_direction: Some(Angle::from_degrees(45.0)),
            scale_x: Some(Fraction::from_ratio(1.0)),
            scale_y: Some(Fraction::from_ratio(-1.0)),
            skew_x: None,
            skew_y: None,
            alignment: Some(RectangleAlignment::BottomLeft),
            rotate_with_shape: Some(false),
        }),
        soft_edge: Some(SoftEdgeEffect {
            radius: Emu::from_emu(112_500),
        }),
    }
}

/// The exact bytes `full_spec()` builds (no namespace declaration — the built element carries only
/// the `a:` prefix), in `CT_EffectList` schema order.
const FULL_BYTES: &str = concat!(
    r#"<a:effectLst>"#,
    r#"<a:blur rad="50800" grow="false"/>"#,
    r#"<a:fillOverlay blend="over"><a:solidFill><a:srgbClr val="FFFF00"/></a:solidFill></a:fillOverlay>"#,
    r#"<a:glow rad="63500"><a:srgbClr val="FF0000"/></a:glow>"#,
    r#"<a:innerShdw blurRad="40000" dist="20000" dir="2700000"><a:srgbClr val="000000"/></a:innerShdw>"#,
    r#"<a:outerShdw blurRad="50000" dist="38100" dir="2700000" sx="90000" sy="90000" kx="600000" algn="tl" rotWithShape="false"><a:srgbClr val="808080"/></a:outerShdw>"#,
    r#"<a:prstShdw prst="shdw13" dist="45000" dir="2700000"><a:srgbClr val="C0C0C0"/></a:prstShdw>"#,
    r#"<a:reflection blurRad="6350" stA="50000" stPos="0" endA="30000" endPos="90000" dist="60000" dir="5400000" fadeDir="2700000" sx="100000" sy="-100000" algn="bl" rotWithShape="false"/>"#,
    r#"<a:softEdge rad="112500"/>"#,
    r#"</a:effectLst>"#
);

// ---------------------------------------------------------------------------------------------
// Emu measure
// ---------------------------------------------------------------------------------------------

#[test]
fn emu_converts_between_emu_and_points() {
    assert_eq!(Emu::from_points(1.0).emu(), 12_700);
    assert_eq!(Emu::from_emu(12_700).points(), 1.0);
    assert_eq!(Emu::from_emu(0).points(), 0.0);
    let e = Emu::from_points(2.5);
    assert_eq!(e.emu(), 31_750);
    assert_eq!(Emu::from_emu(e.emu()), e);
}

// ---------------------------------------------------------------------------------------------
// Full effect list — typed reads + byte-exact round-trip
// ---------------------------------------------------------------------------------------------

#[test]
fn full_effect_list_reads_every_field_and_round_trips() {
    let fragment = format!(
        concat!(
            r#"<a:effectLst xmlns:a="{A}">"#,
            r#"<a:blur rad="50800" grow="false"/>"#,
            r#"<a:fillOverlay blend="over"><a:solidFill><a:srgbClr val="FFFF00"/></a:solidFill></a:fillOverlay>"#,
            r#"<a:glow rad="63500"><a:srgbClr val="FF0000"/></a:glow>"#,
            r#"<a:innerShdw blurRad="40000" dist="20000" dir="2700000"><a:srgbClr val="000000"/></a:innerShdw>"#,
            r#"<a:outerShdw blurRad="50000" dist="38100" dir="2700000" sx="90000" sy="90000" kx="600000" algn="tl" rotWithShape="false"><a:srgbClr val="808080"/></a:outerShdw>"#,
            r#"<a:prstShdw prst="shdw13" dist="45000" dir="2700000"><a:srgbClr val="C0C0C0"/></a:prstShdw>"#,
            r#"<a:reflection blurRad="6350" stA="50000" stPos="0" endA="30000" endPos="90000" dist="60000" dir="5400000" fadeDir="2700000" sx="100000" sy="-100000" algn="bl" rotWithShape="false"/>"#,
            r#"<a:softEdge rad="112500"/>"#,
            r#"</a:effectLst>"#
        ),
        A = A
    );
    let (effects, doc): (EffectList, _) = parse_typed(fragment.as_bytes());
    let i = &doc.interner;

    assert_eq!(
        effects.blur(i),
        Some(BlurEffect {
            radius: Some(Emu::from_emu(50_800)),
            grow: Some(false),
        })
    );

    let overlay = effects.fill_overlay(i).expect("fill overlay");
    assert_eq!(overlay.blend, BlendMode::Over);
    assert_eq!(
        overlay.fill,
        FillSpec::Solid(ColorSpec::Srgb("FFFF00".to_owned()))
    );

    let glow = effects.glow(i).expect("glow");
    assert_eq!(glow.color, ColorSpec::Srgb("FF0000".to_owned()));
    assert_eq!(glow.radius, Some(Emu::from_emu(63_500)));

    assert_eq!(
        effects.inner_shadow(i),
        Some(InnerShadowEffect {
            color: ColorSpec::Srgb("000000".to_owned()),
            blur_radius: Some(Emu::from_emu(40_000)),
            distance: Some(Emu::from_emu(20_000)),
            direction: Some(Angle::from_degrees(45.0)),
        })
    );

    let outer = effects.outer_shadow(i).expect("outer shadow");
    assert_eq!(outer.color, ColorSpec::Srgb("808080".to_owned()));
    assert_eq!(outer.blur_radius, Some(Emu::from_emu(50_000)));
    assert_eq!(outer.distance, Some(Emu::from_emu(38_100)));
    assert_eq!(outer.direction, Some(Angle::from_degrees(45.0)));
    assert_eq!(outer.scale_x, Some(Fraction::from_ratio(0.9)));
    assert_eq!(outer.scale_y, Some(Fraction::from_ratio(0.9)));
    assert_eq!(outer.skew_x, Some(Angle::from_degrees(10.0)));
    assert_eq!(outer.skew_y, None);
    assert_eq!(outer.alignment, Some(RectangleAlignment::TopLeft));
    assert_eq!(outer.rotate_with_shape, Some(false));

    assert_eq!(
        effects.preset_shadow(i),
        Some(PresetShadowEffect {
            preset: PresetShadow::Shadow13,
            color: ColorSpec::Srgb("C0C0C0".to_owned()),
            distance: Some(Emu::from_emu(45_000)),
            direction: Some(Angle::from_degrees(45.0)),
        })
    );

    let reflection = effects.reflection(i).expect("reflection");
    assert_eq!(reflection.start_alpha, Some(Fraction::from_ratio(0.5)));
    assert_eq!(reflection.start_position, Some(Fraction::from_ratio(0.0)));
    assert_eq!(reflection.end_alpha, Some(Fraction::from_ratio(0.3)));
    assert_eq!(reflection.end_position, Some(Fraction::from_ratio(0.9)));
    assert_eq!(reflection.direction, Some(Angle::from_degrees(90.0)));
    assert_eq!(reflection.fade_direction, Some(Angle::from_degrees(45.0)));
    assert_eq!(reflection.scale_y, Some(Fraction::from_ratio(-1.0)));
    assert_eq!(reflection.alignment, Some(RectangleAlignment::BottomLeft));

    assert_eq!(
        effects.soft_edge(i),
        Some(SoftEdgeEffect {
            radius: Emu::from_emu(112_500),
        })
    );

    assert_round_trips(&effects, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// Fidelity — opaque internals preserved
// ---------------------------------------------------------------------------------------------

#[test]
fn unknown_child_extlst_and_unknown_attr_survive_verbatim() {
    let fragment = format!(
        concat!(
            r#"<a:effectLst xmlns:a="{A}" data-foo="bar">"#,
            r#"<a:outerShdw dist="20000"><a:srgbClr val="000000"/></a:outerShdw>"#,
            r#"<a:extLst><a:ext uri="{{FA7F}}"><a:foo/></a:ext></a:extLst>"#,
            r#"</a:effectLst>"#
        ),
        A = A
    );
    let (effects, doc): (EffectList, _) = parse_typed(fragment.as_bytes());
    let i = &doc.interner;

    // The modeled effect still reads; the unknown attribute and extLst are not modeled but must
    // round-trip byte-for-byte.
    let outer = effects.outer_shadow(i).expect("outer shadow");
    assert_eq!(outer.distance, Some(Emu::from_emu(20_000)));
    assert_round_trips(&effects, doc, fragment.as_bytes());
}

#[test]
fn empty_effect_list_round_trips_self_closing() {
    let fragment = format!(r#"<a:effectLst xmlns:a="{A}"/>"#);
    let (effects, doc): (EffectList, _) = parse_typed(fragment.as_bytes());
    let i = &doc.interner;
    assert_eq!(effects.blur(i), None);
    assert_eq!(effects.outer_shadow(i), None);
    assert_eq!(effects.soft_edge(i), None);
    assert_round_trips(&effects, doc, fragment.as_bytes());
}

#[test]
fn colored_effect_without_color_reads_none_but_round_trips() {
    // A shadow/glow whose required color is absent is invalid; it reads as None (not modeled) while
    // the wrapper still preserves its raw bytes.
    let fragment = format!(
        concat!(
            r#"<a:effectLst xmlns:a="{A}">"#,
            r#"<a:glow rad="1000"/>"#,
            r#"</a:effectLst>"#
        ),
        A = A
    );
    let (effects, doc): (EffectList, _) = parse_typed(fragment.as_bytes());
    assert_eq!(effects.glow(&doc.interner), None);
    assert_round_trips(&effects, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// EffectListSpec — value tier (spec/to_effect_list) and the builder byte output
// ---------------------------------------------------------------------------------------------

#[test]
fn effect_list_spec_round_trips_through_the_element() {
    // A spec built in code rebuilds an element whose own `spec()` equals the original — the
    // read/write symmetry, independent of byte layout.
    let spec = full_spec();
    let mut interner = Interner::new();
    let effects = spec.to_effect_list(&mut interner);
    assert_eq!(
        effects.spec(&interner),
        spec,
        "EffectListSpec round-trip mismatch"
    );
}

#[test]
fn effect_list_spec_builds_expected_bytes_in_schema_order() {
    let spec = full_spec();
    let mut interner = Interner::new();
    let effects = spec.to_effect_list(&mut interner);
    assert_eq!(serialize_built(interner, &effects), FULL_BYTES);
}

#[test]
fn effect_list_spec_empty_default() {
    let mut interner = Interner::new();
    let empty = EffectListSpec::new().to_effect_list(&mut interner);
    assert_eq!(serialize_built(interner, &empty), r#"<a:effectLst/>"#);
}
