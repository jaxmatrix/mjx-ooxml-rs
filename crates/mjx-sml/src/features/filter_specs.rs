//! Plain-data descriptions of the autofilters and sort states a caller can author.
//!
//! # Why a description and not the model
//!
//! The same reason [`crate::features::conditional_specs`] gives: [`AutoFilter`] is *markup*. It keeps
//! the [`RawName`](mjx_ooxml_core::RawName) it was read with, and every name in it is a symbol
//! interned in the document the part was parsed from — so constructing one needs that exact
//! [`Interner`], which a caller of a package-tier `set_auto_filter` does not hold and should not
//! have to. The authoring vocabulary is therefore these five plain structs: public fields, no
//! interner, no lifetime, and one `build` method each that turns a description into markup *inside*
//! the part that will hold it.
//!
//! MJXOFF-97 set this precedent with [`RichTextRunSpec`](crate::RichTextRunSpec), MJXOFF-105 with
//! [`PatternFillSpec`](crate::PatternFillSpec) and MJXOFF-120 with
//! [`ConditionalRuleSpec`](crate::ConditionalRuleSpec).
//!
//! # All six filter kinds are describable, and that is deliberate
//!
//! [`ConditionalRuleSpecKind`](crate::ConditionalRuleSpecKind) carries five of `ST_CfType`'s
//! eighteen members because the other thirteen need attributes a caller would have to supply
//! separately, and a variant that wrote knowingly-incomplete markup is worse than no variant. That
//! test comes out the other way here: every one of `CT_FilterColumn`'s six kinds is **completely**
//! stated by what [`FilterSpecKind`] carries, `colorFilter@dxfId` included — a caller allocates that
//! index with
//! [`StylesheetPart::append_differential_format`](crate::StylesheetPart::append_differential_format),
//! which appends, exactly as they would for a conditional rule.
//!
//! # Authoring a filter still filters nothing
//!
//! [`AutoFilterSpec::build`] writes an `x:autoFilter`. It does not read a single cell, and it sets
//! no row's `@hidden`; [`SortStateSpec::build`] writes an `x:sortState` and moves no row. That is
//! [`crate::features::filters`]' rule, restated at the authoring tier because this is the tier where
//! being helpful would be easiest.

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::spreadsheetml::{DynamicFilterType, FilterOperator, IconSetType, SortBy};

use crate::address::CellRange;

use super::filters::{
    AutoFilter, ColorFilter, CustomFilter, CustomFilters, Filter, FilterColumn, FilterKind,
    Filters, IconFilter, SortCondition, SortState, Top10Filter,
};

/// One comparison of a `customFilters` pair: an operator and the text it compares against.
///
/// The text is an `ST_Xstring` and is written exactly as given — a wildcard `Sm?th*` stays a
/// wildcard, and a number stays the digits the caller typed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomFilterSpec {
    /// `@operator`.
    pub operator: FilterOperator,
    /// `@val`.
    pub value: String,
}

impl CustomFilterSpec {
    /// A comparison of `operator` against `value`.
    #[must_use]
    pub fn new(operator: FilterOperator, value: impl Into<String>) -> Self {
        Self {
            operator,
            value: value.into(),
        }
    }
}

/// Which of `CT_FilterColumn`'s six filter kinds a column carries.
///
/// The seventh member of the schema's choice is `extLst`, which is an extension slot rather than a
/// filter and has no place in an authoring vocabulary: a caller who needs one builds the
/// [`FilterColumn`] directly and puts a [`FilterKind::Raw`] in it.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterSpecKind {
    /// `x:filters` — the values the user ticked, and whether *(Blanks)* is among them.
    Values {
        /// One `x:filter` per entry, in this order. Each is written as text.
        values: Vec<String>,
        /// `@blank` — *(Blanks)*. Not expressible as a value, because an empty `@val` is the empty
        /// string rather than the absence of one.
        includes_blanks: bool,
    },
    /// `x:top10` — the top or bottom N, or N %.
    Top10 {
        /// `@val` — the count or percentage.
        count: f64,
        /// `@top` — `true` for the top N, `false` for the bottom.
        takes_the_top: bool,
        /// `@percent`.
        is_percentage: bool,
    },
    /// `x:customFilters` — one or two comparisons.
    Custom {
        /// The comparisons, in order. The schema allows one or two.
        comparisons: Vec<CustomFilterSpec>,
        /// `@and` — `true` joins the pair with *And*, `false` (the default) with *Or*.
        requires_both: bool,
    },
    /// `x:dynamicFilter` — a relative period or an average.
    ///
    /// `@val`/`@maxVal` are deliberately not here: they are the bounds **Excel derives** when it
    /// applies the filter, and writing an invented pair would be this library claiming to have
    /// evaluated a filter it never evaluates.
    Dynamic(DynamicFilterType),
    /// `x:colorFilter` — by cell or font colour, named by `dxf` index.
    Color {
        /// `@dxfId` — a **position** in `xl/styles.xml`'s `dxfs` table, allocated by appending.
        differential_format_index: u32,
        /// `@cellColor` — the fill (`true`) or the font (`false`).
        is_cell_color: bool,
    },
    /// `x:iconFilter` — by conditional-formatting icon.
    Icon {
        /// `@iconSet` — `use="required"`.
        icon_set: IconSetType,
        /// `@iconId` — the zero-based icon within the set, or `None` to filter on the whole set.
        icon_index: Option<u32>,
    },
}

impl FilterSpecKind {
    /// The value filter a caller almost always wants: these values, no *(Blanks)*.
    #[must_use]
    pub fn values<I, S>(values: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self::Values {
            values: values.into_iter().map(Into::into).collect(),
            includes_blanks: false,
        }
    }

    /// Builds the filter element this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> FilterKind {
        match self {
            Self::Values {
                values,
                includes_blanks,
            } => {
                let mut filters = Filters::new(interner, prefix);
                for value in values {
                    let mut entry = Filter::new(interner, prefix);
                    entry.set_value(interner, Some(value.as_str()));
                    filters.push_value(entry);
                }
                if *includes_blanks {
                    filters.set_includes_blanks(interner, Some(true));
                }
                FilterKind::Values(filters)
            }
            Self::Top10 {
                count,
                takes_the_top,
                is_percentage,
            } => {
                let mut filter = Top10Filter::new(interner, prefix);
                filter.set_value(interner, *count);
                if !*takes_the_top {
                    filter.set_takes_the_top(interner, Some(false));
                }
                if *is_percentage {
                    filter.set_is_percentage(interner, Some(true));
                }
                FilterKind::Top10(filter)
            }
            Self::Custom {
                comparisons,
                requires_both,
            } => {
                let mut filters = CustomFilters::new(interner, prefix);
                if *requires_both {
                    filters.set_requires_both(interner, Some(true));
                }
                for comparison in comparisons {
                    let mut filter = CustomFilter::new(interner, prefix);
                    filter.set_operator(interner, Some(comparison.operator));
                    filter.set_value(interner, Some(comparison.value.as_str()));
                    filters.push(filter);
                }
                FilterKind::Custom(filters)
            }
            Self::Dynamic(kind) => {
                let mut filter = super::filters::DynamicFilter::new(interner, prefix);
                filter.set_kind(interner, *kind);
                FilterKind::Dynamic(filter)
            }
            Self::Color {
                differential_format_index,
                is_cell_color,
            } => {
                let mut filter = ColorFilter::new(interner, prefix);
                filter.set_differential_format_index(interner, Some(*differential_format_index));
                if !*is_cell_color {
                    filter.set_is_cell_color(interner, Some(false));
                }
                FilterKind::Color(filter)
            }
            Self::Icon {
                icon_set,
                icon_index,
            } => {
                let mut filter = IconFilter::new(interner, prefix);
                filter.set_icon_set(interner, *icon_set);
                if let Some(index) = icon_index {
                    filter.set_icon_index(interner, Some(*index));
                }
                FilterKind::Icon(filter)
            }
        }
    }
}

/// One filtered column to author: which column of the autofilter's range, and which filter.
#[derive(Debug, Clone, PartialEq)]
pub struct FilterColumnSpec {
    /// `@colId` — the **zero-based offset within the autofilter's own `@ref`**, not a worksheet
    /// column index. See [`FilterColumn`]'s own documentation.
    pub column_offset: u32,
    /// The filter itself.
    pub kind: FilterSpecKind,
}

impl FilterColumnSpec {
    /// A filtered column at `column_offset` carrying `kind`.
    #[must_use]
    pub fn new(column_offset: u32, kind: FilterSpecKind) -> Self {
        Self {
            column_offset,
            kind,
        }
    }

    /// Builds the `x:filterColumn` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> FilterColumn {
        let mut column = FilterColumn::new(interner, prefix);
        column.set_column_offset(interner, self.column_offset);
        column.set_filter(Some(self.kind.build(interner, prefix)));
        column
    }
}

/// One key of a sort to author.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortConditionSpec {
    /// `@ref` — the one column this key sorts on. `use="required"`.
    pub range: CellRange,
    /// `@descending`.
    pub is_descending: bool,
    /// `@sortBy` — value, cell colour, font colour or icon.
    pub sort_by: SortBy,
    /// `@customList` — the comma-separated user order, when there is one.
    pub custom_list: Option<String>,
}

impl SortConditionSpec {
    /// An ascending sort on the values of `range`.
    #[must_use]
    pub fn ascending(range: CellRange) -> Self {
        Self {
            range,
            is_descending: false,
            sort_by: SortBy::Value,
            custom_list: None,
        }
    }

    /// A descending sort on the values of `range`.
    #[must_use]
    pub fn descending(range: CellRange) -> Self {
        Self {
            range,
            is_descending: true,
            sort_by: SortBy::Value,
            custom_list: None,
        }
    }

    /// Builds the `x:sortCondition` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> SortCondition {
        // The attributes are set in `CT_SortCondition`'s own declaration order, so an authored
        // element reads the way the schema lists it. XML does not order attributes, but a file this
        // library writes should still look like one a producer wrote.
        let mut condition = SortCondition::new(interner, prefix);
        if self.is_descending {
            condition.set_is_descending(interner, Some(true));
        }
        if self.sort_by != SortBy::Value {
            condition.set_sort_by(interner, Some(self.sort_by));
        }
        condition.set_range(interner, self.range);
        if let Some(list) = &self.custom_list {
            condition.set_custom_list(interner, Some(list.as_str()));
        }
        condition
    }
}

/// A sort state to author: the sorted range, and its keys in significance order.
///
/// **Writing one sorts nothing.** It records that a sort happened; see
/// [`crate::features::filters`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortStateSpec {
    /// `@ref` — the sorted range. `use="required"`.
    pub range: CellRange,
    /// The conditions, most significant first.
    pub conditions: Vec<SortConditionSpec>,
    /// `@caseSensitive`.
    pub is_case_sensitive: bool,
    /// `@columnSort` — the sort ran left-to-right rather than top-to-bottom.
    pub sorts_columns: bool,
}

impl SortStateSpec {
    /// A top-to-bottom, case-insensitive sort of `range` on `conditions`.
    #[must_use]
    pub fn new(range: CellRange, conditions: Vec<SortConditionSpec>) -> Self {
        Self {
            range,
            conditions,
            is_case_sensitive: false,
            sorts_columns: false,
        }
    }

    /// Builds the `x:sortState` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> SortState {
        // `CT_SortState`'s declaration order: `columnSort`, `caseSensitive`, `sortMethod`, `ref`.
        let mut state = SortState::new(interner, prefix);
        if self.sorts_columns {
            state.set_sorts_columns(interner, Some(true));
        }
        if self.is_case_sensitive {
            state.set_is_case_sensitive(interner, Some(true));
        }
        state.set_range(interner, self.range);
        for condition in &self.conditions {
            state.push_condition(condition.build(interner, prefix));
        }
        state
    }
}

/// An autofilter to author: the filtered range, its per-column filters, and an optional sort state.
#[derive(Debug, Clone, PartialEq)]
pub struct AutoFilterSpec {
    /// `@ref` — the filtered range, **header row included**. `use="optional"`, so `None` is legal
    /// and means the range comes from the `_FilterDatabase` defined name.
    pub range: Option<CellRange>,
    /// The filtered columns, in the order they should be written.
    pub columns: Vec<FilterColumnSpec>,
    /// `x:sortState` — the sort last performed over the range, if any.
    pub sort_state: Option<SortStateSpec>,
}

impl AutoFilterSpec {
    /// An autofilter over `range` with the drop-downs shown and nothing filtered yet.
    #[must_use]
    pub fn over(range: CellRange) -> Self {
        Self {
            range: Some(range),
            columns: Vec::new(),
            sort_state: None,
        }
    }

    /// Adds a filtered column, in builder style.
    #[must_use]
    pub fn with_column(mut self, column: FilterColumnSpec) -> Self {
        self.columns.push(column);
        self
    }

    /// Sets the sort state, in builder style.
    #[must_use]
    pub fn with_sort_state(mut self, state: SortStateSpec) -> Self {
        self.sort_state = Some(state);
        self
    }

    /// Builds the `x:autoFilter` this describes, interning its names into `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> AutoFilter {
        let mut filter = AutoFilter::new(interner, prefix);
        if let Some(range) = self.range {
            filter.set_range(interner, Some(range));
        }
        for column in &self.columns {
            filter.push_column(column.build(interner, prefix));
        }
        if let Some(state) = &self.sort_state {
            filter.set_sort_state(Some(state.build(interner, prefix)));
        }
        filter
    }
}
