# Changelog

All notable changes to **mjx-ooxml-rs** are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Versioning

The project is pre-release and uses `v0.0.x`: the patch number is incremented each development
iteration until the first milestone. Milestones then advance the minor version:

- **`v0.1`** — PowerPoint (`.pptx`) complete
- **`v0.2`** — Word (`.docx`) complete
- **`v0.3`** — Excel (`.xlsx`) complete

Further milestones (rendering, bindings, …) are defined as that work is scheduled. The public API is
**not** stable until `v0.1`.

## [Unreleased — 0.1.0]

`v0.1` is where the public API stops being free to change. The milestone ships when the PowerPoint
slice is complete; until then the working versions stay `0.0.x` and this section accumulates every
break made on the way, so the migration note for `0.1.0` is written as the breaks happen rather than
reconstructed afterwards.

### Breaking changes

| Was | Is | Why |
|-----|----|-----|
| `Presentation::cell_span` → `(columns, rows)` | → `(rows, columns)` | `table_dimensions` answers `(rows, columns)`, `merged_cell_anchor` answers `(row, column)`, and every cell method takes `(row, column)`. Two same-typed `usize`s are read as a habit, not as a signature. |
| `mjx_dml::BlipFill`, `BlipFillMode` | `PictureFill`, `PictureFillMode` | `blip` is ECMA's abbreviation for "binary large image or picture" and nothing else's. This crate already expanded `a:buBlip` to `BulletPicture`. |
| `mjx_dml::Fill::Blip`, `FillSpec::Blip` | `Fill::Picture`, `FillSpec::Picture` | Same token, same expansion. Office's own name for it is "Picture fill". |
| `mjx_dml::StyleMatrixReference::idx` | `index` | An abbreviation named after the `@idx` attribute; the docs already called it an index. |
| `mjx_chart::DataLabelSpec::show_*` (7 fields) | `shows_*` | The struct a caller reads (`DataLabelSettings`) already said `shows_*`; the two differed by one letter. `TrendlineSpec` / `ChartTrendlineData` agree on all nine of theirs. |
| `mjx_chart::ErrorBarSpec::plus`, `minus` | `plus_values`, `minus_values` | Matches `ChartErrorBarData` and `ErrorBars::plus_values()`; `plus` alone did not say plus *what*. |
| `mjx_pptx::ChartErrorBarData::has_no_end_cap` | `no_end_cap` | Matches `ErrorBarSpec`, the struct that writes the same `c:noEndCap`. |
| `mjx_pptx::PptxError::PictureHasNoBlipFill` | `PictureHasNoImage` | Drops the token, and says what the caller can act on. |
| `mjx_pptx::Presentation::activex_binary_bytes` | `activex_state_bytes` | Reads exactly what `set_activex_state` writes; the pair named one artefact two ways. |

Nothing else in the public surface changed name or shape. The sweep read all 1,561 public
identifiers of the eleven merged PowerPoint children; everything else either already followed the
convention or is a spec-sourced proper noun (`Srgb`, `ScRgb`, `OleObject`, the preset-shape names
whose digits are part of their identity).

One candidate is deliberately **not** taken here and needs a decision:
`Presentation::delete_chart_data_labels` (writes `c:delete val="1"` — *draw nothing here*) sits
beside `remove_chart_data_labels` (removes the element — *say nothing here*), and `delete` and
`remove` are near-synonyms in English. Renaming the first to `suppress_*` would fix the collision,
but `delete` is the spec element's own name and is used consistently across a dozen `mjx-chart`
identifiers (`Axis::is_deleted`, `DataLabels::delete_all`, `auto_title_deleted`, …); renaming only
the `mjx-pptx` method would trade one inconsistency for another, and renaming the family is a larger
break through a subsystem that is currently coherent.

## [0.0.66] - 2026-09-02

API review and reorganisation — the last iteration before `v0.1` freezes the surface (MJX-37).

`crates/mjx-pptx/src/presentation.rs` had reached **12,771 lines and 266 public methods** in a single
`impl Presentation` block. It is the file the whole PowerPoint surface lives in, and the file
`mjx-docx` and `mjx-xlsx` will copy on day one of Phases C and D, so its shape is worth more than its
size suggests.

**The split changes no path a caller imports.** `presentation/` is sixteen modules along the seams
the guide already reads in — deck addressing, slide lifecycle, the shape tree, notes, text,
hyperlinks, table cells, table structure, bounds, appearance, the effective readers, charts, chart
decoration, pictures, legacy content, and the element builders shared by more than one of them. Every
method stays an inherent method on the one re-exported `Presentation`; the helpers that moved out are
`pub(super)`, visible only inside `presentation`. The public-item lines before and after are identical
as a set, and the workspace suite was unchanged at 1,528 passing tests across the move — which is what
a reorganisation is supposed to look like. It is committed separately from every behaviour change so a
reviewer can see that the move moved nothing.

`tests/public_paths.rs` guards that from outside the crate: one authored deck driven through every
seam using only `mjx_pptx::` paths, because a test that reached into `crate::presentation::text` would
prove nothing a caller can rely on.

**The three named inconsistencies are settled.** `cell_span` answers `(rows, columns)` like
everything else on the table surface. The eight DrawingML effects each take what the schema makes
required in `new` and name the rest with `with_` — so a shadow's distance no longer costs eight
`None`s — while an attribute the builder does not name stays unset, and an unset attribute is not
written. And the three `#[allow(clippy::too_many_arguments)]` sites, re-examined now that `Cells` and
`CellFormat` exist: one was dead and is gone, and the two that remain — eight distinct cell
coordinates apiece — are `#[expect]` with their reason, so the day the list fits, the attribute fails
the build instead of quietly outliving its cause.

**A loop in a doc example is a design defect.** Three remained across the guide, the README-adjacent
pages and the examples, all the same shape: `for i in 0..count()` with a fallible accessor inside,
rebuilding a list the deck already has and re-borrowing the part once per entry.
`Presentation::layouts` answers the layout inventory as `Vec<LayoutInfo>`, `Presentation::shapes`
answers a surface's shapes as `Vec<ShapeInfo>` — index, kind, and the placeholder slot each fills —
in one read, and `Presentation::shape_for_placeholder` answers the search *where did this template put
the title?*. Every loop still standing in a doc example iterates a collection the API handed over,
with no `?` inside it.

**`Presentation::from_package` is public**, as the facade needs: the constructor for a caller who
already holds the package — one `mjx-opc` opened directly, or one a facade opened once and dispatched
on by content type rather than handing the bytes back to each format crate to re-open.
`mjx_opc::Package` is re-exported from `mjx-pptx` alongside it, on the same reasoning the chart types
already are: a caller should not have to name another crate to state a parameter type.

**The naming sweep** covered all 1,561 public identifiers of the eleven merged children. Its nine
breaks are tabulated under **Unreleased — 0.1.0** above; the summary is that `blip` is not a word,
that an abbreviation named after an attribute is still an abbreviation, and that a struct a caller
reads and the struct it writes back should name the same field the same way.

Fidelity is unchanged and was the acceptance criterion throughout: per-part byte identity, modeled
round-trips, and edit isolation all hold, `MJX_REQUIRE_SCHEMA=1` passes 51 (52 with `--features
vml`), and all eight examples verify their own output.

## [0.0.65] - 2026-09-02

Chart decoration — data labels, per-point formatting, trendlines and error bars (MJX-116).

`c:dLbls`, `c:dLbl`, `c:dPt`, `c:trendline` and `c:errBars` were preserved verbatim and had no typed
surface at all. A5 closed the chart *data* half completely — every plot type's series, literal and
multi-level sources, axes, gridlines, titles, legend and series fill/outline — and stopped at the
decoration deliberately rather than half-modelling it. That was right for its scope; leaving it
unowned was not. **Data labels are the part of a chart a reader actually reads**, and until this
release a caller could not ask what one said, could not switch a series from value to percentage, and
could not author a chart that labelled itself.

All four families now **read, author and edit**. `crates/mjx-chart/src/decoration.rs` adds
`DataLabels`, `DataLabel`, `DataPointFormat`, `Trendline` and `ErrorBars`, each with the same
ordered-`content` + `Raw` shape as everything else in the crate, so an element nothing touched still
re-emits byte-for-byte. `c:plus` and `c:minus` are the same `CT_NumDataSource` a series' `c:val` is,
so a custom error bar's lengths read and write through the existing `NumericData`.

**The three tiers of a data label.** ECMA-376 §21.2.2.49 says `c:dLbls` states the settings "for an
entire series **or the entire chart**", and a `c:dLbl` overrides them for one point — so a label
resolves over three tiers, and `DataLabelSettings::inherit` merges them **per setting**, not per
tier: a series that only says `c:showVal` still takes its plot's `c:dLblPos`. A `c:delete`
short-circuits the chain, because `CT_DLbls` puts it in one `xsd:choice` with the settings group and
an element carrying one cannot carry the other. There is deliberately no fourth tier — `CT_Chart`
declares no `c:dLbls` of its own — and the model says so rather than inventing one.
`ChartLabelScope` names the three tiers on the `mjx-pptx` surface, so "label this series" and "label
this point" cannot be the same call, and three verbs separate three intentions: *state settings*,
*draw nothing here* (`c:delete`), and *say nothing here* (remove the element, inherit again).

**A `c:dPt`'s `c:idx` is never renumbered.** Per-point formatting is anchored by index into the
series; renumbering one when a series changes length would move a point's colour silently onto a
different point, which is worse than leaving it dangling. Nothing in this release rewrites an index
except an explicit `set_index`. `Series::decoration_beyond_data` and
`Presentation::chart_dangling_decoration` *report* the anchors an edit left past the end;
`drop_chart_dangling_decoration` removes them, and nothing removes them on a caller's behalf. A
`c:idx` that is not a number — `-1`, or a value past `u32::MAX` — addresses no point, is never
matched by a lookup, is never renumbered, and rides through a round-trip untouched. Writing past the
end is the same rule from the other side: it is refused with a typed error rather than written as an
anchor that names nothing.

**Writing is bound to the owning plot's kind, and both the placement and the refusal come from the
schema.** `SeriesDecoration` carries a `ChartKind` because `CT_BarSer` puts `c:dPt` at rank 6 and
`CT_PieSer` at rank 5, and because `CT_PieSer` declares no `c:trendline` and no `c:errBars` while
`CT_SurfaceSer` declares no decoration at all. Both questions are asked of the generated
`child_order` tables, which gain 29 named constants for this — the five decoration types, the eight
`CT_*Ser` and the sixteen `CT_*Chart` — rather than of a list written by hand. `ChartDataError` gains
seven variants, every one raised **before anything is written**, the way `ChartData::validate`
already refused a shape the schema rejects: a point index past the end of a series, a decoration the
series type does not declare, leader lines on one point's label (only `Group_DLbls` declares them),
an `ST_Order` outside 2–6, an `ST_Period` below 2, a non-finite measure, and custom error bars whose
length nothing determines.

Every name is sourced from the ECMA-376 Part 1 prose, never guessed: §21.2.3.11 for `ST_DLblPos`
(`ctr` → `Center`, `inEnd` → `InsideEnd`, `bestFit` → `BestFit`), §21.2.3.50 for `ST_TrendlineType`
(`movingAvg` → `MovingAverage`, `exp` → `Exponential`), §§21.2.3.12–14 for the error bars
(`cust` → `Custom`, `stdErr` → `StandardError`). The exact wire token appears in each item's docs, and
a token the schema does not admit reads as `None` rather than as a guess. The schema's own defaults
are honoured: a bare `<c:showVal/>` is `true`, `<c:trendlineType/>` is `linear`, `<c:errBarType/>` is
`both`, `<c:order/>` and `<c:period/>` are 2.

`ChartData::data_labels` lets a chart label itself the moment it is authored, refused by `validate`
for the two surface kinds, which declare no `c:dLbls`.

**Preservation gained reach and changed nothing.** The `mjx-opc` round-trip suites and tier-3 edit
isolation are unchanged, and decorating a chart dirties `chart1.xml` and *nothing else* — not even
the embedded workbook, because decoration is not data. With `MJX_REQUIRE_SCHEMA=1`, every authored
decoration validates against `dml-chart.xsd` under `xmllint`, including the two cases the ranks make
distinct: a pie chart, whose `CT_PieSer` places `c:dPt` differently, and a scatter chart with two sets
of error bars, which `CT_ScatterSer` admits and `CT_BarSer` does not.

The *Limitations* row in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` naming these four families
is **removed**, not softened — it moves to "what used to be here", where rows go when they close by
being done.

## [0.0.64] - 2026-09-02

Subtree copy-on-write — the copy-on-write `mjx-opc` does per part, now done per subtree (MJX-248).

A6 found that the fidelity reader records each attribute's name, value and quote but **not the
whitespace separating it from the previous one**, so a start tag Office wrapped across lines
re-emitted on one. It pinned the shortfall in `KNOWN_REFLOWS` and stopped, because the obvious fix —
a whitespace field on `RawAttribute` and a trailing-whitespace field on `RawElement` — costs a value
at every construction site, adds size to the hottest data structure in the library, and buys exactly
one preserved property. The next one (entity spelling, comment placement, self-closing style) would
cost the same again.

**The tree is span-preserving instead.** Every element parsed by `mjx_xml::fidelity` remembers the
byte range it came from; the document keeps the buffer; and the serializer writes an unmodified
element by copying that range rather than rebuilding it. One field subsumes the whole family:
whitespace between attributes, whitespace before `/>`, quote style, the spelling of a character
reference (`&#38;` stays `&#38;`), and the placement of comments and processing instructions inside a
subtree. `KNOWN_REFLOWS` is **deleted**, not emptied — its two-way pin fires when a part starts
round-tripping, so the entry could not have been left behind — and `crates/mjx-opc/tests/roundtrip.rs`
and `tree_roundtrip.rs` now have no exceptions at all.

**The invariant is structural, not remembered.** A range is only sound while the element still is
what was parsed, and `RawElement`'s fields are public, so there is nowhere to hook a "clear the span"
call. `RawElement` therefore keeps its attribute and child lists in a `RawElementContent` it
`Deref`s to: reads are unchanged (`element.children`, `element.attributes` still resolve), and any
*mutable* access goes through `DerefMut`, which drops the range. Because mutable descent into a child
passes through every ancestor's child list, that drops the range along the whole path from the root —
which is exactly "a mutation clears the span on that node and every ancestor", obtained by
construction rather than by discipline. `Clone` drops it too, so a subtree copied into another
document can never be written from the buffer it left behind, and `PartialEq` ignores it, so
`RawElement` equality still means "the same markup".

**The one way this could corrupt a file is namespaces**, and it is pinned first. A verbatim subtree
carries prefixes but not the `xmlns:` declarations that bind them; if a rewritten ancestor pruned a
declaration, every descendant beneath it would come silently unbound. It cannot, and the reason is
structural: the reader keeps `xmlns` declarations as ordinary attributes in document order and the
writer emits every attribute an element holds without inspecting any of them.
`crates/mjx-xml/tests/subtree_cow.rs` opens with the test that fails if that ever stops being true —
it namespace-resolves the *output*, the way a consumer does.

**The range is untrusted on the way out.** It is sliced fallibly, and then checked against the
element it claims to describe: the bytes must open with `<` plus that element's qualified name
followed by a delimiter, and close the way the element says it closes. That is what catches a mutated
`name` or `empty` — the two fields deliberately left outside the `Deref` because navigation reads
them constantly — and it means a wrong range degrades to a re-flow, never to wrong bytes. Adversarial
cases are pinned: out-of-bounds, inverted, pointing at a different element, and the one a naive
`starts_with` gets wrong (`<a>` must not claim `<abbr>`'s range).

Measured on a synthetic 2.3 MiB slide (80,004 elements, `cargo run --release -p mjx-xml --example
mjx248_measure`): `size_of::<RawElement>()` 64 → 72 bytes, **+8 bytes per element** — the span packs
into a `u32` start plus a `NonZeroU32` end, so `Option` needs no discriminant, and moving the lists
behind the `Deref` costs nothing. Serializing that part after editing one attribute of one element:
**4.59 ms → 0.27 ms, 17x faster**; untouched, 0.08 ms. A part read but not edited retains no extra
memory at all — `mjx-opc` now shares one `Arc<[u8]>` between the bytes it re-emits and the tree that
indexes into them — and an edited part holds its source buffer, which
`Package::release_unused_part_sources` reclaims once nothing can be copied from it.

One byte-fidelity defect the new adversarial corpus found is fixed with it: `<!DOCTYPE a>` lost the
space after `<!DOCTYPE`, because quick-xml trims it and the writer rebuilt the wrapper. The doctype's
inner bytes now come out of the source.

`crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` states the stronger guarantee — every subtree you
did not touch is byte-for-byte what it was — and drops the re-flow limitation, which is gone. It
gains a narrower one in its place: three surfaces (`edit_vml_drawing`, `edit_chart`, the table-style
list) read a whole part into a typed model and write the whole part back, so subtree copy-on-write
does not reach inside those; slide edits, which navigate in place, are unaffected.

## [0.0.63] - 2026-09-02

Schema-order emission — children are written in `xsd:sequence` order by construction (MJX-248).
OOXML complex types are overwhelmingly sequences, and **children in the wrong order are invalid even
when every child is present and every child is itself correct** — a repair-dialog defect, not a
cosmetic one. Nothing in the workspace enforced that on write: order was whatever each hand-written
serializer happened to do, so correctness rested on the author having read the XSD for that type and
on a fixture happening to exercise it. Fourteen separate hand-copied rank tables had grown across
`mjx-dml`, `mjx-chart` and `mjx-pptx`, one per type, each added by whoever noticed.

**The order now comes from the schema.** `cargo run -p xtask -- codegen` reads the `xsd:complexType`
content models of `dml-main.xsd`, `pml.xsd` and `dml-chart.xsd`, flattens each one — resolving
`xsd:group` references across schemas — and commits
`mjx-ooxml-types::child_order`: every child of every complex type of those schemas, with the position
it occupies. Alternatives of an `xsd:choice` *share* a position, which is exactly the "either an
`a:solidFill` or an `a:noFill`, and whichever is there is the one to replace" question a writer asks.
A type whose own model is `xsd:choice` or `xsd:all` — `CT_Path2D`'s repeating path commands, for
instance — is recorded as unordered rather than given a false order.

### The boundary this does not cross

Placement is a write-side operation and only ever runs on a child a caller asked to write. **Nothing
reads a document and rewrites it into schema order.** A real file may carry children in an order the
schema permits but this table would not have chosen, and re-ordering it would be corruption of the
caller's document rather than a fix. Existing children are never sorted; a new child is inserted
after the last sibling that must precede it. Markup the table does not name — an unmodelled element,
a foreign namespace, a comment, an `mc:AlternateContent` — is invisible to placement: it never moves,
and it never moves the insertion point, so it keeps its position relative to its known neighbours.

### Added

- `mjx-ooxml-types::child_order` — `ChildOrder`, `ChildSlot`, `ContentModel`, `TypeReference`, the
  placement primitives (`ChildOrder::replace_or_insert`, `insert`, `insert_index`,
  `insert_index_of_names`, `rank_of`, `slot`), the ordering audit (`ChildOrder::first_out_of_order`,
  `audit_tree`, `TreeAudit`, `OutOfOrderChild`), the by-symbol lookups (`find`, `root_element`), the
  three generated tables (`DML_MAIN_TYPES`, `PML_TYPES`, `DML_CHART_TYPES`) and twenty-eight named
  constants for the types this workspace writes.
- A child-order audit inside the schema-validity suite that runs on **every** authored-deck case,
  with or without `References/`: it walks every element of every part whose root the tables name and
  fails on the first child out of its type's sequence. `xmllint` catches an ordering fault only for
  the shape some case happens to author, and only where the schemas are installed.

### Changed

- Every insertion path in `mjx-dml`, `mjx-chart` and `mjx-pptx` now places children through the
  generated table. The fourteen hand-written rank tables are gone.

### Removed

- `TableStylePart::rank` and `CellBorder::rank`. Both existed only to feed a hand-written ordering
  table; the generated one is now the single source, and a second copy of a sequence is the thing
  this release exists to remove. `TableStylePart::all` and `CellBorder::all` are unchanged.

## [0.0.62] - 2026-09-02

Package invariant validation — a deck that would need repair is not written (MJX-248). This library
was very good at *not touching* what it does not understand, and had essentially no defence for what
it *does* write. `Package::save` performed no package-level check of any kind, so markup naming a
relationship its `.rels` never declared, a relationship pointing at a part that was not there, a part
no content-type rule covered, and duplicate identifiers where the format requires uniqueness could all
be written and shipped. Every one of them makes PowerPoint say it "found a problem with the content
and needs to repair", and none of them is visible to the schema gate: A1/A2 validate each part against
its XSD in isolation, and every one of these defects is a property of the package *graph*, perfectly
schema-valid part by part.

**`save` now validates first, and the check is not opt-in.** A check you have to remember is a check
that ships the fault it was meant to catch.

- `mjx-opc`: `Package::validate`, `Package::save_unchecked`, and `PackageDefect` — one variant per
  invariant, each naming the part, relationship and identifier at fault:
  `PartWithoutContentType` (ECMA-376 Part 2 §6.2.3), `RelationshipTargetMissing`,
  `UnresolvableRelationshipTarget`, `DuplicateRelationshipId` (§6.5.3),
  `UndeclaredRelationshipReference` — every attribute in the shared relationship-reference namespace,
  not `r:id` alone, because `shared-relationshipReference.xsd` types all fourteen of them
  `ST_RelationshipId` — and `PartIsNotWellFormedXml`. Reached through `OpcError::Invalid`.
- `mjx-pptx`: `Presentation::validate`, `Presentation::save_unchecked`, and `PresentationDefect`:
  `DuplicateShapeId`, `DuplicateListEntryId`, `DuplicateListEntryReference`,
  `ListEntryTargetHasWrongContentType` and `UnlistedRelationship` — the `p:sldIdLst` /
  `p:sldMasterIdLst` / `p:sldLayoutIdLst` agreement with a part's relationships, in both directions.
  Reached through `PptxError::InvalidPresentation`.
- `mjx-opc`: `Package::authored_xml_parts`, `ZipEntry::provenance`, `ZipEntry::tree` and
  `PartProvenance` — the validation scope, defined once so both layers agree on it.

**The scope is the markup this library will write.** A part still holding the bytes it was opened with
is re-emitted verbatim and is never faulted, so a file that arrives broken can still be written back,
and *reading* a part can never change whether a package saves. The moment an edit makes those bytes
ours, the same defect is refused. `save_unchecked` is the deliberate escape hatch.

**A corrupting bug the validator found on its first run.** `add_ole_object` gave its snapshot picture
a hard-coded `p:cNvPr@id` of `0` while the frame took an allocated id, so two OLE objects on one slide
wrote two shapes with the same non-visual id — a duplicate PowerPoint repairs. Fixed, with a
regression test that asserts the ids rather than only that the save succeeded.

The cost, measured on the largest fixture (`charts.pptx`, 43 entries, 39 relationships): **35.7 µs**
to validate against 3.2 ms to write the container — about 1% of a save. Nothing that arrived as
container bytes is ever tokenised.

## [0.0.61] - 2026-09-02

The remaining model gaps, and an honest gap table (MJX-43). The guide's gap list had accumulated
rows that were no longer true, rows that were real, and rows that were deliberate decisions filed as
though they were oversights. This release closes the real ones, restates the decisions as decisions
with their reasoning, and rewrites the page around the difference.

**A shape's own list style is authorable.** Tier 3 of the text ladder — `a:lstStyle` on a shape's
text body, the tier that says *every paragraph at this indent level, in this shape* — could be read
and resolved through since the ladder was written, and could not be stated. It now can:

- `mjx-pptx`: `Presentation::shape_list_style_level`, `set_shape_list_style_level`,
  `clear_shape_list_style_level`, `shape_list_style_default`, `set_shape_list_style_default`,
  `clear_shape_list_style_default`, and `clear_shape_list_style` for the whole element. The setters
  merge, as every other setter does; a clear that finds nothing changes nothing and does not dirty the
  part.
- `mjx-dml`: `TextListStyle::new`, `set_level`, `set_default_properties`, `remove_level`,
  `remove_default_properties`; `TextBody::set_list_style` and `remove_list_style`. A new level is
  placed by `CT_TextListStyle`'s sequence and a new `a:lstStyle` by `CT_TextBody`'s — between
  `a:bodyPr` and the first `a:p` — because order is validity, not style.

**The gap table is now two lists.** Non-goals, each with the reason it is a decision, and *built but
not yet verified against Office*, each with the work that will verify it. Four rows closed outright:
merge-aware selections (already true in the code and now proven by the cases that discriminate — a
merge anchored outside the selection, and the text and paragraph formatters, not just the cell
formatter), the `a:lstStyle` setter above, `Scene3D::backdrop`, and a font slot the theme does not
define — which was correct behaviour listed as a gap. `extLst` is restated as what it has always
been: the schema's own unknown bucket (`CT_OfficeArtExtension` is a required `uri` plus
`xsd:any processContents="lax"`), preserved verbatim through an edit and pinned there by tests at
both tiers rather than merely asserted.

- New fixture `tests/fixtures/table_extensions.pptx` — a table whose `a:tblPr` and one `a:tcPr` carry
  a vendor extension — registered with the OPC round-trip suites, the fidelity-tree suite and the
  schema gate.

## [0.0.60] - 2026-09-02

Typed surfaces for the content that is not DrawingML (MJX-140, absorbing MJX-139). Five kinds of
content — OLE objects, ActiveX controls, ink, SmartArt diagrams and legacy VML — round-tripped
perfectly and could be *read*, and that was all. There was no authoring, no editing, and no way to
answer the question that makes any of it useful: **which shape is this?** An InkML part was findable
but untraceable; a diagram was `GraphicFrameKind::Diagram` and nothing more; `mjx-vml` had been 69
lines since Phase 0, its own doc comment deferring "rich modeling and shape-level references" to "a
later phase" that had no owner and no date.

Every one of the five now has read, author **and** edit coverage. Nothing about the round-trip
guarantee changes: modelling a type only adds reach, and the edit-isolation tier over each fixture is
the gate this release is measured against.

`mjx-vml` — from 69 lines to a real model:

- `Drawing` (the `<xml>` root of a `vmlDrawingN.vml`, or any element holding VML shapes — a Word
  `w:pict`, an `mc:Fallback` branch), `Shape`, `ShapeTemplate`, `ShapeGroup`, `ImageData`, `TextBox`,
  `Fill`, `Stroke`, `ShapePath`, `DiagramText`; the Office extensions that carry the references —
  `ShapeLayout` / `ShapeIdMap`, `EmbeddedOleObject`, `Ink`, `ShapeProtections`; and
  `AttachedObjectData`, the legacy form control's own record. `DrawingPart` reads and writes a whole
  part.
- The point of it is one hop: `p:oleObj@spid`, `p:control@spid` and `o:OLEObject@ShapeID` all name a
  VML shape's `id`, and `Drawing::shape_by_identifier` resolves it.
- Names come from the ECMA-376 Part 4 §19 prose, never the wire token — `v:shapetype` is a
  `ShapeTemplate`, `o:idmap` a `ShapeIdMap`, `x:ClientData` an `AttachedObjectData` — and
  `ST_ObjectType`'s nineteen values expand to `PushButton`, `DropdownBox`, `AuditingLine` and the rest.

`mjx-pptx`:

- **Ink.** `ink_references` ties every InkML part to the content part that names it, finding both
  PresentationML's `p:contentPart` and the `p14:contentPart` producers wrap in `mc:AlternateContent`;
  `ink_part_for_shape` and `shape_for_ink_part` walk it either way. `add_ink` writes the part and the
  reference; `set_ink_content` replaces the strokes without touching the slide. Both check the root
  namespace, so a package cannot end up declaring `application/inkml+xml` over something else.
- **SmartArt.** `diagram_relationship_ids` and `diagram_parts` expose the whole graph — the four parts
  a `dgm:relIds` names plus the cached drawing, which hangs off the *data* part rather than the frame.
  `add_diagram` writes all four with their relationships and the frame; `DiagramContent::vertical_list`
  generates a working diagram from a list of labels, `from_parts` takes four documents of your own.
  `set_diagram_part` replaces one of them in place.
- **OLE and ActiveX.** `add_ole_object` (an embedded stream, a whole embedded package, or a link) and
  `add_activex_control` (the `ax:ocx` part, its `.bin` state and the `p:controls` container), plus
  `set_ole_prog_id`, `set_ole_object_data`, `set_ole_snapshot_image`, `set_activex_control_name`,
  `set_activex_state`, `set_activex_snapshot_image` and `remove_activex_control`. Reading gains
  `activex_class_id` and `activex_persistence`. Both kinds can be bound to their legacy fallback with
  `set_ole_legacy_shape_id` / `set_activex_control_shape_id` and read back with
  `ole_legacy_shape_id` / `activex_control_shape_id`.
- **VML** (behind the `vml` feature): `vml_drawing_part`, `with_vml_drawing`, `edit_vml_drawing`,
  `add_vml_drawing`, and the headline `with_vml_shape_for_ole_object` /
  `with_vml_shape_for_activex_control`, which walk from the modern frame to the legacy shape that
  draws it. The feature's boundary is unchanged and now documented: it decides only whether *this*
  crate re-exposes the surface, since `mjx-vml` is a normal crate `mjx-docx` will depend on directly.

Verification: the schema gate gains a DrawingML-diagram arm, because this project now writes those
four parts — `dml-diagram.xsd` joins the markers `harness()` requires, and a new case pins that all
four are *validated* rather than skipped. Seven new schema cases cover the authored diagram, OLE
object, ActiveX control, ink and VML deck plus an edited OLE object. `mjx-opc`'s `tree_roundtrip` now
covers the four legacy fixtures.

One limitation surfaced and is recorded rather than hidden: the fidelity reader does not preserve the
whitespace *between* attributes, so a start tag whose attributes were wrapped across lines re-flows
onto one line when its part is edited. It never touches a part nobody edited — those keep their
original bytes and are never re-serialised — but Office wraps VML start tags far more often than it
wraps a slide's, so it shows there first. `KNOWN_REFLOWS` in `crates/mjx-opc/tests/tree_roundtrip.rs`
pins it, and the fidelity guide states it. Fixing it means adding a field to `RawAttribute` and
`RawElement` in `mjx-ooxml-core` and touching ~140 construction sites across every crate, which is an
architectural decision rather than a fix to take inside this change.

The guide's "preserved but not modelled" table is gone, replaced by a read/author/edit table for the
five and five honest non-goal rows (InkML strokes, the SmartArt layout engine, `ax:ocxPr`, VML path
evaluation, and the re-flow above). A seventh example, `legacy_content`, exercises all five and runs
in both feature modes.

Still open from MJX-140: **producer-authentic validation**. Every fixture here is still hand-crafted,
so what the schema gate proves is that our reader agrees with our writer against markup we wrote.
Obtaining decks Microsoft PowerPoint actually produced needs Office, and belongs with the runtime
verification work.

## [0.0.59] - 2026-07-31

The usage guide and the first runnable examples (MJX-209). The repository documented every *item* —
every public item has rustdoc, `missing_docs` is a lint, a strict rustdoc job gates CI — and one
*concept*, the effective-properties page. It documented no *task*: nothing answered "I have a `.pptx`
and want to change the title", nothing answered "I want to produce a deck", and there was no
`examples/` directory or runnable program anywhere in the workspace.

First of three workstreams to `v0.1`: **documentation → external application surface → validation**.

`mjx-pptx`:

- A five-page guide under `crates/mjx-pptx/docs/guide/`, surfaced as the doc-only `mjx_pptx::guide`
  module tree: *building a deck* (the whole story once, end to end), *shapes and text*, *tables,
  charts and pictures*, *inheritance, layouts and masters*, and *fidelity and the known gaps*. The
  last is a candour page listing every deliberate gap with its issue — no embedded chart workbook, no
  guide-formula evaluator, selections that are not merge-aware, colour transforms implemented from the
  prose but unverified against Office, and the fact that no test in this repository reads a file
  PowerPoint wrote. **All 48 doctests in the guides compile against the real API.**
- Six examples under `crates/mjx-pptx/examples/`, each reopening what it wrote and asserting something
  about it: `build_a_deck`, `read_deck` (which re-saves and proves all 17 parts stayed byte-identical),
  `edit_text` (which reports that retitling a slide dirties exactly one part), `style_shapes`,
  `build_table`, `charts_and_media`. `anyhow` is added as a dev-dependency; examples are the one place
  file I/O belongs, because the library is bytes-in/bytes-out and the caller reads and writes.

CI: a new `examples` job runs all six, and the office-open job now feeds `build_a_deck`'s output
through LibreOffice — so "the guide's headline example produces a deck Office opens" is a merge gate.

Also: the README gains a quickstart, a guide table and the example commands; the `mjx-ooxml` facade
gains the guide ladder; and PLAN.md's Phase 3b, which still described tables as in progress and
speaker notes as open, records what actually shipped and adds Phase 3c.

Two API observations surfaced while writing the examples, recorded for the `v0.1` review (MJX-37):
`cell_span` answers `(columns, rows)` while `table_dimensions` answers `(rows, columns)`, and
`OuterShadowEffect` has no `Default` though `EffectListSpec` does. Both are documented where they
bite rather than worked around silently. No behaviour change in this release.

## [0.0.58] - 2026-07-31

Paragraph-hierarchy audit (MJX-22, closing MJX-38). The seven-tier text ladder passed its tests, but
those tests reached the interesting cases by mutating a deck through the builder API rather than by
reading a file, so two disagreements with ECMA-376 Part 1 had gone unnoticed. Both are fixed here,
each cited to the prose that settles it.

`mjx-pptx`:

- **A list-style tier now contributes its `a:defPPr` beneath its level.** `TextListStyle::default_properties`
  had existed since the text model landed and resolution never called it, so a tier supplying nothing
  but an `a:defPPr` contributed nothing and a paragraph at a level its style does not define came back
  empty. §21.1.2.2.2 defines `a:defPPr` as the properties applied "when no other paragraph properties
  have been specified"; §21.1.2.2.6 says the same of a paragraph. The audit also confirms there is
  **no** fallback to `a:lvl1pPr` — §21.1.2.4.13 keys the nine level elements strictly to `a:pPr@lvl`,
  so the existing level behaviour was already right.
- **A shape that is not a placeholder now takes a master text style.** Tier 5 was gated on `p:ph`;
  §19.3.1.35 instead splits by kind — `p:bodyStyle` for a text box (`p:cNvSpPr@txBox`), `p:otherStyle`
  for any other non-placeholder shape. Tier 4 keeps its gate: without a slot there is nothing to
  match. This changes what effective text a deck containing plain shapes or text boxes reports. Real
  PowerPoint is believed to match the previous behaviour, so it is isolated in one commit and tracked
  for validation against an Office-saved deck.
- New `slide::shape_is_text_box`, the reader counterpart of the `txBox="1"` the text-box builder
  already writes.
- The effective-properties guide records both rungs.

Tests: a new hand-authored `tests/fixtures/text_levels.pptx` in which every tier owns a facet no other
tier touches — nine body levels' worth of structure with `a:lvl5pPr` deliberately absent, a layout
overriding only two levels, a shape-level `a:lstStyle` no public setter can author, a footer, a text
box and a plain autoshape. `crates/mjx-pptx/tests/paragraph_hierarchy.rs` pins fifteen rungs against
it; the fixture is registered in the `mjx-opc` round-trip suites and in the LibreOffice open canary.
`layouts.pptx` is untouched.

## [0.0.57] - 2026-07-31

The effective-properties guide (MJX-23). Ten `effective_*` readers had shipped and nothing explained
the idea behind them: the knowledge was spread across ten per-method doc comments and the frozen
`docs/*_HANDOFF.md` files, which are history rather than user documentation. Documentation only — no
behaviour, no API change.

`mjx-pptx`:

- New guide at `crates/mjx-pptx/docs/effective_properties.md`, pulled in with `include_str!` on a
  documentation-only `effective_properties` module, so it reads as prose on a source host and renders
  as its own page in `cargo doc`. It covers: *what a file states* versus *what a renderer shows*; the
  one candidate walk every shape resolver is built on, and why a shape that is not a placeholder
  inherits nothing; the three-source ladder fill, outline and effects share; why a transform is
  inherited whole while text merges tier by tier; the seven text tiers and the level axis cutting
  across them; the shorter table-cell ladder (MJX-33's `effective_cell_*` trio); why colours bake to
  concrete `RRGGBB`; every stop condition, including why text answers with an empty spec where a fill
  answers `None`; and what one read costs.
- Each of the ten readers gains a link to the guide. Their own doc comments stay authoritative for
  their own ladders and stop conditions.

Also: the workspace README grows a guides list, and the `mjx-ooxml` facade — the crate its own docs
name as the entry point for reading the docs — grows a Guides section.

## [0.0.56] - 2026-07-30

Cell 3-D review and direct-cell authoring (MJX-109, closing the last code follow-up of MJX-38). D4
(MJX-100) left `Cell3D` with two material accessors pending a decision and gave a typed 3-D surface
only to the table-*style* cell3D (`a:tcStyle > a:cell3D`); a direct cell's `a:tcPr > a:cell3D` had
none. Both are settled here.

`mjx-dml`:

- `Cell3D::material` (typed) and `Cell3D::preset_material` (raw wire token) are kept as a deliberate
  pair — the typed accessor is the normal path and mirrors `Shape3D::material`; the raw one is an
  escape hatch for a producer value outside `ST_PresetMaterialType`. Docs rewritten to say so; no API
  change.
- `TableCellProperties` gains typed `cell_3d()` / `set_cell_3d()`, the direct-cell counterpart of
  `TableStyleCellStyle`'s, reusing the same `Cell3D` model and honoring `CT_TableCellProperties`
  schema order (`cell3D` after the borders, before the fill).

`mjx-pptx`:

- `CellFormat` gains `with_cell_material` / `with_cell_bevel` / `with_cell_light_rig`, mirroring
  `TableStyleFormat`. `format_cells` now authors a direct cell's `a:cell3D`; any facet set gives the
  cell a `cell3D` with the schema-required bevel. Additive, non-breaking.

- `docs/CUSTOM_GEOMETRY_HANDOFF.md` records the four shipped atoms (CG1–CG4), the design decisions,
  the verified schema, the known follow-ups (chiefly a guide-formula evaluator), and the 3-D audit
  that found `a:scene3d` / `a:sp3d` already complete — so MJX-44's opaque-geometry gap is closed.

## [0.0.54] - 2026-07-30

Custom geometry, the PowerPoint surface (MJX-44 CG4). The `mjx-dml` custom-geometry model (CG1–CG3)
now reaches `.pptx`: one accessor reads and writes both preset and custom geometry.

`mjx-pptx`:

- New `Geometry` enum — `Preset(ShapeGeometry)` | `Custom(CustomGeometrySpec)` | `Inherited`.
- **Breaking:** `Presentation::shape_geometry` now returns `Geometry` (was `ShapeGeometry`), and
  `set_shape_geometry` / the cursor's `.geometry(..)` now take a `Geometry` (was `ShapeGeometry`).
  Migrate a preset call by wrapping it: `Geometry::Preset(ShapeGeometry::…)`. `shape_geometry` no
  longer errors when a shape declares no geometry — it returns `Geometry::Inherited` — so
  `PptxError::ShapeHasNoGeometry` is no longer produced by these methods.
- `shape_geometry` now reads `a:custGeom` (as `Geometry::Custom`) as well as `a:prstGeom`;
  `set_shape_geometry` writes either, converts between them (the two are mutually exclusive), and for
  `Geometry::Inherited` removes the shape's own geometry element so an inherited one takes over.

Pre-`v0.1`, so the API is still unstable; this is the deliberate unification MJX-44 called for.

## [0.0.53] - 2026-07-30

Custom geometry, the container and auxiliary lists (MJX-44 CG3). Completes the `mjx-dml` model of
`a:custGeom` — the path list (CG2) now sits inside the whole `CT_CustomGeometry2D`, with its guides,
adjust handles, connection sites, and text rectangle.

`mjx-dml`:

- `CustomGeometry` (`a:custGeom`, `CT_CustomGeometry2D`) — a fidelity wrapper reading every child
  typed (`adjust_values`/`guides`/`adjust_handles`/`connection_sites`/`text_rectangle`/`paths`) and
  round-tripping byte-for-byte (an unmodeled child such as `extLst` re-emits verbatim).
- Interner-free value types: `GuideSpec` (`a:gd` name + formula), `AdjustHandle` (`a:ahXY` / `a:ahPolar`
  with their `gdRef*` / min / max bounds), `ConnectionSite` (`a:cxn` angle + position), and
  `Rectangle` (`a:rect` edges).
- `CustomGeometrySpec` + `to_custom_geometry` — the interner-free read/author surface; builds children
  in schema order, omits empty auxiliary lists, always writes the required `a:pathLst`.

Additive and non-breaking.

## [0.0.52] - 2026-07-30

Custom geometry, the path list (MJX-44 CG2). The drawing commands a freeform `a:custGeom` is traced
from — the render-critical core, on top of the CG1 value types.

`mjx-dml`:

- `Path2DList` (`a:pathLst`, `CT_Path2DList`) and `Path2D` (`a:path`, `CT_Path2D`) — fidelity wrappers
  that read their paths / flags typed and round-trip byte-for-byte (an unmodeled child re-emits
  verbatim). `Path2D` exposes `width`/`height`/`fill`/`stroke`/`extrusion_ok` (each `None` when
  unstated, distinct from the schema default) and `commands`.
- `DrawCommand` — the interner-free, ordered instruction a renderer follows: `MoveTo`, `LineTo`,
  `ArcTo { width_radius, height_radius, start_angle, swing_angle }`, `QuadBezierTo`, `CubicBezierTo`,
  `Close` (the `a:path` choice group `close`/`moveTo`/`lnTo`/`arcTo`/`quadBezTo`/`cubicBezTo`).
- `Point` — an interner-free `(x, y)` of `AdjustCoordinate`s; `AdjustPoint::value` resolves one.
- `Path2DSpec` (with `to_path_2d`) and `Path2DList::new` / `paths` / `specs` — the read/author surface.

Additive and non-breaking.

## [0.0.51] - 2026-07-30

Custom geometry, foundation types (MJX-44 CG1). Groundwork for a typed surface over `a:custGeom`
(`CT_CustomGeometry2D`) — the freeform path list a hand-drawn PowerPoint shape uses, until now
preserved only opaquely. This iteration adds the value types every piece of a custom geometry is
expressed in; the path list, guide/handle/connection lists, and the pptx accessor follow.

`mjx-ooxml-types`:

- Generated `PathFillMode` (`ST_PathFillMode`: `none`/`norm`→`Normal`/`lighten`/`lightenLess`/
  `darken`/`darkenLess`) — how a freeform `a:path` is filled (`a:path@fill`). Added to the DrawingML
  codegen allowlist.

`mjx-dml`:

- `AdjustCoordinate` (`ST_AdjCoordinate`) and `AdjustAngle` (`ST_AdjAngle`) — each a union of a
  numeric literal (`Emu` / `Angle`) and a geometry-guide reference by name (`Guide`), the two forms a
  custom-geometry coordinate or angle can take.
- `AdjustPoint` (`a:pt` / `a:pos`, `CT_AdjPoint2D`) — the `(x, y)` a path command, adjust handle, or
  connection site is drawn through; a fidelity leaf that reads its coordinates typed and round-trips
  byte-for-byte. Re-exported alongside `PathFillMode` from the crate root.

Additive and non-breaking.

## [0.0.50] - 2026-07-30

Inaccessible external sources — audio/video media (MJX-201 P4, **completing MJX-201**). A slide can
reference audio or video that lives online/externally and is unreachable on another platform. Every
media carrier — `a:videoFile`/`a:audioFile@r:link`, the `a14:media` fallback, `p:snd`/`p:sndTgt`
timing/transition sounds — resolves through a media-typed relationship in the slide's `.rels`, so a
media reference is neutralized by redirecting that relationship.

`mjx-pptx`:

- `Presentation::replace_media_with_placeholder` inserts a placeholder media part and retargets the
  relationship at it (`mjx_opc::Package::retarget_relationship`), so every carrier that named it
  resolves inside the package; the poster image is untouched. The placeholder is caller-supplied bytes
  or a built-in one matching the kind — `default_placeholder_audio()` (a minimal valid silent WAV) or
  `default_placeholder_video()` (a minimal structurally valid MP4 with an empty video track). A
  non-media relationship yields the new `PptxError::NotAMediaReference`.
- `Presentation::media_references` lists a surface's audio/video/media relationships (by id, with kind,
  target, and whether external) — the discovery surface for what to replace. `MediaKind` and
  `MediaReference` are the reported types.

Additive and non-breaking.

## [0.0.49] - 2026-07-30

Inaccessible external sources — OLE objects (MJX-201 P3). An OLE object can reference embedded (or
linked/external) data that is unreachable on another platform. Unlike a chart, an OLE object has no
cached fallback — but it is displayed via its snapshot image and its data stream is read only on
activation, so it is neutralized by redirecting the reference to an in-package placeholder.

`mjx-pptx`:

- `Presentation::replace_ole_object_with_placeholder` inserts a placeholder object part and retargets
  the OLE frame's data relationship at it (`mjx_opc::Package::retarget_relationship`, this feature's
  first consumer), so the object resolves inside the package. The placeholder is caller-supplied bytes
  or the new `default_placeholder_ole()` — a minimal but structurally valid MS-CFB compound file (an
  empty root storage). The `p:oleObj` markup is untouched; a replaced embedded part is left
  unreferenced and can be swept with `Package::remove_unreferenced_parts`. A non-OLE shape yields the
  new `PptxError::ShapeIsNotAnOleObject`.
- `Presentation::ole_objects` lists the OLE frames on a surface, each with its data target, `progId`,
  and whether the reference is external — the discovery surface for what to replace.

Additive and non-breaking. Next: audio/video media (P4).

## [0.0.48] - 2026-07-30

Inaccessible external sources — chart backing workbook (MJX-201 P2). A chart can reference a workbook
that lives online/externally; that reference can be unreachable on another platform. A chart renders
entirely from its cached data (`c:numCache`/`c:strCache`), so the workbook is only needed to *edit* the
data — which means the reference can simply be detached.

`mjx-pptx`:

- `Presentation::detach_chart_workbook` removes a chart's `c:externalData` reference — the element and
  its relationship — leaving the chart to render from its cache (the same cache-only shape a freshly
  authored chart has). An embedded workbook part is left unreferenced and can be swept with
  `Package::remove_unreferenced_parts`. A non-chart shape yields `PptxError::ShapeIsNotAChart`; a chart
  with no backing workbook yields the new `PptxError::ChartHasNoExternalData`.
- `Presentation::chart_workbooks` lists the charts on a surface that reference a workbook, each with its
  target and whether the reference is external — the discovery surface for what to detach.

Additive and non-breaking. Follow-up phases extend to OLE objects and media, where (unlike charts)
there is no cached fallback and the P1 redirect-to-placeholder is used.

## [0.0.47] - 2026-07-30

Inaccessible external sources — foundation + linked-image placeholder (MJX-201 P1, spun out of MJX-42).
Many element sources can be external/online (linked images, a chart's backing workbook, OLE, media),
and an unreachable target can crash a consumer. This begins the caller-driven capability to neutralize
one by substituting an in-package placeholder of the same kind; the library does no external I/O, so
the caller decides which references are inaccessible.

`mjx-opc` gains the general redirect lever:

- `Package::external_relationships` lists every `TargetMode::External` relationship (with its owning
  part) — the discovery surface for what might be unreachable.
- `Package::retarget_relationship` repoints a relationship at a new target/mode while keeping its id
  and its `.rels` position (editing the control tree and the navigation view in tandem). The recipe:
  `insert_part` a placeholder, then retarget the external relationship at it as `Internal` — so the
  binding element resolves in-package without touching its own markup, which is what the many unmodeled
  element kinds need.

`mjx-pptx` applies it to images (the one modeled kind, via element rewrite):

- `Presentation::replace_linked_image_with_placeholder` embeds a placeholder — caller-supplied bytes or
  the new `DEFAULT_PLACEHOLDER_IMAGE` — into a picture that links an external image, rewriting
  `@r:link` → `@r:embed` and dropping the dangling link relationship. An embedded picture yields
  `PptxError::PictureImageNotLinked`.
- `Presentation::linked_images` lists the linked pictures on a surface (with their targets) so callers
  need not walk the shapes.

Additive and non-breaking. Follow-up phases extend the same redirect to the chart workbook, OLE, and
media.

## [0.0.46] - 2026-07-30

Linked images become addressable (MJX-42, second of two package-gap fixes). A picture that *links* its
image (`p:blipFill > a:blip@r:link`) rather than embedding it was invisible to the API:
`picture_image_rel_id` read only `@r:embed` and returned `None`, so a linked image could not be
reached even though it round-tripped fine.

- `Presentation::picture_image_rel_id` now falls back to the link id, returning whichever relationship
  binds the image (embed preferred when both are present).
- New `Presentation::picture_image_link_target` returns where a linked image points — the relationship
  target string, external path/URL or in-package part alike — so a linked image is fully addressable.
- `picture_image_bytes` consequently reaches linked images: an embedded image or an internal link
  resolves to bytes; an external link reports `PptxError::ExternalTarget` (its bytes live outside the
  package).

Additive and non-breaking — an embedded picture reads exactly as before. Also elides a needless
lifetime flagged by newer stable clippy and unwraps single-literal `concat!`/drops unused imports in
`mjx-dml` tests, keeping the workspace clippy-clean under the current toolchain.

## [0.0.45] - 2026-07-30

Orphaned-part sweep (MJX-42, first of two package-gap fixes). Replacing an image, deleting a slide, or
any edit that unwires a relationship can leave a part with nothing pointing at it — a legal but dead
media blob. Until now nothing removed them; `remove_part_cascading` only walks downward from one named
part.

New `Package::remove_unreferenced_parts` on `mjx-opc` is the package-wide garbage collector. It returns
the swept part names and is conservative by construction: a part survives if it is reachable by
following `Internal` relationships transitively from the package root (`_rels/.rels`), so a media part
reached only through a live slide stays, and OPC-required roots (core properties, thumbnail) stay
because the root relationships name them. Control parts are never removed — `[Content_Types].xml` is
not a part, and every `.rels` part is spared. Reference cycles terminate.

The relationship-resolution logic shared by the reachability walk and the existing reference checks is
unified behind one `resolve_rel` helper (root-vs-part base).

## [0.0.44] - 2026-07-29

Run coalescing (MJX-41, third and last text-model gap — **completing MJX-41**) — formatting a
sub-range with `set_text_range_properties` splits a run, and repeatedly formatting overlapping ranges
leaves a paragraph with more runs than it needs. Nothing merged them back; now an explicit pass does.

New `Presentation::coalesce_paragraph_runs` and `coalesce_shape_runs` merge adjacent runs that would
render identically, returning the number of runs merged away. Two adjacent runs merge only when
**both** hold, so the paragraph reads exactly the same afterwards:

- their **effective** formatting is identical — resolved through the full inheritance ladder, so a run
  that sets a property explicitly merges with a neighbour that inherits the same value (this compares
  meaning, not raw XML); and
- neither carries distinguishing state this model does not describe — a hyperlink, an `rtl`, an
  `a:extLst`, a foreign attribute — so nothing is dropped by the merge (`dirty`/`err`/`smtClean`
  housekeeping is ignored and never blocks a merge).

A line break or field between two runs keeps them apart. When nothing merges, the call changes nothing
and does not dirty the part.

```rust
let merged = pres.coalesce_paragraph_runs(surface, shape, para)?;   // runs removed
let total = pres.coalesce_shape_runs(surface, shape)?;              // across the whole body
```

The supporting pieces are in `mjx-dml`: `CharacterProperties::unmodeled_state_eq` /
`has_only_modeled_state` (the safety gate) and `Paragraph::coalesce_adjacent_runs` (the content-vec
merge). Every part still round-trips byte-for-byte.

## [0.0.43] - 2026-07-29

`a:br` / `a:fld` addressability (MJX-41, second of three text-model gaps) — a line break (`a:br`) and
a text field (`a:fld`) are paragraph children like a run, but until now both fell into the opaque
`Raw` bucket, so a slide-number or date field's text could not be read and a break could not be
located.

Both are now typed: new `TextLineBreak` (`CT_TextLineBreak`, an optional `a:rPr`) and `TextField`
(`CT_TextField` — `@id`/`@type` and optional `a:rPr`/`a:pPr`/`a:t`) fidelity wrappers, added as
`ParagraphContent::LineBreak` / `Field` variants. Following the decision recorded on the issue, they
get **their own accessors** rather than joining the run index space, so this is **non-breaking** —
`runs()`, run indices, and `Paragraph::text()` are unchanged. New `Paragraph::line_breaks()` /
`fields()` (and `_mut`) enumerate them; `TextField::text()` reads the field's cached rendering.

`mjx-pptx` gains a read surface mirroring `run_text`/`run_count` — `paragraph_field_count`,
`paragraph_field_text`, and `paragraph_field_type` on `Presentation` — so a field's cached value and
kind are readable at the format level (a new `PptxError::FieldIndexOutOfRange` reports a bad index).

```rust
let count = pres.paragraph_field_count(surface, shape, para)?;
let text = pres.paragraph_field_text(surface, shape, para, 0)?;   // e.g. "1/27/13"
let kind = pres.paragraph_field_type(surface, shape, para, 0)?;   // e.g. Some("datetimeFigureOut")
```

Every part still round-trips byte-for-byte; reading a field dirties nothing.

## [0.0.42] - 2026-07-29

Underline line/fill groups (MJX-41, first of three text-model gaps) — the underline line group
(`a:uLn` / `a:uLnTx`) and fill group (`a:uFill` / `a:uFillTx`) on a run's `a:rPr` now have a typed
surface, so an underline can be recoloured and restyled independently of the text it sits under.
Previously both were preserved opaquely with no way to read or set them.

Each group is a three-state choice — unset (inherited), *follow text* (the marker element), or an
explicit value — modeled as `UnderlineLine` / `UnderlineFill`. The explicit forms reuse the existing
line and fill models (`LineSpec` for `a:uLn`, `FillSpec` for `a:uFill`), and the two members of a
group are mutually exclusive: writing one replaces the other in place. The groups flow through the
whole run-formatting surface for free — `CharacterPropertiesSpec` builders (`with_underline_line` /
`with_underline_fill`), `merge_under`, `set_text_range_properties`, and `effective_run_properties`
(where the colours are baked like any other fill or outline).

```rust
use mjx_dml::{CharacterPropertiesSpec, ColorSpec, FillSpec, LineSpec, LineWidth, UnderlineFill,
    UnderlineLine};

let spec = CharacterPropertiesSpec::new()
    .with_underline_line(UnderlineLine::Explicit(LineSpec::solid(
        LineWidth::from_points(1.0),
        ColorSpec::Srgb("FF0000".into()),
    )))
    .with_underline_fill(UnderlineFill::FollowText);
```

Additive and non-breaking; every untouched part still round-trips byte-for-byte.

## [0.0.41] - 2026-07-29

Ink (MJX-138, third and last tier of MJX-135) — **preserve-first** recognition of legacy ink (InkML)
content parts, **completing MJX-135**. Handwriting ink is carried as an InkML part
(`/ppt/ink/inkN.xml`, `application/inkml+xml`) referenced from the shape tree by a `p14:contentPart`.
Producers wrap that reference in `mc:AlternateContent` — a shape-tree child in the Markup-Compatibility
namespace that the shape index space cannot reach — so, like VML, ink is recognized by its content type
rather than navigated from a shape. The InkML markup is carried through a round-trip verbatim, not
modeled. Unconditional, like the OLE and ActiveX tiers.

```rust
use mjx_pptx::Presentation;

let deck = Presentation::open(&bytes)?;
for part in deck.ink_part_names() {
    let inkml = deck.ink_part_bytes(&part); // raw InkML, verbatim
}
```

### Added

- **`Presentation::ink_part_names`** — every InkML part in the package, recognized by content type.
- **`Presentation::ink_part_bytes`** — an ink part's bytes, verbatim and non-dirtying.
- Constants `REL_INK` (the shared `customXml` relationship type) and `CONTENT_TYPE_INKML`.

### Scope

Recognition + preserve + a read window only — no authoring, and the ink is not modeled (a typed stroke
surface, trace points → paths, is deferred). Per-shape association (`p14:contentPart@r:id`) and the
`mc:Fallback` snapshot are deferred with it. **MJX-135 (OLE / ActiveX / Ink) is now complete**;
producer-authentic fixture validation across all three tiers is a follow-up (MJX-140).

## [0.0.40] - 2026-07-26

ActiveX controls (MJX-137, second tier of MJX-135) — **preserve-first** recognition of legacy ActiveX
form controls. Unlike an OLE object (a graphic frame in the shape tree), a control lives in
`p:cSld > p:controls > p:control` — beside the shape tree — so it is addressed per-slide by a control
index. Its persisted state is a **two-hop** chain: `p:control@r:id` names the control part
(`/ppt/activeX/activeXN.xml`, `ax:ocx` markup), which in turn relates to its binary blob
(`/ppt/activeX/activeXN.bin`). The control markup, its binary, and its fallback snapshot image are each
carried through a round-trip verbatim, none modeled. Unconditional, like OLE.

```rust
use mjx_pptx::Presentation;

let mut deck = Presentation::open(&bytes)?;
for i in 0..deck.activex_control_count(slide)? {
    let name = deck.activex_control_name(slide, i)?;          // e.g. "CommandButton1"
    let ocx = deck.activex_part_bytes(slide, i)?;             // ax:ocx markup, verbatim
    let blob = deck.activex_binary_bytes(slide, i)?;          // persisted state (.bin), two-hop
    let snapshot = deck.activex_snapshot_image_bytes(slide, i)?; // fallback image for rendering
}
```

### Added

- **`Presentation::activex_control_count`** — the number of ActiveX controls on a surface.
- **`Presentation::activex_control_rel_id` / `activex_control_name`** — a control's control-part
  relationship id and its declared `name`.
- **`Presentation::activex_part_bytes`** — the `ax:ocx` control part's verbatim bytes.
- **`Presentation::activex_binary_bytes`** — the control's binary blob, resolved across the two-hop
  `activeXControlBinary` chain.
- **`Presentation::activex_snapshot_rel_id` / `activex_snapshot_image_bytes`** — the fallback snapshot
  image a renderer draws in place of the (never-executed) control.
- Constants `REL_CONTROL`, `REL_ACTIVEX_CONTROL_BINARY`, `CONTENT_TYPE_ACTIVEX`,
  `CONTENT_TYPE_ACTIVEX_BINARY`.

### Scope

Recognition + preserve + a read window only — no authoring, and the control is not modeled (opaque
`ax:ocx` markup + binary state). The last MJX-135 tier is ink (MJX-138); producer-authentic fixture
validation is a follow-up (MJX-140).

## [0.0.39] - 2026-07-26

OLE objects (MJX-136, first tier of MJX-135) — **preserve-first** recognition of legacy embedded OLE
objects. An OLE object is an embedded document (a legacy `.xls`/`.doc` or an OLE `.bin` stream)
referenced from a `p:graphicFrame` via `p:oleObj@r:id`, drawn from a fallback image snapshot. Such a
frame previously surfaced only as the opaque `GraphicFrameKind::Other`; it now reads as `OleObject`,
with accessors for the embedded object's bytes and the snapshot image — both carried through a
round-trip verbatim, neither modeled. Unlike VML this is **not** feature-gated: an OLE frame is ordinary
PresentationML, so it mirrors the (unconditional) chart surface.

```rust
use mjx_pptx::{GraphicFrameKind, Presentation};

let mut deck = Presentation::open(&bytes)?;
if deck.graphic_frame_kind(slide, shape)? == Some(GraphicFrameKind::OleObject) {
    let prog = deck.ole_prog_id(slide, shape)?;               // e.g. "Excel.Sheet.12"
    let object = deck.ole_object_part_bytes(slide, shape)?;   // embedded object, verbatim
    let snapshot = deck.ole_snapshot_image_bytes(slide, shape)?; // fallback image for rendering
}
```

### Added

- **`GraphicFrameKind::OleObject`** — a graphic frame framing a `p:oleObj` (refines the former `Other`).
- **`Presentation::ole_object_rel_id` / `ole_object_part_bytes`** — the embedded object's relationship
  and its verbatim bytes (`/ppt/embeddings/oleObjectN.bin` or an embedded package).
- **`Presentation::ole_snapshot_rel_id` / `ole_snapshot_image_bytes`** — the fallback snapshot image a
  renderer draws in place of the (never-executed) object.
- **`Presentation::ole_prog_id`** — the owning application's `progId`.
- Constants `REL_OLE_OBJECT`, `REL_PACKAGE`, `CONTENT_TYPE_OLE_OBJECT`.

### Scope

Recognition + preserve + a read window only — no authoring, and the embedded object is not modeled
(it is an opaque OLE stream or embedded document). The `p:oleObj` is reached through its
`mc:AlternateContent` wrapper (preferring the `mc:Choice` branch) by a bounded structural descent, not
by running full MCE resolution. The remaining MJX-135 tiers are ActiveX controls (MJX-137) and ink
(MJX-138); producer-authentic fixture validation is a follow-up (MJX-140).

## [0.0.38] - 2026-07-26

VML, tier V1 (MJX-115) — **preserve-first** legacy VML round-trip. VML is the Transitional-only drawing
markup producers still emit for OLE-object fallbacks, comment shapes, ink and legacy controls, carried
as standalone `vmlDrawingN.vml` parts. Such parts already round-trip byte-identically through the
generic part-level copy-on-write; this release adds a **recognition surface** so callers can find and
read them — behind the new `vml` crate feature (opt-in, off by default). The VML XML is **not modeled**.

```rust
// with `mjx-pptx` (or `mjx-ooxml`) built with the `vml` feature
let deck = Presentation::open(&bytes)?;
for part in deck.vml_part_names() {
    let xml = deck.vml_part_bytes(&part); // raw legacy VML, verbatim
}
```

### Added

- **`mjx-vml`** becomes real (was a scaffold stub): the VML vocabulary — `CONTENT_TYPE_VML`,
  `REL_VML_DRAWING`, `VML_DEFAULT_EXTENSION` — and an `is_vml_content_type` recognition predicate.
- **`Presentation::vml_part_names`** / **`vml_part_bytes`** (behind the `vml` feature) — enumerate the
  legacy VML drawing parts a package carries (by content type, so VML referenced from any part is
  found) and read a part's bytes verbatim, without dirtying anything.
- **`vml` Cargo feature** on `mjx-pptx` (the repo's first), re-exposed by the `mjx-ooxml` facade.

### Scope

Preserve-first only: VML is recognized and readable as raw bytes, never parsed or modeled, and never
authored. Recognition is package-level (content type), not yet shape/relationship-level — the OLE /
ActiveX / ink references that cite a specific VML shape are the next tier (MJX-135), and Word-side
legacy VML (`w:pict`, header/footer fallback) is tracked under the Word slice (MJX-139). The fixture is
hand-crafted; validation against genuine producer decks is a follow-up (MJX-140). This completes the
chart + VML arc (MJX-47) except for the chart embedded workbook (MJX-116).

## [0.0.37] - 2026-07-25

Charts, tier C4 (MJX-114) — **authoring** a brand-new chart. C0–C3 recognized, modeled and edited an
existing chart; this release creates one from scratch. A chart is described fluently with `ChartData`
(a kind, shared categories, named series) and added to a slide with `Presentation::add_chart`, which
writes a new chart part (`ppt/charts/chartN.xml`) and a `p:graphicFrame` that references it. All six
kinds are supported: bar, line, area, pie, doughnut and scatter.

```rust
use mjx_pptx::{ChartData, ChartKind, ShapeBounds};

let chart = ChartData::new(ChartKind::Bar)
    .categories(["Q1", "Q2", "Q3"])
    .series("Revenue", [10.0, 20.5, 15.0])
    .series("Cost", [5.0, 8.0, 7.25]);
let shape = deck.add_chart(slide, &chart, ShapeBounds::from_inches(1.0, 1.0, 6.0, 4.0))?;
```

### Added

- **`Presentation::add_chart`** — authors a chart on a surface from a `ChartData`, returning its shape
  index. Creates the chart part with its `CONTENT_TYPE_CHART` Override and a `REL_CHART` relationship
  from the slide; every pre-existing part stays byte-identical.
- **`ChartData`** (re-exported from `mjx-pptx`, alongside `ChartKind`) — a fluent builder
  (`new(kind).categories(...).series(name, values)`) that serializes a complete `c:chartSpace` part.
- Error **`InvalidChartData`** — a chart with no series (or only empty series) is refused at creation.

### Scope

Authoring writes **cached data only** (`c:strCache`/`c:numCache`, with synthesized `c:f` formulas so
the references are schema-valid) and **no embedded workbook**: the chart renders everywhere from its
cache, while PowerPoint's "Edit Data" is degraded until the embedded-workbook follow-up (MJX-116).
Scatter's shared categories become numeric X values, falling back to the point position for a
non-numeric label. This completes the chart arc except for VML (V1) and the embedded workbook.

## [0.0.36] - 2026-07-25

Charts, tier C3 (MJX-113) — the first **mutating** chart tier. C1/C2 modeled a chart read-only; this
release rewrites a series' cached values and category labels (`c:numCache` / `c:strCache`) on an
existing chart, through a `mjx-pptx` surface. The cached values are what **render**; a chart's
embedded workbook is **not** rewritten and goes stale (a separate follow-up). Only the edited chart
part is dirtied — every other part, the embedded workbook included, is left byte-identical.

```rust
// read the series, then rewrite the first series' values
for s in deck.chart_series(slide, shape)? {           // name, categories, values per series
    println!("{:?}: {:?} = {:?}", s.name, s.categories, s.values);
}
deck.set_chart_series_values(slide, shape, 0, &[1.0, 2.5, 3.0])?;
deck.set_chart_series_categories(slide, shape, 0, &["Q1", "Q2", "Q3"])?;
```

### Added

- **`Presentation::chart_series`** — each series of a chart as a `ChartSeriesData` (`name`,
  `categories`, `values`; a scatter series' `xVal`/`yVal`), flattened across the chart's plots.
  Non-dirtying.
- **`Presentation::set_chart_series_values` / `set_chart_series_categories`** — rewrite the cached
  data of the `series_idx`-th series (0-based across the plots), dirtying only the chart part.
  `set_chart_series_values` targets `c:val`, or a scatter series' `c:yVal`.
- **`ChartSeriesData`** — the read DTO.
- **`mjx-chart` mutation** — `NumberCache::set_values` / `StringCache::set_labels`, the
  reference/data-source `set_values`/`set_labels`, `Series::set_values`/`set_categories`, and the
  mutable navigation (`series_mut`, `all_series_mut`, `ChartSpace::series_mut`/`series_count`).
- Errors **`ShapeIsNotAChart`**, **`ChartSeriesOutOfRange`**, **`ChartSeriesNotEditable`**.

### Fidelity

A cache edit rebuilds only its `c:pt` points and its `c:ptCount`; the `c:formatCode` and everything
outside the edited cache (the axes, other series, styling) survive verbatim. A rewritten number is
formatted with Rust's shortest round-trip representation (the exact inverse of the read parse); a
non-finite value, which has no valid spelling, is skipped. `mjx-pptx` gains a dependency on
`mjx-chart` (both shared-markup/format tiers, cycle-free).

## [0.0.35] - 2026-07-25

Charts, tier C2 (MJX-112) — the remaining common plot types. C1 modeled the bar plot; this release
extends the same read-only, byte-identical model to **line** (`c:lineChart`), **pie** (`c:pieChart`),
**area** (`c:areaChart`), **scatter** (`c:scatterChart`) and **doughnut** (`c:doughnutChart`), and to
**combo charts** — a `c:plotArea` may legitimately hold more than one plot.

```rust
use mjx_ooxml_core::FromXml;

let doc = mjx_xml::fidelity::parse(chart_part_bytes)?;
let space = mjx_chart::ChartSpace::from_xml(&doc.root, &doc.interner)?;
for kind in space.chart_kinds() {          // e.g. [Bar, Line] for a combo chart
    println!("{kind:?}");
}
if let Some(scatter) = space.plot_area().and_then(|p| p.scatter_chart()) {
    for series in scatter.series() {
        let xs = series.x_data().map(|x| x.values());   // c:xVal, not c:cat
        let ys = series.y_data().map(|y| y.values());   // c:yVal, not c:val
    }
}
```

### Added

- **Plot types** — `LineChart`, `PieChart`, `AreaChart`, `ScatterChart`, `DoughnutChart` alongside the
  existing `BarChart`, each with `series()`/`series_at()`/`series_count()`/`kind()`. `ChartKind` gains
  `Line`, `Pie`, `Area`, `Scatter`, `Doughnut`.
- **`PlotArea` accessors** — `line_chart()`/`pie_chart()`/`area_chart()`/`scatter_chart()`/
  `doughnut_chart()` beside `bar_chart()`; `chart_kinds()` (one entry per plot, for combo charts) and
  `all_series()` (every plot's series, flattened). `ChartSpace::chart_kinds()` mirrors it.
- **Scatter data** — `Series::x_data()` (`c:xVal`) and `y_data()` (`c:yVal`), plus
  `CategoryData::values()` (the numeric companion to `labels()`), for the one series type that carries
  X/Y data instead of `c:cat`/`c:val`.

### Fidelity

Each plot type is its own struct but they share one `Series` type and one `PlotContent` bucket; every
plot preserves its own element name and buckets its type-specific scalars (`barDir`, `grouping`,
`firstSliceAng`, `holeSize`, `scatterStyle`) and axes into `Raw`, so a chart of any modeled type — or
a combo — round-trips byte-for-byte. Unmodeled plot types (radar, bubble, 3-D, …) ride through `Raw`.

## [0.0.34] - 2026-07-25

Charts, tier C1 (MJX-111) — the chart XML gets a typed home. C0 recognized a chart frame and handed
back the chart part's raw bytes; this release **models** that part in `mjx-chart` (until now a
scaffold stub). It derives the chart-space spine `c:chartSpace → c:chart → c:plotArea` and one plot
type end to end — the bar/column plot (`c:barChart` / `c:ser` / `c:cat` / `c:val`) — with read-only
accessors for a chart's kind, its series, and each series' category labels and values, read down
through the `c:strCache` / `c:numCache`.

```rust
use mjx_ooxml_core::FromXml;

let doc = mjx_xml::fidelity::parse(chart_part_bytes)?;      // the /ppt/charts/chartN.xml bytes
let space = mjx_chart::ChartSpace::from_xml(&doc.root, &doc.interner)?;
if let Some(bar) = space.bar_chart() {                       // c:chart → c:plotArea → c:barChart
    for series in bar.series() {
        let name = series.name();                            // "Sales", from c:tx
        let labels = series.categories().map(|c| c.labels()); // ["North", "South", "West"]
        let values = series.values().map(|v| v.values());     // [19.2, 21.4, 16.7]
    }
}
```

### Added

- **`mjx-chart` chart model** — `ChartSpace` (`c:chartSpace`), `Chart`, `PlotArea`, `BarChart`,
  `Series`, and the data layer (`NumericData`/`CategoryData`/`SeriesText`,
  `NumberReference`/`StringReference`, `NumberCache`/`StringCache`, `DataPoint`, `Value`, `Formula`),
  each parsed with `FromXml` and re-emitted byte-for-byte with `ToXml`.
- **Read accessors** — `ChartSpace::{chart, plot_area, bar_chart, chart_kind}`;
  `BarChart::{series, series_at, series_count, direction, grouping}`;
  `Series::{name, categories, values, index, order}`; `CategoryData::labels`, `NumericData::values`,
  and the underlying reference/cache/point accessors. Chart kinds are the extensible `ChartKind`
  enum; a bar plot's `BarDirection` and `BarGrouping` are typed.

### Fidelity

Every modeled container keeps an ordered `content` list of typed children plus a `Raw` catch-all
(mirroring the `mjx-dml` table model), so the axes, text properties, an external-data reference, a
literal data source or an `extLst` this tier does not interpret round-trip byte-for-byte. A cached
value is parsed on demand from its point's preserved wire text — never reformatted on write.

### Scope

Read-only, bar plot only. Cached data (`c:numCache` / `c:strCache`) is the read path; a literal
source (`c:numLit` / `c:strLit`) or a multi-level category rides through the `Raw` bucket for now.
Other plot types are tier C2; editing (C3) and authoring (C4) are later tiers.

## [0.0.33] - 2026-07-25

Charts, tier C0 (MJX-47) — the first step of the chart workstream. A `p:graphicFrame` that frames a
chart (its `a:graphicData@uri` is the chart URI and its payload is a `c:chart`) points at a **separate
part** (`/ppt/charts/chartN.xml`) by relationship id, unlike a table, whose `a:tbl` is inline. This
release recognizes such a frame, resolves that relationship, and reads the chart part's bytes — the
chart XML itself is not modeled yet; it and its satellites (an embedded workbook, colour and style
parts) are carried through a round-trip **verbatim**.

```rust
if deck.graphic_frame_kind(slide, shape)? == Some(GraphicFrameKind::Chart) {
    let rel = deck.chart_rel_id(slide, shape)?;        // the slide relationship the frame names
    let xml = deck.chart_part_bytes(slide, shape)?;    // the /ppt/charts/chartN.xml bytes, borrowed
}
```

### Added

- **`Presentation::chart_rel_id`** — the relationship id a chart frame names
  (`p:graphicFrame > a:graphic > a:graphicData > c:chart@r:id`), or `None` for any shape that frames
  no chart. The `c:chart` element is looked for rather than the frame's `uri` trusted — the payload
  decides. Reading is non-dirtying.
- **`Presentation::chart_part_bytes`** — the raw XML of the chart part that frame references, borrowed
  from the package exactly as stored (never re-serialized), or `None` when the shape frames no chart.
  The read window onto a chart until `mjx-chart` models it.
- **`constants::REL_CHART` and `constants::CONTENT_TYPE_CHART`** — the chart relationship type and the
  chart part's content type, for the authoring tiers to come.
- A `tests/fixtures/charts.pptx` fixture (two slides, one clustered-column chart with an embedded
  workbook) and integration tests proving a chart deck round-trips byte-identically, that reading a
  chart dirties nothing, and that editing another slide leaves every chart part untouched.

### Changed

- The private `image_part_for_rel` helper is generalized to `part_for_rel` (it resolves any
  relationship id to its part), now shared by the image and chart read paths.

## [0.0.32] - 2026-07-24

DrawingML 3-D, part 3 (MJX-49 D4) — and with it the 3-D workstream is complete. `Cell3D`
(`CT_Cell3D`, a table cell's 3-D corner), until now a fidelity wrapper that kept its `a:bevel` /
`a:lightRig` opaque, becomes the **first consumer** of the typed model: it reads and authors them
through the same `Bevel` / `LightRig` the shape surface uses.

```rust
// a header row whose cells stand up in metal, bevelled and lit
deck.format_table_style_part(style_id, TableStylePart::FirstRow,
    &TableStyleFormat::new()
        .with_cell_material(PresetMaterial::Metal)
        .with_cell_bevel(Bevel { width: Some(Emu::from_emu(76_200)), ..Bevel::default() })
        .with_cell_light_rig(LightRig { rig: LightRigType::ThreePoint, direction: LightRigDirection::Top, rotation: None }))?;
```

### Added

- **`mjx-dml`: `Cell3D` decomposed** — typed `material()` (a `PresetMaterial`, alongside the retained
  raw `preset_material()`), `bevel()` and `light_rig()` accessors, and authoring via `Cell3D::new`
  (seeded with the schema-required empty bevel) + `set_material` / `set_bevel` / `set_light_rig`, with
  `TableStyleCellStyle::set_cell_3d` placing the child at its schema rank. The `a:bevel` / `a:lightRig`
  wire helpers are shared with the shape surface; `extLst` stays opaque, so a `cell3D` still
  round-trips byte-for-byte.
- **`mjx-pptx`: cell-3-D on the table-style builder** — `TableStyleFormat::with_cell_material`,
  `with_cell_bevel` and `with_cell_light_rig` give a styled part's cells an `a:cell3D`, applied
  through the shared and inline `tableStyles` paths alike.

## [0.0.31] - 2026-07-24

DrawingML 3-D, part 2 (MJX-49 D3) — the `mjx-pptx` shape surface. The typed 3-D model from 0.0.30
gains its `Presentation` accessors, a 1:1 mirror of the shape-effects surface (E3): a shape's 3-D
scene and its own 3-D properties are now readable, writable and clearable, on a group member as on a
top-level shape.

```rust
deck.set_shape_scene_3d(0, shape, &Scene3DSpec { camera, light_rig })?;   // how it is lit and viewed
deck.set_shape_3d_properties(0, shape, &Shape3DSpec { extrusion_height, bevel_top, .. })?;
deck.shape(0, shape)?.scene_3d(scene).shape_3d_properties(props).apply()?; // or fluently, one commit
```

### Added

- **`Presentation::shape_scene_3d` / `set_shape_scene_3d` / `clear_shape_scene_3d`** — a shape's
  `p:spPr > a:scene3d` (`CT_Scene3D`) as an interner-free [`Scene3DSpec`]. Reading is non-dirtying and
  returns `None` when the shape is flat (3-D has no inheritance chain) or the scene omits a
  schema-required camera/light rig. Setting rebuilds the element in `CT_ShapeProperties` order — after
  any fill, outline and effects, before `a:sp3d`. Clearing **removes** the element (an empty
  `a:scene3d` would be schema-invalid), a no-op when absent.
- **`Presentation::shape_3d_properties` / `set_shape_3d_properties` / `clear_shape_3d_properties`** —
  a shape's `p:spPr > a:sp3d` (`CT_Shape3D`: extrusion, contour, bevels, material, edge colors) as a
  [`Shape3DSpec`]. `a:sp3d` is the last visual property, so it lands after everything else and before
  any `a:extLst`. An unstated attribute reads `None`, not the schema default.
- **`ShapeCursor::scene_3d` / `clear_scene_3d` / `shape_3d_properties` / `clear_shape_3d_properties`**
  — the same edits recorded on the fluent cursor, applied in one commit alongside fill/outline/effects.
- All six flat methods and the cursor take `impl Into<ShapePath>`, so a group member is addressed the
  same as a top-level shape.

## [0.0.30] - 2026-07-24

DrawingML 3-D, part 1 of the workstream (MJX-49 D1+D2) — the `a:scene3d` / `a:sp3d` subsystem, until
now round-tripped opaquely, gains a typed model. Mirrors the effects/outline/fill workstreams:
generated preset enums, then the `mjx-dml` value types and fidelity wrappers. The `mjx-pptx` shape
surface (D3) and the `Cell3D` upgrade (D4) follow.

### Added

- **`mjx-ooxml-types::drawingml`** — five generated preset enums: `BevelPreset` (12),
  `LightRigType` (27), `LightRigDirection` (8), `PresetMaterial` (15) and `PresetCamera` (62). Each
  cryptic token is expanded to a self-explanatory name sourced from the ECMA-376 token (the light
  direction's compass abbreviations, `threePt`/`twoPt`, `dkEdge`, `softmetal`).
- **`mjx-dml`: the 3-D model** — value types `Bevel`, `SphereCoordinates`, `Camera` (preset view +
  field of view + zoom + rotation) and `LightRig`; fidelity wrappers `Scene3D` (`CT_Scene3D`) and
  `Shape3D` (`CT_Shape3D`) with typed accessors and interner-free `Scene3DSpec` / `Shape3DSpec`. The
  camera and light rig, and a shape's bevels, extrusion/contour colors and material, are read typed;
  the rarer `a:backdrop` and any `extLst` stay opaque, so an element round-trips byte-for-byte. Every
  measure is `Option`, so an unstated attribute reads `None`, not the schema default.

## [0.0.29] - 2026-07-24

Group descent, part 4 — **group structure**, and with it the group workstream is complete. A
`p:grpSp` is now addressable, measurable, editable *and* something a caller can make, dissolve and
move shapes through.

```rust
let group = deck.group_shapes(0, &[1.into(), 2.into()])?;  // select these, group them
deck.move_shape_into_group(0, 3, &group)?;                 // and take that one too
deck.set_shape_fill(0, group.child(0), &navy)?;
```

### Added

- **`Presentation::group_shapes(surface, members)`** — wraps sibling shapes in a new group, returning
  its [`ShapePath`]. The group's box is the union of the members' own boxes (how ECMA-376 Part 1
  §L.4.7.4 defines a child bounding box) and its child space is set **identical** to it, so the
  mapping is the identity: the members keep their coordinates exactly, with no rounding anywhere. The
  group takes the earliest member's z-order slot, and the members keep their relative order inside it
  whatever order they were named in.
- **`Presentation::ungroup(surface, group)`** — dissolves a group, returning where its members now
  are. Each keeps its absolute placement, the group's mapping unwound into its own transform.
- **`Presentation::move_shape_into_group` / `move_shape_out_of_group`** — move one shape one level,
  in or out. The shape does not move on screen: its transform is restated for its new coordinate
  system, **mirrors and rotation included**, so joining a scaled, turned or flipped group leaves it
  exactly where it was.
- **`ShapeCursor::into_group` / `out_of_group` / `group_with` / `ungroup`** — the same, said mid-chain
  and following the shape. Each is a **commit point**: it writes what has been recorded so far,
  performs the change, then re-anchors, so no recorded edit is ever applied against a tree it was not
  recorded against.
- **`ShapePath::child` / `parent`** — step down to a member or up to the enclosing group, which is
  how the group returned by `group_shapes` is addressed.
- **`ShapeBounds::union`** — the smallest rectangle containing both.
- `PptxError::GroupNeedsTwoShapes`, `ShapesAreNotSiblings`, `ShapeCannotContainItself`,
  `ShapeHasNoBounds`.

### Notes

There is deliberately **no empty-group constructor**. §L.4.7.4 records that a group with no shapes is
degenerate and produces no visible output, and one with a single shape "has no representational power
beyond that of the one shape" — and an empty group has no honest `chOff`/`chExt` to be given. Every
group these create is well-formed by construction.

## [0.0.28] - 2026-07-24

Group descent, part 3 — a group member now says **where it is on the slide**. Addressing it, styling
it and measuring it are finally the same three things they are for a top-level shape.

### Changed

- **`Presentation::shape_bounds` answers in absolute slide EMU for a group member**, composing every
  enclosing group's child coordinate space instead of returning the member's raw `a:off` / `a:ext`.
  `set_shape_bounds` takes the same absolute rectangle and maps it back, so read and write stay in
  one space. This is a **behaviour change** for nested addresses only — a top-level shape reads and
  writes exactly as before, because composing over no ancestors is the identity. `shape_transform` /
  `set_shape_transform` are untouched and remain the accessors for what the file literally states, in
  the shape's own space. `effective_shape_bounds` / `effective_shape_transform` compose too, after
  resolving placeholder inheritance; the latter is where the composed rotation and mirror flags are
  read, since an axis-aligned `ShapeBounds` cannot hold a rotation.
- The shape cursor's `.bounds(…)` is slide-absolute to match, and runs the same conversion;
  `.transform(…)` still writes verbatim in the shape's own space.

### Added

- **`mjx-dml`: `Transform2D::child_scale` / `child_to_parent` / `parent_to_child`** — one rung of the
  mapping between a group's child coordinate space and its parent's, and its exact inverse.
- `PptxError::ShapeCannotBePlaced`, when an enclosing group states no `a:chOff` / `a:chExt`: there is
  then no mapping to invert, so the member reads as unplaced and the write is refused rather than
  putting the shape somewhere wrong.

### Notes

The composition follows **ECMA-376 Part 1 §L.4.7.4**, not the naive "apply each ancestor transform in
turn": a nested shape is scaled and mirrored by the *product* of its ancestors' factors, rotated by
their *sum*, and translated so its **centre** lands where the whole chain — rotations included — puts
it. A mirrored or rotated group therefore places its members correctly, which composing corners would
not. Round-tripping `set_shape_bounds(shape_bounds(…))` is exact whenever the groups' scales are, and
within a few EMU — millionths of an inch — when they are not.

## [0.0.27] - 2026-07-24

Group descent, part 2 — the **shape cursor**: a shape is addressed once and edited fluently, and a
group is restyled in one expression.

```rust
deck.shape(0, 2)?                                  // the group at top-level index 2
    .effects(shadow)
    .member(0)?.fill(navy).outline(rule)           // its first member
    .sibling(1)?.fill(gold).text("Q3").all_run_properties(bold)
    .apply()?;                                     // one write pass, one dirty part
```

### Added

- **`Presentation::shape(surface, path)` → `ShapeCursor`** — the ergonomic layer over the
  `set_shape_*` methods. Edit methods record intent and return the cursor; `.apply()` consumes it,
  writes every edit in the order it was recorded, and marks the part dirty once. A cursor that is
  never applied changes nothing, so it is `#[must_use]`. Every edit it records is executed by the
  code the mirrored flat method calls — a cursor is a way of *saying* the edits, not a second way of
  doing them.
- **Moving through a group** — `.member(i)` descends into a `p:grpSp`, `.sibling(i)` moves to another
  shape in the same container, `.parent()` steps back out; `.kind()`, `.member_count()` and `.path()`
  say where the cursor is. Each move checks the address as it lands, so a bad one fails where it was
  written. Recorded edits stay bound to the address they were recorded at, so one `.apply()` commits
  work spread over a group and its members.
- **What a cursor records** — the `p:spPr` surface (`fill` / `no_fill`, `outline` / `no_outline`,
  `effects` / `no_effects`, `geometry`, `bounds`, `transform`), `text`, the text-formatting specs
  (`run_properties`, `paragraph_run_properties`, `all_run_properties`, `end_run_properties`,
  `paragraph_properties`, `text_range_properties` and its `_by_grapheme` sibling), the shape's own
  `hyperlink` / `clear_hyperlink`, and a picture's `image`. Hyperlinks on a *run* or a text range are
  addressed by paragraph and run and stay on the flat API.
- **`Presentation::set_shape_text_content(surface, shape, text)`** — replaces a shape's whole text
  with one paragraph per line, each holding one run, so `shape_text` reads back exactly what was
  written. Only the paragraphs are swapped: the body's own `a:bodyPr` and `a:lstStyle` survive, so
  restating a placeholder's text does not disturb how it is laid out.
- **`Presentation::shape_member_count(surface, shape)`** — how many members a group holds (`0` for
  anything that is not a group).
- `PptxError::ShapeIsNotAGroup` and `PptxError::ShapeHasNoParent`, the two ways a cursor move is
  refused.

### Changed

- The per-shape element edits (fill, outline, effects, preset geometry, a picture's blip) moved into
  `slide.rs` as primitives taking an already-resolved shape, so the flat setters and the cursor share
  one implementation each. No behaviour change.

## [0.0.26] - 2026-07-22

Group descent, part 1 — shapes inside a `p:grpSp` are now addressable. Every shape API takes an
address as `impl Into<ShapePath>`: a bare index is a top-level shape (unchanged), and an array
`[group, member, …]` descends into nested groups. A group member can be read, edited and removed
exactly like a top-level shape; `shape_count` still counts only the top level, and a member's
`shape_bounds` are its own `a:off`/`a:ext` in the group's child space (the absolute-rectangle mapping
lands in a later atom). `PptxError::ShapeIndexOutOfRange` now carries the requested `ShapePath` and
the shape count of the container where the address ran out of range.

## [0.0.25] - 2026-07-22

Hyperlinks on runs and shapes — set, read, and clear links, external URLs and slide jumps.

### Added

- **`Hyperlink`** — a resolved link: `Hyperlink::Url(String)` (an external target) or
  `Hyperlink::Slide(usize)` (a jump to another slide in the deck). The relationship indirection
  (`r:id` → external URL or internal slide part) stays inside `Presentation`.
- **`Presentation::run_hyperlink` / `set_run_hyperlink` / `clear_run_hyperlink`** — the click
  hyperlink on a run, read back as a `Hyperlink`; setting adds its relationship (creating the run's
  `a:rPr` if absent), clearing removes the relationship once nothing else in the part still names it.
- **`Presentation::set_text_range_hyperlink`** — links a scalar range, splitting runs at the
  boundaries so exactly the selected text carries the link (one shared relationship).
- **`Presentation::shape_hyperlink` / `set_shape_hyperlink` / `clear_shape_hyperlink`** — the same on
  a shape's own `p:cNvPr > a:hlinkClick`.
- `mjx-dml`: `CharacterProperties` and `TextRun` gain `hyperlink_rel_id` / `set_hyperlink` (the raw
  `a:hlinkClick` accessors the packaging layer drives).

## [0.0.24] - 2026-07-22

Speaker notes, part 2 — the ergonomic notes surface: read, set, and clear a slide's notes.

### Added

- **`Presentation::notes_text(slide)`** — the speaker notes of a slide, read from its notes slide's
  `body` placeholder by kind (the caller never needs the shape index); `None` when the slide has no
  notes.
- **`Presentation::set_notes_text(slide, text)`** — sets the notes, **creating the notes slide on
  demand** (and, when the deck has none, **synthesizing a notes master** for it to follow) with its
  relationships and content-type overrides. Creating a notes slide adds exactly that part, its
  `.rels`, the slide → notes-slide relationship and the override — every pre-existing part stays
  byte-identical.
- **`Presentation::clear_notes(slide)`** — removes a slide's notes slide (and its `.rels` and
  override); the shared notes master and the slide survive. A no-op when the slide has no notes.

This completes MJX-34 — the last feature before the `v0.1` PowerPoint milestone.

## [0.0.23] - 2026-07-22

Speaker notes, part 1 — a notes slide and the notes master become addressable surfaces.

### Added

- **`Surface::Notes(slide)`** and **`Surface::NotesMaster`** — a slide's notes slide carries the same
  `p:cSld > p:spTree` a slide does, so every existing shape, text, fill, outline, effect, transform and
  table method now works on it unchanged, addressed by the slide it belongs to. `Surface::NotesMaster`
  addresses the single notes master every notes slide inherits from.
- Notes text inherits from the notes master's **`p:notesStyle`** exactly as slide text inherits from a
  slide master's `p:txStyles`; `color_map` and `theme` resolve through the notes master too.

### Changed

- `Presentation::surface_part` now returns an owned `PartName` (a notes part is resolved lazily by
  relationship, not stored), simplifying every call site that previously cloned the borrow.

## [0.0.22] - 2026-07-22

Author inline table styles — a lean, self-contained styling path.

### Added

- **`Presentation::set_inline_table_style(surface, shape, &TableStyleDefinition)`** — gives a table its
  own **inline** `a:tableStyle`, replacing any inline or referenced style. The whole look is declared
  up front and travels with the table: no shared `tableStyles.xml` part, relationship, content-type or
  referenced GUID. Plus the incremental **`format_inline_table_style_part`**.
- **`TableStyleDefinition`** — a declarative builder (`with_name` / `with_id` / `with_part`) reusing
  `TableStyleFormat`; the vestigial `styleId` / `styleName` default.
- **`TableProperties::set_inline_style`** (`mjx-dml`) — writes the style as `a:tableStyle` at its rank,
  replacing any `a:tableStyle` / `a:tableStyleId`.

### Notes

- The style resolves and renders through the existing `with_table_style` and `effective_cell_*`
  readers exactly as a shared one does — an inline style is the same `CT_TableStyle`, spelled out on
  the table.
- **Flags stay the caller's job**: a styled part renders only when its `a:tblPr` flag is on
  (`set_table_part`; `add_table` sets `firstRow`/`bandRow`).

## [0.0.21] - 2026-07-22

Table gaps closed — merge-aware formatting, inline styles, accessibility headers, and more.

### Added

- **`Presentation::cell_headers` / `set_cell_headers`** — the accessibility header associations of a
  cell (`a:tcPr > a:headers`), plus `TableCellProperties::headers` / `set_headers` and `TableCell::id`
  in `mjx-dml`.
- **`Presentation::visible_cell_text`** — the text that renders at a position: the cell's own, or its
  merge anchor's when it is covered.
- **`Presentation::graphic_frame_kind`** returning **`GraphicFrameKind`** (`Table` / `Chart` /
  `Diagram` / `Other`) — tells "not a table" from "a graphic not modeled yet"; a chart or diagram
  frame still answers `ShapeIsNotATable` to the table methods.
- **`TableProperties::inline_style`** (`mjx-dml`) — reports a style defined **inline** on the table
  (`a:tableStyle`); `with_table_style` and the effective-formatting resolvers now resolve an inline
  style as well as a referenced one.

### Changed

- **Formatting a cell selection is merge-aware**: `format_cells` / `format_cell_text` /
  `format_cell_paragraphs` skip merge-covered cells (which render nothing), so unmerging restores a
  covered cell's own formatting. Merging and unmerging still reach covered cells; single-cell methods
  addressed by `(row, column)` are unchanged.

## [0.0.20] - 2026-07-22

Effective cell formatting — what a table cell actually renders as. Closes the tables workstream.

### Added

- **`applicable_parts`** and **`TableStyleFlags`** (`mjx-dml`) — the style parts that cover a cell,
  most specific first, per the ECMA-376 §17.7.6 layering (corner cells > first/last column >
  first/last row > row bands > column bands > `wholeTbl`), with banding over data cells only.
- **`Presentation::effective_cell_fill` / `effective_cell_border`** — the fill or border a cell
  renders, resolving the cell's own `a:tcPr`, then the applicable style parts (explicit or a theme
  `fillRef`/`lnRef`), then the theme, colours baked to concrete `RRGGBB`. A border takes the outer
  edge for a rim cell and the interior edge (`insideH`/`insideV`) for one within the table.
- **`Presentation::effective_cell_run_properties`** — a cell's text run resolved down a
  table-specific ladder: the run's own `a:rPr`, the paragraph default, the table style's `a:tcTxStyle`
  for each applicable part (bold / italic / colour), then the presentation `p:defaultTextStyle`.

### Notes

- This is what the modeled `tableStyles.xml` exists for: everything before reported what a file
  *states*; this resolves what a renderer would show. An explicit property on the cell always wins.
- Reading resolves nothing into the file — every effective read leaves the package byte-identical.

## [0.0.19] - 2026-07-22

The `tableStyles.xml` part is modeled, and table styles can be authored and resolved.

### Added

- **The table-style model** (`mjx-dml`) — `TableStyleList`, `TableStyle`, the thirteen part slots
  (`TableStylePart`) plus `tblBg`, and the `TablePartStyle` / `TableStyleTextStyle` /
  `TableStyleCellStyle` / `TableCellBorderStyle` / `TableBackgroundStyle` / `FontReference` / `Cell3D`
  leaves. Every accessor reuses the DrawingML already modeled (fills, `LineProperties`, `Color`,
  `EffectList`, theme references via `StyleMatrixReference`). Two new generated types: the tri-state
  `OnOffStyle` (`on`/`off`/`def`) and `FontCollectionIndex`. `Cell3D`'s `a:bevel`/`a:lightRig` are
  preserved opaque pending the 3-D workstream.
- **Authoring the style tree** (`mjx-dml`) — constructors and setters that build a style from parts
  (fill, borders, text emphasis), each merge-not-rebuild and default-dropping.
- **`Presentation` surface** (`mjx-pptx`):
  - the seven `a:tblPr` flags — `table_part` / `set_table_part` (`TablePart`).
  - `table_style_id` / `set_table_style` — read and assign a table's `a:tableStyleId`.
  - `create_table_style`, creating the `tableStyles.xml` part on demand (relationship + content-type
    wired like an image part), and `format_table_style_part` with the new `TableStyleFormat` builder.
  - `with_table_style` — resolve a table's style through the shared part.
  - `PptxError::TableStyleNotFound`.
- **`tests/fixtures/tables.pptx`** — a deck carrying a real `tableStyles.xml` and a table naming its
  style.

### Notes

- A table style is layered formatting keyed by which part of the table a cell is in; modeling the
  part is what makes a `tableStyleId` resolve — the basis for effective cell formatting (next).
- Authoring a style touches exactly the content-types manifest, the presentation's relationships, and
  the new part; every other part stays byte-identical, and reading a styled table dirties nothing.

### Added

- **`Presentation::insert_row`, `remove_row`, `insert_column`, `remove_column`** — an index equal to
  the current count appends; beyond it is `TableCellOutOfRange`. A new row copies the height of the
  row beside it and a new column the width of the column beside it; the frame's own bounds are left
  alone, as PowerPoint leaves them.
- **`Table::insert_row`, `remove_row`, `insert_column`, `remove_column`** (`mjx-dml`) — the
  span-adjustment logic, plus `TableColumn::new`, `TableRow::new`,
  `TableCell::set_body_and_properties`, and grid/row/cell insert-and-remove helpers.

### Notes

- **The grid and every row stay in step.** A column edit changes `a:tblGrid` and one `a:tc` in every
  row together, so the rows never disagree with the width the grid declares.
- **Merges are adjusted, not left dangling.** A merge the new line falls inside grows by one; a merge
  the removed line lies inside shrinks by one; a merge whose **anchor** is removed promotes the next
  cell of the region, which takes over the anchor's `a:txBody` and `a:tcPr` and the reduced span so
  the table looks unchanged — including a region merged in both directions at once.
- **Removing the last row or column is refused** with `InvalidTableSize`: PowerPoint will not open a
  table with no cells.
- **Insert then remove is byte-identical to no change** — a span that falls back to one loses its
  attribute rather than being written as `gridSpan="1"`.
- The structural edit runs on the typed `Table` (parse, mutate, write back), not the raw tree:
  unlike a single-cell text edit it touches every row anyway, so parsing the whole table costs
  nothing extra and the merge logic is expressed in terms of the model.

## [0.0.17] - 2026-07-22

Cells can be merged, and unmerged.

### Added

- **`Presentation::merge_cells`** — takes a `Cells` selection, since every selection is a rectangle
  and a rectangle is the only shape a merged region can take.
- **`Presentation::unmerge_cells`** — given **any** cell of a region, not only its anchor.
- **`TableCell::set_spans`, `set_merged`, `clear_merge`** (`mjx-dml`).
- **`PptxError::TableMergeCrossesSelection`.**

### Notes

- **Merging never removes a cell.** The anchor states how far it reaches; the covered cells stay in
  the table, each stating that something to its left or above owns it. So the grid stays
  rectangular, `(row, column)` addressing keeps working, and a covered cell **keeps its own text** —
  invisible until unmerged, which is what makes unmerging give everything back.
- **A merge then an unmerge is byte-identical to no change at all.** A default is *removed* rather
  than written: `gridSpan="1"` and `hMerge="0"` are what the schema already assumes.
- **A selection that would cut an existing merge in half is refused.** Truncating it would leave the
  table claiming a span that no longer fits, and growing the selection would merge cells the caller
  never named. A region wholly inside the selection is absorbed instead.
- Merging one cell, or none, changes nothing rather than writing a span of one.

## [0.0.16] - 2026-07-21

Say it once. The table surface stops needing loops.

### Added

- **`Cells`** — which cells an operation is about: `one`, `row`, `column`, `rectangle`, `all`.
- **`CellFormat`** — a builder naming the cell properties to write (`with_fill`, `with_border`,
  `with_outline`, `with_margins`, `with_anchor`, `with_text_direction`), plus `without_fill` /
  `without_border` / `without_borders` for removal.
- **`Presentation::format_cells`, `format_cell_text`, `format_cell_paragraphs`** — apply a spec
  across a selection in one call.

### Notes

- Styling a header row took nine calls in a loop and read like nine things rather than the one thing
  it is. In the office-open canary this change turns twenty-two lines and four loops into nine lines
  and none.
- **Neither half is a new pattern.** The crate already builds specs with `with_`-prefixed setters
  (`CharacterPropertiesSpec`, `LineSpec`), and `set_shape_run_properties` already means "every run in
  this much of the shape". Tables simply never got either.
- **A format writes only what it names**, so recolouring a region cannot flatten borders it never
  mentioned. A format naming nothing writes nothing — not even an empty `a:tcPr`.
- `without_fill` is not `with_fill(FillSpec::None)`: removing lets the table style decide again,
  stating "none" stops it. Same for borders.
- The table is located **once** and the selection walked within it, so formatting a whole table is
  one traversal rather than one per cell.
- The per-cell, per-property setters remain for the single-property case; both paths now share one
  get-or-create for `a:tcPr`.
- Selecting nothing (`Cells::rectangle(1..1, ..)`) is well-formed and changes nothing; a selection
  reaching past an edge reports the table's real dimensions.

## [0.0.15] - 2026-07-21

A table can be made to look like something.

### Added

- **Cell formatting on `Presentation`** — `cell_fill` / `set_cell_fill` / `clear_cell_fill`,
  `cell_border` / `set_cell_border` / `clear_cell_border` (all six edges, both diagonals included),
  `cell_margins` / `set_cell_margins`, `cell_anchor` / `set_cell_anchor`, and
  `cell_text_direction` / `set_cell_text_direction`.
- **`CellMargins`** (`mjx-pptx`) — the four insets, each optional.
- **`TableCellProperties` can now be written** (`mjx-dml`): `set_border`, `set_fill`, `set_margins`,
  `set_anchor`, `set_text_direction`, `set_horizontal_overflow`, plus the matching typed reads.
- **`TextAnchoring`, `TextDirection`, `TextHorizontalOverflow`** — generated from
  `ST_TextAnchoringType`, `ST_TextVerticalType` and `ST_TextHorzOverflowType`.

### Notes

- **A border is an `a:ln` under another name** — same `CT_LineProperties` content, different tag —
  which is why one `LineSpec` describes all six edges and no border type was needed.
- **Merge, not rebuild.** `a:tcPr` carries a `cell3D`, a `headers` and an `extLst` this tier does not
  model, so a child is replaced in place or inserted at its rank in the schema's sequence. Setting
  one border cannot disturb the other five.
- **Removing a fill is not writing `FillSpec::None`.** The first lets the table style decide again;
  the second states that the cell is deliberately unfilled and stops the style. Same for borders.
- **An unstated margin is absent, not zero.** The schema defaults are `0.1"` horizontally and
  `0.05"` vertically, so the two are different facts; `CellMargins` keeps every field optional, and
  a `None` on write leaves that inset exactly as it was.
- `ST_TextVerticalType` is named **`TextDirection`** because its own values include `horz`
  (Horizontal) — it selects which way text flows, so a "vertical" name would misdescribe most of its
  range. `wordArtVertRtl` is `VerticalWordArtRightToLeft`, the title ECMA gives it, even though it
  reads oddly beside `WordArtVertical`.
- The seven `a:tblPr` flags are deliberately **not** here: they emphasize nothing on their own, they
  tell a table style which parts to treat specially, and they land with the `tableStyles.xml` part.

## [0.0.14] - 2026-07-21

Tables exist on the deck — created, sized, and filled in.

### Added

- **`Presentation::add_table`** — builds the whole `p:graphicFrame`: the grid, every row and every
  cell, ready for text. A table is a shape on the existing index space, so it is positioned with
  `set_shape_bounds` and dropped with `remove_shape`.
- **`table_dimensions`, `column_width` / `set_column_width`, `row_height` / `set_row_height`,
  `cell_span`, `merged_cell_anchor`** — the table's shape, and which cell renders where.
- **Thirteen `cell_*` text methods** — `cell_text`, `set_cell_text`, the paragraph and run readers,
  and the formatting setters including the run-splitting `set_cell_text_range_properties`. Each is
  the corresponding shape method addressed at a cell instead: same operation, same errors.
- **`PptxError::ShapeIsNotATable`, `TableCellOutOfRange`, `InvalidTableSize`.**

### Changed

- The private text-body locator now takes a *site* — a shape's `p:txBody` or a cell's `a:txBody` —
  and every text operation is a named function both spellings call. `shape_text` and
  `set_shape_text` inlined their own copy of the locate and are folded in. No behaviour change; the
  text suites pass untouched.

### Notes

- **A cell's `a:txBody` is the same `CT_TextBody` as a shape's**, which is why the cell surface is
  delegation rather than a second implementation — a future text feature stays one change.
- Reaching a cell **walks the raw tree** rather than parsing the table, so editing one cell costs
  what editing a shape costs; only the addressed `a:txBody` is parsed and rebuilt.
- The column count comes from `a:tblGrid`, never from counting a row's cells.
- A new table's columns share the frame width evenly with the **last absorbing the rounding**, so
  they sum to exactly the frame rather than leaving it a few EMU short.
- A new table carries `firstRow` and `bandRow`, as PowerPoint's does: they claim nothing about
  appearance on their own, they tell a table style which parts to emphasize.
- `set_column_width` does **not** resize the frame — a table whose columns no longer sum to its
  frame is what PowerPoint itself produces when a column is dragged.
- Creating a table adds no parts and no relationships: only the slide changes.
- Effective (inherited) cell formatting is not here — a cell inherits from the table style, which
  needs the `tableStyles.xml` part, later in this workstream.

## [0.0.13] - 2026-07-21

The table, modeled. The first tier of the tables workstream.

### Added

- **`Table`, `TableProperties`, `TableGrid`, `TableColumn`, `TableRow`, `TableCell`,
  `TableCellProperties`** (`mjx-dml`) — `a:tbl` and everything under it, typed for the first time.
  A `p:graphicFrame` could already be positioned; now what it frames can be read.
- **`TablePart`** — the seven `a:tblPr` flags (`firstRow`, `bandRow`, …), which do not draw anything
  themselves but tell the table style which parts to emphasize.
- **`CellBorder`** — the six `CT_LineProperties` edges of a cell, including the two diagonals.

### Notes

- **How little of this is new.** A cell's content is a `CT_TextBody` — the *same* type a shape's
  `p:txBody` is — so the whole text tree and its formatting model apply inside a cell unchanged.
  Cell borders are `LineProperties`; cell and table fills are the fill model; widths, heights and
  margins are `Emu`. The genuinely new part is the two-dimensional shape.
- **Merging never removes a cell.** A merged region is anchored at its top-left cell, which carries
  `gridSpan`/`rowSpan`; every covered cell remains present carrying `hMerge`/`vMerge`. So a row holds
  as many `a:tc` as the grid has `a:gridCol`, `(row, column)` addressing has no holes, and
  `Table::merge_anchor` answers which cell actually renders at a position by walking left then up.
- The **grid** is the authority on column count: `a:tblGrid` is where a table declares its width.
  A table missing it reports no columns rather than inferring one from the rows.
- A cell's four margins have **non-zero schema defaults** (0.1" horizontal, 0.05" vertical), so an
  unstated margin is not a zero one; the accessors report what the file states and the defaults are
  exposed as constants.
- `a:tableStyleId` is **reported but not resolved** — the `tableStyles.xml` part it names is a later
  tier of this workstream.
- Nothing in `mjx-pptx` uses this yet: creating a table, reaching cell text, and formatting cells
  are the next PRs.

## [0.0.12] - 2026-07-21

Where a shape actually renders. The transform workstream is complete.

### Added

- **`Presentation::effective_shape_bounds`** and **`Presentation::effective_shape_transform`** — the
  position a shape *renders* at, not the one it declares. A placeholder that places itself nowhere
  resolves through the same-slot placeholder on its layout, and failing that its master.

### Changed

- The candidate walk every effective property starts with — the addressed shape, then the same-slot
  placeholder on each part the surface inherits from — is now **one** private helper
  (`placeholder_candidates` + `candidate_shape`) rather than a copy inside `effective_shape_fill`,
  `_outline` and `_effects`. Behaviour is unchanged; those suites pass untouched.

### Notes

- **Inheritance is all-or-nothing at the `a:xfrm` level.** Text formatting merges tier by tier, each
  supplying what the ones above left unset; a transform does not. A shape cannot take its position
  from the layout and its size from the master, so the first tier that states anything wins whole.
- **A present-but-empty `<a:xfrm/>` states nothing**, so resolution steps past it exactly as it steps
  past a tier with no transform element at all — what `Transform2D::is_empty` exists for.
- A shape that is **not a placeholder** has no tier to inherit from, so its effective transform is
  its explicit one.
- A tier that answers with only a rotation yields `effective_shape_bounds == None`: bounds are all
  four numbers, and the all-or-nothing rule means no other tier is consulted.
- `tests/fixtures/layouts.pptx`'s `slideLayout2` title placeholder no longer declares an `a:xfrm`,
  so it defers to the master — ordinary in real decks, and the only way the master tier becomes
  reachable. A slide built from that layout now resolves its title at the master and its body at the
  layout.
- `docs/TRANSFORM_HANDOFF.md` closes the workstream; `PLAN.md` now names **tables** and **speaker
  notes** as what remains before `v0.1`.

## [0.0.11] - 2026-07-21

A shape can be moved. The transform reaches the deck.

### Added

- **`Presentation::shape_bounds` / `set_shape_bounds`** — read, move and resize any shape. Until now
  `ShapeBounds` was written once, at shape creation, and could be neither read back nor changed.
- **`Presentation::shape_transform` / `set_shape_transform`** — the whole `a:xfrm`: position, size,
  rotation, the two mirror flags, and a group's child coordinate space. Rotation and flips had no
  expression at all before this.
- **`ShapeBounds::from_transform` / `to_transform`** — the bridge to `mjx_dml::Transform2D`.
- **`PptxError::ShapeCannotBePositioned`** — names the one shape kind (`p:contentPart`) whose schema
  has nowhere to put a transform, instead of reporting a missing element.

### Notes

- **A transform is not in the same place for every shape kind**, which is what made this its own
  piece of work: `p:spPr > a:xfrm` for a shape, picture or connector; `p:grpSpPr > a:xfrm` for a
  group (a `CT_GroupTransform2D`, carrying `a:chOff`/`a:chExt`); and `p:xfrm` for a graphic frame —
  PresentationML's namespace, a direct child, and required rather than optional. Only the wrapper
  differs; the `a:off`/`a:ext` inside are DrawingML in every case.
- **`None` from `shape_bounds` is not "at the origin"** — it means the shape places itself nowhere,
  and a placeholder's real position is on its layout or master. Resolving that is the next PR.
- **Setting bounds cannot disturb anything else.** `to_transform` names only position and size, and
  `Transform2D::apply` writes only named fields, so moving a shape leaves its rotation alone and
  moving a group keeps the child space its members are laid out in. Resizing a group does rescale
  its members — a group maps its child space onto its own extent, which is what PowerPoint does.
- Shape creation now emits its `a:xfrm` through the same writer as shape editing, so the two cannot
  drift apart. The bytes are unchanged.
- `tests/fixtures/layouts.pptx` gained a `p:grpSp` and a `p:graphicFrame` (holding a real one-cell
  table) on slide 2, appended so existing shape indices keep their meaning — the two exotic locator
  paths now meet a real file, and the tables workstream inherits a fixture.
- Group members are still not addressable, so bounds are always in the parent tree's coordinate
  space. Computing an absolute rectangle for a shape inside a group needs group descent.

## [0.0.10] - 2026-07-21

Where a shape sits, and which way up — the model tier of the transform workstream.

### Added

- **`Transform2D`, `Position` and `Size`** (`mjx-dml`) — `a:xfrm` typed for the first time: an offset
  (`a:off`), an extent (`a:ext`), a rotation (`@rot`) and the two mirror flags (`@flipH` / `@flipV`).
  One type covers both `CT_Transform2D` and a group's `CT_GroupTransform2D`, whose `a:chOff` /
  `a:chExt` child coordinate space is the same sequence with two more members.
- **`Transform2D::apply`** — writes only the fields a caller names, editing the element in place.

### Notes

- **Every field is optional, and absent is not zero.** A placeholder that declares no `a:xfrm` is
  asking its layout where it goes; a transform that read as "origin, zero-sized" could not be told
  from one that means *ask someone else*, and the inheritance walk depends on telling them apart.
- `apply` **merges rather than rebuilds**, because an `a:xfrm` carries content this model does not
  describe — a group's child coordinate space, an `extLst`, unknown attributes on the `a:off` itself.
  Rebuilding it wholesale would move every member of a group whose position was changed. New children
  are inserted at their rank in the schema's sequence (`off` → `ext` → `chOff` → `chExt`).
- A transform reads the same whether its wrapper is DrawingML's `a:xfrm` or the `p:xfrm` a
  `p:graphicFrame` holds — the wrapper's namespace differs, its children do not.
- The measure attribute readers/writers (`attr_emu`, `push_angle`, …) moved from `effect.rs` to
  `build.rs`: a measure-valued attribute is not an effect's idea, and now has one spelling on read
  and one on write rather than one per module.
- Nothing in `mjx-pptx` uses this yet — reading and writing a shape's bounds is the next PR.

## [0.0.9] - 2026-07-21

What the text actually renders as. The text-formatting workstream is complete.

### Added

- **`Presentation::effective_run_properties`** and **`Presentation::effective_paragraph_properties`**
  — the formatting a run and a paragraph *render* with, not the formatting they declare. Seven tiers
  resolve, each contributing only what the tiers above left unset: the run's `a:rPr`, the paragraph's
  `a:defRPr`, the shape's `a:lstStyle`, the same-slot placeholder's on the layout and master, the
  master's `p:txStyles`, `p:defaultTextStyle`, and the theme font scheme.
- **`p:txStyles` and `p:defaultTextStyle` are read** for the first time — the tiers where a
  placeholder's real size, bullet and alignment have always lived.

### Notes

- The paragraph's level is read **once**, before the walk, and selects which `a:lvlNpPr` every tier
  from the third down contributes: a level-2 paragraph that declares nothing answers with the master
  `bodyStyle`'s `a:lvl3pPr`.
- Colors bake to concrete `RRGGBB`, consistent with `effective_shape_fill`.
- A shape that is **not a placeholder** takes no master text style; it falls through to
  `p:defaultTextStyle`, as PowerPoint does. A font slot the theme leaves undefined keeps its
  `+mj-lt` reference rather than inventing a font.
- `tests/fixtures/layouts.pptx` gained three distinct `bodyStyle` levels and a layout-placeholder
  `a:lstStyle`, so the level axis and the placeholder tier are demonstrable on a real deck.

## [0.0.8] - 2026-07-21

What "inherited" means, made explicit — the merge one tier of the text-formatting ladder performs.

### Added

- **`CharacterPropertiesSpec::merge_under`** and **`ParagraphPropertiesSpec::merge_under`**
  (`mjx-dml`) — merge a lower inheritance tier under a spec: the receiver is the higher tier and
  wins, and the argument supplies only what the receiver leaves unset. Folding from the top reads as
  the ladder does: `run.merge_under(&paragraph).merge_under(&shape)`.

### Notes

- Properties merge as **whole values**, so an explicit "off" — `b="0"`, `a:noFill`, `<a:buNone/>` —
  is a present value that blocks the tier below rather than an absence that falls through it.
- Four fields are not a plain field-wise fallback: fonts merge **per script slot**, tab stops as one
  **list** (`a:tabLst` replaces wholesale), `a:defRPr` **recursively**, and each of the four bullet
  groups **as a unit**.
- These are the merge halves of effective text formatting; the inheritance walk that calls them
  follows.

## [0.0.7] - 2026-07-21

The theme's font scheme — where a typeface of `+mj-lt` finally leads.

### Added

- **`FontScheme`** (`mjx-dml`) — `a:fontScheme` modeled as `{ name, major, minor }`, on both `Theme`
  and the interner-free `ThemeInfo` (`Theme::font_scheme` / `ThemeInfo::font_scheme`), so a deck's
  font scheme is reachable through the existing `Presentation::theme`.
- **`FontCollection`** — one collection's latin / East Asian / complex-script fonts, keyed by the
  existing `FontSlot` (`FontSlot::Symbol` is always absent: a collection has no `a:sym`), plus its
  `SupplementalFont` per-script fallbacks, looked up by ISO 15924 script tag.
- **Theme font references** — `TextFont::theme_reference` parses the six spellings the schema
  defines (`+mj-lt`, `+mj-ea`, `+mj-cs`, `+mn-lt`, `+mn-ea`, `+mn-cs`) into a `ThemeFontReference`;
  anything else, including other `+…` strings, is not a reference. `FontScheme::resolve` answers
  what a font is actually drawn with — itself when literal, the scheme's font when a reference.

### Notes

- The theme part stays read-only: the font scheme is a parsed value view, with no write path.
- This is the last piece the effective-text-formatting resolution needs; the inheritance walk that
  consumes it follows.

## [0.0.6] - 2026-07-21

Text formatting reaches the deck. Everything the previous four releases modeled is now callable on a
real `.pptx`, at every scope a user can select.

### Added

- **The paragraph axis** on `Presentation` — `paragraph_count`, `run_count`, `paragraph_text`,
  `run_text`. Run indices are paragraph-local, matching the document tree. The existing flat
  `set_shape_text` is unchanged.
- **Reading formatting** — `paragraph_properties`, `run_properties`, `end_run_properties`. Reading
  never dirties a part.
- **Writing formatting, one call per selection granularity**:
  - `set_run_properties` — one run.
  - `set_paragraph_run_properties` — every run in a paragraph, and its paragraph mark.
  - `set_shape_run_properties` — every run in the shape, and every mark.
  - `set_text_range_properties` — an arbitrary character range, splitting runs where the range cuts
    across them.
  - `set_text_range_properties_by_grapheme` — the same, addressed in grapheme clusters, so an emoji
    and its modifier are one unit.
  - `set_paragraph_properties` — a paragraph's layout (alignment, level, margins, spacing, bullet).
  - `set_end_run_properties` — the format of an **empty** paragraph, which is what a placeholder
    added but not yet typed into holds.
- **`TextRun::split_at` / `Paragraph::split_run_at`** in `mjx-dml` — divide a run's text, giving both
  halves the original's formatting, so splitting alone changes nothing about how the text renders.
- **`Paragraph::set_end_properties`** — the write half of the `a:endParaRPr` surface.

### Notes

- Formatting a paragraph or a shape also formats the paragraph mark, so text typed at the end takes
  the same formatting — what "select and restyle" means to a user.
- Runs are split but never merged, keeping each edit minimal. A range already aligned to run
  boundaries splits nothing, so repeated edits do not accumulate runs.

## [0.0.5] - 2026-07-21

Bullets and numbering — the marks that express a deck's paragraph hierarchy.

### Added

- **`Bullet`** — what marks a paragraph: `None` (an explicit "no bullet", which overrides an
  inherited one), `Character` (a literal glyph), `AutoNumber` (a scheme plus where its sequence
  starts), or `Picture` (an image by relationship id).
- **`BulletColor`, `BulletSize`, `BulletTypeface`** — the bullet's colour, size and font, each with a
  `FollowText` variant for the schema's "match the text" arm. All four groups are set and inherited
  **independently**, as the schema defines them.
- **Builder support** on `ParagraphPropertiesSpec`: `with_bullet`, `with_bullet_color`,
  `with_bullet_size`, `with_bullet_typeface`, plus `with_bullet_character("•")` and
  `without_bullet()` for the common cases.

### Notes

- A bullet percentage is written in the form both schemas specify and ECMA §21.1.2.4.9 illustrates
  (`val="111%"`); the integer spelling found in some files is still read.
- Setting one bullet group never disturbs the others, and a group left unnamed keeps whatever the
  file had.

## [0.0.4] - 2026-07-21

Paragraph formatting: how a paragraph is laid out, and the per-level styles it inherits from.

### Added

- **`ParagraphProperties`** (`CT_TextParagraphProperties`) — indent level, alignment, left/right
  margins, first-line indent, default tab size, reading direction and font alignment, plus line
  spacing, space before/after, tab stops, and the `a:defRPr` a paragraph's runs default to. One type
  serves `a:pPr`, `a:defPPr` and `a:lvl1pPr`…`a:lvl9pPr`; the line-breaking attributes, bullets and
  anything unknown round-trip verbatim.
- **`ParagraphPropertiesSpec`** — the builder, matching the character-properties conventions.
  Margins, indents and tab stops are stated **in points**; EMU is the file's unit and stays reachable
  through `Emu`.
- **`IndentLevel`** — the 0–8 nesting level a paragraph's inherited bullet, size and indent are
  selected by. `IndentLevel::of(2)` for a literal, `::new(raw)` for a value off the wire, `::TOP` for
  the outermost.
- **`TextSpacing`** — a proportion of the line height (`a:spcPct`) or a fixed distance (`a:spcPts`),
  kept apart because they are different measurements. **`TabStop`** — position and alignment.
- **`TextListStyle`** (`a:lstStyle`) — the paragraph properties a container offers at each level, by
  `level(IndentLevel)`. The same type covers a shape's own list style, a placeholder's, and each of a
  master's three text styles.
- **Typed access from the text tree** — `Paragraph::properties` / `set_properties` and
  `TextBody::list_style`, so `a:pPr` and `a:lstStyle` are no longer opaque.

## [0.0.3] - 2026-07-20

Text formatting begins: the vocabulary and the run-level model. A run's appearance — its size, weight,
slant, underline, colour, font — can now be read and written. (Reaching it through a `Presentation`,
and resolving what a run *inherits*, come next.)

### Added

- **Text simple types** — `TextUnderline`, `TextStrike`, `TextCapitalization`, `TextAlignment`,
  `FontAlignment`, `TabAlignment` and `AutonumberScheme` (41 bullet-numbering schemes), generated from
  `dml-main.xsd` and named from the ECMA-376 §20.1.10 enumeration tables.
- **`FontSize` and `TextPoint`** — text measures stated **in points** (`from_points` / `points`), the
  unit every size control uses. The file's hundredths of a point are reachable only through
  `from_wire` / `to_wire`.
- **`CharacterProperties`** (`CT_TextCharacterProperties`) — size, bold, italic, underline, strike,
  capitalization, spacing, kerning, baseline, language, plus the text fill, glyph outline, effects,
  highlight and the four script fonts. One type serves `a:rPr`, `a:defRPr` and `a:endParaRPr`, and
  everything it does not model — hyperlinks, `dirty`/`err`/`smtClean`, unknown children — round-trips
  verbatim.
- **`CharacterPropertiesSpec`** — an interner-free builder:
  `CharacterPropertiesSpec::new().with_size_points(28.0).with_bold(true).with_color(…)`. Naming a
  property sets it; leaving it unnamed means *inherit*, so `with_bold(false)` and
  `with_underline(TextUnderline::None)` are how a caller overrides an inherited value.
- **`TextFont`** — a typeface reference, whether a literal name or a `+mj-lt`-style theme reference.
- **`resolve_character_properties`** — bakes a run's colours (text fill, glyph outline, effects,
  highlight) down to concrete RGB against a theme scheme and colour map.
- **Typed access from the text tree** — `TextRun::properties` / `set_properties` and
  `Paragraph::end_properties`, so `a:rPr` and `a:endParaRPr` are no longer opaque.

### Notes

- Setting a run's properties **merges** onto its existing `a:rPr` rather than replacing it, so the
  state this model does not describe (`lang`, `dirty`, a hyperlink) survives a restyle. An unset
  property means "leave it alone", never "clear it".

## [0.0.2] - 2026-07-20

The PowerPoint slice — Phases 2 and 3. A real `.pptx` can now be opened, read, edited, built up from
its own layouts and pruned back down, and written out so PowerPoint and LibreOffice open it with every
untouched part byte-identical. Phase 3 closes here; Word (Phase 4) is next.

### Added

- **De/serialization (Phase 2)** — `FromXml`/`ToXml` in `mjx-ooxml-core::convert` and the
  `#[derive(FromXml, ToXml)]` proc-macro in `mjx-derive`. Every modeled type keeps an unknown-content
  bucket, so what we do not model survives a round trip.
- **DrawingML text (Phase 2)** — `mjx-dml`'s `TextBody`/`Paragraph`/`TextRun`/`Text`, with a mutation
  surface.
- **PresentationML (Phase 2)** — `mjx-pptx::Presentation`: `open`/`save`, slide inventory, shape
  enumeration, `shape_text`/`set_shape_text`, and construction — `add_text_box`, `add_shape`,
  `add_slide`. The **office-open canary** (LibreOffice headless must render the produced deck to a
  valid PDF) became a CI gate.
- **Preset geometry (Phase 3)** — all 187 `ST_ShapeType` values generated, and the 117 adjustable
  shapes given **named, spec-sourced control parameters** (a rounded rectangle exposes
  `corner_radius`, never `adj1`), with the meaning derived from `presetShapeDefinitions.xml`.
- **Color, theme and the `spPr` visual trilogy (Phase 3)** — theme (`clrScheme`/`fmtScheme`) with
  color resolution to concrete RGB, and **fill**, **outline** (`a:ln`) and **effects**
  (`a:effectLst`), each modeled both *explicitly* and *effectively* — resolved through style
  references and placeholder inheritance to what actually renders.
- **Images (Phase 3)** — `add_image` media parts (de-duplicated by content, format identified by
  magic bytes), `add_picture` `p:pic` shapes, and picture read/replace — on one shape index space
  covering every shape kind.
- **Layouts and masters (Phase 3)** — the layout/master inventory, generated PresentationML simple
  types, **`Surface` addressing** (every shape call works on a slide, a layout or a master, so editing
  a layout reaches every slide inheriting it), and `add_slide_from_layout`, which returns a slide
  carrying the layout's placeholders ready to fill.
- **Removal (Phase 3)** — `remove_shape` on any surface, and `remove_slide`, which unwires
  `p:sldIdLst` → relationship → part and takes with it every part only that slide referenced (its
  notes slide, unshared media) while sparing anything the rest of the deck still uses.
- **Packaging** — `Package::{insert_part, remove_part, remove_part_cascading,
  set_content_type_default/override, add_relationship, remove_relationship}` over a copy-on-write part
  body, plus `PartName::{resolve, resolve_from_root, relative_target}` — the part-name algebra Word
  and Excel will share.

### Fixed

- `add_shape` / `add_text_box` built a paragraph with no run, so the shape they returned could not be
  filled by `set_shape_text`. Every paragraph they create now holds exactly one run, blank lines
  included.
- `add_slide_from_layout` cloned the date, footer and slide-number placeholders. Those render *from
  the layout* for slides that do not declare them, so the clones suppressed the layout's rendering and
  showed as empty boxes; they are now skipped, as PowerPoint does.

### Notes

- The round-trip contract is unchanged and continuously asserted: per-part decompressed-payload byte
  identity plus structural container identity. Reading dirties nothing; an edit re-serializes only its
  own part.
- Public API remains unstable until `v0.1`.

## [0.0.1] - 2026-07-15

First versioned snapshot. Establishes the workspace, the packaging + fidelity + compatibility core,
the schema-type generator, and full documentation. No format models yet.

### Added

- **Packaging (Phase 0)** — `mjx-opc`: load an OOXML package fully into RAM as an ordered part graph,
  parse `[Content_Types].xml` and `_rels/*.rels`, and re-zip with per-part decompressed-byte identity.
  Minimal namespace-resolving reader in `mjx-xml`.
- **Schema codegen (Phase 0)** — `xtask` generates `mjx-ooxml-types` (namespace table +
  `shared-commonSimpleTypes`) with comprehensive, self-explanatory names and exact wire tokens;
  output is deterministic and committed.
- **Fidelity layer (Phase 1)** — `mjx-ooxml-core` string interner + the `RawDocument` preservation
  tree, and `mjx-xml::fidelity`, a byte-preserving reader + hand-written writer. Parsing then
  re-serializing any part reproduces the source **byte-for-byte** (verified on real `.pptx`/`.docx`/
  `.xlsx` fixtures).
- **Markup Compatibility (Phase 1)** — `mjx-mce`: preserve mode (the untouched tree) and a
  non-mutating resolve mode (`AlternateContent` Choice/Fallback, `Ignorable`, `ProcessContent`,
  `MustUnderstand`).
- **Documentation** — comprehensive rustdoc across all crates (crate guides + runnable examples), a
  facade docs hub (`mjx-ooxml`), enforced via `missing_docs` and a strict-rustdoc CI job.
- **Project** — CI (fmt/clippy/test + wasm/Android/iOS/macOS/Windows cross-compile build matrix),
  dual `MIT OR Apache-2.0` license, and the contributor/agent guides.

### Notes

- Cross-platform: pure-Rust dependency graph; the library crates cross-compile to
  `wasm32-unknown-unknown`, `aarch64-linux-android`, and Apple/Windows targets.
- A broader multi-producer sample corpus and fuzzing are planned for later iterations.

[0.0.9]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.9
[0.0.8]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.8
[0.0.7]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.7
[0.0.6]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.6
[0.0.5]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.5
[0.0.4]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.4
[0.0.3]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.3
[0.0.2]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.2
[0.0.1]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.1
