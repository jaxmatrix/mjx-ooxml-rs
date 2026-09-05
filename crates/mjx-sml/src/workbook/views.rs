//! `x:bookViews` and `x:customWorkbookViews` — how a consumer opens the workbook window, and the
//! saved views a user can switch between.
//!
//! `CT_BookViews` (`sml.xsd:4130`) holds one or more `x:workbookView` (`CT_BookView`, `4135`);
//! `CT_CustomWorkbookViews` (`4166`) holds one or more `x:customWorkbookView` (`CT_CustomWorkbookView`,
//! `4172`). They are two lists of window geometry, and the second is the largest attribute bag in the
//! cluster — **twenty-four** direct `xsd:attribute` declarations, counted from the schema.
//!
//! # Window geometry is a producer's, not a consumer's
//!
//! `xWindow`, `yWindow`, `windowWidth`, `windowHeight` and `tabRatio` describe the window on the
//! machine that saved the file, in screen units that mean nothing anywhere else. They are carried
//! because a file that loses them reopens differently; they are not interpreted, and nothing here
//! clamps them to a screen.
//!
//! `activeTab` and `firstSheet` are **indices into the tab list**, and the schema constrains neither
//! against the number of tabs. An `activeTab` past the end is a defect in the file, reported as the
//! number it is and never clamped — the same "do not repair" rule that governs an out-of-range
//! `definedName/@localSheetId`.

use mjx_ooxml_core::{Enumeration, Number, RawAttribute, RawName, RawNode, Text};
use mjx_ooxml_types::spreadsheetml::{CommentDisplay, ObjectDisplay, Visibility};
use mjx_ooxml_types::support::OnOff;

use super::leaf::attribute_bag;

attribute_bag! {
    /// `x:workbookView` (`CT_BookView`) — one window position, and what it shows.
    ///
    /// `sample.xlsx` writes all of `showHorizontalScroll`, `showVerticalScroll`, `showSheetTabs`,
    /// `xWindow`, `yWindow`, `windowWidth`, `windowHeight`, `tabRatio`, `firstSheet` and
    /// `activeTab` — ten of the thirteen.
    #[xml(attribute(local = "visibility", codec = Enumeration<Visibility>, accessor = visibility, default = Visibility::Visible))]
    #[xml(attribute(local = "minimized", codec = OnOff, accessor = minimized, default = false))]
    #[xml(attribute(local = "showHorizontalScroll", codec = OnOff, accessor = show_horizontal_scroll_bar, default = true))]
    #[xml(attribute(local = "showVerticalScroll", codec = OnOff, accessor = show_vertical_scroll_bar, default = true))]
    #[xml(attribute(local = "showSheetTabs", codec = OnOff, accessor = show_sheet_tabs, default = true))]
    #[xml(attribute(local = "xWindow", codec = Number<i32>, accessor = window_left))]
    #[xml(attribute(local = "yWindow", codec = Number<i32>, accessor = window_top))]
    #[xml(attribute(local = "windowWidth", codec = Number<u32>, accessor = window_width))]
    #[xml(attribute(local = "windowHeight", codec = Number<u32>, accessor = window_height))]
    #[xml(attribute(local = "tabRatio", codec = Number<u32>, accessor = tab_strip_ratio, default = 600))]
    #[xml(attribute(local = "firstSheet", codec = Number<u32>, accessor = first_visible_tab_index, default = 0))]
    #[xml(attribute(local = "activeTab", codec = Number<u32>, accessor = active_tab_index, default = 0))]
    #[xml(attribute(local = "autoFilterDateGrouping", codec = OnOff, accessor = auto_filter_date_grouping, default = true))]
    WorkbookView, "workbookView"
}

attribute_bag! {
    /// `x:customWorkbookView` (`CT_CustomWorkbookView`) — a named, saved view: window geometry plus
    /// everything a user's *custom view* remembers about print settings, hidden rows and the
    /// filter state.
    ///
    /// Twenty-four attributes, of which `name`, `guid`, `windowWidth`, `windowHeight` and
    /// `activeSheetId` are `use="required"`. They are declared required here too, so a getter says
    /// [`AttributeError::Missing`](mjx_ooxml_core::AttributeError::Missing) rather than substituting
    /// a value the file does not contain — reading is never repairing, and the element still writes
    /// back exactly as it arrived.
    #[xml(attribute(local = "name", codec = Text, accessor = name, required))]
    #[xml(attribute(local = "guid", codec = Text, accessor = guid, required))]
    #[xml(attribute(local = "autoUpdate", codec = OnOff, accessor = auto_update, default = false))]
    #[xml(attribute(local = "mergeInterval", codec = Number<u32>, accessor = merge_interval_minutes))]
    #[xml(attribute(local = "changesSavedWin", codec = OnOff, accessor = changes_saved_win, default = false))]
    #[xml(attribute(local = "onlySync", codec = OnOff, accessor = only_synchronize, default = false))]
    #[xml(attribute(local = "personalView", codec = OnOff, accessor = personal_view, default = false))]
    #[xml(attribute(local = "includePrintSettings", codec = OnOff, accessor = include_print_settings, default = true))]
    #[xml(attribute(local = "includeHiddenRowCol", codec = OnOff, accessor = include_hidden_rows_and_columns, default = true))]
    #[xml(attribute(local = "maximized", codec = OnOff, accessor = maximized, default = false))]
    #[xml(attribute(local = "minimized", codec = OnOff, accessor = minimized, default = false))]
    #[xml(attribute(local = "showHorizontalScroll", codec = OnOff, accessor = show_horizontal_scroll_bar, default = true))]
    #[xml(attribute(local = "showVerticalScroll", codec = OnOff, accessor = show_vertical_scroll_bar, default = true))]
    #[xml(attribute(local = "showSheetTabs", codec = OnOff, accessor = show_sheet_tabs, default = true))]
    #[xml(attribute(local = "xWindow", codec = Number<i32>, accessor = window_left, default = 0))]
    #[xml(attribute(local = "yWindow", codec = Number<i32>, accessor = window_top, default = 0))]
    #[xml(attribute(local = "windowWidth", codec = Number<u32>, accessor = window_width, required))]
    #[xml(attribute(local = "windowHeight", codec = Number<u32>, accessor = window_height, required))]
    #[xml(attribute(local = "tabRatio", codec = Number<u32>, accessor = tab_strip_ratio, default = 600))]
    #[xml(attribute(local = "activeSheetId", codec = Number<u32>, accessor = active_sheet_id, required))]
    #[xml(attribute(local = "showFormulaBar", codec = OnOff, accessor = show_formula_bar, default = true))]
    #[xml(attribute(local = "showStatusbar", codec = OnOff, accessor = show_status_bar, default = true))]
    #[xml(attribute(local = "showComments", codec = Enumeration<CommentDisplay>, accessor = comment_display, default = CommentDisplay::IndicatorOnly))]
    #[xml(attribute(local = "showObjects", codec = Enumeration<ObjectDisplay>, accessor = object_display, default = ObjectDisplay::All))]
    CustomWorkbookView, "customWorkbookView"
}

/// `x:bookViews` (`CT_BookViews`) — the workbook's window views, in document order.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct BookViews {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "workbookView", variant = View, ty = WorkbookView))]
    content: Vec<BookViewsContent>,
}

/// One child of [`BookViews`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BookViewsContent {
    /// `x:workbookView`.
    View(WorkbookView),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl BookViews {
    /// Builds an empty `x:bookViews`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "bookViews"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:workbookView`, in document order.
    pub fn views(&self) -> impl Iterator<Item = &WorkbookView> + '_ {
        self.content.iter().filter_map(|item| match item {
            BookViewsContent::View(view) => Some(view),
            BookViewsContent::Raw(_) => None,
        })
    }

    /// Appends a view after the ones already present.
    pub fn push(&mut self, view: WorkbookView) {
        self.content.push(BookViewsContent::View(view));
    }
}

/// `x:customWorkbookViews` (`CT_CustomWorkbookViews`) — the saved custom views, in document order.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::ToXml)]
#[xml(namespace = SML)]
pub struct CustomWorkbookViews {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "customWorkbookView", variant = View, ty = CustomWorkbookView))]
    content: Vec<CustomWorkbookViewsContent>,
}

/// One child of [`CustomWorkbookViews`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomWorkbookViewsContent {
    /// `x:customWorkbookView`.
    View(CustomWorkbookView),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CustomWorkbookViews {
    /// Builds an empty `x:customWorkbookViews`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut mjx_ooxml_core::Interner, prefix: Option<&str>) -> Self {
        Self {
            name: super::leaf::sml_name(interner, prefix, "customWorkbookViews"),
            attributes: Vec::new(),
            empty: true,
            content: Vec::new(),
        }
    }

    /// Every `x:customWorkbookView`, in document order.
    pub fn views(&self) -> impl Iterator<Item = &CustomWorkbookView> + '_ {
        self.content.iter().filter_map(|item| match item {
            CustomWorkbookViewsContent::View(view) => Some(view),
            CustomWorkbookViewsContent::Raw(_) => None,
        })
    }

    /// Appends a view after the ones already present.
    pub fn push(&mut self, view: CustomWorkbookView) {
        self.content.push(CustomWorkbookViewsContent::View(view));
    }
}
