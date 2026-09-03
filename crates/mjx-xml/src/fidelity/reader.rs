//! Byte-preserving reader: tokenizes with quick-xml and builds a [`RawDocument`], preserving raw
//! escaped values, attribute order + quote style, prefixes, self-closing style, and the prologue /
//! epilogue. It never unescapes; it never trims text.
//!
//! It also records, for every element, the byte range that element occupied in the input, and hands
//! the input buffer to the document. That is what lets [`serialize`](super::serialize) copy an
//! unmodified subtree verbatim instead of rebuilding it — the properties a decomposed tree does not
//! record (whitespace between attributes, whitespace before `/>`) survive because the bytes do.
//!
//! quick-xml's events tile the input exactly: the position after one event is the position of the
//! next, so an element's range is *the position before its start-tag event* through *the position
//! after its end-tag event*.

use std::sync::Arc;

use mjx_ooxml_core::{
    Interner, QuoteStyle, RawAttribute, RawDocument, RawElement, RawName, RawNode, Symbol,
};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;

use crate::XmlError;

/// The deepest element nesting [`parse`] will build a tree for.
///
/// # Why the reader is where this belongs
///
/// The reader itself is iterative and would happily build any depth. **The cost is paid by everyone
/// who later walks the tree**, and those walks are recursive because the data is: `Drop` and `Clone`
/// for [`RawNode`] are compiler-generated and recursive, [`serialize`](super::serialize) descends
/// into a dirty element's children, and `mjx_mce::resolve` descends the whole document. None of them
/// takes an attacker's input directly, so none of them is the place to check; all of them receive a
/// tree that *this function* built out of untrusted bytes. One bound here bounds every one of them,
/// including the ones Phase C and D have not written yet.
///
/// # Why 256 — measured from both ends, not guessed
///
/// **From above.** The deepest part in the committed fixture corpus is **depth 13**
/// (`layouts.pptx :: ppt/slides/slide2.xml`), so this is nearly twenty times the deepest markup
/// PowerPoint, Word and Excel actually write.
///
/// **From below.** The worst walk is `mjx_mce::resolve`, and the worst configuration is a debug
/// build on a 2 MiB thread — the default for a spawned thread, which is where a library gets called
/// from. Measured there, depth 768 completes and depth 1,024 overflows. On an optimised build with
/// the main thread's 8 MiB, the overflow moves out to between 10,000 and 20,000. This limit is a
/// third of the *smallest* measured survivor, which leaves roughly three quarters of a 2 MiB stack
/// for the caller who was already on it.
///
/// The bound has to be small because the walks cannot be made iterative. `Drop` and `Clone` for
/// [`RawNode`] are compiler-generated, and giving [`RawElement`] a hand-written `Drop` would forbid
/// the partial moves this crate and `mjx-opc` both rely on. Bounding the tree at the one point where
/// untrusted bytes become one is the fix that needs no cooperation from anybody downstream.
///
/// A document deeper than this is refused with [`XmlError::DepthLimit`]. That is the *stricter*
/// answer, not the looser one: nothing that used to be accepted is now mis-parsed, and no byte that
/// used to round-trip round-trips differently. Raising it is one constant — and the measurement
/// above is what has to be redone with it.
pub const MAXIMUM_DEPTH: usize = 256;

/// Parses XML bytes into the lossless preservation tree.
///
/// The input is copied into a buffer the document retains, so unmodified subtrees can later be
/// serialized straight from those bytes. Use [`parse_shared`] to hand over a buffer you already hold
/// and skip the copy.
///
/// We rely on quick-xml's defaults: no text trimming, empty elements are *not* expanded (`<a/>`
/// stays distinct from `<a></a>`), and end-tag names are checked (well-formedness → typed error).
///
/// # Errors
///
/// Returns [`XmlError`] if the input is not well-formed XML (unbalanced tags, no root element,
/// malformed attributes, or non-UTF-8 element/attribute names).
///
/// # Examples
///
/// ```
/// use mjx_xml::fidelity;
/// let doc = fidelity::parse(br#"<w:p xmlns:w="urn:w"><w:r>text</w:r></w:p>"#).unwrap();
/// assert_eq!(doc.interner.resolve(doc.root.name.local), "p");
/// ```
pub fn parse(input: &[u8]) -> Result<RawDocument, XmlError> {
    parse_shared(Arc::from(input))
}

/// Parses XML bytes the caller already holds in a shared buffer, without copying them.
///
/// The document keeps `source` alive and records byte ranges into it, so a package that retains a
/// part's bytes *and* its tree pays for one buffer rather than two.
///
/// # Errors
///
/// As [`parse`].
///
/// # Examples
///
/// ```
/// use std::sync::Arc;
/// use mjx_xml::fidelity;
///
/// let bytes: Arc<[u8]> = Arc::from(&b"<a:p xmlns:a=\"urn:a\"/>"[..]);
/// let doc = fidelity::parse_shared(Arc::clone(&bytes)).unwrap();
/// assert_eq!(doc.source(), Some(&bytes[..]));
/// ```
pub fn parse_shared(source: Arc<[u8]>) -> Result<RawDocument, XmlError> {
    // The tokenizer borrows the buffer; the document owns it. Keep the borrow inside its own call so
    // the `Arc` is free to move afterwards.
    let parts = tokenize(&source)?;
    Ok(RawDocument::parsed(
        parts.interner,
        parts.bom,
        parts.prologue,
        parts.root,
        parts.epilogue,
        source,
    ))
}

/// Everything [`parse_shared`] recovers from the bytes, before the buffer itself is handed over.
struct ParsedParts {
    interner: Interner,
    bom: bool,
    prologue: Vec<RawNode>,
    root: RawElement,
    epilogue: Vec<RawNode>,
}

fn tokenize(source: &[u8]) -> Result<ParsedParts, XmlError> {
    let (bom, body) = strip_bom(source);
    // Ranges index the whole buffer, byte-order mark included, so the document and its spans agree
    // on one coordinate system.
    let base = source.len() - body.len();
    // A part larger than 4 GiB cannot be addressed by a `u32` range; it still parses, it just never
    // serializes verbatim.
    let spans_fit = u32::try_from(source.len()).is_ok();

    let mut reader = NsReader::from_reader(body);
    let mut interner = Interner::new();
    let mut buf = Vec::new();

    let mut stack: Vec<OpenElement> = Vec::new();
    let mut prologue: Vec<RawNode> = Vec::new();
    let mut root: Option<RawElement> = None;
    let mut epilogue: Vec<RawNode> = Vec::new();
    let mut cursor = 0usize;

    loop {
        buf.clear();
        let (ns, event) = reader
            .read_resolved_event_into(&mut buf)
            .map_err(|e| XmlError::Syntax(e.to_string()))?;
        // Resolve the namespace immediately: it is the only thing borrowing the reader, and the
        // reader has to be readable again for its byte position.
        let namespace = match &event {
            Event::Start(_) | Event::Empty(_) => resolve_namespace(ns, &mut interner)?,
            _ => None,
        };
        let start = base + cursor;
        cursor = usize::try_from(reader.buffer_position()).unwrap_or(cursor);
        let end = base + cursor;
        match event {
            Event::Start(e) => {
                if stack.len() >= MAXIMUM_DEPTH {
                    return Err(XmlError::DepthLimit {
                        limit: MAXIMUM_DEPTH,
                    });
                }
                let (name, attributes) = build_element(&mut interner, namespace, &e)?;
                stack.push(OpenElement {
                    name,
                    attributes,
                    children: Vec::new(),
                    start,
                });
            }
            Event::Empty(e) => {
                let (name, attributes) = build_element(&mut interner, namespace, &e)?;
                let element = finish(name, attributes, Vec::new(), true, start, end, spans_fit);
                place(
                    &mut stack,
                    &mut prologue,
                    &mut root,
                    &mut epilogue,
                    RawNode::Element(element),
                )?;
            }
            Event::End(_) => {
                let open = stack
                    .pop()
                    .ok_or_else(|| XmlError::Syntax("unbalanced end tag".to_owned()))?;
                let element = finish(
                    open.name,
                    open.attributes,
                    open.children,
                    false,
                    open.start,
                    end,
                    spans_fit,
                );
                place(
                    &mut stack,
                    &mut prologue,
                    &mut root,
                    &mut epilogue,
                    RawNode::Element(element),
                )?;
            }
            Event::Text(e) => {
                let node = RawNode::Text(e.into_inner().into_owned().into_boxed_slice());
                place(&mut stack, &mut prologue, &mut root, &mut epilogue, node)?;
            }
            Event::CData(e) => {
                let node = RawNode::CData(e.into_inner().into_owned().into_boxed_slice());
                place(&mut stack, &mut prologue, &mut root, &mut epilogue, node)?;
            }
            Event::Comment(e) => {
                let node = RawNode::Comment(e.into_inner().into_owned().into_boxed_slice());
                place(&mut stack, &mut prologue, &mut root, &mut epilogue, node)?;
            }
            Event::PI(e) => {
                let node = RawNode::ProcessingInstruction(Box::from(e.as_ref()));
                place(&mut stack, &mut prologue, &mut root, &mut epilogue, node)?;
            }
            Event::Decl(e) => {
                let node = RawNode::Declaration(Box::from(e.as_ref()));
                place(&mut stack, &mut prologue, &mut root, &mut epilogue, node)?;
            }
            Event::DocType(e) => {
                // quick-xml hands back the doctype's *content* with the whitespace after
                // `<!DOCTYPE` already trimmed, so rebuilding `<!DOCTYPE` + content + `>` would eat a
                // byte. Now that every event's range is known, take the inner bytes out of the
                // source instead and the wrapper the writer adds is exact.
                let verbatim = source
                    .get(start..end)
                    .and_then(|raw| raw.strip_prefix(&b"<!DOCTYPE"[..]))
                    .and_then(|raw| raw.strip_suffix(&b">"[..]));
                let inner: Box<[u8]> = match verbatim {
                    Some(bytes) => Box::from(bytes),
                    None => e.into_inner().into_owned().into_boxed_slice(),
                };
                place(
                    &mut stack,
                    &mut prologue,
                    &mut root,
                    &mut epilogue,
                    RawNode::DocType(inner),
                )?;
            }
            Event::Eof => break,
        }
    }

    if !stack.is_empty() {
        return Err(XmlError::Syntax(
            "unclosed element at end of input".to_owned(),
        ));
    }
    let root = root.ok_or_else(|| XmlError::Syntax("document has no root element".to_owned()))?;
    Ok(ParsedParts {
        interner,
        bom,
        prologue,
        root,
        epilogue,
    })
}

/// An element whose start tag has been read and whose end tag has not.
struct OpenElement {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    /// Offset of this element's `<`, in the whole source buffer.
    start: usize,
}

/// Builds the finished element, recording its byte range when the range is representable and looks
/// like a start tag. A range we cannot vouch for is simply not recorded: the element then
/// serializes from the model, which is always correct and only ever reflows.
fn finish(
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
    start: usize,
    end: usize,
    spans_fit: bool,
) -> RawElement {
    if spans_fit && start < end {
        if let (Ok(start), Ok(end)) = (u32::try_from(start), u32::try_from(end)) {
            return RawElement::parsed(name, attributes, children, empty, start..end);
        }
    }
    RawElement::new(name, attributes, children, empty)
}

fn strip_bom(input: &[u8]) -> (bool, &[u8]) {
    match input {
        [0xEF, 0xBB, 0xBF, rest @ ..] => (true, rest),
        _ => (false, input),
    }
}

/// Places a finished node: as a child of the open element if any, else into prologue/epilogue, or as
/// the root element itself.
fn place(
    stack: &mut [OpenElement],
    prologue: &mut Vec<RawNode>,
    root: &mut Option<RawElement>,
    epilogue: &mut Vec<RawNode>,
    node: RawNode,
) -> Result<(), XmlError> {
    if let Some(top) = stack.last_mut() {
        top.children.push(node);
        return Ok(());
    }
    match node {
        RawNode::Element(element) => {
            if root.is_some() {
                return Err(XmlError::Syntax("multiple root elements".to_owned()));
            }
            *root = Some(element);
        }
        other => {
            if root.is_none() {
                prologue.push(other);
            } else {
                epilogue.push(other);
            }
        }
    }
    Ok(())
}

fn build_element(
    interner: &mut Interner,
    namespace: Option<Symbol>,
    e: &BytesStart<'_>,
) -> Result<(RawName, Vec<RawAttribute>), XmlError> {
    let qname = e.name();
    let name = intern_qname(interner, qname.as_ref(), namespace)?;

    let mut attributes = Vec::new();
    for scanned in scan_attributes(e.attributes_raw())? {
        let attr_name = intern_qname(interner, &scanned.name, None)?;
        attributes.push(RawAttribute {
            name: attr_name,
            value: scanned.value.into_boxed_slice(),
            quote: scanned.quote,
        });
    }

    Ok((name, attributes))
}

fn resolve_namespace(
    ns: ResolveResult<'_>,
    interner: &mut Interner,
) -> Result<Option<Symbol>, XmlError> {
    match ns {
        ResolveResult::Bound(namespace) => {
            let uri = std::str::from_utf8(namespace.as_ref())?;
            Ok(Some(interner.intern(uri)))
        }
        ResolveResult::Unbound | ResolveResult::Unknown(_) => Ok(None),
    }
}

fn intern_qname(
    interner: &mut Interner,
    raw: &[u8],
    namespace: Option<Symbol>,
) -> Result<RawName, XmlError> {
    let text = std::str::from_utf8(raw)?;
    let (prefix, local) = match text.split_once(':') {
        Some((p, l)) => (Some(interner.intern(p)), interner.intern(l)),
        None => (None, interner.intern(text)),
    };
    Ok(RawName {
        prefix,
        local,
        namespace,
    })
}

/// One attribute recovered from the raw attribute region.
struct ScannedAttribute {
    name: Vec<u8>,
    quote: QuoteStyle,
    value: Vec<u8>,
}

/// Scans the raw attribute region (`e.attributes_raw()`) into name/quote/value triples, preserving
/// the raw escaped value and the quote character.
fn scan_attributes(raw: &[u8]) -> Result<Vec<ScannedAttribute>, XmlError> {
    let mut out = Vec::new();
    let mut i = 0;
    let n = raw.len();
    let malformed = || XmlError::Syntax("malformed attribute".to_owned());

    loop {
        while i < n && raw[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= n {
            break;
        }
        let name_start = i;
        while i < n && raw[i] != b'=' && !raw[i].is_ascii_whitespace() {
            i += 1;
        }
        let name = raw[name_start..i].to_vec();
        if name.is_empty() {
            return Err(malformed());
        }
        while i < n && raw[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= n || raw[i] != b'=' {
            return Err(malformed());
        }
        i += 1; // consume '='
        while i < n && raw[i].is_ascii_whitespace() {
            i += 1;
        }
        let quote = match raw.get(i) {
            Some(b'"') => QuoteStyle::Double,
            Some(b'\'') => QuoteStyle::Single,
            _ => return Err(malformed()),
        };
        i += 1;
        let value_start = i;
        let quote_byte = quote.byte();
        while i < n && raw[i] != quote_byte {
            i += 1;
        }
        if i >= n {
            return Err(malformed());
        }
        let value = raw[value_start..i].to_vec();
        i += 1; // consume closing quote
        out.push(ScannedAttribute { name, quote, value });
    }
    Ok(out)
}
