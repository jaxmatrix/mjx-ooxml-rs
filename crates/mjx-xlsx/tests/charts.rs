//! MJXOFF-111 (E4) — charts on the Excel surface, including the one chart case that exists nowhere
//! else in this library: a chart whose data source is a **live range in the same workbook**.
//!
//! # The non-discriminating test this file exists to avoid
//!
//! The ticket names it exactly: *"a fixture whose cached values equal its cell values proves nothing
//! about which one the reader used."* Every reader here could be answering from the cache, from the
//! cells, or from a coin toss, and a workbook whose chart caches `10, 20, 30` over cells holding
//! `10, 20, 30` would let all three pass.
//!
//! So the discriminating cases are driven by `tests/fixtures/chart_stale_cache.xlsx`, in which the
//! two **disagree on every point**: its chart caches say `North/South/East = 10/20/30` and its cells
//! say `Alpha/Beta/Gamma = 111/222/333`. A reader that answers `10` is reading the cache; one that
//! answers `111` is reading the cells; and there is no answer that could be either.
//!
//! Both fixtures were written by **LibreOffice 25.8.7.3**, driven headless over UNO — this project
//! wrote neither. `chart_in_sheet.xlsx` is one run of that script exactly as LibreOffice saved it;
//! `chart_stale_cache.xlsx` is the same package with the `xl/worksheets/sheet1.xml` and
//! `xl/sharedStrings.xml` of a *second* run spliced in, so every byte of it is LibreOffice's too and
//! its chart part is untouched. That is what a workbook edited by a tool which does not recalculate
//! looks like — which is precisely what this library is.
//!
//! The producer fixture disagrees with this library's own writer in ways a byte-identity claim over
//! it cannot satisfy by accident: it writes `t="n"` on every number where this library writes no
//! `t`, spells its flags `false` where this library writes `0`, numbers its `xdr:cNvPr` `1` with an
//! empty `@name`, and puts `xml:space="preserve"` on every shared string.

use mjx_chart::{ChartData, ChartKind, LegendPosition};
use mjx_dml::spreadsheet_drawing::CellMarker;
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_opc::{Package, PartName};
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::{RangeProblem, SheetChartSeries, SheetChartSource, Workbook, XlsxError};

/// The producer-written fixture whose caches and cells **agree**.
fn producer_workbook() -> Vec<u8> {
    mjx_fixtures::fixture("chart_in_sheet.xlsx")
}

/// The producer-written fixture whose caches and cells **disagree**.
fn stale_cache_workbook() -> Vec<u8> {
    mjx_fixtures::fixture("chart_stale_cache.xlsx")
}

/// Every part of `bytes`, as `(name, decompressed payload)`, sorted by name.
fn part_payloads(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let package = Package::open(bytes).expect("the package opens");
    let mut parts: Vec<(String, Vec<u8>)> = package
        .part_names()
        .map(|name| {
            (
                name.as_str().to_owned(),
                package
                    .part_bytes(&name)
                    .expect("a named part has bytes")
                    .to_vec(),
            )
        })
        .collect();
    parts.sort_by(|a, b| a.0.cmp(&b.0));
    parts
}

/// A cell reference from a literal address.
fn at(address: &str) -> CellReference {
    CellReference::parse(address).expect("a literal address")
}

// =================================================================================================
// Reading a chart somebody else wrote
// =================================================================================================

#[test]
fn a_producer_authored_workbook_with_a_chart_round_trips_byte_for_byte() {
    // Tier 1, over a file this project did not write. `tests/roundtrip.rs` sweeps the whole corpus
    // for the same property; this pins it for the fixture whose chart part, drawing part and
    // relationship graph are the subject of every case below, so a regression here names the chart
    // rather than "one of sixteen workbooks".
    let before = producer_workbook();
    let workbook = Workbook::open(&before).expect("the producer workbook opens");
    let after = workbook.save().expect("it saves");

    let (left, right) = (part_payloads(&before), part_payloads(&after));
    assert_eq!(
        left.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        right.iter().map(|(name, _)| name).collect::<Vec<_>>(),
        "the part set changed"
    );
    for ((name, before), (_, after)) in left.iter().zip(right.iter()) {
        assert_eq!(before, after, "{name} did not survive open and save");
    }
}

#[test]
fn the_producer_fixtures_chart_is_found_through_its_anchor_and_read_through_its_caches() {
    let workbook = Workbook::open(&producer_workbook()).expect("opens");

    // The chart is reached the way a consumer reaches it: sheet → `x:drawing` → drawing part →
    // anchor → `xdr:graphicFrame` → `a:graphicData` → `c:chart@r:id` → chart part. A reader that
    // found charts by walking the package for `xl/charts/*.xml` would pass every case below and
    // still be wrong, so this asserts the *address* rather than only the answer.
    assert_eq!(
        workbook.chart_anchor_indices(0).expect("anchors"),
        vec![0],
        "the fixture anchors exactly one chart, first in paint order"
    );
    assert_eq!(
        workbook.chart_rel_id(0, 0).expect("rel id").as_deref(),
        Some("rId1")
    );
    let bytes = workbook
        .chart_part_bytes(0, 0)
        .expect("part bytes")
        .expect("the anchor frames a chart");
    assert!(
        bytes.starts_with(b"<?xml"),
        "the chart part comes back as the package holds it"
    );

    assert_eq!(
        workbook.chart_kinds(0, 0).expect("kinds"),
        vec![ChartKind::Bar]
    );
    let series = workbook.chart_series(0, 0).expect("series");
    assert_eq!(series.len(), 1);
    assert_eq!(series[0].name.as_deref(), Some("Revenue"));
    assert_eq!(series[0].categories, ["North", "South", "East"]);
    assert_eq!(series[0].values, [10.0, 20.0, 30.0]);
    assert_eq!(workbook.chart_axes(0, 0).expect("axes").len(), 2);
}

#[test]
fn an_anchor_that_frames_no_chart_is_a_typed_refusal_rather_than_a_guess() {
    let workbook = Workbook::open(&producer_workbook()).expect("opens");
    // Index 1 is past the fixture's only anchor.
    assert_eq!(workbook.chart_rel_id(0, 1).expect("no chart"), None);
    assert!(matches!(
        workbook.chart_series(0, 1),
        Err(XlsxError::AnchorIsNotAChart {
            sheet_index: 0,
            anchor_index: 1
        })
    ));
    // …and a sheet index that names no tab is the other refusal, told apart from the first.
    assert!(matches!(
        workbook.chart_anchor_indices(9),
        Err(XlsxError::NoSuchSheet {
            index: 9,
            sheets: 1
        })
    ));
}

#[test]
fn the_chart_says_where_its_data_lives_and_the_text_is_the_producers() {
    let workbook = Workbook::open(&producer_workbook()).expect("opens");
    let references = workbook.chart_series_references(0, 0).expect("references");
    assert_eq!(references.len(), 1);
    // Preserved as written, absolute markers and all — never re-spelled.
    assert_eq!(references[0].name.as_deref(), Some("Data!$B$1"));
    assert_eq!(references[0].categories.as_deref(), Some("Data!$A$2:$A$4"));
    assert_eq!(references[0].values.as_deref(), Some("Data!$B$2:$B$4"));
}

// =================================================================================================
// Cache versus cells — the whole point of this child
// =================================================================================================

#[test]
fn the_cache_reader_and_the_cell_reader_answer_differently_when_the_two_disagree() {
    // THE discriminating case. The fixture's caches and cells disagree on every single point, so
    // each assertion below can only be satisfied by the source it names.
    let mut workbook = Workbook::open(&stale_cache_workbook()).expect("opens");

    let cached = workbook.chart_series(0, 0).expect("cached series");
    assert_eq!(
        (cached[0].categories.clone(), cached[0].values.clone()),
        (
            vec!["North".to_owned(), "South".to_owned(), "East".to_owned()],
            vec![10.0, 20.0, 30.0]
        ),
        "chart_series must answer from the chart's caches"
    );

    let from_cells = workbook.chart_series_from_cells(0, 0).expect("cell series");
    assert_eq!(
        (
            from_cells[0].categories.clone(),
            from_cells[0].values.clone()
        ),
        (
            vec!["Alpha".to_owned(), "Beta".to_owned(), "Gamma".to_owned()],
            vec![111.0, 222.0, 333.0]
        ),
        "chart_series_from_cells must answer from the sheet's cells"
    );

    // Neither is preferred: both are reported and each is named.
    let freshness = workbook.chart_series_freshness(0, 0).expect("freshness");
    assert_eq!(freshness.len(), 1);
    assert_eq!(freshness[0].cached.values, [10.0, 20.0, 30.0]);
    assert_eq!(freshness[0].from_cells.values, [111.0, 222.0, 333.0]);
    assert_eq!(freshness[0].values_agree, Some(false));
    assert_eq!(freshness[0].categories_agree, Some(false));
    assert_eq!(freshness[0].values_problem, None);
    assert_eq!(freshness[0].categories_problem, None);
    // The series' name is a reference too, and the cells still agree with the cache about that one
    // — which is what makes the two `false`s above statements about the data rather than about the
    // whole series.
    assert_eq!(freshness[0].from_cells.name.as_deref(), Some("Revenue"));
}

#[test]
fn a_workbook_whose_cells_agree_with_its_caches_says_so() {
    // The other half of the pair. Without this, `values_agree` could be a constant `false` and the
    // case above would not notice.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let freshness = workbook.chart_series_freshness(0, 0).expect("freshness");
    assert_eq!(freshness[0].values_agree, Some(true));
    assert_eq!(freshness[0].categories_agree, Some(true));
    assert_eq!(freshness[0].cached.values, freshness[0].from_cells.values);
}

#[test]
fn editing_a_cell_the_chart_reads_leaves_the_cache_stale_and_the_staleness_is_reported() {
    // The decision this child had to make, stated as a test rather than as prose: **the cached
    // values are left alone and reported as stale.** Rewriting them on a cell edit would be this
    // library recalculating, which MJXOFF-115 settles as a permanent non-goal, and it would silently
    // change a part the caller did not ask to touch.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    assert_eq!(
        workbook.chart_series_freshness(0, 0).expect("before")[0].values_agree,
        Some(true)
    );

    workbook
        .set_cell_value(0, at("B3"), CellValue::Number(999.0))
        .expect("the store accepts the value");

    let cached = workbook.chart_series(0, 0).expect("cached");
    assert_eq!(
        cached[0].values,
        [10.0, 20.0, 30.0],
        "a cell edit must not touch the chart's cache"
    );
    let freshness = workbook.chart_series_freshness(0, 0).expect("after");
    assert_eq!(freshness[0].from_cells.values, [10.0, 999.0, 30.0]);
    assert_eq!(
        freshness[0].values_agree,
        Some(false),
        "the staleness must be reported, not left to be discovered"
    );
}

#[test]
fn refreshing_the_cache_from_the_cells_is_opt_in_and_makes_the_two_agree() {
    let mut workbook = Workbook::open(&stale_cache_workbook()).expect("opens");
    let changed = workbook
        .refresh_chart_cache_from_cells(0, 0)
        .expect("refresh");
    assert_eq!(changed, 2, "one series' values and its categories");

    let cached = workbook.chart_series(0, 0).expect("cached");
    assert_eq!(cached[0].values, [111.0, 222.0, 333.0]);
    assert_eq!(cached[0].categories, ["Alpha", "Beta", "Gamma"]);
    let freshness = workbook.chart_series_freshness(0, 0).expect("freshness");
    assert_eq!(freshness[0].values_agree, Some(true));
    assert_eq!(freshness[0].categories_agree, Some(true));

    // Nothing was refreshed a second time: the repair is idempotent, and a chart already in step
    // reports zero rather than rewriting its own caches for nothing.
    assert_eq!(
        workbook
            .refresh_chart_cache_from_cells(0, 0)
            .expect("second refresh"),
        0
    );
}

#[test]
fn refreshing_the_cache_touches_the_chart_part_and_nothing_else() {
    // Edit isolation, over a producer file: the repair rewrites one part and leaves every other one
    // byte-identical — the sheet it read from included.
    let before = stale_cache_workbook();
    let mut workbook = Workbook::open(&before).expect("opens");
    workbook
        .refresh_chart_cache_from_cells(0, 0)
        .expect("refresh");
    let after = workbook.save().expect("saves");

    let (left, right) = (part_payloads(&before), part_payloads(&after));
    let mut changed: Vec<&str> = Vec::new();
    for ((name, before), (_, after)) in left.iter().zip(right.iter()) {
        if before != after {
            changed.push(name);
        }
    }
    assert_eq!(changed, ["/xl/charts/chart1.xml"]);
}

// =================================================================================================
// The refresh path declines — two tests, different results
// =================================================================================================

#[test]
fn a_live_range_chart_declines_to_refresh_a_workbook_it_never_had() {
    // MJXOFF-111's own *Done when* clause. `false`, never a fabricated workbook — and the package
    // must come out of the call with the parts it went in with, because the failure this guards
    // against is a `refresh` that helpfully *creates* the embedded workbook a live-range chart is
    // defined by not having.
    let before = producer_workbook();
    let mut workbook = Workbook::open(&before).expect("opens");
    assert!(
        !workbook.refresh_chart_workbook(0, 0).expect("refresh"),
        "a chart with no c:externalData has nothing to refresh"
    );
    assert!(
        workbook.chart_workbooks().expect("workbooks").is_empty(),
        "and it names no workbook to begin with"
    );
    let after = workbook.save().expect("saves");
    assert_eq!(
        part_payloads(&before),
        part_payloads(&after),
        "declining to refresh must change nothing at all"
    );

    // Detaching is the one call that *does* raise, because detaching nothing is a caller error.
    assert!(matches!(
        workbook.detach_chart_workbook(0, 0),
        Err(XlsxError::ChartHasNoExternalData)
    ));
}

#[test]
fn a_sheet_chart_that_carries_a_workbook_refreshes_normally() {
    // The other half of the pair, and the reason the case above is not vacuous: the same method on
    // the same surface answers `true` when there really is a workbook behind the chart.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2"])
        .series("Plan", [1.0, 2.0]);
    let anchor = workbook
        .add_chart(
            0,
            &chart,
            CellMarker::new(0, 0, 10, 0),
            CellMarker::new(5, 0, 25, 0),
            "Plan",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a chart with an embedded workbook");

    assert!(workbook.refresh_chart_workbook(0, anchor).expect("refresh"));
    let workbooks = workbook.chart_workbooks().expect("workbooks");
    assert_eq!(workbooks.len(), 1);
    assert_eq!(workbooks[0].anchor_index, anchor);
    assert_eq!(
        workbooks[0].target,
        "../embeddings/Microsoft_Excel_Sheet1.xlsx"
    );
    assert!(!workbooks[0].external);

    // …and the live-range chart already in the file still declines, in the same package.
    assert!(!workbook.refresh_chart_workbook(0, 0).expect("refresh"));

    // Detaching removes the reference, the relationship **and the workbook part**, and the chart
    // then declines too. The part removal is not a courtesy: `save` runs `Package::validate`, which
    // refuses a SpreadsheetML part no relationship chain reaches — so a detach that left the
    // embedded workbook behind would hand back a workbook this library then declines to write, and
    // the caller would meet that on the next save rather than here.
    workbook.detach_chart_workbook(0, anchor).expect("detach");
    assert!(!workbook.refresh_chart_workbook(0, anchor).expect("refresh"));
    assert!(workbook.chart_workbooks().expect("workbooks").is_empty());
    assert!(
        !workbook
            .package()
            .part_names()
            .any(|part| part.as_str().starts_with("/xl/embeddings/")),
        "the detached workbook part must go with its relationship"
    );
    workbook.save().expect("and the package still validates");
}

#[test]
fn detaching_one_chart_of_two_that_share_a_workbook_leaves_the_part_where_it_is() {
    // The other half of the sweep, and the reason it is conditional rather than unconditional: a
    // second chart naming the same workbook keeps it alive. Removing it on the first detach would
    // leave the second chart pointing at a part that is not there — the defect `Package::validate`
    // exists to catch, planted by the code that was avoiding it.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["A"])
        .series("S", [1.0]);
    for (name, row) in [("First", 10), ("Second", 30)] {
        workbook
            .add_chart(
                0,
                &chart,
                CellMarker::new(0, 0, row, 0),
                CellMarker::new(5, 0, row + 15, 0),
                name,
                ResizingBehavior::MoveWithCellsButDoNotResize,
            )
            .expect("a chart");
    }

    // Each chart got a workbook of its own; point the second at the first's and drop the spare.
    // Through `mjx-opc` directly, because nothing on this surface makes two charts share a workbook
    // — which is the point: the file being tested is one this library would not author, and a file
    // it did not author is exactly what the conditional exists for.
    let mut package = Package::open(&workbook.save().expect("saves")).expect("reopens");
    let second_chart = PartName::new("/xl/charts/chart3.xml").expect("a part name");
    package
        .remove_relationship(Some(&second_chart), "rId1")
        .expect("the second chart's own workbook relationship goes");
    package
        .add_relationship(
            Some(&second_chart),
            mjx_opc::Relationship {
                id: "rId1".to_owned(),
                rel_type:
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package"
                        .to_owned(),
                target: "../embeddings/Microsoft_Excel_Sheet1.xlsx".to_owned(),
                mode: mjx_opc::TargetMode::Internal,
            },
        )
        .expect("and it names the first chart's instead");
    package
        .remove_part(&PartName::new("/xl/embeddings/Microsoft_Excel_Sheet2.xlsx").expect("a part"))
        .expect("the now-unreferenced second workbook goes");
    let mut workbook = Workbook::open(&package.save().expect("saves")).expect("reopens");

    assert_eq!(
        workbook.chart_workbooks().expect("workbooks").len(),
        2,
        "both charts name the one workbook"
    );
    workbook.detach_chart_workbook(0, 1).expect("detach");
    assert!(
        workbook
            .package()
            .part_names()
            .any(|part| part.as_str() == "/xl/embeddings/Microsoft_Excel_Sheet1.xlsx"),
        "the second chart still names it, so it must stay"
    );
    assert!(workbook.refresh_chart_workbook(0, 2).expect("refresh"));
    workbook.save().expect("and the package still validates");
}

// =================================================================================================
// Authoring
// =================================================================================================

#[test]
fn authoring_a_range_chart_writes_live_formulas_and_no_embedded_workbook() {
    let before = producer_workbook();
    let mut workbook = Workbook::open(&before).expect("opens");
    let source = SheetChartSource {
        categories: Some("Data!$A$2:$A$4".to_owned()),
        series: vec![SheetChartSeries {
            name_cell: Some("Data!$B$1".to_owned()),
            name: "unused".to_owned(),
            values: "Data!$B$2:$B$4".to_owned(),
        }],
    };
    let anchor = workbook
        .add_range_chart(
            0,
            ChartKind::Line,
            &source,
            CellMarker::new(0, 0, 30, 0),
            CellMarker::new(5, 0, 45, 0),
            "Live",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a live-range chart");

    // The formulas name the cells the caller gave, not the embedded workbook `ChartData` would
    // otherwise have pointed at.
    let references = workbook
        .chart_series_references(0, anchor)
        .expect("references");
    assert_eq!(references[0].values.as_deref(), Some("Data!$B$2:$B$4"));
    assert_eq!(references[0].categories.as_deref(), Some("Data!$A$2:$A$4"));
    assert_eq!(references[0].name.as_deref(), Some("Data!$B$1"));

    // The caches were seeded from the cells, and the series took its name from the header cell
    // rather than from the fallback.
    let series = workbook.chart_series(0, anchor).expect("series");
    assert_eq!(series[0].name.as_deref(), Some("Revenue"));
    assert_eq!(series[0].values, [10.0, 20.0, 30.0]);
    assert_eq!(series[0].categories, ["North", "South", "East"]);

    // No embedded workbook, now or ever.
    assert!(!workbook.refresh_chart_workbook(0, anchor).expect("refresh"));
    let saved = workbook.save().expect("saves");
    let package = Package::open(&saved).expect("reopens");
    assert!(
        !package
            .part_names()
            .any(|name| name.as_str().starts_with("/xl/embeddings/")),
        "a live-range chart must not create an embedded workbook"
    );
    assert!(!String::from_utf8_lossy(
        package
            .part_bytes(&PartName::new("/xl/charts/chart2.xml").expect("a part name"))
            .expect("the authored chart part")
    )
    .contains("externalData"));
}

#[test]
fn authoring_a_chart_adds_exactly_its_own_parts_and_leaves_every_other_one_byte_identical() {
    // The edit-isolation tier, stated for this feature: adding a chart to a sheet touches the
    // drawing part (which gains an anchor), `[Content_Types].xml` and the relationship stream that
    // reaches the new parts — and nothing else. The worksheet is the assertion that matters: adding
    // a chart writes no cell, exactly as MJXOFF-107's adding a picture does not.
    let before = producer_workbook();
    let mut workbook = Workbook::open(&before).expect("opens");
    let chart = ChartData::new(ChartKind::Pie)
        .categories(["A", "B"])
        .series("Share", [30.0, 70.0])
        .title("Share of revenue")
        .legend(LegendPosition::Bottom);
    workbook
        .add_chart(
            0,
            &chart,
            CellMarker::new(6, 0, 1, 0),
            CellMarker::new(12, 0, 15, 0),
            "Share",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a chart");
    let after = workbook.save().expect("saves");

    let left = part_payloads(&before);
    let right = part_payloads(&after);
    let added: Vec<&str> = right
        .iter()
        .map(|(name, _)| name.as_str())
        .filter(|name| !left.iter().any(|(old, _)| old == name))
        .collect();
    assert_eq!(
        added,
        [
            "/xl/charts/_rels/chart2.xml.rels",
            "/xl/charts/chart2.xml",
            "/xl/embeddings/Microsoft_Excel_Sheet1.xlsx",
        ],
        "exactly the parts a chart with an embedded workbook needs"
    );

    let changed: Vec<&str> = left
        .iter()
        .filter_map(|(name, before)| {
            let after = right.iter().find(|(other, _)| other == name)?;
            (before != &after.1).then_some(name.as_str())
        })
        .collect();
    assert_eq!(
        changed,
        [
            "/xl/drawings/_rels/drawing1.xml.rels",
            "/xl/drawings/drawing1.xml",
        ],
        "the worksheet, the styles, the theme and the shared strings are untouched — adding a \
         chart writes no cell, exactly as adding a picture does not"
    );
}

#[test]
fn a_range_chart_naming_cells_this_workbook_does_not_have_is_refused_before_anything_is_written() {
    // A chart being *read* is treated leniently — a `c:f` naming a deleted sheet is a fact about
    // that file. A chart being *authored* is the opposite case: writing a reference that names
    // nothing would put the defect there on purpose.
    let before = producer_workbook();
    let mut workbook = Workbook::open(&before).expect("opens");
    let source = SheetChartSource {
        categories: None,
        series: vec![SheetChartSeries {
            name_cell: None,
            name: "Ghost".to_owned(),
            values: "NoSuchSheet!$B$2:$B$4".to_owned(),
        }],
    };
    let refusal = workbook.add_range_chart(
        0,
        ChartKind::Bar,
        &source,
        CellMarker::new(0, 0, 30, 0),
        CellMarker::new(5, 0, 45, 0),
        "Ghost",
        ResizingBehavior::MoveWithCellsButDoNotResize,
    );
    assert!(matches!(refusal, Err(XlsxError::TargetResolution { .. })));
    assert_eq!(
        part_payloads(&before),
        part_payloads(&workbook.save().expect("saves")),
        "a refused authoring call must leave the package exactly as it was"
    );
}

#[test]
fn a_chart_description_with_nothing_to_draw_is_refused_with_its_own_variant() {
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let empty = ChartData::new(ChartKind::Bar);
    assert!(matches!(
        workbook.add_chart(
            0,
            &empty,
            CellMarker::new(0, 0, 1, 0),
            CellMarker::new(1, 0, 2, 0),
            "Nothing",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        ),
        Err(XlsxError::InvalidChartData)
    ));
    // …and a plot type whose series count the schema constrains keeps the *other* variant, so a
    // caller can tell the two mistakes apart.
    let one_series = ChartData::new(ChartKind::Stock)
        .categories(["A"])
        .series("Open", [1.0]);
    assert!(matches!(
        workbook.add_chart(
            0,
            &one_series,
            CellMarker::new(0, 0, 1, 0),
            CellMarker::new(1, 0, 2, 0),
            "Stock",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        ),
        Err(XlsxError::ChartData(_))
    ));
}

#[test]
fn a_chart_frame_is_removed_by_the_anchor_surface_that_removes_every_other_object() {
    // The address is MJXOFF-107's, so `add_chart`'s return value is accepted by
    // `remove_sheet_drawing_object` exactly as `add_two_cell_anchored_picture`'s is. A chart with a
    // second addressing scheme of its own would have needed a second removal call.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["A"])
        .series("S", [1.0]);
    let anchor = workbook
        .add_chart(
            0,
            &chart,
            CellMarker::new(0, 0, 10, 0),
            CellMarker::new(5, 0, 25, 0),
            "Removable",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a chart");
    assert_eq!(
        workbook.chart_anchor_indices(0).expect("anchors"),
        [0, anchor]
    );
    assert!(workbook
        .remove_sheet_drawing_object(0, anchor)
        .expect("removed"));
    assert_eq!(workbook.chart_anchor_indices(0).expect("anchors"), [0]);
}

// =================================================================================================
// Edits, on the same surface as the other two formats
// =================================================================================================

#[test]
fn the_chart_edit_family_works_from_a_workbook_exactly_as_it_does_from_a_deck() {
    // Every method here is the same name, taking the same vocabulary, as `mjx_pptx::Presentation`'s
    // and `mjx_docx::Document`'s. What differs is only the address in front of it.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");

    workbook
        .set_chart_title(0, 0, Some("Regional revenue"))
        .expect("title");
    assert_eq!(
        workbook.chart_title(0, 0).expect("title").as_deref(),
        Some("Regional revenue")
    );

    workbook
        .set_chart_legend(0, 0, Some(LegendPosition::Right))
        .expect("legend");
    assert_eq!(
        workbook
            .chart_legend(0, 0)
            .expect("legend")
            .expect("a legend")
            .position,
        Some(LegendPosition::Right)
    );

    workbook
        .set_chart_series_values(0, 0, 0, &[40.0, 41.0, 42.0])
        .expect("values");
    assert_eq!(
        workbook.chart_series(0, 0).expect("series")[0].values,
        [40.0, 41.0, 42.0]
    );
    workbook
        .set_chart_series_categories(0, 0, 0, &["X", "Y", "Z"])
        .expect("categories");
    assert_eq!(
        workbook.chart_series(0, 0).expect("series")[0].categories,
        ["X", "Y", "Z"]
    );

    workbook
        .set_chart_axis_scale(0, 0, 1, Some(0.0), Some(50.0))
        .expect("scale");
    workbook
        .set_chart_axis_title(0, 0, 1, Some("Millions"))
        .expect("axis title");
    workbook
        .set_chart_axis_gridlines(0, 0, 1, true, false)
        .expect("gridlines");
    let axis = &workbook.chart_axes(0, 0).expect("axes")[1];
    assert_eq!(axis.minimum, Some(0.0));
    assert_eq!(axis.maximum, Some(50.0));
    assert_eq!(axis.title.as_deref(), Some("Millions"));

    // …and the whole edited chart still round-trips through the reader that found it.
    assert_eq!(
        workbook.chart_kinds(0, 0).expect("kinds"),
        vec![ChartKind::Bar]
    );

    // Editing the chart writes the chart part and nothing else — the sheet the chart reads from is
    // not touched by an edit to the numbers it draws, which is the other half of the rule the cell
    // edit above states.
    let saved = workbook.save().expect("saves");
    let (left, right) = (part_payloads(&producer_workbook()), part_payloads(&saved));
    let changed: Vec<&str> = left
        .iter()
        .filter_map(|(name, before)| {
            let after = right.iter().find(|(other, _)| other == name)?;
            (before != &after.1).then_some(name.as_str())
        })
        .collect();
    assert_eq!(changed, ["/xl/charts/chart1.xml"]);
}

#[test]
fn an_index_past_the_last_series_is_a_chart_level_refusal_carrying_the_shared_error() {
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let refusal = workbook.set_chart_series_values(0, 0, 7, &[1.0]);
    let Err(XlsxError::ChartAccess(problem)) = refusal else {
        panic!("expected a ChartAccess refusal, got {refusal:?}");
    };
    assert_eq!(
        problem,
        mjx_chart::ChartAccessError::SeriesOutOfRange { index: 7, count: 1 },
        "the verdict is `mjx-chart`'s own, not a second one written here"
    );
}

// =================================================================================================
// The range resolver
// =================================================================================================

#[test]
fn a_reference_resolves_to_its_cells_with_the_offsets_a_cache_would_use() {
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let resolved = workbook
        .resolve_range_reference(0, "Data!$B$2:$B$4")
        .expect("resolves");
    assert!(resolved.is_fully_resolved());
    assert_eq!(resolved.addressed_cells, 3);
    assert_eq!(
        resolved
            .cells
            .iter()
            .map(|cell| (cell.offset, cell.value.number()))
            .collect::<Vec<_>>(),
        [(0, Some(10.0)), (1, Some(20.0)), (2, Some(30.0))]
    );
    assert_eq!(resolved.numbers(3), [Some(10.0), Some(20.0), Some(30.0)]);
}

#[test]
fn a_blank_cell_is_absent_rather_than_zero_and_the_offsets_still_line_up() {
    // The shape a cache has, and the reason the resolver has it too: a `c:numCache` writes a `c:pt`
    // for the points it has and omits the rest. A resolver that padded blanks with zeros would plot
    // a zero where a chart draws a gap.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let resolved = workbook
        .resolve_range_reference(0, "Data!$B$1:$B$10")
        .expect("resolves");
    assert_eq!(resolved.addressed_cells, 10);
    // `B1` is the header (a string), `B2:B4` the numbers, `B5:B10` blank.
    assert_eq!(resolved.cells.len(), 4);
    assert_eq!(
        resolved
            .cells
            .iter()
            .map(|cell| cell.offset)
            .collect::<Vec<_>>(),
        [0, 1, 2, 3]
    );
    assert_eq!(
        resolved.numbers(10),
        [
            None,
            Some(10.0),
            Some(20.0),
            Some(30.0),
            None,
            None,
            None,
            None,
            None,
            None
        ]
    );
}

#[test]
fn a_reference_that_names_several_areas_resolves_each_in_the_order_it_wrote_them() {
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    for text in [
        "Data!$B$2:$B$3,Data!$B$4:$B$4",
        "(Data!$B$2:$B$3,Data!$B$4:$B$4)",
    ] {
        let resolved = workbook.resolve_range_reference(0, text).expect("resolves");
        assert!(resolved.is_fully_resolved(), "{text}");
        assert_eq!(resolved.areas.len(), 2, "{text}");
        assert_eq!(resolved.addressed_cells, 3, "{text}");
        assert_eq!(
            resolved
                .cells
                .iter()
                .map(|cell| (cell.offset, cell.value.number()))
                .collect::<Vec<_>>(),
            [(0, Some(10.0)), (1, Some(20.0)), (2, Some(30.0))],
            "{text}"
        );
    }
}

/// The producer workbook with `definedNames` spliced into its `xl/workbook.xml`.
///
/// Nothing in this library authors a defined name — the gap is recorded in the Excel guide's
/// `deliberate_limitations` page — so the markup is written here, in the spelling Excel and
/// LibreOffice write, and read back through the *same* resolver a chart's `c:f` goes through. The
/// element goes between `</sheets>` and `<calcPr`, which is where rank 12 of `CT_Workbook` puts it.
fn workbook_with_defined_names(names: &[(&str, Option<u32>, &str)]) -> Vec<u8> {
    let mut package = Package::open(&producer_workbook()).expect("the package opens");
    let part = PartName::new("/xl/workbook.xml").expect("a valid part name");
    let bytes = package
        .part_bytes(&part)
        .expect("the workbook part")
        .to_vec();
    let text = String::from_utf8(bytes).expect("the workbook part is utf-8");
    let mut block = String::from("<definedNames>");
    for (name, scope, definition) in names {
        block.push_str("<definedName name=\"");
        block.push_str(name);
        block.push('"');
        if let Some(index) = scope {
            block.push_str(&format!(" localSheetId=\"{index}\""));
        }
        block.push('>');
        block.push_str(definition);
        block.push_str("</definedName>");
    }
    block.push_str("</definedNames>");
    let patched = text.replace("</sheets>", &format!("</sheets>{block}"));
    assert_ne!(patched, text, "the anchor `</sheets>` was not found");
    package
        .replace_part_bytes(&part, patched.into_bytes())
        .expect("the package accepts the replacement");
    package.save().expect("the package saves")
}

#[test]
fn a_defined_name_is_followed_to_the_cells_it_stands_for() {
    let bytes = workbook_with_defined_names(&[
        ("Revenue", None, "Data!$B$2:$B$4"),
        // A name defined in terms of another, which Excel allows and which the resolver follows.
        ("RevenueAgain", None, "Revenue"),
        // A *sheet-scoped* name that shadows the workbook-scoped one, which is the resolution order
        // §18.2.6 states and which a resolver that only looked at global names would get wrong.
        ("Revenue", Some(0), "Data!$B$2:$B$2"),
    ]);
    let mut workbook = Workbook::open(&bytes).expect("opens");

    let sheet_scoped = workbook
        .resolve_range_reference(0, "Revenue")
        .expect("resolves");
    assert!(
        sheet_scoped.is_fully_resolved(),
        "{:?}",
        sheet_scoped.problem()
    );
    assert_eq!(
        sheet_scoped
            .cells
            .iter()
            .filter_map(|cell| cell.value.number())
            .collect::<Vec<_>>(),
        [10.0],
        "the sheet-scoped name must win over the workbook-scoped one of the same name"
    );

    let chained = workbook
        .resolve_range_reference(0, "RevenueAgain")
        .expect("resolves");
    assert!(chained.is_fully_resolved(), "{:?}", chained.problem());
    assert_eq!(
        chained
            .cells
            .iter()
            .filter_map(|cell| cell.value.number())
            .collect::<Vec<_>>(),
        [10.0],
        "a name defined in terms of another is followed through to the cells"
    );
}

#[test]
fn every_unresolvable_reference_is_a_typed_report_rather_than_a_panic_or_a_guess() {
    // A `c:f` is untrusted input. None of these may panic, none may be silently answered with the
    // wrong cells, and each must say *which* thing it could not do.
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let mut problem = |text: &str| {
        workbook
            .resolve_range_reference(0, text)
            .expect("resolving never fails on the reference itself")
            .problem()
            .cloned()
            .unwrap_or_else(|| panic!("{text} resolved when it should not have"))
    };

    assert!(matches!(
        problem("Ghost!$A$1"),
        RangeProblem::UnknownSheet { ref name } if name == "Ghost"
    ));
    assert!(matches!(
        problem("Data:Ghost!$A$1"),
        RangeProblem::UnknownSpanEnd { ref name } if name == "Ghost"
    ));
    assert!(matches!(
        problem("[1]Data!$A$1"),
        RangeProblem::ExternalBook { index: 1 }
    ));
    assert!(matches!(
        problem("NoSuchName"),
        RangeProblem::UnknownDefinedName { ref name } if name == "NoSuchName"
    ));
    assert!(matches!(problem("Data!$A$"), RangeProblem::Malformed(_)));
    assert!(matches!(problem("(Data!$A$1"), RangeProblem::Malformed(_)));
    assert!(matches!(problem(""), RangeProblem::Malformed(_)));
}

#[test]
fn a_defined_name_that_reaches_itself_is_refused_rather_than_followed_forever() {
    // Two shapes, because neither guard alone refuses both: a name that *is* itself, and a pair that
    // point at each other. A file can carry either, and following one would not return.
    let bytes = workbook_with_defined_names(&[
        ("Loop", None, "Loop"),
        ("Ping", None, "Pong"),
        ("Pong", None, "Ping"),
    ]);
    let mut workbook = Workbook::open(&bytes).expect("opens");
    for name in ["Loop", "Ping"] {
        let resolved = workbook.resolve_range_reference(0, name).expect("resolves");
        assert!(
            matches!(
                resolved.problem(),
                Some(RangeProblem::DefinedNameCycle { .. })
            ),
            "{name} reported {:?}",
            resolved.problem()
        );
    }
}

#[test]
fn a_reference_naming_a_sheet_with_no_cells_says_so_rather_than_answering_nothing() {
    // A dialogsheet — like a chartsheet — has no `sheetData` at all. "No cells here" and "no cells
    // matched" are different answers, and a caller comparing a chart against a sheet needs to be
    // told which it got.
    let mut workbook =
        Workbook::open(&mjx_fixtures::fixture("print_and_sheet_kinds.xlsx")).expect("opens");
    let cell_less = workbook
        .sheets()
        .iter()
        .position(|sheet| sheet.kind == Some(mjx_xlsx::SheetKind::Dialogsheet))
        .expect("the fixture carries a dialogsheet");
    let name = workbook.sheets()[cell_less].name.clone();
    let resolved = workbook
        .resolve_range_reference(0, &format!("{name}!$A$1:$A$3"))
        .expect("resolves");
    assert!(matches!(
        resolved.problem(),
        Some(RangeProblem::SheetHasNoCells { .. })
    ));
    assert!(resolved.cells.is_empty());
}

#[test]
fn a_series_whose_values_are_a_literal_says_it_cannot_be_compared_rather_than_that_it_disagrees() {
    // The third answer. `values_agree: None` means *cannot say* — there are no cells behind this
    // series at all — and a caller acting on "the cells disagree" must not be told it by "there are
    // no cells".
    let mut workbook = Workbook::open(&producer_workbook()).expect("opens");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["A", "B"])
        .series("Literal", [1.0, 2.0]);
    let anchor = workbook
        .add_chart(
            0,
            &chart,
            CellMarker::new(0, 0, 10, 0),
            CellMarker::new(5, 0, 25, 0),
            "Literal",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a chart");
    // Its `c:f` names the embedded workbook, which is a package of its own rather than a sheet of
    // this one — so the reference is present but names nothing here.
    let freshness = workbook
        .chart_series_freshness(0, anchor)
        .expect("freshness");
    assert!(freshness[0].references.values.is_some());
    assert_eq!(freshness[0].values_agree, None);
    assert!(matches!(
        freshness[0].values_problem,
        Some(RangeProblem::UnknownSheet { .. })
    ));
    // …and the opt-in repair leaves such a series exactly as it was.
    assert_eq!(
        workbook
            .refresh_chart_cache_from_cells(0, anchor)
            .expect("refresh"),
        0
    );
    assert_eq!(
        workbook.chart_series(0, anchor).expect("series")[0].values,
        [1.0, 2.0]
    );
}
