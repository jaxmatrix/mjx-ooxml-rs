# Fidelity, and the known gaps

Read this before you rely on the library in production. The first half is the guarantee; the second
half is an honest list of what this library does not do.

## The guarantee

**Open any `.pptx`, change one word, write it back, and every part you did not touch is
byte-for-byte what it was.**

Not re-serialised from a model that happens to agree — the original decompressed bytes. That holds for
parts this library has never heard of, for vendor extensions, for markup from a future version of
Office.

Four mechanisms make it true:

- **Part-level laziness with copy-on-write.** A part is raw bytes until something needs it parsed. On
  first edit it is serialised from the model and the raw bytes are dropped; until then it re-emits
  verbatim. This is also why every reader takes `&mut self` — reading may materialise a tree, though
  it never marks a part modified.
- **The unknown bucket.** Every modelled type carries the children it does not understand, and
  preserves unknown attributes, attribute order and namespace prefixes. A shape with an extension this
  library cannot read still round-trips through an edit to its text.
- **Markup Compatibility.** `mc:AlternateContent`, `mc:Ignorable` and `mc:ProcessContent` are
  preserved on write and resolved non-destructively on read.
- **The round-trip contract.** *Per-part decompressed-payload byte identity, plus structural container
  identity* — the same part set, content types and relationships. The container's ZIP bytes are **not**
  reproduced identically, because deflate parameters vary by encoder. If you need to compare two decks,
  compare parts, not archives.

## The content that is not DrawingML

A `.pptx` carries five kinds of content that are not DrawingML shapes: OLE objects, ActiveX controls,
ink, SmartArt diagrams and legacy VML. Each of them lives in its own part (or four), referenced from
the slide by relationship id — and each now has the same three things every other shape kind has.

| Content | Read | Author | Edit |
|---|---|---|---|
| OLE objects | `ole_objects`, `ole_prog_id`, the payload bytes, the snapshot image, `ole_legacy_shape_id` | `add_ole_object` — an embedded stream, a whole embedded package, or a link | `set_ole_prog_id`, `set_ole_object_data`, `set_ole_snapshot_image`, `replace_ole_object_with_placeholder` |
| ActiveX controls | `activex_control_count` / `_name` / `_shape_id`, `activex_class_id`, `activex_persistence`, the `.bin` state, the snapshot image | `add_activex_control` | `set_activex_control_name`, `set_activex_state`, `set_activex_snapshot_image`, `remove_activex_control` |
| Ink (InkML) | `ink_references` ties each part to the shape that names it, both ways (`ink_part_for_shape`, `shape_for_ink_part`) | `add_ink` | `set_ink_content` |
| SmartArt / diagrams | `diagram_relationship_ids`, `diagram_parts` (all five, the cached drawing included), `diagram_part_bytes` | `add_diagram` — four generated documents, or four of your own | `set_diagram_part` |
| Legacy VML | `vml_drawing_part`, `with_vml_drawing` (a typed `mjx_vml::Drawing`), `with_vml_shape_for_ole_object` / `_for_activex_control` | `add_vml_drawing` | `edit_vml_drawing` |

The VML column is behind the `vml` Cargo feature. That flag decides only whether **this** crate
re-exposes the surface: `mjx-vml` is a normal crate any consumer depends on directly, and a VML part
round-trips byte-identically whether or not the feature is on, because that is the packaging layer's
job rather than the feature's.

The one thing worth knowing about the VML surface is *why* it exists. A legacy construct is only
useful if you can get from the modern markup that points at it to the legacy shape that draws it, and
that hop is an identifier match: `p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all name
a VML shape's `id`. `with_vml_shape_for_ole_object` walks it for you.

## The gaps

Two lists, kept apart on purpose. The first is what this library **decides not to do**, each entry
with the reason it is a decision. The second is what is **built but not yet verified against Office**,
each entry with the work that will verify it. Nothing here is an oversight, and nothing here is a
fidelity hole: every gap below is *reach* — something you cannot ask for — never something the library
loses. A deck carrying any of it round-trips unchanged.

### Non-goals

| Non-goal | What you get instead | Why |
|---|---|---|
| **`extLst` is never modelled** — on a cell, on a table's properties, on a shape, a line, a chart, a text run | The extension list, the `uri` of every extension in it, and all of its content come back exactly as they went in, through an edit to the element that carries it | `extLst` **is** the unknown bucket, at the schema's own insistence: `CT_OfficeArtExtension` is a required `uri` plus `xsd:any processContents="lax"`, so an extension's content is markup in a namespace nobody but its author defines. Modelling it would mean modelling `a16:`, `p14:` and every vendor namespace after them. What matters is that an extension survives an edit *and stays where the sequence puts it* — `extLst` is last in `CT_TableCellProperties`, `CT_TableProperties`, `CT_TextListStyle` and the rest — and that is pinned by tests rather than asserted here |
| **A font slot the theme does not define keeps its reference** (`+mj-lt`, `+mn-ea`, …) | The reference itself, verbatim, as the effective answer | The alternative is a guess. A deck naming a slot its theme leaves undefined — or a `+…` spelling `a:fontScheme` has no slot for — is telling you something, and substituting a plausible typeface would hide it. Resolution replaces a reference only with a font the theme actually names |
| **A transform naming a rotation but not both `a:off` and `a:ext`** answers `None` for its bounds | `effective_shape_bounds` says "no answer", not "at the origin" | A transform is inherited **whole**: the first tier that places a shape wins entirely, and a shape cannot take its position from one tier and its size from another. A partial transform therefore places nothing, and `None` is the honest report of that |
| **A chart's workbook is regenerated, not patched** | A data edit rewrites the embedded workbook from the chart's own data, so the two always agree | Reconciling an arbitrary third-party workbook with edited chart data is a merge problem with no correct answer. Detach the workbook first if you would rather keep it stale than lose the formatting or extra sheets it carried (MJX-116) |
| **Chart colour and style parts** (`colors1.xml`, `style1.xml`) are preserved, not modelled | The parts, verbatim | They are Office 2013+ extensions outside ECMA-376, and a chart renders without them. The in-schema styling — `c:style`, `c:varyColors`, a series' `c:spPr` — *is* modelled |
| **InkML strokes are not modelled** | The stroke set, verbatim, plus `add_ink` / `set_ink_content` checking the root namespace | InkML is a W3C vocabulary with no OOXML semantics of its own. Parsing it would buy reach into a format this library does not render |
| **A SmartArt layout is not run** | `add_diagram` writes the data, layout, style and colour documents and the frame naming them; PowerPoint regenerates the cached `dsp:drawing`, and a diagram that already has one keeps it verbatim | A layout engine is a rendering feature, and there is no rendering here |
| **An ActiveX control's properties (`ax:ocxPr`) are not modelled** | The class id and persistence read and write; the property bag inside is verbatim | A Microsoft extension outside ECMA-376 whose meaning is per-control-class |
| **VML geometry is preserved, not evaluated** | A `v:shape`'s identity, style, references, fill, stroke and children typed; the `path` command string and `v:formulas` verbatim | The same non-goal as the layout engine: evaluating them into a path is a rendering feature |
| **Markup Compatibility parts are not schema-validated** | They are skipped by the schema gate with a named reason, never silently | `mc:AlternateContent` lives outside the base schema by design. This shades nothing this library writes: no authoring path emits it |
| **The schema gate covers the markup this project authors, not the markup it only preserves** | Every fixture part and every authored deck is validated against the ECMA-376 XSDs for PresentationML, DrawingML, DrawingML charts, DrawingML diagrams, SpreadsheetML and the two OPC control streams, as a blocking CI job | InkML, ActiveX, VML and document properties are namespaces we carry rather than write. Validating them against a schema they were not written to would report noise, so they are reported as *skipped*, by name (MJX-248) |
| **No rendering, of any kind** | Measurement: resolved geometry, effective properties, absolute bounds | No layout engine, no SVG, no PDF. Rendering is a separate phase with no date |
| **Encrypted and password-protected packages are out of scope**; digital signatures are preserved, not processed | A typed error rather than a guess | Decrypting an ECMA-376 Part 2 protected package is a cryptography project, and validating a signature this library may then invalidate by rewriting the container would be worse than not claiming to |

Two more, still true and worth stating plainly because they are *limitations* rather than choices:

| Limitation | Consequence | What would remove it |
|---|---|---|
| **A start tag whose attributes were wrapped across lines re-flows when its part is edited** | The fidelity reader records each attribute's name, value and quote, but not the whitespace separating it from the previous one, so re-serialising writes one space. This never touches a part you did not edit — an untouched part re-emits its original bytes and is never serialised at all — but a part you *do* edit comes back with its start tags on one line. Office wraps VML start tags far more often than a slide's, so it shows there first | A whitespace field on `RawAttribute` and `RawElement` in `mjx-ooxml-core`: a breaking change to the hottest data structure in the library, taken deliberately rather than incidentally. `crates/mjx-opc/tests/tree_roundtrip.rs` pins the current behaviour both ways, so the day it changes, that test says so |
| **A series' data labels, trendlines, error bars and per-point formatting** (`c:dLbls`, `c:trendline`, `c:errBars`, `c:dPt`) have no typed surface | Preserved verbatim; a chart carrying them survives a data edit unchanged | The chart workstream. Phase A's chart child closed the data half — every plot type's series, literal and multi-level sources, axes, gridlines, titles, legend and series fill/outline (MJX-116) — and stopped at the decoration deliberately rather than half-modelling it |

The embedded workbook a chart authors is written by a **minimal SpreadsheetML writer inside
`mjx-chart`** — one sheet, a shared-string table and a styles skeleton, and deliberately nothing else.
It is scheduled for removal once `mjx-xlsx` can write, which is the Excel slice's job (`v0.3`).

### Built, not yet verified against Office

Everything here works and is tested against markup **we wrote**. What none of it has is a run through
real PowerPoint, and saying so is the point of this section.

| Not yet verified | What is in place | Who verifies it |
|---|---|---|
| **Every fixture is hand-crafted** | No test in this repository reads a file that Microsoft PowerPoint wrote. LibreOffice confirms decks *open*; nothing yet confirms they *render as intended* | The Office-authored fixture corpus (A12), which retires this weakness for every other row at once |
| `comp` / `gray` / `gamma` / `invGamma` **colour transforms** | Implemented from the ECMA-376 prose and unit-tested against it | Validation against real PowerPoint (MJX-211 R3) |
| **The 0.0.58 text-inheritance change** | A non-placeholder shape now takes the master's `p:otherStyle` / `p:bodyStyle` per ECMA-376 §19.3.1.35. This follows the spec, but real PowerPoint is believed to match the *previous* behaviour, so it is isolated in one revertible commit | Validation against real PowerPoint (MJX-211 R1, MJX-208) |

### Whole formats

`.docx` and `.xlsx` open and round-trip through the OPC and fidelity layers, and `mjx-docx` /
`mjx-xlsx` have no editing surface — they are scaffolds. That is a schedule, not a decision: Word is
the `v0.2` slice and Excel the `v0.3` slice, each with its own phase of work. Nothing in this page
about `.pptx` changes when they land.

### What used to be here

Rows close by being *done*, and the ones that have are named so a reader returning to this page can
tell the difference between "gone" and "quietly dropped":

- **Selections are now merge-aware.** `Cells::rectangle` still names positions, but a formatter
  resolves them through the merge grid: a cell covered by a merge renders nothing, so it is skipped,
  and unmerging gives it back its own formatting. A merged region anchored *outside* the selection is
  left alone entirely rather than painted from a cell the caller did not name. All three formatters —
  [`format_cells`](Presentation::format_cells),
  [`format_cell_text`](Presentation::format_cell_text) and
  [`format_cell_paragraphs`](Presentation::format_cell_paragraphs) — follow the same rule; merging and
  unmerging deliberately do not, because they must reach the cells they cover.
- **A shape's `a:lstStyle` is authorable.** Tier 3 of the text ladder was readable and resolvable and
  had no setter. It now has six: read, set and clear, for a level and for the `a:defPPr` beneath the
  levels, plus [`clear_shape_list_style`](Presentation::clear_shape_list_style) for the whole element.
  See [list formatting for the whole shape](crate::guide::shapes_and_text).
- **`Scene3D::backdrop`** is typed, alongside the bevels, light rigs and cameras it sits with.
- **The guide-formula evaluator**, **chart depth**, and **every kind of content that is not
  DrawingML** each closed a block of this page; the table above them is what is left.

## Stability

The public API is **not stable until `v0.1`**. The version is `v0.0.x` and the patch number increments
each development iteration; a breaking change can land in any of them, and one already has. Pin an
exact version.
