//! Loading a face, and reading its numbers.
//!
//! # Why the parsed face is not stored
//!
//! `ttf_parser::Face<'a>` borrows the bytes it was parsed from, so a struct holding both the bytes
//! and the parsed face is self-referential — which in Rust means either a crate like `ouroboros` or
//! a hand-written `unsafe`, and this crate is under the workspace's `unsafe_code = "deny"`.
//!
//! The way out costs nothing. Parsing a face is a *validation pass over table directories*, not a
//! decode of the glyph data: everything below is `O(number of tables)` plus a handful of bounds
//! checks. So [`FontFace`] eagerly extracts the values that are read once per face — the metrics,
//! the identity, the colour formats, the variation axes — and keeps the bytes for the values that
//! are read once per *glyph*. A caller that wants those opens a [`FaceReader`] with
//! [`FontFace::reader`], pays the one parse, and then does as many glyph lookups as it likes
//! against a borrow.
//!
//! That is also the seam R03 (shaping) and R04 (rasterisation) need: they build their own
//! `rustybuzz`/`swash` view over [`FontFace::data`], and never see a `ttf_parser` type from here.

use std::fmt;
use std::sync::Arc;

use crate::error::FontError;

/// The lowest and highest `head.unitsPerEm` the OpenType specification permits.
const UNITS_PER_EM_RANGE: std::ops::RangeInclusive<u16> = 16..=16384;

/// A glyph's index within its face.
///
/// This is the same number `ttf-parser`, `rustybuzz` and the font file itself call a glyph id; it is
/// wrapped so that a caller cannot pass a character where a glyph was meant.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct GlyphIndex(pub u16);

/// A rectangle in font units, as the `glyf`/`CFF` tables report it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GlyphBounds {
    /// Leftmost extent.
    pub left: i16,
    /// Bottommost extent, below the baseline when negative.
    pub bottom: i16,
    /// Rightmost extent.
    pub right: i16,
    /// Topmost extent.
    pub top: i16,
}

/// How wide a glyph is, and the em square that width is measured against.
///
/// Advance widths from two faces are only comparable once both are expressed against the same em,
/// and the two em squares in daily use — 1000 for CFF outlines and 2048 for TrueType — are not the
/// same. Carrying the denominator with the numerator is what makes [`AdvanceWidth::equals`] able to
/// answer exactly rather than after a lossy rescale.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AdvanceWidth {
    /// The advance, in the face's own units.
    pub font_units: i32,
    /// The face's `head.unitsPerEm`.
    pub units_per_em: u16,
}

impl AdvanceWidth {
    /// The advance expressed in thousandths of an em, the scale published metric tables use.
    #[must_use]
    pub fn per_mille(self) -> f64 {
        f64::from(self.font_units) * 1000.0 / f64::from(self.units_per_em)
    }

    /// The advance in typographic points at `size_in_points`.
    #[must_use]
    pub fn at_size(self, size_in_points: f64) -> f64 {
        f64::from(self.font_units) * size_in_points / f64::from(self.units_per_em)
    }

    /// Whether two advances are the same fraction of an em, compared exactly.
    ///
    /// `a₁/e₁ == a₂/e₂` is tested as `a₁·e₂ == a₂·e₁`, in `i64`, so a 1000-unit em and a 2048-unit
    /// em are compared without either being rescaled into the other's rounding.
    #[must_use]
    pub fn equals(self, other: Self) -> bool {
        i64::from(self.font_units) * i64::from(other.units_per_em)
            == i64::from(other.font_units) * i64::from(self.units_per_em)
    }

    /// How far this advance is from `other`, in thousandths of an em. Always non-negative.
    #[must_use]
    pub fn deviation_per_mille(self, other: Self) -> f64 {
        (self.per_mille() - other.per_mille()).abs()
    }
}

impl fmt::Display for AdvanceWidth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.font_units, self.units_per_em)
    }
}

/// Everything about a face that is read once per face rather than once per glyph.
///
/// All values are in the face's own units; [`FaceMetrics::units_per_em`] is the denominator.
///
/// `Eq` is deliberately absent: `italic_angle` is a `f32` straight out of `post`, and a total
/// equality over a float would be a lie about a value the face may store as `-0.0` or as a
/// non-canonical `NaN`.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct FaceMetrics {
    /// `head.unitsPerEm`. Guaranteed to be within 16..=16384, because [`FontFace::parse`] refuses
    /// a face that says otherwise.
    pub units_per_em: u16,
    /// `hhea.ascender` — how far above the baseline the platform should reserve.
    pub ascender: i16,
    /// `hhea.descender`, negative below the baseline.
    pub descender: i16,
    /// `hhea.lineGap` — the leading between one line's descent and the next line's ascent.
    pub line_gap: i16,
    /// `OS/2.sTypoAscender`, present only when the face sets `USE_TYPO_METRICS` or fills the field.
    /// Word and PowerPoint prefer the `hhea` numbers, which is why those are not optional here.
    pub typographic_ascender: Option<i16>,
    /// `OS/2.sTypoDescender`.
    pub typographic_descender: Option<i16>,
    /// `OS/2.sTypoLineGap`.
    pub typographic_line_gap: Option<i16>,
    /// `OS/2.sCapHeight` — the height of a flat capital such as `H`.
    pub cap_height: Option<i16>,
    /// `OS/2.sxHeight` — the height of a flat lowercase such as `x`.
    pub x_height: Option<i16>,
    /// `post.italicAngle`, in degrees counter-clockwise from vertical; negative for a face that
    /// slopes to the right, which is what an italic does.
    pub italic_angle: f32,
    /// Where the underline sits relative to the baseline, and how thick it is drawn.
    pub underline: Option<LineDecorationMetrics>,
    /// Where the strikethrough sits, and how thick it is drawn.
    pub strikeout: Option<LineDecorationMetrics>,
    /// `head.xMin`/`yMin`/`xMax`/`yMax` — the union of every glyph's bounding box.
    pub global_bounds: GlyphBounds,
    /// Whether `post.isFixedPitch` says every glyph has the same advance.
    pub is_monospaced: bool,
    /// How many glyphs the face holds.
    pub glyph_count: u16,
}

impl FaceMetrics {
    /// The distance from one baseline to the next, in font units: ascent − descent + line gap.
    #[must_use]
    pub fn line_height(&self) -> i32 {
        i32::from(self.ascender) - i32::from(self.descender) + i32::from(self.line_gap)
    }

    /// The same value at `size_in_points`.
    #[must_use]
    pub fn line_height_at_size(&self, size_in_points: f64) -> f64 {
        f64::from(self.line_height()) * size_in_points / f64::from(self.units_per_em)
    }
}

/// Where a drawn rule sits, and how thick it is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LineDecorationMetrics {
    /// Offset from the baseline, in font units. Negative is below.
    pub position: i16,
    /// Stroke thickness, in font units.
    pub thickness: i16,
}

/// The colour-glyph formats a face carries.
///
/// R04 rasterises these; this crate only records which are present, because *which* format a face
/// uses decides whether a fallback is even needed. A face with none of them is a plain outline face.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub struct ColourGlyphFormats {
    /// `COLR`/`CPAL`: layered outlines with a palette. The format Windows and the web prefer.
    pub layered_outlines: bool,
    /// `sbix`: embedded bitmaps at fixed strike sizes. Apple's colour-emoji format.
    pub apple_bitmaps: bool,
    /// `CBDT`/`CBLC`: embedded PNG bitmaps. Google's colour-emoji format.
    pub bitmap_data: bool,
    /// `SVG `: an SVG document per glyph.
    pub scalable_vector_graphics: bool,
}

impl ColourGlyphFormats {
    /// Whether the face carries colour glyphs in any format at all.
    #[must_use]
    pub fn any(&self) -> bool {
        self.layered_outlines
            || self.apple_bitmaps
            || self.bitmap_data
            || self.scalable_vector_graphics
    }
}

/// One axis of a variable font, in the axis's own units.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct VariationAxis {
    /// The four-character axis tag — `wght`, `wdth`, `slnt`, `ital`, `opsz`, or a private one.
    pub tag: [u8; 4],
    /// The lowest value the axis accepts.
    pub minimum: f32,
    /// The value the face has when no variation is applied.
    pub default: f32,
    /// The highest value the axis accepts.
    pub maximum: f32,
    /// Whether the axis is meant to be shown in a user interface.
    pub is_hidden: bool,
}

impl VariationAxis {
    /// The axis tag as text, for a message or a user interface.
    #[must_use]
    pub fn tag_name(&self) -> String {
        self.tag.iter().map(|byte| char::from(*byte)).collect()
    }
}

/// Whether a face is upright or sloped.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum FontSlant {
    /// Not sloped.
    #[default]
    Upright,
    /// Sloped, with the cursive letterforms a true italic draws.
    Italic,
    /// Sloped without the cursive letterforms — a slanted roman.
    Oblique,
}

/// A face's weight on the usual 1–1000 scale, where 400 is regular and 700 is bold.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FontWeight(pub u16);

impl FontWeight {
    /// 100.
    pub const THIN: Self = Self(100);
    /// 200.
    pub const EXTRA_LIGHT: Self = Self(200);
    /// 300.
    pub const LIGHT: Self = Self(300);
    /// 400 — the weight a face has when a document says nothing.
    pub const REGULAR: Self = Self(400);
    /// 500.
    pub const MEDIUM: Self = Self(500);
    /// 600.
    pub const SEMI_BOLD: Self = Self(600);
    /// 700 — what `<b>` and `w:b` mean.
    pub const BOLD: Self = Self(700);
    /// 800.
    pub const EXTRA_BOLD: Self = Self(800);
    /// 900.
    pub const BLACK: Self = Self(900);
}

impl Default for FontWeight {
    fn default() -> Self {
        Self::REGULAR
    }
}

/// A face's horizontal proportions, as `OS/2.usWidthClass` names them.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Debug)]
pub enum FontWidth {
    /// 50% of normal.
    UltraCondensed,
    /// 62.5%.
    ExtraCondensed,
    /// 75%.
    Condensed,
    /// 87.5%.
    SemiCondensed,
    /// 100% — the width a face has when a document says nothing.
    #[default]
    Normal,
    /// 112.5%.
    SemiExpanded,
    /// 125%.
    Expanded,
    /// 150%.
    ExtraExpanded,
    /// 200%.
    UltraExpanded,
}

impl FontWidth {
    /// The width class as the 1–9 number `OS/2.usWidthClass` stores, which is also the distance
    /// metric the CSS font-matching rules step through.
    #[must_use]
    pub fn width_class(self) -> u8 {
        match self {
            Self::UltraCondensed => 1,
            Self::ExtraCondensed => 2,
            Self::Condensed => 3,
            Self::SemiCondensed => 4,
            Self::Normal => 5,
            Self::SemiExpanded => 6,
            Self::Expanded => 7,
            Self::ExtraExpanded => 8,
            Self::UltraExpanded => 9,
        }
    }
}

/// What a face calls itself.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FaceIdentity {
    /// The typographic family — `Liberation Sans`, `Carlito`.
    pub family: String,
    /// The style within the family — `Regular`, `Bold Italic`.
    pub subfamily: String,
    /// The PostScript name, unique across faces — `LiberationSans-Regular`.
    pub postscript_name: Option<String>,
    /// The face's weight.
    pub weight: FontWeight,
    /// The face's width class.
    pub width: FontWidth,
    /// Whether the face is upright, italic or oblique.
    pub slant: FontSlant,
}

/// A loaded face: its bytes, and the values that are read once per face.
///
/// Cheap to clone through an [`Arc`]; the bytes are never copied.
#[derive(Debug)]
pub struct FontFace {
    data: Arc<[u8]>,
    index: u32,
    metrics: FaceMetrics,
    identity: FaceIdentity,
    colour_formats: ColourGlyphFormats,
    variation_axes: Vec<VariationAxis>,
}

impl FontFace {
    /// How many faces a font file holds. A plain font holds one; a `.ttc`/`.otc` collection holds
    /// as many as its header says.
    #[must_use]
    pub fn face_count(data: &[u8]) -> u32 {
        ttf_parser::fonts_in_collection(data).unwrap_or(1)
    }

    /// Parse face `index` out of `data`.
    ///
    /// # Errors
    ///
    /// [`FontError::FaceIndexOutOfRange`] when the file holds fewer faces than that,
    /// [`FontError::MalformedFace`] when the bytes do not parse, and
    /// [`FontError::ImplausibleUnitsPerEm`] when the face declares an em square nothing can be
    /// scaled against. Never panics, whatever the bytes are.
    pub fn parse(data: Arc<[u8]>, index: u32) -> Result<Self, FontError> {
        let available = Self::face_count(&data);
        if index >= available {
            return Err(FontError::FaceIndexOutOfRange { index, available });
        }
        let parsed = ttf_parser::Face::parse(&data, index)?;

        let units_per_em = parsed.units_per_em();
        if !UNITS_PER_EM_RANGE.contains(&units_per_em) {
            return Err(FontError::ImplausibleUnitsPerEm { units_per_em });
        }

        let metrics = read_metrics(&parsed, units_per_em);
        let identity = read_identity(&parsed);
        let colour_formats = read_colour_formats(&parsed);
        let variation_axes = read_variation_axes(&parsed);

        Ok(Self {
            data,
            index,
            metrics,
            identity,
            colour_formats,
            variation_axes,
        })
    }

    /// The face's bytes, exactly as they were loaded.
    ///
    /// This is what R03's shaper and R04's rasteriser build their own views over, which is why this
    /// crate never hands out a `ttf_parser` type.
    #[must_use]
    pub fn data(&self) -> &Arc<[u8]> {
        &self.data
    }

    /// Which face within the file this is; 0 unless the file is a collection.
    #[must_use]
    pub fn index(&self) -> u32 {
        self.index
    }

    /// The face's metrics.
    #[must_use]
    pub fn metrics(&self) -> &FaceMetrics {
        &self.metrics
    }

    /// What the face calls itself.
    #[must_use]
    pub fn identity(&self) -> &FaceIdentity {
        &self.identity
    }

    /// The colour-glyph formats the face carries.
    #[must_use]
    pub fn colour_formats(&self) -> ColourGlyphFormats {
        self.colour_formats
    }

    /// The face's variation axes; empty for a static face.
    #[must_use]
    pub fn variation_axes(&self) -> &[VariationAxis] {
        &self.variation_axes
    }

    /// Open a reader over the face, paying the one parse, so that many glyph lookups cost one
    /// table-directory walk between them.
    ///
    /// # Errors
    ///
    /// The same failures as [`FontFace::parse`], which cannot in practice occur a second time on
    /// bytes that already parsed — but they are returned rather than unwrapped, because "cannot in
    /// practice" is not a proof and this is the untrusted-input path.
    pub fn reader(&self) -> Result<FaceReader<'_>, FontError> {
        let inner = ttf_parser::Face::parse(&self.data, self.index)?;
        let units_per_em = inner.units_per_em();
        if !UNITS_PER_EM_RANGE.contains(&units_per_em) {
            return Err(FontError::ImplausibleUnitsPerEm { units_per_em });
        }
        Ok(FaceReader {
            inner,
            units_per_em,
        })
    }
}

/// A parsed view over a [`FontFace`], for the lookups that happen once per glyph.
#[derive(Debug)]
pub struct FaceReader<'a> {
    inner: ttf_parser::Face<'a>,
    units_per_em: u16,
}

impl FaceReader<'_> {
    /// The face's em square.
    #[must_use]
    pub fn units_per_em(&self) -> u16 {
        self.units_per_em
    }

    /// The glyph the face maps `character` to, or `None` when it has no glyph for it.
    #[must_use]
    pub fn glyph_for_character(&self, character: char) -> Option<GlyphIndex> {
        self.inner.glyph_index(character).map(|id| GlyphIndex(id.0))
    }

    /// Whether the face has a glyph for `character`.
    #[must_use]
    pub fn covers(&self, character: char) -> bool {
        self.glyph_for_character(character).is_some()
    }

    /// How wide `glyph` is.
    #[must_use]
    pub fn advance(&self, glyph: GlyphIndex) -> Option<AdvanceWidth> {
        self.inner
            .glyph_hor_advance(ttf_parser::GlyphId(glyph.0))
            .map(|advance| AdvanceWidth {
                font_units: i32::from(advance),
                units_per_em: self.units_per_em,
            })
    }

    /// How wide the glyph for `character` is, or `None` when the face has no glyph for it.
    #[must_use]
    pub fn advance_for_character(&self, character: char) -> Option<AdvanceWidth> {
        self.glyph_for_character(character)
            .and_then(|glyph| self.advance(glyph))
    }

    /// The total advance of `text`, ignoring kerning and every other shaping effect.
    ///
    /// This is a *metrics* sum, not a shaped one: R03 owns shaping, and a shaped run's width can
    /// differ from this wherever the face applies kerning or a substitution. It is the right number
    /// for exactly one job — comparing two faces' widths for the same characters — which is what
    /// metric compatibility means.
    ///
    /// Returns `None` if the face has no glyph for one of the characters, because a sum that
    /// silently skipped a character would compare unequal things.
    #[must_use]
    pub fn unshaped_advance(&self, text: &str) -> Option<AdvanceWidth> {
        let mut total = 0_i32;
        for character in text.chars() {
            total = total.checked_add(self.advance_for_character(character)?.font_units)?;
        }
        Some(AdvanceWidth {
            font_units: total,
            units_per_em: self.units_per_em,
        })
    }

    /// The glyph's bounding box, or `None` for a glyph with no outline — a space, for instance.
    #[must_use]
    pub fn bounding_box(&self, glyph: GlyphIndex) -> Option<GlyphBounds> {
        self.inner
            .glyph_bounding_box(ttf_parser::GlyphId(glyph.0))
            .map(|rect| GlyphBounds {
                left: rect.x_min,
                bottom: rect.y_min,
                right: rect.x_max,
                top: rect.y_max,
            })
    }
}

fn read_metrics(parsed: &ttf_parser::Face<'_>, units_per_em: u16) -> FaceMetrics {
    let bounds = parsed.global_bounding_box();
    FaceMetrics {
        units_per_em,
        ascender: parsed.ascender(),
        descender: parsed.descender(),
        line_gap: parsed.line_gap(),
        typographic_ascender: parsed.typographic_ascender(),
        typographic_descender: parsed.typographic_descender(),
        typographic_line_gap: parsed.typographic_line_gap(),
        cap_height: parsed.capital_height(),
        x_height: parsed.x_height(),
        italic_angle: parsed.italic_angle(),
        underline: parsed
            .underline_metrics()
            .map(|line| LineDecorationMetrics {
                position: line.position,
                thickness: line.thickness,
            }),
        strikeout: parsed
            .strikeout_metrics()
            .map(|line| LineDecorationMetrics {
                position: line.position,
                thickness: line.thickness,
            }),
        global_bounds: GlyphBounds {
            left: bounds.x_min,
            bottom: bounds.y_min,
            right: bounds.x_max,
            top: bounds.y_max,
        },
        is_monospaced: parsed.is_monospaced(),
        glyph_count: parsed.number_of_glyphs(),
    }
}

fn read_identity(parsed: &ttf_parser::Face<'_>) -> FaceIdentity {
    // `name` IDs from the OpenType specification: 1 family, 2 subfamily, 6 PostScript name,
    // 16 typographic family, 17 typographic subfamily. The typographic pair is preferred where it
    // exists, because it is the one that names `Liberation Sans` rather than `Liberation Sans Bold`
    // for a bold face — grouping by the family-1 name would split a family across its weights.
    const FAMILY: u16 = 1;
    const SUBFAMILY: u16 = 2;
    const POSTSCRIPT: u16 = 6;
    const TYPOGRAPHIC_FAMILY: u16 = 16;
    const TYPOGRAPHIC_SUBFAMILY: u16 = 17;

    let name = |wanted: u16| -> Option<String> {
        parsed
            .names()
            .into_iter()
            .filter(|entry| entry.name_id == wanted && entry.is_unicode())
            .find_map(|entry| entry.to_string())
    };

    FaceIdentity {
        family: name(TYPOGRAPHIC_FAMILY)
            .or_else(|| name(FAMILY))
            .unwrap_or_default(),
        subfamily: name(TYPOGRAPHIC_SUBFAMILY)
            .or_else(|| name(SUBFAMILY))
            .unwrap_or_default(),
        postscript_name: name(POSTSCRIPT),
        weight: FontWeight(parsed.weight().to_number()),
        width: match parsed.width() {
            ttf_parser::Width::UltraCondensed => FontWidth::UltraCondensed,
            ttf_parser::Width::ExtraCondensed => FontWidth::ExtraCondensed,
            ttf_parser::Width::Condensed => FontWidth::Condensed,
            ttf_parser::Width::SemiCondensed => FontWidth::SemiCondensed,
            ttf_parser::Width::Normal => FontWidth::Normal,
            ttf_parser::Width::SemiExpanded => FontWidth::SemiExpanded,
            ttf_parser::Width::Expanded => FontWidth::Expanded,
            ttf_parser::Width::ExtraExpanded => FontWidth::ExtraExpanded,
            ttf_parser::Width::UltraExpanded => FontWidth::UltraExpanded,
        },
        slant: match parsed.style() {
            ttf_parser::Style::Normal => FontSlant::Upright,
            ttf_parser::Style::Italic => FontSlant::Italic,
            ttf_parser::Style::Oblique => FontSlant::Oblique,
        },
    }
}

fn read_colour_formats(parsed: &ttf_parser::Face<'_>) -> ColourGlyphFormats {
    let tables = parsed.tables();
    ColourGlyphFormats {
        // `ttf-parser` folds `CPAL` into its `COLR` table, because a `COLR` without the palette it
        // indexes into cannot be drawn; so one `Option` answers for the pair.
        layered_outlines: tables.colr.is_some(),
        apple_bitmaps: tables.sbix.is_some(),
        // The same folding applies to `CBLC`, which is the location table for `CBDT`'s strikes.
        bitmap_data: tables.cbdt.is_some(),
        scalable_vector_graphics: tables.svg.is_some(),
    }
}

fn read_variation_axes(parsed: &ttf_parser::Face<'_>) -> Vec<VariationAxis> {
    parsed
        .variation_axes()
        .into_iter()
        .map(|axis| VariationAxis {
            tag: axis.tag.to_bytes(),
            minimum: axis.min_value,
            default: axis.def_value,
            maximum: axis.max_value,
            is_hidden: axis.hidden,
        })
        .collect()
}
