//! `x:definedNames` — names a formula can use in place of a range, and the built-in ones a consumer
//! reads for its own purposes.
//!
//! **Filled by MJXOFF-100 (D06).** Nothing here yet: MJXOFF-91 (D02) builds the package and the part
//! graph and models nothing at all.
//!
//! What belongs here: `CT_DefinedNames`/`CT_DefinedName` — the name, its `localSheetId` scope, the
//! formula text it stands for, and the reserved names ECMA-376 Part 1 §18.2.5 lists
//! (`_xlnm.Print_Area`, `_xlnm.Print_Titles`, `_xlnm.Criteria`, …), whose meaning is the consumer's
//! rather than the author's.
//!
//! A defined name's value is a formula, so this module's content depends on MJXOFF-115 (D11)'s
//! formulas-as-text decision in `mjx_sml::formula`; it is listed under D06 because the *element*
//! belongs to the workbook part, not because the two can be written in either order.
