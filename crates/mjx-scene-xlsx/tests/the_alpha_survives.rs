//! **The counterpart of `mjx-scene-pptx`'s `the_opacity_is_lost_at_the_spec_boundary.rs`, and it
//! asserts the opposite.**
//!
//! MJXOFF-243 records a real loss: `mjx-dml`'s `resolve_fill` / `resolve_line` / `resolve_effects`
//! bake every DrawingML colour to a `ColorSpec::Srgb` hex triplet, which has no alpha channel, so
//! the `<a:alpha val="63000"/>` the standard Office theme puts on every shadow arrives opaque and
//! every theme shadow in a `.pptx` currently renders solid black.
//!
//! MJXOFF-244's brief asked whether the same loss exists on Excel's path **rather than assuming it
//! either way**, and it does not. The reason is structural rather than lucky:
//!
//! * a SpreadsheetML colour is `CT_Color`, **one element with five attributes**, and its `@rgb` is
//!   `ST_UnsignedIntHex` — eight hex digits, **alpha first**;
//! * `mjx-layout-xlsx` carries the `mjx_sml::Color` through unresolved, because resolving a theme
//!   position needs a part the box model does not hold;
//! * `mjx_sml::styles::resolve_color` answers with a `ResolvedColor` whose `alpha` is an `f64`; and
//! * this crate converts that to the byte `mjx_scene::Color` already has room for.
//!
//! So there is no six-digit hex triplet anywhere on the path, which is what makes this suite an
//! assertion and not a hope. **If somebody routes Excel's colours through `mjx-dml`'s `ColorSpec`
//! for convenience, this file goes red**, which is the whole reason it is written as a fixture
//! rather than as a paragraph.
//!
//! The assertion is taken **at the display list**, not at the resolver: `DisplayList::paint` reads
//! the encoded paint table back out of the bytes, so what is checked is the number a painter will
//! actually read.

mod support;

use mjx_layout::{DecorationRef, Fragment};
use mjx_scene::{
    build_scene, Command, DisplayList, FillStyle, Paint, ResourceResolver, SceneOptions,
};
use mjx_text::GlyphAtlas;

use support::{resolve, styles, viewport, workbook, worksheet};

/// A sheet whose one cell is filled half-transparent red, and whose text is quarter-transparent
/// blue.
const SHEET: &str = r#"<dimension ref="A1:A1"/>
<sheetData><row r="1"><c r="A1" s="1" t="inlineStr"><is><t>Half</t></is></c></row></sheetData>"#;

/// `80FF0000` is red at 50.2 % opacity, and `40 00 00 FF` is blue at 25.1 %. Both are written the
/// way a file writes them — alpha first — and neither is a round number in bytes, which is what
/// makes a rounding mistake visible.
fn book() -> mjx_xlsx::Workbook {
    workbook(
        &worksheet(SHEET),
        &styles(
            r#"<font><sz val="11"/><name val="Liberation Sans"/><color rgb="400000FF"/></font>"#,
            r#"<fill><patternFill patternType="solid"><fgColor rgb="80FF0000"/><bgColor indexed="64"/></patternFill></fill>"#,
            "",
            &[
                r#"<xf numFmtId="0" fontId="1" fillId="2" borderId="0" xfId="0" applyFont="true" applyFill="true"/>"#,
            ],
        ),
    )
}

/// Every `Paint::Solid` the list encodes, in table order.
fn solid_paints(list: &DisplayList) -> Vec<mjx_scene::Color> {
    let mut out = Vec::new();
    let mut index = 0_u32;
    while let Some(paint) = list.paint(mjx_scene::ResourceIndex::new(index)) {
        if let Paint::Solid(colour) = paint {
            out.push(colour);
        }
        index += 1;
    }
    out
}

#[test]
fn a_half_transparent_cell_fill_resolves_at_half_opacity() {
    let mut book = book();
    let constraints = viewport(4.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);

    let handle: DecorationRef = resolved
        .tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::Box(cell) if cell.cell.is_some() => cell.decoration,
            _ => None,
        })
        .expect("the cell carries a decoration handle");

    let decoration = resolved
        .resources
        .decoration(handle)
        .expect("the handle resolves");
    let FillStyle::Solid(colour) = decoration.fill else {
        panic!(
            "the cell's fill resolved to {:?} rather than to a solid colour. \
             `patternType=\"solid\"` paints its **fgColor**; reading `bgColor` there gives the \
             system window colour, which looks exactly like no fill at all.",
            decoration.fill
        );
    };
    assert_eq!(
        (colour.red, colour.green, colour.blue),
        (0xff, 0x00, 0x00),
        "`rgb=\"80FF0000\"` is AARRGGBB: the leading pair is the alpha and the colour is red. \
         Reading it as RRGGBBAA would give {colour:?}."
    );
    assert_eq!(
        colour.alpha, 0x80,
        "the fill reached the resolver at alpha {:#04x} rather than {:#04x}. This is the MJXOFF-243 \
         loss appearing on Excel's path, which it structurally cannot do unless somebody routed \
         these colours through `mjx-dml`'s `ColorSpec` — see this file's own documentation.",
        colour.alpha, 0x80_u8
    );
}

#[test]
fn the_alpha_is_still_there_in_the_encoded_display_list() {
    let mut book = book();
    let constraints = viewport(4.0, 2.0);
    let mut resolved = resolve(&mut book, 0, &constraints);
    let options = SceneOptions::new(constraints.page);
    let mut atlas = GlyphAtlas::new();
    let list = build_scene(
        &resolved.tree,
        &resolved.resources,
        resolved.model.rasteriser_mut(),
        &mut atlas,
        &options,
    )
    .expect("the fragment tree becomes a display list");

    let paints = solid_paints(&list);
    assert!(
        paints.iter().any(|colour| colour.alpha == 0x80
            && (colour.red, colour.green, colour.blue) == (0xff, 0x00, 0x00)),
        "the encoded paint table holds {paints:?} and none of them is half-transparent red. The \
         resolver answering correctly is not enough: the alpha has to survive the *encoding* as \
         well, because a painter reads the bytes and never the resolver."
    );
    assert!(
        paints.iter().any(|colour| colour.alpha == 0x40
            && (colour.red, colour.green, colour.blue) == (0x00, 0x00, 0xff)),
        "the encoded paint table holds {paints:?} and none of them is quarter-transparent blue. A \
         run's own `x:font > x:color` reaches the list through `text_decoration`, which is the \
         method PowerPoint's companion still answers `None` from."
    );
    assert!(
        list.commands()
            .any(|command| matches!(command, Command::FillPath { .. })),
        "a filled cell produced no `FillPath` command at all"
    );
}

/// The narrower loss that **does** exist, recorded where a reader of the two suites will look.
///
/// `mjx_dml::SchemeColors::from_scheme` resolves each theme slot and drops the slot's own alpha, so
/// a theme whose `<a:dk1>` carried an `<a:alpha>` would reach a cell opaque. That is one crate below
/// this one and it is a *theme-part* transform rather than a cell's own colour — no workbook this
/// project has read writes one — so it is stated rather than worked around. This test asserts the
/// **cell-level** path is unaffected by it: a `@tint` still resolves, and an `@rgb` alpha still
/// arrives, on a colour that also names a theme position.
#[test]
fn a_tint_resolves_without_touching_the_alpha_of_a_stated_colour() {
    let mut book = workbook(
        &worksheet(SHEET),
        &styles(
            "",
            // Two stops: one addressing the theme by position with a tint, one stating its own
            // half-transparent colour. The second is the one this asserts.
            r#"<fill><gradientFill type="linear" degree="90"><stop position="0"><color theme="4" tint="-0.25"/></stop><stop position="1"><color rgb="8012AB34"/></stop></gradientFill></fill>"#,
            "",
            &[r#"<xf numFmtId="0" fontId="0" fillId="2" borderId="0" xfId="0" applyFill="true"/>"#],
        ),
    );
    let constraints = viewport(4.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);
    let handle: DecorationRef = resolved
        .tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::Box(cell) if cell.cell.is_some() => cell.decoration,
            _ => None,
        })
        .expect("the cell carries a decoration handle");
    let decoration = resolved
        .resources
        .decoration(handle)
        .expect("the handle resolves");
    let FillStyle::Gradient(gradient) = &decoration.fill else {
        panic!("the gradient fill resolved to {:?}", decoration.fill);
    };
    let stated = gradient
        .stops
        .iter()
        .find(|stop| stop.color.red == 0x12)
        .expect("the stop that states its own colour");
    assert_eq!(
        stated.color.alpha, 0x80,
        "a gradient stop's stated alpha was lost. Every stop goes through the same resolution as a \
         solid fill, so losing it here means losing it everywhere."
    );
}
