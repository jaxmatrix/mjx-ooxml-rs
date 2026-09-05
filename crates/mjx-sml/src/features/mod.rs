//! The optional worksheet features — everything a sheet may carry beside its cells.
//!
//! **Filled by MJXOFF-120 (D13) conditional formatting, MJXOFF-123 (D14) data validation,
//! autofilters and sort state, MJXOFF-125 (D15) worksheet tables, MJXOFF-127 (D16) hyperlinks and
//! the object-anchor vocabulary, MJXOFF-129 (D17) print setup, headers/footers and custom views.**
//! Nothing here yet — this child (MJXOFF-132) creates the crate and the tree, and models nothing.
//!
//! These are separated from [`crate::worksheet`] deliberately. The spine is what every worksheet
//! has; a feature is what some worksheets have, each with its own vocabulary of a dozen or more
//! complex types, and each landing in a different `CT_Worksheet` slot. Keeping them apart is what
//! stops five children from editing one file.
