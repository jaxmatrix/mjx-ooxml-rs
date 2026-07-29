//! `a:fld` — a text field: generated text (a slide number, a date) with a cached rendering.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::attr_str;

use super::character::CharacterProperties;
use super::paragraph_properties::ParagraphProperties;
use super::run::Text;

/// One ordered child of a [`TextField`]: its typed `a:rPr` / `a:pPr` / `a:t`, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldContent {
    /// The field's `a:rPr` — how its cached text looks.
    Properties(CharacterProperties),
    /// The field's `a:pPr` — the paragraph properties its text takes.
    ParagraphProperties(ParagraphProperties),
    /// The field's `a:t` — the cached rendering of the generated text.
    Text(Text),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:fld` — a text field (`CT_TextField`): generated text such as a slide number or a date.
///
/// A field is a paragraph child like a run. It names a `type` (what it generates — `slidenum`,
/// `datetime`, …) and an `id`, and carries a **cached** rendering in its `a:t`: the text the producer
/// last computed, which is what [`text`](Self::text) reads. Its text is **not** reflected by
/// [`Paragraph::text`](super::paragraph::Paragraph::text), which concatenates run text alone.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TextField {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rPr", variant = Properties, ty = CharacterProperties),
        child(local = "pPr", variant = ParagraphProperties, ty = ParagraphProperties),
        child(local = "t", variant = Text, ty = Text)
    )]
    content: Vec<FieldContent>,
}

impl TextField {
    /// The field's identifier (`@id`, an `ST_Guid`), or `None` if it declares none — a well-formed
    /// field always does, so `None` signals malformed markup.
    #[must_use]
    pub fn id<'a>(&'a self, interner: &Interner) -> Option<&'a str> {
        attr_str(&self.attributes, interner, "id")
    }

    /// What the field generates (`@type`, e.g. `slidenum` or `datetime`), or `None` if it names none.
    #[must_use]
    pub fn field_type<'a>(&'a self, interner: &Interner) -> Option<&'a str> {
        attr_str(&self.attributes, interner, "type")
    }

    /// The field's cached text (the content of its `a:t`), or `""` if it has none. This is the last
    /// rendering the producer computed, not a live value.
    #[must_use]
    pub fn text(&self) -> &str {
        self.content
            .iter()
            .find_map(|item| match item {
                FieldContent::Text(text) => Some(text.text()),
                _ => None,
            })
            .unwrap_or("")
    }

    /// The field's character properties (`a:rPr`), or `None` if it declares none.
    #[must_use]
    pub fn properties(&self) -> Option<&CharacterProperties> {
        self.content.iter().find_map(|item| match item {
            FieldContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The field's paragraph properties (`a:pPr`), or `None` if it declares none.
    #[must_use]
    pub fn paragraph_properties(&self) -> Option<&ParagraphProperties> {
        self.content.iter().find_map(|item| match item {
            FieldContent::ParagraphProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The field's ordered content (its typed `a:rPr` / `a:pPr` / `a:t` interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[FieldContent] {
        &self.content
    }
}
