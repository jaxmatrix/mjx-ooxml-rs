//! `x:calcPr` (`CT_CalcPr`, `sml.xsd:4284`) — what a **calculation engine** was told, by a producer
//! that had one.
//!
//! # Reported, never acted on
//!
//! There is no calculation engine in this workspace and there will not be one. Every attribute here
//! is a statement the file makes about how *some other* application recalculates, and this crate's
//! whole job is to hand it back unchanged:
//!
//! * `@calcId` is the build number of the engine that last calculated. Excel compares it against its
//!   own and recalculates if they differ. Nothing here derives it, and nothing here bumps it — a
//!   library that changed `calcId` because it had edited a cell would be asserting that it had
//!   recalculated, which it has not.
//! * `@calcCompleted`, `@fullCalcOnLoad` and `@forceFullCalc` are that engine's own record of
//!   whether its last pass finished and what the next one must do. They are hints of exactly the
//!   kind [`SharedStringTable`](crate::SharedStringTable)'s `count`/`uniqueCount` are: read, never
//!   recomputed.
//! * `@iterate`, `@iterateCount` and `@iterateDelta` configure circular-reference iteration, which
//!   presupposes evaluation.
//!
//! # `@refMode` is one of two reference-syntax signals, and not the only one
//!
//! `@refMode` (`ST_RefMode`) selects A1 or R1C1 syntax for the whole workbook, and it is the
//! [`ReferenceMode`] D03's address vocabulary is parameterised on.
//!
//! It is worth knowing that it is not the only place a file can say this. `tests/fixtures/sample.xlsx`
//! writes `refMode="A1"` **and** a LibreOffice extension —
//! `<ext uri="{7626C862-…}"><loext:extCalcPr stringRefSyntax="CalcA1"/></ext>` — under
//! `CT_Workbook`'s own `extLst`. The two are different producers' mechanisms and they agree in that
//! file, but a consumer that read only `@refMode` would be ignoring what the file also says, and one
//! that "normalised" the extension away would destroy a producer's own setting. The extension is
//! therefore preserved verbatim through [`WorkbookContent::Raw`](super::WorkbookContent::Raw),
//! prefix and all, and this type does not pretend to summarise it.

use mjx_ooxml_core::{Enumeration, Number};
use mjx_ooxml_types::spreadsheetml::CalculationMode;
use mjx_ooxml_types::support::OnOff;

use crate::address::ReferenceMode;

use super::leaf::attribute_bag;

attribute_bag! {
    /// `x:calcPr` (`CT_CalcPr`) — the calculation settings, reported exactly as the file states
    /// them. See [`crate::workbook`]'s own documentation for why nothing here is ever acted on.
    ///
    /// `sample.xlsx` writes `<calcPr iterateCount="100" refMode="A1" iterate="false"
    /// iterateDelta="0.001"/>` — four attributes, in an order that is **not** the schema's
    /// declaration order, which is one more reason the attribute vector is preserved rather than
    /// rebuilt.
    #[xml(attribute(local = "calcId", codec = Number<u32>, accessor = calculation_engine_id))]
    #[xml(attribute(local = "calcMode", codec = Enumeration<CalculationMode>, accessor = calculation_mode, default = CalculationMode::Auto))]
    #[xml(attribute(local = "fullCalcOnLoad", codec = OnOff, accessor = full_calculation_on_load, default = false))]
    #[xml(attribute(local = "refMode", codec = Enumeration<ReferenceMode>, accessor = reference_mode, default = ReferenceMode::A1))]
    #[xml(attribute(local = "iterate", codec = OnOff, accessor = iterate_on_circular_references, default = false))]
    #[xml(attribute(local = "iterateCount", codec = Number<u32>, accessor = iteration_limit, default = 100))]
    #[xml(attribute(local = "iterateDelta", codec = Number<f64>, accessor = iteration_convergence_delta, default = 0.001))]
    #[xml(attribute(local = "fullPrecision", codec = OnOff, accessor = full_precision, default = true))]
    #[xml(attribute(local = "calcCompleted", codec = OnOff, accessor = calculation_completed, default = true))]
    #[xml(attribute(local = "calcOnSave", codec = OnOff, accessor = calculate_on_save, default = true))]
    #[xml(attribute(local = "concurrentCalc", codec = OnOff, accessor = concurrent_calculation, default = true))]
    #[xml(attribute(local = "concurrentManualCount", codec = Number<u32>, accessor = concurrent_thread_count))]
    #[xml(attribute(local = "forceFullCalc", codec = OnOff, accessor = force_full_calculation))]
    CalculationProperties, "calcPr"
}
