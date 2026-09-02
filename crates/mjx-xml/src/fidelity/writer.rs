//! Hand-written serializer for the preservation tree.
//!
//! quick-xml's writer is byte-faithful only when handed the original opaque element buffer, which
//! our decomposed/mutable tree deliberately does not keep. Writing the bytes ourselves gives total
//! control (quote char, one-space-per-attribute, self-closing style) and reproduces clean
//! Office/LibreOffice XML exactly.
//!
//! # Subtree copy-on-write
//!
//! An element still in the state the reader left it in is not reconstructed at all: its recorded
//! byte range is copied out of the document's source buffer and the walk does not descend into it.
//! That preserves everything a decomposed tree cannot record — whitespace between attributes,
//! whitespace before `/>`, the exact spelling of a character reference — and turns a lightly-edited
//! part into mostly `memcpy`.
//!
//! Two rules keep it honest.
//!
//! **The range is untrusted.** It is sliced fallibly and then checked against the element it claims
//! to describe: the bytes must open with `<` and this element's qualified name, and close the way
//! this element says it closes. That is exactly what [`RawElement::source_span`] cannot check for
//! itself, because [`name`](RawElement::name) and [`empty`](RawElement::empty) are plain fields with
//! no mutation tracking. Anything that does not check out is reconstructed instead — a reflow, never
//! wrong bytes.
//!
//! **A rewritten element re-emits its namespace declarations.** A verbatim subtree carries prefixes
//! but not the `xmlns:` declarations that bind them, so if a rewritten ancestor dropped or pruned
//! its declarations every descendant beneath it would silently come unbound. It cannot happen here,
//! and the reason is structural rather than a special case: the reader keeps `xmlns` declarations as
//! ordinary [`RawAttribute`](mjx_ooxml_core::RawAttribute)s in document order, and the loop below
//! writes every attribute an element holds, in order, without inspecting any of them. There is no
//! code path that could decide a declaration is unused.

use mjx_ooxml_core::{RawDocument, RawElement, RawName, RawNode};

/// Serializes a document back to bytes, appending to `out`.
pub fn serialize(doc: &RawDocument, out: &mut Vec<u8>) {
    if doc.bom {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    for node in doc.prologue.iter() {
        write_node(doc, node, out);
    }
    write_element(doc, &doc.root, out);
    for node in doc.epilogue.iter() {
        write_node(doc, node, out);
    }
}

/// Convenience: serialize into a fresh `Vec`.
#[must_use]
pub fn serialize_to_vec(doc: &RawDocument) -> Vec<u8> {
    let mut out = Vec::new();
    serialize(doc, &mut out);
    out
}

fn write_node(doc: &RawDocument, node: &RawNode, out: &mut Vec<u8>) {
    match node {
        RawNode::Element(element) => write_element(doc, element, out),
        RawNode::Text(bytes) => out.extend_from_slice(bytes),
        RawNode::CData(bytes) => wrap(out, b"<![CDATA[", bytes, b"]]>"),
        RawNode::Comment(bytes) => wrap(out, b"<!--", bytes, b"-->"),
        RawNode::ProcessingInstruction(bytes) => wrap(out, b"<?", bytes, b"?>"),
        RawNode::Declaration(bytes) => wrap(out, b"<?", bytes, b"?>"),
        RawNode::DocType(bytes) => wrap(out, b"<!DOCTYPE", bytes, b">"),
    }
}

fn write_element(doc: &RawDocument, element: &RawElement, out: &mut Vec<u8>) {
    if let Some(verbatim) = verbatim_bytes(doc, element) {
        out.extend_from_slice(verbatim);
        return;
    }
    out.push(b'<');
    write_qname(doc, &element.name, out);
    // Every attribute, in order, declarations included — see the module docs.
    for attr in element.attributes.iter() {
        out.push(b' ');
        write_qname(doc, &attr.name, out);
        out.push(b'=');
        out.push(attr.quote.byte());
        out.extend_from_slice(&attr.value);
        out.push(attr.quote.byte());
    }
    if element.empty && element.children.is_empty() {
        out.extend_from_slice(b"/>");
    } else {
        out.push(b'>');
        for child in element.children.iter() {
            write_node(doc, child, out);
        }
        out.extend_from_slice(b"</");
        write_qname(doc, &element.name, out);
        out.push(b'>');
    }
}

/// The source bytes `element` may be copied from, or `None` if it must be reconstructed.
///
/// Returns `Some` only when the document still holds its source buffer, the element has not been
/// mutated since it was parsed, the recorded range fits the buffer, and the bytes in that range
/// still describe *this* element — see the module docs for why the last check is not redundant.
fn verbatim_bytes<'d>(doc: &'d RawDocument, element: &RawElement) -> Option<&'d [u8]> {
    let source = doc.source()?;
    let span = element.source_span()?;
    // `get` rejects an inverted or out-of-bounds range, so a nonsense span reflows rather than
    // panicking or reading someone else's markup.
    let bytes = source.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)?;
    if !opens_with(doc, element, bytes) || !closes_as_stated(doc, element, bytes) {
        return None;
    }
    Some(bytes)
}

/// Whether `bytes` starts `<` + the element's qualified name + a delimiter.
///
/// This is what catches a mutated [`RawElement::name`], which carries no mutation tracking.
fn opens_with(doc: &RawDocument, element: &RawElement, bytes: &[u8]) -> bool {
    let mut cursor = 0usize;
    if bytes.first() != Some(&b'<') {
        return false;
    }
    cursor += 1;
    if let Some(prefix) = element.name.prefix {
        if !consume(bytes, &mut cursor, doc.interner.resolve(prefix).as_bytes())
            || !consume(bytes, &mut cursor, b":")
        {
            return false;
        }
    }
    if !consume(
        bytes,
        &mut cursor,
        doc.interner.resolve(element.name.local).as_bytes(),
    ) {
        return false;
    }
    // A name must be followed by whitespace, `>` or the `/` of `/>` — never by more name bytes, or
    // `<a` would happily claim the range of `<abbr>`.
    match bytes.get(cursor) {
        Some(b'>' | b'/') => true,
        Some(byte) => byte.is_ascii_whitespace(),
        None => false,
    }
}

/// Whether `bytes` ends the way this element says it ends: `/>` for a self-closing element, and
/// `</` + the qualified name + optional whitespace + `>` otherwise.
///
/// This is what catches a flipped [`RawElement::empty`], which carries no mutation tracking.
fn closes_as_stated(doc: &RawDocument, element: &RawElement, bytes: &[u8]) -> bool {
    if element.empty && element.children.is_empty() {
        return bytes.ends_with(b"/>");
    }
    let Some(rest) = bytes.strip_suffix(b">") else {
        return false;
    };
    // `</a >` is legal, so trim the whitespace an end tag is allowed to carry.
    let rest = trim_end_ascii_whitespace(rest);
    let Some(rest) = strip_qname_suffix(doc, &element.name, rest) else {
        return false;
    };
    rest.ends_with(b"</")
}

fn strip_qname_suffix<'b>(doc: &RawDocument, name: &RawName, bytes: &'b [u8]) -> Option<&'b [u8]> {
    let rest = bytes.strip_suffix(doc.interner.resolve(name.local).as_bytes())?;
    match name.prefix {
        Some(prefix) => rest
            .strip_suffix(b":")?
            .strip_suffix(doc.interner.resolve(prefix).as_bytes()),
        None => Some(rest),
    }
}

fn trim_end_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let mut end = bytes.len();
    while end > 0 && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    &bytes[..end]
}

/// Advances `cursor` past `expected` if `bytes` continues with it.
fn consume(bytes: &[u8], cursor: &mut usize, expected: &[u8]) -> bool {
    if bytes.len() < *cursor + expected.len()
        || &bytes[*cursor..*cursor + expected.len()] != expected
    {
        return false;
    }
    *cursor += expected.len();
    true
}

fn write_qname(doc: &RawDocument, name: &RawName, out: &mut Vec<u8>) {
    if let Some(prefix) = name.prefix {
        out.extend_from_slice(doc.interner.resolve(prefix).as_bytes());
        out.push(b':');
    }
    out.extend_from_slice(doc.interner.resolve(name.local).as_bytes());
}

fn wrap(out: &mut Vec<u8>, open: &[u8], inner: &[u8], close: &[u8]) {
    out.extend_from_slice(open);
    out.extend_from_slice(inner);
    out.extend_from_slice(close);
}
