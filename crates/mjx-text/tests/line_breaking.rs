//! Line breaking, asserted where Office and UAX #14 give different answers.
//!
//! A test that only checks a break after a space passes for `str::split_whitespace`. The cases here
//! are the ones where the East Asian layer is the *only* thing that produces the right answer, so a
//! build with `east_asian_rules` off gives a different, checkable, wrong result.
//!
//! # The discriminating case, and why it is this one
//!
//! UAX #14's classes `CL`, `CP`, `EX`, `IS` and `NS` were written with *kinsoku* in mind, so most of
//! JIS X 4051's prohibited-at-line-start set is already handled by rules LB13 and LB19. Every
//! character in Word's Japanese set was checked against `unicode-linebreak` in every combination of
//! neighbours, and there is exactly one shape where the two disagree: **rule LB18, "break after
//! spaces", is unconditional**, so UAX #14 offers a break between a space and a following postfix
//! sign or small kana. `25 %` is the everyday form of it — Word will not begin a line with `%`, and
//! UAX #14 alone will.

mod support;

use mjx_text::{
    break_opportunities, BreakKind, KinsokuRules, LineBreakKind, LineBreakOptions, LineBreaker,
};

/// Every allowed opportunity, as byte offsets.
fn allowed(text: &str, options: &LineBreakOptions) -> Vec<usize> {
    break_opportunities(text, options)
        .into_iter()
        .filter(|opportunity| opportunity.kind == BreakKind::Allowed)
        .map(|opportunity| opportunity.at)
        .collect()
}

/// A width function of one unit per character, so a measure is a character count.
fn one_unit_per_character(text: &'_ str) -> impl FnMut(std::ops::Range<usize>) -> f64 + '_ {
    move |range| {
        text.get(range)
            .map_or(0.0, |slice| slice.chars().count() as f64)
    }
}

// ---------------------------------------------------------------------------------------------
// The kinsoku discriminator
// ---------------------------------------------------------------------------------------------

#[test]
fn uax_fourteen_alone_would_begin_a_line_with_a_percent_sign_and_office_would_not() {
    // `温度は 25 %です` — "the temperature is 25 %". Byte offsets:
    //   0..9   温度は     (three characters, three bytes each)
    //   9      the space
    //   10..12 `25`
    //   12     the space
    //   13     `%`
    //   14..20 です
    let text = "温度は 25 %です";
    assert_eq!(text.len(), 20);
    assert_eq!(&text[13..14], "%");

    let unicode_only = allowed(text, &LineBreakOptions::unicode_only());
    let office = allowed(text, &LineBreakOptions::japanese_typesetting());

    // UAX #14 offers a break at byte 13, which would put `%` at the head of the next line.
    assert!(
        unicode_only.contains(&13),
        "UAX #14 alone offers a break before the percent sign: {unicode_only:?}"
    );
    // Office's kinsoku does not.
    assert!(
        !office.contains(&13),
        "the East Asian layer must remove the opportunity before `%`: {office:?}"
    );

    // And nothing else was removed, so the layer is discriminating rather than blunt.
    let removed: Vec<usize> = unicode_only
        .iter()
        .copied()
        .filter(|at| !office.contains(at))
        .collect();
    assert_eq!(removed, vec![13]);
    assert_eq!(unicode_only, vec![3, 6, 10, 13, 14, 17]);
    assert_eq!(office, vec![3, 6, 10, 14, 17]);
}

#[test]
fn the_line_office_produces_and_the_line_uax_fourteen_alone_would_are_different_lines() {
    let text = "温度は 25 %です";

    // The text is `温`,`度`,`は`,` `,`2`,`5`,` `,`%`,`で`,`す` — ten characters. The opportunities
    // are at bytes 3, 6, 10, 13, 14 and 17, which end lines of 1, 2, 4, 7, 8 and 9 characters. A
    // measure of seven is therefore the width at which the two rule sets choose differently: UAX #14
    // takes byte 13, and kinsoku, having no opportunity there, has to fall back to byte 10.
    let measure = 7.0;

    let plain = LineBreaker::new(text, LineBreakOptions::unicode_only());
    let plain_line = plain.next_line(0, measure, &mut one_unit_per_character(text));
    assert_eq!(plain_line.end, 13, "UAX #14 alone ends the line before `%`");
    assert_eq!(plain_line.kind, LineBreakKind::Fitted);
    assert_eq!(&text[..plain_line.end], "温度は 25 ");
    assert_eq!(
        text[plain_line.end..].chars().next(),
        Some('%'),
        "…which begins the next line with a percent sign"
    );

    let office = LineBreaker::new(text, LineBreakOptions::japanese_typesetting());
    let office_line = office.next_line(0, measure, &mut one_unit_per_character(text));
    assert_eq!(
        office_line.end, 10,
        "kinsoku moves the break back, so the number goes down with the sign"
    );
    assert_eq!(office_line.kind, LineBreakKind::Fitted);
    assert_eq!(&text[..office_line.end], "温度は ");
    assert_eq!(text[office_line.end..].chars().next(), Some('2'));

    // The two break in different places. That is the whole claim.
    assert_ne!(plain_line.end, office_line.end);
}

#[test]
fn a_small_kana_may_not_begin_a_line_either() {
    // The other half of the same LB18 shape: a small kana after a space.
    let text = "テスト ぁあ";
    let at = "テスト ".len();
    assert_eq!(&text[at..at + 3], "ぁ");

    assert!(allowed(text, &LineBreakOptions::unicode_only()).contains(&at));
    assert!(!allowed(text, &LineBreakOptions::japanese_typesetting()).contains(&at));
}

#[test]
fn a_line_may_not_end_with_an_opening_bracket() {
    // 行末禁則. UAX #14 rule LB14 already covers this one, so the two rule sets agree — which is
    // worth asserting, because a kinsoku layer that *added* opportunities would be a defect.
    let text = "日本 「東京」";
    let after_bracket = "日本 「".len();
    assert!(!allowed(text, &LineBreakOptions::unicode_only()).contains(&after_bracket));
    assert!(!allowed(text, &LineBreakOptions::japanese_typesetting()).contains(&after_bracket));
}

#[test]
fn a_mandatory_break_survives_kinsoku() {
    // A line feed ends the line whatever follows it. `%` is prohibited at a line start, and this
    // must not remove the hard break.
    let text = "one\n%two";
    let opportunities = break_opportunities(text, &LineBreakOptions::japanese_typesetting());
    let mandatory: Vec<usize> = opportunities
        .iter()
        .filter(|opportunity| opportunity.kind == BreakKind::Mandatory)
        .map(|opportunity| opportunity.at)
        .collect();
    assert_eq!(mandatory, vec![4, 8]);

    let breaker = LineBreaker::new(text, LineBreakOptions::japanese_typesetting());
    let line = breaker.next_line(0, 1000.0, &mut one_unit_per_character(text));
    assert_eq!(line.end, 4);
    assert_eq!(line.kind, LineBreakKind::Mandatory);
}

#[test]
fn switching_the_east_asian_rules_off_is_exactly_uax_fourteen() {
    for text in [
        "温度は 25 %です",
        "hello world",
        "日本、東京",
        "テスト ぁあ",
        "well-known thing",
    ] {
        let ours = allowed(text, &LineBreakOptions::unicode_only());
        let theirs: Vec<usize> = unicode_linebreak::linebreaks(text)
            .filter(|(_, kind)| *kind == unicode_linebreak::BreakOpportunity::Allowed)
            .map(|(at, _)| at)
            .collect();
        assert_eq!(ours, theirs, "for {text:?}");
    }
}

// ---------------------------------------------------------------------------------------------
// Hanging punctuation — `w:overflowPunct`
// ---------------------------------------------------------------------------------------------

#[test]
fn a_trailing_ideographic_full_stop_hangs_past_the_measure() {
    // Six characters, of which the last is a full stop. With a measure of five, the line fits only
    // if the full stop hangs.
    let text = "日本語です。次の行";
    let full_stop = "日本語です".len();
    assert_eq!(&text[full_stop..full_stop + 3], "。");
    let after = full_stop + 3;

    let hanging = LineBreaker::new(text, LineBreakOptions::japanese_typesetting());
    let tail = hanging.hanging_tail(0..after);
    assert_eq!(tail, full_stop..after, "the full stop is the hanging tail");

    let line = hanging.next_line(0, 5.0, &mut one_unit_per_character(text));
    assert_eq!(
        line.end, after,
        "with the full stop hanging, five characters plus it fit a measure of five"
    );
    assert_eq!(line.hanging, full_stop..after);

    // Without `w:overflowPunct` the same measure fits one character fewer.
    let plain = LineBreaker::new(
        text,
        LineBreakOptions {
            hanging_punctuation: false,
            ..LineBreakOptions::japanese_typesetting()
        },
    );
    assert_eq!(plain.hanging_tail(0..after), after..after);
    let plain_line = plain.next_line(0, 5.0, &mut one_unit_per_character(text));
    assert!(
        plain_line.end < after,
        "without hanging punctuation the line must be shorter: {} against {after}",
        plain_line.end
    );
}

/// The Latin case hand-off 5 of MJXOFF-160 says was missing: under `Default`, an English
/// sentence's line-final full stop **counts against the measure**.
///
/// This is the test that would have caught the old default, and it is written from both ends so
/// that neither half can be satisfied by accident: the same text under
/// [`LineBreakOptions::japanese_typesetting`] hangs the stop and fits one character more, and under
/// `Default` it does not. If the two agreed, the fixture would prove nothing.
#[test]
fn an_english_full_stop_counts_against_the_measure_under_the_default_options() {
    // Twelve characters, the last of which is an ASCII full stop, with break opportunities after
    // each space (bytes 4 and 8) and at the end (byte 12). A measure of eleven fits the whole text
    // only if the stop does not count.
    let text = "aaa bbb ccc.";
    let full_stop = "aaa bbb ccc".len();
    assert_eq!(&text[full_stop..], ".");
    assert_eq!(text.len(), 12);

    let default = LineBreaker::new(text, LineBreakOptions::default());
    assert_eq!(
        default.hanging_tail(0..text.len()),
        text.len()..text.len(),
        "nothing hangs under the default options, whatever the character is"
    );
    let line = default.next_line(0, 11.0, &mut one_unit_per_character(text));
    assert_eq!(
        line.end, 8,
        "the full stop counts, so twelve characters do not fit eleven and the line falls back"
    );
    assert_eq!(line.kind, LineBreakKind::Fitted);
    assert_eq!(line.hanging, 8..8);

    // The same text under Japanese typesetting: the ASCII full stop is in JIS X 4051's hangable set,
    // so it hangs past the measure and the whole text fits on one line. That is the behaviour the
    // old `Default` gave an English paragraph, silently.
    let japanese = LineBreaker::new(text, LineBreakOptions::japanese_typesetting());
    assert_eq!(
        japanese.hanging_tail(0..text.len()),
        full_stop..text.len(),
        "under Japanese typesetting the ASCII full stop is the hanging tail"
    );
    let hung = japanese.next_line(0, 11.0, &mut one_unit_per_character(text));
    assert_eq!(
        hung.end,
        text.len(),
        "with the stop hanging, eleven characters plus it fit a measure of eleven"
    );
    assert_eq!(hung.hanging, full_stop..text.len());
    assert_ne!(
        hung.end, line.end,
        "the two option sets must break in different places, or this fixture proves nothing"
    );
}

/// MJXOFF-158's one defect, found while MJXOFF-160 was writing the Latin case above: **a
/// paragraph's last line was exempt from its own measure.**
///
/// UAX #14 reports the end of the text as a *mandatory* break, and `next_line` used to return at the
/// first mandatory opportunity without measuring it. So a paragraph whose final word did not fit
/// came back as one enormous line even when a perfectly good break was available earlier — and the
/// fitting break the loop had already found in `best` was discarded.
///
/// The two halves are both asserted, because only together do they say the rule is *discriminating*:
/// the end sentinel is measured, and a real hard break still is not.
#[test]
fn the_end_of_the_text_is_measured_but_a_hard_break_is_not() {
    // One short word, a break opportunity at byte 2, then a word far longer than the measure.
    let text = "a bbbbbbbbbbbbbbbbbbbb";
    assert_eq!(text.len(), 22);
    let opportunities = break_opportunities(text, &LineBreakOptions::default());
    assert_eq!(
        opportunities,
        vec![
            mjx_text::BreakOpportunity {
                at: 2,
                kind: BreakKind::Allowed
            },
            mjx_text::BreakOpportunity {
                at: 22,
                kind: BreakKind::Mandatory
            },
        ],
        "the fixture rests on the end of the text being reported as mandatory"
    );

    let breaker = LineBreaker::new(text, LineBreakOptions::default());
    let line = breaker.next_line(0, 5.0, &mut one_unit_per_character(text));
    assert_eq!(
        line.end, 2,
        "the fitting break at byte 2 must be taken; returning all 22 characters against a measure \
         of 5 is what this test exists to catch"
    );
    assert_eq!(line.kind, LineBreakKind::Fitted);

    // A *hard* break is still unconditional: `\n` ends the line at byte 2 whatever the measure, and
    // a measure of a thousand does not stretch the line past it.
    let hard = LineBreaker::new("a\nbbbb", LineBreakOptions::default());
    let hard_line = hard.next_line(0, 1000.0, &mut one_unit_per_character("a\nbbbb"));
    assert_eq!(hard_line.end, 2);
    assert_eq!(hard_line.kind, LineBreakKind::Mandatory);
}

#[test]
fn a_line_that_is_nothing_but_hanging_punctuation_does_not_measure_zero() {
    // Otherwise a line of commas would never advance.
    let text = "。。。";
    let breaker = LineBreaker::new(text, LineBreakOptions::japanese_typesetting());
    assert_eq!(breaker.hanging_tail(0..text.len()), text.len()..text.len());
}

// ---------------------------------------------------------------------------------------------
// Fitting
// ---------------------------------------------------------------------------------------------

#[test]
fn a_word_longer_than_the_measure_overflows_rather_than_producing_an_empty_line() {
    let text = "antidisestablishmentarianism and more";
    let breaker = LineBreaker::new(text, LineBreakOptions::default());
    let line = breaker.next_line(0, 5.0, &mut one_unit_per_character(text));
    assert_eq!(line.kind, LineBreakKind::Overflowing);
    assert_eq!(line.end, "antidisestablishmentarianism ".len());
    assert!(
        line.end > 0,
        "a line with nothing on it would never terminate"
    );
}

#[test]
fn a_measure_wide_enough_for_everything_takes_the_whole_text() {
    let text = "one two three";
    let breaker = LineBreaker::new(text, LineBreakOptions::default());
    let line = breaker.next_line(0, 1000.0, &mut one_unit_per_character(text));
    assert_eq!(line.end, text.len());
    assert_eq!(line.kind, LineBreakKind::EndOfText);
}

#[test]
fn breaking_a_paragraph_line_by_line_terminates_and_covers_every_byte() {
    let text = "The quick brown fox jumps over the lazy dog, and then it does it again.";
    let breaker = LineBreaker::new(text, LineBreakOptions::default());
    let mut cursor = 0;
    let mut lines = Vec::new();
    while cursor < text.len() {
        let line = breaker.next_line(cursor, 12.0, &mut one_unit_per_character(text));
        assert!(
            line.end > cursor,
            "the breaker made no progress at {cursor}"
        );
        lines.push(&text[cursor..line.end]);
        cursor = line.end;
    }
    assert_eq!(lines.concat(), text);
    assert!(lines.len() > 4);
}

#[test]
fn breaking_past_the_end_of_the_text_is_not_a_panic() {
    let text = "short";
    let breaker = LineBreaker::new(text, LineBreakOptions::default());
    let line = breaker.next_line(99, 10.0, &mut one_unit_per_character(text));
    assert_eq!(line.end, text.len());
    assert_eq!(line.kind, LineBreakKind::EndOfText);
    // A backwards range is what a caller with a bug hands in, and answering an empty tail is the
    // promise. Clippy is right that the literal range is empty; that is the input under test.
    #[allow(clippy::reversed_empty_ranges)]
    let backwards = 3..1;
    assert!(breaker.hanging_tail(backwards).is_empty());
    assert!(breaker.hanging_tail(0..999).is_empty());
}

#[test]
fn an_empty_paragraph_offers_one_mandatory_break_and_nothing_else() {
    let opportunities = break_opportunities("", &LineBreakOptions::default());
    assert!(opportunities.is_empty() || opportunities.iter().all(|o| o.at == 0));
    let breaker = LineBreaker::new("", LineBreakOptions::default());
    let line = breaker.next_line(0, 10.0, &mut one_unit_per_character(""));
    assert_eq!(line.end, 0);
}

// ---------------------------------------------------------------------------------------------
// A document's own kinsoku sets
// ---------------------------------------------------------------------------------------------

#[test]
fn a_document_may_declare_its_own_kinsoku_sets() {
    // `w:noLineBreaksBefore` and `w:noLineBreaksAfter` carry exactly these two strings. A document
    // that names only `x` gets a rule about `x` and nothing else.
    let text = "aaa xbb";
    let options = LineBreakOptions {
        kinsoku: KinsokuRules::from_character_sets("x", "", ""),
        ..LineBreakOptions::japanese_typesetting()
    };

    assert!(allowed(text, &LineBreakOptions::unicode_only()).contains(&4));
    assert!(!allowed(text, &options).contains(&4));

    // And the standard set no longer applies, because the document replaced it.
    assert!(!options.kinsoku.prohibits_at_line_start('%'));
    assert!(KinsokuRules::japanese_standard().prohibits_at_line_start('%'));
}

#[test]
fn the_standard_set_is_the_one_word_ships() {
    let kinsoku = KinsokuRules::japanese_standard();
    // 行頭禁則 — a sample across each class the set covers.
    for character in [
        '、', '。', '」', '』', '】', 'ぁ', 'っ', 'ゃ', 'ー', '・', '％', '%', '℃', '‰', '°',
    ] {
        assert!(
            kinsoku.prohibits_at_line_start(character),
            "{character:?} must not begin a line"
        );
    }
    // 行末禁則.
    for character in ['「', '『', '【', '（', '(', '¥', '$', '“'] {
        assert!(
            kinsoku.prohibits_at_line_end(character),
            "{character:?} must not end a line"
        );
    }
    // And an ordinary ideograph is in neither.
    assert!(!kinsoku.prohibits_at_line_start('日'));
    assert!(!kinsoku.prohibits_at_line_end('日'));
    // `w:overflowPunct` applies to the comma and the full stop, not to a bracket.
    assert!(kinsoku.hangs('。'));
    assert!(kinsoku.hangs('、'));
    assert!(!kinsoku.hangs('」'));
}
