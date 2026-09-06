//! What happens to a subtree after it is drawn — declared here as data, executed in R08/R09.
//!
//! # A scene says *what*, never *how*
//!
//! An [`Effect`] carries a shadow's colour, its blur radius, its distance and its direction. It does
//! not carry a kernel, a pass count, a texture format or a render-target size, because those are a
//! painter's answers and there are four painters: a GPU one that does it in two separable passes, a
//! software one that does it with a box blur, and two exporters that write it as a filter node. A
//! display list that named a shader would have decided for all four, and three of them would be
//! wrong.
//!
//! # Why a DAG and not a list
//!
//! `CT_EffectList` is a list — at most one of each effect, in a fixed order — and `CT_EffectDag`
//! (`a:effectDag`, the other half of `EG_EffectProperties`) is not: it composes effects into a
//! container tree where one effect's output is another's input, and where two effects can share one
//! input. A vocabulary that could only express the list would be unable to carry the documents that
//! use the DAG, and the DAG expresses the list for free — a list is a chain.
//!
//! So the effect table **is** the DAG: each node names the node it consumes, or names none and
//! consumes the subtree the [`crate::Command::PushEffect`] wraps. The table is stored in
//! topological order and a node may only name a node below it, which is the whole of the acyclicity
//! check and is what stops a painter walking a chain it did not write from looping for ever.

use crate::encoding::ResourceIndex;
use crate::geometry::finite;
use crate::paint::{FillStyle, RectangleAnchor};

/// How one effect's output is combined with what is under it.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BlendMode {
    /// `over` — the usual source-over composite.
    #[default]
    Over,
    /// `mult`.
    Multiply,
    /// `screen`.
    Screen,
    /// `darken`.
    Darken,
    /// `lighten`.
    Lighten,
}

/// Which effect a node applies.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EffectKind {
    /// `a:blur` — a Gaussian blur of the whole input.
    Blur,
    /// `a:glow` — a coloured halo outside the input's alpha.
    Glow,
    /// `a:outerShdw` — a blurred, offset, coloured copy drawn behind the input.
    OuterShadow,
    /// `a:innerShdw` — the same, drawn inside the input's alpha rather than behind it.
    InnerShadow,
    /// `a:softEdge` — the input's alpha feathered inward from its own edge.
    SoftEdge,
    /// `a:reflection` — a flipped, faded copy below the input.
    Reflection,
    /// `a:fillOverlay` — a fill composited over the input in a blend mode.
    FillOverlay,
}

impl EffectKind {
    /// Every kind, in wire order.
    pub const ALL: [Self; 7] = [
        Self::Blur,
        Self::Glow,
        Self::OuterShadow,
        Self::InnerShadow,
        Self::SoftEdge,
        Self::Reflection,
        Self::FillOverlay,
    ];

    /// The wire value.
    #[must_use]
    pub const fn wire_value(self) -> u32 {
        match self {
            Self::Blur => 0,
            Self::Glow => 1,
            Self::OuterShadow => 2,
            Self::InnerShadow => 3,
            Self::SoftEdge => 4,
            Self::Reflection => 5,
            Self::FillOverlay => 6,
        }
    }

    /// The kind `value` names, or `None` for a number this build has no case for.
    #[must_use]
    pub fn from_wire_value(value: u32) -> Option<Self> {
        Self::ALL.get(value as usize).copied()
    }
}

/// One node of the effect DAG, as the effect table stores it.
///
/// Every field is present for every kind, and a kind that does not use a field leaves it at zero.
/// A fixed-stride record is what makes entry *n* addressable without walking, and the alternative —
/// a variable-length record per kind — would trade 64 bytes on a table with a handful of entries for
/// a walk on every lookup.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Effect {
    /// Which effect.
    pub kind: EffectKind,
    /// The node this one consumes, or `None` to consume the subtree the `PushEffect` wraps.
    ///
    /// Always **below** this node's own index, which is what makes the table a DAG rather than a
    /// graph and is checked on decode.
    pub input: Option<ResourceIndex>,
    /// The paint the effect draws in — a shadow's colour, a glow's colour, a fill overlay's fill —
    /// or `None` for an effect that only rearranges what is already there.
    ///
    /// A paint index rather than a colour, because `a:fillOverlay` takes a whole
    /// `EG_FillProperties` and because a glow in a gradient is expressible for free once a shadow's
    /// colour has to be a paint anyway.
    pub paint: Option<ResourceIndex>,
    /// How far the blur reaches, in device pixels. A glow's `@rad`, a shadow's `@blurRad`, a soft
    /// edge's `@rad`, a blur's `@rad`.
    pub radius: f32,
    /// How far a shadow or reflection is offset from what casts it, in device pixels.
    pub distance: f32,
    /// Which way that offset points, in radians clockwise from the positive `x` axis.
    pub direction: f32,
    /// `@sx` — the horizontal scale applied to the shadow or reflection.
    pub scale_x: f32,
    /// `@sy` — the vertical scale.
    pub scale_y: f32,
    /// `@kx` — the horizontal skew, in radians.
    pub skew_x: f32,
    /// `@ky` — the vertical skew, in radians.
    pub skew_y: f32,
    /// `a:reflection@stA` — the reflection's opacity where it begins, `0.0` to `1.0`.
    pub start_alpha: f32,
    /// `@stPos` — how far along the fade that opacity applies.
    pub start_position: f32,
    /// `@endA` — the opacity where it ends.
    pub end_alpha: f32,
    /// `@endPos` — how far along the fade *that* applies.
    pub end_position: f32,
    /// `@fadeDir` — which way the reflection fades, in radians.
    pub fade_direction: f32,
    /// `a:blur@grow` — whether the blur is allowed to grow the shape's bounds, or is clipped to
    /// them.
    pub grow: bool,
    /// `@rotWithShape` — whether the effect turns with the shape.
    pub rotate_with_shape: bool,
    /// `@algn` — which point of the shape's box the scale and skew are applied about.
    pub anchor: RectangleAnchor,
    /// How the effect's output is combined with what is under it.
    pub blend: BlendMode,
}

impl Effect {
    /// An effect of `kind` consuming the subtree, with every number at zero.
    ///
    /// The base every real effect is built from, so that adding a field to [`Effect`] does not
    /// oblige every caller to say something about it.
    #[must_use]
    pub fn new(kind: EffectKind) -> Self {
        Self {
            kind,
            input: None,
            paint: None,
            radius: 0.0,
            distance: 0.0,
            direction: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew_x: 0.0,
            skew_y: 0.0,
            start_alpha: 0.0,
            start_position: 0.0,
            end_alpha: 0.0,
            end_position: 0.0,
            fade_direction: 0.0,
            grow: true,
            rotate_with_shape: false,
            anchor: RectangleAnchor::BottomLeft,
            blend: BlendMode::Over,
        }
    }
}

/// What a caller writes for one node of the DAG: the same node, owning its fill and naming its input
/// by position within the decoration rather than by table index.
///
/// `input` is an index into the decoration's own `effects` vector and must be **below** this
/// effect's position in it, for the reason the module documentation gives. The last entry of that
/// vector is the root — the one the `PushEffect` names.
#[derive(Clone, PartialEq, Debug)]
pub struct EffectStyle {
    /// Which effect.
    pub kind: EffectKind,
    /// Which earlier effect of the same decoration this one consumes, or `None` for the subtree.
    pub input: Option<usize>,
    /// What the effect draws in.
    pub fill: FillStyle,
    /// How far the blur reaches, in device pixels.
    pub radius: f32,
    /// How far the offset reaches, in device pixels.
    pub distance: f32,
    /// Which way it points, in radians.
    pub direction: f32,
    /// The horizontal scale.
    pub scale_x: f32,
    /// The vertical scale.
    pub scale_y: f32,
    /// The horizontal skew, in radians.
    pub skew_x: f32,
    /// The vertical skew, in radians.
    pub skew_y: f32,
    /// A reflection's opacity where it begins.
    pub start_alpha: f32,
    /// How far along the fade that applies.
    pub start_position: f32,
    /// Its opacity where it ends.
    pub end_alpha: f32,
    /// How far along the fade that applies.
    pub end_position: f32,
    /// Which way it fades, in radians.
    pub fade_direction: f32,
    /// Whether a blur may grow the bounds.
    pub grow: bool,
    /// Whether the effect turns with the shape.
    pub rotate_with_shape: bool,
    /// Which point the scale and skew are applied about.
    pub anchor: RectangleAnchor,
    /// How the output is combined with what is under it.
    pub blend: BlendMode,
}

impl EffectStyle {
    /// An effect of `kind` over the subtree, with no fill and every number at rest.
    #[must_use]
    pub fn new(kind: EffectKind) -> Self {
        Self {
            kind,
            input: None,
            fill: FillStyle::None,
            radius: 0.0,
            distance: 0.0,
            direction: 0.0,
            scale_x: 1.0,
            scale_y: 1.0,
            skew_x: 0.0,
            skew_y: 0.0,
            start_alpha: 0.0,
            start_position: 0.0,
            end_alpha: 0.0,
            end_position: 0.0,
            fade_direction: 0.0,
            grow: true,
            rotate_with_shape: false,
            anchor: RectangleAnchor::BottomLeft,
            blend: BlendMode::Over,
        }
    }

    /// A drop shadow: `color` at `distance` device pixels in `direction` radians, blurred by
    /// `radius`.
    #[must_use]
    pub fn outer_shadow(
        color: mjx_tokens::Color,
        radius: f32,
        distance: f32,
        direction: f32,
    ) -> Self {
        Self {
            fill: FillStyle::Solid(color),
            radius: finite(radius),
            distance: finite(distance),
            direction: finite(direction),
            ..Self::new(EffectKind::OuterShadow)
        }
    }
}
