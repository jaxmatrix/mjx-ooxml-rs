//! The one error type this crate returns.
//!
//! A font file is untrusted input in exactly the way a document is: a `.pptx` may carry an embedded
//! face, a system font directory is whatever the platform put there, and a fetched subset arrives
//! over a network. So nothing on the parse path may `unwrap`, `expect` or index into a slice it has
//! not bounds-checked; every failure comes back as one of these.

use std::path::PathBuf;

/// Everything that can go wrong loading or reading a font face.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FontError {
    /// The bytes are not a TrueType, OpenType or font-collection file that can be read.
    #[error("the font data is not a face this parser can read: {source}")]
    MalformedFace {
        /// What `ttf-parser` made of it.
        #[source]
        source: ttf_parser::FaceParsingError,
    },

    /// A face index was asked for that the file does not contain. A plain (non-collection) font
    /// holds exactly one face, at index 0.
    #[error("the font file has no face at index {index}; it holds {available}")]
    FaceIndexOutOfRange {
        /// The index that was asked for.
        index: u32,
        /// How many faces the file actually holds.
        available: u32,
    },

    /// `head.unitsPerEm` is outside the range the OpenType specification permits (16 to 16384). A
    /// face that reports something else cannot be scaled, so every metric derived from it would be
    /// nonsense — better to refuse it than to divide by it.
    #[error(
        "the face declares {units_per_em} units per em, and the OpenType specification permits \
         only 16 to 16384"
    )]
    ImplausibleUnitsPerEm {
        /// What the face declared.
        units_per_em: u16,
    },

    /// A file could not be read.
    #[error("reading the font file `{path}`: {source}")]
    UnreadableFile {
        /// The path that failed.
        path: PathBuf,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },

    /// A directory of faces could not be walked.
    #[error("scanning the font directory `{path}`: {source}")]
    UnreadableDirectory {
        /// The directory that failed.
        path: PathBuf,
        /// The underlying I/O failure.
        #[source]
        source: std::io::Error,
    },

    /// The obfuscation key on an embedded font is not a GUID, so the mask cannot be derived from
    /// it. See [`crate::ObfuscationKey`].
    #[error(
        "`{key}` is not a font obfuscation key: one is a GUID of 32 hexadecimal digits, optionally \
         braced and hyphenated"
    )]
    MalformedObfuscationKey {
        /// The key as it was written in the document.
        key: String,
    },

    /// An obfuscated font is shorter than the 32-byte header the key masks, so de-obfuscating it
    /// would read past its end.
    #[error(
        "the obfuscated font is {length} bytes long, and the obfuscation scheme masks the first 32"
    )]
    ObfuscatedFontTooShort {
        /// How many bytes actually arrived.
        length: usize,
    },

    /// A shaped run's advances sum past what an [`crate::AdvanceWidth`] can carry.
    ///
    /// The sum is accumulated in `i64` and checked, rather than wrapping in `i32`: a run of
    /// untrusted text can be arbitrarily long, and a width that silently became negative would put
    /// a line break in a place no measure explains. It takes roughly a million ems in one run,
    /// which no document produces and a fuzzer produces immediately.
    #[error(
        "the shaped run's advance is {font_units} font units across {characters} characters, which \
         does not fit the 32-bit advance a width carries"
    )]
    ShapedRunTooWide {
        /// The sum that did not fit.
        font_units: i64,
        /// How many characters were in the run.
        characters: usize,
    },

    /// A raster key named a face the rasteriser it was given to has never been shown.
    ///
    /// A [`crate::FaceId`] is issued by one [`crate::GlyphRasteriser`] and means nothing to another,
    /// so this is what a key crossing between two of them comes back as, rather than silently
    /// drawing whichever face happened to be registered at that number.
    #[error("no face is registered as {face} in this rasteriser")]
    UnregisteredFace {
        /// The identity the key carried.
        face: u32,
    },

    /// More than four billion faces were registered with one rasteriser.
    ///
    /// Unreachable by a document and reachable by a loop that registers a fresh face per call; it is
    /// returned rather than wrapped, because a wrapped identity would silently rasterise the wrong
    /// face.
    #[error(
        "this rasteriser already holds {registered} faces, which is all a face identity can name"
    )]
    TooManyRegisteredFaces {
        /// How many were already registered.
        registered: usize,
    },

    /// A glyph was asked for at a size whose bitmap would be too large to be worth allocating.
    ///
    /// Only a colour glyph can reach this: every other glyph becomes an outline above
    /// [`crate::OUTLINE_PIXELS_PER_EM_THRESHOLD`], long before the size here. A colour glyph has no
    /// outline to fall back to, so the request is refused before anything allocates for it — at 4096
    /// pixels to the em one such glyph would be sixty-four megabytes.
    #[error(
        "a colour glyph was asked for at {pixels_per_em} pixels per em, and this rasteriser \
         rasterises at most {maximum}"
    )]
    GlyphTooLargeToRasterise {
        /// The size that was asked for.
        pixels_per_em: f32,
        /// The largest this rasteriser will produce.
        maximum: f32,
    },

    /// A glyph's outline could not be read out of the face, because reading it made the underlying
    /// parser panic.
    ///
    /// This is a **defect in a dependency**, caught rather than propagated. `swash` reads fonts
    /// through `skrifa`, whose pinned `read-fonts 0.41.0` indexes a zero-length slice when a `glyf`
    /// entry's `endPtsOfContours` wraps its point count to zero; one flipped byte in an embedded
    /// font reaches it. `crates/mjx-text/src/raster.rs`'s `read_a_glyph_table` has the whole account,
    /// including where the fix landed upstream and why it is out of reach.
    ///
    /// A caller drawing a document may treat this as "draw nothing here". It is an error rather than
    /// a blank because the face really is broken, and a renderer that silently drew nothing would
    /// give the reader no way to find out.
    #[error(
        "the face's outline for glyph {glyph} at {pixels_per_em} pixels per em could not be read: \
         the glyph tables are malformed in a way the underlying parser does not survive"
    )]
    UnreadableGlyphOutline {
        /// Which glyph in the face.
        glyph: u16,
        /// The size it was asked for at.
        pixels_per_em: f32,
    },

    /// A glyph's bitmap will not fit an empty atlas page, so no amount of eviction would help.
    #[error(
        "a {width}x{height} pixel glyph cannot be packed into a {page_size}x{page_size} pixel atlas \
         page"
    )]
    GlyphTooLargeForAtlas {
        /// How wide the bitmap is.
        width: u16,
        /// How tall the bitmap is.
        height: u16,
        /// How wide and tall a page is.
        page_size: u16,
    },

    /// The atlas needs another page and cannot have one: it is at its byte ceiling and every live
    /// page holds a glyph the frame being drawn has already used.
    ///
    /// **The atlas will not evict the frame it is in the middle of**, because a cache that threw its
    /// working set away would satisfy every byte bound and draw nothing. So this is the honest
    /// answer, and a caller's recovery is to draw the glyph without the atlas, or to raise the
    /// ceiling.
    #[error(
        "the glyph atlas holds {resident_bytes} bytes against a ceiling of {ceiling_bytes}, and \
         every page it could drop holds a glyph this frame has already drawn"
    )]
    GlyphAtlasExhausted {
        /// What the live pages currently cost.
        resident_bytes: usize,
        /// The ceiling they are held under.
        ceiling_bytes: usize,
    },

    /// A font subset was asked for more glyph ids than a font file can hold.
    ///
    /// A glyph id is a `u16`, so this is the format's own ceiling rather than a policy: a request
    /// naming more than this is a caller that has lost count, and answering it would produce a file
    /// whose `maxp` disagrees with its `loca`.
    #[error(
        "a font subset was asked for {requested} glyphs; a font file addresses at most {maximum}"
    )]
    SubsetTooLarge {
        /// How many were asked for.
        requested: usize,
        /// How many a font file can address.
        maximum: usize,
    },
}

impl From<ttf_parser::FaceParsingError> for FontError {
    fn from(source: ttf_parser::FaceParsingError) -> Self {
        Self::MalformedFace { source }
    }
}
