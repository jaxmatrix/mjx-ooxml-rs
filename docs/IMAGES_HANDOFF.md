# Handoff — PowerPoint images / pictures (`p:pic` + media parts) — COMPLETE

A record of the **images** workstream, kept for the decisions it settled. Read after
`docs/PHASE2_HANDOFF.md` (§3 guardrails) and `docs/DRAWINGML_FILL_HANDOFF.md` (the `blipFill` model).

**Status: done — I1 (image parts), I2 (`p:pic` picture shapes), I3 (read/replace) have all shipped.**
The next Phase 3 workstream is **layout/master** modeling.

## What shipped

```rust
let picture = deck.add_picture(0, &png_bytes, ShapeBounds::from_inches(1.0, 1.0, 3.0, 2.0))?;
deck.set_shape_outline(0, picture, &border)?;      // a picture takes the whole spPr surface
deck.set_picture_image(0, picture, &other_bytes)?; // …and its image can be swapped
```
Also `add_image` (parts only, for `FillSpec::Blip`), `picture_image_rel_id`, `picture_image_bytes`,
`shape_kind`. Tests: `crates/mjx-pptx/tests/{images,pictures}.rs` + two LibreOffice canaries, both
eyeballed as rendered PNGs.

Decisions I2/I3 settled:
- **One shape index space over every kind.** `slide::shapes` yields all six `EG_ShapeElements`
  (`sp | pic | grpSp | graphicFrame | cxnSp | contentPart`) in document order, and the public
  `ShapeKind` says which an index is. A picture is simply shape *n*, so fill/outline/effects/geometry
  work on it with no duplicated API family. A group counts as one shape; its members are not
  addressable. **This changed what an existing `shape_idx` means** on any deck containing more than
  `p:sp`.
- **`p:blipFill` is built in `mjx-pptx`**, not by `mjx_dml::BlipFill::new` (that emits `a:blipFill`);
  reading it back does reuse `BlipFill`, whose fidelity wrapper is name-agnostic.
- **The `r` prefix is declared on a built subtree when the part does not bind it**
  (`build::relationship_prefix_declaration`) — attribute namespaces are never resolved, so an
  `r:embed` is meaningless without a binding. `mjx-dml` keeps putting that on the caller.
- **A replaced image's part stays in the package.** Another shape may reference it; sweeping
  unreferenced parts is a package-wide graph operation for a later task.
- Picture bounds are always the caller's — nothing decodes an image, so its natural size is unknown.

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

## Why this workstream existed

With fill, outline (`a:ln`), and effects (`a:effectLst`) all modeled explicitly **and** effectively,
the `spPr` visual trilogy was done, and images were the bigger of Phase 3's two remaining gaps: you
could not put a picture on a slide at all. The fill workstream deliberately deferred it ("adding a
`blipFill` image needs an image part + a relationship — its own step").

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

## Roadmap — all three PRs landed

- **I1 — image parts (foundation).** ✅ *done* — see "What I1 shipped" above.
- **I2 — `p:pic` picture shapes.** ✅ *done* — `add_picture`, on the unified shape index space rather
  than the parallel `picture_*` space originally sketched here.
- **I3 — read/replace.** ✅ *done* — `picture_image_rel_id` / `picture_image_bytes` /
  `set_picture_image`.

## Known follow-ups (not blockers)

- **Unreferenced media parts are never swept.** Replacing an image leaves the old part in place. A
  package-wide "remove parts nothing relates to" pass belongs in `mjx-opc`.
- **Linked images (`a:blip@r:link`) are not resolved** — `picture_image_rel_id` returns `None` for a
  picture that links rather than embeds.
- **Group members are not addressable.** A `p:grpSp` is one shape; reaching inside it needs a nested
  address (`ShapeKind::GroupShape` marks where).

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

## Where to look

`crates/mjx-pptx/src/presentation.rs` (`add_image`, `add_picture`, `build_picture`, the `picture_*`
readers), `crates/mjx-pptx/src/slide.rs` (`ShapeKind`, `shapes`, `nth_shape_mut`),
`crates/mjx-opc/src/media.rs` (`ImageFormat`), and the tests named above.
