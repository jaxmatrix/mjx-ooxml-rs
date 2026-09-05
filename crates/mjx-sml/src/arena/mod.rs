//! The byte space every packed SpreadsheetML store shares, and the element split it is built from.
//!
//! # Why this is a module of its own rather than a file inside [`cells`](crate::cells)
//!
//! MJXOFF-95 built the cell store on one idea: **every value a store preserves is a byte range into
//! one address space**, half of which is the part's own buffer (shared, never copied) and half of
//! which is whatever the store has authored. `docs/BENCHMARKS.md` is why — a 300,000-cell worksheet
//! held as a [`RawElement`](mjx_ooxml_core::RawElement) tree costs ≈ 913 bytes of peak resident set
//! per cell, and the cost is the two small heap allocations every element carries, not the element
//! struct.
//!
//! `PLAN.md` line 26 names **two** bulk-data cases, not one: *"arena/columnar for bulk data (e.g.
//! spreadsheet cells, shared strings)"*. MJXOFF-97 built the second, and it needs exactly the same
//! two primitives the first does — the arena, and the "split one element's bytes into its attribute
//! run and its content" scan that decides whether a range can be trusted. Two copies of either would
//! be two copies of the invariant that a range is a *claim about somebody else's buffer* and has to
//! be re-checked before it is believed.
//!
//! So they live here, below both stores, and neither store reaches into the other's internals.
//!
//! # What is here
//!
//! * [`TextSpan`] and [`TextArena`] — the address space. See [`text`] for the layout, the sentinel
//!   that distinguishes *absent* from *present and empty*, and the four-gigabyte bound.
//! * [`layout_in_arena`] — one element's own bytes split into its attribute run and its content,
//!   in arena addresses, and the checks that make a range refuse rather than hand back somebody
//!   else's bytes. [`decompose`](decompose::decompose) is the offset-level primitive behind it.
//! * [`span_between`] / [`span_present_between`] — the two ways an empty range is read, which are
//!   not the same thing: the whitespace between two rows being empty means *there is none*, and a
//!   `<v></v>` being empty means *there is a value and it is the empty string*.

pub(crate) mod decompose;
pub(crate) mod text;

pub(crate) use decompose::{layout_in_arena, span_between, span_present_between, ElementLayout};
pub(crate) use text::{TextArena, TextSpan};
