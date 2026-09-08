//! Undo units and commit batches are independent, **asserted in both directions**.
//!
//! # Why both directions
//!
//! The classic bug is undo jumping by however much happened to be batched. A suite that only checked
//! *"a unit spanning two commits undoes as one"* would be green for an implementation that made
//! units far **smaller** than batches by accident — every keystroke its own undo — which is the same
//! bug wearing the other costume and just as visible to a user. So:
//!
//! * [`one_undo_unit_spanning_two_commits_undoes_as_one`], and
//! * [`one_commit_containing_three_undo_units_undoes_as_three`].
//!
//! Neither passes for an implementation that ties the two together in either direction.

use mjx_layout::LayoutRect;
use mjx_ooxml_core::measure::Emu;
use mjx_session::{
    CommitPolicy, ManualClock, MemoryDocument, MemoryJournal, Operation, Session, UndoPolicy, Value,
};

#[path = "support/mod.rs"]
mod support;

use support::PlainDocument;

fn session() -> Session<PlainDocument, ManualClock> {
    Session::new(
        PlainDocument::new(2, 4),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::manual())
    .with_undo_policy(UndoPolicy::default())
}

fn type_into(session: &mut Session<PlainDocument, ManualClock>, node: u32, text: &str) {
    session
        .edit(Operation::set_value(
            PlainDocument::address(0, node),
            Value::text(text),
        ))
        .expect("an edit");
}

#[test]
fn one_undo_unit_spanning_two_commits_undoes_as_one() {
    let mut session = session();

    // Ten keystrokes, a commit in the middle of them, ten more — all one word, at one address, with
    // no pause long enough to end the unit.
    let mut so_far = String::new();
    for keystroke in 0..20 {
        so_far.push('x');
        type_into(&mut session, 0, &so_far);
        session.clock().advance(50);
        if keystroke == 9 {
            session.save().expect("a commit lands mid-word");
        }
    }
    session.save().expect("a second commit");

    assert_eq!(session.stats().commits, 2);
    assert_eq!(
        session.undo_units().undoable(),
        1,
        "two commits happened inside one word; the word is still one undo"
    );
    assert_eq!(session.document().text(0, 0), Some(so_far.as_str()));

    session.undo().expect("an undo").expect("a unit");
    assert_eq!(
        session.document().text(0, 0),
        Some(""),
        "one undo took back the whole word, not the half that was in the second batch"
    );
    assert_eq!(session.undo_units().undoable(), 0);

    session.redo().expect("a redo").expect("a unit");
    assert_eq!(session.document().text(0, 0), Some(so_far.as_str()));
}

#[test]
fn one_commit_containing_three_undo_units_undoes_as_three() {
    let mut session = session();

    // Three separate things, all inside one commit window: a word here, a word there, a drag.
    type_into(&mut session, 0, "first");
    session.clock().advance(50);
    type_into(&mut session, 1, "second");
    session.clock().advance(50);
    session.begin_gesture();
    for step in 0..8_i64 {
        session
            .edit(Operation::set_bounds(
                PlainDocument::address(0, 2),
                LayoutRect::from_edges(
                    Emu::from_emu(step),
                    Emu::from_emu(0),
                    Emu::from_emu(step + 5),
                    Emu::from_emu(5),
                ),
            ))
            .expect("an edit");
        session.clock().advance(16);
    }
    session.end_gesture();

    let outcome = session.save().expect("one commit for all three");
    assert_eq!(session.stats().commits, 1);
    assert_eq!(outcome.operations_committed, 10);
    assert_eq!(
        session.undo_units().undoable(),
        3,
        "one batch, three semantic units"
    );

    session.undo().expect("an undo").expect("the drag");
    assert_eq!(session.document().boxed(0, 2), Some(LayoutRect::ZERO));
    assert_eq!(session.document().text(0, 1), Some("second"));

    session.undo().expect("an undo").expect("the second word");
    assert_eq!(session.document().text(0, 1), Some(""));
    assert_eq!(session.document().text(0, 0), Some("first"));

    session.undo().expect("an undo").expect("the first word");
    assert_eq!(session.document().text(0, 0), Some(""));

    assert!(
        session.undo().expect("an undo").is_none(),
        "and there is nothing left"
    );
}

#[test]
fn a_pause_ends_a_unit_and_a_commit_does_not() {
    let mut session = session();
    type_into(&mut session, 0, "a");
    session.save().expect("a commit");
    session.clock().advance(50);
    type_into(&mut session, 0, "ab");
    assert_eq!(
        session.undo_units().undoable(),
        1,
        "a commit is economics and must not end a word"
    );

    session.clock().advance(5_000);
    type_into(&mut session, 0, "abc");
    assert_eq!(
        session.undo_units().undoable(),
        2,
        "a pause is semantics and does"
    );
}

#[test]
fn undo_and_redo_are_journalled_so_a_recovery_lands_where_the_user_was() {
    let journal = MemoryJournal::new();
    let mut session = Session::new(
        PlainDocument::new(2, 4),
        ManualClock::new(),
        journal.clone(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::manual());

    type_into(&mut session, 0, "one");
    session.clock().advance(5_000);
    type_into(&mut session, 0, "two");
    session.undo().expect("an undo").expect("a unit");
    session.flush_journal().expect("a flush");

    let recovered = mjx_session::Recovery::of(&journal.contents()).expect("a readable journal");
    assert_eq!(recovered.len(), 3, "two edits and the undo that followed");
    assert_eq!(recovered.undo_records(), 1);

    // Replaying the whole tail onto a fresh document lands on what the user was looking at, which is
    // "one" and not "two".
    let mut replayed = PlainDocument::new(2, 4);
    for operation in recovered.operations() {
        mjx_session::ResidentDocument::apply(&mut replayed, operation).expect("a replay");
    }
    assert_eq!(replayed.text(0, 0), Some("one"));

    // And again after a redo, because a redo is a record of its own kind and lands the replay
    // somewhere else. Without this the name of this test would be claiming half of what it checks.
    session.redo().expect("a redo").expect("a unit");
    session.flush_journal().expect("a flush");
    let recovered = mjx_session::Recovery::of(&journal.contents()).expect("a readable journal");
    assert_eq!(recovered.len(), 4, "and the redo that followed the undo");
    assert_eq!(
        recovered.undo_records(),
        2,
        "one undo record and one redo record"
    );

    let mut replayed = PlainDocument::new(2, 4);
    for operation in recovered.operations() {
        mjx_session::ResidentDocument::apply(&mut replayed, operation).expect("a replay");
    }
    assert_eq!(
        replayed.text(0, 0),
        Some("two"),
        "the user redid it before the crash, so recovery must too"
    );
}

#[test]
fn a_new_edit_after_an_undo_discards_the_redo_branch() {
    let mut session = session();
    type_into(&mut session, 0, "one");
    session.clock().advance(5_000);
    type_into(&mut session, 0, "two");
    session.undo().expect("an undo").expect("a unit");
    assert_eq!(session.undo_units().redoable(), 1);

    session.clock().advance(5_000);
    type_into(&mut session, 0, "three");
    assert_eq!(session.undo_units().redoable(), 0);
    assert!(session.redo().expect("a redo").is_none());
}

#[test]
fn replaying_the_journal_twice_lands_in_the_same_place() {
    // Every operation is an absolute assignment, which is what makes a commit safe to do *before*
    // truncating the journal: the tail may describe work the document already carries.
    let journal = MemoryJournal::new();
    let mut session = Session::new(
        PlainDocument::new(2, 4),
        ManualClock::new(),
        journal.clone(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::manual());

    type_into(&mut session, 0, "one");
    session.clock().advance(5_000);
    type_into(&mut session, 1, "two");
    session.flush_journal().expect("a flush");

    let recovered = mjx_session::Recovery::of(&journal.contents()).expect("a readable journal");
    let mut once = PlainDocument::new(2, 4);
    let mut twice = PlainDocument::new(2, 4);
    for operation in recovered.operations() {
        mjx_session::ResidentDocument::apply(&mut once, operation).expect("a replay");
    }
    for _ in 0..2 {
        for operation in recovered.operations() {
            mjx_session::ResidentDocument::apply(&mut twice, operation).expect("a replay");
        }
    }
    assert_eq!(once.text(0, 0), twice.text(0, 0));
    assert_eq!(once.text(0, 1), twice.text(0, 1));
    assert_eq!(once.text(0, 0), Some("one"));
    assert_eq!(once.text(0, 1), Some("two"));
}
