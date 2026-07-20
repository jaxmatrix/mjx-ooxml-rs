# Handoff — slide layouts and masters — IN PROGRESS

The last Phase 3 workstream. Read after `docs/PHASE2_HANDOFF.md` (§3 guardrails); the images
workstream (`docs/IMAGES_HANDOFF.md`) is the immediately preceding one and settled the shape-addressing
model this builds on.

**Status: L1 (PresentationML types) and L2 (inventory) are done — resume at L3 (`Surface`
addressing).**

## What L1 + L2 shipped

```rust
let mut deck = Presentation::open(&bytes)?;
deck.layout_count();                 // every layout, master by master
deck.layout_name(1)?;                // Some("Title and Content")
deck.layout_kind(1)?;                // SlideLayoutKind::TitleAndObject
deck.layout_master(1);               // Some(0)
deck.slide_layout(0)?;               // Some(1) — the layout this slide is built on
deck.slide_size()?;                  // SlideSize { width_emu, height_emu, kind }
deck.master_count(); deck.master_name(0)?; deck.master_part(0);
```

- **`mjx_ooxml_types::presentationml`** is generated from `pml.xsd` (allowlist in
  `xtask/src/codegen/mod.rs`): `PlaceholderType`, `PlaceholderSize`, `SlideLayoutKind`,
  `SlideSizeKind`, `Orientation`. Names come from the **ECMA-376 Part 1 enumeration tables**, which
  give every value an official title — `obj` in a layout is `TitleAndObject`, *not* "object" (that is
  `objOnly`/`ObjectOnly`). Extending to more PML types means growing the allowlist and the `spec.rs`
  tables, nothing else.
- **`tests/fixtures/layouts.pptx`** is hand-authored (one master, three layouts, two slides on
  *different* layouts) because every other fixture has a single empty `blank` layout. Its structure is
  tabulated at the top of `crates/mjx-pptx/tests/layouts.rs`, and `office_open.rs` proves LibreOffice
  opens it.
- **`referenced_parts`** walks any PresentationML `r:id` list (`p:sldIdLst`, `p:sldMasterIdLst`,
  `p:sldLayoutIdLst`) — use it for any new list rather than re-inlining the resolution.
- **`slide::Placeholder`** now carries a typed `kind: PlaceholderType`; `is_title_family()` replaces
  the old string match. This is what L3's public placeholder metadata will be built from.

## Settled decisions — do not re-litigate

- **Layout indices are flat across masters**, in (master order, `p:sldLayoutIdLst` order);
  `layout_master(idx)` recovers the owner. A layout no master lists is not enumerated — layouts are
  reached through their master, as PowerPoint reaches them.
- **`p:sldSz@type` and `p:sldLayout@type` fall back to `Custom`** when absent or unrecognized, per the
  XSD defaults, rather than erroring: an unknown token is a forward-compatible file, not a broken one.
- **No `layout_shape_*` accessors.** Reading a layout's shapes waits for L3's one addressing model
  (below) rather than growing a parallel API family — the same call made for pictures in `p:pic`.

## Remaining roadmap

- **L3 — `Surface` addressing.** Every shape API takes `impl Into<Surface>` with
  `impl From<usize> for Surface = Surface::Slide(n)`, so `deck.shape_text(0, 2)` keeps compiling while
  `deck.shape_text(Surface::Layout(1), 0)` becomes possible. Covers the readers, the setters, and the
  `effective_shape_*` chains (a layout shape resolves layout → master; a master shape resolves master
  only). Exposes `PlaceholderInfo { kind, index, size, orientation, name }` for any surface's shapes —
  which is how a caller learns what a layout offers to fill.
- **L4 — `add_slide_from_layout(layout_idx)`.** Create a slide bound to a chosen layout and clone that
  layout's placeholder shapes into it (`p:ph` type/idx preserved, text emptied, no explicit `spPr`), so
  the new slide is immediately fillable with `set_shape_text`. Today `add_slide` blindly reuses slide
  0's layout and produces an empty shape tree. Office-open canary + a rendered-PNG check.
- **Later (not this workstream):** master `p:txStyles` feeding *effective text formatting* (run →
  paragraph → placeholder → layout → master → theme font scheme). That is its own workstream, larger
  than all of L1–L4.

## Guardrails

Standard project rules (`CLAUDE.md`): generated names sourced from the prose, never guessed, with the
`ST_*` symbol and wire token in the docs; generated output committed (`cargo run -p xtask -- codegen`
needs local `References/`); `References/` never staged. Reading must dirty nothing — assert
byte-identity after a read-only pass. No `unwrap`/`panic`/`expect` on untrusted input: a missing list
or an out-of-range index is `None` or a typed error. Commits split by concern (xtask / generated /
fixture / crate / docs).
