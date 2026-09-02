//! Hyperlinks on a run, on a text range, and on a shape as a whole.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::index::index;
use crate::{Deck, Error, Hyperlink, ShapePath, Surface};

impl Deck {
    /// The click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` on `surface`,
    /// resolved to a `Hyperlink` (a URL or a slide index), or `None` if the run has no hyperlink — or
    /// one this build does not model (a mouse-over action, a show jump). Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::run_hyperlink`](mjx_pptx::Presentation::run_hyperlink).
    pub fn run_hyperlink(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<Option<Hyperlink>, Error> {
        Ok(self.presentation.run_hyperlink(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(run_idx),
        )?)
    }

    /// Sets the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx` to
    /// `link`, adding its relationship. If the run already linked somewhere, that relationship is
    /// removed once nothing else in the part still names it.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_run_hyperlink`](mjx_pptx::Presentation::set_run_hyperlink).
    pub fn set_run_hyperlink(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        run_idx: u32,
        link: &Hyperlink,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_run_hyperlink(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(run_idx),
            link,
        )?)
    }

    /// Removes the click hyperlink on run `run_idx` of paragraph `para_idx` in shape `shape_idx`, and
    /// the relationship it named once nothing else in the part still references it. A no-op if the run
    /// has no hyperlink.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_run_hyperlink`](mjx_pptx::Presentation::clear_run_hyperlink).
    pub fn clear_run_hyperlink(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        run_idx: u32,
    ) -> Result<(), Error> {
        Ok(self.presentation.clear_run_hyperlink(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(run_idx),
        )?)
    }

    /// Sets the click hyperlink over a **scalar range** of paragraph `para_idx` in shape `shape_idx`,
    /// splitting runs at the boundaries so exactly the selected text is linked (as
    /// `set_text_range_properties` does). One relationship is added and shared by every run in the
    /// range. An empty range links nothing.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_text_range_hyperlink`](mjx_pptx::Presentation::set_text_range_hyperlink).
    pub fn set_text_range_hyperlink(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        para_idx: u32,
        range: core::ops::Range<u32>,
        link: &Hyperlink,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_text_range_hyperlink(
            surface.to_model(),
            shape_idx.to_model(),
            index(para_idx),
            index(range.start)..index(range.end),
            link,
        )?)
    }

    /// The click hyperlink on shape `shape_idx` itself (`p:cNvPr > a:hlinkClick`), resolved to a
    /// `Hyperlink`, or `None` if the shape has no hyperlink (or one this build does not model). Reading
    /// does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_hyperlink`](mjx_pptx::Presentation::shape_hyperlink).
    pub fn shape_hyperlink(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Hyperlink>, Error> {
        Ok(self
            .presentation
            .shape_hyperlink(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets the click hyperlink on shape `shape_idx` itself to `link`, adding its relationship and
    /// removing the one any previous link named once unreferenced.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_hyperlink`](mjx_pptx::Presentation::set_shape_hyperlink).
    pub fn set_shape_hyperlink(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        link: &Hyperlink,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_hyperlink(surface.to_model(), shape_idx.to_model(), link)?)
    }

    /// Removes the click hyperlink on shape `shape_idx` itself, and the relationship it named once
    /// unreferenced. A no-op if the shape has no hyperlink.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_shape_hyperlink`](mjx_pptx::Presentation::clear_shape_hyperlink).
    pub fn clear_shape_hyperlink(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .clear_shape_hyperlink(surface.to_model(), shape_idx.to_model())?)
    }
}
