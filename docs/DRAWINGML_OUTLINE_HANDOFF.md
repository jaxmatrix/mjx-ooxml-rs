# Handoff — PowerPoint shape outline (`a:ln`) — COMPLETE

A self-contained record of the shape-outline workstream. Read after `docs/DRAWINGML_FILL_HANDOFF.md`
and `docs/DRAWINGML_EFFECTIVE_FILL_HANDOFF.md` — outline is the **line analog** of the fill story and
mirrors it PR-for-PR.

## The goal

`shape_outline(slide, shape)` returns a shape's **explicit** `p:spPr > a:ln` (`CT_LineProperties`), or
`None`. `effective_shape_outline(slide, shape)` returns the outline the shape actually **renders**,
resolved by inheritance to a concrete `RRGGBB` stroke:

```
shape p:spPr > a:ln                                          (explicit)
  else shape p:style > a:lnRef(@idx, color)
     → theme fmtScheme > lnStyleLst[idx]   (1-based; 0 = none)
     → the theme line's <schemeClr val="phClr"/> stroke replaced by the lnRef's own color
     → schemeClr resolved via clrMapOvr→clrMap then clrScheme → concrete RGB
  else, for a placeholder (p:ph), inherit from the same-typed shape on slideLayout → slideMaster
```

The public API is **interner-free** (`LineSpec`, like `FillSpec`); the stroke reuses the fill model
(`Fill`/`FillSpec` — a line fill is a subset of `EG_FillProperties`, no blip/group).

## Roadmap — 4 atomic PRs, all merged

- **O1 — generated line simple types (#33).** `mjx-ooxml-types::drawingml`: `LineCap`, `CompoundLine`,
  `PenAlignment`, `PresetLineDash`, `LineEndType`, `LineEndWidth`, `LineEndLength` (codegen allowlist +
  spec.rs naming overrides; `in`→`Inset` also dodges the Rust keyword).
- **O2 — `mjx-dml::line` model (#34).** `LineProperties` fidelity wrapper over `a:ln` + interner-free
  `LineSpec` (`spec`/`to_line`); helper values `LineDash{Preset,Custom}`, `LineJoin{Round,Bevel,Miter}`,
  `LineEnd{kind,width,length}`; the `LineWidth` EMU measure (in `geometry::measures`). custDash/extLst
  stay opaque (byte-exact on the wrapper; dropped by the value tier).
- **O3 — `mjx-pptx` explicit (#35).** `shape_outline` / `set_shape_outline` / `set_shape_no_outline`
  (writes `<a:ln><a:noFill/></a:ln>`); `slide::shape_line` + `line_child_index`/`line_insert_index` +
  `AFTER_LINE_LOCALS` (= `AFTER_FILL_LOCALS` without `"ln"`) — `a:ln` inserts after fill, before
  effects/3-D/extLst. office-open canary `deck_with_outlined_shape_opens`.
- **O4 — effective outline (this PR).** Theme `lnStyleLst` (`Theme::line_style(idx)` +
  `ThemeInfo::line_style`, `line_styles_of` parser); `resolve_line` (copies structural attrs, bakes the
  stroke via `resolve_fill`); `slide::shape_line_ref` (`p:style > a:lnRef`); `effective_shape_outline`
  reusing the `Candidate` chain + a new `OwnLine`/`shape_own_line` (mirror of `OwnFill`/`shape_own_fill`).

## Verified schema / fixture facts

- `CT_LineProperties` attrs `@w`(EMU)/`@cap`/`@cmpd`/`@algn`; ordered children fill choice
  (`noFill`/`solidFill`/`gradFill`/`pattFill`) → dash (`prstDash`/`custDash`) → join
  (`round`/`bevel`/`miter@lim`) → `headEnd`/`tailEnd` → `extLst`.
- `CT_StyleMatrixReference` (`a:lnRef`) reuses `StyleMatrixReference` (already parsed `a:lnRef`).
- Fixture `theme1.xml` `a:lnStyleLst` = 3× `<a:ln w="{6350|12700|19050}"><a:solidFill><a:schemeClr
  val="phClr"/></a:solidFill></a:ln>`. accent1 `4472C4`, accent2 `ED7D31`. Slide 0 shape 0 is a
  `ctrTitle` placeholder with no `a:ln`/`p:style`.

## Guardrails

Fidelity-first (O4 read paths never write theme/style/map parts; reading effective outline is
byte-non-dirtying). Names spec-sourced. One interner per part; each part borrow fully consumed into owned
`LineSpec`/`OwnLine` before the next. Resolved colors value-pinned against the fixture theme
(`4472C4`/`ED7D31`), not claimed Office-pixel-exact. **The shape-outline workstream is complete.**
