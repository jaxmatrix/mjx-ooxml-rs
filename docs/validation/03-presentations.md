# Presentations — the checks

Eight areas, `V-PPTX-01` … `V-PPTX-08`. `docs/validation/00-method.md` states the id scheme, the
risk levels and the result convention; `docs/validation/01-index.md` binds each area to its
artefact; `docs/validation/02-risk-order.md` says which of these to do first and why.

**Nothing on this page is marked.** Every `Result:` line ships unfilled, and only a person who has
opened the file in real Microsoft PowerPoint may fill one in.

## Where each artefact comes from

A check names its artefact in a code span, and every one of them is a file this repository produces
by a command. There are three producers, and `xtask/tests/validation_calls.rs` holds every named
span to one of them:

| Producer | How to make it | Example |
|---|---|---|
| The validation generator | `cargo run -p xtask -- validation-artefacts` | `v-pptx-01-authored.pptx` |
| An example | `cargo run -p mjx-pptx --example <name> -- out.pptx` | `crates/mjx-pptx/examples/blank_deck.rs` |
| A committed fixture | already in the tree | `tests/fixtures/text_levels.pptx` |

A check with no artefact at all says **blocked** and names what unblocks it. Those are not oversights
and they are not padding: each is a question that needs a file this library cannot author, which is
what `MJXOFF-130` exists to supply.

## Coverage, against the gaps page

`crates/mjx-pptx/docs/guide/fidelity_and_gaps.md` keeps two lists apart on purpose, and only one of
them is what this pass is for. This table says, for every area, which list it touches — so **a
documented gap is never mistaken for a validation failure**.

| Area | Risk | The gaps page says | What that means here |
|---|---|---|---|
| `V-PPTX-01` | high | *Built, not yet verified* — **the 0.0.58 text-inheritance change** (owners `MJX-211` R1, `MJX-208`) | The pass's first job. `V-PPTX-01.1` |
| `V-PPTX-01` | high | *Non-goal* — **a font slot the theme does not define keeps its reference** | Interrogated, not reported: `V-PPTX-01.8` asks whether the documentation is right |
| `V-PPTX-02` | medium | *Built, not yet verified* — **`comp` / `gray` / `gamma` / `invGamma` colour transforms** (owner `MJX-211` R3) | `V-PPTX-02.4`. It was **blocked** — no facade call could author a colour transform at all — until `MJXOFF-219`; the artefact now carries two rows of swatches and the entry is the pass's to answer |
| `V-PPTX-02` | medium | *Non-goal* — **InkML strokes**, **an ActiveX control's `ax:ocxPr`**, **a SmartArt layout is not run**, **VML geometry is preserved, not evaluated** | Non-goals. The legacy checks look at what is *preserved and referenced*, never at an evaluated stroke, property bag, layout or path |
| `V-PPTX-03` | high | *Non-goal* — **`extLst` is never modelled** | Non-goal. `V-PPTX-03.5` checks the extension survives and stays where the sequence puts it, which is the whole claim |
| `V-PPTX-04` | medium | *Non-goal* — **chart colour and style parts are preserved, not modelled**. The workbook non-goal beside it is **retired**: MJXOFF-208 made a data edit *patch* the embedded workbook | R4 is now the claim that patching leaves the rest of a producer's workbook alone |
| `V-PPTX-05` | low | neither list | Plain modelled markup |
| `V-PPTX-06` | low | neither list | Plain modelled markup |
| `V-PPTX-07` | medium | *Non-goal* — **a transform naming a rotation but not both `a:off` and `a:ext` answers `None`** | R7, and the one place this pass interrogates a non-goal on purpose: `V-PPTX-07.6` |
| `V-PPTX-08` | medium | the workbook non-goal is **retired** (MJXOFF-208) | As `V-PPTX-04`; `V-PPTX-08.10` is the detached-workbook half of it |
| every area | — | *Built, not yet verified* — **every fixture is hand-crafted** | R2. Retired for all of them at once by `MJXOFF-130`, and this pass is what feeds it |

## `V-PPTX-01` · `text-inheritance` — text, paragraph and run properties, and what they inherit

Risk **high**. Shipped by `MJXOFF-55` (`Presentation::blank`), `MJXOFF-59` (the shape list style and
the merge-aware selections) and `MJX-22` (the 0.0.58 ladder audit).

#### V-PPTX-01.1 — the tier-5 text style on a shape that is not a placeholder

- **Risk** high — **R1**, and the first thing in the whole pass.
- **Shipped by** `MJX-22` at 0.0.58; the gaps page names `MJX-211` R1 and `MJX-208` as its owners.
- **Artefact** `v-pptx-01-authored.pptx`
- **Object** the third text box, reading *"Nothing stated: this run resolves from the layout and the master"*. It is a text box (`p:cNvSpPr@txBox="1"`), not a placeholder, and it states no size, no weight and no colour.
- **Action** click into it, then read Home → Font size and Home → Font colour.
- **Expect** ECMA-376 §19.3.1.35 splits tier 5 by kind — `p:bodyStyle` for a text box, `p:otherStyle` for any other non-placeholder shape — so the size the master's `p:bodyStyle` level 1 states is what should render. **This is a design question, not a settled expectation:** real PowerPoint is believed to match the *previous* behaviour, in which tier 5 was gated on `p:ph` and a text box took a hard default. **Record which happens.** If PowerPoint renders the hard default, the decision that follows is the user's — the change is isolated in one revertible commit, and the choice is between the prose and the renderer.
  Calls: `Deck::effective_run_properties` · `Deck.effective_run_properties` · `Deck.effectiveRunProperties`
  Result: — · — · — · —

#### V-PPTX-01.2 — a run that states its own size, weight and colour

- **Risk** low.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-01-authored.pptx`
- **Object** the first text box, reading *"Stated on the run: 24pt bold, accent 1"*. Its `a:rPr` is `sz="2400" b="true"` over an `a:schemeClr val="accent1"`.
- **Action** select the text and read Home → Font size, Bold and Font colour → More Colours → Custom.
- **Expect** **24 pt**, bold, and the hex **`4472C4`** — the blank deck's theme states `accent1` as `4472C4`, so the eyedropper and the theme must agree.
  Calls: `Deck::set_shape_run_properties` · `Deck.set_shape_run_properties` · `Deck.setShapeRunProperties`
  Result: — · — · — · —

#### V-PPTX-01.3 — a paragraph that states its own alignment and indents

- **Risk** low.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-01-authored.pptx`
- **Object** the second text box. Its `a:pPr` is `algn="ctr" marL="228600" indent="-228600"` — centred, with an **18 pt** left margin and an **18 pt** hanging first-line indent. Note that the *text inside it* reads "centred, 1.5 line spacing" and no line spacing is written; the markup is what this check is about.
- **Action** select the paragraph and read Home → Paragraph → Alignment, Indentation → Before text and Special.
- **Expect** Centered; Before text **0.25"** (18 pt); Special = **Hanging**, By **0.25"**.
  Calls: `Deck::set_paragraph_properties` · `Deck.set_paragraph_properties` · `Deck.setParagraphProperties`
  Result: — · — · — · —

#### V-PPTX-01.4 — the master's `p:txStyles`, with nothing said on the slide

- **Risk** high.
- **Shipped by** `MJXOFF-55`.
- **Artefact** `crates/mjx-pptx/examples/blank_deck.rs` — run `cargo run -p mjx-pptx --example blank_deck -- out.pptx` and open what it writes.
- **Object** slide 1's title and body placeholders. Neither states a size on the slide; both take one from `p:txStyles` in the master.
- **Action** open the file: it must raise **no repair prompt**. It has **two slides**, and the layout gallery shows **one layout, named *Title and Text***. Click into the title, then into the body, and read Home → Font size for each. Then look at the body's five indent levels.
- **Expect** title **44 pt**, body **28 pt**, neither stated on the slide. The body's bullets render as **`•`** across all **five** indent levels.
  Calls: `Deck::add_slide_from_layout` · `Deck.add_slide_from_layout` · `Deck.addSlideFromLayout`
  Calls: `Deck::effective_run_properties` · `Deck.effective_run_properties` · `Deck.effectiveRunProperties`
  Result: — · — · — · —

#### V-PPTX-01.5 — the blank deck's slide size and theme

- **Risk** low.
- **Shipped by** `MJXOFF-55`, reached through the facade by `MJXOFF-61`.
- **Artefact** `v-pptx-01-authored.pptx`
- **Object** `p:sldSz` and `ppt/theme/theme1.xml`.
- **Action** Design → Slide Size → Custom Slide Size; then Design → Variants → Fonts, and Home → Font colour → More Colours on any accent-1 run.
- **Expect** **Widescreen**, **13.333 × 7.5 in**. The Office palette, with **`accent1` = `4472C4`**. Heading font **Calibri Light** (`+mj-lt`), body font **Calibri** (`+mn-lt`).
  Calls: `Deck::slide_size` · `Deck.slide_size` · `Deck.slideSize`
  Calls: `Deck::theme` · `Deck.theme` · `Deck.theme`
  Result: — · — · — · —

#### V-PPTX-01.6 — the shape's own `a:lstStyle`, and what happens when a level is cleared

- **Risk** high.
- **Shipped by** `MJXOFF-59` (the setter) over `MJX-22` (the ladder).
- **Artefact** `tests/fixtures/text_levels.pptx` — hand-authored so that every tier owns a facet no other tier touches: nine body levels with `a:lvl5pPr` deliberately absent, a layout overriding only two levels, a shape-level `a:lstStyle`, a footer, a text box and a plain autoshape.
- **Object** the shape carrying the `a:lstStyle`, at level 0.
- **Action** click into the level-0 paragraph and read its size; then read a paragraph that states its own `a:defRPr`; then clear the level in PowerPoint and read the size again.
- **Expect** the `a:lstStyle` level-0 setting renders at **44 pt** with no run and no paragraph stating it. A paragraph stating its own `a:defRPr` **still wins**. After clearing a level, the paragraph falls back to the master's **`p:bodyStyle`**, not to a hard default.
  Calls: `Deck::shape_list_style_level` · `Deck.shape_list_style_level` · `Deck.shapeListStyleLevel`
  Calls: `Deck::clear_shape_list_style_level` · `Deck.clear_shape_list_style_level` · `Deck.clearShapeListStyleLevel`
  Result: — · — · — · —

#### V-PPTX-01.7 — does PowerPoint honour `a:lstStyle` > `a:defPPr` at every level?

- **Risk** high — a **design question**, not a check with an expected result.
- **Shipped by** `MJX-22` at 0.0.58, from ECMA-376 §21.1.2.2.2.
- **Artefact** `tests/fixtures/text_levels.pptx`
- **Object** a level the list style does not define — `a:lvl5pPr` is absent on purpose — where the tier supplies only an `a:defPPr`.
- **Action** put a paragraph at that level and read what PowerPoint renders.
- **Expect** §21.1.2.2.2 defines `a:defPPr` as the properties applied *"when no other paragraph properties have been specified"*, and §21.1.2.4.13 keys the nine level elements strictly to `a:pPr@lvl` with **no** fallback to `a:lvl1pPr`. This is the rung real PowerPoint is least likely to agree with. **Record which happens.** If it disagrees, the decision that follows is the user's: this library follows the prose, and changing it means choosing the renderer over the specification in a resolver that three formats now share.
  Calls: `Deck::shape_list_style_default` · `Deck.shape_list_style_default` · `Deck.shapeListStyleDefault`
  Result: — · — · — · —

#### V-PPTX-01.8 — a font reference PowerPoint cannot answer (`+mj-sym`)

- **Risk** medium — an interrogation of a **documented non-goal**.
- **Shipped by** `MJXOFF-59`.
- **Artefact** none — **blocked** on `MJXOFF-130`. `CharacterPropertiesSpec` has no font setter, so no facade call can author a `+mj-sym` reference; this needs a file real PowerPoint wrote whose theme leaves the symbol slot undefined.
- **Object** a run whose typeface is `+mj-sym` in a theme with no `a:fontScheme` symbol slot.
- **Action** open the file, read the run's font in PowerPoint, then save from PowerPoint and read the markup back.
- **Expect** the gaps page's claim is that resolution replaces a reference only with a font the theme actually names, so `effective_run_properties` answers **`+mj-sym` verbatim**. The question is whether that is the right report: PowerPoint renders *something*, and the file must still say `+mj-sym` after a save. **Record which happens.**
  Calls: `Deck::effective_run_properties` · `Deck.effective_run_properties` · `Deck.effectiveRunProperties`
  Result: — · — · — · —

#### V-PPTX-01.9 — moving a placeholder on the master moves it on every slide

- **Risk** medium.
- **Shipped by** `MJXOFF-55`.
- **Artefact** `crates/mjx-pptx/examples/blank_deck.rs` — `cargo run -p mjx-pptx --example blank_deck -- out.pptx`.
- **Object** the master's title placeholder. The layout states **no** transform of its own, so inheritance is live.
- **Action** View → Slide Master, drag the title placeholder, return to Normal view.
- **Expect** the title moves on **every** slide built from that layout, because the layout states no transform to override the master's.
  Calls: `Deck::effective_shape_transform` · `Deck.effective_shape_transform` · `Deck.effectiveShapeTransform`
  Result: — · — · — · —

#### V-PPTX-01.10 — an `mc:AlternateContent` inside a shape, after an unrelated edit

- **Risk** medium.
- **Shipped by** `MJXOFF-85` (schema-order emission) over `MJXOFF-86` (subtree copy-on-write).
- **Artefact** `tests/fixtures/vml.pptx`
- **Object** a slide carrying `mc:AlternateContent` inside a shape.
- **Action** open it in PowerPoint after an unrelated edit has been made through this library.
- **Expect** no repair prompt; the `mc:AlternateContent` comes back **byte-identical and in the same position**. Markup Compatibility parts are deliberately *skipped* by the schema gate with a named reason, so this file is the only place that claim is tested at all.
  Calls: `Deck::set_shape_text_content` · `Deck.set_shape_text_content` · `Deck.setShapeTextContent`
  Result: — · — · — · —

## `V-PPTX-02` · `shape-appearance` — preset geometry, fills, outlines and effects

Risk **medium**. Shipped by `MJXOFF-59` and reorganised by `MJXOFF-60`. This area also carries the
checks for the **content that is not a DrawingML shape** (`MJXOFF-58`, `MJXOFF-148`) and for
**package integrity** (`MJXOFF-84`), because neither has an artefact of its own and both are
questions about what is on the slide.

#### V-PPTX-02.1 — a solid fill and a stroked outline

- **Risk** low.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-02-authored.pptx`
- **Object** the rectangle at the top left.
- **Action** Shape Format → Shape Fill → More Fill Colours → Custom, and Shape Outline → Weight.
- **Expect** fill **`1F3864`**; outline **3 pt**, drawn in the theme's **accent 2**.
  Calls: `Deck::set_shape_fill` · `Deck.set_shape_fill` · `Deck.setShapeFill`
  Calls: `Deck::set_shape_outline` · `Deck.set_shape_outline` · `Deck.setShapeOutline`
  Result: — · — · — · —

#### V-PPTX-02.2 — a two-stop linear gradient at 45°

- **Risk** medium.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-02-authored.pptx`
- **Object** the ellipse, second from the left.
- **Action** Shape Format → Shape Fill → Gradient → More Gradients, and read the two stops and the angle.
- **Expect** two stops — position 0 % **`FFF2CC`**, position 100 % **`C00000`** — at **45°**.
  Calls: `Deck::set_shape_fill` · `Deck.set_shape_fill` · `Deck.setShapeFill`
  Result: — · — · — · —

#### V-PPTX-02.3 — the outer shadow's four numbers, and nothing else applied

- **Risk** medium.
- **Shipped by** `MJXOFF-59`; the regression risk is `MJXOFF-60`'s reorganisation.
- **Artefact** `v-pptx-02-authored.pptx`
- **Object** the rounded rectangle at the top right. Its `a:outerShdw` is `blurRad="50800" dist="38100" dir="2700000"`, over an `a:glow rad="63500"` in accent 1.
- **Action** Shape Format → Shape Effects → Shadow → Shadow Options, and read Transparency, Size, Blur, Angle and Distance; then Glow → Glow Options.
- **Expect** the shadow casts at **45°**, **~3 pt** out, with **~4 pt** blur — and **no scale, no skew, no alignment and no rotate-with-shape** applied. The glow is **5 pt** in accent 1. If any of the four unset facets shows a value, the writer is inventing one.
  Calls: `Deck::set_shape_effects` · `Deck.set_shape_effects` · `Deck.setShapeEffects`
  Calls: `Deck::shape_effects` · `Deck.shape_effects` · `Deck.shapeEffects`
  Result: — · — · — · —

#### V-PPTX-02.4 — the colour transforms against PowerPoint's eyedropper

- **Risk** high — **R3**.
- **Shipped by** implemented from the ECMA-376 prose; the gaps page names `MJX-211` R3 as its owner. The artefact is `MJXOFF-219`'s: until it, `ColorSpec` carried a colour's *kind* and value and **no transform children**, so no facade call could author a `comp`, `gray`, `gamma` or `invGamma` — and no committed fixture contains one either. This was the only entry in the pass with no artefact at all.
- **Artefact** `v-pptx-02-authored.pptx`
- **Object** the two rows of swatches below the three shapes at the top. The **upper** row is a fixed `4472C4` under `comp`, `gray`, `gamma`, `invGamma` and `inv`, led by an untransformed `4472C4`; the **lower** row is the theme's accent 1 under `tint 50%`, `shade 50%`, `satMod 150%`, `lumMod 60% + lumOff 40%` and `alpha 50%`, led by an untransformed accent 1. Each swatch carries its own label.
- **Action** read what `effective_shape_fill` answers for each swatch — the harness calls it on all twelve as it writes them — then sample each rendered shape with PowerPoint's eyedropper. Compare each swatch against the baseline at the start of its row.
- **Expect** the two RGB values agree, swatch by swatch. **The two rows are not equally at risk, and that is why both are here.** The lower row is `lumMod`/`shade`/`tint`/`alpha`/`sat*`, which follow the widely-adopted Apache-POI and LibreOffice algorithm and are value-pinned in `crates/mjx-dml/tests/resolve_model.rs`; a disagreement there is surprising. The upper row is the four (five, with `inv`) that `crates/mjx-dml/src/resolve.rs` says follow *a documented interpretation* and are **not** guaranteed pixel-identical to Office — implemented from the prose and unit-tested against it, never against a renderer. A disagreement in the upper row is the third-highest-risk finding this pass can make, and being able to author these transforms is not evidence that resolving them is right.
  Calls: `Deck::effective_shape_fill` · `Deck.effective_shape_fill` · `Deck.effectiveShapeFill`
  Calls: `Deck::set_shape_fill` · `Deck.set_shape_fill` · `Deck.setShapeFill`
  Result: — · — · — · —

#### V-PPTX-02.5 — the seven rewritten fixtures, and the colour mapping the schema gate rewrote

- **Risk** medium.
- **Shipped by** `MJXOFF-53` and `MJXOFF-54`.
- **Artefact** `tests/fixtures/sample.pptx`, `tests/fixtures/tables.pptx`, `tests/fixtures/ole.pptx`, `tests/fixtures/ink.pptx`, `tests/fixtures/activex.pptx`, `tests/fixtures/vml.pptx`, `tests/fixtures/effects_theme.pptx`
- **Object** each of the seven, and in `effects_theme.pptx` the shape whose style names `a:effectRef idx="3"`.
- **Action** open all seven. Then in `effects_theme.pptx`, look at layout 1's colours and at the `a:effectRef` shape.
- **Expect** **no repair prompt** on any of the seven. Layout 1 renders with the **master's colour scheme** — the `a:masterClrMapping` written in place of an empty `a:overrideClrMapping`. The `a:effectRef idx="3"` shape **still casts its outer shadow**, and the added `a:lightRig rig="threePt"` **does not change how it is lit**.
  Calls: `Deck::color_map` · `Deck.color_map` · `Deck.colorMap`
  Calls: `Deck::effective_shape_effects` · `Deck.effective_shape_effects` · `Deck.effectiveShapeEffects`
  Result: — · — · — · —

#### V-PPTX-02.6 — the standing instruction: schema-valid is necessary, not sufficient

- **Risk** high — this is the boundary the whole pass exists to probe.
- **Shipped by** `MJXOFF-54`.
- **Artefact** `v-pptx-02-authored.pptx` and every other artefact this pass opens.
- **Object** any deck the schema job passes.
- **Action** if PowerPoint **ever** repairs a deck the schema job passed, capture the part it complained about and add a case for it.
- **Expect** no repair. The value of this entry is not the expectation but the instruction: schema validity is *necessary*, not *sufficient*, and that boundary is exactly where this pass's evidence belongs.
  Calls: `Deck::validate` · `Deck.validate` · `Deck.validate`
  Result: — · — · — · —

#### V-PPTX-02.7 — the fetch script and the schema suite, on Windows and on macOS

- **Risk** medium.
- **Shipped by** `MJXOFF-54`.
- **Artefact** `v-pptx-02-authored.pptx` — the artefact is incidental; the check is the platform.
- **Object** the `References/` fetch script and `MJX_REQUIRE_SCHEMA=1 cargo test -p mjx-pptx --features vml --test schema_validity`.
- **Action** run both on a **Windows** workstation and on a **macOS** workstation.
- **Expect** both succeed on both. Every run so far has been on Linux, and a path or line-ending assumption in the fetch script would be invisible until somebody tried.
  Calls: `Deck::open` · `Deck.open` · `Deck.open`
  Result: — · — · — · —

### Content that is not a DrawingML shape

Five kinds live in their own parts and are referenced from the slide by relationship id. All five are
authored by `crates/mjx-pptx/examples/legacy_content.rs`, which is the file `MJXOFF-58`'s own report
was written about:

```sh
cargo run -p mjx-pptx --example legacy_content -- out.pptx
cargo run -p mjx-pptx --features vml --example legacy_content -- out.pptx
```

#### V-PPTX-02.8 — SmartArt: the furthest this repository gets from "schema-valid implies correct"

- **Risk** high.
- **Shipped by** `MJXOFF-58`, with the markup modelled by `MJXOFF-148`.
- **Artefact** `crates/mjx-pptx/examples/legacy_content.rs`
- **Object** the SmartArt frame.
- **Action** open the deck and look at the diagram.
- **Expect** four rounded rectangles reading **Plan / Build / Ship / Measure**, filled from **accent 1**. Running the layout is a documented non-goal — `add_diagram` writes the data, layout, style and colour documents and PowerPoint regenerates the cached `dsp:drawing` — so **only PowerPoint runs the layout engine**, and this is the furthest any path in this repository is from *schema-valid implies correct*.
  Calls: `Deck::add_diagram` · `Deck.add_diagram` · `Deck.addDiagram`
  Calls: `Deck::diagram_parts` · `Deck.diagram_parts` · `Deck.diagramParts`
  Result: — · — · — · —

#### V-PPTX-02.9 — five new content types, and no repair prompt

- **Risk** high.
- **Shipped by** `MJXOFF-58`.
- **Artefact** `crates/mjx-pptx/examples/legacy_content.rs`
- **Object** `[Content_Types].xml` and the five new registrations the example makes.
- **Action** open the deck.
- **Expect** **no repair prompt across five new content-type registrations**. A content type PowerPoint does not accept is refused at the door, not rendered wrongly.
  Calls: `Deck::validate` · `Deck.validate` · `Deck.validate`
  Result: — · — · — · —

#### V-PPTX-02.10 — the OLE object, written without the `mc:AlternateContent` PowerPoint itself uses

- **Risk** high.
- **Shipped by** `MJXOFF-58`.
- **Artefact** `crates/mjx-pptx/examples/legacy_content.rs`
- **Object** the OLE object and its snapshot image.
- **Action** look at the snapshot, then double-click the object.
- **Expect** the snapshot draws, and double-clicking offers the **embedded stream**. Note what this is: the object is written **without the `mc:AlternateContent` PowerPoint itself uses** around `p:oleObj`, so this check is asking whether the plain form is enough.
  Calls: `Deck::add_ole_object` · `Deck.add_ole_object` · `Deck.addOleObject`
  Calls: `Deck::ole_snapshot_image_bytes` · `Deck.ole_snapshot_image_bytes` · `Deck.oleSnapshotImageBytes`
  Result: — · — · — · —

#### V-PPTX-02.11 — the ActiveX control in the VBA object model

- **Risk** medium.
- **Shipped by** `MJXOFF-58`.
- **Artefact** `crates/mjx-pptx/examples/legacy_content.rs`
- **Object** the ActiveX control.
- **Action** Developer → Visual Basic, and look at the slide's object model.
- **Expect** the control appears as **`OkButton`**, class **`{D7053240-…}`**.
  Calls: `Deck::add_activex_control` · `Deck.add_activex_control` · `Deck.addActivexControl`
  Calls: `Deck::activex_class_id` · `Deck.activex_class_id` · `Deck.activexClassId`
  Result: — · — · — · —

#### V-PPTX-02.12 — the ink content part: `p:contentPart` or `p14:contentPart`?

- **Risk** high — a **design question**, not a check with an expected result.
- **Shipped by** `MJXOFF-58`.
- **Artefact** `crates/mjx-pptx/examples/legacy_content.rs`
- **Object** the ink content part. This library writes a plain **`p:contentPart`**; producers emit **`p14:contentPart`** inside an `mc:AlternateContent`.
- **Action** open the deck and look for the ink.
- **Expect** **record which happens.** If PowerPoint **drops it silently**, the decision that follows is stated already: `add_ink` should switch to the `mc:AlternateContent` form and the MCE skip list grows by one. If PowerPoint draws it, the plain form stands.
  Calls: `Deck::add_ink` · `Deck.add_ink` · `Deck.addInk`
  Calls: `Deck::ink_references` · `Deck.ink_references` · `Deck.inkReferences`
  Result: — · — · — · —

#### V-PPTX-02.13 — `p:oleObj@spid` really names a `v:shape@id`, in a deck PowerPoint wrote

- **Risk** high.
- **Shipped by** `MJXOFF-58`.
- **Artefact** none — **blocked** on `MJXOFF-130`. The hop has been asserted only against markup this project authored; the check needs a PowerPoint-written deck carrying an OLE object and its VML backing.
- **Object** `p:oleObj@spid` and the `v:shape@id` it should name.
- **Action** run `with_vml_shape_for_ole_object` — `cargo run -p mjx-pptx --features vml --example legacy_content -- out.pptx` shows the call — against a **PowerPoint-written** deck. Note that the VML hop itself has **no call chain in three languages**: `Deck`'s VML accessors are `#[cfg(feature = "vml")]` in both bindings, so a default build publishes neither `vml_part_names` nor its neighbours. The chains below are the modern half of the hop, which every build has.
- **Expect** the identifier match resolves. `MJXOFF-114` extends the same question to Excel; `MJXOFF-113` and `MJXOFF-131` to Word.
  Calls: `Deck::ole_legacy_shape_id` · `Deck.ole_legacy_shape_id` · `Deck.oleLegacyShapeId`
  Calls: `Deck::ole_objects` · `Deck.ole_objects` · `Deck.oleObjects`
  Result: — · — · — · —

#### V-PPTX-02.14 — two OLE objects, two distinct snapshot ids

- **Risk** medium.
- **Shipped by** `MJXOFF-84`.
- **Artefact** `crates/mjx-pptx/examples/legacy_content.rs`
- **Object** the two OLE objects' snapshot pictures. They used to be `p:cNvPr id="0"` both.
- **Action** open the deck; select each snapshot in turn.
- **Expect** both draw, and both are selectable — **distinct** shape ids. A duplicate `p:cNvPr@id` inside one shape tree is what `PresentationDefect` now refuses, and this is the rendered half of that claim.
  Calls: `Deck::ole_objects` · `Deck.ole_objects` · `Deck.oleObjects`
  Calls: `Deck::validate` · `Deck.validate` · `Deck.validate`
  Result: — · — · — · —

#### V-PPTX-02.15 — the ActiveX `.bin` is byte-identical after a round trip

- **Risk** medium.
- **Shipped by** `MJXOFF-58`, and it is `MJXOFF-60`'s reorganisation that could have broken it.
- **Artefact** `tests/fixtures/activex.pptx`
- **Object** the `ppt/activeX/activeX1.bin` persistence stream.
- **Action** open the deck in PowerPoint, save it, and compare the `.bin` part against the one this library wrote.
- **Expect** **byte-identical**. The property bag inside is a documented non-goal — its meaning is per-control-class — so the only guarantee is that it is carried unchanged.
  Calls: `Deck::activex_state_bytes` · `Deck.activex_state_bytes` · `Deck.activexStateBytes`
  Result: — · — · — · —

#### V-PPTX-02.16 — the placeholders `shapes()` reports are the ones PowerPoint offers to fill

- **Risk** medium.
- **Shipped by** `MJXOFF-60`.
- **Artefact** `crates/mjx-pptx/examples/blank_deck.rs` — `cargo run -p mjx-pptx --example blank_deck -- out.pptx`.
- **Object** the placeholder list the example prints, against the empty placeholders PowerPoint shows.
- **Action** compare the two, kind for kind.
- **Expect** the same set, in the same order.
  Calls: `Deck::shapes` · `Deck.shapes` · `Deck.shapes`
  Calls: `Deck::shape_placeholder` · `Deck.shape_placeholder` · `Deck.shapePlaceholder`
  Result: — · — · — · —

#### V-PPTX-02.17 — a deck that arrived with a dangling `r:id`

- **Risk** high.
- **Shipped by** `MJXOFF-84`.
- **Artefact** none — **blocked** on `MJXOFF-130`. The check needs a **real third-party deck with a pre-existing dangling `r:id`**, which nothing in this repository can author: `save` refuses to write one.
- **Object** the dangling relationship, and the two save paths either side of it.
- **Action** open the deck and save it **unchanged**; it must still open. Then edit one slide of that same deck and save again.
- **Expect** the unchanged save **still opens** — a part still holding the bytes it arrived with is re-emitted verbatim and is never faulted, so a file that arrives broken can be written back. The edited save is **refused** by `PackageDefect` rather than producing a file PowerPoint repairs. And confirm that no legitimate empty `r:dm` / `r:lo` / `r:qs` / `r:cs` on a `dgm:relIds` was rejected on the way.
  Calls: `Deck::validate` · `Deck.validate` · `Deck.validate`
  Calls: `Deck::diagram_relationship_ids` · `Deck.diagram_relationship_ids` · `Deck.diagramRelationshipIds`
  Result: — · — · — · —

#### V-PPTX-02.18 — a fill lands between the geometry and the line

- **Risk** low.
- **Shipped by** `MJXOFF-85`.
- **Artefact** `v-pptx-02-authored.pptx`
- **Object** each shape's `a:spPr`.
- **Action** open the deck; the check is that it opens at all, and that the fills draw.
- **Expect** no repair prompt. `CT_ShapeProperties` is an `xsd:sequence`, so a fill written after the line rather than between the geometry and the line is schema-invalid and PowerPoint repairs it. The generated ordering tables put it in the right place by construction.
  Calls: `Deck::set_shape_fill` · `Deck.set_shape_fill` · `Deck.setShapeFill`
  Result: — · — · — · —

## `V-PPTX-03` · `tables` — style parts, merges, cell fills, borders and margins

Risk **high** — **R6**. Shipped by `MJXOFF-59` (merge-aware selections, the table style part) and
reorganised by `MJXOFF-60`.

#### V-PPTX-03.1 — the merged total row spans one row and two columns

- **Risk** high.
- **Shipped by** `MJXOFF-59`; the regression risk is `MJXOFF-60`.
- **Artefact** `v-pptx-03-authored.pptx`
- **Object** the merge in the last row. The markup is `gridSpan="2"` on the anchor and `hMerge="true"` on the cell to its right.
- **Action** click the merged cell and read Layout → Merge → the selection's extent.
- **Expect** **1 row × 2 columns**. If it renders **2 × 1** the writer moved, not the reader — `(rows, columns)` is the workspace's order and a transposition here would be invisible to every test that reads back what it wrote.
  Calls: `Deck::merge_cells` · `Deck.merge_cells` · `Deck.mergeCells`
  Calls: `Deck::cell_span` · `Deck.cell_span` · `Deck.cellSpan`
  Result: — · — · — · —

#### V-PPTX-03.2 — a table style this library authored, in `tableStyles.xml`

- **Risk** high — **R6**, where built-in style ids and `tableStyles.xml` interact in ways LibreOffice does not reproduce.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-03-authored.pptx`
- **Object** the style `{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}`, named *mjx validation*, and the table that points at it.
- **Action** select the table and open Table Design → Table Styles.
- **Expect** the header row is filled **`1F3864`** with **white** text and a **2 pt white bottom border**. The style appears in the gallery under its own name. This is the one area where LibreOffice's agreement proves nothing at all.
  Calls: `Deck::create_table_style` · `Deck.create_table_style` · `Deck.createTableStyle`
  Calls: `Deck::format_table_style_part` · `Deck.format_table_style_part` · `Deck.formatTableStylePart`
  Calls: `Deck::set_table_style` · `Deck.set_table_style` · `Deck.setTableStyle`
  Result: — · — · — · —

#### V-PPTX-03.6 — the style a new table is born pointing at, and whose colours it uses

- **Risk** high — **R6**, and the check MJXOFF-232 exists for. It is the one question in this area no
  machine here can answer, because it is *what PowerPoint does with the emphasis flags*.
- **Shipped by** `MJXOFF-232`.
- **Artefact** `v-pptx-03-authored.pptx`
- **Object** the **second** style in `ppt/tableStyles.xml` — `{9F6E9C1B-0B4E-4A1E-9B3D-6C2A8F4D7E10}`,
  named *Themed Header and Banded Rows*. It is what `Deck::add_table` authors and points every new
  table at, and it is also this file's `a:tblStyleLst@def`. Not one of its colours is a literal: the
  header row is `<a:schemeClr val="accent1"/>` with `<a:schemeClr val="lt1"/>` text, and the first
  horizontal band is `accent1` with `lumMod="20000" lumOff="80000"`.
  **The table in this artefact does not use it** — `V-PPTX-03.2` repoints that table at *mjx
  validation* — so this check is about the style as it sits in the gallery.
- **Action** open Table Design → Table Styles and find *Themed Header and Banded Rows* in the gallery.
  Apply it to the table. Then Design → Variants → Colours and switch the deck's theme to a visibly
  different palette.
- **Expect** the header row fills with the theme's **accent 1** and its text with **light 1**; the
  first, third … data rows fill with accent 1 at **Lighter 80%**. After the theme change **every one
  of those colours moves with it** — that is the whole claim, and a style that had pinned `4472C4`
  would stay blue in a deck rebranded green. Record whether PowerPoint lists the style in the gallery
  under its name, and whether the banding follows `bandRow`.
  Calls: `Deck::add_table` · `Deck.add_table` · `Deck.addTable`
  Result: — · — · — · —

#### V-PPTX-03.7 — the emphasis flags a table is born with, and what they resolve against

- **Risk** high — the half no gate in this repository could see before MJXOFF-232, and the reason the
  fix needed a person: *what does PowerPoint do with `firstRow="1" bandRow="1"` when nothing resolves
  them?*
- **Shipped by** `MJXOFF-232`.
- **Artefact** `v-pptx-03-authored.pptx`
- **Object** the table's `a:tblPr`, which reads `firstRow="1" bandRow="1"` and carries an
  `a:tableStyleId`. Until MJXOFF-232 it carried the two flags and **no** style id at all.
- **Action** select the table and read Table Design → the *Header Row* and *Banded Rows* checkboxes.
- **Expect** both are **ticked**, and both are visibly doing something. The old behaviour to compare
  against is a table with the same two boxes ticked and no styling anywhere — if PowerPoint had
  silently supplied a built-in style for an unresolved reference, the defect would have been
  cosmetic in this renderer and real in every other one; if it rendered unstyled, the fix is load
  bearing. **Record which.**
  Calls: `Deck::add_table` · `Deck.add_table` · `Deck.addTable`
  Calls: `Deck::table_part` · `Deck.table_part` · `Deck.tablePart`
  Result: — · — · — · —

#### V-PPTX-03.3 — cell anchoring, margins and a single cell border

- **Risk** medium.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-03-authored.pptx`
- **Object** row 0 (anchored centre, **6 pt** uniform margins) and the top border of the cell at row 2, column 0.
- **Action** Table Design → Cell Options for row 0; then select the row-2 cell and read its top border.
- **Expect** row 0's text is vertically **centred**, with **6 pt** on all four inner margins. The row-2 cell's **top** border is **1 pt** in **`C00000`**, and no other edge of it is drawn.
  Calls: `Deck::format_cells` · `Deck.format_cells` · `Deck.formatCells`
  Calls: `Deck::set_cell_border` · `Deck.set_cell_border` · `Deck.setCellBorder`
  Result: — · — · — · —

#### V-PPTX-03.4 — formatting across a merge, then unmerging in PowerPoint

- **Risk** high.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-03-authored.pptx`
- **Object** the merged region in the last row, and the cells it covers.
- **Action** format a selection that only **partly** covers the merge through this library, open in PowerPoint, then **unmerge** there.
- **Expect** each cell shows its **own** formatting. A cell covered by a merge renders nothing, so a formatter skips it rather than painting through it — and unmerging is what makes that visible. A merged region anchored *outside* the selection must be left alone entirely.
  Calls: `Deck::format_cells` · `Deck.format_cells` · `Deck.formatCells`
  Calls: `Deck::unmerge_cells` · `Deck.unmerge_cells` · `Deck.unmergeCells`
  Result: — · — · — · —

#### V-PPTX-03.5 — a vendor `a:ext` after a new `a:solidFill` inside `a:tcPr`

- **Risk** medium — an interrogation of the **`extLst` is never modelled** non-goal.
- **Shipped by** `MJXOFF-59`, ordered by `MJXOFF-85`.
- **Artefact** `tests/fixtures/table_extensions.pptx`
- **Object** a cell whose `a:tcPr` carries an `a:extLst`, with a fill written into it.
- **Action** set a cell fill through this library, then open the deck.
- **Expect** **no repair prompt**. `a:extLst` is last in `CT_TableCellProperties`, and the claim is that a new `a:solidFill` lands before it rather than after — the extension survives an edit *and stays where the sequence puts it*.
  Calls: `Deck::set_cell_fill` · `Deck.set_cell_fill` · `Deck.setCellFill`
  Result: — · — · — · —

## `V-PPTX-04` · `charts` — series, axes, legend, data labels and trendlines

Risk **medium** — **R4**. Shipped by `MJXOFF-57`, with the workbook writer moved to `mjx-sml` by
`MJXOFF-99` and extended to Word by `MJXOFF-103` and Excel by `MJXOFF-111`.

#### V-PPTX-04.1 — *Edit Data* opens a workbook holding the numbers the chart draws

- **Risk** high — **R4**'s core.
- **Shipped by** `MJXOFF-57`, on `MJXOFF-99`'s writer.
- **Artefact** `v-pptx-04-authored.pptx`
- **Object** the chart, and the `.xlsx` embedded at `/ppt/embeddings/`.
- **Action** right-click the chart → Edit Data.
- **Expect** **no repair prompt**; series names across **row 1**, categories down **column A**, values from **B2** — and the same numbers the chart draws: `12`, `15.5`, `14`, `19.25` for *2026* and `10.5`, `13`, `13.75`, `16` for *2025*, over `Q1`…`Q4`.
  Calls: `Deck::add_chart` · `Deck.add_chart` · `Deck.addChart`
  Calls: `Deck::chart_workbooks` · `Deck.chart_workbooks` · `Deck.chartWorkbooks`
  Result: — · — · — · —

#### V-PPTX-04.2 — title, axis titles, legend position and a series fill

- **Risk** low.
- **Shipped by** `MJXOFF-57`.
- **Artefact** `v-pptx-04-authored.pptx`
- **Object** the chart's title, its two axis titles, its legend and series *2026*.
- **Action** read each off the rendered chart.
- **Expect** chart title *Revenue by quarter*; category axis titled *Quarter*; value axis titled *Revenue*; legend **below** the plot; series *2026* filled from **accent 1**; a **linear** trendline on it.
  Calls: `Deck::set_chart_title` · `Deck.set_chart_title` · `Deck.setChartTitle`
  Calls: `Deck::set_chart_legend` · `Deck.set_chart_legend` · `Deck.setChartLegend`
  Calls: `Deck::set_chart_axis_title` · `Deck.set_chart_axis_title` · `Deck.setChartAxisTitle`
  Calls: `Deck::add_chart_trendline` · `Deck.add_chart_trendline` · `Deck.addChartTrendline`
  Result: — · — · — · —

#### V-PPTX-04.3 — R4 in all four hosts

- **Risk** high — **R4**.
- **Shipped by** `MJXOFF-57`, `MJXOFF-99`, `MJXOFF-103` and `MJXOFF-111`.
- **Artefact** `v-pptx-04-authored.pptx`, `v-docx-04-authored.docx`, `v-xlsx-04-authored.xlsx`
- **Object** the embedded workbook of each, and — for Excel — the live range the chart reads instead.
- **Action** open *Edit Data* in each of the three, and confirm the numbers agree with the chart.
- **Expect** all three agree. Since MJXOFF-208 a data edit **patches** the embedded workbook: the new numbers go into the cells the series' own `c:f` names, and every other sheet, cell format, defined name and macro the workbook carried is left exactly as it was. A `c:f` this library will not write over is refused by name rather than rebuilt over; `regenerate_chart_workbook` is the explicit opt-in that does replace the workbook wholesale, and `detach_chart_workbook` drops the reference instead. Exactly **one** SpreadsheetML writer ships, in `mjx-sml`.
  Calls: `Deck::refresh_chart_workbook` · `Deck.refresh_chart_workbook` · `Deck.refreshChartWorkbook`
  Calls: `Document::add_chart` · `Document.add_chart` · `Document.addChart`
  Calls: `Workbook::add_range_chart` · `Workbook.add_range_chart` · `Workbook.addRangeChart`
  Result: — · — · — · —

#### V-PPTX-04.4 — re-save each chart from Office and read it back

- **Risk** high — the **R2** antidote, and `MJXOFF-130`'s input.
- **Shipped by** `MJXOFF-57`.
- **Artefact** `v-pptx-04-authored.pptx`
- **Object** the whole file, after PowerPoint has written it.
- **Action** open each chart artefact, save from PowerPoint, and read the saved file back through this library.
- **Expect** it opens, its series read back, and its parts round-trip. **These saved files are what `MJXOFF-130`'s corpus should be built from** — the first files in this repository's history that Microsoft Office wrote.
  Calls: `Deck::open` · `Deck.open` · `Deck.open`
  Calls: `Deck::chart_series` · `Deck.chart_series` · `Deck.chartSeries`
  Result: — · — · — · —

#### V-PPTX-04.5 — a positioned `p:graphicFrame` puts `p:xfrm` after `p:nvGraphicFramePr`

- **Risk** low.
- **Shipped by** `MJXOFF-85`.
- **Artefact** `v-pptx-04-authored.pptx`
- **Object** the chart's `p:graphicFrame`.
- **Action** open the deck.
- **Expect** no repair prompt. `CT_GraphicalObjectFrame` is a sequence and the transform comes **after** the non-visual properties; the generated ordering tables put it there by construction rather than by a hand-written table.
  Calls: `Deck::add_chart` · `Deck.add_chart` · `Deck.addChart`
  Result: — · — · — · —

## `V-PPTX-05` · `pictures` — pictures and picture fills

Risk **low**. Shipped by `MJXOFF-59`.

#### V-PPTX-05.1 — a picture and a picture-filled shape

- **Risk** low.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-05-authored.pptx`
- **Object** the `p:pic` at the left, and the rectangle beside it whose fill is the same image stretched.
- **Action** open the deck; select each and read Shape Format → Shape Fill → Picture.
- **Expect** both draw the same placeholder image. The rectangle's fill is **stretched**, not tiled, and both point at **one** image part rather than two copies of it.
  Calls: `Deck::add_picture` · `Deck.add_picture` · `Deck.addPicture`
  Calls: `Deck::add_image` · `Deck.add_image` · `Deck.addImage`
  Calls: `Deck::picture_image_bytes` · `Deck.picture_image_bytes` · `Deck.pictureImageBytes`
  Result: — · — · — · —

## `V-PPTX-06` · `notes-and-links` — speaker notes, shape hyperlinks and run hyperlinks

Risk **low**. Shipped by `MJXOFF-59` (hyperlinks) and the notes surface reached through the facade by
`MJXOFF-61`.

#### V-PPTX-06.1 — the notes slide, created on demand

- **Risk** low.
- **Shipped by** `MJXOFF-61`.
- **Artefact** `v-pptx-06-authored.pptx`
- **Object** `ppt/notesSlides/notesSlide1.xml`, which this library created because the deck had none.
- **Action** View → Notes Page.
- **Expect** the notes read *"Lead with the revenue number, then the regional split."* — and the notes master exists, because a notes slide without one is a repair.
  Calls: `Deck::set_notes_text` · `Deck.set_notes_text` · `Deck.setNotesText`
  Calls: `Deck::notes_text` · `Deck.notes_text` · `Deck.notesText`
  Result: — · — · — · —

#### V-PPTX-06.2 — a whole-shape link and a single-run link

- **Risk** low.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-06-authored.pptx`
- **Object** the two text boxes, and their two `a:hlinkClick` relationships.
- **Action** run the slide show and click each.
- **Expect** the first text box is a link **as a whole shape** (clicking anywhere on it navigates to `https://example.com/investors`); on the second, **only the run** is a link (`https://example.com/report`), and clicking the shape's background does nothing.
  Calls: `Deck::set_shape_hyperlink` · `Deck.set_shape_hyperlink` · `Deck.setShapeHyperlink`
  Calls: `Deck::set_run_hyperlink` · `Deck.set_run_hyperlink` · `Deck.setRunHyperlink`
  Result: — · — · — · —

#### V-PPTX-06.3 — `p:notesMasterIdLst` lands between `p:sldMasterIdLst` and `p:sldIdLst`

- **Risk** low.
- **Shipped by** `MJXOFF-85`.
- **Artefact** `v-pptx-06-authored.pptx`
- **Object** `ppt/presentation.xml`.
- **Action** open the deck.
- **Expect** no repair prompt. `CT_Presentation` is an `xsd:sequence` and the notes-master list sits **between** the slide-master list and the slide list; writing it anywhere else is schema-invalid, and this artefact is the only one that has a notes master to place.
  Calls: `Deck::set_notes_text` · `Deck.set_notes_text` · `Deck.setNotesText`
  Result: — · — · — · —

## `V-PPTX-07` · `geometry` — preset adjustments, custom geometry, and a 4:3 deck's rescaled placeholders

Risk **medium**. Shipped by `MJXOFF-56` (the guide-formula evaluator) and `MJXOFF-55` (the blank
deck). **This is the one artefact authored at 4:3** — `9_144_000 × 6_858_000`, `screen4x3` — so A3's
rescale question has a file.

#### V-PPTX-07.1 — a chevron's adjustment maximum, read off the shape and dragged in PowerPoint

- **Risk** medium.
- **Shipped by** `MJXOFF-56`.
- **Artefact** `v-pptx-07-authored.pptx`
- **Object** the two `chevron`s: one **2.5 × 2.5 in** (square) and one **2.5 × 1.25 in** (2:1). `maxAdj` is the guide `*/ 100000 w ss`, so the same preset answers a different bound for each.
- **Action** drag each chevron's yellow adjustment handle **to its far right**, save from PowerPoint, and read the `adj` value back.
- **Expect** the value equals what `shape_adjustments(…)` reports as the maximum: **100000 for the square**, **200000 for the 2:1 shape**. A bound resolved against the wrong side would give 100000 for both.
  Calls: `Deck::shape_adjustments` · `Deck.shape_adjustments` · `Deck.shapeAdjustments`
  Result: — · — · — · —

#### V-PPTX-07.2 — a `custGeom` with all five auxiliary lists, and draggable connection sites

- **Risk** medium.
- **Shipped by** `MJXOFF-56`.
- **Artefact** `v-pptx-07-authored.pptx`
- **Object** the custom-geometry shape at the right. Its `a:custGeom` carries **`a:avLst`, `a:gdLst`, `a:cxnLst`, `a:rect` and `a:pathLst`** — all five.
- **Action** open the deck; then drag a connector onto the shape and look for its connection points.
- **Expect** it **opens clean**, and its three connection sites are **draggable connector targets**. A `custGeom` whose auxiliary lists are written in the wrong order is a repair, not a wrong picture.
  Calls: `Deck::set_shape_geometry` · `Deck.set_shape_geometry` · `Deck.setShapeGeometry`
  Calls: `Deck::shape_geometry` · `Deck.shape_geometry` · `Deck.shapeGeometry`
  Result: — · — · — · —

#### V-PPTX-07.3 — the apex sits at the horizontal centre because its `a:pt@x` is a guide

- **Risk** medium.
- **Shipped by** `MJXOFF-56`.
- **Artefact** `v-pptx-07-authored.pptx`
- **Object** the same shape's path. Its first `a:moveTo` names the guide **`apex`**, whose formula is **`*/ w 1 2`** — half the shape's width — rather than a number.
- **Action** look at where the triangle's apex is drawn; then resize the shape and look again.
- **Expect** the apex sits at the **horizontal centre**, and stays there when the shape is resized. A literal coordinate would not move.
  Calls: `Deck::set_shape_geometry` · `Deck.set_shape_geometry` · `Deck.setShapeGeometry`
  Result: — · — · — · —

#### V-PPTX-07.4 — `moon`, `arc`, `circularArrow` and `gear9` against the spec's normative images

- **Risk** medium.
- **Shipped by** `MJXOFF-56`.
- **Artefact** `v-pptx-07-authored.pptx`
- **Object** the four presets in the bottom row. These are the shapes whose guide formulas take an **arc-tangent argument through zero**, which is where a naive evaluator goes wrong.
- **Action** compare each against the normative image in ECMA-376 Part 1's preset-geometry annex.
- **Expect** all four match. A sign error in the arc-tangent case draws a shape that is still a shape — which is exactly why a machine cannot answer this one.
  Calls: `Deck::add_shape` · `Deck.add_shape` · `Deck.addShape`
  Calls: `Deck::shape_geometry` · `Deck.shape_geometry` · `Deck.shapeGeometry`
  Result: — · — · — · —

#### V-PPTX-07.5 — saving after only *reading* geometry changes nothing

- **Risk** medium.
- **Shipped by** `MJXOFF-56`.
- **Artefact** `v-pptx-07-authored.pptx`
- **Object** every part of the file.
- **Action** open the deck through this library, call the geometry readers on every shape, save, and compare each part's decompressed payload against the original. Then open both files in PowerPoint.
- **Expect** every part is **byte-identical**, and both files render the same. Reading may materialise a tree; it must never mark a part modified.
  Calls: `Deck::shape_geometry` · `Deck.shape_geometry` · `Deck.shapeGeometry`
  Calls: `Deck::shape_adjustments` · `Deck.shape_adjustments` · `Deck.shapeAdjustments`
  Result: — · — · — · —

#### V-PPTX-07.6 — is `effective_shape_bounds == None` really Office's behaviour?

- **Risk** high — **R7**, a **design question**, and the one place this pass interrogates a non-goal on purpose.
- **Shipped by** `MJXOFF-59`.
- **Artefact** none — **blocked** on `MJXOFF-130`. `set_shape_transform` writes **only the fields its argument names** — an unset field means *leave it alone*, never *clear it* — so no facade call can author a transform that names a rotation and neither `a:off` nor `a:ext`, and no committed fixture carries one.
- **Object** a shape whose `a:xfrm` states `@rot` alone.
- **Action** open the file in PowerPoint and read where the shape is placed and how large it is.
- **Expect** the gaps page states this as a **non-goal with a reason**: *a transform is inherited whole — the first tier that places a shape wins entirely, and a shape cannot take its position from one tier and its size from another, so a partial transform places nothing.* The question is not whether the code matches that; it does. The question is **whether the documentation is right**. **Record what PowerPoint does**, and the decision that follows — keep `None`, or resolve position and size from different tiers — is the user's.
  Calls: `Deck::effective_shape_bounds` · `Deck.effective_shape_bounds` · `Deck.effectiveShapeBounds`
  Result: — · — · — · —

#### V-PPTX-07.7 — a 4:3 blank deck's title placeholder sits inside the slide

- **Risk** medium.
- **Shipped by** `MJXOFF-55`.
- **Artefact** `v-pptx-07-authored.pptx` — the only artefact authored at 4:3 (`9_144_000 × 6_858_000`).
- **Object** the title and body placeholders on slide 1, taken from the layout, with the master's rescaled `a:xfrm`.
- **Action** Design → Slide Size; then look at where the two placeholders sit.
- **Expect** the size reads **On-screen Show (4:3)**, **10 × 7.5 in**; the title placeholder sits **inside** the slide, not hanging off its right edge. `Presentation::blank` rescales the master's placeholder geometry to the slide size it was asked for, and a deck built at anything but widescreen is the only place that shows.
  Calls: `Deck::blank` · `Deck.blank` · `Deck.blank`
  Calls: `Deck::shape_bounds` · `Deck.shape_bounds` · `Deck.shapeBounds`
  Result: — · — · — · —

#### V-PPTX-07.8 — a rotated shape

- **Risk** low.
- **Shipped by** `MJXOFF-59`.
- **Artefact** `v-pptx-07-authored.pptx`
- **Object** the small rectangle below the custom-geometry shape, whose `a:xfrm@rot` is `1800000`.
- **Action** select it and read Shape Format → Size → Rotation.
- **Expect** **30°**. Its bounds are still reported, because its `a:off` and `a:ext` are both present — which is what distinguishes it from `V-PPTX-07.6`.
  Calls: `Deck::set_shape_transform` · `Deck.set_shape_transform` · `Deck.setShapeTransform`
  Calls: `Deck::effective_shape_bounds` · `Deck.effective_shape_bounds` · `Deck.effectiveShapeBounds`
  Result: — · — · — · —

## `V-PPTX-08` · `chart-decoration` — data labels, per-point formatting, trendlines, error bars and every plot type

Risk **medium**. Shipped by `MJXOFF-87` (the decoration model) over `MJXOFF-57` (the chart spine).
Six slides: decoration, axes, the dangling anchor, and four of gallery.

#### V-PPTX-08.1 — a series' values, rewritten

- **Risk** high.
- **Shipped by** `MJXOFF-57`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 1, the chart titled *Series values, rewritten*. It was authored at `19.2`, `21.4`, `16.7` and then rewritten.
- **Action** read the plotted values, then right-click → Edit Data and read the workbook.
- **Expect** **41.5 / 42.5 / 43.5**, in both the chart and the workbook — **not** the `19.2 / 21.4 / 16.7` it was authored with. A rewrite that reached the cache and not the workbook, or the workbook and not the cache, shows here as a disagreement between the two.
  Calls: `Deck::set_chart_series_values` · `Deck.set_chart_series_values` · `Deck.setChartSeriesValues`
  Result: — · — · — · —

#### V-PPTX-08.2 — the three label tiers, merged

- **Risk** high.
- **Shipped by** `MJXOFF-87`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 1, the chart titled *Labels, three tiers*. The plot tier shows the value; series 0 adds the category name; point 1 of series 0 is suppressed; series 1 is suppressed entirely.
- **Action** look at each label.
- **Expect** **series 0's labels show the category name**, **point 1 of series 0 shows none**, and **series 1 shows none at all**. Any other arrangement means the merge is applied in the wrong order.
  Calls: `Deck::set_chart_data_labels` · `Deck.set_chart_data_labels` · `Deck.setChartDataLabels`
  Calls: `Deck::suppress_chart_data_labels` · `Deck.suppress_chart_data_labels` · `Deck.suppressChartDataLabels`
  Result: — · — · — · —

#### V-PPTX-08.3 — *Format Data Labels* reports what `chart_data_label_tier` returns

- **Risk** medium.
- **Shipped by** `MJXOFF-87`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** the same chart's plot-tier labels: position **outside end**, separator **`"; "`**, number format **`0.0`**.
- **Action** right-click a label → Format Data Labels, and read Label Position, Separator and Number → Format Code.
- **Expect** all three match what `chart_data_label_tier` answers for that scope. Labels draw on **every** plot kind that admits `c:dLbls` — the gallery on slides 4 to 6 is where that is visible.
  Calls: `Deck::chart_data_label_tier` · `Deck.chart_data_label_tier` · `Deck.chartDataLabelTier`
  Calls: `Deck::chart_data_labels` · `Deck.chart_data_labels` · `Deck.chartDataLabels`
  Result: — · — · — · —

#### V-PPTX-08.4 — `c:dPt`'s `c:idx` addresses the slice we meant

- **Risk** high.
- **Shipped by** `MJXOFF-87`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 1, the pie titled *Slice 1 exploded, slice 0 recoloured*, over `North` / `South` / `East` / `West`.
- **Action** look at which slice is pulled out and which is recoloured; then right-click each → Format Data Point.
- **Expect** **slice 1 exploded 25 %** and **slice 0 filled `2E75B6`**. An off-by-one in `c:idx` puts both on the wrong slice and nothing in this repository would notice.
  Calls: `Deck::set_chart_point_explosion` · `Deck.set_chart_point_explosion` · `Deck.setChartPointExplosion`
  Calls: `Deck::set_chart_point_fill` · `Deck.set_chart_point_fill` · `Deck.setChartPointFill`
  Result: — · — · — · —

#### V-PPTX-08.5 — a polynomial trendline of order 3, extended two categories forward

- **Risk** medium.
- **Shipped by** `MJXOFF-87`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 1, the trendline on series *2026* of the *Labels, three tiers* chart.
- **Action** right-click the trendline → Format Trendline.
- **Expect** **Polynomial**, **Order 3**; Forecast Forward **2** periods, Backward **0**; *Display Equation on chart* and *Display R-squared value on chart* both **on**, and both drawn.
  Calls: `Deck::add_chart_trendline` · `Deck.add_chart_trendline` · `Deck.addChartTrendline`
  Calls: `Deck::chart_trendlines` · `Deck.chart_trendlines` · `Deck.chartTrendlines`
  Result: — · — · — · —

#### V-PPTX-08.6 — two sets of error bars on one scatter series, one per axis

- **Risk** medium.
- **Shipped by** `MJXOFF-87`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 1, the scatter titled *Error bars on both axes*.
- **Action** click the series and read Chart Elements → Error Bars → More Options; the pane offers a horizontal and a vertical set.
- **Expect** **two** sets: the X set is **percentage, 5 %, both directions**; the Y set is **fixed value, 0.5, both directions**. A scatter series is the only plot kind that takes two, and a setter that replaced rather than matched on direction would leave one.
  Calls: `Deck::set_chart_error_bars` · `Deck.set_chart_error_bars` · `Deck.setChartErrorBars`
  Calls: `Deck::chart_error_bars` · `Deck.chart_error_bars` · `Deck.chartErrorBars`
  Result: — · — · — · —

#### V-PPTX-08.7 — a `c:dPt` left past the end of its series

- **Risk** high — **evidence for a design decision**, not a check with an expected result.
- **Shipped by** `MJXOFF-87`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 3, the chart titled *A c:dPt left past the end of its series*. Its last point was formatted `C00000` at index 2, and the series was then shortened to two values; the `c:dPt` at **index 2** now addresses nothing.
- **Action** open the deck and look at the chart.
- **Expect** **record which happens.** The modelled behaviour is that a `c:dPt`'s `c:idx` is **never renumbered** by an edit that changes a series' length, and dangling anchors are reported and dropped **only on request**. If PowerPoint **ignores** the dangling `c:dPt` and renders the two-point series, that decision is right as it stands. If PowerPoint offers a **repair**, the decision that follows is the user's: drop dangling anchors on save, or keep reporting them.
  Calls: `Deck::chart_dangling_decoration` · `Deck.chart_dangling_decoration` · `Deck.chartDanglingDecoration`
  Calls: `Deck::drop_chart_dangling_decoration` · `Deck.drop_chart_dangling_decoration` · `Deck.dropChartDanglingDecoration`
  Result: — · — · — · —

#### V-PPTX-08.8 — all sixteen plot types draw as their type

- **Risk** medium.
- **Shipped by** `MJXOFF-57`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slides 4 to 6, four charts to a slide, each titled with its `c:` element's local name — `barChart`, `bar3DChart`, `lineChart`, `line3DChart`, `pieChart`, `pie3DChart`, `ofPieChart`, `areaChart`, `area3DChart`, `scatterChart`, `doughnutChart`, `radarChart`, `bubbleChart`, `surfaceChart`, `surface3DChart` and `stockChart`.
- **Action** flip through the three gallery slides and compare each chart against its title.
- **Expect** **all sixteen** draw as their type, and the **stock chart draws high-low-close from three series** (`High` 7/8/9, `Low` 3/4/5, `Close` 5/6/7). The sixteen share only eight series types, so a plot written with the wrong series element renders as a different chart or not at all.
  Calls: `Deck::add_chart` · `Deck.add_chart` · `Deck.addChart`
  Calls: `Deck::chart_kinds` · `Deck.chart_kinds` · `Deck.chartKinds`
  Result: — · — · — · —

#### V-PPTX-08.9 — axes bounded, reversed and ruled, with two coloured series

- **Risk** medium.
- **Shipped by** `MJXOFF-57`; the child order is `MJXOFF-85`'s.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 2, the chart titled *Bounded 0-25, reversed, ruled*.
- **Action** read the value axis's bounds and direction; look for gridlines on both axes; read the two series' formats.
- **Expect** the value axis is bounded **0–25** and **reversed** (values in reverse order); the category axis has **major** gridlines and the value axis **major and minor**; series *2026* is filled **`4472C4`** and series *2025* is outlined **`ED7D31`** at 2 pt. And the file opens at all, which is `CT_Scaling`'s doing: **`c:max` precedes `c:min`** there — the one place in these schemas where the intuitive order is wrong.
  Calls: `Deck::set_chart_axis_scale` · `Deck.set_chart_axis_scale` · `Deck.setChartAxisScale`
  Calls: `Deck::set_chart_axis_orientation` · `Deck.set_chart_axis_orientation` · `Deck.setChartAxisOrientation`
  Calls: `Deck::set_chart_axis_gridlines` · `Deck.set_chart_axis_gridlines` · `Deck.setChartAxisGridlines`
  Calls: `Deck::set_chart_series_line` · `Deck.set_chart_series_line` · `Deck.setChartSeriesLine`
  Result: — · — · — · —

#### V-PPTX-08.10 — a chart with no embedded workbook still renders

- **Risk** high — **R4**'s other half.
- **Shipped by** `MJXOFF-57`.
- **Artefact** `v-pptx-08-authored.pptx`
- **Object** slide 2, the chart titled *This chart has no embedded workbook*. Its `c:externalData` — the element and its relationship — has been removed.
- **Action** look at the chart; then right-click → Edit Data.
- **Expect** the chart **still renders**, from its own caches. *Edit Data* makes PowerPoint **offer to create a workbook from the caches** rather than reporting a broken file. This is the escape hatch for a caller who would rather keep a stale third-party workbook than lose the formatting or extra sheets it carried, so it has to be usable.
  Calls: `Deck::detach_chart_workbook` · `Deck.detach_chart_workbook` · `Deck.detachChartWorkbook`
  Calls: `Deck::chart_workbooks` · `Deck.chart_workbooks` · `Deck.chartWorkbooks`
  Result: — · — · — · —

#### V-PPTX-08.11 — editing one shape's text is confined to the edited run

- **Risk** high.
- **Shipped by** `MJXOFF-86`, completed by `MJXOFF-143`.
- **Artefact** `tests/fixtures/vml.pptx`
- **Object** one shape's text, and the slide part around it.
- **Action** change one shape's text through this library and diff the slide part against the original. Then run `edit_vml_drawing` on the same deck and open both results in PowerPoint.
- **Expect** the text change is **confined to the edited run**; everything else in the part comes back byte-for-byte. The `edit_vml_drawing` result **opens and renders identically** — and note that this is no longer the documented re-flow limitation: `MJXOFF-143` carried the source span through `FromXml`/`ToXml`, so a whole-part typed model now copies the bytes of everything it did not change, and that row has moved to *What used to be here* on the gaps page.
  Calls: `Deck::set_shape_text_content` · `Deck.set_shape_text_content` · `Deck.setShapeTextContent`
  Result: — · — · — · —

#### V-PPTX-08.12 — namespace prefixes declared only at the slide root

- **Risk** high — the one failure mode that would be **silent** in this repository's own suite.
- **Shipped by** `MJXOFF-86`.
- **Artefact** none — **blocked** on `MJXOFF-130`. Every deck this library authors declares its prefixes where the writer puts them; the check needs a file whose `xmlns:` prefixes are declared **only at the slide root**.
- **Object** a nested shape deep inside such a slide.
- **Action** edit that nested shape, save, and open in PowerPoint.
- **Expect** every prefix still resolves, and the file opens. A serializer that copied a subtree out of its declaring scope would produce markup that is well-formed, schema-checkable in isolation, and unopenable.
  Calls: `Deck::set_shape_text_content` · `Deck.set_shape_text_content` · `Deck.setShapeTextContent`
  Result: — · — · — · —
