# `ui/` — the client-platform component catalogue

The room the Phase U catalogue lives in: a Node workspace **outside the Rust rank graph**, holding
framework-free custom elements, the TypeScript half of the token resolver, and the harness every
component is audited in.

Nothing here is a Cargo workspace member. `cargo build`, `cargo test` and `cargo metadata` do not
know this directory exists, and `ui/`'s own checks never invoke `cargo`.

---

## Opening the catalogue

```sh
cd ui
npm ci                       # or `npm install` on a first checkout
npx playwright install chromium   # once; the browser tier needs it
npm run storybook            # http://localhost:6006
```

## The check set

Run the lot in the order CI runs it:

```sh
npm run check
```

or one at a time:

| Command | What it proves |
|---|---|
| `npm run tokens:check` | `ui/tokens/*` still agrees with `docs/client-platform/data/tokens.json` |
| `npm run icons:check` | `src/icons/generated.ts` still agrees with `src/icons/manifest.ts` and the vendor package |
| `npm run typecheck` | `tsc --noEmit` under `strict`, `noUncheckedIndexedAccess`, `exactOptionalPropertyTypes` |
| `npm run lint` | ESLint, including `mjx/story-conventions`, `mjx/no-literal-design-values`, the framework ban on `src/` and the icon-vendor ban |
| `npm run test:unit` | the logic with no rendering in it — manifest parser, contrast arithmetic, conventions, the lint rule |
| `npm run build-storybook` | the catalogue builds, failing on errors a dev server tolerates |
| `npm run test:browser` | **the gates** — a11y, theming, container queries, token resolution, visual snapshots |

`npm run test:browser` runs against `storybook-static/`, so `build-storybook` has to have run first.
`npm run check` does both in order.

---

## The five things this harness is built to prove

Storybook that merely *builds* is satisfied by a Storybook with no theming, no responsiveness and a
disabled accessibility rule. Each of the following is a gate that fails when its machinery stops
working, and each is demonstrated on a **throwaway probe** in `dev/` rather than on a real
component — because a gate is only known to work once it has been watched failing, and nobody keeps
a real component broken for long.

1. **The contrast rule can reject.** `Gates/Contrast · Machinery Can Reject` uses a colour that is
   not a token and must produce a `color-contrast` violation; `Token Rejected As Body Text` does the
   same with the palette's own closest miss. `tests/browser/a11y.spec.ts` asserts that a story
   declaring `expectViolations` **does** violate — a sweep that only checked that good stories pass
   would be satisfied by a switched-off rule.
2. **Token drift is caught.** `npm run tokens:check` re-derives every name from the source and
   compares both generated artefacts against it and against each other.
3. **The host-override path works.** `tests/browser/token-resolution.spec.ts` sets a custom property
   on the host, watches the component adopt it, removes it, and watches the generated default
   return — and reads `data-origin` so the *order* is asserted, not just the value.
4. **The page stays true white in both themes.** `tests/browser/theme.spec.ts`, on computed values,
   with every expectation derived from the generated tokens rather than from a hex written here.
5. **The container is the mechanism, not the viewport.** `tests/browser/container.spec.ts` drives
   the container across every preset and both breakpoint boundaries and asserts that
   `window.innerWidth` never moves.

---

## The foundations (MJXOFF-181)

Everything fourteen further children build on lives in `src/foundations/` and `src/icons/`.
**Nothing in a component may write a colour, a radius, a spacing, a duration or an easing directly**
— `mjx/no-literal-design-values` refuses it — so a component composes from these instead.

| Foundation | Where | What a component uses |
|---|---|---|
| Icons | `src/icons/` | `<mjx-icon name size variant label>` |
| Typography | `src/foundations/typography.ts` | `.mjx-type-dense`, `-control`, `-body`, `-label`, `-pane-title`, `-display` |
| Surfaces | `src/foundations/surfaces.ts`, `surface.ts` | `<mjx-surface level radius density>`, or `.mjx-surface-*` / `.mjx-radius-*` |
| Focus | `src/foundations/focus.ts` | nothing — a native focusable gets the ring by installing the sheet |
| Motion | `src/foundations/motion.ts` | `.mjx-motion-panel-enter`, `-sheet-enter`, `-surface-settle`, `-selection`, `-document-object` |
| Density | `src/foundations/density.ts` | `data-density="compact"`, `var(--mjx-density-step)`, `var(--mjx-density-gutter)`, `.mjx-hit-target` |

**A component gets all of them with one call.** A shadow root does not inherit the document's
*rules* (it does inherit custom properties), so:

```ts
import { installFoundations } from '../foundations/stylesheet.ts';

connectedCallback(): void {
  const root = this.attachShadow({ mode: 'open' });
  installFoundations(root);      // …and installFoundations(this.ownerDocument) if the component
                                 //    puts foundation classes on its own host element.
}
```

One `CSSStyleSheet` is constructed once and adopted by every root that asks.

### The icon subset

`@fluentui/svg-icons` ships **20,679** files and 12.7 MB; the catalogue ships **69 glyphs and 14 kB
of path data**. The workflow for a later child that needs a new icon is one step:

```sh
# add a row to src/icons/manifest.ts, then
npm run icons:subset      # rewrites the committed src/icons/generated.ts
```

Four gates keep it a subset, and each catches a different way of losing it — an ESLint ban on
importing the vendor package outside `scripts/`, `npm run icons:check` for drift,
`tests/browser/icons.spec.ts` for what actually reached the bundle (**set equality**, so a subset
that shrank fails too), and `tests/icon-subset.test.ts`, which measures the **real, whole** vendor
set and watches the byte budget refuse it.

> ⚠ `.github/workflows/ui.yml` enumerates its steps individually and does **not** yet run
> `npm run icons:check`; MJXOFF-181 was scoped to `ui/**` and did not edit it. Nothing is currently
> unguarded — `tests/icon-subset.test.ts` runs under `npm run test:unit`, which CI does run, and it
> asserts the same drift in both directions plus path-for-path fidelity against the vendor files.
> What the missing step would add is the *exact-formatting* check on the generated file. One line,
> after the `token drift` step.

### Three rules from `DESIGN_TOKENS.md` §4 that are now tests

They were prose from MJXOFF-156 until this child, and prose does not survive fifteen children:

* **A dense surface is `--text-xs` on `--leading-tight`** — not the source site's relaxed 1.7.
* **Young Serif is display-only** — the `display` role is the only one that may name `--font-serif`.
* **Nothing attached to a document object may overshoot** — `motionRoles` records which roles are,
  and `tests/foundations.test.ts` fails if one of them uses `--ease-spring`.

A fourth is new and has the same shape: **the focus ring must clear 3 : 1 against every rung of the
elevation ladder in both schemes.** It does today. If the palette re-seed breaks a pair, the gate
names the rung and the scheme — which is the correct outcome, because an invisible focus ring is a
defect whether or not anybody chose it.

---

## Story conventions

**Every story file declares four things, and the build enforces it.** A component without them is
not ready for audit.

```ts
import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { storyConventions } from '../../src/story/conventions.ts';

const conventions = storyConventions({
  statesMatrix: [{ name: 'default', description: 'Resting.' }],
  tokenDependencies: ['theme.light.surface', 'radius.control'],
  keyboard: [{ keys: 'Tab', does: 'Focuses the control.' }],
  screenReader: 'Announces its label, then its pressed state.',
});

const meta: Meta = {
  title: 'Controls/Button',
  parameters: { mjx: conventions },
};

export default meta;
```

* `statesMatrix` — every state the auditor must be able to see. One state still means one row.
* `tokenDependencies` — the blast radius of a token change. **Checked against the generated token
  table**: a name that no longer exists throws with the name in the message.
* `keyboard` — every behaviour, including *"not focusable"* where that is the truth.
* `screenReader` — what is announced, in the words it is announced in.

Two gates hold this together. `storyConventions` validates the *contents* at runtime;
`mjx/story-conventions` (in `eslint-rules/`) validates that they are *there* at all, statically —
which is the only way to catch a story that never calls the function.

**Why `parameters.mjx` and not a wrapper function.** Storybook indexes CSF by reading the source,
not by running it, and refuses a default export that is not an object literal. A
`defineStoryMeta(meta, …)` wrapper made every story unindexable. This shape is the one the indexer
accepts.

### Declaring a deliberate violation

Only the throwaway probes may do this:

```ts
export const MachineryCanReject: StoryObj = {
  parameters: expectsViolations(['color-contrast'], 'why this story is deliberately broken'),
  render: () => html`…`,
};
```

---

## Layout

```
ui/
  tokens/            GENERATED by `cargo run -p xtask -- tokens`. Never hand-edited.
  src/               the shipped surface: framework-free custom elements and the resolver
    tokens/          the TypeScript half of the token resolver
    foundations/     the type scale, elevation ladder, focus ring, motion vocabulary and density
    controls/        the ribbon archetypes: button, toggle, split button, dialog launcher
    ribbon/          the containers those controls live in: ribbon, tab, group, contextual tab set
    icons/           the Fluent subset; generated.ts is GENERATED by `npm run icons:subset`
    harness/         <mjx-resizable-container> and its presets
    plates/          the R10 plate-manifest loader and <mjx-plate-gallery>
    story/           the story conventions
  dev/               throwaway probes, the contrast arithmetic, the icon budget and the Word
                     TabHome census. NOT components.
  stories/           the catalogue
  tests/             the unit tier; tests/browser/ is the Playwright tier
  scripts/           the token-drift gate, the static server, the sample-plate fixture
  public/plates/     a committed sample manifest — see below
  .storybook/        Storybook configuration
```

`src/` may not import `lit`, React, Vue, Svelte or anything from Storybook — ESLint refuses. `lit`
is a legitimate *story-authoring* dependency (Storybook's web-components renderer is lit-html) and
an illegitimate *component* dependency, and the only difference between the two is which directory
the import is in.

### Constants a test needs live beside the component, not inside it

`src/harness/presets.ts`, `dev/bands.ts` and `dev/token-choice.ts` exist because **a module that
defines a custom element cannot be imported from Node**: `class extends HTMLElement` is evaluated at
module scope, and the Playwright specs run in Node. Follow the same split when a later child needs a
constant on both sides.

---

## The ribbon archetypes (MJXOFF-182)

`src/controls/` holds the four shapes that cover **10,518 of the 15,346 controls Office publishes** —
`<mjx-button>` (8,687), `<mjx-toggle-button>` (1,261), `<mjx-split-button>` (440) and
`<mjx-dialog-launcher>` (130). Open `Controls/Button → The States Matrix` first.

```html
<mjx-button label="Paste" icon="folder-open" size="large"></mjx-button>
<mjx-toggle-button label="Bold" icon="text-bold" size="icon" pressed="mixed"></mjx-toggle-button>
<mjx-split-button label="Undo" icon="arrow-undo" menu-label="Undo history"></mjx-split-button>
<mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>
```

### One state table, read by the stylesheet and by both gates

`src/controls/control-states.ts` is the whole design. Ten states, each naming *scheme members* —
`accentSurface`, never `'var(--theme-accent-surface)'` — so the same table generates the CSS,
resolves through the generated tokens for a Node gate, and is compared against `getComputedStyle`
by a browser gate. A component adds no paint of its own.

The ticket's trap and the answer to it:

> A states matrix is exactly the kind of thing that is *listed* in a story and not actually
> *distinct* in the CSS. **Two states that render identically pass every "the state exists"
> check.**

So there are two gates and they are not the same claim:

| Gate | Says | Does **not** say |
|---|---|---|
| `every pair of states differs` | no two states compute alike, in both schemes | that any of them is *right* |
| `what it computed is what the model says` | each state's fill, edge, text, weight and opacity equal the table | anything about pairs |

**Both are needed, and MJXOFF-182 proved it by breaking one thing and watching the other catch it.**
Adopting a component's sheet *before* the foundations silently strips the bold from all four held
states; every fill still differs, so the pairwise gate stays green and the correspondence gate fails
on `font-weight is 500, model says bold`.

### Four rules a later child inherits

1. **The base rule paints nothing.** `.control` scores (0,1,0) and every state rule scores (0,0,0),
   so a single `background:` in the base rule would out-specify the entire state table and leave a
   control that renders its resting paint in all ten states. `tests/controls.test.ts` asserts the
   base rule declares none of the seven properties the table owns.
2. **The cascade is source order.** Every selector — pseudo-classes included — is inside `:where()`,
   so `controlStateCascade` is the only thing deciding. It is an array, and a test reads it.
3. **A forced state is the real state.** `force-state="hover"` exists because a static story cannot
   hover; the forced selector and the real pseudo-class share **one declaration block**, and the
   browser gate drives a real pointer and requires the two to compute identically anyway.
4. **`installControlStyles(root, css)` — never `installFoundations` plus a hand-rolled adoption.**
   It is the one place the order is stated.

### Two kinds of unavailable, and only one of them is `disabled`

Office uses *unavailable-but-explained* far more than a plain disabled control, and the two are
genuinely different:

* `disabled` — the native attribute. Out of the tab order, inert to pointer and keyboard by the
  platform's own doing, dimmed (and exempt from axe's contrast rule, which is what lets it dim).
* `unavailable` — `aria-disabled`, **still focusable**, carrying its reason in an `aria-describedby`
  target and in `title`. Activation is refused in JavaScript, which is the price of staying
  reachable. A command a person cannot reach is a command whose reason they can never read.

### `GUESS:` where this diverges from Office

Nothing here is judged against a screenshot of Office and none of it is parity. Three deliberate
guesses are marked at their sites: the `mixed` → pressed transition, the derived
*"More <label> options"* arrow name, and the dialog launcher's **target size**, which is larger than
Office's because every control here carries the `.mjx-hit-target` floor.

### ⚠ A CSF constraint that bites twice

Storybook indexes stories **statically**. MJXOFF-180 found that a `defineStoryMeta` wrapper made a
file unindexable; MJXOFF-182 found the same rule refuses a computed `title` —
*"CSF: unexpected dynamic title"* — so `title:` must be a string literal even when a constant holds
exactly that string. `archetypeStoryTitle` and `stateCellAttribute` are therefore duplicated into
the story files, and kept honest by the gate rather than by the type system: the browser suite looks
every story up by title and asserts it found **exactly** as many matrix cells as the component
claims states, so a divergence fails with a count instead of sweeping an empty list.

## The ribbon structure (MJXOFF-183)

`src/ribbon/` holds the containers MJXOFF-182's controls live in — `group` (1,193 in the published
surface), `tab` (166) and `tabSet` (52) — and **the hardest responsive problem in the catalogue**.
Open `Ribbon/Ribbon → The Priority Ladder` first, and resize the container rather than the window.

```html
<mjx-ribbon label="Word" selected="home" state="expanded">
  <mjx-ribbon-tab tab-id="home" label="Home">
    <mjx-ribbon-group label="Font" priority="primary">
      <mjx-toggle-button slot="essential" label="Bold" icon="text-bold" size="icon"></mjx-toggle-button>
      <mjx-button label="Clear All Formatting" icon="dismiss"></mjx-button>
      <mjx-dialog-launcher slot="dialog-launcher" label="Font settings"></mjx-dialog-launcher>
    </mjx-ribbon-group>
  </mjx-ribbon-tab>
  <mjx-contextual-tab-set label="Table Tools">
    <mjx-ribbon-tab tab-id="table-design" label="Design">…</mjx-ribbon-tab>
  </mjx-contextual-tab-set>
</mjx-ribbon>
```

### One slot, three presentations — which is why nothing can be lost

The obvious way to build a collapsing group is to render its commands twice, once in the strip and
once in a menu. That is also the way to lose one. So a group renders its commands **once**, into one
`<slot>` inside one `.panel`, and the three presentations change what that element *is*: part of the
strip in `full` and `reduced`, an overlay anchored to a single button in `collapsed`. *Reachable in
all three* is therefore structural rather than remembered — the same DOM nodes at every width.

| Presentation | What changes | Reached when |
|---|---|---|
| `full` | large commands stay large; the group's name sits under them | above the priority's reduce width |
| `reduced` | compact density; a large command lays out sideways and clamps to one line; two rows | at or below it |
| `collapsed` | one button carrying the name, the essential commands beside it, the rest in a popup | at or below the priority's collapse width |

### The priority ladder, and why a group declares a *priority* rather than a width

Office's collapse ordering is per-group, and MJXOFF-183 also forbids measuring in a resize handler.
CSS cannot read a number out of an attribute and use it in a `@container` condition, so per-group
*arbitrary widths* would need exactly the JavaScript that is ruled out. What CSS **can** match on is
an attribute's *value*, so a group declares one of four **priorities** and
`src/ribbon/ribbon-model.ts` generates eight `@container` blocks from the ladder. A group says *when
it gives way relative to the others*, which is what Office's ordering actually is.

`Three Presentations At One Width` is the story that shows why this is not a global breakpoint: at
1000px three groups sit in three different presentations at once.

### How a CSS decision is read back without measuring anything

Every presentation block writes `--mjx-group-presentation`, and both the component (to know whether
its panel is currently a popup) and the gate read it with `getComputedStyle`. **A gate that read
only that would be asking the implementation to grade its own homework**, so each presentation is
cross-checked against three facts the token does not control: the trigger's `display`, the panel's
`position`, and the density step inside the group.

### ⚠ The specificity accident, for the third time

MJXOFF-181 found `:where(…):focus:not(:focus-visible)` out-specifying `:where(…):focus-visible`.
MJXOFF-182 found a component sheet adopted before the foundations. This child shipped the simplified
rule as `:host([simplified]) .group[data-priority='x']` — **(0,4,0) against the container rules'
(0,2,0)** — so it won at every width and a simplified ribbon never collapsed a group however narrow
it got. The unit test asserting *emission order* stayed green the whole time, because emission order
only decides between rules of equal specificity.

Only the **correspondence** gate caught it — the one comparing the browser against
`groupPresentationAt()`. Every presentation selector is now wrapped in `:where()` and
`tests/ribbon.test.ts` asserts that wrapping as well as the order, because the order assertion alone
was true and useless.

### The levers a container may pull on a control it does not own

A group never restyles a command. `controlSizeCss` publishes four custom properties —
`--mjx-control-orientation`, `-label-lines`, `-min-inline`, `-max-inline` — whose fallbacks are
exactly what the control's `size` already says, so a control with nothing set above it computes what
it always did. Custom properties inherit through the flat tree, which makes this the only channel
that reaches a slotted component's shadow root. **The icon is deliberately not a lever**: Fluent
draws each size separately, and choosing a drawing is not a scale factor.

### Command demotion — the design decision, stated

`Ribbon/Word TabHome → The Demotion Rules` draws the table from `demotionRules`, so the page cannot
disagree with the model. A command survives a collapse only if **all four** hold: it is immediate
and reversible (nothing that opens a menu, a gallery or a dialog — a popup inside a popup is where a
phone UI dies); it is recognisable with no label; there are at most **three per group**; and
**nothing is ever removed**. The third is a number the browser gate enforces and
`<mjx-ribbon-group>` reports on the console; the first is checked structurally (no essential command
may carry `aria-haspopup`, and none may be a split button); the second is a judgement and is stated
as one.

### The worst case is read out of the census, not out of the ticket

`dev/word-tab-home.ts` is a transcription of `docs/client-platform/data/command-surface.tsv`, and
`tests/ribbon.test.ts` reads that file and fails on any drift. **Three sources disagree about the
size of Word's Home tab** — the ticket says 165 controls across eight groups, the inventory §4.1 says
165 while listing five principal groups, and the committed TSV says **152 across thirteen**. The one
that is *checked* wins, and thirteen groups is the harder case anyway: seven of them hold one or two
controls, which is exactly the shape a uniform collapse rule gets wrong.

### Two popups, two behaviours, on purpose

A collapsed group **traps** focus: it is a command surface with many stops and a person who Tabs out
of it has lost the group. The narrow-width **tab picker does not**: it holds one roving tab stop, it
is a disclosure, and a disclosure closes when focus leaves it. Both return focus to the button that
opened them on Escape.

### What a later child will trip over here

* **An IDREF does not cross a shadow boundary.** The tab buttons live in the ribbon's shadow root
  and the panels are light DOM, so `aria-controls` from tab to panel would resolve to nothing
  whichever id spelling was used. The association is made with `aria-label` on the panel instead.
* **A contextual set's coloured band is a picture.** Each contextual tab therefore carries its set
  in its own accessible name — *"Design, Table Tools"* — and the band is `aria-hidden`. There is one
  contextual tone rather than twenty-one, because this palette has two colour families; the *title*
  is the general mechanism, and a re-seed that brings more hues is where per-set colour would come
  from. Marked `GUESS:` at the site.
* **Container queries cross shadow boundaries; the colour scheme still does not.** A group's rules
  live in its own shadow root and query `mjx-ribbon`, a container established two levels up in the
  light DOM. That was verified in Chromium before the design was committed to, because the whole
  thing rests on it.
* **The ribbon host has no padding and no border**, for the same reason the harness frame has none:
  a container query resolves against the content box. The breathing room is on `.strip` and `.body`
  *inside* the container, where it changes nothing.
* **Two frames is enough for a layout change and not for a paint change.** Every control wears
  `.mjx-motion-surface-settle`, so a background read two frames after a selection change is an
  interpolated colour — `rgba(251, 239, 216, 0.718)` where the token says `rgb(251, 239, 216)`. The
  browser gate waits `duration.transition × 2 + 100`, derived from the token.
* **A deliberate break must be checked to actually break something.** Proving the reachability gate
  could fail was first attempted with `::slotted(:last-child)`, which matched nothing — the group's
  last light-DOM child is the dialog launcher, which is in a different slot — and the gate passed.
  A green run under a break that did nothing proves nothing at all.

## Things a later child should know

* **The scheme layer is `:root`-scoped.** `tokens.css` keys its three rules off `:root`, so
  `data-theme="dark"` on a `<div>` does nothing and a subtree cannot carry its own scheme. Where
  both schemes must be shown at once, name the scheme-specific properties
  (`--document-dark-backdrop`) — they are emitted unconditionally.
* **The container frame has no padding and no border**, deliberately. A container query resolves
  against the content box, so chrome inside the frame would shift every component's breakpoints.
  There is a test that says so.
* **No font is fetched.** `--font-sans` names *Nunito Sans* and falls back to `system-ui` until a
  self-hosted face is added. The fallback is visible in the catalogue rather than hidden behind a
  CDN link — so `Foundations/Typography` shows the right *proportions* and the wrong *shapes*.
* **The container's stage carries `tabindex="0"`, and it is required.** It is a scroll container in
  both axes (a box with `overflow-x: auto` computes `overflow-y: visible` to `auto`), and a scroll
  container a keyboard cannot reach fails axe's `scrollable-region-focusable`. It went unnoticed
  until U02 added stories tall enough to overflow it, so a story with a tall page has one extra tab
  stop before its content.
* **There are two axe runs per story and only one is the gate.** `preview.ts` sets
  `a11y: { test: 'error' }`, so the addon runs axe as a story renders, and the sweep runs it again;
  axe refuses to run twice at once. `openStory` waits for the addon to finish
  (`waitForAxeIdle`), which removes the race rather than retrying past it. If a11y ever goes red on
  one story with *"Axe is already running"*, that is what has regressed.
* **`public/plates/` is a fixture, not a render.** The real plates come from
  `cargo run -p mjx-render-oracle -- gallery <dir>`, which builds `mjx-paint` and links the
  platform's graphics stack; making the catalogue depend on that would put a Vulkan toolchain
  between a TypeScript contributor and a green run. Regenerate the fixture with
  `node scripts/make-sample-plate.mjs`. `<mjx-plate-gallery src="…">` loads a real directory
  unchanged.
* **The visual baselines were generated by the code under test** and no person has approved them.
  They lock the current appearance against accidental change; they do not assert it is right.
