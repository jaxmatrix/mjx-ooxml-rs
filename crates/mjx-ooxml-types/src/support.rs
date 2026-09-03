//! Hand-written support for the generated OOXML types: the wire-parse error, the boolean
//! normalizers referenced by the generated two-valued type aliases, and the
//! [`AttributeCodec`](mjx_ooxml_core::AttributeCodec)s that carry those simple types across the
//! attribute seam.
//!
//! The codecs are the OOXML-specific half of that seam; the XML-generic half — the trait itself, and
//! [`Text`](mjx_ooxml_core::Text) / [`Enumeration`](mjx_ooxml_core::Enumeration) — lives one layer
//! down in `mjx-ooxml-core`, which knows nothing about OOXML.

use std::borrow::Cow;

use mjx_ooxml_core::{AttributeCodec, InvalidAttributeValue};

/// Returned when a string is not a valid wire token for an enumerated OOXML type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownWireValue {
    value: String,
}

impl UnknownWireValue {
    /// Records an unrecognized wire value.
    #[must_use]
    pub fn new(value: &str) -> Self {
        Self {
            value: value.to_owned(),
        }
    }

    /// The offending value.
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }
}

impl core::fmt::Display for UnknownWireValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "unknown OOXML wire value: {:?}", self.value)
    }
}

impl std::error::Error for UnknownWireValue {}

/// Normalizer for `ST_OnOff` — accepts `true`/`false`/`1`/`0`/`on`/`off`; writes `true`/`false`.
pub mod on_off {
    /// Parses any accepted spelling to a boolean, or `None` if unrecognized.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<bool> {
        match s {
            "true" | "1" | "on" => Some(true),
            "false" | "0" | "off" => Some(false),
            _ => None,
        }
    }

    /// The canonical wire spelling for a boolean.
    #[must_use]
    pub fn to_wire(value: bool) -> &'static str {
        if value {
            "true"
        } else {
            "false"
        }
    }
}

/// Normalizer for `ST_TrueFalse` — accepts `t`/`f`/`true`/`false` (any case); writes `true`/`false`.
pub mod true_false {
    /// Parses any accepted spelling to a boolean, or `None` if unrecognized.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<bool> {
        match s {
            "t" | "true" | "True" => Some(true),
            "f" | "false" | "False" => Some(false),
            _ => None,
        }
    }

    /// The canonical wire spelling for a boolean.
    #[must_use]
    pub fn to_wire(value: bool) -> &'static str {
        if value {
            "true"
        } else {
            "false"
        }
    }
}

/// Normalizer for `ST_TrueFalseBlank` — like [`true_false`] but the empty string means "unset".
pub mod true_false_blank {
    /// Parses to `Some(bool)`, or `Some(None)` for the blank/unset value.
    ///
    /// Returns the outer `None` only when the string is not a recognized spelling.
    #[must_use]
    pub fn from_wire(s: &str) -> Option<Option<bool>> {
        match s {
            "" => Some(None),
            "t" | "true" | "True" => Some(Some(true)),
            "f" | "false" | "False" => Some(Some(false)),
            _ => None,
        }
    }

    /// The canonical wire spelling (`""` for unset).
    #[must_use]
    pub fn to_wire(value: Option<bool>) -> &'static str {
        match value {
            None => "",
            Some(true) => "true",
            Some(false) => "false",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn on_off_normalizes_all_spellings() {
        for s in ["true", "1", "on"] {
            assert_eq!(on_off::from_wire(s), Some(true));
        }
        for s in ["false", "0", "off"] {
            assert_eq!(on_off::from_wire(s), Some(false));
        }
        assert_eq!(on_off::from_wire("nope"), None);
        assert_eq!(on_off::to_wire(true), "true");
        assert_eq!(on_off::to_wire(false), "false");
    }

    #[test]
    fn the_on_off_codec_reads_six_spellings_and_writes_two() {
        for spelling in ["true", "1", "on"] {
            assert_eq!(
                OnOff::decode(Cow::Borrowed(spelling)),
                Ok(true),
                "{spelling}"
            );
        }
        for spelling in ["false", "0", "off"] {
            assert_eq!(
                OnOff::decode(Cow::Borrowed(spelling)),
                Ok(false),
                "{spelling}"
            );
        }
        // Not defaulted to `false`: an unrecognized spelling is a reported error.
        let rejected = OnOff::decode(Cow::Borrowed("yes")).expect_err("`yes` is not an ST_OnOff");
        assert!(rejected.detail().contains("\"yes\""), "{rejected}");
        assert_eq!(OnOff::encode(true), "true");
        assert_eq!(OnOff::encode(false), "false");
    }

    #[test]
    fn the_true_false_codecs_keep_the_blank_value_distinct() {
        assert_eq!(TrueFalse::decode(Cow::Borrowed("t")), Ok(true));
        assert!(TrueFalse::decode(Cow::Borrowed("")).is_err());
        assert_eq!(TrueFalseBlank::decode(Cow::Borrowed("")), Ok(None));
        assert_eq!(TrueFalseBlank::encode(None), "");
        assert_eq!(TrueFalseBlank::encode(Some(true)), "true");
    }

    #[test]
    fn the_hex_colour_codec_checks_length_and_alphabet_and_keeps_case() {
        assert_eq!(
            HexColorRgb::decode(Cow::Borrowed("FF0000")).as_deref(),
            Ok("FF0000")
        );
        assert_eq!(
            HexColorRgb::decode(Cow::Borrowed("ff0000")).as_deref(),
            Ok("ff0000")
        );
        for rejected in ["FFF", "FF00000", "GG0000", "", "FF 000"] {
            assert!(
                HexColorRgb::decode(Cow::Borrowed(rejected)).is_err(),
                "{rejected:?} was accepted as a hex colour"
            );
        }
    }

    #[test]
    fn true_false_blank_handles_unset() {
        assert_eq!(true_false_blank::from_wire(""), Some(None));
        assert_eq!(true_false_blank::from_wire("True"), Some(Some(true)));
        assert_eq!(true_false_blank::from_wire("x"), None);
        assert_eq!(true_false_blank::to_wire(None), "");
        assert_eq!(true_false_blank::to_wire(Some(false)), "false");
    }
}

/// `ST_OnOff` as an attribute value: every accepted spelling reads, one canonical spelling writes.
///
/// Reads `true` / `false` / `1` / `0` / `on` / `off` (the six the schema admits) to a `bool`, and
/// writes `true` / `false` and nothing else — the normalization [`on_off`] defines, applied at the
/// attribute seam. Anything else is rejected rather than defaulted, because a value nobody
/// recognizes is not the same fact as `false`.
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::AttributeCodec;
/// use mjx_ooxml_types::support::OnOff;
///
/// for spelling in ["true", "1", "on"] {
///     assert_eq!(OnOff::decode(Cow::Borrowed(spelling)), Ok(true));
/// }
/// assert_eq!(OnOff::encode(true), "true");
/// assert!(OnOff::decode(Cow::Borrowed("yes")).is_err());
/// ```
#[derive(Debug)]
pub struct OnOff;

impl AttributeCodec for OnOff {
    type Value<'a> = bool;
    type Input<'a> = bool;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<bool, InvalidAttributeValue> {
        on_off::from_wire(&raw).ok_or_else(|| {
            InvalidAttributeValue::new(format!(
                "expected one of true/false/1/0/on/off, found {:?}",
                raw.as_ref()
            ))
        })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Borrowed(on_off::to_wire(value))
    }
}

/// `ST_TrueFalse` (the VML spelling) as an attribute value: reads `t`/`f`/`true`/`false`/`True`/
/// `False`, writes `true`/`false`.
#[derive(Debug)]
pub struct TrueFalse;

impl AttributeCodec for TrueFalse {
    type Value<'a> = bool;
    type Input<'a> = bool;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<bool, InvalidAttributeValue> {
        true_false::from_wire(&raw).ok_or_else(|| {
            InvalidAttributeValue::new(format!(
                "expected one of t/f/true/false/True/False, found {:?}",
                raw.as_ref()
            ))
        })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Borrowed(true_false::to_wire(value))
    }
}

/// `ST_TrueFalseBlank` as an attribute value: [`TrueFalse`] plus the empty string, which means
/// "unset" and is a value the attribute genuinely carries — distinct from the attribute being absent.
#[derive(Debug)]
pub struct TrueFalseBlank;

impl AttributeCodec for TrueFalseBlank {
    type Value<'a> = Option<bool>;
    type Input<'a> = Option<bool>;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Option<bool>, InvalidAttributeValue> {
        true_false_blank::from_wire(&raw).ok_or_else(|| {
            InvalidAttributeValue::new(format!(
                "expected one of t/f/true/false/True/False or the empty string, found {:?}",
                raw.as_ref()
            ))
        })
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Borrowed(true_false_blank::to_wire(value))
    }
}

/// `ST_HexColorRGB` — `xsd:hexBinary` of length 3, i.e. exactly six hexadecimal digits (`FF0000`).
///
/// Reads as the digits themselves, borrowed from the attribute, with the file's own letter case
/// preserved: `ff0000` and `FF0000` are the same colour and this codec is not the place to decide
/// which spelling a file should have used.
///
/// A value that is not six hexadecimal digits is **rejected on read** — that is where untrusted
/// bytes arrive. The write direction takes the caller's `&str` as given, because a setter's argument
/// comes from the program, not from the file, and making every setter in the workspace fallible to
/// re-check it would be paid for by the many codecs whose input type cannot be wrong at all.
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::AttributeCodec;
/// use mjx_ooxml_types::support::HexColorRgb;
///
/// assert_eq!(HexColorRgb::decode(Cow::Borrowed("ff0000")).as_deref(), Ok("ff0000"));
/// assert!(HexColorRgb::decode(Cow::Borrowed("FFF")).is_err());      // three digits is not this type
/// assert!(HexColorRgb::decode(Cow::Borrowed("GG0000")).is_err());   // G is not a hexadecimal digit
/// ```
#[derive(Debug)]
pub struct HexColorRgb;

impl AttributeCodec for HexColorRgb {
    type Value<'a> = Cow<'a, str>;
    type Input<'a> = &'a str;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Cow<'a, str>, InvalidAttributeValue> {
        let digits = raw.as_ref();
        if digits.len() == 6 && digits.bytes().all(|b| b.is_ascii_hexdigit()) {
            Ok(raw)
        } else {
            Err(InvalidAttributeValue::new(format!(
                "expected six hexadecimal digits, found {digits:?}"
            )))
        }
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Borrowed(value)
    }
}
