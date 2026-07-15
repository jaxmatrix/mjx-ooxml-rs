//! `a:r` (a text run) and its `a:t` (the run's text).

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};
use mjx_xml::text::{escape_text, unescape_text};

use super::is_dml;

/// `a:t` — the literal text of a run.
///
/// The `t` element of `CT_RegularTextRun` is an `xsd:string`: text content only, no child elements or
/// modeled attributes. [`Text`] stores the **decoded** string (entities unescaped, `CDATA` taken
/// literally); on write the string is re-escaped minimally (only `<` and `&`). Any attributes (e.g.
/// `xml:space="preserve"`) and the self-closing flag are preserved verbatim, and the reader never
/// trims whitespace, so significant spaces survive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Text {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    text: String,
}

impl Text {
    /// The decoded text content (entities resolved).
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The element's attributes, verbatim (e.g. `xml:space`).
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }
}

impl FromXml for Text {
    fn from_xml(element: &RawElement, _interner: &Interner) -> Result<Self, FromXmlError> {
        let mut text = String::new();
        for child in &element.children {
            match child {
                RawNode::Text(bytes) => {
                    let raw = std::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?;
                    let decoded = unescape_text(raw)
                        .map_err(|e| FromXmlError::InvalidEntity(e.to_string()))?;
                    text.push_str(&decoded);
                }
                // CDATA content is literal (not entity-encoded).
                RawNode::CData(bytes) => {
                    let raw = std::str::from_utf8(bytes).map_err(|_| FromXmlError::InvalidUtf8)?;
                    text.push_str(raw);
                }
                // `a:t` is `xsd:string`; element/comment children are not valid content and are
                // dropped (the same limitation the derived `#[xml(text)]` will carry).
                _ => {}
            }
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            text,
        })
    }
}

impl ToXml for Text {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        let mut children = Vec::new();
        if !self.text.is_empty() {
            let escaped = escape_text(&self.text);
            children.push(RawNode::Text(escaped.as_bytes().into()));
        }
        let empty = self.empty && children.is_empty();
        RawElement {
            name: self.name,
            attributes: self.attributes.clone(),
            children,
            empty,
        }
    }
}

/// One ordered child of a [`TextRun`]: the typed [`Text`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunContent {
    /// The run's `a:t` text element.
    Text(Text),
    /// Any other child — `a:rPr`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:r` — a regular text run (`CT_RegularTextRun`): an optional `a:rPr` (kept opaque) followed by the
/// required `a:t`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    content: Vec<RunContent>,
}

impl TextRun {
    /// The run's text (the content of its `a:t`), or `""` if it has none.
    #[must_use]
    pub fn text(&self) -> &str {
        self.content
            .iter()
            .find_map(|item| match item {
                RunContent::Text(text) => Some(text.text()),
                RunContent::Raw(_) => None,
            })
            .unwrap_or("")
    }

    /// The run's ordered content (its typed `a:t` interleaved with opaque nodes such as `a:rPr`).
    #[must_use]
    pub fn content(&self) -> &[RunContent] {
        &self.content
    }
}

impl FromXml for TextRun {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut content = Vec::with_capacity(element.children.len());
        for child in &element.children {
            if let RawNode::Element(child_element) = child {
                if is_dml(&child_element.name, interner, "t") {
                    content.push(RunContent::Text(Text::from_xml(child_element, interner)?));
                    continue;
                }
            }
            content.push(RunContent::Raw(child.clone()));
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            content,
        })
    }
}

impl ToXml for TextRun {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let mut children = Vec::with_capacity(self.content.len());
        for item in &self.content {
            match item {
                RunContent::Text(text) => children.push(RawNode::Element(text.to_xml(interner))),
                RunContent::Raw(node) => children.push(node.clone()),
            }
        }
        let empty = self.empty && children.is_empty();
        RawElement {
            name: self.name,
            attributes: self.attributes.clone(),
            children,
            empty,
        }
    }
}
