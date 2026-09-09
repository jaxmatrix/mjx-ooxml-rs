//! The join to the font engine: itemisation, the runs a [`LineComposer`] takes, and the one place
//! this crate decides where a run may be *cut*.
//!
//! Nothing here measures anything. Every width, ascent and descent comes from a
//! [`ComposedLine`](mjx_layout::ComposedLine) the shaper produced, every break opportunity from
//! `mjx-text`'s UAX #14 implementation, and every face from `mjx-text`'s resolver through
//! [`itemise`]. A box model that chose faces itself would be a second font engine.
//!
//! # Why a run is cut, and where
//!
//! A [`ComposedSegment`](mjx_layout::ComposedSegment) is placed as a unit: it carries a
//! [`ShapedRun`] whose glyph advances are fixed, and a subrange of one is not a
//! `ShapedRun`. So **anything this crate needs to move independently has to be its own segment**,
//! and that is decided before shaping, by cutting the items.
//!
//! Three things need it, and each cuts at a different boundary:
//!
//! * **A tab** is not a glyph whose advance is its width — it is the distance to the next stop,
//!   which depends on where the pen already is. Cut at every `\t`.
//! * **Justification** widens the gaps *between words*, which means each word must move on its own.
//!   Cut after every space, but **only in a justified paragraph** — a left-aligned paragraph gains
//!   nothing from the cut and loses the kerning across the cut, so it is not paid for.
//! * **Distributed** justification widens the gaps between *characters*. Cut at every grapheme
//!   cluster boundary, again only in a paragraph that asks for it.
//!
//! The cost of cutting is real: two adjacent items are shaped separately, so a ligature or a kern
//! across the cut is lost. That is why the cut is conditional and why the condition is the
//! paragraph's own alignment. `w:jc="both"` on a Latin paragraph loses the kern across a space,
//! which is where a kern is worth least.

use std::ops::Range;
use std::sync::Arc;

use mjx_layout::{LineComposer, TextRun};
use mjx_text::{
    grapheme_cluster_boundaries, itemise, BidiAnalysis, BidiLevel, FaceId, FeatureSet, FontError,
    FontFace, FontRequest, FontResolver, FontSize, GlyphRasteriser, Hyphenator, LineBreakOptions,
    ParagraphDirection, ShapedRun, Shaper, ShapingRequest, TextDirection, TextScript,
};

use crate::style::{Alignment, RunStyle};

/// Everything the font engine needs, borrowed together so one call can shape.
///
/// A struct of four `&mut` rather than four arguments because the box model owns all four and Rust
/// will not let `self.shaper` and `self.fonts` both be borrowed through `self`. Destructuring once
/// at the top of a layout pass is the whole of the trick — `mjx-layout-pptx` learned it first.
#[derive(Debug)]
pub struct TextEngine<'a> {
    /// The three resolution tiers, the substitution table and the manifest.
    pub fonts: &'a mut FontResolver,
    /// Where a [`FaceId`] comes from.
    ///
    /// ⚠ **The same rasteriser the painter will draw from.** A `FaceId` is minted here and looked up
    /// there; a box model that measured against a rasteriser of its own would issue identifiers a
    /// painter's atlas has never heard of, and every glyph would draw from an empty page **with no
    /// error anywhere**. See [`crate::DocumentBoxModel::rasteriser_mut`](crate::DocumentBoxModel).
    pub rasteriser: &'a mut GlyphRasteriser,
    /// The shaper, which owns the shaping context.
    pub shaper: &'a mut Shaper,
    /// The OpenType features every run in this document is shaped with.
    pub features: &'a FeatureSet,
}

/// One item of a paragraph, shaped-ready: what [`itemise`] produced plus the size and language the
/// document's run stated.
///
/// It carries the item's fields rather than the [`TextItem`](mjx_text::TextItem) itself because of **one** case, and it
/// is a case a Latin fixture never reaches: a tab. `U+0009` is a control character and no ordinary
/// text face maps it, so asking the resolver for a face that can draw one answers
/// [`FontResolution::Unresolvable`](mjx_text::FontResolution) — and an item with no face is dropped,
/// which means the tab is not a segment, which means **the pen never moves to the stop**. A tab that
/// silently becomes a zero-width nothing is a table of contents with its page numbers piled against
/// its titles.
///
/// So a tab takes the face its *run* resolves to, asked for by family rather than by character, and
/// that is what this type's own `face` field is for.
#[derive(Debug)]
pub struct StyledItem {
    /// The bytes it covers, in the paragraph's own offsets.
    pub range: Range<usize>,
    /// The one script it is in.
    pub script: TextScript,
    /// The UAX #9 level it was resolved at.
    pub level: BidiLevel,
    /// Which way it is shaped.
    pub direction: TextDirection,
    /// The face it is drawn in, when one was found.
    pub face: Option<Arc<FontFace>>,
    /// Which of the paragraph's runs it came from.
    pub run: usize,
    /// The size that run renders at.
    pub size: FontSize,
    /// The language that run declares.
    pub language: Option<String>,
    /// Whether the item is exactly one tab character, which is placed rather than drawn.
    pub is_tab: bool,
    /// A fixed advance in points, for the one `U+FFFC` standing for an inline object.
    ///
    /// It reaches [`mjx_layout::TextRun::advance`] through [`composer_runs`], and it is what makes a
    /// line carrying a picture or an equation **measured with it on**. See
    /// [`crate::generated`] for why an object is one character and not a width beside the run list.
    pub advance: Option<f64>,
}

/// Where a paragraph's runs may be cut before shaping.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CutPolicy {
    /// Only at tabs. What a left-, right- or centre-aligned paragraph needs.
    TabsOnly,
    /// At tabs and after every space, so each word can move — `w:jc="both"`.
    Words,
    /// At tabs and at every grapheme cluster boundary — `w:jc="distribute"`.
    Characters,
}

impl CutPolicy {
    /// What `alignment` needs.
    #[must_use]
    pub fn of(alignment: Alignment) -> Self {
        match alignment {
            Alignment::Start | Alignment::Centre | Alignment::End => Self::TabsOnly,
            Alignment::Justified => Self::Words,
            Alignment::Distributed => Self::Characters,
        }
    }
}

/// Cuts a paragraph into items: one pass of `mjx-text`'s bidirectional resolution, script
/// itemisation and face fallback per document run, with [`CutPolicy`]'s boundaries forced.
///
/// A **hidden** run (`w:vanish`) contributes no items at all, which is what makes it hidden: its
/// text keeps its byte range in the paragraph so every offset still lines up, and nothing is shaped
/// or drawn for it.
///
/// # Errors
/// [`FontError`] when an indexed face will not parse.
pub fn itemise_paragraph(
    engine: &mut TextEngine<'_>,
    text: &str,
    runs: &[RunStyle],
    bidi: &BidiAnalysis,
    policy: CutPolicy,
    objects: &[crate::generated::InlineObject],
) -> Result<Vec<StyledItem>, FontError> {
    let mut styled = Vec::new();
    for (index, run) in runs.iter().enumerate() {
        if run.hidden {
            continue;
        }
        let request = FontRequest::new(&run.family)
            .with_weight(run.weight)
            .with_slant(run.slant);
        for piece in cut(text, run.range.clone(), policy) {
            // An inline object: one `U+FFFC`, given the *run's* face for the same reason a tab is
            // given one — no ordinary text face maps `U+FFFC`, an item with no face is dropped, and
            // a dropped item is an object that reserves nothing. Its glyphs are never drawn: the
            // fixed advance makes `mjx_layout::LineComposer` skip shaping it entirely.
            if let Some(object) = objects.iter().find(|object| object.at == piece.start) {
                let face = engine.fonts.resolve(&request)?.face().map(Arc::clone);
                styled.push(StyledItem {
                    range: piece,
                    script: TextScript::COMMON,
                    level: bidi.level_at(run.range.start),
                    direction: bidi.level_at(run.range.start).direction(),
                    face,
                    run: index,
                    size: run.size,
                    language: run.language.clone(),
                    is_tab: false,
                    advance: Some(object.width.points()),
                });
                continue;
            }
            if text.get(piece.clone()).is_some_and(|slice| slice == "\t") {
                // See [`StyledItem`]: a tab has no glyph in any ordinary face, so it is given the
                // face its run resolves to rather than the one that can draw a `U+0009`.
                let face = engine.fonts.resolve(&request)?.face().map(Arc::clone);
                styled.push(StyledItem {
                    range: piece,
                    script: TextScript::COMMON,
                    level: bidi.level_at(run.range.start),
                    direction: bidi.level_at(run.range.start).direction(),
                    face,
                    run: index,
                    size: run.size,
                    language: run.language.clone(),
                    is_tab: true,
                    advance: None,
                });
                continue;
            }
            for item in itemise(text, piece.clone(), bidi, engine.fonts, &request)? {
                styled.push(StyledItem {
                    range: item.range.clone(),
                    script: item.script,
                    level: item.level,
                    direction: item.direction,
                    face: item.face().map(Arc::clone),
                    run: index,
                    size: run.size,
                    language: run.language.clone(),
                    is_tab: false,
                    advance: None,
                });
            }
        }
    }
    Ok(styled)
}

/// Splits `range` of `text` into the pieces `policy` allows to move independently.
#[must_use]
pub fn cut(text: &str, range: Range<usize>, policy: CutPolicy) -> Vec<Range<usize>> {
    let Some(slice) = text.get(range.clone()) else {
        return Vec::new();
    };
    if range.start >= range.end {
        return Vec::new();
    }
    let mut boundaries: Vec<usize> = Vec::new();
    match policy {
        CutPolicy::TabsOnly | CutPolicy::Words => {
            for (offset, character) in slice.char_indices() {
                let at = range.start + offset;
                let after = at + character.len_utf8();
                if character == '\t' {
                    boundaries.push(at);
                    boundaries.push(after);
                } else if policy == CutPolicy::Words {
                    if is_expansion_space(character) {
                        // After the space, not before it: the space belongs to the word it follows,
                        // so that widening the gap moves the *next* word and leaves this one's
                        // trailing space attached to it. Cutting before would put the space at the
                        // head of the next segment and a justified line would start every word with
                        // a gap.
                        boundaries.push(after);
                    } else if crate::justify::is_east_asian(character) {
                        // **An ideograph is its own word.** A Japanese line has no spaces on it, so
                        // a justifier that only ever cut at spaces would leave it in one piece and
                        // therefore leave it unjustified — a ragged right edge in a paragraph that
                        // asked to be flush. Word widens the gaps *between the characters* instead,
                        // and this is the cut that makes that possible.
                        boundaries.push(at);
                        boundaries.push(after);
                    }
                }
            }
        }
        CutPolicy::Characters => {
            for offset in grapheme_cluster_boundaries(slice) {
                boundaries.push(range.start + offset);
            }
        }
    }
    boundaries.push(range.start);
    boundaries.push(range.end);
    boundaries.retain(|at| *at >= range.start && *at <= range.end && text.is_char_boundary(*at));
    boundaries.sort_unstable();
    boundaries.dedup();
    boundaries
        .windows(2)
        .filter(|pair| pair[0] < pair[1])
        .map(|pair| pair[0]..pair[1])
        .collect()
}

/// Whether `character` is a space whose width justification is allowed to change.
///
/// The ordinary space and the ideographic space. **Not** the no-break space (`U+00A0`), which exists
/// precisely so that it does not behave like a space, and not the fixed-width spaces (`U+2000`…),
/// whose whole definition is their width.
#[must_use]
pub fn is_expansion_space(character: char) -> bool {
    matches!(character, ' ' | '\u{3000}')
}

/// The [`TextRun`]s a [`LineComposer`] takes, borrowing the faces the items own.
///
/// An item whose face will not register with the rasteriser is **dropped** rather than failing the
/// paragraph: a face that will not register is a face that will not draw, and a page missing one
/// word is better than a document that will not open.
#[must_use]
pub fn composer_runs<'a>(
    rasteriser: &mut GlyphRasteriser,
    features: &'a FeatureSet,
    items: &'a [StyledItem],
) -> Vec<TextRun<'a>> {
    let mut runs = Vec::with_capacity(items.len());
    for styled in items {
        let Some(face) = styled.face.as_ref() else {
            continue;
        };
        let Ok(face_id) = rasteriser.register(face) else {
            continue;
        };
        runs.push(TextRun {
            range: styled.range.clone(),
            face,
            face_id,
            script: styled.script,
            direction: styled.direction,
            level: styled.level,
            size: styled.size,
            features,
            language: styled.language.as_deref(),
            advance: styled.advance,
        });
    }
    runs
}

/// Which item a composed segment came from, by its byte range.
#[must_use]
pub fn item_of(items: &[StyledItem], segment: &Range<usize>) -> Option<usize> {
    items
        .iter()
        .position(|styled| segment.start >= styled.range.start && segment.start < styled.range.end)
}

/// The composer for one paragraph, hyphenating or not.
///
/// The hyphen's advance has to be measured *before* the composer exists, because every candidate the
/// fitting loop measures at a hyphenation point is a slice **plus a hyphen** — see
/// [`LineComposer::hyphenating`].
#[must_use]
pub fn composer<'a>(
    text: &'a str,
    runs: &'a [TextRun<'a>],
    bidi: &'a BidiAnalysis,
    options: LineBreakOptions,
    hyphenation: Option<(&dyn Hyphenator, f64)>,
) -> LineComposer<'a> {
    match hyphenation {
        Some((hyphenator, width)) => {
            LineComposer::hyphenating(text, runs, bidi, options, hyphenator, width)
        }
        None => LineComposer::new(text, runs, bidi, options),
    }
}

/// `U+2010 HYPHEN` — what a hyphenated line ends with.
///
/// **Not `U+002D HYPHEN-MINUS`.** The two look alike in most faces and are different characters: the
/// hyphen-minus is the ASCII compromise and `U+2010` is the typographic hyphen, which is what a
/// typesetter inserts at a line break. A face that has one and not the other falls back through
/// `mjx-text`'s resolver like any other character.
pub const HYPHEN: char = '\u{2010}';

/// The hyphen, shaped in `face` at `size`, and how wide it is.
///
/// # Errors
/// [`FontError`] when the face will not shape.
pub fn shape_hyphen(
    shaper: &mut Shaper,
    face: &Arc<FontFace>,
    size: FontSize,
    features: &FeatureSet,
) -> Result<(ShapedRun, f64), FontError> {
    let text = "\u{2010}";
    let request = ShapingRequest::new(text, TextScript::COMMON, size, features);
    let run = shaper.shape(face, &request)?;
    let width = run.advance_in_points();
    Ok((run, width))
}

/// Resolves one face for a run style, for the places that need a face without shaping — an empty
/// paragraph's height, and the hyphen a hyphenated line ends with.
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

/// The paragraph direction a resolved `w:bidi` flag means.
#[must_use]
pub fn paragraph_direction(right_to_left: bool) -> ParagraphDirection {
    if right_to_left {
        ParagraphDirection::RightToLeft
    } else {
        // A paragraph that does not write `w:bidi` takes its direction from its own text, which is
        // exactly what UAX #9's first-strong rule computes. Forcing left-to-right here would draw
        // an Arabic paragraph in an unmarked document backwards.
        ParagraphDirection::FromFirstStrongCharacter
    }
}
