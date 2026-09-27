//! A malformed `formatCode` renders something rather than failing a sheet.
//!
//! # Why this is a suite of its own
//!
//! Broken format codes are ordinary in real files: a truncated `[`, an unterminated quote, a locale
//! prefix from a producer nobody has heard of, forty sections, a bracket with a condition that will
//! not parse. A viewer that refuses a worksheet because one cell's style is odd is worse than one
//! that draws the cell plainly, and the whole engine is written to be **total** for that reason.
//!
//! So this suite asserts three things a green conformance table cannot:
//!
//! 1. Nothing panics. The engine has no `unwrap`, no `expect` and no indexing that could be out of
//!    range, and the way to hold that is to run it against input designed to break it.
//! 2. Nothing loops. Every bound in the parser — the section count, the code length, the fraction
//!    denominator, the continued-fraction term count — is exercised by a code that would otherwise
//!    run away.
//! 3. The degradation is *useful*: a code that is only a broken bracket still writes the value.

use mjx_layout_xlsx::numfmt::{evaluate, CellValue, CompiledFormat, FormatCache};
use mjx_xlsx::DateSystem;

/// Codes that are wrong in a way a real file is wrong.
const BROKEN: &[&str] = &[
    "",
    ";",
    ";;",
    ";;;;;;;;;;;;;;;;",
    "[",
    "]",
    "[Red",
    "[]0.00",
    "[Colour]0.00",
    "[Color]0.00",
    "[Color-1]0",
    "[ColorNaN]0",
    "[>]0.00",
    "[>abc]0.00",
    "[>1e400]0",
    "[<>]0",
    "\"unterminated",
    "\\",
    "_",
    "*",
    "0.00.00",
    "0,,,,,,,,,,,,,,,,,,,,",
    "#########################",
    "?????????????????????????",
    "0.000000000000000000000000000000000000000",
    "/",
    "0/",
    "/0",
    "# ?/",
    "# /?",
    "# ?/0",
    "# ?/00000000000000000000",
    "# ?????????????????????/?????????????????????",
    "E+",
    "0E+",
    "E+0",
    "0.00E",
    "@@@@",
    "@0.00@",
    "General General",
    "[$",
    "[$]",
    "[$-]0",
    "[$-ZZZZ]0",
    "[h",
    "[hhhhhhhhhhhhhhhhhhhhhhhhhhhh]",
    "[q]0",
    "mmmmmmmmmmmmmmmm",
    "yyyyyyyyyyyyyyyy",
    "dddddddddddddddd",
    "hhhhhhhhhhhhhhhh",
    "AM/",
    "A/",
    "\u{0}\u{1}\u{2}",
    "0\u{fffd}0",
    "🙂0🙂",
    "0;0;0;0;0;0;0;0",
];

/// Values chosen to break something: the ends of the range, the sign of zero, the two epochs'
/// boundaries and the phantom day.
const VALUES: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    59.0,
    60.0,
    61.0,
    45719.5,
    2_958_465.0,
    2_958_466.0,
    -1.0e9,
    1.0e9,
    1.0e-9,
    -1.0e-9,
    f64::MIN_POSITIVE,
    f64::MAX,
    f64::MIN,
    1.0 / 3.0,
    0.1 + 0.2,
];

/// Every broken code, against every value, in both epochs, in every kind of cell.
///
/// The assertion is that this returns at all: a panic aborts the test, and an unbounded loop times
/// the run out.
#[test]
fn nothing_panics_and_nothing_runs_away() {
    let mut cache = FormatCache::new();
    for code in BROKEN {
        let compiled = CompiledFormat::compile(code);
        assert!(
            !compiled.sections().is_empty(),
            "{code:?} compiled to no section at all"
        );
        assert!(
            compiled.sections().len() <= 4,
            "{code:?} compiled to {} sections",
            compiled.sections().len()
        );
        for dates in [DateSystem::Windows1900, DateSystem::Macintosh1904] {
            for value in VALUES {
                let _ = evaluate(&compiled, CellValue::Number(*value), dates);
                let _ = cache.format(Some(code), CellValue::Number(*value), dates);
            }
            let _ = evaluate(&compiled, CellValue::Text("abc"), dates);
            let _ = evaluate(&compiled, CellValue::Text(""), dates);
            let _ = evaluate(&compiled, CellValue::Boolean(true), dates);
            let _ = evaluate(&compiled, CellValue::Error("#REF!"), dates);
        }
    }
}

/// A code longer than the parser will read is truncated rather than walked.
#[test]
fn a_pathological_code_is_bounded() {
    let long = "0".repeat(100_000);
    let compiled = CompiledFormat::compile(&long);
    let rendered = evaluate(&compiled, CellValue::Number(1.0), DateSystem::Windows1900);
    // Four thousand and ninety-six placeholders, of which the last one holds the digit; the rest are
    // zeroes. The point is the *bound*, which is what the length says.
    assert_eq!(
        rendered.text.chars().count(),
        4096,
        "the code is read up to its bound and no further"
    );
    let sections = "0;".repeat(100_000);
    assert_eq!(
        CompiledFormat::compile(&sections).sections().len(),
        4,
        "a code with a hundred thousand sections keeps four"
    );
}

/// A broken bracket still shows the number.
///
/// The degradation that matters: the reader sees their data, not an error and not an empty cell.
#[test]
fn a_broken_bracket_still_writes_the_value() {
    for code in ["[Red0.00", "[foo]0.00", "[$]0.00", "[>abc]0.00"] {
        let rendered = evaluate(
            &CompiledFormat::compile(code),
            CellValue::Number(12.5),
            DateSystem::Windows1900,
        );
        assert!(
            rendered.text.contains("12.5"),
            "{code:?} rendered {:?}, which does not show the value",
            rendered.text
        );
    }
}

/// An unreadable code never colours the text.
///
/// A colour is applied to a *reader's* cell, so guessing one from a bracket that did not parse would
/// change what a person sees on the strength of a producer's typo.
#[test]
fn an_unreadable_colour_is_no_colour() {
    for code in ["[Colour]0", "[Color]0", "[Color0]0", "[Color57]0", "[]0"] {
        let rendered = evaluate(
            &CompiledFormat::compile(code),
            CellValue::Number(1.0),
            DateSystem::Windows1900,
        );
        assert_eq!(rendered.colour, None, "{code:?} invented a colour");
    }
}

/// A serial with no date falls back to a number rather than to an invented year.
#[test]
fn a_serial_with_no_date_degrades_to_a_number() {
    for serial in [-1.0, -45719.0, 2_958_466.0, 1.0e12] {
        let rendered = evaluate(
            &CompiledFormat::compile("yyyy-mm-dd"),
            CellValue::Number(serial),
            DateSystem::Windows1900,
        );
        assert!(
            !rendered.text.contains('-') || rendered.text.starts_with('-'),
            "{serial} rendered {:?}, which reads as a date it does not have",
            rendered.text
        );
        assert!(!rendered.text.is_empty(), "{serial} rendered nothing");
    }
}

/// A code that is only literals never acquires a minus sign it did not ask for.
#[test]
fn a_literal_section_does_not_acquire_a_sign() {
    let rendered = evaluate(
        &CompiledFormat::compile("\"n/a\""),
        CellValue::Number(-5.0),
        DateSystem::Windows1900,
    );
    assert_eq!(rendered.text, "n/a");
}
