//! The non-vacuity floor, and the one exception to it — both proved against a real committed part.
//!
//! [`mjx_schema_gate::MINIMUM_ELEMENTS_VISITED`] says an audited part must have visited at least two
//! elements: one means the tables knew the root's complex type and recognised **none** of its
//! children, which is the vacuous audit `child_order.rs` warns about and the shape a `.docx` took
//! when `a:theme` alone satisfied the old `!audited.is_empty()`.
//!
//! The exception — a root with no element children may legitimately visit one — is not an escape
//! hatch someone might reach for. `charts.pptx` ships a real, valid, empty `a:tblStyleLst`, and this
//! file pins it: remove the exception and the case below goes red, naming the part. That is what
//! keeps the qualifier a measured fact rather than a way of making a floor pass.

use mjx_fixtures::fixture;
use mjx_opc::Package;
use mjx_schema_gate::{assert_deck_is_in_schema_order, audit_deck_order, MINIMUM_ELEMENTS_VISITED};

/// `charts.pptx` re-emitted: every `.rels` stream and the content types are rewritten on save, and
/// its `a:tblStyleLst` is carried through empty.
fn saved_charts_deck() -> Vec<u8> {
    Package::open(&fixture("charts.pptx"))
        .expect("open charts.pptx")
        .save()
        .expect("save")
}

#[test]
fn a_root_with_no_element_children_is_audited_completely_by_visiting_one_element() {
    let saved = saved_charts_deck();
    let audited = audit_deck_order("saved charts.pptx", &saved);

    let table_styles = audited
        .iter()
        .find(|part| part.name == "/ppt/tableStyles.xml")
        .expect("charts.pptx carries an a:tblStyleLst, and the tables name that root");
    assert_eq!(
        table_styles.root_child_elements, 0,
        "this case exists because that part is empty; if it grew children, pick another"
    );
    assert_eq!(
        table_styles.elements_visited, 1,
        "an empty root is completely audited by visiting itself"
    );
    assert_eq!(
        table_styles.floor(),
        1,
        "the floor for a root with no element children is one — dropping this exception makes a \
         valid, complete audit report as vacuous"
    );

    // And the floor really is two for everything else, so the exception is narrow.
    for part in &audited {
        if part.root_child_elements > 0 {
            assert_eq!(part.floor(), MINIMUM_ELEMENTS_VISITED);
        }
    }
    assert!(
        audited
            .iter()
            .any(|part| part.root_child_elements > 0 && part.elements_visited > 1),
        "a deck in which every audited part is empty would make this file prove nothing"
    );
}

#[test]
fn the_whole_deck_passes_the_ordering_gate() {
    // The assertion the authoring cases run, pointed at a deck a foreign producer wrote and this
    // library re-emitted. It is what makes the case above a statement about the real gate rather
    // than about a helper nothing calls.
    assert_deck_is_in_schema_order("saved charts.pptx", &saved_charts_deck());
}
