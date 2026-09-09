//! Anchoring: where a floating object actually is, and the [`Exclusion`] it therefore contributes.
//!
//! # The five frames
//!
//! `wp:positionH@relativeFrom` and `wp:positionV@relativeFrom` name what a position is measured
//! against, and the five that matter to a flowing document are the page, the margin box, the column,
//! the paragraph and the character. [`Anchorage`] carries all five as plain edges, resolved by
//! whatever is filling the column, so this module does arithmetic and never asks a question about
//! the document.
//!
//! **The paragraph frame is the one that makes this ordering-sensitive.** A float anchored to the
//! paragraph it sits in cannot be placed until that paragraph's top is known, and that is known only
//! when the column has been filled down to it — which is exactly why [`crate::paginate::fill_column`]
//! resolves floats as it goes rather than up front. A float anchored to the page or the margin needs
//! nothing from the flow and could have been placed first; the two are resolved by the same code
//! because the difference is which edge the offset is added to and nothing else.
//!
//! # What terminates
//!
//! Nothing here iterates. A float's rectangle is a function of its anchor and its frame, both of
//! which are settled before it is asked for; the only loop in the wrapping story is
//! [`crate::flow`]'s per-line one, whose bound is written there.
//!
//! # ⚠ Provenance
//!
//! `EngineDerived` throughout, and honestly so. §20.4.3.4's `ST_RelFromH` and §20.4.3.5's
//! `ST_RelFromV` name their frames and say nothing about what a renderer does when a frame is absent,
//! when an offset pushes an object off the page, or when two objects overlap and `allowOverlap` is
//! off. Every one of those is marked `GUESS:` at the site that decides it.

use mjx_docx::{
    AnchoredDrawing, AxisPlacement, DrawingFormatting, DrawingPlacement, FloatingTableAnchoring,
    WrapFormatting,
};
use mjx_ooxml_core::measure::Emu;
use mjx_ooxml_types::wordprocessingdrawing::{
    HorizontalAlignment, HorizontalRelativeFrom, VerticalAlignment, VerticalRelativeFrom,
};

use crate::wrap::{Exclusion, WrapSide};

/// The frames a floating object may be positioned against, in the column's own coordinates.
///
/// Every field is an edge or a size in the space [`crate::paginate::fill_column`] places blocks in:
/// `x` grows right from the column's left edge, `y` grows down from the column's top.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Anchorage {
    /// How wide the column is.
    pub column_width: Emu,
    /// How tall it is.
    pub column_height: Emu,
    /// The page's left edge, relative to the column's.
    pub page_left: Emu,
    /// Its right edge.
    pub page_right: Emu,
    /// Its top edge, relative to the column's top.
    pub page_top: Emu,
    /// Its bottom.
    pub page_bottom: Emu,
    /// The text-area (margin box) left edge, relative to the column's.
    pub margin_left: Emu,
    /// Its right.
    pub margin_right: Emu,
    /// Its top, relative to the column's top.
    pub margin_top: Emu,
    /// Its bottom.
    pub margin_bottom: Emu,
    /// Where the paragraph the object is anchored in starts, from the column's top.
    pub paragraph_top: Emu,
    /// Where its text begins horizontally, after its own indents.
    pub paragraph_left: Emu,
}

impl Anchorage {
    /// A column with nothing outside it — the frame a header, a footnote or a table cell gives a
    /// float anchored inside it, where page and margin coincide with the column.
    #[must_use]
    pub fn contained(width: Emu, height: Emu, paragraph_top: Emu) -> Self {
        Self {
            column_width: width,
            column_height: height,
            page_left: Emu::ZERO,
            page_right: width,
            page_top: Emu::ZERO,
            page_bottom: height,
            margin_left: Emu::ZERO,
            margin_right: width,
            margin_top: Emu::ZERO,
            margin_bottom: height,
            paragraph_top,
            paragraph_left: Emu::ZERO,
        }
    }

    /// The same with the paragraph moved to `top`.
    #[must_use]
    pub fn at_paragraph(mut self, top: Emu) -> Self {
        self.paragraph_top = top;
        self
    }

    /// The horizontal frame `relative_to` names, as `(left, right)`.
    fn horizontal_frame(self, relative_to: HorizontalRelativeFrom) -> (Emu, Emu) {
        match relative_to {
            HorizontalRelativeFrom::Page => (self.page_left, self.page_right),
            HorizontalRelativeFrom::Margin
            | HorizontalRelativeFrom::InsideMargin
            | HorizontalRelativeFrom::OutsideMargin => (self.margin_left, self.margin_right),
            HorizontalRelativeFrom::LeftMargin => (self.page_left, self.margin_left),
            HorizontalRelativeFrom::RightMargin => (self.margin_right, self.page_right),
            // GUESS: `character` is the paragraph's text start rather than the run the anchor sits
            // in. A run's own x is not knowable before the line it lands on is composed, and the
            // line it lands on depends on this object — a genuine cycle, cut here at the paragraph.
            HorizontalRelativeFrom::Character => (self.paragraph_left, self.column_width),
            HorizontalRelativeFrom::Column => (Emu::ZERO, self.column_width),
        }
    }

    /// The vertical frame `relative_to` names, as `(top, bottom)`.
    fn vertical_frame(self, relative_to: VerticalRelativeFrom) -> (Emu, Emu) {
        match relative_to {
            VerticalRelativeFrom::Page => (self.page_top, self.page_bottom),
            VerticalRelativeFrom::Margin
            | VerticalRelativeFrom::InsideMargin
            | VerticalRelativeFrom::OutsideMargin => (self.margin_top, self.margin_bottom),
            VerticalRelativeFrom::TopMargin => (self.page_top, self.margin_top),
            VerticalRelativeFrom::BottomMargin => (self.margin_bottom, self.page_bottom),
            VerticalRelativeFrom::Paragraph | VerticalRelativeFrom::Line => {
                (self.paragraph_top, self.column_height)
            }
        }
    }
}

/// Where one anchored drawing sits, and what text has to do about it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlacedFloat {
    /// The object's own rectangle, distances **not** included — this is what a painter draws into.
    pub left: Emu,
    /// Its top.
    pub top: Emu,
    /// Its width.
    pub width: Emu,
    /// Its height.
    pub height: Emu,
    /// Whether it draws behind the text (`@behindDoc`).
    pub behind_text: bool,
    /// `@relativeHeight` — the z order among the page's floats.
    pub z_order: u32,
    /// Which paragraph anchored it.
    pub paragraph: usize,
    /// Which of that paragraph's drawings it is.
    pub drawing: usize,
    /// The exclusion it contributes, or `None` when text is not displaced at all.
    pub exclusion: Option<Exclusion>,
}

/// Resolves one anchored drawing against `frame`.
///
/// Returns `None` for an inline drawing — an inline object is a character of a line and does not
/// float — and for a hidden one.
#[must_use]
pub fn place(
    drawing: &DrawingFormatting,
    paragraph: usize,
    index: usize,
    frame: Anchorage,
) -> Option<PlacedFloat> {
    let DrawingPlacement::Anchored(anchor) = &drawing.placement else {
        return None;
    };
    if anchor.hidden {
        return None;
    }
    let width = Emu::from_emu(drawing.width);
    let height = Emu::from_emu(drawing.height);
    let left = horizontal(anchor, frame, width);
    let top = vertical(anchor, frame, height);
    Some(PlacedFloat {
        left,
        top,
        width,
        height,
        behind_text: anchor.behind_text,
        z_order: anchor.relative_height,
        paragraph,
        drawing: index,
        exclusion: exclusion_of(anchor, left, top, width, height),
    })
}

/// Where the object's left edge lands.
fn horizontal(anchor: &AnchoredDrawing, frame: Anchorage, width: Emu) -> Emu {
    let (start, end) = frame.horizontal_frame(anchor.horizontal.relative_to);
    match anchor.horizontal.placement {
        AxisPlacement::Offset(offset) => start + Emu::from_emu(offset),
        AxisPlacement::Aligned(alignment) => match alignment {
            HorizontalAlignment::Left | HorizontalAlignment::Inside => start,
            HorizontalAlignment::Right | HorizontalAlignment::Outside => end - width,
            HorizontalAlignment::Center => start + ((end - start) - width).divided_by(2),
        },
    }
}

/// Where its top edge lands.
fn vertical(anchor: &AnchoredDrawing, frame: Anchorage, height: Emu) -> Emu {
    let (start, end) = frame.vertical_frame(anchor.vertical.relative_to);
    match anchor.vertical.placement {
        AxisPlacement::Offset(offset) => start + Emu::from_emu(offset),
        AxisPlacement::Aligned(alignment) => match alignment {
            VerticalAlignment::Top | VerticalAlignment::Inside => start,
            VerticalAlignment::Bottom | VerticalAlignment::Outside => end - height,
            VerticalAlignment::Center => start + ((end - start) - height).divided_by(2),
        },
    }
}

/// The exclusion the object's wrap mode asks for, in column coordinates.
fn exclusion_of(
    anchor: &AnchoredDrawing,
    left: Emu,
    top: Emu,
    width: Emu,
    height: Emu,
) -> Option<Exclusion> {
    let right = left + width;
    let bottom = top + height;
    match &anchor.wrap {
        // `wp:wrapNone` displaces nothing: the object is behind the text or in front of it, and the
        // text is laid out exactly as if it were not there. **This is the identity value**, and a
        // fixture made only of these would exercise none of `crate::wrap`.
        WrapFormatting::None => None,
        WrapFormatting::Square { side, distance } => Some(Exclusion::rectangle(
            left - Emu::from_emu(distance.left),
            top - Emu::from_emu(distance.top),
            right + Emu::from_emu(distance.right),
            bottom + Emu::from_emu(distance.bottom),
            WrapSide::of(*side),
        )),
        WrapFormatting::Tight {
            side,
            polygon,
            distance_left,
            distance_right,
        }
        | WrapFormatting::Through {
            side,
            polygon,
            distance_left,
            distance_right,
        } => {
            let through = matches!(anchor.wrap, WrapFormatting::Through { .. });
            if polygon.len() < 3 {
                // A tight wrap with no usable polygon is a square wrap; **`GUESS:`** the schema
                // requires the polygon, and falling back to the box keeps text off the object rather
                // than through it.
                return Some(Exclusion::rectangle(
                    left - Emu::from_emu(*distance_left),
                    top,
                    right + Emu::from_emu(*distance_right),
                    bottom,
                    WrapSide::of(*side),
                ));
            }
            Some(Exclusion::from_polygon(
                left - Emu::from_emu(*distance_left),
                top,
                right + Emu::from_emu(*distance_right),
                bottom,
                width,
                height,
                polygon,
                WrapSide::of(*side),
                through,
            ))
        }
        WrapFormatting::TopAndBottom {
            distance_top,
            distance_bottom,
        } => Some(Exclusion::band(
            top - Emu::from_emu(*distance_top),
            bottom + Emu::from_emu(*distance_bottom),
        )),
    }
}

/// Where a floating **table** sits, and the exclusion it contributes.
///
/// A `w:tblpPr` is the same problem in a different vocabulary: `@tblpX`/`@tblpXSpec` against
/// `@horzAnchor` is `wp:positionH` with two attributes instead of two elements, and the four
/// `*FromText` distances are `distL`/`distR`/`distT`/`distB`. **A floating table always wraps
/// square** — WordprocessingML gives it no polygon and no `wrapText` — which is why this returns a
/// rectangle and never a polygon.
///
/// **`GUESS:`** the side. §17.4.59 states no `wrapText` equivalent, so a floating table is read as
/// [`WrapSide::BothSides`]; the alternative (largest) would silently drop text beside a narrow table.
#[must_use]
pub fn place_table(
    anchoring: &FloatingTableAnchoring,
    width: Emu,
    height: Emu,
    frame: Anchorage,
) -> PlacedFloat {
    use mjx_ooxml_types::shared::{RelativeHorizontalAlignment, RelativeVerticalAlignment};
    use mjx_ooxml_types::wordprocessingml::{HorizontalAnchor, VerticalAnchor};

    let (h_start, h_end) = match anchoring.horizontal_anchor {
        Some(HorizontalAnchor::Page) => (frame.page_left, frame.page_right),
        Some(HorizontalAnchor::Margin) => (frame.margin_left, frame.margin_right),
        _ => (Emu::ZERO, frame.column_width),
    };
    let (v_start, v_end) = match anchoring.vertical_anchor {
        Some(VerticalAnchor::Page) => (frame.page_top, frame.page_bottom),
        Some(VerticalAnchor::Margin) => (frame.margin_top, frame.margin_bottom),
        _ => (frame.paragraph_top, frame.column_height),
    };
    let left = match anchoring.x_alignment {
        Some(RelativeHorizontalAlignment::Left | RelativeHorizontalAlignment::Inside) => h_start,
        Some(RelativeHorizontalAlignment::Right | RelativeHorizontalAlignment::Outside) => {
            h_end - width
        }
        Some(RelativeHorizontalAlignment::Center) => {
            h_start + ((h_end - h_start) - width).divided_by(2)
        }
        None => h_start + Emu::from_twips(anchoring.x_twips.unwrap_or(0)),
    };
    let top = match anchoring.y_alignment {
        Some(RelativeVerticalAlignment::Top | RelativeVerticalAlignment::Inside) => v_start,
        Some(RelativeVerticalAlignment::Bottom | RelativeVerticalAlignment::Outside) => {
            v_end - height
        }
        Some(RelativeVerticalAlignment::Center) => {
            v_start + ((v_end - v_start) - height).divided_by(2)
        }
        // `inline` is `ST_YAlign`'s "flow with the text" value, which is what a table that is *not*
        // floating does — so it is read as no vertical displacement at all.
        Some(RelativeVerticalAlignment::Inline) | None => {
            v_start + Emu::from_twips(anchoring.y_twips.unwrap_or(0))
        }
    };
    PlacedFloat {
        left,
        top,
        width,
        height,
        behind_text: false,
        z_order: 0,
        paragraph: 0,
        drawing: 0,
        exclusion: Some(Exclusion::rectangle(
            left - Emu::from_twips(anchoring.left_from_text),
            top - Emu::from_twips(anchoring.top_from_text),
            left + width + Emu::from_twips(anchoring.right_from_text),
            top + height + Emu::from_twips(anchoring.bottom_from_text),
            WrapSide::BothSides,
        )),
    }
}

/// The height an **inline** drawing adds to the line it sits on.
///
/// An inline object is a character of the line, so it raises the line's ascent to its own height —
/// which is the whole of "a very tall run" and is what makes a paragraph containing a picture taller
/// than one containing only text.
///
/// # ⚠ Its *advance* is not measured, and that is a declared gap rather than an oversight
///
/// Reserving an inline object's **width** means giving the line composer a fixed advance for one
/// character, and `mjx_layout::TextRun` has no such field: a composer run is a face, a size and a
/// range, and its width is whatever the shaper says. Adding one is a change to the box-model
/// contract every format shares, not to this crate, so it is stated here rather than approximated:
/// **a line carrying an inline drawing is measured as if the drawing were not on it**, and can
/// therefore be one object too long. Exactly the shape of R20's footnote-mark gap, and it belongs to
/// the same later child.
#[must_use]
pub fn inline_height(drawings: &[DrawingFormatting], range: std::ops::Range<usize>) -> Emu {
    drawings
        .iter()
        .filter(|drawing| {
            matches!(drawing.placement, DrawingPlacement::Inline(_)) && range.contains(&drawing.at)
        })
        .fold(Emu::ZERO, |tallest, drawing| {
            tallest.maximum(Emu::from_emu(drawing.height))
        })
}
