//! Cell addressing: cell references, ranges, `sqref` lists, and the A1 and R1C1 grammars.
//!
//! **Filled by MJXOFF-93 (D03).** Nothing here yet — this child (MJXOFF-132) creates the crate and
//! the tree, and models nothing.
//!
//! What belongs here: the parsed forms of `ST_CellRef` (`B7`, `$B$7`), `ST_Ref`/`ST_Sqref`
//! (`A1:C3`, and the space-separated lists a conditional format or a data validation carries), the
//! sheet-qualified reference a formula names (`Sheet1!$A$2:$A$4`), the absolute/relative flags, and
//! the R1C1 spelling of all of them. A reference is a *value*, not a string: the addressing type is
//! what lets [`crate::cells`] index a sheet and [`crate::formula`] rewrite a range without
//! re-parsing text.
