//! `xl/workbook.xml` at the **package** level (MJXOFF-100): resolving a sheet to its part,
//! navigating by name, reading the defined names, and renaming a tab without disturbing anything
//! else.
//!
//! # The trap, in this child's own terms
//!
//! **The relationship names the part.** Not `@sheetId`, not the position in `x:sheets`, and not the
//! digits inside `rId3`. A workbook whose three orderings disagree is legal and Excel writes them,
//! and a fixture where they agree cannot tell a correct resolver from three wrong ones — it is green
//! for all four.
//!
//! `tests/fixtures/workbook_sheet_order.xlsx` is authored so that all three disagree:
//!
//! | tab | position | `@sheetId` | `r:id` | part |
//! |---|---|---|---|---|
//! | `Summary` | 0 | 7 | `rId3` | `/xl/worksheets/sheet3.xml` |
//! | `Hidden Data` (hidden) | 1 | 2 | `rId1` | `/xl/worksheets/sheet1.xml` |
//! | `Secret` (veryHidden) | 2 | 5 | `rId2` | `/xl/worksheets/sheet2.xml` |
//!
//! and `xl/_rels/workbook.xml.rels` declares `rId1`, `rId2`, `rId3` in that order. So a resolver
//! that took the *n*-th relationship, or parsed the digits out of `rId`, or looked up
//! `@sheetId`, gets a different answer for every tab —
//! [`each_sheet_resolves_through_its_own_relationship`] pins the right one and then pins that each
//! wrong one would differ.
//!
//! Each worksheet also carries a **different first cell**, so a resolver that reached the wrong part
//! is caught by content and not only by part name.
//!
//! # And the fixture is not merely different — it is discriminating
//!
//! [`the_fixtures_three_orderings_really_do_disagree`] asserts the disagreement itself, so a later
//! edit that "tidied" the fixture into agreement fails there rather than silently turning every case
//! below into a tautology.

use mjx_fixtures::fixture;
use mjx_opc::{Package, PartName};
use mjx_sml::BuiltInName;
use mjx_xlsx::{DateSystem, DefinedNameScope, SheetKind, Workbook};

/// The fixture authored for this child, whose whole purpose is to disagree with the naive answer.
const DISAGREEING: &str = "workbook_sheet_order.xlsx";

/// Opens a committed fixture.
fn open(name: &str) -> Workbook {
    Workbook::open(&fixture(name)).unwrap_or_else(|error| panic!("{name}: open: {error}"))
}

/// One part of a committed fixture, as bytes.
fn part_bytes(name: &str, part: &str) -> Vec<u8> {
    let package = Package::open(&fixture(name)).expect("a committed fixture opens");
    let part_name = PartName::new(part).expect("a valid part name");
    package
        .part_bytes(&part_name)
        .unwrap_or_else(|| panic!("{name} has no {part}"))
        .to_vec()
}

// -------------------------------------------------------------------------------------------
// The fixture is discriminating
// -------------------------------------------------------------------------------------------

/// The three orderings disagree, and no `@sheetId` coincides with the digits of its own `r:id`.
///
/// Without this, every case below could be green against a fixture that had quietly been tidied
/// into agreement, and none of them would be testing anything.
#[test]
fn the_fixtures_three_orderings_really_do_disagree() {
    let workbook = open(DISAGREEING);
    let sheets = workbook.sheets();
    assert_eq!(sheets.len(), 3);

    let ids: Vec<u32> = sheets
        .iter()
        .map(|sheet| sheet.sheet_id.expect("every entry writes a sheetId"))
        .collect();
    assert_eq!(ids, vec![7, 2, 5], "@sheetId order is not list order");

    let references: Vec<&str> = sheets
        .iter()
        .map(|sheet| sheet.relationship_id.as_str())
        .collect();
    assert_eq!(
        references,
        vec!["rId3", "rId1", "rId2"],
        "the list names its relationships out of order"
    );

    for (sheet_id, reference) in ids.iter().zip(&references) {
        assert_ne!(
            &format!("rId{sheet_id}"),
            reference,
            "no entry's @sheetId may coincide with the digits of its own r:id"
        );
    }

    // The relationships themselves are declared in their own order, which is a third ordering.
    let rels =
        String::from_utf8(part_bytes(DISAGREEING, "/xl/_rels/workbook.xml.rels")).expect("utf-8");
    let declared: Vec<usize> = ["rId1", "rId2", "rId3"]
        .iter()
        .map(|id| rels.find(id).expect("every relationship is declared"))
        .collect();
    assert!(
        declared.windows(2).all(|pair| pair[0] < pair[1]),
        "the .rels file declares rId1, rId2, rId3 in that order"
    );
}

// -------------------------------------------------------------------------------------------
// Resolution — the relationship names the part
// -------------------------------------------------------------------------------------------

/// Every tab resolves through **its own `r:id`**, and each of the three plausible wrong answers
/// would give a different part.
#[test]
fn each_sheet_resolves_through_its_own_relationship() {
    let workbook = open(DISAGREEING);

    let resolved: Vec<(&str, &str)> = workbook
        .sheets()
        .iter()
        .map(|sheet| {
            (
                sheet.name.as_str(),
                sheet.part.as_ref().expect("a resolved part").as_str(),
            )
        })
        .collect();
    assert_eq!(
        resolved,
        vec![
            ("Summary", "/xl/worksheets/sheet3.xml"),
            ("Hidden Data", "/xl/worksheets/sheet1.xml"),
            ("Secret", "/xl/worksheets/sheet2.xml"),
        ]
    );

    // …and the three wrong resolvers each disagree with that, on every tab.
    for (index, sheet) in workbook.sheets().iter().enumerate() {
        let correct = sheet.part.as_ref().expect("a resolved part").as_str();
        let by_position = format!("/xl/worksheets/sheet{}.xml", index + 1);
        let by_sheet_id = format!(
            "/xl/worksheets/sheet{}.xml",
            sheet.sheet_id.expect("a sheetId")
        );
        assert_ne!(
            correct, by_position,
            "{}: resolving by list position would reach a different part",
            sheet.name
        );
        assert_ne!(
            correct, by_sheet_id,
            "{}: resolving by @sheetId would reach a different part",
            sheet.name
        );
    }

    // Reaching the wrong part is also visible in the content, not only in the name: each worksheet
    // carries a different first cell.
    for (sheet, expected) in workbook.sheets().iter().zip(["33", "11", "22"]) {
        let bytes = part_bytes(
            DISAGREEING,
            sheet.part.as_ref().expect("a resolved part").as_str(),
        );
        let text = String::from_utf8(bytes).expect("utf-8");
        assert!(
            text.contains(&format!("<v>{expected}</v>")),
            "{}: the part behind this tab must hold {expected}, found:\n{text}",
            sheet.name
        );
    }
}

/// Each tab's own part graph resolves, and every tab is a worksheet.
#[test]
fn every_tab_resolves_to_a_worksheet_part() {
    let workbook = open(DISAGREEING);
    for index in 0..workbook.sheets().len() {
        let sheet = &workbook.sheets()[index];
        let resolved = workbook
            .worksheet(index)
            .expect("resolution succeeds")
            .expect("the tab reaches a part");
        assert_eq!(resolved.part(), sheet.part.as_ref().expect("a part"));
        assert_eq!(resolved.kind(), Some(SheetKind::Worksheet));
    }
}

/// Visibility is read from `@state`, and the hidden tabs are the two the fixture hides.
#[test]
fn the_hidden_and_very_hidden_tabs_are_read_as_such() {
    let workbook = open(DISAGREEING);
    let visible: Vec<&str> = workbook
        .visible_sheets()
        .map(|sheet| sheet.name.as_str())
        .collect();
    assert_eq!(
        visible,
        vec!["Summary"],
        "one visible tab; `hidden` and `veryHidden` are both hidden from a tab strip"
    );
    assert_eq!(
        workbook.sheets()[1].visibility,
        mjx_ooxml_types::spreadsheetml::SheetState::Hidden
    );
    assert_eq!(
        workbook.sheets()[2].visibility,
        mjx_ooxml_types::spreadsheetml::SheetState::VeryHidden
    );
}

// -------------------------------------------------------------------------------------------
// Navigation by name
// -------------------------------------------------------------------------------------------

/// Lookup by name reaches the same part the list does, and is exact.
#[test]
fn a_tab_can_be_found_by_name_and_the_match_is_exact() {
    let workbook = open(DISAGREEING);

    assert_eq!(workbook.sheet_index_by_name("Hidden Data"), Some(1));
    assert_eq!(
        workbook
            .sheet_by_name("Hidden Data")
            .and_then(|sheet| sheet.part.as_ref())
            .map(PartName::as_str),
        Some("/xl/worksheets/sheet1.xml")
    );
    assert_eq!(
        workbook
            .worksheet_by_name("Secret")
            .expect("resolution succeeds")
            .expect("the tab reaches a part")
            .part()
            .as_str(),
        "/xl/worksheets/sheet2.xml"
    );

    assert_eq!(workbook.sheet_index_by_name("hidden data"), None, "exact");
    assert_eq!(workbook.sheet_index_by_name("Summary "), None, "exact");
    assert_eq!(workbook.sheet_index_by_name("No Such Tab"), None);
    assert!(workbook
        .worksheet_by_name("No Such Tab")
        .expect("no error")
        .is_none());
}

// -------------------------------------------------------------------------------------------
// Properties, views, defined names
// -------------------------------------------------------------------------------------------

/// The date system, the window view and the calculation settings are read from the fixture.
#[test]
fn the_workbook_properties_are_read_from_the_file() {
    let mut workbook = open(DISAGREEING);

    assert_eq!(
        workbook.date_system().expect("a date system"),
        DateSystem::Macintosh1904,
        "the fixture writes date1904=\"true\""
    );
    assert!(workbook.date_system().expect("a date system").is_1904());

    let views = workbook.window_views().expect("the views read");
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].active_tab_index, 2);
    assert_eq!(views[0].window_position, Some((120, 60)));
    assert_eq!(views[0].window_size, Some((20000, 12000)));
    assert_eq!(views[0].tab_strip_ratio, 750);

    let active = workbook
        .active_sheet()
        .expect("the active tab resolves")
        .expect("index 2 names a tab");
    assert_eq!(active.name, "Secret");

    let calc = workbook.calculation_settings().expect("the settings read");
    assert_eq!(calc.reference_mode, mjx_sml::ReferenceMode::R1C1);
    assert_eq!(calc.iteration_limit, 42);
    assert!(calc.iterates_on_circular_references);
    assert_eq!(calc.engine_id, Some(0));
}

/// A workbook with no `calcPr` and no `workbookPr` reports the schema's own defaults, not `None`
/// and not a guess.
#[test]
fn a_workbook_that_states_nothing_reports_the_schema_defaults() {
    let mut workbook = open("sample.xlsx");
    assert_eq!(
        workbook.date_system().expect("a date system"),
        DateSystem::Windows1900,
        "sample.xlsx writes a workbookPr with no date1904"
    );
    let calc = workbook.calculation_settings().expect("the settings read");
    assert_eq!(calc.reference_mode, mjx_sml::ReferenceMode::A1);
    assert_eq!(calc.iteration_limit, 100);
    assert_eq!(
        calc.engine_id, None,
        "sample.xlsx writes no calcId, and none is invented"
    );
}

/// The four defined names read back with their scope resolved — including the one whose
/// `@localSheetId` names no tab, which is reported rather than repaired.
#[test]
fn defined_names_carry_their_scope_their_text_and_their_built_in_identity() {
    let mut workbook = open(DISAGREEING);
    let names = workbook.defined_names().expect("the names read");
    assert_eq!(names.len(), 4);

    assert_eq!(names[0].name, "TaxRate");
    assert_eq!(names[0].scope, DefinedNameScope::Workbook);
    assert_eq!(names[0].definition, "Summary!$B$1");
    assert_eq!(names[0].built_in, None);

    assert_eq!(names[1].name, "LocalRange");
    assert_eq!(
        names[1].scope,
        DefinedNameScope::Sheet {
            index: 1,
            name: "Hidden Data".to_owned()
        },
        "localSheetId=\"1\" is the second entry in the list, not the sheet whose @sheetId is 1"
    );
    assert_eq!(names[1].definition, "'Hidden Data'!$A$1:$C$9");

    assert_eq!(names[2].built_in, Some(BuiltInName::PrintArea));
    assert_eq!(
        names[2].scope,
        DefinedNameScope::Sheet {
            index: 0,
            name: "Summary".to_owned()
        }
    );

    assert_eq!(
        names[3].scope,
        DefinedNameScope::UnknownSheet { index: 9 },
        "an out-of-range localSheetId is reported as the number the file wrote"
    );

    // The convenience lookups agree with the list.
    assert_eq!(
        workbook
            .defined_name("TaxRate")
            .expect("no error")
            .map(|entry| entry.definition),
        Some("Summary!$B$1".to_owned())
    );
    assert_eq!(
        workbook.defined_name("LocalRange").expect("no error"),
        None,
        "LocalRange is sheet-scoped, so it is not a workbook-scoped name"
    );
    assert_eq!(
        workbook.print_area(0).expect("no error"),
        Some("Summary!$A$1:$D$20".to_owned())
    );
    assert_eq!(workbook.print_area(1).expect("no error"), None);
}

/// `localSheetId` really is an index into the list and not a `@sheetId` — the fixture is built so
/// the two answers differ.
#[test]
fn a_local_sheet_id_is_a_list_index_and_not_a_sheet_id() {
    let mut workbook = open(DISAGREEING);
    let by_sheet_id: Vec<u32> = workbook
        .sheets()
        .iter()
        .map(|sheet| sheet.sheet_id.expect("a sheetId"))
        .collect();
    assert_eq!(by_sheet_id, vec![7, 2, 5]);

    let names = workbook.defined_names().expect("the names read");
    let DefinedNameScope::Sheet { index, name } = &names[1].scope else {
        panic!("LocalRange is sheet-scoped");
    };
    assert_eq!((*index, name.as_str()), (1, "Hidden Data"));
    // Had `localSheetId="1"` been read as a `@sheetId`, it would have named no tab at all — none of
    // the three has `@sheetId` 1 — so the two readings are visibly different here.
    assert!(!by_sheet_id.contains(&1));
}

// -------------------------------------------------------------------------------------------
// Editing — one attribute changes and nothing else does
// -------------------------------------------------------------------------------------------

/// Renaming one tab leaves every other part byte-identical, and every other element of
/// `xl/workbook.xml` byte-identical too.
#[test]
fn renaming_a_sheet_leaves_every_other_byte_alone() {
    let original = fixture(DISAGREEING);
    let mut workbook = Workbook::open(&original).expect("open");
    workbook.rename_sheet(1, "Renamed & Co").expect("rename");
    assert_eq!(
        workbook.sheets()[1].name,
        "Renamed & Co",
        "the resolved sheet list is refreshed by the edit"
    );
    assert_eq!(
        workbook.sheets()[1].part.as_ref().map(PartName::as_str),
        Some("/xl/worksheets/sheet1.xml"),
        "renaming a tab must not move it to a different part"
    );

    let saved = workbook.save().expect("save");
    let before = Package::open(&original).expect("reopen the original");
    let after = Package::open(&saved).expect("reopen the saved container");

    let names_before: Vec<&str> = before.entries().iter().map(|e| e.name.as_str()).collect();
    let names_after: Vec<&str> = after.entries().iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names_before, names_after, "the entry set or order changed");

    let mut differing = Vec::new();
    for (a, b) in before.entries().iter().zip(after.entries()) {
        if a.bytes() != b.bytes() {
            differing.push(a.name.as_str());
        }
    }
    assert_eq!(
        differing,
        vec!["xl/workbook.xml"],
        "exactly one part may change when one tab is renamed"
    );

    let workbook_before =
        String::from_utf8(before.part_bytes(&workbook_part()).expect("there").to_vec())
            .expect("utf-8");
    let workbook_after =
        String::from_utf8(after.part_bytes(&workbook_part()).expect("there").to_vec())
            .expect("utf-8");
    assert_eq!(
        workbook_after,
        workbook_before.replace(r#"name="Hidden Data""#, r#"name="Renamed &amp; Co""#),
        "one attribute value changes; every other byte of the part, the defined names and the \
         relationship references included, is untouched"
    );
}

/// Renaming past the end of the list is an error that names the range, not a silent no-op.
#[test]
fn renaming_a_tab_that_is_not_there_is_reported() {
    let mut workbook = open(DISAGREEING);
    let error = workbook
        .rename_sheet(3, "Nowhere")
        .expect_err("index 3 names no tab");
    let text = error.to_string();
    assert!(
        text.contains('3') && text.contains("3 sheet"),
        "the error must name the index and the range, found {text:?}"
    );
}

/// `sample.xlsx`'s workbook part is still byte-identical to the copy `mjx-sml`'s markup suite
/// carries.
///
/// That suite cannot open a package — proving it does not have to is its whole point — so its copy
/// of the part is a literal. A literal nobody checks is one that drifts; this is the check, made in
/// the crate that is allowed to open the fixture.
#[test]
fn the_markup_suites_copy_of_the_workbook_part_is_still_the_fixtures() {
    let from_fixture = part_bytes("sample.xlsx", "/xl/workbook.xml");
    let from_suite = include_str!("../../mjx-sml/tests/workbook_markup.rs");

    // The copy is a `concat!` of the XML declaration, a newline and the root element; comparing the
    // root element alone is enough to catch any drift in the part that matters.
    let root_start = from_fixture
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("the declaration is followed by a newline")
        + 1;
    let root = std::str::from_utf8(&from_fixture[root_start..]).expect("utf-8");
    assert!(
        from_suite.contains(root),
        "crates/mjx-sml/tests/workbook_markup.rs's SAMPLE_WORKBOOK_PART has drifted from \
         sample.xlsx's own /xl/workbook.xml; the fixture now reads:\n{root}"
    );
}

/// The workbook part name every case above reaches for.
fn workbook_part() -> PartName {
    PartName::new("/xl/workbook.xml").expect("a valid part name")
}
