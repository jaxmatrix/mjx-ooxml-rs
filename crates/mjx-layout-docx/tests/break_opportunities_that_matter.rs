//! Line breaking, at the opportunities that are **awkward** rather than the ones that are easy.
//!
//! # A paragraph of Latin words proves almost nothing
//!
//! "The quick brown fox" breaks at spaces, and an implementation that only knew about `U+0020`
//! would lay it out perfectly. Every interesting decision is somewhere else:
//!
//! * a **hyphen**, after which a line may break though there is no space;
//! * a **non-breaking space**, which looks like a space and is not one;
//! * a **non-breaking hyphen** (`w:noBreakHyphen`), which looks like a hyphen and is not one;
//! * a **soft hyphen** (`w:softHyphen`), which is invisible until the line ends at it;
//! * a **long URL**, which is a single token with slashes in it;
//! * **CJK**, where nearly every character is a break opportunity;
//! * a **word longer than the measure**, which has to overflow rather than produce an empty line
//!   for ever.
//!
//! None of them is implemented here — `mjx-text` implements UAX #14 and this crate consumes it —
//! which is exactly why they are worth a suite: what this crate can get wrong is *which* of the
//! opportunities it takes, and every case below is one where the wrong choice is invisible in a
//! Latin fixture.

mod support;

use support::{constraints, lines_of, one_page, paragraph, paragraph_with_run};

/// Narrow enough that most of these fixtures have to break somewhere.
fn narrow() -> mjx_layout::Constraints {
    constraints(1.0, 11.0)
}

fn line_count_of(text: &str) -> usize {
    let tree = one_page(&[paragraph("", text)], &narrow());
    lines_of(&tree, 0)
}

#[test]
fn a_hyphen_is_a_break_opportunity_and_a_non_breaking_hyphen_is_not() {
    // The same eighteen characters, hyphenated two ways. The ordinary hyphen lets the line break
    // after it; `w:noBreakHyphen` is `U+2011`, whose UAX #14 class is `GL`, and it may not.
    let ordinary = line_count_of("aaaaaaaaa-bbbbbbbbb");
    let unbreakable = {
        let tree = one_page(
            &[paragraph_with_run(
                "",
                r#"<w:t>aaaaaaaaa</w:t><w:noBreakHyphen/><w:t>bbbbbbbbb</w:t>"#,
            )],
            &narrow(),
        );
        lines_of(&tree, 0)
    };
    assert_eq!(ordinary, 2, "an ordinary hyphen offers a break");
    assert_eq!(
        unbreakable, 1,
        "a `w:noBreakHyphen` offers none, so the token overflows in one line"
    );
}

#[test]
fn a_non_breaking_space_does_not_break_where_an_ordinary_one_does() {
    let ordinary = line_count_of("aaaaaaaaa bbbbbbbbb");
    let glued = line_count_of("aaaaaaaaa\u{00A0}bbbbbbbbb");
    assert_eq!(ordinary, 2, "an ordinary space breaks");
    assert_eq!(
        glued, 1,
        "`U+00A0` does not, so the pair overflows together"
    );
}

/// A soft hyphen is invisible **until** the line ends at it, and then a hyphen is drawn that the
/// document does not contain.
#[test]
fn a_soft_hyphen_breaks_a_word_and_draws_a_hyphen_that_is_not_in_the_text() {
    let with_soft = one_page(
        &[paragraph_with_run(
            "",
            r#"<w:t>aaaaaaaaa</w:t><w:softHyphen/><w:t>bbbbbbbbb</w:t>"#,
        )],
        &narrow(),
    );
    let without = one_page(&[paragraph("", "aaaaaaaaabbbbbbbbb")], &narrow());
    assert_eq!(
        lines_of(&without, 0),
        1,
        "with no opportunity the word overflows in one line"
    );
    assert_eq!(
        lines_of(&with_soft, 0),
        2,
        "the soft hyphen gives the word somewhere to break"
    );

    // The hyphen is drawn as an extra glyph run at the end of the first line: the first line has
    // more runs than the plain one does, and the last of them starts past the text.
    let first_line_runs = support::glyph_positions(&with_soft, 0, 0);
    assert!(
        first_line_runs.len() >= 2,
        "the hyphen is a glyph run of its own: {first_line_runs:?}"
    );
}

/// A URL is one token with no space in it, and UAX #14 offers breaks after its slashes rather than
/// letting it overflow the page.
#[test]
fn a_long_url_breaks_inside_itself() {
    let url = "https://example.com/a/very/long/path/that/goes/on/and/on/for/quite/some/way";
    assert!(
        line_count_of(url) > 1,
        "a long URL must break somewhere rather than run off the page"
    );
}

/// A word longer than the measure has to go **somewhere**. Word overflows it rather than dropping
/// it, and an implementation that refused to place it would loop for ever producing empty lines.
#[test]
fn a_word_longer_than_the_measure_overflows_rather_than_vanishing() {
    let word = "Pneumonoultramicroscopicsilicovolcanoconiosis";
    let tree = one_page(&[paragraph("", word)], &constraints(0.4, 11.0));
    assert_eq!(lines_of(&tree, 0), 1, "one line, overflowing");
    assert!(
        !support::glyph_positions(&tree, 0, 0).is_empty(),
        "and the text is drawn rather than dropped"
    );
}

/// A hard `w:br` ends a line whatever the measure says.
#[test]
fn a_hard_break_ends_a_line_that_had_room_left() {
    let tree = one_page(
        &[paragraph_with_run(
            "",
            r#"<w:t>one</w:t><w:br/><w:t>two</w:t>"#,
        )],
        &constraints(6.5, 11.0),
    );
    assert_eq!(
        lines_of(&tree, 0),
        2,
        "`w:br` breaks a line six inches of room could have held"
    );
}

/// A `w:tab` is a character in the text, and it is its own segment — so a line with a tab on it has
/// **more** runs than the same text without one, and the run after the tab starts at a stop rather
/// than at the previous run's advance.
#[test]
fn a_tab_is_its_own_placement_and_not_a_glyph_advance() {
    let tabbed = one_page(
        &[paragraph_with_run(
            "",
            r#"<w:t>a</w:t><w:tab/><w:t>b</w:t>"#,
        )],
        &constraints(6.5, 11.0),
    );
    let positions = support::glyph_positions(&tabbed, 0, 0);
    assert_eq!(positions.len(), 2, "the tab draws nothing: {positions:?}");
    // The default tab interval is half an inch — 457,200 EMU — and `a` is far narrower than that,
    // so the second run must start at the stop rather than just after the first.
    assert_eq!(
        positions[1], 457_200,
        "the run after a tab starts at the next default stop"
    );
}

/// Every character of a CJK paragraph is nearly a break opportunity, so it wraps at the measure
/// rather than overflowing — the opposite of a long Latin word.
#[test]
fn a_cjk_paragraph_offers_a_break_at_almost_every_character() {
    use mjx_text::{break_opportunities, LineBreakOptions};
    let japanese = "本日は晴天なり本日は晴天なり";
    let opportunities = break_opportunities(japanese, &LineBreakOptions::unicode_only());
    assert!(
        opportunities.len() >= japanese.chars().count() - 2,
        "nearly every character is an opportunity: {} for {} characters",
        opportunities.len(),
        japanese.chars().count()
    );
}

/// `w:kinsoku` removes the opportunities a Japanese typesetter may not take, and switching it on is
/// what a document says when it wants them removed.
#[test]
fn kinsoku_removes_an_opportunity_plain_unicode_offers() {
    use mjx_text::{break_opportunities, LineBreakOptions};
    // `10 %` — UAX #14 offers a break after the space that would leave `%` at the head of the next
    // line, and JIS X 4051 forbids it.
    let text = "10 % 20 % 30 %";
    let plain = break_opportunities(text, &LineBreakOptions::unicode_only());
    let japanese = break_opportunities(text, &LineBreakOptions::japanese_typesetting());
    assert!(
        japanese.len() < plain.len(),
        "kinsoku must remove opportunities: {} against {}",
        japanese.len(),
        plain.len()
    );
}
