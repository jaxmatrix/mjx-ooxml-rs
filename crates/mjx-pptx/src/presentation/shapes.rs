//! The shape tree of one surface: how many shapes there are, what each one is, and adding,
//! removing and regrouping them.

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::namespaces::PML;
use mjx_opc::PartName;

use crate::address::ShapePath;
use crate::cursor::{ShapeCursor, ShapeEdit};
use crate::error::PptxError;
use crate::geometry::ShapeBounds;
use crate::slide::GraphicFrameKind;
use crate::slide::{PlaceholderInfo, ShapeKind};
use crate::surface::Surface;
use crate::{build, group, nav, slide};

use super::bounds::child_space_bounds;
use super::deck::relationship_prefix;
use super::effective::{resolve_shape_in, resolve_shape_position_in, resolve_shape_ref};
use super::element_builders::{build_nv_sp_pr, build_paragraph, build_sp_pr, build_text_body};
use super::pictures::picture_at;
use super::text::{apply_edits_to_shape, PreparedEdit};
use super::Presentation;

impl Presentation {
    /// The number of **top-level** shapes on `surface` — of **every** [`ShapeKind`] (autoshapes,
    /// pictures, groups, graphic frames, connectors), in document order. A group counts as one shape
    /// here; its own members are addressed by descending into it with a [`ShapePath`] and are not
    /// included in this count.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the index is out of range or the slide is malformed.
    pub fn shape_count(&mut self, surface: impl Into<Surface>) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        Ok(slide::shapes(sp_tree, &doc.interner).count())
    }

    /// What kind of shape `shape_idx` on `surface` is — which of the index-addressed APIs
    /// apply to it (a [`Picture`](ShapeKind::Picture) takes the `p:spPr` surface but has no text body;
    /// a [`GroupShape`](ShapeKind::GroupShape) has no `p:spPr` at all).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn shape_kind(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<ShapeKind, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        slide::shape_kind(shape, &doc.interner)
            .ok_or(PptxError::MalformedSlide("shape tree child is not a shape"))
    }

    /// How many member shapes the group at `shape_idx` holds — `0` for anything that is not a group,
    /// since only a `p:grpSp` has members. This is the range a [`ShapePath`] may descend into.
    ///
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn shape_member_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        // A group's members are exactly the shape-kind children of its element, which is what
        // `shapes` enumerates at every level; a leaf shape simply has none.
        Ok(slide::shapes(shape, &doc.interner).count())
    }

    /// Opens a [`ShapeCursor`] on shape `shape_idx` of `surface`: the address is stated once, the
    /// edits after it, and nothing is written until [`apply`](ShapeCursor::apply).
    ///
    /// This is the ergonomic layer over the `set_shape_*` methods — the same edits, said once instead
    /// of once per call, with group descent (`.member`, `.sibling`, `.parent`) built in. See the
    /// [cursor docs](crate::ShapeCursor) for what it does and does not do.
    ///
    /// ```no_run
    /// # use mjx_pptx::{Presentation, PptxError};
    /// # use mjx_dml::{FillSpec, LineSpec};
    /// # fn f(deck: &mut Presentation, navy: FillSpec, rule: LineSpec) -> Result<(), PptxError> {
    /// deck.shape(0, 2)?                          // the group at top-level index 2
    ///     .member(0)?.fill(navy).outline(rule)   // its first member
    ///     .apply()?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`PptxError`] if the address is out of range or the part is malformed — a cursor is
    /// never opened on a shape that is not there.
    pub fn shape(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<ShapeCursor<'_>, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        self.shape_kind(surface, &path)?;
        Ok(ShapeCursor::new(self, surface, path))
    }

    /// Writes a cursor's recorded edits — the single commit point behind
    /// [`ShapeCursor::apply`](crate::ShapeCursor::apply), in three passes:
    ///
    /// 1. **package work** — the relationships and media parts the rel-bearing edits need, which
    ///    cannot happen while the part tree is borrowed;
    /// 2. **one mutable borrow**, applying every edit in the order it was recorded and dirtying the
    ///    part exactly once;
    /// 3. **sweep** the hyperlink relationships nothing names any more.
    ///
    /// There is no address-validation pass, because there is nothing for one to catch: a cursor
    /// checks every address as it moves onto it, holds the deck exclusively while it records, and no
    /// edit here adds or removes a shape — so an address recorded is an address that still resolves.
    pub(crate) fn apply_shape_edits(
        &mut self,
        surface: Surface,
        edits: Vec<(ShapePath, ShapeEdit)>,
    ) -> Result<(), PptxError> {
        if edits.is_empty() {
            return Ok(());
        }
        let part = self.surface_part(surface)?;

        // 1 — the package work each rel-bearing edit needs, turning intents into relationship ids.
        // Every hyperlink id involved (the ones already on the shapes, and the ones added here) is
        // remembered for the sweep: an id superseded by a later edit in the same pass is
        // unreferenced by the end of the write and must go with it.
        let mut prepared: Vec<(ShapePath, PreparedEdit)> = Vec::with_capacity(edits.len());
        let mut hyperlink_ids: Vec<String> = Vec::new();
        for (path, edit) in edits {
            let prepared_edit = match edit {
                ShapeEdit::Hyperlink(link) => {
                    if let Some(previous) = self.shape_hyperlink_rel_id(&part, &path)? {
                        hyperlink_ids.push(previous);
                    }
                    match link {
                        Some(link) => {
                            let (rel_id, action) = self.add_hyperlink_rel(&part, &link)?;
                            hyperlink_ids.push(rel_id.clone());
                            PreparedEdit::Hyperlink {
                                rel_id: Some(rel_id),
                                action,
                            }
                        }
                        None => PreparedEdit::Hyperlink {
                            rel_id: None,
                            action: None,
                        },
                    }
                }
                ShapeEdit::Image(bytes) => {
                    // The picture is checked before the package grows, so a wrong address adds no
                    // image part — as `set_picture_image` does.
                    self.check_picture_blip_fill(surface, &path)?;
                    PreparedEdit::Image(self.add_image(surface, &bytes)?)
                }
                other => PreparedEdit::Element(other),
            };
            prepared.push((path, prepared_edit));
        }

        // 2 — the write itself.
        let written = self.write_shape_edits(surface, &part, &prepared);

        // 3 — a link nothing names any more takes its relationship with it. This runs even when the
        // write failed, so a relationship added for an edit that never landed is not left orphaned;
        // the write's own error is the one reported.
        let mut swept = Ok(());
        for rel_id in hyperlink_ids {
            let removed = self.remove_hyperlink_rel_if_unreferenced(&part, &rel_id);
            if swept.is_ok() {
                swept = removed;
            }
        }
        written.and(swept)
    }

    /// Applies every prepared edit to `part` under **one** mutable borrow, in the order recorded, so
    /// the part is parsed once, dirtied once, and re-serialized once.
    fn write_shape_edits(
        &mut self,
        surface: Surface,
        part: &PartName,
        prepared: &[(ShapePath, PreparedEdit)],
    ) -> Result<(), PptxError> {
        let doc = self.package.part_tree_mut(part)?;
        // Split the borrow: `interner` names what the edits build, `root` holds the tree.
        let RawDocument { interner, root, .. } = doc;
        // Both read the part *root*, so they are taken before the borrow descends into the shape
        // tree. Taking them unconditionally is free: interning a prefix nothing ends up using adds a
        // symbol to the table and nothing to the output, which is written by walking the tree.
        let rel_prefix = relationship_prefix(root, interner);
        let blip_declaration = build::relationship_prefix_declaration(root, interner);

        let mut at = 0;
        while at < prepared.len() {
            let path = &prepared[at].0;

            // A bounds edit is stated in *slide* coordinates, so for a group member it must be
            // mapped back through the enclosing groups — which means reading them from the part
            // root, and so cannot happen from inside the borrow of a single shape. It is converted
            // against the tree as the edits before it have left it, which is what makes "move the
            // member, then move its group" mean what it says.
            if let PreparedEdit::Element(ShapeEdit::Bounds(bounds)) = &prepared[at].1 {
                let stated = child_space_bounds(root, interner, surface, path, *bounds)?;
                let shape = resolve_shape_in(root, interner, surface, path)?;
                let slot = slide::shape_transform_slot_mut(shape, interner)?;
                stated.to_transform().apply(slot, interner);
                at += 1;
                continue;
            }

            // Otherwise consecutive edits on one shape share a single resolution of its address.
            let run_end = prepared[at..]
                .iter()
                .position(|(other, edit)| {
                    other != path || matches!(edit, PreparedEdit::Element(ShapeEdit::Bounds(_)))
                })
                .map_or(prepared.len(), |offset| at + offset);
            let shape = resolve_shape_in(root, interner, surface, path)?;
            apply_edits_to_shape(
                shape,
                interner,
                &prepared[at..run_end],
                rel_prefix,
                blip_declaration.as_ref(),
            )?;
            at = run_end;
        }
        Ok(())
    }

    /// Checks that `path` addresses a picture that has the `p:blipFill` an image edit rewrites.
    fn check_picture_blip_fill(
        &mut self,
        surface: Surface,
        path: &ShapePath,
    ) -> Result<(), PptxError> {
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let picture = picture_at(sp_tree, &doc.interner, surface, path)?;
        if nav::child(picture, &doc.interner, PML, "blipFill").is_none() {
            return Err(PptxError::PictureHasNoBlipFill);
        }
        Ok(())
    }

    /// The placeholder shape `shape_idx` on `surface` occupies (`p:nvPr > p:ph`), or `None` if it is
    /// not a placeholder.
    ///
    /// Asked of a **layout**, this is how a caller learns what that layout offers a slide to fill —
    /// its title, body, and content slots, with the names PowerPoint shows. Asked of a **slide**, it
    /// is the slot the shape inherits through. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the part is malformed.
    pub fn shape_placeholder(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<PlaceholderInfo>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        Ok(slide::shape_placeholder_info(shape, &doc.interner))
    }

    /// Appends a new rectangular text-box shape (`p:sp`) to `surface`, laid out at `bounds`
    /// and containing `text` (one paragraph per line, split on `\n`). Returns the index of the new
    /// shape in the slide's one shape index space (see [`shape_count`](Self::shape_count)). Only that
    /// part is marked dirty.
    ///
    /// The shape is a plain text box (`p:cNvSpPr@txBox="1"`, `a:prstGeom@prst="rect"`) with no
    /// placeholder, so it renders as free-standing text. Its non-visual id (`p:cNvPr@id`) is one past
    /// the largest id already present on that part, keeping ids unique.
    ///
    /// Every paragraph created here holds exactly **one run**, an empty line included, so each line is
    /// addressable as run 0 of its paragraph and can be rewritten with
    /// [`set_shape_text`](Self::set_shape_text).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the part is malformed.
    pub fn add_text_box(
        &mut self,
        surface: impl Into<Surface>,
        text: &str,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the new names, `root` receives the new subtree.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let shape = build_text_box(interner, next_id, text, bounds);
        sp_tree.children.push(RawNode::Element(shape));
        sp_tree.empty = false;

        // The new shape is the last child of the shape tree.
        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Appends a new autoshape (`p:sp`) with the given `preset` geometry to `surface`, laid
    /// out at `bounds`, with an empty text body. Returns the index of the new shape in the slide's one
    /// shape index space (see [`shape_count`](Self::shape_count)). Only that part is marked dirty.
    ///
    /// The shape is created with the preset's default adjustments; customize them afterward with
    /// [`set_shape_geometry`](Self::set_shape_geometry). Its non-visual id (`p:cNvPr@id`) is one past
    /// the largest id already present on that part, keeping ids unique.
    ///
    /// Its text body holds one paragraph with one **empty run**, so the shape can be labelled straight
    /// away with [`set_shape_text(surface, idx, 0, "…")`](Self::set_shape_text).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or the part is malformed.
    pub fn add_shape(
        &mut self,
        surface: impl Into<Surface>,
        preset: PresetShapeType,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let shape = build_shape(interner, next_id, preset.to_wire(), bounds);
        sp_tree.children.push(RawNode::Element(shape));
        sp_tree.empty = false;

        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// Removes shape `shape_idx` from `surface`, closing the gap in the shape index space: every later
    /// shape on that surface moves down one index. Only that part is marked dirty.
    ///
    /// Shapes are addressed in the one index space [`shape_count`](Self::shape_count) defines, so this
    /// removes a picture or a group exactly as it removes an autoshape.
    ///
    /// Relationships and parts the shape used are **left in place** — removing a picture does not
    /// remove its image. An unused relationship is valid OOXML, [`add_image`](Self::add_image)
    /// de-duplicates by content so re-adding the same image reuses the part it already has, and a
    /// sibling shape may well be showing the same image.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIndexOutOfRange`] if `shape_idx` is out of range on that surface, or
    /// another [`PptxError`] if the surface index is out of range or the part is malformed.
    pub fn remove_shape(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let (parent, position) =
            resolve_shape_position_in(root, interner, surface, &shape_idx.into())?;
        parent.children.remove(position);
        // The shape's own indentation goes with it, or repeated removals leave a growing run of blank
        // lines behind. Only whitespace is dropped — never a comment or a sibling's text.
        if position > 0 && nav::is_whitespace_text(&parent.children[position - 1]) {
            parent.children.remove(position - 1);
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Group structure — which shapes there are, and where they sit in the tree
    //
    // Everything above edits a shape once it has been found. These four change the tree itself, so
    // they move addresses: after any of them, an index taken beforehand may name a different shape.
    // Each preserves what the slide *looks* like — a shape that changes parent changes coordinate
    // system, and its transform is restated so it does not move a pixel.
    // -----------------------------------------------------------------------------------------

    /// Wraps `members` — which must be siblings — in a new group, returning the group's address.
    ///
    /// This is "select these shapes and group them". The group's box is the union of the members'
    /// own boxes, and its child coordinate space is set identical to it, so the mapping is the
    /// identity: **the members keep their coordinates exactly** and nothing moves on screen, with no
    /// rounding anywhere. The group takes the z-order position of the earliest member, and the
    /// members keep their relative order inside it whatever order they were named in.
    ///
    /// Address the members afterwards through the returned path — `group.child(0)` is the first.
    ///
    /// ```no_run
    /// # use mjx_pptx::{Presentation, PptxError};
    /// # use mjx_dml::FillSpec;
    /// # fn f(deck: &mut Presentation, navy: FillSpec) -> Result<(), PptxError> {
    /// let group = deck.group_shapes(0, &[1.into(), 2.into()])?;
    /// deck.set_shape_fill(0, group.child(0), &navy)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    /// Returns [`GroupNeedsTwoShapes`](PptxError::GroupNeedsTwoShapes) for fewer than two members
    /// (ECMA-376 Part 1 §L.4.7.4 calls a smaller group degenerate),
    /// [`ShapesAreNotSiblings`](PptxError::ShapesAreNotSiblings) if they do not share a container or
    /// one is named twice, [`ShapeHasNoBounds`](PptxError::ShapeHasNoBounds) for a member that states
    /// no position and size of its own, or another [`PptxError`] if an address is out of range or the
    /// part is malformed.
    pub fn group_shapes(
        &mut self,
        surface: impl Into<Surface>,
        members: &[ShapePath],
    ) -> Result<ShapePath, PptxError> {
        let surface = surface.into();
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        group::group_shapes(root, interner, surface, members)
    }

    /// Dissolves the group at `shape_idx`, returning where its members now are.
    ///
    /// The inverse of [`group_shapes`](Self::group_shapes): every member keeps its absolute
    /// placement, because the group's mapping is unwound into each member's own transform. The
    /// members take the group's place in z-order, in the order they were in it.
    ///
    /// # Errors
    /// Returns [`ShapeIsNotAGroup`](PptxError::ShapeIsNotAGroup) if the address is not a `p:grpSp`,
    /// or another [`PptxError`] if it is out of range or the part is malformed.
    pub fn ungroup(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Vec<ShapePath>, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        group::ungroup(root, interner, surface, &path)
    }

    /// Moves shape `shape_idx` into the group at `group_idx`, as its last member, and returns its
    /// new address.
    ///
    /// The shape does not move on screen: its transform is restated for the group's coordinate
    /// space, mirrors and rotation included, so joining a scaled, turned or flipped group leaves it
    /// exactly where it was.
    ///
    /// # Errors
    /// Returns [`ShapeIsNotAGroup`](PptxError::ShapeIsNotAGroup) if the destination is not a group,
    /// [`ShapeCannotContainItself`](PptxError::ShapeCannotContainItself) if the destination is the
    /// shape or something inside it, [`ShapeCannotBePlaced`](PptxError::ShapeCannotBePlaced) if the
    /// group states no child coordinate space, or another [`PptxError`] if an address is out of range
    /// or the part is malformed.
    pub fn move_shape_into_group(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        group_idx: impl Into<ShapePath>,
    ) -> Result<ShapePath, PptxError> {
        let surface = surface.into();
        let shape = shape_idx.into();
        let group = group_idx.into();
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        group::move_into_group(root, interner, surface, &shape, &group)
    }

    /// Moves shape `shape_idx` out of the group holding it, into that group's own container and
    /// directly after it in z-order. Returns its new address.
    ///
    /// The shape does not move on screen, as with
    /// [`move_shape_into_group`](Self::move_shape_into_group).
    ///
    /// # Errors
    /// Returns [`ShapeHasNoParent`](PptxError::ShapeHasNoParent) for a top-level shape, which is not
    /// inside anything, or another [`PptxError`] if the address is out of range or the part is
    /// malformed.
    pub fn move_shape_out_of_group(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<ShapePath, PptxError> {
        let surface = surface.into();
        let path = shape_idx.into();
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        group::move_out_of_group(root, interner, surface, &path)
    }

    /// What the graphic frame `shape_idx` on `surface` frames — a [`Table`](GraphicFrameKind::Table),
    /// a [`Chart`](GraphicFrameKind::Chart), a [`Diagram`](GraphicFrameKind::Diagram) or something
    /// else — or `None` when the shape is not a `p:graphicFrame` at all. Reading does not dirty the
    /// part.
    ///
    /// The table methods answer [`ShapeIsNotATable`](PptxError::ShapeIsNotATable) for a chart or
    /// diagram frame exactly as for a non-frame; this tells "not a table" from "a graphic this
    /// library does not model yet".
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn graphic_frame_kind(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<GraphicFrameKind>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        Ok(slide::graphic_frame_uri(shape, &doc.interner).map(GraphicFrameKind::from_uri))
    }
}

/// A whole `p:sp` text box: `nvSpPr` (`txBox="1"`) + `spPr` (`prst="rect"`) + a `txBody` with one
/// `a:p` per line of `text`.
fn build_text_box(interner: &mut Interner, id: u32, text: &str, bounds: ShapeBounds) -> RawElement {
    let nv_sp_pr = build_nv_sp_pr(interner, id, &format!("TextBox {id}"), true);
    let sp_pr = build_sp_pr(interner, "rect", bounds);

    // One a:p per line of text.
    let paragraphs = text
        .split('\n')
        .map(|line| build_paragraph(interner, line))
        .collect();
    let tx_body = build_text_body(interner, paragraphs);

    build::node(
        interner,
        "p",
        PML,
        "sp",
        Vec::new(),
        vec![
            RawNode::Element(nv_sp_pr),
            RawNode::Element(sp_pr),
            RawNode::Element(tx_body),
        ],
    )
}

/// A whole `p:sp` autoshape: `nvSpPr` (no `txBox`) + `spPr` with the `prst` preset geometry + an
/// empty `txBody` (`a:bodyPr`, `a:lstStyle`, one `a:p` holding one empty run — see
/// [`build_paragraph`]).
fn build_shape(interner: &mut Interner, id: u32, prst: &str, bounds: ShapeBounds) -> RawElement {
    let nv_sp_pr = build_nv_sp_pr(interner, id, &format!("Shape {id}"), false);
    let sp_pr = build_sp_pr(interner, prst, bounds);

    let empty_p = build_paragraph(interner, "");
    let tx_body = build_text_body(interner, vec![empty_p]);

    build::node(
        interner,
        "p",
        PML,
        "sp",
        Vec::new(),
        vec![
            RawNode::Element(nv_sp_pr),
            RawNode::Element(sp_pr),
            RawNode::Element(tx_body),
        ],
    )
}
