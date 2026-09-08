//! Recovery, proved against a **real process kill**.
//!
//! # Why a same-process replay proves nothing
//!
//! The tempting test is: record some operations, decode the journal, assert the operations come
//! back. That is self-consistency. It exercises the encoder against the decoder and says nothing at
//! all about durability, because the bytes never left the process and no crash was involved: an
//! implementation that buffered everything in memory and wrote it on `Drop` would pass it, and would
//! lose every operation in the situation the journal exists for.
//!
//! So [`work_that_reached_the_disk_survives_a_killed_process`] launches this very test binary again,
//! has the child record operations through a **file** sink and then `abort()` — SIGABRT, no
//! unwinding, no destructors, no buffered write flushed — and reads the file back from the parent.
//! What survives is what actually reached the disk and nothing else.
//!
//! # And the exposure window is measured, not assumed
//!
//! The child records four operations and flushes, then records two more and does **not** flush
//! before dying. The parent asserts both halves: the four are there, and the two are not. That is
//! the difference between "the journal works" and "the journal bounds the exposure window", and only
//! the second is a claim about how much a crash costs.
//!
//! The complementary property — that a record caught *mid-write* costs only that record — is
//! asserted exhaustively in `src/journal.rs`, at every byte offset inside the final record. A kill
//! cannot be aimed at a byte, so the two halves are tested where each can be.

use std::path::Path;
use std::process::Command;

use mjx_session::{
    CommitPolicy, ManualClock, MemoryDocument, Operation, Recovery, ResidentDocument, Session,
    Value,
};

#[path = "support/mod.rs"]
mod support;

use support::{scratch_directory, FileDocument, FileJournal, PlainDocument};

/// Where the child writes its journal. Not an `MJX_REQUIRE_…` escape: it is how the parent tells the
/// child where to work, and the child is never run except by the parent.
const JOURNAL_PATH: &str = "MJX_SESSION_RECOVERY_JOURNAL";

/// What the child records before its flush, and what it records after.
const FLUSHED: [&str; 4] = ["alpha", "bravo", "charlie", "delta"];
const UNFLUSHED: [&str; 2] = ["echo", "foxtrot"];

#[test]
fn work_that_reached_the_disk_survives_a_killed_process() {
    let directory = scratch_directory("recovery");
    let journal = directory.join("session.journal");

    let executable = std::env::current_exe().expect("this test binary's own path");
    let status = Command::new(executable)
        .env(JOURNAL_PATH, &journal)
        .args(["--exact", "the_child_records_and_dies", "--ignored"])
        .status()
        .expect("the child process starts");
    assert!(
        !status.success(),
        "the child is supposed to abort; it exited cleanly with {status:?}, which means the kill \
         never happened and this test proved nothing"
    );

    let bytes = std::fs::read(&journal).expect("the journal the child left behind");
    let recovered = Recovery::of(&bytes).expect("a readable journal");

    assert_eq!(
        recovered.len(),
        FLUSHED.len(),
        "the flushed operations are what reached the disk"
    );
    let mut document = PlainDocument::new(1, 1);
    for operation in recovered.operations() {
        document.apply(operation).expect("a replay");
    }
    assert_eq!(
        document.text(0, 0),
        FLUSHED.last().copied(),
        "recovery lands on the last operation that was flushed"
    );

    let survived: Vec<String> = recovered
        .entries()
        .iter()
        .map(|entry| format!("{:?}", entry.operation.kind()))
        .collect();
    for lost in UNFLUSHED {
        assert!(
            !survived.iter().any(|it| it.contains(lost)),
            "`{lost}` was recorded but never flushed, and must not come back — the exposure window \
             is exactly the unflushed tail"
        );
    }

    let _ = std::fs::remove_dir_all(&directory);
}

/// The child. Records, flushes, records more, and dies without unwinding.
///
/// `#[ignore]` so an ordinary `cargo test` never runs it; the parent above selects it by name with
/// `--ignored`. Running it by hand aborts the test process, which is the point.
#[test]
#[ignore = "run as a child process by `work_that_reached_the_disk_survives_a_killed_process`; it aborts on purpose"]
fn the_child_records_and_dies() {
    let Ok(path) = std::env::var(JOURNAL_PATH) else {
        panic!("{JOURNAL_PATH} is unset — this test is only ever run by its parent");
    };
    let path = Path::new(&path);
    let mut session = Session::new(
        PlainDocument::new(1, 1),
        ManualClock::new(),
        FileJournal::create(path),
        FileDocument::at(&path.with_extension("committed")),
    )
    // Nothing may commit on its own: a commit would truncate the journal, and the whole point is to
    // read back a journal that outlived the process that wrote it.
    .with_commit_policy(CommitPolicy::manual());

    for text in FLUSHED {
        session
            .edit(Operation::set_value(
                PlainDocument::address(0, 0),
                Value::text(text),
            ))
            .expect("an edit");
        session.clock().advance(50);
    }
    session
        .flush_journal()
        .expect("a flush that reaches the file");

    for text in UNFLUSHED {
        session
            .edit(Operation::set_value(
                PlainDocument::address(0, 0),
                Value::text(text),
            ))
            .expect("an edit");
        session.clock().advance(50);
    }

    // No unwinding, no destructors, no buffered write flushed.
    std::process::abort();
}

#[test]
fn a_commit_truncates_the_journal_so_recovery_is_the_document_plus_the_tail() {
    let directory = scratch_directory("truncation");
    let journal_path = directory.join("session.journal");
    let document_path = directory.join("session.committed");

    let mut session = Session::new(
        PlainDocument::new(1, 2),
        ManualClock::new(),
        FileJournal::create(&journal_path),
        FileDocument::at(&document_path),
    )
    .with_commit_policy(CommitPolicy::manual());

    session
        .edit(Operation::set_value(
            PlainDocument::address(0, 0),
            Value::text("before the commit"),
        ))
        .expect("an edit");
    session.flush_journal().expect("a flush");
    assert!(
        std::fs::read(&journal_path)
            .expect("the journal file")
            .len()
            > 6,
        "the record reached the file"
    );

    session.save().expect("a commit");
    assert!(
        std::fs::read(&journal_path)
            .expect("the journal file")
            .is_empty(),
        "the commit truncated the journal, which is what keeps it from growing without bound"
    );
    assert!(!std::fs::read(&document_path)
        .expect("the committed document")
        .is_empty());

    // And the next record starts a new file, header and all.
    session
        .edit(Operation::set_value(
            PlainDocument::address(0, 1),
            Value::text("after the commit"),
        ))
        .expect("an edit");
    session.flush_journal().expect("a flush");
    let tail = std::fs::read(&journal_path).expect("the journal file");
    let recovered = Recovery::of(&tail).expect("a readable journal");
    assert_eq!(recovered.len(), 1, "the tail is what happened since");

    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn a_refused_document_write_leaves_the_journal_alone() {
    // The commit order is document first, journal second, precisely so that a failure between them
    // costs nothing: the work is still recorded and still recoverable.
    #[derive(Debug, Default)]
    struct RefusingSink;
    impl mjx_session::DocumentSink for RefusingSink {
        fn write(&mut self, _: &[u8]) -> Result<(), mjx_session::SinkError> {
            Err(mjx_session::SinkError::new("the disk is full"))
        }
    }

    let mut session = Session::new(
        PlainDocument::new(1, 1),
        ManualClock::new(),
        mjx_session::MemoryJournal::new(),
        RefusingSink,
    )
    .with_commit_policy(CommitPolicy::manual());

    session
        .edit(Operation::set_value(
            PlainDocument::address(0, 0),
            Value::text("work"),
        ))
        .expect("an edit");
    let error = session.save().expect_err("the sink refuses");
    assert!(matches!(error, mjx_session::SessionError::Sink(_)));
    assert_eq!(
        session.journal().len(),
        1,
        "nothing recorded is lost when a commit fails"
    );
    assert_eq!(session.stats().commits, 0);
}

#[test]
fn a_refused_journal_write_can_be_retried_without_losing_a_record() {
    #[derive(Debug, Default)]
    struct RefusingOnce {
        refused: bool,
        accepted: Vec<u8>,
    }
    impl mjx_session::JournalSink for RefusingOnce {
        fn append(&mut self, record_bytes: &[u8]) -> Result<(), mjx_session::SinkError> {
            if !self.refused {
                self.refused = true;
                return Err(mjx_session::SinkError::new("the first write is refused"));
            }
            self.accepted.extend_from_slice(record_bytes);
            Ok(())
        }
        fn flush(&mut self) -> Result<(), mjx_session::SinkError> {
            Ok(())
        }
        fn truncate(&mut self) -> Result<(), mjx_session::SinkError> {
            self.accepted.clear();
            Ok(())
        }
    }

    let mut session = Session::new(
        PlainDocument::new(1, 1),
        ManualClock::new(),
        RefusingOnce::default(),
        MemoryDocument::new(),
    )
    .with_commit_policy(CommitPolicy::manual());

    session
        .edit(Operation::set_value(
            PlainDocument::address(0, 0),
            Value::text("work"),
        ))
        .expect("an edit");
    assert!(session.flush_journal().is_err());
    assert!(
        session.journal().wants_header(),
        "a sink that accepted nothing has no header either, so the retry must write one"
    );
    assert_eq!(
        session.flush_journal().expect("the retry"),
        1,
        "the record was not marked flushed by the attempt that failed"
    );
}
