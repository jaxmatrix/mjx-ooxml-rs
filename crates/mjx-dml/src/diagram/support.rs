//! Small shared builders and finders for `dgm:`-prefixed elements — the DrawingML Diagram
//! (SmartArt) namespace's own equivalent of [`crate::build`]'s `a:` helpers.
//!
//! There is nothing here about attributes (see [`mjx_xml::attribute`] and the
//! `#[xml(attribute(..))]` grammar every type in this module uses instead); this is only the
//! by-name child search and element construction the accessors are built from.

use mjx_ooxml_core::{Interner, RawAttribute, RawElement, RawName, RawNode};
use mjx_ooxml_types::namespaces::DML_DIAGRAM;

/// Builds a `dgm:local` qualified name — literal prefix `dgm` plus the resolved transitional
/// namespace, so a built element serializes as `dgm:local` and reads back by `(DML_DIAGRAM, local)`.
pub(crate) fn dgm_name(interner: &mut Interner, local: &str) -> RawName {
    RawName {
        prefix: Some(interner.intern("dgm")),
        local: interner.intern(local),
        namespace: Some(interner.intern(DML_DIAGRAM.transitional)),
    }
}

/// Builds a `dgm:`-prefixed element with `attributes` and `children` (self-closing when it has no
/// children).
pub(crate) fn dgm_element(
    interner: &mut Interner,
    local: &str,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
) -> RawElement {
    let empty = children.is_empty();
    RawElement::new(dgm_name(interner, local), attributes, children, empty)
}

/// Whether `name` is in the DrawingML Diagram namespace (accepting both its transitional and
/// strict URIs), regardless of prefix.
pub(crate) fn is_dgm(name: &RawName, interner: &Interner) -> bool {
    let namespace = name.namespace.map(|symbol| interner.resolve(symbol));
    namespace == Some(DML_DIAGRAM.transitional) || namespace == DML_DIAGRAM.strict
}

/// The first element in `children` named `(DML_DIAGRAM, local)` — matching on the resolved
/// namespace (both URIs), never the prefix.
pub(crate) fn dgm_child<'a>(
    children: &'a [RawNode],
    interner: &Interner,
    local: &str,
) -> Option<&'a RawElement> {
    children.iter().find_map(|node| match node {
        RawNode::Element(child)
            if is_dgm(&child.name, interner) && interner.resolve(child.name.local) == local =>
        {
            Some(child)
        }
        _ => None,
    })
}
