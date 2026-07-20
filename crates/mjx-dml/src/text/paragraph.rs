//! `a:p` — a text paragraph.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use super::character::CharacterProperties;
use super::paragraph_properties::{ParagraphProperties, ParagraphPropertiesSpec};
use super::run::TextRun;

/// One ordered child of a [`Paragraph`]: a typed [`TextRun`] or [`CharacterProperties`], or an opaque
/// node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParagraphContent {
    /// The paragraph's layout properties (`a:pPr`).
    Properties(ParagraphProperties),
    /// A regular text run (`a:r`).
    Run(TextRun),
    /// The paragraph-mark properties (`a:endParaRPr`) — see
    /// [`end_properties`](Paragraph::end_properties).
    EndProperties(CharacterProperties),
    /// Any other child — `a:br`, `a:fld`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:p` — a text paragraph (`CT_TextParagraph`): an optional `a:pPr`, then a run of `a:r` / `a:br` /
/// `a:fld` children, then an optional `a:endParaRPr`. `a:pPr`, `a:r` and `a:endParaRPr` are typed;
/// the line-break (`a:br`) and field (`a:fld`) run kinds are preserved opaquely and are **not**
/// reflected by [`text`](Self::text).
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct Paragraph {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pPr", variant = Properties, ty = ParagraphProperties),
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

    /// The paragraph's layout properties (`a:pPr`), or `None` if it declares none — in which case
    /// every property is inherited, along the paragraph's indent level.
    #[must_use]
    pub fn properties(&self) -> Option<&ParagraphProperties> {
        self.content.iter().find_map(|item| match item {
            ParagraphContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The paragraph's layout properties (`a:pPr`), mutably, or `None` if it declares none.
    pub fn properties_mut(&mut self) -> Option<&mut ParagraphProperties> {
        self.content.iter_mut().find_map(|item| match item {
            ParagraphContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// Applies `spec` to the paragraph's layout properties, creating the `a:pPr` if it has none.
    ///
    /// An existing `a:pPr` is **merged** onto, so the state this model does not describe — the
    /// line-breaking attributes, a bullet — survives. A created one is placed first, where
    /// `CT_TextParagraph` requires it.
    pub fn set_properties(&mut self, spec: &ParagraphPropertiesSpec, interner: &mut Interner) {
        if let Some(properties) = self.properties_mut() {
            properties.apply(spec, interner);
            return;
        }
        let properties = spec.to_properties(interner, "pPr");
        self.content
            .insert(0, ParagraphContent::Properties(properties));
        self.empty = false;
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
