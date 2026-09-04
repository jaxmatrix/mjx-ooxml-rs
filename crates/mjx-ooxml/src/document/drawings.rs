//! Drawings — adding an inline picture, and removing any drawing (picture or otherwise) by its own
//! `wp:docPr` id.

use crate::error::Error;

use super::BlockPath;

impl super::Document {
    /// Adds an inline picture: a new media part holding `image_bytes`, its relationship, and a
    /// `w:drawing/wp:inline` placement wrapping a `pic:pic` that references it — appended as a new
    /// run at the end of the paragraph at `paragraph`. `content_type` is the image part's own MIME
    /// type (e.g. `"image/png"`); `extension` is its own file extension (e.g. `"png"`, no dot);
    /// `width_emu`/`height_emu` are the picture's displayed size in EMU. Returns the drawing's own
    /// `wp:docPr` id, which [`remove_drawing`](Self::remove_drawing) takes to remove it again.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body, or [`ErrorCode::IndexOutOfRange`](crate::ErrorCode::IndexOutOfRange) if `paragraph`
    /// does not address one.
    #[allow(clippy::too_many_arguments)]
    pub fn add_inline_picture(
        &mut self,
        paragraph: impl Into<BlockPath>,
        image_bytes: Vec<u8>,
        content_type: &str,
        extension: &str,
        width_emu: i64,
        height_emu: i64,
        name: &str,
    ) -> Result<u32, Error> {
        Ok(self.document.add_inline_picture(
            paragraph.into().to_model(),
            image_bytes,
            content_type,
            extension,
            width_emu,
            height_emu,
            name,
        )?)
    }

    /// Removes the drawing whose `wp:docPr@id` is `doc_pr_id` — the run holding it, and, when it is
    /// a picture, the image part and relationship it alone referenced. Returns whether one was found
    /// and removed; not finding it is a no-op.
    ///
    /// # Errors
    /// [`ErrorCode::NothingToRead`](crate::ErrorCode::NothingToRead) if the document declares no
    /// body.
    pub fn remove_drawing(&mut self, doc_pr_id: u32) -> Result<bool, Error> {
        Ok(self.document.remove_drawing(doc_pr_id)?)
    }
}
