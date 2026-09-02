//! The deck's parts and how they are addressed: slides, masters, layouts, the theme, the
//! colour map, and the relationship plumbing every other module builds on.

use mjx_dml::{ColorMap, FillSpec, Theme, ThemeInfo};
use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, Symbol};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_ooxml_types::presentationml::{SlideLayoutKind, SlideSizeKind};
use mjx_opc::{Package, PartName, Relationship, TargetMode};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::geometry::SlideSize;
use crate::surface::Surface;
use crate::{build, constants, nav, slide};

use super::effective::{resolve_shape_ref, Candidate};
use super::Presentation;

impl Presentation {
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

    pub(super) fn layout_part_checked(&self, idx: usize) -> Result<&PartName, PptxError> {
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
    pub(super) fn surface_part(&self, surface: Surface) -> Result<PartName, PptxError> {
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

    /// A theme part to hang a synthesized notes master on: the presentation's own theme, else the
    /// first slide master's. `None` only in a deck with no theme at all.
    pub(super) fn deck_theme_part(&self) -> Result<Option<PartName>, PptxError> {
        if let Some(theme) = self.follow_rel(&self.presentation_part, constants::REL_THEME)? {
            return Ok(Some(theme));
        }
        match self.masters.first() {
            Some(master) => self.follow_rel(&master.clone(), constants::REL_THEME),
            None => Ok(None),
        }
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
    pub(super) fn inheritance_chain(&self, surface: Surface) -> Result<Vec<PartName>, PptxError> {
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
    pub(super) fn placeholder_candidates(
        &mut self,
        surface: Surface,
        path: &ShapePath,
    ) -> Result<Vec<(PartName, Candidate)>, PptxError> {
        let own_part = self.surface_part(surface)?;
        let placeholder = {
            let doc = self.package.part_tree(&own_part)?;
            let shape = resolve_shape_ref(doc, surface, path)?;
            slide::shape_placeholder(shape, &doc.interner)
        };

        let mut candidates = vec![(own_part, Candidate::Address(path.clone()))];
        if let Some(ph) = placeholder {
            // The rest of the surface's inheritance chain, each searched for the same-slot placeholder.
            for ancestor in self.inheritance_chain(surface)?.into_iter().skip(1) {
                candidates.push((ancestor, Candidate::Placeholder(ph)));
            }
        }
        Ok(candidates)
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
    pub(super) fn theme_part(&self, surface: Surface) -> Result<Option<PartName>, PptxError> {
        let chain = self.inheritance_chain(surface)?;
        let last = chain
            .last()
            .expect("a chain always holds the surface's own part");
        self.follow_rel(last, constants::REL_THEME)
    }

    // -----------------------------------------------------------------------------------------
    // Shared plumbing for the legacy surfaces
    // -----------------------------------------------------------------------------------------

    /// Adds a relationship of `rel_type` from `source` to `target` and returns its new id.
    pub(super) fn relate(
        &mut self,
        source: &PartName,
        target: &PartName,
        rel_type: &str,
    ) -> Result<String, PptxError> {
        let rel_id = self.next_rid_for(source);
        self.package.add_relationship(
            Some(source),
            Relationship {
                id: rel_id.clone(),
                rel_type: rel_type.to_owned(),
                target: nav::relative_target(source, target),
                mode: TargetMode::Internal,
            },
        )?;
        Ok(rel_id)
    }

    /// A fresh part name `{dir}/{stem}{N}.{extension}` beside the presentation part, with `N` one past
    /// the largest `{stem}*` part already in that directory.
    pub(super) fn next_part_in(
        &self,
        dir: &str,
        stem: &str,
        extension: &str,
    ) -> Result<PartName, PptxError> {
        let directory = format!("{}{dir}/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = stem_number(part.as_str(), &directory, stem) {
                max_n = max_n.max(n);
            }
        }
        PartName::new(&format!("{directory}{stem}{}.{extension}", max_n + 1))
            .map_err(PptxError::from)
    }

    /// The part relationship `rel_id` of `source` points at, or `None` if `source` has no such
    /// relationship. Errors if it points outside the package. Used to resolve an image blip's
    /// `r:embed` and a chart frame's `c:chart@r:id` alike — both name a part by id.
    pub(super) fn part_for_rel(
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

    /// The next free presentation-scoped relationship id (`rId{N}`), one past the current maximum.
    pub(super) fn next_presentation_rid(&self) -> Result<String, PptxError> {
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
    pub(super) fn next_rid_for(&self, part: &PartName) -> String {
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

    pub(super) fn slide_part_checked(&self, slide_idx: usize) -> Result<&PartName, PptxError> {
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
    pub(super) fn follow_rel(
        &self,
        part: &PartName,
        rel_type: &str,
    ) -> Result<Option<PartName>, PptxError> {
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

/// The prefix a part's root binds to the relationships namespace, or the conventional `r` when it
/// binds none — what an `r:id` / `r:embed` attribute must be written with.
pub(super) fn relationship_prefix(part_root: &RawElement, interner: &mut Interner) -> Symbol {
    nav::namespace_prefix(part_root, interner, SHARED_RELATIONSHIP_REFERENCE)
        .unwrap_or_else(|| interner.intern(build::RELATIONSHIP_PREFIX))
}

/// The `xmlns:r` declaration a fill element needs, or `None` when it needs none: only a picture fill
/// carries an `r:embed`, and only a part that does not already bind the prefix needs the declaration.
///
/// It is computed from the part **root**, so it must be taken before the borrow descends into the
/// shape tree.
pub(super) fn fill_relationship_declaration(
    fill: &FillSpec,
    part_root: &RawElement,
    interner: &mut Interner,
) -> Option<RawAttribute> {
    match fill {
        FillSpec::Blip { .. } => build::relationship_prefix_declaration(part_root, interner),
        _ => None,
    }
}

/// The directory portion of an absolute part name, including the trailing `/` (e.g.
/// `/ppt/slides/slide1.xml` → `/ppt/slides/`).
pub(super) fn dir_of(part: &str) -> &str {
    let end = part.rfind('/').map_or(0, |idx| idx + 1);
    &part[..end]
}

/// The parts referenced by one of PresentationML's `r:id` lists — `p:sldIdLst > p:sldId`,
/// `p:sldMasterIdLst > p:sldMasterId`, `p:sldLayoutIdLst > p:sldLayoutId` — in document order.
///
/// Each item names a relationship of `source`; the ids are collected first so the tree borrow ends
/// before the relationships are consulted. An absent list yields no parts (a master need not list
/// layouts); a *present* item with no `r:id`, or an id no relationship matches, is an error, since
/// that is a broken reference rather than an absence.
pub(super) fn referenced_parts(
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

/// The `N` in `{dir}{stem}{N}.{ext}`, whatever the extension, or `None` if `part` does not match.
pub(super) fn stem_number(part: &str, dir: &str, stem: &str) -> Option<u32> {
    let rest = part.strip_prefix(dir)?.strip_prefix(stem)?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() || !rest[digits.len()..].starts_with('.') {
        return None;
    }
    digits.parse::<u32>().ok()
}
