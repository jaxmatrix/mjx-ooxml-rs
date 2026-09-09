//! **The loss, asserted rather than described.**
//!
//! A cell's border reaches the display list as a *filled band* — a box the width of one line, filled
//! with that line's colour — because [`mjx_scene::Decoration`] carries **one** stroke and a cell has
//! four edges that differ in weight, colour and style. `mjx_layout_xlsx::border` says why that is
//! the only shape expressible in `mjx-layout`'s closed six-kind fragment vocabulary.
//!
//! A filled rectangle is solid, so **a dash is lost**: `<bottom style="dashed"/>` draws as a solid
//! line of the right weight and the right colour. This file asserts that, for the same reason
//! `mjx-scene-pptx/tests/the_opacity_is_lost_at_the_spec_boundary.rs` asserts its own loss — so that
//! closing it is a test going red and being *deleted*, rather than a thing nobody remembers was
//! ever wrong.
//!
//! # What is deliberately **not** lost, and why that matters
//!
//! The band's decoration carries the whole `BorderEdge`, style included. So the information a dashed
//! border needs is *in the catalogue*, one accessor away, and the child that draws one changes two
//! crates rather than recovering a value this one threw out. The route is written down in
//! `mjx_layout_xlsx::border`: a `ShapeFragment` whose `GeometryProvider` answers with the edge's
//! centre line and whose decoration carries a dashed stroke. This suite asserts the value is still
//! there, which is the half of the loss that is a *choice* rather than a limitation.

mod support;

use mjx_layout::Fragment;
use mjx_ooxml_types::spreadsheetml::BorderStyle;
use mjx_scene::{FillStyle, ResourceResolver};

use support::{resolve, styles, viewport, workbook, worksheet};

const SHEET: &str = r#"<dimension ref="A1:A1"/>
<sheetData><row r="1"><c r="A1" s="1" t="inlineStr"><is><t>x</t></is></c></row></sheetData>"#;

/// A cell with a dashed bottom edge in a colour nothing else in the sheet uses.
fn book() -> mjx_xlsx::Workbook {
    workbook(
        &worksheet(SHEET),
        &styles(
            "",
            "",
            r#"<border><left/><right/><top/><bottom style="dashed"><color rgb="FF3366CC"/></bottom><diagonal/></border>"#,
            &[
                r#"<xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="true"/>"#,
            ],
        ),
    )
}

#[test]
fn a_dashed_border_draws_solid_in_the_right_colour() {
    let mut book = book();
    let constraints = viewport(4.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);

    let bands: Vec<_> = resolved
        .tree
        .nodes()
        .filter_map(|(_, node)| match node.fragment() {
            Fragment::Box(box_fragment) if box_fragment.cell.is_none() => box_fragment.decoration,
            _ => None,
        })
        .collect();
    assert_eq!(bands.len(), 1, "one stated edge is one band");

    let decoration = resolved
        .resources
        .decoration(bands[0])
        .expect("the band's handle resolves");
    assert_eq!(
        decoration.fill,
        FillStyle::Solid(mjx_scene::Color {
            red: 0x33,
            green: 0x66,
            blue: 0xcc,
            alpha: 0xff,
        }),
        "the band resolved to {:?}. **The colour is not the loss** — that has to be exactly what the \
         file states, or the border is simply wrong rather than simplified.",
        decoration.fill
    );
    assert!(
        decoration.stroke.is_none(),
        "the band resolved with a stroke. A band is a *filled* rectangle; stroking it would outline \
         a rectangle the width of a line, which is two parallel lines rather than one."
    );
}

/// The value the fix will need, still in the catalogue.
///
/// **This is the assertion that makes the loss recoverable rather than permanent.** If a later
/// change stops carrying the style — because "nothing reads it" — the dash becomes unrecoverable
/// without re-reading the document, and that is a much larger fix than the one this leaves open.
#[test]
fn the_style_the_band_cannot_draw_is_still_recorded() {
    let mut book = book();
    let constraints = viewport(4.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);

    let handle = resolved
        .tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::Box(box_fragment) if box_fragment.cell.is_none() => box_fragment.decoration,
            _ => None,
        })
        .expect("the band");
    let entry = resolved
        .resources
        .catalogue()
        .decoration(handle)
        .expect("the catalogue holds it");
    let band = entry
        .border_band
        .as_ref()
        .expect("a band's decoration says which edge it draws");
    assert_eq!(
        band.style,
        BorderStyle::Dashed,
        "the catalogue records the band as {:?}. The renderer cannot draw a dash today; throwing \
         the style away would mean it never can without reading the document again.",
        band.style
    );
    assert!(
        band.colour.is_some(),
        "the stated colour is recorded beside the style"
    );
}

/// A cell's own decoration is never a band, and a band's is never a cell — the two states are
/// exclusive, and a reader that confused them would paint a cell's fill in its border's colour.
#[test]
fn a_cell_and_a_band_are_never_the_same_decoration() {
    let mut book = book();
    let constraints = viewport(4.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);
    let catalogue = resolved.resources.catalogue();

    for (_, node) in resolved.tree.nodes() {
        let Fragment::Box(box_fragment) = node.fragment() else {
            continue;
        };
        let Some(handle) = box_fragment.decoration else {
            continue;
        };
        let entry = catalogue.decoration(handle).expect("every handle resolves");
        assert_eq!(
            box_fragment.cell.is_some(),
            entry.border_band.is_none(),
            "a fragment that is {} carries a decoration whose `border_band` is {}",
            if box_fragment.cell.is_some() {
                "a cell"
            } else {
                "a band"
            },
            if entry.border_band.is_some() {
                "set"
            } else {
                "absent"
            }
        );
    }
}
