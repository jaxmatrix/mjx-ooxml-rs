//! Styles (`word/styles.xml`) — read-only on this facade: every `styleId` the document defines, and
//! a style's own display name. A caller applying a style to a paragraph or run does so through
//! [`super::Document::set_paragraph_style`]/[`super::Document::set_character_style`]; defining or
//! editing a style itself is not part of this ticket's curated surface (see [`super::Document`]'s
//! own module doc) and stays reachable through [`super::Document::document_mut`]'s
//! [`mjx_docx::Document::edit_style_sheet`].

use mjx_docx::DocxError;
use mjx_ooxml_core::FromXmlError;

use crate::error::Error;

impl super::Document {
    /// Every `styleId` this document's `word/styles.xml` defines, in document order — empty if the
    /// document relates to no `word/styles.xml` at all (a [`blank`](super::Document::blank) document,
    /// among others).
    ///
    /// # Errors
    /// [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if `word/styles.xml` is
    /// related but cannot be read.
    pub fn style_ids(&mut self) -> Result<Vec<String>, Error> {
        let read =
            self.document
                .style_sheet(|sheet, interner| -> Result<Vec<String>, DocxError> {
                    sheet
                        .styles()
                        .map(|style| {
                            Ok(style
                                .style_id(interner)
                                .map_err(FromXmlError::from)?
                                .map(|id| id.into_owned())
                                .unwrap_or_default())
                        })
                        .collect()
                })?;
        Ok(read.transpose()?.unwrap_or_default())
    }

    /// The display name (`w:name`) of the style identified by `style_id`, or `None` if no style has
    /// that id (or that style carries no `w:name`).
    ///
    /// # Errors
    /// As [`style_ids`](Self::style_ids).
    pub fn style_name(&mut self, style_id: &str) -> Result<Option<String>, Error> {
        let read =
            self.document
                .style_sheet(|sheet, interner| -> Result<Option<String>, DocxError> {
                    let Some(style) = sheet.styles().find(|style| {
                        style
                            .style_id(interner)
                            .ok()
                            .flatten()
                            .is_some_and(|id| id == style_id)
                    }) else {
                        return Ok(None);
                    };
                    let Some(name) = style.name() else {
                        return Ok(None);
                    };
                    Ok(Some(
                        name.value(interner)
                            .map_err(FromXmlError::from)?
                            .into_owned(),
                    ))
                })?;
        Ok(read.transpose()?.flatten())
    }
}
