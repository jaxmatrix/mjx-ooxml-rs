//! Media (image) formats a package can carry, identified from their leading bytes.
//!
//! Storing a picture in an OOXML package needs two facts about the bytes: the **content type** to
//! register in `[Content_Types].xml`, and the **file extension** for the media part's name
//! (`/ppt/media/image1.png`). Callers should not have to supply either — the bytes say what they are.
//!
//! [`ImageFormat::sniff`] reads only a magic-byte signature. Nothing here decodes, validates, or
//! re-encodes an image: the package stores the caller's bytes verbatim, which is what the fidelity
//! contract requires, and keeps this crate pure Rust with no image dependency.

/// An image format a package part can hold, with the OOXML content type and file extension Office
/// uses for it.
///
/// Obtained from an image's leading bytes with [`sniff`](ImageFormat::sniff).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    /// Portable Network Graphics — `image/png`, extension `png`.
    Png,
    /// JPEG — `image/jpeg`, extension `jpeg`.
    Jpeg,
    /// Graphics Interchange Format — `image/gif`, extension `gif`.
    Gif,
    /// Windows bitmap — `image/bmp`, extension `bmp`.
    Bmp,
    /// Tagged Image File Format — `image/tiff`, extension `tiff`.
    Tiff,
    /// Enhanced metafile (`EMF`) — `image/x-emf`, extension `emf`.
    EnhancedMetafile,
    /// Windows metafile (`WMF`) — `image/x-wmf`, extension `wmf`.
    WindowsMetafile,
    /// Scalable Vector Graphics — `image/svg+xml`, extension `svg`.
    Svg,
}

/// How far into a text-ish payload [`ImageFormat::sniff`] looks for an `<svg` root element.
const SVG_SCAN_LIMIT: usize = 1024;

impl ImageFormat {
    /// Identifies the format of `bytes` from its leading signature, or `None` if it matches no format
    /// this build recognizes (including empty or truncated input — this never panics).
    ///
    /// Only the signature is examined; the rest of the payload is not validated.
    #[must_use]
    pub fn sniff(bytes: &[u8]) -> Option<Self> {
        // Raster and metafile formats, each identified by a fixed-offset signature.
        if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
            return Some(Self::Png);
        }
        if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            return Some(Self::Jpeg);
        }
        if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            return Some(Self::Gif);
        }
        if bytes.starts_with(b"BM") {
            return Some(Self::Bmp);
        }
        // TIFF: byte order mark ("II" little-endian / "MM" big-endian) then the version number 42.
        if bytes.starts_with(b"II\x2A\x00") || bytes.starts_with(b"MM\x00\x2A") {
            return Some(Self::Tiff);
        }
        // EMF: an EMR_HEADER record (type 1) whose fifth dword is the signature " EMF" at offset 40.
        if bytes.starts_with(&[0x01, 0x00, 0x00, 0x00])
            && bytes.get(40..44) == Some(b" EMF".as_slice())
        {
            return Some(Self::EnhancedMetafile);
        }
        // WMF: either the Aldus placeable header, or a standard METAHEADER (memory/disk metafile with
        // the mandatory 9-word header size).
        if bytes.starts_with(&[0xD7, 0xCD, 0xC6, 0x9A])
            || bytes.starts_with(&[0x01, 0x00, 0x09, 0x00])
        {
            return Some(Self::WindowsMetafile);
        }
        if is_svg(bytes) {
            return Some(Self::Svg);
        }
        None
    }

    /// The content type to register for a part holding this format (e.g. `image/png`).
    #[must_use]
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Png => "image/png",
            Self::Jpeg => "image/jpeg",
            Self::Gif => "image/gif",
            Self::Bmp => "image/bmp",
            Self::Tiff => "image/tiff",
            Self::EnhancedMetafile => "image/x-emf",
            Self::WindowsMetafile => "image/x-wmf",
            Self::Svg => "image/svg+xml",
        }
    }

    /// The file extension for a part holding this format, lowercase and without the dot (e.g. `png`).
    #[must_use]
    pub fn file_extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpeg",
            Self::Gif => "gif",
            Self::Bmp => "bmp",
            Self::Tiff => "tiff",
            Self::EnhancedMetafile => "emf",
            Self::WindowsMetafile => "wmf",
            Self::Svg => "svg",
        }
    }
}

/// Whether `bytes` look like an SVG document: XML text (after an optional UTF-8 BOM and leading
/// whitespace) whose first [`SVG_SCAN_LIMIT`] bytes contain an `<svg` element start.
///
/// SVG has no binary signature, so this is a shape test rather than a magic number: the payload must
/// begin as XML (`<?xml`, a comment/doctype, or the `<svg` root itself) *and* actually name an `svg`
/// element, so an arbitrary XML document is not mistaken for a picture.
fn is_svg(bytes: &[u8]) -> bool {
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let body = match body.iter().position(|b| !b.is_ascii_whitespace()) {
        Some(start) => &body[start..],
        None => return false,
    };
    if !body.starts_with(b"<") {
        return false;
    }
    let head = &body[..body.len().min(SVG_SCAN_LIMIT)];
    head.windows(4).any(|w| w == b"<svg")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sniffs_each_raster_format() {
        assert_eq!(
            ImageFormat::sniff(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0]),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            ImageFormat::sniff(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00]),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::sniff(b"GIF89a\x01\x00"),
            Some(ImageFormat::Gif)
        );
        assert_eq!(
            ImageFormat::sniff(b"GIF87a\x01\x00"),
            Some(ImageFormat::Gif)
        );
        assert_eq!(ImageFormat::sniff(b"BM\x36\x00"), Some(ImageFormat::Bmp));
        assert_eq!(
            ImageFormat::sniff(b"II\x2A\x00\x08"),
            Some(ImageFormat::Tiff)
        );
        assert_eq!(
            ImageFormat::sniff(b"MM\x00\x2A\x00"),
            Some(ImageFormat::Tiff)
        );
    }

    #[test]
    fn sniffs_metafiles() {
        // EMF: record type 1, then " EMF" at offset 40.
        let mut emf = vec![0u8; 44];
        emf[0] = 0x01;
        emf[40..44].copy_from_slice(b" EMF");
        assert_eq!(
            ImageFormat::sniff(&emf),
            Some(ImageFormat::EnhancedMetafile)
        );

        assert_eq!(
            ImageFormat::sniff(&[0xD7, 0xCD, 0xC6, 0x9A, 0x00]),
            Some(ImageFormat::WindowsMetafile)
        );
        assert_eq!(
            ImageFormat::sniff(&[0x01, 0x00, 0x09, 0x00, 0x00]),
            Some(ImageFormat::WindowsMetafile)
        );
    }

    #[test]
    fn emf_needs_its_offset_40_signature() {
        // The record-type prefix alone is not enough — without " EMF" this is not an EMF.
        let not_emf = vec![0u8; 64];
        assert_eq!(ImageFormat::sniff(&not_emf), None);
        // Truncated before offset 44: no signature to check, so no match (and no panic).
        assert_eq!(ImageFormat::sniff(&[0x01, 0x00, 0x00, 0x00]), None);
    }

    #[test]
    fn sniffs_svg_with_and_without_a_prologue() {
        assert_eq!(
            ImageFormat::sniff(br#"<svg xmlns="http://www.w3.org/2000/svg"/>"#),
            Some(ImageFormat::Svg)
        );
        assert_eq!(
            ImageFormat::sniff(b"\xEF\xBB\xBF\n  <?xml version=\"1.0\"?><svg/>"),
            Some(ImageFormat::Svg)
        );
    }

    #[test]
    fn other_xml_is_not_svg() {
        assert_eq!(
            ImageFormat::sniff(br#"<?xml version="1.0"?><Types/>"#),
            None
        );
        // An `<svg` beyond the scan limit does not count.
        let mut far = br#"<?xml version="1.0"?>"#.to_vec();
        far.resize(SVG_SCAN_LIMIT + 8, b' ');
        far.extend_from_slice(b"<svg/>");
        assert_eq!(ImageFormat::sniff(&far), None);
    }

    #[test]
    fn rejects_empty_and_unknown_input() {
        assert_eq!(ImageFormat::sniff(&[]), None);
        assert_eq!(ImageFormat::sniff(b"   "), None);
        assert_eq!(ImageFormat::sniff(b"not an image at all"), None);
        // A truncated PNG signature is not a PNG.
        assert_eq!(ImageFormat::sniff(&[0x89, b'P', b'N']), None);
    }

    #[test]
    fn content_types_and_extensions_are_consistent() {
        let all = [
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::Gif,
            ImageFormat::Bmp,
            ImageFormat::Tiff,
            ImageFormat::EnhancedMetafile,
            ImageFormat::WindowsMetafile,
            ImageFormat::Svg,
        ];
        for format in all {
            assert!(format.content_type().starts_with("image/"));
            let ext = format.file_extension();
            assert!(!ext.is_empty() && ext.chars().all(|c| c.is_ascii_lowercase()));
        }
    }
}
