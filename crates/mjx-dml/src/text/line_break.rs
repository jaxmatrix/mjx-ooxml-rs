//! `a:br` — a line break within a paragraph.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{RawAttribute, RawName, RawNode};

use super::character::CharacterProperties;

/// One ordered child of a [`TextLineBreak`]: its typed [`CharacterProperties`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineBreakContent {
    /// The break's `a:rPr` — the run properties the break carries.
    Properties(CharacterProperties),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:br` — a line break (`CT_TextLineBreak`): an optional `a:rPr` and nothing else.
///
/// A break is a paragraph child like a run, but it holds no text — it forces a new line and carries
/// only the run properties the line after it starts with. It is **not** reflected by
/// [`Paragraph::text`](super::paragraph::Paragraph::text), which concatenates run text alone.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TextLineBreak {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "rPr", variant = Properties, ty = CharacterProperties))]
    content: Vec<LineBreakContent>,
}

impl TextLineBreak {
    /// The break's character properties (`a:rPr`), or `None` if it declares none.
    #[must_use]
    pub fn properties(&self) -> Option<&CharacterProperties> {
        self.content.iter().find_map(|item| match item {
            LineBreakContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The break's ordered content (its typed `a:rPr` interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[LineBreakContent] {
        &self.content
    }
}
