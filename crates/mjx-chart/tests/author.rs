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

// =================================================================================================
// Live-range data sources (MJXOFF-111, E4)
// =================================================================================================

/// A chart told where its data lives writes **those** formulas, not the embedded workbook's.
///
/// This is the whole of what `ChartData::ranges` changes. The default `Sheet1!$A$2:$A$4` names a
/// workbook this library writes beside the chart; a chart on a worksheet has no such workbook, and a
/// formula naming one would send a consumer looking for a part that is not there.
#[test]
fn a_chart_given_ranges_names_them_instead_of_the_embedded_workbook() {
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["North", "South", "East"])
        .series("Revenue", [10.0, 20.0, 30.0])
        .series("Cost", [1.0, 2.0, 3.0])
        .ranges(mjx_chart::ChartRanges {
            categories: Some("Data!$A$2:$A$4".to_owned()),
            series: vec![
                mjx_chart::ChartSeriesRange {
                    name: Some("Data!$B$1".to_owned()),
                    values: "Data!$B$2:$B$4".to_owned(),
                },
                mjx_chart::ChartSeriesRange {
                    name: None,
                    values: "Data!$C$2:$C$4".to_owned(),
                },
            ],
        });
    let bytes = chart.to_part_bytes();
    let xml = String::from_utf8(bytes.clone()).expect("utf-8");

    for expected in [
        "<c:f>Data!$A$2:$A$4</c:f>",
        "<c:f>Data!$B$1</c:f>",
        "<c:f>Data!$B$2:$B$4</c:f>",
        "<c:f>Data!$C$2:$C$4</c:f>",
    ] {
        assert!(
            xml.contains(expected),
            "the part must carry {expected}: {xml}"
        );
    }
    assert!(
        !xml.contains("Sheet1!"),
        "no formula may still name the embedded workbook: {xml}"
    );

    // The named series takes the `c:strRef` shape — a reference plus the cache of what it says —
    // while the unnamed one keeps the literal `c:tx > c:v`. The two are not interchangeable: a
    // literal name is not something a consumer can refresh from a cell.
    assert!(
        xml.contains("<c:tx><c:strRef><c:f>Data!$B$1</c:f>"),
        "a series with a name range writes a c:strRef: {xml}"
    );
    assert!(
        xml.contains("<c:tx><c:v>Cost</c:v></c:tx>"),
        "a series without one keeps the literal name: {xml}"
    );

    // …and the whole thing still reads back through the read model, names and all.
    let (space, _) = read_back(&bytes);
    let area = space.plot_area().expect("plot area");
    let series: Vec<_> = area.all_series().collect();
    assert_eq!(series[0].name().as_deref(), Some("Revenue"));
    assert_eq!(series[1].name().as_deref(), Some("Cost"));
    assert_eq!(
        series[0].values().map(|v| v.values()).unwrap_or_default(),
        vec![10.0, 20.0, 30.0]
    );
}

/// A series `ChartRanges` says nothing about keeps the embedded workbook's own formula.
///
/// A partial description is a coherent one — filling in the first series and leaving the second
/// alone must not silently drop the second series' reference.
/// A source a `ChartRanges` does not name is written as a **literal**, never as a reference to the
/// companion workbook.
///
/// The alternative — falling back to `Sheet1!$A$2:$A$N` — writes a formula naming a part that is not
/// in the package, because a chart with ranges has no companion workbook. MJXOFF-111 found that the
/// hard way: a range chart with no category range wrote `Sheet1!$A$2:$A$1`, which happened to
/// resolve against the *host* workbook's own first sheet and made a freshness report claim the
/// chart's categories disagreed with cells it had never named.
#[test]
fn a_source_with_no_range_is_written_as_a_literal_rather_than_naming_the_embedded_workbook() {
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["A", "B"])
        .series("First", [1.0, 2.0])
        .series("Second", [3.0, 4.0])
        .ranges(mjx_chart::ChartRanges {
            categories: None,
            series: vec![mjx_chart::ChartSeriesRange {
                name: None,
                values: "Data!$B$2:$B$3".to_owned(),
            }],
        });
    let bytes = chart.to_part_bytes();
    let xml = String::from_utf8(bytes.clone()).expect("utf-8");
    assert!(xml.contains("<c:f>Data!$B$2:$B$3</c:f>"), "{xml}");
    assert!(
        !xml.contains("Sheet1!"),
        "no source may name the companion workbook once ranges are given: {xml}"
    );
    // The named series keeps its `c:numRef`; the unnamed one and the categories become literals.
    assert!(xml.contains("<c:numRef>"), "{xml}");
    assert!(xml.contains("<c:numLit>"), "{xml}");
    assert!(xml.contains("<c:strLit>"), "{xml}");
    assert!(!xml.contains("<c:strRef>"), "{xml}");

    // …and every value still reads back, because a literal is a source like any other.
    let (space, _) = read_back(&bytes);
    let area = space.plot_area().expect("plot area");
    let series: Vec<_> = area.all_series().collect();
    assert_eq!(
        series[0].values().map(|v| v.values()).unwrap_or_default(),
        vec![1.0, 2.0]
    );
    assert_eq!(
        series[1].values().map(|v| v.values()).unwrap_or_default(),
        vec![3.0, 4.0]
    );
    assert_eq!(
        series[0]
            .categories()
            .map(|c| c.labels())
            .unwrap_or_default(),
        vec!["A", "B"]
    );
}
