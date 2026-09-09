//! **The first end-to-end worksheet**: a committed `.xlsx` travelling the whole pipeline, in one
//! process.
//!
//! ```text
//! Workbook → SheetGrid → SheetBoxModel → FragmentTree
//!          → SheetResources + SheetGeometry → build_scene → DisplayList
//!          → SoftwarePainter → pixels
//! ```
//!
//! R16 built Excel's box model and named its own weakest part: *"nobody has looked at a picture, and
//! here that is not a criticism of the gate — there is no path to one."* MJXOFF-244 built the path;
//! this suite is where it is walked. It lives here for the same reason
//! `a_real_deck_reaches_pixels.rs` does: `mjx-reference-pack` is the only crate in the workspace
//! that may name a format crate (3.0) and `mjx-paint` (5.5) together, because 5.5 is above 3.0 and
//! any crate holding both must sit above the whole graph.
//!
//! # ⚠ This is not parity, and it is not a golden image
//!
//! It proves the **plumbing**: that a sheet's cells, borders and glyphs reach the display list, that
//! the display list reaches a rasteriser, and that ink lands where the fragments said it would.
//! Whether that ink looks like Excel's is a question only a human sitting against real Microsoft
//! Excel on Windows answers (`docs/validation/07-the-reference-pack.md`). LibreOffice is a change
//! detector, and the user has said its export of shades and gradients is not to be trusted even as
//! that.
//!
//! # The vacuity trap, and what is done about it
//!
//! *"A worksheet renders"* is satisfied by a renderer that draws a blank page without erroring. So
//! every assertion below is on **content**: how many fragments of each kind, how many of them are
//! border bands, that `DrawReport::placeholders` matches a number computed **before** the render,
//! that ink covers a stated fraction of the page with a floor *and* a ceiling, and that the ink sits
//! inside the rectangles the fragment tree named.
//!
//! # ⚠ The hairline, which is the assertion this suite exists for as much as any other
//!
//! R15 found `mjx-layout-pptx` flooring a hairline border at one EMU and halving it to zero — a
//! border invisible at every zoom with nothing to notice it — and **Excel is made of hairline
//! borders**. `mjx-layout-xlsx` draws `style="hair"` at half a point, which at the unzoomed 96 dpi
//! scale is **two thirds of a device pixel**: thin enough that a painter which dropped sub-pixel
//! coverage would draw nothing, and the fragment-tier gate in `mjx-layout-xlsx` cannot see that
//! because it has no pixels. [`a_hairline_border_puts_ink_on_the_page`] is the half of that question
//! only a painter can answer.

use mjx_layout::{BoxModel, Constraints, Fragment, FragmentTree, LayoutSize, PageIndex};
use mjx_layout_xlsx::{constraints_for, SheetBoxModel, SheetGrid};
use mjx_ooxml_core::measure::Emu;
use mjx_paint::{
    render_offscreen, DrawReport, NoImages, Pixels, Resources, SoftwarePainter, SOFTWARE_PAINTER,
};
use mjx_scene::{build_scene, Command, DisplayList, SceneOptions};
use mjx_scene_xlsx::{SheetGeometry, SheetPalette, SheetResources};
use mjx_text::{FontResolver, GlyphAtlas};
use mjx_xlsx::{PartName, Workbook};

/// The bundled faces, and nothing the platform happens to have installed.
///
/// A render gate that resolved faces through the operating system would draw one image on a
/// developer's machine and another on CI, and the difference would look like a rendering
/// regression.
fn resolver() -> FontResolver {
    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    FontResolver::builder()
        .with_bundled_font_directory(&fonts)
        .expect("the committed faces index")
        .build()
}

/// What one band's trip through the pipeline produced.
struct Journey {
    tree: FragmentTree,
    list: DisplayList,
    drawn: DrawReport,
    pixels: Pixels,
    /// How many outline handles the provider could not answer from the document's own geometry,
    /// taken from the provider **before** the render so that the placeholder assertion compares two
    /// independent numbers rather than a number with itself.
    unregistered_outlines: usize,
}

/// A viewport of `width` by `height` inches.
fn viewport(width: f64, height: f64) -> Constraints {
    constraints_for(LayoutSize {
        width: Emu::from_inches(width),
        height: Emu::from_inches(height),
    })
}

/// Runs the first band of `book`'s sheet `sheet` all the way to pixels.
fn journey(book: &mut Workbook, sheet: usize, constraints: &Constraints) -> Journey {
    // The theme is read through the workbook itself: a SpreadsheetML colour addresses the scheme by
    // *position*, so `<color theme="4"/>` means nothing without `xl/theme/theme1.xml`, and this
    // crate holds the package — which is exactly the division `SheetPalette` is shaped for.
    let theme = book.theme_colors().expect("the theme part reads");
    let grid = SheetGrid::read(book, sheet).expect("the sheet reads");

    let mut model = SheetBoxModel::new(resolver());
    let page = model
        .layout_page(&grid, PageIndex::FIRST, constraints, None)
        .expect("the band lays out");
    let (tree, _) = page.into_parts();

    let formatting = grid.formatting();
    let interner = formatting
        .resolver()
        .expect("a format resolver")
        .formats()
        .interner();
    let mut palette = SheetPalette::from_stylesheet(formatting.stylesheet(), interner);
    if let Some(theme) = theme {
        palette = palette.with_theme(theme);
    }
    let resources = SheetResources::new(model.catalogue().clone(), palette);

    // A worksheet's fragment tree carries no `ShapeFragment`, so this provider answers no handle —
    // and that is the number the placeholder assertion is taken against.
    let geometry = SheetGeometry::new();
    let unregistered_outlines = geometry.unregistered();

    let options = SceneOptions::new(constraints.page);
    let mut atlas = GlyphAtlas::new();
    let list = build_scene(
        &tree,
        &resources,
        model.rasteriser_mut(),
        &mut atlas,
        &options,
    )
    .expect("the fragment tree becomes a display list");

    let (page_width, page_height) = list.page_size();
    let width = page_width.ceil().max(1.0) as u32;
    let height = page_height.ceil().max(1.0) as u32;

    let mut painter = SoftwarePainter::new();
    let images = NoImages;
    // The scene builder's own atlas *is* the painter's glyph source: `mjx-paint` implements
    // `AtlasSource` for it, so the glyph images the builder placed reach the rasteriser as a delta
    // rather than through a second atlas that would have to be kept in step. Two atlases here draws
    // every glyph from an empty page, with no error anywhere — which R15 shipped once and caught.
    let mut paint_resources = Resources::new(&mut atlas, &geometry, &images);

    let render = render_offscreen(
        &mut painter,
        &list,
        width,
        height,
        1.0,
        &mut paint_resources,
    )
    .expect("the display list rasterises");
    assert_eq!(
        render.painter, SOFTWARE_PAINTER,
        "this gate must run on the pure-Rust painter, so that it needs no graphics stack"
    );

    Journey {
        tree,
        list,
        drawn: render.drawn,
        pixels: render.pixels,
        unregistered_outlines,
    }
}

/// A committed fixture, opened.
fn fixture(name: &str) -> Workbook {
    Workbook::open(&mjx_fixtures::fixture(name)).unwrap_or_else(|error| panic!("{name}: {error}"))
}

/// How many fragments of each kind a tree holds, plus how many boxes are **border bands** — a box
/// with a decoration and no `TableCell`.
struct Kinds {
    boxes: usize,
    cells: usize,
    bands: usize,
    lines: usize,
    glyphs: usize,
    tables: usize,
    shapes: usize,
    images: usize,
}

fn kinds(tree: &FragmentTree) -> Kinds {
    let mut counts = Kinds {
        boxes: 0,
        cells: 0,
        bands: 0,
        lines: 0,
        glyphs: 0,
        tables: 0,
        shapes: 0,
        images: 0,
    };
    for (_, node) in tree.nodes() {
        match node.fragment() {
            Fragment::Box(box_fragment) => {
                counts.boxes += 1;
                if box_fragment.cell.is_some() {
                    counts.cells += 1;
                } else if box_fragment.decoration.is_some() {
                    counts.bands += 1;
                }
            }
            Fragment::Line(_) => counts.lines += 1,
            Fragment::GlyphRun(_) => counts.glyphs += 1,
            Fragment::Image(_) => counts.images += 1,
            Fragment::Shape(_) => counts.shapes += 1,
            Fragment::Table(_) => counts.tables += 1,
        }
    }
    counts
}

/// How many pixels of `pixels` are not the background, and the smallest box containing them.
fn ink(pixels: &Pixels) -> (usize, Option<(u32, u32, u32, u32)>) {
    let mut count = 0;
    let mut bounds: Option<(u32, u32, u32, u32)> = None;
    for y in 0..pixels.height {
        for x in 0..pixels.width {
            let Some([red, green, blue, alpha]) = pixels.pixel(x, y) else {
                continue;
            };
            // Premultiplied, so a fully transparent pixel is all zeroes and anything that painted
            // is not.
            if alpha == 0 && red == 0 && green == 0 && blue == 0 {
                continue;
            }
            if alpha == 0 {
                continue;
            }
            count += 1;
            bounds = Some(match bounds {
                None => (x, y, x, y),
                Some((left, top, right, bottom)) => {
                    (left.min(x), top.min(y), right.max(x), bottom.max(y))
                }
            });
        }
    }
    (count, bounds)
}

#[test]
fn a_sheet_of_text_reaches_pixels() {
    // `sheet_grid.xlsx` rather than `sample.xlsx`: it is the one fixture in the corpus with every
    // knob turned — custom row heights, hidden rows, outline levels, a five-column `col` run, a
    // hidden column and two merged regions — so a pipeline that laid out only the simple cases
    // shows here rather than passing.
    let mut book = fixture("sheet_grid.xlsx");
    let journey = journey(&mut book, 0, &viewport(8.0, 5.0));
    let counts = kinds(&journey.tree);

    assert!(
        counts.cells >= 24,
        "{} cell boxes. The fixture's window is at least eight columns by three visible rows, so a \
         count this low means whole rows or columns were never laid out.",
        counts.cells
    );
    // A count rather than "at least one": one line and one glyph run is what a renderer that laid
    // the first label out and stopped would also report.
    assert!(
        counts.lines >= 7,
        "{} line fragments. `sheet_grid.xlsx` states text in A1, B1, A2, A5, A7 and a number in B2 \
         and D10, and the hidden rows draw nothing — so seven lines is the floor for a band that \
         laid its visible rows out.",
        counts.lines
    );
    assert_eq!(
        counts.glyphs, counts.lines,
        "a cell's text is **one string**: unlike a slide's paragraph, it has no bullet marker and \
         no per-run split, so a line and a glyph run are one to one. {} runs for {} lines means \
         something split a cell's text.",
        counts.glyphs, counts.lines
    );
    assert_eq!(
        (counts.shapes, counts.images),
        (0, 0),
        "a worksheet's fragment tree carries no shape and no picture — drawings are MJXOFF-173's \
         subject — and if it does now, `SheetGeometry` refusing every handle is no longer right"
    );
    assert!(
        counts.tables >= 1,
        "every pane region of a band is a table fragment"
    );
    assert_eq!(
        counts.bands, 0,
        "`sheet_grid.xlsx` states one `<border>` and it is empty, so nothing draws a band. A \
         non-zero count here means bands are being emitted for edges the file never stated."
    );

    // One `DrawGlyphs` per glyph-run fragment, exactly.
    let drawn_runs = journey
        .list
        .commands()
        .filter(|command| matches!(command, Command::DrawGlyphs { .. }))
        .count();
    assert_eq!(
        drawn_runs, counts.glyphs,
        "{drawn_runs} `DrawGlyphs` commands for {} glyph-run fragments. The display list is an \
         encoding of the tree, not a summary of it.",
        counts.glyphs
    );
    assert!(
        journey.drawn.glyphs > 0,
        "the painter built no glyph quads at all. A sheet of labels that draws no glyphs is the \
         exact failure an *'it rendered without erroring'* gate cannot see."
    );
    assert!(
        journey.drawn.draw_calls > 0,
        "the painter issued no draw calls at all"
    );
}

#[test]
fn a_sheet_of_fills_and_borders_reaches_pixels() {
    let mut book = fixture("style_resources.xlsx");
    let journey = journey(&mut book, 0, &viewport(8.0, 5.0));
    let counts = kinds(&journey.tree);

    assert_eq!(
        counts.bands, 5,
        "A1's `borderId=\"1\"` states four edges and its bottom is `double`, which draws two lines \
         — so five bands. {} means the edges are not reaching the tree, and a border that is not a \
         fragment cannot be a pixel: `mjx_scene::Decoration` carries **one** stroke and a cell has \
         four edges.",
        counts.bands
    );
    // The fills and the bands are `FillPath` commands; the cell's text is a `DrawGlyphs`.
    let filled = journey
        .list
        .commands()
        .filter(|command| matches!(command, Command::FillPath { .. }))
        .count();
    assert!(
        filled >= 6,
        "{filled} `FillPath` commands for one gradient-filled cell and five border bands. A band \
         whose colour resolved to nothing emits no command at all, which is what an unresolved \
         indexed palette or an unread theme part looks like from here."
    );
    assert!(
        journey.drawn.draw_calls >= 6,
        "the painter issued {} draw calls; six fills reached the display list",
        journey.drawn.draw_calls
    );
    assert!(
        journey.drawn.glyphs > 0,
        "A1 holds the number 42 and it drew no glyphs"
    );
}

#[test]
fn the_ink_lands_where_the_fragments_said_it_would() {
    let mut book = fixture("sheet_grid.xlsx");
    let journey = journey(&mut book, 0, &viewport(8.0, 5.0));
    let (count, bounds) = ink(&journey.pixels);
    let total = journey.pixels.width as usize * journey.pixels.height as usize;
    assert!(total > 0, "the render has a size");

    let fraction = count as f64 / total as f64;
    // A worksheet is mostly empty grid, so the floor is far lower than a slide's — and it is a
    // floor rather than nothing, because a blank render is the vacuous pass it exists to refuse.
    assert!(
        fraction > 0.0005,
        "only {:.4}% of the page has ink on it. A band of labels covers more than five ten-thousandths \
         of it, and a blank render is the vacuous pass this assertion exists to refuse.",
        fraction * 100.0
    );
    // The ceiling is the half that matters. A floor alone is satisfied by a render that filled the
    // whole page with one colour, which is a real failure mode of a painter that lost its clip.
    assert!(
        fraction < 0.50,
        "{:.1}% of the page has ink on it. A worksheet of labels on an empty grid is mostly \
         background; a page this full is a painter that filled the target rather than the cells.",
        fraction * 100.0
    );

    let (left, top, right, bottom) = bounds.expect("ink implies a bounding box");

    // Where the *fragments* said the ink would be, computed from the tree rather than from the
    // image, so the two numbers are independent. The page's own root fragment is the band, so its
    // rectangle in EMU is the conversion factor between the two spaces.
    let page_width_emu = journey
        .tree
        .roots()
        .first()
        .and_then(|root| journey.tree.node(*root))
        .map_or(1.0, |root| root.rect().width().emu() as f64);
    let mut expected: Option<(f64, f64, f64, f64)> = None;
    for (_, node) in journey.tree.nodes() {
        if matches!(node.fragment(), Fragment::Box(_)) && node.parent().is_none() {
            continue; // The band's own root covers everything and would make this vacuous.
        }
        let rect = node.rect();
        let scale = f64::from(journey.pixels.width) / page_width_emu.max(1.0);
        let (l, t, r, b) = (
            rect.left.emu() as f64 * scale,
            rect.top.emu() as f64 * scale,
            rect.right.emu() as f64 * scale,
            rect.bottom.emu() as f64 * scale,
        );
        expected = Some(match expected {
            None => (l, t, r, b),
            Some((el, et, er, eb)) => (el.min(l), et.min(t), er.max(r), eb.max(b)),
        });
    }
    let (el, et, er, eb) = expected.expect("the tree holds something other than its root");

    // Three pixels of slack each way: antialiasing reaches one pixel past a filled edge, and a
    // glyph's own ink can overhang the advance box its fragment was measured to.
    const SLACK: f64 = 3.0;
    assert!(
        f64::from(left) >= el - SLACK
            && f64::from(top) >= et - SLACK
            && f64::from(right) <= er + SLACK
            && f64::from(bottom) <= eb + SLACK,
        "ink covers pixels ({left},{top})..({right},{bottom}) but the fragment tree only places \
         content in ({el:.0},{et:.0})..({er:.0},{eb:.0}). Ink outside the fragments' own boxes is a \
         painter drawing something the layout never asked for."
    );
}

#[test]
fn every_stand_in_is_counted_rather_than_mistaken_for_the_document() {
    for name in ["sheet_grid.xlsx", "style_resources.xlsx", "sample.xlsx"] {
        let mut book = fixture(name);
        let journey = journey(&mut book, 0, &viewport(8.0, 5.0));
        assert_eq!(
            journey.drawn.placeholders, journey.unregistered_outlines,
            "{name}: the painter drew {} stand-ins and the geometry provider expected to be unable \
             to answer {} handles. The two are computed independently — one from the provider \
             before the render, one from the painter during it — and a worksheet issues no outline \
             handle at all, so both are zero and a stand-in appearing means a `ShapeFragment` \
             arrived from somewhere.",
            journey.drawn.placeholders, journey.unregistered_outlines
        );
        assert_eq!(
            journey.drawn.placeholders, 0,
            "{name}: a worksheet has no shapes, so no draw can be a stand-in"
        );
    }
}

/// **The half of the hairline question only a painter can answer.**
///
/// `style="hair"` is drawn at half a point, which at the unzoomed scale is two thirds of a device
/// pixel. A band that rounded to zero — R15's defect, one EMU halved — or a painter that dropped
/// sub-pixel coverage would draw nothing, and no fragment-tier gate can tell the difference because
/// a fragment tree has no pixels in it.
///
/// The workbook is authored here rather than taken from the corpus because the corpus has no sheet
/// whose *only* mark is a hairline: `style_resources.xlsx` has a gradient and a number in the same
/// cell, so ink on that page proves nothing about the border.
#[test]
fn a_hairline_border_puts_ink_on_the_page() {
    const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<fonts count="1"><font><sz val="11"/><name val="Liberation Sans"/></font></fonts>
<fills count="1"><fill><patternFill patternType="none"/></fill></fills>
<borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left/><right/><top/><bottom style="hair"><color rgb="FF000000"/></bottom><diagonal/></border></borders>
<cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
<cellXfs count="2"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/><xf numFmtId="0" fontId="0" fillId="0" borderId="1" xfId="0" applyBorder="true"/></cellXfs>
</styleSheet>"#;
    // One cell, no text, one hairline bottom edge. Nothing else on the sheet can put ink anywhere.
    const SHEET: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<dimension ref="A1:A1"/>
<sheetData><row r="1"><c r="A1" s="1"/></row></sheetData>
</worksheet>"#;

    let bytes = Workbook::blank()
        .expect("a blank workbook")
        .save_unchecked()
        .expect("blank saves");
    let mut package = mjx_xlsx::Package::open(&bytes).expect("the blank package opens");
    package
        .replace_part_bytes(
            &PartName::new("/xl/worksheets/sheet1.xml").expect("a valid part name"),
            SHEET.as_bytes().to_vec(),
        )
        .expect("the worksheet is replaceable");
    package
        .replace_part_bytes(
            &PartName::new("/xl/styles.xml").expect("a valid part name"),
            STYLES.as_bytes().to_vec(),
        )
        .expect("the styles part is replaceable");
    let mut book = Workbook::from_package(package).expect("the authored package resolves");

    let journey = journey(&mut book, 0, &viewport(4.0, 2.0));
    let counts = kinds(&journey.tree);
    assert_eq!(counts.bands, 1, "one stated edge is one band");
    assert_eq!(
        counts.glyphs, 0,
        "the sheet holds no text, so every pixel of ink below is the border and nothing else"
    );

    let (count, bounds) = ink(&journey.pixels);
    assert!(
        count > 0,
        "**a hairline border drew nothing at all.** `style=\"hair\"` is half a point, which is two \
         thirds of a device pixel at 96 dpi — thin enough to vanish if a band rounds to zero (R15's \
         defect: one EMU, halved) or if the painter drops sub-pixel coverage. Excel is made of \
         these."
    );
    let (_, top, _, bottom) = bounds.expect("ink implies a bounding box");
    assert!(
        bottom - top <= 3,
        "the hairline's ink spans rows {top}..{bottom}, which is thicker than a hairline can be. \
         A band drawn far too wide is as wrong as one drawn not at all, and only a pixel can say."
    );
    assert!(
        journey.drawn.draw_calls >= 1,
        "the painter issued no draw call for the one band on the page"
    );
}
