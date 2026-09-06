//! Packing, eviction and the per-frame upload delta.
//!
//! The byte ceiling is *measured* in `glyph_atlas_allocation.rs`, which is a binary of its own with
//! a counting global allocator. This file asserts the structure around it: that glyphs are packed
//! without overlapping, that a second frame drawing the same words uploads nothing, that eviction
//! really happens and really spares the frame being drawn, and that the two routes out of the
//! rasteriser are both reachable through [`GlyphAtlas::prepare_run`].
//!
//! # The trap, named
//!
//! An eviction test that fills the atlas and finds the ceiling held is **green when eviction is
//! broken and nothing was ever inserted**, and green again when eviction throws everything away — a
//! cache holding nothing satisfies every byte bound perfectly. So every assertion about the ceiling
//! below is paired with one about the working set still being there, and with one about eviction
//! having actually run.

#[path = "support/mod.rs"]
mod support;

use std::collections::BTreeSet;

use mjx_text::{
    place_run, BitmapFormat, DeviceScale, FeatureSet, FontError, FontSize, GlyphAtlas, GlyphIndex,
    GlyphRasterKey, GlyphRasteriser, GlyphRender, GlyphRoute, Hinting, PreparedImage, ScaleBucket,
    Shaper, ShapingRequest, SubpixelPosition, TextScript, ATLAS_GUTTER_PIXELS,
    DESKTOP_GLYPH_ATLAS_BYTE_CEILING, MOBILE_GLYPH_ATLAS_BYTE_CEILING,
    OUTLINE_PIXELS_PER_EM_THRESHOLD,
};

use support::synthetic_font;

/// The alphabet the suite draws with: eight letters, eight different rectangles.
const ALPHABET: &str = "ABCDEFGH";

/// Everything a frame needs, built once per test.
struct Fixture {
    face: std::sync::Arc<mjx_text::FontFace>,
    identity: mjx_text::FaceId,
    rasteriser: GlyphRasteriser,
    shaper: Shaper,
    features: FeatureSet,
}

impl Fixture {
    fn new() -> Self {
        let face = synthetic_font::outlined_latin_face().face();
        let mut rasteriser = GlyphRasteriser::new();
        let identity = rasteriser
            .register(&face)
            .expect("the synthetic face registers");
        Self {
            face,
            identity,
            rasteriser,
            shaper: Shaper::new(),
            features: FeatureSet::default(),
        }
    }

    /// Place `text` at `points`, one point to the pixel, from `origin`.
    fn place(&mut self, text: &str, points: f64, origin: f32) -> mjx_text::RunPlacement {
        let size = FontSize::from_points(points);
        let request = ShapingRequest::new(text, TextScript::LATIN, size, &self.features);
        let run = self.shaper.shape(&self.face, &request).expect("shapes");
        place_run(
            &run,
            self.identity,
            DeviceScale::from_pixels_per_point(1.0),
            Hinting::GridFitted,
            (origin, 0.0),
        )
    }
}

#[test]
fn the_declared_ceilings_are_a_quarter_of_the_texture_budgets_the_plan_sets() {
    // `docs/UI_PLATFORM_PLAN.md` §12 budgets 256 MB of GPU texture on a desktop and 96 MB on a
    // phone. If either figure moves, this is where the atlas's share is reconciled with it.
    assert_eq!(DESKTOP_GLYPH_ATLAS_BYTE_CEILING * 4, 256 * 1024 * 1024);
    assert_eq!(MOBILE_GLYPH_ATLAS_BYTE_CEILING * 4, 96 * 1024 * 1024);
    assert_eq!(
        GlyphAtlas::new().byte_ceiling(),
        DESKTOP_GLYPH_ATLAS_BYTE_CEILING
    );
}

#[test]
fn a_second_frame_drawing_the_same_words_uploads_strictly_less_than_the_first() {
    let mut fixture = Fixture::new();
    let mut atlas = GlyphAtlas::new();
    let placement = fixture.place(ALPHABET, 16.0, 0.0);

    atlas.begin_frame();
    let first_run = atlas
        .prepare_run(&mut fixture.rasteriser, &placement)
        .expect("the first frame prepares");
    let first = atlas.take_delta();

    assert_eq!(first_run.len(), ALPHABET.chars().count());
    assert!(
        first.uploaded_bytes() > 0,
        "the first frame must upload the glyphs it just rasterised"
    );
    assert_eq!(first.uploads().len(), ALPHABET.chars().count());
    assert_eq!(first.pages_created().len(), 1);
    assert!(first.pages_dropped().is_empty());
    let uploaded: usize = first
        .uploads()
        .iter()
        .map(mjx_text::AtlasUpload::byte_len)
        .sum();
    assert_eq!(uploaded, first.uploaded_bytes());

    atlas.begin_frame();
    atlas
        .prepare_run(&mut fixture.rasteriser, &placement)
        .expect("the second frame prepares");
    let second = atlas.take_delta();

    assert!(
        second.uploaded_bytes() < first.uploaded_bytes(),
        "the second frame must upload strictly less than the first: {} against {}",
        second.uploaded_bytes(),
        first.uploaded_bytes()
    );
    assert!(
        second.is_empty(),
        "and in fact nothing at all, because every glyph was already resident"
    );

    let statistics = atlas.statistics();
    assert_eq!(statistics.entries, ALPHABET.chars().count());
    assert_eq!(statistics.misses, ALPHABET.chars().count() as u64);
    assert!(statistics.hits >= ALPHABET.chars().count() as u64);
    assert_eq!(statistics.uploaded_bytes, first.uploaded_bytes() as u64);
}

#[test]
fn glyphs_packed_into_one_page_do_not_overlap_and_keep_their_gutter() {
    let mut fixture = Fixture::new();
    let mut atlas = GlyphAtlas::new();
    let placement = fixture.place(ALPHABET, 24.0, 0.0);

    atlas.begin_frame();
    let prepared = atlas
        .prepare_run(&mut fixture.rasteriser, &placement)
        .expect("prepares");

    let mut rectangles = Vec::new();
    for glyph in prepared.glyphs() {
        let PreparedImage::Atlas(entry) = glyph.image else {
            panic!("every letter at this size belongs in the atlas");
        };
        assert_eq!(entry.format, BitmapFormat::Coverage);
        assert!(entry.width > 0 && entry.height > 0);
        assert_eq!(
            entry.byte_len(),
            usize::from(entry.width) * usize::from(entry.height)
        );
        rectangles.push(entry);
    }

    for (index, one) in rectangles.iter().enumerate() {
        for other in &rectangles[index + 1..] {
            if one.page != other.page {
                continue;
            }
            let separated_horizontally = one.x + one.width + ATLAS_GUTTER_PIXELS <= other.x
                || other.x + other.width + ATLAS_GUTTER_PIXELS <= one.x;
            let separated_vertically = one.y + one.height + ATLAS_GUTTER_PIXELS <= other.y
                || other.y + other.height + ATLAS_GUTTER_PIXELS <= one.y;
            assert!(
                separated_horizontally || separated_vertically,
                "two glyphs on one page must be separated by at least the gutter, or a painter \
                 sampling one with bilinear filtering reads a column of the other: {one:?} against \
                 {other:?}"
            );
        }
    }

    // A page really holds the bytes, which is what makes the byte ceiling a real quantity rather
    // than a bookkeeping one.
    let page = rectangles[0].page;
    let pixels = atlas.page_pixels(page).expect("the page exists");
    assert_eq!(pixels.len(), atlas.statistics().resident_bytes);
    assert!(
        pixels.iter().any(|value| *value > 0),
        "the page must actually have been written to"
    );
}

#[test]
fn both_routes_are_reachable_through_one_prepared_run() {
    let mut fixture = Fixture::new();
    let mut atlas = GlyphAtlas::new();

    atlas.begin_frame();
    let small = fixture.place(
        ALPHABET,
        f64::from(OUTLINE_PIXELS_PER_EM_THRESHOLD) / 6.0,
        0.0,
    );
    let prepared_small = atlas
        .prepare_run(&mut fixture.rasteriser, &small)
        .expect("prepares");
    assert_eq!(
        prepared_small.route_counts(),
        (ALPHABET.chars().count(), 0, 0),
        "every letter at a body size belongs in the atlas"
    );

    let large = fixture.place(
        ALPHABET,
        f64::from(OUTLINE_PIXELS_PER_EM_THRESHOLD) * 2.0,
        0.0,
    );
    let prepared_large = atlas
        .prepare_run(&mut fixture.rasteriser, &large)
        .expect("prepares");
    assert_eq!(
        prepared_large.route_counts(),
        (0, ALPHABET.chars().count(), 0),
        "and every letter above the threshold is a path instead, which is the whole of the second \
         route existing"
    );

    for glyph in prepared_large.glyphs() {
        assert_eq!(glyph.route(), GlyphRoute::Outline);
        let PreparedImage::Outline(outline) = &glyph.image else {
            panic!("the route said outline");
        };
        assert_eq!(outline.len(), 5, "one rectangular contour");
    }

    // A run of spaces takes neither route, and that is a third answer rather than a failure.
    let blank = fixture.place("   ", 16.0, 0.0);
    let prepared_blank = atlas
        .prepare_run(&mut fixture.rasteriser, &blank)
        .expect("prepares");
    assert_eq!(prepared_blank.route_counts(), (0, 0, 3));
}

#[test]
fn eviction_runs_holds_the_ceiling_and_spares_the_frame_being_drawn() {
    let mut fixture = Fixture::new();
    // Small pages and a ceiling of eight of them, so the working set outgrows the atlas within a
    // test rather than within a document — and so that no single frame needs the whole ceiling,
    // which would be a different failure from the one under test.
    const PAGE: u16 = 128;
    const PAGES: usize = 3;
    let ceiling = usize::from(PAGE) * usize::from(PAGE) * PAGES;
    let mut atlas = GlyphAtlas::with_configuration(PAGE, ceiling);

    // Twenty-four frames at a different size each, so every frame's glyphs are different keys and
    // the pages really do accumulate past the ceiling. No single frame needs the whole ceiling,
    // which would be a different failure from the one under test.
    for step in 0..24 {
        atlas.begin_frame();
        let points = 16.0 + f64::from(step);
        let placement = fixture.place(ALPHABET, points, 0.0);
        atlas
            .prepare_run(&mut fixture.rasteriser, &placement)
            .expect("a frame prepares");
        let _ = atlas.take_delta();
        assert!(
            atlas.resident_bytes() <= ceiling,
            "the ceiling must hold on every frame, not only at the end: {} at step {step}",
            atlas.resident_bytes()
        );
    }

    let statistics = atlas.statistics();
    assert!(
        statistics.pages_evicted > 0 && statistics.entries_evicted > 0,
        "eviction must actually have run, or the ceiling above held because nothing was ever \
         inserted: {statistics:?}"
    );
    assert!(
        statistics.entries > 0,
        "and it must not have thrown everything away, which is the other way a byte bound is \
         satisfied vacuously"
    );

    // The last frame. Its glyphs go in first; then enough new work to force eviction again. What
    // must survive is exactly the working set of the frame being drawn.
    atlas.begin_frame();
    let working_set = fixture.place(ALPHABET, 40.0, 0.0);
    atlas
        .prepare_run(&mut fixture.rasteriser, &working_set)
        .expect("the working set prepares");
    let held: Vec<GlyphRasterKey> = working_set
        .glyphs()
        .iter()
        .map(|placed| placed.key)
        .collect();

    let evicted_before = atlas.statistics().pages_evicted;
    // Twelve filler runs, not three, and the difference is the whole strength of the survival
    // assertion below. With three, an atlas that had *not* pinned the frame it was drawing happened
    // to evict around the working set and the assertion stayed green against a broken cache. Twelve
    // asks for several times the whole ceiling inside one frame; deleting the current-frame guard
    // from `GlyphAtlas::evict_one_page` then loses a working-set glyph and turns this red, which is
    // how it was checked.
    for step in 0..12 {
        let filler = fixture.place(ALPHABET, 41.0 + f64::from(step), 0.0);
        // The filler is expected to run the atlas out of pages part way through, which is the point:
        // once every page belongs to this frame there is nothing left to evict, and the refusal is
        // what stops the working set being sacrificed to make room for it.
        match atlas.prepare_run(&mut fixture.rasteriser, &filler) {
            Ok(_) | Err(FontError::GlyphAtlasExhausted { .. }) => {}
            Err(other) => panic!("an unexpected failure: {other}"),
        }
    }
    assert!(
        atlas.statistics().pages_evicted > evicted_before,
        "the filler must have forced more eviction, or the survival asserted below is not a \
         survival of anything"
    );

    for key in &held {
        assert!(
            atlas.get(key).is_some(),
            "a glyph the frame being drawn has already used must not be evicted out from under it: \
             {key:?}"
        );
    }
    assert!(atlas.resident_bytes() <= ceiling);
}

#[test]
fn an_atlas_that_cannot_evict_says_so_rather_than_dropping_the_frame_it_is_drawing() {
    let mut fixture = Fixture::new();
    const PAGE: u16 = 64;
    // One page, and a working set that needs more than one.
    let ceiling = usize::from(PAGE) * usize::from(PAGE);
    let mut atlas = GlyphAtlas::with_configuration(PAGE, ceiling);

    atlas.begin_frame();
    let mut refusals = 0_usize;
    for step in 0..40 {
        let placement = fixture.place(ALPHABET, 40.0 + f64::from(step), 0.0);
        match atlas.prepare_run(&mut fixture.rasteriser, &placement) {
            Ok(_) => {}
            Err(FontError::GlyphAtlasExhausted {
                resident_bytes,
                ceiling_bytes,
            }) => {
                assert!(resident_bytes <= ceiling_bytes);
                refusals += 1;
            }
            Err(other) => panic!("an unexpected failure: {other}"),
        }
    }

    assert!(
        refusals > 0,
        "an atlas whose every page belongs to the frame in hand must refuse rather than evict it, \
         because a cache that throws its working set away satisfies every byte bound and draws \
         nothing"
    );
    assert!(atlas.resident_bytes() <= ceiling);

    // And the next frame recovers, because the pages are no longer this frame's. The text is small
    // enough to fit one page, which is the whole ceiling here: a run that needed two would be
    // refused again, and for a different reason.
    atlas.begin_frame();
    let placement = fixture.place(ALPHABET, 10.0, 0.0);
    atlas
        .prepare_run(&mut fixture.rasteriser, &placement)
        .expect("a new frame may evict the last one's pages");
}

#[test]
fn a_glyph_too_large_for_a_page_is_refused_by_name() {
    let mut fixture = Fixture::new();
    // Pages of sixteen pixels, and a glyph far larger than one.
    let mut atlas = GlyphAtlas::with_configuration(16, 1024 * 1024);
    let key = GlyphRasterKey {
        face: fixture.identity,
        glyph: GlyphIndex(2),
        bucket: ScaleBucket::enclosing(64.0),
        subpixel: SubpixelPosition::ALIGNED,
        hinting: Hinting::GridFitted,
    };
    let GlyphRender::Bitmap(bitmap) = fixture.rasteriser.render(key).expect("rasterises") else {
        panic!("a 64 pixel glyph is a bitmap");
    };

    let refused = atlas.insert(key, &bitmap);
    assert!(
        matches!(refused, Err(FontError::GlyphTooLargeForAtlas { .. })),
        "a glyph that will not fit an empty page must be named as such rather than reported as an \
         exhausted atlas, because no amount of eviction would help: {refused:?}"
    );
    assert_eq!(atlas.statistics().entries, 0);
}

#[test]
fn a_dropped_page_is_reported_before_the_page_that_reuses_its_index() {
    let mut fixture = Fixture::new();
    const PAGE: u16 = 64;
    let ceiling = usize::from(PAGE) * usize::from(PAGE) * 2;
    let mut atlas = GlyphAtlas::with_configuration(PAGE, ceiling);

    let mut created: BTreeSet<u32> = BTreeSet::new();
    let mut dropped_then_reused = false;
    for step in 0..24 {
        atlas.begin_frame();
        let placement = fixture.place(ALPHABET, 30.0 + f64::from(step), 0.0);
        // Exhaustion is a legitimate answer here and is asserted on elsewhere; what this case is
        // about is the ordering of the delta when a page really is dropped.
        let _ = atlas.prepare_run(&mut fixture.rasteriser, &placement);
        let delta = atlas.take_delta();

        let mut dropped_this_frame: BTreeSet<u32> = BTreeSet::new();
        for page in delta.pages_dropped() {
            dropped_this_frame.insert(page.as_u32());
            created.remove(&page.as_u32());
        }
        for creation in delta.pages_created() {
            assert_eq!(creation.size, PAGE);
            if dropped_this_frame.contains(&creation.page.as_u32()) {
                dropped_then_reused = true;
            }
            assert!(
                created.insert(creation.page.as_u32()),
                "an index must never be handed out twice without being dropped in between, or a \
                 painter holds two textures under one name"
            );
        }
        for upload in delta.uploads() {
            assert!(
                created.contains(&upload.page.as_u32()),
                "an upload must never name a page that has been dropped"
            );
            assert_eq!(upload.byte_len(), upload.pixels().len());
        }
    }

    assert!(
        dropped_then_reused,
        "the run must actually have reused a dropped page's index, or the ordering asserted above \
         was never exercised"
    );
}
