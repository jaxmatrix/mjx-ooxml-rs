//! [`Document`] — the binding-shaped Word surface.
//!
//! A `Document` is an [`mjx_docx::Document`] with its Rust ergonomics traded for portability,
//! mirroring [`crate::Deck`]'s own relationship to [`mjx_pptx::Presentation`] exactly:
//!
//! | `mjx_docx::Document`     | `mjx_ooxml::Document`  | why |
//! |---------------------------|-------------------------|-----|
//! | `impl Into<BlockPath>`    | [`BlockPath`](crate::BlockPath) | a generic parameter has no foreign representation |
//! | `impl Into<RunPath>`      | [`RunPath`](crate::RunPath) | likewise |
//! | `usize`                   | `u32`                   | one width on every target, host-independent |
//! | `impl FnOnce(&T, &Interner) -> R` | a concrete return type | neither PyO3 nor wasm-bindgen can accept a Rust closure argument |
//! | `Result<_, DocxError>`    | [`Result<_, Error>`](crate::Error) | thirty-five variants collapse to eleven codes |
//!
//! # Curating a document many times this surface's size
//!
//! `mjx_docx::Document` carries well over a hundred public methods across fourteen files (styles,
//! numbering, sections, headers/footers, tables, fields, hyperlinks, comments, footnotes, endnotes,
//! revisions, drawings, content controls, mail merge, web settings, the font table, recipients,
//! bookmarks, move ranges, custom XML data binding, `altChunk`, the glossary document). This facade
//! selects the subset a Word caller doing ordinary document authoring and editing needs — the same
//! `open`/`blank`/`save`/paragraph-and-run/effective-properties/styles/numbering/sections/headers-
//! footers/tables/fields/hyperlinks/comments-and-footnotes/revisions/drawings/content-controls list
//! this crate's own contributing ticket names — and leaves the rest reachable through
//! [`Document::document_mut`], exactly as [`crate::Deck::presentation_mut`] is the escape hatch for
//! sixteen `Presentation` methods no binding can carry.
//!
//! **Left to `mjx_docx::Document` directly, and why:**
//! - **Equations** (`mjx_omml::Math`/`MathParagraph`) — not curated onto the facade, matching the
//!   PowerPoint facade's own precedent of leaving `mjx-omml`/deep DrawingML trees unexposed: an
//!   equation's tree has no flat, binding-friendly shape the way a run's text or a cell's text does.
//! - **Mail merge, web settings, the font table, recipients** (`document_settings`/
//!   `edit_document_settings`, `web_settings`/`edit_web_settings`, `font_table`/`edit_font_table`,
//!   `recipients`/`edit_recipients`) — document-wide metadata clusters a typical authoring/editing
//!   caller does not touch; each stays reachable, in full, through [`Document::document_mut`].
//! - **Bookmarks, move ranges, custom-XML ranges, data binding, `altChunk`, the glossary document**
//!   (`add_bookmark`/`remove_bookmark`/`resolve_bookmark`, `move_from_range`/`move_to_range`,
//!   `custom_xml_*_range`, `resolve_data_binding`, `alt_chunk_*`, `glossary_document`,
//!   `custom_xml_parts`) — none of these appear in this ticket's own capability list; curating them
//!   in would have grown the surface past what the ticket asked for. Reachable through
//!   [`Document::document_mut`].
//! - **Legacy form fields** (`form_field`/`edit_form_field`/`insert_form_field`) — a narrower, older
//!   sibling of content controls; `w:ffData` stays reachable through [`Document::document_mut`]
//!   rather than doubling the content-control surface for a legacy feature this ticket's list does
//!   not name.
//! - **Content controls' own metadata** (tag, alias, lock state, placeholder, data binding) —
//!   `mjx_docx::Document` itself exposes no closure-free, top-level enumerator for
//!   `w:sdt` the way it does for tables, comments and footnotes (no `content_controls`/
//!   `edit_content_controls` pair exists to wrap), only the raw `Body::content()` a caller already
//!   holding the model can filter for `BlockContent::StructuredDocumentTag`. Building one from
//!   scratch here would be reimplementing, not wrapping. What *is* already reachable, and needs no
//!   new method: a content control's own wrapped text, because MJXOFF-138 made
//!   [`Document::paragraph_text`]/[`Document::run_text`]/[`Document::set_run_text`] recurse straight
//!   through a content-control (or custom-XML) wrapper the same way they already recurse through a
//!   hyperlink — so reading or editing the text inside a content control needs no control-specific
//!   method at all. The control's own properties stay reachable through
//!   [`Document::document_mut`].
//! - **The general escape-hatch closures** (`style_sheet`/`edit_style_sheet` beyond the two readers
//!   below, `numbering`/`edit_numbering` beyond attach/detach, `edit_cell`/`edit_table` beyond the
//!   narrower setters, `header_footer`/`edit_header_footer` beyond the text pair,
//!   `edit_section_properties` beyond page size, `comments`/`footnotes`/`endnotes` beyond the typed
//!   summaries) — every one of these is unbindable **as a closure parameter**, but each already has a
//!   narrower, concrete-typed cover method on this facade (see the `document` submodules) built by
//!   calling the closure-taking method *internally*. What a binding cannot cross is a foreign
//!   function boundary carrying a closure; nothing stops this facade from using one on the Rust side
//!   of that boundary and handing back owned, concrete data.
//!
//! # Addressing
//!
//! A [`BlockPath`](crate::BlockPath) says which paragraph (`BlockPath::from(1)` for the second
//! top-level paragraph); a [`RunPath`](crate::RunPath) says which run within it. Both convert from a
//! bare index, so `1.into()` is the whole ceremony for the common case — see that module's own doc
//! comment.
//!
//! # One document, one thread
//!
//! Same discipline as [`crate::Deck`]: almost every method takes `&mut self` because reading a part
//! materializes it, nothing here hands back a view into the document, and nothing here takes a
//! callback, so a second borrow can never be live. Share a document between threads by moving it, not
//! by aliasing it.

use mjx_docx::PageSize;

use crate::error::{Error, ErrorCode};
use crate::format::{format_of, Format};

mod comments;
mod drawings;
mod effective;
mod fields;
mod headers;
mod hyperlinks;
mod notes;
mod numbering;
mod paths;
mod sections;
mod styles;
mod tables;
mod text;

pub use comments::CommentSummary;
pub use notes::NoteSummary;
pub use paths::{BlockPath, RunPath};
pub use sections::{SectionLocation, SectionSummary};

/// An open Word document.
///
/// ```no_run
/// use mjx_ooxml::{Document, PageSize};
///
/// # fn main() -> Result<(), mjx_ooxml::Error> {
/// let mut document = Document::blank(PageSize::a4())?;
/// document.append_paragraph()?;
/// document.append_run(0, "Hello, document.")?;
/// let bytes = document.save()?;
/// # let _ = bytes;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct Document {
    document: mjx_docx::Document,
    format: Format,
}

impl Document {
    /// A new document with nothing in it beyond one empty paragraph and a body-level `w:sectPr`
    /// naming `size` — see [`mjx_docx::Document::blank`]'s own doc comment for exactly which optional
    /// parts a blank document gets.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidArgument`] if `size` is degenerate — zero on either extent, or narrower or
    /// shorter than this crate's fixed one-inch margins leave room for.
    pub fn blank(size: PageSize) -> Result<Self, Error> {
        Ok(Self {
            document: mjx_docx::Document::blank(size)?,
            format: Format::Document,
        })
    }

    /// Opens a document from the bytes of a `.docx`, `.docm`, `.dotx` or `.dotm`.
    ///
    /// The format is [detected](crate::detect_format) from the package before anything is parsed as
    /// WordprocessingML, so a PowerPoint or Excel package is refused by name rather than by a parse
    /// failure, and the package is read exactly once.
    ///
    /// # Errors
    /// - [`ErrorCode::Io`] if the bytes are not a readable ZIP container.
    /// - [`ErrorCode::UnsupportedFormat`] if the package is a PowerPoint or Excel document, or an OPC
    ///   package that is not an Office document at all.
    /// - [`ErrorCode::MalformedDocument`] if it is a Word document whose `word/document.xml` or
    ///   relationships are not what the schema requires.
    pub fn open(bytes: &[u8]) -> Result<Self, Error> {
        let package = mjx_pptx::Package::open(bytes)?;
        let format = format_of(&package)?;
        if format.family() != crate::FormatFamily::WordProcessing {
            return Err(Error::new(
                ErrorCode::UnsupportedFormat,
                format!(
                    "this build opens WordprocessingML only; these bytes are {:?} (.{}), which is not a Word document",
                    format,
                    format.conventional_extension()
                ),
            ));
        }
        Ok(Self {
            document: mjx_docx::Document::from_package(package)?,
            format,
        })
    }

    /// Which WordprocessingML format this document was opened as — a document, a template,
    /// macro-enabled or not.
    ///
    /// It survives editing and saving: this library never rewrites the main part's content type, so
    /// a `.dotx` opened, edited and saved is still a `.dotx`. A document built with
    /// [`blank`](Self::blank) is a [`Format::Document`].
    #[must_use]
    pub fn format(&self) -> Format {
        self.format
    }

    /// Serializes the document back to `.docx` container bytes, **after** checking its packaging
    /// invariants — the same "the check is not optional" guarantee [`crate::Deck::save`] documents
    /// for PowerPoint.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidDocument`] if a packaging invariant is broken, or [`ErrorCode::Io`] if the
    /// ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, Error> {
        Ok(self.document.save()?)
    }

    /// Serializes the document **without** checking its packaging invariants — the deliberate
    /// override for [`save`](Self::save), for the same reasons [`crate::Deck::save_unchecked`]
    /// documents.
    ///
    /// # Errors
    /// [`ErrorCode::Io`] if the ZIP writer fails.
    pub fn save_unchecked(&self) -> Result<Vec<u8>, Error> {
        Ok(self.document.save_unchecked()?)
    }

    /// Checks the packaging invariants [`save`](Self::save) enforces, without writing anything.
    ///
    /// # Errors
    /// [`ErrorCode::InvalidDocument`], carrying the first invariant broken as its
    /// [`source`](std::error::Error::source).
    pub fn validate(&self) -> Result<(), Error> {
        Ok(self.document.validate()?)
    }

    /// The underlying [`mjx_docx::Document`], for reading — the Rust-only door to everything this
    /// facade's own module doc lists as deliberately left out.
    #[must_use]
    pub fn document(&self) -> &mjx_docx::Document {
        &self.document
    }

    /// The underlying [`mjx_docx::Document`], for editing. See [`document`](Self::document).
    pub fn document_mut(&mut self) -> &mut mjx_docx::Document {
        &mut self.document
    }

    /// Consumes the facade and returns the [`mjx_docx::Document`] inside it.
    #[must_use]
    pub fn into_document(self) -> mjx_docx::Document {
        self.document
    }

    /// The document's conformance class (`w:document/@conformance`) — `Strict` or `Transitional`, or
    /// `None` if the attribute is absent.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if `@conformance` is present but not a value this build
    /// recognizes.
    pub fn conformance(
        &mut self,
    ) -> Result<Option<mjx_ooxml_types::shared::ConformanceClass>, Error> {
        Ok(self.document.conformance()?)
    }

    /// Sets (or, given `None`, removes) `w:document/@conformance`.
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`] if the main document part cannot be read.
    pub fn set_conformance(
        &mut self,
        value: Option<mjx_ooxml_types::shared::ConformanceClass>,
    ) -> Result<(), Error> {
        Ok(self.document.set_conformance(value)?)
    }
}

impl From<mjx_docx::Document> for Document {
    /// Wraps a document this crate did not open — the inverse of
    /// [`into_document`](Document::into_document). Its [`format`](Document::format) reports
    /// [`Format::Document`], since an [`mjx_docx::Document`] carries no record of the content type
    /// it was opened under.
    fn from(document: mjx_docx::Document) -> Self {
        Self {
            document,
            format: Format::Document,
        }
    }
}
