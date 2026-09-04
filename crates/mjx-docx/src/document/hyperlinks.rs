//! Hyperlinks: reading a `w:hyperlink`'s resolved target, and the raw-tree scan
//! [`crate::Document::remove_hyperlink`] needs before dropping its relationship.
//!
//! WordprocessingML's hyperlink model is structural, not an attribute on a run the way DrawingML's
//! `a:hlinkClick` is (see `crates/mjx-pptx/src/hyperlink.rs`): `w:hyperlink` *wraps* the runs it
//! links. [`crate::Document::insert_hyperlink`]/`remove_hyperlink` (in `mod.rs`, alongside every
//! other `Document`-level editing method in this crate) manage the relationship a hyperlink names;
//! this module holds the target-resolution type and the shared "is this relationship still
//! referenced" scan both of those call.

use mjx_ooxml_core::{Interner, RawElement, RawNode};

/// Where a hyperlink points, resolved from its own attributes. See [`super::body::Hyperlink`]'s own
/// doc comment for the `r:id`-over-`anchor` precedence ECMA-376 Part 1 §17.16.22 states — this
/// crate's own `resolve_target` (crate-private) applies it directly, so a caller never sees both at
/// once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HyperlinkTarget {
    /// An external target: the relationship's own resolved target string (a URL, a `mailto:`
    /// address, …).
    Url(String),
    /// A bookmark name in the current document (`w:anchor`), unresolved — MJXOFF-124 owns the
    /// bookmark index this would resolve against; this crate hands back the raw name.
    Anchor(String),
}

/// Resolves a hyperlink's own already-read `rel_id`/`anchor` against `rels` (the owning part's own
/// relationships): `rel_id` (looked up in `rels`) wins over `anchor` when both are present, per
/// §17.16.22. Takes the two attribute values already extracted (rather than the [`super::body::
/// Hyperlink`] and an [`Interner`] directly) so a caller can release the part-tree borrow those
/// values were read from before looking `rel_id` up in `rels` — `Document::hyperlink_target` needs
/// both `self.package.part_tree` and `self.package.relationships_for`, which cannot be borrowed at
/// once. `None` if both are absent, or `rel_id` does not resolve to a relationship `rels` actually
/// has (a dangling reference — read, not panicked on).
pub(crate) fn resolve_target(
    rel_id: Option<&str>,
    anchor: Option<&str>,
    rels: Option<&mjx_opc::Relationships>,
) -> Option<HyperlinkTarget> {
    if let Some(id) = rel_id {
        if let Some(rel) = rels.and_then(|rels| rels.by_id(id)) {
            return Some(HyperlinkTarget::Url(rel.target.clone()));
        }
    }
    anchor.map(|anchor| HyperlinkTarget::Anchor(anchor.to_owned()))
}

/// Every relationship id named by a `w:hyperlink/@r:id` anywhere in `element`'s own tree, appended
/// to `out` — used to decide whether a hyperlink relationship is still in use before removing it.
/// Walks the **raw** tree rather than the typed [`super::body::Body`] model, mirroring
/// `mjx_pptx::presentation::hyperlinks::collect_hyperlink_rel_ids` exactly: a hyperlink can nest
/// inside a table cell or another hyperlink, and the raw walk reaches every depth uniformly without
/// a separate typed traversal for each container `Body`/`HdrFtr`/`tables::Cell` might wrap it in.
pub(crate) fn collect_hyperlink_rel_ids<'a>(
    element: &'a RawElement,
    interner: &'a Interner,
    out: &mut Vec<&'a str>,
) {
    let local = interner.resolve(element.name.local);
    if local == "hyperlink" {
        if let Some(id) = element
            .attributes
            .iter()
            .find(|attr| interner.resolve(attr.name.local) == "id")
            .and_then(|attr| std::str::from_utf8(&attr.value).ok())
        {
            out.push(id);
        }
    }
    for child in &element.children {
        if let RawNode::Element(child) = child {
            collect_hyperlink_rel_ids(child, interner, out);
        }
    }
}
