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
}

impl From<ttf_parser::FaceParsingError> for FontError {
    fn from(source: ttf_parser::FaceParsingError) -> Self {
        Self::MalformedFace { source }
    }
}
