//! What a shape draws itself with: fill, outline, effect list, and the two 3-D surfaces.

use mjx_dml::{
    EffectList, EffectListSpec, Fill, FillSpec, LineProperties, LineSpec, Scene3D, Scene3DSpec,
    Shape3D, Shape3DSpec,
};
use mjx_ooxml_core::{FromXml, RawDocument};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::slide;
use crate::surface::Surface;

use super::deck::fill_relationship_declaration;
use super::effective::{resolve_shape_in, resolve_shape_ref};
use super::Presentation;

impl Presentation {
    /// The explicit fill of shape `shape_idx` on `surface`, as an interner-free [`FillSpec`],
    /// or `None` if the shape declares no fill in its `p:spPr` (its fill is then inherited from the
    /// placeholder / style / theme — resolving that is a separate, future task). Reading does not
    /// dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the fill element
    /// is not well-formed.
    pub fn shape_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<FillSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        match slide::shape_fill(shape, &doc.interner) {
            Some(fill) => {
                let fill = Fill::from_xml(fill, &doc.interner)?;
                Ok(Some(fill.spec(&doc.interner)))
            }
            None => Ok(None),
        }
    }

    /// Sets the fill of shape `shape_idx` on `surface` from an interner-free [`FillSpec`],
    /// rebuilding the `p:spPr` fill element (replacing an existing one in place, or inserting a new
    /// one after any geometry and before `a:ln`). Marks only that part dirty.
    ///
    /// A [`FillSpec::Picture`] writes only the `a:blip@r:embed` reference; the image part and its
    /// relationship must already exist in the package — create both with
    /// [`add_image`](Self::add_image), which returns the id to use.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        fill: &FillSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the fill element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        // A picture fill carries an `r:embed`, so the built element must be able to resolve the `r`
        // prefix — computed from the part root before the borrow descends into the shape tree.
        let rel_declaration = fill_relationship_declaration(fill, root, interner);
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::set_fill(shape, interner, fill, rel_declaration)
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no fill" (`a:noFill`). A shorthand
    /// for [`set_shape_fill`](Self::set_shape_fill) with [`FillSpec::None`].
    ///
    /// # Errors
    /// As [`set_shape_fill`](Self::set_shape_fill).
    pub fn set_shape_no_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        self.set_shape_fill(surface, shape_idx, &FillSpec::None)
    }

    /// The **explicit** outline of shape `shape_idx` on `surface` — its `p:spPr > a:ln` as an
    /// interner-free [`LineSpec`] — or `None` when the shape declares no `a:ln` (its outline is then
    /// inherited; effective outline resolution is a later step). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the outline element
    /// is not well-formed.
    pub fn shape_outline(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<LineSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        match slide::shape_line(shape, &doc.interner) {
            Some(line) => {
                let line = LineProperties::from_xml(line, &doc.interner)?;
                Ok(Some(line.spec(&doc.interner)))
            }
            None => Ok(None),
        }
    }

    /// Sets the outline of shape `shape_idx` on `surface` from an interner-free [`LineSpec`],
    /// rebuilding the `p:spPr` `a:ln` element (replacing an existing one in place, or inserting a new
    /// one after any geometry and fill, before effects). Marks only that part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_outline(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        line: &LineSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the outline element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::set_line(shape, interner, line)
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no outline" (`<a:ln><a:noFill/></a:ln>`).
    /// A shorthand for [`set_shape_outline`](Self::set_shape_outline) with a [`LineSpec`] whose fill is
    /// [`FillSpec::None`] — PowerPoint's "no line", distinct from an absent `a:ln`.
    ///
    /// # Errors
    /// As [`set_shape_outline`](Self::set_shape_outline).
    pub fn set_shape_no_outline(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let line = LineSpec {
            fill: Some(FillSpec::None),
            ..LineSpec::new()
        };
        self.set_shape_outline(surface, shape_idx, &line)
    }

    /// The **explicit** effects of shape `shape_idx` on `surface` — its `p:spPr > a:effectLst`
    /// as an interner-free [`EffectListSpec`] — or `None` when the shape declares no `a:effectLst` (its
    /// effects are then inherited; effective effect resolution is a later step). A shape whose effects
    /// use the rarer `a:effectDag` alternative also reads as `None` (that opaque graph is not modeled).
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the effect element
    /// is not well-formed.
    pub fn shape_effects(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<EffectListSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        match slide::shape_effects(shape, &doc.interner) {
            Some(effects) => {
                let effects = EffectList::from_xml(effects, &doc.interner)?;
                Ok(Some(effects.spec(&doc.interner)))
            }
            None => Ok(None),
        }
    }

    /// Sets the effects of shape `shape_idx` on `surface` from an interner-free
    /// [`EffectListSpec`], rebuilding the `p:spPr` `a:effectLst` element (replacing an existing effect
    /// container in place — either an `a:effectLst` or the mutually-exclusive `a:effectDag`, which is
    /// overwritten — or inserting a new one after any geometry, fill, and outline, before the 3-D and
    /// extension children). Marks only that part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_effects(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        effects: &EffectListSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the effect element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::set_effects(shape, interner, effects)
    }

    /// Sets shape `shape_idx` on `surface` to explicit "no effects" (an empty `<a:effectLst/>`).
    /// A shorthand for [`set_shape_effects`](Self::set_shape_effects) with an empty [`EffectListSpec`] —
    /// the explicitly-cleared effect state that overrides inheritance, distinct from an absent
    /// `a:effectLst`. Reads back as `Some(EffectListSpec::default())`.
    ///
    /// # Errors
    /// As [`set_shape_effects`](Self::set_shape_effects).
    pub fn set_shape_no_effects(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        self.set_shape_effects(surface, shape_idx, &EffectListSpec::new())
    }

    /// The **explicit** 3-D scene of shape `shape_idx` on `surface` — its `p:spPr > a:scene3d`
    /// (`CT_Scene3D`) as an interner-free [`Scene3DSpec`] — or `None` when the shape declares no
    /// `a:scene3d`. 3-D has no inheritance chain, so an absent scene means the shape is flat, not that
    /// it inherits one. A scene present but missing a schema-required part (its `a:camera` or
    /// `a:lightRig`) also reads as `None`. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the `a:scene3d`
    /// element is not well-formed.
    pub fn shape_scene_3d(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<Scene3DSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        match slide::shape_scene_3d(shape, &doc.interner) {
            Some(scene) => {
                let scene = Scene3D::from_xml(scene, &doc.interner)?;
                Ok(scene.spec(&doc.interner))
            }
            None => Ok(None),
        }
    }

    /// Sets the 3-D scene of shape `shape_idx` on `surface` from an interner-free [`Scene3DSpec`],
    /// rebuilding the `p:spPr` `a:scene3d` (replacing an existing one in place, or inserting a new one
    /// after any geometry, fill, outline, and effects, before `a:sp3d`). Rebuilding from a spec drops
    /// any opaque scene internals (`a:backdrop`, `extLst`). Marks only that part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_scene_3d(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        scene: &Scene3DSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::set_scene_3d(shape, interner, scene)
    }

    /// Clears the 3-D scene of shape `shape_idx` on `surface` by **removing** its `a:scene3d`
    /// entirely — a shape without a scene is flat. Unlike effects, there is no "explicitly empty"
    /// scene: `CT_Scene3D` requires a camera and light rig, and 3-D does not inherit, so clearing
    /// removes rather than empties. A no-op (still `Ok`) when the shape has no scene. Marks the part
    /// dirty only if it removed something.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn clear_shape_scene_3d(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::remove_scene_3d(shape, interner);
        Ok(())
    }

    /// The **explicit** 3-D properties of shape `shape_idx` on `surface` — its `p:spPr > a:sp3d`
    /// (`CT_Shape3D`: extrusion, contour, bevels, material) as an interner-free [`Shape3DSpec`] — or
    /// `None` when the shape declares no `a:sp3d`. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the `a:sp3d`
    /// element is not well-formed.
    pub fn shape_3d_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<Shape3DSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        match slide::shape_sp3d(shape, &doc.interner) {
            Some(sp3d) => {
                let sp3d = Shape3D::from_xml(sp3d, &doc.interner)?;
                Ok(Some(sp3d.spec(&doc.interner)))
            }
            None => Ok(None),
        }
    }

    /// Sets the 3-D properties of shape `shape_idx` on `surface` from an interner-free
    /// [`Shape3DSpec`], rebuilding the `p:spPr` `a:sp3d` (replacing an existing one in place, or
    /// inserting a new one after every other visual property, before any `a:extLst`). Rebuilding from
    /// a spec drops any opaque `extLst`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_3d_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        properties: &Shape3DSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::set_sp3d(shape, interner, properties)
    }

    /// Clears the 3-D properties of shape `shape_idx` on `surface` by **removing** its `a:sp3d`
    /// entirely. A no-op (still `Ok`) when the shape has none. Marks the part dirty only if it removed
    /// something.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn clear_shape_3d_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::remove_sp3d(shape, interner);
        Ok(())
    }
}
