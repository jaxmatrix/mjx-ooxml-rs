//! Shape position and size on a slide, in English Metric Units (EMU).
//!
//! DrawingML measures lengths in EMU: 914 400 per inch, 12 700 per point (72 points to the inch). A
//! shape's placement is an offset (`a:off`) plus an extent (`a:ext`); [`ShapeBounds`] names those four
//! numbers so callers never touch the raw `x`/`y`/`cx`/`cy` wire attributes.

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
