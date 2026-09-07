//! A glyph large enough to leave the atlas is tessellated by the **same** code every shape is.
//!
//! MJX-STAND-IN: this crate is rank 1.7 and `mjx-geometry` is 2.5, so the real provider is an
//! upward edge `xtask/tests/layering.rs` refuses by name. A glyph outline is already a resolved
//! path, so the provider here only satisfies the signature and never answers a handle.
//!
//! # The hand-off this file holds
//!
//! MJXOFF-159 gave `mjx-text` two routes out of the rasteriser: a bitmap in the atlas, and — above
//! [`OUTLINE_PIXELS_PER_EM_THRESHOLD`](mjx_text::OUTLINE_PIXELS_PER_EM_THRESHOLD) — an outline.
//! MJXOFF-161 put the outline into the display list as an ordinary [`Geometry::Path`] in the same
//! geometry table a shape's outline lands in, glyph-local, addressed from the glyph record by
//! [`GlyphImage::Outline`].
//!
//! So there is **no second path pipeline for text**, and this file is what says so: the outline of a
//! hundred-and-twenty-point letter goes through `Tessellator::fill` exactly as a rectangle does, and
//! the only thing text-specific about it is where the tolerance comes from —
//! [`TessellationOptions::for_glyph_run`], which reads the bucket and the residual scale **the run's
//! own record already carries** rather than inventing a second bucketing scheme.
//!
//! # Why the box model here is the foreign one
//!
//! Because a scene builder validated only against a PowerPoint tree proves nothing about the seam.
//! The support module is `mjx-layout`'s own plain-text box model, included by path rather than
//! copied, for the reason `tests/fragments_alone_drive_the_builder.rs` gives.

#[path = "../../mjx-layout/tests/support/mod.rs"]
mod support;

use std::sync::Arc;

use mjx_layout::{BoxModel, Constraints};
use mjx_layout::{DecorationRef, ImageRef, LayoutSize, PageIndex, SourceRef};
use mjx_ooxml_core::measure::Emu;
use mjx_scene::{
    build_scene, Decoration, DisplayList, Geometry, GlyphImage, Image, PlaceholderGeometry,
    ResourceIndex, ResourceResolver, SceneGlyphRun, SceneOptions, SectionKind, TessellationOptions,
    Tessellator, TOLERANCE_DEVICE_PIXELS,
};
use mjx_text::{FontSize, GlyphAtlas, GlyphRasteriser, ScaleBucket};

use support::plain_text::{PlainTextColumn, PlainTextDocument};

/// A resolver that answers nothing, because the plain-text box model issues no handles.
struct PlainText;

impl ResourceResolver for PlainText {
    fn decoration(&self, _reference: DecorationRef) -> Option<Decoration> {
        None
    }

    fn text_decoration(&self, _source: &SourceRef) -> Option<Decoration> {
        None
    }

    fn image(&self, _reference: ImageRef) -> Option<Image> {
        None
    }
}

/// A page of very large text: at ninety-six pixels to the inch a hundred-and-twenty-point em is a
/// hundred and sixty pixels, well past the threshold above which a glyph becomes an outline.
fn a_page_of_headline() -> DisplayList {
    let page = LayoutSize::new(Emu::from_points(900.0), Emu::from_points(400.0));
    let mut rasteriser = GlyphRasteriser::new();
    let mut atlas = GlyphAtlas::new();
    let face = support::liberation_sans();
    let mut model = PlainTextColumn::new(
        &mut rasteriser,
        Arc::clone(&face),
        FontSize::from_points(120.0),
    );
    let document = PlainTextDocument::from_paragraphs(["Ogham"]);
    let constraints = Constraints::single_column(page, Emu::from_points(140.0));
    let laid_out = model
        .layout_page(&document, PageIndex::FIRST, &constraints, None)
        .expect("the plain-text box model lays out a headline");
    build_scene(
        laid_out.fragments(),
        &PlainText,
        &mut rasteriser,
        &mut atlas,
        &SceneOptions::new(page),
    )
    .expect("a headline becomes a scene")
}

#[test]
fn a_large_glyph_is_a_path_in_the_geometry_table_and_tessellates_like_any_other() {
    let list = a_page_of_headline();

    // The run is in the list, and it carries the bucket and the residual its glyphs were scaled
    // for. Both are MJXOFF-161's, read here rather than recomputed.
    assert!(
        list.record_count(SectionKind::GlyphRuns) > 0,
        "the headline produced no glyph runs at all"
    );
    let run: SceneGlyphRun = list
        .glyph_run(ResourceIndex::new(0))
        .expect("the first glyph run decodes");
    assert!(
        run.bucket_steps > 0,
        "the run carries no scale bucket, so `for_glyph_run` has nothing to read"
    );
    assert!(
        run.residual_scale > 0.0 && run.residual_scale <= 1.0 + f32::EPSILON,
        "a residual scale of {} is not one bucket step's worth",
        run.residual_scale
    );

    // Above the threshold, the glyphs took the outline route — into the *geometry* table, which is
    // the table a shape's outline goes into. That is the hand-off: one pipeline, not two.
    let outlines: Vec<ResourceIndex> = run
        .glyphs
        .iter()
        .filter_map(|glyph| match glyph.image {
            GlyphImage::Outline(index) => Some(index),
            GlyphImage::Atlas(_) | GlyphImage::Blank => None,
        })
        .collect();
    assert!(
        !outlines.is_empty(),
        "a hundred-and-twenty-point headline produced no outlines; the large-text route was not \
         taken and this test is asserting nothing"
    );

    let options = TessellationOptions::for_glyph_run(&run);
    assert_eq!(options.bucket(), ScaleBucket::from_steps(run.bucket_steps));
    assert!(
        options.tolerance() >= TOLERANCE_DEVICE_PIXELS,
        "a run drawn `residual_scale` times larger needs a tolerance that much finer, and {} is \
         not finer than {TOLERANCE_DEVICE_PIXELS}",
        options.tolerance()
    );

    let mut tessellator = Tessellator::new();
    let mut with_triangles = 0_usize;
    for index in &outlines {
        let geometry = list
            .geometry(*index)
            .expect("a glyph's outline decodes out of the geometry table");
        assert!(
            matches!(geometry, Geometry::Path { .. }),
            "a glyph outline is an ordinary path, not a kind of its own"
        );
        // The provider is never reached: a glyph outline is already resolved.
        let mesh = tessellator
            .fill(&geometry, &PlaceholderGeometry::new(), options)
            .expect("a glyph outline tessellates through the ordinary fill");
        if !mesh.is_empty() {
            with_triangles += 1;
            // Glyph-local, so the outline sits about the origin and the baseline, never at the
            // run's position on the page — the property `mjx-text`'s two routes exist to share.
            let bounds = mesh.bounds();
            assert!(
                bounds.width() > 1.0 && bounds.height() > 1.0,
                "a hundred-and-sixty-pixel letter covers {} by {}",
                bounds.width(),
                bounds.height()
            );
            assert!(
                bounds.left.abs() < 400.0 && bounds.top.abs() < 400.0,
                "the outline is at {bounds:?}, which is not glyph-local"
            );
        }
    }
    assert!(
        with_triangles >= 3,
        "only {with_triangles} of {} outlines became triangles",
        outlines.len()
    );

    // The same letter twice on the page is one geometry and one cache entry — which is what makes
    // interning outlines glyph-local worth doing.
    assert!(
        tessellator.cache().hits() + tessellator.cache().misses() >= outlines.len() as u64,
        "the cache was not consulted once per outline"
    );
}

#[test]
fn a_glyph_run_that_is_not_magnified_asks_for_the_ordinary_tolerance() {
    // A run whose residual scale is exactly one is drawn at the size it was rasterised for, so a
    // quarter of a device pixel at the destination is a quarter of a unit at the source.
    let list = a_page_of_headline();
    let mut run = list
        .glyph_run(ResourceIndex::new(0))
        .expect("the first glyph run decodes");
    run.residual_scale = 1.0;
    assert_eq!(
        TessellationOptions::for_glyph_run(&run).tolerance(),
        TOLERANCE_DEVICE_PIXELS
    );

    // And a run whose record says something impossible still asks for a tolerance a flattener can
    // act on, because a display list may have come off a disk.
    run.residual_scale = 0.0;
    assert_eq!(
        TessellationOptions::for_glyph_run(&run).tolerance(),
        TOLERANCE_DEVICE_PIXELS
    );
    run.residual_scale = f32::NAN;
    assert_eq!(
        TessellationOptions::for_glyph_run(&run).tolerance(),
        TOLERANCE_DEVICE_PIXELS
    );
}
