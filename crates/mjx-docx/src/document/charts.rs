//! Charts on the Word surface (MJXOFF-103, E2) — a `c:chart` inside a `w:drawing`, read, authored
//! and edited.
//!
//! # What this module is, and what it deliberately is not
//!
//! It is **not** a chart model. `mjx-chart` has modelled `c:chartSpace` since Phase A — all sixteen
//! plot types, all four data sources, the axes, the titles, the whole decoration tier — and that
//! part is byte-for-byte the same part in a `.pptx`, a `.docx` and a `.xlsx`. What did not exist
//! until this child was anywhere but a `p:graphicFrame` to *reach* it from. This module is that
//! reach.
//!
//! So every method here is three things in a row: resolve a `wp:docPr` id to a chart part, call the
//! identically-named function in [`mjx_chart::chart_ops`], and (for a data edit) refresh the
//! embedded workbook. The middle step is shared with `mjx-pptx` rather than copied from it — see
//! that module's own doc comment for why the shared body had to move down into `mjx-chart` rather
//! than being reused sideways.
//!
//! # How a Word chart is addressed
//!
//! A slide addresses a chart as `(surface, shape index)`. A document has no shape tree; what it has
//! is the `wp:docPr@id` every inline or anchored drawing carries, which MJXOFF-131 already made
//! this crate's drawing address — [`Document::add_inline_picture`](crate::Document::add_inline_picture)
//! returns one and [`Document::remove_drawing`](crate::Document::remove_drawing) takes one. Charts
//! use the same address rather than inventing a second one, so `add_chart`'s return value is
//! accepted by `remove_drawing` exactly as `add_inline_picture`'s is.
//!
//! Ids are document-wide, so no paragraph or section has to be named again once the chart exists.
//! [`Document::chart_drawing_ids`](crate::Document::chart_drawing_ids) enumerates the ones that
//! frame a chart, which is the Word equivalent of walking a slide's shapes looking for chart frames.
//!
//! # The part layout
//!
//! Authoring one chart writes three parts and one run:
//!
//! * `word/charts/chartN.xml` — the chart, holding the series' caches (what renders) and a
//!   `c:externalData` naming its workbook;
//! * `word/embeddings/Microsoft_Excel_SheetN.xlsx` — the **embedded workbook**, laid out to match
//!   the chart's own `c:f` formulas cell for cell, which is what Word's *Edit Data* opens;
//! * `word/charts/_rels/chartN.xml.rels` — the relationship binding the two.
//!
//! The directory names are the ones Word itself uses (`word/charts/`, `word/embeddings/`), so a
//! document this library authors and one Word authors are laid out alike. The workbook is composed
//! by `mjx-chart`, which hands the rows to `mjx-sml`'s writer — MJXOFF-99's settled arrangement,
//! used here rather than re-created.

use std::borrow::Cow;

use mjx_chart::{
    apply_workbook_patch, chart_ops, embedded_workbook_for_chart_data,
    embedded_workbook_for_chart_space, embedded_workbook_part, plan_workbook_patch,
    AxisOrientation, ChartAxisData, ChartData, ChartDataError, ChartErrorBarData, ChartKind,
    ChartLabelScope, ChartLegendData, ChartPointFormatData, ChartSeriesData, ChartSeriesReferences,
    ChartSpace, ChartTrendlineData, DanglingPointReference, DataLabelSettings, DataLabelSpec,
    ErrorBarSpec, LegendPosition, TrendlineSpec, WorkbookPatch,
};
use mjx_dml::{FillSpec, LineSpec};
use mjx_ooxml_core::{FromXml, Interner, RawDocument, RawNode, ToXml};
use mjx_ooxml_types::namespaces::DML_CHART;
use mjx_opc::{PartName, Relationship, TargetMode};

use crate::address::BlockPath;
use crate::constants;
use crate::error::DocxError;

use super::body::{BlockContent, ParagraphContent, Run, RunInnerContent};
use super::drawing::{Drawing, DrawingContent};
use super::parts::ensure_theme_part;
use super::{Body, Document, MainDocument};

/// Where a chart sits in the text: in the line, or floating beside it.
///
/// This is the one place the Word chart surface has vocabulary the PowerPoint one does not, and it
/// is forced: a slide places every shape at an absolute `ShapeBounds`, while a Word drawing is
/// either an oversized character in a run (`wp:inline`) or a floating object the text flows around
/// (`wp:anchor`, `EG_WrapType`). MJXOFF-131 modelled both placements; this is how a caller picks
/// one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartPlacement {
    /// `wp:inline` — the chart flows with the surrounding text like a large character. This is what
    /// Word inserts by default, and what [`Document::add_chart`] uses.
    Inline,
    /// `wp:anchor` — the chart floats at an offset from the paragraph's own origin and the text
    /// wraps around it as `wrap` says.
    Floating {
        /// How far right of the column the chart's left edge sits, in EMU (`wp:positionH`, relative
        /// to `column`).
        offset_x_emu: i64,
        /// How far below the paragraph the chart's top edge sits, in EMU (`wp:positionV`, relative
        /// to `paragraph`).
        offset_y_emu: i64,
        /// How the surrounding text flows around the chart.
        wrap: ChartWrap,
    },
}

/// How text flows around a floating chart — the three members of `EG_WrapType` a rectangle needs.
///
/// `wp:wrapTight` and `wp:wrapThrough` are **not** offered here, and that is a decision rather than
/// an omission: both require a `wp:wrapPolygon`, and ECMA-376 Part 1 §20.4.2.16 states the polygon's
/// coordinates without stating the space they are measured in for a `CT_WrapPath` (the `a:CT_Point2D`
/// it reuses is EMU, while every producer this project has read writes a small fixed-range value).
/// Rather than guess a convention — which `CLAUDE.md` forbids and which would put a wrong polygon in
/// every document this library writes — a caller who needs an outline wrap builds one through
/// [`mjx_dml::wordprocessing_drawing::WrapOutline::new`], where the polygon is theirs to supply.
/// Reading a document that carries one is unaffected: `mjx-dml` models all five wrap modes and this
/// crate preserves them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartWrap {
    /// `wp:wrapNone` — the chart floats over or under the text and nothing reflows around it.
    None,
    /// `wp:wrapSquare` — text wraps around the chart's bounding box, on the sides given.
    Square(mjx_ooxml_types::wordprocessingdrawing::WrapText),
    /// `wp:wrapTopAndBottom` — text wraps above and below the chart only, never beside it.
    TopAndBottom,
}

/// The part-name stem of a chart's embedded workbook, matching what Office writes
/// (`/word/embeddings/Microsoft_Excel_Sheet1.xlsx`).
const CHART_WORKBOOK_STEM: &str = "Microsoft_Excel_Sheet";

/// Where a chart's backing workbook is, and whether that reference points outside the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentChartWorkbook {
    /// The `wp:docPr@id` of the drawing that frames the chart.
    pub drawing_id: u32,
    /// The relationship target, as written — a part name relative to the chart part for an embedded
    /// workbook, or a URI for an external one.
    pub target: String,
    /// Whether the reference points outside the package, in which case the workbook is not this
    /// document's to rewrite.
    pub external: bool,
}

/// The part a prepared workbook patch is bound for, and the bytes to write there.
///
/// `None` for the whole thing is *there is no embedded workbook*; `Some((part, None))` is *there is
/// one and every cell already said the right thing*, which must leave the part exactly as it is
/// because re-saving a package rewrites its container even when nothing inside it changed.
type PreparedWorkbook = Option<(PartName, Option<Vec<u8>>)>;

impl Document {
    // ---------------------------------------------------------------------------------------------
    // Finding the charts
    // ---------------------------------------------------------------------------------------------

    /// The `wp:docPr@id` of every drawing in the document body that frames a chart, in document
    /// order. Reading does not dirty the part.
    ///
    /// This is the Word counterpart of walking a slide's shapes looking for chart frames: a
    /// document has no shape tree, so the ids *are* the enumeration. The scan covers the body's own
    /// top-level paragraphs, the same scope
    /// [`add_inline_picture`](Self::add_inline_picture) and `remove_drawing` use and for the reason
    /// `Document::next_drawing_id`'s own doc comment gives.
    ///
    /// # Errors
    /// Returns [`DocxError::NoBody`] if the document declares no body, or another [`DocxError`] if
    /// the document part is malformed.
    pub fn chart_drawing_ids(&mut self) -> Result<Vec<u32>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let mut ids = Vec::new();
        for_each_drawing(body, &doc.interner, |drawing_id, rel_id| {
            if rel_id.is_some() {
                ids.push(drawing_id);
            }
        });
        Ok(ids)
    }

    /// The document theme's six accent colours, resolved to RGB — the palette a chart in this
    /// document hands out to series that state no fill of their own (MJXOFF-178).
    ///
    /// `Ok(None)` when the document relates to no `word/theme/themeN.xml`, or when its theme defines
    /// fewer than all six accents. Both answers mean *this document states no palette*, and a caller
    /// that invents one paints a customer's chart in colours their file does not carry.
    ///
    /// `mjx_pptx::Presentation` and `mjx_xlsx::Workbook` carry the identically named method over the
    /// identical `mjx_dml::SchemeColors::accents`, so a chart's colours do not depend on which of
    /// the three hosts it was opened from.
    ///
    /// # Errors
    /// Returns [`DocxError`] if the theme part cannot be read or is not well-formed DrawingML.
    pub fn theme_accent_colors(&mut self) -> Result<Option<[[u8; 3]; 6]>, DocxError> {
        let Some(theme_part) = self.parts.theme.clone() else {
            return Ok(None);
        };
        let doc = self.package.part_tree(&theme_part)?;
        let theme = mjx_dml::Theme::from_xml(&doc.root, &doc.interner)?;
        Ok(theme
            .color_scheme()
            .map(|scheme| mjx_dml::SchemeColors::from_scheme(scheme, &doc.interner))
            .and_then(|colors| colors.accents()))
    }

    /// The relationship id the drawing `drawing_id` names as its chart part
    /// (`w:drawing > … > a:graphicData > c:chart@r:id`), or `None` when that drawing frames no
    /// chart. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_drawing_ids`](Self::chart_drawing_ids).
    pub fn chart_rel_id(&mut self, drawing_id: u32) -> Result<Option<String>, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let mut found = None;
        for_each_drawing(body, &doc.interner, |id, rel_id| {
            if id == drawing_id && found.is_none() {
                found = rel_id;
            }
        });
        Ok(found)
    }

    /// The raw XML bytes of the chart part the drawing `drawing_id` references
    /// (`word/charts/chartN.xml`), exactly as the package holds them, or `None` when that drawing
    /// frames no chart. Borrowed from the package when the part still holds its bytes, and serialized
    /// on the spot when it has been edited — so a chart this crate has just written answers with what
    /// it now contains, and `None` means only that the drawing frames none (MJXOFF-222).
    ///
    /// # Errors
    /// As [`chart_rel_id`](Self::chart_rel_id), plus [`DocxError::ExternalTarget`] if the
    /// relationship points outside the package.
    pub fn chart_part_bytes(
        &mut self,
        drawing_id: u32,
    ) -> Result<Option<Cow<'_, [u8]>>, DocxError> {
        let Some(part) = self.chart_part_for(drawing_id)? else {
            return Ok(None);
        };
        Ok(self.package.part_payload(&part))
    }

    /// The part name of the chart the drawing `drawing_id` frames, or `None` when it frames none.
    fn chart_part_for(&mut self, drawing_id: u32) -> Result<Option<PartName>, DocxError> {
        let Some(rel_id) = self.chart_rel_id(drawing_id)? else {
            return Ok(None);
        };
        let Some(rels) = self.package.relationships_for(Some(&self.document_part)) else {
            return Ok(None);
        };
        let Some(rel) = rels.by_id(&rel_id) else {
            return Ok(None);
        };
        Ok(Some(super::parts::resolve_one(&self.document_part, rel)?))
    }

    /// The part name of the chart the drawing `drawing_id` frames, or
    /// [`DocxError::DrawingIsNotAChart`].
    fn require_chart_part(&mut self, drawing_id: u32) -> Result<PartName, DocxError> {
        self.chart_part_for(drawing_id)?
            .ok_or(DocxError::DrawingIsNotAChart { drawing_id })
    }

    /// Reads the chart the drawing `drawing_id` frames as a typed [`ChartSpace`] and hands it, with
    /// the part's interner, to `read`. Does **not** dirty the part.
    fn with_chart<R>(
        &mut self,
        drawing_id: u32,
        read: impl FnOnce(&ChartSpace, &Interner) -> Result<R, DocxError>,
    ) -> Result<R, DocxError> {
        let part = self.require_chart_part(drawing_id)?;
        let doc = self.package.part_tree(&part)?;
        let space = ChartSpace::from_xml(&doc.root, &doc.interner)?;
        read(&space, &doc.interner)
    }

    /// Parses the chart part the drawing `drawing_id` frames, hands the whole [`ChartSpace`] to
    /// `edit`, and writes the mutated tree back — dirtying **only** the chart part.
    fn edit_chart(
        &mut self,
        drawing_id: u32,
        edit: impl FnOnce(&mut ChartSpace, &mut Interner) -> Result<(), DocxError>,
    ) -> Result<(), DocxError> {
        let part = self.require_chart_part(drawing_id)?;
        let doc = self.package.part_tree_mut(&part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut space = ChartSpace::from_xml(root, interner)?;
        edit(&mut space, interner)?;
        space.write_back(root, interner);
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Authoring
    // ---------------------------------------------------------------------------------------------

    /// Adds `chart` to the document as a new **inline** chart, `width_emu` by `height_emu`, appended
    /// as a new run at the end of the paragraph at `paragraph`. Returns the drawing's own
    /// `wp:docPr` id, which every other method here takes and which
    /// [`remove_drawing`](Self::remove_drawing) takes to remove it again.
    ///
    /// Three parts are written and the paragraph gains one run — see this module's own doc comment
    /// for the layout. The chart takes the *same* [`ChartData`] description a PowerPoint chart
    /// takes, so a caller who has built one for a deck can add it to a document unchanged.
    ///
    /// # Errors
    /// [`DocxError::InvalidChartData`] if `chart` has nothing to draw (no series, or every series
    /// empty), [`DocxError::ChartData`] if the plot type constrains its series count and `chart`
    /// does not satisfy it, [`DocxError::NoBody`] if the document declares no body,
    /// [`DocxError::AddressNotFound`] if `paragraph` does not address a paragraph, or another
    /// [`DocxError`] if the package edit fails.
    pub fn add_chart(
        &mut self,
        paragraph: impl Into<BlockPath>,
        chart: &ChartData,
        width_emu: i64,
        height_emu: i64,
        name: &str,
    ) -> Result<u32, DocxError> {
        self.add_chart_placed(
            paragraph,
            chart,
            width_emu,
            height_emu,
            name,
            ChartPlacement::Inline,
        )
    }

    /// [`add_chart`](Self::add_chart), with the placement given: in the line, or floating with a
    /// wrap mode.
    ///
    /// # Errors
    /// As [`add_chart`](Self::add_chart).
    pub fn add_chart_placed(
        &mut self,
        paragraph: impl Into<BlockPath>,
        chart: &ChartData,
        width_emu: i64,
        height_emu: i64,
        name: &str,
        placement: ChartPlacement,
    ) -> Result<u32, DocxError> {
        match chart.validate() {
            Ok(()) => {}
            // "Nothing to draw" keeps its own variant, as it does on the PowerPoint surface.
            Err(ChartDataError::NoData) => return Err(DocxError::InvalidChartData),
            Err(problem) => return Err(DocxError::ChartData(problem)),
        }

        let paragraph_path = paragraph.into();
        // Everything fallible that does not touch the package happens first, so a failure here
        // leaves the document exactly as it was.
        {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            body.paragraph(&paragraph_path).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
            })?;
        }
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
                target: relative_target(&chart_part, &workbook_part),
                mode: TargetMode::Internal,
            },
        )?;
        let rel_id = self.next_rid_for(&self.document_part.clone());
        self.package.add_relationship(
            Some(&self.document_part),
            Relationship {
                id: rel_id.clone(),
                rel_type: constants::REL_CHART.to_owned(),
                target: relative_target(&self.document_part, &chart_part),
                mode: TargetMode::Internal,
            },
        )?;

        let doc = self.package.part_tree_mut(&self.document_part)?;
        let RawDocument { interner, root, .. } = doc;
        let mut main = MainDocument::from_xml(root, interner)?;
        let body = main.body_mut().ok_or(DocxError::NoBody)?;
        let doc_pr_id = Self::next_drawing_id(body, interner);

        let graphic_data = mjx_dml::GraphicData::for_chart(interner, &rel_id);
        let graphic = mjx_dml::Graphic::new(interner, graphic_data);
        let doc_properties =
            mjx_dml::wordprocessing_drawing::new_doc_properties(interner, doc_pr_id, name);
        let extent = mjx_dml::geometry::Size::from_emu(width_emu, height_emu);
        let content = build_placement(interner, placement, extent, doc_properties, graphic);
        let drawing = Drawing::new(interner, content);
        let run = Run::with_inner_content(interner, RunInnerContent::Drawing(drawing));

        let paragraph_mut = body.paragraph_mut(&paragraph_path).ok_or_else(|| {
            DocxError::AddressNotFound(format!("no paragraph at {paragraph_path}"))
        })?;
        paragraph_mut.append_run(run);
        main.write_back(root, interner);

        // A series this library authors carries no `c:spPr`, so its fill comes from the theme's
        // `accent1…accent6`. A document with no theme part resolves those to nothing and paints no
        // bars at all (MJXOFF-200). One is authored here **only** if the package carries none: a
        // document that arrived with a theme keeps it untouched, because supplying a default in
        // place of the user's own would override the branding of whoever opens the file.
        //
        // It is written **last**, after every step that can refuse, so a chart this call declines to
        // add leaves no theme behind — the same ordering `mjx_xlsx`'s `write_chart` states, and for
        // the same reason the preservation gate (MJXOFF-210) gave there.
        let theme_rel_id = self.next_rid_for(&self.document_part.clone());
        ensure_theme_part(
            &mut self.package,
            &self.document_part.clone(),
            &theme_rel_id,
        )?;

        Ok(doc_pr_id)
    }

    /// A fresh chart part name in `word/charts/`: `chart{N}.xml` with `N` one past the largest
    /// existing chart number.
    fn next_chart_part(&self) -> Result<PartName, DocxError> {
        let charts_dir = format!("{}charts/", dir_of(self.document_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = part
                .as_str()
                .strip_prefix(charts_dir.as_str())
                .and_then(|rest| rest.strip_prefix("chart"))
                .and_then(|rest| rest.strip_suffix(".xml"))
                .and_then(|n| n.parse::<u32>().ok())
            {
                max_n = max_n.max(n);
            }
        }
        PartName::new(&format!("{charts_dir}chart{}.xml", max_n + 1)).map_err(DocxError::from)
    }

    /// A fresh chart-workbook part name: `embeddings/Microsoft_Excel_Sheet{N}.xlsx` beside the
    /// document part, with `N` one past the largest existing one.
    ///
    /// The stem is the one Office itself uses for a chart's embedded workbook, so a document this
    /// library authors and one Word authors are named alike.
    fn next_chart_workbook_part(&self) -> Result<PartName, DocxError> {
        let embeddings_dir = format!("{}embeddings/", dir_of(self.document_part.as_str()));
        let mut max_n = 0u32;
        for part in self.package.part_names() {
            if let Some(n) = part
                .as_str()
                .strip_prefix(embeddings_dir.as_str())
                .and_then(|rest| rest.strip_prefix(CHART_WORKBOOK_STEM))
                .and_then(|rest| rest.strip_suffix(".xlsx"))
                .and_then(|n| n.parse::<u32>().ok())
            {
                max_n = max_n.max(n);
            }
        }
        PartName::new(&format!(
            "{embeddings_dir}{CHART_WORKBOOK_STEM}{}.xlsx",
            max_n + 1
        ))
        .map_err(DocxError::from)
    }

    // ---------------------------------------------------------------------------------------------
    // The embedded workbook
    // ---------------------------------------------------------------------------------------------

    /// Every chart in the document that references a backing workbook (`c:externalData`), with the
    /// drawing that frames it and whether the reference is external.
    ///
    /// An external workbook is the source that can be unreachable on another machine; a chart draws
    /// from its cached data regardless, so [`detach_chart_workbook`](Self::detach_chart_workbook)
    /// can safely remove the reference. Reading does not dirty any part.
    ///
    /// # Errors
    /// As [`chart_drawing_ids`](Self::chart_drawing_ids).
    pub fn chart_workbooks(&mut self) -> Result<Vec<DocumentChartWorkbook>, DocxError> {
        let mut workbooks = Vec::new();
        for drawing_id in self.chart_drawing_ids()? {
            let Some(chart_part) = self.chart_part_for(drawing_id)? else {
                continue;
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
            workbooks.push(DocumentChartWorkbook {
                drawing_id,
                target: rel.target.clone(),
                external: rel.mode == TargetMode::External,
            });
        }
        Ok(workbooks)
    }

    /// The relationship id a chart part's `c:externalData` names, or `None` when it has none.
    fn chart_external_data_rel_id(
        &mut self,
        chart_part: &PartName,
    ) -> Result<Option<String>, DocxError> {
        let doc = self.package.part_tree(chart_part)?;
        let space = ChartSpace::from_xml(&doc.root, &doc.interner)?;
        Ok(space.external_data_rel_id(&doc.interner).map(str::to_owned))
    }

    /// Writes the chart's data into the workbook the chart the drawing `drawing_id` frames already
    /// embeds, and answers whether it wrote one.
    ///
    /// Answers `Ok(false)`, changing nothing, when there is nothing to write: the chart names no
    /// workbook, the one it names is an **external** link rather than a part of this package, or
    /// every one of its series carries its data as a literal and so names no cells at all.
    ///
    /// # The workbook is patched, not replaced (MJXOFF-208)
    ///
    /// Each series' `c:f` says which cells its data lives in; this writes the chart's caches into
    /// **those** cells and touches nothing else. Every other sheet, every cell format, every defined
    /// name, every macro and every document property the workbook carried survives byte for byte,
    /// because the parts holding them are never rewritten.
    ///
    /// Until MJXOFF-208 this method regenerated the workbook instead — a fresh one-sheet package
    /// over the part the producer wrote — and a data edit called it for you, so opening a real
    /// document and changing one number silently discarded everything else that workbook held. That
    /// branch still exists, under the name that says what it does
    /// ([`regenerate_chart_workbook`](Self::regenerate_chart_workbook)), and a caller now has to ask
    /// for it.
    ///
    /// A `c:f` that names another workbook, several sheets, whole columns, a rectangle, or fewer
    /// cells than the data has points is a [`DocxError::ChartAccess`] — never a quiet fall back to
    /// regenerating, which would destroy the very content this method exists to keep.
    ///
    /// # What the answer means
    ///
    /// `true` says the chart **has** an embedded workbook this call is responsible for, not that
    /// bytes changed. A cell that already holds the value it was going to be given is not rewritten,
    /// so a call that finds everything in agreement answers `true` and leaves the part byte-identical
    /// — which is the only way a no-op refresh stays a no-op, because re-saving a package rewrites
    /// its container even when every part inside it is the same.
    ///
    /// [`set_chart_series_values`](Self::set_chart_series_values) and
    /// [`set_chart_series_categories`](Self::set_chart_series_categories) patch the one reference
    /// they edit; this one covers every reference the chart names.
    ///
    /// # Errors
    /// [`DocxError::DrawingIsNotAChart`] if the drawing frames no chart, [`DocxError::ChartAccess`]
    /// if a reference names cells this library will not write, or another [`DocxError`] if the chart
    /// part or the embedded package is malformed.
    pub fn refresh_chart_workbook(&mut self, drawing_id: u32) -> Result<bool, DocxError> {
        let prepared = self.prepare_chart_workbook(drawing_id, WorkbookPatch::EveryReference)?;
        self.commit_chart_workbook(prepared)
    }

    /// Replaces the embedded workbook of the chart the drawing `drawing_id` frames with a freshly
    /// built one, and answers whether it replaced one.
    ///
    /// **This discards whatever that workbook held.** One sheet, column `A` the categories, column
    /// `B` onwards one per series — the layout an authored chart's `c:f` formulas name. Any extra
    /// sheet, cell format, defined name, macro or document property the old workbook carried is
    /// gone, and so is any agreement between the new cells and a producer's own `c:f`.
    ///
    /// So this is the explicit opt-in, and [`refresh_chart_workbook`](Self::refresh_chart_workbook)
    /// is what a caller wants. [`detach_chart_workbook`](Self::detach_chart_workbook) is the other
    /// honest answer: drop the reference rather than write over the file.
    ///
    /// Answers `Ok(false)`, changing nothing, in exactly the cases
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) does.
    ///
    /// # Errors
    /// [`DocxError::DrawingIsNotAChart`] if the drawing frames no chart, or another [`DocxError`] if
    /// the chart part is malformed or the package edit fails.
    pub fn regenerate_chart_workbook(&mut self, drawing_id: u32) -> Result<bool, DocxError> {
        let chart_part = self.require_chart_part(drawing_id)?;

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
        if !self.package.contains_part(&workbook_part) {
            return Ok(false);
        }
        self.package.replace_part_bytes(&workbook_part, workbook)?;
        Ok(true)
    }

    /// Works out what the addressed chart's embedded workbook should become, **without writing it**.
    ///
    /// The half of a data edit that can refuse, kept separate from the half that changes the
    /// package. Answers `None` when there is no workbook to write.
    fn prepare_chart_workbook(
        &mut self,
        drawing_id: u32,
        patch: WorkbookPatch<'_>,
    ) -> Result<PreparedWorkbook, DocxError> {
        let chart_part = self.require_chart_part(drawing_id)?;
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
        let Some(bytes) = self.package.part_payload(&workbook_part) else {
            return Ok(None);
        };
        Ok(Some((workbook_part, apply_workbook_patch(&plan, &bytes)?)))
    }

    /// Writes what [`prepare_chart_workbook`](Self::prepare_chart_workbook) worked out, and answers
    /// whether there was an embedded workbook at all.
    fn commit_chart_workbook(&mut self, prepared: PreparedWorkbook) -> Result<bool, DocxError> {
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

    /// Detaches the backing workbook from the chart the drawing `drawing_id` frames: removes its
    /// `c:externalData` reference — the element and its relationship — leaving the chart to render
    /// from its cached values.
    ///
    /// If the reference was to an *embedded* workbook, that part is left unreferenced; sweep it with
    /// [`mjx_opc::Package::remove_unreferenced_parts`] if wanted. This never removes parts on its
    /// own. Dirties only the chart part.
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
    /// [`DocxError::DrawingIsNotAChart`] if the drawing frames no chart,
    /// [`DocxError::ChartHasNoExternalData`] if the chart references no workbook, or another
    /// [`DocxError`] if the chart is malformed.
    pub fn detach_chart_workbook(&mut self, drawing_id: u32) -> Result<(), DocxError> {
        let chart_part = self.require_chart_part(drawing_id)?;
        let rel_id = self
            .chart_external_data_rel_id(&chart_part)?
            .ok_or(DocxError::ChartHasNoExternalData)?;
        {
            let doc = self.package.part_tree_mut(&chart_part)?;
            let RawDocument { interner, root, .. } = doc;
            root.children.retain(|node| {
                !matches!(node, RawNode::Element(el)
                    if el.name.namespace.map(|ns| interner.resolve(ns))
                        == Some(DML_CHART.transitional)
                        && interner.resolve(el.name.local) == "externalData")
            });
        }
        self.package
            .remove_relationship(Some(&chart_part), &rel_id)?;
        Ok(())
    }

    // ---------------------------------------------------------------------------------------------
    // Reads — every one of these calls the identically-named `mjx_chart::chart_ops` function
    // ---------------------------------------------------------------------------------------------

    /// The series of the chart the drawing `drawing_id` frames — for each, its name, category labels
    /// and values (for a scatter series, its X labels and Y values), flattened across the chart's
    /// plots. Reading does not dirty the part.
    ///
    /// # Errors
    /// [`DocxError::DrawingIsNotAChart`] if the drawing frames no chart, or another [`DocxError`] if
    /// the chart part is malformed.
    pub fn chart_series(&mut self, drawing_id: u32) -> Result<Vec<ChartSeriesData>, DocxError> {
        self.with_chart(drawing_id, |space, _interner| Ok(chart_ops::series(space)))
    }

    /// The kind of every plot the chart draws, in document order — one entry per plot element, so a
    /// combo chart yields several. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_kinds(&mut self, drawing_id: u32) -> Result<Vec<ChartKind>, DocxError> {
        self.with_chart(drawing_id, |space, _interner| Ok(chart_ops::kinds(space)))
    }

    /// Where every series of the chart says its data lives — the `c:f` beside each cache, as the
    /// file wrote it. Reading does not dirty the part.
    ///
    /// The companion of [`chart_series`](Self::chart_series): that answers what the **caches** hold,
    /// this answers what the references **name**. A field is `None` where the source is a literal
    /// and so has no cells behind it at all.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_series_references(
        &mut self,
        drawing_id: u32,
    ) -> Result<Vec<ChartSeriesReferences>, DocxError> {
        self.with_chart(drawing_id, |space, _interner| {
            Ok(chart_ops::series_references(space))
        })
    }

    /// The axes of the chart, in document order. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_axes(&mut self, drawing_id: u32) -> Result<Vec<ChartAxisData>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::axes(space, interner))
        })
    }

    /// The heading of the chart (`c:title`), or `None` when it has none. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_title(&mut self, drawing_id: u32) -> Result<Option<String>, DocxError> {
        self.with_chart(drawing_id, |space, _interner| Ok(chart_ops::title(space)))
    }

    /// The legend of the chart, or `None` when it has none. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_legend(&mut self, drawing_id: u32) -> Result<Option<ChartLegendData>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::legend(space, interner))
        })
    }

    /// The built-in style id the chart names (`c:style@val`, 1 to 48) — the palette and effect set
    /// Word draws an unstyled series with — or `None` when it names none. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series).
    pub fn chart_style_id(&mut self, drawing_id: u32) -> Result<Option<u32>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::style_id(space, interner))
        })
    }

    /// The fill of series `series_idx` — what colour it is drawn in — or `None` when the series
    /// declares none and takes its colour from the chart style. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series`](Self::chart_series), plus [`DocxError::ChartAccess`] when `series_idx` is
    /// past the last series.
    pub fn chart_series_fill(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<Option<FillSpec>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::series_fill(space, interner, series_idx)?)
        })
    }

    /// The data-label settings **in force** for one point of series `series_idx` — the point's
    /// `c:dLbl` merged over the series' `c:dLbls` merged over the owning plot's.
    ///
    /// Pass `point_idx = None` to stop at the series tier. The merge is per setting: a series that
    /// only says "show the value" still takes its plot's label position. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_data_labels(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        point_idx: Option<u32>,
    ) -> Result<DataLabelSettings, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::data_labels(
                space, interner, series_idx, point_idx,
            )?)
        })
    }

    /// The data-label settings one **tier** states in its own right — what that tier contributes to
    /// the merge, with everything it leaves unset reported as `None`. Reading does not dirty the
    /// part.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_data_label_tier(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
    ) -> Result<Option<DataLabelSettings>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::data_label_tier(space, interner, scope)?)
        })
    }

    /// The words one point's label shows in place of its value (`c:dLbl > c:tx`), or `None` when it
    /// states none. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_point_label_text(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        point_idx: u32,
    ) -> Result<Option<String>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::point_label_text(
                space, interner, series_idx, point_idx,
            )?)
        })
    }

    /// Every point of series `series_idx` that carries its own formatting (`c:dPt`), in document
    /// order. Each entry names the point it formats by `c:idx`, not by its position in this list.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_point_formats(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<Vec<ChartPointFormatData>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::point_formats(space, interner, series_idx)?)
        })
    }

    /// Every trendline fitted through series `series_idx` (`c:trendline`), in document order.
    /// Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_trendlines(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<Vec<ChartTrendlineData>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::trendlines(space, interner, series_idx)?)
        })
    }

    /// Every set of error bars series `series_idx` carries (`c:errBars`) — one for a bar or line
    /// series, up to two (x and y) for scatter, area and bubble. Reading does not dirty the part.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_error_bars(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<Vec<ChartErrorBarData>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::error_bars(space, interner, series_idx)?)
        })
    }

    /// Every `c:dPt` and `c:dLbl` of series `series_idx` whose `c:idx` names a point the series no
    /// longer has. Reading does not dirty the part.
    ///
    /// A `c:dPt` is anchored by index into the series, so an edit that shortens the series can leave
    /// one addressing past the end. This library **never renumbers** such an element and never drops
    /// it on the caller's behalf; this reports them and
    /// [`drop_chart_dangling_decoration`](Self::drop_chart_dangling_decoration) removes them.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill).
    pub fn chart_dangling_decoration(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<Vec<DanglingPointReference>, DocxError> {
        self.with_chart(drawing_id, |space, interner| {
            Ok(chart_ops::dangling_decoration(space, interner, series_idx)?)
        })
    }

    // ---------------------------------------------------------------------------------------------
    // Edits
    // ---------------------------------------------------------------------------------------------

    /// Rewrites the values of series `series_idx` (0-based across the chart's plots) — whichever
    /// source the series names: a `c:numRef`'s cache or a `c:numLit`.
    ///
    /// The chart's **embedded workbook is patched in the same call**, so the numbers Word's *Edit
    /// Data* shows are the numbers the chart draws. The new values go into the cells the series' own
    /// `c:f` names and nowhere else — every other sheet, cell, format and defined name that workbook
    /// carried comes back exactly as it was; see
    /// [`refresh_chart_workbook`](Self::refresh_chart_workbook) for the whole story. Marks the chart
    /// part dirty, and the workbook part when there is something in it to change; a non-finite value
    /// is skipped.
    ///
    /// # All of it, or none of it
    ///
    /// The workbook is worked out **before** the chart is touched and written **after**, so a
    /// reference this library will not write refuses the whole call and leaves both parts as they
    /// were.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), plus [`DocxError::ChartAccess`] when the
    /// series has no numeric values to rewrite, or when its `c:f` names cells this library will not
    /// write.
    pub fn set_chart_series_values(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        values: &[f64],
    ) -> Result<(), DocxError> {
        let prepared = self.prepare_chart_workbook(
            drawing_id,
            WorkbookPatch::SeriesValues { series_idx, values },
        )?;
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::set_series_values(
                space, interner, series_idx, values,
            )?)
        })?;
        self.commit_chart_workbook(prepared)?;
        Ok(())
    }

    /// Rewrites the category labels of series `series_idx`, and patches the cells the series' own
    /// category `c:f` names alongside it.
    ///
    /// All-or-nothing in the same way [`set_chart_series_values`](Self::set_chart_series_values)
    /// is, and preserving in the same way.
    ///
    /// # Errors
    /// As [`set_chart_series_values`](Self::set_chart_series_values), with
    /// [`DocxError::ChartAccess`] when the series' category source is numeric or multi-level and so
    /// has no string labels to rewrite.
    pub fn set_chart_series_categories(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        labels: &[&str],
    ) -> Result<(), DocxError> {
        let prepared = self.prepare_chart_workbook(
            drawing_id,
            WorkbookPatch::SeriesCategories { series_idx, labels },
        )?;
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::set_series_categories(
                space, interner, series_idx, labels,
            )?)
        })?;
        self.commit_chart_workbook(prepared)?;
        Ok(())
    }

    /// Sets or clears the explicit bounds of axis `axis_idx` (0-based, document order). `None`
    /// returns that end of the axis to automatic scaling. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), with [`DocxError::ChartAccess`] when
    /// `axis_idx` is past the last axis.
    pub fn set_chart_axis_scale(
        &mut self,
        drawing_id: u32,
        axis_idx: usize,
        minimum: Option<f64>,
        maximum: Option<f64>,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        axis_idx: usize,
        orientation: AxisOrientation,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        axis_idx: usize,
        text: Option<&str>,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::set_axis_title(space, interner, axis_idx, text)?)
        })
    }

    /// Turns the gridlines of axis `axis_idx` on or off. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_axis_scale`](Self::set_chart_axis_scale).
    pub fn set_chart_axis_gridlines(
        &mut self,
        drawing_id: u32,
        axis_idx: usize,
        major: bool,
        minor: bool,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::set_axis_gridlines(
                space, interner, axis_idx, major, minor,
            )?)
        })
    }

    /// Sets or removes the chart's heading. `None` removes it. Marks only the chart part dirty.
    ///
    /// Setting a title also clears `c:autoTitleDeleted`, and removing one sets it — otherwise Word
    /// either refuses to draw the title given to it or invents one of its own.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), with [`DocxError::ChartAccess`] when the
    /// part declares no `c:chart`.
    pub fn set_chart_title(
        &mut self,
        drawing_id: u32,
        text: Option<&str>,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        position: Option<LegendPosition>,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
    /// As [`chart_series_fill`](Self::chart_series_fill), plus [`DocxError::ChartAccess`] for an
    /// image fill.
    pub fn set_chart_series_fill(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        fill: &FillSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        series_idx: usize,
        line: &LineSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::set_series_line(
                space, interner, series_idx, line,
            )?)
        })
    }

    /// Applies `spec` at one tier of the chart's data labels, creating the element if that tier had
    /// none and leaving every setting `spec` does not state alone. Marks only the chart part dirty.
    ///
    /// The three scopes are the three tiers: [`Plot`](ChartLabelScope::Plot) is the default every
    /// series takes, [`Series`](ChartLabelScope::Series) overrides it for one series, and
    /// [`Point`](ChartLabelScope::Point) overrides that for one point.
    ///
    /// # Errors
    /// As [`chart_series_fill`](Self::chart_series_fill), plus [`DocxError::ChartAccess`] carrying a
    /// [`ChartDataError`] when the schema does not admit the markup where it was asked for.
    pub fn set_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
        spec: &DataLabelSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::set_data_labels(space, interner, scope, spec)?)
        })
    }

    /// Suppresses the labels at one tier — a `c:delete val="1"` in place of the settings, which is
    /// how one series of a labelled plot, or one point of a labelled series, is silenced without
    /// disturbing the rest. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels).
    pub fn suppress_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::suppress_data_labels(space, interner, scope)?)
        })
    }

    /// Removes the `c:dLbls`/`c:dLbl` at one tier entirely, so that tier inherits the one above it
    /// again. Answers whether an element was there. Marks only the chart part dirty.
    ///
    /// This is the opposite of
    /// [`suppress_chart_data_labels`](Self::suppress_chart_data_labels): suppressing says "draw
    /// nothing here", removing says "say nothing here".
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels).
    pub fn remove_chart_data_labels(
        &mut self,
        drawing_id: u32,
        scope: ChartLabelScope,
    ) -> Result<bool, DocxError> {
        let mut removed = false;
        self.edit_chart(drawing_id, |space, interner| {
            removed = chart_ops::remove_data_labels(space, interner, scope)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Colours point `point_idx` of series `series_idx` differently from the rest of its series,
    /// creating its `c:dPt` at the schema rank if it had none. Marks only the chart part dirty.
    ///
    /// The point is addressed by index into the series, which is what `c:idx` means. An index at or
    /// past the series' point count is refused rather than written as markup that addresses nothing.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels), plus [`DocxError::ChartAccess`]
    /// for an image fill.
    pub fn set_chart_point_fill(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        point_idx: u32,
        fill: &FillSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        series_idx: usize,
        point_idx: u32,
        line: &LineSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        series_idx: usize,
        point_idx: u32,
        percent: Option<u32>,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        series_idx: usize,
        point_idx: u32,
    ) -> Result<bool, DocxError> {
        let mut removed = false;
        self.edit_chart(drawing_id, |space, interner| {
            removed = chart_ops::remove_point_format(space, interner, series_idx, point_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }

    /// Fits a trendline through series `series_idx`. `c:trendline` repeats, so this **appends** — a
    /// series may carry a linear fit and a moving average at once. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`set_chart_data_labels`](Self::set_chart_data_labels); the plot-type case is a
    /// [`ChartDataError::DecorationNotAllowed`] (pie, doughnut, pie-of-pie, radar and surface series
    /// declare no `c:trendline`).
    pub fn add_chart_trendline(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        spec: &TrendlineSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
            Ok(chart_ops::add_trendline(space, interner, series_idx, spec)?)
        })
    }

    /// Rewrites trendline `trendline_idx` of series `series_idx` from `spec`, **in place** — the
    /// curve keeps its own `c:spPr` and any `c:trendlineLbl` it carries, and every optional setting
    /// `spec` leaves unset is cleared. Marks only the chart part dirty.
    ///
    /// # Errors
    /// As [`add_chart_trendline`](Self::add_chart_trendline), plus [`DocxError::ChartAccess`] when
    /// the series carries fewer trendlines.
    pub fn set_chart_trendline(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        trendline_idx: usize,
        spec: &TrendlineSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<usize, DocxError> {
        let mut removed = 0;
        self.edit_chart(drawing_id, |space, interner| {
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
    /// [`ChartDataError::DecorationNotAllowed`] (pie, doughnut, pie-of-pie, radar and surface series
    /// declare no `c:errBars`).
    pub fn set_chart_error_bars(
        &mut self,
        drawing_id: u32,
        series_idx: usize,
        spec: &ErrorBarSpec,
    ) -> Result<(), DocxError> {
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<usize, DocxError> {
        let mut removed = 0;
        self.edit_chart(drawing_id, |space, interner| {
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
        drawing_id: u32,
        series_idx: usize,
    ) -> Result<usize, DocxError> {
        let mut removed = 0;
        self.edit_chart(drawing_id, |space, interner| {
            removed = chart_ops::drop_dangling_decoration(space, interner, series_idx)?;
            Ok(())
        })?;
        Ok(removed)
    }
}

// =================================================================================================
// Free helpers
// =================================================================================================

/// Builds the placement (`wp:inline` or `wp:anchor`) that wraps a chart's `a:graphic`.
///
/// The anchor's six flag attributes are given the values Word writes for a freshly inserted floating
/// object: it is in front of the text (`behindDoc="0"`), unlocked, laid out in its table cell,
/// allowed to overlap, and at the bottom of the z-order (`relativeHeight="0"`). `simplePos` is
/// written as the element the schema requires but **not** honoured (`simplePos="0"`), so the
/// `positionH`/`positionV` the caller gave are what a consumer uses.
fn build_placement(
    interner: &mut Interner,
    placement: ChartPlacement,
    extent: mjx_dml::geometry::Size,
    doc_properties: mjx_dml::NonVisualDrawingProps,
    graphic: mjx_dml::Graphic,
) -> DrawingContent {
    use mjx_dml::wordprocessing_drawing as wpd;
    match placement {
        ChartPlacement::Inline => {
            DrawingContent::Inline(wpd::Inline::new(interner, extent, doc_properties, graphic))
        }
        ChartPlacement::Floating {
            offset_x_emu,
            offset_y_emu,
            wrap,
        } => {
            let wrap = match wrap {
                ChartWrap::None => wpd::Wrap::None(wpd::WrapNone::new(interner)),
                ChartWrap::Square(text) => wpd::Wrap::Square(wpd::WrapSquare::new(interner, text)),
                ChartWrap::TopAndBottom => {
                    wpd::Wrap::TopAndBottom(wpd::WrapTopAndBottom::new(interner))
                }
            };
            let horizontal = wpd::HorizontalPosition::new(
                mjx_ooxml_types::wordprocessingdrawing::HorizontalRelativeFrom::Column,
                wpd::PositionValue::Offset(mjx_dml::geometry::Emu::from_emu(offset_x_emu)),
            );
            let vertical = wpd::VerticalPosition::new(
                mjx_ooxml_types::wordprocessingdrawing::VerticalRelativeFrom::Paragraph,
                wpd::PositionValue::Offset(mjx_dml::geometry::Emu::from_emu(offset_y_emu)),
            );
            DrawingContent::Anchored(wpd::Anchor::new(
                interner,
                mjx_dml::geometry::Position::from_emu(0, 0),
                false,
                horizontal,
                vertical,
                extent,
                wrap,
                0,
                false,
                false,
                true,
                true,
                doc_properties,
                graphic,
            ))
        }
    }
}

/// Calls `visit` for every drawing in `body`'s own top-level paragraphs, with its `wp:docPr@id` and
/// the relationship id of the chart it frames (`None` when it frames something else).
///
/// One walk serves [`Document::chart_drawing_ids`] and [`Document::chart_rel_id`] alike, so the two
/// cannot disagree about which drawings are charts.
fn for_each_drawing(body: &Body, interner: &Interner, mut visit: impl FnMut(u32, Option<String>)) {
    for paragraph in body.content().iter().filter_map(|item| match item {
        BlockContent::Paragraph(paragraph) => Some(paragraph),
        _ => None,
    }) {
        for run in paragraph.content().iter().filter_map(|item| match item {
            ParagraphContent::Run(run) => Some(run),
            _ => None,
        }) {
            for item in run.content() {
                let RunInnerContent::Drawing(drawing) = item else {
                    continue;
                };
                for placement in drawing.content() {
                    let (properties, graphic) = match placement {
                        DrawingContent::Inline(inline) => {
                            (inline.doc_properties(interner), inline.graphic(interner))
                        }
                        DrawingContent::Anchored(anchor) => {
                            (anchor.doc_properties(interner), anchor.graphic(interner))
                        }
                        DrawingContent::Raw(_) => (None, None),
                    };
                    let Some(id) = properties.and_then(|p| p.id(interner).ok()) else {
                        continue;
                    };
                    let rel_id = graphic.and_then(|graphic| {
                        graphic
                            .data()
                            .and_then(|data| data.chart_relationship_id(interner))
                    });
                    visit(id, rel_id);
                }
            }
        }
    }
}

/// The directory of `part`, including its trailing slash (`/word/document.xml` → `/word/`).
fn dir_of(part: &str) -> String {
    match part.rfind('/') {
        Some(i) => part[..=i].to_owned(),
        None => "/".to_owned(),
    }
}

/// The relationship target that reaches `target` from `source` — a sibling-relative path when the
/// two share a directory, and a package-absolute one otherwise.
///
/// A chart part (`/word/charts/chart1.xml`) and the document part (`/word/document.xml`) do not
/// share a directory, and neither do a chart part and its workbook
/// (`/word/embeddings/Microsoft_Excel_Sheet1.xlsx`), so both of this crate's own uses take the
/// second branch and write `charts/chart1.xml` / `../embeddings/Microsoft_Excel_Sheet1.xlsx`
/// respectively — exactly the forms Word writes.
fn relative_target(source: &PartName, target: &PartName) -> String {
    let source_dir = dir_of(source.as_str());
    if let Some(rest) = target.as_str().strip_prefix(source_dir.as_str()) {
        return rest.to_owned();
    }
    // Walk up from the source's directory to the deepest common prefix, then down to the target.
    let source_segments: Vec<&str> = source_dir.trim_matches('/').split('/').collect();
    let target_str = target.as_str().trim_start_matches('/');
    let target_segments: Vec<&str> = target_str.split('/').collect();
    let shared = source_segments
        .iter()
        .zip(target_segments.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let ups = source_segments.len() - shared;
    let mut out = String::new();
    for _ in 0..ups {
        out.push_str("../");
    }
    out.push_str(&target_segments[shared..].join("/"));
    out
}
