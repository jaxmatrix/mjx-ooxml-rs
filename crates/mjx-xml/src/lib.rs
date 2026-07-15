//! `mjx-xml` — the XML layer for mjx-ooxml-rs.
//!
//! This crate is the **only** place `quick-xml` is used, so the XML backend stays swappable and the
//! rest of the workspace depends on our own stable types. It exposes two readers for two jobs.
//!
//! # [`fidelity`] — byte-preserving parse/serialize
//!
//! [`fidelity::parse`] turns a part's bytes into a [`mjx_ooxml_core::RawDocument`] tree and
//! [`fidelity::serialize`] turns it back. Values and text are kept as raw escaped bytes and the
//! writer is hand-written, so clean Office XML round-trips **byte-for-byte** (entities, attribute
//! order, prefixes, self-closing style, the declaration, and the trailing bytes are all preserved).
//! This is the reader the document model is built on.
//!
//! # [`Reader`] — a small control-part reader
//!
//! A namespace-resolving pull reader that unescapes values and returns owned [`Event`]s. It is used
//! for the tiny OPC control parts (`[Content_Types].xml`, `_rels/*.rels`) and by the schema codegen.
//! It is *not* byte-preserving — use [`fidelity`] for document parts.
//!
//! # Example — byte-preserving round-trip
//!
//! ```
//! let xml = br#"<a:p xmlns:a="urn:a"><a:r>hi &amp; bye</a:r></a:p>"#;
//! let doc = mjx_xml::fidelity::parse(xml).unwrap();
//! let out = mjx_xml::fidelity::serialize_to_vec(&doc);
//! assert_eq!(out, xml); // identical bytes — entities, prefixes, and structure preserved
//! ```

pub mod fidelity;

use std::borrow::Cow;

use quick_xml::events::Event as QxEvent;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;

/// Errors produced while reading XML.
#[derive(Debug, thiserror::Error)]
pub enum XmlError {
    /// The document was not well-formed, or the underlying reader failed.
    #[error("XML syntax error: {0}")]
    Syntax(String),
    /// A byte sequence that should have been UTF-8 text was not.
    #[error("invalid UTF-8 in XML: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

/// A resolved element name: its namespace URI (if bound) and local name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Name {
    /// The bound namespace URI, or `None` if the element is in no namespace.
    pub namespace: Option<String>,
    /// The local (unprefixed) name.
    pub local: String,
}

impl Name {
    /// Returns `true` if this name has the given namespace URI and local name.
    #[must_use]
    pub fn is(&self, namespace: &str, local: &str) -> bool {
        self.local == local && self.namespace.as_deref() == Some(namespace)
    }
}

/// A single attribute (namespace declarations are filtered out by the reader).
///
/// Attribute names are stored as their local name; OOXML control parts use unqualified
/// attributes, so no prefix is retained here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The attribute's local name.
    pub name: String,
    /// The attribute's (entity-unescaped) value.
    pub value: String,
}

/// A start (or empty) element with its resolved name and attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The resolved element name.
    pub name: Name,
    /// The element's attributes, in document order.
    pub attributes: Vec<Attribute>,
}

impl Element {
    /// Returns the value of the attribute with the given local name, if present.
    #[must_use]
    pub fn attr(&self, local_name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.name == local_name)
            .map(|a| a.value.as_str())
    }

    /// The element's local name.
    #[must_use]
    pub fn local(&self) -> &str {
        &self.name.local
    }
}

/// A pull-parsing event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// `<tag ...>` — a start element.
    Start(Element),
    /// `<tag ... />` — an empty element.
    Empty(Element),
    /// `</tag>` — an end element.
    End(Name),
    /// Character data between elements (entity-unescaped).
    Text(String),
    /// End of input.
    Eof,
}

/// A namespace-resolving XML pull reader over an in-memory byte slice.
#[derive(Debug)]
pub struct Reader<'i> {
    inner: NsReader<&'i [u8]>,
    buf: Vec<u8>,
}

impl<'i> Reader<'i> {
    /// Creates a reader over the given XML bytes.
    #[must_use]
    pub fn new(input: &'i [u8]) -> Self {
        Self {
            inner: NsReader::from_reader(input),
            buf: Vec::new(),
        }
    }

    /// Reads the next meaningful event, skipping the XML declaration, comments,
    /// processing instructions, and doctype.
    pub fn read(&mut self) -> Result<Event, XmlError> {
        loop {
            self.buf.clear();
            let (ns, ev) = self
                .inner
                .read_resolved_event_into(&mut self.buf)
                .map_err(|e| XmlError::Syntax(e.to_string()))?;
            match ev {
                QxEvent::Start(ref e) => return Ok(Event::Start(build_element(ns, e)?)),
                QxEvent::Empty(ref e) => return Ok(Event::Empty(build_element(ns, e)?)),
                QxEvent::End(ref e) => {
                    return Ok(Event::End(Name {
                        namespace: resolve_ns(ns)?,
                        local: local_name(e.local_name().as_ref())?,
                    }))
                }
                QxEvent::Text(e) => {
                    let text = e.unescape().map_err(|e| XmlError::Syntax(e.to_string()))?;
                    // Skip pure inter-element whitespace; it is not meaningful for OPC parts.
                    if text.trim().is_empty() {
                        continue;
                    }
                    return Ok(Event::Text(text.into_owned()));
                }
                QxEvent::CData(e) => {
                    let bytes = e.into_inner();
                    return Ok(Event::Text(std::str::from_utf8(&bytes)?.to_owned()));
                }
                QxEvent::Eof => return Ok(Event::Eof),
                // Declaration, Comment, PI, DocType — not meaningful for our parsing.
                _ => continue,
            }
        }
    }
}

fn resolve_ns(ns: ResolveResult<'_>) -> Result<Option<String>, XmlError> {
    match ns {
        ResolveResult::Bound(namespace) => {
            Ok(Some(std::str::from_utf8(namespace.as_ref())?.to_owned()))
        }
        ResolveResult::Unbound | ResolveResult::Unknown(_) => Ok(None),
    }
}

fn local_name(bytes: &[u8]) -> Result<String, XmlError> {
    Ok(std::str::from_utf8(bytes)?.to_owned())
}

fn build_element(
    ns: ResolveResult<'_>,
    e: &quick_xml::events::BytesStart<'_>,
) -> Result<Element, XmlError> {
    let name = Name {
        namespace: resolve_ns(ns)?,
        local: local_name(e.local_name().as_ref())?,
    };

    let mut attributes = Vec::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|e| XmlError::Syntax(e.to_string()))?;
        let key = attr.key.as_ref();
        // Drop namespace declarations (`xmlns` / `xmlns:*`).
        if key == b"xmlns" || key.starts_with(b"xmlns:") {
            continue;
        }
        let name = local_name(attr.key.local_name().as_ref())?;
        let value: Cow<'_, str> = attr
            .unescape_value()
            .map_err(|e| XmlError::Syntax(e.to_string()))?;
        attributes.push(Attribute {
            name,
            value: value.into_owned(),
        });
    }

    Ok(Element { name, attributes })
}

#[cfg(test)]
mod tests {
    use super::*;

    const CT_NS: &str = "http://schemas.openxmlformats.org/package/2006/content-types";

    #[test]
    fn reads_elements_attrs_and_namespace() {
        let xml = br#"<?xml version="1.0"?>
            <Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
              <Default Extension="rels" ContentType="app/rels"/>
              <Override PartName="/ppt/presentation.xml" ContentType="app/pml"/>
            </Types>"#;
        let mut r = Reader::new(xml);

        match r.read().unwrap() {
            Event::Start(e) => {
                assert!(e.name.is(CT_NS, "Types"));
            }
            other => panic!("expected Start(Types), got {other:?}"),
        }

        match r.read().unwrap() {
            Event::Empty(e) => {
                assert_eq!(e.local(), "Default");
                assert_eq!(e.attr("Extension"), Some("rels"));
                assert_eq!(e.attr("ContentType"), Some("app/rels"));
                assert_eq!(e.name.namespace.as_deref(), Some(CT_NS));
            }
            other => panic!("expected Empty(Default), got {other:?}"),
        }

        match r.read().unwrap() {
            Event::Empty(e) => {
                assert_eq!(e.local(), "Override");
                assert_eq!(e.attr("PartName"), Some("/ppt/presentation.xml"));
            }
            other => panic!("expected Empty(Override), got {other:?}"),
        }

        assert_eq!(
            r.read().unwrap(),
            Event::End(Name {
                namespace: Some(CT_NS.to_owned()),
                local: "Types".to_owned(),
            })
        );
        assert_eq!(r.read().unwrap(), Event::Eof);
    }

    #[test]
    fn unescapes_text_and_attrs() {
        let xml = br#"<a:t xmlns:a="urn:x" k="a &amp; b">1 &lt; 2</a:t>"#;
        let mut r = Reader::new(xml);
        match r.read().unwrap() {
            Event::Start(e) => assert_eq!(e.attr("k"), Some("a & b")),
            other => panic!("got {other:?}"),
        }
        assert_eq!(r.read().unwrap(), Event::Text("1 < 2".to_owned()));
    }
}
