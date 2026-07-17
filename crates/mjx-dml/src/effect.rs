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

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml};
use mjx_ooxml_types::support::on_off;

use crate::build::{
    attr_str, dml_attr, dml_child, dml_element, dml_name, fidelity_element_impls, first_color_child,
    first_fill_child, parse_angle, parse_percentage,
};
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
// Attribute readers/writers (measures & booleans)
// ---------------------------------------------------------------------------------------------

/// Reads an EMU-valued attribute (`ST_(Positive)Coordinate`) as an [`Emu`].
fn attr_emu(attributes: &[RawAttribute], interner: &Interner, name: &str) -> Option<Emu> {
    attr_str(attributes, interner, name)
        .and_then(|s| s.trim().parse::<i64>().ok())
        .map(Emu::from_emu)
}

/// Reads an angle attribute (`ST_(Positive)FixedAngle`, 60000ths of a degree) as an [`Angle`].
fn attr_angle(attributes: &[RawAttribute], interner: &Interner, name: &str) -> Option<Angle> {
    attr_str(attributes, interner, name).and_then(parse_angle)
}

/// Reads a percentage attribute (`ST_Percentage` family) as a [`Fraction`].
fn attr_fraction(attributes: &[RawAttribute], interner: &Interner, name: &str) -> Option<Fraction> {
    attr_str(attributes, interner, name).and_then(parse_percentage)
}

/// Reads a boolean attribute (`xsd:boolean`) — accepting every accepted spelling.
fn attr_bool(attributes: &[RawAttribute], interner: &Interner, name: &str) -> Option<bool> {
    attr_str(attributes, interner, name).and_then(on_off::from_wire)
}

/// Pushes an EMU attribute (native integer form) when set.
fn push_emu(attrs: &mut Vec<RawAttribute>, interner: &mut Interner, name: &str, value: Option<Emu>) {
    if let Some(value) = value {
        attrs.push(dml_attr(interner, name, &value.emu().to_string()));
    }
}

/// Pushes an angle attribute (native 60000ths-of-a-degree form) when set.
fn push_angle(
    attrs: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    name: &str,
    value: Option<Angle>,
) {
    if let Some(value) = value {
        let native = (value.degrees() * 60_000.0).round() as i64;
        attrs.push(dml_attr(interner, name, &native.to_string()));
    }
}

/// Pushes a percentage attribute (native 1000ths-of-a-percent integer form) when set.
fn push_fraction(
    attrs: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    name: &str,
    value: Option<Fraction>,
) {
    if let Some(value) = value {
        let native = (value.ratio() * 100_000.0).round() as i64;
        attrs.push(dml_attr(interner, name, &native.to_string()));
    }
}

/// Pushes a boolean attribute (canonical `true`/`false`) when set.
fn push_bool(
    attrs: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    name: &str,
    value: Option<bool>,
) {
    if let Some(value) = value {
        attrs.push(dml_attr(interner, name, on_off::to_wire(value)));
    }
}

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
// Per-effect readers
// ---------------------------------------------------------------------------------------------

fn read_blur(element: &RawElement, interner: &Interner) -> BlurEffect {
    BlurEffect {
        radius: attr_emu(&element.attributes, interner, "rad"),
        grow: attr_bool(&element.attributes, interner, "grow"),
    }
}

fn read_fill_overlay(element: &RawElement, interner: &Interner) -> Option<FillOverlayEffect> {
    let fill = first_fill_child(&element.children, interner)
        .and_then(|el| Fill::from_xml(el, interner).ok())?
        .spec(interner);
    let blend = attr_str(&element.attributes, interner, "blend").and_then(BlendMode::from_wire)?;
    Some(FillOverlayEffect { fill, blend })
}

fn read_glow(element: &RawElement, interner: &Interner) -> Option<GlowEffect> {
    Some(GlowEffect {
        color: effect_color(element, interner)?,
        radius: attr_emu(&element.attributes, interner, "rad"),
    })
}

fn read_inner_shadow(element: &RawElement, interner: &Interner) -> Option<InnerShadowEffect> {
    Some(InnerShadowEffect {
        color: effect_color(element, interner)?,
        blur_radius: attr_emu(&element.attributes, interner, "blurRad"),
        distance: attr_emu(&element.attributes, interner, "dist"),
        direction: attr_angle(&element.attributes, interner, "dir"),
    })
}

fn read_outer_shadow(element: &RawElement, interner: &Interner) -> Option<OuterShadowEffect> {
    let attrs = &element.attributes;
    Some(OuterShadowEffect {
        color: effect_color(element, interner)?,
        blur_radius: attr_emu(attrs, interner, "blurRad"),
        distance: attr_emu(attrs, interner, "dist"),
        direction: attr_angle(attrs, interner, "dir"),
        scale_x: attr_fraction(attrs, interner, "sx"),
        scale_y: attr_fraction(attrs, interner, "sy"),
        skew_x: attr_angle(attrs, interner, "kx"),
        skew_y: attr_angle(attrs, interner, "ky"),
        alignment: attr_str(attrs, interner, "algn").and_then(RectangleAlignment::from_wire),
        rotate_with_shape: attr_bool(attrs, interner, "rotWithShape"),
    })
}

fn read_preset_shadow(element: &RawElement, interner: &Interner) -> Option<PresetShadowEffect> {
    let attrs = &element.attributes;
    let preset = attr_str(attrs, interner, "prst").and_then(PresetShadow::from_wire)?;
    Some(PresetShadowEffect {
        preset,
        color: effect_color(element, interner)?,
        distance: attr_emu(attrs, interner, "dist"),
        direction: attr_angle(attrs, interner, "dir"),
    })
}

fn read_reflection(element: &RawElement, interner: &Interner) -> ReflectionEffect {
    let attrs = &element.attributes;
    ReflectionEffect {
        blur_radius: attr_emu(attrs, interner, "blurRad"),
        start_alpha: attr_fraction(attrs, interner, "stA"),
        start_position: attr_fraction(attrs, interner, "stPos"),
        end_alpha: attr_fraction(attrs, interner, "endA"),
        end_position: attr_fraction(attrs, interner, "endPos"),
        distance: attr_emu(attrs, interner, "dist"),
        direction: attr_angle(attrs, interner, "dir"),
        fade_direction: attr_angle(attrs, interner, "fadeDir"),
        scale_x: attr_fraction(attrs, interner, "sx"),
        scale_y: attr_fraction(attrs, interner, "sy"),
        skew_x: attr_angle(attrs, interner, "kx"),
        skew_y: attr_angle(attrs, interner, "ky"),
        alignment: attr_str(attrs, interner, "algn").and_then(RectangleAlignment::from_wire),
        rotate_with_shape: attr_bool(attrs, interner, "rotWithShape"),
    }
}

fn read_soft_edge(element: &RawElement, interner: &Interner) -> Option<SoftEdgeEffect> {
    attr_emu(&element.attributes, interner, "rad").map(|radius| SoftEdgeEffect { radius })
}

// ---------------------------------------------------------------------------------------------
// Per-effect builders (attributes emitted in schema declaration order)
// ---------------------------------------------------------------------------------------------

fn build_blur(interner: &mut Interner, blur: &BlurEffect) -> RawElement {
    let mut attrs = Vec::new();
    push_emu(&mut attrs, interner, "rad", blur.radius);
    push_bool(&mut attrs, interner, "grow", blur.grow);
    dml_element(interner, "blur", attrs, Vec::new())
}

fn build_fill_overlay(interner: &mut Interner, effect: &FillOverlayEffect) -> RawElement {
    let attrs = vec![dml_attr(interner, "blend", effect.blend.to_wire())];
    let children = vec![RawNode::Element(effect.fill.to_fill(interner).to_xml(interner))];
    dml_element(interner, "fillOverlay", attrs, children)
}

fn build_glow(interner: &mut Interner, glow: &GlowEffect) -> RawElement {
    let mut attrs = Vec::new();
    push_emu(&mut attrs, interner, "rad", glow.radius);
    let mut children = Vec::new();
    push_color(&mut children, interner, &glow.color);
    dml_element(interner, "glow", attrs, children)
}

fn build_inner_shadow(interner: &mut Interner, shadow: &InnerShadowEffect) -> RawElement {
    let mut attrs = Vec::new();
    push_emu(&mut attrs, interner, "blurRad", shadow.blur_radius);
    push_emu(&mut attrs, interner, "dist", shadow.distance);
    push_angle(&mut attrs, interner, "dir", shadow.direction);
    let mut children = Vec::new();
    push_color(&mut children, interner, &shadow.color);
    dml_element(interner, "innerShdw", attrs, children)
}

fn build_outer_shadow(interner: &mut Interner, shadow: &OuterShadowEffect) -> RawElement {
    let mut attrs = Vec::new();
    push_emu(&mut attrs, interner, "blurRad", shadow.blur_radius);
    push_emu(&mut attrs, interner, "dist", shadow.distance);
    push_angle(&mut attrs, interner, "dir", shadow.direction);
    push_fraction(&mut attrs, interner, "sx", shadow.scale_x);
    push_fraction(&mut attrs, interner, "sy", shadow.scale_y);
    push_angle(&mut attrs, interner, "kx", shadow.skew_x);
    push_angle(&mut attrs, interner, "ky", shadow.skew_y);
    if let Some(alignment) = shadow.alignment {
        attrs.push(dml_attr(interner, "algn", alignment.to_wire()));
    }
    push_bool(&mut attrs, interner, "rotWithShape", shadow.rotate_with_shape);
    let mut children = Vec::new();
    push_color(&mut children, interner, &shadow.color);
    dml_element(interner, "outerShdw", attrs, children)
}

fn build_preset_shadow(interner: &mut Interner, shadow: &PresetShadowEffect) -> RawElement {
    let mut attrs = vec![dml_attr(interner, "prst", shadow.preset.to_wire())];
    push_emu(&mut attrs, interner, "dist", shadow.distance);
    push_angle(&mut attrs, interner, "dir", shadow.direction);
    let mut children = Vec::new();
    push_color(&mut children, interner, &shadow.color);
    dml_element(interner, "prstShdw", attrs, children)
}

fn build_reflection(interner: &mut Interner, reflection: &ReflectionEffect) -> RawElement {
    let mut attrs = Vec::new();
    push_emu(&mut attrs, interner, "blurRad", reflection.blur_radius);
    push_fraction(&mut attrs, interner, "stA", reflection.start_alpha);
    push_fraction(&mut attrs, interner, "stPos", reflection.start_position);
    push_fraction(&mut attrs, interner, "endA", reflection.end_alpha);
    push_fraction(&mut attrs, interner, "endPos", reflection.end_position);
    push_emu(&mut attrs, interner, "dist", reflection.distance);
    push_angle(&mut attrs, interner, "dir", reflection.direction);
    push_angle(&mut attrs, interner, "fadeDir", reflection.fade_direction);
    push_fraction(&mut attrs, interner, "sx", reflection.scale_x);
    push_fraction(&mut attrs, interner, "sy", reflection.scale_y);
    push_angle(&mut attrs, interner, "kx", reflection.skew_x);
    push_angle(&mut attrs, interner, "ky", reflection.skew_y);
    if let Some(alignment) = reflection.alignment {
        attrs.push(dml_attr(interner, "algn", alignment.to_wire()));
    }
    push_bool(&mut attrs, interner, "rotWithShape", reflection.rotate_with_shape);
    dml_element(interner, "reflection", attrs, Vec::new())
}

fn build_soft_edge(interner: &mut Interner, soft_edge: &SoftEdgeEffect) -> RawElement {
    let attrs = vec![dml_attr(interner, "rad", &soft_edge.radius.emu().to_string())];
    dml_element(interner, "softEdge", attrs, Vec::new())
}
