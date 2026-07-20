# Handoff — slide layouts and masters — COMPLETE

The last Phase 3 workstream, and with it the PowerPoint slice. Read after `docs/PHASE2_HANDOFF.md`
(§3 guardrails); the images workstream (`docs/IMAGES_HANDOFF.md`) is the immediately preceding one and
settled the shape-addressing model this builds on.

**Status: done — L1 (types), L2 (inventory), L3 (`Surface` addressing) and L4
(`add_slide_from_layout`) have all shipped.**

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

## Known follow-ups (not blockers)

- **Every placeholder is cloned, including `dt`/`ftr`/`sldNum`.** Those three render from the layout
  when a slide does not declare them, so cloned copies show as empty boxes instead. PowerPoint skips
  them; a `skip_kinds` argument or a `remove_shape` would both close this.
- **`add_shape` / `add_text_box` build a shape whose paragraph has no run**, so `set_shape_text`
  cannot fill an added autoshape (`RunIndexOutOfRange`). `build_run` now exists — giving those two the
  same empty run as a placeholder would fix it.
- **There is no `remove_shape` or `remove_slide`.** Everything so far adds.
- **Master `p:txStyles` is unread** — effective *text* formatting (run → paragraph → placeholder →
  layout → master → theme font scheme) is the natural next PowerPoint workstream, larger than L1–L4.

## Guardrails

Standard project rules (`CLAUDE.md`): generated names sourced from the prose, never guessed, with the
`ST_*` symbol and wire token in the docs; generated output committed (`cargo run -p xtask -- codegen`
needs local `References/`); `References/` never staged. Reading must dirty nothing — assert
byte-identity after a read-only pass. No `unwrap`/`panic`/`expect` on untrusted input: a missing list
or an out-of-range index is `None` or a typed error. Commits split by concern (xtask / generated /
fixture / crate / docs).
