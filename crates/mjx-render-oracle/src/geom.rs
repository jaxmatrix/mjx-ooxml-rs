//! Rectangles, in the two coordinate systems a comparison has to hold at once.
//!
//! # Why this is here rather than in a caller
//!
//! A comparison crops the same rectangle out of two rasters and asks whether they agree. If the two
//! sides are positioned by two pieces of arithmetic, a difference between them is indistinguishable
//! from a difference between the renderers — and it shows up as *every* window disagreeing slightly,
//! which reads exactly like a systematic rendering defect. So there is one [`Rect`], one conversion
//! into the display list's coordinates, and one conversion into raster pixels.
//!
//! # The units
//!
//! **One display-list unit is one PDF point.** `PdfPainter` writes a `/MediaBox` in points and
//! PowerPoint's own export of a 13⅓ × 7½ inch slide writes the same box, so `pdftoppm` at a stated
//! DPI rasterises both sides with **one rasteriser** — the only construction in which *"pixel
//! perfect against PowerPoint"* is a coherent phrase.
//!
//! This module was `mjx-reference-pack`'s `layout::Rect` until MJXOFF-165 gave the pack a crate
//! below it; the conversion into a `.pptx` shape's EMU bounds stayed there, on an extension trait,
//! because a rectangle is a general thing and `mjx_pptx::ShapeBounds` is not.

/// EMU in one PDF point.
pub const EMU_PER_POINT: i64 = 12_700;

/// A rectangle in points, with its top-left at (`x`, `y`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rect {
    /// Points from the left edge of the page.
    pub x: i64,
    /// Points from the top edge of the page.
    pub y: i64,
    /// Width in points.
    pub width: i64,
    /// Height in points.
    pub height: i64,
}

impl Rect {
    /// A rectangle from its four numbers.
    #[must_use]
    pub const fn new(x: i64, y: i64, width: i64, height: i64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The same rectangle in the display list's coordinates, which are points.
    #[must_use]
    pub fn to_scene_rect(self) -> mjx_scene::SceneRect {
        #[allow(
            clippy::cast_precision_loss,
            reason = "every field is a small integer number of points; 960 is exact in f32"
        )]
        mjx_scene::SceneRect::new(
            self.x as f32,
            self.y as f32,
            (self.x + self.width) as f32,
            (self.y + self.height) as f32,
        )
    }

    /// The pixel rectangle this becomes when a page is rasterised at `dots_per_inch`, clamped to
    /// `(width, height)`.
    ///
    /// Rounded **inwards** — the left and top edges up, the right and bottom edges down — so a
    /// window never reaches a pixel that belongs to a neighbouring plate. A crop that borrowed one
    /// column from the plate beside it would report a difference that is a cropping error.
    #[must_use]
    pub fn to_pixels(self, dots_per_inch: f64, width: u32, height: u32) -> PixelRect {
        let scale = dots_per_inch / 72.0;
        #[allow(
            clippy::cast_precision_loss,
            reason = "point coordinates on a 960-point page"
        )]
        let (left, top) = (self.x as f64 * scale, self.y as f64 * scale);
        #[allow(
            clippy::cast_precision_loss,
            reason = "point coordinates on a 960-point page"
        )]
        let (right, bottom) = (
            (self.x + self.width) as f64 * scale,
            (self.y + self.height) as f64 * scale,
        );
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to the raster's own size immediately below"
        )]
        let (x0, y0) = (left.ceil().max(0.0) as u32, top.ceil().max(0.0) as u32);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "clamped to the raster's own size immediately below"
        )]
        let (x1, y1) = (
            right.floor().max(0.0) as u32,
            bottom.floor().max(0.0) as u32,
        );
        PixelRect {
            x: x0.min(width),
            y: y0.min(height),
            width: x1.min(width).saturating_sub(x0.min(width)),
            height: y1.min(height).saturating_sub(y0.min(height)),
        }
    }
}

/// A rectangle in raster pixels.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PixelRect {
    /// Pixels from the left edge.
    pub x: u32,
    /// Pixels from the top.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PixelRect {
    /// How many pixels the rectangle covers.
    #[must_use]
    pub const fn area(self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}
