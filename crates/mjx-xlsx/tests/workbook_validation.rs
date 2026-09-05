//! What [`Workbook::save`] refuses, proved on the real fixture rather than on a synthetic package.
//!
//! `crates/mjx-xlsx/src/validate.rs`'s own unit tests build each broken graph from `Package::empty`,
//! which is the right shape for stating one invariant at a time. This file does the other half:
//! it takes `tests/fixtures/sample.xlsx`, breaks one edge of it, and shows the *public* entry point
//! — `Workbook::save`, the method a caller actually holds — refusing to write the result.
//!
//! # The trap this file is written against
//!
//! A validation suite that only ever asserts failures is green when the validator rejects
//! everything, which would be worse than no validator at all. Every case here therefore does the
//! same package twice: **unbroken, it saves**; broken in exactly one place, it does not. The
//! `sample.xlsx` baseline at the top is the same assertion for the fixture as a whole.

use mjx_fixtures::fixture;
use mjx_opc::{Package, PartName, Relationship, TargetMode};
use mjx_xlsx::{Workbook, XlsxError};

fn workbook_part() -> PartName {
    PartName::new("/xl/workbook.xml").expect("a valid part name")
}

/// The baseline every case below is measured against.
#[test]
fn the_untouched_fixture_validates_and_saves() {
    let workbook = Workbook::open(&fixture("sample.xlsx")).expect("open");
    workbook
        .validate()
        .expect("the fixture is a valid workbook");
    workbook.save().expect("and it saves");
}

#[test]
fn dropping_the_shared_strings_relationship_refuses_the_save() {
    // MJXOFF-91's own mutation. `xl/sharedStrings.xml` is reached through one relationship and
    // nothing else — no markup names it by `r:id` — so removing that relationship leaves the part in
    // the container, perfectly well-formed, unreferenced, and invisible to every consumer. Every
    // `t="s"` cell in `xl/worksheets/sheet1.xml` then indexes into a table nothing loads.
    //
    // `mjx-opc` cannot see this: it is explicit that an unreferenced part is legal, merely dead
    // weight. The refusal has to come from the layer that knows what a SpreadsheetML part is for.
    let mut package = Package::open(&fixture("sample.xlsx")).expect("open");

    // Named rather than assumed: the fixture's `sharedStrings` relationship, found by its type.
    let relationship_id = package
        .relationships_for(Some(&workbook_part()))
        .expect("the workbook's own .rels")
        .by_type(mjx_xlsx::parts::REL_SHARED_STRINGS)
        .next()
        .expect("the fixture relates a shared string table")
        .id
        .clone();

    // Before: it saves.
    Workbook::from_package(Package::open(&fixture("sample.xlsx")).expect("open"))
        .expect("open")
        .save()
        .expect("the unbroken package saves");

    package
        .remove_relationship(Some(&workbook_part()), &relationship_id)
        .expect("drop the sharedStrings relationship");

    let workbook = Workbook::from_package(package).expect("the workbook still opens");
    let error = workbook
        .save()
        .expect_err("a shared string table nothing reaches must not be written");
    let text = error.to_string();
    assert!(
        matches!(error, XlsxError::InvalidWorkbook(_)),
        "expected a SpreadsheetML defect, got {error:?}"
    );
    assert!(
        text.contains("/xl/sharedStrings.xml"),
        "the refusal must name the part: {text}"
    );

    // …and `save_unchecked` still writes it, because a caller who means to must be able to.
    let bytes = workbook
        .save_unchecked()
        .expect("the escape hatch is still open");
    let reopened = Package::open(&bytes).expect("reopen");
    assert!(
        reopened
            .part_bytes(&PartName::new("/xl/sharedStrings.xml").expect("a valid part name"))
            .is_some(),
        "the orphaned part is still written, verbatim — refusing to validate is not removing"
    );
}

#[test]
fn retargeting_the_office_document_relationship_refuses_the_save() {
    let mut package = Package::open(&fixture("sample.xlsx")).expect("open");
    let workbook = Workbook::from_package(Package::open(&fixture("sample.xlsx")).expect("open"))
        .expect("open");
    workbook.save().expect("the unbroken package saves");

    let root_relationship_id = package
        .relationships_for(None)
        .expect("the package-root .rels")
        .by_type(mjx_xlsx::parts::REL_OFFICE_DOCUMENT)
        .next()
        .expect("the fixture has an officeDocument relationship")
        .id
        .clone();
    package
        .retarget_relationship(
            None,
            &root_relationship_id,
            "xl/worksheets/sheet1.xml",
            TargetMode::Internal,
        )
        .expect("point the root relationship at a worksheet instead");

    // The workbook can no longer be *opened* through the root relationship either — the part it now
    // names is not rooted in `x:workbook` — which is itself the invariant, met one step earlier.
    let error = Workbook::from_package(package).expect_err("that is not a workbook part");
    assert!(
        matches!(error, XlsxError::MalformedWorkbook(_)),
        "got {error:?}"
    );
}

#[test]
fn a_second_worksheet_the_sheet_list_never_names_refuses_the_save() {
    // The reverse direction of the sheet list, on the real fixture: a worksheet part related from
    // the workbook that `x:sheets` does not list is a tab no consumer will ever show.
    //
    // This one needs the workbook's markup to be in scope, and it is: adding the part *and* leaving
    // `xl/workbook.xml` untouched would put the check out of scope by design (see
    // `crates/mjx-xlsx/src/validate.rs`'s own scope note), so the case dirties the workbook part
    // through the same `part_tree_mut` a later Phase D child will, which is what makes this
    // library's markup this library's to fault.
    let mut package = Package::open(&fixture("sample.xlsx")).expect("open");
    let second = PartName::new("/xl/worksheets/sheet2.xml").expect("a valid part name");
    package
        .insert_part(
            &second,
            mjx_xlsx::parts::CONTENT_TYPE_WORKSHEET,
            br#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>"#.to_vec(),
        )
        .expect("insert a second worksheet");
    package
        .add_relationship(
            Some(&workbook_part()),
            Relationship {
                id: "rId99".to_owned(),
                rel_type: mjx_xlsx::parts::REL_WORKSHEET.to_owned(),
                target: "worksheets/sheet2.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("relate it, but never list it");
    // Dirty `xl/workbook.xml` without changing it, so its markup is in the validator's scope.
    let _ = package
        .part_tree_mut(&workbook_part())
        .expect("read the workbook part for writing");

    let workbook = Workbook::from_package(package).expect("the workbook still opens");
    let error = workbook
        .save()
        .expect_err("a worksheet the list never names must not be written");
    let text = error.to_string();
    assert!(text.contains("rId99"), "{text}");
    assert!(text.contains("/xl/worksheets/sheet2.xml"), "{text}");

    // The discriminating half: list it, and the same package saves.
    let mut package = Package::open(&fixture("sample.xlsx")).expect("open");
    package
        .insert_part(
            &second,
            mjx_xlsx::parts::CONTENT_TYPE_WORKSHEET,
            br#"<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData/></worksheet>"#.to_vec(),
        )
        .expect("insert a second worksheet");
    package
        .add_relationship(
            Some(&workbook_part()),
            Relationship {
                id: "rId99".to_owned(),
                rel_type: mjx_xlsx::parts::REL_WORKSHEET.to_owned(),
                target: "worksheets/sheet2.xml".to_owned(),
                mode: TargetMode::Internal,
            },
        )
        .expect("relate it");
    list_the_second_sheet(&mut package);
    let workbook = Workbook::from_package(package).expect("open");
    assert_eq!(workbook.sheets().len(), 2, "both tabs are read");
    assert_eq!(workbook.sheets()[1].name, "second");
    workbook
        .save()
        .expect("a listed worksheet is exactly what the workbook wanted");
}

/// Appends `<sheet name="second" sheetId="2" r:id="rId99"/>` to the fixture's `x:sheets`.
///
/// Built as a child of the element that was **read** from the part, never as a fresh root written
/// over it — the rule `crates/mjx-xlsx/src/blank.rs` states, applied here because this test is the
/// first thing in the crate to author SpreadsheetML at all.
fn list_the_second_sheet(package: &mut Package) {
    use mjx_ooxml_core::{RawAttribute, RawDocument, RawElement, RawName, RawNode};

    let RawDocument { interner, root, .. } = package
        .part_tree_mut(&workbook_part())
        .expect("edit xl/workbook.xml");
    let sml = interner.intern("http://schemas.openxmlformats.org/spreadsheetml/2006/main");
    let attribute = |interner: &mut mjx_ooxml_core::Interner,
                     prefix: Option<&str>,
                     local,
                     value: &str| RawAttribute {
        name: RawName {
            prefix: prefix.map(|p| interner.intern(p)),
            local: interner.intern(local),
            namespace: None,
        },
        value: value.as_bytes().to_vec().into(),
        quote: mjx_ooxml_core::QuoteStyle::Double,
    };
    let attributes = vec![
        attribute(interner, None, "name", "second"),
        attribute(interner, None, "sheetId", "2"),
        attribute(interner, Some("r"), "id", "rId99"),
    ];
    let entry = RawElement::new(
        RawName {
            prefix: None,
            local: interner.intern("sheet"),
            namespace: Some(sml),
        },
        attributes,
        Vec::new(),
        true,
    );

    let sheets = find_sheets(root, interner).expect("the fixture has an x:sheets");
    sheets.children.push(RawNode::Element(entry));
}

/// The first `x:sheets` element under `element`, depth first.
fn find_sheets<'a>(
    element: &'a mut mjx_ooxml_core::RawElement,
    interner: &mjx_ooxml_core::Interner,
) -> Option<&'a mut mjx_ooxml_core::RawElement> {
    const SML: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";
    let is_sheets = element
        .name
        .namespace
        .is_some_and(|ns| interner.resolve(ns) == SML)
        && interner.resolve(element.name.local) == "sheets";
    if is_sheets {
        return Some(element);
    }
    for child in &mut element.children {
        if let mjx_ooxml_core::RawNode::Element(child) = child {
            if let Some(found) = find_sheets(child, interner) {
                return Some(found);
            }
        }
    }
    None
}
