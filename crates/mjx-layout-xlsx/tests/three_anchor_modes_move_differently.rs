//! ⚠ One fixture that tells the three anchor modes apart, rather than three that each say *a
//! rectangle appeared*.
//!
//! `xdr:twoCellAnchor`, `xdr:oneCellAnchor` and `xdr:absoluteAnchor` differ in exactly one way, and
//! it is not their markup — it is which half of the answer the grid supplies:
//!
//! | Mode | Position | Size |
//! |---|---|---|
//! | two-cell | from the grid | from the grid |
//! | one-cell | from the grid | stated in EMU |
//! | absolute | stated in EMU | stated in EMU |
//!
//! So the whole suite is: **lay the same three objects out over two sheets that differ only in one
//! row's height, and ask which rectangles moved.** Three separate fixtures each asserting that a
//! mode produced *a* rectangle would be green for an implementation that read every anchor the same
//! way; this one is red the moment two of the three agree when they should not.
//!
//! # Provenance
//!
//! * **SpecCode** — the three element names and their children are §20.5.2's; `CT_Marker`'s four
//!   children are all `minOccurs="1"`, which is why a partial marker places nothing.
//! * **DocumentedBehaviour** — a row's height is stated in **points** and a point is 12,700 EMU by
//!   definition, so a row grown from 15 pt to 60 pt moves everything below it by exactly
//!   `45 × 12,700` EMU. That number is arithmetic rather than a reading, which is what makes it
//!   assertable without Excel.
//! * **EngineDerived** — that a marker beyond the grid clamps, and that a drawing is placed against
//!   the *scrolling* pane region.

mod support;

use mjx_layout::{Fragment, FragmentTree};
use mjx_layout_xlsx::{AnchorMode, PlacedDrawing, SheetBoxModel, SheetGrid};
use mjx_ooxml_core::measure::Emu;
use mjx_xlsx::drawing_geometry::{CellMarker, Position, ResizingBehavior, Size};
use mjx_xlsx::Workbook;

/// A 1×1 red PNG — the smallest thing `ImageFormat::sniff` calls a PNG.
const PNG: &[u8] = &[
    0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D', b'R',
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53,
    0xDE, 0x00, 0x00, 0x00, 0x0C, b'I', b'D', b'A', b'T', 0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00,
    0x00, 0x03, 0x01, 0x01, 0x00, 0x18, 0xDD, 0x8D, 0xB0, 0x00, 0x00, 0x00, 0x00, b'I', b'E', b'N',
    b'D', 0xAE, 0x42, 0x60, 0x82,
];

/// A workbook whose first row is `first_row_points` tall, with one object of each anchor mode
/// pinned **below** it.
///
/// Every object starts in row 3 (zero-based 2), so growing row 1 moves whatever follows the grid and
/// leaves whatever does not exactly where it was.
fn book(first_row_points: f64) -> Workbook {
    let body = format!(
        r#"<dimension ref="A1:D6"/><sheetData><row r="1" ht="{first_row_points}" customHeight="1"><c r="A1"><v>1</v></c></row><row r="3"><c r="A3"><v>3</v></c></row></sheetData>"#
    );
    let styles = support::styles(&[], "");
    let mut book = support::workbook(&support::worksheet(&body), &styles);
    // Two-cell: both corners follow the grid, so it moves *and* grows when a row between them does.
    book.add_two_cell_anchored_picture(
        0,
        PNG,
        "two-cell",
        CellMarker::new(1, 0, 2, 0),
        CellMarker::new(3, 0, 4, 0),
        ResizingBehavior::MoveAndResizeWithAnchorCells,
    )
    .expect("the two-cell picture is added");
    // One-cell: the top-left follows the grid and the size is its own.
    book.add_one_cell_anchored_picture(
        0,
        PNG,
        "one-cell",
        CellMarker::new(1, 0, 2, 0),
        Size::from_emu(914_400, 457_200),
    )
    .expect("the one-cell picture is added");
    // Absolute: neither.
    book.add_absolute_anchored_picture(
        0,
        PNG,
        "absolute",
        Position::from_emu(914_400, 914_400),
        Size::from_emu(914_400, 457_200),
    )
    .expect("the absolute picture is added");
    book
}

/// The three placed drawings of a sheet whose first row is `first_row_points` tall.
fn placed(first_row_points: f64) -> Vec<PlacedDrawing> {
    let book = book(first_row_points);
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let constraints = support::viewport(12.0, 12.0);
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    model.catalogue().drawings().to_vec()
}

/// The one drawing of `mode`.
fn of(drawings: &[PlacedDrawing], mode: AnchorMode) -> PlacedDrawing {
    drawings
        .iter()
        .find(|drawing| drawing.mode == mode)
        .cloned()
        .unwrap_or_else(|| panic!("no {mode:?} anchor was placed"))
}

/// Fifteen points is the schema's own default row height; sixty is four times it.
const SHORT: f64 = 15.0;
const TALL: f64 = 60.0;

/// The exact distance row 1 grows by, in EMU: 45 points at 12,700 EMU each.
const GROWTH: i64 = 45 * 12_700;

#[test]
fn all_three_modes_are_placed_and_each_is_named() {
    let drawings = placed(SHORT);
    assert_eq!(drawings.len(), 3, "three anchors, three rectangles");
    for mode in [
        AnchorMode::TwoCell,
        AnchorMode::OneCell,
        AnchorMode::Absolute,
    ] {
        let drawing = of(&drawings, mode);
        assert!(drawing.rect.width() > Emu::ZERO, "{mode:?} has a width");
        assert!(drawing.rect.height() > Emu::ZERO, "{mode:?} has a height");
        assert!(
            drawing.image.is_some(),
            "{mode:?} reaches the image part its `a:blip` names"
        );
        assert_eq!(drawing.object, Some("pic"));
        assert!(drawing.prints_with_sheet, "and prints with the sheet");
    }
}

#[test]
fn growing_a_row_moves_the_two_anchors_that_follow_the_grid_and_not_the_one_that_does_not() {
    // ⚠ The whole suite in one assertion. Row 1 grows by 45 points; every object is anchored below
    // it. A reader that placed all three the same way is red on at least one line here, whichever
    // way it read them.
    let short = placed(SHORT);
    let tall = placed(TALL);

    let two_cell = (
        of(&short, AnchorMode::TwoCell),
        of(&tall, AnchorMode::TwoCell),
    );
    assert_eq!(
        two_cell.1.rect.top.emu() - two_cell.0.rect.top.emu(),
        GROWTH,
        "a two-cell anchor moves down by exactly the growth"
    );

    let one_cell = (
        of(&short, AnchorMode::OneCell),
        of(&tall, AnchorMode::OneCell),
    );
    assert_eq!(
        one_cell.1.rect.top.emu() - one_cell.0.rect.top.emu(),
        GROWTH,
        "so does a one-cell anchor"
    );

    let absolute = (
        of(&short, AnchorMode::Absolute),
        of(&tall, AnchorMode::Absolute),
    );
    assert_eq!(
        absolute.1.rect.top, absolute.0.rect.top,
        "and an absolute anchor does not move at all"
    );
    assert_eq!(absolute.0.rect.top.emu(), 914_400, "it is where it says");
}

#[test]
fn only_the_two_cell_anchor_changes_size_when_the_grid_does() {
    // The second half of the distinction, and the one a *position*-only assertion misses entirely.
    // Row 3 is between the two-cell anchor's corners, so growing **it** stretches that object and
    // leaves the other two the size they were.
    let stretch = |points: f64| {
        let body = format!(
            r#"<dimension ref="A1:D6"/><sheetData><row r="3" ht="{points}" customHeight="1"><c r="A3"><v>3</v></c></row></sheetData>"#
        );
        let styles = support::styles(&[], "");
        let mut book = support::workbook(&support::worksheet(&body), &styles);
        book.add_two_cell_anchored_picture(
            0,
            PNG,
            "two-cell",
            CellMarker::new(1, 0, 1, 0),
            CellMarker::new(3, 0, 4, 0),
            ResizingBehavior::MoveAndResizeWithAnchorCells,
        )
        .expect("added");
        book.add_one_cell_anchored_picture(
            0,
            PNG,
            "one-cell",
            CellMarker::new(1, 0, 1, 0),
            Size::from_emu(914_400, 457_200),
        )
        .expect("added");
        book.add_absolute_anchored_picture(
            0,
            PNG,
            "absolute",
            Position::from_emu(914_400, 914_400),
            Size::from_emu(914_400, 457_200),
        )
        .expect("added");
        let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
        let mut model = SheetBoxModel::new(support::resolver());
        let constraints = support::viewport(12.0, 12.0);
        let _ = support::lay_out(&mut model, &grid, &constraints, 0);
        model.catalogue().drawings().to_vec()
    };

    let short = stretch(SHORT);
    let tall = stretch(TALL);
    assert_eq!(
        of(&tall, AnchorMode::TwoCell).rect.height().emu()
            - of(&short, AnchorMode::TwoCell).rect.height().emu(),
        GROWTH,
        "a two-cell anchor grows with the row it spans"
    );
    assert_eq!(
        of(&tall, AnchorMode::OneCell).rect.height(),
        of(&short, AnchorMode::OneCell).rect.height(),
        "a one-cell anchor keeps its stated size"
    );
    assert_eq!(
        of(&tall, AnchorMode::Absolute).rect.height(),
        of(&short, AnchorMode::Absolute).rect.height(),
        "and so does an absolute one"
    );
    assert_eq!(
        of(&short, AnchorMode::OneCell).rect.height().emu(),
        457_200,
        "which is the extent the file stated"
    );
}

#[test]
fn a_drawing_reaches_the_fragment_tree_under_an_address_that_is_not_a_cells() {
    // A drawing's path is `[u32::MAX, index]` rather than `[row, column]`, so `invalidate` cannot
    // read an anchor's position as a row number — which would make an edit to a picture invalidate
    // the fourth band of the grid.
    let book = book(SHORT);
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let constraints = support::viewport(12.0, 12.0);
    let tree: FragmentTree = support::lay_out(&mut model, &grid, &constraints, 0);
    let drawings: Vec<_> = tree
        .nodes()
        .filter(|(_, node)| {
            node.source().path().segments().first() == Some(&u32::MAX)
                && matches!(node.fragment(), Fragment::Box(_))
        })
        .collect();
    assert_eq!(drawings.len(), 3, "one box per anchor");
    for (_, node) in &drawings {
        let segments = node.source().path().segments();
        assert_eq!(segments.len(), 2, "the sentinel and the anchor's index");
        assert!(segments[1] < 3);
    }
}

#[test]
fn an_anchor_the_file_does_not_finish_places_nothing() {
    // `CT_Marker`'s four children are `minOccurs="1"`, so a `from` missing its `rowOff` names no
    // cell — and a rectangle invented for it would be a position presented as a measurement. The
    // *other* anchor in the same part still places, which is what says the drawing was read rather
    // than refused.
    let body =
        r#"<dimension ref="A1:D6"/><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData>"#;
    let styles = support::styles(&[], "");
    let mut book = support::workbook(&support::worksheet(body), &styles);
    book.add_two_cell_anchored_picture(
        0,
        PNG,
        "whole",
        CellMarker::new(1, 0, 1, 0),
        CellMarker::new(2, 0, 2, 0),
        ResizingBehavior::MoveAndResizeWithAnchorCells,
    )
    .expect("added");
    book.add_one_cell_anchored_picture(
        0,
        PNG,
        "broken",
        CellMarker::new(1, 0, 3, 0),
        Size::from_emu(914_400, 457_200),
    )
    .expect("added");
    // Take the `xdr:ext` off the one-cell anchor, which is the child that makes it placeable. The
    // workbook is saved and reopened rather than edited in place: `Workbook` hands out its package
    // by shared reference only, which is the right shape for a reader and means a suite that wants
    // to author a *malformed* part goes round through the bytes.
    let part = mjx_xlsx::PartName::new("/xl/drawings/drawing1.xml").expect("a part name");
    let saved = book.save_unchecked().expect("the workbook saves");
    let mut package = mjx_xlsx::Package::open(&saved).expect("it reopens");
    let text = String::from_utf8(
        package
            .part_bytes(&part)
            .expect("the drawing part")
            .to_vec(),
    )
    .expect("utf-8");
    let broken = text.replace(r#"<xdr:ext cx="914400" cy="457200"/>"#, "");
    assert_ne!(broken, text, "the extent was found and removed");
    package
        .replace_part_bytes(&part, broken.into_bytes())
        .expect("the part is replaceable");
    let book = Workbook::from_package(package).expect("it still opens");

    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let mut model = SheetBoxModel::new(support::resolver());
    let constraints = support::viewport(12.0, 12.0);
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    let drawings = model.catalogue().drawings();
    assert_eq!(
        drawings.len(),
        1,
        "the finished anchor placed, the other did not"
    );
    assert_eq!(drawings[0].mode, AnchorMode::TwoCell);
}
