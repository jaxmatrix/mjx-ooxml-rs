//! `a:tc` (`CT_TableCell`) and `a:tcPr` (`CT_TableCellProperties`) — a cell, its text, and how it
//! is drawn.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{FromXml as _, Interner, RawAttribute, RawName, RawNode};

use crate::build::{
    attr_bool, attr_emu, attr_str, dml_child, fidelity_element_impls, first_fill_child, set_attr,
};
use crate::fill::Fill;
use crate::geometry::Emu;
use crate::line::LineProperties;
use crate::text::TextBody;

/// `a:tcPr` (`CT_TableCellProperties`) — a cell's margins, text anchoring, borders and fill.
///
/// A fidelity wrapper: the key attributes and the six border elements are exposed typed, while
/// `cell3D`, `headers`, `extLst` and anything unknown are preserved opaque so the cell round-trips.
///
/// The four margins have **non-zero schema defaults** (`91440` EMU left and right, `45720` top and
/// bottom — 0.1" and 0.05"), so an unset margin is not a zero one. The accessors report what the
/// file states; [`DEFAULT_MARGIN_HORIZONTAL`](Self::DEFAULT_MARGIN_HORIZONTAL) and
/// [`DEFAULT_MARGIN_VERTICAL`](Self::DEFAULT_MARGIN_VERTICAL) are what a renderer substitutes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableCellProperties);

/// Which edge of a cell a border is drawn on — the six `CT_LineProperties` children of `a:tcPr`.
///
/// The names are the schema's, expanded: `lnTlToBr` and `lnBlToTr` are the two diagonals, which
/// PowerPoint draws corner to corner inside the cell rather than around it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CellBorder {
    /// `a:lnL` — the left edge.
    Left,
    /// `a:lnR` — the right edge.
    Right,
    /// `a:lnT` — the top edge.
    Top,
    /// `a:lnB` — the bottom edge.
    Bottom,
    /// `a:lnTlToBr` — the diagonal from the top-left corner to the bottom-right.
    TopLeftToBottomRight,
    /// `a:lnBlToTr` — the diagonal from the bottom-left corner to the top-right.
    BottomLeftToTopRight,
}

impl CellBorder {
    /// The element's local name, without its `a:` prefix.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::Left => "lnL",
            Self::Right => "lnR",
            Self::Top => "lnT",
            Self::Bottom => "lnB",
            Self::TopLeftToBottomRight => "lnTlToBr",
            Self::BottomLeftToTopRight => "lnBlToTr",
        }
    }

    /// Every border, in the order `CT_TableCellProperties`'s sequence declares them — which is also
    /// the order a new one must be inserted in, since sequence order is validity.
    #[must_use]
    pub fn all() -> [Self; 6] {
        [
            Self::Left,
            Self::Right,
            Self::Top,
            Self::Bottom,
            Self::TopLeftToBottomRight,
            Self::BottomLeftToTopRight,
        ]
    }

    /// This border's rank in `CT_TableCellProperties`'s sequence.
    #[must_use]
    pub fn rank(self) -> usize {
        match self {
            Self::Left => 0,
            Self::Right => 1,
            Self::Top => 2,
            Self::Bottom => 3,
            Self::TopLeftToBottomRight => 4,
            Self::BottomLeftToTopRight => 5,
        }
    }
}

impl TableCellProperties {
    /// The schema default for the left and right margins (`91440` EMU — 0.1 inch).
    pub const DEFAULT_MARGIN_HORIZONTAL: Emu = Emu::from_emu(91_440);
    /// The schema default for the top and bottom margins (`45720` EMU — 0.05 inch).
    pub const DEFAULT_MARGIN_VERTICAL: Emu = Emu::from_emu(45_720);

    /// The left inset between the cell edge and its text (`@marL`), or `None` if unstated.
    #[must_use]
    pub fn left_margin(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "marL")
    }

    /// The right inset (`@marR`), or `None` if unstated.
    #[must_use]
    pub fn right_margin(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "marR")
    }

    /// The top inset (`@marT`), or `None` if unstated.
    #[must_use]
    pub fn top_margin(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "marT")
    }

    /// The bottom inset (`@marB`), or `None` if unstated.
    #[must_use]
    pub fn bottom_margin(&self, interner: &Interner) -> Option<Emu> {
        attr_emu(&self.attributes, interner, "marB")
    }

    /// Whether the cell's text is centred between its insets (`@anchorCtr`), or `None` if unstated.
    #[must_use]
    pub fn anchor_centered(&self, interner: &Interner) -> Option<bool> {
        attr_bool(&self.attributes, interner, "anchorCtr")
    }

    /// The border on `edge` (`a:lnL` … `a:lnBlToTr`), or `None` if the cell declares none there.
    #[must_use]
    pub fn border(&self, interner: &Interner, edge: CellBorder) -> Option<LineProperties> {
        dml_child(&self.children, interner, edge.wire())
            .and_then(|element| LineProperties::from_xml(element, interner).ok())
    }

    /// The cell's fill (`EG_FillProperties`), or `None` if it declares none — in which case the
    /// table style decides, and failing that the cell is unfilled.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<Fill> {
        first_fill_child(&self.children, interner)
            .and_then(|element| Fill::from_xml(element, interner).ok())
    }

    /// The cell's children, verbatim — for a writer that must place a new child in schema order.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The cell's children, mutably.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }

    /// Sets an attribute on the properties element, rewriting it in place when already present.
    pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
        set_attr(&mut self.attributes, interner, local, value);
        self.empty = self.empty && self.children.is_empty();
    }
}

/// One ordered child of a [`TableCell`]: its typed text body or properties, or an opaque node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableCellContent {
    /// The cell's text (`a:txBody`) — a full `CT_TextBody`, as a shape's is.
    TextBody(TextBody),
    /// The cell's properties (`a:tcPr`).
    Properties(TableCellProperties),
    /// Any other child — `extLst`, whitespace, or an unknown element — preserved verbatim.
    Raw(RawNode),
}

/// `a:tc` (`CT_TableCell`) — one cell of a table row.
///
/// A cell holds a text body and its properties. It also carries the **merge** attributes, and those
/// are the ones worth understanding: a merged region is anchored at its top-left cell, which states
/// `gridSpan` and/or `rowSpan`; the cells it covers are still present, each stating `hMerge` or
/// `vMerge`. Nothing is ever removed from the grid, so a row's cell count always matches the
/// table's column count.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml)]
#[xml(namespace = DML_MAIN)]
pub struct TableCell {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "txBody", variant = TextBody, ty = TextBody),
        child(local = "tcPr", variant = Properties, ty = TableCellProperties)
    )]
    content: Vec<TableCellContent>,
}

impl TableCell {
    /// The cell's text body (`a:txBody`), or `None` if it has none.
    #[must_use]
    pub fn text_body(&self) -> Option<&TextBody> {
        self.content.iter().find_map(|item| match item {
            TableCellContent::TextBody(body) => Some(body),
            _ => None,
        })
    }

    /// The cell's text body, mutably.
    pub fn text_body_mut(&mut self) -> Option<&mut TextBody> {
        self.content.iter_mut().find_map(|item| match item {
            TableCellContent::TextBody(body) => Some(body),
            _ => None,
        })
    }

    /// The cell's text — each paragraph joined by a newline, or `""` if it has no body.
    #[must_use]
    pub fn text(&self) -> String {
        self.text_body().map(TextBody::text).unwrap_or_default()
    }

    /// The cell's properties (`a:tcPr`), or `None` if it declares none.
    #[must_use]
    pub fn properties(&self) -> Option<&TableCellProperties> {
        self.content.iter().find_map(|item| match item {
            TableCellContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// The cell's properties, mutably.
    pub fn properties_mut(&mut self) -> Option<&mut TableCellProperties> {
        self.content.iter_mut().find_map(|item| match item {
            TableCellContent::Properties(properties) => Some(properties),
            _ => None,
        })
    }

    /// How many **columns** this cell spans (`@gridSpan`; schema default `1`).
    ///
    /// Greater than one only on the anchor cell of a horizontally merged region.
    #[must_use]
    pub fn column_span(&self, interner: &Interner) -> usize {
        span_attr(&self.attributes, interner, "gridSpan")
    }

    /// How many **rows** this cell spans (`@rowSpan`; schema default `1`).
    #[must_use]
    pub fn row_span(&self, interner: &Interner) -> usize {
        span_attr(&self.attributes, interner, "rowSpan")
    }

    /// Whether this cell is covered by a horizontal merge to its left (`@hMerge`).
    #[must_use]
    pub fn merged_horizontally(&self, interner: &Interner) -> bool {
        attr_bool(&self.attributes, interner, "hMerge").unwrap_or(false)
    }

    /// Whether this cell is covered by a vertical merge above it (`@vMerge`).
    #[must_use]
    pub fn merged_vertically(&self, interner: &Interner) -> bool {
        attr_bool(&self.attributes, interner, "vMerge").unwrap_or(false)
    }

    /// Whether this cell is **covered** by a merge anchored elsewhere, and so renders nothing of its
    /// own. The anchor of a merged region is not covered, however far it spans.
    #[must_use]
    pub fn is_covered_by_merge(&self, interner: &Interner) -> bool {
        self.merged_horizontally(interner) || self.merged_vertically(interner)
    }

    /// The cell's ordered content (typed children interleaved with opaque nodes).
    #[must_use]
    pub fn content(&self) -> &[TableCellContent] {
        &self.content
    }

    /// The cell's ordered content, mutably.
    pub fn content_mut(&mut self) -> &mut Vec<TableCellContent> {
        &mut self.content
    }

    /// The cell's attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// Sets an attribute on the cell, rewriting it in place when already present — the merge
    /// attributes are written through here.
    pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
        set_attr(&mut self.attributes, interner, local, value);
    }
}

/// Reads a span attribute (`@gridSpan` / `@rowSpan`), defaulting to `1` per the schema. A value
/// below one is not a span, and is read as one — a covered cell states `hMerge`, never `gridSpan="0"`.
fn span_attr(attributes: &[RawAttribute], interner: &Interner, local: &str) -> usize {
    attr_str(attributes, interner, local)
        .and_then(|value| value.trim().parse::<i64>().ok())
        .filter(|span| *span >= 1)
        .map_or(1, |span| span as usize)
}
