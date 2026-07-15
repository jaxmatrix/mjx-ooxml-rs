//! `a:p` — a text paragraph.

use mjx_ooxml_core::{
    FromXml, FromXmlError, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml,
};

use super::is_dml;
use super::run::TextRun;

/// One ordered child of a [`Paragraph`]: a typed [`TextRun`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphContent {
    /// A regular text run (`a:r`).
    Run(TextRun),
    /// Any other child — `a:pPr`, `a:br`, `a:fld`, `a:endParaRPr`, whitespace, or an unknown element
    /// — preserved verbatim.
    Raw(RawNode),
}

/// `a:p` — a text paragraph (`CT_TextParagraph`): an optional `a:pPr` (kept opaque), then a run of
/// `a:r` / `a:br` / `a:fld` children, then an optional `a:endParaRPr` (also opaque). Only `a:r` is
/// typed; the line-break (`a:br`) and field (`a:fld`) run kinds are preserved opaquely and are **not**
/// reflected by [`text`](Self::text).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paragraph {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    content: Vec<ParagraphContent>,
}

impl Paragraph {
    /// The typed runs (`a:r`) of this paragraph, in order (opaque children are skipped).
    pub fn runs(&self) -> impl Iterator<Item = &TextRun> {
        self.content.iter().filter_map(|item| match item {
            ParagraphContent::Run(run) => Some(run),
            ParagraphContent::Raw(_) => None,
        })
    }

    /// The paragraph's text: the text of its runs concatenated with no separator. Opaque `a:br` line
    /// breaks and `a:fld` fields contribute nothing.
    #[must_use]
    pub fn text(&self) -> String {
        self.runs().map(TextRun::text).collect()
    }

    /// The paragraph's ordered content (typed runs interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[ParagraphContent] {
        &self.content
    }
}

impl FromXml for Paragraph {
    fn from_xml(element: &RawElement, interner: &Interner) -> Result<Self, FromXmlError> {
        let mut content = Vec::with_capacity(element.children.len());
        for child in &element.children {
            if let RawNode::Element(child_element) = child {
                if is_dml(&child_element.name, interner, "r") {
                    content.push(ParagraphContent::Run(TextRun::from_xml(
                        child_element,
                        interner,
                    )?));
                    continue;
                }
            }
            content.push(ParagraphContent::Raw(child.clone()));
        }
        Ok(Self {
            name: element.name,
            attributes: element.attributes.clone(),
            empty: element.empty,
            content,
        })
    }
}

impl ToXml for Paragraph {
    fn to_xml(&self, interner: &mut Interner) -> RawElement {
        let mut children = Vec::with_capacity(self.content.len());
        for item in &self.content {
            match item {
                ParagraphContent::Run(run) => children.push(RawNode::Element(run.to_xml(interner))),
                ParagraphContent::Raw(node) => children.push(node.clone()),
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
