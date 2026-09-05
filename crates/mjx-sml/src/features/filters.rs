//! The autofilter cluster: `x:autoFilter`, the six filter kinds a column may carry, and the
//! `x:sortState` that records a sort somebody performed.
//!
//! | Type | `sml.xsd` | Element |
//! |---|---|---|
//! | `CT_AutoFilter` | 16 | `x:autoFilter` (rank **10** of `CT_Worksheet`) |
//! | `CT_FilterColumn` | 24 | `x:autoFilter/filterColumn` |
//! | `CT_Filters` | 38 | `x:filterColumn/filters` |
//! | `CT_Filter` | 47 | `x:filters/filter` |
//! | `CT_CustomFilters` | 50 | `x:filterColumn/customFilters` |
//! | `CT_CustomFilter` | 56 | `x:customFilters/customFilter` |
//! | `CT_Top10` | 60 | `x:filterColumn/top10` |
//! | `CT_ColorFilter` | 66 | `x:filterColumn/colorFilter` |
//! | `CT_IconFilter` | 70 | `x:filterColumn/iconFilter` |
//! | `CT_DynamicFilter` | 86 | `x:filterColumn/dynamicFilter` |
//! | `CT_SortState` | 151 | `x:autoFilter/sortState`, and `x:table/sortState` |
//! | `CT_SortCondition` | 161 | `x:sortState/sortCondition` |
//! | `CT_DateGroupItem` | 180 | `x:filters/dateGroupItem` |
//!
//! # Nothing here filters, and nothing here sorts
//!
//! **The rule that governs every type in this file.** An autofilter hides rows *in Excel* and a sort
//! state records a sort *Excel performed*. This library records both and performs neither:
//!
//! * **No row's `@hidden` is ever set from a filter.** A hidden row read out of a file is
//!   [`RowHeight`](crate::RowHeight)'s neighbour in MJXOFF-117's row properties, preserved exactly as
//!   read, and adding a `filterColumn` over it changes nothing about it. Deriving visibility from a
//!   filter would mean evaluating the filter, which needs the comparison semantics of every
//!   `ST_FilterOperator` against every cell type — and would then write a `@hidden` the caller never
//!   asked for into rows they never named.
//! * **No row is ever reordered.** `CT_SortState` is, in §18.3.1.92's words, a record of *"the sort
//!   state for the auto filter"*: it says what the last sort was, not what the data should now be.
//!   Applying it would rewrite `sheetData` — every row, every formula's relative reference, every
//!   merge — on a read.
//! * **No filter's value is interpreted.** `customFilter@val` is an `ST_Xstring` that may hold
//!   `>=1000`, a wildcard `Sm?th*`, or a serial date; `dynamicFilter@valIso` is an `xsd:dateTime`
//!   this crate never parses. MJXOFF-115's contract governs all of them: text in, the same text out.
//!
//! This is the shape MJXOFF-123 names as its own hazard, and it has three faces because there are
//! three tempting places to be helpful. All three are refused.
//!
//! # The choice is six kinds and an extension slot, not seven kinds
//!
//! `CT_FilterColumn`'s content model is an `xsd:choice` with **seven members**, of which **six are
//! filter kinds** — `filters`, `top10`, `customFilters`, `dynamicFilter`, `colorFilter`,
//! `iconFilter` — and the seventh is `extLst`. [`FilterKind`] carries the six, and `extLst` (with
//! anything else a future extension puts there) lands in [`FilterKind::Raw`] and comes back byte for
//! byte. `@colId`, `@hiddenButton` and `@showButton` are attributes of the column, not members of the
//! choice.
//!
//! Because it is a choice rather than a sequence, every member ranks 0 in
//! [`mjx_ooxml_types::child_order::FILTER_COLUMN`], and that is the schema's own answer rather than
//! a shortcoming of the table: a column carries **at most one** child, so there is no order to hold.
//! [`FilterColumn::set_filter`] therefore replaces whichever kind is there — a `top10` supersedes a
//! `filters` *in its position* — which is the `xsd:choice` case
//! [`ChildOrder::replace_or_insert`](mjx_ooxml_types::child_order::ChildOrder::replace_or_insert)
//! exists for.
//!
//! # `@dxfId` is a position, and this file only reads one
//!
//! `colorFilter@dxfId` and `sortCondition@dxfId` both index `xl/styles.xml`'s `dxfs` table, exactly
//! as `cfRule@dxfId` does. [`DifferentialFormats`](crate::DifferentialFormats)' rule holds unchanged:
//! **append, never reorder**, because inserting a `dxf` silently repoints every index above it. The
//! one allocator is
//! [`StylesheetPart::append_differential_format`](crate::StylesheetPart::append_differential_format)
//! and this file adds no second one.
//!
//! # The icon-set variant names are read, never inferred
//!
//! `iconFilter@iconSet` and `sortCondition@iconSet` are `ST_IconSetType`, whose wire tokens are
//! digit-leading and whose generated names are **not** derivable from them: `3TrafficLights1` is
//! [`IconSetType::ThreeTrafficLights`], and `3Symbols` is `ThreeSymbolsCircled` while `3Symbols2` is
//! `ThreeSymbols`. Nothing here spells a token by hand; the generated enumeration is named and its
//! own documentation carries the wire value.

use mjx_ooxml_core::{
    Enumeration, Interner, Number, RawAttribute, RawElement, RawName, RawNode, Text, ToXml,
};
use mjx_ooxml_types::child_order::{AUTO_FILTER, FILTERS, FILTER_COLUMN, SORT_STATE};
use mjx_ooxml_types::shared::CalendarType;
use mjx_ooxml_types::spreadsheetml::{
    DateTimeGrouping, DynamicFilterType, FilterOperator, IconSetType, SortBy, SortMethod,
};
use mjx_ooxml_types::support::OnOff;

use crate::address::CellRange;
use crate::leaf::attribute_bag;
use crate::worksheet::rebuild_element;

// -----------------------------------------------------------------------------------------------
// The leaves: the eight complex types that declare no children at all
// -----------------------------------------------------------------------------------------------

attribute_bag! {
    /// `x:filter` (`CT_Filter`, `sml.xsd:47`) — one literal value a value filter keeps.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_Filter`. Wire element: `filter`.
    ///
    /// `@val` is an `ST_Xstring` and stays text. Excel writes the cell's *displayed* value here, so a
    /// filter over a date column holds `"3/14/2015"` and not a serial number — which is one more
    /// reason nothing in this crate compares it to anything.
    #[xml(attribute(local = "val", codec = Text, accessor = value))]
    Filter, "filter"
}

attribute_bag! {
    /// `x:customFilter` (`CT_CustomFilter`, `sml.xsd:56`) — one half of a custom filter's comparison.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_CustomFilter`. Wire element: `customFilter`.
    ///
    /// `@operator` defaults to `equal`, and `@val` is an `ST_Xstring` that may hold a wildcard
    /// (`Sm?th*`) as readily as a number. Neither is evaluated; see this module's own documentation.
    #[xml(attribute(
        local = "operator",
        codec = Enumeration<FilterOperator>,
        accessor = operator,
        default = FilterOperator::Equal
    ))]
    #[xml(attribute(local = "val", codec = Text, accessor = value))]
    CustomFilter, "customFilter"
}

attribute_bag! {
    /// `x:top10` (`CT_Top10`, `sml.xsd:60`) — the *top or bottom N* (or N %) filter.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_Top10`. Wire element: `top10`.
    ///
    /// `@val` is the `use="required"` count or percentage the user asked for; `@filterVal` is the
    /// **cell value Excel derived from it** — the threshold the filter actually compares against —
    /// and is cached data this library reports and never recomputes, exactly as
    /// [`SheetDimension`](crate::SheetDimension)'s box is.
    ///
    /// `@top` defaults to `true` (the *top* N rather than the bottom) and `@percent` to `false`.
    #[xml(attribute(local = "top", codec = OnOff, accessor = takes_the_top, default = true))]
    #[xml(attribute(local = "percent", codec = OnOff, accessor = is_percentage, default = false))]
    #[xml(attribute(local = "val", codec = Number<f64>, accessor = value, required))]
    #[xml(attribute(local = "filterVal", codec = Number<f64>, accessor = derived_threshold))]
    Top10Filter, "top10"
}

attribute_bag! {
    /// `x:colorFilter` (`CT_ColorFilter`, `sml.xsd:66`) — *filter by colour*, named by `dxf` index.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_ColorFilter`. Wire element: `colorFilter`.
    ///
    /// `@dxfId` indexes `xl/styles.xml`'s `dxfs` table — a **position**, so the append-only rule in
    /// this module's own documentation holds. `@cellColor` defaults to `true` and says whether the
    /// colour meant is the cell's fill or its font.
    #[xml(attribute(local = "dxfId", codec = Number<u32>, accessor = differential_format_index))]
    #[xml(attribute(local = "cellColor", codec = OnOff, accessor = is_cell_color, default = true))]
    ColorFilter, "colorFilter"
}

attribute_bag! {
    /// `x:iconFilter` (`CT_IconFilter`, `sml.xsd:70`) — *filter by conditional-formatting icon*.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_IconFilter`. Wire element: `iconFilter`.
    ///
    /// `@iconSet` is `use="required"` and names one of `ST_IconSetType`'s seventeen sets; `@iconId`
    /// is the zero-based position of the icon *within* that set, so a `3TrafficLights1` filter on
    /// the red light is `iconId="0"`. An absent `@iconId` means the column is filtered on the set
    /// rather than on one icon.
    #[xml(attribute(
        local = "iconSet",
        codec = Enumeration<IconSetType>,
        accessor = icon_set,
        required
    ))]
    #[xml(attribute(local = "iconId", codec = Number<u32>, accessor = icon_index))]
    IconFilter, "iconFilter"
}

attribute_bag! {
    /// `x:dynamicFilter` (`CT_DynamicFilter`, `sml.xsd:86`) — a filter whose bounds Excel recomputes.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_DynamicFilter`. Wire element: `dynamicFilter`.
    ///
    /// `@type` is `use="required"` and is `ST_DynamicFilterType`'s thirty-five values: the two
    /// averages, the relative periods from `yesterday` to `nextYear`, `yearToDate`, the four quarters
    /// (`Q1`–`Q4`) and the twelve months (`M1`–`M12`). *Dynamic* is the point — `thisMonth` means
    /// something different next month — so `@val`/`@maxVal` and their ISO spellings are the bounds
    /// Excel derived **when it last saved**, cached data this library reports and never recomputes.
    ///
    /// `@valIso` and `@maxValIso` are `xsd:dateTime` and are carried as [`Text`]: parsing them would
    /// buy nothing a caller cannot do, and re-emitting a parsed one would change a file's bytes for
    /// no reason. The ticket calls the second pair `maxValue`; the schema spells it `maxVal`.
    #[xml(attribute(
        local = "type",
        codec = Enumeration<DynamicFilterType>,
        accessor = kind,
        required
    ))]
    #[xml(attribute(local = "val", codec = Number<f64>, accessor = value))]
    #[xml(attribute(local = "valIso", codec = Text, accessor = value_iso))]
    #[xml(attribute(local = "maxVal", codec = Number<f64>, accessor = maximum_value))]
    #[xml(attribute(local = "maxValIso", codec = Text, accessor = maximum_value_iso))]
    DynamicFilter, "dynamicFilter"
}

attribute_bag! {
    /// `x:dateGroupItem` (`CT_DateGroupItem`, `sml.xsd:180`) — one date-grouping node of a value
    /// filter.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_DateGroupItem`. Wire element: `dateGroupItem`.
    ///
    /// This is what the *(Select All) → 2015 → March* tree in Excel's filter drop-down becomes:
    /// `@dateTimeGrouping` says how deep the node sits, and the fields above that depth are the ones
    /// that mean anything. `@year` and `@dateTimeGrouping` are both `use="required"`; a
    /// `dateTimeGrouping="month"` item states `@year` and `@month` and leaves the rest absent.
    ///
    /// Every field is an `xsd:unsignedShort` and none is range-checked here: a `@month` of 13 is the
    /// file's defect, and refusing to read the sheet over it would lose the other 200,000 cells.
    #[xml(attribute(local = "year", codec = Number<u16>, accessor = year, required))]
    #[xml(attribute(local = "month", codec = Number<u16>, accessor = month))]
    #[xml(attribute(local = "day", codec = Number<u16>, accessor = day))]
    #[xml(attribute(local = "hour", codec = Number<u16>, accessor = hour))]
    #[xml(attribute(local = "minute", codec = Number<u16>, accessor = minute))]
    #[xml(attribute(local = "second", codec = Number<u16>, accessor = second))]
    #[xml(attribute(
        local = "dateTimeGrouping",
        codec = Enumeration<DateTimeGrouping>,
        accessor = grouping,
        required
    ))]
    DateGroupItem, "dateGroupItem"
}

attribute_bag! {
    /// `x:sortCondition` (`CT_SortCondition`, `sml.xsd:161`) — one column of a recorded sort.
    ///
    /// **`ST_`/`CT_` symbol:** `CT_SortCondition`. Wire element: `sortCondition`.
    ///
    /// `@ref` is `use="required"` and is the range **this condition** sorts on, which is one column
    /// of the [`SortState`]'s own wider `@ref`. `@sortBy` says what is compared — the value, the cell
    /// colour, the font colour, or the conditional-formatting icon — and `@dxfId` or
    /// `@iconSet`/`@iconId` name *which* colour or icon when it is not the value.
    ///
    /// `@customList` is the comma-separated user-defined order Excel sorts by when it is present
    /// (`"Low,Medium,High"`), and is an `ST_Xstring` carried as text.
    ///
    /// **A condition is a record.** Nothing here reorders a row; see this module's own documentation.
    #[xml(attribute(local = "descending", codec = OnOff, accessor = is_descending, default = false))]
    #[xml(attribute(
        local = "sortBy",
        codec = Enumeration<SortBy>,
        accessor = sort_by,
        default = SortBy::Value
    ))]
    #[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
    #[xml(attribute(local = "customList", codec = Text, accessor = custom_list))]
    #[xml(attribute(local = "dxfId", codec = Number<u32>, accessor = differential_format_index))]
    #[xml(attribute(
        local = "iconSet",
        codec = Enumeration<IconSetType>,
        accessor = icon_set,
        default = IconSetType::ThreeArrows
    ))]
    #[xml(attribute(local = "iconId", codec = Number<u32>, accessor = icon_index))]
    SortCondition, "sortCondition"
}

// -----------------------------------------------------------------------------------------------
// `x:filters` — the value list, and the date-group tree beside it
// -----------------------------------------------------------------------------------------------

/// `x:filters` (`CT_Filters`, `sml.xsd:38`) — the *tick the values you want* filter.
///
/// **`ST_`/`CT_` symbol:** `CT_Filters`. Wire element: `filters`.
///
/// Two child lists in sequence — the literal [`Filter`] values, then the [`DateGroupItem`] tree — and
/// **two attributes the ticket does not list**:
///
/// * `@blank` (default `false`) is the *(Blanks)* entry of the drop-down. It is not expressible as a
///   `filter` child, because an empty `@val` is the empty string rather than the absence of one, so a
///   model that dropped `@blank` would lose *(Blanks)* from every filter that had it.
/// * `@calendarType` (`ST_CalendarType`, default `none`) is the calendar the `dateGroupItem`
///   children are expressed in — `hijri`, `hebrew`, `japan` and eleven more — so dropping it would
///   reinterpret every date in the filter as Gregorian.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "blank", codec = OnOff, accessor = includes_blanks, default = false))]
#[xml(attribute(
    local = "calendarType",
    codec = Enumeration<CalendarType>,
    accessor = calendar,
    default = CalendarType::None
))]
pub struct Filters {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "filter", variant = Value, ty = Filter),
        child(local = "dateGroupItem", variant = DateGroup, ty = DateGroupItem)
    )]
    content: Vec<FiltersContent>,
}

/// One child of [`Filters`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FiltersContent {
    /// `x:filter` (rank 0) — one literal value.
    Value(Filter),
    /// `x:dateGroupItem` (rank 1) — one node of the date-grouping tree.
    DateGroup(DateGroupItem),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl FiltersContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Value(_) => "filter",
            Self::DateGroup(_) => "dateGroupItem",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_Filters`' `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        FILTERS.rank_of(None, self.local()?)
    }
}

impl Filters {
    /// Builds an empty `x:filters`, bound to `prefix` or to the default namespace.
    ///
    /// Empty is legal here: both child lists are `minOccurs="0"`, and a `filters` with nothing but
    /// `blank="1"` is exactly *(Blanks) only*.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "filters"),
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

    /// Every child, in document order, including anything unmodelled.
    #[must_use]
    pub fn content(&self) -> &[FiltersContent] {
        &self.content
    }

    /// Every `x:filter`, in document order.
    pub fn values(&self) -> impl Iterator<Item = &Filter> + '_ {
        self.content.iter().filter_map(|item| match item {
            FiltersContent::Value(filter) => Some(filter),
            _ => None,
        })
    }

    /// Every `x:dateGroupItem`, in document order.
    pub fn date_groups(&self) -> impl Iterator<Item = &DateGroupItem> + '_ {
        self.content.iter().filter_map(|item| match item {
            FiltersContent::DateGroup(item) => Some(item),
            _ => None,
        })
    }

    /// The `index`-th `x:filter`, mutably.
    pub fn value_mut(&mut self, index: usize) -> Option<&mut Filter> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                FiltersContent::Value(filter) => Some(filter),
                _ => None,
            })
            .nth(index)
    }

    /// Appends a literal value, **before** the date-group items if the element has any.
    pub fn push_value(&mut self, filter: Filter) {
        let at =
            FILTERS.insert_index_of_names(self.content.iter().map(FiltersContent::rank), "filter");
        self.content.insert(at, FiltersContent::Value(filter));
        self.empty = false;
    }

    /// Appends a date-group item after the ones already present.
    pub fn push_date_group(&mut self, item: DateGroupItem) {
        let at = FILTERS.insert_index_of_names(
            self.content.iter().map(FiltersContent::rank),
            "dateGroupItem",
        );
        self.content.insert(at, FiltersContent::DateGroup(item));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                FiltersContent::Value(filter) => RawNode::Element(filter.as_raw_element()),
                FiltersContent::DateGroup(item) => RawNode::Element(item.as_raw_element()),
                FiltersContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for Filters {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:customFilters` — the one-or-two comparison pair
// -----------------------------------------------------------------------------------------------

/// `x:customFilters` (`CT_CustomFilters`, `sml.xsd:50`) — *Custom AutoFilter*: one or two
/// comparisons, joined by `@and`.
///
/// **`ST_`/`CT_` symbol:** `CT_CustomFilters`. Wire element: `customFilters`.
///
/// The schema declares `customFilter` `minOccurs="1" maxOccurs="2"`, which is the dialog: one
/// comparison, or two joined by *And*/*Or*. `@and` defaults to `false`, so a pair with no `@and` is
/// an **or**.
///
/// Nothing here enforces the cardinality on read. A producer that wrote three comparisons wrote a
/// file this library reports as it stands, exactly as [`ColorScale`](crate::ColorScale) reports an
/// unbalanced scale rather than repairing it.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "and", codec = OnOff, accessor = requires_both, default = false))]
pub struct CustomFilters {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "customFilter", variant = Comparison, ty = CustomFilter))]
    content: Vec<CustomFiltersContent>,
}

/// One child of [`CustomFilters`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomFiltersContent {
    /// `x:customFilter` — one comparison.
    Comparison(CustomFilter),
    /// Anything else — preserved verbatim, in position.
    Raw(RawNode),
}

impl CustomFilters {
    /// Builds an empty `x:customFilters`, bound to `prefix` or to the default namespace.
    ///
    /// The schema requires at least one comparison, so an empty one is invalid markup; it is still
    /// constructible, because a caller builds one and then fills it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "customFilters"),
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

    /// Every child, in document order, including anything unmodelled.
    #[must_use]
    pub fn content(&self) -> &[CustomFiltersContent] {
        &self.content
    }

    /// Every `x:customFilter`, in document order.
    pub fn comparisons(&self) -> impl Iterator<Item = &CustomFilter> + '_ {
        self.content.iter().filter_map(|item| match item {
            CustomFiltersContent::Comparison(filter) => Some(filter),
            CustomFiltersContent::Raw(_) => None,
        })
    }

    /// How many comparisons this element holds — one or two in any file Excel wrote.
    #[must_use]
    pub fn len(&self) -> usize {
        self.comparisons().count()
    }

    /// Whether the element holds no comparison at all, which the schema forbids.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:customFilter`, mutably.
    pub fn comparison_mut(&mut self, index: usize) -> Option<&mut CustomFilter> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                CustomFiltersContent::Comparison(filter) => Some(filter),
                CustomFiltersContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a comparison after the ones already present.
    pub fn push(&mut self, filter: CustomFilter) {
        self.content.push(CustomFiltersContent::Comparison(filter));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                CustomFiltersContent::Comparison(filter) => {
                    RawNode::Element(filter.as_raw_element())
                }
                CustomFiltersContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for CustomFilters {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:filterColumn` — the choice
// -----------------------------------------------------------------------------------------------

/// `x:filterColumn` (`CT_FilterColumn`, `sml.xsd:24`) — the filter on one column of an autofilter.
///
/// **`ST_`/`CT_` symbol:** `CT_FilterColumn`. Wire element: `filterColumn`.
///
/// `@colId` is `use="required"` and is the **zero-based offset of the column within the autofilter's
/// own `@ref`**, not a worksheet column index: an `autoFilter ref="C1:F20"` with
/// `filterColumn colId="1"` filters column `D`. Nothing here translates between the two, because the
/// translation needs the parent's `@ref` and a `filterColumn` is also reachable from
/// [`AutoFilter`]'s list where it is.
///
/// `@hiddenButton` (default `false`) and `@showButton` (default `true`) are drop-down chrome.
///
/// The content is a *choice*: see this module's own documentation for why [`FilterKind`] has six
/// modelled members rather than seven, and for why every member ranks 0.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "colId", codec = Number<u32>, accessor = column_offset, required))]
#[xml(attribute(
    local = "hiddenButton",
    codec = OnOff,
    accessor = hides_the_button,
    default = false
))]
#[xml(attribute(local = "showButton", codec = OnOff, accessor = shows_the_button, default = true))]
pub struct FilterColumn {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "filters", variant = Values, ty = Filters),
        child(local = "top10", variant = Top10, ty = Top10Filter),
        child(local = "customFilters", variant = Custom, ty = CustomFilters),
        child(local = "dynamicFilter", variant = Dynamic, ty = DynamicFilter),
        child(local = "colorFilter", variant = Color, ty = ColorFilter),
        child(local = "iconFilter", variant = Icon, ty = IconFilter)
    )]
    content: Vec<FilterKind>,
}

/// The `xsd:choice` a [`FilterColumn`] carries: **six** filter kinds, and the extension slot.
///
/// Named `FilterKind` rather than `FilterColumnContent` — the convention this crate's *sequence*
/// content enums follow — because the schema's model here is a choice: the members are alternatives
/// rather than an order, and a column carries at most one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterKind {
    /// `x:filters` — the literal values (and the date-group tree) the user ticked.
    Values(Filters),
    /// `x:top10` — the top or bottom N, or N %.
    Top10(Top10Filter),
    /// `x:customFilters` — one or two comparisons joined by *And*/*Or*.
    Custom(CustomFilters),
    /// `x:dynamicFilter` — a relative period or an average, whose bounds Excel recomputes.
    Dynamic(DynamicFilter),
    /// `x:colorFilter` — by cell or font colour, named by `dxf` index.
    Color(ColorFilter),
    /// `x:iconFilter` — by conditional-formatting icon.
    Icon(IconFilter),
    /// `x:extLst` — the seventh member of the choice, where a future filter kind arrives — and
    /// anything else, preserved verbatim and in position.
    ///
    /// This is not a hypothetical: `extLst` is declared in the choice today, and the `x14`
    /// namespace already puts filters in it.
    Raw(RawNode),
}

impl FilterKind {
    /// This kind's wire local name, or `None` for an unmodelled node.
    #[must_use]
    pub fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Values(_) => "filters",
            Self::Top10(_) => "top10",
            Self::Custom(_) => "customFilters",
            Self::Dynamic(_) => "dynamicFilter",
            Self::Color(_) => "colorFilter",
            Self::Icon(_) => "iconFilter",
            Self::Raw(_) => return None,
        })
    }

    /// Whether this is one of the six modelled filter kinds rather than the extension slot.
    #[must_use]
    pub fn is_modelled(&self) -> bool {
        !matches!(self, Self::Raw(_))
    }

    /// This kind's rank in `CT_FilterColumn`'s `xsd:choice`, from the generated table — **0 for
    /// every member**, because a choice states alternatives rather than an order.
    fn rank(&self) -> Option<u16> {
        FILTER_COLUMN.rank_of(None, self.local()?)
    }
}

impl FilterColumn {
    /// Builds an `x:filterColumn` with no filter at all, bound to `prefix` or to the default
    /// namespace.
    ///
    /// `@colId` is `use="required"` and is **not** set here: a column offset invented on a caller's
    /// behalf would filter a column they did not name. Build the element and then state it.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "filterColumn"),
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

    /// Every child, in document order — the `extLst` and anything else unmodelled included.
    #[must_use]
    pub fn content(&self) -> &[FilterKind] {
        &self.content
    }

    /// The filter this column carries, or `None` when it carries none of the six modelled kinds.
    ///
    /// The first modelled member, which for schema-valid markup is the only one.
    #[must_use]
    pub fn filter(&self) -> Option<&FilterKind> {
        self.content.iter().find(|item| item.is_modelled())
    }

    /// The filter this column carries, mutably.
    pub fn filter_mut(&mut self) -> Option<&mut FilterKind> {
        self.content.iter_mut().find(|item| item.is_modelled())
    }

    /// Sets the column's filter: `None` removes whichever kind is there, `Some` replaces it **in its
    /// position** or inserts one when the column has none.
    ///
    /// Replacing rather than appending is what the `xsd:choice` asks for — a `top10` supersedes a
    /// `filters`, and a column never ends up holding two.
    pub fn set_filter(&mut self, kind: Option<FilterKind>) {
        let existing = self.content.iter().position(FilterKind::is_modelled);
        match (existing, kind) {
            (Some(at), Some(kind)) => self.content[at] = kind,
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(kind)) => {
                let at = match kind.local() {
                    Some(local) => FILTER_COLUMN
                        .insert_index_of_names(self.content.iter().map(FilterKind::rank), local),
                    None => self.content.len(),
                };
                self.content.insert(at, kind);
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
            .map(|item| match item {
                FilterKind::Values(filters) => RawNode::Element(filters.as_raw_element()),
                FilterKind::Top10(filter) => RawNode::Element(filter.as_raw_element()),
                FilterKind::Custom(filters) => RawNode::Element(filters.as_raw_element()),
                FilterKind::Dynamic(filter) => RawNode::Element(filter.as_raw_element()),
                FilterKind::Color(filter) => RawNode::Element(filter.as_raw_element()),
                FilterKind::Icon(filter) => RawNode::Element(filter.as_raw_element()),
                FilterKind::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for FilterColumn {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:sortState` — the record of a sort
// -----------------------------------------------------------------------------------------------

/// `x:sortState` (`CT_SortState`, `sml.xsd:151`) — the sort Excel last performed over a range.
///
/// **`ST_`/`CT_` symbol:** `CT_SortState`. Wire element: `sortState`. It is rank 1 of
/// `CT_AutoFilter` and also a direct child of `CT_Table`, which is why this type lives in
/// [`crate::features::filters`] rather than inside either owner.
///
/// `@ref` is `use="required"` and is the sorted range. `@columnSort` (default `false`) says the sort
/// ran left-to-right rather than top-to-bottom; `@caseSensitive` defaults to `false`; `@sortMethod`
/// (`stroke`, `pinYin`, `none`) is the East Asian collation, defaulting to `none`.
///
/// The schema allows up to **64** conditions. Nothing here enforces that, and nothing here sorts:
/// see this module's own documentation.
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "columnSort", codec = OnOff, accessor = sorts_columns, default = false))]
#[xml(attribute(
    local = "caseSensitive",
    codec = OnOff,
    accessor = is_case_sensitive,
    default = false
))]
#[xml(attribute(
    local = "sortMethod",
    codec = Enumeration<SortMethod>,
    accessor = sort_method,
    default = SortMethod::None
))]
#[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range, required))]
pub struct SortState {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(children, child(local = "sortCondition", variant = Condition, ty = SortCondition))]
    content: Vec<SortStateContent>,
}

/// One child of [`SortState`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortStateContent {
    /// `x:sortCondition` (rank 0) — one column of the sort.
    Condition(SortCondition),
    /// `x:extLst` (rank 1) — and anything else, preserved verbatim and in position.
    Raw(RawNode),
}

impl SortStateContent {
    /// This child's rank in `CT_SortState`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        match self {
            Self::Condition(_) => SORT_STATE.rank_of(None, "sortCondition"),
            Self::Raw(_) => None,
        }
    }
}

impl SortState {
    /// Builds an empty `x:sortState`, bound to `prefix` or to the default namespace.
    ///
    /// `@ref` is `use="required"` and is not set here, for the reason [`FilterColumn::new`] leaves
    /// `@colId` unset.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "sortState"),
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

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[SortStateContent] {
        &self.content
    }

    /// Every `x:sortCondition`, in document order.
    ///
    /// **Document order is the sort's own order**: the first condition is the primary key. Nothing
    /// here sorts the conditions themselves.
    pub fn conditions(&self) -> impl Iterator<Item = &SortCondition> + '_ {
        self.content.iter().filter_map(|item| match item {
            SortStateContent::Condition(condition) => Some(condition),
            SortStateContent::Raw(_) => None,
        })
    }

    /// How many conditions the sort states.
    #[must_use]
    pub fn len(&self) -> usize {
        self.conditions().count()
    }

    /// Whether the sort states no condition at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The `index`-th `x:sortCondition`, mutably.
    pub fn condition_mut(&mut self, index: usize) -> Option<&mut SortCondition> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                SortStateContent::Condition(condition) => Some(condition),
                SortStateContent::Raw(_) => None,
            })
            .nth(index)
    }

    /// Appends a condition after the ones already present, and **before** an `extLst` if there is
    /// one.
    ///
    /// It becomes the least significant key, because the first condition is the primary one.
    pub fn push_condition(&mut self, condition: SortCondition) {
        let at = SORT_STATE.insert_index_of_names(
            self.content.iter().map(SortStateContent::rank),
            "sortCondition",
        );
        self.content
            .insert(at, SortStateContent::Condition(condition));
        self.empty = false;
    }

    /// This element rebuilt as a [`RawElement`], without an interner.
    #[must_use]
    pub fn as_raw_element(&self) -> RawElement {
        let children = self
            .content
            .iter()
            .map(|item| match item {
                SortStateContent::Condition(condition) => {
                    RawNode::Element(condition.as_raw_element())
                }
                SortStateContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for SortState {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}

// -----------------------------------------------------------------------------------------------
// `x:autoFilter` — the cluster's root
// -----------------------------------------------------------------------------------------------

/// `x:autoFilter` (`CT_AutoFilter`, `sml.xsd:16`) — the filtered range, its per-column filters and
/// the sort state over it.
///
/// **`ST_`/`CT_` symbol:** `CT_AutoFilter`. Wire element: `autoFilter`, rank **10** of
/// `CT_Worksheet` — and also a direct child of `CT_Table`, which MJXOFF-125 (D15) reaches through
/// this same type.
///
/// `@ref` is the whole filtered range, **header row included**: Excel writes `A1:D20` for a table
/// whose headings are in row 1. It is `use="optional"` in the schema, and an `autoFilter` with no
/// `@ref` is what a worksheet-scoped `_FilterDatabase` defined name supplies the range for.
///
/// A `filterColumn`'s `@colId` is an offset **into this `@ref`**; see [`FilterColumn`].
#[derive(Debug, Clone, PartialEq, Eq, mjx_derive::FromXml, mjx_derive::XmlAttributes)]
#[xml(namespace = SML)]
#[xml(attribute(local = "ref", codec = Enumeration<CellRange>, accessor = range))]
pub struct AutoFilter {
    name: RawName,
    attributes: Vec<RawAttribute>,
    empty: bool,
    #[xml(
        children,
        child(local = "filterColumn", variant = Column, ty = FilterColumn),
        child(local = "sortState", variant = SortState, ty = SortState)
    )]
    content: Vec<AutoFilterContent>,
}

/// One child of [`AutoFilter`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutoFilterContent {
    /// `x:filterColumn` (rank 0) — one filtered column. `maxOccurs="unbounded"`.
    Column(FilterColumn),
    /// `x:sortState` (rank 1) — the sort last performed over the range.
    SortState(SortState),
    /// `x:extLst` (rank 2) — and anything else, preserved verbatim and in position.
    Raw(RawNode),
}

impl AutoFilterContent {
    /// This child's wire local name, or `None` for an unmodelled node.
    fn local(&self) -> Option<&'static str> {
        Some(match self {
            Self::Column(_) => "filterColumn",
            Self::SortState(_) => "sortState",
            Self::Raw(_) => return None,
        })
    }

    /// This child's rank in `CT_AutoFilter`'s `xsd:sequence`, from the generated table.
    fn rank(&self) -> Option<u16> {
        AUTO_FILTER.rank_of(None, self.local()?)
    }
}

impl AutoFilter {
    /// Builds an empty `x:autoFilter`, bound to `prefix` or to the default namespace.
    #[must_use]
    pub fn new(interner: &mut Interner, prefix: Option<&str>) -> Self {
        Self {
            name: crate::leaf::sml_name(interner, prefix, "autoFilter"),
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

    /// Every child, in document order, including `extLst` and anything else unmodelled.
    #[must_use]
    pub fn content(&self) -> &[AutoFilterContent] {
        &self.content
    }

    /// Every `x:filterColumn`, in document order.
    pub fn columns(&self) -> impl Iterator<Item = &FilterColumn> + '_ {
        self.content.iter().filter_map(|item| match item {
            AutoFilterContent::Column(column) => Some(column),
            _ => None,
        })
    }

    /// How many columns of the range carry a filter.
    #[must_use]
    pub fn column_count(&self) -> usize {
        self.columns().count()
    }

    /// The `index`-th `x:filterColumn`, mutably.
    pub fn column_mut(&mut self, index: usize) -> Option<&mut FilterColumn> {
        self.content
            .iter_mut()
            .filter_map(|item| match item {
                AutoFilterContent::Column(column) => Some(column),
                _ => None,
            })
            .nth(index)
    }

    /// Appends a filtered column after the ones already present, and before `sortState`/`extLst`.
    ///
    /// Columns are **not** sorted by `@colId`: the file's order is the file's, and a producer is free
    /// to write them in any order the schema allows.
    pub fn push_column(&mut self, column: FilterColumn) {
        let at = AUTO_FILTER.insert_index_of_names(
            self.content.iter().map(AutoFilterContent::rank),
            "filterColumn",
        );
        self.content.insert(at, AutoFilterContent::Column(column));
        self.empty = false;
    }

    /// Removes the `index`-th `x:filterColumn` and returns it, or `None` when there are fewer.
    pub fn remove_column(&mut self, index: usize) -> Option<FilterColumn> {
        let at = self
            .content
            .iter()
            .enumerate()
            .filter(|(_, item)| matches!(item, AutoFilterContent::Column(_)))
            .map(|(at, _)| at)
            .nth(index)?;
        match self.content.remove(at) {
            AutoFilterContent::Column(column) => Some(column),
            _ => unreachable!("the position was filtered on `Column`"),
        }
    }

    /// `x:sortState` — the sort last performed over the range, or `None`.
    #[must_use]
    pub fn sort_state(&self) -> Option<&SortState> {
        self.content.iter().find_map(|item| match item {
            AutoFilterContent::SortState(state) => Some(state),
            _ => None,
        })
    }

    /// `x:sortState`, mutably.
    pub fn sort_state_mut(&mut self) -> Option<&mut SortState> {
        self.content.iter_mut().find_map(|item| match item {
            AutoFilterContent::SortState(state) => Some(state),
            _ => None,
        })
    }

    /// Sets `x:sortState`: `None` removes it; `Some` replaces the existing element **where it is**,
    /// or inserts one at rank 1 — after the filtered columns, before `extLst`.
    pub fn set_sort_state(&mut self, state: Option<SortState>) {
        let existing = self
            .content
            .iter()
            .position(|item| matches!(item, AutoFilterContent::SortState(_)));
        match (existing, state) {
            (Some(at), Some(state)) => self.content[at] = AutoFilterContent::SortState(state),
            (Some(at), None) => {
                self.content.remove(at);
            }
            (None, Some(state)) => {
                let at = AUTO_FILTER.insert_index_of_names(
                    self.content.iter().map(AutoFilterContent::rank),
                    "sortState",
                );
                self.content.insert(at, AutoFilterContent::SortState(state));
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
            .map(|item| match item {
                AutoFilterContent::Column(column) => RawNode::Element(column.as_raw_element()),
                AutoFilterContent::SortState(state) => RawNode::Element(state.as_raw_element()),
                AutoFilterContent::Raw(node) => node.clone(),
            })
            .collect();
        rebuild_element(self.name, &self.attributes, children, self.empty)
    }
}

impl ToXml for AutoFilter {
    fn to_xml(&self, _interner: &mut Interner) -> RawElement {
        self.as_raw_element()
    }
}
