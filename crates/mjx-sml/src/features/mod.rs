//! The optional worksheet features — everything a sheet may carry beside its cells.
//!
//! **Filled by MJXOFF-120 (D13) conditional formatting, MJXOFF-123 (D14) data validation,
//! autofilters and sort state, MJXOFF-125 (D15) worksheet tables, MJXOFF-127 (D16) hyperlinks and
//! the object-anchor vocabulary, MJXOFF-129 (D17) print setup, headers/footers and custom views.**
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

mod conditional_chain;
mod conditional_rules;
mod conditional_scales;
mod conditional_specs;

pub use conditional_chain::{
    AppliedConditionalRule, ConditionalCellFormat, ConditionalFormatLayer, ConditionalRuleChain,
};
pub use conditional_rules::{
    ConditionalFormatting, ConditionalFormattingContent, ConditionalFormattingFormula,
    ConditionalFormattingRule, ConditionalFormattingRuleContent,
};
pub use conditional_scales::{
    ColorScale, ColorScaleContent, ConditionalValueObject, DataBar, DataBarContent, IconSet,
    IconSetContent,
};
pub use conditional_specs::{
    ColorScaleSpec, ConditionalRuleSpec, ConditionalRuleSpecKind, ConditionalValueObjectSpec,
    DataBarSpec, DifferentialFormatSpec, IconSetSpec,
};
