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
    GeometryRef, GlyphRunFragment, LayoutError, LayoutPoint, LayoutRect, LineFragment,
    ModelSignature, PageFragments, PageIndex, TableCell, TableFragment, TransformId,
};
use mjx_layout_chart::{ChartOutline, ChartPaint, ChartResourceTable};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::spreadsheetml::{BorderStyle, GradientType, HorizontalAlignment, PatternType};
use mjx_sml::{Color, FontProperties};
use mjx_text::{FeatureSet, FontResolver, GlyphRasteriser, Shaper};

use crate::address;
use crate::autofit::{AutoFit, AutoFitCache};
use crate::border;
use crate::cell::{self, CellContext, CellStyle, PlacedText};
use crate::condfmt::{
    ConditionalEffect, ConditionalEngine, ConditionalSignature, DataBarGeometry, IconChoice,
    RuleValue, ScaleBlend, UnevaluatedRule,
};
use crate::drawings::PlacedDrawing;
use crate::error::SheetLayoutError;
use crate::geometry::{ColumnGeometry, GridGeometry, MaximumDigitWidth, COLUMN_COUNT};
use crate::merge::{MergedRegion, RegionEdge};
use crate::numfmt;
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
    /// The gradient, when the fill is one rather than a pattern.
    ///
    /// **R16 carried a bare `is_gradient: bool` here** on the reasoning that *"`x:gradientFill` is a
    /// resource the scene companion resolves"*. MJXOFF-244 found that the companion cannot: it is
    /// handed this catalogue and nothing else, and a boolean names no stops, so a gradient-filled
    /// cell had no way to reach a pixel at all. Carrying the stops is **not** the flattening that
    /// reasoning was guarding against — nothing here decides what the ramp looks like, it only
    /// records what the file wrote.
    pub gradient: Option<CellGradient>,
}

/// `x:gradientFill` — what a gradient-filled cell states, unresolved.
///
/// Every colour is still an [`mjx_sml::Color`]: a `@theme` position, an `@indexed` row and a
/// `@tint` are all still exactly what the file wrote, because resolving one needs
/// `xl/theme/theme1.xml` and the box model does not have it.
#[derive(Clone, PartialEq, Debug)]
pub struct CellGradient {
    /// `@type` — whether the ramp runs at an angle or converges on a rectangle.
    pub kind: GradientType,
    /// `@degree` — the angle of a linear ramp, in degrees clockwise.
    pub degrees: f64,
    /// `@left`, `@right`, `@top`, `@bottom` — the rectangle a `path` ramp converges on, as
    /// fractions of the cell. Unused for a linear ramp, and carried anyway, because the file wrote
    /// them.
    pub inset: [f64; 4],
    /// The stops, in the order the file wrote them — never sorted, for the reason
    /// [`mjx_sml::GradientFill::stops`] gives.
    pub stops: Vec<CellGradientStop>,
}

/// One `x:stop` of a [`CellGradient`].
#[derive(Clone, PartialEq, Debug)]
pub struct CellGradientStop {
    /// `@position`, in `0.0..=1.0`.
    pub position: f64,
    /// `x:color`, unresolved.
    pub colour: Option<Color>,
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
    /// **Applied since MJXOFF-172**, by [`crate::numfmt`], which is what turns `45719` into
    /// `04/03/2025`. It is still carried here because a caller that wants to know *why* a cell reads
    /// the way it does has no other way to ask, and because a re-layout at a different zoom must not
    /// have to resolve the `xf` ladder a second time to find it.
    pub number_format: Option<(u32, String)>,
    /// `[Red]`, `[Color12]` — the **zero-based** row of the legacy indexed palette this cell's text
    /// is drawn in, or `None` when its format names no colour.
    ///
    /// **This is why a coloured cell does not share its neighbour's decoration.** A number format
    /// states its colour per *section*, and which section runs depends on the value — so the
    /// negative cells of a `#,##0;[Red]#,##0` column are red and the positive ones are not, with one
    /// effective format between them. [`PageCatalogue`]'s sharing table keys on the pair for that
    /// reason.
    ///
    /// A painter draws the text in this row of `indexedColors` **instead of** in
    /// [`Decoration::font`]'s own colour; `mjx-scene-xlsx` is where that happens.
    pub text_colour: Option<u32>,
    /// The colour a **colour-scale** conditional format interpolated for this cell, unresolved.
    ///
    /// Two stops and a position between them rather than one colour, because blending two
    /// `CT_Color`s needs the theme part and the workbook's `indexedColors` and a box model holds
    /// neither — the same division that leaves [`Decoration::text_colour`] a bare palette row. See
    /// [`crate::condfmt::graded`] for why doing the blend here would have been silently wrong for
    /// every workbook whose scale is themed.
    ///
    /// A painter draws this **instead of** [`Decoration::fill`] when both are present: a colour
    /// scale replaces the cell's background rather than tinting it.
    pub scale_fill: Option<ScaleBlend>,
    /// The bar a **data-bar** conditional format drew in this cell, when this decoration belongs to
    /// one.
    ///
    /// A bar is a rectangle of its own inside the cell — see [`crate::model::SheetBoxModel`]'s band
    /// walk — so this decoration carries a solid fill and this field says what it is. Carried so a
    /// report can name the fraction without re-evaluating the rule.
    pub data_bar: Option<DataBarGeometry>,
    /// The edge this decoration draws, when it belongs to a **border band** rather than to a cell.
    ///
    /// A band is a box the width of one border line, filled with that line's colour — see
    /// [`crate::border`] for why an edge cannot be a stroke on the cell's own decoration. The two
    /// states are exclusive: a decoration with a band draws nothing else, and a cell's decoration
    /// never carries one.
    ///
    /// The whole [`BorderEdge`] is here rather than just its colour so that the layer above has the
    /// style a filled band cannot draw. See [`crate::border`] for what that costs today.
    pub border_band: Option<BorderEdge>,
}

/// What two cells must agree on before they share one decoration handle: their effective format,
/// the colour their number format gave *this value*, and what conditional formatting made of them.
///
/// A named type rather than a tuple in a `Vec`, because the tuple is now three deep and a reader
/// meeting it in a field declaration has no way to tell which member is which.
type SharedDecorationKey = (
    mjx_sml::EffectiveCellFormat,
    Option<u32>,
    ConditionalSignature,
);

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
    /// The handle its own box carries.
    ///
    /// **Added by MJXOFF-244, and the reason is a seam the fragment vocabulary leaves open.**
    /// `mjx_scene::ResourceResolver::text_decoration` is addressed by a [`SourceRef`] rather than by
    /// a handle, because a `GlyphRunFragment` carries no decoration of its own — so a companion that
    /// wants a cell's *font colour* has to turn `(row, column)` back into a handle. Every cell that
    /// produces a glyph run produces one of these reports, so this table is exactly the map that
    /// question needs, and building it costs a `Copy` field rather than a second walk of the tree.
    ///
    /// [`SourceRef`]: mjx_layout::SourceRef
    pub decoration: DecorationRef,
    /// The characters the cell displayed — **after** its number format ran.
    ///
    /// Added by MJXOFF-172, and the reason is the same seam [`CellReport::decoration`] names one
    /// paragraph up. A `GlyphRunFragment` carries a shaped run and a byte range; it does not carry
    /// the string those bytes index, and nothing else in the tree does either. So a layer that wants
    /// to know *what a cell says* — a hit test reporting a selection, an accessibility tree, an
    /// exporter, or a suite asserting that a date rendered as a date rather than as `45719` — has
    /// nowhere to ask.
    ///
    /// It costs nothing to carry: the box model built this string to lay the cell out, and this
    /// moves it rather than copying it. Empty for a cell that produced no glyphs.
    pub text: String,
    /// What conditional formatting made of the cell — **including the rules that fired and changed
    /// nothing, and the rules that could not be answered**.
    ///
    /// `None` when no rule reaches this position at all, which is the state of almost every cell in
    /// almost every workbook. See [`crate::condfmt`] for why this is a report rather than only an
    /// appearance: a rule that never fires and a rule that is not implemented render identically,
    /// so the only way to tell them apart is to say which fired.
    pub conditional: Option<ConditionalEffect>,
    /// The icon an icon-set rule chose for this cell, and how many icons its set holds.
    ///
    /// **Reported and not drawn.** An icon set's artwork is Excel's — eighteen sets of three to
    /// five glyphs, none of them in any specification and none of them in this repository — and
    /// drawing a stand-in would be inventing a picture and presenting it as the file's. The index
    /// is asserted by a gate; the pixels wait for artwork somebody is entitled to ship.
    pub icon: Option<IconChoice>,
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
    ///
    /// The third member of the key is the **conditional signature**: a `dxf` layer is a pure
    /// function of which rules fired, so two cells that fired the same rules share, and a cell that
    /// fired none carries the default signature and shares exactly as it did before MJXOFF-173.
    by_format: Vec<(SharedDecorationKey, DecorationRef)>,
    /// Which handle each already-issued **border band** resolved to.
    ///
    /// A worksheet's borders repeat far harder than its formats do — a bordered block of a hundred
    /// cells states the same `thin` black edge four hundred times — so a band is interned by the
    /// [`BorderEdge`] it draws and a whole table costs a handful of handles.
    by_band: Vec<(BorderEdge, DecorationRef)>,
    cells: Vec<CellReport>,
    regions: Vec<PaneRegion>,
    rows: Vec<u32>,
    columns: Vec<u16>,
    fits: Vec<(u16, AutoFit)>,
    unevaluated: Vec<(u32, u16, UnevaluatedRule)>,
    drawings: Vec<PlacedDrawing>,
    /// The paints and outlines a chart anchored on the sheet issued (MJXOFF-178).
    ///
    /// A separate table rather than more entries in [`Self::decorations`], because a chart's outline
    /// is not a cell's: a pie slice is an arc and a gridline is a segment, and this crate's own
    /// `Decoration` is a fill, a border band and a font colour. The two spaces are kept apart by
    /// numbering rather than by type — see [`PageCatalogue::CHART_HANDLE_BASE`].
    charts: ChartResourceTable,
}

impl PageCatalogue {
    /// Where a chart's own handles are numbered from.
    ///
    /// A chart's paints and outlines are resolved by a different table from a cell's, so they are
    /// numbered in a space of their own rather than interleaved. `1 << 32` is above every handle a
    /// band of cells can issue — a `FragmentId` is a `u32`, so a page holds at most four billion
    /// fragments and therefore at most that many handles — which makes
    /// `handle.number() >= CHART_HANDLE_BASE` the test that says which table resolves it.
    ///
    /// **All three box models use the same base**, and `mjx-layout-pptx`'s constant of the same name
    /// is the same number for the same reason: `xtask/tests/one_engine_three_formats.rs` compares
    /// fragment trees, and a handle numbered from the host's own running total would differ between
    /// hosts for reasons that have nothing to do with the chart.
    pub const CHART_HANDLE_BASE: u64 = 1 << 32;

    /// A fresh catalogue, with the chart table numbered from [`Self::CHART_HANDLE_BASE`].
    #[must_use]
    pub fn new() -> Self {
        Self {
            charts: ChartResourceTable::new(Self::CHART_HANDLE_BASE, Self::CHART_HANDLE_BASE),
            ..Self::default()
        }
    }

    /// The table a chart on this page issues its handles from.
    pub fn chart_resources(&mut self) -> &mut ChartResourceTable {
        &mut self.charts
    }

    /// What a chart's paint handle resolves to, or `None` for a handle no chart issued.
    #[must_use]
    pub fn chart_paint(&self, handle: DecorationRef) -> Option<&ChartPaint> {
        self.charts.paint(handle)
    }

    /// What a chart's outline handle resolves to.
    #[must_use]
    pub fn chart_outline(&self, handle: GeometryRef) -> Option<&ChartOutline> {
        self.charts.shape(handle)
    }

    /// Whether `number` names a handle a chart issued rather than one a cell did.
    #[must_use]
    pub fn is_chart_handle(number: u64) -> bool {
        number >= Self::CHART_HANDLE_BASE
    }

    /// What a decoration handle resolves to. `None` for a chart's — see [`Self::chart_paint`].
    #[must_use]
    pub fn decoration(&self, handle: DecorationRef) -> Option<&Decoration> {
        if Self::is_chart_handle(handle.number()) {
            return None;
        }
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
    ///
    /// Keyed on the format **and** the number format's own colour, because the second is a function
    /// of the cell's value rather than of its style: see [`Decoration::text_colour`].
    fn handle_for(
        &self,
        format: &mjx_sml::EffectiveCellFormat,
        colour: Option<u32>,
        conditional: &ConditionalSignature,
    ) -> Option<DecorationRef> {
        self.by_format
            .iter()
            .find(|((candidate, tinted, applied), _)| {
                candidate == format && *tinted == colour && applied == conditional
            })
            .map(|(_, handle)| *handle)
    }

    /// Records `decoration` and answers its handle, remembering `format` when one may be shared.
    fn intern(
        &mut self,
        decoration: Decoration,
        format: Option<SharedDecorationKey>,
    ) -> DecorationRef {
        let handle = DecorationRef::new(u64::try_from(self.decorations.len()).unwrap_or(0));
        self.decorations.push(decoration);
        if let Some(format) = format {
            self.by_format.push((format, handle));
        }
        handle
    }

    /// Records a cell that produced **no glyphs** but did carry a conditional format.
    ///
    /// Three cells reach here and each of them matters: one holding nothing at all that a
    /// `containsBlanks` rule painted, one whose format renders its value as nothing (`;;;`), and one
    /// outside every rule's range that carries neither. The first two must appear in
    /// [`PageCatalogue::cells`] or a gate has no way to ask which rule fired on them; the third is
    /// dropped, because reporting every empty cell of a band would make the report the size of the
    /// window.
    fn push_bare(
        &mut self,
        row: u32,
        column: u16,
        merge: Option<MergedRegion>,
        decoration: DecorationRef,
        conditional: Option<ConditionalEffect>,
        icon: Option<IconChoice>,
    ) {
        if conditional.is_none() {
            return;
        }
        self.cells.push(CellReport {
            row,
            column,
            overflow: Overflow::Fits,
            shrink_scale: 1.0,
            merge,
            decoration,
            text: String::new(),
            conditional,
            icon,
        });
    }

    /// The handle a border band drawing `stated` resolves to, issuing one on first sight.
    fn band_handle(&mut self, stated: &BorderEdge) -> DecorationRef {
        if let Some((_, handle)) = self
            .by_band
            .iter()
            .find(|(candidate, _)| candidate == stated)
        {
            return *handle;
        }
        let handle = self.intern(
            Decoration {
                border_band: Some(stated.clone()),
                ..Decoration::default()
            },
            None,
        );
        self.by_band.push((stated.clone(), handle));
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

    /// Every drawing anchored on this band, in paint order.
    ///
    /// A drawing's *placement* — three anchor modes resolved against this crate's own row heights
    /// and column widths — and not its content: see [`crate::drawings`] for why a box model at rank
    /// 3.6 cannot lay out the DrawingML shapes inside one.
    #[must_use]
    pub fn drawings(&self) -> &[PlacedDrawing] {
        &self.drawings
    }

    /// Every rule that could not be answered, and where.
    ///
    /// The ticket's `partial` ledger, computed rather than written down: an `expression` rule's
    /// condition is a formula and this workspace has no calculation engine, so the rule is neither
    /// faked nor dropped — it is listed here with its reason and its text.
    #[must_use]
    pub fn unevaluated_rules(&self) -> &[(u32, u16, UnevaluatedRule)] {
        &self.unevaluated
    }
}

/// One border band waiting to be emitted.
///
/// Bands are collected while the cells of a region are laid out and pushed **after all of them**,
/// so that every fill is behind every border. Emitting a band as a child of its own cell would put
/// the next cell's fill on top of it, which is how a bordered grid loses every internal line it
/// shares with the cell to its right.
struct PendingBand {
    row: u32,
    column: u16,
    rect: LayoutRect,
    decoration: DecorationRef,
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
    formats: numfmt::FormatCache,
    conditional: ConditionalEngine,
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
            catalogue: PageCatalogue::new(),
            geometry: None,
            fits: AutoFitCache::new(),
            formats: numfmt::FormatCache::new(),
            conditional: ConditionalEngine::new(),
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

    /// The number-format caches, and the counters that say whether they hit.
    ///
    /// Held across pages on purpose: a band of a sheet shares its format codes with every other
    /// band of the same sheet, so a scroll that rebuilt the table would pay for the parse again on
    /// every frame. See [`numfmt::FormatCache`] for what is cached and what deliberately is not.
    #[must_use]
    pub fn formats(&self) -> &numfmt::FormatCache {
        &self.formats
    }

    /// Empties the number-format caches.
    ///
    /// What a caller that has swapped documents wants; a caller that is scrolling one document
    /// wants the opposite, which is why nothing does this on its own.
    pub fn clear_format_cache(&mut self) {
        self.formats.clear();
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
        // Every conditional-formatting statistic is a fact about cell *values* — a minimum, a mean,
        // a multiset for `duplicateValues` — so an edit anywhere invalidates all of them. The
        // number-format cache is not cleared here for the opposite reason: it is keyed on a code and
        // a value, and neither changes meaning because a cell did.
        self.conditional.clear();
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
            ..PageCatalogue::new()
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

        // Drawings, after every cell of every region: an anchored object floats **over** the grid,
        // and a drawing pushed before the cells would be painted under them.
        self.lay_out_drawings(
            &mut builder,
            &mut catalogue,
            content,
            geometry,
            &regions,
            root,
        );

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
        let mut bands: Vec<PendingBand> = Vec::new();
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
                    builder, catalogue, &mut bands, content, geometry, &resolver, interner, region,
                    table, clip, row, column, merge,
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
                &mut bands,
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
        // Every border of the region, after every fill of it. See [`PendingBand`].
        for band in bands {
            builder.push(
                table,
                address::node(content.part(), address::cell_path(band.row, band.column)),
                band.rect,
                TransformId::IDENTITY,
                clip,
                Fragment::Box(BoxFragment {
                    decoration: Some(band.decoration),
                    // A band is not a cell: a `TableCell` here would make a grid of bordered cells
                    // report five times as many cells as it has.
                    cell: None,
                }),
            );
        }
        Ok(())
    }

    /// Places every anchored object that reaches this band, and emits a box for each.
    ///
    /// The **content** of a drawing is not laid out here and cannot be — see [`crate::drawings`] —
    /// so what reaches the tree is a box carrying the object's rectangle and its source address. A
    /// hit test lands on it, an exporter can find it, and a painter draws nothing inside it, which
    /// is the honest picture of what this build knows about a picture on a sheet.
    fn lay_out_drawings(
        &self,
        builder: &mut FragmentTreeBuilder,
        catalogue: &mut PageCatalogue,
        content: &SheetGrid,
        geometry: &GridGeometry,
        regions: &[PaneRegion],
        parent: Option<FragmentId>,
    ) {
        let Some(drawing) = content.drawing() else {
            return;
        };
        // A drawing belongs to the **scrolling** region: it is anchored to cells, and a frozen pane
        // shows the cells it is anchored to only when they are frozen too. GUESS: that an object
        // anchored inside a frozen pane is pinned with it; Excel does pin one, and picking the last
        // region — which `panes::regions` orders as the scrolling one — is the reading that keeps
        // an ordinary unfrozen sheet exactly right.
        let Some(region) = regions.last() else {
            return;
        };
        for placed in crate::drawings::place(drawing, geometry) {
            let rect = translate(placed.rect, region.origin);
            if !rect.intersects(region.view) {
                continue;
            }
            let clip = builder.clip(region.view);
            let address = address::node(content.part(), address::drawing_path(placed.index));
            let node = builder.push(
                parent,
                address.clone(),
                rect,
                TransformId::IDENTITY,
                clip,
                Fragment::Box(BoxFragment {
                    decoration: None,
                    cell: None,
                }),
            );
            // A chart's interior. The anchor is this crate's; everything inside it is
            // `mjx-layout-chart`'s, reached through one call that PowerPoint's and Word's box models
            // make identically — which is what MJXOFF-178's rank 3.55 buys.
            if let (Some(node), Some(chart)) = (node, content.chart(placed.index)) {
                let geometry = mjx_layout_chart::lay_out(
                    chart,
                    rect,
                    content.palette(),
                    &mut mjx_layout_chart::NominalMetrics,
                );
                mjx_layout_chart::emit_into(
                    &geometry,
                    builder,
                    node,
                    &mjx_layout_chart::ChartAddress::new(address),
                    catalogue.chart_resources(),
                );
            }
            catalogue.drawings.push(placed);
        }
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
        bands: &mut Vec<PendingBand>,
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
        // ⚠ The formatted text is resolved **before** the decoration, and that order is the whole
        // reason MJXOFF-172 touched this function. A number format states its colour per *section*
        // — `#,##0;[Red]#,##0` colours the negative cells and not the positive ones — so which
        // colour a cell's text takes depends on its **value**, and a decoration shared by an
        // effective format cannot carry it unless the value has already been read.
        let code = match content.number_format_language() {
            // §18.8.30's locale-dependent ids need a UI language and nothing in the file states one;
            // see `SheetGrid::with_number_format_language`.
            Some(language) => resolver
                .formats()
                .format_code_in(&format, language)
                .ok()
                .flatten(),
            None => resolver.formats().format_code(&format).ok().flatten(),
        };
        let display = cell.as_ref().and_then(|cell| {
            let raw = content.cell_text(cell)?;
            Some(self.formats.format(
                code.as_deref(),
                numfmt::CellValue::read(cell.cell_type(), &raw),
                content.date_system(),
            ))
        });
        let text_colour = display.as_ref().and_then(|value| value.colour);

        // ⚠ Conditional formatting is evaluated here, and it has to be **after** the value has been
        // read and **before** the decoration is chosen. `mjx-sml` reports which rules apply to a
        // cell and what each would impose, and stops there on purpose; this is the consumer that
        // decides whether they hold, and the answer depends on the number in the cell.
        let conditional =
            self.evaluate_conditional(content, resolver, interner, row, column, &cell);
        let signature = conditional
            .as_ref()
            .map(ConditionalEffect::signature)
            .unwrap_or_default();
        let icon = conditional.as_ref().and_then(|effect| effect.icon);
        if let Some(effect) = conditional.as_ref() {
            for rule in &effect.unevaluated {
                catalogue.unevaluated.push((row, column, rule.clone()));
            }
        }

        let shared = merge
            .is_none()
            .then(|| catalogue.handle_for(&format, text_colour, &signature))
            .flatten();
        let handle = match shared {
            Some(handle) => handle,
            None => {
                let mut decoration = Decoration {
                    fill: fill_of(resolver, &format, interner),
                    borders: borders_of(resolver, &format, interner),
                    font: font.clone(),
                    number_format: number_format_of(&format, code.as_deref()),
                    text_colour,
                    scale_fill: None,
                    data_bar: None,
                    border_band: None,
                };
                if let Some(merge) = merge {
                    resolve_merge_borders(&mut decoration, resolver, interner, merge, content);
                }
                if let Some(effect) = conditional.as_ref() {
                    apply_conditional(&mut decoration, effect);
                }
                catalogue.intern(
                    decoration,
                    merge
                        .is_none()
                        .then(|| (format, text_colour, signature.clone())),
                )
            }
        };

        // The bands this cell's own edges draw, held back until every cell of the region has
        // been laid out. The borders are read off the interned decoration rather than off the
        // literal above, because a shared handle answers for a cell that never built one.
        if let Some(borders) = catalogue
            .decoration(handle)
            .map(|decoration| decoration.borders.clone())
        {
            for band in border::bands(rect, &borders) {
                let decoration = catalogue.band_handle(&band.stated);
                bands.push(PendingBand {
                    row,
                    column,
                    rect: band.rect,
                    decoration,
                });
            }
        }

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

        // The bar is a child of the cell's own box and is pushed **before** the text, so a
        // `showValue` bar has its number drawn over it rather than under it. It is a fragment
        // rather than a property of the cell's decoration because its width is a function of the
        // value and the cell's decoration is shared by every cell of one effective format.
        if let Some(bar) = conditional.as_ref().and_then(|effect| effect.bar.clone()) {
            #[allow(clippy::cast_precision_loss)]
            let width =
                Emu::from_emu_rounded(rect.width().emu() as f64 * bar.fraction.clamp(0.0, 1.0));
            if width > Emu::ZERO {
                let bar_rect =
                    LayoutRect::from_edges(rect.left, rect.top, rect.left + width, rect.bottom);
                let decoration = catalogue.intern(
                    Decoration {
                        // GUESS: a solid fill of the bar's own colour, filling the cell's whole
                        // height and growing rightward from its left edge. `CT_DataBar` states no
                        // axis, no border, no gradient and no negative fill — all four are
                        // `x14:dataBar`, in the `extLst` this workspace preserves and does not
                        // model — so this is Excel 2007's bar and not Excel 2010's.
                        fill: Some(CellFill {
                            pattern: Some(PatternType::Solid),
                            foreground: bar.colour.clone(),
                            background: None,
                            gradient: None,
                        }),
                        data_bar: Some(bar),
                        ..Decoration::default()
                    },
                    None,
                );
                builder.push(
                    cell_node,
                    address::node(content.part(), address::cell_path(row, column)),
                    bar_rect,
                    TransformId::IDENTITY,
                    clip,
                    Fragment::Box(BoxFragment {
                        decoration: Some(decoration),
                        // A bar is not a cell: a `TableCell` here would double every count.
                        cell: None,
                    }),
                );
            }
        }

        let Some(cell) = cell else {
            catalogue.push_bare(row, column, merge, handle, conditional, icon);
            return Ok(());
        };
        let Some(display) = display else {
            catalogue.push_bare(row, column, merge, handle, conditional, icon);
            return Ok(());
        };
        // A format may render a value as nothing at all — `;;;` is how a person hides a column
        // without hiding it — and a cell with no glyphs produces no fragments.
        if display.text.is_empty() {
            catalogue.push_bare(row, column, merge, handle, conditional, icon);
            return Ok(());
        }
        let text = display.text;

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
            decoration: handle,
            text,
            conditional,
            icon,
        });
        self.emit_text(
            builder, content, region, cell_node, clip, &placed, row, column,
        );
        Ok(())
    }

    /// Evaluates every conditional-formatting rule that reaches one cell.
    ///
    /// `None` when no rule's `@sqref` covers the position, which is one rectangle test against the
    /// union of every block — so a sheet with no conditional formatting, and a screen away from the
    /// block that has it, pay nothing per cell.
    fn evaluate_conditional(
        &mut self,
        content: &SheetGrid,
        resolver: &mjx_xlsx::SheetFormatResolver<'_>,
        interner: &mjx_ooxml_core::Interner,
        row: u32,
        column: u16,
        cell: &Option<mjx_sml::Cell<'_>>,
    ) -> Option<ConditionalEffect> {
        self.conditional.prepare(content);
        if !self.conditional.may_cover(row, column) {
            return None;
        }
        let text = cell
            .as_ref()
            .and_then(|cell| content.cell_text(cell))
            .unwrap_or_default();
        let value = cell.as_ref().map_or(RuleValue::Blank, |cell| {
            RuleValue::read(cell.cell_type(), &text)
        });
        let effect = self
            .conditional
            .evaluate(content, row, column, &value, interner, |index| {
                resolver.formats().differential_format(index)
            });
        (!effect.is_empty()).then_some(effect)
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
            gradient: None,
        });
    }
    fill.gradient().map(|gradient| CellFill {
        pattern: None,
        foreground: None,
        background: None,
        gradient: Some(gradient_from(gradient, interner)),
    })
}

/// A `x:gradientFill` as the catalogue carries it: the file's own numbers, resolved not at all.
pub(crate) fn gradient_from(
    gradient: &mjx_sml::GradientFill,
    interner: &mjx_ooxml_core::Interner,
) -> CellGradient {
    CellGradient {
        kind: gradient
            .gradient_type(interner)
            .unwrap_or(GradientType::Linear),
        degrees: gradient.degrees(interner).unwrap_or(0.0),
        inset: [
            gradient.left_inset(interner).unwrap_or(0.0),
            gradient.right_inset(interner).unwrap_or(0.0),
            gradient.top_inset(interner).unwrap_or(0.0),
            gradient.bottom_inset(interner).unwrap_or(0.0),
        ],
        stops: gradient
            .stops()
            .map(|stop| CellGradientStop {
                // `@position` is `use="required"`; a stop that omits it is read as the start of the
                // ramp rather than dropped, which is what `mjx-sml` reading it as an `Option`
                // leaves to whoever consumes it.
                position: stop.position(interner).ok().flatten().unwrap_or(0.0),
                colour: stop.colour(interner),
            })
            .collect(),
    }
}

/// Folds a conditional format's `dxf` layer onto a cell's own decoration.
///
/// **The `dxf` wins, member by member**, and only where it states one: §18.8.15 calls it a format
/// *"to be applied on top of or in addition to any formatting already present"*, and every one of
/// its children is `minOccurs="0"` precisely so that an absent one means *leave what is there*.
///
/// `scale_fill` is set rather than merged: a colour scale replaces the cell's background outright.
fn apply_conditional(decoration: &mut Decoration, effect: &ConditionalEffect) {
    if let Some(fill) = effect.fill.clone() {
        decoration.fill = Some(fill);
    }
    if let Some(font) = effect.font.clone() {
        // ⚠ A `dxf` font colour outranks a **number format's** colour, and clearing `text_colour`
        // is how that is said. `mjx-scene-xlsx` prefers `text_colour` over the font's own — because
        // `[Red]` is a statement about *this value* where `x:font/color` is a statement about the
        // cell's style — so a conditional format that says *dark red* would otherwise lose to a
        // `#,##0;[Red]#,##0` on the same cell.
        //
        // GUESS: that the conditional format wins. It is the reading that makes a highlight rule do
        // what a person who wrote it expects; ECMA-376 states no precedence between the two,
        // because §18.8.30 and §18.8.15 do not know about each other.
        if font.color.is_some() {
            decoration.text_colour = None;
        }
        decoration.font = Some(match decoration.font.take() {
            None => font,
            Some(base) => crate::condfmt::merge_fonts(font, base),
        });
    }
    if let Some(borders) = effect.borders.as_ref() {
        for edge in RegionEdge::ALL {
            if let Some(stated) = borders.edge(edge) {
                decoration.borders.set(edge, Some(stated.clone()));
            }
        }
    }
    decoration.scale_fill = effect.scale.clone();
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
    borders_from(border, interner)
}

/// The four edges of one `x:border`, however it was reached.
///
/// Split out of [`borders_of`] because a `dxf` carries a `CT_Border` of its own and reading it a
/// second way would be two answers to one question — the `leading`/`trailing` fallback included,
/// which is the half a second copy would forget.
pub(crate) fn borders_from(
    border: &mjx_sml::Border,
    interner: &mjx_ooxml_core::Interner,
) -> CellBorders {
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

/// The number-format code in force, recorded on the decoration.
///
/// The code itself was already resolved by the caller — it has to be, because the text is formatted
/// before the decoration is chosen — so this only pairs it with the id it came from.
fn number_format_of(
    format: &mjx_sml::EffectiveCellFormat,
    code: Option<&str>,
) -> Option<(u32, String)> {
    let id = format.number_format().resource_index?;
    Some((id, code?.to_owned()))
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
