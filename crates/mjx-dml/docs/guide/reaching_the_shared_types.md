# Reaching the shared types

Every format crate uses this one, and none of them uses it the same way. This page is the map: which
DrawingML each format meets, which host wrapper lives here rather than there, and the one distinction
that catches everybody the first time.

## Where DrawingML turns up in each format

| | Where it appears | The wrapper element | Modelled in |
|---|---|---|---|
| PowerPoint | every shape on every slide, layout and master | `p:sp` / `p:pic` / `p:graphicFrame`, whose `p:spPr` is `a:CT_ShapeProperties` | `mjx-pptx` owns the wrapper, this crate the contents |
| Word | `w:drawing` in a run | `wp:inline` / `wp:anchor` → [`mjx_dml::wordprocessing_drawing`](crate::wordprocessing_drawing) | **here** |
| Excel | `xl/drawings/drawingN.xml` | `xdr:wsDr` and its three anchors → [`mjx_dml::spreadsheet_drawing`](crate::spreadsheet_drawing) | **here** |
| all three | the theme part | `a:theme` → [`mjx_dml::Theme`](crate::Theme) | **here** |
| all three | a picture's payload | `a:graphic` → `a:graphicData` → `pic:pic` | **here** |
| all three | a chart's envelope | `a:graphic` → `a:graphicData@uri` naming ChartML | envelope here, payload in `mjx-chart` |

`crates/mjx-ooxml/docs/shared_markup_reachability.md` answers the neighbouring question — which of
these a `mjx-ooxml` caller can reach through the facade, and where the three surfaces differ.

## The distinction that catches everybody: a type is not a namespace

`CT_ShapeProperties` is declared in `dml-main.xsd`. The **element** that carries it is not.

```xml
<p:spPr>…</p:spPr>      <!-- PresentationML -->
<pic:spPr>…</pic:spPr>  <!-- dml-picture.xsd -->
<xdr:spPr>…</xdr:spPr>  <!-- dml-spreadsheetDrawing.xsd -->
<wps:spPr>…</wps:spPr>  <!-- wml-drawingml.xsd -->
```

All four are the same complex type in a *local element declaration* inside the host's own schema, so
each takes the host's target namespace. There is no such thing as `a:spPr` outside `dml-main.xsd`'s
own types. The same is true of `cNvPr`, `blipFill`, `docPr`, `ext` and `off`.

This is why the constructors come in pairs, and why the second one is nearly always the one you want:

```text
ShapeProperties::new(interner, "spPr")          →  <a:spPr/>          — almost never right
ShapeProperties::with_name(interner, name)      →  <{host}:spPr/>     — what a host wants
```

[`mjx_dml::ShapeProperties::with_name`](crate::ShapeProperties::with_name),
[`mjx_dml::NonVisualDrawingProps::with_name`](crate::NonVisualDrawingProps::with_name) and
[`mjx_dml::PictureFill::with_name`](crate::PictureFill::with_name) each take the fully qualified name
the host builds; `new` exists for the handful of genuinely `a:`-namespaced uses. On the read side it
never matters: `from_xml` matches children by `(namespace, local)` and **never validates the
element's own name**, because the caller is the one who knows what it asked for.

## Which host wrapper lives where, and the rule that decides it

> **A wrapper lives in `mjx-dml` when everything inside it is DrawingML. It lives in the format crate
> the moment one child is that format's own markup.**

That single rule explains every placement:

* **`wp:inline` / `wp:anchor` are here.** Their content is a `wp:docPr`, a wrap mode and an
  `a:graphic` — all DrawingML. [`mjx_dml::wordprocessing_drawing::Inline`](crate::wordprocessing_drawing::Inline)
  and [`mjx_dml::wordprocessing_drawing::Anchor`](crate::wordprocessing_drawing::Anchor) are the two.
* **`wps:wsp` is not.** A Word shape may carry a text box, whose content is `EG_BlockLevelElts` —
  WordprocessingML paragraphs and tables. Typing it would mean this crate reaching up past its own
  rank, so `WordprocessingShape`, `TextboxInfo` and `TextBoxContent` are `mjx-docx`'s, and `mjx-docx`
  reads the generic [`mjx_dml::GraphicData`](crate::GraphicData) and parses its own payload out of the
  raw children. That argument is written out in full in `crates/mjx-dml/src/wordprocessing_drawing.rs`.
* **`xdr:wsDr` and its anchors are here**, for the same reason: twelve of that schema's seventeen
  types exist only to wrap something `dml-main.xsd` already declares.
  [`mjx_dml::WorksheetDrawing`](crate::WorksheetDrawing) is the part root. `mjx-sml` resolves an
  anchor against a sheet's column widths, and `mjx-xlsx` owns the part and the image relationships —
  both from above.
* **`c:chart` is not.** ChartML is `mjx-chart`, rank 2.2. This crate can build the *envelope* around
  it — [`mjx_dml::GraphicData::for_chart`](crate::GraphicData::for_chart) writes
  `<a:graphicData uri="…/chart"><c:chart r:id="…"/></a:graphicData>` — because a `c:chart` is a
  self-closing leaf carrying one relationship id and no chart markup at all. Both `mjx-pptx` and
  `mjx-docx` place a chart through that one builder rather than through two.

## The one payload this crate does type

[`mjx_dml::GraphicData`](crate::GraphicData) dispatches exactly one payload kind:
[`mjx_dml::Picture`](crate::Picture) (`pic:pic`), because `dml-picture.xsd` is three complex types and
eleven lines and every one of them is DrawingML. Everything else — a chart, a diagram's `dgm:relIds`,
a table's `a:tbl`, an OLE object, a Word shape — stays
[`GraphicDataContent::Raw`](crate::GraphicDataContent::Raw), preserved verbatim in its original
position, and the crate above parses it if it wants to.

## `mjx-pptx` does not use `ShapeProperties`, and that is on purpose

[`mjx_dml::ShapeProperties`](crate::ShapeProperties) arrived late — MJXOFF-107 and MJXOFF-131 both
found it missing — and by then `crates/mjx-pptx/src/slide.rs` had been navigating `p:spPr` by hand for
several phases, with its own tests. Migrating it is a refactor with its own risk and no defect behind
it, so it has not been done. **New callers use the type; the PowerPoint reader keeps its own
navigation.** The reasoning is recorded at the head of `crates/mjx-dml/src/shape_properties.rs`, and
both readers place a new child through the same generated
[`SHAPE_PROPERTIES`](mjx_ooxml_types::child_order::SHAPE_PROPERTIES) rank table, so they cannot drift
on the one thing that would actually corrupt a file.

## Going the other way: never

Nothing here may name a crate above rank 2.0, and `xtask/tests/layering.rs` reads the real dependency
graph out of `cargo metadata` and fails on any edge that does not point strictly down. If a model
here seems to need something from `mjx-docx`, that is the rule telling you the model belongs in
`mjx-docx`.
