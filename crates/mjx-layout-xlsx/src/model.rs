//! [`SheetBoxModel`] — the code that turns a worksheet into a [`FragmentTree`], and the second
//! implementation of [`BoxModel`].
//!
//! # A page is a band of rows, and the horizontal window is the box model's own
//!
//! PowerPoint's mapping was free: one slide, one page. Excel's is a decision, and it is the first
//! place the contract's shape had to be read carefully.
//!
//! `mjx-view` windows in **one dimension**: pages stack downward, `Viewport::anchor` names a page and
//! an offset into it, and the prefetch ring is a range of page numbers. A grid scrolls in **two**.
//! So the two axes are answered differently, and the split is deliberate:
//!
//! * **Vertically, a page is a band of the sheet exactly `constraints.content.height()` tall**, so
//!   page *n* covers sheet rows from `content.height() * n` to `content.height() * (n + 1)`. Bands
//!   tile uniformly in sheet space, which is what makes `Extent { pages, page_size }` an honest
//!   answer and what makes resumption *structural*: page *n* does not need page *n − 1*'s checkpoint
//!   to know where it starts, so the equivalence
//!   [`BoxModel::layout_page`] promises cannot be got wrong here.
//! * **Horizontally, the window is [`SheetBoxModel::scroll_to_column`]** — state the box model
//!   holds, because [`Constraints`] has nowhere to put it and inventing a second content type would
//!   have made the *document* carry a scroll position.
//!
//! **That is a seam finding, and it is reported as one:** the box-model contract's page axis is
//! one-dimensional, and a grid is not. Nothing here works around it; the second axis is simply
//! stated where it can be.
//!
//! # Only what is on screen is laid out
//!
//! The walk is over **visible rows** and **visible columns** of the window, and every cell it asks
//! for is a binary search in `mjx-sml`'s packed store. Nothing iterates a coordinate range, in either
//! axis, at any point — which is the property `tests/sparsity_is_measured.rs` measures rather than
//! assumes, on a sheet whose only populated cell is `XFD1048576`.
//!
//! # Why this holds a `GlyphRasteriser`
//!
//! A [`GlyphRunFragment`] names a [`FaceId`](mjx_text::FaceId), and the only thing that mints one is
//! `GlyphRasteriser::register`. A painter must resolve the same identity to the same face, so the box
//! model and the painter share **one** rasteriser: this owns it and lends it through
//! [`SheetBoxModel::rasteriser_mut`]. R14 reported that as a seam finding and it holds here
//! unchanged — a box model that only measures still has to own the thing that numbers faces.

use std::collections::HashSet;

use mjx_layout::{
    BoxFragment, BoxModel, ChangeKind, ChangeSet, Checkpoint, ClipId, Constraints, DecorationRef,
    DirtyPages, Extent, ExtentPrecision, Fragment, FragmentId, FragmentTree, FragmentTreeBuilder,
    GlyphRunFragment, LayoutError, LayoutPoint, LayoutRect, LineFragment, ModelSignature,
    PageFragments, PageIndex, TableCell, TableFragment, TransformId,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::spreadsheetml::{BorderStyle, HorizontalAlignment, PatternType};
use mjx_sml::{Color, FontProperties};
use mjx_text::{FeatureSet, FontResolver, GlyphRasteriser, Shaper};

use crate::address;
use crate::autofit::{AutoFit, AutoFitCache};
use crate::cell::{self, CellContext, CellStyle, PlacedText};
use crate::error::SheetLayoutError;
use crate::geometry::{ColumnGeometry, GridGeometry, MaximumDigitWidth, COLUMN_COUNT};
use crate::merge::{MergedRegion, RegionEdge};
use crate::overflow::Overflow;
use crate::panes::{self, PaneRegion, Window};
use crate::sheet::SheetGrid;
use crate::text::{CellRunStyle, TextEngine};

/// One edge of a cell's border, as the file states it.
#[derive(Clone, PartialEq, Debug)]
pub struct BorderEdge {
    /// `x:border/*@style`.
    pub style: BorderStyle,
    /// The colour it is drawn in, or `None` when the file states none.
    ///
    /// A `mjx_sml::Color`, which addresses the theme **by position** — it is not a
    /// `mjx_dml::SchemeColor`, and resolving a `@theme` index against `xl/theme/theme1.xml` is the
    /// scene companion's job, not a box model's.
    pub colour: Option<Color>,
}

/// A cell's four border edges.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct CellBorders {
    /// The left edge.
    pub left: Option<BorderEdge>,
    /// The right edge.
    pub right: Option<BorderEdge>,
    /// The top edge.
    pub top: Option<BorderEdge>,
    /// The bottom edge.
    pub bottom: Option<BorderEdge>,
}

impl CellBorders {
    /// Whether any edge is drawn.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.left.is_none() && self.right.is_none() && self.top.is_none() && self.bottom.is_none()
    }

    /// One edge, by name.
    #[must_use]
    pub fn edge(&self, edge: RegionEdge) -> Option<&BorderEdge> {
        match edge {
            RegionEdge::Left => self.left.as_ref(),
            RegionEdge::Right => self.right.as_ref(),
            RegionEdge::Top => self.top.as_ref(),
            RegionEdge::Bottom => self.bottom.as_ref(),
        }
    }

    /// Replaces one edge.
    pub fn set(&mut self, edge: RegionEdge, value: Option<BorderEdge>) {
        match edge {
            RegionEdge::Left => self.left = value,
            RegionEdge::Right => self.right = value,
            RegionEdge::Top => self.top = value,
            RegionEdge::Bottom => self.bottom = value,
        }
    }
}

/// What paints a cell's background.
#[derive(Clone, PartialEq, Debug)]
pub struct CellFill {
    /// `x:patternFill@patternType`, or `None` for a fill that states a colour and no pattern —
    /// which is a third state beside `none` and `solid`, and is what a `dxf` writes.
    pub pattern: Option<PatternType>,
    /// `x:fgColor`.
    pub foreground: Option<Color>,
    /// `x:bgColor`.
    pub background: Option<Color>,
    /// Whether the fill is a gradient rather than a pattern.
    ///
    /// The stops themselves are not carried: `x:gradientFill` is a resource the scene companion
    /// resolves, and a box model that flattened it into two colours would have decided something.
    pub is_gradient: bool,
}

/// What a [`DecorationRef`] this box model issued resolves to.
///
/// The fragment tree carries only the number, deliberately: `mjx-layout` may never learn what a
/// SpreadsheetML fill is. This is the table the layer that holds the box model reads it in.
///
/// **Every value here is `mjx-sml`'s, unresolved past what the `xf` ladder already resolved.** A
/// `@theme` colour is still a theme index and a `numFmt` is still a format code — turning the first
/// into pixels needs `xl/theme/theme1.xml` and turning the second into a string is MJXOFF-172's
/// whole subject. A box model that did either would have merged two stages.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Decoration {
    /// What fills the cell, or `None` when nothing does.
    pub fill: Option<CellFill>,
    /// Its four borders.
    pub borders: CellBorders,
    /// The font its text is set in — carried so a painter has the colour, the underline and the
    /// strikethrough without resolving the `xf` ladder a second time.
    pub font: Option<FontProperties>,
    /// The number-format code in force, and the id it came from.
    ///
    /// **Reported, never applied.** R16 renders a cell's raw stored value; MJXOFF-172 is the
    /// evaluator. Carrying the code here is what lets that child be a change to one crate.
    pub number_format: Option<(u32, String)>,
}

/// What one laid-out cell did, beyond where its fragments went.
#[derive(Clone, PartialEq, Debug)]
pub struct CellReport {
    /// Which cell.
    pub row: u32,
    /// Which column.
    pub column: u16,
    /// What happened at its edges.
    pub overflow: Overflow,
    /// What shrink-to-fit multiplied its font size by, `1.0` when it did not run.
    pub shrink_scale: f64,
    /// The merged region it anchors, when it anchors one.
    pub merge: Option<MergedRegion>,
}

/// The tables the handles in one page's fragments resolve through.
///
/// Rebuilt on every [`BoxModel::layout_page`], because a handle is only meaningful for the page it
/// was issued on.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct PageCatalogue {
    decorations: Vec<Decoration>,
    /// Which handle each already-issued *effective format* resolved to.
    ///
    /// **A screen of unformatted cells shares one decoration**, which is what a resource table is
    /// for and what makes a band's catalogue a handful of entries rather than one per visible
    /// position. `EffectiveCellFormat` is `Copy`, `Eq` and built without allocating, so it is the
    /// natural key; a merged region is deliberately *not* shared, because its four borders are
    /// resolved from its own perimeter and two merges with the same anchor format can still differ.
    by_format: Vec<(mjx_sml::EffectiveCellFormat, DecorationRef)>,
    cells: Vec<CellReport>,
    regions: Vec<PaneRegion>,
    rows: Vec<u32>,
    columns: Vec<u16>,
    fits: Vec<(u16, AutoFit)>,
}

impl PageCatalogue {
    /// What a decoration handle resolves to.
    #[must_use]
    pub fn decoration(&self, handle: DecorationRef) -> Option<&Decoration> {
        usize::try_from(handle.number())
            .ok()
            .and_then(|index| self.decorations.get(index))
    }

    /// How many decoration handles were issued.
    ///
    /// One per **distinct effective format** on the band, never one per cell: a screen of
    /// unformatted cells issues one handle, which is what keeps a catalogue small enough to hold on
    /// the frame path.
    #[must_use]
    pub fn decoration_count(&self) -> usize {
        self.decorations.len()
    }

    /// The handle an effective format already resolved to on this page, if it has.
    fn handle_for(&self, format: &mjx_sml::EffectiveCellFormat) -> Option<DecorationRef> {
        self.by_format
            .iter()
            .find(|(candidate, _)| candidate == format)
            .map(|(_, handle)| *handle)
    }

    /// Records `decoration` and answers its handle, remembering `format` when one may be shared.
    fn intern(
        &mut self,
        decoration: Decoration,
        format: Option<mjx_sml::EffectiveCellFormat>,
    ) -> DecorationRef {
        let handle = DecorationRef::new(u64::try_from(self.decorations.len()).unwrap_or(0));
        self.decorations.push(decoration);
        if let Some(format) = format {
            self.by_format.push((format, handle));
        }
        handle
    }

    /// What every cell on the page did.
    #[must_use]
    pub fn cells(&self) -> &[CellReport] {
        &self.cells
    }

    /// What one cell did.
    #[must_use]
    pub fn cell(&self, row: u32, column: u16) -> Option<&CellReport> {
        self.cells
            .iter()
            .find(|report| report.row == row && report.column == column)
    }

    /// The pane regions the page was divided into, in paint order.
    #[must_use]
    pub fn regions(&self) -> &[PaneRegion] {
        &self.regions
    }

    /// Every row the page drew, ascending — hidden rows excluded, because a hidden row draws
    /// nothing.
    #[must_use]
    pub fn rows(&self) -> &[u32] {
        &self.rows
    }

    /// Every column the page drew, ascending.
    #[must_use]
    pub fn columns(&self) -> &[u16] {
        &self.columns
    }

    /// What every visible column stating `col@bestFit="1"` would be fitted to.
    ///
    /// **Reported, never applied**, and the two really are different acts. A `col` that asks to be
    /// best-fitted also *states a width* — `<col … width="12.5" bestFit="1"/>` — and that width is
    /// the one Excel computed when it last laid the column out, exactly as `ht` without
    /// `customHeight` is the height it computed for a row. Honouring the stored width reproduces
    /// what the author saw; recomputing it is running Excel's own measurement, which is what
    /// [`crate::autofit`] says is a reading rather than a fact.
    ///
    /// So the geometry keeps the file's width and this says what a *Format → AutoFit Column Width*
    /// command would set it to. A caller that wants the recomputed layout applies these widths and
    /// lays the band out again; one that wants the author's file does nothing, which is the default.
    #[must_use]
    pub fn auto_fits(&self) -> &[(u16, AutoFit)] {
        &self.fits
    }
}

/// Excel's box model.
#[derive(Debug)]
pub struct SheetBoxModel {
    fonts: FontResolver,
    rasteriser: GlyphRasteriser,
    shaper: Shaper,
    features: FeatureSet,
    catalogue: PageCatalogue,
    geometry: Option<(usize, GridGeometry)>,
    fits: AutoFitCache,
    first_column: u16,
    last_page: PageIndex,
}

impl SheetBoxModel {
    /// This box model's signature.
    ///
    /// `"XLSXLAY1"` in ASCII. The trailing `1` is the version of the *continuation state*, not of
    /// the crate: if the eight bytes a checkpoint carries ever change shape, this number changes and
    /// every old checkpoint is refused rather than misread.
    pub const SIGNATURE: ModelSignature = ModelSignature::new(0x584C_5358_4C41_5931);

    /// A box model that resolves faces through `fonts`.
    #[must_use]
    pub fn new(fonts: FontResolver) -> Self {
        Self {
            fonts,
            rasteriser: GlyphRasteriser::new(),
            shaper: Shaper::new(),
            features: FeatureSet::new(),
            catalogue: PageCatalogue::default(),
            geometry: None,
            fits: AutoFitCache::new(),
            first_column: 0,
            last_page: PageIndex::FIRST,
        }
    }

    /// The same box model shaping every cell with `features`.
    #[must_use]
    pub fn with_features(mut self, features: FeatureSet) -> Self {
        self.features = features;
        self
    }

    /// Scrolls the window so that `column` is the first one on the left of the scrolling region.
    ///
    /// **The second axis the contract has no room for.** `mjx-view` windows vertically — a page is a
    /// band of rows — and a grid also scrolls sideways, so the horizontal origin lives here. It is
    /// state of the *view* and not of the document, which is why it is on the box model and not on
    /// [`SheetGrid`].
    pub fn scroll_to_column(&mut self, column: u16) {
        let column = column.min(u16::try_from(COLUMN_COUNT - 1).unwrap_or(u16::MAX));
        if column != self.first_column {
            self.first_column = column;
        }
    }

    /// Which column the scrolling region starts at.
    #[must_use]
    pub fn first_column(&self) -> u16 {
        self.first_column
    }

    /// The font resolver, for the substitution manifest a workbook reports.
    #[must_use]
    pub fn fonts(&self) -> &FontResolver {
        &self.fonts
    }

    /// The rasteriser that minted every [`FaceId`](mjx_text::FaceId) in the fragments.
    ///
    /// A painter must draw through **this** rasteriser: a `FaceId` is an index into the rasteriser
    /// that issued it, so a second one would number the same faces differently and draw the wrong
    /// glyphs with no error anywhere.
    pub fn rasteriser_mut(&mut self) -> &mut GlyphRasteriser {
        &mut self.rasteriser
    }

    /// What the handles in the last page laid out resolve to.
    #[must_use]
    pub fn catalogue(&self) -> &PageCatalogue {
        &self.catalogue
    }

    /// The grid geometry for `grid`, building it on the first ask.
    ///
    /// It is built here rather than in the snapshot because a column's width is quoted in characters
    /// of the workbook's Normal font, so it needs a face resolved and a glyph measured.
    ///
    /// # Errors
    /// [`SheetLayoutError`] when a `col` is missing a required bound or a face will not shape.
    pub fn geometry(&mut self, grid: &SheetGrid) -> Result<&GridGeometry, SheetLayoutError> {
        if self
            .geometry
            .as_ref()
            .is_none_or(|(sheet, _)| *sheet != grid.index())
        {
            let built = self.build_geometry(grid)?;
            self.geometry = Some((grid.index(), built));
            self.fits.clear();
        }
        // The branch above guarantees the value is present for this sheet.
        self.geometry.as_ref().map(|(_, geometry)| geometry).ok_or(
            SheetLayoutError::NotAWorksheet {
                index: grid.index(),
            },
        )
    }

    /// Measures the Normal font's digit width and reads both axes.
    fn build_geometry(&mut self, grid: &SheetGrid) -> Result<GridGeometry, SheetLayoutError> {
        let worksheet = grid.worksheet();
        let format = worksheet.format_properties();
        let normal = self.normal_font(grid);
        let digit = match crate::text::resolve_face(&mut self.fonts, &mut self.rasteriser, &normal)?
        {
            Some((face, _)) => {
                MaximumDigitWidth::measure(&mut self.shaper, &face, normal.size, &self.features)?
            }
            None => MaximumDigitWidth::ASSUMED,
        };
        let rows = crate::geometry::RowGeometry::read(worksheet, format);
        let columns = ColumnGeometry::read(worksheet, format, digit)?;
        Ok(GridGeometry::new(rows, columns, digit))
    }

    /// The workbook's Normal font — the one `cellXfs[0]` resolves to, which is what an unformatted
    /// cell is set in and what every column width is quoted against.
    fn normal_font(&self, grid: &SheetGrid) -> CellRunStyle {
        let Ok(resolver) = grid.formatting().resolver() else {
            return CellRunStyle::default();
        };
        let Ok(format) = resolver.formats().effective_cell_format(None, None, None) else {
            return CellRunStyle::default();
        };
        let properties = resolver
            .formats()
            .font(&format)
            .map(|font| font.properties(resolver.formats().interner()));
        CellRunStyle::from_font(properties.as_ref())
    }

    /// The width `column` would be auto-fitted to, measuring every populated cell in it once and
    /// remembering the answer.
    ///
    /// # Errors
    /// [`SheetLayoutError`] when a face will not shape or a style index names no `xf`.
    pub fn auto_fit_width(
        &mut self,
        grid: &SheetGrid,
        column: u16,
    ) -> Result<AutoFit, SheetLayoutError> {
        let digit = self.geometry(grid)?.maximum_digit_width();
        let Self {
            fonts,
            rasteriser,
            shaper,
            features,
            fits,
            ..
        } = self;
        let mut engine = TextEngine {
            fonts,
            rasteriser,
            shaper,
            features,
        };
        fits.width(&mut engine, grid, digit, column)
    }
}

impl BoxModel for SheetBoxModel {
    type Content = SheetGrid;
    type Error = SheetLayoutError;

    fn signature(&self) -> ModelSignature {
        Self::SIGNATURE
    }

    fn layout_page(
        &mut self,
        content: &Self::Content,
        page: PageIndex,
        constraints: &Constraints,
        resume: Option<&Checkpoint>,
    ) -> Result<PageFragments, Self::Error> {
        // The checkpoint is validated and then *not used to position anything*: a band's rows come
        // from the page index and the row geometry, so page N alone and pages 1..=N in order agree
        // by construction rather than by care. See this module's own documentation.
        if let Some(checkpoint) = resume {
            let state = checkpoint.state_for(Self::SIGNATURE, page)?;
            if state.len() != 8 {
                return Err(SheetLayoutError::MalformedContinuation(state.len()));
            }
        }
        let band_height = constraints.content.height();
        if band_height <= Emu::ZERO {
            return Err(empty_area(constraints).into());
        }
        let pages = self.page_count(content, constraints);
        if page.number() >= pages && page != PageIndex::FIRST {
            return Err(LayoutError::PageBeyondContent {
                requested: page,
                last: PageIndex::new(pages.saturating_sub(1)),
            }
            .into());
        }

        let tree = self.lay_out_band(content, page, constraints)?;
        let continuation = if page.number().saturating_add(1) >= pages {
            None
        } else {
            let next = page.number().saturating_add(1);
            let first = self.band_first_row(content, next, band_height);
            let mut state = Vec::with_capacity(8);
            state.extend_from_slice(&first.to_le_bytes());
            state.extend_from_slice(&content.used_row_count().to_le_bytes());
            Some(Checkpoint::new(
                Self::SIGNATURE,
                page,
                address::node(content.part(), address::cell_path(first, self.first_column)),
                state,
            )?)
        };
        Ok(PageFragments::new(page, tree, continuation))
    }

    fn estimate_extent(&self, content: &Self::Content, constraints: &Constraints) -> Extent {
        Extent {
            pages: self.page_count(content, constraints).max(1),
            page_size: constraints.page,
            // Not an estimate, and not a boast: a row's height is read and never recomputed
            // (see `crate::geometry`), so the sheet's height is the sum of the stated overrides plus
            // a multiplication — which is exactly what `RowGeometry::top` computes in `O(log k)`.
            // A box model whose rows reflowed could not answer `Exact` here.
            precision: ExtentPrecision::Exact,
        }
    }

    fn invalidate(&mut self, change: &ChangeSet) -> DirtyPages {
        if change.is_empty() {
            return DirtyPages::None;
        }
        let Some((sheet, geometry)) = self.geometry.as_ref() else {
            // Nothing has been laid out, so nothing can be told apart. `All` is the honest answer.
            return DirtyPages::All;
        };
        let mut dirties_this_sheet = false;
        for content_change in change.changes() {
            let source = &content_change.source;
            if address::sheet_of(source.part()) != *sheet {
                // Another tab. It shares no page numbering with this one, so nothing here is stale.
                continue;
            }
            dirties_this_sheet = true;
            let Some(&row) = source.path().segments().first() else {
                // The sheet itself changed — a pane, a default row height, a column width. Every
                // band moves.
                return DirtyPages::All;
            };
            if content_change.kind != ChangeKind::Reformatted && geometry.rows().span(row).is_some()
            {
                // A row that states its own height changed structurally, which can move every row
                // below it — so every band from the first one is stale. A box model that named a
                // single band here would leave a reader looking at rows drawn at the old offsets.
                return DirtyPages::From(PageIndex::FIRST);
            }
            // A cell's text can spill sideways but never downward, so the change is confined to the
            // band the row is in. Which band that is depends on the constraints, which this method
            // is not given — so the only band this can name is the one that was last laid out.
            if !self.catalogue.rows().contains(&row) {
                return DirtyPages::All;
            }
        }
        if !dirties_this_sheet {
            return DirtyPages::None;
        }
        DirtyPages::Pages(vec![self.last_page])
    }
}
impl SheetBoxModel {
    /// How many bands the sheet is worth.
    fn page_count(&self, content: &SheetGrid, constraints: &Constraints) -> u32 {
        let height = constraints.content.height();
        if height <= Emu::ZERO {
            return 1;
        }
        let used = content.used_row_count();
        if used == 0 {
            return 1;
        }
        let total = content.rows().top(used);
        let pages = total.emu().div_euclid(height.emu().max(1))
            + i64::from(total.emu().rem_euclid(height.emu().max(1)) > 0);
        u32::try_from(pages).unwrap_or(u32::MAX).max(1)
    }

    /// The first row band `page` shows.
    fn band_first_row(&self, content: &SheetGrid, page: u32, height: Emu) -> u32 {
        let top = height.times(i64::from(page));
        content.rows().row_at(top).unwrap_or(0)
    }

    /// Lays out one band of rows across every pane region.
    fn lay_out_band(
        &mut self,
        content: &SheetGrid,
        page: PageIndex,
        constraints: &Constraints,
    ) -> Result<FragmentTree, SheetLayoutError> {
        let height = constraints.content.height();
        let band_top = height.times(i64::from(page.number()));
        let band_bottom = band_top + height;
        let first_row = content.rows().row_at(band_top).unwrap_or(0);

        // Taken rather than cloned: a sheet with ten thousand stated rows would otherwise copy ten
        // thousand records onto the frame path. It is put back before this function returns, on
        // every path including the error ones.
        self.geometry(content)?;
        let Some((sheet_index, geometry)) = self.geometry.take() else {
            return Err(empty_area(constraints).into());
        };
        let laid = self.lay_out_band_with(
            content,
            page,
            constraints,
            &geometry,
            first_row,
            band_top,
            band_bottom,
        );
        self.geometry = Some((sheet_index, geometry));
        laid
    }

    /// The band walk itself, with the geometry borrowed out of `self`.
    #[allow(clippy::too_many_arguments)]
    fn lay_out_band_with(
        &mut self,
        content: &SheetGrid,
        page: PageIndex,
        constraints: &Constraints,
        geometry: &GridGeometry,
        first_row: u32,
        band_top: Emu,
        band_bottom: Emu,
    ) -> Result<FragmentTree, SheetLayoutError> {
        let window = Window {
            first_row,
            first_column: self.first_column,
            content: constraints.content,
        };
        let regions = panes::regions(geometry, content.split(), &window);

        let mut catalogue = PageCatalogue {
            regions: regions.clone(),
            ..PageCatalogue::default()
        };
        let mut builder = FragmentTreeBuilder::new();
        let page_rect = LayoutRect::from_origin_and_size(LayoutPoint::ORIGIN, constraints.page);
        let root = builder.push_simple(
            None,
            address::node(content.part(), address::sheet_path()),
            page_rect,
            Fragment::Box(BoxFragment {
                decoration: None,
                cell: None,
            }),
        );

        for region in &regions {
            self.lay_out_region(
                &mut builder,
                &mut catalogue,
                content,
                geometry,
                region,
                root,
                band_top,
                band_bottom,
                page,
            )?;
        }

        catalogue.rows.sort_unstable();
        catalogue.rows.dedup();
        catalogue.columns.sort_unstable();
        catalogue.columns.dedup();

        // Every visible column that asks to be sized to its content gets an answer, **reported
        // rather than applied**. See `PageCatalogue::auto_fits` for why the two are different acts.
        let digit = geometry.maximum_digit_width();
        let asking: Vec<u16> = catalogue
            .columns
            .iter()
            .copied()
            .filter(|column| geometry.columns().wants_auto_fit(*column))
            .collect();
        for column in asking {
            let Self {
                fonts,
                rasteriser,
                shaper,
                features,
                fits,
                ..
            } = self;
            let mut engine = TextEngine {
                fonts,
                rasteriser,
                shaper,
                features,
            };
            if let Ok(fit) = fits.width(&mut engine, content, digit, column) {
                catalogue.fits.push((column, fit));
            }
        }
        self.catalogue = catalogue;
        self.last_page = page;
        Ok(builder.finish())
    }

    /// One pane region: its table fragment, and every cell of the window inside it.
    #[allow(clippy::too_many_arguments)]
    fn lay_out_region(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        catalogue: &mut PageCatalogue,
        content: &SheetGrid,
        geometry: &GridGeometry,
        region: &PaneRegion,
        parent: Option<FragmentId>,
        band_top: Emu,
        band_bottom: Emu,
        page: PageIndex,
    ) -> Result<(), SheetLayoutError> {
        if region.is_empty() {
            return Ok(());
        }
        let rows = self.visible_rows(content, geometry, region, band_top, band_bottom);
        let columns = self.visible_columns(geometry, region);
        if rows.is_empty() || columns.is_empty() {
            return Ok(());
        }
        let (Some(&first_row), Some(&last_row)) = (rows.first(), rows.last()) else {
            return Ok(());
        };
        let (Some(&first_column), Some(&last_column)) = (columns.first(), columns.last()) else {
            return Ok(());
        };

        let table = builder.push_simple(
            parent,
            address::node(content.part(), address::sheet_path()),
            region.view,
            Fragment::Table(TableFragment {
                columns: last_column.saturating_sub(first_column).saturating_add(1),
                rows: first_row..last_row.saturating_add(1),
                header_rows: u16::try_from(content.split().frozen_rows).unwrap_or(u16::MAX),
                continued_from_previous_page: page != PageIndex::FIRST && !region.rows_are_frozen,
                continues_on_next_page: !region.rows_are_frozen
                    && u64::from(last_row).saturating_add(1) < u64::from(content.used_row_count()),
            }),
        );

        let clip = builder.clip(region.view);
        let resolver = content.formatting().resolver()?;
        let interner = resolver.formats().interner();
        let mut drawn: HashSet<(u32, u16)> = HashSet::new();

        // The merged regions that reach into this window but whose anchor does not — a merge that
        // starts above the band or to the left of the window still has to be drawn, and a walk that
        // only visited the window's own cells would lose it. This is the fixture a naive per-cell
        // walk gets wrong.
        let mut pending: Vec<MergedRegion> = content
            .merges()
            .overlapping(first_row..last_row.saturating_add(1))
            .into_iter()
            .filter(|merge| {
                merge.last_column >= first_column
                    && merge.first_column <= last_column
                    && !(rows.contains(&merge.first_row) && columns.contains(&merge.first_column))
            })
            .collect();
        pending.sort_unstable_by_key(|merge| (merge.first_row, merge.first_column));
        pending.dedup_by_key(|merge| (merge.first_row, merge.first_column));

        for &row in &rows {
            catalogue.rows.push(row);
            for &column in &columns {
                if drawn.contains(&(row, column)) {
                    continue;
                }
                let merge = content.merges().covering(row, column);
                if let Some(merge) = merge {
                    if !merge.is_anchor(row, column) {
                        // A covered cell renders nothing at all — not its text, not its own fill,
                        // not its own border. The anchor owns the whole union.
                        continue;
                    }
                    for member_row in merge.first_row..=merge.last_row {
                        for member_column in merge.first_column..=merge.last_column {
                            drawn.insert((member_row, member_column));
                        }
                    }
                }
                self.lay_out_cell(
                    builder, catalogue, content, geometry, &resolver, interner, region, table,
                    clip, row, column, merge,
                )?;
            }
        }
        for &column in &columns {
            catalogue.columns.push(column);
        }
        for merge in pending {
            if drawn.contains(&(merge.first_row, merge.first_column)) {
                continue;
            }
            self.lay_out_cell(
                builder,
                catalogue,
                content,
                geometry,
                &resolver,
                interner,
                region,
                table,
                clip,
                merge.first_row,
                merge.first_column,
                Some(merge),
            )?;
        }
        Ok(())
    }

    /// The rows of `region` that fall inside the band and are not hidden.
    ///
    /// A **frozen** region ignores the band: its rows are pinned and appear on every page, which is
    /// what freezing means.
    fn visible_rows(
        &self,
        content: &SheetGrid,
        geometry: &GridGeometry,
        region: &PaneRegion,
        band_top: Emu,
        band_bottom: Emu,
    ) -> Vec<u32> {
        let rows = geometry.rows();
        let mut out = Vec::new();
        let mut row = region.rows.start;
        let last_used = content.used_row_count();
        let limit = region.view.height();
        let mut used = Emu::ZERO;
        while row < region.rows.end && row < last_used.max(region.rows.start.saturating_add(1)) {
            let Some(visible) = rows.next_visible_row(row) else {
                break;
            };
            if visible >= region.rows.end {
                break;
            }
            let top = rows.top(visible);
            if !region.rows_are_frozen && top >= band_bottom {
                break;
            }
            let height = rows.height(visible);
            if region.rows_are_frozen || top + height > band_top {
                out.push(visible);
                used += height;
                if used >= limit {
                    break;
                }
            }
            row = visible.saturating_add(1);
            if row == 0 {
                break;
            }
        }
        out
    }

    /// The columns of `region` that fit in its view and are not hidden.
    fn visible_columns(&self, geometry: &GridGeometry, region: &PaneRegion) -> Vec<u16> {
        let columns = geometry.columns();
        let limit = region.view.width();
        let mut out = Vec::new();
        let mut used = Emu::ZERO;
        let mut column = u16::try_from(region.columns.start).unwrap_or(u16::MAX);
        let end = region.columns.end;
        while u32::from(column) < end {
            let Some(visible) = columns.next_visible_column(column) else {
                break;
            };
            if u32::from(visible) >= end {
                break;
            }
            out.push(visible);
            used += columns.width(visible);
            if used >= limit {
                break;
            }
            let Some(next) = visible.checked_add(1) else {
                break;
            };
            column = next;
        }
        out
    }

    /// One cell: its box, its decoration, and the fragments its text became.
    #[allow(clippy::too_many_arguments)]
    fn lay_out_cell(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        catalogue: &mut PageCatalogue,
        content: &SheetGrid,
        geometry: &GridGeometry,
        resolver: &mjx_xlsx::SheetFormatResolver<'_>,
        interner: &mjx_ooxml_core::Interner,
        region: &PaneRegion,
        parent: Option<FragmentId>,
        clip: Option<ClipId>,
        row: u32,
        column: u16,
        merge: Option<MergedRegion>,
    ) -> Result<(), SheetLayoutError> {
        let sheet_rect = match merge {
            Some(merge) => geometry.block_rect(
                merge.first_row,
                merge.first_column,
                merge.last_row,
                merge.last_column,
            ),
            None => geometry.cell_rect(row, column),
        };
        let rect = translate(sheet_rect, region.origin);
        let cell = content.cell(row, column);
        let sheet_row = content.row(row);
        let format = resolver.formats().effective_cell_format(
            cell.as_ref(),
            sheet_row.as_ref(),
            resolver
                .columns()
                .style_index(u32::from(column).saturating_add(1)),
        )?;
        let font = resolver
            .formats()
            .font(&format)
            .map(|font| font.properties(interner));
        let shared = merge
            .is_none()
            .then(|| catalogue.handle_for(&format))
            .flatten();
        let handle = match shared {
            Some(handle) => handle,
            None => {
                let mut decoration = Decoration {
                    fill: fill_of(resolver, &format, interner),
                    borders: borders_of(resolver, &format, interner),
                    font: font.clone(),
                    number_format: number_format_of(resolver, &format, interner),
                };
                if let Some(merge) = merge {
                    resolve_merge_borders(&mut decoration, resolver, interner, merge, content);
                }
                catalogue.intern(decoration, merge.is_none().then_some(format))
            }
        };

        let cell_node = builder.push(
            parent,
            address::node(content.part(), address::cell_path(row, column)),
            rect,
            TransformId::IDENTITY,
            clip,
            Fragment::Box(BoxFragment {
                decoration: Some(handle),
                cell: Some(TableCell {
                    column,
                    row,
                    column_span: merge.map_or(1, |merge| merge.column_span()),
                    row_span: merge.map_or(1, |merge| merge.row_span()),
                }),
            }),
        );

        let Some(cell) = cell else {
            return Ok(());
        };
        let Some(text) = content.cell_text(&cell) else {
            return Ok(());
        };
        if text.is_empty() {
            return Ok(());
        }

        let style = CellStyle::resolve(
            resolver.formats().alignment(&format),
            interner,
            font.as_ref(),
            cell.cell_type(),
        );
        let placed = self.place_cell(
            content, geometry, region, &style, &text, row, column, sheet_rect, merge,
        )?;

        catalogue.cells.push(CellReport {
            row,
            column,
            overflow: placed.overflow.clone(),
            shrink_scale: placed.scale,
            merge,
        });
        self.emit_text(
            builder, content, region, cell_node, clip, &placed, row, column,
        );
        Ok(())
    }

    /// Runs [`crate::cell::place`] for one cell, with the two overflow probes taken from the packed
    /// store's own index.
    #[allow(clippy::too_many_arguments)]
    fn place_cell(
        &mut self,
        content: &SheetGrid,
        geometry: &GridGeometry,
        region: &PaneRegion,
        style: &CellStyle,
        text: &str,
        row: u32,
        column: u16,
        sheet_rect: LayoutRect,
        merge: Option<MergedRegion>,
    ) -> Result<PlacedText, SheetLayoutError> {
        let sheet_row = content.row(row);
        let occupied_right = sheet_row
            .as_ref()
            .and_then(|sheet_row| sheet_row.cell_after(column))
            .filter(|cell| content.cell_is_occupied(cell))
            .map(|cell| cell.reference().column());
        let occupied_left = sheet_row
            .as_ref()
            .and_then(|sheet_row| sheet_row.cell_before(column))
            .filter(|cell| content.cell_is_occupied(cell))
            .map(|cell| cell.reference().column());
        let center_continuous_span = (style.horizontal == HorizontalAlignment::CenterContinuous)
            .then(|| cell::center_continuous_span(column, occupied_left, occupied_right));

        let columns = geometry.columns();
        let digit = geometry.maximum_digit_width();
        let mut width_of = |column: u16| columns.width(column);
        let mut left_of = |column: u16| columns.left(column);
        let mut context = CellContext {
            rect: sheet_rect,
            column,
            digit,
            column_width: &mut width_of,
            column_left: &mut left_of,
            occupied_right,
            occupied_left,
            center_continuous_span,
            merged: merge.is_some(),
        };
        let Self {
            fonts,
            rasteriser,
            shaper,
            features,
            ..
        } = self;
        let mut engine = TextEngine {
            fonts,
            rasteriser,
            shaper,
            features,
        };
        let _ = region;
        Ok(cell::place(&mut engine, text, style, &mut context)?)
    }

    /// Emits the line and glyph-run fragments a placed cell produced.
    #[allow(clippy::too_many_arguments)]
    fn emit_text(
        &mut self,
        builder: &mut FragmentTreeBuilder,
        content: &SheetGrid,
        region: &PaneRegion,
        parent: Option<FragmentId>,
        clip: Option<ClipId>,
        placed: &PlacedText,
        row: u32,
        column: u16,
    ) {
        let transform = placed
            .transform
            .map(|transform| builder.transform(transform))
            .unwrap_or(TransformId::IDENTITY);
        let clip = placed
            .clip
            .and_then(|rect| builder.clip(translate(rect, region.origin)))
            .or(clip);
        for line in &placed.lines {
            let rect = translate(line.rect, region.origin);
            let Some(line_node) = builder.push(
                parent,
                address::span(
                    content.part(),
                    address::cell_path(row, column),
                    line.range.clone(),
                ),
                rect,
                transform,
                clip,
                Fragment::Line(LineFragment {
                    baseline: line.baseline,
                    ascent: line.ascent,
                    descent: line.descent,
                    base_direction: line.direction,
                    hanging_width: Emu::ZERO,
                }),
            ) else {
                return;
            };
            let mut pen = translate_point(line.origin, region.origin);
            for segment in &line.segments {
                let width = Emu::from_points(segment.width_in_points);
                let glyph_rect = LayoutRect::from_edges(
                    pen.x,
                    rect.top,
                    pen.x + width,
                    rect.top + rect.height(),
                );
                if builder
                    .push(
                        Some(line_node),
                        address::span(
                            content.part(),
                            address::cell_path(row, column),
                            segment.range.clone(),
                        ),
                        glyph_rect,
                        transform,
                        clip,
                        Fragment::GlyphRun(GlyphRunFragment {
                            face: segment.face,
                            run: segment.run.clone(),
                            origin: pen,
                            direction: segment.direction,
                            level: segment.level,
                        }),
                    )
                    .is_none()
                {
                    return;
                }
                pen = LayoutPoint {
                    x: pen.x + width,
                    y: pen.y,
                };
            }
        }
    }
}

impl SheetBoxModel {
    /// The page the last [`BoxModel::layout_page`] produced.
    ///
    /// [`BoxModel::invalidate`] is not given the constraints, so the only band it can name is this
    /// one; every other change answers [`DirtyPages::All`] rather than guessing.
    #[must_use]
    pub fn last_page(&self) -> PageIndex {
        self.last_page
    }
}

/// Moves a sheet-coordinate rectangle into page coordinates.
fn translate(rect: LayoutRect, origin: LayoutPoint) -> LayoutRect {
    LayoutRect::from_edges(
        rect.left + origin.x,
        rect.top + origin.y,
        rect.right + origin.x,
        rect.bottom + origin.y,
    )
}

/// Moves a sheet-coordinate point into page coordinates.
fn translate_point(point: LayoutPoint, origin: LayoutPoint) -> LayoutPoint {
    LayoutPoint {
        x: point.x + origin.x,
        y: point.y + origin.y,
    }
}

/// What fills a cell, as the file states it.
fn fill_of(
    resolver: &mjx_xlsx::SheetFormatResolver<'_>,
    format: &mjx_sml::EffectiveCellFormat,
    interner: &mjx_ooxml_core::Interner,
) -> Option<CellFill> {
    let fill = resolver.formats().fill(format)?;
    if let Some(pattern) = fill.pattern() {
        return Some(CellFill {
            pattern: pattern.pattern_type(interner).ok().flatten(),
            foreground: pattern.foreground_colour(interner),
            background: pattern.background_colour(interner),
            is_gradient: false,
        });
    }
    fill.gradient().map(|_| CellFill {
        pattern: None,
        foreground: None,
        background: None,
        is_gradient: true,
    })
}

/// A cell's four borders, as the file states them.
fn borders_of(
    resolver: &mjx_xlsx::SheetFormatResolver<'_>,
    format: &mjx_sml::EffectiveCellFormat,
    interner: &mjx_ooxml_core::Interner,
) -> CellBorders {
    let Some(border) = resolver.formats().border(format) else {
        return CellBorders::default();
    };
    CellBorders {
        left: edge_of(border.left_edge().or(border.leading_edge()), interner),
        right: edge_of(border.right_edge().or(border.trailing_edge()), interner),
        top: edge_of(border.top_edge(), interner),
        bottom: edge_of(border.bottom_edge(), interner),
    }
}

/// One border edge, or `None` when it states `style="none"` or nothing at all.
fn edge_of(
    edge: Option<&mjx_sml::BorderEdge>,
    interner: &mjx_ooxml_core::Interner,
) -> Option<BorderEdge> {
    let edge = edge?;
    let style = edge.style(interner).ok()?;
    if style == BorderStyle::None {
        return None;
    }
    Some(BorderEdge {
        style,
        colour: edge.colour(interner),
    })
}

/// The number-format code in force, for MJXOFF-172 to evaluate.
fn number_format_of(
    resolver: &mjx_xlsx::SheetFormatResolver<'_>,
    format: &mjx_sml::EffectiveCellFormat,
    interner: &mjx_ooxml_core::Interner,
) -> Option<(u32, String)> {
    let id = format.number_format().resource_index?;
    let _ = interner;
    let code = resolver.formats().format_code(format).ok()??;
    Some((id, code.into_owned()))
}

/// Resolves the four edges of a merged region's union.
///
/// See [`crate::merge`] for why this scans the perimeter rather than taking the anchor's edges
/// outright, and for the `GUESS` that reading carries.
fn resolve_merge_borders(
    decoration: &mut Decoration,
    resolver: &mjx_xlsx::SheetFormatResolver<'_>,
    interner: &mjx_ooxml_core::Interner,
    merge: MergedRegion,
    content: &SheetGrid,
) {
    for edge in RegionEdge::ALL {
        if decoration.borders.edge(edge).is_some() {
            continue;
        }
        for (row, column) in merge.edge_cells(edge) {
            let Ok(reference) = mjx_sml::CellReference::relative(column, row) else {
                continue;
            };
            let Ok(format) = resolver.effective_cell_format(reference) else {
                continue;
            };
            let stated = borders_of(resolver, &format, interner);
            if let Some(found) = stated.edge(edge) {
                decoration.borders.set(edge, Some(found.clone()));
                break;
            }
        }
    }
    let _ = content;
}

/// The [`LayoutError`] a zero-height or zero-width content area produces.
fn empty_area(constraints: &Constraints) -> LayoutError {
    LayoutError::EmptyContentArea {
        width: constraints.content.width().emu(),
        height: constraints.content.height().emu(),
    }
}
