# Handoff — PowerPoint effective (default) fill resolution

A self-contained brief to resume this workstream from a cold start. Read after
`docs/DRAWINGML_FILL_HANDOFF.md` (the explicit-fill work this builds on) and `docs/PHASE2_HANDOFF.md`.

## The goal

`shape_fill(slide, shape)` returns the shape's **explicit** `p:spPr` fill, or `None` when it declares
none. A shape with no explicit fill still renders a fill, resolved by inheritance:

```
shape p:style > a:fillRef(@idx, color)
   → theme fmtScheme > fillStyleLst[idx]        (1-based; idx 0 = no fill)
   → every <a:schemeClr val="phClr"/> replaced by the fillRef's own color
   → other schemeClr (bg1/accent1/…) resolved via clrMapOvr→clrMap (bg/tx→dk/lt) then clrScheme
   → sysClr/srgbClr → final RGB, after applying color transforms
— OR, when the shape is a placeholder (p:ph), inherited from the same-typed shape on the
  slideLayout → slideMaster.
```

**Two settled decisions:** the public API is **interner-free** (like `ShapeGeometry`/`FillSpec` — a
`Color`/`Fill` value can't cross interners); color resolution targets **full concrete RGB** (implement
the color-transform math, bake to a single hex; Office-pixel-exactness is *not* claimed — that belongs
to rendering — pin expected values from a trusted reference).

## Roadmap (4 atomic PRs)

- **PR-1 — Theme model + navigation. ✅ DONE (this branch).**
  - Generated `ColorSchemeSlot` (`ST_ColorSchemeIndex`, 12 tokens) in `mjx-ooxml-types::drawingml`.
  - `mjx-dml::theme`: interner-bound `Theme` / `ColorScheme` (12 slots → `Color`) / fill-style matrix
    (`fmtScheme > fillStyleLst` → `Vec<Fill>`), read-views with `FromXml`; plus the interner-free
    `ThemeInfo` (`Theme::to_info(&Interner)` → `(slot, ColorSpec)` pairs + `Vec<FillSpec>`).
  - `mjx-pptx`: constants `REL_SLIDE_MASTER`/`REL_THEME`/`CONTENT_TYPE_THEME`; a shared
    `Presentation::follow_rel(part, rel_type)` hop; `Presentation::slide_theme(idx) ->
    Option<ThemeInfo>` walking slide→layout→master→theme.
  - The **interner-bound `Theme`** is retained (public in `mjx-dml`) as the color resolver's input for
    PR-3/PR-4 — it keeps color transforms and opaque fill internals that `ThemeInfo` drops.
- **PR-2 — Shape style ref + color map. ✅ DONE.**
  - `mjx-dml::style`: `StyleMatrixReference { idx: Option<u32>, color: Option<Color> }` (`a:fillRef`
    etc.; `FromXml` parsing `@idx` + `first_color_child`); `ColorMap` value-object (pub fields
    `background1`/`text1`/… → `ColorSchemeSlot`) with `identity()` and `resolve(SchemeColor) ->
    Option<ColorSchemeSlot>` (map for `bg/tx/accent/hlink`, direct for `dk/lt`, `None` for `phClr`).
  - `mjx-pptx`: `Presentation::slide_color_map(idx) -> Option<ColorMap>` (public) — master `p:clrMap`
    via slide→layout→master, overridden by the slide's `p:clrMapOvr` (falls back to master on a
    `masterClrMapping` / absent / attribute-less override); `nav::attr_value` + `slide::parse_color_map`.
  - **Deferred to PR-4:** reading `p:sp > p:style > a:fillRef` (`shape_fill_ref`) lands where it is
    consumed (in `effective_shape_fill`), keeping its interner-bound `Color` internal.
- **PR-3 — Color resolver (full concrete RGB).**
  - `mjx-dml`: `resolve_color(&Color, &ColorScheme, &ColorMap, placeholder: Option<&Color>) ->
    Option<ResolvedColor>` — base resolution (srgb/sys/scheme, phClr substitution, bg/tx map) + the
    full `EG_ColorTransform` set (`lum*`/`sat*`/`hue*`/`shade`/`tint`/`alpha*`/`comp`/`inv`/`gray`/
    channel mods/`gamma`). Parse transforms from `Color::transforms()`. Pin test values from a trusted
    reference (LibreOffice / python-pptx).
- **PR-4 — Placeholder inheritance + `effective_shape_fill`.**
  - `mjx-pptx`: `p:ph` model (`@type`/`@idx`), a slide→layout→master walker matching the same-typed
    shape; `Presentation::effective_shape_fill(slide, shape) -> Option<FillSpec>` composing
    explicit-fill → style-fillRef+theme → placeholder inheritance.

## Verified schema facts

- `a:theme > a:themeElements > { a:clrScheme (CT_ColorScheme, 12 CT_Color slots), a:fontScheme,
  a:fmtScheme (CT_StyleMatrix) }`. `a:fmtScheme > a:fillStyleLst` = 3+ `EG_FillProperties`.
- `CT_StyleMatrixReference` (`a:fillRef`): `@idx` = `unsignedInt` (1-based into `fillStyleLst`;
  0 = none), one optional `EG_ColorChoice` child = the phClr substitute.
- `ST_SchemeColorVal` includes `phClr`; the color map maps `bg1/tx1/bg2/tx2` → `dk1/lt1/dk2/lt2`.
- Master→theme rel type `…/relationships/theme`; theme content type
  `application/vnd.openxmlformats-officedocument.theme+xml`. Fixture chain: `slide1 → slideLayout1 →
  slideMaster1 (rId2) → theme1.xml`.
- Fixture `theme1.xml` (standard "Office"): `dk1`/`lt1` = `sysClr windowText/window`; accents/dk2/lt2
  = `srgbClr` (accent1 `4472C4`); `fillStyleLst` = 3× `solidFill` of `schemeClr phClr`.

## Guardrails

Fidelity-first (all read-only — nothing here writes theme/style/map parts). Names spec-sourced. One
interner per part; no `unwrap`/`panic`/`expect` on parse paths. Attribute namespaces unresolved
(`p:clrMap` attrs, `a:fillRef@idx` are unprefixed; `r:id` hops reuse `nav::namespace_prefix`). Match
elements by `(namespace, local)` both-URI. Branch per PR off `main`; atomic commit when green; no
AI-attribution trailer; never stage `References/`. **PR-3 color-transform math is the deepest risk —
pin expected values, document the algorithm, don't claim Office-exact rendering.**
