//! `mjx-sml` — the SpreadsheetML **markup** model: cells, rows, sheet data, shared strings, styles,
//! number formats and formulas-as-text.
//!
//! # Why SpreadsheetML is shared markup, not a format
//!
//! A `.xlsx` package is Excel's, but SpreadsheetML is not. An authored PowerPoint chart embeds a
//! whole workbook at `/ppt/embeddings/*.xlsx` — that package is what **Edit Data** opens — and a
//! Word document that carries a chart does the same. `mjx-chart` therefore needed a SpreadsheetML
//! writer before Excel existed, and wrote a minimal one of its own
//! ([`mjx_chart::EmbeddedWorkbook`](https://docs.rs/mjx-chart)) with a note naming its executioner.
//!
//! That deletion was **illegal as specified**: `mjx-chart` sits in the shared-markup tier, so an
//! edge from it to `mjx-xlsx` (a format) would point *upward*, and `mjx-pptx → mjx-xlsx` would point
//! *sideways*. Both are forbidden by the layering rule in `CLAUDE.md`. The layer was not missing by
//! accident — it was missing because nobody had split Excel the way DrawingML was already split:
//!
//! * **`mjx-sml`** (here) — the `sml.xsd` *markup*: what a cell, a row, a shared string, an `xf` or
//!   a number format **is**. Shared-markup tier, beside `mjx-dml`.
//! * **`mjx-xlsx`** — the `Workbook` surface, the package and part graph, `open`/`save`/`blank`,
//!   relationships. Format tier, beside `mjx-pptx` and `mjx-docx`.
//!
//! With the split, `mjx-chart → mjx-sml → mjx-dml` is a chain of downward edges and
//! `EmbeddedWorkbook` can finally be deleted (MJXOFF-112, then MJXOFF-99).
//!
//! # Where this crate sits, exactly
//!
//! Shared markup is not flat. This crate is **rank 2.1**: below `mjx-chart`, `mjx-omml` and
//! `mjx-vml` (2.2), above `mjx-dml` (2.0), and far below the format crates (3.0).
//!
//! | Rank | Crates |
//! |---|---|
//! | 0.0 — foundations | `mjx-ooxml-core`, `mjx-xml`, `mjx-derive` |
//! | 1.0 — packaging / compatibility | `mjx-ooxml-types`, `mjx-opc`, `mjx-mce` |
//! | 2.0 — shared markup, base | `mjx-dml` |
//! | **2.1 — shared markup, spreadsheet** | **`mjx-sml`** |
//! | 2.2 — shared markup, upper | `mjx-chart`, `mjx-omml`, `mjx-vml` |
//! | 3.0 — formats | `mjx-pptx`, `mjx-docx`, `mjx-xlsx` |
//! | 4.0 — facade | `mjx-ooxml` |
//! | 5.0 — bindings | `bindings/mjx-python`, `bindings/mjx-wasm` |
//!
//! An edge is legal **iff** it points to a strictly lower rank. So `mjx-chart → mjx-sml` (2.2 → 2.1)
//! and `mjx-sml → mjx-dml` (2.1 → 2.0) are both legal, while `mjx-dml → mjx-sml` and
//! `mjx-sml → mjx-chart` are not, and the graph stays acyclic. This is not a comment anyone has to
//! trust: `xtask/tests/layering.rs` reads the real dependency graph out of `cargo metadata` and
//! fails, naming both crates and both ranks, on any edge that does not point down.
//!
//! # Status — the crate spine, plus the addressing vocabulary
//!
//! MJXOFF-132 creates the crate, the module tree, the `sml` child-order table and the layering test;
//! MJXOFF-93 fills [`address`], which is markup vocabulary rather than package structure and which
//! eleven later children consume. Everything else below is still a named home with the work item
//! that fills it, so that no later child has to invent a place to put its model.
//!
//! | Module | Filled by |
//! |---|---|
//! | [`address`] | **MJXOFF-93 (D03) — done**: references, ranges, `sqref`, `spans`, A1 and R1C1 |
//! | [`cells`] | MJXOFF-95 (D04) — the cell store, the hybrid memory model |
//! | [`strings`] | MJXOFF-97 (D05) — `sharedStrings.xml`, rich-text runs, inline strings |
//! | [`styles`] | MJXOFF-105 (D08), MJXOFF-108 (D09) — resource tables, then the `xf` indirection |
//! | [`formula`] | MJXOFF-115 (D11) — formulas as text, cached values, `calcChain` |
//! | [`worksheet`] | MJXOFF-102 (D07) the 39-slot spine, MJXOFF-117 (D12) the sheet grid |
//! | [`workbook`] | MJXOFF-100 (D06) — the sheet list, properties, views, defined names |
//! | [`features`] | MJXOFF-120/123/125/127/129 (D13–D17) — the optional worksheet features |
//! | [`mod@write`] | MJXOFF-112 (D10) — the package writer that replaces `EmbeddedWorkbook` |
//! | [`error`] | MJXOFF-132 (D01) — this child; every later one adds its variants |
//!
//! The half of `sml.xsd` this workspace deliberately does **not** model — pivot tables, external
//! links, metadata, connections and revisions — is MJXOFF-133's (D18) to write down. Everything
//! unmodelled is still preserved, by the unknown bucket and by `mjx-opc`'s copy-on-write, exactly as
//! it is for every other schema here.
//!
//! # Ordering
//!
//! `sml` is in `xtask`'s `CHILD_ORDER_SCHEMAS` as of this child, so
//! [`mjx_ooxml_types::child_order`] carries the `xsd:sequence` position of every child of all 367
//! SpreadsheetML complex types — `CT_Worksheet`'s 39 slots among them, the largest sequence in the
//! workspace. Every writer added here places children through that table rather than by hand, and
//! `mjx-schema-gate` audits every `x:`-rooted part of every package it inspects.

pub mod address;
pub mod cells;
pub mod error;
pub mod features;
pub mod formula;
pub mod strings;
pub mod styles;
pub mod workbook;
pub mod worksheet;
pub mod write;

pub use address::{
    AddressError, AddressText, Anchoring, CellRange, CellRangeList, CellReference, CellSpan,
    CellSpans, ColumnBound, GridBounds, R1C1Coordinate, R1C1Range, R1C1Reference, ReferenceMode,
    RowBound, SheetName, SheetQualifiedReference,
};
pub use error::SmlError;
