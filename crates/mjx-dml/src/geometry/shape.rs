//! The typed shape-geometry tier: [`ShapeGeometry`] and [`PresetGeometry::shape`] / [`set_shape`].
//!
//! Each variant names a preset shape's adjustments in friendly units (a [`Fraction`]), converted from
//! the mechanical tier's native values. Meanings/units are **derived from `presetShapeDefinitions.xml`**
//! (see the derivation table in the batch plan), never guessed; shapes not yet ported fall through to
//! [`ShapeGeometry::Unmodeled`], so the type is total and still fully round-trips via the fidelity model.
//!
//! [`set_shape`]: PresetGeometry::set_shape

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::drawingml::PresetShapeType;

use super::measures::Fraction;
use super::PresetGeometry;

/// Native denominator for a fraction of the shorter side / width / height (1000ths of a percent).
const SHORTER_SIDE_DENOM: i32 = 100_000;
/// Native denominator for a star's inner-radius fraction (the spec computes the inner vertex as
/// `a / 50000` of the outer point radius).
const STAR_DENOM: i32 = 50_000;

/// A preset shape with its adjustments named in friendly units.
///
/// Returned by [`PresetGeometry::shape`] and consumed by [`PresetGeometry::set_shape`]. Every field is
/// a [`Fraction`] whose meaning is the field's name (e.g. a corner radius as a fraction of the shorter
/// side). Only shapes ported to the typed tier get a dedicated variant; every other preset is
/// [`Unmodeled`](Self::Unmodeled) and can still be read/set by wire name via the mechanical API.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeGeometry {
    /// `roundRect` — a rectangle with all four corners rounded. `corner_radius` is a fraction of the
    /// shorter side (`adj`; `x1 = */ ss a 100000`, drawn as `arcTo wR=x1 hR=x1`).
    RoundedRectangle {
        /// Corner arc radius, as a fraction of the shorter side (0..≈0.5).
        corner_radius: Fraction,
    },
    /// `round1Rect` — a rectangle with one corner rounded. `corner_radius` is a fraction of the
    /// shorter side (`adj`).
    RoundSingleCornerRectangle {
        /// Corner arc radius, as a fraction of the shorter side (0..≈0.5).
        corner_radius: Fraction,
    },
    /// `snip1Rect` — a rectangle with one corner snipped (straight cut). `snip_size` is a fraction of
    /// the shorter side (`adj`; `dx1 = */ ss a 100000`).
    SnipSingleCornerRectangle {
        /// Length of the snipped corner cut, as a fraction of the shorter side (0..≈0.5).
        snip_size: Fraction,
    },
    /// `octagon` — a rectangle with all corners chamfered. `corner_cut` is a fraction of the shorter
    /// side (`adj`; `x1 = */ ss a 100000`).
    Octagon {
        /// Length of each chamfer, as a fraction of the shorter side (0..≈0.5).
        corner_cut: Fraction,
    },
    /// `plaque` — a rectangle with concave (inward-arced) corners. `corner_size` is a fraction of the
    /// shorter side (`adj`; `x1 = */ ss a 100000`).
    Plaque {
        /// Size of each concave corner, as a fraction of the shorter side (0..≈0.5).
        corner_size: Fraction,
    },
    /// `foldedCorner` — a rectangle with a folded (dog-ear) bottom-right corner. `fold_size` is a
    /// fraction of the shorter side (`adj`; `dy2 = */ ss a 100000`).
    FoldedCorner {
        /// Size of the folded corner, as a fraction of the shorter side (0..≈0.5).
        fold_size: Fraction,
    },
    /// `frame` — a rectangular picture frame. `border_thickness` is a fraction of the shorter side
    /// (`adj1`; `x1 = */ ss a1 100000`).
    Frame {
        /// Width of the frame border, as a fraction of the shorter side (0..≈0.5).
        border_thickness: Fraction,
    },
    /// `star4` — a 4-point star. `inner_radius` is the inner-vertex radius as a fraction of the outer
    /// point radius (`adj`; `iwd2 = */ wd2 a 50000`).
    FourPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star5` — a 5-point star. `inner_radius` is a fraction of the outer point radius.
    FivePointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star6` — a 6-point star. `inner_radius` is a fraction of the outer point radius.
    SixPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star7` — a 7-point star. `inner_radius` is a fraction of the outer point radius.
    SevenPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star8` — an 8-point star. `inner_radius` is a fraction of the outer point radius.
    EightPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star10` — a 10-point star. `inner_radius` is a fraction of the outer point radius.
    TenPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star12` — a 12-point star. `inner_radius` is a fraction of the outer point radius.
    TwelvePointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star16` — a 16-point star. `inner_radius` is a fraction of the outer point radius.
    SixteenPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star24` — a 24-point star. `inner_radius` is a fraction of the outer point radius.
    TwentyFourPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// `star32` — a 32-point star. `inner_radius` is a fraction of the outer point radius.
    ThirtyTwoPointStar {
        /// Inner-vertex radius as a fraction of the outer point radius (0..1).
        inner_radius: Fraction,
    },
    /// A known preset this tier does not yet model. Its adjustments (if any) remain available by wire
    /// name through [`PresetGeometry::adjustment`] / [`set_adjustment`](PresetGeometry::set_adjustment).
    Unmodeled(PresetShapeType),
}

impl PresetGeometry {
    /// The shape as a typed [`ShapeGeometry`] with its adjustments in friendly units, or `None` if the
    /// `prst` is absent or names a token this build does not know.
    ///
    /// Adjustment values are the `avLst` override if present, else the shape's spec default.
    #[must_use]
    pub fn shape(&self, interner: &Interner) -> Option<ShapeGeometry> {
        let preset = self.preset(interner)?;
        Some(match preset {
            PresetShapeType::RoundedRectangle => ShapeGeometry::RoundedRectangle {
                corner_radius: self.fraction(interner, "adj", SHORTER_SIDE_DENOM),
            },
            PresetShapeType::RoundSingleCornerRectangle => {
                ShapeGeometry::RoundSingleCornerRectangle {
                    corner_radius: self.fraction(interner, "adj", SHORTER_SIDE_DENOM),
                }
            }
            PresetShapeType::SnipSingleCornerRectangle => {
                ShapeGeometry::SnipSingleCornerRectangle {
                    snip_size: self.fraction(interner, "adj", SHORTER_SIDE_DENOM),
                }
            }
            PresetShapeType::Octagon => ShapeGeometry::Octagon {
                corner_cut: self.fraction(interner, "adj", SHORTER_SIDE_DENOM),
            },
            PresetShapeType::Plaque => ShapeGeometry::Plaque {
                corner_size: self.fraction(interner, "adj", SHORTER_SIDE_DENOM),
            },
            PresetShapeType::FoldedCorner => ShapeGeometry::FoldedCorner {
                fold_size: self.fraction(interner, "adj", SHORTER_SIDE_DENOM),
            },
            PresetShapeType::Frame => ShapeGeometry::Frame {
                border_thickness: self.fraction(interner, "adj1", SHORTER_SIDE_DENOM),
            },
            PresetShapeType::FourPointStar => ShapeGeometry::FourPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::FivePointStar => ShapeGeometry::FivePointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::SixPointStar => ShapeGeometry::SixPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::SevenPointStar => ShapeGeometry::SevenPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::EightPointStar => ShapeGeometry::EightPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::TenPointStar => ShapeGeometry::TenPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::TwelvePointStar => ShapeGeometry::TwelvePointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::SixteenPointStar => ShapeGeometry::SixteenPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::TwentyFourPointStar => ShapeGeometry::TwentyFourPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            PresetShapeType::ThirtyTwoPointStar => ShapeGeometry::ThirtyTwoPointStar {
                inner_radius: self.fraction(interner, "adj", STAR_DENOM),
            },
            other => ShapeGeometry::Unmodeled(other),
        })
    }

    /// Sets the shape from a typed [`ShapeGeometry`]: rewrites `prst` and each named adjustment (in the
    /// `avLst`, creating it if needed), converting friendly units back to native. `Unmodeled` sets only
    /// the preset.
    pub fn set_shape(&mut self, interner: &mut Interner, shape: ShapeGeometry) {
        match shape {
            ShapeGeometry::RoundedRectangle { corner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::RoundedRectangle,
                    "adj",
                    corner_radius,
                    SHORTER_SIDE_DENOM,
                );
            }
            ShapeGeometry::RoundSingleCornerRectangle { corner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::RoundSingleCornerRectangle,
                    "adj",
                    corner_radius,
                    SHORTER_SIDE_DENOM,
                );
            }
            ShapeGeometry::SnipSingleCornerRectangle { snip_size } => {
                self.apply(
                    interner,
                    PresetShapeType::SnipSingleCornerRectangle,
                    "adj",
                    snip_size,
                    SHORTER_SIDE_DENOM,
                );
            }
            ShapeGeometry::Octagon { corner_cut } => {
                self.apply(
                    interner,
                    PresetShapeType::Octagon,
                    "adj",
                    corner_cut,
                    SHORTER_SIDE_DENOM,
                );
            }
            ShapeGeometry::Plaque { corner_size } => {
                self.apply(
                    interner,
                    PresetShapeType::Plaque,
                    "adj",
                    corner_size,
                    SHORTER_SIDE_DENOM,
                );
            }
            ShapeGeometry::FoldedCorner { fold_size } => {
                self.apply(
                    interner,
                    PresetShapeType::FoldedCorner,
                    "adj",
                    fold_size,
                    SHORTER_SIDE_DENOM,
                );
            }
            ShapeGeometry::Frame { border_thickness } => {
                self.apply(
                    interner,
                    PresetShapeType::Frame,
                    "adj1",
                    border_thickness,
                    SHORTER_SIDE_DENOM,
                );
            }
            ShapeGeometry::FourPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::FourPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::FivePointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::FivePointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::SixPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::SixPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::SevenPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::SevenPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::EightPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::EightPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::TenPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::TenPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::TwelvePointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::TwelvePointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::SixteenPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::SixteenPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::TwentyFourPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::TwentyFourPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::ThirtyTwoPointStar { inner_radius } => {
                self.apply(
                    interner,
                    PresetShapeType::ThirtyTwoPointStar,
                    "adj",
                    inner_radius,
                    STAR_DENOM,
                );
            }
            ShapeGeometry::Unmodeled(preset) => self.set_preset(interner, preset),
        }
    }

    /// Reads adjustment `wire` (override or default) and converts it to a [`Fraction`] over `denom`.
    fn fraction(&self, interner: &Interner, wire: &str, denom: i32) -> Fraction {
        let native = self.adjustment(interner, wire).unwrap_or(0);
        Fraction::from_ratio(f64::from(native) / f64::from(denom))
    }

    /// Sets `preset` and writes `value` (a fraction over `denom`) to adjustment `wire` in native units.
    fn apply(
        &mut self,
        interner: &mut Interner,
        preset: PresetShapeType,
        wire: &str,
        value: Fraction,
        denom: i32,
    ) {
        self.set_preset(interner, preset);
        let native = (value.ratio() * f64::from(denom)).round() as i32;
        self.set_adjustment(interner, wire, native);
    }
}
