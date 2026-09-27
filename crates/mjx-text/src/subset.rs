//! Cutting a face down to the glyphs a document actually used.
//!
//! # Why the font engine owns this and an exporter does not
//!
//! A PDF that embeds a face embeds a *font file*, and the whole face is the wrong file: a page with
//! forty letters on it does not need Liberation Sans's two and a half thousand glyphs, and a
//! twenty-slide deck that embedded them once per face would be several megabytes of glyphs nobody
//! can see. So an exporter subsets.
//!
//! But subsetting is **table surgery on untrusted bytes** — `glyf`, `loca`, `hmtx`, `head`, `hhea`,
//! `maxp` — and MJXOFF-164 is explicit that a painter may not grow a second font reader.
//! `CLAUDE.md` puts the same rule the other way round: `mjx-text` *is* the font engine, and "faces,
//! metrics, the three resolution tiers" is a description of who reads font tables, not a list. So
//! this lives here, one crate below every painter, and the PDF exporter calls it.
//!
//! # Truncating rather than renumbering, and why that is the safer subset
//!
//! The textbook subsetter renumbers: it keeps *n* glyphs, gives them ids `0..n`, and rewrites every
//! composite glyph's component ids to match. That is the smallest possible file and it is also
//! where every subsetter bug lives — a composite whose components were renumbered inconsistently
//! draws an accent over the wrong letter, and nothing about the file is invalid.
//!
//! This one **keeps every glyph's own id** and truncates the tables to the highest id it needs:
//!
//! * `glyf` holds the kept glyphs' bytes verbatim and nothing for the rest, so a composite's
//!   component ids are still correct **because they were never touched**;
//! * `loca` gives a dropped glyph a zero-length entry, which is exactly how the specification spells
//!   *"this glyph has no outline"*;
//! * `maxp.numGlyphs`, `loca` and `hmtx` stop one past the highest id that is kept.
//!
//! What it costs is `loca` and `hmtx` for the glyphs below the highest kept id — eight bytes each,
//! and only up to that id rather than to the end of the face. Liberation Sans's Latin letters sit
//! low in the face, so a page of English text keeps a few hundred entries rather than two and a half
//! thousand, and the outlines — which are the bulk — are only the ones that were used. What it buys
//! is that **the one class of bug a subsetter has is structurally absent**.
//!
//! # What is kept, and what is deliberately dropped
//!
//! Kept: `head`, `hhea`, `maxp`, `hmtx`, `loca`, `glyf`, and `cvt `/`fpgm`/`prep` verbatim, because
//! dropping the hinting programs while keeping hinted outlines produces a face that renders
//! differently from the one the document was laid out against.
//!
//! Dropped: `cmap`, `name`, `post`, `OS/2`, and everything else. A PDF `CIDFontType2` with
//! `/CIDToGIDMap /Identity` addresses glyphs by id and never consults a `cmap`; the text a reader
//! extracts comes from the `/ToUnicode` map, which the exporter writes from
//! [`crate::FaceReader::for_each_mapped_character`] and which is a PDF structure rather than a font
//! table. Carrying `post` would be actively wrong — its format 2 glyph-name array is indexed by
//! glyph id and copying it verbatim beside truncated tables is a file that disagrees with itself.
//!
//! # `CFF` is not subsetted, and says so
//!
//! A face whose outlines are in a `CFF` table has no `glyf` to cut, and cutting a `CFF` charstring
//! index is a different algorithm with its own subroutine-renumbering problem. [`subset_truetype`]
//! answers [`FontSubset::whole_face`] for one, which embeds correctly and is merely larger. An
//! exporter that silently embedded nothing would produce a PDF whose text is invisible; one that
//! failed would refuse to export a document that Office exports fine.

use std::collections::BTreeSet;

use crate::error::FontError;
use crate::face::{FontFace, GlyphIndex};

/// How many glyphs a subset may name before it is refused.
///
/// The format's own ceiling: a glyph id is a `u16`, so a face cannot hold more than this and a
/// request naming more than this is a caller that has lost track rather than a real page.
pub const MAXIMUM_GLYPHS: usize = 0x1_0000;

/// A face cut down to a glyph set, and what it can still draw.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FontSubset {
    data: Vec<u8>,
    glyph_count: u16,
    kept: Vec<u16>,
    whole_face: bool,
}

impl FontSubset {
    /// The bytes to embed.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// How many glyph ids the subset's tables cover — one past the highest id it kept.
    ///
    /// This is the `numGlyphs` an embedder writes into a PDF's `/CIDCount`, and it is **not** the
    /// number of glyphs that have outlines: the ids below the highest kept one are present and
    /// empty. See this module's documentation for why that is the trade.
    #[must_use]
    pub fn glyph_count(&self) -> u16 {
        self.glyph_count
    }

    /// The glyph ids that kept their outlines, ascending, glyph zero included.
    #[must_use]
    pub fn kept_glyphs(&self) -> &[u16] {
        &self.kept
    }

    /// Whether the whole face was embedded because it could not be cut.
    ///
    /// True for a `CFF` face, and for one whose `glyf`/`loca` tables could not be read. An exporter
    /// may report it; it must not treat it as a failure, because the file it gets back is correct
    /// either way.
    #[must_use]
    pub fn whole_face(&self) -> bool {
        self.whole_face
    }
}

/// Cut `face` down to `glyphs` and everything they need.
///
/// The answer always draws every glyph in `glyphs`: composite glyphs pull their components in
/// transitively, and glyph zero — the notdef box a reader falls back to — is always kept.
///
/// # Errors
///
/// [`FontError::MalformedFace`] if the face's own bytes stop parsing, which they cannot in practice
/// having already parsed once, and [`FontError::SubsetTooLarge`] for a request naming more glyph ids
/// than a font file can hold.
pub fn subset_truetype(face: &FontFace, glyphs: &[GlyphIndex]) -> Result<FontSubset, FontError> {
    if glyphs.len() > MAXIMUM_GLYPHS {
        return Err(FontError::SubsetTooLarge {
            requested: glyphs.len(),
            maximum: MAXIMUM_GLYPHS,
        });
    }

    let data = face.data().as_ref();
    let parsed = ttf_parser::Face::parse(data, face.index())?;
    let raw = parsed.raw_face();

    let whole = |reason: bool| FontSubset {
        data: data.to_vec(),
        // The face's own count, which is what a reader will find in the `maxp` it is handed.
        glyph_count: parsed.number_of_glyphs(),
        kept: Vec::new(),
        whole_face: reason,
    };

    let (Some(glyf), Some(loca_bytes), Some(head), Some(hhea), Some(maxp)) = (
        raw.table(tag(b"glyf")),
        raw.table(tag(b"loca")),
        raw.table(tag(b"head")),
        raw.table(tag(b"hhea")),
        raw.table(tag(b"maxp")),
    ) else {
        // A `CFF` face, or one whose tables this cannot cut. Embedding it whole is correct and
        // merely larger; see the module documentation.
        return Ok(whole(true));
    };

    let long_loca = read_u16(head, 50).is_some_and(|format| format == 1);
    let original_count = parsed.number_of_glyphs();
    let Some(offsets) = read_loca(loca_bytes, original_count, long_loca) else {
        return Ok(whole(true));
    };

    // Glyph zero is the notdef box, and a face without it is a face a reader cannot fall back
    // through. Every subset keeps it whatever was asked for.
    let mut kept: BTreeSet<u16> = BTreeSet::new();
    kept.insert(0);
    let mut pending: Vec<u16> = vec![0];
    for glyph in glyphs {
        if glyph.0 < original_count && kept.insert(glyph.0) {
            pending.push(glyph.0);
        }
    }
    // A composite glyph is a list of other glyphs' ids, and dropping one of those leaves an accent
    // with nothing under it. The walk is transitive because a composite may name a composite.
    while let Some(glyph) = pending.pop() {
        for component in composite_components(glyf, &offsets, glyph) {
            if component < original_count && kept.insert(component) {
                pending.push(component);
            }
        }
    }

    let Some(highest) = kept.iter().next_back().copied() else {
        return Ok(whole(true));
    };
    let new_count = highest.saturating_add(1);

    // `glyf` and `loca` together. A dropped glyph gets a zero-length entry, which is how the
    // specification spells "this glyph has no outline".
    let mut new_glyf: Vec<u8> = Vec::new();
    let mut new_loca: Vec<u8> = Vec::with_capacity((usize::from(new_count) + 1) * 4);
    for glyph in 0..new_count {
        new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
        if !kept.contains(&glyph) {
            continue;
        }
        let (start, end) = (
            offsets.get(usize::from(glyph)).copied().unwrap_or(0) as usize,
            offsets.get(usize::from(glyph) + 1).copied().unwrap_or(0) as usize,
        );
        if end <= start {
            continue;
        }
        let Some(bytes) = glyf.get(start..end) else {
            continue;
        };
        new_glyf.extend_from_slice(bytes);
        // Long-format `loca` offsets need no alignment, but every real face aligns its glyphs and
        // some rasterisers assume it. Four bytes costs at most three per glyph.
        while !new_glyf.len().is_multiple_of(4) {
            new_glyf.push(0);
        }
    }
    new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());

    // `head`, with the loca format forced long — the offsets above are `u32` — and the checksum
    // adjustment zeroed, because it is a checksum of a file that no longer exists.
    let mut new_head = head.to_vec();
    write_u32(&mut new_head, 8, 0);
    write_u16(&mut new_head, 50, 1);

    // `hmtx`: one full record per glyph the subset covers, so `numberOfHMetrics` is the whole count
    // and there is no trailing left-side-bearing array to get wrong.
    let original_metrics = read_u16(hhea, 34).unwrap_or(0);
    let hmtx = raw.table(tag(b"hmtx")).unwrap_or(&[]);
    let mut new_hmtx = Vec::with_capacity(usize::from(new_count) * 4);
    for glyph in 0..new_count {
        let (advance, bearing) = horizontal_metric(hmtx, original_metrics, glyph);
        new_hmtx.extend_from_slice(&advance.to_be_bytes());
        new_hmtx.extend_from_slice(&bearing.to_be_bytes());
    }
    let mut new_hhea = hhea.to_vec();
    write_u16(&mut new_hhea, 34, new_count);

    let mut new_maxp = maxp.to_vec();
    write_u16(&mut new_maxp, 4, new_count);

    let mut tables: Vec<([u8; 4], Vec<u8>)> = vec![
        (*b"head", new_head),
        (*b"hhea", new_hhea),
        (*b"maxp", new_maxp),
        (*b"hmtx", new_hmtx),
        (*b"loca", new_loca),
        (*b"glyf", new_glyf),
    ];
    // The hinting programs, verbatim. A hinted outline without them grid-fits differently from the
    // face the document was laid out against, which is a fidelity difference rather than a size
    // saving.
    for name in [b"cvt ", b"fpgm", b"prep"] {
        if let Some(bytes) = raw.table(tag(name)) {
            tables.push((*name, bytes.to_vec()));
        }
    }

    Ok(FontSubset {
        data: assemble(&mut tables),
        glyph_count: new_count,
        kept: kept.into_iter().collect(),
        whole_face: false,
    })
}

/// A four-byte table tag.
fn tag(name: &[u8; 4]) -> ttf_parser::Tag {
    ttf_parser::Tag::from_bytes(name)
}

fn read_u16(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]))
}

fn write_u16(bytes: &mut [u8], at: usize, value: u16) {
    let encoded = value.to_be_bytes();
    for (index, byte) in encoded.iter().enumerate() {
        if let Some(slot) = bytes.get_mut(at + index) {
            *slot = *byte;
        }
    }
}

fn write_u32(bytes: &mut [u8], at: usize, value: u32) {
    let encoded = value.to_be_bytes();
    for (index, byte) in encoded.iter().enumerate() {
        if let Some(slot) = bytes.get_mut(at + index) {
            *slot = *byte;
        }
    }
}

/// The `loca` table as `numGlyphs + 1` byte offsets into `glyf`.
fn read_loca(bytes: &[u8], count: u16, long: bool) -> Option<Vec<u32>> {
    let entries = usize::from(count).checked_add(1)?;
    let mut offsets = Vec::with_capacity(entries);
    for index in 0..entries {
        let offset = if long {
            let at = index.checked_mul(4)?;
            u32::from_be_bytes([
                *bytes.get(at)?,
                *bytes.get(at + 1)?,
                *bytes.get(at + 2)?,
                *bytes.get(at + 3)?,
            ])
        } else {
            // The short format stores half the offset, which is why every glyph in such a face is
            // two-byte aligned.
            let at = index.checked_mul(2)?;
            u32::from(u16::from_be_bytes([*bytes.get(at)?, *bytes.get(at + 1)?])) * 2
        };
        offsets.push(offset);
    }
    Some(offsets)
}

/// One glyph's advance and left side bearing, out of a raw `hmtx`.
///
/// A face stores full records for the first `metrics` glyphs and only bearings after that, because
/// the tail of a face is usually monospaced accents; a glyph past the records takes the last
/// record's advance, which is what the specification says and what every reader does.
fn horizontal_metric(hmtx: &[u8], metrics: u16, glyph: u16) -> (u16, i16) {
    let last = metrics.saturating_sub(1);
    let record = glyph.min(last);
    let at = usize::from(record) * 4;
    let advance = read_u16(hmtx, at).unwrap_or(0);
    let bearing = if glyph < metrics {
        read_u16(hmtx, at + 2).unwrap_or(0) as i16
    } else {
        let tail = usize::from(metrics) * 4 + usize::from(glyph - metrics) * 2;
        read_u16(hmtx, tail).unwrap_or(0) as i16
    };
    (advance, bearing)
}

/// The glyph ids a composite glyph is built out of; empty for a simple one.
fn composite_components(glyf: &[u8], offsets: &[u32], glyph: u16) -> Vec<u16> {
    let (Some(start), Some(end)) = (
        offsets.get(usize::from(glyph)).copied(),
        offsets.get(usize::from(glyph) + 1).copied(),
    ) else {
        return Vec::new();
    };
    let (start, end) = (start as usize, end as usize);
    if end <= start {
        return Vec::new();
    }
    let Some(entry) = glyf.get(start..end) else {
        return Vec::new();
    };
    // A negative contour count is the specification's marker for a composite.
    let contours = read_u16(entry, 0).unwrap_or(0) as i16;
    if contours >= 0 {
        return Vec::new();
    }

    const ARGUMENTS_ARE_WORDS: u16 = 0x0001;
    const WE_HAVE_A_SCALE: u16 = 0x0008;
    const MORE_COMPONENTS: u16 = 0x0020;
    const AN_X_AND_Y_SCALE: u16 = 0x0040;
    const A_TWO_BY_TWO: u16 = 0x0080;

    let mut components = Vec::new();
    // Past the ten-byte glyph header: contour count and the bounding box.
    let mut at = 10usize;
    while let (Some(flags), Some(index)) = (read_u16(entry, at), read_u16(entry, at + 2)) {
        components.push(index);
        at += 4;
        at += if flags & ARGUMENTS_ARE_WORDS != 0 {
            4
        } else {
            2
        };
        at += if flags & WE_HAVE_A_SCALE != 0 {
            2
        } else if flags & AN_X_AND_Y_SCALE != 0 {
            4
        } else if flags & A_TWO_BY_TWO != 0 {
            8
        } else {
            0
        };
        if flags & MORE_COMPONENTS == 0 || at >= entry.len() {
            break;
        }
    }
    components
}

/// The sum of a table's bytes as big-endian `u32` words, zero-padded — the sfnt checksum.
fn checksum(bytes: &[u8]) -> u32 {
    let mut total = 0u32;
    for chunk in bytes.chunks(4) {
        let mut word = [0u8; 4];
        for (slot, byte) in word.iter_mut().zip(chunk.iter()) {
            *slot = *byte;
        }
        total = total.wrapping_add(u32::from_be_bytes(word));
    }
    total
}

/// The table directory and the tables, as one sfnt file.
///
/// Tables are written in tag order, which the specification requires of the *directory*; writing the
/// bodies in the same order costs nothing and makes two subsets of the same face byte-identical,
/// which is what lets an export be compared against itself.
fn assemble(tables: &mut [([u8; 4], Vec<u8>)]) -> Vec<u8> {
    tables.sort_by_key(|(name, _)| *name);
    let count = tables.len() as u16;
    // `searchRange`, `entrySelector` and `rangeShift` are a binary search's precomputed bounds. No
    // reader in practice uses them and every reader checks them, so they are computed rather than
    // zeroed.
    let mut power = 1u16;
    let mut selector = 0u16;
    while power * 2 <= count {
        power *= 2;
        selector += 1;
    }
    let search_range = power.saturating_mul(16);

    let mut file = Vec::new();
    file.extend_from_slice(&0x0001_0000u32.to_be_bytes());
    file.extend_from_slice(&count.to_be_bytes());
    file.extend_from_slice(&search_range.to_be_bytes());
    file.extend_from_slice(&selector.to_be_bytes());
    file.extend_from_slice(
        &count
            .saturating_mul(16)
            .saturating_sub(search_range)
            .to_be_bytes(),
    );

    let mut offset = 12 + usize::from(count) * 16;
    let mut records = Vec::new();
    for (name, body) in tables.iter() {
        records.push((*name, checksum(body), offset as u32, body.len() as u32));
        offset += body.len();
        offset += (4 - body.len() % 4) % 4;
    }
    for (name, sum, at, length) in &records {
        file.extend_from_slice(name);
        file.extend_from_slice(&sum.to_be_bytes());
        file.extend_from_slice(&at.to_be_bytes());
        file.extend_from_slice(&length.to_be_bytes());
    }
    for (_, body) in tables.iter() {
        file.extend_from_slice(body);
        while !file.len().is_multiple_of(4) {
            file.push(0);
        }
    }
    file
}
