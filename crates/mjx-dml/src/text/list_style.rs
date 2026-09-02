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
//! `a:lvl9pPr` — level 0 is `lvl1pPr`. [`level_local`] is the only place in the codebase that knows
//! this, so no caller has to.

use mjx_ooxml_core::{FromXml, Interner, RawAttribute, RawName, RawNode, ToXml as _};

use mjx_ooxml_types::child_order::TEXT_LIST_STYLE;

use crate::build::{dml_child, dml_name, fidelity_element_impls, is_dml};
use crate::geometry::IndentLevel;
use crate::text::paragraph_properties::{ParagraphProperties, ParagraphPropertiesSpec};

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
    /// An empty `a:lstStyle`, for a container that declares none.
    ///
    /// Every child of `CT_TextListStyle` is optional, so an empty one is valid — and states nothing,
    /// which is the honest starting point for a caller about to state one level.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dml_name(interner, "lstStyle"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        }
    }

    /// The properties this style defines for `level`, or `None` if it defines none there.
    ///
    /// Level 0 reads `a:lvl1pPr`, level 8 reads `a:lvl9pPr`.
    #[must_use]
    pub fn level(&self, interner: &Interner, level: IndentLevel) -> Option<ParagraphProperties> {
        dml_child(&self.children, interner, &level_local(level))
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

    /// Merges `spec` into the properties this style offers at `level`, creating `a:lvlNpPr` if it
    /// declares none.
    ///
    /// The properties **merge**, as a paragraph's own do: a property `spec` leaves unset is left
    /// where it was, not cleared. Remove the level with [`remove_level`](Self::remove_level) to drop
    /// what an old one carried.
    pub fn set_level(
        &mut self,
        interner: &mut Interner,
        level: IndentLevel,
        spec: &ParagraphPropertiesSpec,
    ) {
        self.merge_properties(interner, &level_local(level), spec);
    }

    /// Merges `spec` into the style's default paragraph properties (`a:defPPr`), creating them if it
    /// declares none. Merges as [`set_level`](Self::set_level) does.
    pub fn set_default_properties(
        &mut self,
        interner: &mut Interner,
        spec: &ParagraphPropertiesSpec,
    ) {
        self.merge_properties(interner, "defPPr", spec);
    }

    /// Removes what this style offers at `level`, returning whether it offered anything there.
    pub fn remove_level(&mut self, interner: &Interner, level: IndentLevel) -> bool {
        self.remove_child(interner, &level_local(level))
    }

    /// Removes the style's default paragraph properties (`a:defPPr`), returning whether it had any.
    pub fn remove_default_properties(&mut self, interner: &Interner) -> bool {
        self.remove_child(interner, "defPPr")
    }

    /// Merges `spec` into the child named `local` (`defPPr` or `lvlNpPr`), creating it in schema
    /// order when absent.
    fn merge_properties(
        &mut self,
        interner: &mut Interner,
        local: &str,
        spec: &ParagraphPropertiesSpec,
    ) {
        let existing = dml_child(&self.children, interner, local)
            .and_then(|element| ParagraphProperties::from_xml(element, interner).ok());
        let element = match existing {
            Some(mut properties) => {
                properties.apply(spec, interner);
                properties.to_xml(interner)
            }
            None => spec.to_properties(interner, local).to_xml(interner),
        };
        TEXT_LIST_STYLE.replace_or_insert(&mut self.children, interner, element, |candidate| {
            candidate == local
        });
        self.empty = false;
    }

    /// Drops the DrawingML child named `local`, returning whether one was there.
    fn remove_child(&mut self, interner: &Interner, local: &str) -> bool {
        let before = self.children.len();
        self.children
            .retain(|node| !is_named(node, interner, local));
        before != self.children.len()
    }
}

/// Whether `node` is the DrawingML element named `local`.
fn is_named(node: &RawNode, interner: &Interner, local: &str) -> bool {
    match node {
        RawNode::Element(element) => {
            is_dml(&element.name, interner) && interner.resolve(element.name.local) == local
        }
        _ => false,
    }
}

/// The element a level's properties live under: level 0 is `a:lvl1pPr`, level 8 is `a:lvl9pPr`.
///
/// This is the one place the `0..=8` / `1..=9` off-by-one is spelled out.
fn level_local(level: IndentLevel) -> String {
    format!("lvl{}pPr", level.value() + 1)
}

fidelity_element_impls!(TextListStyle);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_generated_sequence_ranks_the_default_first_and_the_extension_list_last() {
        // The ranks this model places by are `CT_TextListStyle`'s, generated from the schema — not a
        // table hand-copied into this file.
        assert_eq!(TEXT_LIST_STYLE.symbol, "CT_TextListStyle");
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "defPPr"), Some(0));
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl1pPr"), Some(1));
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl9pPr"), Some(9));
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "extLst"), Some(10));
        // Not a member of the sequence, so it is not placed by rank.
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl0pPr"), None);
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "lvl10pPr"), None);
        assert_eq!(TEXT_LIST_STYLE.rank_of(None, "bodyPr"), None);
    }

    #[test]
    fn a_level_names_the_element_one_higher_than_itself() {
        assert_eq!(level_local(IndentLevel::of(0)), "lvl1pPr");
        assert_eq!(level_local(IndentLevel::of(8)), "lvl9pPr");
    }
}
