//! **The gates count serialisations**, because nothing else distinguishes a batching session from a
//! naive one.
//!
//! # The trap this file is written against
//!
//! *"A commit produces a valid document"* is green for a session that commits on every operation and
//! has no batching in it at all — the check passes precisely when the feature is absent. So does
//! *"the document round-trips"*, and so does *"the edits are all there afterwards"*. Every one of
//! those is a property of the **document**, and batching is a property of the **work**.
//!
//! What separates the two designs is how many times XML was written. So every gate below asserts a
//! count, and each of the three residencies is run **twice** — once under
//! [`CommitPolicy::interactive`] and once under [`CommitPolicy::per_operation`], which is a real
//! policy value describing the naive design. One serialisation against twenty is the measurement;
//! running only the good policy would have proved nothing.
//!
//! # And a count of one is not enough either
//!
//! A journal exercised with one operation, or with one operation *kind*, is the second trap. So:
//! three residencies plus the non-OOXML one, both operation kinds, all seven commit triggers, and a
//! five-hundred-keystroke burst measured as a ratio rather than asserted as a bound.

#![cfg(feature = "ooxml")]

use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;
use mjx_session::{
    CommitPolicy, CommitTrigger, ManualClock, MemoryDocument, MemoryJournal, Operation, Session,
    SessionError, Value,
};

use mjx_pptx::Presentation;

#[path = "support/mod.rs"]
mod support;

use support::{cell_address, first_run_in_deck, first_run_in_document, fixture, PlainDocument};

/// Twenty keystrokes, spelled the way a user produces them: each one replaces the run's text with
/// one more character.
fn typing(count: usize) -> Vec<String> {
    let mut text = String::new();
    (0..count)
        .map(|index| {
            text.push(char::from(b'a' + (index % 26) as u8));
            text.clone()
        })
        .collect()
}

fn deck_session(
    policy: CommitPolicy,
) -> (
    Session<mjx_session::ooxml::PresentationSession, ManualClock>,
    mjx_layout::SourceRef,
    MemoryDocument,
) {
    let bytes = fixture("sample.pptx");
    let mut deck = Presentation::open(&bytes).expect("the fixture opens");
    let address = first_run_in_deck(&mut deck).expect("the fixture has a run with text in it");
    let committed = MemoryDocument::new();
    let session = Session::new(
        mjx_session::ooxml::PresentationSession::new(deck),
        ManualClock::new(),
        MemoryJournal::new(),
        committed.clone(),
    )
    .with_commit_policy(policy);
    (session, address, committed)
}

// -------------------------------------------------------------------------------------------------
// The measurement, on all four residencies.
// -------------------------------------------------------------------------------------------------

#[test]
fn twenty_keystrokes_into_one_slide_run_cost_exactly_one_part_serialisation() {
    let (mut session, address, committed) = deck_session(CommitPolicy::interactive());
    for text in typing(20) {
        session
            .edit(Operation::set_value(address.clone(), Value::text(text)))
            .expect("an edit");
        session.clock().advance(40);
        assert!(
            session.poll().expect("a poll").is_none(),
            "nothing serialises while the user is typing"
        );
    }
    session.clock().advance(2_000);
    let outcome = session.poll().expect("a poll").expect("the idle commit");

    assert_eq!(
        outcome.parts_serialised, 1,
        "twenty edits to one run are one dirty part and therefore one serialisation"
    );
    assert_eq!(outcome.operations_committed, 20);
    assert_eq!(session.stats().parts_serialised, 1);
    assert_eq!(session.stats().commits, 1);
    assert!(committed.contents().is_some());
}

#[test]
fn the_same_twenty_keystrokes_under_the_naive_policy_cost_twenty() {
    let (mut session, address, _committed) = deck_session(CommitPolicy::per_operation());
    for text in typing(20) {
        session
            .edit(Operation::set_value(address.clone(), Value::text(text)))
            .expect("an edit");
    }
    assert_eq!(
        session.stats().parts_serialised,
        20,
        "the counterfactual: without batching, every keystroke writes the slide's XML again"
    );
    assert_eq!(session.stats().commits, 20);
}

#[test]
fn twenty_cell_edits_to_one_sheet_cost_exactly_one_worksheet_serialisation() {
    // The most expensive thing in the workspace to do twenty times, and the one the ticket quotes.
    let bytes = fixture("sample.xlsx");
    let mut session = Session::new(
        mjx_session::ooxml::SpreadsheetSession::open(&bytes).expect("the fixture opens"),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::interactive());

    for row in 100..120_u32 {
        session
            .edit(Operation::set_value(
                cell_address(0, row, 0),
                Value::Number(f64::from(row)),
            ))
            .expect("an edit");
        session.clock().advance(40);
    }
    assert_eq!(
        session.document().resident_sheets(),
        1,
        "the worksheet was parsed once and held, not parsed twenty times"
    );

    session.clock().advance(2_000);
    let outcome = session.poll().expect("a poll").expect("the idle commit");
    assert_eq!(outcome.parts_serialised, 1);
    assert_eq!(session.stats().parts_serialised, 1);
}

#[test]
fn the_same_twenty_cell_edits_under_the_naive_policy_cost_twenty() {
    let bytes = fixture("sample.xlsx");
    let mut session = Session::new(
        mjx_session::ooxml::SpreadsheetSession::open(&bytes).expect("the fixture opens"),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::per_operation());

    for row in 100..120_u32 {
        session
            .edit(Operation::set_value(
                cell_address(0, row, 0),
                Value::Number(f64::from(row)),
            ))
            .expect("an edit");
    }
    assert_eq!(session.stats().parts_serialised, 20);
}

#[test]
fn twenty_keystrokes_into_one_word_run_cost_exactly_one_part_serialisation() {
    let bytes = fixture("sample.docx");
    let mut document = mjx_docx::Document::open(&bytes).expect("the fixture opens");
    let address =
        first_run_in_document(&mut document).expect("the fixture has a run with text in it");
    let mut session = Session::new(
        mjx_session::ooxml::WordSession::new(document),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::interactive());

    for text in typing(20) {
        session
            .edit(Operation::set_value(address.clone(), Value::text(text)))
            .expect("an edit");
        session.clock().advance(40);
    }
    session.clock().advance(2_000);
    let outcome = session.poll().expect("a poll").expect("the idle commit");
    assert_eq!(outcome.parts_serialised, 1);
}

#[test]
fn the_same_twenty_word_keystrokes_under_the_naive_policy_cost_twenty() {
    let bytes = fixture("sample.docx");
    let mut document = mjx_docx::Document::open(&bytes).expect("the fixture opens");
    let address =
        first_run_in_document(&mut document).expect("the fixture has a run with text in it");
    let mut session = Session::new(
        mjx_session::ooxml::WordSession::new(document),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::per_operation());

    for text in typing(20) {
        session
            .edit(Operation::set_value(address.clone(), Value::text(text)))
            .expect("an edit");
    }
    assert_eq!(session.stats().parts_serialised, 20);
}

// -------------------------------------------------------------------------------------------------
// The burst, measured as a ratio.
// -------------------------------------------------------------------------------------------------

#[test]
fn five_hundred_keystrokes_produce_one_commit_and_one_serialisation() {
    // Fifty milliseconds a keystroke — brisk typing — for five hundred keystrokes, which is
    // twenty-five seconds. The idle trigger never fires inside that, so the only automatic commit is
    // the thirty-second max-age one, which does not arrive either; the burst therefore costs exactly
    // the one commit that follows it. The naive design costs five hundred.
    let (mut session, address, _committed) = deck_session(CommitPolicy::interactive());
    let mut polls_that_committed = 0;
    for text in typing(500) {
        session
            .edit(Operation::set_value(address.clone(), Value::text(text)))
            .expect("an edit");
        session.clock().advance(50);
        if session.poll().expect("a poll").is_some() {
            polls_that_committed += 1;
        }
    }
    session.clock().advance(2_000);
    if session.poll().expect("a poll").is_some() {
        polls_that_committed += 1;
    }

    let operations = session.stats().operations_recorded;
    let serialisations = session.stats().parts_serialised;
    assert_eq!(operations, 500);
    assert_eq!(polls_that_committed, 1, "one commit for five hundred edits");
    assert_eq!(serialisations, 1);
    assert!(
        operations / serialisations >= 100,
        "the operation-to-serialisation ratio is {}:1, which is not batching",
        operations / serialisations
    );
}

// -------------------------------------------------------------------------------------------------
// The commit walks a dirty set.
// -------------------------------------------------------------------------------------------------

#[test]
fn a_commit_serialises_the_parts_that_were_edited_and_no_others() {
    let mut session = plain_session(CommitPolicy::interactive());
    session
        .edit(Operation::set_value(
            PlainDocument::address(1, 0),
            Value::text("one"),
        ))
        .expect("an edit");
    session
        .edit(Operation::set_value(
            PlainDocument::address(3, 2),
            Value::text("two"),
        ))
        .expect("an edit");
    let outcome = session.save().expect("an explicit save");
    assert_eq!(outcome.parts_serialised, 2);
    assert_eq!(session.document().serialised, vec![1, 3]);

    // A second commit that touched only one part serialises only that one. The naive shape — a
    // dirty set that never clears — would report two again, for ever.
    session
        .edit(Operation::set_value(
            PlainDocument::address(3, 2),
            Value::text("three"),
        ))
        .expect("an edit");
    let outcome = session.save().expect("an explicit save");
    assert_eq!(outcome.parts_serialised, 1);
    assert_eq!(session.document().serialised, vec![1, 3, 3]);
}

#[test]
fn a_second_commit_does_not_re_serialise_a_slide_nothing_touched_since_the_first() {
    // The same property, through the real copy-on-write machinery rather than a test double: the
    // package settles an edited part back to clean-but-resident, so the next save writes its bytes
    // verbatim.
    let (mut session, address, _committed) = deck_session(CommitPolicy::manual());
    session
        .edit(Operation::set_value(address.clone(), Value::text("first")))
        .expect("an edit");
    assert_eq!(session.save().expect("a save").parts_serialised, 1);
    assert_eq!(
        session.save().expect("a second save").parts_serialised,
        0,
        "nothing changed between the two, so the second commit writes no XML at all"
    );
    session
        .edit(Operation::set_value(address, Value::text("second")))
        .expect("an edit");
    assert_eq!(session.save().expect("a third save").parts_serialised, 1);
    assert_eq!(session.stats().parts_serialised, 2);
}

// -------------------------------------------------------------------------------------------------
// Every trigger, and the gesture that defers four of them.
// -------------------------------------------------------------------------------------------------

fn plain_session(policy: CommitPolicy) -> Session<PlainDocument, ManualClock> {
    Session::new(
        PlainDocument::new(4, 4),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    .with_commit_policy(policy)
}

fn one_edit(session: &mut Session<PlainDocument, ManualClock>) {
    session
        .edit(Operation::set_value(
            PlainDocument::address(0, 0),
            Value::text("x"),
        ))
        .expect("an edit");
}

#[test]
fn the_idle_trigger_fires_two_seconds_after_the_last_edit() {
    let mut session = plain_session(CommitPolicy::interactive());
    one_edit(&mut session);
    session.clock().advance(1_999);
    assert!(session.poll().expect("a poll").is_none());
    session.clock().advance(1);
    assert_eq!(
        session.poll().expect("a poll").map(|it| it.trigger),
        Some(CommitTrigger::Idle)
    );
}

#[test]
fn the_max_age_trigger_fires_while_the_typing_never_stops() {
    let mut session = plain_session(CommitPolicy::interactive());
    let mut triggers = Vec::new();
    for _ in 0..40 {
        one_edit(&mut session);
        session.clock().advance(1_000);
        if let Some(outcome) = session.poll().expect("a poll") {
            triggers.push(outcome.trigger);
        }
    }
    assert_eq!(
        triggers,
        vec![CommitTrigger::MaxAge],
        "thirty seconds of continuous editing commits once, on max age, and never on idle"
    );
}

#[test]
fn the_dirty_byte_threshold_fires_before_any_clock_does() {
    let mut session = plain_session(CommitPolicy {
        dirty_bytes: Some(64),
        ..CommitPolicy::interactive()
    });
    session
        .edit(Operation::set_value(
            PlainDocument::address(0, 0),
            Value::text("a".repeat(100)),
        ))
        .expect("an edit");
    session.clock().advance(1);
    assert_eq!(
        session.poll().expect("a poll").map(|it| it.trigger),
        Some(CommitTrigger::DirtyBytes),
        "a paste should not have to wait for a clock"
    );
}

#[test]
fn the_journal_memory_bound_fires_and_is_what_bounds_an_uncommitted_session() {
    let mut session = plain_session(CommitPolicy {
        journal_bytes: Some(256),
        idle_millis: None,
        max_age_millis: None,
        dirty_bytes: None,
        ..CommitPolicy::interactive()
    });
    let mut trigger = None;
    for index in 0..40 {
        session
            .edit(Operation::set_value(
                PlainDocument::address(0, 0),
                Value::text(format!("{index:020}")),
            ))
            .expect("an edit");
        session.clock().advance(10);
        if let Some(outcome) = session.poll().expect("a poll") {
            trigger = Some(outcome.trigger);
            break;
        }
    }
    assert_eq!(trigger, Some(CommitTrigger::JournalBytes));
    assert!(
        session.journal().heap_bytes() == 0,
        "the commit truncated the journal, which is what makes the bound a bound"
    );
}

#[test]
fn an_explicit_save_commits_immediately_and_a_gesture_cannot_defer_it() {
    let mut session = plain_session(CommitPolicy::interactive());
    session.begin_gesture();
    one_edit(&mut session);
    session.clock().advance(10_000);
    assert!(session.poll().expect("a poll").is_none(), "deferred");
    assert_eq!(
        session.save().expect("a save").trigger,
        CommitTrigger::Explicit
    );
}

#[test]
fn backgrounding_forces_a_commit_even_in_the_middle_of_a_gesture() {
    // Mandatory: iOS terminates backgrounded applications without warning, and this is the single
    // likeliest way to lose work on the mobile target.
    let mut session = plain_session(CommitPolicy::interactive());
    session.begin_gesture();
    one_edit(&mut session);
    session.clock().advance(10_000);
    assert!(session.poll().expect("a poll").is_none());

    let outcome = session
        .note_backgrounded()
        .expect("a backgrounding commit")
        .expect("it committed rather than deferring");
    assert_eq!(outcome.trigger, CommitTrigger::Backgrounded);
    assert_eq!(session.stats().commits, 1);
}

#[test]
fn a_consistency_point_commits_before_whatever_is_about_to_read_the_document() {
    let mut session = plain_session(CommitPolicy::interactive());
    one_edit(&mut session);
    let outcome = session
        .note_consistency_point()
        .expect("a consistency commit")
        .expect("it committed");
    assert_eq!(outcome.trigger, CommitTrigger::Consistency);
    assert!(
        session
            .note_consistency_point()
            .expect("a second consistency point")
            .is_some(),
        "an explicit point commits whether or not anything is pending — it is asked for, not due"
    );
}

#[test]
fn a_deferred_commit_lands_on_the_first_poll_after_the_gesture_ends() {
    let mut session = plain_session(CommitPolicy::interactive());
    session.begin_gesture();
    for step in 0..100_i64 {
        session
            .edit(Operation::set_bounds(
                PlainDocument::address(0, 1),
                LayoutRect::from_edges(
                    Emu::from_emu(step),
                    Emu::from_emu(0),
                    Emu::from_emu(step + 10),
                    Emu::from_emu(10),
                ),
            ))
            .expect("an edit");
        session.clock().advance(16);
        assert!(session.poll().expect("a poll").is_none(), "at step {step}");
    }
    session.clock().advance(5_000);
    assert!(session.poll().expect("a poll").is_none());
    session.end_gesture();
    let outcome = session
        .poll()
        .expect("a poll")
        .expect("the deferred commit");
    assert_eq!(outcome.trigger, CommitTrigger::Idle);
    assert_eq!(
        outcome.parts_serialised, 1,
        "a hundred pointer moves are one transform and one dirty part"
    );
    assert_eq!(outcome.operations_committed, 100);
}

// -------------------------------------------------------------------------------------------------
// The commit is invisible to the render pipeline.
// -------------------------------------------------------------------------------------------------

#[test]
fn invalidations_come_from_edits_and_never_from_a_commit() {
    let mut session = plain_session(CommitPolicy::interactive());
    one_edit(&mut session);
    one_edit(&mut session);
    assert_eq!(session.pending_invalidations(), 2);
    let drained = session.drain_invalidations();
    assert_eq!(drained.len(), 2);
    assert!(drained
        .iter()
        .all(|it| it.part() == mjx_layout::PartId::new(0)));
    assert_eq!(session.pending_invalidations(), 0);

    session.save().expect("a save");
    assert_eq!(
        session.pending_invalidations(),
        0,
        "layout, scene and paint caches must never see the commit at all"
    );
}

// -------------------------------------------------------------------------------------------------
// The journal is flushed on its own, much faster schedule.
// -------------------------------------------------------------------------------------------------

#[test]
fn the_journal_reaches_its_sink_long_before_the_document_does() {
    let journal = MemoryJournal::new();
    let committed = MemoryDocument::new();
    let mut session = Session::new(
        PlainDocument::new(2, 2),
        ManualClock::new(),
        journal.clone(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::interactive());

    for _ in 0..10 {
        one_edit(&mut session);
        session.clock().advance(100);
        session.poll().expect("a poll");
    }
    assert!(
        journal.flush_count() >= 3,
        "one second of typing at 250 ms a flush is at least three flushes, not {}",
        journal.flush_count()
    );
    assert!(
        !journal.contents().is_empty(),
        "the exposure window is bounded by records that actually reached the sink"
    );
    assert_eq!(
        committed.write_count(),
        0,
        "and the expensive artefact has not been written once"
    );
}

// -------------------------------------------------------------------------------------------------
// Off the frame path.
// -------------------------------------------------------------------------------------------------

#[test]
fn a_whole_session_moves_to_a_worker_thread() {
    // `SESSION_AND_PERSISTENCE.md` §3: serialisation runs on a worker thread natively. Nothing in
    // this crate spawns one — a library that started threads would be a library `wasm32` could not
    // use, and the sinks are injected for the same reason — so what it owes the host is that the
    // session is **movable**, whole, across a thread boundary. That is a claim about `Send`, and a
    // claim about `Send` is worth nothing until something actually sends it: an added `Rc` anywhere
    // in the type would fail to compile here and nowhere else.
    let bytes = fixture("sample.pptx");
    let mut deck = Presentation::open(&bytes).expect("the fixture opens");
    let address = first_run_in_deck(&mut deck).expect("a run with text");
    let committed = MemoryDocument::new();
    let session = Session::new(
        mjx_session::ooxml::PresentationSession::new(deck),
        ManualClock::new(),
        MemoryJournal::new(),
        committed.clone(),
    )
    .with_commit_policy(CommitPolicy::manual());

    let worker = std::thread::spawn(move || {
        let mut session = session;
        for text in typing(20) {
            session
                .edit(Operation::set_value(address.clone(), Value::text(text)))
                .expect("an edit");
        }
        let outcome = session.save().expect("a save on the worker");
        (
            outcome.parts_serialised,
            session.stats().operations_recorded,
        )
    });
    let (serialised, recorded) = worker.join().expect("the worker finishes");

    assert_eq!(serialised, 1);
    assert_eq!(recorded, 20);
    assert!(
        committed.contents().is_some(),
        "and the sink saw the result"
    );
}

// -------------------------------------------------------------------------------------------------
// Refusals.
// -------------------------------------------------------------------------------------------------

#[test]
fn a_refused_operation_records_nothing_and_leaves_the_schedule_alone() {
    let mut session = plain_session(CommitPolicy::interactive());
    let error = session
        .edit(Operation::set_value(
            PlainDocument::address(9, 9),
            Value::text("nowhere"),
        ))
        .expect_err("a refusal");
    assert!(matches!(error, SessionError::NoSuchNode { .. }));
    assert_eq!(session.journal().len(), 0);
    assert_eq!(session.stats().operations_recorded, 0);
    assert_eq!(session.pending_invalidations(), 0);
    assert_eq!(session.undo_units().undoable(), 0);
    session.clock().advance(10_000);
    assert!(
        session.poll().expect("a poll").is_none(),
        "an operation that did not happen must not schedule a commit"
    );
}

#[test]
fn a_spreadsheet_refuses_a_box_and_a_slide_refuses_a_pooled_string() {
    let bytes = fixture("sample.xlsx");
    let mut sheet = Session::new(
        mjx_session::ooxml::SpreadsheetSession::open(&bytes).expect("the fixture opens"),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    );
    let error = sheet
        .edit(Operation::set_bounds(
            cell_address(0, 0, 0),
            LayoutRect::ZERO,
        ))
        .expect_err("a cell has no rectangle");
    assert!(matches!(error, SessionError::Unsupported { .. }));

    let (mut deck, address, _committed) = deck_session(CommitPolicy::manual());
    let error = deck
        .edit(Operation::set_value(address, Value::PooledText(3)))
        .expect_err("a slide has no string pool");
    assert!(matches!(error, SessionError::Unsupported { .. }));
}
