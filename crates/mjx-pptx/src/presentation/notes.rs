//! Speaker notes: the notes slide behind a slide, its notes master, and the text of its body
//! placeholder.

use mjx_ooxml_core::{RawDocument, RawNode};
use mjx_ooxml_types::namespaces::{PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_ooxml_types::presentationml::PlaceholderType;
use mjx_opc::{PartName, Relationship, TargetMode};

use mjx_ooxml_types::child_order::PRESENTATION;

use crate::error::PptxError;
use crate::surface::Surface;
use crate::{build, constants, nav, slide};

use super::deck::dir_of;
use super::element_builders::{build_paragraph, build_text_body};
use super::text::replace_txbody;
use super::Presentation;

impl Presentation {
    /// The notes slide part of slide `slide_idx`, or `None` if the slide owns none. Reading does not
    /// touch the package tree.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `slide_idx` is out of range or the relationship points outside the
    /// package.
    pub(super) fn notes_part(&self, slide_idx: usize) -> Result<Option<PartName>, PptxError> {
        let slide_part = self.slide_part_checked(slide_idx)?.clone();
        self.follow_rel(&slide_part, constants::REL_NOTES_SLIDE)
    }

    /// The presentation's notes master part, or `None` if the deck has none.
    ///
    /// # Errors
    /// Returns [`PptxError`] if the relationship points outside the package.
    pub(super) fn notes_master_part(&self) -> Result<Option<PartName>, PptxError> {
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
        let body_idx = self
            .notes_body_index(&notes_part)?
            .ok_or(PptxError::MalformedSlide(
                "notes slide has no body placeholder",
            ))?;

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
        self.package
            .remove_relationship(Some(&slide_part), &rel_id)?;
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

        // Placed by `CT_Presentation`'s generated `xsd:sequence` — after `p:sldMasterIdLst` and
        // before `p:sldIdLst` — rather than by a hand-written order with fallbacks.
        PRESENTATION.insert(&mut root.children, interner, lst);
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
