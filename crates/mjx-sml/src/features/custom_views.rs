//! Custom sheet views: `x:customSheetViews` and the per-user view saved in each one.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_CustomSheetViews` | 2531 | `x:customSheetViews` (rank 13 of `CT_Worksheet`) |
//! | `CT_CustomSheetView` | 2537 | `x:customSheetViews/customSheetView` |
//!
//! # What a custom view is, and what it is not
//!
//! Excel's **Custom Views** feature saves the state of a sheet under a name: which rows and columns
//! are hidden, where the panes are split, what is selected, the print setup and the filter. Each
//! saved view writes one `x:customSheetView`, keyed by a `@guid` that matches an entry in the
//! workbook's own `x:customWorkbookViews`
//! ([`CustomWorkbookView`](crate::CustomWorkbookView)) — one workbook view spans the sheets, one
//! sheet view per sheet under it.
//!
//! **A custom view is a record, never an instruction.** Reading one sets no row's `@hidden`, moves
//! no pane, applies no filter and changes no print setup, exactly as
//! [`crate::features::filters`] states for an autofilter. `@hiddenRows` on a view is that view's
//! *memory* of what was hidden when it was saved; the sheet's actual hidden rows are
//! [`RowHeight`](crate::RowHeight)'s and are untouched by anything here.
//!
//! # This type is where four of this crate's clusters meet
//!
//! `CT_CustomSheetView`'s `xsd:sequence` is nine elements borrowed from elsewhere plus `extLst`:
//!
//! | rank | element | type | modelled by |
//! |---|---|---|---|
//! | 0 | `pane` | `CT_Pane` | [`SheetPane`](crate::SheetPane) — MJXOFF-102 (D07) |
//! | 1 | `selection` | `CT_Selection` | [`Selection`](crate::Selection) — MJXOFF-102 (D07) |
//! | 2 | `rowBreaks` | `CT_PageBreak` | [`PageBreaks`](crate::PageBreaks) — MJXOFF-117 (D12) |
//! | 3 | `colBreaks` | `CT_PageBreak` | [`PageBreaks`](crate::PageBreaks), the other axis |
//! | 4 | `pageMargins` | `CT_PageMargins` | [`PageMargins`](super::print::PageMargins) |
//! | 5 | `printOptions` | `CT_PrintOptions` | [`PrintOptions`](super::print::PrintOptions) |
//! | 6 | `pageSetup` | `CT_PageSetup` | [`PageSetup`](super::print::PageSetup) |
//! | 7 | `headerFooter` | `CT_HeaderFooter` | [`HeaderFooter`](super::print::HeaderFooter) |
//! | 8 | `autoFilter` | `CT_AutoFilter` | [`AutoFilter`](crate::AutoFilter) — MJXOFF-123 (D14) |
//! | 9 | `extLst` | — | the unknown bucket |
//!
//! **Not one of them is re-modelled here.** That is the whole reason this slot was left raw by
//! MJXOFF-127 (D16) and handed to this child: four of the nine types did not exist yet, so modelling
//! `customSheetView` then would have meant either a second copy of the print block or a type that
//! held half its own children as `RawNode`.
//!
//! # A view's `pageSetup` is `CT_PageSetup`, not `CT_CsPageSetup`
//!
//! Even on a chartsheet. `CT_CustomSheetView` is the *worksheet* shape and always carries the full
//! `CT_PageSetup`; a chartsheet's saved views are a different type entirely,
//! [`CustomChartSheetView`](crate::sheets::CustomChartSheetView) (`CT_CustomChartsheetView`), whose
//! three children are `pageMargins`, `CT_CsPageSetup` and `headerFooter`.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::CUSTOM_SHEET_VIEW;
use mjx_ooxml_types::spreadsheetml::{SheetState, SheetViewType};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellReference;
use crate::features::filters::AutoFilter;
use crate::features::print::{HeaderFooter, PageMargins, PageSetup, PrintOptions};
use crate::worksheet::{rebuild_element, PageBreaks, Selection, SheetPane};

/// `x:customSheetView` (`CT_CustomSheetView`, `sml.xsd:2537`) — one saved view of one sheet.
///
/// **`ST_`/`CT_` symbol:** `CT_CustomSheetView`. Wire element: `customSheetView`.
///
/// Twenty attributes, of which exactly one — `@guid` — is `use="required"`. It is the key that ties
/// this view to the workbook-level [`CustomWorkbookView`](crate::CustomWorkbookView) of the same
/// `@guid`, and it is carried as the text the file wrote: `ST_Guid` is a pattern-restricted
/// `xsd:token` this project stores verbatim rather than reformatting.
///
/// `@colorId` is an index into the workbook's indexed palette
/// ([`IndexedColorPalette`](crate::IndexedColorPalette)); `64` is the schema's default and means
/// "the system foreground", the same convention `x:sheetView/@colorId` uses.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "guid", codec = Text, accessor = guid, required))]
#[xml(attribute(local = "scale", codec = Number<u32>, accessor = zoom_scale, default = 100))]
#[xml(attribute(local = "colorId", codec = Number<u32>, accessor = grid_colour_index, default = 64))]
#[xml(attribute(local = "showPageBreaks", codec = OnOff, accessor = shows_page_breaks, default = false))]
#[xml(attribute(local = "showFormulas", codec = OnOff, accessor = shows_formulas, default = false))]
#[xml(attribute(local = "showGridLines", codec = OnOff, accessor = shows_grid_lines, default = true))]
#[xml(attribute(local = "showRowCol", codec = OnOff, accessor = shows_row_and_column_headings, default = true))]
#[xml(attribute(local = "outlineSymbols", codec = OnOff, accessor = shows_outline_symbols, default = true))]
#[xml(attribute(local = "zeroValues", codec = OnOff, accessor = shows_zero_values, default = true))]
#[xml(attribute(local = "fitToPage", codec = OnOff, accessor = fits_to_page, default = false))]
#[xml(attribute(local = "printArea", codec = OnOff, accessor = has_print_area, default = false))]
#[xml(attribute(local = "filter", codec = OnOff, accessor = has_filter, default = false))]
#[xml(attribute(local = "showAutoFilter", codec = OnOff, accessor = shows_auto_filter_arrows, default = false))]
#[xml(attribute(local = "hiddenRows", codec = OnOff, accessor = has_hidden_rows, default = false))]
#[xml(attribute(local = "hiddenColumns", codec = OnOff, accessor = has_hidden_columns, default = false))]
#[xml(attribute(local = "state", codec = Enumeration<SheetState>, accessor = sheet_state, default = SheetState::Visible))]
#[xml(attribute(local = "filterUnique", codec = OnOff, accessor = filters_unique_values_only, default = false))]
#[xml(attribute(local = "view", codec = Enumeration<SheetViewType>, accessor = view_type, default = SheetViewType::Normal))]
#[xml(attribute(local = "showRuler", codec = OnOff, accessor = shows_ruler, default = true))]
#[xml(attribute(local = "topLeftCell", codec = Enumeration<CellReference>, accessor = top_left_cell))]
pub struct CustomSheetView {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "pane", variant = Pane, ty = SheetPane),
        child(local = "selection", variant = Selection, ty = Selection),
        child(local = "rowBreaks", variant = RowBreaks, ty = PageBreaks),
        child(local = "colBreaks", variant = ColumnBreaks, ty = PageBreaks),
        child(local = "pageMargins", variant = PageMargins, ty = PageMargins),
        child(local = "printOptions", variant = PrintOptions, ty = PrintOptions),
        child(local = "pageSetup", variant = PageSetup, ty = PageSetup),
        child(local = "headerFooter", variant = HeaderFooter, ty = HeaderFooter),
        child(local = "autoFilter", variant = AutoFilter, ty = AutoFilter)
    )]
    content: Vec<CustomSheetViewContent>,
}

/// One child of [`CustomSheetView`] — nine modelled slots, and everything else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomSheetViewContent {
    /// `x:pane` (rank 0) — where the view's panes were split or frozen.
    Pane(SheetPane),
    /// `x:selection` (rank 1) — what was selected when the view was saved.
    Selection(Selection),
    /// `x:rowBreaks` (rank 2) — the view's own page breaks, in the row axis.
    RowBreaks(PageBreaks),
    /// `x:colBreaks` (rank 3) — the same complex type in the column axis.
    ColumnBreaks(PageBreaks),
    /// `x:pageMargins` (rank 4).
    PageMargins(PageMargins),
    /// `x:printOptions` (rank 5).
    PrintOptions(PrintOptions),
    /// `x:pageSetup` (rank 6) — the full `CT_PageSetup`, even under a chartsheet.
    PageSetup(PageSetup),
    /// `x:headerFooter` (rank 7).
    HeaderFooter(HeaderFooter),
    /// `x:autoFilter` (rank 8) — the filter this view remembers, applied to nothing.
    AutoFilter(AutoFilter),
    /// Anything else — `x:extLst` above all — preserved verbatim, in position.
    Raw(RawNode),
}

impl CustomSheetViewContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    #[must_use]
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Pane(_) => "pane",
            Self::Selection(_) => "selection",
            Self::RowBreaks(_) => "rowBreaks",
            Self::ColumnBreaks(_) => "colBreaks",
            Self::PageMargins(_) => "pageMargins",
            Self::PrintOptions(_) => "printOptions",
            Self::PageSetup(_) => "pageSetup",
            Self::HeaderFooter(_) => "headerFooter",
            Self::AutoFilter(_) => "autoFilter",
            Self::Raw(_) => return None,
        })
    }

    /// This child rebuilt as a node.
    #[must_use]
    fn as_raw_node(&self) -> RawNode {
        match self {
            Self::Pane(value) => RawNode::Element(value.as_raw_element()),
            Self::Selection(value) => RawNode::Element(value.as_raw_element()),
            Self::RowBreaks(value) | Self::ColumnBreaks(value) => {
                RawNode::Element(value.as_raw_element())
            }
            Self::PageMargins(value) => RawNode::Element(value.as_raw_element()),
            Self::PrintOptions(value) => RawNode::Element(value.as_raw_element()),
            Self::PageSetup(value) => RawNode::Element(value.as_raw_element()),
            Self::HeaderFooter(value) => RawNode::Element(value.as_raw_element()),
            Self::AutoFilter(value) => RawNode::Element(value.as_raw_element()),
            Self::Raw(node) => node.clone(),
        }
    }
}

/// Declares one singleton child of [`CustomSheetView`]: a getter, a mutable getter and a setter that
/// places a new child at its rank in `CT_CustomSheetView`'s `xsd:sequence`.
macro_rules! view_child {
    ($getter:ident, $getter_mut:ident, $setter:ident, $variant:ident, $ty:ty, $local:literal, $doc:literal) => {
        #[doc = $doc]
        #[must_use]
        pub fn $getter(&self) -> Option<&$ty> {
            self.content.iter().find_map(|child| match child {
                CustomSheetViewContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("`x:", $local, "`, mutably — `None` if the view writes none.")]
        #[must_use]
        pub fn $getter_mut(&mut self) -> Option<&mut $ty> {
            self.content.iter_mut().find_map(|child| match child {
                CustomSheetViewContent::$variant(value) => Some(value),
                _ => None,
            })
        }

        #[doc = concat!("Sets `x:", $local, "`: `None` removes it; `Some(value)` replaces the \
            existing element **where it is**, or inserts a new one at its rank in \
            `CT_CustomSheetView`'s `xsd:sequence`.")]
        pub fn $setter(&mut self, value: Option<$ty>) {
            self.replace_or_insert(
                $local,
                |child| matches!(child, CustomSheetViewContent::$variant(_)),
                value.map(CustomSheetViewContent::$variant),
            );
        }
    };
}

impl CustomSheetView {
    /// Builds an empty `x:customSheetView`, bound to `prefix` or to the default namespace.
    ///
    /// Carries no `@guid`, which the schema declares required — set it before the view is written
    /// into a part, or the part will not validate. Nothing here invents one, because a GUID a
    /// library made up would not match any `x:customWorkbookView` in the workbook.
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
    pub fn content(&self) -> &[CustomSheetViewContent] {
        &self.content
    }

    view_child!(
        pane,
        pane_mut,
        set_pane,
        Pane,
        SheetPane,
        "pane",
        "`x:pane` (rank 0) — where this view's panes were split or frozen. **A record: nothing \
         here splits a pane.**"
    );
    view_child!(
        selection,
        selection_mut,
        set_selection,
        Selection,
        Selection,
        "selection",
        "`x:selection` (rank 1) — what was selected when the view was saved."
    );
    view_child!(
        row_breaks,
        row_breaks_mut,
        set_row_breaks,
        RowBreaks,
        PageBreaks,
        "rowBreaks",
        "`x:rowBreaks` (rank 2) — this view's own page breaks in the row axis, MJXOFF-117's \
         [`PageBreaks`](crate::PageBreaks) and not a second model of one."
    );
    view_child!(
        column_breaks,
        column_breaks_mut,
        set_column_breaks,
        ColumnBreaks,
        PageBreaks,
        "colBreaks",
        "`x:colBreaks` (rank 3) — the same complex type in the column axis."
    );
    view_child!(
        page_margins,
        page_margins_mut,
        set_page_margins,
        PageMargins,
        PageMargins,
        "pageMargins",
        "`x:pageMargins` (rank 4) — this view's own margins."
    );
    view_child!(
        print_options,
        print_options_mut,
        set_print_options,
        PrintOptions,
        PrintOptions,
        "printOptions",
        "`x:printOptions` (rank 5) — this view's own print options."
    );
    view_child!(
        page_setup,
        page_setup_mut,
        set_page_setup,
        PageSetup,
        PageSetup,
        "pageSetup",
        "`x:pageSetup` (rank 6) — the full `CT_PageSetup`, including its own `r:id` to a printer \
         settings part."
    );
    view_child!(
        header_footer,
        header_footer_mut,
        set_header_footer,
        HeaderFooter,
        HeaderFooter,
        "headerFooter",
        "`x:headerFooter` (rank 7) — this view's own six opaque strings."
    );
    view_child!(
        auto_filter,
        auto_filter_mut,
        set_auto_filter,
        AutoFilter,
        AutoFilter,
        "autoFilter",
        "`x:autoFilter` (rank 8) — the filter this view remembers, MJXOFF-123's \
         [`AutoFilter`](crate::AutoFilter). **Recorded, never applied:** no row's `@hidden` is set \
         from it."
    );

    /// Replaces the first child `is_target` accepts, keeping its position; inserts at the schema
    /// rank when there is none; removes it when `value` is `None`.
    ///
    /// The rank comes from [`mjx_ooxml_types::child_order::CUSTOM_SHEET_VIEW`], generated from
    /// `sml.xsd`. A child this type does not model contributes `None` and is stepped over, so an
    /// `extLst` or a foreign element never moves and never moves anything else.
    fn replace_or_insert(
        &mut self,
        local: &str,
        is_target: impl Fn(&CustomSheetViewContent) -> bool,
        value: Option<CustomSheetViewContent>,
    ) {
        let existing = self.content.iter().position(&is_target);
        match (existing, value) {
            (Some(at), Some(value)) => self.content[at] = value,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(value)) => {
                let at = CUSTOM_SHEET_VIEW.insert_index_of_names(
                    self.content.iter().map(|child| {
                        child
                            .local()
                            .and_then(|l| CUSTOM_SHEET_VIEW.rank_of(None, l))
                    }),
                    local,
                );
                self.content.insert(at, value);
                self.empty = false;
            }
            (None, None) => {}
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(CustomSheetViewContent::as_raw_node)
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CustomSheetView {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

/// `x:customSheetViews` (`CT_CustomSheetViews`, `sml.xsd:2531`) — every saved view of this sheet, in
/// document order.
///
/// **`ST_`/`CT_` symbol:** `CT_CustomSheetViews`. Wire element: `customSheetViews`, rank **13** of
/// `CT_Worksheet`, rank **10** of `CT_Macrosheet` and rank **4** of `CT_Dialogsheet`.
///
/// The schema declares `customSheetView` `minOccurs="1"`, so a sheet that writes this element at all
/// writes at least one view — the rule [`MergedCells`](crate::MergedCells) and
/// [`Hyperlinks`](crate::Hyperlinks) already follow, and the reason
/// [`remove`](Self::remove) leaves an empty element behind for the frame to take out rather than
/// silently emitting invalid markup.
///
/// `CT_CustomSheetViews` declares **no `@count`**, so there is no cache here to refresh or leave
/// stale.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml)]
#[xml(namespace = SML)]
pub struct CustomSheetViews {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "customSheetView", variant = View, ty = CustomSheetView))]
    content: Vec<CustomSheetViewsContent>,
}

/// One child of [`CustomSheetViews`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomSheetViewsContent {
    /// `x:customSheetView` — one saved view.
    View(CustomSheetView),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CustomSheetViews {
    /// Builds an empty `x:customSheetViews`, bound to `prefix` or to the default namespace.
    ///
    /// The schema declares `customSheetView` `minOccurs="1"`, so an element with no view is invalid
    /// markup; it is still constructible, because a caller builds one and then fills it.
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
    pub fn content(&self) -> &[CustomSheetViewsContent] {
        &self.content
    }

    /// Every `x:customSheetView`, in document order.
    pub fn views(&self) -> impl Iterator<Item = &CustomSheetView> + '_ {
        self.content.iter().filter_map(|child| match child {
            CustomSheetViewsContent::View(view) => Some(view),
            CustomSheetViewsContent::Raw(_) => None,
        })
    }

    /// How many views the sheet has saved.
    #[must_use]
    pub fn len(&self) -> usize {
        self.views().count()
    }

    /// Whether the element lists no view at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:customSheetView`, mutably.
    pub fn view_mut(&mut self, index: usize) -> Option<&mut CustomSheetView> {
        self.content
            .iter_mut()
            .filter_map(|child| match child {
                CustomSheetViewsContent::View(view) => Some(view),
                CustomSheetViewsContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// The view whose `@guid` is `guid`, or `None`.
    ///
    /// The comparison is byte-for-byte on the text the file wrote. `ST_Guid`'s XSD pattern permits
    /// only upper-case hexadecimal, so nothing here case-folds: a lower-case GUID is a file that
    /// does not match its own schema, and quietly matching it would hide that.
    #[must_use]
    pub fn view_with_guid(&self, interner: &Interner, guid: &str) -> Option<&CustomSheetView> {
        self.views()
            .find(|view| view.guid(interner).is_ok_and(|value| value == guid))
    }

    /// Appends a view after the ones already present.
    pub fn push(&mut self, view: CustomSheetView) {
        self.content.push(CustomSheetViewsContent::View(view));
        self.empty = false;
    }

    /// Removes the `index`-th `x:customSheetView` and returns it, or `None` when the element holds
    /// fewer.
    ///
    /// **The matching `x:customWorkbookView` in `xl/workbook.xml` is not touched** — this crate
    /// models the two elements and does not join them. A workbook view left naming a sheet view
    /// that no longer exists is a real shape Excel tolerates, and repairing it would be editing a
    /// second part on a guess.
    pub fn remove(&mut self, index: usize) -> Option<CustomSheetView> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, child)| matches!(child, CustomSheetViewsContent::View(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        match self.content.remove(at) {
            CustomSheetViewsContent::View(view) => Some(view),
            CustomSheetViewsContent::Raw(_) => unreachable!("the position was filtered on `View`"),
        }
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|child| match child {
                CustomSheetViewsContent::View(view) => RawNode::Element(view.as_raw_element()),
                CustomSheetViewsContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CustomSheetViews {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
