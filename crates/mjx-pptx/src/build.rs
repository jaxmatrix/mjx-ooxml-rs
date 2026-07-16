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

use mjx_ooxml_core::{Interner, QuoteStyle, RawAttribute, RawElement, RawName, RawNode, Symbol};
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

/// A prefixed attribute `prefix:local="value"` (value escaped), reusing an already-interned prefix
/// symbol — used for attributes whose namespace the fidelity reader does not resolve (e.g. the `r`
/// bound to the relationships namespace, for `r:id`).
pub(crate) fn attr_prefixed(
    interner: &mut Interner,
    prefix: Symbol,
    local: &str,
    value: &str,
) -> RawAttribute {
    RawAttribute {
        name: RawName {
            prefix: Some(prefix),
            local: interner.intern(local),
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

/// The bytes of a minimal, Office-valid empty slide: a `p:sld` with an empty shape tree and a
/// master colour-map override (`a:masterClrMapping` = inherit the master's colours). This is a fresh
/// part with its own root, so — unlike a subtree spliced into an existing slide — it must declare its
/// own namespaces.
pub(crate) fn empty_slide_bytes() -> Vec<u8> {
    concat!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
        "\n",
        r#"<p:sld xmlns:p="http://schemas.openxmlformats.org/presentationml/2006/main""#,
        r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
        r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">"#,
        r#"<p:cSld><p:spTree>"#,
        r#"<p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>"#,
        r#"<p:grpSpPr/>"#,
        r#"</p:spTree></p:cSld>"#,
        r#"<p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>"#,
        r#"</p:sld>"#,
    )
    .as_bytes()
    .to_vec()
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
    fn empty_slide_template_parses_to_a_blank_shape_tree() {
        let doc = mjx_xml::fidelity::parse(&empty_slide_bytes()).expect("template is well-formed");
        // p:sld > p:cSld > p:spTree, and the tree has no p:sp shapes.
        assert!(nav::name_is(&doc.root.name, &doc.interner, PML, "sld"));
        let c_sld = nav::child(&doc.root, &doc.interner, PML, "cSld").expect("p:cSld");
        let sp_tree = nav::child(c_sld, &doc.interner, PML, "spTree").expect("p:spTree");
        assert_eq!(nav::children(sp_tree, &doc.interner, PML, "sp").count(), 0);
    }

    #[test]
    fn attr_prefixed_emits_prefixed_name() {
        let mut interner = Interner::new();
        let prefix = interner.intern("r");
        let a = attr_prefixed(&mut interner, prefix, "id", "rId7");
        let element = leaf(&mut interner, "p", PML, "sldId", vec![a]);
        assert_eq!(serialize(interner, element), r#"<p:sldId r:id="rId7"/>"#);
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
