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
//! # What MJXOFF-105 must **not** rewrite
//!
//! `CT_Font` — a font-table entry — is character for character the same fifteen font-property slots
//! as `CT_RPrElt`, a rich-text run's `rPr`, differing only in `rFont` vs `name` and in `family`'s
//! declared type. MJXOFF-97 modelled that family once, in [`crate::font`], deliberately outside both
//! subjects so that neither has to reach into the other:
//!
//! * [`FontProperties`](crate::FontProperties), with
//!   [`FontPropertyOwner::FontTableEntry`](crate::FontPropertyOwner::FontTableEntry) — read from a
//!   [`RawElement`](mjx_ooxml_core::RawElement) or from preserved bytes, and written back.
//! * [`Color`](crate::Color) — `CT_Color`, which every colour in this part uses: a font's, a pattern
//!   fill's `fgColor`/`bgColor`, a border's, a sheet's `tabColor`. It is **not** `mjx_dml::Color`,
//!   and `crates/mjx-sml/docs/SHARED_STRINGS.md` says why in full.
//!
//! **A slot a font-table entry needs and `FontProperties` does not carry is a slot to add there**, so
//! that both callers get it. Forking it would put a second copy of the `val`-wrapper family in the
//! workspace with nothing scheduled to remove it — the debt MJXOFF-99 exists to discharge once
//! already.
//!
//! It is a **directory** because those are two independent vocabularies that happen to share a part:
//! a resource table is a list of values, and an `xf` is a pointer into four of them with its own
//! per-field "apply" flags. `mjx-pptx`'s `presentation.rs` reached 12,771 lines before MJXOFF-60
//! (A8) split it into subject modules; nothing here is allowed to start down that road.
