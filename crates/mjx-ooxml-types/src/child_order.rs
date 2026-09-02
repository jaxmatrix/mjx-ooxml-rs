//! Schema child order — where a child element belongs among its siblings.
//!
//! OOXML complex types are overwhelmingly `xsd:sequence`. **Children in the wrong order are invalid
//! even when every child is present and every child is itself correct**, and an application that
//! opens such a file offers to repair it. Order is therefore validity, not style.
//!
//! This module is how a writer gets it right without having read the XSD. The tables re-exported
//! here — [`DML_MAIN_TYPES`], [`PML_TYPES`], [`DML_CHART_TYPES`] and the named constants beside them
//! — are generated from the reference schemas by `cargo run -p xtask -- codegen`; this module is the
//! hand-written vocabulary they are expressed in and the placement primitives every serializer in
//! the workspace uses.
//!
//! # Rank
//!
//! Every child a complex type can hold has a **rank**: its position in the type's flattened content
//! model. Members of an `xsd:sequence` get successive ranks; the alternatives of an `xsd:choice`
//! *share* one rank, because the schema lets any of them stand in that place. So in
//! `CT_ShapeProperties` the six fill elements (`a:noFill`, `a:solidFill`, …) all have the same rank:
//! whichever one is present is the one a new fill replaces.
//!
//! # What is never reordered
//!
//! Placement is a **write-side** operation. Nothing here reads a document and rewrites it into
//! schema order:
//!
//! - A child the table does not name — an unmodelled element, a foreign namespace, a comment, an
//!   `mc:AlternateContent` — is invisible to placement. It never moves and it never moves the
//!   insertion point, so unmodelled markup keeps its position relative to its known neighbours.
//! - Existing known children are never sorted. A new child is inserted *after the last sibling that
//!   must precede it*; everything already in the element stays exactly where the caller's file had
//!   it.
//! - A type whose content model is [`ContentModel::Choice`] or [`ContentModel::All`] imposes no
//!   order, and this module does not invent one for it.
//!
//! # Cost
//!
//! A [`ChildOrder`] is a `&'static` slice of a handful of names (the median complex type has four
//! children; the largest in these schemas has around forty). Lookup is a linear scan of `&str`
//! comparisons over that slice — no hashing, no allocation, nothing built per call. Serializers hold
//! the `&'static ChildOrder` for their own type as a constant, so the per-call cost is the scan
//! alone. The by-symbol lookups ([`find`], [`root_element`]) exist for tree-walking audits, not for
//! the serialization path.
//!
//! # Example
//!
//! ```
//! use mjx_ooxml_types::child_order::TEXT_LIST_STYLE;
//!
//! // `CT_TextListStyle` is `defPPr`, `lvl1pPr` … `lvl9pPr`, `extLst`.
//! assert_eq!(TEXT_LIST_STYLE.rank_of(None, "defPPr"), Some(0));
//! assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl9pPr"), Some(9));
//! assert_eq!(TEXT_LIST_STYLE.rank_of(None, "extLst"), Some(10));
//! assert_eq!(TEXT_LIST_STYLE.rank_of(None, "bodyPr"), None); // not a child of this type
//! ```

use mjx_ooxml_core::{Interner, RawElement, RawNode};

use crate::namespaces::SchemaNamespace;

pub use crate::generated::child_order::*;

/// How a complex type constrains the order of its children.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentModel {
    /// The type declares no child elements (an attribute-only or empty type).
    Empty,
    /// `xsd:sequence` — children must appear in the order the schema declares. This is the only
    /// model under which order is validity.
    Sequence,
    /// `xsd:choice` — the alternatives may stand in any order the type allows, so none is imposed.
    Choice,
    /// `xsd:all` — the members may appear in any order, so none is imposed.
    All,
}

impl ContentModel {
    /// Whether this model makes child order a matter of validity.
    #[must_use]
    pub fn is_ordered(self) -> bool {
        matches!(self, Self::Sequence)
    }
}

/// A reference to a complex type, as the schema names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TypeReference {
    /// The namespace the type is declared in.
    pub namespace: SchemaNamespace,
    /// The XSD symbol, e.g. `CT_TextParagraphProperties`.
    pub symbol: &'static str,
}

/// One child element a complex type can hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildSlot {
    /// The child element's namespace.
    pub namespace: SchemaNamespace,
    /// The child element's local name, exactly as the schema spells it.
    pub local: &'static str,
    /// The child's own complex type, when it has one — what lets a tree walk continue downwards.
    pub complex_type: Option<TypeReference>,
    /// The child's position in the flattened content model. Children that share a rank are
    /// alternatives of one `xsd:choice`.
    pub rank: u16,
    /// Whether the schema allows more than one of this child in the same place.
    pub repeatable: bool,
    /// Whether the schema reaches this child at more than one rank, so it has no single position.
    /// Such a child is neither placed nor audited by rank.
    pub ambiguous: bool,
}

/// A complex type's children, in schema order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChildOrder {
    /// The XSD symbol, e.g. `CT_TextListStyle`.
    pub symbol: &'static str,
    /// The namespace the type is declared in.
    pub namespace: SchemaNamespace,
    /// How the type constrains child order.
    pub model: ContentModel,
    /// Every child the type can hold, in rank order.
    pub slots: &'static [ChildSlot],
}

/// A child found out of its type's `xsd:sequence` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutOfOrderChild {
    /// The complex type whose sequence was violated.
    pub complex_type: &'static str,
    /// The path of element local names from the walk's root down to the offending element.
    pub path: String,
    /// The name of the child that must have come later.
    pub earlier: &'static str,
    /// The name of the child that appeared after it but ranks before it.
    pub later: &'static str,
}

impl core::fmt::Display for OutOfOrderChild {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{}: `{}` is written after `{}`, but {}'s xsd:sequence puts it before",
            self.path, self.later, self.earlier, self.complex_type
        )
    }
}

/// Whether `candidate` is one of a schema namespace's two URIs (Strict or Transitional).
///
/// The generated tables call this to route a lookup to the right schema, so it is visible to the
/// crate rather than to this module alone.
#[must_use]
pub(crate) fn in_namespace(candidate: &str, schema: SchemaNamespace) -> bool {
    candidate == schema.transitional || Some(candidate) == schema.strict
}

/// [`in_namespace`] for an optional namespace — an element in no namespace matches nothing.
fn is_namespace(schema: SchemaNamespace, candidate: Option<&str>) -> bool {
    candidate.is_some_and(|uri| in_namespace(uri, schema))
}

impl ChildOrder {
    /// The slot for a child named `local` in `namespace`, or `None` if this type has no such child.
    ///
    /// `namespace` is matched against both the Strict and the Transitional URI. Passing `None`
    /// matches on the local name alone — convenient for a caller that has already established the
    /// namespace, and the reason the doc-test above can write `rank_of(None, "defPPr")`.
    #[must_use]
    pub fn slot(&self, namespace: Option<&str>, local: &str) -> Option<&'static ChildSlot> {
        self.slots.iter().find(|slot| {
            slot.local == local && (namespace.is_none() || is_namespace(slot.namespace, namespace))
        })
    }

    /// The rank of a child named `local`, or `None` if this type has no such child or the schema
    /// gives it no single position.
    #[must_use]
    pub fn rank_of(&self, namespace: Option<&str>, local: &str) -> Option<u16> {
        self.slot(namespace, local)
            .filter(|slot| !slot.ambiguous)
            .map(|slot| slot.rank)
    }

    /// The rank of a raw node, or `None` for anything this type does not name — a foreign element,
    /// an unmodelled one, text, a comment, an `mc:AlternateContent`.
    #[must_use]
    pub fn rank_of_node(&self, node: &RawNode, interner: &Interner) -> Option<u16> {
        let RawNode::Element(element) = node else {
            return None;
        };
        self.rank_of_element(element, interner)
    }

    /// The rank of an element among this type's children.
    #[must_use]
    pub fn rank_of_element(&self, element: &RawElement, interner: &Interner) -> Option<u16> {
        let namespace = element
            .name
            .namespace
            .map(|symbol| interner.resolve(symbol));
        self.rank_of(namespace, interner.resolve(element.name.local))
    }

    /// The index at which a child named `local` belongs among `children`.
    ///
    /// The result is one past the last child that must precede it. Nodes this type does not name are
    /// **skipped** rather than treated as a boundary, so a new child lands beside its ranked
    /// neighbours instead of after markup this model does not understand — and the unnamed node does
    /// not move. A `local` the type does not name goes at the end.
    #[must_use]
    pub fn insert_index(&self, children: &[RawNode], interner: &Interner, local: &str) -> usize {
        self.insert_index_of_names(
            children.iter().map(|node| match node {
                RawNode::Element(element) => {
                    let namespace = element.name.namespace.map(|s| interner.resolve(s));
                    self.rank_of(namespace, interner.resolve(element.name.local))
                }
                _ => None,
            }),
            local,
        )
    }

    /// The index at which a child named `local` belongs, given each existing child's rank in
    /// document order (`None` for a child this type does not name).
    ///
    /// The rank-carrying counterpart of [`insert_index`](Self::insert_index), for a model that keeps
    /// its children as typed values rather than as raw nodes.
    #[must_use]
    pub fn insert_index_of_names(
        &self,
        existing: impl Iterator<Item = Option<u16>>,
        local: &str,
    ) -> usize {
        let Some(rank) = self.rank_of(None, local) else {
            return existing.count();
        };
        let mut at = 0;
        for (index, other) in existing.enumerate() {
            match other {
                Some(other) if other <= rank => at = index + 1,
                Some(_) => return at,
                None => {}
            }
        }
        at
    }

    /// Replaces the first child element for which `matches` holds, keeping its position; inserts
    /// `element` **at its rank in the schema's sequence** when there is none.
    ///
    /// `matches` receives a candidate child's local name and is only consulted for elements in one
    /// of this type's own namespaces. It exists so a caller can say which existing child a new one
    /// supersedes — usually itself, but for an `xsd:choice` the alternative that occupies the same
    /// slot (an `a:solidFill` replacing an `a:noFill`).
    pub fn replace_or_insert(
        &self,
        children: &mut Vec<RawNode>,
        interner: &Interner,
        element: RawElement,
        matches: impl Fn(&str) -> bool,
    ) {
        let existing = children.iter().position(|node| match node {
            RawNode::Element(child) => {
                let namespace = child.name.namespace.map(|s| interner.resolve(s));
                let local = interner.resolve(child.name.local);
                // Only a child this type actually names can be superseded — an element of the same
                // local name in a namespace the schema does not put here is somebody else's markup.
                self.slot(namespace, local).is_some() && matches(local)
            }
            _ => false,
        });
        if let Some(index) = existing {
            children[index] = RawNode::Element(element);
            return;
        }
        let local = interner.resolve(element.name.local);
        let at = self.insert_index(children, interner, local);
        children.insert(at, RawNode::Element(element));
    }

    /// Inserts `element` at its rank, without replacing anything — for a repeatable child, and for a
    /// builder assembling a fresh element one child at a time.
    pub fn insert(&self, children: &mut Vec<RawNode>, interner: &Interner, element: RawElement) {
        let local = interner.resolve(element.name.local);
        let at = self.insert_index(children, interner, local);
        children.insert(at, RawNode::Element(element));
    }

    /// The first pair of children of `element` that this type's `xsd:sequence` puts the other way
    /// round, or `None` if its named children are in order.
    ///
    /// Only named, unambiguous children are compared, and only when the model is a sequence — a
    /// `xsd:choice` or `xsd:all` type, and any child this type does not name, is never faulted.
    #[must_use]
    pub fn first_out_of_order(
        &self,
        element: &RawElement,
        interner: &Interner,
        path: &str,
    ) -> Option<OutOfOrderChild> {
        if !self.model.is_ordered() {
            return None;
        }
        let mut previous: Option<&'static ChildSlot> = None;
        for node in &element.children {
            let RawNode::Element(child) = node else {
                continue;
            };
            let namespace = child.name.namespace.map(|s| interner.resolve(s));
            let Some(slot) = self.slot(namespace, interner.resolve(child.name.local)) else {
                continue;
            };
            if slot.ambiguous {
                continue;
            }
            if let Some(previous) = previous {
                if slot.rank < previous.rank {
                    return Some(OutOfOrderChild {
                        complex_type: self.symbol,
                        path: path.to_owned(),
                        earlier: previous.local,
                        later: slot.local,
                    });
                }
            }
            previous = Some(slot);
        }
        None
    }
}

/// What a tree walk found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeAudit {
    /// How many elements the walk actually checked — those whose complex type the tables name.
    ///
    /// An audit that visits nothing passes vacuously, so a caller that means to prove coverage
    /// asserts on this as well as on [`defect`](Self::defect).
    pub elements_visited: usize,
    /// The first child found out of its type's `xsd:sequence`, if any.
    pub defect: Option<OutOfOrderChild>,
}

/// Walks `element` (whose complex type is `order`) and every descendant whose type the tables know,
/// reporting the first child found out of its type's `xsd:sequence`.
///
/// This is the audit that makes an ordering table impossible to ignore: it re-derives, from the
/// schemas alone, whether markup could have come from a conforming writer. It is for verifying
/// **authored** markup — running it over a document a caller supplied would only report what that
/// caller's application wrote, which is not ours to fault.
///
/// The walk stops at the first defect, and never descends into a child whose type the tables do not
/// name — foreign markup, `mc:AlternateContent`, an `a:ext` payload — so nothing outside these
/// schemas is ever judged.
#[must_use]
pub fn audit_tree(
    order: &'static ChildOrder,
    element: &RawElement,
    interner: &Interner,
) -> TreeAudit {
    let root = qualified_name(element, interner);
    let mut audit = TreeAudit {
        elements_visited: 0,
        defect: None,
    };
    walk_for_order(order, element, interner, &root, &mut audit);
    audit
}

fn walk_for_order(
    order: &'static ChildOrder,
    element: &RawElement,
    interner: &Interner,
    path: &str,
    audit: &mut TreeAudit,
) {
    audit.elements_visited += 1;
    if let Some(defect) = order.first_out_of_order(element, interner, path) {
        audit.defect = Some(defect);
        return;
    }
    for node in &element.children {
        let RawNode::Element(child) = node else {
            continue;
        };
        let namespace = child.name.namespace.map(|s| interner.resolve(s));
        let Some(slot) = order.slot(namespace, interner.resolve(child.name.local)) else {
            continue;
        };
        let Some(reference) = slot.complex_type else {
            continue;
        };
        let Some(child_order) = find(reference.namespace.transitional, reference.symbol) else {
            continue;
        };
        let child_path = format!("{path}/{}", qualified_name(child, interner));
        walk_for_order(child_order, child, interner, &child_path, audit);
        if audit.defect.is_some() {
            return;
        }
    }
}

/// An element's name as written, e.g. `a:lstStyle`.
fn qualified_name(element: &RawElement, interner: &Interner) -> String {
    match element.name.prefix {
        Some(prefix) => format!(
            "{}:{}",
            interner.resolve(prefix),
            interner.resolve(element.name.local)
        ),
        None => interner.resolve(element.name.local).to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::namespaces::DML_MAIN;
    use mjx_ooxml_core::RawName;

    fn element(interner: &mut Interner, local: &str) -> RawElement {
        RawElement {
            name: RawName {
                prefix: Some(interner.intern("a")),
                local: interner.intern(local),
                namespace: Some(interner.intern(DML_MAIN.transitional)),
            },
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    fn locals(children: &[RawNode], interner: &Interner) -> Vec<String> {
        children
            .iter()
            .map(|node| match node {
                RawNode::Element(e) => interner.resolve(e.name.local).to_owned(),
                RawNode::Comment(bytes) => {
                    format!("<!--{}-->", String::from_utf8_lossy(bytes))
                }
                _ => "?".to_owned(),
            })
            .collect()
    }

    #[test]
    fn the_generated_sequence_matches_the_schema_for_a_known_type() {
        // `CT_TextListStyle` — `defPPr`, `lvl1pPr` … `lvl9pPr`, `extLst`.
        assert_eq!(TEXT_LIST_STYLE.model, ContentModel::Sequence);
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "defPPr"), Some(0));
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl1pPr"), Some(1));
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl9pPr"), Some(9));
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "extLst"), Some(10));
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl10pPr"), None);
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "bodyPr"), None);
    }

    #[test]
    fn the_alternatives_of_a_choice_share_one_rank() {
        // `EG_FillProperties` sits at one position in `CT_ShapeProperties`' sequence.
        let fills = [
            "noFill",
            "solidFill",
            "gradFill",
            "blipFill",
            "pattFill",
            "grpFill",
        ];
        let ranks: Vec<_> = fills
            .iter()
            .map(|local| SHAPE_PROPERTIES.rank_of(None, local))
            .collect();
        assert!(
            ranks.windows(2).all(|w| w[0] == w[1] && w[0].is_some()),
            "the fill alternatives must share one rank, got {ranks:?}"
        );
        // …and it is after the geometry and before the line.
        let fill = SHAPE_PROPERTIES.rank_of(None, "solidFill").expect("ranked");
        let geometry = SHAPE_PROPERTIES.rank_of(None, "prstGeom").expect("ranked");
        let line = SHAPE_PROPERTIES.rank_of(None, "ln").expect("ranked");
        assert!(geometry < fill && fill < line, "{geometry} {fill} {line}");
    }

    #[test]
    fn a_choice_type_is_recorded_as_unordered_rather_than_given_a_false_order() {
        // `CT_Path2D` is a repeating `xsd:choice` of path commands: no order to impose.
        assert_eq!(PATH_2D.model, ContentModel::Choice);
        assert!(!PATH_2D.model.is_ordered());
        assert!(PATH_2D.slots.iter().all(|slot| slot.rank == 0));
        assert!(PATH_2D.slots.iter().all(|slot| slot.repeatable));
    }

    #[test]
    fn a_new_child_is_placed_at_its_rank_not_appended() {
        let mut interner = Interner::new();
        let mut children = vec![
            RawNode::Element(element(&mut interner, "lvl1pPr")),
            RawNode::Element(element(&mut interner, "extLst")),
        ];
        let new = element(&mut interner, "defPPr");
        TEXT_LIST_STYLE.replace_or_insert(&mut children, &interner, new, |local| local == "defPPr");
        assert_eq!(
            locals(&children, &interner),
            vec!["defPPr", "lvl1pPr", "extLst"]
        );
    }

    #[test]
    fn an_unmodelled_node_between_two_known_children_keeps_its_anchors() {
        // The comment sits between `a:defPPr` (rank 0) and `a:lvl9pPr` (rank 9). Inserting a rank-1
        // level must leave it between the same two known neighbours — it may not be carried to the
        // end, and it may not drag the new level past `a:lvl9pPr`.
        let mut interner = Interner::new();
        let mut children = vec![
            RawNode::Element(element(&mut interner, "defPPr")),
            RawNode::Comment(Box::from(&b" caller's note "[..])),
            RawNode::Element(element(&mut interner, "lvl9pPr")),
        ];
        let new = element(&mut interner, "lvl1pPr");
        TEXT_LIST_STYLE
            .replace_or_insert(&mut children, &interner, new, |local| local == "lvl1pPr");
        let after = locals(&children, &interner);
        let comment = after
            .iter()
            .position(|name| name.starts_with("<!--"))
            .expect("the comment survives");
        let default = after.iter().position(|n| n == "defPPr").expect("defPPr");
        let deepest = after.iter().position(|n| n == "lvl9pPr").expect("lvl9pPr");
        assert!(
            default < comment && comment < deepest,
            "the comment kept neither anchor: {after:?}"
        );
        assert_eq!(
            after
                .iter()
                .filter(|n| !n.starts_with("<!--"))
                .cloned()
                .collect::<Vec<_>>(),
            vec!["defPPr", "lvl1pPr", "lvl9pPr"],
            "the levels must still be in schema order"
        );
    }

    #[test]
    fn an_unmodelled_trailing_node_does_not_drag_a_new_child_to_the_end() {
        // The discriminating case: a writer that simply appends, or that treats any element as a
        // boundary, puts `a:lvl1pPr` after `a:lvl9pPr` here — which is invalid.
        let mut interner = Interner::new();
        let mut children = vec![
            RawNode::Element(element(&mut interner, "lvl9pPr")),
            RawNode::Comment(Box::from(&b" trailing "[..])),
        ];
        let new = element(&mut interner, "lvl1pPr");
        TEXT_LIST_STYLE
            .replace_or_insert(&mut children, &interner, new, |local| local == "lvl1pPr");
        assert_eq!(
            locals(&children, &interner),
            vec!["lvl1pPr", "lvl9pPr", "<!-- trailing -->"],
            "the unmodelled node must neither move nor move the insertion point"
        );
    }

    #[test]
    fn an_existing_child_is_replaced_in_place() {
        let mut interner = Interner::new();
        let mut children = vec![
            RawNode::Element(element(&mut interner, "defPPr")),
            RawNode::Element(element(&mut interner, "lvl1pPr")),
        ];
        let mut replacement = element(&mut interner, "defPPr");
        replacement.attributes.push(mjx_ooxml_core::RawAttribute {
            name: RawName {
                prefix: None,
                local: interner.intern("marL"),
                namespace: None,
            },
            value: Box::from(&b"0"[..]),
            quote: mjx_ooxml_core::QuoteStyle::Double,
        });
        TEXT_LIST_STYLE.replace_or_insert(&mut children, &interner, replacement, |local| {
            local == "defPPr"
        });
        assert_eq!(locals(&children, &interner), vec!["defPPr", "lvl1pPr"]);
        let RawNode::Element(first) = &children[0] else {
            panic!("first child is an element");
        };
        assert_eq!(first.attributes.len(), 1, "replaced in place, not inserted");
    }

    #[test]
    fn an_out_of_order_pair_is_reported_and_an_ordered_one_is_not() {
        let mut interner = Interner::new();
        let mut holder = element(&mut interner, "lstStyle");
        holder.empty = false;
        holder.children = vec![
            RawNode::Element(element(&mut interner, "lvl2pPr")),
            RawNode::Element(element(&mut interner, "lvl1pPr")),
        ];
        let defect = TEXT_LIST_STYLE
            .first_out_of_order(&holder, &interner, "a:lstStyle")
            .expect("lvl1pPr after lvl2pPr is out of sequence");
        assert_eq!(defect.earlier, "lvl2pPr");
        assert_eq!(defect.later, "lvl1pPr");
        assert!(defect.to_string().contains("CT_TextListStyle"));

        holder.children.reverse();
        assert_eq!(
            TEXT_LIST_STYLE.first_out_of_order(&holder, &interner, "a:lstStyle"),
            None
        );
    }

    #[test]
    fn a_foreign_child_is_never_faulted_for_being_out_of_order() {
        let mut interner = Interner::new();
        let mut holder = element(&mut interner, "lstStyle");
        holder.empty = false;
        let foreign = RawElement {
            name: RawName {
                prefix: Some(interner.intern("p14")),
                local: interner.intern("lvl1pPr"),
                namespace: Some(interner.intern("urn:not-drawingml")),
            },
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        holder.children = vec![
            RawNode::Element(element(&mut interner, "lvl9pPr")),
            RawNode::Element(foreign),
        ];
        assert_eq!(
            TEXT_LIST_STYLE.first_out_of_order(&holder, &interner, "a:lstStyle"),
            None,
            "an element in another namespace is not this type's child"
        );
    }

    #[test]
    fn the_tables_are_reachable_by_symbol_and_by_root_element() {
        let order = find(DML_MAIN.transitional, "CT_TextListStyle").expect("dml table");
        assert_eq!(order.symbol, "CT_TextListStyle");
        assert!(find(DML_MAIN.transitional, "CT_NoSuchType").is_none());
        let theme = root_element(DML_MAIN.transitional, "theme").expect("a:theme is global");
        assert_eq!(theme.symbol, "CT_OfficeStyleSheet");
    }

    #[test]
    fn every_table_is_sorted_by_symbol_so_the_lookup_can_bisect() {
        for table in [&DML_MAIN_TYPES[..], &PML_TYPES[..], &DML_CHART_TYPES[..]] {
            assert!(
                table.windows(2).all(|w| w[0].symbol < w[1].symbol),
                "the generated tables must be sorted and free of duplicates"
            );
        }
    }

    #[test]
    fn every_slot_list_is_in_rank_order() {
        for table in [&DML_MAIN_TYPES[..], &PML_TYPES[..], &DML_CHART_TYPES[..]] {
            for order in table {
                assert!(
                    order.slots.windows(2).all(|w| w[0].rank <= w[1].rank),
                    "{} is not in rank order",
                    order.symbol
                );
            }
        }
    }

    #[test]
    fn no_unordered_type_is_given_a_false_order() {
        // The guard in `first_out_of_order` only ever *skips* work; what actually makes an
        // `xsd:choice` safe is that the generator never gave its alternatives distinct ranks. Pin
        // that, because a flattener change that started ranking a choice's branches would make this
        // table worse than none — it would fault conforming markup.
        for table in [&DML_MAIN_TYPES[..], &PML_TYPES[..], &DML_CHART_TYPES[..]] {
            for order in table {
                if order.model.is_ordered() {
                    continue;
                }
                assert!(
                    order.slots.iter().all(|slot| slot.rank == 0),
                    "{} is a {:?} type, so no child of it may outrank another",
                    order.symbol,
                    order.model
                );
            }
        }
    }

    #[test]
    fn the_census_of_content_models_is_what_the_schemas_say() {
        // A census, not a target: it says out loud how much of these schemas is genuinely ordered,
        // and it moves only when the schemas or the flattener do.
        let mut sequence = 0;
        let mut choice = 0;
        let mut all = 0;
        let mut empty = 0;
        for table in [&DML_MAIN_TYPES[..], &PML_TYPES[..], &DML_CHART_TYPES[..]] {
            for order in table {
                match order.model {
                    ContentModel::Sequence => sequence += 1,
                    ContentModel::Choice => choice += 1,
                    ContentModel::All => all += 1,
                    ContentModel::Empty => empty += 1,
                }
            }
        }
        assert_eq!(
            (sequence, choice, all, empty),
            (318, 19, 0, 179),
            "DrawingML, PresentationML and DrawingML-chart hold 516 complex types: 318 sequences \
             where order is validity, 19 genuine choices, no xsd:all, and 179 with no children"
        );
    }

    #[test]
    fn a_scaling_order_places_a_maximum_before_a_minimum() {
        // `CT_Scaling` is `logBase`, `orientation`, `max`, `min`, `extLst` — the one place in these
        // schemas where the intuitive order is the wrong one.
        assert!(
            SCALING.rank_of(None, "max") < SCALING.rank_of(None, "min"),
            "the schema puts c:max before c:min"
        );
    }
}
