//! Points, rectangles, affine maps and paths — all in **device pixels**, all `f32`.
//!
//! # Why this is not [`mjx_layout::measure`]
//!
//! A `FragmentTree` is in [`Emu`], an integer unit, because layout
//! *accumulates* and a `f64` page does not land on the number the document says it should. A display
//! list accumulates nothing: it is produced in one pass at one device scale and handed to a painter
//! that will put it in a vertex buffer. So it is in device pixels, `f32`, which is the width of
//! every graphics API this platform targets, and the conversion happens once — at
//! [`pixels_from_emu`], where the rounding is visible.
//!
//! That is also what makes a display list **page-local**: the origin is the page's top-left corner,
//! not the viewport's, so scrolling does not invalidate one and the cache R13 builds can hold a page
//! across a scroll.
//!
//! # Non-finite numbers are normalised, never stored
//!
//! A transform composed out of a document's own numbers can produce an infinity or a `NaN`, and a
//! `NaN` in a display list is worse than a wrong number: it is not equal to itself, so a frame diff
//! would report the same record changed for ever, and a byte-identical rebuild would not be
//! byte-identical. Every `f32` that enters a record goes through [`finite`] first.

use mjx_ooxml_core::measure::Emu;
use mjx_text::DeviceScale;

/// A number fit to store: `0.0` for an infinity or a `NaN`, the number itself otherwise.
///
/// Applied at every boundary where a `f32` enters a record. A caller that wants to know a value was
/// rejected compares before and after; a display list that carried the `NaN` instead would diff
/// against itself for ever.
#[must_use]
pub fn finite(value: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

/// How many device pixels `length` covers at `scale`.
///
/// The one conversion between the layout tier's integer EMU and this tier's floating-point pixels.
/// A scale is pixels per *point* and an EMU is 1/12700 of a point, so the product is the pixel
/// count; the arithmetic is done in `f64` and narrowed once, because an EMU is an `i64` and a page
/// of them does not fit a `f32` exactly.
#[must_use]
pub fn pixels_from_emu(length: Emu, scale: DeviceScale) -> f32 {
    finite((length.points() * f64::from(scale.pixels_per_point())) as f32)
}

/// A position on a page, in device pixels, with `y` increasing downward.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct ScenePoint {
    /// Pixels right of the page's top-left corner.
    pub x: f32,
    /// Pixels below it.
    pub y: f32,
}

impl ScenePoint {
    /// The page's top-left corner.
    pub const ORIGIN: Self = Self { x: 0.0, y: 0.0 };

    /// The point at `(x, y)`, with any non-finite coordinate normalised to zero.
    #[must_use]
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            x: finite(x),
            y: finite(y),
        }
    }
}

/// An axis-aligned rectangle in device pixels, with `y` increasing downward.
///
/// Also the shape a *unit* rectangle takes — a crop, a gradient's fill-to box, a tile rect — where
/// the four numbers are fractions rather than pixels. One type rather than two because the
/// arithmetic is identical and the field that holds it says which it is.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub struct SceneRect {
    /// The left edge.
    pub left: f32,
    /// The top edge, which is the smaller `y`.
    pub top: f32,
    /// The right edge.
    pub right: f32,
    /// The bottom edge, which is the larger `y`.
    pub bottom: f32,
}

impl SceneRect {
    /// Nothing at all, at the origin.
    pub const EMPTY: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    /// The whole of something, as fractions: `0.0` to `1.0` on both axes. The identity crop, the
    /// default fill-to box, and the tile rect of a gradient that does not tile.
    pub const UNIT: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 1.0,
        bottom: 1.0,
    };

    /// A rectangle from its four edges, normalised so that `left <= right` and `top <= bottom` and
    /// every edge is finite.
    #[must_use]
    pub fn new(left: f32, top: f32, right: f32, bottom: f32) -> Self {
        let (left, top, right, bottom) = (finite(left), finite(top), finite(right), finite(bottom));
        Self {
            left: left.min(right),
            top: top.min(bottom),
            right: left.max(right),
            bottom: top.max(bottom),
        }
    }

    /// How wide.
    #[must_use]
    pub fn width(self) -> f32 {
        self.right - self.left
    }

    /// How tall.
    #[must_use]
    pub fn height(self) -> f32 {
        self.bottom - self.top
    }

    /// Whether it encloses no area, so that nothing drawn inside it is visible.
    #[must_use]
    pub fn is_empty(self) -> bool {
        !(self.right > self.left && self.bottom > self.top)
    }

    /// The rectangle grown to contain `point`.
    #[must_use]
    pub fn including(self, point: ScenePoint) -> Self {
        Self {
            left: self.left.min(point.x),
            top: self.top.min(point.y),
            right: self.right.max(point.x),
            bottom: self.bottom.max(point.y),
        }
    }

    /// The same rectangle in device pixels, from one in EMU.
    #[must_use]
    pub fn from_layout(rect: mjx_layout::LayoutRect, scale: DeviceScale) -> Self {
        Self::new(
            pixels_from_emu(rect.left, scale),
            pixels_from_emu(rect.top, scale),
            pixels_from_emu(rect.right, scale),
            pixels_from_emu(rect.bottom, scale),
        )
    }
}

/// An affine map, `x' = a·x + c·y + e` and `y' = b·x + d·y + f`, with the translation in device
/// pixels.
///
/// The same six numbers [`mjx_layout::Transform`] carries, narrowed: a display list's translation is
/// a pixel count rather than an EMU, because everything else in the list already is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct SceneTransform {
    /// `a` — how much of the source `x` reaches the target `x`.
    pub scale_x: f32,
    /// `b` — how much of the source `x` reaches the target `y`.
    pub shear_y: f32,
    /// `c` — how much of the source `y` reaches the target `x`.
    pub shear_x: f32,
    /// `d` — how much of the source `y` reaches the target `y`.
    pub scale_y: f32,
    /// `e` — the translation along `x`, in device pixels.
    pub translate_x: f32,
    /// `f` — the translation along `y`, in device pixels.
    pub translate_y: f32,
}

impl SceneTransform {
    /// The map that changes nothing.
    pub const IDENTITY: Self = Self {
        scale_x: 1.0,
        shear_y: 0.0,
        shear_x: 0.0,
        scale_y: 1.0,
        translate_x: 0.0,
        translate_y: 0.0,
    };

    /// Whether this is the identity, which is what lets a page with no rotation carry no transform
    /// command at all.
    #[must_use]
    pub fn is_identity(self) -> bool {
        self == Self::IDENTITY
    }

    /// The same map in device pixels, from one in EMU.
    ///
    /// The linear part is a ratio and carries no unit, so it narrows unchanged; only the translation
    /// is a length and is converted like every other length here.
    #[must_use]
    pub fn from_layout(transform: mjx_layout::Transform, scale: DeviceScale) -> Self {
        Self {
            scale_x: finite(transform.scale_x as f32),
            shear_y: finite(transform.shear_y as f32),
            shear_x: finite(transform.shear_x as f32),
            scale_y: finite(transform.scale_y as f32),
            translate_x: pixels_from_emu(transform.translate_x, scale),
            translate_y: pixels_from_emu(transform.translate_y, scale),
        }
    }
}

impl Default for SceneTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// One step of a path.
///
/// Quadratic and cubic curves are both here and neither is converted into the other, for the reason
/// [`mjx_text::OutlineCommand`] gives: a TrueType outline is quadratic and a `CFF` one is cubic, and
/// elevating one to the other adds control points the tessellator in R07 then has to flatten more
/// of.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PathCommand {
    /// Begin a new contour here.
    MoveTo(ScenePoint),
    /// A straight line to here.
    LineTo(ScenePoint),
    /// A quadratic curve through one control point to the end point.
    QuadraticTo {
        /// The control point.
        control: ScenePoint,
        /// Where the curve ends.
        end: ScenePoint,
    },
    /// A cubic curve through two control points to the end point.
    CubicTo {
        /// The control point nearer the start.
        first_control: ScenePoint,
        /// The control point nearer the end.
        second_control: ScenePoint,
        /// Where the curve ends.
        end: ScenePoint,
    },
    /// Close the current contour back to where it began.
    Close,
}

/// Which side of a path is inside it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum FillRule {
    /// A point is inside when the winding number is not zero. What DrawingML's custom geometry and
    /// every glyph outline mean.
    #[default]
    NonZero,
    /// A point is inside when the crossing count is odd.
    EvenOdd,
}

/// What a [`crate::Command::FillPath`] or [`crate::Command::StrokePath`] draws.
///
/// Three kinds, and the third is what keeps the display list from having to know what a preset
/// shape is. `docs/UI_PLATFORM_PLAN.md` §4 L4 puts shape geometry behind a `GeometryProvider` seam;
/// a `ShapeFragment` carries an opaque [`mjx_layout::GeometryRef`] and nothing else, so a scene
/// built from fragments alone can say *this outline, at this size* and no more.
///
/// A [`GeometryProvider`](crate::GeometryProvider) is what turns a [`Geometry::Unresolved`] into a
/// [`Geometry::Path`], and since MJXOFF-206 the one a document is rendered with is
/// `mjx-geometry`'s `PresetGeometryProvider` — which this crate may not name, because 2.5 is above
/// 1.7 and that is the seam working. A painter handed a list nobody resolved still meets an
/// `Unresolved`, and can say so about it rather than drawing nothing.
#[derive(Clone, PartialEq, Debug)]
pub enum Geometry {
    /// A rectangle — a box's background, a table cell, a clip.
    Rectangle(SceneRect),
    /// An explicit path, with the box that contains it.
    Path {
        /// The steps, in order.
        commands: Vec<PathCommand>,
        /// Which side is inside.
        fill_rule: FillRule,
        /// The smallest rectangle containing every point named, for a viewport cull that must not
        /// walk the path to decide.
        bounds: SceneRect,
    },
    /// An outline a geometry provider has yet to resolve: the handle a box model issued, and the
    /// box it is to be drawn at.
    Unresolved {
        /// The handle, as [`mjx_layout::GeometryRef::number`] gave it.
        outline: u64,
        /// The rectangle the outline is resolved against.
        bounds: SceneRect,
    },
}

impl Geometry {
    /// A path, with its bounds computed from the points it names.
    ///
    /// A control point counts: the box a curve is *drawn* inside is contained by the box its control
    /// polygon spans, so a cull that used the tighter box would be wrong in the direction that
    /// loses pixels, and one that used this box is only ever conservative.
    #[must_use]
    pub fn path(commands: Vec<PathCommand>, fill_rule: FillRule) -> Self {
        let mut bounds: Option<SceneRect> = None;
        let mut include = |point: ScenePoint| {
            bounds = Some(match bounds {
                None => SceneRect {
                    left: point.x,
                    top: point.y,
                    right: point.x,
                    bottom: point.y,
                },
                Some(box_so_far) => box_so_far.including(point),
            });
        };
        for command in &commands {
            match *command {
                PathCommand::MoveTo(point) | PathCommand::LineTo(point) => include(point),
                PathCommand::QuadraticTo { control, end } => {
                    include(control);
                    include(end);
                }
                PathCommand::CubicTo {
                    first_control,
                    second_control,
                    end,
                } => {
                    include(first_control);
                    include(second_control);
                    include(end);
                }
                PathCommand::Close => {}
            }
        }
        Self::Path {
            commands,
            fill_rule,
            bounds: bounds.unwrap_or(SceneRect::EMPTY),
        }
    }

    /// The box that certainly contains whatever this draws.
    #[must_use]
    pub fn bounds(&self) -> SceneRect {
        match self {
            Self::Rectangle(rect) => *rect,
            Self::Path { bounds, .. } | Self::Unresolved { bounds, .. } => *bounds,
        }
    }
}
