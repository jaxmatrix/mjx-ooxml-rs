# HTML in the canvas — can we support it?

**Short answer: yes, and it is the natural second implementation of the `BoxModel` trait rather than a
bolt-on.** But "HTML in canvas" names three different problems on the web, and only one of them is
worth solving here. Separating them is most of the answer.

---

## 1 · The three things the phrase means

### (a) Rasterise a live DOM into a canvas — reject

What the phrase usually means on the web: `html2canvas`, `dom-to-image`, or the
`<foreignObject>`-to-data-URL trick. Each re-implements a slice of CSS in JavaScript and paints the
result into a 2-D context.

These are not candidates, for reasons that hold regardless of our architecture:

- **They are re-implementations, and incomplete ones.** Every one of them has a long list of
  unsupported CSS. Fidelity is exactly the thing this project refuses to compromise.
- **They produce a snapshot, not a surface.** No hit-testing, no selection, no caret, no editing.
- **`<foreignObject>` carries canvas-tainting and font restrictions** — external images taint the
  canvas, external stylesheets and fonts must be inlined, and Safari has long-standing bugs.
- **It is architecturally backwards for us anyway.** We have no DOM inside the document surface. The
  renderer is Rust; there is nothing to screenshot.

### (b) Render the app's own chrome into the canvas — not wanted

Deliberately rejected already: the chrome is TypeScript and HTML in a webview, by decision. Only the
in-canvas UI that must track document geometry at 60 fps — selection handles, guides, marching ants —
is Rust-drawn.

### (c) A real HTML/CSS box model that emits our `FragmentTree` — **this is the one**

Implement `BoxModel` over an HTML parser and a CSS layout engine, producing the same `FragmentTree`
that `mjx-layout-docx` produces. Everything above that seam — scene building, tessellation, painting,
hit-testing, selection, PDF and SVG export — then works on HTML content **unchanged**, because none of
it has ever heard of OOXML.

This is precisely the swap the box-model layer exists for.

---

## 2 · The Rust ecosystem already has the pieces

The stack is proven — [Blitz](https://github.com/DioxusLabs/blitz) (DioxusLabs) wires exactly these
together into a working HTML/CSS renderer:

| Concern | Crate | Provenance |
|---|---|---|
| HTML parsing | `html5ever` | Servo's spec-compliant parser |
| CSS parsing, cascade, computed values | `stylo` | the shared style engine of **Servo and Firefox** |
| Box layout — block, flexbox, grid | `taffy` | used by Blitz, Dioxus, Bevy, Zed |
| Rich text layout | `parley` | Linebender; shaping via HarfRust, analysis via ICU4X |

All pure Rust, so the workspace's no-C rule is unaffected.

**We would take Blitz's layout stack and not its renderer.** Blitz paints through Vello, which is
compute-shader based and therefore incompatible with the WebGL2 baseline the browser target requires.
That is a neat illustration of why the two seams in the architecture earn their keep: we adopt the
layer below `FragmentTree` and substitute everything above it. Blitz is also self-described as
pre-alpha, which is fine for borrowing a dependency choice and not fine for depending on the whole
engine.

---

## 3 · Why we would actually want this — four uses, ascending

**1 · `w:altChunk` is a real Word feature we currently cannot render.** WordprocessingML embeds a
foreign-format chunk by relationship — most commonly XHTML, also RTF and plain text — with
`matchSrc` deciding whether the source's own formatting is honoured. Verified in the schema at
`wml.xsd:3394`, in the block-level content group, `maxOccurs="unbounded"`. Today that content round-
trips faithfully and draws as nothing.

**2 · Pasting from a browser.** HTML is the dominant clipboard flavour from every web source. Office
converts HTML into WordprocessingML on paste, and doing that conversion well requires resolving a CSS
cascade regardless — so the work is owed whether or not we ever render HTML directly.

**3 · It is the "something else" the brief anticipated.** The requirement was that another box model
could be adopted and reuse the same surfaces. HTML is the highest-value candidate there is: Markdown,
web content, email bodies, help content, and templates all reduce to it.

**4 · It de-risks the abstraction, which is the strongest argument.** A `FragmentTree` with only
OOXML producers will silently acquire OOXML assumptions — a box that assumes twips, a line that
assumes a `w:p`, an anchor that assumes a drawing. A genuinely foreign second implementation is the
only reliable way to find them, and finding them *early* is worth far more than the feature itself.

---

## 4 · Recommended staging

Taking Stylo is a real commitment — it is Servo's style system, with the build weight and the
`wasm32` friction that implies. So stage it, and get the valuable 80% without that bet:

**Stage 1 — a restricted HTML box model.** `html5ever` + `taffy` + our own `mjx-text`, covering the
subset that actually occurs in `altChunk` payloads and pasted clipboard HTML: block and inline flow,
inline and `<style>`-block declarations over a bounded CSS property set, tables, lists, images, basic
typography. No full cascade, no flexbox or grid — that content does not use them. Small, self-
contained, and it delivers `altChunk` rendering and high-quality HTML paste.

**Stage 2 — the general HTML surface, if wanted.** Replace style resolution with `stylo` and layout
with full `taffy` behind the *same* `BoxModel` implementation boundary. Nothing above `FragmentTree`
changes, and the cost is known before it is paid.

---

## 5 · What this explicitly does not give us

Worth stating so nobody expects a browser:

- **No scripting.** No JavaScript, no layout-affecting DOM mutation, no event model.
- **No network.** Resources arrive from the package or the `DocumentSource`; nothing is fetched
  because a document said so — which is also a security property worth keeping.
- **No forms, no media elements, no plugins.**
- **Not a compliance target.** The goal is faithful rendering of the HTML that appears in documents
  and clipboards, not passing web platform tests.

Blitz draws the same line, for the same reasons.

---

## 6 · The architectural consequence

Nothing in the plan changes. `mjx-layout` already defines `BoxModel` and `FragmentTree`; this adds a
fourth implementation beside `mjx-layout-pptx`, `-docx` and `-xlsx`:

```
mjx-layout-html  (rank 3.6, beside the OOXML box models)
    ├── html5ever   → DOM
    ├── style       → computed values   (restricted set, later stylo)
    ├── taffy       → box layout
    ├── mjx-text    → shaping, bidi, line breaking
    └──             → FragmentTree      ← the same output as every other box model
```

It is a **ledger row, not a phase** — schedulable whenever `altChunk` or HTML paste becomes the most
valuable next thing, with no dependency on the OOXML box models beyond the contract they share.

## Sources

- [DioxusLabs/blitz](https://github.com/DioxusLabs/blitz) — the modular HTML/CSS engine over Stylo, Taffy and Parley
- [Blitz — a truly modular, hackable web renderer](https://webengineshackfest.org/2024/slides/blitz_a_truly_modular_hackable_web_renderer_by_nico_burns.pdf) (Web Engines Hackfest 2024)
- [linebender/parley](https://github.com/linebender/parley) — rich text layout
- [stylo_taffy](https://lib.rs/crates/stylo_taffy) — the Stylo-to-Taffy style bridge
- ECMA-376 Part 1, `wml.xsd` — `CT_AltChunk` / `CT_AltChunkPr`, local in `References/`
