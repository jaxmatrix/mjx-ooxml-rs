//! MJXOFF-103 (E2) — charts on the Word surface: read, authored, edited.
//!
//! # The non-discriminating test this file exists to avoid
//!
//! The ticket names it exactly: *"asserting that a chart we authored reads back through our own
//! reader. That passes whether or not the chart is reachable the way Word reaches it."* Every
//! authored-chart case below would have stayed green against a reader that found charts by, say,
//! walking the package for `word/charts/*.xml` and never looking at `w:drawing` at all.
//!
//! So the first half of this file is driven by `tests/fixtures/chart_in_word.docx`, which **this
//! project did not write**: it was produced by **Apache POI 5.5.1** (`XWPFDocument::createChart`),
//! and it disagrees with our writer in ways that are load-bearing here —
//!
//! * its drawing's `wp:docPr@id` is **`0`**, a value our own `next_drawing_id` never produces
//!   (it starts at 1), so a reader that assumed ids begin at 1 fails on it;
//! * its parts are ordered chart-part-before-`document.xml`, and it ships **no** `word/styles.xml`
//!   or theme at all;
//! * it writes booleans as `val="false"`, values as `18.0`, and names its sheet `Sheet0` — three
//!   spellings our writer never emits, so a byte-identity claim over it cannot be satisfied by
//!   accident;
//! * its embedded workbook is `Microsoft_Excel_Worksheet1.xlsx`, not the `Microsoft_Excel_Sheet1`
//!   stem we author, so the reference has to be *read* rather than reconstructed.
//!
//! That fixture already found one real defect while this child was being written:
//! `GraphicData::chart_relationship_id` first looked the `r:id` up by namespace, and the fidelity
//! reader does not resolve namespaces for *attributes* — so the lookup matched nothing and
//! `chart_drawing_ids` answered `[]` on a document that plainly holds a chart. Every authored-chart
//! test in this file was green at that moment.

use mjx_chart::{ChartData, ChartKind, ChartLabelScope, DataLabelSpec, LegendPosition};
use mjx_dml::{ColorSpec, FillSpec};
use mjx_docx::{ChartPlacement, ChartWrap, Document, DocxError, PageSize};
use mjx_ooxml_core::{FromXml, ToXml};
use mjx_ooxml_types::wordprocessingdrawing::WrapText;
use mjx_opc::{Package, PartName};

/// The producer-written fixture's bytes.
fn producer_docx() -> Vec<u8> {
    mjx_fixtures::fixture("chart_in_word.docx")
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

/// The decompressed text of one part of a package.
fn part_text(bytes: &[u8], part: &str) -> String {
    let package = Package::open(bytes).expect("the package opens");
    let name = PartName::new(part).expect("a part name");
    String::from_utf8_lossy(
        package
            .part_bytes(&name)
            .unwrap_or_else(|| panic!("no part {part}; the package holds {:?}", part_names(bytes))),
    )
    .into_owned()
}

/// The raw bytes of one part of a package — for a part that is not text (a chart's embedded
/// workbook is a whole `.xlsx`, i.e. a ZIP), where `part_text`'s lossy UTF-8 conversion would
/// corrupt the very bytes under test.
fn part_bytes_of(bytes: &[u8], part: &str) -> Vec<u8> {
    let package = Package::open(bytes).expect("the package opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .unwrap_or_else(|| panic!("no part {part}; the package holds {:?}", part_names(bytes)))
        .to_vec()
}

/// The text of one part *inside* a chart's embedded workbook package.
fn workbook_part_text(document_bytes: &[u8], workbook_part: &str, inner_part: &str) -> String {
    let workbook = part_bytes_of(document_bytes, workbook_part);
    let inner = Package::open(&workbook).expect("the embedded workbook opens");
    String::from_utf8_lossy(
        inner
            .part_bytes(&PartName::new(inner_part).expect("a part name"))
            .unwrap_or_else(|| panic!("no {inner_part} in {workbook_part}")),
    )
    .into_owned()
}

/// Every part name in a package, sorted.
fn part_names(bytes: &[u8]) -> Vec<String> {
    let package = Package::open(bytes).expect("the package opens");
    let mut names: Vec<String> = package
        .part_names()
        .map(|name| name.as_str().to_owned())
        .collect();
    names.sort();
    names
}

/// A blank A4 document, whose body already holds one empty `w:p` (`blank.rs`'s own template) — the
/// paragraph every case here addresses as `0`.
fn blank_with_a_paragraph() -> Document {
    Document::blank(PageSize::a4()).expect("a blank document")
}

/// Rewrites one part of a saved document and hands back the reopened [`Document`].
///
/// `Document` exposes no package handle, so a test that needs to plant bytes goes out through
/// `save` and back in through `open` — which is stricter anyway: the planted part has to survive a
/// real round-trip rather than living only in an in-memory tree.
fn with_part_replaced(document: &Document, part: &str, bytes: Vec<u8>) -> Document {
    let saved = document.save().expect("it saves");
    let mut package = Package::open(&saved).expect("the package opens");
    package
        .replace_part_bytes(&PartName::new(part).expect("a part name"), bytes)
        .expect("the part is replaced");
    Document::open(&package.save().expect("the package saves")).expect("it reopens")
}

/// The two-series chart every authoring case here adds.
fn sample_chart() -> ChartData {
    ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3"])
        .series("North", [12.5, 18.0, 21.5])
        .series("South", [9.0, 11.5, 14.0])
}

// =================================================================================================
// The producer-written fixture — tier 1, and the reachability that makes tier 1 non-vacuous
// =================================================================================================

#[test]
fn a_producer_written_docx_carrying_a_chart_round_trips_byte_for_byte() {
    let original = producer_docx();
    let document = Document::open(&original).expect("the fixture opens");
    let written = document.save().expect("it saves");

    let before = part_payloads(&original);
    let after = part_payloads(&written);
    assert_eq!(
        before.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        after.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        "opening and saving must add and remove no parts"
    );
    for ((name, before_bytes), (_, after_bytes)) in before.iter().zip(after.iter()) {
        assert_eq!(
            String::from_utf8_lossy(before_bytes),
            String::from_utf8_lossy(after_bytes),
            "part {name} must re-emit byte for byte"
        );
    }
}

#[test]
fn the_producer_written_chart_part_round_trips_through_the_enriched_model() {
    // The `mjx-chart` half of the same claim, and the direct Word counterpart of
    // `crates/mjx-chart/tests/preservation.rs`: hand every element of a chart part this project did
    // not write to the typed model, write it back, and require the payload to be identical.
    let original = producer_docx();
    let chart_part = part_text(&original, "/word/charts/chart1.xml");

    let mut document = mjx_xml::fidelity::parse(chart_part.as_bytes()).expect("the part parses");
    let space =
        mjx_chart::ChartSpace::from_xml(&document.root, &document.interner).expect("from_xml");
    document.root = space.to_xml(&mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);

    assert_eq!(
        String::from_utf8_lossy(&written),
        chart_part,
        "reaching a chart from a Word document may add reach, never change what is written back"
    );
}

#[test]
fn the_fixture_really_does_disagree_with_what_this_library_would_write() {
    // The counterpart of the two byte-identity cases above: both would also pass against a fixture
    // our own writer produced, which is the trap §7 names. These four assertions pin the specific
    // ways this file is *not* ours, so a later replacement of the fixture with an authored one
    // fails here rather than quietly weakening both.
    let original = producer_docx();
    let document_part = part_text(&original, "/word/document.xml");
    let chart_part = part_text(&original, "/word/charts/chart1.xml");

    assert!(
        document_part.contains(r#"<wp:docPr id="0" name="chart 0"/>"#),
        "the producer numbered its drawing from 0; ours never does: {document_part}"
    );
    assert!(
        chart_part.contains(r#"<c:overlay val="false"/>"#),
        "the producer spells booleans `false`; `mjx-ooxml-types::support` writes `0`"
    );
    assert!(
        chart_part.contains("<c:v>18.0</c:v>"),
        "the producer writes a trailing `.0`; our own numeric writer does not"
    );
    assert!(
        part_names(&original).contains(&"/word/embeddings/Microsoft_Excel_Worksheet1.xlsx".to_owned()),
        "the producer's workbook stem is `Microsoft_Excel_Worksheet`, ours is `Microsoft_Excel_Sheet`"
    );
    assert!(
        !part_names(&original).contains(&"/word/styles.xml".to_owned()),
        "the producer ships no styles part at all"
    );
}

#[test]
fn editing_a_producer_written_chart_changes_only_what_the_edit_is_about() {
    // A7d recorded a standing limitation and handed it forward: `edit_chart` reads a *whole part*
    // into a typed model and writes the whole part back, so subtree copy-on-write does not reach
    // inside a chart part and such a part "comes back re-flowed". **MJXOFF-143 closed that**, by
    // carrying the source span through `FromXml`/`ToXml`, and the Word chart path inherits the fix
    // rather than documenting the limitation.
    //
    // This is that inheritance, measured rather than assumed, and measured on a part *this project
    // did not write* — which is the only place a re-flow could show up, since our own writer's
    // output is by definition in our own writer's shape. Moving the legend rewrites one attribute
    // value; every other byte of a 2,985-byte producer-written part must survive.
    let original = producer_docx();
    let before = part_text(&original, "/word/charts/chart1.xml");

    let mut document = Document::open(&original).expect("the fixture opens");
    let chart = document.chart_drawing_ids().expect("charts")[0];
    document
        .set_chart_legend(chart, Some(LegendPosition::Top))
        .expect("the legend moves");
    let after = part_text(
        &document.save().expect("it saves"),
        "/word/charts/chart1.xml",
    );

    assert!(
        before.contains(r#"<c:legendPos val="b"/>"#) && after.contains(r#"<c:legendPos val="t"/>"#),
        "the edit really did move the legend"
    );
    assert_eq!(
        before.replace(r#"<c:legendPos val="b"/>"#, "@"),
        after.replace(r#"<c:legendPos val="t"/>"#, "@"),
        "outside the one attribute the edit is about, a chart part must come back byte for byte —          if this fails, MJXOFF-143's span carry has regressed and A7d's re-flow limitation is back"
    );
    assert_eq!(
        before.len(),
        after.len(),
        "same length, because `val=\"b\"` and `val=\"t\"` are the same length — a re-flow would          change this even when the assertion above happened to hold"
    );
}

#[test]
fn a_chart_a_third_party_producer_wrote_is_reachable_the_way_word_reaches_it() {
    // The whole read surface, against markup nobody here chose. Reaching it means walking
    // `w:body > w:p > w:r > w:drawing > wp:inline > a:graphic > a:graphicData@uri > c:chart@r:id`
    // and resolving that relationship — the path Word walks. A reader that found charts by scanning
    // the package for `word/charts/*.xml` would pass every authored case in this file and fail here.
    let mut document = Document::open(&producer_docx()).expect("the fixture opens");

    let ids = document.chart_drawing_ids().expect("the charts are found");
    assert_eq!(
        ids,
        [0],
        "the producer's own drawing id is 0, and it is read rather than assumed"
    );
    let chart = ids[0];

    assert_eq!(
        document.chart_rel_id(chart).expect("a relationship"),
        Some("rId2".to_owned()),
        "the chart's own r:id is read from the file, not reconstructed"
    );
    assert_eq!(
        document.chart_kinds(chart).expect("kinds"),
        [ChartKind::Bar]
    );

    let series = document.chart_series(chart).expect("series");
    assert_eq!(series.len(), 2);
    assert_eq!(series[0].name.as_deref(), Some("North"));
    assert_eq!(series[0].categories, ["Q1", "Q2", "Q3", "Q4"]);
    assert_eq!(series[0].values, [12.5, 18.0, 21.5, 27.0]);
    assert_eq!(series[1].name.as_deref(), Some("South"));
    assert_eq!(series[1].values, [9.0, 11.5, 14.0, 16.5]);

    assert_eq!(
        document.chart_title(chart).expect("a title"),
        Some("Revenue by quarter".to_owned())
    );
    let legend = document
        .chart_legend(chart)
        .expect("a legend")
        .expect("one");
    assert_eq!(legend.position, Some(LegendPosition::Bottom));
    assert_eq!(legend.overlays_plot, Some(false));

    let axes = document.chart_axes(chart).expect("axes");
    assert_eq!(axes.len(), 2, "the producer drew against two axes");
    assert_eq!(axes[0].kind, mjx_chart::AxisKind::Category);
    assert_eq!(axes[1].kind, mjx_chart::AxisKind::Value);
    assert_eq!(axes[1].title.as_deref(), Some("Millions"));

    // The embedded workbook is found through the chart part's own relationship, whose target
    // (`../embeddings/Microsoft_Excel_Worksheet1.xlsx`) is a name we would never have generated.
    let workbooks = document.chart_workbooks().expect("workbooks");
    assert_eq!(workbooks.len(), 1);
    assert_eq!(workbooks[0].drawing_id, 0);
    assert_eq!(
        workbooks[0].target,
        "../embeddings/Microsoft_Excel_Worksheet1.xlsx"
    );
    assert!(!workbooks[0].external);
}

// =================================================================================================
// Authoring
// =================================================================================================

#[test]
fn an_authored_chart_writes_three_parts_and_a_run_that_references_it() {
    let mut document = blank_with_a_paragraph();
    let before = part_names(&document.save().expect("it saves"));

    let drawing = document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");
    let bytes = document.save().expect("it saves");
    let after = part_names(&bytes);

    for part in [
        "/word/charts/chart1.xml",
        "/word/charts/_rels/chart1.xml.rels",
        "/word/embeddings/Microsoft_Excel_Sheet1.xlsx",
    ] {
        assert!(
            after.contains(&part.to_owned()) && !before.contains(&part.to_owned()),
            "{part} must be new; before {before:?} after {after:?}"
        );
    }
    // Four parts appear, not three: a blank document relates to nothing, so it ships no
    // `word/_rels/document.xml.rels` at all until the chart gives it its first relationship.
    assert!(
        after.contains(&"/word/_rels/document.xml.rels".to_owned())
            && !before.contains(&"/word/_rels/document.xml.rels".to_owned()),
        "the document's own .rels is created by the chart's relationship: before {before:?}"
    );
    assert_eq!(
        after.len(),
        before.len() + 4,
        "those four parts and no others: {after:?}"
    );

    // The run really references the chart part, through a `wp:inline` — not through anything a
    // reader of ours invented.
    let document_part = part_text(&bytes, "/word/document.xml");
    assert!(document_part.contains("<w:drawing"), "{document_part}");
    assert!(document_part.contains("<wp:inline"), "{document_part}");
    assert!(
        document_part.contains(r#"uri="http://schemas.openxmlformats.org/drawingml/2006/chart""#),
        "{document_part}"
    );
    assert!(document_part.contains("<c:chart"), "{document_part}");

    // And it reads back through the address `add_chart` handed out.
    let mut reopened = Document::open(&bytes).expect("it reopens");
    assert_eq!(reopened.chart_drawing_ids().expect("ids"), [drawing]);
    assert_eq!(
        reopened
            .chart_series(drawing)
            .expect("series")
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>(),
        [Some("North".to_owned()), Some("South".to_owned())]
    );
}

#[test]
fn an_unnamed_series_still_leaves_the_data_where_the_formulas_point() {
    // The Word counterpart of `mjx-pptx`'s test of the same name, and the reason MJXOFF-99 asked for
    // one: a chart whose series carry no `c:tx` has an all-blank header row, and
    // `AuthoredWorksheet::appended_row_count` is what keeps that row *consuming its row number*
    // rather than letting the categories slide up to row 1 while the chart's own `c:f` still says
    // `$A$2`. Word authoring goes through the identical `mjx-sml` writer, so it inherits both the
    // bug and the fix — which is exactly why it needs its own guard.
    let mut document = blank_with_a_paragraph();
    let drawing = document
        .add_chart(
            0usize,
            &ChartData::new(ChartKind::Bar)
                .categories(["North", "South"])
                .series("Sales", [19.2, 21.4]),
            4_572_000,
            2_743_200,
            "Sales",
        )
        .expect("the chart is added");

    // Strip the series' own name from the authored chart part, exactly as the PowerPoint case does,
    // then make the workbook agree with it again.
    let saved = document.save().expect("it saves");
    let text = part_text(&saved, "/word/charts/chart1.xml");
    let opening = text
        .find("<c:tx>")
        .expect("an authored series names itself");
    let closing = text.find("</c:tx>").expect("the name element closes") + "</c:tx>".len();
    let stripped = format!("{}{}", &text[..opening], &text[closing..]);
    let mut document =
        with_part_replaced(&document, "/word/charts/chart1.xml", stripped.into_bytes());

    assert_eq!(
        document
            .chart_series(drawing)
            .expect("series")
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>(),
        [None],
        "the header row now has nothing at all to write"
    );
    assert!(
        document
            .refresh_chart_workbook(drawing)
            .expect("the workbook refreshes"),
        "there is an embedded workbook to refresh"
    );

    let bytes = document.save().expect("it saves");
    let sheet = workbook_part_text(
        &bytes,
        "/word/embeddings/Microsoft_Excel_Sheet1.xlsx",
        "/xl/worksheets/sheet1.xml",
    );

    assert!(
        sheet.contains(r#"<row r="2">"#),
        "the categories start on row 2, where the chart's `c:f` says they are: {sheet}"
    );
    assert!(
        !sheet.contains(r#"<row r="1">"#),
        "and the blank header row writes no <row> at all: {sheet}"
    );
}

#[test]
fn a_floating_chart_writes_the_anchor_and_the_wrap_it_was_asked_for() {
    let mut document = blank_with_a_paragraph();
    document
        .add_chart_placed(
            0usize,
            &sample_chart(),
            4_572_000,
            2_743_200,
            "Floating revenue",
            ChartPlacement::Floating {
                offset_x_emu: 228_600,
                offset_y_emu: 114_300,
                wrap: ChartWrap::Square(WrapText::BothSides),
            },
        )
        .expect("the chart is added");
    let bytes = document.save().expect("it saves");
    let part = part_text(&bytes, "/word/document.xml");

    assert!(part.contains("<wp:anchor"), "{part}");
    assert!(
        !part.contains("<wp:inline"),
        "a floating chart is not inline: {part}"
    );
    assert!(
        part.contains(r#"<wp:wrapSquare wrapText="bothSides"/>"#),
        "the wrap asked for is the wrap written: {part}"
    );
    assert!(
        part.contains(r#"<wp:posOffset>228600</wp:posOffset>"#),
        "the horizontal offset is written in EMU: {part}"
    );
    assert!(
        part.contains(r#"<wp:posOffset>114300</wp:posOffset>"#),
        "and so is the vertical one: {part}"
    );

    // A floating chart is found by the same enumeration an inline one is: the anchor branch of
    // `for_each_drawing` is exercised here and nowhere else.
    let mut reopened = Document::open(&bytes).expect("it reopens");
    let ids = reopened.chart_drawing_ids().expect("ids");
    assert_eq!(ids.len(), 1, "an anchored chart is still a chart");
    assert_eq!(
        reopened.chart_kinds(ids[0]).expect("kinds"),
        [ChartKind::Bar]
    );
}

#[test]
fn the_three_wrap_modes_each_write_their_own_element() {
    for (wrap, expected) in [
        (ChartWrap::None, "<wp:wrapNone/>"),
        (
            ChartWrap::Square(WrapText::Left),
            r#"<wp:wrapSquare wrapText="left"/>"#,
        ),
        (ChartWrap::TopAndBottom, "<wp:wrapTopAndBottom/>"),
    ] {
        let mut document = blank_with_a_paragraph();
        document
            .add_chart_placed(
                0usize,
                &sample_chart(),
                914_400,
                914_400,
                "Chart",
                ChartPlacement::Floating {
                    offset_x_emu: 0,
                    offset_y_emu: 0,
                    wrap,
                },
            )
            .expect("the chart is added");
        let bytes = document.save().expect("it saves");
        let part = part_text(&bytes, "/word/document.xml");
        assert!(
            part.contains(expected),
            "{wrap:?} must write {expected}: {part}"
        );
    }
}

#[test]
fn a_chart_with_nothing_to_draw_is_refused_before_a_part_is_written() {
    let mut document = blank_with_a_paragraph();
    let before = part_names(&document.save().expect("it saves"));
    let failure = document
        .add_chart(
            0usize,
            &ChartData::new(ChartKind::Bar),
            914_400,
            914_400,
            "Empty",
        )
        .expect_err("an empty chart is refused");
    assert!(
        matches!(failure, DocxError::InvalidChartData),
        "{failure:?}"
    );
    assert_eq!(
        part_names(&document.save().expect("it saves")),
        before,
        "a refused chart writes no part at all"
    );
}

// =================================================================================================
// Editing, and tier-3 edit isolation
// =================================================================================================

#[test]
fn adding_a_chart_leaves_every_other_part_byte_identical() {
    let document = blank_with_a_paragraph();
    let before_bytes = document.save().expect("it saves");
    let before = part_payloads(&before_bytes);

    let mut document = Document::open(&before_bytes).expect("it reopens");
    document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");
    let after = part_payloads(&document.save().expect("it saves"));

    for (name, payload) in &before {
        let Some((_, after_payload)) = after.iter().find(|(n, _)| n == name) else {
            panic!("part {name} disappeared");
        };
        // `document.xml` gains the run, and the content types and the document's own `.rels` gain
        // the new parts. Everything else must be untouched.
        if matches!(
            name.as_str(),
            "/word/document.xml" | "/[Content_Types].xml" | "/word/_rels/document.xml.rels"
        ) {
            continue;
        }
        assert_eq!(
            String::from_utf8_lossy(payload),
            String::from_utf8_lossy(after_payload),
            "adding a chart must leave {name} byte-identical"
        );
    }
}

#[test]
fn editing_a_charts_data_touches_the_chart_part_and_its_workbook_and_nothing_else() {
    let mut document = blank_with_a_paragraph();
    let drawing = document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");
    let before_bytes = document.save().expect("it saves");
    let before = part_payloads(&before_bytes);

    let mut document = Document::open(&before_bytes).expect("it reopens");
    document
        .set_chart_series_values(drawing, 0, &[40.0, 41.0, 42.0])
        .expect("the values are rewritten");
    let after_bytes = document.save().expect("it saves");
    let after = part_payloads(&after_bytes);

    assert_eq!(
        before.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        after.iter().map(|(n, _)| n).collect::<Vec<_>>(),
        "a data edit adds and removes no parts"
    );
    let mut changed: Vec<&str> = Vec::new();
    for ((name, before_payload), (_, after_payload)) in before.iter().zip(after.iter()) {
        if before_payload != after_payload {
            changed.push(name);
        }
    }
    assert_eq!(
        changed,
        [
            "/word/charts/chart1.xml",
            "/word/embeddings/Microsoft_Excel_Sheet1.xlsx"
        ],
        "only the chart and the workbook it opens may change"
    );

    // And the workbook really carries the new numbers, not the old ones: the refresh is not a
    // no-op that merely re-compressed the part.
    let sheet = workbook_part_text(
        &after_bytes,
        "/word/embeddings/Microsoft_Excel_Sheet1.xlsx",
        "/xl/worksheets/sheet1.xml",
    );
    assert!(
        sheet.contains("<v>42</v>"),
        "the workbook holds the new value: {sheet}"
    );
    assert!(
        !sheet.contains("<v>21.5</v>"),
        "and not the old one: {sheet}"
    );
}

#[test]
fn editing_a_charts_styling_touches_only_the_chart_part() {
    let mut document = blank_with_a_paragraph();
    let drawing = document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");
    let before_bytes = document.save().expect("it saves");
    let before = part_payloads(&before_bytes);

    let mut document = Document::open(&before_bytes).expect("it reopens");
    document
        .set_chart_series_fill(
            drawing,
            0,
            &FillSpec::Solid(ColorSpec::Srgb("1F77B4".to_owned())),
        )
        .expect("the fill is set");
    document
        .set_chart_legend(drawing, Some(LegendPosition::Right))
        .expect("the legend moves");
    let after = part_payloads(&document.save().expect("it saves"));

    let changed: Vec<&str> = before
        .iter()
        .zip(after.iter())
        .filter(|((_, b), (_, a))| b != a)
        .map(|((name, _), _)| name.as_str())
        .collect();
    assert_eq!(
        changed,
        ["/word/charts/chart1.xml"],
        "styling does not touch the workbook — the numbers did not change"
    );
}

#[test]
fn every_edit_reads_back_through_the_word_surface() {
    // One document driven through the whole edit family, so a method wired to the wrong operation
    // shows up as the wrong value coming back rather than as a compile error nobody sees.
    let mut document = blank_with_a_paragraph();
    let chart = document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");

    document
        .set_chart_title(chart, Some("Regional revenue"))
        .expect("title");
    assert_eq!(
        document.chart_title(chart).expect("read back"),
        Some("Regional revenue".to_owned())
    );

    document
        .set_chart_series_categories(chart, 0, &["Spring", "Summer", "Autumn"])
        .expect("categories");
    assert_eq!(
        document.chart_series(chart).expect("series")[0].categories,
        ["Spring", "Summer", "Autumn"]
    );

    document
        .set_chart_axis_scale(chart, 1, Some(0.0), Some(50.0))
        .expect("scale");
    let axes = document.chart_axes(chart).expect("axes");
    assert_eq!(axes[1].minimum, Some(0.0));
    assert_eq!(axes[1].maximum, Some(50.0));

    document
        .set_chart_axis_title(chart, 1, Some("Millions"))
        .expect("axis title");
    assert_eq!(
        document.chart_axes(chart).expect("axes")[1]
            .title
            .as_deref(),
        Some("Millions")
    );

    document
        .set_chart_axis_gridlines(chart, 1, true, false)
        .expect("gridlines");
    assert!(document.chart_axes(chart).expect("axes")[1].major_gridlines);

    document
        .set_chart_axis_orientation(chart, 1, mjx_chart::AxisOrientation::MaximumToMinimum)
        .expect("orientation");
    assert_eq!(
        document.chart_axes(chart).expect("axes")[1].orientation,
        Some(mjx_chart::AxisOrientation::MaximumToMinimum)
    );

    document
        .set_chart_data_labels(
            chart,
            ChartLabelScope::Series { series_idx: 0 },
            &DataLabelSpec::new().value(true),
        )
        .expect("data labels");
    assert_eq!(
        document
            .chart_data_labels(chart, 0, None)
            .expect("read back")
            .shows_value,
        Some(true)
    );

    document
        .set_chart_point_explosion(chart, 0, 1, Some(25))
        .expect("explosion");
    let formats = document
        .chart_point_formats(chart, 0)
        .expect("point formats");
    assert_eq!(formats.len(), 1);
    assert_eq!(formats[0].index, Some(1));
    assert_eq!(formats[0].explosion, Some(25));

    document
        .add_chart_trendline(
            chart,
            0,
            &mjx_chart::TrendlineSpec::new(mjx_chart::TrendlineKind::Linear),
        )
        .expect("trendline");
    assert_eq!(
        document
            .chart_trendlines(chart, 0)
            .expect("trendlines")
            .len(),
        1
    );
    assert_eq!(
        document.remove_chart_trendlines(chart, 0).expect("removed"),
        1
    );

    document
        .set_chart_error_bars(
            chart,
            0,
            &mjx_chart::ErrorBarSpec::fixed(
                mjx_chart::ErrorBarType::Both,
                mjx_chart::ErrorValueType::FixedValue,
                1.5,
            ),
        )
        .expect("error bars");
    assert_eq!(document.chart_error_bars(chart, 0).expect("bars").len(), 1);
    assert_eq!(
        document.remove_chart_error_bars(chart, 0).expect("removed"),
        1
    );

    assert!(
        document
            .remove_chart_data_labels(chart, ChartLabelScope::Series { series_idx: 0 })
            .expect("removed"),
        "the labels set above were there to remove"
    );
    assert!(
        document
            .remove_chart_point_format(chart, 0, 1)
            .expect("removed"),
        "the point format set above was there to remove"
    );

    // The document is still schema-valid after all of that — see `schema_gate.rs` for the gate that
    // says so against the real XSDs; this is the cheap structural check that nothing was lost.
    let bytes = document.save().expect("it saves");
    let mut reopened = Document::open(&bytes).expect("it reopens");
    assert_eq!(
        reopened.chart_title(chart).expect("title survives a save"),
        Some("Regional revenue".to_owned())
    );
}

#[test]
fn detaching_the_workbook_leaves_a_cache_only_chart() {
    let mut document = blank_with_a_paragraph();
    let chart = document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");
    assert_eq!(document.chart_workbooks().expect("workbooks").len(), 1);

    document.detach_chart_workbook(chart).expect("it detaches");
    assert!(
        document.chart_workbooks().expect("workbooks").is_empty(),
        "the reference is gone"
    );
    assert!(
        !document
            .refresh_chart_workbook(chart)
            .expect("no workbook to refresh"),
        "and there is now nothing to refresh"
    );
    // The chart still draws: its caches are untouched.
    assert_eq!(
        document.chart_series(chart).expect("series")[0].values,
        [12.5, 18.0, 21.5]
    );

    let failure = document
        .detach_chart_workbook(chart)
        .expect_err("detaching twice is refused");
    assert!(
        matches!(failure, DocxError::ChartHasNoExternalData),
        "{failure:?}"
    );
}

#[test]
fn removing_a_chart_drawing_sweeps_its_chart_part_and_the_workbook_that_part_alone_referenced() {
    // The gap MJXOFF-103 found in MJXOFF-131's `remove_drawing`: it looked only for a *picture's*
    // image relationship, so removing a chart left `word/_rels/document.xml.rels` pointing at a
    // chart part nothing referenced. Neutralise `find_drawing_referenced_rel_id`'s chart arm and
    // this case goes red on the leftover parts.
    let mut document = blank_with_a_paragraph();
    let chart = document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");
    let with_chart = part_names(&document.save().expect("it saves"));
    assert!(with_chart.contains(&"/word/charts/chart1.xml".to_owned()));

    assert!(document.remove_drawing(chart).expect("it is removed"));
    let bytes = document.save().expect("it saves");
    let after = part_names(&bytes);

    assert!(
        !after.contains(&"/word/charts/chart1.xml".to_owned()),
        "the chart part must go with its drawing: {after:?}"
    );
    assert!(
        !after.contains(&"/word/embeddings/Microsoft_Excel_Sheet1.xlsx".to_owned()),
        "and so must the workbook only that chart part referenced: {after:?}"
    );
    assert!(
        !part_text(&bytes, "/word/_rels/document.xml.rels").contains("relationships/chart"),
        "no relationship may be left pointing at a part that is gone"
    );
    assert!(Document::open(&bytes)
        .expect("it reopens")
        .chart_drawing_ids()
        .expect("ids")
        .is_empty());
}

#[test]
fn removing_a_producer_written_chart_sweeps_its_parts_too() {
    // The read-path counterpart of the case above, and it exists because a mutation showed the
    // authored one could not see a real bug.
    //
    // `GraphicData::for_chart` builds the `r:id` attribute with its namespace field populated, so a
    // chart this library has just authored and not yet round-tripped carries information a *parsed*
    // chart never does — the fidelity reader resolves namespaces for elements and leaves an
    // attribute's own `namespace` empty. Keying `chart_relationship_id` on that namespace therefore
    // leaves every authored case green and breaks every file anyone else wrote. That is §7's shape
    // exactly, in the mirror: the authored path carries more than the read path, so it passes a
    // check the read path fails.
    //
    // So this removes a chart from `chart_in_word.docx`, which has been through a real parse.
    let mut document = Document::open(&producer_docx()).expect("the fixture opens");
    let chart = document.chart_drawing_ids().expect("charts")[0];
    assert!(document.remove_drawing(chart).expect("it is removed"));

    let bytes = document.save().expect("it saves");
    let after = part_names(&bytes);
    assert!(
        !after.contains(&"/word/charts/chart1.xml".to_owned()),
        "the producer's chart part must go with its drawing: {after:?}"
    );
    assert!(
        !after.contains(&"/word/embeddings/Microsoft_Excel_Worksheet1.xlsx".to_owned()),
        "and so must the workbook only that chart part referenced: {after:?}"
    );
    assert!(
        !part_text(&bytes, "/word/_rels/document.xml.rels").contains("relationships/chart"),
        "no relationship may be left pointing at a part that is gone"
    );
}

// =================================================================================================
// Refusals
// =================================================================================================

#[test]
fn a_drawing_that_frames_no_chart_is_refused_by_the_id_it_was_given() {
    let mut document = blank_with_a_paragraph();
    let picture = document
        .add_inline_picture(
            0usize,
            b"not really a png".to_vec(),
            "image/png",
            "png",
            914_400,
            914_400,
            "Picture",
        )
        .expect("the picture is added");

    assert!(
        document.chart_drawing_ids().expect("ids").is_empty(),
        "a picture is not a chart"
    );
    let failure = document
        .chart_series(picture)
        .expect_err("a picture has no series");
    assert!(
        matches!(failure, DocxError::DrawingIsNotAChart { drawing_id } if drawing_id == picture),
        "{failure:?}"
    );

    let failure = document
        .chart_series(9_999)
        .expect_err("an unknown id has no series");
    assert!(
        matches!(failure, DocxError::DrawingIsNotAChart { drawing_id } if drawing_id == 9_999),
        "{failure:?}"
    );
}

#[test]
fn an_index_past_the_end_is_a_typed_refusal_carrying_the_count() {
    let mut document = blank_with_a_paragraph();
    let chart = document
        .add_chart(0usize, &sample_chart(), 4_572_000, 2_743_200, "Revenue")
        .expect("the chart is added");

    let failure = document
        .chart_series_fill(chart, 7)
        .expect_err("series 7 is past the end");
    assert!(
        matches!(
            failure,
            DocxError::ChartAccess(mjx_chart::ChartAccessError::SeriesOutOfRange {
                index: 7,
                count: 2
            })
        ),
        "the refusal names both the index and the count: {failure:?}"
    );

    let failure = document
        .set_chart_axis_scale(chart, 9, Some(0.0), None)
        .expect_err("axis 9 is past the end");
    assert!(
        matches!(
            failure,
            DocxError::ChartAccess(mjx_chart::ChartAccessError::AxisOutOfRange { index: 9, .. })
        ),
        "{failure:?}"
    );

    let failure = document
        .set_chart_series_fill(
            chart,
            0,
            &FillSpec::Picture {
                rel_id: "rId9".into(),
                mode: mjx_dml::PictureFillMode::Stretch,
            },
        )
        .expect_err("a chart part relates to no images");
    assert!(
        matches!(
            failure,
            DocxError::ChartAccess(mjx_chart::ChartAccessError::FillNotSupported)
        ),
        "{failure:?}"
    );
}

// =================================================================================================
// The restated constants
// =================================================================================================

#[test]
fn the_word_chart_constants_are_the_same_strings_powerpoint_uses() {
    // `mjx-docx` cannot depend on `mjx-pptx` (both rank 3.0), so `REL_CHART`, `REL_PACKAGE` and
    // `CONTENT_TYPE_CHART` are declared a second time. A restatement that drifts is a chart Word
    // opens and PowerPoint does not, so the two spellings are compared here rather than trusted.
    assert_eq!(
        mjx_docx::constants::REL_CHART,
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart"
    );
    assert_eq!(
        mjx_docx::constants::REL_PACKAGE,
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/package"
    );
    assert_eq!(
        mjx_docx::constants::CONTENT_TYPE_CHART,
        "application/vnd.openxmlformats-officedocument.drawingml.chart+xml"
    );
}
