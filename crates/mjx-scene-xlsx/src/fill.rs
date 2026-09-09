//! Fills: `mjx-layout-xlsx`'s [`CellFill`] as `mjx-scene`'s [`FillStyle`].
//!
//! # A translation, not a resolution
//!
//! Every value arriving here has already been through `mjx-xlsx`'s `xf` ladder — the
//! `cellXfs`/`cellStyleXfs` layering, the `applyX` gating and the cell → row → column → default walk
//! have all run, and `mjx-layout-xlsx` consumed the answer. So nothing here reads a document and
//! nothing decides what a cell *says*; it decides only how to write the same thing in the other
//! vocabulary.
//!
//! # ⚠ `patternType="solid"` paints the **foreground** colour
//!
//! This is the single most commonly got-wrong rule in SpreadsheetML, and it is the one every
//! spreadsheet in the world depends on: a solid fill is written
//! `<patternFill patternType="solid"><fgColor rgb="FFFFFF00"/><bgColor indexed="64"/></patternFill>`,
//! and the colour a reader sees is the **`fgColor`**. Reading `bgColor` there paints every
//! highlighted cell in the system window colour, which looks exactly like no fill at all.
//! `tests/a_real_sheet_resolves.rs` asserts the rule against a fixture rather than restating it.
//!
//! # ⚠ Every hatch mapping is a GUESS
//!
//! SpreadsheetML declares nineteen `ST_PatternValues` — `none`, `solid` and seventeen hatches;
//! `mjx-scene`'s [`PatternPreset`] is DrawingML's fifty-four, and the two vocabularies are **not**
//! the same list. Eight of the seventeen hatches have an exact twin (the four `dark`/`light`
//! horizontals and verticals, and the four diagonals DrawingML spells out in full); the nine grey,
//! grid and trellis values do not, and each is matched to the nearest DrawingML preset with its
//! reasoning at the site. `darkTrellis` and `lightTrellis` both reach the one `Trellis` DrawingML
//! defines, which is a real loss and is stated rather than hidden.

use mjx_layout_xlsx::{CellFill, CellGradient};
use mjx_ooxml_types::spreadsheetml::{GradientType, PatternType};
use mjx_scene::{FillStyle, Gradient, GradientStop, PatternPreset, SceneRect};

use crate::colour::{SheetPalette, SystemRole};

/// What paints a cell whose catalogue entry states `fill`, or [`FillStyle::None`] when nothing
/// does.
#[must_use]
pub fn fill_style(fill: &CellFill, palette: &SheetPalette) -> FillStyle {
    if let Some(gradient) = &fill.gradient {
        return gradient_style(gradient, palette);
    }
    match fill.pattern {
        // `<patternFill patternType="none"/>`, which is the first `<fill>` of every workbook and
        // what an unformatted cell resolves to. Nothing is painted, which is not the same as
        // painting white: a cell with no fill lets the sheet's own background through.
        Some(PatternType::None) => FillStyle::None,
        // The foreground, and see this module's own warning about why.
        Some(PatternType::Solid) => fill
            .foreground
            .as_ref()
            .and_then(|colour| palette.resolve(colour, SystemRole::Foreground))
            .map_or(FillStyle::None, FillStyle::Solid),
        Some(pattern) => {
            let Some(preset) = pattern_preset(pattern) else {
                // Unreachable for the two handled above; written rather than assumed so that adding
                // a nineteenth `ST_PatternValues` cannot silently paint nothing.
                return FillStyle::None;
            };
            FillStyle::Pattern {
                preset,
                // A hatch whose colours the file leaves out is drawn in the window's own colours,
                // which is what makes `<patternFill patternType="gray125"/>` — the second `<fill>`
                // of every workbook Excel writes — a grey stipple rather than nothing.
                foreground: palette
                    .resolve_or_system(fill.foreground.as_ref(), SystemRole::Foreground),
                background: palette
                    .resolve_or_system(fill.background.as_ref(), SystemRole::Background),
            }
        }
        // A `<patternFill>` that states a colour and **no** `@patternType` at all. R16 calls this
        // "a third state beside `none` and `solid`, and is what a `dxf` writes", and it is: a
        // conditional format's fill is written `<patternFill><bgColor rgb="FFFFC7CE"/></patternFill>`
        // and Excel paints that cell solid pink.
        //
        // GUESS: it is drawn as a solid fill of `bgColor`, falling back to `fgColor`. That is the
        // opposite colour from the `solid` case above, and it is not a mistake — it is what makes
        // MJXOFF-173's conditional formatting land the colour a reader expects. Which of the two
        // Excel actually reads when a `dxf` writes both is a question for the Windows sitting.
        None => fill
            .background
            .as_ref()
            .or(fill.foreground.as_ref())
            .and_then(|colour| palette.resolve(colour, SystemRole::Background))
            .map_or(FillStyle::None, FillStyle::Solid),
    }
}

/// What paints a gradient-filled cell.
///
/// GUESS: `@degree` is degrees clockwise from the positive `x` axis, which is the sense
/// [`Gradient::angle`] is in, so the conversion is a change of unit and not of convention. ECMA-376
/// §18.8.24 gives the attribute no worked example, and whether Excel measures it the other way is a
/// question for the Windows sitting.
fn gradient_style(gradient: &CellGradient, palette: &SheetPalette) -> FillStyle {
    let stops: Vec<GradientStop> = gradient
        .stops
        .iter()
        .filter_map(|stop| {
            let colour = stop
                .colour
                .as_ref()
                .and_then(|colour| palette.resolve(colour, SystemRole::Foreground))?;
            Some(GradientStop::new(stop.position as f32, colour))
        })
        .collect();
    if stops.is_empty() {
        // A gradient with no readable stop is not a gradient. Painting nothing is visible and
        // reportable; painting one of the two system colours would look like a decision.
        return FillStyle::None;
    }
    let mut ramp = Gradient::linear(stops, (gradient.degrees as f32).to_radians());
    if gradient.kind == GradientType::Path {
        ramp.kind = mjx_scene::GradientKind::Path;
        let [left, right, top, bottom] = gradient.inset;
        // `@left`/`@right`/`@top`/`@bottom` are the *inner* rectangle the ramp converges on, as
        // fractions of the cell — which is exactly what `Gradient::focus` is.
        ramp.focus = SceneRect::new(left as f32, top as f32, right as f32, bottom as f32);
    }
    FillStyle::Gradient(ramp)
}

/// The display list's name for a SpreadsheetML hatch, or `None` for the two values that are not
/// hatches at all.
///
/// # Why nineteen arms and not a cast
///
/// The two enumerations are generated from **different** simple types — `ST_PatternValues` has
/// nineteen values and `ST_PresetPatternVal` fifty-four — so there is no arithmetic between them to
/// get wrong, and this is a genuine table rather than a cast in disguise. Written out, the match is
/// exhaustive on the input side: a twentieth pattern fails this file to compile, which is the
/// strongest gate available and costs nothing to keep.
#[must_use]
pub fn pattern_preset(pattern: PatternType) -> Option<PatternPreset> {
    Some(match pattern {
        // Not hatches: `fill_style` answers both before reaching here.
        PatternType::None | PatternType::Solid => return None,
        // The five greys. GUESS, each of them: SpreadsheetML names a *density* and DrawingML names
        // a percentage, and these are the nearest percentages DrawingML offers. `gray125` is
        // 12.5 %, which falls between `pct10` and `pct20`; the lighter of the two is chosen because
        // Excel's own `gray125` is the faintest stipple in the gallery and rounding it up makes the
        // second `<fill>` of every workbook visibly darker than it should be.
        PatternType::MediumGray => PatternPreset::Percent50,
        PatternType::DarkGray => PatternPreset::Percent75,
        PatternType::LightGray => PatternPreset::Percent25,
        PatternType::Gray12Point5Percent => PatternPreset::Percent10,
        PatternType::Gray6Point25Percent => PatternPreset::Percent5,
        // The eight directional hatches, each with an exact twin by name.
        PatternType::DarkHorizontal => PatternPreset::DarkHorizontal,
        PatternType::DarkVertical => PatternPreset::DarkVertical,
        PatternType::DarkDown => PatternPreset::DarkDownwardDiagonal,
        PatternType::DarkUp => PatternPreset::DarkUpwardDiagonal,
        PatternType::LightHorizontal => PatternPreset::LightHorizontal,
        PatternType::LightVertical => PatternPreset::LightVertical,
        PatternType::LightDown => PatternPreset::LightDownwardDiagonal,
        PatternType::LightUp => PatternPreset::LightUpwardDiagonal,
        // GUESS: DrawingML's two grids are `smGrid` (a fine mesh) and `lgGrid` (a coarse one), and
        // SpreadsheetML's two are `darkGrid` (dense) and `lightGrid` (sparse). Density is the axis
        // both vocabularies vary along, so they are matched on it.
        PatternType::DarkGrid => PatternPreset::SmallGrid,
        PatternType::LightGrid => PatternPreset::LargeGrid,
        // GUESS, and a stated **loss**: DrawingML defines one `trellis` and SpreadsheetML two, so
        // `darkTrellis` and `lightTrellis` are drawn identically. Telling them apart needs a hatch
        // mask `mjx-paint`'s fifty-four-entry `PATTERN_MASKS` does not have, which is a change to
        // the single source of truth every painter reads and not something to invent here.
        PatternType::DarkTrellis | PatternType::LightTrellis => PatternPreset::Trellis,
    })
}
