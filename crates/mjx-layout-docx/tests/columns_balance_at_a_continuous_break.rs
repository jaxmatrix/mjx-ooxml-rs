//! **Columns, and the difference a trailing `continuous` break makes to them.**
//!
//! # "Columns exist" is not an assertion about columns
//!
//! A page divided into two columns and filled straight down the first one looks like a two-column
//! page, produces two-column fragments, and is what almost every naive implementation does. So the
//! assertions below are on **column heights**: a section that ends at a `continuous` break has its
//! columns levelled, and the same content without that break leaves the last column short. The pair
//! is the assertion — one of them alone is green for an engine that balances everything and for one
//! that balances nothing, depending on which one you write.
//!
//! # Why the balanced height is not the total divided by two
//!
//! Content is placed in whole lines. Dividing the total height by the column count gives a target
//! that is routinely a hair short, and a column a hair short spills a whole line into the next one —
//! *unbalancing* what was being balanced. [`mjx_layout_docx::assemble`] therefore searches for the
//! shortest height at which everything still fits, which is exact. The assertion here is that the
//! two columns come out **within one line of each other**, which is the strongest statement that is
//! true of whole lines.

mod support;

use mjx_layout::PageIndex;
use mjx_layout_docx::{DocumentBoxModel, PageReport};
use mjx_ooxml_core::measure::Emu;
use support::{constraints, flow, model};

/// Two equal columns half an inch apart.
const TWO_COLUMNS: &str = r#"<w:cols w:num="2" w:equalWidth="true" w:space="720"/>"#;

/// Twelve short paragraphs — enough to fill more than one column and not a page.
fn body() -> Vec<String> {
    (0..12)
        .map(|index| support::paragraph("", &format!("Paragraph number {index} of the column.")))
        .collect()
}

/// Every page's report.
fn reports(markup: Vec<u8>, area: &mjx_layout::Constraints, limit: usize) -> Vec<PageReport> {
    let mut document = support::document_from_bytes(markup);
    let flow = flow(&mut document);
    let mut model = model();
    let mut out = Vec::new();
    let mut resume = None;
    for number in 0..limit {
        let Ok(page) = <DocumentBoxModel as mjx_layout::BoxModel>::layout_page(
            &mut model,
            &flow,
            PageIndex::new(u32::try_from(number).expect("a page number")),
            area,
            resume.as_ref(),
        ) else {
            break;
        };
        resume = page.continuation().cloned();
        out.push(model.last_page().clone());
        if resume.is_none() {
            break;
        }
    }
    out
}

/// A `w:cols` section really does divide the text area, and the bands do not overlap.
#[test]
fn two_columns_split_the_text_area_with_a_gap_between_them() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let markup = support::document_markup_with(&body(), &format!("{geometry}{TWO_COLUMNS}"));
    let pages = reports(markup, &constraints(8.5, 11.0), 4);
    let first = pages.first().expect("a page");
    assert_eq!(first.column_heights.len(), 2, "two columns: {first:?}");
    assert_eq!(first.body.width(), Emu::from_inches(6.5));
    assert!(
        !first.column_separator,
        "this section states no `w:cols@sep`"
    );
}

/// `w:cols@sep` reaches the page report — **and is not drawn**, which is the same decision
/// `w:pBdr/w:between` and a `bar` tab stop already carry: a rule is a paint, and painting is
/// `mjx-scene-docx`'s (MJXOFF-255). Reporting it is what lets the companion draw it without
/// re-reading the document.
#[test]
fn a_column_separator_is_reported_and_not_drawn() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let separated = r#"<w:cols w:num="2" w:equalWidth="true" w:space="720" w:sep="true"/>"#;
    let with = reports(
        support::document_markup_with(&body(), &format!("{geometry}{separated}")),
        &constraints(8.5, 11.0),
        4,
    );
    let without = reports(
        support::document_markup_with(&body(), &format!("{geometry}{TWO_COLUMNS}")),
        &constraints(8.5, 11.0),
        4,
    );
    assert!(with.first().expect("a page").column_separator);
    assert!(!without.first().expect("a page").column_separator);
    assert_eq!(
        with.first().expect("a page").column_heights,
        without.first().expect("a page").column_heights,
        "and it changes no geometry, because nothing is drawn for it yet"
    );
}

/// **The assertion the ticket asks for.** The same content, once with a trailing `continuous` break
/// and once without, compared on the two columns' heights.
#[test]
fn a_trailing_continuous_break_levels_the_columns_and_its_absence_does_not() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let area = constraints(8.5, 11.0);

    // Unbalanced: the multi-column section is the document's last, so nothing follows it and the
    // first column simply fills.
    let unbalanced = reports(
        support::document_markup_with(&body(), &format!("{geometry}{TWO_COLUMNS}")),
        &area,
        4,
    );
    // Balanced: the multi-column section ends at a paragraph-level `w:sectPr`, and the section that
    // follows begins with a `continuous` break — which is the trick every typesetter knows and the
    // rule this engine implements.
    let mut paragraphs = body();
    let last = paragraphs.pop().expect("a paragraph");
    paragraphs.push(support::paragraph(
        &format!("<w:sectPr>{geometry}{TWO_COLUMNS}</w:sectPr>"),
        &strip(&last),
    ));
    paragraphs.push(support::paragraph("", "After the break."));
    let balanced = reports(
        support::document_markup_with(
            &paragraphs,
            &format!(r#"<w:type w:val="continuous"/>{geometry}"#),
        ),
        &area,
        4,
    );

    let unbalanced_first = unbalanced.first().expect("a page").column_heights.clone();
    let balanced_first = balanced.first().expect("a page").column_heights.clone();
    assert_eq!(unbalanced_first.len(), 2);
    assert!(balanced_first.len() >= 2);

    let line = support::measured_line_height();
    let unbalanced_gap = (unbalanced_first[0] - unbalanced_first[1]).absolute();
    let balanced_gap = (balanced_first[0] - balanced_first[1]).absolute();
    assert!(
        balanced_gap <= line,
        "a `continuous` break levels the columns to within one line: {balanced_first:?}"
    );
    assert!(
        unbalanced_gap > line,
        "and without one the last column is short, or this pair asserts nothing: \
         {unbalanced_first:?}"
    );
}

/// A `w:br@type="column"` ends the **column** and not the page, which is the difference MJXOFF-174
/// could not express because it had one column.
#[test]
fn a_column_break_moves_to_the_next_column_rather_than_the_next_page() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let mut paragraphs = vec![support::paragraph_with_run(
        "",
        r#"<w:t xml:space="preserve">Before the break.</w:t><w:br w:type="column"/><w:t xml:space="preserve">After the break.</w:t>"#,
    )];
    paragraphs.push(support::paragraph("", "And a following paragraph."));
    let area = constraints(8.5, 11.0);
    let two = reports(
        support::document_markup_with(&paragraphs, &format!("{geometry}{TWO_COLUMNS}")),
        &area,
        4,
    );
    let one = reports(
        support::document_markup_with(&paragraphs, &geometry),
        &area,
        4,
    );
    assert_eq!(
        two.len(),
        1,
        "with two columns the break stays on the sheet: {two:?}"
    );
    assert_eq!(
        one.len(),
        2,
        "with one column the column *is* the page, so the same break opens a new one: {one:?}"
    );
}

/// An explicit `w:col` list is honoured with its own widths, which is what `w:equalWidth` false
/// means.
#[test]
fn an_explicit_column_list_uses_its_own_widths() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let uneven = r#"<w:cols w:equalWidth="false"><w:col w:w="4320" w:space="360"/><w:col w:w="4680"/></w:cols>"#;
    let markup = support::document_markup_with(&body(), &format!("{geometry}{uneven}"));
    let mut document = support::document_from_bytes(markup);
    let flow = flow(&mut document);
    let sections = flow.formatting().sections();
    let columns = &sections.first().expect("a section").columns;
    assert_eq!(columns.count, 2);
    assert_eq!(columns.columns.len(), 2);
    assert_eq!(columns.columns[0].width_twips, 4320);
    assert_eq!(columns.columns[0].space_after_twips, 360);
    assert_eq!(columns.columns[1].width_twips, 4680);

    let geometry_of = mjx_layout_docx::SectionGeometry::of(
        sections.first(),
        &constraints(8.5, 11.0),
        flow.formatting().settings(),
        1,
    );
    assert_eq!(geometry_of.columns.len(), 2);
    assert_eq!(geometry_of.column(0).width(), Emu::from_twips(4320));
    assert_eq!(geometry_of.column(1).width(), Emu::from_twips(4680));
    assert_eq!(
        geometry_of.column(1).left - geometry_of.column(0).right,
        Emu::from_twips(360),
        "the gap is the first column's own `w:space`, not the `w:cols` one"
    );
}

/// `w:equalWidth="true"` beats an explicit list — ECMA-376 Part 1 §17.6.4, quoted in `mjx-docx`'s
/// own `sections.rs`.
#[test]
fn equal_width_outranks_an_explicit_list() {
    let geometry = support::page_geometry(8.5, 11.0, 1.0);
    let both = r#"<w:cols w:num="3" w:equalWidth="true" w:space="720"><w:col w:w="1000"/><w:col w:w="2000"/></w:cols>"#;
    let markup = support::document_markup_with(&body(), &format!("{geometry}{both}"));
    let mut document = support::document_from_bytes(markup);
    let flow = flow(&mut document);
    let sections = flow.formatting().sections();
    let columns = &sections.first().expect("a section").columns;
    assert_eq!(columns.count, 3, "`w:num` wins");
    assert!(
        columns.columns.is_empty(),
        "and the explicit list is dropped rather than carried alongside, so nothing downstream can \
         read the losing half by accident"
    );
}

/// The text of a paragraph the support module built, so a fixture can rebuild it with different
/// properties.
fn strip(paragraph: &str) -> String {
    let start = paragraph.find("preserve\">").map_or(0, |at| at + 10);
    let end = paragraph[start..]
        .find("</w:t>")
        .map_or(paragraph.len(), |at| start + at);
    paragraph[start..end].to_owned()
}
