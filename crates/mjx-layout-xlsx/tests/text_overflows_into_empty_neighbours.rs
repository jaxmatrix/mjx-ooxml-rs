//! Excel's overflow rule, in **all four of its states** — the behaviour that looks like a bug when
//! it is missing and like a bug when it is wrong.
//!
//! Each case is a sheet a person would recognise: a long label in a narrow column, with one thing
//! changed. The assertion is on where the *text* ended up, not on an internal flag, because a flag
//! that says `Spills` while the glyphs sit inside the cell is exactly the failure the rule has.

mod support;

use mjx_layout_xlsx::Overflow;

use support::{grid_from, model, styles, viewport};

/// A long label that certainly does not fit a nine-character column.
const LABEL: &str = "Quarterly revenue by region and product line";

/// A sheet with `LABEL` in `A1`, `styles` index 1 applied to it, and whatever else `extra` states.
fn sheet(extra: &str, style: u32) -> String {
    format!(
        r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetData><row r="1"><c r="A1" s="{style}" t="inlineStr"><is><t>{LABEL}</t></is></c>{extra}</row></sheetData>"#
    )
}

/// Lays out band zero of a sheet and answers what `A1` did.
fn overflow_of(body: &str, styles_markup: &[u8]) -> (Overflow, mjx_layout::FragmentTree) {
    let grid = grid_from(body, styles_markup);
    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);
    let report = model
        .catalogue()
        .cell(0, 0)
        .unwrap_or_else(|| panic!("A1 was laid out"))
        .clone();
    (report.overflow, tree)
}

/// How far right the *drawn* text of `A1` reached, in EMU — the natural line extent bounded by the
/// clip it is drawn under, which is what a reader sees.
fn text_right(tree: &mjx_layout::FragmentTree) -> i64 {
    support::drawn_right(tree, 0, 0)
}

/// Where column `column`'s left edge is, in EMU, for the sheet the cases here build.
fn column_left(body: &str, styles_markup: &[u8], column: u16) -> i64 {
    let grid = grid_from(body, styles_markup);
    let mut model = model();
    let geometry = model.geometry(&grid).expect("the geometry builds");
    geometry.columns().left(column).emu()
}

#[test]
fn state_one_the_text_spills_across_empty_neighbours() {
    let body = sheet("", 0);
    let plain = styles(&[], "");
    let (overflow, tree) = overflow_of(&body, &plain);

    assert!(
        overflow.spills(),
        "a long left-aligned label with empty neighbours spills: {overflow:?}"
    );
    let (first, last) = overflow.columns(0);
    assert_eq!(first, 0, "a left-aligned cell does not spill leftward");
    assert!(last > 0, "and it does spill rightward");
    assert!(
        text_right(&tree) > column_left(&body, &plain, 1),
        "the glyphs really do cross into column B"
    );
}

#[test]
fn state_two_the_text_stops_at_the_first_non_empty_cell() {
    // The same sheet with `C1` occupied. Excel cuts the label off at C1's left edge — this is the
    // state a reader recognises as a truncated label, and the one an implementation without the rule
    // gets wrong by drawing the label straight over C1's own text.
    let body = sheet(r#"<c r="C1" t="inlineStr"><is><t>x</t></is></c>"#, 0);
    let plain = styles(&[], "");
    let (overflow, tree) = overflow_of(&body, &plain);

    match &overflow {
        Overflow::Spills {
            columns,
            stopped_right_by,
            ..
        } => {
            assert_eq!(
                *stopped_right_by,
                Some(2),
                "C1 is what stopped it: {overflow:?}"
            );
            assert_eq!(columns.1, 1, "so the text reaches B and no further");
        }
        other => panic!("expected a bounded spill, got {other:?}"),
    }
    assert!(
        text_right(&tree) <= column_left(&body, &plain, 2),
        "and the glyphs stop at C1's left edge rather than crossing it"
    );

    // The instrument's own instrument: without `C1` the same label reaches further. A gate that only
    // asserted the bounded case would pass for an implementation that never spills at all.
    let (unbounded, unbounded_tree) = overflow_of(&sheet("", 0), &plain);
    assert!(unbounded.spills());
    assert!(
        text_right(&unbounded_tree) > text_right(&tree),
        "removing the neighbour must let the label reach further, or the rule is doing nothing"
    );
}

#[test]
fn state_three_wrap_suppresses_it_entirely() {
    let wrapping = styles(
        &[
            r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" applyAlignment="1"><alignment wrapText="1"/></xf>"#,
        ],
        "",
    );
    let body = sheet("", 1);
    let (overflow, tree) = overflow_of(&body, &wrapping);

    assert!(
        matches!(overflow, Overflow::SuppressedByWrap),
        "wrapText stops the overflow outright: {overflow:?}"
    );
    assert!(
        support::lines_of(&tree, 0, 0).len() > 1,
        "and produces several lines instead"
    );
    assert!(
        text_right(&tree) <= column_left(&body, &wrapping, 1),
        "none of which leaves the cell"
    );
}

#[test]
fn state_four_a_fill_alignment_suppresses_it_entirely() {
    let filling = styles(
        &[
            r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" applyAlignment="1"><alignment horizontal="fill"/></xf>"#,
        ],
        "",
    );
    let body = sheet("", 1);
    let (overflow, tree) = overflow_of(&body, &filling);

    assert!(
        matches!(overflow, Overflow::SuppressedByFill),
        "horizontal=\"fill\" repeats the text inside the cell and never leaves it: {overflow:?}"
    );
    assert!(
        overflow.is_clipped(),
        "which means the text is clipped to its own box"
    );
    assert!(
        !support::lines_of(&tree, 0, 0).is_empty(),
        "and there is still text drawn"
    );
}

#[test]
fn a_number_overflows_leftward_because_general_resolves_to_right() {
    // §18.8.1's `general` is *"align depending on the type of data"*, and the direction of the
    // overflow follows the resolved alignment rather than the stated one. A renderer that read
    // `general` as `left` would spill a long number rightward, which is visibly not Excel.
    let body = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="9"/>
<sheetData><row r="1"><c r="E1"><v>123456789012345678</v></c></row></sheetData>"#;
    let grid = grid_from(body, &styles(&[], ""));
    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    let report = model.catalogue().cell(0, 4).expect("E1 was laid out");

    match &report.overflow {
        Overflow::Spills { columns, .. } => {
            assert!(
                columns.0 < 4,
                "a number spills leftward: {:?}",
                report.overflow
            );
            assert_eq!(columns.1, 4, "and not rightward");
        }
        other => panic!("expected a leftward spill, got {other:?}"),
    }
}

#[test]
fn a_short_label_does_not_spill_at_all() {
    // The fourth value of the parameter: a gate that only ever saw long labels could not tell a
    // correct implementation from one that always reports a spill.
    let body = r#"<sheetFormatPr defaultRowHeight="15" defaultColWidth="20"/>
<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>ok</t></is></c></row></sheetData>"#;
    let grid = grid_from(body, &styles(&[], ""));
    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let _ = support::lay_out(&mut model, &grid, &constraints, 0);
    let report = model.catalogue().cell(0, 0).expect("A1 was laid out");
    assert!(
        matches!(report.overflow, Overflow::Fits),
        "{:?}",
        report.overflow
    );
}

#[test]
fn shrink_to_fit_makes_the_text_smaller_rather_than_letting_it_leave() {
    let shrinking = styles(
        &[
            r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="0" applyAlignment="1"><alignment shrinkToFit="1"/></xf>"#,
        ],
        "",
    );
    let body = sheet("", 1);
    let grid = grid_from(&body, &shrinking);
    let constraints = viewport(10.0, 5.0);
    let mut model = model();
    let tree = support::lay_out(&mut model, &grid, &constraints, 0);
    let report = model.catalogue().cell(0, 0).expect("A1 was laid out");

    assert!(
        report.shrink_scale < 1.0,
        "the font was scaled down: {}",
        report.shrink_scale
    );
    assert!(
        report.shrink_scale > 0.0,
        "and not to nothing: {}",
        report.shrink_scale
    );
    assert!(
        matches!(report.overflow, Overflow::Fits),
        "a shrunk cell has nothing left to overflow with: {:?}",
        report.overflow
    );
    assert!(
        text_right(&tree) <= column_left(&body, &shrinking, 1) + 1,
        "and the glyphs are inside the cell"
    );
}
