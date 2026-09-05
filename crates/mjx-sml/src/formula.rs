//! Formulas, as text.
//!
//! **Filled by MJXOFF-115 (D11).** Nothing here yet — this child (MJXOFF-132) creates the crate and
//! the tree, and models nothing.
//!
//! What belongs here: `CT_CellFormula` and its `ST_CellFormulaType` kinds — normal, `shared` (the
//! master cell holds the text and a `si`, the followers hold only the index), `array` (the formula
//! governs a range) and `dataTable` — together with the cached value beside it and the `calcChain`
//! part's evaluation order.
//!
//! **This crate does not evaluate formulas and is not scheduled to.** A formula is preserved as the
//! text its producer wrote, which is what fidelity requires: re-writing a formula's text, or
//! recomputing a cached value, would change bytes for a part the library was not asked to edit.
