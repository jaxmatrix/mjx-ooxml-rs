//! DrawingML color resolution — baking a [`Color`] down to a concrete RGB.
//!
//! A shape's color and the theme's [`ColorScheme`] live in **different part interners**, so the theme
//! scheme is first resolved to an interner-free [`SchemeColors`] (each slot → RGB). [`resolve_color`]
//! then resolves a color against that and a [`ColorMap`], all in the shape's own interner:
//! `srgbClr`/`sysClr`/`scrgbClr`/`hslClr`/`prstClr` directly, and `schemeClr` through the color map into
//! a scheme slot (or, for `phClr`, into a substituted placeholder color).
//!
//! **Scope (this stage):** color **transforms** (`a:lumMod`, `a:shade`, …) are not yet applied — a
//! color that carries any transform child resolves to `None` here, to avoid silently returning a wrong
//! value. Applying the transform math on top of the base is the next step.

use mjx_ooxml_core::Interner;

use crate::build::{attr_str, parse_angle, parse_percentage};
use crate::color::{Color, ColorKind, SchemeColor};
use crate::style::ColorMap;
use crate::theme::{ColorScheme, ColorSchemeSlot};

/// A fully resolved color: 8-bit sRGB channels plus an alpha in `0.0..=1.0` (always `1.0` until the
/// alpha transforms are applied).
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
/// A slot whose color could not be resolved at this stage (e.g. it carries a transform) is omitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemeColors {
    slots: Vec<(ColorSchemeSlot, [u8; 3])>,
}

impl SchemeColors {
    /// Resolves each slot of `scheme` (parsed with `interner`) to its base RGB.
    #[must_use]
    pub fn from_scheme(scheme: &ColorScheme, interner: &Interner) -> Self {
        let slots = scheme
            .slots()
            .filter_map(|(slot, color)| Some((slot, concrete_rgb(color, interner)?)))
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
/// `map`, with `placeholder` supplying the substitute for a `phClr` reference (a shape's `a:fillRef`
/// color). `interner` is the interner of `color` / `placeholder` (the shape's part).
///
/// Returns `None` when the color cannot be resolved: an unknown/absent value, a `phClr` with no
/// `placeholder`, a scheme slot the theme does not define — or (this stage) **any color that carries a
/// transform child**, since the transform math is not applied yet.
#[must_use]
pub fn resolve_color(
    color: &Color,
    scheme: &SchemeColors,
    map: &ColorMap,
    placeholder: Option<&Color>,
    interner: &Interner,
) -> Option<ResolvedColor> {
    let [red, green, blue] = base_rgb(color, scheme, map, placeholder, interner)?;
    Some(ResolvedColor {
        red,
        green,
        blue,
        alpha: 1.0,
    })
}

/// The base RGB of `color` (before transforms): `schemeClr` resolves through the map into a scheme
/// slot or (for `phClr`) the placeholder; every other kind is a concrete color.
fn base_rgb(
    color: &Color,
    scheme: &SchemeColors,
    map: &ColorMap,
    placeholder: Option<&Color>,
    interner: &Interner,
) -> Option<[u8; 3]> {
    if !color.transforms().is_empty() {
        return None; // transforms deferred to the next stage
    }
    if color.kind(interner) == ColorKind::Scheme {
        return match color.scheme_color(interner)? {
            SchemeColor::PlaceholderColor => base_rgb(placeholder?, scheme, map, None, interner),
            other => scheme.rgb(map.resolve(other)?),
        };
    }
    concrete_rgb(color, interner)
}

/// The base RGB of a **concrete** color (`srgbClr`/`sysClr`/`scrgbClr`/`hslClr`/`prstClr`); `None` for
/// a `schemeClr` / unknown element or a transform-bearing color.
fn concrete_rgb(color: &Color, interner: &Interner) -> Option<[u8; 3]> {
    if !color.transforms().is_empty() {
        return None;
    }
    match color.kind(interner) {
        ColorKind::Srgb => hex_to_rgb(color.value(interner)?),
        ColorKind::System => hex_to_rgb(attr_str(color.attributes(), interner, "lastClr")?),
        ColorKind::ScRgb => {
            let r = channel_percentage(color, interner, "r")?;
            let g = channel_percentage(color, interner, "g")?;
            let b = channel_percentage(color, interner, "b")?;
            Some(scrgb_to_srgb(r, g, b))
        }
        ColorKind::Hsl => {
            let hue = attr_str(color.attributes(), interner, "hue")
                .and_then(parse_angle)?
                .degrees();
            let sat = channel_percentage(color, interner, "sat")?;
            let lum = channel_percentage(color, interner, "lum")?;
            Some(hsl_to_rgb(hue, sat, lum))
        }
        ColorKind::Preset => preset_color_rgb(color.value(interner)?),
        ColorKind::Scheme | ColorKind::Unknown => None,
    }
}

/// Reads a color's percentage-valued attribute (`r`/`g`/`b` of `scrgbClr`, `sat`/`lum` of `hslClr`) as
/// a ratio (`1.0` = 100%).
fn channel_percentage(color: &Color, interner: &Interner, local: &str) -> Option<f64> {
    attr_str(color.attributes(), interner, local)
        .and_then(parse_percentage)
        .map(|fraction| fraction.ratio())
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

/// Converts a linear-light channel (`0.0..=1.0`) to an 8-bit sRGB value (sRGB gamma encoding).
fn linear_to_srgb_byte(linear: f64) -> u8 {
    let c = linear.clamp(0.0, 1.0);
    let encoded = if c <= 0.003_130_8 {
        12.92 * c
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (encoded * 255.0).round() as u8
}

/// Converts an `scrgbClr` (linear RGB percentages, `1.0` = 100%) to 8-bit sRGB.
fn scrgb_to_srgb(r: f64, g: f64, b: f64) -> [u8; 3] {
    [
        linear_to_srgb_byte(r),
        linear_to_srgb_byte(g),
        linear_to_srgb_byte(b),
    ]
}

/// Converts an `hslClr` (`hue` in degrees, `sat`/`lum` as ratios) to 8-bit sRGB via the standard
/// HSL→RGB algorithm.
fn hsl_to_rgb(hue_degrees: f64, sat: f64, lum: f64) -> [u8; 3] {
    let s = sat.clamp(0.0, 1.0);
    let l = lum.clamp(0.0, 1.0);
    let h = hue_degrees.rem_euclid(360.0) / 360.0;
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return [v, v, v];
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    let channel = |t: f64| -> u8 {
        let mut t = t.rem_euclid(1.0);
        let value = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 1.0 / 2.0 {
            q
        } else if t < 2.0 / 3.0 {
            t = 2.0 / 3.0 - t;
            p + (q - p) * 6.0 * t
        } else {
            p
        };
        (value * 255.0).round() as u8
    };
    [channel(h + 1.0 / 3.0), channel(h), channel(h - 1.0 / 3.0)]
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
}
