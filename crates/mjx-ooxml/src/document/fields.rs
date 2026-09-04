//! Fields (`w:fldSimple`/`w:fldChar`) — reading the fields a paragraph holds, and editing a field's
//! instruction or cached result text in place. A field's own nested address (`FieldPath` in
//! `mjx_docx`) is a plain `&[u32]` here rather than a fourth opaque path type: unlike a paragraph or
//! a run, a field is never the *subject* of a paragraph/run-shaped address elsewhere on this facade,
//! so the extra type would buy nothing a slice does not already give a binding.

use crate::error::Error;
use crate::index::index;

use super::BlockPath;

impl super::Document {
    /// Every field the paragraph at `paragraph` holds, at its own top level and (recursively) nested
    /// inside one of those, in document order.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `paragraph` does
    /// not address one, or [`ErrorCode::MalformedDocument`](crate::ErrorCode::MalformedDocument) if
    /// a `w:fldChar` marker sequence in that paragraph does not balance.
    pub fn fields(
        &mut self,
        paragraph: impl Into<BlockPath>,
    ) -> Result<Vec<mjx_docx::Field>, Error> {
        Ok(self.document.fields(paragraph.into().to_model())?)
    }

    /// Sets the field at `field` (within the paragraph at `paragraph`)'s own instruction, leaving
    /// its cached result — and every other field, and every other part — byte-identical. `field` is
    /// the sequence of indices from [`fields`](Self::fields)'s own top level down to the target
    /// field: `&[0]` for the paragraph's first field, `&[0, 1]` for that field's second nested one.
    ///
    /// # Errors
    /// As [`fields`](Self::fields), plus
    /// [`ErrorCode::NotFound`](crate::ErrorCode::NotFound) if `field` does not address a field, or
    /// [`ErrorCode::StructureConflict`](crate::ErrorCode::StructureConflict) if the instruction zone
    /// itself holds a nested field.
    pub fn set_field_instruction(
        &mut self,
        paragraph: impl Into<BlockPath>,
        field: &[u32],
        text: &str,
    ) -> Result<(), Error> {
        let field_indices: Vec<usize> = field.iter().copied().map(index).collect();
        Ok(self
            .document
            .set_field_instruction(paragraph.into().to_model(), field_indices, text)?)
    }

    /// Sets the field at `field`'s own cached result, leaving its instruction — and every other
    /// field, and every other part — byte-identical. See
    /// [`set_field_instruction`](Self::set_field_instruction) for how `field` addresses one.
    ///
    /// # Errors
    /// As [`set_field_instruction`](Self::set_field_instruction), plus
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the field's complex form
    /// carries no `separate` marker.
    pub fn set_field_cached_result_text(
        &mut self,
        paragraph: impl Into<BlockPath>,
        field: &[u32],
        text: &str,
    ) -> Result<(), Error> {
        let field_indices: Vec<usize> = field.iter().copied().map(index).collect();
        Ok(self.document.set_field_cached_result_text(
            paragraph.into().to_model(),
            field_indices,
            text,
        )?)
    }
}
