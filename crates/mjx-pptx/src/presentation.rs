//! The [`Presentation`] entry point: open, read shape text, edit a run, save.

use mjx_dml::{
    applicable_parts, resolve_character_properties, resolve_color, resolve_effects, resolve_fill,
    resolve_line, BlipFill, CellBorder, CharacterPropertiesSpec, ColorMap, ColorSpec, EffectList,
    EffectListSpec, Emu, Fill, FillSpec, FontSlot, IndentLevel, LineProperties, LineSpec,
    OnOffStyle, ParagraphProperties, ParagraphPropertiesSpec, PresetGeometry, ResolvedColor,
    SchemeColors, ShapeGeometry, Table, TableCell, TableCellProperties, TableColumn, TablePart,
    TablePartStyle, TableProperties, TableRow, TableStyle, TableStyleBorder, TableStyleCellStyle,
    TableStyleFlags, TableStyleList, TableStylePart, TableStyleTextStyle, TextAnchoring, TextBody,
    TextDirection, TextFont, TextListStyle, Theme, ThemeInfo, ThemeableLineStyle, Transform2D,
};
use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::drawingml::PresetShapeType;
use mjx_ooxml_types::namespaces::{DML_MAIN, PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_ooxml_types::presentationml::{
    Orientation, PlaceholderSize, PlaceholderType, SlideLayoutKind, SlideSizeKind,
};
use mjx_opc::{ImageFormat, Package, PartName, Relationship, TargetMode};

use crate::error::PptxError;
use crate::geometry::{CellMargins, ShapeBounds, SlideSize};
use crate::slide::GraphicFrameKind;
use crate::slide::{PlaceholderInfo, ShapeKind};
use crate::surface::Surface;
use crate::table::{CellFormat, Cells, TableStyleDefinition, TableStyleFormat};
use crate::{build, constants, nav, slide};

/// An open PresentationML document: an OPC [`Package`] plus its resolved presentation part and the
/// ordered list of slide parts.
///
/// Reads and edits are addressed by a [`Surface`] (a slide, layout, or master — a bare `usize` means
/// a slide) plus `shape_idx` / `run_idx`. Reading a part never dirties it; editing marks only that one
/// part dirty, so [`save`](Self::save) re-emits every other part byte-identically.
///
/// Editing a **layout or master** is how one change reaches many slides: a slide placeholder that
/// declares no property of its own inherits from the same-slot placeholder up its chain (see
/// [`effective_shape_fill`](Self::effective_shape_fill)).
#[derive(Debug)]
pub struct Presentation {
    package: Package,
    presentation_part: PartName,
    slides: Vec<PartName>,
    masters: Vec<PartName>,
    /// Every master's layouts, master by master (see [`Presentation::layout_count`]).
    layouts: Vec<PartName>,
    /// `layout_owners[i]` is the index in `masters` of the master that lists `layouts[i]`.
    layout_owners: Vec<usize>,
}

impl Presentation {
    /// Opens a presentation from its container bytes, resolving the presentation part and the ordered
    /// slide parts.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the package is unreadable, has no `officeDocument` relationship, or its
    /// `presentation.xml` / relationships are malformed.
    pub fn open(bytes: &[u8]) -> Result<Self, PptxError> {
        let mut package = Package::open(bytes)?;

        // Package root -> officeDocument relationship -> the presentation part.
        let presentation_part = {
            let root_rels = package
                .relationships_for(None)
                .ok_or(PptxError::MissingOfficeDocument)?;
            let rel = root_rels
                .by_type(constants::REL_OFFICE_DOCUMENT)
                .next()
                .ok_or(PptxError::MissingOfficeDocument)?;
            if rel.mode == TargetMode::External {
                return Err(PptxError::ExternalTarget {
                    target: rel.target.clone(),
                });
            }
            nav::resolve_from_root(&rel.target)?
        };
        if package.part_bytes(&presentation_part).is_none() {
            return Err(PptxError::MissingPresentationPart(
                presentation_part.as_str().to_owned(),
            ));
        }

        // presentation.xml -> p:sldIdLst -> each p:sldId's r:id -> the slide parts. A deck must have
        // the list (an empty one is fine); the same walk resolves masters and, per master, layouts.
        {
            let doc = package.part_tree(&presentation_part)?;
            if nav::child(&doc.root, &doc.interner, PML, "sldIdLst").is_none() {
                return Err(PptxError::MalformedPresentation("missing p:sldIdLst"));
            }
        }
        let slides = referenced_parts(&mut package, &presentation_part, "sldIdLst", "sldId")?;
        let masters = referenced_parts(
            &mut package,
            &presentation_part,
            "sldMasterIdLst",
            "sldMasterId",
        )?;

        // Each master lists its own layouts; the flat layout index runs master by master, in order.
        let mut layouts = Vec::new();
        let mut layout_owners = Vec::new();
        for (master_idx, master) in masters.iter().enumerate() {
            let master = master.clone();
            for layout in referenced_parts(&mut package, &master, "sldLayoutIdLst", "sldLayoutId")?
            {
                layouts.push(layout);
                layout_owners.push(master_idx);
            }
        }

        Ok(Self {
            package,
            presentation_part,
            slides,
            masters,
            layouts,
            layout_owners,
        })
    }

    /// Serializes the presentation back to container bytes (only edited parts re-serialize).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the ZIP writer fails.
    pub fn save(&self) -> Result<Vec<u8>, PptxError> {
        Ok(self.package.save()?)
    }

    /// The part name of the main presentation part (`/ppt/presentation.xml`).
    #[must_use]
    pub fn presentation_part(&self) -> &PartName {
        &self.presentation_part
    }

    /// The number of slides, in presentation order.
    #[must_use]
    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    /// The part name of slide `idx` (does not touch the package).
    #[must_use]
    pub fn slide_part(&self, idx: usize) -> Option<&PartName> {
        self.slides.get(idx)
    }

    /// The number of slide masters, in `p:sldMasterIdLst` order.
    #[must_use]
    pub fn master_count(&self) -> usize {
        self.masters.len()
    }

    /// The part name of master `idx` (does not touch the package).
    #[must_use]
    pub fn master_part(&self, idx: usize) -> Option<&PartName> {
        self.masters.get(idx)
    }

    /// The name of master `idx` (`p:cSld@name`, e.g. `Office Theme`), or `None` if it is unnamed.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the index is out of range or the master is malformed.
    pub fn master_name(&mut self, idx: usize) -> Result<Option<String>, PptxError> {
        let part = self.master_part_checked(idx)?.clone();
        self.common_slide_data_name(&part)
    }

    /// The number of slide layouts across the whole deck, in (master order, `p:sldLayoutIdLst` order)
    /// — so layout indices run master by master. [`layout_master`](Self::layout_master) says which
    /// master an index belongs to.
    ///
    /// A layout no master lists is not counted: layouts are reached through their master, as
    /// PowerPoint reaches them.
    #[must_use]
    pub fn layout_count(&self) -> usize {
        self.layouts.len()
    }

    /// The part name of layout `idx` (does not touch the package).
    #[must_use]
    pub fn layout_part(&self, idx: usize) -> Option<&PartName> {
        self.layouts.get(idx)
    }

    /// The index of the master that lists layout `idx`.
    #[must_use]
    pub fn layout_master(&self, idx: usize) -> Option<usize> {
        self.layout_owners.get(idx).copied()
    }

    /// The name of layout `idx` (`p:cSld@name`, e.g. `Title and Content` — the name PowerPoint shows
    /// in its layout gallery), or `None` if it is unnamed.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the index is out of range or the layout is malformed.
    pub fn layout_name(&mut self, idx: usize) -> Result<Option<String>, PptxError> {
        let part = self.layout_part_checked(idx)?.clone();
        self.common_slide_data_name(&part)
    }

    /// How layout `idx` arranges its content (`p:sldLayout@type`) — a coarse description of which
    /// placeholders it offers, which an application can use to map between layouts.
    ///
    /// Defaults to [`SlideLayoutKind::Custom`] when the attribute is absent (as the schema does) or
    /// names a value this build does not recognize.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the index is out of range or the layout is malformed.
    pub fn layout_kind(&mut self, idx: usize) -> Result<SlideLayoutKind, PptxError> {
        let part = self.layout_part_checked(idx)?.clone();
        let doc = self.package.part_tree(&part)?;
        Ok(nav::attr_value(&doc.root, &doc.interner, "type")
            .and_then(SlideLayoutKind::from_wire)
            .unwrap_or(SlideLayoutKind::Custom))
    }

    /// The index of the layout slide `slide_idx` is built on, or `None` if the slide relates to no
    /// layout (or to one no master lists).
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range or the relationship points outside the
    /// package.
    pub fn slide_layout(&self, slide_idx: usize) -> Result<Option<usize>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?;
        let Some(layout) = self.follow_rel(slide_part, constants::REL_SLIDE_LAYOUT)? else {
            return Ok(None);
        };
        Ok(self.layouts.iter().position(|part| *part == layout))
    }

    /// The size of every slide in the deck (`p:sldSz`) — the extent shape bounds are laid out in.
    ///
    /// # Errors
    /// Returns [`PptxError::MalformedPresentation`] if `p:sldSz` is missing or its extent attributes
    /// are absent or unparseable.
    pub fn slide_size(&mut self) -> Result<SlideSize, PptxError> {
        let part = self.presentation_part.clone();
        let doc = self.package.part_tree(&part)?;
        let sld_sz = nav::child(&doc.root, &doc.interner, PML, "sldSz")
            .ok_or(PptxError::MalformedPresentation("missing p:sldSz"))?;
        let extent = |local| {
            nav::attr_value(sld_sz, &doc.interner, local)
                .and_then(|value| value.parse::<i64>().ok())
                .ok_or(PptxError::MalformedPresentation("p:sldSz has no extent"))
        };
        Ok(SlideSize {
            width_emu: extent("cx")?,
            height_emu: extent("cy")?,
            kind: nav::attr_value(sld_sz, &doc.interner, "type")
                .and_then(SlideSizeKind::from_wire)
                .unwrap_or(SlideSizeKind::Custom),
        })
    }

    /// The `p:cSld@name` of a slide-bearing part (master, layout, or slide).
    fn common_slide_data_name(&mut self, part: &PartName) -> Result<Option<String>, PptxError> {
        let doc = self.package.part_tree(part)?;
        let c_sld = nav::child(&doc.root, &doc.interner, PML, "cSld")
            .ok_or(PptxError::MalformedSlide("missing p:cSld"))?;
        Ok(nav::attr_value(c_sld, &doc.interner, "name")
            .filter(|name| !name.is_empty())
            .map(str::to_owned))
    }

    fn master_part_checked(&self, idx: usize) -> Result<&PartName, PptxError> {
        self.masters
            .get(idx)
            .ok_or(PptxError::MasterIndexOutOfRange {
                index: idx,
                count: self.masters.len(),
            })
    }

    fn layout_part_checked(&self, idx: usize) -> Result<&PartName, PptxError> {
        self.layouts
            .get(idx)
            .ok_or(PptxError::LayoutIndexOutOfRange {
                index: idx,
                count: self.layouts.len(),
            })
    }

    /// The part a [`Surface`] addresses, or the typed error for its kind (index out of range, or a
    /// notes surface the deck does not have).
    ///
    /// A slide/layout/master part is stored, so this clones a name out of the owning `Vec`; a notes
    /// part is resolved lazily by relationship. Either way the result is owned, which is what every
    /// caller needs — none holds the borrow across the package edit that follows.
    fn surface_part(&self, surface: Surface) -> Result<PartName, PptxError> {
        match surface {
            Surface::Slide(idx) => self.slide_part_checked(idx).cloned(),
            Surface::Layout(idx) => self.layout_part_checked(idx).cloned(),
            Surface::Master(idx) => self.master_part_checked(idx).cloned(),
            Surface::Notes(slide) => self
                .notes_part(slide)?
                .ok_or(PptxError::SurfaceHasNoNotes { slide }),
            Surface::NotesMaster => self
                .notes_master_part()?
                .ok_or(PptxError::SurfaceHasNoNotesMaster),
        }
    }

    /// The notes slide part of slide `slide_idx`, or `None` if the slide owns none. Reading does not
    /// touch the package tree.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range or the relationship points outside the
    /// package.
    fn notes_part(&self, slide_idx: usize) -> Result<Option<PartName>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        self.follow_rel(&slide_part, constants::REL_NOTES_SLIDE)
    }

    /// The presentation's notes master part, or `None` if the deck has none.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the relationship points outside the package.
    fn notes_master_part(&self) -> Result<Option<PartName>, PptxError> {
        self.follow_rel(&self.presentation_part, constants::REL_NOTES_MASTER)
    }

    /// The speaker notes of slide `slide_idx` — the text of its notes slide's `body` placeholder — or
    /// `None` if the slide has no notes slide (or its notes slide has no body placeholder).
    ///
    /// This is the ergonomic read: it addresses the body placeholder **by kind**, so a caller never
    /// has to know its shape index. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range, a relationship points outside the
    /// package, or the notes slide is malformed.
    pub fn notes_text(&mut self, slide_idx: usize) -> Result<Option<String>, PptxError> {
        let Some(notes_part) = self.notes_part(slide_idx)? else {
            return Ok(None);
        };
        let Some(body_idx) = self.notes_body_index(&notes_part)? else {
            return Ok(None);
        };
        Ok(Some(self.shape_text(Surface::Notes(slide_idx), body_idx)?))
    }

    /// Sets the speaker notes of slide `slide_idx` to `text`, creating the notes slide (and, if the
    /// deck has none, the notes master it follows) on demand.
    ///
    /// The body placeholder's whole text body is replaced with a single paragraph holding `text`; any
    /// prior notes text and its run formatting are discarded. To remove notes entirely, use
    /// [`clear_notes`](Self::clear_notes) rather than passing an empty string.
    ///
    /// When the notes slide is created, exactly the new notes slide part, its `.rels`, the
    /// slide → notes-slide relationship and the content-type override are added (plus the notes master
    /// and its wiring if the deck had none); every pre-existing part stays byte-identical.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range or a package edit fails.
    pub fn set_notes_text(&mut self, slide_idx: usize, text: &str) -> Result<(), PptxError> {
        let notes_part = self.ensure_notes_slide_part(slide_idx)?;
        let body_idx = self.notes_body_index(&notes_part)?.ok_or(
            PptxError::MalformedSlide("notes slide has no body placeholder"),
        )?;

        let doc = self.package.part_tree_mut(&notes_part)?;
        let RawDocument { interner, root, .. } = doc;
        let paragraph = build_paragraph(interner, text);
        let new_body = build_text_body(interner, vec![paragraph]);
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let shape = slide::nth_shape_mut(sp_tree, interner, body_idx)
            .ok_or(PptxError::MalformedSlide("notes body placeholder vanished"))?;
        replace_txbody(shape, interner, new_body);
        Ok(())
    }

    /// Removes the speaker notes of slide `slide_idx`: unwires the slide → notes-slide relationship and
    /// removes the notes slide part (with its `.rels` and content-type override). A no-op if the slide
    /// has no notes.
    ///
    /// The notes master and the slide survive — the presentation still references both — so only the
    /// notes slide and its own subtree go. After this, [`notes_text`](Self::notes_text) reads `None`.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range, the slide's wiring is malformed, or a
    /// package edit fails.
    pub fn clear_notes(&mut self, slide_idx: usize) -> Result<(), PptxError> {
        let Some(notes_part) = self.notes_part(slide_idx)? else {
            return Ok(());
        };
        let slide_part = self.slide_part_checked(slide_idx)?.clone();

        // The slide's relationship naming this notes slide — matched by resolved target, as
        // `remove_slide` matches the presentation's relationship to a slide.
        let rel_id = {
            let rels = self
                .package
                .relationships_for(Some(&slide_part))
                .ok_or(PptxError::MalformedSlide("slide has no relationships"))?;
            rels.by_type(constants::REL_NOTES_SLIDE)
                .find(|rel| {
                    rel.mode == TargetMode::Internal
                        && nav::resolve_target(&slide_part, &rel.target)
                            .is_ok_and(|resolved| resolved == notes_part)
                })
                .map(|rel| rel.id.clone())
                .ok_or(PptxError::MalformedSlide(
                    "no relationship names the notes slide",
                ))?
        };
        self.package.remove_relationship(Some(&slide_part), &rel_id)?;
        self.package.remove_part_cascading(&notes_part)?;
        Ok(())
    }

    /// The shape index of the `body` placeholder on a notes slide part — where the speaker's text
    /// lives — or `None` if the notes slide has no body placeholder. Matched **by kind**, since a
    /// notes slide has exactly one body placeholder.
    fn notes_body_index(&mut self, notes_part: &PartName) -> Result<Option<usize>, PptxError> {
        let doc = self.package.part_tree(notes_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        Ok(slide::shapes(sp_tree, &doc.interner).position(|shape| {
            slide::shape_placeholder(shape, &doc.interner)
                .is_some_and(|ph| ph.kind == PlaceholderType::Body)
        }))
    }

    /// The notes slide part of slide `slide_idx`, creating it (and the notes master it follows, if the
    /// deck has none) when the slide has no notes.
    ///
    /// The new notes slide relates *back* to its slide and *up* to the notes master, and the slide
    /// gains a relationship to it. Mirrors [`insert_slide_part`](Self::insert_slide_part).
    fn ensure_notes_slide_part(&mut self, slide_idx: usize) -> Result<PartName, PptxError> {
        if let Some(part) = self.notes_part(slide_idx)? {
            return Ok(part);
        }
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        let notes_master = self.ensure_notes_master()?;
        let notes_part = self.next_notes_slide_part()?;

        // 1. Insert the notes slide part (registers its content-type Override).
        self.package.insert_part(
            &notes_part,
            constants::CONTENT_TYPE_NOTES_SLIDE,
            build::empty_notes_slide_bytes(),
        )?;
        // 2. Synthesize its .rels: up to the notes master, and back to the slide.
        self.package.add_relationship(
            Some(&notes_part),
            Relationship {
                id: "rId1".to_owned(),
                rel_type: constants::REL_NOTES_MASTER.to_owned(),
                target: nav::relative_target(&notes_part, &notes_master),
                mode: TargetMode::Internal,
            },
        )?;
        self.package.add_relationship(
            Some(&notes_part),
            Relationship {
                id: "rId2".to_owned(),
                rel_type: constants::REL_SLIDE.to_owned(),
                target: nav::relative_target(&notes_part, &slide_part),
                mode: TargetMode::Internal,
            },
        )?;
        // 3. Add the slide → notes-slide relationship.
        let rel_id = self.next_rid_for(&slide_part);
        self.package.add_relationship(
            Some(&slide_part),
            Relationship {
                id: rel_id,
                rel_type: constants::REL_NOTES_SLIDE.to_owned(),
                target: nav::relative_target(&slide_part, &notes_part),
                mode: TargetMode::Internal,
            },
        )?;
        Ok(notes_part)
    }

    /// The presentation's notes master part, creating `ppt/notesMasters/notesMaster1.xml` (with its
    /// theme relationship, presentation relationship, content-type override, and `p:notesMasterIdLst`
    /// entry) if the deck has none. Mirrors [`ensure_table_styles_part`](Self::ensure_table_styles_part).
    fn ensure_notes_master(&mut self) -> Result<PartName, PptxError> {
        if let Some(part) = self.notes_master_part()? {
            return Ok(part);
        }
        let part = PartName::new(&format!(
            "{}notesMasters/notesMaster1.xml",
            dir_of(self.presentation_part.as_str())
        ))?;
        self.package.insert_part(
            &part,
            constants::CONTENT_TYPE_NOTES_MASTER,
            build::notes_master_bytes(),
        )?;
        // A theme relationship, reusing the deck's theme (a notes master always follows one).
        if let Some(theme) = self.deck_theme_part()? {
            let rel_id = self.next_rid_for(&part);
            self.package.add_relationship(
                Some(&part),
                Relationship {
                    id: rel_id,
                    rel_type: constants::REL_THEME.to_owned(),
                    target: nav::relative_target(&part, &theme),
                    mode: TargetMode::Internal,
                },
            )?;
        }
        // The presentation → notes-master relationship, and the `p:notesMasterId` naming it.
        let rel_id = self.next_presentation_rid()?;
        self.package.add_relationship(
            Some(&self.presentation_part),
            Relationship {
                id: rel_id.clone(),
                rel_type: constants::REL_NOTES_MASTER.to_owned(),
                target: nav::relative_target(&self.presentation_part, &part),
                mode: TargetMode::Internal,
            },
        )?;
        self.insert_notes_master_id(&rel_id)?;
        Ok(part)
    }

    /// A theme part to hang a synthesized notes master on: the presentation's own theme, else the
    /// first slide master's. `None` only in a deck with no theme at all.
    fn deck_theme_part(&self) -> Result<Option<PartName>, PptxError> {
        if let Some(theme) = self.follow_rel(&self.presentation_part, constants::REL_THEME)? {
            return Ok(Some(theme));
        }
        match self.masters.first() {
            Some(master) => self.follow_rel(&master.clone(), constants::REL_THEME),
            None => Ok(None),
        }
    }

    /// Inserts `<p:notesMasterIdLst><p:notesMasterId r:id="new_rid"/></p:notesMasterIdLst>` into
    /// `presentation.xml` in schema order — immediately after `p:sldMasterIdLst`.
    fn insert_notes_master_id(&mut self, new_rid: &str) -> Result<(), PptxError> {
        let part = self.presentation_part.clone();
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;

        let rels_prefix = nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)
            .ok_or(PptxError::MalformedPresentation(
                "no relationships namespace declared",
            ))?;
        let id_attr = build::attr_prefixed(interner, rels_prefix, "id", new_rid);
        let notes_master_id = build::leaf(interner, "p", PML, "notesMasterId", vec![id_attr]);
        let lst = build::node(
            interner,
            "p",
            PML,
            "notesMasterIdLst",
            Vec::new(),
            vec![RawNode::Element(notes_master_id)],
        );

        // Schema order is sldMasterIdLst → notesMasterIdLst → handoutMasterIdLst → sldIdLst. Place it
        // right after the slide-master list; fall back to before the slide-id list, then to the end.
        let after_masters = root.children.iter().position(|child| {
            matches!(child, RawNode::Element(e) if nav::name_is(&e.name, interner, PML, "sldMasterIdLst"))
        });
        let before_slides = || {
            root.children.iter().position(|child| {
                matches!(child, RawNode::Element(e) if nav::name_is(&e.name, interner, PML, "sldIdLst"))
            })
        };
        let pos = after_masters
            .map(|i| i + 1)
            .or_else(before_slides)
            .unwrap_or(root.children.len());
        root.children.insert(pos, RawNode::Element(lst));
        root.empty = false;
        Ok(())
    }

    /// A fresh notes slide part name: `notesSlides/notesSlide{N}.xml` beside the presentation part,
    /// with `N` one past the largest existing notes slide number.
    fn next_notes_slide_part(&self) -> Result<PartName, PptxError> {
        let dir = format!("{}notesSlides/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = notes_slide_number(part.as_str(), &dir) {
                max_n = max_n.max(n);
            }
        }
        PartName::new(&format!("{dir}notesSlide{}.xml", max_n + 1)).map_err(PptxError::from)
    }

    /// The parts a surface inherits from, nearest first: the surface's own part, then the parts a
    /// placeholder on it falls back to — a slide resolves through its layout then that layout's
    /// master, a layout through its master, a master stands alone.
    ///
    /// This is the spine of every "effective" property: the same chain decides where an inherited
    /// fill, outline, or effect comes from, and (via its last element) which theme applies.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range or a relationship points outside
    /// the package.
    fn inheritance_chain(&self, surface: Surface) -> Result<Vec<PartName>, PptxError> {
        let own = self.surface_part(surface)?;
        let mut chain = vec![own];

        // A notes slide follows the notes master directly (there is no notes layout); every other
        // non-master surface climbs the slide → layout → slide-master spine.
        if matches!(surface, Surface::Notes(_)) {
            if let Some(master) = self.follow_rel(&chain[0], constants::REL_NOTES_MASTER)? {
                chain.push(master);
            }
            return Ok(chain);
        }

        if matches!(surface, Surface::Slide(_)) {
            let Some(layout) = self.follow_rel(&chain[0], constants::REL_SLIDE_LAYOUT)? else {
                return Ok(chain);
            };
            chain.push(layout);
        }
        if !surface.is_master_like() {
            let last = chain.last().expect("the chain always holds the own part");
            if let Some(master) = self.follow_rel(last, constants::REL_SLIDE_MASTER)? {
                chain.push(master);
            }
        }
        Ok(chain)
    }

    /// The shapes an **effective** property consults, in inheritance order: the addressed shape
    /// itself, then — only if it is a placeholder (`p:ph`) — the same-slot placeholder on each part
    /// the surface inherits from.
    ///
    /// This is the spine every `effective_*` property walks. A shape that is not a placeholder
    /// inherits nothing and yields a one-element list, which is why a plain text box never takes a
    /// layout's fill, outline, effects or position.
    ///
    /// The parts are returned by name rather than borrowed, so a caller can visit each in turn
    /// without holding a borrow on the package across the walk.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the shape index is out of range, the surface's part is malformed, or
    /// a relationship in the chain points outside the package.
    fn placeholder_candidates(
        &mut self,
        surface: Surface,
        shape_idx: usize,
    ) -> Result<Vec<(PartName, Candidate)>, PptxError> {
        let own_part = self.surface_part(surface)?;
        let placeholder = {
            let doc = self.package.part_tree(&own_part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            let count = slide::shapes(sp_tree, &doc.interner).count();
            let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
                PptxError::ShapeIndexOutOfRange {
                    surface,
                    index: shape_idx,
                    count,
                },
            )?;
            slide::shape_placeholder(shape, &doc.interner)
        };

        let mut candidates = vec![(own_part, Candidate::Index(shape_idx))];
        if let Some(ph) = placeholder {
            // The rest of the surface's inheritance chain, each searched for the same-slot placeholder.
            for ancestor in self.inheritance_chain(surface)?.into_iter().skip(1) {
                candidates.push((ancestor, Candidate::Placeholder(ph)));
            }
        }
        Ok(candidates)
    }

    /// The number of shapes on `surface` — of **every** [`ShapeKind`] (autoshapes, pictures,
    /// groups, graphic frames, connectors), in document order. A group counts as one shape; its
    /// members are not separately addressable.
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
        shape_idx: usize,
    ) -> Result<ShapeKind, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        slide::shape_kind(shape, &doc.interner)
            .ok_or(PptxError::MalformedSlide("shape tree child is not a shape"))
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
        shape_idx: usize,
    ) -> Result<Option<PlaceholderInfo>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        Ok(slide::shape_placeholder_info(shape, &doc.interner))
    }

    /// The full text of shape `shape_idx` on `surface` (paragraphs joined by `\n`).
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body ([`ShapeHasNoTextBody`](PptxError::ShapeHasNoTextBody) — a picture or group never
    /// has one).
    pub fn shape_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<String, PptxError> {
        let surface = surface.into();
        self.with_text_body(surface, shape_idx, |body, _| Ok(body.text()))
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the shape's paragraphs, in document
    /// order) of shape `shape_idx` on `surface`. Marks only that part dirty.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, the shape has no
    /// text body, or the selected run has no `a:t`.
    pub fn set_shape_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        run_idx: usize,
        text: &str,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        self.edit_text_body(surface, shape_idx, |body, _| {
            set_run_text(body, run_idx, text)
        })
    }

    /// Reads a shape's text body and hands it, with the part's interner, to `read`. Does **not**
    /// dirty the part.
    fn with_text_body<R>(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        read: impl FnOnce(&TextBody, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        self.with_text_body_at(surface, TextSite::Shape(shape_idx), read)
    }

    /// Locates a shape's text body, hands it to `edit`, and writes the result back.
    fn edit_text_body(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        edit: impl FnOnce(&mut TextBody, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(surface, TextSite::Shape(shape_idx), edit)
    }

    /// Reads the text body at `site` and hands it, with the part's interner, to `read` — for the
    /// accessors that need the interner to resolve what they return. Does **not** dirty the part.
    ///
    /// The interner is borrowed rather than cloned: a part's interner holds every string in it, and
    /// copying that per property read would be absurd.
    fn with_text_body_at<R>(
        &mut self,
        surface: Surface,
        site: TextSite,
        read: impl FnOnce(&TextBody, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner)
            .nth(site.shape_index())
            .ok_or(PptxError::ShapeIndexOutOfRange {
                surface,
                index: site.shape_index(),
                count,
            })?;
        let txbody = locate_text_body(shape, &doc.interner, site)?;
        let body = TextBody::from_xml(txbody, &doc.interner)?;
        read(&body, &doc.interner)
    }

    /// Locates the text body at `site`, hands it to `edit`, and writes the result back — the one
    /// place every text-editing call shares, so the split borrow and the rebuild happen once.
    ///
    /// Marks only that part dirty, and only when `edit` succeeds is the body written back. Only the
    /// addressed `a:txBody` is parsed and rebuilt: reaching a table cell walks the raw tree rather
    /// than parsing the whole table, so editing one cell costs the same as editing a shape.
    fn edit_text_body_at(
        &mut self,
        surface: Surface,
        site: TextSite,
        edit: impl FnOnce(&mut TextBody, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        // Split the borrow: `interner` for names and rebuilding, `root` for locate + replace.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, site.shape_index()).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: site.shape_index(),
                count,
            },
        )?;
        let slot = locate_text_body_mut(shape, interner, site)?;

        let mut body = TextBody::from_xml(slot, interner)?;
        edit(&mut body, interner)?;
        *slot = body.to_xml(interner);
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Text formatting — the paragraph axis
    //
    // `set_shape_text` above addresses runs *flat* across the whole body, which is the shorthand for
    // the common one-paragraph case. Everything below addresses a paragraph first and a run within
    // it, matching the document tree — and matching what a user selects.
    // -----------------------------------------------------------------------------------------

    /// The number of paragraphs in shape `shape_idx`'s text body. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            Ok(paragraph_count_of(body))
        })
    }

    /// The number of runs in paragraph `para_idx` of shape `shape_idx`. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn run_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            run_count_of(body, para_idx)
        })
    }

    /// The text of paragraph `para_idx` — its runs concatenated. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            paragraph_text_of(body, para_idx)
        })
    }

    /// The text of one run. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn run_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, _| {
            run_text_of(body, para_idx, run_idx)
        })
    }

    /// The layout properties a paragraph declares of its own (`a:pPr`), or `None` if it declares
    /// none — in which case every property is inherited. Reading does not dirty the part.
    ///
    /// This is what the paragraph *says*, not what it renders as: a property left unset here is
    /// inherited from the shape's list style, the placeholder, the layout, the master and the theme,
    /// in that order.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
    ) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            paragraph_properties_of(body, interner, para_idx)
        })
    }

    /// The character properties a run declares of its own (`a:rPr`), or `None` if it declares none.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            run_properties_of(body, interner, para_idx, run_idx)
        })
    }

    /// The paragraph-mark properties (`a:endParaRPr`), or `None` if the paragraph declares none.
    ///
    /// This is the format an **empty** paragraph holds — what keeps a blank line its size, and what
    /// text typed into it would take on. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body(surface.into(), shape_idx, |body, interner| {
            end_run_properties_of(body, interner, para_idx)
        })
    }

    /// Applies `spec` to one run's character properties, creating its `a:rPr` if it has none.
    ///
    /// The properties **merge**: what the spec names is set, and everything else the run carried —
    /// including the state this model does not describe, like `lang` or `dirty` — is left alone.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        run_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_run_properties_in(body, interner, para_idx, run_idx, spec)
        })
    }

    /// Applies `spec` to **every run** in paragraph `para_idx`, and to its `a:endParaRPr` if it has
    /// one — so text typed at the end of the paragraph takes the same formatting, which is what
    /// selecting a paragraph and restyling it means.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_paragraph_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_paragraph_run_properties_in(body, interner, para_idx, spec)
        })
    }

    /// Applies `spec` to **every run of every paragraph** in the shape, and to each paragraph's
    /// `a:endParaRPr` where present — selecting a whole text box and restyling it.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_shape_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_all_run_properties_in(body, interner, spec)
        })
    }

    /// Applies `spec` to the paragraph-mark properties (`a:endParaRPr`), creating the element if the
    /// paragraph has none.
    ///
    /// This is how an **empty** paragraph is formatted — a placeholder that has been added but not
    /// yet typed into, for instance.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_end_run_properties_in(body, interner, para_idx, spec)
        })
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if it has
    /// none. The properties **merge**, as run properties do.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// text body.
    pub fn set_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_paragraph_properties_in(body, interner, para_idx, spec)
        })
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **Unicode
    /// scalars** across the paragraph's whole text.
    ///
    /// A run boundary is where formatting changes, so formatting part of a run **splits** it: after
    /// this call the paragraph holds up to two more runs than before, and only those inside `range`
    /// carry `spec`. A range that already lines up with run boundaries splits nothing. Runs are never
    /// merged back together, so the file changes only where it had to.
    ///
    /// For a range taken from a real text selection, prefer
    /// [`set_text_range_properties_by_grapheme`](Self::set_text_range_properties_by_grapheme):
    /// scalar offsets can fall inside a grapheme cluster, splitting an emoji from its modifier.
    ///
    /// # Errors
    /// Returns [`PptxError::TextRangeOutOfBounds`] if the range ends before it starts or runs past
    /// the paragraph's text, or another [`PptxError`] if an index is out of range, the slide is
    /// malformed, or the shape has no text body.
    pub fn set_text_range_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        range: core::ops::Range<usize>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body(surface.into(), shape_idx, |body, interner| {
            set_range_properties_in(body, interner, para_idx, range, spec)
        })
    }

    /// Applies `spec` to part of a paragraph — the characters in `range`, counted in **grapheme
    /// clusters**: what a reader would call characters, and what a text selection actually spans.
    ///
    /// `👍🏽` is one grapheme (two scalars), so a range that covers it cannot split it in half. The
    /// offsets are converted to scalars and the work is done by
    /// [`set_text_range_properties`](Self::set_text_range_properties).
    ///
    /// # Errors
    /// As [`set_text_range_properties`](Self::set_text_range_properties), with the bounds reported in
    /// graphemes.
    pub fn set_text_range_properties_by_grapheme(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        range: core::ops::Range<usize>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let text = self.paragraph_text(surface, shape_idx, para_idx)?;
        let scalars = grapheme_range_to_scalars(&text, &range)?;
        self.set_text_range_properties(surface, shape_idx, para_idx, scalars, spec)
    }

    // -----------------------------------------------------------------------------------------
    // Text in a table cell
    //
    // A cell's `a:txBody` is the same `CT_TextBody` as a shape's `p:txBody`, so every one of these
    // is the corresponding shape method addressed at a cell instead — same operation, same errors,
    // same guarantees. The pair `(row, column)` addresses the cell; everything after it means what
    // it means on a shape.
    //
    // A cell covered by a merge still holds its own text body, and these reach it. Ask
    // `merged_cell_anchor` which cell actually renders at a position before reading text from one.
    // -----------------------------------------------------------------------------------------

    /// The text of the cell at `(row, column)` — its paragraphs joined by newlines.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotATable`] if the shape frames no table,
    /// [`PptxError::TableCellOutOfRange`] if there is no such cell, or another [`PptxError`] if an
    /// index is out of range, the part is malformed, or the cell has no text body.
    pub fn cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            Ok(body.text())
        })
    }

    /// The text that actually **renders** at `(row, column)` — the text of the cell if it stands
    /// alone, or of the merge **anchor** covering it if it is merged away.
    ///
    /// [`cell_text`](Self::cell_text) returns a covered cell's own (hidden) text, which is what an
    /// unmerge restores; this follows the merge to what a reader sees. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text), plus [`PptxError::TableCellOutOfRange`].
    pub fn visible_cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<String, PptxError> {
        let surface = surface.into();
        let (anchor_row, anchor_column) =
            self.merged_cell_anchor(surface, shape_idx, row, column)?;
        self.cell_text(surface, shape_idx, anchor_row, anchor_column)
    }

    /// Replaces the text of the `run_idx`-th run (flattened over the cell's paragraphs) of the cell
    /// at `(row, column)`. Marks only that part dirty.
    ///
    /// A cell created by [`add_table`](Self::add_table) has one empty run, so `run_idx` is `0` for
    /// the common case of filling in a fresh table.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text), plus [`PptxError::RunHasNoText`] if the selected run has
    /// no `a:t`.
    pub fn set_cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        run_idx: usize,
        text: &str,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            set_run_text(body, run_idx, text)
        })
    }

    /// The number of paragraphs in the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_paragraph_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            Ok(paragraph_count_of(body))
        })
    }

    /// The number of runs in one paragraph of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_run_count(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<usize, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            run_count_of(body, para_idx)
        })
    }

    /// The text of one paragraph of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_paragraph_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            paragraph_text_of(body, para_idx)
        })
    }

    /// The text of one run of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_run_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<String, PptxError> {
        self.with_text_body_at(surface.into(), cell(shape_idx, row, column), |body, _| {
            run_text_of(body, para_idx, run_idx)
        })
    }

    /// The layout properties a paragraph of the cell at `(row, column)` declares of its own.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
        self.with_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| paragraph_properties_of(body, interner, para_idx),
        )
    }

    /// The character properties a run of the cell at `(row, column)` declares of its own.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| run_properties_of(body, interner, para_idx, run_idx),
        )
    }

    /// The paragraph-mark properties (`a:endParaRPr`) of a paragraph of the cell at `(row, column)`
    /// — the format an empty cell holds, and what text typed into it would take on.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
    ) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
        self.with_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| end_run_properties_of(body, interner, para_idx),
        )
    }

    /// Applies `spec` to one run of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    // The deepest cell addresses take eight parameters: a surface, the frame, a cell's row and
    // column, a paragraph, a run, and the spec — every one of them a distinct coordinate, and the
    // price of addressing a cell with plain indices rather than a handle object. Bundling them into
    // an address struct would be the only way to shorten the list, and would make the common calls
    // read worse than the deep ones read now.
    #[allow(clippy::too_many_arguments)]
    pub fn set_cell_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_run_properties_in(body, interner, para_idx, run_idx, spec),
        )
    }

    /// Applies `spec` to **every run** of one paragraph of the cell at `(row, column)`, and to its
    /// paragraph mark.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_paragraph_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_paragraph_run_properties_in(body, interner, para_idx, spec),
        )
    }

    /// Applies `spec` to **every run of every paragraph** of the cell at `(row, column)` — what
    /// selecting a whole cell and restyling it means, and the usual way to make a header bold.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_run_properties_all(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_all_run_properties_in(body, interner, spec),
        )
    }

    /// Applies `spec` to a paragraph mark (`a:endParaRPr`) of the cell at `(row, column)`, creating
    /// the element if the paragraph has none — how an **empty** cell is formatted.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_end_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_end_run_properties_in(body, interner, para_idx, spec),
        )
    }

    /// Applies `spec` to a paragraph's layout properties (`a:pPr`) in the cell at `(row, column)`,
    /// creating the element if it has none. The properties **merge**, as run properties do.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_paragraph_properties_in(body, interner, para_idx, spec),
        )
    }

    /// Applies `spec` to part of a paragraph of the cell at `(row, column)` — the characters in
    /// `range`, counted in **Unicode scalars**. Splits runs at the range's edges, exactly as the
    /// shape-addressed form does.
    ///
    /// # Errors
    /// As [`set_text_range_properties`](Self::set_text_range_properties), plus the table errors of
    /// [`cell_text`](Self::cell_text).
    // The deepest cell addresses take eight parameters: a surface, the frame, a cell's row and
    // column, a paragraph, a run, and the spec — every one of them a distinct coordinate, and the
    // price of addressing a cell with plain indices rather than a handle object. Bundling them into
    // an address struct would be the only way to shorten the list, and would make the common calls
    // read worse than the deep ones read now.
    #[allow(clippy::too_many_arguments)]
    pub fn set_cell_text_range_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        range: core::ops::Range<usize>,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_text_body_at(
            surface.into(),
            cell(shape_idx, row, column),
            |body, interner| set_range_properties_in(body, interner, para_idx, range, spec),
        )
    }

    // -----------------------------------------------------------------------------------------
    // Cell formatting — what actually draws
    //
    // A cell's fill and its six borders are the same `EG_FillProperties` and `CT_LineProperties`
    // a shape uses, so `FillSpec` and `LineSpec` carry them unchanged; only the element's tag
    // differs, which is why one `LineSpec` serves all six edges.
    //
    // Everything here writes into `a:tcPr`, creating it when the cell has none. An unstated value
    // reads as `None` rather than as the schema default, because the two are different facts: the
    // margins default to 0.1"/0.05", not to zero.
    // -----------------------------------------------------------------------------------------

    /// The fill the cell at `(row, column)` declares, or `None` when it declares none — in which
    /// case the table style decides. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties
                    .and_then(|properties| properties.fill(interner))
                    .map(|fill| fill.spec(interner)))
            },
        )
    }

    /// Fills the cell at `(row, column)`. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        fill: &FillSpec,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_fill(interner, Some(fill));
                Ok(())
            },
        )
    }

    /// Removes the cell's own fill, so the table style decides how it is filled again.
    ///
    /// This is **not** the same as filling it with [`FillSpec::None`], which states *no fill at all*
    /// and blocks the style.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn clear_cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_fill(interner, None);
                Ok(())
            },
        )
    }

    /// The border the cell at `(row, column)` declares on `edge`, or `None` if it declares none
    /// there. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties
                    .and_then(|properties| properties.border(interner, edge))
                    .map(|line| line.spec(interner)))
            },
        )
    }

    /// Draws a border on one edge of the cell at `(row, column)`. Marks only that part dirty.
    ///
    /// The five other edges are untouched: each is its own element, and this writes one of them.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    #[allow(clippy::too_many_arguments)]
    pub fn set_cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        edge: CellBorder,
        line: &LineSpec,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_border(interner, edge, Some(line));
                Ok(())
            },
        )
    }

    /// The ids of the header cells that describe the cell at `(row, column)` (`a:tcPr > a:headers`),
    /// in order — the accessibility association a screen reader announces. Empty when the cell names
    /// none. Reading does not dirty the part.
    ///
    /// Each id is another cell's `@id`; a table that uses headers gives its header cells ids and
    /// points each data cell at the ones above and beside it.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_headers(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<Vec<String>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties.map_or_else(Vec::new, |properties| properties.headers(interner)))
            },
        )
    }

    /// Sets the header-cell ids that describe the cell at `(row, column)`, replacing whatever it had;
    /// an empty slice removes the association. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_headers(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        header_ids: &[&str],
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_headers(interner, header_ids);
                Ok(())
            },
        )
    }

    /// Removes the border on one edge of the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn clear_cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        edge: CellBorder,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_border(interner, edge, None);
                Ok(())
            },
        )
    }

    /// The four insets between the cell's edges and its text, each `None` when the cell does not
    /// state it. Reading does not dirty the part.
    ///
    /// An unstated margin is **not** a zero one — the schema defaults are `0.1"` horizontally and
    /// `0.05"` vertically, exposed as
    /// [`TableCellProperties::DEFAULT_MARGIN_HORIZONTAL`](mjx_dml::TableCellProperties::DEFAULT_MARGIN_HORIZONTAL)
    /// and its vertical counterpart.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_margins(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<CellMargins, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                let Some(properties) = properties else {
                    return Ok(CellMargins::default());
                };
                Ok(CellMargins {
                    left: properties.left_margin(interner),
                    right: properties.right_margin(interner),
                    top: properties.top_margin(interner),
                    bottom: properties.bottom_margin(interner),
                })
            },
        )
    }

    /// Sets the cell's insets. Each field left `None` is **not written**, so a caller can set one
    /// margin without stating the other three.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_margins(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        margins: CellMargins,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_margins(
                    interner,
                    margins.left,
                    margins.right,
                    margins.top,
                    margins.bottom,
                );
                Ok(())
            },
        )
    }

    /// Where the text sits vertically in the cell at `(row, column)`, or `None` if unstated (the
    /// wire default is [`TextAnchoring::Top`]). Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_anchor(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<Option<TextAnchoring>, PptxError> {
        self.with_cell_properties(surface.into(), shape_idx, row, column, |properties, interner| {
            Ok(properties.and_then(|properties| properties.anchor(interner)))
        })
    }

    /// Sets where the text sits vertically in the cell at `(row, column)`.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_anchor(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        anchor: TextAnchoring,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_anchor(interner, anchor);
                Ok(())
            },
        )
    }

    /// Which way the text flows in the cell at `(row, column)`, or `None` if unstated (the wire
    /// default is [`TextDirection::Horizontal`]). Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn cell_text_direction(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<Option<TextDirection>, PptxError> {
        self.with_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                Ok(properties.and_then(|properties| properties.text_direction(interner)))
            },
        )
    }

    /// Sets which way the text flows in the cell at `(row, column)` — how a rotated header row is
    /// made.
    ///
    /// # Errors
    /// As [`cell_text`](Self::cell_text).
    pub fn set_cell_text_direction(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        direction: TextDirection,
    ) -> Result<(), PptxError> {
        self.edit_cell_properties(
            surface.into(),
            shape_idx,
            row,
            column,
            |properties, interner| {
                properties.set_text_direction(interner, direction);
                Ok(())
            },
        )
    }

    /// Reads the `a:tcPr` of the cell at `(row, column)` — `None` when the cell declares none — and
    /// hands it, with the part's interner, to `read`. Does **not** dirty the part.
    fn with_cell_properties<R>(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        row: usize,
        column: usize,
        read: impl FnOnce(Option<&TableCellProperties>, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let table = slide::shape_table(shape, &doc.interner).ok_or(PptxError::ShapeIsNotATable)?;
        let cell = table_cell(table, &doc.interner, row, column)?;
        let properties = match nav::child(cell, &doc.interner, DML_MAIN, "tcPr") {
            Some(element) => Some(TableCellProperties::from_xml(element, &doc.interner)?),
            None => None,
        };
        read(properties.as_ref(), &doc.interner)
    }

    /// Hands the `a:tcPr` of the cell at `(row, column)` to `edit` and writes it back, **creating
    /// the element when the cell has none** — inserted after the cell's `a:txBody`, per
    /// `CT_TableCell`'s sequence.
    ///
    /// Only the `a:tcPr` is parsed and rebuilt; the table around it is untouched.
    fn edit_cell_properties(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        row: usize,
        column: usize,
        edit: impl FnOnce(&mut TableCellProperties, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;

        // Bounds first, against an immutable view, so the error can name the table's real shape.
        let (rows, columns) = {
            let table = slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
            table_dimensions_of(table, interner)
        };
        if row >= rows || column >= columns {
            return Err(PptxError::TableCellOutOfRange {
                row,
                column,
                rows,
                columns,
            });
        }

        let table = slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
        let row_element = slide::nth_row_mut(table, interner, row)
            .ok_or(PptxError::MalformedSlide("table row vanished"))?;
        let cell = slide::nth_cell_mut(row_element, interner, column)
            .ok_or(PptxError::MalformedSlide("table cell vanished"))?;

        let slot = cell_properties_slot(cell, interner)?;
        let mut properties = TableCellProperties::from_xml(slot, interner)?;
        edit(&mut properties, interner)?;
        *slot = properties.to_xml(interner);
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Formatting many cells at once
    //
    // The per-property setters above each say one thing, which is right when a caller means one
    // thing. A navy header row with a rule under it is *one* intention, and saying it nine times in
    // a loop reads like nine. These take a `Cells` selection and a spec, in the shape the crate
    // already uses everywhere else.
    // -----------------------------------------------------------------------------------------

    /// Applies `format` to every cell in `cells`. Marks only that part dirty.
    ///
    /// **Only the properties `format` names are written**, so a fill can be applied across a region
    /// whose cells carry different borders without flattening them. A format that names nothing
    /// changes nothing, and creates no `a:tcPr` for a cell that had none.
    ///
    /// The table is located once and the selection walked within it, so formatting a whole table
    /// costs one traversal rather than one per cell.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotATable`] if the shape frames no table,
    /// [`PptxError::TableCellOutOfRange`] if the selection reaches outside it, or another
    /// [`PptxError`] if an index is out of range or the part is malformed.
    pub fn format_cells(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        cells: Cells,
        format: &CellFormat,
    ) -> Result<(), PptxError> {
        if format.is_empty() {
            return Ok(());
        }
        self.edit_selected_cells(
            surface.into(),
            shape_idx,
            &cells,
            true,
            |cell, interner, _, _| {
                let slot = cell_properties_slot(cell, interner)?;
                let mut properties = TableCellProperties::from_xml(slot, interner)?;
                apply_cell_format(&mut properties, interner, format);
                *slot = properties.to_xml(interner);
                Ok(())
            },
        )
    }

    /// Applies `spec` to **every run of every paragraph** in each cell of `cells`, and to each
    /// paragraph's mark — bolding a header row in one call.
    ///
    /// This is the cell-selection form of
    /// [`set_cell_run_properties_all`](Self::set_cell_run_properties_all).
    ///
    /// # Errors
    /// As [`format_cells`](Self::format_cells), plus a malformed text body.
    pub fn format_cell_text(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        cells: Cells,
        spec: &CharacterPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_selected_cells(
            surface.into(),
            shape_idx,
            &cells,
            true,
            |cell, interner, _, _| {
                let Some(slot) = nav::child_mut(cell, interner, DML_MAIN, "txBody") else {
                    return Ok(()); // A cell with no text body has no runs to format.
                };
                let mut body = TextBody::from_xml(slot, interner)?;
                set_all_run_properties_in(&mut body, interner, spec)?;
                *slot = body.to_xml(interner);
                Ok(())
            },
        )
    }

    /// Applies `spec` to the layout properties of **every paragraph** in each cell of `cells` —
    /// right-aligning a column of numbers in one call.
    ///
    /// # Errors
    /// As [`format_cell_text`](Self::format_cell_text).
    pub fn format_cell_paragraphs(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        cells: Cells,
        spec: &ParagraphPropertiesSpec,
    ) -> Result<(), PptxError> {
        self.edit_selected_cells(
            surface.into(),
            shape_idx,
            &cells,
            true,
            |cell, interner, _, _| {
                let Some(slot) = nav::child_mut(cell, interner, DML_MAIN, "txBody") else {
                    return Ok(());
                };
                let mut body = TextBody::from_xml(slot, interner)?;
                let count = body.paragraphs().count();
                for index in 0..count {
                    set_paragraph_properties_in(&mut body, interner, index, spec)?;
                }
                *slot = body.to_xml(interner);
                Ok(())
            },
        )
    }

    /// Locates the table once, resolves `cells` against its real dimensions, and hands each selected
    /// `a:tc` to `edit` in row-major order.
    ///
    /// When `visible_only`, a cell covered by a merge (which renders nothing) is skipped — so
    /// formatting a selection touches only the anchors that actually show, and unmerging restores a
    /// covered cell's own formatting. Merging and unmerging pass `false`: they must reach covered
    /// cells to set and clear the merge flags.
    fn edit_selected_cells(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        cells: &Cells,
        visible_only: bool,
        edit: impl Fn(&mut RawElement, &mut Interner, usize, usize) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;

        let (rows, columns) = {
            let table = slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
            table_dimensions_of(table, interner)
        };
        let positions = cells.resolve(rows, columns).map_err(|(row, column)| {
            PptxError::TableCellOutOfRange {
                row,
                column,
                rows,
                columns,
            }
        })?;

        let table = slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
        for (row, column) in positions {
            let row_element = slide::nth_row_mut(table, interner, row)
                .ok_or(PptxError::MalformedSlide("table row vanished"))?;
            let cell = slide::nth_cell_mut(row_element, interner, column)
                .ok_or(PptxError::MalformedSlide("table cell vanished"))?;
            if visible_only && raw_cell_is_covered(cell, interner) {
                continue;
            }
            edit(cell, interner, row, column)?;
        }
        Ok(())
    }

    // -----------------------------------------------------------------------------------------
    // Merging
    //
    // A merged region is anchored at its top-left cell, which states how far it reaches; the cells
    // it covers stay in the table, each stating that something to its left or above owns it. So the
    // grid never loses a cell, `(row, column)` addressing keeps working, and unmerging is simply
    // taking four attributes back off.
    // -----------------------------------------------------------------------------------------

    /// Merges `cells` into one region. Marks only that part dirty.
    ///
    /// The top-left cell becomes the anchor and is what renders; every other cell in the region is
    /// marked as covered. **No cell is removed and no text is touched** — a covered cell keeps its
    /// own text body, invisible until the region is unmerged again, so merging loses nothing.
    ///
    /// A merged region already **inside** the selection is absorbed into the new one. A selection of
    /// a single cell, or an empty one, changes nothing.
    ///
    /// # Errors
    /// Returns [`PptxError::TableMergeCrossesSelection`] if a cell in the selection belongs to a
    /// merged region reaching outside it — unmerge that region first — plus the errors of
    /// [`format_cells`](Self::format_cells).
    pub fn merge_cells(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        cells: Cells,
    ) -> Result<(), PptxError> {
        let surface = surface.into();

        // Read first: the region to merge, and whether any existing merge would be cut in half.
        let region = self.with_table(surface, shape_idx, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let (row_range, column_range) =
                cells.bounds(rows, columns).map_err(|(row, column)| {
                    PptxError::TableCellOutOfRange {
                        row,
                        column,
                        rows,
                        columns,
                    }
                })?;
            if row_range.is_empty() || column_range.is_empty() {
                return Ok(None);
            }
            check_merges_fit(table, interner, &row_range, &column_range)?;
            Ok(Some((row_range, column_range)))
        })?;

        let Some((row_range, column_range)) = region else {
            return Ok(());
        };
        let (first_row, first_column) = (row_range.start, column_range.start);
        let (height, width) = (row_range.len(), column_range.len());
        let selection = Cells::rectangle(row_range, column_range);

        self.edit_selected_cells(
            surface,
            shape_idx,
            &selection,
            false, // merging must reach the cells it covers, to mark them merged
            |cell, interner, row, column| {
                let mut typed = TableCell::from_xml(cell, interner)?;
                if row == first_row && column == first_column {
                    typed.set_spans(interner, width, height);
                    typed.set_merged(interner, false, false);
                } else {
                    // Covered: it says what owns it, not which cell that is — left, above, or both.
                    typed.set_spans(interner, 1, 1);
                    typed.set_merged(interner, column > first_column, row > first_row);
                }
                *cell = typed.to_xml(interner);
                Ok(())
            },
        )
    }

    /// Undoes the merge covering the cell at `(row, column)`, whichever cell of the region is named.
    /// Marks only that part dirty.
    ///
    /// Every cell in the region becomes an ordinary cell again, and each gets back the text it was
    /// holding all along. A cell that is not merged is left alone.
    ///
    /// # Errors
    /// As [`format_cells`](Self::format_cells).
    pub fn unmerge_cells(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<(), PptxError> {
        let surface = surface.into();

        // The region is defined by its anchor, which the addressed cell may only point towards.
        let region = self.with_table(surface, shape_idx, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let out_of_range = || PptxError::TableCellOutOfRange {
                row,
                column,
                rows,
                columns,
            };
            let (anchor_row, anchor_column) = table
                .merge_anchor(interner, row, column)
                .ok_or_else(out_of_range)?;
            let anchor = table
                .cell(anchor_row, anchor_column)
                .ok_or_else(out_of_range)?;
            Ok(Cells::rectangle(
                anchor_row..anchor_row + anchor.row_span(interner),
                anchor_column..anchor_column + anchor.column_span(interner),
            ))
        })?;

        self.edit_selected_cells(
            surface,
            shape_idx,
            &region,
            false,
            |cell, interner, _, _| {
                let mut typed = TableCell::from_xml(cell, interner)?;
                typed.clear_merge(interner);
                *cell = typed.to_xml(interner);
                Ok(())
            },
        )
    }

    /// The **explicit** position and size of shape `shape_idx` on `surface` — the `a:off` and
    /// `a:ext` of its transform — or `None` when the shape does not place itself.
    ///
    /// A `None` here is not "at the origin": it means the shape declares no bounds of its own, so a
    /// placeholder takes them from its layout and then its master — resolving *that* is a separate,
    /// future `effective_shape_bounds`. It is also `None` for a transform that names only one of the
    /// two, since bounds are all four numbers.
    ///
    /// Bounds are absolute within [`slide_size`](Self::slide_size), except for a shape inside a
    /// `p:grpSp` — group members are not addressable, so this never returns one. Reading does not
    /// dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn shape_bounds(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<ShapeBounds>, PptxError> {
        Ok(self
            .shape_transform(surface, shape_idx)?
            .as_ref()
            .and_then(ShapeBounds::from_transform))
    }

    /// Moves and resizes shape `shape_idx` on `surface` to `bounds`, creating its transform element
    /// if it had none. Marks only that part dirty; everything else re-emits verbatim.
    ///
    /// Only the position and size are written — a rotation, a flip, or the child coordinate space of
    /// a group are left exactly as they were. Note that resizing a **group** rescales its members,
    /// because a group maps its child space (`a:chOff` / `a:chExt`) onto its own extent; that is what
    /// PowerPoint does when you drag a group's handle.
    ///
    /// # Errors
    /// As [`set_shape_transform`](Self::set_shape_transform).
    pub fn set_shape_bounds(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        bounds: ShapeBounds,
    ) -> Result<(), PptxError> {
        self.set_shape_transform(surface, shape_idx, &bounds.to_transform())
    }

    /// The **explicit** transform of shape `shape_idx` on `surface` — its position, size, rotation
    /// and mirror flags, plus the child coordinate space if it is a group — or `None` when the shape
    /// declares no transform at all.
    ///
    /// Where that transform lives depends on the shape's [`ShapeKind`]: `p:spPr > a:xfrm` for a
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
        shape_idx: usize,
    ) -> Result<Option<Transform2D>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
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
        shape_idx: usize,
        transform: &Transform2D,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` names the element, `root` holds the tree it lands in.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let slot = slide::shape_transform_slot_mut(shape, interner)?;
        transform.apply(slot, interner);
        Ok(())
    }

    /// The preset geometry of shape `shape_idx` on `surface`, as a typed [`ShapeGeometry`]
    /// (named adjustments in friendly units). Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, the shape has no
    /// `a:prstGeom` ([`ShapeHasNoGeometry`](PptxError::ShapeHasNoGeometry)), or its `prst` names a
    /// shape type this build does not recognize ([`UnknownShapeType`](PptxError::UnknownShapeType)).
    pub fn shape_geometry(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<ShapeGeometry, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let prst_geom =
            slide::shape_prstgeom(shape, &doc.interner).ok_or(PptxError::ShapeHasNoGeometry)?;
        let geometry = PresetGeometry::from_xml(prst_geom, &doc.interner)?;
        geometry
            .shape(&doc.interner)
            .ok_or(PptxError::UnknownShapeType)
    }

    /// Sets the preset geometry of shape `shape_idx` on `surface` from a typed
    /// [`ShapeGeometry`] — rewriting the shape's `a:prstGeom@prst` and its adjustment `a:gd`s. Marks
    /// only that slide part dirty; everything else re-emits verbatim.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `a:prstGeom` to edit.
    pub fn set_shape_geometry(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        geometry: ShapeGeometry,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` for name resolution / rebuild, `root` for locate + replace.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoGeometry)?;
        let slot = nav::child_mut(sp_pr, interner, DML_MAIN, "prstGeom")
            .ok_or(PptxError::ShapeHasNoGeometry)?;

        let mut geom = PresetGeometry::from_xml(slot, interner)?;
        geom.set_shape(interner, geometry);
        // The edit lands here: rebuild the prstGeom in place, reusing the part's own interner.
        *slot = geom.to_xml(interner);
        Ok(())
    }

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
        shape_idx: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
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
    /// A [`FillSpec::Blip`] writes only the `a:blip@r:embed` reference; the image part and its
    /// relationship must already exist in the package — create both with
    /// [`add_image`](Self::add_image), which returns the id to use.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, or the shape has no
    /// `p:spPr` ([`ShapeHasNoProperties`](PptxError::ShapeHasNoProperties)).
    pub fn set_shape_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        fill: &FillSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the fill element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        // A picture fill carries an `r:embed`, so the built element must be able to resolve the `r`
        // prefix — computed from the part root before the borrow descends into the shape tree.
        let rel_declaration = match fill {
            FillSpec::Blip { .. } => build::relationship_prefix_declaration(root, interner),
            _ => None,
        };
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;

        let mut element = fill.to_fill(interner).to_xml(interner);
        if let Some(declaration) = rel_declaration {
            element.attributes.push(declaration);
        }
        let node = RawNode::Element(element);
        match slide::fill_child_index(sp_pr, interner) {
            Some(index) => sp_pr.children[index] = node,
            None => {
                let at = slide::fill_insert_index(sp_pr, interner);
                sp_pr.children.insert(at, node);
                sp_pr.empty = false;
            }
        }
        Ok(())
    }

    /// Sets shape `shape_idx` on `surface` to an explicit "no fill" (`a:noFill`). A shorthand
    /// for [`set_shape_fill`](Self::set_shape_fill) with [`FillSpec::None`].
    ///
    /// # Errors
    /// As [`set_shape_fill`](Self::set_shape_fill).
    pub fn set_shape_no_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
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
        shape_idx: usize,
    ) -> Result<Option<LineSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
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
        shape_idx: usize,
        line: &LineSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the outline element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;

        let element = line.to_line(interner).to_xml(interner);
        let node = RawNode::Element(element);
        match slide::line_child_index(sp_pr, interner) {
            Some(index) => sp_pr.children[index] = node,
            None => {
                let at = slide::line_insert_index(sp_pr, interner);
                sp_pr.children.insert(at, node);
                sp_pr.empty = false;
            }
        }
        Ok(())
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
        shape_idx: usize,
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
        shape_idx: usize,
    ) -> Result<Option<EffectListSpec>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
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
        shape_idx: usize,
        effects: &EffectListSpec,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        // Split the borrow: `interner` builds the effect element, `root` receives it.
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let sp_pr =
            nav::child_mut(shape, interner, PML, "spPr").ok_or(PptxError::ShapeHasNoProperties)?;

        let element = effects.to_effect_list(interner).to_xml(interner);
        let node = RawNode::Element(element);
        match slide::effect_child_index(sp_pr, interner) {
            Some(index) => sp_pr.children[index] = node,
            None => {
                let at = slide::effect_insert_index(sp_pr, interner);
                sp_pr.children.insert(at, node);
                sp_pr.empty = false;
            }
        }
        Ok(())
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
        shape_idx: usize,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        self.set_shape_effects(surface, shape_idx, &EffectListSpec::new())
    }

    /// The theme that governs `surface`, as an interner-free [`ThemeInfo`] (its color scheme +
    /// fill-style matrix) — the theme related to the last part of the surface's inheritance chain
    /// (slide → slideLayout → slideMaster → theme, and the shorter walks from a layout or master).
    /// Returns `Ok(None)` if any hop is absent (a deck without a theme). Reading does not dirty any
    /// part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range, a relationship points outside the
    /// package ([`ExternalTarget`](PptxError::ExternalTarget)), or the theme part is not well-formed.
    pub fn theme(&mut self, surface: impl Into<Surface>) -> Result<Option<ThemeInfo>, PptxError> {
        let surface = surface.into();
        let Some(theme_part) = self.theme_part(surface)? else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&theme_part)?;
        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
        Ok(Some(theme.to_info(&doc.interner)))
    }

    /// The theme [`PartName`] governing `surface`: the theme related to the last part of its
    /// inheritance chain (the master, where there is one); `None` if that part relates to no theme.
    fn theme_part(&self, surface: Surface) -> Result<Option<PartName>, PptxError> {
        let chain = self.inheritance_chain(surface)?;
        let last = chain
            .last()
            .expect("a chain always holds the surface's own part");
        self.follow_rel(last, constants::REL_THEME)
    }

    /// The **effective** fill of shape `shape_idx` on `surface`, as an interner-free
    /// [`FillSpec`] whose colors are resolved to concrete `RRGGBB` values — the fill the shape actually
    /// renders. Three sources are tried, in order: an explicit `p:spPr` fill; a `p:style > a:fillRef`
    /// (the theme fill-style at that index, `phClr` substituted by the reference's color); and, for a
    /// placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on the layout
    /// then the master. Scheme colors and color transforms are baked against the surface's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields a fill. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        let surface = surface.into();
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        let candidates = self.placeholder_candidates(surface, shape_idx)?;

        for (part, candidate) in candidates {
            // Extract the candidate's own fill while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                match candidate_shape(doc, candidate)? {
                    Some(shape) => shape_own_fill(shape, &doc.interner, &scheme, &map)?,
                    None => OwnFill::Absent,
                }
            };

            match own {
                OwnFill::Resolved(spec) => return Ok(Some(spec)),
                OwnFill::StyleRef(idx, color) => {
                    // Resolve the referenced theme fill-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.fill_style(idx) {
                            return Ok(Some(resolve_fill(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnFill::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** outline of shape `shape_idx` on `surface`, as an interner-free
    /// [`LineSpec`] whose stroke color is resolved to a concrete `RRGGBB` value — the outline the shape
    /// actually renders. Three sources are tried, in order: an explicit `p:spPr > a:ln`; a
    /// `p:style > a:lnRef` (the theme line-style at that index, `phClr` substituted by the reference's
    /// color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot placeholder on
    /// the slide layout then the master. Scheme colors and color transforms are baked against the
    /// slide's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields an outline. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_outline(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<LineSpec>, PptxError> {
        let surface = surface.into();
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        let candidates = self.placeholder_candidates(surface, shape_idx)?;

        for (part, candidate) in candidates {
            // Extract the candidate's own outline while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                match candidate_shape(doc, candidate)? {
                    Some(shape) => shape_own_line(shape, &doc.interner, &scheme, &map)?,
                    None => OwnLine::Absent,
                }
            };

            match own {
                OwnLine::Resolved(spec) => return Ok(Some(spec)),
                OwnLine::StyleRef(idx, color) => {
                    // Resolve the referenced theme line-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.line_style(idx) {
                            return Ok(Some(resolve_line(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnLine::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** effects of shape `shape_idx` on `surface`, as an interner-free
    /// [`EffectListSpec`] whose colors are resolved to concrete `RRGGBB` values — the effects the shape
    /// actually renders. Three sources are tried, in order: an explicit `p:spPr > a:effectLst`; a
    /// `p:style > a:effectRef` (the theme effect-style at that index, `phClr` substituted by the
    /// reference's color); and, for a placeholder shape (`p:ph`), **inheritance** from the same-slot
    /// placeholder on the slide layout then the master. Scheme colors and color transforms are baked
    /// against the slide's theme + map.
    ///
    /// Returns `Ok(None)` when no source yields effects. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the slide is malformed, a relationship points
    /// outside the package, or a part is not well-formed.
    pub fn effective_shape_effects(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<EffectListSpec>, PptxError> {
        let surface = surface.into();
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        // The resolved color scheme (interner-free) — bridges the theme-part vs shape-part interners.
        let scheme = match &theme_part {
            Some(part) => {
                let doc = self.package.part_tree(part)?;
                let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                theme
                    .color_scheme()
                    .map(|cs| SchemeColors::from_scheme(cs, &doc.interner))
                    .unwrap_or_default()
            }
            None => SchemeColors::default(),
        };

        let candidates = self.placeholder_candidates(surface, shape_idx)?;

        for (part, candidate) in candidates {
            // Extract the candidate's own effects while holding its part's borrow (fully owned).
            let own = {
                let doc = self.package.part_tree(&part)?;
                match candidate_shape(doc, candidate)? {
                    Some(shape) => shape_own_effects(shape, &doc.interner, &scheme, &map)?,
                    None => OwnEffects::Absent,
                }
            };

            match own {
                OwnEffects::Resolved(spec) => return Ok(Some(*spec)),
                OwnEffects::StyleRef(idx, color) => {
                    // Resolve the referenced theme effect-style (theme-part interner), substituting phClr.
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.effect_style(idx) {
                            return Ok(Some(resolve_effects(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnEffects::Absent => {}
            }
        }

        Ok(None)
    }

    /// The **effective** transform of shape `shape_idx` on `surface` — where the shape actually
    /// renders, not what it declares. For a placeholder that places itself nowhere, this is the
    /// same-slot placeholder's transform on the slide layout, and failing that on the master.
    ///
    /// Returns `Ok(None)` when no tier places the shape. Reading does not dirty any part.
    ///
    /// # Inheritance is all-or-nothing
    ///
    /// Unlike text formatting, whose tiers each contribute what the ones above left unset, a
    /// transform is inherited **whole**: the first tier that states anything wins entirely. A shape
    /// cannot take its position from the layout and its size from the master, because PowerPoint
    /// offers no such thing — a shape that places itself places itself completely.
    ///
    /// A **present but empty** `<a:xfrm/>` states nothing, so the walk steps past it exactly as it
    /// steps past a tier with no transform element at all.
    ///
    /// A shape that is **not a placeholder** has no tiers to inherit from, so its effective transform
    /// is its explicit one.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, a part is malformed, or a relationship in
    /// the inheritance chain points outside the package.
    pub fn effective_shape_transform(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<Transform2D>, PptxError> {
        let surface = surface.into();
        let candidates = self.placeholder_candidates(surface, shape_idx)?;

        for (part, candidate) in candidates {
            let doc = self.package.part_tree(&part)?;
            let Some(shape) = candidate_shape(doc, candidate)? else {
                continue; // This tier does not define the slot at all.
            };
            let Some(element) = slide::shape_transform(shape, &doc.interner) else {
                continue; // …or defines it without placing it.
            };
            let transform = Transform2D::read(element, &doc.interner);
            if !transform.is_empty() {
                return Ok(Some(transform));
            }
        }

        Ok(None)
    }

    /// The **effective** position and size of shape `shape_idx` on `surface` — where the shape
    /// actually renders, with the layout and master consulted for a placeholder that declares no
    /// bounds of its own.
    ///
    /// This is the question [`shape_bounds`](Self::shape_bounds) cannot answer: a title that
    /// declares no `a:xfrm` still renders somewhere, and where is on its layout. Returns `Ok(None)`
    /// when no tier places the shape, or when the tier that does names a rotation or a flip without
    /// naming both an `a:off` and an `a:ext`.
    ///
    /// Bounds are absolute within [`slide_size`](Self::slide_size). Reading does not dirty any part.
    ///
    /// # Errors
    /// As [`effective_shape_transform`](Self::effective_shape_transform).
    pub fn effective_shape_bounds(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<ShapeBounds>, PptxError> {
        Ok(self
            .effective_shape_transform(surface, shape_idx)?
            .as_ref()
            .and_then(ShapeBounds::from_transform))
    }

    // -----------------------------------------------------------------------------------------
    // Effective text formatting — what the text actually renders as
    //
    // Every reader above answers what a paragraph or run *declares*. These two answer what it
    // *renders as*, which is a different question: a placeholder that declares nothing still has a
    // size, and that size lives in the master's `p:txStyles`. Seven tiers, each contributing only
    // what the tiers above left unset — see `text_style_tiers` for the walk they share.
    // -----------------------------------------------------------------------------------------

    /// The **effective** character properties of run `run_idx` — what the run actually renders as,
    /// with every tier of inheritance resolved and its colors baked to concrete `RRGGBB`.
    ///
    /// Seven tiers contribute, highest priority first, each supplying only what the tiers above left
    /// unset:
    ///
    /// 1. the run's own `a:rPr`;
    /// 2. the paragraph's `a:pPr > a:defRPr`;
    /// 3. the shape's `a:lstStyle`, at the paragraph's level;
    /// 4. the same-slot placeholder's `a:lstStyle` on the layout, then the master;
    /// 5. the master's `p:txStyles` — `p:titleStyle` for a title placeholder, `p:otherStyle` for the
    ///    date/footer/slide-number slots, `p:bodyStyle` for the rest. A shape that is not a
    ///    placeholder takes none of these;
    /// 6. `p:defaultTextStyle` in `presentation.xml`;
    /// 7. the theme's font scheme, for a typeface still naming `+mj-lt` / `+mn-lt`.
    ///
    /// The paragraph's level (`a:pPr@lvl`, [`IndentLevel::TOP`] when unstated) is read once and
    /// selects which `a:lvlNpPr` every tier from 3 down contributes — which is why demoting a line
    /// changes its size and bullet without anything being written to the run.
    ///
    /// Returns an empty spec when no tier contributes anything. Reading does not dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range, the shape has no text body, a relationship
    /// points outside the package, or a part is not well-formed.
    pub fn effective_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);

        // Tiers 1 and 2, and the level the rest are read at — all from the shape's own body.
        let (level, own, paragraph_default) =
            self.with_text_body(surface, shape_idx, |body, interner| {
                let paragraph = nth_paragraph(body, para_idx)?;
                let count = paragraph.runs().count();
                let run = paragraph
                    .runs()
                    .nth(run_idx)
                    .ok_or(PptxError::RunIndexOutOfRange {
                        index: run_idx,
                        count,
                    })?;
                let properties = paragraph.properties();
                Ok((
                    paragraph_level(body, para_idx, interner),
                    run.properties()
                        .map(|rpr| resolve_character_properties(rpr, &scheme, &map, None, interner))
                        .unwrap_or_default(),
                    properties
                        .and_then(|ppr| ppr.default_run_properties(interner))
                        .map(|def| {
                            resolve_character_properties(&def, &scheme, &map, None, interner)
                        })
                        .unwrap_or_default(),
                ))
            })?;

        // Tiers 3–6 contribute their level's `a:defRPr`.
        let effective = self
            .text_style_tiers(surface, shape_idx, level, &scheme, &map)?
            .iter()
            .filter_map(ParagraphPropertiesSpec::default_run_properties)
            .fold(own.merge_under(&paragraph_default), |resolved, tier| {
                resolved.merge_under(tier)
            });

        // Tier 7: a typeface that still names a theme font.
        self.resolve_theme_fonts(surface, effective)
    }

    /// The **effective** paragraph properties of paragraph `para_idx` — the layout it actually
    /// renders with, every tier of inheritance resolved.
    ///
    /// The same ladder as [`effective_run_properties`](Self::effective_run_properties), minus the
    /// run-level tiers: the paragraph's own `a:pPr`, then the shape's `a:lstStyle`, the same-slot
    /// placeholder's on the layout and master, the master's `p:txStyles`, and `p:defaultTextStyle`.
    /// Its [`default_run_properties`](ParagraphPropertiesSpec::default_run_properties) carry the
    /// merged `a:defRPr` of every tier, with colors baked.
    ///
    /// This is where a bullet comes from: a level-2 paragraph that declares nothing still answers with
    /// the master `bodyStyle`'s `a:lvl3pPr` bullet, size and indent.
    ///
    /// Returns an empty spec when no tier contributes anything. Reading does not dirty any part.
    ///
    /// # Errors
    /// As [`effective_run_properties`](Self::effective_run_properties).
    pub fn effective_paragraph_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        para_idx: usize,
    ) -> Result<ParagraphPropertiesSpec, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);

        let (level, own) = self.with_text_body(surface, shape_idx, |body, interner| {
            let level = paragraph_level(body, para_idx, interner);
            let own = nth_paragraph(body, para_idx)?
                .properties()
                .map(|ppr| resolved_paragraph_spec(ppr, &scheme, &map, interner))
                .unwrap_or_default();
            Ok((level, own))
        })?;

        Ok(self
            .text_style_tiers(surface, shape_idx, level, &scheme, &map)?
            .iter()
            .fold(own, |resolved, tier| resolved.merge_under(tier)))
    }

    // ---------------------------------------------------------------------------------------------
    // Effective cell formatting — what a table cell actually renders as.
    //
    // Resolution order: the cell's own `a:tcPr` wins; then the table style's parts, selected by the
    // cell's position and the `a:tblPr` flags (`applicable_parts`), most specific first; then the
    // theme, for an `lnRef` / `fillRef`. Colours bake to concrete `RRGGBB`, exactly as the shape
    // resolvers do. Every read walks three parts (slide, `tableStyles.xml`, theme), extracting owned
    // values while each is borrowed. Reading dirties nothing.
    // ---------------------------------------------------------------------------------------------

    /// The **effective** fill of the cell at `(row, column)` of the table shape `shape_idx` frames — an
    /// interner-free [`FillSpec`] with its colour baked to concrete `RRGGBB`, or `None` if nothing
    /// fills the cell. The cell's own `a:tcPr` fill wins; else the first applicable style part with a
    /// fill (explicit or a theme `fillRef`).
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus [`PptxError::TableCellOutOfRange`].
    pub fn effective_cell_fill(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<Option<FillSpec>, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        let (own, dims, flags) = self.with_table(surface, shape_idx, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let cell = cell_at(table, row, column)?;
            let own = cell
                .properties()
                .and_then(|tcpr| tcpr.fill(interner))
                .map(|fill| resolve_fill(&fill, &scheme, &map, None, interner));
            Ok((own, (rows, columns), table_flags(table, interner)))
        })?;

        if let Some(spec) = own {
            return Ok(Some(spec));
        }

        let parts = applicable_parts(row, column, dims.0, dims.1, flags);
        let Some(part_fills) =
            self.cell_style_candidates(surface, shape_idx, &parts, |cell_style, interner| {
                part_own_fill(cell_style, interner, &scheme, &map)
            })?
        else {
            return Ok(None);
        };

        for own in part_fills {
            match own {
                OwnFill::Resolved(spec) => return Ok(Some(spec)),
                OwnFill::StyleRef(idx, color) => {
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.fill_style(idx) {
                            return Ok(Some(resolve_fill(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnFill::Absent => {}
            }
        }
        Ok(None)
    }

    /// The **effective** border on one `edge` of the cell at `(row, column)` — an interner-free
    /// [`LineSpec`] with its stroke colour baked, or `None`. The cell's own `a:tcPr` edge wins; else
    /// the applicable style parts' `a:tcBdr`, taking the outer edge (`top`/`left`/…) for a cell on the
    /// table's rim and the interior edge (`insideH`/`insideV`) for one within it.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus [`PptxError::TableCellOutOfRange`].
    pub fn effective_cell_border(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        edge: CellBorder,
    ) -> Result<Option<LineSpec>, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);
        let theme_part = self.theme_part(surface)?;

        let (own, dims, flags) = self.with_table(surface, shape_idx, |table, interner| {
            let (rows, columns) = (table.row_count(), table.column_count());
            let cell = cell_at(table, row, column)?;
            let own = cell
                .properties()
                .and_then(|tcpr| tcpr.border(interner, edge))
                .map(|line| resolve_line(&line, &scheme, &map, None, interner));
            Ok((own, (rows, columns), table_flags(table, interner)))
        })?;

        if let Some(spec) = own {
            return Ok(Some(spec));
        }

        let (rows, columns) = dims;
        let style_edge = style_border_key(edge, row, column, rows, columns);
        let parts = applicable_parts(row, column, rows, columns, flags);
        let Some(part_lines) =
            self.cell_style_candidates(surface, shape_idx, &parts, |cell_style, interner| {
                cell_style
                    .borders(interner)
                    .and_then(|borders| borders.border(interner, style_edge))
                    .map_or(OwnLine::Absent, |themeable| {
                        part_own_line(themeable, interner, &scheme, &map)
                    })
            })?
        else {
            return Ok(None);
        };

        for own in part_lines {
            match own {
                OwnLine::Resolved(spec) => return Ok(Some(spec)),
                OwnLine::StyleRef(idx, color) => {
                    if let Some(theme_part) = &theme_part {
                        let doc = self.package.part_tree(theme_part)?;
                        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
                        if let Some(style) = theme.line_style(idx) {
                            return Ok(Some(resolve_line(
                                style,
                                &scheme,
                                &map,
                                color,
                                &doc.interner,
                            )));
                        }
                    }
                }
                OwnLine::Absent => {}
            }
        }
        Ok(None)
    }

    /// The **effective** run properties of a cell's text run — the [`CharacterPropertiesSpec`] it
    /// actually renders with, colours baked. A shorter ladder than a shape's (a cell inherits from its
    /// table style, not a placeholder chain), highest first: the run's own `a:rPr`, the paragraph's
    /// `a:defRPr`, the table style's `a:tcTxStyle` for each applicable part (bold / italic / colour),
    /// then the presentation's `p:defaultTextStyle`.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus [`PptxError::TableCellOutOfRange`] and the
    /// paragraph/run index errors.
    pub fn effective_cell_run_properties(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
        para_idx: usize,
        run_idx: usize,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        let surface = surface.into();
        let scheme = self.resolved_scheme_colors(surface)?;
        let map = self.color_map(surface)?.unwrap_or_else(ColorMap::identity);

        let (level, own, para_default, dims, flags) =
            self.with_table(surface, shape_idx, |table, interner| {
                let (rows, columns) = (table.row_count(), table.column_count());
                let cell = cell_at(table, row, column)?;
                let body = cell
                    .text_body()
                    .ok_or(PptxError::MalformedSlide("table cell has no text body"))?;
                let level = paragraph_level(body, para_idx, interner);
                let paragraph = nth_paragraph(body, para_idx)?;
                let run = nth_run(paragraph, run_idx)?;
                let own = run
                    .properties()
                    .map(|rpr| resolve_character_properties(rpr, &scheme, &map, None, interner))
                    .unwrap_or_default();
                let para_default = paragraph
                    .properties()
                    .and_then(|ppr| ppr.default_run_properties(interner))
                    .map(|def| resolve_character_properties(&def, &scheme, &map, None, interner))
                    .unwrap_or_default();
                Ok((
                    level,
                    own,
                    para_default,
                    (rows, columns),
                    table_flags(table, interner),
                ))
            })?;

        let parts = applicable_parts(row, column, dims.0, dims.1, flags);
        let style_text = self.cell_style_text(surface, shape_idx, &parts, &scheme, &map)?;
        let default_text = self.default_text_run_properties(level, &scheme, &map)?;

        let effective = own
            .merge_under(&para_default)
            .merge_under(&style_text)
            .merge_under(&default_text);
        self.resolve_theme_fonts(surface, effective)
    }

    /// Runs `extract` over each applicable style part's `a:tcStyle`, most specific first, returning the
    /// results in that order — or `None` when the table resolves to no style (so the caller stops at
    /// the cell's own properties). Resolves an inline `a:tableStyle` or a shared one alike.
    fn cell_style_candidates<T>(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        parts: &[TableStylePart],
        extract: impl Fn(&TableStyleCellStyle, &Interner) -> T,
    ) -> Result<Option<Vec<T>>, PptxError> {
        self.with_resolved_style(surface, shape_idx, |style, interner| {
            Ok(parts
                .iter()
                .filter_map(|&part| {
                    let cell_style = style.part(interner, part)?.cell_style(interner)?;
                    Some(extract(&cell_style, interner))
                })
                .collect())
        })
    }

    /// The table style's text contribution for a cell — the `a:tcTxStyle` of each applicable part,
    /// merged most-specific-first. Empty when the table resolves to no style.
    fn cell_style_text(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        parts: &[TableStylePart],
        scheme: &SchemeColors,
        map: &ColorMap,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        Ok(self
            .with_resolved_style(surface, shape_idx, |style, interner| {
                let mut spec = CharacterPropertiesSpec::new();
                for &part in parts {
                    if let Some(text_style) = style
                        .part(interner, part)
                        .and_then(|part| part.text_style(interner))
                    {
                        spec =
                            spec.merge_under(&style_text_spec(&text_style, scheme, map, interner));
                    }
                }
                Ok(spec)
            })?
            .unwrap_or_default())
    }

    /// The presentation's `p:defaultTextStyle` run properties at `level`, colours baked — the bottom
    /// tier of a cell's text ladder. Empty when the presentation declares none.
    fn default_text_run_properties(
        &mut self,
        level: IndentLevel,
        scheme: &SchemeColors,
        map: &ColorMap,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        let presentation_part = self.presentation_part.clone();
        let doc = self.package.part_tree(&presentation_part)?;
        let Some(default) = nav::child(&doc.root, &doc.interner, PML, "defaultTextStyle") else {
            return Ok(CharacterPropertiesSpec::new());
        };
        let list_style = TextListStyle::from_xml(default, &doc.interner)?;
        let spec = list_style_tier(Some(&list_style), level, scheme, map, &doc.interner)
            .iter()
            .filter_map(ParagraphPropertiesSpec::default_run_properties)
            .fold(CharacterPropertiesSpec::new(), |resolved, tier| {
                resolved.merge_under(tier)
            });
        Ok(spec)
    }

    /// Tiers 3–6 of the ladder, in order and already interner-free: the shape's own `a:lstStyle`, the
    /// same-slot placeholder's on each ancestor part, the master's `p:txStyles`, and the
    /// presentation's `p:defaultTextStyle` — each taken at `level`.
    ///
    /// One walk serves both public answers: a tier's `a:lvlNpPr` *is* the paragraph contribution, and
    /// its `a:defRPr` is the character one.
    fn text_style_tiers(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        level: IndentLevel,
        scheme: &SchemeColors,
        map: &ColorMap,
    ) -> Result<Vec<ParagraphPropertiesSpec>, PptxError> {
        let mut tiers = Vec::new();

        // Tier 3 — the shape's own list style, and the placeholder slot the rest are matched on.
        let placeholder = {
            let part = self.surface_part(surface)?;
            let doc = self.package.part_tree(&part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            let count = slide::shapes(sp_tree, &doc.interner).count();
            let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
                PptxError::ShapeIndexOutOfRange {
                    surface,
                    index: shape_idx,
                    count,
                },
            )?;
            if let Some(txbody) = slide::shape_txbody(shape, &doc.interner) {
                let body = TextBody::from_xml(txbody, &doc.interner)?;
                tiers.extend(list_style_tier(
                    body.list_style(),
                    level,
                    scheme,
                    map,
                    &doc.interner,
                ));
            }
            slide::shape_placeholder(shape, &doc.interner)
        };

        // A shape that is not a placeholder inherits from no ancestor shape and takes no master text
        // style: its text falls straight through to the presentation default.
        if let Some(slot) = placeholder {
            // Tier 4 — the same-slot placeholder's list style, on the layout then the master.
            for ancestor in self.inheritance_chain(surface)?.into_iter().skip(1) {
                let doc = self.package.part_tree(&ancestor)?;
                let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
                let Some(shape) = slide::find_placeholder(sp_tree, slot, &doc.interner) else {
                    continue;
                };
                let Some(txbody) = slide::shape_txbody(shape, &doc.interner) else {
                    continue;
                };
                let body = TextBody::from_xml(txbody, &doc.interner)?;
                tiers.extend(list_style_tier(
                    body.list_style(),
                    level,
                    scheme,
                    map,
                    &doc.interner,
                ));
            }

            // Tier 5 — the master's text styles. A slide master names them by slot in `p:txStyles`
            // (`p:titleStyle` / `p:otherStyle` / `p:bodyStyle`); a notes master instead carries a
            // single `p:notesStyle` that styles its body text. An absent element simply means the
            // chain never reached a master (or it declares no text styles).
            let chain = self.inheritance_chain(surface)?;
            let master = chain
                .last()
                .expect("a chain always holds the surface's own part");
            let doc = self.package.part_tree(master)?;
            let master_style = if matches!(surface, Surface::Notes(_) | Surface::NotesMaster) {
                nav::child(&doc.root, &doc.interner, PML, "notesStyle")
            } else {
                nav::child(&doc.root, &doc.interner, PML, "txStyles")
                    .and_then(|styles| nav::child(styles, &doc.interner, PML, master_style_local(slot)))
            };
            if let Some(named) = master_style {
                let list_style = TextListStyle::from_xml(named, &doc.interner)?;
                tiers.extend(list_style_tier(
                    Some(&list_style),
                    level,
                    scheme,
                    map,
                    &doc.interner,
                ));
            }
        }

        // Tier 6 — `p:defaultTextStyle`, which applies to every shape, placeholder or not.
        let presentation_part = self.presentation_part.clone();
        let doc = self.package.part_tree(&presentation_part)?;
        if let Some(default) = nav::child(&doc.root, &doc.interner, PML, "defaultTextStyle") {
            let list_style = TextListStyle::from_xml(default, &doc.interner)?;
            tiers.extend(list_style_tier(
                Some(&list_style),
                level,
                scheme,
                map,
                &doc.interner,
            ));
        }

        Ok(tiers)
    }

    /// Tier 7 — replaces any typeface still naming a theme font (`+mj-lt`, `+mn-ea`, …) with the one
    /// the surface's theme actually names. A slot the scheme leaves undefined keeps its reference,
    /// which is the honest answer: the file points somewhere the theme does not go.
    fn resolve_theme_fonts(
        &mut self,
        surface: Surface,
        spec: CharacterPropertiesSpec,
    ) -> Result<CharacterPropertiesSpec, PptxError> {
        if !FontSlot::all_slots()
            .into_iter()
            .any(|slot| spec.font(slot).is_some_and(TextFont::is_theme_reference))
        {
            return Ok(spec);
        }
        let Some(theme_part) = self.theme_part(surface)? else {
            return Ok(spec);
        };
        let doc = self.package.part_tree(&theme_part)?;
        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
        let Some(font_scheme) = theme.font_scheme() else {
            return Ok(spec);
        };

        let mut resolved = spec.clone();
        for slot in FontSlot::all_slots() {
            let Some(font) = spec.font(slot) else {
                continue;
            };
            if let Some(named) = font_scheme.resolve(font) {
                if named != font {
                    resolved = resolved.with_font_for(slot, named.clone());
                }
            }
        }
        Ok(resolved)
    }

    /// The surface's theme color scheme, resolved to concrete RGB — the interner-free bridge every
    /// effective reader builds once before walking parts.
    fn resolved_scheme_colors(&mut self, surface: Surface) -> Result<SchemeColors, PptxError> {
        let Some(part) = self.theme_part(surface)? else {
            return Ok(SchemeColors::default());
        };
        let doc = self.package.part_tree(&part)?;
        let theme = Theme::from_xml(&doc.root, &doc.interner)?;
        Ok(theme
            .color_scheme()
            .map(|scheme| SchemeColors::from_scheme(scheme, &doc.interner))
            .unwrap_or_default())
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

        let next_id = max_cnvpr_id(sp_tree, interner).max(1) + 1;
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

        let next_id = max_cnvpr_id(sp_tree, interner).max(1) + 1;
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
        shape_idx: usize,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let count = slide::shapes(sp_tree, interner).count();
        let position = slide::nth_shape_position(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        sp_tree.children.remove(position);
        // The shape's own indentation goes with it, or repeated removals leave a growing run of blank
        // lines behind. Only whitespace is dropped — never a comment or a sibling's text.
        if position > 0 && nav::is_whitespace_text(&sp_tree.children[position - 1]) {
            sp_tree.children.remove(position - 1);
        }
        Ok(())
    }

    /// Adds a new empty slide at the end of the deck, wired to the same slide layout as slide 0, and
    /// returns its index. The new slide is a blank shape tree; add content with
    /// [`add_text_box`](Self::add_text_box) or use [`add_slide_with_text`](Self::add_slide_with_text).
    ///
    /// This performs the package edits an added slide requires: it inserts the new slide part (with
    /// its content type), synthesizes the slide's relationships (to the layout), adds the
    /// presentation → slide relationship, and appends a `p:sldId` to `p:sldIdLst`. Every pre-existing
    /// part other than `presentation.xml` stays byte-identical.
    ///
    /// # Errors
    /// Returns [`PptxError::NoSlideLayout`] if the deck has no slide to inherit a layout from, or
    /// another [`PptxError`] if `presentation.xml` is malformed or a package edit fails.
    pub fn add_slide(&mut self) -> Result<usize, PptxError> {
        // Inherit slide 0's layout: reuse its relationship target verbatim (the new slide shares the
        // same directory, so the relative target resolves identically).
        let first_slide = self.slides.first().ok_or(PptxError::NoSlideLayout)?.clone();
        let layout_target = {
            let rels = self
                .package
                .relationships_for(Some(&first_slide))
                .ok_or(PptxError::NoSlideLayout)?;
            rels.by_type(constants::REL_SLIDE_LAYOUT)
                .next()
                .ok_or(PptxError::NoSlideLayout)?
                .target
                .clone()
        };
        self.insert_slide_part(&layout_target)
    }

    /// Adds a new slide at the end of the deck built on layout `layout_idx`, carrying a copy of every
    /// placeholder that layout declares, and returns the slide's index.
    ///
    /// This is how a deck is normally built: pick a layout (`Title and Content`, say — see
    /// [`layout_name`](Self::layout_name) and [`layout_kind`](Self::layout_kind)), then fill the
    /// placeholders it hands you with [`set_shape_text`](Self::set_shape_text). The cloned shapes are
    /// empty and carry no `p:spPr` content of their own, so their position, size and appearance all
    /// keep inheriting from the layout — editing the layout still moves them.
    ///
    /// The date, footer and slide-number slots are **not** cloned, which is what PowerPoint does: those
    /// three render *from the layout* precisely when a slide does not declare them, so a copy on the
    /// slide would suppress the layout's rendering and show an empty box instead. Every other slot the
    /// layout declares is cloned, in the layout's own order.
    ///
    /// # Errors
    /// Returns [`PptxError::LayoutIndexOutOfRange`] if `layout_idx` is out of range, or another
    /// [`PptxError`] if the layout is malformed or a package edit fails.
    pub fn add_slide_from_layout(&mut self, layout_idx: usize) -> Result<usize, PptxError> {
        let layout_part = self.layout_part_checked(layout_idx)?.clone();

        // The slots the layout offers a slide to fill, read before anything is inserted. Date, footer
        // and slide-number slots are excluded: they inherit-render from the layout, and a copy here
        // would replace that rendering with an empty box.
        let slots = {
            let doc = self.package.part_tree(&layout_part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            slide::shapes(sp_tree, &doc.interner)
                .filter_map(|shape| slide::shape_placeholder_info(shape, &doc.interner))
                .filter(|slot| !is_layout_rendered_slot(slot.kind))
                .collect::<Vec<_>>()
        };

        let new_part = self.next_slide_part()?;
        let layout_target = nav::relative_target(&new_part, &layout_part);
        let slide_idx = self.insert_slide_part(&layout_target)?;

        // Clone the slots into the new part, built with *its* interner (symbols are per-part). Ids
        // start at 2: the shape tree's own `p:cNvPr@id` is 1 (see `build::empty_slide_bytes`).
        let doc = self.package.part_tree_mut(&new_part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        for (n, slot) in slots.iter().enumerate() {
            let shape = build_placeholder(interner, n as u32 + 2, slot);
            sp_tree.children.push(RawNode::Element(shape));
        }
        sp_tree.empty = false;

        Ok(slide_idx)
    }

    /// Creates an empty slide part at the end of the deck, related to the layout at `layout_target`
    /// (a relationship target relative to the new slide part), and returns its slide index.
    ///
    /// This is the package work every "add a slide" entry point shares: insert the part with its
    /// content type, synthesize its `.rels` with the slideLayout relationship, add the presentation →
    /// slide relationship, and append a `p:sldId` to `p:sldIdLst`. Every pre-existing part other than
    /// `presentation.xml` stays byte-identical. Shapes are added afterwards, built with the new
    /// part's own interner.
    fn insert_slide_part(&mut self, layout_target: &str) -> Result<usize, PptxError> {
        let new_part = self.next_slide_part()?;
        let new_rid = self.next_presentation_rid()?;
        let slide_target = nav::relative_target(&self.presentation_part, &new_part);

        // 1. Insert the new slide part (registers its content-type Override).
        self.package.insert_part(
            &new_part,
            constants::CONTENT_TYPE_SLIDE,
            build::empty_slide_bytes(),
        )?;
        // 2. Synthesize the new slide's .rels with the slideLayout relationship.
        self.package.add_relationship(
            Some(&new_part),
            Relationship {
                id: "rId1".to_owned(),
                rel_type: constants::REL_SLIDE_LAYOUT.to_owned(),
                target: layout_target.to_owned(),
                mode: TargetMode::Internal,
            },
        )?;
        // 3. Add the presentation → slide relationship.
        self.package.add_relationship(
            Some(&self.presentation_part),
            Relationship {
                id: new_rid.clone(),
                rel_type: constants::REL_SLIDE.to_owned(),
                target: slide_target,
                mode: TargetMode::Internal,
            },
        )?;
        // 4. Append the p:sldId (with its r:id) to p:sldIdLst.
        self.append_sld_id(&new_rid)?;

        self.slides.push(new_part);
        Ok(self.slides.len() - 1)
    }

    /// Removes slide `slide_idx` from the deck, unwiring it completely: the `p:sldId` naming it, the
    /// presentation's relationship to it, the slide part, its own `.rels`, and its content-type
    /// `Override`.
    ///
    /// **Slide indices shift**: every later slide moves down one index, exactly as
    /// [`remove_shape`](Self::remove_shape) shifts shapes. Layout and master indices are unaffected —
    /// they are reached through `p:sldMasterIdLst`, which this does not touch. Slide part names are
    /// never recycled either: [`add_slide`](Self::add_slide) numbers a new part one past the highest
    /// `slideN.xml` in the package, so removing `slide2.xml` and adding a slide yields `slide3.xml`.
    ///
    /// Parts the slide alone referenced go with it — its notes slide (which holds a relationship
    /// *back* to the slide, so leaving it behind would leave a dangling reference) and any image no
    /// other part still shows. Anything shared with the rest of the deck stays. See
    /// [`Package::remove_part_cascading`](mjx_opc::Package::remove_part_cascading).
    ///
    /// # Errors
    /// Returns [`PptxError::SlideIndexOutOfRange`] if `slide_idx` is out of range,
    /// [`PptxError::MalformedPresentation`] if `presentation.xml` has no `p:sldIdLst`, no relationship
    /// to that slide, or no relationships namespace, or another [`PptxError`] if a package edit fails.
    pub fn remove_slide(&mut self, slide_idx: usize) -> Result<(), PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();

        // The presentation-scoped relationship naming this slide — matched by resolved target, since
        // the target string is relative and two spellings can name the same part.
        let rel_id = {
            let rels = self
                .package
                .relationships_for(Some(&self.presentation_part))
                .ok_or(PptxError::MalformedPresentation(
                    "presentation has no relationships",
                ))?;
            rels.by_type(constants::REL_SLIDE)
                .find(|rel| {
                    rel.mode == TargetMode::Internal
                        && nav::resolve_target(&self.presentation_part, &rel.target)
                            .is_ok_and(|resolved| resolved == slide_part)
                })
                .map(|rel| rel.id.clone())
                .ok_or(PptxError::MalformedPresentation(
                    "no presentation relationship names this slide",
                ))?
        };

        // Unwire in the reverse of the order `insert_slide_part` wired it up.
        self.remove_sld_id(&rel_id)?;
        self.package
            .remove_relationship(Some(&self.presentation_part), &rel_id)?;
        self.package.remove_part_cascading(&slide_part)?;
        self.slides.remove(slide_idx);
        Ok(())
    }

    /// Removes the `p:sldId` whose `r:id` is `rel_id` from `p:sldIdLst`, with the whitespace that
    /// indented it.
    fn remove_sld_id(&mut self, rel_id: &str) -> Result<(), PptxError> {
        let part = self.presentation_part.clone();
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;

        // Attribute namespaces are never resolved, so `r:id` is found through the prefix bound to the
        // relationships namespace (guardrail C).
        let rels_prefix = nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)
            .ok_or(PptxError::MalformedPresentation(
                "no relationships namespace declared",
            ))?;
        let sld_id_lst = nav::child_mut(root, interner, PML, "sldIdLst")
            .ok_or(PptxError::MalformedPresentation("missing p:sldIdLst"))?;

        let position = sld_id_lst
            .children
            .iter()
            .position(|child| match child {
                RawNode::Element(element) => {
                    nav::name_is(&element.name, interner, PML, "sldId")
                        && nav::prefixed_attr_value(element, interner, rels_prefix, "id")
                            .and_then(Result::ok)
                            .is_some_and(|id| id == rel_id)
                }
                _ => false,
            })
            .ok_or(PptxError::MalformedPresentation(
                "no p:sldId names this slide's relationship",
            ))?;
        sld_id_lst.children.remove(position);
        if position > 0 && nav::is_whitespace_text(&sld_id_lst.children[position - 1]) {
            sld_id_lst.children.remove(position - 1);
        }
        Ok(())
    }

    /// Appends a picture (`p:pic`) showing `bytes` to `surface`, laid out at `bounds`.
    /// Returns the index of the new shape in the slide's one shape index space (see
    /// [`shape_count`](Self::shape_count)); [`shape_kind`](Self::shape_kind) reports it as
    /// [`ShapeKind::Picture`], and the whole `p:spPr` surface — outline, effects, geometry — applies
    /// to it like any other shape.
    ///
    /// The image part and its relationship are created by [`add_image`](Self::add_image), so adding
    /// the same picture twice stores the bytes once. The image is stretched to fill `bounds`; since
    /// nothing here decodes the image, its natural size is unknown and the caller chooses the extent
    /// (the emitted `a:picLocks@noChangeAspect` keeps the ratio locked for later interactive resizing).
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range, the bytes match no known image format
    /// ([`UnrecognizedImageFormat`](PptxError::UnrecognizedImageFormat)), the slide is malformed, or a
    /// package edit fails.
    pub fn add_picture(
        &mut self,
        surface: impl Into<Surface>,
        bytes: &[u8],
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let surface = surface.into();
        // The image part and relationship first: if the bytes are not an image, nothing is edited.
        let rel_id = self.add_image(surface, bytes)?;

        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_declaration = build::relationship_prefix_declaration(root, interner);
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let picture = build_picture(interner, next_id, &rel_id, bounds, rel_declaration);
        sp_tree.children.push(RawNode::Element(picture));
        sp_tree.empty = false;

        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    // -----------------------------------------------------------------------------------------
    // Tables
    //
    // A table is what a `p:graphicFrame` frames, so it is a shape like any other on the index
    // space: it is positioned with `set_shape_bounds`, counted by `shape_count`, and removed by
    // `remove_shape`. What is addressed *inside* it is a cell, by `(row, column)`.
    //
    // Merging never removes a cell, so the grid is rectangular and every position within the table
    // is addressable — a cell covered by a merge is a real cell that simply renders nothing.
    // -----------------------------------------------------------------------------------------

    /// Adds a `rows` x `columns` table to `surface`, laid out inside `bounds`, and returns its
    /// index in the shape tree.
    ///
    /// Columns share the width evenly and rows the height; resize either afterwards with
    /// [`set_column_width`](Self::set_column_width) / [`set_row_height`](Self::set_row_height).
    /// Every cell starts with one empty paragraph, ready for
    /// [`set_cell_text`](Self::set_cell_text).
    ///
    /// The table is a shape: move it with [`set_shape_bounds`](Self::set_shape_bounds), and drop it
    /// with [`remove_shape`](Self::remove_shape).
    ///
    /// # Errors
    /// Returns [`PptxError::InvalidTableSize`] if either dimension is zero — a table with no cells
    /// is not something PowerPoint will open — or another [`PptxError`] if the surface index is out
    /// of range or the part is malformed.
    pub fn add_table(
        &mut self,
        surface: impl Into<Surface>,
        rows: usize,
        columns: usize,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        if rows == 0 || columns == 0 {
            return Err(PptxError::InvalidTableSize { rows, columns });
        }
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let frame = build_table_frame(interner, next_id, rows, columns, bounds);
        sp_tree.children.push(RawNode::Element(frame));
        sp_tree.empty = false;

        Ok(slide::shapes(sp_tree, interner).count() - 1)
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
        shape_idx: usize,
    ) -> Result<Option<GraphicFrameKind>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        Ok(slide::graphic_frame_uri(shape, &doc.interner).map(GraphicFrameKind::from_uri))
    }

    /// The shape of the table shape `shape_idx` on `surface` frames, as `(rows, columns)`.
    ///
    /// The column count comes from the table's `a:tblGrid`, which is where a table declares its
    /// width — not from counting some row's cells. Reading does not dirty the part.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotATable`] if the shape frames no table, or another
    /// [`PptxError`] if an index is out of range or the part is malformed.
    pub fn table_dimensions(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<(usize, usize), PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let _ = interner;
            Ok((table.row_count(), table.column_count()))
        })
    }

    /// The width of column `column` of the table shape `shape_idx` frames, or `None` if the column
    /// states none. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`] if there is no such column.
    pub fn column_width(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        column: usize,
    ) -> Result<Option<Emu>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let columns = table.column_count();
            let grid_column = table.grid().and_then(|grid| grid.column(column)).ok_or(
                PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows: table.row_count(),
                    columns,
                },
            )?;
            Ok(grid_column.width(interner))
        })
    }

    /// Sets the width of column `column`. Marks only that part dirty.
    ///
    /// The frame's own bounds are **not** adjusted: a table whose columns no longer sum to its
    /// frame width is what PowerPoint itself produces when a column is dragged, and the frame is
    /// resized separately with [`set_shape_bounds`](Self::set_shape_bounds).
    ///
    /// # Errors
    /// As [`column_width`](Self::column_width).
    pub fn set_column_width(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        column: usize,
        width: Emu,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let (rows, columns) = table_dimensions_of(table, interner);
            if column >= columns {
                return Err(PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows,
                    columns,
                });
            }
            let grid = nav::child_mut(table, interner, DML_MAIN, "tblGrid")
                .ok_or(PptxError::MalformedSlide("table has no a:tblGrid"))?;
            let slot = nav::nth_child_matching_mut(grid, interner, column, |element, interner| {
                nav::name_is(&element.name, interner, DML_MAIN, "gridCol")
            })
            .ok_or(PptxError::MalformedSlide("table column vanished"))?;
            // Through the model's own setter, so a width has one spelling in the codebase.
            let mut typed = TableColumn::from_xml(slot, interner)?;
            typed.set_width(interner, width);
            *slot = typed.to_xml(interner);
            Ok(())
        })
    }

    /// The height row `row` asks for, or `None` if it states none. PowerPoint grows a row whose
    /// content does not fit, so a rendered row is never shorter than this but may be taller.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`] if there is no such row.
    pub fn row_height(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
    ) -> Result<Option<Emu>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let rows = table.row_count();
            let table_row = table.row(row).ok_or(PptxError::TableCellOutOfRange {
                row,
                column: 0,
                rows,
                columns: table.column_count(),
            })?;
            Ok(table_row.height(interner))
        })
    }

    /// Sets the height row `row` asks for. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`row_height`](Self::row_height).
    pub fn set_row_height(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        height: Emu,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let (rows, columns) = table_dimensions_of(table, interner);
            if row >= rows {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column: 0,
                    rows,
                    columns,
                });
            }
            let slot = slide::nth_row_mut(table, interner, row)
                .ok_or(PptxError::MalformedSlide("table row vanished"))?;
            let mut typed = TableRow::from_xml(slot, interner)?;
            typed.set_height(interner, height);
            *slot = typed.to_xml(interner);
            Ok(())
        })
    }

    // ---------------------------------------------------------------------------------------------
    // Structural edits — grow and shrink a table by whole rows and columns.
    //
    // Unlike a cell text edit (which reaches one `a:tc` in the raw tree), a row or column edit
    // touches every row, so the whole `a:tbl` is parsed to the typed `Table`, mutated there — where
    // merge adjustment and anchor promotion are expressed in terms of the model — and written back.
    // Round-tripping the fidelity wrappers preserves everything this workstream does not model, and
    // the span-adjustment logic itself lives in `mjx-dml`. These wrappers own only the range checks,
    // which need the dimensions the model already reports.
    // ---------------------------------------------------------------------------------------------

    /// Inserts a row into the table shape `shape_idx` frames so it becomes row `row`; `row` equal to
    /// the current row count appends at the end. The new row copies the height of the row beside it
    /// and its cells are empty and ready for [`set_cell_text`](Self::set_cell_text). A merge the new
    /// row falls inside grows to include it. Marks only that part dirty; the frame's own bounds are
    /// **not** enlarged (as PowerPoint does not either — resize with
    /// [`set_shape_bounds`](Self::set_shape_bounds)).
    ///
    /// # Errors
    /// [`PptxError::TableCellOutOfRange`] if `row` is past the end, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn insert_row(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let rows = typed.row_count();
            if row > rows {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column: 0,
                    rows,
                    columns: typed.column_count(),
                });
            }
            typed.insert_row(interner, row, build_table_cell)?;
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// Removes row `row` from the table shape `shape_idx` frames. A merge the row lies inside shrinks;
    /// a merge anchored in the row promotes the cell below it, which takes over the anchor's text and
    /// formatting so the table looks unchanged. Marks only that part dirty.
    ///
    /// # Errors
    /// [`PptxError::InvalidTableSize`] if `row` is the table's only row (a table cannot have none),
    /// [`PptxError::TableCellOutOfRange`] if `row` is out of range, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn remove_row(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let (rows, columns) = (typed.row_count(), typed.column_count());
            if row >= rows {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column: 0,
                    rows,
                    columns,
                });
            }
            if rows == 1 {
                return Err(PptxError::InvalidTableSize { rows: 0, columns });
            }
            typed.remove_row(interner, row);
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// Inserts a column into the table shape `shape_idx` frames so it becomes column `column`;
    /// `column` equal to the current column count appends. The grid gains one `a:gridCol` (width
    /// copied from the column beside it) and every row gains one empty cell, so the grid and rows
    /// stay in step. A merge the new column falls inside grows to include it. Marks only that part
    /// dirty; the frame's own bounds are **not** enlarged.
    ///
    /// # Errors
    /// [`PptxError::TableCellOutOfRange`] if `column` is past the end, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn insert_column(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        column: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let columns = typed.column_count();
            if column > columns {
                return Err(PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows: typed.row_count(),
                    columns,
                });
            }
            typed.insert_column(interner, column, build_table_cell)?;
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// Removes column `column` from the table shape `shape_idx` frames: its `a:gridCol` and one cell
    /// from every row, together. A merge the column lies inside shrinks; a merge anchored in the
    /// column promotes the cell to its right, which takes over the anchor's text and formatting.
    /// Marks only that part dirty.
    ///
    /// # Errors
    /// [`PptxError::InvalidTableSize`] if `column` is the table's only column,
    /// [`PptxError::TableCellOutOfRange`] if `column` is out of range, plus the errors of
    /// [`table_dimensions`](Self::table_dimensions).
    pub fn remove_column(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        column: usize,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface.into(), shape_idx, |table, interner| {
            let mut typed = Table::from_xml(table, interner)?;
            let (rows, columns) = (typed.row_count(), typed.column_count());
            if column >= columns {
                return Err(PptxError::TableCellOutOfRange {
                    row: 0,
                    column,
                    rows,
                    columns,
                });
            }
            if columns == 1 {
                return Err(PptxError::InvalidTableSize { rows, columns: 0 });
            }
            typed.remove_column(interner, column);
            *table = typed.to_xml(interner);
            Ok(())
        })
    }

    /// How many columns and rows the cell at `(row, column)` spans, as `(columns, rows)`.
    ///
    /// `(1, 1)` for an ordinary cell. A cell **covered** by a merge also reports `(1, 1)` — ask
    /// [`merged_cell_anchor`](Self::merged_cell_anchor) which cell actually renders there.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`].
    pub fn cell_span(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<(usize, usize), PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            let cell = table
                .cell(row, column)
                .ok_or(PptxError::TableCellOutOfRange {
                    row,
                    column,
                    rows: table.row_count(),
                    columns: table.column_count(),
                })?;
            Ok((cell.column_span(interner), cell.row_span(interner)))
        })
    }

    /// Which cell actually renders at `(row, column)` — itself when it is not merged away, or the
    /// anchor of the merged region covering it.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), plus
    /// [`PptxError::TableCellOutOfRange`].
    pub fn merged_cell_anchor(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        row: usize,
        column: usize,
    ) -> Result<(usize, usize), PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            table
                .merge_anchor(interner, row, column)
                .ok_or(PptxError::TableCellOutOfRange {
                    row,
                    column,
                    rows: table.row_count(),
                    columns: table.column_count(),
                })
        })
    }

    /// Reads the table shape `shape_idx` frames as a typed [`Table`] and hands it, with the part's
    /// interner, to `read`. Does **not** dirty the part.
    fn with_table<R>(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        read: impl FnOnce(&Table, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let count = slide::shapes(sp_tree, &doc.interner).count();
        let shape = slide::shapes(sp_tree, &doc.interner).nth(shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let element =
            slide::shape_table(shape, &doc.interner).ok_or(PptxError::ShapeIsNotATable)?;
        let table = Table::from_xml(element, &doc.interner)?;
        read(&table, &doc.interner)
    }

    /// Hands the raw `a:tbl` of the table shape `shape_idx` frames to `edit`, which reaches the one
    /// child it means to change.
    ///
    /// The table element itself is not reparsed or rebuilt — only whatever `edit` replaces — so
    /// resizing a column costs one small element, not the whole table.
    fn edit_table_child(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        edit: impl FnOnce(&mut RawElement, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let count = slide::shapes(sp_tree, interner).count();
        let shape = slide::nth_shape_mut(sp_tree, interner, shape_idx).ok_or(
            PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            },
        )?;
        let table = slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
        edit(table, interner)
    }

    // ---------------------------------------------------------------------------------------------
    // Table styles and the seven a:tblPr flags.
    //
    // The flags (`firstRow`, `bandRow`, …) live on the table's own `a:tblPr`; they emphasize nothing
    // by themselves, they tell the table **style** which parts to treat specially. The style lives in
    // the presentation's `tableStyles.xml` part, named by GUID from `a:tblPr > a:tableStyleId`. This
    // block reads and writes both: the flags on the table, the style in the shared part.
    // ---------------------------------------------------------------------------------------------

    /// Whether the table shape `shape_idx` frames declares banding/emphasis `part` (a `a:tblPr` flag),
    /// or `None` if it does not state the flag. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn table_part(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        part: TablePart,
    ) -> Result<Option<bool>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            Ok(table
                .properties()
                .and_then(|props| props.part(interner, part)))
        })
    }

    /// Turns a table's banding/emphasis flag `part` on or off, creating its `a:tblPr` if it had none.
    /// `false` removes the flag rather than writing a `"0"`. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn set_table_part(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        part: TablePart,
        on: bool,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |props, interner| {
            props.set_part(interner, part, on);
            Ok(())
        })
    }

    /// The GUID of the table style the table shape `shape_idx` frames names (`a:tableStyleId`), or
    /// `None` if it names none. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn table_style_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        self.with_table(surface.into(), shape_idx, |table, interner| {
            Ok(table
                .properties()
                .and_then(|props| props.table_style_id(interner))
                .map(str::to_owned))
        })
    }

    /// Points the table shape `shape_idx` frames at the table style `style_id`, creating its
    /// `a:tblPr` if it had none. Does not check that the style exists — pair it with
    /// [`create_table_style`](Self::create_table_style). Marks only that part dirty.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn set_table_style(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        style_id: &str,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |props, interner| {
            props.set_table_style_id(interner, style_id);
            Ok(())
        })
    }

    /// Creates the presentation's `tableStyles.xml` part if it has none, and adds a style with GUID
    /// `style_id` and gallery name `style_name` — replacing one already carrying that GUID. The style
    /// is born empty; give its parts formatting with
    /// [`format_table_style_part`](Self::format_table_style_part), and point a table at it with
    /// [`set_table_style`](Self::set_table_style).
    ///
    /// # Errors
    /// Returns a [`PptxError`] if the package is malformed or the part cannot be created.
    pub fn create_table_style(
        &mut self,
        style_id: &str,
        style_name: &str,
    ) -> Result<(), PptxError> {
        let part = self.ensure_table_styles_part(style_id)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut list = TableStyleList::from_xml(root, interner)?;
        let style = TableStyle::new(interner, style_id, style_name);
        list.upsert_style(interner, &style);
        *root = list.to_xml(interner);
        Ok(())
    }

    /// Sets the formatting the style `style_id` gives table `part` (`wholeTbl`, `firstRow`, a banded
    /// row, a corner cell). Only the facets `format` sets are written; the part keeps whatever else
    /// it held. Marks only the `tableStyles.xml` part dirty.
    ///
    /// # Errors
    /// [`PptxError::TableStyleNotFound`] if no `tableStyles.xml` defines `style_id`.
    pub fn format_table_style_part(
        &mut self,
        style_id: &str,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), PptxError> {
        let not_found = || PptxError::TableStyleNotFound {
            style_id: style_id.to_owned(),
        };
        let part_name = self.table_styles_part()?.ok_or_else(not_found)?;
        let doc = self.package.part_tree_mut(&part_name)?;
        let RawDocument { interner, root, .. } = doc;
        let mut list = TableStyleList::from_xml(root, interner)?;
        let mut style = list.style(interner, style_id).ok_or_else(not_found)?;
        let mut part_style = style
            .part(interner, part)
            .unwrap_or_else(|| TablePartStyle::new(interner));
        format.apply(&mut part_style, interner);
        style.set_part(interner, part, &part_style);
        list.upsert_style(interner, &style);
        *root = list.to_xml(interner);
        Ok(())
    }

    /// Gives the table shape `shape_idx` frames its own **inline** style (`a:tableStyle`), replacing
    /// any inline or referenced style it had — the lean alternative to a shared `tableStyles.xml`
    /// style: the whole look is spelled out in `definition` and travels with the table, so no shared
    /// part, relationship or referenced GUID is involved. Marks only that part dirty.
    ///
    /// A styled part renders only when the table declares it: pair this with
    /// [`set_table_part`](Self::set_table_part) to turn on the `firstRow` / `bandRow` / … flags a part
    /// needs (a table from [`add_table`](Self::add_table) already has `firstRow` and `bandRow` on).
    /// The style resolves through [`with_table_style`](Self::with_table_style) and the
    /// `effective_cell_*` readers exactly as a shared one does.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn set_inline_table_style(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        definition: &TableStyleDefinition,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |properties, interner| {
            let mut style =
                TableStyle::new(interner, definition.style_id(), definition.style_name());
            for (part, format) in definition.parts() {
                let mut part_style = TablePartStyle::new(interner);
                format.apply(&mut part_style, interner);
                style.set_part(interner, *part, &part_style);
            }
            properties.set_inline_style(interner, &style);
            Ok(())
        })
    }

    /// Sets the formatting the table's **inline** style gives one `part`, creating the inline style if
    /// the table had none — the incremental sibling of [`set_inline_table_style`](Self::set_inline_table_style),
    /// mirroring [`format_table_style_part`](Self::format_table_style_part) for a self-contained style.
    /// Only the facets `format` sets are written. Marks only that part dirty.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions).
    pub fn format_inline_table_style_part(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        part: TableStylePart,
        format: &TableStyleFormat,
    ) -> Result<(), PptxError> {
        self.edit_table_properties(surface.into(), shape_idx, |properties, interner| {
            let mut style = properties.inline_style(interner).unwrap_or_else(|| {
                TableStyle::new(
                    interner,
                    crate::table::DEFAULT_INLINE_STYLE_ID,
                    crate::table::DEFAULT_INLINE_STYLE_NAME,
                )
            });
            let mut part_style = style
                .part(interner, part)
                .unwrap_or_else(|| TablePartStyle::new(interner));
            format.apply(&mut part_style, interner);
            style.set_part(interner, part, &part_style);
            properties.set_inline_style(interner, &style);
            Ok(())
        })
    }

    /// Reads the table style the table shape `shape_idx` frames resolves to and hands it, with the
    /// `tableStyles.xml` interner, to `read`. `None` when the table names no style or the named style
    /// is not defined. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`table_dimensions`](Self::table_dimensions), or if the package is malformed.
    pub fn with_table_style<R>(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        read: impl FnOnce(&TableStyle, &Interner) -> Result<R, PptxError>,
    ) -> Result<Option<R>, PptxError> {
        self.with_resolved_style(surface.into(), shape_idx, read)
    }

    /// The style a table resolves to, handed to `read` — an **inline** `a:tableStyle` if the table
    /// carries one, else the shared style its `a:tableStyleId` names. `None` when it resolves to
    /// neither. An inline style is read against the slide part's interner (where it lives), a shared
    /// one against the `tableStyles.xml` interner; either way the [`TableStyle`] model is the same.
    fn with_resolved_style<R>(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        read: impl FnOnce(&TableStyle, &Interner) -> Result<R, PptxError>,
    ) -> Result<Option<R>, PptxError> {
        // An inline style wins, and lives in the slide part — the `TableStyle` is owned, but its
        // symbols resolve against that part's interner, which re-opening the part hands back.
        let inline = self.with_table(surface, shape_idx, |table, interner| {
            Ok(table
                .properties()
                .and_then(|properties| properties.inline_style(interner)))
        })?;
        if let Some(style) = inline {
            let part = self.surface_part(surface)?;
            let doc = self.package.part_tree(&part)?;
            return read(&style, &doc.interner).map(Some);
        }

        let Some(style_id) = self.table_style_id(surface, shape_idx)? else {
            return Ok(None);
        };
        let Some(part) = self.table_styles_part()? else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&part)?;
        let list = TableStyleList::from_xml(&doc.root, &doc.interner)?;
        match list.style(&doc.interner, &style_id) {
            Some(style) => read(&style, &doc.interner).map(Some),
            None => Ok(None),
        }
    }

    /// Reads the table's `a:tblPr` (creating it if absent) as a typed [`TableProperties`], hands it to
    /// `edit`, and writes it back. Only the `a:tblPr` is reparsed — the rest of the table is untouched.
    fn edit_table_properties(
        &mut self,
        surface: Surface,
        shape_idx: usize,
        edit: impl FnOnce(&mut TableProperties, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        self.edit_table_child(surface, shape_idx, |table, interner| {
            let slot = table_properties_slot(table, interner)?;
            let mut typed = TableProperties::from_xml(slot, interner)?;
            edit(&mut typed, interner)?;
            *slot = typed.to_xml(interner);
            Ok(())
        })
    }

    /// The presentation's `tableStyles.xml` part, or `None` if it has none.
    fn table_styles_part(&self) -> Result<Option<PartName>, PptxError> {
        self.follow_rel(&self.presentation_part, constants::REL_TABLE_STYLES)
    }

    /// The `tableStyles.xml` part, creating it (with an empty list whose default is `default_style_id`)
    /// and wiring its relationship and content type if the presentation had none.
    fn ensure_table_styles_part(&mut self, default_style_id: &str) -> Result<PartName, PptxError> {
        if let Some(part) = self.table_styles_part()? {
            return Ok(part);
        }
        let part = PartName::new(&format!(
            "{}tableStyles.xml",
            dir_of(self.presentation_part.as_str())
        ))?;
        self.package.insert_part(
            &part,
            constants::CONTENT_TYPE_TABLE_STYLES,
            build::table_styles_bytes(default_style_id),
        )?;
        let rel_id = self.next_presentation_rid()?;
        let target = nav::relative_target(&self.presentation_part, &part);
        self.package.add_relationship(
            Some(&self.presentation_part),
            Relationship {
                id: rel_id,
                rel_type: constants::REL_TABLE_STYLES.to_owned(),
                target,
                mode: TargetMode::Internal,
            },
        )?;
        Ok(part)
    }

    /// The relationship id of the image that picture `shape_idx` on `surface` embeds
    /// (`p:blipFill > a:blip@r:embed`), or `None` when the blip embeds nothing — a picture may instead
    /// *link* an external image (`@r:link`), which this does not resolve. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// Returns [`PptxError::ShapeIsNotAPicture`] if the shape is not a `p:pic`,
    /// [`PptxError::PictureHasNoBlipFill`] if it is missing its `p:blipFill`, or another
    /// [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn picture_image_rel_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
        let picture = picture_at(sp_tree, &doc.interner, surface, shape_idx)?;
        let blip_fill = nav::child(picture, &doc.interner, PML, "blipFill")
            .ok_or(PptxError::PictureHasNoBlipFill)?;
        let blip_fill = BlipFill::from_xml(blip_fill, &doc.interner)?;
        Ok(blip_fill.image_rel_id(&doc.interner).map(str::to_owned))
    }

    /// The stored bytes of the image that picture `shape_idx` on `surface` embeds, exactly as
    /// the package holds them (never decoded or re-encoded), or `None` when the picture embeds no
    /// image. Borrowed from the package, so a large image is not copied.
    ///
    /// # Errors
    /// As [`picture_image_rel_id`](Self::picture_image_rel_id), plus
    /// [`PptxError::ExternalTarget`] if the relationship points outside the package.
    pub fn picture_image_bytes(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(rel_id) = self.picture_image_rel_id(surface, shape_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let Some(part) = self.image_part_for_rel(&slide_part, &rel_id)? else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&part))
    }

    /// Points picture `shape_idx` on `surface` at `bytes`, adding the image to the package if
    /// it is not already there ([`add_image`](Self::add_image), so identical bytes are stored once)
    /// and rewriting the blip's `@r:embed`. Any `@r:link` is dropped — the picture now embeds its
    /// image — and the rest of the `p:blipFill` (source rect, tile/stretch) is preserved.
    ///
    /// The previously embedded image part is **left in the package**: another shape may still show it,
    /// and sweeping unreferenced parts is a package-wide graph operation, not this method's job. An
    /// unreferenced part is legal and simply unused.
    ///
    /// # Errors
    /// As [`picture_image_rel_id`](Self::picture_image_rel_id), plus
    /// [`UnrecognizedImageFormat`](PptxError::UnrecognizedImageFormat) if the bytes match no known
    /// image format.
    pub fn set_picture_image(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: usize,
        bytes: &[u8],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        // Validate the shape kind before editing the package, so a wrong index adds no image part.
        {
            let slide_part = self.surface_part(surface)?;
            let doc = self.package.part_tree(&slide_part)?;
            let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
            let picture = picture_at(sp_tree, &doc.interner, surface, shape_idx)?;
            if nav::child(picture, &doc.interner, PML, "blipFill").is_none() {
                return Err(PptxError::PictureHasNoBlipFill);
            }
        }
        let rel_id = self.add_image(surface, bytes)?;

        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_prefix = nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)
            .unwrap_or_else(|| interner.intern(build::RELATIONSHIP_PREFIX));
        let sp_tree = slide::sp_tree_mut(root, interner)?;
        let picture = slide::nth_shape_mut(sp_tree, interner, shape_idx)
            .ok_or(PptxError::ShapeIsNotAPicture)?;
        let blip_fill = nav::child_mut(picture, interner, PML, "blipFill")
            .ok_or(PptxError::PictureHasNoBlipFill)?;
        let blip = nav::child_mut(blip_fill, interner, DML_MAIN, "blip")
            .ok_or(PptxError::PictureHasNoBlipFill)?;

        // Attribute namespaces are unresolved, so the embed/link attributes are matched by local name.
        blip.attributes
            .retain(|attr| interner.resolve(attr.name.local) != "link");
        let embed = build::attr_prefixed(interner, rel_prefix, "embed", &rel_id);
        match blip
            .attributes
            .iter()
            .position(|attr| interner.resolve(attr.name.local) == "embed")
        {
            Some(index) => blip.attributes[index] = embed,
            None => blip.attributes.push(embed),
        }
        Ok(())
    }

    /// The part an image relationship of `source` points at, or `None` if there is no such
    /// relationship. Errors if it points outside the package.
    fn image_part_for_rel(
        &self,
        source: &PartName,
        rel_id: &str,
    ) -> Result<Option<PartName>, PptxError> {
        let Some(rels) = self.package.relationships_for(Some(source)) else {
            return Ok(None);
        };
        let Some(rel) = rels.by_id(rel_id) else {
            return Ok(None);
        };
        if rel.mode == TargetMode::External {
            return Err(PptxError::ExternalTarget {
                target: rel.target.clone(),
            });
        }
        Ok(Some(nav::resolve_target(source, &rel.target)?))
    }

    /// Stores `bytes` as an image part of the package and relates it to `surface`, returning
    /// the **slide-scoped relationship id** that names the image — the `rel_id` to hand to
    /// [`FillSpec::Blip`] via [`set_shape_fill`](Self::set_shape_fill).
    ///
    /// The format is identified from the bytes ([`ImageFormat::sniff`]), which decides the media part's
    /// extension and its content type; the bytes themselves are stored verbatim and never re-encoded.
    /// The part is named `media/image{N}.{ext}` beside the presentation part, with `N` one past the
    /// largest existing image number.
    ///
    /// **Identical images are stored once**: if a media part already holds exactly these bytes it is
    /// reused, and if that surface already relates to it, the existing relationship id is returned and
    /// the package is not touched at all. Otherwise only `[Content_Types].xml`, the new media part, and
    /// that part's `.rels` change — every other pre-existing part stays byte-identical.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range,
    /// [`PptxError::UnrecognizedImageFormat`] if the bytes match no known image format, or another
    /// [`PptxError`] if a package edit fails.
    pub fn add_image(
        &mut self,
        surface: impl Into<Surface>,
        bytes: &[u8],
    ) -> Result<String, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let format = ImageFormat::sniff(bytes).ok_or(PptxError::UnrecognizedImageFormat)?;

        let media_part = match self.media_part_with_bytes(bytes) {
            Some(existing) => {
                // Already stored: reuse the slide's relationship to it when there is one.
                if let Some(id) = self.image_rel_id_for(&slide_part, &existing)? {
                    return Ok(id);
                }
                existing
            }
            None => {
                let part = self.next_media_part(format.file_extension())?;
                // Registering the Default first means `insert_part` adds no per-part Override.
                self.package
                    .set_content_type_default(format.file_extension(), format.content_type())?;
                self.package
                    .insert_part(&part, format.content_type(), bytes.to_vec())?;
                part
            }
        };

        let rel_id = self.next_rid_for(&slide_part);
        self.package.add_relationship(
            Some(&slide_part),
            Relationship {
                id: rel_id.clone(),
                rel_type: constants::REL_IMAGE.to_owned(),
                target: nav::relative_target(&slide_part, &media_part),
                mode: TargetMode::Internal,
            },
        )?;
        Ok(rel_id)
    }

    /// The media part whose stored bytes equal `bytes`, if the package already holds one. Comparing
    /// slices short-circuits on length, so this is a cheap scan even for large images.
    fn media_part_with_bytes(&self, bytes: &[u8]) -> Option<PartName> {
        let media_dir = format!("{}media/", dir_of(self.presentation_part.as_str()));
        self.package
            .part_names()
            .filter(|part| part.as_str().starts_with(&media_dir))
            .find(|part| self.package.part_bytes(part) == Some(bytes))
    }

    /// The id of `source`'s existing [`REL_IMAGE`](constants::REL_IMAGE) relationship pointing at
    /// `target`, or `None` if it has none.
    fn image_rel_id_for(
        &self,
        source: &PartName,
        target: &PartName,
    ) -> Result<Option<String>, PptxError> {
        let Some(rels) = self.package.relationships_for(Some(source)) else {
            return Ok(None);
        };
        for rel in rels.by_type(constants::REL_IMAGE) {
            if rel.mode == TargetMode::External {
                continue; // a linked image never names a part in this package
            }
            if &nav::resolve_target(source, &rel.target)? == target {
                return Ok(Some(rel.id.clone()));
            }
        }
        Ok(None)
    }

    /// A fresh image part name in the presentation's `media/` directory: `image{N}.{extension}` with
    /// `N` one past the largest existing image number, whatever its extension.
    fn next_media_part(&self, extension: &str) -> Result<PartName, PptxError> {
        let media_dir = format!("{}media/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = image_number(part.as_str(), &media_dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{media_dir}image{}.{extension}", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }

    /// Adds a new slide (via [`add_slide`](Self::add_slide)) carrying a single text box with `text`
    /// laid out at `bounds`, and returns the new slide's index.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the slide cannot be added (see [`add_slide`](Self::add_slide)).
    pub fn add_slide_with_text(
        &mut self,
        text: &str,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        let idx = self.add_slide()?;
        self.add_text_box(idx, text, bounds)?;
        Ok(idx)
    }

    /// A fresh slide part name in slide 0's directory: `slide{N}.xml` with `N` one past the largest
    /// existing slide number.
    fn next_slide_part(&self) -> Result<PartName, PptxError> {
        let first = self.slides.first().ok_or(PptxError::NoSlideLayout)?;
        let dir = dir_of(first.as_str());
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = slide_number(part.as_str(), dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{dir}slide{}.xml", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }

    /// The next free presentation-scoped relationship id (`rId{N}`), one past the current maximum.
    fn next_presentation_rid(&self) -> Result<String, PptxError> {
        if self
            .package
            .relationships_for(Some(&self.presentation_part))
            .is_none()
        {
            return Err(PptxError::MalformedPresentation(
                "presentation has no relationships",
            ));
        }
        Ok(self.next_rid_for(&self.presentation_part))
    }

    /// The next free relationship id (`rId{N}`) in `part`'s `.rels`, one past the current maximum —
    /// `rId1` when the part has no relationships yet (a slide need not have any).
    fn next_rid_for(&self, part: &PartName) -> String {
        let mut max_n = 0u32;
        if let Some(rels) = self.package.relationships_for(Some(part)) {
            for rel in rels.iter() {
                if let Some(n) = rel
                    .id
                    .strip_prefix("rId")
                    .and_then(|s| s.parse::<u32>().ok())
                {
                    max_n = max_n.max(n);
                }
            }
        }
        format!("rId{}", max_n + 1)
    }

    /// Appends `<p:sldId id=".." r:id="new_rid"/>` to `p:sldIdLst`, choosing the next slide id (≥256,
    /// one past the largest existing `p:sldId@id` — masters in `p:sldMasterIdLst` are not considered).
    fn append_sld_id(&mut self, new_rid: &str) -> Result<(), PptxError> {
        let part = self.presentation_part.clone();
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;

        // The `r:id` prefix: attribute namespaces are not resolved by the reader, so find the prefix
        // bound to the relationships namespace.
        let rels_prefix = nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)
            .ok_or(PptxError::MalformedPresentation(
                "no relationships namespace declared",
            ))?;
        let sld_id_lst = nav::child_mut(root, interner, PML, "sldIdLst")
            .ok_or(PptxError::MalformedPresentation("missing p:sldIdLst"))?;

        let mut max_id = 255u32;
        for child in &sld_id_lst.children {
            if let RawNode::Element(element) = child {
                if nav::name_is(&element.name, interner, PML, "sldId") {
                    if let Some(id) = element
                        .attributes
                        .iter()
                        .find(|attr| {
                            attr.name.prefix.is_none() && interner.resolve(attr.name.local) == "id"
                        })
                        .and_then(|attr| std::str::from_utf8(&attr.value).ok())
                        .and_then(|value| value.parse::<u32>().ok())
                    {
                        max_id = max_id.max(id);
                    }
                }
            }
        }
        let new_id = max_id + 1;

        let attrs = vec![
            build::attr(interner, "id", &new_id.to_string()),
            build::attr_prefixed(interner, rels_prefix, "id", new_rid),
        ];
        let sld_id = build::leaf(interner, "p", PML, "sldId", attrs);
        sld_id_lst.children.push(RawNode::Element(sld_id));
        sld_id_lst.empty = false;
        Ok(())
    }

    fn slide_part_checked(&self, slide_idx: usize) -> Result<&PartName, PptxError> {
        self.slides
            .get(slide_idx)
            .ok_or(PptxError::SlideIndexOutOfRange {
                index: slide_idx,
                count: self.slides.len(),
            })
    }

    /// Follows the single relationship of type `rel_type` from `part` to a target [`PartName`], or
    /// `None` if `part` has no such relationship. Errors if the relationship points outside the
    /// package. This is the shared hop used to walk slide → layout → master → theme.
    fn follow_rel(&self, part: &PartName, rel_type: &str) -> Result<Option<PartName>, PptxError> {
        let Some(rels) = self.package.relationships_for(Some(part)) else {
            return Ok(None);
        };
        let Some(rel) = rels.by_type(rel_type).next() else {
            return Ok(None);
        };
        if rel.mode == TargetMode::External {
            return Err(PptxError::ExternalTarget {
                target: rel.target.clone(),
            });
        }
        Ok(Some(nav::resolve_target(part, &rel.target)?))
    }

    /// The effective theme [`ColorMap`] for `surface`: the master's `p:clrMap` (reached along the
    /// surface's inheritance chain), replaced by the surface's own `p:clrMapOvr >
    /// a:overrideClrMapping` when it supplies a full mapping (a `masterClrMapping`, an absent override,
    /// or a schema-loose attribute-less override all inherit the master's map). It maps the logical
    /// color names a shape may reference (`bg1`/`tx1`/…) to the theme's concrete scheme slots.
    /// `Ok(None)` when there is no reachable master or no `p:clrMap`. Reading does not dirty a part.
    ///
    /// A master surface has no override of its own, so it simply reports its `p:clrMap`.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the surface index is out of range, a relationship points outside the
    /// package ([`ExternalTarget`](PptxError::ExternalTarget)), or a part is not well-formed.
    pub fn color_map(
        &mut self,
        surface: impl Into<Surface>,
    ) -> Result<Option<ColorMap>, PptxError> {
        let surface = surface.into();
        let chain = self.inheritance_chain(surface)?;
        let own = chain[0].clone();
        let master = chain
            .last()
            .expect("a chain always holds the surface's own part")
            .clone();
        if master == own && !surface.is_master_like() {
            return Ok(None); // the chain never reached a master
        }

        let base = {
            let doc = self.package.part_tree(&master)?;
            nav::child(&doc.root, &doc.interner, PML, "clrMap")
                .and_then(|clr_map| slide::parse_color_map(clr_map, &doc.interner))
        };
        let Some(base) = base else {
            return Ok(None);
        };
        if own == master {
            return Ok(Some(base));
        }

        let doc = self.package.part_tree(&own)?;
        let effective = nav::child(&doc.root, &doc.interner, PML, "clrMapOvr")
            .and_then(|ovr| nav::child(ovr, &doc.interner, DML_MAIN, "overrideClrMapping"))
            .and_then(|mapping| slide::parse_color_map(mapping, &doc.interner))
            .unwrap_or(base);
        Ok(Some(effective))
    }
}

/// The level a paragraph is read at (`a:pPr@lvl`), or [`IndentLevel::TOP`] when it states none.
///
/// Read **once**, before the walk: it selects which `a:lvlNpPr` every list-style tier contributes, so
/// every tier must be asked about the same level. A paragraph index past the end reads as the top
/// level rather than failing — the caller's own index error surfaces from the tier that needs it.
fn paragraph_level(body: &TextBody, para_idx: usize, interner: &Interner) -> IndentLevel {
    nth_paragraph(body, para_idx)
        .ok()
        .and_then(|paragraph| paragraph.properties())
        .and_then(|properties| properties.level(interner))
        .unwrap_or(IndentLevel::TOP)
}

/// One list-style tier: the properties `list_style` defines at `level`, as an interner-free spec with
/// its colors baked. Yields nothing when the style defines nothing there — an absent tier contributes
/// no value rather than an empty one, so the fold stays honest about which tiers spoke.
fn list_style_tier(
    list_style: Option<&TextListStyle>,
    level: IndentLevel,
    scheme: &SchemeColors,
    map: &ColorMap,
    interner: &Interner,
) -> Option<ParagraphPropertiesSpec> {
    let properties = list_style?.level(interner, level)?;
    Some(resolved_paragraph_spec(&properties, scheme, map, interner))
}

/// A tier's paragraph properties as an interner-free spec, with the colors of its `a:defRPr` resolved
/// to concrete RGB (`ParagraphProperties::spec` leaves a scheme color a scheme color).
fn resolved_paragraph_spec(
    properties: &ParagraphProperties,
    scheme: &SchemeColors,
    map: &ColorMap,
    interner: &Interner,
) -> ParagraphPropertiesSpec {
    let spec = properties.spec(interner);
    match properties.default_run_properties(interner) {
        Some(default) => spec.with_default_run_properties(resolve_character_properties(
            &default, scheme, map, None, interner,
        )),
        None => spec,
    }
}

/// Which of a master's three text styles governs a placeholder slot: titles are styled by
/// `p:titleStyle`, the date / footer / slide-number chrome by `p:otherStyle`, and everything else —
/// body, subtitle, object, chart, table — by `p:bodyStyle`.
fn master_style_local(slot: slide::Placeholder) -> &'static str {
    if slot.is_title_family() {
        return "titleStyle";
    }
    match slot.kind {
        PlaceholderType::DateAndTime
        | PlaceholderType::Footer
        | PlaceholderType::SlideNumber
        | PlaceholderType::Header => "otherStyle",
        _ => "bodyStyle",
    }
}

// ---------------------------------------------------------------------------------------------
// Text-body operations
//
// Each of these is one text operation, named once. A shape's `p:txBody` and a table cell's
// `a:txBody` are the same `CT_TextBody`, so the public surface spells the two apart while every
// operation below has exactly one definition — adding a cell method is delegation, not a second
// implementation, and a new text feature stays a single change.
// ---------------------------------------------------------------------------------------------

/// The number of typed paragraphs in a body.
fn paragraph_count_of(body: &TextBody) -> usize {
    body.paragraphs().count()
}

/// The number of typed runs in one paragraph.
fn run_count_of(body: &TextBody, para_idx: usize) -> Result<usize, PptxError> {
    Ok(nth_paragraph(body, para_idx)?.runs().count())
}

/// One paragraph's text — its runs concatenated.
fn paragraph_text_of(body: &TextBody, para_idx: usize) -> Result<String, PptxError> {
    Ok(nth_paragraph(body, para_idx)?.text())
}

/// One run's text.
fn run_text_of(body: &TextBody, para_idx: usize, run_idx: usize) -> Result<String, PptxError> {
    let paragraph = nth_paragraph(body, para_idx)?;
    Ok(nth_run(paragraph, run_idx)?.text().to_owned())
}

/// The layout properties a paragraph declares of its own.
fn paragraph_properties_of(
    body: &TextBody,
    interner: &Interner,
    para_idx: usize,
) -> Result<Option<ParagraphPropertiesSpec>, PptxError> {
    Ok(nth_paragraph(body, para_idx)?
        .properties()
        .map(|properties| properties.spec(interner)))
}

/// The character properties a run declares of its own.
fn run_properties_of(
    body: &TextBody,
    interner: &Interner,
    para_idx: usize,
    run_idx: usize,
) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
    let paragraph = nth_paragraph(body, para_idx)?;
    Ok(nth_run(paragraph, run_idx)?
        .properties()
        .map(|properties| properties.spec(interner)))
}

/// The paragraph-mark properties (`a:endParaRPr`) a paragraph declares.
fn end_run_properties_of(
    body: &TextBody,
    interner: &Interner,
    para_idx: usize,
) -> Result<Option<CharacterPropertiesSpec>, PptxError> {
    Ok(nth_paragraph(body, para_idx)?
        .end_properties()
        .map(|properties| properties.spec(interner)))
}

/// Applies `spec` to one run.
fn set_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    run_idx: usize,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    let paragraph = nth_paragraph_mut(body, para_idx)?;
    let count = paragraph.runs().count();
    let run = paragraph
        .runs_mut()
        .nth(run_idx)
        .ok_or(PptxError::RunIndexOutOfRange {
            index: run_idx,
            count,
        })?;
    run.set_properties(spec, interner);
    Ok(())
}

/// Applies `spec` to every run of one paragraph, and to its paragraph mark.
fn set_paragraph_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    let paragraph = nth_paragraph_mut(body, para_idx)?;
    apply_to_paragraph(paragraph, spec, interner);
    Ok(())
}

/// Applies `spec` to every run of every paragraph, and to each paragraph mark.
fn set_all_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    for paragraph in body.paragraphs_mut() {
        apply_to_paragraph(paragraph, spec, interner);
    }
    Ok(())
}

/// Applies `spec` to a paragraph's mark (`a:endParaRPr`), creating the element if absent.
fn set_end_run_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    nth_paragraph_mut(body, para_idx)?.set_end_properties(spec, interner);
    Ok(())
}

/// Applies `spec` to a paragraph's layout properties (`a:pPr`), creating the element if absent.
fn set_paragraph_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    spec: &ParagraphPropertiesSpec,
) -> Result<(), PptxError> {
    nth_paragraph_mut(body, para_idx)?.set_properties(spec, interner);
    Ok(())
}

/// Applies `spec` to a scalar-offset range within one paragraph, splitting runs at its edges.
fn set_range_properties_in(
    body: &mut TextBody,
    interner: &mut Interner,
    para_idx: usize,
    range: core::ops::Range<usize>,
    spec: &CharacterPropertiesSpec,
) -> Result<(), PptxError> {
    let paragraph = nth_paragraph_mut(body, para_idx)?;
    apply_to_scalar_range(paragraph, range, spec, interner)
}

/// Replaces the text of the `run_idx`-th run of `body`, flattened over its paragraphs in document
/// order — what `set_shape_text` and `set_cell_text` both mean by "set the text".
fn set_run_text(body: &mut TextBody, run_idx: usize, text: &str) -> Result<(), PptxError> {
    let count = body
        .paragraphs()
        .flat_map(|paragraph| paragraph.runs())
        .count();
    let run = body
        .paragraphs_mut()
        .flat_map(|paragraph| paragraph.runs_mut())
        .nth(run_idx)
        .ok_or(PptxError::RunIndexOutOfRange {
            index: run_idx,
            count,
        })?;
    if !run.set_text(text) {
        return Err(PptxError::RunHasNoText);
    }
    Ok(())
}

/// A `TextSite` naming one cell of the table a shape frames.
fn cell(shape: usize, row: usize, column: usize) -> TextSite {
    TextSite::Cell { shape, row, column }
}

/// Checks that no merged region touching the rectangle reaches outside it.
///
/// A region wholly inside is fine — it is absorbed. One that crosses the boundary is not, because
/// truncating it would leave the table claiming a span that no longer fits, and growing the
/// selection to swallow it would merge cells the caller never named.
fn check_merges_fit(
    table: &Table,
    interner: &Interner,
    rows: &core::ops::Range<usize>,
    columns: &core::ops::Range<usize>,
) -> Result<(), PptxError> {
    for row in rows.clone() {
        for column in columns.clone() {
            let Some((anchor_row, anchor_column)) = table.merge_anchor(interner, row, column)
            else {
                continue;
            };
            let Some(anchor) = table.cell(anchor_row, anchor_column) else {
                continue;
            };
            let reaches_row = anchor_row + anchor.row_span(interner);
            let reaches_column = anchor_column + anchor.column_span(interner);
            let contained = anchor_row >= rows.start
                && reaches_row <= rows.end
                && anchor_column >= columns.start
                && reaches_column <= columns.end;
            if !contained {
                return Err(PptxError::TableMergeCrossesSelection { row, column });
            }
        }
    }
    Ok(())
}

/// The `a:tcPr` of a raw `a:tc`, creating it when the cell has none — placed after the cell's
/// `a:txBody`, since `CT_TableCell` is a sequence.
fn cell_properties_slot<'a>(
    cell: &'a mut RawElement,
    interner: &mut Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let index = match cell.children.iter().position(|node| match node {
        RawNode::Element(element) => nav::name_is(&element.name, interner, DML_MAIN, "tcPr"),
        _ => false,
    }) {
        Some(index) => index,
        None => {
            let at = cell
                .children
                .iter()
                .position(|node| match node {
                    RawNode::Element(element) => {
                        !nav::name_is(&element.name, interner, DML_MAIN, "txBody")
                    }
                    _ => false,
                })
                .unwrap_or(cell.children.len());
            let element = build::leaf(interner, "a", DML_MAIN, "tcPr", Vec::new());
            cell.children.insert(at, RawNode::Element(element));
            cell.empty = false;
            at
        }
    };
    match &mut cell.children[index] {
        RawNode::Element(element) => Ok(element),
        _ => Err(PptxError::MalformedSlide(
            "cell properties are not an element",
        )),
    }
}

/// The `a:tblPr` of a raw `a:tbl`, creating it when the table has none — placed **first**, since
/// `CT_Table` is a sequence of `tblPr?`, `tblGrid`, `tr*`.
fn table_properties_slot<'a>(
    table: &'a mut RawElement,
    interner: &mut Interner,
) -> Result<&'a mut RawElement, PptxError> {
    let index = match table.children.iter().position(|node| match node {
        RawNode::Element(element) => nav::name_is(&element.name, interner, DML_MAIN, "tblPr"),
        _ => false,
    }) {
        Some(index) => index,
        None => {
            let element = build::leaf(interner, "a", DML_MAIN, "tblPr", Vec::new());
            table.children.insert(0, RawNode::Element(element));
            table.empty = false;
            0
        }
    };
    match &mut table.children[index] {
        RawNode::Element(element) => Ok(element),
        _ => Err(PptxError::MalformedSlide(
            "table properties are not an element",
        )),
    }
}

/// Writes the properties a [`CellFormat`] names onto one cell's `a:tcPr`, leaving the rest alone.
fn apply_cell_format(
    properties: &mut TableCellProperties,
    interner: &mut Interner,
    format: &CellFormat,
) {
    if let Some(fill) = format.fill() {
        properties.set_fill(interner, fill);
    }
    for (edge, line) in format.borders() {
        properties.set_border(interner, *edge, line.as_ref());
    }
    let margins = format.margins();
    properties.set_margins(
        interner,
        margins.left,
        margins.right,
        margins.top,
        margins.bottom,
    );
    let (anchor, direction, overflow) = format.framing();
    if let Some(anchor) = anchor {
        properties.set_anchor(interner, anchor);
    }
    if let Some(direction) = direction {
        properties.set_text_direction(interner, direction);
    }
    if let Some(overflow) = overflow {
        properties.set_horizontal_overflow(interner, overflow);
    }
}

/// Which text body an index-addressed text call is about.
///
/// A shape's `p:txBody` and a table cell's `a:txBody` are the *same* `CT_TextBody`, so every text
/// operation applies to either; this is how the private locators say which one. The public surface
/// spells the two apart (`shape_text` / `cell_text`), but the logic below them exists once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextSite {
    /// The shape's own text body.
    Shape(usize),
    /// A cell of the table the shape frames.
    Cell {
        /// The graphic frame's index in the shape tree.
        shape: usize,
        /// The cell's row.
        row: usize,
        /// The cell's column.
        column: usize,
    },
}

impl TextSite {
    /// The shape this site is inside, whichever kind it is.
    fn shape_index(self) -> usize {
        match self {
            Self::Shape(index) | Self::Cell { shape: index, .. } => index,
        }
    }
}

/// The text body `site` names within `shape`.
fn locate_text_body<'a>(
    shape: &'a RawElement,
    interner: &Interner,
    site: TextSite,
) -> Result<&'a RawElement, PptxError> {
    match site {
        TextSite::Shape(_) => {
            slide::shape_txbody(shape, interner).ok_or(PptxError::ShapeHasNoTextBody)
        }
        TextSite::Cell { row, column, .. } => {
            let table = slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
            let cell = table_cell(table, interner, row, column)?;
            nav::child(cell, interner, DML_MAIN, "txBody").ok_or(PptxError::ShapeHasNoTextBody)
        }
    }
}

/// The text body `site` names within `shape`, mutably.
fn locate_text_body_mut<'a>(
    shape: &'a mut RawElement,
    interner: &Interner,
    site: TextSite,
) -> Result<&'a mut RawElement, PptxError> {
    match site {
        TextSite::Shape(_) => {
            nav::child_mut(shape, interner, PML, "txBody").ok_or(PptxError::ShapeHasNoTextBody)
        }
        TextSite::Cell { row, column, .. } => {
            // The bounds are checked against an immutable view first, so the error can report the
            // table's real shape before the tree is borrowed mutably.
            let (rows, columns) = {
                let table =
                    slide::shape_table(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
                table_dimensions_of(table, interner)
            };
            if row >= rows || column >= columns {
                return Err(PptxError::TableCellOutOfRange {
                    row,
                    column,
                    rows,
                    columns,
                });
            }
            let table =
                slide::shape_table_mut(shape, interner).ok_or(PptxError::ShapeIsNotATable)?;
            let row_element = slide::nth_row_mut(table, interner, row)
                .ok_or(PptxError::MalformedSlide("table row vanished"))?;
            let cell = slide::nth_cell_mut(row_element, interner, column)
                .ok_or(PptxError::MalformedSlide("table cell vanished"))?;
            nav::child_mut(cell, interner, DML_MAIN, "txBody").ok_or(PptxError::ShapeHasNoTextBody)
        }
    }
}

/// The cell at `(row, column)` of a raw `a:tbl`, or a typed out-of-range error naming the table's
/// real shape.
fn table_cell<'a>(
    table: &'a RawElement,
    interner: &Interner,
    row: usize,
    column: usize,
) -> Result<&'a RawElement, PptxError> {
    let (rows, columns) = table_dimensions_of(table, interner);
    let out_of_range = || PptxError::TableCellOutOfRange {
        row,
        column,
        rows,
        columns,
    };
    if row >= rows || column >= columns {
        return Err(out_of_range());
    }
    let row_element = slide::nth_dml_child(table, interner, "tr", row).ok_or_else(out_of_range)?;
    slide::nth_dml_child(row_element, interner, "tc", column).ok_or_else(out_of_range)
}

/// A raw `a:tbl`'s dimensions: its row count, and its column count **as the grid declares it**
/// (`a:tblGrid` is where a table states its width, not any row's cell count).
///
/// A table this model cannot parse reports `(0, 0)` rather than failing — the callers all turn that
/// into an out-of-range error naming the shape, which is the more useful thing to say.
fn table_dimensions_of(table: &RawElement, interner: &Interner) -> (usize, usize) {
    Table::from_xml(table, interner)
        .map(|table| (table.row_count(), table.column_count()))
        .unwrap_or_default()
}

/// How to locate a candidate shape within a part's shape tree while resolving an effective property.
#[derive(Debug, Clone, Copy)]
enum Candidate {
    /// The originally-requested shape, by index (the surface's own part).
    Index(usize),
    /// The matching placeholder on an ancestor part (layout / master).
    Placeholder(slide::Placeholder),
}

/// Resolves a [`Candidate`] to the shape it names in `doc`, or `None` when that part has no such
/// shape — an ancestor that simply does not define the slot, which every effective walk treats as
/// "this tier says nothing" and steps past.
///
/// Takes the document rather than the package so the caller owns the borrow and can extract what it
/// needs before the next candidate is fetched.
fn candidate_shape(
    doc: &RawDocument,
    candidate: Candidate,
) -> Result<Option<&RawElement>, PptxError> {
    let sp_tree = slide::sp_tree(&doc.root, &doc.interner)?;
    Ok(match candidate {
        Candidate::Index(idx) => slide::shapes(sp_tree, &doc.interner).nth(idx),
        Candidate::Placeholder(ph) => slide::find_placeholder(sp_tree, ph, &doc.interner),
    })
}

/// A candidate shape's own fill, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved fill, a theme style reference to resolve against the theme, or
/// no fill.
enum OwnFill {
    /// An explicit `p:spPr` fill, already resolved to concrete colors.
    Resolved(FillSpec),
    /// A `p:style > a:fillRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no fill of its own.
    Absent,
}

// ---------------------------------------------------------------------------------------------
// Text-body helpers — one place for "find the body, do the thing, put it back"
// ---------------------------------------------------------------------------------------------

/// The `para_idx`-th paragraph of a body, or a typed out-of-range error.
fn nth_paragraph(body: &TextBody, para_idx: usize) -> Result<&mjx_dml::Paragraph, PptxError> {
    let count = body.paragraphs().count();
    body.paragraphs()
        .nth(para_idx)
        .ok_or(PptxError::ParagraphIndexOutOfRange {
            index: para_idx,
            count,
        })
}

/// The `run_idx`-th run of a paragraph, or a typed out-of-range error.
fn nth_run(paragraph: &mjx_dml::Paragraph, run_idx: usize) -> Result<&mjx_dml::TextRun, PptxError> {
    let count = paragraph.runs().count();
    paragraph
        .runs()
        .nth(run_idx)
        .ok_or(PptxError::RunIndexOutOfRange {
            index: run_idx,
            count,
        })
}

/// The `para_idx`-th paragraph of a body, mutably.
fn nth_paragraph_mut(
    body: &mut TextBody,
    para_idx: usize,
) -> Result<&mut mjx_dml::Paragraph, PptxError> {
    let count = body.paragraphs().count();
    body.paragraphs_mut()
        .nth(para_idx)
        .ok_or(PptxError::ParagraphIndexOutOfRange {
            index: para_idx,
            count,
        })
}

/// Applies `spec` to every run of `paragraph` and to its paragraph mark, so text typed at the end
/// takes the same formatting.
fn apply_to_paragraph(
    paragraph: &mut mjx_dml::Paragraph,
    spec: &CharacterPropertiesSpec,
    interner: &mut Interner,
) {
    for run in paragraph.runs_mut() {
        run.set_properties(spec, interner);
    }
    if paragraph.end_properties().is_some() {
        paragraph.set_end_properties(spec, interner);
    }
}

/// Splits `paragraph`'s runs at the range's boundaries, then applies `spec` to every run that now
/// falls wholly inside it.
fn apply_to_scalar_range(
    paragraph: &mut mjx_dml::Paragraph,
    range: core::ops::Range<usize>,
    spec: &CharacterPropertiesSpec,
    interner: &mut Interner,
) -> Result<(), PptxError> {
    let length = paragraph.text().chars().count();
    if range.start > range.end || range.end > length {
        return Err(PptxError::TextRangeOutOfBounds {
            start: range.start,
            end: range.end,
            length,
        });
    }
    if range.start == range.end {
        return Ok(()); // An empty selection formats nothing.
    }

    // Split at the far boundary first: splitting at the near one would shift everything after it,
    // while the far offset is expressed in the *original* coordinates.
    split_at_offset(paragraph, range.end);
    split_at_offset(paragraph, range.start);

    // After the splits every run lies wholly inside or wholly outside the range, so a running count
    // of scalars is enough to tell which.
    let mut consumed = 0;
    let mut targets = Vec::new();
    for (index, run) in paragraph.runs().enumerate() {
        let len = run.text().chars().count();
        if consumed >= range.start && consumed + len <= range.end {
            targets.push(index);
        }
        consumed += len;
    }
    for index in targets {
        if let Some(run) = paragraph.runs_mut().nth(index) {
            run.set_properties(spec, interner);
        }
    }
    Ok(())
}

/// Splits whichever run contains the paragraph-level scalar `offset`, unless it already falls on a
/// run boundary — where there is nothing to split.
fn split_at_offset(paragraph: &mut mjx_dml::Paragraph, offset: usize) {
    let mut consumed = 0;
    let mut target = None;
    for (index, run) in paragraph.runs().enumerate() {
        let len = run.text().chars().count();
        if offset > consumed && offset < consumed + len {
            target = Some((index, offset - consumed));
            break;
        }
        consumed += len;
    }
    if let Some((index, within)) = target {
        paragraph.split_run_at(index, within);
    }
}

/// Converts a grapheme-cluster range into the scalar range covering the same text.
fn grapheme_range_to_scalars(
    text: &str,
    range: &core::ops::Range<usize>,
) -> Result<core::ops::Range<usize>, PptxError> {
    use unicode_segmentation::UnicodeSegmentation;

    let clusters: Vec<&str> = text.graphemes(true).collect();
    if range.start > range.end || range.end > clusters.len() {
        return Err(PptxError::TextRangeOutOfBounds {
            start: range.start,
            end: range.end,
            length: clusters.len(),
        });
    }
    let scalars_before = |count: usize| -> usize {
        clusters[..count]
            .iter()
            .map(|cluster| cluster.chars().count())
            .sum()
    };
    Ok(scalars_before(range.start)..scalars_before(range.end))
}

/// The fill a `shape` declares itself (explicit `p:spPr` fill, or a `p:style > a:fillRef`), resolved
/// against `scheme` / `map`. The style-reference case returns its index + resolved color for the
/// caller to resolve against the theme (which lives in a different part interner).
fn shape_own_fill(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnFill, PptxError> {
    if let Some(fill_element) = slide::shape_fill(shape, interner) {
        let fill = Fill::from_xml(fill_element, interner)?;
        return Ok(OwnFill::Resolved(resolve_fill(
            &fill, scheme, map, None, interner,
        )));
    }
    if let Some(reference) = slide::shape_fill_ref(shape, interner) {
        if let Some(idx) = reference.idx().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnFill::StyleRef(idx, color));
        }
    }
    Ok(OwnFill::Absent)
}

/// A candidate shape's own outline, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved outline, a theme style reference to resolve against the theme,
/// or no outline.
enum OwnLine {
    /// An explicit `p:spPr > a:ln`, already resolved to a concrete stroke color.
    Resolved(LineSpec),
    /// A `p:style > a:lnRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no outline of its own.
    Absent,
}

/// The outline a `shape` declares itself (explicit `p:spPr > a:ln`, or a `p:style > a:lnRef`), resolved
/// against `scheme` / `map`. The style-reference case returns its index + resolved color for the caller
/// to resolve against the theme (which lives in a different part interner).
fn shape_own_line(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnLine, PptxError> {
    if let Some(line_element) = slide::shape_line(shape, interner) {
        let line = LineProperties::from_xml(line_element, interner)?;
        return Ok(OwnLine::Resolved(resolve_line(
            &line, scheme, map, None, interner,
        )));
    }
    if let Some(reference) = slide::shape_line_ref(shape, interner) {
        if let Some(idx) = reference.idx().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnLine::StyleRef(idx, color));
        }
    }
    Ok(OwnLine::Absent)
}

// --- Effective cell formatting helpers -----------------------------------------------------------

/// Whether a raw `a:tc` is covered by a merge (states a truthy `hMerge` or `vMerge`), so it renders
/// nothing. A checked cheaply off the attributes, without parsing the whole cell — formatting a
/// selection skips such cells, though merging and unmerging must still reach them.
fn raw_cell_is_covered(cell: &RawElement, interner: &Interner) -> bool {
    cell.attributes.iter().any(|attribute| {
        attribute.name.prefix.is_none()
            && matches!(interner.resolve(attribute.name.local), "hMerge" | "vMerge")
            && matches!(
                std::str::from_utf8(&attribute.value).map(str::trim),
                Ok("1" | "true" | "on")
            )
    })
}

/// The cell at `(row, column)`, or a typed out-of-range error naming the table's shape.
fn cell_at(table: &Table, row: usize, column: usize) -> Result<&TableCell, PptxError> {
    let (rows, columns) = (table.row_count(), table.column_count());
    table
        .cell(row, column)
        .ok_or(PptxError::TableCellOutOfRange {
            row,
            column,
            rows,
            columns,
        })
}

/// The table's banding/emphasis flags, or all-false when it declares no `a:tblPr`.
fn table_flags(table: &Table, interner: &Interner) -> TableStyleFlags {
    table
        .properties()
        .map(|properties| TableStyleFlags::from_properties(properties, interner))
        .unwrap_or_default()
}

/// A style part's cell fill: an explicit fill (baked) or a theme `a:fillRef` (index + resolved
/// `phClr` substitute), mirroring [`shape_own_fill`] for a `a:tcStyle`.
fn part_own_fill(
    cell_style: &TableStyleCellStyle,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> OwnFill {
    if let Some(fill) = cell_style.fill(interner) {
        return OwnFill::Resolved(resolve_fill(&fill, scheme, map, None, interner));
    }
    if let Some(reference) = cell_style.fill_reference(interner) {
        if let Some(idx) = reference.idx().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|color| resolve_color(color, scheme, map, None, interner));
            return OwnFill::StyleRef(idx, color);
        }
    }
    OwnFill::Absent
}

/// A themeable border line: an explicit `a:ln` (baked) or a theme `a:lnRef` (index + resolved colour).
fn part_own_line(
    border: ThemeableLineStyle,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> OwnLine {
    match border {
        ThemeableLineStyle::Line(line) => {
            OwnLine::Resolved(resolve_line(&line, scheme, map, None, interner))
        }
        ThemeableLineStyle::Reference(reference) => match reference.idx().filter(|idx| *idx > 0) {
            Some(idx) => {
                let color = reference
                    .color()
                    .and_then(|color| resolve_color(color, scheme, map, None, interner));
                OwnLine::StyleRef(idx, color)
            }
            None => OwnLine::Absent,
        },
    }
}

/// Which `a:tcBdr` edge draws a cell's `edge`: the outer edge (`top`/`left`/…) for a cell on the
/// table's rim, the interior edge (`insideH`/`insideV`) for one within it; diagonals map straight
/// across.
fn style_border_key(
    edge: CellBorder,
    row: usize,
    column: usize,
    rows: usize,
    columns: usize,
) -> TableStyleBorder {
    match edge {
        CellBorder::Left if column == 0 => TableStyleBorder::Left,
        CellBorder::Left => TableStyleBorder::InsideVertical,
        CellBorder::Right if column + 1 == columns => TableStyleBorder::Right,
        CellBorder::Right => TableStyleBorder::InsideVertical,
        CellBorder::Top if row == 0 => TableStyleBorder::Top,
        CellBorder::Top => TableStyleBorder::InsideHorizontal,
        CellBorder::Bottom if row + 1 == rows => TableStyleBorder::Bottom,
        CellBorder::Bottom => TableStyleBorder::InsideHorizontal,
        CellBorder::TopLeftToBottomRight => TableStyleBorder::TopLeftToBottomRight,
        CellBorder::BottomLeftToTopRight => TableStyleBorder::TopRightToBottomLeft,
        // `CellBorder` is `#[non_exhaustive]`; the six edges above are its entire present set, so this
        // is unreachable today — a future edge falls back to an interior vertical rather than panic.
        _ => TableStyleBorder::InsideVertical,
    }
}

/// A table style's text contribution as an interner-free spec: its take on bold/italic (the tri-state
/// [`OnOffStyle`], `Default` contributing nothing) and its text colour, baked to concrete `RRGGBB`.
fn style_text_spec(
    text_style: &TableStyleTextStyle,
    scheme: &SchemeColors,
    map: &ColorMap,
    interner: &Interner,
) -> CharacterPropertiesSpec {
    let mut spec = CharacterPropertiesSpec::new();
    match text_style.bold(interner) {
        OnOffStyle::On => spec = spec.with_bold(true),
        OnOffStyle::Off => spec = spec.with_bold(false),
        OnOffStyle::Default => {}
    }
    match text_style.italic(interner) {
        OnOffStyle::On => spec = spec.with_italic(true),
        OnOffStyle::Off => spec = spec.with_italic(false),
        OnOffStyle::Default => {}
    }
    if let Some(color) = text_style.color(interner) {
        if let Some(resolved) = resolve_color(&color, scheme, map, None, interner) {
            spec = spec.with_fill(FillSpec::Solid(ColorSpec::Srgb(resolved.to_hex())));
        }
    }
    spec
}

/// A candidate shape's own effects, extracted while its part's tree is borrowed (fully owned, so no
/// borrow escapes): an already-resolved effect list, a theme style reference to resolve against the
/// theme, or no effects.
enum OwnEffects {
    /// An explicit `p:spPr > a:effectLst`, already resolved to concrete colors. Boxed — an
    /// [`EffectListSpec`] is far larger than the other variants.
    Resolved(Box<EffectListSpec>),
    /// A `p:style > a:effectRef@idx` with its (already-resolved) `phClr` substitute color.
    StyleRef(u32, Option<ResolvedColor>),
    /// The shape declares no effects of its own.
    Absent,
}

/// The effects a `shape` declares itself (explicit `p:spPr > a:effectLst`, or a `p:style > a:effectRef`),
/// resolved against `scheme` / `map`. The style-reference case returns its index + resolved color for the
/// caller to resolve against the theme (which lives in a different part interner).
fn shape_own_effects(
    shape: &RawElement,
    interner: &Interner,
    scheme: &SchemeColors,
    map: &ColorMap,
) -> Result<OwnEffects, PptxError> {
    if let Some(effect_element) = slide::shape_effects(shape, interner) {
        let effects = EffectList::from_xml(effect_element, interner)?;
        return Ok(OwnEffects::Resolved(Box::new(resolve_effects(
            &effects, scheme, map, None, interner,
        ))));
    }
    if let Some(reference) = slide::shape_effect_ref(shape, interner) {
        if let Some(idx) = reference.idx().filter(|idx| *idx > 0) {
            let color = reference
                .color()
                .and_then(|c| resolve_color(c, scheme, map, None, interner));
            return Ok(OwnEffects::StyleRef(idx, color));
        }
    }
    Ok(OwnEffects::Absent)
}

/// The directory portion of an absolute part name, including the trailing `/` (e.g.
/// `/ppt/slides/slide1.xml` → `/ppt/slides/`).
fn dir_of(part: &str) -> &str {
    let end = part.rfind('/').map_or(0, |idx| idx + 1);
    &part[..end]
}

/// Extracts `N` from a `slide{N}.xml` part directly inside `dir` (e.g. `/ppt/slides/slide3.xml` with
/// `dir = /ppt/slides/` → `3`). Returns `None` for anything else (e.g. the `_rels` subfolder).
fn slide_number(part: &str, dir: &str) -> Option<u32> {
    part.strip_prefix(dir)?
        .strip_prefix("slide")?
        .strip_suffix(".xml")?
        .parse::<u32>()
        .ok()
}

/// Extracts `N` from a `notesSlide{N}.xml` part directly inside `dir`
/// (e.g. `/ppt/notesSlides/notesSlide2.xml` with `dir = /ppt/notesSlides/` → `2`).
fn notes_slide_number(part: &str, dir: &str) -> Option<u32> {
    part.strip_prefix(dir)?
        .strip_prefix("notesSlide")?
        .strip_suffix(".xml")?
        .parse::<u32>()
        .ok()
}

/// The parts referenced by one of PresentationML's `r:id` lists — `p:sldIdLst > p:sldId`,
/// `p:sldMasterIdLst > p:sldMasterId`, `p:sldLayoutIdLst > p:sldLayoutId` — in document order.
///
/// Each item names a relationship of `source`; the ids are collected first so the tree borrow ends
/// before the relationships are consulted. An absent list yields no parts (a master need not list
/// layouts); a *present* item with no `r:id`, or an id no relationship matches, is an error, since
/// that is a broken reference rather than an absence.
fn referenced_parts(
    package: &mut Package,
    source: &PartName,
    list_local: &str,
    item_local: &str,
) -> Result<Vec<PartName>, PptxError> {
    let rids: Vec<String> = {
        let doc = package.part_tree(source)?;
        let interner = &doc.interner;
        let rels_prefix = nav::namespace_prefix(&doc.root, interner, SHARED_RELATIONSHIP_REFERENCE)
            .ok_or(PptxError::MalformedPresentation(
                "no relationships namespace declared",
            ))?;
        let Some(list) = nav::child(&doc.root, interner, PML, list_local) else {
            return Ok(Vec::new());
        };
        let mut rids = Vec::new();
        for item in nav::children(list, interner, PML, item_local) {
            rids.push(
                nav::prefixed_attr_value(item, interner, rels_prefix, "id").ok_or(
                    PptxError::MalformedPresentation("id list entry has no r:id"),
                )??,
            );
        }
        rids
    };

    let rels = package
        .relationships_for(Some(source))
        .ok_or(PptxError::MalformedPresentation(
            "presentation has no relationships",
        ))?;
    let mut parts = Vec::with_capacity(rids.len());
    for rid in &rids {
        let rel = rels
            .by_id(rid)
            .ok_or_else(|| PptxError::SlideRelNotFound { id: rid.clone() })?;
        if rel.mode == TargetMode::External {
            return Err(PptxError::ExternalTarget {
                target: rel.target.clone(),
            });
        }
        parts.push(nav::resolve_target(source, &rel.target)?);
    }
    Ok(parts)
}

/// The `p:pic` at `shape_idx` in `sp_tree`, or [`PptxError::ShapeIsNotAPicture`] when that index
/// addresses a shape of another kind (the one index space covers every kind).
fn picture_at<'a>(
    sp_tree: &'a RawElement,
    interner: &'a Interner,
    surface: Surface,
    shape_idx: usize,
) -> Result<&'a RawElement, PptxError> {
    let count = slide::shapes(sp_tree, interner).count();
    let shape =
        slide::shapes(sp_tree, interner)
            .nth(shape_idx)
            .ok_or(PptxError::ShapeIndexOutOfRange {
                surface,
                index: shape_idx,
                count,
            })?;
    match slide::shape_kind(shape, interner) {
        Some(ShapeKind::Picture) => Ok(shape),
        _ => Err(PptxError::ShapeIsNotAPicture),
    }
}

/// Extracts `N` from an `image{N}.{ext}` part directly inside `dir` (e.g. `/ppt/media/image3.png`
/// with `dir = /ppt/media/` → `3`), whatever the extension. Returns `None` for anything else.
fn image_number(part: &str, dir: &str) -> Option<u32> {
    let name = part.strip_prefix(dir)?.strip_prefix("image")?;
    let digits = &name[..name.find('.').unwrap_or(name.len())];
    digits.parse::<u32>().ok()
}

/// The largest `p:cNvPr@id` anywhere under `sp_tree` (0 if none). Non-visual ids are unique per
/// slide, so the next free id is one past this maximum.
fn max_cnvpr_id(sp_tree: &RawElement, interner: &Interner) -> u32 {
    fn walk(element: &RawElement, interner: &Interner, max: &mut u32) {
        if nav::name_is(&element.name, interner, PML, "cNvPr") {
            if let Some(id) = element
                .attributes
                .iter()
                .find(|attr| {
                    attr.name.prefix.is_none() && interner.resolve(attr.name.local) == "id"
                })
                .and_then(|attr| std::str::from_utf8(&attr.value).ok())
                .and_then(|value| value.parse::<u32>().ok())
            {
                *max = (*max).max(id);
            }
        }
        for child in &element.children {
            if let RawNode::Element(child) = child {
                walk(child, interner, max);
            }
        }
    }
    let mut max = 0;
    walk(sp_tree, interner, &mut max);
    max
}

/// Builds a plain text-box `p:sp` with non-visual id `id`, laid out at `bounds`, whose text body
/// holds one paragraph per line of `text`.
/// `p:nvSpPr` — non-visual shape properties: `p:cNvPr@id,name`, `p:cNvSpPr` (with `txBox="1"` iff
/// `tx_box`), and an empty `p:nvPr`.
fn build_nv_sp_pr(interner: &mut Interner, id: u32, name: &str, tx_box: bool) -> RawElement {
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", name),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let cnvsppr_attrs = if tx_box {
        vec![build::attr(interner, "txBox", "1")]
    } else {
        Vec::new()
    };
    let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", cnvsppr_attrs);
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    build::node(
        interner,
        "p",
        PML,
        "nvSpPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_sp_pr),
            RawNode::Element(nv_pr),
        ],
    )
}

/// `p:spPr` — visual shape properties: an `a:xfrm` transform at `bounds` plus `a:prstGeom@prst` with
/// an empty `a:avLst` (the preset's default adjustments).
fn build_sp_pr(interner: &mut Interner, prst: &str, bounds: ShapeBounds) -> RawElement {
    // One spelling of an `a:xfrm` in the crate: creating a shape and moving one go through the
    // same writer, so a built transform and an edited transform cannot drift apart.
    let mut xfrm = Transform2D::empty_element(interner);
    bounds.to_transform().apply(&mut xfrm, interner);
    let av_lst = build::leaf(interner, "a", DML_MAIN, "avLst", Vec::new());
    let prstgeom_attrs = vec![build::attr(interner, "prst", prst)];
    let prst_geom = build::node(
        interner,
        "a",
        DML_MAIN,
        "prstGeom",
        prstgeom_attrs,
        vec![RawNode::Element(av_lst)],
    );
    build::node(
        interner,
        "p",
        PML,
        "spPr",
        Vec::new(),
        vec![RawNode::Element(xfrm), RawNode::Element(prst_geom)],
    )
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

/// `p:txBody` — the required `a:bodyPr` + `a:lstStyle`, then `paragraphs`.
fn build_text_body(interner: &mut Interner, paragraphs: Vec<RawElement>) -> RawElement {
    build_body(interner, "p", PML, paragraphs)
}

/// Replaces `shape`'s `p:txBody` with `new_body`, preserving its position among the shape's children;
/// appends it if the shape had none. Used to overwrite a notes body placeholder's text wholesale.
fn replace_txbody(shape: &mut RawElement, interner: &Interner, new_body: RawElement) {
    let existing = shape.children.iter().position(|child| {
        matches!(child, RawNode::Element(element) if nav::name_is(&element.name, interner, PML, "txBody"))
    });
    match existing {
        Some(index) => shape.children[index] = RawNode::Element(new_body),
        None => {
            shape.children.push(RawNode::Element(new_body));
            shape.empty = false;
        }
    }
}

/// A `CT_TextBody` under whichever name its container gives it — `p:txBody` in a shape, `a:txBody`
/// in a table cell. The content is identical, which is the whole reason one text model serves both.
fn build_body(
    interner: &mut Interner,
    prefix: &str,
    namespace: mjx_ooxml_types::namespaces::SchemaNamespace,
    paragraphs: Vec<RawElement>,
) -> RawElement {
    let body_pr = build::leaf(interner, "a", DML_MAIN, "bodyPr", Vec::new());
    let lst_style = build::leaf(interner, "a", DML_MAIN, "lstStyle", Vec::new());
    let mut children = vec![RawNode::Element(body_pr), RawNode::Element(lst_style)];
    children.extend(paragraphs.into_iter().map(RawNode::Element));
    build::node(interner, prefix, namespace, "txBody", Vec::new(), children)
}

/// One fresh `a:tc`: an `a:txBody` with one empty paragraph and an empty `a:tcPr` — what both a
/// created table's cells and a cell inserted by a row/column edit are born as. A caller's first act
/// is `set_cell_text`; formatting is added afterwards with `format_cells`, never inherited here.
fn build_table_cell(interner: &mut Interner) -> RawElement {
    let paragraph = build_paragraph(interner, "");
    let body = build_body(interner, "a", DML_MAIN, vec![paragraph]);
    let tc_pr = build::leaf(interner, "a", DML_MAIN, "tcPr", Vec::new());
    build::node(
        interner,
        "a",
        DML_MAIN,
        "tc",
        Vec::new(),
        vec![RawNode::Element(body), RawNode::Element(tc_pr)],
    )
}

/// A whole `p:graphicFrame` holding a `rows` x `columns` table, laid out inside `bounds`.
///
/// Columns share the width evenly and rows the height — a caller resizes either afterwards. Each
/// cell gets an `a:txBody` with one empty paragraph, because PowerPoint expects a cell to have one
/// and a caller's first act is to put text in it. `firstRow` and `bandRow` are what PowerPoint
/// itself writes for a new table: they claim nothing about appearance on their own, they tell a
/// table style which parts to emphasize.
fn build_table_frame(
    interner: &mut Interner,
    id: u32,
    rows: usize,
    columns: usize,
    bounds: ShapeBounds,
) -> RawElement {
    // p:nvGraphicFramePr — cNvPr, cNvGraphicFramePr (locked against grouping, as Office writes it),
    // and an empty nvPr.
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", &format!("Table {id}")),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let lock_attrs = vec![build::attr(interner, "noGrp", "1")];
    let frame_locks = build::leaf(interner, "a", DML_MAIN, "graphicFrameLocks", lock_attrs);
    let c_nv_frame_pr = build::node(
        interner,
        "p",
        PML,
        "cNvGraphicFramePr",
        Vec::new(),
        vec![RawNode::Element(frame_locks)],
    );
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    let nv_frame_pr = build::node(
        interner,
        "p",
        PML,
        "nvGraphicFramePr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_frame_pr),
            RawNode::Element(nv_pr),
        ],
    );

    // p:xfrm — a graphic frame's transform is PresentationML's, not DrawingML's, and is required.
    let mut xfrm = build::node(interner, "p", PML, "xfrm", Vec::new(), Vec::new());
    bounds.to_transform().apply(&mut xfrm, interner);

    // a:tblGrid — the grid is where a table declares its width.
    let column_width = bounds.width_emu / columns.max(1) as i64;
    let grid_columns: Vec<RawNode> = (0..columns)
        .map(|index| {
            // The last column absorbs the rounding, so the columns sum to the frame's width.
            let width = if index + 1 == columns {
                bounds.width_emu - column_width * (columns as i64 - 1)
            } else {
                column_width
            };
            let attrs = vec![build::attr(interner, "w", &width.to_string())];
            RawNode::Element(build::leaf(interner, "a", DML_MAIN, "gridCol", attrs))
        })
        .collect();
    let grid = build::node(interner, "a", DML_MAIN, "tblGrid", Vec::new(), grid_columns);

    let row_height = bounds.height_emu / rows.max(1) as i64;
    let table_rows: Vec<RawNode> = (0..rows)
        .map(|_| {
            let cells: Vec<RawNode> = (0..columns)
                .map(|_| RawNode::Element(build_table_cell(interner)))
                .collect();
            let attrs = vec![build::attr(interner, "h", &row_height.to_string())];
            RawNode::Element(build::node(interner, "a", DML_MAIN, "tr", attrs, cells))
        })
        .collect();

    let tbl_pr_attrs = vec![
        build::attr(interner, "firstRow", "1"),
        build::attr(interner, "bandRow", "1"),
    ];
    let tbl_pr = build::leaf(interner, "a", DML_MAIN, "tblPr", tbl_pr_attrs);
    let mut table_children = vec![RawNode::Element(tbl_pr), RawNode::Element(grid)];
    table_children.extend(table_rows);
    let table = build::node(interner, "a", DML_MAIN, "tbl", Vec::new(), table_children);

    let data_attrs = vec![build::attr(interner, "uri", slide::TABLE_GRAPHIC_URI)];
    let graphic_data = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphicData",
        data_attrs,
        vec![RawNode::Element(table)],
    );
    let graphic = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphic",
        Vec::new(),
        vec![RawNode::Element(graphic_data)],
    );

    build::node(
        interner,
        "p",
        PML,
        "graphicFrame",
        Vec::new(),
        vec![
            RawNode::Element(nv_frame_pr),
            RawNode::Element(xfrm),
            RawNode::Element(graphic),
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

/// Whether a placeholder slot is one a slide leaves to its layout to render.
///
/// A date (`dt`), footer (`ftr`) or slide-number (`sldNum`) placeholder is drawn from the layout for
/// every slide that does **not** declare one of its own — that is the mechanism by which one footer
/// reaches a whole deck. Cloning such a slot onto a new slide therefore does not copy the footer, it
/// *suppresses* it and leaves an empty box, so [`add_slide_from_layout`](Presentation::add_slide_from_layout)
/// skips these three, as PowerPoint does.
fn is_layout_rendered_slot(kind: PlaceholderType) -> bool {
    matches!(
        kind,
        PlaceholderType::DateAndTime | PlaceholderType::Footer | PlaceholderType::SlideNumber
    )
}

/// A `p:sp` placeholder shape for a slide built from a layout: the layout's slot (`p:ph`) and name,
/// a fresh id, an **empty** `p:spPr` so position, size and geometry keep inheriting from the layout,
/// and a text body holding one empty run.
///
/// The empty run matters: [`set_shape_text`](Presentation::set_shape_text) replaces the `run_idx`-th
/// run, so a body with no runs could not be filled in at all.
///
/// `p:ph` attributes are written only where they differ from the schema defaults (`type` = `obj`,
/// `idx` = `0`, `sz` = `full`, `orient` = `horz`), which is how Office writes them.
fn build_placeholder(interner: &mut Interner, id: u32, slot: &PlaceholderInfo) -> RawElement {
    let mut ph_attrs = Vec::new();
    if slot.kind != PlaceholderType::Object {
        ph_attrs.push(build::attr(interner, "type", slot.kind.to_wire()));
    }
    if slot.orientation != Orientation::Horizontal {
        ph_attrs.push(build::attr(interner, "orient", slot.orientation.to_wire()));
    }
    if slot.size != PlaceholderSize::Full {
        ph_attrs.push(build::attr(interner, "sz", slot.size.to_wire()));
    }
    if slot.index != 0 {
        ph_attrs.push(build::attr(interner, "idx", &slot.index.to_string()));
    }
    let ph = build::leaf(interner, "p", PML, "ph", ph_attrs);

    let name = slot
        .name
        .clone()
        .unwrap_or_else(|| format!("Placeholder {id}"));
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", &name),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    // Placeholders are not groupable — `a:spLocks@noGrp`, as every Office-written placeholder has.
    let sp_locks_attrs = vec![build::attr(interner, "noGrp", "1")];
    let sp_locks = build::leaf(interner, "a", DML_MAIN, "spLocks", sp_locks_attrs);
    let c_nv_sp_pr = build::node(
        interner,
        "p",
        PML,
        "cNvSpPr",
        Vec::new(),
        vec![RawNode::Element(sp_locks)],
    );
    let nv_pr = build::node(
        interner,
        "p",
        PML,
        "nvPr",
        Vec::new(),
        vec![RawNode::Element(ph)],
    );
    let nv_sp_pr = build::node(
        interner,
        "p",
        PML,
        "nvSpPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_sp_pr),
            RawNode::Element(nv_pr),
        ],
    );

    let sp_pr = build::leaf(interner, "p", PML, "spPr", Vec::new());
    let run = build_run(interner, "");
    let paragraph = build::node(
        interner,
        "a",
        DML_MAIN,
        "p",
        Vec::new(),
        vec![RawNode::Element(run)],
    );
    let tx_body = build_text_body(interner, vec![paragraph]);

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

/// A whole `p:pic` picture: `nvPicPr` (with `a:picLocks@noChangeAspect`) + a `p:blipFill` embedding
/// `rel_id` stretched to the shape + `spPr` with a rectangular geometry at `bounds`.
///
/// `p:blipFill` is a PresentationML element of the DrawingML `CT_BlipFillProperties` type, so it is
/// built here with the `p`-prefixed builders rather than by `mjx_dml::BlipFill::new` (which emits
/// `a:blipFill`); reading it back does reuse `BlipFill`, whose fidelity wrapper is name-agnostic.
/// `rel_declaration` is an `xmlns:r` declaration for the `r:embed` prefix when the slide does not
/// already bind it (see [`build::relationship_prefix_declaration`]).
fn build_picture(
    interner: &mut Interner,
    id: u32,
    rel_id: &str,
    bounds: ShapeBounds,
    rel_declaration: Option<RawAttribute>,
) -> RawElement {
    // p:nvPicPr — cNvPr, cNvPicPr (locking the aspect ratio, as Office writes it), and an empty nvPr.
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", &format!("Picture {id}")),
    ];
    let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
    let lock_attrs = vec![build::attr(interner, "noChangeAspect", "1")];
    let pic_locks = build::leaf(interner, "a", DML_MAIN, "picLocks", lock_attrs);
    let c_nv_pic_pr = build::node(
        interner,
        "p",
        PML,
        "cNvPicPr",
        Vec::new(),
        vec![RawNode::Element(pic_locks)],
    );
    let nv_pr = build::leaf(interner, "p", PML, "nvPr", Vec::new());
    let nv_pic_pr = build::node(
        interner,
        "p",
        PML,
        "nvPicPr",
        Vec::new(),
        vec![
            RawNode::Element(c_nv_pr),
            RawNode::Element(c_nv_pic_pr),
            RawNode::Element(nv_pr),
        ],
    );

    // p:blipFill — the image reference, stretched over the whole shape.
    let rel_prefix = interner.intern(build::RELATIONSHIP_PREFIX);
    let embed = build::attr_prefixed(interner, rel_prefix, "embed", rel_id);
    let blip = build::leaf(interner, "a", DML_MAIN, "blip", vec![embed]);
    let fill_rect = build::leaf(interner, "a", DML_MAIN, "fillRect", Vec::new());
    let stretch = build::node(
        interner,
        "a",
        DML_MAIN,
        "stretch",
        Vec::new(),
        vec![RawNode::Element(fill_rect)],
    );
    let blip_fill = build::node(
        interner,
        "p",
        PML,
        "blipFill",
        Vec::new(),
        vec![RawNode::Element(blip), RawNode::Element(stretch)],
    );

    let sp_pr = build_sp_pr(interner, "rect", bounds);
    let mut picture = build::node(
        interner,
        "p",
        PML,
        "pic",
        Vec::new(),
        vec![
            RawNode::Element(nv_pic_pr),
            RawNode::Element(blip_fill),
            RawNode::Element(sp_pr),
        ],
    );
    if let Some(declaration) = rel_declaration {
        picture.attributes.push(declaration);
    }
    picture
}

/// Builds one `a:p` holding exactly one run (`a:r > a:t`) carrying the line's text — **including when
/// the line is empty**, which yields an empty run rather than an empty paragraph.
///
/// That is what makes a newly added shape fillable: [`set_shape_text`](Presentation::set_shape_text)
/// *replaces* the `run_idx`-th run, so a paragraph with no runs could not be filled in at all (it
/// answered [`RunIndexOutOfRange`](PptxError::RunIndexOutOfRange)). An empty run renders exactly like
/// an empty paragraph, so the blank line a caller asked for still looks blank.
fn build_paragraph(interner: &mut Interner, line: &str) -> RawElement {
    let run = build_run(interner, line);
    build::node(
        interner,
        "a",
        DML_MAIN,
        "p",
        Vec::new(),
        vec![RawNode::Element(run)],
    )
}

/// One `a:r` text run carrying `text` (which may be empty — an empty run is what makes a shape
/// fillable by [`set_shape_text`](Presentation::set_shape_text), which replaces an existing run).
fn build_run(interner: &mut Interner, text: &str) -> RawElement {
    let t = build::text_leaf(interner, "a", DML_MAIN, "t", Vec::new(), text);
    build::node(
        interner,
        "a",
        DML_MAIN,
        "r",
        Vec::new(),
        vec![RawNode::Element(t)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use mjx_dml::{ColorSchemeSlot, SchemeColor};

    fn fixture() -> Vec<u8> {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/sample.pptx");
        std::fs::read(&path).unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()))
    }

    #[test]
    fn color_map_resolves_master_mapping() {
        // The fixture master's p:clrMap is the standard mapping (bg1=lt1, tx1=dk1, …), and slide 0
        // has no p:clrMapOvr — so the effective map is the master's.
        let mut pres = Presentation::open(&fixture()).expect("open");
        let map = pres
            .color_map(0)
            .expect("color_map")
            .expect("fixture has a color map");
        assert_eq!(
            map.resolve(SchemeColor::Background1),
            Some(ColorSchemeSlot::Light1)
        );
        assert_eq!(
            map.resolve(SchemeColor::Text1),
            Some(ColorSchemeSlot::Dark1)
        );
        assert_eq!(
            map.resolve(SchemeColor::Accent1),
            Some(ColorSchemeSlot::Accent1)
        );
    }

    /// Injects a `p:sp` placeholder of `ph_type` with an explicit `solidFill schemeClr {scheme}` into
    /// `part`'s shape tree (the layout/master have empty trees in the fixture).
    fn inject_placeholder_fill(
        pres: &mut Presentation,
        part: &PartName,
        ph_type: &str,
        scheme: &str,
    ) {
        let doc = pres.package.part_tree_mut(part).expect("part tree");
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");

        let ph_attrs = vec![build::attr(interner, "type", ph_type)];
        let ph = build::leaf(interner, "p", PML, "ph", ph_attrs);
        let nv_pr = build::node(
            interner,
            "p",
            PML,
            "nvPr",
            Vec::new(),
            vec![RawNode::Element(ph)],
        );
        let cnvpr_attrs = vec![
            build::attr(interner, "id", "10"),
            build::attr(interner, "name", "Injected"),
        ];
        let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
        let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", Vec::new());
        let nv_sp_pr = build::node(
            interner,
            "p",
            PML,
            "nvSpPr",
            Vec::new(),
            vec![
                RawNode::Element(c_nv_pr),
                RawNode::Element(c_nv_sp_pr),
                RawNode::Element(nv_pr),
            ],
        );

        let clr_attrs = vec![build::attr(interner, "val", scheme)];
        let scheme_clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
        let solid = build::node(
            interner,
            "a",
            DML_MAIN,
            "solidFill",
            Vec::new(),
            vec![RawNode::Element(scheme_clr)],
        );
        let sp_pr = build::node(
            interner,
            "p",
            PML,
            "spPr",
            Vec::new(),
            vec![RawNode::Element(solid)],
        );
        let sp = build::node(
            interner,
            "p",
            PML,
            "sp",
            Vec::new(),
            vec![RawNode::Element(nv_sp_pr), RawNode::Element(sp_pr)],
        );
        sp_tree.children.push(RawNode::Element(sp));
        sp_tree.empty = false;
    }

    #[test]
    fn effective_fill_inherits_from_layout_placeholder() {
        let mut pres = Presentation::open(&fixture()).expect("open");
        let slide0 = pres.slide_part_checked(0).expect("slide").clone();
        let layout = pres
            .follow_rel(&slide0, constants::REL_SLIDE_LAYOUT)
            .expect("rel")
            .expect("layout");

        // The layout's ctrTitle placeholder carries an explicit accent2 fill.
        inject_placeholder_fill(&mut pres, &layout, "ctrTitle", "accent2");

        // Slide 0's ctrTitle placeholder declares no fill of its own, so it inherits the layout's —
        // resolved against the real theme (accent2 = ED7D31).
        assert_eq!(
            pres.effective_shape_fill(0, 0).expect("effective fill"),
            Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("ED7D31".into())))
        );
    }

    /// Injects a `p:sp` placeholder of `ph_type` whose `spPr` holds an `a:ln` with a
    /// `solidFill schemeClr {scheme}` stroke into `part`'s shape tree.
    fn inject_placeholder_outline(
        pres: &mut Presentation,
        part: &PartName,
        ph_type: &str,
        scheme: &str,
    ) {
        let doc = pres.package.part_tree_mut(part).expect("part tree");
        let RawDocument { interner, root, .. } = doc;
        let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");

        let ph_attrs = vec![build::attr(interner, "type", ph_type)];
        let ph = build::leaf(interner, "p", PML, "ph", ph_attrs);
        let nv_pr = build::node(
            interner,
            "p",
            PML,
            "nvPr",
            Vec::new(),
            vec![RawNode::Element(ph)],
        );
        let cnvpr_attrs = vec![
            build::attr(interner, "id", "11"),
            build::attr(interner, "name", "InjectedLine"),
        ];
        let c_nv_pr = build::leaf(interner, "p", PML, "cNvPr", cnvpr_attrs);
        let c_nv_sp_pr = build::leaf(interner, "p", PML, "cNvSpPr", Vec::new());
        let nv_sp_pr = build::node(
            interner,
            "p",
            PML,
            "nvSpPr",
            Vec::new(),
            vec![
                RawNode::Element(c_nv_pr),
                RawNode::Element(c_nv_sp_pr),
                RawNode::Element(nv_pr),
            ],
        );

        let clr_attrs = vec![build::attr(interner, "val", scheme)];
        let scheme_clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
        let solid = build::node(
            interner,
            "a",
            DML_MAIN,
            "solidFill",
            Vec::new(),
            vec![RawNode::Element(scheme_clr)],
        );
        let ln = build::node(
            interner,
            "a",
            DML_MAIN,
            "ln",
            Vec::new(),
            vec![RawNode::Element(solid)],
        );
        let sp_pr = build::node(
            interner,
            "p",
            PML,
            "spPr",
            Vec::new(),
            vec![RawNode::Element(ln)],
        );
        let sp = build::node(
            interner,
            "p",
            PML,
            "sp",
            Vec::new(),
            vec![RawNode::Element(nv_sp_pr), RawNode::Element(sp_pr)],
        );
        sp_tree.children.push(RawNode::Element(sp));
        sp_tree.empty = false;
    }

    #[test]
    fn effective_outline_inherits_from_layout_placeholder() {
        let mut pres = Presentation::open(&fixture()).expect("open");
        let slide0 = pres.slide_part_checked(0).expect("slide").clone();
        let layout = pres
            .follow_rel(&slide0, constants::REL_SLIDE_LAYOUT)
            .expect("rel")
            .expect("layout");

        // The layout's ctrTitle placeholder carries an explicit accent2 outline.
        inject_placeholder_outline(&mut pres, &layout, "ctrTitle", "accent2");

        // Slide 0's ctrTitle declares no outline of its own, so it inherits the layout's — resolved
        // against the real theme (accent2 = ED7D31).
        let effective = pres
            .effective_shape_outline(0, 0)
            .expect("effective outline")
            .expect("inherited outline");
        assert_eq!(
            effective.fill,
            Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("ED7D31".into())))
        );
    }

    #[test]
    fn effective_outline_resolves_a_line_ref_against_the_theme() {
        let mut pres = Presentation::open(&fixture()).expect("open");
        let idx = pres
            .add_shape(
                0,
                PresetShapeType::Rectangle,
                ShapeBounds::from_inches(1.0, 1.0, 2.0, 1.0),
            )
            .expect("add shape");

        // Give the shape a p:style > a:lnRef into the theme's line-style 2 (w=12700), with accent1 as
        // the phClr substitute.
        {
            let part = pres.slide_part_checked(0).expect("slide").clone();
            let doc = pres.package.part_tree_mut(&part).expect("part tree");
            let RawDocument { interner, root, .. } = doc;
            let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");
            let sp = slide::nth_shape_mut(sp_tree, interner, idx).expect("sp");
            let clr_attrs = vec![build::attr(interner, "val", "accent1")];
            let clr = build::leaf(interner, "a", DML_MAIN, "schemeClr", clr_attrs);
            let ln_ref_attrs = vec![build::attr(interner, "idx", "2")];
            let ln_ref = build::node(
                interner,
                "a",
                DML_MAIN,
                "lnRef",
                ln_ref_attrs,
                vec![RawNode::Element(clr)],
            );
            let style = build::node(
                interner,
                "p",
                PML,
                "style",
                Vec::new(),
                vec![RawNode::Element(ln_ref)],
            );
            sp.children.push(RawNode::Element(style));
            sp.empty = false;
        }

        // The effective outline is theme line-style 2 (w=12700) with phClr baked to accent1 (4472C4).
        let effective = pres
            .effective_shape_outline(0, idx)
            .expect("effective outline")
            .expect("line-ref outline");
        assert_eq!(effective.width, Some(mjx_dml::LineWidth::from_emu(12700)));
        assert_eq!(
            effective.fill,
            Some(FillSpec::Solid(mjx_dml::ColorSpec::Srgb("4472C4".into())))
        );
    }

    #[test]
    fn a_shapes_own_list_style_beats_the_layout_and_the_master() {
        // Tier 3 of the text ladder. A shape's `a:lstStyle` has no public setter — it is authored by
        // the designer, not the caller — so it is injected here the way the fill tests inject theirs.
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/layouts.pptx");
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("reading fixture {}: {e}", path.display()));
        let mut pres = Presentation::open(&bytes).expect("open");

        // `<a:lstStyle><a:lvl1pPr algn="r"><a:defRPr sz="1400"/></a:lvl1pPr></a:lstStyle>` on the
        // body placeholder of slide 0, replacing the empty one the fixture ships.
        {
            let part = pres.slide_part_checked(0).expect("slide").clone();
            let doc = pres.package.part_tree_mut(&part).expect("part tree");
            let RawDocument { interner, root, .. } = doc;
            let sp_tree = slide::sp_tree_mut(root, interner).expect("spTree");
            let sp = slide::nth_shape_mut(sp_tree, interner, 1).expect("body placeholder");
            let tx_body = nav::child_mut(sp, interner, PML, "txBody").expect("txBody");

            let def_rpr_attrs = vec![build::attr(interner, "sz", "1400")];
            let def_rpr = build::leaf(interner, "a", DML_MAIN, "defRPr", def_rpr_attrs);
            let lvl1_attrs = vec![build::attr(interner, "algn", "r")];
            let lvl1 = build::node(
                interner,
                "a",
                DML_MAIN,
                "lvl1pPr",
                lvl1_attrs,
                vec![RawNode::Element(def_rpr)],
            );
            let lst_style = build::node(
                interner,
                "a",
                DML_MAIN,
                "lstStyle",
                Vec::new(),
                vec![RawNode::Element(lvl1)],
            );
            let slot = nav::child_mut(tx_body, interner, DML_MAIN, "lstStyle")
                .expect("the fixture ships an empty a:lstStyle");
            *slot = lst_style;
        }

        let paragraph = pres
            .effective_paragraph_properties(0, 1, 0)
            .expect("effective paragraph");
        assert_eq!(paragraph.alignment(), Some(mjx_dml::TextAlignment::Right));
        // The bullet still comes from the master: tier 3 named an alignment, not a bullet.
        assert!(matches!(
            paragraph.bullet(),
            Some(mjx_dml::Bullet::Character(_))
        ));

        let run = pres
            .effective_run_properties(0, 1, 0, 0)
            .expect("effective run");
        assert_eq!(run.size_points(), Some(14.0), "the shape's own size wins");
        assert_eq!(run.is_bold(), Some(true), "still the layout's bold");
    }
}
