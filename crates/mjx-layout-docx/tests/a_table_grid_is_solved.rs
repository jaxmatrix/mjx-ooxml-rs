//! Auto-fit as a **solve**, fixed as a reading, nested tables, and the spans.
//!
//! # Why the two algorithms are compared against each other rather than against a number
//!
//! Nobody has run Word, so *the right column width* is not available to this suite. What **is**
//! available is that the two algorithms are genuinely different functions of the same input: fixed
//! reads `w:tblGrid` and stops, auto-fit measures the content and distributes. So every assertion
//! here is either
//!
//! * a **relation** between the two — the same table, the same content, one attribute changed, and
//!   different numbers; or
//! * a **property of the solve** that no heuristic satisfies by accident: a column whose content is
//!   wider gets more of the table than one whose content is narrower, and the columns still sum to
//!   the table's own width.
//!
//! An engine that returned the declared grid for both would fail the first. An engine that split the
//! width evenly would fail the second.

mod support;

use mjx_layout::BoxModel;
use mjx_layout_docx::{BlockLayout, DocumentFlow, TableLayout};
use mjx_layout_docx::{LayoutRequest, TableContext};
use mjx_ooxml_core::measure::Emu;
use support::{cell, constraints, document, flow, model, paragraph, row, table, text_cell};

/// The table that is the only block of `markup`, laid out at `width_inches`.
fn solved(markup: String, width_inches: f64) -> TableLayout {
    let mut document = document(&[markup]);
    let flow: DocumentFlow = flow(&mut document);
    let mut engine = model();
    // `DocumentBoxModel::lay_out_block` is the same call the paginator makes, so this measures what
    // a page would actually place rather than a second implementation of it.
    let laid = engine
        .lay_out_block(
            &flow,
            0,
            LayoutRequest {
                width: Emu::from_inches(width_inches),
                top: Emu::ZERO,
                exclusions: &[],
            },
        )
        .expect("the table lays out");
    match laid {
        BlockLayout::Table(table) => *table,
        BlockLayout::Paragraph(_) => panic!("block 0 is the table"),
    }
}

/// A two-column table whose first column's content is much wider than its second's.
fn lopsided(layout: &str) -> String {
    let properties = format!("{layout}<w:tblW w:w=\"7200\" w:type=\"dxa\"/>");
    table(
        &properties,
        &[3600, 3600],
        &[row(
            "",
            &[
                text_cell(
                    "",
                    "a considerably longer stretch of words that needs a great deal of room",
                ),
                text_cell("", "short"),
            ],
        )],
    )
}

#[test]
fn auto_fit_gives_the_wider_content_the_wider_column() {
    let solved = solved(lopsided(""), 6.0);
    assert_eq!(solved.columns.len(), 2);
    assert!(
        solved.columns[0] > solved.columns[1],
        "the column with more content is wider: {:?}",
        solved.columns
    );
    // And the declared grid said the two were equal, so this cannot have come from `w:tblGrid`.
    assert_ne!(
        solved.columns[0], solved.columns[1],
        "an auto-fit table does not simply return its declared grid"
    );
}

#[test]
fn a_fixed_table_returns_its_declared_grid_and_auto_fit_does_not() {
    let fixed = solved(lopsided(r#"<w:tblLayout w:type="fixed"/>"#), 6.0);
    let automatic = solved(lopsided(r#"<w:tblLayout w:type="autofit"/>"#), 6.0);

    assert_eq!(
        fixed.columns[0], fixed.columns[1],
        "a fixed table keeps the equal columns its grid declared: {:?}",
        fixed.columns
    );
    assert_ne!(
        fixed.columns, automatic.columns,
        "the same content under the two algorithms must not produce the same widths"
    );
    assert!(
        fixed.fixed,
        "the fixed table reports which algorithm it used"
    );
    assert!(!automatic.fixed);
}

#[test]
fn the_solved_columns_sum_to_the_table_the_file_asked_for() {
    let solved = solved(lopsided(""), 6.0);
    // `w:tblW` asked for 7200 twips — five inches.
    assert_eq!(
        solved.width(),
        Emu::from_twips(7200),
        "integer division must not leave the table a hairline narrow: {:?}",
        solved.columns
    );
}

#[test]
fn a_grid_span_covers_its_columns() {
    let markup = table(
        "",
        &[2000, 2000, 2000],
        &[
            row(
                "",
                &[
                    cell(
                        r#"<w:gridSpan w:val="2"/>"#,
                        &paragraph("", "two columns wide"),
                    ),
                    text_cell("", "one"),
                ],
            ),
            row(
                "",
                &[text_cell("", "a"), text_cell("", "b"), text_cell("", "c")],
            ),
        ],
    );
    let solved = solved(markup, 6.0);
    let spanned = &solved.rows[0].cells[0];
    assert_eq!(spanned.span, 2, "the cell reports its own span");
    assert_eq!(
        spanned.width,
        solved.columns[0] + solved.columns[1],
        "and it is as wide as the two columns it covers"
    );
    assert_eq!(
        solved.rows[0].cells[1].column, 2,
        "the cell after a span starts at the column the span ended on"
    );
}

#[test]
fn a_nested_table_lays_out_inside_its_cell() {
    let inner = table(
        "",
        &[1000, 1000],
        &[row(
            "",
            &[text_cell("", "inner a"), text_cell("", "inner b")],
        )],
    );
    let outer = table(
        "",
        &[3000, 3000],
        &[row("", &[cell("", &inner), text_cell("", "outer right")])],
    );
    let solved = solved(outer, 6.0);
    let cell = &solved.rows[0].cells[0];
    let nested = cell
        .content
        .first()
        .expect("the cell holds something")
        .layout
        .as_table()
        .expect("and it is a table");
    assert_eq!(nested.rows.len(), 1);
    assert_eq!(nested.columns.len(), 2);
    assert!(
        nested.width() <= cell.content_width,
        "the nested table fits inside its cell: {:?} in {:?}",
        nested.width(),
        cell.content_width
    );
    assert!(
        solved.rows[0].height >= nested.height(),
        "and the outer row is at least as tall as what is inside it"
    );
}

#[test]
fn a_table_three_levels_deep_still_lays_out() {
    // The ticket asks for two levels; this asserts the recursion does not stop there, because a
    // depth limit that happened to be two would pass a two-level assertion.
    let innermost = table("", &[800], &[row("", &[text_cell("", "deepest")])]);
    let middle = table("", &[1600], &[row("", &[cell("", &innermost)])]);
    let outer = table("", &[3200], &[row("", &[cell("", &middle)])]);
    let solved = solved(outer, 6.0);
    let level_two = solved.rows[0].cells[0]
        .content
        .first()
        .expect("a nested table")
        .layout
        .as_table()
        .expect("level two");
    let level_three = level_two.rows[0].cells[0]
        .content
        .first()
        .expect("a doubly nested table")
        .layout
        .as_table()
        .expect("level three");
    assert_eq!(level_three.rows.len(), 1);
    assert!(level_three.height() > Emu::ZERO);
}

#[test]
fn a_row_height_rule_changes_the_row() {
    let make = |rule: &str| {
        let properties = if rule.is_empty() {
            String::new()
        } else {
            format!(r#"<w:trHeight w:val="2000" w:hRule="{rule}"/>"#)
        };
        table(
            "",
            &[4000],
            &[row(&properties, &[text_cell("", "one short line")])],
        )
    };
    let automatic = solved(make(""), 6.0);
    let at_least = solved(make("atLeast"), 6.0);
    let exact = solved(make("exact"), 6.0);

    assert!(
        at_least.rows[0].height > automatic.rows[0].height,
        "atLeast raises a row shorter than the request"
    );
    assert_eq!(
        at_least.rows[0].height,
        Emu::from_twips(2000),
        "and it raises it to exactly the request"
    );
    assert_eq!(
        exact.rows[0].height,
        Emu::from_twips(2000),
        "exact states the height whatever the content is"
    );
}

#[test]
fn a_cell_margin_narrows_its_content() {
    // A **fixed** table, so that the column width is the file's and the margin can only come out of
    // the text. Under auto-fit the same change makes the *column* wider instead and the content keeps
    // its width, which is the algorithm working rather than the margin being ignored.
    let make = |margins: &str| {
        table(
            &format!(r#"<w:tblLayout w:type="fixed"/><w:tblW w:w="4000" w:type="dxa"/>{margins}"#),
            &[4000],
            &[row("", &[text_cell("", "content")])],
        )
    };
    let plain = solved(make(""), 6.0);
    let inset = solved(
        make(
            r#"<w:tblCellMar><w:left w:w="720" w:type="dxa"/><w:right w:w="720" w:type="dxa"/></w:tblCellMar>"#,
        ),
        6.0,
    );
    assert!(
        inset.rows[0].cells[0].content_width < plain.rows[0].cells[0].content_width,
        "a wider cell margin leaves less room for text: {:?} vs {:?}",
        inset.rows[0].cells[0].content_width,
        plain.rows[0].cells[0].content_width
    );
}

#[test]
fn the_table_context_is_reachable() {
    // The public surface a caller outside this crate would use to lay a table out itself.
    let mut document = document(&[table("", &[2000], &[row("", &[text_cell("", "x")])])]);
    let formatting = document.formatting().expect("it resolves");
    let context = TableContext {
        available: Emu::from_inches(4.0),
        paragraphs: formatting.paragraphs(),
        settings: formatting.settings(),
    };
    assert_eq!(context.available, Emu::from_inches(4.0));
    let request = LayoutRequest {
        width: Emu::from_inches(4.0),
        top: Emu::ZERO,
        exclusions: &[],
    };
    assert!(request.exclusions.is_empty());
    let _ = constraints(8.5, 11.0);
}

#[test]
fn a_cell_vertical_alignment_moves_its_content_down_the_row() {
    // A tall row — `w:trHeight` fixes it — with one short line in each cell, aligned three ways. The
    // assertion is on **where the line's baseline is**, because vertical alignment changes nothing
    // else at all: the same glyphs, the same widths, a different y.
    let make = |alignment: &str| {
        let properties = if alignment.is_empty() {
            String::new()
        } else {
            format!(r#"<w:vAlign w:val="{alignment}"/>"#)
        };
        table(
            "",
            &[4000],
            &[row(
                r#"<w:trHeight w:val="2000" w:hRule="exact"/>"#,
                &[cell(&properties, &paragraph("", "one short line"))],
            )],
        )
    };

    let baseline = |markup: String| {
        let mut document = document(&[markup]);
        let flow = flow(&mut document);
        let mut engine = model();
        let page = engine
            .layout_page(
                &flow,
                mjx_layout::PageIndex::FIRST,
                &constraints(8.5, 11.0),
                None,
            )
            .expect("page one lays out");
        let found = page
            .fragments()
            .nodes()
            .find_map(|(_, node)| match node.fragment() {
                mjx_layout::Fragment::Line(_) => Some(node.rect().top.emu()),
                _ => None,
            });
        found.expect("the cell's line is drawn")
    };

    let top = baseline(make("top"));
    let centred = baseline(make("center"));
    let bottom = baseline(make("bottom"));
    assert!(
        top < centred && centred < bottom,
        "the three alignments put the line in three places: {top} / {centred} / {bottom}"
    );
    assert_eq!(
        top,
        baseline(make("")),
        "and an absent w:vAlign is `top`, which is §17.4.83's own default"
    );
}
