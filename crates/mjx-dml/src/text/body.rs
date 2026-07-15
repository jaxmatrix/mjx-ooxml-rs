//! `a:txBody` — a text body.

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use super::is_dml;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextBody {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
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

impl FromXml for TextBody {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut content = Vec::with_capacity(element.children.len());
        for child in &element.children {
            if let RawNode::Element(child_element) = child {
                if is_dml(&child_element.name, interner, "p") {
                    content.push(TextBodyContent::Paragraph(Paragraph::from_xml(
                        child_element,
                        interner,
                    )?));
                    continue;
                }
            }
            content.push(TextBodyContent::Raw(child.clone()));
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            content,
        })
    }
}

impl ToXml for TextBody {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let mut children = Vec::with_capacity(self.content.len());
        for item in &self.content {
            match item {
                TextBodyContent::Paragraph(paragraph) => {
                    children.push(RawNode::Element(paragraph.to_xml(interner)));
                }
                TextBodyContent::Raw(node) => children.push(node.clone()),
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
