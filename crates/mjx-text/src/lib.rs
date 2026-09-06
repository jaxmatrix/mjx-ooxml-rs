//! Typography for the client platform: which face, and what are its numbers.
//!
//! This crate answers exactly two questions and no others. *Which face should this run be drawn
//! in?* — through three tiers and a substitution table. *What are that face's numbers?* — units per
//! em, ascent, descent, line gap, cap height, x-height, advance widths, glyph bounds, variation
//! axes and colour tables. Shaping, bidirectional reordering, itemisation and line breaking are
//! **R03**; rasterisation and the glyph atlas are **R04**; nothing here lays anything out.
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

pub mod compatibility;
pub mod embedded;
pub mod error;
pub mod face;
pub mod index;
pub mod manifest;
pub mod reference;
pub mod resolver;
pub mod substitution;

pub use compatibility::{verify_metric_compatibility, MetricCompatibility, UnverifiedReason};
pub use embedded::{EmbeddedFont, EmbeddedFontEncoding, ObfuscationKey};
pub use error::FontError;
pub use face::{
    AdvanceWidth, ColourGlyphFormats, FaceIdentity, FaceMetrics, FaceReader, FontFace, FontSlant,
    FontWeight, FontWidth, GlyphBounds, GlyphIndex, LineDecorationMetrics, VariationAxis,
};
pub use index::{FaceIndex, FontRequest, ResolutionTier};
pub use manifest::{SubstitutionManifest, SubstitutionRecord};
pub use reference::{
    reference_for_family, ReferenceAuthority, ReferenceMetrics, ReferenceVerticalMetrics,
    ADVANCE_TOLERANCE_PER_MILLE,
};
pub use resolver::{FontResolution, FontResolver, FontResolverBuilder, ResolvedFont};
pub use substitution::{
    fetchable_subset_for_character, substitution_for_family, FetchPlan, FetchableSubset,
    GenericFamily, SubstitutionRule, FETCHABLE_SUBSETS, SUBSTITUTION_TABLE,
};
