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
    Angle, CharacterPropertiesSpec, FillSpec, LineSpec, ParagraphPropertiesSpec,
    TextBodyPropertiesSpec,
};
use mjx_layout::{LayoutRect, LayoutSize};
use mjx_ooxml_core::measure::Emu;
use mjx_pptx::{PptxError, Presentation, ShapeKind, Surface};

/// One deck, read into the values layout needs.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct SlideDeck {
    page: LayoutSize,
    slides: Vec<Slide>,
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
}

impl ShapeDecoration {
    /// Whether it paints nothing at all, in which case a fragment carries no handle.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fill.is_none() && self.outline.is_none()
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
        for index in 0..count {
            slides.push(read_slide(deck, Surface::Slide(index))?);
        }
        Ok(Self { page, slides })
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
        },
        _ => ShapeDecoration::default(),
    };

    let body = match kind {
        ShapeKind::Shape => read_text_body(deck, surface, path)?,
        _ => None,
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
