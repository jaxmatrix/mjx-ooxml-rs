//! Byte-preserving reader: tokenizes with quick-xml and builds a [`RawDocument`], preserving raw
//! escaped values, attribute order + quote style, prefixes, self-closing style, and the prologue /
//! epilogue. It never unescapes; it never trims text.

use mjx_ooxml_core::{
    Interner, QuoteStyle, RawAttribute, RawDocument, RawElement, RawName, RawNode, Symbol,
};
use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;

use crate::XmlError;

/// Parses XML bytes into the lossless preservation tree.
///
/// We rely on quick-xml's defaults: no text trimming, empty elements are *not* expanded (`<a/>`
/// stays distinct from `<a></a>`), and end-tag names are checked (well-formedness → typed error).
pub fn parse(input: &[u8]) -> Result<RawDocument, XmlError> {
    let (bom, bytes) = strip_bom(input);
    let mut reader = NsReader::from_reader(bytes);
    let mut interner = Interner::new();
    let mut buf = Vec::new();

    let mut stack: Vec<RawElement> = Vec::new();
    let mut prologue: Vec<RawNode> = Vec::new();
    let mut root: Option<RawElement> = None;
    let mut epilogue: Vec<RawNode> = Vec::new();

    loop {
        buf.clear();
        let (ns, event) = reader
            .read_resolved_event_into(&mut buf)
            .map_err(|e| XmlError::Syntax(e.to_string()))?;
        match event {
            Event::Start(e) => {
                let element = build_element(&mut interner, ns, &e, false)?;
                stack.push(element);
            }
            Event::Empty(e) => {
                let element = build_element(&mut interner, ns, &e, true)?;
                place(
                    &mut stack,
                    &mut prologue,
                    &mut root,
                    &mut epilogue,
                    RawNode::Element(element),
                )?;
            }
            Event::End(_) => {
                let element = stack
                    .pop()
                    .ok_or_else(|| XmlError::Syntax("unbalanced end tag".to_owned()))?;
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
                let node = RawNode::DocType(e.into_inner().into_owned().into_boxed_slice());
                place(&mut stack, &mut prologue, &mut root, &mut epilogue, node)?;
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
    Ok(RawDocument {
        interner,
        bom,
        prologue,
        root,
        epilogue,
    })
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
    stack: &mut [RawElement],
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
    ns: ResolveResult<'_>,
    e: &BytesStart<'_>,
    empty: bool,
) -> Result<RawElement, XmlError> {
    let namespace = resolve_namespace(ns, interner)?;
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

    Ok(RawElement {
        name,
        attributes,
        children: Vec::new(),
        empty,
    })
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
