//! Turning a run of Unicode into positioned glyphs.
//!
//! # Why `rustybuzz`, and why not Parley
//!
//! Shaping is where Office typography either is or is not reproduced. Kerning, ligatures,
//! contextual alternates, Arabic joining, Indic reordering and mark positioning are all decided
//! here, and a renderer that draws one glyph per code point looks approximately right in Latin and
//! visibly broken in half the world's scripts.
//!
//! `rustybuzz` is a pure-Rust port of HarfBuzz, which is the engine Office itself shapes with. What
//! it does with an Arabic joining form or a Devanagari cluster is what Word does with one, and that
//! behavioural correspondence — not merely "it produces glyphs" — is why it is the choice.
//!
//! **Parley is deliberately not adopted, and this decision does not need re-opening.** Parley is a
//! *layout* library: it owns line breaking, alignment, and the arrangement of runs into lines, and
//! it makes those decisions the way a web engine does. This project's line and page decisions have
//! to come out where Office's do, which is a different set of rules — `docs/UI_PLATFORM_PLAN.md` §4
//! puts them in `mjx-layout` and the three box models above it. What this crate needs from a text
//! stack is the **primitives**: shape this run, resolve these levels, offer these break
//! opportunities. Adopting Parley would mean adopting a layout policy in order to reach them, and
//! then fighting it everywhere it disagreed.
//!
//! # Shaping happens in font units
//!
//! `rustybuzz` scales its output by the face's `unitsPerEm` unless it is told a pixel size, and this
//! crate never tells it one: hinting and pixel grid-fitting are R04's, and a shaped run that was
//! rounded to a pixel size could not be reused at another zoom level. So every advance and offset in
//! a [`ShapedRun`] is in the face's own units, exactly as [`crate::AdvanceWidth`] carries them, and
//! [`ShapedRun::advance_in_points`] is where a size is finally applied.
//!
//! The [`FontSize`] in a [`ShapingRequest`] is therefore not an input to the shaper — it is part of
//! the run's identity, because the size decides whether the document's kerning threshold applied and
//! what the caller will scale the result by. It is in the cache key for that reason.
//!
//! # Nothing here panics on a font or on text
//!
//! A face inside a `.pptx` is exactly as untrusted as the `.pptx`, and so is the text. The advance
//! sum is accumulated in `i64` and refused as [`crate::FontError::ShapedRunTooWide`] if it will not
//! fit the `i32` an advance carries, rather than wrapping; a face whose bytes will not parse a
//! second time comes back as [`crate::FontError::MalformedFace`]; and unassigned code points,
//! unpaired combining marks, noncharacters and default-ignorables all shape to whatever the face
//! maps them to, which is usually `.notdef`.

use std::fmt;
use std::sync::Arc;

use crate::cache::{CacheStatistics, ShapedRunCache};
use crate::direction::TextDirection;
use crate::error::FontError;
use crate::face::{AdvanceWidth, FontFace, GlyphIndex};
use crate::feature::FeatureSet;
use crate::script::TextScript;

/// A type size, held exactly so that it can be part of a cache key.
///
/// Stored as thousandths of a point, which is finer than any document can express: OOXML writes
/// sizes in half-points (`w:sz`) and hundredths of a point (`a:rPr/@sz`), and both divide into a
/// thousandth exactly. A `f64` could not be a key at all — it is neither `Eq` nor `Hash`, and two
/// sizes that print the same can differ in the last bit.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct FontSize(u32);

impl FontSize {
    /// The largest size this type can hold, a little over four million points.
    pub const MAXIMUM: Self = Self(u32::MAX);

    /// A size in points.
    ///
    /// Saturating rather than fallible, and deliberately: a size comes from a document, and a
    /// document may say `sz="2147483647"` or nothing at all. A negative size, a `NaN` and an
    /// infinity all become zero or the maximum rather than a panic or an error a caller would have
    /// to invent a recovery for, and the value is exact for every size a document can actually
    /// express.
    #[must_use]
    pub fn from_points(points: f64) -> Self {
        // `NaN` compares false against everything, so it has to be named rather than ordered.
        if points.is_nan() || points <= 0.0 {
            return Self(0);
        }
        let thousandths = points * 1000.0;
        // A positive infinity lands here, which is the honest answer for it: unboundedly large
        // saturates to the largest size this type holds rather than collapsing to nothing.
        if thousandths >= f64::from(u32::MAX) {
            return Self::MAXIMUM;
        }
        // `thousandths` is finite, positive and below `u32::MAX`, so the cast is exact within the
        // rounding `round` already applied.
        Self(thousandths.round() as u32)
    }

    /// A size in thousandths of a point.
    #[must_use]
    pub const fn from_thousandths_of_a_point(thousandths: u32) -> Self {
        Self(thousandths)
    }

    /// A size in half-points, which is how `w:sz` writes one.
    #[must_use]
    pub const fn from_half_points(half_points: u32) -> Self {
        Self(half_points.saturating_mul(500))
    }

    /// A size in hundredths of a point, which is how `a:rPr/@sz` writes one.
    #[must_use]
    pub const fn from_hundredths_of_a_point(hundredths: u32) -> Self {
        Self(hundredths.saturating_mul(10))
    }

    /// The size in points.
    #[must_use]
    pub fn in_points(self) -> f64 {
        f64::from(self.0) / 1000.0
    }

    /// The size in thousandths of a point, as it is stored.
    #[must_use]
    pub const fn in_thousandths_of_a_point(self) -> u32 {
        self.0
    }
}

impl fmt::Debug for FontSize {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "FontSize({}pt)", self.in_points())
    }
}

/// One glyph the shaper produced.
///
/// Advances and offsets are in the face's own units; `units_per_em` is on the [`ShapedRun`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ShapedGlyph {
    /// Which glyph in the face.
    pub glyph: GlyphIndex,
    /// The byte offset, **within the shaped text**, of the first character this glyph belongs to.
    ///
    /// Several glyphs may share a cluster — a Devanagari syllable draws as several — and one glyph
    /// may span several characters, which is what a ligature is. It is therefore a mapping in both
    /// directions, and the only sound way to map a caret position onto a glyph.
    pub cluster: u32,
    /// How far the pen moves horizontally after drawing it.
    pub x_advance: i32,
    /// How far the pen moves vertically after drawing it — zero for horizontal text.
    pub y_advance: i32,
    /// Where the glyph is drawn relative to the pen, horizontally.
    pub x_offset: i32,
    /// Where the glyph is drawn relative to the pen, vertically. This is what positions an Arabic
    /// or Devanagari mark over its base.
    pub y_offset: i32,
    /// Whether breaking the run immediately **before** this glyph would change the shaping.
    ///
    /// HarfBuzz sets this where a substitution or a kern spans the boundary. A line breaker that
    /// splits here must re-shape both halves rather than slicing the glyph list, and one that
    /// slices anyway will lose a ligature or a kern at every line end.
    pub unsafe_to_break: bool,
}

/// Everything about a run that is not the face it is drawn in.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ShapingRequest<'a> {
    /// The run's text, in **logical** order. The shaper reverses it for a right-to-left run; the
    /// caller never does.
    pub text: &'a str,
    /// Which way the run is written, from its embedding level.
    pub direction: TextDirection,
    /// The one script the run is in. Itemisation guarantees there is only one; see
    /// [`crate::itemise_by_script`].
    pub script: TextScript,
    /// The run's language, as a BCP 47 tag — `w:lang`, `a:rPr/@lang`.
    ///
    /// It selects a language system inside the font, which is how a Serbian `б` gets its cursive
    /// form and a Turkish `i` keeps its dot. A tag the shaper cannot read is ignored rather than
    /// refused, because a document may carry anything.
    pub language: Option<&'a str>,
    /// The size the run is set at. Not an input to the shaper — see the module documentation — but
    /// part of the run's identity.
    pub size: FontSize,
    /// The features the run is shaped with.
    pub features: &'a FeatureSet,
}

impl<'a> ShapingRequest<'a> {
    /// A left-to-right request for `text` in `script` with `features`, at `size`.
    #[must_use]
    pub fn new(
        text: &'a str,
        script: TextScript,
        size: FontSize,
        features: &'a FeatureSet,
    ) -> Self {
        Self {
            text,
            direction: TextDirection::LeftToRight,
            script,
            language: None,
            size,
            features,
        }
    }

    /// The same request, in `direction`.
    #[must_use]
    pub fn in_direction(mut self, direction: TextDirection) -> Self {
        self.direction = direction;
        self
    }

    /// The same request, in `language`.
    #[must_use]
    pub fn in_language(mut self, language: &'a str) -> Self {
        self.language = Some(language);
        self
    }
}

/// A run of text, shaped.
///
/// Cheap to clone: the glyphs sit behind an [`Arc`] the cache also holds, so a cache hit and the run
/// it was shaped from share one allocation. [`ShapedRun::shares_glyphs_with`] is how a caller — or a
/// test — can tell that they did.
///
/// # Equality is by value, identity is by [`ShapedRun::shares_glyphs_with`]
///
/// Two runs are equal when they carry the same glyphs at the same size in the same direction against
/// the same em square, whether or not they came from the same shaping call. The distinction matters
/// and both halves are needed: `mjx-layout` proves that resuming a page from a checkpoint produces
/// the *same fragments* as laying the pages out in order, which is a question about values — the two
/// runs are shaped by two different calls and must still compare equal — while the shaped-run
/// cache's own gate is a question about identity. Added in MJXOFF-160 for the first of those.
#[derive(Clone, PartialEq, Debug)]
pub struct ShapedRun {
    glyphs: Arc<[ShapedGlyph]>,
    units_per_em: u16,
    size: FontSize,
    direction: TextDirection,
    advance: AdvanceWidth,
}

impl ShapedRun {
    /// The glyphs, in the order they are drawn — left to right on the page for a left-to-right run,
    /// and **also** left to right for a right-to-left one, because the shaper has already reordered
    /// them. A painter walks this list forwards whatever the direction.
    #[must_use]
    pub fn glyphs(&self) -> &[ShapedGlyph] {
        &self.glyphs
    }

    /// How many glyphs the run produced. Not the number of characters, and the difference is the
    /// point: a ligature makes it smaller and a decomposed Indic syllable makes it larger.
    #[must_use]
    pub fn len(&self) -> usize {
        self.glyphs.len()
    }

    /// Whether the run produced no glyphs at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.glyphs.is_empty()
    }

    /// The run's total advance, in the face's units, with its em square attached.
    #[must_use]
    pub fn advance(&self) -> AdvanceWidth {
        self.advance
    }

    /// The run's total advance in typographic points at the size it was shaped for.
    #[must_use]
    pub fn advance_in_points(&self) -> f64 {
        self.advance.at_size(self.size.in_points())
    }

    /// Which way the run was shaped.
    #[must_use]
    pub fn direction(&self) -> TextDirection {
        self.direction
    }

    /// The em square every number in the run is measured against.
    #[must_use]
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// The size the run was shaped for.
    #[must_use]
    pub fn size(&self) -> FontSize {
        self.size
    }

    /// Whether this run is exactly as wide as `other`, as a fraction of an em, **across differing em
    /// squares**.
    ///
    /// This is the question metric compatibility asks of a substitution: a run set in a 1000-unit-em
    /// face and the same run set in the 2048-unit-em face that replaced it occupy the same width if
    /// and only if `a₁/e₁ == a₂/e₂`, and comparing the two `font_units` directly answers `false` for
    /// every pair that is in fact identical. [`AdvanceWidth::equals`] does the comparison exactly, in
    /// `i64`, without rescaling either side into the other's rounding — which is why the answer is
    /// not "within a tolerance" like [`crate::ADVANCE_TOLERANCE_PER_MILLE`], but *equal*.
    #[must_use]
    pub fn occupies_the_same_width_as(&self, other: &Self) -> bool {
        self.advance.equals(other.advance)
    }

    /// Whether the two runs are backed by the **same** glyph allocation, which they are exactly when
    /// one came out of the shaped-run cache and the other put it there.
    ///
    /// This is the observable difference between a cache hit and a re-shape, and it is what proves
    /// the cache did no work rather than merely produced the same answer twice.
    #[must_use]
    pub fn shares_glyphs_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.glyphs, &other.glyphs)
    }
}

/// Shapes runs, and remembers the ones it has already shaped.
///
/// A document repeats text constantly — the same word in the same face at the same size, the same
/// empty cell, the same bullet — and a scroll re-shapes whatever came back into view. The cache is
/// what makes that affordable; see [`crate::ShapedRunCache`] for how it is keyed and evicted.
#[derive(Debug)]
pub struct Shaper {
    cache: ShapedRunCache,
}

impl Shaper {
    /// A shaper with the default cache capacity.
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: ShapedRunCache::new(),
        }
    }

    /// A shaper whose cache holds at most `capacity` runs.
    ///
    /// A capacity of zero disables the cache without disabling the shaper, which is the right
    /// configuration for a one-pass export that will never ask for the same run twice.
    #[must_use]
    pub fn with_cache_capacity(capacity: usize) -> Self {
        Self {
            cache: ShapedRunCache::with_capacity(capacity),
        }
    }

    /// What the cache has been doing.
    #[must_use]
    pub fn cache_statistics(&self) -> CacheStatistics {
        self.cache.statistics()
    }

    /// Forget every shaped run.
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Shape `request` in `face`.
    ///
    /// # Errors
    ///
    /// [`FontError::MalformedFace`] if the face's bytes will not parse — they already did once, when
    /// the [`FontFace`] was built, so this is the file-vanished-underneath case rather than a
    /// condition a caller can provoke. [`FontError::ShapedRunTooWide`] if the run's advances sum past
    /// what an [`AdvanceWidth`] can carry, which needs roughly a million ems of text in one run.
    pub fn shape(
        &mut self,
        face: &Arc<FontFace>,
        request: &ShapingRequest<'_>,
    ) -> Result<ShapedRun, FontError> {
        let units_per_em = face.metrics().units_per_em;

        if let Some(glyphs) = self.cache.get(face, request) {
            let advance = total_advance(&glyphs, units_per_em, request.text)?;
            return Ok(ShapedRun {
                glyphs,
                units_per_em,
                size: request.size,
                direction: request.direction,
                advance,
            });
        }

        let glyphs = shape_uncached(face, request)?;
        let advance = total_advance(&glyphs, units_per_em, request.text)?;
        self.cache.insert(face, request, Arc::clone(&glyphs));
        Ok(ShapedRun {
            glyphs,
            units_per_em,
            size: request.size,
            direction: request.direction,
            advance,
        })
    }
}

impl Default for Shaper {
    fn default() -> Self {
        Self::new()
    }
}

/// Shape one run without consulting or filling any cache.
///
/// Exposed because a one-shot caller — a measurement, a test — should not have to build a
/// [`Shaper`] and then throw its cache away.
///
/// # Errors
///
/// [`FontError::MalformedFace`] if the face's bytes will not parse.
pub fn shape_uncached(
    face: &Arc<FontFace>,
    request: &ShapingRequest<'_>,
) -> Result<Arc<[ShapedGlyph]>, FontError> {
    let Some(shaper_face) = rustybuzz::Face::from_slice(face.data(), face.index()) else {
        // `FontFace::parse` already read these bytes, so reaching here means the `Arc` no longer
        // holds what it did. Reporting it is right; asserting it cannot happen is not.
        return Err(FontError::MalformedFace {
            source: ttf_parser::FaceParsingError::MalformedFont,
        });
    };

    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(request.text);
    buffer.set_direction(match request.direction {
        TextDirection::LeftToRight => rustybuzz::Direction::LeftToRight,
        TextDirection::RightToLeft => rustybuzz::Direction::RightToLeft,
    });
    if let Some(script) = rustybuzz::Script::from_iso15924_tag(ttf_parser::Tag::from_bytes(
        &request
            .script
            .code()
            .as_bytes()
            .try_into()
            .unwrap_or(*b"Zzzz"),
    )) {
        buffer.set_script(script);
    }
    if let Some(language) = request
        .language
        .and_then(|tag| tag.parse::<rustybuzz::Language>().ok())
    {
        buffer.set_language(language);
    }
    // Stated rather than inherited from the default: a cluster is the unit a caret sits between and
    // a selection extends by, and `MonotoneGraphemes` is the level that keeps clusters
    // non-decreasing and grapheme-aligned. R11's caret depends on it, so it is not left to a
    // default a dependency bump could move.
    buffer.set_cluster_level(rustybuzz::BufferClusterLevel::MonotoneGraphemes);

    let features: Vec<rustybuzz::Feature> = request
        .features
        .features()
        .iter()
        .map(|feature| {
            rustybuzz::Feature::new(
                ttf_parser::Tag::from_bytes(&feature.tag.bytes()),
                feature.value,
                ..,
            )
        })
        .collect();

    let shaped = rustybuzz::shape(&shaper_face, &features, buffer);
    let infos = shaped.glyph_infos();
    let positions = shaped.glyph_positions();

    let glyphs: Vec<ShapedGlyph> = infos
        .iter()
        .zip(positions.iter())
        .map(|(info, position)| ShapedGlyph {
            // A glyph id is a `u16` in every OpenType table; HarfBuzz widens it to carry its own
            // sentinels, and a face with more than 65535 glyphs cannot exist. Truncating rather
            // than refusing keeps a malformed face drawable as `.notdef` instead of failing a page.
            glyph: GlyphIndex(u16::try_from(info.glyph_id).unwrap_or(0)),
            cluster: info.cluster,
            x_advance: position.x_advance,
            y_advance: position.y_advance,
            x_offset: position.x_offset,
            y_offset: position.y_offset,
            unsafe_to_break: info.unsafe_to_break(),
        })
        .collect();

    Ok(Arc::from(glyphs))
}

fn total_advance(
    glyphs: &[ShapedGlyph],
    units_per_em: u16,
    text: &str,
) -> Result<AdvanceWidth, FontError> {
    let mut total = 0_i64;
    for glyph in glyphs {
        total += i64::from(glyph.x_advance);
    }
    let font_units = i32::try_from(total).map_err(|_| FontError::ShapedRunTooWide {
        font_units: total,
        characters: text.chars().count(),
    })?;
    Ok(AdvanceWidth {
        font_units,
        units_per_em,
    })
}
