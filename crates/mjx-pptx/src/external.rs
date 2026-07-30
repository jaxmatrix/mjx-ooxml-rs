//! Neutralizing inaccessible external sources.
//!
//! An OOXML element can bind its content through a *linked* (external) relationship — a picture that
//! links its image, a chart that links its workbook, a linked OLE object or media file. When that
//! target is unreachable on the current platform, a consumer can fail on it. This module provides the
//! caller-driven tools to replace such a source with an in-package placeholder of the same kind, so
//! the presentation stands on its own. The library performs no external I/O and cannot judge
//! reachability itself — the caller decides which references to neutralize.
//!
//! P1 covers linked *images*; later phases extend the same idea to the workbook behind a chart, OLE
//! objects, and media, composing [`mjx_opc::Package::retarget_relationship`] for the element kinds the
//! library does not model.

/// The built-in placeholder image used when a caller does not supply their own — a tiny valid PNG,
/// embedded so a neutralized picture always resolves inside the package. Callers who want recognizable
/// artwork pass their own bytes instead.
///
/// A valid 2×2 truecolour PNG (76 bytes).
pub const DEFAULT_PLACEHOLDER_IMAGE: &[u8] = &[
    0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x02, 0x08, 0x02, 0x00, 0x00, 0x00, 0xFD, 0xD4, 0x9A,
    0x73, 0x00, 0x00, 0x00, 0x13, 0x49, 0x44, 0x41, 0x54, 0x78, 0xDA, 0x63, 0x78, 0x60, 0x60, 0x60,
    0x90, 0xF0, 0x80, 0x01, 0x88, 0x81, 0x2C, 0x00, 0x25, 0xAE, 0x05, 0x61, 0x56, 0x69, 0x41, 0x72,
    0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
];

/// A picture that *links* its image (`a:blip@r:link`) rather than embedding it — a candidate for
/// [`Presentation::replace_linked_image_with_placeholder`](crate::Presentation::replace_linked_image_with_placeholder),
/// as reported by [`Presentation::linked_images`](crate::Presentation::linked_images).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkedImage {
    /// The shape index of the picture on the surface it was found on.
    pub shape_index: usize,
    /// Where the image is linked from — the relationship target (an external path/URL, or an
    /// in-package part target for an internal link).
    pub target: String,
}

/// A chart frame that references a backing workbook (`c:externalData`) — a candidate for
/// [`Presentation::detach_chart_workbook`](crate::Presentation::detach_chart_workbook), as reported by
/// [`Presentation::chart_workbooks`](crate::Presentation::chart_workbooks). A chart renders from its
/// cached data, so detaching an inaccessible workbook leaves the chart intact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChartWorkbook {
    /// The shape index of the chart frame on the surface it was found on.
    pub shape_index: usize,
    /// Where the workbook is referenced from — the relationship target (an external path/URL, or an
    /// in-package part target for an embedded workbook).
    pub target: String,
    /// Whether that relationship is external (`TargetMode="External"`), i.e. the case that can be
    /// unreachable on another platform. An embedded workbook is `false`.
    pub external: bool,
}

/// An OLE object frame (`p:oleObj`) and the object data it references — a candidate for
/// [`Presentation::replace_ole_object_with_placeholder`](crate::Presentation::replace_ole_object_with_placeholder),
/// as reported by [`Presentation::ole_objects`](crate::Presentation::ole_objects). An OLE object is
/// displayed via its snapshot image, so replacing an unreachable object with a placeholder leaves the
/// slide looking the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OleObject {
    /// The shape index of the OLE frame on the surface it was found on.
    pub shape_index: usize,
    /// Where the object data is referenced from — the relationship target (an external path/URL, or an
    /// in-package part target for an embedded object).
    pub target: String,
    /// Whether that relationship is external (`TargetMode="External"`), i.e. the case that can be
    /// unreachable on another platform. An embedded object is `false`.
    pub external: bool,
    /// The object's program id (`p:oleObj@progId`, e.g. `"Excel.Sheet.12"`), if present.
    pub prog_id: Option<String>,
}

/// The built-in placeholder for an OLE object's embedded data — a minimal but structurally valid
/// [MS-CFB] compound file (an empty root storage). Used when a caller neutralizing an inaccessible OLE
/// object supplies no bytes of their own. An OLE object renders from its snapshot image and its data
/// stream is read only on activation, so an empty-but-valid container is a faithful inert stand-in.
///
/// The file is exactly three 512-byte sectors (1536 bytes): the header, one FAT sector, and one
/// directory sector holding a single "Root Entry" root storage with no children and no mini stream.
#[must_use]
pub fn default_placeholder_ole() -> Vec<u8> {
    // MS-CFB special FAT/stream sector values.
    const FREESECT: u32 = 0xFFFF_FFFF;
    const ENDOFCHAIN: u32 = 0xFFFF_FFFE;
    const FATSECT: u32 = 0xFFFF_FFFD;
    const NOSTREAM: u32 = 0xFFFF_FFFF;

    let mut out = Vec::with_capacity(3 * 512);

    // --- Header (512 bytes) ---
    out.extend_from_slice(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]); // signature
    out.extend_from_slice(&[0u8; 16]); // CLSID (unused)
    out.extend_from_slice(&0x003Eu16.to_le_bytes()); // minor version
    out.extend_from_slice(&0x0003u16.to_le_bytes()); // major version (3 → 512-byte sectors)
    out.extend_from_slice(&0xFFFEu16.to_le_bytes()); // byte order (little-endian)
    out.extend_from_slice(&0x0009u16.to_le_bytes()); // sector shift (2^9 = 512)
    out.extend_from_slice(&0x0006u16.to_le_bytes()); // mini sector shift (2^6 = 64)
    out.extend_from_slice(&[0u8; 6]); // reserved
    out.extend_from_slice(&0u32.to_le_bytes()); // number of directory sectors (0 for v3)
    out.extend_from_slice(&1u32.to_le_bytes()); // number of FAT sectors
    out.extend_from_slice(&1u32.to_le_bytes()); // first directory sector location (sector 1)
    out.extend_from_slice(&0u32.to_le_bytes()); // transaction signature
    out.extend_from_slice(&0x0000_1000u32.to_le_bytes()); // mini stream cutoff size
    out.extend_from_slice(&ENDOFCHAIN.to_le_bytes()); // first mini FAT sector (none)
    out.extend_from_slice(&0u32.to_le_bytes()); // number of mini FAT sectors
    out.extend_from_slice(&ENDOFCHAIN.to_le_bytes()); // first DIFAT sector (none)
    out.extend_from_slice(&0u32.to_le_bytes()); // number of DIFAT sectors
    out.extend_from_slice(&0u32.to_le_bytes()); // DIFAT[0] → the FAT sector is sector 0
    for _ in 1..109 {
        out.extend_from_slice(&FREESECT.to_le_bytes()); // DIFAT[1..109] unused
    }
    debug_assert_eq!(out.len(), 512, "CFB header must be one sector");

    // --- FAT sector (sector 0): 128 entries ---
    out.extend_from_slice(&FATSECT.to_le_bytes()); // entry 0: the FAT sector itself
    out.extend_from_slice(&ENDOFCHAIN.to_le_bytes()); // entry 1: the directory chain ends here
    for _ in 2..128 {
        out.extend_from_slice(&FREESECT.to_le_bytes());
    }
    debug_assert_eq!(out.len(), 2 * 512, "CFB FAT must be one sector");

    // --- Directory sector (sector 1): 4 × 128-byte entries ---
    // Entry 0 — the root storage.
    let dir_start = out.len();
    let mut root = [0u8; 128];
    let mut pos = 0;
    for unit in "Root Entry".encode_utf16() {
        root[pos..pos + 2].copy_from_slice(&unit.to_le_bytes());
        pos += 2;
    }
    // (a UTF-16 null terminator follows, already zero)
    root[64..66].copy_from_slice(&22u16.to_le_bytes()); // name byte length incl. terminator
    root[66] = 0x05; // object type: root storage
    root[67] = 0x01; // colour flag: black
    root[68..72].copy_from_slice(&NOSTREAM.to_le_bytes()); // left sibling
    root[72..76].copy_from_slice(&NOSTREAM.to_le_bytes()); // right sibling
    root[76..80].copy_from_slice(&NOSTREAM.to_le_bytes()); // child (no entries)
    root[116..120].copy_from_slice(&ENDOFCHAIN.to_le_bytes()); // starting sector: no mini stream
                                                               // stream size (120..128) stays 0
    out.extend_from_slice(&root);
    // Entries 1..4 — unallocated: object type 0 (unknown), all sibling/child ids NOSTREAM.
    for _ in 1..4 {
        let mut unused = [0u8; 128];
        unused[68..72].copy_from_slice(&NOSTREAM.to_le_bytes());
        unused[72..76].copy_from_slice(&NOSTREAM.to_le_bytes());
        unused[76..80].copy_from_slice(&NOSTREAM.to_le_bytes());
        out.extend_from_slice(&unused);
    }
    debug_assert_eq!(
        out.len() - dir_start,
        512,
        "CFB directory must be one sector"
    );

    out
}

/// Which kind of media a [`MediaReference`] binds — decided by the relationship type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// An audio reference (`a:audioFile@r:link`, or a `p:snd` transition/timing sound).
    Audio,
    /// A video reference (`a:videoFile@r:link`).
    Video,
    /// The generic MS 2010 media reference (`a14:media`), which can back either audio or video.
    Media,
}

/// A media relationship on a surface — an audio or video reference that can be unreachable on another
/// platform, as reported by [`Presentation::media_references`](crate::Presentation::media_references)
/// and neutralized by
/// [`Presentation::replace_media_with_placeholder`](crate::Presentation::replace_media_with_placeholder).
///
/// Media is addressed by its **relationship id** rather than a shape index: a single media object is
/// referenced from several places (the `p:pic`, its `a14:media` fallback, timing/transition sounds),
/// and timing/transition sounds are not shapes at all. All of them resolve through the surface's media
/// relationships, so redirecting the relationship neutralizes every carrier at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaReference {
    /// The relationship id within the surface's `.rels`.
    pub rel_id: String,
    /// Which kind of media the relationship binds.
    pub kind: MediaKind,
    /// Where the media is referenced from — the relationship target (an external path/URL, or an
    /// in-package part target for embedded media).
    pub target: String,
    /// Whether the relationship is external (`TargetMode="External"`), i.e. the case that can be
    /// unreachable on another platform. Embedded media is `false`.
    pub external: bool,
}

/// The built-in placeholder for an inaccessible **audio** reference — a minimal valid WAV file (44
/// bytes): a silent, empty PCM stream (mono, 8 kHz, 8-bit). Used when a caller neutralizing an audio
/// reference supplies no bytes of their own.
#[must_use]
pub fn default_placeholder_audio() -> Vec<u8> {
    let mut out = Vec::with_capacity(44);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&36u32.to_le_bytes()); // chunk size = 36 + data (0)
    out.extend_from_slice(b"WAVE");
    out.extend_from_slice(b"fmt ");
    out.extend_from_slice(&16u32.to_le_bytes()); // PCM fmt chunk size
    out.extend_from_slice(&1u16.to_le_bytes()); // audio format: PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // channels: mono
    out.extend_from_slice(&8000u32.to_le_bytes()); // sample rate
    out.extend_from_slice(&8000u32.to_le_bytes()); // byte rate = rate * channels * bits/8
    out.extend_from_slice(&1u16.to_le_bytes()); // block align = channels * bits/8
    out.extend_from_slice(&8u16.to_le_bytes()); // bits per sample
    out.extend_from_slice(b"data");
    out.extend_from_slice(&0u32.to_le_bytes()); // data size: empty
    out
}

/// The built-in placeholder for an inaccessible **video** reference — a minimal, structurally valid
/// ISO base media (MP4) file: an `ftyp` plus a `moov` describing a single video track with an empty
/// sample table (no frames). It plays nothing, but its box tree parses cleanly. Used when a caller
/// neutralizing a video reference supplies no bytes of their own.
#[must_use]
pub fn default_placeholder_video() -> Vec<u8> {
    /// `[size:u32 BE][type][payload]`.
    fn bx(box_type: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + payload.len());
        out.extend_from_slice(&((8 + payload.len()) as u32).to_be_bytes());
        out.extend_from_slice(box_type);
        out.extend_from_slice(payload);
        out
    }
    /// A full box: a 1-byte version and 3-byte flags precede `body`.
    fn full_bx(box_type: &[u8; 4], version: u8, flags: u32, body: &[u8]) -> Vec<u8> {
        let mut payload = Vec::with_capacity(4 + body.len());
        payload.push(version);
        payload.extend_from_slice(&flags.to_be_bytes()[1..]); // low 3 bytes
        payload.extend_from_slice(body);
        bx(box_type, &payload)
    }
    // The 3x3 video transform matrix (identity, 16.16 fixed point) shared by tkhd/mvhd.
    fn identity_matrix() -> [u8; 36] {
        let values: [u32; 9] = [0x0001_0000, 0, 0, 0, 0x0001_0000, 0, 0, 0, 0x4000_0000];
        let mut out = [0u8; 36];
        for (i, v) in values.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
        }
        out
    }

    let ftyp = bx(b"ftyp", &{
        let mut p = Vec::new();
        p.extend_from_slice(b"isom"); // major brand
        p.extend_from_slice(&0x0000_0200u32.to_be_bytes()); // minor version
        p.extend_from_slice(b"isom");
        p.extend_from_slice(b"mp41"); // compatible brands
        p
    });

    // stbl: an empty sample table (no descriptions, no samples).
    let stsd = full_bx(b"stsd", 0, 0, &0u32.to_be_bytes()); // entry_count = 0
    let stts = full_bx(b"stts", 0, 0, &0u32.to_be_bytes());
    let stsc = full_bx(b"stsc", 0, 0, &0u32.to_be_bytes());
    let stsz = full_bx(b"stsz", 0, 0, &{
        let mut p = Vec::new();
        p.extend_from_slice(&0u32.to_be_bytes()); // sample_size
        p.extend_from_slice(&0u32.to_be_bytes()); // sample_count
        p
    });
    let stco = full_bx(b"stco", 0, 0, &0u32.to_be_bytes());
    let stbl = bx(b"stbl", &[stsd, stts, stsc, stsz, stco].concat());

    // dinf > dref with a single self-contained data reference (flags bit 0 = data is in this file).
    let url = full_bx(b"url ", 0, 1, &[]);
    let dref = full_bx(b"dref", 0, 0, &[&1u32.to_be_bytes()[..], &url].concat());
    let dinf = bx(b"dinf", &dref);

    let vmhd = full_bx(b"vmhd", 0, 1, &[0u8; 8]); // graphicsmode + opcolor
    let minf = bx(b"minf", &[vmhd, dinf, stbl].concat());

    let mdhd = full_bx(b"mdhd", 0, 0, &{
        let mut p = Vec::new();
        p.extend_from_slice(&0u32.to_be_bytes()); // creation time
        p.extend_from_slice(&0u32.to_be_bytes()); // modification time
        p.extend_from_slice(&1000u32.to_be_bytes()); // timescale
        p.extend_from_slice(&0u32.to_be_bytes()); // duration
        p.extend_from_slice(&0x55C4u16.to_be_bytes()); // language 'und'
        p.extend_from_slice(&0u16.to_be_bytes()); // pre_defined
        p
    });
    let hdlr = full_bx(b"hdlr", 0, 0, &{
        let mut p = Vec::new();
        p.extend_from_slice(&0u32.to_be_bytes()); // pre_defined
        p.extend_from_slice(b"vide"); // handler type
        p.extend_from_slice(&[0u8; 12]); // reserved
        p.push(0); // empty, null-terminated name
        p
    });
    let mdia = bx(b"mdia", &[mdhd, hdlr, minf].concat());

    let tkhd = full_bx(b"tkhd", 0, 0x0000_0007, &{
        let mut p = Vec::new();
        p.extend_from_slice(&0u32.to_be_bytes()); // creation
        p.extend_from_slice(&0u32.to_be_bytes()); // modification
        p.extend_from_slice(&1u32.to_be_bytes()); // track_id
        p.extend_from_slice(&0u32.to_be_bytes()); // reserved
        p.extend_from_slice(&0u32.to_be_bytes()); // duration
        p.extend_from_slice(&[0u8; 8]); // reserved
        p.extend_from_slice(&0u16.to_be_bytes()); // layer
        p.extend_from_slice(&0u16.to_be_bytes()); // alternate_group
        p.extend_from_slice(&0u16.to_be_bytes()); // volume
        p.extend_from_slice(&0u16.to_be_bytes()); // reserved
        p.extend_from_slice(&identity_matrix());
        p.extend_from_slice(&0u32.to_be_bytes()); // width
        p.extend_from_slice(&0u32.to_be_bytes()); // height
        p
    });
    let trak = bx(b"trak", &[tkhd, mdia].concat());

    let mvhd = full_bx(b"mvhd", 0, 0, &{
        let mut p = Vec::new();
        p.extend_from_slice(&0u32.to_be_bytes()); // creation
        p.extend_from_slice(&0u32.to_be_bytes()); // modification
        p.extend_from_slice(&1000u32.to_be_bytes()); // timescale
        p.extend_from_slice(&0u32.to_be_bytes()); // duration
        p.extend_from_slice(&0x0001_0000u32.to_be_bytes()); // rate 1.0
        p.extend_from_slice(&0x0100u16.to_be_bytes()); // volume 1.0
        p.extend_from_slice(&0u16.to_be_bytes()); // reserved
        p.extend_from_slice(&[0u8; 8]); // reserved
        p.extend_from_slice(&identity_matrix());
        p.extend_from_slice(&[0u8; 24]); // pre_defined
        p.extend_from_slice(&2u32.to_be_bytes()); // next_track_id
        p
    });
    let moov = bx(b"moov", &[mvhd, trak].concat());

    [ftyp, moov].concat()
}
