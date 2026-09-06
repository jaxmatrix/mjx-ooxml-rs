//! What a glyph looks like at a size, and which of the two routes out of the rasteriser it took.
//!
//! # Why these faces are built rather than committed
//!
//! `assets/fonts/` holds five metric-compatible Latin faces and nothing else. None of them carries a
//! `COLR`/`CPAL` pair, so the colour route has no committed input; and a rasterisation assertion
//! against a real letterform is an assertion about a vendor's outline rather than about the scan
//! converter. The faces here are boxes of *chosen* size, so what comes out of the rasteriser is
//! predictable from arithmetic and an assertion about it is an assertion about this crate.
//!
//! The alternative was committing an emoji font, which MJXOFF-157 already escalated as a repository
//! owner's decision rather than an agent's.
//!
//! # The trap this file is written against
//!
//! Every assertion below is two-sided wherever a one-sided one would pass vacuously. "It returned
//! RGBA" is satisfied by an image that is entirely one colour, which is what a colour path that
//! silently fell back to the outline route would produce — so the colour case counts *distinct*
//! colours and names both of them. "Nothing was rasterised again" is satisfied by a placer that
//! ignored the size it was given — so the bucketing case also asserts that the two requests really
//! were different sizes, by comparing the residual scales they produced.

#[path = "support/mod.rs"]
mod support;

use std::collections::BTreeSet;

use mjx_text::{
    place_run, shape_uncached, DeviceScale, FeatureSet, FontError, FontSize, GlyphAtlas,
    GlyphIndex, GlyphRasterKey, GlyphRasteriser, GlyphRender, GlyphRoute, Hinting, OutlineCommand,
    ScaleBucket, Shaper, ShapingRequest, SubpixelPosition, TextScript, MAXIMUM_PIXELS_PER_EM,
    MAXIMUM_RASTERISED_PIXELS_PER_EM, OUTLINE_PIXELS_PER_EM_THRESHOLD,
    SCALE_BUCKET_STEP_PIXELS_PER_EM, SUBPIXEL_POSITION_COUNT,
};

use support::synthetic_font;

/// The first letter of [`synthetic_font::outlined_latin_face`], which is a 380 by 480 unit box on a
/// 1000-unit em.
const LETTER_A: GlyphIndex = GlyphIndex(2);

/// A key for `glyph` in `face` at `pixels_per_em`, aligned to the pixel grid and hinted.
fn key_at(face: mjx_text::FaceId, glyph: GlyphIndex, pixels_per_em: f32) -> GlyphRasterKey {
    GlyphRasterKey {
        face,
        glyph,
        bucket: ScaleBucket::enclosing(pixels_per_em),
        subpixel: SubpixelPosition::ALIGNED,
        hinting: Hinting::GridFitted,
    }
}

#[test]
fn a_scale_bucket_is_the_smallest_step_at_or_above_the_size_that_was_asked_for() {
    let step = SCALE_BUCKET_STEP_PIXELS_PER_EM;

    // Exactly on a boundary stays on it; anything above it moves to the next one, never below.
    assert_eq!(ScaleBucket::enclosing(16.0).pixels_per_em(), 16.0);
    assert_eq!(
        ScaleBucket::enclosing(16.0 + step / 2.0).pixels_per_em(),
        16.0 + step
    );
    assert!(
        ScaleBucket::enclosing(15.9).pixels_per_em() >= 15.9,
        "a bucket must never be smaller than the size asked for, or every glyph is drawn from a \
         bitmap that is too soft"
    );

    // Two sizes a fifth of a step apart are one bucket, which is the whole point of the mechanism.
    assert_eq!(
        ScaleBucket::enclosing(15.8),
        ScaleBucket::enclosing(15.9),
        "two sizes within one step must share a bucket, or a zoom re-rasterises every frame"
    );

    let bucket = ScaleBucket::enclosing(15.8);
    let residual = bucket.residual_scale(15.8);
    assert!(
        (residual - 15.8 / bucket.pixels_per_em()).abs() < 1e-6,
        "the residual scale is what takes the bucket back to the requested size"
    );
    assert!(
        residual <= 1.0 && residual > 1.0 - 0.1,
        "the residual is always a small reduction, never an enlargement: it was {residual}"
    );
}

#[test]
fn a_hostile_size_produces_a_bucket_rather_than_an_infinity() {
    for hostile in [f32::NAN, f32::NEG_INFINITY, -1.0, 0.0] {
        assert!(
            ScaleBucket::enclosing(hostile).is_empty(),
            "{hostile} must quantise to the empty bucket"
        );
        assert_eq!(ScaleBucket::enclosing(hostile).residual_scale(hostile), 1.0);
    }
    assert_eq!(
        ScaleBucket::enclosing(f32::INFINITY).pixels_per_em(),
        MAXIMUM_PIXELS_PER_EM,
        "an infinite size clamps to the largest bucket rather than poisoning every position \
         derived from it"
    );
    assert_eq!(
        ScaleBucket::enclosing(1.0e30).pixels_per_em(),
        MAXIMUM_PIXELS_PER_EM
    );
}

#[test]
fn a_subpixel_phase_is_quantised_and_carries_into_the_next_pixel() {
    let count = f32::from(SUBPIXEL_POSITION_COUNT);

    assert_eq!(SubpixelPosition::split(0.0), (0, SubpixelPosition::ALIGNED));
    assert_eq!(
        SubpixelPosition::split(3.5),
        (3, SubpixelPosition::from_index(SUBPIXEL_POSITION_COUNT / 2))
    );

    // The carry is the reason the pixel and the phase come back together. Rounding 3.99 to the
    // nearest quarter is 4.0, which is the *next* pixel at phase zero, not pixel three at a phase
    // that does not exist.
    assert_eq!(
        SubpixelPosition::split(3.99),
        (4, SubpixelPosition::ALIGNED),
        "rounding up past the last phase must carry into the next whole pixel"
    );

    // Negative positions are as ordinary as positive ones: a run may begin left of its origin.
    let (whole, phase) = SubpixelPosition::split(-0.75);
    assert_eq!(whole, -1);
    assert!((phase.offset_in_pixels() - 0.25).abs() < 1e-6);

    for hostile in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert_eq!(
            SubpixelPosition::split(hostile),
            (0, SubpixelPosition::ALIGNED)
        );
    }

    let mut seen = BTreeSet::new();
    for step in 0..SUBPIXEL_POSITION_COUNT {
        let phase = SubpixelPosition::from_index(step);
        seen.insert(phase.index());
        assert!((phase.offset_in_pixels() - f32::from(step) / count).abs() < 1e-6);
    }
    assert_eq!(seen.len(), usize::from(SUBPIXEL_POSITION_COUNT));
}

#[test]
fn a_small_glyph_is_rasterised_and_a_large_one_becomes_an_outline() {
    let face = synthetic_font::outlined_latin_face().face();
    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser
        .register(&face)
        .expect("the synthetic face registers");

    let small = OUTLINE_PIXELS_PER_EM_THRESHOLD / 6.0;
    let large = OUTLINE_PIXELS_PER_EM_THRESHOLD * 2.0;

    let rasterised = rasteriser
        .render(key_at(identity, LETTER_A, small))
        .expect("a small glyph rasterises");
    assert_eq!(
        rasterised.route(),
        GlyphRoute::Bitmap,
        "below the threshold a glyph must be a bitmap, not a path"
    );
    let GlyphRender::Bitmap(bitmap) = &rasterised else {
        panic!("the route said bitmap, so this is one");
    };
    assert_eq!(bitmap.format(), mjx_text::BitmapFormat::Coverage);
    // 380 by 480 units on a 1000-unit em, at 16 pixels to the em, is 6.08 by 7.68 pixels — so a
    // bitmap tight around it is seven by eight, and it sits entirely above the baseline.
    assert_eq!((bitmap.width(), bitmap.height()), (7, 8));
    assert_eq!(bitmap.offset_from_origin_x(), 0);
    assert!(
        bitmap.offset_from_origin_y() < 0,
        "a letter is drawn above its baseline, so its top edge is above the origin"
    );
    assert_eq!(bitmap.byte_len(), 7 * 8);
    assert!(
        bitmap.pixels().iter().any(|coverage| *coverage > 200),
        "a filled box must have pixels the glyph almost entirely covers"
    );

    let outlined = rasteriser
        .render(key_at(identity, LETTER_A, large))
        .expect("a large glyph produces a path");
    assert_eq!(
        outlined.route(),
        GlyphRoute::Outline,
        "above the threshold a glyph must be a path, so that R07 can tessellate it once instead of \
         re-rasterising it at every bucket a zoom passes through"
    );
    let GlyphRender::Outline(outline) = &outlined else {
        panic!("the route said outline, so this is one");
    };
    // One rectangular contour: a move, three lines and a close.
    assert_eq!(outline.len(), 5);
    assert!(matches!(
        outline.commands().first(),
        Some(OutlineCommand::MoveTo(_))
    ));
    assert!(matches!(
        outline.commands().last(),
        Some(OutlineCommand::Close)
    ));
    // The box is 480 units tall, and this surface's y grows downward, so every point of it is at or
    // above the baseline — which is to say at a non-positive y.
    let lowest = outline
        .commands()
        .iter()
        .filter_map(|command| match command {
            OutlineCommand::MoveTo(point) | OutlineCommand::LineTo(point) => Some(point.y),
            _ => None,
        })
        .fold(f32::NEG_INFINITY, f32::max);
    assert!(
        lowest <= 0.0,
        "a box sitting on the baseline must not have points below it, so y is not upside down"
    );

    let statistics = rasteriser.statistics();
    assert_eq!(statistics.bitmaps, 1);
    assert_eq!(statistics.outlines, 1);
    assert_eq!(statistics.glyphs_rendered(), 2);
}

#[test]
fn an_outline_asked_for_twice_is_scaled_once() {
    let face = synthetic_font::outlined_latin_face().face();
    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser
        .register(&face)
        .expect("the synthetic face registers");
    let key = key_at(identity, LETTER_A, OUTLINE_PIXELS_PER_EM_THRESHOLD * 2.0);

    let first = rasteriser.render(key).expect("scales");
    let second = rasteriser.render(key).expect("scales");
    assert_eq!(first, second);
    assert_eq!(rasteriser.statistics().outlines, 1);
    assert_eq!(rasteriser.statistics().outline_cache_hits, 1);

    rasteriser.clear_outline_cache();
    let third = rasteriser.render(key).expect("scales");
    assert_eq!(first, third);
    assert_eq!(
        rasteriser.statistics().outlines,
        2,
        "clearing the cache must really clear it, or the count above proves nothing"
    );
}

#[test]
fn a_colour_glyph_rasterises_to_rgba_carrying_both_of_its_layers_colours() {
    let face = synthetic_font::colour_glyph_face().face();
    assert!(
        face.colour_formats().layered_outlines,
        "the face must declare `COLR`, because that is the answer the rasteriser switches on rather \
         than re-probing the table for itself"
    );

    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser
        .register(&face)
        .expect("the colour face registers");
    let rendered = rasteriser
        .render(key_at(identity, GlyphIndex(2), 32.0))
        .expect("a colour glyph rasterises");

    assert_eq!(rendered.route(), GlyphRoute::Bitmap);
    let GlyphRender::Bitmap(bitmap) = &rendered else {
        panic!("the route said bitmap, so this is one");
    };
    assert_eq!(
        bitmap.format(),
        mjx_text::BitmapFormat::Rgba,
        "a colour glyph carries its own colours, so coverage cannot express it"
    );
    assert_eq!(
        bitmap.byte_len(),
        usize::from(bitmap.width()) * usize::from(bitmap.height()) * 4
    );

    // Counting *distinct* colours is what makes this a test of the colour route. An image that is
    // entirely one colour is what a fall-back to the plain outline route would produce, and it would
    // satisfy every assertion above this one.
    let mut reds = 0_usize;
    let mut blues = 0_usize;
    for pixel in bitmap.pixels().as_chunks::<4>().0 {
        if pixel[3] == 0 {
            continue;
        }
        if pixel[0] > 200 && pixel[2] < 50 {
            reds += 1;
        }
        if pixel[2] > 200 && pixel[0] < 50 {
            blues += 1;
        }
    }
    assert!(
        reds > 0 && blues > 0,
        "both `CPAL` entries must appear: the face's first layer is red and its second is blue, and \
         an image with only one of them means one layer was dropped or the palette was misread \
         (red pixels {reds}, blue pixels {blues})"
    );
}

#[test]
fn a_colour_face_rasterises_at_every_size_and_refuses_the_ones_that_would_not_fit_memory() {
    let face = synthetic_font::colour_glyph_face().face();
    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser
        .register(&face)
        .expect("the colour face registers");

    // Well above the outline threshold, and still a bitmap: a colour glyph has no single outline to
    // hand a tessellator, so the threshold does not apply to it.
    let big = rasteriser
        .render(key_at(
            identity,
            GlyphIndex(2),
            OUTLINE_PIXELS_PER_EM_THRESHOLD * 3.0,
        ))
        .expect("a large colour glyph still rasterises");
    assert_eq!(big.route(), GlyphRoute::Bitmap);

    let refused = rasteriser.render(key_at(
        identity,
        GlyphIndex(2),
        MAXIMUM_RASTERISED_PIXELS_PER_EM * 2.0,
    ));
    assert!(
        matches!(refused, Err(FontError::GlyphTooLargeToRasterise { .. })),
        "a colour glyph past the rasterisation ceiling must be refused before anything allocates \
         for it, not rasterised into a gigabyte: {refused:?}"
    );
}

#[test]
fn a_face_is_registered_once_however_many_times_it_is_offered() {
    let face = synthetic_font::outlined_latin_face().face();
    let other = synthetic_font::outlined_latin_face().face();
    let mut rasteriser = GlyphRasteriser::new();

    let first = rasteriser.register(&face).expect("registers");
    let again = rasteriser.register(&face).expect("registers");
    let second = rasteriser.register(&other).expect("registers");

    assert_eq!(first, again, "the same face is one face");
    assert_ne!(
        first, second,
        "two faces built from equal bytes are two faces: identity, not content, is the rule the \
         shaped-run cache already keys on"
    );
    assert_eq!(rasteriser.statistics().faces, 2);
    assert!(rasteriser.face(first).is_some());
}

#[test]
fn a_key_naming_a_face_this_rasteriser_has_never_seen_is_refused() {
    let face = synthetic_font::outlined_latin_face().face();
    let mut known = GlyphRasteriser::new();
    let mut stranger = GlyphRasteriser::new();
    let identity = stranger.register(&face).expect("registers");

    let refused = known.render(key_at(identity, LETTER_A, 16.0));
    assert!(
        matches!(refused, Err(FontError::UnregisteredFace { .. })),
        "a face identity means nothing outside the rasteriser that issued it, and drawing whichever \
         face happened to be at that number would be worse than refusing: {refused:?}"
    );
}

#[test]
fn an_empty_bucket_and_a_glyph_the_face_does_not_hold_are_blank_rather_than_errors() {
    let face = synthetic_font::outlined_latin_face().face();
    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser.register(&face).expect("registers");

    assert_eq!(
        rasteriser
            .render(key_at(identity, LETTER_A, 0.0))
            .expect("a zero size is not an error")
            .route(),
        GlyphRoute::Blank
    );
    // Glyph 1 is the space: it has an advance and no outline, which is the ordinary case rather
    // than a malformed one.
    assert_eq!(
        rasteriser
            .render(key_at(identity, GlyphIndex(1), 16.0))
            .expect("a space is not an error")
            .route(),
        GlyphRoute::Blank
    );
    assert_eq!(
        rasteriser
            .render(key_at(identity, GlyphIndex(60_000), 16.0))
            .expect("a glyph past the end of the face is not an error")
            .route(),
        GlyphRoute::Blank
    );
}

#[test]
fn a_run_is_placed_at_its_bucket_s_scale_with_one_residual_left_for_the_painter() {
    let face = synthetic_font::outlined_latin_face().face();
    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser.register(&face).expect("registers");
    let features = FeatureSet::default();
    let mut shaper = Shaper::new();

    let request = ShapingRequest::new(
        "AB",
        TextScript::LATIN,
        FontSize::from_points(15.8),
        &features,
    );
    let run = shaper.shape(&face, &request).expect("shapes");
    let placement = place_run(
        &run,
        identity,
        DeviceScale::from_pixels_per_point(1.0),
        Hinting::GridFitted,
        (0.0, 0.0),
    );

    assert_eq!(placement.len(), 2);
    assert_eq!(placement.glyphs()[0].x, 0);
    assert_eq!(placement.glyphs()[0].y, 0);
    assert_eq!(placement.glyphs()[0].glyph(), LETTER_A);

    // The advance in bucket pixels, scaled by the residual, is the run's real width — which is what
    // makes bucketing cost nothing geometrically.
    let expected = run.advance().at_size(15.8);
    assert!(
        (f64::from(placement.advance_at_requested_size()) - expected).abs() < 0.05,
        "the run must be exactly as wide as its advance says: {} against {expected}",
        placement.advance_at_requested_size()
    );
    assert_eq!(placement.requested_pixels_per_em(), 15.8);
    assert_eq!(placement.bucket(), ScaleBucket::enclosing(15.8));
    assert!(placement.residual_scale() < 1.0);
    for placed in placement.glyphs() {
        assert_eq!(placed.key.bucket, placement.bucket());
        assert_eq!(placed.key.hinting, Hinting::GridFitted);
    }
    assert!(rasteriser.face(identity).is_some());
}

#[test]
fn a_mark_is_lifted_off_the_baseline_and_pulled_back_over_its_base() {
    let face = synthetic_font::combining_mark_face().face();
    let features = FeatureSet::default();
    // `a` followed by a combining acute: two characters, one cluster, and the second glyph is
    // positioned by the shaper rather than by the pen.
    let request = ShapingRequest::new(
        "a\u{0301}",
        TextScript::LATIN,
        FontSize::from_points(32.0),
        &features,
    );

    let glyphs = shape_uncached(&face, &request).expect("shapes");
    assert_eq!(glyphs.len(), 2);
    let base = glyphs[0];
    let mark = glyphs[1];
    assert_eq!(base.y_offset, 0, "the base sits on the baseline");
    assert!(
        mark.y_offset > 0,
        "the shaper must lift the mark off the baseline, or it is drawn through the letter it \
         belongs over; it said {}",
        mark.y_offset
    );
    assert!(
        mark.x_offset < 0,
        "and pull it back over the letter, since a combining mark advances nothing of its own; it \
         said {}",
        mark.x_offset
    );
    assert_eq!(mark.x_advance, 0);

    // The offsets have to survive placement, which is the half a rasteriser can get wrong on its
    // own: a placer that dropped them would put every mark on the baseline at the pen.
    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser.register(&face).expect("registers");
    let mut shaper = Shaper::new();
    let run = shaper.shape(&face, &request).expect("shapes");
    let placement = place_run(
        &run,
        identity,
        DeviceScale::from_pixels_per_point(1.0),
        Hinting::GridFitted,
        (0.0, 0.0),
    );

    let placed_base = placement.glyphs()[0];
    let placed_mark = placement.glyphs()[1];
    assert_eq!(placed_base.y, 0);
    assert!(
        placed_mark.y <= -10,
        "the mark must be placed well above the baseline: it was at y {} against the base's {}",
        placed_mark.y,
        placed_base.y
    );

    // Where the pen would have put it, had the horizontal offset been dropped.
    let scale = placement.bucket().pixels_per_em() / f32::from(run.units_per_em());
    let pen_after_the_base = f32::from(base.x_advance as i16) * scale;
    assert!(
        placed_mark.exact_x_in_pixels() < pen_after_the_base - 1.0,
        "the mark must be pulled back over the letter, not left at the pen: it was at {} with the \
         pen at {pen_after_the_base}",
        placed_mark.exact_x_in_pixels()
    );
    assert_eq!(placed_mark.cluster, placed_base.cluster);
}

#[test]
fn two_sizes_in_one_bucket_place_identically_and_rasterise_nothing_twice() {
    let face = synthetic_font::outlined_latin_face().face();
    let mut rasteriser = GlyphRasteriser::new();
    let identity = rasteriser.register(&face).expect("registers");
    let mut atlas = GlyphAtlas::new();
    let mut shaper = Shaper::new();
    let features = FeatureSet::default();
    let scale = DeviceScale::from_pixels_per_point(1.0);

    // 15.8 and 15.9 pixels to the em are 63.2 and 63.6 steps of a quarter of a pixel, so both round
    // up to step 64 — the same bucket, a tenth of a pixel apart.
    let mut place = |points: f64| {
        let size = FontSize::from_points(points);
        let request = ShapingRequest::new("ABCDEFGH", TextScript::LATIN, size, &features);
        let run = shaper.shape(&face, &request).expect("shapes");
        place_run(&run, identity, scale, Hinting::GridFitted, (0.0, 0.0))
    };

    let first = place(15.8);
    let second = place(15.9);

    // The guard against a vacuous pass. If `place_run` ignored the size it was handed, everything
    // below would hold and mean nothing.
    assert_ne!(
        first.residual_scale(),
        second.residual_scale(),
        "the two requests must really be different sizes, or the reuse proved below is the reuse of \
         a placer that ignored its input"
    );
    assert_eq!(first.bucket(), second.bucket());
    assert_eq!(
        first.glyphs(),
        second.glyphs(),
        "two sizes in one bucket must place identically, down to the subpixel phase — that is what \
         makes the images reusable, and positions at the requested size rather than the bucket's \
         would miss on every glyph after the first"
    );

    atlas.begin_frame();
    atlas
        .prepare_run(&mut rasteriser, &first)
        .expect("the first frame prepares");
    let after_the_first = rasteriser.statistics().glyphs_rendered();
    assert!(
        after_the_first >= 8,
        "eight distinct letters must have produced eight images, or the reuse below is trivial"
    );

    atlas.begin_frame();
    atlas
        .prepare_run(&mut rasteriser, &second)
        .expect("the second frame prepares");
    assert_eq!(
        rasteriser.statistics().glyphs_rendered(),
        after_the_first,
        "a size a tenth of a pixel away must reuse every image, or a pinch zoom re-rasterises the \
         alphabet on every frame"
    );
}

#[test]
fn a_corrupted_face_rasterises_to_something_or_to_an_error_and_never_panics() {
    // The source grep in `untrusted_faces.rs` proves the absence of a *token*. Rasterisation does
    // real arithmetic over glyph tables — offsets into `loca`, point counts out of `glyf`, palette
    // indices out of `CPAL` — and none of that is visible to a grep. This is the instrument for it.
    let original = synthetic_font::colour_glyph_face().build();

    let mut rendered = 0_usize;
    let mut unreadable = 0_usize;
    for at in 0..original.len() {
        for replacement in [0x00_u8, 0xFF, 0x7F] {
            let mut damaged = original.clone();
            damaged[at] = replacement;

            let Ok(face) = mjx_text::FontFace::parse(std::sync::Arc::from(damaged.as_slice()), 0)
            else {
                continue;
            };
            let face = std::sync::Arc::new(face);
            let mut rasteriser = GlyphRasteriser::new();
            let Ok(identity) = rasteriser.register(&face) else {
                continue;
            };
            let mut refused_here = 0_u64;
            for glyph in 0..6_u16 {
                for pixels_per_em in [1.0_f32, 17.0, OUTLINE_PIXELS_PER_EM_THRESHOLD * 2.0] {
                    // What a damaged face *draws* is deliberately not asserted on — it may draw
                    // anything. The claim is that it comes back as a value or a typed error rather
                    // than unwinding the process.
                    match rasteriser.render(key_at(identity, GlyphIndex(glyph), pixels_per_em)) {
                        Ok(_) => {}
                        Err(FontError::UnreadableGlyphOutline { .. }) => {
                            refused_here += 1;
                            unreadable += 1;
                        }
                        Err(other) => panic!("an unexpected failure on a damaged face: {other}"),
                    }
                    rendered += 1;
                }
            }
            assert_eq!(
                rasteriser.statistics().unreadable_glyphs,
                refused_here,
                "the rasteriser's own count of glyphs it refused must match the errors it returned"
            );
        }
    }

    assert!(
        rendered > 1_000,
        "the corruption sweep must actually have reached the rasteriser: it made {rendered} \
         attempts, which means almost every damaged face was rejected before it got there"
    );
    // Without this the test is the vacuous kind this programme keeps finding: "nothing panicked" is
    // green for a sweep that never reached anything hostile. `read-fonts 0.41.0` — which `swash`
    // pins through `skrifa` — indexes a zero-length slice when a `glyf` entry's `endPtsOfContours`
    // wraps its point count to zero, and the boundary in `raster.rs` is what turns that into the
    // error counted here. If this number is ever zero, either the sweep stopped reaching the defect
    // or the boundary stopped being needed; both are worth knowing.
    assert!(
        unreadable > 0,
        "the sweep must actually have provoked the upstream parser defect the guard exists for, or \
         it proves only that nothing happened"
    );
}
