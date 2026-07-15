//! `a:p` — a text paragraph.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};

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
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct Paragraph {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "r", variant = Run, ty = TextRun))]
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
