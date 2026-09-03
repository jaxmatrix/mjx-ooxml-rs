//! `tableStyles.xml` (`CT_TableStyleList`) — the table styles a table's `a:tableStyleId` resolves to.
//!
//! A table names its look by GUID (`a:tblPr > a:tableStyleId`); the look itself lives in the
//! presentation's `tableStyles.xml` part. This models that part so the reference **resolves** — which
//! is what lets a later tier answer what a cell actually renders as.
//!
//! # How little of this is new
//!
//! A table style is layered formatting keyed by *which part of the table* a cell is in — the whole
//! table, a banded row, the header row, a corner cell. Each part's formatting is the DrawingML this
//! crate already models: a cell fill is the [fill model](crate::fill), its borders are
//! [`LineProperties`], its text colour is a [`Color`], its background effects an [`EffectList`]. The
//! genuinely new pieces are small: the tri-state [`OnOffStyle`] a style takes on bold/italic, and the
//! *themeable* wrappers — every line, fill, effect and font may be given **explicitly** or named as a
//! **reference into the theme's style matrix** ([`StyleMatrixReference`]).
//!
//! # Fidelity
//!
//! Each type keeps its children opaque and exposes typed accessors, exactly as [`a:tcPr`] and `a:ln`
//! do, so an `extLst`, an unmodelled child or an unknown attribute round-trips byte-for-byte. The
//! containers ([`TableStyleList`], [`TableStyle`]) reach their typed children by name.
//!
//! [`a:tcPr`]: super::TableCellProperties

use std::borrow::Cow;

use mjx_ooxml_core::{
    AttributeError, Enumeration, FromXml as _, Interner, RawAttribute, RawElement, RawName,
    RawNode, Text, ToXml as _,
};

use super::properties::{TablePart, TableProperties};
use mjx_ooxml_types::child_order::{
    TABLE_CELL_3D, TABLE_CELL_BORDER_STYLE, TABLE_PART_STYLE, TABLE_STYLE, TABLE_STYLE_CELL_STYLE,
    TABLE_STYLE_TEXT_STYLE,
};

use crate::build::{
    dml_child, dml_element, dml_name, fidelity_element_impls, first_fill_child, is_dml,
};

/// `a:tcTxStyle`'s two tri-state flags (`@b` / `@i`, `ST_OnOffStyleType`).
///
/// A face of their own rather than a declaration on [`TableStyleTextStyle`], because the public
/// pair maps the wire's third state onto the *absence* of the attribute: `def` is the schema
/// default, so [`OnOffStyle::Default`] reads from an absent attribute and writes by removing one.
#[derive(mjx_derive::XmlAttributes)]
#[xml(attribute(local = "b", codec = Enumeration<OnOffStyle>, accessor = bold))]
#[xml(attribute(local = "i", codec = Enumeration<OnOffStyle>, accessor = italic))]
struct TextStyleAttributes<A> {
    attributes: A,
}
use crate::color::{Color, ColorSpec};
use crate::effect::EffectList;
use crate::fill::{Fill, FillSpec};
use crate::line::{LineProperties, LineSpec};
use crate::shape3d::{build_bevel, build_light_rig, read_bevel, read_light_rig, Bevel, LightRig};
use crate::style::StyleMatrixReference;
use crate::theme::FontCollection;

use mjx_ooxml_types::drawingml::PresetMaterial;

pub use mjx_ooxml_types::drawingml::{FontCollectionIndex, OnOffStyle};

/// The first `EG_ColorChoice` child of `children` (`a:srgbClr`, `a:schemeClr`, …), read as a
/// [`Color`] — a table style's text colour or a font reference's tint.
fn first_color(children: &[RawNode], interner: &Interner) -> Option<Color> {
    children.iter().find_map(|node| match node {
        RawNode::Element(element)
            if is_dml(&element.name, interner)
                && Color::is_choice_local(interner.resolve(element.name.local)) =>
        {
            Color::from_xml(element, interner).ok()
        }
        _ => None,
    })
}

/// `a:tblStyleLst` (`CT_TableStyleList`) — every table style a presentation defines, and which is the
/// default.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "def", codec = Text, accessor = default_style_id, required))]
pub struct TableStyleList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableStyleList);

impl TableStyleList {
    /// Every style the list defines, in order.
    #[must_use]
    pub fn styles(&self, interner: &Interner) -> Vec<TableStyle> {
        self.children
            .iter()
            .filter_map(|node| match node {
                RawNode::Element(element)
                    if is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == "tblStyle" =>
                {
                    TableStyle::from_xml(element, interner).ok()
                }
                _ => None,
            })
            .collect()
    }

    /// The style whose `@styleId` is `style_id`, or `None` if the list defines no such style — which
    /// is how a dangling `a:tableStyleId` reads: a reference with nothing to resolve to.
    #[must_use]
    pub fn style(&self, interner: &Interner, style_id: &str) -> Option<TableStyle> {
        self.styles(interner)
            .into_iter()
            .find(|style| matches!(style.style_id(interner), Ok(id) if id == style_id))
    }

    /// The list's children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The list's children, mutably — for adding a style.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }
}

/// `a:tblStyle` (`CT_TableStyle`) — one named table style: its identity and the formatting it gives
/// each part of a table.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "styleId", codec = Text, accessor = style_id, required))]
#[xml(attribute(local = "styleName", codec = Text, accessor = style_name, required))]
pub struct TableStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableStyle);

impl TableStyle {
    /// The formatting the style gives `part` of a table (`a:wholeTbl`, `a:firstRow`, …), or `None` if
    /// it leaves that part unstyled.
    #[must_use]
    pub fn part(&self, interner: &Interner, part: TableStylePart) -> Option<TablePartStyle> {
        dml_child(&self.children, interner, part.wire())
            .and_then(|element| TablePartStyle::from_xml(element, interner).ok())
    }

    /// The style's whole-table background (`a:tblBg`), or `None` if it declares none.
    #[must_use]
    pub fn background(&self, interner: &Interner) -> Option<TableBackgroundStyle> {
        dml_child(&self.children, interner, "tblBg")
            .and_then(|element| TableBackgroundStyle::from_xml(element, interner).ok())
    }

    /// The style's children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The style's children, mutably — for setting a part.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }

    /// The style's attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }
}

/// A part of a table a style formats separately — the thirteen `CT_TablePartStyle` slots of
/// `a:tblStyle`.
///
/// A cell may belong to several at once (a header cell in a banded table); a renderer layers them
/// from the most general ([`WholeTable`](Self::WholeTable)) to the most specific (a corner cell),
/// which is the resolution a later tier performs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TableStylePart {
    /// `a:wholeTbl` — every cell of the table.
    WholeTable,
    /// `a:band1H` — the first of the two alternating horizontal (row) bands.
    Band1Horizontal,
    /// `a:band2H` — the second alternating horizontal (row) band.
    Band2Horizontal,
    /// `a:band1V` — the first of the two alternating vertical (column) bands.
    Band1Vertical,
    /// `a:band2V` — the second alternating vertical (column) band.
    Band2Vertical,
    /// `a:firstRow` — the header row.
    FirstRow,
    /// `a:lastRow` — the total row.
    LastRow,
    /// `a:firstCol` — the header column.
    FirstColumn,
    /// `a:lastCol` — the total column.
    LastColumn,
    /// `a:nwCell` — the top-left corner cell.
    NorthWestCell,
    /// `a:neCell` — the top-right corner cell.
    NorthEastCell,
    /// `a:swCell` — the bottom-left corner cell.
    SouthWestCell,
    /// `a:seCell` — the bottom-right corner cell.
    SouthEastCell,
}

impl TableStylePart {
    /// The element's local name, without its `a:` prefix.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::WholeTable => "wholeTbl",
            Self::Band1Horizontal => "band1H",
            Self::Band2Horizontal => "band2H",
            Self::Band1Vertical => "band1V",
            Self::Band2Vertical => "band2V",
            Self::FirstRow => "firstRow",
            Self::LastRow => "lastRow",
            Self::FirstColumn => "firstCol",
            Self::LastColumn => "lastCol",
            Self::NorthWestCell => "nwCell",
            Self::NorthEastCell => "neCell",
            Self::SouthWestCell => "swCell",
            Self::SouthEastCell => "seCell",
        }
    }

    /// Every part, in the order `CT_TableStyle`'s sequence declares them — which is also the order a
    /// new one must be inserted in, since sequence order is validity.
    #[must_use]
    pub fn all() -> [Self; 13] {
        [
            Self::WholeTable,
            Self::Band1Horizontal,
            Self::Band2Horizontal,
            Self::Band1Vertical,
            Self::Band2Vertical,
            Self::LastColumn,
            Self::FirstColumn,
            Self::LastRow,
            Self::SouthEastCell,
            Self::SouthWestCell,
            Self::FirstRow,
            Self::NorthEastCell,
            Self::NorthWestCell,
        ]
    }
}

/// The six `a:tblPr` banding/emphasis flags a table states — the flags that decide which style parts
/// a cell picks up. Unstated flags are `false`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TableStyleFlags {
    /// `@firstRow` — the table has a header row.
    pub first_row: bool,
    /// `@lastRow` — the table has a total row.
    pub last_row: bool,
    /// `@firstCol` — the table has a header column.
    pub first_column: bool,
    /// `@lastCol` — the table has a total column.
    pub last_column: bool,
    /// `@bandRow` — rows alternate between two banded formats.
    pub banded_rows: bool,
    /// `@bandCol` — columns alternate between two banded formats.
    pub banded_columns: bool,
}

impl TableStyleFlags {
    /// The flags a table's `a:tblPr` states (an unstated flag is `false`).
    #[must_use]
    pub fn from_properties(properties: &TableProperties, interner: &Interner) -> Self {
        Self {
            first_row: properties.has_part(interner, TablePart::FirstRow),
            last_row: properties.has_part(interner, TablePart::LastRow),
            first_column: properties.has_part(interner, TablePart::FirstColumn),
            last_column: properties.has_part(interner, TablePart::LastColumn),
            banded_rows: properties.has_part(interner, TablePart::BandedRows),
            banded_columns: properties.has_part(interner, TablePart::BandedColumns),
        }
    }
}

/// The style parts that cover the cell at `(row, column)` of a `rows`×`columns` table, **most
/// specific first** — the order a resolver tries them, and the reverse of the order a renderer layers
/// them.
///
/// The layering is fixed by ECMA-376 (§17.7.6: "these conditional formats shall be applied in the
/// following order … subsequent formats override previous"): whole table, banded columns, banded
/// rows, first/last row, first/last column, corner cells. Reversed, the precedence is corner cells >
/// first/last **column** > first/last **row** > **row** bands > **column** bands > `wholeTbl`.
///
/// - A corner part applies only where **both** its edge flags do (`nwCell` needs `firstRow` and
///   `firstCol` at `(0, 0)`); a corner cell still stacks the edge and whole-table parts beneath it.
/// - Banding covers only the **data** rows/columns — the first/last row/column, when flagged, are
///   excluded — and the first data row/column is `band1*` (an odd grouping), then `band2*`.
#[must_use]
pub fn applicable_parts(
    row: usize,
    column: usize,
    rows: usize,
    columns: usize,
    flags: TableStyleFlags,
) -> Vec<TableStylePart> {
    let is_first_row = flags.first_row && row == 0;
    let is_last_row = flags.last_row && row + 1 == rows;
    let is_first_col = flags.first_column && column == 0;
    let is_last_col = flags.last_column && column + 1 == columns;

    let mut parts = Vec::new();

    // Corner cells — highest precedence, and only where both edge flags meet.
    if is_first_row && is_first_col {
        parts.push(TableStylePart::NorthWestCell);
    } else if is_first_row && is_last_col {
        parts.push(TableStylePart::NorthEastCell);
    } else if is_last_row && is_first_col {
        parts.push(TableStylePart::SouthWestCell);
    } else if is_last_row && is_last_col {
        parts.push(TableStylePart::SouthEastCell);
    }

    // Column edges override row edges.
    if is_first_col {
        parts.push(TableStylePart::FirstColumn);
    }
    if is_last_col {
        parts.push(TableStylePart::LastColumn);
    }
    if is_first_row {
        parts.push(TableStylePart::FirstRow);
    }
    if is_last_row {
        parts.push(TableStylePart::LastRow);
    }

    // Row banding overrides column banding; both cover data cells only.
    if flags.banded_rows && !is_first_row && !is_last_row {
        let data_row = row - usize::from(flags.first_row);
        parts.push(if data_row.is_multiple_of(2) {
            TableStylePart::Band1Horizontal
        } else {
            TableStylePart::Band2Horizontal
        });
    }
    if flags.banded_columns && !is_first_col && !is_last_col {
        let data_column = column - usize::from(flags.first_column);
        parts.push(if data_column.is_multiple_of(2) {
            TableStylePart::Band1Vertical
        } else {
            TableStylePart::Band2Vertical
        });
    }

    // The whole table underlies everything.
    parts.push(TableStylePart::WholeTable);
    parts
}

/// `a:*` (`CT_TablePartStyle`) — the formatting a table style gives one part: its text style and its
/// cell style.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TablePartStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TablePartStyle);

impl TablePartStyle {
    /// How the part's text is styled (`a:tcTxStyle`), or `None` if it says nothing about text.
    #[must_use]
    pub fn text_style(&self, interner: &Interner) -> Option<TableStyleTextStyle> {
        dml_child(&self.children, interner, "tcTxStyle")
            .and_then(|element| TableStyleTextStyle::from_xml(element, interner).ok())
    }

    /// How the part's cells are styled (`a:tcStyle`) — fill and borders — or `None` if it says
    /// nothing about the cell.
    #[must_use]
    pub fn cell_style(&self, interner: &Interner) -> Option<TableStyleCellStyle> {
        dml_child(&self.children, interner, "tcStyle")
            .and_then(|element| TableStyleCellStyle::from_xml(element, interner).ok())
    }

    /// The part style's children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The part style's children, mutably.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }
}

/// `a:tcTxStyle` (`CT_TableStyleTextStyle`) — how a part's text is styled: its font, colour, and the
/// tri-state take on bold and italic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableStyleTextStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableStyleTextStyle);

impl TableStyleTextStyle {
    /// The style's take on **bold** (`@b`) — [`On`](OnOffStyle::On) to force it, [`Off`] to forbid
    /// it, [`Default`] (the wire and schema default) to follow the property inheritance chain.
    ///
    /// [`Off`]: OnOffStyle::Off
    /// [`Default`]: OnOffStyle::Default
    #[must_use]
    pub fn bold(&self, interner: &Interner) -> OnOffStyle {
        or_default(self.on_off_face().bold(interner))
    }

    /// This style's `@b` / `@i` face, borrowed.
    fn on_off_face(&self) -> TextStyleAttributes<&[RawAttribute]> {
        TextStyleAttributes {
            attributes: &self.attributes,
        }
    }

    /// The style's take on **italic** (`@i`); see [`bold`](Self::bold).
    #[must_use]
    pub fn italic(&self, interner: &Interner) -> OnOffStyle {
        or_default(self.on_off_face().italic(interner))
    }

    /// The text colour (`EG_ColorChoice`), or `None` if the style leaves it to be inherited.
    #[must_use]
    pub fn color(&self, interner: &Interner) -> Option<Color> {
        first_color(&self.children, interner)
    }

    /// The explicit font (`a:font`), or `None` — a style names a font either outright or by theme
    /// reference (see [`font_reference`](Self::font_reference)).
    #[must_use]
    pub fn font(&self, interner: &Interner) -> Option<FontCollection> {
        dml_child(&self.children, interner, "font")
            .map(|element| FontCollection::read(element, interner))
    }

    /// The theme font reference (`a:fontRef`), or `None`.
    #[must_use]
    pub fn font_reference(&self, interner: &Interner) -> Option<FontReference> {
        dml_child(&self.children, interner, "fontRef")
            .and_then(|element| FontReference::from_xml(element, interner).ok())
    }

    /// The text style's children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The text style's children, mutably.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }

    /// The text style's attributes, verbatim.
    #[must_use]
    pub fn attributes(&self) -> &[RawAttribute] {
        &self.attributes
    }
}

/// `a:tcStyle` (`CT_TableStyleCellStyle`) — how a part's cells are drawn: their borders, fill, and
/// optional 3-D bevel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableStyleCellStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableStyleCellStyle);

impl TableStyleCellStyle {
    /// The cell borders (`a:tcBdr`), or `None` if the style states none.
    #[must_use]
    pub fn borders(&self, interner: &Interner) -> Option<TableCellBorderStyle> {
        dml_child(&self.children, interner, "tcBdr")
            .and_then(|element| TableCellBorderStyle::from_xml(element, interner).ok())
    }

    /// The explicit cell fill (`a:fill`, wrapping an `EG_FillProperties`), or `None` — a cell style
    /// fills either outright or by theme reference (see [`fill_reference`](Self::fill_reference)).
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<Fill> {
        let fill = dml_child(&self.children, interner, "fill")?;
        first_fill_child(&fill.children, interner)
            .and_then(|element| Fill::from_xml(element, interner).ok())
    }

    /// The theme fill reference (`a:fillRef`), or `None`.
    #[must_use]
    pub fn fill_reference(&self, interner: &Interner) -> Option<StyleMatrixReference> {
        dml_child(&self.children, interner, "fillRef")
            .and_then(|element| StyleMatrixReference::from_xml(element, interner).ok())
    }

    /// The cell's 3-D bevel (`a:cell3D`), or `None`.
    #[must_use]
    pub fn cell_3d(&self, interner: &Interner) -> Option<Cell3D> {
        dml_child(&self.children, interner, "cell3D")
            .and_then(|element| Cell3D::from_xml(element, interner).ok())
    }

    /// The cell style's children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The cell style's children, mutably.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }
}

/// One of the eight edges a table style's cell borders describe — the children of `a:tcBdr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TableStyleBorder {
    /// `a:left` — the left edge.
    Left,
    /// `a:right` — the right edge.
    Right,
    /// `a:top` — the top edge.
    Top,
    /// `a:bottom` — the bottom edge.
    Bottom,
    /// `a:insideH` — the horizontal edges *between* rows.
    InsideHorizontal,
    /// `a:insideV` — the vertical edges *between* columns.
    InsideVertical,
    /// `a:tl2br` — the diagonal from the top-left corner to the bottom-right.
    TopLeftToBottomRight,
    /// `a:tr2bl` — the diagonal from the top-right corner to the bottom-left.
    TopRightToBottomLeft,
}

impl TableStyleBorder {
    /// The element's local name, without its `a:` prefix.
    #[must_use]
    pub fn wire(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
            Self::Top => "top",
            Self::Bottom => "bottom",
            Self::InsideHorizontal => "insideH",
            Self::InsideVertical => "insideV",
            Self::TopLeftToBottomRight => "tl2br",
            Self::TopRightToBottomLeft => "tr2bl",
        }
    }

    /// Every edge, in `CT_TableCellBorderStyle`'s sequence order.
    #[must_use]
    pub fn all() -> [Self; 8] {
        [
            Self::Left,
            Self::Right,
            Self::Top,
            Self::Bottom,
            Self::InsideHorizontal,
            Self::InsideVertical,
            Self::TopLeftToBottomRight,
            Self::TopRightToBottomLeft,
        ]
    }
}

/// `a:tcBdr` (`CT_TableCellBorderStyle`) — the eight border edges a cell style may describe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableCellBorderStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableCellBorderStyle);

impl TableCellBorderStyle {
    /// The line on `edge`, or `None` if the style leaves that edge alone.
    #[must_use]
    pub fn border(
        &self,
        interner: &Interner,
        edge: TableStyleBorder,
    ) -> Option<ThemeableLineStyle> {
        dml_child(&self.children, interner, edge.wire())
            .and_then(|element| ThemeableLineStyle::from_edge(element, interner))
    }

    /// The border set's children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The border set's children, mutably.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }
}

/// `CT_ThemeableLineStyle` — a table-style line given **explicitly** or as a **reference into the
/// theme's line style matrix**. The two ways a table style names a border.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeableLineStyle {
    /// An explicit line (`a:ln`).
    Line(LineProperties),
    /// A reference into the theme's line style matrix (`a:lnRef`).
    Reference(StyleMatrixReference),
}

impl ThemeableLineStyle {
    /// Reads the `a:ln` / `a:lnRef` inside a border-edge element (`a:left`, `a:top`, …).
    fn from_edge(edge: &RawElement, interner: &Interner) -> Option<Self> {
        if let Some(line) = dml_child(&edge.children, interner, "ln") {
            return LineProperties::from_xml(line, interner)
                .ok()
                .map(Self::Line);
        }
        if let Some(reference) = dml_child(&edge.children, interner, "lnRef") {
            return StyleMatrixReference::from_xml(reference, interner)
                .ok()
                .map(Self::Reference);
        }
        None
    }
}

/// `a:tblBg` (`CT_TableBackgroundStyle`) — the fill and effects drawn behind the whole table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableBackgroundStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableBackgroundStyle);

impl TableBackgroundStyle {
    /// The explicit background fill (`a:fill`, wrapping an `EG_FillProperties`), or `None`.
    #[must_use]
    pub fn fill(&self, interner: &Interner) -> Option<Fill> {
        let fill = dml_child(&self.children, interner, "fill")?;
        first_fill_child(&fill.children, interner)
            .and_then(|element| Fill::from_xml(element, interner).ok())
    }

    /// The theme background-fill reference (`a:fillRef`), or `None`.
    #[must_use]
    pub fn fill_reference(&self, interner: &Interner) -> Option<StyleMatrixReference> {
        dml_child(&self.children, interner, "fillRef")
            .and_then(|element| StyleMatrixReference::from_xml(element, interner).ok())
    }

    /// The explicit background effects (`a:effect > a:effectLst`), or `None` — including when the
    /// background instead carries the rarer `a:effectDag`, which is preserved but not modelled.
    #[must_use]
    pub fn effect(&self, interner: &Interner) -> Option<EffectList> {
        let effect = dml_child(&self.children, interner, "effect")?;
        dml_child(&effect.children, interner, "effectLst")
            .and_then(|element| EffectList::from_xml(element, interner).ok())
    }

    /// The theme background-effect reference (`a:effectRef`), or `None`.
    #[must_use]
    pub fn effect_reference(&self, interner: &Interner) -> Option<StyleMatrixReference> {
        dml_child(&self.children, interner, "effectRef")
            .and_then(|element| StyleMatrixReference::from_xml(element, interner).ok())
    }

    /// The background's children, verbatim.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// The background's children, mutably.
    pub fn children_mut(&mut self) -> &mut Vec<RawNode> {
        &mut self.children
    }
}

/// `a:fontRef` (`CT_FontReference`) — a reference to one of the theme's font slots, optionally
/// tinted.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "idx", codec = Enumeration<FontCollectionIndex>, accessor = index))]
pub struct FontReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(FontReference);

impl FontReference {
    /// The tint applied to the referenced font (`EG_ColorChoice`), or `None`.
    #[must_use]
    pub fn color(&self, interner: &Interner) -> Option<Color> {
        first_color(&self.children, interner)
    }
}

/// `a:cell3D` (`CT_Cell3D`) — a cell's 3-D bevel and lighting.
///
/// A fidelity wrapper: the preset material, the `a:bevel` and the optional `a:lightRig` are read
/// typed (reusing the DrawingML 3-D model in [`crate::shape3d`]); any `a:extLst` stays opaque and
/// re-emits verbatim, so the element round-trips byte-for-byte. Unlike `a:sp3d`, a cell carries a
/// single `a:bevel` (not a top and bottom), which the schema requires.
///
/// `@prstMaterial` is declared as [`Text`], so that both readings of the same bytes are available:
/// the typed [`material`](Self::material) and the raw [`preset_material`](Self::preset_material).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::XmlAttributes)]
#[xml(attribute(local = "prstMaterial", codec = Text, accessor = preset_material))]
pub struct Cell3D {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(Cell3D);

impl Cell3D {
    /// The preset material the cell's surface imitates (`@prstMaterial`; schema default `plastic`),
    /// typed. `None` when unstated or when the value is not a known `ST_PresetMaterialType`. This is
    /// the normal accessor and mirrors [`Shape3D::material`](crate::Shape3D::material); reach for
    /// [`preset_material`](Self::preset_material) only to see a non-conforming raw token.
    #[must_use]
    pub fn material(&self, interner: &Interner) -> Option<PresetMaterial> {
        self.preset_material(interner)
            .ok()
            .flatten()
            .as_deref()
            .and_then(PresetMaterial::from_wire)
    }

    /// The cell's bevel (`a:bevel`), or `None` if absent (a malformed cell without the schema-required
    /// bevel).
    #[must_use]
    pub fn bevel(&self, interner: &Interner) -> Option<Bevel> {
        dml_child(&self.children, interner, "bevel").map(|element| read_bevel(element, interner))
    }

    /// The cell's light rig (`a:lightRig`), or `None` if absent or stating no rig / direction.
    #[must_use]
    pub fn light_rig(&self, interner: &Interner) -> Option<LightRig> {
        dml_child(&self.children, interner, "lightRig")
            .and_then(|element| read_light_rig(element, interner))
    }

    /// The cell-3D's children, verbatim — the modeled `a:bevel`/`a:lightRig` plus any opaque
    /// `a:extLst`.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }

    /// A fresh `a:cell3D`, seeded with an empty `<a:bevel/>` — `CT_Cell3D` requires a bevel, and every
    /// `CT_Bevel` attribute is optional, so a stated-nothing bevel is the valid empty starting point.
    /// Refine it with [`set_material`](Self::set_material), [`set_bevel`](Self::set_bevel) and
    /// [`set_light_rig`](Self::set_light_rig).
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        let bevel = RawNode::Element(dml_element(interner, "bevel", Vec::new(), Vec::new()));
        Self {
            name: dml_name(interner, "cell3D"),
            attributes: Vec::new(),
            children: vec![bevel],
            empty: false,
        }
    }

    /// Sets the surface material (`@prstMaterial`).
    pub fn set_material(&mut self, interner: &mut Interner, material: PresetMaterial) {
        self.set_preset_material(interner, Some(material.to_wire()));
    }

    /// Sets the cell's bevel (`a:bevel`), replacing the existing one in place.
    pub fn set_bevel(&mut self, interner: &mut Interner, bevel: &Bevel) {
        let element = build_bevel(interner, "bevel", bevel);
        TABLE_CELL_3D.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "bevel"
        });
        self.empty = false;
    }

    /// Sets the cell's light rig (`a:lightRig`), replacing any existing one in place.
    pub fn set_light_rig(&mut self, interner: &mut Interner, light_rig: &LightRig) {
        let element = build_light_rig(interner, light_rig);
        TABLE_CELL_3D.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "lightRig"
        });
        self.empty = false;
    }
}

// =================================================================================================
// Authoring — building a table style up from parts.
//
// Every setter is **merge, not rebuild**: a child is replaced in place or inserted at its rank in the
// schema sequence, so content this tier does not model (an `extLst`, a `cell3D`, an unknown child)
// survives. The ranks below *are* those sequences — extend them, never append.
// =================================================================================================

/// An `ST_OnOffStyleType` read as the tri-state it is: the schema default `def` for an absent
/// attribute, and for one this model cannot read — the attribute round-trips verbatim either way.
fn or_default(read: Result<Option<OnOffStyle>, AttributeError>) -> OnOffStyle {
    read.ok().flatten().unwrap_or(OnOffStyle::Default)
}

/// [`OnOffStyle`] as the setters want it: `Default` is the *absence* of a claim, so it removes the
/// attribute rather than writing `b="def"`.
fn stated(value: OnOffStyle) -> Option<OnOffStyle> {
    match value {
        OnOffStyle::Default => None,
        other => Some(other),
    }
}

/// The `@styleId` of an `a:tblStyle` element this list has not built a [`TableStyle`] for yet — one
/// call to the workspace's single attribute read, on the attribute `TableStyle` declares.
fn style_id_of<'a>(element: &'a RawElement, interner: &Interner) -> Option<Cow<'a, str>> {
    mjx_xml::attribute::read::<Text>(&element.attributes, interner, None, "styleId", "styleId")
        .ok()
        .flatten()
}

impl TableStyleList {
    /// A fresh, empty `a:tblStyleLst` whose default style (`@def`) is `default_style_id`.
    #[must_use]
    pub fn new(interner: &mut Interner, default_style_id: &str) -> Self {
        let mut list = Self {
            name: dml_name(interner, "tblStyleLst"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: false,
        };
        list.set_default_style_id(interner, default_style_id);
        list
    }

    /// Adds `style`, replacing any existing style with the same `@styleId` in place — so authoring
    /// the same style twice updates it rather than duplicating it.
    pub fn upsert_style(&mut self, interner: &mut Interner, style: &TableStyle) {
        if let Some(style_id) = style.style_id(interner).ok().map(Cow::into_owned) {
            let existing = self.children.iter().position(|node| match node {
                RawNode::Element(element) => {
                    is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == "tblStyle"
                        && style_id_of(element, interner).as_deref() == Some(style_id.as_str())
                }
                _ => false,
            });
            let element = RawNode::Element(style.to_xml(interner));
            match existing {
                Some(index) => self.children[index] = element,
                None => self.children.push(element),
            }
        } else {
            self.children.push(RawNode::Element(style.to_xml(interner)));
        }
        self.empty = false;
    }
}

impl TableStyle {
    /// A fresh, empty `a:tblStyle` with the given GUID and gallery name.
    #[must_use]
    pub fn new(interner: &mut Interner, style_id: &str, style_name: &str) -> Self {
        let mut style = Self {
            name: dml_name(interner, "tblStyle"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: false,
        };
        style.set_style_id(interner, style_id);
        style.set_style_name(interner, style_name);
        style
    }

    /// Sets the formatting for `part`, replacing whatever the slot held.
    pub fn set_part(
        &mut self,
        interner: &mut Interner,
        part: TableStylePart,
        part_style: &TablePartStyle,
    ) {
        let mut element = part_style.to_xml(interner);
        element.name = dml_name(interner, part.wire());
        let wire = part.wire();
        TABLE_STYLE.replace_or_insert(&mut self.children, interner, element, |local| local == wire);
        self.empty = false;
    }

    /// Sets the whole-table background (`a:tblBg`).
    pub fn set_background(&mut self, interner: &mut Interner, background: &TableBackgroundStyle) {
        let mut element = background.to_xml(interner);
        element.name = dml_name(interner, "tblBg");
        TABLE_STYLE.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "tblBg"
        });
        self.empty = false;
    }
}

impl TablePartStyle {
    /// A fresh, empty part style. Its slot name is set when [`TableStyle::set_part`] places it.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dml_name(interner, "wholeTbl"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: false,
        }
    }

    /// Sets the part's text style (`a:tcTxStyle`).
    pub fn set_text_style(&mut self, interner: &mut Interner, text: &TableStyleTextStyle) {
        let mut element = text.to_xml(interner);
        element.name = dml_name(interner, "tcTxStyle");
        TABLE_PART_STYLE.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "tcTxStyle"
        });
        self.empty = false;
    }

    /// Sets the part's cell style (`a:tcStyle`).
    pub fn set_cell_style(&mut self, interner: &mut Interner, cell: &TableStyleCellStyle) {
        let mut element = cell.to_xml(interner);
        element.name = dml_name(interner, "tcStyle");
        TABLE_PART_STYLE.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "tcStyle"
        });
        self.empty = false;
    }
}

impl TableStyleTextStyle {
    /// A fresh, empty text style — bold and italic follow the parent, no colour or font stated.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dml_name(interner, "tcTxStyle"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: false,
        }
    }

    /// Sets the take on bold (`@b`), **removing** it for [`OnOffStyle::Default`] — the wire and
    /// schema default is `def`, so "follow the parent" is the absence of a claim.
    pub fn set_bold(&mut self, interner: &mut Interner, value: OnOffStyle) {
        TextStyleAttributes {
            attributes: &mut self.attributes,
        }
        .set_bold(interner, stated(value));
    }

    /// Sets the take on italic (`@i`); see [`set_bold`](Self::set_bold).
    pub fn set_italic(&mut self, interner: &mut Interner, value: OnOffStyle) {
        TextStyleAttributes {
            attributes: &mut self.attributes,
        }
        .set_italic(interner, stated(value));
    }

    /// Sets the text colour (`EG_ColorChoice`).
    pub fn set_color(&mut self, interner: &mut Interner, color: &ColorSpec) {
        if let Some(color) = Color::from_spec(interner, color) {
            let element = color.to_xml(interner);
            TABLE_STYLE_TEXT_STYLE.replace_or_insert(
                &mut self.children,
                interner,
                element,
                Color::is_choice_local,
            );
            self.empty = false;
        }
    }
}

impl TableStyleCellStyle {
    /// A fresh, empty cell style.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dml_name(interner, "tcStyle"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: false,
        }
    }

    /// Sets the cell fill (`a:fill` wrapping an `EG_FillProperties`), replacing any explicit fill or
    /// theme fill reference.
    pub fn set_fill(&mut self, interner: &mut Interner, fill: &FillSpec) {
        let group = fill.to_fill(interner).to_xml(interner);
        let wrapper = dml_element(interner, "fill", Vec::new(), vec![RawNode::Element(group)]);
        TABLE_STYLE_CELL_STYLE.replace_or_insert(&mut self.children, interner, wrapper, |local| {
            local == "fill" || local == "fillRef"
        });
        self.empty = false;
    }

    /// Sets the line on one border `edge`, creating the `a:tcBdr` set if the style had none.
    pub fn set_border(&mut self, interner: &mut Interner, edge: TableStyleBorder, line: &LineSpec) {
        let mut borders = dml_child(&self.children, interner, "tcBdr")
            .and_then(|element| TableCellBorderStyle::from_xml(element, interner).ok())
            .unwrap_or_else(|| TableCellBorderStyle::new(interner));
        borders.set_border(interner, edge, line);
        let element = borders.to_xml(interner);
        TABLE_STYLE_CELL_STYLE.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "tcBdr"
        });
        self.empty = false;
    }

    /// Sets the cell's 3-D (`a:cell3D`), replacing any existing one in place.
    pub fn set_cell_3d(&mut self, interner: &mut Interner, cell_3d: &Cell3D) {
        let element = cell_3d.to_xml(interner);
        TABLE_STYLE_CELL_STYLE.replace_or_insert(&mut self.children, interner, element, |local| {
            local == "cell3D"
        });
        self.empty = false;
    }
}

impl TableCellBorderStyle {
    /// A fresh, empty border set.
    #[must_use]
    pub fn new(interner: &mut Interner) -> Self {
        Self {
            name: dml_name(interner, "tcBdr"),
            attributes: Vec::new(),
            children: Vec::new(),
            empty: false,
        }
    }

    /// Sets the line on `edge` — an explicit `a:ln` inside the edge element.
    pub fn set_border(&mut self, interner: &mut Interner, edge: TableStyleBorder, line: &LineSpec) {
        let mut ln = line.to_line(interner).to_xml(interner);
        ln.name = dml_name(interner, "ln");
        let edge_element = dml_element(
            interner,
            edge.wire(),
            Vec::new(),
            vec![RawNode::Element(ln)],
        );
        let wire = edge.wire();
        TABLE_CELL_BORDER_STYLE.replace_or_insert(
            &mut self.children,
            interner,
            edge_element,
            |local| local == wire,
        );
        self.empty = false;
    }
}
