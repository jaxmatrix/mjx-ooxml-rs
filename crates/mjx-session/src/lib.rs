//! `mjx-session` — the resident document, the operation journal, and batched commit (MJXOFF-167).
//!
//! # What this crate is for
//!
//! The rest of the workspace is a batch library and holds nothing between calls.
//! `crates/mjx-xlsx/docs/guide/large_workbooks.md` states it plainly: *"A worksheet is cheap to hold
//! and expensive to open, and this library holds nothing between calls… every call that reaches into
//! a sheet's cells parses that sheet's part again."* That is exactly right for a program that opens
//! a file, changes it and writes it out, and it is fatal for an editor, where a keystroke must reach
//! a repainted screen in 30 ms.
//!
//! A **session** is the fix: it owns a parsed, mutable document and keeps it, records every mutation
//! as an operation, and writes bytes on a schedule rather than on every operation.
//!
//! # The three layers, and the one that is batched
//!
//! [`docs/client-platform/SESSION_AND_PERSISTENCE.md`](https://github.com/jaxmatrix/mjx-ooxml-rs)
//! §1 names them, and conflating them is the predictable way this goes wrong:
//!
//! | Layer | When | Why |
//! |---|---|---|
//! | **1 · Record** — append to the journal | immediately, synchronously | it is undo/redo and crash recovery; an operation not recorded the instant it happens can be lost |
//! | **2 · Apply** — mutate the model | immediately | the keystroke-to-repaint budget is 30 ms, and a pending queue means the user types and nothing appears |
//! | **3 · Commit** — serialise dirty parts and write bytes | **batched, on a schedule** | this is the expensive layer, and the only one worth deferring |
//!
//! Batching layer 1 loses work. Batching layer 2 makes the editor feel broken. Batching layer 3 is
//! pure win: serialising a worksheet part costs orders of magnitude more than mutating one cell in
//! the model.
//!
//! # Coalescing is the saving, not the timer
//!
//! A timer alone still serialises a part once per interval whether one character changed or ten
//! thousand. The saving is that the commit **walks a dirty set and never replays the journal**:
//! twenty keystrokes into one run are twenty journal entries and *one* dirty part, and a drag's
//! hundred pointer moves are one transform. That granularity is not new machinery — parts are
//! already the unit of copy-on-write in `mjx-opc`, and untouched parts already re-emit verbatim.
//!
//! **The gates in this crate count serialisations**, because *"a commit produces a valid document"*
//! is green for a session that commits on every operation and has no batching in it at all —
//! the check passes precisely when the feature is absent. `tests/batching.rs` runs the same twenty
//! edits under [`CommitPolicy::interactive`] and under [`CommitPolicy::per_operation`] and
//! compares: one serialisation against twenty.
//!
//! # One refinement to the documented copy-on-write rule
//!
//! `CLAUDE.md` stated the contract as *on first edit, serialize from the model and drop raw bytes.*
//! Under batched commit that is two separate moments, and MJXOFF-167 amended the rule to say so:
//!
//! > On first edit, **drop the raw bytes and mark the part dirty** — the model is now authoritative.
//! > **Serialise at commit**, once, however many edits have accumulated.
//!
//! The round-trip guarantee is untouched: an untouched part is never marked dirty and still re-emits
//! byte for byte. Only the *timing* moves, and `mjx_opc::Package::settle_edited_parts` is where it
//! moved to.
//!
//! # Where this crate sits, and what it may not reach
//!
//! Rank **3.5** — above the three format crates, below the facade. That rank is what lets it name a
//! `Presentation`, a `Document` and a `Workbook` at all.
//!
//! But a session that could *only* hold a `.pptx` would put editing back on the wrong side of the
//! seam `mjx-layout` (1.6) exists to hold, so the crate is built the other way up: everything under
//! `src/` outside `src/ooxml/` is generic over [`ResidentDocument`] and names no format crate. It is
//! written in `mjx-layout`'s address vocabulary — [`mjx_layout::SourceRef`], a part number and a path
//! of small integers — which is the same vocabulary a hit test answers in and the same one a
//! non-OOXML box model produces. `crates/mjx-session/tests/the_seam_holds.rs` holds that as a source
//! gate *and* drives the whole journal, scheduler and undo machinery from a document that has never
//! heard of a package; `cargo test -p mjx-session --no-default-features` builds the crate with the
//! format crates absent entirely.
//!
//! # What it does not do
//!
//! Selection, hit-testing, IME and the clipboard are a later unit. The collaboration transport is
//! not in scope either — but the journal is its natural seam, which is why operations are
//! self-describing and address-based here rather than later.
//!
//! **And one thing from the specification is deliberately absent, rather than quietly missing.**
//! `SESSION_AND_PERSISTENCE.md` §3 asks for the commit to run *off the frame path*: on a worker
//! thread natively, and **chunked across frames with a time budget on single-threaded `wasm32`**.
//! The first half is here and is checked — nothing in this crate spawns a thread, because a library
//! that did would be one `wasm32` could not use, but a whole [`Session`] moves across a thread
//! boundary and `tests/batching.rs` actually sends one. The second half is **not implemented**: a
//! commit is one call, and chunking it would mean a resumable [`ResidentDocument::commit`] — a
//! serialisation that can stop halfway and be asked to continue, which `mjx_opc::Package::save` is
//! not and which is a change to the fidelity writer rather than to this crate. The trait method is
//! where it would go, and it is named here so that the next reader finds a stated gap rather than an
//! assumption.
//!
//! # Example
//!
//! ```
//! use mjx_layout::{PartId, SourcePath, SourceRef};
//! use mjx_session::{
//!     Applied, Committed, Invalidation, ManualClock, MemoryDocument, MemoryJournal, Operation,
//!     ResidentDocument, Session, SessionError, Value,
//! };
//!
//! // The smallest possible residency: one string.
//! #[derive(Default)]
//! struct OneString(String);
//!
//! impl ResidentDocument for OneString {
//!     fn apply(&mut self, operation: &Operation) -> Result<Applied, SessionError> {
//!         let was = Value::text(self.0.as_str());
//!         if let mjx_session::OperationKind::SetValue(Value::Text(text)) = operation.kind() {
//!             self.0 = text.to_string();
//!         }
//!         Ok(Applied {
//!             inverse: Operation::set_value(operation.address().clone(), was),
//!             invalidation: Invalidation::at(operation.address()),
//!         })
//!     }
//!     fn dirty_bytes(&self) -> usize {
//!         self.0.len()
//!     }
//!     fn commit(&mut self) -> Result<Committed, SessionError> {
//!         Ok(Committed { parts_serialised: 1, bytes: self.0.clone().into_bytes() })
//!     }
//! }
//!
//! let clock = ManualClock::new();
//! let committed = MemoryDocument::new();
//! let mut session = Session::new(
//!     OneString::default(),
//!     clock,
//!     MemoryJournal::new(),
//!     committed.clone(),
//! );
//! let address = SourceRef::node(PartId::PRIMARY, SourcePath::root());
//!
//! for letter in "hello".chars() {
//!     session.edit(Operation::set_value(address.clone(), Value::text(letter.to_string())))?;
//!     session.clock().advance(20);
//!     assert!(session.poll()?.is_none(), "nothing commits mid-burst");
//! }
//!
//! // The user pauses; the idle trigger fires, and five edits cost one serialisation.
//! session.clock().advance(2_000);
//! let outcome = session.poll()?.expect("an idle commit");
//! assert_eq!(outcome.parts_serialised, 1);
//! assert_eq!(session.stats().operations_recorded, 5);
//! assert_eq!(committed.contents().as_deref(), Some(&b"o"[..]));
//! # Ok::<(), mjx_session::SessionError>(())
//! ```

pub mod document;
pub mod error;
pub mod journal;
pub mod operation;
pub mod recover;
pub mod schedule;
pub mod session;
pub mod sink;
pub mod undo;

#[cfg(feature = "ooxml")]
pub mod ooxml;

pub use document::{Applied, Committed, Invalidation, ResidentDocument};
pub use error::{SessionError, SinkError};
pub use journal::{DecodedJournal, EntryKind, Journal, JournalEntry, OperationId};
pub use operation::{Operation, OperationKind, Value};
pub use recover::Recovery;
pub use schedule::{Clock, CommitPolicy, CommitScheduler, CommitTrigger, ManualClock, Timestamp};
pub use session::{CommitOutcome, Session, SessionStats};
pub use sink::{DocumentSink, JournalSink, MemoryDocument, MemoryJournal};
pub use undo::{UndoPolicy, UndoStep, UndoUnit, UndoUnitId, UndoUnits};
