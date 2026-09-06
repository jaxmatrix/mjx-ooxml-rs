//! The workbook a chart embeds: that the cells it fills are the cells the chart's formulas name.
//!
//! # Where these came from, and why they are here
//!
//! These assertions were `crates/mjx-chart/tests/workbook.rs` until MJXOFF-99 deleted
//! `mjx-chart`'s minimal SpreadsheetML writer. `mjx-chart` now lays out the grid and `mjx-sml`
//! writes it, and `mjx-chart` is required to hold *no* SpreadsheetML at all — not an element name,
//! not a part name, not in a test. `mjx-pptx` is the crate that embeds the result, sees both
//! `mjx-chart` and `mjx-sml`, and is where the two meeting can be checked.
//!
//! # What makes these discriminating
//!
//! Asserting that a workbook we just wrote contains what our own writer put in it proves nothing.
//! Every layout assertion below is anchored to something stated independently:
//!
//! * the chart's own `c:f` formulas, **read off the authored chart part** rather than restated here,
//!   so the workbook and the chart cannot drift apart without this failing;
//! * `sml.xsd` itself, through `tests/schema_validity.rs`, which descends into the nested package;
//! * the values the caller handed the chart, which neither writer chose.
//!
//! What the `mjx-sml` writer emits *in general* — the part list, the relationship graph, the styles
//! skeleton, first-use interning, `dimension`, determinism — is asserted against the bytes in
//! `crates/mjx-sml/tests/package_writer.rs`. This file asserts only the half that is about a chart.

use mjx_chart::{
    embedded_workbook_for_chart_data, embedded_workbook_for_chart_space, ChartData, ChartKind,
    ChartSpace,
};
use mjx_ooxml_core::FromXml;
use mjx_opc::{Package, PartName};

/// One part of a written workbook, as text.
fn part_text(workbook: &[u8], name: &str) -> String {
    let package = Package::open(workbook).expect("the workbook is an OPC package");
    let bytes = package
        .entries()
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.bytes().map(<[u8]>::to_vec))
        .unwrap_or_else(|| panic!("the workbook has no {name}"));
    String::from_utf8(bytes).expect("utf-8")
}

fn bar_chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["North", "South", "West"])
        .series("Sales", [19.2, 21.4, 16.7])
        .series("Costs", [9.0, 8.5, 7.25])
}

fn bar_chart_workbook() -> Vec<u8> {
    embedded_workbook_for_chart_data(&bar_chart()).expect("write the workbook")
}

#[test]
fn a_written_workbook_is_a_complete_package() {
    let bytes = bar_chart_workbook();
    let package = Package::open(&bytes).expect("the workbook is an OPC package");
    let names: Vec<&str> = package
        .entries()
        .iter()
        .map(|entry| entry.name.as_str())
        .collect();
    for expected in [
        "[Content_Types].xml",
        "_rels/.rels",
        "xl/workbook.xml",
        "xl/worksheets/sheet1.xml",
        "xl/sharedStrings.xml",
        "xl/styles.xml",
        "xl/_rels/workbook.xml.rels",
    ] {
        assert!(names.contains(&expected), "missing {expected}: {names:?}");
    }

    // Every part is typed, and the workbook part carries the content type Office looks for.
    assert_eq!(
        package.content_type_of(&PartName::new("/xl/workbook.xml").expect("part name")),
        Some("application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml")
    );
    // The package-root relationship points at the workbook, which is how Office finds it.
    let root = package
        .relationships_for(None)
        .expect("the package root has relationships");
    assert!(
        root.iter().any(|rel| rel.target == "xl/workbook.xml"
            && rel.rel_type.ends_with("/officeDocument")),
        "the root relates to the workbook"
    );
}

#[test]
fn the_workbook_fills_exactly_the_cells_the_chart_names() {
    let chart = bar_chart();
    let sheet = part_text(&bar_chart_workbook(), "xl/worksheets/sheet1.xml");

    // The chart's formulas are `Sheet1!$A$2:$A$4` for the categories and `$B$2:$B$4` / `$C$2:$C$4`
    // for the two series. Read them off the authored part rather than restating them here, so the
    // two cannot drift apart without this failing.
    let part = chart.to_part_bytes();
    let document = mjx_xml::fidelity::parse(&part).expect("the chart part parses");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    let area = space.plot_area().expect("plot area");
    let formulas: Vec<String> = area
        .all_series()
        .filter_map(|series| {
            series
                .values()?
                .reference()?
                .formula()
                .map(mjx_chart::Formula::text)
        })
        .collect();
    assert_eq!(formulas, ["Sheet1!$B$2:$B$4", "Sheet1!$C$2:$C$4"]);

    // Every value cell the formulas name is written, with the value the chart caches.
    for (cell, value) in [
        ("B2", "19.2"),
        ("B3", "21.4"),
        ("B4", "16.7"),
        ("C2", "9"),
        ("C3", "8.5"),
        ("C4", "7.25"),
    ] {
        assert!(
            sheet.contains(&format!(r#"<c r="{cell}"><v>{value}</v></c>"#)),
            "cell {cell} should hold {value}: {sheet}"
        );
    }
    // The sheet declares the range it fills.
    assert!(sheet.contains(r#"<dimension ref="A1:C4"/>"#), "{sheet}");

    // The labels go through the shared-string table, in first-use order: the series names sit in
    // row 1 and the categories in column A.
    let strings = part_text(&bar_chart_workbook(), "xl/sharedStrings.xml");
    assert!(
        strings.contains(
            r#"<si><t>Sales</t></si><si><t>Costs</t></si><si><t>North</t></si><si><t>South</t></si><si><t>West</t></si>"#
        ),
        "{strings}"
    );
    assert!(
        strings.contains(r#"count="5" uniqueCount="5""#),
        "{strings}"
    );
    // A string cell names its index in that table, not the text.
    assert!(sheet.contains(r#"<c r="A2" t="s"><v>2</v></c>"#), "{sheet}");
}

#[test]
fn a_scatter_chart_writes_numeric_x_values_not_labels() {
    // A scatter series' `c:xVal` is a `c:numRef`: writing its column as text would make the
    // workbook disagree with the chart about what kind of data it holds.
    let chart = ChartData::new(ChartKind::Scatter)
        .categories(["1.5", "2.5"])
        .series("Points", [10.0, 20.0]);
    let sheet = part_text(
        &embedded_workbook_for_chart_data(&chart).expect("write"),
        "xl/worksheets/sheet1.xml",
    );
    assert!(sheet.contains(r#"<c r="A2"><v>1.5</v></c>"#), "{sheet}");
    assert!(sheet.contains(r#"<c r="A3"><v>2.5</v></c>"#), "{sheet}");
    assert!(
        !sheet.contains(r#"<c r="A2" t="s">"#),
        "an X value is a number, not a shared string: {sheet}"
    );
}

#[test]
fn the_workbook_of_an_existing_chart_is_read_from_its_caches() {
    // The refresh path: what a chart *now* draws, whatever it drew when it was written.
    let part = bar_chart().to_part_bytes();
    let document = mjx_xml::fidelity::parse(&part).expect("parse");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");

    let from_space = embedded_workbook_for_chart_space(&space).expect("write");
    let from_data = bar_chart_workbook();
    assert_eq!(
        from_space, from_data,
        "reading a chart back gives the same workbook the chart was authored from"
    );
}

/// A row that fills nothing still occupies its number, so the grid stays where the formulas say.
///
/// A chart read back from a file whose series declare no `c:tx` has an entirely blank header row:
/// the corner cell is blank by construction and every other cell is the missing name. If that row
/// did not consume row 1, the categories would slide up to row 1 and the chart's own
/// `Sheet1!$A$2:$A$3` would name the wrong cells — an embedded workbook that disagrees with the
/// chart it backs, which is worse than no workbook at all.
///
/// The `c:tx` is removed from an authored part rather than asked for through `ChartData`, because
/// `ChartData` always writes one; a third-party chart need not, and this is the refresh path that
/// reads one. Anchored on the formula read off that same part, not on a restatement of it.
#[test]
fn an_unnamed_series_still_leaves_the_data_where_the_formulas_point() {
    let authored = ChartData::new(ChartKind::Bar)
        .categories(["North", "South"])
        .series("Sales", [19.2, 21.4])
        .to_part_bytes();
    let text = String::from_utf8(authored).expect("utf-8");
    let opening = text
        .find("<c:tx>")
        .expect("an authored series names itself");
    let closing = text.find("</c:tx>").expect("the name element closes") + "</c:tx>".len();
    let stripped = format!("{}{}", &text[..opening], &text[closing..]);

    let document = mjx_xml::fidelity::parse(stripped.as_bytes()).expect("parse");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    let area = space.plot_area().expect("plot area");
    let series_names: Vec<Option<String>> = area.all_series().map(|series| series.name()).collect();
    assert_eq!(
        series_names,
        [None],
        "the header row has nothing at all to write"
    );

    let categories: Vec<String> = area
        .all_series()
        .filter_map(|series| {
            series
                .categories()?
                .string_reference()?
                .formula()
                .map(mjx_chart::Formula::text)
        })
        .collect();
    assert_eq!(categories, ["Sheet1!$A$2:$A$3"], "the chart says row 2");

    let sheet = part_text(
        &embedded_workbook_for_chart_space(&space).expect("write"),
        "xl/worksheets/sheet1.xml",
    );
    assert!(
        sheet.contains(r#"<row r="2"><c r="A2" t="s"><v>0</v></c>"#),
        "the categories start on row 2, not row 1: {sheet}"
    );
    assert!(
        !sheet.contains(r#"<row r="1">"#),
        "and the blank header row writes no <row> at all: {sheet}"
    );
}

#[test]
fn a_ragged_or_empty_grid_writes_a_valid_sheet() {
    // A series shorter than the categories leaves blanks rather than inventing zeros.
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["A", "B"])
        .series("S", [1.0]);
    let sheet = part_text(
        &embedded_workbook_for_chart_data(&chart).expect("write"),
        "xl/worksheets/sheet1.xml",
    );
    assert!(
        sheet.contains(r#"<row r="3"><c r="A3" t="s"><v>2</v></c></row>"#),
        "{sheet}"
    );

    // A non-finite value has no SpreadsheetML spelling, so its cell is simply not written.
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["A"])
        .series("S", [f64::NAN]);
    let sheet = part_text(
        &embedded_workbook_for_chart_data(&chart).expect("write"),
        "xl/worksheets/sheet1.xml",
    );
    assert!(!sheet.contains("NaN"), "{sheet}");
    assert!(
        !sheet.contains(r#"<c r="B2""#),
        "a value with no spelling writes no cell: {sheet}"
    );

    // A chart with nothing in it is still a package that opens.
    let empty = embedded_workbook_for_chart_data(&ChartData::new(ChartKind::Bar)).expect("write");
    let sheet = part_text(&empty, "xl/worksheets/sheet1.xml");
    assert!(sheet.contains("<sheetData/>"), "{sheet}");
    assert!(!sheet.contains("<dimension"), "no range is filled: {sheet}");
}

#[test]
fn labels_needing_escaping_survive_the_round_trip() {
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["R&D", "<Ops>"])
        .series("A & B", [1.0, 2.0]);
    let strings = part_text(
        &embedded_workbook_for_chart_data(&chart).expect("write"),
        "xl/sharedStrings.xml",
    );
    assert!(strings.contains("<t>A &amp; B</t>"), "{strings}");
    assert!(strings.contains("<t>R&amp;D</t>"), "{strings}");
    // `>` is legal unescaped in character data, and the writer leaves it alone rather than
    // gratuitously rewriting text it was handed.
    assert!(strings.contains("<t>&lt;Ops></t>"), "{strings}");
}

/// Two calls produce the same bytes, or every round-trip assertion downstream is flaky.
///
/// `add_chart` stores these bytes in the package and `refresh_chart_workbook` replaces them; a
/// writer that varied between calls would make a no-op edit dirty the workbook part and break the
/// tier-3 isolation assertions in `tests/charts.rs`.
#[test]
fn writing_the_same_chart_twice_produces_the_same_bytes() {
    assert_eq!(bar_chart_workbook(), bar_chart_workbook());

    let part = bar_chart().to_part_bytes();
    let document = mjx_xml::fidelity::parse(&part).expect("parse");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    assert_eq!(
        embedded_workbook_for_chart_space(&space).expect("write"),
        embedded_workbook_for_chart_space(&space).expect("write"),
    );
}
