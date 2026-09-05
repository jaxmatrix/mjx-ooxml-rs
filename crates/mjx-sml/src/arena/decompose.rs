//! Splitting one element's own bytes into its start tag, its attribute run and its content.
//!
//! # Why a range is checked rather than believed
//!
//! `mjx-xml`'s reader records, for every element it parses, the range that element occupied in the
//! part's buffer. A packed store leans on those ranges hard: they are what lets an untouched row, an
//! untouched cell or an untouched `si` be re-emitted with a `memcpy` instead of being serialized
//! from a model. But a range is a *claim about somebody else's buffer*, and a store that believes an
//! unchecked claim writes somebody else's markup into a cell.
//!
//! So [`decompose`] takes the bytes a range resolved to **and the qualified name they are supposed
//! to describe**, and returns `None` unless they open with `<` + that name followed by a delimiter
//! and close the way that element closes. That is the same check `mjx-xml`'s own writer makes before
//! trusting a range, for the same reason. Every caller here degrades to a rebuild when the check
//! fails; none of them degrades to wrong bytes.

use super::text::TextSpan;

/// A span over `start..end`, or [`TextSpan::NONE`] when that is empty or inverted.
pub(crate) fn span_between(start: u32, end: u32) -> TextSpan {
    if end > start {
        TextSpan::new(start, end - start)
    } else {
        TextSpan::NONE
    }
}

/// A span over `start..end` that stays **present** when empty — `<v></v>` is a value, and an absent
/// one is not the same thing.
pub(crate) fn span_present_between(start: u32, end: u32) -> TextSpan {
    if end >= start {
        TextSpan::new(start, end - start)
    } else {
        TextSpan::NONE
    }
}

/// The attribute run of an element whose own bytes are `bytes`, **without being told its name**.
///
/// [`decompose`] is the right tool when the caller knows what the element is supposed to be, which
/// is nearly always: checking the name is what stops a range handing back somebody else's markup.
/// This exists for the one case where the name is genuinely not known — a `phoneticPr` whose prefix
/// need not match the `si` around it — and it is therefore deliberately weaker: it says what the
/// start tag's attributes are, and makes no claim about which element they belong to.
///
/// `None` unless the bytes open with `<`, a name, and a start tag that closes.
pub(crate) fn attribute_run_of(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.first() != Some(&b'<') {
        return None;
    }
    let mut at = 1;
    while let Some(byte) = bytes.get(at) {
        if byte.is_ascii_whitespace() || *byte == b'/' || *byte == b'>' {
            break;
        }
        at += 1;
    }
    let run_start = at;
    let mut quote = 0u8;
    let tag_end = loop {
        let byte = *bytes.get(at)?;
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = byte;
        } else if byte == b'>' {
            break at;
        }
        at += 1;
    };
    let run_end = if tag_end > run_start && bytes[tag_end - 1] == b'/' {
        tag_end - 1
    } else {
        tag_end
    };
    bytes.get(run_start..run_end)
}

/// The qualified name in `bytes`' start tag — the prefix and local name exactly as written.
///
/// [`decompose`] wants to be *told* the name, because checking it is what stops a byte range handing
/// back somebody else's markup. That is right wherever the caller knows what the element is supposed
/// to be from its position in a content model. It is not available for a run of bytes the store kept
/// **because** it matched a local name: `crate::cells`'s reader finds a cell's `<f>` by local name
/// alone, so the prefix on those bytes is the file's choice and not the `sheetData` element's, and a
/// reader of them has to take the name from the bytes before it can check anything against it.
///
/// `None` unless the bytes open with `<` and a name that is followed by a delimiter — so a caller
/// still gets a checked decomposition, it simply supplies the name from the same source.
pub(crate) fn qualified_name_of(bytes: &[u8]) -> Option<&[u8]> {
    if bytes.first() != Some(&b'<') {
        return None;
    }
    let mut at = 1;
    while let Some(byte) = bytes.get(at) {
        if byte.is_ascii_whitespace() || *byte == b'/' || *byte == b'>' {
            break;
        }
        at += 1;
    }
    // A name has to end *at* a delimiter, and it has to be non-empty: `<`, `< a` and `<>` are not
    // elements, and a name that ran to the end of the buffer was never closed.
    bytes.get(at)?;
    bytes.get(1..at).filter(|name| !name.is_empty())
}

/// Where an element's start tag ends and its content begins and ends, **in arena addresses**.
///
/// [`Decomposed`] answers the same question in offsets into the element's own bytes; this is the
/// same answer translated into the one address space the store indexes everything by, which is what
/// every caller actually wants.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ElementLayout {
    /// The bytes between the qualified name and the `>` (or `/>`) that closes the start tag,
    /// leading whitespace included — exactly what the file wrote.
    pub(crate) attribute_run: TextSpan,
    /// The first content byte.
    pub(crate) inner_start: u32,
    /// One past the last content byte.
    pub(crate) inner_end: u32,
    /// Whether the file wrote `<name/>` rather than `<name></name>`. The two are different bytes and
    /// must stay different.
    pub(crate) self_closing: bool,
}

/// [`decompose`] against the bytes `extent` covers, translated into arena addresses.
///
/// `None` when `extent` is absent, resolves to nothing, or does not describe an element named
/// `qname` — the three ways a range can fail to be the claim it makes. Every caller falls back to
/// rebuilding from the model there, which reflows whitespace but never writes another element's
/// bytes.
pub(crate) fn layout_in_arena(
    bytes: &[u8],
    qname: &[u8],
    extent: TextSpan,
) -> Option<ElementLayout> {
    if extent.is_none() || bytes.is_empty() {
        return None;
    }
    let parts = decompose(bytes, qname)?;
    let base = extent.start();
    Some(ElementLayout {
        attribute_run: TextSpan::new(
            base + parts.attribute_run.start as u32,
            (parts.attribute_run.end - parts.attribute_run.start) as u32,
        ),
        inner_start: base + parts.inner.start as u32,
        inner_end: base + parts.inner.end as u32,
        self_closing: parts.self_closing,
    })
}

/// Where an element's start tag ends and its content begins and ends, as offsets into its own bytes.
pub(crate) struct Decomposed {
    pub(crate) attribute_run: core::ops::Range<usize>,
    pub(crate) inner: core::ops::Range<usize>,
    pub(crate) self_closing: bool,
}

/// Splits `bytes` — which must be exactly one element — into its attribute run and its content.
///
/// Returns `None` unless the bytes open with `<` and `qname` followed by a delimiter, and close the
/// way an element closes. That is the same check `mjx-xml`'s writer makes before trusting a range,
/// and for the same reason: the range is a claim about somebody else's buffer, and a claim that does
/// not check out must degrade to a rebuild rather than to wrong bytes.
pub(crate) fn decompose(bytes: &[u8], qname: &[u8]) -> Option<Decomposed> {
    if bytes.first() != Some(&b'<') || !bytes.get(1..)?.starts_with(qname) {
        return None;
    }
    let run_start = 1 + qname.len();
    match bytes.get(run_start) {
        Some(b'>' | b'/') => {}
        Some(byte) if byte.is_ascii_whitespace() => {}
        _ => return None,
    }

    // Scan to the `>` that closes the start tag, stepping over quoted attribute values — `>` is
    // perfectly legal inside one, so the first `>` is not necessarily the tag's.
    let mut at = run_start;
    let mut quote = 0u8;
    let tag_end = loop {
        let byte = *bytes.get(at)?;
        if quote != 0 {
            if byte == quote {
                quote = 0;
            }
        } else if byte == b'"' || byte == b'\'' {
            quote = byte;
        } else if byte == b'>' {
            break at;
        }
        at += 1;
    };

    let self_closing = tag_end > run_start && bytes[tag_end - 1] == b'/';
    let run_end = if self_closing { tag_end - 1 } else { tag_end };
    let attribute_run = run_start..run_end;
    if self_closing {
        if tag_end + 1 != bytes.len() {
            return None;
        }
        return Some(Decomposed {
            attribute_run,
            inner: bytes.len()..bytes.len(),
            self_closing: true,
        });
    }

    // `</name >` is legal, so trim the whitespace an end tag may carry before its `>`.
    let rest = bytes.strip_suffix(b">")?;
    let mut end = rest.len();
    while end > 0 && rest[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    let rest = rest.get(..end)?.strip_suffix(qname)?.strip_suffix(b"</")?;
    let inner_end = rest.len();
    let inner_start = tag_end + 1;
    if inner_end < inner_start {
        return None;
    }
    Some(Decomposed {
        attribute_run,
        inner: inner_start..inner_end,
        self_closing: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parts(markup: &str, qname: &str) -> Option<(String, String, bool)> {
        let parsed = decompose(markup.as_bytes(), qname.as_bytes())?;
        Some((
            markup[parsed.attribute_run].to_owned(),
            markup[parsed.inner].to_owned(),
            parsed.self_closing,
        ))
    }

    #[test]
    fn an_attribute_run_is_readable_without_knowing_the_element_name() {
        assert_eq!(
            attribute_run_of(br#"<phoneticPr fontId="1" type="Hiragana"/>"#),
            Some(&br#" fontId="1" type="Hiragana""#[..])
        );
        assert_eq!(
            attribute_run_of(br#"<x:phoneticPr fontId="1"/>"#),
            Some(&br#" fontId="1""#[..]),
            "the prefix is not the caller's business here — that is the whole point"
        );
        assert_eq!(attribute_run_of(b"<t/>"), Some(&b""[..]));
        assert_eq!(attribute_run_of(b"<t>x</t>"), Some(&b""[..]));
        assert_eq!(
            attribute_run_of(br#"<c note="a>b"/>"#),
            Some(&br#" note="a>b""#[..]),
            "a `>` inside a quoted value does not end the tag"
        );
        assert_eq!(attribute_run_of(b"phoneticPr/>"), None);
        assert_eq!(
            attribute_run_of(b"<t"),
            None,
            "a start tag that never closes"
        );
    }

    #[test]
    fn splits_a_start_tag_from_its_content() {
        assert_eq!(
            parts(r#"<c r="A1"><v>12</v></c>"#, "c"),
            Some((" r=\"A1\"".to_owned(), "<v>12</v>".to_owned(), false))
        );
        assert_eq!(
            parts(r#"<x:c r="A1"/>"#, "x:c"),
            Some((" r=\"A1\"".to_owned(), String::new(), true))
        );
        assert_eq!(
            parts("<c></c>", "c"),
            Some((String::new(), String::new(), false))
        );
    }

    #[test]
    fn an_angle_bracket_inside_an_attribute_value_does_not_end_the_tag() {
        // Legal XML: `>` needs no escaping in an attribute value, and a naive scan for the first
        // `>` would cut the tag in half and read the rest of it as content.
        assert_eq!(
            parts(r#"<c note="a>b" r="A1">x</c>"#, "c"),
            Some((r#" note="a>b" r="A1""#.to_owned(), "x".to_owned(), false))
        );
    }

    #[test]
    fn whitespace_an_end_tag_is_allowed_to_carry_is_not_content() {
        assert_eq!(
            parts("<v>12</v >", "v"),
            Some((String::new(), "12".to_owned(), false))
        );
        assert_eq!(
            parts("<v >12</v>", "v"),
            Some((" ".to_owned(), "12".to_owned(), false))
        );
    }

    #[test]
    fn bytes_that_do_not_describe_this_element_are_refused() {
        // Each of these would otherwise be a way to write somebody else's markup into a cell.
        assert_eq!(parts("<cc r=\"A1\"/>", "c"), None, "a longer name");
        assert_eq!(parts("<b/>", "c"), None, "a different name");
        assert_eq!(parts("c/>", "c"), None, "no opening angle bracket");
        assert_eq!(parts("<c><v>1</v></b>", "c"), None, "a mismatched end tag");
        assert_eq!(parts("<c/><c/>", "c"), None, "more than one element");
        assert_eq!(parts("<c>", "c"), None, "no end tag at all");
    }
}
