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

Each row is a deliberate decision with an issue behind it, not an oversight.

### Charts

| Gap | Consequence | Issue |
|---|---|---|
| Refreshing a chart's workbook **regenerates** it | A data edit rewrites the embedded workbook from the chart's own data, so the two always agree. Formatting or extra sheets a third-party workbook carried do not survive that. Detach the workbook first if you would rather keep it stale than lose it | MJX-116 |
| Chart **colour and style parts** (`colors1.xml`, `style1.xml`) | Preserved verbatim, no typed surface. These are Office 2013+ extensions outside ECMA-376, and a chart renders without them; the in-schema styling — `c:style`, `c:varyColors`, a series' `c:spPr` — is modelled | — |
| A series' data **labels**, **trendlines**, **error bars** and per-point formatting (`c:dLbls`, `c:trendline`, `c:errBars`, `c:dPt`) | Preserved verbatim, no typed surface | — |

The rest of this block is closed (MJX-116): an authored chart now writes its embedded workbook and a
data edit refreshes it; all sixteen plot types read their series; literal (`c:numLit`/`c:strLit`) and
multi-level (`c:multiLvlStrRef`) sources read; and the axes, gridlines, titles, legend and series
fill/outline have a typed surface. **The workbook is written by a minimal SpreadsheetML writer inside
`mjx-chart`, scheduled for removal once `mjx-xlsx` can write** — it writes one sheet, a shared-string
table and a styles skeleton, and deliberately nothing else.

### Legacy and non-DrawingML content

| Gap | Consequence | Issue |
|---|---|---|
| The **InkML strokes themselves** are not modelled | A stroke set is carried verbatim and handed to you as bytes. InkML is a W3C vocabulary with no OOXML semantics of its own; `add_ink` and `set_ink_content` check the root namespace and store what you give them. A deliberate non-goal: parsing it would buy reach into a format this library does not render | — |
| A **SmartArt layout is not run** | `add_diagram` writes the data, layout, style and colour documents and the frame that names them; it does not compute where the nodes land. PowerPoint regenerates the cached `dsp:drawing` when it opens the deck, and a diagram that already has one keeps it verbatim (`DiagramParts::drawing`). A deliberate non-goal: a layout engine is a rendering feature, and there is no rendering here | — |
| An **ActiveX control's properties** (`ax:ocxPr`) are not modelled | The control part's class id and persistence read and write; the property bag inside it is carried verbatim. It is a Microsoft extension outside ECMA-376, and its meaning is per-control-class | — |
| **VML geometry is preserved, not evaluated** | A `v:shape`'s identity, style, references, fill, stroke and children are typed; the `path` command string and `v:formulas` are carried verbatim rather than evaluated into a path. The same non-goal as the layout engine: evaluating them is a rendering feature | — |
| A **start tag whose attributes are wrapped across lines re-flows** when its part is edited | The fidelity reader records each attribute's name, value and quote, but not the whitespace that separated it from the previous one, so re-serialising writes one space. This never touches a part you did not edit — an untouched part re-emits its original bytes and is never serialised at all — but a part you *do* edit comes back with its start tags on one line. Office wraps VML start tags far more often than it wraps a slide's, so it shows there first; `crates/mjx-opc/tests/tree_roundtrip.rs` pins it | — |

### Tables

| Gap | Consequence | Issue |
|---|---|---|
| Selections are not merge-aware | `Cells::rectangle` selects *positions*. Formatting a selection that partly covers a merge writes to the covered cells too — harmless, since they render nothing, but not what you asked for | MJX-43 follow-up |
| `extLst` on cells and table properties | Opaque | — |

### Text

| Gap | Consequence | Issue |
|---|---|---|
| Shape-level `a:lstStyle` has no public setter | Readable and resolved, but not authorable | — |
| A font slot the theme does not define | Keeps its `+mj-lt` reference rather than being replaced by a guess | — |

### Colour and layout

| Gap | Consequence | Issue |
|---|---|---|
| `comp` / `gray` / `gamma` / `invGamma` transforms | Implemented from the prose; **not verified against Office** | MJX-211 R3 |
| A transform naming a rotation but not both `a:off` and `a:ext` | `effective_shape_bounds` answers `None` — the honest result of whole-transform inheritance | MJX-211 R7 |

### Verification

| Gap | Consequence | Issue |
|---|---|---|
| **Every fixture is hand-crafted** | No test in this repository reads a file that Microsoft PowerPoint wrote. LibreOffice confirms decks *open*; nothing yet confirms they *render as intended*. This is the one part of the legacy-content work that is not closed: the surfaces are built and schema-checked, but against markup we wrote ourselves | MJX-211 R2, MJX-140 |
| **The schema gate covers only the markup this project authors** | `crates/mjx-pptx/tests/schema_validity.rs` validates every fixture part and every deck this library authors against the ECMA-376 XSDs, and CI runs it as a blocking job (`schema-validity`, with `MJX_REQUIRE_SCHEMA=1`, fetching the schemas by pinned checksum). The namespaces it validates are the ones we write — PresentationML, DrawingML, DrawingML charts, DrawingML diagrams, SpreadsheetML, and the two OPC control streams; InkML, ActiveX, VML and document properties are markup we only preserve and are reported as skipped, never validated against a schema they were not written to | MJX-248 |
| Parts carrying `mc:AlternateContent` are not schema-validated | Markup Compatibility lives outside the base schema by design, so such parts are skipped with a named reason. This shades nothing this library writes — no authoring path emits `mc:AlternateContent` | MJX-248 |
| The 0.0.58 text-inheritance change | A non-placeholder shape now takes the master's `p:otherStyle` / `p:bodyStyle` per ECMA-376 §19.3.1.35. This follows the spec, but real PowerPoint is believed to match the *previous* behaviour. It is isolated in one revertible commit pending validation | MJX-211 R1, MJX-208 |

### Whole formats

`.docx` and `.xlsx` open and round-trip through the OPC and fidelity layers, but `mjx-docx` and
`mjx-xlsx` have no editing surface — they are scaffolds for `v0.2` and `v0.3`. There is no rendering
of any kind: no layout engine, no SVG, no PDF. Encrypted and password-protected packages are out of
scope; digital signatures are preserved, not processed.

## Stability

The public API is **not stable until `v0.1`**. The version is `v0.0.x` and the patch number increments
each development iteration; a breaking change can land in any of them, and one already has. Pin an
exact version.
