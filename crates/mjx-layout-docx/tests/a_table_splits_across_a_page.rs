//! A table that **must** break, and what breaking it changes.
//!
//! # The trap this suite exists to avoid
//!
//! **A table that fits on one page is laid out identically by an implementation that cannot split a
//! table at all**, and every assertion about its cells is green for that implementation. So every
//! fixture here is taller than its page, and every assertion is on **which rows land on which page**
//! rather than on what a page contains.
//!
//! The same reasoning applies twice more:
//!
//! * `w:tblHeader` is **invisible unless the table splits** — a heading row that repeats on one page
//!   is a heading row — so the assertion is that row 0 appears on page one *and* on page two.
//! * `w:cantSplit` is an **ordering constraint** and not an appearance: a row that may not break
//!   moves whole to the next page. The assertion is the page assignment of its last row, taken with
//!   the attribute on and with it off, and the two must differ.

mod support;

use mjx_layout_docx::DocumentFlow;
use support::{
    cells_on, constraints, document, flow, model, page_of_each_paragraph, paragraph, row, rows_on,
    table, text_cell, walk,
};

/// A row whose cells hold `lines` lines of text each, so a table's height is a number the fixture
/// states rather than one it hopes for.
fn tall_row(properties: &str, label: &str, lines: usize) -> String {
    let body: String = (0..lines)
        .map(|line| paragraph("", &format!("{label} line {line}")))
        .collect();
    row(
        properties,
        &[
            support::cell("", &body),
            support::cell("", &paragraph("", label)),
        ],
    )
}

/// Eight rows of four lines each on a page that holds far fewer — a table that cannot help splitting.
fn splitting_table(row_properties: &[&str]) -> String {
    let rows: Vec<String> = row_properties
        .iter()
        .enumerate()
        .map(|(index, properties)| tall_row(properties, &format!("row{index}"), 4))
        .collect();
    table("", &[4000, 4000], &rows)
}

#[test]
fn a_table_taller_than_its_page_puts_its_rows_on_more_than_one() {
    let markup = splitting_table(&[""; 8]);
    let mut document = document(&[markup]);
    let flow = flow(&mut document);
    let mut model = model();
    let constraints = constraints(8.5, 4.0);
    let pages = walk(&mut model, &flow, &constraints, 12);

    assert!(
        pages.len() > 1,
        "the fixture must split; it produced {} page(s)",
        pages.len()
    );
    let first = rows_on(pages[0].fragments());
    let second = rows_on(pages[1].fragments());
    assert!(!first.is_empty(), "page one holds some of the table");
    assert!(!second.is_empty(), "page two holds the rest");
    assert!(
        first.last() <= second.first(),
        "rows are placed in order: page one ended at {first:?} and page two began at {second:?}"
    );
    // The union covers every row exactly once at its *start* — a row split across the boundary is on
    // both, which is why this asserts coverage rather than disjointness.
    let mut seen: Vec<u32> = first.iter().chain(second.iter()).copied().collect();
    for page in pages.iter().skip(2) {
        seen.extend(rows_on(page.fragments()));
    }
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen, (0..8).collect::<Vec<u32>>(), "every row is placed");
}

#[test]
fn a_heading_row_is_drawn_on_every_page_the_table_reaches() {
    // Row 0 repeats; the rest do not.
    let mut properties = vec![r#"<w:tblHeader/>"#];
    properties.extend(std::iter::repeat_n("", 7));
    let mut heading = document(&[splitting_table(&properties)]);
    let heading_flow = flow(&mut heading);
    let mut engine = model();
    let constraints = constraints(8.5, 4.0);
    let pages = walk(&mut engine, &heading_flow, &constraints, 12);
    assert!(pages.len() > 1, "the fixture must split");

    for (number, page) in pages.iter().enumerate() {
        let rows = rows_on(page.fragments());
        if rows.is_empty() {
            continue;
        }
        assert!(
            rows.contains(&0),
            "page {number} holds table rows {rows:?} and must repeat the heading row"
        );
    }

    // And the control: without `w:tblHeader`, row 0 is on page one and nowhere else. Without this
    // half the assertion above is green for an engine that simply never splits.
    let mut plain = document(&[splitting_table(&[""; 8])]);
    let plain_flow = flow(&mut plain);
    let mut engine = model();
    let plain_pages = walk(&mut engine, &plain_flow, &constraints, 12);
    let later: Vec<u32> = plain_pages
        .iter()
        .skip(1)
        .flat_map(|page| rows_on(page.fragments()))
        .collect();
    assert!(
        !later.contains(&0),
        "with no w:tblHeader the first row must not reappear; later pages held {later:?}"
    );
}

#[test]
fn a_row_that_may_not_split_moves_whole() {
    // A single row tall enough to straddle the boundary, with a short row before it so that the
    // boundary really does fall inside it.
    let rows = |cant_split: bool| {
        let flag = if cant_split { r#"<w:cantSplit/>"# } else { "" };
        vec![
            tall_row("", "first", 2),
            tall_row(flag, "second", 30),
            tall_row("", "third", 2),
        ]
    };
    let page = constraints(8.5, 3.0);

    let mut splittable = document(&[table("", &[4000, 4000], &rows(false))]);
    let splittable_flow = flow(&mut splittable);
    let mut engine = model();
    let splittable_pages = walk(&mut engine, &splittable_flow, &page, 12);

    let mut whole = document(&[table("", &[4000, 4000], &rows(true))]);
    let whole_flow = flow(&mut whole);
    let mut engine = model();
    let whole_pages = walk(&mut engine, &whole_flow, &page, 12);

    let straddles = |pages: &[mjx_layout::PageFragments]| {
        pages
            .iter()
            .filter(|page| rows_on(page.fragments()).contains(&1))
            .count()
    };
    assert_eq!(
        straddles(&splittable_pages),
        2,
        "without w:cantSplit the middle row is drawn on two pages"
    );
    assert_eq!(
        straddles(&whole_pages),
        1,
        "with w:cantSplit the middle row is drawn on exactly one"
    );
}

#[test]
fn a_vertically_merged_cell_crosses_the_break() {
    // Column 0 is one merged cell across all six rows; column 1 has its own text per row. The merge
    // therefore *has* to cross whatever page boundary the table's height produces.
    let mut rows = Vec::new();
    for index in 0..24 {
        let merge = if index == 0 {
            r#"<w:vMerge w:val="restart"/>"#
        } else {
            r#"<w:vMerge/>"#
        };
        let left = if index == 0 {
            support::cell(merge, &paragraph("", "merged"))
        } else {
            support::cell(merge, &paragraph("", ""))
        };
        let _ = index;
        rows.push(row("", &[left, text_cell("", &format!("right {index}"))]));
    }
    let mut document = document(&[table("", &[4000, 4000], &rows)]);
    let flow = flow(&mut document);
    let mut model = model();
    let constraints = constraints(8.5, 3.0);
    let pages = walk(&mut model, &flow, &constraints, 12);
    assert!(pages.len() > 1, "the fixture must split");

    // The merged column is addressed as column 0 on every page it reaches, and the continuation
    // cells carry the row they are actually in — which is what makes a hit test on page two answer
    // "row 4" rather than "row 0".
    for (number, page) in pages.iter().enumerate() {
        let cells = cells_on(page.fragments());
        if cells.is_empty() {
            continue;
        }
        assert!(
            cells.iter().any(|cell| cell.1 == 0),
            "page {number} holds the merged column"
        );
    }
    let rows_seen: Vec<u32> = pages
        .iter()
        .flat_map(|page| rows_on(page.fragments()))
        .collect();
    assert!(
        rows_seen.contains(&0) && rows_seen.contains(&23),
        "the merge's first and last rows are both placed: {rows_seen:?}"
    );
}

#[test]
fn a_table_between_two_paragraphs_moves_the_paragraph_after_it() {
    // The gate that says a table takes up space at all: the same two paragraphs, with and without a
    // tall table between them, and the second paragraph on a different page.
    let after = paragraph("", "after the table");
    let without = [paragraph("", "before the table"), after.clone()];
    let with = [
        paragraph("", "before the table"),
        splitting_table(&[""; 8]),
        after,
    ];
    let constraints = constraints(8.5, 4.0);

    let mut plain = document(&without);
    let plain_flow: DocumentFlow = flow(&mut plain);
    let mut engine = model();
    let plain_pages = page_of_each_paragraph(&mut engine, &plain_flow, &constraints, 12);

    let mut tabled = document(&with);
    let tabled_flow = flow(&mut tabled);
    let mut engine = model();
    let tabled_pages = page_of_each_paragraph(&mut engine, &tabled_flow, &constraints, 12);

    assert_eq!(plain_pages.first().copied().flatten(), Some(0));
    assert_eq!(
        plain_pages.get(1).copied().flatten(),
        Some(0),
        "with no table the second paragraph is on page one"
    );
    assert!(
        tabled_pages.get(2).copied().flatten().unwrap_or(0) > 0,
        "with a table between them the second paragraph is pushed off page one: {tabled_pages:?}"
    );
}
