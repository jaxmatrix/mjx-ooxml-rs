# Fidelity, and the known gaps

Read this before you rely on the library in production. The first half is the guarantee; the second
half is an honest list of what this library does not do.

It is deliberately the same page, in the same order, as
[`mjx_pptx`'s](https://docs.rs/mjx-pptx/latest/mjx_pptx/guide/fidelity_and_gaps/) — a reader who has
read one should recognise the other, and a difference between the two should mean a real difference
between the formats rather than a difference in who wrote the page.

## The guarantee

**Open any `.docx`, change one word, write it back, and every part you did not touch is byte-for-byte
what it was — and inside the part you did touch, every subtree you did not touch is byte-for-byte
what it was too.**

Not re-serialised from a model that happens to agree — the original decompressed bytes. That holds
for parts this library has never heard of, for vendor extensions (`w14:`, `w15:` and whatever comes
after them), for markup from a future version of Word.

The five mechanisms are the packaging layer's, not this crate's, which is why the guarantee is the
same one PowerPoint gets:

- **Part-level laziness with copy-on-write.** A part is raw bytes until something needs it parsed.
  Until it is edited it re-emits verbatim. This is also why every reader takes `&mut self` — reading
  may materialise a tree, though it never marks a part modified.
  `examples/read_document.rs` proves exactly that: it reads `sample.docx` exhaustively, saves it, and
  compares every part's decompressed payload against the original.
- **Subtree-level copy-on-write.** Once a part *is* edited, the same rule applies one level down.
  Every element remembers the byte range it was parsed from, and the serializer copies that range
  rather than rebuilding the element — so editing one run's text rewrites that run and the elements
  above it, and copies everything else. That is what preserves the properties a decomposed tree does
  not record at all: the whitespace *between* attributes, the spelling of a character reference
  (`&#38;` stays `&#38;`), the placement of comments and processing instructions.
  `crates/mjx-docx/src/document/mod.rs`'s own
  `editing_one_run_retains_the_untouched_sibling_paragraphs_source_span` proves it on bytes, and
  `tests/equations.rs` proves it five levels deep inside an equation.
- **The unknown bucket.** Every modelled type carries the children it does not understand, and
  preserves unknown attributes, attribute order and namespace prefixes. `word/settings.xml` is the
  extreme case: all 98 of `CT_Settings`' children are modelled, and an element in a namespace this
  crate has never met falls into `SettingsContent::Raw` **in its original position relative to its
  known neighbours** — never dropped, never reordered.
- **Markup Compatibility.** `mc:AlternateContent`, `mc:Ignorable` and `mc:ProcessContent` are
  preserved on write and resolved non-destructively on read. `sample.docx`'s own root carries eleven
  namespace declarations and `mc:Ignorable="w14 wp14 w15"`; `tests/roundtrip.rs` forces the typed
  model to run and then checks they all came back.
- **The round-trip contract.** *Per-part decompressed-payload byte identity, plus structural container
  identity* — the same part set, content types and relationships. The container's ZIP bytes are **not**
  reproduced identically, because deflate parameters vary by encoder. If you need to compare two
  documents, compare parts, not archives.

## What the library refuses to write

The guarantee above is about what this library *does not touch*. Its other half is about what it
does: **a document that would make Word say it "found unreadable content" is not written at all.**

[`save`](Document::save) validates before it writes a byte, and the check is not opt-in — a check you
have to remember is a check that ships the fault it was meant to catch. The layer is the package
graph (`mjx-opc`): every part has a content type; every internal relationship resolves to a part the
package holds; relationship ids are unique within their `.rels`; and markup never names a
relationship its own `.rels` does not declare. Each is refused as a typed
[`PackageDefect`](mjx_opc::PackageDefect) naming the part, the relationship and the identifier. None
of these is visible to a schema check: each is a property of the *graph*, and every part can be
perfectly valid against its XSD while the package is broken.

The check is scoped to **the markup this library will write** — a part still holding the bytes it was
opened with is re-emitted verbatim and is never faulted, so a file that arrives broken can still be
written back, and *reading* a document never changes whether it saves. The moment an edit makes those
bytes ours, the same defect is refused.

[`save_unchecked`](Document::save_unchecked) is the escape hatch, for when writing an inconsistent
document is the point:

```
# fn main() -> Result<(), mjx_docx::DocxError> {
use mjx_docx::{Document, PageSize};

let mut document = Document::blank(PageSize::a4())?;
document.insert_run(0, 0, "Q3 results")?;

// The default: refuses to write a document that would need repair.
let bytes = document.save()?;
assert!(!bytes.is_empty());

// Or ask for the invariants without writing anything.
document.validate()?;

// The deliberate escape hatch — writes what `save` refuses.
let unchecked = document.save_unchecked()?;
assert_eq!(unchecked.len(), bytes.len());
# Ok(())
# }
```

**What Word's check does not include, and what would close it.** `mjx-pptx` has a *second* layer —
[`PresentationDefect`](https://docs.rs/mjx-pptx/latest/mjx_pptx/enum.PresentationDefect.html) — for
PresentationML's own graph invariants (`p:cNvPr@id` uniqueness, the three id lists agreeing with the
relationships). **`mjx-docx` has no `DocumentDefect` counterpart.** WordprocessingML's equivalent
invariants — a `w:headerReference` naming a related header part, comment and bookmark ids unique
within the document, a `w:numPr` resolving to a `w:num` that resolves to a `w:abstractNum` — are
enforced *at the point of the edit* instead: [`add_bookmark`](Document::add_bookmark) refuses a name
already in use, [`remove_comment`](Document::remove_comment) and
[`remove_header`](Document::remove_header) sweep the part they orphaned, and
[`resolve_numbering`](Document::resolve_numbering) reports an unresolvable id rather than inventing a
level. What that does **not** cover is a document this library *opened* and then edited into
inconsistency by a route that predates the edit — a header reference already dangling when the file
arrived. Closing it is a whole-document validator in the shape of `mjx-pptx`'s, and it is named here
because a gap nobody has written down is a gap nobody closes.

## The content that is not WordprocessingML

A `.docx` carries several kinds of content that are not `wml` markup. Each lives in its own part (or
inside a run), referenced by relationship id — and each has a different depth of support, which this
table states rather than leaves to be discovered.

| Content | Read | Author | Edit |
|---|---|---|---|
| DrawingML pictures and shapes (`w:drawing`) | [`paragraph_run_content`](Document::paragraph_run_content) → `RunInnerContent::Drawing`, typed through `mjx_dml::wordprocessing_drawing`; `WordprocessingShape` / `TextBoxContent` for a shape and its text box | [`add_inline_picture`](Document::add_inline_picture) | [`remove_drawing`](Document::remove_drawing), by `wp:docPr` id |
| Legacy VML (`w:pict`) | `RunInnerContent::LegacyPicture`, and [`header_footer_vml_drawings`](Document::header_footer_vml_drawings) for the watermarks and text boxes headers routinely carry — typed as `mjx_vml::Drawing` | — | through the typed `mjx_vml::Drawing` |
| Embedded objects (`w:object`) | `RunInnerContent::EmbeddedObject`, with `ObjectEmbed` / `ObjectLink` typed | — | — |
| ActiveX controls (`w:control`) | `Control`, typed | — | — |
| Office MathML (`m:oMath`) | typed through [`mjx_omml`] | [`append_math`](Document::append_math) | [`set_equation_run_text`](Document::set_equation_run_text), addressed by element-name chain |
| Custom XML data (`customXml/itemN.xml`) | [`custom_xml_parts`](Document::custom_xml_parts), [`resolve_data_binding`](Document::resolve_data_binding) | — | — |
| External content (`w:altChunk`) | [`alt_chunk_parts`](Document::alt_chunk_parts), [`alt_chunk_payload`](Document::alt_chunk_payload) | [`add_alt_chunk`](Document::add_alt_chunk) | — |

Unlike `mjx-pptx`'s, **none of this is behind a Cargo feature**: `mjx-vml` is an unconditional
dependency of `mjx-docx`, because Word headers are the primary place VML still appears in the wild
and gating it would spare a caller a dependency they almost certainly need. See
`crates/mjx-docx/Cargo.toml`'s own comment for the reasoning that mirrors — and does not extend —
`mjx-pptx`'s `vml` flag.

### The `wml` preserve-only ledger

`wml.xsd` declares **285 complex types** and **14 global elements**. Phase C models all fourteen part
kinds ([`PartKind`](crate::PartKind)) and every complex type reachable from them; the residue below is
what is *deliberately* preserved rather than typed, each with its reason. Nothing on this list is a
fidelity hole — every one round-trips byte-for-byte — and nothing on it is an oversight.

**One complex type has no Rust type at all.**

| Type | Where it appears | Why it has none |
|---|---|---|
| `CT_ShapeDefaults` | `w:settings/w:shapeDefaults`, `w:settings/w:hdrShapeDefaults` | Its entire content model is `<xsd:any processContents="lax" namespace="urn:schemas-microsoft-com:office:office"/>` — an unknown bucket by the schema's own definition, holding VML office-drawing defaults. There is nothing to model: the element is present as [`Unmodeled`](crate::Unmodeled) and everything inside it is preserved verbatim. A Rust type here would be a wrapper around the raw bucket that already exists |

**Elements typed as [`Unmodeled`](crate::Unmodeled) — structurally present, content opaque.** Most of
these are `CT_Empty`: the element carries no attributes and no children, so "unmodelled" is not a
shortcut, it is the complete truth about them (`w:cr`, `w:tab`, `w:noBreakHyphen`, `w:softHyphen`,
`w:dayShort`/`w:monthShort`/`w:yearShort`/`w:dayLong`/`w:monthLong`/`w:yearLong`, `w:annotationRef`,
`w:footnoteRef`, `w:endnoteRef`, `w:separator`, `w:continuationSeparator`, `w:pgNum`,
`w:lastRenderedPageBreak`, `w:forceUpgrade`). Three are not:

| Element | Why it is opaque |
|---|---|
| `w:pgSz`, `w:pgMar` | The *element* is `Unmodeled`, but its attributes are fully typed — reach them through [`SectionProperties::page_size`](crate::SectionProperties::page_size) / [`page_margins`](crate::SectionProperties::page_margins), which read and write [`PageSize`] / [`PageMargins`]. A second element-shaped type would duplicate what those accessors already own |
| `w:numPicBullet`'s `w:pict` / `w:drawing` | A picture bullet's own artwork. `pict` is VML and `drawing` is DrawingML, both already typed elsewhere; wiring either into the numbering model would pull a second copy of that machinery into a part whose job is list formats. Preserved opaque, exactly as `body.rs` treats the same two names in the one place a caller reaches them typed |

**Content whose *reference* is typed and whose *payload* is preserved.** In each of these the pointer
is modelled — you can find it, follow it, and read the bytes — and the bytes themselves are never
parsed, because parsing them is a different project:

| Cluster | What is typed | Why the payload is not |
|---|---|---|
| `w:altChunk` payload parts | The `w:altChunk`, its `w:altChunkPr`, the relationship and the part's bytes and content type | The payload is a whole document in another format — HTML, RTF, plain text, or a nested `.docx`. Word performs the import when the file is opened; converting one format into another is not this library's job, and a lossy conversion would be worse than none |
| Custom XML Data Storage parts | The relationship, the `ds:itemID`, and an XPath subset good enough to resolve a `w:dataBinding` | The document inside is defined by whoever authored the template. There is no schema to model, by design |
| `w:printerSettings` | The `RelationshipReference` on `w:sectPr` | The part is a binary Windows `DEVMODE`/`DEVNAMES` blob, outside ECMA-376 and outside this project's platform |
| Embedded fonts (`w:embedRegular`/`w:embedBold`/`w:embedItalic`/`w:embedBoldItalic`) | `FontRel` — the relationship and its `w:fontKey` | The parts are obfuscated binary font files. De-obfuscating and parsing a font is a typography project, and re-embedding one has licensing implications this library should not silently take on |
| OLE payloads inside `w:object` | `EmbeddedObject`, `ObjectEmbed`, `ObjectLink` | The stream is an arbitrary application's own format |
| `w:subDoc` | The reference, as `CT_Rel` | A master document's subdocument is a **separate `.docx`** this library was not handed. Opening it would mean reaching outside the package, which nothing here does — the library is bytes-in, bytes-out, and never touches a filesystem |
| Charts and SmartArt inside a `w:drawing` | The `wp:inline`/`wp:anchor` and its `a:graphicData` | `mjx-docx` does not depend on `mjx-chart`, so a chart in a Word document round-trips through `a:graphicData`'s own unknown bucket rather than being typed. `mjx-chart` is a normal crate a caller can depend on directly and point at those bytes; wiring it into `mjx-docx` is a decision nobody has needed yet, not an accident |
| VML beyond `mjx-vml`'s own coverage | Everything `mjx_vml` models | `vml-officeDrawing` and `vml-main` are partially modelled workspace-wide, with no completion owner. What `mjx-vml` does not type falls to its own `Raw` bucket and round-trips |

**Deliberately unmodelled recursion.** `w:divsChild` — a `w:div` nested inside a `w:div` in
`word/webSettings.xml` — falls to `DivContent::Raw`. Every `w:div` a `w:divId` can actually point at
is top-level, and modelling the recursion would add a self-referential type for markup nothing in
this project addresses.

**Not modelled, and not a `mjx-schema-gate` allowlist entry either:** `docProps/custom.xml`, exactly
as for PowerPoint. No committed fixture carries one and nothing here has a use for open-ended
caller-defined properties; a future child that starts authoring them adds the writer and the
allowlist entry together, deliberately, rather than finding a stale one waiting.

## The gaps

Two lists, kept apart on purpose. The first is what this library **decides not to do**, each entry
with the reason it is a decision. The second is what is **built but not yet verified against Office**,
each entry with the work that will verify it. Nothing here is a fidelity hole: every gap below is
*reach* — something you cannot ask for — never something the library loses. A document carrying any
of it round-trips unchanged.

### Non-goals

| Non-goal | What you get instead | Why |
|---|---|---|
| **No rendering, of any kind** | Measurement and resolution: [`effective_run_properties`](Document::effective_run_properties), [`effective_paragraph_properties`](Document::effective_paragraph_properties), the three cell readers | No line breaker, no pagination, no SVG, no PDF. Rendering is a separate phase with no date |
| **A field is never evaluated or refreshed** | The instruction and the cached result, read and written separately and never confused for one another; nesting paired with a stack rather than counted | `TOC`, `PAGE`, `DATE` and `REF` are computed from things a renderer knows — page numbers, the current time, the resolved document — and this library has no renderer and no clock. Editing the code is a structural operation; recomputing the value is not |
| **Tracked changes are read, never applied** | [`revisions`](Document::revisions), and [`text_with_revisions_accepted`](Document::text_with_revisions_accepted) / [`_rejected`](Document::text_with_revisions_rejected), which answer *what the text would be* without rewriting anything | Accepting a revision is a document rewrite with a great many cases (a deletion inside a move inside a table row whose properties also changed), and getting one of them wrong silently destroys the author's content. Answering the question is safe; performing the edit is a feature that needs its own unit of work |
| **A list's displayed number is not computed** | The numbering definition, the instance, the level, the format, the template and the start value, all resolved — [`resolve_numbering`](Document::resolve_numbering) | "3.2.1" depends on every preceding paragraph in the document, the restart rules at each level, and `w:lvlOverride`s along the way. That is a walk of the whole document in reading order — a rendering feature, and the module says so |
| **Pagination is preserved, never recomputed** | `w:lastRenderedPageBreak` comes back exactly where it was | It is a *cache* Word writes describing where its own layout engine broke the page. Recomputing it means having a layout engine; rewriting it without one would be a lie in the file |
| **Hyphenation, kinsoku and line-breaking settings are preserved, never applied** | `w:settings`' own flags and `w:kinsoku` tables, typed and round-tripping | Each is an instruction to a layout engine |
| **`w:documentProtection` is preserved, never enforced** | The typed setting: what kind of protection, and its hash and salt | It is advisory in the format itself — the flags tell an editor what to disallow, and this library is not an interactive editor. Enforcing it would be theatre |
| **Encrypted and password-protected packages are out of scope**; digital signatures are preserved, not processed | A typed error rather than a guess | Decrypting an ECMA-376 Part 2 protected package is a cryptography project, and validating a signature this library may then invalidate by rewriting the container would be worse than not claiming to |
| **A mail merge is not performed** | `w:mailMerge`, `w:odso` and `word/recipients.xml`, all typed and editable | Performing the merge means executing the data source connection the settings describe, which is a network and ODBC concern, not a document one |
| **VML geometry is preserved, not evaluated** | A `v:shape`'s identity, style, references, fill, stroke and children typed; the `path` command string and `v:formulas` verbatim | The same non-goal as the layout engine: turning them into a path is a rendering feature |
| **The schema gate covers the markup this project authors, not the markup it only preserves** | Every fixture part and every authored document is validated against the ECMA-376 XSDs, as a blocking CI job | Custom XML, `altChunk` payloads, printer settings and embedded fonts are namespaces and formats we carry rather than write. Validating them against a schema they were not written to would report noise, so they are reported as *skipped*, by name |
| **A table inside a block-level content control is not a top-level table** | The wrapper's own `content()`, walked by hand — `examples/structured_content.rs` does it | [`table_count`](Document::table_count) and its neighbours address `w:body`'s own content. Row and cell addressing *does* see through row- and cell-level wrappers, because `(row, column)` would otherwise mean two different things depending on markup the caller did not write. Extending that to block level would mean a single flat table index whose numbering changes when someone wraps a table in Word, which is a worse contract than a documented boundary |

### Built, not yet verified against Office

Everything here works and is tested against markup **we wrote**. What none of it has is a run through
real Microsoft Word, and saying so is the point of this section. This list is
[MJXOFF-128](https://github.com/jaxmatrix/mjx-ooxml-rs)'s input: it turns each row into a checklist
entry a person with Office works through. **No agent may mark any of it verified.**

| Not yet verified | What is in place | Who verifies it |
|---|---|---|
| **Every fixture is hand-crafted** | No test in this repository reads a file that Microsoft Word wrote. `sample.docx` and its fifteen siblings were authored for the tests that read them | The Office-authored corpus (MJXOFF-130), which retires this weakness for every other row at once |
| **The effective-properties ladder order** | `docDefaults → table style → numbering → paragraph style → character style → direct`, implemented from ECMA-376 Part 1 §17.7.2's own prose — and against a ticket that asserted the opposite order. The spec is unambiguous; Word's agreement with it is assumed | MJXOFF-122/128 |
| **The toggle-property XOR rule** | §17.7.3's twelve properties combine by XOR, so a paragraph style and a character style that both say bold render *not* bold. Implemented from the prose and unit-tested against it. This is the single most surprising answer this library gives, and the one most worth checking against the real renderer first | MJXOFF-122/128 |
| **Header and footer resolution** | `w:titlePg`, `w:evenAndOddHeaders` and inheritance from the previous section (§17.10.1), implemented from the prose. Where no section back to the first states a reference, this crate answers `None` rather than fabricating the blank header Word would create | MJXOFF-122/128 |
| **`Document::blank`'s part set** | A complete document with `word/document.xml`, both `docProps` parts, and deliberately no styles, numbering, settings, fonts or theme. Margins are fixed at Word's "Normal" template default regardless of page size, and a page those margins do not fit inside is refused with [`DocxError::InvalidPageSize`] rather than written for Word to repair | MJXOFF-122/128 |
| **Everything this crate authors** | Comments, footnotes, endnotes, bookmarks, headers, footers, tables, inline pictures, form fields, `altChunk` imports, styles and numbering definitions are each schema-valid against the ECMA-376 XSDs, in child order, and reopen through this library unchanged | MJXOFF-122/128 — schema-valid is not the same claim as *Word opens it without a repair prompt* |
| **Field instruction editing** | An instruction spread across several `w:instrText` runs is rewritten as one; a field whose zone holds a nested field is refused rather than flattened | MJXOFF-122/128 |
| **The `w:altChunk` import** | The part, its content type and its relationship are written, and the payload is stored byte-for-byte | MJXOFF-122/128 — whether Word performs the import as expected has never been watched happen |

### Whole formats

`.xlsx` opens and round-trips through the OPC and fidelity layers, and `mjx-xlsx` has no editing
surface — it is a scaffold. That is a schedule, not a decision: Excel is the `v0.3` slice, with its
own phase of work. Nothing on this page about `.docx` changes when it lands.

`.pptx` is complete and has [its own copy of this
page](https://docs.rs/mjx-pptx/latest/mjx_pptx/guide/fidelity_and_gaps/).

### What used to be here

Rows close by being *done*, and the ones that have are named so a reader returning to this page can
tell the difference between "gone" and "quietly dropped". Word's list is short because Word's guide
is new; it will grow the way PowerPoint's did.

- **`docProps/core.xml` and `docProps/app.xml` are authored, not merely preserved.** They used to be
  a documented non-goal for every format. `mjx_opc::doc_props` writes both at the packaging layer, so
  [`Document::blank`] and [`Document::blank_with_properties`] and PowerPoint's own constructors all
  call the one writer (MJXOFF-149).
- **`word/settings.xml`, `word/webSettings.xml`, `word/fontTable.xml` and `word/recipients.xml` have
  an owner.** Four of `wml.xsd`'s fourteen global elements were named by no Phase C unit at all until
  MJXOFF-136; all four are now modelled, `CT_Settings`' 98 children included.
- **Content controls, custom XML, smart tags, `w:altChunk` and the glossary document have an owner**
  (MJXOFF-138), which was the other half of the same omission.
- **This page exists.** `fidelity_and_gaps.md` appeared in seven tickets and was chartered by none of
  them, so MJXOFF-128 depended on an artefact no unit produced (MJXOFF-150).

## Stability

The public API is **not stable until `v0.1`**, and Word's surface is younger than PowerPoint's. The
version is `v0.0.x` and the patch number increments each development iteration; a breaking change can
land in any of them, and several already have — the `Unreleased — 0.1.0` section of `CHANGELOG.md` is
the running ledger. Pin an exact version.
