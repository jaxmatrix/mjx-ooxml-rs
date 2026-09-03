//! `title` / `desc` / `cat` / `catLst` — the naming and categorisation metadata every one of the
//! four diagram parts carries, whichever of `dml-diagram.xsd`'s three redundant complex-type
//! symbols declares it. See the [module-level note](super) on why this crate gives each wire shape
//! one Rust type rather than three.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, Number, RawAttribute, RawName, RawNode, Text};

use crate::build::fidelity_element_impls;

use super::support::dgm_name;

/// `dgm:title` (`CT_CTName` / `CT_Name` / `CT_SDName`) — a localized display name.
///
/// ECMA-376 Part 1 §21.4.2.30 *title (Title)*: "Title of the Diagram Layout." The same element also
/// names a colours part (§21.4.4.11) and a style part (§21.4.5.11) — same two attributes, same
/// meaning, so one type serves all three.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lang", codec = Text, accessor = language))]
#[xml(attribute(local = "val", codec = Text, accessor = value, required))]
pub struct DiagramTitle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(DiagramTitle);

impl DiagramTitle {
    /// A fresh `dgm:title` carrying `value` (and `language`, when given).
    #[must_use]
    pub fn new(interner: &mut Interner, value: &str, language: Option<&str>) -> Self {
        let mut title = Self {
            name: dgm_name(interner, "title"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        title.set_value(interner, value);
        title.set_language(interner, language);
        title
    }
}

/// `dgm:desc` (`CT_CTDescription` / `CT_Description` / `CT_SDDescription`) — a localized
/// description, wire-identical to [`DiagramTitle`] but carrying the part's description rather than
/// its name (§21.4.2.11 *desc (Description)*: "This element holds a description for a layout
/// definition.").
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "lang", codec = Text, accessor = language))]
#[xml(attribute(local = "val", codec = Text, accessor = value, required))]
pub struct DiagramDescription {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(DiagramDescription);

impl DiagramDescription {
    /// A fresh `dgm:desc` carrying `value` (and `language`, when given).
    #[must_use]
    pub fn new(interner: &mut Interner, value: &str, language: Option<&str>) -> Self {
        let mut desc = Self {
            name: dgm_name(interner, "desc"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        desc.set_value(interner, value);
        desc.set_language(interner, language);
        desc
    }
}

/// `dgm:cat` (`CT_CTCategory` / `CT_Category` / `CT_SDCategory`) — one category a part is filed
/// under in the "Choose a SmartArt Graphic" gallery (§21.4.2.4 *cat (Category)*: "This element
/// specifies a category in the user interface where this layout definition displays to the user.").
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "type", codec = Text, accessor = category_type, required))]
#[xml(attribute(local = "pri", codec = Number<u32>, accessor = priority, required))]
pub struct DiagramCategory {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(DiagramCategory);

impl DiagramCategory {
    /// A fresh `dgm:cat` of `category_type` at `priority` — the priority orders the category's
    /// members in the gallery UI, lower first (§21.4.2.4).
    #[must_use]
    pub fn new(interner: &mut Interner, category_type: &str, priority: u32) -> Self {
        let mut cat = Self {
            name: dgm_name(interner, "cat"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        cat.set_category_type(interner, category_type);
        cat.set_priority(interner, priority);
        cat
    }
}

/// One ordered child of a [`DiagramCategoryList`]: a typed [`DiagramCategory`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramCategoryListContent {
    /// A category (`dgm:cat`).
    Category(DiagramCategory),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:catLst` (`CT_CTCategories` / `CT_Categories` / `CT_SDCategories`) — a part's categories, in
/// order (§21.4.2.5 *catLst (Category List)*: "This element is simply a list of `cat` elements.").
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_DIAGRAM)]
pub struct DiagramCategoryList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "cat", variant = Category, ty = DiagramCategory))]
    content: Vec<DiagramCategoryListContent>,
}

impl DiagramCategoryList {
    /// A fresh `dgm:catLst` of `categories`.
    #[must_use]
    pub fn new(interner: &mut Interner, categories: Vec<DiagramCategory>) -> Self {
        let content: Vec<DiagramCategoryListContent> = categories
            .into_iter()
            .map(DiagramCategoryListContent::Category)
            .collect();
        let empty = content.is_empty();
        Self {
            name: dgm_name(interner, "catLst"),
            attributes: Vec::new(),
            empty,
            content,
        }
    }

    /// The list's categories, in order (opaque children are skipped).
    pub fn categories(&self) -> impl Iterator<Item = &DiagramCategory> {
        self.content.iter().filter_map(|item| match item {
            DiagramCategoryListContent::Category(cat) => Some(cat),
            _ => None,
        })
    }

    /// The list's ordered content (typed categories interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[DiagramCategoryListContent] {
        &self.content
    }
}
