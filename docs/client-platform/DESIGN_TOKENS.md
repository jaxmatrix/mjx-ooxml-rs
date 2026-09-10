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

> **⚠ This section is the measurement, not the shipped palette.** MJXOFF-271 re-seeded the tokens
> from the *product* — `allr-agent/apps/hermes-universal`, the application this platform is embedded
> in — rather than from the marketing site, so that the two can never disagree about a colour. It is
> the same brand and almost the same numbers; where they differ the product wins. The one changed
> constant is `--color-paper`, `#fdfcf9` here and `#fbf8f2` there. §3.1 has the shape of what
> replaced this, and `docs/client-platform/data/tokens.json` is the file to read for a current value.

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

MJXOFF-271 replaced the hand-derived dark palette this section first carried with
hermes-universal's own `allr` dark skin, and made both schemes *derived* rather than stated — see
§3.1. The seeds differ between the schemes; the expressions do not, which is what keeps the dark
theme reading as the same brand rather than a different product. Every value below is generated:

| Semantic | Light | Dark | How |
|---|---|---|---|
| app background | `#fbf7f1` | `#14221d` | derived: background seed at the chrome knob over the chrome neutral |
| surface | `#fffdfa` | `#223b33` | derived: card seed at the card knob over the card neutral |
| surface raised | `#fffefb` | `#284239` | derived: elevated seed at the elevated knob over the card neutral |
| text primary | `#223b33` | `#f0e9da` | the foreground seed itself — see §2.2 for why it is not softened |
| text secondary | `#5a6c64` | `#b7b5a9` | derived: the foreground at 74% over the background |
| border | `#e3d9c4` | `#43432e` | derived: the ring colour at the secondary-stroke knob over ink-at-7% over the background |
| border subtle | `#eae3d3` | `#343829` | derived: the same, at the tertiary knob over ink-at-5% |
| accent | `#2e9e63` | `#2e9e63` | the brand green, in **both** schemes — see §2.2's note on ordering |
| accent deep / pressed | `#1e7a49` | `#45b87f` | the `primary` seed — hermes lifts the dark one off the brand green to clear 4.5 : 1 on pine |
| accent surface | `#eef6ee` | `#234035` | derived: the accent at the tertiary-fill knob over the **surface** |
| accent border | `#cde7d6` | `#25533f` | derived: the accent at the primary-stroke knob over the surface |
| secondary accent | `#b77e1f` | `#e9a83e` | the `midground` seed: honey-deep on paper, honey on pine — the same decision from both sides |
| secondary surface | `#f9f3e9` | `#2c4034` | derived: the secondary accent at the tertiary-fill knob over the surface |

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
the light side would have left half the tokens unchecked. A **derived** colour is measured after it
is derived and against a background that may itself be derived, so the rule keeps biting through
the second tier rather than stopping at the seeds.

**⚠ Primary text is the ink itself and is deliberately not softened.** hermes paints
`--ui-text-primary` as the ink at 94% alpha; this palette does not, and the reason is measured
rather than aesthetic. A token tagged for text must be opaque — the generator refuses a translucent
one, because it would be measured against a surface it is never seen on — and an *opaque*
approximation of `rgba(ink, 0.94)` is only exact on the one surface it was composited over, which
is a promise a primary text colour cannot keep across a background, a surface and a raised surface.
Softening it also costs the palette its dynamic range: at 94% the colour picker's swatch indicator,
which chooses between primary text and the surface, bottomed out at **2.99 : 1** over its
4,352-colour sweep — under WCAG 1.4.11's floor. hermes's softening survives where it is a hierarchy
step rather than a texture, which is `text-secondary` at 74%.

**⚠ The accent-coloured label sets the strength of the accent fill, and the two schemes need
opposite answers.** In the light scheme the accent-coloured text (`accent-pressed`) is *darker* than
the accent fill, so a tint of that fill carries the label for free. Two things had to move so the
dark scheme keeps the same ordering rather than inverting it:

- **`theme.dark.accent` is the brand green, not a lifted one.** At `#4fc98a` the dark accent *fill*
  was as light as `accent-pressed`, and no tint of it could carry an accent-coloured label — the
  harness's own preset button measured 3.90 : 1. The old hand-written dark palette had the same
  defect at **4.11 : 1**, unseen because the catalogue's a11y sweep runs the light story.
- **`theme.dark.fill-tertiary-accent-mix` is 5% where the light scheme's is 8%** — the only
  accent-mix knob a scheme overrides. Every point of tint moves a dark fill *toward* its light label;
  at 8% the pairing is 4.36 : 1 and at 5% it is **4.53 : 1**.

That last margin is 0.03 above the minimum, which is the thinnest number in this palette. 3% would
be comfortable and would make the pressed fill indistinguishable from the surface it sits on, so 5%
— hermes's own quaternary value — is the largest knob that clears the floor at all.

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
| canvas backdrop | `#fbf7f1` | `#14221d` |
| page | `#ffffff` | `#ffffff` |
| page border | `#e3d9c4` | `#43432e` |
| page shadow | `0 2px 10px #223b3314` | `0 2px 14px #00000059` |
| selection fill | `#2e9e63` @ 18% | `#2e9e63` @ 22% |
| selection handle | `#1e7a49` | `#45b87f` |
| alignment guide | `#b77e1f` | `#e9a83e` |
| grid / ruler line | `#eae3d3` | `#343829` |
| comment anchor | `#b77e1f` | `#e9a83e` |
| tracked-change insert / delete | `#2e9e63` / `#b77e1f` | `#2e9e63` / `#e9a83e` |

**The page border is the chrome's border**, not a colour of its own — `document.*.page-border`
aliases `theme.*.border`, and `derivations.css` writes that as `var(--theme-border)`. This is the
one place chrome meets canvas, so it is the one place the two must not be able to drift; a shared
name is a stronger guarantee than two values that happen to agree. It is also where the
end-to-end-precision defect §3.2 describes would have shown, and did.

A **reading-mode inversion** for the page itself is a user setting, not a theme consequence.

---

## 3 · One source, four artefacts

The chrome is HTML and the document canvas is Rust, and a canvas cannot inherit a CSS custom
property. So the token pipeline emits four artefacts from one source — the project's existing
codegen doctrine: an `xtask` generator, output committed, never a `build.rs`.

```
docs/client-platform/data/tokens.json          (W3C Design Tokens format — the source)
        │   cargo run -p xtask -- tokens        (--check verifies; --out-dir writes elsewhere)
        │
        ├── ui/tokens/tokens.css               — custom properties for the Web-Component chrome
        ├── ui/tokens/derivations.css          — the derived tier, as the color-mix() it came from
        ├── ui/tokens/tokens.ts                — typed constants, the Tokens type, customProperties
        └── crates/mjx-tokens/src/generated.rs  — a `Tokens` struct and const default table
```

All four are **generated and committed**; the source is the only file to edit. Three gates hold them
together: `xtask/tests/tokens.rs` proves they are *derived* from the source, `mjx-tokens`'s
`artefacts_agree` suite proves they *agree with each other* by parsing the emitted CSS and TypeScript
from disk and comparing every value against the Rust table, and `ui/scripts/check-tokens.mjs` gives
the TypeScript half a gate that needs no Rust toolchain.

### 3.1 · The source is two tiers, and the second one is derived

Re-seeded in MJXOFF-271 from the application this platform embeds into
(`allr-agent/apps/hermes-universal`), whose token system is layered rather than flat: a handful of
**seeds** — the skin's raw colours — and **knobs** — mix percentages — from which ~250 surfaces,
text colours and strokes are *derived* by `color-mix()`. Dark mode is the same expressions over
different seeds, not a second palette. Copying the resolved hexes would have destroyed exactly that.

So `tokens.json` has two tiers:

| Tier | Written as | Example |
|---|---|---|
| seeds and knobs | a literal | `theme.light.midground: {color.honey-deep}`, `theme.light.stroke-secondary-accent-mix: 16%` |
| derived | `{ "mix": [ … ] }` | `theme.light.border` = 16% of the ring colour over the ink at 7% over the background |

The seeds' and knobs' **custom properties are hermes-universal's own names** — `--theme-foreground`,
`--theme-primary`, `--theme-midground`, `--theme-background-seed`, `--theme-mix-chrome` — because
that application writes them inline on `:root` when a person picks a skin. Dropping this platform
into it therefore re-themes the editor along with everything else, which is the whole point of the
re-seed.

### 3.2 · One `color-mix()` implementation, checked against Chromium

Every artefact carries the **resolved** colour: a canvas cannot paint an expression, a typed
constant cannot be one, and the contrast rule below cannot measure one. `derivations.css` is what
keeps the derivation alive anyway — it restates the derived tier as literal `color-mix(in srgb, …)`
in terms of the scheme-relative aliases, so a host that overrides one seed re-themes everything
mixed from it through the cascade, with no code. The renderer does the same through
`mjx_tokens::Tokens::rederive`.

That leaves exactly two evaluators: **`mjx_tokens::color_mix`, and the browser's.** `tokens.ts`
carries resolved colours and no algorithm, so the TypeScript half has none of its own to disagree
with. `ui/tokens/chromium-agreement.mjs` asserts the two agree — for every derived token, in both
schemes, against Chromium's own `getComputedStyle` — and it earns its place: it caught our
implementation quantising to eight bits at every token boundary where a browser evaluates the whole
expression in floating point, which had put five tokens (`--theme-border` and
`--document-page-border` among them) one step away from the chrome around them.

Two properties of `color-mix()` are easy to get wrong and are both load-bearing here:
`color-mix(in srgb, C p%, transparent)` is an **alpha** operation, not a blend toward black; and
percentages that do not sum to 100% **renormalise**, the shortfall becoming transparency.

At runtime the shell resolves in this order — **explicit host configuration → CSS custom properties
read off the host element → derivation from whichever seeds won → built-in defaults** — and pushes
the resolved snapshot across the bridge to the renderer, so a theme change repaints canvas and
chrome in the same frame. The shell watches for host changes with a `MutationObserver` and
`prefers-color-scheme`.

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
