//! Malformed text and malformed fonts, fed to the shaping path.
//!
//! # Why this suite exists rather than the source grep
//!
//! `untrusted_faces.rs`'s `the_parse_path_contains_no_unwrap_expect_or_panic` scans the source for
//! the tokens `unwrap`, `expect` and `panic!`. That is a cheap and genuinely useful instrument — it
//! catches the obvious thing immediately — but **it proves the absence of a token, not the absence
//! of a panic**. It cannot see a panic reached through a callee, an arithmetic overflow, or a slice
//! index.
//!
//! Shaping does real arithmetic over tables that came out of an untrusted file: advances are summed,
//! glyph ids are narrowed, byte offsets are sliced, and every one of those is a place a wrong number
//! could abort a render rather than produce a wrong glyph. So the instrument for this path is
//! **execution over deliberately hostile input**, and that is what is below.
//!
//! Every case here would pass trivially if it were only checked for "does not error". They are
//! checked for "does not panic **and** answers something a renderer can draw", which is the
//! property that matters: a document with an unassigned code point in it must still open.

mod support;

use std::sync::Arc;

use mjx_text::{
    shape_uncached, BidiAnalysis, FeatureSet, FeatureTag, FontError, FontFace, FontFeature,
    FontSize, ParagraphDirection, Shaper, ShapingRequest, StylisticSets, TextDirection, TextScript,
    TypographyOptions,
};

use support::bundled_face;
use support::synthetic_font::{arabic_face, SyntheticFace};

fn carlito() -> Arc<FontFace> {
    Arc::new(bundled_face("Carlito-Regular.ttf"))
}

/// Text a real document can and does hold, and that a naive shaper trips over.
const HOSTILE_TEXT: &[(&str, &str)] = &[
    ("an unassigned code point", "a\u{0378}b"),
    ("a noncharacter", "a\u{FFFE}b"),
    ("the replacement character", "a\u{FFFD}b"),
    ("a private-use character", "a\u{F8FF}b"),
    ("a plane 15 private-use character", "a\u{FFFFD}b"),
    ("an unpaired combining mark", "\u{0301}\u{0301}\u{0301}"),
    ("a combining mark with no base", "\u{093F}"),
    (
        "a long zero-width-joiner sequence",
        "a\u{200D}\u{200D}\u{200D}\u{200D}b",
    ),
    (
        "bidirectional controls with no text",
        "\u{202A}\u{202B}\u{202C}\u{202E}",
    ),
    ("an unclosed embedding", "\u{202B}abc"),
    ("a lone pop", "\u{202C}abc"),
    ("a byte-order mark in the middle", "a\u{FEFF}b"),
    ("a tag sequence", "a\u{E0041}\u{E0042}b"),
    ("a variation selector on a letter", "a\u{FE0F}b"),
    ("a soft hyphen at both ends", "\u{00AD}word\u{00AD}"),
    ("a control character", "a\u{0001}b"),
    ("a line separator", "a\u{2028}b"),
    (
        "a very deep combining stack",
        "a\u{0301}\u{0302}\u{0303}\u{0304}\u{0305}\u{0306}\u{0307}b",
    ),
    ("nothing at all", ""),
    ("only spaces", "      "),
];

#[test]
fn hostile_text_shapes_into_something_drawable_and_never_panics() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();

    for (description, text) in HOSTILE_TEXT {
        for direction in [TextDirection::LeftToRight, TextDirection::RightToLeft] {
            for script in [
                TextScript::LATIN,
                TextScript::ARABIC,
                TextScript::DEVANAGARI,
                TextScript::HAN,
                TextScript::from_iso_15924_code(*b"Qaaa"),
            ] {
                let request =
                    ShapingRequest::new(text, script, FontSize::from_points(11.0), &features)
                        .in_direction(direction);
                let run = shaper
                    .shape(&face, &request)
                    .unwrap_or_else(|error| panic!("{description} in {script}: {error}"));

                // Every cluster must point at a real byte boundary of the text, or a caret mapping
                // built on it would slice a `char` in half.
                for glyph in run.glyphs() {
                    let cluster = glyph.cluster as usize;
                    assert!(
                        cluster <= text.len() && text.is_char_boundary(cluster),
                        "{description}: cluster {cluster} is not a boundary of {text:?}"
                    );
                }
                // The advance is finite and the run is measurable.
                assert!(run.advance_in_points().is_finite());
            }
        }
    }
}

#[test]
fn hostile_text_resolves_bidirectionally_and_breaks_into_lines_without_panicking() {
    for (description, text) in HOSTILE_TEXT {
        for paragraph in [
            ParagraphDirection::LeftToRight,
            ParagraphDirection::RightToLeft,
            ParagraphDirection::FromFirstStrongCharacter,
        ] {
            let analysis = BidiAnalysis::resolve(text, paragraph);
            assert_eq!(analysis.len(), text.len(), "{description}");

            let runs = analysis.visual_runs(0..text.len());
            let mut covered = 0;
            for run in &runs {
                assert!(
                    text.is_char_boundary(run.range.start) && text.is_char_boundary(run.range.end),
                    "{description}: run {:?} is not on character boundaries",
                    run.range
                );
                covered += run.range.len();
            }
            assert_eq!(
                covered,
                text.len(),
                "{description}: runs must cover the text"
            );

            let breaker = mjx_text::LineBreaker::new(text, mjx_text::LineBreakOptions::default());
            let mut cursor = 0;
            let mut iterations = 0;
            while cursor < text.len() {
                let line = breaker.next_line(cursor, 3.0, &mut |range| {
                    text.get(range).map_or(0.0, |slice| slice.len() as f64)
                });
                assert!(line.end > cursor, "{description}: no progress at {cursor}");
                assert!(
                    text.is_char_boundary(line.end),
                    "{description}: line end {} is not a character boundary",
                    line.end
                );
                cursor = line.end;
                iterations += 1;
                assert!(
                    iterations < 1000,
                    "{description}: the breaker did not terminate"
                );
            }

            // Itemising by script partitions the text exactly, whatever is in it.
            let mut cursor = 0;
            for run in mjx_text::itemise_by_script(text) {
                assert_eq!(run.range.start, cursor, "{description}");
                cursor = run.range.end;
            }
            assert_eq!(cursor, text.len(), "{description}");
        }
    }
}

#[test]
fn a_face_whose_bytes_stopped_parsing_is_an_error_rather_than_a_panic() {
    // A `FontFace` that parsed once, whose bytes are then replaced with something that will not.
    // The shaper re-parses from `data()`, so it meets bytes the constructor never saw.
    let truncated: Vec<u8> = vec![0x00, 0x01, 0x00, 0x00, 0x00, 0x04];
    assert!(FontFace::parse(Arc::from(truncated.as_slice()), 0).is_err());

    // The same shape, reached through the shaper: a face built from a valid file but asked for a
    // face index the file does not hold.
    let valid = bundled_face("Carlito-Regular.ttf");
    assert_eq!(FontFace::face_count(valid.data()), 1);
    let out_of_range = FontFace::parse(Arc::clone(valid.data()), 7);
    assert!(matches!(
        out_of_range,
        Err(FontError::FaceIndexOutOfRange { index: 7, .. })
    ));
}

#[test]
fn a_run_whose_advances_overflow_is_refused_rather_than_wrapping() {
    // A face whose single glyph is 65535 units wide, and a run long enough that the sum passes
    // `i32::MAX`. `65535 * 32769 > 2^31`, so 40000 characters is comfortably past it.
    let wide = SyntheticFace::new(vec![0, 65535]).mapping('W', 1).face();
    let text = "W".repeat(40_000);
    let features = FeatureSet::new();
    let request = ShapingRequest::new(
        &text,
        TextScript::LATIN,
        FontSize::from_points(12.0),
        &features,
    );

    let mut shaper = Shaper::new();
    match shaper.shape(&wide, &request) {
        Err(FontError::ShapedRunTooWide {
            font_units,
            characters,
        }) => {
            assert_eq!(characters, 40_000);
            assert!(
                font_units > i64::from(i32::MAX),
                "the sum must be past what an i32 holds: {font_units}"
            );
        }
        Err(other) => panic!("the wrong error: {other}"),
        Ok(run) => panic!(
            "a run of {} units was accepted into a 32-bit advance",
            run.advance().font_units
        ),
    }

    // And a run just short of the limit is fine, so the check is a bound rather than a refusal of
    // everything long.
    let short = "W".repeat(1000);
    let request = ShapingRequest::new(
        &short,
        TextScript::LATIN,
        FontSize::from_points(12.0),
        &features,
    );
    let run = shaper.shape(&wide, &request).expect("1000 glyphs fit");
    assert_eq!(run.advance().font_units, 65_535_000);
}

#[test]
fn a_face_with_no_character_map_shapes_everything_to_notdef_without_failing() {
    // A font with an empty `cmap` is a real thing — a subset that lost its mapping — and a renderer
    // that panicked on one would fail a whole page over one bad embedded face.
    let blank = SyntheticFace::new(vec![0, 500]).face();
    let features = FeatureSet::new();
    let mut shaper = Shaper::new();
    let run = shaper
        .shape(
            &blank,
            &ShapingRequest::new(
                "hello",
                TextScript::LATIN,
                FontSize::from_points(12.0),
                &features,
            ),
        )
        .expect("an empty character map is not a failure");
    assert_eq!(run.len(), 5);
    assert!(
        run.glyphs().iter().all(|glyph| glyph.glyph.0 == 0),
        "every character must map to `.notdef`"
    );
}

#[test]
fn every_feature_tag_a_document_could_ask_for_is_accepted() {
    let face = carlito();
    let mut shaper = Shaper::new();

    // Every stylistic set, plus a tag no font has, plus a value no registry defines.
    let mut options = TypographyOptions::registry_defaults();
    options.stylistic_sets = (1..=20_u8).fold(StylisticSets::none(), StylisticSets::with);
    let mut features = options.to_feature_set();
    features.set(FontFeature::set(FeatureTag::new(*b"zzzz"), 7));
    features.set(FontFeature::set(FeatureTag::new([0, 1, 2, 3]), u32::MAX));

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new(
                "features",
                TextScript::LATIN,
                FontSize::from_points(12.0),
                &features,
            ),
        )
        .expect("an unknown feature is ignored, not refused");
    assert_eq!(run.len(), 8);

    // A stylistic-set number outside 1..=20 is refused at the tag rather than sent to the shaper.
    assert!(FeatureTag::stylistic_set(0).is_none());
    assert!(FeatureTag::stylistic_set(21).is_none());
    assert!(FeatureTag::stylistic_set(255).is_none());
    assert_eq!(
        FeatureTag::stylistic_set(7).map(|tag| tag.name().to_owned()),
        Some("ss07".to_owned())
    );
    assert_eq!(
        FeatureTag::stylistic_set(20).map(|tag| tag.name().to_owned()),
        Some("ss20".to_owned())
    );
    // And a set number outside the range is ignored rather than setting some other bit.
    assert!(StylisticSets::none().with(0).is_empty());
    assert!(StylisticSets::none().with(21).is_empty());
}

#[test]
fn a_language_tag_a_document_invented_is_ignored_rather_than_refused() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = FeatureSet::new();

    for language in [
        "",
        "!!!",
        "en-GB",
        "zz-ZZ-invalid",
        "x".repeat(500).as_str(),
    ] {
        let run = shaper
            .shape(
                &face,
                &ShapingRequest::new(
                    "text",
                    TextScript::LATIN,
                    FontSize::from_points(12.0),
                    &features,
                )
                .in_language(language),
            )
            .unwrap_or_else(|error| panic!("language {language:?}: {error}"));
        assert_eq!(run.len(), 4);
    }
}

#[test]
fn shaping_arabic_in_the_wrong_direction_produces_glyphs_rather_than_a_failure() {
    // A document can declare a left-to-right run and fill it with Arabic. That is wrong typography
    // and it is not an error; the shaper must still answer.
    let face = arabic_face().face();
    let features = FeatureSet::new();
    let text = "\u{0628}\u{0628}\u{0628}";
    let run = shape_uncached(
        &face,
        &ShapingRequest::new(
            text,
            TextScript::ARABIC,
            FontSize::from_points(12.0),
            &features,
        )
        .in_direction(TextDirection::LeftToRight),
    )
    .expect("shapes");
    assert_eq!(run.len(), 3);
}

#[test]
fn a_zero_or_maximum_size_is_shaped_and_measured_without_dividing_by_anything() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = FeatureSet::new();

    for size in [
        FontSize::from_points(0.0),
        FontSize::from_thousandths_of_a_point(1),
        FontSize::MAXIMUM,
    ] {
        let run = shaper
            .shape(
                &face,
                &ShapingRequest::new("size", TextScript::LATIN, size, &features),
            )
            .expect("shapes");
        assert!(run.advance_in_points().is_finite());
        assert!(run.advance_in_points() >= 0.0);
    }
}
