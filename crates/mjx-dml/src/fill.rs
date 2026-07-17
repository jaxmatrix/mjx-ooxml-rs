//! DrawingML fill: `a:solidFill` (a solid color fill).

use mjx_ooxml_core::{Interner, RawAttribute, RawName, RawNode};

use crate::build::dml_name;
use crate::color::Color;

/// One ordered child of a [`SolidFill`]: the typed fill [`Color`], or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolidFillContent {
    /// The fill color (any `EG_ColorChoice` element).
    Color(Color),
    /// Any other child — whitespace or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:solidFill` (`CT_SolidColorFillProperties`) — a solid color fill: at most one color child.
///
/// The child is any `EG_ColorChoice` element (`a:srgbClr`, `a:schemeClr`, …), typed as [`Color`];
/// anything else is kept opaque so the fill round-trips. The color is optional (an empty
/// `<a:solidFill/>` is schema-legal).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct SolidFill {
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
    content: Vec<SolidFillContent>,
}

impl SolidFill {
    /// Builds an `a:solidFill` around `color` (a self-closing `<a:solidFill/>` when `None`).
    #[must_use]
    pub fn new(interner: &mut Interner, color: Option<Color>) -> Self {
        let empty = color.is_none();
        Self {
            name: dml_name(interner, "solidFill"),
            attributes: Vec::new(),
            empty,
            content: color.into_iter().map(SolidFillContent::Color).collect(),
        }
    }

    /// The fill color, if present.
    #[must_use]
    pub fn color(&self) -> Option<&Color> {
        self.content.iter().find_map(|item| match item {
            SolidFillContent::Color(color) => Some(color),
            SolidFillContent::Raw(_) => None,
        })
    }

    /// The fill's ordered content (the typed color interleaved with any opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[SolidFillContent] {
        &self.content
    }
}
