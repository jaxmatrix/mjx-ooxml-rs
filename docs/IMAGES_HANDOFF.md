# Handoff — PowerPoint images / pictures (`p:pic` + media parts) — IN PROGRESS

A self-contained brief for the **images** workstream. Read after `docs/PHASE2_HANDOFF.md` (§3
guardrails) and `docs/DRAWINGML_FILL_HANDOFF.md` (the `blipFill` model).

**Status: I1 (image parts) is done — resume at I2 (`p:pic` picture shapes).** See "What I1 shipped"
below for the decisions it settled; the rest is the plan plus facts already verified against the repo
and the spec, so the next session does not re-derive them.

## What I1 shipped

`Presentation::add_image(slide_idx, bytes) -> Result<String /* rel id */, PptxError>` — sniff the
format, store the bytes verbatim as `/ppt/media/image{N}.{ext}`, register the content type, add the
slide → image relationship, return the id to hand to `FillSpec::Blip`. **A shape can now be filled
with a real picture end-to-end** (`crates/mjx-pptx/tests/images.rs`, plus a LibreOffice canary that
renders the image).

Decisions I1 settled — do not re-litigate them in I2/I3:
- **`ImageFormat` lives in `mjx-opc`** (`crates/mjx-opc/src/media.rs`), not `mjx-pptx`: its payload
  *is* an OPC content type, and `mjx-docx`/`mjx-xlsx` sit above `mjx-opc` so they inherit it. Magic
  bytes only — PNG/JPEG/GIF/BMP/TIFF/EMF/WMF/SVG — never a decode or re-encode.
- **Content types use a `Default` extension rule**, as Office writes them, not a per-part `Override`:
  `Package::set_content_type_default` (new in `mjx-opc`), called *before* `insert_part` so that call
  finds the type already resolved and adds no `Override`.
- **Identical bytes are stored once**: an existing media part with the same bytes is reused, and a
  slide that already relates to it gets its existing relationship id back untouched.
- `nav::relative_target` is the inverse of `nav::resolve_target` (it replaced the old
  `slide_rel_target`) — use it for any new relationship target.
- Media part numbering: `image{N}` is one past the largest existing image number, whatever the
  extension; relationship ids come from `next_rid_for(part)`, which starts at `rId1` for a part with
  no `.rels`.

## Why this is next

With fill, outline (`a:ln`), and effects (`a:effectLst`) all modeled explicitly **and** effectively,
the `spPr` visual trilogy is done. `PLAN.md` Phase 3 lists two remaining items — **images** and
**layout/master** — and images is the bigger user-visible gap: **you currently cannot put a picture on
a slide.** The fill workstream deliberately deferred it ("adding a `blipFill` image needs an image part
+ a relationship — its own step").

## Current state — verified, not assumed

- **Image *parts* are done (I1)** — `Presentation::add_image`, `mjx_opc::ImageFormat`,
  `Package::set_content_type_default`, `constants::REL_IMAGE`, `nav::relative_target`. What is still
  missing is the **`p:pic` shape** (I2) and reading an image back out (I3).
- **The OPC plumbing** (`crates/mjx-opc/src/package.rs`):
  - `Package::insert_part(&PartName, content_type: &str, bytes: Vec<u8>)` — works for **binary** parts
    and auto-registers a content-type `Override` iff the type isn't already resolved.
  - `Package::add_relationship(Option<&PartName>, Relationship)` — appends to the source's `.rels`, or
    synthesizes one when absent. Caller supplies `rel.id` (no de-collision — allocate it yourself).
  - `Package::part_names` / `part_bytes` / `content_type_of` for scanning + reading back.
- **`mjx-dml::BlipFill` already models the picture fill** — `BlipFill::new(interner, rel_id, mode)`,
  `image_rel_id()`, `image_link_id()`, `mode()`, plus interner-free `FillSpec::Blip { rel_id, mode }`
  and `BlipFillMode::{Tile, Stretch, None}`. **No `mjx-dml` changes were needed for I1.**
- **`tests/fixtures/sample.pptx` has NO media parts** (11 parts total), so tests synthesize a tiny
  valid PNG in-test rather than committing a binary fixture — reuse the `TINY_PNG` const in
  `crates/mjx-pptx/tests/images.rs`.
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

- **I1 — image parts (foundation).** ✅ *done* — see "What I1 shipped" above.
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

**Commits:** split by concern — the shared plumbing, the package plumbing, the public API, tests, and
docs each land as their own commit (see the git-workflow memory).

## First actions (for I2)

1. `git switch main && git pull --ff-only` (ensure the I1 PR has merged).
2. Read this + `docs/DRAWINGML_FILL_HANDOFF.md` (blipFill) + `mjx-pptx/src/{presentation.rs,build.rs}`
   (`build_shape`/`build_sp_pr` — the subtree-construction pattern I2 copies) + `add_image` and
   `crates/mjx-pptx/tests/images.rs` (what I1 left you).
3. Discussion-first, then implement I2 — `add_picture` should call `add_image` for the part/rel and
   then build the `p:pic` subtree, so the two layers stay separable.
