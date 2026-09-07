//! MJXOFF-103 (E2) and MJXOFF-111 (E4) — "Done when" #1, in Rust: **the same chart vocabulary
//! reads, authors and edits from a presentation, a document and a workbook, under the same method
//! names.**
//!
//! The facade is the only crate that may hold a `Deck`, a `Document` and a `Workbook` at once —
//! `mjx-pptx`, `mjx-docx` and `mjx-xlsx` are all rank 3.0, so none may reach another — which makes
//! this file the only place the claim can be *checked* rather than asserted in prose. MJXOFF-111
//! added the third surface here rather than in a file of its own, because a parity claim written
//! twice is a parity claim that can disagree with itself.
//!
//! Two separate things are checked, and the second is the one that would have been easy to skip:
//!
//! 1. **The names exist on both surfaces**, with matching argument order after the address. That is
//!    a compile-time fact, and every call below is its proof.
//! 2. **They answer the same thing about the same bytes.** A `Deck` and a `Document` are given
//!    byte-identical chart parts, and every reader is compared pairwise. This is what would catch a
//!    Word method wired to the wrong `mjx_chart::chart_ops` function — a mis-wiring that "the method
//!    exists" cannot see, and that the two crates' own test suites cannot see either, because each
//!    only ever compares a surface against itself.

use mjx_ooxml::{
    AxisOrientation, ChartData, ChartKind, ChartLabelScope, DataLabelSpec, Deck, Document,
    ErrorCode, FillSpec, LegendPosition, PageSize, ResizingBehavior, ShapeBounds, SlideSize,
    Surface, Workbook,
};

/// The chart both surfaces are given.
///
/// It carries a **title**, and the cases below give its value axis a *different* title, for a
/// reason a mutation found: comparing two surfaces on a chart whose title is `None` makes the
/// `chart_title` comparison vacuous — both answer `None` whether or not the delegate is wired to
/// the right thing. Wiring `Document::chart_title` to the axis title instead left this whole file
/// green until the two strings were made distinct.
fn chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("North", [12.5, 18.0, 21.5])
        .series("South", [9.0, 11.5, 14.0])
        .title("Quarterly revenue".to_owned())
}

/// A deck with one chart, and that chart's shape index.
fn deck_with_a_chart() -> (Deck, u32) {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    deck.add_slide().expect("a slide");
    let shape = deck
        .add_chart(
            Surface::Slide(0),
            &chart(),
            ShapeBounds::from_inches(1.0, 1.0, 5.0, 3.0),
        )
        .expect("a chart on the slide");
    (deck, shape)
}

/// A document with one chart, and that chart's drawing id.
fn document_with_a_chart() -> (Document, u32) {
    let mut document = Document::blank(PageSize::a4()).expect("a blank document");
    let drawing = document
        .add_chart(0.into(), &chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("a chart in the document");
    (document, drawing)
}

/// A workbook with one chart, and that chart's `(sheet, anchor)` address.
///
/// The address is two numbers where the other two surfaces take one, and that is the *only*
/// difference: a worksheet has no shape tree and no `wp:docPr` id, so a chart is addressed by the
/// tab and by the anchor's position in that sheet's drawing part — the address MJXOFF-107 already
/// gave every anchored object.
fn workbook_with_a_chart() -> (Workbook, u32, u32) {
    let mut workbook = Workbook::blank().expect("a blank workbook");
    let anchor = workbook
        .add_chart(
            0,
            &chart(),
            1,
            1,
            7,
            16,
            "Revenue",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a chart on the sheet");
    (workbook, 0, anchor)
}

#[test]
fn all_three_surfaces_author_a_chart_that_reads_back_identically() {
    let (mut deck, shape) = deck_with_a_chart();
    let (mut document, drawing) = document_with_a_chart();
    let (mut workbook, sheet, anchor) = workbook_with_a_chart();

    // Give the value axis a title that is *not* the chart's, so that a reader wired to the wrong
    // one of the two answers a different string rather than the same `None`.
    deck.set_chart_axis_title(Surface::Slide(0), shape.into(), 1, Some("Millions"))
        .unwrap();
    document
        .set_chart_axis_title(drawing, 1, Some("Millions"))
        .unwrap();
    workbook
        .set_chart_axis_title(sheet, anchor, 1, Some("Millions"))
        .unwrap();
    assert_eq!(
        document.chart_title(drawing).unwrap().as_deref(),
        Some("Quarterly revenue"),
        "the chart's own title, not the axis'"
    );
    assert_eq!(
        document.chart_axes(drawing).unwrap()[1].title.as_deref(),
        Some("Millions"),
        "and the axis' own, not the chart's"
    );

    // The two packages differ everywhere else — one is a deck, the other a document — but the chart
    // description they were given is the same, so every chart reader must answer the same thing.
    assert_eq!(
        deck.chart_series(Surface::Slide(0), shape.into()).unwrap(),
        document.chart_series(drawing).unwrap(),
        "chart_series"
    );
    assert_eq!(
        deck.chart_kinds(Surface::Slide(0), shape.into()).unwrap(),
        document.chart_kinds(drawing).unwrap(),
        "chart_kinds"
    );
    assert_eq!(
        deck.chart_axes(Surface::Slide(0), shape.into()).unwrap(),
        document.chart_axes(drawing).unwrap(),
        "chart_axes"
    );
    assert_eq!(
        deck.chart_title(Surface::Slide(0), shape.into()).unwrap(),
        document.chart_title(drawing).unwrap(),
        "chart_title"
    );
    assert_eq!(
        deck.chart_legend(Surface::Slide(0), shape.into()).unwrap(),
        document.chart_legend(drawing).unwrap(),
        "chart_legend"
    );
    assert_eq!(
        deck.chart_style_id(Surface::Slide(0), shape.into())
            .unwrap(),
        document.chart_style_id(drawing).unwrap(),
        "chart_style_id"
    );
    assert_eq!(
        deck.chart_series_fill(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.chart_series_fill(drawing, 0).unwrap(),
        "chart_series_fill"
    );
    assert_eq!(
        deck.chart_point_formats(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.chart_point_formats(drawing, 0).unwrap(),
        "chart_point_formats"
    );
    assert_eq!(
        deck.chart_trendlines(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.chart_trendlines(drawing, 0).unwrap(),
        "chart_trendlines"
    );
    assert_eq!(
        deck.chart_error_bars(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.chart_error_bars(drawing, 0).unwrap(),
        "chart_error_bars"
    );
    assert_eq!(
        deck.chart_data_labels(Surface::Slide(0), shape.into(), 0, None)
            .unwrap(),
        document.chart_data_labels(drawing, 0, None).unwrap(),
        "chart_data_labels"
    );
    assert_eq!(
        deck.chart_dangling_decoration(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.chart_dangling_decoration(drawing, 0).unwrap(),
        "chart_dangling_decoration"
    );

    // …and the third surface answers the same thing about the same bytes, method for method. Every
    // comparison is against the *document*, so a triple that agreed only because two of them share
    // an implementation would still have to explain the third.
    assert_eq!(
        workbook.chart_series(sheet, anchor).unwrap(),
        document.chart_series(drawing).unwrap(),
        "chart_series"
    );
    assert_eq!(
        workbook.chart_kinds(sheet, anchor).unwrap(),
        document.chart_kinds(drawing).unwrap(),
        "chart_kinds"
    );
    assert_eq!(
        workbook.chart_axes(sheet, anchor).unwrap(),
        document.chart_axes(drawing).unwrap(),
        "chart_axes"
    );
    assert_eq!(
        workbook.chart_title(sheet, anchor).unwrap(),
        document.chart_title(drawing).unwrap(),
        "chart_title"
    );
    assert_eq!(
        workbook.chart_legend(sheet, anchor).unwrap(),
        document.chart_legend(drawing).unwrap(),
        "chart_legend"
    );
    assert_eq!(
        workbook.chart_style_id(sheet, anchor).unwrap(),
        document.chart_style_id(drawing).unwrap(),
        "chart_style_id"
    );
    assert_eq!(
        workbook.chart_series_fill(sheet, anchor, 0).unwrap(),
        document.chart_series_fill(drawing, 0).unwrap(),
        "chart_series_fill"
    );
    assert_eq!(
        workbook.chart_point_formats(sheet, anchor, 0).unwrap(),
        document.chart_point_formats(drawing, 0).unwrap(),
        "chart_point_formats"
    );
    assert_eq!(
        workbook.chart_trendlines(sheet, anchor, 0).unwrap(),
        document.chart_trendlines(drawing, 0).unwrap(),
        "chart_trendlines"
    );
    assert_eq!(
        workbook.chart_error_bars(sheet, anchor, 0).unwrap(),
        document.chart_error_bars(drawing, 0).unwrap(),
        "chart_error_bars"
    );
    assert_eq!(
        workbook.chart_data_labels(sheet, anchor, 0, None).unwrap(),
        document.chart_data_labels(drawing, 0, None).unwrap(),
        "chart_data_labels"
    );
    assert_eq!(
        workbook
            .chart_dangling_decoration(sheet, anchor, 0)
            .unwrap(),
        document.chart_dangling_decoration(drawing, 0).unwrap(),
        "chart_dangling_decoration"
    );
    assert_eq!(
        workbook.chart_series_references(sheet, anchor).unwrap(),
        document.chart_series_references(drawing).unwrap(),
        "chart_series_references"
    );
}

#[test]
fn the_same_sequence_of_edits_leaves_all_three_charts_saying_the_same_thing() {
    let (mut deck, shape) = deck_with_a_chart();
    let (mut document, drawing) = document_with_a_chart();
    let (mut workbook, sheet, anchor) = workbook_with_a_chart();

    // Every edit applied to all three, in the same order, with the same arguments. A setter wired
    // to a different `chart_ops` function than its namesakes diverges here even though every call
    // succeeds.
    deck.set_chart_title(Surface::Slide(0), shape.into(), Some("Regional revenue"))
        .unwrap();
    document
        .set_chart_title(drawing, Some("Regional revenue"))
        .unwrap();
    workbook
        .set_chart_title(sheet, anchor, Some("Regional revenue"))
        .unwrap();

    deck.set_chart_legend(Surface::Slide(0), shape.into(), Some(LegendPosition::Right))
        .unwrap();
    document
        .set_chart_legend(drawing, Some(LegendPosition::Right))
        .unwrap();
    workbook
        .set_chart_legend(sheet, anchor, Some(LegendPosition::Right))
        .unwrap();

    deck.set_chart_series_values(Surface::Slide(0), shape.into(), 0, &[40.0, 41.0, 42.0])
        .unwrap();
    document
        .set_chart_series_values(drawing, 0, &[40.0, 41.0, 42.0])
        .unwrap();
    workbook
        .set_chart_series_values(sheet, anchor, 0, &[40.0, 41.0, 42.0])
        .unwrap();

    deck.set_chart_series_categories(Surface::Slide(0), shape.into(), 1, &["A", "B", "C"])
        .unwrap();
    document
        .set_chart_series_categories(drawing, 1, &["A", "B", "C"])
        .unwrap();
    workbook
        .set_chart_series_categories(sheet, anchor, 1, &["A", "B", "C"])
        .unwrap();

    deck.set_chart_axis_scale(Surface::Slide(0), shape.into(), 1, Some(0.0), Some(50.0))
        .unwrap();
    document
        .set_chart_axis_scale(drawing, 1, Some(0.0), Some(50.0))
        .unwrap();
    workbook
        .set_chart_axis_scale(sheet, anchor, 1, Some(0.0), Some(50.0))
        .unwrap();

    deck.set_chart_axis_title(Surface::Slide(0), shape.into(), 1, Some("Millions"))
        .unwrap();
    document
        .set_chart_axis_title(drawing, 1, Some("Millions"))
        .unwrap();
    workbook
        .set_chart_axis_title(sheet, anchor, 1, Some("Millions"))
        .unwrap();

    deck.set_chart_axis_gridlines(Surface::Slide(0), shape.into(), 1, true, false)
        .unwrap();
    document
        .set_chart_axis_gridlines(drawing, 1, true, false)
        .unwrap();
    workbook
        .set_chart_axis_gridlines(sheet, anchor, 1, true, false)
        .unwrap();

    deck.set_chart_axis_orientation(
        Surface::Slide(0),
        shape.into(),
        1,
        AxisOrientation::MaximumToMinimum,
    )
    .unwrap();
    document
        .set_chart_axis_orientation(drawing, 1, AxisOrientation::MaximumToMinimum)
        .unwrap();
    workbook
        .set_chart_axis_orientation(sheet, anchor, 1, AxisOrientation::MaximumToMinimum)
        .unwrap();

    let fill = FillSpec::Solid(mjx_ooxml::ColorSpec::Srgb("1F77B4".to_owned()));
    deck.set_chart_series_fill(Surface::Slide(0), shape.into(), 0, &fill)
        .unwrap();
    document.set_chart_series_fill(drawing, 0, &fill).unwrap();
    workbook
        .set_chart_series_fill(sheet, anchor, 0, &fill)
        .unwrap();

    let labels = DataLabelSpec::new().value(true).category_name(true);
    deck.set_chart_data_labels(
        Surface::Slide(0),
        shape.into(),
        ChartLabelScope::Series { series_index: 0 },
        &labels,
    )
    .unwrap();
    document
        .set_chart_data_labels(
            drawing,
            ChartLabelScope::Series { series_index: 0 },
            &labels,
        )
        .unwrap();
    workbook
        .set_chart_data_labels(
            sheet,
            anchor,
            ChartLabelScope::Series { series_index: 0 },
            &labels,
        )
        .unwrap();

    deck.set_chart_point_explosion(Surface::Slide(0), shape.into(), 0, 1, Some(25))
        .unwrap();
    document
        .set_chart_point_explosion(drawing, 0, 1, Some(25))
        .unwrap();
    workbook
        .set_chart_point_explosion(sheet, anchor, 0, 1, Some(25))
        .unwrap();

    let trendline = mjx_ooxml::TrendlineSpec::new(mjx_ooxml::TrendlineKind::Linear);
    deck.add_chart_trendline(Surface::Slide(0), shape.into(), 0, &trendline)
        .unwrap();
    document
        .add_chart_trendline(drawing, 0, &trendline)
        .unwrap();
    workbook
        .add_chart_trendline(sheet, anchor, 0, &trendline)
        .unwrap();

    let bars = mjx_ooxml::ErrorBarSpec::fixed(
        mjx_ooxml::ErrorBarType::Both,
        mjx_ooxml::ErrorValueType::FixedValue,
        1.5,
    );
    deck.set_chart_error_bars(Surface::Slide(0), shape.into(), 0, &bars)
        .unwrap();
    document.set_chart_error_bars(drawing, 0, &bars).unwrap();
    workbook
        .set_chart_error_bars(sheet, anchor, 0, &bars)
        .unwrap();

    // Now every reader must still agree — which is the assertion the edits above exist to make
    // meaningful. Comparing the chart *parts* byte-for-byte is stronger still: all three are
    // authored by the same writer from the same description and edited by the same operations, so
    // the only thing that could differ is the relationship id each names, which is `rId1` on all
    // three.
    //
    // **The comparison is made after saving, and MJXOFF-111 had to change it to be.** It read
    // `chart_part_bytes` on the live surfaces, and `mjx_opc::Package::part_bytes` answers `None`
    // for a part whose body is `Edited` — which every chart here is, twelve edits in. So the
    // assertion was `None == None` from the day it was written: a chart part wired to the wrong
    // bytes would have satisfied it. The `is_some` check below is what stops it becoming vacuous
    // again.
    let saved_chart_part = |bytes: &[u8], part: &str| -> Option<String> {
        let package = mjx_opc::Package::open(bytes).expect("the saved package opens");
        let name = mjx_opc::PartName::new(part).expect("a valid part name");
        package
            .part_bytes(&name)
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
    };
    let from_deck = saved_chart_part(&deck.save().unwrap(), "/ppt/charts/chart1.xml");
    let from_document = saved_chart_part(&document.save().unwrap(), "/word/charts/chart1.xml");
    let from_workbook = saved_chart_part(&workbook.save().unwrap(), "/xl/charts/chart1.xml");
    assert!(
        from_deck.is_some() && from_document.is_some() && from_workbook.is_some(),
        "each package must really carry a chart part — a comparison of three `None`s proves nothing"
    );
    assert_eq!(
        from_deck, from_document,
        "one description, one sequence of edits, two surfaces — one chart part"
    );
    assert_eq!(
        from_workbook, from_document,
        "…and the third surface's chart part is the same bytes again"
    );

    assert_eq!(
        deck.chart_series(Surface::Slide(0), shape.into()).unwrap(),
        document.chart_series(drawing).unwrap()
    );
    assert_eq!(
        deck.chart_axes(Surface::Slide(0), shape.into()).unwrap(),
        document.chart_axes(drawing).unwrap()
    );
    assert_eq!(
        deck.chart_trendlines(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.chart_trendlines(drawing, 0).unwrap()
    );
    assert_eq!(
        deck.chart_error_bars(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.chart_error_bars(drawing, 0).unwrap()
    );

    // And the removals answer the same counts.
    assert_eq!(
        deck.remove_chart_trendlines(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.remove_chart_trendlines(drawing, 0).unwrap()
    );
    assert_eq!(
        deck.remove_chart_error_bars(Surface::Slide(0), shape.into(), 0)
            .unwrap(),
        document.remove_chart_error_bars(drawing, 0).unwrap()
    );
    assert_eq!(
        deck.remove_chart_point_format(Surface::Slide(0), shape.into(), 0, 1)
            .unwrap(),
        document.remove_chart_point_format(drawing, 0, 1).unwrap()
    );
    assert_eq!(
        deck.suppress_chart_data_labels(
            Surface::Slide(0),
            shape.into(),
            ChartLabelScope::Series { series_index: 1 }
        )
        .is_ok(),
        document
            .suppress_chart_data_labels(drawing, ChartLabelScope::Series { series_index: 1 })
            .is_ok()
    );
    assert_eq!(
        deck.remove_chart_data_labels(
            Surface::Slide(0),
            shape.into(),
            ChartLabelScope::Series { series_index: 0 }
        )
        .unwrap(),
        document
            .remove_chart_data_labels(drawing, ChartLabelScope::Series { series_index: 0 })
            .unwrap()
    );
}

#[test]
fn the_same_refusal_answers_the_same_error_code_on_all_three_surfaces() {
    // A9's contract, across the three surfaces: `mjx-pptx` reaches these verdicts through its own
    // pre-existing `PptxError` variants, `mjx-docx` and `mjx-xlsx` through a `ChartAccess` variant
    // wrapping `mjx-chart`'s own enum, so nothing but this test says the three collapse to the
    // *same* stable code. A binding caller switching between a deck, a document and a workbook must
    // not have to learn three error vocabularies.
    let (mut deck, shape) = deck_with_a_chart();
    let (mut document, drawing) = document_with_a_chart();
    let (mut workbook, sheet, anchor) = workbook_with_a_chart();

    let from_deck = deck
        .chart_series_fill(Surface::Slide(0), shape.into(), 7)
        .expect_err("series 7 is past the end");
    let from_document = document
        .chart_series_fill(drawing, 7)
        .expect_err("series 7 is past the end");
    assert_eq!(from_deck.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(
        from_deck.code(),
        from_document.code(),
        "series out of range"
    );
    let from_workbook = workbook
        .chart_series_fill(sheet, anchor, 7)
        .expect_err("series 7 is past the end");
    assert_eq!(
        from_deck.detail().index,
        from_document.detail().index,
        "and both name which index"
    );
    assert_eq!(
        from_deck.code(),
        from_workbook.code(),
        "series out of range, from a workbook"
    );
    assert_eq!(
        from_deck.detail().index,
        from_workbook.detail().index,
        "and it names which index too"
    );

    let from_deck = deck
        .set_chart_axis_scale(Surface::Slide(0), shape.into(), 9, Some(0.0), None)
        .expect_err("axis 9 is past the end");
    let from_document = document
        .set_chart_axis_scale(drawing, 9, Some(0.0), None)
        .expect_err("axis 9 is past the end");
    let from_workbook = workbook
        .set_chart_axis_scale(sheet, anchor, 9, Some(0.0), None)
        .expect_err("axis 9 is past the end");
    assert_eq!(from_deck.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(from_deck.code(), from_document.code(), "axis out of range");
    assert_eq!(
        from_deck.code(),
        from_workbook.code(),
        "axis out of range, from a workbook"
    );

    let picture_fill = FillSpec::Picture {
        rel_id: "rId9".to_owned(),
        mode: mjx_ooxml::PictureFillMode::Stretch,
    };
    let from_deck = deck
        .set_chart_series_fill(Surface::Slide(0), shape.into(), 0, &picture_fill)
        .expect_err("a chart part relates to no images");
    let from_document = document
        .set_chart_series_fill(drawing, 0, &picture_fill)
        .expect_err("a chart part relates to no images");
    let from_workbook = workbook
        .set_chart_series_fill(sheet, anchor, 0, &picture_fill)
        .expect_err("a chart part relates to no images");
    assert_eq!(from_deck.code(), ErrorCode::UnsupportedContent);
    assert_eq!(from_deck.code(), from_document.code(), "image fill");
    assert_eq!(
        from_deck.code(),
        from_workbook.code(),
        "image fill, from a workbook"
    );

    let from_deck = deck
        .add_chart(
            Surface::Slide(0),
            &ChartData::new(ChartKind::Bar),
            ShapeBounds::from_inches(1.0, 1.0, 2.0, 2.0),
        )
        .expect_err("an empty chart is refused");
    let from_document = document
        .add_chart(
            0.into(),
            &ChartData::new(ChartKind::Bar),
            914_400,
            914_400,
            "Empty",
        )
        .expect_err("an empty chart is refused");
    let from_workbook = workbook
        .add_chart(
            sheet,
            &ChartData::new(ChartKind::Bar),
            1,
            1,
            3,
            3,
            "Empty",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect_err("an empty chart is refused");
    assert_eq!(from_deck.code(), ErrorCode::InvalidArgument);
    assert_eq!(from_deck.code(), from_document.code(), "nothing to draw");
    assert_eq!(
        from_deck.code(),
        from_workbook.code(),
        "nothing to draw, from a workbook"
    );
}

#[test]
fn only_the_workbook_surface_has_a_chart_whose_data_is_a_live_range() {
    // The one place the three surfaces deliberately do **not** agree, stated as a test so that it is
    // a decision rather than an omission. A slide and a document keep a chart's data in an embedded
    // workbook; a worksheet can point at its own cells, and only a worksheet has cells to point at.
    let (mut deck, shape) = deck_with_a_chart();
    let (mut document, drawing) = document_with_a_chart();
    let mut workbook = Workbook::blank().expect("a blank workbook");
    workbook
        .write_cells(
            0,
            &[
                mjx_ooxml::CellWrite::new("A1", mjx_ooxml::CellInput::Number(10.0)),
                mjx_ooxml::CellWrite::new("A2", mjx_ooxml::CellInput::Number(20.0)),
                mjx_ooxml::CellWrite::new("A3", mjx_ooxml::CellInput::Number(30.0)),
            ],
        )
        .expect("three cells");

    // Every chart the other two surfaces can author carries a workbook, and `refresh` patches it in
    // place (MJXOFF-208). `regenerate` is the explicit opt-in that replaces it instead, and all
    // three surfaces carry both — a binding that projects one of the pair would be a surface two
    // languages could only half use.
    assert!(deck
        .refresh_chart_workbook(Surface::Slide(0), shape.into())
        .unwrap());
    assert!(document.refresh_chart_workbook(drawing).unwrap());
    assert!(deck
        .regenerate_chart_workbook(Surface::Slide(0), shape.into())
        .unwrap());
    assert!(document.regenerate_chart_workbook(drawing).unwrap());

    // A chart over a live range has none, and `refresh` says so rather than making one — the same
    // method, the same `false` a document answers for an external link, on a chart that is
    // *ordinary* here rather than exceptional.
    let live = workbook
        .add_range_chart(
            0,
            ChartKind::Line,
            None,
            &[mjx_ooxml::ChartRangeSeries::new(
                "Revenue",
                "Sheet1!$A$1:$A$3",
            )],
            2,
            1,
            8,
            16,
            "Live",
            ResizingBehavior::MoveWithCellsButDoNotResize,
        )
        .expect("a live-range chart");
    assert!(
        !workbook.refresh_chart_workbook(0, live).unwrap(),
        "a live-range chart has no workbook to refresh, and none is fabricated"
    );
    assert!(workbook.chart_workbooks().unwrap().is_empty());

    // Its caches were seeded from the cells, and the freshness report says so.
    assert_eq!(
        workbook.chart_series(0, live).unwrap()[0].values,
        [10.0, 20.0, 30.0]
    );
    let freshness = workbook.chart_series_freshness(0, live).unwrap();
    assert_eq!(freshness[0].values_agree, Some(true));
    assert_eq!(
        freshness[0].references.values.as_deref(),
        Some("Sheet1!$A$1:$A$3")
    );

    // …and once a cell moves, the cache is stale and is reported stale rather than rewritten.
    workbook
        .write_cells(
            0,
            &[mjx_ooxml::CellWrite::new(
                "A2",
                mjx_ooxml::CellInput::Number(999.0),
            )],
        )
        .expect("one cell");
    assert_eq!(
        workbook.chart_series(0, live).unwrap()[0].values,
        [10.0, 20.0, 30.0],
        "writing a cell must not touch a chart"
    );
    let freshness = workbook.chart_series_freshness(0, live).unwrap();
    assert_eq!(freshness[0].values_agree, Some(false));
    assert_eq!(freshness[0].from_cells.values, [10.0, 999.0, 30.0]);

    // The opt-in repair is the counterpart of `refresh_chart_workbook`, pointing the other way.
    assert_eq!(workbook.refresh_chart_cache_from_cells(0, live).unwrap(), 1);
    assert_eq!(
        workbook.chart_series(0, live).unwrap()[0].values,
        [10.0, 999.0, 30.0]
    );
}
