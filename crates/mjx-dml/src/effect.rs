//! DrawingML effects: `a:effectLst` (`CT_EffectList`) — the shadows, glow, blur, reflection, soft
//! edges, and fill overlay a shape renders on top of its geometry.
//!
//! [`EffectList`] is a **fidelity wrapper** over the `a:effectLst` element (its name, attributes,
//! children, and self-closing flag preserved verbatim); the eight effect children are exposed by typed
//! accessors, while any unmodeled child (`extLst`, an MCE bucket) stays opaque so the effect list
//! round-trips byte-for-byte. [`EffectListSpec`] is the interner-free value an interner-less caller
//! (`mjx-pptx`'s future `shape_effects` / `set_shape_effects`) reads and writes.
//!
//! `CT_EffectList` is an ordered sequence of at-most-one of each effect, in this fixed schema order:
//! `blur` → `fillOverlay` → `glow` → `innerShdw` → `outerShdw` → `prstShdw` → `reflection` →
//! `softEdge`. The colored effects (`glow`/`innerShdw`/`outerShdw`/`prstShdw`) each carry a required
//! `EG_ColorChoice`, reused as [`Color`] / [`ColorSpec`]; `fillOverlay` carries a full `EG_FillProperties`,
//! reused as [`Fill`] / [`FillSpec`]. The rarer `effectDag` alternative of `EG_EffectProperties` is not
//! an `effectLst` child and is handled (opaque) at the packaging layer.

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};
use mjx_ooxml_types::support::OnOff;

use crate::build::{
    dml_child, dml_element, dml_name, fidelity_element_impls, first_color_child, first_fill_child,
};
use crate::codec::{EmuCoordinate, Percentage, SixtyThousandthsOfADegree};
use crate::color::{Color, ColorSpec};
use crate::fill::{Fill, FillSpec};
use crate::geometry::{Angle, Emu, Fraction};

pub use mjx_ooxml_types::drawingml::{BlendMode, PresetShadow, RectangleAlignment};

// ---------------------------------------------------------------------------------------------
// Typed effect values (interner-free)
// ---------------------------------------------------------------------------------------------

/// `a:blur` (`CT_BlurEffect`) — a Gaussian blur applied to the shape and its effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlurEffect {
    /// The blur radius (`@rad`, EMU; schema default `0`).
    pub radius: Option<Emu>,
    /// Whether the blur grows the bounds of the object (`@grow`; schema default `true`).
    pub grow: Option<bool>,
}

/// `a:fillOverlay` (`CT_FillOverlayEffect`) — a fill layered over the shape, blended with the effect
/// beneath it. Carries a full [`FillSpec`] and a required blend [`BlendMode`].
#[derive(Debug, Clone, PartialEq)]
pub struct FillOverlayEffect {
    /// The overlay fill (`EG_FillProperties`).
    pub fill: FillSpec,
    /// How the overlay blends with what is beneath it (`@blend`, required).
    pub blend: BlendMode,
}

/// `a:glow` (`CT_GlowEffect`) — a colored radiance around the shape's edges.
#[derive(Debug, Clone, PartialEq)]
pub struct GlowEffect {
    /// The glow color (`EG_ColorChoice`, required).
    pub color: ColorSpec,
    /// The glow radius (`@rad`, EMU; schema default `0`).
    pub radius: Option<Emu>,
}

/// `a:innerShdw` (`CT_InnerShadowEffect`) — a shadow cast inside the shape's edges.
#[derive(Debug, Clone, PartialEq)]
pub struct InnerShadowEffect {
    /// The shadow color (`EG_ColorChoice`, required).
    pub color: ColorSpec,
    /// The blur radius (`@blurRad`, EMU; schema default `0`).
    pub blur_radius: Option<Emu>,
    /// The offset distance (`@dist`, EMU; schema default `0`).
    pub distance: Option<Emu>,
    /// The offset direction (`@dir`; schema default `0`).
    pub direction: Option<Angle>,
}

/// `a:outerShdw` (`CT_OuterShadowEffect`) — a shadow cast outside the shape's edges.
#[derive(Debug, Clone, PartialEq)]
pub struct OuterShadowEffect {
    /// The shadow color (`EG_ColorChoice`, required).
    pub color: ColorSpec,
    /// The blur radius (`@blurRad`, EMU; schema default `0`).
    pub blur_radius: Option<Emu>,
    /// The offset distance (`@dist`, EMU; schema default `0`).
    pub distance: Option<Emu>,
    /// The offset direction (`@dir`; schema default `0`).
    pub direction: Option<Angle>,
    /// The horizontal scaling factor (`@sx`; schema default `100%`).
    pub scale_x: Option<Fraction>,
    /// The vertical scaling factor (`@sy`; schema default `100%`).
    pub scale_y: Option<Fraction>,
    /// The horizontal skew angle (`@kx`; schema default `0`).
    pub skew_x: Option<Angle>,
    /// The vertical skew angle (`@ky`; schema default `0`).
    pub skew_y: Option<Angle>,
    /// The origin the shadow is scaled/skewed about (`@algn`; schema default `b`).
    pub alignment: Option<RectangleAlignment>,
    /// Whether the shadow rotates with the shape (`@rotWithShape`; schema default `true`).
    pub rotate_with_shape: Option<bool>,
}

/// `a:prstShdw` (`CT_PresetShadowEffect`) — one of the 20 preset shadows, colored and offset.
#[derive(Debug, Clone, PartialEq)]
pub struct PresetShadowEffect {
    /// The preset shadow kind (`@prst`, required).
    pub preset: PresetShadow,
    /// The shadow color (`EG_ColorChoice`, required).
    pub color: ColorSpec,
    /// The offset distance (`@dist`, EMU; schema default `0`).
    pub distance: Option<Emu>,
    /// The offset direction (`@dir`; schema default `0`).
    pub direction: Option<Angle>,
}

/// `a:reflection` (`CT_ReflectionEffect`) — a mirrored, fading copy of the shape below it.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ReflectionEffect {
    /// The blur radius (`@blurRad`, EMU; schema default `0`).
    pub blur_radius: Option<Emu>,
    /// The starting alpha of the reflection (`@stA`; schema default `100%`).
    pub start_alpha: Option<Fraction>,
    /// The starting position of the alpha gradient (`@stPos`; schema default `0%`).
    pub start_position: Option<Fraction>,
    /// The ending alpha of the reflection (`@endA`; schema default `0%`).
    pub end_alpha: Option<Fraction>,
    /// The ending position of the alpha gradient (`@endPos`; schema default `100%`).
    pub end_position: Option<Fraction>,
    /// The offset distance (`@dist`, EMU; schema default `0`).
    pub distance: Option<Emu>,
    /// The offset direction (`@dir`; schema default `0`).
    pub direction: Option<Angle>,
    /// The direction in which the alpha gradient fades (`@fadeDir`; schema default `5400000`, i.e. 90°).
    pub fade_direction: Option<Angle>,
    /// The horizontal scaling factor (`@sx`; schema default `100%`).
    pub scale_x: Option<Fraction>,
    /// The vertical scaling factor (`@sy`; schema default `100%`).
    pub scale_y: Option<Fraction>,
    /// The horizontal skew angle (`@kx`; schema default `0`).
    pub skew_x: Option<Angle>,
    /// The vertical skew angle (`@ky`; schema default `0`).
    pub skew_y: Option<Angle>,
    /// The origin the reflection is scaled/skewed about (`@algn`; schema default `b`).
    pub alignment: Option<RectangleAlignment>,
    /// Whether the reflection rotates with the shape (`@rotWithShape`; schema default `true`).
    pub rotate_with_shape: Option<bool>,
}

/// `a:softEdge` (`CT_SoftEdgesEffect`) — feathered (blurred) shape edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoftEdgeEffect {
    /// The feathering radius (`@rad`, EMU, required).
    pub radius: Emu,
}

// ---------------------------------------------------------------------------------------------
// Constructing an effect
// ---------------------------------------------------------------------------------------------
//
// Every effect above is a plain struct of `pub` fields, which is right for reading one but wrong
// for writing one: naming a shadow's distance meant spelling out the eight other fields as `None`.
// So each carries a `new` that takes exactly what the schema makes **required** — a color, a preset,
// a radius — and a `with_` method per optional attribute, the same spec-builder shape `LineSpec`,
// `CharacterPropertiesSpec` and `CellFormat` already use. An attribute a builder does not name stays
// unset, and an unset attribute is not written, so the renderer applies the schema default.

impl BlurEffect {
    /// A blur that names no radius and no grow flag — the schema defaults (`0` EMU, growing).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the blur radius (`@rad`).
    #[must_use]
    pub fn with_radius(mut self, radius: Emu) -> Self {
        self.radius = Some(radius);
        self
    }

    /// Sets whether the blur grows the object's bounds (`@grow`).
    #[must_use]
    pub fn with_grow(mut self, grow: bool) -> Self {
        self.grow = Some(grow);
        self
    }
}

impl FillOverlayEffect {
    /// An overlay of `fill`, blended with `blend`. Both are required by the schema.
    #[must_use]
    pub fn new(fill: FillSpec, blend: BlendMode) -> Self {
        Self { fill, blend }
    }
}

impl GlowEffect {
    /// A glow of `color`, with no radius of its own (the schema default, `0`).
    #[must_use]
    pub fn new(color: ColorSpec) -> Self {
        Self {
            color,
            radius: None,
        }
    }

    /// Sets the glow radius (`@rad`).
    #[must_use]
    pub fn with_radius(mut self, radius: Emu) -> Self {
        self.radius = Some(radius);
        self
    }
}

impl InnerShadowEffect {
    /// An inner shadow of `color`, with no blur, offset or direction of its own.
    #[must_use]
    pub fn new(color: ColorSpec) -> Self {
        Self {
            color,
            blur_radius: None,
            distance: None,
            direction: None,
        }
    }

    /// Sets the blur radius (`@blurRad`).
    #[must_use]
    pub fn with_blur_radius(mut self, blur_radius: Emu) -> Self {
        self.blur_radius = Some(blur_radius);
        self
    }

    /// Sets the offset distance (`@dist`).
    #[must_use]
    pub fn with_distance(mut self, distance: Emu) -> Self {
        self.distance = Some(distance);
        self
    }

    /// Sets the offset direction (`@dir`).
    #[must_use]
    pub fn with_direction(mut self, direction: Angle) -> Self {
        self.direction = Some(direction);
        self
    }
}

impl OuterShadowEffect {
    /// An outer shadow of `color`, naming nothing else — no blur, offset, scale, skew, alignment or
    /// rotation, so every one of them renders at its schema default.
    #[must_use]
    pub fn new(color: ColorSpec) -> Self {
        Self {
            color,
            blur_radius: None,
            distance: None,
            direction: None,
            scale_x: None,
            scale_y: None,
            skew_x: None,
            skew_y: None,
            alignment: None,
            rotate_with_shape: None,
        }
    }

    /// Sets the blur radius (`@blurRad`).
    #[must_use]
    pub fn with_blur_radius(mut self, blur_radius: Emu) -> Self {
        self.blur_radius = Some(blur_radius);
        self
    }

    /// Sets the offset distance (`@dist`).
    #[must_use]
    pub fn with_distance(mut self, distance: Emu) -> Self {
        self.distance = Some(distance);
        self
    }

    /// Sets the offset direction (`@dir`).
    #[must_use]
    pub fn with_direction(mut self, direction: Angle) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Sets the horizontal scaling factor (`@sx`).
    #[must_use]
    pub fn with_scale_x(mut self, scale_x: Fraction) -> Self {
        self.scale_x = Some(scale_x);
        self
    }

    /// Sets the vertical scaling factor (`@sy`).
    #[must_use]
    pub fn with_scale_y(mut self, scale_y: Fraction) -> Self {
        self.scale_y = Some(scale_y);
        self
    }

    /// Sets the horizontal skew angle (`@kx`).
    #[must_use]
    pub fn with_skew_x(mut self, skew_x: Angle) -> Self {
        self.skew_x = Some(skew_x);
        self
    }

    /// Sets the vertical skew angle (`@ky`).
    #[must_use]
    pub fn with_skew_y(mut self, skew_y: Angle) -> Self {
        self.skew_y = Some(skew_y);
        self
    }

    /// Sets the origin the shadow is scaled and skewed about (`@algn`).
    #[must_use]
    pub fn with_alignment(mut self, alignment: RectangleAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Sets whether the shadow rotates with the shape (`@rotWithShape`).
    #[must_use]
    pub fn with_rotate_with_shape(mut self, rotate_with_shape: bool) -> Self {
        self.rotate_with_shape = Some(rotate_with_shape);
        self
    }
}

impl PresetShadowEffect {
    /// One of the 20 preset shadows, in `color`, with no offset of its own.
    #[must_use]
    pub fn new(preset: PresetShadow, color: ColorSpec) -> Self {
        Self {
            preset,
            color,
            distance: None,
            direction: None,
        }
    }

    /// Sets the offset distance (`@dist`).
    #[must_use]
    pub fn with_distance(mut self, distance: Emu) -> Self {
        self.distance = Some(distance);
        self
    }

    /// Sets the offset direction (`@dir`).
    #[must_use]
    pub fn with_direction(mut self, direction: Angle) -> Self {
        self.direction = Some(direction);
        self
    }
}

impl ReflectionEffect {
    /// A reflection that names nothing — every attribute at its schema default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the blur radius (`@blurRad`).
    #[must_use]
    pub fn with_blur_radius(mut self, blur_radius: Emu) -> Self {
        self.blur_radius = Some(blur_radius);
        self
    }

    /// Sets the alpha the reflection starts at (`@stA`).
    #[must_use]
    pub fn with_start_alpha(mut self, start_alpha: Fraction) -> Self {
        self.start_alpha = Some(start_alpha);
        self
    }

    /// Sets where the alpha gradient starts (`@stPos`).
    #[must_use]
    pub fn with_start_position(mut self, start_position: Fraction) -> Self {
        self.start_position = Some(start_position);
        self
    }

    /// Sets the alpha the reflection ends at (`@endA`).
    #[must_use]
    pub fn with_end_alpha(mut self, end_alpha: Fraction) -> Self {
        self.end_alpha = Some(end_alpha);
        self
    }

    /// Sets where the alpha gradient ends (`@endPos`).
    #[must_use]
    pub fn with_end_position(mut self, end_position: Fraction) -> Self {
        self.end_position = Some(end_position);
        self
    }

    /// Sets the offset distance (`@dist`).
    #[must_use]
    pub fn with_distance(mut self, distance: Emu) -> Self {
        self.distance = Some(distance);
        self
    }

    /// Sets the offset direction (`@dir`).
    #[must_use]
    pub fn with_direction(mut self, direction: Angle) -> Self {
        self.direction = Some(direction);
        self
    }

    /// Sets the direction the alpha gradient fades in (`@fadeDir`).
    #[must_use]
    pub fn with_fade_direction(mut self, fade_direction: Angle) -> Self {
        self.fade_direction = Some(fade_direction);
        self
    }

    /// Sets the horizontal scaling factor (`@sx`).
    #[must_use]
    pub fn with_scale_x(mut self, scale_x: Fraction) -> Self {
        self.scale_x = Some(scale_x);
        self
    }

    /// Sets the vertical scaling factor (`@sy`).
    #[must_use]
    pub fn with_scale_y(mut self, scale_y: Fraction) -> Self {
        self.scale_y = Some(scale_y);
        self
    }

    /// Sets the horizontal skew angle (`@kx`).
    #[must_use]
    pub fn with_skew_x(mut self, skew_x: Angle) -> Self {
        self.skew_x = Some(skew_x);
        self
    }

    /// Sets the vertical skew angle (`@ky`).
    #[must_use]
    pub fn with_skew_y(mut self, skew_y: Angle) -> Self {
        self.skew_y = Some(skew_y);
        self
    }

    /// Sets the origin the reflection is scaled and skewed about (`@algn`).
    #[must_use]
    pub fn with_alignment(mut self, alignment: RectangleAlignment) -> Self {
        self.alignment = Some(alignment);
        self
    }

    /// Sets whether the reflection rotates with the shape (`@rotWithShape`).
    #[must_use]
    pub fn with_rotate_with_shape(mut self, rotate_with_shape: bool) -> Self {
        self.rotate_with_shape = Some(rotate_with_shape);
        self
    }
}

impl SoftEdgeEffect {
    /// Feathered edges of `radius`, which the schema requires.
    #[must_use]
    pub fn new(radius: Emu) -> Self {
        Self { radius }
    }
}

// ---------------------------------------------------------------------------------------------
// EffectList — the fidelity wrapper
// ---------------------------------------------------------------------------------------------

/// `a:effectLst` (`CT_EffectList`) — a shape's list of rendered effects: an optional blur, fill
/// overlay, glow, inner/outer/preset shadow, reflection, and soft edge, in that fixed order.
///
/// A fidelity wrapper: the eight effects are exposed typed, while any unmodeled child (`extLst`, an
/// MCE bucket) and unknown attributes are preserved opaque so the effect list round-trips byte-for-byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl EffectList {
    /// The blur effect (`a:blur`), or `None` if absent.
    #[must_use]
    pub fn blur(&self, interner: &Interner) -> Option<BlurEffect> {
        dml_child(&self.children, interner, "blur").map(|el| read_blur(el, interner))
    }

    /// The fill-overlay effect (`a:fillOverlay`), or `None` if absent (or missing its required fill /
    /// blend mode).
    #[must_use]
    pub fn fill_overlay(&self, interner: &Interner) -> Option<FillOverlayEffect> {
        dml_child(&self.children, interner, "fillOverlay")
            .and_then(|el| read_fill_overlay(el, interner))
    }

    /// The glow effect (`a:glow`), or `None` if absent (or missing its required color).
    #[must_use]
    pub fn glow(&self, interner: &Interner) -> Option<GlowEffect> {
        dml_child(&self.children, interner, "glow").and_then(|el| read_glow(el, interner))
    }

    /// The inner-shadow effect (`a:innerShdw`), or `None` if absent (or missing its required color).
    #[must_use]
    pub fn inner_shadow(&self, interner: &Interner) -> Option<InnerShadowEffect> {
        dml_child(&self.children, interner, "innerShdw")
            .and_then(|el| read_inner_shadow(el, interner))
    }

    /// The outer-shadow effect (`a:outerShdw`), or `None` if absent (or missing its required color).
    #[must_use]
    pub fn outer_shadow(&self, interner: &Interner) -> Option<OuterShadowEffect> {
        dml_child(&self.children, interner, "outerShdw")
            .and_then(|el| read_outer_shadow(el, interner))
    }

    /// The preset-shadow effect (`a:prstShdw`), or `None` if absent (or missing its required color /
    /// preset).
    #[must_use]
    pub fn preset_shadow(&self, interner: &Interner) -> Option<PresetShadowEffect> {
        dml_child(&self.children, interner, "prstShdw")
            .and_then(|el| read_preset_shadow(el, interner))
    }

    /// The reflection effect (`a:reflection`), or `None` if absent.
    #[must_use]
    pub fn reflection(&self, interner: &Interner) -> Option<ReflectionEffect> {
        dml_child(&self.children, interner, "reflection").map(|el| read_reflection(el, interner))
    }

    /// The soft-edge effect (`a:softEdge`), or `None` if absent (or missing its required radius).
    #[must_use]
    pub fn soft_edge(&self, interner: &Interner) -> Option<SoftEdgeEffect> {
        dml_child(&self.children, interner, "softEdge").and_then(|el| read_soft_edge(el, interner))
    }

    /// This effect list as an interner-free [`EffectListSpec`] — resolving the eight effects and
    /// dropping opaque internals (`extLst`). Reading does not need a mutable interner.
    #[must_use]
    pub fn spec(&self, interner: &Interner) -> EffectListSpec {
        EffectListSpec {
            blur: self.blur(interner),
            fill_overlay: self.fill_overlay(interner),
            glow: self.glow(interner),
            inner_shadow: self.inner_shadow(interner),
            outer_shadow: self.outer_shadow(interner),
            preset_shadow: self.preset_shadow(interner),
            reflection: self.reflection(interner),
            soft_edge: self.soft_edge(interner),
        }
    }

    /// The glow's interner-bound [`Color`] (`a:glow`'s `EG_ColorChoice`), or `None` if absent — the raw
    /// color the effect resolver bakes (preserving color transforms a [`ColorSpec`] would drop).
    #[must_use]
    pub fn glow_color(&self, interner: &Interner) -> Option<Color> {
        effect_child_color(&self.children, interner, "glow")
    }

    /// The inner shadow's interner-bound [`Color`] (`a:innerShdw`'s `EG_ColorChoice`), or `None`.
    #[must_use]
    pub fn inner_shadow_color(&self, interner: &Interner) -> Option<Color> {
        effect_child_color(&self.children, interner, "innerShdw")
    }

    /// The outer shadow's interner-bound [`Color`] (`a:outerShdw`'s `EG_ColorChoice`), or `None`.
    #[must_use]
    pub fn outer_shadow_color(&self, interner: &Interner) -> Option<Color> {
        effect_child_color(&self.children, interner, "outerShdw")
    }

    /// The preset shadow's interner-bound [`Color`] (`a:prstShdw`'s `EG_ColorChoice`), or `None`.
    #[must_use]
    pub fn preset_shadow_color(&self, interner: &Interner) -> Option<Color> {
        effect_child_color(&self.children, interner, "prstShdw")
    }

    /// The fill-overlay's interner-bound [`Fill`] (`a:fillOverlay`'s `EG_FillProperties`), or `None` —
    /// the raw fill the effect resolver bakes.
    #[must_use]
    pub fn fill_overlay_fill(&self, interner: &Interner) -> Option<Fill> {
        let overlay = dml_child(&self.children, interner, "fillOverlay")?;
        first_fill_child(&overlay.children, interner)
            .and_then(|el| Fill::from_xml(el, interner).ok())
    }
}

/// The `EG_ColorChoice` child of a colored effect element named `local`, as an interner-bound [`Color`].
fn effect_child_color(children: &[RawNode], interner: &Interner, local: &str) -> Option<Color> {
    let effect = dml_child(children, interner, local)?;
    first_color_child(effect, interner)
}

fidelity_element_impls!(EffectList);

// ---------------------------------------------------------------------------------------------
// EffectListSpec — the interner-free description
// ---------------------------------------------------------------------------------------------

/// An interner-free description of a shape's effect list (`a:effectLst`) — the friendly value an
/// interner-less caller reads and writes. Convert with [`EffectList::spec`] /
/// [`EffectListSpec::to_effect_list`]. A spec is a value description, not a fidelity view: converting an
/// `EffectList` to a spec and back rebuilds the element from its effects and drops any opaque internals
/// (`extLst`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EffectListSpec {
    /// The blur effect (`a:blur`).
    pub blur: Option<BlurEffect>,
    /// The fill-overlay effect (`a:fillOverlay`).
    pub fill_overlay: Option<FillOverlayEffect>,
    /// The glow effect (`a:glow`).
    pub glow: Option<GlowEffect>,
    /// The inner-shadow effect (`a:innerShdw`).
    pub inner_shadow: Option<InnerShadowEffect>,
    /// The outer-shadow effect (`a:outerShdw`).
    pub outer_shadow: Option<OuterShadowEffect>,
    /// The preset-shadow effect (`a:prstShdw`).
    pub preset_shadow: Option<PresetShadowEffect>,
    /// The reflection effect (`a:reflection`).
    pub reflection: Option<ReflectionEffect>,
    /// The soft-edge effect (`a:softEdge`).
    pub soft_edge: Option<SoftEdgeEffect>,
}

impl EffectListSpec {
    /// An empty effect list (no effects) — the same as [`EffectListSpec::default`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Builds the fidelity [`EffectList`] for this description, interning against `interner`. The
    /// children are assembled in `CT_EffectList` order: `blur` → `fillOverlay` → `glow` → `innerShdw`
    /// → `outerShdw` → `prstShdw` → `reflection` → `softEdge`.
    #[must_use]
    pub fn to_effect_list(&self, interner: &mut Interner) -> EffectList {
        let mut children = Vec::new();
        if let Some(blur) = &self.blur {
            children.push(RawNode::Element(build_blur(interner, blur)));
        }
        if let Some(fill_overlay) = &self.fill_overlay {
            children.push(RawNode::Element(build_fill_overlay(interner, fill_overlay)));
        }
        if let Some(glow) = &self.glow {
            children.push(RawNode::Element(build_glow(interner, glow)));
        }
        if let Some(inner) = &self.inner_shadow {
            children.push(RawNode::Element(build_inner_shadow(interner, inner)));
        }
        if let Some(outer) = &self.outer_shadow {
            children.push(RawNode::Element(build_outer_shadow(interner, outer)));
        }
        if let Some(preset) = &self.preset_shadow {
            children.push(RawNode::Element(build_preset_shadow(interner, preset)));
        }
        if let Some(reflection) = &self.reflection {
            children.push(RawNode::Element(build_reflection(interner, reflection)));
        }
        if let Some(soft_edge) = &self.soft_edge {
            children.push(RawNode::Element(build_soft_edge(interner, soft_edge)));
        }

        EffectList {
            name: dml_name(interner, "effectLst"),
            attributes: Vec::new(),
            empty: children.is_empty(),
            children,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Effect-specific builders
// ---------------------------------------------------------------------------------------------

/// Pushes the `EG_ColorChoice` child element for `color` when it can be rebuilt.
fn push_color(children: &mut Vec<RawNode>, interner: &mut Interner, color: &ColorSpec) {
    if let Some(color) = Color::from_spec(interner, color) {
        children.push(RawNode::Element(color.to_xml(interner)));
    }
}

/// The `EG_ColorChoice` child of a colored effect (`a:glow`/`a:*Shdw`), as a [`ColorSpec`].
fn effect_color(element: &RawElement, interner: &Interner) -> Option<ColorSpec> {
    first_color_child(element, interner).map(|color| color.spec(interner))
}

// ---------------------------------------------------------------------------------------------
// The attribute faces of the eight effect elements
// ---------------------------------------------------------------------------------------------
//
// Every effect above is an interner-free *value*, not a fidelity wrapper: `EffectList` retains the
// raw children and each effect is a projection out of one of them. The attribute face is what carries
// the `#[xml(attribute(..))]` declaration, over whichever attribute vector it is handed —
// `&element.attributes` when reading, which copies nothing, and a fresh `Vec` when building, which is
// the vector the new element will own. One declaration therefore serves the read and the write, and
// the same generated accessor performs both.
//
// The attributes are declared in **schema declaration order**, because a setter appends to an empty
// vector, so on the build path the declaration order is the emitted order.

/// `a:blur` (`CT_BlurEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "rad", codec = EmuCoordinate, accessor = radius))]
#[xml(attribute(local = "grow", codec = OnOff, accessor = grow))]
struct BlurAttributes<A> {
    attributes: A,
}

/// `a:fillOverlay` (`CT_FillOverlayEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "blend", codec = Enumeration<BlendMode>, accessor = blend, required))]
struct FillOverlayAttributes<A> {
    attributes: A,
}

/// `a:glow` (`CT_GlowEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "rad", codec = EmuCoordinate, accessor = radius))]
struct GlowAttributes<A> {
    attributes: A,
}

/// `a:innerShdw` (`CT_InnerShadowEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "blurRad", codec = EmuCoordinate, accessor = blur_radius))]
#[xml(attribute(local = "dist", codec = EmuCoordinate, accessor = distance))]
#[xml(attribute(local = "dir", codec = SixtyThousandthsOfADegree, accessor = direction))]
struct InnerShadowAttributes<A> {
    attributes: A,
}

/// `a:outerShdw` (`CT_OuterShadowEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "blurRad", codec = EmuCoordinate, accessor = blur_radius))]
#[xml(attribute(local = "dist", codec = EmuCoordinate, accessor = distance))]
#[xml(attribute(local = "dir", codec = SixtyThousandthsOfADegree, accessor = direction))]
#[xml(attribute(local = "sx", codec = Percentage, accessor = scale_x))]
#[xml(attribute(local = "sy", codec = Percentage, accessor = scale_y))]
#[xml(attribute(local = "kx", codec = SixtyThousandthsOfADegree, accessor = skew_x))]
#[xml(attribute(local = "ky", codec = SixtyThousandthsOfADegree, accessor = skew_y))]
#[xml(attribute(local = "algn", codec = Enumeration<RectangleAlignment>, accessor = alignment))]
#[xml(attribute(local = "rotWithShape", codec = OnOff, accessor = rotate_with_shape))]
struct OuterShadowAttributes<A> {
    attributes: A,
}

/// `a:prstShdw` (`CT_PresetShadowEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "prst", codec = Enumeration<PresetShadow>, accessor = preset, required))]
#[xml(attribute(local = "dist", codec = EmuCoordinate, accessor = distance))]
#[xml(attribute(local = "dir", codec = SixtyThousandthsOfADegree, accessor = direction))]
struct PresetShadowAttributes<A> {
    attributes: A,
}

/// `a:reflection` (`CT_ReflectionEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "blurRad", codec = EmuCoordinate, accessor = blur_radius))]
#[xml(attribute(local = "stA", codec = Percentage, accessor = start_alpha))]
#[xml(attribute(local = "stPos", codec = Percentage, accessor = start_position))]
#[xml(attribute(local = "endA", codec = Percentage, accessor = end_alpha))]
#[xml(attribute(local = "endPos", codec = Percentage, accessor = end_position))]
#[xml(attribute(local = "dist", codec = EmuCoordinate, accessor = distance))]
#[xml(attribute(local = "dir", codec = SixtyThousandthsOfADegree, accessor = direction))]
#[xml(attribute(local = "fadeDir", codec = SixtyThousandthsOfADegree, accessor = fade_direction))]
#[xml(attribute(local = "sx", codec = Percentage, accessor = scale_x))]
#[xml(attribute(local = "sy", codec = Percentage, accessor = scale_y))]
#[xml(attribute(local = "kx", codec = SixtyThousandthsOfADegree, accessor = skew_x))]
#[xml(attribute(local = "ky", codec = SixtyThousandthsOfADegree, accessor = skew_y))]
#[xml(attribute(local = "algn", codec = Enumeration<RectangleAlignment>, accessor = alignment))]
#[xml(attribute(local = "rotWithShape", codec = OnOff, accessor = rotate_with_shape))]
struct ReflectionAttributes<A> {
    attributes: A,
}

/// `a:softEdge` (`CT_SoftEdgesEffect`).
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "rad", codec = EmuCoordinate, accessor = radius, required))]
struct SoftEdgeAttributes<A> {
    attributes: A,
}

// ---------------------------------------------------------------------------------------------
// Per-effect readers
// ---------------------------------------------------------------------------------------------

fn read_blur(element: &RawElement, interner: &Interner) -> BlurEffect {
    let blur = BlurAttributes {
        attributes: &element.attributes,
    };
    BlurEffect {
        radius: blur.radius(interner).ok().flatten(),
        grow: blur.grow(interner).ok().flatten(),
    }
}

fn read_fill_overlay(element: &RawElement, interner: &Interner) -> Option<FillOverlayEffect> {
    let fill = first_fill_child(&element.children, interner)
        .and_then(|el| Fill::from_xml(el, interner).ok())?
        .spec(interner);
    let blend = FillOverlayAttributes {
        attributes: &element.attributes,
    }
    .blend(interner)
    .ok()?;
    Some(FillOverlayEffect { fill, blend })
}

fn read_glow(element: &RawElement, interner: &Interner) -> Option<GlowEffect> {
    Some(GlowEffect {
        color: effect_color(element, interner)?,
        radius: GlowAttributes {
            attributes: &element.attributes,
        }
        .radius(interner)
        .ok()
        .flatten(),
    })
}

fn read_inner_shadow(element: &RawElement, interner: &Interner) -> Option<InnerShadowEffect> {
    let shadow = InnerShadowAttributes {
        attributes: &element.attributes,
    };
    Some(InnerShadowEffect {
        color: effect_color(element, interner)?,
        blur_radius: shadow.blur_radius(interner).ok().flatten(),
        distance: shadow.distance(interner).ok().flatten(),
        direction: shadow.direction(interner).ok().flatten(),
    })
}

fn read_outer_shadow(element: &RawElement, interner: &Interner) -> Option<OuterShadowEffect> {
    let shadow = OuterShadowAttributes {
        attributes: &element.attributes,
    };
    Some(OuterShadowEffect {
        color: effect_color(element, interner)?,
        blur_radius: shadow.blur_radius(interner).ok().flatten(),
        distance: shadow.distance(interner).ok().flatten(),
        direction: shadow.direction(interner).ok().flatten(),
        scale_x: shadow.scale_x(interner).ok().flatten(),
        scale_y: shadow.scale_y(interner).ok().flatten(),
        skew_x: shadow.skew_x(interner).ok().flatten(),
        skew_y: shadow.skew_y(interner).ok().flatten(),
        alignment: shadow.alignment(interner).ok().flatten(),
        rotate_with_shape: shadow.rotate_with_shape(interner).ok().flatten(),
    })
}

fn read_preset_shadow(element: &RawElement, interner: &Interner) -> Option<PresetShadowEffect> {
    let shadow = PresetShadowAttributes {
        attributes: &element.attributes,
    };
    Some(PresetShadowEffect {
        preset: shadow.preset(interner).ok()?,
        color: effect_color(element, interner)?,
        distance: shadow.distance(interner).ok().flatten(),
        direction: shadow.direction(interner).ok().flatten(),
    })
}

fn read_reflection(element: &RawElement, interner: &Interner) -> ReflectionEffect {
    let reflection = ReflectionAttributes {
        attributes: &element.attributes,
    };
    ReflectionEffect {
        blur_radius: reflection.blur_radius(interner).ok().flatten(),
        start_alpha: reflection.start_alpha(interner).ok().flatten(),
        start_position: reflection.start_position(interner).ok().flatten(),
        end_alpha: reflection.end_alpha(interner).ok().flatten(),
        end_position: reflection.end_position(interner).ok().flatten(),
        distance: reflection.distance(interner).ok().flatten(),
        direction: reflection.direction(interner).ok().flatten(),
        fade_direction: reflection.fade_direction(interner).ok().flatten(),
        scale_x: reflection.scale_x(interner).ok().flatten(),
        scale_y: reflection.scale_y(interner).ok().flatten(),
        skew_x: reflection.skew_x(interner).ok().flatten(),
        skew_y: reflection.skew_y(interner).ok().flatten(),
        alignment: reflection.alignment(interner).ok().flatten(),
        rotate_with_shape: reflection.rotate_with_shape(interner).ok().flatten(),
    }
}

fn read_soft_edge(element: &RawElement, interner: &Interner) -> Option<SoftEdgeEffect> {
    SoftEdgeAttributes {
        attributes: &element.attributes,
    }
    .radius(interner)
    .ok()
    .map(|radius| SoftEdgeEffect { radius })
}

// ---------------------------------------------------------------------------------------------
// Per-effect builders (attributes emitted in schema declaration order)
// ---------------------------------------------------------------------------------------------

fn build_blur(interner: &mut Interner, blur: &BlurEffect) -> RawElement {
    let mut attributes = BlurAttributes {
        attributes: Vec::new(),
    };
    attributes.set_radius(interner, blur.radius);
    attributes.set_grow(interner, blur.grow);
    dml_element(interner, "blur", attributes.attributes, Vec::new())
}

fn build_fill_overlay(interner: &mut Interner, effect: &FillOverlayEffect) -> RawElement {
    let mut attributes = FillOverlayAttributes {
        attributes: Vec::new(),
    };
    attributes.set_blend(interner, effect.blend);
    let children = vec![RawNode::Element(
        effect.fill.to_fill(interner).to_xml(interner),
    )];
    dml_element(interner, "fillOverlay", attributes.attributes, children)
}

fn build_glow(interner: &mut Interner, glow: &GlowEffect) -> RawElement {
    let mut attributes = GlowAttributes {
        attributes: Vec::new(),
    };
    attributes.set_radius(interner, glow.radius);
    let mut children = Vec::new();
    push_color(&mut children, interner, &glow.color);
    dml_element(interner, "glow", attributes.attributes, children)
}

fn build_inner_shadow(interner: &mut Interner, shadow: &InnerShadowEffect) -> RawElement {
    let mut attributes = InnerShadowAttributes {
        attributes: Vec::new(),
    };
    attributes.set_blur_radius(interner, shadow.blur_radius);
    attributes.set_distance(interner, shadow.distance);
    attributes.set_direction(interner, shadow.direction);
    let mut children = Vec::new();
    push_color(&mut children, interner, &shadow.color);
    dml_element(interner, "innerShdw", attributes.attributes, children)
}

fn build_outer_shadow(interner: &mut Interner, shadow: &OuterShadowEffect) -> RawElement {
    let mut attributes = OuterShadowAttributes {
        attributes: Vec::new(),
    };
    attributes.set_blur_radius(interner, shadow.blur_radius);
    attributes.set_distance(interner, shadow.distance);
    attributes.set_direction(interner, shadow.direction);
    attributes.set_scale_x(interner, shadow.scale_x);
    attributes.set_scale_y(interner, shadow.scale_y);
    attributes.set_skew_x(interner, shadow.skew_x);
    attributes.set_skew_y(interner, shadow.skew_y);
    attributes.set_alignment(interner, shadow.alignment);
    attributes.set_rotate_with_shape(interner, shadow.rotate_with_shape);
    let mut children = Vec::new();
    push_color(&mut children, interner, &shadow.color);
    dml_element(interner, "outerShdw", attributes.attributes, children)
}

fn build_preset_shadow(interner: &mut Interner, shadow: &PresetShadowEffect) -> RawElement {
    let mut attributes = PresetShadowAttributes {
        attributes: Vec::new(),
    };
    attributes.set_preset(interner, shadow.preset);
    attributes.set_distance(interner, shadow.distance);
    attributes.set_direction(interner, shadow.direction);
    let mut children = Vec::new();
    push_color(&mut children, interner, &shadow.color);
    dml_element(interner, "prstShdw", attributes.attributes, children)
}

fn build_reflection(interner: &mut Interner, reflection: &ReflectionEffect) -> RawElement {
    let mut attributes = ReflectionAttributes {
        attributes: Vec::new(),
    };
    attributes.set_blur_radius(interner, reflection.blur_radius);
    attributes.set_start_alpha(interner, reflection.start_alpha);
    attributes.set_start_position(interner, reflection.start_position);
    attributes.set_end_alpha(interner, reflection.end_alpha);
    attributes.set_end_position(interner, reflection.end_position);
    attributes.set_distance(interner, reflection.distance);
    attributes.set_direction(interner, reflection.direction);
    attributes.set_fade_direction(interner, reflection.fade_direction);
    attributes.set_scale_x(interner, reflection.scale_x);
    attributes.set_scale_y(interner, reflection.scale_y);
    attributes.set_skew_x(interner, reflection.skew_x);
    attributes.set_skew_y(interner, reflection.skew_y);
    attributes.set_alignment(interner, reflection.alignment);
    attributes.set_rotate_with_shape(interner, reflection.rotate_with_shape);
    dml_element(interner, "reflection", attributes.attributes, Vec::new())
}

fn build_soft_edge(interner: &mut Interner, soft_edge: &SoftEdgeEffect) -> RawElement {
    let mut attributes = SoftEdgeAttributes {
        attributes: Vec::new(),
    };
    attributes.set_radius(interner, soft_edge.radius);
    dml_element(interner, "softEdge", attributes.attributes, Vec::new())
}
