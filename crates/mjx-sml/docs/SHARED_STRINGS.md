# The shared string table — the decision record

**MJXOFF-97 (Phase D, position 5).** What `crates/mjx-sml/src/strings/` and `crates/mjx-sml/src/font/`
are, what was measured, which alternatives lost, and the two policies this child was asked to
*decide* rather than to implement. The module documentation carries the same reasoning next to the
code; this page is the record with the numbers and the machine attached.

Its companion is `docs/CELL_STORE.md` (MJXOFF-95). The two halves fit together in one sentence: **the
cell store holds a shared-string cell as a `u32` index and no text at all, and this table is what
that index means.**

## Why this exists

`PLAN.md` line 26 settles the in-memory model as *"Hybrid: arena/columnar for bulk data (e.g.
spreadsheet cells, **shared strings**), owned trees for small structures"*, and line 31 settles
strings as *"Interning + `Cow`"*. MJXOFF-95 built the first bulk-data case. This is the second, and
it is the only table in OOXML that exists **solely** to deduplicate text — so it is where the
interning half of that line actually has to be true rather than merely stated.

Before this child, `mjx-sml` could hold every cell of a workbook and tell nobody what any of them
said. `<c r="A1" t="s"><v>3</v></c>` says *"the string at position 3"*, and nothing in the worksheet
says what that is.

## The contract a `t="s"` cell is read through

Two calls, one on each side of the index, and neither crate holds the other's data:

```rust
let index = cell.shared_string_index()?;   // mjx_sml::Cell — MJXOFF-95
let value = table.item(index)?;            // mjx_sml::SharedStringTable — this child
let text  = value.text()?;                 // Cow<str>, borrowed from the part's own bytes
```

* `Cell::shared_string_index` answers `None` for a cell whose `t` is not `s`, **and** for an `s` cell
  whose `<v>` is not a number. A file can write either; both are reported as absence rather than
  repaired.
* `SharedStringTable::item` answers `None` for an index past the end. A file can write that too —
  `shared_strings_rich_text.xlsx` does, at `A4`, on purpose.
* `StringItem::text` answers for a plain `<t>` and for a sequence of `<r>` runs alike, so a caller
  reading a column of text never has to know which shape each entry used.

The **inline-string** path produces the same type from the same reader:

```rust
let inline = InlineString::parse(cell.inline_string_markup()?)?;
let text   = inline.item().text()?;        // the same StringItem, the same accessors
```

Nothing in either crate ever moves a cell's text between the two forms. A cell read as
`t="inlineStr"` is written back as `t="inlineStr"` with its own `<is>` bytes, and a `t="s"` cell keeps
its index. Converting between them is a decision about the workbook, not something a reader does on
the way past.

## The representation

Four flat arrays over the same byte arena the cell store uses (`crates/mjx-sml/src/arena/`).

```
items:     Vec<PackedStringItem>    48 B each, one per <si>
runs:      Vec<PackedRun>           36 B each, one per <r>
phonetics: Vec<PackedPhoneticRun>   24 B each, one per <rPh>
extras:    Vec<ItemExtras>          16 B each, only for an item carrying phonetic markup
arena:     the part's own bytes (shared, never copied) + whatever has been edited
```

`PackedStringItem` is the record that gets multiplied:

| Field | Bytes | What it is |
|---|---|---|
| `extent` | 8 | the whole `<si>…</si>`; **always present** |
| `leading` | 8 | the bytes between the previous item and this one |
| `text_element` | 8 | the `<t>…</t>`, so an edit can splice it |
| `text` | 8 | its still-escaped inner text |
| `first_run` + `run_count` | 8 | this item's slice of `runs` |
| `extras` | 4 | index into `extras`, or "none" |
| `flags` | 4 | `xml:space` state, and whether the entry may be interned to |
| | **48** | |

### The invariant everything rests on: the extent is always present

An item read from a part points at the part's own bytes; an item this table authored points at bytes
it appended to the arena; an item read from a tree that had lost its ranges is serialized into the
arena on the way in. **There is no "not backed by bytes yet" state**, and that single fact buys three
things:

* **Writing an item is a `memcpy`.** The whitespace inside a start tag, an `rPr` this workspace does
  not model, a comment between two runs and the exact spelling of `xml:space="preserve"` all come
  back because nothing ever re-serializes them.
* **Editing is a splice, not a rebuild.** Replacing a run's text replaces the bytes of that one `<t>`
  inside the item's bytes and leaves every other byte alone. A run's `rPr` survives an edit to the
  text beside it *exactly*, which a rebuild from a decoded model could only approximate.
* **Authoring and reading are one path.** An authored item is serialized to bytes, stored, and read
  back through the same reader a file's entries go through, so the records and the bytes cannot
  disagree — they were never produced separately.

### What was measured

`crates/mjx-sml/tests/shared_string_allocation.rs`, a second `harness = false` binary with
`mjx-allocation-counter`'s counting global allocator. (A second binary rather than a case inside
`cell_store_allocation.rs`, because a global allocator is process-wide and a second measurement's
zero would depend on what the first left live.)

| Case | Measured | Bound |
|---|---|---|
| One entry | **192 B** live | 8 KiB |
| 65,536 entries, `RawElement` tree | 43,254,531 B live — **660 B/entry** | — |
| 65,536 entries, string table | **3,145,728 B live — 48.0 B/entry** | 56 B/entry |
| The same 65,536 entries, text 10× longer | **3,145,728 B live — 48.0 B/entry** | must be *equal* |
| Bytes authored by a table nobody edited | **0** | 0 |

**13.75× smaller than the tree it was read from**, and the table shares the part's buffer rather than
copying it, so the process holds one copy of the text rather than two.

### The bound is not the gate — the fourth row is

This is worth stating plainly, because it is the shape of gate this project keeps having to
remediate. The obvious alternative design is `Vec<String>`: one owned string per entry. Against
*short* strings that costs 24 bytes of header plus the text, which for a twelve-character entry is
**less** than a 48-byte record. **A bytes-per-entry bound measured on short strings would have passed
the design this one exists to reject.**

So the load-bearing assertion is the fourth row: two tables with the same entry count and text
differing by an order of magnitude in length retain **the same bytes, to the byte**. An entry holding
a `(start, length)` pair into the part's own buffer has that property; an entry that owns its text
cannot have it at any string length.

### The alternatives, and why each lost

| Shape | Cost per entry | Why not |
|---|---|---|
| `RawElement` tree | **660 B** (measured) | The baseline. A 100,000-entry table costs 66 MB. |
| `Vec<String>` | 24 B + the text, plus one heap allocation each | Two copies of every string in the process — the package already holds the part's bytes. Its cost grows with the text; this one does not. And it is 65,536 small allocations, which is exactly what `docs/BENCHMARKS.md` identified as the real cost of the tree. |
| `HashMap<String, u32>` as the table | ~60 B + node overhead + two copies of the text | Dissolves the **order**, and the order *is* the index: entry 3 is entry 3 because it is third. It also loses duplicate entries, which real files carry (`shared_strings_rich_text.xlsx` has `"Alpha"` twice) and which must survive because cells point at both. |
| An index built eagerly at read time | one hash entry per plain entry, always | A table is read on every open and interned into only when something is written. The index is built on first `intern` and dropped by any edit that could invalidate it, so a read-only open pays nothing. |
| **Flat arenas + splice-on-edit — what shipped** | **48 B** | — |

## Policy 1 — `count` and `uniqueCount` are hints, and which is which

`CT_Sst` declares two optional attributes, and the difference between them is the difference between
what this table can know and what it cannot:

| Attribute | Means | Can the table know it? | What happens to it |
|---|---|---|---|
| `uniqueCount` | how many `si` entries there are | **yes** — the table *is* the entries | round-trips as read; recomputed **only** when the entry list changes, and **only** if the file wrote the attribute at all |
| `count` | how many `t="s"` **cells** in the whole workbook point into it | **no** — it cannot see a single cell | round-trips as read; **never** derived. `set_reference_count` is the only thing that writes it |

Both are hints a producer wrote, not values derived from the file, and real files disagree with
themselves. So both come back exactly as they were read, and a table whose `uniqueCount` says `6`
over seven entries writes `6` back.

**The one thing that moves `uniqueCount` is a change to the entry list**, because appending or
compacting makes the old value definitely wrong. An edit to an entry's *text* moves neither: it
changes neither the number of entries nor the number of referencing cells.

A file that wrote no `uniqueCount` never gains one. That matters: an attribute this library added
would be a byte the producer did not write, in a part it was not asked to author.

This is the second of the two mutations MJXOFF-97 required as proof. Making the writer recompute
`uniqueCount` unconditionally turns the fixture's `uniqueCount="6"` into `uniqueCount="7"` and takes
three cases red.

## Policy 2 — entry lifetime: nothing is ever renumbered

MJXOFF-97 asked this child to *decide* what happens to an entry whose last referencing cell was
deleted. The answer, and the consequence, in full:

**An entry's index is a public address.** It is written into the `<v>` of every `t="s"` cell that
uses it, in every worksheet of the workbook, and this crate can see none of them. Removing entry 3
therefore does not merely free entry 3 — it changes what entries 4, 5, 6 … *mean*, and every cell
holding one of those numbers now says something different.

So:

* **Entries are append-only.** Nothing in this crate ever removes one on its own. An entry left
  unreferenced by an edit stays where it is, costing a few dozen bytes and breaking nothing. That is
  also what MJXOFF-97's Tier-3 clause requires: *"editing one cell's text must leave every other `si`
  byte-identical, including entries the edit made unreferenced."*
* **`compact` is an explicit call, and it hands the problem back.** It takes a predicate saying which
  indices are still referenced, removes the rest, and **returns the old-index-to-new-index map**. A
  caller that ignores the return value has silently changed the text of an unpredictable number of
  cells; a caller that applies it must rewrite every shared-string cell in the workbook before the
  file is written.

**The consequence, stated so nobody discovers it later:** a workbook this library edits many times
accumulates dead entries. They are bytes in one part, they never affect a cell, and the alternative —
compacting on the library's own initiative — is a silent corruption of every sheet. When a caller
that *can* see the sheets wants the space back (MJXOFF-112's package writer is the first that could),
`compact` is there and the remapping is its problem, which is the only place that problem can
correctly be solved.

## Policy 3 — `xml:space="preserve"`, which is not merely preserved

`xml:space="preserve"` on a `t` is the difference between `"  total  "` and `"total"`: without it, a
consumer is free to normalize the leading and trailing whitespace. It is therefore part of the value.

And it does not validate. **`sml.xsd` types a `t` as the *simple* type `s:ST_Xstring`, which can
carry no attribute at all**, and the schema does not import the XML namespace. Both Excel and
LibreOffice write it anyway; `crates/mjx-schema-gate/src/tolerances.rs` has recorded that as a
producer-wide divergence since MJXOFF-91.

Preserving it on read is not in question. *Authoring* it is a different decision, and it is decided
on what the alternative costs:

* **Written where its absence would change the value** — that is, where the text has leading or
  trailing whitespace. Losing the string is worse than diverging from a schema every producer
  diverges from.
* **Not written anywhere else.** A table of ordinary strings is schema-valid markup, and it is
  byte-identical to what `mjx-chart`'s writer produces, which is what MJXOFF-112's parity gate rests
  on.
* **Re-decided on every edit.** An entry whose new text needs the attribute gains it; one whose new
  text does not, loses it. The attribute is a property of the value, not a sticky flag.

This is the first of the two required mutations. Dropping `xml:space` on write takes six cases red
across the unit tests and the fidelity suite.

## The `val`-wrapper family, and where MJXOFF-105 reaches for it

`crates/mjx-sml/src/font/` — **not** inside `strings/`, and that placement is the point.

`sml.xsd` declares the same fifteen font-property children twice:

* **`CT_RPrElt`** (line 1826) — a rich-text run's `rPr`, which is this child's.
* **`CT_Font`** (line 3781) — one entry of `styles.xml`'s font table, which is **MJXOFF-105 (D08)**'s.

They differ in exactly **two** places and nowhere else:

| | `CT_RPrElt` | `CT_Font` |
|---|---|---|
| the font-name element | `rFont` | `name` |
| `family`'s declared type | `CT_IntProperty` (`xsd:int`) | `CT_FontFamily` (`ST_FontFamily`) |

Both name types are `CT_FontName`; both `family` types are an integer on the wire. So the pair is one
Rust type — `FontProperties` — with a two-valued `FontPropertyOwner` saying which spelling to read and
write.

**MJXOFF-105 reaches for `mjx_sml::font`:**

| What D08 needs | What to use |
|---|---|
| a `styles.xml` `<font>` entry, decoded | `FontProperties::read(element, interner, FontPropertyOwner::FontTableEntry)` |
| the same from preserved bytes | `FontProperties::from_markup(bytes, FontPropertyOwner::FontTableEntry)` |
| writing one out | `FontProperties::write_into(out, prefix, "font", FontPropertyOwner::FontTableEntry)` |
| any `CT_Color` — a font's, a fill's `fgColor`/`bgColor`, a border's, a sheet's `tabColor` | `mjx_sml::Color`, with `Color::write_into(out, prefix, slot_name)` |
| one member of the `val`-wrapper family on its own | `font::value` (crate-private) — read `val`, apply the slot's default, write the element |

**If a font-table entry needs something `FontProperties` does not carry, grow this type.** A slot
added here is a slot both callers get. Forking it would put a second copy of eighteen lines of
`val`-parsing in the workspace with nothing scheduled to delete it — which is exactly the debt
MJXOFF-99 exists to discharge for `mjx-chart`'s duplicate SpreadsheetML writer, and not a debt to
take on knowingly a second time.

### `CT_Color` is not `mjx_dml::Color`, and this is a correction to the ticket

MJXOFF-97's ticket says to use `mjx_dml::Color` for a run's colour and not to introduce an
Excel-specific colour type. That does not survive contact with the two schemas:

* **DrawingML's colour is an element choice.** `EG_ColorChoice` is six elements (`a:srgbClr`,
  `a:schemeClr`, `a:sysClr`, `a:prstClr`, `a:scrgbClr`, `a:hslClr`) and the element *name* is the
  kind. `mjx_dml::Color` is built on exactly that.
* **SpreadsheetML's colour is one element with five attributes.** `CT_Color` (`sml.xsd` line 3502) is
  `auto`, `indexed`, `rgb`, `theme` and `tint`, with no children, and the element is named for its
  slot (`color`, `fgColor`, `tabColor`) rather than for its kind.

`indexed` (a row of the legacy 56-colour palette), `theme` (a *position* in the theme's colour
scheme, not a `SchemeColor` token) and `tint` have no representation in `mjx_dml::Color`. Routing
them through its `ColorSpec::Other` bucket would file `indexed="8"` under an element kind that does
not exist, and `tint` nowhere at all — data loss dressed as reuse.

What the ticket was protecting against is real and is honoured: there is exactly **one** spreadsheet
colour type in the workspace, and D08's fonts, fills, borders and tab colours all use it.

## Where the ordering table was consulted, and what it said

`mjx_ooxml_types::child_order` reports `CT_Rst` as `ContentModel::Sequence` — `t` (0), `r` (1), `rPh`
(2), `phoneticPr` (3) — and **`CT_RPrElt` and `CT_Font` as `ContentModel::Choice`, every slot at rank
zero**. So there is no ordering to impose on a run's properties, and inventing one would be this
crate making up a rule the schema does not have. The writer emits them in the schema's *declaration*
order because a writer has to pick something and a deterministic choice is testable; the fixture's
rich-text entry deliberately writes them in a *different* order, and it round-trips.

Nothing here reorders anything on read, ever, for either type.

## What this child did not model

* **`sst/extLst`** is preserved as bytes, like every other unmodelled child. MJXOFF-133 (D18) writes
  down the half of `sml.xsd` this workspace deliberately does not type.
* **`styles.xml`'s font table** is MJXOFF-105 (D08), which reuses `font/` rather than copying it.
* **The `t="str"` formula-result string** is MJXOFF-115 (D11); it is a cell value, not a table entry.
* **`mjx-chart`'s duplicate `SharedStrings`** is still there. This table reproduces its output byte
  for byte (`an_authored_table_matches_the_chart_writers_bytes_exactly` pins it), MJXOFF-112 (D10)
  holds the parity gate from the other side, and MJXOFF-99 performs the deletion.
