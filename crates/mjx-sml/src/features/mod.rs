//! The optional worksheet features — everything a sheet may carry beside its cells.
//!
//! **Filled by MJXOFF-120 (D13) conditional formatting, MJXOFF-123 (D14) data validation,
//! autofilters and sort state, and MJXOFF-125 (D15) worksheet tables; MJXOFF-127 (D16) hyperlinks
//! and the object-anchor vocabulary, MJXOFF-129 (D17) print setup, headers/footers and custom views
//! fill the rest.**
//!
//! These are separated from [`crate::worksheet`] deliberately. The spine is what every worksheet
//! has; a feature is what some worksheets have, each with its own vocabulary of a dozen or more
//! complex types, and each landing in a different `CT_Worksheet` slot. Keeping them apart is what
//! stops five children from editing one file.
//!
//! # The module tree, and the child that fills each file
//!
//! | Module | Subject | Filled by |
//! |---|---|---|
//! | `conditional_rules.rs` | `conditionalFormatting`, `cfRule`, and a rule's `formula` | MJXOFF-120 (D13) |
//! | `conditional_scales.rs` | `cfvo`, `colorScale`, `dataBar`, `iconSet` | MJXOFF-120 (D13) |
//! | `conditional_chain.rs` | the cross-block priority order, and the `dxf` layer beside a cell's base format | MJXOFF-120 (D13) |
//! | `conditional_specs.rs` | the plain-data authoring vocabulary, and appending a `dxf` | MJXOFF-120 (D13) |
//! | `filters.rs` | `autoFilter`, `filterColumn`'s six filter kinds, `sortState` — **the cluster `CT_Table` and the pivot types reuse** | MJXOFF-123 (D14) |
//! | `filter_specs.rs` | the plain-data authoring vocabulary for those | MJXOFF-123 (D14) |
//! | `validation.rs` | `dataValidations`, `dataValidation`, and its authoring vocabulary | MJXOFF-123 (D14) |
//! | `tables.rs` | `CT_Table` and everything under it, plus the `tableParts` list — **a part of its own** | MJXOFF-125 (D15) |
//! | `table_specs.rs` | the plain-data authoring vocabulary for a whole table | MJXOFF-125 (D15) |
//!
//! # Conditional formatting reports; it never evaluates
//!
//! The rule stated once, here, because it is the thing most likely to be assumed the other way:
//! **this crate reports which rules apply to a cell and what formatting each would impose. It never
//! decides whether a rule's condition is true.** Doing that needs a calculation engine — the same
//! one MJXOFF-115 says will not exist — so a caller holding a chain of three rules holds three
//! candidates, in the order a consumer would consider them, and not an answer about how the cell
//! looks.
//!
//! That is why a cell's conditional layer is reported **alongside** its
//! [`EffectiveCellFormat`](crate::EffectiveCellFormat) and never folded into it: folding would be
//! claiming a rule fired.
//!
//! # A filter never hides, a sort never reorders, a validation never rejects
//!
//! The same rule at the other three doors MJXOFF-123 opened, stated once here because it is the
//! thing most likely to be assumed the other way at each of them:
//!
//! * an `x:autoFilter` is a **record of a filter**. Reading or writing one sets no row's `@hidden`
//!   — a hidden row is MJXOFF-117's row property and is preserved exactly as read;
//! * an `x:sortState` is a **record of a sort**. Nothing reorders a row;
//! * an `x:dataValidation` is a **record of a constraint**. Nothing compares a cell against one, and
//!   a `list` rule's `formula1` — a range reference as often as a literal list — is text that is
//!   never resolved into the values it names.
//!
//! Each is spelled out where it lives: [`filters`] for the first two, [`validation`] for the third.
//!
//! # A table is a part, and a calculated column is never expanded
//!
//! MJXOFF-125's own two, for the same reason. `x:table` lives in `xl/tables/tableN.xml` rather than
//! in the worksheet, and the `tablePart@r:id` that reaches it is held here as **text**: resolving it
//! to a part is `mjx-xlsx`'s. A table's `calculatedColumnFormula` is the expression Excel fills a
//! whole column with, and this library neither expands it into per-cell formulas nor evaluates it —
//! nor computes a totals row, nor parses the structured reference (`Sales[[#This Row],[Q1]]`) inside
//! either. See [`tables`].

// The subject modules are public, as [`crate::formula`]'s and [`crate::styles`]' are and for the
// same reason: each carries the design record for its own piece — why conditional formatting is
// reported and never evaluated, why a filter hides no row, why a `list` validation's source is text
// — and a reader who reaches one of these types through its re-export should be able to reach the
// reasoning behind it too.
pub mod conditional_chain;
pub mod conditional_rules;
pub mod conditional_scales;
pub mod conditional_specs;
pub mod filter_specs;
pub mod filters;
pub mod table_specs;
pub mod tables;
pub mod validation;

pub use conditional_chain::{
    AppliedConditionalRule, ConditionalCellFormat, ConditionalFormatLayer, ConditionalRuleChain,
};
pub use conditional_rules::{
    ConditionalFormatting, ConditionalFormattingContent, ConditionalFormattingRule,
    ConditionalFormattingRuleContent,
};
pub use conditional_scales::{
    ColorScale, ColorScaleContent, ConditionalValueObject, DataBar, DataBarContent, IconSet,
    IconSetContent,
};
pub use conditional_specs::{
    ColorScaleSpec, ConditionalRuleSpec, ConditionalRuleSpecKind, ConditionalValueObjectSpec,
    DataBarSpec, DifferentialFormatSpec, IconSetSpec,
};
pub use filter_specs::{
    AutoFilterSpec, CustomFilterSpec, FilterColumnSpec, FilterSpecKind, SortConditionSpec,
    SortStateSpec,
};
pub use filters::{
    AutoFilter, AutoFilterContent, ColorFilter, CustomFilter, CustomFilters, CustomFiltersContent,
    DateGroupItem, DynamicFilter, Filter, FilterColumn, FilterKind, Filters, FiltersContent,
    IconFilter, SortCondition, SortState, SortStateContent, Top10Filter,
};
pub use table_specs::{TableColumnSpec, TableStyleReferenceSpec, WorksheetTableSpec};
pub use tables::{
    TableColumn, TableColumnContent, TableColumns, TableColumnsContent, TableFormula, TablePart,
    TableParts, TablePartsContent, TableStyleReference, WorksheetTable, WorksheetTableContent,
    XmlColumnProperties,
};
pub use validation::{
    DataValidation, DataValidationContent, DataValidationSpec, DataValidations,
    DataValidationsContent,
};
