# Guide

**The upper shared markup: `mjx-chart`, `mjx-omml`, `mjx-vml`.** One guide set over three crates,
because they are one thing — the markup that sits *on top of* DrawingML and SpreadsheetML rather than
beside them, and that every format crate reaches down into rather than owning.

`mjx-dml` is the vocabulary a shape is drawn in and `mjx-sml` is the vocabulary a cell is written in.
These three are what you get when you build something *out of* those: a chart is a plot area drawn
with DrawingML fills over numbers held in a SpreadsheetML workbook; an equation is Word's own
mathematical typesetting; a VML drawing is the legacy surface a modern construct falls back to. None
of them is a format, and none of them is a container. Each is a vocabulary a `.pptx`, a `.docx` or an
`.xlsx` embeds.

## Where they sit, and what "rank 2.2" actually means

| Crate | What it models | Who reaches it |
|---|---|---|
| `mjx-chart` | `dml-chart.xsd` — the `c:chartSpace` part a `p:graphicFrame`, a `w:drawing` or an `xdr:graphicFrame` points at | `mjx-pptx`, `mjx-docx`, `mjx-xlsx`, `mjx-ooxml` |
| `mjx-omml` | `shared-math.xsd` — the `m:oMath` Word embeds inline in a paragraph | `mjx-docx` |
| `mjx-vml` | the five `vml-*.xsd` schemas — the legacy `vmlDrawingN.vml` part and the inline `w:pict` | `mjx-pptx` (behind its `vml` feature), `mjx-docx`, `mjx-xlsx` |

All three are **rank 2.2**, the top of shared markup. A rank in this project is a *ceiling on what a
crate may reach*, not a claim about what it does reach, and only one of these three uses the height:

* `mjx-chart` genuinely reaches down — to `mjx-sml` (2.1) for the workbook it embeds, and to
  `mjx-dml` (2.0) for every fill, outline and text body a chart draws with. **That edge is the whole
  reason rank 2.2 exists.** A chart carries a real `.xlsx` package inside it, and a chart part is
  full of DrawingML; put the crate any lower and neither edge is legal.
* `mjx-omml` and `mjx-vml` declare **no dependency on `mjx-dml` or `mjx-sml` at all** — read their
  `Cargo.toml` files, which name `mjx-ooxml-core`, `mjx-xml`, `mjx-derive` and `mjx-ooxml-types` and
  nothing else. They sit at 2.2 because that is where shared markup a format crate embeds belongs,
  and because an edge either of them may one day want (an equation carrying a DrawingML colour, a VML
  shape resolving a theme name) is legal from here and would not be from 2.0.

**And none of the three can see the other two.** `CLAUDE.md`'s layering rule makes a sideways edge as
illegal as an upward one, and `xtask/tests/layering.rs` reads the real graph out of `cargo metadata`
and fails on either. That is not a formality; it decides what these crates can model. A chart's text
properties cannot be typed as OMML here, and a VML fallback branch inside a chart part stays raw —
both stay in the unknown bucket and both round-trip verbatim, and the crate that can see both sides
(`mjx-docx`, rank 3.0) is where a typed accessor over one goes. `mjx-omml`'s own crate documentation
records the same rule biting harder still: `CT_CtrlPr` is a `w:rPr` by the schema's own declaration,
and `mjx-omml` sits *below* `mjx-docx`, so it preserves the child wholesale and `mjx-docx` types it
from above.

## The pages

Six here, plus one hosted by each of the other two crates — hosted there for the layering reason
above, exactly as `mjx_opc::guide`'s sixth page is hosted by `mjx-mce`.

| Page | Read it when |
|---|---|
| [Reading a chart](reading_a_chart) | You have a chart part and want its series, its numbers or its kinds |
| [Axes, titles and decoration](axes_titles_and_decoration) | You are reading or writing anything around the plot rather than in it |
| [Authoring a chart](authoring_a_chart) | You are building a chart from nothing, or pointing one at a worksheet |
| [The embedded workbook](the_embedded_workbook) | **Before any call that changes a chart's data** — it decides what happens to the producer's own spreadsheet |
| [Fidelity and the known gaps](fidelity_and_gaps) | Before you rely on any of it: the two mechanisms, what checks each crate, and what is not checked |
| `mjx_omml::guide` | A paragraph contains an equation |
| `mjx_vml::guide` | You have reached legacy markup — an OLE fallback, a comment box, a form control |

## The one thing to know before reading further

**The three crates do not have the same guarantee, and the difference is not small.**

| | Schema-validated | Child order from the XSD | Round-trip |
|---|---|---|---|
| `mjx-chart` | **yes** — `dml-chart.xsd`, a category-1 modelled schema in `mjx-schema-gate` | **yes** — generated into `mjx_ooxml_types::child_order` | yes |
| `mjx-omml` | **yes**, as a subtree — there is no math *part*; an `m:oMath` is validated with the `word/document.xml` that carries it | **yes** — `shared-math` is in the generator's `CHILD_ORDER_SCHEMAS` | yes |
| `mjx-vml` | **one child at a time** — `vml-main.xsd`, through a `WrapperRoot` in `mjx-schema-gate` | **no** | yes |

A `.vml` part's root is a bare `<xml>` wrapper in no namespace at all, which no VML schema declares a
global element for, so the document as a whole cannot be handed to a validator. Its children can:
`crates/mjx-schema-gate/src/categories.rs` files a VML part as a `WrapperRoot` and validates each
child of the wrapper separately against a driver over `vml-main.xsd`.

It was category 2 — *markup this project preserves verbatim and never validates* — until MJXOFF-245,
on a reason that also claimed `vml-main.xsd` could not compile without an `xml.xsd` the Transitional
set does not ship. MJXOFF-134's driver schema had removed that obstacle two phases earlier.

What VML still has no gate for is **child order**: no `vml-*` schema is in the generator's
`CHILD_ORDER_SCHEMAS` (MJXOFF-264). `mjx_vml::guide` states what that does and does not buy a caller,
and it is worth reading before trusting anything this library does to a VML part.

## What none of these crates does

**None of them opens a package.** `mjx-opc` is rank 1.0 and models the container; these model markup.
`mjx-chart` names `mjx_opc::Package` in exactly one place — `mjx_chart::embedded_workbook_part` and
the patch beside it, which have to walk a relationship to reach the workbook a chart embeds — and
nothing else in the three crates knows what a part is.

**None of them renders, and none resolves a reference.** `mjx_chart::ChartSeriesReferences` hands
back the `c:f` text a producer wrote, as text; turning that into cells needs a package, so it is
`mjx-xlsx`'s. A chart's cached values are what draw, and this crate never recomputes one.

**None of them is reached directly by an ordinary caller.** An application opens a file through
`mjx_ooxml::Deck`, `mjx_ooxml::Document` or `mjx_ooxml::Workbook`, and the chart vocabulary is
projected onto all three — see `crates/mjx-ooxml/docs/shared_markup_reachability.md` for which of
these types a facade caller can reach and by which method. This set is for someone working on the
library, or someone who has reached through one of the facade's three escape hatches.
