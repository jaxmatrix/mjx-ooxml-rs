//! [`SlideDeck`] — a deck read once, in the vocabulary layout needs, and nothing else.
//!
//! # Why laying out reads a snapshot rather than the `Presentation`
//!
//! Two reasons, and both are worth stating because the first one looks like a workaround and is not.
//!
//! **The seam only offers `&Content`.** [`BoxModel`](mjx_layout::BoxModel) takes `content: &Self::Content`
//! in all four methods, and `mjx-pptx` — like every OOXML reader in this workspace — answers every
//! question through `&mut self`, because a part is parsed the first time something asks for it.
//! `Content = Presentation` therefore does not compile, and the two ways to make it compile are
//! interior mutability (a `RefCell` in the *public* content type, whose re-entrant borrow is a panic
//! this crate may not have) or a snapshot. That is a real finding about the contract and it is
//! reported as one; it is not the reason the answer here is a snapshot.
//!
//! **The reason is cost.** Laying out one paragraph needs
//! [`effective_run_properties`](mjx_pptx::Presentation::effective_run_properties) per run, and each
//! call walks seven tiers across three parts and bakes a theme colour. A viewport re-lays a page on
//! every zoom step and every scroll into a new window, so paying that per frame would make the
//! ladder the dominant cost of scrolling. Reading it **once** and laying out from the result many
//! times is what any real renderer does, and it is what makes
//! [`BoxModel::layout_page`](mjx_layout::BoxModel::layout_page) cheap enough to be called from
//! `mjx-view`'s frame budget.
//!
//! # What it is *not*
//!
//! It is not a second model of a `.pptx`. Every value in it was **answered by `mjx-pptx`** —
//! `effective_shape_bounds`, `effective_shape_transform`, `effective_body_properties`,
//! `effective_paragraph_properties`, `effective_run_properties`, `effective_shape_fill`,
//! `effective_shape_outline` — and nothing here resolves an inheritance tier, matches a placeholder
//! slot, reads a `p:txStyles`, or looks at a raw element. `tests/the_ladder_is_consumed.rs` is what
//! holds that: it greps this crate's own source for the identifiers a re-derivation would need.
//!
//! # Re-reading one slide
//!
//! An edit changes one shape, and re-reading a three-hundred-slide deck to see it would make a
//! keystroke cost a document. [`SlideDeck::re_read_slide`] refreshes exactly one slide, which is the
//! granularity [`BoxModel::invalidate`](mjx_layout::BoxModel::invalidate) reports in.

use std::ops::Range;

use mjx_dml::{
    Angle, CellBorder, CharacterPropertiesSpec, EffectListSpec, FillSpec, LineSpec,
    ParagraphPropertiesSpec, TextAnchoring, TextBodyPropertiesSpec,
};
use mjx_layout::{LayoutRect, LayoutSize};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{PptxError, Presentation, ShapeKind, Surface};

/// One deck, read into the values layout needs.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SlideDeck {
    page: LayoutSize,
    slides: Vec<Slide>,
    notes: Vec<Option<Slide>>,
}

/// One slide's shapes, flattened into paint order.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Slide {
    shapes: Vec<Shape>,
}

/// One shape, with everything laying it out needs and nothing else.
#[derive(Clone, PartialEq, Debug)]
pub struct Shape {
    /// Its path through the shape tree — one index for a top-level shape, more for a group member.
    pub path: Vec<u32>,
    /// What kind of shape it is, which decides what fragment it becomes.
    pub kind: ShapeKind,
    /// Where it sits on the slide, **before** its own rotation, or `None` when no tier places it.
    ///
    /// A shape with no bounds is not laid out at all. That is the honest answer rather than placing
    /// it at the origin: a shape nothing places has no position, and inventing one would draw
    /// something a reader would not see in PowerPoint.
    pub bounds: Option<LayoutRect>,
    /// Its rotation about the centre of its own box (`a:xfrm@rot`, composed through its groups).
    pub rotation: Angle,
    /// Whether it is mirrored horizontally (`@flipH`).
    pub flip_horizontal: bool,
    /// Whether it is mirrored vertically (`@flipV`).
    pub flip_vertical: bool,
    /// What paints it — resolved, so the scene builder above reads a value rather than a document.
    pub decoration: ShapeDecoration,
    /// Its text body, when it has one.
    pub body: Option<TextBody>,
    /// What it *contains*, for the two shape kinds that contain something other than text.
    ///
    /// A `p:sp` is its text body and nothing else, so it is [`ShapeContent::Nothing`]; a
    /// `p:graphicFrame` framing an `a:tbl` is a grid; a `p:pic` is a picture. Keeping this apart
    /// from [`kind`](Self::kind) is deliberate: `kind` is what the *element* is, and a graphic frame
    /// holding a chart is the same kind as one holding a table while containing something this
    /// crate does not lay out.
    pub content: ShapeContent,
}

/// What a shape contains, beyond its own text.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum ShapeContent {
    /// Nothing — an autoshape, a connector, a group, or a graphic frame holding something this
    /// crate does not lay out (a chart or a diagram, which are R23).
    #[default]
    Nothing,
    /// A table (`a:tbl` inside a `p:graphicFrame`).
    Table(TableContent),
    /// A picture (`p:pic`).
    Picture(PictureContent),
}

/// A table, in the vocabulary laying it out needs.
///
/// Every value here was answered by `mjx-pptx` — `table_dimensions`, `column_width`, `row_height`,
/// `cell_span`, `merged_cell_anchor`, `effective_cell_fill`, `effective_cell_border`,
/// `cell_margins`, `cell_anchor` — and the six conditional bands of the table style have already
/// been resolved by the time a value arrives here. Nothing in this crate walks a `a:tblStyle`.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct TableContent {
    /// Each column's stated width, or `None` where the column states none.
    pub columns: Vec<Option<Emu>>,
    /// Each row's stated height, or `None` where the row states none.
    pub rows: Vec<Option<Emu>>,
    /// Every grid position, row-major — including the ones a merge covers, which is what makes
    /// `(row, column)` addressing hole-free. See [`Cell::covered_by`].
    pub cells: Vec<Cell>,
}

impl TableContent {
    /// How many rows the grid has.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// How many columns it has.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// The cell at `(row, column)`, or `None` past the grid.
    #[must_use]
    pub fn cell(&self, row: usize, column: usize) -> Option<&Cell> {
        if column >= self.column_count() {
            return None;
        }
        self.cells
            .get(row.checked_mul(self.column_count())?.checked_add(column)?)
    }
}

/// One grid position of a table.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Cell {
    /// How many columns it spans (`@gridSpan`); one when it is not merged across.
    pub column_span: usize,
    /// How many rows it spans (`@rowSpan`); one when it is not merged down.
    pub row_span: usize,
    /// Which cell renders here, when a merge covers this position — `None` when this cell renders
    /// itself.
    ///
    /// A merge never removes a cell (`docs/TABLES_HANDOFF.md`, decision 1), so a covered position
    /// still holds its own text and its own properties; what it does not do is draw them. Laying
    /// out a covered position anyway is the naive walk this field exists to make impossible.
    pub covered_by: Option<(usize, usize)>,
    /// Its four insets, each `None` where the cell states none and the schema default applies.
    pub margins: CellInsets,
    /// Where its text sits vertically (`@anchor`).
    pub anchor: Option<TextAnchoring>,
    /// Its effective fill, through the table style's conditional bands.
    pub fill: Option<FillSpec>,
    /// Its effective borders, in [`CELL_EDGES`] order.
    pub borders: [Option<LineSpec>; 6],
    /// Its text, when it has any.
    pub body: Option<TextBody>,
}

/// The six edges a cell can outline, in the order [`Cell::borders`] stores them.
pub const CELL_EDGES: [CellBorder; 6] = [
    CellBorder::Left,
    CellBorder::Right,
    CellBorder::Top,
    CellBorder::Bottom,
    CellBorder::TopLeftToBottomRight,
    CellBorder::BottomLeftToTopRight,
];

/// A cell's four insets, each `None` where the cell states none.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct CellInsets {
    /// `@marL`.
    pub left: Option<Emu>,
    /// `@marR`.
    pub right: Option<Emu>,
    /// `@marT`.
    pub top: Option<Emu>,
    /// `@marB`.
    pub bottom: Option<Emu>,
}

/// A picture, in the vocabulary drawing it needs.
///
/// **The pixels are not here and never will be.** A box model says *where* a picture is and *which*
/// picture it is; decoding it is the layer above's, under `mjx-view`'s byte budget. That is the same
/// division [`mjx_layout::ImageRef`] describes.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct PictureContent {
    /// The relationship id of the image it shows (`a:blip@r:embed`, else `@r:link`), or `None` when
    /// the picture binds none.
    ///
    /// The identity a resource table keys on: two pictures showing the same image name the same
    /// relationship, so they decode once.
    pub image_rel_id: Option<String>,
}

/// What paints a shape.
///
/// Carried so that the [`DecorationRef`](mjx_layout::DecorationRef) a fragment issues resolves to a
/// value. The fragment tree deliberately holds only the number; this is the table the layer that
/// holds the box model reads it in, exactly as `mjx-layout`'s own documentation describes.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ShapeDecoration {
    /// Its effective fill, or `None` when nothing fills it.
    pub fill: Option<FillSpec>,
    /// Its effective outline, or `None` when nothing outlines it.
    pub outline: Option<LineSpec>,
    /// Its effective effect list, or `None` when it has none.
    ///
    /// ⚠ **The colours in it have lost their opacity**, and this is not a defect of this crate:
    /// `mjx-dml`'s `resolve_effects` bakes every effect colour to a `ColorSpec::Srgb` hex triplet,
    /// which has no alpha channel, and says so in its own documentation. The standard Office theme
    /// puts `<a:alpha val="63000"/>` on the shadow of every shape, so this loss is universal rather
    /// than exotic. `tests/the_opacity_is_lost_at_the_spec_boundary.rs` proves it is still lost, so
    /// that fixing it is a test going green rather than a thing nobody remembers.
    pub effects: Option<EffectListSpec>,
}

impl ShapeDecoration {
    /// Whether it paints nothing at all, in which case a fragment carries no handle.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fill.is_none() && self.outline.is_none() && self.effects.is_none()
    }
}

/// A shape's text, with every property already resolved.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct TextBody {
    /// The body's effective geometry — insets, anchor, wrap, columns, autofit.
    pub geometry: TextBodyPropertiesSpec,
    /// Its paragraphs, in order.
    pub paragraphs: Vec<Paragraph>,
}

/// One paragraph, with its effective properties and its text.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct Paragraph {
    /// Its effective properties — level, alignment, margins, spacing, bullet, tab stops.
    pub properties: ParagraphPropertiesSpec,
    /// Its text: every run's text concatenated, which is the string a line is composed from.
    pub text: String,
    /// Its runs, each covering a range of [`text`](Self::text).
    pub runs: Vec<Run>,
}

/// One run of a paragraph.
#[derive(Clone, PartialEq, Debug)]
pub struct Run {
    /// The bytes of the paragraph's text this run covers.
    pub range: Range<usize>,
    /// Its effective character properties — typeface, size, weight, slant, and the rest.
    pub properties: CharacterPropertiesSpec,
}

impl SlideDeck {
    /// Reads every slide of `deck`.
    ///
    /// # Errors
    /// [`PptxError`] if the package is malformed or a relationship points outside it.
    pub fn read(deck: &mut Presentation) -> Result<Self, PptxError> {
        let size = deck.slide_size()?;
        let page = LayoutSize::new(
            Emu::from_emu(size.width_emu),
            Emu::from_emu(size.height_emu),
        );
        let count = deck.slide_count();
        let mut slides = Vec::with_capacity(count);
        let mut notes = Vec::with_capacity(count);
        for index in 0..count {
            slides.push(read_slide(deck, Surface::Slide(index))?);
            // A notes slide is a slide by another name — the same shape tree, the same text bodies,
            // the same placeholder inheritance — so it costs one more read and nothing else. A slide
            // with no notes part is `None` rather than an empty slide, because *there is no notes
            // page* and *there is a blank one* are different answers to "print the notes".
            //
            // A slide states whether it has one by *having the part*, and `mjx-pptx` reports a
            // missing one as an error from the first question asked of it. So the absence is read
            // from the failure of `read_slide` rather than from a predicate — there is no
            // `has_notes` on `Presentation`, and adding one to answer a question the error already
            // answers would be a second source of the same fact. A malformed notes part is treated
            // the same way, deliberately: the slide itself already read, and refusing the whole deck
            // over an unreadable notes page would lose the deck to save the notes.
            notes.push(read_slide(deck, Surface::Notes(index)).ok());
        }
        Ok(Self {
            page,
            slides,
            notes,
        })
    }

    /// The notes slide of slide `index`, or `None` when that slide has none.
    #[must_use]
    pub fn notes(&self, index: usize) -> Option<&Slide> {
        self.notes.get(index).and_then(Option::as_ref)
    }

    /// How many slides carry a notes page.
    #[must_use]
    pub fn notes_count(&self) -> usize {
        self.notes.iter().filter(|notes| notes.is_some()).count()
    }

    /// Re-reads one slide, leaving every other slide's values untouched.
    ///
    /// The granularity an edit invalidates at: a keystroke changes one run on one slide, and
    /// re-reading the deck to see it would make typing cost a document.
    ///
    /// # Errors
    /// [`PptxError`] as [`SlideDeck::read`]. A slide index past the end is not an error — it is a
    /// slide that no longer exists, and the answer is to leave the deck alone.
    pub fn re_read_slide(
        &mut self,
        deck: &mut Presentation,
        index: usize,
    ) -> Result<(), PptxError> {
        if index >= self.slides.len() || index >= deck.slide_count() {
            return Ok(());
        }
        let slide = read_slide(deck, Surface::Slide(index))?;
        if let Some(slot) = self.slides.get_mut(index) {
            *slot = slide;
        }
        Ok(())
    }

    /// How big every slide is.
    #[must_use]
    pub fn page(&self) -> LayoutSize {
        self.page
    }

    /// How many slides there are.
    #[must_use]
    pub fn slide_count(&self) -> usize {
        self.slides.len()
    }

    /// Whether the deck has no slides at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.slides.is_empty()
    }

    /// One slide, or `None` past the end.
    #[must_use]
    pub fn slide(&self, index: usize) -> Option<&Slide> {
        self.slides.get(index)
    }
}

impl Slide {
    /// Its shapes, in paint order — the order the shape tree lists them, which is back to front.
    #[must_use]
    pub fn shapes(&self) -> &[Shape] {
        &self.shapes
    }
}

impl Paragraph {
    /// The run covering byte `offset` of [`text`](Self::text), and the offset inside that run.
    ///
    /// The join between a hit test's answer — a byte of the paragraph — and the pair
    /// `mjx-pptx`'s text API takes.
    #[must_use]
    pub fn run_at(&self, offset: usize) -> Option<(usize, usize)> {
        self.runs
            .iter()
            .position(|run| offset >= run.range.start && offset < run.range.end)
            .or_else(|| {
                // A caret at the very end of the paragraph belongs to the last run rather than to
                // no run, which is what makes "click past the end of the text" land somewhere.
                self.runs
                    .len()
                    .checked_sub(1)
                    .filter(|_| offset >= self.text.len())
            })
            .and_then(|index| {
                let run = self.runs.get(index)?;
                Some((index, offset.saturating_sub(run.range.start)))
            })
    }
}

/// Reads one surface's shapes.
fn read_slide(deck: &mut Presentation, surface: Surface) -> Result<Slide, PptxError> {
    let mut shapes = Vec::new();
    let count = deck.shape_count(surface)?;
    for index in 0..count {
        read_shape(deck, surface, &mut vec![index], &mut shapes)?;
    }
    Ok(Slide { shapes })
}

/// Reads the shape at `path` and, when it is a group, every member under it.
///
/// A group is emitted *before* its members so that paint order in the flattened list is the paint
/// order of the tree: a group's own box is behind everything inside it.
fn read_shape(
    deck: &mut Presentation,
    surface: Surface,
    path: &mut Vec<usize>,
    into: &mut Vec<Shape>,
) -> Result<(), PptxError> {
    let kind = deck.shape_kind(surface, path.clone())?;
    let transform = deck.effective_shape_transform(surface, path.clone())?;
    let bounds = deck
        .effective_shape_bounds(surface, path.clone())?
        .map(|bounds| {
            LayoutRect::from_edges(
                Emu::from_emu(bounds.offset_x_emu),
                Emu::from_emu(bounds.offset_y_emu),
                Emu::from_emu(bounds.offset_x_emu.saturating_add(bounds.width_emu)),
                Emu::from_emu(bounds.offset_y_emu.saturating_add(bounds.height_emu)),
            )
        });

    let decoration = match kind {
        // Only a shape and a connector carry `p:spPr`'s fill and outline. Asking for a group's or a
        // graphic frame's would be asking a question the schema does not have an answer to.
        ShapeKind::Shape | ShapeKind::ConnectionShape => ShapeDecoration {
            fill: deck.effective_shape_fill(surface, path.clone())?,
            outline: deck.effective_shape_outline(surface, path.clone())?,
            effects: deck.effective_shape_effects(surface, path.clone())?,
        },
        _ => ShapeDecoration::default(),
    };

    let body = match kind {
        ShapeKind::Shape => read_text_body(deck, surface, path)?,
        _ => None,
    };

    let content = match kind {
        // A graphic frame holds whatever `a:graphicData` holds. A table is laid out; a chart and a
        // diagram are R23, and both report `ShapeIsNotATable` here, which is the frame saying it
        // holds something else rather than the read failing.
        ShapeKind::GraphicFrame => match read_table(deck, surface, path) {
            Ok(table) => ShapeContent::Table(table),
            Err(PptxError::ShapeIsNotATable) => ShapeContent::Nothing,
            Err(error) => return Err(error),
        },
        ShapeKind::Picture => ShapeContent::Picture(PictureContent {
            image_rel_id: deck.picture_image_rel_id(surface, path.clone())?,
        }),
        _ => ShapeContent::Nothing,
    };

    into.push(Shape {
        path: path.iter().map(|&index| index as u32).collect(),
        kind,
        bounds,
        rotation: transform
            .as_ref()
            .and_then(|transform| transform.rotation)
            .unwrap_or_else(|| Angle::from_degrees(0.0)),
        flip_horizontal: transform
            .as_ref()
            .and_then(|transform| transform.flip_horizontal)
            .unwrap_or(false),
        flip_vertical: transform
            .as_ref()
            .and_then(|transform| transform.flip_vertical)
            .unwrap_or(false),
        decoration,
        body,
        content,
    });

    if kind == ShapeKind::GroupShape {
        let members = deck.shape_member_count(surface, path.clone())?;
        for member in 0..members {
            path.push(member);
            read_shape(deck, surface, path, into)?;
            path.pop();
        }
    }
    Ok(())
}

/// Reads a shape's text body, or `None` when it has none.
fn read_text_body(
    deck: &mut Presentation,
    surface: Surface,
    path: &[usize],
) -> Result<Option<TextBody>, PptxError> {
    let paragraph_count = match deck.paragraph_count(surface, path.to_vec()) {
        Ok(count) => count,
        // A shape with no `p:txBody` is not a failure; it is a shape with no text.
        Err(PptxError::ShapeHasNoTextBody) => return Ok(None),
        Err(error) => return Err(error),
    };

    let geometry = deck.effective_body_properties(surface, path.to_vec())?;
    let mut paragraphs = Vec::with_capacity(paragraph_count);
    for index in 0..paragraph_count {
        paragraphs.push(read_paragraph(deck, surface, path, index)?);
    }
    Ok(Some(TextBody {
        geometry,
        paragraphs,
    }))
}

/// Reads one paragraph: its effective properties, its text, and where each run sits in it.
fn read_paragraph(
    deck: &mut Presentation,
    surface: Surface,
    path: &[usize],
    index: usize,
) -> Result<Paragraph, PptxError> {
    let properties = deck.effective_paragraph_properties(surface, path.to_vec(), index)?;
    let run_count = deck.run_count(surface, path.to_vec(), index)?;

    let mut text = String::new();
    let mut runs = Vec::with_capacity(run_count);
    for run_index in 0..run_count {
        let run_text = deck.run_text(surface, path.to_vec(), index, run_index)?;
        let start = text.len();
        text.push_str(&run_text);
        runs.push(Run {
            range: start..text.len(),
            properties: deck.effective_run_properties(surface, path.to_vec(), index, run_index)?,
        });
    }

    Ok(Paragraph {
        properties,
        text,
        runs,
    })
}

/// Reads the table a `p:graphicFrame` frames.
///
/// Every value comes from `mjx-pptx`, already resolved: the grid from `table_dimensions` /
/// `column_width` / `row_height`, the merges from `cell_span` / `merged_cell_anchor`, and the
/// formatting from the three `effective_cell_*` readers, which have already walked the table
/// style's six conditional bands. Nothing here reads an `a:tblStyle`.
///
/// # Errors
/// [`PptxError::ShapeIsNotATable`] when the frame holds something else — a chart, a diagram — which
/// is the caller's signal to lay the frame out as a box rather than a failure. Any other
/// [`PptxError`] as the readers report it.
fn read_table(
    deck: &mut Presentation,
    surface: Surface,
    path: &[usize],
) -> Result<TableContent, PptxError> {
    let (rows, columns) = deck.table_dimensions(surface, path.to_vec())?;

    let mut column_widths = Vec::with_capacity(columns);
    for column in 0..columns {
        column_widths.push(deck.column_width(surface, path.to_vec(), column)?);
    }
    let mut row_heights = Vec::with_capacity(rows);
    for row in 0..rows {
        row_heights.push(deck.row_height(surface, path.to_vec(), row)?);
    }

    let mut cells = Vec::with_capacity(rows.saturating_mul(columns));
    for row in 0..rows {
        for column in 0..columns {
            cells.push(read_cell(deck, surface, path, row, column)?);
        }
    }

    Ok(TableContent {
        columns: column_widths,
        rows: row_heights,
        cells,
    })
}

/// Reads one grid position of a table.
fn read_cell(
    deck: &mut Presentation,
    surface: Surface,
    path: &[usize],
    row: usize,
    column: usize,
) -> Result<Cell, PptxError> {
    let (row_span, column_span) = deck.cell_span(surface, path.to_vec(), row, column)?;
    let anchor_cell = deck.merged_cell_anchor(surface, path.to_vec(), row, column)?;
    let covered_by = (anchor_cell != (row, column)).then_some(anchor_cell);

    let margins = deck.cell_margins(surface, path.to_vec(), row, column)?;
    let mut borders: [Option<LineSpec>; 6] = Default::default();
    for (slot, edge) in CELL_EDGES.into_iter().enumerate() {
        let resolved = deck.effective_cell_border(surface, path.to_vec(), row, column, edge)?;
        if let Some(entry) = borders.get_mut(slot) {
            *entry = resolved;
        }
    }

    // A covered cell keeps its own text (`docs/TABLES_HANDOFF.md`, decision 2) and does not draw
    // it, so reading it would be work whose result is discarded. Its geometry still matters: the
    // anchor's rectangle is the union of the positions it covers, and that union is computed from
    // the grid rather than from these cells.
    let body = if covered_by.is_some() {
        None
    } else {
        read_cell_text(deck, surface, path, row, column)?
    };

    Ok(Cell {
        column_span,
        row_span,
        covered_by,
        margins: CellInsets {
            left: margins.left,
            right: margins.right,
            top: margins.top,
            bottom: margins.bottom,
        },
        anchor: deck.cell_anchor(surface, path.to_vec(), row, column)?,
        fill: deck.effective_cell_fill(surface, path.to_vec(), row, column)?,
        borders,
        body,
    })
}

/// Reads a cell's paragraphs, or `None` when it has no text body.
///
/// The cell's own geometry — its insets and its anchor — is **not** part of this, because a cell
/// states those on `a:tcPr` rather than on an `a:bodyPr`, and turning them into one is the layout
/// step's job rather than the reader's. What comes back is the paragraphs and nothing else, in the
/// same shape a shape's text body has, so that one text engine lays out both.
fn read_cell_text(
    deck: &mut Presentation,
    surface: Surface,
    path: &[usize],
    row: usize,
    column: usize,
) -> Result<Option<TextBody>, PptxError> {
    let paragraph_count = match deck.cell_paragraph_count(surface, path.to_vec(), row, column) {
        Ok(count) => count,
        Err(PptxError::ShapeHasNoTextBody) => return Ok(None),
        Err(error) => return Err(error),
    };

    let mut paragraphs = Vec::with_capacity(paragraph_count);
    for index in 0..paragraph_count {
        // A cell's paragraph properties are the cell's **own**: `mjx-pptx` has no
        // `effective_cell_paragraph_properties`, because a table style's conditional bands carry
        // character properties and a cell fill and no `a:pPr` at all. So a paragraph that states
        // nothing takes the schema's defaults, which is what an unstated `ParagraphPropertiesSpec`
        // means everywhere else in this crate.
        let properties = deck
            .cell_paragraph_properties(surface, path.to_vec(), row, column, index)?
            .unwrap_or_default();
        let run_count = deck.cell_run_count(surface, path.to_vec(), row, column, index)?;
        let mut text = String::new();
        let mut runs = Vec::with_capacity(run_count);
        for run_index in 0..run_count {
            let run_text =
                deck.cell_run_text(surface, path.to_vec(), row, column, index, run_index)?;
            let start = text.len();
            text.push_str(&run_text);
            runs.push(Run {
                range: start..text.len(),
                properties: deck.effective_cell_run_properties(
                    surface,
                    path.to_vec(),
                    row,
                    column,
                    index,
                    run_index,
                )?,
            });
        }
        paragraphs.push(Paragraph {
            properties,
            text,
            runs,
        });
    }

    Ok(Some(TextBody {
        // A cell's geometry is written by `crate::table`, from `a:tcPr`'s own attributes. This is
        // the *unstated* body: every field inherits, and the layout step replaces it wholesale.
        geometry: TextBodyPropertiesSpec::new(),
        paragraphs,
    }))
}
