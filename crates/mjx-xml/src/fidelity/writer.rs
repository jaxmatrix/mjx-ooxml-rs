//! Hand-written serializer for the preservation tree.
//!
//! quick-xml's writer is byte-faithful only when handed the original opaque element buffer, which
//! our decomposed/mutable tree deliberately does not keep. Writing the bytes ourselves gives total
//! control (quote char, one-space-per-attribute, self-closing style) and reproduces clean
//! Office/LibreOffice XML exactly.

use mjx_ooxml_core::{RawDocument, RawElement, RawName, RawNode};

/// Serializes a document back to bytes, appending to `out`.
pub fn serialize(doc: &RawDocument, out: &mut Vec<u8>) {
    if doc.bom {
        out.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    for node in &doc.prologue {
        write_node(doc, node, out);
    }
    write_element(doc, &doc.root, out);
    for node in &doc.epilogue {
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
    out.push(b'<');
    write_qname(doc, &element.name, out);
    for attr in &element.attributes {
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
        for child in &element.children {
            write_node(doc, child, out);
        }
        out.extend_from_slice(b"</");
        write_qname(doc, &element.name, out);
        out.push(b'>');
    }
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
