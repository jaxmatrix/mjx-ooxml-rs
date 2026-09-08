//! [`WordSession`] — a `.docx` held open.

use mjx_docx::{BlockPath, Document, RunPath};
use mjx_layout::SourceRef;

use crate::document::{Applied, Committed, Invalidation, ResidentDocument};
use crate::error::SessionError;
use crate::ooxml::DirtyEstimate;
use crate::operation::{Operation, OperationKind, Value};

/// A Word document a session holds open.
///
/// # The addressing scheme
///
/// | Piece | Meaning |
/// |---|---|
/// | [`PartId::PRIMARY`](mjx_layout::PartId::PRIMARY) | `word/document.xml`, the main story |
/// | path segments `0 .. n-1` | the block path — one index for a top-level paragraph, more to descend through tables |
/// | the last path segment | which run inside that paragraph |
///
/// So `[4, 1]` is the fifth top-level block's second run, and `[2, 0, 3, 0]` addresses a run inside a
/// table. A text operation needs at least two segments.
///
/// Only the main document part is addressed. Headers, footers, footnotes and comments are separate
/// stories and would take part numbers of their own; nothing above this layer asks for them yet, and
/// inventing a numbering that the box model then has to match would be the wrong order.
///
/// # What it can do
///
/// [`Value::Text`] and [`Value::Empty`] set a run's text. [`OperationKind::SetBounds`] is refused: a
/// run in flowed text has no rectangle of its own to set — where it lands is the box model's answer,
/// not the document's.
///
/// A run's edit re-serialises **only the byte range containing that run** on the way out, because
/// `Document::set_run_text` writes back through the fidelity writer; every sibling paragraph keeps
/// its original bytes.
#[derive(Debug)]
pub struct WordSession {
    document: Document,
    dirty: DirtyEstimate,
}

impl WordSession {
    /// Holds `document` open.
    #[must_use]
    pub fn new(document: Document) -> Self {
        Self {
            document,
            dirty: DirtyEstimate::default(),
        }
    }

    /// Opens a document from its container bytes and holds it open.
    ///
    /// # Errors
    /// [`SessionError::Document`] carrying the `DocxError` if the package is unreadable.
    pub fn open(bytes: &[u8]) -> Result<Self, SessionError> {
        Document::open(bytes)
            .map(Self::new)
            .map_err(SessionError::document)
    }

    /// The document, for reading.
    #[must_use]
    pub const fn document(&self) -> &Document {
        &self.document
    }

    /// The document, mutably. Anything done through here is not journalled — see
    /// [`Session::document_mut`](crate::Session::document_mut).
    pub fn document_mut(&mut self) -> &mut Document {
        &mut self.document
    }

    /// Gives the document back.
    #[must_use]
    pub fn into_document(self) -> Document {
        self.document
    }

    /// The paragraph and run an address names.
    fn text_path(address: &SourceRef) -> Result<(BlockPath, RunPath), SessionError> {
        if address.part() != mjx_layout::PartId::PRIMARY {
            return Err(SessionError::no_such_node(format!(
                "part {} — this residency addresses `word/document.xml` alone, which is part 0",
                address.part().number()
            )));
        }
        let segments = address.path().segments();
        if segments.len() < 2 {
            return Err(SessionError::no_such_node(format!(
                "path {segments:?} — a run needs a block path and a run index"
            )));
        }
        let split = segments.len() - 1;
        let block: Vec<usize> = segments[..split].iter().map(|&it| it as usize).collect();
        Ok((
            BlockPath::from(block),
            RunPath::from(segments[split] as usize),
        ))
    }

    fn apply_text(&mut self, address: &SourceRef, text: &str) -> Result<Applied, SessionError> {
        let (block, run) = Self::text_path(address)?;
        let was = self
            .document
            .run_text(&block, &run)
            .map_err(SessionError::document)?;
        self.document
            .set_run_text(&block, &run, text)
            .map_err(SessionError::document)?;
        self.dirty.note(text.len());
        Ok(Applied {
            inverse: Operation::set_value(address.clone(), Value::text(was)),
            // **Reflowing, not reformatting.** A run's text just changed length, and Word's box
            // model is a flow: every page from this one to the end may move. Reporting the narrow
            // kind here would be the one place in this crate where an invalidation understated what
            // it did, and a consumer that trusted it would leave stale pages on screen.
            invalidation: Invalidation::reflowing(address),
        })
    }
}

impl ResidentDocument for WordSession {
    fn apply(&mut self, operation: &Operation) -> Result<Applied, SessionError> {
        match operation.kind() {
            OperationKind::SetValue(Value::Text(text)) => {
                self.apply_text(operation.address(), text)
            }
            OperationKind::SetValue(Value::Empty) => self.apply_text(operation.address(), ""),
            OperationKind::SetValue(other) => Err(SessionError::unsupported(format!(
                "put {other:?} into a run — a Word run holds text and nothing else this vocabulary spells"
            ))),
            OperationKind::SetBounds(_) => Err(SessionError::unsupported(
                "place a run — flowed text has no rectangle of its own",
            )),
        }
    }

    fn dirty_bytes(&self) -> usize {
        self.dirty.bytes()
    }

    fn commit(&mut self) -> Result<Committed, SessionError> {
        let serialised = self.document.settle_dirty_parts().len();
        let bytes = self.document.save().map_err(SessionError::document)?;
        self.dirty.clear();
        Ok(Committed {
            parts_serialised: serialised,
            bytes,
        })
    }
}
