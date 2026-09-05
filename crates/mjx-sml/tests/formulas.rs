//! **MJXOFF-115's markup gate.** Formulas as text: read, reported, and — above all — left alone.
//!
//! # What this file is written against
//!
//! The most damaging thing this library could do is be *helpful*. An edit that changes a cell a
//! formula depends on leaves that formula's cached value stale, and that is correct behaviour here:
//! a "helpful" invalidation destroys data in a file the caller opened to change a label, silently,
//! in a part they never named. So the assertions below are mostly about things **not** happening.
//!
//! # The fixture, and why it is authored the way it is
//!
//! Six Phase A children in a row shipped a test that could not fail, and twice the cause was a
//! fixture written in the order the writer emits. This child's exposure is different but as sharp: a
//! store that decomposed `<f>` into fields and regenerated it on write would produce the same
//! *meaning* and different *bytes*, and a fixture written the way that writer emits would never say
//! so. `tests/fixtures/formulas.xlsx` is authored against exactly that:
//!
//! * **A shared group of five** — host `B2` carrying the text, `@ref` and `@si`, and four members
//!   carrying `@si` and **no text at all**. Written four different ways on purpose: `B3` and `B4`
//!   plainly, `B5` with **two spaces and `si` before `t`**, `B6` as `<f t="shared" si="0"></f>`
//!   rather than self-closing. No regenerating writer reproduces any of the last two.
//! * **`C2` writes no `t` and `C3` writes `t='normal'`, single-quoted.** The schema declares
//!   `t` with `default="normal"`, so the two mean the same thing and must not come back the same
//!   way. A model that collapsed absent into `Normal` passes every semantic assertion and fails
//!   [`an_absent_t_and_a_written_normal_stay_different_bytes`].
//! * **An array formula** over `D2:D4` with `@aca`, whose other two cells carry a `<v>` and **no
//!   `<f>` at all** — which is how an array formula is written and a trap for a model that expects
//!   one formula per covered cell.
//! * **A data-table formula** at `F5` with all six of its attributes, including the `r1` input cell.
//! * **Formula text carrying `&lt;`, `&amp;` and `&quot;`**, beside a `t="str"` cached value that
//!   carries `&amp;` too. Minimal re-escaping would turn `&quot;` into `"` — same string, different
//!   bytes.
//! * **`B2` and `C2` really depend on `A2`**: `A2*2` and `SUM(A2:A6)`. The survival gate edits `A2`,
//!   because an unrelated edit tests nothing — and `A2` is in the *same row* as four formula cells,
//!   so the edit forces that row to be rebuilt and the formulas have to survive a rebuild rather
//!   than a copy.
//!
//! # No `mjx_opc` in the models
//!
//! The suite reaches a package only to get a part's bytes. Every assertion after that is against
//! [`WorksheetPart`] and [`CalculationChain`], neither of which has heard of a `PartName`.

use mjx_ooxml_core::ToXml;
use mjx_ooxml_types::spreadsheetml::CellType;
use mjx_opc::{Package, PartName};
use mjx_sml::{
    CalculationChain, CellFormula, CellReference, CellValue, FormulaKind, WorksheetPart,
};

/// The fixture this child is written against.
const FIXTURE: &str = "formulas.xlsx";

/// One part of one committed fixture.
fn part_bytes(fixture: &str, part: &str) -> Vec<u8> {
    let bytes = mjx_fixtures::fixture(fixture);
    let package = Package::open(&bytes).expect("a committed fixture opens");
    let name = PartName::new(part).expect("a part name");
    package
        .part_bytes(&name)
        .unwrap_or_else(|| panic!("{fixture} has no {part}"))
        .to_vec()
}

fn sheet_bytes() -> Vec<u8> {
    part_bytes(FIXTURE, "/xl/worksheets/sheet1.xml")
}

fn sheet() -> WorksheetPart {
    WorksheetPart::read_part(&sheet_bytes())
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet")
}

fn reference(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

/// The `<f>` of one cell, as bytes.
fn formula_markup(part: &WorksheetPart, cell: &str) -> Vec<u8> {
    part.cell(reference(cell))
        .unwrap_or_else(|| panic!("{cell} is populated"))
        .formula_markup()
        .unwrap_or_else(|| panic!("{cell} carries a formula"))
        .to_vec()
}

/// The `<v>` text of one cell, as bytes, or `None` when it has none.
fn cached_bytes(part: &WorksheetPart, cell: &str) -> Option<Vec<u8>> {
    part.cell(reference(cell))?.raw_value().map(<[u8]>::to_vec)
}

/// The fixture must actually be in the corpus, or every byte-identity suite below it is vacuous.
#[test]
fn the_fixture_is_in_the_directory_derived_corpus() {
    assert!(
        mjx_fixtures::package_fixtures_with_extension("xlsx").contains(&FIXTURE.to_owned()),
        "{FIXTURE} is not in the committed corpus, so the three byte-identity tiers never see it"
    );
}

/// The markup tier's round trip: read the whole part, write it straight back, get the file's bytes.
///
/// This is the tier the other two cannot stand in for. `mjx-opc` re-emits a stored part without
/// looking inside it and `Workbook::save` does the same, so both are green for a worksheet this
/// crate never parsed. Here the part is parsed into the model, every formula with it, and the bytes
/// come out of the model.
#[test]
fn every_formula_survives_the_model_round_trip_byte_for_byte() {
    let original = sheet_bytes();
    let rebuilt = sheet().to_markup();
    assert_eq!(
        rebuilt, original,
        "the worksheet did not come back byte-identical through the markup model"
    );
}

/// The negative for the assertion above: it is shown to fail when one byte of one formula changes,
/// so a green run means the formulas really were compared.
#[test]
fn one_changed_byte_in_a_formula_is_caught() {
    let mut mutated = sheet_bytes();
    let at = mutated
        .windows(4)
        .position(|window| window == b"A2*2")
        .expect("the shared host's text is in the fixture");
    mutated[at + 3] = b'3';
    let rebuilt = WorksheetPart::read_part(&mutated)
        .expect("the mutated worksheet still parses")
        .expect("the root is an x:worksheet")
        .to_markup();
    assert_eq!(rebuilt, mutated, "the mutated part still round-trips");
    assert_ne!(
        rebuilt,
        sheet_bytes(),
        "a one-byte change inside a formula must make the comparison fail"
    );
}

/// All four `ST_CellFormulaType` values, read as what they are.
#[test]
fn each_of_the_four_kinds_is_read_as_the_kind_it_is() {
    let part = sheet();
    let kind = |cell: &str| {
        CellFormula::parse(&formula_markup(&part, cell))
            .expect("an f element")
            .kind()
            .expect("a declared kind")
    };
    assert_eq!(kind("C2"), FormulaKind::Normal, "no `t` at all");
    assert_eq!(kind("C3"), FormulaKind::Normal, "`t='normal'`, written out");
    assert_eq!(kind("B2"), FormulaKind::Shared);
    assert_eq!(kind("B3"), FormulaKind::Shared);
    assert_eq!(kind("D2"), FormulaKind::Array);
    assert_eq!(kind("F5"), FormulaKind::DataTable);
}

/// **The distinction the round trip turns on.** `t` absent and `t="normal"` are the same meaning and
/// different bytes; a model that collapsed the first into the second would pass every assertion in
/// the test above and fail here.
#[test]
fn an_absent_t_and_a_written_normal_stay_different_bytes() {
    let part = sheet();
    let absent_markup = formula_markup(&part, "C2");
    let written_markup = formula_markup(&part, "C3");
    let absent = CellFormula::parse(&absent_markup).expect("an f element");
    let written = CellFormula::parse(&written_markup).expect("an f element");

    assert!(!absent.has_written_kind());
    assert_eq!(absent.written_kind(), Ok(None));
    assert!(written.has_written_kind());
    assert_eq!(written.written_kind(), Ok(Some(FormulaKind::Normal)));
    assert_eq!(absent.kind(), written.kind(), "the same meaning");

    // …and the bytes are what a round trip reproduces, single quotes included.
    assert_eq!(absent.markup(), b"<f>SUM(A2:A6)</f>");
    assert_eq!(written.markup(), b"<f t='normal' ca=\"1\">A2+A3</f>");
    assert_eq!(written.needs_recalculation(), Ok(true));
    assert_eq!(absent.needs_recalculation(), Ok(false));
}

/// The array and data-table formulas, and the cells an array formula covers without carrying one.
#[test]
fn the_array_and_data_table_formulas_report_their_own_attributes() {
    let part = sheet();

    let array_markup = formula_markup(&part, "D2");
    let array = CellFormula::parse(&array_markup).expect("an f element");
    assert_eq!(array.kind(), Ok(FormulaKind::Array));
    assert_eq!(array.always_calculate_array(), Ok(true));
    assert_eq!(
        array
            .range()
            .expect("an ST_Ref")
            .map(|range| range.text().as_str().to_owned()),
        Some("D2:D4".to_owned())
    );
    assert_eq!(array.text().expect("decodes"), "TRANSPOSE(A2:A4)");

    // The other two cells of the spill range carry a value and no formula at all. A model that
    // expected one `<f>` per covered cell would trip here rather than in a user's file.
    for covered in ["D3", "D4"] {
        let cell = part.cell(reference(covered)).expect("populated");
        assert!(!cell.has_formula(), "{covered} carries no <f>");
        assert!(
            cell.raw_value().is_some(),
            "{covered} carries a cached value"
        );
        assert!(
            cell.cached_value().is_none(),
            "a value with no formula is not a cache"
        );
    }

    let table_markup = formula_markup(&part, "F5");
    let table = CellFormula::parse(&table_markup).expect("an f element");
    assert_eq!(table.kind(), Ok(FormulaKind::DataTable));
    assert_eq!(table.is_two_dimensional_data_table(), Ok(false));
    assert_eq!(table.is_row_oriented_data_table(), Ok(false));
    assert_eq!(table.first_input_cell_deleted(), Ok(false));
    assert_eq!(table.second_input_cell_deleted(), Ok(false));
    assert_eq!(table.first_input_cell(), Ok(Some(reference("A2"))));
    assert_eq!(table.second_input_cell(), Ok(None));
    assert!(
        !table.has_text(),
        "a data table's formula is the implied TABLE()"
    );
}

/// Formula text is untrusted, and the escaped bytes are what round-trips.
#[test]
fn escaped_formula_text_is_decoded_for_reading_and_kept_for_writing() {
    let part = sheet();
    let markup = formula_markup(&part, "E2");
    let formula = CellFormula::parse(&markup).expect("an f element");
    assert_eq!(
        formula.text().expect("decodes"),
        r#"IF(A2<3,"low & slow","high")"#
    );
    assert_eq!(
        formula.raw_text(),
        br#"IF(A2&lt;3,&quot;low &amp; slow&quot;,&quot;high&quot;)"#,
        "minimal re-escaping would write `\"` for `&quot;` — the same string and different bytes"
    );

    let cached = part
        .cell(reference("E2"))
        .expect("populated")
        .cached_value()
        .expect("E2 has a formula and a value");
    assert_eq!(cached.cell_type(), CellType::FormulaString);
    assert_eq!(
        cached.as_formula_string().expect("decodes").as_deref(),
        Some("low & slow")
    );
    assert_eq!(cached.raw_text(), b"low &amp; slow");
}

/// Cached values are read through the `c@t` beside them, and only where there is a formula.
#[test]
fn a_cached_value_is_read_through_the_cell_type_and_only_beside_a_formula() {
    let part = sheet();
    let cached = |cell: &str| {
        part.cell(reference(cell))
            .unwrap_or_else(|| panic!("{cell} is populated"))
            .cached_value()
    };

    let number = cached("B2").expect("B2 caches a number");
    assert_eq!(number.cell_type(), CellType::Number);
    assert!(!number.has_written_cell_type(), "`n` is the schema default");
    assert_eq!(number.as_number(), Some(2.0));

    let boolean = cached("G6").expect("G6 caches a boolean");
    assert_eq!(boolean.cell_type(), CellType::Boolean);
    assert!(boolean.has_written_cell_type());
    assert_eq!(boolean.as_boolean(), Some(true));
    assert_eq!(
        boolean.as_number(),
        None,
        "the same `1` is not the number one"
    );

    // A2 holds a value and no formula, so it has no cache — the distinction this type exists for.
    assert!(part
        .cell(reference("A2"))
        .expect("populated")
        .raw_value()
        .is_some());
    assert!(cached("A2").is_none());
}

/// `@bx` — the attribute whose meaning nothing about the token gives away.
#[test]
fn the_two_rarest_flags_are_read_from_the_bytes_that_carry_them() {
    let part = sheet();
    let markup = formula_markup(&part, "G6");
    let formula = CellFormula::parse(&markup).expect("an f element");
    assert_eq!(formula.assigns_value_to_name(), Ok(true), "@bx");
    assert_eq!(formula.needs_recalculation(), Ok(false), "@ca is absent");
    assert!(!formula.has_written_kind(), "and so is @t");
}

/// The shared group: five cells, one host, four members with no text.
#[test]
fn the_shared_group_has_one_host_and_four_text_less_members() {
    let part = sheet();
    let data = part.sheet_data().expect("the sheet has a sheetData");
    let groups = data.shared_formula_groups().expect("the groups index");
    assert_eq!(groups.len(), 1, "one @si group");

    let group = groups.get(0).expect("group 0");
    assert_eq!(group.index(), 0);
    assert_eq!(group.cell_count(), 5, "host plus four members");
    assert_eq!(group.host_count(), 1, "exactly one master cell");
    assert_eq!(group.host(), Some(reference("B2")));
    assert_eq!(
        group.range().map(|range| range.text().as_str().to_owned()),
        Some("B2:B6".to_owned())
    );
    let host = group.host_formula().expect("the host carries the text");
    assert_eq!(host.raw_text(), b"A2*2");

    assert_text_distribution(&part);
}

/// **The distribution gate.** The host carries the text and every member carries none — before an
/// edit and after one, because expanding a group into per-cell text on write is a corruption and not
/// an optimisation.
fn assert_text_distribution(part: &WorksheetPart) {
    let host_markup = formula_markup(part, "B2");
    let host = CellFormula::parse(&host_markup).expect("an f element");
    assert!(host.has_text(), "the host carries the text");
    assert_eq!(host.raw_text(), b"A2*2");
    assert_eq!(host.raw_attribute("ref"), Some(&b"B2:B6"[..]));

    for member in ["B3", "B4", "B5", "B6"] {
        let markup = formula_markup(part, member);
        let formula = CellFormula::parse(&markup).expect("an f element");
        assert_eq!(
            formula.is_shared_group_member(),
            Ok(true),
            "{member} is a member"
        );
        assert!(
            !formula.has_text(),
            "{member} must carry no formula text at all, and carries {:?}",
            String::from_utf8_lossy(formula.raw_text())
        );
        assert_eq!(formula.raw_attribute("ref"), None, "{member} has no @ref");
        assert_eq!(formula.shared_group_index(), Ok(Some(0)));
    }
}

/// **The distribution gate, under an edit.** Changing one member's *style* rewrites that cell's start
/// tag and its row; the group's text distribution must be exactly what it was.
#[test]
fn editing_one_members_style_leaves_the_groups_text_distribution_unchanged() {
    let mut part = sheet();
    let before = formula_markup(&part, "B4");
    part.sheet_data_mut()
        .expect("the sheet has a sheetData")
        .set_cell_style(reference("B4"), Some(1))
        .expect("the style is set");

    // The edit really happened: the cell's start tag now carries `s="1"`, and the part's bytes moved.
    let edited = part.to_markup();
    assert_ne!(edited, sheet_bytes(), "the style edit must change the part");
    assert!(
        edited
            .windows(23)
            .any(|w| w == br#"<c r="B4" s="1"><f t="s"#),
        "B4 gained its style attribute in place"
    );

    // …and the formula came through the rebuild untouched, text distribution and all.
    let reread = WorksheetPart::read_part(&edited)
        .expect("the edited part parses")
        .expect("the root is an x:worksheet");
    assert_eq!(formula_markup(&reread, "B4"), before);
    assert_text_distribution(&reread);

    let groups = reread
        .sheet_data()
        .expect("a sheetData")
        .shared_formula_groups()
        .expect("the groups index");
    let group = groups.get(0).expect("group 0");
    assert_eq!(group.cell_count(), 5);
    assert_eq!(group.host(), Some(reference("B2")));
}

/// **The survival gate.** `A2` is what `B2`'s `A2*2` and `C2`'s `SUM(A2:A6)` are computed from.
/// Changing it leaves both formulas *and both cached values* exactly as they were — the cached values
/// are now stale, and that is this library's contract rather than a defect.
#[test]
fn a_formula_and_its_cached_value_survive_an_edit_to_a_cell_they_reference() {
    let part = sheet();
    let dependents = ["B2", "C2", "D2", "E2"];
    let formulas: Vec<Vec<u8>> = dependents
        .iter()
        .map(|cell| formula_markup(&part, cell))
        .collect();
    let cached: Vec<Option<Vec<u8>>> = dependents
        .iter()
        .map(|cell| cached_bytes(&part, cell))
        .collect();

    let mut edited = sheet();
    edited
        .set_cell_value(reference("A2"), CellValue::Number(11.0))
        .expect("A2 takes a number");
    let bytes = edited.to_markup();

    // The edit happened, and it happened in the row the formulas live in — so those cells were
    // re-emitted through the writer rather than copied with an untouched row.
    assert_ne!(bytes, sheet_bytes());
    let edited_cell = br#"<c r="A2"><v>11</v></c>"#;
    assert!(
        bytes
            .windows(edited_cell.len())
            .any(|window| window == edited_cell),
        "A2 was rewritten in place, in the row four formula cells live in"
    );

    let reread = WorksheetPart::read_part(&bytes)
        .expect("the edited part parses")
        .expect("the root is an x:worksheet");
    assert_eq!(
        reread.cell(reference("A2")).expect("populated").number(),
        Some(11.0),
        "the dependency really changed"
    );

    for (index, cell) in dependents.iter().enumerate() {
        assert_eq!(
            formula_markup(&reread, cell),
            formulas[index],
            "{cell}'s formula must be byte-identical after an edit to a cell it references"
        );
        assert_eq!(
            cached_bytes(&reread, cell),
            cached[index],
            "{cell}'s cached value must be byte-identical — a stale cache is correct here, and \
             blanking it would destroy data in a file the caller opened to change one number"
        );
    }

    // The rest of the sheet is untouched, formulas in other rows included.
    assert_eq!(formula_markup(&reread, "B6"), formula_markup(&part, "B6"));
    assert_eq!(formula_markup(&reread, "F5"), formula_markup(&part, "F5"));
    assert_text_distribution(&reread);
}

/// The same gate for an edit in a *different* row from the formula: `A4` is inside `D2`'s
/// `TRANSPOSE(A2:A4)` and inside `C2`'s `SUM(A2:A6)`.
#[test]
fn a_dependency_edit_in_another_row_changes_nothing_either() {
    let part = sheet();
    let before: Vec<Vec<u8>> = ["C2", "D2", "B2"]
        .iter()
        .map(|cell| formula_markup(&part, cell))
        .collect();

    let mut edited = sheet();
    edited
        .set_cell_value(reference("A4"), CellValue::Number(30.0))
        .expect("A4 takes a number");
    let bytes = edited.to_markup();
    let reread = WorksheetPart::read_part(&bytes)
        .expect("the edited part parses")
        .expect("the root is an x:worksheet");

    for (index, cell) in ["C2", "D2", "B2"].iter().enumerate() {
        assert_eq!(formula_markup(&reread, cell), before[index], "{cell}");
        assert_eq!(
            cached_bytes(&reread, cell),
            cached_bytes(&part, cell),
            "{cell}"
        );
    }
    // Row 2 was never touched, so it is still the file's own bytes.
    let row = reread
        .sheet_data()
        .expect("a sheetData")
        .row(2)
        .expect("row 2");
    assert!(
        row.is_verbatim(),
        "a row nobody edited must still be written straight out of the part's buffer"
    );
}

/// `calcChain.xml`: modelled, reported, and written back exactly as read.
#[test]
fn the_calculation_chain_reads_and_round_trips() {
    let bytes = part_bytes(FIXTURE, "/xl/calcChain.xml");
    let mut document = mjx_xml::fidelity::parse(&bytes).expect("the calcChain parses");
    let chain = CalculationChain::read_part(&document)
        .expect("the part reads")
        .expect("the root is an x:calcChain");

    assert_eq!(chain.len(), 11);
    assert!(!chain.is_empty());
    let first = chain.cells().next().expect("an entry");
    assert_eq!(
        first.reference(&document.interner).expect("an ST_CellRef"),
        Some(reference("B2"))
    );
    assert_eq!(first.sheet_id(&document.interner).expect("an int"), Some(1));

    // §18.6.1's two carry-forward rules: `@i` and `@s` take the previous entry's value when absent.
    let resolved = chain
        .resolved(&document.interner)
        .expect("the chain resolves");
    assert_eq!(resolved.len(), 11);
    for (index, entry) in resolved.iter().enumerate() {
        assert_eq!(
            entry.sheet_id,
            Some(1),
            "entry {index} inherits @i from the first"
        );
    }
    assert!(!resolved[0].is_on_child_chain, "B2 starts no child chain");
    assert!(resolved[1].is_on_child_chain, "B3 says s=\"1\"");
    assert!(
        resolved[5].is_on_child_chain,
        "C3 writes no @s, so §18.6.1 carries the previous entry's forward"
    );

    // The attribute `CT_CalcCell` declares and §18.6.1's prose never describes.
    let with_range = chain
        .cells()
        .find(|cell| {
            cell.range(&document.interner)
                .expect("an ST_CellRef")
                .is_some()
        })
        .expect("one entry carries the undocumented @ref");
    assert_eq!(
        with_range
            .reference(&document.interner)
            .expect("an ST_CellRef"),
        Some(reference("F5"))
    );
    assert_eq!(with_range.is_array_formula(&document.interner), Ok(true));
    assert_eq!(
        with_range
            .range(&document.interner)
            .expect("an ST_CellRef")
            .map(|range| range.text().as_str().to_owned()),
        Some("F5:G7".to_owned()),
        "`ST_CellRef` is an unrestricted xsd:string, so a range is exactly as valid there as a cell"
    );

    // …and the whole part comes back byte-identical through the model.
    chain.write_back(&mut document.root, &mut document.interner);
    assert_eq!(mjx_xml::fidelity::serialize_to_vec(&document), bytes);
}

/// A part that is not a calculation chain is a question, not an error.
#[test]
fn a_part_that_is_not_a_calc_chain_reads_as_none() {
    let bytes = part_bytes(FIXTURE, "/xl/workbook.xml");
    let document = mjx_xml::fidelity::parse(&bytes).expect("the workbook parses");
    assert!(CalculationChain::read_part(&document)
        .expect("the read succeeds")
        .is_none());
}
