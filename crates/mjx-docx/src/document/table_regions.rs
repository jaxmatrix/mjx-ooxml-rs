//! Table-style conditional-formatting **region resolution** — which of a table style's regions
//! apply to a given cell, and the three `effective_cell_*` readers that fold them into the ladder
//! MJXOFF-106 built, named for their `mjx_pptx::presentation::effective` counterparts
//! (`effective_cell_fill`, `effective_cell_border`, `effective_cell_run_properties`).
//!
//! # `ConditionalFormatRegion` — never bare `Cnf`
//!
//! `ST_TblStyleOverrideType` (the twelve regions a `w:tblStylePr/@type` names) is exactly the
//! vocabulary a cell's applicable regions are drawn from, so [`ConditionalFormatRegion`] reuses the
//! generated [`TableStyleOverrideType`] rather than restating it under a second name — the naming
//! instruction ("never bare `Cnf`") is about not naming the *bitmask* `Cnf`, not about avoiding this
//! reuse.
//!
//! # `w:tblLook`/`w:cnfStyle`'s `val` is never the authority — the named attributes are
//!
//! An earlier dispatch note for this ticket assumed `w:cnfStyle`'s `val` (`ST_Cnf`, a fixed
//! twelve-character `[01]*` bitmask) carried a bit-position-to-region mapping to establish from the
//! prose. **It does not.** ECMA-376 Part 1 §17.3.1.8 (`cnfStyle`, paragraph), §17.4.7/§17.4.8 (the
//! row/cell variants) and §17.4.55 (`tblLook`) each document only the *named* `ST_OnOff` attributes
//! (`firstRow`, `oddHBand`, …) — `val` is never mentioned in any of the four sections' prose, has no
//! stated bit order, and every worked example in Part 1 writes the named attributes directly
//! (`<w:cnfStyle w:firstRow="true" w:lastColumn="true" .../>`). `wml.xsd`'s Transitional schema
//! carries `val` alongside the named attributes purely as a legacy artifact (the Strict schema for
//! `CT_Cnf` does not even declare it — see `table_properties.rs`'s own doc comment). This module
//! therefore reads region membership exclusively from [`TableLook`]'s six named flags and, for a
//! direct per-cell/per-row override, [`ConditionalFormatting`]'s twelve named flags — never from a
//! bitmask position.
//!
//! # The precedence, verified against ECMA-376 Part 1 §17.7.6.6 itself
//!
//! > When specified, these conditional formats shall be applied in the following order (therefore
//! > subsequent formats override properties on previous formats): Whole table; Banded columns, even
//! > column banding; Banded rows, even row banding; First row, last row; First column, last column;
//! > Top left, top right, bottom left, bottom right.
//!
//! [`applicable_regions`] returns regions in exactly this **application order** — a caller folds
//! them left to right, each later region's stated properties overriding the earlier ones', which is
//! also `mjx_dml::table::style::applicable_parts`'s own precedence (PowerPoint's analogous table-part
//! resolver), reversed: that function returns most-specific-first for a "first `Some` wins" walk,
//! this one returns least-specific-first for a "later `Some` wins" fold — the same ordering, read in
//! the direction each caller's own algorithm needs. Two consequences worth restating because they are
//! easy to get backwards: **column edges beat row edges** (first/last column comes after first/last
//! row in this list), and **row banding beats column banding** (banded rows comes after banded
//! columns).

use mjx_ooxml_core::{AttributeError, FromXml, Interner};
pub use mjx_ooxml_types::wordprocessingml::TableStyleOverrideType as ConditionalFormatRegion;

use crate::error::DocxError;

use super::effective::{
    attr, extract_border, extract_run_properties, extract_shading, merge_character_chain,
    recombine_toggles, ChainCache, EffectiveBorder, EffectiveCharacterProperties, EffectiveShading,
    ThemeContext,
};
use super::run_properties::RunProperties;
use super::styles::{StyleDefinition, StyleIndex, TableStyleOverride};
use super::table_properties::{CellBorders, TableBorders, TableLook, TableProperties};
use super::tables::CellProperties;
use super::{Document, MainDocument};

/// `w:tblLook`'s six flags, resolved: an unstated flag reads `false`, and a table with no `w:tblLook`
/// at all reads as [`TableLookFlags::default`] — ECMA-376 Part 1 §17.4.55's own stated default (row
/// and column banding on, every edge off).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableLookFlags {
    /// `@firstRow`.
    pub first_row: bool,
    /// `@lastRow`.
    pub last_row: bool,
    /// `@firstColumn`.
    pub first_column: bool,
    /// `@lastColumn`.
    pub last_column: bool,
    /// `@noHBand`.
    pub no_horizontal_band: bool,
    /// `@noVBand`.
    pub no_vertical_band: bool,
}

impl TableLookFlags {
    /// The flags `look` states, defaulted; [`TableLookFlags::default`] when `look` is `None`.
    ///
    /// # Errors
    /// An [`AttributeError`] if one of the six flags is present but not a valid `ST_OnOff`.
    pub fn from_look(
        look: Option<&TableLook>,
        interner: &Interner,
    ) -> Result<Self, AttributeError> {
        let Some(look) = look else {
            return Ok(Self::default());
        };
        Ok(Self {
            first_row: look.first_row(interner)?.unwrap_or(false),
            last_row: look.last_row(interner)?.unwrap_or(false),
            first_column: look.first_column(interner)?.unwrap_or(false),
            last_column: look.last_column(interner)?.unwrap_or(false),
            no_horizontal_band: look.no_horizontal_band(interner)?.unwrap_or(false),
            no_vertical_band: look.no_vertical_band(interner)?.unwrap_or(false),
        })
    }
}

/// The regions covering the cell at `(row, column)` of a `rows`×`columns` table, in **application
/// order** (later overrides earlier) — see this module's own doc comment for the ECMA-376 citation
/// and the two easy-to-reverse consequences (column beats row, row-band beats column-band).
///
/// Band membership: the `band_size`th data row/column starts a new band, alternating
/// [`ConditionalFormatRegion::Band1Horizontal`]/[`ConditionalFormatRegion::Band2Horizontal`]
/// (respectively `*Vertical`) —
/// ECMA-376 Part 1 §17.7.6.5/§17.7.6.7's own examples (`tblStyleColBandSize w:val="2"` bands columns
/// 1–2, 3–4, …). A first/last row or column, when `look` flags it, is excluded from banding
/// entirely, never counted as a data row/column.
#[must_use]
pub fn applicable_regions(
    row: usize,
    column: usize,
    rows: usize,
    columns: usize,
    look: TableLookFlags,
    row_band_size: usize,
    column_band_size: usize,
) -> Vec<ConditionalFormatRegion> {
    use ConditionalFormatRegion as R;

    let is_first_row = look.first_row && row == 0;
    let is_last_row = look.last_row && rows > 0 && row + 1 == rows;
    let is_first_col = look.first_column && column == 0;
    let is_last_col = look.last_column && columns > 0 && column + 1 == columns;

    let mut regions = vec![R::WholeTable];

    if !look.no_vertical_band && !is_first_col && !is_last_col {
        let data_column = column - usize::from(look.first_column);
        let band_size = column_band_size.max(1);
        regions.push(if (data_column / band_size).is_multiple_of(2) {
            R::Band1Vertical
        } else {
            R::Band2Vertical
        });
    }
    if !look.no_horizontal_band && !is_first_row && !is_last_row {
        let data_row = row - usize::from(look.first_row);
        let band_size = row_band_size.max(1);
        regions.push(if (data_row / band_size).is_multiple_of(2) {
            R::Band1Horizontal
        } else {
            R::Band2Horizontal
        });
    }
    if is_first_row {
        regions.push(R::FirstRow);
    }
    if is_last_row {
        regions.push(R::LastRow);
    }
    if is_first_col {
        regions.push(R::FirstColumn);
    }
    if is_last_col {
        regions.push(R::LastColumn);
    }
    // Corners — most specific, applied last. At most one applies (a table with a single row and
    // column is both first and last on both axes at once; §17.7.6.6's own diagram shows the four
    // corners as mutually exclusive quadrants, so the first match — top edge before bottom, left
    // before right — is the only one that can legitimately fire here since `is_first_row`/
    // `is_last_row` are themselves mutually exclusive except in that degenerate one-row/one-column
    // case, where "top" naturally wins).
    if is_first_row && is_first_col {
        regions.push(R::TopLeftCell);
    } else if is_first_row && is_last_col {
        regions.push(R::TopRightCell);
    } else if is_last_row && is_first_col {
        regions.push(R::BottomLeftCell);
    } else if is_last_row && is_last_col {
        regions.push(R::BottomRightCell);
    }
    regions
}

/// The table style's own base contribution plus every applicable region's, root-to-leaf across
/// `chain` — the generic fold both [`fill_tier`] and [`border_tier`] share, parameterized only by
/// how to extract one `Option<T>` contribution from a [`StyleDefinition`] or a [`TableStyleOverride`].
/// Later assignments win outright (a region either restates the whole value or leaves it alone),
/// matching `mjx_pptx::presentation::effective`'s own "first `Some` wins" walk read in the opposite
/// direction (least-specific first, so the *last* `Some` encountered is the most specific one).
fn fold_overwrite<'a, T>(
    chain: &[&'a StyleDefinition],
    regions: &[ConditionalFormatRegion],
    interner: &Interner,
    mut base: impl FnMut(&'a StyleDefinition) -> Option<T>,
    mut region_value: impl FnMut(&'a TableStyleOverride) -> Option<T>,
) -> Option<T> {
    let mut result = None;
    for &style in chain.iter().rev() {
        if let Some(value) = base(style) {
            result = Some(value);
        }
        for region in regions {
            let Some(value) = style
                .table_style_overrides()
                .find(|override_| override_.region(interner).ok() == Some(*region))
                .and_then(&mut region_value)
            else {
                continue;
            };
            result = Some(value);
        }
    }
    result
}

/// The table style's cell-shading contribution, resolved across `chain` and `regions` — see
/// [`fold_overwrite`].
///
/// # Errors
/// A [`DocxError`] if a matched `w:shd` is malformed.
fn fill_tier(
    chain: &[&StyleDefinition],
    regions: &[ConditionalFormatRegion],
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<Option<EffectiveShading>, DocxError> {
    let raw = fold_overwrite(
        chain,
        regions,
        interner,
        |style| style.cell_properties().and_then(CellProperties::shading),
        |override_| {
            override_
                .cell_properties()
                .and_then(CellProperties::shading)
        },
    );
    raw.map(|shading| extract_shading(shading, theme, interner))
        .transpose()
}

/// The table style's border contribution for `edge`, resolved across `chain` and `regions`.
///
/// `edge` selects the same wire slot on both [`TableBorders`] (a style's own `w:tblPr/w:tblBorders`)
/// and [`CellBorders`] (a style's `w:tcPr/w:tcBorders`) — a cell's own border reader
/// ([`Document::effective_cell_border`]) tries the cell-shaped source first (more specific), then the
/// table-shaped one, for both the style's own base and every applicable region.
///
/// # Errors
/// A [`DocxError`] if a matched border is malformed.
fn border_tier(
    chain: &[&StyleDefinition],
    regions: &[ConditionalFormatRegion],
    edge: CellBorderEdge,
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<Option<EffectiveBorder>, DocxError> {
    let raw = fold_overwrite(
        chain,
        regions,
        interner,
        |style: &super::styles::StyleDefinition| {
            style
                .cell_properties()
                .and_then(CellProperties::borders)
                .and_then(|borders| edge.select_cell_border(borders))
                .or_else(move || {
                    style
                        .table_properties()
                        .and_then(TableProperties::borders)
                        .and_then(|borders| edge.select_table_border(borders))
                })
        },
        |override_: &TableStyleOverride| {
            override_
                .cell_properties()
                .and_then(CellProperties::borders)
                .and_then(|borders| edge.select_cell_border(borders))
                .or_else(move || {
                    override_
                        .table_properties()
                        .and_then(TableProperties::borders)
                        .and_then(|borders| edge.select_table_border(borders))
                })
        },
    );
    raw.map(|border| extract_border(border, theme, interner))
        .transpose()
}

/// The table style's run-properties contribution, resolved across `chain` and `regions` — each
/// style's own base `w:rPr` merged under by every applicable region's `w:tblStylePr[type]/w:rPr` (a
/// region's own stated fields win, the base fills whatever a region leaves unset — the field-level
/// merge every other tier in this crate's ladder already uses, [`EffectiveCharacterProperties::merge_under`]),
/// then the chain itself folded the same way, leaf highest priority.
///
/// # Errors
/// A [`DocxError`] if a matched `w:rPr` is malformed.
fn run_properties_tier(
    chain: &[&StyleDefinition],
    regions: &[ConditionalFormatRegion],
    theme: &ThemeContext,
    interner: &Interner,
) -> Result<EffectiveCharacterProperties, DocxError> {
    let mut result = EffectiveCharacterProperties::default();
    for style in chain {
        let mut contribution = style
            .run_properties()
            .map(|rpr| extract_run_properties(rpr, theme, interner))
            .transpose()?
            .unwrap_or_default();
        for region in regions {
            let Some(rpr) = style
                .table_style_overrides()
                .find(|override_| override_.region(interner).ok() == Some(*region))
                .and_then(TableStyleOverride::run_properties)
            else {
                continue;
            };
            let region_contribution = extract_run_properties(rpr, theme, interner)?;
            contribution = region_contribution.merge_under(&contribution);
        }
        result = contribution.merge_under(&result);
    }
    Ok(result)
}

/// Which border slot [`Document::effective_cell_border`] reads — the same six the table-level and
/// cell-level border containers both name (see [`TableBorders`]/[`CellBorders`]); the two diagonals
/// are cell-only and read directly through [`super::tables::CellProperties::borders`] instead, since
/// no table-wide "diagonal" exists to fall back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CellBorderEdge {
    /// `w:top`.
    Top,
    /// `w:start`.
    Start,
    /// `w:left`.
    Left,
    /// `w:bottom`.
    Bottom,
    /// `w:end`.
    End,
    /// `w:right`.
    Right,
    /// `w:insideH` — between rows.
    InsideHorizontal,
    /// `w:insideV` — between columns.
    InsideVertical,
}

impl CellBorderEdge {
    fn select_cell_border(self, borders: &CellBorders) -> Option<&super::run_properties::Border> {
        match self {
            Self::Top => borders.top(),
            Self::Start => borders.start(),
            Self::Left => borders.left(),
            Self::Bottom => borders.bottom(),
            Self::End => borders.end(),
            Self::Right => borders.right(),
            Self::InsideHorizontal => borders.inside_horizontal(),
            Self::InsideVertical => borders.inside_vertical(),
        }
    }

    fn select_table_border(self, borders: &TableBorders) -> Option<&super::run_properties::Border> {
        match self {
            Self::Top => borders.top(),
            Self::Start => borders.start(),
            Self::Left => borders.left(),
            Self::Bottom => borders.bottom(),
            Self::End => borders.end(),
            Self::Right => borders.right(),
            Self::InsideHorizontal => borders.inside_horizontal(),
            Self::InsideVertical => borders.inside_vertical(),
        }
    }
}

/// What `word/document.xml` alone states about the table at `table` and the cell at `(row, column)`:
/// its dimensions, its `w:tblLook`/band sizes (defaulted), and the style id it references — gathered
/// once, before any `styles.xml` lookup, exactly as [`Document::direct_run_context`] gathers a plain
/// paragraph's own direct state before crossing into the style sheet.
struct CellTableContext {
    rows: usize,
    columns: usize,
    look: TableLookFlags,
    row_band_size: usize,
    column_band_size: usize,
    style_id: Option<String>,
}

/// The style chain (leaf first) for `context`'s own `w:tblStyle` reference, or an empty chain when
/// the table names no style — an unstated (or dangling) table style simply contributes nothing to
/// any of the three `effective_cell_*` readers, the same "no style, no opinion" degradation every
/// other tier in this crate already uses. A free function, not a [`Document`] method: every call site
/// is already inside a [`Document::style_sheet`] closure, which holds `&mut self` for its own
/// duration — a method taking `&self` cannot be called from inside it.
fn table_style_chain<'a>(
    context: &CellTableContext,
    style_index: &StyleIndex<'a>,
    interner: &Interner,
) -> Result<Vec<&'a StyleDefinition>, DocxError> {
    match &context.style_id {
        Some(id) => match style_index.based_on_chain(id, interner) {
            Ok(chain) => Ok(chain),
            Err(DocxError::UnknownStyleId(_)) => Ok(Vec::new()),
            Err(other) => Err(other),
        },
        None => Ok(Vec::new()),
    }
}

impl Document {
    /// The table at `table`'s own dimensions, `w:tblLook`/band sizes and style reference.
    ///
    /// # Errors
    /// [`DocxError::NoBody`] if the document declares no body, [`DocxError::AddressNotFound`] if no
    /// table sits at `table`, or another [`DocxError`] if the part cannot be read.
    fn cell_table_context(&mut self, table: usize) -> Result<CellTableContext, DocxError> {
        let doc = self.package.part_tree(&self.document_part)?;
        let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
        let body = main.body().ok_or(DocxError::NoBody)?;
        let table_ref = body
            .table(table)
            .ok_or_else(|| DocxError::AddressNotFound(format!("no table at {table}")))?;
        let properties = table_ref.properties();
        let look =
            TableLookFlags::from_look(properties.and_then(TableProperties::look), &doc.interner)
                .map_err(|error| DocxError::from(mjx_ooxml_core::FromXmlError::from(error)))?;
        let row_band_size = properties
            .map(|value| attr(value.effective_row_band_size(&doc.interner)))
            .transpose()?
            .unwrap_or(1);
        let column_band_size = properties
            .map(|value| attr(value.effective_column_band_size(&doc.interner)))
            .transpose()?
            .unwrap_or(1);
        let style_id = properties
            .map(|value| attr(value.style_id(&doc.interner)))
            .transpose()?
            .flatten();
        Ok(CellTableContext {
            rows: table_ref.row_count(),
            columns: table_ref.column_count(),
            look,
            row_band_size,
            column_band_size,
            style_id,
        })
    }

    /// The **effective** fill (background shading) of the cell at `(row, column)` of the table at
    /// `table` — the cell's own `w:tcPr/w:shd` wins outright; otherwise the table style's applicable
    /// regions, most specific last (see this module's own doc comment for the precedence), then the
    /// style's own base `w:tcPr/w:shd`. `None` if nothing shades the cell.
    ///
    /// Same name and `(table, row, column)` shape as `mjx_pptx::Presentation::effective_cell_fill`
    /// (which additionally takes a `Surface`, since PowerPoint slides have layouts/masters this
    /// format has no analogue for).
    ///
    /// # Errors
    /// [`DocxError::NoBody`]/[`DocxError::AddressNotFound`] as `Document::cell_table_context` (crate-private), plus
    /// any error a malformed `w:shd` or style sheet produces.
    pub fn effective_cell_fill(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
    ) -> Result<Option<EffectiveShading>, DocxError> {
        let theme = self.load_theme_context()?;
        let context = self.cell_table_context(table)?;

        let own = {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            let table_ref = body
                .table(table)
                .ok_or_else(|| DocxError::AddressNotFound(format!("no table at {table}")))?;
            let cell = table_ref.cell(&doc.interner, row, column).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no cell at ({row}, {column})"))
            })?;
            cell.properties()
                .and_then(CellProperties::shading)
                .map(|shading| extract_shading(shading, &theme, &doc.interner))
                .transpose()?
        };
        if own.is_some() {
            return Ok(own);
        }

        let regions = applicable_regions(
            row,
            column,
            context.rows,
            context.columns,
            context.look,
            context.row_band_size,
            context.column_band_size,
        );

        let result = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let style_index = StyleIndex::build(sheet, interner)?;
            let chain = table_style_chain(&context, &style_index, interner)?;
            fill_tier(&chain, &regions, &theme, interner)
        })?;
        match result {
            Some(fill) => fill,
            None => Ok(None),
        }
    }

    /// The **effective** border on one `edge` of the cell at `(row, column)` of the table at `table`
    /// — the cell's own `w:tcPr/w:tcBorders` edge wins outright; otherwise the table style's
    /// applicable regions (cell-shaped border first, then table-shaped, both tried at the style's
    /// own base and at each region), most specific last.
    ///
    /// Same name and `(table, row, column, edge)` shape as
    /// `mjx_pptx::Presentation::effective_cell_border`.
    ///
    /// # Errors
    /// As [`Document::effective_cell_fill`].
    pub fn effective_cell_border(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
        edge: CellBorderEdge,
    ) -> Result<Option<EffectiveBorder>, DocxError> {
        let theme = self.load_theme_context()?;
        let context = self.cell_table_context(table)?;

        let own = {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            let table_ref = body
                .table(table)
                .ok_or_else(|| DocxError::AddressNotFound(format!("no table at {table}")))?;
            let cell = table_ref.cell(&doc.interner, row, column).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no cell at ({row}, {column})"))
            })?;
            cell.properties()
                .and_then(CellProperties::borders)
                .and_then(|borders| edge.select_cell_border(borders))
                .map(|border| extract_border(border, &theme, &doc.interner))
                .transpose()?
        };
        if own.is_some() {
            return Ok(own);
        }

        let regions = applicable_regions(
            row,
            column,
            context.rows,
            context.columns,
            context.look,
            context.row_band_size,
            context.column_band_size,
        );

        let result = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let style_index = StyleIndex::build(sheet, interner)?;
            let chain = table_style_chain(&context, &style_index, interner)?;
            border_tier(&chain, &regions, edge, &theme, interner)
        })?;
        match result {
            Some(border) => border,
            None => Ok(None),
        }
    }

    /// The **effective** character formatting of the run at `(paragraph, run)` within the cell at
    /// `(row, column)` of the table at `table` — the ladder ECMA-376 Part 1 §17.7.2 states:
    /// `w:docDefaults` → the table style's applicable regions (this ticket's own rung) → the
    /// paragraph style's `w:basedOn` chain → the character style's `w:basedOn` chain → this run's own
    /// direct `w:rPr`. The twelve toggle properties (§17.7.3) combine by XOR across all of table,
    /// paragraph-style and character-style tiers — see `combine_toggle` (crate-private)'s own
    /// doc comment for why a bold table style over a bold paragraph style resolves to **not bold**.
    ///
    /// This reader does not yet resolve a cell paragraph's own numbering (`w:numPr`) — no fixture in
    /// this crate's own table coverage carries one; a caller with that need should extend this
    /// alongside [`Document::effective_run_properties`]'s own numbering resolution.
    ///
    /// Same name and `(table, row, column, paragraph, run)` shape as
    /// `mjx_pptx::Presentation::effective_cell_run_properties`.
    ///
    /// # Errors
    /// As [`Document::effective_cell_fill`], plus [`DocxError::AddressNotFound`] if `(paragraph,
    /// run)` does not resolve within the addressed cell.
    pub fn effective_cell_run_properties(
        &mut self,
        table: usize,
        row: usize,
        column: usize,
        paragraph: usize,
        run: usize,
    ) -> Result<EffectiveCharacterProperties, DocxError> {
        let theme = self.load_theme_context()?;
        let context = self.cell_table_context(table)?;

        let (direct, character_style_id, paragraph_style_id) = {
            let doc = self.package.part_tree(&self.document_part)?;
            let main = MainDocument::from_xml(&doc.root, &doc.interner)?;
            let body = main.body().ok_or(DocxError::NoBody)?;
            let table_ref = body
                .table(table)
                .ok_or_else(|| DocxError::AddressNotFound(format!("no table at {table}")))?;
            let cell = table_ref.cell(&doc.interner, row, column).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no cell at ({row}, {column})"))
            })?;
            let paragraph_ref = cell.paragraph(paragraph).ok_or_else(|| {
                DocxError::AddressNotFound(format!("no paragraph at {paragraph}"))
            })?;
            let run_ref = paragraph_ref
                .run(run)
                .ok_or_else(|| DocxError::AddressNotFound(format!("no run at {run}")))?;

            let direct = match run_ref.run_properties() {
                Some(rpr) => extract_run_properties(rpr, &theme, &doc.interner)?,
                None => EffectiveCharacterProperties::default(),
            };
            let character_style_id = run_ref
                .run_properties()
                .and_then(RunProperties::character_style)
                .map(|reference| attr(reference.style_id(&doc.interner)))
                .transpose()?
                .map(std::borrow::Cow::into_owned);
            let paragraph_style_id = paragraph_ref
                .properties()
                .and_then(super::paragraph_properties::ParagraphProperties::style)
                .map(|reference| attr(reference.style_id(&doc.interner)))
                .transpose()?
                .map(std::borrow::Cow::into_owned);

            (direct, character_style_id, paragraph_style_id)
        };

        let regions = applicable_regions(
            row,
            column,
            context.rows,
            context.columns,
            context.look,
            context.row_band_size,
            context.column_band_size,
        );

        let style_results = self.style_sheet(|sheet, interner| -> Result<_, DocxError> {
            let style_index = StyleIndex::build(sheet, interner)?;
            let cache = ChainCache::new(&style_index, interner);

            let doc_defaults = sheet
                .document_defaults()
                .and_then(super::styles::DocumentDefaults::run_properties_default)
                .and_then(super::styles::DefaultRunProperties::run_properties)
                .map(|rpr| extract_run_properties(rpr, &theme, interner))
                .transpose()?
                .unwrap_or_default();

            let paragraph_chain = match &paragraph_style_id {
                Some(id) => cache.chain(id)?,
                None => Vec::new(),
            };
            let character_chain = match &character_style_id {
                Some(id) => cache.chain(id)?,
                None => Vec::new(),
            };
            let paragraph_tier = merge_character_chain(&paragraph_chain, &theme, interner)?;
            let character_tier = merge_character_chain(&character_chain, &theme, interner)?;

            let table_chain = table_style_chain(&context, &style_index, interner)?;
            let table_tier = run_properties_tier(&table_chain, &regions, &theme, interner)?;

            Ok((doc_defaults, paragraph_tier, character_tier, table_tier))
        })?;
        let (doc_defaults, paragraph_tier, character_tier, table_tier) = match style_results {
            Some(result) => result?,
            None => (
                EffectiveCharacterProperties::default(),
                EffectiveCharacterProperties::default(),
                EffectiveCharacterProperties::default(),
                EffectiveCharacterProperties::default(),
            ),
        };

        let numbering_effective = EffectiveCharacterProperties::default();

        let mut merged = direct
            .merge_under(&character_tier)
            .merge_under(&paragraph_tier)
            .merge_under(&numbering_effective)
            .merge_under(&table_tier)
            .merge_under(&doc_defaults);
        recombine_toggles(
            &mut merged,
            &direct,
            &doc_defaults,
            &table_tier,
            &numbering_effective,
            &paragraph_tier,
            &character_tier,
        );
        Ok(merged)
    }
}
