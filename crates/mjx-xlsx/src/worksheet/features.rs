//! The optional things a worksheet carries beside its cells — autofilters and tables, data
//! validation, conditional formatting, comments, hyperlinks, protection, form controls.
//!
//! **Filled by MJXOFF-120, MJXOFF-123, MJXOFF-125, MJXOFF-127 and MJXOFF-129 (D13-D17).** Nothing
//! here yet: MJXOFF-91 (D02) builds the package and the part graph and models nothing.
//!
//! What belongs here: the [`crate::Worksheet`] accessors for the features that reach *out of the
//! worksheet part into other parts* — a table definition part, a comments part and the legacy VML
//! drawing that draws its pop-up box, a query table, a pivot table. [`crate::WorksheetParts`]
//! already resolves every one of those relationships; this file is where they stop being part names
//! and start being features.
//!
//! The features that live entirely inside the worksheet's own markup (`CT_DataValidations`,
//! `CT_ConditionalFormatting`, `CT_SheetProtection`) are modelled in `mjx_sml::features`, and
//! reached from here.
