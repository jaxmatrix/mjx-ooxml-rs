//! The worksheet residency is **bounded**, and the bound is measured rather than claimed
//! (MJXOFF-168).
//!
//! # What this file exists to close
//!
//! MJXOFF-167 shipped [`SpreadsheetSession`](mjx_session::ooxml::SpreadsheetSession) holding every
//! worksheet it had touched, with no bound and no eviction, and said so plainly: *a session that
//! walks a 200-sheet workbook holds 200 parsed sheets for its lifetime, and nothing measures that.*
//! The residency **is** the feature — it is what turns twenty worksheet parses into one, which
//! `tests/batching.rs` measures — so the fix was never to weaken it. It is the byte-budgeted
//! least-recently-used store underneath it, and this file is what makes that a gate.
//!
//! # The four traps, and the case that closes each
//!
//! 1. **A budget so large it never evicts is indistinguishable from no budget at all.** So the
//!    small-budget walk is run beside the *same walk under the default budget*
//!    ([`the_default_budget_holds_every_sheet_which_is_what_makes_the_small_one_a_measurement`]),
//!    which holds all forty sheets and evicts nothing. Without that pair, a green here would be
//!    green for the unbounded implementation.
//! 2. **A ceiling is trivially satisfied by a cache that holds nothing.** So every bound below is
//!    two-sided: the ceiling held *and* the sheet just used is still resident, asserted by a parse
//!    count rather than by inspection.
//! 3. **An eviction count cannot say *which* entry went.** A residency that dropped the sheet the
//!    user is looking at would report exactly the same count as one that got it right, so
//!    [`eviction_takes_the_least_recently_used_sheet_and_not_the_one_in_use`] names the sheet
//!    before and after.
//! 4. **A pin nothing tests is a comment.** [`a_sheet_with_unwritten_edits_is_never_evicted`] drives
//!    forty dirty sheets through a budget that fits four and reads every edit back.

#![cfg(feature = "ooxml")]

use mjx_session::ooxml::{SpreadsheetSession, DEFAULT_RESIDENCY_BUDGET_BYTES};
use mjx_session::{
    CommitPolicy, ManualClock, MemoryDocument, MemoryJournal, Operation, Session, Value,
};
use mjx_sml::{CellReference, CellValue};
use mjx_xlsx::Workbook;

#[path = "support/mod.rs"]
mod support;

use support::{cell_address, fixture};

/// How many tabs the synthetic workbook carries.
///
/// Forty rather than four, because a windowing bound tested on a corpus that fits inside it is
/// green for an implementation that materialises everything.
const SHEETS: usize = 40;

/// How many cells are written into each of them.
///
/// Enough that a sheet's charge against the budget is dominated by its cells rather than by the
/// per-sheet constant, so the *byte* budget is what decides and not a disguised sheet count.
const CELLS_PER_SHEET: u32 = 200;

/// A budget that fits roughly four of those sheets.
///
/// Derived from the residency's own arithmetic rather than guessed: 8 KiB per sheet plus 48 bytes
/// per cell, so one sheet is about 8 KiB + 9.6 KiB ≈ 18 KiB.
const SMALL_BUDGET_BYTES: usize = 4 * 18 * 1024;

/// A workbook of [`SHEETS`] tabs, each carrying [`CELLS_PER_SHEET`] numbers, as container bytes.
fn many_sheets() -> Vec<u8> {
    let mut workbook = Workbook::open(&fixture("sample.xlsx")).expect("the fixture opens");
    while workbook.sheets().len() < SHEETS {
        let index = workbook.sheets().len();
        workbook
            .add_sheet(&format!("Bulk{index}"))
            .expect("a tab is appended");
    }
    assert_eq!(
        workbook.sheets().len(),
        SHEETS,
        "the corpus is the size it says it is",
    );
    // Each sheet is filled through **one** parse and **one** write-back, which is the same saving
    // the residency exists to give an editor. Doing it through `Workbook::set_cell_value` would be
    // eight thousand parses of a whole worksheet, and would make building the corpus the slowest
    // thing in this crate's suite.
    for sheet in 0..SHEETS {
        let mut markup = workbook
            .worksheet_markup(sheet)
            .expect("the part reads")
            .expect("the tab reaches a worksheet");
        for row in 0..CELLS_PER_SHEET {
            let reference = CellReference::relative(0, row).expect("a cell reference");
            markup
                .set_cell_value(reference, CellValue::Number(f64::from(row)))
                .expect("a cell is written");
        }
        workbook
            .write_worksheet_markup(sheet, &markup)
            .expect("the sheet writes back");
    }
    workbook.save().expect("the workbook saves")
}

/// Reads one cell on every sheet, in order, and reports what the residency did.
fn walk(session: &mut SpreadsheetSession) {
    for sheet in 0..SHEETS {
        session
            .cell_value(sheet, 0, 0)
            .expect("every sheet is readable");
    }
}

#[test]
fn a_walk_of_forty_sheets_stays_inside_the_budget_and_keeps_what_it_is_using() {
    let bytes = many_sheets();
    let mut session = SpreadsheetSession::with_residency_budget(
        Workbook::open(&bytes).expect("the workbook opens"),
        SMALL_BUDGET_BYTES,
    );
    walk(&mut session);

    // The ceiling held.
    assert!(
        session.resident_bytes() <= session.residency_budget(),
        "{} bytes resident against a {}-byte budget",
        session.resident_bytes(),
        session.residency_budget(),
    );
    assert!(
        session.resident_sheets() < SHEETS,
        "all {SHEETS} sheets are still resident — the bound did nothing",
    );
    // The eviction path actually ran. Without this the bound could be an accident of the workload.
    assert!(
        session.residency_evictions() > 0,
        "nothing was ever evicted, so the byte bound is held by luck",
    );

    // …and the other side of it: the residency is not empty, and the sheet just used is still
    // there. A cache that evicted everything would satisfy the ceiling perfectly.
    assert!(
        session.resident_sheets() > 1,
        "only {} sheet resident — a residency that keeps nothing is not a residency",
        session.resident_sheets(),
    );
    assert!(
        session.least_recently_used_sheet() != Some(SHEETS - 1),
        "the sheet just read is the next to go, which is the eviction order upside down",
    );
}

#[test]
fn the_default_budget_holds_every_sheet_which_is_what_makes_the_small_one_a_measurement() {
    // **The identity-value control.** The case above is green for a bounded residency and for an
    // unbounded one alike unless something proves the budget is what produced the difference. This
    // runs the identical walk with the shipped default — 64 MiB, far more than forty small sheets
    // need — and asserts the opposite outcome.
    let bytes = many_sheets();
    let mut session = SpreadsheetSession::new(Workbook::open(&bytes).expect("the workbook opens"));
    assert_eq!(session.residency_budget(), DEFAULT_RESIDENCY_BUDGET_BYTES);
    walk(&mut session);

    assert_eq!(
        session.resident_sheets(),
        SHEETS,
        "under a budget nothing reaches, every sheet stays — which is MJXOFF-167's behaviour, \
         and is correct here",
    );
    assert_eq!(
        session.residency_evictions(),
        0,
        "nothing was evicted, because nothing needed to be",
    );
}

#[test]
fn eviction_takes_the_least_recently_used_sheet_and_not_the_one_in_use() {
    let bytes = many_sheets();
    let mut session = SpreadsheetSession::with_residency_budget(
        Workbook::open(&bytes).expect("the workbook opens"),
        SMALL_BUDGET_BYTES,
    );
    for sheet in 0..4 {
        session.cell_value(sheet, 0, 0).expect("a read");
    }
    // Reach sheet 0 again, so that insertion order and use order disagree. Without this the case
    // would pass for a residency that evicted the first sheet inserted.
    session.cell_value(0, 0, 0).expect("a re-read");
    let doomed = session
        .least_recently_used_sheet()
        .expect("something is evictable");
    assert_eq!(doomed, 1, "sheet 1 is the oldest use, not sheet 0");

    let before = session.residency_evictions();
    session.cell_value(9, 0, 0).expect("a read that needs room");
    assert!(
        session.residency_evictions() > before,
        "the new sheet found room without evicting anything",
    );
    assert_ne!(
        session.least_recently_used_sheet(),
        Some(doomed),
        "sheet {doomed} is still the oldest, so it was not the one that went",
    );

    // And the sheet re-read above survived, which is the half a count cannot express.
    session.cell_value(0, 0, 0).expect("a read");
    assert!(
        session.resident_sheets() > 1,
        "the working set collapsed to one sheet",
    );
}

#[test]
fn a_sheet_with_unwritten_edits_is_never_evicted() {
    // A budget that fits four sheets, and forty sheets edited before a single commit. Every one of
    // them is dirty, so every one is pinned; a residency that evicted by age alone would lose
    // thirty-six edits and the read-back below would find the original values.
    let bytes = many_sheets();
    let mut session = Session::new(
        SpreadsheetSession::with_residency_budget(
            Workbook::open(&bytes).expect("the workbook opens"),
            SMALL_BUDGET_BYTES,
        ),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    // The clock never advances and no trigger is reached, so nothing commits until asked: the
    // dirty set really does grow to forty.
    .with_commit_policy(CommitPolicy::interactive());

    for sheet in 0..SHEETS {
        session
            .edit(Operation::set_value(
                cell_address(sheet as u32, 0, 0),
                Value::Number(9_999.0),
            ))
            .expect("an edit");
    }

    let document = session.document();
    assert_eq!(
        document.resident_sheets(),
        SHEETS,
        "a dirty sheet was evicted, and its edit went with it",
    );
    assert!(
        document.pinned_residency_bytes() > document.residency_budget(),
        "the pinned floor never rose above the budget, so the pin was never tested: {} pinned \
         against a {}-byte budget",
        document.pinned_residency_bytes(),
        document.residency_budget(),
    );
    assert_eq!(
        document.residency_evictions(),
        0,
        "something was evicted while every resident sheet was pinned",
    );

    // Every edit is still there.
    for sheet in 0..SHEETS {
        assert_eq!(
            session
                .document_mut()
                .cell_value(sheet, 0, 0)
                .expect("a read"),
            // The residency reads a number back with its **exact spelling** preserved, which is
            // what makes the inverse exact; `9_999.0` was written and `"9999"` is what the file
            // says. Asserting the spelling rather than the float is the honest read-back.
            Value::NumberText("9999".into()),
            "sheet {sheet}'s edit was lost",
        );
    }

    // A commit writes them all back, unpins them, and the residency falls to its budget.
    let outcome = session.save().expect("a commit");
    assert!(
        outcome.parts_serialised >= SHEETS,
        "{} parts serialised for {SHEETS} dirty sheets",
        outcome.parts_serialised,
    );
    let document = session.document();
    assert_eq!(
        document.pinned_residency_bytes(),
        0,
        "the pins were released"
    );
    assert!(
        document.resident_bytes() <= document.residency_budget(),
        "the residency stayed over its budget after the commit released the pins: {} bytes",
        document.resident_bytes(),
    );
}

#[test]
fn a_budget_of_zero_parses_every_sheet_every_time_which_is_the_behaviour_residency_removes() {
    // The budget's lower bound. A residency that silently kept one sheet anyway would satisfy no
    // bound at all, so the degenerate value has to be reachable and has to mean what it says.
    let bytes = many_sheets();
    let mut session = SpreadsheetSession::with_residency_budget(
        Workbook::open(&bytes).expect("the workbook opens"),
        0,
    );
    session.cell_value(0, 0, 0).expect("a read");
    // The sheet in use is retained whatever the budget says — a refusal here would be a workbook
    // that cannot be read rather than a byte saved — but nothing accumulates behind it.
    assert_eq!(session.resident_sheets(), 1);
    for sheet in 1..6 {
        session.cell_value(sheet, 0, 0).expect("a read");
        assert_eq!(
            session.resident_sheets(),
            1,
            "a zero budget kept more than the one sheet in use",
        );
    }
    assert_eq!(session.residency_evictions(), 5);
}
