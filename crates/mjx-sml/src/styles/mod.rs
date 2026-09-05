//! `styles.xml`: the resource tables, the `xf` indirection and number formats.
//!
//! **Filled by MJXOFF-105 (D08), then MJXOFF-108 (D09).** Nothing here yet — this child
//! (MJXOFF-132) creates the crate and the tree, and models nothing.
//!
//! What belongs here: `CT_Stylesheet`'s eleven children (the generated
//! [`STYLESHEET`](mjx_ooxml_types::child_order::STYLESHEET) order) split across the two children
//! that own them — MJXOFF-105 takes the resource tables (`fonts`, `fills`, `borders`, `dxfs` and the
//! indexed-colour legacy), MJXOFF-108 takes the `cellStyleXfs`/`cellXfs` indirection, `numFmts` and
//! the effective-formatting ladder that resolves a cell's `@s` through both.
//!
//! It is a **directory** because those are two independent vocabularies that happen to share a part:
//! a resource table is a list of values, and an `xf` is a pointer into four of them with its own
//! per-field "apply" flags. `mjx-pptx`'s `presentation.rs` reached 12,771 lines before MJXOFF-60
//! (A8) split it into subject modules; nothing here is allowed to start down that road.
