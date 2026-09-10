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
//!
//! # There are two ways to reach the exception, and only one of them is an empty part (MJXOFF-273)
//!
//! Since MJXOFF-272 the walk audits the markup-compatibility-resolved view, which bought a second
//! route to a root with no element children: `legacy_form_control.xlsx`'s `/xl/drawings/drawing1.xml`
//! arrives carrying an `mc:AlternateContent` whose single `mc:Choice Requires="a14"` has no
//! `mc:Fallback`, so resolution drops the subtree and leaves the root bare. Every number the floor
//! reads is then identical to `charts.pptx`'s — and the facts are not: one part is empty, the other
//! is *full of markup no ECMA-376 schema describes*, which this gate deliberately declines to audit.
//!
//! [`mjx_schema_gate::AuditedPart::raw_root_child_elements`] separates them, and the pair below is
//! what holds it up. **Both halves are required**: a flag that fires on the drawing but also on the
//! `a:tblStyleLst` has not distinguished anything, it has made every empty part look suspicious. And
//! neither half is a defect — no assertion here reddens on the flag; see `order.rs`'s own
//! documentation for why auditing the losing choice is the thing the gate must not do.

use mjx_fixtures::fixture;
use mjx_opc::Package;
use mjx_schema_gate::{
    assert_deck_is_in_schema_order, audit_deck_order, AuditedPart, MINIMUM_ELEMENTS_VISITED,
};

/// `charts.pptx` re-emitted: every `.rels` stream and the content types are rewritten on save, and
/// its `a:tblStyleLst` is carried through empty.
fn saved_charts_deck() -> Vec<u8> {
    Package::open(&fixture("charts.pptx"))
        .expect("open charts.pptx")
        .save()
        .expect("save")
}

/// `legacy_form_control.xlsx` re-emitted. Its `/xl/drawings/drawing1.xml` is the emptied-by-
/// resolution case; the workbook is otherwise ordinary.
fn saved_legacy_form_control_workbook() -> Vec<u8> {
    Package::open(&fixture("legacy_form_control.xlsx"))
        .expect("open legacy_form_control.xlsx")
        .save()
        .expect("save")
}

/// The one audited part of `audited` with this name, or a message that says the case proves nothing
/// rather than a bare `unwrap`.
fn part<'a>(audited: &'a [AuditedPart], name: &str) -> &'a AuditedPart {
    audited
        .iter()
        .find(|part| part.name == name)
        .unwrap_or_else(|| {
            panic!("{name} was not audited at all, so this case is comparing nothing")
        })
}

#[test]
fn a_root_with_no_element_children_is_audited_completely_by_visiting_one_element() {
    let saved = saved_charts_deck();
    let audited = audit_deck_order("saved charts.pptx", &saved);

    let table_styles = part(&audited, "/ppt/tableStyles.xml");
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

/// Half one of MJXOFF-273's closing condition: the emptied root *says* it was emptied.
///
/// The assertions are deliberately in two groups. The first shows the row is indistinguishable from
/// `charts.pptx`'s empty `a:tblStyleLst` on **every number the floor reads** — that is the problem
/// restated as an assertion, so a future change that made the two differ some other way would not
/// let this case pass while the reported row still lied. The second is the one number that differs.
///
/// The last group is what keeps the flag from meaning "this part carries markup compatibility":
/// somewhere in this same workbook a root loses an element child to resolution without being
/// emptied, and it must not be flagged.
#[test]
fn a_root_markup_compatibility_emptied_says_so() {
    let saved = saved_legacy_form_control_workbook();
    let audited = audit_deck_order("saved legacy_form_control.xlsx", &saved);
    let drawing = part(&audited, "/xl/drawings/drawing1.xml");

    // Everything the floor reads — identical to the genuinely empty part below.
    assert_eq!(
        drawing.root_child_elements, 0,
        "the a14 choice loses and there is no mc:Fallback, so the conforming view of this root is \
         empty; if the fixture grew a fallback this case is about something else"
    );
    assert_eq!(
        drawing.elements_visited, 1,
        "an emptied root is completely audited by visiting itself, which is exactly why it reads \
         like a clean audit"
    );
    assert_eq!(
        drawing.floor(),
        1,
        "and the floor lets it pass, which is right — there was nothing left to descend into"
    );

    // The one number that tells the two apart.
    assert!(
        drawing.raw_root_child_elements > 0,
        "the part as the file holds it has element children — that is the whole distinction, and \
         without it this row claims a complete audit over content nobody looked at"
    );
    assert!(
        drawing.emptied_by_markup_compatibility_resolution(),
        "so the audit must say the root was emptied by resolution rather than empty"
    );
    assert_eq!(
        audited
            .iter()
            .filter(|part| part.emptied_by_markup_compatibility_resolution())
            .map(|part| part.name.as_str())
            .collect::<Vec<_>>(),
        vec!["/xl/drawings/drawing1.xml"],
        "and it must be the only flagged part of this workbook"
    );

    // The flag is about being *emptied*, not about carrying markup compatibility at all.
    assert!(
        audited.iter().any(|part| {
            part.raw_root_child_elements > part.root_child_elements
                && !part.emptied_by_markup_compatibility_resolution()
        }),
        "this workbook has a root that resolution shrinks without emptying, and if that ever \
         became flagged the flag would just mean `carries mc:` — pick another fixture rather than \
         relaxing this"
    );
}

/// Half two of MJXOFF-273's closing condition, and it is not optional: a flag that also fires on a
/// genuinely empty part has distinguished nothing.
///
/// `charts.pptx`'s `a:tblStyleLst` is the empty part the floor's exception is pinned against by
/// [`a_root_with_no_element_children_is_audited_completely_by_visiting_one_element`], so it is the
/// right one to hold the negative — and the deck-wide assertion below is what stops the case
/// passing because some unrelated change stopped flagging anything at all.
#[test]
fn a_genuinely_empty_root_is_not_flagged_as_emptied() {
    let saved = saved_charts_deck();
    let audited = audit_deck_order("saved charts.pptx", &saved);
    let table_styles = part(&audited, "/ppt/tableStyles.xml");

    assert_eq!(
        table_styles.root_child_elements, 0,
        "this case is about an empty root; if the fixture grew children, pick another"
    );
    assert_eq!(
        table_styles.raw_root_child_elements, 0,
        "and it was empty in the file too — nothing was resolved away, so the two counts agree"
    );
    assert!(
        !table_styles.emptied_by_markup_compatibility_resolution(),
        "an empty part must not be reported as one resolution emptied, or the distinction says \
         nothing"
    );

    let flagged: Vec<&str> = audited
        .iter()
        .filter(|part| part.emptied_by_markup_compatibility_resolution())
        .map(|part| part.name.as_str())
        .collect();
    assert!(
        flagged.is_empty(),
        "no root in this deck is emptied by resolution, so flagging any of it would make the \
         report noise: {flagged:?}"
    );
}

#[test]
fn the_whole_deck_passes_the_ordering_gate() {
    // The assertion the authoring cases run, pointed at a deck a foreign producer wrote and this
    // library re-emitted. It is what makes the case above a statement about the real gate rather
    // than about a helper nothing calls.
    assert_deck_is_in_schema_order("saved charts.pptx", &saved_charts_deck());
}
