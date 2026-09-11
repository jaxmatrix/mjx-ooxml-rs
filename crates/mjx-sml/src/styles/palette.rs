//! The legacy indexed palette, the theme-position mapping, and the tint — everything needed to turn
//! a [`Color`] into an actual RGB.
//!
//! # `indexed`: sixty-four entries, of which fifty-six are distinct
//!
//! ECMA-376 Part 1 **§18.8.27 `indexedColors` (Color Indexes)** calls it *"a legacy indexing scheme
//! for colors that is still required for some records, and for backwards compatibility with legacy
//! formats"*, and prints the whole table. Two things about it are commonly got wrong:
//!
//! * **It has sixty-four rows, `0` through `63` — not fifty-six.** The spec's own note explains the
//!   arithmetic: *"Note that 0-7 are redundant of 8-15 to preserve backwards compatibility."* The
//!   fifty-six figure counts the distinct BIFF palette (`8`..=`63`); indices `0`..=`7` repeat
//!   `8`..=`15` exactly, and a lookup table that started at `8` would answer `None` for the eight
//!   indices a file is most likely to use for black and white.
//! * **Two further indices exist and are not colours.** The table ends with `indexed="64"` *System
//!   Foreground* and `indexed="65"` *System Background*, both printed with an ARGB of `n/a`. They
//!   resolve to whatever the consumer's system colours are at render time, so
//!   [`IndexedColorPalette::lookup`] answers with [`IndexedColor::SystemForeground`] /
//!   [`IndexedColor::SystemBackground`] rather than inventing one. `bgColor indexed="64"` is on the
//!   second `<fill>` of practically every workbook Excel has ever written.
//!
//! The default values are the spec's own, including its alpha of `00` throughout — this crate
//! reports what Part 1 prints rather than "fixing" it to `FF`.
//!
//! A workbook may replace the whole palette with an `indexedColors` block, and Part 1 is explicit
//! that it is all-or-nothing: *"When using the default indexed color palette, the values are not
//! written out, but instead are implied. When the color palette has been modified from default, then
//! the entire color palette is written out."*
//!
//! # `theme`: a position, not a token — and the two readings of it
//!
//! `CT_Color`'s `@theme` is *"a zero-based index into the `<clrScheme>` collection (§20.1.6.2)"* —
//! the wording is identical on all four SpreadsheetML colour elements (§18.3.1.15 `color`,
//! §18.3.1.93 `tabColor`, §18.8.3 `bgColor`, §18.8.19 `fgColor`) and it is the *only* thing Part 1's
//! prose says about the numbering. §20.1.6.2 is a **DrawingML** clause about `clrScheme` itself, and
//! the table it prints is headed *"Sequence Index"*: `0` `dk1`, `1` `lt1`, `2` `dk2`, `3` `lt2`,
//! `4`–`9` `accent1`–`accent6`, `10` `hlink`, `11` `folHlink`. That is a statement about the
//! **document order of `CT_ColorScheme`'s children**, which is what the clause is describing; that
//! it is also the SpreadsheetML lookup table is an *inference* from the cross-reference, not
//! something either clause states.
//!
//! The inference is wrong on the first two pairs, and this crate used to make it. What actually
//! holds is
//!
//! ```text
//! 0 lt1   1 dk1   2 lt2   3 dk2   4..9 accent1..accent6   10 hlink   11 folHlink
//! ```
//!
//! — the sequence table with each dark/light pair swapped, accents and hyperlinks untouched. That is
//! [`theme_color_slot`], and [`theme_color_position`] is its inverse.
//!
//! ## Why, and how it is checked rather than asserted
//!
//! **The evidence is ECMA's own, and it is markup rather than prose.** The Part 1 5th-edition
//! package ships `OfficeOpenXML-SpreadsheetMLStyles/`, two of whose files are SpreadsheetML styles
//! written by the standard: `presetCellStyles.xml` (the built-in cell styles) and
//! `presetTableStyles.xml` (the built-in table styles). Both spell their colours as `@theme`
//! positions, and both were authored to be **legible**. Three things they say cannot be reconciled
//! with the sequence reading:
//!
//! * every one of the sixty-three `Normal` fonts is `<color theme="1"/>`, drawn on the sheet's white
//!   background — white on white under the sequence reading;
//! * `Accent1`…`Accent6` and `Check Cell` are `<color theme="0"/>` over a mid-tone fill, and a table
//!   style paints `theme="0"` text onto a `theme="0"` fill darkened 35% — under the sequence reading
//!   those are the *same* colour;
//! * `Title` and `Heading 1`…`4` are `<color theme="3"/>` with no fill. Under the sequence reading
//!   that is `lt2`, a pale cream, on a white sheet; under this one it is `dk2`.
//!
//! **The extent of that evidence, stated exactly.** Those three cover positions `1`, `0` and `3`.
//! ECMA's preset styles never name position `2` at all, so `lt2` is reached by the symmetry of the
//! swap rather than by a measurement — the one step here that rests on an argument. The gate in
//! `theme_index.rs` asserts the covered set exactly, so that limit stays visible instead of being
//! rounded off to "the evidence covers the table".
//!
//! So this is not *Office over the specification*: it is the standard's own normative sample data
//! over an inference from a cross-reference in the same edition of the same standard. It happens to
//! agree with what Excel writes and with what every third-party producer in `tests/fixtures/` writes
//! — LibreOffice, Apache POI and XlsxWriter all emit `<color theme="1"/>` for a default dark font —
//! which is the outcome the fidelity rule wants, but the argument does not rest on any of them.
//!
//! `crates/mjx-sml/tests/theme_index.rs` re-derives the mapping from those two files rather than
//! restating it: it resolves every font colour against the ground beside it and requires the two to
//! be tellable apart. ECMA's worst pair separates by 39.7 of 255, so nothing falls under the gate's
//! floor; the sequence reading puts 144 pairs under it, the worst of them exactly 0.0. The same
//! suite holds the *writer* to the same table, by resolving the colour `mjx_sml::write` authors for
//! font 0 back out of the package it wrote. Before MJXOFF-246 the two halves of this crate disagreed
//! and nothing could see it, because both candidate slots are defined in every theme, so the
//! reference gate resolved either way and both the schema gate and the round trip passed.
//!
//! There is no colour *map* in the way DrawingML has one: a workbook has no `clrMapOvr`, so a
//! position goes straight to a scheme slot.
//!
//! # `tint`: the algorithm is in the prose, and it is not `lerp`
//!
//! Part 1 §18.8.19 spells the tint out, in luminance rather than in RGB:
//!
//! ```text
//! In loading the RGB value, it is converted to HLS where HLS values are (0..HLSMAX),
//! where HLSMAX is currently 255.
//!
//! If (tint < 0)  Lum' = Lum * (1.0 + tint)
//! If (tint > 0)  Lum' = Lum * (1.0-tint) + (HLSMAX - HLSMAX * (1.0-tint))
//! ```
//!
//! Darkening 50% is *"Lum = 200; tint = -0.5 → Lum' = 100"*, and lightening 75% is *"Lum = 100;
//! tint = 0.75 → 217"*. [`apply_tint_to_luminance`] is those two lines, and its unit cases are the
//! spec's own four worked examples rather than numbers this crate chose.
//!
//! Both branches are linear in `Lum`, so `HLSMAX` cancels: scaling luminance to `0.0..=1.0` gives
//! the same colour and rounds once, at the end, instead of twice. The value of `HLSMAX` therefore
//! does not enter the implementation, which is why nothing below mentions 255.
//!
//! A tint applies to whichever of the four spellings supplied the base colour — an `rgb`, an
//! `indexed` and a `theme` can all carry one — and it is applied last.

use mjx_dml::{ResolvedColor, SchemeColors};
use mjx_ooxml_core::Interner;
use mjx_ooxml_types::drawingml::ColorSchemeSlot;

use crate::font::Color;

use super::colors::IndexedColors;

/// What one row of the indexed palette resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexedColor<'a> {
    /// An `ST_UnsignedIntHex` value — eight hex digits, alpha first — exactly as the palette states
    /// it. The defaults are Part 1 §18.8.27's, whose alpha is `00`.
    Rgb(&'a str),
    /// `indexed="64"` — *System Foreground*. Part 1 gives it no ARGB: it is whatever the consumer's
    /// system foreground colour is when the sheet is drawn.
    SystemForeground,
    /// `indexed="65"` — *System Background*, on the same footing. This is the index practically
    /// every workbook's second `<fill>` writes as its `bgColor`.
    SystemBackground,
}

/// The legacy indexed colour palette: the default sixty-four rows, or a workbook's replacement.
///
/// See the [module documentation](self) for the spec text this is taken from, and for why the table
/// has sixty-four rows plus two system indices rather than the fifty-six a BIFF palette has.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IndexedColorPalette {
    /// The workbook's own entries, or empty for the default palette. Borrowed rows are not an option
    /// here: the palette outlives the `indexedColors` element in every caller that has one.
    entries: Vec<String>,
}

impl IndexedColorPalette {
    /// The default palette, indices `0` through `63`, exactly as ECMA-376 Part 1 §18.8.27 prints it.
    ///
    /// Rows `0`..=`7` repeat rows `8`..=`15`, which is the spec's own note rather than an error
    /// here. Indices `64` and `65` are not in this table because they are not colours; see
    /// [`IndexedColor`].
    pub const DEFAULT: [&'static str; 64] = [
        "00000000", "00FFFFFF", "00FF0000", "0000FF00", "000000FF", "00FFFF00", "00FF00FF",
        "0000FFFF", "00000000", "00FFFFFF", "00FF0000", "0000FF00", "000000FF", "00FFFF00",
        "00FF00FF", "0000FFFF", "00800000", "00008000", "00000080", "00808000", "00800080",
        "00008080", "00C0C0C0", "00808080", "009999FF", "00993366", "00FFFFCC", "00CCFFFF",
        "00660066", "00FF8080", "000066CC", "00CCCCFF", "00000080", "00FF00FF", "00FFFF00",
        "0000FFFF", "00800080", "00800000", "00008080", "000000FF", "0000CCFF", "00CCFFFF",
        "00CCFFCC", "00FFFF99", "0099CCFF", "00FF99CC", "00CC99FF", "00FFCC99", "003366FF",
        "0033CCCC", "0099CC00", "00FFCC00", "00FF9900", "00FF6600", "00666699", "00969696",
        "00003366", "00339966", "00003300", "00333300", "00993300", "00993366", "00333399",
        "00333333",
    ];

    /// `indexed="64"` — the row Part 1 names *System Foreground*.
    pub const SYSTEM_FOREGROUND_INDEX: u32 = 64;

    /// `indexed="65"` — the row Part 1 names *System Background*.
    pub const SYSTEM_BACKGROUND_INDEX: u32 = 65;

    /// The default palette — what a workbook that writes no `indexedColors` means.
    #[must_use]
    pub fn default_palette() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// The palette a workbook's `indexedColors` block replaces the default with.
    ///
    /// Part 1 §18.8.27 requires such a block to be written **whole**, so this takes the entries as
    /// they stand and does not fill the tail from the default: a palette shorter than sixty-four
    /// rows is the file saying something unusual, and reporting `None` past its end is more honest
    /// than quietly answering with a row the file replaced.
    #[must_use]
    pub fn from_indexed_colors(colors: &IndexedColors, interner: &Interner) -> Self {
        let entries = colors
            .entries()
            .map(|entry| {
                entry
                    .rgb(interner)
                    .ok()
                    .flatten()
                    .map(std::borrow::Cow::into_owned)
                    .unwrap_or_default()
            })
            .collect();
        Self { entries }
    }

    /// Whether this is the default palette rather than a workbook's replacement.
    #[must_use]
    pub fn is_default(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many rows this palette declares — 64 for the default.
    #[must_use]
    pub fn len(&self) -> usize {
        if self.entries.is_empty() {
            Self::DEFAULT.len()
        } else {
            self.entries.len()
        }
    }

    /// Whether the palette has no rows. Never true for the default palette.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// What `indexed="index"` means.
    ///
    /// The palette's own rows are consulted first, so a workbook that writes sixty-six of them
    /// redefines the two system indices too; otherwise `64` and `65` are the system colours Part 1
    /// names, and anything beyond is `None` — Part 1: *"When values not present in the above list
    /// are used, the behavior is implementation-defined."*
    #[must_use]
    pub fn lookup(&self, index: u32) -> Option<IndexedColor<'_>> {
        let row = usize::try_from(index).ok()?;
        let rows: &[&str] = &Self::DEFAULT;
        let value = if self.entries.is_empty() {
            rows.get(row).copied()
        } else {
            self.entries.get(row).map(String::as_str)
        };
        match value {
            Some(value) => Some(IndexedColor::Rgb(value)),
            None if index == Self::SYSTEM_FOREGROUND_INDEX => Some(IndexedColor::SystemForeground),
            None if index == Self::SYSTEM_BACKGROUND_INDEX => Some(IndexedColor::SystemBackground),
            None => None,
        }
    }
}

/// The scheme slot a SpreadsheetML `@theme` position names.
///
/// The two dark/light pairs are **swapped** relative to the *Sequence Index* table ECMA-376 Part 1
/// §20.1.6.2 prints for `CT_ColorScheme`'s children: `0` is `lt1` and `1` is `dk1`, `2` is `lt2` and
/// `3` is `dk2`. Accents and hyperlinks are the same in both readings. See the
/// [module documentation](self) for the evidence — it is ECMA's own `presetCellStyles.xml` and
/// `presetTableStyles.xml` — and `crates/mjx-sml/tests/theme_index.rs` for the gate that re-derives
/// this from them rather than restating it.
///
/// `None` past `11`: the colour scheme is twelve slots and the spec defines no thirteenth.
#[must_use]
pub fn theme_color_slot(position: u32) -> Option<ColorSchemeSlot> {
    Some(match position {
        0 => ColorSchemeSlot::Light1,
        1 => ColorSchemeSlot::Dark1,
        2 => ColorSchemeSlot::Light2,
        3 => ColorSchemeSlot::Dark2,
        4 => ColorSchemeSlot::Accent1,
        5 => ColorSchemeSlot::Accent2,
        6 => ColorSchemeSlot::Accent3,
        7 => ColorSchemeSlot::Accent4,
        8 => ColorSchemeSlot::Accent5,
        9 => ColorSchemeSlot::Accent6,
        10 => ColorSchemeSlot::Hyperlink,
        11 => ColorSchemeSlot::FollowedHyperlink,
        _ => return None,
    })
}

/// The SpreadsheetML `@theme` position that names `slot` — [`theme_color_slot`]'s inverse.
///
/// It is total where its inverse is partial, because every one of the twelve slots has a position
/// and only a position can be out of range. It exists so an *authoring* caller never has to write
/// the number — and MJXOFF-246 is why that matters more than convenience: for as long as the writer
/// spelled font 0's colour as the literal `1`, nothing tied the number it wrote to the slot it meant,
/// and the two halves of this crate could and did drift apart. Every authoring path in this crate
/// now names a [`ColorSchemeSlot`] and lets this function supply the number.
#[must_use]
pub fn theme_color_position(slot: ColorSchemeSlot) -> u32 {
    match slot {
        ColorSchemeSlot::Light1 => 0,
        ColorSchemeSlot::Dark1 => 1,
        ColorSchemeSlot::Light2 => 2,
        ColorSchemeSlot::Dark2 => 3,
        ColorSchemeSlot::Accent1 => 4,
        ColorSchemeSlot::Accent2 => 5,
        ColorSchemeSlot::Accent3 => 6,
        ColorSchemeSlot::Accent4 => 7,
        ColorSchemeSlot::Accent5 => 8,
        ColorSchemeSlot::Accent6 => 9,
        ColorSchemeSlot::Hyperlink => 10,
        ColorSchemeSlot::FollowedHyperlink => 11,
    }
}

/// ECMA-376 Part 1 §18.8.19's tint, applied to one luminance in `0.0..=1.0`.
///
/// The prose states it over `0..HLSMAX`; both branches are linear in `Lum`, so the scale cancels and
/// this is the same function on the unit interval. `tint` outside `-1.0..=1.0` is clamped, because
/// the attribute's stated range is that and a `tint="4"` would otherwise send luminance past white.
#[must_use]
pub fn apply_tint_to_luminance(luminance: f64, tint: f64) -> f64 {
    let tint = tint.clamp(-1.0, 1.0);
    if tint < 0.0 {
        luminance * (1.0 + tint)
    } else if tint > 0.0 {
        luminance * (1.0 - tint) + tint
    } else {
        luminance
    }
}

/// `rgb`, with §18.8.19's tint applied through HLS.
#[must_use]
pub fn apply_tint(rgb: [u8; 3], tint: f64) -> [u8; 3] {
    if tint == 0.0 {
        return rgb;
    }
    let (hue, luminance, saturation) = to_hls(rgb);
    from_hls(hue, apply_tint_to_luminance(luminance, tint), saturation)
}

/// Resolves `color` to a concrete RGB, against the workbook's theme and its indexed palette.
///
/// `None` when the file does not say — an empty `<color/>`, an `auto="1"` (whose value is the
/// consumer's *system* colour, which this library does not invent), an index the palette does not
/// define or that names a system colour, a `theme` position past the scheme's twelve slots, a
/// `theme` slot the theme part left undefined, and an `@rgb` that is not hex.
///
/// `theme` is [`SchemeColors`] — `mjx-dml`'s interner-free resolved colour scheme — so a workbook's
/// colour and the theme part it refers to may live in different parts, and a theme colour resolves
/// to exactly what a DrawingML `a:schemeClr` naming the same slot resolves to. Building it is
/// `mjx_dml::SchemeColors::from_scheme`, and getting the theme part out of the package is
/// `mjx-xlsx`'s: this crate has never heard of one.
#[must_use]
pub fn resolve_color(
    color: &Color,
    theme: &SchemeColors,
    palette: &IndexedColorPalette,
) -> Option<ResolvedColor> {
    let (rgb, alpha) = base_rgba(color, theme, palette)?;
    let [red, green, blue] = apply_tint(rgb, color.tint.unwrap_or(0.0));
    Some(ResolvedColor {
        red,
        green,
        blue,
        alpha,
    })
}

/// The colour before the tint: whichever of the four spellings the file used.
///
/// The order is the schema's declaration order, which is also the order of decreasing indirection:
/// an explicit `rgb` beats a `theme` beats an `indexed`. A file writing more than one is malformed,
/// and answering deterministically is better than answering by whichever branch happens to run.
fn base_rgba(
    color: &Color,
    theme: &SchemeColors,
    palette: &IndexedColorPalette,
) -> Option<([u8; 3], f64)> {
    if let Some(rgb) = &color.rgb {
        return parse_argb(rgb);
    }
    if let Some(position) = color.theme {
        let slot = theme_color_slot(position)?;
        return Some((theme.rgb(slot)?, 1.0));
    }
    if let Some(index) = color.indexed {
        return match palette.lookup(index)? {
            IndexedColor::Rgb(value) => parse_argb(value),
            IndexedColor::SystemForeground | IndexedColor::SystemBackground => None,
        };
    }
    None
}

/// An `ST_UnsignedIntHex` value as `(rgb, alpha)`.
///
/// Eight digits are `AARRGGBB`, which is what the type declares. Six are accepted as an opaque
/// `RRGGBB` because producers write them and refusing would lose a colour the file plainly states;
/// anything else is not a colour this can read.
fn parse_argb(value: &str) -> Option<([u8; 3], f64)> {
    let value = value.trim();
    let (alpha, rgb) = match value.len() {
        8 => (
            f64::from(u8::from_str_radix(&value[0..2], 16).ok()?) / 255.0,
            &value[2..],
        ),
        6 => (1.0, value),
        _ => return None,
    };
    let red = u8::from_str_radix(&rgb[0..2], 16).ok()?;
    let green = u8::from_str_radix(&rgb[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&rgb[4..6], 16).ok()?;
    Some(([red, green, blue], alpha))
}

/// sRGB to hue, luminance and saturation, each in `0.0..=1.0`.
fn to_hls(rgb: [u8; 3]) -> (f64, f64, f64) {
    let [red, green, blue] = rgb.map(|channel| f64::from(channel) / 255.0);
    let max = red.max(green).max(blue);
    let min = red.min(green).min(blue);
    let luminance = (max + min) / 2.0;
    let delta = max - min;
    if delta == 0.0 {
        return (0.0, luminance, 0.0);
    }
    let saturation = if luminance > 0.5 {
        delta / (2.0 - max - min)
    } else {
        delta / (max + min)
    };
    let hue = if max == red {
        ((green - blue) / delta).rem_euclid(6.0)
    } else if max == green {
        (blue - red) / delta + 2.0
    } else {
        (red - green) / delta + 4.0
    } / 6.0;
    (hue, luminance, saturation)
}

/// Hue, luminance and saturation back to sRGB, rounding once.
fn from_hls(hue: f64, luminance: f64, saturation: f64) -> [u8; 3] {
    let luminance = luminance.clamp(0.0, 1.0);
    if saturation == 0.0 {
        let channel = to_byte(luminance);
        return [channel, channel, channel];
    }
    let q = if luminance < 0.5 {
        luminance * (1.0 + saturation)
    } else {
        luminance + saturation - luminance * saturation
    };
    let p = 2.0 * luminance - q;
    [
        to_byte(hue_to_channel(p, q, hue + 1.0 / 3.0)),
        to_byte(hue_to_channel(p, q, hue)),
        to_byte(hue_to_channel(p, q, hue - 1.0 / 3.0)),
    ]
}

/// One channel of the HSL-to-RGB conversion.
fn hue_to_channel(p: f64, q: f64, hue: f64) -> f64 {
    let hue = hue.rem_euclid(1.0);
    if hue < 1.0 / 6.0 {
        p + (q - p) * 6.0 * hue
    } else if hue < 0.5 {
        q
    } else if hue < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - hue) * 6.0
    } else {
        p
    }
}

/// A `0.0..=1.0` channel as a byte, rounded half away from zero.
fn to_byte(channel: f64) -> u8 {
    (channel.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ECMA-376 Part 1 §18.8.19's own four worked examples, in the spec's `0..255` scale.
    ///
    /// The prose states the algorithm over `0..HLSMAX`; this crate applies it on the unit interval
    /// because both branches are linear, and these cases are what says the two agree.
    ///
    /// **The third row is where the spec's own arithmetic rounds.** It prints
    /// *"Lum' = 100 * .25 + (255 – 255 * .25) = 25 + (255 – 63) = 217"* — but `255 * .25` is
    /// `63.75`, not `63`, so the exact result is `216.25` and the printed `217` comes of truncating
    /// an intermediate in integer HLS arithmetic. Both numbers are asserted: this crate is exact,
    /// and it stays within one luminance step of every figure the specification prints.
    #[test]
    fn the_tint_matches_the_specifications_worked_examples() {
        for (luminance, tint, printed, exact) in [
            (200.0, -0.5, 100.0, 100.0),
            (200.0, -1.0, 0.0, 0.0),
            (100.0, 0.75, 217.0, 216.25),
            (100.0, 1.0, 255.0, 255.0),
        ] {
            let got: f64 = apply_tint_to_luminance(luminance / 255.0, tint) * 255.0;
            assert!(
                (got - exact).abs() < 1e-9,
                "Lum = {luminance}, tint = {tint}: exactly {exact}, got {got}"
            );
            assert!(
                (printed - exact).abs() <= 1.0,
                "Part 1 §18.8.19 prints {printed} where the exact value is {exact}; a gap of more \
                 than one luminance step would mean this is not the spec's algorithm at all"
            );
        }
    }

    #[test]
    fn a_tint_of_zero_and_an_absent_tint_change_nothing() {
        assert_eq!(apply_tint([0x44, 0x72, 0xC4], 0.0), [0x44, 0x72, 0xC4]);
    }

    #[test]
    fn a_tint_darkens_towards_black_and_lightens_towards_white() {
        assert_eq!(apply_tint([0x44, 0x72, 0xC4], -1.0), [0, 0, 0]);
        assert_eq!(apply_tint([0x44, 0x72, 0xC4], 1.0), [255, 255, 255]);
        let darkened = apply_tint([0x44, 0x72, 0xC4], -0.5);
        let brightness = |rgb: [u8; 3]| rgb.iter().map(|c| u32::from(*c)).sum::<u32>();
        assert!(
            brightness(darkened) < brightness([0x44, 0x72, 0xC4]),
            "a negative tint must darken: {darkened:?}"
        );
    }

    #[test]
    fn a_round_trip_through_hls_returns_the_colour() {
        for rgb in [
            [0x00, 0x00, 0x00],
            [0xFF, 0xFF, 0xFF],
            [0x44, 0x72, 0xC4],
            [0xED, 0x7D, 0x31],
            [0x18, 0xA3, 0x03],
            [0x80, 0x80, 0x80],
        ] {
            let (hue, luminance, saturation) = to_hls(rgb);
            assert_eq!(from_hls(hue, luminance, saturation), rgb, "{rgb:?}");
        }
    }

    #[test]
    fn the_default_palette_is_the_specifications_table() {
        let palette = IndexedColorPalette::default_palette();
        assert!(palette.is_default());
        assert_eq!(palette.len(), 64);
        // §18.8.27: "0-7 are redundant of 8-15".
        for index in 0..8 {
            assert_eq!(
                palette.lookup(index),
                palette.lookup(index + 8),
                "index {index} must repeat index {}",
                index + 8
            );
        }
        assert_eq!(palette.lookup(8), Some(IndexedColor::Rgb("00000000")));
        assert_eq!(palette.lookup(22), Some(IndexedColor::Rgb("00C0C0C0")));
        assert_eq!(palette.lookup(63), Some(IndexedColor::Rgb("00333333")));
        assert_eq!(palette.lookup(64), Some(IndexedColor::SystemForeground));
        assert_eq!(palette.lookup(65), Some(IndexedColor::SystemBackground));
        assert_eq!(palette.lookup(66), None);
    }

    /// The table itself, stated once.
    ///
    /// This restates the mapping and is therefore *not* what establishes it — that is
    /// `crates/mjx-sml/tests/theme_index.rs`, which re-derives the same table from ECMA's own preset
    /// styles and would redden if this one were edited to agree with §20.1.6.2's sequence. What this
    /// is for is the two ends and the four positions the two readings differ about, in one place a
    /// reader can see them.
    #[test]
    fn the_two_dark_light_pairs_are_swapped_against_the_sequence_table() {
        // The four the two readings disagree about. See the module documentation.
        assert_eq!(theme_color_slot(0), Some(ColorSchemeSlot::Light1));
        assert_eq!(theme_color_slot(1), Some(ColorSchemeSlot::Dark1));
        assert_eq!(theme_color_slot(2), Some(ColorSchemeSlot::Light2));
        assert_eq!(theme_color_slot(3), Some(ColorSchemeSlot::Dark2));
        // And the eight they agree about, which a swap of the wrong pair would disturb.
        assert_eq!(theme_color_slot(4), Some(ColorSchemeSlot::Accent1));
        assert_eq!(theme_color_slot(9), Some(ColorSchemeSlot::Accent6));
        assert_eq!(theme_color_slot(10), Some(ColorSchemeSlot::Hyperlink));
        assert_eq!(
            theme_color_slot(11),
            Some(ColorSchemeSlot::FollowedHyperlink)
        );
        assert_eq!(theme_color_slot(12), None);
    }

    /// The authoring direction is the reading direction turned round, and neither is written twice.
    ///
    /// Both halves are asserted: every position the reader defines comes back from the writer, and
    /// the writer never produces a position the reader refuses. A second table that had drifted by
    /// one slot would pass the first half on eleven of twelve and fail here.
    #[test]
    fn a_theme_slot_and_its_position_are_inverses_across_all_twelve() {
        for position in 0..12 {
            let slot = theme_color_slot(position).expect("twelve slots are defined");
            assert_eq!(
                theme_color_position(slot),
                position,
                "position {position} resolves to {slot:?}, which names a different position"
            );
        }
        for slot in [
            ColorSchemeSlot::Dark1,
            ColorSchemeSlot::Light1,
            ColorSchemeSlot::Dark2,
            ColorSchemeSlot::Light2,
            ColorSchemeSlot::Accent1,
            ColorSchemeSlot::Accent2,
            ColorSchemeSlot::Accent3,
            ColorSchemeSlot::Accent4,
            ColorSchemeSlot::Accent5,
            ColorSchemeSlot::Accent6,
            ColorSchemeSlot::Hyperlink,
            ColorSchemeSlot::FollowedHyperlink,
        ] {
            assert_eq!(
                theme_color_slot(theme_color_position(slot)),
                Some(slot),
                "{slot:?} names a position that reads back as something else"
            );
        }
    }

    #[test]
    fn an_eight_digit_value_carries_its_alpha_and_a_six_digit_one_is_opaque() {
        assert_eq!(parse_argb("FF18A303"), Some(([0x18, 0xA3, 0x03], 1.0)));
        assert_eq!(parse_argb("0018A303"), Some(([0x18, 0xA3, 0x03], 0.0)));
        assert_eq!(parse_argb("18A303"), Some(([0x18, 0xA3, 0x03], 1.0)));
        assert_eq!(parse_argb("nonsense"), None);
        assert_eq!(parse_argb("FFF"), None);
    }

    #[test]
    fn a_colour_that_says_nothing_resolves_to_nothing() {
        let theme = SchemeColors::default();
        let palette = IndexedColorPalette::default_palette();
        assert_eq!(resolve_color(&Color::default(), &theme, &palette), None);
        let automatic = Color {
            automatic: Some(true),
            ..Color::default()
        };
        assert_eq!(
            resolve_color(&automatic, &theme, &palette),
            None,
            "`auto` is the consumer's system colour, which this library does not invent"
        );
        assert_eq!(
            resolve_color(&Color::from_theme(4, None), &theme, &palette),
            None,
            "an empty scheme defines no slot"
        );
    }

    #[test]
    fn an_indexed_colour_resolves_through_the_palette_and_a_system_index_does_not() {
        let theme = SchemeColors::default();
        let palette = IndexedColorPalette::default_palette();
        let color = Color {
            indexed: Some(10),
            ..Color::default()
        };
        let resolved = resolve_color(&color, &theme, &palette).expect("index 10 is red");
        assert_eq!(resolved.to_hex(), "FF0000");
        assert_eq!(
            resolved.alpha, 0.0,
            "the spec's table writes the palette with an alpha of 00, and this reports what it says"
        );

        let system = Color {
            indexed: Some(64),
            ..Color::default()
        };
        assert_eq!(resolve_color(&system, &theme, &palette), None);
    }
}
