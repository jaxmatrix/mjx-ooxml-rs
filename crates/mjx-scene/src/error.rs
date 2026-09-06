//! Everything building or reading a display list can go wrong with.
//!
//! # Why every one of these is an error and not a panic
//!
//! A display list is **cacheable to disk** and **crosses a transport boundary**. That makes its
//! bytes untrusted in exactly the sense the rest of this workspace means it: a truncated cache file,
//! a byte flipped on the wire, a list written by an older version. Decoding one is therefore parsing
//! an untrusted input, and the project's standing rule applies — no `unwrap`, no `expect`, no
//! `panic!`, no slice index, on any path a caller can reach with bytes it did not write.
//!
//! `crates/mjx-scene/tests/a_malformed_list_errors.rs` holds that true by decoding lists that are
//! truncated at every length, versioned wrongly, sectioned wrongly and pointed out of range.

use crate::encoding::SectionKind;

/// What went wrong.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SceneError {
    /// The bytes are shorter than the fixed header, or than a structure the header describes.
    #[error("a display list needs {needed} bytes at offset {offset} and the blob has {available}")]
    Truncated {
        /// Where the read started.
        offset: usize,
        /// How many bytes it wanted.
        needed: usize,
        /// How many bytes there are in total.
        available: usize,
    },

    /// The first four bytes are not `MJXS`, so this is not a display list at all.
    #[error("a display list begins with the magic `MJXS`; these bytes begin with {found:?}")]
    NotADisplayList {
        /// The four bytes that were there instead.
        found: [u8; 4],
    },

    /// The list was written by a different version of this encoding.
    #[error("this display list is version {found}; this build reads version {supported}")]
    UnsupportedVersion {
        /// What the header says.
        found: u16,
        /// What this build understands.
        supported: u16,
    },

    /// The header is the right shape but says something impossible.
    #[error("the display list header is malformed: {reason}")]
    MalformedHeader {
        /// What about it.
        reason: &'static str,
    },

    /// A section's extent leaves the blob, overlaps its neighbour, or arrives out of order.
    #[error(
        "the `{section}` section is declared at offset {offset} for {length} bytes, which {reason}"
    )]
    MalformedSection {
        /// Which table.
        section: SectionKind,
        /// Where it claims to start.
        offset: usize,
        /// How long it claims to be.
        length: usize,
        /// Why that cannot be.
        reason: &'static str,
    },

    /// A fixed-stride table's length is not a whole number of records.
    #[error(
        "the `{section}` section is {length} bytes, which is not a whole number of {stride}-byte \
         records"
    )]
    RaggedSection {
        /// Which table.
        section: SectionKind,
        /// Its declared length.
        length: usize,
        /// The record size it must be a multiple of.
        stride: usize,
    },

    /// A command record carries an opcode this build has no case for.
    #[error("command {index} carries the unknown opcode {opcode}")]
    UnknownOpcode {
        /// Which command in the stream.
        index: usize,
        /// The byte that was there.
        opcode: u8,
    },

    /// A command record's declared length does not match its opcode, or is not a multiple of four.
    #[error("command {index} (opcode {opcode}) declares {length} bytes, which {reason}")]
    MalformedCommand {
        /// Which command in the stream.
        index: usize,
        /// Its opcode.
        opcode: u8,
        /// Its declared length.
        length: usize,
        /// Why that cannot be.
        reason: &'static str,
    },

    /// A record in a fixed-stride table names a kind, a preset or a mode this version does not
    /// define.
    ///
    /// Distinct from [`SceneError::MalformedSection`] on purpose: the section is exactly the right
    /// shape and exactly the right length, and one record inside it is from a vocabulary this build
    /// has not got. That is what a display list written by a *later* version looks like, and saying
    /// so names the record rather than condemning the table.
    #[error("entry {index} of the `{section}` table {reason}")]
    UnknownRecordKind {
        /// Which table.
        section: SectionKind,
        /// Which entry of it.
        index: u32,
        /// What it names that this build does not define.
        reason: &'static str,
    },

    /// A record names a resource the list does not contain.
    #[error("a record names entry {index} of the `{section}` table, which holds {count}")]
    ResourceOutOfRange {
        /// Which table was addressed.
        section: SectionKind,
        /// The index that was written.
        index: u32,
        /// How many entries the table really has.
        count: u32,
    },

    /// A geometry's path data is truncated, or carries an unknown step.
    #[error("the path at byte {offset} of the path-data section is malformed: {reason}")]
    MalformedPath {
        /// Where in the path-data section.
        offset: usize,
        /// What about it.
        reason: &'static str,
    },

    /// An effect consumes an effect that is not strictly before it in the table.
    ///
    /// The effect table is a DAG and is stored in topological order, so an input index below the
    /// node's own is the whole of the acyclicity check — and it is what stops a painter walking an
    /// effect chain from looping for ever on a list it did not write.
    #[error("effect {index} consumes effect {input}, which is not below it — the DAG has a cycle")]
    EffectCycle {
        /// The effect that names a bad input.
        index: u32,
        /// What it named.
        input: u32,
    },

    /// A `Pop` arrived with nothing pushed, or the stream ended with something still pushed.
    #[error("the command stream is unbalanced: {reason}")]
    UnbalancedStack {
        /// What about it.
        reason: &'static str,
    },

    /// A table would grow past the four billion entries an index can name, or the blob past the
    /// four gigabytes an offset can name.
    ///
    /// Returned rather than truncated: a display list that silently dropped its four billionth
    /// record would draw a page missing something, and the honest answer to a document that
    /// pathological is to say so.
    #[error("the `{section}` table is full at {count} entries; a display list cannot hold more")]
    TableFull {
        /// Which table.
        section: SectionKind,
        /// How many entries it already holds.
        count: usize,
    },

    /// An effect description names an input that is not one of the effects before it.
    #[error("effect {index} of a decoration names input {input}, which is not below it")]
    MalformedEffectChain {
        /// Which effect of the decoration.
        index: usize,
        /// What it named.
        input: usize,
    },

    /// A run of glyphs reached the builder with a fill that paints nothing.
    ///
    /// Unreachable while [`crate::DEFAULT_TEXT_COLOR`] stands: `text_paint` substitutes it for a
    /// resolver that says nothing, so the fill it interns is never [`crate::FillStyle::None`]. It is
    /// an error rather than an `expect` because this crate has no `expect`, and it is a *named*
    /// error rather than a silent skip because a page of invisible text is a defect a reader would
    /// report as "my document is blank" and nobody would find.
    ///
    /// It exists because removing the fallback used to be a **green mutation**: `text_paint` had a
    /// second fallback underneath the first, so deleting one changed nothing and no test noticed.
    /// One fallback, one error if it is ever removed.
    #[error(
        "a run of glyphs has no paint, which cannot happen while the default text colour stands"
    )]
    UnpaintableText,

    /// The text engine could not rasterise a glyph.
    #[error("preparing a glyph run: {0}")]
    Text(#[from] mjx_text::FontError),
}
