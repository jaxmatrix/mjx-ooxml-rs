//! **MJXOFF-115 at the package tier.** The formula gate driven through [`Workbook`] rather than
//! through the markup model: an edit made the way a caller makes one, a container saved and
//! reopened, and the parts compared byte for byte.
//!
//! # Why this tier is not the markup tier under another name
//!
//! `crates/mjx-sml/tests/formulas.rs` proves that a worksheet's formulas survive an edit *inside one
//! part*. Two things only this tier can say:
//!
//! 1. **`xl/calcChain.xml` is not touched.** The chain is a different part, and a library that
//!    decided to be helpful about it would edit or drop it on save — which no assertion inside
//!    `mjx-sml` can see, because `mjx-sml` has never heard of a package.
//!    [`no_calculation_chain_maintenance_is_attempted`] pins both its bytes *and* its provenance, so
//!    a chain that was rewritten to the same bytes would still fail.
//! 2. **Reading the chain does not dirty it.** The real hazard in a copy-on-write design is a read
//!    that promotes a part to `Edited`, after which its bytes come from this project's writer rather
//!    than from the container.

use mjx_fixtures::fixture;
use mjx_opc::{Package, PartName, PartProvenance};
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

const FIXTURE: &str = "formulas.xlsx";
const SHEET: &str = "/xl/worksheets/sheet1.xml";
const CHAIN: &str = "/xl/calcChain.xml";

fn part_name(part: &str) -> PartName {
    PartName::new(part).expect("a part name")
}

fn reference(text: &str) -> CellReference {
    CellReference::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

/// The `<f>` and `<v>` of one cell of the saved container's worksheet, as the exact bytes the part
/// holds — rendered as strings so a failing comparison prints the markup rather than byte values.
fn formula_and_cache(container: &[u8], cell: &str) -> (String, Option<String>) {
    let package = Package::open(container).expect("the container opens");
    let bytes = package
        .part_bytes(&part_name(SHEET))
        .expect("the worksheet part is there");
    let part = mjx_sml::WorksheetPart::read_part(bytes)
        .expect("the worksheet reads")
        .expect("the root is an x:worksheet");
    let cell = part
        .cell(reference(cell))
        .unwrap_or_else(|| panic!("{cell} is populated"));
    (
        String::from_utf8_lossy(cell.formula_markup().expect("the cell carries a formula"))
            .into_owned(),
        cell.raw_value()
            .map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
    )
}

/// **The survival gate, through the package.** `A2` is what `B2`'s `A2*2` and `C2`'s `SUM(A2:A6)`
/// are computed from; the ticket is explicit that an unrelated edit would test nothing.
///
/// After the edit every formula and every cached value in the workbook is byte-identical to what was
/// read. The cached values are now **stale**, and that is this library's contract: Excel recalculates
/// on open when it needs to, and a "helpful" blanking of `<v>` would destroy data in a file the
/// caller opened to change one number.
#[test]
fn a_dependency_edit_leaves_every_formula_and_cached_value_byte_identical() {
    let original = fixture(FIXTURE);
    let dependents = ["B2", "C2", "D2", "E2", "F5", "G6", "B3", "B6"];
    let before: Vec<(String, Option<String>)> = dependents
        .iter()
        .map(|cell| formula_and_cache(&original, cell))
        .collect();

    let mut workbook = Workbook::open(&original).expect("the workbook opens");
    workbook
        .set_cell_value(0, reference("A2"), CellValue::Number(11.0))
        .expect("A2 takes a number");
    // A second dependency edit, in the row a *shared-group member* lives in — `A3` is inside `C2`'s
    // `SUM(A2:A6)`, and row 3 carries `B3`. Editing it forces that row to be rebuilt with the member
    // in it, which is the only way a writer that expanded a group into per-cell text would show up
    // at this tier: rows nobody touched are copied whole and never reach the cell writer at all.
    workbook
        .set_cell_value(0, reference("A3"), CellValue::Number(12.0))
        .expect("A3 takes a number");
    let saved = workbook.save().expect("the workbook saves");

    // The edit really landed.
    let reopened = Workbook::open(&saved).expect("the saved workbook opens");
    assert_eq!(
        reopened
            .cell_text(0, reference("A2"))
            .expect("the cell reads")
            .as_deref(),
        Some("11"),
        "the dependency really changed"
    );
    assert_eq!(
        reopened
            .cell_text(0, reference("A3"))
            .expect("the cell reads")
            .as_deref(),
        Some("12"),
        "and so did the second one, in the row the shared-group member lives in"
    );

    for (index, cell) in dependents.iter().enumerate() {
        let after = formula_and_cache(&saved, cell);
        assert_eq!(
            after.0, before[index].0,
            "{cell}'s formula changed across an edit to a cell it references"
        );
        assert_eq!(
            after.1, before[index].1,
            "{cell}'s cached value changed across an edit to a cell it references — a stale cache \
             is correct behaviour here, and blanking it destroys data"
        );
    }
}

/// **The `calcChain` policy, asserted.** The chain is derived data Excel owns: this library leaves
/// it exactly as it found it, neither maintaining it nor dropping it.
///
/// Both halves are pinned, because either alone is weak: identical bytes would not catch a chain
/// this library rewrote to the same content, and an unchanged provenance would not catch one it
/// replaced with different bytes.
#[test]
fn no_calculation_chain_maintenance_is_attempted() {
    let original = fixture(FIXTURE);
    let before = Package::open(&original).expect("the container opens");
    let chain_before = before
        .part_bytes(&part_name(CHAIN))
        .expect("the fixture has a calcChain")
        .to_vec();

    let mut workbook = Workbook::open(&original).expect("the workbook opens");
    workbook
        .set_cell_value(0, reference("A2"), CellValue::Number(11.0))
        .expect("A2 takes a number");
    // A second edit, this time on a cell that carries a formula of its own — the case where a
    // library is most tempted to "fix" the chain.
    workbook
        .set_cell_value(0, reference("B3"), CellValue::Number(99.0))
        .expect("B3 takes a number");

    // Before saving: the chain part is still the container's, untouched.
    let entry = workbook
        .package()
        .entries()
        .iter()
        .find(|entry| entry.name == "xl/calcChain.xml")
        .expect("the chain entry is in the container");
    assert_eq!(
        entry.provenance(),
        PartProvenance::FromContainer,
        "editing formula cells must not touch the calculation chain"
    );

    let saved = workbook.save().expect("the workbook saves");
    let after = Package::open(&saved).expect("the saved container opens");
    assert_eq!(
        after.part_bytes(&part_name(CHAIN)),
        Some(chain_before.as_slice()),
        "the calculation chain must come back byte-identical"
    );
    assert!(
        after
            .part_names()
            .any(|name| name.as_str() == CHAIN),
        "the calculation chain must not be dropped either — that is an edit the caller did not ask \
         for, on every save"
    );

    // And the edits themselves did land, so this was not green because nothing happened.
    let reopened = Workbook::open(&saved).expect("the saved workbook opens");
    assert_eq!(
        reopened
            .cell_text(0, reference("B3"))
            .expect("reads")
            .as_deref(),
        Some("99")
    );
}

/// The chain read through the workbook: eleven entries, in the order the file wrote them.
#[test]
fn the_calculation_chain_is_read_through_the_workbook() {
    let mut workbook = Workbook::open(&fixture(FIXTURE)).expect("the workbook opens");
    let cells = workbook
        .calculation_chain(|chain, interner| {
            chain
                .cells()
                .map(|cell| {
                    cell.reference(interner)
                        .expect("an ST_CellRef")
                        .map(|reference| reference.text().as_str().to_owned())
                })
                .collect::<Vec<_>>()
        })
        .expect("the chain reads")
        .expect("the fixture has a calcChain");

    assert_eq!(
        cells,
        ["B2", "B3", "B4", "B5", "B6", "C3", "C2", "D2", "E2", "G6", "F5"]
            .map(|cell| Some(cell.to_owned()))
            .to_vec(),
        "the chain's order is its content, and it is reported as written"
    );
}

/// A workbook with no `calcChain.xml` answers `None` rather than erroring.
#[test]
fn a_workbook_without_a_chain_answers_none() {
    let mut workbook = Workbook::open(&fixture("sample.xlsx")).expect("the workbook opens");
    assert!(workbook
        .calculation_chain(|_, _| ())
        .expect("the read succeeds")
        .is_none());
}

/// Reading the chain is not a mutation: the part keeps its container bytes and the whole container
/// still round-trips.
#[test]
fn reading_the_calculation_chain_never_dirties_a_part() {
    let original = fixture(FIXTURE);
    let mut workbook = Workbook::open(&original).expect("the workbook opens");
    let count = workbook
        .calculation_chain(|chain, _| chain.len())
        .expect("the chain reads")
        .expect("the fixture has a calcChain");
    assert_eq!(count, 11);

    let saved = workbook.save().expect("the workbook saves");
    let before = Package::open(&original).expect("open");
    let after = Package::open(&saved).expect("reopen");
    for (a, b) in before.entries().iter().zip(after.entries()) {
        assert_eq!(a.name, b.name);
        assert_eq!(
            a.bytes(),
            b.bytes(),
            "{} was rewritten after nothing but a read",
            a.name
        );
    }
    assert_eq!(
        after
            .entries()
            .iter()
            .find(|entry| entry.name == "xl/calcChain.xml")
            .expect("the chain entry")
            .provenance(),
        PartProvenance::FromContainer
    );
}
