//! The slide list: adding a slide on a layout, and removing one with the parts only it referenced.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::{count, index};
use crate::{Deck, Error, ShapeBounds};

impl Deck {
    /// Adds a new empty slide at the end of the deck, wired to the same slide layout as slide 0 — or,
    /// on a deck with no slides yet, to the deck's first layout — and returns its index. The new slide
    /// is a blank shape tree; add content with `add_text_box` or use `add_slide_with_text`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_slide`](mjx_pptx::Presentation::add_slide).
    pub fn add_slide(&mut self) -> Result<u32, Error> {
        Ok(count(self.presentation.add_slide()?))
    }

    /// Adds a new slide at the end of the deck built on layout `layout_idx`, carrying a copy of every
    /// placeholder that layout declares, and returns the slide's index.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_slide_from_layout`](mjx_pptx::Presentation::add_slide_from_layout).
    pub fn add_slide_from_layout(&mut self, layout_idx: u32) -> Result<u32, Error> {
        Ok(count(
            self.presentation.add_slide_from_layout(index(layout_idx))?,
        ))
    }

    /// Removes slide `slide_idx` from the deck, unwiring it completely: the `p:sldId` naming it, the
    /// presentation's relationship to it, the slide part, its own `.rels`, and its content-type
    /// `Override`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_slide`](mjx_pptx::Presentation::remove_slide).
    pub fn remove_slide(&mut self, slide_idx: u32) -> Result<(), Error> {
        Ok(self.presentation.remove_slide(index(slide_idx))?)
    }

    /// Adds a new slide (via `add_slide`) carrying a single text box with `text` laid out at `bounds`,
    /// and returns the new slide's index.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_slide_with_text`](mjx_pptx::Presentation::add_slide_with_text).
    pub fn add_slide_with_text(&mut self, text: &str, bounds: ShapeBounds) -> Result<u32, Error> {
        Ok(count(self.presentation.add_slide_with_text(text, bounds)?))
    }
}
