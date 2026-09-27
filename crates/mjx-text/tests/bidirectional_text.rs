//! UAX #9, asserted where visual order and logical order genuinely disagree.
//!
//! A bidirectional test that only checks a right-to-left string comes back reversed proves nothing:
//! so does `text.chars().rev()`. The cases here are the ones where the *order of the runs* is not
//! any simple function of the string — a number inside an Arabic phrase reads left to right while
//! the phrase around it reads right to left, so the three pieces come out in an order none of
//! logical, reversed-logical or per-character produces.

mod support;

use mjx_text::{BidiAnalysis, BidiLevel, DeclaredRunDirection, ParagraphDirection, TextDirection};

/// `العربية` — the Arabic word for "Arabic".
const ARABIC: &str = "\u{0627}\u{0644}\u{0639}\u{0631}\u{0628}\u{064A}\u{0629}";
/// `עברית` — the Hebrew word for "Hebrew".
const HEBREW: &str = "\u{05E2}\u{05D1}\u{05E8}\u{05D9}\u{05EA}";

fn visual_text<'a>(text: &'a str, analysis: &BidiAnalysis) -> Vec<&'a str> {
    analysis
        .visual_runs(0..text.len())
        .into_iter()
        .map(|run| &text[run.range])
        .collect()
}

#[test]
fn a_number_inside_an_arabic_phrase_reads_left_to_right_inside_a_phrase_that_does_not() {
    // "abc العربية 123 def": Latin, Arabic, a number, Latin. Byte offsets, since the Arabic is two
    // bytes per character:
    //   0..4   "abc "
    //   4..18  the seven Arabic characters
    //   18..19 " "
    //   19..22 "123"
    //   22..26 " def"
    let text = format!("abc {ARABIC} 123 def");
    assert_eq!(text.len(), 26);

    let analysis = BidiAnalysis::resolve(&text, ParagraphDirection::LeftToRight);
    assert_eq!(analysis.base_direction(), TextDirection::LeftToRight);

    // Three levels: 0 for the Latin, 1 for the Arabic and the space inside it, 2 for the number.
    assert_eq!(analysis.level_at(0), BidiLevel(0), "`a`");
    assert_eq!(
        analysis.level_at(4),
        BidiLevel(1),
        "the first Arabic letter"
    );
    assert_eq!(analysis.level_at(18), BidiLevel(1), "the space after it");
    assert_eq!(analysis.level_at(19), BidiLevel(2), "`1`");
    assert_eq!(
        analysis.level_at(22),
        BidiLevel(0),
        "the space before `def`"
    );

    // Logical order is the order of the bytes.
    let logical: Vec<&str> = analysis
        .logical_runs(0..text.len())
        .into_iter()
        .map(|run| &text[run.range])
        .collect();
    assert_eq!(
        logical,
        vec!["abc ", &format!("{ARABIC} ")[..], "123", " def"]
    );

    // Visual order is not. The number comes out *before* the Arabic, because the Arabic run is
    // reversed around it and the number is not reversed within itself.
    assert_eq!(
        visual_text(&text, &analysis),
        vec!["abc ", "123", &format!("{ARABIC} ")[..], " def"],
        "the number is drawn to the left of the Arabic it sits inside"
    );

    // Position by position, so the assertion is not a vector comparison that could pass by luck.
    let visual = analysis.visual_runs(0..text.len());
    assert_eq!(visual.len(), 4);
    assert_eq!(visual[0].range, 0..4);
    assert_eq!(visual[0].direction(), TextDirection::LeftToRight);
    assert_eq!(visual[1].range, 19..22);
    assert_eq!(visual[1].direction(), TextDirection::LeftToRight);
    assert_eq!(visual[2].range, 4..19);
    assert_eq!(visual[2].direction(), TextDirection::RightToLeft);
    assert_eq!(visual[3].range, 22..26);
    assert_eq!(visual[3].direction(), TextDirection::LeftToRight);

    // And the visual order is not the logical order, nor its reverse: the third run is neither
    // first nor last.
    let visual_starts: Vec<usize> = visual.iter().map(|run| run.range.start).collect();
    assert_eq!(visual_starts, vec![0, 19, 4, 22]);
    assert_ne!(visual_starts, vec![0, 4, 19, 22], "not logical order");
    assert_ne!(visual_starts, vec![22, 19, 4, 0], "not reversed order");
}

#[test]
fn the_same_text_in_a_right_to_left_paragraph_comes_out_in_a_different_order_again() {
    let text = format!("abc {ARABIC} 123 def");
    let analysis = BidiAnalysis::resolve(&text, ParagraphDirection::RightToLeft);
    assert_eq!(analysis.base_direction(), TextDirection::RightToLeft);

    // The base level is now 1, so the Latin is raised to 2 and the paragraph reads the other way.
    assert_eq!(analysis.level_at(0), BidiLevel(2), "`a`");
    assert_eq!(
        analysis.level_at(4),
        BidiLevel(1),
        "the first Arabic letter"
    );

    // Five runs now, not four: in a right-to-left paragraph the space at byte 3 is a neutral
    // between an L and an R, so UAX #9 rule N2 gives it the *paragraph's* direction and it leaves
    // the `abc` run to join the Arabic. The same happens to the space at byte 22.
    //   0..3   `abc`      level 2
    //   3..19  ` العربية ` level 1
    //   19..22 `123`      level 2
    //   22..23 ` `        level 1
    //   23..26 `def`      level 2
    let visual = analysis.visual_runs(0..text.len());
    let visual_starts: Vec<usize> = visual.iter().map(|run| run.range.start).collect();
    assert_eq!(
        visual_starts,
        vec![23, 22, 19, 3, 0],
        "the trailing Latin is now leftmost"
    );

    // Nothing about the *text* changed — only the declaration — which is the whole reason the base
    // direction is taken from the document rather than inferred.
    let inferred = BidiAnalysis::resolve(&text, ParagraphDirection::LeftToRight);
    assert_ne!(
        inferred
            .visual_runs(0..text.len())
            .into_iter()
            .map(|run| run.range.start)
            .collect::<Vec<_>>(),
        visual_starts
    );
}

#[test]
fn a_right_to_left_paragraph_that_opens_with_latin_is_still_right_to_left() {
    // This is the case UAX #9's P2/P3 gets wrong for Word: the first strong character is Latin, so
    // inferring would answer left-to-right, and `w:bidi` says otherwise.
    let text = format!("Word {HEBREW}");

    let inferred = BidiAnalysis::resolve(&text, ParagraphDirection::FromFirstStrongCharacter);
    assert_eq!(
        inferred.base_direction(),
        TextDirection::LeftToRight,
        "P2/P3 takes the direction of the first strong character, which is `W`"
    );

    let declared = BidiAnalysis::resolve(&text, ParagraphDirection::RightToLeft);
    assert_eq!(
        declared.base_direction(),
        TextDirection::RightToLeft,
        "`w:bidi` is obeyed, not overruled by the content"
    );

    // And the difference is visible: the Hebrew leads the line in the declared paragraph. It leads
    // it from byte 4 rather than 5, because in a right-to-left paragraph the space between `Word`
    // and the Hebrew is a neutral that rule N2 resolves to the paragraph's own direction, so it
    // belongs to the Hebrew run.
    let inferred_first = inferred.visual_runs(0..text.len())[0].range.start;
    let declared_first = declared.visual_runs(0..text.len())[0].range.start;
    assert_eq!(inferred_first, 0);
    assert_eq!(declared_first, 4);
    assert_eq!(&text[4..], format!(" {HEBREW}"));
}

#[test]
fn a_paragraph_with_no_strong_character_falls_back_to_left_to_right() {
    let analysis = BidiAnalysis::resolve("123 456", ParagraphDirection::FromFirstStrongCharacter);
    assert_eq!(analysis.base_direction(), TextDirection::LeftToRight);
    assert_eq!(analysis.level_at(0), BidiLevel(0));
}

#[test]
fn a_run_the_document_declared_right_to_left_raises_its_level() {
    // `w:rtl` on a run of Latin. In a left-to-right paragraph the Latin would sit at level 0; the
    // declaration puts it at level 2 — an *embedding*, so the run is right-to-left and its Latin is
    // still resolved left-to-right inside it.
    let text = "one two three";
    let plain = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
    assert_eq!(plain.level_at(4), BidiLevel(0), "`two`, undeclared");

    let declared = BidiAnalysis::resolve_with_run_directions(
        text,
        ParagraphDirection::LeftToRight,
        &[DeclaredRunDirection {
            range: 4..7,
            direction: TextDirection::RightToLeft,
        }],
    );
    assert_eq!(declared.level_at(0), BidiLevel(0), "`one`");
    assert_eq!(
        declared.level_at(4),
        BidiLevel(2),
        "`two`, declared `w:rtl`"
    );
    assert_eq!(declared.level_at(8), BidiLevel(0), "`three`");

    // Level 2 is even, so the Latin inside the declared run still reads left to right — which is
    // what makes this an embedding rather than an override.
    assert_eq!(declared.level_at(4).direction(), TextDirection::LeftToRight);

    // And the runs did move: the undeclared text is one run, the declared text is three.
    assert_eq!(plain.logical_runs(0..text.len()).len(), 1);
    assert_eq!(declared.logical_runs(0..text.len()).len(), 3);
}

#[test]
fn a_number_inside_a_declared_right_to_left_run_still_reads_left_to_right() {
    // The distinction an *override* would lose: the number keeps its own direction inside a run the
    // document declared right-to-left, because an embedding resolves its contents and an override
    // would not.
    //
    // The declared run is `2026 עברית`, and the number is what the declaration changes. Without it,
    // UAX #9 rule W7 sees a preceding strong `L` (`total`) and treats the digits as `L`, so they sit
    // at the paragraph's own level 0. Inside a right-to-left embedding there is no preceding strong
    // `L`, so the digits stay `EN` and are raised to level 2 by rule I1.
    let text = format!("total 2026 {HEBREW} end");
    let run_start = "total ".len();
    let run_end = text.len() - " end".len();
    let number_start = text.find("2026").expect("the number is in the text");
    let hebrew_start = text.find(HEBREW).expect("the Hebrew is in the text");

    let undeclared = BidiAnalysis::resolve(&text, ParagraphDirection::LeftToRight);
    assert_eq!(
        undeclared.level_at(number_start),
        BidiLevel(0),
        "with no declaration the digits follow the Latin before them"
    );

    let declared = BidiAnalysis::resolve_with_run_directions(
        &text,
        ParagraphDirection::LeftToRight,
        &[DeclaredRunDirection {
            range: run_start..run_end,
            direction: TextDirection::RightToLeft,
        }],
    );
    assert_eq!(
        declared.level_at(hebrew_start),
        BidiLevel(1),
        "the embedding puts the run at the next odd level above the paragraph's"
    );
    assert_eq!(
        declared.level_at(number_start),
        BidiLevel(2),
        "and the number is raised once more, so it still reads left to right"
    );
    assert_eq!(
        declared.level_at(number_start).direction(),
        TextDirection::LeftToRight
    );

    // The declaration moved the number, which is what proves the embedding controls reached the
    // algorithm at all.
    assert_ne!(
        declared.level_at(number_start),
        undeclared.level_at(number_start)
    );

    // Visually, the digits are now drawn to the *right* of the Hebrew rather than the left of it.
    let order = |analysis: &BidiAnalysis| -> Vec<usize> {
        analysis
            .visual_runs(0..text.len())
            .into_iter()
            .map(|run| run.range.start)
            .collect()
    };
    assert_ne!(order(&declared), order(&undeclared));
}

#[test]
fn a_declared_run_whose_offsets_do_not_land_on_characters_is_skipped_not_panicked_on() {
    let text = format!("x{ARABIC}y");
    // Byte 2 is inside the first Arabic character.
    let analysis = BidiAnalysis::resolve_with_run_directions(
        &text,
        ParagraphDirection::LeftToRight,
        &[DeclaredRunDirection {
            range: 2..4,
            direction: TextDirection::RightToLeft,
        }],
    );
    let plain = BidiAnalysis::resolve(&text, ParagraphDirection::LeftToRight);
    assert_eq!(analysis.len(), plain.len());
    assert_eq!(analysis.level_at(1), plain.level_at(1));
}

#[test]
fn overlapping_and_out_of_order_declarations_are_skipped_rather_than_corrupting_the_levels() {
    let text = "aaaa bbbb cccc";
    let analysis = BidiAnalysis::resolve_with_run_directions(
        text,
        ParagraphDirection::LeftToRight,
        &[
            DeclaredRunDirection {
                range: 5..9,
                direction: TextDirection::RightToLeft,
            },
            // Starts before the previous one ended.
            DeclaredRunDirection {
                range: 7..12,
                direction: TextDirection::RightToLeft,
            },
            // Empty.
            DeclaredRunDirection {
                range: 12..12,
                direction: TextDirection::RightToLeft,
            },
            // Past the end.
            DeclaredRunDirection {
                range: 13..99,
                direction: TextDirection::RightToLeft,
            },
        ],
    );
    assert_eq!(analysis.len(), text.len());
    assert_eq!(analysis.level_at(5), BidiLevel(2), "the usable declaration");
    assert_eq!(analysis.level_at(10), BidiLevel(0), "the rest is untouched");
}

#[test]
fn an_empty_paragraph_resolves_to_nothing_without_panicking() {
    let analysis = BidiAnalysis::resolve("", ParagraphDirection::RightToLeft);
    assert!(analysis.is_empty());
    assert_eq!(analysis.len(), 0);
    assert!(analysis.logical_runs(0..0).is_empty());
    assert!(analysis.visual_runs(0..0).is_empty());
    // An offset past the end answers the base level rather than panicking.
    assert_eq!(analysis.level_at(99), BidiLevel::RIGHT_TO_LEFT);
}

#[test]
fn a_line_range_that_is_out_of_bounds_is_clamped_rather_than_panicked_on() {
    let text = "hello";
    let analysis = BidiAnalysis::resolve(text, ParagraphDirection::LeftToRight);
    assert!(analysis.visual_runs(99..200).is_empty());
    assert_eq!(analysis.visual_runs(0..999).len(), 1);
    // A backwards range is empty, not a panic. Clippy is right that the literal is an empty
    // range; that is exactly the input this asserts is survivable.
    #[allow(clippy::reversed_empty_ranges)]
    let backwards = 4..1;
    assert!(analysis.logical_runs(backwards).is_empty());
}

#[test]
fn a_purely_right_to_left_line_is_one_run_and_is_not_reordered() {
    let analysis = BidiAnalysis::resolve(HEBREW, ParagraphDirection::RightToLeft);
    let visual = analysis.visual_runs(0..HEBREW.len());
    assert_eq!(visual.len(), 1);
    assert_eq!(visual[0].range, 0..HEBREW.len());
    assert_eq!(visual[0].direction(), TextDirection::RightToLeft);
}
