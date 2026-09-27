//! A table whose merges a row-and-column walk gets wrong.
//!
//! # The defect, exactly
//!
//! `docs/TABLES_HANDOFF.md`'s first decision is that **merging never removes a cell**: a merged
//! region is anchored at its top-left `a:tc`, which states `@gridSpan` / `@rowSpan`, and every
//! position it covers stays in the file stating `@hMerge` / `@vMerge`. A renderer that walks the
//! rows and draws each `a:tc` at the rectangle of its own row and column therefore draws **all** of
//! them: a 1 × 3 merge becomes three unmerged cells, the anchor's text is clipped at the first
//! column's edge, and the two covered cells draw their own hidden text over the top.
//!
//! It looks nearly right — a table with lines in the wrong places — which is why the assertions
//! below are on *rectangles and counts* rather than on "something was drawn".
//!
//! # How many distinct values this gate sees
//!
//! One merge proves one arm. The table below carries **four** different shapes at once — a
//! horizontal merge, a vertical merge, a rectangular merge that is both, and unmerged cells beside
//! all three — because a walk that handled `@gridSpan` and ignored `@rowSpan` would pass a suite
//! that only merged across.

mod support;

use mjx_layout::{Fragment, FragmentTree, TableCell};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{Cells, Presentation, ShapeBounds};

/// The table's frame: four inches wide from half an inch in, so every number below is a round one.
fn frame() -> ShapeBounds {
    ShapeBounds::from_inches(0.5, 0.5, 6.0, 4.0)
}

/// A 4 × 3 table with four different merge shapes in it.
///
/// ```text
///        col 0        col 1        col 2
/// row 0  [ A ------------------------ ]   one 1x3 horizontal merge
/// row 1  [ B ]      [ C ---- ]            a 2x1 vertical merge and a 1x2 horizontal one
/// row 2  [ | ]      [ D ]   [ E ]         (B continues; C's second row is covered)
/// row 3  [ F ]      [ G ]   [ H ]         nothing merged at all
/// ```
fn merged_table() -> (Presentation, usize) {
    let (mut deck, slide) = support::blank_deck();
    let table = deck.add_table(slide, 4, 3, frame()).expect("a table");

    for row in 0..4 {
        for column in 0..3 {
            deck.set_cell_text(slide, table, row, column, 0, &format!("r{row}c{column}"))
                .expect("cell text");
        }
    }

    // Row 0, all three columns: a purely horizontal merge.
    deck.merge_cells(slide, table, Cells::rectangle(0..1, 0..3))
        .expect("the header merge");
    // Rows 1–2 of column 0: a purely vertical merge. A walk that reads `@gridSpan` and not
    // `@rowSpan` draws this one twice, at half its height each time.
    deck.merge_cells(slide, table, Cells::rectangle(1..3, 0..1))
        .expect("the vertical merge");
    // Row 1, columns 1–2: horizontal again, but *not* in the first row, so a walk that special-cased
    // the header would still be wrong here.
    deck.merge_cells(slide, table, Cells::rectangle(1..2, 1..3))
        .expect("the second horizontal merge");

    (deck, slide)
}

/// Every cell fragment of `tree`, with the rectangle it was drawn at.
fn cells(tree: &FragmentTree) -> Vec<(TableCell, mjx_layout::LayoutRect)> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::Box(box_fragment) => box_fragment.cell.map(|cell| (cell, node.rect())),
            _ => None,
        })
        .collect()
}

#[test]
fn only_the_cells_that_render_become_fragments() {
    let (mut deck, slide) = merged_table();
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);
    let cells = cells(&tree);

    // Twelve grid positions. The three merges cover 2 + 1 + 1 = 4 of them, so eight render.
    assert_eq!(
        cells.len(),
        8,
        "a 4x3 grid with three merges has eight cells that render and four that are covered; \
         {} fragments were emitted. Twelve means every `a:tc` was drawn, which is the naive walk \
         this suite exists to refuse.",
        cells.len()
    );

    let covered = [(0_u32, 1_u16), (0, 2), (2, 0), (1, 2)];
    for (row, column) in covered {
        assert!(
            !cells
                .iter()
                .any(|(cell, _)| cell.row == row && cell.column == column),
            "the covered position (row {row}, column {column}) became a fragment. It states \
             `@hMerge` or `@vMerge`, keeps its own text (handoff decision 2) and must draw none of \
             it."
        );
    }
}

#[test]
fn a_merged_cell_spans_the_positions_it_covers() {
    let (mut deck, slide) = merged_table();
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);
    let cells = cells(&tree);

    let find = |row: u32, column: u16| {
        cells
            .iter()
            .find(|(cell, _)| cell.row == row && cell.column == column)
            .unwrap_or_else(|| panic!("the cell at row {row}, column {column} renders"))
    };

    let (unmerged, unmerged_rect) = find(3, 0);
    assert_eq!((unmerged.row_span, unmerged.column_span), (1, 1));
    let single_width = unmerged_rect.width();
    let single_height = unmerged_rect.height();
    assert!(
        single_width > Emu::ZERO && single_height > Emu::ZERO,
        "an unmerged cell has to have a size before any comparison below means anything"
    );

    let (header, header_rect) = find(0, 0);
    assert_eq!(
        (header.row_span, header.column_span),
        (1, 3),
        "the header cell states a three-column span"
    );
    assert_eq!(
        header_rect.width(),
        Emu::from_emu(single_width.emu() * 3),
        "the header spans three columns, so its rectangle is three columns wide. Under the naive \
         walk it is one column wide and its text is clipped at the first column's edge."
    );
    assert_eq!(
        header_rect.height(),
        single_height,
        "it spans one row, so it is one row tall"
    );

    let (vertical, vertical_rect) = find(1, 0);
    assert_eq!(
        (vertical.row_span, vertical.column_span),
        (2, 1),
        "the vertical merge states a two-row span"
    );
    assert_eq!(
        vertical_rect.height(),
        Emu::from_emu(single_height.emu() * 2),
        "the vertical merge covers two rows, so its rectangle is two rows tall. A walk that read \
         `@gridSpan` and ignored `@rowSpan` draws it at one row's height — which is exactly what a \
         suite that only merged across would never catch."
    );
    assert_eq!(vertical_rect.width(), single_width);

    let (second, second_rect) = find(1, 1);
    assert_eq!((second.row_span, second.column_span), (1, 2));
    assert_eq!(
        second_rect.width(),
        Emu::from_emu(single_width.emu() * 2),
        "the second horizontal merge is not in the header row, so a walk that special-cased the \
         first row would still be wrong here"
    );
}

#[test]
fn the_anchors_text_is_laid_out_across_its_whole_span() {
    // The visible consequence of the rectangle assertions above, taken one step further: the
    // anchor's own glyphs have to be positioned inside the *merged* box, not the single-column one.
    let (mut deck, slide) = merged_table();
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);

    let (_, header_rect) = cells(&tree)
        .into_iter()
        .find(|(cell, _)| cell.row == 0 && cell.column == 0)
        .expect("the header cell renders");

    let header_glyphs: Vec<_> = tree
        .nodes()
        .filter(|(_, node)| matches!(node.fragment(), Fragment::GlyphRun(_)))
        .filter(|(_, node)| {
            let rect = node.rect();
            rect.top >= header_rect.top && rect.bottom <= header_rect.bottom
        })
        .collect();
    assert!(
        !header_glyphs.is_empty(),
        "the header cell's own text has to be drawn, or the rest of this assertion is vacuous"
    );
    for (_, node) in &header_glyphs {
        assert!(
            node.rect().right <= header_rect.right,
            "a glyph run of the header row reaches past the merged cell's right edge"
        );
    }
}

#[test]
fn a_table_with_no_merges_at_all_renders_every_position() {
    // The other direction, so that "eight cells" above is a consequence of the merges rather than
    // of the layout losing four cells for some unrelated reason.
    let (mut deck, slide) = support::blank_deck();
    let table = deck.add_table(slide, 4, 3, frame()).expect("a table");
    for row in 0..4 {
        for column in 0..3 {
            deck.set_cell_text(slide, table, row, column, 0, "x")
                .expect("cell text");
        }
    }
    let mut model = support::model();
    let tree = support::lay_out(&mut model, &mut deck, slide);
    assert_eq!(
        cells(&tree).len(),
        12,
        "a 4x3 grid with nothing merged renders all twelve positions"
    );
}
