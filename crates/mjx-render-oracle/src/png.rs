//! A PNG encoder, and **the decision about premultiplied alpha** that a baseline cannot be committed
//! without.
//!
//! # ⚠ The premultiplication decision, made here and recorded once (MJXOFF-164 hand-off 3)
//!
//! `mjx_paint::Pixels::rgba` is **premultiplied**. R08's documentation said it was not, R09 measured
//! it (`80 00 00 80` for a half-alpha red, from *both* rasterisers), corrected the sentence, and
//! deliberately left the convention alone — because changing it moves every pixel assertion in
//! `mjx-paint` and every golden image taken after that point, and that is the decision of whoever
//! owns the comparison. This crate owns the comparison, and this is the decision:
//!
//! **The painter's convention does not move. `Pixels` stays premultiplied, and every golden image is
//! written straight.**
//!
//! Three reasons, in the order they mattered:
//!
//! 1. **Premultiplied is what the hardware holds.** Every render target in `mjx-paint` is
//!    premultiplied and source-over is a blend factor rather than a formula precisely because of it.
//!    A readback that un-premultiplied would be a conversion on the hot path of every frame, and a
//!    *lossy* one: alpha zero destroys the colour, so `Pixels -> straight -> Pixels` is not the
//!    identity and a painter's own round-trip assertions would stop holding.
//! 2. **Straight is what PNG is.** The specification says an RGBA sample is not premultiplied, so a
//!    golden image written premultiplied would be a file every viewer, every diff tool and every
//!    browser in R11's gallery renders wrong. A golden image nobody can look at is not a golden
//!    image.
//! 3. **The conversion therefore happens exactly once, at the file boundary**, where it is named
//!    ([`straight_from_premultiplied`]), tested, and — the part that matters — **checked by a decoder
//!    this workspace did not write**: `tests/the_instruments_measure_what_they_claim.rs` writes a
//!    half-alpha red through here and has ImageMagick read `#FF000080` back out. A convention
//!    asserted only by the code that implements it is a convention that quietly stops being true.
//!
//! # Why the encoder is hand-written
//!
//! `mjx-paint` deliberately does **not** enable `tiny-skia`'s `png-format` feature, so that a PNG
//! *decoder* does not end up in the dependency tree of a crate that decodes nothing. Adding an image
//! crate here to write test artefacts would undo that decision one level up and put a second image
//! pipeline in the workspace. MJXOFF-165's ticket says to reuse `mjx_paint::export::png`; **there is
//! no such module** — R09 never wrote one — so it is written here, where its only consumer is.
//!
//! # The compressed stream
//!
//! Fixed-Huffman deflate (RFC 1951 §3.2.6) over an LZ77 match search, wrapped in a zlib stream
//! (RFC 1950) with an Adler-32. Fixed Huffman rather than dynamic because the code lengths are in the
//! specification instead of in the file: there is no tree to build, no tree to write, and the whole
//! encoder is a hundred lines that cannot produce a stream whose header disagrees with its body. A
//! plate is mostly flat colour, so the match search does the real work — a 640 × 480 plate is a
//! megabyte and a bit raw and a few kilobytes written, which is what makes committing baselines
//! reasonable at all.
//!
//! **Determinism is a requirement, not a nicety.** The same pixels must produce the same bytes, or a
//! baseline's digest would change without its image changing and every approval would expire on the
//! next machine. There is no time, no randomness and no capacity-dependent behaviour anywhere below;
//! `tests/the_baselines_are_approved_and_reproducible.rs` encodes the same image twice and compares.

use mjx_paint::Pixels;

/// The eight-byte signature every PNG starts with.
const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];

/// Colour type 6: truecolour with alpha.
const COLOUR_TYPE_RGBA: u8 = 6;

/// How far back a match may reach, as RFC 1951 fixes it.
const WINDOW: usize = 32_768;

/// The longest match deflate can encode.
const MAXIMUM_MATCH: usize = 258;

/// The shortest match worth encoding: two bytes cost more as a match than as two literals.
const MINIMUM_MATCH: usize = 3;

/// How many candidates the match search walks before taking the best it has.
///
/// A bound rather than an exhaustive search, and a *fixed* bound: the output has to be identical on
/// every machine, so a search that stopped on a timer or on a heuristic tuned to the input would
/// make a baseline's digest depend on where it was generated.
const MAXIMUM_CHAIN: usize = 64;

/// One RGBA image, straight (not premultiplied), ready to be written.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Image {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
    /// `width * height * 4` bytes of **straight** RGBA, row zero at the top.
    pub rgba: Vec<u8>,
}

impl Image {
    /// The four bytes at (`x`, `y`), or `None` outside the image.
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let offset = ((y as usize) * (self.width as usize) + (x as usize)) * 4;
        let bytes = self.rgba.get(offset..offset + 4)?;
        Some([bytes[0], bytes[1], bytes[2], bytes[3]])
    }

    /// How many pixels are not fully transparent.
    #[must_use]
    pub fn covered(&self) -> usize {
        self.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|p| p[3] != 0)
            .count()
    }
}

/// One straight-alpha pixel from one premultiplied one.
///
/// **The whole of the premultiplication decision, in four lines.** Rounded rather than truncated —
/// `(channel * 255 + alpha / 2) / alpha` — so that a channel equal to alpha, which is what a fully
/// saturated colour premultiplies to, comes back as exactly `0xff` rather than one level short. A
/// fully transparent pixel has no colour to recover and answers all zeroes, which is the one place
/// the conversion loses information and the reason it is not applied to `Pixels` itself.
#[must_use]
pub fn straight_from_premultiplied(pixel: [u8; 4]) -> [u8; 4] {
    let alpha = pixel[3];
    if alpha == 0 {
        return [0, 0, 0, 0];
    }
    if alpha == 0xff {
        return pixel;
    }
    let recover = |channel: u8| -> u8 {
        let numerator = u32::from(channel) * 255 + u32::from(alpha) / 2;
        // A premultiplied channel can exceed its alpha only in an image that was never
        // premultiplied; clamping rather than wrapping keeps such a pixel visible instead of
        // turning it into a wildly wrong colour.
        (numerator / u32::from(alpha)).min(255) as u8
    };
    [
        recover(pixel[0]),
        recover(pixel[1]),
        recover(pixel[2]),
        alpha,
    ]
}

/// A painter's premultiplied readback as a straight-alpha [`Image`].
#[must_use]
pub fn image_from_pixels(pixels: &Pixels) -> Image {
    let mut rgba = Vec::with_capacity(pixels.rgba.len());
    for pixel in pixels.rgba.as_chunks::<4>().0 {
        rgba.extend_from_slice(&straight_from_premultiplied(*pixel));
    }
    Image {
        width: pixels.width,
        height: pixels.height,
        rgba,
    }
}

/// Encode `image` as a PNG.
///
/// # Errors
///
/// A sentence, for a size that does not match the payload or a dimension of zero. Both are
/// programming errors rather than inputs, and both are refused rather than written as a file whose
/// header disagrees with its body.
pub fn encode(image: &Image) -> Result<Vec<u8>, String> {
    if image.width == 0 || image.height == 0 {
        return Err(format!(
            "a PNG cannot be {} by {}",
            image.width, image.height
        ));
    }
    let wanted = (image.width as usize)
        .checked_mul(image.height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| format!("{} by {} overflows", image.width, image.height))?;
    if image.rgba.len() != wanted {
        return Err(format!(
            "a {} by {} image needs {wanted} bytes and carries {}",
            image.width,
            image.height,
            image.rgba.len()
        ));
    }

    // Filter type 0 (None) on every scanline. Not a compromise: a plate is flat colour in large
    // regions, which the match search already turns into two-byte length/distance pairs, and a
    // per-line filter choice would be a heuristic — the one thing a byte-reproducible artefact must
    // not contain.
    let stride = (image.width as usize) * 4;
    let mut raw = Vec::with_capacity(image.rgba.len() + image.height as usize);
    for row in image.rgba.chunks_exact(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    let mut png = Vec::with_capacity(raw.len() / 4 + 1024);
    png.extend_from_slice(&SIGNATURE);

    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&image.width.to_be_bytes());
    header.extend_from_slice(&image.height.to_be_bytes());
    header.push(8); // bit depth
    header.push(COLOUR_TYPE_RGBA);
    header.push(0); // compression method: deflate
    header.push(0); // filter method: the five adaptive filters
    header.push(0); // interlace: none
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &zlib(&raw));
    chunk(&mut png, b"IEND", &[]);
    Ok(png)
}

/// One PNG chunk: length, type, payload, CRC over type and payload.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]) {
    // A chunk longer than `u32::MAX` cannot exist here: the only large one is `IDAT`, whose input is
    // an image this crate built.
    out.extend_from_slice(&(u32::try_from(payload.len()).unwrap_or(u32::MAX)).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    let mut crc = Crc32::new();
    crc.write(kind);
    crc.write(payload);
    out.extend_from_slice(&crc.finish().to_be_bytes());
}

/// A zlib stream (RFC 1950) around fixed-Huffman deflate.
fn zlib(raw: &[u8]) -> Vec<u8> {
    // CMF: deflate, 32 KiB window. FLG: no dictionary, and the low five bits chosen so that the
    // two bytes read as a big-endian multiple of 31.
    let mut out = vec![0x78, 0x01];
    deflate_fixed(raw, &mut out);
    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// A little-endian bit sink, which is what a deflate stream is.
struct Bits {
    out: Vec<u8>,
    accumulator: u32,
    bits: u32,
}

impl Bits {
    const fn new() -> Self {
        Self {
            out: Vec::new(),
            accumulator: 0,
            bits: 0,
        }
    }

    /// `count` bits of `value`, least significant first — how deflate writes everything **except** a
    /// Huffman code.
    fn push(&mut self, value: u32, count: u32) {
        self.accumulator |= value << self.bits;
        self.bits += count;
        while self.bits >= 8 {
            // The mask keeps this inside a byte; the shift cannot lose a set bit.
            self.out.push((self.accumulator & 0xff) as u8);
            self.accumulator >>= 8;
            self.bits -= 8;
        }
    }

    /// A Huffman code: `count` bits of `code`, **most** significant first, as RFC 1951 §3.1.1
    /// requires.
    fn push_code(&mut self, code: u32, count: u32) {
        for index in (0..count).rev() {
            self.push((code >> index) & 1, 1);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.bits > 0 {
            self.out.push((self.accumulator & 0xff) as u8);
        }
        self.out
    }
}

/// The fixed literal/length code for `symbol`, as RFC 1951 §3.2.6 tabulates it.
const fn fixed_literal(symbol: u32) -> (u32, u32) {
    match symbol {
        0..=143 => (0x30 + symbol, 8),
        144..=255 => (0x190 + symbol - 144, 9),
        256..=279 => (symbol - 256, 7),
        _ => (0xc0 + symbol - 280, 8),
    }
}

/// The length codes: `(code, extra bits, smallest length)`.
const LENGTHS: [(u32, u32, usize); 29] = [
    (257, 0, 3),
    (258, 0, 4),
    (259, 0, 5),
    (260, 0, 6),
    (261, 0, 7),
    (262, 0, 8),
    (263, 0, 9),
    (264, 0, 10),
    (265, 1, 11),
    (266, 1, 13),
    (267, 1, 15),
    (268, 1, 17),
    (269, 2, 19),
    (270, 2, 23),
    (271, 2, 27),
    (272, 2, 31),
    (273, 3, 35),
    (274, 3, 43),
    (275, 3, 51),
    (276, 3, 59),
    (277, 4, 67),
    (278, 4, 83),
    (279, 4, 99),
    (280, 4, 115),
    (281, 5, 131),
    (282, 5, 163),
    (283, 5, 195),
    (284, 5, 227),
    (285, 0, 258),
];

/// The distance codes: `(code, extra bits, smallest distance)`.
const DISTANCES: [(u32, u32, usize); 30] = [
    (0, 0, 1),
    (1, 0, 2),
    (2, 0, 3),
    (3, 0, 4),
    (4, 1, 5),
    (5, 1, 7),
    (6, 2, 9),
    (7, 2, 13),
    (8, 3, 17),
    (9, 3, 25),
    (10, 4, 33),
    (11, 4, 49),
    (12, 5, 65),
    (13, 5, 97),
    (14, 6, 129),
    (15, 6, 193),
    (16, 7, 257),
    (17, 7, 385),
    (18, 8, 513),
    (19, 8, 769),
    (20, 9, 1025),
    (21, 9, 1537),
    (22, 10, 2049),
    (23, 10, 3073),
    (24, 11, 4097),
    (25, 11, 6145),
    (26, 12, 8193),
    (27, 12, 12_289),
    (28, 13, 16_385),
    (29, 13, 24_577),
];

/// One deflate block, `BFINAL = 1`, `BTYPE = 01`, over an LZ77 pass.
fn deflate_fixed(raw: &[u8], out: &mut Vec<u8>) {
    let mut bits = Bits::new();
    bits.push(1, 1); // final block
    bits.push(1, 2); // fixed Huffman codes

    // A three-byte hash to the most recent position with that hash, and a chain back through the
    // earlier ones. `usize::MAX` rather than `Option` so the tables are two flat allocations.
    let mut head = vec![usize::MAX; 1 << 15];
    let mut previous = vec![usize::MAX; raw.len()];
    let hash = |window: &[u8]| -> usize {
        ((usize::from(window[0]) << 10) ^ (usize::from(window[1]) << 5) ^ usize::from(window[2]))
            & ((1 << 15) - 1)
    };

    let mut position = 0usize;
    while position < raw.len() {
        let mut best_length = 0usize;
        let mut best_distance = 0usize;
        if position + MINIMUM_MATCH <= raw.len() {
            let key = hash(&raw[position..position + MINIMUM_MATCH]);
            let mut candidate = head[key];
            let limit = position.saturating_sub(WINDOW);
            let mut walked = 0usize;
            while candidate != usize::MAX && candidate >= limit && walked < MAXIMUM_CHAIN {
                walked += 1;
                let span = (raw.len() - position).min(MAXIMUM_MATCH);
                let mut length = 0usize;
                while length < span && raw[candidate + length] == raw[position + length] {
                    length += 1;
                }
                if length > best_length {
                    best_length = length;
                    best_distance = position - candidate;
                    if length == MAXIMUM_MATCH {
                        break;
                    }
                }
                candidate = previous[candidate];
            }
            previous[position] = head[key];
            head[key] = position;
        }

        if best_length >= MINIMUM_MATCH {
            let (code, extra, base) = *LENGTHS
                .iter()
                .rev()
                .find(|(_, _, base)| *base <= best_length)
                .unwrap_or(&LENGTHS[0]);
            let (literal, width) = fixed_literal(code);
            bits.push_code(literal, width);
            if extra > 0 {
                bits.push((best_length - base) as u32, extra);
            }
            let (code, extra, base) = *DISTANCES
                .iter()
                .rev()
                .find(|(_, _, base)| *base <= best_distance)
                .unwrap_or(&DISTANCES[0]);
            bits.push_code(code, 5);
            if extra > 0 {
                bits.push((best_distance - base) as u32, extra);
            }
            // Every position the match covered still has to enter the hash chains, or the next match
            // cannot reach back through it.
            for skipped in position + 1..position + best_length {
                if skipped + MINIMUM_MATCH <= raw.len() {
                    let key = hash(&raw[skipped..skipped + MINIMUM_MATCH]);
                    previous[skipped] = head[key];
                    head[key] = skipped;
                }
            }
            position += best_length;
        } else {
            let (literal, width) = fixed_literal(u32::from(raw[position]));
            bits.push_code(literal, width);
            position += 1;
        }
    }

    let (literal, width) = fixed_literal(256); // end of block
    bits.push_code(literal, width);
    out.extend_from_slice(&bits.finish());
}

/// RFC 1950's Adler-32.
fn adler32(bytes: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    // 5552 is the largest run for which neither accumulator can overflow a `u32`.
    for block in bytes.chunks(5552) {
        for byte in block {
            a += u32::from(*byte);
            b += a;
        }
        a %= 65_521;
        b %= 65_521;
    }
    (b << 16) | a
}

/// PNG's CRC-32, built from its polynomial rather than from a committed table, so there is no table
/// to get wrong.
pub(crate) struct Crc32 {
    value: u32,
}

impl Crc32 {
    pub(crate) const fn new() -> Self {
        Self { value: 0xffff_ffff }
    }

    pub(crate) fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= u32::from(*byte);
            for _ in 0..8 {
                let carry = self.value & 1;
                self.value >>= 1;
                if carry != 0 {
                    self.value ^= 0xedb8_8320;
                }
            }
        }
    }

    pub(crate) const fn finish(self) -> u32 {
        self.value ^ 0xffff_ffff
    }
}

// -------------------------------------------------------------------------------------------
// Reading back
// -------------------------------------------------------------------------------------------

/// Decode a PNG **this crate wrote**.
///
/// # What this is, and what it is deliberately not
///
/// It is not a PNG library. It reads 8-bit truecolour-with-alpha, non-interlaced, out of a `zlib`
/// stream of stored or fixed-Huffman deflate blocks — which is exactly the set [`encode`] produces —
/// and it refuses anything else by name rather than guessing. Its whole job is to bring **an
/// approved baseline back into memory**, because two things need that and neither can be done from
/// the bytes alone: running the perceptual metric so a failure says *where* the page moved, and
/// writing the diff image a reviewer looks at. MJXOFF-165's constraint is that a failure a reviewer
/// cannot see is a failure nobody fixes, and a diff image cannot be made without the other side.
///
/// **A round trip through this proves nothing about PNG validity** — it is this crate's encoder
/// checked by this crate's decoder. Validity is checked by a decoder this workspace did not write:
/// `tests/the_instruments_measure_what_they_claim.rs` has ImageMagick read a plate back, dimensions,
/// straight alpha and all.
///
/// # Errors
///
/// A sentence naming what was wrong and where.
pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    if bytes.len() < 8 || bytes[..8] != SIGNATURE {
        return Err("this is not a PNG: the signature is wrong".to_owned());
    }
    let mut cursor = 8usize;
    let mut header: Option<(u32, u32)> = None;
    let mut compressed = Vec::new();
    while cursor + 8 <= bytes.len() {
        let length = u32::from_be_bytes([
            bytes[cursor],
            bytes[cursor + 1],
            bytes[cursor + 2],
            bytes[cursor + 3],
        ]) as usize;
        let kind = &bytes[cursor + 4..cursor + 8];
        let start = cursor + 8;
        let payload = bytes
            .get(start..start + length)
            .ok_or_else(|| format!("a chunk at byte {cursor} claims {length} bytes it has not"))?;
        let recorded = bytes
            .get(start + length..start + length + 4)
            .map(|slice| u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
            .ok_or_else(|| format!("a chunk at byte {cursor} has no checksum"))?;
        let mut crc = Crc32::new();
        crc.write(kind);
        crc.write(payload);
        if crc.finish() != recorded {
            return Err(format!("the chunk at byte {cursor} fails its own CRC"));
        }
        match kind {
            b"IHDR" => {
                if payload.len() != 13 {
                    return Err("the header chunk is not thirteen bytes".to_owned());
                }
                let width = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]);
                let height = u32::from_be_bytes([payload[4], payload[5], payload[6], payload[7]]);
                if payload[8] != 8 {
                    return Err(format!("bit depth {} is not eight", payload[8]));
                }
                if payload[9] != COLOUR_TYPE_RGBA {
                    return Err(format!("colour type {} is not RGBA", payload[9]));
                }
                if payload[12] != 0 {
                    return Err("an interlaced PNG is not read here".to_owned());
                }
                header = Some((width, height));
            }
            b"IDAT" => compressed.extend_from_slice(payload),
            b"IEND" => break,
            _ => {}
        }
        cursor = start + length + 4;
    }
    let (width, height) = header.ok_or("the PNG has no header chunk")?;
    let raw = inflate_zlib(&compressed)?;

    let stride = (width as usize)
        .checked_mul(4)
        .ok_or("the image is too wide")?;
    let wanted = (stride + 1)
        .checked_mul(height as usize)
        .ok_or("the image is too large")?;
    if raw.len() != wanted {
        return Err(format!(
            "the decompressed image is {} bytes and a {width} by {height} one is {wanted}",
            raw.len()
        ));
    }

    // Un-filter. All five filters are implemented even though `encode` only writes `None`: they are
    // twenty lines, and a decoder that silently mis-read a filtered row would corrupt a baseline
    // rather than refuse it.
    let mut rgba = vec![0u8; stride * (height as usize)];
    for row in 0..height as usize {
        let filter = raw[row * (stride + 1)];
        let source = &raw[row * (stride + 1) + 1..row * (stride + 1) + 1 + stride];
        for column in 0..stride {
            let left = if column >= 4 {
                rgba[row * stride + column - 4]
            } else {
                0
            };
            let up = if row > 0 {
                rgba[(row - 1) * stride + column]
            } else {
                0
            };
            let up_left = if row > 0 && column >= 4 {
                rgba[(row - 1) * stride + column - 4]
            } else {
                0
            };
            rgba[row * stride + column] = match filter {
                0 => source[column],
                1 => source[column].wrapping_add(left),
                2 => source[column].wrapping_add(up),
                3 => source[column].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => source[column].wrapping_add(paeth(left, up, up_left)),
                other => return Err(format!("filter type {other} is not one of the five")),
            };
        }
    }
    Ok(Image {
        width,
        height,
        rgba,
    })
}

/// PNG's Paeth predictor.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(up_left);
    let distance_left = (estimate - i16::from(left)).abs();
    let distance_up = (estimate - i16::from(up)).abs();
    let distance_up_left = (estimate - i16::from(up_left)).abs();
    if distance_left <= distance_up && distance_left <= distance_up_left {
        left
    } else if distance_up <= distance_up_left {
        up
    } else {
        up_left
    }
}

/// Unwrap a zlib stream and inflate it, checking the Adler-32.
fn inflate_zlib(stream: &[u8]) -> Result<Vec<u8>, String> {
    if stream.len() < 6 {
        return Err("the compressed stream is too short to be one".to_owned());
    }
    if stream[0] & 0x0f != 8 {
        return Err(format!(
            "compression method {} is not deflate",
            stream[0] & 0x0f
        ));
    }
    if stream[1] & 0x20 != 0 {
        return Err("a preset dictionary is not read here".to_owned());
    }
    let body = &stream[2..stream.len() - 4];
    let recorded = u32::from_be_bytes([
        stream[stream.len() - 4],
        stream[stream.len() - 3],
        stream[stream.len() - 2],
        stream[stream.len() - 1],
    ]);
    let out = inflate(body)?;
    if adler32(&out) != recorded {
        return Err("the decompressed image fails its own Adler-32".to_owned());
    }
    Ok(out)
}

/// A little-endian bit source, the mirror of [`Bits`].
struct BitReader<'a> {
    bytes: &'a [u8],
    position: usize,
    bit: u32,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            bit: 0,
        }
    }

    /// `count` bits, least significant first.
    fn take(&mut self, count: u32) -> Result<u32, String> {
        let mut value = 0u32;
        for index in 0..count {
            let byte = *self
                .bytes
                .get(self.position)
                .ok_or("the compressed stream ends inside a code")?;
            value |= u32::from((byte >> self.bit) & 1) << index;
            self.bit += 1;
            if self.bit == 8 {
                self.bit = 0;
                self.position += 1;
            }
        }
        Ok(value)
    }

    /// Discard the rest of the current byte.
    fn align(&mut self) {
        if self.bit != 0 {
            self.bit = 0;
            self.position += 1;
        }
    }
}

/// Inflate a raw deflate stream of stored and fixed-Huffman blocks.
fn inflate(bytes: &[u8]) -> Result<Vec<u8>, String> {
    let mut reader = BitReader::new(bytes);
    let mut out = Vec::with_capacity(bytes.len() * 4);
    loop {
        let final_block = reader.take(1)? == 1;
        let kind = reader.take(2)?;
        match kind {
            0 => {
                reader.align();
                let length = reader.take(16)? as usize;
                let complement = reader.take(16)? as usize;
                if length ^ 0xffff != complement {
                    return Err("a stored block's length disagrees with its complement".to_owned());
                }
                let start = reader.position;
                let payload = reader
                    .bytes
                    .get(start..start + length)
                    .ok_or("a stored block runs past the end of the stream")?;
                out.extend_from_slice(payload);
                reader.position += length;
            }
            1 => inflate_fixed_block(&mut reader, &mut out)?,
            2 => {
                return Err(
                    "a dynamic-Huffman block: this decoder reads only what this crate's \
                            own encoder writes, and that is stored and fixed blocks"
                        .to_owned(),
                )
            }
            _ => return Err("block type 3 is reserved".to_owned()),
        }
        if final_block {
            return Ok(out);
        }
    }
}

/// One fixed-Huffman block.
fn inflate_fixed_block(reader: &mut BitReader<'_>, out: &mut Vec<u8>) -> Result<(), String> {
    loop {
        let symbol = fixed_symbol(reader)?;
        match symbol {
            0..=255 => out.push(symbol as u8),
            256 => return Ok(()),
            257..=285 => {
                let (_, extra, base) = *LENGTHS
                    .iter()
                    .find(|(code, _, _)| *code == symbol)
                    .ok_or_else(|| format!("length code {symbol} is not one of the twenty-nine"))?;
                let length = base + reader.take(extra)? as usize;
                let distance_code = reverse(reader.take(5)?, 5);
                let (_, extra, base) = *DISTANCES
                    .iter()
                    .find(|(code, _, _)| *code == distance_code)
                    .ok_or_else(|| format!("distance code {distance_code} is not one of thirty"))?;
                let distance = base + reader.take(extra)? as usize;
                if distance > out.len() {
                    return Err(format!(
                        "a match reaches {distance} bytes back into a {} byte output",
                        out.len()
                    ));
                }
                let start = out.len() - distance;
                // Byte at a time, because a match may overlap itself — which is how deflate writes
                // a run of one repeated byte.
                for offset in 0..length {
                    let byte = out[start + offset];
                    out.push(byte);
                }
            }
            other => return Err(format!("literal/length symbol {other} does not exist")),
        }
    }
}

/// One fixed literal/length symbol, decoded by its prefix.
///
/// The fixed code is a prefix code with four ranges, so reading seven bits and extending is the
/// whole of its decoding. Every bound below is RFC 1951 §3.2.6's own table.
fn fixed_symbol(reader: &mut BitReader<'_>) -> Result<u32, String> {
    let mut code = reverse(reader.take(7)?, 7);
    if code <= 0b001_0111 {
        return Ok(256 + code);
    }
    code = (code << 1) | reader.take(1)?;
    if (0b0011_0000..=0b1011_1111).contains(&code) {
        return Ok(code - 0b0011_0000);
    }
    if (0b1100_0000..=0b1100_0111).contains(&code) {
        return Ok(280 + code - 0b1100_0000);
    }
    code = (code << 1) | reader.take(1)?;
    if (0b1_1001_0000..=0b1_1111_1111).contains(&code) {
        return Ok(144 + code - 0b1_1001_0000);
    }
    Err(format!("{code:b} is not a fixed Huffman code"))
}

/// The low `count` bits of `value`, reversed — a Huffman code is written most significant first and
/// read least significant first, so one of the two has to turn round.
const fn reverse(value: u32, count: u32) -> u32 {
    let mut out = 0u32;
    let mut index = 0u32;
    while index < count {
        out = (out << 1) | ((value >> index) & 1);
        index += 1;
    }
    out
}
