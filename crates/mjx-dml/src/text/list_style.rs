//! `CT_TextListStyle` — the paragraph properties a container offers at each of the nine indent
//! levels.
//!
//! One type covers every place a list style appears: a shape's own `a:lstStyle`, a placeholder's, and
//! each of the three styles in a slide master's `p:txStyles`. That matters because resolving what a
//! paragraph *actually* looks like walks all of them in turn, and walking one type at every tier is
//! what keeps that walk honest.
//!
//! # The off-by-one lives here
//!
//! A paragraph's level is `0..=8` ([`IndentLevel`]) but the elements are named `a:lvl1pPr` through
//! `a:lvl9pPr` — level 0 is `lvl1pPr`. [`TextListStyle::level`] is the only place in the codebase that
//! knows this, so no caller has to.

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawName, RawNode};

use crate::build::{dml_child, fidelity_element_impls};
use crate::geometry::IndentLevel;
use crate::text::paragraph_properties::ParagraphProperties;

/// `a:lstStyle` (`CT_TextListStyle`) — an optional default plus up to nine per-level paragraph
/// property sets.
///
/// A fidelity wrapper: levels are read on demand and everything is preserved verbatim, so a list style
/// this model only partly understands still round-trips.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextListStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

impl TextListStyle {
    /// The properties this style defines for `level`, or `None` if it defines none there.
    ///
    /// Level 0 reads `a:lvl1pPr`, level 8 reads `a:lvl9pPr`.
    #[must_use]
    pub fn level(&self, interner: &Interner, level: IndentLevel) -> Option<ParagraphProperties> {
        let local = format!("lvl{}pPr", level.value() + 1);
        dml_child(&self.children, interner, &local)
            .and_then(|element| ParagraphProperties::from_xml(element, interner).ok())
    }

    /// The style's default paragraph properties (`a:defPPr`), which apply where no level does, or
    /// `None` if it declares none.
    #[must_use]
    pub fn default_properties(&self, interner: &Interner) -> Option<ParagraphProperties> {
        dml_child(&self.children, interner, "defPPr")
            .and_then(|element| ParagraphProperties::from_xml(element, interner).ok())
    }

    /// Every level this style defines, shallowest first.
    pub fn levels<'a>(
        &'a self,
        interner: &'a Interner,
    ) -> impl Iterator<Item = (IndentLevel, ParagraphProperties)> + 'a {
        (0..=IndentLevel::DEEPEST)
            .map(IndentLevel::of)
            .filter_map(move |level| {
                self.level(interner, level)
                    .map(|properties| (level, properties))
            })
    }
}

fidelity_element_impls!(TextListStyle);
