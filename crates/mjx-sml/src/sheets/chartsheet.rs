//! `xl/chartsheets/sheetN.xml` — `CT_Chartsheet`, a whole sheet tab occupied by one chart.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_Chartsheet` | 2955 | `x:chartsheet` — the part's root |
//! | `CT_ChartsheetPr` | 2974 | `x:sheetPr` (rank 0) |
//! | `CT_ChartsheetViews` | 2981 | `x:sheetViews` (rank 1) |
//! | `CT_ChartsheetView` | 2987 | `x:sheetViews/sheetView` |
//! | `CT_ChartsheetProtection` | 2996 | `x:sheetProtection` (rank 2) |
//! | `CT_CustomChartsheetViews` | 3020 | `x:customSheetViews` (rank 3) |
//! | `CT_CustomChartsheetView` | 3026 | `x:customSheetViews/customSheetView` |
//! | `CT_Drawing` | 2504 | `x:drawing` (rank 7) — **`minOccurs="1"`** |
//!
//! # A chartsheet has no cells, and that is a property of the type
//!
//! `CT_Chartsheet`'s fourteen-member `xsd:sequence` has **no `sheetData`**, no `dimension`, no
//! `cols`, no `mergeCells`, no `conditionalFormatting` and no `dataValidations`. There is nothing to
//! address with a [`CellReference`](crate::CellReference) because there is no grid.
//!
//! So [`ChartSheetPart`] has **no cell accessor at all** — not one that answers `None`, not one that
//! answers an empty collection. Asking a chartsheet for its cells is a compile error, which is the
//! ticket's own constraint: *"A chartsheet's absence of cells is part of its type, not an
//! empty-collection special case."* The type-level consequence is that a caller holding
//! `mjx_xlsx::SheetMarkup` must match out the `Worksheet` variant before it can reach a cell, and
//! the match is the place the distinction is made — once, at the door, rather than at every
//! accessor.
//!
//! # Fourteen slots, ten modelled, four held
//!
//! `pageMargins`, `pageSetup`, `headerFooter` and `picture` come straight from
//! [`crate::features::print`] and `webPublishItems` from [`crate::features::publishing`], so six of
//! the ten are shared markup this file does not re-model. The four held verbatim are
//! `legacyDrawing`, `legacyDrawingHF`, `drawingHF` and `extLst`.
//!
//! Neither figure is written down anywhere it can rot: `sheets/frame.rs`'s
//! `every_slot_of_every_sheet_kind_is_accounted_for` reads a chartsheet holding one of every slot,
//! asks the reader which it typed, and holds this heading to the answer.
//!
//! # The six shared-markup slots, and why they are not re-modelled here
//!
//! `pageMargins`, `pageSetup`, `headerFooter` and `picture` come straight from
//! [`crate::features::print`]; `webPublishItems` from [`crate::features::publishing`]. The one
//! per-kind difference is `pageSetup`: a chartsheet's is
//! [`ChartSheetPageSetup`] (`CT_CsPageSetup`), not
//! [`PageSetup`](crate::features::print::PageSetup), because the six grid-only attributes do not apply. See
//! [`crate::features::print`] for why that is two types rather than one.
//!
//! # `sheetPr@tabColor` is a SpreadsheetML colour
//!
//! [`ColorElement`], not `mjx_dml::ColorSpec`. `sml.xsd`'s own `CT_Color`
//! carries `@indexed` (a slot in the workbook's palette), `@theme` (a slot in the theme) and
//! `@tint`, and none of those is a DrawingML concept: a colour that says "palette entry 13" cannot
//! be flattened to an RGB triple without reading `xl/styles.xml`, and flattening it would be
//! resolving a reference this crate deliberately reports instead. The same type
//! [`SheetProperties::tab_color_element`](crate::SheetProperties::tab_color_element) answers with
//! for a worksheet.
//!
//! # The drawing is a relationship, and the chart in it belongs to another child
//!
//! `x:drawing` is `CT_Drawing`, one required `r:id`. [`SheetDrawing`] holds that identifier as the
//! string the file wrote, and this crate never resolves it: the part it names —
//! `xl/drawings/drawingN.xml`, rooted in SpreadsheetDrawingML — is **MJXOFF-107's (E3)**, and the
//! chart definition inside it is **MJXOFF-111's (E4)**. Reporting which relationship a chartsheet's
//! chart hangs off is this child's; opening it is not.
//!
//! `legacyDrawing`, `legacyDrawingHF` and `drawingHF` are held verbatim for the same reason they are
//! on a worksheet: they are the header/footer and VML halves of the drawing family, and splitting
//! that family across children would put two work items in one file.

use mjx_ooxml_core::{
    Enumeration, FromXml, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::{CHARTSHEET, CHARTSHEET_VIEWS, CUSTOM_CHARTSHEET_VIEW};
use mjx_ooxml_types::spreadsheetml::SheetState;
use mjx_ooxml_types::support::OnOff;

use crate::error::SmlError;
use crate::features::print::{
    ChartSheetPageSetup, HeaderFooter, PageMargins, SheetBackgroundPicture,
};
use crate::features::publishing::WebPublishItems;
use crate::font::ColorElement;
use crate::leaf::{attribute_bag, bag_without_declared_attributes, relationship_reference};
use crate::sheets::frame::{sheet_part_surface, sheet_slot, SheetContent, SheetFrame};
use crate::worksheet::rebuild_element;

// -----------------------------------------------------------------------------------------------
// The leaves
// -----------------------------------------------------------------------------------------------

bag_without_declared_attributes! {
    /// `x:drawing` (`CT_Drawing`, `sml.xsd:2504`) — the relationship to a sheet's drawings part.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_Drawing`. Wire element: `drawing`, rank **7** of `CT_Chartsheet`
    /// (`minOccurs="1"`, the only child of that type that is required beside `sheetViews`), rank
    /// **9** of `CT_Dialogsheet` and rank **29** of `CT_Worksheet`.
    ///
    /// One attribute, `xsd:attribute ref="r:id" use="required"`. **The part it names is not this
    /// crate's and is not this child's:** `xl/drawings/drawingN.xml` is rooted in
    /// SpreadsheetDrawingML, whose model is MJXOFF-107's (E3) and whose chart content is
    /// MJXOFF-111's (E4). This type is the *element* — modelled here because a chartsheet cannot be
    /// read without it, and reusable unchanged when E3 fills `CT_Worksheet`'s rank 29.
    SheetDrawing, "drawing"
}

relationship_reference!(SheetDrawing);

attribute_bag! {
    /// `x:sheetProtection` **on a chartsheet** (`CT_ChartsheetProtection`, `sml.xsd:2996`).
    ///
    /// **`ST_`/`CT_` symbol:** `CT_ChartsheetProtection`. Wire element: `sheetProtection`, rank
    /// **2** of `CT_Chartsheet`.
    ///
    /// Two protection flags rather than a worksheet's sixteen, because there are only two things to
    /// lock on a sheet with no cells: `@content` (the chart itself) and `@objects` (the shapes
    /// drawn over it).
    ///
    /// **This is not security, and neither is [`SheetProtection`](crate::SheetProtection).** The
    /// legacy `@password` is a sixteen-bit hash of a hash; the modern trio
    /// (`@algorithmName`/`@hashValue`/`@saltValue`/`@spinCount`) is stronger but still guards a flag
    /// a consumer is free to ignore, not an encrypted payload. Every one of them is carried through
    /// exactly as written and **nothing here verifies, computes or clears one**: a library that
    /// silently re-hashed a password would be changing a file's protection state.
    #[xml(attribute(local = "password", codec = Text, accessor = legacy_password_hash))]
    #[xml(attribute(local = "algorithmName", codec = Text, accessor = hash_algorithm_name))]
    #[xml(attribute(local = "hashValue", codec = Text, accessor = password_hash))]
    #[xml(attribute(local = "saltValue", codec = Text, accessor = password_salt))]
    #[xml(attribute(local = "spinCount", codec = Number<u32>, accessor = hash_iteration_count))]
    #[xml(attribute(local = "content", codec = OnOff, accessor = protects_content, default = false))]
    #[xml(attribute(local = "objects", codec = OnOff, accessor = protects_objects, default = false))]
    ChartSheetProtection, "sheetProtection"
}

attribute_bag! {
    /// `x:sheetView` **on a chartsheet** (`CT_ChartsheetView`, `sml.xsd:2987`) — one window's view
    /// of the chart.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_ChartsheetView`. Wire element: `sheetView`.
    ///
    /// Four attributes against a worksheet [`SheetView`](crate::SheetView)'s twenty-one: there is
    /// no grid to show gridlines for, no formula bar state, no frozen pane and no top-left cell.
    /// `@workbookViewId` is `use="required"` and indexes the workbook's own `x:bookViews`.
    ///
    /// `@zoomToFit` is the chartsheet's own: the chart is scaled to the window rather than to
    /// `@zoomScale`. Both are reported; nothing here decides which a consumer honours.
    ///
    /// The only child the schema allows is `extLst`, which is why this is an attribute bag rather
    /// than a container: an `extLst` is held in [`extra`](Self::extra) like every other unmodelled
    /// child.
    #[xml(attribute(local = "tabSelected", codec = OnOff, accessor = tab_selected, default = false))]
    #[xml(attribute(local = "zoomScale", codec = Number<u32>, accessor = zoom_scale, default = 100))]
    #[xml(attribute(local = "workbookViewId", codec = Number<u32>, accessor = workbook_view_index, required))]
    #[xml(attribute(local = "zoomToFit", codec = OnOff, accessor = zooms_to_fit_window, default = false))]
    ChartSheetView, "sheetView"
}

/// `x:sheetViews` **on a chartsheet** (`CT_ChartsheetViews`, `sml.xsd:2981`) — every window's view
/// of this chart, in document order.
///
/// **`ST_`/`CT_` symbol:** `CT_ChartsheetViews`. Wire element: `sheetViews`, rank **1** of
/// `CT_Chartsheet` and `minOccurs="1"` — the one slot a chartsheet may not omit besides `drawing`.
///
/// The schema declares `sheetView` `minOccurs="1"`, so an element with no view is invalid markup.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct ChartSheetViews {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "sheetView", variant = View, ty = ChartSheetView))]
    content: Vec<ChartSheetViewsContent>,
}

/// One child of [`ChartSheetViews`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartSheetViewsContent {
    /// `x:sheetView` (rank 0) — one window's view.
    View(ChartSheetView),
    /// Anything else — `x:extLst` (rank 1) above all — preserved verbatim, in position.
    Raw(RawNode),
}

impl ChartSheetViews {
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

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ChartSheetViewsContent] {
        &self.content
    }

    /// Every `x:sheetView`, in document order.
    pub fn views(&self) -> impl Iterator<Item = &ChartSheetView> + '_ {
        self.content.iter().filter_map(|child| match child {
            ChartSheetViewsContent::View(view) => Some(view),
            ChartSheetViewsContent::Raw(_) => None,
        })
    }

    /// The `index`-th `x:sheetView`, mutably.
    pub fn view_mut(&mut self, index: usize) -> Option<&mut ChartSheetView> {
        self.content
            .iter_mut()
            .filter_map(|child| match child {
                ChartSheetViewsContent::View(view) => Some(view),
                ChartSheetViewsContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a view at its rank in `CT_ChartsheetViews`'s sequence — before an `x:extLst` if the
    /// element already carries one.
    pub fn push(&mut self, view: ChartSheetView) {
        let at = CHARTSHEET_VIEWS.insert_index_of_names(
            self.content.iter().map(|child| match child {
                ChartSheetViewsContent::View(_) => CHARTSHEET_VIEWS.rank_of(None, "sheetView"),
                ChartSheetViewsContent::Raw(_) => None,
            }),
            "sheetView",
        );
        self.content.insert(at, ChartSheetViewsContent::View(view));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|child| match child {
                ChartSheetViewsContent::View(view) => RawNode::Element(view.as_raw_element()),
                ChartSheetViewsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for ChartSheetViews {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:sheetPr` **on a chartsheet** (`CT_ChartsheetPr`, `sml.xsd:2974`) — the tab's colour, its
/// publish flag and its VBA code name.
///
/// **`ST_`/`CT_` symbol:** `CT_ChartsheetPr`. Wire element: `sheetPr`, rank **0** of
/// `CT_Chartsheet`.
///
/// Three of a worksheet [`SheetProperties`](crate::SheetProperties)' nine attributes and one of its
/// three children, because the six that are missing describe a grid: outline direction, the sync
/// range, filter mode, the `pageSetUpPr` fit flag.
///
/// `@published` says the sheet may be published to a server, and defaults to `true`; `@codeName` is
/// the identifier a VBA project refers to the sheet by, which is **not** the tab name and is not
/// kept in step with it by anything here.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "published", codec = OnOff, accessor = published, default = true))]
#[xml(attribute(local = "codeName", codec = Text, accessor = code_name))]
pub struct ChartSheetProperties {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "tabColor", variant = TabColor, ty = ColorElement))]
    content: Vec<ChartSheetPropertiesContent>,
}

/// One child of [`ChartSheetProperties`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartSheetPropertiesContent {
    /// `x:tabColor` (rank 0) — SpreadsheetML's own `CT_Color`, which can name a palette index or a
    /// theme slot as well as an RGB value.
    TabColor(ColorElement),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl ChartSheetProperties {
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

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[ChartSheetPropertiesContent] {
        &self.content
    }

    /// `x:tabColor` — the colour of this sheet's tab, unresolved.
    ///
    /// A SpreadsheetML colour, so it may be an `@rgb`, an `@indexed` palette slot, a `@theme` slot
    /// with a `@tint`, or `@auto`. Resolving one against `xl/styles.xml` is
    /// the caller's job, through [`ColorElement::color`](crate::ColorElement::color) and the
    /// workbook's palette; nothing here flattens it.
    #[must_use]
    pub fn tab_color_element(&self) -> Option<&ColorElement> {
        self.content.iter().find_map(|child| match child {
            ChartSheetPropertiesContent::TabColor(colour) => Some(colour),
            ChartSheetPropertiesContent::Raw(_) => None,
        })
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|child| match child {
                ChartSheetPropertiesContent::TabColor(colour) => {
                    RawNode::Element(colour.as_raw_element())
                }
                ChartSheetPropertiesContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for ChartSheetProperties {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// A chartsheet's saved views
// -----------------------------------------------------------------------------------------------

/// `x:customSheetView` **on a chartsheet** (`CT_CustomChartsheetView`, `sml.xsd:3026`) — one saved
/// view of one chartsheet.
///
/// **`ST_`/`CT_` symbol:** `CT_CustomChartsheetView`. Wire element: `customSheetView`.
///
/// Three children and four attributes, against
/// [`CustomSheetView`](crate::features::custom_views::CustomSheetView)'s nine and twenty. Everything a worksheet's saved
/// view remembers about a grid — the pane, the selection, the breaks, the filter, the hidden rows
/// and columns — has no meaning here, so the schema declares a second type rather than reusing the
/// first, and so does this crate.
///
/// Its `pageSetup` is [`ChartSheetPageSetup`] (`CT_CsPageSetup`), which
/// is the one place the two saved-view types differ in a way a caller can trip on.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "guid", codec = Text, accessor = guid, required))]
#[xml(attribute(local = "scale", codec = Number<u32>, accessor = zoom_scale, default = 100))]
#[xml(attribute(local = "state", codec = Enumeration<SheetState>, accessor = sheet_state, default = SheetState::Visible))]
#[xml(attribute(local = "zoomToFit", codec = OnOff, accessor = zooms_to_fit_window, default = false))]
pub struct CustomChartSheetView {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pageMargins", variant = PageMargins, ty = PageMargins),
        child(local = "pageSetup", variant = PageSetup, ty = ChartSheetPageSetup),
        child(local = "headerFooter", variant = HeaderFooter, ty = HeaderFooter)
    )]
    content: Vec<CustomChartSheetViewContent>,
}

/// One child of [`CustomChartSheetView`] — three modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomChartSheetViewContent {
    /// `x:pageMargins` (rank 0).
    PageMargins(PageMargins),
    /// `x:pageSetup` (rank 1) — `CT_CsPageSetup`, not `CT_PageSetup`.
    PageSetup(ChartSheetPageSetup),
    /// `x:headerFooter` (rank 2).
    HeaderFooter(HeaderFooter),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CustomChartSheetViewContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    #[must_use]
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::PageMargins(_) => "pageMargins",
            Self::PageSetup(_) => "pageSetup",
            Self::HeaderFooter(_) => "headerFooter",
            Self::Raw(_) => return None,
        })
    }

    /// This child rebuilt as a node.
    #[must_use]
    fn as_raw_node(&self) -> RawNode {
        match self {
            Self::PageMargins(value) => RawNode::Element(value.as_raw_element()),
            Self::PageSetup(value) => RawNode::Element(value.as_raw_element()),
            Self::HeaderFooter(value) => RawNode::Element(value.as_raw_element()),
            Self::Raw(node) => node.clone(),
        }
    }
}

impl CustomChartSheetView {
    /// Builds an empty `x:customSheetView`, bound to `prefix` or to the default namespace.
    ///
    /// Carries no `@guid`, which the schema declares required — see
    /// [`CustomSheetView::new`](crate::CustomSheetView::new) for why nothing invents one.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "customSheetView"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CustomChartSheetViewContent] {
        &self.content
    }

    /// `x:pageMargins` (rank 0) — this view's own margins.
    #[must_use]
    pub fn page_margins(&self) -> Option<&PageMargins> {
        self.content.iter().find_map(|child| match child {
            CustomChartSheetViewContent::PageMargins(value) => Some(value),
            _ => None,
        })
    }

    /// `x:pageSetup` (rank 1) — this view's own chartsheet page setup.
    #[must_use]
    pub fn page_setup(&self) -> Option<&ChartSheetPageSetup> {
        self.content.iter().find_map(|child| match child {
            CustomChartSheetViewContent::PageSetup(value) => Some(value),
            _ => None,
        })
    }

    /// `x:headerFooter` (rank 2) — this view's own six opaque strings.
    #[must_use]
    pub fn header_footer(&self) -> Option<&HeaderFooter> {
        self.content.iter().find_map(|child| match child {
            CustomChartSheetViewContent::HeaderFooter(value) => Some(value),
            _ => None,
        })
    }

    /// Sets one child: `None` removes it, `Some(value)` replaces the existing element **where it
    /// is** or inserts a new one at its rank in `CT_CustomChartsheetView`'s `xsd:sequence`.
    pub fn set_child(&mut self, value: CustomChartSheetViewContent) {
        let Some(local) = value.local() else {
            return;
        };
        let existing = self
            .content
            .iter()
            .position(|child| child.local() == Some(local));
        match existing {
            Some(at) => self.content[at] = value,
            None => {
                let at = CUSTOM_CHARTSHEET_VIEW.insert_index_of_names(
                    self.content.iter().map(|child| {
                        child
                            .local()
                            .and_then(|local| CUSTOM_CHARTSHEET_VIEW.rank_of(None, local))
                    }),
                    local,
                );
                self.content.insert(at, value);
                self.empty = false;
            }
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(CustomChartSheetViewContent::as_raw_node)
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CustomChartSheetView {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:customSheetViews` **on a chartsheet** (`CT_CustomChartsheetViews`, `sml.xsd:3020`) — every
/// saved view of this chartsheet, in document order.
///
/// **`ST_`/`CT_` symbol:** `CT_CustomChartsheetViews`. Wire element: `customSheetViews`, rank **3**
/// of `CT_Chartsheet`.
///
/// Unlike [`CustomSheetViews`](crate::CustomSheetViews), the schema declares `customSheetView`
/// `minOccurs="0"` here — so an empty `<customSheetViews/>` on a chartsheet is **valid markup** and
/// is preserved rather than removed.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct CustomChartSheetViews {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "customSheetView", variant = View, ty = CustomChartSheetView))]
    content: Vec<CustomChartSheetViewsContent>,
}

/// One child of [`CustomChartSheetViews`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomChartSheetViewsContent {
    /// `x:customSheetView` — one saved view.
    View(CustomChartSheetView),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CustomChartSheetViews {
    /// Builds an empty `x:customSheetViews`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "customSheetViews"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// The element's own qualified name, as the file wrote it.
    #[must_use]
    pub fn element_name(&self) -> RawName {
        self.name
    }

    /// Every child, in document order, including anything this type does not model.
    #[must_use]
    pub fn content(&self) -> &[CustomChartSheetViewsContent] {
        &self.content
    }

    /// Every `x:customSheetView`, in document order.
    pub fn views(&self) -> impl Iterator<Item = &CustomChartSheetView> + '_ {
        self.content.iter().filter_map(|child| match child {
            CustomChartSheetViewsContent::View(view) => Some(view),
            CustomChartSheetViewsContent::Raw(_) => None,
        })
    }

    /// The `index`-th `x:customSheetView`, mutably.
    pub fn view_mut(&mut self, index: usize) -> Option<&mut CustomChartSheetView> {
        self.content
            .iter_mut()
            .filter_map(|child| match child {
                CustomChartSheetViewsContent::View(view) => Some(view),
                CustomChartSheetViewsContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a view after the ones already present.
    pub fn push(&mut self, view: CustomChartSheetView) {
        self.content.push(CustomChartSheetViewsContent::View(view));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|child| match child {
                CustomChartSheetViewsContent::View(view) => RawNode::Element(view.as_raw_element()),
                CustomChartSheetViewsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CustomChartSheetViews {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// The part
// -----------------------------------------------------------------------------------------------

/// One child of [`ChartSheetPart`]: nine modelled slots, and everything else.
#[derive(Debug)]
pub enum ChartSheetContent {
    /// `x:sheetPr` (rank 0).
    Properties(ChartSheetProperties),
    /// `x:sheetViews` (rank 1) — **`minOccurs="1"`**.
    SheetViews(ChartSheetViews),
    /// `x:sheetProtection` (rank 2) — two advisory flags and a preserved hash, never security.
    Protection(ChartSheetProtection),
    /// `x:customSheetViews` (rank 3) — `CT_CustomChartsheetViews`, not the worksheet type.
    CustomSheetViews(CustomChartSheetViews),
    /// `x:pageMargins` (rank 4).
    PageMargins(PageMargins),
    /// `x:pageSetup` (rank 5) — `CT_CsPageSetup`, not `CT_PageSetup`.
    PageSetup(ChartSheetPageSetup),
    /// `x:headerFooter` (rank 6).
    HeaderFooter(HeaderFooter),
    /// `x:drawing` (rank 7) — **`minOccurs="1"`**: the relationship the chart hangs off. The part it
    /// names is MJXOFF-107's (E3) and the chart in it MJXOFF-111's (E4).
    Drawing(SheetDrawing),
    /// `x:picture` (rank 11) — the image drawn behind the chart.
    BackgroundPicture(SheetBackgroundPicture),
    /// `x:webPublishItems` (rank 12) — the fragments of this sheet published as HTML.
    WebPublishItems(WebPublishItems),
    /// Everything this type does not model: `legacyDrawing` (rank 8), `legacyDrawingHF` (9),
    /// `drawingHF` (10), `extLst` (13), any foreign element, any `mc:AlternateContent`, and the
    /// text, comments and processing instructions between siblings.
    Raw(RawNode),
}

impl SheetContent for ChartSheetContent {
    const ORDER: &'static mjx_ooxml_types::child_order::ChildOrder = CHARTSHEET;

    fn read(element: &RawElement, interner: &Interner) -> Result<Option<Self>, SmlError> {
        Ok(Some(match interner.resolve(element.name.local) {
            "sheetPr" => Self::Properties(ChartSheetProperties::from_xml(element, interner)?),
            "sheetViews" => Self::SheetViews(ChartSheetViews::from_xml(element, interner)?),
            "sheetProtection" => {
                Self::Protection(ChartSheetProtection::from_xml(element, interner)?)
            }
            "customSheetViews" => {
                Self::CustomSheetViews(CustomChartSheetViews::from_xml(element, interner)?)
            }
            "pageMargins" => Self::PageMargins(PageMargins::from_xml(element, interner)?),
            "pageSetup" => Self::PageSetup(ChartSheetPageSetup::from_xml(element, interner)?),
            "headerFooter" => Self::HeaderFooter(HeaderFooter::from_xml(element, interner)?),
            "drawing" => Self::Drawing(SheetDrawing::from_xml(element, interner)?),
            "picture" => {
                Self::BackgroundPicture(SheetBackgroundPicture::from_xml(element, interner)?)
            }
            "webPublishItems" => {
                Self::WebPublishItems(WebPublishItems::from_xml(element, interner)?)
            }
            _ => return Ok(None),
        }))
    }

    fn raw(node: RawNode) -> Self {
        Self::Raw(node)
    }

    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Properties(_) => "sheetPr",
            Self::SheetViews(_) => "sheetViews",
            Self::Protection(_) => "sheetProtection",
            Self::CustomSheetViews(_) => "customSheetViews",
            Self::PageMargins(_) => "pageMargins",
            Self::PageSetup(_) => "pageSetup",
            Self::HeaderFooter(_) => "headerFooter",
            Self::Drawing(_) => "drawing",
            Self::BackgroundPicture(_) => "picture",
            Self::WebPublishItems(_) => "webPublishItems",
            Self::Raw(_) => return None,
        })
    }

    fn raw_node(&self) -> Option<&RawNode> {
        match self {
            Self::Raw(node) => Some(node),
            _ => None,
        }
    }

    fn as_raw_element(&self) -> Option<RawElement> {
        Some(match self {
            Self::Properties(value) => value.as_raw_element(),
            Self::SheetViews(value) => value.as_raw_element(),
            Self::Protection(value) => value.as_raw_element(),
            Self::CustomSheetViews(value) => value.as_raw_element(),
            Self::PageMargins(value) => value.as_raw_element(),
            Self::PageSetup(value) => value.as_raw_element(),
            Self::HeaderFooter(value) => value.as_raw_element(),
            Self::Drawing(value) => value.as_raw_element(),
            Self::BackgroundPicture(value) => value.as_raw_element(),
            Self::WebPublishItems(value) => value.as_raw_element(),
            Self::Raw(_) => return None,
        })
    }
}

/// `x:chartsheet` (`CT_Chartsheet`, `sml.xsd:2955`) — the whole chartsheet part.
///
/// See the [module documentation](crate::sheets::chartsheet) for the fourteen slots, for why a
/// chartsheet has no cell accessor at all, and for which of its slots are shared with the other
/// sheet kinds.
///
/// The part-level and slot-level copy-on-write are
/// [`SheetFrame`](crate::sheets)'s and are identical to
/// [`WorksheetPart`](crate::WorksheetPart)'s: a part nobody edited is one `memcpy`, and after an
/// edit every *other* slot still writes from its own bytes.
#[derive(Debug)]
pub struct ChartSheetPart {
    frame: SheetFrame<ChartSheetContent>,
}

sheet_part_surface!(
    ChartSheetPart,
    ChartSheetContent,
    "chartsheet",
    "chartsheet"
);

impl ChartSheetPart {
    sheet_slot!(
        ChartSheetContent,
        properties,
        properties_mut,
        set_properties,
        Properties,
        ChartSheetProperties,
        "sheetPr",
        "`x:sheetPr` (rank 0) — the tab's colour, its publish flag and its VBA code name."
    );
    sheet_slot!(
        ChartSheetContent,
        sheet_views,
        sheet_views_mut,
        set_sheet_views,
        SheetViews,
        ChartSheetViews,
        "sheetViews",
        "`x:sheetViews` (rank 1) — every window's view of this chart. The schema declares the slot \
         `minOccurs=\"1\"`, so a chartsheet that answers `None` here is one that will not validate."
    );
    sheet_slot!(
        ChartSheetContent,
        protection,
        protection_mut,
        set_protection,
        Protection,
        ChartSheetProtection,
        "sheetProtection",
        "`x:sheetProtection` (rank 2) — two advisory flags and a preserved hash. **Never security:** \
         nothing here verifies, computes or clears a password."
    );
    sheet_slot!(
        ChartSheetContent,
        custom_sheet_views,
        custom_sheet_views_mut,
        set_custom_sheet_views,
        CustomSheetViews,
        CustomChartSheetViews,
        "customSheetViews",
        "`x:customSheetViews` (rank 3) — `CT_CustomChartsheetViews`, whose entries carry only \
         margins, a chartsheet page setup and a header/footer."
    );
    sheet_slot!(
        ChartSheetContent,
        page_margins,
        page_margins_mut,
        set_page_margins,
        PageMargins,
        PageMargins,
        "pageMargins",
        "`x:pageMargins` (rank 4) — the six margins of a printed page, in inches."
    );
    sheet_slot!(
        ChartSheetContent,
        page_setup,
        page_setup_mut,
        set_page_setup,
        PageSetup,
        ChartSheetPageSetup,
        "pageSetup",
        "`x:pageSetup` (rank 5) — **`CT_CsPageSetup`**, which is `CT_PageSetup` without the six \
         attributes that only mean something over a grid."
    );
    sheet_slot!(
        ChartSheetContent,
        header_footer,
        header_footer_mut,
        set_header_footer,
        HeaderFooter,
        HeaderFooter,
        "headerFooter",
        "`x:headerFooter` (rank 6) — the six opaque code strings, never re-serialised."
    );
    sheet_slot!(
        ChartSheetContent,
        drawing,
        drawing_mut,
        set_drawing,
        Drawing,
        SheetDrawing,
        "drawing",
        "`x:drawing` (rank 7) — the relationship the chart hangs off. `minOccurs=\"1\"`, so a \
         chartsheet that answers `None` here will not validate. **Resolving the `r:id` is \
         `mjx-xlsx`'s and opening the part it names is MJXOFF-107's (E3).**"
    );
    sheet_slot!(
        ChartSheetContent,
        background_picture,
        background_picture_mut,
        set_background_picture,
        BackgroundPicture,
        SheetBackgroundPicture,
        "picture",
        "`x:picture` (rank 11) — the image drawn behind the chart, named by an `r:id`."
    );
    sheet_slot!(
        ChartSheetContent,
        web_publish_items,
        web_publish_items_mut,
        set_web_publish_items,
        WebPublishItems,
        WebPublishItems,
        "webPublishItems",
        "`x:webPublishItems` (rank 12) — the fragments of this sheet published as HTML, and the \
         untrusted destination path each names. Never resolved, never opened."
    );
}
