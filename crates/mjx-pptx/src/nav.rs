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

/// The `n`-th child element matching `(ns, local)`, mutably.
pub(crate) fn nth_child_mut<'a>(
    parent: &'a mut RawElement,
    interner: &Interner,
    ns: SchemaNamespace,
    local: &str,
    n: usize,
) -> Option<&'a mut RawElement> {
    parent
        .children
        .iter_mut()
        .filter_map(|node| match node {
            RawNode::Element(element) if name_is(&element.name, interner, ns, local) => {
                Some(element)
            }
            _ => None,
        })
        .nth(n)
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
pub(crate) fn resolve_from_root(target: &str) -> Result<PartName, PptxError> {
    resolve_in_dir("/", target)
}

/// Resolves a relationship `target` relative to `source`'s directory to an absolute [`PartName`].
pub(crate) fn resolve_target(source: &PartName, target: &str) -> Result<PartName, PptxError> {
    let source = source.as_str();
    // `source` is absolute, so there is always a leading '/'; include it in the base directory.
    let dir_end = source.rfind('/').map_or(0, |idx| idx + 1);
    resolve_in_dir(&source[..dir_end], target)
}

fn resolve_in_dir(base_dir: &str, target: &str) -> Result<PartName, PptxError> {
    if is_external(target) {
        return Err(PptxError::ExternalTarget {
            target: target.to_owned(),
        });
    }
    let joined = if target.starts_with('/') {
        target.to_owned()
    } else {
        format!("{base_dir}{target}")
    };
    let normalized = normalize(&joined).ok_or_else(|| PptxError::TargetResolution {
        target: target.to_owned(),
    })?;
    PartName::new(&normalized).map_err(PptxError::from)
}

/// Whether a target points outside the package (an absolute URI).
fn is_external(target: &str) -> bool {
    target.contains("://") || target.starts_with("//")
}

/// Normalizes an absolute path, folding `.` and `..` segments. Returns `None` if `..` escapes the root.
fn normalize(path: &str) -> Option<String> {
    let mut segments: Vec<&str> = Vec::new();
    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                segments.pop()?;
            }
            other => segments.push(other),
        }
    }
    Some(format!("/{}", segments.join("/")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_types::namespaces::DML_MAIN;
    use mjx_xml::fidelity;

    fn part(name: &str) -> PartName {
        PartName::new(name).expect("valid part name")
    }

    #[test]
    fn resolve_relative_simple() {
        let resolved = resolve_target(&part("/ppt/presentation.xml"), "slides/slide1.xml").unwrap();
        assert_eq!(resolved.as_str(), "/ppt/slides/slide1.xml");
    }

    #[test]
    fn resolve_with_dotdot() {
        let resolved = resolve_target(
            &part("/ppt/slides/slide1.xml"),
            "../slideLayouts/slideLayout1.xml",
        )
        .unwrap();
        assert_eq!(resolved.as_str(), "/ppt/slideLayouts/slideLayout1.xml");
    }

    #[test]
    fn resolve_from_root_prepends_slash() {
        assert_eq!(
            resolve_from_root("ppt/presentation.xml").unwrap().as_str(),
            "/ppt/presentation.xml"
        );
    }

    #[test]
    fn resolve_rejects_root_escape() {
        let err = resolve_target(&part("/a/b.xml"), "../../x").unwrap_err();
        assert!(matches!(err, PptxError::TargetResolution { .. }), "{err:?}");
    }

    #[test]
    fn resolve_rejects_external() {
        let err =
            resolve_target(&part("/ppt/presentation.xml"), "http://example.com/x").unwrap_err();
        assert!(matches!(err, PptxError::ExternalTarget { .. }), "{err:?}");
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
