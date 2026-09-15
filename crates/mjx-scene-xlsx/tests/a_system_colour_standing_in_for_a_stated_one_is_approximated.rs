//! A system colour drawn in place of a colour the file states and this build cannot resolve is counted as an approximation (MJXOFF-299).

mod support;

use mjx_scene::{build_scene, Command, DisplayList, SceneLossKind, SceneOptions};
use mjx_text::GlyphAtlas;

use support::{resolve, styles, viewport, workbook, worksheet};

const ONE_CELL: &str =
    r#"<dimension ref="A1:A1"/><sheetData><row r="1"><c r="A1" s="1"/></row></sheetData>"#;

// The display list of a one-cell sheet whose cell takes `fill` and `border`.
fn list_with(fill: &str, border: &str) -> DisplayList {
    let fill_id = if fill.is_empty() { 0 } else { 2 };
    let border_id = if border.is_empty() { 0 } else { 1 };
    let mut book = workbook(
        &worksheet(ONE_CELL),
        &styles(
            "",
            fill,
            border,
            &[&format!(
                r#"<xf numFmtId="0" fontId="0" fillId="{fill_id}" borderId="{border_id}" xfId="0" applyFill="true" applyBorder="true"/>"#
            )],
        ),
    );
    let constraints = viewport(4.0, 2.0);
    let mut resolved = resolve(&mut book, 0, &constraints);
    build_scene(
        &resolved.tree,
        &resolved.resources,
        resolved.model.rasteriser_mut(),
        &mut GlyphAtlas::new(),
        &SceneOptions::new(constraints.page),
    )
    .expect("no loss fails a scene")
}

// How many fills the list draws.
fn fills(list: &DisplayList) -> usize {
    list.commands()
        .filter(|command| matches!(command, Command::FillPath { .. }))
        .count()
}

#[test]
fn a_border_whose_stated_colour_does_not_resolve_is_drawn_and_approximated() {
    let list = list_with(
        "",
        r#"<border><left/><right style="thin"><color theme="99"/></right><top/><bottom/><diagonal/></border>"#,
    );
    assert_eq!(
        (
            list.losses().count(SceneLossKind::PaintApproximated),
            list.losses().len(),
            list.placeholders().len()
        ),
        (1, 1, 0),
        "the edge is drawn in the system colour and counted"
    );
    assert!(fills(&list) > 0, "the band still draws");
}

#[test]
fn a_border_and_a_hatch_that_state_no_colour_take_excels_automatic_colours_and_lose_nothing() {
    let list = list_with(
        r#"<fill><patternFill patternType="darkGrid"/></fill>"#,
        r#"<border><left/><right style="thin"/><top/><bottom/><diagonal/></border>"#,
    );
    assert_eq!(list.losses().len(), 0);
    assert!(fills(&list) >= 2, "the hatch and the band both draw");
}

#[test]
fn a_hatch_whose_stated_colour_does_not_resolve_is_drawn_and_approximated() {
    let list = list_with(
        r#"<fill><patternFill patternType="darkGrid"><fgColor theme="99"/><bgColor rgb="FFFFFFFF"/></patternFill></fill>"#,
        "",
    );
    assert_eq!(
        (
            list.losses().count(SceneLossKind::PaintApproximated),
            list.losses().len(),
            list.placeholders().len()
        ),
        (1, 1, 0)
    );
}
