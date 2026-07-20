//! `a:r` (a text run) and its `a:t` (the run's text).

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use super::character::{CharacterProperties, CharacterPropertiesSpec};

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

    /// Replaces the text content. It is re-escaped (minimally) when the run is serialized.
    pub fn set_text(&mut self, text: &str) {
        self.text = text.to_owned();
    }
}

/// One ordered child of a [`TextRun`]: its typed [`CharacterProperties`] or [`Text`], or an opaque
/// node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunContent {
    /// The run's `a:rPr` — how the text looks.
    Properties(CharacterProperties),
    /// The run's `a:t` text element.
    Text(Text),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:r` — a regular text run (`CT_RegularTextRun`): an optional `a:rPr` followed by the required
/// `a:t`.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TextRun {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "rPr", variant = Properties, ty = CharacterProperties),
        child(local = "t", variant = Text, ty = Text)
    )]
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
                _ => None,
            })
            .unwrap_or("")
    }

    /// The run's character properties (`a:rPr`), or `None` if it declares none — in which case every
    /// property is inherited (from the paragraph, the placeholder, the layout, the master, the theme).
    #[must_use]
    pub fn properties(&self) -> Option<&CharacterProperties> {
        self.content.iter().find_map(|item| match item {
            RunContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The run's character properties (`a:rPr`), mutably, or `None` if it declares none.
    pub fn properties_mut(&mut self) -> Option<&mut CharacterProperties> {
        self.content.iter_mut().find_map(|item| match item {
            RunContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// Applies `spec` to the run's character properties, creating the `a:rPr` if the run has none.
    ///
    /// An existing `a:rPr` is **merged** onto, so the state this model does not describe — `lang`,
    /// `dirty`, a hyperlink — survives (see [`CharacterProperties::apply`]). A created one is placed
    /// first, where `CT_RegularTextRun` requires it.
    pub fn set_properties(&mut self, spec: &CharacterPropertiesSpec, interner: &mut Interner) {
        if let Some(properties) = self.properties_mut() {
            properties.apply(spec, interner);
            return;
        }
        let properties = spec.to_properties(interner, "rPr");
        self.content.insert(0, RunContent::Properties(properties));
        self.empty = false;
    }

    /// The run's ordered content (its typed `a:rPr` and `a:t` interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[RunContent] {
        &self.content
    }

    /// Sets the run's `a:t` text, re-escaped on write.
    ///
    /// Returns `false` (and changes nothing) if the run has no `a:t` child. A well-formed
    /// `CT_RegularTextRun` always has one, so `false` signals a malformed or non-text run — a fresh
    /// `a:t` is deliberately **not** synthesized (that would require interning a new element name).
    pub fn set_text(&mut self, text: &str) -> bool {
        for item in &mut self.content {
            if let RunContent::Text(run_text) = item {
                run_text.set_text(text);
                return true;
            }
        }
        false
    }
}
