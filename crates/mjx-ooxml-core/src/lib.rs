//! `mjx-ooxml-core` — shared foundations for the mjx-ooxml-rs workspace.
//!
//! This is the bottom layer of the workspace: it has **no dependencies**, and every other crate
//! builds on it. It provides two things today.
//!
//! # String interning
//!
//! OOXML repeats the same namespace URIs and element/attribute names thousands of times per part.
//! [`Interner`] stores each distinct string once and hands out a 4-byte [`Symbol`], so equality is an
//! integer compare and memory stays flat. One interner is created per parsed part (it lives on
//! [`RawDocument`]).
//!
//! # The raw preservation tree
//!
//! [`RawDocument`] → [`RawNode`] → [`RawElement`] is a **lossless DOM**. Names are interned
//! [`Symbol`]s; attribute values and text are stored as *raw, escaped bytes* exactly as they appeared
//! in the source. That is what lets the `mjx-xml` fidelity writer reproduce a part's bytes exactly.
//! A `Vec<RawNode>` also serves as the "unknown content bucket" the future typed model carries so it
//! can round-trip anything it does not itself model.
//!
//! Trees are normally produced by the `mjx-xml` fidelity reader rather than built by hand.
//!
//! Later phases add the arena + stable-handle primitives and the `FromXml`/`ToXml` traits, when the
//! typed model needs them (see `PLAN.md`).
//!
//! # Example
//!
//! ```
//! use mjx_ooxml_core::Interner;
//!
//! let mut interner = Interner::new();
//! let a = interner.intern("w:val");
//! let b = interner.intern("w:val"); // same string → same symbol, no second allocation
//! assert_eq!(a, b);
//! assert_eq!(interner.resolve(a), "w:val");
//! ```

pub mod intern;
pub mod raw;

pub use intern::{Interner, Symbol};
pub use raw::{QuoteStyle, RawAttribute, RawDocument, RawElement, RawName, RawNode};
