//! The three counters a section owns — page numbers, line numbers and note marks — and the numeral
//! systems they are written in.
//!
//! # These are computed here and rendered by different things
//!
//! A **page** number is displayed by a `PAGE` field, and fields are MJXOFF-177 (R22); this child
//! computes the number and draws nothing. A **line** number is not a field at all — Word generates
//! it and draws it in the margin — so it is computed *and* drawn here. A **note** mark is generated
//! the same way, and the number that appears at the head of the note itself is drawn here too, while
//! the reference mark in the body is not: the body's run stream contributes no character for it (see
//! `mjx_docx::NoteReference`), so there is nothing on that line to hang a glyph on until R22 makes
//! generated marks part of the text.
//!
//! # Why a note's number needs no checkpoint field
//!
//! The obvious design carries a running count in the continuation state, and it is unnecessary: the
//! *n*th footnote reference in document order is footnote *n*, and how many references precede a
//! position is a **prefix sum over the paragraphs**, which [`crate::model::DocumentFlow`] computes
//! once when it reads the document. A restart per section is the same sum taken from the section's
//! first paragraph, and a restart per page is a count of what is on the page. None of the three
//! needs to know how the pages before it were broken, so none of them costs a byte of checkpoint or
//! a page of walking.
//!
//! A **line** number is the exception and it is the reason the continuation state grew: under
//! `w:restart="continuous"` the count is over *lines*, and how many lines precede a position is
//! exactly the thing that cannot be known without laying them out.

use mjx_ooxml_types::wordprocessingml::NumberFormat;

/// `value` written in `format`.
///
/// # What is implemented, and what falls back
///
/// `ST_NumberFormat` has sixty-three members and most of them are East Asian, Hebrew, Thai or
/// Vietnamese numeral systems whose digits this repository commits no face for. The five that carry
/// almost every real document — decimal, the two Roman cases and the two Latin-letter cases — are
/// implemented exactly; every other member falls back to decimal.
///
/// **The fallback is a decision and it is visible.** A document asking for `ideographDigital` gets
/// `3` rather than 三, which is wrong and is *legible*; producing nothing, or producing a box glyph,
/// would leave a reader unable to tell a numbering scheme this engine cannot write from one it got
/// wrong. `crate::model::DocumentBoxModel` reports the format it was asked for, so a caller can say
/// so.
#[must_use]
pub fn format_number(value: i64, format: NumberFormat) -> String {
    match format {
        NumberFormat::UppercaseRomanNumerals => roman(value, true),
        NumberFormat::LowercaseRomanNumerals => roman(value, false),
        NumberFormat::UppercaseLatinAlphabet => latin(value, true),
        NumberFormat::LowercaseLatinAlphabet => latin(value, false),
        NumberFormat::None => String::new(),
        _ => value.to_string(),
    }
}

/// Whether [`format_number`] writes `format` in the system the document asked for.
///
/// `false` says the number came out in decimal because nothing here can write that system — which
/// is a fidelity difference a reader would see, and one worth reporting rather than hiding.
#[must_use]
pub fn is_written_exactly(format: NumberFormat) -> bool {
    matches!(
        format,
        NumberFormat::Decimal
            | NumberFormat::UppercaseRomanNumerals
            | NumberFormat::LowercaseRomanNumerals
            | NumberFormat::UppercaseLatinAlphabet
            | NumberFormat::LowercaseLatinAlphabet
            | NumberFormat::None
    )
}

/// The largest number [`format_number`] writes as Roman numerals rather than as decimal digits.
///
/// Above it there is no additive notation without the overline that means "times a thousand", and
/// `MMMM…` is not a numeral anybody reads. A document with four thousand pages gets decimal.
pub const LARGEST_ROMAN: i64 = 3999;

/// `value` in Roman numerals, or in decimal when it is outside [`LARGEST_ROMAN`].
fn roman(value: i64, upper: bool) -> String {
    if value <= 0 || value > LARGEST_ROMAN {
        return value.to_string();
    }
    const TABLE: [(i64, &str); 13] = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut left = value;
    let mut out = String::new();
    for (amount, numeral) in TABLE {
        while left >= amount {
            out.push_str(numeral);
            left -= amount;
        }
    }
    if upper {
        out.to_uppercase()
    } else {
        out
    }
}

/// `value` as `a`, `b`, … `z`, `aa`, `bb`, … — Word's own scheme, which repeats a letter rather than
/// counting in base twenty-six.
///
/// **`upperLetter` is not a bijective base-26 count**, and reading it as one is the plausible wrong
/// answer: `ST_NumberFormat`'s own description is *"Uppercase Latin Alphabet"* and Word writes the
/// twenty-seventh as `AA`, the twenty-eighth as `BB` and the fifty-third as `AAA`. Counting in base
/// twenty-six would write `AA`, `AB`, `AC`, which is a different sequence from the third item on.
fn latin(value: i64, upper: bool) -> String {
    if value <= 0 {
        return value.to_string();
    }
    let zero_based = value - 1;
    let letter = u32::try_from(zero_based.rem_euclid(26)).unwrap_or(0);
    let repeats = usize::try_from(zero_based.div_euclid(26) + 1).unwrap_or(1);
    let base = if upper { b'A' } else { b'a' };
    let character = char::from(base + u8::try_from(letter).unwrap_or(0));
    std::iter::repeat_n(character, repeats).collect()
}

/// Whether the line with ordinal `ordinal` carries a printed number, given `count_by` and `start`.
///
/// **GUESS:** the *n*th line is printed when its own number is a multiple of `w:countBy`. §17.6.10
/// says `countBy` is the "Line Number Increment" and does not say what the increment is measured
/// from, so a section starting at 5 and counting by 5 might reasonably print 5, 10, 15 (this
/// reading) or 5, 10, 15 measured from the start, which is the same — but a section starting at 3
/// and counting by 5 prints 5, 10 here and 3, 8, 13 under the other reading. That is a visible
/// difference in the margin of every page and it is a question only Word can settle.
#[must_use]
pub fn is_numbered(ordinal: i64, count_by: i64, start: i64) -> bool {
    let _ = start;
    if count_by <= 1 {
        return true;
    }
    ordinal.rem_euclid(count_by) == 0
}
