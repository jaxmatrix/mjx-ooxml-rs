//! Unit tests for the chart model (`c:chartSpace`), through the public API only.
//!
//! Two things get the most attention: **reading the bar data** end to end (kind → series → category
//! labels and values, down through the `c:strCache`/`c:numCache`), and **round-trip fidelity** — a
//! chart carries axes, text properties, an external-data reference and a `c:date1904` this tier does
//! not interpret, and every byte of it has to come back out unchanged via the `Raw` bucket.

use mjx_chart::{BarDirection, BarGrouping, ChartKind, ChartSpace};
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_xml::fidelity;

/// The `c:chartSpace` element of `tests/fixtures/charts.pptx → ppt/charts/chart1.xml`, verbatim
/// (the OPC prolog is the package layer's concern, exercised by the pptx C0 round-trip tests). One
/// clustered-column series "Sales" over North/South/West = 19.2/21.4/16.7, plus the axes, text
/// properties and external-data reference this tier does not model.
const CHART1: &str = concat!(
    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
    r#"<c:date1904 val="0"/><c:chart><c:autoTitleDeleted val="0"/><c:plotArea><c:barChart>"#,
    r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#,
    r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#,
    r#"<c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>Sales</c:v></c:pt></c:strCache></c:strRef></c:tx>"#,
    r#"<c:cat><c:strRef><c:f>Sheet1!$A$2:$A$4</c:f><c:strCache><c:ptCount val="3"/><c:pt idx="0"><c:v>North</c:v></c:pt><c:pt idx="1"><c:v>South</c:v></c:pt><c:pt idx="2"><c:v>West</c:v></c:pt></c:strCache></c:strRef></c:cat>"#,
    r#"<c:val><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f><c:numCache><c:formatCode>General</c:formatCode><c:ptCount val="3"/><c:pt idx="0"><c:v>19.2</c:v></c:pt><c:pt idx="1"><c:v>21.4</c:v></c:pt><c:pt idx="2"><c:v>16.7</c:v></c:pt></c:numCache></c:numRef></c:val>"#,
    r#"</c:ser>"#,
    r#"<c:axId val="-2068027336"/><c:axId val="-2113994440"/></c:barChart>"#,
    r#"<c:catAx><c:axId val="-2068027336"/><c:scaling><c:orientation val="minMax"/></c:scaling><c:delete val="0"/><c:axPos val="b"/><c:majorTickMark val="out"/><c:minorTickMark val="none"/><c:tickLblPos val="nextTo"/><c:crossAx val="-2113994440"/><c:crosses val="autoZero"/><c:auto val="1"/><c:lblAlgn val="ctr"/><c:lblOffset val="100"/><c:noMultiLvlLbl val="0"/></c:catAx>"#,
    r#"<c:valAx><c:axId val="-2113994440"/><c:scaling/><c:delete val="0"/><c:axPos val="l"/><c:majorGridlines/><c:majorTickMark val="out"/><c:minorTickMark val="none"/><c:tickLblPos val="nextTo"/><c:crossAx val="-2068027336"/><c:crosses val="autoZero"/></c:valAx>"#,
    r#"</c:plotArea><c:dispBlanksAs val="gap"/></c:chart>"#,
    r#"<c:txPr><a:bodyPr/><a:lstStyle/><a:p><a:pPr><a:defRPr sz="1800"/></a:pPr><a:endParaRPr lang="en-US"/></a:p></c:txPr>"#,
    r#"<c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData>"#,
    r#"</c:chartSpace>"#,
);

fn parse(xml: &str) -> (ChartSpace, RawDocument) {
    let doc = fidelity::parse(xml.as_bytes()).expect("fragment parses");
    let space = ChartSpace::from_xml(&doc.root, &doc.interner).expect("from_xml");
    (space, doc)
}

#[track_caller]
fn assert_round_trips(space: &ChartSpace, mut doc: RawDocument, expected: &str) {
    doc.root = space.to_xml(&mut doc.interner);
    let out = fidelity::serialize_to_vec(&doc);
    assert_eq!(
        String::from_utf8_lossy(&out),
        expected,
        "round-trip mismatch"
    );
}

// ---------------------------------------------------------------------------------------------
// Reading the bar data
// ---------------------------------------------------------------------------------------------

#[test]
fn reads_the_chart_kind_and_bar_grouping() {
    let (space, doc) = parse(CHART1);

    assert_eq!(space.chart_kind(), Some(ChartKind::Bar));
    let bar = space.bar_chart().expect("a bar plot");
    assert_eq!(bar.series_count(), 1);
    assert_eq!(bar.direction(&doc.interner), Some(BarDirection::Column));
    assert_eq!(bar.grouping(&doc.interner), Some(BarGrouping::Clustered));
}

#[test]
fn reads_the_series_name_categories_and_values() {
    let (space, doc) = parse(CHART1);
    let series = space
        .bar_chart()
        .expect("bar")
        .series_at(0)
        .expect("series 0");

    assert_eq!(series.index(&doc.interner), Some(0));
    assert_eq!(series.order(&doc.interner), Some(0));
    assert_eq!(series.name().as_deref(), Some("Sales"));
    assert_eq!(
        series.categories().expect("categories").labels(),
        vec!["North", "South", "West"]
    );
    assert_eq!(
        series.values().expect("values").values(),
        vec![19.2, 21.4, 16.7]
    );
}

#[test]
fn reads_the_workbook_formula_behind_the_values() {
    let (space, _doc) = parse(CHART1);
    let values = space
        .bar_chart()
        .unwrap()
        .series_at(0)
        .unwrap()
        .values()
        .unwrap();

    let formula = values.reference().expect("numRef").formula().expect("f");
    assert_eq!(formula.text(), "Sheet1!$B$2:$B$4");
}

#[test]
fn navigates_the_spine_both_ways() {
    let (space, _doc) = parse(CHART1);

    // Step by step, and by the convenience shortcut — same plot.
    assert!(space.chart().is_some());
    assert!(space.chart().unwrap().plot_area().is_some());
    assert!(space.plot_area().is_some());
    assert!(space.plot_area().unwrap().bar_chart().is_some());
    assert!(space.bar_chart().is_some());
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn the_whole_chart_round_trips_byte_for_byte() {
    let (space, doc) = parse(CHART1);

    // Read something through the model first, so the round-trip is not passing by never looking.
    assert_eq!(space.chart_kind(), Some(ChartKind::Bar));
    assert_round_trips(&space, doc, CHART1);
}

#[test]
fn what_this_tier_does_not_model_survives() {
    // A lang element, an unknown attribute on the space, an extLst, a spPr on the series, an axis
    // and an external-data reference: none of these are interpreted here, and all must come back out.
    let source = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" unknownAttr="kept">"#,
        r#"<c:lang val="en-US"/>"#,
        r#"<c:chart><c:plotArea><c:barChart><c:barDir val="bar"/><c:grouping val="stacked"/>"#,
        r#"<c:ser><c:idx val="0"/><c:spPr><a:noFill/></c:spPr>"#,
        r#"<c:val><c:numRef><c:f>Sheet1!$B$2</c:f><c:numCache><c:pt idx="0"><c:v>1</c:v></c:pt></c:numCache></c:numRef></c:val>"#,
        r#"</c:ser></c:barChart><c:extLst><c:ext uri="x"/></c:extLst></c:plotArea></c:chart>"#,
        r#"<c:externalData r:id="rId1"/></c:chartSpace>"#,
    );
    let (space, doc) = parse(source);

    // Read through the model: the bar plot and its (single) value are still reachable past the noise.
    let bar = space.bar_chart().expect("bar");
    assert_eq!(bar.direction(&doc.interner), Some(BarDirection::Bar));
    assert_eq!(bar.grouping(&doc.interner), Some(BarGrouping::Stacked));
    assert_eq!(
        bar.series_at(0).unwrap().values().unwrap().values(),
        vec![1.0]
    );
    assert_round_trips(&space, doc, source);
}

#[test]
fn an_empty_chart_space_round_trips_and_reads_as_nothing() {
    let source =
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"/>"#;
    let (space, doc) = parse(source);

    assert!(space.chart().is_none());
    assert!(space.bar_chart().is_none());
    assert_eq!(space.chart_kind(), None);
    assert_round_trips(&space, doc, source);
}

#[test]
fn a_series_without_a_cache_reads_as_empty_and_round_trips() {
    // `c:numRef` with a formula but no `c:numCache` (values not yet cached): accessors return empty,
    // nothing panics, and the reference survives verbatim.
    let source = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart">"#,
        r#"<c:chart><c:plotArea><c:barChart><c:ser>"#,
        r#"<c:val><c:numRef><c:f>Sheet1!$B$2:$B$4</c:f></c:numRef></c:val>"#,
        r#"</c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#,
    );
    let (space, doc) = parse(source);
    let series = space.bar_chart().unwrap().series_at(0).unwrap();

    assert!(series.values().unwrap().values().is_empty());
    assert!(series.name().is_none());
    assert_round_trips(&space, doc, source);
}
