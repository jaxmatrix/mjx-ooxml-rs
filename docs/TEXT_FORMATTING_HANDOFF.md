# Handoff — text formatting (`a:rPr` / `a:pPr` / bullets / inheritance) — IN PROGRESS

The workstream that makes text *look* like something. Read after `docs/PHASE2_HANDOFF.md` (§3
guardrails); `docs/LAYOUT_MASTER_HANDOFF.md` is the immediately preceding workstream and left this as
its one open follow-up.

**Status: T1–T4 shipped (#48, #49, #50, #51, #52) — `main` at `0.0.6`, 528 tests green.**
**➡ NEXT: T5 (theme font scheme), then T6 (effective resolution).** T6 is what the workstream is
*for*: it answers *what size is this title actually rendered at*, which is a question no amount of
explicit modeling can answer, because the answer lives in the master's `p:txStyles`.

## Why this workstream exists

After Phase 3 a caller could add a shape and set its text — and could not make it bold, resize it,
recolour it or centre it. `a:rPr` and `a:pPr` were opaque `RawNode`s. Worse, nothing could report what
a placeholder's text *renders* as, because that is inherited through five tiers the code never read.

## What shipped

```rust
// Read and write formatting at whatever the user selected.
deck.set_run_properties(slide, shape, para, run, &bold)?;          // one run
deck.set_paragraph_run_properties(slide, shape, para, &bold)?;     // a paragraph + its mark
deck.set_shape_run_properties(slide, shape, &bold)?;               // a whole text box
deck.set_text_range_properties(slide, shape, para, 2..10, &bold)?; // part of a paragraph — splits runs
deck.set_text_range_properties_by_grapheme(slide, shape, para, 5..6, &bold)?;

// Paragraph layout, including the bullet that expresses its level.
deck.set_paragraph_properties(slide, shape, para, &ParagraphPropertiesSpec::new()
    .with_level(IndentLevel::of(1))
    .with_alignment(TextAlignment::Justified)
    .with_left_margin_points(36.0)
    .with_indent_points(-18.0)
    .with_bullet_character("•"))?;

// The format an *empty* paragraph holds — what a placeholder starts as, and what typing inherits.
deck.set_end_run_properties(slide, shape, para, &CharacterPropertiesSpec::new().with_size_points(24.0))?;

// The paragraph axis, and the readers.
deck.paragraph_count(slide, shape)?;  deck.run_count(slide, shape, para)?;
deck.paragraph_text(slide, shape, para)?;  deck.run_text(slide, shape, para, run)?;
deck.paragraph_properties(slide, shape, para)?;  deck.run_properties(slide, shape, para, run)?;
deck.end_run_properties(slide, shape, para)?;
```

Per PR:

- **T1 (#48)** — generated `TextUnderline`, `TextStrike`, `TextCapitalization`, `TextAlignment`,
  `FontAlignment`, `TabAlignment`, `AutonumberScheme` (41 values), named from the ECMA §20.1.10
  tables. Measures `FontSize` and `TextPoint`.
- **T2 (#49)** — `CharacterProperties` + `CharacterPropertiesSpec`, `TextFont`,
  `resolve_character_properties`; typed `a:rPr` and `a:endParaRPr` in the text tree.
- **T3 (#50)** — `ParagraphProperties` + `ParagraphPropertiesSpec`, `TextSpacing`, `TabStop`,
  `IndentLevel`, `TextListStyle`; typed `a:pPr` and `a:lstStyle`.
- **T3b (#51)** — `Bullet`, `BulletCharacter`, `AutoNumberBullet`, `BulletPicture`, `BulletColor`,
  `BulletSize`, `BulletTypeface`.
- **T4 (#52)** — the `Presentation` surface above, plus `TextRun::split_at` /
  `Paragraph::split_run_at` / `Paragraph::set_end_properties` in `mjx-dml`.

Tests: `crates/mjx-dml/tests/{character_model,paragraph_model,text_model,text_measures}.rs`,
`crates/mjx-pptx/tests/text_formatting.rs`, and an `office_open.rs` canary that renders a formatted,
multi-level bulleted deck through LibreOffice.

## Decisions settled — do not re-litigate

1. **Friendly units on the surface, wire units behind `from_wire`/`to_wire`.** Sizes and spacing are
   **points**; margins, indents and tab stops are **points**. A caller never meets hundredths of a
   point or EMU. (`Emu`/`FontSize`/`TextPoint` still expose the wire form for de/serialization.)
2. **Spec types are builders with `with_`-prefixed setters**, so the *readers* keep the plain property
   names (`spec.underline()`, not `spec.underline_value()`). Rust cannot have both; reading wins,
   because reading is the primary use once T6 lands.
3. **Merge, don't rebuild.** `a:rPr` carries `lang`/`dirty`/`err`/`smtClean`/hyperlinks and `a:pPr`
   carries `eaLnBrk`/`hangingPunct`/bullets on the same tag as what we model, so `apply` writes only
   what a spec *names*. **An unset property means "leave it alone", never "clear it."** Fill/outline/
   effects rebuild their elements wholesale and that is fine — those are self-contained.
4. **Bullets are four independent fields**, not one bundled struct, because the schema's four choice
   groups inherit separately (a level may set the character and inherit the colour). Each group's
   **`FollowText` arm is a real variant**: `<a:buClrTx/>` ("match the text") is a decision, while an
   absent group ("inherit") is not.
5. **The level → `lvlNpPr` off-by-one lives only in `TextListStyle::level`.** Level 0 reads
   `lvl1pPr`, level 8 reads `lvl9pPr`. Nowhere else in the codebase knows this.
6. **Four selection scopes**, because a run boundary exists *because* formatting changes there: run,
   paragraph, shape, character range. **Paragraph- and shape-wide setters also reach `a:endParaRPr`**
   (what a user means by "I selected this and made it bold" — the next keystroke is bold too); the
   single-run setter does not.
7. **Runs split but never merge.** Splitting gives both halves the original's `a:rPr`, so it changes
   nothing visually. A range already aligned to run boundaries splits nothing, so repeated edits do
   not accumulate runs; only overlapping *different* ranges can grow the count.
8. **Two offset spaces, one implementation** — scalars (`char`) by default, graphemes opt-in via
   `..._by_grapheme` (which converts and delegates). A scalar offset *can* split a grapheme cluster;
   that is the caller's choice, since they chose the number.
9. **`buSzPct` is written `val="111%"`** — the form both the strict and transitional schemas specify
   and ECMA §21.1.2.4.9 illustrates — and **read in both** spellings, since the integer form appears in
   the wild.
10. **Schema order is validity, not style.** New children go in at their rank via
    `build::replace_or_insert_child`; `known_rank` in `paragraph_properties.rs` ranks the bullet groups
    between the spacing elements and `a:tabLst`.

## Current state — verified, not assumed

- **`crates/mjx-dml/src/theme.rs` models `a:clrScheme` and `a:fmtScheme` but NOT `a:fontScheme`.**
  `Theme` exposes `color_scheme`, `fill_style(s)`, `line_style(s)`, `effect_style`; `ThemeInfo` mirrors
  them interner-free. That gap is T5.
- **`TextFont` already models `CT_TextFont`** (`crates/mjx-dml/src/text/font.rs`) with
  `typeface`/`panose`/`pitch_family`/`charset` and **`is_theme_reference()`**, which spots a `+mj-lt`
  style reference. It was given its own file precisely so T5 could reuse it.
- **`TextListStyle` reads any `CT_TextListStyle`** — a shape's `a:lstStyle`, a placeholder's, and each
  of a master's three `p:txStyles` children are all the same type, so T6's ladder walks one type at
  every tier.
- **Nothing reads `p:txStyles` or `p:defaultTextStyle` yet.** Both are T6.
- **Fixtures** (read out of the zips, not assumed):
  - `layouts.pptx` theme: `<a:fontScheme name="Office">`, `majorFont` latin **`Calibri Light`**,
    `minorFont` latin **`Calibri`** — so T5 is testable on day one.
  - `layouts.pptx` master `p:txStyles`: all three styles present, each with **only `lvl1pPr`**
    (`titleStyle` `algn="ctr"` + `sz="4400"`, `bodyStyle` `sz="3200"`, `otherStyle` `sz="1800"`).
  - **Neither fixture has a `p:defaultTextStyle`.**
  - `sample.pptx`'s master has **no `p:txStyles` at all** — keep it that way; it is the absent-tier
    fallback path.

## Verified schema

```xml
<!-- dml-main.xsd — T5 -->
<CT_FontScheme name="…">  majorFont, minorFont : CT_FontCollection,  extLst? </CT_FontScheme>
<CT_FontCollection>  latin, ea, cs : CT_TextFont,  font* : CT_SupplementalFont,  extLst? </CT_FontCollection>
<CT_SupplementalFont script="…" typeface="…"/>

<!-- pml.xsd — T6 -->
<CT_SlideMasterTextStyles>  titleStyle?, bodyStyle?, otherStyle? : a:CT_TextListStyle,  extLst? </CT_SlideMasterTextStyles>
<!-- and on CT_Presentation: -->
<xsd:element name="defaultTextStyle" type="a:CT_TextListStyle" minOccurs="0" maxOccurs="1"/>
```

Theme font references are `+mj-lt` / `+mn-lt` and the `-ea` / `-cs` forms (ECMA Part 1 shows
`<a:latin typeface="+mj-lt"/>` in the `a:fontRef` examples).

## Roadmap

- **T1 enums + measures** ✅ · **T2 character properties** ✅ · **T3 paragraph properties + list
  styles** ✅ · **T3b bullets** ✅ · **T4 the `Presentation` surface** ✅
- **➡ T5 — the theme font scheme.** Model `a:fontScheme` → `FontScheme { name, major, minor }` with
  `FontCollection { latin, ea, cs, supplemental }`, on `Theme` **and** on the interner-free
  `ThemeInfo` (mirror how `ColorScheme` → `SchemeColors` bridges part interners). Then resolve a
  `+mj-lt`-style typeface to the scheme's actual font — the font analogue of a scheme colour, and the
  reason a run naming no font still has one.
- **T6 — effective text formatting.** `effective_run_properties(surface, shape, para, run)` and
  `effective_paragraph_properties(surface, shape, para)`, resolving the ladder below.

### T6's ladder, highest priority first

Each tier contributes only what the tiers above left unset:

1. the run's own `a:rPr`;
2. the paragraph's `a:pPr/a:defRPr`;
3. the shape's `a:lstStyle` at the paragraph's level (`TextBody::list_style`);
4. the **same-slot placeholder's** `a:lstStyle` on each part of `inheritance_chain(surface)` — layout,
   then master — matched with `slide::find_placeholder`;
5. the master's `p:txStyles`: `p:titleStyle` when `Placeholder::is_title_family()`, `p:bodyStyle` for
   body/object slots, `p:otherStyle` otherwise, at the paragraph's level;
6. `p:defaultTextStyle` in `presentation.xml`;
7. the theme's `fontScheme`, for a typeface still naming `+mj-lt`/`+mn-lt` (T5).

**Read the level once, before the walk** — `a:pPr@lvl`, defaulting to `IndentLevel::TOP`. It selects
which `lvlNpPr` every tier from 3 down contributes, which is why a level-2 paragraph that declares
nothing else still answers with the master `bodyStyle`'s `lvl3pPr` bullet, size and indent. That is
exactly what a user sees when they demote a line, and it is why bullets were modeled *before* this.

**Reuse `Presentation::effective_shape_fill`'s structure** — it already does this shape of thing:
build a candidate list (the shape itself, then the same-slot placeholder on each ancestor), resolve
the theme's colour scheme once across the part-interner boundary, then walk. Do not invent a second
pattern.

**Put the merges in `mjx-dml`**, as `CharacterPropertiesSpec::merge_under(&other)` and
`ParagraphPropertiesSpec::merge_under(&other)`: each field takes the lower tier's value only where the
higher tier left it `None`, and **each bullet group merges as a unit** (a tier that sets `buChar`
supplies the whole bullet, not a field of it). One place, unit-tested there, rather than inside the
pptx walk.

**Fixture work T6 needs:** extend `layouts.pptx`'s master `bodyStyle` from its single `lvl1pPr` to
three levels with distinct `buChar`/`sz`/`marL`, so a level-0/1/2 paragraph can be shown to resolve to
*different* answers, each traceable to its `lvlNpPr`. Leave `sample.pptx` without `p:txStyles`.

## Known follow-ups (not blockers)

- **`coalesce_runs`** — runs split but never merge. Only overlapping *different* ranges grow the count,
  so this is an optimization, not a correctness gap.
- **Hyperlinks** (`a:hlinkClick` / `a:hlinkMouseOver`) round-trip but are not modeled: they carry
  `r:id` relationships, so they need the packaging layer, like images did.
- **The underline line/fill groups** (`a:uLn` / `a:uLnTx` / `a:uFill` / `a:uFillTx`) and
  `a:effectDag` are preserved opaquely and not surfaced.
- **`a:buBlip` bullets** carry a relationship id, but nothing creates the image part for one — the
  caller does that with `add_image`, exactly as for a picture fill.
- **`a:br` and `a:fld` stay opaque**, so a line break is not addressable and a slide-number field's
  text is not readable. `Paragraph::text()` skips both.
- **`ST_TextBulletSizePercent` interop**: we write `"111%"` per the schema. If a real-world consumer is
  found that rejects it and demands the integer form, the decision is recorded in §9 above and is a
  one-line change in `build_bullet_size`.

## Guardrails

Standard project rules (`CLAUDE.md`, `PHASE2_HANDOFF.md` §3). The ones this workstream keeps bumping
into:

- **Fidelity first.** Every write test pairs its structural assertion with "every other part is
  byte-identical"; every read test asserts reading dirtied nothing. `txbody_roundtrip.rs` is the
  regression net for the typed text tree — it must stay green untouched.
- **One interner per part**, split-borrow for edits (`let RawDocument { interner, root, .. } = doc`).
  `Presentation::with_text_body` / `edit_text_body` already encapsulate this for text; use them rather
  than re-deriving the borrow dance.
- **Names come from the ECMA-376 Part 1 prose, never guessed.** Two traps this workstream hit:
  underline titles read modifier-first (`dashHeavy` is *"Heavy Dashed"*), and
  `ST_TextAutonumberScheme`'s title column merely repeats the wire token, so those 41 names were
  derived from the **Description** column instead. The reasoning is in `xtask/src/codegen/spec.rs`
  comments.
- **Check the schema before assuming a type.** `buSzPts@val` is an `ST_TextFontSize` (a font size),
  not a text point — the plan had it wrong until the XSD was read.
- No `unwrap`/`panic`/`expect` on untrusted input; pure-Rust only (`unicode-segmentation` is the one
  dependency this workstream added, and it is pure Rust so the cross-compile matrix is unaffected);
  never stage `References/`.
- **Every PR bumps the patch version and adds a `CHANGELOG.md` entry** as its last commit.
- Commits split by concern (xtask / generated / dml / pptx / tests / docs).

## Where to look

`crates/mjx-dml/src/text/` (`character.rs`, `paragraph_properties.rs`, `bullet.rs`, `list_style.rs`,
`font.rs`, `run.rs`, `paragraph.rs`, `body.rs`), `crates/mjx-dml/src/theme.rs` (where T5 goes),
`crates/mjx-dml/src/resolve.rs` (`resolve_character_properties`; T6's merges go beside it),
`crates/mjx-dml/src/geometry/measures.rs` (`FontSize`, `TextPoint`, `IndentLevel`),
`crates/mjx-pptx/src/presentation.rs` (the text surface, `with_text_body`/`edit_text_body`, and
`effective_shape_fill` as T6's template), and the tests named above.
