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
- **PR-3 — Color resolver (full concrete RGB). Split into 3a + 3b.**
  - **PR-3a — base resolution. ✅ DONE.** `mjx-dml::resolve`: `ResolvedColor { red,green,blue,alpha }`
    (`to_hex()`); `SchemeColors` (interner-free slot→RGB, built via `SchemeColors::from_scheme(
    &ColorScheme, &Interner)` — bridges the theme-part vs shape-part interner boundary);
    `resolve_color(&Color, &SchemeColors, &ColorMap, placeholder: Option<&Color>, &Interner) ->
    Option<ResolvedColor>` resolving `srgbClr`/`sysClr`(lastClr)/`scrgbClr`(linear→sRGB)/`hslClr`/
    `prstClr` (**190-color table**)/`schemeClr` (map→slot; `phClr`→placeholder recursion). Lifted
    `parse_percentage`/`parse_angle` into `crate::build`. **Contract:** a color carrying any transform
    child resolves to `None` (deferred to 3b).
  - **PR-3b — color transforms. ✅ DONE.** `resolve.rs`: `apply_transforms` applies the full
    `EG_ColorTransform` set (HSL `lum*`/`sat*`/`hue*` via `rgb_to_hsl`/`hsl_to_rgb_f64`; linear-RGB
    `shade`/`tint` via `srgb_to_linear`/`linear_to_srgb`; `alpha*`; per-channel `red*`/`green*`/
    `blue*`; `inv`/`gray`/`comp`/`gamma`/`invGamma`) **per level** of the chain (`resolve_rgba` recurses
    for `phClr`/scheme; `SchemeColors::from_scheme` honors slot transforms). `ResolvedColor.alpha` is
    now real. Common ops value-pinned (`shade 50%` white→`BCBCBC`, `lumMod 50%` red→`800000`,
    `lumMod60+lumOff40`→`FF6666`); `comp`/`gray`/`gamma` are documented-interpretation, not Office-exact.
    **The color resolver is complete.**
- **PR-4 — `effective_shape_fill`. Split into 4a + 4b.**
  - **PR-4a — shape-level effective fill. ✅ DONE.** `resolve_color`'s placeholder became an
    interner-free `ResolvedColor` (the theme fill-style and a shape's `fillRef` color live in different
    interners). New `resolve_fill(&Fill, &SchemeColors, &ColorMap, Option<ResolvedColor>, &Interner) ->
    FillSpec` bakes every color of a fill. `mjx-pptx`: re-added `slide::shape_fill_ref`; factored
    `slide_theme_part`; public `Presentation::effective_shape_fill(slide, shape) -> Option<FillSpec>`
    composing **explicit `p:spPr` fill** and **`p:style > a:fillRef` (theme fill-style + phClr
    substitution)**, resolved to concrete `RRGGBB` (each source under its own part borrow; interner-free
    `SchemeColors`/`ColorMap`/`ResolvedColor` carried across). `None` when the shape declares neither.
  - **PR-4b — placeholder inheritance. ✅ DONE.** `slide::Placeholder { title_family, idx }` +
    `shape_placeholder` / `matches` / `find_placeholder` (title-family match; else by `idx` — a
    documented heuristic). `effective_shape_fill` now walks an owned candidate chain (slide shape →
    layout-matched → master-matched `p:ph`), resolving each candidate's own fill via `shape_own_fill`
    (`OwnFill::{Resolved, StyleRef, Absent}`) and returning the first hit. Tested by injecting a
    `ctrTitle` placeholder-with-fill into the fixture layout in-test (no new binary fixture) →
    inheritance resolves to `accent2` (`ED7D31`). **The effective-fill workstream is complete.**

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
