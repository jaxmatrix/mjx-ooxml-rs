//! The plain-data authoring vocabulary for worksheet tables: what a caller describes, and where it
//! is turned into markup.
//!
//! # Why a description rather than the markup types
//!
//! [`WorksheetTable`] keeps the [`RawName`](mjx_ooxml_core::RawName) it was read with, and every name
//! in it is a symbol interned in the document that part was parsed from. Constructing one therefore
//! needs that exact [`Interner`], which a caller of a package-tier `add_table` does not hold and
//! should not have to. So the authoring vocabulary is these three structs: public fields, no
//! interner, no lifetime, one `build` method each that turns a description into markup **inside** the
//! part that will hold it.
//!
//! MJXOFF-105 set this precedent with [`PatternFillSpec`](crate::write::PatternFillSpec), MJXOFF-120
//! with [`ConditionalRuleSpec`](crate::ConditionalRuleSpec) and MJXOFF-123 with
//! [`AutoFilterSpec`](crate::AutoFilterSpec), which this file reuses rather than restates.
//!
//! # What a spec deliberately does not decide
//!
//! * **The table's `@id`.** It is workbook-unique (§18.5.1.2: *"Each table in the workbook shall have
//!   a unique id"*), and only something holding the whole package can know which ids are taken.
//!   [`WorksheetTableSpec::id`] is therefore a field the caller states, and
//!   [`Workbook::add_table`](https://docs.rs/mjx-xlsx) is what fills it in from the package. Nothing
//!   here derives one, and **nothing anywhere renumbers an existing table.**
//! * **Whether the range fits.** [`WorksheetTableSpec::build`] refuses a header/totals combination
//!   the range cannot hold, exactly as [`WorksheetTable::resize`] does, because writing it would
//!   author a table whose own totals row falls outside itself.
//! * **What the cells say.** Building a table writes `xl/tables/tableN.xml` and touches no cell. A
//!   table's `@ref` is a claim about a region of the sheet; making the headings in row 1 match
//!   [`TableColumnSpec::name`] is the caller's, and doing it silently would write over values nobody
//!   asked to lose.

use mjx_ooxml_core::Interner;
use mjx_ooxml_types::spreadsheetml::TotalsRowFunction;

use crate::address::CellRange;
use crate::error::SmlError;

use super::filter_specs::{AutoFilterSpec, SortStateSpec};
use super::tables::{TableColumn, TableColumns, TableFormula, TableStyleReference, WorksheetTable};

/// One `x:tableStyleInfo` to author: the style's name, and which parts of the table wear it.
///
/// Every flag is an `Option<bool>` because `CT_TableStyleInfo` gives none of the four a schema
/// default: `None` writes no attribute at all, which is a different statement from writing `"0"`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TableStyleReferenceSpec {
    /// `@name` — the style to wear. Usually one of ECMA-376's 144 preset names
    /// (`TableStyleMedium2`), which **no `.xlsx` contains**; see
    /// [`TableStyles::lookup`](crate::TableStyles::lookup). `None` writes no `@name`, which asks for
    /// no style at all.
    pub name: Option<String>,
    /// `@showFirstColumn`.
    pub show_first_column: Option<bool>,
    /// `@showLastColumn`.
    pub show_last_column: Option<bool>,
    /// `@showRowStripes`.
    pub show_row_stripes: Option<bool>,
    /// `@showColumnStripes`.
    pub show_column_stripes: Option<bool>,
}

impl TableStyleReferenceSpec {
    /// A style reference naming `name`, with every banding flag left unstated.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Self::default()
        }
    }

    /// Builds the `x:tableStyleInfo` element, interning its names in `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> TableStyleReference {
        let mut style = TableStyleReference::new(interner, prefix);
        style.set_name(interner, self.name.as_deref());
        style.set_shows_first_column(interner, self.show_first_column);
        style.set_shows_last_column(interner, self.show_last_column);
        style.set_shows_row_stripes(interner, self.show_row_stripes);
        style.set_shows_column_stripes(interner, self.show_column_stripes);
        style
    }
}

/// One `x:tableColumn` to author.
///
/// `@id` and `@name` are both `use="required"`, so both are plain fields rather than options. A
/// column's `@id` is unique **within its table** — unlike the table's own `@id`, which is unique
/// across the workbook — and [`WorksheetTableSpec::build`] numbers them from 1 in order unless the
/// caller states otherwise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumnSpec {
    /// `@id`, unique within the table and counting from 1.
    pub id: u32,
    /// `@name` — the heading text, and what a structured reference (`Table1[Region]`) names. This
    /// library carries structured references inside formula text and **never parses one**, so
    /// renaming a column here does not rewrite any formula that mentions it.
    pub name: String,
    /// `@totalsRowFunction`. `None` writes no attribute, which is the schema default `none`.
    ///
    /// The variant names are **not** the wire tokens: `count` is
    /// [`TotalsRowFunction::CountNonEmpty`], `countNums` is `CountNumbers`, `stdDev` is
    /// `EstimatedStandardDeviation`, `var` is `EstimatedVariance`, `custom` is `CustomFormula`.
    pub totals_row_function: Option<TotalsRowFunction>,
    /// `@totalsRowLabel` — the text a totals cell shows instead of an aggregate, which is what the
    /// leftmost column of a totals row normally carries ("Total").
    pub totals_row_label: Option<String>,
    /// `x:calculatedColumnFormula` — the expression every data cell of this column carries.
    ///
    /// **Written as text and never expanded**: no per-cell `<f>` is authored anywhere in the sheet
    /// from this, and nothing evaluates it.
    pub calculated_column_formula: Option<String>,
    /// `x:totalsRowFormula` — the custom aggregation this column's totals cell uses. §18.5.1.6 pairs
    /// it with `totalsRowFunction="custom"`; stating one without the other is markup a producer is
    /// free to write and this library does not correct.
    pub totals_row_formula: Option<String>,
}

impl TableColumnSpec {
    /// A column numbered `id` and headed `name`, with no totals row and no formula.
    #[must_use]
    pub fn new(id: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            totals_row_function: None,
            totals_row_label: None,
            calculated_column_formula: None,
            totals_row_formula: None,
        }
    }

    /// Builds the `x:tableColumn` element, interning its names in `interner`.
    #[must_use]
    pub fn build(&self, interner: &mut Interner, prefix: Option<&str>) -> TableColumn {
        let mut column = TableColumn::new(interner, prefix);
        column.set_id(interner, self.id);
        column.set_name(interner, &self.name);
        column.set_totals_row_function(interner, self.totals_row_function);
        column.set_totals_row_label(interner, self.totals_row_label.as_deref());
        if let Some(text) = &self.calculated_column_formula {
            column.set_calculated_column_formula(Some(TableFormula::new(
                interner,
                prefix,
                "calculatedColumnFormula",
                text.clone(),
            )));
        }
        if let Some(text) = &self.totals_row_formula {
            column.set_totals_row_formula(Some(TableFormula::new(
                interner,
                prefix,
                "totalsRowFormula",
                text.clone(),
            )));
        }
        column
    }
}

/// One whole `x:table` to author — the contents of a new `xl/tables/tableN.xml`.
///
/// See this module's own documentation for the three things this deliberately does not decide.
// `PartialEq` but not `Eq`: `AutoFilterSpec` carries a `Top10` filter's `f64` bounds, and `f64` is
// not `Eq`. The bound stops here rather than being worked around, because a spec that compared two
// `NaN` thresholds as equal would be this crate deciding something the format does not.
#[derive(Debug, Clone, PartialEq)]
pub struct WorksheetTableSpec {
    /// `@id` — workbook-unique and **stated by the caller**, because only something holding the
    /// package knows which ids are taken. §18.5.1.2 requires it to be non-zero.
    pub id: u32,
    /// `@displayName` — what formulas reference the table by. §18.5.1.2: it *"shall not have any
    /// spaces in it, and it shall be unique amongst all other displayNames and definedNames in the
    /// workbook"*.
    pub display_name: String,
    /// `@name` — the programmatic name, unique per sheet. `None` writes no attribute; §18.5.1.2 says
    /// *"By default this should be the same as the table's displayName"*, and leaving it out is what
    /// says exactly that rather than restating it.
    pub name: Option<String>,
    /// `@ref` — the whole region the table occupies, **header and totals rows included**.
    pub range: CellRange,
    /// `@headerRowCount`. The schema default is 1, and this field is written whatever its value so
    /// that an authored table states its geometry rather than relying on a default a reader has to
    /// look up.
    pub header_row_count: u32,
    /// `@totalsRowCount`. The schema default is 0.
    pub totals_row_count: u32,
    /// The columns, left to right. `CT_TableColumns` declares `tableColumn` `minOccurs="1"`, so an
    /// empty list is refused by [`build`](Self::build).
    pub columns: Vec<TableColumnSpec>,
    /// `x:tableStyleInfo`, or `None` for a table that names no style.
    pub style: Option<TableStyleReferenceSpec>,
    /// `x:autoFilter` — the table's **own** filter, distinct from the sheet's. Recorded, never
    /// applied: writing one hides no row.
    pub auto_filter: Option<AutoFilterSpec>,
    /// `x:sortState` — the table's **own** recorded sort, distinct both from the sheet's and from
    /// the one the autofilter above may carry. Recorded, never performed.
    pub sort_state: Option<SortStateSpec>,
}

impl WorksheetTableSpec {
    /// A table over `range` called `display_name`, with one header row, no totals row, and one
    /// column per entry of `headings` numbered from 1.
    ///
    /// The `@id` is left at 0, which §18.5.1.2 forbids: it is
    /// [`Workbook::add_table`](https://docs.rs/mjx-xlsx)'s to fill in from the package, because only
    /// something holding the package knows which ids are taken.
    #[must_use]
    pub fn new(display_name: impl Into<String>, range: CellRange, headings: &[&str]) -> Self {
        Self {
            id: 0,
            display_name: display_name.into(),
            name: None,
            range,
            header_row_count: 1,
            totals_row_count: 0,
            columns: headings
                .iter()
                .enumerate()
                .map(|(offset, heading)| {
                    TableColumnSpec::new(u32::try_from(offset).unwrap_or(u32::MAX) + 1, *heading)
                })
                .collect(),
            style: None,
            auto_filter: None,
            sort_state: None,
        }
    }

    /// Builds the whole `x:table` element, interning its names in `interner`.
    ///
    /// The children are placed through
    /// [`WORKSHEET_TABLE`](mjx_ooxml_types::child_order::WORKSHEET_TABLE), so they come out in
    /// `CT_Table`'s own order whatever order they are set in here.
    ///
    /// # Errors
    /// [`SmlError::TableGeometryDoesNotFit`] when the header and totals rows do not fit inside
    /// `range`, and [`SmlError::TableHasNoColumns`] for an empty [`columns`](Self::columns) — the
    /// schema declares `tableColumn` `minOccurs="1"`, so a table with none is markup no consumer
    /// will load.
    pub fn build(
        &self,
        interner: &mut Interner,
        prefix: Option<&str>,
    ) -> Result<WorksheetTable, SmlError> {
        let mut table = WorksheetTable::new(interner, prefix);
        self.apply_to(&mut table, interner, prefix)?;
        Ok(table)
    }

    /// Writes everything this describes onto an **existing** `x:table` element.
    ///
    /// [`build`](Self::build) is this over a fresh element; [`AuthoredTable`](crate::write::AuthoredTable)
    /// is this over the one parsed out of its seed, which is what keeps the seed's namespace
    /// declaration — the rule `crates/mjx-xlsx/src/blank.rs` states, and the reason a whole part is
    /// never written from a freshly constructed root.
    ///
    /// Attributes and children the spec does not mention are left exactly as they stand.
    ///
    /// # Errors
    /// As [`build`](Self::build).
    pub fn apply_to(
        &self,
        table: &mut WorksheetTable,
        interner: &mut Interner,
        prefix: Option<&str>,
    ) -> Result<(), SmlError> {
        if self.columns.is_empty() {
            return Err(SmlError::TableHasNoColumns {
                display_name: self.display_name.clone(),
            });
        }
        table.set_id(interner, self.id);
        table.set_display_name(interner, &self.display_name);
        table.set_name(interner, self.name.as_deref());
        // `resize` is what writes `@ref` — there is no `set_range` — so the three attributes that
        // state the geometry are written together or not at all, and a combination that does not fit
        // is refused here rather than saved and left for a consumer to repair.
        table.resize(
            interner,
            self.range,
            self.header_row_count,
            self.totals_row_count,
        )?;

        let mut columns = TableColumns::new(interner, prefix);
        // The authored element declares `@count`, so every later `push` maintains it — the rule
        // `crates/mjx-sml/src/write/stylesheet.rs` states for the six tables it seeds.
        columns.set_declared_count(interner, Some(0));
        for spec in &self.columns {
            let column = spec.build(interner, prefix);
            columns.push(interner, column);
        }
        table.set_columns(Some(columns));

        if let Some(style) = &self.style {
            let style = style.build(interner, prefix);
            table.set_style(Some(style));
        }
        if let Some(filter) = &self.auto_filter {
            let filter = filter.build(interner, prefix);
            table.set_auto_filter(Some(filter));
        }
        if let Some(sort) = &self.sort_state {
            let sort = sort.build(interner, prefix);
            table.set_sort_state(Some(sort));
        }
        Ok(())
    }
}
