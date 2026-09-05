//! The cell store: `CT_SheetData`, `CT_Row` and `CT_Cell`, and the memory model behind them.
//!
//! **This is where `PLAN.md`'s hybrid model stops being a sentence.** Line 26 settles the in-memory
//! design as *"arena/columnar for bulk data (e.g. spreadsheet cells, shared strings), owned trees for
//! small structures"*, and until this module only the owned-tree half had ever been exercised —
//! PowerPoint and Word have no bulk-data case. A worksheet does: the cell count, not the element
//! count, decides whether the library is usable on a real workbook.
//!
//! # The number this was designed against
//!
//! `docs/BENCHMARKS.md` (MJXOFF-147) measures a **300,000-cell, 610,005-element worksheet** — 8.54
//! MiB of raw XML — costing **+274 MiB of peak resident set** to materialise as a
//! [`RawElement`](mjx_ooxml_core::RawElement) tree. That is **≈ 913 bytes per populated cell**, about
//! 32× the source bytes, and it puts a million-cell workbook at roughly a gigabyte. The measurement
//! also says *where* the cost is, which is the part that decided the design: not the 72-byte element
//! struct, but the two small heap allocations every element carries for its `children` and its
//! `attributes`, over 610,005 elements and about twice that many attributes.
//!
//! So the store has **no per-cell allocation at all**. It is:
//!
//! * one `Vec<PackedCell>` — **36 bytes** a cell, every field either an index or a byte range;
//! * one `Vec<PackedRow>` — 48 bytes a row, of which there can be at most 1,048,576;
//! * one side table for the rare cell that carries something unusual, so the common one pays four
//!   bytes for the index and nothing else;
//! * one byte arena, which for a worksheet nobody has edited is **the part's own buffer, shared with
//!   the package and never copied**.
//!
//! `cells/record.rs` carries the byte-by-byte accounting and the table of alternatives — an owned typed
//! tree, a `BTreeMap` keyed on the address, a dense grid over the addressable range — with what each
//! costs and why it lost.
//!
//! # Sparse means sparse
//!
//! A sheet whose only populated cell is `XFD1048576` holds **one** row record and **one** cell
//! record. It does not hold 1,048,576 row slots, and it does not reserve for the 17 billion cells the
//! grid can address. `crates/mjx-sml/tests/cell_store_allocation.rs` proves that with a counting
//! global allocator in a dedicated single-threaded binary, asserting a hard byte bound — not with
//! `size_of_val`, which reports the same twenty-four bytes for a `Vec` that reserved a million slots
//! as for an empty one and so passes against precisely the defect the gate exists to catch.
//!
//! # How a row nobody touched re-emits without being re-serialised
//!
//! This is the load-bearing decision of Phase D, so it is written down rather than implied.
//!
//! `RawElement` gives a *tree* subtree-level copy-on-write: an element remembers the byte range it
//! was parsed from, a serializer copies that range instead of descending into it, and the range is
//! dropped by any mutation — on the node and, because mutable descent goes through every ancestor's
//! child list, on the whole path to the root. The store is not a tree, so it cannot inherit that
//! mechanism. It **restates** it:
//!
//! * The sheet, every row and every cell each hold the byte range they were read from — three
//!   [`TextSpan`](crate::arena::TextSpan)s in the arena's address space, eight bytes each. The arena
//!   itself lives in [`crate::arena`], shared with the shared-string table, which is built on the
//!   same address space for the same reason.
//! * Writing asks the same question at each level, outermost first. A sheet with its range intact is
//!   one `memcpy` and the rows are never visited. A sheet with one edited cell copies every *other*
//!   row whole, and inside the edited row copies every other *cell* whole.
//! * Every edit goes through one function that clears the range on the record it touched, on that
//!   record's row, and on the sheet. What `DerefMut` enforces structurally for a tree, the store
//!   enforces by having exactly one door.
//!
//! The reader depends on the tree's invariant in one further place worth naming. A row's *gap* — the
//! newline or comment between it and the row before — is not read off the nodes; it is derived as
//! `[end of the previous row, start of this one)`. That is only sound because a `sheetData` element
//! that still has a range is one in which **every parsed descendant still has a range too**: an
//! authored or edited child would have cleared the ancestor's. Where the derivation is not available
//! the nodes are serialized into the arena instead, which reaches the same answer the slow way.
//!
//! # How the unknown bucket survives a packed store
//!
//! `CLAUDE.md` states the rule as *"every modeled complex type carries `extra: Vec<RawNode>` for
//! unknown children, and preserves unknown attributes, attribute order, and namespace prefixes."* A
//! `Vec<RawNode>` per cell is exactly the per-cell allocation the 913-byte measurement is made of, so
//! this store keeps the same rule in the only representation it can afford — and, as it turns out,
//! in a stricter one:
//!
//! * **Unknown children.** A cell's content is three byte runs — before the value, the value, after
//!   the value — and the first and last are replayed exactly. A `c/extLst` full of foreign markup, a
//!   `<f>` formula this child does not model, a comment between two cells: all come back byte for
//!   byte, in their original order, with their original prefixes.
//! * **Unknown attributes, order and prefixes.** A cell's start tag is kept as the bytes the file
//!   wrote **unless regenerating it from `r`, `s` and `t` would reproduce it exactly** — and that is
//!   decided by doing the regeneration and comparing, not by a rule of thumb. So an `x14ac:` on a
//!   cell, a single-quoted value, a `t` written before `r`, or two spaces between attributes all keep
//!   the file's bytes, and a cell Excel wrote plainly costs nothing. Editing such a cell rewrites its
//!   run *in place*, so the unmodelled attribute survives the edit too.
//! * Raw bytes preserve one thing a `Vec<RawNode>` cannot: the whitespace *inside* a start tag, which
//!   a decomposed attribute list does not record. `mjx-xml`'s own writer gives that up for any
//!   element it rewrites; this store does not.
//!
//! # What is refused, and what is preserved
//!
//! One thing a file can say is an error: a `c@r` that is not a cell reference. The store is keyed on
//! it, and a key it cannot parse is not a key. Everything else a worksheet can get wrong — rows out
//! of order, a duplicated row number, a `c@r` naming a different row than its `row@r`, cells out of
//! column order, a `t` that disagrees with the child element present — is read as it stands, written
//! back as it stands, and described by [`SheetData::anomalies`]. Nothing is sorted, deduplicated or
//! corrected, because every one of those would change the bytes of a part nobody asked to edit.
//!
//! # Not in scope here
//!
//! The shared-string table is MJXOFF-97 (D05) — a `t="s"` cell holds an index and this is the
//! contract it is read through. Formulas are preserved byte for byte and parsed by MJXOFF-115 (D11).
//! `CT_Rst`, the rich text an `<is>` holds, is MJXOFF-97's too. The worksheet's other 38 children are
//! MJXOFF-102 (D07), and styles are MJXOFF-105 / MJXOFF-108.

mod anomaly;
mod attributes;
mod read;
mod record;
mod store;
mod view;
mod write;

pub use anomaly::SheetDataAnomaly;
pub use record::PayloadShape;
pub use store::SheetData;
pub use view::{Cell, CellValue, Row};
