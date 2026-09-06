//! `c:externalData` — how a chart names the workbook that backs it.
//!
//! This is the half of `crates/mjx-chart/tests/workbook.rs` that survived MJXOFF-99 in this crate.
//! The other half asserted what the embedded workbook's *cells* hold, and moved to
//! `crates/mjx-pptx/tests/chart_workbook.rs` with the writer: `mjx-chart` lays out the grid and
//! `mjx-sml` writes it, so this crate holds no SpreadsheetML — not an element name, not a part name,
//! not in a test. What is left is purely DrawingML: the reference, its rank, and its absence.

use mjx_chart::{ChartData, ChartKind, ChartSpace};
use mjx_ooxml_core::FromXml;

fn bar_chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["North", "South", "West"])
        .series("Sales", [19.2, 21.4, 16.7])
        .series("Costs", [9.0, 8.5, 7.25])
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
