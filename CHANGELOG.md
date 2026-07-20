# Changelog

All notable changes to **mjx-ooxml-rs** are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Versioning

The project is pre-release and uses `v0.0.x`: the patch number is incremented each development
iteration until the first milestone. Milestones then advance the minor version:

- **`v0.1`** — PowerPoint (`.pptx`) complete
- **`v0.2`** — Word (`.docx`) complete
- **`v0.3`** — Excel (`.xlsx`) complete

Further milestones (rendering, bindings, …) are defined as that work is scheduled. The public API is
**not** stable until `v0.1`.

## [0.0.6] - 2026-07-21

Text formatting reaches the deck. Everything the previous four releases modeled is now callable on a
real `.pptx`, at every scope a user can select.

### Added

- **The paragraph axis** on `Presentation` — `paragraph_count`, `run_count`, `paragraph_text`,
  `run_text`. Run indices are paragraph-local, matching the document tree. The existing flat
  `set_shape_text` is unchanged.
- **Reading formatting** — `paragraph_properties`, `run_properties`, `end_run_properties`. Reading
  never dirties a part.
- **Writing formatting, one call per selection granularity**:
  - `set_run_properties` — one run.
  - `set_paragraph_run_properties` — every run in a paragraph, and its paragraph mark.
  - `set_shape_run_properties` — every run in the shape, and every mark.
  - `set_text_range_properties` — an arbitrary character range, splitting runs where the range cuts
    across them.
  - `set_text_range_properties_by_grapheme` — the same, addressed in grapheme clusters, so an emoji
    and its modifier are one unit.
  - `set_paragraph_properties` — a paragraph's layout (alignment, level, margins, spacing, bullet).
  - `set_end_run_properties` — the format of an **empty** paragraph, which is what a placeholder
    added but not yet typed into holds.
- **`TextRun::split_at` / `Paragraph::split_run_at`** in `mjx-dml` — divide a run's text, giving both
  halves the original's formatting, so splitting alone changes nothing about how the text renders.
- **`Paragraph::set_end_properties`** — the write half of the `a:endParaRPr` surface.

### Notes

- Formatting a paragraph or a shape also formats the paragraph mark, so text typed at the end takes
  the same formatting — what "select and restyle" means to a user.
- Runs are split but never merged, keeping each edit minimal. A range already aligned to run
  boundaries splits nothing, so repeated edits do not accumulate runs.

## [0.0.5] - 2026-07-21

Bullets and numbering — the marks that express a deck's paragraph hierarchy.

### Added

- **`Bullet`** — what marks a paragraph: `None` (an explicit "no bullet", which overrides an
  inherited one), `Character` (a literal glyph), `AutoNumber` (a scheme plus where its sequence
  starts), or `Picture` (an image by relationship id).
- **`BulletColor`, `BulletSize`, `BulletTypeface`** — the bullet's colour, size and font, each with a
  `FollowText` variant for the schema's "match the text" arm. All four groups are set and inherited
  **independently**, as the schema defines them.
- **Builder support** on `ParagraphPropertiesSpec`: `with_bullet`, `with_bullet_color`,
  `with_bullet_size`, `with_bullet_typeface`, plus `with_bullet_character("•")` and
  `without_bullet()` for the common cases.

### Notes

- A bullet percentage is written in the form both schemas specify and ECMA §21.1.2.4.9 illustrates
  (`val="111%"`); the integer spelling found in some files is still read.
- Setting one bullet group never disturbs the others, and a group left unnamed keeps whatever the
  file had.

## [0.0.4] - 2026-07-21

Paragraph formatting: how a paragraph is laid out, and the per-level styles it inherits from.

### Added

- **`ParagraphProperties`** (`CT_TextParagraphProperties`) — indent level, alignment, left/right
  margins, first-line indent, default tab size, reading direction and font alignment, plus line
  spacing, space before/after, tab stops, and the `a:defRPr` a paragraph's runs default to. One type
  serves `a:pPr`, `a:defPPr` and `a:lvl1pPr`…`a:lvl9pPr`; the line-breaking attributes, bullets and
  anything unknown round-trip verbatim.
- **`ParagraphPropertiesSpec`** — the builder, matching the character-properties conventions.
  Margins, indents and tab stops are stated **in points**; EMU is the file's unit and stays reachable
  through `Emu`.
- **`IndentLevel`** — the 0–8 nesting level a paragraph's inherited bullet, size and indent are
  selected by. `IndentLevel::of(2)` for a literal, `::new(raw)` for a value off the wire, `::TOP` for
  the outermost.
- **`TextSpacing`** — a proportion of the line height (`a:spcPct`) or a fixed distance (`a:spcPts`),
  kept apart because they are different measurements. **`TabStop`** — position and alignment.
- **`TextListStyle`** (`a:lstStyle`) — the paragraph properties a container offers at each level, by
  `level(IndentLevel)`. The same type covers a shape's own list style, a placeholder's, and each of a
  master's three text styles.
- **Typed access from the text tree** — `Paragraph::properties` / `set_properties` and
  `TextBody::list_style`, so `a:pPr` and `a:lstStyle` are no longer opaque.

## [0.0.3] - 2026-07-20

Text formatting begins: the vocabulary and the run-level model. A run's appearance — its size, weight,
slant, underline, colour, font — can now be read and written. (Reaching it through a `Presentation`,
and resolving what a run *inherits*, come next.)

### Added

- **Text simple types** — `TextUnderline`, `TextStrike`, `TextCapitalization`, `TextAlignment`,
  `FontAlignment`, `TabAlignment` and `AutonumberScheme` (41 bullet-numbering schemes), generated from
  `dml-main.xsd` and named from the ECMA-376 §20.1.10 enumeration tables.
- **`FontSize` and `TextPoint`** — text measures stated **in points** (`from_points` / `points`), the
  unit every size control uses. The file's hundredths of a point are reachable only through
  `from_wire` / `to_wire`.
- **`CharacterProperties`** (`CT_TextCharacterProperties`) — size, bold, italic, underline, strike,
  capitalization, spacing, kerning, baseline, language, plus the text fill, glyph outline, effects,
  highlight and the four script fonts. One type serves `a:rPr`, `a:defRPr` and `a:endParaRPr`, and
  everything it does not model — hyperlinks, `dirty`/`err`/`smtClean`, unknown children — round-trips
  verbatim.
- **`CharacterPropertiesSpec`** — an interner-free builder:
  `CharacterPropertiesSpec::new().with_size_points(28.0).with_bold(true).with_color(…)`. Naming a
  property sets it; leaving it unnamed means *inherit*, so `with_bold(false)` and
  `with_underline(TextUnderline::None)` are how a caller overrides an inherited value.
- **`TextFont`** — a typeface reference, whether a literal name or a `+mj-lt`-style theme reference.
- **`resolve_character_properties`** — bakes a run's colours (text fill, glyph outline, effects,
  highlight) down to concrete RGB against a theme scheme and colour map.
- **Typed access from the text tree** — `TextRun::properties` / `set_properties` and
  `Paragraph::end_properties`, so `a:rPr` and `a:endParaRPr` are no longer opaque.

### Notes

- Setting a run's properties **merges** onto its existing `a:rPr` rather than replacing it, so the
  state this model does not describe (`lang`, `dirty`, a hyperlink) survives a restyle. An unset
  property means "leave it alone", never "clear it".

## [0.0.2] - 2026-07-20

The PowerPoint slice — Phases 2 and 3. A real `.pptx` can now be opened, read, edited, built up from
its own layouts and pruned back down, and written out so PowerPoint and LibreOffice open it with every
untouched part byte-identical. Phase 3 closes here; Word (Phase 4) is next.

### Added

- **De/serialization (Phase 2)** — `FromXml`/`ToXml` in `mjx-ooxml-core::convert` and the
  `#[derive(FromXml, ToXml)]` proc-macro in `mjx-derive`. Every modeled type keeps an unknown-content
  bucket, so what we do not model survives a round trip.
- **DrawingML text (Phase 2)** — `mjx-dml`'s `TextBody`/`Paragraph`/`TextRun`/`Text`, with a mutation
  surface.
- **PresentationML (Phase 2)** — `mjx-pptx::Presentation`: `open`/`save`, slide inventory, shape
  enumeration, `shape_text`/`set_shape_text`, and construction — `add_text_box`, `add_shape`,
  `add_slide`. The **office-open canary** (LibreOffice headless must render the produced deck to a
  valid PDF) became a CI gate.
- **Preset geometry (Phase 3)** — all 187 `ST_ShapeType` values generated, and the 117 adjustable
  shapes given **named, spec-sourced control parameters** (a rounded rectangle exposes
  `corner_radius`, never `adj1`), with the meaning derived from `presetShapeDefinitions.xml`.
- **Color, theme and the `spPr` visual trilogy (Phase 3)** — theme (`clrScheme`/`fmtScheme`) with
  color resolution to concrete RGB, and **fill**, **outline** (`a:ln`) and **effects**
  (`a:effectLst`), each modeled both *explicitly* and *effectively* — resolved through style
  references and placeholder inheritance to what actually renders.
- **Images (Phase 3)** — `add_image` media parts (de-duplicated by content, format identified by
  magic bytes), `add_picture` `p:pic` shapes, and picture read/replace — on one shape index space
  covering every shape kind.
- **Layouts and masters (Phase 3)** — the layout/master inventory, generated PresentationML simple
  types, **`Surface` addressing** (every shape call works on a slide, a layout or a master, so editing
  a layout reaches every slide inheriting it), and `add_slide_from_layout`, which returns a slide
  carrying the layout's placeholders ready to fill.
- **Removal (Phase 3)** — `remove_shape` on any surface, and `remove_slide`, which unwires
  `p:sldIdLst` → relationship → part and takes with it every part only that slide referenced (its
  notes slide, unshared media) while sparing anything the rest of the deck still uses.
- **Packaging** — `Package::{insert_part, remove_part, remove_part_cascading,
  set_content_type_default/override, add_relationship, remove_relationship}` over a copy-on-write part
  body, plus `PartName::{resolve, resolve_from_root, relative_target}` — the part-name algebra Word
  and Excel will share.

### Fixed

- `add_shape` / `add_text_box` built a paragraph with no run, so the shape they returned could not be
  filled by `set_shape_text`. Every paragraph they create now holds exactly one run, blank lines
  included.
- `add_slide_from_layout` cloned the date, footer and slide-number placeholders. Those render *from
  the layout* for slides that do not declare them, so the clones suppressed the layout's rendering and
  showed as empty boxes; they are now skipped, as PowerPoint does.

### Notes

- The round-trip contract is unchanged and continuously asserted: per-part decompressed-payload byte
  identity plus structural container identity. Reading dirties nothing; an edit re-serializes only its
  own part.
- Public API remains unstable until `v0.1`.

## [0.0.1] - 2026-07-15

First versioned snapshot. Establishes the workspace, the packaging + fidelity + compatibility core,
the schema-type generator, and full documentation. No format models yet.

### Added

- **Packaging (Phase 0)** — `mjx-opc`: load an OOXML package fully into RAM as an ordered part graph,
  parse `[Content_Types].xml` and `_rels/*.rels`, and re-zip with per-part decompressed-byte identity.
  Minimal namespace-resolving reader in `mjx-xml`.
- **Schema codegen (Phase 0)** — `xtask` generates `mjx-ooxml-types` (namespace table +
  `shared-commonSimpleTypes`) with comprehensive, self-explanatory names and exact wire tokens;
  output is deterministic and committed.
- **Fidelity layer (Phase 1)** — `mjx-ooxml-core` string interner + the `RawDocument` preservation
  tree, and `mjx-xml::fidelity`, a byte-preserving reader + hand-written writer. Parsing then
  re-serializing any part reproduces the source **byte-for-byte** (verified on real `.pptx`/`.docx`/
  `.xlsx` fixtures).
- **Markup Compatibility (Phase 1)** — `mjx-mce`: preserve mode (the untouched tree) and a
  non-mutating resolve mode (`AlternateContent` Choice/Fallback, `Ignorable`, `ProcessContent`,
  `MustUnderstand`).
- **Documentation** — comprehensive rustdoc across all crates (crate guides + runnable examples), a
  facade docs hub (`mjx-ooxml`), enforced via `missing_docs` and a strict-rustdoc CI job.
- **Project** — CI (fmt/clippy/test + wasm/Android/iOS/macOS/Windows cross-compile build matrix),
  dual `MIT OR Apache-2.0` license, and the contributor/agent guides.

### Notes

- Cross-platform: pure-Rust dependency graph; the library crates cross-compile to
  `wasm32-unknown-unknown`, `aarch64-linux-android`, and Apple/Windows targets.
- A broader multi-producer sample corpus and fuzzing are planned for later iterations.

[0.0.6]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.6
[0.0.5]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.5
[0.0.4]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.4
[0.0.3]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.3
[0.0.2]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.2
[0.0.1]: https://github.com/jaxmatrix/mjx-ooxml-rs/releases/tag/v0.0.1
