//! A colour a worksheet states and this build cannot resolve is counted where the fill is built (MJXOFF-299).

mod support;

use mjx_scene::{build_scene, Command, DisplayList, SceneLossKind, SceneOptions};
use mjx_text::GlyphAtlas;

use support::{resolve, styles, viewport, workbook, worksheet};

const ONE_CELL: &str =
    r#"<dimension ref="A1:A1"/><sheetData><row r="1"><c r="A1" s="1"/></row></sheetData>"#;

// The display list of a one-cell sheet whose cell takes `fill` and `border`.
pub(crate) fn list_with(fill: &str, border: &str) -> DisplayList {
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
pub(crate) fn fills(list: &DisplayList) -> usize {
    list.commands()
        .filter(|command| matches!(command, Command::FillPath { .. }))
        .count()
}

#[test]
fn a_solid_fill_whose_colour_does_not_resolve_is_one_placeholder() {
    let list = list_with(
        r#"<fill><patternFill patternType="solid"><fgColor theme="99"/></patternFill></fill>"#,
        "",
    );
    assert_eq!(
        (
            list.losses().count(SceneLossKind::ColourNotResolved),
            list.placeholders().len()
        ),
        (1, 1),
        "the fill paints nothing, so its cell stands under a placeholder"
    );
}

#[test]
fn a_gradient_stop_that_does_not_resolve_is_counted_and_the_rest_is_drawn() {
    let resolved = list_with(
        r#"<fill><gradientFill degree="90"><stop position="0"><color rgb="FFFF0000"/></stop><stop position="1"><color rgb="FF0000FF"/></stop></gradientFill></fill>"#,
        "",
    );
    let list = list_with(
        r#"<fill><gradientFill degree="90"><stop position="0"><color rgb="FFFF0000"/></stop><stop position="1"><color theme="99"/></stop></gradientFill></fill>"#,
        "",
    );
    assert_eq!(
        (
            list.losses().count(SceneLossKind::ColourNotResolved),
            list.placeholders().len(),
            fills(&list)
        ),
        (1, 0, fills(&resolved)),
        "one stop is lost and the gradient still draws"
    );
}

#[test]
fn a_differential_fill_whose_colour_does_not_resolve_is_counted() {
    let list = list_with(
        r#"<fill><patternFill><bgColor theme="99"/></patternFill></fill>"#,
        "",
    );
    assert_eq!(
        (
            list.losses().count(SceneLossKind::ColourNotResolved),
            list.placeholders().len()
        ),
        (1, 1)
    );
}
