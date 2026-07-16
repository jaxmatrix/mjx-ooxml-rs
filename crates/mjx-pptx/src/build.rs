//! Builders for new preservation-tree subtrees (prefixed elements, attributes, text leaves).
//!
//! Constructing a shape means synthesizing `RawElement`s by hand rather than parsing them. Two
//! fidelity rules shape these helpers (see the fidelity writer):
//!
//! - The writer emits `prefix:local` from [`RawName::prefix`] and never consults the resolved
//!   namespace, so every builder sets the literal prefix (`p` for PresentationML, `a` for DrawingML)
//!   the surrounding slide already declares — no new `xmlns` attributes are needed for a subtree
//!   spliced under an element whose ancestor binds those prefixes.
//! - Attribute values and text are stored as **raw, escaped bytes**, so any string coming from a Rust
//!   caller is escaped here ([`attr`] escapes attribute values, [`text_leaf`] escapes character data).
//!
//! Each element also records its resolved namespace ([`qname`] interns `ns.transitional`) so the
//! read-back path ([`crate::nav::name_is`]) can find the element by `(namespace, local)`.

use mjx_ooxml_core::{Interner, QuoteStyle, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::namespaces::SchemaNamespace;
use mjx_xml::text::escape_text;

/// A qualified name with the given literal `prefix`, `local` name, and resolved namespace (interned
/// as the transitional URI of `ns`).
pub(crate) fn qname(
    interner: &mut Interner,
    prefix: &str,
    ns: SchemaNamespace,
    local: &str,
) -> RawName {
    RawName {
        prefix: Some(interner.intern(prefix)),
        local: interner.intern(local),
        namespace: Some(interner.intern(ns.transitional)),
    }
}

/// An unprefixed attribute `name="value"`, with `value` escaped for a double-quoted attribute.
pub(crate) fn attr(interner: &mut Interner, name: &str, value: &str) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: None,
            local: interner.intern(name),
            namespace: None,
        },
        value: escape_attribute(value),
        quote: QuoteStyle::Double,
    }
}

/// A self-closing element `<prefix:local attrs/>` (`empty = true`).
pub(crate) fn leaf(
    interner: &mut Interner,
    prefix: &str,
    ns: SchemaNamespace,
    local: &str,
    attributes: Vec<RawAttribute>,
) -> RawElement {
    RawElement {
        name: qname(interner, prefix, ns, local),
        attributes,
        children: Vec::new(),
        empty: true,
    }
}

/// A container element `<prefix:local attrs>children</prefix:local>` (`empty = false`).
pub(crate) fn node(
    interner: &mut Interner,
    prefix: &str,
    ns: SchemaNamespace,
    local: &str,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
) -> RawElement {
    RawElement {
        name: qname(interner, prefix, ns, local),
        attributes,
        children,
        empty: false,
    }
}

/// A text-bearing element `<prefix:local attrs>text</prefix:local>`, with `text` escaped for
/// character data. An empty `text` yields `<prefix:local attrs></prefix:local>` (no child node).
pub(crate) fn text_leaf(
    interner: &mut Interner,
    prefix: &str,
    ns: SchemaNamespace,
    local: &str,
    attributes: Vec<RawAttribute>,
    text: &str,
) -> RawElement {
    let escaped = escape_text(text);
    let children = if escaped.is_empty() {
        Vec::new()
    } else {
        vec![RawNode::Text(escaped.as_bytes().into())]
    };
    RawElement {
        name: qname(interner, prefix, ns, local),
        attributes,
        children,
        empty: false,
    }
}

/// Escapes `value` as bytes for a double-quoted XML attribute (`&`, `<`, `"`). The fidelity writer
/// emits attribute bytes verbatim, so a value injected from a Rust string must be escaped here.
fn escape_attribute(value: &str) -> Box<[u8]> {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out.into_bytes().into_boxed_slice()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nav;
    use mjx_ooxml_core::RawDocument;
    use mjx_ooxml_types::namespaces::{DML_MAIN, PML};

    /// Serializes a single element (wrapped in a throwaway document) to bytes.
    fn serialize(interner: Interner, root: RawElement) -> String {
        let doc = RawDocument {
            interner,
            bom: false,
            prologue: Vec::new(),
            root,
            epilogue: Vec::new(),
        };
        String::from_utf8(mjx_xml::fidelity::serialize_to_vec(&doc)).unwrap()
    }

    #[test]
    fn leaf_is_self_closing() {
        let mut interner = Interner::new();
        let element = leaf(&mut interner, "a", DML_MAIN, "prstGeom", Vec::new());
        assert_eq!(serialize(interner, element), "<a:prstGeom/>");
    }

    #[test]
    fn node_is_not_self_closing_even_when_childless() {
        let mut interner = Interner::new();
        let element = node(&mut interner, "p", PML, "nvPr", Vec::new(), Vec::new());
        assert_eq!(serialize(interner, element), "<p:nvPr></p:nvPr>");
    }

    #[test]
    fn attr_escapes_reserved_characters() {
        let mut interner = Interner::new();
        let a = attr(&mut interner, "name", r#"a<b&c"d"#);
        let element = leaf(&mut interner, "p", PML, "cNvPr", vec![a]);
        assert_eq!(
            serialize(interner, element),
            r#"<p:cNvPr name="a&lt;b&amp;c&quot;d"/>"#
        );
    }

    #[test]
    fn text_leaf_escapes_character_data() {
        let mut interner = Interner::new();
        let element = text_leaf(&mut interner, "a", DML_MAIN, "t", Vec::new(), "a<b&c");
        assert_eq!(serialize(interner, element), "<a:t>a&lt;b&amp;c</a:t>");
    }

    #[test]
    fn built_element_is_found_by_namespace_and_local() {
        // A built `p:sp` must be locatable by (PML, "sp") — proving prefix *and* namespace are set.
        let mut interner = Interner::new();
        let child = leaf(&mut interner, "p", PML, "spPr", Vec::new());
        let sp = node(
            &mut interner,
            "p",
            PML,
            "sp",
            Vec::new(),
            vec![RawNode::Element(child)],
        );
        assert!(nav::name_is(&sp.name, &interner, PML, "sp"));
        assert!(nav::child(&sp, &interner, PML, "spPr").is_some());
        // Wrong namespace is rejected.
        assert!(!nav::name_is(&sp.name, &interner, DML_MAIN, "sp"));
    }
}
