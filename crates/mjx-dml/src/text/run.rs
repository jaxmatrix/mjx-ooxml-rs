//! `a:r` (a text run) and its `a:t` (the run's text).

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};

/// `a:t` — the literal text of a run.
///
/// The `t` element of `CT_RegularTextRun` is an `xsd:string`: text content only, no child elements or
/// modeled attributes. [`Text`] stores the **decoded** string (entities unescaped, `CDATA` taken
/// literally); on write the string is re-escaped minimally (only `<` and `&`). Any attributes (e.g.
/// `xml:space="preserve"`) and the self-closing flag are preserved verbatim, and the reader never
/// trims whitespace, so significant spaces survive.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
pub struct Text {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(text)]
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
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TextRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "t", variant = Text, ty = Text))]
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
