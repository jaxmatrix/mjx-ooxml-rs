//! Shaping, asserted where a naive implementation is wrong.
//!
//! # The trap this suite exists to avoid
//!
//! *"Shaping a Latin string produces the same number of glyphs as characters"* is true of an
//! implementation that ignores every OpenType feature, reads no `GSUB`, applies no kern and simply
//! maps each code point through `cmap`. It is the assertion that always passes, and it proves
//! nothing.
//!
//! Every case below is one where that implementation gives a **different, checkable, wrong** answer:
//!
//! | Case | Naive answer | Correct answer |
//! |---|---|---|
//! | `fi` in Carlito | 2 glyphs, 1095 units | **1 glyph** (id 67), **1084 units** |
//! | `بب` in the synthetic Arabic face | glyph 10 twice | **12 then 14** — one form per position |
//! | `कि` in the synthetic Devanagari face | ka then matra | **matra then ka** — reordered |
//! | `AV` in Carlito | 2347 units | **2258 units** — 89 units of kerning |
//!
//! Every expected value is a literal, and every one of them was read out of the shaper before being
//! written here, not the other way round.

mod support;

use std::sync::Arc;

use mjx_text::{
    shape_uncached, AdvanceWidth, FeatureSet, FeatureTag, FontFeature, FontSize, Shaper,
    ShapingRequest, TextDirection, TextScript, TypographyOptions,
};

use support::synthetic_font::{arabic_face, devanagari_face, proportional_latin_face};
use support::{bundled_face, bundled_font_directory};

/// Twelve points, the size nothing here depends on but every request must carry.
fn twelve_points() -> FontSize {
    FontSize::from_points(12.0)
}

fn carlito() -> Arc<mjx_text::FontFace> {
    Arc::new(bundled_face("Carlito-Regular.ttf"))
}

fn glyph_ids(run: &mjx_text::ShapedRun) -> Vec<u16> {
    run.glyphs().iter().map(|glyph| glyph.glyph.0).collect()
}

// ---------------------------------------------------------------------------------------------
// 1 · A ligature reduces the glyph count.
// ---------------------------------------------------------------------------------------------

#[test]
fn the_fi_ligature_makes_two_characters_into_one_glyph() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new("fi", TextScript::LATIN, twelve_points(), &features),
        )
        .expect("Carlito shapes `fi`");

    // Two characters, ONE glyph. A shaper that mapped each code point through `cmap` would answer
    // two, with ids 61 and 98.
    assert_eq!("fi".chars().count(), 2);
    assert_eq!(
        run.len(),
        1,
        "`fi` is one glyph in Carlito, not {}",
        run.len()
    );
    assert_eq!(glyph_ids(&run), vec![67]);
    assert_eq!(run.advance().font_units, 1084);
    assert_eq!(run.advance().units_per_em, 2048);

    // The ligature is genuinely narrower than its parts: 1084 against 625 + 470 = 1095.
    let reader = face.reader().expect("Carlito re-parses");
    let unligated = reader
        .unshaped_advance("fi")
        .expect("Carlito covers `f` and `i`");
    assert_eq!(unligated.font_units, 1095);
    assert!(run.advance().font_units < unligated.font_units);
}

#[test]
fn switching_the_ligature_off_brings_both_glyphs_back() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let without = FeatureSet::new().with(FontFeature::off(FeatureTag::STANDARD_LIGATURES));

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new("fi", TextScript::LATIN, twelve_points(), &without),
        )
        .expect("Carlito shapes `fi`");

    // The same string, the same face, one feature switched off, and a different answer — which is
    // what proves the feature set reaches the shaper at all.
    assert_eq!(glyph_ids(&run), vec![61, 98]);
    assert_eq!(run.advance().font_units, 1095);
}

#[test]
fn a_three_character_ligature_reduces_six_characters_to_four_glyphs() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new("office", TextScript::LATIN, twelve_points(), &features),
        )
        .expect("Carlito shapes `office`");

    // `ffi` is one glyph, so six characters become four. The clusters show which characters each
    // glyph came from: the second glyph spans bytes 1, 2 and 3.
    assert_eq!("office".chars().count(), 6);
    assert_eq!(glyph_ids(&run), vec![111, 76, 49, 59]);
    let clusters: Vec<u32> = run.glyphs().iter().map(|glyph| glyph.cluster).collect();
    assert_eq!(clusters, vec![0, 1, 4, 5]);
    assert_eq!(run.advance().font_units, 4619);
}

// ---------------------------------------------------------------------------------------------
// 2 · An Arabic string whose glyph ids differ by position.
// ---------------------------------------------------------------------------------------------

/// `U+0628 ARABIC LETTER BEH`.
const BEH: &str = "\u{0628}";

#[test]
fn an_arabic_letter_takes_a_different_glyph_in_each_position() {
    let face = arabic_face().face();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();

    let shape = |shaper: &mut Shaper, text: &str| {
        glyph_ids(
            &shaper
                .shape(
                    &face,
                    &ShapingRequest::new(text, TextScript::ARABIC, twelve_points(), &features)
                        .in_direction(TextDirection::RightToLeft),
                )
                .expect("the synthetic Arabic face shapes"),
        )
    };

    // `cmap` maps BEH to glyph 10, and a naive shaper answers `[10]`, `[10, 10]`, `[10, 10, 10]`.
    // The shaper answers with one glyph per joining position.
    assert_eq!(shape(&mut shaper, BEH), vec![11], "isolated");
    let two = format!("{BEH}{BEH}");
    let three = format!("{BEH}{BEH}{BEH}");
    // Right-to-left output is in visual order, so the *final* form is leftmost.
    assert_eq!(shape(&mut shaper, &two), vec![14, 12], "final then initial");
    assert_eq!(
        shape(&mut shaper, &three),
        vec![14, 13, 12],
        "final, medial, initial"
    );

    // Not one of those four ids is 10, so nothing here could be a `cmap` lookup that happened to
    // agree.
    assert!(!shape(&mut shaper, &three).contains(&10));
}

#[test]
fn the_arabic_joining_forms_have_different_widths_too() {
    let face = arabic_face().face();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();
    let three = format!("{BEH}{BEH}{BEH}");

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new(&three, TextScript::ARABIC, twelve_points(), &features)
                .in_direction(TextDirection::RightToLeft),
        )
        .expect("the synthetic Arabic face shapes");

    // 540 (final) + 530 (medial) + 520 (initial). A shaper that drew glyph 10 three times would
    // answer 3 × 500 = 1500.
    assert_eq!(run.advance().font_units, 1590);
    let advances: Vec<i32> = run.glyphs().iter().map(|glyph| glyph.x_advance).collect();
    assert_eq!(advances, vec![540, 530, 520]);
}

#[test]
fn an_arabic_run_keeps_its_clusters_in_logical_order() {
    let face = arabic_face().face();
    let mut shaper = Shaper::new();
    let features = FeatureSet::new();
    let three = format!("{BEH}{BEH}{BEH}");

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new(&three, TextScript::ARABIC, twelve_points(), &features)
                .in_direction(TextDirection::RightToLeft),
        )
        .expect("the synthetic Arabic face shapes");

    // Glyphs come out in visual order; their clusters are the byte offsets they came from, so the
    // clusters run *down*. A caret mapping that assumed they ascend would jump.
    let clusters: Vec<u32> = run.glyphs().iter().map(|glyph| glyph.cluster).collect();
    assert_eq!(clusters, vec![4, 2, 0]);
}

// ---------------------------------------------------------------------------------------------
// 3 · A Devanagari cluster that reorders.
// ---------------------------------------------------------------------------------------------

/// `U+0915 DEVANAGARI LETTER KA`.
const KA: &str = "\u{0915}";
/// `U+093F DEVANAGARI VOWEL SIGN I`, which is *stored* after its consonant and *drawn* before it.
const VOWEL_SIGN_I: &str = "\u{093F}";

#[test]
fn a_devanagari_pre_base_matra_is_drawn_before_the_consonant_it_follows() {
    let face = devanagari_face().face();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();
    let syllable = format!("{KA}{VOWEL_SIGN_I}");

    let alone = shaper
        .shape(
            &face,
            &ShapingRequest::new(KA, TextScript::DEVANAGARI, twelve_points(), &features),
        )
        .expect("the synthetic Devanagari face shapes");
    assert_eq!(glyph_ids(&alone), vec![10]);

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new(
                &syllable,
                TextScript::DEVANAGARI,
                twelve_points(),
                &features,
            ),
        )
        .expect("the synthetic Devanagari face shapes");

    // The characters are ka (glyph 10) then the matra (glyph 11). A shaper that walked the string
    // would answer `[10, 11]`. The Indic shaper moves the pre-base matra in front of its base.
    assert_eq!(
        glyph_ids(&run),
        vec![11, 10],
        "the matra is drawn before the consonant it is written after"
    );

    // Every glyph belongs to the same cluster: the syllable is one unit, and a caret may not sit
    // inside it.
    let clusters: Vec<u32> = run.glyphs().iter().map(|glyph| glyph.cluster).collect();
    assert_eq!(clusters, vec![0, 0]);

    // The width is the sum either way, which is exactly why a width assertion would not have caught
    // the ordering and a glyph-order assertion is the one that does.
    assert_eq!(run.advance().font_units, 1000);
}

// ---------------------------------------------------------------------------------------------
// 4 · A kerned pair narrower than the sum of its parts.
// ---------------------------------------------------------------------------------------------

#[test]
fn a_kerned_pair_is_narrower_than_its_two_glyphs_measured_apart() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let with_kerning = TypographyOptions::registry_defaults().to_feature_set();

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new("AV", TextScript::LATIN, twelve_points(), &with_kerning),
        )
        .expect("Carlito shapes `AV`");

    let reader = face.reader().expect("Carlito re-parses");
    let unkerned = reader
        .unshaped_advance("AV")
        .expect("Carlito covers `A` and `V`");

    // 1185 + 1162 = 2347 apart; 2258 together. Eighty-nine units of kerning, and a shaper that
    // summed `hmtx` would answer 2347.
    assert_eq!(unkerned.font_units, 2347);
    assert_eq!(run.advance().font_units, 2258);
    assert_eq!(unkerned.font_units - run.advance().font_units, 89);

    // The kerning is applied to the *first* glyph's advance, which is where a painter reads it.
    let advances: Vec<i32> = run.glyphs().iter().map(|glyph| glyph.x_advance).collect();
    assert_eq!(advances, vec![1096, 1162]);
}

#[test]
fn switching_kerning_off_restores_the_sum_of_the_parts() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let without = FeatureSet::new().with(FontFeature::off(FeatureTag::KERNING));

    let run = shaper
        .shape(
            &face,
            &ShapingRequest::new("AV", TextScript::LATIN, twelve_points(), &without),
        )
        .expect("Carlito shapes `AV`");

    assert_eq!(run.advance().font_units, 2347);
    let advances: Vec<i32> = run.glyphs().iter().map(|glyph| glyph.x_advance).collect();
    assert_eq!(advances, vec![1185, 1162]);
}

// ---------------------------------------------------------------------------------------------
// The shaped-run cache
// ---------------------------------------------------------------------------------------------

#[test]
fn shaping_the_same_run_twice_does_no_shaping_work_the_second_time() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();
    let request = ShapingRequest::new(
        "the same run, twice",
        TextScript::LATIN,
        twelve_points(),
        &features,
    );

    let first = shaper.shape(&face, &request).expect("shapes once");
    let after_first = shaper.cache_statistics();
    assert_eq!(after_first.hits, 0);
    assert_eq!(after_first.misses, 1);
    assert_eq!(after_first.entries, 1);

    let second = shaper.shape(&face, &request).expect("shapes twice");
    let after_second = shaper.cache_statistics();
    assert_eq!(after_second.hits, 1, "the second call must be a cache hit");
    assert_eq!(
        after_second.misses, 1,
        "the second call must not have re-shaped"
    );
    assert_eq!(after_second.entries, 1);

    // Equal output would be true of a re-shape. Sharing the *allocation* is only true of a hit.
    assert!(
        first.shares_glyphs_with(&second),
        "a cache hit returns the glyphs the first call produced, not an equal copy"
    );
    assert_eq!(glyph_ids(&first), glyph_ids(&second));
}

#[test]
fn every_part_of_the_key_makes_the_cache_miss() {
    let face = carlito();
    let other_face = Arc::new(bundled_face("LiberationSans-Regular.ttf"));
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();
    let without_kerning = FeatureSet::new().with(FontFeature::off(FeatureTag::KERNING));

    let base = ShapingRequest::new("Wa", TextScript::LATIN, twelve_points(), &features);
    shaper.shape(&face, &base).expect("shapes");
    assert_eq!(shaper.cache_statistics().misses, 1);

    // A different face.
    shaper.shape(&other_face, &base).expect("shapes");
    // A different size.
    let mut request = base;
    request.size = FontSize::from_points(14.0);
    shaper.shape(&face, &request).expect("shapes");
    // A different direction.
    let mut request = base;
    request.direction = TextDirection::RightToLeft;
    shaper.shape(&face, &request).expect("shapes");
    // A different script.
    let mut request = base;
    request.script = TextScript::CYRILLIC;
    shaper.shape(&face, &request).expect("shapes");
    // A different language.
    let mut request = base;
    request.language = Some("tr");
    shaper.shape(&face, &request).expect("shapes");
    // Different features.
    let mut request = base;
    request.features = &without_kerning;
    shaper.shape(&face, &request).expect("shapes");
    // Different text.
    let mut request = base;
    request.text = "aW";
    shaper.shape(&face, &request).expect("shapes");

    let statistics = shaper.cache_statistics();
    assert_eq!(
        statistics.misses, 8,
        "every one of the seven variations must be a miss, plus the first call"
    );
    assert_eq!(statistics.hits, 0);
    assert_eq!(statistics.entries, 8);
}

#[test]
fn a_cache_of_zero_capacity_still_shapes_and_never_hits() {
    let face = carlito();
    let mut shaper = Shaper::with_cache_capacity(0);
    let features = FeatureSet::new();
    let request = ShapingRequest::new("fi", TextScript::LATIN, twelve_points(), &features);

    let first = shaper.shape(&face, &request).expect("shapes");
    let second = shaper.shape(&face, &request).expect("shapes");

    assert_eq!(glyph_ids(&first), glyph_ids(&second));
    assert!(!first.shares_glyphs_with(&second));
    assert_eq!(shaper.cache_statistics().hits, 0);
    assert_eq!(shaper.cache_statistics().entries, 0);
}

#[test]
fn a_full_cache_evicts_rather_than_growing() {
    let face = carlito();
    let mut shaper = Shaper::with_cache_capacity(8);
    let features = FeatureSet::new();

    for index in 0..64_u32 {
        let text = format!("run number {index}");
        let request = ShapingRequest::new(&text, TextScript::LATIN, twelve_points(), &features);
        shaper.shape(&face, &request).expect("shapes");
    }

    let statistics = shaper.cache_statistics();
    assert!(
        statistics.entries <= 8,
        "the cache held {} entries against a capacity of 8",
        statistics.entries
    );
    assert!(statistics.evictions > 0, "nothing was ever evicted");
    assert_eq!(statistics.misses, 64);
}

// ---------------------------------------------------------------------------------------------
// `AdvanceWidth::equals` — the cross-em comparison MJXOFF-157 handed over with no caller.
// ---------------------------------------------------------------------------------------------

#[test]
fn two_advances_are_equal_when_they_are_the_same_fraction_of_different_ems() {
    // Half an em, twice. `font_units` differ by a factor of two; the widths are identical.
    let thousand = AdvanceWidth {
        font_units: 500,
        units_per_em: 1000,
    };
    let two_thousand_and_forty_eight = AdvanceWidth {
        font_units: 1024,
        units_per_em: 2048,
    };

    assert_ne!(
        thousand.font_units, two_thousand_and_forty_eight.font_units,
        "the two advances must differ in font units, or the test proves nothing"
    );
    assert!(
        thousand.equals(two_thousand_and_forty_eight),
        "500/1000 and 1024/2048 are the same fraction of an em"
    );
    assert!(
        two_thousand_and_forty_eight.equals(thousand),
        "and both ways"
    );
    assert_eq!(thousand.per_mille(), 500.0);
    assert_eq!(two_thousand_and_forty_eight.per_mille(), 500.0);
    assert_eq!(
        thousand.deviation_per_mille(two_thousand_and_forty_eight),
        0.0
    );

    // One unit off at 2048 is not equal, and is a fifth of a per-mille away — well inside the
    // tolerance the metric gate compares with, which is exactly why `equals` is a different
    // question from `deviation_per_mille`.
    let nearly = AdvanceWidth {
        font_units: 1025,
        units_per_em: 2048,
    };
    assert!(!thousand.equals(nearly));
    assert!(thousand.deviation_per_mille(nearly) < 1.0);
}

#[test]
fn equal_font_units_against_different_ems_are_not_equal_widths() {
    // The case a naive `font_units == font_units` gets backwards: the numbers agree and the widths
    // do not.
    let half = AdvanceWidth {
        font_units: 1024,
        units_per_em: 2048,
    };
    let whole = AdvanceWidth {
        font_units: 1024,
        units_per_em: 1024,
    };
    assert_eq!(half.font_units, whole.font_units);
    assert!(!half.equals(whole));
    assert_eq!(half.per_mille(), 500.0);
    assert_eq!(whole.per_mille(), 1000.0);
}

#[test]
fn two_shaped_runs_at_different_em_squares_occupy_the_same_width() {
    // The same string in two faces that are proportionally identical and differ only in their em
    // square. This is the metric-compatibility question a substitution asks, and it is
    // `ShapedRun::occupies_the_same_width_as` — which is `AdvanceWidth::equals` — that answers it.
    let thousand = proportional_latin_face(1000).face();
    let two_thousand_and_forty_eight = proportional_latin_face(2048).face();
    let mut shaper = Shaper::new();
    let features = FeatureSet::new();
    let request = ShapingRequest::new("AB", TextScript::LATIN, twelve_points(), &features);

    let narrow = shaper.shape(&thousand, &request).expect("shapes");
    let wide = shaper
        .shape(&two_thousand_and_forty_eight, &request)
        .expect("shapes");

    // Three quarters of an em, expressed two ways.
    assert_eq!(narrow.advance().font_units, 750);
    assert_eq!(narrow.advance().units_per_em, 1000);
    assert_eq!(wide.advance().font_units, 1536);
    assert_eq!(wide.advance().units_per_em, 2048);
    assert_ne!(narrow.advance().font_units, wide.advance().font_units);

    assert!(
        narrow.occupies_the_same_width_as(&wide),
        "750/1000 and 1536/2048 are both three quarters of an em"
    );
    assert!(wide.occupies_the_same_width_as(&narrow));

    // And at a size, the two are the same number of points.
    assert_eq!(narrow.advance_in_points(), wide.advance_in_points());
    assert_eq!(narrow.advance_in_points(), 9.0);
}

#[test]
fn two_shaped_runs_of_genuinely_different_widths_are_not_equal() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();

    let kerned = shaper
        .shape(
            &face,
            &ShapingRequest::new("AV", TextScript::LATIN, twelve_points(), &features),
        )
        .expect("shapes");
    let unkerned = shaper
        .shape(
            &face,
            &ShapingRequest::new(
                "AV",
                TextScript::LATIN,
                twelve_points(),
                &FeatureSet::new().with(FontFeature::off(FeatureTag::KERNING)),
            ),
        )
        .expect("shapes");

    assert!(!kerned.occupies_the_same_width_as(&unkerned));
}

// ---------------------------------------------------------------------------------------------
// Sizes, and the uncached path
// ---------------------------------------------------------------------------------------------

#[test]
fn a_font_size_survives_every_way_a_document_writes_one() {
    assert_eq!(FontSize::from_points(12.0).in_points(), 12.0);
    // `w:sz` counts half-points; 24 half-points is 12 points.
    assert_eq!(FontSize::from_half_points(24), FontSize::from_points(12.0));
    // `a:rPr/@sz` counts hundredths; 1200 hundredths is 12 points.
    assert_eq!(
        FontSize::from_hundredths_of_a_point(1200),
        FontSize::from_points(12.0)
    );
    // Nothing a document can say produces a panic.
    assert_eq!(FontSize::from_points(f64::NAN).in_points(), 0.0);
    assert_eq!(FontSize::from_points(-1.0).in_points(), 0.0);
    assert_eq!(FontSize::from_points(f64::INFINITY), FontSize::MAXIMUM);
    assert_eq!(FontSize::from_half_points(u32::MAX), FontSize::MAXIMUM);
}

#[test]
fn the_uncached_path_produces_the_same_glyphs_as_the_cached_one() {
    let face = carlito();
    let mut shaper = Shaper::new();
    let features = TypographyOptions::registry_defaults().to_feature_set();
    let request = ShapingRequest::new("office", TextScript::LATIN, twelve_points(), &features);

    let cached = shaper.shape(&face, &request).expect("shapes");
    let direct = shape_uncached(&face, &request).expect("shapes");

    assert_eq!(cached.glyphs(), &direct[..]);
}

#[test]
fn the_bundled_font_directory_holds_the_faces_this_suite_reads() {
    // A guard against the suite silently testing nothing if the assets move: `bundled_face` would
    // panic rather than skip, but naming the file here says which files this suite depends on.
    let directory = bundled_font_directory();
    for name in ["Carlito-Regular.ttf", "LiberationSans-Regular.ttf"] {
        assert!(
            directory.join(name).is_file(),
            "{name} is missing from {}",
            directory.display()
        );
    }
}
