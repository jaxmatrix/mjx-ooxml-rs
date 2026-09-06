//! MJXOFF-103 (E2) — "Done when" #1, in Rust: **the same chart vocabulary reads, authors and edits
//! from a presentation and a document, under the same method names.**
//!
//! The facade is the only crate that may hold a `Deck` and a `Document` at once — `mjx-pptx` and
//! `mjx-docx` are both rank 3.0, so neither may reach the other — which makes this file the only
//! place the claim can be *checked* rather than asserted in prose.
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
    ErrorCode, FillSpec, LegendPosition, PageSize, ShapeBounds, SlideSize, Surface,
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

#[test]
fn both_surfaces_author_a_chart_that_reads_back_identically() {
    let (mut deck, shape) = deck_with_a_chart();
    let (mut document, drawing) = document_with_a_chart();

    // Give the value axis a title that is *not* the chart's, so that a reader wired to the wrong
    // one of the two answers a different string rather than the same `None`.
    deck.set_chart_axis_title(Surface::Slide(0), shape.into(), 1, Some("Millions"))
        .unwrap();
    document
        .set_chart_axis_title(drawing, 1, Some("Millions"))
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
}

#[test]
fn the_same_sequence_of_edits_leaves_both_charts_saying_the_same_thing() {
    let (mut deck, shape) = deck_with_a_chart();
    let (mut document, drawing) = document_with_a_chart();

    // Every edit applied to both, in the same order, with the same arguments. A Word setter wired
    // to a different `chart_ops` function than its PowerPoint namesake diverges here even though
    // both calls succeed.
    deck.set_chart_title(Surface::Slide(0), shape.into(), Some("Regional revenue"))
        .unwrap();
    document
        .set_chart_title(drawing, Some("Regional revenue"))
        .unwrap();

    deck.set_chart_legend(Surface::Slide(0), shape.into(), Some(LegendPosition::Right))
        .unwrap();
    document
        .set_chart_legend(drawing, Some(LegendPosition::Right))
        .unwrap();

    deck.set_chart_series_values(Surface::Slide(0), shape.into(), 0, &[40.0, 41.0, 42.0])
        .unwrap();
    document
        .set_chart_series_values(drawing, 0, &[40.0, 41.0, 42.0])
        .unwrap();

    deck.set_chart_series_categories(Surface::Slide(0), shape.into(), 1, &["A", "B", "C"])
        .unwrap();
    document
        .set_chart_series_categories(drawing, 1, &["A", "B", "C"])
        .unwrap();

    deck.set_chart_axis_scale(Surface::Slide(0), shape.into(), 1, Some(0.0), Some(50.0))
        .unwrap();
    document
        .set_chart_axis_scale(drawing, 1, Some(0.0), Some(50.0))
        .unwrap();

    deck.set_chart_axis_title(Surface::Slide(0), shape.into(), 1, Some("Millions"))
        .unwrap();
    document
        .set_chart_axis_title(drawing, 1, Some("Millions"))
        .unwrap();

    deck.set_chart_axis_gridlines(Surface::Slide(0), shape.into(), 1, true, false)
        .unwrap();
    document
        .set_chart_axis_gridlines(drawing, 1, true, false)
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

    let fill = FillSpec::Solid(mjx_ooxml::ColorSpec::Srgb("1F77B4".to_owned()));
    deck.set_chart_series_fill(Surface::Slide(0), shape.into(), 0, &fill)
        .unwrap();
    document.set_chart_series_fill(drawing, 0, &fill).unwrap();

    let labels = DataLabelSpec::new().value(true).category_name(true);
    deck.set_chart_data_labels(
        Surface::Slide(0),
        shape.into(),
        ChartLabelScope::Series { series_idx: 0 },
        &labels,
    )
    .unwrap();
    document
        .set_chart_data_labels(drawing, ChartLabelScope::Series { series_idx: 0 }, &labels)
        .unwrap();

    deck.set_chart_point_explosion(Surface::Slide(0), shape.into(), 0, 1, Some(25))
        .unwrap();
    document
        .set_chart_point_explosion(drawing, 0, 1, Some(25))
        .unwrap();

    let trendline = mjx_ooxml::TrendlineSpec::new(mjx_ooxml::TrendlineKind::Linear);
    deck.add_chart_trendline(Surface::Slide(0), shape.into(), 0, &trendline)
        .unwrap();
    document
        .add_chart_trendline(drawing, 0, &trendline)
        .unwrap();

    let bars = mjx_ooxml::ErrorBarSpec::fixed(
        mjx_ooxml::ErrorBarType::Both,
        mjx_ooxml::ErrorValueType::FixedValue,
        1.5,
    );
    deck.set_chart_error_bars(Surface::Slide(0), shape.into(), 0, &bars)
        .unwrap();
    document.set_chart_error_bars(drawing, 0, &bars).unwrap();

    // Now every reader must still agree — which is the assertion the edits above exist to make
    // meaningful. Comparing the chart *parts* byte-for-byte would be even stronger, and it is: the
    // two are authored by the same writer from the same description and edited by the same
    // operations, so the only thing that could differ is the relationship id each names, which is
    // `rId1` on both.
    assert_eq!(
        deck.chart_part_bytes(Surface::Slide(0), shape.into())
            .unwrap()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        document
            .chart_part_bytes(drawing)
            .unwrap()
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        "one description, one sequence of edits, two surfaces — one chart part"
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
            ChartLabelScope::Series { series_idx: 1 }
        )
        .is_ok(),
        document
            .suppress_chart_data_labels(drawing, ChartLabelScope::Series { series_idx: 1 })
            .is_ok()
    );
    assert_eq!(
        deck.remove_chart_data_labels(
            Surface::Slide(0),
            shape.into(),
            ChartLabelScope::Series { series_idx: 0 }
        )
        .unwrap(),
        document
            .remove_chart_data_labels(drawing, ChartLabelScope::Series { series_idx: 0 })
            .unwrap()
    );
}

#[test]
fn the_same_refusal_answers_the_same_error_code_on_both_surfaces() {
    // A9's contract, across the two surfaces: `mjx-pptx` reaches these verdicts through its own
    // pre-existing `PptxError` variants and `mjx-docx` through `DocxError::ChartAccess`, so nothing
    // but this test says the two collapse to the *same* stable code. A binding caller switching
    // between a deck and a document must not have to learn two error vocabularies.
    let (mut deck, shape) = deck_with_a_chart();
    let (mut document, drawing) = document_with_a_chart();

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
    assert_eq!(
        from_deck.detail().index,
        from_document.detail().index,
        "and both name which index"
    );

    let from_deck = deck
        .set_chart_axis_scale(Surface::Slide(0), shape.into(), 9, Some(0.0), None)
        .expect_err("axis 9 is past the end");
    let from_document = document
        .set_chart_axis_scale(drawing, 9, Some(0.0), None)
        .expect_err("axis 9 is past the end");
    assert_eq!(from_deck.code(), ErrorCode::IndexOutOfRange);
    assert_eq!(from_deck.code(), from_document.code(), "axis out of range");

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
    assert_eq!(from_deck.code(), ErrorCode::UnsupportedContent);
    assert_eq!(from_deck.code(), from_document.code(), "image fill");

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
    assert_eq!(from_deck.code(), ErrorCode::InvalidArgument);
    assert_eq!(from_deck.code(), from_document.code(), "nothing to draw");
}
