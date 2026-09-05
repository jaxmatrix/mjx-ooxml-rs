//! Formulas, as text — and the guarantee that nothing here ever acts on one.
//!
//! # The position, stated before the model
//!
//! `PLAN.md` and MJXOFF-21 record it as settled scope rather than as an omission: **formulas are
//! carried as the text their producer wrote, there is no calculation engine, and there will not be
//! one.** That makes this module unusual. Its job is to *model* a thing and to *guarantee the
//! library never acts on it*, and the second half is the one that matters:
//!
//! * An edit that changes a cell a formula depends on leaves that formula's cached value **stale**,
//!   and that is correct behaviour here. Nothing in this workspace blanks a `<v>`, marks a workbook
//!   dirty for calculation, or recomputes anything. Excel recalculates on open when it needs to.
//! * A formula's text is never reformatted, re-derived, or translated between `A1` and `R1C1`. There
//!   is no expression tree, no tokeniser and no dependency graph — [`CellFormula`] is a *reader* over
//!   the bytes the file wrote and has no way to produce different ones.
//! * `calcChain.xml` is derived data Excel owns. This workspace **leaves it exactly as it found it**;
//!   see [`CalculationChain`] for the policy and for why the alternative — deleting it — was
//!   rejected.
//!
//! A "helpful" invalidation is the most damaging thing this library could do: it destroys data in a
//! file the caller opened to change a label, silently, in a part they never named. So the stale
//! cached value is not a gap that some later child closes. It is the contract, it is written down in
//! `crates/mjx-xlsx/docs/guide/formulas_and_cached_values.md` and in
//! `docs/fidelity_and_gaps.md`, and `crates/mjx-sml/tests/formulas.rs` fails if it ever stops
//! being true.
//!
//! # Where a formula lives, and what it costs
//!
//! Nowhere new. MJXOFF-95's cell store already keeps a cell's `<f …>…</f>` as a byte range
//! (`CellExtras::formula`), because a formula has to come back byte for byte whatever else happens
//! to the cell; this child gives those bytes a **type** rather than a second home.
//! [`CellFormula::parse`] decomposes them on demand — one bounded scan of a start tag, no allocation
//! — and every accessor reads out of the same range the writer will copy. So:
//!
//! * **This child adds zero bytes per cell.** `PackedCell` is still 36 bytes and `CellExtras` still
//!   40; `crates/mjx-sml/src/cells/record.rs` asserts both, and
//!   `crates/mjx-sml/tests/cell_store_allocation.rs` case 5 measures a 300,000-cell sheet in which
//!   *every* cell carries a formula.
//! * **A shared-group member costs what any formula cell costs and nothing for being in a group.**
//!   There is no formula table, no per-cell index into one, and above all no copy of the host's text
//!   per member — [`SharedFormulaGroups`] is built on demand by the caller that wants it and is not
//!   part of the store.
//! * The typed view cannot drift from the bytes, because there is only one copy of the formula and
//!   the accessors read it.
//!
//! # `t` absent and `t="normal"` are the same meaning and different bytes
//!
//! `CT_CellFormula`'s `t` is declared `use="optional" default="normal"` (`sml.xsd:2751`). So a file
//! that wrote nothing and a file that wrote `t="normal"` *mean* the same thing — and a file that
//! said nothing must come back saying nothing. [`CellFormula::kind`] applies the default and
//! [`CellFormula::written_kind`] does not; [`CellFormula::has_written_kind`] is the question the
//! round trip actually turns on.
//!
//! This is MJXOFF-95's situation for `c@t` one element down, where it is solved by storing the type
//! "as one byte with a distinct code for *the attribute was absent*". Here the distinction costs
//! nothing at all to keep, because the attribute run is never decomposed in the first place. It is
//! the mirror image of `CT_Xf`'s six `applyX` attributes (MJXOFF-108), which have **no** declared
//! default and where absent is therefore a third state with a meaning of its own.
//!
//! # What is modelled
//!
//! | Type | Schema | Line |
//! |---|---|---|
//! | [`CellFormula`] | `CT_CellFormula`, a `simpleContent` extension of `ST_Formula` with twelve attributes | `sml.xsd:2751` |
//! | [`FormulaKind`] | `ST_CellFormulaType` — `normal`, `array`, `dataTable`, `shared` | `sml.xsd:2298` |
//! | [`CachedValue`] | the `<v>` beside an `<f>`, read through the `c@t` that says what it means | `sml.xsd:1683` (`CT_Cell`) |
//! | [`SharedFormulaGroups`] | not a type — the `@si` grouping across one sheet, indexed on demand | — |
//! | [`CalculationChain`] / [`CalculationChainCell`] | `CT_CalcChain` / `CT_CalcCell` | `sml.xsd:257` / `263` |
//!
//! # Not here
//!
//! `CT_TableFormula` inside a `tableN.xml` is MJXOFF-127's (D15), on the same no-evaluation terms.
//! A defined name's definition is formula text too and lives in
//! [`DefinedName`](crate::DefinedName), which states the identical position. Volatile dependencies
//! (`volTypes`) are MJXOFF-133's (D18).

// The subject modules are public, as `crate::styles`' are and for the same reason: each carries the
// design record for its own piece — why `CellFormula` is a view rather than a struct, why a shared
// group's text distribution is data rather than an encoding, why `calcChain.xml` is left alone — and
// a reader who reaches one of those types through its re-export should be able to reach the
// reasoning behind it too.
pub mod cached;
pub mod calc_chain;
pub mod cell;
pub mod shared;

pub use cached::CachedValue;
pub use calc_chain::{
    CalculationChain, CalculationChainCell, CalculationChainContent, ResolvedCalculationChainCell,
};
pub use cell::CellFormula;
pub use shared::{SharedFormulaGroup, SharedFormulaGroups};

/// `ST_CellFormulaType` (`sml.xsd:2298`) — which of the four things an `<f>` is.
///
/// This is [`mjx_ooxml_types::spreadsheetml::CellFormulaType`] under the name the ticket asks for,
/// **not a second enumeration**. The four variants and their wire tokens are generated from the
/// schema by `xtask codegen`, so `normal`, `array`, `dataTable` and `shared` cannot drift from
/// `sml.xsd` by anyone editing this crate; a hand-written copy beside the generated one is the shape
/// this workspace has already deleted fourteen of.
pub use mjx_ooxml_types::spreadsheetml::CellFormulaType as FormulaKind;
