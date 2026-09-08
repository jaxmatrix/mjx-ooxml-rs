//! Composing a cell's lines — the part of the layout that talks to the font engine.
//!
//! Nothing here measures anything. Every width, ascent and descent comes from a
//! [`ComposedLine`] the shaper produced, and every face comes from
//! `mjx-text`'s resolver through [`itemise`], which does the bidirectional resolution, the script
//! itemisation and the per-character face fallback in one pass.
//!
//! # A cell is one run, and that is the document model rather than a simplification
//!
//! A DrawingML text body is paragraphs of runs and a Word paragraph is runs of characters, so both
//! box models above this one carry a run vocabulary. A worksheet cell is **one string in one
//! format** — `EffectiveCellFormat` resolves to exactly one `x:font` — so a cell's whole text is one
//! [`CellRunStyle`] and there is no per-run ladder to walk.
//!
//! The one exception is a shared string with **rich-text runs** (`CT_Rst`'s `<r>` children), where
//! the cell's text really is several formats. R16 lays such a string out in the cell's own format and
//! marks it: honouring the runs needs the per-run `rPr` projected into a run style, which is a
//! straightforward extension of this module and not a redesign.

use std::ops::Range;
use std::sync::Arc;

use mjx_layout::{ComposedLine, LineComposer, TextRun};
use mjx_ooxml_core::measure::Emu;
use mjx_sml::FontProperties;
use mjx_text::{
    itemise, BidiAnalysis, FaceId, FeatureSet, FontError, FontFace, FontRequest, FontResolver,
    FontSize, FontSlant, FontWeight, GlyphRasteriser, LineBreakOptions, ParagraphDirection, Shaper,
    TextItem,
};

/// The family a cell asks for when no `x:font` names one.
///
/// `mjx-text`'s resolver treats an unknown family as a substitution and answers from the bundled
/// tier, so this is a *name to record in the manifest* rather than a face this crate picks. Naming a
/// concrete face here would override the branding of whoever opens the workbook, which is the one
/// thing a renderer must never do.
pub const UNNAMED_FAMILY: &str = "";

/// The size a cell renders at when no `x:font` states one.
///
/// `sml.xsd` gives `CT_FontSize@val` no default, and Excel's own Normal style is 11 pt. This is **a
/// guess about Excel** rather than a value from the specification, and it is only reached by a
/// workbook whose `cellXfs[0]` resolves to no font at all.
pub const ASSUMED_FONT_SIZE_POINTS: f64 = 11.0;

/// Everything the font engine needs, borrowed together so one call can shape.
///
/// A struct of four `&mut` rather than four arguments because the box model owns all four and Rust
/// will not let `self.shaper` and `self.fonts` both be borrowed through `self`.
#[derive(Debug)]
pub struct TextEngine<'a> {
    /// The three resolution tiers, the substitution table and the manifest.
    pub fonts: &'a mut FontResolver,
    /// Where a [`FaceId`] comes from. See [`crate::SheetBoxModel::rasteriser_mut`] for why a box
    /// model holds one.
    pub rasteriser: &'a mut GlyphRasteriser,
    /// The shaper, which owns the shaped-run cache.
    pub shaper: &'a mut Shaper,
    /// The OpenType features every cell in this workbook is shaped with.
    pub features: &'a FeatureSet,
}

/// A cell's text as the font engine wants it: a family, a size and two style axes.
#[derive(Clone, PartialEq, Debug)]
pub struct CellRunStyle {
    /// The family the workbook asked for. Empty when no `x:font` named one.
    pub family: String,
    /// The size it renders at.
    pub size: FontSize,
    /// Its weight.
    pub weight: FontWeight,
    /// Its slant.
    pub slant: FontSlant,
}

impl CellRunStyle {
    /// The style a resolved `x:font`'s properties describe.
    ///
    /// **Every value here was resolved by `mjx-sml`**: the `xf` indirection, the cell → row →
    /// column → default walk and the `cellXfs`/`cellStyleXfs` layering have all already run, so this
    /// is a projection into the font engine's vocabulary and not a resolution of its own.
    #[must_use]
    pub fn from_font(font: Option<&FontProperties>) -> Self {
        let Some(font) = font else {
            return Self::default();
        };
        Self {
            family: font
                .font_name
                .clone()
                .unwrap_or_else(|| UNNAMED_FAMILY.to_owned()),
            size: font
                .size_in_points
                .filter(|points| points.is_finite() && *points > 0.0)
                .map_or_else(default_size, FontSize::from_points),
            weight: if font.bold.unwrap_or(false) {
                FontWeight::BOLD
            } else {
                FontWeight::REGULAR
            },
            slant: if font.italic.unwrap_or(false) {
                FontSlant::Italic
            } else {
                FontSlant::Upright
            },
        }
    }

    /// The same style with its size multiplied by `scale` — what shrink-to-fit applies.
    #[must_use]
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            size: FontSize::from_points(self.size.in_points() * scale),
            ..self.clone()
        }
    }

    /// The font request this style makes.
    #[must_use]
    pub fn request(&self) -> FontRequest<'_> {
        FontRequest::new(&self.family)
            .with_weight(self.weight)
            .with_slant(self.slant)
    }
}

impl Default for CellRunStyle {
    fn default() -> Self {
        Self {
            family: UNNAMED_FAMILY.to_owned(),
            size: default_size(),
            weight: FontWeight::REGULAR,
            slant: FontSlant::Upright,
        }
    }
}

fn default_size() -> FontSize {
    FontSize::from_points(ASSUMED_FONT_SIZE_POINTS)
}

/// Cuts a cell's text into items: one pass of `mjx-text`'s bidirectional resolution, script
/// itemisation and face fallback.
///
/// # Errors
/// [`FontError`] when an indexed face will not parse.
pub fn itemise_cell(
    engine: &mut TextEngine<'_>,
    text: &str,
    style: &CellRunStyle,
    bidi: &BidiAnalysis,
) -> Result<Vec<TextItem>, FontError> {
    itemise(text, 0..text.len(), bidi, engine.fonts, &style.request())
}

/// The [`TextRun`]s a [`LineComposer`] takes, borrowing the faces the items own.
///
/// Returns the runs that could be registered; an item whose face will not register is dropped rather
/// than failing the cell, because a face that will not register is a face that will not draw and a
/// sheet missing one word is better than a workbook that will not open.
#[must_use]
pub fn composer_runs<'a>(
    rasteriser: &mut GlyphRasteriser,
    features: &'a FeatureSet,
    items: &'a [TextItem],
    size: FontSize,
) -> Vec<TextRun<'a>> {
    let mut runs = Vec::with_capacity(items.len());
    for item in items {
        let Some(face) = item.face() else { continue };
        let Ok(face_id) = rasteriser.register(face) else {
            continue;
        };
        runs.push(TextRun {
            range: item.range.clone(),
            face,
            face_id,
            script: item.script,
            direction: item.direction,
            level: item.level,
            size,
            features,
            language: None,
        });
    }
    runs
}

/// Which item a composed segment came from, by its byte range.
#[must_use]
pub fn item_of(items: &[TextItem], segment: &Range<usize>) -> Option<usize> {
    items
        .iter()
        .position(|item| segment.start >= item.range.start && segment.start < item.range.end)
}

/// Composes every line of a cell at a fixed `measure` in points.
///
/// Unlike a paragraph's, a cell's measure does not change between lines: there is no first-line
/// indent in SpreadsheetML, so one number serves.
///
/// # Errors
/// [`FontError`] when a face will not shape.
pub fn compose_cell(
    shaper: &mut Shaper,
    text: &str,
    runs: &[TextRun<'_>],
    bidi: &BidiAnalysis,
    options: LineBreakOptions,
    measure: f64,
) -> Result<Vec<ComposedLine>, FontError> {
    let composer = LineComposer::new(text, runs, bidi, options);
    let mut lines = Vec::new();
    let mut offset = 0_usize;
    while offset < text.len() {
        let line = composer.next_line(shaper, offset, measure)?;
        let end = line.range.end;
        lines.push(line);
        if end <= offset {
            // The composer could not advance — a measure narrower than one glyph. Taking the rest of
            // the text as one line is what stops the loop, and it is what a reader sees in Excel too:
            // a wrapped cell too narrow for one character still shows the character.
            if let Some(last) = lines.last_mut() {
                last.range = offset..text.len();
            }
            break;
        }
        offset = end;
    }
    Ok(lines)
}

/// The height of one line of `face` at `size`, for a cell with no text to measure.
#[must_use]
pub fn empty_line_height(face: &Arc<FontFace>, size: FontSize) -> Emu {
    Emu::from_points(face.metrics().line_height_at_size(size.in_points()))
}

/// The ascent of `face` at `size`, for the same reason.
#[must_use]
pub fn empty_line_ascent(face: &Arc<FontFace>, size: FontSize) -> Emu {
    let metrics = face.metrics();
    let units = f64::from(metrics.units_per_em.max(1));
    Emu::from_points(f64::from(metrics.ascender) * size.in_points() / units)
}

/// Resolves one face for a cell style, for the places that need a face without shaping — an empty
/// cell's row height, and the maximum digit width every column is quoted in.
///
/// # Errors
/// [`FontError`] when an indexed face will not parse.
pub fn resolve_face(
    fonts: &mut FontResolver,
    rasteriser: &mut GlyphRasteriser,
    style: &CellRunStyle,
) -> Result<Option<(Arc<FontFace>, FaceId)>, FontError> {
    let resolution = fonts.resolve(&style.request())?;
    let Some(face) = resolution.face() else {
        return Ok(None);
    };
    let face_id = rasteriser.register(face)?;
    Ok(Some((Arc::clone(face), face_id)))
}

/// The paragraph direction a cell's `@readingOrder` means.
///
/// `CT_CellAlignment@readingOrder` is `0` (context), `1` (left to right) or `2` (right to left),
/// which is exactly the three states [`ParagraphDirection`] carries. Anything else is read as
/// context, because a document may say anything.
#[must_use]
pub fn reading_order(order: Option<u32>) -> ParagraphDirection {
    match order {
        Some(1) => ParagraphDirection::LeftToRight,
        Some(2) => ParagraphDirection::RightToLeft,
        _ => ParagraphDirection::FromFirstStrongCharacter,
    }
}
