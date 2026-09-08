//! The `mc:Ignorable` / `CT_Extension` seam, from both sides (MJXOFF-196).
//!
//! # What the seam was
//!
//! `mjx-schema-gate` validates the **markup-compatibility-resolved** view of a part, because
//! `mc:Ignorable` names attributes the base schemas have no declaration for. Resolution removes an
//! ignorable element *together with its content*, which is what ECMA-376 Part 3 says. But
//! `sml.xsd`'s and `dml-chart.xsd`'s `CT_Extension` declare their whole content model as a bare
//! `<xsd:any processContents="lax"/>`, whose `minOccurs` defaults to **1** — so the emptied `<ext>`
//! was rejected with *Missing child element(s)*, on every conformant file Office has written since
//! 2010, in all three formats, because Office 2016 writes a `c16:uniqueId` extension under
//! `mc:Ignorable` on chart series.
//!
//! # What was done about it, and the shape that was refused
//!
//! MJXOFF-196 offered "validate the least-modified view that passes" — resolve fully, and on failure
//! retry with the ignorable content kept. That is a **try-then-fall-back** arrangement and it was
//! refused: with two views a deviation must appear in both to be reported, which is a gate that
//! goes quiet, and MJXOFF-88 §7 names the signature. Exactly one view is validated and it is always
//! the same one.
//!
//! The rule is instead content-dependent and consults the schema, in the cheapest form that is still
//! exact: `crates/mjx-schema-gate/src/wildcard_slots.rs` derives, *from the pinned XSDs*, every
//! element whose content model is built of `xsd:any` and nothing else and cannot match the empty
//! sequence — five of them, where the ticket named two — and
//! `crates/mjx-schema-gate/src/inspect.rs` drops such an element when resolution emptied it. An
//! element that exists only to carry an extension is ignored along with the extension it carried.
//!
//! # Why this file exists rather than a tolerance
//!
//! A tolerance in `crates/mjx-schema-gate/src/tolerances.rs` is for one file and one message, and
//! never for markup we author. The seam is neither file-specific nor a producer's fault, so
//! recording it as one would have filed a defect of the gate as a quirk of somebody's spreadsheet.
//! Every deck below is authored **here**, through the facade, and held to
//! `assert_authored_deck_is_schema_valid`, which tolerates nothing at all.
//!
//! # What each case is for
//!
//! * [`a_worksheet_carrying_an_ignorable_extension_validates`] and
//!   [`a_chart_series_carrying_an_ignorable_extension_validates`] are the ticket's *done when*: the
//!   two schemas that carry the defect, through the gate that excuses nothing.
//! * [`an_extension_slot_we_author_empty_is_still_a_failure`] is the discrimination. The rule fires
//!   only when the source element *had* children, so an `<ext/>` this library wrote empty is still
//!   reported. Without that condition every empty extension slot in the workspace would go quiet.
//! * [`an_ignorable_element_the_schema_has_no_wildcard_for_is_still_removed`] is the other
//!   discrimination, and it is the counterexample MJXOFF-196 used to rule out "keep ignorable
//!   elements always": Word writes `w14:` elements inside a `w:rPr`, which has no wildcard, and they
//!   must still go.
//! * [`the_two_views_that_remain`] records what the three-view diagnosis has collapsed to.

use std::sync::Once;

use mjx_ooxml::{ChartData, ChartKind, Deck, ShapeBounds, SlideSize, Workbook};
use mjx_opc::{Package, PartName};

/// The SpreadsheetML namespace, as every part of this file spells it.
const SML_NAMESPACE: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

/// A namespace no schema in the reference tree declares, so it is exactly what `mc:Ignorable` is
/// for. Authored here, and presented as nothing else: `tests/office-authored/` is the only place a
/// file's provenance is a claim, and nothing in this file came out of Office.
const IGNORABLE_NAMESPACE: &str = "urn:mjx:demo";

/// Prints once why a run without `References/` proves less than a run with it.
fn note_if_the_schema_half_is_skipping() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if mjx_schema_gate::harness().is_none() {
            println!(
                "the schema half of this suite is skipping: no References/ tree or no xmllint. \
                 The child-order half still runs. MJX_REQUIRE_SCHEMA=1 makes the absence a failure."
            );
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Building the two packages
// ---------------------------------------------------------------------------------------------

/// The `<extLst>` Office's shape is: one `<ext>` with a GUID `uri`, holding one element in a
/// namespace the root declares `mc:Ignorable`.
fn extension_list(prefix: &str) -> String {
    format!(
        "<{prefix}extLst><{prefix}ext uri=\"{{2C3FCC01-B0D6-4A2A-9C1A-000000000001}}\">\
         <demo:note weight=\"3\">an extension only its author understands</demo:note>\
         </{prefix}ext></{prefix}extLst>"
    )
}

/// Declares `demo` on a part's root element and makes it ignorable, exactly as a producer does.
///
/// Byte surgery on a part this library just wrote, rather than a model API, because no model API
/// authors `mc:Ignorable` — that is the point of the seam. `root` is the root tag as written, e.g.
/// `<worksheet ` or `<c:chartSpace `.
fn declare_the_ignorable_namespace(part: &str, root_tag: &str) -> String {
    let declarations = format!(
        "{root_tag}xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
         xmlns:demo=\"{IGNORABLE_NAMESPACE}\" mc:Ignorable=\"demo\" "
    );
    assert!(
        part.contains(root_tag),
        "the part does not open with `{root_tag}`, so this test is editing markup it does not \
         understand:\n{}",
        &part[..part.len().min(400)]
    );
    part.replacen(root_tag, &declarations, 1)
}

/// Rewrites one part of a saved package and saves it again.
fn rewrite_part(package_bytes: &[u8], part_suffix: &str, edit: impl Fn(&str) -> String) -> Vec<u8> {
    let mut package = Package::open(package_bytes).expect("the package this library just saved");
    let part: PartName = package
        .part_names()
        .find(|name| name.as_str().ends_with(part_suffix))
        .unwrap_or_else(|| {
            panic!(
                "no part ends with {part_suffix}; the package holds: {:?}",
                package.part_names().collect::<Vec<_>>()
            )
        });
    let source = package
        .part_bytes(&part)
        .map(|bytes| String::from_utf8(bytes.to_vec()).expect("the part is UTF-8"))
        .expect("the part has bytes");
    package
        .replace_part_bytes(&part, edit(&source).into_bytes())
        .expect("replacing a part this library wrote");
    package.save().expect("saving the edited package")
}

/// A blank workbook whose worksheet carries an ignorable extension on its `<worksheet>` root.
fn workbook_with_an_ignorable_worksheet_extension() -> Vec<u8> {
    let workbook = Workbook::blank().expect("a blank workbook");
    let saved = workbook.save().expect("saving a blank workbook");
    rewrite_part(&saved, "/worksheets/sheet1.xml", |part| {
        let part = declare_the_ignorable_namespace(part, "<worksheet ");
        // `extLst` is the last child of `CT_Worksheet`, so it goes immediately before the close.
        part.replacen(
            "</worksheet>",
            &format!("{}</worksheet>", extension_list("")),
            1,
        )
    })
}

/// A blank deck with one bar chart whose first `<c:ser>` carries an ignorable extension.
///
/// That is Office 2016's own shape: `c16:uniqueId` under `mc:Ignorable`, on a chart series.
fn deck_with_an_ignorable_chart_series_extension() -> Vec<u8> {
    let saved = authored_deck_with_a_chart();
    rewrite_part(&saved, "/charts/chart1.xml", |part| {
        let part = declare_the_ignorable_namespace(part, "<c:chartSpace ");
        // `extLst` is the last child of `CT_BarSer`.
        part.replacen("</c:ser>", &format!("{}</c:ser>", extension_list("c:")), 1)
    })
}

/// A blank deck carrying one chart, saved. The baseline both chart cases start from.
fn authored_deck_with_a_chart() -> Vec<u8> {
    let mut deck = Deck::blank(SlideSize::widescreen()).expect("a blank deck");
    let slide = deck.add_slide().expect("a slide");
    let chart = ChartData::new(ChartKind::Bar)
        .categories(["Q1", "Q2", "Q3", "Q4"])
        .series("2026", [12.0, 15.5, 14.0, 19.25]);
    deck.add_chart(
        slide.into(),
        &chart,
        ShapeBounds::from_inches(1.0, 1.0, 8.0, 4.0),
    )
    .expect("a chart");
    deck.save().expect("saving the deck")
}

// ---------------------------------------------------------------------------------------------
// The two cases the ticket asks for
// ---------------------------------------------------------------------------------------------

#[test]
fn a_worksheet_carrying_an_ignorable_extension_validates() {
    note_if_the_schema_half_is_skipping();
    let bytes = workbook_with_an_ignorable_worksheet_extension();
    mjx_schema_gate::assert_authored_deck_is_schema_valid(
        "a worksheet carrying an ignorable extension",
        &bytes,
    );
}

#[test]
fn a_chart_series_carrying_an_ignorable_extension_validates() {
    note_if_the_schema_half_is_skipping();
    let bytes = deck_with_an_ignorable_chart_series_extension();
    mjx_schema_gate::assert_authored_deck_is_schema_valid(
        "a chart series carrying an ignorable extension",
        &bytes,
    );
}

// ---------------------------------------------------------------------------------------------
// The two discriminations: what the fix must still refuse
// ---------------------------------------------------------------------------------------------

/// An `<ext/>` with no content **in the source** is a defect of whoever wrote it, and the gate must
/// still say so. The rule fires only when resolution is what emptied the slot.
///
/// This is the case that stops the fix from becoming "extension slots are never validated". Revert
/// the `held_something` condition in `inspect.rs` and only this case reddens.
#[test]
fn an_extension_slot_we_author_empty_is_still_a_failure() {
    let Some(harness) = mjx_schema_gate::harness() else {
        note_if_the_schema_half_is_skipping();
        return;
    };
    let schema = mjx_schema_gate::schema_for_namespace(SML_NAMESPACE)
        .expect("SpreadsheetML is a modelled schema");
    let work = mjx_schema_gate::WorkDir::new("mce-seam-empty-slot");

    // The same worksheet, and the same `mc:Ignorable`, but the `<ext>` was written empty. Nothing
    // was ignored away; the hole is the author's.
    let source = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <worksheet xmlns=\"{SML_NAMESPACE}\" \
         xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
         xmlns:demo=\"{IGNORABLE_NAMESPACE}\" mc:Ignorable=\"demo\">\
         <sheetData/><extLst><ext uri=\"{{2C3FCC01-B0D6-4A2A-9C1A-000000000001}}\"/></extLst>\
         </worksheet>\n"
    )
    .into_bytes();
    let document = mjx_xml::fidelity::parse(&source).expect("the authored worksheet parses");
    let resolved = mjx_schema_gate::markup_compatibility_resolved(&document)
        .expect("nothing here is an unsatisfied mc:MustUnderstand");
    let text = String::from_utf8(resolved.clone()).expect("the resolved view is UTF-8");
    assert!(
        text.contains("<ext uri"),
        "an `<ext>` the author wrote empty was dropped, so the gate can no longer report one:\n\
         {text}"
    );

    let path = work.path().join("empty-slot.xml");
    std::fs::write(&path, &resolved).expect("writing the resolved view");
    let report = harness
        .validate(schema, SML_NAMESPACE, &path)
        .unwrap_or_else(|| {
            panic!(
                "an `<ext>` authored with no content validated. The wildcard-slot rule has stopped \
                 distinguishing a hole markup-compatibility resolution left from a hole the author \
                 left, and every empty extension slot in this workspace is now unreported."
            )
        });
    assert!(
        report.contains("Missing child element(s)"),
        "the empty slot fails for some other reason:\n{report}"
    );
    println!(
        "an author's empty `<ext>` is still reported: {}",
        report.trim()
    );
}

/// The counterexample MJXOFF-196 used to rule out its option 2 — *remove ignorable attributes
/// always, ignorable elements never*.
///
/// Word writes `w14:` elements inside `w:rPr`, and `CT_RPr` has no wildcard, so keeping them would
/// fail. They must still be removed, and the wildcard-slot rule leaves that path exactly as it was.
#[test]
fn an_ignorable_element_the_schema_has_no_wildcard_for_is_still_removed() {
    const WML: &str = "http://schemas.openxmlformats.org/wordprocessingml/2006/main";
    let source = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <w:document xmlns:w=\"{WML}\" \
         xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
         xmlns:w14=\"http://schemas.microsoft.com/office/word/2010/wordml\" mc:Ignorable=\"w14\">\
         <w:body><w:p><w:r><w:rPr><w14:ligatures w14:val=\"standardContextual\"/></w:rPr>\
         <w:t>text</w:t></w:r></w:p></w:body></w:document>\n"
    )
    .into_bytes();
    let document = mjx_xml::fidelity::parse(&source).expect("the authored document parses");
    let resolved = mjx_schema_gate::markup_compatibility_resolved(&document)
        .expect("nothing here is an unsatisfied mc:MustUnderstand");
    let text = String::from_utf8(resolved).expect("the resolved view is UTF-8");
    assert!(
        !text.contains("ligatures"),
        "an ignorable element under a parent with no wildcard survived resolution. `CT_RPr` has no \
         `xsd:any`, so keeping it is the failure MJXOFF-196's option 2 was rejected for:\n{text}"
    );
    assert!(
        text.contains("<w:rPr"),
        "the emptied `w:rPr` was dropped as well. It is not a wildcard slot — every child of \
         `CT_RPr` is `minOccurs=\"0\"`, so an empty one is valid and there is nothing to \
         remove:\n{text}"
    );

    let Some(harness) = mjx_schema_gate::harness() else {
        note_if_the_schema_half_is_skipping();
        return;
    };
    let schema = mjx_schema_gate::schema_for_namespace(WML).expect("WordprocessingML is modelled");
    let work = mjx_schema_gate::WorkDir::new("mce-seam-no-wildcard");
    let path = work.path().join("document.xml");
    std::fs::write(&path, text.as_bytes()).expect("writing the resolved view");
    assert!(
        harness.validate(schema, WML, &path).is_none(),
        "the resolved WordprocessingML view does not validate: {}",
        harness.validate(schema, WML, &path).unwrap_or_default()
    );
}

// ---------------------------------------------------------------------------------------------
// What the three-view diagnosis collapsed to
// ---------------------------------------------------------------------------------------------

/// The reproduction MJXOFF-196 was diagnosed with, kept and inverted.
///
/// It measured three views of one worksheet. Two of them are still worth stating, and the third —
/// *fully resolved, rejected* — is the defect and is now green:
///
/// | View | Verdict | What it establishes |
/// |---|---|---|
/// | as a producer writes it | rejected — `mc:Ignorable` *is not allowed* | why the gate resolves at all |
/// | as the gate resolves it | **validates** | the seam is closed |
#[test]
fn the_two_views_that_remain() {
    let Some(harness) = mjx_schema_gate::harness() else {
        note_if_the_schema_half_is_skipping();
        return;
    };
    let schema = mjx_schema_gate::schema_for_namespace(SML_NAMESPACE)
        .expect("SpreadsheetML is a modelled schema");
    let work = mjx_schema_gate::WorkDir::new("mce-seam-two-views");

    let as_written = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n\
         <worksheet xmlns=\"{SML_NAMESPACE}\" \
         xmlns:mc=\"http://schemas.openxmlformats.org/markup-compatibility/2006\" \
         xmlns:demo=\"{IGNORABLE_NAMESPACE}\" mc:Ignorable=\"demo\">\
         <sheetData><row r=\"1\"><c r=\"A1\"><v>1</v>{}</c></row></sheetData></worksheet>\n",
        extension_list("")
    )
    .into_bytes();
    let document = mjx_xml::fidelity::parse(&as_written).expect("the authored worksheet parses");
    let resolved = mjx_schema_gate::markup_compatibility_resolved(&document)
        .expect("nothing here is an unsatisfied mc:MustUnderstand");

    let verdict = |name: &str, bytes: &[u8]| -> Option<String> {
        let path = work.path().join(name);
        std::fs::write(&path, bytes).expect("writing a view");
        harness.validate(schema, SML_NAMESPACE, &path)
    };

    let written_report = verdict("as-written.xml", &as_written).unwrap_or_else(|| {
        panic!(
            "the worksheet as a producer writes it validates with `mc:Ignorable` still on it, so \
             the gate has no reason to resolve markup compatibility and this whole seam is gone"
        )
    });
    assert!(
        written_report.contains("Ignorable"),
        "the authored view fails for a reason other than its compatibility \
         attribute:\n{written_report}"
    );

    if let Some(report) = verdict("mce-resolved.xml", &resolved) {
        panic!(
            "the resolved view of an ignorable extension is rejected again. MJXOFF-196 is back: \
             `crates/mjx-schema-gate/src/wildcard_slots.rs` no longer names the element the \
             schema requires content in.\n{report}"
        );
    }

    let text = String::from_utf8(resolved).expect("the resolved view is UTF-8");
    assert!(
        !text.contains("demo:note"),
        "the ignorable child survived resolution, which is not what ECMA-376 Part 3 says and not \
         what this fix does:\n{text}"
    );
    // `<ext ` with the space, because `<extLst` starts with `<ext` and must stay.
    assert!(
        !text.contains("<ext "),
        "the `<ext>` husk survived and the resolved view still validated, so `CT_Extension` is no \
         longer the shape this fix is written against:\n{text}"
    );
    assert!(
        text.contains("<extLst"),
        "the whole `extLst` was dropped. Only the emptied slot goes; `CT_ExtensionList` declares \
         `ext` with `minOccurs=\"0\"`, so an empty list is valid and removing it would be a \
         cascade nothing asked for:\n{text}"
    );
    println!(
        "the MCE/CT_Extension seam, closed:\n  as written:        {}\n  MCE-resolved:      \
         validates\n  resolved view:     {}",
        written_report.trim(),
        text.trim()
    );
}
