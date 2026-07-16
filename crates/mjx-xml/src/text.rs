//! Escaping and unescaping of XML **text content** (character data between tags).
//!
//! The [`fidelity`](crate::fidelity) layer deliberately treats text and attribute bytes as opaque and
//! never touches entities, which is what makes byte-identical round-trips possible. A *typed* model,
//! by contrast, wants the decoded string (e.g. the text of an `a:t` run) and must re-encode it when
//! writing back. These two helpers are that bridge, and they keep `quick-xml` — which owns the entity
//! tables — behind `mjx-xml` (no other crate may depend on it directly).
//!
//! # Round-trip fidelity
//!
//! [`escape_text`] uses **minimal** escaping: it escapes only `<` and `&`, the two characters an XML
//! parser requires to be escaped inside text content. Decoding then re-encoding is therefore
//! **byte-identical** whenever the source used only the canonical forms (`&amp;`, `&lt;`) or no
//! entities at all. It is *not* byte-identical for other spellings — a literal `&gt;` decodes to `>`
//! and re-encodes as `>` (unescaped), and a numeric reference like `&#65;` decodes to `A`. That is
//! the documented contract for *modeled* parts (canonical-XML equality), as opposed to the
//! byte-identity guaranteed for pass-through parts.

use std::borrow::Cow;

use crate::XmlError;

/// Escapes XML text content, escaping only `<` and `&` (the minimum XML requires between tags).
///
/// Returns a borrowed [`Cow`] when nothing needed escaping. See the [module docs](self) for the
/// round-trip guarantees. Note this is for element text content, **not** attribute values.
///
/// # Example
/// ```
/// assert_eq!(mjx_xml::text::escape_text("a < b & c"), "a &lt; b &amp; c");
/// assert_eq!(mjx_xml::text::escape_text("plain"), "plain"); // borrowed, unchanged
/// ```
#[must_use]
pub fn escape_text(raw: &str) -> Cow<'_, str> {
    quick_xml::escape::minimal_escape(raw)
}

/// Escapes a string for use inside a **double-quoted** XML attribute value, escaping `&`, `<`, and
/// `"` (the minimum a parser requires between double quotes). `>` and `'` are left literal.
///
/// Returns a borrowed [`Cow`] when nothing needed escaping. Text content uses [`escape_text`]
/// instead — attributes additionally require `"` to be escaped.
///
/// # Example
/// ```
/// assert_eq!(mjx_xml::text::escape_attribute(r#"a<b&c"d"#), "a&lt;b&amp;c&quot;d");
/// assert_eq!(mjx_xml::text::escape_attribute("plain > it's"), "plain > it's"); // borrowed
/// ```
#[must_use]
pub fn escape_attribute(raw: &str) -> Cow<'_, str> {
    if !raw.bytes().any(|b| matches!(b, b'&' | b'<' | b'"')) {
        return Cow::Borrowed(raw);
    }
    let mut out = String::with_capacity(raw.len() + 8);
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            other => out.push(other),
        }
    }
    Cow::Owned(out)
}

/// Decodes XML entity and character references in text content back to their characters.
///
/// Handles the five predefined entities (`&amp; &lt; &gt; &quot; &apos;`) and numeric references
/// (`&#N;` / `&#xN;`).
///
/// # Errors
/// Returns [`XmlError::Syntax`] if a reference is malformed or names an unknown entity.
///
/// # Example
/// ```
/// assert_eq!(mjx_xml::text::unescape_text("a &amp;&lt; b").unwrap(), "a &< b");
/// assert_eq!(mjx_xml::text::unescape_text("&#65;").unwrap(), "A");
/// assert!(mjx_xml::text::unescape_text("&bogus;").is_err());
/// ```
pub fn unescape_text(raw: &str) -> Result<Cow<'_, str>, XmlError> {
    quick_xml::escape::unescape(raw).map_err(|e| XmlError::Syntax(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escape_is_minimal() {
        // `<` and `&` are escaped; `>`, `"`, `'` are left literal (minimal, not full).
        assert_eq!(escape_text("a<b&c"), "a&lt;b&amp;c");
        assert_eq!(escape_text("keep > \" ' literal"), "keep > \" ' literal");
    }

    #[test]
    fn escape_borrows_when_unchanged() {
        assert!(matches!(escape_text("Hello OOXML"), Cow::Borrowed(_)));
    }

    #[test]
    fn escape_attribute_escapes_quote_amp_lt_only() {
        // `"` is escaped (unlike text content); `>` and `'` are left literal.
        assert_eq!(escape_attribute(r#"a<b&c"d"#), "a&lt;b&amp;c&quot;d");
        assert_eq!(escape_attribute("keep > ' literal"), "keep > ' literal");
        assert!(matches!(escape_attribute("rect"), Cow::Borrowed(_)));
    }

    #[test]
    fn unescape_decodes_named_and_numeric() {
        assert_eq!(unescape_text("a &amp;&lt; b").unwrap(), "a &< b");
        assert_eq!(unescape_text("&#65;").unwrap(), "A");
        assert_eq!(unescape_text("&#x41;").unwrap(), "A");
        assert_eq!(unescape_text("nothing here").unwrap(), "nothing here");
    }

    #[test]
    fn unescape_rejects_unknown_entity() {
        assert!(unescape_text("&bogus;").is_err());
    }

    #[test]
    fn amp_and_lt_round_trip_byte_identical() {
        // The canonical spellings survive decode -> encode unchanged.
        for original in ["a &amp; b", "x &lt; y", "Hello OOXML"] {
            let decoded = unescape_text(original).unwrap();
            assert_eq!(escape_text(&decoded), original);
        }
    }
}
