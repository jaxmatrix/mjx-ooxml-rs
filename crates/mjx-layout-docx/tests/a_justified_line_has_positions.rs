//! Justification, asserted on **where the words are** — and the identity value that a
//! left-aligned fixture would never notice.
//!
//! # The trap this file exists for
//!
//! Left-aligned text is the *no-op* case: the inter-word spacing is untouched and every word sits
//! where its predecessor's advance put it. A suite whose fixtures are all left-aligned has run the
//! justification code at an identity value and proved nothing about it — the same way a
//! number-format suite that only ever formats zero has run the formatter and learned nothing.
//!
//! # Why a justified line is compared against **itself**
//!
//! The obvious comparison — the same text left-aligned and justified, word for word — does not
//! exist, and finding that out was worth the attempt. A justified paragraph is **cut at every
//! space** so that its words can move independently ([`mjx_layout_docx::CutPolicy`]), and a
//! left-aligned one is not cut at all; so a line that is one glyph run left-aligned is seventeen
//! justified, and there is no correspondence to compare. What every test below compares instead is
//! the *actual* position of each run against the position its predecessors' own advances would have
//! put it at. The shifts are then `0, e, 2e, 3e…` for a justified line and `0, 0, 0…` for a
//! left-aligned one, which is a statement about justification and not about line width.
//!
//! # The East Asian case is asserted one layer down, and that is a limitation
//!
//! **This repository commits no CJK face** — `crates/mjx-text/assets/fonts/` holds four Latin
//! families — so a Japanese paragraph produces no glyph runs at all here and an end-to-end position
//! assertion is not available. The decision is asserted where it is made instead:
//! [`mjx_layout_docx::CutPolicy`] cuts around each ideograph and
//! [`mjx_layout_docx::expansion_points`] finds the gap. That is a real gap in the evidence and it
//! is stated rather than papered over.

mod support;

use mjx_layout_docx::{cut, expansion_points, Alignment, CutPolicy};
use support::{constraints, glyph_positions, one_page, paragraph, shifts};

/// Long enough to wrap twice at 6.5 inches, so there is a middle line that is neither the first nor
/// the last.
const PROSE: &str = "The quick brown fox jumps over the lazy dog again and again while the \
                     patient tortoise watches from a comfortable distance and says nothing at \
                     all about the matter under discussion this afternoon in the garden";

fn left(text: &str) -> String {
    paragraph("", text)
}

fn justified(text: &str) -> String {
    paragraph(r#"<w:jc w:val="both"/>"#, text)
}

fn distributed(text: &str) -> String {
    paragraph(r#"<w:jc w:val="distribute"/>"#, text)
}

/// Every value must be the same, and non-zero — the shape "one equal share per gap" takes.
fn one_equal_share(steps: &[i64]) -> bool {
    let widest = steps.iter().copied().max().unwrap_or(0);
    let narrowest = steps.iter().copied().min().unwrap_or(0);
    widest > 0 && widest - narrowest <= 1
}

#[test]
fn a_left_aligned_line_starts_at_the_margin_and_moves_no_word() {
    let constraints = constraints(6.5, 11.0);
    let tree = one_page(&[left(PROSE)], &constraints);
    assert_eq!(
        glyph_positions(&tree, 0, 0).first().copied(),
        Some(0),
        "a left-aligned line starts at the leading margin"
    );
    assert!(
        shifts(&tree, 0, 0).iter().all(|shift| *shift == 0),
        "a left-aligned line moves nothing: {:?}",
        shifts(&tree, 0, 0)
    );
}

/// **The load-bearing assertion.** Every word after the first sits further right than its own
/// advance would have put it, by one more share of the slack each time.
#[test]
fn a_justified_line_widens_every_gap_by_the_same_share() {
    let constraints = constraints(6.5, 11.0);
    let tree = one_page(&[justified(PROSE)], &constraints);
    let moved = shifts(&tree, 0, 0);
    assert!(
        moved.len() > 5,
        "the fixture must give the line several words: {}",
        moved.len()
    );
    assert_eq!(
        moved.first().copied(),
        Some(0),
        "the first word does not move"
    );
    let steps: Vec<i64> = moved.windows(2).map(|pair| pair[1] - pair[0]).collect();
    assert!(
        one_equal_share(&steps),
        "every gap must be widened by the same non-zero share: {steps:?}"
    );
}

/// The **last** line of a `w:jc="both"` paragraph is not stretched. A justifier that stretched it
/// would spread three words across the page, which is the single most recognisable rendering defect
/// there is.
#[test]
fn the_last_line_of_a_justified_paragraph_is_not_stretched() {
    let constraints = constraints(6.5, 11.0);
    let tree = one_page(&[justified(PROSE)], &constraints);
    let last = support::lines_of(&tree, 0) - 1;
    assert!(last > 0, "the fixture must wrap");
    assert!(
        shifts(&tree, 0, last).iter().all(|shift| *shift == 0),
        "a justified paragraph's last line is left-aligned: {:?}",
        shifts(&tree, 0, last)
    );
    // …and the line before it *is* stretched, which is what says the exemption is the last line and
    // not the whole paragraph.
    assert!(
        shifts(&tree, 0, last - 1).iter().any(|shift| *shift > 0),
        "the line before the last must still be stretched"
    );
}

/// **The identity value, stated as one.** A paragraph of a single unbroken word has no gap to
/// widen, so justifying it must place it exactly where left-aligning it does.
#[test]
fn a_line_with_no_gap_cannot_be_justified_and_is_not_moved() {
    let constraints = constraints(6.5, 11.0);
    let word = "Antidisestablishmentarianism";
    let plain = one_page(&[left(word)], &constraints);
    let stretched = one_page(&[justified(word)], &constraints);
    assert_eq!(
        glyph_positions(&stretched, 0, 0),
        glyph_positions(&plain, 0, 0),
        "there is nothing to widen, so nothing moves"
    );
}

/// `w:jc="distribute"` stretches the **last** line too, which is the one thing that tells it from
/// `both`.
#[test]
fn distributed_justification_stretches_the_last_line_as_well() {
    let constraints = constraints(6.5, 11.0);
    let both = one_page(&[justified(PROSE)], &constraints);
    let spread = one_page(&[distributed(PROSE)], &constraints);

    let last_both = support::lines_of(&both, 0) - 1;
    let last_spread = support::lines_of(&spread, 0) - 1;
    assert!(
        shifts(&both, 0, last_both).iter().all(|shift| *shift == 0),
        "`both` leaves the last line alone"
    );
    assert!(
        shifts(&spread, 0, last_spread)
            .iter()
            .any(|shift| *shift > 0),
        "`distribute` stretches the last line: {:?}",
        shifts(&spread, 0, last_spread)
    );
}

/// `distribute` widens the gaps between **characters**, not only between words — so a line with a
/// dozen words has far more expansion points under it than under `both`.
#[test]
fn distributed_justification_expands_between_characters_and_not_only_between_words() {
    let text = "one two three";
    let by_words = cut(text, 0..text.len(), CutPolicy::Words);
    let by_characters = cut(text, 0..text.len(), CutPolicy::Characters);
    assert_eq!(
        expansion_points(text, &by_words, Alignment::Justified).len(),
        2,
        "two spaces, two gaps"
    );
    assert_eq!(
        expansion_points(text, &by_characters, Alignment::Distributed).len(),
        text.chars().count() - 1,
        "every boundary between characters is a gap"
    );
}

/// **The East Asian difference, asserted where it is decided.** A Japanese line has no spaces on it,
/// so a justifier that only cut at spaces would leave it in one piece — and a line in one piece
/// cannot be stretched, which is a ragged right edge in a paragraph that asked to be flush.
#[test]
fn a_japanese_line_is_cut_between_its_characters_and_a_latin_word_is_not() {
    let japanese = "本日は晴天なり";
    let pieces = cut(japanese, 0..japanese.len(), CutPolicy::Words);
    assert_eq!(
        pieces.len(),
        japanese.chars().count(),
        "each ideograph is its own expansion unit: {pieces:?}"
    );
    assert_eq!(
        expansion_points(japanese, &pieces, Alignment::Justified).len(),
        japanese.chars().count() - 1,
        "and every boundary between two of them is a gap justification may widen"
    );

    // The Latin control, in the same call, so the difference is the assertion rather than a claim.
    let latin = "Antidisestablishmentarianism";
    let latin_pieces = cut(latin, 0..latin.len(), CutPolicy::Words);
    assert_eq!(latin_pieces.len(), 1, "a Latin word is one unit");
    assert!(
        expansion_points(latin, &latin_pieces, Alignment::Justified).is_empty(),
        "and offers justification nothing at all"
    );
}

/// A mixed line does both at once, which is what Word does and what a single-policy justifier
/// cannot: the Latin half is widened at its spaces and the Japanese half between its characters.
#[test]
fn a_mixed_line_offers_both_kinds_of_gap() {
    let mixed = "one two 本日は";
    let pieces = cut(mixed, 0..mixed.len(), CutPolicy::Words);
    let points = expansion_points(mixed, &pieces, Alignment::Justified);
    assert!(
        points.len() >= 4,
        "two spaces and the gaps between three ideographs: {points:?} over {pieces:?}"
    );
}

/// Centring and right-alignment are **rigid** translations: every word moves by the same amount.
/// Asserting that is what tells a translation from a stretch.
#[test]
fn centring_and_right_alignment_translate_the_whole_line_by_one_amount() {
    let constraints = constraints(6.5, 11.0);
    let short = "Three short words";
    let plain = one_page(&[left(short)], &constraints);
    let centred = one_page(
        &[paragraph(r#"<w:jc w:val="center"/>"#, short)],
        &constraints,
    );
    let ended = one_page(
        &[paragraph(r#"<w:jc w:val="right"/>"#, short)],
        &constraints,
    );

    let start = glyph_positions(&plain, 0, 0)[0];
    let centre_shift = glyph_positions(&centred, 0, 0)[0] - start;
    let end_shift = glyph_positions(&ended, 0, 0)[0] - start;

    assert!(
        centre_shift > 0,
        "centring must move the line: {centre_shift}"
    );
    assert!(
        shifts(&centred, 0, 0).iter().all(|shift| *shift == 0),
        "centring moves the line rigidly, widening no gap"
    );
    assert!(
        shifts(&ended, 0, 0).iter().all(|shift| *shift == 0),
        "so does right-alignment"
    );
    assert!(
        (end_shift - centre_shift * 2).abs() <= 2,
        "right alignment must be twice centring: {centre_shift} and {end_shift}"
    );
}

/// `left` and `right` are **not** synonyms for `start` and `end`: in a right-to-left paragraph,
/// `w:jc="left"` still means the physical left edge, which is the *trailing* margin.
#[test]
fn left_and_right_are_physical_and_start_and_end_are_not() {
    use mjx_ooxml_types::wordprocessingml::Justification;
    use mjx_text::TextDirection;
    assert_eq!(
        Alignment::of(Some(Justification::Left), TextDirection::RightToLeft),
        Alignment::End,
        "in a right-to-left paragraph the physical left edge is the trailing one"
    );
    assert_eq!(
        Alignment::of(Some(Justification::Start), TextDirection::RightToLeft),
        Alignment::Start,
        "`start` follows the paragraph's direction and `left` does not"
    );
    assert_eq!(
        Alignment::of(Some(Justification::Left), TextDirection::LeftToRight),
        Alignment::Start
    );
}

/// **The identity value, read from the engine rather than inferred from a picture.**
///
/// A fragment tree shows where a run went and not why. `LinePlacement::expansion_each` is the number
/// the justifier actually widened each gap by, and it is **zero** for every alignment but the two
/// justifying ones — which is what makes a left-aligned fixture an identity case rather than a
/// passing test.
#[test]
fn the_expansion_is_reported_and_is_zero_where_it_should_be() {
    for (name, properties) in [
        ("start", ""),
        ("centre", r#"<w:jc w:val="center"/>"#),
        ("end", r#"<w:jc w:val="right"/>"#),
    ] {
        let layout = support::lay_out_one(properties, PROSE, 6.5);
        for (index, line) in layout.lines.iter().enumerate() {
            assert_eq!(
                line.placement.expansion_each,
                mjx_ooxml_core::measure::Emu::ZERO,
                "{name}: line {index} widened a gap, and only justification may"
            );
            assert_eq!(line.placement.expansion_points, 0, "{name}: line {index}");
        }
    }

    let justified = support::lay_out_one(r#"<w:jc w:val="both"/>"#, PROSE, 6.5);
    let first = justified.lines.first().expect("a first line");
    assert!(
        first.placement.expansion_points > 3,
        "a justified line of prose has several gaps: {}",
        first.placement.expansion_points
    );
    assert!(
        first.placement.expansion_each > mjx_ooxml_core::measure::Emu::ZERO,
        "and each of them was widened by something"
    );
    // The arithmetic, checked: the gaps widened plus the natural width is the measure.
    let filled = first.placement.natural_width
        + first
            .placement
            .expansion_each
            .times(i64::try_from(first.placement.expansion_points).expect("a count"));
    let measure = mjx_ooxml_core::measure::Emu::from_inches(6.5);
    assert!(
        (filled - measure).emu().abs() <= i64::try_from(first.placement.expansion_points).expect("a count"),
        "the widened line must fill the measure, up to one EMU a gap: {filled:?} against {measure:?}"
    );

    // The last line is the exemption, and it reports zero — the same identity, on the same page.
    let last = justified.lines.last().expect("a last line");
    assert_eq!(
        last.placement.expansion_each,
        mjx_ooxml_core::measure::Emu::ZERO,
        "the last line of a `both` paragraph is not stretched"
    );
}
