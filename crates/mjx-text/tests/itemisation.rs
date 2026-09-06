//! Script and font-fallback itemisation — where R02's fallback chain is actually exercised.
//!
//! The claim under test is that a paragraph is cut so that **every** item a shaper is handed is one
//! direction, one script and one face. A test that only counted items would pass for an
//! implementation that returned one item per character, so every case here asserts the item
//! *boundaries* and what each item resolved to.

mod support;

use mjx_text::{
    itemise, itemise_by_script, BidiAnalysis, FontRequest, FontResolver, ParagraphDirection,
    ResolutionTier, TextDirection, TextScript,
};

use support::bundled_font_directory;

/// `العربية`.
const ARABIC: &str = "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}";
/// `日本語`.
const JAPANESE: &str = "\u{65E5}\u{672C}\u{8A9E}";

/// A resolver with the bundled tier only, so the outcome does not depend on this machine's fonts.
fn bundled_only_resolver() -> FontResolver {
    FontResolver::builder()
        .with_bundled_font_directory(&bundled_font_directory())
        .expect("the bundled font directory is committed")
        .build()
}

// ---------------------------------------------------------------------------------------------
// Script itemisation
// ---------------------------------------------------------------------------------------------

#[test]
fn a_run_is_cut_where_the_script_changes() {
    let text = format!("Latin{ARABIC}Latin");
    let runs = itemise_by_script(&text);
    let scripts: Vec<TextScript> = runs.iter().map(|run| run.script).collect();
    assert_eq!(
        scripts,
        vec![TextScript::LATIN, TextScript::ARABIC, TextScript::LATIN]
    );
    assert_eq!(runs[0].range, 0..5);
    assert_eq!(runs[1].range, 5..19);
    assert_eq!(runs[2].range, 19..24);
    // The runs partition the text exactly.
    assert_eq!(runs.last().expect("at least one run").range.end, text.len());
}

#[test]
fn spaces_and_punctuation_extend_a_run_rather_than_splitting_it() {
    // If `Zyyy` characters split runs, `hello, world.` would be seven items and every kern across a
    // space would be lost.
    let runs = itemise_by_script("hello, world.");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].script, TextScript::LATIN);
    assert_eq!(runs[0].range, 0..13);
}

#[test]
fn a_combining_mark_takes_the_script_of_what_it_is_attached_to() {
    // `e` + `U+0301 COMBINING ACUTE ACCENT`. The mark is `Zinh`, and splitting before it would put
    // the accent in a different shaping call from the letter it sits on.
    let runs = itemise_by_script("e\u{0301}f");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].script, TextScript::LATIN);
}

#[test]
fn leading_undetermined_characters_join_the_run_that_follows_them() {
    let text = format!("  {ARABIC}");
    let runs = itemise_by_script(&text);
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].script, TextScript::ARABIC);
    assert_eq!(runs[0].range, 0..text.len(), "including the two spaces");
}

#[test]
fn text_with_no_determined_script_at_all_is_one_common_run() {
    let runs = itemise_by_script("123 456 ...");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].script, TextScript::COMMON);
    assert_eq!(runs[0].range, 0..11);
}

#[test]
fn an_empty_string_produces_no_runs() {
    assert!(itemise_by_script("").is_empty());
}

#[test]
fn japanese_is_cut_at_the_boundary_between_its_three_scripts() {
    // `日本語のテキスト` — Han, then hiragana, then katakana. Each is a different script and each
    // wants a different shaping engine's default.
    let text = "\u{65E5}\u{672C}\u{8A9E}\u{306E}\u{30C6}\u{30AD}\u{30B9}\u{30C8}";
    let runs = itemise_by_script(text);
    let scripts: Vec<TextScript> = runs.iter().map(|run| run.script).collect();
    assert_eq!(
        scripts,
        vec![TextScript::HAN, TextScript::HIRAGANA, TextScript::KATAKANA]
    );
}

#[test]
fn a_script_knows_whether_it_is_written_right_to_left() {
    assert!(TextScript::ARABIC.is_right_to_left());
    assert!(TextScript::HEBREW.is_right_to_left());
    assert!(!TextScript::LATIN.is_right_to_left());
    assert!(!TextScript::DEVANAGARI.is_right_to_left());
    assert!(!TextScript::HAN.is_right_to_left());
    // And a script this build has never heard of is answerable rather than a panic.
    let invented = TextScript::from_iso_15924_code(*b"Qaaa");
    assert_eq!(invented.code(), "Qaaa");
    assert!(!invented.is_right_to_left());
}

#[test]
fn a_range_that_is_not_on_a_character_boundary_produces_no_runs() {
    let text = format!("x{ARABIC}");
    // Byte 2 is inside the first Arabic character.
    assert!(mjx_text::itemise_range_by_script(&text, 2..4).is_empty());
    assert!(mjx_text::itemise_range_by_script(&text, 0..999).is_empty());
    #[allow(clippy::reversed_empty_ranges)]
    let backwards = 4..1;
    assert!(mjx_text::itemise_range_by_script(&text, backwards).is_empty());
}

// ---------------------------------------------------------------------------------------------
// Face itemisation
// ---------------------------------------------------------------------------------------------

#[test]
fn a_latin_paragraph_in_a_bundled_family_is_one_item() {
    let text = "The quick brown fox.";
    let mut resolver = bundled_only_resolver();
    let bidi = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
    let items = itemise(
        text,
        0..text.len(),
        &bidi,
        &mut resolver,
        &FontRequest::new("Carlito"),
    )
    .expect("Carlito is bundled");

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].range, 0..text.len());
    assert_eq!(items[0].script, TextScript::LATIN);
    assert_eq!(items[0].direction, TextDirection::LeftToRight);
    let face = items[0].face().expect("a face was resolved");
    assert_eq!(face.identity().family, "Carlito");
}

#[test]
fn a_paragraph_is_cut_where_the_direction_changes_and_each_half_is_shaped_its_own_way() {
    let text = format!("English {ARABIC} again");
    let mut resolver = bundled_only_resolver();
    let bidi = BidiAnalysis::resolve(&text, ParagraphDirection::LeftToRight);
    let items = itemise(
        &text,
        0..text.len(),
        &bidi,
        &mut resolver,
        &FontRequest::new("Carlito"),
    )
    .expect("resolves");

    // At least three items, and the Arabic one is right-to-left while its neighbours are not.
    let directions: Vec<TextDirection> = items.iter().map(|item| item.direction).collect();
    assert!(
        directions.contains(&TextDirection::RightToLeft),
        "the Arabic must be shaped right-to-left: {directions:?}"
    );
    assert!(directions.contains(&TextDirection::LeftToRight));

    // Every item is one script.
    for item in &items {
        let scripts: Vec<TextScript> = text[item.range.clone()]
            .chars()
            .map(TextScript::of_character)
            .filter(|script| !script.is_undetermined())
            .collect();
        assert!(
            scripts.iter().all(|script| *script == item.script),
            "item {:?} holds more than one script",
            &text[item.range.clone()]
        );
    }

    // And the items cover the paragraph exactly, in order.
    let mut cursor = 0;
    for item in &items {
        assert_eq!(item.range.start, cursor);
        cursor = item.range.end;
    }
    assert_eq!(cursor, text.len());
}

#[test]
fn a_character_no_bundled_face_covers_is_still_an_item() {
    // Nothing in `assets/fonts/` has a Japanese glyph, so the resolver has to answer with a
    // substitution, a fetch plan or nothing — and whichever it is, the characters must still appear
    // in the itemisation. A renderer that dropped them would silently lose text.
    let text = format!("before {JAPANESE} after");
    let mut resolver = bundled_only_resolver();
    let bidi = BidiAnalysis::resolve(&text, ParagraphDirection::LeftToRight);
    let items = itemise(
        &text,
        0..text.len(),
        &bidi,
        &mut resolver,
        &FontRequest::new("Carlito"),
    )
    .expect("resolves");

    let mut cursor = 0;
    for item in &items {
        assert_eq!(item.range.start, cursor, "items must be adjacent");
        assert!(item.range.end > item.range.start, "and non-empty");
        cursor = item.range.end;
    }
    assert_eq!(cursor, text.len(), "every byte is in an item");

    let japanese_start = text.find(JAPANESE).expect("the Japanese is in the text");
    let covering = items
        .iter()
        .find(|item| item.range.contains(&japanese_start))
        .expect("the Japanese is in some item");
    assert_eq!(covering.script, TextScript::HAN);
}

#[test]
fn a_missing_family_resolves_through_the_substitution_table_and_is_recorded() {
    // `Calibri` is not bundled; `Carlito` is, and the substitution table pairs them. This is the
    // path itemisation drives, and the manifest is where the record ends up.
    let text = "Calibri text";
    let mut resolver = bundled_only_resolver();
    let bidi = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
    let items = itemise(
        text,
        0..text.len(),
        &bidi,
        &mut resolver,
        &FontRequest::new("Calibri"),
    )
    .expect("resolves");

    assert_eq!(items.len(), 1);
    let face = items[0].face().expect("a face was resolved");
    assert_eq!(face.identity().family, "Carlito");
    if let mjx_text::FontResolution::Resolved(resolved) = &items[0].font {
        assert_eq!(resolved.tier, ResolutionTier::Bundled);
        assert!(resolved.is_substitution());
    } else {
        panic!("Calibri must resolve to the bundled Carlito");
    }
    assert!(
        !resolver.manifest().is_empty(),
        "the substitution must be recorded"
    );
}

#[test]
fn an_out_of_range_or_misaligned_range_produces_no_items_rather_than_a_panic() {
    let text = format!("x{ARABIC}");
    let mut resolver = bundled_only_resolver();
    let bidi = BidiAnalysis::resolve(&text, ParagraphDirection::LeftToRight);
    let request = FontRequest::new("Carlito");

    // The third of these is backwards on purpose: a caller with a bug hands one in, and no items
    // rather than a panic is the promise.
    #[allow(clippy::reversed_empty_ranges)]
    let hostile = [2..4, 0..999, 5..1, 0..0];
    for range in hostile {
        let items = itemise(&text, range, &bidi, &mut resolver, &request).expect("no panic");
        assert!(items.is_empty());
    }
}
