//! The shape tree: counting, identifying, adding, removing, and regrouping shapes on a surface.
//!
//! Every method here delegates to the identically-named method on
//! [`Presentation`](mjx_pptx::Presentation); see [the module documentation](crate::deck) for
//! the signature changes the facade makes and the reasons for each.

use crate::address::to_model_paths;
use crate::index::count;
use crate::{
    Deck, Error, GraphicFrameKind, PlaceholderInfo, PlaceholderType, PresetShapeType, ShapeBounds,
    ShapeInfo, ShapeKind, ShapePath, Surface,
};

impl Deck {
    /// The number of **top-level** shapes on `surface` — of **every** `ShapeKind` (autoshapes,
    /// pictures, groups, graphic frames, connectors), in document order. A group counts as one shape
    /// here; its own members are addressed by descending into it with a `ShapePath` and are not
    /// included in this count.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_count`](mjx_pptx::Presentation::shape_count).
    pub fn shape_count(&mut self, surface: Surface) -> Result<u32, Error> {
        Ok(count(self.presentation.shape_count(surface.to_model())?))
    }

    /// What kind of shape `shape_idx` on `surface` is — which of the index-addressed APIs apply to it
    /// (a `Picture` takes the `p:spPr` surface but has no text body; a `GroupShape` has no `p:spPr` at
    /// all).
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_kind`](mjx_pptx::Presentation::shape_kind).
    pub fn shape_kind(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<ShapeKind, Error> {
        Ok(self
            .presentation
            .shape_kind(surface.to_model(), shape_idx.to_model())?)
    }

    /// How many member shapes the group at `shape_idx` holds — `0` for anything that is not a group,
    /// since only a `p:grpSp` has members. This is the range a `ShapePath` may descend into.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_member_count`](mjx_pptx::Presentation::shape_member_count).
    pub fn shape_member_count(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.shape_member_count(
            surface.to_model(),
            shape_idx.to_model(),
        )?))
    }

    /// Every shape of `surface`, in document order — what it is and the placeholder slot it fills.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shapes`](mjx_pptx::Presentation::shapes).
    pub fn shapes(&mut self, surface: Surface) -> Result<Vec<ShapeInfo>, Error> {
        Ok(self.presentation.shapes(surface.to_model())?)
    }

    /// The address of the first shape on `surface` that fills the `kind` placeholder slot, or `None` if
    /// the surface offers none.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_for_placeholder`](mjx_pptx::Presentation::shape_for_placeholder).
    pub fn shape_for_placeholder(
        &mut self,
        surface: Surface,
        kind: PlaceholderType,
    ) -> Result<Option<u32>, Error> {
        Ok(self
            .presentation
            .shape_for_placeholder(surface.to_model(), kind)?
            .map(count))
    }

    /// The placeholder shape `shape_idx` on `surface` occupies (`p:nvPr > p:ph`), or `None` if it is
    /// not a placeholder.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::shape_placeholder`](mjx_pptx::Presentation::shape_placeholder).
    pub fn shape_placeholder(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<PlaceholderInfo>, Error> {
        Ok(self
            .presentation
            .shape_placeholder(surface.to_model(), shape_idx.to_model())?)
    }

    /// Appends a new rectangular text-box shape (`p:sp`) to `surface`, laid out at `bounds` and
    /// containing `text` (one paragraph per line, split on `\n`). Returns the index of the new shape in
    /// the slide's one shape index space (see `shape_count`). Only that part is marked dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_text_box`](mjx_pptx::Presentation::add_text_box).
    pub fn add_text_box(
        &mut self,
        surface: Surface,
        text: &str,
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_text_box(
            surface.to_model(),
            text,
            bounds,
        )?))
    }

    /// Appends a new autoshape (`p:sp`) with the given `preset` geometry to `surface`, laid out at
    /// `bounds`, with an empty text body. Returns the index of the new shape in the slide's one shape
    /// index space (see `shape_count`). Only that part is marked dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::add_shape`](mjx_pptx::Presentation::add_shape).
    pub fn add_shape(
        &mut self,
        surface: Surface,
        preset: PresetShapeType,
        bounds: ShapeBounds,
    ) -> Result<u32, Error> {
        Ok(count(self.presentation.add_shape(
            surface.to_model(),
            preset,
            bounds,
        )?))
    }

    /// Removes shape `shape_idx` from `surface`, closing the gap in the shape index space: every later
    /// shape on that surface moves down one index. Only that part is marked dirty.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::remove_shape`](mjx_pptx::Presentation::remove_shape).
    pub fn remove_shape(&mut self, surface: Surface, shape_idx: ShapePath) -> Result<(), Error> {
        Ok(self
            .presentation
            .remove_shape(surface.to_model(), shape_idx.to_model())?)
    }

    /// Wraps `members` — which must be siblings — in a new group, returning the group's address.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::group_shapes`](mjx_pptx::Presentation::group_shapes).
    pub fn group_shapes(
        &mut self,
        surface: Surface,
        members: &[ShapePath],
    ) -> Result<ShapePath, Error> {
        Ok(ShapePath::from(self.presentation.group_shapes(
            surface.to_model(),
            &to_model_paths(members),
        )?))
    }

    /// Dissolves the group at `shape_idx`, returning where its members now are.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::ungroup`](mjx_pptx::Presentation::ungroup).
    pub fn ungroup(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Vec<ShapePath>, Error> {
        Ok(self
            .presentation
            .ungroup(surface.to_model(), shape_idx.to_model())?
            .into_iter()
            .map(ShapePath::from)
            .collect())
    }

    /// Moves shape `shape_idx` into the group at `group_idx`, as its last member, and returns its new
    /// address.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::move_shape_into_group`](mjx_pptx::Presentation::move_shape_into_group).
    pub fn move_shape_into_group(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        group_idx: ShapePath,
    ) -> Result<ShapePath, Error> {
        Ok(ShapePath::from(self.presentation.move_shape_into_group(
            surface.to_model(),
            shape_idx.to_model(),
            group_idx.to_model(),
        )?))
    }

    /// Moves shape `shape_idx` out of the group holding it, into that group's own container and
    /// directly after it in z-order. Returns its new address.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::move_shape_out_of_group`](mjx_pptx::Presentation::move_shape_out_of_group).
    pub fn move_shape_out_of_group(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<ShapePath, Error> {
        Ok(ShapePath::from(self.presentation.move_shape_out_of_group(
            surface.to_model(),
            shape_idx.to_model(),
        )?))
    }

    /// What the graphic frame `shape_idx` on `surface` frames — a `Table`, a `Chart`, a `Diagram` or
    /// something else — or `None` when the shape is not a `p:graphicFrame` at all. Reading does not
    /// dirty the part.
    ///
    /// # Errors
    /// Returns an [`Error`] whose [`code`](Error::code) classifies the failure and whose
    /// [`detail`](Error::detail) names where it happened.
    ///
    /// See [`Presentation::graphic_frame_kind`](mjx_pptx::Presentation::graphic_frame_kind).
    pub fn graphic_frame_kind(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
    ) -> Result<Option<GraphicFrameKind>, Error> {
        Ok(self
            .presentation
            .graphic_frame_kind(surface.to_model(), shape_idx.to_model())?)
    }
}
