//! Shape position and size on a slide, in English Metric Units (EMU).
//!
//! DrawingML measures lengths in EMU: 914 400 per inch, 12 700 per point (72 points to the inch). A
//! shape's placement is an offset (`a:off`) plus an extent (`a:ext`); [`ShapeBounds`] names those four
//! numbers so callers never touch the raw `x`/`y`/`cx`/`cy` wire attributes. [`SlideSize`] is the
//! extent those bounds sit inside.

use mjx_dml::{Emu, Position, Size, Transform2D};
use mjx_ooxml_types::presentationml::SlideSizeKind;

/// A shape's position and size on a slide, in English Metric Units (914 400 EMU = 1 inch).
///
/// Maps to DrawingML's `a:xfrm`: `offset_x_emu` / `offset_y_emu` are the `a:off` `x` / `y`, and
/// `width_emu` / `height_emu` are the `a:ext` `cx` / `cy`. Offsets may be negative (a shape can sit
/// partly off-slide); extents are expected to be non-negative but this type does not enforce it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapeBounds {
    /// Horizontal offset of the shape's top-left corner from the slide origin (`a:off` `x`), in EMU.
    pub offset_x_emu: i64,
    /// Vertical offset of the shape's top-left corner from the slide origin (`a:off` `y`), in EMU.
    pub offset_y_emu: i64,
    /// Width of the shape (`a:ext` `cx`), in EMU.
    pub width_emu: i64,
    /// Height of the shape (`a:ext` `cy`), in EMU.
    pub height_emu: i64,
}

impl ShapeBounds {
    /// English Metric Units per inch.
    pub const EMU_PER_INCH: i64 = 914_400;
    /// English Metric Units per point (72 points to the inch).
    pub const EMU_PER_POINT: i64 = 12_700;

    /// Bounds from raw EMU values.
    #[must_use]
    pub fn new(offset_x_emu: i64, offset_y_emu: i64, width_emu: i64, height_emu: i64) -> Self {
        Self {
            offset_x_emu,
            offset_y_emu,
            width_emu,
            height_emu,
        }
    }

    /// Bounds from inches, each dimension rounded to the nearest EMU.
    #[must_use]
    pub fn from_inches(x: f64, y: f64, width: f64, height: f64) -> Self {
        let to_emu = |inches: f64| (inches * Self::EMU_PER_INCH as f64).round() as i64;
        Self {
            offset_x_emu: to_emu(x),
            offset_y_emu: to_emu(y),
            width_emu: to_emu(width),
            height_emu: to_emu(height),
        }
    }

    /// The smallest rectangle containing both — what a group's box is, since ECMA-376 Part 1
    /// §L.4.7.4 defines a group's child bounding box as the union of its members' boxes taken before
    /// their individual rotations.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        let left = self.offset_x_emu.min(other.offset_x_emu);
        let top = self.offset_y_emu.min(other.offset_y_emu);
        let right = (self.offset_x_emu + self.width_emu).max(other.offset_x_emu + other.width_emu);
        let bottom =
            (self.offset_y_emu + self.height_emu).max(other.offset_y_emu + other.height_emu);
        Self {
            offset_x_emu: left,
            offset_y_emu: top,
            width_emu: right - left,
            height_emu: bottom - top,
        }
    }

    /// The bounds a [`Transform2D`] describes, or `None` unless it carries **both** an `a:off` and an
    /// `a:ext` — bounds are all four numbers, and a transform that names only one of the two does not
    /// place the shape on its own.
    #[must_use]
    pub fn from_transform(transform: &Transform2D) -> Option<Self> {
        let position = transform.position?;
        let size = transform.size?;
        Some(Self {
            offset_x_emu: position.x.emu(),
            offset_y_emu: position.y.emu(),
            width_emu: size.width.emu(),
            height_emu: size.height.emu(),
        })
    }

    /// These bounds as a [`Transform2D`] that names **only** position and size.
    ///
    /// Everything else is left unset, which is what makes setting bounds non-destructive: applying
    /// this transform moves and resizes a shape without disturbing its rotation, its flips, or the
    /// child coordinate space a group's members are laid out in.
    #[must_use]
    pub fn to_transform(self) -> Transform2D {
        Transform2D {
            position: Some(Position::from_emu(self.offset_x_emu, self.offset_y_emu)),
            size: Some(Size::from_emu(self.width_emu, self.height_emu)),
            ..Transform2D::default()
        }
    }
}

/// The four insets between a table cell's edges and its text (`a:tcPr`'s `@marL` / `@marR` /
/// `@marT` / `@marB`).
///
/// Every field is `Option`, and `None` means the cell does not state that margin — **not** that it
/// is zero. The schema defaults are non-zero (`0.1"` horizontally, `0.05"` vertically, exposed as
/// [`TableCellProperties::DEFAULT_MARGIN_HORIZONTAL`](mjx_dml::TableCellProperties::DEFAULT_MARGIN_HORIZONTAL)
/// and its vertical counterpart), so collapsing the two would silently shrink every cell it touched.
///
/// On write, a `None` field is left exactly as it was, so one margin can be set without restating
/// the other three.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CellMargins {
    /// The left inset (`@marL`).
    pub left: Option<Emu>,
    /// The right inset (`@marR`).
    pub right: Option<Emu>,
    /// The top inset (`@marT`).
    pub top: Option<Emu>,
    /// The bottom inset (`@marB`).
    pub bottom: Option<Emu>,
}

impl CellMargins {
    /// The same inset on all four sides.
    #[must_use]
    pub fn uniform(margin: Emu) -> Self {
        Self {
            left: Some(margin),
            right: Some(margin),
            top: Some(margin),
            bottom: Some(margin),
        }
    }
}

/// The size of every slide in a deck (`p:sldSz`), in English Metric Units.
///
/// Shape bounds are absolute within this extent, so it is what a layout computation measures
/// against: a full-width shape on a 4:3 deck is `width_emu` wide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlideSize {
    /// Slide width (`p:sldSz@cx`), in EMU.
    pub width_emu: i64,
    /// Slide height (`p:sldSz@cy`), in EMU.
    pub height_emu: i64,
    /// What the size is optimized for (`p:sldSz@type`); [`SlideSizeKind::Custom`] when unstated.
    pub kind: SlideSizeKind,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_inches_rounds_to_nearest_emu() {
        let bounds = ShapeBounds::from_inches(1.0, 2.0, 4.0, 0.5);
        assert_eq!(bounds.offset_x_emu, 914_400);
        assert_eq!(bounds.offset_y_emu, 1_828_800);
        assert_eq!(bounds.width_emu, 3_657_600);
        assert_eq!(bounds.height_emu, 457_200);
    }

    #[test]
    fn new_is_verbatim() {
        let bounds = ShapeBounds::new(-5, 0, 100, 200);
        assert_eq!(
            (
                bounds.offset_x_emu,
                bounds.offset_y_emu,
                bounds.width_emu,
                bounds.height_emu
            ),
            (-5, 0, 100, 200)
        );
    }
}
