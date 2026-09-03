//! `mjx-ooxml-types` — comprehensively-named OOXML simple types and namespace constants.
//!
//! Most of this crate is **generated** by `xtask` from the ECMA-376 XSD schemas (see the naming
//! convention in `PLAN.md`): every cryptic `ST_*` symbol becomes a self-explanatory Rust name, each
//! type carries wire (de)serialization, and the original symbol + wire token are documented on the
//! item. Regenerate with `cargo run -p xtask -- codegen`.
//!
//! Child order — where a child element belongs among its siblings, so a serializer cannot write an
//! `xsd:sequence` out of order — is in [`child_order`], generated from the same schemas.
//!
//! Two-valued OOXML toggles (`ST_OnOff` family) are modeled as `bool` / `Option<bool>`; all wire
//! spellings are normalized on read and one canonical form is written — see [`support`].
//!
//! `shared`, [`wordprocessingml`], [`spreadsheetml`], [`officemath`] and [`diagram`] are complete —
//! every `ST_*` their schema declares. `drawingml` and `presentationml` are curated slices that grow
//! with their workstreams; `crates/mjx-ooxml-types/COVERAGE.md` reports every schema of the set and
//! the status of each.
//!
//! [`support`] also holds the OOXML-specific [`AttributeCodec`](mjx_ooxml_core::AttributeCodec)s —
//! [`OnOff`], [`TrueFalse`], [`TrueFalseBlank`], [`HexColorRgb`] — that carry those simple types
//! across the attribute seam. An enumeration needs no codec of its own: every generated one is
//! `Enumeration<T>`, because they all spell themselves with `FromStr` and `Display`.
//!
//! # Example
//!
//! ```
//! use mjx_ooxml_types::shared::CalendarType;
//! assert_eq!(CalendarType::from_wire("gregorianUs"), Some(CalendarType::GregorianUnitedStates));
//! assert_eq!(CalendarType::GregorianUnitedStates.to_wire(), "gregorianUs");
//! ```

pub mod child_order;
pub mod drawingml;
pub mod presentationml;
pub mod support;

mod generated;

pub use generated::{diagram, namespaces, officemath, shared, spreadsheetml, wordprocessingml};
pub use support::{
    on_off, true_false, true_false_blank, HexColorRgb, OnOff, TrueFalse, TrueFalseBlank,
    UnknownWireValue,
};
