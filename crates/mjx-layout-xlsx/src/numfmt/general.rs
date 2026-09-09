//! `General`, and the fifteen-significant-digit display Excel does its arithmetic behind.
//!
//! # ⚠ Fifteen digits is a requirement, not a rounding convenience
//!
//! Excel stores a `f64` and *displays* it as though it held fifteen significant decimal digits. That
//! is why `=0.1+0.2` shows `0.3` in a cell and `=(0.1+0.2)=0.3` is `FALSE` in the next one: the
//! comparison sees the binary value and the display sees the fifteen-digit decimal.
//!
//! [`to_display_precision`] is that step, and every value entering the evaluator goes through it
//! exactly once. It is also what makes rounding *decimal*: `2.675` is stored as
//! `2.67499999999999982…`, and a renderer that rounded the binary value to two places would print
//! `2.67` where Excel prints `2.68`. Rounding the fifteen-digit decimal instead gives Excel's answer
//! for the same reason Excel gives it.
//!
//! `tests/the_excel_quirks_are_reproduced.rs` asserts both, and a renderer that "fixes" either fails
//! there deliberately.
//!
//! # ⚠ `General` is width-dependent in Excel and is not here
//!
//! Excel fits `General` to the column: `1/3` is `0.333333` in a default-width column and shows more
//! digits as the column is widened, and a number too wide for its column becomes `#######`. This
//! engine turns a value into a string with no cell in scope, so it cannot do that, and pretending
//! otherwise would put a *width* in the cache key of every formatted value on the sheet.
//!
//! GUESS: eleven significant digits, and scientific notation outside `1e-4 ..< 1e11`, which is what
//! a default-width column shows. The Windows sitting is what settles whether the width dependence is
//! worth carrying into the layout tier.

/// How many significant decimal digits Excel displays a stored `f64` to.
pub const DISPLAY_SIGNIFICANT_DIGITS: usize = 15;

/// How many significant digits `General` writes.
///
/// GUESS: see the [module documentation](self).
pub const GENERAL_SIGNIFICANT_DIGITS: i32 = 11;

/// The magnitude at or above which `General` switches to scientific notation.
///
/// GUESS: `1e11` is where an eleven-digit integer stops fitting in eleven characters.
pub const GENERAL_SCIENTIFIC_CEILING: f64 = 1e11;

/// The magnitude below which `General` switches to scientific notation.
///
/// GUESS: `0.0001` displays as `0.0001` and `0.00001` displays as `1E-05`.
pub const GENERAL_SCIENTIFIC_FLOOR: f64 = 1e-4;

/// How many significant digits `General`'s scientific form writes.
///
/// GUESS: `1.23457E+11` is six, which is what fits beside a three-character exponent.
const GENERAL_SCIENTIFIC_DIGITS: i32 = 6;

/// `value`, rounded to the fifteen significant decimal digits Excel displays.
///
/// The round trip through a decimal string is the point rather than an implementation detail: it is
/// what makes `0.1 + 0.2` exactly `0.3` for every later step, which is what Excel shows.
#[must_use]
pub fn to_display_precision(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    let digits = DISPLAY_SIGNIFICANT_DIGITS.saturating_sub(1);
    format!("{value:.digits$e}").parse::<f64>().unwrap_or(value)
}

/// `value` as `General` renders it, without a sign.
///
/// The caller supplies the sign: every section in this engine is rendered from a magnitude and
/// prefixed, so that `-` lands in front of a currency symbol rather than behind it.
#[must_use]
pub fn render(value: f64) -> String {
    let magnitude = value.abs();
    if !magnitude.is_finite() {
        return magnitude.to_string();
    }
    if magnitude == 0.0 {
        return "0".to_owned();
    }
    if magnitude >= GENERAL_SCIENTIFIC_CEILING || magnitude < GENERAL_SCIENTIFIC_FLOOR {
        return scientific(magnitude);
    }
    decimal_string(magnitude, GENERAL_SIGNIFICANT_DIGITS)
}

/// `value` as `General` renders it, with its sign.
///
/// What the degradations reach for: a malformed code, a serial with no date, a conditional code no
/// section matched. [`render`] is the one the element writer uses, because a section's `-` is
/// written in front of the whole section rather than in front of its digits.
#[must_use]
pub fn render_signed(value: f64) -> String {
    if value < 0.0 {
        let mut out = String::from("-");
        out.push_str(&render(value));
        return out;
    }
    render(value)
}

/// `magnitude` in `General`'s scientific form — `1.23457E+11`.
fn scientific(magnitude: f64) -> String {
    let exponent = magnitude.abs().log10().floor();
    #[allow(clippy::cast_possible_truncation)]
    let exponent = exponent as i32;
    let mantissa = magnitude / 10_f64.powi(exponent);
    let digits = decimal_string(mantissa, GENERAL_SCIENTIFIC_DIGITS);
    let sign = if exponent < 0 { '-' } else { '+' };
    format!("{digits}E{sign}{:02}", exponent.abs())
}

/// `magnitude` to `significant` significant digits, positional, with trailing zeros removed.
#[must_use]
pub fn decimal_string(magnitude: f64, significant: i32) -> String {
    if magnitude == 0.0 {
        return "0".to_owned();
    }
    #[allow(clippy::cast_possible_truncation)]
    let exponent = magnitude.abs().log10().floor() as i32;
    let decimals = usize::try_from((significant - 1 - exponent).clamp(0, 30)).unwrap_or(0);
    let mut text = format!("{magnitude:.decimals$}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text.is_empty() {
        text.push('0');
    }
    text
}
