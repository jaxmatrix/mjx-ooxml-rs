//! The [`AttributeCodec`]s for the DrawingML measures — the wire ⇄ Rust conversion for the four
//! value kinds `a:` attributes are written in that no lower crate can name.
//!
//! A crate that owns a measure type owns its codec: [`Emu`], [`LineWidth`], [`Angle`] and
//! [`Fraction`] live in [`crate::geometry`], so their spellings live here. Everything *else* a
//! DrawingML attribute can be already has one — an enumeration is
//! [`Enumeration<T>`](mjx_ooxml_core::Enumeration) because all 568 generated variants spell
//! themselves, a boolean is [`OnOff`](mjx_ooxml_types::support::OnOff), a colour is
//! [`HexColorRgb`](mjx_ooxml_types::support::HexColorRgb), a plain string is
//! [`Text`](mjx_ooxml_core::Text) and a plain integer is [`Number<T>`](mjx_ooxml_core::Number).
//!
//! Every codec here reads **every spelling the schema accepts** and writes **exactly one**. That
//! asymmetry is the fidelity contract: a value nobody assigned to is never rewritten, because a read
//! borrows the file's own bytes and cannot change them, while a write goes through
//! [`AttributeCodec::encode`] and therefore has one canonical form.

use std::borrow::Cow;

use mjx_ooxml_core::{AttributeCodec, InvalidAttributeValue};

use crate::geometry::{Angle, Emu, Fraction, LineWidth};

/// How many 60,000ths of a degree there are in one degree — the unit every DrawingML angle
/// attribute is written in (`ST_Angle` and its fixed/positive restrictions).
const SIXTY_THOUSANDTHS_PER_DEGREE: f64 = 60_000.0;

/// The integer a whole `ST_Percentage` is written as: `100000` is 100 %.
const PERCENTAGE_SCALE: f64 = 100_000.0;

/// `ST_Coordinate` and its relatives (`ST_PositiveCoordinate`, `ST_AdjCoordinate`) as an [`Emu`] —
/// a signed 64-bit count of English Metric Units, 914,400 to the inch.
///
/// Wire token: a decimal integer, e.g. `a:off@x="914400"` (one inch) or `a:outerShdw@dist="38100"`
/// (three points). Surrounding whitespace is accepted on read, as `xsd:long` allows; the written
/// form never has any.
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::AttributeCodec;
/// use mjx_dml::codec::EmuCoordinate;
/// use mjx_dml::Emu;
///
/// assert_eq!(EmuCoordinate::decode(Cow::Borrowed("914400")), Ok(Emu::from_emu(914_400)));
/// assert_eq!(EmuCoordinate::encode(Emu::from_emu(-1)), "-1");
/// assert!(EmuCoordinate::decode(Cow::Borrowed("1.5")).is_err());
/// ```
#[derive(Debug)]
pub struct EmuCoordinate;

impl AttributeCodec for EmuCoordinate {
    type Value<'a> = Emu;
    type Input<'a> = Emu;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Emu, InvalidAttributeValue> {
        raw.trim()
            .parse::<i64>()
            .map(Emu::from_emu)
            .map_err(|error| {
                InvalidAttributeValue::new(format!("expected a whole number of EMU: {error}"))
            })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Owned(value.emu().to_string())
    }
}

/// `ST_LineWidth` as a [`LineWidth`] — the same EMU integer as [`EmuCoordinate`], carried by the one
/// attribute that has a type of its own for it.
///
/// Wire token: a decimal integer of EMU, e.g. `a:ln@w="12700"` (one point). The schema bounds it to
/// `0..=20116800`; the bound is documented rather than enforced, as everywhere else in this crate,
/// because a file may carry an out-of-range value and reading one must not fail.
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::AttributeCodec;
/// use mjx_dml::codec::EmuLineWidth;
/// use mjx_dml::LineWidth;
///
/// assert_eq!(EmuLineWidth::decode(Cow::Borrowed("12700")), Ok(LineWidth::from_emu(12_700)));
/// assert_eq!(EmuLineWidth::encode(LineWidth::from_points(1.0)), "12700");
/// ```
#[derive(Debug)]
pub struct EmuLineWidth;

impl AttributeCodec for EmuLineWidth {
    type Value<'a> = LineWidth;
    type Input<'a> = LineWidth;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<LineWidth, InvalidAttributeValue> {
        raw.trim()
            .parse::<i64>()
            .map(LineWidth::from_emu)
            .map_err(|error| {
                InvalidAttributeValue::new(format!("expected a whole number of EMU: {error}"))
            })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Owned(value.emu().to_string())
    }
}

/// The `ST_Angle` family (`ST_FixedAngle`, `ST_PositiveFixedAngle`) as an [`Angle`].
///
/// Wire token: **60,000ths of a degree** as a decimal number, so `a:xfrm@rot="2700000"` is 45° and
/// `a:outerShdw@dir="5400000"` is 90°. The written form rounds to the nearest 60,000th, which is the
/// finest distinction the format records.
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::AttributeCodec;
/// use mjx_dml::codec::SixtyThousandthsOfADegree;
/// use mjx_dml::Angle;
///
/// let forty_five = SixtyThousandthsOfADegree::decode(Cow::Borrowed("2700000")).expect("an angle");
/// assert!((forty_five.degrees() - 45.0).abs() < 1e-9);
/// assert_eq!(SixtyThousandthsOfADegree::encode(Angle::from_degrees(90.0)), "5400000");
/// ```
#[derive(Debug)]
pub struct SixtyThousandthsOfADegree;

impl AttributeCodec for SixtyThousandthsOfADegree {
    type Value<'a> = Angle;
    type Input<'a> = Angle;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Angle, InvalidAttributeValue> {
        raw.trim()
            .parse::<f64>()
            .map(|value| Angle::from_degrees(value / SIXTY_THOUSANDTHS_PER_DEGREE))
            .map_err(|error| {
                InvalidAttributeValue::new(format!(
                    "expected an angle in 60000ths of a degree: {error}"
                ))
            })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        let sixty_thousandths = (value.degrees() * SIXTY_THOUSANDTHS_PER_DEGREE).round() as i64;
        Cow::Owned(sixty_thousandths.to_string())
    }
}

/// The `ST_Percentage` family (`ST_PositivePercentage`, `ST_FixedPercentage`,
/// `ST_PositiveFixedPercentage`) as a [`Fraction`], where `1.0` is 100 %.
///
/// **Two wire spellings are legal and both are read.** ECMA-376 originally wrote a percentage as an
/// integer of 1000ths of a percent (`a:alpha@val="50000"` is 50 %); the later strict/ISO form writes
/// it with an explicit sign (`val="50%"`), and Office both emits and accepts that. Reading accepts
/// either. Writing always emits the integer form, which is what every fixture in this workspace and
/// every PowerPoint release so far produces.
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::AttributeCodec;
/// use mjx_dml::codec::Percentage;
/// use mjx_dml::Fraction;
///
/// assert_eq!(Percentage::decode(Cow::Borrowed("50000")), Ok(Fraction::from_ratio(0.5)));
/// assert_eq!(Percentage::decode(Cow::Borrowed("50%")), Ok(Fraction::from_ratio(0.5)));
/// assert_eq!(Percentage::encode(Fraction::from_ratio(0.5)), "50000");
/// ```
#[derive(Debug)]
pub struct Percentage;

impl AttributeCodec for Percentage {
    type Value<'a> = Fraction;
    type Input<'a> = Fraction;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Fraction, InvalidAttributeValue> {
        let text = raw.trim();
        let (number, scale) = match text.strip_suffix('%') {
            Some(stripped) => (stripped.trim(), 100.0),
            None => (text, PERCENTAGE_SCALE),
        };
        number
            .parse::<f64>()
            .map(|value| Fraction::from_ratio(value / scale))
            .map_err(|error| {
                InvalidAttributeValue::new(format!(
                    "expected a percentage, as 1000ths of a percent or with a `%` sign: {error}"
                ))
            })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        let thousandths_of_a_percent = (value.ratio() * PERCENTAGE_SCALE).round() as i64;
        Cow::Owned(thousandths_of_a_percent.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_measure_reads_every_accepted_spelling_and_writes_one() {
        // Whitespace is accepted around a number, as the schema's base types allow.
        assert_eq!(
            EmuCoordinate::decode(Cow::Borrowed("  914400 ")),
            Ok(Emu::from_emu(914_400))
        );
        // …and both percentage spellings mean the same thing, but only one is ever written.
        assert_eq!(
            Percentage::decode(Cow::Borrowed("12500")),
            Percentage::decode(Cow::Borrowed("12.5%"))
        );
        assert_eq!(Percentage::encode(Fraction::from_ratio(0.125)), "12500");
    }

    #[test]
    fn a_malformed_measure_is_an_error_and_never_a_panic() {
        for bad in ["", "one", "1.5", "0x10", "9223372036854775808"] {
            assert!(
                EmuCoordinate::decode(Cow::Borrowed(bad)).is_err(),
                "{bad:?} was accepted as an EMU coordinate"
            );
        }
        assert!(Percentage::decode(Cow::Borrowed("half")).is_err());
        assert!(Percentage::decode(Cow::Borrowed("%")).is_err());
        assert!(SixtyThousandthsOfADegree::decode(Cow::Borrowed("north")).is_err());
    }

    #[test]
    fn an_angle_round_trips_through_its_wire_unit() {
        for sixty_thousandths in ["0", "-2700000", "5400000", "21600000"] {
            let angle = SixtyThousandthsOfADegree::decode(Cow::Borrowed(sixty_thousandths))
                .expect("a legal angle");
            assert_eq!(
                SixtyThousandthsOfADegree::encode(angle),
                sixty_thousandths,
                "{sixty_thousandths} did not survive its own unit"
            );
        }
    }
}
