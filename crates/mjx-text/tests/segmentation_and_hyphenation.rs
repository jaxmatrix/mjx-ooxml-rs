//! Grapheme clusters, word boundaries and hyphenation.
//!
//! Every case here is one where the naive answer — a `char`, a `split_whitespace`, a hyphen already
//! in the string — is different from the right one.

use mjx_text::{
    grapheme_cluster_boundaries, grapheme_cluster_count, next_grapheme_boundary,
    previous_grapheme_boundary, word_at, words, HyphenationPatterns, Hyphenator, NoHyphenation,
    PatternHyphenator, SoftHyphenHyphenator,
};

// ---------------------------------------------------------------------------------------------
// Grapheme clusters
// ---------------------------------------------------------------------------------------------

#[test]
fn a_combining_accent_is_part_of_the_cluster_it_sits_on_not_a_place_the_caret_can_be() {
    // `e` + `U+0301 COMBINING ACUTE ACCENT` + `f`. Three `char`s, two clusters.
    let text = "e\u{0301}f";
    assert_eq!(text.chars().count(), 3);
    assert_eq!(grapheme_cluster_count(text), 2);
    assert_eq!(grapheme_cluster_boundaries(text), vec![0, 3, 4]);

    // The caret steps over the accent, not into it.
    assert_eq!(next_grapheme_boundary(text, 0), Some(3));
    assert_eq!(previous_grapheme_boundary(text, 3), Some(0));
    // Byte 1 is a character boundary but not a cluster boundary, and asking from there still moves
    // by whole clusters rather than landing between `e` and its accent.
    assert_eq!(next_grapheme_boundary(text, 1), Some(3));
}

#[test]
fn a_flag_built_from_two_regional_indicators_is_one_cluster() {
    // `U+1F1EF U+1F1F5` — the Japanese flag. Two `char`s, eight bytes, one place the caret can be
    // on either side of.
    let text = "\u{1F1EF}\u{1F1F5}";
    assert_eq!(text.chars().count(), 2);
    assert_eq!(text.len(), 8);
    assert_eq!(grapheme_cluster_count(text), 1);
    assert_eq!(grapheme_cluster_boundaries(text), vec![0, 8]);
    assert_eq!(next_grapheme_boundary(text, 0), Some(8));
}

#[test]
fn a_devanagari_syllable_with_a_matra_is_one_cluster() {
    // `कि` — ka plus the vowel sign i. Two `char`s, one cluster, and the same unit the shaper
    // reports because the buffer is asked for `MonotoneGraphemes`.
    let text = "\u{0915}\u{093F}";
    assert_eq!(text.chars().count(), 2);
    assert_eq!(grapheme_cluster_count(text), 1);
}

#[test]
fn the_caret_stops_at_the_ends_and_answers_nothing_beyond_them() {
    let text = "abc";
    assert_eq!(previous_grapheme_boundary(text, 0), None);
    assert_eq!(next_grapheme_boundary(text, 3), None);
    assert_eq!(next_grapheme_boundary(text, 99), None);
    assert_eq!(previous_grapheme_boundary(text, 99), None);
    assert_eq!(grapheme_cluster_boundaries(""), vec![0]);
}

// ---------------------------------------------------------------------------------------------
// Word boundaries
// ---------------------------------------------------------------------------------------------

#[test]
fn a_word_is_not_what_splitting_on_whitespace_gives() {
    let text = "don't stop; 3.14 is pi";
    let found: Vec<&str> = words(text).map(|(_, word)| word).collect();
    // `don't` stays together, `3.14` stays together, and the semicolon is not part of `stop`.
    assert_eq!(found, vec!["don't", "stop", "3.14", "is", "pi"]);

    let naive: Vec<&str> = text.split_whitespace().collect();
    assert_eq!(naive, vec!["don't", "stop;", "3.14", "is", "pi"]);
    assert_ne!(found, naive);
}

#[test]
fn a_word_boundary_sits_between_latin_and_an_adjacent_ideograph_with_no_space() {
    let text = "word\u{65E5}\u{672C}\u{8A9E}";
    let found: Vec<&str> = words(text).map(|(_, word)| word).collect();
    assert!(found.len() > 1, "one word would be wrong: {found:?}");
    assert_eq!(found[0], "word");
}

#[test]
fn the_word_a_double_click_selects_is_found_by_offset() {
    let text = "select this word";
    assert_eq!(word_at(text, 9), Some(7..11));
    assert_eq!(&text[7..11], "this");
    // A caret resting immediately after a word belongs to that word, which is what a double-click
    // at the right-hand edge of `select` should select.
    assert_eq!(word_at(text, 6), Some(0..6));
    // Genuinely between two words, there is no word.
    assert_eq!(word_at("select  this", 7), None);
    // Past the end, and off a character boundary, both answer nothing rather than panicking.
    assert_eq!(word_at(text, 999), None);
    assert_eq!(word_at("\u{65E5}", 1), None);
}

// ---------------------------------------------------------------------------------------------
// Hyphenation
// ---------------------------------------------------------------------------------------------

#[test]
fn the_default_hyphenator_never_splits_anything() {
    assert!(NoHyphenation
        .hyphenation_points_of("hyphenation")
        .is_empty());
}

#[test]
fn an_author_placed_soft_hyphen_is_a_hyphenation_point() {
    // `hy·phen·ation` with two soft hyphens, at bytes 2 and 8.
    let word = "hy\u{00AD}phen\u{00AD}ation";
    assert_eq!(SoftHyphenHyphenator.hyphenation_points_of(word), vec![2, 8]);

    // A soft hyphen at either end is not a point: a break there puts nothing on one of the lines.
    assert!(SoftHyphenHyphenator
        .hyphenation_points_of("\u{00AD}word")
        .is_empty());
    assert!(SoftHyphenHyphenator
        .hyphenation_points_of("word\u{00AD}")
        .is_empty());
    // A word with none is not split.
    assert!(SoftHyphenHyphenator
        .hyphenation_points_of("hyphenation")
        .is_empty());
}

#[test]
fn liangs_algorithm_finds_a_point_the_word_does_not_contain() {
    // The point of pattern hyphenation: the break is *not* in the text. These four patterns are
    // Liang's own worked example for `hyphenation`, and the odd totals they produce are what put a
    // break after `hy` and after `phen`.
    let mut patterns = HyphenationPatterns::new();
    patterns.add_pattern("hy3ph");
    patterns.add_pattern("he2n");
    patterns.add_pattern("hena4");
    patterns.add_pattern("hen5at");
    let hyphenator = PatternHyphenator::new(patterns);

    let points = hyphenator.hyphenation_points_of("hyphenation");
    assert_eq!(points, vec![2, 6], "hy-phen-ation");
    assert_eq!(&"hyphenation"[..2], "hy");
    assert_eq!(&"hyphenation"[2..6], "phen");

    // Nothing in the word is a hyphen, so a naive "split at the hyphens" answer is empty.
    assert!(!"hyphenation".contains('-'));
    assert!(SoftHyphenHyphenator
        .hyphenation_points_of("hyphenation")
        .is_empty());
}

#[test]
fn an_even_score_is_not_a_hyphenation_point() {
    // The whole of Liang's rule is the parity of the total. A pattern scoring 2 must produce
    // nothing, and one scoring 3 at the same position must produce a point — which is the
    // difference an implementation that ignored the parity would lose.
    let mut even = HyphenationPatterns::new();
    even.add_pattern("hy2ph");
    assert!(PatternHyphenator::new(even)
        .hyphenation_points_of("hyphen")
        .is_empty());

    let mut odd = HyphenationPatterns::new();
    odd.add_pattern("hy3ph");
    assert_eq!(
        PatternHyphenator::new(odd).hyphenation_points_of("hyphen"),
        vec![2]
    );
}

#[test]
fn the_minima_keep_a_break_away_from_the_ends_of_a_word() {
    let mut patterns = HyphenationPatterns::new();
    // A pattern that would score a break after the first letter and before the last.
    patterns.add_pattern("a1b");
    patterns.add_pattern("y1z");
    let default = PatternHyphenator::new(patterns.clone());
    // `abcdxyz`: the points would be at 1 and 6; two before and three after excludes both.
    assert!(default.hyphenation_points_of("abcdxyz").is_empty());

    let permissive = PatternHyphenator::new(patterns).with_minima(1, 1);
    assert_eq!(permissive.hyphenation_points_of("abcdxyz"), vec![1, 6]);
}

#[test]
fn an_exception_overrides_the_patterns() {
    let mut patterns = HyphenationPatterns::new();
    patterns.add_pattern("pr3es");
    patterns.add_exception("pre-sent");
    let hyphenator = PatternHyphenator::new(patterns).with_minima(1, 1);
    assert_eq!(hyphenator.hyphenation_points_of("present"), vec![3]);
    assert_eq!(&"present"[..3], "pre");
}

#[test]
fn a_word_is_hyphenated_whatever_its_case() {
    let mut patterns = HyphenationPatterns::new();
    patterns.add_pattern("hy3ph");
    let hyphenator = PatternHyphenator::new(patterns);
    assert_eq!(hyphenator.hyphenation_points_of("Hyphen"), vec![2]);
    assert_eq!(hyphenator.hyphenation_points_of("HYPHEN"), vec![2]);
}

#[test]
fn the_tex_pattern_format_is_read_including_its_comments_and_anchors() {
    let source = "% a comment line\n.hy3ph he2n  % trailing comment\nhena4\nhen5at\n";
    let patterns = HyphenationPatterns::from_tex_patterns(source);
    assert_eq!(patterns.len(), 4);
    let hyphenator = PatternHyphenator::new(patterns);
    // `.hy3ph` is anchored to the start of the word, so it still fires on `hyphenation`.
    assert_eq!(hyphenator.hyphenation_points_of("hyphenation"), vec![2, 6]);
}

#[test]
fn an_empty_pattern_set_never_splits_anything() {
    // The honest answer for a language whose patterns have not been supplied: no points, rather
    // than guessed ones.
    let hyphenator = PatternHyphenator::new(HyphenationPatterns::new());
    assert!(hyphenator.patterns().is_empty());
    assert!(hyphenator.hyphenation_points_of("hyphenation").is_empty());
    assert!(hyphenator.hyphenation_points_of("").is_empty());
}

#[test]
fn hyphenating_text_that_is_not_a_word_is_not_a_panic() {
    let mut patterns = HyphenationPatterns::new();
    patterns.add_pattern("a1b");
    patterns.add_pattern(""); // no letters at all
    patterns.add_pattern("123"); // digits only
    let hyphenator = PatternHyphenator::new(patterns).with_minima(1, 1);
    for word in [
        "",
        " ",
        "\u{0915}\u{093F}",
        "\u{1F1EF}\u{1F1F5}",
        "\u{FFFD}",
        "İstanbul",
        "\u{0378}",
    ] {
        let _ = hyphenator.hyphenation_points_of(word);
        let _ = SoftHyphenHyphenator.hyphenation_points_of(word);
    }
}

#[test]
fn a_hyphenator_appends_rather_than_clearing_so_a_caller_can_accumulate() {
    let mut points = vec![99];
    SoftHyphenHyphenator.hyphenation_points("hy\u{00AD}phen", &mut points);
    assert_eq!(points, vec![99, 2]);
}
