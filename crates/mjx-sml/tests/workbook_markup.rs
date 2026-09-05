//! The workbook part's fidelity contract (MJXOFF-100): what survives a read, an edit and a write,
//! and what a reader is told — proved **without naming the packaging crate once**.
//!
//! # Why this suite links no package
//!
//! The ticket's fourth "done when" clause is that `mjx-sml`'s half of this child parses and re-emits
//! `xl/workbook.xml` with no `mjx-opc` type in its public signature. A dependency-graph check says
//! that from the outside — `xtask/tests/layering.rs` reads `cargo metadata` — but it cannot say that
//! the *reading path* is reachable without a package, which is the property an embedded workbook
//! inside a `.pptx` actually needs. This file says it from the inside: every case here starts from
//! **bytes**, and the packaging crate's name does not appear below. A future edit that reached for a
//! `PartName` to read a workbook would have to add the import, and the case that pins its absence
//! ([`this_suite_names_no_package_type`]) fails when it does.
//!
//! # The traps these cases are written against
//!
//! * *"`sample.xlsx`'s `workbook.xml` round-trips byte-identically"* is satisfied by a model that
//!   reads nothing and echoes the bytes — which is exactly what `mjx-opc`'s part-level
//!   copy-on-write already does one layer down, before a line of this crate runs. So every
//!   round-trip case here goes **through the model**: `WorkbookPart::read_part`, then
//!   `ToXml::write_back`, then serialize. And every one of them also reads values out, so a model
//!   that parsed into an empty shell would fail on the assertions rather than pass on the bytes.
//! * *"the LibreOffice extension survives"* is satisfied by any writer at all when the writer is a
//!   `memcpy`. [`the_libreoffice_extension_survives_prefix_and_all`] therefore asserts on the
//!   extension's bytes **after an unrelated edit has re-flowed the part**, which is the only state
//!   in which preserving it is work.
//! * *"unknown attributes are preserved"* is satisfied vacuously by a fixture with none.
//!   `sample.xlsx`'s `workbookPr` carries `dateCompatibility`, which the Transitional `sml.xsd` does
//!   not declare and this crate therefore has no accessor for, and
//!   [`an_undeclared_attribute_survives_an_edit_to_its_own_element`] edits that very element.
//!
//! # The copy of `sample.xlsx`'s part, and why it cannot rot
//!
//! [`SAMPLE_WORKBOOK_PART`] is a byte-exact copy of `tests/fixtures/sample.xlsx`'s
//! `xl/workbook.xml`, embedded here because reaching into the package would need the very crate this
//! suite exists to do without. A copy that nobody checks is a copy that drifts, so
//! `crates/mjx-xlsx/tests/workbook_part.rs`'s
//! `the_markup_suites_copy_of_the_workbook_part_is_still_the_fixtures` opens the real fixture and
//! compares the two, in the crate that is allowed to.

use mjx_ooxml_core::{RawDocument, RawNode, ToXml};
use mjx_ooxml_types::spreadsheetml::{
    CalculationMode, ObjectDisplay, SheetState, UpdateLinksBehavior,
};
use mjx_sml::{BuiltInName, ReferenceMode, WorkbookContent, WorkbookPart};

/// A byte-exact copy of `tests/fixtures/sample.xlsx`'s `xl/workbook.xml`. See the module docs for
/// why it lives here and for what stops it drifting.
const SAMPLE_WORKBOOK_PART: &[u8] = concat!(
    r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#,
    "\n",
    r#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><fileVersion appName="Calc"/><workbookPr backupFile="false" showObjects="all" dateCompatibility="false"/><workbookProtection/><bookViews><workbookView showHorizontalScroll="true" showVerticalScroll="true" showSheetTabs="true" xWindow="0" yWindow="0" windowWidth="16384" windowHeight="8192" tabRatio="500" firstSheet="0" activeTab="0"/></bookViews><sheets><sheet name="sample" sheetId="1" state="visible" r:id="rId3"/></sheets><calcPr iterateCount="100" refMode="A1" iterate="false" iterateDelta="0.001"/><extLst><ext xmlns:loext="http://schemas.libreoffice.org/" uri="{7626C862-2A13-11E5-B345-FEFF819CDC9F}"><loext:extCalcPr stringRefSyntax="CalcA1"/></ext></extLst></workbook>"#
)
.as_bytes();

/// A workbook authored to disagree with every naive answer, in one part:
///
/// * three sheets whose **list order, `@sheetId` order and `r:id` order all differ**;
/// * a `veryHidden` sheet and a `hidden` one;
/// * a global defined name, a sheet-scoped one, a `_xlnm.Print_Area` and one whose
///   `@localSheetId` is out of range;
/// * a `calcPr` in `R1C1` reference mode with attributes in non-schema order;
/// * a single-quoted attribute, a doubled space in a start tag, a comment between two children, and
///   an `extLst` holding a foreign element — none of which a rebuild from a decoded model would
///   reproduce.
const DISCRIMINATING: &[u8] = br#"<x:workbook xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:rel="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><x:fileVersion appName='Calc' rupBuild="9999"/><x:bookViews><x:workbookView  activeTab="2" tabRatio='750'/></x:bookViews><x:sheets><x:sheet name="Summary" sheetId="7" rel:id="rId3"/><!-- between --><x:sheet name="Hidden Data" sheetId="2" state="hidden" rel:id="rId1"/><x:sheet name="Secret" sheetId="5" state="veryHidden" rel:id="rId2"/></x:sheets><x:definedNames><x:definedName name="TaxRate">Summary!$B$1</x:definedName><x:definedName name="LocalRange" localSheetId="1">'Hidden Data'!$A$1:$C$9</x:definedName><x:definedName name="_xlnm.Print_Area" localSheetId="0">Summary!$A$1:$D$20</x:definedName><x:definedName name="OutOfRangeScope" localSheetId="9">Summary!$A$1</x:definedName></x:definedNames><x:calcPr iterateCount="42" refMode="R1C1" iterate="true"/><x:extLst><x:ext xmlns:q="urn:q" uri="{QQ}"><q:keep note="kept"/></x:ext></x:extLst></x:workbook>"#;

// -------------------------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------------------------

/// Parses `markup` and reads its `x:workbook`.
fn read(markup: &[u8]) -> (RawDocument, WorkbookPart) {
    let document = mjx_xml::fidelity::parse(markup).expect("the workbook part parses");
    let part = WorkbookPart::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:workbook");
    (document, part)
}

/// Reads `markup`, writes the model straight back, and serializes — the whole model round trip.
fn round_trip(markup: &[u8]) -> Vec<u8> {
    let (mut document, part) = read(markup);
    part.write_back(&mut document.root, &mut document.interner);
    mjx_xml::fidelity::serialize_to_vec(&document)
}

/// The three sheet entries of [`DISCRIMINATING`], as `(name, sheetId, state, r:id)`.
fn discriminating_sheets(
    document: &RawDocument,
    part: &WorkbookPart,
) -> Vec<(String, Option<u32>, SheetState, Option<String>)> {
    let interner = &document.interner;
    let prefix = part.relationship_prefix(interner);
    part.sheets()
        .expect("a sheet list")
        .entries()
        .map(|entry| {
            (
                entry
                    .name(interner)
                    .expect("a name")
                    .map(std::borrow::Cow::into_owned)
                    .unwrap_or_default(),
                entry.sheet_id(interner).expect("a sheet id"),
                entry.visibility(interner).expect("a visibility"),
                entry.relationship_id(interner, prefix).expect("an r:id"),
            )
        })
        .collect()
}

// -------------------------------------------------------------------------------------------
// Tier 1 — the part re-emits byte for byte, through the model
// -------------------------------------------------------------------------------------------

/// Both parts survive a full read-and-rebuild byte for byte, **and** their contents read back.
///
/// The second half is what stops this passing on a model that parsed into nothing: the sample's
/// seven exercised slots are named and its sheet is read out, and the discriminating part's three
/// sheets and four defined names are counted.
#[test]
fn both_workbook_parts_re_emit_byte_for_byte_and_read_back() {
    assert_eq!(
        round_trip(SAMPLE_WORKBOOK_PART),
        SAMPLE_WORKBOOK_PART,
        "sample.xlsx's workbook.xml must come back exactly as it went in"
    );
    assert_eq!(
        round_trip(DISCRIMINATING),
        DISCRIMINATING,
        "a single-quoted attribute, a doubled space, a comment and a foreign extension must all \
         survive the model"
    );

    let (document, part) = read(SAMPLE_WORKBOOK_PART);
    let present: Vec<Option<&str>> = part
        .content()
        .iter()
        .map(|child| match child {
            WorkbookContent::FileVersion(_) => Some("fileVersion"),
            WorkbookContent::Properties(_) => Some("workbookPr"),
            WorkbookContent::Protection(_) => Some("workbookProtection"),
            WorkbookContent::BookViews(_) => Some("bookViews"),
            WorkbookContent::Sheets(_) => Some("sheets"),
            WorkbookContent::Calculation(_) => Some("calcPr"),
            WorkbookContent::Raw(_) => None,
            other => panic!("sample.xlsx writes no {other:?}"),
        })
        .collect();
    assert_eq!(
        present,
        vec![
            Some("fileVersion"),
            Some("workbookPr"),
            Some("workbookProtection"),
            Some("bookViews"),
            Some("sheets"),
            Some("calcPr"),
            None, // extLst — deliberately unmodelled
        ],
        "sample.xlsx exercises seven of the nineteen slots, six modelled and extLst raw"
    );
    let sheets = part.sheets().expect("a sheet list");
    assert_eq!(sheets.len(), 1);

    let (document2, part2) = read(DISCRIMINATING);
    assert_eq!(part2.sheets().expect("a sheet list").len(), 3);
    assert_eq!(part2.defined_names().expect("defined names").len(), 4);
    let _ = (&document, &document2);
}

/// The LibreOffice extension comes back byte-identical — including its `loext:` prefix and the
/// `xmlns:loext` declaration that binds it — **after an unrelated edit has re-flowed the part**.
///
/// Renaming the sheet is what makes this a real assertion. Without an edit, the whole part is
/// copied out of its own source buffer and preserving an extension takes no work at all.
#[test]
fn the_libreoffice_extension_survives_prefix_and_all() {
    const EXTENSION: &[u8] = br#"<extLst><ext xmlns:loext="http://schemas.libreoffice.org/" uri="{7626C862-2A13-11E5-B345-FEFF819CDC9F}"><loext:extCalcPr stringRefSyntax="CalcA1"/></ext></extLst>"#;
    assert!(
        SAMPLE_WORKBOOK_PART
            .windows(EXTENSION.len())
            .any(|window| window == EXTENSION),
        "the copy of the fixture must actually carry the extension this case is about"
    );

    let (mut document, mut part) = read(SAMPLE_WORKBOOK_PART);
    part.sheets_mut()
        .expect("a sheet list")
        .entry_mut(0)
        .expect("one entry")
        .set_name(&mut document.interner, Some("renamed"));
    part.write_back(&mut document.root, &mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);

    assert!(
        written.windows(EXTENSION.len()).any(|w| w == EXTENSION),
        "the extension must survive an edit elsewhere in the part, prefix and all:\n{}",
        String::from_utf8_lossy(&written)
    );
    assert!(
        written.windows(9).any(|w| w == b"\"renamed\""),
        "the edit itself must have landed"
    );

    // …and the extension really is the unknown bucket, not a modelled slot.
    let (_, unedited) = read(SAMPLE_WORKBOOK_PART);
    let last = unedited.content().last().expect("a last child");
    let WorkbookContent::Raw(RawNode::Element(element)) = last else {
        panic!("the extension list must be an unmodelled child, found {last:?}");
    };
    let (document, _) = read(SAMPLE_WORKBOOK_PART);
    assert_eq!(document.interner.resolve(element.name.local), "extLst");
}

/// `workbookPr/@dateCompatibility` — which the Transitional `sml.xsd` does not declare and this
/// crate has no accessor for — survives an edit to **its own element**.
#[test]
fn an_undeclared_attribute_survives_an_edit_to_its_own_element() {
    let (mut document, mut part) = read(SAMPLE_WORKBOOK_PART);
    let properties = part.properties_mut().expect("workbookPr");
    assert!(
        !properties
            .create_backup_file(&document.interner)
            .expect("backupFile"),
        "the fixture writes backupFile=\"false\""
    );
    properties.set_create_backup_file(&mut document.interner, Some(true));
    part.write_back(&mut document.root, &mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);
    let text = String::from_utf8(written).expect("utf-8");

    assert!(
        text.contains(r#"<workbookPr backupFile="true" showObjects="all" dateCompatibility="false"/>"#),
        "the edited attribute is rewritten in place and the undeclared one keeps its position:\n{text}"
    );
}

// -------------------------------------------------------------------------------------------
// The sheet list — the relationship names the part
// -------------------------------------------------------------------------------------------

/// Every sheet entry reads back what the file wrote, and the three orderings really do disagree.
///
/// The last two assertions are the point: if list order agreed with `r:id` order, or `@sheetId`
/// agreed with the digits in `rId`, this fixture would not be able to catch a resolver that used
/// the wrong one.
#[test]
fn the_sheet_list_is_read_in_list_order_and_the_three_orderings_disagree() {
    let (document, part) = read(DISCRIMINATING);
    let sheets = discriminating_sheets(&document, &part);

    assert_eq!(
        sheets,
        vec![
            (
                "Summary".to_owned(),
                Some(7),
                SheetState::Visible,
                Some("rId3".to_owned())
            ),
            (
                "Hidden Data".to_owned(),
                Some(2),
                SheetState::Hidden,
                Some("rId1".to_owned())
            ),
            (
                "Secret".to_owned(),
                Some(5),
                SheetState::VeryHidden,
                Some("rId2".to_owned())
            ),
        ]
    );

    let ids: Vec<Option<u32>> = sheets.iter().map(|sheet| sheet.1).collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    assert_ne!(ids, sorted, "@sheetId order must differ from list order");

    let references: Vec<&str> = sheets
        .iter()
        .map(|sheet| sheet.3.as_deref().expect("an r:id"))
        .collect();
    let mut sorted_references = references.clone();
    sorted_references.sort_unstable();
    assert_ne!(
        references, sorted_references,
        "relationship order must differ from list order"
    );
    for (sheet_id, reference) in ids.iter().zip(&references) {
        assert_ne!(
            format!("rId{}", sheet_id.expect("a sheet id")),
            *reference,
            "no entry's @sheetId may coincide with the digits of its own r:id, or a resolver that \
             used @sheetId would accidentally be right"
        );
    }
}

/// A comment between two `sheet` elements does not shift the tab numbering a caller sees.
#[test]
fn an_unmodelled_node_between_entries_does_not_shift_the_tab_index() {
    let (document, part) = read(DISCRIMINATING);
    let sheets = part.sheets().expect("a sheet list");
    assert_eq!(sheets.len(), 3, "the comment is not a tab");
    let second = sheets.entries().nth(1).expect("a second tab");
    assert_eq!(
        second.name(&document.interner).expect("a name").as_deref(),
        Some("Hidden Data"),
        "the entry after the comment is the second tab, not the third"
    );
}

/// An entry with no `r:id` reads as one, rather than as an error or a guess.
#[test]
fn an_entry_that_names_no_relationship_is_read_as_naming_none() {
    let (document, part) = read(
        br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets><sheet name="Orphan" sheetId="1"/></sheets></workbook>"#,
    );
    let entry = part
        .sheets()
        .expect("a sheet list")
        .entries()
        .next()
        .expect("an entry");
    assert_eq!(
        part.relationship_prefix(&document.interner),
        None,
        "the part binds the relationship namespace nowhere"
    );
    assert_eq!(
        entry
            .relationship_id(&document.interner, None)
            .expect("no error"),
        None
    );
    assert_eq!(entry.sheet_id(&document.interner).expect("an id"), Some(1));
}

/// Renaming one sheet rewrites one attribute and leaves every other byte of the part alone.
#[test]
fn renaming_a_sheet_changes_one_attribute_and_nothing_else() {
    let (mut document, mut part) = read(DISCRIMINATING);
    part.sheets_mut()
        .expect("a sheet list")
        .entry_mut(1)
        .expect("the second tab")
        .set_name(&mut document.interner, Some("Q1 & Q2"));
    part.write_back(&mut document.root, &mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);

    let expected = String::from_utf8_lossy(DISCRIMINATING)
        .replace(r#"name="Hidden Data""#, r#"name="Q1 &amp; Q2""#);
    assert_eq!(
        String::from_utf8_lossy(&written),
        expected,
        "one attribute value changes; the single-quoted fileVersion attribute, the doubled space in \
         workbookView, the comment and the extension are all still there"
    );
}

// -------------------------------------------------------------------------------------------
// Properties, calculation settings and defined names — reported, never derived
// -------------------------------------------------------------------------------------------

/// `workbookPr` reports what the file wrote and the schema's own defaults for what it did not.
#[test]
fn workbook_properties_report_the_file_and_then_the_schema_default() {
    let (document, part) = read(SAMPLE_WORKBOOK_PART);
    let interner = &document.interner;
    let properties = part.properties().expect("workbookPr");

    assert!(!properties.create_backup_file(interner).expect("backupFile"));
    assert_eq!(
        properties.object_display(interner).expect("showObjects"),
        ObjectDisplay::All
    );
    // Absent in the file: the schema default, returned and never written.
    assert!(
        !properties
            .uses_1904_date_system(interner)
            .expect("date1904"),
        "sample.xlsx writes no date1904, and the 1900 system is the schema default"
    );
    assert_eq!(
        properties
            .update_links_behavior(interner)
            .expect("updateLinks"),
        UpdateLinksBehavior::UserSet
    );
    assert_eq!(properties.code_name(interner).expect("codeName"), None);
}

/// `date1904` is exposed and nothing acts on it — a workbook that sets it reads it back set, and
/// the cell values in the same package are untouched by that.
#[test]
fn the_1904_date_system_is_reported_rather_than_applied() {
    let (document, part) = read(
        br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><workbookPr date1904="1"/><sheets/></workbook>"#,
    );
    assert!(
        part.properties()
            .expect("workbookPr")
            .uses_1904_date_system(&document.interner)
            .expect("date1904"),
        "`1` is a legal xsd:boolean and means the Macintosh epoch"
    );
}

/// `calcPr` is read exactly as written — including an attribute order that is not the schema's —
/// and nothing here derives `calcId` or clears `calcCompleted`.
#[test]
fn calculation_properties_are_reported_and_never_derived() {
    let (document, part) = read(DISCRIMINATING);
    let interner = &document.interner;
    let calc = part.calculation_properties().expect("calcPr");

    assert_eq!(
        calc.reference_mode(interner).expect("refMode"),
        ReferenceMode::R1C1
    );
    assert_eq!(calc.iteration_limit(interner).expect("iterateCount"), 42);
    assert!(calc
        .iterate_on_circular_references(interner)
        .expect("iterate"));
    assert_eq!(
        calc.calculation_engine_id(interner).expect("calcId"),
        None,
        "no calcId in the file means no calcId reported — not one this crate made up"
    );
    assert_eq!(
        calc.calculation_mode(interner).expect("calcMode"),
        CalculationMode::Auto,
        "absent means the schema default"
    );
    assert_eq!(
        calc.iteration_convergence_delta(interner)
            .expect("iterateDelta"),
        0.001
    );

    // sample.xlsx writes its four calcPr attributes out of schema order; they come back that way.
    let text = String::from_utf8_lossy(SAMPLE_WORKBOOK_PART).into_owned();
    assert!(
        text.contains(
            r#"<calcPr iterateCount="100" refMode="A1" iterate="false" iterateDelta="0.001"/>"#
        ),
        "the fixture's own attribute order is what this crate must not tidy"
    );
    assert_eq!(round_trip(SAMPLE_WORKBOOK_PART), SAMPLE_WORKBOOK_PART);
}

/// The four defined names read back with their scope, their formula text and their built-in
/// identity — and the out-of-range scope is reported rather than repaired.
#[test]
fn defined_names_are_read_with_their_scope_and_never_renumbered() {
    let (document, part) = read(DISCRIMINATING);
    let interner = &document.interner;
    let names: Vec<_> = part
        .defined_names()
        .expect("definedNames")
        .names()
        .map(|name| {
            (
                name.name(interner).expect("a name").into_owned(),
                name.local_sheet_index(interner).expect("a localSheetId"),
                name.definition().to_owned(),
                name.built_in(interner).expect("a built-in check"),
            )
        })
        .collect();

    assert_eq!(
        names,
        vec![
            ("TaxRate".to_owned(), None, "Summary!$B$1".to_owned(), None),
            (
                "LocalRange".to_owned(),
                Some(1),
                "'Hidden Data'!$A$1:$C$9".to_owned(),
                None
            ),
            (
                "_xlnm.Print_Area".to_owned(),
                Some(0),
                "Summary!$A$1:$D$20".to_owned(),
                Some(BuiltInName::PrintArea)
            ),
            (
                "OutOfRangeScope".to_owned(),
                Some(9),
                "Summary!$A$1".to_owned(),
                None
            ),
        ]
    );

    // The workbook has three sheets; a localSheetId of 9 names none of them. It is reported as the
    // number the file wrote, and the part still writes back unchanged.
    assert_eq!(part.sheets().expect("a sheet list").len(), 3);
    assert_eq!(round_trip(DISCRIMINATING), DISCRIMINATING);
}

/// A defined name whose definition is edited keeps every other name's bytes.
#[test]
fn editing_one_definition_leaves_the_others_alone() {
    let (mut document, mut part) = read(DISCRIMINATING);
    part.defined_names_mut()
        .expect("definedNames")
        .names_mut()
        .next()
        .expect("the first name")
        .set_definition("Summary!$C$1");
    part.write_back(&mut document.root, &mut document.interner);
    let written = mjx_xml::fidelity::serialize_to_vec(&document);
    let expected = String::from_utf8_lossy(DISCRIMINATING).replace("Summary!$B$1", "Summary!$C$1");
    assert_eq!(String::from_utf8_lossy(&written), expected);
}

/// `bookViews` reads its window geometry, and an absent `visibility` is the schema's default.
#[test]
fn the_book_view_reports_its_geometry() {
    let (document, part) = read(SAMPLE_WORKBOOK_PART);
    let interner = &document.interner;
    let view = part
        .book_views()
        .expect("bookViews")
        .views()
        .next()
        .expect("a workbookView");
    assert_eq!(
        view.window_width(interner).expect("windowWidth"),
        Some(16384)
    );
    assert_eq!(
        view.window_height(interner).expect("windowHeight"),
        Some(8192)
    );
    assert_eq!(view.tab_strip_ratio(interner).expect("tabRatio"), 500);
    assert_eq!(view.active_tab_index(interner).expect("activeTab"), 0);
    assert_eq!(
        view.first_visible_tab_index(interner).expect("firstSheet"),
        0
    );
}

/// An empty `workbookProtection` is present-with-defaults, which is not the same as absent.
#[test]
fn an_empty_protection_element_is_present_rather_than_absent() {
    let (document, part) = read(SAMPLE_WORKBOOK_PART);
    let protection = part
        .protection()
        .expect("sample.xlsx writes <workbookProtection/>");
    assert!(
        !protection
            .lock_structure(&document.interner)
            .expect("lockStructure"),
        "absent means the schema default, and the element still writes back as an empty tag"
    );
    assert_eq!(round_trip(SAMPLE_WORKBOOK_PART), SAMPLE_WORKBOOK_PART);
}

// -------------------------------------------------------------------------------------------
// Untrusted input
// -------------------------------------------------------------------------------------------

/// A part whose root is not `x:workbook` is a question, not a failure — and neither is one whose
/// attributes are nonsense.
#[test]
fn a_malformed_workbook_is_reported_rather_than_panicked_on() {
    let other = mjx_xml::fidelity::parse(
        br#"<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"/>"#,
    )
    .expect("parses");
    assert!(WorkbookPart::read_part(&other).expect("no error").is_none());

    let (document, part) = read(
        br#"<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheets><sheet name="S" sheetId="not-a-number" state="quiteHidden"/></sheets></workbook>"#,
    );
    let entry = part
        .sheets()
        .expect("a sheet list")
        .entries()
        .next()
        .expect("an entry");
    assert!(
        entry.sheet_id(&document.interner).is_err(),
        "a sheetId that is not an unsignedInt is an error the caller is told about, not a silent 0"
    );
    assert!(
        entry.visibility(&document.interner).is_err(),
        "a state outside ST_SheetState is an error, not a silent Visible"
    );
    // …and the part still round-trips, because reading never repaired anything.
    let mut document = document;
    part.write_back(&mut document.root, &mut document.interner);
    assert!(
        String::from_utf8_lossy(&mjx_xml::fidelity::serialize_to_vec(&document))
            .contains(r#"state="quiteHidden""#)
    );
}

/// This file names no package type. See the module documentation for why that is the deliverable
/// and not a stylistic preference.
///
/// The needle is assembled at run time rather than written out, so that the case does not defeat
/// itself by putting the very string it forbids into the file it scans. The second assertion is the
/// positive control: the same search, over text that *does* contain the name, finds it — so a green
/// run means the scan works and the file is clean, not that the scan matches nothing.
#[test]
fn this_suite_names_no_package_type() {
    let needle = ["mjx", "_opc"].concat();
    assert_eq!(
        ["use ", needle.as_str(), "::PartName;"]
            .concat()
            .matches(needle.as_str())
            .count(),
        1,
        "the scan must be able to find the name it is looking for"
    );

    let source = include_str!("workbook_markup.rs");
    assert_eq!(
        source.matches(needle.as_str()).count(),
        0,
        "this suite reads and re-emits a whole workbook part from bytes alone; a reader path that \
         needed a package would have to name one here"
    );
}
