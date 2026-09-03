//! Reading and writing one attribute of a retained attribute vector, in place.
//!
//! These four functions are what a typed accessor is made of — the ones
//! `#[derive(XmlAttributes)]` generates calls to, and the ones a hand-written accessor should use
//! instead of open-coding a search over
//! [`RawAttribute`](mjx_ooxml_core::RawAttribute)s. Every one of them treats the vector as the
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

use mjx_ooxml_core::{AttributeError, Interner, QuoteStyle, RawAttribute, RawName};

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
