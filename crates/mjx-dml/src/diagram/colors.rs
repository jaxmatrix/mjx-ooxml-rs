//! `dgm:colorsDef` (`CT_ColorTransform`) — the Diagram Colors part: which colours each quick-style
//! label's nodes take.
//!
//! ECMA-376 Part 1 §21.4.4 groups the colours part around the same title/description/category
//! metadata every part carries (see [`super::common`]) plus a list of *style labels*
//! ([`StyleLabelColors`], `CT_CTStyleLabel` — not to be confused with the *style* part's own
//! [`super::StyleLabel`], `CT_StyleLabel`, a different complex type the schema happens to name almost
//! identically). Each style label names up to six colour lists ([`ColorList`], `CT_Colors`) — fill,
//! line, effect, and the same three again for text — that a consumer cycles or spans across a
//! diagram's nodes.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{Enumeration, Interner, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::diagram::{ClrAppMethod, HueDirection};

use crate::color::Color;

use super::common::{DiagramCategoryList, DiagramDescription, DiagramTitle};
use super::support::dgm_name;

// ---------------------------------------------------------------------------------------------
// dgm:fillClrLst / linClrLst / effectClrLst / txLinClrLst / txFillClrLst / txEffectClrLst
// (CT_Colors) — six element names, one complex type
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`ColorList`]: the typed [`Color`] (any `a:EG_ColorChoice` member), or an
/// opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorListContent {
    /// A colour (`a:srgbClr`, `a:schemeClr`, `a:sysClr`, `a:scrgbClr`, `a:hslClr` or `a:prstClr`).
    Color(Color),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `CT_Colors` — a list of colours a consumer applies across a diagram's nodes, under whichever of
/// the six element names ([`fill`](StyleLabelColors::fill), [`line`](StyleLabelColors::line), …) a
/// [`StyleLabelColors`] gives it. ECMA-376 Part 1 §21.4.4.2 *CT_Colors*: "list of colors".
///
/// `@meth` ([`ClrAppMethod`]) selects whether the list is applied once per node in a `span`, cycled,
/// or `repeat`ed; `@hueDir` ([`HueDirection`]) is the direction a `cycle` rotates hue in.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "meth", codec = Enumeration<ClrAppMethod>, accessor = method))]
#[xml(attribute(local = "hueDir", codec = Enumeration<HueDirection>, accessor = hue_direction))]
pub struct ColorList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "srgbClr", variant = Color, ty = Color),
        child(local = "schemeClr", variant = Color, ty = Color),
        child(local = "sysClr", variant = Color, ty = Color),
        child(local = "scrgbClr", variant = Color, ty = Color),
        child(local = "hslClr", variant = Color, ty = Color),
        child(local = "prstClr", variant = Color, ty = Color)
    )]
    content: Vec<ColorListContent>,
}

impl ColorList {
    /// A fresh colour list named `local` (one of the six wire names a [`StyleLabelColors`] gives a
    /// `CT_Colors`), carrying `colors` in order.
    #[must_use]
    pub fn new(interner: &mut Interner, local: &str, colors: Vec<Color>) -> Self {
        let content: Vec<ColorListContent> = colors.into_iter().map(ColorListContent::Color).collect();
        let empty = content.is_empty();
        Self {
            name: dgm_name(interner, local),
            attributes: Vec::new(),
            empty,
            content,
        }
    }

    /// The list's colours, in order (opaque children are skipped).
    pub fn colors(&self) -> impl Iterator<Item = &Color> {
        self.content.iter().filter_map(|item| match item {
            ColorListContent::Color(color) => Some(color),
            ColorListContent::Raw(_) => None,
        })
    }

    /// The list's ordered content (typed colours interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[ColorListContent] {
        &self.content
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:styleLbl (CT_CTStyleLabel)
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`StyleLabelColors`]: one of its six typed [`ColorList`]s, or an opaque
/// node (`extLst`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleLabelColorsContent {
    /// `fillClrLst` — the fill colours.
    Fill(ColorList),
    /// `linClrLst` — the line colours.
    Line(ColorList),
    /// `effectClrLst` — the effect colours.
    Effect(ColorList),
    /// `txLinClrLst` — the text outline colours.
    TextLine(ColorList),
    /// `txFillClrLst` — the text fill colours.
    TextFill(ColorList),
    /// `txEffectClrLst` — the text effect colours.
    TextEffect(ColorList),
    /// Any other child — `extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:styleLbl` (`CT_CTStyleLabel`) — the colours a [`ColorTransform`] assigns to one quick-style
/// label (`@name`, matching a `dgm:pt/dgm:prSet/@presStyleLbl` a data model's points bind to).
/// ECMA-376 Part 1 §21.4.4.4 *CT_CTStyleLabel*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "name", codec = Text, accessor = label_name, required))]
pub struct StyleLabelColors {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "fillClrLst", variant = Fill, ty = ColorList),
        child(local = "linClrLst", variant = Line, ty = ColorList),
        child(local = "effectClrLst", variant = Effect, ty = ColorList),
        child(local = "txLinClrLst", variant = TextLine, ty = ColorList),
        child(local = "txFillClrLst", variant = TextFill, ty = ColorList),
        child(local = "txEffectClrLst", variant = TextEffect, ty = ColorList)
    )]
    content: Vec<StyleLabelColorsContent>,
}

impl StyleLabelColors {
    /// The label's fill colours (`fillClrLst`), or `None`.
    #[must_use]
    pub fn fill(&self) -> Option<&ColorList> {
        self.content.iter().find_map(|item| match item {
            StyleLabelColorsContent::Fill(list) => Some(list),
            _ => None,
        })
    }
    /// The label's line colours (`linClrLst`), or `None`.
    #[must_use]
    pub fn line(&self) -> Option<&ColorList> {
        self.content.iter().find_map(|item| match item {
            StyleLabelColorsContent::Line(list) => Some(list),
            _ => None,
        })
    }
    /// The label's effect colours (`effectClrLst`), or `None`.
    #[must_use]
    pub fn effect(&self) -> Option<&ColorList> {
        self.content.iter().find_map(|item| match item {
            StyleLabelColorsContent::Effect(list) => Some(list),
            _ => None,
        })
    }
    /// The label's text outline colours (`txLinClrLst`), or `None`.
    #[must_use]
    pub fn text_line(&self) -> Option<&ColorList> {
        self.content.iter().find_map(|item| match item {
            StyleLabelColorsContent::TextLine(list) => Some(list),
            _ => None,
        })
    }
    /// The label's text fill colours (`txFillClrLst`), or `None`.
    #[must_use]
    pub fn text_fill(&self) -> Option<&ColorList> {
        self.content.iter().find_map(|item| match item {
            StyleLabelColorsContent::TextFill(list) => Some(list),
            _ => None,
        })
    }
    /// The label's text effect colours (`txEffectClrLst`), or `None`.
    #[must_use]
    pub fn text_effect(&self) -> Option<&ColorList> {
        self.content.iter().find_map(|item| match item {
            StyleLabelColorsContent::TextEffect(list) => Some(list),
            _ => None,
        })
    }
    /// The label's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[StyleLabelColorsContent] {
        &self.content
    }
}

// ---------------------------------------------------------------------------------------------
// dgm:colorsDef (CT_ColorTransform) — the colours part's root
// ---------------------------------------------------------------------------------------------

/// One ordered child of a [`ColorTransform`]: its typed members, or an opaque node (`extLst`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorTransformContent {
    /// A display name (`title`) — repeatable, one per locale.
    Title(DiagramTitle),
    /// A description (`desc`) — repeatable, one per locale.
    Description(DiagramDescription),
    /// The gallery categories this colour transform belongs to (`catLst`).
    Categories(DiagramCategoryList),
    /// One quick-style label's colours (`styleLbl`) — repeatable.
    StyleLabel(StyleLabelColors),
    /// Any other child — `extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `dgm:colorsDef` (`CT_ColorTransform`) — the root of the Diagram Colors part: the display
/// name/description/gallery category this colour transform is offered under, and the colours it
/// assigns to each quick-style label. ECMA-376 Part 1 §21.4.4.3 *CT_ColorTransform*.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_DIAGRAM)]
#[xml(attribute(local = "uniqueId", codec = Text, accessor = unique_id))]
#[xml(attribute(local = "minVer", codec = Text, accessor = minimum_version))]
pub struct ColorTransform {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "title", variant = Title, ty = DiagramTitle),
        child(local = "desc", variant = Description, ty = DiagramDescription),
        child(local = "catLst", variant = Categories, ty = DiagramCategoryList),
        child(local = "styleLbl", variant = StyleLabel, ty = StyleLabelColors)
    )]
    content: Vec<ColorTransformContent>,
}

impl ColorTransform {
    /// A fresh `dgm:colorsDef` naming `unique_id` (`@uniqueId`), with `style_labels` as its
    /// per-label colour assignments.
    #[must_use]
    pub fn new(
        interner: &mut Interner,
        unique_id: &str,
        style_labels: Vec<StyleLabelColors>,
    ) -> Self {
        let content: Vec<ColorTransformContent> = style_labels
            .into_iter()
            .map(ColorTransformContent::StyleLabel)
            .collect();
        let empty = content.is_empty();
        let mut definition = Self {
            name: dgm_name(interner, "colorsDef"),
            attributes: Vec::new(),
            empty,
            content,
        };
        definition.set_unique_id(interner, Some(unique_id));
        definition
    }

    /// The colour transform's display names (`title`), one per locale.
    pub fn titles(&self) -> impl Iterator<Item = &DiagramTitle> {
        self.content.iter().filter_map(|item| match item {
            ColorTransformContent::Title(title) => Some(title),
            _ => None,
        })
    }
    /// The colour transform's gallery categories (`catLst`), or `None`.
    #[must_use]
    pub fn categories(&self) -> Option<&DiagramCategoryList> {
        self.content.iter().find_map(|item| match item {
            ColorTransformContent::Categories(categories) => Some(categories),
            _ => None,
        })
    }
    /// The colour transform's per-label colour assignments (`styleLbl`), in order.
    pub fn style_labels(&self) -> impl Iterator<Item = &StyleLabelColors> {
        self.content.iter().filter_map(|item| match item {
            ColorTransformContent::StyleLabel(label) => Some(label),
            _ => None,
        })
    }
    /// The colour transform whose `styleLbl/@name` is `label_name`, or `None`.
    #[must_use]
    pub fn style_label(&self, interner: &Interner, label_name: &str) -> Option<&StyleLabelColors> {
        self.style_labels()
            .find(|label| label.label_name(interner).is_ok_and(|name| name.as_ref() == label_name))
    }
    /// The colour transform's ordered content, verbatim.
    #[must_use]
    pub fn content(&self) -> &[ColorTransformContent] {
        &self.content
    }
}
