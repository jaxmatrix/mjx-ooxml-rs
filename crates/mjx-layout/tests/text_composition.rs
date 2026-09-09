//! Composing a line — and the first assertions in the workspace on
//! [`ShapedGlyph::unsafe_to_break`](mjx_text::ShapedGlyph).
//!
//! # Why that flag needed a consumer
//!
//! MJXOFF-158 produced it, documented it as *"a line breaker that splits here must re-shape both
//! halves rather than slicing the glyph list, and one that slices anyway will lose a ligature or a
//! kern at every line end"*, and nothing anywhere read it. A field with **zero readers** cannot be
//! depended on by any assertion, so until something consumed it there was no way to tell a correct
//! value from a constant `false`.
//!
//! `mjx-layout` is its first consumer, in [`slice_width`], and these are its first assertions. They
//! are written so that the flag's *value* is what decides the outcome — a fixture where slicing and
//! re-shaping give different numbers, and a boundary where the only thing distinguishing safe from
//! unsafe is the flag itself.
//!
//! # The two fixtures, and why both are needed
//!
//! **Carlito, `"office"` — the ligature.** Carlito forms `ffi` as one glyph spanning bytes 1..4, so
//! bytes 2 and 3 have *no glyph of their own*. Slicing there is not merely inaccurate, it is
//! undefined: there is nothing to slice at. This is the case the hand-off names.
//!
//! **Liberation Sans, `"AV"` — the kern.** Here a glyph *does* begin at byte 1, so an implementation
//! that only checked "is there a glyph at this offset" would happily slice. The pair is kerned, so
//! `A`'s advance inside `"AV"` is not `A`'s advance alone, and the **only** thing that says so is
//! `unsafe_to_break`. That is why this fixture exists beside the ligature one: it is the case that
//! distinguishes reading the flag from ignoring it.

mod support;

use std::sync::Arc;

use mjx_layout::{slice_width, LineComposer, TextRun};
use mjx_text::{
    BidiAnalysis, FeatureSet, FontFace, FontSize, GlyphRasteriser, LineBreakOptions,
    ParagraphDirection, ShapedRun, Shaper, ShapingRequest, TextDirection, TextScript,
};

const SIZE: FontSize = FontSize::from_half_points(24); // 12 pt

fn shape(shaper: &mut Shaper, face: &Arc<FontFace>, text: &str) -> ShapedRun {
    let features = FeatureSet::default();
    shaper
        .shape(
            face,
            &ShapingRequest::new(text, TextScript::of_character('a'), SIZE, &features),
        )
        .expect("the bundled faces shape")
}

// -------------------------------------------------------------------------------------------
// The flag itself
// -------------------------------------------------------------------------------------------

#[test]
fn slicing_at_a_kerned_boundary_is_refused_because_the_shaper_flagged_it() {
    let face = support::liberation_sans();
    let mut shaper = Shaper::new();

    let pair = shape(&mut shaper, &face, "AV");
    assert_eq!(
        pair.len(),
        2,
        "`AV` shapes to two glyphs in Liberation Sans"
    );

    // The fixture rests on the flag being set here — and on a glyph *existing* at byte 1, which is
    // what makes this different from the ligature case below.
    let boundary = pair.glyphs()[1];
    assert_eq!(boundary.cluster, 1, "a glyph begins at byte 1");
    assert!(
        boundary.unsafe_to_break,
        "Liberation Sans kerns `AV`, so HarfBuzz must mark the boundary unsafe; without that this \
         test proves nothing"
    );

    // So slicing is refused.
    assert_eq!(
        slice_width(&pair, &(0..2), &(0..1)),
        None,
        "the width of `A` may not be taken from the shaping of `AV`"
    );
    assert_eq!(slice_width(&pair, &(0..2), &(1..2)), None);

    // And the refusal matters: the two answers differ by a real kern.
    let sliced = pair.glyphs()[0].x_advance;
    let reshaped = shape(&mut shaper, &face, "A");
    assert_eq!(reshaped.len(), 1);
    let alone = reshaped.glyphs()[0].x_advance;
    assert_ne!(
        sliced, alone,
        "if `A` measured the same inside `AV` as it does alone, the flag would be describing \
         nothing and this fixture would be the wrong one"
    );
    assert!(
        alone > sliced,
        "the kern pulls `V` toward `A`, so `A` alone is wider: {alone} against {sliced}"
    );

    // The whole run and the two halves shaped separately are different widths — which is the error a
    // slicing line breaker makes at every line end.
    let halves =
        reshaped.advance().font_units + shape(&mut shaper, &face, "V").advance().font_units;
    assert_ne!(pair.advance().font_units, halves);
}

#[test]
fn slicing_inside_a_ligature_is_refused_because_there_is_no_glyph_there() {
    let face = support::carlito();
    let mut shaper = Shaper::new();

    let word = shape(&mut shaper, &face, "office");
    // Carlito ligates `ffi`, so `office` shapes to four glyphs at clusters 0, 1, 4 and 5.
    let clusters: Vec<u32> = word.glyphs().iter().map(|glyph| glyph.cluster).collect();
    assert_eq!(
        clusters,
        vec![0, 1, 4, 5],
        "Carlito must form the `ffi` ligature, or this fixture is not about a ligature"
    );

    // Bytes 2 and 3 are inside the ligature's cluster: there is no glyph to slice at.
    assert_eq!(slice_width(&word, &(0..6), &(0..2)), None);
    assert_eq!(slice_width(&word, &(0..6), &(0..3)), None);
    // Byte 4 has a glyph, and it is not flagged, so slicing there is allowed.
    assert!(slice_width(&word, &(0..6), &(0..4)).is_some());

    // What slicing would have cost. A naive breaker that attributed the whole ligature glyph to the
    // first half would measure `of` as `offi` and lose the ligature entirely on the second line.
    let naive: i32 = word.glyphs()[..2].iter().map(|glyph| glyph.x_advance).sum();
    let honest = shape(&mut shaper, &face, "of").advance().font_units;
    assert_ne!(
        naive, honest,
        "slicing `office` at byte 2 must give a different width from shaping `of`, or the ligature \
         is not doing anything"
    );
    assert!(
        naive > honest,
        "the sliced width carries three characters' worth of ligature: {naive} against {honest}"
    );

    // And re-shaping the two halves loses the ligature, which is the visible consequence: four
    // glyphs become five.
    let second_half = shape(&mut shaper, &face, "fice");
    assert_eq!(
        shape(&mut shaper, &face, "of").len() + second_half.len(),
        5,
        "split at byte 2, `office` draws five glyphs instead of four — the ligature is gone"
    );
}

#[test]
fn a_boundary_the_shaper_did_not_flag_slices_exactly() {
    // The other side of the rule. If every boundary were refused, `slice_width` would be a constant
    // `None` and the measurement path it exists for would never run.
    let face = support::liberation_sans();
    let mut shaper = Shaper::new();
    let text = "many words here";
    let whole = shape(&mut shaper, &face, text);

    let space = text.find(' ').expect("a space");
    let sliced = slice_width(&whole, &(0..text.len()), &(0..space))
        .expect("`many` is not a kerned boundary in Liberation Sans");
    let reshaped = shape(&mut shaper, &face, &text[..space]).advance_in_points();
    assert!(
        (sliced - reshaped).abs() < 1e-9,
        "at a safe boundary the sliced and re-shaped widths must be identical: {sliced} against \
         {reshaped}"
    );

    // The whole run and the empty slice are the trivial boundaries, and both are safe by definition.
    assert_eq!(
        slice_width(&whole, &(0..text.len()), &(0..text.len())),
        Some(whole.advance_in_points())
    );
    assert_eq!(
        slice_width(&whole, &(0..text.len()), &(0..0)),
        Some(0.0),
        "an empty slice is zero wide"
    );
}

// -------------------------------------------------------------------------------------------
// The composer
// -------------------------------------------------------------------------------------------

fn runs<'a>(
    text: &str,
    face: &'a Arc<FontFace>,
    face_id: mjx_text::FaceId,
    features: &'a FeatureSet,
) -> Vec<TextRun<'a>> {
    vec![TextRun {
        range: 0..text.len(),
        face,
        face_id,
        script: TextScript::of_character('a'),
        direction: TextDirection::LeftToRight,
        level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
        size: SIZE,
        features,
        language: None,
        advance: None,
    }]
}

#[test]
fn a_composed_line_covers_its_text_and_its_segments_measure_what_they_draw() {
    let face = support::liberation_sans();
    let mut rasteriser = GlyphRasteriser::new();
    let face_id = rasteriser.register(&face).expect("the face registers");
    let features = FeatureSet::default();
    let mut shaper = Shaper::new();

    let text = "The quick brown fox jumps over the lazy dog";
    let bidi = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
    let items = runs(text, &face, face_id, &features);
    let composer = LineComposer::new(text, &items, &bidi, LineBreakOptions::default());

    let mut cursor = 0;
    let mut covered = String::new();
    let mut lines = 0;
    while cursor < text.len() {
        let line = composer
            .next_line(&mut shaper, cursor, 80.0)
            .expect("the face shapes");
        assert!(line.range.end > cursor, "no progress at {cursor}");
        covered.push_str(&text[line.range.clone()]);
        assert_eq!(line.segments.len(), 1, "one face, so one segment per line");
        let segment = &line.segments[0];
        assert_eq!(segment.range, line.range);
        assert!(
            (segment.width_in_points - line.width_in_points).abs() < 1e-9,
            "one segment, so the line is as wide as it"
        );
        assert!(line.ascent_in_points > 0.0 && line.descent_in_points > 0.0);

        // Horizontal layout ignores `y_advance` because it is zero here. Asserted rather than
        // assumed, so the claim is checked: a face that produced a vertical advance in a horizontal
        // run would break the line height silently.
        for glyph in segment.run.glyphs() {
            assert_eq!(glyph.y_advance, 0, "horizontal text advances only in x");
        }

        cursor = line.range.end;
        lines += 1;
    }
    assert_eq!(covered, text, "every byte of the paragraph reached a line");
    assert!(lines >= 3, "the measure must have wrapped: {lines} line(s)");
}

#[test]
fn a_paragraph_with_no_interior_break_opportunity_is_composed_without_probing_the_measure() {
    // The reason `LineBreaker::opportunities` is read rather than the free `break_opportunities`
    // being called: knowing the paragraph's opportunities up front lets the composer skip the
    // measuring loop for a paragraph that cannot break at all — a title, a cell, one long word.
    let face = support::liberation_sans();
    let mut rasteriser = GlyphRasteriser::new();
    let face_id = rasteriser.register(&face).expect("the face registers");
    let features = FeatureSet::default();
    let mut shaper = Shaper::new();

    let unbreakable = "antidisestablishmentarianism";
    let bidi = BidiAnalysis::resolve(unbreakable, ParagraphDirection::LeftToRight);
    let items = runs(unbreakable, &face, face_id, &features);
    let composer = LineComposer::new(unbreakable, &items, &bidi, LineBreakOptions::default());
    assert!(!composer.can_break_before_the_end(0));

    let line = composer
        .next_line(&mut shaper, 0, 5.0)
        .expect("the face shapes");
    assert_eq!(line.range, 0..unbreakable.len());
    assert_eq!(
        line.measure_probes, 0,
        "a paragraph that cannot break must not be measured candidate by candidate"
    );
    assert!(line.is_last());

    // And a paragraph that *can* break is probed, so the fast path is discriminating rather than
    // universal.
    let breakable = "anti dis establishment arian ism";
    let bidi = BidiAnalysis::resolve(breakable, ParagraphDirection::LeftToRight);
    let items = runs(breakable, &face, face_id, &features);
    let composer = LineComposer::new(breakable, &items, &bidi, LineBreakOptions::default());
    assert!(composer.can_break_before_the_end(0));
    let line = composer
        .next_line(&mut shaper, 0, 40.0)
        .expect("the face shapes");
    assert!(line.measure_probes > 0, "a breakable paragraph is measured");
    assert!(line.range.end < breakable.len(), "and it wrapped");
}

#[test]
fn a_hanging_tail_is_drawn_but_not_counted_against_the_measure() {
    // `w:overflowPunct`. The line's own width includes the hanging tail — it *is* drawn there — and
    // `hanging_width_in_points` says how much of that was not counted when the line was fitted. A
    // justification pass and a column-balancing pass both need the difference.
    //
    // Found by mutation: zeroing `hanging_width_in_points` was **green** across the whole suite
    // before this test existed, because every other case runs under `LineBreakOptions::default()`,
    // where nothing hangs. A field produced and never asserted non-zero is a field no assertion can
    // depend on.
    let face = support::liberation_sans();
    let mut rasteriser = GlyphRasteriser::new();
    let face_id = rasteriser.register(&face).expect("the face registers");
    let features = FeatureSet::default();
    let mut shaper = Shaper::new();

    // The ASCII full stop is in JIS X 4051's hangable set, so Japanese typesetting hangs it even in
    // Latin text — which is exactly the behaviour MJXOFF-160 took out of `Default` and left here.
    let text = "aaa bbb ccc.";
    let full_stop = text.len() - 1;
    let bidi = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
    let items = runs(text, &face, face_id, &features);

    let full_stop_width = shape(&mut shaper, &face, ".").advance_in_points();
    assert!(full_stop_width > 0.0, "a full stop is not zero wide");
    let whole_width = shape(&mut shaper, &face, text).advance_in_points();

    let japanese = LineComposer::new(
        text,
        &items,
        &bidi,
        LineBreakOptions::japanese_typesetting(),
    );
    // A measure that fits everything except the stop.
    let measure = whole_width - full_stop_width / 2.0;
    let line = japanese
        .next_line(&mut shaper, 0, measure)
        .expect("the face shapes");
    assert_eq!(
        line.range,
        0..text.len(),
        "the whole line fits once the stop is allowed to hang"
    );
    assert_eq!(line.hanging, full_stop..text.len());
    assert!(
        (line.hanging_width_in_points - full_stop_width).abs() < 1e-9,
        "the hanging width is the width of the stop: {} against {full_stop_width}",
        line.hanging_width_in_points
    );
    assert!(
        line.width_in_points > measure,
        "the line is drawn wider than the measure, which is what hanging means: {} against \
         {measure}",
        line.width_in_points
    );
    assert!(
        line.width_in_points - line.hanging_width_in_points <= measure,
        "…and what was counted against the measure fitted it"
    );

    // The same text under the default options does not hang, so the line falls back to an earlier
    // break. If the two agreed, this fixture would prove nothing.
    let plain = LineComposer::new(text, &items, &bidi, LineBreakOptions::default());
    let plain_line = plain
        .next_line(&mut shaper, 0, measure)
        .expect("the face shapes");
    assert_eq!(plain_line.hanging_width_in_points, 0.0);
    assert!(
        plain_line.range.end < text.len(),
        "with nothing hanging, the line cannot reach the end: {:?}",
        plain_line.range
    );
}

#[test]
fn a_right_to_left_line_is_shaped_logically_and_emitted_visually() {
    // Shape in `logical_runs` order, place in `visual_runs` order. Getting it backwards produces
    // text that measures correctly and draws in the wrong order, which is why the assertion is on
    // the *order of the segments* rather than on the line's width.
    let face = support::liberation_sans();
    let mut rasteriser = GlyphRasteriser::new();
    let face_id = rasteriser.register(&face).expect("the face registers");
    let features = FeatureSet::default();
    let mut shaper = Shaper::new();

    // A Hebrew word between two Latin ones, in a left-to-right paragraph. The Hebrew is one level
    // run at level 1; the two Latin stretches are at level 0.
    let text = "one שלום two";
    let hebrew_start = "one ".len();
    let hebrew_end = hebrew_start + "שלום".len();
    let bidi = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
    assert_eq!(
        bidi.level_at(hebrew_start).direction(),
        TextDirection::RightToLeft,
        "the fixture rests on the middle word resolving right-to-left"
    );

    // Three items, cut at the direction boundaries, each carrying its own resolved direction — which
    // is what `itemise` produces from a real document.
    let items = vec![
        TextRun {
            range: 0..hebrew_start,
            face: &face,
            face_id,
            script: TextScript::of_character('a'),
            direction: TextDirection::LeftToRight,
            level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
            size: SIZE,
            features: &features,
            language: None,
            advance: None,
        },
        TextRun {
            range: hebrew_start..hebrew_end,
            face: &face,
            face_id,
            script: TextScript::of_character('ש'),
            direction: TextDirection::RightToLeft,
            level: mjx_text::BidiLevel::RIGHT_TO_LEFT,
            size: SIZE,
            features: &features,
            language: None,
            advance: None,
        },
        TextRun {
            range: hebrew_end..text.len(),
            face: &face,
            face_id,
            script: TextScript::of_character('a'),
            direction: TextDirection::LeftToRight,
            level: mjx_text::BidiLevel::LEFT_TO_RIGHT,
            size: SIZE,
            features: &features,
            language: None,
            advance: None,
        },
    ];
    let composer = LineComposer::new(text, &items, &bidi, LineBreakOptions::default());
    let line = composer
        .next_line(&mut shaper, 0, 10_000.0)
        .expect("the face shapes");
    assert_eq!(line.range, 0..text.len(), "it all fits on one line");
    assert_eq!(line.segments.len(), 3);

    // The segments are in visual order, and their ranges stay logical — which is what lets a caret
    // walk them in document order while a painter walks them left to right.
    let starts: Vec<usize> = line
        .segments
        .iter()
        .map(|segment| segment.range.start)
        .collect();
    assert_eq!(
        starts,
        vec![0, hebrew_start, hebrew_end],
        "one Hebrew run between two Latin ones puts the three in the same order visually as \
         logically; the segments' ranges are logical either way"
    );
    assert_eq!(
        line.segments[1].direction,
        TextDirection::RightToLeft,
        "the middle segment carries its own direction"
    );
    assert!(line.segments[1].level.direction().is_right_to_left());
}

#[test]
fn two_right_to_left_items_in_one_level_run_come_out_reversed() {
    // The half of rule L2 that a level-run-only implementation gets wrong: within one right-to-left
    // level run, a font change splits it into two items, and the logically *last* is the visually
    // leftmost. A composer that emitted them in logical order would draw the words the wrong way
    // round while measuring the line correctly.
    let sans = support::liberation_sans();
    let serif = support::bundled_face("LiberationSerif-Regular.ttf");
    let mut rasteriser = GlyphRasteriser::new();
    let sans_id = rasteriser.register(&sans).expect("the face registers");
    let serif_id = rasteriser.register(&serif).expect("the face registers");
    let features = FeatureSet::default();
    let mut shaper = Shaper::new();

    let text = "שלום עולם";
    let split = "שלום ".len();
    let bidi = BidiAnalysis::resolve(text, ParagraphDirection::RightToLeft);
    assert_eq!(bidi.base_direction(), TextDirection::RightToLeft);
    assert_eq!(
        bidi.visual_runs(0..text.len()).len(),
        1,
        "the whole line is one level run, so only the item split can reverse anything"
    );

    let items = vec![
        TextRun {
            range: 0..split,
            face: &sans,
            face_id: sans_id,
            script: TextScript::of_character('ש'),
            direction: TextDirection::RightToLeft,
            level: mjx_text::BidiLevel::RIGHT_TO_LEFT,
            size: SIZE,
            features: &features,
            language: None,
            advance: None,
        },
        TextRun {
            range: split..text.len(),
            face: &serif,
            face_id: serif_id,
            script: TextScript::of_character('ש'),
            direction: TextDirection::RightToLeft,
            level: mjx_text::BidiLevel::RIGHT_TO_LEFT,
            size: SIZE,
            features: &features,
            language: None,
            advance: None,
        },
    ];
    let composer = LineComposer::new(text, &items, &bidi, LineBreakOptions::default());
    let line = composer
        .next_line(&mut shaper, 0, 10_000.0)
        .expect("the faces shape");
    assert_eq!(line.segments.len(), 2);
    assert_eq!(
        line.segments[0].range.start, split,
        "the logically second item is drawn first, because the level run is right-to-left"
    );
    assert_eq!(line.segments[1].range.start, 0);
    assert_eq!(line.segments[0].face, serif_id);
    assert_eq!(line.segments[1].face, sans_id);
}

#[test]
fn a_slice_taken_from_a_right_to_left_run_measures_what_shaping_it_would() {
    // A right-to-left run's glyphs are in visual order, so a logical prefix is a glyph *suffix*. An
    // implementation that took the prefix would measure the wrong end of the word and be wrong by
    // however much the two ends differ — which is why this compares against re-shaping rather than
    // against itself.
    let face = support::liberation_sans();
    let mut shaper = Shaper::new();
    let features = FeatureSet::default();
    let text = "שלום";
    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new(text, TextScript::of_character('ש'), SIZE, &features)
                .in_direction(TextDirection::RightToLeft),
        )
        .expect("the face shapes");
    assert!(run.direction().is_right_to_left());

    let clusters: Vec<u32> = run.glyphs().iter().map(|glyph| glyph.cluster).collect();
    let mut descending = clusters.clone();
    descending.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(
        clusters, descending,
        "the fixture rests on the glyphs coming out in visual order, clusters descending"
    );

    // Every safe boundary must agree with re-shaping that prefix, and at least one must be safe or
    // the test is vacuous.
    let mut compared = 0;
    for boundary in text.char_indices().map(|(at, _)| at).chain([text.len()]) {
        let Some(sliced) = slice_width(&run, &(0..text.len()), &(0..boundary)) else {
            continue;
        };
        let reshaped = shaper
            .shape(
                &face,
                &ShapingRequest::new(
                    &text[..boundary],
                    TextScript::of_character('ש'),
                    SIZE,
                    &features,
                )
                .in_direction(TextDirection::RightToLeft),
            )
            .expect("the face shapes")
            .advance_in_points();
        assert!(
            (sliced - reshaped).abs() < 1e-9,
            "prefix 0..{boundary} sliced to {sliced} and re-shapes to {reshaped}"
        );
        compared += 1;
    }
    assert!(
        compared >= 3,
        "only {compared} boundaries were safe; the right-to-left slice path is barely exercised"
    );
}
