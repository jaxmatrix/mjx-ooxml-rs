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

## What is preserved but not modelled

These round-trip perfectly and can be *read*, but there is no typed surface and no authoring:

| Content | What you get |
|---|---|
| OLE objects | `ole_objects`, the payload bytes, the `progId`, the fallback image |
| ActiveX controls | control count and name, the `.bin` state bytes, the fallback image |
| Ink (InkML) | the part names and bytes — **not** tied back to a shape |
| Legacy VML | part names and bytes, behind the `vml` Cargo feature |
| SmartArt / diagrams | recognised as `GraphicFrameKind::Diagram`, nothing more |

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
| **Every fixture is hand-crafted** | No test in this repository reads a file that Microsoft PowerPoint wrote. LibreOffice confirms decks *open*; nothing yet confirms they *render as intended* | MJX-211 R2, MJX-140 |
| **The schema gate covers only the markup this project authors** | `crates/mjx-pptx/tests/schema_validity.rs` validates every fixture part and every deck this library authors against the ECMA-376 XSDs, and CI runs it as a blocking job (`schema-validity`, with `MJX_REQUIRE_SCHEMA=1`, fetching the schemas by pinned checksum). The namespaces it validates are the ones we write — PresentationML, DrawingML, DrawingML charts, and the two OPC control streams; InkML, ActiveX, VML and document properties are markup we only preserve and are reported as skipped, never validated against a schema they were not written to | MJX-248 |
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
