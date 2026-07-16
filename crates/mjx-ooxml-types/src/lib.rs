//! `mjx-ooxml-types` — comprehensively-named OOXML simple types and namespace constants.
//!
//! Most of this crate is **generated** by `xtask` from the ECMA-376 XSD schemas (see the naming
//! convention in `PLAN.md`): every cryptic `ST_*` symbol becomes a self-explanatory Rust name, each
//! type carries wire (de)serialization, and the original symbol + wire token are documented on the
//! item. Regenerate with `cargo run -p xtask -- codegen`.
//!
//! Two-valued OOXML toggles (`ST_OnOff` family) are modeled as `bool` / `Option<bool>`; all wire
//! spellings are normalized on read and one canonical form is written — see [`support`].
//!
//! # Example
//!
//! ```
//! use mjx_ooxml_types::shared::CalendarType;
//! assert_eq!(CalendarType::from_wire("gregorianUs"), Some(CalendarType::GregorianUnitedStates));
//! assert_eq!(CalendarType::GregorianUnitedStates.to_wire(), "gregorianUs");
//! ```

pub mod drawingml;
pub mod support;

mod generated;

pub use generated::{namespaces, shared};
pub use support::{on_off, true_false, true_false_blank, UnknownWireValue};
