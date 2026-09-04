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
| `mjx_pptx::PptxError` was `#[non_exhaustive]` | it is not | A `#[non_exhaustive]` enum forces a wildcard arm on every downstream `match`, which is exactly what would let a new failure mode be silently filed under a catch-all. `mjx_ooxml::Error`'s classification is deliberately exhaustive: adding a variant now fails the build until someone decides which of the eleven `ErrorCode`s it belongs to. |
| `delete_chart_data_labels`, `Axis::is_deleted`, `DataLabels::delete_all`, `auto_title_deleted` (12 public identifiers) | `suppress_chart_data_labels`, `is_suppressed`, `suppress_all`, `auto_title_suppressed` | `delete_*` wrote a `c:delete` (*draw nothing here*) and sat beside `remove_*`, which removes the element (*say nothing here*). Two operations, two near-synonyms, no way to tell them apart from the method list. `delete` was the spec element's own name; a public identifier that needs the spec open to be read is the thing the convention forbids. The wire token is unchanged and still named in every item's docs. |
| `mjx_docx::PageOrientation` (hand-written, MJXOFF-98) | `mjx_docx::PageOrientation` (re-export of `mjx_ooxml_types::wordprocessingml::PageOrientation`) | A duplicate of the generated enum, caught in MJXOFF-109's own pre-dispatch review — "consume, do not re-create" is the generator's whole reason to exist. `PageOrientation::to_wire(self) -> Option<&'static str>` (`None` for `Portrait`, the schema default) is **removed**: the generated type's own `to_wire(self) -> &'static str` always returns a token, and the "omit the attribute for `Portrait`" convenience now lives in `SectionProperties`'s writer (`crate::page::orientation_wire_value`, crate-private), not as a method on the value type. |

Nothing else in the public surface changed name or shape. The sweep read all 1,561 public
identifiers of the eleven merged PowerPoint children; everything else either already followed the
convention or is a spec-sourced proper noun (`Srgb`, `ScRgb`, `OleObject`, the preset-shape names
whose digits are part of their identity).

The one candidate the sweep declined to settle on its own — whether `delete_chart_data_labels`
should become `suppress_*`, given that `delete` is the spec element's own name and runs through a
dozen coherent `mjx-chart` identifiers — was decided in favour of the rename and taken in 0.0.69,
whole rather than in part: renaming only the `mjx-pptx` method would have traded one inconsistency
for another. It is the row above. A grep in CI now keeps the spelling from drifting back.

## [0.0.90] - 2026-09-04

Word headers and footers (MJXOFF-113, Phase C position 11): `CT_HdrFtr`, variant resolution, and the
legacy VML they carry — `crates/mjx-docx/src/document/headers.rs`.

**Header/footer parts reuse MJXOFF-92's block-content addressing rather than duplicating it.**
`body.rs`'s paragraph-vec logic (`paragraph`/`paragraph_mut`/`insert_paragraph`/`append_paragraph`/
`remove_paragraph`) is now five free functions (`block_paragraph[_mut]`, `block_insert_paragraph`,
`block_remove_paragraph`, …) operating on `&[BlockContent]`/`&mut Vec<BlockContent>`; `Body` delegates
to them, and the new `HdrFtr` (`CT_HdrFtr`, reusing `BlockContent` itself — `w:sectPr` is mapped in its
own `#[xml(children, …)]` list purely so the derive macro's exhaustive match compiles, never
constructed) uses the same functions. A header's paragraphs and runs are ordinary
`Paragraph`/`Run` — MJXOFF-94's run properties, MJXOFF-96's paragraph properties and MJXOFF-106's
effective-property ladder already work inside a header with no further wiring.

**Variant resolution — `Document::resolve_header`/`resolve_footer` — implements ECMA-376 Part 1
§17.10.1/.5/.2/.6, not a lookup.** A first/even query whose governing flag (`w:titlePg`/
`w:evenAndOddHeaders`) is off downgrades to the default (odd) query *before* the previous-section
inheritance walk runs — confirmed against the prose directly: *"If \[`titlePg`\] is set to false and a
first page header/footer is specified, then it shall be ignored and only the odd page header/footer
shall be displayed"* (§17.10.6), identically for `evenAndOddHeaders` (§17.10.1) and the even variant.
Inheritance is per-variant, from the nearest preceding section that states that specific type
(§17.10.5/.2, identical prose in both): *"If no headerReference for the \[…\] page header is specified
\[…\] the \[…\] page header shall be inherited from the previous section or, if this is the first
section in the document, a new blank header shall be created."* `w:evenAndOddHeaders` is read directly
from `word/settings.xml` (`Document::even_and_odd_headers`) — MJXOFF-136 models the part; this reads
only the one flag.

**`SectionProperties::remove_header_reference`/`remove_footer_reference` and
`ParagraphProperties::section_properties_mut` are new** — MJXOFF-109 built the field and its structural
push/read but not its removal, since resolution (and therefore "replace" and "remove") was this
child's own scope. `section_properties_mut` (the paragraph-level counterpart of `Body`'s own
`section_properties_mut`) exists so removing a reference never fabricates a `w:sectPr` a section did
not already carry.

**`mjx-vml` is a plain dependency of `mjx-docx` now, ungated** — unlike `mjx-pptx`'s `vml` feature
flag, which exists only to spare PresentationML callers a dependency they may never touch; Word headers
are the primary place VML watermarks and text boxes still appear in the wild.
`Document::header_footer_vml_drawings` resolves a header or footer's `mc:AlternateContent` via
`mjx-mce` (non-mutating) and reads every surviving `w:pict` through `mjx_vml::Drawing` — the first
consumer of MJXOFF-58's model outside PowerPoint.

**Two committed fixtures, both authored through this crate's own public API**
(`Document::blank`/`create_header`/`create_footer`/`edit_header_footer` — never a template):
`header_footer_variants.docx` (two sections; section 1 states all three header and footer variants
with `w:titlePg` absent, section 2 states none at all) and `header_watermark.docx` (one header holding
real, hand-authored `mc:AlternateContent`/`w:pict` VML — the one literal XML fragment in the change,
since this crate has no VML-authoring surface). Mutation-proved: neutralising the `w:titlePg` check,
the `w:evenAndOddHeaders` check, or the previous-section inheritance walk each turns a distinct set of
`crates/mjx-docx/tests/headers.rs` tests red; restored by re-editing.

Fixes a stale `document/mod.rs` module doc that still listed `styles.rs`, `numbering.rs`,
`effective.rs` and `sections.rs` among files "later children are expected to add" — all four already
existed.

## [0.0.89] - 2026-09-04

Word sections (MJXOFF-109, Phase C position 10): `w:sectPr`, page setup, columns, section breaks,
line numbering, header/footer references and `w:printerSettings` — `crates/mjx-docx/src/document/sections.rs`.

**A section's properties live at the END of the range they govern, not the start.** A `w:sectPr`
inside a paragraph's `w:pPr` ends a section *at* that paragraph; the body-level one is always the
document's last section. `SectionProperties::sections` (via the new `sections_in`) walks a body's
paragraphs and returns `SectionSpan`s accordingly. **A single-section fixture cannot catch a reader
that only ever looks at the body-level `w:sectPr`** — `tests/fixtures/three_section_document.docx`
is authored specifically to: section 1 (paragraphs 0–1, landscape A4), section 2 (paragraphs 2–3,
portrait A4, two equal-width columns), section 3 (paragraph 4, the body-level `w:sectPr`, portrait
A4, one column). Mutation-proved: neutralising the paragraph-level scan in `sections_in` turns five
tests red, including the mutation-gate test itself (`paragraph_to_section_assignment_is_correct_on_the_three_section_fixture`,
`left: 0, right: 1`); restored by re-editing.

**All 19 of `EG_SectPrContents` and `EG_HdrFtrReferences`** are modelled on `SectionProperties`:
`w:type`, `w:pgSz`/`w:pgMar` (bridged to the shared `PageSize`/`PageMargins` value types — see
below), `w:paperSrc`, `w:pgBorders` (reusing MJXOFF-94's `Border` model for `w:top`/`w:left`/
`w:bottom`/`w:right` via a `xsd:extension` — `Border::extension_attributes[_mut]`, a small
crate-visible escape hatch, rather than a fourth copy of `CT_Border`'s nine attributes),
`w:lnNumType`, `w:pgNumType`, `w:cols`, `w:formProt`/`w:noEndnote`/`w:titlePg`/`w:bidi`/`w:rtlGutter`
(reusing `Toggle`), `w:vAlign`, `w:textDirection` (reusing `ParagraphTextFlowDirection` directly —
`CT_TextDirection` is the identical type under the identical local name at both `w:pPr` and
`w:sectPr`), `w:docGrid`, `w:printerSettings` (reusing `RelationshipReference`), `w:headerReference`/
`w:footerReference` (the flag and the field are modelled here; *which* header/footer applies is
MJXOFF-113's), and `w:sectPrChange`/`w:footnotePr`/`w:endnotePr` (structure only, kept opaque —
MJXOFF-126/MJXOFF-124 own their semantics).

**`w:equalWidth="true"` wins over an explicit `w:col` list, confirmed against ECMA-376 Part 1
§17.6.4's own prose** ("If `equalWidth` is true, then the columns are defined using the data stored
as attributes of the `cols` element … If `equalWidth` is false, then the columns are defined using
the presence and data on each child `col` element", with a worked example describing the `w:col`
children as "ignored" once `equalWidth="1"`). `Columns` does not resolve this itself (no page-margin
knowledge to compute a width from) — it exposes `is_equal_width` and the explicit `columns()` list
independently, with the ruling written down once in `sections.rs`'s own module doc.

**`w:pgMar/w:header`/`w:footer` are measured from the page edge, not the text body** — confirmed
directly against ECMA-376 Part 1 §17.6.11 ("`header` … Specifies the distance … from the top edge of
the page to the top edge of the header"; "`footer` … from the bottom edge of the page to the bottom
edge of the footer"), restated on `PageMargins`'s own field docs.

**`PageOrientation` de-duplicated** (see Breaking changes): the public API now exposes exactly one
orientation type, the generated `mjx_ooxml_types::wordprocessingml::PageOrientation`, re-exported
from `mjx_docx::page`.

**`blank.rs`'s hand-written minimal `w:sectPr` is replaced by the real modelled writer.** The outer
skeleton (`<w:document>`/`<w:body>`/`<w:p/>`) is still a hand-written template — matching
`mjx_pptx::blank`'s own established convention for a part built from nothing — but the `w:sectPr`
fragment itself now comes from `SectionProperties::new` + its own setters, serialized on its own and
spliced in as bytes, never hand-formatted. Fixing this surfaced a real, previously-latent gap: a
`Document::blank`-authored document never declared `xmlns:r`, so any `r:`-prefixed attribute this
child's own new functionality can now write (`w:printerSettings@r:id`, `w:pgBorders`' corner
relationships, `w:headerReference`/`footerReference@r:id`) would have produced namespace-unbound,
invalid XML. `blank.rs` now declares `xmlns:r` alongside `xmlns:w` on the root, matching every real
Word/LibreOffice-authored document (`tests/fixtures/sample.docx` included).

**`w:printerSettings` never rewrites the binary part it references.** Proved on
`tests/fixtures/printer_settings_reference.docx` (authored — no fixture in the corpus carried a
Printer Settings part): editing an unrelated field of the *same* `w:sectPr` that carries
`w:printerSettings` leaves the referenced part's bytes and the relationship's id/target byte-identical.

**Splitting a document into a new section places the new `w:sectPr` inside the terminating
paragraph's own `w:pPr`, never appended to the body** — `Document::edit_section_properties` (get-or-
insert, unifying "change an existing section" and "create a new one") and
`Document::remove_section_properties`, both addressed by the new `SectionLocation` enum.

## [0.0.88] - 2026-09-04

Word effective-properties ladder (MJXOFF-106, Phase C position 9): `Document::effective_run_properties`
and `Document::effective_paragraph_properties` — every `EG_RPrBase` (38 fields) / `CT_PPrBase`
(32 fields) member resolved across `w:docDefaults` → the numbering level → the paragraph-style
`w:basedOn` chain → the character-style chain → direct formatting, with colours baked to concrete
`RRGGBB` through `mjx-dml`'s own theme model.

**The ladder order the ticket stated was wrong, verified against ECMA-376 Part 1 §17.7.2's own
prose, not assumed.** The ticket ordered the paragraph-style chain above the numbering level;
§17.7.2 states the opposite ("First, the document defaults … Next, … numbered item and paragraph
properties are applied … Next, paragraph and run properties are applied … as defined by the
paragraph style"). `tests/effective.rs`'s discriminating fixture (`w:sz` set to three different
values at docDefaults/numbering/the paragraph-style chain, moved one rung at a time across three
paragraphs) is built to fail under the ticket's own order; mutating the merge fold to that order
turns two tests red, pasted in the PR.

**Toggle properties combine by XOR across ladder tiers, and only twelve of them (ECMA-376 Part 1
§17.7.3), not every `CT_OnOff`-shaped member.** A run whose paragraph style and character style both
state `w:b="true"` renders **not bold** — `true XOR true = false` — the opposite of what a naive
override-based resolver (the same rule every other field correctly uses) would answer. Proved by
mutation: replacing the twelve-field XOR recombination with plain fallback turns the cancellation
test red.

**Theme colour and theme font resolve through `mjx-dml`'s own theme model — no second one.** Word's
`ST_ThemeColor` (17 wire tokens, including the `background1`/`text1`/`background2`/`text2` aliases)
maps onto DrawingML's `a:schemeClr` vocabulary; the `bg1`/`tx1`/`bg2`/`tx2` half of that mapping
reuses `mjx_dml::ColorMap::identity` directly rather than restating it, since its own default
pairing (`bg1→lt1`, `tx1→dk1`, …) is exactly what ECMA-376 Part 1 §17.15.1.20 states for Word's
`w:clrSchemeMapping` when absent (true of every fixture in this workspace — `word/settings.xml` is
not modelled by any child yet). `w:rFonts`'s theme attributes resolve the same way against the font
scheme's major/minor × Latin/East-Asian/complex-script slots.

**Cache design:** a chain, once resolved, is reused for every field of one `effective_*` call rather
than re-walked per field — `ChainCache`, memoized by `styleId`, scoped to a single call. It does not
survive across separate calls (`mjx_ooxml_core::Interner` is not `Clone`, and every `Document`
accessor already re-parses its part fresh); the guide states the caller-side alternative for a loop
over many runs.

**A gap found and fixed while wiring the ladder:** `StyleParagraphProperties` (MJXOFF-101) modelled
`w:spacing`/`w:ind` structurally (both round-tripped) but exposed no `spacing()`/`indentation()`
accessor at all — a caller could not read or write a style's own spacing/indentation. Added with the
same `value_property!` macro every sibling accessor already uses.

`crates/mjx-docx/docs/effective_properties.md` is wired into a real doctest gate the way
`mjx-pptx`'s own page is — `src/effective_properties.rs` is `#![doc = include_str!(...)]` with no
items of its own, so the guide's snippets are compiled by `cargo test --doc`, not merely present;
proved by breaking one assertion and watching the doctest go red before restoring it.

## [0.0.87] - 2026-09-04

Word numbering definitions (MJXOFF-104, Phase C position 8): `word/numbering.xml` in full —
abstract numbering definitions (`w:abstractNum`, up to nine `w:lvl` each), numbering instances
(`w:num`), per-instance level overrides (`w:lvlOverride`), picture bullets (`w:numPicBullet`), and
the two-hop resolution from a paragraph's `w:numPr` to the level it actually uses.

**Two-hop resolution, indexed by real key, never by position.** `w:numPr/w:numId` names a `w:num`;
that instance's own `w:abstractNumId` names a `w:abstractNum`; the abstract definition holds the
levels. `NumberingIndex` (built once from a `&Numbering` snapshot, the same design
`StyleIndex`/MJXOFF-101 already uses) indexes both hops by `numId`/`abstractNumId`, never by
document-order position — `numId` values need not be contiguous or ascending, and two instances may
share one abstract definition. `tests/fixtures/numbering_definitions.docx`, authored for this child
(no fixture in the corpus carried `word/numbering.xml` at all), seeds exactly that trap: `numId` 2
and 5 share one abstract definition, deliberately out of order against `numId` 9, and only `numId` 2
carries a `w:lvlOverride/w:startOverride`. Mutation-proved: neutralising the override handling turns
`numId` 5's own (un-overridden) resolved start wrong too, confirmed red and restored by re-editing.

**`numId = 0` is "no numbering", not a lookup failure; a genuinely dangling `numId` is a typed
error.** Proved against the real, already-committed `tests/fixtures/paragraph_properties.docx`
(MJXOFF-96), which carries a real `w:numPr` (`numId` 5) while relating to no `word/numbering.xml` at
all — not only against a synthetic case.

**`w:numStyleLink` resolves through `StyleIndex` — the seam between two OPC parts.**
`Document::resolve_numbering` follows the redirect (a numbering-type style's own `w:pPr/w:numPr`
substitutes for the numStyleLink-carrying definition's own, typically empty, level list), one part
parse at a time since each OPC part carries its own `Interner` and two cannot be held open on the
same `Package` at once; bounded (`MAX_NUM_STYLE_LINK_DEPTH`), the same design
`MAX_BASED_ON_CHAIN_DEPTH` already uses for `w:basedOn`.

**Displayed list numbers are not computed** — deliberately. Turning a resolved level into "1.2.3"
or a bullet glyph requires counting every preceding paragraph in the same list, `w:lvlRestart`,
restart on entering a higher level, and continuation across sections; a counter correct only for a
flat single-level list would be actively misleading. `numbering.rs`'s own module doc states the
boundary explicitly, in the style the PowerPoint effective-properties page already uses for its own
deliberate absences. Rendering a list's text remains MJXOFF-106's.

**Two ticket corrections, verified directly against `wml.xsd`:** `CT_NumRestart` and
`CT_TrackChangeNumbering` are both unreachable from `CT_Numbering` — the former is
footnote/endnote restart (`EG_FtnEdnNumProps`, MJXOFF-124's scope), the latter is `CT_NumPr`'s own
tracked-change wrapper (already opaque, MJXOFF-96) and `CT_FldChar`'s. Neither belongs to this
child.

`CT_Lvl`'s own `w:pPr`/`w:rPr` are `CT_PPrGeneral`/`CT_RPr` — confirmed against the schema, not
assumed — so `NumberingLevel` reuses `StyleParagraphProperties` (MJXOFF-101) and `RunProperties`
(MJXOFF-94) directly rather than restating either. Picture bullets preserve whichever payload a
real file carries (`w:pict` legacy VML, the common case, or `w:drawing`) as opaque, pending
MJXOFF-113/MJXOFF-131's own typed models. `Document::{numbering, edit_numbering,
attach_paragraph_to_list, detach_paragraph_from_list}` mirror the `styles.xml` authoring surface,
including creating `word/numbering.xml` — relationship and content type — on first use.

## [0.0.86] - 2026-09-04

Word style definitions (MJXOFF-101, Phase C position 7): `word/styles.xml` in full —
`w:docDefaults`, every `CT_Style` member, `w:basedOn` chain resolution with cycle safety, and
`w:latentStyles`.

**`CT_Style/w:pPr` is `CT_PPrGeneral`, not `CT_PPr`.** Verified directly against `wml.xsd`, not
assumed from the ticket's own text (which named `CT_PPrGeneral` as already built — it was not):
`CT_PPr` (a live paragraph's own `w:pPr`, MJXOFF-96) is `CT_PPrBase` plus `rPr` (`CT_ParaRPr`),
`sectPr` and `pPrChange`; `CT_PPrGeneral` — what a style definition, `w:pPrDefault` and
`w:tblStylePr` all actually carry — is `CT_PPrBase` plus `pPrChange` only. A style's own paragraph
properties may not carry a pilcrow's run properties or a section break, so `StyleParagraphProperties`
is its own container rather than `ParagraphProperties` reused; every one of its 33 leaf types
(`Toggle`, `FrameProperties`, `Spacing`, `ParagraphBorders`, …) is still the exact struct
MJXOFF-96 built, reused directly — only the wiring is new. `CT_Style/w:rPr`, by contrast, genuinely
is plain `CT_RPr` and reuses `RunProperties` with no wrapper at all. `w:tblPr`/`w:trPr`/`w:tcPr`
(on both `CT_Style` and `CT_TblStylePr`) stay opaque, the same treatment `w:pPrChange` already
gets — no shipped crate models table properties yet, and inventing a first model of them here would
be scope this child was not given.

**Cycle safety is a bounded depth, not a visited-set — and hitting the bound is a typed error,
never a silently truncated chain.** `StyleIndex::based_on_chain` walks `w:basedOn` from a style
upward, accumulating each ancestor into the `Vec` it must return anyway; that accumulation *is*
the bound (`MAX_BASED_ON_CHAIN_DEPTH = 64`) — a chain that has not terminated by then returns
`Err(DocxError::BasedOnChainTooDeep)`, never a partial `Ok` chain a later caller could resolve
properties against without anything going red. Proved by mutation: turning the bound check into a
silent `break` (an `Ok` chain of 64 repeated entries instead of an error) turns both cycle tests red.
`sample.docx`'s `Normal` style does **not** self-reference — checked directly against the fixture's
own bytes by two independent methods, refuting an earlier dispatch brief's claim — so the corpus has
no cycle to test against; `tests/fixtures/style_based_on_cycle.docx` (a self-reference and a mutual
pair) is the only cycle evidence in the suite, and `based_on_chain` is separately exercised against
`sample.docx`'s own real, non-cyclic chains to prove the depth cap never false-positives.

**The three-deep `basedOn` trap, closed with a discriminating fixture:** `Base → Middle → Leaf`,
where `Middle` overrides `Base`'s font size and `Leaf` overrides nothing, so `Leaf`'s correct
effective font size can only come from walking to `Middle` — reading only the leaf, only the base,
or only direct properties each gives a different wrong answer. Mutation-proved: neutralising the
chain walk (stop after the first push) turns this test red.

`w:styleId` matching is case-sensitive; `w:name` matching is case-insensitive (full Unicode case
fold), matching Word's own "apply style by name" UI — `sample.docx` already shows two producers
disagreeing on capitalisation (`PreformattedText` vs. `"Preformatted Text"`). `w:link` resolves in
both directions through `LinkedStyleResolution`, reporting a missing or wrong-kind target as a
value, never a panic.

`w:count` on `w:latentStyles` is preserved, never silently recomputed — `LatentStyles::sync_count`
is the explicit, opt-in way to keep it consistent with the exception list after an edit.
`tests/fixtures/style_latent_styles.docx` is the only committed coverage: `sample.docx` carries no
`w:latentStyles` at all (checked directly).

`Document::edit_style_sheet` creates `word/styles.xml` — content-type registration and the
`styles` relationship from the main document part — on first use for a document that has none (a
[`Document::blank`] document, among others), then runs the same parse/mutate/write-back shape
every other typed edit in this crate uses; `Document::style_sheet` is its read-only, closure-based
counterpart.

### Added

- **`mjx_docx::{StyleSheet, StyleDefinition, DocumentDefaults, DefaultRunProperties,
  DefaultParagraphProperties, LatentStyles, LatentStyleException, StyleParagraphProperties,
  TableStyleOverride, StyleString, RevisionSaveId}`** and their content enums — the full
  `word/styles.xml` model (`CT_Styles`, `CT_Style`, `CT_DocDefaults`, `CT_RPrDefault`,
  `CT_PPrDefault`, `CT_LatentStyles`, `CT_LsdException`, `CT_PPrGeneral`, `CT_TblStylePr`).
- **`mjx_docx::{StyleIndex, LinkedStyleResolution, MAX_BASED_ON_CHAIN_DEPTH}`** — the style index
  (built once from a `&StyleSheet` snapshot, reused for every lookup), `w:basedOn` chain walking,
  and `w:link` resolution.
- **`mjx_docx::Document::{style_sheet, edit_style_sheet}`** — reading and authoring
  `word/styles.xml`, creating it (with its relationship and content type) on first use.
- **`mjx_docx::DocxError::{UnknownStyleId, BasedOnChainTooDeep}`**.
- New fixtures: `tests/fixtures/style_based_on_chain.docx`, `style_based_on_cycle.docx`,
  `style_latent_styles.docx` — authored for this child; `sample.docx` supplies neither a
  three-deep override chain, a `basedOn` cycle, nor `w:latentStyles`.
- **`mjx_ooxml_types::child_order::{PARAGRAPH_PROPERTIES_GENERAL, DOCUMENT_DEFAULTS,
  DEFAULT_RUN_PROPERTIES, DEFAULT_PARAGRAPH_PROPERTIES, LATENT_STYLES, STYLE_DEFINITION, STYLES,
  TABLE_STYLE_OVERRIDE}`** — generated child-order tables for `CT_PPrGeneral`, `CT_DocDefaults`,
  `CT_RPrDefault`, `CT_PPrDefault`, `CT_LatentStyles`, `CT_Style`, `CT_Styles` and `CT_TblStylePr`
  (`xtask/src/codegen/spec.rs::CHILD_ORDER_EXPORTS`).

## [0.0.85] - 2026-09-04

Word document authoring from nothing (MJXOFF-98, Phase C position 6): `Document::blank` and
`Document::blank_with_properties`, mirroring `mjx_pptx::Presentation::blank`'s shape — one call, no
file, no template. On top of `mjx_opc::Package::empty` (the same OPC primitives `mjx-pptx`'s own
`blank.rs` uses), this writes `word/document.xml` (one empty paragraph and a body-level `w:sectPr`
naming the caller's page) plus `docProps/core.xml`/`docProps/app.xml` (MJXOFF-149's packaging-layer
decision, restated rather than re-derived). `PageSize`/`PageOrientation` give the caller `a4()` or
`us_letter()`, portrait or `landscape()`, refused with a typed `DocxError::InvalidPageSize` before
any byte is written if this crate's fixed "Normal" margins (1 inch, matching Word's own template)
would leave no printable area.

**Which optional parts a blank document gets, and why the answer differs from PowerPoint's own.**
`tests/fixtures/sample.docx` — LibreOffice's own output — ships ten parts, four beyond
`word/document.xml` and the two `docProps`: `styles.xml`, `fontTable.xml`, `settings.xml` and
`theme/theme1.xml`. None is schema-required (`wml.xsd`'s thirteen part-bearing global elements are
all `minOccurs="0"` from wherever they are reached). `mjx_pptx::blank`'s answer to the same "what
beyond the schema minimum" question is to include the master, layout and theme, because without them
a deck is *structurally* unusable — there is no layout to build a slide from. WordprocessingML has no
such dependency: a paragraph with no `w:pStyle` and a run with no `w:rStyle` are both legal, and every
real Word implementation falls back to a built-in appearance when a document names no style to
inherit from, so `Document::blank`'s body is fully usable through MJXOFF-92's `insert_paragraph` /
`append_run` / `set_run_text` with zero related parts. Writing even a throwaway `docDefaults`-only
`styles.xml` — legal under this ticket's own wording — would be work MJXOFF-101 replaces on day one,
so this module writes none of the four, and `crates/mjx-docx/src/blank.rs`'s module doc names every
inclusion and every deliberate absence.

**A ticket correction, caught by checking `wml.xsd` directly rather than trusting the brief's own
claim:** `w:sectPr`, `w:pgSz` and `w:pgMar` are *not* schema-required either — `CT_Body`'s `sectPr`,
and `pgSz`/`pgMar` inside `EG_SectPrContents`, are all `minOccurs="0"`. All three are included for the
same "not required, but what makes the result usable" reasoning `mjx_pptx::blank` uses for its
placeholders, not because the schema demands them. The one attribute-level claim that genuinely is
`use="required"` — `CT_PageMar`'s seven attributes (`top`, `right`, `bottom`, `left`, `header`,
`footer`, `gutter`), if `w:pgMar` is written at all — is proved by mutation, once per attribute, in
`tests/schema_gate.rs`.

**A second, unrelated defect the schema gate caught while building this:** the first draft of
`document_bytes` wrote `w:pgSz`'s and `w:pgMar`'s attributes with no `w:` prefix (`w="11906"` rather
than `w:w="11906"`) — `wml.xsd` is `attributeFormDefault="qualified"`, the same class of defect
MJXOFF-152 fixed for this crate's typed attribute accessors, this time in a hand-written XML
template rather than a codec. `xmllint` rejected it immediately (`the attribute 'w' is not allowed`),
before it ever reached a test file.

The LibreOffice open canary (`tests/office_open.rs`, mirroring `mjx-pptx`'s own) is implemented and
skips cleanly on a machine with no `soffice` installed; `crates/mjx-docx/examples/blank_document.rs`
and the crate's first guide page (`crates/mjx-docx/src/guide.rs`, `building_a_document`) are the
runnable and prose versions of the same story.

## [0.0.84] - 2026-09-04

Word paragraph properties (MJXOFF-96, Phase C position 5): `w:pPr` (`CT_PPr`) and all 33
`CT_PPrBase` children, plus the paragraph mark's own run properties (`w:pPr/w:rPr`, `CT_ParaRPr`).
`CT_PPrBase` is the other half of Word's direct formatting — the base MJXOFF-101 (styles) and
MJXOFF-109 (numbering levels) both build on.

Two traps this child exists to close: the paragraph-mark run properties are not a run's own — `w:b`
set through `Paragraph::paragraph_mark_properties_or_insert` can never touch a run's `w:rPr`, and
setting the paragraph's justification can never touch the pilcrow's — proved on bytes, not just by
type distinctness. And `w:spacing/@line` is meaningless without `@lineRule` (`auto` means 240ths of a
line, `exact`/`atLeast` mean twips): there is no `Spacing::line` accessor, only
`Spacing::line_spacing`, which always returns both together (`LineSpacing`), demonstrated by a
doctest.

`CT_ParaRPr` reuses `run_properties.rs`'s (MJXOFF-94) 39 `EG_RPrBase` leaf types directly —
`Toggle`, `Fonts`, `Color`, `Border`, `Shading`, … — rather than restating them; only `Toggle::new`
and `HalfPointMeasureValue::new` needed widening from private to `pub(crate)` to make that reuse
possible. `CT_PBdr`'s six borders and `w:pPr/w:shd` likewise reuse `CT_Border`/`CT_Shd`
(`super::run_properties::{Border, Shading}`) rather than defining a second border or shading type.

`CT_Ind`'s logical (`w:start`/`w:end`) and physical (`w:left`/`w:right`) spellings are both preserved
independently — nothing is normalised on write — with `Indentation::leading_edge`/`trailing_edge`
resolving between them when a file carries both: the logical spelling wins, since Annex M records it
as the later, Strict-compatible addition (ECMA-376 Part 1's own prose states no explicit precedence
here, unlike the `…Chars`-supersedes-twips rule it does state).

One correction to the ticket's own text: `w:kinsoku` inside `w:pPr` is `CT_OnOff` (a plain toggle),
not the two-attribute `CT_Kinsoku` complex type named in the ticket's "Complex types" list — that
type belongs to `w:noLineBreaksAfter`/`w:noLineBreaksBefore` in document settings, unrelated to
paragraph properties. `CT_DecimalNumberOrPrecent` (`w:summaryLength`) and `CT_ParaRPrOriginal`
(reachable only through `w:pPrChange`, MJXOFF-126's scope) are likewise not `CT_PPrBase` children and
have no home in this child.

New fixture: `tests/fixtures/paragraph_properties.docx` — the only committed `.docx` carrying
`w:line`, `w:tabs`, `w:ind`, `w:pBdr` or `w:framePr` before this child; its two paragraphs cover all
33 `CT_PPrBase` members between them, including the legacy physical indentation spelling and a
`w:tab` with `val="clear"` (which removes an inherited stop rather than adding one, so is preserved
structurally like any other stop).

Reachability: `Paragraph::properties`/`properties_mut`/`properties_or_insert` reach `w:pPr` from
`Paragraph`'s own public surface — MJXOFF-152 found `CT_R`'s legacy leaf types were correct but
unreachable through `Document`/`Body`/`Paragraph`/`Run`; this child does not repeat that gap for
`w:pPr` itself. `ParagraphProperties::section_properties`/`change` similarly reach `w:sectPr`/
`w:pPrChange` structurally (as `Unmodeled`), ahead of MJXOFF-106/MJXOFF-126 giving them real content.

### Added

- **`mjx_docx::ParagraphProperties`** (`CT_PPr`, `w:pPr`) — all 33 `CT_PPrBase` members plus the
  paragraph mark's own properties, the section this paragraph ends, and the tracked-change wrapper.
  Reached off `Paragraph::properties`/`properties_mut`/`properties_or_insert`.
- **`mjx_docx::ParagraphMarkRunProperties`** (`CT_ParaRPr`, `w:pPr/w:rPr`) — the pilcrow's own
  character formatting, distinct from a run's `w:rPr`.
- **`mjx_docx::{Spacing, LineSpacing, Indentation, FrameProperties, TabStops, TabStop,
  ParagraphBorders, NumberingProperties, ConditionalFormatting, ParagraphStyle, ParagraphAlignment,
  ParagraphTextFlowDirection, VerticalCharacterAlignment, TextBoxTightWrapSetting,
  DecimalNumberValue}`** and their content enums — the leaf and container types `CT_PPrBase`'s 33
  members are built from.
- **`mjx_ooxml_types::child_order::{PARAGRAPH_PROPERTIES, PARAGRAPH_MARK_RUN_PROPERTIES,
  PARAGRAPH_BORDERS, NUMBERING_PROPERTIES}`** — generated child-order tables for `CT_PPr`,
  `CT_ParaRPr`, `CT_PBdr` and `CT_NumPr` (`xtask/src/codegen/spec.rs::CHILD_ORDER_EXPORTS`).

## [0.0.83] - 2026-09-04

Fixes the defect 0.0.82's own changelog reported and left open (MJXOFF-152): `crates/mjx-docx/src/
document/body.rs`'s `Break`/`PositionalTab`/`Symbol`/`ProofingError`/`PermissionRangeStart`/
`PermissionRangeEnd` (MJXOFF-92) declared their attributes with no `prefix`, so every accessor
matched only a bare, unprefixed local name — but `wml.xsd` is `attributeFormDefault="qualified"`,
and real markup writes `w:font`, `w:alignment`, `w:type`, never bare. Every accessor on these six
types returned `None` (or `Missing`, for a required attribute) against a file that plainly carries
the value — confirmed against `run_content.docx`'s own `<w:sym w:font="Wingdings" w:char="F0E0"/>`
and `<w:ptab w:alignment="right" w:relativeTo="margin" w:leader="dot"/>`, committed since MJXOFF-92
and never once read correctly. Round-trip fidelity was unaffected throughout — the attribute vector
is retained and re-emitted verbatim regardless, which is why every byte-identity suite and the
schema gate stayed green; only the typed reads were broken, and nothing exercised them.

Audited every `#[xml(attribute(…))]` declaration in the file against `wml.xsd` by hand: 17 of 19
needed `prefix = "w"` added; the other two were already correct (`CT_Text`'s `xml:space`, prefix
`xml`; `CT_Rel`'s `id`, prefix `r` — a relationship reference into a different namespace's own
schema, not `wml`'s). **Do not blanket-add `w`** applied literally: those two stayed untouched.

Second, unrelated defect caught by the same audit: `body.rs`'s two local `AttributeCodec` tag types
(`WhitespacePreservation`, `ShortHex`) were private. That compiles inside the crate — same-module
visibility hides it — but `Text::preserve_whitespace` and `Symbol::character` name the private type
in their return type via `AttributeCodec::Value`, which is a hard compile error for any caller
outside this crate. The same class MJXOFF-94 found and fixed for `run_properties.rs`'s own seven
tag types the release before this one. Made both `pub` and re-exported.

A workspace-wide audit (MJXOFF-152's own scope, not just `mjx-docx`) confirmed `wml.xsd` and
`shared-math.xsd` are the *only* two of the schemas this project models that declare
`attributeFormDefault="qualified"`; `sml.xsd`, `pml.xsd` and `dml-main.xsd` declare no
`attributeFormDefault` at all (XSD's default is `unqualified`), and `dml-chart.xsd`,
`dml-diagram.xsd` and `vml-main.xsd` say `unqualified` explicitly. `mjx-dml`'s 318 attribute
declarations (5 of them correctly `prefix = "r"` for relationship references, the rest correctly
unprefixed) confirm the unqualified reading in practice; `mjx-chart`, `mjx-vml` and `mjx-pptx`
declare no typed attribute accessors yet, so there was nothing there to audit. **`shared-math.xsd`
being qualified is a live warning for MJXOFF-134** (`mjx-omml`, not yet written): its leaf types will
need the same `prefix = "w"`-style treatment `wml.xsd` needed here, from the first declaration,
recorded on that ticket.

### Fixed

- **`mjx_docx::{Break, PositionalTab, Symbol, ProofingError, PermissionRangeStart,
  PermissionRangeEnd}`** — every attribute accessor now reads the value real, `w:`-prefixed markup
  states, proved against `run_content.docx` (already committed) and a new
  `tests/fixtures/leaf_attributes.docx` (for the three elements — `w:br` with real values,
  `w:proofErr`, `w:permStart`/`w:permEnd` — neither existing fixture carries with attribute values
  set, so neither could discriminate this defect).
- **`mjx_docx::{WhitespacePreservation, ShortHex}`** — made `pub` and re-exported; both were private,
  which made `Text::preserve_whitespace` and `Symbol::character` uncallable (a compile error) from
  outside this crate.

## [0.0.82] - 2026-09-04

Run properties (MJXOFF-94): `w:rPr` and the character-formatting vocabulary — `EG_RPrBase`'s **39
members** (the ticket said 38 plus `oMath`; the schema gives the group exactly 39, and `oMath` is
`CT_OnOff`-shaped like nineteen of its siblings, not a fortieth special case). `EG_RPrBase` is the
most-referenced group in `wml.xsd`: `CT_RPr`, `CT_ParaRPr`, `CT_RPrOriginal`, `CT_ParaRPrOriginal`,
`CT_Style` and `CT_RPrDefault` all build on it, so MJXOFF-96, MJXOFF-101, MJXOFF-104, MJXOFF-119 and
MJXOFF-126 all needed this landed first.

### Added

- **`mjx_docx::RunProperties`** (`CT_RPr`), reached off `Run::run_properties`/`run_properties_mut`/
  `run_properties_or_insert` — the last placing a freshly authored `w:rPr` at its schema rank via the
  generated `wml` child-order table.
- **`mjx_docx::Toggle`** — the twenty `CT_OnOff`-shaped members (`b`, `bCs`, `caps`, `cs`, `dstrike`,
  `emboss`, `i`, `iCs`, `imprint`, `noProof`, `oMath`, `outline`, `rtl`, `shadow`, `smallCaps`,
  `snapToGrid`, `specVanish`, `strike`, `vanish`, `webHidden`) share one type, reused exactly as
  `mjx_docx::Text` is reused across four `EG_RunInnerContent` members. `val` is declared with the
  attribute grammar's `default = true` — ECMA-376 Part 1's own prose for every one of these elements
  ("if this element is present without a val attribute, its default value is true") — so
  `RunProperties`'s twenty per-property accessors (`bold`, `italic`, …) return `Option<bool>`: `None`
  for the element absent, `Some(true)`/`Some(false)` for present-and-on/present-and-off, never
  collapsed to a bare `bool`.
- **The other eighteen complex types**: `CharacterStyle`, `Fonts`, `Color`, `Underline`, `TextEffect`,
  `Border`, `Shading`, `VerticalAlignment`, `ManualRunWidth`, `Emphasis`, `Languages`,
  `EastAsianLayout`, `Highlight`, and three measure-value wrappers (`HalfPointMeasureValue`, reused
  across `sz`/`szCs`/`kern`; `SignedHalfPointMeasureValue` for `position`;
  `SignedTwipsMeasureValue` for `spacing`) and `TextScaleValue` for `w`. Colour (`Color`,
  `Underline`'s and `Border`'s and `Shading`'s own colour attributes) is Word's own four-attribute
  model (`val`, `themeColor`, `themeTint`, `themeShade`) — not DrawingML's `a:schemeClr` with child
  transforms.
- **`tests/fixtures/run_properties.docx`** — the three `w:rPr` emptiness states `sample.docx` and
  `run_content.docx` don't between them cover (self-closed, absent, and a separate end tag with no
  children), and a run carrying all 39 properties at once, including `w:b w:val="0"` (explicit off,
  distinct from absent), `w:rFonts` with only a hint and no font name, and `w:u` with `color` and
  `themeColor` alongside `val`.

### Fixed

- Caught while writing this child's own tests: `wml.xsd` is `attributeFormDefault="qualified"`, so
  every WordprocessingML attribute is written `w:val`, not `val` — but nothing in this new
  vocabulary's attribute declarations named a `prefix`, so every single one matched only the
  unprefixed spelling and silently fell through to its schema default. `crates/mjx-docx/src/document/
  body.rs`'s pre-existing `Break`/`PositionalTab`/`Symbol`/`ProofingError`/`PermissionRangeStart`/
  `PermissionRangeEnd` (MJXOFF-92) carry the same latent defect on their own attributes, untested for
  the same reason: their round-trip tests pass the whole attribute vector through verbatim and never
  call the typed accessors. Not fixed here — out of this child's scope — and reported on the ticket.

## [0.0.81] - 2026-09-04

The WordprocessingML block content model (MJXOFF-92): `mjx-docx` could open a `.docx` and name its
parts (MJXOFF-90) but could not read a single word of one. This gives it `w:body`'s block content —
paragraphs, runs, text and the rest of `EG_RunInnerContent`'s 33 members — the content spine every
later Word child hangs off.

### Added

- **`mjx_docx::{Body, Paragraph, Run, Text, Hyperlink}`** and the two content enums that hold them
  together — `BlockContent` (`EG_ContentBlockContent`, plus `w:sectPr`) and `ParagraphContent`
  (`EG_PContent`). `Paragraph`/`Run`/`Hyperlink` are typed for real reach — a `w:hyperlink`'s own
  runs stay reachable; `w:customXml`/`w:smartTag`/`w:sdt`/`w:dir`/`w:bdo`/`w:tbl` stay
  `mjx_docx::Unmodeled` (opaque, unowned) until a later child claims one.
- **`mjx_docx::RunInnerContent`** — all 33 `EG_RunInnerContent` members, every one with a variant now
  (`Break`, `Text` reused for `t`/`delText`/`instrText`/`delInstrText`, `RelationshipReference`,
  `Symbol`, `PositionalTab`, `PhoneticGuide` fully typed; the sixteen `CT_Empty`-based members and
  seven later-child payloads — `w:fldChar` (MJXOFF-121), `w:object`/`w:pict`/`w:drawing`
  (MJXOFF-131), `w:footnoteReference`/`w:endnoteReference`/`w:commentReference` — stay `Unmodeled`).
  Adding a variant later would be a breaking change to an enum fifteen children depend on; a variant
  whose payload is still `Unmodeled` is not.
- **`mjx_docx::{BlockPath, RunPath}`** — the address of a paragraph and of a run, in
  `crates/mjx-docx/src/address.rs`, mirroring `mjx_pptx::ShapePath`'s manners (a bare index for the
  common case, an array/slice/`Vec` to descend a level) for WordprocessingML's own kind of nesting —
  block containers for paragraphs, run containers (`w:hyperlink`, so far) for runs — rather than
  `p:grpSp` groups.
- **`Document::{paragraph_count, run_count, paragraph_text, run_text, set_run_text, insert_paragraph,
  append_paragraph, remove_paragraph, insert_run, append_run, remove_run}`** — reading and editing
  paragraphs and runs, each edit going through `ToXml::write_back` so only the touched subtree
  re-serializes.
- **The `xml:space` rule** for `w:t` (`Text::set_text`): writes `xml:space="preserve"` when the new
  text starts or ends with ASCII whitespace, and removes the attribute otherwise — reading never
  trims, regardless.
- **`tests/fixtures/run_content.docx`** — a fixture carrying `w:br`, `w:tab`, `w:sym`, `w:cr`,
  `w:noBreakHyphen`, `w:ptab`, `w:ruby`, a `w:t` with `xml:space="preserve"`, a `w:hyperlink`
  wrapping two runs, and a `w:fldChar` (a run-inner element whose payload is still `Unmodeled`),
  swept automatically into every byte-identity suite and the schema gate.

### Also in this release: the Word crate spine (MJXOFF-90)

MJXOFF-90 shipped without a version bump of its own, so its work reaches a release here rather than
in a `0.0.81` of its own. Recorded rather than renumbered — the history is linear and a rewrite
would cost more than the misfiled heading does.

- **`mjx_docx::{Document, PartKind, DocumentParts, DocxError}`** — `Document::open`/`save`/
  `save_unchecked`/`validate`, mirroring `Presentation`'s names so the Word method is guessable from
  the deck one, and the part graph over `wml.xsd`'s fourteen global elements. `crates/mjx-docx` was
  thirteen lines and zero public items before it.
- **`xtask`'s child-order generator resolves `xsd:complexContent` and `xsd:simpleContent`.** An
  extension splices the resolved base chain *before* the derived type's own particle; a restriction
  replaces it; `simpleContent` contributes nothing. This is why `wml` can have an ordering table at
  all — its schema uses `complexContent` in 41 derived types — and it is what unblocks MJXOFF-132
  (`sml`, 6 `simpleContent`) and MJXOFF-134 (`shared-math`, 2) without either repeating the work.
- **The `wml` child-order table**, generated and committed, with the ordering audit proved red then
  green on real WordprocessingML.

## [0.0.80] - 2026-09-04

Document properties (MJXOFF-149): the programme held two contradictory positions on `docProps/*` —
"deliberately absent" in `mjx-pptx`'s own blank-deck module doc, and already assumed in two Word/Excel
tickets' part lists. Settled in favour of authoring: every file real Office writes carries
`docProps/core.xml` and `docProps/app.xml`, and `mjx-schema-gate`'s three-category rule was written
anticipating exactly this flip.

### Added

- **`mjx_opc::doc_props`** — `CoreProperties` (`title`, `creator`, `created`, `modified`),
  `ExtendedProperties` (`application`) and `DocumentTimestamp` (built only from explicit calendar
  fields — there is no `now()`), plus the writer, part-name, content-type and relationship-type
  constants for `docProps/core.xml` (ECMA-376 Part 2's `opc-coreProperties.xsd`, Dublin Core) and
  `docProps/app.xml` (`shared-documentPropertiesExtended.xsd`). Packaging-layer, so `mjx-pptx` and
  the Word/Excel `blank()` constructors still to come share one implementation.
- **`mjx_pptx::Presentation::blank_with_properties`** — `blank` with document properties set, rather
  than left absent. `blank` itself now writes both parts on every call, all-`None` by default (a
  schema-valid, childless part, since both are `xs:all` groups with every child optional).

### Fixed

- `mjx-schema-gate`'s `opc-coreProperties` and `shared-documentPropertiesExtended` namespaces move
  from the preserved-foreign allowlist to the modelled-schema table: `docProps/core.xml` and
  `docProps/app.xml`, in every fixture and every authored deck alike, are now genuinely validated
  against ECMA-376 rather than skipped as foreign markup. `opc-coreProperties.xsd`'s Dublin Core
  imports (`dc:`, `dcterms:`, real network `schemaLocation`s, unlike `wml.xsd`'s bare `xml:` import)
  are resolved through a committed local XML catalog rather than a live fetch.

## [0.0.79] - 2026-09-03

The DrawingML diagram (SmartArt) model (MJXOFF-148): `add_diagram` authored `dgm:` markup this
project neither modelled nor ordered, which is exactly the condition MJXOFF-110 exists to make
impossible. Closes the hole.

### Added

- **`mjx_dml::diagram`** — a typed model of `dml-diagram.xsd`, 50 of its 58 complex types down to
  their attributes: the data part as a point-and-connection graph (`DataModel`, `PointList`/`Point`,
  `ConnectionList`/`Connection`), the layout definition's whole algorithm tree (`LayoutDefinition`,
  `LayoutNode`, `Algorithm`, `Constraint`, `NumericRule`, `Choose`), the quick style
  (`StyleDefinition`/`StyleLabel`) and the colour transform
  (`ColorTransform`/`StyleLabelColors`/`ColorList`). A handful of externally-defined DrawingML
  formatting groups (`spPr`, `style`, `txPr`, `bg`, `whole`, `scene3d`, `sp3d`) and the SmartArt
  gallery-catalog header types this project never authors or reads stay unmodelled, by name and
  reason, in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md`. Running a `dgm:layoutDef` to compute
  where a consumer draws each point remains a documented non-goal — a rendering concern.
- **`mjx_ooxml_types::diagram`** — the whole `ST_*` family of `dml-diagram.xsd` (66 simple types),
  comprehensively named; `dml-diagram` joins `CHILD_ORDER_SCHEMAS`, so an authored diagram's four
  parts are ordered by construction rather than emitted from a fixed template with no writer checking
  its sequence.

### Fixed

- `mjx-schema-gate`'s `dml-diagram` row now validates for real: `add_diagram`'s four parts were
  already checked against `dml-diagram.xsd`, and a new case proves the check is live by writing
  markup the schema rejects and asserting it is caught, naming the schema — not merely that markup
  this project already writes happens to pass.

## [0.0.78] - 2026-09-03

A performance baseline and a large-file corpus generator (MJXOFF-147) — the numbers MJXOFF-95 (the
Excel cell store) designs its memory budget against, and the numbers a later regression is compared
to instead of intuition. Not an optimisation pass: nothing here got faster, the point is knowing.

### Added

- **`cargo run -p xtask -- corpus`** — (re)builds a git-ignored large-file corpus into
  `target/corpus/`: a 300-slide `.pptx` (`mjx_pptx::Presentation`'s real edit surface), a
  20,000-paragraph `.docx` and a 300,000-cell `.xlsx` (raw WordprocessingML/SpreadsheetML on
  `mjx_opc::Package` — neither format has a model yet), and prints size/element/cell counts.
  `corpus --mem <pptx|docx|xlsx>` runs its peak-resident-set checkpoints (open / first-mutation
  materialisation / edit / save) in one process via `/proc/self/status`'s `VmHWM`, the kernel's own
  peak-RSS counter — chosen over a counting allocator because it answers the literal question asked
  ("peak resident set") rather than a proxy for it. Not a substitute for MJXOFF-130's Office-authored
  fixtures, and does not claim to be.

- **Criterion benchmarks** — `crates/{mjx-pptx,mjx-docx,mjx-xlsx}/benches/`, six operations per
  format (`open`, `first_mutation_materialisation`, `edit_after_materialised`, and the three save
  paths `save_untouched` / `save_lightly_edited` / `save_fully_materialized`, measured separately
  because the gap between them is the result), plus a seventh for `mjx-pptx` exercising the real
  `Presentation` edit surface rather than only the lower `Package` layer.

- **`docs/BENCHMARKS.md`** — the baseline: the machine, the (existing, previously undocumented)
  release profile, all four operations × three formats' time and peak RSS, the three save paths
  compared, A7d's `mjx248_measure` reproduced on this machine (matches within ~10–35%, one direction,
  explained by MJXOFF-143), the short-list of figures MJXOFF-95 designs against, and two measurement
  bugs this child caught in its own harness before trusting its numbers.

### Findings, filed rather than fixed here

- Materialising the 610,005-element / 300,000-cell worksheet costs **+274 MiB of peak RSS** over an
  8.54 MiB raw-XML part — roughly 32× the source bytes, ≈ 913 B/cell. Filed as **MJXOFF-151** (under
  MJXOFF-88) for MJXOFF-95 to design against (an arena/columnar layout, per `PLAN.md`'s hybrid model),
  not fixed in this child.

## [0.0.77] - 2026-09-03

The untrusted-input paths are fuzzed, and three defects they were hiding are fixed (MJXOFF-146).

`CLAUDE.md` has always said it: *no `unwrap`/`panic`/`expect` on untrusted input — inputs are
untrusted files.* Nothing in the repository proved it. A grep finds the obvious cases and says
nothing about a recursion depth, a slice index, or an allocation an attacker sizes. This adds a
campaign that tries, and it found three things a grep could not.

### Added

- **`cargo run -p xtask -- fuzz`** — a campaign against the three untrusted-input entry points
  (`mjx_xml::fidelity::parse`, `mjx_opc::Package::open`, `mjx_mce::resolve`) plus the round-trip
  oracle, in five targets. Run **on demand, not on every push**; `--list`, `--target`, `--seed`,
  `--iterations` and `--seconds` select and bound a run, and a seed makes one reproducible.

  It is stable Rust with no new dependency. `cargo-fuzz` needs a nightly toolchain for its sanitizer
  flags, and a gate only some machines can run is not a gate. It lives in `xtask`, which is host-only
  and which nothing depends on, so the harness cannot reach the shipped graph.

  It asserts properties rather than the absence of a crash: every input the reader accepts must
  re-serialize **byte-for-byte**; the same corpus is re-run with the document dirtied at its root,
  where a byte range that does not describe its element shows; a package written back and reopened
  must hold the same part bytes. Panics are caught per execution, a counting global allocator
  measures each execution's peak against a ceiling so unbounded allocation is a *finding* rather than
  an OOM kill, and a watchdog turns a hang into an abort that names its input.

- **`mjx_fixtures::adversarial_xml`** and **`adversarial_xml_dirtied_at_the_root`** — the hostile XML
  corpus, moved out of `crates/mjx-xml/tests/subtree_cow.rs` so the hand-written gate and the
  campaign read the same list instead of drifting apart.

- **`mjx_xml::fidelity::MAXIMUM_DEPTH`** and **`XmlError::DepthLimit`** — see below.

- Regression suites for every finding, in the crate that owns the path:
  `crates/mjx-xml/tests/untrusted_input.rs`, `crates/mjx-opc/tests/untrusted_input.rs`,
  `crates/mjx-mce/tests/untrusted_input.rs`, and the minimised container
  `tests/fixtures/declared_size_lie.zip`.

### Fixed

- **A 140 KB document could abort the process.** The reader is iterative and would build a tree of
  any depth; every walk *over* that tree recurses, because the data does — `Drop` and `Clone` are
  compiler-generated, the serializer descends a dirty element, and `mjx_mce::resolve` descends the
  whole document. `resolve` died first, overflowing the stack at a nesting depth reachable in about
  140 KB of `<a>`. Not a catchable panic: an abort. `fidelity::parse` now refuses to build a tree
  deeper than `MAXIMUM_DEPTH` (256), which bounds every walk downstream, including the ones Phase C
  and D have not written yet. The deepest part in the committed corpus is **13**.

- **A 757-byte container could ask for four gigabytes.** A ZIP entry's uncompressed size is a header
  field, attacker-controlled and checked against the data only after the data has arrived.
  `Package::open` reserved exactly that many bytes per part, so a container declaring 4 GiB for a
  four-byte payload allocated 4 GiB before it could return an error. The speculative reservation is
  now capped at 1 MiB and the buffer grows from bytes that actually arrive. **Nothing about what is
  accepted changed.**

- **`<!DoCTYPE a>` lost a byte and changed case.** The writer wraps a doctype in the constant
  `<!DOCTYPE` … `>`, so a source spelling the keyword any other way could not come back —
  sixteen bytes in, fifteen out. `quick-xml` accepts spellings XML 1.0 §2.8 does not, and the reader
  now refuses a doctype it could not reproduce rather than silently rewriting it.

- **An element name that could not be written back is now refused.** `quick-xml` scans an element
  name up to whitespace, so `<a" b"c="1"/>` produced an element literally named `a"`. Untouched it
  round-tripped; *rewritten*, the writer put that name between `<` and `>` and emitted markup that
  will not parse. Names carrying a byte that would end a name or the tag around it are refused; names
  XML would reject but that re-serialize exactly (a leading digit, say) are still preserved, because
  fidelity is the tie-breaker in both directions.

Every one of these fixes **tightens** what the readers accept. None loosens anything: trading a crash
for a corruption is the one thing this project exists to prevent.

## [0.0.76] - 2026-09-03

The SpreadsheetML vocabulary is generated (MJXOFF-145).

`sml.xsd` is the largest schema in the set — 4,439 lines, 367 complex types, 96 simple types — and
nothing in `mjx-ooxml-types` covered any of it. MJXOFF-132 (`mjx-sml`) is built on this vocabulary,
so without it an Excel crate would have invented its own cell-type and error-value enumerations.
This adds it **whole**, not as an allowlist:

- **`mjx_ooxml_types::spreadsheetml`** — all 96 simple types of `sml.xsd`, carrying all 559
  enumeration values of its named types. Cell types, formula kinds, the 18 conditional-format rule
  kinds and 17 icon sets, the 66 PivotTable filters, the 28 table-style elements, border and
  pattern fills, data-validation kinds and IME modes, the MDX cube vocabulary, and the rest.

Every item documents its original `ST_*` symbol and its exact wire token, and 149 of the values are
named from the ECMA-376 prose rather than from their token — `s` is a shared string and `str` a
formula string, `3TrafficLights1` is `ThreeTrafficLights`, `gray125` is 12.5% grey, and `stdDevp`
is the population standard deviation as against `stdDev`'s sample estimate. The `wire` suite grows
26 → 61 → **94** tests: one per overridden `ST_*` pinning its named variants to exact bytes in both
directions, plus an exhaustive pass over all 559 tokens.

### Fixed

- **The simple-type reader lost a type when one nested another.** `xtask`'s XSD reader closed a
  named `xsd:simpleType` on the first `</xsd:simpleType>` it saw, so an `xsd:union` written with
  inline anonymous members — `sml.xsd`'s `ST_TextRotation`, the only one in the emitted set — closed
  its own definition early, swallowed the type declared after it, and attributed the inner
  restrictions' base and facets to the type around them. The reader now tracks nesting depth.
- **A union of one number is now that number.** `ST_TextRotation` (0–180 degrees, or 255) would
  have been a `String` newtype. A union every member of which resolves to the same Rust primitive
  is emitted as that primitive, the way a plain numeric restriction already was.

### Changed

- `assert_every_token_round_trips!` in the `wire` suite is now
  `assert_every_token_round_trips_to_its_own_variant!`, and asserts that the number of distinct
  variants an enumeration reaches equals the number of values its schema declares — the failure two
  colliding naming-override rows would cause. It covers `wml`, `shared-math` and `sml` alike.

## [0.0.75] - 2026-09-03

The WordprocessingML and Office Math vocabularies are generated (MJXOFF-144).

`mjx-ooxml-types` covered the shared common simple types, a curated slice of `dml-main` and a
curated slice of `pml`. Word and equations had nothing, so MJXOFF-90 (`mjx-docx`) and MJXOFF-134
(`mjx-omml`) would each have invented their own enumerations and the naming convention would have
fractured across two crates at once. This adds both vocabularies **whole**, not as an allowlist:

- **`mjx_ooxml_types::wordprocessingml`** — all 110 simple types of `wml.xsd`, carrying all 733
  enumeration values. Justification, underline kinds, the 193 border styles, shading patterns,
  section breaks, the 63 numbering formats, theme colours, text-flow direction, table-style
  overrides, the glossary-document galleries, and the rest.
- **`mjx_ooxml_types::officemath`** — all 14 simple types of `shared-math.xsd`, carrying all 30
  enumeration values.

Every item documents its original `ST_*` symbol and its exact wire token, and 183 of the values are
named from the ECMA-376 prose rather than from their token — `pct12` is 12.5%, `neCell` is the top
**right** table cell, `ideographZodiac` is the zodiac ideograph format, and `--`/`-+`/`+-` would
otherwise have collapsed onto one identifier. The `wire` suite round-trips all 763 tokens and pins
every one of those 183 names to its exact bytes in both directions.

### The naming tables are now per schema

An `ST_*` symbol is scoped to the schema that declares it, and OOXML reuses symbols: `ST_Jc` is
declared by both `wml.xsd` and `shared-math.xsd`, and `ST_Direction` by both `wml.xsd` (`ltr`/`rtl`)
and `pml.xsd` (`horz`/`vert`, already emitted as `Orientation`). One flat override table keyed on the
bare symbol cannot hold two meanings. `xtask`'s naming data is therefore partitioned the way the
symbols are — one `NameEngine` per emitted module — and the engine in `naming.rs` is unchanged: it
already took its tables by reference. Adding a schema still means growing the tables. The existing
`shared`, `drawingml` and `presentationml` output is byte-identical.

### The generator refuses names that would lose a token

Two `ST_*` types that reach one Rust type name, or two values of one enumeration that reach one Rust
variant, are now hard errors in `xtask` rather than Rust that compiles with a wire token nobody can
write back. So is a naming-override row that matched nothing — a misspelled symbol used to do
nothing at all, silently leaving the mechanical name it was written to replace.

### `COVERAGE.md` reports every schema

The generated manifest listed six schemas of the twenty-six in the Transitional set, and printed
`pending` for `wml`, `sml` and `shared-math` from a hard-coded string — so it would have kept saying
`pending` after the work was done. It now has a row for **every** schema in **both** tables, with
each status derived: the simple-type column from the generator's module table, the child-order column
from `CHILD_ORDER_SCHEMAS`. A pending row names the work item that owns it, and
`mjx-schema-gate`'s `the_declared_owners_agree_with_the_generated_coverage_document` fails if the
document and the gate's `OrderingCoverage::Pending` name different owners. A schema that is in
neither table and has no written reason fails the generator.

No `CHILD_ORDER_SCHEMAS` row was added: those belong to the children that start authoring the markup
(MJXOFF-90 for `wml`, MJXOFF-134 for `shared-math`, MJXOFF-132 for `sml`).

## [0.0.74] - 2026-09-03

A typed model's round trip no longer re-flows the part it came from (MJXOFF-143).

0.0.64 gave every parsed element the byte range it came from, so a serializer copies untouched
subtrees rather than rebuilding them. It stopped at the typed layer, and that left one limitation in
`fidelity_and_gaps.md`: **a model is a view**, so a `from_xml` / `to_xml` pass rebuilds every element
it looked at — including the ones nothing changed — and `*slot = value.to_xml(interner)` throws the
range of each of them away. Three surfaces read a whole part that way (`edit_vml_drawing`,
`edit_chart`, and the table-style list), and a dozen more read a single element that way. Editing one
word of a chart title re-flowed the whole chart.

Both halves of the loss were deliberate design rather than oversight, which is why this needed a
decision rather than a patch. `RawElement`'s `Clone` drops the range because a range means nothing
against another document's buffer, and cloning is how a subtree leaves the document that owns one.
`RawElement::new` records none because a newly authored element has no original.

### The design

**`RawElement::replace_preserving_verbatim_source`**, and `ToXml::write_back` over it. Instead of
assigning the rebuild over the original, the two are walked together in one pass, and a range is
moved onto a rebuilt node **only where that node compares equal to the one it replaces**. Two facts
discharge the burden of proof, and both are structural rather than remembered:

- *The bytes still describe the element.* `RawElement`'s `PartialEq` compares name, self-closing
  style, attributes in order with their quoting, and, recursively, children — precisely the
  properties an element's markup determines. So "equal" **is** "these bytes spell this element".
- *The buffer is the right one.* The range comes from the element being overwritten and lands on its
  replacement at that same position, so the destination document is by construction the one that
  measured it. A caller cannot pair an original from one document with a rebuild bound for another,
  because the original *is* the destination.

The three candidates it was chosen over: a `clone_within_document` on `RawElement` would give ranges
back only to the markup a model does *not* understand — everything it does model is rebuilt after the
clone, so a wrapped `v:shape` would still re-flow — and its soundness ("only while the clone stays in
this document") is a convention no type can check. `FromXml` taking its element by value moves the
content instead of cloning it, but breaks every implementor and every call site while giving the same
partial answer, and the read-only surfaces (`with_chart`, `with_vml_drawing`) hold a shared reference
and could not give ownership at all. A retained element plus a dirty flag reaches everything, but the
flag must be cleared by every mutator in three crates, and one missed mutator writes the wrong bytes
— the single failure mode this design exists to make impossible. Here there is no flag to forget:
cleanliness is *computed*, against the element still sitting in the document.

`mjx-xml`'s writer is unchanged and still checks every range before trusting it — it must fit, open
with `<` plus the element's qualified name, and close the way `empty` says it closes — so a range
that reached it wrongly degrades to a re-flow rather than to wrong bytes. `RawElement` does not grow:
the eight-byte budget test is untouched.

### What changed for callers

- `ToXml` gains a **provided** method, `write_back`, so no implementor changes. Every whole-part and
  sub-element edit surface in `mjx-pptx` now goes through it — 19 call sites.
- `mjx_vml::DrawingPart` keeps the `RawDocument` it parsed instead of scattering its pieces, because
  a standalone part has no document to write back into otherwise. It costs the parsed tree alongside
  the typed model; a caller that already owns the part's `RawDocument` (through
  `mjx_opc::Package::part_tree_mut`) should use `Drawing::from_xml` and `write_back` directly and pay
  nothing.
- The *Limitations* table in `crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` is gone — it had one
  row and this was it. The row is recorded under "What used to be here", so a reader can tell "gone"
  from "quietly dropped".

`crates/mjx-vml/tests/drawing.rs`'s `attributes_wrapped_across_lines_reflow_when_the_part_is_re_serialized`
asserted the re-flow *happened* and instructed its reader to replace it with byte identity the moment
it stopped. It has stopped. Every new case is written against a fixture whose start tags are wrapped
across lines — `vmlDrawing1.vml`'s with CRLF — because a part that is already on one line
reconstructs to its own bytes and would pass with the mechanism deleted.

Tests 1,676 → 1,690 default and 1,690 → 1,705 with `--all-features`.

## [0.0.73] - 2026-09-03

`mjx-dml`'s composite tiers on the attribute grammar — geometry, tables, text and the colour
resolver (MJXOFF-142).

0.0.72 put the seven shared property tiers on the grammar. This completes the crate: **the 107
remaining `attr_*` call sites across `geometry/`, `table/`, `text/` and `resolve.rs` became 0**, and
with them every `dml_attr`, `prefixed_attr`, `push_*`, `set_attr`, `angle_to_wire`,
`parse_percentage` and `parse_angle`. `crates/mjx-dml/src/build.rs` no longer mentions attributes at
all: what is left there builds and finds *elements*.

**There is one path from a wire attribute to a typed value in `mjx-dml`, and one back**, and both go
through `mjx_xml::attribute::{read, write}`. A helper family with two callers left is the
half-migrated family CI's naming check exists to warn about, so the family is gone rather than
reduced.

### Two shapes, one grammar

The tiers here have the same split 0.0.72 found: some types retain an attribute vector and declare
on themselves; most sites are *value projections* over elements the crate has no type for — an
`a:pt`, an `a:arcTo`, an `a:tab`, an `a:buChar`, an `a:hlinkClick`, a colour transform's `@val` —
and declare on a generic attribute face reached through `AsRef<[RawAttribute]>` / `AsMut<Vec<..>>`.
Twenty-two such faces are declared here.

Two attributes are declared as `Text` rather than as an `Enumeration<T>` *deliberately*:
`a:prstGeom@prst` and `a:cell3D@prstMaterial` each expose **both** readings of the same bytes — the
typed one and the raw token — which is what lets a shape kind or a material this build does not know
still be named. The typed reading layers the generated enumeration's own `from_wire` over the one
read, so the token → enum mapping still has one implementation.

### New

- **`mjx_dml::codec`** gains five: `TextFontSize` (`ST_TextFontSize`), `TextPointSize`
  (`ST_TextPoint`), `TextIndentLevel` (`ST_TextIndentLevelType`, whose `0..=8` range is enforced),
  `PercentageWithPercentSign` (the `111%` spelling `a:buSzPct@val` is written in), and
  `EmuOrGuideName` / `AngleOrGuideName` for the two `ST_Adj*` unions custom geometry places points
  with.
- **`crates/mjx-dml/tests/in_context_roundtrip.rs`** grows from 16 cases to 28: a table lifted out
  of `tables.pptx` and `table_extensions.pptx` and asserted **at the outermost container**, so a
  cell's attributes must survive being rebuilt as part of a row rebuilt as part of a table; a
  paragraph-level body and a preset geometry out of `text_levels.pptx`; a transform read and written
  back; and five hand-written literals in forms this project's writer never emits, for the run
  properties, paragraph properties, table, custom geometry and transform tiers.

### Changed behaviour: a boolean a setter writes is spelled `true`

`TableProperties::set_part` and `TableCell::set_merged` wrote `1`; they now write `true`, the one
canonical `ST_OnOff` spelling every other boolean in the workspace is written in, because they go
through the same `OnOff` codec. Reading is unchanged and still accepts all six spellings, and an
attribute **nobody assigns to keeps its own spelling** — a file that says `firstRow="1"` still says
`firstRow="1"` after an unrelated edit. (`mjx_pptx`'s `add_table` builds a fresh `a:tblPr` from a
literal template and still writes `firstRow="1" bandRow="1"`, as PowerPoint does.)

### Breaking changes

As in 0.0.72: an accessor over a declared attribute reports a malformed value instead of silently
reading `None`, so it returns `Result<Option<T>, AttributeError>` — or `Result<T, AttributeError>`
where the attribute is `use="required"` — and a text-valued one returns a `Cow<str>` (entity
references in the file are decoded) where it returned `&str`.

| Was | Is |
|-----|----|
| `AdjustPoint::{x, y}` → `Option<AdjustCoordinate>` | `Result<AdjustCoordinate, AttributeError>` |
| `Path2D::{width, height, fill, stroke, extrusion_ok}` → `Option<T>` | `Result<Option<T>, _>` |
| `GeometryGuide::{name, formula}` → `Option<&str>` | `Result<Cow<str>, _>` |
| `PresetGeometry::preset_token` → `Option<&str>` | `Result<Cow<str>, _>` |
| `TableColumn::width`, `TableRow::height` → `Option<Emu>` | `Result<Option<Emu>, _>` |
| `TableColumn::set_width`, `TableRow::set_height` took `Emu` | take `Option<Emu>` (`None` removes) |
| `TableCellProperties`'s eight accessors → `Option<T>` | `Result<Option<T>, _>` |
| `TableCellProperties::{set_anchor, set_text_direction, set_horizontal_overflow}` took a value | take an `Option` |
| `TableCell::id`, `TextField::{id, field_type}` → `Option<&str>` | `Result<Option<Cow<str>>, _>` |
| `TableStyleList::default_style_id`, `TableStyle::{style_id, style_name}` → `Option<&str>` | `Result<Cow<str>, _>` |
| `FontReference::index` → `Option<FontCollectionIndex>` | `Result<Option<..>, _>` |
| `Cell3D::preset_material` → `Option<&str>` | `Result<Option<Cow<str>>, _>` |
| `CharacterProperties`'s ten attribute accessors → `Option<T>` | `Result<Option<T>, _>` |
| `CharacterProperties::{hyperlink_rel_id, hyperlink_action}`, `TextRun::hyperlink_rel_id` → `Option<&str>` | `Option<String>` |
| `ParagraphProperties`'s eight attribute accessors → `Option<T>` | `Result<Option<T>, _>` |
| `ResolvedGuides::define` took `&'a str` | takes `impl Into<Cow<'a, str>>` |

`Transform2D::read`, `CustomGeometry`'s spec readers, `TableCell::{column_span, row_span,
merged_horizontally, merged_vertically}`, `TableProperties::part`, `TableStyleTextStyle::{bold,
italic}`, `PresetGeometry::preset`, `Cell3D::material` and every `resolve_*` function keep their
shape: each is a **total** projection with a documented answer for "the file does not say", and a
value this model cannot read is the file not saying. That decision is written down in
`resolve.rs`'s module docs, where it is load-bearing — a renderer that refused to draw a shape over
one malformed colour transform would be worse than one that drew it without the transform.

## [0.0.72] - 2026-09-03

`mjx-dml`'s shared property tiers on the attribute grammar — colour, fill, outline, effects, 3-D,
theme and style (MJXOFF-141).

MJXOFF-140 proved the `#[xml(attribute(..))]` grammar on a synthetic type inside `mjx-derive`'s own
tests. This release is the first time anything shipped uses it: the seven files every other
DrawingML tier reaches through no longer parse an attribute by hand. **86 calls to the `attr_*`
family became 0 in those files**, and the four hand-written measure readers and writers they were
the last users of are deleted.

### There is now exactly one path from a wire attribute to a typed value

`mjx_xml::attribute::read` and `mjx_xml::attribute::write` are that path — find, decode, hand to a
codec; encode, escape for the quote in use, set or remove. Every accessor
`#[derive(XmlAttributes)]` generates is one call to one of them, `mjx-dml`'s remaining `attr_*` /
`push_*` helpers (which the tiers MJXOFF-142 owns still use) are one call to one of them, and a model
reading an element it has no type for calls them directly. Two implementations of "attribute to
value" is the duplicate this workstream exists to prevent; there is one.

### A declaration no longer requires a type that owns its attributes

`#[derive(XmlAttributes)]` reaches the vector through `AsRef<[RawAttribute]>` to read and
`AsMut<Vec<RawAttribute>>` to write, so the `attributes` field may be a `Vec`, a `&[RawAttribute]`
view (getters only — the bound that would give it setters is simply not satisfied), a
`&mut Vec<RawAttribute>` cursor, or generic over all of them.

That last form is what `mjx-dml`'s **value projections** use. An effect, a bevel, a camera, a
gradient stop, a line end and a blip are facts read out of an element the crate does not model as a
type; a conduit generic over its attribute container declares them once and serves both directions —
`{ attributes: &element.attributes }` to read, which copies nothing, and `{ attributes: Vec::new() }`
to write the vector the new element will own.

### New

- **`mjx_dml::codec`** — `EmuCoordinate`, `EmuLineWidth`, `SixtyThousandthsOfADegree` and
  `Percentage`, the four measure codecs. A crate that owns a measure owns its codec.
- **`mjx_ooxml_core::Number<T>`** — an alias for `Enumeration<T>`, so a numeric attribute is declared
  `codec = Number<u32>` rather than claiming an integer is an enumeration.
- **`RawElement::rebuilt`** — the single construction point every `ToXml` now goes through. Identical
  to `new` today; it exists so that carrying a source range through a typed round trip (MJXOFF-143)
  is one edit rather than one per `to_xml` in the workspace.
- **`crates/mjx-dml/tests/in_context_roundtrip.rs`** — the generalised in-context harness (was
  `txbody_roundtrip.rs`), now covering seven types out of real parts plus a corpus of hand-written
  literals in forms this project's writer never emits, and both tier-3 isolation cases.

### Breaking changes

An accessor over a declared attribute reports a malformed value instead of silently reading `None`,
so several `mjx-dml` accessors return `Result<Option<T>, AttributeError>` where they returned
`Option<T>`, and the text-valued ones return a `Cow<str>` (entity references in the file are decoded)
where they returned `&str`. The affected methods are `Color::{value, hex}`,
`LineProperties::{width, cap, compound, pen_alignment}`, `GradientFill::{flip, rot_with_shape}`,
`PatternFill::preset` and `Shape3D::{z, extrusion_height, contour_width, material}`.
`PictureFill::{image_rel_id, image_link_id}` return `Option<String>`: they read through the blip's
attribute face, which does not outlive the call, and both callers copied the id anyway. The value
tiers (`LineSpec`, `Shape3DSpec`, every effect) are unchanged — a spec is a value description and
still drops what it cannot represent.

## [0.0.71] - 2026-09-03

An attribute grammar for `mjx-derive` — accessors over the retained attribute vector, not a lifting
form (MJXOFF-140).

`mjx-derive` modeled elements, children and text; **attributes it did not model at all.** Every typed
type in the workspace reached into its own `Vec<RawAttribute>` and parsed the value by hand.
DrawingML survived that because Phase A grew it a tier at a time, but `wml.xsd` has 110 simple types
and `sml.xsd` 96, both far more attribute-dense than `pml`, and hand-parsing each one across two new
format crates is where the `ST_OnOff` spellings would quietly go wrong.

Nothing shipped changes. No emitted byte moves; this release adds a way to declare what a hand-written
accessor already does, and the additions are new items beside the existing ones.

### `#[derive(XmlAttributes)]`

A third derive, independent of `FromXml` / `ToXml` and composing with them, with a hand-written pair
of impls, or with neither. It asks only for the retained `attributes: Vec<RawAttribute>` field and
generates **one getter and one setter per declared attribute** over that vector:

```rust
#[derive(FromXml, ToXml, XmlAttributes)]
#[xml(attribute(local = "val", codec = HexColorRgb, accessor = color, required))]
#[xml(attribute(local = "rtlCol", codec = OnOff, default = false))]
#[xml(attribute(local = "cap", codec = Enumeration<LineCap>, accessor = line_cap))]
#[xml(attribute(local = "embed", prefix = "r", codec = Text, accessor = image_relationship))]
struct SolidColor { /* .. */ }
```

`local` and `codec` are required; `prefix` matches and writes a prefixed attribute; `accessor` names
the Rust method (the default is the wire name in snake case, which the naming convention will usually
want overriding); `required` makes an absent attribute a typed error and `default` gives it a schema
default. Writing neither makes it optional — the third case, whose getter returns `Option`.

**The accessor form is the point.** A grammar that lifted attributes into struct fields would make
the writer *reconstruct* the attribute list, and reconstruction is how unknown attributes, their
order, their prefixes and their quote characters get lost. Nothing in the generated code builds an
attribute list: a getter borrows the vector, a setter reaches exactly one element of it.

### Read never normalizes; a write does

A getter takes `&self`, so it cannot change the file: `rtlCol='on'` that nobody assigned to still
writes `on`, single-quoted, in the position it was read from, and `val='50%'` stays `50%`. The one
canonical form is written only by a setter — `set_rtl_col(true)` writes `true` — which rewrites the
attribute **in place**, keeping its position and the quote character the file used, and escaping the
new value for *that* quote. An attribute that was not there is appended, double-quoted.

A grammar that canonicalized on read would rewrite every file it opened, and would do it invisibly,
because our reader and our writer would agree with each other.

### The codecs

`mjx_ooxml_core::AttributeCodec` is the wire ⇄ Rust conversion for one *kind* of value — a type-level
tag, never constructed. `mjx-ooxml-core` ships the XML-generic ones (`Text`, `Enumeration<T>`, which
covers every generated `ST_*` enumeration because they all spell themselves with `FromStr` +
`Display`); `mjx-ooxml-types` ships the OOXML-specific ones (`OnOff`, `TrueFalse`, `TrueFalseBlank`,
`HexColorRgb`), consuming the `support` normalizers rather than re-deriving them. A crate that owns a
measure type owns its codec, in about fifteen lines — which is how `mjx-dml` will carry `Emu` and
`Fraction` across the seam.

A malformed value is `AttributeError`, never a panic: these are attacker-controlled files.

### Also

`mjx_xml::attribute` — `find`, `decoded_value`, `set`, `remove`: the four in-place operations a typed
accessor is made of, usable by hand. `mjx_xml::text::escape_attribute_in` escapes for a given quote
character (`'` → `&apos;`), which is what lets a setter keep a single-quoted attribute single-quoted
without being able to emit `attr='it's'`. `FromXmlError` gains an `Attribute` variant.

## [0.0.70] - 2026-09-03

The gates reach Word and Excel — before a line of `wml` or `sml` model code exists (MJXOFF-110).

`sample.docx` and `sample.xlsx` have been in `tests/fixtures/` since the first phase and **nothing
had ever schema-validated either of them.** A `w:` part with no arm in the schema table was reported
"skipped, foreign namespace"; the suite counted the remaining parts, found four of them valid, and
reported green. The sentence "the schema gate covers Word" was true and empty at the same time. So
were the ordering half (`assert_deck_is_in_schema_order` asserted only that *some* part had been
audited, and `word/theme/theme1.xml` satisfied it) and the byte-identity half (three suites carried
hand-maintained fixture lists that between them omitted six of the fifteen committed fixtures).

Nothing shipped changes. Every byte this library writes is identical before and after; this release
is test and CI infrastructure, and the round-trip suites did not move.

### The harness is a crate

`crates/mjx-schema-gate` is a new **test-only** crate (`publish = false`, a `dev-dependency` of
`mjx-pptx`, `mjx-docx` and `mjx-xlsx` and of nothing else). An integration test compiles only into
its own crate, so the harness that lived in `mjx-pptx/tests/schema_validity.rs` could never be
reached from the two crates Phases C and D will fill. `crates/mjx-fixtures` is a second, entirely
dependency-free test-only crate holding the committed corpus, so `mjx-opc`'s byte-identity suites —
which sit *below* the gate in the layering — can read the same corpus without an upward edge.

### The three-category rule, with no fourth branch

`mjx_schema_gate::categories` is the only place the line is drawn. Markup we model is **validated**
against its XSD; foreign markup we only preserve (VML, InkML, ActiveX, and the two `docProps`
streams) is **skipped with a written reason**; a root element in a namespace on neither list is a
**hard failure naming the namespace and the part**. There is no "skip anything we have no arm for"
fallback, because that fallback is the hole being closed.

`WordprocessingML` joins the table, so `sample.docx`'s `word/document.xml`, `word/styles.xml`,
`word/fontTable.xml` and `word/settings.xml` are validated against `wml.xsd` for the first time.

### `wml.xsd` can now be compiled at all

`wml.xsd:21` and `shared-math.xsd:13` import `http://www.w3.org/XML/1998/namespace` with no
`schemaLocation`, and the Transitional set ships no `xml.xsd`, so libxml2 could not resolve
`xml:space` and both schemas failed to *compile*. A bare import gives libxml2 no URI, so a catalog
has nothing to rewrite. `crates/mjx-schema-gate/schemas/xml.xsd` — hand-written for this repository,
no third-party licence, nothing fetched at build time — is paired with each XSD through a generated
driver schema. Every validation goes through one, so `shared-math.xsd` inherits the fix.

### Markup compatibility is resolved, not skipped

A part carrying `mc:AlternateContent` or `mc:Ignorable` used to be skipped, which is why LibreOffice's
`word/document.xml` could never be reached. The gate now resolves it with the existing `mjx-mce`
crate — the winning `mc:Choice` selected, ignorable markup in namespaces ECMA-376 does not define
dropped — and validates that view. Only parts that actually carry markup compatibility are
re-serialized; every other part is validated as the exact bytes the package holds.

### Two pre-existing divergences in `sample.xlsx`, now recorded

LibreOffice writes `xml:space="preserve"` on every `s:t` (which `sml.xsd` types as a simple type that
can carry no attribute) and `dateCompatibility` on `s:workbookPr` (not in the 5th-edition
Transitional schema). Both are inputs this project preserves verbatim, so both are recorded as
tolerated deviations with their reasons, matched error-by-error: a *new* defect in either part still
fails.

### The corpus is the directory

`crates/mjx-opc/tests/{roundtrip,tree_roundtrip,package_validation}.rs` and the schema gate all read
`tests/fixtures/` instead of a list. All fifteen fixtures are now inside all four contracts; a file
whose extension is on no list fails, naming it.

### CI

A new required `test (--all-features)` job runs `cargo clippy --workspace --all-targets
--all-features` and `cargo test --workspace --all-features --no-fail-fast`; the existing test steps
gain `--no-fail-fast`; the `schema-validity` job runs the Word and Excel gates beside the PowerPoint
one. `.github/scripts/merge-when-checks-pass.sh` makes the merge step a command whose exit status
gates the merge rather than a sentence instructing a person to look at one.

## [0.0.69] - 2026-09-03

The chart `delete_*` family is `suppress_*` — the naming question v0.0.66's API review raised and
left open, settled before Word and Excel copy the shape (MJXOFF-89).

Twelve public identifiers change across `mjx-chart`, `mjx-pptx` and `mjx-ooxml`, and three of them
are re-projected by each binding. Three `is_deleted` accessors (`DataLabel`, `DataLabels`, `Axis`)
and two `deleted` fields (`DataLabelSettings`, `ChartAxisData`) become `is_suppressed` /
`suppressed`; `delete_chart_data_labels` (on both `Presentation` and `Deck`),
`delete_plot_data_labels`, `delete_data_labels`, `delete_point_label` and `delete_label_for_point`
take a `suppress_` prefix; and `auto_title_deleted` becomes `auto_title_suppressed`. The crate-private
`DataLabels::delete_all` and the two private `clear_delete` helpers move with them. Python sees the
same names (the binding is the identity mapping); TypeScript sees `suppressChartDataLabels` and a
`suppressed` getter in place of `deleteChartDataLabels` and `deleted`.

Nothing else changes. This is a rename: the bytes written for any file are identical before and
after, and no test was added, removed or skipped.

The spelling is now enforced rather than remembered. `.github/scripts/check-suppress-naming.sh` — a
new `naming` job in CI, plus a step in `wasm-pack` over the generated `.d.ts` — fails the build if
any identifier under `crates/*/src`, `bindings/*/src`, the committed `.pyi` or the generated
TypeScript declarations spells this concept `delete`. The wire token is untouched and explicitly
permitted: `flag("delete")`, `"autoTitleDeleted"`, `c:delete` in prose, and the generated ordering
tables in `mjx-ooxml-types` all pass, and each item's docs still name the exact element it writes.

## [0.0.68] - 2026-09-02

Python (PyO3) and WebAssembly/TypeScript (wasm-bindgen) bindings — the facade, projected whole
(MJX-210).

`mjx-ooxml` has been "the binding-ready public API" since v0.0.67. Nothing was bound to it. Two
workspace members now are, and both project the **whole** surface rather than a sample of it:

- **`bindings/mjx-python`** — PyO3, module `mjx_ooxml`, abi3-py39 wheels. 253 methods on `Deck`
  (257 with the `vml` feature), 192 classes, a committed `.pyi` plus `py.typed`, and an exception
  hierarchy of eleven classes rooted at `OoxmlError` — each carrying `.code` and the coordinates
  `.surface`, `.shape`, `.row`, `.column`, `.index`, with `IndexOutOfRangeError` also an
  `IndexError`. The mapping is the **identity**: nothing is renamed except the `None` member of nine
  enumerations, which Python's grammar will not permit.
- **`bindings/mjx-wasm`** — wasm-bindgen, one npm package with conditional exports for a bundler
  build and a browser build. The same surface in **camelCase**, `Uint8Array` in and out, and
  failures as real `Error` objects with `name === "OoxmlError"`, a stable `code` and a `detail`
  object. `deck.free()` is mandatory and the documentation says so in every place a reader might
  look.

### The acceptance test

`crates/mjx-ooxml/examples/build_a_deck.rs` — the guide's whole walkthrough — now exists three
times: once in Rust, once as `test_build_a_deck.py`, once as `build_a_deck.mjs`. Each of the two
bindings runs the Rust one and compares its own deck **part by part, byte for byte**. That is what
proves the curated subset is sufficient, and it is what would catch a method wired to the wrong
`Deck` method: nothing about the types would complain, and one part payload would differ.

### Added to `mjx-ooxml`

Ten types were reachable from the re-exported vocabulary but not themselves re-exported, so a
binding could hold a value it could not name: `AdjustHandle`, `ConnectionSite`, `ColorKind`,
`FontSlot`, `TableStyleBorder`, `ThemeFontReference`, `GuideFormulaError`, `AxisKind`, and
`AdjustmentSpec` with `AdjustmentAxis` / `AdjustmentBound`. `tests/vocabulary_closure.rs` uses all
ten through the facade, so the list stays closed.

`PartialEq` was added to `mjx_pptx::TableStyleFormat`, `mjx_pptx::TableStyleDefinition` and
`mjx_chart::ChartData`, whose siblings all had it; and the two terse "Delegates to …" doc summaries
on `Deck::set_cell_run_properties` / `set_cell_text_range_properties` were written out, because the
bindings use those summaries as their docstrings.

### The recorded divergence

`PLAN.md`, `README.md` and `CLAUDE.md` said bindings would live in a **separate cargo project** on a
**UniFFI → wasm → C-ABI** stack targeting Kotlin, Swift, JavaScript and C, deferred to Phase 7. They
do not, and it is not. All three files now say what was built and why — see the "Recorded
divergence" section of `PLAN.md`.

### `unsafe`

The two binding crates are the first in this workspace to carry `#![allow(unsafe_code)]`, and the
first use of the `deny`-not-`forbid` escape hatch the workspace lints were written to permit. The
justification is that no `unsafe` is hand-written: every unsafe block is generated by `#[pyclass]`
or `#[wasm_bindgen]`. CI greps `bindings/*/src` and `bindings/*/tests` for `unsafe` outside a
comment and fails if it finds any, so the justification cannot quietly become false.

### Measured, not gated

The WebAssembly payload is **2,484,641 bytes raw and 848,380 gzipped (828 KiB)** with `lto = true`,
`codegen-units = 1`, `strip = "debuginfo"` and `wasm-opt -Oz`. The specification estimated
400–700 KB and asked for a measurement before a budget; this is the measurement, and CI reports it
on every run rather than failing on a number nobody has justified yet. `panic = "unwind"` is kept
workspace-wide because PyO3 needs it to turn a panic into a Python exception rather than a process
abort.

### CI

Three new jobs — `bindings-build` (the `unsafe` check, both crates built the way they ship),
`wasm-pack` (headless Chrome, Node, both npm targets, the size report) and `python-wheel` (abi3
wheels on Linux, macOS and Windows, installed from the wheel, then `pytest` and `mypy --strict`, and
the same wheel re-checked on a much later interpreter). The cross-build matrix excludes both binding
members: a PyO3 `cdylib` needs a host interpreter and a wasm `cdylib` means nothing off `wasm32`.
The `examples` job now runs every crate's examples, not only `mjx-pptx`'s.

## [0.0.67] - 2026-09-02

The `mjx-ooxml` facade — `detect_format`, `Deck`, FFI-shaped errors, the curated surface (MJX-210).

`mjx-ooxml` had been **62 lines of documentation and no code** since the workspace was laid out,
while the docs called it "the binding-ready public API". It is now that API.

**`detect_format` reads the package, not the filename.** It opens the OPC container, follows the root
`officeDocument` relationship and maps the main part's content type against the fifteen ECMA-376 and
macro-enabled types. That is the only way `.pptm` and `.potx` — the same PresentationML markup under a
different declaration — can be told from `.pptx`, and the only answer that survives a renamed file.
Word and Excel are recognized and refused by name (`ErrorCode::UnsupportedFormat`), so a caller who
hands a `.docx` to a PowerPoint library is told it is a Word document rather than that some part
failed to parse. Detection working before editing does is the whole point.

**`Deck` restates 251 of `Presentation`'s 273 methods in types a foreign function boundary can
express**: `impl Into<Surface>` becomes a concrete `Surface`, `impl Into<ShapePath>` a concrete
`ShapePath`, `usize` becomes `u32` on every parameter and return, `&PartName` becomes `&str`, and a
borrowed `Option<&[u8]>` becomes an owned `Option<Vec<u8>>`. The facade owns its own `Surface` and
`ShapePath` — carrying `u32`, converting at the boundary, allocation-free for the top-level case —
because adding `From<u32>` beside `From<usize>` on `mjx-pptx`'s would have made every bare integer
literal in `deck.shape_fill(0, 2)` ambiguous across the workspace.

Sixteen methods are deliberately absent, each unreachable across FFI or reachable another way:
`Presentation::shape` (returns a cursor borrowing the deck), the five closure-taking table-style and
VML readers, the four surface `*_part` accessors and the six `*_rel_id` accessors (part-graph
identity for content that is already reachable by index or by bytes). `Deck::presentation_mut` is the
Rust-only door to all of them; there is no `Deck::package`, because handing out `&mut Package` would
give a caller the whole part graph and make every invariant `save` enforces unenforceable.

**One exclusion the specification proposed was checked and reversed.** The per-cell formatting
setters were to be dropped as reachable through `format_cells(Cells, &CellFormat)`. They are not:
`format_cells` deliberately skips a cell covered by a merge, so only what renders is touched, while
`set_cell_fill` reaches a covered cell — whose own formatting reappears when the region is unmerged.
Dropping them would have dropped that, so all fifteen are exposed, and a test asserts the two
spellings really are different calls.

**One `Error`, eleven stable codes.** `Error { code, message, detail, source }` collapses all 65
`PptxError` variants and all 9 `OpcError` variants into `Io`, `MalformedDocument`, `InvalidDocument`,
`IndexOutOfRange`, `WrongKind`, `NotFound`, `NothingToRead`, `InvalidArgument`, `StructureConflict`,
`UnsupportedContent` and `UnsupportedFormat`, plus the human message and the `surface` / `shape` /
`row` / `column` / `index` coordinates a binding turns into exception attributes. Rust callers lose
nothing: `source()` downcasts back to the `PptxError`. The classification is an exhaustive `match`
with no wildcard arm — which is why `PptxError` stopped being `#[non_exhaustive]` — so a new variant
fails the build until it is classified.

**`Deck::save` inherits the validation `Presentation::save` performs** rather than routing around it.
A facade that widened what a caller could break would be a regression, so this is tested on a deck
that is genuinely invalid: `save` refuses it and `save_unchecked` writes it.

Also here: `Presentation::{remove_unused_parts, external_links, retarget_external_link}` — package
hygiene as three thin delegates rather than an exposed `package()` — and `SlideSize::{widescreen,
standard, from_emu}`, so a caller building a deck from nothing states a size by name instead of by
struct literal.

`examples/build_a_deck.rs` is the guide's walkthrough written through the facade, **naming no crate
below `mjx-ooxml`**; if the re-export list were insufficient it would not compile.

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
