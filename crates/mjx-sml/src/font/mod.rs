//! Font properties and colour — the vocabulary a rich-text run and a `styles.xml` font share.
//!
//! # Why this is its own module and not part of [`strings`](crate::strings)
//!
//! `sml.xsd` declares the same fifteen font-property children twice: once as `CT_RPrElt`, the
//! properties of a rich-text run inside a shared string or an inline string, and once as `CT_Font`,
//! one entry of `styles.xml`'s font table. The two differ in **two** places — the font-name element
//! is `rFont` in one and `name` in the other, and `family` is declared `CT_IntProperty` in one and
//! `CT_FontFamily` in the other — and are otherwise character for character the same content model
//! over the same eight `val`-wrapper types.
//!
//! MJXOFF-97 (D05) needs the run half and MJXOFF-105 (D08) needs the font-table half. Modelling it
//! inside `strings` would have made D08 either reach into another subject's module or copy it, and
//! **a copy would arrive with no executioner**: this workspace already has a scheduled child
//! (MJXOFF-99) whose whole job is to delete one duplicated SpreadsheetML writer, and a second
//! duplicate is not a thing to create knowingly. So the family sits in a module of its own that
//! belongs to neither subject and is below both.
//!
//! **MJXOFF-105 reaches for [`FontProperties`], [`FontPropertyOwner::FontTableEntry`] and
//! [`Color`].** If a font-table entry needs something this type does not carry, the fix is to grow
//! this type — a slot added here is a slot both callers get.
//!
//! # What is here
//!
//! * [`FontProperties`] — the fifteen slots, decoded, with [`FontPropertyOwner`] saying which of the
//!   two complex types they came from.
//! * [`Color`] — `CT_Color`, SpreadsheetML's five-attribute colour, which is not DrawingML's
//!   six-element one. [`color`] says why in full.
//! * [`ColorElement`] — the same complex type as an *element*, under whichever of its five local
//!   names (`color`, `fgColor`, `bgColor`, `tabColor`) the file wrote. Preservation; [`Color`] is
//!   interpretation.
//! * `font::value` (crate-private) — the `val`-wrapper family itself: eight complex types that are
//!   one shape, read and written once instead of eight times.

pub mod color;
pub mod properties;
pub(crate) mod value;

pub use color::{Color, ColorElement};
pub use properties::{FontProperties, FontPropertyOwner};
