//! `a:txBody` — a text body.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};

use super::paragraph::Paragraph;

/// One ordered child of a [`TextBody`]: a typed [`Paragraph`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextBodyContent {
    /// A text paragraph (`a:p`).
    Paragraph(Paragraph),
    /// Any other child — `a:bodyPr`, `a:lstStyle`, whitespace, or an unknown element — preserved
    /// verbatim.
    Raw(RawNode),
}

/// `CT_TextBody` — the text body of a shape.
///
/// Per the schema its children are `a:bodyPr` (required), an optional `a:lstStyle`, then one or more
/// `a:p`. Only the paragraphs are typed; `a:bodyPr` / `a:lstStyle` (and anything unknown) are kept
/// opaque so the body round-trips.
///
/// The element's tag and prefix are context-dependent — a slide serializes this type as `p:txBody`
/// (presentationml), other containers as `a:txBody` — so [`from_xml`](mjx_ooxml_core::FromXml::from_xml)
/// does not check the element's own name; the caller decides that the element *is* a text body.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TextBody {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "p", variant = Paragraph, ty = Paragraph))]
    content: Vec<TextBodyContent>,
}

impl TextBody {
    /// The typed paragraphs (`a:p`) of this body, in order (opaque children are skipped).
    pub fn paragraphs(&self) -> impl Iterator<Item = &Paragraph> {
        self.content.iter().filter_map(|item| match item {
            TextBodyContent::Paragraph(paragraph) => Some(paragraph),
            TextBodyContent::Raw(_) => None,
        })
    }

    /// The typed paragraphs (`a:p`), mutably, in order (opaque children are skipped).
    pub fn paragraphs_mut(&mut self) -> impl Iterator<Item = &mut Paragraph> {
        self.content.iter_mut().filter_map(|item| match item {
            TextBodyContent::Paragraph(paragraph) => Some(paragraph),
            TextBodyContent::Raw(_) => None,
        })
    }

    /// The body's text: each paragraph's text joined by a newline (`\n`).
    #[must_use]
    pub fn text(&self) -> String {
        self.paragraphs()
            .map(Paragraph::text)
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The body's ordered content (typed paragraphs interleaved with opaque nodes such as `a:bodyPr`).
    #[must_use]
    pub fn content(&self) -> &[TextBodyContent] {
        &self.content
    }
}
