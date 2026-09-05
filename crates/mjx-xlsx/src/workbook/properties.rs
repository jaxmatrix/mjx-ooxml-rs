//! `x:workbookPr`, `x:workbookProtection`, `x:calcPr`, `x:fileVersion` — what the workbook is,
//! rather than what it contains.
//!
//! **Filled by MJXOFF-100 (D06).** Nothing here yet: MJXOFF-91 (D02) builds the package and the part
//! graph and models nothing at all.
//!
//! What belongs here: `CT_WorkbookPr` (the 1900/1904 date system, `showObjects`, the filter and
//! backup flags), `CT_WorkbookProtection`, `CT_CalcPr` (calculation mode, iteration, `calcId`) and
//! `CT_FileVersion`.
//!
//! Two of these are already load-bearing without being modelled. `tests/fixtures/sample.xlsx`'s
//! `x:workbookPr` carries LibreOffice's `dateCompatibility` attribute, which the Transitional
//! `sml.xsd` does not declare — a divergence the schema gate records as a *tolerated* deviation
//! rather than a failure, because it is an input this project preserves verbatim. Whatever models
//! `CT_WorkbookPr` must keep that attribute through a read and a write; see
//! `crates/mjx-schema-gate/src/tolerances.rs` for the record of it.
