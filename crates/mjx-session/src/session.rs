//! [`Session`] — the three layers wired together.
//!
//! # The whole design in one place
//!
//! ```text
//!   edit()  ──▶ document.apply()   ── layer 2, immediate, in memory only
//!           ──▶ undo units         ── semantic, independent of the schedule
//!           ──▶ journal.append()   ── layer 1, immediate, in memory only
//!           ──▶ invalidation       ── emitted here, never at commit
//!
//!   poll()  ──▶ journal flush      ── sub-second, cheap, bounds the exposure window
//!           ──▶ scheduler.due()    ── six conditions, deferred by a gesture
//!                 └▶ commit()      ── layer 3, batched: serialise dirty parts ONCE
//! ```
//!
//! Nothing on the `edit` path touches a sink or serialises a part, and nothing on the `commit` path
//! produces an invalidation. Those two sentences are the design, and both are asserted:
//! `tests/batching.rs` counts serialisations across a burst, and `invalidations_come_from_edits_and_never_from_a_commit`
//! below asserts the second.
//!
//! # Apply, then record — and why that order
//!
//! `SESSION_AND_PERSISTENCE.md` §1 says both happen immediately and says nothing about their order,
//! so it is worth stating the choice. [`Session::edit`] applies first and records second, because an
//! operation the document *refused* must not reach the journal: recovery replays the journal, and
//! replaying an operation that never happened would make the recovered document differ from the one
//! the user was looking at. Nothing is at risk in the gap — a crash between the two loses the model
//! too, since the model is in memory, so the journal and the (lost) model agree either way.

use crate::document::{Committed, Invalidation, ResidentDocument};
use crate::error::SessionError;
use crate::journal::{EntryKind, Journal, OperationId};
use crate::operation::Operation;
use crate::schedule::{Clock, CommitPolicy, CommitScheduler, CommitTrigger, Timestamp};
use crate::sink::{DocumentSink, JournalSink};
use crate::undo::{UndoPolicy, UndoUnitId, UndoUnits};

/// What a commit did.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CommitOutcome {
    /// Why it fired.
    pub trigger: CommitTrigger,
    /// How many parts were serialised — the number the batching gates count.
    pub parts_serialised: usize,
    /// How many container bytes were written.
    pub bytes_written: usize,
    /// How many recorded operations the commit made redundant.
    pub operations_committed: usize,
}

/// Everything the session has done, for a caller measuring it rather than trusting it.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct SessionStats {
    /// Operations recorded, including undo and redo steps.
    pub operations_recorded: usize,
    /// Commits that reached the document sink.
    pub commits: usize,
    /// **Parts serialised across the whole session.** A batching session and a naive one differ
    /// here and nowhere else that a test can see.
    pub parts_serialised: usize,
    /// Journal flushes that reached the journal sink.
    pub journal_flushes: usize,
    /// Undo units taken back.
    pub undos: usize,
    /// Undo units put back.
    pub redos: usize,
}

/// A document held open, with its journal and its commit schedule.
///
/// Generic over the document, so the machinery below has never heard of OOXML — see
/// [`ResidentDocument`].
pub struct Session<D: ResidentDocument, C: Clock> {
    document: D,
    clock: C,
    journal: Journal,
    units: UndoUnits,
    scheduler: CommitScheduler,
    journal_sink: Box<dyn JournalSink>,
    document_sink: Box<dyn DocumentSink>,
    /// Reused across flushes, so a session flushing four times a second for an hour allocates this
    /// once.
    flush_buffer: Vec<u8>,
    invalidations: Vec<Invalidation>,
    stats: SessionStats,
}

impl<D: ResidentDocument, C: Clock> std::fmt::Debug for Session<D, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Session")
            .field("journal", &self.journal)
            .field("units", &self.units)
            .field("scheduler", &self.scheduler)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

impl<D: ResidentDocument, C: Clock> Session<D, C> {
    /// Opens a session over `document`, reading time from `clock` and writing through the two sinks.
    ///
    /// The commit policy starts at [`CommitPolicy::interactive`] and the undo policy at its default;
    /// change either with [`with_commit_policy`](Self::with_commit_policy) or
    /// [`with_undo_policy`](Self::with_undo_policy).
    pub fn new(
        document: D,
        clock: C,
        journal_sink: impl JournalSink + 'static,
        document_sink: impl DocumentSink + 'static,
    ) -> Self {
        let origin = clock.now();
        Self {
            document,
            clock,
            journal: Journal::new(),
            units: UndoUnits::default(),
            scheduler: CommitScheduler::new(CommitPolicy::interactive(), origin),
            journal_sink: Box::new(journal_sink),
            document_sink: Box::new(document_sink),
            flush_buffer: Vec::new(),
            invalidations: Vec::new(),
            stats: SessionStats::default(),
        }
    }

    /// The same session under a different commit policy.
    #[must_use]
    pub fn with_commit_policy(mut self, policy: CommitPolicy) -> Self {
        self.scheduler.set_policy(policy);
        self
    }

    /// The same session under a different undo policy.
    #[must_use]
    pub fn with_undo_policy(mut self, policy: UndoPolicy) -> Self {
        self.units = UndoUnits::new(policy);
        self
    }

    /// The document, for reading.
    pub const fn document(&self) -> &D {
        &self.document
    }

    /// The document, mutably.
    ///
    /// **Anything changed through here is not journalled, is not undoable, and produces no
    /// invalidation.** It is here for two honest reasons and no others: a residency's *reads* may
    /// need `&mut` (a format crate that parses a part on demand does), and a format-specific
    /// operation this crate's vocabulary does not express has to be reachable. An editor's inner
    /// loop goes through [`edit`](Self::edit).
    pub fn document_mut(&mut self) -> &mut D {
        &mut self.document
    }

    /// The clock this session reads.
    pub const fn clock(&self) -> &C {
        &self.clock
    }

    /// What has been recorded since the last commit.
    pub const fn journal(&self) -> &Journal {
        &self.journal
    }

    /// The undo stacks.
    pub const fn undo_units(&self) -> &UndoUnits {
        &self.units
    }

    /// The commit schedule.
    pub const fn scheduler(&self) -> &CommitScheduler {
        &self.scheduler
    }

    /// What the session has done.
    pub const fn stats(&self) -> SessionStats {
        self.stats
    }

    // ---------------------------------------------------------------------------------------------
    // Layer 1 and layer 2: record and apply.
    // ---------------------------------------------------------------------------------------------

    /// Applies `operation`, records it, and reports what it dirtied.
    ///
    /// Allocation-light and synchronous: no sink is touched and no part is serialised, unless the
    /// policy is [`CommitPolicy::after_every_operation`], which is the design this crate exists to
    /// avoid and is here only so a gate can measure against it.
    ///
    /// # Errors
    /// [`SessionError`] if the document refuses the operation — in which case **nothing is
    /// recorded**, no invalidation is produced and the schedule is untouched — or, under the naive
    /// policy, if the commit it forces fails.
    pub fn edit(&mut self, operation: Operation) -> Result<OperationId, SessionError> {
        let now = self.clock.now();
        let applied = self.document.apply(&operation)?;
        let unit = self.units.record(&operation, applied.inverse.clone(), now);
        let id = self
            .journal
            .append(EntryKind::Edit, unit, now, operation, applied.inverse);
        self.invalidations.push(applied.invalidation);
        self.stats.operations_recorded += 1;
        self.scheduler.note_edit(now);
        if self.scheduler.policy().after_every_operation {
            self.commit(CommitTrigger::Explicit)?;
        }
        Ok(id)
    }

    /// Takes back the newest undo unit, whatever it spans.
    ///
    /// Returns the unit that was undone, or `None` when there is nothing to undo. The unit is one
    /// *semantic* unit and is unrelated to how many commits happened inside it.
    ///
    /// # Errors
    /// [`SessionError`] if the document refuses one of the inverses. The unit stays undoable and
    /// retrying is safe, because every operation here is an absolute assignment and re-applying one
    /// that already landed changes nothing.
    pub fn undo(&mut self) -> Result<Option<UndoUnitId>, SessionError> {
        let Some(unit) = self.units.take_undo() else {
            return Ok(None);
        };
        let now = self.clock.now();
        // By index, and the operation cloned out, so the unit is not borrowed when the error arm
        // has to hand it back.
        for index in (0..unit.steps().len()).rev() {
            let inverse = unit.steps()[index].inverse.clone();
            if let Err(error) = self.replay(EntryKind::Undo, unit.id(), now, &inverse) {
                self.units.return_undo(unit);
                return Err(error);
            }
        }
        let id = unit.id();
        self.units.finish_undo(unit);
        self.stats.undos += 1;
        Ok(Some(id))
    }

    /// Puts back the newest undone unit.
    ///
    /// # Errors
    /// As [`undo`](Self::undo).
    pub fn redo(&mut self) -> Result<Option<UndoUnitId>, SessionError> {
        let Some(unit) = self.units.take_redo() else {
            return Ok(None);
        };
        let now = self.clock.now();
        for index in 0..unit.steps().len() {
            let forward = unit.steps()[index].forward.clone();
            if let Err(error) = self.replay(EntryKind::Redo, unit.id(), now, &forward) {
                self.units.return_redo(unit);
                return Err(error);
            }
        }
        let id = unit.id();
        self.units.finish_redo(unit);
        self.stats.redos += 1;
        Ok(Some(id))
    }

    /// Applies one step of an undo or a redo, journalling it as what it is.
    ///
    /// Undo and redo are recorded rather than rewound out of the journal, because the journal is a
    /// history of what happened and an undo happened: recovery replays it and lands where the user
    /// actually was.
    fn replay(
        &mut self,
        kind: EntryKind,
        unit: UndoUnitId,
        now: Timestamp,
        operation: &Operation,
    ) -> Result<(), SessionError> {
        let applied = self.document.apply(operation)?;
        self.journal
            .append(kind, unit, now, operation.clone(), applied.inverse);
        self.invalidations.push(applied.invalidation);
        self.stats.operations_recorded += 1;
        self.scheduler.note_edit(now);
        Ok(())
    }

    /// Everything dirtied since the last drain, and clears the list.
    ///
    /// The render pipeline drains this once a frame. A commit never adds to it.
    pub fn drain_invalidations(&mut self) -> Vec<Invalidation> {
        std::mem::take(&mut self.invalidations)
    }

    /// How many invalidations are waiting.
    #[must_use]
    pub fn pending_invalidations(&self) -> usize {
        self.invalidations.len()
    }

    // ---------------------------------------------------------------------------------------------
    // Interaction boundaries.
    // ---------------------------------------------------------------------------------------------

    /// A gesture began: a drag, a fling, a slider. Economic commit triggers now wait for it to end.
    pub fn begin_gesture(&mut self) {
        self.scheduler.begin_gesture();
    }

    /// A gesture ended. The next poll may commit, and the next edit starts a new undo unit — one
    /// drag is one undo however long it took.
    pub fn end_gesture(&mut self) {
        self.scheduler.end_gesture();
        self.units.close_unit();
    }

    /// A semantic boundary the clock cannot see — a selection change, a menu command. Ends the undo
    /// unit in progress and nothing else.
    pub fn close_undo_unit(&mut self) {
        self.units.close_unit();
    }

    // ---------------------------------------------------------------------------------------------
    // Layer 3: commit.
    // ---------------------------------------------------------------------------------------------

    /// Flushes the journal if the schedule says so, then commits if any trigger fires.
    ///
    /// This is what a host calls once a frame, or on a timer. It is the only automatic path to a
    /// commit; nothing commits behind a caller's back.
    ///
    /// # Errors
    /// [`SessionError`] if the journal sink or the commit refuses.
    pub fn poll(&mut self) -> Result<Option<CommitOutcome>, SessionError> {
        let now = self.clock.now();
        if self.scheduler.journal_flush_due(now) {
            self.flush_journal()?;
        }
        let dirty = self.document.dirty_bytes();
        let recorded = self.journal.heap_bytes();
        let Some(trigger) = self.scheduler.due(now, dirty, recorded) else {
            return Ok(None);
        };
        self.commit(trigger).map(Some)
    }

    /// Writes everything not yet handed to the journal sink, and makes it durable.
    ///
    /// Cheap by construction: a record is tens of bytes and the write is sequential, which is what
    /// lets layer 1 run sub-second while layer 3 runs on a schedule.
    ///
    /// # Errors
    /// [`SessionError::Sink`] if the sink refuses. Nothing is marked flushed in that case, so the
    /// next attempt writes the same records — including this journal file's header.
    pub fn flush_journal(&mut self) -> Result<usize, SessionError> {
        let now = self.clock.now();
        self.flush_buffer.clear();
        let count = self.journal.encode_unflushed(&mut self.flush_buffer);
        if count == 0 {
            self.scheduler.note_journal_flush(now);
            return Ok(0);
        }
        let written = self
            .journal_sink
            .append(&self.flush_buffer)
            .and_then(|()| self.journal_sink.flush());
        match written {
            Ok(()) => {
                self.stats.journal_flushes += 1;
                self.scheduler.note_journal_flush(now);
                Ok(count)
            }
            Err(error) => {
                self.journal.retract_flush(count);
                Err(SessionError::Sink(error))
            }
        }
    }

    /// Serialises the dirty parts once, writes the container, and truncates the journal.
    ///
    /// **The journal is not flushed on the way in**, deliberately: the document about to be written
    /// already contains everything the unflushed tail describes, so flushing it would be work whose
    /// only product is bytes this call is about to discard.
    ///
    /// The order is document first, journal second. A crash between them leaves a tail that will be
    /// replayed over work already written, and every operation in this crate is an absolute
    /// assignment, so that replay is idempotent. The other order would leave a window in which the
    /// work is in neither place.
    ///
    /// It does **not** end the undo unit in progress. That is the independence
    /// `SESSION_AND_PERSISTENCE.md` §5 asks for and the reason undo does not jump by however much
    /// happened to be batched.
    ///
    /// # Errors
    /// [`SessionError`] if the document refuses to serialise or a sink refuses a write. On a
    /// document or document-sink failure the journal is left intact, so nothing recorded is lost.
    pub fn commit(&mut self, trigger: CommitTrigger) -> Result<CommitOutcome, SessionError> {
        let now = self.clock.now();
        let operations_committed = self.journal.len();
        let Committed {
            parts_serialised,
            bytes,
        } = self.document.commit()?;
        self.document_sink
            .write(&bytes)
            .map_err(SessionError::Sink)?;

        self.stats.commits += 1;
        self.stats.parts_serialised += parts_serialised;
        self.journal.truncate();
        self.scheduler.note_commit(now);

        let outcome = CommitOutcome {
            trigger,
            parts_serialised,
            bytes_written: bytes.len(),
            operations_committed,
        };
        self.journal_sink.truncate().map_err(SessionError::Sink)?;
        Ok(outcome)
    }

    /// An explicit save: immediate, and never deferred by a gesture.
    ///
    /// # Errors
    /// As [`commit`](Self::commit).
    pub fn save(&mut self) -> Result<CommitOutcome, SessionError> {
        self.commit(CommitTrigger::Explicit)
    }

    /// The application is going to the background.
    ///
    /// **Commits immediately**, because iOS terminates backgrounded applications without warning and
    /// this is the single likeliest way to lose work on the mobile target. It is the one trigger a
    /// gesture cannot defer and the one a host must not forget to call.
    ///
    /// # Errors
    /// As [`commit`](Self::commit).
    pub fn note_backgrounded(&mut self) -> Result<Option<CommitOutcome>, SessionError> {
        self.scheduler.request(CommitTrigger::Backgrounded);
        self.poll_requested()
    }

    /// Something that needs the document consistent is about to happen — an export, a print, a
    /// close.
    ///
    /// # Errors
    /// As [`commit`](Self::commit).
    pub fn note_consistency_point(&mut self) -> Result<Option<CommitOutcome>, SessionError> {
        self.scheduler.request(CommitTrigger::Consistency);
        self.poll_requested()
    }

    /// Commits whatever the scheduler now says is due, without the journal-flush step — the
    /// requested triggers are immediate and a flush would only produce bytes the commit
    /// discards.
    fn poll_requested(&mut self) -> Result<Option<CommitOutcome>, SessionError> {
        let now = self.clock.now();
        let dirty = self.document.dirty_bytes();
        let recorded = self.journal.heap_bytes();
        let Some(trigger) = self.scheduler.due(now, dirty, recorded) else {
            return Ok(None);
        };
        self.commit(trigger).map(Some)
    }

    /// Gives the document back, ending the session.
    ///
    /// Nothing is committed on the way out: a caller that wants the work written calls
    /// [`save`](Self::save) first, and one that is discarding it should not have to.
    #[must_use]
    pub fn into_document(self) -> D {
        self.document
    }
}
