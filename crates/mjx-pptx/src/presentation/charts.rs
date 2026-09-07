//! Charts: adding one, its embedded workbook, and the series, axes, title and legend it draws.

use mjx_chart::chart_ops;
use mjx_chart::{
    apply_workbook_patch, embedded_workbook_for_chart_data, embedded_workbook_for_chart_space,
    embedded_workbook_part, plan_workbook_patch, AxisOrientation, ChartAxisData, ChartData,
    ChartDataError, ChartKind, ChartLegendData, ChartSeriesData, ChartSeriesReferences, ChartSpace,
    LegendPosition, WorkbookPatch,
};
use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawDocument, RawElement, RawNode, ToXml};
use mjx_ooxml_types::namespaces::{DML_CHART, DML_MAIN, PML};
use mjx_opc::{PartName, Relationship, TargetMode};

use crate::address::ShapePath;
use crate::error::PptxError;
use crate::external::ChartWorkbook;
use crate::geometry::ShapeBounds;
use crate::surface::Surface;
use crate::{build, constants, nav, slide};

use super::deck::{dir_of, stem_number};
use super::effective::resolve_shape_ref;
use super::Presentation;

/// The part a prepared workbook patch is bound for, and the bytes to write there.
///
/// `None` for the whole thing is *there is no embedded workbook*; `Some((part, None))` is *there is
/// one and every cell already said the right thing*, which must leave the part exactly as it is
/// because re-saving a package rewrites its container even when nothing inside it changed.
type PreparedWorkbook = Option<(PartName, Option<Vec<u8>>)>;

impl Presentation {
    /// Adds `chart` to `surface` as a new chart, laid out inside `bounds`, and returns its index in
    /// the shape tree.
    ///
    /// Three parts are written, and the slide gains one shape:
    ///
    /// * `ppt/charts/chartN.xml` — the chart, holding the series' `c:strCache`/`c:numCache` values
    ///   (what renders) and a `c:externalData` naming its workbook;
    /// * `ppt/embeddings/Microsoft_Excel_SheetN.xlsx` — the **embedded workbook**, laid out to match
    ///   the chart's `c:f` formulas cell for cell, which is what PowerPoint's *Edit Data* opens;
    /// * `ppt/charts/_rels/chartN.xml.rels` — the relationship binding the two.
    ///
    /// The chart is a shape: move it with [`set_shape_bounds`](Self::set_shape_bounds), drop it with
    /// [`remove_shape`](Self::remove_shape), and read it back with [`chart_series`](Self::chart_series).
    ///
    /// # Errors
    /// Returns [`PptxError::InvalidChartData`] if `chart` has nothing to draw (no series, or every
    /// series empty), [`PptxError::ChartData`] if the plot type constrains its series count and
    /// `chart` does not satisfy it, or another [`PptxError`] if the surface index is out of range or
    /// a package edit fails.
    pub fn add_chart(
        &mut self,
        surface: impl Into<Surface>,
        chart: &ChartData,
        bounds: ShapeBounds,
    ) -> Result<usize, PptxError> {
        match chart.validate() {
            Ok(()) => {}
            // "Nothing to draw" has had its own variant since charts could be authored at all.
            Err(ChartDataError::NoData) => return Err(PptxError::InvalidChartData),
            Err(problem) => return Err(PptxError::ChartData(problem)),
        }
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;

        // Everything fallible that does not touch the package happens first, so a failure here
        // leaves the document exactly as it was.
        let workbook = embedded_workbook_for_chart_data(chart)?;
        let chart_part = self.next_chart_part()?;
        let workbook_part = self.next_chart_workbook_part()?;

        // The chart part names its workbook by a relationship of its own. The chart part is brand
        // new, so its relationship space is empty and `rId1` is free.
        let workbook_rel_id = "rId1";
        self.package.insert_part(
            &chart_part,
            constants::CONTENT_TYPE_CHART,
            chart.to_part_bytes_linking_workbook(workbook_rel_id),
        )?;
        self.package.insert_part(
            &workbook_part,
            mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE,
            workbook,
        )?;
        self.package.add_relationship(
            Some(&chart_part),
            Relationship {
                id: workbook_rel_id.to_owned(),
                rel_type: constants::REL_PACKAGE.to_owned(),
                target: nav::relative_target(&chart_part, &workbook_part),
                mode: TargetMode::Internal,
            },
        )?;
        let rel_id = self.next_rid_for(&slide_part);
        self.package.add_relationship(
            Some(&slide_part),
            Relationship {
                id: rel_id.clone(),
                rel_type: constants::REL_CHART.to_owned(),
                target: nav::relative_target(&slide_part, &chart_part),
                mode: TargetMode::Internal,
            },
        )?;

        let doc = self.package.part_tree_mut(&slide_part)?;
        let RawDocument { interner, root, .. } = doc;
        let rel_declaration = build::relationship_prefix_declaration(root, interner);
        let sp_tree = slide::sp_tree_mut(root, interner)?;

        let next_id = slide::max_cnvpr_id(sp_tree, interner).max(1) + 1;
        let frame = build_chart_frame(interner, next_id, &rel_id, bounds, rel_declaration);
        sp_tree.children.push(RawNode::Element(frame));
        sp_tree.empty = false;

        Ok(slide::shapes(sp_tree, interner).count() - 1)
    }

    /// The relationship id the chart frame `shape_idx` on `surface` names
    /// (`p:graphicFrame > a:graphic > a:graphicData > c:chart@r:id`), or `None` when the shape is not
    /// a graphic frame holding a chart. Reading does not dirty the part.
    ///
    /// The chart itself lives in a separate part (`/ppt/charts/chartN.xml`); this returns the id of
    /// the slide relationship that names it. [`chart_part_bytes`](Self::chart_part_bytes) resolves
    /// that relationship to the chart's bytes.
    ///
    /// # Errors
    /// Returns [`PptxError`] if an index is out of range or the slide is malformed.
    pub fn chart_rel_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        let surface = surface.into();
        let slide_part = self.surface_part(surface)?;
        let doc = self.package.part_tree(&slide_part)?;
        let shape = resolve_shape_ref(doc, surface, &shape_idx.into())?;
        Ok(slide::chart_rel_id(shape, &doc.interner).map(str::to_owned))
    }

    /// The raw XML bytes of the chart part the chart frame `shape_idx` on `surface` references
    /// (`/ppt/charts/chartN.xml`), exactly as the package holds them, or `None` when the shape frames
    /// no chart. Borrowed from the package, so the part is not copied.
    ///
    /// The chart part is **not modeled** yet — it and its satellites (an embedded workbook, colour and
    /// style parts) are carried through a round-trip verbatim. This is the read window onto a chart
    /// until [`mjx-chart`] models it; reading does not dirty anything.
    ///
    /// # Errors
    /// As [`chart_rel_id`](Self::chart_rel_id), plus [`PptxError::ExternalTarget`] if the relationship
    /// points outside the package.
    ///
    /// [`mjx-chart`]: https://docs.rs/mjx-chart
    pub fn chart_part_bytes(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<&[u8]>, PptxError> {
        let surface = surface.into();
        let Some(rel_id) = self.chart_rel_id(surface, shape_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        let Some(part) = self.part_for_rel(&slide_part, &rel_id)? else {
            return Ok(None);
        };
        Ok(self.package.part_bytes(&part))
    }

    /// Every chart on `surface` that references a backing workbook (`c:externalData`), with where each
    /// is referenced from and whether that reference is external.
    ///
    /// An external workbook is the source that can be unreachable on another platform; a chart draws
    /// from its cached data regardless, so [`detach_chart_workbook`](Self::detach_chart_workbook) can
    /// safely remove the reference. This saves the caller from walking the shapes. Reading does not
    /// dirty any part.
    ///
    /// # Errors
    /// Returns [`PptxError`] if `surface` cannot be resolved or a slide is malformed.
    pub fn chart_workbooks(
        &mut self,
        surface: impl Into<Surface>,
    ) -> Result<Vec<ChartWorkbook>, PptxError> {
        let surface = surface.into();
        let count = self.shape_count(surface)?;
        let mut workbooks = Vec::new();
        for shape_index in 0..count {
            let Some(chart_part) = self.chart_part(surface, shape_index)? else {
                continue; // not a chart frame
            };
            let Some(rel_id) = self.chart_external_data_rel_id(&chart_part)? else {
                continue; // a chart with no backing workbook
            };
            let Some(rel) = self
                .package
                .relationships_for(Some(&chart_part))
                .and_then(|rels| rels.by_id(&rel_id))
            else {
                continue; // the reference names no relationship — nothing to report
            };
            workbooks.push(ChartWorkbook {
                shape_index,
                target: rel.target.clone(),
                external: rel.mode == TargetMode::External,
            });
        }
        Ok(workbooks)
    }

    /// Detaches the backing workbook from the chart `shape_idx` on `surface`: removes its
    /// `c:externalData` reference — the element and its relationship — leaving the chart to render from
    /// its cached values. This neutralizes a chart that links an unreachable external workbook (the
    /// caller decides accessibility; use [`chart_workbooks`](Self::chart_workbooks) to find the
    /// candidates), and yields exactly the cache-only shape a freshly authored chart has.
    ///
    /// If the reference was to an *embedded* workbook, that part is left unreferenced; sweep it with
    /// [`Package::remove_unreferenced_parts`](mjx_opc::Package::remove_unreferenced_parts) if wanted.
    /// This never removes parts on its own. Dirties only the chart part.
    ///
    ///
    /// # Its role since MJXOFF-208
    ///
    /// It used to be the escape hatch from a data edit that would otherwise **rebuild** the embedded
    /// workbook and throw its contents away. It is not that any more: a data edit now *patches* the
    /// cells the chart's own `c:f` names and leaves the rest of that workbook alone, so detaching to
    /// protect its contents is no longer something a caller has to think of. What is left is what
    /// the name says — cut the chart loose from a workbook it should not be carrying at all.
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape is not a chart frame,
    /// [`PptxError::ChartHasNoExternalData`] if the chart references no workbook, or another
    /// [`PptxError`] if an index is out of range or the chart is malformed.
    pub fn detach_chart_workbook(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let chart_part = self
            .chart_part(surface, shape_idx)?
            .ok_or(PptxError::ShapeIsNotAChart)?;
        // The workbook reference must exist; capture the relationship id it names (if any) first.
        let rel_id = {
            let doc = self.package.part_tree(&chart_part)?;
            let external_data = nav::child(&doc.root, &doc.interner, DML_CHART, "externalData")
                .ok_or(PptxError::ChartHasNoExternalData)?;
            external_data
                .attributes
                .iter()
                .find(|attr| doc.interner.resolve(attr.name.local) == "id")
                .and_then(|attr| std::str::from_utf8(&attr.value).ok())
                .map(str::to_owned)
        };
        // Drop the `c:externalData` element from the chart tree.
        {
            let doc = self.package.part_tree_mut(&chart_part)?;
            let RawDocument { interner, root, .. } = doc;
            root.children.retain(|node| {
                !matches!(node, RawNode::Element(el)
                    if nav::name_is(&el.name, interner, DML_CHART, "externalData"))
            });
        }
        // Drop its relationship, if the element named one.
        if let Some(rel_id) = rel_id {
            self.package
                .remove_relationship(Some(&chart_part), &rel_id)?;
        }
        Ok(())
    }

    /// The relationship id a chart part's `c:externalData` names (its backing workbook), or `None` when
    /// the chart has no `c:externalData` or it carries no `r:id`. Reads only.
    fn chart_external_data_rel_id(
        &mut self,
        chart_part: &PartName,
    ) -> Result<Option<String>, PptxError> {
        let doc = self.package.part_tree(chart_part)?;
        let Some(external_data) = nav::child(&doc.root, &doc.interner, DML_CHART, "externalData")
        else {
            return Ok(None);
        };
        Ok(external_data
            .attributes
            .iter()
            .find(|attr| doc.interner.resolve(attr.name.local) == "id")
            .and_then(|attr| std::str::from_utf8(&attr.value).ok())
            .map(str::to_owned))
    }

    /// The part name of the chart the frame `shape_idx` on `surface` references, or `None` when the
    /// shape frames no chart.
    fn chart_part(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<PartName>, PptxError> {
        let Some(rel_id) = self.chart_rel_id(surface, shape_idx)? else {
            return Ok(None);
        };
        let slide_part = self.surface_part(surface)?;
        self.part_for_rel(&slide_part, &rel_id)
    }

    /// Reads the chart the frame `shape_idx` on `surface` references as a typed [`ChartSpace`] and
    /// hands it, with the part's interner, to `read`. Does **not** dirty the part.
    pub(super) fn with_chart<R>(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        read: impl FnOnce(&ChartSpace, &Interner) -> Result<R, PptxError>,
    ) -> Result<R, PptxError> {
        let part = self
            .chart_part(surface, shape_idx)?
            .ok_or(PptxError::ShapeIsNotAChart)?;
        let doc = self.package.part_tree(&part)?;
        let space = ChartSpace::from_xml(&doc.root, &doc.interner)?;
        read(&space, &doc.interner)
    }

    /// Parses the chart part the frame `shape_idx` on `surface` references, hands the whole
    /// [`ChartSpace`] to `edit`, and writes the mutated tree back — dirtying **only** the chart part.
    ///
    /// The chart part's root *is* the `c:chartSpace`, so the edit replaces the whole part tree from
    /// the model; the untouched parts of the chart (axes, styling, other series) survive because the
    /// model round-trips them verbatim.
    pub(super) fn edit_chart(
        &mut self,
        surface: Surface,
        shape_idx: impl Into<ShapePath>,
        edit: impl FnOnce(&mut ChartSpace, &mut Interner) -> Result<(), PptxError>,
    ) -> Result<(), PptxError> {
        let part = self
            .chart_part(surface, shape_idx)?
            .ok_or(PptxError::ShapeIsNotAChart)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut space = ChartSpace::from_xml(root, interner)?;
        edit(&mut space, interner)?;
        space.write_back(root, interner);
        Ok(())
    }

    /// The series of the chart the frame `shape_idx` on `surface` references — for each, its name,
    /// category labels and values (for a scatter series, its X labels and Y values), flattened across
    /// the chart's plots. Reading does not dirty the part.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart, or another [`PptxError`] if an
    /// index is out of range or the chart part is malformed.
    pub fn chart_series(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Vec<ChartSeriesData>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, _interner| {
            Ok(chart_ops::series(space))
        })
    }

    /// Rewrites the values of series `series_idx` (0-based across the chart's plots) of the chart the
    /// frame `shape_idx` on `surface` references — whichever source the series names: a `c:numRef`'s
    /// cache or a `c:numLit`.
    ///
    /// The chart's **embedded workbook is patched in the same call**, so the numbers PowerPoint's
    /// *Edit Data* shows are the numbers the chart draws. The new values go into the cells the
    /// series' own `c:f` names and nowhere else — every other sheet, cell, format and defined name
    /// that workbook carried comes back exactly as it was. See
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) for the whole story, including the
    /// references this library refuses to write rather than guess at.
    ///
    /// Marks the chart part dirty, and the workbook part when there is one to patch; a non-finite
    /// value is skipped.
    ///
    /// # All of it, or none of it
    ///
    /// The workbook is worked out **before** the chart is touched and written **after**, so a
    /// reference this library will not write refuses the whole call and leaves both parts as they
    /// were. There is no state in which the chart says one thing and its workbook another because
    /// half the call succeeded.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart,
    /// [`PptxError::ChartSeriesOutOfRange`] if `series_idx` is past the last series,
    /// [`PptxError::ChartSeriesNotEditable`] if the series has no numeric values to rewrite, or
    /// [`PptxError::ChartEmbeddedWorkbookNotWritable`] if the series' `c:f` names cells this library
    /// will not write.
    pub fn set_chart_series_values(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        values: &[f64],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let shape_idx = shape_idx.into();
        let prepared = self.prepare_chart_workbook(
            surface,
            shape_idx.clone(),
            WorkbookPatch::SeriesValues { series_idx, values },
        )?;
        self.edit_chart(surface, shape_idx, |space, interner| {
            Ok(chart_ops::set_series_values(
                space, interner, series_idx, values,
            )?)
        })?;
        self.commit_chart_workbook(prepared)?;
        Ok(())
    }

    /// Rewrites the category labels of series `series_idx` (0-based across the chart's plots) of the
    /// chart the frame `shape_idx` on `surface` references, and patches the cells the series' own
    /// category `c:f` names alongside it.
    ///
    /// All-or-nothing in the same way [`set_chart_series_values`](Self::set_chart_series_values)
    /// is, and preserving in the same way.
    ///
    /// # Errors
    /// As [`set_chart_series_values`](Self::set_chart_series_values), with
    /// [`PptxError::ChartSeriesNotEditable`] when the series' category source is numeric or
    /// multi-level and so has no string labels to rewrite.
    pub fn set_chart_series_categories(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        series_idx: usize,
        labels: &[&str],
    ) -> Result<(), PptxError> {
        let surface = surface.into();
        let shape_idx = shape_idx.into();
        let prepared = self.prepare_chart_workbook(
            surface,
            shape_idx.clone(),
            WorkbookPatch::SeriesCategories { series_idx, labels },
        )?;
        self.edit_chart(surface, shape_idx, |space, interner| {
            Ok(chart_ops::set_series_categories(
                space, interner, series_idx, labels,
            )?)
        })?;
        self.commit_chart_workbook(prepared)?;
        Ok(())
    }

    /// Writes the chart's data into the workbook the chart the frame `shape_idx` on `surface`
    /// references already embeds, and answers whether it wrote one.
    ///
    /// Answers `Ok(false)`, changing nothing, when there is nothing to write: the chart names no
    /// workbook (`c:externalData` absent — an authored chart before this tier, or one
    /// [`detach_chart_workbook`](Self::detach_chart_workbook) has been used on), the workbook it
    /// names is an **external** link rather than a part of this package, which belongs to whoever
    /// hosts it, or every one of its series carries its data as a literal and so names no cells at
    /// all.
    ///
    /// # The workbook is patched, not replaced (MJXOFF-208)
    ///
    /// Each series' `c:f` says which cells its data lives in; this writes the chart's caches into
    /// **those** cells and touches nothing else. Every other sheet, every cell format, every defined
    /// name, every macro and every document property the workbook carried survives byte for byte,
    /// because the parts holding them are never rewritten.
    ///
    /// Until MJXOFF-208 this method regenerated the workbook instead — a fresh one-sheet package
    /// over the part the producer wrote — and a data edit called it for you, so opening a real file
    /// and changing one number silently discarded everything else that workbook held. That branch
    /// still exists, under the name that says what it does
    /// ([`regenerate_chart_workbook`](Self::regenerate_chart_workbook)), and a caller now has to ask
    /// for it.
    ///
    ///
    /// # What the answer means
    ///
    /// `true` says the chart **has** an embedded workbook this call is responsible for, not that
    /// bytes changed. A cell that already holds the value it was going to be given is not rewritten,
    /// so a call that finds everything in agreement answers `true` and leaves the part byte-identical
    /// — which is the only way a no-op refresh stays a no-op, because re-saving a package rewrites
    /// its container even when every part inside it is the same.
    /// # A reference this library will not write is refused, never guessed at
    ///
    /// A `c:f` that names another workbook, several sheets, whole columns, a rectangle, or fewer
    /// cells than the data has points is a
    /// [`PptxError::ChartEmbeddedWorkbookNotWritable`] — not a quiet fall back to regenerating,
    /// which would destroy the very content this method exists to keep.
    ///
    /// [`set_chart_series_values`](Self::set_chart_series_values) and
    /// [`set_chart_series_categories`](Self::set_chart_series_categories) patch the one reference
    /// they edit; this one covers every reference the chart names, which is what a caller that has
    /// edited a chart another way wants.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart,
    /// [`PptxError::ChartEmbeddedWorkbookNotWritable`] if a reference names cells this library will
    /// not write, or another [`PptxError`] if the chart part or the embedded package is malformed.
    pub fn refresh_chart_workbook(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<bool, PptxError> {
        let prepared = self.prepare_chart_workbook(
            surface.into(),
            shape_idx.into(),
            WorkbookPatch::EveryReference,
        )?;
        self.commit_chart_workbook(prepared)
    }

    /// Replaces the embedded workbook of the chart the frame `shape_idx` on `surface` references
    /// with a freshly built one, and answers whether it replaced one.
    ///
    /// **This discards whatever that workbook held.** One sheet, column `A` the categories, column
    /// `B` onwards one per series — the layout an authored chart's `c:f` formulas name. Any extra
    /// sheet, cell format, defined name, macro or document property the old workbook carried is
    /// gone, and so is any agreement between the new cells and a producer's own `c:f`, which may
    /// name a different sheet or a different corner of it entirely.
    ///
    /// So this is the explicit opt-in, and [`refresh_chart_workbook`](Self::refresh_chart_workbook)
    /// is what a caller wants: it writes the same data into the cells the chart already names and
    /// leaves the rest of the workbook alone. Reach for this one only when a fresh workbook is
    /// genuinely the ask — a chart this library authored, or one whose workbook is beyond repair.
    /// [`detach_chart_workbook`](Self::detach_chart_workbook) is the other honest answer: drop the
    /// reference rather than write over the file.
    ///
    /// Answers `Ok(false)`, changing nothing, in exactly the cases
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) does.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart, or another [`PptxError`] if an
    /// index is out of range, the chart part is malformed, or the package edit fails.
    pub fn regenerate_chart_workbook(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<bool, PptxError> {
        let surface = surface.into();
        let shape_idx = shape_idx.into();
        let chart_part = self
            .chart_part(surface, shape_idx)?
            .ok_or(PptxError::ShapeIsNotAChart)?;

        let doc = self.package.part_tree(&chart_part)?;
        let space = ChartSpace::from_xml(&doc.root, &doc.interner)?;
        let Some(rel_id) = space.external_data_rel_id(&doc.interner).map(str::to_owned) else {
            return Ok(false);
        };
        let workbook = embedded_workbook_for_chart_space(&space)?;

        // An external workbook is not ours to rewrite; a relationship we cannot resolve is not
        // either. Neither is an error — there is simply no embedded workbook here.
        let Some(workbook_part) = embedded_workbook_part(&self.package, &chart_part, &rel_id)
        else {
            return Ok(false);
        };
        if self.package.part_bytes(&workbook_part).is_none() {
            return Ok(false);
        }
        self.package.replace_part_bytes(&workbook_part, workbook)?;
        Ok(true)
    }

    /// Works out what the addressed chart's embedded workbook should become, **without writing it**.
    ///
    /// The half of a data edit that can refuse, kept separate from the half that changes the
    /// package. Answers `None` when there is no workbook to write, or nothing in it to write.
    fn prepare_chart_workbook(
        &mut self,
        surface: Surface,
        shape_idx: ShapePath,
        patch: WorkbookPatch<'_>,
    ) -> Result<PreparedWorkbook, PptxError> {
        let chart_part = self
            .chart_part(surface, shape_idx)?
            .ok_or(PptxError::ShapeIsNotAChart)?;
        let doc = self.package.part_tree(&chart_part)?;
        let space = ChartSpace::from_xml(&doc.root, &doc.interner)?;
        let Some(rel_id) = space.external_data_rel_id(&doc.interner).map(str::to_owned) else {
            return Ok(None);
        };
        let plan = plan_workbook_patch(&space, &doc.interner, patch)?;
        let Some(workbook_part) = embedded_workbook_part(&self.package, &chart_part, &rel_id)
        else {
            return Ok(None);
        };
        let Some(bytes) = self.package.part_bytes(&workbook_part) else {
            return Ok(None);
        };
        Ok(Some((workbook_part, apply_workbook_patch(&plan, bytes)?)))
    }

    /// Writes what [`prepare_chart_workbook`](Self::prepare_chart_workbook) worked out, and answers
    /// whether there was anything to write.
    fn commit_chart_workbook(&mut self, prepared: PreparedWorkbook) -> Result<bool, PptxError> {
        let Some((part, bytes)) = prepared else {
            return Ok(false);
        };
        // `Some((part, None))` is *there is a workbook and it already says the right thing*, which
        // is a different answer from *there is no workbook* and is reported as such. Writing it
        // anyway would re-zip the part for no change at all.
        if let Some(bytes) = bytes {
            self.package.replace_part_bytes(&part, bytes)?;
        }
        Ok(true)
    }

    /// The kind of every plot the chart the frame `shape_idx` on `surface` references draws, in
    /// document order — one entry per plot element, so a combo chart yields several. Reading does not
    /// dirty the part.
    ///
    /// All sixteen plot types `CT_PlotArea` admits are recognised, including the ones a chart frame
    /// used to read as nothing: radar, bubble, stock, pie-of-pie, the surfaces and the
    /// three-dimensional forms.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart, or another [`PptxError`] if an
    /// index is out of range or the chart part is malformed.
    pub fn chart_kinds(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Vec<ChartKind>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, _interner| {
            Ok(chart_ops::kinds(space))
        })
    }

    /// Where every series of the chart says its data lives — the `c:f` beside each cache, as the
    /// file wrote it. Reading does not dirty the part.
    ///
    /// The companion of [`chart_series`](Self::chart_series): that answers what the **caches** hold,
    /// this answers what the references **name**. For a chart this library authored, those
    /// references name the embedded workbook beside it; for one a producer wrote they may name
    /// anything, and the text comes back as written. A field is `None` where the source is a
    /// literal and so has no cells behind it at all.
    ///
    /// # Errors
    /// As [`chart_kinds`](Self::chart_kinds).
    pub fn chart_series_references(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Vec<ChartSeriesReferences>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, _interner| {
            Ok(chart_ops::series_references(space))
        })
    }

    /// The axes of the chart the frame `shape_idx` on `surface` references, in document order.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart, or another [`PptxError`] if an
    /// index is out of range or the chart part is malformed.
    pub fn chart_axes(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Vec<ChartAxisData>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::axes(space, interner))
        })
    }

    /// Sets or clears the explicit bounds of axis `axis_idx` (0-based, document order) of the chart
    /// the frame `shape_idx` on `surface` references. `None` returns that end of the axis to
    /// automatic scaling. Marks only the chart part dirty.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart,
    /// [`PptxError::ChartAxisOutOfRange`] if `axis_idx` is past the last axis, or another
    /// [`PptxError`] if an index is out of range or the chart part is malformed.
    pub fn set_chart_axis_scale(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        axis_idx: usize,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_axis_scale(
                space, interner, axis_idx, minimum, maximum,
            )?)
        })
    }

    /// Sets the direction of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references — smallest value first, or reversed. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_axis_scale`](Self::set_chart_axis_scale).
    pub fn set_chart_axis_orientation(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        axis_idx: usize,
        orientation: AxisOrientation,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_axis_orientation(
                space,
                interner,
                axis_idx,
                orientation,
            )?)
        })
    }

    /// Sets or removes the title of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references. `None` removes the title. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_axis_scale`](Self::set_chart_axis_scale).
    pub fn set_chart_axis_title(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        axis_idx: usize,
        text: Option<&str>,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_axis_title(space, interner, axis_idx, text)?)
        })
    }

    /// Turns the gridlines of axis `axis_idx` of the chart the frame `shape_idx` on `surface`
    /// references on or off. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_axis_scale`](Self::set_chart_axis_scale).
    pub fn set_chart_axis_gridlines(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        axis_idx: usize,
        major: bool,
        minor: bool,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_axis_gridlines(
                space, interner, axis_idx, major, minor,
            )?)
        })
    }

    /// The heading of the chart the frame `shape_idx` on `surface` references (`c:title`), or `None`
    /// when it has none. Reading does not dirty the part.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart, or another [`PptxError`] if an
    /// index is out of range or the chart part is malformed.
    pub fn chart_title(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<String>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, _interner| {
            Ok(chart_ops::title(space))
        })
    }

    /// Sets or removes the heading of the chart the frame `shape_idx` on `surface` references.
    /// `None` removes it. Marks only the chart part dirty.
    ///
    /// Setting a title also clears `c:autoTitleDeleted`, and removing one sets it — otherwise Office
    /// either refuses to draw the title given to it or invents one of its own.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart,
    /// [`PptxError::ChartHasNoChartElement`] if the part declares no `c:chart`, or another
    /// [`PptxError`] if an index is out of range or the chart part is malformed.
    pub fn set_chart_title(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        text: Option<&str>,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_title(space, interner, text)?)
        })
    }

    /// The legend of the chart the frame `shape_idx` on `surface` references, or `None` when it has
    /// none. Reading does not dirty the part.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart, or another [`PptxError`] if an
    /// index is out of range or the chart part is malformed.
    pub fn chart_legend(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<ChartLegendData>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::legend(space, interner))
        })
    }

    /// Places the legend of the chart the frame `shape_idx` on `surface` references at `position`,
    /// adding one if the chart had none. `None` removes the legend. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_title`](Self::set_chart_title).
    pub fn set_chart_legend(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
        position: Option<LegendPosition>,
    ) -> Result<(), PptxError> {
        self.edit_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::set_legend(space, interner, position)?)
        })
    }

    /// The built-in style id the chart the frame `shape_idx` on `surface` references names
    /// (`c:style@val`, 1 to 48) — the palette and effect set Office draws an unstyled series with —
    /// or `None` when it names none. Reading does not dirty the part.
    ///
    /// # Errors
    /// [`PptxError::ShapeIsNotAChart`] if the shape frames no chart, or another [`PptxError`] if an
    /// index is out of range or the chart part is malformed.
    pub fn chart_style_id(
        &mut self,
        surface: impl Into<Surface>,
        shape_idx: impl Into<ShapePath>,
    ) -> Result<Option<u32>, PptxError> {
        self.with_chart(surface.into(), shape_idx, |space, interner| {
            Ok(chart_ops::style_id(space, interner))
        })
    }

    /// A fresh chart part name in the presentation's `charts/` directory: `chart{N}.xml` with `N` one
    /// past the largest existing chart number.
    fn next_chart_part(&self) -> Result<PartName, PptxError> {
        let charts_dir = format!("{}charts/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = chart_number(part.as_str(), &charts_dir) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{charts_dir}chart{}.xml", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }

    /// A fresh chart-workbook part name: `embeddings/Microsoft_Excel_Sheet{N}.xlsx` beside the
    /// presentation part, with `N` one past the largest existing one.
    ///
    /// The stem is the name Office itself uses for a chart's embedded workbook, so a deck this
    /// library authors and one PowerPoint authors are named alike; the `oleObject{N}` stem
    /// [`next_embedding_part`](Self::next_embedding_part) uses is a different series and the two
    /// never collide.
    fn next_chart_workbook_part(&self) -> Result<PartName, PptxError> {
        let embeddings_dir = format!("{}embeddings/", dir_of(self.presentation_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = stem_number(part.as_str(), &embeddings_dir, CHART_WORKBOOK_STEM) {
                max_n = max_n.max(n);
            }
        }
        let name = format!("{embeddings_dir}{CHART_WORKBOOK_STEM}{}.xlsx", max_n + 1);
        PartName::new(&name).map_err(PptxError::from)
    }
}

/// The part-name stem of a chart's embedded workbook, matching what Office writes
/// (`/ppt/embeddings/Microsoft_Excel_Sheet1.xlsx`).
const CHART_WORKBOOK_STEM: &str = "Microsoft_Excel_Sheet";

/// Extracts `N` from a `chart{N}.xml` part directly inside `dir` (e.g. `/ppt/charts/chart2.xml` with
/// `dir = /ppt/charts/` → `2`). Returns `None` for anything else (e.g. the `_rels` subfolder).
fn chart_number(part: &str, dir: &str) -> Option<u32> {
    part.strip_prefix(dir)?
        .strip_prefix("chart")?
        .strip_suffix(".xml")?
        .parse::<u32>()
        .ok()
}

/// A `p:graphicFrame` that frames a chart: the same non-visual + transform scaffolding as
/// [`build_table_frame`], but its `a:graphicData@uri` is [`CHART_GRAPHIC_URI`](slide::CHART_GRAPHIC_URI)
/// and its payload is a self-closing `c:chart` naming the chart part by `r:id = rel_id`.
///
/// The slide binds `p`/`a`/`r` but not `c`, so the `c:chart` declares `xmlns:c` itself; `rel_declaration`
/// (an `xmlns:r`, usually `None` because the slide already binds `r`) is added when the slide does not
/// bind `r` — exactly how Office writes `<c:chart xmlns:c="…" r:id="…"/>`.
fn build_chart_frame(
    interner: &mut Interner,
    id: u32,
    rel_id: &str,
    bounds: ShapeBounds,
    rel_declaration: Option<RawAttribute>,
) -> RawElement {
    // p:nvGraphicFramePr — cNvPr, cNvGraphicFramePr (locked against grouping), and an empty nvPr.
    let cnvpr_attrs = vec![
        build::attr(interner, "id", &id.to_string()),
        build::attr(interner, "name", &format!("Chart {id}")),
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

    // c:chart — references the chart part by r:id, declaring the chart namespace it introduces (and
    // the relationships prefix when the slide does not already bind it).
    let mut chart_attrs = vec![build::namespace_declaration(
        interner,
        "c",
        DML_CHART.transitional,
    )];
    if let Some(declaration) = rel_declaration {
        chart_attrs.push(declaration);
    }
    let rel_prefix = interner.intern(build::RELATIONSHIP_PREFIX);
    chart_attrs.push(build::attr_prefixed(interner, rel_prefix, "id", rel_id));
    let chart = build::leaf(interner, "c", DML_CHART, "chart", chart_attrs);

    let data_attrs = vec![build::attr(interner, "uri", slide::CHART_GRAPHIC_URI)];
    let graphic_data = build::node(
        interner,
        "a",
        DML_MAIN,
        "graphicData",
        data_attrs,
        vec![RawNode::Element(chart)],
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
