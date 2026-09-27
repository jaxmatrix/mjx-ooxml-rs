//! What fills a shape and what strokes it — a neutral vocabulary with DrawingML's semantics in it.
//!
//! # Neutral, but not impoverished
//!
//! `mjx-dml`'s `fill.rs`, `line.rs` and `effect.rs` are where the semantics come from, and none of
//! their types appear here: this crate sits at rank 1.7 and `mjx-dml` at 2.0, so the edge points
//! straight up and `xtask/tests/layering.rs` refuses it by name — and, more to the point, a painter
//! that had to understand `a:gradFill` would
//! be a painter that could only ever draw OOXML. What is reproduced is the *meaning* — a gradient's
//! stops and its tile rectangle, a pattern's 54 presets, a picture fill's crop and its adjustments,
//! a stroke's compound type and its two ends — under names a reader does not need the spec for.
//!
//! Where a name is not inferable from DrawingML's own token, it is taken from the ECMA-376 Part 1
//! prose, which is the same rule `CLAUDE.md` sets for the generated types.
//!
//! # Two vocabularies, and why
//!
//! A *style* ([`FillStyle`], [`StrokeStyle`], [`Decoration`](crate::Decoration)) owns everything it
//! needs and names no table. It is what a caller writes and what a [`crate::ResourceResolver`]
//! returns, because a resolver that had to hand back indices would have to be handed the builder.
//!
//! A *record* ([`Paint`], [`Stroke`], [`Gradient`], [`Image`]) is what a display list stores, and it
//! addresses the other tables by index, because that is what makes a fill repeated on four hundred
//! shapes cost four hundred integers. [`crate::SceneBuilder::add_fill_style`] is the bridge.
//!
//! # Nothing here needs a compute shader
//!
//! Every construct in this module is drawable by a fragment shader over tessellated triangles on
//! WebGL2 — which is the constraint `docs/UI_PLATFORM_PLAN.md` §4 L5 says decides the whole
//! pipeline. Gradients are a ramp texture and a coordinate, patterns are a 8×8 mask, a blip fill is
//! a sampler with a wrap mode, and the image adjustments are per-fragment arithmetic. A construct
//! that needed a compute pass would be one the browser could not draw, and there is none.

use mjx_tokens::Color;

use crate::encoding::ResourceIndex;
use crate::geometry::{finite, ScenePoint, SceneRect};

/// How a gradient's ramp is laid over the shape.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum GradientKind {
    /// `a:lin` — the ramp runs along a straight line at [`Gradient::angle`].
    #[default]
    Linear,
    /// `a:path path="circle"` — the ramp runs outward from [`Gradient::focus`].
    Radial,
    /// `a:path path="shape"` or `"rect"` — the ramp follows the shape's own outline inward.
    Path,
}

/// Which outline a [`GradientKind::Path`] gradient follows inward.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PathShade {
    /// `shape` — the shape's own geometry.
    #[default]
    Shape,
    /// `circle` — a circle inscribed in the shape's box.
    Circle,
    /// `rect` — the shape's bounding rectangle.
    Rectangle,
}

/// How a gradient or a tiled image repeats past its own rectangle.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TileFlip {
    /// `none` — every repeat is a copy.
    #[default]
    None,
    /// `x` — alternate columns are mirrored.
    Horizontal,
    /// `y` — alternate rows are mirrored.
    Vertical,
    /// `xy` — both.
    Both,
}

/// One stop of a gradient: where along the ramp, and what colour there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GradientStop {
    /// Where along the ramp, as a fraction from `0.0` to `1.0`.
    ///
    /// Stored as a `u16` count of ten-thousandths rather than a `f32` so that a stop written at
    /// exactly 50% is exactly 50% in every list that carries it — which is what lets two frames
    /// that did not change the gradient compare equal byte for byte.
    pub position_in_ten_thousandths: u16,
    /// The colour at that stop.
    pub color: Color,
}

impl GradientStop {
    /// A stop of `color` at `position`, a fraction from `0.0` to `1.0`, clamped and quantised.
    #[must_use]
    pub fn new(position: f32, color: Color) -> Self {
        let clamped = finite(position).clamp(0.0, 1.0);
        Self {
            // Rounded, not truncated: 0.9999 is a stop at the far end, not one just short of it.
            position_in_ten_thousandths: (clamped * 10_000.0).round() as u16,
            color,
        }
    }

    /// Where along the ramp, as a fraction.
    #[must_use]
    pub fn position(self) -> f32 {
        f32::from(self.position_in_ten_thousandths) / 10_000.0
    }
}

/// A gradient, with DrawingML's full stop and tile semantics.
#[derive(Clone, PartialEq, Debug)]
pub struct Gradient {
    /// Linear, radial or path.
    pub kind: GradientKind,
    /// The stops, in ramp order.
    pub stops: Vec<GradientStop>,
    /// `a:lin@ang` — the direction of a linear ramp, in radians clockwise from the positive `x`
    /// axis, because `y` grows downward here.
    pub angle: f32,
    /// `a:lin@scaled` — whether the angle is measured in the shape's own aspect ratio rather than
    /// in a square. The difference is visible on any shape that is not square, which is most.
    pub angle_is_scaled: bool,
    /// Which outline a path gradient follows.
    pub path_shade: PathShade,
    /// `a:path > a:fillToRect` — the rectangle the ramp converges on, in fractions of the shape's
    /// box. For a radial gradient this is the focus.
    pub focus: SceneRect,
    /// `a:tileRect` — the rectangle the ramp is laid into before it repeats, in fractions of the
    /// shape's box. [`SceneRect::UNIT`] for a gradient that fills the shape once.
    pub tile: SceneRect,
    /// How the repeats are mirrored.
    pub flip: TileFlip,
    /// `@rotWithShape` — whether the ramp turns with the shape or stays fixed to the page.
    pub rotate_with_shape: bool,
}

impl Gradient {
    /// A linear gradient through `stops` at `angle` radians, filling the shape once.
    #[must_use]
    pub fn linear(stops: Vec<GradientStop>, angle: f32) -> Self {
        Self {
            kind: GradientKind::Linear,
            stops,
            angle: finite(angle),
            angle_is_scaled: false,
            path_shade: PathShade::Shape,
            focus: SceneRect::UNIT,
            tile: SceneRect::UNIT,
            flip: TileFlip::None,
            rotate_with_shape: true,
        }
    }
}

/// One of DrawingML's 54 preset patterns.
///
/// The whole table rather than a subset, because a pattern a reader can select in PowerPoint is a
/// pattern a document can contain, and a painter that mapped an unknown preset onto a near-enough
/// one would draw the wrong page silently. The names are the expanded, self-explanatory forms
/// `mjx-ooxml-types` already generates for `ST_PresetPatternVal`.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u32)]
#[allow(
    missing_docs,
    reason = "fifty-four preset names that document themselves"
)]
pub enum PatternPreset {
    Percent5,
    Percent10,
    Percent20,
    Percent25,
    Percent30,
    Percent40,
    Percent50,
    Percent60,
    Percent70,
    Percent75,
    Percent80,
    Percent90,
    Horizontal,
    Vertical,
    LightHorizontal,
    LightVertical,
    DarkHorizontal,
    DarkVertical,
    NarrowHorizontal,
    NarrowVertical,
    DashedHorizontal,
    DashedVertical,
    Cross,
    DownwardDiagonal,
    UpwardDiagonal,
    LightDownwardDiagonal,
    LightUpwardDiagonal,
    DarkDownwardDiagonal,
    DarkUpwardDiagonal,
    WideDownwardDiagonal,
    WideUpwardDiagonal,
    DashedDownwardDiagonal,
    DashedUpwardDiagonal,
    DiagonalCross,
    SmallCheckerboard,
    LargeCheckerboard,
    SmallGrid,
    LargeGrid,
    DottedGrid,
    SmallConfetti,
    LargeConfetti,
    HorizontalBrick,
    DiagonalBrick,
    SolidDiamond,
    OpenDiamond,
    DottedDiamond,
    Plaid,
    Sphere,
    Weave,
    Divot,
    Shingle,
    Wave,
    Trellis,
    ZigZag,
}

/// How many preset patterns DrawingML defines.
pub const PATTERN_PRESET_COUNT: u32 = 54;

impl PatternPreset {
    /// The wire value, which is the preset's position in `ST_PresetPatternVal`.
    #[must_use]
    pub const fn wire_value(self) -> u32 {
        self as u32
    }

    /// The preset `value` names, or `None` for a number outside the table.
    #[must_use]
    pub fn from_wire_value(value: u32) -> Option<Self> {
        // A `match` over fifty-four arms would say the same thing at fifty-four times the length,
        // and this transmute-free form cannot drift from the discriminants: it *is* them.
        Self::ALL.get(value as usize).copied()
    }

    /// Every preset, in wire order.
    pub const ALL: [Self; PATTERN_PRESET_COUNT as usize] = [
        Self::Percent5,
        Self::Percent10,
        Self::Percent20,
        Self::Percent25,
        Self::Percent30,
        Self::Percent40,
        Self::Percent50,
        Self::Percent60,
        Self::Percent70,
        Self::Percent75,
        Self::Percent80,
        Self::Percent90,
        Self::Horizontal,
        Self::Vertical,
        Self::LightHorizontal,
        Self::LightVertical,
        Self::DarkHorizontal,
        Self::DarkVertical,
        Self::NarrowHorizontal,
        Self::NarrowVertical,
        Self::DashedHorizontal,
        Self::DashedVertical,
        Self::Cross,
        Self::DownwardDiagonal,
        Self::UpwardDiagonal,
        Self::LightDownwardDiagonal,
        Self::LightUpwardDiagonal,
        Self::DarkDownwardDiagonal,
        Self::DarkUpwardDiagonal,
        Self::WideDownwardDiagonal,
        Self::WideUpwardDiagonal,
        Self::DashedDownwardDiagonal,
        Self::DashedUpwardDiagonal,
        Self::DiagonalCross,
        Self::SmallCheckerboard,
        Self::LargeCheckerboard,
        Self::SmallGrid,
        Self::LargeGrid,
        Self::DottedGrid,
        Self::SmallConfetti,
        Self::LargeConfetti,
        Self::HorizontalBrick,
        Self::DiagonalBrick,
        Self::SolidDiamond,
        Self::OpenDiamond,
        Self::DottedDiamond,
        Self::Plaid,
        Self::Sphere,
        Self::Weave,
        Self::Divot,
        Self::Shingle,
        Self::Wave,
        Self::Trellis,
        Self::ZigZag,
    ];
}

/// How a picture fills the area it is drawn into.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ImageFillMode {
    /// `a:stretch` — one copy, scaled to the whole area. The default, and what an unadorned
    /// `a:blipFill` means.
    #[default]
    Stretch,
    /// `a:tile` — repeated at its natural size, offset and scaled by the tile fields.
    Tile,
}

/// The image adjustments DrawingML defines, all of them per-fragment arithmetic.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ImageAdjustments {
    /// `a:alphaModFix@amt` — the whole image's opacity, in ten-thousandths, `10_000` being opaque.
    pub alpha_in_ten_thousandths: u16,
    /// `a:lum@bright` — added to the luminance, in ten-thousandths, signed.
    pub brightness_in_ten_thousandths: i16,
    /// `a:lum@contrast` — the contrast multiplier's offset, in ten-thousandths, signed.
    pub contrast_in_ten_thousandths: i16,
    /// `a:grayscl` — whether the image is drawn without hue.
    pub grayscale: bool,
    /// `a:duotone` — the two colours the image's luminance ramp is remapped onto, shadow first.
    pub duotone: Option<(Color, Color)>,
    /// `a:clrChange` — one colour replaced by another, `from` then `to`.
    pub color_change: Option<(Color, Color)>,
}

impl ImageAdjustments {
    /// No adjustment at all: fully opaque, unchanged luminance, no duotone and no replacement.
    #[must_use]
    pub fn none() -> Self {
        Self {
            alpha_in_ten_thousandths: 10_000,
            brightness_in_ten_thousandths: 0,
            contrast_in_ten_thousandths: 0,
            grayscale: false,
            duotone: None,
            color_change: None,
        }
    }

    /// Whether this leaves the image exactly as it is.
    #[must_use]
    pub fn is_identity(&self) -> bool {
        *self == Self::none()
    }
}

/// A picture, and everything that decides how it is drawn — the record the image table holds.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Image {
    /// The handle a box model issued, as [`mjx_layout::ImageRef::number`] gave it.
    ///
    /// The pixels are **not** here and never will be: they live in whatever decoded them, which
    /// outlives a frame and is shared between every page that draws the same picture. A display list
    /// that embedded them would be a copy of the document rather than a description of a page.
    pub handle: u64,
    /// One copy stretched, or repeated.
    pub fill_mode: ImageFillMode,
    /// `a:srcRect` — which part of the image is shown, in fractions of the whole.
    pub crop: SceneRect,
    /// `a:tile@tx`/`@ty` — where the first tile's corner sits, in device pixels.
    pub tile_offset: ScenePoint,
    /// `a:tile@sx` — the tile's horizontal scale, `1.0` being its natural size.
    pub tile_scale_x: f32,
    /// `a:tile@sy` — its vertical scale.
    pub tile_scale_y: f32,
    /// `a:tile@flip` — how the repeats are mirrored.
    pub flip: TileFlip,
    /// `a:tile@algn` — which corner or edge the tiling is anchored to.
    pub anchor: RectangleAnchor,
    /// `@rotWithShape` — whether the picture turns with the shape.
    pub rotate_with_shape: bool,
    /// What is done to the pixels on the way to the screen.
    pub adjustments: ImageAdjustments,
}

impl Image {
    /// The picture `handle`, stretched once over the area, unadjusted and uncropped.
    #[must_use]
    pub fn stretched(handle: u64) -> Self {
        Self {
            handle,
            fill_mode: ImageFillMode::Stretch,
            crop: SceneRect::UNIT,
            tile_offset: ScenePoint::ORIGIN,
            tile_scale_x: 1.0,
            tile_scale_y: 1.0,
            flip: TileFlip::None,
            anchor: RectangleAnchor::TopLeft,
            rotate_with_shape: true,
            adjustments: ImageAdjustments::none(),
        }
    }
}

/// One of the nine points of a rectangle, which is what DrawingML's `ST_RectAlignment` names.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum RectangleAnchor {
    /// The top-left corner.
    #[default]
    TopLeft,
    /// The middle of the top edge.
    Top,
    /// The top-right corner.
    TopRight,
    /// The middle of the left edge.
    Left,
    /// The centre.
    Center,
    /// The middle of the right edge.
    Right,
    /// The bottom-left corner.
    BottomLeft,
    /// The middle of the bottom edge.
    Bottom,
    /// The bottom-right corner.
    BottomRight,
}

/// What a paint record is: a kind, and an address for whatever the kind needs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Paint {
    /// One colour everywhere.
    Solid(Color),
    /// The gradient at this index of the gradient table.
    Gradient(ResourceIndex),
    /// A two-colour preset pattern.
    Pattern {
        /// Which of the 54.
        preset: PatternPreset,
        /// The colour the pattern's marks are drawn in.
        foreground: Color,
        /// The colour behind them.
        background: Color,
    },
    /// The picture at this index of the image table.
    Image(ResourceIndex),
}

/// What a caller writes, and what a [`crate::ResourceResolver`] returns: a fill that owns
/// everything it needs and addresses no table.
#[derive(Clone, PartialEq, Debug, Default)]
pub enum FillStyle {
    /// Nothing is painted. Distinct from a fully transparent colour, because a shape with no fill is
    /// not hit by a click in its middle and one filled with transparency is.
    #[default]
    None,
    /// One colour everywhere.
    Solid(Color),
    /// A gradient, stops and all.
    Gradient(Gradient),
    /// A two-colour preset pattern.
    Pattern {
        /// Which of the 54.
        preset: PatternPreset,
        /// The colour the pattern's marks are drawn in.
        foreground: Color,
        /// The colour behind them.
        background: Color,
    },
    /// A picture.
    Image(Image),
}

impl FillStyle {
    /// Whether this paints nothing at all.
    #[must_use]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// How a stroke's ends are finished.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LineCap {
    /// `flat` — the stroke stops at the end point.
    #[default]
    Flat,
    /// `rnd` — a half-disc past the end point.
    Round,
    /// `sq` — a half-square past the end point.
    Square,
}

/// How a stroke's corners are finished.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum LineJoin {
    /// `a:round` — the corner is arced.
    #[default]
    Round,
    /// `a:bevel` — the corner is flattened.
    Bevel,
    /// `a:miter` — the corner is pointed, until the point would extend past `limit` times the
    /// stroke's width, at which point it is bevelled instead.
    Miter {
        /// How far past the corner a point may reach, as a multiple of the width.
        limit: f32,
    },
}

/// One of DrawingML's preset dash patterns.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum DashPattern {
    /// `solid` — no dashes at all.
    #[default]
    Solid,
    /// `dot`.
    Dot,
    /// `dash`.
    Dash,
    /// `lgDash` — a long dash.
    LargeDash,
    /// `dashDot`.
    DashDot,
    /// `lgDashDot`.
    LargeDashDot,
    /// `lgDashDotDot`.
    LargeDashDotDot,
    /// `sysDash` — the system's own dash, which is shorter than `dash`.
    SystemDash,
    /// `sysDot`.
    SystemDot,
    /// `sysDashDot`.
    SystemDashDot,
    /// `sysDashDotDot`.
    SystemDashDotDot,
}

/// Where a stroke sits relative to the path it follows.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum StrokeAlignment {
    /// `ctr` — half the width on each side.
    #[default]
    Centered,
    /// `in` — the whole width inside the path.
    Inset,
}

/// How many parallel lines a stroke is drawn as, and in what proportion.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CompoundStroke {
    /// `sng` — one line.
    #[default]
    Single,
    /// `dbl` — two of equal width.
    Double,
    /// `thickThin` — a thick line then a thin one.
    ThickThin,
    /// `thinThick` — a thin line then a thick one.
    ThinThick,
    /// `tri` — thin, thick, thin.
    Triple,
}

/// What decorates one end of a stroke.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LineEndShape {
    /// Nothing.
    #[default]
    None,
    /// A filled triangle.
    Triangle,
    /// A concave arrowhead.
    Stealth,
    /// A rhombus.
    Diamond,
    /// A circle.
    Oval,
    /// An open arrow.
    Arrow,
}

/// How large a line end is, relative to the stroke's width.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LineEndSize {
    /// `sm`.
    Small,
    /// `med`.
    #[default]
    Medium,
    /// `lg`.
    Large,
}

/// The decoration on one end of a stroke.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct LineEnd {
    /// What shape.
    pub shape: LineEndShape,
    /// How wide.
    pub width: LineEndSize,
    /// How long.
    pub length: LineEndSize,
}

/// A stroke record: how a path's outline is drawn, with its paint addressed by index.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Stroke {
    /// Which paint fills the stroke itself. A stroke can be a gradient or a picture, not only a
    /// colour, which `a:ln > a:gradFill` is exactly.
    pub paint: ResourceIndex,
    /// How wide, in device pixels.
    pub width: f32,
    /// How the ends are finished.
    pub cap: LineCap,
    /// How the corners are finished.
    pub join: LineJoin,
    /// Which preset dash.
    pub dash: DashPattern,
    /// Where the stroke sits relative to the path.
    pub alignment: StrokeAlignment,
    /// How many parallel lines.
    pub compound: CompoundStroke,
    /// What decorates the start.
    pub head: LineEnd,
    /// What decorates the finish.
    pub tail: LineEnd,
}

/// What a caller writes for a stroke: the same thing, owning its fill.
#[derive(Clone, PartialEq, Debug)]
pub struct StrokeStyle {
    /// What fills the stroke.
    pub fill: FillStyle,
    /// How wide, in device pixels.
    pub width: f32,
    /// How the ends are finished.
    pub cap: LineCap,
    /// How the corners are finished.
    pub join: LineJoin,
    /// Which preset dash.
    pub dash: DashPattern,
    /// Where the stroke sits relative to the path.
    pub alignment: StrokeAlignment,
    /// How many parallel lines.
    pub compound: CompoundStroke,
    /// What decorates the start.
    pub head: LineEnd,
    /// What decorates the finish.
    pub tail: LineEnd,
}

impl StrokeStyle {
    /// A plain stroke of `width` device pixels in `color`.
    #[must_use]
    pub fn solid(width: f32, color: Color) -> Self {
        Self {
            fill: FillStyle::Solid(color),
            width: finite(width),
            cap: LineCap::Flat,
            join: LineJoin::Round,
            dash: DashPattern::Solid,
            alignment: StrokeAlignment::Centered,
            compound: CompoundStroke::Single,
            head: LineEnd::default(),
            tail: LineEnd::default(),
        }
    }
}
