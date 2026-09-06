# Design tokens — the Allr system, and what an editor needs added to it

The client platform is integrated into **[allr.work](https://allr.work)**, so its visual system is
the source, not an inspiration. This document records the tokens as they actually are — read from the
site's own stylesheet, not eyeballed from a screenshot — and specifies the three things an Office
editor needs that a marketing site does not: a dark theme, a document-surface palette, and semantic
aliases.

---

## 1 · The source, as measured

Extracted from `allr.work/_next/static/chunks/1d9upyu2uoml7.css`. The site is Tailwind v4 with a
custom theme layer; these are its declared custom properties, verbatim.

### Colour

| Token | Value | Role on the site |
|---|---|---|
| `--color-paper` | `#fdfcf9` | page background — warm off-white |
| `--color-card` / `--color-white` | `#ffffff` | raised surfaces |
| `--color-ink` | `#223b33` | primary text — a deep, desaturated green-black |
| `--color-ink-soft` | `#5c7168` | secondary text |
| `--color-line` | `#e7e0d2` | borders — warm beige, not grey |
| `--color-line-soft` | `#efe9dc` | subtle dividers |
| `--color-green` | `#2e9e63` | primary accent |
| `--color-green-deep` | `#1e7a49` | accent, pressed / on-light text |
| `--color-green-line` | `#c2e5d0` | accent border |
| `--color-green-tint` | `#e4f4ea` | accent surface |
| `--color-honey` | `#e9a83e` | secondary accent |
| `--color-honey-deep` | `#b77e1f` | secondary accent, deep |
| `--color-honey-line` | `#f0dcb4` | secondary border |
| `--color-honey-tint` | `#fbefd8` | secondary surface |
| `--color-sage-line` | `#dce8dd` | tertiary border |
| `--color-sage-tint` | `#ecf2ec` | tertiary surface |
| `--color-clay-tint` | `#f6ede2` | warm neutral surface |

The whole palette is **warm and organic** — the neutrals are beige rather than grey and the "black"
is a green. That is a deliberate character and the editor must not flatten it into generic grey
chrome.

### Type

| Token | Value |
|---|---|
| `--font-sans` | `"Nunito Sans", system-ui, sans-serif` |
| `--font-serif` | `"Young Serif", serif` |
| `--font-mono` | `ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", …` |
| weights | `medium 500`, `semibold 600`, `bold 700`, `extrabold 800` |
| `--text-xs` / `--text-sm` | `0.75rem` / `0.875rem` |
| `--leading-tight/snug/relaxed` | `1.25` / `1.375` / `1.625` |
| body line-height | `1.7` |
| `--tracking-tight` | `-0.025em` |

### Shape, depth and motion

| Token | Value |
|---|---|
| `--radius-chip` | `8px` |
| `--radius-control` | `10px` |
| `--radius-card` | `16px` |
| `--radius-panel` | `20px` |
| `--radius-frame` | `22px` |
| `--radius-phone` | `36px` |
| `--shadow-lift` | `0 18px 48px #223b331c` — tinted with ink, never neutral black |
| `--spacing` | `0.25rem` base step |
| `--ease-ink` | `cubic-bezier(.45, 0, .2, 1)` |
| `--ease-out-soft` | `cubic-bezier(.22, 1, .36, 1)` |
| `--ease-spring` | `cubic-bezier(.34, 1.56, .64, 1)` |
| default duration | `0.15s` |

---

## 2 · Three things the source does not provide

### 2.1 · There is no dark theme

The stylesheet contains **no** `prefers-color-scheme` block, no `.dark` class and no `[data-theme]`
selector. An Office editor needs one — people write at night, and a bright chrome around a white page
is fatiguing in a way a marketing page never is.

Derived by holding the source's hue relationships and inverting lightness, so the dark theme reads as
the same brand rather than a different product:

| Semantic | Light | Dark (derived) |
|---|---|---|
| app background | `#fdfcf9` | `#131a17` |
| surface | `#ffffff` | `#1b2420` |
| surface raised | `#ffffff` | `#23302a` |
| text primary | `#223b33` | `#e8efe9` |
| text secondary | `#5c7168` | `#9aaba2` |
| border | `#e7e0d2` | `#2e3b35` |
| border subtle | `#efe9dc` | `#26312c` |
| accent | `#2e9e63` | `#4fc98a` |
| accent deep / pressed | `#1e7a49` | `#2e9e63` |
| accent surface | `#e4f4ea` | `#163023` |
| accent border | `#c2e5d0` | `#224934` |
| secondary accent | `#e9a83e` | `#f0be62` |
| secondary surface | `#fbefd8` | `#33260f` |

### 2.2 · Contrast rules the source never had to satisfy

A marketing page uses accent colour decoratively. An editor uses it for state that must be legible.
Measured against white:

- `--color-green` `#2e9e63` → **3.39 : 1**. Passes WCAG AA for UI components and large text; **fails**
  for body text.
- `--color-green-deep` `#1e7a49` → **5.34 : 1**. Passes AA for body text.

So the rule is: **`green` for fills, borders, indicators and icons; `green-deep` for any accent-coloured
text on a light surface.** Getting this backwards is the most likely accessibility defect in the
chrome.

**This is no longer advice — it is a build failure.** Every colour token in the source carries a
`usage` tag in its `$extensions.mjx` block, and the generator refuses to emit if a token tagged for
text does not clear 4.5 : 1 against the background it declares. Three values:

| `usage` | Means | Checked against |
|---|---|---|
| `on-light-text` | legible as body text on a light surface | ≥ 4.5 : 1 vs its declared light background |
| `on-dark-text` | legible as body text on a dark surface | ≥ 4.5 : 1 vs its declared dark background |
| `fill-only` | fills, borders, indicators, icons — **never glyphs** | not contrast-checked as text |

`on-dark-text` exists because §2.1's derived dark palette needs the same guarantee, and measuring only
the light side would have left half the tokens unchecked.

**⚠ The honey ramp has no text-legal step, and this was not known when §2.2 was first written.**
`--color-honey-deep` `#b77e1f` measures **3.49 : 1** on white — below the body-text minimum. Unlike
green, which has `green-deep` at 5.34 : 1, honey has no deeper step in the Allr source. So honey and
**both tracked-change colours** are `fill-only`: a tracked change may colour its change bar, its
underline or its margin marker, but **not its glyphs**. Colouring tracked-change *text* requires a
deeper honey step the source does not define — an open design decision, not one to invent.

### 2.3 · The document surface is not the app surface

An editor has two palettes, and conflating them is the classic mistake.

**The page stays true white (`#ffffff`) in both themes.** A document is white paper; a warm-tinted or
dark page changes what the author sees relative to what they will print or send. What changes with the
theme is the *canvas backdrop* behind the page, the page shadow, and the in-canvas UI.

This is also why the warm chrome works so well here: `--color-paper` `#fdfcf9` behind a true-white
page makes the page read as a distinct, lit object rather than dissolving into the background — which
is exactly the figure/ground separation a document editor wants. In dark mode the backdrop drops to
`#131a17` and the effect is stronger still.

| Document-surface token | Light | Dark |
|---|---|---|
| canvas backdrop | `#fdfcf9` | `#131a17` |
| page | `#ffffff` | `#ffffff` |
| page border | `#e7e0d2` | `#2e3b35` |
| page shadow | `0 2px 10px #223b3314` | `0 2px 14px #00000059` |
| selection fill | `#2e9e63` @ 18% | `#4fc98a` @ 22% |
| selection handle | `#1e7a49` | `#4fc98a` |
| alignment guide | `#e9a83e` | `#f0be62` |
| grid / ruler line | `#efe9dc` | `#26312c` |
| comment anchor | `#e9a83e` | `#f0be62` |
| tracked-change insert / delete | `#2e9e63` / `#b77e1f` | `#4fc98a` / `#f0be62` |

A **reading-mode inversion** for the page itself is a user setting, not a theme consequence.

---

## 3 · One source, three consumers

The chrome is HTML and the document canvas is Rust, and a canvas cannot inherit a CSS custom
property. So the token pipeline emits three artefacts from one source — the project's existing
codegen doctrine: an `xtask` generator, output committed, never a `build.rs`.

```
docs/client-platform/data/tokens.json          (W3C Design Tokens format — the source)
        │   cargo run -p xtask -- tokens        (--check verifies; --out-dir writes elsewhere)
        │
        ├── ui/tokens/tokens.css               — custom properties for the Web-Component chrome
        ├── ui/tokens/tokens.ts                — typed constants, the Tokens type, customProperties
        └── crates/mjx-tokens/src/generated.rs  — a `Tokens` struct and const default table
```

All three are **generated and committed**; the source is the only file to edit. Two gates hold them
together: `xtask/tests/tokens.rs` proves they are *derived* from the source, and `mjx-tokens`'s
`artefacts_agree` suite proves they *agree with each other* by parsing the emitted CSS and TypeScript
from disk and comparing every value against the Rust table.

At runtime the shell resolves in this order — **explicit host configuration → CSS custom properties
read off the host element → built-in defaults** — and pushes the resolved snapshot across the bridge
to the renderer, so a theme change repaints canvas and chrome in the same frame. The shell watches
for host changes with a `MutationObserver` and `prefers-color-scheme`.

That resolution order is what satisfies the original brief: *if tokens are set, adopt them.* Dropping
the platform into a host that already defines these properties re-themes it with no code change; the
Allr values are simply the defaults it ships with.

---

## 4 · Notes for the chrome

- **Radii are large** (`10px` controls, `16px` cards, `20px` panels). Toolbars and inspectors should
  keep that softness rather than reverting to the 2–4px corners Office uses; it is the most
  recognisable part of the source's character.
- **Shadows are ink-tinted**, never neutral black. `--shadow-lift` is `#223b331c`.
- **Nunito Sans is a rounded humanist face** and reads slightly wider than a typical UI font. Dense
  surfaces — the formula bar, the cell grid, a properties inspector — should use `--text-xs`
  (`0.75rem`) with `--leading-tight`, not the site's relaxed `1.7`.
- **`--ease-spring` overshoots** (`cubic-bezier(.34, 1.56, .64, 1)`). Good for a panel or sheet
  entering; wrong for anything attached to a document object, where overshoot reads as imprecision.
  Use `--ease-ink` for selection, handles and canvas motion.
- **Young Serif is display-only.** It belongs in empty states and onboarding, never in the chrome or
  in document content.
