//! Charts on the Excel surface (MJXOFF-111, E4) — a `c:chart` inside an `xdr:graphicFrame`, read,
//! authored and edited, and the one chart case that exists nowhere else in this library.
//!
//! # This is the third wrapper, not a third implementation
//!
//! MJXOFF-103 moved every chart read and every chart edit **down** into
//! [`mjx_chart::chart_ops`], because `mjx-pptx` and `mjx-docx` are both rank 3.0 and a chart is the
//! same `c:chartSpace` part in either. This crate is rank 3.0 too, so it takes the same road: every
//! method below is three things in a row — resolve `(sheet, anchor)` to a chart part, call the
//! identically-named function in `chart_ops`, and (for a data edit) refresh whatever the chart's
//! data really lives in. There is no Excel-local chart path and there must never be one.
//!
//! # How a worksheet chart is addressed
//!
//! A slide addresses a chart as `(surface, shape index)`; a document by its drawing's own
//! `wp:docPr@id`. A worksheet has neither. What it has is MJXOFF-107's **anchor index** — the
//! anchor's position in `xl/drawings/drawingN.xml`, which is also its paint order and which
//! [`SheetDrawingObject::index`](crate::SheetDrawingObject) already reports and
//! [`Workbook::remove_sheet_drawing_object`] already takes. A chart is addressed the same way, so
//! `add_chart`'s return value is accepted by `remove_sheet_drawing_object` exactly as
//! `add_two_cell_anchored_picture`'s is, and no second addressing scheme exists to keep in step.
//!
//! [`Workbook::chart_anchor_indices`] enumerates the anchors that frame a chart, which is the Excel
//! equivalent of walking a slide's shapes looking for chart frames.
//!
//! # The part layout, and the two ways a worksheet chart gets its data
//!
//! | part | written by |
//! |---|---|
//! | `xl/drawings/drawingN.xml` | MJXOFF-107, created on demand for a sheet that has none |
//! | `xl/charts/chartN.xml` | here — the `c:chartSpace` |
//! | `xl/drawings/_rels/drawingN.xml.rels` | here — [`REL_CHART`] **from the drawing part** |
//!
//! …and, for one of the two authoring calls, two more: `xl/embeddings/Microsoft_Excel_SheetN.xlsx`
//! and the [`REL_PACKAGE`] relationship from the chart part to it.
//!
//! **[`Workbook::add_chart`]** takes the same [`ChartData`] a slide and a document take and writes
//! the embedded workbook beside it. A caller who built a chart description for a deck adds it to a
//! workbook unchanged, which is the whole of MJXOFF-80's cross-format gate.
//!
//! **[`Workbook::add_range_chart`]** writes the case Excel itself writes: the `c:f` formulas name
//! **cells in this workbook**, the caches are seeded from what those cells say right now, and there
//! is no embedded workbook at all. `mjx_chart::ChartData::ranges` is what makes the first half
//! possible and [`crate::worksheet::chart_ranges`] the second.
//!
//! # A live-range chart never grows a workbook, and a data edit never invents one
//!
//! [`Workbook::refresh_chart_workbook`] answers `Ok(false)` — changing nothing — for a chart that
//! names no `c:externalData`, which is the *ordinary* state of a chart on a sheet. It is the same
//! shape `mjx_docx::Document::refresh_chart_workbook` takes for an unresolvable relationship or an
//! external target: **`false`, never a fabricated workbook**. A chart on a sheet that *does* carry
//! one refreshes normally.
//!
//! # Editing a cell a chart reads leaves the cache stale, and says so
//!
//! [`Workbook::set_cell_value`] touches no chart. That is the rule MJXOFF-107 states at the other
//! door — *adding a picture writes no cell, and editing a cell moves no picture* — and it is the
//! only rule consistent with a library that recalculates nothing (MJXOFF-115): a chart's cache is
//! what a consumer drew last, and rewriting it on a cell edit would be this library deciding it
//! knows better than the file.
//!
//! Silence about it would be the defect, so there is none.
//! [`Workbook::chart_series_freshness`] reports the cache and the cells side by side and names which
//! is which, and [`Workbook::refresh_chart_cache_from_cells`] is the **opt-in** repair — the exact
//! counterpart of `refresh_chart_workbook`, pointing the other way.

use mjx_chart::{
    chart_ops, embedded_workbook_for_chart_data, embedded_workbook_for_chart_space,
    AxisOrientation, ChartAxisData, ChartData, ChartDataError, ChartErrorBarData, ChartKind,
    ChartLabelScope, ChartLegendData, ChartPointFormatData, ChartRanges, ChartSeriesData,
    ChartSeriesRange, ChartSeriesReferences, ChartSpace, ChartTrendlineData,
    DanglingPointReference, DataLabelSettings, DataLabelSpec, ErrorBarSpec, LegendPosition,
    TrendlineSpec,
};
use mjx_dml::spreadsheet_drawing::{
    new_anchored_graphic_frame, new_two_cell_anchor, Anchor, AnchoredObject, CellMarker,
    WorksheetDrawing,
};
use mjx_dml::{FillSpec, LineSpec};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawNode, ToXml};
use mjx_ooxml_types::namespaces::DML_CHART;
use mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior;
use mjx_opc::{PartName, Relationship, TargetMode};

use crate::error::XlsxError;
use crate::parts::{CONTENT_TYPE_CHART, REL_CHART, REL_PACKAGE};
use crate::workbook::Workbook;

use super::chart_ranges::{RangeProblem, ResolvedRange};

/// The part-name stem of a chart's embedded workbook, matching what Office writes and what
/// `mjx-docx` writes beside a Word chart.
const CHART_WORKBOOK_STEM: &str = "Microsoft_Excel_Sheet";

/// Where one series of a chart authored by [`Workbook::add_range_chart`] takes its data from.
///
/// Every field is reference **text**, exactly as it will be written into the chart's `c:f`. This
/// crate parses it — [`Workbook::resolve_range_reference`] is the same code path a chart already in
/// the file goes through — so a reference that names nothing is refused rather than written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetChartSeries {
    /// The cell the series takes its name from (`Data!$B$1`), or `None` for a literal name.
    pub name_cell: Option<String>,
    /// The name to use when `name_cell` is `None`, or when the cell it names holds nothing.
    ///
    /// Always usable, which is why it is a `String` rather than an `Option<String>`: a series has a
    /// name in the cache whichever way the chart addresses it, and a blank header cell must not
    /// produce a nameless series.
    pub name: String,
    /// The cells the series' values come from (`Data!$B$2:$B$4`).
    pub values: String,
}

/// Where a chart authored by [`Workbook::add_range_chart`] takes its data from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetChartSource {
    /// The cells the shared category labels come from (`Data!$A$2:$A$4`), or `None` for a chart
    /// whose categories are the positions `1, 2, 3, …`.
    pub categories: Option<String>,
    /// The series, in the order they are to be plotted.
    pub series: Vec<SheetChartSeries>,
}

/// Where a chart on a sheet keeps its backing workbook, and whether that reference points outside
/// the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetChartWorkbook {
    /// The tab the chart is anchored on.
    pub sheet_index: usize,
    /// The anchor that frames it, in the drawing part's paint order.
    pub anchor_index: usize,
    /// The relationship target, as written — a part name relative to the chart part for an embedded
    /// workbook, or a URI for an external one.
    pub target: String,
    /// Whether the reference points outside the package, in which case the workbook is not this
    /// package's to rewrite.
    pub external: bool,
}

/// One series' cache set beside what its cells actually say (MJXOFF-111).
///
/// **Both are reported and each is named**, which is the honesty this library already applies to a
/// chart whose embedded workbook disagrees with its caches: the cache is what draws until a consumer
/// recalculates, and the cells are what a consumer would recalculate *from*. Neither is silently
/// preferred.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartSeriesFreshness {
    /// Which series this is, in the order [`Workbook::chart_series`] reports them.
    pub series_index: usize,
    /// What the chart draws today — its caches.
    pub cached: ChartSeriesData,
    /// The `c:f` of each of the series' sources, as the file wrote them. A field is `None` where the
    /// source is a literal and so has no cells behind it at all.
    pub references: ChartSeriesReferences,
    /// What the cells say, for the sources that are references *and* resolved. A source that is
    /// literal or unresolved contributes nothing here and says why below.
    pub from_cells: ChartSeriesData,
    /// Why the values reference did not resolve, or `None` when it did — or when there is none.
    pub values_problem: Option<RangeProblem>,
    /// Why the categories reference did not resolve, or `None`.
    pub categories_problem: Option<RangeProblem>,
    /// Whether the cached values and the cells agree.
    ///
    /// `None` means **cannot say**: the series' values are a literal, or its reference did not
    /// resolve. That is a third answer rather than a `false`, because "the cells disagree" and "there
    /// are no cells" are different things and a caller acting on the first must not be told it by
    /// the second.
    pub values_agree: Option<bool>,
    /// Whether the cached category labels and the cells agree. `None` as above.
    pub categories_agree: Option<bool>,
}

impl Workbook {
    // ---------------------------------------------------------------------------------------------
    // Finding the charts
    // ---------------------------------------------------------------------------------------------

    /// The index of every anchor on the tab at `sheet_index` that frames a chart, in paint order.
    ///
    /// The Excel counterpart of walking a slide's shapes looking for chart frames. An empty vector
    /// for a sheet with no drawing part, which is not an error.
    ///
    /// # Errors
    /// [`XlsxError::NoSuchSheet`] if `sheet_index` names no tab, or [`XlsxError`] if the worksheet or
    /// the drawing part is not well-formed XML.
    pub fn chart_anchor_indices(&self, sheet_index: usize) -> Result<Vec<usize>, XlsxError> {
        let mut indices = Vec::new();
        self.for_each_chart_frame(sheet_index, |index, rel_id| {
            if rel_id.is_some() {
                indices.push(index);
            }
        })?;
        Ok(indices)
    }

    /// The relationship id the anchor at `anchor_index` names as its chart part
    /// (`xdr:graphicFrame > a:graphic > a:graphicData > c:chart@r:id`), or `None` when that anchor
    /// frames no chart.
    ///
    /// # Errors
    /// As [`chart_anchor_indices`](Self::chart_anchor_indices).
    pub fn chart_rel_id(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Option<String>, XlsxError> {
        let mut found = None;
        self.for_each_chart_frame(sheet_index, |index, rel_id| {
            if index == anchor_index && found.is_none() {
                found = rel_id;
            }
        })?;
        Ok(found)
    }

    /// The raw XML bytes of the chart part the anchor at `anchor_index` frames
    /// (`xl/charts/chartN.xml`), exactly as the package holds them, or `None` when that anchor frames
    /// no chart. Borrowed from the package, so the part is not copied.
    ///
    /// # Errors
    /// As [`chart_anchor_indices`](Self::chart_anchor_indices).
    pub fn chart_part_bytes(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Option<&[u8]>, XlsxError> {
        let Some(part) = self.chart_part_for(sheet_index, anchor_index)? else {
            return Ok(None);
        };
        Ok(self.package().part_bytes(&part))
    }

    /// Calls `visit` for every anchor on the tab, with its index and the relationship id of the chart
    /// it frames (`None` when it frames something else).
    ///
    /// One walk serves every finder above, so they cannot disagree about which anchors are charts.
    /// The anchor is read through **the drawing part's own interner**, which is the trap MJXOFF-107
    /// found and fixed one door along: a name resolved through the wrong interner answers whatever
    /// string sits at that index rather than failing.
    fn for_each_chart_frame(
        &self,
        sheet_index: usize,
        mut visit: impl FnMut(usize, Option<String>),
    ) -> Result<(), XlsxError> {
        let Some((part, _)) = self.sheet_drawing_part(sheet_index)? else {
            return Ok(());
        };
        let Some((document, drawing)) = self.read_drawing_document(&part)? else {
            return Ok(());
        };
        let interner = &document.interner;
        for (index, anchor) in drawing.anchors(interner).enumerate() {
            let rel_id = match anchor.object(interner) {
                Some(AnchoredObject::GraphicFrame(frame)) => frame
                    .graphic(interner)
                    .and_then(|graphic| graphic.data().chart_relationship_id(interner)),
                _ => None,
            };
            visit(index, rel_id);
        }
        Ok(())
    }

    /// The chart part the anchor at `anchor_index` frames, or `None` when it frames none.
    ///
    /// The relationship is resolved against the **drawing** part, never the sheet: a `c:chart@r:id`
    /// sits inside the drawing, and OPC resolves a relationship identifier against the part that
    /// carries it.
    fn chart_part_for(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Option<PartName>, XlsxError> {
        let Some(rel_id) = self.chart_rel_id(sheet_index, anchor_index)? else {
            return Ok(None);
        };
        let Some((drawing_part, _)) = self.sheet_drawing_part(sheet_index)? else {
            return Ok(None);
        };
        self.resolve_sheet_relationship(&drawing_part, &rel_id)
    }

    /// The chart part the anchor frames, or [`XlsxError::AnchorIsNotAChart`].
    fn require_chart_part(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<PartName, XlsxError> {
        self.chart_part_for(sheet_index, anchor_index)?
            .ok_or(XlsxError::AnchorIsNotAChart {
                sheet_index,
                anchor_index,
            })
    }

    /// Reads the chart as a typed [`ChartSpace`] and hands it, with the part's interner, to `read`.
    /// Does **not** dirty the part.
    fn with_chart<R>(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        read: impl FnOnce(&ChartSpace, &Interner) -> Result<R, XlsxError>,
    ) -> Result<R, XlsxError> {
        let part = self.require_chart_part(sheet_index, anchor_index)?;
        let Some(bytes) = self.package().part_bytes(&part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        let document = mjx_xml::fidelity::parse(bytes).map_err(mjx_sml::SmlError::from)?;
        let space = ChartSpace::from_xml(&document.root, &document.interner)?;
        read(&space, &document.interner)
    }

    /// Parses the chart part, hands the whole [`ChartSpace`] to `edit`, and writes the mutated tree
    /// back — dirtying **only** the chart part.
    ///
    /// The document the model came from is what goes back out, not a fresh one:
    /// [`ToXml::write_back`] restores the source range of every node a rebuild reproduced unchanged,
    /// and serializing the original document keeps the part's own XML declaration and anything beside
    /// its root. Rebuilding a part one means to edit is the mistake MJXOFF-107 found in
    /// [`edit_drawing_markup`](Self::edit_drawing_markup) and fixed there.
    fn edit_chart(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        edit: impl FnOnce(&mut ChartSpace, &mut Interner) -> Result<(), XlsxError>,
    ) -> Result<(), XlsxError> {
        let part = self.require_chart_part(sheet_index, anchor_index)?;
        let Some(bytes) = self.package().part_bytes(&part) else {
            return Err(XlsxError::MissingWorkbookPart(part.as_str().to_owned()));
        };
        let mut document = mjx_xml::fidelity::parse(bytes).map_err(mjx_sml::SmlError::from)?;
        {
            let RawDocument { interner, root, .. } = &mut document;
            let mut space = ChartSpace::from_xml(root, interner)?;
            edit(&mut space, interner)?;
            space.write_back(root, interner);
        }
        self.package_mut()
            .replace_part_bytes(&part, mjx_xml::fidelity::serialize_to_vec(&document))?;
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Authoring
    // ---------------------------------------------------------------------------------------------

    /// Anchors `chart` between two cells on the tab at `sheet_index`, **with the embedded workbook**
    /// that Office's *Edit Data* opens, and answers the anchor's position in the drawing's paint
    /// order.
    ///
    /// This is the cross-format door: `chart` is the same [`ChartData`] a slide and a document take,
    /// so a caller who built one for a deck adds it to a workbook unchanged. Because a `ChartData`
    /// carries *values* and no cells to point at, the chart's `c:f` name the workbook this call
    /// writes beside it — exactly as they do in a `.pptx` and a `.docx`.
    ///
    /// For the case Excel itself writes — a chart whose data is a live range in this workbook, with
    /// no embedded copy at all — use [`add_range_chart`](Self::add_range_chart).
    ///
    /// Five things are written and they are written together: the drawing part (created if the sheet
    /// has none) with its content type, relationship and `x:drawing` entry; the chart part with its
    /// content type; the [`REL_CHART`] relationship **from the drawing part**; the embedded workbook
    /// part; and the [`REL_PACKAGE`] relationship from the chart part to it.
    ///
    /// `resizing` is the frame's `@editAs`, taking the same
    /// [`ResizingBehavior`](mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior) the three picture
    /// calls take. Excel and LibreOffice both write
    /// [`MoveWithCellsButDoNotResize`](mjx_ooxml_types::spreadsheetdrawing::ResizingBehavior::MoveWithCellsButDoNotResize)
    /// for a chart — a chart that stretched when a column widened would be redrawn at whatever shape
    /// the grid happened to make — and that is what to pass unless there is a reason not to.
    ///
    /// # Errors
    /// [`XlsxError::InvalidChartData`] if `chart` has nothing to draw, [`XlsxError::ChartData`] if
    /// the plot type constrains its series count and `chart` does not satisfy it,
    /// [`XlsxError::NoSuchSheet`] if `sheet_index` names no tab, or [`XlsxError`] if the package
    /// refuses an edit.
    pub fn add_chart(
        &mut self,
        sheet_index: usize,
        chart: &ChartData,
        from: CellMarker,
        to: CellMarker,
        name: &str,
        resizing: ResizingBehavior,
    ) -> Result<usize, XlsxError> {
        validate_chart_data(chart)?;
        let workbook = embedded_workbook_for_chart_data(chart)?;
        // The chart part is brand new, so its relationship space is empty and `rId1` is free.
        let workbook_rel_id = "rId1";
        let bytes = chart.to_part_bytes_linking_workbook(workbook_rel_id);
        let (anchor_index, chart_part) =
            self.write_chart(sheet_index, bytes, from, to, name, resizing)?;

        let workbook_part = PartName::new(&self.free_chart_workbook_part_name())?;
        self.package_mut().insert_part(
            &workbook_part,
            mjx_sml::write::CONTENT_TYPE_WORKBOOK_PACKAGE,
            workbook,
        )?;
        self.package_mut().add_relationship(
            Some(&chart_part),
            Relationship {
                id: workbook_rel_id.to_owned(),
                rel_type: REL_PACKAGE.to_owned(),
                target: super::tables::relative_target(&chart_part, &workbook_part),
                mode: TargetMode::Internal,
            },
        )?;
        Ok(anchor_index)
    }

    /// Anchors a chart of `kind` between two cells on the tab at `sheet_index`, taking its data from
    /// **cells in this workbook**, and answers the anchor's position in the drawing's paint order.
    ///
    /// This is the case that exists nowhere else in this library. The chart's `c:f` name the ranges
    /// `source` gives, its caches are seeded from what those cells say **right now**, and **no
    /// embedded workbook is written** — so [`refresh_chart_workbook`](Self::refresh_chart_workbook)
    /// answers `false` for the result, and [`chart_series_freshness`](Self::chart_series_freshness)
    /// can tell a caller when the sheet has moved on.
    ///
    /// Every reference is resolved before anything is written, through the same resolver a chart
    /// already in the file goes through, so a range naming a sheet this workbook does not have is
    /// refused rather than written as a `c:f` pointing at nothing.
    ///
    /// # Errors
    /// [`XlsxError::InvalidChartData`] if `source` names no series or every series resolves to
    /// nothing, [`XlsxError::ChartData`] if the plot type constrains its series count,
    /// [`XlsxError::Sml`] carrying an [`AddressError`](mjx_sml::AddressError) if a reference will not
    /// parse or names something this workbook does not have, [`XlsxError::NoSuchSheet`] if
    /// `sheet_index` names no tab, or [`XlsxError`] if the package refuses an edit.
    #[allow(clippy::too_many_arguments)]
    pub fn add_range_chart(
        &mut self,
        sheet_index: usize,
        kind: ChartKind,
        source: &SheetChartSource,
        from: CellMarker,
        to: CellMarker,
        name: &str,
        resizing: ResizingBehavior,
    ) -> Result<usize, XlsxError> {
        let (chart, ranges) = self.describe_range_chart(sheet_index, kind, source)?;
        validate_chart_data(&chart)?;
        let bytes = chart.ranges(ranges).to_part_bytes();
        let (anchor_index, _) = self.write_chart(sheet_index, bytes, from, to, name, resizing)?;
        Ok(anchor_index)
    }

    /// Reads `source`'s ranges out of the cells and builds the [`ChartData`] and [`ChartRanges`] that
    /// describe the chart they make.
    ///
    /// Every reference goes through [`resolve_range_reference`](Self::resolve_range_reference), so
    /// this is where an unresolvable range is refused — before a single part is written.
    fn describe_range_chart(
        &mut self,
        sheet_index: usize,
        kind: ChartKind,
        source: &SheetChartSource,
    ) -> Result<(ChartData, ChartRanges), XlsxError> {
        let mut categories: Vec<String> = Vec::new();
        if let Some(reference) = &source.categories {
            let resolved = self.require_resolved(sheet_index, reference)?;
            categories = resolved
                .cells
                .iter()
                .map(|cell| cell.value.label())
                .collect();
        }

        let mut chart = ChartData::new(kind);
        let mut ranges = ChartRanges {
            categories: source.categories.clone(),
            series: Vec::new(),
        };
        if !categories.is_empty() {
            chart = chart.categories(categories);
        }
        for series in &source.series {
            let resolved = self.require_resolved(sheet_index, &series.values)?;
            let values: Vec<f64> = resolved
                .cells
                .iter()
                .filter_map(|cell| cell.value.number())
                .collect();
            // A name cell that resolves to nothing falls back to the literal name rather than
            // producing a nameless series — see `SheetChartSeries::name`.
            let name = match &series.name_cell {
                None => series.name.clone(),
                Some(reference) => {
                    let resolved = self.require_resolved(sheet_index, reference)?;
                    resolved
                        .cells
                        .first()
                        .map(|cell| cell.value.label())
                        .filter(|label| !label.is_empty())
                        .unwrap_or_else(|| series.name.clone())
                }
            };
            chart = chart.series(name, values);
            ranges.series.push(ChartSeriesRange {
                name: series.name_cell.clone(),
                values: series.values.clone(),
            });
        }
        Ok((chart, ranges))
    }

    /// Resolves `reference` and refuses it unless every area came back.
    ///
    /// A chart already in a file is *read* leniently — a `c:f` naming a deleted sheet is a fact
    /// about that file, reported rather than raised. A chart this library is being asked to
    /// **author** is the opposite case: writing a reference that names nothing would put the defect
    /// there on purpose.
    fn require_resolved(
        &mut self,
        sheet_index: usize,
        reference: &str,
    ) -> Result<ResolvedRange, XlsxError> {
        let resolved = self.resolve_range_reference(sheet_index, reference)?;
        match resolved.problem() {
            None => Ok(resolved),
            Some(RangeProblem::Malformed(problem)) => {
                Err(XlsxError::Sml(mjx_sml::SmlError::Address(*problem)))
            }
            Some(_) => Err(XlsxError::TargetResolution {
                target: reference.to_owned(),
            }),
        }
    }

    /// Writes a chart part, relates it from the drawing part, and anchors a frame for it between two
    /// cells. Answers the anchor's paint-order position and the chart part's name.
    #[allow(clippy::too_many_arguments)]
    fn write_chart(
        &mut self,
        sheet_index: usize,
        chart_bytes: Vec<u8>,
        from: CellMarker,
        to: CellMarker,
        name: &str,
        resizing: ResizingBehavior,
    ) -> Result<(usize, PartName), XlsxError> {
        let drawing_part = self.drawing_part_or_create(sheet_index)?;
        let chart_part = PartName::new(&self.free_chart_part_name())?;
        self.package_mut()
            .insert_part(&chart_part, CONTENT_TYPE_CHART, chart_bytes)?;

        let relationship_id = self.next_sheet_relationship_id(&drawing_part);
        self.package_mut().add_relationship(
            Some(&drawing_part),
            Relationship {
                id: relationship_id.clone(),
                rel_type: REL_CHART.to_owned(),
                target: super::tables::relative_target(&drawing_part, &chart_part),
                mode: TargetMode::Internal,
            },
        )?;

        let Some((mut document, mut drawing)) = self.read_drawing_document(&drawing_part)? else {
            return Err(XlsxError::MissingWorkbookPart(
                drawing_part.as_str().to_owned(),
            ));
        };
        let at = {
            let RawDocument { interner, root, .. } = &mut document;
            let id = next_frame_id(&drawing, interner);
            let frame = new_anchored_graphic_frame(interner, id, name, &relationship_id);
            let anchor = Anchor::TwoCell(new_two_cell_anchor(
                interner,
                from,
                to,
                &AnchoredObject::GraphicFrame(frame),
                resizing,
            ));
            drawing.push_anchor(interner, &anchor);
            let at = drawing.anchor_count(interner).saturating_sub(1);
            drawing.write_back(root, interner);
            at
        };
        self.package_mut().replace_part_bytes(
            &drawing_part,
            mjx_xml::fidelity::serialize_to_vec(&document),
        )?;
        Ok((at, chart_part))
    }

    /// `/xl/charts/chartN.xml` for the smallest `N` the package does not already hold.
    ///
    /// Not the chart count, for the reason [`free_drawing_part_name`] is not the drawing count: a
    /// workbook whose second chart was deleted holds `chart1.xml` and `chart3.xml`.
    ///
    /// [`free_drawing_part_name`]: crate::worksheet::drawings
    fn free_chart_part_name(&self) -> String {
        self.free_numbered_part_name("/xl/charts/chart", ".xml")
    }

    /// `/xl/embeddings/Microsoft_Excel_SheetN.xlsx` for the smallest free `N`.
    ///
    /// The stem is the one Office itself uses for a chart's embedded workbook, and the one
    /// `mjx-docx` writes beside a Word chart, so a workbook this library authors is laid out like the
    /// documents it authors and like the files Office writes.
    fn free_chart_workbook_part_name(&self) -> String {
        self.free_numbered_part_name(&format!("/xl/embeddings/{CHART_WORKBOOK_STEM}"), ".xlsx")
    }

    /// `{prefix}{N}{suffix}` for the smallest `N` no part in the package already uses.
    fn free_numbered_part_name(&self, prefix: &str, suffix: &str) -> String {
        let taken: Vec<String> = self
            .package()
            .part_names()
            .map(|part| part.as_str().to_ascii_lowercase())
            .collect();
        let lower_prefix = prefix.to_ascii_lowercase();
        let lower_suffix = suffix.to_ascii_lowercase();
        for number in 1..=u32::MAX {
            let candidate = format!("{lower_prefix}{number}{lower_suffix}");
            if !taken.iter().any(|name| name == &candidate) {
                return format!("{prefix}{number}{suffix}");
            }
        }
        // Unreachable: the loop runs to four billion and a package cannot hold that many parts.
        format!("{prefix}1{suffix}")
    }

    // ---------------------------------------------------------------------------------------------
    // The embedded workbook — and the chart that has none
    // ---------------------------------------------------------------------------------------------

    /// Every chart in the workbook that references a backing workbook (`c:externalData`), with the
    /// anchor that frames it and whether the reference is external.
    ///
    /// A chart on a sheet usually has none, and is absent from this list rather than present with an
    /// empty target. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_anchor_indices`](Self::chart_anchor_indices).
    pub fn chart_workbooks(&self) -> Result<Vec<SheetChartWorkbook>, XlsxError> {
        let mut workbooks = Vec::new();
        for sheet_index in 0..self.sheets().len() {
            for anchor_index in self.chart_anchor_indices(sheet_index)? {
                let Some(chart_part) = self.chart_part_for(sheet_index, anchor_index)? else {
                    continue;
                };
                let Some(rel_id) = self.chart_external_data_rel_id(&chart_part)? else {
                    continue; // a chart with no backing workbook — the ordinary case on a sheet
                };
                let Some(rel) = self
                    .package()
                    .relationships_for(Some(&chart_part))
                    .and_then(|rels| rels.by_id(&rel_id))
                else {
                    continue; // the reference names no relationship — nothing to report
                };
                workbooks.push(SheetChartWorkbook {
                    sheet_index,
                    anchor_index,
                    target: rel.target.clone(),
                    external: rel.mode == TargetMode::External,
                });
            }
        }
        Ok(workbooks)
    }

    /// The relationship id a chart part's `c:externalData` names, or `None` when it has none.
    fn chart_external_data_rel_id(
        &self,
        chart_part: &PartName,
    ) -> Result<Option<String>, XlsxError> {
        let Some(bytes) = self.package().part_bytes(chart_part) else {
            return Ok(None);
        };
        let document = mjx_xml::fidelity::parse(bytes).map_err(mjx_sml::SmlError::from)?;
        let space = ChartSpace::from_xml(&document.root, &document.interner)?;
        Ok(space
            .external_data_rel_id(&document.interner)
            .map(str::to_owned))
    }

    /// Rewrites the embedded workbook of the chart at `(sheet_index, anchor_index)` so its cells hold
    /// exactly what the chart now draws, and answers whether it rewrote one.
    ///
    /// Answers `Ok(false)`, changing nothing, when there is nothing to refresh: the chart names no
    /// workbook — **which is the ordinary state of a chart on a sheet** — or the one it names is an
    /// external link, an unresolvable relationship, or a part the package does not hold. A
    /// live-range chart therefore never grows a workbook it did not have, and no workbook is ever
    /// fabricated.
    ///
    /// The workbook is **regenerated**, not patched: one sheet, column `A` the categories, column `B`
    /// onwards one per series, matching the layout the chart's own `c:f` name. That is what makes
    /// the two agree, and it is why formatting a third-party workbook carried does not survive a data
    /// edit.
    ///
    /// # Errors
    /// [`XlsxError::AnchorIsNotAChart`] if the anchor frames no chart, or [`XlsxError`] if the chart
    /// part is malformed or the package edit fails.
    pub fn refresh_chart_workbook(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<bool, XlsxError> {
        let chart_part = self.require_chart_part(sheet_index, anchor_index)?;
        let Some(bytes) = self.package().part_bytes(&chart_part) else {
            return Ok(false);
        };
        let document = mjx_xml::fidelity::parse(bytes).map_err(mjx_sml::SmlError::from)?;
        let space = ChartSpace::from_xml(&document.root, &document.interner)?;
        let Some(rel_id) = space
            .external_data_rel_id(&document.interner)
            .map(str::to_owned)
        else {
            return Ok(false);
        };
        let workbook = embedded_workbook_for_chart_space(&space)?;

        let Some(relationship) = self
            .package()
            .relationships_for(Some(&chart_part))
            .and_then(|rels| rels.by_id(&rel_id))
        else {
            return Ok(false);
        };
        if relationship.mode == TargetMode::External {
            return Ok(false);
        }
        let workbook_part = crate::nav::resolve_target(&chart_part, &relationship.target)?;
        if self.package().part_bytes(&workbook_part).is_none() {
            return Ok(false);
        }
        self.package_mut()
            .replace_part_bytes(&workbook_part, workbook)?;
        Ok(true)
    }

    /// Detaches the backing workbook from the chart at `(sheet_index, anchor_index)`: removes its
    /// `c:externalData` reference — the element and its relationship — leaving the chart to render
    /// from its cached values.
    ///
    /// If the reference was to an *embedded* workbook, that part is left unreferenced; sweep it with
    /// [`mjx_opc::Package::remove_unreferenced_parts`] if wanted. This never removes parts on its
    /// own. Dirties only the chart part.
    ///
    /// # Errors
    /// [`XlsxError::AnchorIsNotAChart`] if the anchor frames no chart,
    /// [`XlsxError::ChartHasNoExternalData`] if the chart references no workbook — which is the
    /// ordinary state of a chart on a sheet — or [`XlsxError`] if the chart is malformed.
    pub fn detach_chart_workbook(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<(), XlsxError> {
        let chart_part = self.require_chart_part(sheet_index, anchor_index)?;
        let rel_id = self
            .chart_external_data_rel_id(&chart_part)?
            .ok_or(XlsxError::ChartHasNoExternalData)?;
        {
            let Some(bytes) = self.package().part_bytes(&chart_part) else {
                return Err(XlsxError::MissingWorkbookPart(
                    chart_part.as_str().to_owned(),
                ));
            };
            let mut document = mjx_xml::fidelity::parse(bytes).map_err(mjx_sml::SmlError::from)?;
            let RawDocument { interner, root, .. } = &mut document;
            root.children.retain(|node| {
                !matches!(node, RawNode::Element(el)
                    if el.name.namespace.map(|ns| interner.resolve(ns))
                        == Some(DML_CHART.transitional)
                        && interner.resolve(el.name.local) == "externalData")
            });
            let bytes = mjx_xml::fidelity::serialize_to_vec(&document);
            self.package_mut().replace_part_bytes(&chart_part, bytes)?;
        }
        self.package_mut()
            .remove_relationship(Some(&chart_part), &rel_id)?;
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // The live range — the case that exists nowhere else
    // ---------------------------------------------------------------------------------------------

    /// Where every series of the chart says its data lives — the `c:f` beside each cache, as the file
    /// wrote it. Reading dirties nothing.
    ///
    /// The companion of [`chart_series`](Self::chart_series): that answers what the **caches** hold,
    /// this answers what the references **name**.
    ///
    /// # Errors
    /// [`XlsxError::AnchorIsNotAChart`] if the anchor frames no chart, or [`XlsxError`] if the chart
    /// part is malformed.
    pub fn chart_series_references(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Vec<ChartSeriesReferences>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, _interner| {
            Ok(chart_ops::series_references(space))
        })
    }

    /// Every series of the chart, read **from the cells its `c:f` names** rather than from its
    /// caches.
    ///
    /// The same [`ChartSeriesData`] shape [`chart_series`](Self::chart_series) answers, so the two
    /// can be compared directly — which is what [`chart_series_freshness`](Self::chart_series_freshness)
    /// does. A source that is a literal, or a reference that does not resolve, contributes an empty
    /// list rather than a guess; `chart_series_freshness` is what says which of those it was.
    ///
    /// Reading dirties nothing. See [`resolve_range_reference`](Self::resolve_range_reference) for
    /// what is resolved, what is not (a formula's *value*), and why the walk is bounded by the range
    /// rather than by the sheet.
    ///
    /// # Errors
    /// As [`chart_series_references`](Self::chart_series_references), plus [`XlsxError`] if a
    /// worksheet part a reference named is malformed.
    pub fn chart_series_from_cells(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Vec<ChartSeriesData>, XlsxError> {
        Ok(self
            .chart_series_freshness(sheet_index, anchor_index)?
            .into_iter()
            .map(|series| series.from_cells)
            .collect())
    }

    /// Every series' cache set beside what its cells actually say, with each named.
    ///
    /// This is the honest answer to *"what happens when the two disagree"*: **the cache is what
    /// draws until a consumer recalculates**, the cells are what a consumer would recalculate from,
    /// and neither is silently preferred. A caller that wants the sheet's answer to win calls
    /// [`refresh_chart_cache_from_cells`](Self::refresh_chart_cache_from_cells); one that wants the
    /// drawn answer to win does nothing.
    ///
    /// Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series_from_cells`](Self::chart_series_from_cells).
    pub fn chart_series_freshness(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Vec<ChartSeriesFreshness>, XlsxError> {
        let cached = self.chart_series(sheet_index, anchor_index)?;
        let references = self.chart_series_references(sheet_index, anchor_index)?;
        let mut out = Vec::with_capacity(cached.len());
        for (series_index, cached) in cached.into_iter().enumerate() {
            let references = references.get(series_index).cloned().unwrap_or_default();

            let mut from_cells = ChartSeriesData {
                name: None,
                categories: Vec::new(),
                values: Vec::new(),
            };
            let mut values_problem = None;
            let mut categories_problem = None;

            if let Some(reference) = &references.name {
                let resolved = self.resolve_range_reference(sheet_index, reference)?;
                from_cells.name = resolved.cells.first().map(|cell| cell.value.label());
            }
            let mut values_agree = None;
            if let Some(reference) = &references.values {
                let resolved = self.resolve_range_reference(sheet_index, reference)?;
                match resolved.problem() {
                    Some(problem) => values_problem = Some(problem.clone()),
                    None => {
                        from_cells.values = resolved
                            .cells
                            .iter()
                            .filter_map(|cell| cell.value.number())
                            .collect();
                        values_agree = Some(from_cells.values == cached.values);
                    }
                }
            }
            let mut categories_agree = None;
            if let Some(reference) = &references.categories {
                let resolved = self.resolve_range_reference(sheet_index, reference)?;
                match resolved.problem() {
                    Some(problem) => categories_problem = Some(problem.clone()),
                    None => {
                        from_cells.categories = resolved
                            .cells
                            .iter()
                            .map(|cell| cell.value.label())
                            .collect();
                        categories_agree = Some(from_cells.categories == cached.categories);
                    }
                }
            }

            out.push(ChartSeriesFreshness {
                series_index,
                cached,
                references,
                from_cells,
                values_problem,
                categories_problem,
                values_agree,
                categories_agree,
            });
        }
        Ok(out)
    }

    /// Rewrites the chart's caches from the cells its `c:f` name, and answers how many series it
    /// changed.
    ///
    /// The **opt-in** repair, and the exact counterpart of
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) pointing the other way: that one
    /// makes the workbook say what the chart draws, this one makes the chart draw what the sheet
    /// says. Nothing calls it for you — a cell edit deliberately leaves the caches alone, and
    /// [`chart_series_freshness`](Self::chart_series_freshness) is how a caller learns it wants this.
    ///
    /// A series whose reference does not resolve, or whose source is a literal, is left exactly as it
    /// was and is not counted. Dirties only the chart part, and only if something changed.
    ///
    /// # Errors
    /// As [`chart_series_from_cells`](Self::chart_series_from_cells).
    pub fn refresh_chart_cache_from_cells(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<usize, XlsxError> {
        let freshness = self.chart_series_freshness(sheet_index, anchor_index)?;
        let mut changed = 0usize;
        for series in freshness {
            if series.values_agree == Some(false) {
                self.set_chart_series_values(
                    sheet_index,
                    anchor_index,
                    series.series_index,
                    &series.from_cells.values,
                )?;
                changed += 1;
            }
            if series.categories_agree == Some(false) {
                let labels: Vec<&str> = series
                    .from_cells
                    .categories
                    .iter()
                    .map(String::as_str)
                    .collect();
                // A numeric or multi-level category source has no string labels to rewrite, and
                // `chart_ops` says so rather than inventing a shape for them. That refusal is not a
                // failure of this call: the series simply is not one whose labels can be refreshed.
                match self.set_chart_series_categories(
                    sheet_index,
                    anchor_index,
                    series.series_index,
                    &labels,
                ) {
                    Ok(()) => changed += 1,
                    Err(XlsxError::ChartAccess(_)) => {}
                    Err(other) => return Err(other),
                }
            }
        }
        Ok(changed)
    }

    // ---------------------------------------------------------------------------------------------
    // Reads — every one of these calls the identically-named `mjx_chart::chart_ops` function
    // ---------------------------------------------------------------------------------------------

    /// The series of the chart — for each, its name, category labels and values (for a scatter
    /// series, its X labels and Y values), flattened across the chart's plots, **from its caches**.
    /// Reading dirties nothing.
    ///
    /// [`chart_series_from_cells`](Self::chart_series_from_cells) is the same question asked of the
    /// cells.
    ///
    /// # Errors
    /// [`XlsxError::AnchorIsNotAChart`] if the anchor frames no chart, or [`XlsxError`] if the chart
    /// part is malformed.
    pub fn chart_series(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Vec<ChartSeriesData>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, _interner| {
            Ok(chart_ops::series(space))
        })
    }

    /// The kind of every plot the chart draws, in document order — one entry per plot element, so a
    /// combo chart yields several. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_kinds(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Vec<ChartKind>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, _interner| {
            Ok(chart_ops::kinds(space))
        })
    }

    /// The axes of the chart, in document order. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_axes(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Vec<ChartAxisData>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::axes(space, interner))
        })
    }

    /// The heading of the chart (`c:title`), or `None` when it has none. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_title(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Option<String>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, _interner| {
            Ok(chart_ops::title(space))
        })
    }

    /// The legend of the chart, or `None` when it has none. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_legend(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Option<ChartLegendData>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::legend(space, interner))
        })
    }

    /// The built-in style id the chart names (`c:style@val`, 1 to 48), or `None`. Reading dirties
    /// nothing.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_style_id(
        &self,
        sheet_index: usize,
        anchor_index: usize,
    ) -> Result<Option<u32>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::style_id(space, interner))
        })
    }

    /// The fill of series `series_idx` — what colour it is drawn in — or `None` when the series
    /// declares none and takes its colour from the chart style. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series), plus [`XlsxError::ChartAccess`] when `series_idx` is
    /// past the last series.
    pub fn chart_series_fill(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<Option<FillSpec>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::series_fill(space, interner, series_idx)?)
        })
    }

    /// The data-label settings **in force** for one point of series `series_idx` — the point's
    /// `c:dLbl` merged over the series' `c:dLbls` merged over the owning plot's.
    ///
    /// Pass `point_idx = None` to stop at the series tier. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_data_labels(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        point_idx: Option<u32>,
    ) -> Result<DataLabelSettings, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::data_labels(
                space, interner, series_idx, point_idx,
            )?)
        })
    }

    /// The data-label settings one **tier** states in its own right — what that tier contributes to
    /// the merge, with everything it leaves unset reported as `None`. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_data_label_tier(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        scope: ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::data_label_tier(space, interner, scope)?)
        })
    }

    /// The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None`. Reading
    /// dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_point_label_text(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        point_idx: u32,
    ) -> Result<Option<String>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::point_label_text(
                space, interner, series_idx, point_idx,
            )?)
        })
    }

    /// Every point of series `series_idx` that carries its own formatting (`c:dPt`), in document
    /// order. Each entry names the point it formats by `c:idx`. Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_point_formats(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<Vec<ChartPointFormatData>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::point_formats(space, interner, series_idx)?)
        })
    }

    /// Every trendline fitted through series `series_idx` (`c:trendline`), in document order.
    /// Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_trendlines(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<Vec<ChartTrendlineData>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::trendlines(space, interner, series_idx)?)
        })
    }

    /// Every set of error bars series `series_idx` carries (`c:errBars`). Reading dirties nothing.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_error_bars(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<Vec<ChartErrorBarData>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::error_bars(space, interner, series_idx)?)
        })
    }

    /// Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series no
    /// longer has. Reading dirties nothing.
    ///
    /// This library **never renumbers** such an element and never drops it on the caller's behalf;
    /// [`drop_chart_dangling_decoration`](Self::drop_chart_dangling_decoration) removes them.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_dangling_decoration(
        &self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<Vec<DanglingPointReference>, XlsxError> {
        self.with_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::dangling_decoration(space, interner, series_idx)?)
        })
    }

    // ---------------------------------------------------------------------------------------------
    // Edits
    // ---------------------------------------------------------------------------------------------

    /// Rewrites the values of series `series_idx` (0-based across the chart's plots) — whichever
    /// source the series names: a `c:numRef`'s cache or a `c:numLit`.
    ///
    /// The chart's embedded workbook is refreshed in the same call **when there is one**; a chart
    /// whose data is a live range has none, so nothing beyond the chart part is written and no
    /// workbook is fabricated. Marks the chart part dirty; a non-finite value is skipped.
    ///
    /// **This writes no cell.** Rewriting a series' cache does not change the range it reads from,
    /// exactly as changing a cell does not change the cache — see this module's own documentation.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), plus [`XlsxError::ChartAccess`] when the
    /// series has no numeric values to rewrite.
    pub fn set_chart_series_values(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        values: &[f64],
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_series_values(
                space, interner, series_idx, values,
            )?)
        })?;
        self.refresh_chart_workbook(sheet_index, anchor_index)?;
        Ok(())
    }

    /// Rewrites the category labels of series `series_idx`, refreshing the chart's embedded workbook
    /// alongside it when there is one.
    ///
    /// # Errors
    /// As [`set_chart_series_values`](Self::set_chart_series_values), with
    /// [`XlsxError::ChartAccess`] when the series' category source is numeric or multi-level and so
    /// has no string labels to rewrite.
    pub fn set_chart_series_categories(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        labels: &[&str],
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_series_categories(
                space, interner, series_idx, labels,
            )?)
        })?;
        self.refresh_chart_workbook(sheet_index, anchor_index)?;
        Ok(())
    }

    /// Sets or clears the explicit bounds of axis `axis_idx` (0-based, document order). `None`
    /// returns that end of the axis to automatic scaling. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), with [`XlsxError::ChartAccess`] when
    /// `axis_idx` is past the last axis.
    pub fn set_chart_axis_scale(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        axis_idx: usize,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_axis_scale(
                space, interner, axis_idx, minimum, maximum,
            )?)
        })
    }

    /// Sets the direction of axis `axis_idx` — smallest value first, or reversed. Marks only the
    /// chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_axis_scale`](Self::set_chart_axis_scale).
    pub fn set_chart_axis_orientation(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        axis_idx: usize,
        orientation: AxisOrientation,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_axis_orientation(
                space,
                interner,
                axis_idx,
                orientation,
            )?)
        })
    }

    /// Sets or removes the title of axis `axis_idx`. `None` removes the title. Marks only the chart
    /// part dirty.
    ///
    /// # Errors
    /// As [`set_chart_axis_scale`](Self::set_chart_axis_scale).
    pub fn set_chart_axis_title(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        axis_idx: usize,
        text: Option<&str>,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_axis_title(space, interner, axis_idx, text)?)
        })
    }

    /// Turns the gridlines of axis `axis_idx` on or off. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_axis_scale`](Self::set_chart_axis_scale).
    pub fn set_chart_axis_gridlines(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        axis_idx: usize,
        major: bool,
        minor: bool,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_axis_gridlines(
                space, interner, axis_idx, major, minor,
            )?)
        })
    }

    /// Sets or removes the chart's heading. `None` removes it. Marks only the chart part dirty.
    ///
    /// Setting a title also clears `c:autoTitleDeleted`, and removing one sets it — otherwise a
    /// consumer either refuses to draw the title given to it or invents one of its own.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series), with [`XlsxError::ChartAccess`] when the part
    /// declares no `c:chart`.
    pub fn set_chart_title(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        text: Option<&str>,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_title(space, interner, text)?)
        })
    }

    /// Places the chart's legend at `position`, adding one if the chart had none. `None` removes the
    /// legend. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_title`](Self::set_chart_title).
    pub fn set_chart_legend(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        position: Option<LegendPosition>,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_legend(space, interner, position)?)
        })
    }

    /// Sets the fill of series `series_idx`, creating its `c:spPr` if it had none. Marks only the
    /// chart part dirty.
    ///
    /// A [`FillSpec::Picture`] is **not** accepted: an image fill names an image relationship, and a
    /// chart part relates to no images, so it is refused rather than silently written as a dangling
    /// reference.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), plus [`XlsxError::ChartAccess`] for an
    /// image fill.
    pub fn set_chart_series_fill(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        fill: &FillSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_series_fill(
                space, interner, series_idx, fill,
            )?)
        })
    }

    /// Sets the outline of series `series_idx` — the line a line or radar plot draws, or the border
    /// of a bar or area. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn set_chart_series_line(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        line: &LineSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_series_line(
                space, interner, series_idx, line,
            )?)
        })
    }

    /// Applies `spec` at one tier of the chart's data labels, creating the element if that tier had
    /// none and leaving every setting `spec` does not state alone. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), plus [`XlsxError::ChartAccess`] carrying a
    /// [`ChartDataError`] when the schema does not admit the markup where it was asked for.
    pub fn set_chart_data_labels(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        scope: ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_data_labels(space, interner, scope, spec)?)
        })
    }

    /// Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings. Marks only
    /// the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels).
    pub fn suppress_chart_data_labels(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        scope: ChartLabelScope,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::suppress_data_labels(space, interner, scope)?)
        })
    }

    /// Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above it
    /// again. Answers whether an element was there. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels).
    pub fn remove_chart_data_labels(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        scope: ChartLabelScope,
    ) -> Result<bool, XlsxError> {
        let mut removed = false;
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            removed = chart_ops::remove_data_labels(space, interner, scope)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series,
    /// creating its `c:dPt` at the schema rank if it had none. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels), plus [`XlsxError::ChartAccess`]
    /// for an image fill.
    pub fn set_chart_point_fill(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        point_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_point_fill(
                space, interner, series_idx, point_idx, fill,
            )?)
        })
    }

    /// Outlines point `point_idx` of series `series_idx` differently from the rest of its series.
    /// Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_point_fill`](Self::set_chart_point_fill), minus the image-fill case.
    pub fn set_chart_point_line(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        point_idx: u32,
        line: &LineSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_point_line(
                space, interner, series_idx, point_idx, line,
            )?)
        })
    }

    /// Pulls slice `point_idx` of series `series_idx` out of the centre of its pie or doughnut by
    /// `percent` of the radius (`c:explosion`), or (for `None`) puts it back. Marks only the chart
    /// part dirty.
    ///
    /// # Errors
    /// As [`set_chart_point_fill`](Self::set_chart_point_fill), minus the image-fill case.
    pub fn set_chart_point_explosion(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        point_idx: u32,
        percent: Option<u32>,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_point_explosion(
                space, interner, series_idx, point_idx, percent,
            )?)
        })
    }

    /// Removes the formatting of point `point_idx` of series `series_idx`, so it is drawn like the
    /// rest of its series. Answers whether any was there. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn remove_chart_point_format(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        point_idx: u32,
    ) -> Result<bool, XlsxError> {
        let mut removed = false;
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            removed = chart_ops::remove_point_format(space, interner, series_idx, point_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends**.
    /// Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels); the plot-type case is a
    /// [`ChartDataError::DecorationNotAllowed`] (pie, doughnut, pie-of-pie, radar and surface series
    /// declare no `c:trendline`).
    pub fn add_chart_trendline(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        spec: &TrendlineSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::add_trendline(space, interner, series_idx, spec)?)
        })
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place**. Marks
    /// only the chart part dirty.
    ///
    /// # Errors
    /// As [`add_chart_trendline`](Self::add_chart_trendline), plus [`XlsxError::ChartAccess`] when
    /// the series carries fewer trendlines.
    pub fn set_chart_trendline(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        trendline_idx: usize,
        spec: &TrendlineSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_trendline(
                space,
                interner,
                series_idx,
                trendline_idx,
                spec,
            )?)
        })
    }

    /// Removes every trendline from series `series_idx`, answering how many went. Marks only the
    /// chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn remove_chart_trendlines(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<usize, XlsxError> {
        let mut removed = 0;
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            removed = chart_ops::remove_trendlines(space, interner, series_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Gives series `series_idx` error bars, replacing an existing set that runs along the same
    /// axis. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels); the plot-type case is a
    /// [`ChartDataError::DecorationNotAllowed`].
    pub fn set_chart_error_bars(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
        spec: &ErrorBarSpec,
    ) -> Result<(), XlsxError> {
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            Ok(chart_ops::set_error_bars(
                space, interner, series_idx, spec,
            )?)
        })
    }

    /// Removes every set of error bars from series `series_idx`, answering how many went. Marks only
    /// the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn remove_chart_error_bars(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<usize, XlsxError> {
        let mut removed = 0;
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            removed = chart_ops::remove_error_bars(space, interner, series_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Removes every `c:dPt` and `c:dLbl` of series `series_idx` that names a point past the end of
    /// its data, answering how many went. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn drop_chart_dangling_decoration(
        &mut self,
        sheet_index: usize,
        anchor_index: usize,
        series_idx: usize,
    ) -> Result<usize, XlsxError> {
        let mut removed = 0;
        self.edit_chart(sheet_index, anchor_index, |space, interner| {
            removed = chart_ops::drop_dangling_decoration(space, interner, series_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }
}

/// Refuses a chart description that cannot be written, before anything is.
///
/// "Nothing to draw" keeps its own variant, as it does on the other two surfaces, because a caller
/// that forgot to add a series and one that gave a stock chart two of them have made different
/// mistakes.
fn validate_chart_data(chart: &ChartData) -> Result<(), XlsxError> {
    match chart.validate() {
        Ok(()) => Ok(()),
        Err(ChartDataError::NoData) => Err(XlsxError::InvalidChartData),
        Err(problem) => Err(XlsxError::ChartData(problem)),
    }
}

/// The next `cNvPr@id` free in `drawing` — one past the highest any object in it uses.
///
/// The rule [`crate::worksheet::drawings`]' own `next_drawing_id` follows, applied to the frame this
/// module adds: one past the **maximum** rather than the count, because an id a caller removed may
/// still be named by markup this library did not write.
fn next_frame_id(drawing: &WorksheetDrawing, interner: &Interner) -> u32 {
    let mut highest = 0u32;
    for anchor in drawing.anchors(interner) {
        if let Some(id) = anchor
            .object(interner)
            .and_then(|object| object.identity(interner))
            .and_then(|props| props.id(interner).ok())
        {
            highest = highest.max(id);
        }
    }
    highest.saturating_add(1)
}
