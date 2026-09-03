//! Reading and writing one attribute of a retained attribute vector, in place.
//!
//! These four functions are what a typed accessor is made of — the ones
//! `#[derive(XmlAttributes)]` generates calls to, and the ones a hand-written accessor should use
//! instead of open-coding a search over
//! [`RawAttribute`]s. Every one of them treats the vector as the
//! source of truth and touches **only** the attribute it was asked about:
//!
//! * [`find`] and [`decoded_value`] read. They take `&[RawAttribute]` and cannot change anything, so
//!   an attribute nobody assigns to re-emits the bytes it was read with — its own spelling, its own
//!   quote character, its own position among its siblings. Normalization belongs to the write side.
//! * [`set`] rewrites an existing attribute **in place**, keeping its position in document order and
//!   the quote character it was written with (escaping the new value for *that* quote), and appends
//!   only when the attribute is genuinely new. [`remove`] takes one out.
//!
//! Nothing here rebuilds the vector, so an attribute no model knows about is never at risk: it is
//! not copied, not re-escaped and not moved.
//!
//! # Matching
//!
//! An attribute is matched on its **local name plus its literal prefix** — `None` for the unprefixed
//! attributes that are the overwhelming majority, `Some("r")` for `r:embed`.
//!
//! Prefix rather than resolved namespace, and deliberately: an unprefixed attribute is in *no*
//! namespace (unlike an element, which inherits the default declaration), so for those two the
//! prefix *is* the namespace, exactly. For a prefixed attribute the [fidelity
//! reader](crate::fidelity) records no resolved URI at all — it interns attribute names with
//! `namespace: None` — so the URI is not information this layer has. Matching on the prefix is
//! therefore the strongest rule available here, and strictly stronger than matching on the local
//! name alone.

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeCodec, AttributeError, Interner, InvalidAttributeValue, QuoteStyle, RawAttribute,
    RawName,
};

use crate::text;

/// Whether `attribute` is the one named by `prefix` (literal, `None` for unprefixed) and `local`.
fn matches(
    attribute: &RawAttribute,
    interner: &Interner,
    prefix: Option<&str>,
    local: &str,
) -> bool {
    if interner.resolve(attribute.name.local) != local {
        return false;
    }
    match (prefix, attribute.name.prefix) {
        (None, None) => true,
        (Some(wanted), Some(actual)) => interner.resolve(actual) == wanted,
        _ => false,
    }
}

/// The first attribute named `prefix:local` (or unprefixed `local` when `prefix` is `None`).
///
/// See the [module docs](self) for why the match is on the literal prefix.
#[must_use]
pub fn find<'a>(
    attributes: &'a [RawAttribute],
    interner: &Interner,
    prefix: Option<&str>,
    local: &str,
) -> Option<&'a RawAttribute> {
    attributes
        .iter()
        .find(|attribute| matches(attribute, interner, prefix, local))
}

/// An attribute's value as text: UTF-8 checked, then entity- and character-reference decoded.
///
/// Borrows the attribute's own bytes when the value carried no references, which is the common case,
/// so reading a typed attribute usually allocates nothing.
///
/// `name` is the qualified wire name to report in an error (`"val"`, `"r:embed"`); it is what the
/// caller declared, not something re-derived from the interner.
///
/// # Errors
/// [`AttributeError::InvalidUtf8`] if the raw bytes are not UTF-8, or
/// [`AttributeError::InvalidEntity`] if a reference in the value cannot be decoded. Both come from
/// untrusted files and are reported, never panicked on.
pub fn decoded_value<'a>(
    attribute: &'a RawAttribute,
    name: &'static str,
) -> Result<Cow<'a, str>, AttributeError> {
    let raw = std::str::from_utf8(&attribute.value)
        .map_err(|_| AttributeError::InvalidUtf8 { attribute: name })?;
    text::unescape_text(raw).map_err(|error| AttributeError::InvalidEntity {
        attribute: name,
        detail: error.to_string(),
    })
}

/// Writes `value` to the attribute named `prefix:local`, escaped for the quote it will sit in.
///
/// An attribute that is already present is rewritten **where it is**, keeping its position in
/// document order and the quote character the file used. One that is absent is appended,
/// double-quoted — what this library writes when it has no precedent to follow.
///
/// `value` is the canonical spelling the model chose (an [`AttributeCodec`]'s output, typically);
/// escaping for the quote character is this function's job, not the codec's.
///
/// [`AttributeCodec`]: mjx_ooxml_core::AttributeCodec
pub fn set(
    attributes: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    prefix: Option<&str>,
    local: &str,
    value: &str,
) {
    let local_symbol = interner.intern(local);
    let prefix_symbol = prefix.map(|prefix| interner.intern(prefix));

    if let Some(existing) = attributes.iter_mut().find(|attribute| {
        attribute.name.local == local_symbol && attribute.name.prefix == prefix_symbol
    }) {
        existing.value = text::escape_attribute_in(value, existing.quote)
            .as_bytes()
            .into();
        return;
    }

    attributes.push(RawAttribute {
        name: RawName {
            prefix: prefix_symbol,
            local: local_symbol,
            // The fidelity reader resolves no namespace for an attribute, so neither does a write:
            // an attribute built here compares equal to the same attribute read back from a file.
            namespace: None,
        },
        value: text::escape_attribute_in(value, QuoteStyle::Double)
            .as_bytes()
            .into(),
        quote: QuoteStyle::Double,
    });
}

/// Removes every attribute named `prefix:local`, returning whether any was there.
///
/// The attributes around it keep their order; this is how a typed setter spells "unset", so that an
/// optional attribute set to `None` leaves markup that simply does not carry it.
pub fn remove(
    attributes: &mut Vec<RawAttribute>,
    interner: &Interner,
    prefix: Option<&str>,
    local: &str,
) -> bool {
    let before = attributes.len();
    attributes.retain(|attribute| !matches(attribute, interner, prefix, local));
    attributes.len() != before
}

/// Reads one typed attribute — **the single path from a wire attribute to a Rust value**.
///
/// The three steps a typed read is made of, composed once: locate the attribute by prefix and local
/// name ([`find`]), turn its bytes into text ([`decoded_value`]), and hand that text to the
/// [`AttributeCodec`] that knows the kind. `Ok(None)` means the attribute is simply not there, which
/// is not an error — what an absent attribute *means* (a default, a `None`, a
/// [`Missing`](AttributeError::Missing)) is the caller's decision, and the three presences of the
/// `#[xml(attribute(..))]` grammar are exactly those three decisions.
///
/// The accessors `#[derive(XmlAttributes)]` generates are one call to this function each, and a
/// model that reads an element it does not have a type for calls it directly. There is therefore one
/// implementation of "attribute to value" in the workspace, and one place a bug in it could live.
///
/// **Reading never normalizes.** Nothing here can change the file: `attributes` is borrowed shared,
/// so an attribute that is read and not assigned to re-emits its own spelling, its own quote
/// character, in its own position. Canonicalization is [`write()`]'s job alone.
///
/// `qualified` is the attribute's wire name as the caller declared it (`"val"`, `"r:embed"`) — a
/// `&'static str`, so naming the offending attribute in an error costs no allocation.
///
/// # Errors
/// [`AttributeError::InvalidUtf8`] or [`AttributeError::InvalidEntity`] if the value's bytes are not
/// readable as text, or [`AttributeError::InvalidValue`] if the codec rejects what they say. Every
/// one of those comes from an untrusted file and is reported, never panicked on.
pub fn read<'a, C>(
    attributes: &'a [RawAttribute],
    interner: &Interner,
    prefix: Option<&str>,
    local: &str,
    qualified: &'static str,
) -> Result<Option<C::Value<'a>>, AttributeError>
where
    C: AttributeCodec,
{
    let Some(attribute) = find(attributes, interner, prefix, local) else {
        return Ok(None);
    };
    let raw = decoded_value(attribute, qualified)?;
    C::decode(raw)
        .map(Some)
        .map_err(|invalid| InvalidAttributeValue::into_error(invalid, qualified))
}

/// Writes one typed attribute, or removes it — **the single path from a Rust value to a wire
/// attribute**.
///
/// `Some(value)` encodes through the [`AttributeCodec`] and [`set`]s the result: an attribute already
/// in the element is rewritten *where it is*, keeping its position among its siblings and the quote
/// character the file used, with the new value escaped for that quote; a genuinely new one is
/// appended, double-quoted. `None` [`remove`]s it, which is how an optional attribute is unset.
///
/// **A write is the only thing that canonicalizes.** The codec has exactly one output spelling per
/// value, so `write::<OnOff>(.., Some(true))` writes `true` whatever the file said before — but only
/// for the attribute actually assigned to. Every other attribute in the vector, including every one
/// no model has heard of, is untouched: this function reaches one element of the list and never
/// rebuilds it.
///
/// The setters `#[derive(XmlAttributes)]` generates are one call to this function each.
pub fn write<'a, C>(
    attributes: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    prefix: Option<&str>,
    local: &str,
    value: Option<C::Input<'a>>,
) where
    C: AttributeCodec,
{
    match value {
        Some(value) => {
            let encoded = C::encode(value);
            set(attributes, interner, prefix, local, &encoded);
        }
        None => {
            remove(attributes, interner, prefix, local);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `<e a='1' z:keep='x' b="2"/>` — an unknown attribute between two known ones, mixed quotes.
    fn sample() -> (Interner, Vec<RawAttribute>) {
        let mut interner = Interner::new();
        let mut attributes = Vec::new();
        for (prefix, local, value, quote) in [
            (None, "a", "1", QuoteStyle::Single),
            (Some("z"), "keep", "x", QuoteStyle::Single),
            (None, "b", "2", QuoteStyle::Double),
        ] {
            attributes.push(RawAttribute {
                name: RawName {
                    prefix: prefix.map(|p| interner.intern(p)),
                    local: interner.intern(local),
                    namespace: None,
                },
                value: value.as_bytes().into(),
                quote,
            });
        }
        (interner, attributes)
    }

    fn spellings(attributes: &[RawAttribute], interner: &Interner) -> Vec<String> {
        attributes
            .iter()
            .map(|attribute| {
                let name = match attribute.name.prefix {
                    Some(prefix) => format!(
                        "{}:{}",
                        interner.resolve(prefix),
                        interner.resolve(attribute.name.local)
                    ),
                    None => interner.resolve(attribute.name.local).to_owned(),
                };
                let quote = char::from(attribute.quote.byte());
                let value = String::from_utf8_lossy(&attribute.value);
                format!("{name}={quote}{value}{quote}")
            })
            .collect()
    }

    #[test]
    fn an_unprefixed_name_does_not_match_a_prefixed_one() {
        let (interner, attributes) = sample();
        assert!(find(&attributes, &interner, None, "keep").is_none());
        assert!(find(&attributes, &interner, Some("z"), "keep").is_some());
        assert!(find(&attributes, &interner, Some("z"), "a").is_none());
        assert!(find(&attributes, &interner, None, "a").is_some());
    }

    #[test]
    fn setting_rewrites_in_place_keeping_position_and_quote() {
        let (mut interner, mut attributes) = sample();
        set(&mut attributes, &mut interner, None, "a", "9");
        assert_eq!(
            spellings(&attributes, &interner),
            ["a='9'", "z:keep='x'", "b=\"2\""],
            "the rewritten attribute moved, changed quote, or disturbed its neighbours"
        );
    }

    #[test]
    fn setting_a_new_attribute_appends_it_double_quoted() {
        let (mut interner, mut attributes) = sample();
        set(&mut attributes, &mut interner, None, "c", "3");
        assert_eq!(
            spellings(&attributes, &interner),
            ["a='1'", "z:keep='x'", "b=\"2\"", "c=\"3\""]
        );
    }

    #[test]
    fn a_value_is_escaped_for_the_quote_it_lands_in() {
        let (mut interner, mut attributes) = sample();
        // `a` is single-quoted, `b` double-quoted: the same value escapes two different ways.
        set(
            &mut attributes,
            &mut interner,
            None,
            "a",
            r#"it's <b> & "q""#,
        );
        set(
            &mut attributes,
            &mut interner,
            None,
            "b",
            r#"it's <b> & "q""#,
        );
        assert_eq!(
            spellings(&attributes, &interner),
            [
                r#"a='it&apos;s &lt;b> &amp; "q"'"#,
                "z:keep='x'",
                r#"b="it's &lt;b> &amp; &quot;q&quot;""#
            ]
        );
    }

    /// A codec that accepts only `yes`/`no` and writes `yes` — small enough to be obviously right,
    /// and different enough from the sample's values to make a wrong read visible.
    #[derive(Debug)]
    struct YesNo;

    impl mjx_ooxml_core::AttributeCodec for YesNo {
        type Value<'a> = bool;
        type Input<'a> = bool;

        fn decode<'a>(raw: Cow<'a, str>) -> Result<bool, InvalidAttributeValue> {
            match raw.as_ref() {
                "yes" => Ok(true),
                "no" => Ok(false),
                other => Err(InvalidAttributeValue::new(format!(
                    "expected yes or no, found {other:?}"
                ))),
            }
        }

        fn encode<'a>(value: Self::Input<'a>) -> Cow<'a, str> {
            Cow::Borrowed(if value { "yes" } else { "no" })
        }
    }

    #[test]
    fn reading_distinguishes_absent_from_malformed() {
        let (mut interner, mut attributes) = sample();
        // Absent is `Ok(None)`: not being there is not an error, and what it *means* is the
        // caller's decision, not this function's.
        assert_eq!(
            read::<YesNo>(&attributes, &interner, None, "c", "c"),
            Ok(None)
        );
        // Present but not a legal value is an error naming the attribute.
        assert_eq!(
            read::<YesNo>(&attributes, &interner, None, "a", "a"),
            Err(AttributeError::InvalidValue {
                attribute: "a",
                detail: "expected yes or no, found \"1\"".to_owned(),
            })
        );
        set(&mut attributes, &mut interner, None, "a", "yes");
        assert_eq!(
            read::<YesNo>(&attributes, &interner, None, "a", "a"),
            Ok(Some(true))
        );
    }

    #[test]
    fn reading_decodes_references_and_never_normalizes() {
        let (interner, mut attributes) = sample();
        // `b` carries an entity: reading resolves it, and the stored bytes are untouched.
        attributes[2].value = "5 &lt; 6".as_bytes().into();
        assert_eq!(
            read::<mjx_ooxml_core::Text>(&attributes, &interner, None, "b", "b"),
            Ok(Some(Cow::Owned("5 < 6".to_owned())))
        );
        assert_eq!(
            spellings(&attributes, &interner),
            ["a='1'", "z:keep='x'", "b=\"5 &lt; 6\""],
            "a read changed the file"
        );
    }

    #[test]
    fn writing_canonicalizes_only_the_attribute_it_is_given() {
        let (mut interner, mut attributes) = sample();
        // `a` was `'1'`; the codec's one output spelling is `yes`, written into the single quotes
        // the file already used, where it already was.
        write::<YesNo>(&mut attributes, &mut interner, None, "a", Some(true));
        assert_eq!(
            spellings(&attributes, &interner),
            ["a='yes'", "z:keep='x'", "b=\"2\""]
        );
        // `None` removes that attribute and only that one.
        write::<YesNo>(&mut attributes, &mut interner, None, "a", None);
        assert_eq!(spellings(&attributes, &interner), ["z:keep='x'", "b=\"2\""]);
        // A genuinely new one is appended, double-quoted.
        write::<YesNo>(&mut attributes, &mut interner, None, "c", Some(false));
        assert_eq!(
            spellings(&attributes, &interner),
            ["z:keep='x'", "b=\"2\"", "c=\"no\""]
        );
    }

    #[test]
    fn removing_leaves_the_others_in_order() {
        let (interner, mut attributes) = sample();
        assert!(remove(&mut attributes, &interner, None, "a"));
        assert_eq!(spellings(&attributes, &interner), ["z:keep='x'", "b=\"2\""]);
        assert!(!remove(&mut attributes, &interner, None, "a"));
    }

    #[test]
    fn decoding_reports_rather_than_panics() {
        let mut interner = Interner::new();
        let name = RawName {
            prefix: None,
            local: interner.intern("val"),
            namespace: None,
        };
        let decodes = RawAttribute {
            name,
            value: b"a &amp; b".as_slice().into(),
            quote: QuoteStyle::Double,
        };
        assert_eq!(decoded_value(&decodes, "val").as_deref(), Ok("a & b"));

        let bad_entity = RawAttribute {
            value: b"a &bogus; b".as_slice().into(),
            ..decodes
        };
        assert!(matches!(
            decoded_value(&bad_entity, "val"),
            Err(AttributeError::InvalidEntity {
                attribute: "val",
                ..
            })
        ));

        let bad_utf8 = RawAttribute {
            value: vec![0xff, 0xfe].into_boxed_slice(),
            ..decodes
        };
        assert_eq!(
            decoded_value(&bad_utf8, "val"),
            Err(AttributeError::InvalidUtf8 { attribute: "val" })
        );
    }
}
