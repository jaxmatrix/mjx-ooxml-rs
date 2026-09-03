//! `a:tc` (`CT_TableCell`) and `a:tcPr` (`CT_TableCellProperties`) — a cell, its text, and how it
//! is drawn.

use mjx_derive::{FromXml, ToXml};
use mjx_ooxml_core::{
    AttributeError, Enumeration, FromXml as _, Interner, Number, RawAttribute, RawElement, RawName,
    RawNode, Text, ToXml as _,
};
use mjx_ooxml_types::support::OnOff;

use mjx_ooxml_types::child_order::TABLE_CELL_PROPERTIES;

use crate::build::{
    dml_child, dml_element, dml_name, fidelity_element_impls, first_fill_child, is_dml,
};
use crate::codec::EmuCoordinate;
use crate::fill::{Fill, FillSpec};
use crate::geometry::Emu;
use crate::line::{LineProperties, LineSpec};
use crate::table::style::Cell3D;
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
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "marL", codec = EmuCoordinate, accessor = left_margin))]
#[xml(attribute(local = "marR", codec = EmuCoordinate, accessor = right_margin))]
#[xml(attribute(local = "marT", codec = EmuCoordinate, accessor = top_margin))]
#[xml(attribute(local = "marB", codec = EmuCoordinate, accessor = bottom_margin))]
#[xml(attribute(local = "anchor", codec = Enumeration<TextAnchoring>, accessor = anchor))]
#[xml(attribute(local = "anchorCtr", codec = OnOff, accessor = anchor_centered))]
#[xml(attribute(local = "vert", codec = Enumeration<TextDirection>, accessor = text_direction))]
#[xml(attribute(
    local = "horzOverflow",
    codec = Enumeration<TextHorizontalOverflow>,
    accessor = horizontal_overflow
))]
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
}

impl TableCellProperties {
    /// The schema default for the left and right margins (`91440` EMU — 0.1 inch).
    pub const DEFAULT_MARGIN_HORIZONTAL: Emu = Emu::from_emu(91_440);
    /// The schema default for the top and bottom margins (`45720` EMU — 0.05 inch).
    pub const DEFAULT_MARGIN_VERTICAL: Emu = Emu::from_emu(45_720);

    /// Sets the four insets between the cell's edges and its text, each independently: a `None`
    /// leaves that margin exactly as it was, stated or not.
    ///
    /// That is *not* what the generated per-margin setters mean — `set_left_margin(None)` removes
    /// `@marL` — which is why this method exists beside them: "leave it alone" and "clear it" are
    /// different instructions, and a caller adjusting one inset of four means the first.
    pub fn set_margins(
        &mut self,
        interner: &mut Interner,
        left: Option<Emu>,
        right: Option<Emu>,
        top: Option<Emu>,
        bottom: Option<Emu>,
    ) {
        if left.is_some() {
            self.set_left_margin(interner, left);
        }
        if right.is_some() {
            self.set_right_margin(interner, right);
        }
        if top.is_some() {
            self.set_top_margin(interner, top);
        }
        if bottom.is_some() {
            self.set_bottom_margin(interner, bottom);
        }
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
        TABLE_CELL_PROPERTIES.replace_or_insert(
            &mut self.children,
            interner,
            element,
            |candidate| candidate == local,
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
        TABLE_CELL_PROPERTIES.replace_or_insert(
            &mut self.children,
            interner,
            element,
            Fill::is_fill_local,
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
    ///
    /// The untyped escape hatch, for the `CT_TableCellProperties` attributes this model does not
    /// name.
    pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
        mjx_xml::attribute::set(&mut self.attributes, interner, None, local, value);
        self.empty = self.empty && self.children.is_empty();
    }

    /// The ids of the header cells that describe this cell (`a:headers > a:header`), in order — the
    /// accessibility association a screen reader reads to announce which headers a data cell sits
    /// under. Empty when the cell names none.
    ///
    /// Each string is another cell's `@id` (see [`TableCell::id`]).
    #[must_use]
    pub fn headers(&self, interner: &Interner) -> Vec<String> {
        let Some(headers) = dml_child(&self.children, interner, "headers") else {
            return Vec::new();
        };
        headers
            .children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element)
                    if is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == "header" =>
                {
                    Some(element_text(element))
                }
                _ => None,
            })
            .collect()
    }

    /// Sets the header-cell ids that describe this cell (`a:headers`), replacing whatever it had.
    /// An empty slice **removes** the `a:headers` child entirely.
    pub fn set_headers(&mut self, interner: &mut Interner, header_ids: &[&str]) {
        if header_ids.is_empty() {
            self.children.retain(|node| match node {
                RawNode::Element(element) => {
                    !(is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == "headers")
                }
                _ => true,
            });
            return;
        }
        let entries: Vec<RawNode> = header_ids
            .iter()
            .map(|id| {
                let text = RawNode::Text(Box::from(id.as_bytes()));
                RawNode::Element(dml_element(interner, "header", Vec::new(), vec![text]))
            })
            .collect();
        let element = dml_element(interner, "headers", Vec::new(), entries);
        TABLE_CELL_PROPERTIES.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "headers"
        });
        self.empty = false;
    }

    /// The cell's own 3-D bevel and lighting (`a:cell3D`), or `None` if it declares none.
    ///
    /// This is the *direct*-cell counterpart of the table-style
    /// [`TableStyleCellStyle::cell_3d`](crate::table::TableStyleCellStyle::cell_3d): both carry the
    /// same [`Cell3D`] model, one authored on a single cell and the other on a named style part.
    #[must_use]
    pub fn cell_3d(&self, interner: &Interner) -> Option<Cell3D> {
        dml_child(&self.children, interner, "cell3D")
            .and_then(|element| Cell3D::from_xml(element, interner).ok())
    }

    /// Sets the cell's 3-D (`a:cell3D`), replacing any existing one in place. Build the [`Cell3D`]
    /// with [`Cell3D::new`] and its setters.
    pub fn set_cell_3d(&mut self, interner: &mut Interner, cell_3d: &Cell3D) {
        let element = cell_3d.to_xml(interner);
        TABLE_CELL_PROPERTIES.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "cell3D"
        });
        self.empty = false;
    }
}

/// The concatenated text of an element's direct text nodes, trimmed.
fn element_text(element: &RawElement) -> String {
    let mut text = String::new();
    for node in &element.children {
        if let RawNode::Text(bytes) | RawNode::CData(bytes) = node {
            text.push_str(&String::from_utf8_lossy(bytes));
        }
    }
    text.trim().to_owned()
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

/// `a:tc`'s span and merge attributes — the four that [`TableCell`] projects into *total* answers.
///
/// Declared on a face of their own rather than on [`TableCell`] itself, because what a caller wants
/// from them is not "what does the file say" but "how many columns does this cell cover" and "is it
/// covered by a merge": every one has a schema default that makes the question always answerable, so
/// the public methods return a `usize` and a `bool` and this face is where the file's own answer —
/// absent, present, or unreadable — is turned into one.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "gridSpan", codec = Number<i64>, accessor = column_span))]
#[xml(attribute(local = "rowSpan", codec = Number<i64>, accessor = row_span))]
#[xml(attribute(local = "hMerge", codec = OnOff, accessor = merged_horizontally))]
#[xml(attribute(local = "vMerge", codec = OnOff, accessor = merged_vertically))]
struct CellMergeAttributes<A> {
    attributes: A,
}

/// `a:tc` (`CT_TableCell`) — one cell of a table row.
///
/// A cell holds a text body and its properties. It also carries the **merge** attributes, and those
/// are the ones worth understanding: a merged region is anchored at its top-left cell, which states
/// `gridSpan` and/or `rowSpan`; the cells it covers are still present, each stating `hMerge` or
/// `vMerge`. Nothing is ever removed from the grid, so a row's cell count always matches the
/// table's column count.
#[derive(Debug, Clone, PartialEq, Eq, FromXml, ToXml, mjx_derive::XmlAttributes)]
#[xml(namespace = DML_MAIN)]
#[xml(attribute(local = "id", codec = Text, accessor = id))]
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

    /// This cell's span and merge attributes, borrowed.
    fn merge_face(&self) -> CellMergeAttributes<&[RawAttribute]> {
        CellMergeAttributes {
            attributes: &self.attributes,
        }
    }

    /// How many **columns** this cell spans (`@gridSpan`; schema default `1`).
    ///
    /// Greater than one only on the anchor cell of a horizontally merged region. A value below one
    /// is not a span and reads as one — a covered cell states `hMerge`, never `gridSpan="0"` — and
    /// so does a value that is not a number at all.
    #[must_use]
    pub fn column_span(&self, interner: &Interner) -> usize {
        span(self.merge_face().column_span(interner))
    }

    /// How many **rows** this cell spans (`@rowSpan`; schema default `1`); see
    /// [`column_span`](Self::column_span).
    #[must_use]
    pub fn row_span(&self, interner: &Interner) -> usize {
        span(self.merge_face().row_span(interner))
    }

    /// Whether this cell is covered by a horizontal merge to its left (`@hMerge`).
    #[must_use]
    pub fn merged_horizontally(&self, interner: &Interner) -> bool {
        self.merge_face()
            .merged_horizontally(interner)
            .ok()
            .flatten()
            .unwrap_or(false)
    }

    /// Whether this cell is covered by a vertical merge above it (`@vMerge`).
    #[must_use]
    pub fn merged_vertically(&self, interner: &Interner) -> bool {
        self.merge_face()
            .merged_vertically(interner)
            .ok()
            .flatten()
            .unwrap_or(false)
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
    ///
    /// The untyped escape hatch, for the `CT_TableCell` attributes this model does not name.
    pub fn set_attribute(&mut self, interner: &mut Interner, local: &str, value: &str) {
        mjx_xml::attribute::set(&mut self.attributes, interner, None, local, value);
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
        let stated = |span: usize| (span > 1).then_some(span as i64);
        let mut face = CellMergeAttributes {
            attributes: &mut self.attributes,
        };
        face.set_column_span(interner, stated(columns));
        face.set_row_span(interner, stated(rows));
    }

    /// Marks this cell as **covered** by a merge anchored to its left (`hMerge`) and/or above it
    /// (`vMerge`). A cell covered from both directions states both.
    ///
    /// `false` **removes** the attribute rather than writing `hMerge="0"`: the schema default is
    /// already false, and "not merged" is the absence of a claim, not a claim of absence.
    pub fn set_merged(&mut self, interner: &mut Interner, horizontally: bool, vertically: bool) {
        let mut face = CellMergeAttributes {
            attributes: &mut self.attributes,
        };
        face.set_merged_horizontally(interner, horizontally.then_some(true));
        face.set_merged_vertically(interner, vertically.then_some(true));
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
}

/// One span attribute's reading as a cell count: the schema default `1` for an absent, unreadable,
/// or below-one value.
fn span(read: Result<Option<i64>, AttributeError>) -> usize {
    read.ok()
        .flatten()
        .filter(|span| *span >= 1)
        .map_or(1, |span| span as usize)
}
