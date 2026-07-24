//! Tests for editing a chart's cached series data (C3), through the public API only.
//!
//! The invariants that matter: a rewritten cache reads its new values back, its `c:ptCount` tracks
//! the new count, the `c:formatCode` and everything outside the cache (axes, other children) survive
//! untouched, the edited chart re-serializes to well-formed XML, and a non-finite value is skipped
//! rather than written as an invalid token.

use mjx_chart::ChartSpace;
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_xml::fidelity;

/// A one-series bar chart: categories North/South/West (a `c:strCache`), values 19.2/21.4/16.7 (a
/// `c:numCache` with a `c:formatCode` and `c:ptCount`), plus a category axis to prove edits leave
/// everything outside the edited cache alone.
const BAR: &str = concat!(
    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea><c:barChart>"#,
    r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#,
    r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#,
    r#"<c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>Sales</c:v></c:pt></c:strCache></c:strRef></c:tx>"#,
    r#"<c:cat><c:strRef><c:f>Sheet1!$A$2:$A$4</c:f><c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v>North</c:v></c:pt><c:pt idx="1"><c:v>South</c:v></c:pt><c:pt idx="2"><c:v>West</c:v></c:pt></c:strCache></c:strRef></c:cat>"#,
    r#"<c:val><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>19.2</c:v></c:pt><c:pt idx="1"><c:v>21.4</c:v></c:pt><c:pt idx="2"><c:v>16.7</c:v></c:pt></c:numCache></c:numRef></c:val>"#,
    r#"</c:ser><c:axId val="1"/></c:barChart>"#,
    r#"<c:catAx><c:axId val="1"/><c:delete val="0"/></c:catAx>"#,
    r#"</c:plotArea></c:chart></c:chartSpace>"#,
);

fn parse(xml: &str) -> (ChartSpace, RawDocument) {
    let doc = fidelity::parse(xml.as_bytes()).expect("parses");
    let space = ChartSpace::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (space, doc)
}

/// Re-serializes the (edited) chart to a string.
fn serialize(space: &ChartSpace, doc: &mut RawDocument) -> String {
    doc.root = space.to_xml(&mut doc.interner);
    String::from_utf8(fidelity::serialize_to_vec(doc)).expect("utf-8")
}

/// Re-parses the edited chart, proving the output is well-formed and reads back through the model.
fn reparse(space: &ChartSpace, doc: &mut RawDocument) -> ChartSpace {
    let xml = serialize(space, doc);
    let (reparsed, _) = parse(&xml);
    reparsed
}

#[test]
fn rewrites_series_values_and_tracks_ptcount() {
    let (mut space, mut doc) = parse(BAR);

    assert!(space
        .series_mut(0)
        .unwrap()
        .set_values(&mut doc.interner, &[1.0, 2.0, 3.0, 4.0]));

    // Read back through a fresh parse of the serialized result.
    let reparsed = reparse(&space, &mut doc);
    let series = reparsed.bar_chart().unwrap().series_at(0).unwrap();
    assert_eq!(series.values().unwrap().values(), vec![1.0, 2.0, 3.0, 4.0]);

    let xml = serialize(&space, &mut doc);
    assert!(
        xml.contains(r#"<c:ptCount val="4"/>"#),
        "ptCount tracks the new count: {xml}"
    );
}

#[test]
fn preserves_format_code_and_everything_outside_the_cache() {
    let (mut space, mut doc) = parse(BAR);
    space
        .series_mut(0)
        .unwrap()
        .set_values(&mut doc.interner, &[5.0]);

    let xml = serialize(&space, &mut doc);
    // The formatCode inside the edited cache survives...
    assert!(
        xml.contains(r#"<c:formatCode>General</c:formatCode>"#),
        "formatCode preserved: {xml}"
    );
    // ...as does the category axis and the untouched category cache outside it.
    assert!(xml.contains("<c:catAx>"), "axis preserved: {xml}");
    assert!(
        xml.contains("<c:v>North</c:v>"),
        "categories untouched: {xml}"
    );
    // The formatCode still precedes the (rewritten) points.
    let fmt = xml.find("c:formatCode").unwrap();
    let first_pt = xml
        .find(r#"<c:pt idx="0"><c:v>5</c:v>"#)
        .expect("new point present");
    assert!(fmt < first_pt, "formatCode stays before the points");
}

#[test]
fn rewrites_category_labels() {
    let (mut space, mut doc) = parse(BAR);

    assert!(space
        .series_mut(0)
        .unwrap()
        .set_categories(&mut doc.interner, &["East", "West"]));

    let reparsed = reparse(&space, &mut doc);
    let series = reparsed.bar_chart().unwrap().series_at(0).unwrap();
    assert_eq!(series.categories().unwrap().labels(), vec!["East", "West"]);
}

#[test]
fn skips_non_finite_values() {
    let (mut space, mut doc) = parse(BAR);
    space
        .series_mut(0)
        .unwrap()
        .set_values(&mut doc.interner, &[1.0, f64::NAN, 3.0, f64::INFINITY]);

    let reparsed = reparse(&space, &mut doc);
    let series = reparsed.bar_chart().unwrap().series_at(0).unwrap();
    // NaN and inf are skipped — only the two finite values are written.
    assert_eq!(series.values().unwrap().values(), vec![1.0, 3.0]);
    let xml = serialize(&space, &mut doc);
    assert!(
        xml.contains(r#"<c:ptCount val="2"/>"#),
        "ptCount counts only finite points: {xml}"
    );
    assert!(
        !xml.contains("NaN") && !xml.contains("inf"),
        "no invalid tokens: {xml}"
    );
}

#[test]
fn escapes_labels_with_special_characters() {
    let (mut space, mut doc) = parse(BAR);
    space
        .series_mut(0)
        .unwrap()
        .set_categories(&mut doc.interner, &["R&D", "A<B"]);

    // Round-trips through a re-parse (proves the escaping is well-formed) and decodes back.
    let reparsed = reparse(&space, &mut doc);
    let labels = reparsed
        .bar_chart()
        .unwrap()
        .series_at(0)
        .unwrap()
        .categories()
        .unwrap()
        .labels();
    assert_eq!(labels, vec!["R&D", "A<B"]);

    let xml = serialize(&space, &mut doc);
    assert!(
        xml.contains("R&amp;D") && xml.contains("A&lt;B"),
        "labels are escaped: {xml}"
    );
}
