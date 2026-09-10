//! The ordering arm audits the markup-compatibility-**resolved** view, and the case that proves it
//! is the one no count could have caught.
//!
//! MJXOFF-272 has two shapes, and they are not equally visible.
//!
//! The **loud** one is a part whose *only* root child is an `mc:AlternateContent`. The tables name
//! no `mc:AlternateContent` slot, so a walk over the raw tree recognises none of the root's children
//! and visits exactly one element — and [`mjx_schema_gate::MINIMUM_ELEMENTS_VISITED`] fires. That is
//! a red, which is a working alarm; `tests/fixtures/legacy_form_control.xlsx` was tripping it, and
//! `mjx-xlsx`'s `ORDER_SWEEP_EXCLUSIONS` register held the row until this was fixed.
//!
//! The **quiet** one is an `mc:AlternateContent` that is one root child *among several*. The walk
//! recognises the siblings, descends into them, and reports a perfectly plausible count — while the
//! whole subtree behind the `mc:` element goes unaudited. **No floor can see this**, because the
//! number a floor reads is the number a healthy audit of the siblings produces. It is the shape this
//! project keeps finding: a surface exercised at one point and reported as covered.
//!
//! So no case here asserts a total. They are written against *what was visited*: the fallback holds
//! a copy of the sibling anchor, so a walk that entered it must have visited exactly twice the
//! structure the raw walk did — and an anchor put out of its `xsd:sequence` inside that fallback must
//! turn the audit red while the raw walk still reports it clean.
//!
//! The third case is the branch the fix opened rather than the one it closed. A part whose markup
//! compatibility will not resolve is *reported*, because falling back to the raw tree would put the
//! arm straight back to auditing markup the schema arm never sees — silently, and for exactly the
//! parts most likely to be hiding something.

use mjx_fixtures::fixture;
use mjx_ooxml_types::child_order::{self, TreeAudit};
use mjx_opc::{Package, PartName};
use mjx_schema_gate::{audit_deck_order, audit_order_report, MINIMUM_ELEMENTS_VISITED};

/// The SpreadsheetDrawingML namespace, as `dml-spreadsheetDrawing.xsd` declares it.
const XDR_NS: &str = "http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing";

/// The part this file rewrites. `worksheet_drawings.xlsx` already declares it, already gives it the
/// right content type, and is already swept clean by `mjx-xlsx`'s gate — so a red here is this
/// file's markup and nothing else.
const DRAWING_PART: &str = "/xl/drawings/drawing1.xml";

/// Everything an `xdr:twoCellAnchor` holds except its trailing `xdr:clientData`, which the two
/// variants below place differently. `CT_TwoCellAnchor`'s `xsd:sequence` is `from`, `to`, the
/// object, then `clientData`.
const ANCHOR_BODY: &str = concat!(
    r#"<xdr:from><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff>"#,
    r#"<xdr:row>1</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from>"#,
    r#"<xdr:to><xdr:col>3</xdr:col><xdr:colOff>0</xdr:colOff>"#,
    r#"<xdr:row>3</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:to>"#,
    r#"<xdr:sp><xdr:nvSpPr><xdr:cNvPr id="2" name="Shape 1"/><xdr:cNvSpPr/></xdr:nvSpPr>"#,
    r#"<xdr:spPr><a:xfrm><a:off x="0" y="0"/><a:ext cx="0" cy="0"/></a:xfrm>"#,
    r#"<a:prstGeom prst="rect"><a:avLst/></a:prstGeom></xdr:spPr></xdr:sp>"#,
);

/// Where the fallback anchor puts its `xdr:clientData`.
#[derive(Clone, Copy)]
enum FallbackAnchor {
    /// Last, where `CT_TwoCellAnchor`'s sequence puts it.
    InOrder,
    /// First, ahead of `xdr:from` — a defect the ordering tables alone are enough to catch, with no
    /// schema validator involved.
    ClientDataFirst,
}

/// `worksheet_drawings.xlsx` with its drawing part replaced by an `xdr:wsDr` carrying **two** root
/// children: a plain `xdr:twoCellAnchor`, and an `mc:AlternateContent` beside it.
///
/// The `mc:Choice` requires `a14`, the Microsoft 2010 drawing extension, which ECMA-376 does not
/// define and [`mjx_schema_gate::categories::ecma_376_namespaces`] therefore does not list — so the
/// choice loses and the `mc:Fallback` wins, exactly as it does in the files Office writes. The
/// fallback holds a **copy of the sibling anchor**, which is what makes the element counts below
/// comparable rather than arbitrary.
fn a_drawing_with_an_alternate_content_among_its_root_children(
    fallback: FallbackAnchor,
) -> Vec<u8> {
    let fallback_anchor = match fallback {
        FallbackAnchor::InOrder => format!("{ANCHOR_BODY}<xdr:clientData/>"),
        FallbackAnchor::ClientDataFirst => format!("<xdr:clientData/>{ANCHOR_BODY}"),
    };
    let part = format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<xdr:wsDr xmlns:xdr="{xdr}""#,
            r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
            r#" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">"#,
            r#"<xdr:twoCellAnchor>{body}<xdr:clientData/></xdr:twoCellAnchor>"#,
            r#"<mc:AlternateContent>"#,
            r#"<mc:Choice xmlns:a14="http://schemas.microsoft.com/office/drawing/2010/main""#,
            r#" Requires="a14">"#,
            r#"<xdr:twoCellAnchor>{body}<xdr:clientData/></xdr:twoCellAnchor>"#,
            r#"</mc:Choice>"#,
            r#"<mc:Fallback><xdr:twoCellAnchor>{fallback}</xdr:twoCellAnchor></mc:Fallback>"#,
            r#"</mc:AlternateContent>"#,
            r#"</xdr:wsDr>"#,
        ),
        xdr = XDR_NS,
        body = ANCHOR_BODY,
        fallback = fallback_anchor,
    );

    let mut package = Package::open(&fixture("worksheet_drawings.xlsx")).expect("open");
    let name = PartName::new(DRAWING_PART).expect("a valid part name");
    package
        .replace_part_bytes(&name, part.into_bytes())
        .expect("the fixture declares the drawing part");
    package.save().expect("save the rewritten drawing")
}

/// The walk the ordering arm used to do: `child_order::audit_tree` over the tree the package holds,
/// with no markup compatibility resolved. This is the *defect*, kept alive here on purpose, because
/// a case about a silent gap has to be able to show the silence.
fn raw_walk(saved: &[u8]) -> TreeAudit {
    let mut package = Package::open(saved).expect("open the rewritten package");
    let name = PartName::new(DRAWING_PART).expect("a valid part name");
    let document = package.part_tree(&name).expect("parse the drawing");
    let order = child_order::root_element(XDR_NS, "wsDr")
        .expect("the generated tables name xdr:wsDr; if they stopped, this case proves nothing");
    child_order::audit_tree(order, &document.root, &document.interner)
}

#[test]
fn an_alternate_content_among_root_siblings_is_walked_rather_than_stepped_over() {
    let saved =
        a_drawing_with_an_alternate_content_among_its_root_children(FallbackAnchor::InOrder);

    let raw = raw_walk(&saved);
    let audited = audit_deck_order("a drawing with an mc:AlternateContent sibling", &saved);
    let entry = audited
        .iter()
        .find(|part| part.name == DRAWING_PART)
        .expect("the drawing is rooted in SpreadsheetDrawingML, so the walk must have audited it");
    println!(
        "{DRAWING_PART} — raw walk visited {}, the gate visited {} (root has {} element children, \
         floor {})",
        raw.elements_visited,
        entry.elements_visited,
        entry.root_child_elements,
        entry.floor()
    );

    // The half that makes this the *quiet* variant, and the reason a floor was never going to be
    // enough: the raw walk looks completely healthy. It finds no defect, and it visits well past the
    // floor, because the sibling anchor it *did* enter is real structure.
    assert!(
        raw.defect.is_none(),
        "the raw walk must report the part clean, or this case is about the loud variant instead: \
         {:?}",
        raw.defect
    );
    assert!(
        raw.elements_visited > MINIMUM_ELEMENTS_VISITED,
        "the raw walk visited {} element(s) against a floor of {MINIMUM_ELEMENTS_VISITED} — this \
         case exists because that count is *plausible*, so a raw walk that trips the floor would be \
         the loud variant and would prove nothing about the quiet one",
        raw.elements_visited
    );

    // …and the half that says what was actually visited, rather than how much. The `mc:Fallback`
    // holds a copy of the sibling anchor, so entering it doubles the structure below the root — and
    // the root itself is visited once either way. Nothing here is a quoted total: both numbers are
    // measured on the same run, and the relation between them is a property of the markup above.
    assert_eq!(
        entry.elements_visited - 1,
        2 * (raw.elements_visited - 1),
        "the mc:Fallback holds a copy of the anchor beside it, so a walk that entered the resolved \
         view visits exactly twice the structure below the root that the raw walk does; the raw \
         walk visited {} and the gate visited {}, which means the mc: subtree was stepped over",
        raw.elements_visited,
        entry.elements_visited,
    );
}

#[test]
fn an_out_of_sequence_child_inside_an_mc_fallback_turns_the_ordering_audit_red() {
    let saved = a_drawing_with_an_alternate_content_among_its_root_children(
        FallbackAnchor::ClientDataFirst,
    );

    // First, the silence this closes: the raw walk reports the part clean and past the floor, so
    // neither the ordering assertion nor its vacuity guard would ever have said a word.
    let raw = raw_walk(&saved);
    assert!(
        raw.defect.is_none() && raw.elements_visited > MINIMUM_ELEMENTS_VISITED,
        "the raw walk must be clean and plausible — that is the defect; it reported {:?} after {} \
         element(s)",
        raw.defect,
        raw.elements_visited
    );

    // `audit_deck_order` panics on a defect, so the red is caught and read rather than asserted
    // around. The default hook is silenced first: this panic is the expected result, and letting it
    // print would make a passing run look like a failing one.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let outcome = std::panic::catch_unwind(|| {
        audit_deck_order("a drawing with an out-of-order mc:Fallback", &saved)
    });
    std::panic::set_hook(previous);

    let payload = outcome
        .expect_err("an anchor out of its sequence inside an mc:Fallback must turn the audit red");
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("<non-string panic>")
        .to_owned();
    assert!(
        message.contains(DRAWING_PART),
        "the audit must name the part: {message}"
    );
    assert!(
        message.contains("CT_TwoCellAnchor") && message.contains("clientData"),
        "the audit must name the type whose sequence was broken and the child that broke it — and \
         that child is inside the mc:Fallback, which is the whole point: {message}"
    );
    println!("the resolved view, proved load-bearing:\n{message}");
}

/// `worksheet_drawings.xlsx` with a drawing part whose `mc:Choice` has no `Requires` — the
/// malformed shape [`mjx_mce::ResolveError::MalformedAlternateContent`] names.
fn a_drawing_whose_alternate_content_will_not_resolve() -> Vec<u8> {
    let part = format!(
        concat!(
            r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<xdr:wsDr xmlns:xdr="{xdr}""#,
            r#" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main""#,
            r#" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006">"#,
            r#"<xdr:twoCellAnchor>{body}<xdr:clientData/></xdr:twoCellAnchor>"#,
            r#"<mc:AlternateContent><mc:Choice>"#,
            r#"<xdr:twoCellAnchor>{body}<xdr:clientData/></xdr:twoCellAnchor>"#,
            r#"</mc:Choice></mc:AlternateContent>"#,
            r#"</xdr:wsDr>"#,
        ),
        xdr = XDR_NS,
        body = ANCHOR_BODY,
    );

    let mut package = Package::open(&fixture("worksheet_drawings.xlsx")).expect("open");
    let name = PartName::new(DRAWING_PART).expect("a valid part name");
    package
        .replace_part_bytes(&name, part.into_bytes())
        .expect("the fixture declares the drawing part");
    package.save().expect("save the unresolvable drawing")
}

/// A part whose markup compatibility will not resolve is **reported**, not quietly walked raw.
///
/// This is the branch the fix opened, and leaving it untested would be the defect above wearing a
/// different hat: falling back to the raw tree here would put the arm back to auditing markup the
/// schema arm never sees, and would do it silently, for exactly the parts most likely to be hiding
/// something. The audit says so instead, and names the part.
#[test]
fn an_alternate_content_that_will_not_resolve_is_reported_rather_than_walked_raw() {
    let saved = a_drawing_whose_alternate_content_will_not_resolve();
    let report = audit_order_report("a drawing whose mc:AlternateContent is malformed", &saved);

    let defect = report
        .defects
        .iter()
        .find(|defect| defect.contains(DRAWING_PART))
        .unwrap_or_else(|| {
            panic!(
                "the malformed part must be reported by name; the audit found {:?} and audited \
                 {:?}",
                report.defects,
                report
                    .audited
                    .iter()
                    .map(|part| part.name.as_str())
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        defect.contains("could not be audited"),
        "the defect must say the ordering arm could not do its job, rather than read as an \
         ordering fault of the markup: {defect}"
    );
    assert!(
        !report.audited.iter().any(|part| part.name == DRAWING_PART),
        "a part the arm could not audit must not appear in the audited list — that is the false \
         green this reports instead of"
    );

    // The discriminating half: only that part is affected. The worksheet beside it is still audited
    // cleanly, so this case cannot pass because the whole package stopped resolving.
    assert!(
        report
            .audited
            .iter()
            .any(|part| part.name == "/xl/worksheets/sheet1.xml"),
        "the worksheet must still be audited; the audit reported {:?}",
        report.defects
    );
    println!("the unresolvable branch, proved live:\n{defect}");
}
