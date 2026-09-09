//! The number-format engine: `numFmt@formatCode` applied to a cell's stored value.
//!
//! # What this is for
//!
//! A worksheet stores `45678`, not `04/03/2025`, and `1234.5`, not `$1,234.50`. Turning the first
//! into the second is the single most visible thing a spreadsheet does, and it is the one place a
//! renderer can be wrong in a way nobody needs a reference to see.
//!
//! `mjx-sml` already answers *which* format code is in force — the `numFmts` table, the eight-and-a-
//! half implied built-in tables of ECMA-376 Part 1 §18.8.30, and the `xf` indirection that reaches
//! them. **This module is the evaluator and holds none of that.**
//! [`mjx_sml::builtin_format_code`] is the table; `super::model::number_format_of` is the resolver;
//! `tests/the_ladder_is_consumed.rs` refuses a second copy of either.
//!
//! # The shape of the language
//!
//! A format code is up to four `;`-separated sections. Which one runs depends on the value, and on
//! whether the code states bracketed conditions:
//!
//! | sections | positive | negative | zero | text |
//! |---|---|---|---|---|
//! | 1 | 1 | 1 | 1 | verbatim |
//! | 2 | 1 | **2, unsigned** | 1 | verbatim |
//! | 3 | 1 | **2, unsigned** | 3 | verbatim |
//! | 4 | 1 | **2, unsigned** | 3 | 4 |
//!
//! A code with `[>100]`-style conditions replaces that table entirely: sections are tried in order
//! and the first whose condition holds wins, with an unconditioned section acting as the *otherwise*.
//!
//! GUESS: that the second *positional* section renders the value unsigned **whether or not it
//! carries a condition**. It is what makes `[<0]"("0.00")"` write `(5.00)` rather than `(-5.00)`,
//! which is plainly what such a code is for; Excel's exact rule for a conditioned second section has
//! not been observed on Windows.
//!
//! GUESS: that a conditional code no section matches falls back to `General`. Excel fills the cell
//! with `#` instead, which needs a *width*, and a width in this engine would put the column's
//! geometry into the cache key of every formatted value on the sheet.
//!
//! # What is implemented, and what deliberately is not
//!
//! Implemented: the three digit placeholders and their padding, decimal points, thousands grouping,
//! trailing-comma scaling, percentages, scientific and engineering notation, fractions with both
//! variable and fixed denominators, quoted and escaped literals, `_` skips, `*` fills (reported,
//! see [`FormattedValue::repeat`]), `@` text substitution, `General`, the eight colour names and
//! `[ColorN]`, bracketed conditions, `[$…]` currency and locale prefixes, every date and time token,
//! `AM/PM` in four spellings, sub-seconds, elapsed `[h]`/`[mm]`/`[ss]`, both date systems and the
//! 1900 leap-year bug.
//!
//! Deliberately not implemented, and each degrades to a literal or to `General` rather than failing:
//!
//! * **Localised month and weekday names.** English only — see [`datetime`] for why a locale
//!   database is not linked in to answer a question `[$-40C]` does not actually ask.
//! * **The Japanese era calendar** (`g`, `gg`, `ggg`, `e`) and the **Thai Buddhist calendar**
//!   (`b`, `bb`). Both need a second calendar, not a second token.
//! * **`*` expansion.** The fill character is reported; expanding it needs a cell width.
//! * **`#######` overflow.** Same reason.
//! * **`_`'s true width.** One space, because the real answer is a glyph advance.
//!
//! # Nothing here panics and nothing here fails
//!
//! A malformed format code is common in real files, and a cell whose style is odd must still draw.
//! [`parse::CompiledFormat::compile`] is total, every renderer is total, and
//! `tests/a_broken_format_degrades.rs` walks a fixture of genuinely broken codes to hold it.
//!
//! # ⚠ Nothing here is parity with Excel
//!
//! Every behaviour chosen rather than read is marked `GUESS:` at its site. Confirmation is a human
//! sitting against real Microsoft Excel on Windows (`docs/validation/07-the-reference-pack.md`);
//! LibreOffice is a change detector and not a reference.

pub mod cache;
pub mod datetime;
pub mod general;
pub mod number;
pub mod parse;

pub use cache::FormatCache;
pub use datetime::{CivilDateTime, DateToken, MeridiemStyle};
pub use parse::{
    Comparison, CompiledFormat, Condition, Denominator, Element, Placeholder, Section, SectionKind,
};

use mjx_ooxml_types::spreadsheetml::CellType;
use mjx_xlsx::DateSystem;

/// What a cell holds, as the evaluator sees it.
///
/// Formula evaluation does not exist in this loop: a formula cell's **cached** value is what is
/// formatted, which is what a viewer wants and what the file always carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellValue<'a> {
    /// A number — including every date, time and currency, all of which are numbers with a format.
    Number(f64),
    /// Text, from a shared string, an inline string or a formula's string result.
    Text(&'a str),
    /// `TRUE` or `FALSE`.
    Boolean(bool),
    /// An error code, as the file spells it — `#DIV/0!`, `#N/A`.
    Error(&'a str),
}

impl<'a> CellValue<'a> {
    /// What a cell of `cell_type` whose display text is `text` holds.
    ///
    /// A `<c t="n">` whose `<v>` will not parse as a number is the file's own error, and showing the
    /// characters it wrote is a better answer than showing nothing.
    #[must_use]
    pub fn read(cell_type: CellType, text: &'a str) -> Self {
        match cell_type {
            CellType::Number => match text.trim().parse::<f64>() {
                Ok(value) if value.is_finite() => Self::Number(value),
                _ => Self::Text(text),
            },
            CellType::Boolean => Self::Boolean(text.trim() != "0"),
            CellType::Error => Self::Error(text),
            CellType::SharedString | CellType::FormulaString | CellType::InlineString => {
                Self::Text(text)
            }
        }
    }

    /// Whether this is a number, which is what decides a `general` cell's alignment.
    #[must_use]
    pub fn is_number(self) -> bool {
        matches!(self, Self::Number(_))
    }
}

/// What formatting one value produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FormattedValue {
    /// The characters a reader sees.
    pub text: String,
    /// `[Red]`, `[Color12]` — the **zero-based** row of the legacy indexed palette the text is drawn
    /// in, or `None` when the format names no colour.
    ///
    /// A colour states itself per *value* rather than per format — the negative section of
    /// `#,##0;[Red]#,##0` colours only the negative cells — so it cannot live on a decoration shared
    /// by every cell of one effective format. See [`crate::Decoration::text_colour`].
    pub colour: Option<u32>,
    /// `*c` — the character the format asked to repeat until the cell is full.
    ///
    /// Reported rather than expanded: the width it would fill is a *cell's*, and a formatted string
    /// that depended on a column width could not be cached by `(format, value)`. Nothing draws it
    /// today; see [the module documentation](self).
    pub repeat: Option<char>,
}

impl FormattedValue {
    /// A value that rendered as `text` in no particular colour.
    #[must_use]
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            colour: None,
            repeat: None,
        }
    }
}

/// Formats `value` through `format` under `dates`.
///
/// The whole engine in one call, with no cache. [`FormatCache::format`] is the one a layout path
/// wants; this is what it calls on a miss, and what a test that wants no caching in the way calls
/// directly.
#[must_use]
pub fn evaluate(
    format: &CompiledFormat,
    value: CellValue<'_>,
    dates: DateSystem,
) -> FormattedValue {
    match value {
        // A boolean and an error ignore the number format entirely. That is not a simplification:
        // Excel shows `TRUE` in a cell formatted as currency, because the format language has no
        // token that reads either of them.
        CellValue::Boolean(state) => FormattedValue::plain(if state { "TRUE" } else { "FALSE" }),
        CellValue::Error(text) => FormattedValue::plain(text),
        CellValue::Text(text) => format_text(format, text),
        CellValue::Number(raw) => format_number(format, raw, dates),
    }
}

/// Renders a text value.
fn format_text(format: &CompiledFormat, text: &str) -> FormattedValue {
    let sections = format.sections();
    // The fourth section is the text section, and only a code that writes all four has one. A code
    // with fewer says nothing about text, and text passes through — which is why a column of labels
    // under a currency format still reads.
    let Some(section) = sections.get(3) else {
        return FormattedValue::plain(text);
    };
    if section.is_empty() {
        return FormattedValue {
            text: String::new(),
            colour: section.colour,
            repeat: section.repeat,
        };
    }
    let mut out = String::new();
    for element in &section.elements {
        match element {
            Element::Literal(literal) => out.push_str(literal),
            Element::TextValue | Element::General => out.push_str(text),
            Element::Percent => out.push('%'),
            Element::Skip(_) => out.push(' '),
            _ => {}
        }
    }
    FormattedValue {
        text: out,
        colour: section.colour,
        repeat: section.repeat,
    }
}

/// Renders a numeric value, choosing its section first.
fn format_number(format: &CompiledFormat, raw: f64, dates: DateSystem) -> FormattedValue {
    // ⚠ The fifteen-digit clamp happens **once**, here, before anything reads a digit. See
    // `general`'s documentation for why it is a requirement rather than a rounding convenience.
    let value = general::to_display_precision(raw);
    let Some((section, unsigned)) = select(format, value) else {
        // GUESS: a conditional code that matches nothing falls back to `General`; Excel fills the
        // cell with `#`. See [the module documentation](self).
        return FormattedValue::plain(general::render_signed(value));
    };
    if section.is_empty() {
        return FormattedValue {
            text: String::new(),
            colour: section.colour,
            repeat: section.repeat,
        };
    }
    FormattedValue {
        text: number::render(section, value, unsigned, dates),
        colour: section.colour,
        repeat: section.repeat,
    }
}

/// Which section `value` renders through, and whether its sign is stripped.
///
/// The table in the [module documentation](self), as a function. `None` when the code is conditional
/// and no section matches — which is the one case that has no answer rather than a chosen one.
///
/// Public because it is the part of the contract a reader is most likely to want to check
/// independently: *which* section a value reaches is a different question from what that section
/// renders, and a caller debugging a surprising cell wants to separate them.
#[must_use]
pub fn select(format: &CompiledFormat, value: f64) -> Option<(&Section, bool)> {
    let sections = format.sections();
    if format.is_conditional() {
        // First match wins, and a section with no condition is the *otherwise*. Only the first three
        // are candidates: the fourth is the text section and a number never reaches it.
        for (index, section) in sections.iter().take(3).enumerate() {
            let matched = section
                .condition
                .is_none_or(|condition| condition.holds(value));
            if matched {
                return Some((section, index == 1));
            }
        }
        return None;
    }
    let negative = value < 0.0;
    let zero = value == 0.0;
    match sections.len() {
        0 => None,
        1 => sections.first().map(|section| (section, false)),
        2 => {
            if negative {
                sections.get(1).map(|section| (section, true))
            } else {
                sections.first().map(|section| (section, false))
            }
        }
        _ => {
            if negative {
                sections.get(1).map(|section| (section, true))
            } else if zero {
                sections.get(2).map(|section| (section, false))
            } else {
                sections.first().map(|section| (section, false))
            }
        }
    }
}
