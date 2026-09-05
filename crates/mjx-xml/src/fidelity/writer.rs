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

use mjx_ooxml_core::{Interner, RawAttribute, RawDocument, RawElement, RawName, RawNode};

/// Everything the walk below reads off a document: the interner its names resolve through, and the
/// source buffer an unmodified element may still be copied from.
///
/// Split out from [`RawDocument`] so that [`serialize_element`] can serialize one element of a
/// document without a document — which is what a model that holds *rows* rather than a tree needs
/// (`mjx_sml::cells`, MJXOFF-95). It is `Copy` and two words wide, so passing it costs what passing
/// the reference cost.
#[derive(Clone, Copy)]
struct Context<'a> {
    interner: &'a Interner,
    source: Option<&'a [u8]>,
}

impl<'a> Context<'a> {
    fn of(doc: &'a RawDocument) -> Self {
        Self {
            interner: &doc.interner,
            source: doc.source(),
        }
    }
}

/// Serializes a document back to bytes, appending to `out`.
pub fn serialize(doc: &RawDocument, out: &mut Vec<u8>) {
    let ctx = Context::of(doc);
    if doc.bom {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    for node in doc.prologue.iter() {
        write_node(ctx, node, out);
    }
    write_element(ctx, &doc.root, out);
    for node in doc.epilogue.iter() {
        write_node(ctx, node, out);
    }
}

/// Serializes **one element** — start tag, children, end tag — appending to `out`.
///
/// Same rules as [`serialize`], applied to a subtree instead of a document: an element still in the
/// state a reader left it in is copied verbatim out of `source`, and everything else is written from
/// the model. `interner` must be the one the element's [`Symbol`](mjx_ooxml_core::Symbol)s came
/// from, and `source` the buffer its [source ranges](RawElement::source_span) were measured against
/// — pass `None` if the element was authored, or if the buffer is not to hand, and every element is
/// written from the model instead. A wrong buffer cannot produce wrong bytes: the range is checked
/// against the element it claims to describe before it is trusted, exactly as it is for a document.
///
/// This exists for a model that does **not** hold a tree. `mjx_sml::cells` stores a worksheet's rows
/// as packed records plus byte ranges, so when it has to write an unmodelled child back it has an
/// element and an interner but no [`RawDocument`] to put them in.
///
/// # Examples
///
/// ```
/// use mjx_xml::fidelity;
/// let doc = fidelity::parse(br#"<a:root xmlns:a="urn:a"><a:kid v="1"/></a:root>"#).unwrap();
/// let mjx_ooxml_core::RawNode::Element(kid) = &doc.root.children[0] else {
///     panic!("expected an element");
/// };
/// let mut out = Vec::new();
/// fidelity::serialize_element(kid, &doc.interner, doc.source(), &mut out);
/// assert_eq!(out, br#"<a:kid v="1"/>"#);
/// ```
pub fn serialize_element(
    element: &RawElement,
    interner: &Interner,
    source: Option<&[u8]>,
    out: &mut Vec<u8>,
) {
    write_element(Context { interner, source }, element, out);
}

/// Serializes **one node** — an element, text, a comment, a processing instruction — appending to
/// `out`.
///
/// [`serialize_element`] for anything that is not an element. Same rules, same arguments, same
/// reason to exist: a model that holds byte ranges rather than a tree has to be able to turn a node
/// it was handed into the bytes it will keep.
///
/// # Examples
///
/// ```
/// use mjx_xml::fidelity;
/// let doc = fidelity::parse(b"<root>text<!-- note --></root>").unwrap();
/// let mut out = Vec::new();
/// for node in doc.root.children.iter() {
///     fidelity::serialize_node(node, &doc.interner, doc.source(), &mut out);
/// }
/// assert_eq!(out, b"text<!-- note -->");
/// ```
pub fn serialize_node(
    node: &RawNode,
    interner: &Interner,
    source: Option<&[u8]>,
    out: &mut Vec<u8>,
) {
    write_node(Context { interner, source }, node, out);
}

/// Serializes an element's **start tag alone** — `<name attr="value" …>`, or `<name …/>` when
/// `self_closing` — appending to `out`.
///
/// # Why this exists beside [`serialize_element`]
///
/// [`serialize_element`] is for a model that holds no tree but does hold whole elements. A model
/// that holds no tree and whose *children are not elements either* cannot use it for its own
/// container: `mjx_sml::worksheet`'s frame writes `<worksheet …>`, then thirty-nine slots of which
/// one is `mjx_sml::cells`'s packed byte store rather than a [`RawElement`], then `</worksheet>`.
/// Without this it would have to assemble a `RawElement` holding the whole `sheetData` — the
/// hundreds of megabytes the packed store exists to avoid — purely to write one start tag.
///
/// No verbatim shortcut applies: a start tag on its own has no recorded range (a
/// [source span](RawElement::source_span) covers a whole element), so this always writes from the
/// model. Attributes come back in order with their original quoting and escaping; the whitespace
/// *between* them does not, which is the same property every rebuilt element has.
///
/// # Examples
///
/// ```
/// use mjx_xml::fidelity;
/// let doc = fidelity::parse(br#"<a:root xmlns:a="urn:a">text</a:root>"#).unwrap();
/// let mut out = Vec::new();
/// fidelity::serialize_start_tag(&doc.root.name, &doc.root.attributes, false, &doc.interner, &mut out);
/// out.extend_from_slice(b"text");
/// fidelity::serialize_end_tag(&doc.root.name, &doc.interner, &mut out);
/// assert_eq!(out, br#"<a:root xmlns:a="urn:a">text</a:root>"#);
/// ```
pub fn serialize_start_tag(
    name: &RawName,
    attributes: &[RawAttribute],
    self_closing: bool,
    interner: &Interner,
    out: &mut Vec<u8>,
) {
    let ctx = Context {
        interner,
        source: None,
    };
    out.push(b'<');
    write_qname(ctx, name, out);
    write_attributes(ctx, attributes, out);
    if self_closing {
        out.extend_from_slice(b"/>");
    } else {
        out.push(b'>');
    }
}

/// Serializes an element's **end tag alone** — `</name>` — appending to `out`.
///
/// The counterpart of [`serialize_start_tag`]; see there for why both exist.
pub fn serialize_end_tag(name: &RawName, interner: &Interner, out: &mut Vec<u8>) {
    let ctx = Context {
        interner,
        source: None,
    };
    out.extend_from_slice(b"</");
    write_qname(ctx, name, out);
    out.push(b'>');
}

/// Convenience: serialize into a fresh `Vec`.
#[must_use]
pub fn serialize_to_vec(doc: &RawDocument) -> Vec<u8> {
    let mut out = Vec::new();
    serialize(doc, &mut out);
    out
}

fn write_node(ctx: Context<'_>, node: &RawNode, out: &mut Vec<u8>) {
    match node {
        RawNode::Element(element) => write_element(ctx, element, out),
        RawNode::Text(bytes) => out.extend_from_slice(bytes),
        RawNode::CData(bytes) => wrap(out, b"<![CDATA[", bytes, b"]]>"),
        RawNode::Comment(bytes) => wrap(out, b"<!--", bytes, b"-->"),
        RawNode::ProcessingInstruction(bytes) => wrap(out, b"<?", bytes, b"?>"),
        RawNode::Declaration(bytes) => wrap(out, b"<?", bytes, b"?>"),
        RawNode::DocType(bytes) => wrap(out, b"<!DOCTYPE", bytes, b">"),
    }
}

fn write_element(ctx: Context<'_>, element: &RawElement, out: &mut Vec<u8>) {
    if let Some(verbatim) = verbatim_bytes(ctx, element) {
        out.extend_from_slice(verbatim);
        return;
    }
    out.push(b'<');
    write_qname(ctx, &element.name, out);
    write_attributes(ctx, &element.attributes, out);
    if element.empty && element.children.is_empty() {
        out.extend_from_slice(b"/>");
    } else {
        out.push(b'>');
        for child in element.children.iter() {
            write_node(ctx, child, out);
        }
        out.extend_from_slice(b"</");
        write_qname(ctx, &element.name, out);
        out.push(b'>');
    }
}

/// Every attribute, in order, declarations included — see the module docs.
fn write_attributes(ctx: Context<'_>, attributes: &[RawAttribute], out: &mut Vec<u8>) {
    for attr in attributes.iter() {
        out.push(b' ');
        write_qname(ctx, &attr.name, out);
        out.push(b'=');
        out.push(attr.quote.byte());
        out.extend_from_slice(&attr.value);
        out.push(attr.quote.byte());
    }
}

/// The source bytes `element` may be copied from, or `None` if it must be reconstructed.
///
/// Returns `Some` only when the document still holds its source buffer, the element has not been
/// mutated since it was parsed, the recorded range fits the buffer, and the bytes in that range
/// still describe *this* element — see the module docs for why the last check is not redundant.
fn verbatim_bytes<'d>(ctx: Context<'d>, element: &RawElement) -> Option<&'d [u8]> {
    let source = ctx.source?;
    let span = element.source_span()?;
    // `get` rejects an inverted or out-of-bounds range, so a nonsense span reflows rather than
    // panicking or reading someone else's markup.
    let bytes = source.get(usize::try_from(span.start).ok()?..usize::try_from(span.end).ok()?)?;
    if !opens_with(ctx, element, bytes) || !closes_as_stated(ctx, element, bytes) {
        return None;
    }
    Some(bytes)
}

/// Whether `bytes` starts `<` + the element's qualified name + a delimiter.
///
/// This is what catches a mutated [`RawElement::name`], which carries no mutation tracking.
fn opens_with(ctx: Context<'_>, element: &RawElement, bytes: &[u8]) -> bool {
    let mut cursor = 0usize;
    if bytes.first() != Some(&b'<') {
        return false;
    }
    cursor += 1;
    if let Some(prefix) = element.name.prefix {
        if !consume(bytes, &mut cursor, ctx.interner.resolve(prefix).as_bytes())
            || !consume(bytes, &mut cursor, b":")
        {
            return false;
        }
    }
    if !consume(
        bytes,
        &mut cursor,
        ctx.interner.resolve(element.name.local).as_bytes(),
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
fn closes_as_stated(ctx: Context<'_>, element: &RawElement, bytes: &[u8]) -> bool {
    if element.empty && element.children.is_empty() {
        return bytes.ends_with(b"/>");
    }
    let Some(rest) = bytes.strip_suffix(b">") else {
        return false;
    };
    // `</a >` is legal, so trim the whitespace an end tag is allowed to carry.
    let rest = trim_end_ascii_whitespace(rest);
    let Some(rest) = strip_qname_suffix(ctx, &element.name, rest) else {
        return false;
    };
    rest.ends_with(b"</")
}

fn strip_qname_suffix<'b>(ctx: Context<'_>, name: &RawName, bytes: &'b [u8]) -> Option<&'b [u8]> {
    let rest = bytes.strip_suffix(ctx.interner.resolve(name.local).as_bytes())?;
    match name.prefix {
        Some(prefix) => rest
            .strip_suffix(b":")?
            .strip_suffix(ctx.interner.resolve(prefix).as_bytes()),
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

fn write_qname(ctx: Context<'_>, name: &RawName, out: &mut Vec<u8>) {
    if let Some(prefix) = name.prefix {
        out.extend_from_slice(ctx.interner.resolve(prefix).as_bytes());
        out.push(b':');
    }
    out.extend_from_slice(ctx.interner.resolve(name.local).as_bytes());
}

fn wrap(out: &mut Vec<u8>, open: &[u8], inner: &[u8], close: &[u8]) {
    out.extend_from_slice(open);
    out.extend_from_slice(inner);
    out.extend_from_slice(close);
}
