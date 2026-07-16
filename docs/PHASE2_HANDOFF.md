# Phase 2 Handoff — PowerPoint vertical slice

A self-contained brief to **resume Phase 2 from a cold start**. Read this first, then `PLAN.md`,
`CLAUDE.md`, and `docs/DRAWINGML_PRESET_SHAPES.md`. It is written to survive a full context reset: it
records what is done, the exact next piece, and — critically — **the places a fresh session tends to
drift or hallucinate**, so you don't re-derive (wrongly) what is already settled.

---

## 0. The mission — do not lose this (read before touching code)

This project exists for **one overriding reason: fidelity.** Open any `.pptx`/`.docx`/`.xlsx`, edit the
part you mean to edit, write it back, and **every part you did not touch is byte-for-byte unchanged.**
That is the product. Everything else (an ergonomic API, rendering, shapes) is secondary and must never
be bought at the cost of fidelity.

Concretely, the **round-trip contract** is: **per-part decompressed-payload byte identity** + structural
container identity. It is *not* identical ZIP bytes (compression may differ) — do not assert whole-file
ZIP equality. A part we model and edit re-serializes from its tree; a part we don't touch re-emits its
**original bytes verbatim**.

If you ever find yourself "cleaning up" XML — normalizing whitespace, reordering attributes,
pretty-printing, dropping unknown elements, re-escaping text, changing namespace prefixes — **stop. That
is a fidelity violation, not an improvement.** The preservation tree keeps raw escaped bytes on purpose.

---

## 1. What the project is

A **pure-Rust** Cargo workspace (`mjx-ooxml-rs`) to parse / edit / generate / (later) render OOXML —
PowerPoint, Word, Excel — that **cross-compiles** to desktop, Android, iOS, and WebAssembly (for Tauri).

- Repo: `github.com/jaxmatrix/mjx-ooxml-rs` (remote `origin`). Default branch `main` is
  **branch-protected** — all changes land via PR.
- `References/` (ECMA-376 spec + XSD schemas, ~80 MB) is **git-ignored**, local-only. The generated
  `mjx-ooxml-types` source is **committed** so CI never needs `References/`. Regenerate only when the
  schema tables change: `cargo run -p xtask -- codegen` (needs local `References/`). Test inputs live in
  `tests/fixtures/`.
- Crate layering (dependencies point **downward only**, never up or sideways):
  `mjx-ooxml-core` · `mjx-xml` · `mjx-derive` → `mjx-opc` · `mjx-mce` · `mjx-ooxml-types` →
  `mjx-dml` · `mjx-omml` · `mjx-chart` · `mjx-vml` → `mjx-pptx` · `mjx-docx` · `mjx-xlsx` → `mjx-ooxml`.

---

## 2. Current state — everything merged to `main`, green, version `0.0.1`

**Foundations (Phase 0–1):**
- **Phase 0** — 14-crate workspace + `xtask`; CI (fmt · clippy `-D warnings` · test + a
  wasm/Android/iOS/macOS/Windows cross-compile matrix + a strict rustdoc job); `MIT OR Apache-2.0`.
  `mjx-opc` OPC container. `xtask` schema codegen → `mjx-ooxml-types` (committed).
- **Phase 1** — `mjx-ooxml-core` interner + `RawDocument` **raw preservation tree**; `mjx-xml::fidelity`
  byte-preserving reader + writer (round-trips every fixture XML part byte-identically); `mjx-mce`
  markup-compatibility preserve + non-mutating resolve.

**Phase 2 (PowerPoint slice) — merged so far:**
- **PR 2a (#4)** — `mjx-opc` copy-on-write edit surface. `PartBody` = `Raw` / `Parsed{original,tree}` /
  `Edited(tree)`. `part_tree` (read, non-dirtying, keeps original bytes) vs `part_tree_mut` (dirties →
  re-serialized). `insert_part` / `remove_part`, content-type `Override` + `Relationship` mutation
  (control parts kept in lock-step). Control parts rejected by the generic tree API.
- **PR 2b.1 (#6)** — `FromXml`/`ToXml` traits in `mjx-ooxml-core::convert` (dep-free) +
  `mjx_xml::text::{escape_text,unescape_text}` + the hand-written DrawingML text model in `mjx-dml`
  (`TextBody`/`Paragraph`/`TextRun`/`Text`).
- **PR 2b.2 (#7)** — `mjx-derive` proc-macro `#[derive(FromXml, ToXml)]`; the 4 text types migrated to
  it (hand-written impls deleted). Content **enums stay hand-written**; the macro emits only impl blocks.
- **Ledger (#5)** — `docs/DRAWINGML_PRESET_SHAPES.md`: the preset-shape porting ledger + naming
  methodology (see §8).
- **PR 2c (#8)** — first PresentationML typed model in `mjx-pptx`: `Presentation` (owns `Package` +
  resolved presentation part + ordered slide `PartName`s) with **index-addressed** read/edit:
  `open`/`save`, `slide_count`/`slide_part`, `shape_count`/`shape_text`, `set_shape_text`.
- **PR 2d.1 (#9)** — **construction begins**: `Presentation::add_text_box(slide_idx, text, bounds)`
  builds a whole `p:sp` text box and splices it under the existing `p:spTree`; `ShapeBounds` (EMU
  geometry); internal `build` element builders; the **LibreOffice office-open canary** (the Phase-2 exit
  gate) + a CI `office-open` job. Details in §6/§7.

**~139 workspace tests green.** `main` is at commit `ae4f681` (top of 2d.1). **No open PRs.**

> **➡ RESUME POINT: PR 2d.2 — "add a slide".** Fully specced in §9. This is the immediate next piece.

---

## 3. Guardrails — where a fresh session drifts or hallucinates (READ THIS)

These are the mistakes a new session makes when it lacks the accumulated context. Each is a real hazard,
with the correct fact.

**A. Objective drift — "making the API nicer" at fidelity's expense.**
The temptation is to normalize/prettify XML or drop things we don't model. Never. Untouched parts emit
verbatim; modeled types carry a `Raw` catch-all for everything unknown. If a test only checks that text
round-trips, it is **vacuous** — pair every round-trip assertion with a structural one *and* a byte-level
fidelity assertion (only the edited part changed).

**B. The interner-per-part invariant.**
There is **one `Interner` per part**, living on that part's `RawDocument`. A value produced by
`to_xml(&mut interner)` **must** be serialized with the *same* interner it was built against — its
`Symbol`s are indices into that interner only. The proven edit pattern is the split-borrow:
```rust
let RawDocument { interner, root, .. } = package.part_tree_mut(&part)?;
// build/edit using `interner` and place the result into `root`
```
Never build a subtree with interner A and splice it into a tree owned by interner B.

**C. Element namespaces are resolved; ATTRIBUTE namespaces are NOT.**
The fidelity reader resolves an *element's* namespace (`RawName.namespace` is `Some(uri_symbol)`), but
for *attributes* it keeps only the literal **prefix** (`RawName.namespace` is `None`). So `r:id` cannot
be found by namespace — you find the prefix bound to the relationships URI via
`nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)`, then match that prefix symbol.
This bites everyone once; it's already solved in `nav.rs`.

**D. The writer emits `prefix:local` and ignores the resolved namespace.**
When you *build* a new element it needs the correct **literal prefix** (`p` for PresentationML, `a` for
DrawingML) for the bytes to come out right — and it *also* needs the resolved **namespace symbol** so
read-back (`nav::name_is`) can find it by `(namespace, local)`. The `build` module sets both. A subtree
spliced under an element whose ancestor already declares `xmlns:p`/`xmlns:a` needs **no** new `xmlns`
attributes; a brand-new *part* (a new slide) is a fresh document and **must** declare its own.

**E. Match children by `(namespace, local)`, accepting BOTH URIs, never by prefix.**
Every schema has a *strict* (`purl.oclc.org/ooxml/...`) and a *transitional*
(`schemas.openxmlformats.org/...`) URI. Office files use transitional; the code accepts either
(`namespace == ns.transitional || Some(namespace) == ns.strict`). See `nav::name_is`. Do not match on
the prefix string, and do not validate a wrapper element's own name against a namespace (a slide
serializes `CT_TextBody` as `p:txBody`, not `a:txBody`).

**F. Self-explanatory names, sourced from the spec — never guessed.**
Public identifiers must not be cryptic OOXML tokens (`ST_Jc`, `t`, `ctr`). Expand them to full words,
source the meaning from **ECMA-376 Part 1 prose**, and record the exact wire token in the docs for
(de)serialization. **For DrawingML shape control points specifically: do NOT invent adjustment names.**
Derive the meaning/math from `presetShapeDefinitions.xml` (the handle definitions + guide formulas). See
§8 — this is the single biggest hallucination risk in the upcoming shapes work.

**G. Pure-Rust shipped; C tools are test/CI-only.**
`soffice` (LibreOffice) and `xmllint` may be used in tests/CI **only** — never as a dependency of a
shipped crate. `quick-xml` lives only behind `mjx-xml`; the ZIP backend only behind `mjx-opc`.
`unsafe_code = "deny"` workspace-wide. **No `unwrap`/`panic`/`expect` on untrusted input** in library
code — inputs are hostile files; return typed `thiserror` errors. `anyhow` only in `xtask`/tests.

**H. Git discipline.**
Feature branch per piece → PR → review. **Atomic commits, only when `cargo build` + `cargo test
--workspace` are green.** **Never add `Co-Authored-By` or any AI-attribution trailer.** Never stage
`References/`. `main` is branch-protected — you cannot push to it directly.

**I. Don't hallucinate the fixture.**
`tests/fixtures/sample.pptx` has exactly **one slide** with **one `p:sp`** (a title) whose text is
`Hello OOXML`. Concrete ids you can rely on (verify, don't assume for other files): slide `p:spTree`
group `p:cNvPr id="1"`, title `p:cNvPr id="2"`; `presentation.xml` `p:sldIdLst` = `<p:sldId id="256"
r:id="rId2"/>`; masters live in a **separate** `p:sldMasterIdLst` at `id="2147483648"`;
`presentation.xml.rels` max id `rId3`; `slide1.xml.rels` has one `slideLayout` rel →
`../slideLayouts/slideLayout1.xml`. Root elements declare transitional `xmlns:p`/`xmlns:a`/`xmlns:r`.

**J. Process, not just output.**
Every unit of work: **Plan → Plan-Optimization (memory/speed/reliability/correctness; no shortcuts) →
thorough atomic implementation with tests + full docs.** Discussion-first each session. Smaller PRs, each
carrying all its own tests and complete docs on every public item.

---

## 4. The verified API surface Phase 2 builds on

**`mjx-ooxml-core`** — the raw tree is public & mutable:
`RawDocument { pub interner, pub bom, pub prologue, pub root, pub epilogue }`;
`RawElement { pub name: RawName, pub attributes: Vec<RawAttribute>, pub children: Vec<RawNode>, pub
empty: bool }` (invariant: `empty == true` ⟹ `children` empty; writer self-closes iff `empty &&
children.is_empty()`);
`RawNode = Element | Text(Box<[u8]>) | CData | Comment | ProcessingInstruction | Declaration | DocType`
(Text/attr values are **raw escaped bytes**);
`RawName { pub prefix: Option<Symbol>, pub local: Symbol, pub namespace: Option<Symbol> }` (Copy);
`Interner::{new, intern(&mut)->Symbol, resolve(&self)->&str, ...}`;
`convert::{FromXml::from_xml(&RawElement,&Interner)->Result<Self,FromXmlError>, ToXml::to_xml(&self,&mut
Interner)->RawElement}`.

**`mjx-xml`** — `fidelity::{parse(&[u8])->Result<RawDocument,XmlError>, serialize(&RawDocument,&mut
Vec<u8>), serialize_to_vec(&RawDocument)->Vec<u8>}`; `text::{escape_text(&str)->Cow (escapes `<` `&`),
unescape_text(&str)->Result<Cow>}`.

**`mjx-opc`** — the edit surface (PR 2a): `Package::{open, save()->Vec<u8>, entries, content_types,
relationships, relationships_for(Option<&PartName>), content_type_of, part_names, part_bytes,
part_tree(&PartName)->&RawDocument (read, non-dirtying), part_tree_mut->&mut RawDocument (dirties),
insert_part(&PartName, content_type, Vec<u8>) (stores Raw + auto-registers a content-type Override iff
not already resolved), remove_part, set_content_type_override, remove_content_type_override,
add_relationship(Option<&PartName>, Relationship) (appends to the source's .rels, or SYNTHESIZES a fresh
one when none exists — caller supplies rel.id, no de-collision), remove_relationship}`.
`Relationship { id, rel_type, target, mode: TargetMode::{Internal,External} }`. `PartName::{new(&str)
(rejects `.`/`..`, requires leading `/`), as_str, zip_name, from_zip_name}`.

**`mjx-dml`** — derive-based text model; fields private; read accessors + `text()`; mutation:
`Text::set_text`, `TextRun::set_text(&str)->bool` (false if no `a:t`; does not synthesize one),
`Paragraph::runs`/`runs_mut`, `TextBody::paragraphs`/`paragraphs_mut`. **No public constructors** — this
is why `mjx-pptx` builds shapes itself (§6).

**`mjx-pptx`** — `Presentation { package, presentation_part, slides: Vec<PartName> }`. Public:
`open(&[u8])`, `save()->Vec<u8>`, `presentation_part`, `slide_count`, `slide_part(idx)`,
`shape_count(slide)`, `shape_text(slide, shape)`, `set_shape_text(slide, shape, run, text)`,
`add_text_box(slide, text, ShapeBounds)->usize`. `ShapeBounds { offset_x_emu, offset_y_emu, width_emu,
height_emu }` + `EMU_PER_INCH`/`EMU_PER_POINT`/`new`/`from_inches`. Internal modules: `nav` (namespace
navigation + rel target resolution), `slide` (spTree navigation), `build` (prefixed element builders),
`constants`, `error` (`PptxError`, `#[non_exhaustive]`, `thiserror`).

**`mjx-mce`** — `resolve(&RawDocument, &UnderstoodNamespaces)->Result<ResolvedElement,ResolveError>`
(non-mutating borrowed view).

---

## 5. Reference — the slide model (`tests/fixtures/sample.pptx`)

`p:sld` → `p:cSld` → `p:spTree` → (`p:nvGrpSpPr`, `p:grpSpPr`, then shapes) → `p:sp` (`p:nvSpPr` /
`p:cNvPr@id,name` · `p:spPr` · `p:txBody`) → text body children `a:bodyPr` · `a:lstStyle?` · `a:p`+ →
`a:p` (`a:pPr?` · runs `a:r`/`a:br`/`a:fld` · `a:endParaRPr?`) → `a:r` (`a:rPr?` · `a:t` = text). Only
`sp`/`txBody`/`p`/`r`/`t` are typed; `bodyPr`/`pPr`/`spPr`/`nvGrpSpPr`/`grpSpPr`/… and non-`sp`
shape-tree children (`grpSp`/`pic`/…) stay opaque `Raw`. **Significant whitespace text nodes sit between
block elements — preserve them.**

---

## 6. How construction works today (the pattern PR 2d.2 extends)

`mjx-pptx` builds the **entire** shape subtree itself — **no `mjx-dml` changes** — because the DML text
types have no public constructors and the shape skeleton (`nvSpPr`/`spPr`/`xfrm`/`prstGeom`) isn't
DML-modeled anyway. The `build` module has generic prefixed builders — `qname`, `attr` (unprefixed,
value escaped), `leaf` (self-closing), `node` (container), `text_leaf` (escaped char-data) — each
setting both the literal prefix and the resolved namespace (Guardrail D). `add_text_box` uses the
split-borrow (Guardrail B), computes a fresh unique `p:cNvPr@id` (max descendant id + 1), builds
`p:sp` → (`p:nvSpPr` with `cNvPr`/`cNvSpPr txBox="1"`/`nvPr`) + (`p:spPr` with `a:xfrm`(`a:off`/`a:ext`)
+ `a:prstGeom prst="rect"`/`a:avLst`) + (`p:txBody` with `a:bodyPr`/`a:lstStyle` + one `a:p` per line),
pushes it onto `spTree.children`, and returns the new shape index.

**The office-open canary (Phase-2 exit gate)** lives in `crates/mjx-pptx/tests/office_open.rs`: it drives
LibreOffice headless (`--convert-to pdf:impress_pdf_Export`, fresh `-env:UserInstallation` profile,
std-only timeout) and asserts a **valid non-empty `%PDF`** came out — soffice's exit code is unreliable,
so the produced PDF is the real signal it parsed and rendered. It **skips cleanly** when no
`soffice`/`libreoffice` is found, unless `MJX_REQUIRE_SOFFICE=1` (set by the CI `office-open` job, which
installs `libreoffice-impress`), which turns "missing" into a hard failure so coverage can't vanish.

---

## 7. The remaining Phase-2 PR sequence

| PR | Status | Scope |
|----|--------|-------|
| 2a | ✅ merged (#4) | OPC copy-on-write edit surface |
| 2b.1 | ✅ merged (#6) | `FromXml`/`ToXml` + hand-written DML text model |
| 2b.2 | ✅ merged (#7) | `mjx-derive` proc-macro; text types migrated |
| 2c | ✅ merged (#8) | `mjx-pptx` open/read/edit/save (index API) |
| 2d.1 | ✅ merged (#9) | `add_text_box` + `ShapeBounds` + builders + office-open canary + CI |
| **2d.2** | **➡ NEXT** | **`add_slide`** — see §9 |

After 2d.2, Phase 2's construction story is complete. The **DrawingML preset-shape geometry workstream**
(§8) is the next major body of work.

---

## 8. The DrawingML preset-shape workstream (after 2d) — and its #1 hallucination risk

Ledger: `docs/DRAWINGML_PRESET_SHAPES.md` (186 preset shapes). A `.pptx` stores only
`prstGeom@prst` (one of 187 `ST_ShapeType`) + an `avLst` of guide (`gd`) overrides.

**Plan: fidelity model first (preserve `prst` + `avLst` exactly), THEN named control parameters in
batches.** The user's explicit, non-negotiable instruction: traditional PowerPoint shapes expose raw
adjustments (`adj1`, `adj2`) whose meaning is non-obvious. **We expose self-explanatory named
parameters instead** (e.g. a rounded rectangle's `corner_radius_fraction`, not `adj1`) — and the
meaning/math is **DERIVED from `presetShapeDefinitions.xml`** (the `ahXY`/`ahPolar` handle definitions +
guide formulas), **never guessed**. ~43 complex shapes need hand-named parameters sourced from ECMA
prose. Maintain the ledger as shapes are ported. The path/evaluator/render model is deferred to the
rendering phase. Known spec anomalies: `upArrow` missing, `upDownArrow` duplicated in the spec file.

**If a future session starts naming shape adjustments from intuition instead of the spec file, it has
drifted — stop and go to `presetShapeDefinitions.xml`.**

---

## 9. ➡ THE NEXT PIECE: PR 2d.2 — add a slide

Add a brand-new blank slide (optionally with a text box) to the deck, wired to the same layout as slide
0, so the produced `.pptx` opens in LibreOffice/PowerPoint with every existing part byte-identical.

**New API (on `Presentation`):**
```rust
pub fn add_slide(&mut self) -> Result<usize, PptxError>;                     // returns new slide index
pub fn add_slide_with_text(&mut self, text: &str, bounds: ShapeBounds) -> Result<usize, PptxError>;
```

**Empty-slide byte template** (`build::empty_slide_bytes()`): a fresh document, so it **declares its own**
`xmlns:p`/`xmlns:a`/`xmlns:r`. Minimal valid body — note `p:cNvGrpSpPr` (the *group* non-visual props,
not `p:cNvSpPr`) and `a:masterClrMapping` ("inherit the master's color map", the safe empty choice):
```xml
<p:sld xmlns:p="…/presentationml/2006/main" xmlns:a="…/drawingml/2006/main" xmlns:r="…/relationships">
  <p:cSld><p:spTree>
    <p:nvGrpSpPr><p:cNvPr id="1" name=""/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>
    <p:grpSpPr/>
  </p:spTree></p:cSld>
  <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>
</p:sld>
```

**Three independent id domains — DO NOT cross them:**
1. `p:sldId@id` (deck-scoped, ≥256): scan **only** `p:sldIdLst > p:sldId@id` (never `p:sldMasterIdLst`),
   `max(that, 255) + 1` → `257` for the fixture.
2. presentation-scoped `rId`: scan `presentation.xml.rels` ids, `max + 1` → `rId4`.
3. per-slide `p:cNvPr@id`: only relevant for `add_slide_with_text` (reuse `add_text_box`).

**Slide part name:** scan existing `/ppt/slides/slideK.xml` basenames, `N = max K + 1` →
`/ppt/slides/slide2.xml`.

**Layout:** read slide 0's `.rels` (`relationships_for(Some(&self.slides[0]))`), find the
`by_type(REL_SLIDE_LAYOUT)` rel; reuse its `target` string **verbatim** (new slide is in the same
directory). No slide 0 / no layout rel → new `PptxError::NoSlideLayout`.

**Four OPC touches (in order), then update `self.slides`:**
1. `insert_part(&new_part, constants::CONTENT_TYPE_SLIDE, empty_slide_bytes())`.
2. `add_relationship(Some(&new_part), Relationship{ id:"rId1", rel_type:REL_SLIDE_LAYOUT, target:<layout
   target>, mode:Internal })` — synthesizes the new slide's `.rels`.
3. `add_relationship(Some(&self.presentation_part), Relationship{ id:new_rid, rel_type:REL_SLIDE,
   target:"slides/slideN.xml", mode:Internal })` — relative to `/ppt/`.
4. `part_tree_mut(&self.presentation_part)`: append `<p:sldId id="{new_sld_id}" r:id="{new_rid}"/>` to
   `p:sldIdLst`. The `r:id` is a **prefixed** attribute — resolve the `r` prefix via
   `nav::namespace_prefix(root, interner, SHARED_RELATIONSHIP_REFERENCE)` (Guardrail C) and build a
   prefixed attribute with that symbol (new `build::attr_prefixed(interner, prefix_sym, local, value)`).
   No `r` prefix bound → `MalformedPresentation`.
Then `self.slides.push(new_part)`.

**New symbols:** `constants::REL_SLIDE_LAYOUT =
"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout"`;
`PptxError::NoSlideLayout`; `build::{empty_slide_bytes, attr_prefixed}`.

**Tests (each round-trip paired with a structural assertion):** `slide_count` 1→2; `slide_part(1) ==
"/ppt/slides/slide2.xml"`; **reopen with a fresh `Presentation::open`** and assert `slide_count()==2` +
`shape_count(1)` (0 for `add_slide`, 1 + `shape_text(1,0)` for `add_slide_with_text`) — this proves the
whole rels→sldIdLst→rels chain we wrote is internally consistent; new part has `CONTENT_TYPE_SLIDE`; new
`.rels` has exactly the slideLayout rel to slide 0's target; id domains (`sldId@id==257`, `rId==rId4`,
layout rel `rId1`); **fidelity** (every pre-existing part except `presentation.xml` byte-identical to a
fresh-open snapshot; `presentation.xml` differs only by the appended `p:sldId`); and **extend
`office_open.rs`** with an `add_slide_with_text` case (assert valid `%PDF`).

**The full, decisive design (both 2d.1 and 2d.2) is preserved in the plan file
`~/.claude/plans/okay-we-will-start-purrfect-puddle.md`** (agent-local, not committed).

---

## 10. How to resume (first actions for the new session)

1. `git switch main && git pull --ff-only` (should already contain 2d.1 at/after commit `ae4f681`).
2. **Discussion-first** — restate the 2d.2 plan (§9), confirm the three id domains and the empty-slide
   template, then proceed.
3. `git switch -c feat/phase2d2-add-slide`; implement TDD (constants/error → `empty_slide_bytes` +
   `attr_prefixed` → `add_slide` → `add_slide_with_text` → tests → office-open case); keep every
   increment green; **atomic commits, no AI-attribution trailer**; push; open a PR vs `main`.

**Per-PR verification gate:** `cargo test --workspace` · `cargo clippy --workspace --all-targets -- -D
warnings` · `cargo fmt --all --check` · `RUSTDOCFLAGS="-D warnings -D rustdoc::broken_intra_doc_links -D
rustdoc::private_intra_doc_links" cargo doc --workspace --no-deps` · the per-PR exit test · run the
office-open canary locally (LibreOffice **is** installed on this machine). **Phase-2 exit** = a real
constructed/edited `.pptx` opens in LibreOffice with untouched parts byte-identical.
