//! Position, size, transform and geometry: where a shape sits and what outline it draws.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::{
    BoundedAdjustment, Deck, Error, Geometry, GuideContext, ShapeBounds, ShapePath, Surface,
    Transform2D,
};

impl Deck {
    /// The position and size of shape `shape_idx` on `surface` **on the slide** — absolute within
    /// `slide_size`, whether the shape is top-level or nested inside groups.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_bounds`](mjx_pptx::Presentation::shape_bounds).
    pub fn shape_bounds(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<ShapeBounds>, Error> {
        Ok(self
            .presentation
            .shape_bounds(surface.to_model(), shape_idx.to_model())?)
    }

    /// Moves and resizes shape `shape_idx` on `surface` to `bounds`, given **on the slide** — the same
    /// absolute space `shape_bounds` answers in. Creates the shape's transform element if it had none,
    /// and marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_bounds`](mjx_pptx::Presentation::set_shape_bounds).
    pub fn set_shape_bounds(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        bounds: ShapeBounds,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_bounds(surface.to_model(), shape_idx.to_model(), bounds)?)
    }

    /// The **explicit** transform of shape `shape_idx` on `surface` — its position, size, rotation and
    /// mirror flags, plus the child coordinate space if it is a group — or `None` when the shape
    /// declares no transform at all.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_transform`](mjx_pptx::Presentation::shape_transform).
    pub fn shape_transform(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Transform2D>, Error> {
        Ok(self
            .presentation
            .shape_transform(surface.to_model(), shape_idx.to_model())?)
    }

    /// Applies `transform` to shape `shape_idx` on `surface`, creating its transform element if it had
    /// none. Marks only that part dirty; everything else re-emits verbatim.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_transform`](mjx_pptx::Presentation::set_shape_transform).
    pub fn set_shape_transform(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        transform: &Transform2D,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_transform(
            surface.to_model(),
            shape_idx.to_model(),
            transform,
        )?)
    }

    /// The geometry of shape `shape_idx` on `surface`, as a `Geometry` — a preset shape
    /// (`Geometry::Preset`), a custom path list (`Geometry::Custom`), or `Geometry::Inherited` when the
    /// shape states no geometry of its own (it takes one from its placeholder / layout). Reading does
    /// not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_geometry`](mjx_pptx::Presentation::shape_geometry).
    pub fn shape_geometry(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Geometry, Error> {
        Ok(self
            .presentation
            .shape_geometry(surface.to_model(), shape_idx.to_model())?)
    }

    /// Every adjustment of shape `shape_idx`'s **preset** geometry, resolved against a concrete shape
    /// size: each value *and* the numeric domain it may move in.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_adjustments`](mjx_pptx::Presentation::shape_adjustments).
    pub fn shape_adjustments(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        size: GuideContext,
    ) -> Result<Vec<BoundedAdjustment>, Error> {
        Ok(self
            .presentation
            .shape_adjustments(surface.to_model(), shape_idx.to_model(), size)?)
    }

    /// Restates named adjustments of shape `shape_idx`'s **preset** geometry — the `a:gd` entries of
    /// its `a:avLst` — by their wire names (`adj`, `adj1`, `adj2`, …), in native spec units. An
    /// adjustment not named is left exactly as it was, and so are the `prst` token and every other
    /// property of the shape. Marks only that slide part dirty.
    ///
    /// This is the writing half of [`shape_adjustments`](Self::shape_adjustments), and it takes back
    /// the first two of the four things that reader reports: `wire_name` and `value`.
    ///
    /// It is not [`set_shape_geometry`](Self::set_shape_geometry): that call carries a **typed**
    /// `ShapeGeometry`, which cannot name the two adjustable presets with no typed variant (`sun`,
    /// `teardrop`) and states its values as a `Fraction` or an `Angle` where a file states an
    /// integer. The two are complementary — `set_shape_geometry` may change *which* shape it is and
    /// this may not.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_adjustments`](mjx_pptx::Presentation::set_shape_adjustments).
    pub fn set_shape_adjustments(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        adjustments: &[(&str, i32)],
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_adjustments(
            surface.to_model(),
            shape_idx.to_model(),
            adjustments,
        )?)
    }

    /// Sets the geometry of shape `shape_idx` on `surface` from a `Geometry`: a preset shape
    /// (`Geometry::Preset`) rewrites the `a:prstGeom`, a custom path list (`Geometry::Custom`) writes
    /// an `a:custGeom`, and `Geometry::Inherited` removes the shape's own geometry so an inherited one
    /// takes over. The two kinds are mutually exclusive, so setting one drops the other. Marks only
    /// that slide part dirty; everything else re-emits verbatim.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_geometry`](mjx_pptx::Presentation::set_shape_geometry).
    pub fn set_shape_geometry(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        geometry: Geometry,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_geometry(
            surface.to_model(),
            shape_idx.to_model(),
            geometry,
        )?)
    }
}
