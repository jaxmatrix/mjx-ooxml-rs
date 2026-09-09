//! The numeric renderer: a magnitude, a [`Section`], and the string Excel writes for the pair.
//!
//! # Rounding is decimal, and that is the whole design
//!
//! Every value reaching here has already been through
//! [`super::general::to_display_precision`], so the number this
//! module works with *is* the fifteen-digit decimal a person sees in the formula bar. It is then
//! rounded **as a digit string**, half away from zero, rather than with `format!("{:.n}")`.
//!
//! Both halves matter. `format!` rounds ties to even, so `0.125` to two places is `0.12`, and Excel
//! writes `0.13`. And rounding the *binary* value would make `2.675` to two places `2.67`, because
//! the stored double is `2.67499999999999982…`, where Excel writes `2.68` because its display
//! rounds the decimal. Doing the arithmetic on the digits gives Excel's answer for Excel's reason.
//!
//! # Digits are distributed right to left, and the leftmost placeholder takes the rest
//!
//! `000-0000` on `1234567` is `123-4567`, which only works if each digit position is filled
//! separately and the literals between them are kept — so the assignment is per placeholder rather
//! than "write the number where the first `#` was". The leftmost placeholder receives every digit
//! the pattern has no room for, which is why `00` on `12345` is `12345` and not `45`.
//!
//! Thousands separators travel with the digit that follows them, which is what puts the comma of
//! `#,##0` in the right place when the placeholder count and the digit count disagree.

use super::datetime::{civil_from_serial, render_token};
use super::general;
use super::parse::{Denominator, Element, Placeholder, Section, SectionKind};
use mjx_xlsx::DateSystem;

/// The largest denominator a `?/?` fraction will search for.
///
/// Nine placeholders is already far past anything a person writes; the bound exists so that a
/// malformed code of forty `?` characters cannot ask for a search over a number that does not fit.
const MAX_FRACTION_PLACEHOLDERS: u32 = 9;

/// How wide `Element::Skip` renders.
///
/// GUESS: `_)` reserves the width of a `)` and this writes one space. Reproducing it exactly needs
/// the *glyph advance* of the skipped character, which is a measurement rather than a string, and
/// would make the formatted text depend on the font — see [`super`] for why that seam is not
/// crossed here.
const SKIP_WIDTH: &str = " ";

/// Renders `value` through `section`.
///
/// `unsigned` says whether the caller chose the **positional negative** section, which renders the
/// value's magnitude and leaves the sign to the parentheses or the literal minus the code writes for
/// itself. Everywhere else the sign is written, and it is written in front of the *whole* rendered
/// section — which is what puts the `-` of `-$5.00` before the currency sign rather than after it.
///
/// GUESS: that placement. Excel's own accounting formats state the sign themselves with parentheses,
/// so the single-section case — `$#,##0.00` against a negative — is a reading rather than an
/// observation.
///
/// The sign reaches the **date** branch too, and has to: a negative serial names no day in either
/// epoch, so a renderer handed a magnitude would print `1900-01-01` for `-1`.
#[must_use]
pub fn render(section: &Section, value: f64, unsigned: bool, dates: DateSystem) -> String {
    let signed = if unsigned { value.abs() } else { value };
    if section.kind == SectionKind::DateTime {
        return render_datetime(section, signed, dates);
    }
    let negative = signed < 0.0;
    let magnitude = signed.abs();
    let mut scaled = magnitude;
    for _ in 0..section.percent_multiplier {
        scaled *= 100.0;
    }
    for _ in 0..section.scale_by_thousands {
        scaled /= 1000.0;
    }
    // The fifteen-digit clamp runs again, on the scaled value, and it has to. `0.12345` under
    // `0.00%` is `0.12345 * 100`, which in binary is `12.344999999999999`; rounding that to two
    // places writes `12.34` where Excel writes `12.35`. Excel's display precision applies to the
    // quantity being shown, not only to the one that was stored.
    let scaled = general::to_display_precision(scaled);
    let body = match section.kind {
        SectionKind::Scientific => render_scientific(section, scaled),
        SectionKind::Fraction => render_fraction(section, scaled),
        _ => render_fixed(section, scaled),
    };
    if negative && writes_a_number(section) {
        let mut out = String::with_capacity(body.len() + 1);
        out.push('-');
        out.push_str(&body);
        return out;
    }
    body
}

/// Whether the section actually writes the value, and therefore whether a `-` in front of it means
/// anything.
///
/// A section of pure literal text — `"n/a"` — is not a number and does not acquire a minus sign
/// because the value happened to be negative.
fn writes_a_number(section: &Section) -> bool {
    section.elements.iter().any(|element| {
        matches!(
            element,
            Element::IntegerDigit(_)
                | Element::DecimalDigit(_)
                | Element::NumeratorDigit(_)
                | Element::DenominatorDigit(_)
                | Element::ExponentDigit(_)
                | Element::General
        )
    })
}

/// Renders a fixed-point section.
fn render_fixed(section: &Section, magnitude: f64) -> String {
    let (integer, fraction) = split_and_round(magnitude, section.decimal_places);
    emit(section, &integer, &fraction, None, magnitude)
}

/// Renders a scientific section — the mantissa through the ordinary machinery, then the exponent.
fn render_scientific(section: &Section, magnitude: f64) -> String {
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    let places = section.integer_places.clamp(1, 9) as i32;
    let mut exponent = 0_i32;
    let mut mantissa = magnitude;
    if magnitude != 0.0 && magnitude.is_finite() {
        #[allow(clippy::cast_possible_truncation)]
        let order = magnitude.abs().log10().floor() as i32;
        // `##0.0E+0` is engineering notation: three integer placeholders means the exponent moves in
        // threes, which is what puts `12345` at `12.3E+3` rather than at `1.2E+4`.
        exponent = (f64::from(order) / f64::from(places)).floor() as i32 * places;
        mantissa = magnitude / 10_f64.powi(exponent);
    }
    let (mut integer, mut fraction) = split_and_round(mantissa, section.decimal_places);
    // Rounding can carry the mantissa past the pattern's width — `9.99` to one place is `10.0` — in
    // which case the exponent takes the carry instead.
    if integer.len() > usize::try_from(places).unwrap_or(1) {
        exponent += places;
        mantissa = magnitude / 10_f64.powi(exponent);
        let split = split_and_round(mantissa, section.decimal_places);
        integer = split.0;
        fraction = split.1;
    }
    emit(section, &integer, &fraction, Some(exponent), magnitude)
}

/// Renders a fraction section.
fn render_fraction(section: &Section, magnitude: f64) -> String {
    let limit = match section.denominator {
        Denominator::Fixed(fixed) => u64::from(fixed.max(1)),
        Denominator::Placeholders(places) => 10_u64
            .pow(places.clamp(1, MAX_FRACTION_PLACEHOLDERS))
            .saturating_sub(1)
            .max(1),
    };
    let fixed = matches!(section.denominator, Denominator::Fixed(_));
    // `??/??` with no integer placeholder states the whole value as one improper fraction: `1.25`
    // is `5/4`, not `1 1/4`.
    if section.integer_places == 0 {
        let (numerator, denominator) = approximate(magnitude, limit, fixed);
        return emit_fraction(section, "", numerator, denominator);
    }
    let mut whole = magnitude.trunc();
    let (mut numerator, denominator) = approximate(magnitude - whole, limit, fixed);
    if denominator > 0 && numerator >= denominator {
        // A remainder that rounded up to a whole one belongs in the integer part; `# ?/?` on `2.99`
        // is `3` rather than `2 1/1`.
        let carried = numerator / denominator;
        #[allow(clippy::cast_precision_loss)]
        {
            whole += carried as f64;
        }
        numerator -= carried * denominator;
    }
    let integer = if whole == 0.0 {
        String::new()
    } else {
        general::decimal_string(whole, 15)
    };
    emit_fraction(section, &integer, numerator, denominator)
}

/// `value` over `limit` exactly, or as its best approximation with a denominator no larger.
fn approximate(value: f64, limit: u64, fixed: bool) -> (u64, u64) {
    if fixed {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let numerator = (value * limit as f64).round().max(0.0) as u64;
        return (numerator, limit);
    }
    best_rational(value, limit)
}

/// The best rational approximation of `value` with a denominator no larger than `limit`.
///
/// A continued-fraction expansion, which is what makes `0.3333333` render as `1/3` under `?/?` and
/// as `333/1000`-shaped answers under wider patterns. Stops as soon as the denominator would pass
/// the limit, so the cost is bounded by the number of terms rather than by the value.
fn best_rational(value: f64, limit: u64) -> (u64, u64) {
    if !value.is_finite() || value <= 0.0 {
        let _ = limit;
        return (0, 1);
    }
    let (mut previous_numerator, mut previous_denominator) = (0_u64, 1_u64);
    let (mut numerator, mut denominator) = (1_u64, 0_u64);
    let mut remainder = value;
    // Sixty-four terms is far past the point where a `f64` has any information left; the loop
    // normally stops on the denominator bound one or two terms in.
    for _ in 0..64 {
        let whole = remainder.floor();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let whole_part = whole.max(0.0).min(u64::MAX as f64) as u64;
        let next_numerator = whole_part
            .checked_mul(numerator)
            .and_then(|product| product.checked_add(previous_numerator));
        let next_denominator = whole_part
            .checked_mul(denominator)
            .and_then(|product| product.checked_add(previous_denominator));
        let (Some(next_numerator), Some(next_denominator)) = (next_numerator, next_denominator)
        else {
            break;
        };
        if next_denominator > limit {
            break;
        }
        previous_numerator = numerator;
        previous_denominator = denominator;
        numerator = next_numerator;
        denominator = next_denominator;
        let fraction = remainder - whole;
        if fraction.abs() < 1e-12 {
            break;
        }
        remainder = 1.0 / fraction;
    }
    if denominator == 0 {
        return (0, 1);
    }
    (numerator, denominator)
}

/// Writes a fraction section's elements.
fn emit_fraction(section: &Section, integer: &str, numerator: u64, denominator: u64) -> String {
    let blank = numerator == 0 && !integer.is_empty();
    let integer_slots = assign_integer(section, integer);
    // The numerator is padded on the **left** and the denominator on the right, which is what makes
    // a column of `# ??/??` line its bars up.
    let numerator_text = numerator.to_string();
    let numerator_slots = assign_from_the_right(
        section.elements.iter().filter_map(|element| match element {
            Element::NumeratorDigit(placeholder) => Some(*placeholder),
            _ => None,
        }),
        &numerator_text,
    );
    let denominator_text = denominator.to_string();
    let denominator_slots = assign_run(
        section.elements.iter().filter_map(|element| match element {
            Element::DenominatorDigit(placeholder) => Some(*placeholder),
            _ => None,
        }),
        &denominator_text,
    );
    let mut out = String::new();
    let mut integer_at = 0;
    let mut numerator_at = 0;
    let mut denominator_at = 0;
    for element in &section.elements {
        match element {
            Element::IntegerDigit(_) => {
                push_slot(&mut out, &integer_slots, &mut integer_at);
            }
            Element::NumeratorDigit(_) => {
                if blank {
                    out.push(' ');
                    numerator_at += 1;
                } else {
                    push_slot(&mut out, &numerator_slots, &mut numerator_at);
                }
            }
            Element::DenominatorDigit(_) => {
                if blank {
                    out.push(' ');
                    denominator_at += 1;
                } else {
                    push_slot(&mut out, &denominator_slots, &mut denominator_at);
                }
            }
            Element::FractionBar => {
                // GUESS: a whole number under `# ?/?` blanks its fraction rather than writing
                // `0/1`. Excel shows `3` followed by the width the fraction would have taken, which
                // is what a column of fractions lines up against.
                out.push(if blank { ' ' } else { '/' });
            }
            Element::FixedDenominator(digits) => {
                if blank {
                    out.push_str(&" ".repeat(digits.chars().count()));
                } else {
                    out.push_str(digits);
                }
            }
            other => push_other(&mut out, section, other, 0.0, None),
        }
    }
    out
}

/// Splits `magnitude` into its integer and fraction digit strings, rounded to `decimals` places.
///
/// Half away from zero, on the digits. See the [module documentation](self).
fn split_and_round(magnitude: f64, decimals: usize) -> (String, String) {
    let magnitude = magnitude.abs();
    if !magnitude.is_finite() {
        return (String::new(), String::new());
    }
    // Rust's `Display` for `f64` is positional and shortest-round-trip, which is exactly the
    // fifteen-digit decimal this engine has already narrowed the value to.
    let text = format!("{magnitude}");
    let (integer, fraction) = match text.split_once('.') {
        Some((integer, fraction)) => (integer.to_owned(), fraction.to_owned()),
        None => (text, String::new()),
    };
    let (mut integer, mut fraction) = round_digits(&integer, &fraction, decimals);
    while fraction.len() < decimals {
        fraction.push('0');
    }
    // A leading zero is not a digit the pattern has to place: `#.##` on `0.5` is `.5`, and `0.00`
    // gets its zero from the placeholder rather than from the value.
    let trimmed = integer.trim_start_matches('0');
    integer = trimmed.to_owned();
    (integer, fraction)
}

/// Rounds the digit strings `integer`.`fraction` to `decimals` places, half away from zero.
fn round_digits(integer: &str, fraction: &str, decimals: usize) -> (String, String) {
    if fraction.len() <= decimals {
        return (integer.to_owned(), fraction.to_owned());
    }
    let mut digits: Vec<u8> = integer
        .bytes()
        .chain(fraction.bytes())
        .map(|byte| byte.wrapping_sub(b'0'))
        .collect();
    let keep = integer.len() + decimals;
    let rounds_up = fraction
        .as_bytes()
        .get(decimals)
        .is_some_and(|byte| *byte >= b'5');
    digits.truncate(keep);
    if rounds_up {
        let mut at = keep;
        loop {
            if at == 0 {
                digits.insert(0, 1);
                return (
                    digits_to_string(&digits[..integer.len() + 1]),
                    digits_to_string(&digits[integer.len() + 1..]),
                );
            }
            at -= 1;
            if digits[at] == 9 {
                digits[at] = 0;
            } else {
                digits[at] += 1;
                break;
            }
        }
    }
    (
        digits_to_string(&digits[..integer.len()]),
        digits_to_string(&digits[integer.len()..]),
    )
}

/// Turns a slice of decimal digit values back into a string.
fn digits_to_string(digits: &[u8]) -> String {
    digits
        .iter()
        .map(|digit| char::from(b'0' + digit.min(&9)))
        .collect()
}

/// Groups an integer digit string in threes.
fn group(integer: &str) -> String {
    let digits: Vec<char> = integer.chars().collect();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.iter().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*digit);
    }
    out
}

/// Fills the section's integer placeholders from `integer`, right to left.
fn assign_integer(section: &Section, integer: &str) -> Vec<String> {
    let grouped = if section.grouped {
        group(integer)
    } else {
        integer.to_owned()
    };
    assign_from_the_right(
        section.elements.iter().filter_map(|element| match element {
            Element::IntegerDigit(placeholder) => Some(*placeholder),
            _ => None,
        }),
        &grouped,
    )
}

/// Fills `placeholders` from the right of `digits`, giving the leftmost every digit left over.
fn assign_from_the_right(
    placeholders: impl Iterator<Item = Placeholder>,
    digits: &str,
) -> Vec<String> {
    let placeholders: Vec<Placeholder> = placeholders.collect();
    let mut slots = vec![String::new(); placeholders.len()];
    let mut remaining: Vec<char> = digits.chars().collect();
    for index in (0..placeholders.len()).rev() {
        if index == 0 {
            let rest: String = remaining.iter().collect();
            slots[0] = if rest.is_empty() {
                placeholders[0].padding().to_owned()
            } else {
                rest
            };
            remaining.clear();
            break;
        }
        // A separator sits to the right of the digit it follows, so it is taken with that digit.
        let mut tail = String::new();
        while remaining.last().is_some_and(|ch| !ch.is_ascii_digit()) {
            if let Some(ch) = remaining.pop() {
                tail.insert(0, ch);
            }
        }
        match remaining.pop() {
            Some(digit) => {
                let mut slot = String::with_capacity(1 + tail.len());
                slot.push(digit);
                slot.push_str(&tail);
                slots[index] = slot;
            }
            None => slots[index] = placeholders[index].padding().to_owned(),
        }
    }
    slots
}

/// Fills `placeholders` from the left of `digits` — what a decimal or a denominator run does.
fn assign_run(placeholders: impl Iterator<Item = Placeholder>, digits: &str) -> Vec<String> {
    let placeholders: Vec<Placeholder> = placeholders.collect();
    let mut characters = digits.chars();
    placeholders
        .iter()
        .map(|placeholder| match characters.next() {
            Some(digit) => digit.to_string(),
            None => placeholder.padding().to_owned(),
        })
        .collect()
}

/// Appends the next slot, advancing the cursor past the end without panicking.
fn push_slot(out: &mut String, slots: &[String], at: &mut usize) {
    if let Some(slot) = slots.get(*at) {
        out.push_str(slot);
    }
    *at += 1;
}

/// Writes the elements of a fixed-point or scientific section.
fn emit(
    section: &Section,
    integer: &str,
    fraction: &str,
    exponent: Option<i32>,
    magnitude: f64,
) -> String {
    let integer_slots = assign_integer(section, integer);
    // A trailing zero in the fraction is padding, never information: `#.##` on `0.5` is `.5` and
    // `0.00` on `0.5` is `0.50`, and the difference is entirely in what the *placeholder* writes
    // when the value hands it no digit. Stripping here and letting each placeholder answer for
    // itself is what makes `0`, `?` and `#` differ at all.
    let significant = fraction.trim_end_matches('0');
    let decimal_slots = assign_run(
        section.elements.iter().filter_map(|element| match element {
            Element::DecimalDigit(placeholder) => Some(*placeholder),
            _ => None,
        }),
        significant,
    );
    let mut out = String::new();
    let mut integer_at = 0;
    let mut decimal_at = 0;
    for element in &section.elements {
        match element {
            Element::IntegerDigit(_) => push_slot(&mut out, &integer_slots, &mut integer_at),
            Element::DecimalDigit(_) => push_slot(&mut out, &decimal_slots, &mut decimal_at),
            Element::DecimalPoint => {
                // `.00` states no integer placeholder at all, and the integer digits still have to
                // go somewhere: `.00` on `3.14159` is `3.14`.
                if section.integer_places == 0 && !integer.is_empty() {
                    out.push_str(&if section.grouped {
                        group(integer)
                    } else {
                        integer.to_owned()
                    });
                }
                out.push('.');
            }
            Element::Exponent { positive_sign } => {
                out.push('E');
                let exponent = exponent.unwrap_or(0);
                if exponent < 0 {
                    out.push('-');
                } else if *positive_sign {
                    out.push('+');
                }
                let width = section.exponent_places.max(1);
                out.push_str(&format!("{:0width$}", exponent.abs()));
            }
            // The exponent's digits were written whole by the element above; the placeholders that
            // stated their width are consumed here so they do not write themselves a second time.
            Element::ExponentDigit(_) => {}
            other => push_other(&mut out, section, other, magnitude, exponent),
        }
    }
    out
}

/// Writes the elements every renderer shares.
fn push_other(
    out: &mut String,
    section: &Section,
    element: &Element,
    magnitude: f64,
    _exponent: Option<i32>,
) {
    match element {
        Element::Literal(text) => out.push_str(text),
        Element::Percent => out.push('%'),
        Element::Skip(_) => out.push_str(SKIP_WIDTH),
        // GUESS, and a reported one: `*c` repeats `c` until the cell is full, and this engine has no
        // cell. The character reaches the caller on `FormattedValue::repeat` instead of being
        // expanded to a width that would be wrong.
        Element::Repeat(_) => {}
        // A number in a section whose only content is `@` — the built-in `Text` format, id 49 — is
        // shown as `General`. `@` substitutes a *string*, and a number has none.
        Element::TextValue | Element::General => out.push_str(&general::render(magnitude)),
        Element::FixedDenominator(_)
        | Element::Date(_)
        | Element::IntegerDigit(_)
        | Element::DecimalDigit(_)
        | Element::DecimalPoint
        | Element::NumeratorDigit(_)
        | Element::DenominatorDigit(_)
        | Element::FractionBar
        | Element::Exponent { .. }
        | Element::ExponentDigit(_) => {
            let _ = section;
        }
    }
}

/// Renders a date or time section.
///
/// A serial with no date — negative, or past 9999-12-31 — falls back to `General`, which is the
/// degradation this engine makes everywhere rather than refusing a cell.
fn render_datetime(section: &Section, serial: f64, dates: DateSystem) -> String {
    let digits = section
        .elements
        .iter()
        .filter_map(|element| match element {
            Element::Date(super::datetime::DateToken::SubSecond(digits)) => Some(*digits),
            _ => None,
        })
        .max()
        .unwrap_or(0);
    let Some(at) = civil_from_serial(serial, dates, digits) else {
        // No day in this epoch — a negative serial, or one past 9999-12-31. Excel fills the cell
        // with `#`; `General` is the degradation this engine makes everywhere, and it at least shows
        // the reader what is stored. See `super`'s documentation for why `#` needs a width.
        return general::render_signed(serial);
    };
    let mut out = String::new();
    for element in &section.elements {
        match element {
            Element::Date(token) => render_token(&mut out, *token, at, section.has_meridiem),
            other => push_other(&mut out, section, other, serial.abs(), None),
        }
    }
    out
}
