//! Namespace-aware navigation over the raw preservation tree, and relationship-target resolution.
//!
//! [`mjx_ooxml_core::RawElement`] has no finder methods, so these helpers match children and
//! attributes by `(namespace, local)` using the same both-URI rule the derive uses: a schema's
//! Strict *and* Transitional namespace both count. The fidelity reader resolves **element**
//! namespaces but leaves **attribute** namespaces unresolved (only the literal prefix is kept), so a
//! prefixed attribute such as `r:id` is located by first resolving which prefix binds the
//! relationship-reference namespace ([`namespace_prefix`]).
//!
//! # Why this file exists at all
//!
//! It is not on the module list MJXOFF-91's ticket wrote out. It is here because the two modules
//! that *are* — [`crate::workbook::sheets`] reading `x:sheets`, and [`crate::validate`] checking
//! that list against the relationships — would otherwise each hand-roll namespace matching and
//! `r:id` prefix resolution, which is one rule with two implementations and a drift waiting to
//! happen. `mjx-pptx` reached the same conclusion and has the same file; this is a deliberately
//! narrow copy of it, holding only what SpreadsheetML's package layer actually needs.

use mjx_ooxml_core::{Interner, RawAttribute, RawElement, RawName, RawNode, Symbol};
use mjx_ooxml_types::namespaces::SchemaNamespace;
use mjx_opc::PartName;
use mjx_xml::text::unescape_text;
use mjx_xml::XmlError;

use crate::error::XlsxError;

/// Whether an element `name` is `(ns, local)` — accepting both the Strict and Transitional URIs of
/// `ns`, matching on the resolved namespace and never on the prefix.
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

/// All child elements matching `(ns, local)`, in document order.
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

/// The prefix (as an interned [`Symbol`]) that `element` binds to `ns` through an
/// `xmlns:PREFIX="uri"` declaration, if any — how a prefixed attribute whose namespace the reader
/// leaves unresolved is found.
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

/// The decoded value of the attribute with this `prefix` symbol and `local` name.
pub(crate) fn prefixed_attr_value(
    element: &RawElement,
    interner: &Interner,
    prefix: Symbol,
    local: &str,
) -> Option<Result<String, XlsxError>> {
    element
        .attributes
        .iter()
        .find(|attr| attr.name.prefix == Some(prefix) && interner.resolve(attr.name.local) == local)
        .map(decode_value)
}

/// The decoded value of the first **unprefixed** attribute named `local`.
///
/// Decoded rather than handed back raw: a sheet's `@name` is user text, and a workbook whose tab is
/// called `Q1 &amp; Q2` must read back as `Q1 & Q2`.
pub(crate) fn attr_value(
    element: &RawElement,
    interner: &Interner,
    local: &str,
) -> Option<Result<String, XlsxError>> {
    element
        .attributes
        .iter()
        .find(|attr| attr.name.prefix.is_none() && interner.resolve(attr.name.local) == local)
        .map(decode_value)
}

fn decode_value(attr: &RawAttribute) -> Result<String, XlsxError> {
    let raw = std::str::from_utf8(&attr.value).map_err(XmlError::from)?;
    Ok(unescape_text(raw)?.into_owned())
}

/// Resolves a relationship `target` relative to the package root (base directory `/`) — for the
/// package-root `officeDocument` relationship, which has no source part.
pub(crate) fn resolve_from_root(target: &str) -> Result<PartName, XlsxError> {
    PartName::resolve_from_root(target).map_err(|err| target_error(err, target))
}

/// Resolves a relationship `target` relative to `source`'s directory, to an absolute [`PartName`].
pub(crate) fn resolve_target(source: &PartName, target: &str) -> Result<PartName, XlsxError> {
    source
        .resolve(target)
        .map_err(|err| target_error(err, target))
}

/// Restates an OPC target-resolution failure as the SpreadsheetML error naming the same target.
fn target_error(err: mjx_opc::OpcError, target: &str) -> XlsxError {
    match err {
        mjx_opc::OpcError::ExternalTarget(_) => XlsxError::ExternalTarget {
            target: target.to_owned(),
        },
        mjx_opc::OpcError::TargetResolution(_) | mjx_opc::OpcError::Malformed(_) => {
            XlsxError::TargetResolution {
                target: target.to_owned(),
            }
        }
        other => XlsxError::from(other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_ooxml_types::namespaces::{SHARED_RELATIONSHIP_REFERENCE, SML};
    use mjx_xml::fidelity;

    /// Both worlds of one schema resolve, and a *different* namespace with the same local name does
    /// not — the property `name_is` exists for, and the one a prefix-only match would get wrong.
    #[test]
    fn an_element_is_matched_in_both_conformance_worlds_and_never_by_prefix_alone() {
        let transitional = fidelity::parse(
            br#"<s:workbook xmlns:s="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#,
        )
        .expect("parse");
        let strict = fidelity::parse(
            br#"<x:workbook xmlns:x="http://purl.oclc.org/ooxml/spreadsheetml/main"/>"#,
        )
        .expect("parse");
        // Same *prefix* as the Transitional document above, bound to something else entirely.
        let impostor =
            fidelity::parse(br#"<s:workbook xmlns:s="urn:not-spreadsheetml"/>"#).expect("parse");

        assert!(name_is(
            &transitional.root.name,
            &transitional.interner,
            SML,
            "workbook"
        ));
        assert!(name_is(
            &strict.root.name,
            &strict.interner,
            SML,
            "workbook"
        ));
        assert!(
            !name_is(&impostor.root.name, &impostor.interner, SML, "workbook"),
            "the prefix is `s` in both, so a prefix match would wrongly accept this"
        );
    }

    /// `r:id` is found through whichever prefix the root binds to the relationship-reference
    /// namespace, and an entity in an unprefixed attribute is decoded.
    #[test]
    fn a_sheet_entrys_name_is_decoded_and_its_relationship_id_is_found_by_namespace() {
        let doc = fidelity::parse(
            br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
                          xmlns:rel="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
                  <sheets><sheet name="Q1 &amp; Q2" sheetId="1" rel:id="rId7"/></sheets>
                </workbook>"#,
        )
        .expect("parse");
        let interner = &doc.interner;
        let sheets = child(&doc.root, interner, SML, "sheets").expect("s:sheets");
        let sheet = children(sheets, interner, SML, "sheet")
            .next()
            .expect("one s:sheet");

        assert_eq!(
            attr_value(sheet, interner, "name")
                .expect("@name is present")
                .expect("@name decodes"),
            "Q1 & Q2",
            "the tab name is user text and must come back unescaped"
        );

        // `rel`, not `r` — the binding is what matters, never the spelling of the prefix.
        let prefix = namespace_prefix(&doc.root, interner, SHARED_RELATIONSHIP_REFERENCE)
            .expect("the root binds the relationship-reference namespace");
        assert_eq!(
            prefixed_attr_value(sheet, interner, prefix, "id")
                .expect("r:id is present")
                .expect("r:id decodes"),
            "rId7"
        );
    }

    /// A target that climbs above the package root is a resolution failure naming the target, never
    /// a panic — these strings come from files this library did not write.
    #[test]
    fn an_unresolvable_target_is_a_named_error() {
        let source = PartName::new("/xl/workbook.xml").expect("a valid part name");
        let error = resolve_target(&source, "../../../etc/passwd").expect_err("climbs out");
        assert!(
            matches!(&error, XlsxError::TargetResolution { target } if target == "../../../etc/passwd"),
            "got {error:?}"
        );
        assert!(resolve_from_root("xl/workbook.xml").is_ok());
    }
}
