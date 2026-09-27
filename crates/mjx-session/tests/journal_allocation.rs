//! The record path's cost, **measured** rather than asserted.
//!
//! # Two budgets, one instrument
//!
//! `docs/client-platform/SESSION_AND_PERSISTENCE.md` §7 states them:
//!
//! | Budget | Target |
//! |---|---|
//! | Record an operation | < 50 µs, **allocation-free in the common case** |
//! | Memory held by an uncommitted journal | **bounded**; forces a commit when exceeded |
//!
//! Both are claims about allocation, and the workspace already owns the instrument that turns a
//! claim about allocation into a test: `mjx-allocation-counter`, the counting global allocator
//! `mjx-sml`'s cell-store gate reads. It is installed here for the same reason it is installed
//! there — asserting a byte bound beats asserting by inspection — and this is its third consumer.
//!
//! # Why this binary has no test harness
//!
//! `mjx-sml`'s gate learned it first and this one learned it again, the expensive way: a
//! `#[global_allocator]` is **process-wide**, and `cargo test` runs a harness's cases on several
//! threads at once. The first version of this file was a three-case harness, and the zero-allocation
//! measurement read 92,376 bytes — every one of them another case's, on another thread, inside the
//! measured window. One target, one `main`, one thread.

use mjx_layout::{PartId, SourcePath, SourceRef};
use mjx_session::{
    CommitPolicy, EntryKind, Journal, ManualClock, MemoryDocument, MemoryJournal, Operation,
    Session, Timestamp, UndoUnitId, Value,
};

#[path = "support/mod.rs"]
mod support;

use support::PlainDocument;

#[global_allocator]
static ALLOCATOR: mjx_allocation_counter::Counting = mjx_allocation_counter::Counting;

/// The bound the memory-bound case runs under: 16 KiB of recorded operations.
const JOURNAL_BOUND: usize = 16 << 10;

fn main() {
    let handed_out = appending_to_a_reserved_journal_hands_the_allocator_no_work();
    println!("record path: {handed_out} bytes handed to the allocator for 1,000 operations");

    the_journal_reports_the_bytes_it_holds();
    println!("byte accounting: the journal's own figure matches the payloads it was given");

    let (highest, commits) = an_uncommitted_session_stays_inside_its_memory_bound();
    println!(
        "memory bound: {highest} bytes held at most against a {JOURNAL_BOUND}-byte bound, over \
         {commits} forced commits"
    );
}

fn address(node: u32) -> SourceRef {
    SourceRef::node(PartId::PRIMARY, SourcePath::new(&[node]))
}

/// Recording is layer 1: it must not be where a keystroke pays.
///
/// `total_allocated` rather than `live` or `peak`, because the question is *how much work did this
/// make the allocator do* — a path that allocated and freed a thousand times would read identically
/// on the other two and is exactly the shape a hot loop must not have.
///
/// The `Vec` is reserved up front, which is what "in the common case" means: a host that knows a
/// burst is coming reserves once rather than paying a doubling in the middle of one.
fn appending_to_a_reserved_journal_hands_the_allocator_no_work() -> usize {
    let mut journal = Journal::with_capacity(4_096);
    let operations: Vec<(Operation, Operation)> = (0..1_000_u32)
        .map(|index| {
            (
                Operation::set_value(address(index % 8), Value::Number(f64::from(index))),
                Operation::set_value(address(index % 8), Value::Empty),
            )
        })
        .collect();

    let before = mjx_allocation_counter::total_allocated();
    for (index, (forward, inverse)) in operations.iter().enumerate() {
        journal.append(
            EntryKind::Edit,
            UndoUnitId::new(index as u64),
            Timestamp::from_millis(index as u64),
            forward.clone(),
            inverse.clone(),
        );
    }
    let handed_out = mjx_allocation_counter::total_allocated() - before;

    assert_eq!(journal.len(), 1_000);
    assert_eq!(
        journal.heap_bytes(),
        0,
        "a numeric operation carries nothing on the heap"
    );
    assert_eq!(
        handed_out, 0,
        "recording a thousand payload-free operations into a reserved journal handed the allocator \
         {handed_out} bytes of work; the record path is layer 1 and must not be where a keystroke \
         pays"
    );
    handed_out
}

/// The figure the memory bound is read against is the payloads, and truncation forgets them.
fn the_journal_reports_the_bytes_it_holds() {
    let mut journal = Journal::with_capacity(64);
    let mut expected = 0;
    for index in 0..64_u32 {
        let text = "z".repeat(index as usize);
        expected += text.len();
        journal.append(
            EntryKind::Edit,
            UndoUnitId::new(0),
            Timestamp::ORIGIN,
            Operation::set_value(address(0), Value::text(text)),
            Operation::set_value(address(0), Value::Empty),
        );
    }
    assert_eq!(journal.heap_bytes(), expected);
    journal.truncate();
    assert_eq!(journal.heap_bytes(), 0);
}

/// A session that never pauses must not hold a day of edits.
///
/// Five thousand operations under a 16 KiB journal bound: the trigger fires, the journal truncates,
/// and the high-water mark never approaches what five thousand unbounded records would cost.
fn an_uncommitted_session_stays_inside_its_memory_bound() -> (usize, usize) {
    let mut session = Session::new(
        PlainDocument::new(1, 8),
        ManualClock::new(),
        MemoryJournal::new(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy {
        idle_millis: None,
        max_age_millis: None,
        dirty_bytes: None,
        journal_bytes: Some(JOURNAL_BOUND),
        journal_flush_millis: None,
        after_every_operation: false,
    });

    let mut highest = 0;
    let payload = "y".repeat(96);
    for index in 0..5_000_u32 {
        session
            .edit(Operation::set_value(
                address(index % 8),
                Value::text(payload.clone()),
            ))
            .expect("an edit");
        highest = highest.max(session.journal().heap_bytes());
        session.clock().advance(1);
        session.poll().expect("a poll");
    }

    assert!(
        highest <= JOURNAL_BOUND + 4 * payload.len(),
        "the journal reached {highest} bytes against a {JOURNAL_BOUND}-byte bound; the bound fires \
         on the poll *after* the operation that crossed it, so one operation's worth of overshoot \
         is the most it may hold"
    );
    let commits = session.stats().commits;
    assert!(
        commits > 20,
        "only {commits} commits for five thousand edits under a {JOURNAL_BOUND}-byte bound, which \
         means the bound never fired"
    );
    assert_eq!(session.stats().operations_recorded, 5_000);
    (highest, commits)
}
