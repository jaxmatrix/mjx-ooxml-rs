//! The child-order half of the gate: no element carries a child out of its complex type's
//! `xsd:sequence`.
//!
//! This is the half that needs no external tool. `xmllint` catches an ordering fault only for the
//! shape some case happens to author, and only where `References/` exists; this walks **every
//! element of every part** whose root the generated tables know, on every run.
//!
//! # Why the old assertion was vacuous, and what replaced it
//!
//! The previous form asserted `!audited.is_empty()` — that *at least one part* had been audited.
//! `root_element` maps `a:theme`, so a `.docx` "covered" by that assertion meant
//! `word/theme/theme1.xml` was audited and nothing else was. [`TreeAudit::elements_visited`] was
//! already returned per part and thrown away, and `child_order.rs` says outright that an audit which
//! visits nothing passes vacuously.
//!
//! [`assert_deck_is_in_schema_order`] now asserts three separate facts:
//!
//! 1. **Coverage**, stated over the *category* table rather than over the audit's own lookup — see
//!    [`parts_that_must_be_audited`] for why that distinction is the difference between an
//!    assertion and a tautology. It is the assertion that grows itself: the moment MJXOFF-90 adds
//!    the `wml` rows and flips its `OrderingCoverage`, every `word/*.xml` part becomes required
//!    here with no edit to this file.
//! 2. **Non-vacuity, per part.** Each audited part descended into whatever structure its root had
//!    — see [`MINIMUM_ELEMENTS_VISITED`].
//! 3. **Something was audited at all.**
//!
//! [`TreeAudit::elements_visited`]: mjx_ooxml_types::child_order::TreeAudit::elements_visited

use mjx_ooxml_types::child_order;
use mjx_opc::{Package, PartName};

use crate::categories::{categorise, NamespaceCategory, OrderingCoverage};

/// The least number of elements an audited part must have visited, **when its root has element
/// children at all**.
///
/// **Two, and the number is not arbitrary.** The walk visits an element only when the tables name
/// its complex type, so a count of **one** means exactly this: the tables knew the part's *root*
/// type and recognised **none** of its children — the vacuous audit `child_order.rs` warns about,
/// and the shape a `.docx` took when `a:theme` alone satisfied the old `!audited.is_empty()`.
///
/// The qualifier is not a loophole; it is a measured fact. `charts.pptx` ships a legitimately empty
/// `/ppt/tableStyles.xml` — an `a:tblStyleLst` with no children — and a walk that visits one element
/// there is complete, not vacuous. So the floor is *one* for a root with no element children and
/// *two* otherwise: the assertion is "the walk descended into whatever structure was there", which
/// is what non-vacuity means, rather than "the part was big enough".
///
/// A higher floor would be a guess about part size rather than a statement about the walk. Cases
/// that want a specific depth pin it themselves — `mjx-pptx`'s coverage case asserts more than five
/// on each of the five parts a filled blank deck writes.
pub const MINIMUM_ELEMENTS_VISITED: usize = 2;

/// One audited part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditedPart {
    /// The part name, e.g. `/ppt/slides/slide1.xml`.
    pub name: String,
    /// How many elements the walk actually checked.
    pub elements_visited: usize,
    /// How many element children the root has — the structure the walk had available to descend
    /// into, which is what makes [`MINIMUM_ELEMENTS_VISITED`] a statement about the walk rather
    /// than about the part's size.
    pub root_child_elements: usize,
}

impl AuditedPart {
    /// The least number of elements this particular part's audit must have visited.
    #[must_use]
    pub fn floor(&self) -> usize {
        if self.root_child_elements == 0 {
            1
        } else {
            MINIMUM_ELEMENTS_VISITED
        }
    }
}

/// Whether a content type names an XML payload — the same rule the inspection side uses.
fn is_xml_content_type(content_type: &str) -> bool {
    content_type.ends_with("+xml")
        || content_type.ends_with("/xml")
        || content_type.ends_with("vmlDrawing")
}

/// Runs the child-order audit over every part of `bytes` whose root element the generated tables
/// name, panicking on the first defect. Returns one [`AuditedPart`] per audited part, so a caller
/// can prove the walk is not passing vacuously.
///
/// # Panics
/// If the package cannot be opened, or a part carries a child out of its `xsd:sequence`.
#[must_use]
pub fn audit_deck_order(label: &str, bytes: &[u8]) -> Vec<AuditedPart> {
    let mut audited = Vec::new();
    let mut package =
        Package::open(bytes).unwrap_or_else(|e| panic!("{label}: opening package: {e}"));
    let parts: Vec<PartName> = package.part_names().collect();
    for part in parts {
        let Some(content_type) = package.content_type_of(&part).map(str::to_owned) else {
            continue;
        };
        if !is_xml_content_type(&content_type) {
            continue;
        }
        let Ok(document) = package.part_tree(&part) else {
            continue;
        };
        let interner = &document.interner;
        let root = &document.root;
        let Some(namespace) = root.name.namespace.map(|symbol| interner.resolve(symbol)) else {
            continue;
        };
        let Some(order) = child_order::root_element(namespace, interner.resolve(root.name.local))
        else {
            // A part whose root the tables do not name. Which parts those may be is not left open:
            // `assert_deck_is_in_schema_order` re-derives the set from the category tables and
            // fails if one of them *should* have been audited.
            continue;
        };
        let audit = child_order::audit_tree(order, root, interner);
        if let Some(defect) = audit.defect {
            panic!(
                "{label}: {} is out of schema order — {defect}",
                part.as_str()
            );
        }
        audited.push(AuditedPart {
            name: part.as_str().to_owned(),
            elements_visited: audit.elements_visited,
            root_child_elements: root
                .children
                .iter()
                .filter(|child| matches!(child, mjx_ooxml_core::RawNode::Element(_)))
                .count(),
        });
    }
    audited
}

/// Every part of `bytes` the ordering audit **must** have reached: one whose root namespace is a
/// modelled schema whose child-order table is [`Generated`](crate::OrderingCoverage::Generated).
///
/// This is deliberately derived from the *category* table rather than from `root_element`. Asking
/// `root_element` which parts it knows and then asserting the audit reached exactly those would be
/// a tautology — the audit uses the same lookup. Asking the category table instead states something
/// the audit cannot make true by itself: *this schema is generated, so every part rooted in it is
/// audited*. A new global element the generator missed fails here, naming the part.
///
/// A schema whose coverage is `Pending` is not required — and the requirement appears on its own
/// the moment its owner flips the entry. There is no such schema left: WordprocessingML flipped with
/// MJXOFF-90, DrawingML diagrams with MJXOFF-148 and SpreadsheetML with MJXOFF-132, so every
/// modelled markup namespace this gate categorises is now audited. `Pending` stays because the enum
/// is how a *future* gap gets a named owner, not because one is open.
///
/// # Panics
/// If the package cannot be opened.
#[must_use]
pub fn parts_that_must_be_audited(label: &str, bytes: &[u8]) -> Vec<String> {
    let mut expected = Vec::new();
    let mut package =
        Package::open(bytes).unwrap_or_else(|e| panic!("{label}: opening package: {e}"));
    let parts: Vec<PartName> = package.part_names().collect();
    for part in parts {
        let Some(content_type) = package.content_type_of(&part).map(str::to_owned) else {
            continue;
        };
        if !is_xml_content_type(&content_type) {
            continue;
        }
        let Ok(document) = package.part_tree(&part) else {
            continue;
        };
        let namespace = document
            .root
            .name
            .namespace
            .map(|symbol| document.interner.resolve(symbol).to_owned());
        if let NamespaceCategory::Modeled(schema) = categorise(namespace.as_deref()) {
            if schema.ordering == OrderingCoverage::Generated {
                expected.push(part.as_str().to_owned());
            }
        }
    }
    expected
}

/// Asserts that no element of any part of `bytes` carries a child out of its complex type's
/// `xsd:sequence`, that every part the tables *could* audit was audited, and that no audit passed
/// vacuously.
///
/// # Why this may be pointed at a whole deck
///
/// Some of these decks are a committed fixture opened, edited and saved, so they carry parts this
/// library did not write. That is deliberate and safe: the fixtures are themselves schema-valid (the
/// `*_fixture_is_schema_valid` cases prove it with `xmllint`), so a fault this raises is markup
/// **this** library placed. The library never re-orders what it reads — placement only ever runs on
/// a child a caller asked to write — and the byte-identity suites in `mjx-opc` hold that line.
///
/// # Panics
/// On an ordering defect, a part the tables could audit but the walk missed, a vacuous audit, or a
/// deck in which nothing at all was audited.
pub fn assert_deck_is_in_schema_order(label: &str, bytes: &[u8]) {
    let audited = audit_deck_order(label, bytes);
    let expected = parts_that_must_be_audited(label, bytes);

    let missed: Vec<&String> = expected
        .iter()
        .filter(|part| !audited.iter().any(|entry| &entry.name == *part))
        .collect();
    assert!(
        missed.is_empty(),
        "{label}: {missed:?} are rooted in a schema whose child-order table is generated, but the \
         walk did not audit them — the generated `root_element` map does not name their root, which \
         is a codegen gap, not a permitted skip"
    );
    assert!(
        !audited.is_empty(),
        "{label}: not one part was audited for child order — an ordering gate that reaches nothing \
         is the vacuous pass this assertion exists to prevent"
    );
    for part in &audited {
        assert!(
            part.elements_visited >= part.floor(),
            "{label}: {} visited {} element(s) though its root has {} element child(ren); the walk \
             knew the root's complex type and recognised none of its children, which is a vacuous \
             audit rather than a clean one",
            part.name,
            part.elements_visited,
            part.root_child_elements
        );
    }
}
