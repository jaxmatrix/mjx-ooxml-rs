//! Typed attribute values over the retained attribute vector.
//!
//! A modeled type keeps its [`attributes`](crate::RawElementContent::attributes) exactly as they were
//! read — every attribute, in its original order, with its original prefix, spelling and quote
//! character — and exposes *accessors* over that vector. Nothing here ever rebuilds an attribute
//! list, because rebuilding it is how unknown attributes, their order and their quote style get lost.
//!
//! Two halves live here:
//!
//! * [`AttributeCodec`] — the wire ⇄ Rust conversion for one *kind* of value (`ST_OnOff`, an
//!   enumeration, a measure). A codec is a type-level tag: it is never constructed, only named.
//! * [`AttributeError`] — what a read of a typed attribute can fail with. A malformed value in an
//!   untrusted file is one of these, never a panic.
//!
//! The `#[derive(XmlAttributes)]` macro in `mjx-derive` generates accessors in terms of both; the
//! generated code is the intended reader, but every item here is usable by hand.
//!
//! # Read never normalizes; a write does
//!
//! [`AttributeCodec::decode`] is handed the value that was in the file and produces a Rust value; it
//! cannot and does not change the file. An attribute nobody assigned to therefore re-emits its
//! original bytes — `rtlCol='on'` stays `on`, single-quoted, where it was. Canonicalization happens
//! in [`AttributeCodec::encode`], which runs only when a caller *sets* a value: `set_rtl_col(true)`
//! writes the one canonical spelling `true`.

use std::borrow::Cow;

/// The wire ⇄ Rust conversion for one kind of attribute value.
///
/// Implementors are **type-level tags**, never values: the trait's methods are associated functions
/// and a codec type is only ever named (`Enumeration<LineCap>`), so declaring one costs nothing at
/// runtime.
///
/// Two associated types rather than one, because the two directions want different shapes: a text
/// attribute *reads* as a [`Cow<str>`](Cow) borrowed straight from the attribute's bytes (no
/// allocation on the hot path) but is far more convenient to *write* from a `&str`.
///
/// # Implementing one
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::{AttributeCodec, InvalidAttributeValue};
///
/// /// English Metric Units — 914,400 per inch, signed 64-bit.
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// struct Emu(i64);
///
/// /// `ST_Coordinate` as an [`Emu`].
/// #[derive(Debug)]
/// struct EmuCoordinate;
///
/// impl AttributeCodec for EmuCoordinate {
///     type Value<'a> = Emu;
///     type Input<'a> = Emu;
///
///     fn decode<'a>(raw: Cow<'a, str>) -> Result<Emu, InvalidAttributeValue> {
///         raw.parse::<i64>()
///             .map(Emu)
///             .map_err(|error| InvalidAttributeValue::new(format!("not an EMU coordinate: {error}")))
///     }
///
///     fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
///         Cow::Owned(value.0.to_string())
///     }
/// }
///
/// assert_eq!(EmuCoordinate::decode(Cow::Borrowed("-914400")), Ok(Emu(-914_400)));
/// assert!(EmuCoordinate::decode(Cow::Borrowed("12.5")).is_err());
/// ```
pub trait AttributeCodec {
    /// What a read produces. Borrows from the attribute when it usefully can.
    type Value<'a>;
    /// What a write consumes.
    type Input<'a>;

    /// Converts the attribute's value — already UTF-8 checked and entity-decoded — to a Rust value.
    ///
    /// # Errors
    /// Returns [`InvalidAttributeValue`] if the string is not a legal value for this kind. Attribute
    /// values come from untrusted files, so this is the only reporting channel: never panic.
    fn decode<'a>(raw: Cow<'a, str>) -> Result<Self::Value<'a>, InvalidAttributeValue>;

    /// Converts a Rust value to the **one canonical** wire spelling this kind writes.
    ///
    /// The result is escaped for the target quote character by the caller, not here.
    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str>;
}

/// Why an [`AttributeCodec`] rejected a value.
///
/// Carries a human-readable detail only; the attribute's *name* is added by the accessor that knows
/// it, via [`into_error`](Self::into_error).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidAttributeValue {
    detail: String,
}

impl InvalidAttributeValue {
    /// Records why a value was rejected (e.g. `"expected six hexadecimal digits"`).
    #[must_use]
    pub fn new(detail: impl Into<String>) -> Self {
        Self {
            detail: detail.into(),
        }
    }

    /// The human-readable reason.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Names the offending attribute, producing the error an accessor returns.
    #[must_use]
    pub fn into_error(self, attribute: &'static str) -> AttributeError {
        AttributeError::InvalidValue {
            attribute,
            detail: self.detail,
        }
    }
}

impl core::fmt::Display for InvalidAttributeValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(&self.detail)
    }
}

impl std::error::Error for InvalidAttributeValue {}

/// What reading a typed attribute can fail with.
///
/// `attribute` is the attribute's qualified wire name as written in the model declaration
/// (`"val"`, `"r:embed"`) — a `&'static str`, because the declaration is compile-time and an error
/// path should not allocate to say which attribute it was talking about.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AttributeError {
    /// A required attribute is absent. Never substituted with a default — a required attribute has
    /// none, and inventing one would write markup the file did not contain.
    Missing {
        /// The qualified wire name of the attribute.
        attribute: &'static str,
    },
    /// The attribute's value bytes were not valid UTF-8.
    InvalidUtf8 {
        /// The qualified wire name of the attribute.
        attribute: &'static str,
    },
    /// An entity or character reference in the value could not be decoded.
    InvalidEntity {
        /// The qualified wire name of the attribute.
        attribute: &'static str,
        /// A description of the offending reference.
        detail: String,
    },
    /// The decoded value is not legal for the attribute's declared kind.
    InvalidValue {
        /// The qualified wire name of the attribute.
        attribute: &'static str,
        /// Why the codec rejected it.
        detail: String,
    },
}

impl AttributeError {
    /// The qualified wire name of the attribute this error is about.
    #[must_use]
    pub fn attribute(&self) -> &'static str {
        match self {
            Self::Missing { attribute }
            | Self::InvalidUtf8 { attribute }
            | Self::InvalidEntity { attribute, .. }
            | Self::InvalidValue { attribute, .. } => attribute,
        }
    }
}

impl core::fmt::Display for AttributeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing { attribute } => {
                write!(f, "required attribute `{attribute}` is absent")
            }
            Self::InvalidUtf8 { attribute } => {
                write!(f, "attribute `{attribute}` is not valid UTF-8")
            }
            Self::InvalidEntity { attribute, detail } => write!(
                f,
                "attribute `{attribute}` has an undecodable reference: {detail}"
            ),
            Self::InvalidValue { attribute, detail } => {
                write!(f, "attribute `{attribute}` has an invalid value: {detail}")
            }
        }
    }
}

impl std::error::Error for AttributeError {}

/// Text as written — the identity codec.
///
/// Reads as a [`Cow<str>`](Cow), borrowed from the attribute's own bytes whenever the value carried
/// no entity references, so the common case allocates nothing. Never rejects a value: every string
/// is a legal string.
#[derive(Debug)]
pub struct Text;

impl AttributeCodec for Text {
    type Value<'a> = Cow<'a, str>;
    type Input<'a> = &'a str;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<Cow<'a, str>, InvalidAttributeValue> {
        Ok(raw)
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Borrowed(value)
    }
}

/// Any OOXML enumeration — every generated `ST_*` enum in `mjx-ooxml-types`, and any other type that
/// spells itself with [`FromStr`](core::str::FromStr) and [`Display`](core::fmt::Display).
///
/// The generated enumerations implement both in terms of their own `from_wire` / `to_wire`, so
/// `Enumeration<LineCap>` reads and writes exactly the schema's wire tokens (`rnd`, `sq`, `flat`) and
/// nothing else.
///
/// The example below stands in for one of them — `mjx-ooxml-types` sits *above* this crate, so its
/// enumerations cannot be named here, but their generated shape is exactly this.
///
/// ```
/// use std::borrow::Cow;
/// use mjx_ooxml_core::{AttributeCodec, Enumeration};
///
/// /// `ST_LineCap`, as `mjx-ooxml-types` generates it.
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// enum LineCap { Round, Square, Flat }
///
/// impl core::str::FromStr for LineCap {
///     type Err = String;
///     fn from_str(s: &str) -> Result<Self, String> {
///         match s {
///             "rnd" => Ok(Self::Round),
///             "sq" => Ok(Self::Square),
///             "flat" => Ok(Self::Flat),
///             other => Err(format!("unknown OOXML wire value: {other:?}")),
///         }
///     }
/// }
///
/// impl core::fmt::Display for LineCap {
///     fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
///         f.write_str(match self { Self::Round => "rnd", Self::Square => "sq", Self::Flat => "flat" })
///     }
/// }
///
/// assert_eq!(Enumeration::<LineCap>::decode(Cow::Borrowed("sq")), Ok(LineCap::Square));
/// assert_eq!(Enumeration::<LineCap>::encode(LineCap::Square), "sq");
/// assert!(Enumeration::<LineCap>::decode(Cow::Borrowed("square")).is_err());
/// ```
#[derive(Debug)]
pub struct Enumeration<T>(core::marker::PhantomData<T>);

impl<T> AttributeCodec for Enumeration<T>
where
    T: core::str::FromStr + core::fmt::Display,
    <T as core::str::FromStr>::Err: core::fmt::Display,
{
    type Value<'a> = T;
    type Input<'a> = T;

    fn decode<'a>(raw: Cow<'a, str>) -> Result<T, InvalidAttributeValue> {
        raw.parse::<T>()
            .map_err(|error| InvalidAttributeValue::new(error.to_string()))
    }

    fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
        Cow::Owned(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_codec_borrows_and_never_rejects() {
        let decoded = Text::decode(Cow::Borrowed("anything at all")).expect("Text never rejects");
        assert!(matches!(decoded, Cow::Borrowed("anything at all")));
        assert_eq!(Text::encode("round trip"), "round trip");
    }

    #[test]
    fn invalid_value_names_the_attribute_it_is_told_about() {
        let error = InvalidAttributeValue::new("expected six hexadecimal digits").into_error("val");
        assert_eq!(error.attribute(), "val");
        assert!(error
            .to_string()
            .contains("expected six hexadecimal digits"));
        assert!(error.to_string().contains("val"));
    }

    #[test]
    fn every_error_displays_and_is_a_std_error() {
        for error in [
            AttributeError::Missing { attribute: "val" },
            AttributeError::InvalidUtf8 { attribute: "val" },
            AttributeError::InvalidEntity {
                attribute: "val",
                detail: "&bogus;".to_owned(),
            },
            AttributeError::InvalidValue {
                attribute: "val",
                detail: "nope".to_owned(),
            },
        ] {
            assert!(error.to_string().contains("val"), "unnamed in {error:?}");
            let _dyn: &dyn std::error::Error = &error;
        }
    }
}
