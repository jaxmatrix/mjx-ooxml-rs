//! `x:workbookPr` and `x:calcPr`, as a caller holding a [`Workbook`] sees them — what the workbook
//! *is*, rather than what it contains.
//!
//! # Why these are owned snapshots rather than borrows of the model
//!
//! [`mjx_sml::WorkbookProperties`] and [`mjx_sml::CalculationProperties`] are the markup, and every
//! one of their accessors needs the part's [`Interner`](mjx_ooxml_core::Interner) to resolve a name
//! — which means a caller reaching them has to hold the parsed part open. That is the right shape
//! for an editor and the wrong one for the two questions a caller of this crate almost always has:
//! *which date epoch do these serial numbers count from*, and *what did the producer say about
//! recalculation*.
//!
//! So [`Workbook::date_system`] and [`Workbook::calculation_settings`] answer those with small
//! `Copy` values decoded once. A caller that wants the other sixteen `workbookPr` attributes, or
//! wants to change one, goes through [`Workbook::workbook_markup`] /
//! [`Workbook::edit_workbook_markup`] and the model itself — this file adds a shortcut, not a second
//! model.
//!
//! # Nothing here is applied
//!
//! [`DateSystem`] is reported. This library does no date arithmetic: a cell's value is the number
//! the file wrote, and turning that into a calendar date is the caller's to do, with the epoch this
//! type hands them. [`CalculationSettings`] is reported for the same reason one layer down — there
//! is no calculation engine here, and `calcId` is never bumped and `calcCompleted` never cleared
//! because this crate has not recalculated anything.

use mjx_ooxml_types::spreadsheetml::CalculationMode;
use mjx_sml::ReferenceMode;

use crate::error::XlsxError;

use super::Workbook;

/// Which epoch a date-serial number in this workbook counts from — `workbookPr/@date1904`.
///
/// The difference is 1,462 days, so reading a workbook with the wrong system shifts every date in
/// it by just over four years. That is why this is a named two-valued type rather than a `bool`
/// somewhere: a caller cannot use it without having noticed which one it has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateSystem {
    /// The 1900 system: serial 1 is 1900-01-01, and the epoch a serial counts from is 1899-12-30.
    /// The schema default, and what `date1904="false"` — or an absent `workbookPr` — means.
    Windows1900,
    /// The 1904 system: serial 0 is 1904-01-01. What `date1904="true"` means, and what Excel for
    /// Macintosh wrote for many years.
    Macintosh1904,
}

impl DateSystem {
    /// Whether this is the 1904 system.
    #[must_use]
    pub fn is_1904(self) -> bool {
        self == Self::Macintosh1904
    }
}

/// `x:calcPr`, decoded — what the producer's calculation engine was told.
///
/// **Reported, never acted on.** Every field is a statement the file makes about how some *other*
/// application recalculates; see [`mjx_sml::CalculationProperties`] for the full attribute set and
/// for why nothing here derives one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalculationSettings {
    /// `@calcId` — the build number of the engine that last calculated, or `None` if the file names
    /// none. Never derived and never bumped.
    pub engine_id: Option<u32>,
    /// `@calcMode` — automatic, automatic-except-tables, or manual.
    pub mode: CalculationMode,
    /// `@refMode` — whether this workbook's formulas are written in A1 or R1C1 syntax. See
    /// [`mjx_sml::CalculationProperties`] on why a file can also say this through a preserved
    /// producer extension, which this field does not summarise.
    pub reference_mode: ReferenceMode,
    /// `@iterate` — whether circular references are iterated rather than reported.
    pub iterates_on_circular_references: bool,
    /// `@iterateCount` — the iteration limit.
    pub iteration_limit: u32,
    /// `@iterateDelta` — the convergence threshold.
    pub iteration_convergence_delta: f64,
    /// `@fullCalcOnLoad` — whether a consumer must recalculate everything on open.
    pub full_calculation_on_load: bool,
}

impl Default for CalculationSettings {
    /// The settings a workbook that writes no `calcPr` at all has: every schema default.
    fn default() -> Self {
        Self {
            engine_id: None,
            mode: CalculationMode::Auto,
            reference_mode: ReferenceMode::A1,
            iterates_on_circular_references: false,
            iteration_limit: 100,
            iteration_convergence_delta: 0.001,
            full_calculation_on_load: false,
        }
    }
}

impl Workbook {
    /// Which epoch this workbook's date serials count from.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the workbook part cannot be read, or if `@date1904` holds a value
    /// that is not an `xsd:boolean` — which is reported rather than guessed at.
    pub fn date_system(&mut self) -> Result<DateSystem, XlsxError> {
        let uses_1904 = self.workbook_markup(|part, interner| {
            part.properties()
                .map_or(Ok(false), |properties| {
                    properties.uses_1904_date_system(interner)
                })
                .map_err(XlsxError::from)
        })??;
        Ok(if uses_1904 {
            DateSystem::Macintosh1904
        } else {
            DateSystem::Windows1900
        })
    }

    /// What the producer said about recalculation — the schema defaults for a workbook that wrote no
    /// `calcPr`.
    ///
    /// # Errors
    /// Returns [`XlsxError`] if the workbook part cannot be read, or if one of the attributes holds
    /// a value its declared type rejects.
    pub fn calculation_settings(&mut self) -> Result<CalculationSettings, XlsxError> {
        self.workbook_markup(
            |part, interner| -> Result<CalculationSettings, mjx_ooxml_core::AttributeError> {
                let Some(calc) = part.calculation_properties() else {
                    return Ok(CalculationSettings::default());
                };
                Ok(CalculationSettings {
                    engine_id: calc.calculation_engine_id(interner)?,
                    mode: calc.calculation_mode(interner)?,
                    reference_mode: calc.reference_mode(interner)?,
                    iterates_on_circular_references: calc
                        .iterate_on_circular_references(interner)?,
                    iteration_limit: calc.iteration_limit(interner)?,
                    iteration_convergence_delta: calc.iteration_convergence_delta(interner)?,
                    full_calculation_on_load: calc.full_calculation_on_load(interner)?,
                })
            },
        )?
        .map_err(XlsxError::from)
    }
}
