//! Proves the generated `wml` child-order table (MJXOFF-90): a `w:p` with children deliberately out
//! of `xsd:sequence` order is caught by [`audit_tree`], and inserting the same two children through
//! [`PARAGRAPH::insert`] — "the writer using the table" — produces a tree the same audit reports
//! clean, in the same call order, so the ordering comes from the table and not from the caller
//! having gotten lucky.
//!
//! Both tests would fail to *compile* if the `"wml"` row were removed from `xtask`'s
//! `CHILD_ORDER_SCHEMAS`: `PARAGRAPH` is generated only when that row is present (see
//! `xtask/src/codegen/spec.rs::CHILD_ORDER_EXPORTS`'s `wml` section) — never a silent empty table.

use mjx_ooxml_core::{Interner, RawElement, RawName, RawNode};
use mjx_ooxml_types::child_order::{audit_tree, PARAGRAPH};
use mjx_ooxml_types::namespaces::WML;

/// A childless `<w:{local}/>` in the WordprocessingML namespace.
fn wml_leaf(interner: &mut Interner, local: &str) -> RawElement {
    RawElement::new(
        RawName {
            prefix: Some(interner.intern("w")),
            local: interner.intern(local),
            namespace: Some(interner.intern(WML.transitional)),
        },
        Vec::new(),
        Vec::new(),
        true,
    )
}

/// A `<w:p>` wrapping `children`.
fn wml_paragraph(interner: &mut Interner, children: Vec<RawNode>) -> RawElement {
    let empty = children.is_empty();
    RawElement::new(
        RawName {
            prefix: Some(interner.intern("w")),
            local: interner.intern("p"),
            namespace: Some(interner.intern(WML.transitional)),
        },
        Vec::new(),
        children,
        empty,
    )
}

#[test]
fn a_paragraph_with_children_out_of_sequence_order_is_caught_by_the_audit() {
    let mut interner = Interner::new();
    let run = wml_leaf(&mut interner, "r");
    let paragraph_properties = wml_leaf(&mut interner, "pPr");

    // Deliberately wrong: CT_P's sequence is pPr (rank 0), then the EG_PContent choice — r among
    // them — at rank 1. Putting the run first violates that.
    let paragraph = wml_paragraph(
        &mut interner,
        vec![
            RawNode::Element(run),
            RawNode::Element(paragraph_properties),
        ],
    );

    let audit = audit_tree(PARAGRAPH, &paragraph, &interner);
    let defect = audit
        .defect
        .expect("a w:p with its run before its own properties must be flagged");
    assert_eq!(defect.complex_type, "CT_P");
    assert_eq!(defect.earlier, "r");
    assert_eq!(defect.later, "pPr");
}

#[test]
fn inserting_through_the_table_produces_a_tree_the_same_audit_reports_clean() {
    let mut interner = Interner::new();
    let run = wml_leaf(&mut interner, "r");
    let paragraph_properties = wml_leaf(&mut interner, "pPr");

    // The same two children, in the same (wrong) construction order as the red test above — but
    // placed with `PARAGRAPH.insert` instead of pushed directly, so the table decides where each
    // one lands rather than the caller.
    let mut children = Vec::new();
    PARAGRAPH.insert(&mut children, &interner, run);
    PARAGRAPH.insert(&mut children, &interner, paragraph_properties);

    let paragraph = wml_paragraph(&mut interner, children);

    let audit = audit_tree(PARAGRAPH, &paragraph, &interner);
    assert_eq!(
        audit.defect, None,
        "PARAGRAPH.insert must place pPr before r regardless of call order"
    );
    // Not just a vacuous pass: the walk must actually have descended into pPr's and r's own
    // (empty) subtrees, not stopped at the root.
    assert!(
        audit.elements_visited > 1,
        "expected the audit to visit the paragraph and its two typed children, visited {}",
        audit.elements_visited
    );
}
