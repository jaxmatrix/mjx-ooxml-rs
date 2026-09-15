//! Each content this box model reads and cannot lay out is counted at the cell that holds it (MJXOFF-299).

mod support;

use mjx_layout::{BoxModel, FrameContent, LayoutLossKind, LayoutLosses, PageIndex};

use support::{grid_from, model, styles, viewport};

// The losses of the first band of a sheet authored from `body` and `styles_markup`.
fn losses(body: &str, styles_markup: &[u8]) -> LayoutLosses {
    let grid = grid_from(body, styles_markup);
    let mut model = model();
    model
        .layout_page(&grid, PageIndex::FIRST, &viewport(6.0, 4.0), None)
        .expect("the band lays out")
        .losses()
        .clone()
}

// A border table whose second border is `border`.
fn borders(border: &str) -> String {
    format!(
        r#"<borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border>{border}</borders>"#
    )
}

const BORDERED: &str = r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="1" applyBorder="1"/>"#;

#[test]
fn every_chosen_icon_is_a_picture_not_laid_out_over_its_cell() {
    let found = losses(
        r#"<sheetData><row r="1"><c r="A1"><v>10</v></c></row><row r="2"><c r="A2"><v>20</v></c></row><row r="3"><c r="A3"><v>30</v></c></row></sheetData><conditionalFormatting sqref="A1:A3"><cfRule type="iconSet" priority="1"><iconSet iconSet="3Arrows"><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule></conditionalFormatting>"#,
        &styles(&[], ""),
    );
    let picture = LayoutLossKind::FrameContentNotLaidOut(FrameContent::Picture);
    assert_eq!(found.count(picture), 3);
    assert_eq!(found.len(), 3);
    assert!(found.iter().all(|loss| loss.area.is_some()));
}

#[test]
fn a_flagged_diagonal_is_content_not_read_and_an_unflagged_one_is_nothing() {
    let flagged = losses(
        r#"<sheetData><row r="2"><c r="B2" s="1"/></row></sheetData>"#,
        &styles(
            &[BORDERED],
            &borders(
                r#"<border diagonalDown="1"><left/><right/><top/><bottom/><diagonal style="thin"><color rgb="FF000000"/></diagonal></border>"#,
            ),
        ),
    );
    assert_eq!(flagged.count(LayoutLossKind::DroppedByReader), 1);
    assert_eq!(flagged.len(), 1);
    let loss = flagged.iter().next().expect("one loss");
    assert_eq!(loss.source.path().segments(), &[1, 1]);

    let unflagged = losses(
        r#"<sheetData><row r="2"><c r="B2" s="1"/></row></sheetData>"#,
        &styles(
            &[BORDERED],
            &borders(
                r#"<border><left/><right/><top/><bottom/><diagonal style="thin"><color rgb="FF000000"/></diagonal></border>"#,
            ),
        ),
    );
    assert_eq!(
        unflagged.len(),
        0,
        "a diagonal neither flag asks for draws nothing"
    );
}

#[test]
fn a_dashed_edge_drawn_solid_is_approximated_and_a_solid_edge_is_not() {
    let dashed = losses(
        r#"<sheetData><row r="1"><c r="A1" s="1"/></row></sheetData>"#,
        &styles(
            &[BORDERED],
            &borders(
                r#"<border><left/><right style="dashed"><color rgb="FF000000"/></right><top/><bottom/><diagonal/></border>"#,
            ),
        ),
    );
    assert_eq!(dashed.count(LayoutLossKind::ValueApproximated), 1);
    assert!(
        dashed.iter().all(|loss| loss.area.is_none()),
        "an approximation draws no placeholder"
    );

    let solid = losses(
        r#"<sheetData><row r="1"><c r="A1" s="1"/></row></sheetData>"#,
        &styles(
            &[BORDERED],
            &borders(
                r#"<border><left/><right style="thin"><color rgb="FF000000"/></right><top/><bottom/><diagonal/></border>"#,
            ),
        ),
    );
    assert_eq!(solid.len(), 0);
}

#[test]
fn a_string_of_formatted_runs_is_approximated_and_a_plain_string_is_not() {
    let rich = losses(
        r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><r><rPr><b/></rPr><t>Bold</t></r><r><t xml:space="preserve"> plain</t></r></is></c></row></sheetData>"#,
        &styles(&[], ""),
    );
    assert_eq!(rich.count(LayoutLossKind::ValueApproximated), 1);
    assert_eq!(rich.len(), 1);

    let plain = losses(
        r#"<sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>plain</t></is></c></row></sheetData>"#,
        &styles(&[], ""),
    );
    assert_eq!(plain.len(), 0);
}
