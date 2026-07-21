//! Integration tests for cell formatting — fill, the six borders, margins, anchoring and text
//! direction.
//!
//! Two claims get the most attention. **Each property is its own element or attribute**, so setting
//! one must leave the other five borders (and everything else) alone — a writer that rebuilt
//! `a:tcPr` would pass a single-border test and fail every one of these. And **unstated is not
//! zero**: a cell's margins default to 0.1"/0.05", so reporting an absent margin as zero would
//! describe a cell that does not exist.

use std::collections::BTreeMap;
use std::path::PathBuf;

use mjx_dml::{
    CellBorder, ColorSpec, Emu, FillSpec, LineSpec, LineWidth, TableCellProperties, TextAnchoring,
    TextDirection,
};
use mjx_opc::Package;
use mjx_pptx::{CellMargins, PptxError, Presentation, ShapeBounds};

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
}

fn byte_map(pkg: &Package) -> BTreeMap<String, Vec<u8>> {
    pkg.entries()
        .iter()
        .filter_map(|e| e.bytes().map(|b| (e.name.clone(), b.to_vec())))
        .collect()
}

/// A deck with a 2x2 table on slide 0, returned with the table's shape index.
fn deck_with_table() -> (Presentation, usize) {
    let mut pres = Presentation::open(&fixture("sample.pptx")).expect("open");
    let table = pres
        .add_table(0, 2, 2, ShapeBounds::from_inches(1.0, 1.0, 6.0, 2.0))
        .expect("add table");
    (pres, table)
}

fn red_line() -> LineSpec {
    LineSpec {
        width: Some(LineWidth::from_emu(12_700)),
        fill: Some(FillSpec::Solid(ColorSpec::Srgb("FF0000".to_owned()))),
        ..LineSpec::default()
    }
}

// ---------------------------------------------------------------------------------------------
// Borders
// ---------------------------------------------------------------------------------------------

#[test]
fn each_of_the_six_edges_holds_its_own_border() {
    // Every edge, one at a time, in a deck of its own, so nothing else can have written it.
    for edge in CellBorder::all() {
        let (mut pres, table) = deck_with_table();
        pres.set_cell_border(0, table, 0, 0, edge, &red_line())
            .expect("set border");

        assert!(
            pres.cell_border(0, table, 0, 0, edge)
                .expect("read")
                .is_some(),
            "{edge:?} was not written"
        );
        for other in CellBorder::all().into_iter().filter(|e| *e != edge) {
            assert!(
                pres.cell_border(0, table, 0, 0, other)
                    .expect("read")
                    .is_none(),
                "setting {edge:?} also wrote {other:?}"
            );
        }
    }
}

#[test]
fn a_diagonal_is_a_real_border() {
    // `lnTlToBr` is drawn corner to corner inside the cell rather than around it, and is otherwise
    // an ordinary `CT_LineProperties` — the same `LineSpec` describes it.
    let (mut pres, table) = deck_with_table();
    pres.set_cell_border(
        0,
        table,
        1,
        1,
        CellBorder::TopLeftToBottomRight,
        &red_line(),
    )
    .expect("set a diagonal");

    let line = pres
        .cell_border(0, table, 1, 1, CellBorder::TopLeftToBottomRight)
        .expect("read")
        .expect("declared");
    assert_eq!(line.width.map(LineWidth::emu), Some(12_700));
}

#[test]
fn borders_survive_a_save_and_reopen() {
    let (mut pres, table) = deck_with_table();
    for edge in [CellBorder::Top, CellBorder::Bottom] {
        pres.set_cell_border(0, table, 0, 0, edge, &red_line())
            .expect("set border");
    }

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    for edge in [CellBorder::Top, CellBorder::Bottom] {
        assert!(reopened
            .cell_border(0, table, 0, 0, edge)
            .expect("read")
            .is_some());
    }
    assert!(reopened
        .cell_border(0, table, 0, 0, CellBorder::Left)
        .expect("read")
        .is_none());
}

#[test]
fn a_border_can_be_removed_again() {
    let (mut pres, table) = deck_with_table();
    pres.set_cell_border(0, table, 0, 0, CellBorder::Left, &red_line())
        .expect("set");
    pres.set_cell_border(0, table, 0, 0, CellBorder::Right, &red_line())
        .expect("set");

    pres.clear_cell_border(0, table, 0, 0, CellBorder::Left)
        .expect("clear");

    assert!(pres
        .cell_border(0, table, 0, 0, CellBorder::Left)
        .expect("read")
        .is_none());
    assert!(
        pres.cell_border(0, table, 0, 0, CellBorder::Right)
            .expect("read")
            .is_some(),
        "clearing one edge must not clear another"
    );
}

#[test]
fn borders_land_in_the_schemas_order_however_they_were_written() {
    // `CT_TableCellProperties` is a sequence: lnL, lnR, lnT, lnB, lnTlToBr, lnBlToTr, …
    // Written back to front, they must still come out in that order or Office cannot read the cell.
    let (mut pres, table) = deck_with_table();
    for edge in [
        CellBorder::BottomLeftToTopRight,
        CellBorder::Bottom,
        CellBorder::Left,
        CellBorder::Top,
    ] {
        pres.set_cell_border(0, table, 0, 0, edge, &red_line())
            .expect("set border");
    }

    let saved = pres.save().expect("save");
    let xml = String::from_utf8(
        Package::open(&saved)
            .expect("reopen")
            .entries()
            .iter()
            .find(|e| e.name.ends_with("slide1.xml"))
            .and_then(|e| e.bytes().map(<[u8]>::to_vec))
            .expect("slide part"),
    )
    .expect("utf-8");

    let at = |needle: &str| {
        xml.find(needle)
            .unwrap_or_else(|| panic!("{needle} missing"))
    };
    assert!(at("<a:lnL") < at("<a:lnT"), "{xml}");
    assert!(at("<a:lnT") < at("<a:lnB"), "{xml}");
    assert!(at("<a:lnB") < at("<a:lnBlToTr"), "{xml}");
}

// ---------------------------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------------------------

#[test]
fn a_cell_can_be_filled_and_unfilled() {
    let (mut pres, table) = deck_with_table();
    assert_eq!(pres.cell_fill(0, table, 0, 0).expect("read"), None);

    let blue = FillSpec::Solid(ColorSpec::Srgb("0000FF".to_owned()));
    pres.set_cell_fill(0, table, 0, 0, &blue).expect("fill");
    assert_eq!(pres.cell_fill(0, table, 0, 0).expect("read"), Some(blue));
    assert_eq!(
        pres.cell_fill(0, table, 0, 1).expect("read"),
        None,
        "the neighbouring cell was not filled"
    );

    pres.clear_cell_fill(0, table, 0, 0).expect("clear");
    assert_eq!(pres.cell_fill(0, table, 0, 0).expect("read"), None);
}

#[test]
fn no_fill_is_a_statement_and_absent_is_not() {
    // Removing the fill lets the table style decide; `FillSpec::None` says the cell is deliberately
    // unfilled. Both read back differently, which is the whole point of keeping them apart.
    let (mut pres, table) = deck_with_table();
    pres.set_cell_fill(0, table, 0, 0, &FillSpec::None)
        .expect("state no fill");
    assert_eq!(
        pres.cell_fill(0, table, 0, 0).expect("read"),
        Some(FillSpec::None)
    );

    pres.clear_cell_fill(0, table, 0, 0).expect("remove it");
    assert_eq!(pres.cell_fill(0, table, 0, 0).expect("read"), None);
}

#[test]
fn a_fill_and_a_border_coexist_on_one_cell() {
    let (mut pres, table) = deck_with_table();
    pres.set_cell_fill(
        0,
        table,
        0,
        0,
        &FillSpec::Solid(ColorSpec::Srgb("00FF00".to_owned())),
    )
    .expect("fill");
    pres.set_cell_border(0, table, 0, 0, CellBorder::Top, &red_line())
        .expect("border");

    assert!(pres.cell_fill(0, table, 0, 0).expect("read").is_some());
    assert!(pres
        .cell_border(0, table, 0, 0, CellBorder::Top)
        .expect("read")
        .is_some());
}

// ---------------------------------------------------------------------------------------------
// Margins, anchoring, direction
// ---------------------------------------------------------------------------------------------

#[test]
fn an_unstated_margin_is_absent_not_zero() {
    let (mut pres, table) = deck_with_table();
    let margins = pres.cell_margins(0, table, 0, 0).expect("read");

    assert_eq!(margins, CellMargins::default());
    assert_eq!(margins.left, None, "absent, not zero");
    // What a renderer would substitute is a different fact, and the model states it.
    assert_eq!(TableCellProperties::DEFAULT_MARGIN_HORIZONTAL.emu(), 91_440);
    assert_eq!(TableCellProperties::DEFAULT_MARGIN_VERTICAL.emu(), 45_720);
}

#[test]
fn one_margin_can_be_set_without_stating_the_others() {
    let (mut pres, table) = deck_with_table();
    pres.set_cell_margins(
        0,
        table,
        0,
        0,
        CellMargins {
            left: Some(Emu::from_emu(180_000)),
            ..CellMargins::default()
        },
    )
    .expect("set one margin");

    let margins = pres.cell_margins(0, table, 0, 0).expect("read");
    assert_eq!(margins.left, Some(Emu::from_emu(180_000)));
    assert_eq!(margins.right, None);
    assert_eq!(margins.top, None);
    assert_eq!(margins.bottom, None);
}

#[test]
fn uniform_margins_state_all_four() {
    let (mut pres, table) = deck_with_table();
    pres.set_cell_margins(0, table, 1, 1, CellMargins::uniform(Emu::from_points(6.0)))
        .expect("set margins");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    let margins = reopened.cell_margins(0, table, 1, 1).expect("read");
    let expected = Some(Emu::from_points(6.0));
    assert_eq!(
        (margins.left, margins.right, margins.top, margins.bottom),
        (expected, expected, expected, expected)
    );
}

#[test]
fn anchoring_and_direction_round_trip() {
    let (mut pres, table) = deck_with_table();
    assert_eq!(pres.cell_anchor(0, table, 0, 0).expect("read"), None);

    pres.set_cell_anchor(0, table, 0, 0, TextAnchoring::Center)
        .expect("anchor");
    pres.set_cell_text_direction(0, table, 0, 1, TextDirection::Vertical270)
        .expect("rotate");

    let saved = pres.save().expect("save");
    let mut reopened = Presentation::open(&saved).expect("reopen");
    assert_eq!(
        reopened.cell_anchor(0, table, 0, 0).expect("read"),
        Some(TextAnchoring::Center)
    );
    assert_eq!(
        reopened.cell_text_direction(0, table, 0, 1).expect("read"),
        Some(TextDirection::Vertical270)
    );
    assert_eq!(
        reopened.cell_anchor(0, table, 0, 1).expect("read"),
        None,
        "rotating one cell did not anchor it"
    );
}

// ---------------------------------------------------------------------------------------------
// Fidelity
// ---------------------------------------------------------------------------------------------

#[test]
fn formatting_a_cell_dirties_only_its_slide() {
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let before = byte_map(&Package::open(&fixture("layouts.pptx")).expect("baseline"));
    let count = pres.shape_count(1).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(1, idx).is_ok())
        .expect("the fixture table");

    pres.set_cell_border(1, table, 0, 0, CellBorder::Bottom, &red_line())
        .expect("border");

    let saved = pres.save().expect("save");
    let after = byte_map(&Package::open(&saved).expect("reopen"));
    for (name, original) in &before {
        if name.ends_with("slide2.xml") {
            continue;
        }
        assert_eq!(after.get(name), Some(original), "dirtied {name}");
    }
}

#[test]
fn reading_cell_formatting_dirties_nothing() {
    let bytes = fixture("layouts.pptx");
    let original = byte_map(&Package::open(&bytes).expect("baseline"));

    let mut pres = Presentation::open(&bytes).expect("open");
    let count = pres.shape_count(1).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(1, idx).is_ok())
        .expect("the fixture table");
    let _ = pres.cell_fill(1, table, 0, 0).expect("fill");
    let _ = pres.cell_margins(1, table, 0, 0).expect("margins");
    let _ = pres.cell_anchor(1, table, 0, 0).expect("anchor");
    for edge in CellBorder::all() {
        let _ = pres.cell_border(1, table, 0, 0, edge).expect("border");
    }

    assert_eq!(
        byte_map(&Package::open(&pres.save().expect("save")).expect("reopen")),
        original
    );
}

#[test]
fn each_edit_leaves_the_earlier_ones_standing() {
    // Four properties written in turn onto one cell. A writer that rebuilt `a:tcPr` instead of
    // merging into it would pass any one of these alone and lose the other three.
    let mut pres = Presentation::open(&fixture("layouts.pptx")).expect("open");
    let count = pres.shape_count(1).expect("count");
    let table = (0..count)
        .find(|&idx| pres.table_dimensions(1, idx).is_ok())
        .expect("the fixture table");

    pres.set_cell_anchor(1, table, 0, 0, TextAnchoring::Bottom)
        .expect("anchor");
    pres.set_cell_fill(
        1,
        table,
        0,
        0,
        &FillSpec::Solid(ColorSpec::Srgb("ABCDEF".to_owned())),
    )
    .expect("fill");
    pres.set_cell_border(1, table, 0, 0, CellBorder::Left, &red_line())
        .expect("border");
    pres.set_cell_margins(1, table, 0, 0, CellMargins::uniform(Emu::from_emu(1_000)))
        .expect("margins");

    // Every earlier edit is still there after the later ones.
    assert_eq!(
        pres.cell_anchor(1, table, 0, 0).expect("read"),
        Some(TextAnchoring::Bottom)
    );
    assert!(pres.cell_fill(1, table, 0, 0).expect("read").is_some());
    assert!(pres
        .cell_border(1, table, 0, 0, CellBorder::Left)
        .expect("read")
        .is_some());
    assert_eq!(
        pres.cell_margins(1, table, 0, 0).expect("read").left,
        Some(Emu::from_emu(1_000))
    );
    // And the cell's text was never in the way.
    assert_eq!(pres.cell_text(1, table, 0, 0).expect("text"), "Cell");
}

#[test]
fn formatting_a_cell_that_is_not_there_is_a_typed_error() {
    let (mut pres, table) = deck_with_table();
    assert!(matches!(
        pres.set_cell_fill(0, table, 5, 0, &FillSpec::None),
        Err(PptxError::TableCellOutOfRange {
            row: 5,
            rows: 2,
            columns: 2,
            ..
        })
    ));
    assert!(matches!(
        pres.cell_margins(0, table, 0, 9),
        Err(PptxError::TableCellOutOfRange { column: 9, .. })
    ));
    // And a shape that frames no table says so rather than inventing a cell.
    assert!(matches!(
        pres.cell_fill(0, 0, 0, 0),
        Err(PptxError::ShapeIsNotATable)
    ));
}
