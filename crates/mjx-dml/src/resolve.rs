//! DrawingML color resolution — baking a [`Color`] down to a concrete RGB.
//!
//! A shape's color and the theme's [`ColorScheme`] live in **different part interners**, so the theme
//! scheme is first resolved to an interner-free [`SchemeColors`] (each slot → RGB). [`resolve_color`]
//! then resolves a color against that and a [`ColorMap`], all in the shape's own interner:
//! `srgbClr`/`sysClr`/`scrgbClr`/`hslClr`/`prstClr` directly, and `schemeClr` through the color map into
//! a scheme slot (or, for `phClr`, into a substituted placeholder color).
//!
//! **Color transforms** (`EG_ColorTransform`: `a:lumMod`, `a:shade`, `a:alpha`, …) are applied on top
//! of the base, in document order, at every level of the chain (the reference, the `phClr` placeholder,
//! and each scheme slot can carry their own). The common transforms (`lumMod`/`lumOff`/`shade`/`tint`/
//! `alpha`/`sat*`) follow the widely-adopted Apache-POI / LibreOffice algorithm and are value-pinned in
//! the tests; the rarely-seen `comp`/`gray`/`gamma`/`invGamma` follow a documented interpretation and
//! are **not** guaranteed pixel-identical to Microsoft Office's renderer.

use mjx_ooxml_core::{Interner, RawNode};

use crate::build::{attr_str, parse_angle, parse_percentage};
use crate::color::{Color, ColorKind, ColorSpec, SchemeColor};
use crate::effect::{EffectList, EffectListSpec};
use crate::fill::{Fill, FillSpec, GradientStopSpec};
use crate::line::{LineProperties, LineSpec};
use crate::style::ColorMap;
use crate::theme::{ColorScheme, ColorSchemeSlot};

/// A fully resolved color: 8-bit sRGB channels plus an alpha (opacity) in `0.0..=1.0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedColor {
    /// The red channel (`0..=255`).
    pub red: u8,
    /// The green channel (`0..=255`).
    pub green: u8,
    /// The blue channel (`0..=255`).
    pub blue: u8,
    /// The alpha (opacity), `0.0` (transparent) to `1.0` (opaque).
    pub alpha: f64,
}

impl ResolvedColor {
    /// The color as a 6-digit uppercase `RRGGBB` hex string (alpha is not encoded).
    #[must_use]
    pub fn to_hex(self) -> String {
        format!("{:02X}{:02X}{:02X}", self.red, self.green, self.blue)
    }
}

/// A theme color scheme resolved to concrete RGB per slot — the interner-free bridge that lets a
/// color from one part be resolved against a theme in another part. Build it once from a
/// [`ColorScheme`] with the theme part's interner, then pass it to [`resolve_color`].
///
/// A slot whose color could not be resolved (an unrecognized value) is omitted. The [`Default`] is an
/// empty scheme (no slots defined) — resolution then yields `None` for any scheme color.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemeColors {
    slots: Vec<(ColorSchemeSlot, [u8; 3])>,
}

impl SchemeColors {
    /// Resolves each slot of `scheme` (parsed with `interner`) to its RGB, honoring any transforms a
    /// slot color carries.
    #[must_use]
    pub fn from_scheme(scheme: &ColorScheme, interner: &Interner) -> Self {
        let slots = scheme
            .slots()
            .filter_map(|(slot, color)| {
                let base = concrete_base(color, interner)?;
                let (rgb, _alpha) = apply_transforms(base, 1.0, color.transforms(), interner);
                Some((slot, floats_to_bytes(rgb)))
            })
            .collect();
        Self { slots }
    }

    /// The RGB of `slot`, or `None` if the scheme did not define (or could not resolve) it.
    #[must_use]
    fn rgb(&self, slot: ColorSchemeSlot) -> Option<[u8; 3]> {
        self.slots
            .iter()
            .find_map(|(candidate, rgb)| (*candidate == slot).then_some(*rgb))
    }
}

/// Resolves `color` to a concrete [`ResolvedColor`] against the resolved theme `scheme` and color
/// `map`, with `placeholder` supplying the already-resolved substitute for a `phClr` reference (a
/// shape's resolved `a:fillRef` color). `interner` is the interner of `color` (the shape's or theme's
/// part) — `placeholder` is interner-free, so `color` and its `phClr` substitute may come from
/// different parts.
///
/// Returns `None` when the color cannot be resolved: an unknown/absent value, a `phClr` with no
/// `placeholder`, or a scheme slot the theme does not define.
#[must_use]
pub fn resolve_color(
    color: &Color,
    scheme: &SchemeColors,
    map: &ColorMap,
    placeholder: Option<ResolvedColor>,
    interner: &Interner,
) -> Option<ResolvedColor> {
    let (rgb, alpha) = resolve_rgba(color, scheme, map, placeholder, interner)?;
    let [red, green, blue] = floats_to_bytes(rgb);
    Some(ResolvedColor {
        red,
        green,
        blue,
        alpha,
    })
}

/// Resolves every color of `fill` to concrete RGB, producing an interner-free [`FillSpec`] whose
/// colors are [`ColorSpec::Srgb`] hex values. `placeholder` is the resolved `phClr` substitute (a
/// shape's `a:fillRef` color) used when `fill` is a theme fill-style; pass `None` for an explicit
/// shape fill. A color that cannot be resolved falls back to its own (unresolved) [`ColorSpec`].
///
/// Note: [`FillSpec`] colors are RGB-only, so a resolved alpha (from an `a:alpha` transform) is not
/// represented in the result.
#[must_use]
pub fn resolve_fill(
    fill: &Fill,
    scheme: &SchemeColors,
    map: &ColorMap,
    placeholder: Option<ResolvedColor>,
    interner: &Interner,
) -> FillSpec {
    let to_spec = |color: &Color| -> ColorSpec {
        resolve_color(color, scheme, map, placeholder, interner).map_or_else(
            || color.spec(interner),
            |resolved| ColorSpec::Srgb(resolved.to_hex()),
        )
    };
    match fill {
        Fill::None(_) => FillSpec::None,
        Fill::Group(_) => FillSpec::Group,
        Fill::Solid(solid) => FillSpec::Solid(solid.color().map_or(
            ColorSpec::Other {
                kind: ColorKind::Unknown,
                value: None,
            },
            to_spec,
        )),
        Fill::Gradient(gradient) => FillSpec::Gradient {
            stops: gradient
                .stops(interner)
                .iter()
                .map(|stop| GradientStopSpec {
                    position: stop.position,
                    color: to_spec(&stop.color),
                })
                .collect(),
            angle: gradient.linear_angle(interner),
        },
        Fill::Blip(blip) => FillSpec::Blip {
            rel_id: blip
                .image_rel_id(interner)
                .or_else(|| blip.image_link_id(interner))
                .unwrap_or_default()
                .to_owned(),
            mode: blip.mode(interner),
        },
        Fill::Pattern(pattern) => FillSpec::Pattern {
            preset: pattern.preset(interner),
            foreground: pattern.foreground(interner).map(|color| to_spec(&color)),
            background: pattern.background(interner).map(|color| to_spec(&color)),
        },
    }
}

/// Resolves `line`'s stroke color to concrete RGB, producing an interner-free [`LineSpec`]. The
/// width/cap/compound/pen-alignment/dash/join/end attributes are copied verbatim; the stroke fill is
/// baked via [`resolve_fill`]. `placeholder` is the resolved `phClr` substitute (a shape's `a:lnRef`
/// color) used when `line` is a theme line-style; pass `None` for an explicit shape outline.
#[must_use]
pub fn resolve_line(
    line: &LineProperties,
    scheme: &SchemeColors,
    map: &ColorMap,
    placeholder: Option<ResolvedColor>,
    interner: &Interner,
) -> LineSpec {
    LineSpec {
        width: line.width(interner),
        cap: line.cap(interner),
        compound: line.compound(interner),
        pen_alignment: line.pen_alignment(interner),
        fill: line
            .fill(interner)
            .map(|fill| resolve_fill(&fill, scheme, map, placeholder, interner)),
        dash: line.dash(interner),
        join: line.join(interner),
        head_end: line.head_end(interner),
        tail_end: line.tail_end(interner),
    }
}

/// Resolves the colors of `effects` to concrete RGB, producing an interner-free [`EffectListSpec`]. The
/// structural fields (radii, distances, angles, scales, alignment, blend, preset) are copied verbatim;
/// each colored effect's `EG_ColorChoice` is baked via [`resolve_color`] and the fill overlay via
/// [`resolve_fill`]. `placeholder` is the resolved `phClr` substitute (a shape's `a:effectRef` color)
/// used when `effects` is a theme effect-style; pass `None` for an explicit shape effect list.
///
/// A color that cannot be resolved falls back to its own (unresolved) [`ColorSpec`]. Resolved alpha
/// (from an `a:alpha` transform) is not represented, as effect colors are RGB-only.
#[must_use]
pub fn resolve_effects(
    effects: &EffectList,
    scheme: &SchemeColors,
    map: &ColorMap,
    placeholder: Option<ResolvedColor>,
    interner: &Interner,
) -> EffectListSpec {
    let to_spec = |color: &Color| -> ColorSpec {
        resolve_color(color, scheme, map, placeholder, interner).map_or_else(
            || color.spec(interner),
            |resolved| ColorSpec::Srgb(resolved.to_hex()),
        )
    };

    let mut spec = effects.spec(interner);
    if let (Some(glow), Some(color)) = (spec.glow.as_mut(), effects.glow_color(interner)) {
        glow.color = to_spec(&color);
    }
    if let (Some(shadow), Some(color)) = (
        spec.inner_shadow.as_mut(),
        effects.inner_shadow_color(interner),
    ) {
        shadow.color = to_spec(&color);
    }
    if let (Some(shadow), Some(color)) = (
        spec.outer_shadow.as_mut(),
        effects.outer_shadow_color(interner),
    ) {
        shadow.color = to_spec(&color);
    }
    if let (Some(shadow), Some(color)) = (
        spec.preset_shadow.as_mut(),
        effects.preset_shadow_color(interner),
    ) {
        shadow.color = to_spec(&color);
    }
    if let (Some(overlay), Some(fill)) = (
        spec.fill_overlay.as_mut(),
        effects.fill_overlay_fill(interner),
    ) {
        overlay.fill = resolve_fill(&fill, scheme, map, placeholder, interner);
    }
    spec
}

/// Resolves `color` to sRGB floats (`0.0..=1.0`) + alpha, applying transforms at every level:
/// `schemeClr` resolves through the map into a scheme slot or (for `phClr`) the pre-resolved
/// placeholder, then this node's own transforms apply on top.
fn resolve_rgba(
    color: &Color,
    scheme: &SchemeColors,
    map: &ColorMap,
    placeholder: Option<ResolvedColor>,
    interner: &Interner,
) -> Option<([f64; 3], f64)> {
    let (base_rgb, base_alpha) = if color.kind(interner) == ColorKind::Scheme {
        match color.scheme_color(interner)? {
            SchemeColor::PlaceholderColor => {
                let ph = placeholder?;
                (bytes_to_floats([ph.red, ph.green, ph.blue]), ph.alpha)
            }
            other => (bytes_to_floats(scheme.rgb(map.resolve(other)?)?), 1.0),
        }
    } else {
        (concrete_base(color, interner)?, 1.0)
    };
    Some(apply_transforms(
        base_rgb,
        base_alpha,
        color.transforms(),
        interner,
    ))
}

/// The base sRGB (`0.0..=1.0`) of a **concrete** color (`srgbClr`/`sysClr`/`scrgbClr`/`hslClr`/
/// `prstClr`), before any transforms; `None` for a `schemeClr` / unknown element.
fn concrete_base(color: &Color, interner: &Interner) -> Option<[f64; 3]> {
    Some(match color.kind(interner) {
        ColorKind::Srgb => bytes_to_floats(hex_to_rgb(color.value(interner)?)?),
        ColorKind::System => bytes_to_floats(hex_to_rgb(attr_str(
            color.attributes(),
            interner,
            "lastClr",
        )?)?),
        ColorKind::ScRgb => {
            let r = channel_percentage(color, interner, "r")?;
            let g = channel_percentage(color, interner, "g")?;
            let b = channel_percentage(color, interner, "b")?;
            [linear_to_srgb(r), linear_to_srgb(g), linear_to_srgb(b)]
        }
        ColorKind::Hsl => {
            let hue = attr_str(color.attributes(), interner, "hue")
                .and_then(parse_angle)?
                .degrees();
            let sat = channel_percentage(color, interner, "sat")?;
            let lum = channel_percentage(color, interner, "lum")?;
            hsl_to_rgb_f64(hue, sat, lum)
        }
        ColorKind::Preset => bytes_to_floats(preset_color_rgb(color.value(interner)?)?),
        ColorKind::Scheme | ColorKind::Unknown => return None,
    })
}

/// Reads a color's percentage-valued attribute (`r`/`g`/`b` of `scrgbClr`, `sat`/`lum` of `hslClr`) as
/// a ratio (`1.0` = 100%).
fn channel_percentage(color: &Color, interner: &Interner, local: &str) -> Option<f64> {
    attr_str(color.attributes(), interner, local)
        .and_then(parse_percentage)
        .map(|fraction| fraction.ratio())
}

// ---------------------------------------------------------------------------------------------
// Color transforms (EG_ColorTransform)
// ---------------------------------------------------------------------------------------------

/// Applies the transform children (`EG_ColorTransform`) to a base color, in document order. `rgb` is
/// sRGB `0.0..=1.0`, `alpha` is `0.0..=1.0`; each transform converts to the space it is defined in and
/// back. Unrecognized transform elements are ignored.
fn apply_transforms(
    mut rgb: [f64; 3],
    mut alpha: f64,
    transforms: &[RawNode],
    interner: &Interner,
) -> ([f64; 3], f64) {
    for node in transforms {
        let RawNode::Element(element) = node else {
            continue;
        };
        let local = interner.resolve(element.name.local);
        let value = attr_str(&element.attributes, interner, "val");
        let percent = || value.and_then(parse_percentage).map(|f| f.ratio());
        let angle = || value.and_then(parse_angle).map(|a| a.degrees());

        match local {
            "alpha" => {
                if let Some(p) = percent() {
                    alpha = p;
                }
            }
            "alphaMod" => {
                if let Some(p) = percent() {
                    alpha *= p;
                }
            }
            "alphaOff" => {
                if let Some(p) = percent() {
                    alpha += p;
                }
            }
            "lum" | "lumMod" | "lumOff" | "sat" | "satMod" | "satOff" | "hue" | "hueMod"
            | "hueOff" => {
                let (mut h, mut s, mut l) = rgb_to_hsl(rgb);
                match local {
                    "lum" => {
                        if let Some(p) = percent() {
                            l = p;
                        }
                    }
                    "lumMod" => {
                        if let Some(p) = percent() {
                            l *= p;
                        }
                    }
                    "lumOff" => {
                        if let Some(p) = percent() {
                            l += p;
                        }
                    }
                    "sat" => {
                        if let Some(p) = percent() {
                            s = p;
                        }
                    }
                    "satMod" => {
                        if let Some(p) = percent() {
                            s *= p;
                        }
                    }
                    "satOff" => {
                        if let Some(p) = percent() {
                            s += p;
                        }
                    }
                    "hue" => {
                        if let Some(a) = angle() {
                            h = a;
                        }
                    }
                    "hueMod" => {
                        if let Some(p) = percent() {
                            h *= p;
                        }
                    }
                    "hueOff" => {
                        if let Some(a) = angle() {
                            h += a;
                        }
                    }
                    _ => {}
                }
                rgb = hsl_to_rgb_f64(h, s.clamp(0.0, 1.0), l.clamp(0.0, 1.0));
            }
            "shade" => {
                if let Some(p) = percent() {
                    rgb = map_channels(rgb, |c| linear_to_srgb(srgb_to_linear(c) * p));
                }
            }
            "tint" => {
                if let Some(p) = percent() {
                    rgb = map_channels(rgb, |c| linear_to_srgb(srgb_to_linear(c) * p + (1.0 - p)));
                }
            }
            "red" => channel_op(&mut rgb[0], percent(), ChannelOp::Set),
            "redMod" => channel_op(&mut rgb[0], percent(), ChannelOp::Mul),
            "redOff" => channel_op(&mut rgb[0], percent(), ChannelOp::Add),
            "green" => channel_op(&mut rgb[1], percent(), ChannelOp::Set),
            "greenMod" => channel_op(&mut rgb[1], percent(), ChannelOp::Mul),
            "greenOff" => channel_op(&mut rgb[1], percent(), ChannelOp::Add),
            "blue" => channel_op(&mut rgb[2], percent(), ChannelOp::Set),
            "blueMod" => channel_op(&mut rgb[2], percent(), ChannelOp::Mul),
            "blueOff" => channel_op(&mut rgb[2], percent(), ChannelOp::Add),
            "inv" => rgb = map_channels(rgb, |c| 1.0 - c),
            "gray" => {
                let y = 0.299 * rgb[0] + 0.587 * rgb[1] + 0.114 * rgb[2];
                rgb = [y, y, y];
            }
            "comp" => {
                let max = rgb[0].max(rgb[1]).max(rgb[2]);
                let min = rgb[0].min(rgb[1]).min(rgb[2]);
                rgb = map_channels(rgb, |c| max + min - c);
            }
            "gamma" => rgb = map_channels(rgb, linear_to_srgb),
            "invGamma" => rgb = map_channels(rgb, srgb_to_linear),
            _ => {} // unknown transform: leave the color unchanged
        }

        rgb = map_channels(rgb, |c| c.clamp(0.0, 1.0));
        alpha = alpha.clamp(0.0, 1.0);
    }
    (rgb, alpha)
}

/// A per-channel set / multiply / offset (`red`/`redMod`/`redOff` and the green/blue equivalents).
enum ChannelOp {
    Set,
    Mul,
    Add,
}

fn channel_op(channel: &mut f64, value: Option<f64>, op: ChannelOp) {
    if let Some(p) = value {
        *channel = match op {
            ChannelOp::Set => p,
            ChannelOp::Mul => *channel * p,
            ChannelOp::Add => *channel + p,
        };
    }
}

/// Applies `f` to each of the three channels.
fn map_channels(rgb: [f64; 3], f: impl Fn(f64) -> f64) -> [f64; 3] {
    [f(rgb[0]), f(rgb[1]), f(rgb[2])]
}

// ---------------------------------------------------------------------------------------------
// Color-space conversions
// ---------------------------------------------------------------------------------------------

/// Scales sRGB floats (`0.0..=1.0`) to 8-bit, rounding and clamping.
fn floats_to_bytes(rgb: [f64; 3]) -> [u8; 3] {
    [
        (rgb[0].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgb[1].clamp(0.0, 1.0) * 255.0).round() as u8,
        (rgb[2].clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

/// Scales 8-bit sRGB to floats (`0.0..=1.0`).
fn bytes_to_floats(rgb: [u8; 3]) -> [f64; 3] {
    [
        f64::from(rgb[0]) / 255.0,
        f64::from(rgb[1]) / 255.0,
        f64::from(rgb[2]) / 255.0,
    ]
}

/// Parses a `RRGGBB` hex string (an optional leading `#` is tolerated) into RGB bytes.
fn hex_to_rgb(s: &str) -> Option<[u8; 3]> {
    let hex = s.strip_prefix('#').unwrap_or(s).trim();
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([red, green, blue])
}

/// Encodes a linear-light channel (`0.0..=1.0`) to sRGB (`0.0..=1.0`), the sRGB gamma curve.
fn linear_to_srgb(linear: f64) -> f64 {
    let c = linear.clamp(0.0, 1.0);
    if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

/// Decodes an sRGB channel (`0.0..=1.0`) to linear light (the inverse of [`linear_to_srgb`]).
fn srgb_to_linear(srgb: f64) -> f64 {
    let c = srgb.clamp(0.0, 1.0);
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Converts an `hslClr` (`hue` in degrees, `sat`/`lum` as ratios) to sRGB floats via the standard
/// HSL→RGB algorithm.
fn hsl_to_rgb_f64(hue_degrees: f64, sat: f64, lum: f64) -> [f64; 3] {
    let s = sat.clamp(0.0, 1.0);
    let l = lum.clamp(0.0, 1.0);
    let h = hue_degrees.rem_euclid(360.0) / 360.0;
    if s == 0.0 {
        return [l, l, l];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |t: f64| -> f64 {
        let mut t = t.rem_euclid(1.0);
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            t = 2.0 / 3.0 - t;
            p + (q - p) * 6.0 * t
        } else {
            p
        }
    };
    [channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
}

/// Converts sRGB floats to `(hue_degrees, sat, lum)` — the inverse of [`hsl_to_rgb_f64`].
fn rgb_to_hsl(rgb: [f64; 3]) -> (f64, f64, f64) {
    let [r, g, b] = rgb;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if (max - min).abs() < f64::EPSILON {
        return (0.0, 0.0, l);
    }
    let d = max - min;
    let s = if l > 0.5 {
        d / (2.0 - max - min)
    } else {
        d / (max + min)
    };
    let h = if (max - r).abs() < f64::EPSILON {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if (max - g).abs() < f64::EPSILON {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    };
    (h * 60.0, s, l)
}

/// A `[u8; 3]` sRGB → HSL helper retained for the in-module test.
#[cfg(test)]
fn hsl_to_rgb(hue_degrees: f64, sat: f64, lum: f64) -> [u8; 3] {
    floats_to_bytes(hsl_to_rgb_f64(hue_degrees, sat, lum))
}

/// The RGB of an `a:prstClr` preset color name, or `None` if the token is unrecognized.
fn preset_color_rgb(wire: &str) -> Option<[u8; 3]> {
    PRESET_COLORS
        .iter()
        .find_map(|(name, rgb)| (*name == wire).then_some(*rgb))
}

/// The 190 `ST_PresetColorVal` named colors → RGB (ECMA-376 Part 1 §20.1.10.47; the SVG/X11 palette,
/// with the OOXML `dk`/`lt`/`med` abbreviations aliasing their `dark`/`light`/`medium` values).
const PRESET_COLORS: &[(&str, [u8; 3])] = &[
    ("aliceBlue", [0xF0, 0xF8, 0xFF]),
    ("antiqueWhite", [0xFA, 0xEB, 0xD7]),
    ("aqua", [0x00, 0xFF, 0xFF]),
    ("aquamarine", [0x7F, 0xFF, 0xD4]),
    ("azure", [0xF0, 0xFF, 0xFF]),
    ("beige", [0xF5, 0xF5, 0xDC]),
    ("bisque", [0xFF, 0xE4, 0xC4]),
    ("black", [0x00, 0x00, 0x00]),
    ("blanchedAlmond", [0xFF, 0xEB, 0xCD]),
    ("blue", [0x00, 0x00, 0xFF]),
    ("blueViolet", [0x8A, 0x2B, 0xE2]),
    ("brown", [0xA5, 0x2A, 0x2A]),
    ("burlyWood", [0xDE, 0xB8, 0x87]),
    ("cadetBlue", [0x5F, 0x9E, 0xA0]),
    ("chartreuse", [0x7F, 0xFF, 0x00]),
    ("chocolate", [0xD2, 0x69, 0x1E]),
    ("coral", [0xFF, 0x7F, 0x50]),
    ("cornflowerBlue", [0x64, 0x95, 0xED]),
    ("cornsilk", [0xFF, 0xF8, 0xDC]),
    ("crimson", [0xDC, 0x14, 0x3C]),
    ("cyan", [0x00, 0xFF, 0xFF]),
    ("darkBlue", [0x00, 0x00, 0x8B]),
    ("darkCyan", [0x00, 0x8B, 0x8B]),
    ("darkGoldenrod", [0xB8, 0x86, 0x0B]),
    ("darkGray", [0xA9, 0xA9, 0xA9]),
    ("darkGrey", [0xA9, 0xA9, 0xA9]),
    ("darkGreen", [0x00, 0x64, 0x00]),
    ("darkKhaki", [0xBD, 0xB7, 0x6B]),
    ("darkMagenta", [0x8B, 0x00, 0x8B]),
    ("darkOliveGreen", [0x55, 0x6B, 0x2F]),
    ("darkOrange", [0xFF, 0x8C, 0x00]),
    ("darkOrchid", [0x99, 0x32, 0xCC]),
    ("darkRed", [0x8B, 0x00, 0x00]),
    ("darkSalmon", [0xE9, 0x96, 0x7A]),
    ("darkSeaGreen", [0x8F, 0xBC, 0x8F]),
    ("darkSlateBlue", [0x48, 0x3D, 0x8B]),
    ("darkSlateGray", [0x2F, 0x4F, 0x4F]),
    ("darkSlateGrey", [0x2F, 0x4F, 0x4F]),
    ("darkTurquoise", [0x00, 0xCE, 0xD1]),
    ("darkViolet", [0x94, 0x00, 0xD3]),
    ("dkBlue", [0x00, 0x00, 0x8B]),
    ("dkCyan", [0x00, 0x8B, 0x8B]),
    ("dkGoldenrod", [0xB8, 0x86, 0x0B]),
    ("dkGray", [0xA9, 0xA9, 0xA9]),
    ("dkGrey", [0xA9, 0xA9, 0xA9]),
    ("dkGreen", [0x00, 0x64, 0x00]),
    ("dkKhaki", [0xBD, 0xB7, 0x6B]),
    ("dkMagenta", [0x8B, 0x00, 0x8B]),
    ("dkOliveGreen", [0x55, 0x6B, 0x2F]),
    ("dkOrange", [0xFF, 0x8C, 0x00]),
    ("dkOrchid", [0x99, 0x32, 0xCC]),
    ("dkRed", [0x8B, 0x00, 0x00]),
    ("dkSalmon", [0xE9, 0x96, 0x7A]),
    ("dkSeaGreen", [0x8F, 0xBC, 0x8F]),
    ("dkSlateBlue", [0x48, 0x3D, 0x8B]),
    ("dkSlateGray", [0x2F, 0x4F, 0x4F]),
    ("dkSlateGrey", [0x2F, 0x4F, 0x4F]),
    ("dkTurquoise", [0x00, 0xCE, 0xD1]),
    ("dkViolet", [0x94, 0x00, 0xD3]),
    ("deepPink", [0xFF, 0x14, 0x93]),
    ("deepSkyBlue", [0x00, 0xBF, 0xFF]),
    ("dimGray", [0x69, 0x69, 0x69]),
    ("dimGrey", [0x69, 0x69, 0x69]),
    ("dodgerBlue", [0x1E, 0x90, 0xFF]),
    ("firebrick", [0xB2, 0x22, 0x22]),
    ("floralWhite", [0xFF, 0xFA, 0xF0]),
    ("forestGreen", [0x22, 0x8B, 0x22]),
    ("fuchsia", [0xFF, 0x00, 0xFF]),
    ("gainsboro", [0xDC, 0xDC, 0xDC]),
    ("ghostWhite", [0xF8, 0xF8, 0xFF]),
    ("gold", [0xFF, 0xD7, 0x00]),
    ("goldenrod", [0xDA, 0xA5, 0x20]),
    ("gray", [0x80, 0x80, 0x80]),
    ("grey", [0x80, 0x80, 0x80]),
    ("green", [0x00, 0x80, 0x00]),
    ("greenYellow", [0xAD, 0xFF, 0x2F]),
    ("honeydew", [0xF0, 0xFF, 0xF0]),
    ("hotPink", [0xFF, 0x69, 0xB4]),
    ("indianRed", [0xCD, 0x5C, 0x5C]),
    ("indigo", [0x4B, 0x00, 0x82]),
    ("ivory", [0xFF, 0xFF, 0xF0]),
    ("khaki", [0xF0, 0xE6, 0x8C]),
    ("lavender", [0xE6, 0xE6, 0xFA]),
    ("lavenderBlush", [0xFF, 0xF0, 0xF5]),
    ("lawnGreen", [0x7C, 0xFC, 0x00]),
    ("lemonChiffon", [0xFF, 0xFA, 0xCD]),
    ("lightBlue", [0xAD, 0xD8, 0xE6]),
    ("lightCoral", [0xF0, 0x80, 0x80]),
    ("lightCyan", [0xE0, 0xFF, 0xFF]),
    ("lightGoldenrodYellow", [0xFA, 0xFA, 0xD2]),
    ("lightGray", [0xD3, 0xD3, 0xD3]),
    ("lightGrey", [0xD3, 0xD3, 0xD3]),
    ("lightGreen", [0x90, 0xEE, 0x90]),
    ("lightPink", [0xFF, 0xB6, 0xC1]),
    ("lightSalmon", [0xFF, 0xA0, 0x7A]),
    ("lightSeaGreen", [0x20, 0xB2, 0xAA]),
    ("lightSkyBlue", [0x87, 0xCE, 0xFA]),
    ("lightSlateGray", [0x77, 0x88, 0x99]),
    ("lightSlateGrey", [0x77, 0x88, 0x99]),
    ("lightSteelBlue", [0xB0, 0xC4, 0xDE]),
    ("lightYellow", [0xFF, 0xFF, 0xE0]),
    ("ltBlue", [0xAD, 0xD8, 0xE6]),
    ("ltCoral", [0xF0, 0x80, 0x80]),
    ("ltCyan", [0xE0, 0xFF, 0xFF]),
    ("ltGoldenrodYellow", [0xFA, 0xFA, 0xD2]),
    ("ltGray", [0xD3, 0xD3, 0xD3]),
    ("ltGrey", [0xD3, 0xD3, 0xD3]),
    ("ltGreen", [0x90, 0xEE, 0x90]),
    ("ltPink", [0xFF, 0xB6, 0xC1]),
    ("ltSalmon", [0xFF, 0xA0, 0x7A]),
    ("ltSeaGreen", [0x20, 0xB2, 0xAA]),
    ("ltSkyBlue", [0x87, 0xCE, 0xFA]),
    ("ltSlateGray", [0x77, 0x88, 0x99]),
    ("ltSlateGrey", [0x77, 0x88, 0x99]),
    ("ltSteelBlue", [0xB0, 0xC4, 0xDE]),
    ("ltYellow", [0xFF, 0xFF, 0xE0]),
    ("lime", [0x00, 0xFF, 0x00]),
    ("limeGreen", [0x32, 0xCD, 0x32]),
    ("linen", [0xFA, 0xF0, 0xE6]),
    ("magenta", [0xFF, 0x00, 0xFF]),
    ("maroon", [0x80, 0x00, 0x00]),
    ("medAquamarine", [0x66, 0xCD, 0xAA]),
    ("medBlue", [0x00, 0x00, 0xCD]),
    ("medOrchid", [0xBA, 0x55, 0xD3]),
    ("medPurple", [0x93, 0x70, 0xDB]),
    ("medSeaGreen", [0x3C, 0xB3, 0x71]),
    ("medSlateBlue", [0x7B, 0x68, 0xEE]),
    ("medSpringGreen", [0x00, 0xFA, 0x9A]),
    ("medTurquoise", [0x48, 0xD1, 0xCC]),
    ("medVioletRed", [0xC7, 0x15, 0x85]),
    ("mediumAquamarine", [0x66, 0xCD, 0xAA]),
    ("mediumBlue", [0x00, 0x00, 0xCD]),
    ("mediumOrchid", [0xBA, 0x55, 0xD3]),
    ("mediumPurple", [0x93, 0x70, 0xDB]),
    ("mediumSeaGreen", [0x3C, 0xB3, 0x71]),
    ("mediumSlateBlue", [0x7B, 0x68, 0xEE]),
    ("mediumSpringGreen", [0x00, 0xFA, 0x9A]),
    ("mediumTurquoise", [0x48, 0xD1, 0xCC]),
    ("mediumVioletRed", [0xC7, 0x15, 0x85]),
    ("midnightBlue", [0x19, 0x19, 0x70]),
    ("mintCream", [0xF5, 0xFF, 0xFA]),
    ("mistyRose", [0xFF, 0xE4, 0xE1]),
    ("moccasin", [0xFF, 0xE4, 0xB5]),
    ("navajoWhite", [0xFF, 0xDE, 0xAD]),
    ("navy", [0x00, 0x00, 0x80]),
    ("oldLace", [0xFD, 0xF5, 0xE6]),
    ("olive", [0x80, 0x80, 0x00]),
    ("oliveDrab", [0x6B, 0x8E, 0x23]),
    ("orange", [0xFF, 0xA5, 0x00]),
    ("orangeRed", [0xFF, 0x45, 0x00]),
    ("orchid", [0xDA, 0x70, 0xD6]),
    ("paleGoldenrod", [0xEE, 0xE8, 0xAA]),
    ("paleGreen", [0x98, 0xFB, 0x98]),
    ("paleTurquoise", [0xAF, 0xEE, 0xEE]),
    ("paleVioletRed", [0xDB, 0x70, 0x93]),
    ("papayaWhip", [0xFF, 0xEF, 0xD5]),
    ("peachPuff", [0xFF, 0xDA, 0xB9]),
    ("peru", [0xCD, 0x85, 0x3F]),
    ("pink", [0xFF, 0xC0, 0xCB]),
    ("plum", [0xDD, 0xA0, 0xDD]),
    ("powderBlue", [0xB0, 0xE0, 0xE6]),
    ("purple", [0x80, 0x00, 0x80]),
    ("red", [0xFF, 0x00, 0x00]),
    ("rosyBrown", [0xBC, 0x8F, 0x8F]),
    ("royalBlue", [0x41, 0x69, 0xE1]),
    ("saddleBrown", [0x8B, 0x45, 0x13]),
    ("salmon", [0xFA, 0x80, 0x72]),
    ("sandyBrown", [0xF4, 0xA4, 0x60]),
    ("seaGreen", [0x2E, 0x8B, 0x57]),
    ("seaShell", [0xFF, 0xF5, 0xEE]),
    ("sienna", [0xA0, 0x52, 0x2D]),
    ("silver", [0xC0, 0xC0, 0xC0]),
    ("skyBlue", [0x87, 0xCE, 0xEB]),
    ("slateBlue", [0x6A, 0x5A, 0xCD]),
    ("slateGray", [0x70, 0x80, 0x90]),
    ("slateGrey", [0x70, 0x80, 0x90]),
    ("snow", [0xFF, 0xFA, 0xFA]),
    ("springGreen", [0x00, 0xFF, 0x7F]),
    ("steelBlue", [0x46, 0x82, 0xB4]),
    ("tan", [0xD2, 0xB4, 0x8C]),
    ("teal", [0x00, 0x80, 0x80]),
    ("thistle", [0xD8, 0xBF, 0xD8]),
    ("tomato", [0xFF, 0x63, 0x47]),
    ("turquoise", [0x40, 0xE0, 0xD0]),
    ("violet", [0xEE, 0x82, 0xEE]),
    ("wheat", [0xF5, 0xDE, 0xB3]),
    ("white", [0xFF, 0xFF, 0xFF]),
    ("whiteSmoke", [0xF5, 0xF5, 0xF5]),
    ("yellow", [0xFF, 0xFF, 0x00]),
    ("yellowGreen", [0x9A, 0xCD, 0x32]),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_table_is_complete_and_unique() {
        assert_eq!(PRESET_COLORS.len(), 190);
        let mut names: Vec<&str> = PRESET_COLORS.iter().map(|(n, _)| *n).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), 190, "duplicate preset color name");
    }

    #[test]
    fn hsl_primary_and_grey() {
        assert_eq!(hsl_to_rgb(120.0, 1.0, 0.5), [0, 255, 0]); // pure green
        assert_eq!(hsl_to_rgb(0.0, 0.0, 0.5), [128, 128, 128]); // mid grey
    }
    #[test]
    fn rgb_to_hsl_round_trips() {
        for rgb in [
            [0.2, 0.6, 0.4],
            [1.0, 0.0, 0.0],
            [0.5, 0.5, 0.5],
            [0.1, 0.1, 0.9],
        ] {
            let (h, s, l) = rgb_to_hsl(rgb);
            let back = hsl_to_rgb_f64(h, s, l);
            for i in 0..3 {
                assert!(
                    (rgb[i] - back[i]).abs() < 1e-9,
                    "channel {i}: {:?} -> {:?}",
                    rgb,
                    back
                );
            }
        }
    }
}
