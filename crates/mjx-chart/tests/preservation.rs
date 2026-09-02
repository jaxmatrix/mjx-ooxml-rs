//! Tier-1 fidelity against a chart this project did **not** write.
//!
//! `tests/fixtures/charts.pptx` is python-pptx's template deck: its chart part carries axes, a
//! `c:txPr`, a `c:externalData`, negative `c:axId` values and a shape of markup nobody here chose.
//! This tier promoted ten plot types and all four axis types from the `Raw` bucket into typed
//! models, and a typed model that re-emits even one byte differently is a regression, however
//! correct its accessors.
//!
//! The assertion is therefore the strongest available: parse the real part, hand every element to
//! the model, write it back, and require the decompressed payload to be **byte-identical**.

use std::path::PathBuf;

use mjx_chart::ChartSpace;
use mjx_ooxml_core::{FromXml, ToXml};
use mjx_opc::{Package, PartName};

/// The chart part of the `charts.pptx` fixture, exactly as the package holds it.
fn fixture_chart_part() -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/charts.pptx");
    let bytes =
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));
    let package = Package::open(&bytes).expect("the fixture opens");
    package
        .part_bytes(&PartName::new("/ppt/charts/chart1.xml").expect("part name"))
        .expect("the fixture has a chart part")
        .to_vec()
}

#[test]
fn a_producer_written_chart_part_round_trips_byte_for_byte() {
    let original = fixture_chart_part();
    let mut document = mjx_xml::fidelity::parse(&original).expect("the chart part parses");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    document.root = space.to_xml(&mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);

    assert_eq!(
        String::from_utf8_lossy(&written),
        String::from_utf8_lossy(&original),
        "modelling the axes and the remaining plot types may add reach, never change what is \
         written back"
    );
}

#[test]
fn the_newly_typed_children_of_that_part_are_reachable() {
    // The counterpart of the byte-identity assertion: the round-trip would also pass if nothing had
    // been modeled at all, so this pins that the axes really are typed now.
    let original = fixture_chart_part();
    let document = mjx_xml::fidelity::parse(&original).expect("parse");
    let space = ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    let area = space.plot_area().expect("plot area");

    let axes: Vec<_> = area.axes().collect();
    assert_eq!(axes.len(), 2, "the fixture's chart draws against two axes");
    assert_eq!(axes[0].0, mjx_chart::AxisKind::Category);
    assert_eq!(axes[1].0, mjx_chart::AxisKind::Value);
    assert_eq!(
        axes[1].1.position(&document.interner),
        Some(mjx_chart::AxisPosition::Left)
    );
    assert!(
        axes[1].1.has_major_gridlines(),
        "the value axis rules gridlines"
    );

    // python-pptx derives its axis ids from a signed hash, so they do not fit `xs:unsignedInt`.
    // Reading them answers `None` rather than pretending — and, crucially, the bytes above prove we
    // write them back exactly as they came.
    assert_eq!(
        axes[0].1.axis_id(&document.interner),
        None,
        "a negative axis id is not an unsigned integer, and is not silently coerced into one"
    );
    assert!(
        String::from_utf8_lossy(&original).contains(r#"<c:axId val="-2068027336"/>"#),
        "the fixture really does carry a negative axis id"
    );

    // And the chart's own workbook reference reads.
    assert_eq!(space.external_data_rel_id(&document.interner), Some("rId1"));
}
