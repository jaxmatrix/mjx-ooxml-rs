//! MJXOFF-93's first "Done when": **every reference form in `tests/fixtures/sample.xlsx` parses and
//! re-emits byte-identically**, including `dimension ref="A1:C3"` and `selection sqref="A1"`.
//!
//! # Why this walks the raw tree
//!
//! Nothing in `mjx-sml` reads a worksheet yet — `CT_Row` and `CT_Cell` are MJXOFF-95's (D04), the
//! `CT_Worksheet` spine is MJXOFF-102's (D07). So this suite goes to the source the same way
//! `crates/mjx-docx/tests/leaf_attributes.rs` does: `Package::part_tree` gives the parsed part,
//! `Interner::resolve` names each element and attribute, and the table below says which
//! *(element, attribute)* pairs carry an address and which grammar each one is written in. Every one
//! found is parsed with this crate's public API and rendered back, and the rendering must equal the
//! attribute's own bytes.
//!
//! # Why the found set is pinned rather than counted
//!
//! A scan that finds nothing passes a "everything found round-trips" assertion perfectly. So the
//! full list of what the fixture carries — thirteen addresses in the worksheet plus the workbook's
//! `refMode` — is written down below and compared element for element. Removing the scan, or
//! narrowing the table until it matches nothing, fails on the pinned list rather than passing
//! quietly. `sample.xlsx` is LibreOffice-authored, which is why `row@spans` (an Excel-only
//! optimisation hint) has no case here and is covered against authored markup at the bottom of this
//! file instead.

use mjx_ooxml_core::{Interner, RawElement, RawNode};
use mjx_opc::{Package, PartName};
use mjx_sml::{CellRange, CellRangeList, CellReference, CellSpans, ReferenceMode};

/// Which grammar an address-bearing attribute is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddressKind {
    /// `ST_CellRef` — one cell (`c@r`, `selection@activeCell`, `sheetView@topLeftCell`).
    CellReference,
    /// `ST_Ref` — a range (`dimension@ref`, `mergeCell@ref`, `autoFilter@ref`, `table@ref`).
    Range,
    /// `ST_Sqref` — a whitespace-separated range list (`selection@sqref`, and the two on
    /// conditional formats and data validations).
    RangeList,
    /// `ST_CellSpans` — `row@spans`.
    Spans,
}

impl AddressKind {
    /// Parses `value` with the grammar this kind names, and renders it back.
    ///
    /// The returned text is what the library would write; the caller asserts it equals what the
    /// file said.
    fn round_trip(self, value: &str) -> Result<String, mjx_sml::AddressError> {
        Ok(match self {
            Self::CellReference => CellReference::parse(value)?.text().as_str().to_owned(),
            Self::Range => CellRange::parse(value)?.text().as_str().to_owned(),
            Self::RangeList => CellRangeList::parse(value)?.to_string(),
            Self::Spans => CellSpans::parse(value)?.to_string(),
        })
    }
}

/// Every *(element, attribute)* pair in `sml.xsd` that carries an address, and its grammar.
///
/// Scoped by element on purpose: `row@r` is a row *number* and `c@r` is a cell *reference*, and a
/// table keyed on the attribute name alone would feed the first to the second's parser.
const ADDRESS_ATTRIBUTES: &[(&str, &str, AddressKind)] = &[
    ("c", "r", AddressKind::CellReference),
    ("cellWatch", "r", AddressKind::CellReference),
    ("selection", "activeCell", AddressKind::CellReference),
    ("sheetView", "topLeftCell", AddressKind::CellReference),
    ("pane", "topLeftCell", AddressKind::CellReference),
    ("dimension", "ref", AddressKind::Range),
    ("mergeCell", "ref", AddressKind::Range),
    ("hyperlink", "ref", AddressKind::Range),
    ("autoFilter", "ref", AddressKind::Range),
    ("table", "ref", AddressKind::Range),
    ("selection", "sqref", AddressKind::RangeList),
    ("conditionalFormatting", "sqref", AddressKind::RangeList),
    ("dataValidation", "sqref", AddressKind::RangeList),
    ("row", "spans", AddressKind::Spans),
];

/// One address found in a part: the element that carried it, the attribute, and its exact bytes.
type Found = (String, String, String);

/// Collects every address-bearing attribute in `element` and its descendants, in document order.
fn collect(element: &RawElement, interner: &Interner, found: &mut Vec<Found>) {
    let local = interner.resolve(element.name.local);
    for attribute in &element.attributes {
        let attribute_local = interner.resolve(attribute.name.local);
        let carries = ADDRESS_ATTRIBUTES
            .iter()
            .any(|(owner, name, _)| *owner == local && *name == attribute_local);
        if !carries {
            continue;
        }
        let value = std::str::from_utf8(&attribute.value).expect("an address is ASCII");
        assert!(
            !value.contains('&'),
            "{local}@{attribute_local} carries an entity reference; this suite compares raw bytes"
        );
        found.push((
            local.to_owned(),
            attribute_local.to_owned(),
            value.to_owned(),
        ));
    }
    for child in &element.children {
        if let RawNode::Element(child) = child {
            collect(child, interner, found);
        }
    }
}

/// The grammar declared for a found *(element, attribute)* pair.
fn kind_of(element: &str, attribute: &str) -> AddressKind {
    ADDRESS_ATTRIBUTES
        .iter()
        .find(|(owner, name, _)| *owner == element && *name == attribute)
        .map(|(_, _, kind)| *kind)
        .expect("the collector only keeps pairs the table declares")
}

/// Every address `sample.xlsx`'s worksheet carries, in document order.
///
/// Written down rather than counted, so that a scan which stops finding things fails here.
const EXPECTED: &[(&str, &str, &str)] = &[
    ("dimension", "ref", "A1:C3"),
    ("sheetView", "topLeftCell", "A1"),
    ("selection", "activeCell", "A1"),
    ("selection", "sqref", "A1"),
    ("c", "r", "A1"),
    ("c", "r", "B1"),
    ("c", "r", "C1"),
    ("c", "r", "A2"),
    ("c", "r", "B2"),
    ("c", "r", "C2"),
    ("c", "r", "A3"),
    ("c", "r", "B3"),
    ("c", "r", "C3"),
];

#[test]
fn every_address_in_the_fixture_parses_and_re_emits_byte_identically() {
    let bytes = mjx_fixtures::fixture("sample.xlsx");
    let mut package = Package::open(&bytes).expect("sample.xlsx opens");
    let part = PartName::new("/xl/worksheets/sheet1.xml").expect("a valid part name");
    let document = package.part_tree(&part).expect("the worksheet parses");

    let mut found = Vec::new();
    collect(&document.root, &document.interner, &mut found);

    let expected: Vec<Found> = EXPECTED
        .iter()
        .map(|(element, attribute, value)| {
            (
                (*element).to_owned(),
                (*attribute).to_owned(),
                (*value).to_owned(),
            )
        })
        .collect();
    assert_eq!(
        found, expected,
        "the fixture's addresses are pinned: this suite is worthless if the scan finds nothing"
    );

    for (element, attribute, value) in &found {
        let kind = kind_of(element, attribute);
        let written = kind.round_trip(value).unwrap_or_else(|error| {
            panic!("{element}@{attribute}={value:?} did not parse: {error}")
        });
        assert_eq!(
            &written, value,
            "{element}@{attribute} must re-emit byte-identically"
        );
    }
}

#[test]
fn the_workbook_reports_its_reference_mode_without_applying_it() {
    let bytes = mjx_fixtures::fixture("sample.xlsx");
    let mut package = Package::open(&bytes).expect("sample.xlsx opens");
    let part = PartName::new("/xl/workbook.xml").expect("a valid part name");
    let document = package.part_tree(&part).expect("the workbook parses");

    let mut modes = Vec::new();
    fn walk(element: &RawElement, interner: &Interner, modes: &mut Vec<String>) {
        if interner.resolve(element.name.local) == "calcPr" {
            for attribute in &element.attributes {
                if interner.resolve(attribute.name.local) == "refMode" {
                    modes.push(
                        std::str::from_utf8(&attribute.value)
                            .expect("a wire token is ASCII")
                            .to_owned(),
                    );
                }
            }
        }
        for child in &element.children {
            if let RawNode::Element(child) = child {
                walk(child, interner, modes);
            }
        }
    }
    walk(&document.root, &document.interner, &mut modes);

    assert_eq!(modes, ["A1"], "sample.xlsx carries calcPr@refMode=\"A1\"");
    assert_eq!(
        ReferenceMode::from_wire(&modes[0]),
        Some(ReferenceMode::A1),
        "the mode is read from the generated simple type"
    );
}

/// The other half of the `spans` rule — *never drop it when the source did carry it*.
///
/// `sample.xlsx` is LibreOffice-authored and carries no `row@spans` anywhere, so the corpus can only
/// ever exercise "never derive it". The positive half needs a row that has one, which is authored
/// here as `x:`-prefixed markup and read through the same scanner the fixture case uses — the same
/// technique `crates/mjx-docx/tests/wml_child_order.rs` uses to reach markup no fixture carries.
/// MJXOFF-95 (D04) models `CT_Row` and will assert it through a real row; this asserts everything
/// the addressing layer can be held to on its own.
#[test]
fn an_authored_row_keeps_the_spans_it_arrived_with() {
    const MARKUP: &[u8] = br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<x:worksheet xmlns:x="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><x:sheetData><x:row r="1" spans="1:3"><x:c r="A1"/><x:c r="C1"/></x:row><x:row r="2" spans="1:3  5:7"><x:c r="A2"/></x:row><x:row r="3"><x:c r="A3"/></x:row></x:sheetData></x:worksheet>"#;

    let document = mjx_xml::fidelity::parse(MARKUP).expect("the authored worksheet parses");
    let mut found = Vec::new();
    collect(&document.root, &document.interner, &mut found);

    assert_eq!(
        found,
        vec![
            ("row".to_owned(), "spans".to_owned(), "1:3".to_owned()),
            ("c".to_owned(), "r".to_owned(), "A1".to_owned()),
            ("c".to_owned(), "r".to_owned(), "C1".to_owned()),
            ("row".to_owned(), "spans".to_owned(), "1:3  5:7".to_owned()),
            ("c".to_owned(), "r".to_owned(), "A2".to_owned()),
            ("c".to_owned(), "r".to_owned(), "A3".to_owned()),
        ],
        "the third row carries no `spans`, and nothing may invent one for it"
    );

    for (element, attribute, value) in &found {
        let written = kind_of(element, attribute)
            .round_trip(value)
            .unwrap_or_else(|error| {
                panic!("{element}@{attribute}={value:?} did not parse: {error}")
            });
        assert_eq!(
            &written, value,
            "{element}@{attribute} must re-emit byte-identically, odd whitespace and all"
        );
    }
}
