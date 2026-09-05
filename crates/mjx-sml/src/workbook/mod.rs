//! The workbook part: the sheet list, workbook properties, views, protection and defined names.
//!
//! **Filled by MJXOFF-100 (D06).** Nothing here yet — this child (MJXOFF-132) creates the crate and
//! the tree, and models nothing.
//!
//! What belongs here: `CT_Workbook`'s nineteen children (the generated
//! [`WORKBOOK`](mjx_ooxml_types::child_order::WORKBOOK) order) — `fileVersion`, `workbookPr` and its
//! date system, `bookViews`, the `sheets` list that names every worksheet by relationship id,
//! `definedNames`, `calcPr`.
//!
//! The **package** graph those relationship ids resolve against is not here: it is `mjx-xlsx`'s, at
//! MJXOFF-91 (D02). This module models the markup of the part; the format crate owns the part graph.
//! That division is the whole reason the two crates exist.
