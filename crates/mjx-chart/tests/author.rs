//! Authoring tests (MJX-47, tier C4): [`ChartData`] serializes a fresh `c:chartSpace` part with
//! cached data only, and every authored chart reads back through the C1/C2 model — the same kind,
//! series names, categories/labels and values it was built from. This closes the authoring path
//! against the read path.

use mjx_chart::{ChartData, ChartKind, ChartSpace};
use mjx_ooxml_core::FromXml;

/// Parses authored bytes back into the read model.
fn read_back(bytes: &[u8]) -> (ChartSpace, mjx_ooxml_core::Interner) {
    let doc = mjx_xml::fidelity::parse(bytes).expect("authored chart parses");
    let space = ChartSpace::from_xml(&doc.root, &doc.interner).expect("chart space reads");
    (space, doc.interner)
}

/// A category/value chart (bar) reads back with its kind, series names, shared categories and values.
#[test]
fn a_bar_chart_reads_back_its_series() {
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [10.0, 20.5, 15.0])
        .series("Cost", [5.0, 8.0, 7.25]);
    let (space, _) = read_back(&chart.to_part_bytes());

    assert_eq!(space.chart_kind(), Some(ChartKind::Bar));
    let area = space.plot_area().expect("plot area");
    let series: Vec<_> = area.all_series().collect();
    assert_eq!(series.len(), 2);

    assert_eq!(series[0].name().as_deref(), Some("Revenue"));
    assert_eq!(
        series[0]
            .categories()
            .map(|c| c.labels())
            .unwrap_or_default(),
        vec!["Q1", "Q2", "Q3"]
    );
    assert_eq!(
        series[0].values().map(|v| v.values()).unwrap_or_default(),
        vec![10.0, 20.5, 15.0]
    );
    assert_eq!(series[1].name().as_deref(), Some("Cost"));
    assert_eq!(
        series[1].values().map(|v| v.values()).unwrap_or_default(),
        vec![5.0, 8.0, 7.25]
    );
}

/// Every category/value kind authors a plot the read model recognizes as that kind, with its data.
#[test]
fn every_category_value_kind_round_trips() {
    for kind in [
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Area,
        ChartKind::Pie,
        ChartKind::Doughnut,
    ] {
        let chart = ChartData::new(kind)
            .categories(["A", "B"])
            .series("Series 1", [3.0, 4.0]);
        let (space, _) = read_back(&chart.to_part_bytes());

        assert_eq!(space.chart_kind(), Some(kind), "kind {kind:?} reads back");
        let series: Vec<_> = space.plot_area().expect("plot area").all_series().collect();
        assert_eq!(series.len(), 1, "kind {kind:?} has one series");
        assert_eq!(
            series[0]
                .categories()
                .map(|c| c.labels())
                .unwrap_or_default(),
            vec!["A", "B"],
            "kind {kind:?} categories"
        );
        assert_eq!(
            series[0].values().map(|v| v.values()).unwrap_or_default(),
            vec![3.0, 4.0],
            "kind {kind:?} values"
        );
    }
}

/// A scatter chart authors `c:xVal`/`c:yVal` (not `c:cat`/`c:val`): categories become numeric X
/// values, the series values become Y.
#[test]
fn a_scatter_chart_authors_xy_data() {
    let chart = ChartData::new(ChartKind::Scatter)
        .categories(["1", "2", "4"])
        .series("Points", [10.0, 40.0, 160.0]);
    let (space, _) = read_back(&chart.to_part_bytes());

    assert_eq!(space.chart_kind(), Some(ChartKind::Scatter));
    let series: Vec<_> = space.plot_area().expect("plot area").all_series().collect();
    assert_eq!(series.len(), 1);
    // Scatter uses xVal/yVal, so cat/val are absent.
    assert!(series[0].categories().is_none());
    assert!(series[0].values().is_none());
    assert_eq!(
        series[0].x_data().map(|x| x.values()).unwrap_or_default(),
        vec![1.0, 2.0, 4.0]
    );
    assert_eq!(
        series[0].y_data().map(|y| y.values()).unwrap_or_default(),
        vec![10.0, 40.0, 160.0]
    );
}

/// Non-numeric scatter categories fall back to the point position for X.
#[test]
fn scatter_x_falls_back_to_position_for_non_numeric_categories() {
    let chart = ChartData::new(ChartKind::Scatter)
        .categories(["Mon", "Tue", "Wed"])
        .series("Points", [7.0, 8.0, 9.0]);
    let (space, _) = read_back(&chart.to_part_bytes());
    let series: Vec<_> = space.plot_area().expect("plot area").all_series().collect();
    assert_eq!(
        series[0].x_data().map(|x| x.values()).unwrap_or_default(),
        vec![0.0, 1.0, 2.0]
    );
}

/// The authored part declares the chart namespace and carries no embedded workbook.
#[test]
fn authored_part_is_cached_only() {
    let bytes = ChartData::new(ChartKind::Bar)
        .categories(["A"])
        .series("S", [1.0])
        .to_part_bytes();
    let xml = std::str::from_utf8(&bytes).expect("utf-8");

    assert!(xml.starts_with(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#));
    assert!(
        xml.contains(r#"xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart""#),
        "declares the chart namespace"
    );
    // Cached data only — no embedded-workbook reference.
    assert!(!xml.contains("c:externalData"), "no embedded workbook");
    assert!(xml.contains("<c:numCache>"), "caches numeric values");
    assert!(xml.contains("<c:strCache>"), "caches category labels");
}

/// A chart with no series (or only empty series) is reported empty, so a caller can reject it.
#[test]
fn empty_charts_are_reported_empty() {
    assert!(ChartData::new(ChartKind::Bar).is_empty());
    assert!(ChartData::new(ChartKind::Bar)
        .categories(["A"])
        .series("Empty", [])
        .is_empty());
    assert!(!ChartData::new(ChartKind::Bar)
        .categories(["A"])
        .series("S", [1.0])
        .is_empty());
}
