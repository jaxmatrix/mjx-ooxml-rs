//! Slide lifecycle: adding a slide (empty, from a layout, or with text) and removing one,
//! including the `p:sldIdLst` bookkeeping that goes with it.

use mjx_ooxml_core::{Interner, RawDocument, RawElement, RawNode};
use mjx_ooxml_types::namespaces::{DML_MAIN, PML, SHARED_RELATIONSHIP_REFERENCE};
use mjx_ooxml_types::presentationml::{Orientation, PlaceholderSize, PlaceholderType};
use mjx_opc::{PartName, Relationship, TargetMode};

use crate::error::PptxError;
use crate::geometry::ShapeBounds;
use crate::slide::PlaceholderInfo;
use crate::{build, constants, nav, slide};

use super::deck::dir_of;
use super::element_builders::{build_run, build_text_body};
use super::Presentation;

impl Presentation {
    /// Adds a new empty slide at the end of the deck, wired to the same slide layout as slide 0 — or,
    /// on a deck with no slides yet, to the deck's first layout — and returns its index. The new
    /// slide is a blank shape tree; add content with [`add_text_box`](Self::add_text_box) or use
    /// [`add_slide_with_text`](Self::add_slide_with_text).
    ///
    /// This performs the package edits an added slide requires: it inserts the new slide part (with
    /// its content type), synthesizes the slide's relationships (to the layout), adds the
    /// presentation → slide relationship, and appends a `p:sldId` to `p:sldIdLst`. Every pre-existing
    /// part other than `presentation.xml` stays byte-identical.
    ///
    /// # Errors
    /// Returns [`PptxError::NoSlideLayout`] if the deck offers no layout at all — neither a slide to
    /// inherit one from nor a layout of its own — or another [`PptxError`] if `presentation.xml` is
    /// malformed or a package edit fails.
    pub fn add_slide(&mut self) -> Result<usize, PptxError> {
        // Inherit slide 0's layout: reuse its relationship target verbatim (the new slide shares the
        // same directory, so the relative target resolves identically).
        let Some(first_slide) = self.slides.first().cloned() else {
            // A deck with no slides at all — one from [`blank`](Self::blank) — has nothing to
            // inherit from, so build on the deck's first layout instead. Refusing here would make a
            // blank deck a deck you cannot put a slide on, which is no deck at all.
            let layout = self
                .layouts
                .first()
                .ok_or(PptxError::NoSlideLayout)?
                .clone();
            let new_part = self.next_slide_part()?;
            let layout_target = nav::relative_target(&new_part, &layout);
            return self.insert_slide_part(&layout_target);
        };
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
    pub(super) fn insert_slide_part(&mut self, layout_target: &str) -> Result<usize, PptxError> {
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

    /// A fresh slide part name: `slide{N}.xml` with `N` one past the largest existing slide number,
    /// in the directory the deck's slides already live in.
    ///
    /// A deck with no slides yet — one from [`blank`](Self::blank) — has no slide 0 to take the
    /// directory from, so it falls back to `slides/` beside the presentation part, which is where
    /// every Office-written package puts them. The scan still runs: a package can hold a
    /// `slideN.xml` that no `p:sldId` lists, and reusing that name would collide on insert.
    fn next_slide_part(&self) -> Result<PartName, PptxError> {
        let dir = match self.slides.first() {
            Some(first) => dir_of(first.as_str()).to_owned(),
            None => format!("{}slides/", dir_of(self.presentation_part.as_str())),
        };
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = slide_number(part.as_str(), &dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{dir}slide{}.xml", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
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
