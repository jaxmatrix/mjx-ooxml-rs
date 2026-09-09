//! **A committed workbook's own fills, borders and font colour, resolved.**
//!
//! A resolver that only ever answers colours a test authored is a resolver proved against its own
//! author. `style_resources.xlsx` is the corpus's styles fixture and it states, in one cell, almost
//! everything this crate has to translate: a `path` gradient whose first stop addresses the theme by
//! position and whose second states its own `@rgb`, four border edges of four different weights
//! whose colours are spelled four different ways (`@rgb`, `@indexed`, `@theme`+`@tint`, `@auto`),
//! and a font that states a colour of its own.
//!
//! That fixture is also where the **indexed-palette alpha** shows up: `<left style="medium"><color
//! indexed="8"/></left>` resolves through ECMA-376 §18.8.27's table, whose every row is printed with
//! an ARGB alpha of `00`. Read as an opacity that border is invisible, and this suite is what says
//! it is not.

mod support;

use mjx_layout::{DecorationRef, Fragment};
use mjx_layout_xlsx::{SheetBoxModel, SheetGrid};
use mjx_scene::{FillStyle, ResourceResolver};
use mjx_scene_xlsx::{SheetPalette, SheetResources};
use mjx_xlsx::Workbook;

use support::{fixture, resolver, viewport};

/// The fixture, laid out, with its palette built from its own theme and styles parts.
fn resolved() -> (mjx_layout::FragmentTree, SheetResources) {
    let mut book = Workbook::open(&fixture("style_resources.xlsx")).expect("a well-formed package");
    let theme = book
        .theme_colors()
        .expect("the theme part reads")
        .expect("`style_resources.xlsx` relates a theme part");
    let grid = SheetGrid::read(&book, 0).expect("the sheet reads");
    let constraints = viewport(8.0, 5.0);
    let mut model = SheetBoxModel::new(resolver());
    let page = mjx_layout::BoxModel::layout_page(
        &mut model,
        &grid,
        mjx_layout::PageIndex::FIRST,
        &constraints,
        None,
    )
    .expect("the band lays out");
    let (tree, _) = page.into_parts();

    let formatting = grid.formatting();
    let interner = formatting
        .resolver()
        .expect("a format resolver")
        .formats()
        .interner();
    let palette =
        SheetPalette::from_stylesheet(formatting.stylesheet(), interner).with_theme(theme);
    (
        tree,
        SheetResources::new(model.catalogue().clone(), palette),
    )
}

/// The handle the cell at `row`, `column` carries.
fn cell_handle(tree: &mjx_layout::FragmentTree, row: u32, column: u16) -> DecorationRef {
    tree.nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::Box(box_fragment) => box_fragment
                .cell
                .filter(|cell| cell.row == row && cell.column == column)
                .and(box_fragment.decoration),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no cell box at r{row}c{column}"))
}

/// Every band fragment's resolved fill, in tree order.
fn band_fills(tree: &mjx_layout::FragmentTree, resources: &SheetResources) -> Vec<FillStyle> {
    tree.nodes()
        .filter_map(|(_, node)| match node.fragment() {
            // A band is a box with a decoration and **no** `TableCell`; a cell is a box with both.
            Fragment::Box(box_fragment) if box_fragment.cell.is_none() => box_fragment.decoration,
            _ => None,
        })
        .filter_map(|handle| resources.decoration(handle))
        .map(|decoration| decoration.fill)
        .collect()
}

#[test]
fn the_gradient_reaches_both_of_its_stops() {
    let (tree, resources) = resolved();
    let decoration = resources
        .decoration(cell_handle(&tree, 0, 0))
        .expect("A1's handle resolves");
    let FillStyle::Gradient(gradient) = &decoration.fill else {
        panic!(
            "A1's `fillId=\"3\"` is a `<gradientFill>` and it resolved to {:?}. A gradient that \
             cannot reach the display list is what MJXOFF-244 found: R16 carried a bare \
             `is_gradient: bool`, which names no stops.",
            decoration.fill
        );
    };
    assert_eq!(
        gradient.stops.len(),
        2,
        "the fixture writes two `<stop>`s and {} reached the display list. A stop is dropped when \
         its colour cannot be resolved, so a count of one means the theme did not arrive.",
        gradient.stops.len()
    );
    assert_eq!(
        gradient.kind,
        mjx_scene::GradientKind::Path,
        "the fixture writes `type=\"path\"`"
    );

    // `<color theme="4" tint="-0.25"/>` — the fifth slot of the theme's `<clrScheme>`, darkened by
    // a quarter. Asserting it is *not* the raw slot colour is what proves the tint ran; asserting
    // it is not black is what proves the theme part was read at all.
    let first = gradient.stops[0].color;
    assert!(
        first.alpha == 0xff && (first.red, first.green, first.blue) != (0, 0, 0),
        "the theme stop resolved to {first:?}. Black would mean the `@theme` position found no \
         slot, which is what happens when the theme part is never opened."
    );
    let second = gradient.stops[1].color;
    assert_eq!(
        (second.red, second.green, second.blue, second.alpha),
        (0x03, 0x69, 0xa3, 0xff),
        "`rgb=\"FF0369A3\"` is opaque blue; it resolved to {second:?}"
    );
}

#[test]
fn all_four_borders_become_bands_with_a_visible_colour() {
    let (tree, resources) = resolved();
    let fills = band_fills(&tree, &resources);
    // Left, right, top, bottom — and the bottom is `double`, which is two lines.
    assert_eq!(
        fills.len(),
        5,
        "`borderId=\"1\"` states four edges and its bottom is `double`, so five bands are drawn; \
         {} reached the resolver",
        fills.len()
    );
    for fill in &fills {
        let FillStyle::Solid(colour) = fill else {
            panic!("a border band resolved to {fill:?} rather than to a solid colour");
        };
        assert_eq!(
            colour.alpha, 0xff,
            "a border band resolved to {colour:?}. **This is the indexed-palette trap**: \
             ECMA-376 §18.8.27 prints every row of the legacy palette with an alpha of `00`, and a \
             renderer that reads those two nibbles as an opacity draws no border at all — at every \
             zoom, in every export, with nothing to notice it."
        );
    }
}

#[test]
fn the_font_colour_reaches_the_text() {
    let (tree, resources) = resolved();
    // Every fragment of a cell carries the cell's own two-segment path, so a glyph run's source is
    // what `text_decoration` is addressed by.
    let source = tree
        .nodes()
        .find_map(|(_, node)| match node.fragment() {
            Fragment::GlyphRun(_) => Some(node.source().clone()),
            _ => None,
        })
        .expect("A1 holds the number 42, which lays out as a glyph run");

    let decoration = resources
        .text_decoration(&source)
        .expect("the run's cell states a font colour, so the resolver answers");
    assert_eq!(
        decoration.fill,
        FillStyle::Solid(mjx_scene::Color {
            red: 0x18,
            green: 0xa3,
            blue: 0x03,
            alpha: 0xff,
        }),
        "the fixture's `fontId=\"1\"` states `<color rgb=\"FF18A303\"/>`, and the run resolved to \
         {:?}. Answering `None` here is not a small loss: `build_scene` falls back to \
         `DEFAULT_TEXT_COLOR`, so every run in a workbook would be black and nothing would say so.",
        decoration.fill
    );
}

/// The other half of the same question: a cell that states **no** font colour must answer `None`,
/// so that `build_scene` uses its own default rather than a colour this crate invented.
#[test]
fn a_cell_with_no_stated_colour_answers_nothing() {
    let (tree, resources) = resolved();
    // B1..M1 carry the default format, which states no colour at all.
    let handle = cell_handle(&tree, 0, 1);
    let decoration = resources.decoration(handle).expect("the handle resolves");
    assert_eq!(
        decoration.fill,
        FillStyle::None,
        "an unformatted cell is not filled: `<patternFill patternType=\"none\"/>` means the sheet's \
         own background shows through, and painting it white would put a rectangle over anything \
         drawn beneath."
    );
    assert!(
        decoration.stroke.is_none(),
        "a cell's decoration never carries a stroke: its edges are their own fragments, and one \
         stroke here would draw one of the four edges on all four sides"
    );
    assert!(decoration.effects.is_empty());
}
