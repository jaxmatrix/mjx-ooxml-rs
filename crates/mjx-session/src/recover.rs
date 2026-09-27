//! Recovery — the last committed document, plus the journal tail after it.
//!
//! # What batching costs, and how this pays it back
//!
//! Deferring commits means a crash loses everything since the last one, and the answer
//! (`SESSION_AND_PERSISTENCE.md` §4) is that the *journal* is not deferred: it is small, sequential
//! and flushed sub-second, so nothing is exposed beyond the last flushed operation while the
//! expensive work still happens on a schedule.
//!
//! So recovery is two artefacts and one rule:
//!
//! 1. open the document the sink last committed;
//! 2. replay the journal that survived it.
//!
//! # Why replay is safe over work already written
//!
//! A commit writes the document and *then* truncates the journal, so a crash between the two leaves
//! a journal describing operations the document already carries. Replaying it lands in the same
//! state, because every operation in this crate is an absolute assignment — see
//! [`crate::operation`]. That is why the order is this way round: the other order has a window in
//! which the work is in neither artefact.
//!
//! # An unclean shutdown is reported, not repaired
//!
//! [`Recovery::of`] returns what it found and says whether the journal was torn. The specification
//! asks for recovery to be *offered* rather than performed silently, and a caller that has not been
//! shown the choice is not in a position to make it.

use crate::error::SessionError;
use crate::journal::{decode, EntryKind, JournalEntry};
use crate::operation::Operation;

/// What was left behind by a session that did not shut down cleanly.
#[derive(Clone, PartialEq, Debug)]
pub struct Recovery {
    entries: Vec<JournalEntry>,
    torn_tail_bytes: usize,
}

impl Recovery {
    /// Reads a journal's bytes.
    ///
    /// # Errors
    /// [`SessionError::UnreadableJournal`] if the bytes are not a journal this build wrote. A
    /// *truncated* journal is not an error — that is the ordinary shape of one a crash caught
    /// mid-write, and [`was_torn`](Self::was_torn) reports it.
    pub fn of(journal_bytes: &[u8]) -> Result<Self, SessionError> {
        let decoded = decode(journal_bytes)?;
        Ok(Self {
            entries: decoded.entries,
            torn_tail_bytes: decoded.torn_tail_bytes,
        })
    }

    /// Whether the last record did not reach the sink whole.
    ///
    /// True is the *expected* state after a kill, and it is the one thing that distinguishes "the
    /// process died here" from "the process exited without committing".
    #[must_use]
    pub const fn was_torn(&self) -> bool {
        self.torn_tail_bytes > 0
    }

    /// How many trailing bytes could not be read.
    #[must_use]
    pub const fn torn_tail_bytes(&self) -> usize {
        self.torn_tail_bytes
    }

    /// Whether there is anything to recover.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many operations survived.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Every record that survived, in the order it happened — edits, undos and redos alike.
    #[must_use]
    pub fn entries(&self) -> &[JournalEntry] {
        &self.entries
    }

    /// The operations to apply, in order, to bring a document from its last commit up to the last
    /// flushed operation.
    ///
    /// Undo and redo records are included as the operations they applied, which is why they were
    /// journalled as records of their own: a user who undid something before the crash gets a
    /// recovered document with it undone.
    pub fn operations(&self) -> impl Iterator<Item = &Operation> + '_ {
        self.entries.iter().map(|entry| &entry.operation)
    }

    /// How many of the surviving records were undos or redos rather than edits.
    #[must_use]
    pub fn undo_records(&self) -> usize {
        self.entries
            .iter()
            .filter(|entry| entry.kind != EntryKind::Edit)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_layout::{PartId, SourcePath, SourceRef};

    use crate::journal::{encode_entry, header, OperationId};
    use crate::operation::{OperationKind, Value};
    use crate::schedule::Timestamp;
    use crate::undo::UndoUnitId;

    fn entry(id: u64, kind: EntryKind, text: &str) -> JournalEntry {
        let address = SourceRef::node(
            PartId::PRIMARY,
            SourcePath::new(&[0, u32::try_from(id).unwrap_or(0)]),
        );
        JournalEntry {
            id: OperationId::new(id),
            unit: UndoUnitId::new(0),
            kind,
            at: Timestamp::from_millis(id),
            operation: Operation::set_value(address.clone(), Value::text(text)),
            inverse: Operation::set_value(address, Value::Empty),
        }
    }

    fn file(entries: &[JournalEntry]) -> Vec<u8> {
        let mut bytes = header();
        for entry in entries {
            encode_entry(entry, &mut bytes);
        }
        bytes
    }

    #[test]
    fn an_untorn_journal_recovers_every_operation_in_order() {
        let entries = vec![
            entry(0, EntryKind::Edit, "a"),
            entry(1, EntryKind::Edit, "b"),
            entry(2, EntryKind::Undo, "a"),
        ];
        let recovery = Recovery::of(&file(&entries)).expect("a readable journal");
        assert!(!recovery.was_torn());
        assert_eq!(recovery.len(), 3);
        assert!(!recovery.is_empty());
        assert_eq!(recovery.undo_records(), 1);
        let texts: Vec<Value> = recovery
            .operations()
            .map(|operation| match operation.kind() {
                OperationKind::SetValue(value) => value.clone(),
                OperationKind::SetBounds(_) => Value::Empty,
            })
            .collect();
        assert_eq!(
            texts,
            vec![Value::text("a"), Value::text("b"), Value::text("a")],
            "the undo record is replayed as the operation it applied"
        );
    }

    #[test]
    fn a_torn_journal_recovers_everything_before_the_tear_and_says_so() {
        let entries = vec![
            entry(0, EntryKind::Edit, "a"),
            entry(1, EntryKind::Edit, "b"),
        ];
        let whole = file(&entries);
        let recovery = Recovery::of(&whole[..whole.len() - 3]).expect("a readable journal");
        assert!(recovery.was_torn());
        assert!(recovery.torn_tail_bytes() > 0);
        assert_eq!(recovery.len(), 1);
    }

    #[test]
    fn an_empty_journal_file_recovers_nothing_and_is_not_torn() {
        let recovery = Recovery::of(&header()).expect("a readable journal");
        assert!(recovery.is_empty());
        assert!(!recovery.was_torn());
    }

    #[test]
    fn something_that_is_not_a_journal_is_refused() {
        assert!(matches!(
            Recovery::of(b"not a journal"),
            Err(SessionError::UnreadableJournal { .. })
        ));
    }
}
