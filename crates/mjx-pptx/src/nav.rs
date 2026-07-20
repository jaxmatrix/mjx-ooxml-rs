//! Namespace-aware navigation over the raw preservation tree, and relationship-target resolution.
//!
//! `RawElement` has no finder methods, so these helpers match children/attributes by `(namespace,
//! local)` using the same both-URI rule the derive uses (accept a schema's strict *and* transitional
//! URIs). Note the fidelity reader resolves **element** namespaces but leaves **attribute** namespaces
//! unresolved (only the literal prefix is kept), so a prefixed attribute like `r:id` is located by
//! first resolving which prefix binds the relationship namespace ([`namespace_prefix`]).

use mjx_ooxml_core::{Interner, RawAttribute, RawElement, RawName, RawNode, Symbol};
use mjx_ooxml_types::namespaces::SchemaNamespace;
use mjx_opc::PartName;
use mjx_xml::text::unescape_text;
use mjx_xml::XmlError;

use crate::error::PptxError;

/// Whether an element `name` is `(ns, local)` — accepting both the strict and transitional URIs of
/// `ns`, matching on the resolved namespace (never the prefix).
pub(crate) fn name_is(
    name: &RawName,
    interner: &Interner,
    ns: SchemaNamespace,
    local: &str,
) -> bool {
    if interner.resolve(name.local) != local {
        return false;
    }
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(ns.transitional) || namespace == ns.strict
}

/// The first child element matching `(ns, local)`.
pub(crate) fn child<'a>(
    parent: &'a RawElement,
    interner: &Interner,
    ns: SchemaNamespace,
    local: &str,
) -> Option<&'a RawElement> {
    parent.children.iter().find_map(|node| match node {
        RawNode::Element(element) if name_is(&element.name, interner, ns, local) => Some(element),
        _ => None,
    })
}

/// The first child element matching `(ns, local)`, mutably.
pub(crate) fn child_mut<'a>(
    parent: &'a mut RawElement,
    interner: &Interner,
    ns: SchemaNamespace,
    local: &str,
) -> Option<&'a mut RawElement> {
    parent.children.iter_mut().find_map(|node| match node {
        RawNode::Element(element) if name_is(&element.name, interner, ns, local) => Some(element),
        _ => None,
    })
}

/// All child elements matching `(ns, local)`, in order.
pub(crate) fn children<'a>(
    parent: &'a RawElement,
    interner: &'a Interner,
    ns: SchemaNamespace,
    local: &'a str,
) -> impl Iterator<Item = &'a RawElement> {
    parent.children.iter().filter_map(move |node| match node {
        RawNode::Element(element) if name_is(&element.name, interner, ns, local) => Some(element),
        _ => None,
    })
}

/// The `n`-th child element satisfying `predicate`, mutably. Used to reach a shape by its index in
/// the one shape-kind-agnostic index space (see `slide::shapes`), where a plain `(namespace, local)`
/// match is not enough.
pub(crate) fn nth_child_matching_mut<'a>(
    parent: &'a mut RawElement,
    interner: &Interner,
    n: usize,
    predicate: impl Fn(&RawElement, &Interner) -> bool,
) -> Option<&'a mut RawElement> {
    parent
        .children
        .iter_mut()
        .filter_map(|node| match node {
            RawNode::Element(element) if predicate(element, interner) => Some(element),
            _ => None,
        })
        .nth(n)
}

/// The position in `parent.children` of the `n`-th child element satisfying `predicate` — the
/// index-space sibling of [`nth_child_matching_mut`], for callers that must *remove* the child rather
/// than edit it.
pub(crate) fn nth_child_matching_position(
    parent: &RawElement,
    interner: &Interner,
    n: usize,
    predicate: impl Fn(&RawElement, &Interner) -> bool,
) -> Option<usize> {
    parent
        .children
        .iter()
        .enumerate()
        .filter_map(|(position, node)| match node {
            RawNode::Element(element) if predicate(element, interner) => Some(position),
            _ => None,
        })
        .nth(n)
}

/// Whether a node is a text node made only of whitespace — the indentation between block elements.
///
/// Such a node is significant to the round-trip contract and is never touched on a read; it is only
/// consulted when an element is *removed*, so its own indentation goes with it instead of piling up
/// as blank lines.
pub(crate) fn is_whitespace_text(node: &RawNode) -> bool {
    match node {
        RawNode::Text(bytes) => !bytes.is_empty() && bytes.iter().all(u8::is_ascii_whitespace),
        _ => false,
    }
}

/// The prefix (as an interned [`Symbol`]) that `element` binds to `ns` via an `xmlns:PREFIX="uri"`
/// declaration, if any. Used to locate prefixed attributes whose namespace the reader does not resolve.
pub(crate) fn namespace_prefix(
    element: &RawElement,
    interner: &Interner,
    ns: SchemaNamespace,
) -> Option<Symbol> {
    element.attributes.iter().find_map(|attr| {
        let prefix = attr.name.prefix?;
        if interner.resolve(prefix) != "xmlns" {
            return None;
        }
        let uri = std::str::from_utf8(&attr.value).ok()?;
        if uri == ns.transitional || Some(uri) == ns.strict {
            Some(attr.name.local) // the bound prefix, e.g. `r`
        } else {
            None
        }
    })
}

/// The value of the attribute with the given `prefix` symbol and `local` name (decoded).
pub(crate) fn prefixed_attr_value(
    element: &RawElement,
    interner: &Interner,
    prefix: Symbol,
    local: &str,
) -> Option<Result<String, PptxError>> {
    element
        .attributes
        .iter()
        .find(|attr| attr.name.prefix == Some(prefix) && interner.resolve(attr.name.local) == local)
        .map(decode_value)
}

fn decode_value(attr: &RawAttribute) -> Result<String, PptxError> {
    let raw = std::str::from_utf8(&attr.value).map_err(XmlError::from)?;
    Ok(unescape_text(raw)?.into_owned())
}

/// The UTF-8 value of the first **unprefixed** attribute named `local`, or `None` if absent (or not
/// UTF-8). Returned verbatim — the attributes read this way (`p:clrMap`'s `bg1`/`tx1`/… scheme-slot
/// tokens) contain no XML-special characters.
pub(crate) fn attr_value<'a>(
    element: &'a RawElement,
    interner: &Interner,
    local: &str,
) -> Option<&'a str> {
    element
        .attributes
        .iter()
        .find(|attr| attr.name.prefix.is_none() && interner.resolve(attr.name.local) == local)
        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
}

/// Resolves a relationship `target` relative to the package root (base directory `/`).
///
/// Part-name algebra lives in [`PartName`]; this only restates the failure in PresentationML terms.
pub(crate) fn resolve_from_root(target: &str) -> Result<PartName, PptxError> {
    PartName::resolve_from_root(target).map_err(|err| target_error(err, target))
}

/// Resolves a relationship `target` relative to `source`'s directory to an absolute [`PartName`].
pub(crate) fn resolve_target(source: &PartName, target: &str) -> Result<PartName, PptxError> {
    source
        .resolve(target)
        .map_err(|err| target_error(err, target))
}

/// The relationship target to write in `source`'s `.rels` so that it resolves to `target` — the
/// inverse of [`resolve_target`].
pub(crate) fn relative_target(source: &PartName, target: &PartName) -> String {
    source.relative_target(target)
}

/// Restates an OPC target-resolution failure as the PresentationML error naming the same target.
fn target_error(err: mjx_opc::OpcError, target: &str) -> PptxError {
    match err {
        mjx_opc::OpcError::ExternalTarget(_) => PptxError::ExternalTarget {
            target: target.to_owned(),
        },
        mjx_opc::OpcError::TargetResolution(_) | mjx_opc::OpcError::Malformed(_) => {
            PptxError::TargetResolution {
                target: target.to_owned(),
            }
        }
        other => PptxError::from(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_types::namespaces::DML_MAIN;
    use mjx_xml::fidelity;

    fn part(name: &str) -> PartName {
        PartName::new(name).expect("valid part name")
    }

    // The part-name algebra itself is tested in `mjx_opc::name`; what matters here is that a
    // failure arrives as the PresentationML error naming the offending target.
    #[test]
    fn resolve_restates_failures_in_presentationml_terms() {
        let err = resolve_target(&part("/a/b.xml"), "../../x").unwrap_err();
        assert!(matches!(err, PptxError::TargetResolution { .. }), "{err:?}");

        let err =
            resolve_target(&part("/ppt/presentation.xml"), "http://example.com/x").unwrap_err();
        assert!(matches!(err, PptxError::ExternalTarget { .. }), "{err:?}");

        let resolved = resolve_target(&part("/ppt/presentation.xml"), "slides/slide1.xml").unwrap();
        assert_eq!(resolved.as_str(), "/ppt/slides/slide1.xml");
    }

    #[test]
    fn child_matches_both_uris_not_prefix() {
        // Two same-local children in different namespaces: `child` must pick the DrawingML one.
        let xml = br#"<root xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:z="urn:z"><z:foo/><a:foo bar="1"/></root>"#;
        let doc = fidelity::parse(xml).unwrap();
        let found = child(&doc.root, &doc.interner, DML_MAIN, "foo").expect("finds a:foo");
        // The matched element is the DrawingML one (it has the bar attribute; z:foo does not).
        assert!(found
            .attributes
            .iter()
            .any(|a| doc.interner.resolve(a.name.local) == "bar"));
    }

    #[test]
    fn namespace_prefix_and_prefixed_attr() {
        // r-prefixed attribute, r bound to the relationships namespace on the root.
        let xml = br#"<p:x xmlns:p="urn:p" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" r:id="rId7"/>"#;
        let doc = fidelity::parse(xml).unwrap();
        let rns = mjx_ooxml_types::namespaces::SHARED_RELATIONSHIP_REFERENCE;
        let prefix = namespace_prefix(&doc.root, &doc.interner, rns).expect("r prefix bound");
        let value = prefixed_attr_value(&doc.root, &doc.interner, prefix, "id")
            .expect("r:id present")
            .unwrap();
        assert_eq!(value, "rId7");
    }
}
