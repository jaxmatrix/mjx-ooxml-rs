//! A shape's own fill, outline, effect list and 3-D properties — what the shape *states*, not what
//! a renderer resolves.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::{
    Deck, EffectListSpec, Error, FillSpec, LineSpec, Scene3DSpec, Shape3DSpec, ShapePath, Surface,
};

impl Deck {
    /// The explicit fill of shape `shape_idx` on `surface`, as an interner-free `FillSpec`, or `None`
    /// if the shape declares no fill in its `p:spPr` (its fill is then inherited from the placeholder /
    /// style / theme — resolving that is a separate, future task). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_fill`](mjx_pptx::Presentation::shape_fill).
    pub fn shape_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<FillSpec>, Error> {
        Ok(self
            .presentation
            .shape_fill(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets the fill of shape `shape_idx` on `surface` from an interner-free `FillSpec`, rebuilding the
    /// `p:spPr` fill element (replacing an existing one in place, or inserting a new one after any
    /// geometry and before `a:ln`). Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_fill`](mjx_pptx::Presentation::set_shape_fill).
    pub fn set_shape_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        fill: &FillSpec,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_fill(surface.to_model(), shape_idx.to_model(), fill)?)
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no fill" (`a:noFill`). A shorthand for
    /// `set_shape_fill` with `FillSpec::None`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_no_fill`](mjx_pptx::Presentation::set_shape_no_fill).
    pub fn set_shape_no_fill(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_no_fill(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **explicit** outline of shape `shape_idx` on `surface` — its `p:spPr > a:ln` as an interner-
    /// free `LineSpec` — or `None` when the shape declares no `a:ln` (its outline is then inherited;
    /// effective outline resolution is a later step). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_outline`](mjx_pptx::Presentation::shape_outline).
    pub fn shape_outline(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<LineSpec>, Error> {
        Ok(self
            .presentation
            .shape_outline(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets the outline of shape `shape_idx` on `surface` from an interner-free `LineSpec`, rebuilding
    /// the `p:spPr` `a:ln` element (replacing an existing one in place, or inserting a new one after
    /// any geometry and fill, before effects). Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_outline`](mjx_pptx::Presentation::set_shape_outline).
    pub fn set_shape_outline(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        line: &LineSpec,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_outline(surface.to_model(), shape_idx.to_model(), line)?)
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no outline" (`<a:ln><a:noFill/></a:ln>`). A
    /// shorthand for `set_shape_outline` with a `LineSpec` whose fill is `FillSpec::None` —
    /// PowerPoint's "no line", distinct from an absent `a:ln`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_no_outline`](mjx_pptx::Presentation::set_shape_no_outline).
    pub fn set_shape_no_outline(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_no_outline(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **explicit** effects of shape `shape_idx` on `surface` — its `p:spPr > a:effectLst` as an
    /// interner-free `EffectListSpec` — or `None` when the shape declares no `a:effectLst` (its effects
    /// are then inherited; effective effect resolution is a later step). A shape whose effects use the
    /// rarer `a:effectDag` alternative also reads as `None` (that opaque graph is not modeled). Reading
    /// does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_effects`](mjx_pptx::Presentation::shape_effects).
    pub fn shape_effects(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<EffectListSpec>, Error> {
        Ok(self
            .presentation
            .shape_effects(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets the effects of shape `shape_idx` on `surface` from an interner-free `EffectListSpec`,
    /// rebuilding the `p:spPr` `a:effectLst` element (replacing an existing effect container in place —
    /// either an `a:effectLst` or the mutually-exclusive `a:effectDag`, which is overwritten — or
    /// inserting a new one after any geometry, fill, and outline, before the 3-D and extension
    /// children). Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_effects`](mjx_pptx::Presentation::set_shape_effects).
    pub fn set_shape_effects(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        effects: &EffectListSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_effects(
            surface.to_model(),
            shape_idx.to_model(),
            effects,
        )?)
    }

    /// Sets shape `shape_idx` on `surface` to explicit "no effects" (an empty `<a:effectLst/>`). A
    /// shorthand for `set_shape_effects` with an empty `EffectListSpec` — the explicitly-cleared effect
    /// state that overrides inheritance, distinct from an absent `a:effectLst`. Reads back as
    /// `Some(EffectListSpec::default())`.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_no_effects`](mjx_pptx::Presentation::set_shape_no_effects).
    pub fn set_shape_no_effects(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_no_effects(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **explicit** 3-D scene of shape `shape_idx` on `surface` — its `p:spPr > a:scene3d`
    /// (`CT_Scene3D`) as an interner-free `Scene3DSpec` — or `None` when the shape declares no
    /// `a:scene3d`. 3-D has no inheritance chain, so an absent scene means the shape is flat, not that
    /// it inherits one. A scene present but missing a schema-required part (its `a:camera` or
    /// `a:lightRig`) also reads as `None`. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_scene_3d`](mjx_pptx::Presentation::shape_scene_3d).
    pub fn shape_scene_3d(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Scene3DSpec>, Error> {
        Ok(self
            .presentation
            .shape_scene_3d(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets the 3-D scene of shape `shape_idx` on `surface` from an interner-free `Scene3DSpec`,
    /// rebuilding the `p:spPr` `a:scene3d` (replacing an existing one in place, or inserting a new one
    /// after any geometry, fill, outline, and effects, before `a:sp3d`). Rebuilding from a spec drops
    /// any opaque scene internals (`a:backdrop`, `extLst`). Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_scene_3d`](mjx_pptx::Presentation::set_shape_scene_3d).
    pub fn set_shape_scene_3d(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        scene: &Scene3DSpec,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .set_shape_scene_3d(surface.to_model(), shape_idx.to_model(), scene)?)
    }

    /// Clears the 3-D scene of shape `shape_idx` on `surface` by **removing** its `a:scene3d` entirely
    /// — a shape without a scene is flat. Unlike effects, there is no "explicitly empty" scene:
    /// `CT_Scene3D` requires a camera and light rig, and 3-D does not inherit, so clearing removes
    /// rather than empties. A no-op (still `Ok`) when the shape has no scene. Marks the part dirty only
    /// if it removed something.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_shape_scene_3d`](mjx_pptx::Presentation::clear_shape_scene_3d).
    pub fn clear_shape_scene_3d(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .clear_shape_scene_3d(surface.to_model(), shape_idx.to_model())?)
    }

    /// The **explicit** 3-D properties of shape `shape_idx` on `surface` — its `p:spPr > a:sp3d`
    /// (`CT_Shape3D`: extrusion, contour, bevels, material) as an interner-free `Shape3DSpec` — or
    /// `None` when the shape declares no `a:sp3d`. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_3d_properties`](mjx_pptx::Presentation::shape_3d_properties).
    pub fn shape_3d_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<Shape3DSpec>, Error> {
        Ok(self
            .presentation
            .shape_3d_properties(surface.to_model(), shape_idx.to_model())?)
    }

    /// Sets the 3-D properties of shape `shape_idx` on `surface` from an interner-free `Shape3DSpec`,
    /// rebuilding the `p:spPr` `a:sp3d` (replacing an existing one in place, or inserting a new one
    /// after every other visual property, before any `a:extLst`). Rebuilding from a spec drops any
    /// opaque `extLst`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::set_shape_3d_properties`](mjx_pptx::Presentation::set_shape_3d_properties).
    pub fn set_shape_3d_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        properties: &Shape3DSpec,
    ) -> Result<(), Error> {
        Ok(self.presentation.set_shape_3d_properties(
            surface.to_model(),
            shape_idx.to_model(),
            properties,
        )?)
    }

    /// Clears the 3-D properties of shape `shape_idx` on `surface` by **removing** its `a:sp3d`
    /// entirely. A no-op (still `Ok`) when the shape has none. Marks the part dirty only if it removed
    /// something.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::clear_shape_3d_properties`](mjx_pptx::Presentation::clear_shape_3d_properties).
    pub fn clear_shape_3d_properties(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<(), Error> {
        Ok(self
            .presentation
            .clear_shape_3d_properties(surface.to_model(), shape_idx.to_model())?)
    }
}
