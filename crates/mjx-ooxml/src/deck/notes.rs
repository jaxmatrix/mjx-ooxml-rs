//! Speaker notes: the notes slide's body text.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::index;
use crate::{Deck, Error};

impl Deck {
    /// The speaker notes of slide `slide_idx` — the text of its notes slide's `body` placeholder — or
    /// `None` if the slide has no notes slide (or its notes slide has no body placeholder).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::notes_text`](mjx_pptx::Presentation::notes_text).
    pub fn notes_text(&mut self, slide_idx: u32) -> Result<Option<String>, Error> {
        Ok(self.presentation.notes_text(index(slide_idx))?)
    }

    /// Sets the speaker notes of slide `slide_idx` to `text`, creating the notes slide (and, if the
    /// deck has none, the notes master it follows) on demand.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_notes_text`](mjx_pptx::Presentation::set_notes_text).
    pub fn set_notes_text(&mut self, slide_idx: u32, text: &str) -> Result<(), Error> {
        Ok(self.presentation.set_notes_text(index(slide_idx), text)?)
    }

    /// Removes the speaker notes of slide `slide_idx`: unwires the slide → notes-slide relationship and
    /// removes the notes slide part (with its `.rels` and content-type override). A no-op if the slide
    /// has no notes.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_notes`](mjx_pptx::Presentation::clear_notes).
    pub fn clear_notes(&mut self, slide_idx: u32) -> Result<(), Error> {
        Ok(self.presentation.clear_notes(index(slide_idx))?)
    }
}
