# Phase 2 Handoff — PowerPoint vertical slice

A self-contained brief to resume work on **Phase 2**. Read this first, then `PLAN.md` and `CLAUDE.md`.

## What the project is

A **pure-Rust** Cargo workspace (`mjx-ooxml-rs`) to parse / edit / generate / (later) render OOXML —
PowerPoint, Word, Excel — that **cross-compiles** to desktop, Android, iOS, and WebAssembly (for Tauri).
The overriding requirement is **fidelity**: open any file, edit it, and write it back **without
corrupting the parts you did not touch**.

- Repo: `github.com/jaxmatrix/mjx-ooxml-rs` (remote `origin`).
- `References/` (ECMA-376 spec + schemas, ~80 MB) is **git-ignored**, local-only; the generated
  `mjx-ooxml-types` source is committed so CI never needs it. Test inputs live in `tests/fixtures/`.

## Current state (all merged to `main`, all green, version `0.0.1`)

- **Phase 0** — workspace skeleton (14 crates + `xtask`), CI (fmt · clippy `-D warnings` · test + a
  wasm/Android/iOS/macOS/Windows cross-compile build matrix + a strict rustdoc job), docs, dual
  `MIT OR Apache-2.0` license. `mjx-opc` OPC container with per-part byte-identical round-trip.
  `xtask` schema codegen → `mjx-ooxml-types` (namespaces + `shared-commonSimpleTypes`, deterministic).
- **Phase 1** — `mjx-ooxml-core` string interner + the `RawDocument` raw preservation tree;
  `mjx-xml::fidelity` byte-preserving reader + hand-written writer (round-trips **every fixture XML
  part byte-identically**); `mjx-mce` markup-compatibility preserve + non-mutating resolve.
- **Docs/versioning** — full rustdoc (guides + doctests + a facade docs hub), `missing_docs` enforced,
  `CHANGELOG.md`, milestone scheme (`v0.1` PowerPoint, `v0.2` Word, `v0.3` Excel).
- ~66 tests green. Start Phase 2 from a fresh `main` (`git switch main && git pull`).

## Process & constraints (also in `CLAUDE.md` / `AGENTS.md` / `CONTRIBUTING.md` and agent memory)

- **Process:** Plan → **Plan-Optimization** (memory / speed / reliability / correctness; no
  monkey-patching) → thorough atomic implementation. **Discussion-first every session**: discuss the
  plan and tradeoffs before coding. Finish each atomic piece completely, correctly, with tests.
- **Fidelity-first, pure-Rust only** (no C/system libs shipped), `unsafe_code = deny`, no
  `unwrap`/`panic` on untrusted input (typed `thiserror`). Layering points **downward only**;
  `quick-xml` lives only behind `mjx-xml`, ZIP only behind `mjx-opc`.
- **Comprehensive, self-explanatory names** — never cryptic OOXML symbols; expand them, source meaning
  from the ECMA-376 prose, and preserve the exact wire token for (de)serialization.
- **Git:** feature branch per piece → **PR** → review before continuing. **Atomic commits**, commit
  only when green. **No `Co-Authored-By` / AI-attribution trailers.**
- **Settled model decisions:** hybrid (arena for bulk data / owned trees for small structures);
  copy-on-write parts (raw bytes until first mutation); interning + `Cow`; **one interner per part**.
- **TDD:** write the failing test first; keep every increment green.

## API surface Phase 2 builds on (verified)

- **`mjx-ooxml-core`** — the raw tree is **fully public & mutable**:
  `RawDocument { pub interner: Interner, pub bom: bool, pub prologue: Vec<RawNode>, pub root:
  RawElement, pub epilogue: Vec<RawNode> }`;
  `RawElement { pub name: RawName, pub attributes: Vec<RawAttribute>, pub children: Vec<RawNode>, pub
  empty: bool }`;
  `RawNode = Element | Text(Box<[u8]>) | CData | Comment | ProcessingInstruction | Declaration |
  DocType`;
  `RawName { pub prefix: Option<Symbol>, pub local: Symbol, pub namespace: Option<Symbol> }` (Copy);
  `RawAttribute { pub name, pub value: Box<[u8]> /*raw escaped*/, pub quote: QuoteStyle }`;
  `Interner::{new, intern(&mut), get, resolve(&self)->&str, len, is_empty}` (`Symbol` inner is private
  — get symbols via `interner.intern`).
- **`mjx-xml`** — `fidelity::parse(&[u8]) -> Result<RawDocument, XmlError>`,
  `fidelity::serialize(&RawDocument, &mut Vec<u8>)`, `fidelity::serialize_to_vec(&RawDocument) ->
  Vec<u8>`. **Edit path = mutate the tree, then `serialize_to_vec`.** The writer emits self-closing
  only when `empty && children.is_empty()` (push a `RawNode::Text` child to turn `<a:t/>` into
  `<a:t>…</a:t>`).
- **`mjx-opc`** — **READ-ONLY today; this is the PR 2a gap.** `Package` fields are private; only
  accessors exist: `open`, `save() -> Vec<u8>` (re-zips entries in order), `entries`, `content_types`,
  `relationships`, `relationships_for`, `content_type_of(&PartName)`, `part_names`,
  `part_bytes(&PartName) -> Option<&[u8]>`. `PartBody = Raw(Vec<u8>)` **only** (no `Parsed`). There is
  no way to replace / add / remove a part or mutate content-types / rels — **PR 2a adds this.**
- **`mjx-mce`** — `resolve(&RawDocument, &UnderstoodNamespaces) -> Result<ResolvedElement,
  ResolveError>` (non-mutating borrowed read view).
- Stubs to fill: `mjx-derive` (proc-macro, **empty deps** — add `syn`/`quote`/`proc-macro2`),
  `mjx-dml`, `mjx-pptx` (deps already wired).

## The slide to model (`tests/fixtures/sample.pptx`, `ppt/slides/slide1.xml`)

`p:sld` → `p:cSld` → `p:spTree` → `p:sp` (`p:nvSpPr` / `p:cNvPr@id,name` · `p:spPr` · `p:txBody`) →
`a:txBody` (`a:bodyPr` · `a:lstStyle?` · `a:p`+) → `a:p` (`a:pPr?` · runs `a:r`/`a:br`/`a:fld` ·
`a:endParaRPr?`) → `a:r` (`a:rPr?` · `a:t` = text). The slide's text is `Hello OOXML`. **Significant
whitespace text nodes sit between block elements — the fidelity model must preserve them.**
Model `sp`/`txBody`/`p`/`r`/`t` + `cNvPr@id,name` as typed; keep `bodyPr`/`pPr`/`spPr`/`nvGrpSpPr`/
`style`/`extLst` and non-`sp` shape-tree children (`grpSp`/`pic`/…) as opaque `Raw` content.
Adding a slide touches **5 parts**: new `ppt/slides/slideN.xml`, its `_rels`, `presentation.xml`
(`p:sldId`), `presentation.xml.rels` (`Relationship` type `.../slide`), and `[Content_Types].xml`
(`Override`). Note `p:sldId@id` (≥256) ≠ `p:cNvPr@id`.

## Decided architecture (owned typed model — the ambitious path)

**Full owned typed model + the `FromXml`/`ToXml` derive**, delivered as **several small PRs**.

- **Fidelity mechanism — ordered mixed content:** each modeled complex type stores its children as an
  ordered `Vec` of a per-type **content enum** whose variants are the known typed children **plus
  `Raw(RawNode)`** for whitespace / comments / unknown elements → order + insignificant whitespace are
  preserved in place. Attributes = dedicated typed fields + `extra_attributes: Vec<RawAttribute>`.
- **Traits (in `mjx-ooxml-core`):** `from_xml(&RawElement, &Interner) -> Result<Self>` and
  `to_xml(&self, &mut Interner) -> RawElement`. Typed fields hold owned `String`s; the `Raw` content
  and `extra_attributes` carry interned `Symbol`s. **One interner per part** throughout.
- **`mjx-derive`** generates these from field attributes
  (`#[xml(attribute=… / child=… / children=… / choice / text)]`); **design it against 2–3 hand-written
  impls first**, then automate.
- **Copy-on-write parts (`mjx-opc`):** keep raw bytes; parse a `RawDocument` on demand and cache it; a
  `dirty` flag → `save()` emits raw bytes for clean parts, `fidelity::serialize` for dirty ones.

## Sub-PR sequence (implement in order; discussion-first each)

1. **PR 2a — `mjx-opc` copy-on-write edit surface (THE IMMEDIATE NEXT PIECE).** Add
   `part_tree(&PartName)` (read, no dirty) / `part_tree_mut(&PartName)` (parse + mark dirty) →
   `&mut RawDocument`; `save()` emits raw bytes for clean parts and `fidelity::serialize` for dirty
   ones; `insert_part` / `remove_part`; content-type `Override` + `Relationship` mutation. Works
   purely on raw trees (no typed model yet).
   **Exit test:** open a fixture, edit one part's tree, `save()`, reopen → that part reflects the edit
   and **every other part is byte-identical**. Optimize: don't hold raw bytes + tree longer than
   needed; **reading a part must NOT dirty it** (byte-identity contract).
2. **PR 2b —** `FromXml`/`ToXml` traits + `mjx-derive` + DrawingML text types (`mjx-dml`): hand-write
   `a:t`/`a:r`/`a:p`/`a:txBody`, then derive. **Exit:** `txBody` `RawElement` → typed → `to_xml` →
   byte-identical; read its text.
3. **PR 2c —** PresentationML slide model (`mjx-pptx`): `Presentation` (owns a `Package`) → `Slide` →
   `Shape` (`p:sp`) → `TextBody`; `slide.text()`, `run.set_text()`. **Exit:** open `sample.pptx`, read
   text, change a run, `save()`, reopen → text changed, other parts byte-identical.
4. **PR 2d —** construction + package ops + **end-to-end**: add a text-box shape, add a slide (5
   parts). **Phase 2 exit:** the produced `.pptx` opens cleanly in PowerPoint + LibreOffice Impress;
   untouched parts byte-identical; `soffice --headless` convert as a CI corruption canary.

## How to resume (first actions)

1. `git switch main && git pull` (PR #1 is merged; the old `feat/phase1-fidelity-mce` branch is stale).
2. **Discussion-first** on **PR 2a** design (the `mjx-opc` copy-on-write edit surface) — confirm the
   `part_tree` / `part_tree_mut` + `dirty` + save-from-tree design and the add/remove + content-type +
   rels mutation API.
3. `git switch -c feat/phase2a-opc-edit`, implement TDD, keep green, push, open a PR, continue after
   review.

Per-PR verification: `cargo build` / `test --workspace` / `clippy -D warnings` / `fmt --check` / strict
`cargo doc` green; Android cross-build; the per-PR exit test. Phase 2 exit = a real edited `.pptx` opens
in Office / LibreOffice with untouched parts byte-identical.
