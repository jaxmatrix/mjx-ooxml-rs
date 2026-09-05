//! Headers and footers (`w:hdr`/`w:ftr`) — reading a resolved variant's text, and setting or
//! removing one. Structured header/footer content (tables, hyperlinks, multiple paragraphs) is not
//! part of this ticket's curated surface (see [`super::Document`]'s own module doc) and stays
//! reachable through [`super::Document::document_mut`]'s
//! [`mjx_docx::Document::header_footer`]/`edit_header_footer`, once
//! [`mjx_docx::Document::resolve_header`]/`resolve_footer`/`create_header`/`create_footer` names the
//! part.

use mjx_docx::HeaderFooterType;

use crate::error::Error;

use super::SectionLocation;

impl super::Document {
    /// Whether this document's sections use different headers/footers for even and odd pages
    /// (`w:settings/w:evenAndOddHeaders`).
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if `word/settings.xml`
    /// is related but cannot be read.
    pub fn even_and_odd_headers(&mut self) -> Result<bool, Error> {
        Ok(self.document.even_and_odd_headers()?)
    }

    /// The text of the header of variant `kind` that actually applies to `section`'s pages (ECMA-376
    /// Part 1's `titlePg`/`evenAndOddHeaders`/inheritance rules — see
    /// [`mjx_docx::Document::resolve_header`]'s own doc comment) — every paragraph's text, joined by
    /// a newline. `None` if no section from `section` back to the document's first states a
    /// reference of this variant.
    ///
    /// # Errors
    /// [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `section` names no
    /// section, or [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if a
    /// related part cannot be read.
    pub fn header_text(
        &mut self,
        section: u32,
        kind: HeaderFooterType,
    ) -> Result<Option<String>, Error> {
        self.header_footer_text(section, kind, true)
    }

    /// As [`header_text`](Self::header_text), for footers.
    ///
    /// # Errors
    /// See [`header_text`](Self::header_text).
    pub fn footer_text(
        &mut self,
        section: u32,
        kind: HeaderFooterType,
    ) -> Result<Option<String>, Error> {
        self.header_footer_text(section, kind, false)
    }

    fn header_footer_text(
        &mut self,
        section: u32,
        kind: HeaderFooterType,
        is_header: bool,
    ) -> Result<Option<String>, Error> {
        let index = crate::index::index(section);
        let part = if is_header {
            self.document.resolve_header(index, kind)?
        } else {
            self.document.resolve_footer(index, kind)?
        };
        let Some(part) = part else {
            return Ok(None);
        };
        Ok(Some(self.document.header_footer(&part, |content, _| {
            content
                .paragraphs()
                .map(mjx_docx::Paragraph::text)
                .collect::<Vec<_>>()
                .join("\n")
        })?))
    }

    /// Creates (or replaces, per [`mjx_docx::Document::create_header`]'s own "the old reference is
    /// replaced" contract) a new header part of `kind` for the section at `location`, holding one
    /// paragraph of `text`, and wires `w:headerReference` into that section's own `w:sectPr`.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if
    /// [`SectionLocation::Paragraph`] does not address a paragraph.
    pub fn set_header_text(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
        text: &str,
    ) -> Result<(), Error> {
        self.set_header_footer_text(location, kind, text, true)
    }

    /// As [`set_header_text`](Self::set_header_text), for footers.
    ///
    /// # Errors
    /// See [`set_header_text`](Self::set_header_text).
    pub fn set_footer_text(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
        text: &str,
    ) -> Result<(), Error> {
        self.set_header_footer_text(location, kind, text, false)
    }

    fn set_header_footer_text(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
        text: &str,
        is_header: bool,
    ) -> Result<(), Error> {
        let model_location = location.to_model();
        let part = if is_header {
            self.document.create_header(model_location, kind)?
        } else {
            self.document.create_footer(model_location, kind)?
        };
        self.document
            .edit_header_footer(&part, |content, interner| {
                while content.paragraph_count() > 1 {
                    content.remove_paragraph(content.paragraph_count() - 1);
                }
                if content.paragraph_count() == 0 {
                    content.append_paragraph(mjx_docx::Paragraph::new(interner));
                }
                let paragraph = match content.paragraph_mut(0) {
                    Some(paragraph) => paragraph,
                    None => unreachable!("just ensured at least one paragraph above"),
                };
                while paragraph.run_count() > 0 {
                    paragraph.remove_run(paragraph.run_count() - 1);
                }
                paragraph.append_run(mjx_docx::Run::with_text(interner, text));
            })?;
        Ok(())
    }

    /// Removes the section at `location`'s own `kind` header reference, if it states one (a no-op
    /// otherwise), sweeping the now-unreferenced part.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if
    /// [`SectionLocation::Paragraph`] does not address a paragraph.
    pub fn remove_header(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<(), Error> {
        Ok(self.document.remove_header(location.to_model(), kind)?)
    }

    /// As [`remove_header`](Self::remove_header), for footers.
    ///
    /// # Errors
    /// See [`remove_header`](Self::remove_header).
    pub fn remove_footer(
        &mut self,
        location: SectionLocation,
        kind: HeaderFooterType,
    ) -> Result<(), Error> {
        Ok(self.document.remove_footer(location.to_model(), kind)?)
    }
}
