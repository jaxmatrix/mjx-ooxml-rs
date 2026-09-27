//! Where the two artefacts go — and why there are two of them.
//!
//! `docs/client-platform/SESSION_AND_PERSISTENCE.md` §4: batching the expensive layer only avoids
//! losing work because a **second, cheap** artefact is not batched. So a session writes
//!
//! * the **journal** — append-only, small, sequential, flushed sub-second; and
//! * the **document** — the whole committed container, written rarely.
//!
//! Recovery is the last committed document plus the journal tail after it, and the journal is
//! truncated at each successful commit.
//!
//! # Why both go through an injected sink
//!
//! `wasm32` has no files, and nothing below the facade in this workspace has ever touched a
//! filesystem. A session that called `std::fs` would be a session that cannot run in a browser, so
//! the crate ships the two traits and one in-memory implementation of each, and the host supplies
//! the rest: a file on the desktop, the origin-private filesystem in a browser, an object store on a
//! server. [`crate::Session`]'s own suites drive a file-backed pair defined in the test tree, which
//! is also what lets the recovery test **kill a real process** and read back what actually reached
//! the disk.

use std::sync::{Arc, Mutex};

use crate::error::SinkError;

/// Where journal records go.
///
/// Records arrive framed and self-delimiting (see [`crate::journal`]), so an implementation appends
/// bytes and never has to understand them. A torn final record — the shape a crash leaves — is the
/// reader's problem, not the writer's.
pub trait JournalSink: Send {
    /// Appends `record_bytes` to the journal. Does not have to reach durable storage until
    /// [`flush`](Self::flush).
    ///
    /// # Errors
    /// [`SinkError`] if the write is refused. The session treats that as fatal to the flush and
    /// keeps the records, so nothing is dropped on the floor.
    fn append(&mut self, record_bytes: &[u8]) -> Result<(), SinkError>;

    /// Makes everything appended so far durable.
    ///
    /// # Errors
    /// [`SinkError`] if the flush is refused.
    fn flush(&mut self) -> Result<(), SinkError>;

    /// Discards the journal, because the document it described has been committed.
    ///
    /// # Errors
    /// [`SinkError`] if the truncation is refused. A journal that will not truncate is not fatal to
    /// the commit — the document is already written and replaying the tail over it is idempotent —
    /// but the session reports it.
    fn truncate(&mut self) -> Result<(), SinkError>;
}

/// Where the committed document goes.
pub trait DocumentSink: Send {
    /// Writes the whole container.
    ///
    /// # Errors
    /// [`SinkError`] if the write is refused. The session then leaves the journal alone, so the
    /// work is still recoverable.
    fn write(&mut self, container_bytes: &[u8]) -> Result<(), SinkError>;
}

/// A journal in memory, and a handle onto what it holds.
///
/// The handle is what makes it useful beyond a smoke test: a caller can read the bytes the sink
/// received *while the session is still running*, which is how the exposure window is measured.
#[derive(Clone, Debug, Default)]
pub struct MemoryJournal {
    bytes: Arc<Mutex<Vec<u8>>>,
    flushes: Arc<Mutex<usize>>,
}

impl MemoryJournal {
    /// An empty journal.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Everything appended and not truncated. Empty if the mutex was poisoned by a panicking
    /// writer, which is the only state this can be in that a caller cannot fix.
    #[must_use]
    pub fn contents(&self) -> Vec<u8> {
        self.bytes.lock().map(|it| it.clone()).unwrap_or_default()
    }

    /// How many times [`flush`](JournalSink::flush) has been called.
    #[must_use]
    pub fn flush_count(&self) -> usize {
        self.flushes.lock().map(|it| *it).unwrap_or_default()
    }
}

impl JournalSink for MemoryJournal {
    fn append(&mut self, record_bytes: &[u8]) -> Result<(), SinkError> {
        let mut bytes = self
            .bytes
            .lock()
            .map_err(|_| SinkError::new("the in-memory journal's lock was poisoned"))?;
        bytes.extend_from_slice(record_bytes);
        Ok(())
    }

    fn flush(&mut self) -> Result<(), SinkError> {
        let mut flushes = self
            .flushes
            .lock()
            .map_err(|_| SinkError::new("the in-memory journal's lock was poisoned"))?;
        *flushes += 1;
        Ok(())
    }

    fn truncate(&mut self) -> Result<(), SinkError> {
        let mut bytes = self
            .bytes
            .lock()
            .map_err(|_| SinkError::new("the in-memory journal's lock was poisoned"))?;
        bytes.clear();
        Ok(())
    }
}

/// A committed document in memory, and a handle onto the last bytes written.
#[derive(Clone, Debug, Default)]
pub struct MemoryDocument {
    bytes: Arc<Mutex<Option<Vec<u8>>>>,
    writes: Arc<Mutex<usize>>,
}

impl MemoryDocument {
    /// A sink nothing has been written to yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The most recently committed container, or `None` before the first commit.
    #[must_use]
    pub fn contents(&self) -> Option<Vec<u8>> {
        self.bytes.lock().ok().and_then(|it| it.clone())
    }

    /// How many commits have been written.
    #[must_use]
    pub fn write_count(&self) -> usize {
        self.writes.lock().map(|it| *it).unwrap_or_default()
    }
}

impl DocumentSink for MemoryDocument {
    fn write(&mut self, container_bytes: &[u8]) -> Result<(), SinkError> {
        let mut bytes = self
            .bytes
            .lock()
            .map_err(|_| SinkError::new("the in-memory document sink's lock was poisoned"))?;
        *bytes = Some(container_bytes.to_vec());
        drop(bytes);
        let mut writes = self
            .writes
            .lock()
            .map_err(|_| SinkError::new("the in-memory document sink's lock was poisoned"))?;
        *writes += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_memory_journal_accumulates_appends_and_forgets_them_on_truncate() {
        let handle = MemoryJournal::new();
        let mut sink = handle.clone();
        sink.append(b"one").expect("an append");
        sink.append(b"two").expect("an append");
        assert_eq!(handle.contents(), b"onetwo");
        assert_eq!(handle.flush_count(), 0);
        sink.flush().expect("a flush");
        assert_eq!(handle.flush_count(), 1);
        sink.truncate().expect("a truncate");
        assert!(handle.contents().is_empty());
    }

    #[test]
    fn a_memory_document_keeps_only_the_last_write_and_counts_them() {
        let handle = MemoryDocument::new();
        let mut sink = handle.clone();
        assert_eq!(handle.contents(), None);
        sink.write(b"first").expect("a write");
        sink.write(b"second").expect("a write");
        assert_eq!(handle.contents().as_deref(), Some(&b"second"[..]));
        assert_eq!(handle.write_count(), 2);
    }
}
