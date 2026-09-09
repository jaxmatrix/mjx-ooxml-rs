//! `[Red]` in a number format is a colour a **painter** has to see, and this is where it arrives.
//!
//! # Why this is a suite of its own
//!
//! Every other colour in a worksheet is a property of the cell's *style*: a font's `<color>`, a
//! fill's `<fgColor>`, a border's. A number format's colour is not. `#,##0.00;[Red]#,##0.00` states
//! one `xf`, and whether a cell is red depends on **the number in it** — so two cells with the same
//! effective format, the same font and the same fill take different colours, and every mechanism
//! this crate and `mjx-layout-xlsx` share for *not* duplicating a decoration works against that.
//!
//! MJXOFF-172 answered it by keying the box model's decoration table on the pair
//! `(effective format, number-format colour)` and carrying the colour on
//! [`mjx_layout_xlsx::Decoration::text_colour`] as a **row of `indexedColors`**, unresolved — for
//! exactly the reason a `mjx_sml::Color` arrives here unresolved: resolving a palette row is this
//! crate's job, and a box model that resolved one would have merged two stages.
//!
//! The assertion is taken **at the encoded display list** as well as at the resolver, because a
//! painter reads the bytes and never the resolver.

mod support;

use mjx_layout::{Fragment, SourceRef};
use mjx_scene::{build_scene, DisplayList, FillStyle, Paint, ResourceResolver, SceneOptions};
use mjx_text::GlyphAtlas;

use support::{resolve, styles, viewport, workbook, worksheet};

/// Three cells in one `xf`: a positive, a negative and another positive.
const SHEET: &str = r#"<dimension ref="A1:C1"/>
<sheetData><row r="1">
<c r="A1" s="1"><v>5</v></c>
<c r="B1" s="1"><v>-5</v></c>
<c r="C1" s="1"><v>7</v></c>
</row></sheetData>"#;

/// A workbook whose one custom format colours its negative section red and says nothing about the
/// font, so that the only thing that can make a cell red is the format code.
fn book() -> mjx_xlsx::Workbook {
    let base = styles(
        "",
        "",
        "",
        &[r#"<xf numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
    );
    let text = String::from_utf8(base).expect("the support markup is UTF-8");
    let numfmts = r##"<numFmts count="1"><numFmt numFmtId="164" formatCode="#,##0.00;[Red]#,##0.00"/></numFmts>"##;
    let markup = text
        .replace(
            "<fonts count=\"9\">",
            &format!("{numfmts}<fonts count=\"9\">"),
        )
        .into_bytes();
    workbook(&worksheet(SHEET), &markup)
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
fn the_negative_cells_text_resolves_red_and_its_neighbours_does_not() {
    let mut book = book();
    let constraints = viewport(6.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);

    // Every glyph run, by the cell it came from, with whatever `text_decoration` answered — which is
    // `None` for a cell whose font states no `<color>`, and is the pre-existing behaviour: a painter
    // draws such a run in the window's own foreground rather than in a colour the file never named.
    let mut answered: Vec<(u16, Option<mjx_scene::Color>)> = Vec::new();
    for (_, node) in resolved.tree.nodes() {
        if !matches!(node.fragment(), Fragment::GlyphRun(_)) {
            continue;
        }
        let source: SourceRef = node.source().clone();
        let Some(hit) = mjx_layout_xlsx::CellHit::from_source(&source) else {
            continue;
        };
        let colour = resolved
            .resources
            .text_decoration(&source)
            .map(|decoration| match decoration.fill {
                FillStyle::Solid(colour) => colour,
                other => panic!("a glyph run's text decoration resolved to {other:?}"),
            });
        answered.push((hit.column, colour));
    }
    answered.sort_by_key(|(column, _)| *column);
    answered.dedup_by_key(|(column, _)| *column);

    assert_eq!(
        answered.len(),
        3,
        "three cells produced glyph runs; {answered:?}"
    );
    let colour_of = |column: u16| {
        answered
            .iter()
            .find(|(found, _)| *found == column)
            .map(|(_, colour)| *colour)
            .expect("the cell answered")
    };
    let red = colour_of(1).expect(
        "the negative cell's format names `[Red]`, so its text decoration must resolve to a          colour even though its font states none",
    );
    assert_eq!(
        (red.red, red.green, red.blue),
        (0xff, 0x00, 0x00),
        "`[Red]` is row two of `indexedColors`, which is `00FF0000`. The negative cell resolved to          {red:?} instead."
    );
    assert_eq!(
        red.alpha, 0xff,
        "the palette prints every row with an alpha of `00`, which is a BIFF artefact and not an \
         opacity. Reading it as one makes every `[Red]` in every workbook invisible."
    );
    for column in [0_u16, 2] {
        assert_eq!(
            colour_of(column),
            None,
            "column {column} is positive, its section names no colour and its font states none, so \
             it must answer nothing rather than inherit the red its neighbour asked for"
        );
    }
}

#[test]
fn the_red_is_still_there_in_the_encoded_display_list() {
    let mut book = book();
    let constraints = viewport(6.0, 2.0);
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
        paints.iter().any(
            |colour| (colour.red, colour.green, colour.blue, colour.alpha)
                == (0xff, 0x00, 0x00, 0xff)
        ),
        "the encoded paint table holds {paints:?} and none of them is opaque red. The resolver \
         answering correctly is not enough: a painter reads the bytes."
    );
}

/// `[Color15]` is the fifteenth row of `indexedColors`, not the fifteenth colour name.
///
/// The two spellings address one table — `[Red]` *is* `[Color3]` — and a workbook that replaces
/// `indexedColors` moves both. This asserts the second half of that: a replaced palette changes what
/// `[Color3]` paints, which is what makes the row-rather-than-colour design load-bearing.
#[test]
fn a_replaced_indexed_palette_moves_what_a_format_colour_paints() {
    let mut rows = String::new();
    for entry in 0..64 {
        // Every row green except row two, which is the one `[Red]` names.
        let colour = if entry == 2 { "0000CC00" } else { "00123456" };
        rows.push_str(&format!(r#"<rgbColor rgb="{colour}"/>"#));
    }
    let base = styles(
        "",
        "",
        "",
        &[r#"<xf numFmtId="164" fontId="0" fillId="0" borderId="0" applyNumberFormat="1"/>"#],
    );
    let text = String::from_utf8(base).expect("the support markup is UTF-8");
    let numfmts = r##"<numFmts count="1"><numFmt numFmtId="164" formatCode="#,##0.00;[Red]#,##0.00"/></numFmts>"##;
    let colors = format!("<colors><indexedColors>{rows}</indexedColors></colors>");
    let markup = text
        .replace(
            "<fonts count=\"9\">",
            &format!("{numfmts}<fonts count=\"9\">"),
        )
        .replace("</styleSheet>", &format!("{colors}</styleSheet>"))
        .into_bytes();
    let mut book = workbook(&worksheet(SHEET), &markup);

    let constraints = viewport(6.0, 2.0);
    let resolved = resolve(&mut book, 0, &constraints);
    let mut found = None;
    for (_, node) in resolved.tree.nodes() {
        if !matches!(node.fragment(), Fragment::GlyphRun(_)) {
            continue;
        }
        let source = node.source().clone();
        let Some(hit) = mjx_layout_xlsx::CellHit::from_source(&source) else {
            continue;
        };
        if hit.column != 1 {
            continue;
        }
        if let Some(decoration) = resolved.resources.text_decoration(&source) {
            if let FillStyle::Solid(colour) = decoration.fill {
                found = Some(colour);
            }
        }
    }
    let colour = found.expect("the negative cell's text decoration resolved");
    assert_eq!(
        (colour.red, colour.green, colour.blue),
        (0x00, 0xcc, 0x00),
        "the workbook replaced `indexedColors`, so `[Red]` paints whatever it put in row two — \
         {colour:?} says the palette was ignored and a literal red was written somewhere instead"
    );
}
