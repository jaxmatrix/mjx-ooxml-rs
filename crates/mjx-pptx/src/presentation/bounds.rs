//! Where a shape sits and what shape it is: its bounds, its full 2-D transform, and its preset
//! or custom geometry.

use mjx_dml::{BoundedAdjustment, CustomGeometry, GuideContext, PresetGeometry, Transform2D};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawElement};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::geometry::{Geometry, ShapeBounds};
use crate::surface::Surface;
use crate::{placement, slide};

use super::effective::{resolve_shape_in, resolve_shape_ref};
use super::Presentation;

impl Presentation {
    /// The position and size of shape `shape_idx` on `surface` **on the slide** — absolute within
    /// [`slide_size`](Self::slide_size), whether the shape is top-level or nested inside groups.
    ///
    /// A `None` here is not "at the origin": it means the shape declares no bounds of its own, so a
    /// placeholder takes them from its layout and then its master — resolve *that* with
    /// [`effective_shape_bounds`](Self::effective_shape_bounds). It is also `None` for a transform
    /// that names only one of the two (bounds are all four numbers), and for a member whose enclosing
    /// group states no child coordinate space, since there is then no way to place it.
    ///
    /// For a **group member** the member's own `a:off` / `a:ext` are written in the group's child
    /// coordinate space (`a:chOff` / `a:chExt`); this composes every enclosing group's mapping —
    /// scale, mirror and rotation alike — and answers in slide EMU. To read what the file literally
    /// states instead, use [`shape_transform`](Self::shape_transform), which is left in the shape's
    /// own space.
    ///
    /// The rectangle is axis-aligned and, as for any shape, describes the box **before** the shape's
    /// own rotation; the composed rotation is on
    /// [`effective_shape_transform`](Self::effective_shape_transform). Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn shape_bounds(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<ShapeBounds>, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &path)?;
        let Some(element) = slide::shape_transform(shape, &doc.interner) else {
            return Ok(None);
        };
        let own = Transform2D::read(element, &doc.interner);
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        Ok(placement::compose(sp_tree, &doc.interner, &path, &own)
            .as_ref()
            .and_then(ShapeBounds::from_transform))
    }

    /// Moves and resizes shape `shape_idx` on `surface` to `bounds`, given **on the slide** — the
    /// same absolute space [`shape_bounds`](Self::shape_bounds) answers in. Creates the shape's
    /// transform element if it had none, and marks only that part dirty.
    ///
    /// For a **group member** the absolute rectangle is mapped back through every enclosing group's
    /// child coordinate space before it is written, so a caller places a member where it wants it on
    /// the slide and never converts by hand. Round-tripping `set_shape_bounds(shape_bounds(…))` is
    /// exact whenever the groups' scales are (a group at half size, say) and within a few EMU —
    /// millionths of an inch — when they are not.
    ///
    /// Only the position and size are written — a rotation, a flip, or the child coordinate space of
    /// a group are left exactly as they were. Note that resizing a **group** rescales its members,
    /// because a group maps its child space (`a:chOff` / `a:chExt`) onto its own extent; that is what
    /// PowerPoint does when you drag a group's handle.
    ///
    /// # Errors
    /// As [`set_shape_transform`](Self::set_shape_transform), plus
    /// [`ShapeCannotBePlaced`](PptxError::ShapeCannotBePlaced) when an enclosing group states no
    /// child coordinate space, leaving no way to convert the rectangle.
    pub fn set_shape_bounds(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        bounds: ShapeBounds,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` names what the write builds, `root` holds the tree — and the
        // ancestor groups the conversion reads live in that same tree.
        let RawDocument { interner, root, .. } = doc;
        let stated = child_space_bounds(root, interner, surface, &path, bounds)?;
        let shape = resolve_shape_in(root, interner, surface, &path)?;
        let slot = slide::shape_transform_slot_mut(shape, interner)?;
        stated.to_transform().apply(slot, interner);
        Ok(())
    }

    /// The **explicit** transform of shape `shape_idx` on `surface` — its position, size, rotation
    /// and mirror flags, plus the child coordinate space if it is a group — or `None` when the shape
    /// declares no transform at all.
    ///
    /// Where that transform lives depends on the shape's [`ShapeKind`](crate::ShapeKind): `p:spPr > a:xfrm` for a
    /// shape, picture or connector, `p:grpSpPr > a:xfrm` for a group, and `p:xfrm` — a direct child,
    /// in PresentationML's own namespace — for a graphic frame. A `p:contentPart` has none, and
    /// reads as `None`.
    ///
    /// Every field of the returned [`Transform2D`] is itself optional, and an unset one means the
    /// file does not state it rather than that it is zero. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn shape_transform(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<Transform2D>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        Ok(slide::shape_transform(shape, &doc.interner)
            .map(|element| Transform2D::read(element, &doc.interner)))
    }

    /// Applies `transform` to shape `shape_idx` on `surface`, creating its transform element if it
    /// had none. Marks only that part dirty; everything else re-emits verbatim.
    ///
    /// **Only the fields `transform` names are written**, in place — an unset field means *leave it
    /// alone*, never *clear it*. That is what lets a caller rotate a shape without restating its
    /// position, and what keeps a group's `a:chOff` / `a:chExt` intact when it is merely moved.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, the shape's kind
    /// has no transform in its schema
    /// ([`ShapeCannotBePositioned`](PptxError::ShapeCannotBePositioned) — only a `p:contentPart`), or
    /// it is missing the properties element its transform would live in
    /// ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_transform(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        transform: &Transform2D,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` names the element, `root` holds the tree it lands in.
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        let slot = slide::shape_transform_slot_mut(shape, interner)?;
        transform.apply(slot, interner);
        Ok(())
    }

    /// The geometry of shape `shape_idx` on `surface`, as a [`Geometry`] — a preset shape
    /// ([`Geometry::Preset`]), a custom path list ([`Geometry::Custom`]), or [`Geometry::Inherited`]
    /// when the shape states no geometry of its own (it takes one from its placeholder / layout).
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a preset shape's
    /// `prst` names a shape type this build does not recognize
    /// ([`UnknownShapeType`](PptxError::UnknownShapeType)).
    pub fn shape_geometry(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Geometry, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        if let Some(prst_geom) = slide::shape_prstgeom(shape, &doc.interner) {
            let geometry = PresetGeometry::from_xml(prst_geom, &doc.interner)?;
            let shape_geometry = geometry
                .shape(&doc.interner)
                .ok_or(PptxError::UnknownShapeType)?;
            Ok(Geometry::Preset(shape_geometry))
        } else if let Some(cust_geom) = slide::shape_custgeom(shape, &doc.interner) {
            let geometry = CustomGeometry::from_xml(cust_geom, &doc.interner)?;
            Ok(Geometry::Custom(geometry.spec(&doc.interner)))
        } else {
            Ok(Geometry::Inherited)
        }
    }

    /// Every adjustment of shape `shape_idx`'s **preset** geometry, resolved against a concrete
    /// shape size: each value *and* the numeric domain it may move in.
    ///
    /// A preset shape's adjustment domain is often not a number in the file at all but the name of a
    /// `gdLst` guide — `roundRect`'s corner radius stops at `maxAdj`, and what `maxAdj` *is* depends
    /// on the shape's width and height. Give the size and the guide-formula evaluator turns every one
    /// of them into a number; [`BoundedAdjustment::pinned_value`] then says what the shape actually
    /// draws when a file states a value outside its domain.
    ///
    /// `size` is a parameter rather than the shape's own extents because a shape may state none and
    /// inherit them; pass what [`shape_transform`](Self::shape_transform) reports, or the size the
    /// shape will be placed at. Empty when the shape has no `a:prstGeom` of its own, when its preset
    /// is fixed-geometry, or when its `prst` names a shape this build does not know. Reading does not
    /// dirty the part.
    ///
    /// ```no_run
    /// # use mjx_pptx::Presentation;
    /// use mjx_dml::{GuideContext, Size};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut deck = Presentation::open(&std::fs::read("deck.pptx")?)?;
    /// let size = Size::from_emu(1_828_800, 914_400);
    /// for adjustment in deck.shape_adjustments(0, 0, GuideContext::from_size(size))? {
    ///     println!(
    ///         "{}: {} in {}..={}",
    ///         adjustment.spec.wire_name,
    ///         adjustment.value,
    ///         adjustment.minimum,
    ///         adjustment.maximum,
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or a guide the
    /// domain depends on cannot be evaluated
    /// ([`GuideFormula`](PptxError::GuideFormula) — a zero width or height divides by zero in guides
    /// such as `*/ 50000 w ss`).
    pub fn shape_adjustments(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        size: GuideContext,
    ) -> Result<Vec<BoundedAdjustment>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        let Some(prst_geom) = slide::shape_prstgeom(shape, &doc.interner) else {
            return Ok(Vec::new());
        };
        let geometry = PresetGeometry::from_xml(prst_geom, &doc.interner)?;
        Ok(geometry.adjustments_for_size(&doc.interner, size)?)
    }

    /// Sets the geometry of shape `shape_idx` on `surface` from a [`Geometry`]: a preset shape
    /// ([`Geometry::Preset`]) rewrites the `a:prstGeom`, a custom path list ([`Geometry::Custom`])
    /// writes an `a:custGeom`, and [`Geometry::Inherited`] removes the shape's own geometry so an
    /// inherited one takes over. The two kinds are mutually exclusive, so setting one drops the other.
    /// Marks only that slide part dirty; everything else re-emits verbatim.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` to hold a geometry ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_geometry(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        geometry: Geometry,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` for name resolution / rebuild, `root` for locate + replace.
        let RawDocument { interner, root, .. } = doc;
        let shape = resolve_shape_in(root, interner, surface, &shape_idx.into())?;
        slide::set_geometry(shape, interner, &geometry)
    }
}

/// Converts slide-absolute `bounds` into the space the shape at `path` states its own transform in,
/// reading the enclosing groups from the part `root` the caller already holds.
///
/// The one place the write half of the mapping lives: `set_shape_bounds` and the shape cursor's
/// `bounds` edit both land here, so a member is placed the same way whichever surface asked.
pub(super) fn child_space_bounds(
    root: &RawElement,
    interner: &Interner,
    surface: Surface,
    path: &ShapePath,
    bounds: ShapeBounds,
) -> Result<ShapeBounds, PptxError> {
    if path.is_top_level() {
        return Ok(bounds); // A top-level shape states slide coordinates directly.
    }
    let sp_tree = slide::sp_tree(root, interner)?;
    placement::to_child_space(sp_tree, interner, path, bounds).ok_or_else(|| {
        PptxError::ShapeCannotBePlaced {
            surface,
            path: path.clone(),
        }
    })
}
