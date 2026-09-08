//! Unit tests for the DrawingML color + solidFill model, through the public API only. Every round-trip
//! assertion is paired with a structural one so byte-identity can't pass by dumping into `Raw`.

use mjx_dml::{
    Angle, Color, ColorKind, ColorSpec, ColorTransform, ColorTransformKind, ColorTransformValue,
    Fraction, SchemeColor, SolidFill, SolidFillContent,
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
    let doc = RawDocument::new(interner, false, Vec::new(), root, Vec::new());
    String::from_utf8(fidelity::serialize_to_vec(&doc)).expect("utf-8")
}

#[test]
fn srgb_color_reads_and_preserves_transforms() {
    let fragment =
        format!(r#"<a:srgbClr xmlns:a="{A}" val="FF0000"><a:lumMod val="50000"/></a:srgbClr>"#);
    let (color, doc): (Color, _) = parse_typed(fragment.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::Srgb);
    assert_eq!(color.hex(&doc.interner).as_deref(), Some("FF0000"));
    assert_eq!(color.scheme_color(&doc.interner), None);
    // The lumMod transform is preserved opaquely.
    assert_eq!(color.transforms().len(), 1);
    assert_round_trips(&color, doc, fragment.as_bytes());
}

#[test]
fn scheme_color_reads_the_theme_slot() {
    let fragment = format!(r#"<a:schemeClr xmlns:a="{A}" val="accent1"/>"#);
    let (color, doc): (Color, _) = parse_typed(fragment.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::Scheme);
    assert_eq!(
        color.scheme_color(&doc.interner),
        Some(SchemeColor::Accent1)
    );
    assert_eq!(color.hex(&doc.interner), None);
    assert_round_trips(&color, doc, fragment.as_bytes());
}

#[test]
fn system_and_preset_colors_round_trip() {
    // sysClr keeps its optional lastClr; both read as their kind and round-trip verbatim.
    let sys = format!(r#"<a:sysClr xmlns:a="{A}" val="windowText" lastClr="000000"/>"#);
    let (color, doc): (Color, _) = parse_typed(sys.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::System);
    assert_round_trips(&color, doc, sys.as_bytes());

    let prst = format!(r#"<a:prstClr xmlns:a="{A}" val="red"/>"#);
    let (color, doc): (Color, _) = parse_typed(prst.as_bytes());
    assert_eq!(color.kind(&doc.interner), ColorKind::Preset);
    assert_round_trips(&color, doc, prst.as_bytes());
}

#[test]
fn builds_srgb_and_scheme_colors() {
    let mut interner = Interner::new();
    let srgb = Color::srgb(&mut interner, "FF0000");
    assert_eq!(
        serialize_built(interner, &srgb),
        r#"<a:srgbClr val="FF0000"/>"#
    );

    let mut interner = Interner::new();
    let scheme = Color::scheme(&mut interner, SchemeColor::Background1);
    assert_eq!(
        serialize_built(interner, &scheme),
        r#"<a:schemeClr val="bg1"/>"#
    );
}

#[test]
fn solid_fill_round_trips_and_exposes_its_color() {
    let fragment = format!(r#"<a:solidFill xmlns:a="{A}"><a:srgbClr val="00FF00"/></a:solidFill>"#);
    let (fill, doc): (SolidFill, _) = parse_typed(fragment.as_bytes());
    assert_eq!(fill.content().len(), 1);
    assert!(matches!(fill.content()[0], SolidFillContent::Color(_)));
    assert_eq!(
        fill.color().unwrap().hex(&doc.interner).as_deref(),
        Some("00FF00")
    );
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

#[test]
fn builds_solid_fill_and_empty_fill_round_trips() {
    let mut interner = Interner::new();
    let color = Color::srgb(&mut interner, "00FF00");
    let fill = SolidFill::new(&mut interner, Some(color));
    assert_eq!(
        serialize_built(interner, &fill),
        r#"<a:solidFill><a:srgbClr val="00FF00"/></a:solidFill>"#
    );

    // An empty (color-less) solidFill is legal and round-trips; color() is None.
    let fragment = format!(r#"<a:solidFill xmlns:a="{A}"/>"#);
    let (fill, doc): (SolidFill, _) = parse_typed(fragment.as_bytes());
    assert!(fill.color().is_none());
    assert_round_trips(&fill, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// Colour transforms — MJXOFF-219
// ---------------------------------------------------------------------------------------------

/// Every member of `EG_ColorTransform` is authorable through `ColorSpec` and writes the element the
/// schema names, in the order the builder appended it.
///
/// This is the assertion the whole unit exists for: before MJXOFF-219 no path in this workspace
/// could author *any* of the twenty-eight, and `V-PPTX-02.4` had no file because of it. It is also
/// what fails if the writer stops emitting a transform — remove the `push_transform` loop from
/// `Color::from_spec` and every one of the twenty-eight comparisons below goes red.
#[test]
fn every_transform_in_the_group_is_authorable_and_writes_its_element() {
    for kind in ColorTransformKind::ALL {
        let local = kind.local_name().expect("a member names an element");
        let (transform, written) = match kind.value_kind() {
            ColorTransformValue::Percentage => (
                ColorTransform::from_percentage(kind, Fraction::from_ratio(0.5))
                    .expect("a percentage member"),
                format!(r#"<a:{local} val="50000"/>"#),
            ),
            ColorTransformValue::Angle => (
                ColorTransform::from_angle(kind, Angle::from_degrees(45.0))
                    .expect("an angle member"),
                format!(r#"<a:{local} val="2700000"/>"#),
            ),
            ColorTransformValue::Marker => (
                ColorTransform::marker(kind).expect("a valueless member"),
                format!("<a:{local}/>"),
            ),
            ColorTransformValue::Raw => unreachable!("a member is never raw"),
        };

        let spec = ColorSpec::Srgb("FF0000".into()).with_transform(transform.clone());
        let mut interner = Interner::new();
        let color = Color::from_spec(&mut interner, &spec).expect("a colour");
        assert_eq!(
            serialize_built(interner, &color),
            format!(r#"<a:srgbClr val="FF0000">{written}</a:srgbClr>"#),
            "{kind:?} did not write the element the schema names"
        );

        // …and reading it back answers the same transform, so the surface closes.
        let fragment = format!(r#"<a:srgbClr xmlns:a="{A}" val="FF0000">{written}</a:srgbClr>"#);
        let (read, doc): (Color, _) = parse_typed(fragment.as_bytes());
        assert_eq!(read.spec(&doc.interner), spec, "{kind:?} did not read back");
    }
}

/// Order is part of the markup, so the builder appends: the same two transforms in the other order
/// are a different colour and a different file.
#[test]
fn transforms_append_in_order_rather_than_merging() {
    let lighter = ColorSpec::Scheme(SchemeColor::Accent1)
        .with_luminance_modulation(Fraction::from_ratio(0.6))
        .with_luminance_offset(Fraction::from_ratio(0.4));
    let reversed = ColorSpec::Scheme(SchemeColor::Accent1)
        .with_luminance_offset(Fraction::from_ratio(0.4))
        .with_luminance_modulation(Fraction::from_ratio(0.6));
    assert_ne!(lighter, reversed);

    let mut interner = Interner::new();
    let color = Color::from_spec(&mut interner, &lighter).expect("a colour");
    assert_eq!(
        serialize_built(interner, &color),
        r#"<a:schemeClr val="accent1"><a:lumMod val="60000"/><a:lumOff val="40000"/></a:schemeClr>"#
    );

    let mut interner = Interner::new();
    let color = Color::from_spec(&mut interner, &reversed).expect("a colour");
    assert_eq!(
        serialize_built(interner, &color),
        r#"<a:schemeClr val="accent1"><a:lumOff val="40000"/><a:lumMod val="60000"/></a:schemeClr>"#
    );

    // Repeating a member is legal — the group is an unbounded choice — and both copies survive.
    let twice = ColorSpec::Srgb("FF0000".into())
        .with_tint(Fraction::from_ratio(0.25))
        .with_tint(Fraction::from_ratio(0.75));
    assert_eq!(twice.transforms().len(), 2);
    let mut interner = Interner::new();
    let color = Color::from_spec(&mut interner, &twice).expect("a colour");
    assert_eq!(
        serialize_built(interner, &color),
        r#"<a:srgbClr val="FF0000"><a:tint val="25000"/><a:tint val="75000"/></a:srgbClr>"#
    );
}

/// A transform is a child of a colour, not a different kind of colour: `base` sees through it, and
/// so does everything a reader asks about the colour itself.
#[test]
fn a_transformed_colour_is_still_the_colour_underneath() {
    let spec = ColorSpec::Scheme(SchemeColor::Accent1).with_tint(Fraction::from_ratio(0.5));
    assert_eq!(spec.base(), &ColorSpec::Scheme(SchemeColor::Accent1));
    assert_eq!(spec.transforms().len(), 1);

    // The builders keep a `Transformed` exactly one level deep, whatever order they are called in.
    let twice = spec.with_shade(Fraction::from_ratio(0.25));
    assert_eq!(twice.base(), &ColorSpec::Scheme(SchemeColor::Accent1));
    assert_eq!(twice.transforms().len(), 2);

    // A colour with no transforms is unchanged: no wrapper, no allocation, and `transforms` is empty.
    let plain = ColorSpec::Srgb("FF0000".into());
    assert_eq!(plain.base(), &plain);
    assert!(plain.transforms().is_empty());
}

/// The round trip MJXOFF-219 closed: `spec()` used to drop a producer's transforms, so
/// `spec()` → `from_spec()` silently lost them. Now it keeps them, in order.
#[test]
fn the_spec_round_trip_keeps_a_producers_transforms() {
    let fragment = format!(
        r#"<a:schemeClr xmlns:a="{A}" val="accent1"><a:lumMod val="60000"/><a:lumOff val="40000"/><a:alpha val="50000"/></a:schemeClr>"#
    );
    let (color, doc): (Color, _) = parse_typed(fragment.as_bytes());
    let spec = color.spec(&doc.interner);
    assert_eq!(
        spec.transforms(),
        &[
            ColorTransform::LuminanceModulation(Fraction::from_ratio(0.6)),
            ColorTransform::LuminanceOffset(Fraction::from_ratio(0.4)),
            ColorTransform::Alpha(Fraction::from_ratio(0.5)),
        ]
    );

    let mut interner = Interner::new();
    let rebuilt = Color::from_spec(&mut interner, &spec).expect("a colour");
    assert_eq!(
        serialize_built(interner, &rebuilt),
        r#"<a:schemeClr val="accent1"><a:lumMod val="60000"/><a:lumOff val="40000"/><a:alpha val="50000"/></a:schemeClr>"#
    );
}

/// A transform this model cannot read still survives the round trip, because the alternative is
/// deleting a producer's markup: an element the group does not name, and one it does whose `@val`
/// is missing or unparseable, both land in `ColorTransform::Other` and come back out verbatim.
#[test]
fn an_unreadable_transform_survives_as_other() {
    let fragment = format!(
        r#"<a:srgbClr xmlns:a="{A}" val="FF0000"><a:tint val="not-a-number"/><a:lumMod/><a:futureTransform val="3"/></a:srgbClr>"#
    );
    let (color, doc): (Color, _) = parse_typed(fragment.as_bytes());
    let spec = color.spec(&doc.interner);
    assert_eq!(
        spec.transforms(),
        &[
            ColorTransform::other("tint", Some("not-a-number".into())),
            ColorTransform::other("lumMod", None),
            ColorTransform::other("futureTransform", Some("3".into())),
        ]
    );
    assert_eq!(spec.transforms()[0].kind(), ColorTransformKind::Other);

    let mut interner = Interner::new();
    let rebuilt = Color::from_spec(&mut interner, &spec).expect("a colour");
    assert_eq!(
        serialize_built(interner, &rebuilt),
        r#"<a:srgbClr val="FF0000"><a:tint val="not-a-number"/><a:lumMod/><a:futureTransform val="3"/></a:srgbClr>"#
    );
}

/// The percentage wire form Office writes is read and written exactly, and the ISO `%` spelling is
/// read as the same value — `crate::codec::Percentage`'s contract, exercised through a transform.
#[test]
fn both_percentage_spellings_read_and_one_is_written() {
    let fragment =
        format!(r#"<a:srgbClr xmlns:a="{A}" val="FF0000"><a:tint val="50%"/></a:srgbClr>"#);
    let (color, doc): (Color, _) = parse_typed(fragment.as_bytes());
    assert_eq!(
        color.spec(&doc.interner).transforms(),
        &[ColorTransform::Tint(Fraction::from_ratio(0.5))]
    );

    let mut interner = Interner::new();
    let rebuilt = Color::from_spec(
        &mut interner,
        &ColorSpec::Srgb("FF0000".into()).with_tint(Fraction::from_ratio(0.5)),
    )
    .expect("a colour");
    assert_eq!(
        serialize_built(interner, &rebuilt),
        r#"<a:srgbClr val="FF0000"><a:tint val="50000"/></a:srgbClr>"#
    );
}

/// A transform on a colour still resolves: the authoring surface and `resolve_color` meet on the
/// same markup, which is the whole point of authoring one.
#[test]
fn an_authored_transform_is_what_resolution_reads() {
    let mut interner = Interner::new();
    let color = Color::from_spec(
        &mut interner,
        &ColorSpec::Srgb("FF0000".into()).with_transform(ColorTransform::Grayscale),
    )
    .expect("a colour");
    let resolved = mjx_dml::resolve_color(
        &color,
        &mjx_dml::SchemeColors::default(),
        &mjx_dml::ColorMap::identity(),
        None,
        &interner,
    )
    .expect("an sRGB colour always resolves");
    assert_eq!(resolved.red, resolved.green);
    assert_eq!(resolved.green, resolved.blue);
    assert_ne!(
        resolved.to_hex(),
        "FF0000",
        "a:gray did not reach resolution"
    );
}

/// A `ColorSpec::Transformed` a caller built by hand — the variant's fields are public, so one
/// *can* be nested — flattens rather than panicking or writing a colour inside a colour. The
/// innermost level is written first, which is what nesting one transformed colour inside another
/// says.
#[test]
fn a_hand_built_nest_flattens_innermost_first() {
    let nested = ColorSpec::Transformed {
        base: Box::new(ColorSpec::Transformed {
            base: Box::new(ColorSpec::Srgb("FF0000".into())),
            transforms: vec![ColorTransform::Tint(Fraction::from_ratio(0.25))],
        }),
        transforms: vec![ColorTransform::Shade(Fraction::from_ratio(0.75))],
    };
    assert_eq!(nested.base(), &ColorSpec::Srgb("FF0000".into()));

    let mut interner = Interner::new();
    let color = Color::from_spec(&mut interner, &nested).expect("a colour");
    assert_eq!(
        serialize_built(interner, &color),
        r#"<a:srgbClr val="FF0000"><a:tint val="25000"/><a:shade val="75000"/></a:srgbClr>"#
    );
}
