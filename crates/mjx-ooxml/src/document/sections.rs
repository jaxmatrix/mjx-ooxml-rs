//! Sections (`w:sectPr`) — page geometry per section, and where a section break lives.
//!
//! [`SectionSummary`] is a facade-only type: `mjx_docx::SectionSpan`/`SectionProperties` need the
//! part's own [`mjx_ooxml_core::Interner`] to answer `page_size`/`page_margins`, which is exactly
//! the kind of borrowed-view dependency the facade cannot carry across a binding boundary (see
//! [`super::Document`]'s own module doc). [`sections`](super::Document::sections) resolves both
//! while the interner is in scope and hands back owned, `Interner`-free data — the same pattern
//! [`crate::deck::effective`] and this module's own [`super::effective`] sibling use for the
//! PresentationML/WordprocessingML effective-property ladders.

use mjx_docx::DocxError;
use mjx_ooxml_core::FromXmlError;

use crate::error::Error;

use super::BlockPath;

/// One section of the document: the paragraphs it governs, and its own page geometry — `None` for
/// either measure if the section's `w:sectPr` carries no `w:pgSz`/`w:pgMar` at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionSummary {
    /// The first paragraph index this section governs.
    pub first_paragraph: u32,
    /// The last paragraph index this section governs, inclusive — `None` if it governs no paragraph
    /// at all.
    pub last_paragraph: Option<u32>,
    /// This section's page extent and orientation (`w:pgSz`), if it states one.
    pub page_size: Option<mjx_docx::PageSize>,
    /// This section's page margins (`w:pgMar`), if it states one.
    pub page_margins: Option<mjx_docx::PageMargins>,
}

/// Which `w:sectPr` a section-editing method addresses — the facade's own `u32`-addressed mirror of
/// [`mjx_docx::SectionLocation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SectionLocation {
    /// The `w:sectPr` inside the paragraph at this [`BlockPath`]'s own `w:pPr` — ending the section
    /// at that paragraph.
    Paragraph(BlockPath),
    /// The body-level `w:sectPr` — the document's last section.
    Body,
}

impl SectionLocation {
    pub(crate) fn to_model(&self) -> mjx_docx::SectionLocation {
        match self {
            Self::Paragraph(path) => mjx_docx::SectionLocation::Paragraph(path.to_model()),
            Self::Body => mjx_docx::SectionLocation::Body,
        }
    }
}

impl From<BlockPath> for SectionLocation {
    fn from(path: BlockPath) -> Self {
        Self::Paragraph(path)
    }
}

/// Converts one `AttributeError` (from a `page_size`/`page_margins` read) into the `DocxError` this
/// module's own closures report through.
fn attr(error: mjx_ooxml_core::AttributeError) -> DocxError {
    DocxError::from(FromXmlError::from(error))
}

impl super::Document {
    /// How many sections the document has (`n` paragraph-level `w:sectPr`s plus, when the body
    /// itself carries one, the trailing body-level section — see [`mjx_docx::SectionSpan`]'s own
    /// doc comment for why "ends" and not "starts" is the correct word for what a `w:sectPr`
    /// does).
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body.
    pub fn section_count(&mut self) -> Result<u32, Error> {
        Ok(crate::index::count(self.sections()?.len()))
    }

    /// Every section this document has, in document order, with its own resolved page geometry.
    ///
    /// # Errors
    /// As [`section_count`](Self::section_count).
    pub fn sections(&mut self) -> Result<Vec<SectionSummary>, Error> {
        let spans = self.document.sections(
            |spans, interner| -> Result<Vec<SectionSummary>, DocxError> {
                spans
                    .iter()
                    .map(|span| {
                        let (page_size, page_margins) = match &span.properties {
                            Some(properties) => (
                                properties.page_size(interner).map_err(attr)?,
                                properties.page_margins(interner).map_err(attr)?,
                            ),
                            None => (None, None),
                        };
                        Ok(SectionSummary {
                            first_paragraph: crate::index::count(span.first_paragraph),
                            last_paragraph: span.last_paragraph.map(crate::index::count),
                            page_size,
                            page_margins,
                        })
                    })
                    .collect()
            },
        )??;
        Ok(spans)
    }

    /// Sets (or, given `None`, removes) the page size and orientation of the `w:sectPr` at
    /// `location`, creating an empty `w:sectPr` first if `location` does not already carry one.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if
    /// [`SectionLocation::Paragraph`] does not address a paragraph.
    pub fn set_section_page_size(
        &mut self,
        location: SectionLocation,
        size: Option<mjx_docx::PageSize>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .edit_section_properties(location.to_model(), |properties, interner| {
                properties.set_page_size(interner, size);
            })?)
    }

    /// As [`set_section_page_size`](Self::set_section_page_size), for page margins (`w:pgMar`).
    ///
    /// # Errors
    /// As [`set_section_page_size`](Self::set_section_page_size).
    pub fn set_section_page_margins(
        &mut self,
        location: SectionLocation,
        margins: Option<mjx_docx::PageMargins>,
    ) -> Result<(), Error> {
        Ok(self
            .document
            .edit_section_properties(location.to_model(), |properties, interner| {
                properties.set_page_margins(interner, margins);
            })?)
    }

    /// Removes the `w:sectPr` at `location`, if it carries one (a no-op otherwise) — the section's
    /// former range joins whatever section follows it.
    ///
    /// # Errors
    /// As [`set_section_page_size`](Self::set_section_page_size).
    pub fn remove_section_properties(&mut self, location: SectionLocation) -> Result<(), Error> {
        Ok(self
            .document
            .remove_section_properties(location.to_model())?)
    }
}
