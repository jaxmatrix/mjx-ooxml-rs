//! The contract itself: what a band of a sheet produces, how the four `BoxModel` methods answer,
//! how a cell's fragments say which cell they came from, and what auto-fit measures.

mod support;

use mjx_layout::{
    BoxModel, ChangeKind, ChangeSet, ContentChange, DirtyPages, ExtentPrecision, Fragment,
    PageIndex, SourcePath, SourceRef,
};
use mjx_layout_xlsx::{CellHit, SheetBoxModel, SheetGrid, SheetLayoutError};
use mjx_ooxml_core::measure::Emu;

use support::{grid_from, model, styles, viewport};

/// A small sheet with text in three cells.
const SHEET: &str = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="12"/>
<sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>Quarter</t></is></c><c r="B1" t="inlineStr"><is><t>Revenue</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Q1</t></is></c><c r="B2"><v>1200</v></c></row>
</sheetData>"#;

#[test]
fn a_band_is_a_page_box_a_table_and_the_cells_inside_it() {
    let grid = grid_from(SHEET, &styles(&[], ""));
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);

    assert_eq!(tree.roots().len(), 1, "one page box");
    assert_eq!(
        support::tables(&tree).len(),
        1,
        "one pane region, so one table"
    );
    let cells = support::cells(&tree);
    assert!(
        cells
            .iter()
            .any(|(row, column, _)| *row == 0 && *column == 0),
        "A1 is there"
    );
    assert!(
        cells
            .iter()
            .any(|(row, column, _)| *row == 1 && *column == 1),
        "B2 is there"
    );

    // A table fragment says what a reader needs to be told — how wide the grid is and which rows are
    // on this page — which is what makes a sheet's continuation announceable.
    let table = tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::Table(table) => Some(table.clone()),
            _ => None,
        })
        .expect("a table fragment");
    assert!(table.columns > 1);
    assert_eq!(table.rows.start, 0);
    assert!(!table.continued_from_previous_page);
}

#[test]
fn every_fragment_of_a_cell_addresses_that_cell_and_nothing_else() {
    let grid = grid_from(SHEET, &styles(&[], ""));
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);

    let mut boxes = 0;
    let mut glyphs = 0;
    for (_, node) in tree.nodes() {
        let Some(hit) = CellHit::from_source(node.source()) else {
            // The page box and the pane's table carry the sheet's own path and name no cell.
            assert!(
                node.source().path().is_root(),
                "only structural fragments name no cell: {:?}",
                node.source()
            );
            continue;
        };
        assert_eq!(hit.sheet, 0, "one tab, part zero");
        assert!(hit.reference().is_ok(), "every hit is a real reference");
        match node.fragment() {
            Fragment::Box(_) => {
                assert!(hit.offset.is_none(), "a cell's box covers no characters");
                boxes += 1;
            }
            Fragment::GlyphRun(_) => {
                assert!(
                    hit.offset.is_some(),
                    "a glyph run lands on an offset in the cell's own text"
                );
                glyphs += 1;
            }
            _ => {}
        }
    }
    assert!(
        boxes > 0 && glyphs > 0,
        "{boxes} boxes, {glyphs} glyph runs"
    );
}

#[test]
fn the_addressing_agrees_with_the_sessions_own() {
    // `mjx-session`'s `SpreadsheetSession` documents the scheme as: the part is the tab, path segment
    // 0 is the zero-based row and segment 1 the zero-based column. The two crates never depend on
    // each other — 3.5 and 3.6 are sideways — so the agreement is held by both writing it down and
    // by this test reading the *other* crate's source to check the sentence is still there.
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../mjx-session/src/ooxml/spreadsheet.rs"),
    )
    .expect("mjx-session's spreadsheet residency");
    assert!(
        source.contains("path segment `0`") && source.contains("the zero-based row"),
        "the session still documents segment 0 as the row"
    );
    assert!(
        source.contains("path segment `1`") && source.contains("the zero-based column"),
        "and segment 1 as the column"
    );
    assert!(
        source.contains("[6, 2]") && source.contains("C7"),
        "and gives the same worked example this crate's `address` module does"
    );

    // And the fragments really do use it: C7 is row 6, column 2.
    let grid = grid_from(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="12"/>
<sheetData><row r="7"><c r="C7" t="inlineStr"><is><t>here</t></is></c></row></sheetData>"#,
        &styles(&[], ""),
    );
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);
    let hit = tree
        .nodes()
        .filter_map(|(_, node)| CellHit::from_source(node.source()))
        .find(|hit| hit.offset.is_some())
        .expect("a glyph run");
    assert_eq!((hit.row, hit.column), (6, 2));
    assert_eq!(
        hit.reference().expect("a reference").text().as_str(),
        "C7",
        "which round-trips back to the address the file wrote"
    );
}

#[test]
fn the_extent_is_exact_because_no_row_height_is_recomputed() {
    let grid = grid_from(SHEET, &styles(&[], ""));
    let constraints = viewport(6.0, 3.0);
    let model = model();
    let extent = model.estimate_extent(&grid, &constraints);

    assert_eq!(extent.precision, ExtentPrecision::Exact);
    assert_eq!(extent.pages, 1, "two rows fit in three inches");
    assert_eq!(extent.page_size, constraints.page);

    // A taller sheet is more bands, and the count comes from the row geometry rather than from
    // laying anything out.
    let mut rows = String::new();
    for row in 1..=200 {
        rows.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}" t="inlineStr"><is><t>r</t></is></c></row>"#
        ));
    }
    let tall = grid_from(
        &format!(
            r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="12"/><sheetData>{rows}</sheetData>"#
        ),
        &styles(&[], ""),
    );
    let tall_extent = model.estimate_extent(&tall, &constraints);
    assert!(
        tall_extent.pages > 10,
        "two hundred rows is many bands: {}",
        tall_extent.pages
    );
}

#[test]
fn a_band_laid_out_alone_matches_the_same_band_reached_through_every_checkpoint() {
    // The equivalence the contract asks for. It is structural here — a band's rows come from the
    // page index and the row geometry, and never from the checkpoint — but "structural" is a
    // property of this implementation rather than of the trait, so it is asserted anyway.
    let mut rows = String::new();
    for row in 1..=200 {
        rows.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}" t="inlineStr"><is><t>row {row}</t></is></c></row>"#
        ));
    }
    let grid = grid_from(
        &format!(
            r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="12"/><sheetData>{rows}</sheetData>"#
        ),
        &styles(&[], ""),
    );
    let constraints = viewport(6.0, 2.0);

    let mut stepped = model();
    let through_checkpoints = support::lay_out(&mut stepped, &grid, &constraints, 4);

    let mut direct = model();
    let alone = direct
        .layout_page(&grid, PageIndex::new(4), &constraints, None)
        .expect("band four alone")
        .into_parts()
        .0;

    assert_eq!(
        support::snapshot(&through_checkpoints),
        support::snapshot(&alone),
        "reaching a band through four checkpoints and laying it out alone must give the same page"
    );
}

#[test]
fn a_checkpoint_from_another_box_model_is_refused() {
    let grid = grid_from(SHEET, &styles(&[], ""));
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    let foreign = mjx_layout::Checkpoint::new(
        mjx_layout::ModelSignature::new(0x0102_0304_0506_0708),
        PageIndex::FIRST,
        SourceRef::node(mjx_layout::PartId::PRIMARY, SourcePath::root()),
        vec![0; 8],
    )
    .expect("a checkpoint");
    let error = model
        .layout_page(&grid, PageIndex::new(1), &constraints, Some(&foreign))
        .expect_err("a foreign checkpoint is refused");
    assert!(
        matches!(error, SheetLayoutError::Layout(_)),
        "and refused by the shared machinery rather than by this crate: {error}"
    );
}

#[test]
fn a_continuation_carrying_the_wrong_number_of_bytes_is_refused() {
    // The perturbation that reaches this crate's own variant: the right signature, the right page,
    // the wrong state.
    let mut rows = String::new();
    for row in 1..=200 {
        rows.push_str(&format!(
            r#"<row r="{row}"><c r="A{row}"><v>1</v></c></row>"#
        ));
    }
    let grid = grid_from(
        &format!(
            r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="12"/><sheetData>{rows}</sheetData>"#
        ),
        &styles(&[], ""),
    );
    let constraints = viewport(6.0, 2.0);
    let mut model = model();
    let malformed = mjx_layout::Checkpoint::new(
        SheetBoxModel::SIGNATURE,
        PageIndex::FIRST,
        SourceRef::node(mjx_layout::PartId::PRIMARY, SourcePath::root()),
        vec![0; 3],
    )
    .expect("a checkpoint");
    let error = model
        .layout_page(&grid, PageIndex::new(1), &constraints, Some(&malformed))
        .expect_err("a malformed state is refused");
    assert!(
        matches!(error, SheetLayoutError::MalformedContinuation(3)),
        "{error}"
    );
}

#[test]
fn an_edit_to_a_cell_dirties_the_band_it_is_in_and_a_row_height_dirties_everything_below() {
    let grid = grid_from(SHEET, &styles(&[], ""));
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);

    let mut reformat = ChangeSet::new();
    reformat.record(ContentChange {
        source: SourceRef::node(grid.part(), SourcePath::new(&[0, 0])),
        kind: ChangeKind::Reformatted,
    });
    assert_eq!(
        model.invalidate(&reformat),
        DirtyPages::Pages(vec![PageIndex::FIRST]),
        "reformatting a cell dirties the band it is on and no other"
    );

    // A change to the *sheet* — a pane, a default height, a column width — has no row segment.
    let mut structural = ChangeSet::new();
    structural.record(ContentChange {
        source: SourceRef::node(grid.part(), SourcePath::root()),
        kind: ChangeKind::Reformatted,
    });
    assert_eq!(
        model.invalidate(&structural),
        DirtyPages::All,
        "and a change to the sheet itself moves every band"
    );

    // Another tab shares no page numbering with this one.
    let mut elsewhere = ChangeSet::new();
    elsewhere.record(ContentChange {
        source: SourceRef::node(mjx_layout::PartId::new(7), SourcePath::new(&[0, 0])),
        kind: ChangeKind::Reformatted,
    });
    assert_eq!(model.invalidate(&elsewhere), DirtyPages::None);
    assert_eq!(model.invalidate(&ChangeSet::new()), DirtyPages::None);
}

#[test]
fn auto_fit_measures_a_known_string_and_answers_a_number() {
    // The ticket asks for auto-fit width "asserted as a number against a known string". The number
    // has to come from somewhere other than this code, so the assertion is relational and
    // arithmetic: a column of `WIDE` must fit `WIDE`, must be wider than a column of `NARROW`, and
    // must be wider by about the ratio of the two strings' lengths.
    const NARROW: &str = "Q1";
    const WIDE: &str = "Quarterly revenue by region";

    let body = format!(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetData>
<row r="1"><c r="A1" t="inlineStr"><is><t>{NARROW}</t></is></c><c r="B1" t="inlineStr"><is><t>{WIDE}</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>{NARROW}</t></is></c></row>
</sheetData>"#
    );
    let grid = grid_from(&body, &styles(&[], ""));
    let mut model = model();

    let narrow = model.auto_fit_width(&grid, 0).expect("A fits");
    let wide = model.auto_fit_width(&grid, 1).expect("B fits");

    assert_eq!(narrow.cells_measured, 2, "both A1 and A2 were measured");
    assert!(narrow.sampled_every_cell, "and none was skipped");
    assert_eq!(wide.cells_measured, 1);

    assert!(wide.width > narrow.width, "{wide:?} against {narrow:?}");
    assert!(
        narrow.width > Emu::ZERO,
        "a two-character column is still a column"
    );

    // The ratio of the fitted widths tracks the ratio of the strings, within the padding each
    // carries. Twenty-seven characters against two is roughly thirteen times; the padding pulls that
    // down, so the assertion is a band rather than a point.
    #[allow(clippy::cast_precision_loss)]
    let ratio = wide.width.emu() as f64 / narrow.width.emu().max(1) as f64;
    assert!(
        (3.0..14.0).contains(&ratio),
        "a 27-character label fits a column several times the width of a 2-character one: {ratio}"
    );

    // And the answer is expressible in the unit `col@width` is written in, so a caller that wants to
    // *store* the fit has the number the file takes.
    assert!(wide.characters > narrow.characters);
    assert!(wide.characters <= mjx_layout_xlsx::autofit::MAXIMUM_FITTED_CHARACTERS);

    // Memoised: asking twice measures once.
    let again = model.auto_fit_width(&grid, 1).expect("B fits again");
    assert_eq!(again, wide);
}

#[test]
fn a_sheet_the_workbook_does_not_have_is_an_error_rather_than_a_panic() {
    let book = mjx_xlsx::Workbook::blank().expect("a blank workbook");
    let error = SheetGrid::read(&book, 9).expect_err("there is no tab nine");
    assert!(
        matches!(
            error,
            SheetLayoutError::NoSuchSheet {
                requested: 9,
                count: 1
            }
        ),
        "{error}"
    );
}

#[test]
fn a_best_fit_column_is_reported_rather_than_resized() {
    // `col@bestFit="1"` asks a consumer to size the column to its content — and the column also
    // *states a width*, which is the one Excel computed when it last did exactly that. Honouring the
    // stored width reproduces what the author saw; the fit is reported beside it so a
    // *Format -> AutoFit Column Width* command has a number to apply.
    let body = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<cols><col min="1" max="1" width="9" bestFit="true" customWidth="true"/></cols>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Quarterly revenue by region</t></is></c></row></sheetData>"#;
    let grid = grid_from(body, &styles(&[], ""));
    let constraints = viewport(6.0, 3.0);
    let mut model = model();
    let stated = model
        .geometry(&grid)
        .expect("the geometry")
        .columns()
        .width(0);
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);

    let fits = model.catalogue().auto_fits();
    assert_eq!(fits.len(), 1, "column A asked to be fitted: {fits:?}");
    let (column, fit) = fits[0];
    assert_eq!(column, 0);
    assert!(
        fit.width > stated,
        "the label is much wider than the nine characters the file states: {fit:?}"
    );

    // And the geometry still says what the file says.
    assert_eq!(
        model
            .geometry(&grid)
            .expect("the geometry")
            .columns()
            .width(0),
        stated,
        "reporting a fit must not resize the column underneath the reader"
    );
}
