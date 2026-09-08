//! **The first end-to-end deck**: a committed `.pptx` travelling the whole pipeline, in one process.
//!
//! ```text
//! Presentation → SlideDeck → SlideBoxModel → FragmentTree
//!              → SlideResources + SlideGeometry → build_scene → DisplayList
//!              → SoftwarePainter → pixels
//! ```
//!
//! Every stage of that chain has existed and been gated on its own since R06. **Nothing had ever
//! driven all of it at once with a real document**, and this suite is where that first happens. It
//! lives here because `mjx-reference-pack` is the only crate in the workspace that may name
//! `mjx-pptx` (3.0) and `mjx-paint` (5.5) together: 5.5 is above 3.0, so any crate holding both must
//! sit above the whole graph, and above the graph is where nothing in the document tier may go.
//!
//! # ⚠ This is not parity, and it is not a golden image
//!
//! It proves the **plumbing**, in the sense `tests/the_plumbing_is_proved_and_not_the_fidelity.rs`
//! already uses in this crate: that a deck's shapes, table cells and glyphs reach the display list,
//! that the display list reaches a rasteriser, and that ink lands where the fragments said it
//! would. Whether that ink looks like PowerPoint's is a question only a human sitting against real
//! Microsoft Office on Windows answers (`docs/validation/07-the-reference-pack.md`). LibreOffice is
//! a change detector, and the user has said its export of shades and gradients is not to be trusted
//! even as that.
//!
//! # The vacuity trap, and what is done about it
//!
//! *"A real deck renders"* is satisfied by a renderer that draws a blank page without erroring, and
//! by one that draws every shape as a stand-in rectangle. So every assertion below is on **content**:
//!
//! * how many fragments of each kind the tree holds, and that the counts match the document;
//! * that `DrawReport::placeholders` is what the geometry provider said it would be, checked
//!   against a number computed **before** the render rather than read out of it;
//! * that ink covers a stated fraction of the page, with a floor *and* a ceiling — a floor alone
//!   passes for a page painted entirely black;
//! * that the ink sits inside the rectangles the fragment tree named, so a render that filled the
//!   page with one colour fails even at the right coverage.

use mjx_layout::{BoxModel, Fragment, FragmentTree, PageIndex};
use mjx_layout_pptx::{constraints_for, SlideBoxModel, SlideDeck};
use mjx_paint::{
    render_offscreen, DrawReport, NoImages, Pixels, Resources, SoftwarePainter, SOFTWARE_PAINTER,
};
use mjx_pptx::Presentation;
use mjx_scene::{build_scene, DisplayList, SceneOptions};
use mjx_scene_pptx::{SlideGeometry, SlideResources};
use mjx_text::{FontResolver, GlyphAtlas};

/// The bundled faces, and nothing the platform happens to have installed.
///
/// A render gate that resolved faces through the operating system would draw one image on a
/// developer's machine and another on CI, and the difference would look like a rendering
/// regression. `mjx-text` commits four metric-compatible faces; those are the whole tier here.
fn resolver() -> FontResolver {
    let fonts = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../mjx-text/assets/fonts");
    FontResolver::builder()
        .with_bundled_font_directory(&fonts)
        .expect("the committed faces index")
        .build()
}

/// What one slide's trip through the pipeline produced.
struct Journey {
    tree: FragmentTree,
    list: DisplayList,
    drawn: DrawReport,
    pixels: Pixels,
    /// How many outline handles the provider could not answer from the document's own geometry.
    ///
    /// Computed from the catalogue **before** the render, so that the placeholder assertion
    /// compares two independent numbers rather than a number with itself.
    unregistered_outlines: usize,
    /// How many shapes the box model asked for an outline for.
    outline_requests: usize,
}

/// Runs one slide of `fixture` all the way to pixels.
fn journey(fixture: &str, slide: usize) -> Journey {
    let bytes = mjx_fixtures::fixture(fixture);
    let mut presentation = Presentation::open(&bytes).expect("a well-formed package");
    let deck = SlideDeck::read(&mut presentation).expect("the deck reads");
    let constraints = constraints_for(&deck);

    let mut model = SlideBoxModel::new(resolver());
    let page = model
        .layout_page(
            &deck,
            PageIndex::new(u32::try_from(slide).expect("a small deck")),
            &constraints,
            None,
        )
        .expect("the slide lays out");
    let (tree, _) = page.into_parts();

    // The geometry provider is fed the document's own `a:prstGeom`. This crate holds the package,
    // which is exactly the division `SlideGeometry::register_all` is shaped for: the catalogue says
    // *which shape at what size*, and the caller — which is the only half that can open a file —
    // says what that shape is.
    let mut geometry = SlideGeometry::new();
    geometry.register_all(model.catalogue(), |request| {
        let extents =
            mjx_dml::Size::from_emu(request.rect.width().emu(), request.rect.height().emu());
        let path: Vec<usize> = request.shape.iter().map(|&index| index as usize).collect();
        outline_of(
            &mut presentation,
            request.surface_index as usize,
            path,
            extents,
        )
    });

    let options = SceneOptions::new(constraints.page);
    let resources = SlideResources::new(model.catalogue().clone(), options.device_scale);
    let outline_requests = model.catalogue().geometry_count();
    let unregistered_outlines = geometry.unregistered();

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
    // rather than through a second atlas that would have to be kept in step.
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
        outline_requests,
    }
}

/// The `a:prstGeom` of one shape, as `mjx-geometry` wants it, or `None` when the shape has none.
///
/// Two readers, because they answer two different questions: `shape_preset` says *which* preset —
/// the `ST_ShapeType` token the path tables are indexed by — and `shape_adjustments` says what its
/// guides have been moved to. Only the **overridden** adjustments are carried across, for the reason
/// `ShapeOutline::from_preset_geometry` gives: the defaults are already in the generated table, and
/// copying them into every registry entry would put two sources of one number in the process.
///
/// A shape with a custom path, or with none of its own, answers `None` and reaches the provider's
/// stand-in policy — counted, never mistaken for the document's own geometry.
fn outline_of(
    presentation: &mut Presentation,
    surface: usize,
    path: Vec<usize>,
    extents: mjx_dml::Size,
) -> Option<mjx_geometry::ShapeOutline> {
    let surface = mjx_pptx::Surface::Slide(surface);
    let preset = presentation.shape_preset(surface, path.clone()).ok()??;
    let adjustments = presentation
        .shape_adjustments(surface, path, mjx_dml::GuideContext::from_size(extents))
        .unwrap_or_default();
    Some(mjx_geometry::ShapeOutline {
        preset,
        extents,
        adjustments: adjustments
            .into_iter()
            .filter(|adjustment| adjustment.is_overridden)
            .map(|adjustment| {
                mjx_geometry::AdjustmentOverride::new(adjustment.spec.wire_name, adjustment.value)
            })
            .collect(),
    })
}

/// How many fragments of each kind a tree holds.
fn kinds(tree: &FragmentTree) -> (usize, usize, usize, usize, usize, usize) {
    let mut counts = (0, 0, 0, 0, 0, 0);
    for (_, node) in tree.nodes() {
        match node.fragment() {
            Fragment::Box(_) => counts.0 += 1,
            Fragment::Line(_) => counts.1 += 1,
            Fragment::GlyphRun(_) => counts.2 += 1,
            Fragment::Image(_) => counts.3 += 1,
            Fragment::Shape(_) => counts.4 += 1,
            Fragment::Table(_) => counts.5 += 1,
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
            if alpha == 0 {
                continue;
            }
            // Premultiplied, so a fully transparent pixel is all zeroes and anything that painted
            // is not. See `Pixels::rgba`'s own note on the convention.
            if red == 0 && green == 0 && blue == 0 && alpha == 0 {
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
fn a_deck_of_text_reaches_pixels() {
    // `text_levels.pptx` rather than `sample.pptx`, and the reason is a real R14 finding: the
    // sample's one shape is a placeholder **no tier places** — its `p:spPr` is empty and its layout
    // has no matching slot — so the honest fragment tree for it is the page and nothing else. A
    // pipeline gate run against it would have proved that an empty page rasterises.
    let journey = journey("text_levels.pptx", 0);
    let (boxes, lines, glyphs, images, shapes, tables) = kinds(&journey.tree);

    assert!(
        shapes >= 1,
        "the fixture's slide holds autoshapes, and none became a shape fragment"
    );
    // A count rather than "at least one", because one line and one glyph run is what a renderer
    // that laid out the title and stopped would also report.
    assert!(
        lines >= 9,
        "the fixture states a title and a body of indented paragraphs, which lay out as nine lines; \
         {lines} means some of them were not laid out"
    );
    // More glyph runs than lines, because a bulleted paragraph draws its **marker** as a run of its
    // own beside its text. A renderer that laid the indents out and drew no bullets reports exactly
    // one run per line, which reads as success on any count that only had a floor.
    assert!(
        glyphs > lines,
        "{glyphs} glyph runs for {lines} lines. Every bulleted paragraph draws a marker run beside \
         its text, so one run per line means the bullets were laid out and never drawn."
    );
    assert_eq!(
        (images, tables),
        (0, 0),
        "`text_levels.pptx` holds no picture and no table; if it does now, the counts below are \
         about a different document"
    );
    assert!(boxes >= 1, "the page's own root is a box fragment");

    // One `DrawGlyphs` per glyph-run fragment, exactly. A `>=` would pass for a walk that stopped
    // at the title and a `>` for one that emitted spurious commands; the equality is what says the
    // display list is a faithful encoding of the tree.
    //
    // The *total* command count is deliberately not asserted to be larger: `build_scene` skips a
    // `PushTransform` for an identity map and a `PushClip` for a page nothing overflows, so a slide
    // of unrotated, unclipped text is fourteen commands and nothing else. Expecting pushes here was
    // this suite's own first wrong guess.
    let drawn_runs = journey
        .list
        .commands()
        .filter(|command| matches!(command, mjx_scene::Command::DrawGlyphs { .. }))
        .count();
    assert_eq!(
        drawn_runs, glyphs,
        "{drawn_runs} `DrawGlyphs` commands for {glyphs} glyph-run fragments. The display list is \
         supposed to be an encoding of the tree, not a summary of it."
    );

    assert!(
        journey.drawn.glyphs > 0,
        "the painter built {} glyph quads. A slide of text that draws no glyphs is the exact \
         failure a *'it rendered without erroring'* gate cannot see.",
        journey.drawn.glyphs
    );
    assert!(
        journey.drawn.draw_calls > 0,
        "the painter issued no draw calls at all"
    );
}

#[test]
fn a_deck_of_tables_reaches_pixels_with_its_cells_intact() {
    let journey = journey("tables.pptx", 0);
    let (_, lines, glyphs, _, _, tables) = kinds(&journey.tree);

    assert_eq!(
        tables, 1,
        "`tables.pptx` slide 0 frames exactly one `a:tbl`"
    );

    let cells = journey
        .tree
        .nodes()
        .filter(|(_, node)| {
            matches!(node.fragment(), Fragment::Box(box_fragment) if box_fragment.cell.is_some())
        })
        .count();
    assert_eq!(
        cells, 9,
        "the fixture's table is 3x3 with nothing merged, so nine cells render"
    );
    assert!(
        lines >= 9 && glyphs >= 9,
        "every cell of the fixture carries text: {lines} lines and {glyphs} glyph runs for nine \
         cells means some cells' text was never laid out"
    );
    assert!(
        journey.drawn.glyphs >= 9,
        "the painter built {} glyph quads for a table of nine filled cells",
        journey.drawn.glyphs
    );
}

#[test]
fn the_ink_lands_where_the_fragments_said_it_would() {
    let journey = journey("tables.pptx", 0);
    let (count, bounds) = ink(&journey.pixels);
    let total = journey.pixels.width as usize * journey.pixels.height as usize;
    assert!(total > 0, "the render has a size");

    let fraction = count as f64 / total as f64;
    assert!(
        fraction > 0.01,
        "only {:.4}% of the page has ink on it. A page of shapes and a table covers more than one \
         per cent, and a blank render is the vacuous pass this assertion exists to refuse.",
        fraction * 100.0
    );
    // The ceiling is the half that matters. A floor alone is satisfied by a render that filled the
    // whole page with one colour, which is a real failure mode of a painter that lost its clip.
    assert!(
        fraction < 0.90,
        "{:.1}% of the page has ink on it. A slide is mostly background; a page this full is a \
         painter that filled the target rather than the shapes.",
        fraction * 100.0
    );

    let (left, top, right, bottom) = bounds.expect("ink implies a bounding box");

    // Where the *fragments* said the ink would be, computed from the tree rather than from the
    // image, so the two numbers are independent. The page's own root fragment is the slide, so its
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
            continue; // The page's own root covers everything and would make this vacuous.
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

    // Two pixels of slack each way: a stroke is centred on its path, and antialiasing reaches one
    // pixel past a filled edge.
    const SLACK: f64 = 3.0;
    assert!(
        f64::from(left) >= el - SLACK
            && f64::from(top) >= et - SLACK
            && f64::from(right) <= er + SLACK
            && f64::from(bottom) <= eb + SLACK,
        "ink covers pixels ({left},{top})..({right},{bottom}) but the fragment tree only places \
         content in ({el:.0},{et:.0})..({er:.0},{eb:.0}). Ink outside the fragments' own boxes is \
         a painter drawing something the layout never asked for."
    );
}

#[test]
fn every_stand_in_is_counted_rather_than_mistaken_for_the_document() {
    // The brief for this child said real geometry was still a placeholder; the tree says otherwise
    // — `mjx-geometry`'s `PresetGeometryProvider` has answered from the document's own path tables
    // since MJXOFF-206. Either way, the number is *asserted* rather than assumed: this compares the
    // provider's own count of unanswerable handles, taken before the render, with the painter's
    // count of stand-ins, taken during it.
    for fixture in ["sample.pptx", "tables.pptx", "effects_theme.pptx"] {
        let journey = journey(fixture, 0);
        assert_eq!(
            journey.drawn.placeholders, journey.unregistered_outlines,
            "{fixture}: the painter drew {} stand-ins and the geometry provider expected to be \
             unable to answer {} handles out of {}. The two are computed independently — one from \
             the catalogue before the render, one by the painter during it — and a disagreement \
             means a shape drew from something other than what the provider was asked for.",
            journey.drawn.placeholders, journey.unregistered_outlines, journey.outline_requests
        );
    }
}

#[test]
fn a_deck_with_theme_effects_reaches_the_painters_layers() {
    // An effect that silently no-ops produces an image very close to one with the effect applied,
    // and a loose tolerance passes it. So the assertion is not on pixels: it is on the painter
    // having *opened a layer*, which is the one thing an effect cannot do without.
    let with = journey("effects_theme.pptx", 0);
    assert!(
        with.drawn.layers > 0,
        "the fixture's second shape takes theme effect style 3 — an outer shadow — and the painter \
         opened {} offscreen layers. An effect that reached the display list and opened no layer is \
         an effect that drew nothing, and it is invisible in a pixel comparison.",
        with.drawn.layers
    );
}
