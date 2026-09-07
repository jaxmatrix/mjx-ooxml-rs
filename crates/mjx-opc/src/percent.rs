//! Percent-encoding of the two places OPC writes a part reference: a relationship `Target` and a
//! content-type `Override`'s `PartName`.
//!
//! ECMA-376 Part 2 §9.1.1 defines a part name as an IRI, so a character outside the RFC 3986 `pchar`
//! set **must** be written percent-encoded and a consumer decodes it before matching. Office writes
//! these: an image whose file name contains a space becomes `Target="media/image%20one.png"` while
//! the ZIP entry is `word/media/image one.png`. Comparing the two without decoding matches nothing.
//!
//! # Decoding is for resolution only
//!
//! Nothing here ever writes a decoded value back into a producer's markup. [`Relationship::target`]
//! keeps the exact bytes the producer wrote, an unedited `.rels` re-emits verbatim, and decoding
//! happens on the way *into* a [`PartName`] and nowhere else. That is deliberate: decode-then-encode
//! is not the identity — `%2520` and `%20`, `%5F` and `%5f`, `%41` and `A` all decode to something
//! whose canonical re-encoding differs from what was read — so a library that normalised on write
//! would change the bytes of `.rels` parts in files nobody asked it to touch.
//!
//! [`Relationship::target`]: crate::Relationship::target
//! [`PartName`]: crate::PartName
//!
//! # Malformed escapes are passed through, not rejected
//!
//! A relationship target is untrusted input, and `%ZZ`, a truncated `%4`, or a trailing `%` are all
//! things a non-conforming producer writes — most often a file name that genuinely contains a `%`
//! and was never encoded at all (`100% margin.png`). [`decode_segment`] leaves any `%` that does not
//! introduce two hexadecimal digits exactly as it found it, so such a name still resolves to the ZIP
//! entry that carries it. Refusing would break a file this library can read today, which §2 of the
//! Phase G brief rules out; and there is nothing here that can panic on any byte string.
//!
//! The one thing that *is* refused is a segment whose decoding introduces a `/`
//! (`image%2Fone.png`): OPC forbids an encoded path separator in a part name precisely because it
//! would make one segment silently become two, and resolving it to a different part is worse than
//! reporting that it does not resolve. [`decode_part_reference`] returns `None` for that.

use std::borrow::Cow;

/// The RFC 3986 *unreserved* set — the characters [`encode_part_reference`] leaves bare.
///
/// Everything else is encoded. That is stricter than OPC requires (a `pchar` also admits the
/// sub-delimiters and `:@`), and deliberately so: over-encoding is always legal and always decodes
/// back to the same text, whereas guessing which sub-delimiter a consumer tolerates bare is not.
const fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// The value of one hexadecimal digit, or `None` if the byte is not one.
const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// The uppercase hexadecimal digit for a nibble. RFC 3986 §2.1 prefers uppercase, and so does Office.
const fn hex_digit(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        _ => b'A' + (nibble - 10),
    }
}

/// Percent-decodes one path segment, borrowing when there is nothing to decode.
///
/// A `%` followed by two hexadecimal digits becomes the byte they name; every other `%` is kept
/// literally (see the module comment). If the decoded bytes are not valid UTF-8 the segment is
/// returned unchanged — a part name is a Rust `String` here, so there is no name to resolve to, and
/// leaving the escape in place keeps the failure a lookup miss rather than a panic.
pub(crate) fn decode_segment(segment: &str) -> Cow<'_, str> {
    let bytes = segment.as_bytes();
    let Some(first) = bytes.iter().position(|&b| b == b'%') else {
        return Cow::Borrowed(segment);
    };
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..first]);
    let mut idx = first;
    while idx < bytes.len() {
        let decoded = if bytes[idx] == b'%' {
            match (
                bytes.get(idx + 1).copied().and_then(hex_value),
                bytes.get(idx + 2).copied().and_then(hex_value),
            ) {
                (Some(high), Some(low)) => Some((high << 4) | low),
                _ => None,
            }
        } else {
            None
        };
        match decoded {
            Some(byte) => {
                out.push(byte);
                idx += 3;
            }
            None => {
                out.push(bytes[idx]);
                idx += 1;
            }
        }
    }
    match String::from_utf8(out) {
        Ok(decoded) => Cow::Owned(decoded),
        Err(_) => Cow::Borrowed(segment),
    }
}

/// Percent-decodes every `/`-separated segment of a part reference, borrowing when it holds no `%`.
///
/// The split happens **before** decoding, so an encoded separator can never become a real one.
/// Returns `None` when a segment nonetheless decodes to text containing `/` — the one shape this
/// refuses rather than passes through, because resolving it would silently name a different part.
pub(crate) fn decode_part_reference(reference: &str) -> Option<Cow<'_, str>> {
    if !reference.contains('%') {
        return Some(Cow::Borrowed(reference));
    }
    let mut out = String::with_capacity(reference.len());
    for (index, segment) in reference.split('/').enumerate() {
        if index > 0 {
            out.push('/');
        }
        let decoded = decode_segment(segment);
        if decoded.contains('/') {
            return None;
        }
        out.push_str(&decoded);
    }
    Some(Cow::Owned(out))
}

/// Percent-encodes a part reference for writing into a `Target` or `PartName` attribute, borrowing
/// when every byte is already safe (which is the case for every name this library generates).
///
/// `/` is the segment separator and is left alone; `..` and `.` survive untouched because `.` is
/// unreserved. Everything outside [`is_unreserved`] — a space, a `%`, any non-ASCII byte — is
/// written as `%HH`, so [`decode_part_reference`] recovers the input exactly.
pub(crate) fn encode_part_reference(reference: &str) -> Cow<'_, str> {
    if reference
        .bytes()
        .all(|byte| byte == b'/' || is_unreserved(byte))
    {
        return Cow::Borrowed(reference);
    }
    let mut out = String::with_capacity(reference.len());
    for &byte in reference.as_bytes() {
        if byte == b'/' || is_unreserved(byte) {
            out.push(byte as char);
        } else {
            out.push('%');
            out.push(hex_digit(byte >> 4) as char);
            out.push(hex_digit(byte & 0x0F) as char);
        }
    }
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reference_without_escapes_is_borrowed_unchanged() {
        let decoded = decode_part_reference("media/image1.png").expect("resolvable");
        assert!(matches!(decoded, Cow::Borrowed("media/image1.png")));
        let encoded = encode_part_reference("media/image1.png");
        assert!(matches!(encoded, Cow::Borrowed("media/image1.png")));
    }

    #[test]
    fn decodes_the_escapes_office_writes() {
        assert_eq!(
            decode_part_reference("media/image%20one.png").expect("resolvable"),
            "media/image one.png"
        );
    }

    #[test]
    fn hexadecimal_case_does_not_matter_and_decoding_is_exactly_one_level() {
        // `%5F` and `%5f` are the same character, and `%2520` is an encoded `%20`, not a space.
        assert_eq!(decode_segment("lower%5fcase"), "lower_case");
        assert_eq!(decode_segment("UPPER%5FCASE"), "UPPER_CASE");
        assert_eq!(decode_segment("plain%2520name.png"), "plain%20name.png");
    }

    #[test]
    fn an_unnecessary_escape_still_decodes() {
        // A producer may encode a character that never needed it; `%75` is an ordinary `u`.
        assert_eq!(decode_segment("%75pper.png"), "upper.png");
    }

    #[test]
    fn a_malformed_escape_is_left_alone_rather_than_rejected() {
        // Each of these is a file name a non-conforming producer really writes. None may panic, and
        // none may lose the reference: `100% margin.png` has to keep resolving.
        assert_eq!(decode_segment("100% margin.png"), "100% margin.png");
        assert_eq!(decode_segment("%ZZ.png"), "%ZZ.png");
        assert_eq!(decode_segment("truncated%4"), "truncated%4");
        assert_eq!(decode_segment("%"), "%");
        assert_eq!(decode_segment("%%20"), "% ");
    }

    #[test]
    fn a_decoding_that_is_not_utf8_leaves_the_segment_as_written() {
        // `%FF` is not a valid UTF-8 sequence on its own; the segment stays addressable as written
        // rather than becoming a decoding error.
        assert_eq!(decode_segment("image%FF.png"), "image%FF.png");
        // A multi-byte character spelled out in escapes does decode.
        assert_eq!(decode_segment("caf%C3%A9.png"), "café.png");
    }

    #[test]
    fn an_encoded_separator_is_refused_rather_than_split() {
        assert!(decode_part_reference("media/image%2Fone.png").is_none());
        assert!(decode_part_reference("media/image%2fone.png").is_none());
    }

    #[test]
    fn encoding_then_decoding_is_the_identity() {
        for name in [
            "/word/media/image one.png",
            "/word/media/plain%20name.png",
            "/word/media/100% margin.png",
            "/word/media/café.png",
            "/word/media/a+b&c.png",
            "../media/image1.png",
        ] {
            let encoded = encode_part_reference(name);
            assert_eq!(
                decode_part_reference(&encoded).expect("resolvable"),
                name,
                "round trip failed for {name:?} (encoded as {encoded:?})"
            );
        }
    }

    #[test]
    fn encoding_uses_uppercase_hex_and_spares_the_separator_and_dot_segments() {
        assert_eq!(
            encode_part_reference("../media/a b.png"),
            "../media/a%20b.png"
        );
        assert_eq!(encode_part_reference("a%20b"), "a%2520b");
    }
}
