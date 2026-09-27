//! [`ResidentDocument`] — the seam between a session and whatever it is holding open.
//!
//! # Why this trait exists at all
//!
//! `mjx-layout` is ranked 1.6 so that no OOXML type can appear in a `FragmentTree`, and everything
//! above that seam — scene building, painting, hit-testing, selection, export — is written once
//! against fragments instead of three times against `.pptx`, `.docx` and `.xlsx`. A session that
//! could only ever hold a `.pptx` would put the *editing* half of the platform back on the wrong
//! side of that line.
//!
//! So a session is generic over this trait. Everything the crate does around it — recording,
//! coalescing, undo units, the commit schedule, the journal encoding, recovery — names only
//! [`Operation`] and `mjx-layout`'s address vocabulary, and compiles with the format crates absent
//! entirely (`--no-default-features`). The three OOXML residencies in [`crate::ooxml`] are
//! implementations of this trait and nothing more; `tests/the_seam_holds.rs` drives the same
//! machinery from a fourth one that has never heard of a package.
//!
//! # The three layers, and which of them this trait is
//!
//! `SESSION_AND_PERSISTENCE.md` §1 names them: **record** (immediate, into the journal),
//! **apply** (immediate, into the model), **commit** (batched, on a schedule). This trait is layers
//! 2 and 3 and never layer 1 — [`apply`](ResidentDocument::apply) mutates the model and returns
//! immediately, [`commit`](ResidentDocument::commit) is the expensive one, and the journal is the
//! session's own and is never handed to a document.

use mjx_layout::{ChangeKind, ContentChange, PartId, SourcePath, SourceRef};

use crate::error::SessionError;
use crate::operation::Operation;

/// A document a session holds open across edits.
///
/// # What an implementation owes the session
///
/// * [`apply`](Self::apply) mutates **only the in-memory model** and returns an exact inverse. It
///   must not serialise, must not touch a sink, and must not defer: the keystroke-to-repaint budget
///   is 30 ms and a pending queue makes the editor feel broken.
/// * [`commit`](Self::commit) is where serialisation is allowed to happen, and it must serialise
///   each dirty part **once** — not once per operation that touched it. Reporting
///   [`parts_serialised`](Committed::parts_serialised) honestly is what makes coalescing a
///   measurement rather than a claim, so it must be the number of parts the call actually wrote XML
///   for.
/// * [`dirty_bytes`](Self::dirty_bytes) is an estimate the scheduler uses to fire a commit before a
///   bulk edit has to wait for a clock. An estimate is enough; a lie is not.
pub trait ResidentDocument {
    /// Applies `operation` to the model and reports what it dirtied and how to undo it.
    ///
    /// # Errors
    /// [`SessionError`] if the address names nothing editable, if this document cannot express the
    /// operation, or if the model refuses it. The document must be **unchanged** when this returns
    /// an error: the session records nothing for a failed operation, so a model that changed anyway
    /// would be a change no journal describes.
    fn apply(&mut self, operation: &Operation) -> Result<Applied, SessionError>;

    /// Roughly how many bytes of model are waiting to be serialised.
    fn dirty_bytes(&self) -> usize;

    /// Serialises every dirty part once and produces the whole committed container.
    ///
    /// # Errors
    /// [`SessionError`] if the document refuses to serialise. The session then leaves the journal
    /// intact, so nothing recorded is lost.
    fn commit(&mut self) -> Result<Committed, SessionError>;
}

/// What an [`apply`](ResidentDocument::apply) produced.
#[derive(Clone, PartialEq, Debug)]
pub struct Applied {
    /// The operation that puts the document back exactly as it was.
    ///
    /// It is an [`Operation`] like any other, which is what makes undo and redo the same code path:
    /// undo applies the inverse, and *that* apply returns the inverse of the inverse, which is the
    /// redo. Every operation in this crate is an assignment, so the inverse is always expressible.
    pub inverse: Operation,
    /// What the edit dirtied, for the render pipeline.
    pub invalidation: Invalidation,
}

/// What an edit dirtied, in the address vocabulary a fragment tree is keyed by.
///
/// It is emitted at **apply** time and never at commit time
/// (`SESSION_AND_PERSISTENCE.md` §6): layout, scene and paint caches react to the edit in the frame
/// it happened, and the commit is invisible to them. `crate::Session`'s own suite asserts the second
/// half of that — a commit produces no invalidation at all.
///
/// # Why it carries a [`ChangeKind`]
///
/// An address alone says *where*, and a box model needs *what*, because the three kinds invalidate
/// differently and the difference is the whole saving: reformatting a run cannot move anything
/// before it and, in a document that places absolutely, cannot move anything after it either, while
/// inserting or removing content moves every page that follows. A consumer handed only an address
/// has to assume the worst, and assuming the worst is how a keystroke re-lays out a document.
///
/// The residency chooses, because only it knows what its own operation did — MJXOFF-168 added the
/// field and set each of the three: a slide's shape and a spreadsheet cell are placed absolutely and
/// report [`ChangeKind::Reformatted`], and a paragraph's text is flowing content whose length just
/// changed and reports [`ChangeKind::Inserted`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Invalidation {
    part: PartId,
    path: SourcePath,
    kind: ChangeKind,
}

impl Invalidation {
    /// Everything at or below `path` in `part` changed in the way `kind` describes.
    #[must_use]
    pub const fn new(part: PartId, path: SourcePath, kind: ChangeKind) -> Self {
        Self { part, path, kind }
    }

    /// The subtree an address names was **reformatted** — the same content, differently.
    ///
    /// The narrow reading, and the right default: a caller that means more says so with
    /// [`reflowing`](Self::reflowing). Widening an invalidation is always safe and always expensive,
    /// so the wide one is the one that has to be asked for by name.
    #[must_use]
    pub fn at(address: &SourceRef) -> Self {
        Self::new(
            address.part(),
            address.path().clone(),
            ChangeKind::Reformatted,
        )
    }

    /// The subtree an address names **changed length** — everything after it may move.
    #[must_use]
    pub fn reflowing(address: &SourceRef) -> Self {
        Self::new(address.part(), address.path().clone(), ChangeKind::Inserted)
    }

    /// Which part.
    #[must_use]
    pub const fn part(&self) -> PartId {
        self.part
    }

    /// Which subtree of it.
    #[must_use]
    pub const fn path(&self) -> &SourcePath {
        &self.path
    }

    /// What kind of change it was.
    #[must_use]
    pub const fn kind(&self) -> ChangeKind {
        self.kind
    }

    /// The same fact in the vocabulary a box model is told changes in.
    ///
    /// [`mjx_layout::ChangeSet`] is what [`mjx_layout::BoxModel::invalidate`] takes, and this is the
    /// one conversion between the two — written here rather than in every consumer, so that a second
    /// consumer cannot invent a different reading of the same invalidation.
    #[must_use]
    pub fn content_change(&self) -> ContentChange {
        ContentChange {
            source: SourceRef::node(self.part, self.path.clone()),
            kind: self.kind,
        }
    }

    /// Whether `other` names the same part and a subtree inside this one — the test a cache uses to
    /// decide whether one invalidation subsumes another.
    ///
    /// Deliberately blind to [`kind`](Self::kind): subsumption is a question about *addresses*, and
    /// a reformat inside a reflow is still inside it.
    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        self.part == other.part && self.path.contains(&other.path)
    }
}

/// What a [`commit`](ResidentDocument::commit) produced.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Committed {
    /// How many parts this call actually wrote XML for.
    ///
    /// **The number the whole design is measured by.** A session that commits on every operation
    /// reports one here every time and totals twenty over twenty keystrokes; a batched one reports
    /// one, once. "A commit produces a valid document" is green for both, which is why the gates in
    /// this crate count this instead.
    pub parts_serialised: usize,
    /// The whole container.
    pub bytes: Vec<u8>,
}
