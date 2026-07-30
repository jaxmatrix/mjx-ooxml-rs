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

use mjx_opc::{Package, PartName, TargetMode};
use mjx_pptx::{
    ChartData, ChartKind, ChartSeriesData, ChartWorkbook, GraphicFrameKind, PptxError,
    Presentation, ShapeBounds,
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
fn editing_a_chart_dirties_only_the_chart_xml() {
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
    // ...but everything else — including the now-stale embedded workbook — is untouched.
    for name in [
        "ppt/charts/_rels/chart1.xml.rels",
        "ppt/embeddings/Microsoft_Excel_Sheet1.xlsx",
        "ppt/slides/slide2.xml",
        "ppt/slides/_rels/slide2.xml.rels",
        "ppt/slides/slide1.xml",
    ] {
        assert_eq!(
            reopened.get(name),
            original.get(name),
            "editing chart data must leave {name} byte-identical"
        );
    }
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

    // A freshly authored chart carries cached data only — no workbook to detach.
    let mut authored = Presentation::open(&fixture("sample.pptx")).expect("open");
    let idx = authored
        .add_chart(0, &bar_chart(), bounds())
        .expect("add chart");
    assert!(matches!(
        authored.detach_chart_workbook(0, idx),
        Err(PptxError::ChartHasNoExternalData)
    ));
}
