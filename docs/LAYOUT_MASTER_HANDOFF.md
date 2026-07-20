# Handoff — slide layouts and masters — COMPLETE

The last Phase 3 workstream, and with it the PowerPoint slice. Read after `docs/PHASE2_HANDOFF.md`
(§3 guardrails); the images workstream (`docs/IMAGES_HANDOFF.md`) is the immediately preceding one and
settled the shape-addressing model this builds on.

**Status: done — L1 (types), L2 (inventory), L3 (`Surface` addressing) and L4
(`add_slide_from_layout`) have all shipped, and the follow-up round after them
(`remove_shape`, `remove_slide`, and the two construction fixes) is in too.**

## Building a deck, end to end

```rust
let mut deck = Presentation::open(&bytes)?;
let layout = (0..deck.layout_count())
    .find(|&i| deck.layout_kind(i).unwrap() == SlideLayoutKind::TitleAndObject)
    .unwrap();
let slide = deck.add_slide_from_layout(layout)?;   // arrives with the layout's placeholders
deck.set_shape_text(slide, 0, 0, "Quarterly results")?;
deck.set_shape_text(slide, 1, 0, "Revenue is up")?;
```

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

## What L3 shipped

```rust
deck.shape_fill(0, 2)?;                            // a slide, as before — a bare usize is Surface::Slide
deck.set_shape_fill(Surface::Layout(1), 0, &red)?; // …and every slide on that layout inherits it
deck.shape_placeholder(Surface::Layout(1), 0)?;    // what that layout offers a slide to fill
deck.theme(Surface::Master(0))?;
```

- **`Surface { Slide | Layout | Master }`** addresses every shape call. `From<usize>` means
  `Surface::Slide`, so no existing call site changed. `Display` renders `layout 1`, which
  `ShapeIndexOutOfRange` now carries instead of a slide index that would have been a lie.
- **`inheritance_chain(surface)`** is the single walk everything resolves along — a slide through its
  layout then master, a layout through its master, a master alone. The three `effective_shape_*`
  resolvers, `theme` and `color_map` all use it; none of them hand-rolls the hops any more.
- **`slide_theme` / `slide_color_map` were renamed to `theme` / `color_map`**, since the old names
  contradict a layout argument.
- **`PlaceholderInfo`** (kind, slot index, size, orientation, name) is the public reading of `p:ph`;
  the internal `Placeholder { kind, idx }` remains the projection inheritance matches on.

## Settled decisions — do not re-litigate

- **Layout indices are flat across masters**, in (master order, `p:sldLayoutIdLst` order);
  `layout_master(idx)` recovers the owner. A layout no master lists is not enumerated — layouts are
  reached through their master, as PowerPoint reaches them.
- **`p:sldSz@type` and `p:sldLayout@type` fall back to `Custom`** when absent or unrecognized, per the
  XSD defaults, rather than erroring: an unknown token is a forward-compatible file, not a broken one.
- **No `layout_shape_*` accessors.** One `Surface`-addressed API family, never a parallel one — the
  same call made for pictures in `p:pic`.
- **A bare `usize` stays a slide.** The ergonomic default is the common case; `impl Into<Surface>` is
  what keeps that true without a second set of methods.

## Remaining roadmap

- **L4 — `add_slide_from_layout`.** ✅ *done* — clones one `p:sp` per slot, with the layout's name and
  `p:ph`, an empty `p:spPr` (position, size and appearance keep inheriting) and a text body holding
  one **empty run**, which is what lets `set_shape_text` fill it: that method replaces an existing
  run, so a body with none could not be filled at all.
- **Later (not this workstream):** master `p:txStyles` feeding *effective text formatting* (run →
  paragraph → placeholder → layout → master → theme font scheme). That is its own workstream, larger
  than all of L1–L4.

## Known follow-ups — all closed except the last

- ~~**Every placeholder is cloned, including `dt`/`ftr`/`sldNum`.**~~ ✅ *done* —
  `add_slide_from_layout` skips those three (`is_layout_rendered_slot`), as PowerPoint does: they
  render *from the layout* precisely for slides that do not declare them, so a clone suppressed the
  layout's rendering and left an empty box. `tests/fixtures/layouts.pptx` layout 1 now carries the
  trio so the rule is testable.
- ~~**`add_shape` / `add_text_box` build a shape whose paragraph has no run.**~~ ✅ *done* —
  `build_paragraph` always emits exactly one run, an empty line included, so an added shape can be
  filled by `set_shape_text` immediately and every line is addressable as a run of its own.
- ~~**There is no `remove_shape` or `remove_slide`.**~~ ✅ *done* — see below.
- **Master `p:txStyles` is unread** — effective *text* formatting (run → paragraph → placeholder →
  layout → master → theme font scheme) is the natural next PowerPoint workstream, larger than L1–L4.
  **This is the only PowerPoint item still open.**

## What removal shipped

```rust
deck.remove_shape(slide, 2)?;                 // any Surface — a slide, a layout, a master
deck.remove_shape(Surface::Layout(1), 3)?;
deck.remove_slide(0)?;                        // later slides shift down one index
```

- **`remove_shape`** removes the `shape_idx`-th shape in the one shape index space (so a picture or a
  group goes exactly as an autoshape does) plus the whitespace that indented it, and **leaves
  relationships and parts alone** — an unused relationship is valid OOXML, `add_image` de-duplicates
  by content, and a sibling shape may share the image.
- **`remove_slide`** unwires in the reverse of the order `insert_slide_part` wired: the `p:sldId`
  (found via its `r:id` — attribute namespaces are never resolved), the presentation relationship,
  then the part through the new **`Package::remove_part_cascading`**, which also removes every part no
  remaining relationship resolves to. That takes the notes slide (which holds a relationship *back* to
  the slide, so leaving it behind leaves a dangling reference) and unshared media, and spares media
  the rest of the deck still shows. Cycles terminate; unresolvable targets are skipped.
- **Part-name algebra moved down to `mjx-opc`**: `PartName::{resolve, resolve_from_root,
  relative_target}` — pure OPC arithmetic that `mjx-docx`/`mjx-xlsx` will need and could not reach in
  `mjx-pptx` without a sideways dependency. `pptx::nav` keeps thin wrappers that restate the failure
  as `PptxError::{TargetResolution, ExternalTarget}`.
- **Indices**: removing a slide shifts later *slide* indices down; layout and master indices are
  unaffected. A freed `slideN.xml` name is never recycled — `next_slide_part` numbers past every
  existing part.

## Guardrails

Standard project rules (`CLAUDE.md`): generated names sourced from the prose, never guessed, with the
`ST_*` symbol and wire token in the docs; generated output committed (`cargo run -p xtask -- codegen`
needs local `References/`); `References/` never staged. Reading must dirty nothing — assert
byte-identity after a read-only pass. No `unwrap`/`panic`/`expect` on untrusted input: a missing list
or an out-of-range index is `None` or a typed error. Commits split by concern (xtask / generated /
fixture / crate / docs).
