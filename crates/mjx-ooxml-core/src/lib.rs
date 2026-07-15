//! `mjx-ooxml-core` — shared foundations for the mjx-ooxml-rs workspace.
//!
//! Currently provides:
//! - [`Interner`] — a pure-Rust string interner for hot repeated names (one per parsed part).
//! - the **raw preservation tree** ([`RawDocument`] / [`RawNode`] / [`RawElement`]) — a lossless DOM
//!   that reproduces a part's source bytes and doubles as the "unknown content bucket" for the
//!   future typed model.
//!
//! The arena + stable-handle primitives and the `FromXml`/`ToXml` traits land in later phases, when
//! the typed model needs them (see `PLAN.md`).

pub mod intern;
pub mod raw;

pub use intern::{Interner, Symbol};
pub use raw::{QuoteStyle, RawAttribute, RawDocument, RawElement, RawName, RawNode};
