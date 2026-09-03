//! `dgm:styleDef` (`CT_StyleDefinition`) — the Diagram Style part: the shape formatting
//! (line/fill/effect/font references, 3-D scene and bevel, text properties) each quick-style label
//! applies to a diagram's nodes.
//!
//! ECMA-376 Part 1 §21.4.5's formatting groups (`a:CT_Scene3D`, `a:CT_Shape3D`, `a:CT_ShapeStyle`,
//! and this schema's own `CT_TextProps` wrapper around `a:EG_Text3D`) are DrawingML groups this crate
//! does not yet model standalone outside a shape's own surface — the same choice [`super::data`]
//! makes for a point's `spPr` and property-set `style` — so [`StyleLabel`] types the one attribute
//! that names *which* node kind a label formats and preserves everything else opaque. See the
//! [module docs](super) for the full modelled-vs-preserved line.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode, Text};

use crate::build::fidelity_element_impls;

use super::common::{DiagramCategoryList, DiagramDescription, DiagramTitle};
use super::support::dgm_name;

// ---------------------------------------------------------------------------------------------
// dgm:styleLbl (CT_StyleLabel)
// ---------------------------------------------------------------------------------------------

/// `dgm:styleLbl` (`CT_StyleLabel`) — the formatting a [`StyleDefinition`] assigns to one
/// quick-style label (`@name`, matching a `dgm:pt/dgm:prSet/@presStyleLbl` a data model's points bind
/// to). ECMA-376 Part 1 §21.4.5.13 *CT_StyleLabel*.
///
/// Its four content children — `scene3d` (`a:CT_Scene3D`), `sp3d` (`a:CT_Shape3D`), `txPr`
/// (`CT_TextProps`) and `style` (`a:CT_ShapeStyle`) — are the externally-defined formatting groups
/// this crate does not model standalone (see the [module docs](super::style)); `extLst` and any
/// unknown child are likewise preserved. Only `@name` is typed.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "name", codec = Text, accessor = label_name, required))]
pub struct StyleLabel {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}
fidelity_element_impls!(StyleLabel);

impl StyleLabel {
    /// A fresh, empty `dgm:styleLbl` named `label_name` (`@name`) — no formatting yet.
    #[must_use]
    pub fn new(interner: &mut Interner, label_name: &str) -> Self {
        let mut label = Self {
            name: dgm_name(interner, "styleLbl"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: true,
        };
        label.set_label_name(interner, label_name);
        label
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:styleDef (CT_StyleDefinition) — the style part's root
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`StyleDefinition`]: its typed members, or an opaque node (`scene3d`,
/// `extLst`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleDefinitionContent {
    /// A display name (`title`) — repeatable, one per locale.
    Title(DiagramTitle),
    /// A description (`desc`) — repeatable, one per locale.
    Description(DiagramDescription),
    /// The gallery categories this style belongs to (`catLst`).
    Categories(DiagramCategoryList),
    /// One quick-style label's formatting (`styleLbl`) — repeatable, at least one required.
    StyleLabel(StyleLabel),
    /// Any other child — `scene3d` (`a:CT_Scene3D`), `extLst`, whitespace, or an unknown element —
    /// preserved verbatim.
    Raw(RawNode),
}

/// `dgm:styleDef` (`CT_StyleDefinition`) — the root of the Diagram Style part: the display
/// name/description/gallery category this quick style is offered under, and the formatting it
/// assigns to each style label. ECMA-376 Part 1 §21.4.5.12 *CT_StyleDefinition*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "uniqueId", codec = Text, accessor = unique_id))]
#[xml(attribute(local = "minVer", codec = Text, accessor = minimum_version))]
pub struct StyleDefinition {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "title", variant = Title, ty = DiagramTitle),
        child(local = "desc", variant = Description, ty = DiagramDescription),
        child(local = "catLst", variant = Categories, ty = DiagramCategoryList),
        child(local = "styleLbl", variant = StyleLabel, ty = StyleLabel)
    )]
    content: Vec<StyleDefinitionContent>,
}

impl StyleDefinition {
    /// A fresh `dgm:styleDef` naming `unique_id` (`@uniqueId`), with `style_labels` as its per-label
    /// formatting — `CT_StyleDefinition`'s one required child, at least one `styleLbl`.
    #[must_use]
    pub fn new(interner: &mut Interner, unique_id: &str, style_labels: Vec<StyleLabel>) -> Self {
        let content: Vec<StyleDefinitionContent> = style_labels
            .into_iter()
            .map(StyleDefinitionContent::StyleLabel)
            .collect();
        let mut definition = Self {
            name: dgm_name(interner, "styleDef"),
            attributes: Vec::new(),
            empty: content.is_empty(),
            content,
        };
        definition.set_unique_id(interner, Some(unique_id));
        definition
    }

    /// The style's display names (`title`), one per locale.
    pub fn titles(&self) -> impl Iterator<Item = &DiagramTitle> {
        self.content.iter().filter_map(|item| match item {
            StyleDefinitionContent::Title(title) => Some(title),
            _ => None,
        })
    }
    /// The style's gallery categories (`catLst`), or `None`.
    #[must_use]
    pub fn categories(&self) -> Option<&DiagramCategoryList> {
        self.content.iter().find_map(|item| match item {
            StyleDefinitionContent::Categories(categories) => Some(categories),
            _ => None,
        })
    }
    /// The style's per-label formatting (`styleLbl`), in order.
    pub fn style_labels(&self) -> impl Iterator<Item = &StyleLabel> {
        self.content.iter().filter_map(|item| match item {
            StyleDefinitionContent::StyleLabel(label) => Some(label),
            _ => None,
        })
    }
    /// The style label whose `@name` is `label_name`, or `None`.
    #[must_use]
    pub fn style_label(&self, interner: &Interner, label_name: &str) -> Option<&StyleLabel> {
        self.style_labels()
            .find(|label| label.label_name(interner).is_ok_and(|name| name.as_ref() == label_name))
    }
    /// The style's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[StyleDefinitionContent] {
        &self.content
    }
}
