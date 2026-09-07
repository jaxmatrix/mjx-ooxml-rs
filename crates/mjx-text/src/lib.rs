//! Typography for the client platform: which face, what are its numbers, and what glyphs does this
//! text become.
//!
//! *Which face should this run be drawn in?* — through three tiers and a substitution table.
//! *What are that face's numbers?* — units per em, ascent, descent, line gap, cap height, x-height,
//! advance widths, glyph bounds, variation axes and colour tables. *What glyphs does this text
//! become, in what order, at what positions, and where may a line end?* — shaping, bidirectional
//! resolution, script and face itemisation, line breaking, grapheme segmentation and hyphenation.
//!
//! *And what does one of those glyphs look like at this zoom?* — rasterisation, scale bucketing,
//! quantised subpixel positioning, and a byte-bounded glyph atlas with a per-frame upload delta.
//!
//! **Nothing here lays anything out**: this crate says where a line *may* end and where a glyph sits
//! relative to the run's own origin, never where the line goes, how tall it is or what flows around
//! it. Those are the box model's, from R05 onward. It also draws nothing — the atlas produces
//! **bytes**, and putting them on a surface is R08's.
//!
//! # Why not Parley
//!
//! Parley is the obvious Rust text stack and it is deliberately **not** adopted. It is a *layout*
//! library: it owns line breaking, alignment and the arrangement of runs into lines, and it makes
//! those decisions the way a web engine does. This project's line and page decisions have to come
//! out where Office's do, and `docs/UI_PLATFORM_PLAN.md` §4 puts them in `mjx-layout` and the three
//! box models above it. What is needed from a text stack here is the **primitives** — shape this
//! run, resolve these levels, offer these break opportunities — and taking Parley would mean taking
//! a layout policy in order to reach them, then fighting it wherever it disagreed. See
//! [`mod@shaping`] for the same note where a shaper's author will meet it.
//!
//! # Why metric compatibility is a correctness requirement
//!
//! Fonts are the largest fidelity variable in the whole renderer. If a substituted face's advance
//! widths differ from the original's, every line breaks in a different place, and pagination
//! diverges from Office on page one rather than on page three hundred where it might be excusable.
//! So a substitution is not merely "find something that looks similar": the substitution table
//! ([`mod@substitution`]) is consulted before any blind fallback, and every substitution it makes
//! is **measured** against published metrics for the font it replaced ([`mod@reference`]) and the
//! verdict written into a per-document [`SubstitutionManifest`].
//!
//! The measurement is deliberately not self-referential. A face always agrees with itself, so a
//! table derived from Carlito and checked against Carlito proves nothing; every reference number
//! this crate ships is transcribed from a source outside the repository and carries the citation it
//! came from.
//!
//! # The tiers
//!
//! ```text
//! embedded in the document  →  installed on the device  →  bundled with the application  →  fetched
//! ```
//!
//! **The second of those can be empty, permanently, and that is a supported configuration.** iOS
//! does not expose system font files to a sandboxed process, and reaching them would need CoreText,
//! which is a C API this crate may not link. So the bundled tier is the first one iOS can reach,
//! which is why it is held to the metric-compatibility bar rather than a "looks similar" one.
//!
//! The fourth tier has **no transport in this loop**. A resolution that reaches it produces a
//! [`FetchPlan`] naming the subset that would cover the run, and the caller decides.
//!
//! # Nothing here has heard of OOXML
//!
//! There is no `.pptx` in this crate's vocabulary and no dependency on a format crate or on
//! `mjx-dml`. A document's font *reference* — `mjx_dml`'s model of `<a:latin>` and friends — is a
//! different thing from a font *engine*, and the two meet above this layer.
//!
//! # Nothing here panics on a font
//!
//! A font file inside a `.pptx` is exactly as untrusted as the `.pptx`. No `unwrap`, `expect`,
//! slice index or arithmetic overflow sits on any path that reads one; every failure is a
//! [`FontError`].

pub mod atlas;
pub mod cache;
pub mod compatibility;
pub mod direction;
pub mod embedded;
pub mod error;
pub mod face;
pub mod feature;
pub mod hyphenation;
pub mod index;
pub mod itemisation;
pub mod line_breaking;
pub mod manifest;
pub mod placement;
pub mod raster;
pub mod reference;
pub mod resolver;
pub mod script;
pub mod segmentation;
pub mod shaping;
pub mod subset;
pub mod substitution;

pub use atlas::{
    AtlasDelta, AtlasEntry, AtlasPageCreation, AtlasPageIndex, AtlasStatistics, AtlasUpload,
    GlyphAtlas, PreparedGlyph, PreparedImage, PreparedRun, ATLAS_GUTTER_PIXELS,
    ATLAS_PAGE_SIZE_PIXELS, DESKTOP_GLYPH_ATLAS_BYTE_CEILING, MOBILE_GLYPH_ATLAS_BYTE_CEILING,
};
pub use cache::{CacheStatistics, ShapedRunCache, DEFAULT_CACHE_CAPACITY};
pub use compatibility::{verify_metric_compatibility, MetricCompatibility, UnverifiedReason};
pub use direction::{
    BidiAnalysis, BidiLevel, DeclaredRunDirection, DirectionalRun, ParagraphDirection,
    TextDirection,
};
pub use embedded::{EmbeddedFont, EmbeddedFontEncoding, ObfuscationKey};
pub use error::FontError;
pub use face::{
    AdvanceWidth, ColourGlyphFormats, FaceIdentity, FaceMetrics, FaceReader, FontFace, FontSlant,
    FontWeight, FontWidth, GlyphBounds, GlyphIndex, LineDecorationMetrics, VariationAxis,
};
pub use feature::{
    FeatureSet, FeatureTag, FigureSpacing, FigureStyle, FontFeature, LigatureOptions,
    SmallCapitals, StylisticSets, TypographyOptions,
};
pub use hyphenation::{
    HyphenationPatterns, Hyphenator, NoHyphenation, PatternHyphenator, SoftHyphenHyphenator,
    SOFT_HYPHEN,
};
pub use index::{FaceIndex, FontRequest, ResolutionTier};
pub use itemisation::{itemise, TextItem};
pub use line_breaking::{
    break_opportunities, BreakKind, BreakOpportunity, KinsokuRules, LineBreak, LineBreakKind,
    LineBreakOptions, LineBreaker,
};
pub use manifest::{SubstitutionManifest, SubstitutionRecord};
pub use placement::{place_run, DeviceScale, PlacedGlyph, RunPlacement};
pub use raster::{
    BitmapFormat, FaceId, GlyphBitmap, GlyphOutline, GlyphRasterKey, GlyphRasteriser, GlyphRender,
    GlyphRoute, Hinting, OutlineCommand, OutlinePoint, RasterStatistics, ScaleBucket,
    SubpixelPosition, DEFAULT_OUTLINE_CACHE_CAPACITY, MAXIMUM_PIXELS_PER_EM,
    MAXIMUM_RASTERISED_PIXELS_PER_EM, OUTLINE_PIXELS_PER_EM_THRESHOLD,
    SCALE_BUCKET_STEP_PIXELS_PER_EM, SUBPIXEL_POSITION_COUNT,
};
pub use reference::{
    reference_for_family, ReferenceAuthority, ReferenceMetrics, ReferenceVerticalMetrics,
    ADVANCE_TOLERANCE_PER_MILLE,
};
pub use resolver::{FontResolution, FontResolver, FontResolverBuilder, ResolvedFont};
pub use script::{itemise_by_script, itemise_range_by_script, ScriptRun, TextScript};
pub use segmentation::{
    grapheme_cluster_boundaries, grapheme_cluster_count, grapheme_clusters, next_grapheme_boundary,
    previous_grapheme_boundary, word_at, word_segments, words,
};
pub use shaping::{shape_uncached, FontSize, ShapedGlyph, ShapedRun, Shaper, ShapingRequest};
pub use subset::{subset_truetype, FontSubset, MAXIMUM_GLYPHS};
pub use substitution::{
    fetchable_subset_for_character, substitution_for_family, FetchPlan, FetchableSubset,
    GenericFamily, SubstitutionRule, FETCHABLE_SUBSETS, SUBSTITUTION_TABLE,
};
