//! Integration tests for chart graphic frames (MJX-47, tier C0): recognizing a `p:graphicFrame` that
//! frames a chart, resolving its `c:chart@r:id` to the separate chart part, and reading that part's
//! bytes — all without modeling the chart XML, and all with fidelity (a chart deck round-trips
//! byte-identically, and editing another slide leaves every chart part untouched).
//!
//! The fixture `charts.pptx` has two slides: slide 1 carries a plain text box ("Edit me"); slide 2
//! carries one clustered-column chart whose `p:graphicFrame > a:graphicData > c:chart` points at
//! `/ppt/charts/chart1.xml`, which in turn relates to an embedded `.xlsx` workbook.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{ColorSpec, FillSpec, LineSpec, LineWidth};
use mjx_opc::{Package, PartName, TargetMode};
use mjx_pptx::{
    AxisKind, AxisOrientation, AxisPosition, ChartData, ChartKind, ChartLabelScope,
    ChartSeriesData, ChartWorkbook, DanglingPointReference, DataLabelPosition, DataLabelSpec,
    ErrorBarDirection, ErrorBarSpec, ErrorBarType, ErrorValueType, GraphicFrameKind,
    LegendPosition, PptxError, Presentation, ShapeBounds, TrendlineKind, TrendlineSpec,
};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

fn part(name: &str) -> PartName {
    PartName::new(name).expect("valid part name")
}

/// One part of an embedded workbook, as text — the workbook is a nested OPC package, so reading it
/// means opening it.
fn workbook_part(workbook: &[u8], name: &str) -> String {
    let pkg = Package::open(workbook).expect("the embedded workbook is a package");
    let bytes = pkg
        .entries()
        .iter()
        .find(|entry| entry.name == name)
        .and_then(|entry| entry.bytes().map(<[u8]>::to_vec))
        .unwrap_or_else(|| panic!("the workbook has no {name}"));
    String::from_utf8(bytes).expect("utf-8")
}

/// The embedded workbook's one worksheet, as text.
fn workbook_sheet(workbook: &[u8]) -> String {
    workbook_part(workbook, "xl/worksheets/sheet1.xml")
}

/// The chart frame is on slide 2 (surface index 1), shape 0; slide 1 (surface 0) shape 0 is a text box.
const CHART_SURFACE: usize = 1;

/// Every part that belongs to the chart and must survive an edit made elsewhere byte-for-byte.
const CHART_PARTS: &[&str] = &[
    "ppt/charts/chart1.xml",
    "ppt/charts/_rels/chart1.xml.rels",
    "ppt/embeddings/Microsoft_Excel_Sheet1.xlsx",
    "ppt/slides/slide2.xml",
    "ppt/slides/_rels/slide2.xml.rels",
];

#[test]
fn a_chart_frame_reads_as_a_chart() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    assert_eq!(
        pres.graphic_frame_kind(CHART_SURFACE, 0)
            .expect("read kind"),
        Some(GraphicFrameKind::Chart),
        "slide 2 shape 0 frames a chart"
    );
    // The text box on slide 1 is not a graphic frame at all.
    assert_eq!(
        pres.graphic_frame_kind(0, 0).expect("read kind"),
        None,
        "a text box is not a graphic frame"
    );
}

#[test]
fn chart_rel_id_names_the_chart_relationship() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    assert_eq!(
        pres.chart_rel_id(CHART_SURFACE, 0).expect("read"),
        Some("rId2".to_owned()),
        "the chart frame names its slide relationship"
    );
    // A shape that frames no chart answers None, not an error.
    assert_eq!(pres.chart_rel_id(0, 0).expect("read"), None);
}

#[test]
fn chart_part_bytes_resolves_to_the_verbatim_chart_part() {
    let bytes = fixture("charts.pptx");
    let baseline = Package::open(&bytes).expect("baseline");
    let chart_xml = baseline
        .part_bytes(&part("/ppt/charts/chart1.xml"))
        .expect("fixture has a chart part")
        .to_vec();

    let mut pres = Presentation::open(&bytes).expect("open");
    assert_eq!(
        pres.chart_part_bytes(CHART_SURFACE, 0).expect("read"),
        Some(chart_xml.as_slice()),
        "the resolved bytes are exactly the package's chart part"
    );
    // A non-chart shape answers None.
    assert_eq!(pres.chart_part_bytes(0, 0).expect("read"), None);
}

#[test]
fn reading_a_chart_leaves_every_part_byte_identical() {
    let bytes = fixture("charts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Exercise every read accessor; none may dirty a part.
    pres.graphic_frame_kind(CHART_SURFACE, 0).expect("kind");
    pres.chart_rel_id(CHART_SURFACE, 0).expect("rel id");
    pres.chart_part_bytes(CHART_SURFACE, 0).expect("bytes");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_eq!(reopened, original, "reading a chart must dirty nothing");
}

#[test]
fn editing_another_slide_leaves_the_chart_parts_byte_identical() {
    let bytes = fixture("charts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    // Edit the text box on slide 1 — a part with nothing to do with the chart on slide 2.
    pres.set_shape_text(0, 0, 0, "Edited").expect("set text");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    for &name in CHART_PARTS {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "chart part {name} must be untouched by an edit elsewhere"
        );
    }
    assert_ne!(
        reopened.get("ppt/slides/slide1.xml"),
        original.get("ppt/slides/slide1.xml"),
        "the edited slide should have changed"
    );
}

// -------------------------------------------------------------------------------------------------
// C3 — editing series data
// -------------------------------------------------------------------------------------------------

#[test]
fn chart_series_reads_the_series() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    let series = pres.chart_series(CHART_SURFACE, 0).expect("read series");

    assert_eq!(
        series,
        vec![ChartSeriesData {
            name: Some("Sales".to_owned()),
            categories: vec!["North".to_owned(), "South".to_owned(), "West".to_owned()],
            values: vec![19.2, 21.4, 16.7],
        }]
    );

    // A shape that frames no chart is an error, not empty data.
    assert!(matches!(
        pres.chart_series(0, 0),
        Err(PptxError::ShapeIsNotAChart)
    ));
}

#[test]
fn set_chart_series_values_rewrites_and_persists() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    pres.set_chart_series_values(CHART_SURFACE, 0, 0, &[1.0, 2.5, 3.0])
        .expect("set values");

    // Visible immediately through the read surface...
    assert_eq!(
        pres.chart_series(CHART_SURFACE, 0).expect("read")[0].values,
        vec![1.0, 2.5, 3.0]
    );

    // ...and after a save + reopen round-trip.
    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.chart_series(CHART_SURFACE, 0).expect("read")[0].values,
        vec![1.0, 2.5, 3.0]
    );
}

#[test]
fn set_chart_series_categories_rewrites() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    pres.set_chart_series_categories(CHART_SURFACE, 0, 0, &["Q1", "Q2", "Q3"])
        .expect("set categories");
    assert_eq!(
        pres.chart_series(CHART_SURFACE, 0).expect("read")[0].categories,
        vec!["Q1".to_owned(), "Q2".to_owned(), "Q3".to_owned()]
    );
}

#[test]
fn editing_a_chart_dirties_only_the_chart_xml_and_its_workbook() {
    let bytes = fixture("charts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_chart_series_values(CHART_SURFACE, 0, 0, &[1.0, 2.0, 3.0])
        .expect("set values");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    // The chart XML changed...
    assert_ne!(
        reopened.get("ppt/charts/chart1.xml"),
        original.get("ppt/charts/chart1.xml"),
        "the edited chart part must have changed"
    );
    // ...and so did the workbook behind it, which is the point: the numbers PowerPoint's Edit Data
    // opens are the numbers the chart draws.
    assert_ne!(
        reopened.get("ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"),
        original.get("ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"),
        "the embedded workbook must be refreshed, not left stale"
    );
    // Every other part is byte-identical — including the chart's own relationships, which still name
    // the same workbook part.
    for name in [
        "ppt/charts/_rels/chart1.xml.rels",
        "ppt/slides/slide2.xml",
        "ppt/slides/_rels/slide2.xml.rels",
        "ppt/slides/slide1.xml",
        "[Content_Types].xml",
    ] {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "editing chart data must leave {name} byte-identical"
        );
    }
    // Nothing was added or removed from the package.
    let before: Vec<&String> = original.keys().collect();
    let after: Vec<&String> = reopened.keys().collect();
    assert_eq!(before, after, "editing chart data must add no parts");
}

#[test]
fn the_refreshed_workbook_holds_the_edited_values() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    pres.set_chart_series_values(CHART_SURFACE, 0, 0, &[41.5, 42.5, 43.5])
        .expect("set values");
    pres.set_chart_series_categories(CHART_SURFACE, 0, 0, &["Alpha", "Beta", "Gamma"])
        .expect("set categories");

    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    let sheet = workbook_sheet(
        pkg.part_bytes(&part("/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"))
            .expect("the workbook part survives"),
    );
    for value in ["41.5", "42.5", "43.5"] {
        assert!(
            sheet.contains(&format!("<v>{value}</v>")),
            "the refreshed sheet should hold {value}: {sheet}"
        );
    }
    // The old cached numbers are gone from the sheet entirely.
    for stale in ["19.2", "21.4", "16.7"] {
        assert!(
            !sheet.contains(&format!("<v>{stale}</v>")),
            "the stale value {stale} must not survive the refresh: {sheet}"
        );
    }
    let strings = workbook_part(
        pkg.part_bytes(&part("/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"))
            .expect("the workbook part survives"),
        "xl/sharedStrings.xml",
    );
    for label in ["Alpha", "Beta", "Gamma", "Sales"] {
        assert!(
            strings.contains(&format!("<t>{label}</t>")),
            "the refreshed shared strings should hold {label}: {strings}"
        );
    }
}

#[test]
fn refreshing_a_chart_with_no_workbook_changes_nothing() {
    // A chart whose workbook has been detached names none, so there is nothing to refresh — and the
    // absence is not an error.
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    pres.detach_chart_workbook(CHART_SURFACE, 0)
        .expect("detach");
    assert!(
        !pres
            .refresh_chart_workbook(CHART_SURFACE, 0)
            .expect("refresh"),
        "a chart with no c:externalData has no workbook to refresh"
    );
    // And a data edit on such a chart still succeeds.
    pres.set_chart_series_values(CHART_SURFACE, 0, 0, &[1.0, 2.0, 3.0])
        .expect("edit a workbook-less chart");
}

#[test]
fn editing_a_non_chart_or_out_of_range_series_errors() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");

    // The text box on slide 1 is not a chart.
    assert!(matches!(
        pres.set_chart_series_values(0, 0, 0, &[1.0]),
        Err(PptxError::ShapeIsNotAChart)
    ));

    // The chart has one series; index 5 is out of range.
    assert!(matches!(
        pres.set_chart_series_values(CHART_SURFACE, 0, 5, &[1.0]),
        Err(PptxError::ChartSeriesOutOfRange { index: 5, count: 1 })
    ));
}

// -------------------------------------------------------------------------------------------------
// C4 — authoring a brand-new chart (cached data only)
//
// These build a chart on the single-slide `sample.pptx` (no chart of its own), so the new chart part
// is `chart1.xml` and the touched slide is `slide1.xml`.
// -------------------------------------------------------------------------------------------------

fn bounds() -> ShapeBounds {
    ShapeBounds::from_inches(1.0, 1.0, 6.0, 4.0)
}

fn bar_chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [10.0, 20.5, 15.0])
        .series("Cost", [5.0, 8.0, 7.25])
}

#[test]
fn add_chart_authors_a_readable_chart() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");

    // It is a chart-framing graphic frame on the slide.
    assert_eq!(
        pres.graphic_frame_kind(0, idx).expect("kind"),
        Some(GraphicFrameKind::Chart)
    );
    assert!(pres.chart_rel_id(0, idx).expect("rel id").is_some());

    // Its series read back exactly as authored.
    assert_eq!(
        pres.chart_series(0, idx).expect("series"),
        vec![
            ChartSeriesData {
                name: Some("Revenue".to_owned()),
                categories: vec!["Q1".to_owned(), "Q2".to_owned(), "Q3".to_owned()],
                values: vec![10.0, 20.5, 15.0],
            },
            ChartSeriesData {
                name: Some("Cost".to_owned()),
                categories: vec!["Q1".to_owned(), "Q2".to_owned(), "Q3".to_owned()],
                values: vec![5.0, 8.0, 7.25],
            },
        ]
    );
}

#[test]
fn added_chart_survives_save_and_reopen() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    let saved = pres.save().expect("save");

    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.graphic_frame_kind(0, idx).expect("kind"),
        Some(GraphicFrameKind::Chart)
    );
    assert_eq!(
        reopened.chart_series(0, idx).expect("series")[0].values,
        vec![10.0, 20.5, 15.0]
    );
}

#[test]
fn add_chart_creates_the_chart_part_with_its_content_type_and_relationship() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");

    // The chart part exists...
    assert!(
        pkg.part_bytes(&part("/ppt/charts/chart1.xml")).is_some(),
        "the chart part was written"
    );
    // ...registered as a per-part Override in [Content_Types].xml...
    let content_types = String::from_utf8(
        pkg.part_bytes(&part("/[Content_Types].xml"))
            .expect("content types")
            .to_vec(),
    )
    .expect("utf-8");
    assert!(
        content_types.contains("/ppt/charts/chart1.xml")
            && content_types.contains("drawingml.chart+xml"),
        "the chart part has a content-type Override"
    );
    // ...and named by a chart relationship from the slide.
    let rels = String::from_utf8(
        pkg.part_bytes(&part("/ppt/slides/_rels/slide1.xml.rels"))
            .expect("slide rels")
            .to_vec(),
    )
    .expect("utf-8");
    assert!(
        rels.contains("relationships/chart") && rels.contains("../charts/chart1.xml"),
        "the slide relates to the chart part"
    );
}

#[test]
fn adding_a_chart_leaves_pre_existing_parts_untouched() {
    let bytes = fixture("sample.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    // A brand-new part.
    assert!(!original.contains_key("ppt/charts/chart1.xml"));
    assert!(reopened.contains_key("ppt/charts/chart1.xml"));

    // Only the wiring parts change; every other pre-existing part is byte-identical.
    let touched = [
        "ppt/slides/slide1.xml",
        "ppt/slides/_rels/slide1.xml.rels",
        "[Content_Types].xml",
    ];
    for (name, bytes) in &original {
        if touched.contains(&name.as_str()) {
            continue;
        }
        assert_eq!(
            reopened.get(name),
            Some(bytes),
            "adding a chart must leave {name} byte-identical"
        );
    }
}

#[test]
fn every_chart_kind_authors_a_frame_of_that_kind() {
    for kind in [
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Area,
        ChartKind::Pie,
        ChartKind::Doughnut,
        ChartKind::Scatter,
    ] {
        let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
        let chart = ChartData::new(kind)
            .categories(["A", "B"])
            .series("S", [1.0, 2.0]);
        let idx = pres.add_chart(0, &chart, bounds()).expect("add chart");

        // Round-trips through a save, and reads back as a chart with its one series' values.
        let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
        assert_eq!(
            reopened.graphic_frame_kind(0, idx).expect("kind"),
            Some(GraphicFrameKind::Chart),
            "kind {kind:?} frames a chart"
        );
        assert_eq!(
            reopened.chart_series(0, idx).expect("series")[0].values,
            vec![1.0, 2.0],
            "kind {kind:?} values survive"
        );
    }
}

#[test]
fn add_chart_rejects_empty_data() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let empty = ChartData::new(ChartKind::Bar)
        .categories(["A"])
        .series("S", []);
    assert!(matches!(
        pres.add_chart(0, &empty, bounds()),
        Err(PptxError::InvalidChartData)
    ));
    // Nothing was written — no chart part leaked.
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert!(pkg.part_bytes(&part("/ppt/charts/chart1.xml")).is_none());
}

// -------------------------------------------------------------------------------------------------
// MJX-201 P2 — detaching an inaccessible chart backing workbook
// -------------------------------------------------------------------------------------------------

#[test]
fn chart_workbooks_reports_the_embedded_workbook() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");

    assert_eq!(
        pres.chart_workbooks(CHART_SURFACE).expect("workbooks"),
        vec![ChartWorkbook {
            shape_index: 0,
            target: "../embeddings/Microsoft_Excel_Sheet1.xlsx".to_owned(),
            external: false,
        }],
        "the chart's embedded workbook is reported, not flagged external"
    );
    // The text-box slide has no charts.
    assert!(pres.chart_workbooks(0).expect("workbooks").is_empty());
}

#[test]
fn chart_workbooks_flags_an_external_workbook() {
    // Flip the fixture's embedded workbook relationship to an external link with the P1 primitive.
    let mut pkg = Package::open(&fixture("charts.pptx")).expect("open");
    let chart = part("/ppt/charts/chart1.xml");
    assert!(
        pkg.retarget_relationship(
            Some(&chart),
            "rId1",
            "https://example.com/data.xlsx",
            TargetMode::External,
        )
        .expect("retarget"),
        "the workbook relationship exists"
    );
    let mut pres = Presentation::open(&pkg.save().expect("save")).expect("reopen");

    assert_eq!(
        pres.chart_workbooks(CHART_SURFACE).expect("workbooks"),
        vec![ChartWorkbook {
            shape_index: 0,
            target: "https://example.com/data.xlsx".to_owned(),
            external: true,
        }],
        "an externally linked workbook is flagged"
    );
}

#[test]
fn detach_chart_workbook_removes_the_reference_and_keeps_the_chart() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");

    pres.detach_chart_workbook(CHART_SURFACE, 0)
        .expect("detach the workbook");
    assert!(
        pres.chart_workbooks(CHART_SURFACE)
            .expect("workbooks")
            .is_empty(),
        "the chart no longer references a workbook"
    );

    // Reopen: the chart still frames a chart, and its part carries no c:externalData any more.
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened.graphic_frame_kind(CHART_SURFACE, 0).expect("kind"),
        Some(GraphicFrameKind::Chart),
        "the chart survives the detach"
    );
    {
        let bytes = reopened
            .chart_part_bytes(CHART_SURFACE, 0)
            .expect("bytes")
            .expect("chart part present");
        assert!(
            !bytes
                .windows(b"externalData".len())
                .any(|w| w == b"externalData"),
            "the c:externalData element must be gone from the chart part"
        );
    }

    // The embedded workbook is now unreferenced and sweepable.
    let mut pkg = Package::open(&reopened.save().expect("save")).expect("reopen pkg");
    let removed = pkg.remove_unreferenced_parts().expect("sweep");
    assert!(
        removed
            .iter()
            .any(|p| p.as_str() == "/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"),
        "the detached workbook should be swept: {removed:?}"
    );
}

#[test]
fn detach_chart_workbook_rejects_the_wrong_shapes() {
    let mut pres = Presentation::open(&fixture("charts.pptx")).expect("open");
    // The text box on slide 1 frames no chart.
    assert!(matches!(
        pres.detach_chart_workbook(0, 0),
        Err(PptxError::ShapeIsNotAChart)
    ));

    // An authored chart now embeds a workbook of its own, so detaching it is meaningful and
    // succeeds; only a chart that has already been detached has nothing left to detach.
    let mut authored = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = authored
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    authored
        .detach_chart_workbook(0, idx)
        .expect("an authored chart embeds a workbook");
    assert!(matches!(
        authored.detach_chart_workbook(0, idx),
        Err(PptxError::ChartHasNoExternalData)
    ));
}

// -------------------------------------------------------------------------------------------------
// A5 / MJX-116 — the embedded workbook, the remaining plot types, and the typed axis/legend/styling
// surface, through the `Presentation` façade.
// -------------------------------------------------------------------------------------------------

#[test]
fn add_chart_writes_the_embedded_workbook_and_wires_it_to_the_chart() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");

    // The workbook part exists, named the way Office names one...
    let workbook = pkg
        .part_bytes(&part("/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"))
        .expect("the embedded workbook was written");
    assert_eq!(&workbook[..2], b"PK", "and it is a real package");

    // ...registered with the spreadsheet content type...
    let content_types = String::from_utf8(
        pkg.part_bytes(&part("/[Content_Types].xml"))
            .expect("content types")
            .to_vec(),
    )
    .expect("utf-8");
    assert!(
        content_types.contains("Microsoft_Excel_Sheet1.xlsx")
            && content_types.contains("spreadsheetml.sheet"),
        "the workbook has a content-type Override: {content_types}"
    );

    // ...related from the *chart* part, not the slide...
    let rels = String::from_utf8(
        pkg.part_bytes(&part("/ppt/charts/_rels/chart1.xml.rels"))
            .expect("the chart has relationships")
            .to_vec(),
    )
    .expect("utf-8");
    assert!(
        rels.contains("relationships/package")
            && rels.contains("../embeddings/Microsoft_Excel_Sheet1.xlsx"),
        "the chart relates to its workbook: {rels}"
    );

    // ...and named by the chart's own c:externalData.
    let chart = String::from_utf8(
        pkg.part_bytes(&part("/ppt/charts/chart1.xml"))
            .expect("chart part")
            .to_vec(),
    )
    .expect("utf-8");
    assert!(
        chart.contains(r#"<c:externalData r:id="rId1"><c:autoUpdate val="0"/></c:externalData>"#),
        "the chart names its workbook: {chart}"
    );

    // The workbook holds the numbers the chart draws.
    let sheet = workbook_sheet(workbook);
    for (cell, value) in [("B2", "10"), ("B3", "20.5"), ("B4", "15"), ("C2", "5")] {
        assert!(
            sheet.contains(&format!(r#"<c r="{cell}"><v>{value}</v></c>"#)),
            "cell {cell} should hold {value}: {sheet}"
        );
    }
}

#[test]
fn a_second_authored_chart_gets_its_own_workbook() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    pres.add_chart(0, &bar_chart(), bounds()).expect("first");
    pres.add_chart(0, &bar_chart(), bounds()).expect("second");
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");

    for name in [
        "/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx",
        "/ppt/embeddings/Microsoft_Excel_Sheet2.xlsx",
    ] {
        assert!(
            pkg.part_bytes(&part(name)).is_some(),
            "{name} should have been written"
        );
    }
    // And the second chart names the second workbook, not the first.
    let rels = String::from_utf8(
        pkg.part_bytes(&part("/ppt/charts/_rels/chart2.xml.rels"))
            .expect("the second chart has relationships")
            .to_vec(),
    )
    .expect("utf-8");
    assert!(rels.contains("Microsoft_Excel_Sheet2.xlsx"), "{rels}");
}

#[test]
fn adding_a_chart_beside_an_existing_workbook_does_not_collide_with_it() {
    // `charts.pptx` already holds `Microsoft_Excel_Sheet1.xlsx`; a chart added to it must not
    // overwrite that part or reuse its name.
    let bytes = fixture("charts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));
    let mut pres = Presentation::open(&bytes).expect("open");
    pres.add_chart(0, &bar_chart(), bounds())
        .expect("add a chart to the text-box slide");
    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_eq!(
        reopened.get("ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"),
        original.get("ppt/embeddings/Microsoft_Excel_Sheet1.xlsx"),
        "the pre-existing workbook is untouched"
    );
    assert!(
        reopened.contains_key("ppt/embeddings/Microsoft_Excel_Sheet2.xlsx"),
        "the new chart got a workbook of its own: {:?}",
        reopened.keys().collect::<Vec<_>>()
    );
}

#[test]
fn every_plot_type_the_crate_names_reads_its_series() {
    // Part 2 of MJX-116, through the façade: a chart of each kind is authored, saved, reopened, and
    // its series read back. Before this tier the ten new kinds read as no series at all.
    let kinds = [
        ChartKind::Bar,
        ChartKind::Bar3D,
        ChartKind::Line,
        ChartKind::Line3D,
        ChartKind::Pie,
        ChartKind::Pie3D,
        ChartKind::OfPie,
        ChartKind::Area,
        ChartKind::Area3D,
        ChartKind::Scatter,
        ChartKind::Doughnut,
        ChartKind::Radar,
        ChartKind::Bubble,
        ChartKind::Surface,
        ChartKind::Surface3D,
    ];
    for kind in kinds {
        let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
        let chart = ChartData::new(kind)
            .categories(["A", "B"])
            .series("S", [1.0, 2.0]);
        let idx = pres.add_chart(0, &chart, bounds()).expect("add chart");
        let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");

        assert_eq!(
            reopened.chart_kinds(0, idx).expect("kinds"),
            vec![kind],
            "kind {kind:?} reads back as itself"
        );
        let series = reopened.chart_series(0, idx).expect("series");
        assert_eq!(series.len(), 1, "kind {kind:?} exposes its one series");
        assert_eq!(series[0].name.as_deref(), Some("S"), "kind {kind:?} name");
        assert_eq!(series[0].values, vec![1.0, 2.0], "kind {kind:?} values");
    }
}

#[test]
fn a_stock_chart_needs_three_series_and_says_so() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let too_few = ChartData::new(ChartKind::Stock)
        .categories(["Mon", "Tue"])
        .series("Close", [1.0, 2.0]);
    let error = pres
        .add_chart(0, &too_few, bounds())
        .expect_err("a one-series stock chart is not schema-valid");
    assert!(
        matches!(error, PptxError::ChartData(_)),
        "expected a chart-data problem, got {error:?}"
    );
    assert!(
        error.to_string().contains("stockChart"),
        "the message names the plot type: {error}"
    );
    // Nothing was written.
    let pkg = Package::open(&pres.save().expect("save")).expect("reopen");
    assert!(pkg.part_bytes(&part("/ppt/charts/chart1.xml")).is_none());

    // Three series is enough.
    let ok = ChartData::new(ChartKind::Stock)
        .categories(["Mon", "Tue"])
        .series("High", [3.0, 4.0])
        .series("Low", [1.0, 2.0])
        .series("Close", [2.0, 3.0]);
    let idx = pres.add_chart(0, &ok, bounds()).expect("a stock chart");
    assert_eq!(pres.chart_series(0, idx).expect("series").len(), 3);
}

#[test]
fn chart_axes_read_an_authored_chart() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    let axes = pres.chart_axes(0, idx).expect("axes");

    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].kind, AxisKind::Category);
    assert_eq!(axes[0].position, Some(AxisPosition::Bottom));
    assert_eq!(axes[0].suppressed, Some(false));
    assert_eq!(axes[1].kind, AxisKind::Value);
    assert_eq!(axes[1].position, Some(AxisPosition::Left));

    // The axis ids we author are unsigned — never the negative ones python-pptx writes.
    let category_id = axes[0].axis_id.expect("an unsigned category axis id");
    let value_id = axes[1].axis_id.expect("an unsigned value axis id");
    assert_eq!(axes[0].cross_axis_id, Some(value_id));
    assert_eq!(axes[1].cross_axis_id, Some(category_id));

    // The fixture's chart, in contrast, carries ids no unsigned integer can hold — and we read them
    // as absent rather than coercing them.
    let mut producer = Presentation::open(&fixture("charts.pptx")).expect("open");
    let axes = producer.chart_axes(CHART_SURFACE, 0).expect("axes");
    assert_eq!(axes.len(), 2);
    assert_eq!(axes[0].axis_id, None);
    assert!(axes[1].major_gridlines, "the fixture rules gridlines");
}

#[test]
fn the_axis_legend_title_and_styling_surfaces_round_trip() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");

    pres.set_chart_title(0, idx, Some("Quarterly results"))
        .expect("title");
    pres.set_chart_legend(0, idx, Some(LegendPosition::Bottom))
        .expect("legend");
    pres.set_chart_axis_title(0, idx, 1, Some("Millions"))
        .expect("axis title");
    pres.set_chart_axis_scale(0, idx, 1, Some(0.0), Some(25.0))
        .expect("axis scale");
    pres.set_chart_axis_orientation(0, idx, 1, AxisOrientation::MaximumToMinimum)
        .expect("axis orientation");
    pres.set_chart_axis_gridlines(0, idx, 1, true, false)
        .expect("gridlines");
    pres.set_chart_series_fill(
        0,
        idx,
        0,
        &FillSpec::Solid(ColorSpec::Srgb("4472C4".to_owned())),
    )
    .expect("series fill");
    pres.set_chart_series_line(
        0,
        idx,
        1,
        &LineSpec::solid(
            LineWidth::from_points(1.5),
            ColorSpec::Srgb("ED7D31".to_owned()),
        ),
    )
    .expect("series line");

    // Everything survives a save and reopen.
    let mut reopened = Presentation::open(&pres.save().expect("save")).expect("reopen");
    assert_eq!(
        reopened.chart_title(0, idx).expect("title").as_deref(),
        Some("Quarterly results")
    );
    let legend = reopened
        .chart_legend(0, idx)
        .expect("legend")
        .expect("a legend was placed");
    assert_eq!(legend.position, Some(LegendPosition::Bottom));
    assert_eq!(legend.overlays_plot, Some(false));

    let axes = reopened.chart_axes(0, idx).expect("axes");
    assert_eq!(axes[1].title.as_deref(), Some("Millions"));
    assert_eq!(axes[1].minimum, Some(0.0));
    assert_eq!(axes[1].maximum, Some(25.0));
    assert_eq!(axes[1].orientation, Some(AxisOrientation::MaximumToMinimum));
    assert!(axes[1].major_gridlines);
    assert!(!axes[1].minor_gridlines);

    assert_eq!(
        reopened.chart_series_fill(0, idx, 0).expect("fill"),
        Some(FillSpec::Solid(ColorSpec::Srgb("4472C4".to_owned())))
    );
    assert_eq!(
        reopened.chart_series_fill(0, idx, 1).expect("fill"),
        None,
        "the second series was given an outline, not a fill"
    );

    // Removing the title and the legend takes both elements away.
    reopened.set_chart_title(0, idx, None).expect("clear title");
    reopened
        .set_chart_legend(0, idx, None)
        .expect("clear legend");
    assert_eq!(reopened.chart_title(0, idx).expect("title"), None);
    assert_eq!(reopened.chart_legend(0, idx).expect("legend"), None);
}

#[test]
fn the_typed_chart_surfaces_reject_the_wrong_shapes_and_indices() {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");

    // The text box on the same slide frames no chart.
    assert!(matches!(
        pres.chart_axes(0, 0),
        Err(PptxError::ShapeIsNotAChart)
    ));
    // An axis past the last is an error, not a silent no-op.
    assert!(matches!(
        pres.set_chart_axis_title(0, idx, 9, Some("x")),
        Err(PptxError::ChartAxisOutOfRange { index: 9, count: 2 })
    ));
    // So is a series past the last.
    assert!(matches!(
        pres.set_chart_series_fill(0, idx, 9, &FillSpec::None),
        Err(PptxError::ChartSeriesOutOfRange { index: 9, count: 2 })
    ));
    // An image fill would name a relationship a chart part does not have.
    assert!(matches!(
        pres.set_chart_series_fill(
            0,
            idx,
            0,
            &FillSpec::Picture {
                rel_id: "rId9".to_owned(),
                mode: mjx_dml::PictureFillMode::Stretch,
            }
        ),
        Err(PptxError::ChartFillNotSupported)
    ));
}

#[test]
fn styling_a_chart_dirties_only_the_chart_part() {
    // Tier 3: the axis, legend, title and fill setters touch the chart part and nothing else — in
    // particular they do not disturb the embedded workbook, which holds the data, not the styling.
    let bytes = fixture("sample.pptx");
    let mut pres = Presentation::open(&bytes).expect("open");
    let idx = pres
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    let baseline = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    let mut pres = Presentation::open(&pres.save().expect("save")).expect("reopen");
    pres.set_chart_title(0, idx, Some("Styled")).expect("title");
    pres.set_chart_axis_gridlines(0, idx, 1, true, true)
        .expect("gridlines");
    pres.set_chart_series_fill(0, idx, 0, &FillSpec::None)
        .expect("fill");
    let after = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));

    assert_ne!(
        after.get("ppt/charts/chart1.xml"),
        baseline.get("ppt/charts/chart1.xml"),
        "the chart part must have changed"
    );
    for (name, bytes) in &baseline {
        if name == "ppt/charts/chart1.xml" {
            continue;
        }
        assert_eq!(
            after.get(name),
            Some(bytes),
            "styling a chart must leave {name} byte-identical"
        );
    }
}

#[test]
fn chart_style_id_reads_what_the_part_declares() {
    // An authored chart declares no `c:style`, so it inherits — and that reads as `None`, not as a
    // guessed default.
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = pres
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    assert_eq!(pres.chart_style_id(0, idx).expect("style"), None);

    // A chart that *does* name one reads it. `c:style` is the first thing after `c:roundedCorners`
    // in `CT_ChartSpace`, so splicing it in ahead of `c:chart` gives a part shaped like Office's.
    let mut pkg = Package::open(&pres.save().expect("save")).expect("reopen package");
    let chart_part = part("/ppt/charts/chart1.xml");
    let styled = String::from_utf8(pkg.part_bytes(&chart_part).expect("chart").to_vec())
        .expect("utf-8")
        .replacen("<c:chart>", r#"<c:style val="34"/><c:chart>"#, 1);
    pkg.replace_part_bytes(&chart_part, styled.into_bytes())
        .expect("splice in a chart style");

    let mut styled = Presentation::open(&pkg.save().expect("save")).expect("reopen");
    assert_eq!(
        styled.chart_style_id(0, idx).expect("style"),
        Some(34),
        "the built-in style the part names"
    );
}

// -------------------------------------------------------------------------------------------------
// Chart decoration — data labels, per-point formatting, trendlines and error bars (MJX-116, part 5)
// -------------------------------------------------------------------------------------------------

/// The 16:9 slide size every decoration deck below is built at.
fn widescreen() -> mjx_pptx::SlideSize {
    mjx_pptx::SlideSize {
        width_emu: 12_192_000,
        height_emu: 6_858_000,
        kind: mjx_ooxml_types::presentationml::SlideSizeKind::Screen16X9,
    }
}

/// A three-series bar chart, the base every decoration case below builds on.
fn labelled_chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("Revenue", [10.0, 20.0, 15.0])
        .series("Cost", [5.0, 8.0, 7.0])
}

/// A blank deck with one slide — nothing in it came from a file, so every byte the decoration cases
/// assert on is this project's own.
fn blank_deck_with_a_slide() -> Presentation {
    let mut pres = Presentation::blank(widescreen()).expect("blank deck");
    pres.add_slide_from_layout(0).expect("add a slide");
    pres
}

/// Adds `chart` to a blank deck's first slide and returns the presentation and the frame's index.
fn deck_with(chart: &ChartData) -> (Presentation, usize) {
    let mut pres = blank_deck_with_a_slide();
    let frame = pres
        .add_chart(0, chart, ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0))
        .expect("add chart");
    (pres, frame)
}

#[test]
fn an_authored_chart_can_label_itself_and_read_the_labels_back() {
    let (mut pres, frame) = deck_with(
        &labelled_chart().data_labels(
            DataLabelSpec::new()
                .value(true)
                .position(DataLabelPosition::OutsideEnd)
                .number_format("#,##0")
                .separator("; "),
        ),
    );
    // The plot tier is what every series takes.
    for series_idx in [0, 1] {
        let settings = pres
            .chart_data_labels(0, frame, series_idx, None)
            .expect("read labels");
        assert_eq!(settings.shows_value, Some(true));
        assert_eq!(settings.position, Some(DataLabelPosition::OutsideEnd));
        assert_eq!(settings.number_format.as_deref(), Some("#,##0"));
        assert_eq!(settings.separator.as_deref(), Some("; "));
    }
    // …and it survives a save and reopen.
    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened
            .chart_data_labels(0, frame, 0, None)
            .expect("read labels")
            .position,
        Some(DataLabelPosition::OutsideEnd)
    );
}

#[test]
fn the_three_label_tiers_are_addressed_and_merged_separately() {
    let (mut pres, frame) = deck_with(
        &labelled_chart().data_labels(
            DataLabelSpec::new()
                .value(true)
                .category_name(false)
                .position(DataLabelPosition::Center)
                .number_format("0.0"),
        ),
    );
    // Series 0 disagrees with the plot on two settings and is silent about the rest.
    pres.set_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Series { series_idx: 0 },
        &DataLabelSpec::new()
            .category_name(true)
            .position(DataLabelPosition::OutsideEnd),
    )
    .expect("set series labels");
    // Point 1 of series 0 disagrees with both.
    pres.set_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Point {
            series_idx: 0,
            point_idx: 1,
        },
        &DataLabelSpec::new()
            .value(false)
            .position(DataLabelPosition::InsideEnd),
    )
    .expect("set point label");

    let saved = pres.save().expect("save");
    assert_authored_deck_is_readable(&saved);
    let mut pres = Presentation::open(&saved).expect("reopen");

    // Each tier states only what it was given.
    let plot = pres
        .chart_data_label_tier(0, frame, ChartLabelScope::Plot { plot_idx: 0 })
        .expect("read the plot tier")
        .expect("the plot states settings");
    assert_eq!(plot.position, Some(DataLabelPosition::Center));
    let series = pres
        .chart_data_label_tier(0, frame, ChartLabelScope::Series { series_idx: 0 })
        .expect("read the series tier")
        .expect("series 0 states settings");
    assert_eq!(series.position, Some(DataLabelPosition::OutsideEnd));
    assert_eq!(
        series.shows_value, None,
        "the series says nothing about the value"
    );
    assert!(
        pres.chart_data_label_tier(0, frame, ChartLabelScope::Series { series_idx: 1 })
            .expect("read the series tier")
            .is_none(),
        "series 1 carries no c:dLbls at all"
    );

    // …and the merge is per setting, with every tier disagreeing with the one below it.
    let resolved = pres
        .chart_data_labels(0, frame, 0, None)
        .expect("resolve series 0");
    assert_eq!(resolved.position, Some(DataLabelPosition::OutsideEnd));
    assert_eq!(
        resolved.shows_category_name,
        Some(true),
        "the series says 1 where the plot says 0"
    );
    assert_eq!(
        resolved.shows_value,
        Some(true),
        "the series is silent, so the plot's 1 falls through"
    );
    assert_eq!(resolved.number_format.as_deref(), Some("0.0"));

    let point = pres
        .chart_data_labels(0, frame, 0, Some(1))
        .expect("resolve point 1");
    assert_eq!(point.position, Some(DataLabelPosition::InsideEnd));
    assert_eq!(point.shows_value, Some(false));
    assert_eq!(
        point.shows_category_name,
        Some(true),
        "the point is silent, so the SERIES' value must survive — not the plot's"
    );
    assert_eq!(point.number_format.as_deref(), Some("0.0"));

    // A point with no override of its own, and a series with none, both fall back correctly.
    assert_eq!(
        pres.chart_data_labels(0, frame, 0, Some(0))
            .expect("point 0"),
        resolved
    );
    assert_eq!(
        pres.chart_data_labels(0, frame, 1, None)
            .expect("series 1")
            .position,
        Some(DataLabelPosition::Center)
    );
}

#[test]
fn one_point_can_carry_its_own_words_and_be_silenced() {
    let (mut pres, frame) =
        deck_with(&labelled_chart().data_labels(DataLabelSpec::new().value(true)));
    pres.set_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Point {
            series_idx: 0,
            point_idx: 2,
        },
        &DataLabelSpec::new().category_name(true),
    )
    .expect("set point label");
    pres.suppress_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Point {
            series_idx: 0,
            point_idx: 1,
        },
    )
    .expect("silence point 1");

    let saved = pres.save().expect("save");
    let mut pres = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        pres.chart_data_labels(0, frame, 0, Some(1))
            .expect("point 1")
            .suppressed,
        Some(true)
    );
    assert_eq!(
        pres.chart_data_labels(0, frame, 0, Some(2))
            .expect("point 2")
            .shows_category_name,
        Some(true)
    );
    assert_eq!(
        pres.chart_data_labels(0, frame, 0, Some(0))
            .expect("point 0")
            .suppressed,
        None,
        "silencing one point leaves its neighbours alone"
    );
    // Removing the override returns the point to its series' settings.
    assert!(pres
        .remove_chart_data_labels(
            0,
            frame,
            ChartLabelScope::Point {
                series_idx: 0,
                point_idx: 1
            }
        )
        .expect("remove"));
    assert_eq!(
        pres.chart_data_labels(0, frame, 0, Some(1))
            .expect("point 1")
            .suppressed,
        None
    );
}

#[test]
fn a_point_can_be_coloured_differently_from_its_series() {
    let (mut pres, frame) = deck_with(&labelled_chart());
    let red = FillSpec::Solid(ColorSpec::Srgb("FF0000".into()));
    pres.set_chart_point_fill(0, frame, 0, 1, &red)
        .expect("colour point 1");
    pres.set_chart_point_line(
        0,
        frame,
        0,
        1,
        &LineSpec {
            width: Some(LineWidth::from_points(1.5)),
            ..LineSpec::default()
        },
    )
    .expect("outline point 1");
    pres.set_chart_point_explosion(0, frame, 0, 2, Some(30))
        .expect("explode point 2");

    let saved = pres.save().expect("save");
    let mut pres = Presentation::open(&saved).expect("reopen");
    let formats = pres.chart_point_formats(0, frame, 0).expect("read formats");
    assert_eq!(formats.len(), 2);
    // The two `c:dPt` are written in ascending `c:idx`, and each names the point it formats.
    assert_eq!(formats[0].index, Some(1));
    assert_eq!(formats[0].fill, Some(red));
    assert!(formats[0].line.is_some());
    assert_eq!(formats[1].index, Some(2));
    assert_eq!(formats[1].explosion, Some(30));
    // The other series is untouched.
    assert!(pres
        .chart_point_formats(0, frame, 1)
        .expect("read formats")
        .is_empty());

    assert!(pres
        .remove_chart_point_format(0, frame, 0, 1)
        .expect("remove"));
    assert!(!pres
        .remove_chart_point_format(0, frame, 0, 1)
        .expect("remove"));
    assert_eq!(
        pres.chart_point_formats(0, frame, 0).expect("read").len(),
        1
    );
}

#[test]
fn a_series_length_edit_never_re_points_its_per_point_formatting() {
    let (mut pres, frame) = deck_with(&labelled_chart());
    pres.set_chart_point_fill(
        0,
        frame,
        0,
        2,
        &FillSpec::Solid(ColorSpec::Srgb("00FF00".into())),
    )
    .expect("colour point 2");
    pres.set_chart_data_labels(
        0,
        frame,
        ChartLabelScope::Point {
            series_idx: 0,
            point_idx: 2,
        },
        &DataLabelSpec::new().value(true),
    )
    .expect("label point 2");

    // Shorten the series from three points to two. The decoration anchored to point 2 now addresses
    // past the end — and must still say "point 2", not "point 1".
    pres.set_chart_series_values(0, frame, 0, &[1.0, 2.0])
        .expect("shorten the series");

    let saved = pres.save().expect("save");
    let mut pres = Presentation::open(&saved).expect("reopen");
    let formats = pres.chart_point_formats(0, frame, 0).expect("read formats");
    assert_eq!(formats.len(), 1);
    assert_eq!(
        formats[0].index,
        Some(2),
        "the c:dPt was renumbered onto a different point"
    );
    assert_eq!(
        pres.chart_dangling_decoration(0, frame, 0)
            .expect("read dangling"),
        vec![
            DanglingPointReference {
                element: "dPt",
                index: 2
            },
            DanglingPointReference {
                element: "dLbl",
                index: 2
            },
        ]
    );
    // Nothing is dropped until a caller asks.
    assert_eq!(
        pres.drop_chart_dangling_decoration(0, frame, 0)
            .expect("drop"),
        2
    );
    assert!(pres
        .chart_dangling_decoration(0, frame, 0)
        .expect("read dangling")
        .is_empty());
    assert!(pres
        .chart_point_formats(0, frame, 0)
        .expect("read formats")
        .is_empty());
}

#[test]
fn a_trendline_and_error_bars_are_authored_read_and_edited() {
    let (mut pres, frame) = deck_with(&labelled_chart());
    pres.add_chart_trendline(
        0,
        frame,
        0,
        &TrendlineSpec::new(TrendlineKind::Polynomial)
            .name("Fit")
            .polynomial_order(3)
            .projection(2.0, 1.0)
            .display(true, true),
    )
    .expect("add a trendline");
    pres.add_chart_trendline(
        0,
        frame,
        0,
        &TrendlineSpec::new(TrendlineKind::MovingAverage).moving_average_period(2),
    )
    .expect("add a second trendline");
    pres.set_chart_error_bars(
        0,
        frame,
        0,
        &ErrorBarSpec::custom(ErrorBarType::Both, vec![1.0, 2.0, 3.0], vec![0.5, 1.0, 1.5])
            .direction(ErrorBarDirection::Y)
            .no_end_cap(true),
    )
    .expect("add error bars");

    let saved = pres.save().expect("save");
    let mut pres = Presentation::open(&saved).expect("reopen");

    let trendlines = pres.chart_trendlines(0, frame, 0).expect("read trendlines");
    assert_eq!(trendlines.len(), 2);
    assert_eq!(trendlines[0].kind, Some(TrendlineKind::Polynomial));
    assert_eq!(trendlines[0].name.as_deref(), Some("Fit"));
    assert_eq!(trendlines[0].polynomial_order, Some(3));
    assert_eq!(trendlines[0].forward_periods, Some(2.0));
    assert_eq!(trendlines[0].backward_periods, Some(1.0));
    assert_eq!(trendlines[0].displays_equation, Some(true));
    assert_eq!(trendlines[0].displays_r_squared, Some(true));
    assert_eq!(trendlines[1].kind, Some(TrendlineKind::MovingAverage));
    assert_eq!(trendlines[1].moving_average_period, Some(2));

    let bars = pres.chart_error_bars(0, frame, 0).expect("read error bars");
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].direction, Some(ErrorBarDirection::Y));
    assert_eq!(bars[0].bar_type, Some(ErrorBarType::Both));
    assert_eq!(bars[0].value_type, Some(ErrorValueType::Custom));
    assert_eq!(bars[0].no_end_cap, Some(true));
    assert_eq!(bars[0].plus_values, vec![1.0, 2.0, 3.0]);
    assert_eq!(bars[0].minus_values, vec![0.5, 1.0, 1.5]);

    // Editing in place: the first trendline changes kind, the second does not; the error bars are
    // replaced rather than joined by a second set.
    pres.set_chart_trendline(
        0,
        frame,
        0,
        0,
        &TrendlineSpec::new(TrendlineKind::Linear).display(true, false),
    )
    .expect("rewrite the trendline");
    pres.set_chart_error_bars(
        0,
        frame,
        0,
        &ErrorBarSpec::fixed(ErrorBarType::Plus, ErrorValueType::Percentage, 5.0)
            .direction(ErrorBarDirection::Y),
    )
    .expect("rewrite the error bars");

    let trendlines = pres.chart_trendlines(0, frame, 0).expect("read trendlines");
    assert_eq!(trendlines.len(), 2);
    assert_eq!(trendlines[0].kind, Some(TrendlineKind::Linear));
    assert_eq!(trendlines[0].polynomial_order, None);
    assert_eq!(trendlines[1].kind, Some(TrendlineKind::MovingAverage));
    let bars = pres.chart_error_bars(0, frame, 0).expect("read error bars");
    assert_eq!(bars.len(), 1);
    assert_eq!(bars[0].value_type, Some(ErrorValueType::Percentage));
    assert_eq!(bars[0].value, Some(5.0));
    assert!(bars[0].plus_values.is_empty());

    assert_eq!(
        pres.remove_chart_trendlines(0, frame, 0).expect("remove"),
        2
    );
    assert_eq!(
        pres.remove_chart_error_bars(0, frame, 0).expect("remove"),
        1
    );
    assert!(pres.chart_trendlines(0, frame, 0).expect("read").is_empty());
    assert!(pres.chart_error_bars(0, frame, 0).expect("read").is_empty());
}

#[test]
fn decorating_a_chart_dirties_only_the_chart_part() {
    let bytes = fixture("charts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    pres.set_chart_data_labels(
        CHART_SURFACE,
        0,
        ChartLabelScope::Plot { plot_idx: 0 },
        &DataLabelSpec::new()
            .value(true)
            .position(DataLabelPosition::OutsideEnd),
    )
    .expect("label the plot");
    pres.set_chart_point_fill(
        CHART_SURFACE,
        0,
        0,
        1,
        &FillSpec::Solid(ColorSpec::Srgb("FF0000".into())),
    )
    .expect("colour a point");
    pres.add_chart_trendline(
        CHART_SURFACE,
        0,
        0,
        &TrendlineSpec::new(TrendlineKind::Linear),
    )
    .expect("add a trendline");
    pres.set_chart_error_bars(
        CHART_SURFACE,
        0,
        0,
        &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::StandardError, 1.0),
    )
    .expect("add error bars");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    assert_ne!(
        reopened.get("ppt/charts/chart1.xml"),
        original.get("ppt/charts/chart1.xml"),
        "the decorated chart part must have changed"
    );
    // Decoration is not data, so the embedded workbook is *not* refreshed — and every other part,
    // including the chart's own relationships, is byte-identical.
    for name in [
        "ppt/embeddings/Microsoft_Excel_Sheet1.xlsx",
        "ppt/charts/_rels/chart1.xml.rels",
        "ppt/slides/slide2.xml",
        "ppt/slides/_rels/slide2.xml.rels",
        "ppt/slides/slide1.xml",
        "[Content_Types].xml",
    ] {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "decorating a chart must leave {name} byte-identical"
        );
    }
    let before: Vec<&String> = original.keys().collect();
    let after: Vec<&String> = reopened.keys().collect();
    assert_eq!(before, after, "decorating a chart must add no parts");
}

#[test]
fn reading_chart_decoration_leaves_every_part_byte_identical() {
    let bytes = fixture("charts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let _ = pres
        .chart_data_labels(CHART_SURFACE, 0, 0, Some(0))
        .expect("read");
    let _ = pres
        .chart_data_label_tier(CHART_SURFACE, 0, ChartLabelScope::Plot { plot_idx: 0 })
        .expect("read");
    let _ = pres.chart_point_formats(CHART_SURFACE, 0, 0).expect("read");
    let _ = pres.chart_trendlines(CHART_SURFACE, 0, 0).expect("read");
    let _ = pres.chart_error_bars(CHART_SURFACE, 0, 0).expect("read");
    let _ = pres
        .chart_dangling_decoration(CHART_SURFACE, 0, 0)
        .expect("read");
    let _ = pres
        .chart_point_label_text(CHART_SURFACE, 0, 0, 0)
        .expect("read");

    let reopened = byte_map(&Package::open(&pres.save().expect("save")).expect("reopen"));
    for name in original.keys() {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "reading chart decoration must leave {name} byte-identical"
        );
    }
}

#[test]
fn the_decoration_surfaces_reject_the_wrong_shapes_indices_and_plot_types() {
    let (mut pres, frame) = deck_with(&labelled_chart());

    // A shape that frames no chart — the slide's layout placeholders sit before the frame.
    assert!(
        frame > 0,
        "the blank slide must carry a shape before the chart"
    );
    assert!(matches!(
        pres.chart_trendlines(0, 0, 0),
        Err(PptxError::ShapeIsNotAChart)
    ));

    // A series past the last.
    assert!(matches!(
        pres.chart_data_labels(0, frame, 9, None),
        Err(PptxError::ChartSeriesOutOfRange { index: 9, count: 2 })
    ));
    assert!(matches!(
        pres.add_chart_trendline(0, frame, 9, &TrendlineSpec::new(TrendlineKind::Linear)),
        Err(PptxError::ChartSeriesOutOfRange { index: 9, count: 2 })
    ));

    // A plot past the last.
    assert!(matches!(
        pres.chart_data_label_tier(0, frame, ChartLabelScope::Plot { plot_idx: 4 }),
        Err(PptxError::ChartPlotOutOfRange { index: 4, count: 1 })
    ));

    // A point past the end of its series — an index that addresses nothing.
    assert!(matches!(
        pres.set_chart_point_fill(
            0,
            frame,
            0,
            9,
            &FillSpec::Solid(ColorSpec::Srgb("FF0000".into()))
        ),
        Err(PptxError::ChartData(
            mjx_pptx::ChartDataError::DataPointOutOfRange { index: 9, count: 3 }
        ))
    ));

    // A trendline past the last.
    assert!(matches!(
        pres.set_chart_trendline(0, frame, 0, 0, &TrendlineSpec::new(TrendlineKind::Linear)),
        Err(PptxError::ChartTrendlineOutOfRange { index: 0, count: 0 })
    ));

    // An image fill on a point, which a chart part cannot relate to.
    assert!(matches!(
        pres.set_chart_point_fill(
            0,
            frame,
            0,
            0,
            &FillSpec::Picture {
                rel_id: "rId9".into(),
                mode: mjx_dml::PictureFillMode::Stretch,
            }
        ),
        Err(PptxError::ChartFillNotSupported)
    ));

    // A pie series declares no trendline and no error bars.
    let (mut pie, pie_frame) = deck_with(
        &ChartData::new(ChartKind::Pie)
            .categories(["A", "B"])
            .series("Share", [1.0, 2.0]),
    );
    assert!(matches!(
        pie.add_chart_trendline(0, pie_frame, 0, &TrendlineSpec::new(TrendlineKind::Linear)),
        Err(PptxError::ChartData(
            mjx_pptx::ChartDataError::DecorationNotAllowed {
                element: "trendline",
                ..
            }
        ))
    ));
    assert!(matches!(
        pie.set_chart_error_bars(
            0,
            pie_frame,
            0,
            &ErrorBarSpec::fixed(ErrorBarType::Both, ErrorValueType::FixedValue, 1.0)
        ),
        Err(PptxError::ChartData(
            mjx_pptx::ChartDataError::DecorationNotAllowed {
                element: "errBars",
                ..
            }
        ))
    ));
    // …but a slice may still be labelled and exploded.
    assert!(pie
        .set_chart_data_labels(
            0,
            pie_frame,
            ChartLabelScope::Series { series_idx: 0 },
            &DataLabelSpec::new().percentage(true)
        )
        .is_ok());
    assert!(pie
        .set_chart_point_explosion(0, pie_frame, 0, 0, Some(20))
        .is_ok());

    // A surface plot declares no data labels at all.
    let surface = ChartData::new(ChartKind::Surface)
        .categories(["A", "B"])
        .series("Height", [1.0, 2.0])
        .data_labels(DataLabelSpec::new().value(true));
    let mut deck = blank_deck_with_a_slide();
    assert!(matches!(
        deck.add_chart(0, &surface, ShapeBounds::from_inches(0.5, 0.5, 4.0, 3.0)),
        Err(PptxError::ChartData(
            mjx_pptx::ChartDataError::DecorationNotAllowed {
                element: "dLbls",
                ..
            }
        ))
    ));
}

/// Opens `bytes` so a deck this suite saved is at least readable end to end. The schema gate proper
/// lives in `schema_validity.rs`.
fn assert_authored_deck_is_readable(bytes: &[u8]) {
    Presentation::open(bytes).expect("the saved deck reopens");
}

// -------------------------------------------------------------------------------------------
// Tier 3, inside a part: an edit through the whole-part typed model rewrites only what it changed
// -------------------------------------------------------------------------------------------

/// `charts.pptx`'s chart part is written with every start tag on one line, so a "did it re-flow?"
/// assertion against it would pass whether or not anything preserved the source bytes. This puts the
/// wrapping back — the way Office writes it — and rebuilds the deck around it.
///
/// The transformation is whitespace *inside start tags* only, which XML says is insignificant, so
/// the deck still parses, validates and means exactly what it did.
fn deck_whose_chart_part_wraps_its_start_tags() -> (Vec<u8>, Vec<u8>) {
    let name = part("/ppt/charts/chart1.xml");
    let mut pkg = Package::open(&fixture("charts.pptx")).expect("open");
    let flat = String::from_utf8(pkg.part_bytes(&name).expect("the chart part").to_vec())
        .expect("the chart part is UTF-8");
    let wrapped = flat
        .replace(
            r#"<c:barDir val="col"/>"#,
            "<c:barDir\r\n          val=\"col\"/>",
        )
        .replace(
            r#"<c:grouping val="clustered"/>"#,
            "<c:grouping val=\"clustered\"\r\n          />",
        )
        .replace(
            r#"<c:ptCount val="3"/>"#,
            "<c:ptCount\r\n            val=\"3\"\r\n          />",
        );
    assert_ne!(
        wrapped, flat,
        "the chart part did not have the shape expected"
    );
    pkg.replace_part_bytes(&name, wrapped.clone().into_bytes())
        .expect("replace the chart part");
    (pkg.save().expect("save"), wrapped.into_bytes())
}

/// `edit_chart` reads the whole chart part into a [`ChartSpace`] and writes the whole part back from
/// it. Until MJXOFF-143 that rebuilt every element, so a one-word title change re-flowed the entire
/// chart. Now only the path from the root down to what changed is reconstructed.
#[test]
fn setting_a_chart_title_leaves_every_other_element_byte_identical() {
    let (deck, before) = deck_whose_chart_part_wraps_its_start_tags();
    let name = part("/ppt/charts/chart1.xml");

    let mut pres = Presentation::open(&deck).expect("open");
    pres.set_chart_title(CHART_SURFACE, 0, Some("Regional sales"))
        .expect("set the title");
    let saved = pres.save().expect("save");
    let after = Package::open(&saved)
        .expect("reopen")
        .part_bytes(&name)
        .expect("the chart part")
        .to_vec();

    let contains = |haystack: &[u8], needle: &[u8]| {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    };
    assert!(
        contains(&after, b"Regional sales"),
        "the edit did not land:\n{}",
        String::from_utf8_lossy(&after)
    );
    assert_ne!(after, before, "the edit should have changed the part");

    // The three wrapped start tags sit under `c:plotArea`, which the title edit never reaches. Each
    // is still literally the bytes it was parsed from — CRLF and indent included, which no
    // reconstruction from the model could produce.
    for wrapped in [
        &b"<c:barDir\r\n          val=\"col\"/>"[..],
        &b"<c:grouping val=\"clustered\"\r\n          />"[..],
        &b"<c:ptCount\r\n            val=\"3\"\r\n          />"[..],
    ] {
        assert!(
            contains(&after, wrapped),
            "an untouched element was re-flowed:\n{}\n\nwanted verbatim:\n{}",
            String::from_utf8_lossy(&after),
            String::from_utf8_lossy(wrapped)
        );
    }

    // And the deck still reads back as itself.
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.chart_title(CHART_SURFACE, 0).expect("read"),
        Some("Regional sales".to_owned())
    );
}
