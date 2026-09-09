# What is not projected

Three different things get called "missing", and only one of them is a gap. This page separates
them: the nine methods that stay in Rust **on purpose**, the one thing no caller in either language
can do, and two classes that are exported and lead nowhere.

## The nine that stay in Rust, and why

`Deck` carries 262 methods on the facade and 259 here; `Document` 126 and 123; `Workbook` 141 and
138. The difference is nine methods and, on `Deck`, four more that a default build does not compile
at all. None of the nine is an oversight, and they are the same nine in both languages.

| Left behind | On | Why |
|---|---|---|
| `presentation`, `presentation_mut`, `into_presentation` | `mjx_ooxml::Deck` | Each hands back a whole `mjx_pptx::Presentation` |
| `document`, `document_mut`, `into_document` | `mjx_ooxml::Document` | The same, for `mjx_docx::Document` |
| `workbook`, `workbook_mut`, `into_workbook` | `mjx_ooxml::Workbook` | The same, for `mjx_xlsx::Workbook` |

These are the **escape hatches**, and their value is exactly what a binding cannot carry: a
`mjx_pptx::ShapeCursor` borrows the deck it walks, and the closure-taking readers beside it hand a
borrowed view to a function. Neither shape survives a boundary where every value must be owned. A
projection of them would have to copy the whole model out and back, which is not the same operation
and would quietly stop being one the first time somebody edited through it.

Four more `Deck` methods — `add_vml_drawing`, `vml_drawing_part`, `vml_part_bytes`,
`vml_part_names` — are absent from a *default* build only. They sit behind the `vml` feature that
`bindings/mjx-python/Cargo.toml` and `bindings/mjx-wasm/Cargo.toml` both declare and neither turns
on, exactly as `mjx-ooxml` leaves it off. See [Installing](installing).

## The one gap that is a gap

**No caller of either of these two languages can set a document's title, author or creation time.**

`mjx_pptx::Presentation::blank_with_properties`, `mjx_docx::Document::blank_with_properties` and
`mjx_xlsx::Workbook::blank_with_properties` all exist: each takes an
`mjx_opc::doc_props::CoreProperties` and an `ExtendedProperties`, so an authored file can name a
title and a creator. **None of the three facade surfaces projects it**, and a binding can only
project what the facade carries — so the method reaches neither Python nor TypeScript.

It is worth being exact about who is affected, because it is easy to overstate: a **Rust** caller who
reaches past the facade *can* do this today, and `crates/mjx-pptx/tests/schema_validity.rs` does. It
is the facade and its two bindings that cannot. `xtask/tests/facade_curation.rs` records it as the
ledger's single `NoFacadeEquivalent` entry, which is where the decision to close it will be taken.

What `Deck.blank`, `Document.blank` and `Workbook.blank` do instead is write both `docProps` parts
with this library's own defaults. **An opened file keeps its own untouched**, which is the half that
matters for fidelity: nothing here overwrites properties somebody else wrote.

Two smaller ones from the same audit, both recorded on `crates/mjx-ooxml/docs/guide/fidelity_and_gaps.md`:

* **Writing a formula.** `CellBlock.formula` reads one; nothing on `Workbook` writes one. A Rust
  caller can write the `<c>` markup through `mjx_ooxml::Workbook::workbook_mut` — which is on the
  list above, so a Python or TypeScript caller cannot round-trip a formula they just read.
* **Package inspection is Excel's alone.** `Workbook.part_names`, `part_bytes` and `content_type_of`
  have no `Deck` or `Document` counterpart in any of the three languages.

## Three classes that led nowhere, and the gate that would have found a fourth

**Closed by MJXOFF-228.** `ResolvedColor` and `TableStyleFlags` were exported by the facade and by
**both** bindings, and in all three languages nothing returned one, nothing took one, and — for
`ResolvedColor` — nothing constructed one either. A caller could import the name and never obtain a
value.

It was never the bindings' doing: `crates/mjx-ooxml/src/lib.rs` re-exported both types, no facade
method mentioned either, and each binding faithfully projected what the facade published.

The fork was real — project the producers, or stop re-exporting the types — and the producers won,
because removing an export is a breaking change to three surfaces while adding a reader cannot break
anyone. Each type now has exactly one:

| Type | Obtained from |
|---|---|
| `ResolvedColor` | `Deck.resolved_scheme_color` / `Deck.resolvedSchemeColor` — what `a:schemeClr@val` actually paints on a surface, through its colour map and its theme |
| `TableStyleFlags` | `Deck.table_style_flags` / `Deck.tableStyleFlags` — all six emphasis flags in one read, the shape `applicable_parts` takes |
| `Backdrop` | `Deck.shape_backdrop` / `Deck.shapeBackdrop` — the plane a 3-D scene's shadows fall on |

**`Backdrop` was not on the ticket.** MJXOFF-228 named two; the sweep written to close the class
found three, which is the whole argument for writing the sweep rather than the two assertions. It is
`every_exported_class_is_obtainable_from_some_other_call` in `xtask/tests/facade_curation.rs`, and it
asks the type-level form of the reachability rule: **is every exported class named by some signature
anywhere?** It reads the committed `.pyi` — a declaration that already exists, is parity-checked
against the compiled module, and is checked by `mypy --strict` — rather than parsing signatures out
of two hand-written Rust crates, which is what `xtask/tests/binding_projection.rs` refused to do and
was right to refuse.

To reproduce the shape it looks for, from the repository root:

```sh
cargo test -p xtask --test facade_curation every_exported_class_is_obtainable_from_some_other_call
```

## Three arguments that read differently in Python

Not gaps, but the places a Python caller meets an inconsistency the Rust caller never notices,
because Rust arguments are positional and Python's are named.

* **`series_idx` against `series`.** Twenty-five chart methods on `Deck` and `Document` spell the
  parameter `series_idx`, `point_idx`, `axis_idx` or `trendline_idx`; the same methods on `Workbook`
  spell it `series`, `point`, `axis`, `trendline`. In Rust and TypeScript both are positional and the
  difference is invisible; in Python it is keyword-visible, on the one family the workspace
  advertises as identical across all three surfaces. `_idx` is the wider convention here —
  `shape_idx` alone appears 286 times in the stub — which is also the argument against it, because
  `shape_idx` accepts a `ShapePath` and is not an index at all.
* **The four calls that take the column first.** `Workbook.add_chart`, `add_range_chart`,
  `add_one_cell_anchored_picture` and `add_two_cell_anchored_picture` take `(column, row)` where 47
  other facade methods take `(row, column)`. The reason is good and is written down on
  `crates/mjx-ooxml/docs/guide/addressing.md`: an `xdr` marker is
  `<xdr:col><xdr:colOff><xdr:row><xdr:rowOff>`, so its offsets interleave with its indices.
* **`CellReference.new`, `relative` and `absolute`** take `(column, row)` for the same reason, and
  say so on `mjx_sml::CellReference` itself.

All three are recorded as the user's decisions rather than settled here: each is a rename or a
signature change in three languages at once.
