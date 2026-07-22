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

use mjx_ooxml_core::{
    FromXml as _, Interner, RawAttribute, RawElement, RawName, RawNode, ToXml as _,
};

use crate::build::{
    attr_str, dml_attr, dml_child, dml_element, dml_name, fidelity_element_impls, first_fill_child,
    is_dml, replace_or_insert_child, set_attr,
};
use crate::color::{Color, ColorSpec};
use crate::effect::EffectList;
use crate::fill::{Fill, FillSpec};
use crate::line::{LineProperties, LineSpec};
use crate::style::StyleMatrixReference;
use crate::theme::FontCollection;

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableStyleList {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableStyleList);

impl TableStyleList {
    /// The GUID of the default style (`@def`) — the one a table with no `a:tableStyleId` of its own
    /// takes.
    #[must_use]
    pub fn default_style_id<'a>(&'a self, interner: &'a Interner) -> Option<&'a str> {
        attr_str(&self.attributes, interner, "def")
    }

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
            .find(|style| style.style_id(interner) == Some(style_id))
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableStyle {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(TableStyle);

impl TableStyle {
    /// The style's GUID (`@styleId`) — what a table's `a:tableStyleId` names.
    #[must_use]
    pub fn style_id<'a>(&'a self, interner: &'a Interner) -> Option<&'a str> {
        attr_str(&self.attributes, interner, "styleId")
    }

    /// The style's human-readable name (`@styleName`), as shown in a designer's style gallery.
    #[must_use]
    pub fn style_name<'a>(&'a self, interner: &'a Interner) -> Option<&'a str> {
        attr_str(&self.attributes, interner, "styleName")
    }

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

    /// This part's rank in `CT_TableStyle`'s sequence (`tblBg` is `0`; the parts follow). Order is
    /// validity, so a newly inserted part is placed by this rather than appended.
    #[must_use]
    pub fn rank(self) -> usize {
        // The XSD order: tblBg, wholeTbl, band1H, band2H, band1V, band2V, lastCol, firstCol, lastRow,
        // seCell, swCell, firstRow, neCell, nwCell.
        match self {
            Self::WholeTable => 1,
            Self::Band1Horizontal => 2,
            Self::Band2Horizontal => 3,
            Self::Band1Vertical => 4,
            Self::Band2Vertical => 5,
            Self::LastColumn => 6,
            Self::FirstColumn => 7,
            Self::LastRow => 8,
            Self::SouthEastCell => 9,
            Self::SouthWestCell => 10,
            Self::FirstRow => 11,
            Self::NorthEastCell => 12,
            Self::NorthWestCell => 13,
        }
    }
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
        attr_str(&self.attributes, interner, "b")
            .and_then(OnOffStyle::from_wire)
            .unwrap_or(OnOffStyle::Default)
    }

    /// The style's take on **italic** (`@i`); see [`bold`](Self::bold).
    #[must_use]
    pub fn italic(&self, interner: &Interner) -> OnOffStyle {
        attr_str(&self.attributes, interner, "i")
            .and_then(OnOffStyle::from_wire)
            .unwrap_or(OnOffStyle::Default)
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontReference {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(FontReference);

impl FontReference {
    /// Which theme font slot this names (`@idx`: `major` / `minor` / `none`), or `None` if unstated
    /// or unrecognised.
    #[must_use]
    pub fn index(&self, interner: &Interner) -> Option<FontCollectionIndex> {
        attr_str(&self.attributes, interner, "idx").and_then(FontCollectionIndex::from_wire)
    }

    /// The tint applied to the referenced font (`EG_ColorChoice`), or `None`.
    #[must_use]
    pub fn color(&self, interner: &Interner) -> Option<Color> {
        first_color(&self.children, interner)
    }
}

/// `a:cell3D` (`CT_Cell3D`) — a cell's 3-D bevel and lighting.
///
/// The preset material is exposed; the `a:bevel` and `a:lightRig` children are **preserved opaque**
/// pending the DrawingML 3-D subsystem (its own workstream), so they round-trip untouched but are not
/// yet decomposed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cell3D {
    name: RawName,
    attributes: Vec<RawAttribute>,
    children: Vec<RawNode>,
    empty: bool,
}

fidelity_element_impls!(Cell3D);

impl Cell3D {
    /// The preset material the cell's surface imitates (`@prstMaterial`; wire default `plastic`), as
    /// its raw wire token — the typed `ST_PresetMaterialType` arrives with the 3-D workstream.
    #[must_use]
    pub fn preset_material<'a>(&'a self, interner: &'a Interner) -> Option<&'a str> {
        attr_str(&self.attributes, interner, "prstMaterial")
    }

    /// The cell-3D's children, verbatim — the still-opaque `a:bevel` and `a:lightRig`.
    #[must_use]
    pub fn children(&self) -> &[RawNode] {
        &self.children
    }
}

// =================================================================================================
// Authoring — building a table style up from parts.
//
// Every setter is **merge, not rebuild**: a child is replaced in place or inserted at its rank in the
// schema sequence, so content this tier does not model (an `extLst`, a `cell3D`, an unknown child)
// survives. The ranks below *are* those sequences — extend them, never append.
// =================================================================================================

/// A child's rank in `CT_TableStyle`'s sequence: `tblBg`, the thirteen part slots, then `extLst`.
fn table_style_child_rank(local: &str) -> Option<usize> {
    if local == "tblBg" {
        return Some(0);
    }
    if let Some(part) = TableStylePart::all()
        .into_iter()
        .find(|p| p.wire() == local)
    {
        return Some(part.rank());
    }
    if local == "extLst" {
        return Some(14);
    }
    None
}

/// A child's rank in `CT_TablePartStyle`'s sequence: `tcTxStyle`, then `tcStyle`.
fn part_style_child_rank(local: &str) -> Option<usize> {
    match local {
        "tcTxStyle" => Some(0),
        "tcStyle" => Some(1),
        _ => None,
    }
}

/// A child's rank in `CT_TableStyleCellStyle`'s sequence: `tcBdr`, the fill choice, then `cell3D`.
fn cell_style_child_rank(local: &str) -> Option<usize> {
    match local {
        "tcBdr" => Some(0),
        "fill" | "fillRef" => Some(1),
        "cell3D" => Some(2),
        _ => None,
    }
}

/// A child's rank in `CT_TableStyleTextStyle`'s sequence: the font choice, the colour, then `extLst`.
fn text_style_child_rank(local: &str) -> Option<usize> {
    if local == "font" || local == "fontRef" {
        return Some(0);
    }
    if Color::is_choice_local(local) {
        return Some(1);
    }
    if local == "extLst" {
        return Some(2);
    }
    None
}

/// A child's rank in `CT_TableCellBorderStyle`'s sequence: the eight edges, then `extLst`.
fn border_style_child_rank(local: &str) -> Option<usize> {
    match local {
        "left" => Some(0),
        "right" => Some(1),
        "top" => Some(2),
        "bottom" => Some(3),
        "insideH" => Some(4),
        "insideV" => Some(5),
        "tl2br" => Some(6),
        "tr2bl" => Some(7),
        "extLst" => Some(8),
        _ => None,
    }
}

/// Removes an unprefixed attribute, if present.
fn remove_unprefixed_attr(attributes: &mut Vec<RawAttribute>, interner: &Interner, local: &str) {
    attributes.retain(|attribute| {
        attribute.name.prefix.is_some() || interner.resolve(attribute.name.local) != local
    });
}

/// Sets an `ST_OnOffStyleType` attribute (`@b` / `@i`), **removing** it for [`OnOffStyle::Default`]
/// — the wire and schema default is `def`, so "follow the parent" is the absence of a claim.
fn set_on_off(
    attributes: &mut Vec<RawAttribute>,
    interner: &mut Interner,
    local: &str,
    value: OnOffStyle,
) {
    match value {
        OnOffStyle::Default => remove_unprefixed_attr(attributes, interner, local),
        other => set_attr(attributes, interner, local, other.to_wire()),
    }
}

impl TableStyleList {
    /// A fresh, empty `a:tblStyleLst` whose default style (`@def`) is `default_style_id`.
    #[must_use]
    pub fn new(interner: &mut Interner, default_style_id: &str) -> Self {
        Self {
            name: dml_name(interner, "tblStyleLst"),
            attributes: vec![dml_attr(interner, "def", default_style_id)],
            children: Vec::new(),
            empty: false,
        }
    }

    /// Sets the default style GUID (`@def`).
    pub fn set_default_style_id(&mut self, interner: &mut Interner, style_id: &str) {
        set_attr(&mut self.attributes, interner, "def", style_id);
    }

    /// Adds `style`, replacing any existing style with the same `@styleId` in place — so authoring
    /// the same style twice updates it rather than duplicating it.
    pub fn upsert_style(&mut self, interner: &mut Interner, style: &TableStyle) {
        if let Some(style_id) = style.style_id(interner).map(str::to_owned) {
            let existing = self.children.iter().position(|node| match node {
                RawNode::Element(element) => {
                    is_dml(&element.name, interner)
                        && interner.resolve(element.name.local) == "tblStyle"
                        && attr_str(&element.attributes, interner, "styleId") == Some(&style_id)
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
        Self {
            name: dml_name(interner, "tblStyle"),
            attributes: vec![
                dml_attr(interner, "styleId", style_id),
                dml_attr(interner, "styleName", style_name),
            ],
            children: Vec::new(),
            empty: false,
        }
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
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            |local| local == wire,
            table_style_child_rank,
        );
        self.empty = false;
    }

    /// Sets the whole-table background (`a:tblBg`).
    pub fn set_background(&mut self, interner: &mut Interner, background: &TableBackgroundStyle) {
        let mut element = background.to_xml(interner);
        element.name = dml_name(interner, "tblBg");
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            |local| local == "tblBg",
            table_style_child_rank,
        );
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
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            |local| local == "tcTxStyle",
            part_style_child_rank,
        );
        self.empty = false;
    }

    /// Sets the part's cell style (`a:tcStyle`).
    pub fn set_cell_style(&mut self, interner: &mut Interner, cell: &TableStyleCellStyle) {
        let mut element = cell.to_xml(interner);
        element.name = dml_name(interner, "tcStyle");
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            |local| local == "tcStyle",
            part_style_child_rank,
        );
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

    /// Sets the take on bold (`@b`).
    pub fn set_bold(&mut self, interner: &mut Interner, value: OnOffStyle) {
        set_on_off(&mut self.attributes, interner, "b", value);
    }

    /// Sets the take on italic (`@i`).
    pub fn set_italic(&mut self, interner: &mut Interner, value: OnOffStyle) {
        set_on_off(&mut self.attributes, interner, "i", value);
    }

    /// Sets the text colour (`EG_ColorChoice`).
    pub fn set_color(&mut self, interner: &mut Interner, color: &ColorSpec) {
        if let Some(color) = Color::from_spec(interner, color) {
            let element = color.to_xml(interner);
            replace_or_insert_child(
                &mut self.children,
                interner,
                element,
                Color::is_choice_local,
                text_style_child_rank,
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
        replace_or_insert_child(
            &mut self.children,
            interner,
            wrapper,
            |local| local == "fill" || local == "fillRef",
            cell_style_child_rank,
        );
        self.empty = false;
    }

    /// Sets the line on one border `edge`, creating the `a:tcBdr` set if the style had none.
    pub fn set_border(&mut self, interner: &mut Interner, edge: TableStyleBorder, line: &LineSpec) {
        let mut borders = dml_child(&self.children, interner, "tcBdr")
            .and_then(|element| TableCellBorderStyle::from_xml(element, interner).ok())
            .unwrap_or_else(|| TableCellBorderStyle::new(interner));
        borders.set_border(interner, edge, line);
        let element = borders.to_xml(interner);
        replace_or_insert_child(
            &mut self.children,
            interner,
            element,
            |local| local == "tcBdr",
            cell_style_child_rank,
        );
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
        replace_or_insert_child(
            &mut self.children,
            interner,
            edge_element,
            |local| local == wire,
            border_style_child_rank,
        );
        self.empty = false;
    }
}
