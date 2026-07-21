# Handoff — the shape transform (`a:xfrm`) — COMPLETE

Where a shape sits, how big it is, and which way up. Read after `docs/PHASE2_HANDOFF.md` (§3
guardrails); `docs/TEXT_FORMATTING_HANDOFF.md` is the immediately preceding workstream.

**Status: COMPLETE — X1–X3 shipped, `0.0.12`, 607 tests green.**

**➡ NEXT: tables (`a:tbl` inside a `p:graphicFrame`), then speaker notes.** Those two are the
remaining v0.1 "PowerPoint complete" scope. The fixture work below already hands tables a starting
point.

## Why this workstream exists

After Phase 3 a caller could create a shape and never move it. `ShapeBounds` existed, but was only
ever *written*, at creation, by `add_shape` / `add_text_box` / `add_picture`. There was no reader, no
setter, no rotation, no flips — and nothing could report where a placeholder actually renders,
because that answer lives on the layout or the master. `a:xfrm` was also the last member of the
`p:spPr` family without the explicit + effective pair that fill, outline and effects all have.

## What shipped

```rust
// Explicit — what the shape declares.
deck.shape_bounds(slide, shape)?;                       // Option<ShapeBounds>
deck.set_shape_bounds(slide, shape, bounds)?;           // move / resize anything
deck.shape_transform(Surface::Layout(1), shape)?;       // Option<Transform2D>
deck.set_shape_transform(slide, shape, &Transform2D {   // rotation, flips, group child space
    rotation: Some(Angle::from_degrees(30.0)),
    flip_horizontal: Some(true),
    ..Transform2D::default()
})?;

// Effective — where it actually renders, layout and master consulted.
deck.effective_shape_bounds(slide, shape)?;
deck.effective_shape_transform(slide, shape)?;
```

Per PR:

- **X1 (#56, `0.0.10`)** — `Transform2D`, `Position`, `Size` in
  `crates/mjx-dml/src/geometry/transform.rs`, with `read`, `apply`, `is_empty`, `empty_element`. The
  measure attribute helpers (`attr_emu`, `push_angle`, …) moved from `effect.rs` to `build.rs`.
- **X2 (#57, `0.0.11`)** — the `mjx-pptx` surface above, the per-kind locator in `slide.rs`,
  `ShapeBounds::from_transform` / `to_transform`, and `PptxError::ShapeCannotBePositioned`.
- **X3 (`0.0.12`)** — `effective_shape_transform` / `effective_shape_bounds`, and the extraction of
  the candidate walk into `placeholder_candidates` + `candidate_shape`.

Tests: `crates/mjx-dml/tests/transform_model.rs`, `crates/mjx-pptx/tests/transform.rs`,
`crates/mjx-pptx/tests/transform_inheritance.rs`, the locator's fragment tests inside `slide.rs`, and
an `office_open.rs` canary that moves, resizes, rotates and mirrors shapes through LibreOffice.

## Decisions settled — do not re-litigate

1. **Absent is not zero.** Every field of `Transform2D` is `Option`, and `None` means the file does
   not state it. Collapsing that to zero would make a shape saying *I am at the origin*
   indistinguishable from one saying *ask my layout*, and the whole effective walk turns on the
   difference.
2. **`apply` merges, it does not rebuild.** Fill, outline and effects rebuild their elements
   wholesale, which is right — they are self-contained. An `a:xfrm` is not: it carries a group's
   `a:chOff`/`a:chExt`, an `extLst`, and unknown attributes on the `a:off`. Rebuilding a group's
   transform to move it would discard the child space and drag every member with it. An unset field
   means *leave it alone*, never *clear it* — the same call text formatting made for `a:rPr`.
3. **The transform is not in the same place for every shape kind**, and all of that knowledge lives
   in `slide::shape_transform` / `shape_transform_slot_mut`. See the table below.
4. **Inheritance is all-or-nothing**, not field-wise. A shape cannot take its position from the
   layout and its size from the master, so the first tier that states anything wins whole. A
   present-but-empty `<a:xfrm/>` states nothing and the walk steps past it.
5. **The pptx surface returns the `mjx-dml` type directly** (`Transform2D`), as `shape_fill` →
   `FillSpec` and `shape_geometry` → `ShapeGeometry` already do. `ShapeBounds` stays as the
   friendly four-number pair a caller places shapes with; it is not a competing model.
6. **Schema order is validity.** A created `a:xfrm` is inserted at its rank — first in `p:spPr` and
   `p:grpSpPr`, after `p:nvGraphicFramePr` in a `p:graphicFrame`.

## Verified schema — read from the XSDs, not assumed

| `ShapeKind` | transform | type |
| --- | --- | --- |
| `Shape`, `Picture`, `ConnectionShape` | `p:spPr > a:xfrm` | `CT_Transform2D` |
| `GroupShape` | `p:grpSpPr > a:xfrm` | `CT_GroupTransform2D` (adds `a:chOff`, `a:chExt`) |
| `GraphicFrame` | `p:xfrm` — **PresentationML** namespace, direct child, `minOccurs="1"` | `CT_Transform2D` |
| `ContentPart` | none (`CT_Rel`) | — |

Only the *wrapper* differs; the `a:off` / `a:ext` inside are DrawingML in every case, which is why
one `Transform2D` reads them all. `CT_Point2D`'s `x`/`y` and `CT_PositiveSize2D`'s `cx`/`cy` are all
`use="required"`, which is why a child carrying only one reads as `None` rather than half a point.

## Fixtures, as this workstream left them

- **`tests/fixtures/layouts.pptx` slide 2** gained a `p:grpSp` (whose `a:chOff`/`a:chExt` deliberately
  differ from its `a:off`/`a:ext`, so a move test can prove the child space survived rather than
  coincidentally matching) and a `p:graphicFrame` holding a **real one-cell table**. Appended, so
  shape indices 0 and 1 kept their meaning.
- **`slideLayout2`'s `title` placeholder no longer declares an `a:xfrm`**, so it defers to the
  master. This is what makes the master tier reachable: a slide built from that layout resolves its
  title at the master and its body at the layout.
- **`sample.pptx` is untouched** — its `ctrTitle` has an empty `p:spPr` and its layout has no shapes,
  which makes it the "nothing anywhere answers" case.

## Known follow-ups (not blockers)

- **Group descent.** Group members are not addressable at all, so bounds are always in the *parent
  tree's* coordinate space. Computing an absolute rectangle for a shape inside a `p:grpSp` means
  mapping the child space onto the group's extent, and needs an addressing scheme for members
  (the shape index space would become a path). This is the largest single gap the transform leaves.
- **Resizing a group rescales its members**, because rendering maps `a:chOff`/`a:chExt` onto the
  group's extent. That is PowerPoint's behaviour and falls out of changing only `a:ext`; it is
  documented on `set_shape_bounds` rather than special-cased.
- **`a:custGeom`, `a:scene3d`, `a:sp3d`** stay opaque. This workstream touched only `a:xfrm`.
- **A transform naming only a rotation** resolves to `effective_shape_bounds == None`. That is the
  all-or-nothing rule applied honestly; it is pathological in real files.
- **`p:graphicFrame`'s `p:xfrm` is now readable and writable**, which is the natural entry point for
  the tables workstream — a table is positioned by its frame, not by anything inside `a:tbl`.

## Guardrails

Standard project rules (`CLAUDE.md`, `PHASE2_HANDOFF.md` §3). The ones this workstream kept meeting:

- **Fidelity first.** Every write test pairs its structural assertion with "every other part is
  byte-identical"; every read test asserts reading dirtied nothing. Routing `build_sp_pr` through
  `Transform2D::apply` was verified byte-identical rather than assumed.
- **Check the schema before assuming a type.** The graphic frame's `p:xfrm` would have been wrong on
  all three counts (namespace, position, optionality) if it had been guessed from the other kinds.
- **One interner per part**, split-borrow for edits (`let RawDocument { interner, root, .. } = doc`).
- **`placeholder_candidates` is the one candidate walk** — `effective_shape_fill`, `_outline`,
  `_effects` and `_transform` all use it. Do not add a fifth copy; extend the helper.
- Every PR bumps the patch version and adds a `CHANGELOG.md` entry as its last commit; commits split
  by concern.

## Where to look

`crates/mjx-dml/src/geometry/transform.rs` (the model), `crates/mjx-dml/src/build.rs` (the shared
measure attribute helpers), `crates/mjx-pptx/src/slide.rs` (`shape_transform`,
`shape_transform_slot_mut` — the per-kind knowledge), `crates/mjx-pptx/src/geometry.rs`
(`ShapeBounds` ↔ `Transform2D`), and `crates/mjx-pptx/src/presentation.rs` (the surface, the
`effective_*` family, and `placeholder_candidates` / `candidate_shape`).
