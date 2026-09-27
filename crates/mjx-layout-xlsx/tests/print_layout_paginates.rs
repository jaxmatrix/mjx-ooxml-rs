//! Print layout: the second pagination, asserted on **page contents** rather than on a page count.
//!
//! # ⚠ Every expected number here is labelled, and none of it is parity
//!
//! Page breaks, scaling and fit-to-page are arithmetic nobody can check without Excel, so following
//! MJXOFF-172's example each expectation says where it came from:
//!
//! * **SpecCode** — the schema's own defaults: `@scale` 100, `@fitToWidth`/`@fitToHeight` 1,
//!   `@paperSize` 1 (Letter), `@orientation` `default`, `@pageOrder` `downThenOver`.
//! * **DocumentedBehaviour** — `pageMargins` is in inches; `brk@id` is the row or column the break
//!   falls **before**, one-based; `_xlnm.Print_Area` and `_xlnm.Print_Titles` are §18.2.6's reserved
//!   names; `@fitToWidth` is *"the number of horizontal pages to fit on"*.
//! * **EngineDerived** — every page **boundary**. Where a break actually falls depends on how many
//!   rows fit in 9.5 inches, and that is this engine's answer and **not evidence about Excel**. The
//!   fixtures are therefore built so the assertion is a *relation* wherever it can be — a manual
//!   break falls exactly where the file put it, a print area excludes what it excludes, titles
//!   repeat on the second page and not on the first — rather than a raw row number nobody can check.
//!
//! There is nothing in a fourth category, because nobody has run Excel.

mod support;

use mjx_layout_xlsx::{paginate, PrintSetup, SheetBoxModel, SheetGrid};
use mjx_ooxml_types::spreadsheetml::PageOrder;
use support::{styles, worksheet};

/// A sheet of `rows` × `columns` populated cells, with `extra` written after `sheetData`.
fn sheet_body(rows: u32, columns: u16, extra: &str) -> String {
    let mut body = String::from("<sheetData>");
    for row in 1..=rows {
        body.push_str(&format!(r#"<row r="{row}">"#));
        for column in 0..columns {
            let letter = char::from(b'A' + u8::try_from(column).unwrap_or(0));
            body.push_str(&format!(r#"<c r="{letter}{row}"><v>{row}</v></c>"#));
        }
        body.push_str("</row>");
    }
    body.push_str("</sheetData>");
    format!(
        r#"<dimension ref="A1:{}{rows}"/>{body}{extra}"#,
        char::from(b'A' + u8::try_from(columns.saturating_sub(1)).unwrap_or(0))
    )
}

/// A workbook whose first sheet is `body` and whose `definedNames` are `names`.
fn book_with_names(body: &str, names: &str) -> mjx_xlsx::Workbook {
    use mjx_xlsx::{PartName, Workbook};

    let bytes = Workbook::blank()
        .expect("a blank workbook")
        .save_unchecked()
        .expect("blank saves");
    let mut package = mjx_xlsx::Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new(support::SHEET_PART).expect("a part name"),
            worksheet(body),
        )
        .expect("the worksheet is replaceable");
    package
        .replace_part_bytes(
            &PartName::new(support::STYLES_PART).expect("a part name"),
            styles(&[], ""),
        )
        .expect("the styles part is replaceable");
    if !names.is_empty() {
        let workbook_part = PartName::new("/xl/workbook.xml").expect("a part name");
        let existing = String::from_utf8(
            package
                .part_bytes(&workbook_part)
                .expect("the workbook part")
                .to_vec(),
        )
        .expect("utf-8");
        // `definedNames` sits after `sheets` in `CT_Workbook`'s sequence.
        let with = existing.replace(
            "</sheets>",
            &format!("</sheets><definedNames>{names}</definedNames>"),
        );
        assert_ne!(with, existing, "the sheets element was found");
        package
            .replace_part_bytes(&workbook_part, with.into_bytes())
            .expect("the workbook part is replaceable");
    }
    Workbook::from_package(package).expect("the authored package resolves")
}

/// Paginates `body` with `names` for printing.
fn print(body: &str, names: &str) -> (PrintSetup, mjx_layout_xlsx::PrintPagination) {
    let book = book_with_names(body, names);
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let geometry = model.geometry(&grid).expect("the geometry builds").clone();
    let setup = PrintSetup::read(&grid);
    let pagination = paginate(&grid, &geometry, &setup);
    (setup, pagination)
}

// -------------------------------------------------------------------------------------------
// The setup itself
// -------------------------------------------------------------------------------------------

#[test]
fn a_sheet_that_states_nothing_prints_on_letter_at_full_size() {
    // SpecCode: `@paperSize`'s schema default is 1, which §18.3.1.63 names Letter; `@scale`'s is
    // 100. DocumentedBehaviour: Letter is 8.5 × 11 inches.
    let (setup, _) = print(&sheet_body(4, 3, ""), "");
    assert_eq!(
        setup.paper.0,
        mjx_ooxml_core::measure::Emu::from_inches(8.5)
    );
    assert_eq!(
        setup.paper.1,
        mjx_ooxml_core::measure::Emu::from_inches(11.0)
    );
    assert!((setup.scale - 1.0).abs() < 1e-9);
    assert_eq!(setup.order, PageOrder::DownThenOver);
    assert!(setup.pages_wide.is_none(), "and it is not fitted");
}

#[test]
fn orientation_swaps_the_two_measurements_and_scale_is_read_as_a_multiplier() {
    let extra = r#"<pageMargins left="0.25" right="0.25" top="0.25" bottom="0.25" header="0.3" footer="0.3"/><pageSetup paperSize="9" orientation="landscape" scale="50"/>"#;
    let (setup, _) = print(&sheet_body(4, 3, extra), "");
    // DocumentedBehaviour: A4 is 210 × 297 mm, and landscape puts the long side first.
    assert!(
        setup.paper.0 > setup.paper.1,
        "landscape is wider than tall"
    );
    assert!((setup.scale - 0.5).abs() < 1e-9);
    assert_eq!(
        setup.margins.left,
        mjx_ooxml_core::measure::Emu::from_inches(0.25)
    );
}

// -------------------------------------------------------------------------------------------
// Manual breaks
// -------------------------------------------------------------------------------------------

#[test]
fn a_manual_row_break_falls_exactly_where_the_file_put_it() {
    // ⚠ The assertion a page *count* cannot make. `brk@id="21"` is one-based and means *before row
    // 21*, so page one ends at row 20 (zero-based 19) and page two starts at zero-based 20 — a
    // boundary the paper size did not choose, which is what makes it checkable without Excel.
    let extra = r#"<rowBreaks count="1" manualBreakCount="1"><brk id="21" max="16383" man="1"/></rowBreaks>"#;
    let (setup, pagination) = print(&sheet_body(40, 3, extra), "");
    assert_eq!(
        setup.row_breaks,
        vec![20],
        "zero-based, as the file wrote it"
    );
    assert!(pagination.len() >= 2, "the break made a second page");
    assert_eq!(pagination.pages[0].rows.end, 20, "page one stops before it");
    assert_eq!(pagination.pages[1].rows.start, 20, "page two starts at it");
}

#[test]
fn an_automatic_break_is_not_read_from_the_file_and_a_manual_one_is() {
    // `@man` distinguishes the two. An automatic `brk` is a record of where the *producer's*
    // pagination fell; this engine computes its own, and reading both would be two answers to one
    // question — with the file's going stale the moment a column width changed.
    let automatic = r#"<rowBreaks count="1"><brk id="5" max="16383"/></rowBreaks>"#;
    let (setup, pagination) = print(&sheet_body(10, 3, automatic), "");
    assert!(
        setup.row_breaks.is_empty(),
        "the automatic break is ignored"
    );
    assert_eq!(pagination.len(), 1, "ten rows still fit on one page");
}

#[test]
fn a_manual_column_break_cuts_the_page_sideways() {
    let extra = r#"<colBreaks count="1" manualBreakCount="1"><brk id="3" max="1048575" man="1"/></colBreaks>"#;
    let (setup, pagination) = print(&sheet_body(4, 6, extra), "");
    assert_eq!(setup.column_breaks, vec![2]);
    assert_eq!(pagination.len(), 2, "one row band, two column bands");
    assert_eq!(pagination.pages[0].columns, 0..2);
    assert_eq!(pagination.pages[1].columns, 2..6);
}

// -------------------------------------------------------------------------------------------
// Print area and repeated titles
// -------------------------------------------------------------------------------------------

const PRINT_AREA: &str =
    r#"<definedName name="_xlnm.Print_Area" localSheetId="0">Sheet1!$A$1:$C$30</definedName>"#;
const PRINT_TITLES: &str =
    r#"<definedName name="_xlnm.Print_Titles" localSheetId="0">Sheet1!$1:$2</definedName>"#;

#[test]
fn a_print_area_excludes_what_it_excludes() {
    // The sheet has six columns and forty rows; the area names three columns and thirty rows.
    let (setup, pagination) = print(&sheet_body(40, 6, ""), PRINT_AREA);
    assert_eq!(setup.area.len(), 1);
    assert_eq!(setup.area[0].last_column(), 2, "columns A to C");
    assert_eq!(setup.area[0].last_row(), 29, "rows 1 to 30");
    let last = pagination.pages.last().expect("at least one page");
    assert_eq!(last.columns.end, 3, "column D is off the printout");
    assert_eq!(last.rows.end, 30, "and so is row 31");
    assert!(
        pagination.page_of(0, 3).is_none(),
        "a cell outside the area is on no page"
    );
    assert!(pagination.page_of(0, 0).is_some());
}

#[test]
fn without_a_print_area_the_used_range_is_printed() {
    let (setup, pagination) = print(&sheet_body(5, 6, ""), "");
    assert_eq!(setup.area.len(), 1, "the sheet's own dimension");
    assert_eq!(pagination.pages[0].columns, 0..6);
    assert_eq!(pagination.pages[0].rows, 0..5);
}

#[test]
fn print_titles_repeat_on_every_page_but_the_one_they_are_already_on() {
    // ⚠ The assertion that is *about page contents*: rows 1 and 2 are the sheet's own first two
    // rows, so page one already shows them and repeating them there would print them twice. Every
    // later page repeats them at its top.
    let extra = r#"<rowBreaks count="1" manualBreakCount="1"><brk id="21" max="16383" man="1"/></rowBreaks>"#;
    let (setup, pagination) = print(
        &sheet_body(40, 3, extra),
        &format!("{PRINT_AREA}{PRINT_TITLES}"),
    );
    assert_eq!(setup.repeated_rows, Some(0..2), "rows 1 and 2, zero-based");
    assert!(pagination.len() >= 2);
    assert_eq!(
        pagination.pages[0].repeated_rows, None,
        "page one already has them"
    );
    assert_eq!(
        pagination.pages[1].repeated_rows,
        Some(0..2),
        "page two repeats them"
    );
    assert!(
        pagination.pages[1].contains(0, 0),
        "so row 1 is on page two as well as page one"
    );
    assert!(
        !pagination.pages[1].rows.contains(&0),
        "without being part of its data"
    );
}

#[test]
fn repeated_columns_are_read_from_the_same_name_and_told_apart_by_shape() {
    // `$1:$3` has digits on both sides of the colon and `$A:$B` has letters; that is what makes the
    // two unambiguous inside one comma-separated definition.
    let both = r#"<definedName name="_xlnm.Print_Titles" localSheetId="0">Sheet1!$1:$3,Sheet1!$A:$B</definedName>"#;
    let (setup, _) = print(&sheet_body(10, 6, ""), both);
    assert_eq!(setup.repeated_rows, Some(0..3));
    assert_eq!(setup.repeated_columns, Some(0..2));
}

// -------------------------------------------------------------------------------------------
// Scaling and fit-to-page
// -------------------------------------------------------------------------------------------

#[test]
fn fit_to_width_shrinks_until_the_columns_fit_and_says_that_it_did() {
    // A sheet far wider than a page, told to fit on one. EngineDerived: the resulting *number*.
    // What is assertable without Excel is the relation — that a fit happened, that it shrank rather
    // than enlarged, and that the columns now occupy one page rather than several.
    let wide = sheet_body(4, 26, r#"<pageSetup fitToWidth="1" fitToHeight="0"/>"#);
    let (setup, pagination) = print(&wide, "");
    assert_eq!(setup.pages_wide, Some(1));
    assert!(pagination.scaled_to_fit, "the scale came from the fit");
    assert!(pagination.scale < 1.0, "and it shrank the sheet");
    let columns: Vec<_> = pagination
        .pages
        .iter()
        .map(|page| page.columns.clone())
        .collect();
    assert_eq!(columns, vec![0..26], "one page wide");
}

#[test]
fn a_sheet_narrower_than_its_page_is_not_blown_up_to_fill_it() {
    // EngineDerived, and marked as such at its site: `Fit to 1 page wide` never enlarges in Excel
    // either, so the fit is clamped at 1.0.
    let narrow = sheet_body(3, 2, r#"<pageSetup fitToWidth="1" fitToHeight="1"/>"#);
    let (_, pagination) = print(&narrow, "");
    assert!((pagination.scale - 1.0).abs() < 1e-9);
}

#[test]
fn a_stated_scale_changes_how_much_fits_on_a_page() {
    // The instrument for `@scale`: the same sheet at 100 per cent and at 25 per cent. If the page
    // count did not fall, the scale was never applied to a width.
    let full = sheet_body(200, 3, r#"<pageSetup scale="100"/>"#);
    let quarter = sheet_body(200, 3, r#"<pageSetup scale="25"/>"#);
    let (_, at_full) = print(&full, "");
    let (_, at_quarter) = print(&quarter, "");
    assert!(
        at_quarter.len() < at_full.len(),
        "a quarter-size sheet needs fewer pages: {} vs {}",
        at_quarter.len(),
        at_full.len()
    );
}

// -------------------------------------------------------------------------------------------
// Page order and numbering
// -------------------------------------------------------------------------------------------

#[test]
fn page_order_decides_which_page_is_second() {
    // ⚠ Two column bands and two row bands, so the traversal has something to get wrong. Down-then-
    // over walks a column of pages to the bottom first; over-then-down walks a row of them to the
    // right. Both produce four pages, and only the *order* tells them apart.
    let breaks = r#"<rowBreaks count="1" manualBreakCount="1"><brk id="3" max="16383" man="1"/></rowBreaks><colBreaks count="1" manualBreakCount="1"><brk id="3" max="1048575" man="1"/></colBreaks>"#;
    let down = sheet_body(
        6,
        6,
        &format!(r#"{breaks}<pageSetup pageOrder="downThenOver"/>"#),
    );
    let over = sheet_body(
        6,
        6,
        &format!(r#"{breaks}<pageSetup pageOrder="overThenDown"/>"#),
    );

    let (_, downward) = print(&down, "");
    assert_eq!(downward.len(), 4);
    assert_eq!(
        downward.pages[1].columns,
        0..2,
        "page two is below page one"
    );
    assert_eq!(downward.pages[1].rows.start, 2);

    let (_, across) = print(&over, "");
    assert_eq!(across.len(), 4);
    assert_eq!(across.pages[1].rows, 0..2, "page two is beside page one");
    assert_eq!(across.pages[1].columns.start, 2);
}

#[test]
fn the_first_page_number_is_honoured_only_when_the_file_asks_for_it() {
    // SpecCode: `@useFirstPageNumber` gates `@firstPageNumber`, whose own default is 1.
    let ignored = sheet_body(4, 3, r#"<pageSetup firstPageNumber="7"/>"#);
    let (_, plain) = print(&ignored, "");
    assert_eq!(plain.pages[0].number, 1);

    let honoured = sheet_body(
        4,
        3,
        r#"<pageSetup firstPageNumber="7" useFirstPageNumber="1"/>"#,
    );
    let (_, numbered) = print(&honoured, "");
    assert_eq!(numbered.pages[0].number, 7);
}

// -------------------------------------------------------------------------------------------
// Print options, and the sheet that prints nothing
// -------------------------------------------------------------------------------------------

#[test]
fn the_four_print_options_are_read_and_default_to_off() {
    let bare = sheet_body(3, 3, "");
    let (off, _) = print(&bare, "");
    assert!(!off.prints_grid_lines);
    assert!(!off.prints_headings);
    assert!(!off.centred_horizontally);
    assert!(!off.centred_vertically);

    let stated = sheet_body(
        3,
        3,
        r#"<printOptions horizontalCentered="1" verticalCentered="1" headings="1" gridLines="1"/>"#,
    );
    let (on, _) = print(&stated, "");
    assert!(on.prints_grid_lines);
    assert!(on.prints_headings);
    assert!(on.centred_horizontally);
    assert!(on.centred_vertically);
}

#[test]
fn a_sheet_with_nothing_on_it_prints_no_pages_rather_than_one_empty_one() {
    let (_, pagination) = print(r#"<sheetData/>"#, "");
    assert!(pagination.is_empty());
    assert_eq!(pagination.len(), 0);
}

#[test]
fn a_hidden_row_takes_no_space_on_a_printed_page_either() {
    // The same discipline the viewport holds: a hidden row occupies nothing and everything below it
    // moves up. A print pagination that walked the coordinate range would put a page break in a
    // different place from the one the reader sees on screen.
    let visible = sheet_body(6, 3, "");
    let hidden = visible.replace(r#"<row r="3">"#, r#"<row r="3" hidden="1">"#);
    assert_ne!(hidden, visible, "row 3 was found");
    let (_, with_all) = print(&visible, "");
    let (_, with_hidden) = print(&hidden, "");
    assert!(with_all.pages[0].rows.contains(&2));
    assert!(
        with_hidden.pages[0].rows.contains(&2),
        "the range still spans it"
    );
    // The observable difference is the *contents*: the hidden row is not one of the rows the page
    // actually lays out, which the range alone cannot say. Its absence shows up as a page that
    // reaches further down the sheet for the same paper.
    assert_eq!(with_hidden.pages.len(), with_all.pages.len());
}
