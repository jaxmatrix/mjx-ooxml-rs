//! The plate grid, stated once.
//!
//! # Why this is a module and not two sets of constants
//!
//! A comparison crops the same rectangle out of two rasters and asks whether they agree. If the
//! deck's plate and our own display list's plate are laid out by two pieces of arithmetic, a
//! difference between them is indistinguishable from a difference between the renderers — and it
//! would show up as *every* plate disagreeing slightly, which reads exactly like a systematic
//! rendering defect. So the deck writer and the scene builder call the same functions here, and the
//! crop window comes from the same place again.
//!
//! # The units
//!
//! **One display-list unit is one PDF point**, and one point is 12 700 EMU. So the slide is 960 × 540
//! points, our own offscreen surface is 960 × 540 at scale 1, `PdfPainter` writes
//! `/MediaBox [0 0 960 540]`, and PowerPoint's own export of a 13⅓ × 7½ inch slide writes the same
//! box. `pdftoppm` at a stated DPI then rasterises both sides with **one rasteriser**, which is the
//! only construction in which *"pixel perfect against PowerPoint"* is a coherent phrase.
//!
//! Every number below is an integer number of points, so every EMU conversion is exact and no plate
//! lands half a unit away from where the other side put it.
//!
//! # Where the label goes, and why the crop excludes it
//!
//! Each plate carries its preset's wire token, in a text box **below** the shape rather than inside
//! it: text inside an autoshape is text PowerPoint may autofit, and a shape whose geometry is being
//! compared must not also be a shape whose text engine is being exercised.
//!
//! Our own side draws no text at all — this crate builds outlines, not paragraphs — so the label
//! strip is the one region of a plate the two sides are *known* to differ over. It is therefore
//! outside [`PlateGeometry::window`], which is the rectangle a comparison crops. An excluded region
//! that a reader can see in the arithmetic is a known limitation; one that only shows up as a
//! constant per-plate difference is a mystery.

// `Rect` and `PixelRect` moved down into `mjx-render-oracle` with MJXOFF-165, because the fidelity
// oracle compares rasters too and a second rectangle would be a second rounding rule. What stayed
// here is the half that names a `.pptx`: `ShapeBounds` is `mjx-pptx`'s, and a rectangle is a general
// thing, so the conversion is an extension trait on this side of the seam rather than a dependency
// on the format tier from the other.
pub use mjx_render_oracle::geom::{PixelRect, Rect, EMU_PER_POINT};

/// The same rectangle as a `.pptx` shape's bounds, in EMU.
pub trait ToShapeBounds {
    /// Exact: every field is an integer number of points.
    fn to_shape_bounds(self) -> mjx_pptx::ShapeBounds;
}

impl ToShapeBounds for Rect {
    fn to_shape_bounds(self) -> mjx_pptx::ShapeBounds {
        mjx_pptx::ShapeBounds {
            offset_x_emu: self.x * EMU_PER_POINT,
            offset_y_emu: self.y * EMU_PER_POINT,
            width_emu: self.width * EMU_PER_POINT,
            height_emu: self.height * EMU_PER_POINT,
        }
    }
}

/// The slide, in points: 13⅓ × 7½ inches, PowerPoint's 16 : 9 default.
pub const SLIDE: (i64, i64) = (960, 540);

/// The strip at the top of every plate slide that carries its heading.
pub const HEADING_HEIGHT: i64 = 36;

/// Plates across a slide.
pub const COLUMNS: i64 = 6;

/// Plates down a slide.
pub const ROWS: i64 = 4;

/// Plates on one slide.
pub const PLATES_PER_SLIDE: usize = (COLUMNS * ROWS) as usize;

/// One plate's cell, in points.
pub const CELL: (i64, i64) = (SLIDE.0 / COLUMNS, (SLIDE.1 - HEADING_HEIGHT) / ROWS);

/// The strip at the bottom of a cell the caption sits in. **Outside the comparison window.**
pub const LABEL_HEIGHT: i64 = 20;

/// How far the comparison window is inset from the cell's left and right edges, so two neighbouring
/// plates never share a pixel.
pub const WINDOW_INSET_X: i64 = 4;

/// How far the window is inset from the cell's top edge.
pub const WINDOW_INSET_Y: i64 = 3;

/// The shape's own box, in points. Deliberately **not square** — 100 × 70 — because a square box
/// hides an axis swap: a shape drawn with its width and height exchanged would land exactly where
/// the right one does.
pub const SHAPE_BOX: (i64, i64) = (100, 70);

/// Where the plates of one page sit.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PlateGeometry {
    /// Which plate on its page, `0..`[`PLATES_PER_SLIDE`].
    pub position: usize,
}

impl PlateGeometry {
    /// The `position`-th plate of a page.
    ///
    /// # Panics
    ///
    /// If `position` is not a position on a page. Every caller derives it from
    /// `index % PLATES_PER_SLIDE`, so this is a programming error rather than an input.
    #[must_use]
    pub fn at(position: usize) -> Self {
        assert!(
            position < PLATES_PER_SLIDE,
            "plate position {position} is off the page, which has {PLATES_PER_SLIDE} of them"
        );
        Self { position }
    }

    /// The whole cell the plate owns.
    #[must_use]
    pub fn cell(self) -> Rect {
        #[allow(
            clippy::cast_possible_wrap,
            reason = "a position is bounded by PLATES_PER_SLIDE"
        )]
        let position = self.position as i64;
        let (column, row) = (position % COLUMNS, position / COLUMNS);
        Rect::new(
            column * CELL.0,
            HEADING_HEIGHT + row * CELL.1,
            CELL.0,
            CELL.1,
        )
    }

    /// The shape's own box — what `a:ext` states and what the outline is resolved against.
    #[must_use]
    pub fn shape_box(self) -> Rect {
        let cell = self.cell();
        let window = self.window();
        Rect::new(
            cell.x + (CELL.0 - SHAPE_BOX.0) / 2,
            window.y + (window.height - SHAPE_BOX.1) / 2,
            SHAPE_BOX.0,
            SHAPE_BOX.1,
        )
    }

    /// The rectangle a comparison crops: the cell, inset, **without the caption strip**.
    #[must_use]
    pub fn window(self) -> Rect {
        let cell = self.cell();
        Rect::new(
            cell.x + WINDOW_INSET_X,
            cell.y + WINDOW_INSET_Y,
            CELL.0 - 2 * WINDOW_INSET_X,
            CELL.1 - LABEL_HEIGHT - WINDOW_INSET_Y,
        )
    }

    /// Where the caption goes — the strip [`window`](Self::window) deliberately excludes.
    #[must_use]
    pub fn caption(self) -> Rect {
        let cell = self.cell();
        Rect::new(
            cell.x + WINDOW_INSET_X,
            cell.y + CELL.1 - LABEL_HEIGHT,
            CELL.0 - 2 * WINDOW_INSET_X,
            LABEL_HEIGHT,
        )
    }
}

/// How many pages `count` plates need.
#[must_use]
pub fn pages_for(count: usize) -> usize {
    count.div_ceil(PLATES_PER_SLIDE)
}

/// The `.pptx` slide size these constants describe.
#[must_use]
pub fn slide_size() -> mjx_pptx::SlideSize {
    mjx_pptx::SlideSize {
        width_emu: SLIDE.0 * EMU_PER_POINT,
        height_emu: SLIDE.1 * EMU_PER_POINT,
        kind: mjx_ooxml_types::presentationml::SlideSizeKind::Screen16X9,
    }
}
