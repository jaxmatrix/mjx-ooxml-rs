//! `a:p` — a text paragraph.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};

use super::character::CharacterProperties;
use super::run::TextRun;

/// One ordered child of a [`Paragraph`]: a typed [`TextRun`] or [`CharacterProperties`], or an opaque
/// node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphContent {
    /// A regular text run (`a:r`).
    Run(TextRun),
    /// The paragraph-mark properties (`a:endParaRPr`) — see
    /// [`end_properties`](Paragraph::end_properties).
    EndProperties(CharacterProperties),
    /// Any other child — `a:pPr`, `a:br`, `a:fld`, whitespace, or an unknown element — preserved
    /// verbatim.
    Raw(RawNode),
}

/// `a:p` — a text paragraph (`CT_TextParagraph`): an optional `a:pPr` (kept opaque for now), then a
/// run of `a:r` / `a:br` / `a:fld` children, then an optional `a:endParaRPr`. `a:r` and
/// `a:endParaRPr` are typed; the line-break (`a:br`) and field (`a:fld`) run kinds are preserved
/// opaquely and are **not** reflected by [`text`](Self::text).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct Paragraph {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "r", variant = Run, ty = TextRun),
        child(local = "endParaRPr", variant = EndProperties, ty = CharacterProperties)
    )]
    content: Vec<ParagraphContent>,
}

impl Paragraph {
    /// The typed runs (`a:r`) of this paragraph, in order (opaque children are skipped).
    pub fn runs(&self) -> impl Iterator<Item = &TextRun> {
        self.content.iter().filter_map(|item| match item {
            ParagraphContent::Run(run) => Some(run),
            _ => None,
        })
    }

    /// The typed runs (`a:r`), mutably, in order (opaque children are skipped).
    pub fn runs_mut(&mut self) -> impl Iterator<Item = &mut TextRun> {
        self.content.iter_mut().filter_map(|item| match item {
            ParagraphContent::Run(run) => Some(run),
            _ => None,
        })
    }

    /// The paragraph-mark properties (`a:endParaRPr`), or `None` if the paragraph declares none.
    ///
    /// This is how a paragraph with no runs still has a size: PowerPoint records what text *would*
    /// look like here, so an empty line keeps its height and a run typed into it inherits its format.
    #[must_use]
    pub fn end_properties(&self) -> Option<&CharacterProperties> {
        self.content.iter().find_map(|item| match item {
            ParagraphContent::EndProperties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The paragraph-mark properties (`a:endParaRPr`), mutably, or `None` if there are none.
    pub fn end_properties_mut(&mut self) -> Option<&mut CharacterProperties> {
        self.content.iter_mut().find_map(|item| match item {
            ParagraphContent::EndProperties(properties) => Some(properties),
            _ => None,
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
