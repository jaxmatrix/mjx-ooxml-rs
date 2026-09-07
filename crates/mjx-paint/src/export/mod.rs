//! The two **document** painters — SVG and PDF — and what they share.
//!
//! # Exporters, not stages on the way to raster
//!
//! `PLAN.md`'s Phase 7 line describes an intermediate representation turned into SVG, rasterised,
//! and then printed to PDF. **That chain is superseded and this module is the supersession.** Both
//! exporters consume the display list directly, through the same [`crate::plan_frame_with`] every
//! painter uses, and neither is built out of the other: an SVG made by rasterising and a PDF made by
//! printing an SVG are both pictures of a page rather than the page.
//!
//! # What a document painter is, and what it is not
//!
//! It implements [`crate::Painter`], because a caller that can choose between four renderers should
//! not need four call shapes — and because R10's oracle drives all of them through one loop. Three
//! consequences follow, and all three are stated rather than worked around:
//!
//! * [`crate::Painter::read_pixels`] answers `None`. A document is not pixels, and answering with
//!   some would be a painter pretending to be a rasteriser. The trait's own documentation says a
//!   golden-image suite asks every painter and skips the ones that cannot answer; this is what it
//!   is talking about.
//! * [`crate::BackendReport::antialiasing`] is [`crate::Antialiasing::None`], because the decision
//!   belongs to whatever opens the file.
//! * [`crate::Painter::end`] finishes the document, and the bytes come back from the exporter's own
//!   accessor rather than through the trait.
//!
//! # What both exporters need that a rasteriser does not
//!
//! **Outlines.** A rasteriser is handed triangles; a document made of a shape's trapezoidation is a
//! hundred times the file with a hairline seam along every interior edge in any viewer that
//! antialiases. So both plans are built with [`crate::PlanOptions::for_vector`], which keeps the
//! outline the triangles were made from — resolved by the same call the tessellator resolved its
//! own input with, so the two are one interpretation of the shape.
//!
//! **Faces.** A display list records where a glyph's *pixels* are, and neither format can use that:
//! a PDF embeds a font file and addresses glyphs by id, and an SVG draws their outlines. Both ask a
//! [`crate::FontSource`], and one that was not given a face writes the run's metadata and no
//! glyphs — visibly, in the output, rather than silently.

pub mod pdf;
pub mod svg;

use mjx_scene::{Color, PathCommand, SceneRect, SceneTransform};

/// A number, as short as it can be written without losing a document's precision.
///
/// Three decimal places: a device pixel is the unit, and a thousandth of one is far below what any
/// renderer resolves. Writing `f32`'s full precision instead would triple the size of every path in
/// the file for digits nothing can draw, and writing integers would visibly quantise a rotated
/// shape's corners.
///
/// Trailing zeros are trimmed, so `4.0` is `4` and `4.500` is `4.5`.
#[must_use]
pub fn number(value: f32) -> String {
    if !value.is_finite() {
        // A document may not contain `NaN`, and a coordinate that is one is a document that already
        // said something impossible. Zero is the one value that cannot make the file unreadable.
        return "0".to_owned();
    }
    let mut text = format!("{value:.3}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        text = "0".to_owned();
    }
    text
}

/// A colour as `#rrggbb`, with its alpha left to the caller.
///
/// Alpha is deliberately not in the string: SVG carries it in `fill-opacity` and PDF in an
/// `ExtGState`, and a `#rrggbbaa` that only one of them understands would be a shared helper that
/// is wrong for one caller.
#[must_use]
pub fn hex(color: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
}

/// A colour's alpha as a fraction.
#[must_use]
pub fn opacity(color: Color) -> f32 {
    f32::from(color.alpha) / 255.0
}

/// A transform as the six numbers both formats spell it with.
///
/// `matrix(a b c d e f)` in SVG and `a b c d e f cm` in PDF are the same six numbers in the same
/// order, which is why this is shared rather than written twice.
#[must_use]
pub fn matrix(transform: SceneTransform) -> [f32; 6] {
    [
        transform.scale_x,
        transform.shear_y,
        transform.shear_x,
        transform.scale_y,
        transform.translate_x,
        transform.translate_y,
    ]
}

/// Whether a transform is close enough to the identity to be left out of the file.
///
/// Not `== IDENTITY`: a transform composed out of several exact ones accumulates a few units in the
/// last place, and writing `matrix(1 0 0 1 0.0000001 0)` on every element of a page is noise a
/// reader has to look past. The threshold is far below a thousandth of a pixel, which is what
/// [`number`] would print anyway.
#[must_use]
pub fn is_identity(transform: SceneTransform) -> bool {
    const NEAR: f32 = 1e-6;
    (transform.scale_x - 1.0).abs() < NEAR
        && (transform.scale_y - 1.0).abs() < NEAR
        && transform.shear_x.abs() < NEAR
        && transform.shear_y.abs() < NEAR
        && transform.translate_x.abs() < NEAR
        && transform.translate_y.abs() < NEAR
}

/// A path's steps, in the operators `write` spells them with.
///
/// The two formats differ only in the operator names and in whether a close is a letter or a word,
/// so the walk is shared and the spelling is the caller's. Curves are **not** flattened: both
/// formats draw cubics, a quadratic elevates to a cubic exactly, and flattening would put a
/// document's smooth curve into the file as a polyline nobody can edit.
pub fn write_path<W: PathWriter>(commands: &[PathCommand], writer: &mut W) {
    // Where the pen is, because a quadratic elevates to a cubic using its own start point and
    // neither format has a quadratic operator that takes one implicitly.
    let mut pen = (0.0f32, 0.0f32);
    let mut started = false;
    for command in commands {
        match *command {
            PathCommand::MoveTo(to) => {
                writer.move_to(to.x, to.y);
                pen = (to.x, to.y);
                started = true;
            }
            PathCommand::LineTo(to) => {
                // A step before the first `MoveTo` is a malformed path, and beginning it at the
                // page's corner would draw a line the document never asked for. `mjx-scene`'s own
                // tessellator drops these for the same reason.
                if !started {
                    continue;
                }
                writer.line_to(to.x, to.y);
                pen = (to.x, to.y);
            }
            PathCommand::QuadraticTo { control, end } => {
                if !started {
                    continue;
                }
                // Degree elevation, exactly: a quadratic is the cubic whose control points are one
                // third and two thirds of the way from each end to the quadratic's own control.
                let first = (
                    pen.0 + 2.0 / 3.0 * (control.x - pen.0),
                    pen.1 + 2.0 / 3.0 * (control.y - pen.1),
                );
                let second = (
                    end.x + 2.0 / 3.0 * (control.x - end.x),
                    end.y + 2.0 / 3.0 * (control.y - end.y),
                );
                writer.cubic_to(first.0, first.1, second.0, second.1, end.x, end.y);
                pen = (end.x, end.y);
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                if !started {
                    continue;
                }
                writer.cubic_to(
                    first_control.x,
                    first_control.y,
                    second_control.x,
                    second_control.y,
                    end.x,
                    end.y,
                );
                pen = (end.x, end.y);
            }
            PathCommand::Close => {
                if !started {
                    continue;
                }
                writer.close();
            }
        }
    }
}

/// What [`write_path`] writes into.
pub trait PathWriter {
    /// Begin a contour.
    fn move_to(&mut self, x: f32, y: f32);
    /// A straight line.
    fn line_to(&mut self, x: f32, y: f32);
    /// A cubic curve.
    fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32);
    /// Close the contour.
    fn close(&mut self);
}

/// The SVG `d` attribute a path becomes.
#[must_use]
pub fn svg_path_data(commands: &[PathCommand]) -> String {
    struct Svg(String);
    impl PathWriter for Svg {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push_str(&format!("M{} {}", number(x), number(y)));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push_str(&format!("L{} {}", number(x), number(y)));
        }
        fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
            self.0.push_str(&format!(
                "C{} {} {} {} {} {}",
                number(x1),
                number(y1),
                number(x2),
                number(y2),
                number(x),
                number(y)
            ));
        }
        fn close(&mut self) {
            self.0.push('Z');
        }
    }
    let mut writer = Svg(String::new());
    write_path(commands, &mut writer);
    writer.0
}

/// The PDF path operators a path becomes.
#[must_use]
pub fn pdf_path_operators(commands: &[PathCommand]) -> String {
    struct Pdf(String);
    impl PathWriter for Pdf {
        fn move_to(&mut self, x: f32, y: f32) {
            self.0.push_str(&format!("{} {} m\n", number(x), number(y)));
        }
        fn line_to(&mut self, x: f32, y: f32) {
            self.0.push_str(&format!("{} {} l\n", number(x), number(y)));
        }
        fn cubic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
            self.0.push_str(&format!(
                "{} {} {} {} {} {} c\n",
                number(x1),
                number(y1),
                number(x2),
                number(y2),
                number(x),
                number(y)
            ));
        }
        fn close(&mut self) {
            self.0.push_str("h\n");
        }
    }
    let mut writer = Pdf(String::new());
    write_path(commands, &mut writer);
    writer.0
}

/// The bounding box of a path's own points.
///
/// Control points included, so the box can be larger than the ink. That is the right answer for the
/// one thing it is used for — placing a gradient — because it is the same box
/// [`crate::plan::GradientMapping::resolve`] was given: the mesh's bounds, which lyon computes from
/// the flattened curve. The two differ on a shape with a curve that bulges outward, by at most the
/// curve's own sagitta.
pub fn outline_bounds(commands: &[mjx_scene::PathCommand]) -> SceneRect {
    let mut bounds: Option<SceneRect> = None;
    let mut absorb = |point: mjx_scene::ScenePoint| {
        bounds = Some(match bounds {
            Some(rect) => rect.including(point),
            None => SceneRect::new(point.x, point.y, point.x, point.y),
        });
    };
    for command in commands {
        match *command {
            PathCommand::MoveTo(to) | PathCommand::LineTo(to) => absorb(to),
            PathCommand::QuadraticTo { control, end } => {
                absorb(control);
                absorb(end);
            }
            PathCommand::CubicTo {
                first_control,
                second_control,
                end,
            } => {
                absorb(first_control);
                absorb(second_control);
                absorb(end);
            }
            PathCommand::Close => {}
        }
    }
    bounds.unwrap_or(SceneRect::UNIT)
}

/// Text with the five characters XML reserves replaced by their entities.
///
/// Applied to every value this crate writes into an attribute, including the ones that "cannot"
/// contain a reserved character: a geometry provider's label reaches an attribute and comes from
/// outside this workspace, and a document that produced `label="a<b"` would be an SVG that
/// `xmllint` refuses and a browser silently truncates.
#[must_use]
pub fn xml_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            // A control character is not representable in XML 1.0 at all, entity or not.
            c if (c as u32) < 0x20 && c != '\t' && c != '\n' && c != '\r' => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// `data` as base64, for a data URI.
#[must_use]
pub fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let a = chunk.first().copied().unwrap_or(0);
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        let word = (u32::from(a) << 16) | (u32::from(b) << 8) | u32::from(c);
        let indices = [
            (word >> 18) & 0x3f,
            (word >> 12) & 0x3f,
            (word >> 6) & 0x3f,
            word & 0x3f,
        ];
        for (position, index) in indices.iter().enumerate() {
            if position > chunk.len() {
                out.push('=');
            } else {
                out.push(char::from(
                    ALPHABET.get(*index as usize).copied().unwrap_or(b'A'),
                ));
            }
        }
    }
    out
}

/// A non-premultiplied `RGBA` image as a PNG.
///
/// # Why this crate encodes its own
///
/// An SVG that carries a picture carries it as a data URI, and the only formats a browser is
/// obliged to decode there are PNG and JPEG. `tiny-skia` can encode a PNG, but only with its
/// `png-format` feature, which puts the whole `png` **decoder** into the dependency tree of a crate
/// that decodes nothing — see this workspace's manifest, where the feature is refused for that
/// reason.
///
/// So this writes one, using deflate's **stored** blocks: no compression, no Huffman tables, no
/// dependency. What that costs is file size on a page with pictures on it, and it costs nothing at
/// all on a page without. What it buys is that the only image codec in this workspace's shipped
/// graph is sixty lines that can be read in one sitting.
#[must_use]
pub fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    // Each row is prefixed with a filter byte; zero means "no filter", which is what an uncompressed
    // encoder wants and what makes the row data a straight copy.
    let mut raw = Vec::with_capacity((width as usize + 1) * height as usize * 4);
    for y in 0..height as usize {
        raw.push(0);
        let start = y * width as usize * 4;
        let end = start + width as usize * 4;
        match rgba.get(start..end) {
            Some(row) => raw.extend_from_slice(row),
            None => raw.extend(std::iter::repeat_n(0u8, width as usize * 4)),
        }
    }

    let mut file = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    // Eight bits per channel, colour type 6 (truecolour with alpha), deflate, no filter, no
    // interlacing.
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut file, b"IHDR", &header);
    chunk(&mut file, b"IDAT", &zlib_stored(&raw));
    chunk(&mut file, b"IEND", &[]);
    file
}

/// One PNG chunk: length, tag, payload, checksum.
fn chunk(into: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
    into.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    into.extend_from_slice(tag);
    into.extend_from_slice(payload);
    let mut checked = Vec::with_capacity(4 + payload.len());
    checked.extend_from_slice(tag);
    checked.extend_from_slice(payload);
    into.extend_from_slice(&crc32(&checked).to_be_bytes());
}

/// A zlib stream made entirely of deflate's stored blocks.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    // Deflate, 32K window, no preset dictionary, fastest level. The two header bytes must be a
    // multiple of 31 read as a big-endian sixteen-bit number, which `0x78 0x01` is.
    let mut out = vec![0x78, 0x01];
    // A stored block's length is a sixteen-bit number, so a large image is several blocks.
    let mut chunks = data.chunks(0xffff).peekable();
    if data.is_empty() {
        out.extend_from_slice(&[0x01, 0x00, 0x00, 0xff, 0xff]);
    }
    while let Some(block) = chunks.next() {
        let last = if chunks.peek().is_none() { 1u8 } else { 0u8 };
        out.push(last);
        let length = block.len() as u16;
        out.extend_from_slice(&length.to_le_bytes());
        out.extend_from_slice(&(!length).to_le_bytes());
        out.extend_from_slice(block);
    }
    out.extend_from_slice(&adler32(data).to_be_bytes());
    out
}

/// The Adler-32 checksum a zlib stream ends with.
fn adler32(data: &[u8]) -> u32 {
    let mut low = 1u32;
    let mut high = 0u32;
    for byte in data {
        low = (low + u32::from(*byte)) % 65521;
        high = (high + low) % 65521;
    }
    (high << 16) | low
}

/// The CRC-32 a PNG chunk ends with.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
