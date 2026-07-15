//! `mjx-mce` — Markup Compatibility & Extensibility (ECMA-376 Part 3).
//!
//! OOXML producers embed forward-compatibility markup (`mc:AlternateContent`, `mc:Ignorable`, …) so
//! that newer features degrade gracefully in older consumers. This crate operates on the
//! [`mjx_ooxml_core::RawDocument`] tree in two modes:
//!
//! - **Preserve** (the default, and what round-tripping does): the stored tree already contains every
//!   `mc:*` node and attribute verbatim, so serializing it re-emits the compatibility markup intact.
//!   There is nothing to call — preservation *is* the untouched tree.
//! - **Resolve**: given the namespaces a consumer understands, [`resolve`] returns a flattened,
//!   **non-mutating** view with the winning `Choice`/`Fallback` selected and ignorable content
//!   applied — without changing the source tree (so a later serialize is still byte-identical).
//!
//! MCE appears in no OOXML XSD, so its namespace constant lives here.

mod resolve;
mod scope;

/// The Markup Compatibility namespace URI (ECMA-376 Part 3).
pub const MARKUP_COMPATIBILITY_2006: &str =
    "http://schemas.openxmlformats.org/markup-compatibility/2006";

pub use resolve::{resolve, ResolveError, ResolvedElement, ResolvedNode, UnderstoodNamespaces};
pub use scope::NamespaceScope;
