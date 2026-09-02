//! The ten plot types that used to ride through the `Raw` bucket unread — radar, bubble, stock,
//! pie-of-pie, the two surfaces and the four three-dimensional forms (MJX-116, part 2).
//!
//! Each is exercised the same way as the six that were already modeled: it names its kind, its
//! series read their name, categories and values, its type-specific scalar reads, and the whole
//! plot round-trips byte-for-byte. The point of the round-trip assertion here is not that these
//! types *can* be preserved — the `Raw` bucket already did that — but that **modelling them changed
//! nothing about what is written back**.

use mjx_chart::{ChartKind, ChartSpace, OfPieType, RadarStyle};
use mjx_ooxml_core::{FromXml, RawDocument, ToXml};
use mjx_xml::fidelity;

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

/// Wraps plot-area body XML in the chart-space spine.
fn wrap(plots: &str) -> String {
    format!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart><c:plotArea>{plots}</c:plotArea></c:chart></c:chartSpace>"#
    )
}

/// A category/value plot element `local` carrying `scalars` then one series "Series 1" over
/// Alpha/Beta = 10/20.
fn cat_val_plot(local: &str, scalars: &str) -> String {
    format!(
        r#"<c:{local}>{scalars}<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>Series 1</c:v></c:tx><c:cat><c:strRef><c:f>Sheet1!$A$2:$A$3</c:f><c:strCache><c:ptCount val="2"/><c:pt idx="0"><c:v>Alpha</c:v></c:pt><c:pt idx="1"><c:v>Beta</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:f>Sheet1!$B$2:$B$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser></c:{local}>"#
    )
}

#[track_caller]
fn assert_reads_cat_val_series(space: &ChartSpace, kind: ChartKind) {
    assert_eq!(space.chart_kind(), Some(kind), "kind");
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .unwrap_or_else(|| panic!("{kind:?} should expose its series"));
    assert_eq!(series.name().as_deref(), Some("Series 1"));
    assert_eq!(
        series.categories().expect("categories").labels(),
        vec!["Alpha", "Beta"]
    );
    assert_eq!(series.values().expect("values").values(), vec![10.0, 20.0]);
}

#[test]
fn every_category_value_plot_type_reads_its_series_and_round_trips() {
    // Each entry is the element name, the scalars its schema puts before `c:ser`, and its kind.
    let cases: [(&str, &str, ChartKind); 8] = [
        (
            "bar3DChart",
            r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#,
            ChartKind::Bar3D,
        ),
        (
            "line3DChart",
            r#"<c:grouping val="standard"/>"#,
            ChartKind::Line3D,
        ),
        (
            "area3DChart",
            r#"<c:grouping val="standard"/>"#,
            ChartKind::Area3D,
        ),
        ("pie3DChart", r#"<c:varyColors val="1"/>"#, ChartKind::Pie3D),
        (
            "ofPieChart",
            r#"<c:ofPieType val="bar"/><c:varyColors val="1"/>"#,
            ChartKind::OfPie,
        ),
        (
            "radarChart",
            r#"<c:radarStyle val="filled"/>"#,
            ChartKind::Radar,
        ),
        ("stockChart", "", ChartKind::Stock),
        (
            "surfaceChart",
            r#"<c:wireframe val="1"/>"#,
            ChartKind::Surface,
        ),
    ];
    for (local, scalars, kind) in cases {
        let xml = wrap(&cat_val_plot(local, scalars));
        let (space, doc) = parse(&xml);
        assert_reads_cat_val_series(&space, kind);
        assert_round_trips(&space, doc, &xml);
    }
}

#[test]
fn a_surface_3d_plot_reads_its_series() {
    let xml = wrap(&cat_val_plot("surface3DChart", r#"<c:wireframe val="0"/>"#));
    let (space, doc) = parse(&xml);
    assert_reads_cat_val_series(&space, ChartKind::Surface3D);
    assert!(
        space
            .plot_area()
            .and_then(mjx_chart::PlotArea::surface_chart)
            .is_none(),
        "a surface3DChart is not a surfaceChart"
    );
    assert_round_trips(&space, doc, &xml);
}

#[test]
fn a_bubble_plot_reads_its_x_y_and_size_data() {
    let plot = r#"<c:bubbleChart><c:varyColors val="1"/><c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:v>Bubbles</c:v></c:tx><c:xVal><c:numRef><c:f>Sheet1!$A$2:$A$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numCache></c:numRef></c:xVal><c:yVal><c:numRef><c:f>Sheet1!$B$2:$B$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt></c:numCache></c:numRef></c:yVal><c:bubbleSize><c:numRef><c:f>Sheet1!$C$2:$C$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>4</c:v></c:pt><c:pt idx="1"><c:v>9</c:v></c:pt></c:numCache></c:numRef></c:bubbleSize></c:ser><c:bubbleScale val="150"/><c:showNegBubbles val="0"/><c:axId val="1"/><c:axId val="2"/></c:bubbleChart>"#;
    let xml = wrap(plot);
    let (space, doc) = parse(&xml);

    assert_eq!(space.chart_kind(), Some(ChartKind::Bubble));
    let area = space.plot_area().expect("plot area");
    let series = area.all_series().next().expect("a series");
    assert_eq!(series.name().as_deref(), Some("Bubbles"));
    assert_eq!(series.x_data().expect("x").values(), vec![1.0, 2.0]);
    assert_eq!(series.y_data().expect("y").values(), vec![10.0, 20.0]);
    assert_eq!(
        series.bubble_sizes().expect("sizes").values(),
        vec![4.0, 9.0],
        "the third data channel a bubble series carries"
    );

    let plot = area.bubble_chart().expect("bubble plot");
    assert_eq!(plot.bubble_scale(&doc.interner), Some(150));
    assert_eq!(plot.shows_negative_bubbles(&doc.interner), Some(false));
    assert_eq!(plot.vary_colors(&doc.interner), Some(true));
    assert_eq!(plot.axis_ids(&doc.interner), vec![1, 2]);

    assert_round_trips(&space, doc, &xml);
}

#[test]
fn the_type_specific_scalars_of_the_new_plots_read() {
    let xml = wrap(&format!(
        "{}{}",
        cat_val_plot("radarChart", r#"<c:radarStyle val="filled"/>"#),
        cat_val_plot(
            "ofPieChart",
            r#"<c:ofPieType val="bar"/><c:secondPieSize val="70"/>"#
        )
    ));
    let (space, doc) = parse(&xml);
    let area = space.plot_area().expect("plot area");

    assert_eq!(
        area.radar_chart()
            .expect("radar")
            .radar_style(&doc.interner),
        Some(RadarStyle::Filled)
    );
    let of_pie = area
        .content()
        .iter()
        .find_map(|item| match item {
            mjx_chart::PlotAreaContent::OfPie(plot) => Some(plot),
            _ => None,
        })
        .expect("of-pie plot");
    assert_eq!(of_pie.of_pie_type(&doc.interner), Some(OfPieType::Bar));
    assert_eq!(of_pie.second_plot_size(&doc.interner), Some(70));

    // A combo of two of the newly-modeled types reads as two kinds, in document order.
    assert_eq!(
        space.chart_kinds(),
        vec![ChartKind::Radar, ChartKind::OfPie]
    );
    assert_round_trips(&space, doc, &xml);
}

#[test]
fn an_unknown_plot_element_still_rides_through_the_raw_bucket() {
    // The seventeenth plot type does not exist, but a future one might: an element the model does
    // not name must still round-trip and must not be counted as a kind.
    let xml = wrap(r#"<c:hologramChart><c:ser><c:idx val="0"/></c:ser></c:hologramChart>"#);
    let (space, doc) = parse(&xml);
    assert!(
        space.chart_kinds().is_empty(),
        "an unknown plot element names no kind"
    );
    assert_eq!(
        space.plot_area().expect("plot area").all_series().count(),
        0,
        "and its series are not reachable"
    );
    assert_round_trips(&space, doc, &xml);
}
