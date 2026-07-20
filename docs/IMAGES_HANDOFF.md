# Handoff — PowerPoint images / pictures (`p:pic` + media parts) — NEXT WORKSTREAM

A self-contained brief to start the **images** workstream from a cold start. Read after
`docs/PHASE2_HANDOFF.md` (§3 guardrails) and `docs/DRAWINGML_FILL_HANDOFF.md` (the `blipFill` model).
Nothing here is implemented yet — this is the plan plus the facts already verified against the repo
and the spec, so the next session does not re-derive them.

## Why this is next

With fill, outline (`a:ln`), and effects (`a:effectLst`) all modeled explicitly **and** effectively,
the `spPr` visual trilogy is done. `PLAN.md` Phase 3 lists two remaining items — **images** and
**layout/master** — and images is the bigger user-visible gap: **you currently cannot put a picture on
a slide.** The fill workstream deliberately deferred it ("adding a `blipFill` image needs an image part
+ a relationship — its own step").

## Current state — verified, not assumed

- **`mjx-pptx` has zero image support.** The only trace in the whole crate is a doc comment on
  `set_shape_fill` noting that a `FillSpec::Blip` writes only the `a:blip@r:embed` reference and that
  "the image part and its relationship must already exist in the package".
- **The OPC plumbing is ready** (`crates/mjx-opc/src/package.rs`):
  - `Package::insert_part(&PartName, content_type: &str, bytes: Vec<u8>)` — works for **binary** parts
    and auto-registers a content-type `Override` iff the type isn't already resolved.
  - `Package::add_relationship(Option<&PartName>, Relationship)` — appends to the source's `.rels`, or
    synthesizes one when absent. Caller supplies `rel.id` (no de-collision — allocate it yourself).
  - `Package::part_names` / `part_bytes` / `content_type_of` for scanning + reading back.
  - Precedent: `crates/mjx-opc/tests/edit_surface.rs` already adds a `../media/image1.png` relationship.
- **`mjx-dml::BlipFill` already models the picture fill** — `BlipFill::new(interner, rel_id, mode)`,
  `image_rel_id()`, `image_link_id()`, `mode()`, plus interner-free `FillSpec::Blip { rel_id, mode }`
  and `BlipFillMode::{Tile, Stretch, None}`. **No `mjx-dml` changes are expected for I1.**
- **`tests/fixtures/sample.pptx` has NO media parts** (11 parts total). So tests should synthesize a
  tiny valid PNG in-test rather than committing a binary fixture.
- **Missing constants** in `crates/mjx-pptx/src/constants.rs`: no `REL_IMAGE`, no image content types
  (it currently has only office-document/slide/slideLayout/slideMaster/theme + 3 content types).
- **Shape enumeration is `p:sp`-only** — `slide::shapes` = `nav::children(sp_tree, .., PML, "sp")`, so
  every index-addressed API (`shape_count`, `shape_text`, `shape_fill`, …) skips `p:pic` entirely.
  Pictures therefore need their own parallel index space, not a silent change to `shapes()`.

## Verified schema (`pml.xsd`, transitional)

```xml
<xsd:complexType name="CT_Picture">
  <xsd:sequence>
    <xsd:element name="nvPicPr"  type="CT_PictureNonVisual"        minOccurs="1" maxOccurs="1"/>
    <xsd:element name="blipFill" type="a:CT_BlipFillProperties"    minOccurs="1" maxOccurs="1"/>
    <xsd:element name="spPr"     type="a:CT_ShapeProperties"       minOccurs="1" maxOccurs="1"/>
    <xsd:element name="style"    type="a:CT_ShapeStyle"            minOccurs="0" maxOccurs="1"/>
    <xsd:element name="extLst"   type="CT_ExtensionListModify"     minOccurs="0" maxOccurs="1"/>
  </xsd:sequence>
</xsd:complexType>
<xsd:complexType name="CT_PictureNonVisual">
  <xsd:sequence>
    <xsd:element name="cNvPr"    type="a:CT_NonVisualDrawingProps"        minOccurs="1" maxOccurs="1"/>
    <xsd:element name="cNvPicPr" type="a:CT_NonVisualPictureProperties"   minOccurs="1" maxOccurs="1"/>
    <xsd:element name="nvPr"     type="CT_ApplicationNonVisualDrawingProps" minOccurs="1" maxOccurs="1"/>
  </xsd:sequence>
</xsd:complexType>
```

**Key reuse:** `p:pic`'s `blipFill` is the very `a:CT_BlipFillProperties` that `mjx-dml::BlipFill`
already models — but inside `p:pic` the element is **`p:blipFill`** (PML namespace, DML type), so build
it with the `mjx-pptx::build` prefixed builders (the crate already constructs whole `p:sp` subtrees
itself; see `build_shape`/`build_sp_pr` in `presentation.rs`). `BlipFill::from_xml` reads it back fine —
the fidelity wrapper is name-agnostic.

Well-known strings (not in the XSD; from ECMA-376 Part 1 / OPC):
- image relationship type: `http://schemas.openxmlformats.org/officeDocument/2006/relationships/image`
- content types: `image/png`, `image/jpeg`, `image/gif`, `image/bmp`, `image/tiff`, `image/x-emf`,
  `image/x-wmf`, `image/svg+xml`.

## Roadmap — 3 atomic PRs

- **I1 — image parts (foundation).** An `ImageFormat` enum (magic-byte sniffing → `content_type()` /
  `extension()`); `REL_IMAGE` + image content-type constants; `Presentation::add_image(slide_idx,
  bytes) -> Result<String /* rel id */, PptxError>` = sniff → allocate `/ppt/media/imageN.ext` (scan
  existing media for max N; **dedupe identical bytes** so re-adding one image doesn't bloat the
  package) → `insert_part` → `add_relationship` from the slide with a fresh `rIdN`. This alone makes
  `set_shape_fill(.., &FillSpec::Blip { .. })` work **end-to-end** — a shape filled with a real
  picture. Office-open canary.
  *Placement of `ImageFormat`:* `mjx-pptx` for now (keeps the PR focused); promote to a shared crate
  when `mjx-docx`/`mjx-xlsx` need it.
- **I2 — `p:pic` picture shapes.** `Presentation::add_picture(slide_idx, bytes, ShapeBounds) ->
  Result<usize, PptxError>` building a whole `p:pic` (`nvPicPr`/`p:blipFill` with `a:blip r:embed` +
  `a:stretch`/`a:fillRect`/`spPr` with `a:xfrm` + `prstGeom prst="rect"`), appended to `p:spTree`;
  plus a parallel picture index space (`picture_count`, `picture_image_rel_id`). Office-open canary.
- **I3 (optional) — read/replace.** `picture_image_bytes` (resolve `r:embed` → part → bytes) and
  `set_picture_image` / replacing an existing image part.

## Guardrails

Standard project rules apply (`CLAUDE.md`, `PHASE2_HANDOFF.md` §3): fidelity first (adding an image
must leave every untouched part byte-identical — assert it); one interner per part, split-borrow for
edits; `r:embed` is a **prefixed** attribute whose namespace the reader leaves unresolved (resolve via
`nav::namespace_prefix`, or match by local name as `BlipFill::image_rel_id` already does); no
`unwrap`/`panic`/`expect` on untrusted input — a malformed/unknown image should be a typed error, not a
panic; pure-Rust only (**no image-decoding crate** — we only sniff magic bytes and store bytes
verbatim, we never re-encode); never stage `References/`.

**Commits:** split by concern — constants/`ImageFormat`, the package plumbing, the public API, and docs
each land as their own commit (see the git-workflow memory).

## First actions

1. `git switch main && git pull --ff-only` (ensure effects PR #40 has merged).
2. Read this + `docs/DRAWINGML_FILL_HANDOFF.md` (blipFill) + `mjx-pptx/src/{presentation.rs,build.rs}`
   (`build_shape`/`build_sp_pr` — the subtree-construction pattern I2 copies).
3. Discussion-first, then implement I1.
