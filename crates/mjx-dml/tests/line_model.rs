//! Unit tests for the DrawingML line (outline) model, through the public API only. Every round-trip
//! assertion is paired with a structural/typed one so byte-identity can't pass by dumping everything
//! into an opaque bucket; typed reads and the `LineSpec` builder are checked against expected
//! values/bytes.

use mjx_dml::{
    ColorSpec, CompoundLine, Fill, FillSpec, Fraction, LineCap, LineDash, LineEnd, LineEndLength,
    LineEndType, LineEndWidth, LineJoin, LineProperties, LineSpec, LineWidth, PenAlignment,
    PresetLineDash,
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

// ---------------------------------------------------------------------------------------------
// LineWidth measure
// ---------------------------------------------------------------------------------------------

#[test]
fn line_width_converts_between_emu_and_points() {
    assert_eq!(LineWidth::from_points(1.5).emu(), 19_050);
    assert_eq!(LineWidth::from_emu(12_700).points(), 1.0);
    assert_eq!(LineWidth::from_emu(0).points(), 0.0);
    // Round-trip through EMU.
    let w = LineWidth::from_points(2.25);
    assert_eq!(w.emu(), 28_575);
    assert_eq!(LineWidth::from_emu(w.emu()), w);
}

// ---------------------------------------------------------------------------------------------
// Full outline — typed reads + byte-exact round-trip
// ---------------------------------------------------------------------------------------------

#[test]
fn full_line_reads_every_field_and_round_trips() {
    let fragment = format!(
        concat!(
            r#"<a:ln xmlns:a="{A}" w="19050" cap="rnd" cmpd="sng" algn="ctr">"#,
            r#"<a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>"#,
            r#"<a:prstDash val="dash"/>"#,
            r#"<a:miter lim="800000"/>"#,
            r#"<a:headEnd type="triangle"/>"#,
            r#"<a:tailEnd type="arrow" w="lg" len="lg"/>"#,
            r#"</a:ln>"#
        ),
        A = A
    );
    let (line, doc): (LineProperties, _) = parse_typed(fragment.as_bytes());
    let i = &doc.interner;

    assert_eq!(line.width(i), Some(LineWidth::from_emu(19_050)));
    assert_eq!(line.width(i).unwrap().points(), 1.5);
    assert_eq!(line.cap(i), Some(LineCap::Round));
    assert_eq!(line.compound(i), Some(CompoundLine::Single));
    assert_eq!(line.pen_alignment(i), Some(PenAlignment::Center));

    match line.fill(i) {
        Some(Fill::Solid(solid)) => {
            assert_eq!(solid.color().unwrap().hex(i), Some("FF0000"));
        }
        other => panic!("expected a solid stroke fill, got {other:?}"),
    }

    assert_eq!(line.dash(i), Some(LineDash::Preset(PresetLineDash::Dash)));
    match line.join(i) {
        Some(LineJoin::Miter { limit: Some(limit) }) => {
            assert_eq!(limit, Fraction::from_ratio(8.0))
        }
        other => panic!("expected a miter join with limit, got {other:?}"),
    }
    assert_eq!(
        line.head_end(i),
        Some(LineEnd {
            kind: Some(LineEndType::Triangle),
            width: None,
            length: None,
        })
    );
    assert_eq!(
        line.tail_end(i),
        Some(LineEnd {
            kind: Some(LineEndType::Arrow),
            width: Some(LineEndWidth::Large),
            length: Some(LineEndLength::Large),
        })
    );

    assert_round_trips(&line, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// Fidelity — opaque internals preserved
// ---------------------------------------------------------------------------------------------

#[test]
fn custom_dash_extlst_and_unknown_attr_survive_verbatim() {
    let fragment = format!(
        concat!(
            r#"<a:ln xmlns:a="{A}" w="9525" cap="flat" data-foo="bar">"#,
            r#"<a:custDash><a:ds d="300000" sp="150000"/><a:ds d="100000" sp="150000"/></a:custDash>"#,
            r#"<a:extLst><a:ext uri="{{FA7F}}"><a:foo/></a:ext></a:extLst>"#,
            r#"</a:ln>"#
        ),
        A = A
    );
    let (line, doc): (LineProperties, _) = parse_typed(fragment.as_bytes());
    let i = &doc.interner;

    // The dash is reported as a custom dash; its stops are not modeled but must round-trip.
    assert_eq!(line.dash(i), Some(LineDash::Custom));
    assert_eq!(line.cap(i), Some(LineCap::Flat));
    assert_eq!(line.width(i), Some(LineWidth::from_emu(9_525)));
    // Unknown attribute and extLst are not modeled — verified by the byte-exact round-trip below.
    assert_round_trips(&line, doc, fragment.as_bytes());
}

#[test]
fn empty_line_round_trips_self_closing() {
    let fragment = format!(r#"<a:ln xmlns:a="{A}"/>"#);
    let (line, doc): (LineProperties, _) = parse_typed(fragment.as_bytes());
    assert_eq!(line.width(&doc.interner), None);
    assert_eq!(line.fill(&doc.interner), None);
    assert_eq!(line.dash(&doc.interner), None);
    assert_round_trips(&line, doc, fragment.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// Stroke fill variety (reuses the Fill model)
// ---------------------------------------------------------------------------------------------

#[test]
fn gradient_and_pattern_stroke_fills_read_back() {
    let grad = format!(
        concat!(
            r#"<a:ln xmlns:a="{A}" w="12700"><a:gradFill><a:gsLst>"#,
            r#"<a:gs pos="0"><a:srgbClr val="FF0000"/></a:gs>"#,
            r#"<a:gs pos="100000"><a:srgbClr val="0000FF"/></a:gs>"#,
            r#"</a:gsLst><a:lin ang="5400000"/></a:gradFill></a:ln>"#
        ),
        A = A
    );
    let (line, doc): (LineProperties, _) = parse_typed(grad.as_bytes());
    assert!(matches!(line.fill(&doc.interner), Some(Fill::Gradient(_))));
    assert_round_trips(&line, doc, grad.as_bytes());

    let patt = format!(
        concat!(
            r#"<a:ln xmlns:a="{A}"><a:pattFill prst="pct25">"#,
            r#"<a:fgClr><a:srgbClr val="000000"/></a:fgClr>"#,
            r#"<a:bgClr><a:srgbClr val="FFFFFF"/></a:bgClr></a:pattFill></a:ln>"#
        ),
        A = A
    );
    let (line, doc): (LineProperties, _) = parse_typed(patt.as_bytes());
    assert!(matches!(line.fill(&doc.interner), Some(Fill::Pattern(_))));
    assert_round_trips(&line, doc, patt.as_bytes());
}

// ---------------------------------------------------------------------------------------------
// LineSpec — value tier (spec/to_line) and the builder byte output
// ---------------------------------------------------------------------------------------------

#[test]
fn line_spec_round_trips_through_the_element() {
    // A spec built in code rebuilds an element whose own `spec()` equals the original — the
    // read/write symmetry, independent of byte layout.
    let spec = LineSpec {
        width: Some(LineWidth::from_emu(19_050)),
        cap: Some(LineCap::Round),
        compound: Some(CompoundLine::Single),
        pen_alignment: Some(PenAlignment::Center),
        fill: Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".to_owned()))),
        dash: Some(LineDash::Preset(PresetLineDash::Dash)),
        join: Some(LineJoin::Miter {
            limit: Some(Fraction::from_ratio(8.0)),
        }),
        head_end: Some(LineEnd {
            kind: Some(LineEndType::Triangle),
            width: None,
            length: None,
        }),
        tail_end: Some(LineEnd {
            kind: Some(LineEndType::Arrow),
            width: Some(LineEndWidth::Large),
            length: Some(LineEndLength::Large),
        }),
    };

    let mut interner = Interner::new();
    let line = spec.to_line(&mut interner);
    assert_eq!(line.spec(&interner), spec, "LineSpec round-trip mismatch");
}

#[test]
fn line_spec_builds_expected_bytes_in_schema_order() {
    let spec = LineSpec {
        width: Some(LineWidth::from_emu(19_050)),
        cap: Some(LineCap::Round),
        compound: Some(CompoundLine::Single),
        pen_alignment: Some(PenAlignment::Center),
        fill: Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".to_owned()))),
        dash: Some(LineDash::Preset(PresetLineDash::Dash)),
        join: Some(LineJoin::Miter {
            limit: Some(Fraction::from_ratio(8.0)),
        }),
        head_end: Some(LineEnd {
            kind: Some(LineEndType::Triangle),
            width: None,
            length: None,
        }),
        tail_end: Some(LineEnd {
            kind: Some(LineEndType::Arrow),
            width: Some(LineEndWidth::Large),
            length: Some(LineEndLength::Large),
        }),
    };
    let mut interner = Interner::new();
    let line = spec.to_line(&mut interner);
    assert_eq!(
        serialize_built(interner, &line),
        concat!(
            r#"<a:ln w="19050" cap="rnd" cmpd="sng" algn="ctr">"#,
            r#"<a:solidFill><a:srgbClr val="FF0000"/></a:solidFill>"#,
            r#"<a:prstDash val="dash"/>"#,
            r#"<a:miter lim="800000"/>"#,
            r#"<a:headEnd type="triangle"/>"#,
            r#"<a:tailEnd type="arrow" w="lg" len="lg"/>"#,
            r#"</a:ln>"#
        )
    );
}

#[test]
fn line_spec_solid_constructor_and_empty_default() {
    let spec = LineSpec::solid(
        LineWidth::from_points(2.0),
        ColorSpec::Srgb("00FF00".to_owned()),
    );
    let mut interner = Interner::new();
    let line = spec.to_line(&mut interner);
    assert_eq!(
        serialize_built(interner, &line),
        r#"<a:ln w="25400"><a:solidFill><a:srgbClr val="00FF00"/></a:solidFill></a:ln>"#
    );

    // An empty spec builds a self-closing <a:ln/>.
    let mut interner = Interner::new();
    let empty = LineSpec::new().to_line(&mut interner);
    assert_eq!(serialize_built(interner, &empty), r#"<a:ln/>"#);
}

#[test]
fn custom_dash_spec_rebuilds_empty_custdash() {
    // The value tier does not model custom dash stops; rebuilding a Custom dash emits an empty element.
    let spec = LineSpec {
        dash: Some(LineDash::Custom),
        ..LineSpec::new()
    };
    let mut interner = Interner::new();
    let line = spec.to_line(&mut interner);
    assert_eq!(
        serialize_built(interner, &line),
        r#"<a:ln><a:custDash/></a:ln>"#
    );
}
