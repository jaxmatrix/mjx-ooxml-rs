//! The embedded workbook a chart carries (MJX-116, part 1).
//!
//! The chart's `c:f` formulas name cells; until this tier there was no workbook behind them, so
//! PowerPoint's *Edit Data* had nothing to open. These tests assert the two agree — that the range
//! the chart names is the range the workbook fills, cell for cell — because a workbook that is
//! merely *present* and disagrees is worse than none at all.

use mjx_chart::{
    ChartData, ChartKind, ChartSpace, EmbeddedWorkbook, WorkbookCell, CONTENT_TYPE_WORKBOOK_PACKAGE,
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

fn bar_chart_workbook() -> Vec<u8> {
    EmbeddedWorkbook::for_chart_data(&bar_chart())
        .to_package_bytes()
        .expect("write the workbook")
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
        &EmbeddedWorkbook::for_chart_data(&chart)
            .to_package_bytes()
            .expect("write"),
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

    let from_space = EmbeddedWorkbook::for_chart_space(&space);
    let from_data = EmbeddedWorkbook::for_chart_data(&bar_chart());
    assert_eq!(
        from_space, from_data,
        "reading a chart back gives the same grid the chart was authored from"
    );
}

#[test]
fn a_ragged_or_empty_grid_writes_a_valid_sheet() {
    // A series shorter than the categories leaves blanks rather than inventing zeros.
    let mut workbook = EmbeddedWorkbook::new("Sheet1");
    workbook.push_row(vec![WorkbookCell::Blank, WorkbookCell::text("S")]);
    workbook.push_row(vec![WorkbookCell::text("A"), WorkbookCell::Number(1.0)]);
    workbook.push_row(vec![WorkbookCell::text("B")]);
    let bytes = workbook.to_package_bytes().expect("write");
    let sheet = part_text(&bytes, "xl/worksheets/sheet1.xml");
    assert!(
        sheet.contains(r#"<row r="3"><c r="A3" t="s"><v>2</v></c></row>"#),
        "{sheet}"
    );

    // A non-finite value has no SpreadsheetML spelling, so its cell is simply not written.
    let mut workbook = EmbeddedWorkbook::new("Sheet1");
    workbook.push_row(vec![WorkbookCell::Number(f64::NAN)]);
    let sheet = part_text(
        &workbook.to_package_bytes().expect("write"),
        "xl/worksheets/sheet1.xml",
    );
    assert!(!sheet.contains("NaN"), "{sheet}");

    // An empty workbook is still a package that opens.
    let empty = EmbeddedWorkbook::new("Sheet1")
        .to_package_bytes()
        .expect("write");
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
        &EmbeddedWorkbook::for_chart_data(&chart)
            .to_package_bytes()
            .expect("write"),
        "xl/sharedStrings.xml",
    );
    assert!(strings.contains("<t>A &amp; B</t>"), "{strings}");
    assert!(strings.contains("<t>R&amp;D</t>"), "{strings}");
    // `>` is legal unescaped in character data, and the writer leaves it alone rather than
    // gratuitously rewriting text it was handed.
    assert!(strings.contains("<t>&lt;Ops></t>"), "{strings}");
}

#[test]
fn the_content_type_constant_is_the_one_a_host_package_must_register() {
    assert_eq!(
        CONTENT_TYPE_WORKBOOK_PACKAGE,
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );
}

#[test]
fn a_chart_can_name_its_workbook() {
    let part = bar_chart().to_part_bytes_linking_workbook("rId1");
    let text = String::from_utf8(part.clone()).expect("utf-8");
    assert!(
        text.contains(r#"<c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData>"#),
        "{text}"
    );
    // `CT_ChartSpace` puts `c:externalData` after `c:chart`.
    assert!(
        text.find("</c:chart>").expect("chart") < text.find("<c:externalData").expect("external"),
        "{text}"
    );

    let document = mjx_xml::fidelity::parse(&part).expect("parse");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    assert_eq!(space.external_data_rel_id(&document.interner), Some("rId1"));

    // Without the workbook, no reference is written at all.
    let bare = bar_chart().to_part_bytes();
    let document = mjx_xml::fidelity::parse(&bare).expect("parse");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    assert_eq!(space.external_data_rel_id(&document.interner), None);
}
