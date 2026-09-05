# The cell store — the decision record

**MJXOFF-95 (Phase D, position 4).** What `crates/mjx-sml/src/cells/` is, what it was measured
against, and which alternatives lost. The module documentation in `cells/mod.rs` and `cells/record.rs`
carries the same reasoning next to the code; this page is the record with the numbers and the machine
attached, so a later reader can tell whether a figure still holds.

## Why this exists at all

`PLAN.md` settles the in-memory model as *"Hybrid: arena/columnar for bulk data (e.g. spreadsheet
cells, shared strings), owned trees for small structures"* and strings as *"Interning + `Cow`"*.
Three phases in, **only the owned-tree half had ever been exercised** — PowerPoint and Word have no
bulk-data case. A worksheet is the case that decision was made for: on a real workbook the *cell*
count, not the element count, decides whether the library is usable.

## The numbers this was designed against

From `docs/BENCHMARKS.md` (MJXOFF-147), measured on:

```
CPU     Intel(R) Core(TM) i7-9750H CPU @ 2.60GHz, 12 logical cores (6 cores, 2 threads/core)
RAM     32704624 kB (32 GiB)
OS      Manjaro Linux, kernel 7.1.9-1-MANJARO
rustc   1.98.0 (88d9e12ae 2026-08-18) / cargo 1.98.0 (797e8a9bc 2026-08-05)
build   release, with the workspace's `lto = true`, `codegen-units = 1`, `strip = "debuginfo"`
```

| Figure | Value | Where |
|---|---|---|
| The corpus worksheet | 300,000 cells / 610,005 elements / 8.54 MiB of raw XML | `xtask/src/corpus/xlsx.rs` |
| Materialising it as a `RawElement` tree | **+274 MiB peak RSS ≈ 913 B/cell**, ≈ 32× the source bytes | `docs/BENCHMARKS.md` |
| Where that cost is | *not* `size_of::<RawElement>()` (72 B, which would predict ~42 MiB) — it is the two small heap allocations every element carries for `children` and `attributes`, over 610,005 elements and roughly twice as many attributes | same |
| First materialisation vs. one edit | 306.5 ms vs. 2.05 µs — five orders of magnitude | same |
| Save at this scale | 305.5–349.9 ms, dominated by DEFLATE at ≈ 28 MiB/s | same |

Two consequences drove the design directly. **The per-node allocation is the cost**, so the store has
none. And **opening is the bottleneck, not editing**, so the design effort went into what a worksheet
costs to *hold* rather than into per-cell edit latency.

## The representation

Three flat arrays and one byte arena.

```
rows:        Vec<PackedRow>     48 B each, at most 1,048,576 of them
cells:       Vec<PackedCell>    36 B each, up to a million times more
cell_extras: Vec<CellExtras>    40 B each, allocated only for a cell that carries something unusual
arena:       the part's own bytes (shared, never copied) + whatever has been edited
```

`PackedCell` is the struct that gets multiplied:

| Field | Bytes | What it is |
|---|---|---|
| `reference` | 8 | MJXOFF-93's `CellReference` — `u32` row, `u16` column, two anchorings |
| `extent` | 8 | the cell's own `<c …>…</c>` range; **its copy-on-write state** |
| `payload` | 8 | the `<v>` text, or the whole `<is>` element |
| `style` | 4 | `c@s` |
| `extra` | 4 | index into `cell_extras`, or "none" |
| `kind` + `flags` | 2 | `c@t` as a code, plus five bits |
| | **36** | |

Every value the store preserves is a `(start, length)` pair of `u32`s over **one address space**: the
part's source bytes first, then the store's own `edits` vector. An address below `source.len()`
resolves into the part's buffer, which the store shares with the package through an `Arc` and never
copies; anything at or above it is a byte the store authored. That is copy-on-write stated at value
scale — **a worksheet nobody has touched owns no bytes of its own**, which
`SheetData::edited_bytes() == 0` asserts on every committed fixture.

### What was measured

`crates/mjx-sml/tests/cell_store_allocation.rs`, a dedicated single-threaded binary with
`mjx-allocation-counter`'s counting global allocator installed (debug build — the figures below are
allocation counts, not timings, so the profile does not move them):

| Case | Measured | Bound |
|---|---|---|
| One cell at `XFD1048576` | **368 B** at peak, 1 row record, 1 cell record | 8 KiB |
| 300,000 cells (5,000 × 60), `RawElement` tree | 240,626,381 B live — **802 B/cell** | — |
| 300,000 cells, cell store | **11,040,000 B live — 36.8 B/cell** | 48 B/cell |

**36.8 B/cell against MJXOFF-147's 913 B/cell of peak RSS is a 24.8× reduction**, and against the
802 B/cell the same allocator counts for the tree it was read from, **21.8×**. (Allocation count is
lower than resident set, as expected: the kernel's figure includes what the allocator holds without
returning it.)

The same store read from the **real corpus file**, through MJXOFF-147's own harness rather than a
parallel one — `cargo run --release -p xtask -- corpus --mem xlsx`, which gained a fifth checkpoint
next to the four it already had:

```
open                                  14088 KiB peak RSS so far
first-mutation materialisation       288196 KiB peak RSS so far
cell store (tree still alive)        299760 KiB peak RSS so far
    5000 rows, 300000 cells, 11040000 bytes of records (36.8 B/cell), 0 bytes edited
edit (one attribute, already materialised)     299720 KiB peak RSS so far
save                                 299720 KiB peak RSS so far
```

Reading the whole worksheet into the store costs **11,564 KiB of peak RSS on top of the tree —
38.7 B/cell against 913, a 23.6× reduction** — and the store reports `0 bytes edited`, which is the
copy-on-write rule as a number.

## The alternatives, and why each lost

| Shape | Cost per cell | Why not |
|---|---|---|
| `RawElement` tree, as PowerPoint and Word hold their parts | **913 B** (measured) | The baseline. A million-cell workbook costs about a gigabyte. |
| An owned typed tree — `Vec<Row>` of `Vec<Cell>`, each cell owning its strings | ≥ 200 B, unmeasured | The same shape with different type names: two allocations per cell plus one per row, which is exactly what the 913 B is made of. This is the answer the ticket names as the one not to arrive at by default, and it is rejected on the measurement rather than on taste. |
| `BTreeMap<CellReference, Cell>` | ~60 B + node overhead | Sparse and ordered, but it **dissolves the row**, and the row is both the unit the file is written in and the unit an untouched part is re-emitted in. A store that cannot say "these bytes are row 7" cannot re-emit row 7 verbatim. It also loses document order, which is what a file that wrote its rows out of order needs preserved. |
| A dense grid over the addressable range | 17 GB at one byte a slot | 1,048,576 × 16,384 slots for a sheet that may hold one cell. This is what the allocation gate is written against, and the mutation below turns it on to prove the gate can fail. |
| **Row-major flat arenas — what shipped** | **36 B** | — |

Two narrower shapes were costed on the record's own terms:

* **32 B**, by storing a cell's column and anchoring (4 B) instead of its whole `CellReference` (8 B)
  and recovering the row from the row record. It saves 11%, and pays for it by turning *"`c@r`
  disagrees with `row@r`"* — one of the untrusted-input cases this store must preserve rather than
  repair — into a special case routed through a side table.
* **32 B**, by moving `extra` into a side table keyed by cell *position*. The ticket suggests exactly
  this, and it is right that the common cell should pay nothing — but a key that is a position is
  invalidated by every insertion into the middle of the arena. The side table is still here; what the
  cell holds is a **stable index** into it rather than a position that shifts.

## The load-bearing decision: how an untouched row re-emits without being re-serialised

`RawElement` gives a *tree* subtree-level copy-on-write: an element remembers the byte range it was
parsed from (`RawElement::source_span`), `mjx-xml`'s writer copies that range instead of descending
into it, and the range is dropped by any mutation — on the node and, because mutable descent goes
through every ancestor's child list, on the whole path to the root. **The store is not a tree, so it
cannot inherit that mechanism. It restates it.**

* The sheet, every row and every cell each hold the byte range they were read from.
* Writing asks the same question at each level, outermost first. An untouched sheet is one `memcpy`
  and the rows are never visited. A sheet with one edited cell copies every *other* row whole, and
  inside the edited row copies every other *cell* whole.
* Every edit goes through one function (`SheetData::dirty_cell`) that clears the range on the record
  it touched, on that record's row, and on the sheet. What `DerefMut` enforces structurally for a
  tree, the store enforces by having exactly one door.

**Where the span invariant does real work for the reader.** A row's *gap* — the newline or comment
between it and the row before — is not read off the nodes at all; it is derived as `[end of the
previous row, start of this one)`. That is exact and allocates nothing, and it is sound **only**
because a `sheetData` element that still has a range is one in which every parsed descendant still has
one: an authored or edited child would have cleared the ancestor's. The reader depends on that
invariant explicitly, checks it, and falls back to serializing the nodes into the arena where it does
not hold.

**What the store deliberately does not do:** it does not stream. The store is built from a
`RawElement` tree, so *first materialisation* still costs the 274 MiB MJXOFF-147 recorded — the tree
is transient and can be dropped, but it is paid. That is within the budget this child was given
("opens, holds and saves within the budget MJXOFF-147 recorded"), and it is the right scope line:
building the store straight from bytes would need a streaming reader in `mjx-xml`, `quick-xml` being
allowed behind that crate and nowhere else. It is worth filing rather than pretending: **the holding
cost is now 25× better and the opening cost is unchanged.**

## How the unknown bucket survives a packed store

`CLAUDE.md` states the rule as *"every modeled complex type carries `extra: Vec<RawNode>` for unknown
children, and preserves unknown attributes, attribute order, and namespace prefixes."* A
`Vec<RawNode>` per cell is precisely the per-cell allocation the 913-byte measurement is made of, so
the store keeps the same rule in the representation it can afford — and, as it turns out, in a
stricter one.

* **Unknown children.** A cell's content is three byte runs — before the value, the value, after the
  value — and the first and last are replayed exactly. A `c/extLst` full of foreign markup, an `<f>`
  formula this child does not model, a comment between two cells, a row-level `extLst`: all come back
  byte for byte, in their original order, with their original prefixes.
* **Unknown attributes, order and prefixes.** A cell's start tag is kept as the bytes the file wrote
  **unless regenerating it from `r`, `s` and `t` would reproduce it exactly** — and that is decided by
  doing the regeneration and comparing byte for byte, not by a rule of thumb. So an `x14ac:` attribute,
  a single-quoted value, a `t` written before `r`, or two spaces between attributes all keep the file's
  bytes, and a cell Excel wrote plainly costs nothing. **Editing such a cell rewrites its run in
  place**, so the unmodelled attribute survives the edit too — that is what
  `cells/attributes.rs::set_attribute` is for.
* Raw bytes preserve one thing a `Vec<RawNode>` cannot: the whitespace *inside* a start tag, which a
  decomposed attribute list does not record and which `mjx-xml`'s own writer gives up on for any
  element it rewrites.

**What a rebuild does lose**, stated so it is not discovered later: the whitespace an *end* tag is
allowed to carry (`</v >` comes back `</v>`), and the qualified name of a row or cell whose prefix
differs from the `sheetData` element's. Both are reflows on a record somebody edited, the same
contract `mjx-xml` states for a rewritten element — and neither can reach a record nobody touched,
because that record is copied rather than rebuilt.

## What is refused, and what is preserved

Exactly one thing a file can say is an error: **a `c@r` that is not a cell reference.** The store is
keyed on it, and a key it cannot parse is not a key; `SmlError::Address` says so.

Everything else a worksheet can get wrong is read as it stands, written back as it stands, and
described by `SheetData::anomalies()` — rows out of order, a duplicated row number, a row with no `r`
at all, a `c@r` naming a different row than its `row@r`, two cells at one address, cells out of column
order, a `t` that disagrees with the child element present. **Nothing is sorted, deduplicated or
corrected**, because every one of those would change the bytes of a part nobody asked to edit. The
report is computed on demand and caches nothing, so a caller who never asks pays nothing.

## Complexity

| Operation | Cost |
|---|---|
| `row(n)` | `O(log rows)`, or `O(rows)` for a file whose rows are not ascending |
| `cell(reference)` | `O(log rows + log columns)`, likewise |
| Appending a cell at the end of the last row | amortised `O(1)` — building a sheet the way a file is written is linear |
| Inserting a cell in the middle | `O(cells after it)` for the arena `memmove`, plus `O(rows after it)` for the slice fixups |
| Writing an untouched sheet | one `memcpy` |
| Writing a sheet with one edited cell | one copy per untouched row, one rebuild for the edited row |

The middle-insertion cost is the price of one flat arena, and it is documented on
`SheetData::set_cell_value` rather than hidden. A caller who builds top-to-bottom, left-to-right —
which is the order a worksheet is written in — never pays it.

## What this child did not model

Shared strings are MJXOFF-97 (D05): a `t="s"` cell holds an index, and `Cell::shared_string_index` is
the contract that table will be read through. Formulas are preserved byte for byte and parsed by
MJXOFF-115 (D11); `Cell::formula_markup` is the opaque handle. `CT_Rst` — the rich text an `<is>`
holds — is MJXOFF-97's. The worksheet's other 38 children are MJXOFF-102 (D07), and styles are
MJXOFF-105 / MJXOFF-108.
