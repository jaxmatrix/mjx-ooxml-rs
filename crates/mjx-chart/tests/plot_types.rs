//! Tests for the C2 plot types — line, pie, area, scatter, doughnut — and combo charts, through the
//! public API only. The bar plot has its own suite in `bar_model.rs`; here the focus is that each
//! added plot type reads its series and round-trips, that scatter's `xVal`/`yVal` are read (it has no
//! `cat`/`val`), and that a plot area holding more than one plot (a combo chart) is read as such.

use mjx_chart::{ChartKind, ChartSpace};
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

/// A category/value plot element `local` carrying `scalars` (its type-specific children) then one
/// series "Series 1" over Alpha/Beta = 10/20 — the shape bar, line, pie, area and doughnut share.
fn cat_val_plot(local: &str, scalars: &str) -> String {
    format!(
        r#"<c:{local}>{scalars}<c:ser><c:idx val="0"/><c:order val="0"/><c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>Series 1</c:v></c:pt></c:strCache></c:strRef></c:tx><c:cat><c:strRef><c:f>Sheet1!$A$2:$A$3</c:f><c:strCache><c:ptCount val="2"/><c:pt idx="0"><c:v>Alpha</c:v></c:pt><c:pt idx="1"><c:v>Beta</c:v></c:pt></c:strCache></c:strRef></c:cat><c:val><c:numRef><c:f>Sheet1!$B$2:$B$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>10</c:v></c:pt><c:pt idx="1"><c:v>20</c:v></c:pt></c:numCache></c:numRef></c:val></c:ser></c:{local}>"#
    )
}

// ---------------------------------------------------------------------------------------------
// Category/value plot types
// ---------------------------------------------------------------------------------------------

#[track_caller]
fn assert_reads_cat_val_series(space: &ChartSpace, kind: ChartKind) {
    assert_eq!(space.chart_kind(), Some(kind));
    let series = space
        .plot_area()
        .expect("plot area")
        .all_series()
        .next()
        .expect("a series");
    assert_eq!(series.name().as_deref(), Some("Series 1"));
    assert_eq!(
        series.categories().expect("categories").labels(),
        vec!["Alpha", "Beta"]
    );
    assert_eq!(series.values().expect("values").values(), vec![10.0, 20.0]);
}

#[test]
fn reads_a_line_chart() {
    let source = wrap(&cat_val_plot(
        "lineChart",
        r#"<c:grouping val="standard"/>"#,
    ));
    let (space, doc) = parse(&source);

    assert_reads_cat_val_series(&space, ChartKind::Line);
    assert!(space.plot_area().unwrap().line_chart().is_some());
    assert_round_trips(&space, doc, &source);
}

#[test]
fn reads_a_pie_chart() {
    let source = wrap(&cat_val_plot("pieChart", r#"<c:varyColors val="1"/>"#));
    let (space, doc) = parse(&source);

    assert_reads_cat_val_series(&space, ChartKind::Pie);
    assert!(space.plot_area().unwrap().pie_chart().is_some());
    assert_round_trips(&space, doc, &source);
}

#[test]
fn reads_an_area_chart() {
    let source = wrap(&cat_val_plot("areaChart", r#"<c:grouping val="stacked"/>"#));
    let (space, doc) = parse(&source);

    assert_reads_cat_val_series(&space, ChartKind::Area);
    assert!(space.plot_area().unwrap().area_chart().is_some());
    assert_round_trips(&space, doc, &source);
}

#[test]
fn reads_a_doughnut_chart() {
    let source = wrap(&cat_val_plot("doughnutChart", r#"<c:holeSize val="50"/>"#));
    let (space, doc) = parse(&source);

    assert_reads_cat_val_series(&space, ChartKind::Doughnut);
    assert!(space.plot_area().unwrap().doughnut_chart().is_some());
    assert_round_trips(&space, doc, &source);
}

// ---------------------------------------------------------------------------------------------
// Scatter — the one type with xVal/yVal instead of cat/val
// ---------------------------------------------------------------------------------------------

#[test]
fn reads_a_scatter_chart_x_and_y_data() {
    let source = wrap(concat!(
        r#"<c:scatterChart><c:scatterStyle val="lineMarker"/>"#,
        r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#,
        r#"<c:tx><c:strRef><c:f>Sheet1!$B$1</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>XY</c:v></c:pt></c:strCache></c:strRef></c:tx>"#,
        r#"<c:xVal><c:numRef><c:f>Sheet1!$A$2:$A$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>1</c:v></c:pt><c:pt idx="1"><c:v>2</c:v></c:pt></c:numCache></c:numRef></c:xVal>"#,
        r#"<c:yVal><c:numRef><c:f>Sheet1!$B$2:$B$3</c:f><c:numCache><c:ptCount val="2"/><c:pt idx="0"><c:v>3.5</c:v></c:pt><c:pt idx="1"><c:v>4.5</c:v></c:pt></c:numCache></c:numRef></c:yVal>"#,
        r#"</c:ser></c:scatterChart>"#,
    ));
    let (space, doc) = parse(&source);

    assert_eq!(space.chart_kind(), Some(ChartKind::Scatter));
    let series = space
        .plot_area()
        .unwrap()
        .scatter_chart()
        .unwrap()
        .series_at(0)
        .unwrap();

    // A scatter series has no cat/val — it carries xVal/yVal.
    assert!(series.categories().is_none());
    assert!(series.values().is_none());
    assert_eq!(series.name().as_deref(), Some("XY"));
    assert_eq!(series.x_data().expect("xVal").values(), vec![1.0, 2.0]);
    assert_eq!(series.y_data().expect("yVal").values(), vec![3.5, 4.5]);
    assert_round_trips(&space, doc, &source);
}

// ---------------------------------------------------------------------------------------------
// Combo — more than one plot in a single plot area
// ---------------------------------------------------------------------------------------------

#[test]
fn reads_a_combo_bar_and_line_chart() {
    let body = format!(
        "{}{}",
        cat_val_plot(
            "barChart",
            r#"<c:barDir val="col"/><c:grouping val="clustered"/>"#
        ),
        cat_val_plot("lineChart", r#"<c:grouping val="standard"/>"#),
    );
    let source = wrap(&body);
    let (space, doc) = parse(&source);

    let plot_area = space.plot_area().expect("plot area");
    assert_eq!(
        plot_area.chart_kinds(),
        vec![ChartKind::Bar, ChartKind::Line]
    );
    assert_eq!(space.chart_kind(), Some(ChartKind::Bar), "first plot");
    assert!(plot_area.bar_chart().is_some());
    assert!(plot_area.line_chart().is_some());

    // Both plots' series flatten into one sequence.
    let names: Vec<_> = plot_area
        .all_series()
        .map(|s| s.name().unwrap_or_default())
        .collect();
    assert_eq!(names, vec!["Series 1", "Series 1"]);
    assert_round_trips(&space, doc, &source);
}

// ---------------------------------------------------------------------------------------------
// Fidelity — type-specific scalars and axes survive
// ---------------------------------------------------------------------------------------------

#[test]
fn plot_specific_scalars_and_axes_survive() {
    // A pie with firstSliceAng and an extLst, plus an unknown attribute on the space: none of these
    // are interpreted, and all must come back out.
    let source = concat!(
        r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" unknownAttr="kept">"#,
        r#"<c:chart><c:plotArea><c:pieChart><c:varyColors val="1"/>"#,
        r#"<c:ser><c:idx val="0"/><c:order val="0"/>"#,
        r#"<c:cat><c:strRef><c:f>Sheet1!$A$2</c:f><c:strCache><c:ptCount val="1"/><c:pt idx="0"><c:v>One</c:v></c:pt></c:strCache></c:strRef></c:cat>"#,
        r#"<c:val><c:numRef><c:f>Sheet1!$B$2</c:f><c:numCache><c:ptCount val="1"/><c:pt idx="0"><c:v>7</c:v></c:pt></c:numCache></c:numRef></c:val>"#,
        r#"</c:ser><c:firstSliceAng val="90"/></c:pieChart>"#,
        r#"<c:extLst><c:ext uri="x"/></c:extLst></c:plotArea></c:chart></c:chartSpace>"#,
    );
    let (space, doc) = parse(source);

    // Read through the model first, so the round-trip is not passing by never looking.
    assert_eq!(space.chart_kind(), Some(ChartKind::Pie));
    assert_eq!(
        space
            .plot_area()
            .unwrap()
            .pie_chart()
            .unwrap()
            .series_at(0)
            .unwrap()
            .values()
            .unwrap()
            .values(),
        vec![7.0]
    );
    assert_round_trips(&space, doc, source);
}
