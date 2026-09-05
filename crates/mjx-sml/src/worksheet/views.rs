//! What a consumer *shows* of a sheet: the tab's own properties, the sheet views, the frozen or
//! split pane, and the selection inside each pane.
//!
//! Five complex types, all reached from two of [`WorksheetPart`](crate::WorksheetPart)'s
//! thirty-nine slots — `sheetPr` (rank 0) and `sheetViews` (rank 2):
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_SheetPr` | 2306 | `x:sheetPr` |
//! | `CT_OutlinePr` | 2423 | `x:sheetPr/outlinePr` |
//! | `CT_PageSetUpPr` | 2429 | `x:sheetPr/pageSetUpPr` |
//! | `CT_SheetViews` | 2326 | `x:sheetViews` |
//! | `CT_SheetView` | 2332 | `x:sheetViews/sheetView` |
//! | `CT_Pane` | 2359 | `x:sheetView/pane` |
//! | `CT_Selection` | 2388 | `x:sheetView/selection` |
//! | `CT_PivotSelection` | 2366 | `x:sheetView/pivotSelection` |
//!
//! # Why `tabColor` is a preserved element and a decoded snapshot, not one type
//!
//! `sheetPr/tabColor` is a `CT_Color`, and [`Color`](crate::Color) — MJXOFF-97's — already decodes
//! that type. But `Color` is a *snapshot*: five `Option` fields, read out of the attributes it
//! recognises. Holding the tab colour as a `Color` and writing it back from one would drop any
//! attribute this project has not heard of, and would re-order and re-quote the ones it has.
//!
//! So the element is held as [`ColorElement`], which keeps the file's own attribute vector, order,
//! quoting and prefixes, and [`SheetProperties::tab_colour`] decodes it through [`Color`] on demand.
//! Preservation and interpretation are different jobs and this is the one place in the worksheet
//! spine where both are wanted at once.
//!
//! MJXOFF-102 declared that holder as a `tabColor` attribute bag of its own. MJXOFF-105 found four
//! more slots of the same complex type in `styles.xml` — a font's `color`, a pattern fill's
//! `fgColor` and `bgColor`, a border edge's and a gradient stop's `color` — and replaced the bag
//! with the one [`ColorElement`], which carries whichever local name the file wrote. Four bag types
//! for one complex type is the duplication this crate already has a scheduled child to undo once.
//!
//! # `sheetView` selections: up to four, and the `pane` attribute is which one
//!
//! `CT_SheetView` declares `selection` `maxOccurs="4"` and `pivotSelection` `maxOccurs="4"`, one per
//! pane of a split. So [`SheetView::selections`] is an iterator, never an `Option`: a sheet frozen
//! at `B2` carries four `<selection>` elements and a model that kept one would silently discard
//! three. `sample.xlsx` writes exactly one, `pane="topLeft"`.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::spreadsheetml::{Pane, PaneState, PivotTableAxis, SheetViewType};
use mjx_ooxml_types::support::OnOff;

use crate::address::{CellRange, CellRangeList, CellReference};
use crate::font::{Color, ColorElement};
use crate::leaf::attribute_bag;

use super::rebuild_element;

attribute_bag! {
    /// `x:sheetPr/outlinePr` (`CT_OutlinePr`, `sml.xsd:2423`) — where an outline's summary row and
    /// column sit relative to the rows they summarise.
    ///
    /// `summaryBelow` and `summaryRight` both default to `true`, which is Excel's own default and
    /// the opposite of what a reader who assumes "absent means off" would guess.
    #[xml(attribute(local = "applyStyles", codec = OnOff, accessor = applies_outline_styles, default = false))]
    #[xml(attribute(local = "summaryBelow", codec = OnOff, accessor = summary_row_below_detail, default = true))]
    #[xml(attribute(local = "summaryRight", codec = OnOff, accessor = summary_column_right_of_detail, default = true))]
    #[xml(attribute(local = "showOutlineSymbols", codec = OnOff, accessor = show_outline_symbols, default = true))]
    OutlineProperties, "outlinePr"
}

attribute_bag! {
    /// `x:sheetPr/pageSetUpPr` (`CT_PageSetUpPr`, `sml.xsd:2429`) — whether the sheet shows
    /// automatic page breaks, and whether its print setup is "fit to page" rather than scaled.
    ///
    /// `sample.xlsx` writes `<pageSetUpPr fitToPage="false"/>` — present, and saying the default.
    #[xml(attribute(local = "autoPageBreaks", codec = OnOff, accessor = shows_automatic_page_breaks, default = true))]
    #[xml(attribute(local = "fitToPage", codec = OnOff, accessor = fit_to_page, default = false))]
    PageSetupProperties, "pageSetUpPr"
}

attribute_bag! {
    /// `x:sheetView/pane` (`CT_Pane`, `sml.xsd:2359`) — where a sheet is frozen or split, and which
    /// of the resulting panes is active.
    ///
    /// `xSplit` and `ySplit` are `xsd:double` and mean two different things depending on
    /// [`state`](Self::state): for [`PaneState::Frozen`] they are a **number of columns and rows**,
    /// and for [`PaneState::Split`] they are a **twentieth of a point** of window space. Nothing
    /// here converts between them; both are the number the file wrote.
    #[xml(attribute(local = "xSplit", codec = Number<f64>, accessor = horizontal_split, default = 0.0))]
    #[xml(attribute(local = "ySplit", codec = Number<f64>, accessor = vertical_split, default = 0.0))]
    #[xml(attribute(local = "topLeftCell", codec = Enumeration<CellReference>, accessor = top_left_cell))]
    #[xml(attribute(local = "activePane", codec = Enumeration<Pane>, accessor = active_pane, default = Pane::TopLeft))]
    #[xml(attribute(local = "state", codec = Enumeration<PaneState>, accessor = state, default = PaneState::Split))]
    SheetPane, "pane"
}

attribute_bag! {
    /// `x:sheetView/selection` (`CT_Selection`, `sml.xsd:2388`) — what is selected in one pane.
    ///
    /// `@sqref` is a **list** of ranges, not one: a user who control-clicks three blocks leaves
    /// `sqref="A1:B2 D4 F6:F9"`, and [`CellRangeList`] is MJXOFF-93's parser for exactly that.
    /// `@activeCellId` indexes into it, so a model that collapsed the list would make the index
    /// meaningless.
    #[xml(attribute(local = "pane", codec = Enumeration<Pane>, accessor = pane, default = Pane::TopLeft))]
    #[xml(attribute(local = "activeCell", codec = Enumeration<CellReference>, accessor = active_cell))]
    #[xml(attribute(local = "activeCellId", codec = Number<u32>, accessor = active_range_index, default = 0))]
    #[xml(attribute(local = "sqref", codec = Enumeration<CellRangeList>, accessor = selected_ranges))]
    Selection, "selection"
}

attribute_bag! {
    /// `x:sheetView/pivotSelection` (`CT_PivotSelection`, `sml.xsd:2366`) — what is selected inside
    /// a pivot table shown on this sheet.
    ///
    /// Its one child, `pivotArea`, is **not** modelled: pivot tables are MJXOFF-133's (D18) to write
    /// down as deliberately unmodelled, and a typed `CT_PivotArea` here would be the first half of a
    /// model nothing finishes. It falls into the bag's `extra` and comes back verbatim, prefixes and
    /// all.
    ///
    /// `@r:id` is likewise preserved as the attribute the file wrote rather than reached through a
    /// typed accessor: resolving it needs a package, which this crate does not have.
    #[xml(attribute(local = "pane", codec = Enumeration<Pane>, accessor = pane, default = Pane::TopLeft))]
    #[xml(attribute(local = "showHeader", codec = OnOff, accessor = shows_header, default = false))]
    #[xml(attribute(local = "label", codec = OnOff, accessor = is_label_selection, default = false))]
    #[xml(attribute(local = "data", codec = OnOff, accessor = is_data_selection, default = false))]
    #[xml(attribute(local = "extendable", codec = OnOff, accessor = is_extendable, default = false))]
    #[xml(attribute(local = "count", codec = Number<u32>, accessor = selection_count, default = 0))]
    #[xml(attribute(local = "axis", codec = Enumeration<PivotTableAxis>, accessor = axis))]
    #[xml(attribute(local = "dimension", codec = Number<u32>, accessor = dimension, default = 0))]
    #[xml(attribute(local = "start", codec = Number<u32>, accessor = start, default = 0))]
    #[xml(attribute(local = "min", codec = Number<u32>, accessor = minimum, default = 0))]
    #[xml(attribute(local = "max", codec = Number<u32>, accessor = maximum, default = 0))]
    #[xml(attribute(local = "activeRow", codec = Number<u32>, accessor = active_row, default = 0))]
    #[xml(attribute(local = "activeCol", codec = Number<u32>, accessor = active_column, default = 0))]
    #[xml(attribute(local = "previousRow", codec = Number<u32>, accessor = previous_row, default = 0))]
    #[xml(attribute(local = "previousCol", codec = Number<u32>, accessor = previous_column, default = 0))]
    #[xml(attribute(local = "click", codec = Number<u32>, accessor = click_count, default = 0))]
    PivotSelection, "pivotSelection"
}

// -------------------------------------------------------------------------------------------
// `x:sheetPr`
// -------------------------------------------------------------------------------------------

/// `x:sheetPr` (`CT_SheetPr`, `sml.xsd:2306`) — the sheet tab's own properties: its colour, its
/// outline behaviour, its page-setup flags, and nine attributes.
///
/// **The nine attributes, counted from the schema:** `syncHorizontal`, `syncVertical`, `syncRef`,
/// `transitionEvaluation`, `transitionEntry`, `published`, `codeName`, `filterMode`,
/// `enableFormatConditionsCalculation`. `sample.xlsx` writes one of them (`filterMode="false"`) and
/// one child (`pageSetUpPr`).
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "syncHorizontal", codec = OnOff, accessor = synchronise_horizontal_scrolling, default = false))]
#[xml(attribute(local = "syncVertical", codec = OnOff, accessor = synchronise_vertical_scrolling, default = false))]
#[xml(attribute(local = "syncRef", codec = Enumeration<CellRange>, accessor = synchronisation_anchor))]
#[xml(attribute(local = "transitionEvaluation", codec = OnOff, accessor = lotus_formula_evaluation, default = false))]
#[xml(attribute(local = "transitionEntry", codec = OnOff, accessor = lotus_formula_entry, default = false))]
#[xml(attribute(local = "published", codec = OnOff, accessor = published, default = true))]
#[xml(attribute(local = "codeName", codec = Text, accessor = code_name))]
#[xml(attribute(local = "filterMode", codec = OnOff, accessor = filter_mode, default = false))]
#[xml(attribute(local = "enableFormatConditionsCalculation", codec = OnOff, accessor = recalculate_conditional_formats, default = true))]
pub struct SheetProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "tabColor", variant = TabColor, ty = ColorElement),
        child(local = "outlinePr", variant = Outline, ty = OutlineProperties),
        child(local = "pageSetUpPr", variant = PageSetup, ty = PageSetupProperties)
    )]
    content: Vec<SheetPropertiesContent>,
}

/// One child of [`SheetProperties`] — three modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetPropertiesContent {
    /// `x:tabColor` (rank 0) — one of the five slots `CT_Color` stands in.
    TabColor(ColorElement),
    /// `x:outlinePr` (rank 1).
    Outline(OutlineProperties),
    /// `x:pageSetUpPr` (rank 2).
    PageSetup(PageSetupProperties),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl SheetProperties {
    /// Builds an empty `x:sheetPr`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "sheetPr"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[SheetPropertiesContent] {
        &self.content
    }

    /// `x:tabColor` as the element the file wrote, or `None`.
    #[must_use]
    pub fn tab_color_element(&self) -> Option<&ColorElement> {
        self.content.iter().find_map(|item| match item {
            SheetPropertiesContent::TabColor(value) => Some(value),
            _ => None,
        })
    }

    /// The tab colour, decoded — `None` if the sheet writes no `tabColor` at all.
    ///
    /// A decoded snapshot, in the sense [`Color`]'s own documentation gives: it reports what the
    /// four mutually-exclusive spellings say, and it is **not** what is written back. The element
    /// itself is [`tab_color_element`](Self::tab_color_element) and is preserved whole.
    #[must_use]
    pub fn tab_colour(&self, interner: &Interner) -> Option<Color> {
        self.tab_color_element()
            .map(|element| element.color(interner))
    }

    /// `x:outlinePr` — where an outline's summary row and column sit. `None` if absent.
    #[must_use]
    pub fn outline(&self) -> Option<&OutlineProperties> {
        self.content.iter().find_map(|item| match item {
            SheetPropertiesContent::Outline(value) => Some(value),
            _ => None,
        })
    }

    /// `x:pageSetUpPr` — the automatic-page-break and fit-to-page flags. `None` if absent.
    #[must_use]
    pub fn page_setup(&self) -> Option<&PageSetupProperties> {
        self.content.iter().find_map(|item| match item {
            SheetPropertiesContent::PageSetup(value) => Some(value),
            _ => None,
        })
    }

    /// This element rebuilt as a [`RawElement`], without an interner. See
    /// the [`worksheet` module documentation](crate::worksheet) for why the worksheet spine needs one of these on every type it holds.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                SheetPropertiesContent::TabColor(value) => RawNode::Element(value.as_raw_element()),
                SheetPropertiesContent::Outline(value) => RawNode::Element(value.as_raw_element()),
                SheetPropertiesContent::PageSetup(value) => {
                    RawNode::Element(value.as_raw_element())
                }
                SheetPropertiesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for SheetProperties {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -------------------------------------------------------------------------------------------
// `x:sheetViews` and `x:sheetView`
// -------------------------------------------------------------------------------------------

/// `x:sheetViews` (`CT_SheetViews`, `sml.xsd:2326`) — one view per workbook window, in document
/// order.
///
/// The schema declares `sheetView` `minOccurs="1" maxOccurs="unbounded"`, so a worksheet that writes
/// this element at all writes at least one view; a workbook with two windows open writes two, one
/// per `workbookViewId`.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct SheetViews {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "sheetView", variant = View, ty = SheetView))]
    content: Vec<SheetViewsContent>,
}

/// One child of [`SheetViews`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetViewsContent {
    /// `x:sheetView`.
    View(SheetView),
    /// Anything else — `x:extLst` above all — preserved verbatim, in position.
    Raw(RawNode),
}

impl SheetViews {
    /// Builds an empty `x:sheetViews`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "sheetViews"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[SheetViewsContent] {
        &self.content
    }

    /// Every `x:sheetView`, in document order.
    pub fn views(&self) -> impl Iterator<Item = &SheetView> + '_ {
        self.content.iter().filter_map(|item| match item {
            SheetViewsContent::View(view) => Some(view),
            SheetViewsContent::Raw(_) => None,
        })
    }

    /// The `index`-th `x:sheetView`, mutably.
    pub fn view_mut(&mut self, index: usize) -> Option<&mut SheetView> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                SheetViewsContent::View(view) => Some(view),
                SheetViewsContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a view after the ones already present.
    pub fn push(&mut self, view: SheetView) {
        self.content.push(SheetViewsContent::View(view));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                SheetViewsContent::View(view) => RawNode::Element(view.as_raw_element()),
                SheetViewsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for SheetViews {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:sheetView` (`CT_SheetView`, `sml.xsd:2332`) — one window's view of the sheet: what is shown,
/// how far it is zoomed, where it is frozen, and what is selected in each pane.
///
/// **Nineteen attributes, counted from the schema**, of which `workbookViewId` is the only one
/// `use="required"`: `windowProtection`, `showFormulas`, `showGridLines`, `showRowColHeaders`,
/// `showZeros`, `rightToLeft`, `tabSelected`, `showRuler`, `showOutlineSymbols`, `defaultGridColor`,
/// `showWhiteSpace`, `view`, `topLeftCell`, `colorId`, `zoomScale`, `zoomScaleNormal`,
/// `zoomScaleSheetLayoutView`, `zoomScalePageLayoutView`, `workbookViewId`.
///
/// `tests/fixtures/sample.xlsx` writes **fifteen** of them — every one except `windowProtection`,
/// `showRuler`, `showWhiteSpace` and `zoomScaleSheetLayoutView` — plus one `<selection>`.
/// `crates/mjx-sml/tests/worksheet_spine.rs` asserts all fifteen come back, counted from the file.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "windowProtection", codec = OnOff, accessor = window_protection, default = false))]
#[xml(attribute(local = "showFormulas", codec = OnOff, accessor = shows_formulas, default = false))]
#[xml(attribute(local = "showGridLines", codec = OnOff, accessor = shows_grid_lines, default = true))]
#[xml(attribute(local = "showRowColHeaders", codec = OnOff, accessor = shows_row_and_column_headers, default = true))]
#[xml(attribute(local = "showZeros", codec = OnOff, accessor = shows_zero_values, default = true))]
#[xml(attribute(local = "rightToLeft", codec = OnOff, accessor = right_to_left, default = false))]
#[xml(attribute(local = "tabSelected", codec = OnOff, accessor = tab_selected, default = false))]
#[xml(attribute(local = "showRuler", codec = OnOff, accessor = shows_ruler, default = true))]
#[xml(attribute(local = "showOutlineSymbols", codec = OnOff, accessor = shows_outline_symbols, default = true))]
#[xml(attribute(local = "defaultGridColor", codec = OnOff, accessor = uses_default_grid_colour, default = true))]
#[xml(attribute(local = "showWhiteSpace", codec = OnOff, accessor = shows_page_margin_white_space, default = true))]
#[xml(attribute(local = "view", codec = Enumeration<SheetViewType>, accessor = view_type, default = SheetViewType::Normal))]
#[xml(attribute(local = "topLeftCell", codec = Enumeration<CellReference>, accessor = top_left_cell))]
#[xml(attribute(local = "colorId", codec = Number<u32>, accessor = grid_colour_index, default = 64))]
#[xml(attribute(local = "zoomScale", codec = Number<u32>, accessor = zoom_scale, default = 100))]
#[xml(attribute(local = "zoomScaleNormal", codec = Number<u32>, accessor = zoom_scale_normal_view, default = 0))]
#[xml(attribute(local = "zoomScaleSheetLayoutView", codec = Number<u32>, accessor = zoom_scale_page_break_view, default = 0))]
#[xml(attribute(local = "zoomScalePageLayoutView", codec = Number<u32>, accessor = zoom_scale_page_layout_view, default = 0))]
#[xml(attribute(local = "workbookViewId", codec = Number<u32>, accessor = workbook_view_index, required))]
pub struct SheetView {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pane", variant = Pane, ty = SheetPane),
        child(local = "selection", variant = Selection, ty = Selection),
        child(local = "pivotSelection", variant = PivotSelection, ty = PivotSelection)
    )]
    content: Vec<SheetViewContent>,
}

/// One child of [`SheetView`] — three modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetViewContent {
    /// `x:pane` (rank 0).
    Pane(SheetPane),
    /// `x:selection` (rank 1) — up to four, one per pane.
    Selection(Selection),
    /// `x:pivotSelection` (rank 2) — up to four.
    PivotSelection(PivotSelection),
    /// Anything else — `x:extLst` above all — preserved verbatim, in position.
    Raw(RawNode),
}

impl SheetView {
    /// Builds an empty `x:sheetView`, bound to `prefix` or to the default namespace.
    ///
    /// Carries no `workbookViewId`, which the schema declares required — set it before the view is
    /// written into a part, or the part will not validate. Nothing here invents one, because `0` is
    /// a real window index rather than a neutral value.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "sheetView"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every child, in document order, including the ones this type does not model.
    #[must_use]
    pub fn content(&self) -> &[SheetViewContent] {
        &self.content
    }

    /// `x:pane` — where the sheet is frozen or split. `None` for a view with neither.
    #[must_use]
    pub fn pane(&self) -> Option<&SheetPane> {
        self.content.iter().find_map(|item| match item {
            SheetViewContent::Pane(pane) => Some(pane),
            _ => None,
        })
    }

    /// Every `x:selection`, in document order — up to four, one per pane of a split.
    pub fn selections(&self) -> impl Iterator<Item = &Selection> + '_ {
        self.content.iter().filter_map(|item| match item {
            SheetViewContent::Selection(selection) => Some(selection),
            _ => None,
        })
    }

    /// Every `x:pivotSelection`, in document order — up to four.
    pub fn pivot_selections(&self) -> impl Iterator<Item = &PivotSelection> + '_ {
        self.content.iter().filter_map(|item| match item {
            SheetViewContent::PivotSelection(selection) => Some(selection),
            _ => None,
        })
    }

    /// How many attributes this view carries, including any the schema does not declare.
    ///
    /// Counted off the element rather than off a list of the ones this type models, so a producer's
    /// extension is included — which is what makes it usable as a fidelity assertion.
    #[must_use]
    pub fn attribute_count(&self) -> usize {
        self.attributes.len()
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                SheetViewContent::Pane(value) => RawNode::Element(value.as_raw_element()),
                SheetViewContent::Selection(value) => RawNode::Element(value.as_raw_element()),
                SheetViewContent::PivotSelection(value) => RawNode::Element(value.as_raw_element()),
                SheetViewContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for SheetView {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
