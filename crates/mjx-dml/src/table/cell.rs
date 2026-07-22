//! `a:tc` (`CT_TableCell`) and `a:tcPr` (`CT_TableCellProperties`) — a cell, its text, and how it
//! is drawn.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{FromXml as _, Interner, RawAttribute, RawName, RawNode, ToXml as _};

use crate::build::{
    attr_bool, attr_emu, attr_str, dml_child, dml_name, fidelity_element_impls, first_fill_child,
    is_dml, replace_or_insert_child, set_attr,
};
use crate::fill::{Fill, FillSpec};
use crate::geometry::Emu;
use crate::line::{LineProperties, LineSpec};
use crate::text::TextBody;

pub use mjx_ooxml_types::drawingml::{TextAnchoring, TextDirection, TextHorizontalOverflow};

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

    /// Where the text sits vertically within the cell (`@anchor`; wire default `t`), or `None` if
    /// unstated.
    #[must_use]
    pub fn anchor(&self, interner: &Interner) -> Option<TextAnchoring> {
        attr_str(&self.attributes, interner, "anchor").and_then(TextAnchoring::from_wire)
    }

    /// Which way the cell's text flows (`@vert`; wire default `horz`), or `None` if unstated.
    #[must_use]
    pub fn text_direction(&self, interner: &Interner) -> Option<TextDirection> {
        attr_str(&self.attributes, interner, "vert").and_then(TextDirection::from_wire)
    }

    /// What a character too wide for the cell does (`@horzOverflow`; wire default `clip`), or
    /// `None` if unstated.
    #[must_use]
    pub fn horizontal_overflow(&self, interner: &Interner) -> Option<TextHorizontalOverflow> {
        attr_str(&self.attributes, interner, "horzOverflow")
            .and_then(TextHorizontalOverflow::from_wire)
    }

    /// Sets the four insets between the cell's edges and its text, each independently: a `None`
    /// leaves that margin exactly as it was, stated or not.
    pub fn set_margins(
        &mut self,
        interner: &mut Interner,
        left: Option<Emu>,
        right: Option<Emu>,
        top: Option<Emu>,
        bottom: Option<Emu>,
    ) {
        for (local, value) in [
            ("marL", left),
            ("marR", right),
            ("marT", top),
            ("marB", bottom),
        ] {
            if let Some(value) = value {
                set_attr(
                    &mut self.attributes,
                    interner,
                    local,
                    &value.emu().to_string(),
                );
            }
        }
    }

    /// Sets where the text sits vertically (`@anchor`).
    pub fn set_anchor(&mut self, interner: &mut Interner, anchor: TextAnchoring) {
        set_attr(&mut self.attributes, interner, "anchor", anchor.to_wire());
    }

    /// Sets which way the text flows (`@vert`).
    pub fn set_text_direction(&mut self, interner: &mut Interner, direction: TextDirection) {
        set_attr(&mut self.attributes, interner, "vert", direction.to_wire());
    }

    /// Sets what a character too wide for the cell does (`@horzOverflow`).
    pub fn set_horizontal_overflow(
        &mut self,
        interner: &mut Interner,
        overflow: TextHorizontalOverflow,
    ) {
        set_attr(
            &mut self.attributes,
            interner,
            "horzOverflow",
            overflow.to_wire(),
        );
    }

    /// Sets the border on `edge`, or removes it when `line` is `None`.
    ///
    /// The element is replaced in place when the edge already has one, and otherwise inserted at
    /// `edge`'s rank in `CT_TableCellProperties`'s sequence — order is validity here, and the five
    /// other edges, a `cell3D`, a `headers` and an `extLst` all have their own places in it.
    pub fn set_border(
        &mut self,
        interner: &mut Interner,
        edge: CellBorder,
        line: Option<&LineSpec>,
    ) {
        let local = edge.wire();
        let Some(line) = line else {
            self.children.retain(|node| match node {
                RawNode::Element(element) => {
                    !(is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == local)
                }
                _ => true,
            });
            return;
        };
        // A border is an `a:ln` under another name: same `CT_LineProperties` content, different tag,
        // which is exactly why one `LineSpec` serves all six edges.
        let mut element = line.to_line(interner).to_xml(interner);
        element.name = dml_name(interner, local);
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            |candidate| candidate == local,
            tcpr_child_rank,
        );
        self.empty = false;
    }

    /// Sets the cell's fill, or removes it when `fill` is `None` — in which case the table style
    /// decides how the cell is filled.
    pub fn set_fill(&mut self, interner: &mut Interner, fill: Option<&FillSpec>) {
        let Some(fill) = fill else {
            self.children.retain(|node| match node {
                RawNode::Element(element) => {
                    !(is_dml(&element.name, interner)
                        && Fill::is_fill_local(interner.resolve(element.name.local)))
                }
                _ => true,
            });
            return;
        };
        let element = fill.to_fill(interner).to_xml(interner);
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            Fill::is_fill_local,
            tcpr_child_rank,
        );
        self.empty = false;
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

/// A child's position in `CT_TableCellProperties`'s `xsd:sequence`: the six borders, then the 3-D
/// cell style, the fill group, the accessibility headers, and the extension list.
///
/// Order is validity here, not style — a fill written before the borders makes the cell unreadable
/// to Office — so a newly inserted child is placed by this rather than appended.
fn tcpr_child_rank(local: &str) -> Option<usize> {
    if let Some(edge) = CellBorder::all().into_iter().find(|e| e.wire() == local) {
        return Some(edge.rank());
    }
    match local {
        "cell3D" => Some(6),
        _ if Fill::is_fill_local(local) => Some(7),
        "headers" => Some(8),
        "extLst" => Some(9),
        _ => None,
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

    /// Replaces the cell's text body and properties, keeping any opaque children (an `extLst`,
    /// whitespace) it already had, in `CT_TableCell` sequence order (`txBody?`, `tcPr?`, then the
    /// rest).
    ///
    /// This is how a cell **promoted** to a merge anchor takes the old anchor's `a:txBody` and
    /// `a:tcPr` so the table looks unchanged: the promoted cell's own (previously hidden) text is
    /// discarded in favour of what was rendering there.
    pub fn set_body_and_properties(
        &mut self,
        body: Option<TextBody>,
        properties: Option<TableCellProperties>,
    ) {
        self.content
            .retain(|item| matches!(item, TableCellContent::Raw(_)));
        let mut rebuilt = Vec::with_capacity(self.content.len() + 2);
        if let Some(body) = body {
            rebuilt.push(TableCellContent::TextBody(body));
        }
        if let Some(properties) = properties {
            rebuilt.push(TableCellContent::Properties(properties));
        }
        rebuilt.append(&mut self.content);
        self.content = rebuilt;
        self.empty = self.content.is_empty();
    }

    /// The cell's attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }

    /// Sets an attribute on the cell, rewriting it in place when already present.
    pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
        set_attr(&mut self.attributes, interner, local, value);
    }

    /// Makes this cell the **anchor** of a merged region `columns` wide and `rows` tall.
    ///
    /// A span of `1` is the schema default, so it is **removed** rather than written: a file states
    /// `gridSpan` only when a cell really does span, and emitting `gridSpan="1"` everywhere would
    /// add noise to every table this library touches.
    ///
    /// This says nothing about the cells being covered — they must be told separately with
    /// [`set_merged`](Self::set_merged), which is what makes the region a region.
    pub fn set_spans(&mut self, interner: &mut Interner, columns: usize, rows: usize) {
        for (local, span) in [("gridSpan", columns), ("rowSpan", rows)] {
            if span > 1 {
                set_attr(&mut self.attributes, interner, local, &span.to_string());
            } else {
                self.remove_attribute(interner, local);
            }
        }
    }

    /// Marks this cell as **covered** by a merge anchored to its left (`hMerge`) and/or above it
    /// (`vMerge`). A cell covered from both directions states both.
    ///
    /// `false` **removes** the attribute rather than writing `hMerge="0"`: the schema default is
    /// already false, and "not merged" is the absence of a claim, not a claim of absence.
    pub fn set_merged(&mut self, interner: &mut Interner, horizontally: bool, vertically: bool) {
        for (local, merged) in [("hMerge", horizontally), ("vMerge", vertically)] {
            if merged {
                set_attr(&mut self.attributes, interner, local, "1");
            } else {
                self.remove_attribute(interner, local);
            }
        }
    }

    /// Clears every trace of merging from this cell — both spans and both covered flags — leaving
    /// an ordinary cell that stands alone.
    ///
    /// The cell's text and properties are untouched, which is what lets unmerging give back exactly
    /// what merging covered up.
    pub fn clear_merge(&mut self, interner: &mut Interner) {
        self.set_spans(interner, 1, 1);
        self.set_merged(interner, false, false);
    }

    /// Removes an unprefixed attribute, if the cell has one.
    fn remove_attribute(&mut self, interner: &Interner, local: &str) {
        self.attributes.retain(|attribute| {
            attribute.name.prefix.is_some() || interner.resolve(attribute.name.local) != local
        });
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
