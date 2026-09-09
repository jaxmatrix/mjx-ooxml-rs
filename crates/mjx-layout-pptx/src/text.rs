//! Composing a text body's lines — the part of the layout that talks to the font engine.
//!
//! Nothing here measures anything. Every width, ascent and descent comes from a
//! [`ComposedLine`] the shaper produced, and every face comes from
//! `mjx-text`'s resolver through [`itemise`], which does the bidirectional resolution, the script
//! itemisation and the per-character face fallback in one pass. A box model that chose faces itself
//! would be a second font engine.
//!
//! # A tab is its own item, and why
//!
//! DrawingML puts tab characters inside the run text, and a tab's *width* is not a property of the
//! glyph — it is the distance to the next tab stop, which depends on where the pen already is. So a
//! tab may not sit inside a shaped item whose advances are summed: it has to be a boundary.
//!
//! The runs handed to [`LineComposer`] are therefore cut at every `\t`, which makes each tab its own
//! [`ComposedSegment`](mjx_layout::ComposedSegment) — and a segment that is exactly a tab is placed
//! by [`TabStops::next_after`] rather than by its advance, and emits no glyph run.
//!
//! **What this deliberately does not do** is feed the tab's resolved width back into line breaking:
//! the fitting loop measures a tab by its glyph advance, so a line that would break differently
//! because a tab pushed it past the measure breaks in the wrong place. That is a known
//! simplification, it is confined to paragraphs that contain tabs, and it is one of the things the
//! Windows sitting should look at.

use std::ops::Range;
use std::sync::Arc;

use mjx_dml::{CharacterPropertiesSpec, FontSlot, TabAlignment, TabStop};
use mjx_layout::{ComposedLine, LineComposer, TextRun};
use mjx_ooxml_core::measure::Emu;
use mjx_text::{
    itemise, BidiAnalysis, FaceId, FeatureSet, FontError, FontFace, FontRequest, FontResolver,
    FontSize, FontSlant, FontWeight, GlyphRasteriser, LineBreakOptions, ParagraphDirection, Shaper,
    TextItem,
};

/// The family a run asks for when nothing anywhere names one.
///
/// `mjx-text`'s resolver treats an unknown family as a substitution and answers from the bundled
/// tier, so this is a *name to record in the manifest* rather than a face this crate picks. Naming
/// a concrete face here instead would override the branding of whoever opens the file, which is the
/// one thing a renderer must never do.
pub const UNNAMED_FAMILY: &str = "";

/// The font size a run renders at when no tier of the ladder states one.
///
/// ECMA-376 gives `a:rPr@sz` no default, and PowerPoint's own new-text-box default is 18 pt. This is
/// **a guess about PowerPoint** rather than a value from the specification, and it is only ever
/// reached by a run that inherits nothing at all — which a deck PowerPoint authored never has,
/// because its master's `p:txStyles` always states a size.
pub const ASSUMED_FONT_SIZE_POINTS: f64 = 18.0;

/// Everything the font engine needs, borrowed together so one call can shape.
///
/// A struct of four `&mut` rather than four arguments because the box model owns all four and Rust
/// will not let `self.shaper` and `self.fonts` both be borrowed through `self`. Destructuring once
/// at the top of a layout pass is the whole of the trick.
#[derive(Debug)]
pub struct TextEngine<'a> {
    /// The three resolution tiers, the substitution table and the manifest.
    pub fonts: &'a mut FontResolver,
    /// Where a [`FaceId`] comes from. See [`crate::SlideBoxModel::rasteriser_mut`] for why a box
    /// model holds one.
    pub rasteriser: &'a mut GlyphRasteriser,
    /// The shaper, which caches nothing across calls but owns the shaping context.
    pub shaper: &'a mut Shaper,
    /// The OpenType features every run in this document is shaped with.
    pub features: &'a FeatureSet,
}

/// A run of a paragraph as the font engine wants it: a range, a family, a size and two style axes.
#[derive(Clone, PartialEq, Debug)]
pub struct RunStyle {
    /// The bytes of the paragraph's text it covers.
    pub range: Range<usize>,
    /// The family the document asked for. Empty when no tier named one.
    pub family: String,
    /// The size it renders at.
    pub size: FontSize,
    /// Its weight.
    pub weight: FontWeight,
    /// Its slant.
    pub slant: FontSlant,
    /// The language tag it declares, for the shaper's language-sensitive features.
    pub language: Option<String>,
}

impl RunStyle {
    /// The style of a run whose effective character properties are `properties`.
    ///
    /// **Every value here was resolved by `mjx-pptx`.** The seven-tier ladder — including the theme
    /// font substitution that turns `+mn-lt` into a family name — has already run, so this is a
    /// projection of a resolved value into the font engine's vocabulary and not a resolution of its
    /// own.
    #[must_use]
    pub fn from_properties(range: Range<usize>, properties: &CharacterPropertiesSpec) -> Self {
        let family = properties
            .font(FontSlot::Latin)
            .map(|font| font.typeface.clone())
            .unwrap_or_else(|| UNNAMED_FAMILY.to_owned());
        Self {
            range,
            family,
            size: properties
                .size_points()
                .map_or_else(default_size, FontSize::from_points),
            weight: if properties.is_bold().unwrap_or(false) {
                FontWeight::BOLD
            } else {
                FontWeight::REGULAR
            },
            slant: if properties.is_italic().unwrap_or(false) {
                FontSlant::Italic
            } else {
                FontSlant::Upright
            },
            language: properties.language().map(str::to_owned),
        }
    }

    /// The same style with every size multiplied by `scale` — what an autofit pass applies.
    #[must_use]
    pub fn scaled(&self, scale: f64) -> Self {
        Self {
            size: FontSize::from_points(self.size.in_points() * scale),
            ..self.clone()
        }
    }
}

fn default_size() -> FontSize {
    FontSize::from_points(ASSUMED_FONT_SIZE_POINTS)
}

/// One item of a paragraph, shaped-ready: what [`itemise`] produced plus the size and language the
/// document's run stated.
#[derive(Debug)]
pub struct StyledItem {
    /// The item the font engine cut.
    pub item: TextItem,
    /// Which of the paragraph's runs it came from.
    pub run: usize,
    /// The size that run renders at.
    pub size: FontSize,
    /// The language that run declares.
    pub language: Option<String>,
    /// Whether the item is exactly one tab character, which is placed rather than drawn.
    pub is_tab: bool,
}

/// Cuts a paragraph into items: one pass of `mjx-text`'s bidirectional resolution, script
/// itemisation and face fallback per document run, with tabs forced to be boundaries.
///
/// # Errors
/// [`FontError`] when an indexed face will not parse.
pub fn itemise_paragraph(
    engine: &mut TextEngine<'_>,
    text: &str,
    runs: &[RunStyle],
    bidi: &BidiAnalysis,
) -> Result<Vec<StyledItem>, FontError> {
    let mut styled = Vec::new();
    for (index, run) in runs.iter().enumerate() {
        let request = FontRequest::new(&run.family)
            .with_weight(run.weight)
            .with_slant(run.slant);
        for piece in split_at_tabs(text, run.range.clone()) {
            let is_tab = text.get(piece.clone()).is_some_and(|slice| slice == "\t");
            for item in itemise(text, piece.clone(), bidi, engine.fonts, &request)? {
                styled.push(StyledItem {
                    item,
                    run: index,
                    size: run.size,
                    language: run.language.clone(),
                    is_tab,
                });
            }
        }
    }
    Ok(styled)
}

/// Splits `range` of `text` so that every tab character is a piece of its own.
fn split_at_tabs(text: &str, range: Range<usize>) -> Vec<Range<usize>> {
    let Some(slice) = text.get(range.clone()) else {
        return Vec::new();
    };
    if !slice.contains('\t') {
        return if range.start < range.end {
            vec![range]
        } else {
            Vec::new()
        };
    }
    let mut pieces = Vec::new();
    let mut cursor = range.start;
    for (offset, character) in slice.char_indices() {
        if character != '\t' {
            continue;
        }
        let at = range.start + offset;
        if cursor < at {
            pieces.push(cursor..at);
        }
        pieces.push(at..at + character.len_utf8());
        cursor = at + character.len_utf8();
    }
    if cursor < range.end {
        pieces.push(cursor..range.end);
    }
    pieces
}

/// The [`TextRun`]s a [`LineComposer`] takes, borrowing the faces the items own.
///
/// Returns `None` when an item's face could not be registered with the rasteriser, which is the one
/// thing that can go wrong here and which drops the item rather than failing the paragraph — a face
/// that will not register is a face that will not draw, and a page missing one word is better than
/// a slide that will not open.
#[must_use]
pub fn composer_runs<'a>(
    rasteriser: &mut GlyphRasteriser,
    features: &'a FeatureSet,
    items: &'a [StyledItem],
) -> Vec<TextRun<'a>> {
    let mut runs = Vec::with_capacity(items.len());
    for styled in items {
        let Some(face) = styled.item.face() else {
            continue;
        };
        let Ok(face_id) = rasteriser.register(face) else {
            continue;
        };
        runs.push(TextRun {
            range: styled.item.range.clone(),
            face,
            face_id,
            script: styled.item.script,
            direction: styled.item.direction,
            level: styled.item.level,
            size: styled.size,
            features,
            language: styled.language.as_deref(),
            // A slide's inline objects — a `a:fld`'s rendered text above all — are ordinary
            // characters of the paragraph rather than atomic boxes, so nothing here declares a fixed
            // advance yet. See `mjx_layout::TextRun::advance`.
            advance: None,
        });
    }
    runs
}

/// Which item a composed segment came from, by its byte range.
///
/// A segment is a sub-range of exactly one item, so the item whose range contains the segment's
/// start is the one — and a segment that matches no item at all (which the composer does not
/// produce) is reported as `None` rather than assumed to be the first.
#[must_use]
pub fn item_of(items: &[StyledItem], segment: &Range<usize>) -> Option<usize> {
    items.iter().position(|styled| {
        segment.start >= styled.item.range.start && segment.start < styled.item.range.end
    })
}

/// A paragraph's tab stops: the ones it states, and the regular grid it falls back to.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TabStops {
    stops: Vec<TabStop>,
    default_size: Emu,
}

impl TabStops {
    /// `a:pPr@defTabSz`'s schema default — one inch.
    pub const DEFAULT_SIZE: Emu = Emu::from_emu(914_400);

    /// The stops a paragraph states, sorted, with `default_size` for the grid past the last one.
    ///
    /// A `default_size` of zero or less would make the grid an infinite loop, so it falls back to
    /// [`TabStops::DEFAULT_SIZE`] — a document may state `defTabSz="0"` and a renderer may not hang.
    #[must_use]
    pub fn new(mut stops: Vec<TabStop>, default_size: Emu) -> Self {
        stops.sort_by_key(|stop| stop.position.emu());
        Self {
            stops,
            default_size: if default_size > Emu::ZERO {
                default_size
            } else {
                Self::DEFAULT_SIZE
            },
        }
    }

    /// The stops it states, in order.
    #[must_use]
    pub fn stated(&self) -> &[TabStop] {
        &self.stops
    }

    /// Where a tab at `position` advances to, measured from the same origin the stops are.
    ///
    /// The first stated stop strictly past `position` wins; past the last stated stop the grid takes
    /// over, at the next multiple of the default size. A tab that would not move at all still moves
    /// one whole grid step, because a tab that advanced nothing would let a paragraph of tabs draw
    /// every one of them on top of the last.
    #[must_use]
    pub fn next_after(&self, position: Emu) -> Emu {
        if let Some(stop) = self
            .stops
            .iter()
            .find(|stop| stop.position.emu() > position.emu())
        {
            return Emu::from_emu(stop.position.emu());
        }
        let step = self.default_size.emu().max(1);
        let past = self.stops.last().map_or(0, |stop| stop.position.emu());
        let from = position.emu().max(past);
        // The next multiple of `step` strictly greater than `from`, for a negative `from` too.
        let multiples = from.div_euclid(step).saturating_add(1);
        Emu::from_emu(multiples.saturating_mul(step))
    }

    /// The alignment of the stop a tab at `position` lands on, or `None` for the default grid.
    ///
    /// Reported rather than applied: [`TabAlignment::Decimal`] and [`TabAlignment::Center`] need the
    /// width of the text *after* the tab, which is a second pass this box model does not make.
    /// Saying so with a value is what lets a later child implement it without guessing what was
    /// meant.
    #[must_use]
    pub fn alignment_after(&self, position: Emu) -> Option<TabAlignment> {
        self.stops
            .iter()
            .find(|stop| stop.position.emu() > position.emu())
            .and_then(|stop| stop.alignment)
    }
}

/// Composes every line of one paragraph at `measure_of(line_index)` points.
///
/// The measure is a function of the line number because the first line of a paragraph is indented
/// differently from the rest — that is what `a:pPr@indent` is — and a composer that took one measure
/// could not express a hanging indent.
///
/// # Errors
/// [`FontError`] when a face will not shape.
pub fn compose_paragraph(
    shaper: &mut Shaper,
    text: &str,
    runs: &[TextRun<'_>],
    bidi: &BidiAnalysis,
    options: LineBreakOptions,
    mut measure_of: impl FnMut(usize) -> f64,
) -> Result<Vec<ComposedLine>, FontError> {
    let composer = LineComposer::new(text, runs, bidi, options);
    let mut lines = Vec::new();
    let mut offset = 0_usize;
    while offset < text.len() {
        let measure = measure_of(lines.len());
        let line = composer.next_line(shaper, offset, measure)?;
        let end = line.range.end;
        lines.push(line);
        if end <= offset {
            // The composer could not advance — a measure narrower than one glyph. Taking the rest
            // of the paragraph as one line is what stops the loop, and it is what a reader sees in
            // PowerPoint too: text that overflows a box too narrow to hold it.
            if let Some(last) = lines.last_mut() {
                last.range = offset..text.len();
            }
            break;
        }
        offset = end;
    }
    Ok(lines)
}

/// The height of one line of `face` at `size`, for a paragraph that has no text to measure.
///
/// An empty paragraph still occupies a line — a reader must be able to put a caret in it — and there
/// are no glyphs to take an ascent from, so the face's own metrics answer.
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

/// Resolves one face for a run style, for the places that need a face without shaping — an empty
/// paragraph's height, and a bullet character.
///
/// # Errors
/// [`FontError`] when an indexed face will not parse.
pub fn resolve_face(
    fonts: &mut FontResolver,
    rasteriser: &mut GlyphRasteriser,
    style: &RunStyle,
) -> Result<Option<(Arc<FontFace>, FaceId)>, FontError> {
    let request = FontRequest::new(&style.family)
        .with_weight(style.weight)
        .with_slant(style.slant);
    let resolution = fonts.resolve(&request)?;
    let Some(face) = resolution.face() else {
        return Ok(None);
    };
    let face_id = rasteriser.register(face)?;
    Ok(Some((Arc::clone(face), face_id)))
}

/// The paragraph direction a resolved `rtl` flag means.
#[must_use]
pub fn paragraph_direction(right_to_left: Option<bool>) -> ParagraphDirection {
    match right_to_left {
        Some(true) => ParagraphDirection::RightToLeft,
        Some(false) => ParagraphDirection::LeftToRight,
        // `a:pPr@rtl` unstated means the paragraph takes its direction from its own text, which is
        // exactly what UAX #9's first-strong rule computes.
        None => ParagraphDirection::FromFirstStrongCharacter,
    }
}
