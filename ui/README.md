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

`@fluentui/svg-icons` ships **20,679** files and 12.7 MB; the catalogue ships **73 glyphs and 16 kB
of path data** — 71 until MJXOFF-185, which added the two the gallery's affordance rail needs. The
workflow for a later child that needs a new icon is one step:

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
    menus/           menu, menu item, separator, section and the context menu
    gallery/         the in-ribbon strip, the expanded flyout, and the live-preview protocol
    inputs/          the seven fields a task pane, a dialog and an inspector are made of
    pickers/         the colour picker and the font picker
    surfaces/        dialog, modal, sheet, popover, flyout and the task pane — six rows, three tags
    overlay/         the primitives those surfaces share: floating.ts (placement, the boundary, the
                     safe triangle, the top layer) and modality.ts (inert, the trap, focus return)
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

## The menus (MJXOFF-184)

`src/menus/` holds `menu` (649 published controls) and `contextMenu` (302) — and the doorway to the
**largest bucket in the whole inventory**: *"None (Context Menu)"* is 1,155 controls in Word, 1,104
in Excel and 1,660 in PowerPoint. More of Office is reached by right-clicking than through any
ribbon tab. Open `Menus/Menu → Submenus And Diagonal Travel` first, **with a pointer**.

```html
<mjx-menu label="Edit">
  <mjx-menu-item label="Cut" icon="delete" shortcut="Ctrl+X"></mjx-menu-item>
  <mjx-menu-separator></mjx-menu-separator>
  <mjx-menu-section label="Paste Options">
    <mjx-menu-item kind="radio" label="Keep Source Formatting" checked></mjx-menu-item>
  </mjx-menu-section>
  <mjx-menu-item kind="checkbox" label="Ruler" checked></mjx-menu-item>
  <mjx-menu-item label="More Options">
    <mjx-menu slot="submenu" label="More Options">…</mjx-menu>
  </mjx-menu-item>
</mjx-menu>

<mjx-context-menu>
  <div tabindex="0">…the thing being acted on…</div>
  <mjx-menu slot="menu" label="Slide">…</mjx-menu>
</mjx-context-menu>
```

### Trap or disclosure — the question MJXOFF-183 left open, answered

MJXOFF-183 shipped two popups with two different answers and asked for an audit. **The number of
tab stops decides**, and `focusManagementPatterns` in `src/menus/menu-model.ts` records it:

| Pattern | Tab stops | Movement | Dismissal | Surfaces |
|---|---|---|---|---|
| `roving` | one | arrows, Home/End, type-ahead | closes when focus leaves | a menu, a submenu, a context menu, the ribbon's **tab picker** |
| `trap` | many | `Tab`, because the stops are independent controls | Escape or a click outside | the ribbon's **collapsed group** |

A surface with one tab stop is free to let `Tab` mean *leave* — which is what the ARIA menu pattern
requires — and a surface whose `Tab` means *leave* has to close when focus leaves it. A surface with
many tab stops uses `Tab` as its internal navigation, so `Tab` cannot also be the way out, and a
surface a person can Tab out of but not back into has lost them.

**So MJXOFF-183's asymmetry stands, and it is not an asymmetry between popups.** It is one rule
applied to two surfaces with different contents: the tab picker holds one roving stop and agrees
with the menu; the collapsed group is a container of separately focusable *buttons*. The audit
finding is that the picker and the menu should stay identical — and they do, including returning
focus to the invoker on Escape.

### Two measurements that changed the design, both invisible in a screenshot

**1. A submenu is clipped by the menu that opened it, and a fixed position does not save it.** A
submenu is a DOM descendant of its row, which is a descendant of the parent menu's box, and that box
scrolls its own list. The submenu had the right rectangle, was painted nowhere, and
`elementsFromPoint` at its own coordinates returned the *parent* menu. A pointer could not reach it,
and no snapshot could have shown it. The fix is the platform's own: a floating menu carries
`popover="manual"` and lives in the **top layer**. `manual` rather than `auto`, because dismissal,
nesting and focus are this component's own and an `auto` popover closes a parent when its child
opens. `menuCss` then undoes the UA popover stylesheet — `inset: 0`, `margin: auto`, a border, a
padding and `color: CanvasText` — every one of which beats what a menu is supposed to look like.

**2. A shortcut hint cannot be grey.** `--theme-text-secondary` measures **4.32 : 1** against
`--theme-border-subtle`, which is the fill the `hover` state paints across the whole row. Office
greys its hints; this palette cannot, because the hint would become illegible *exactly when the
pointer is on it*. So the secondary lines are told apart by **size** — the row is `control`, the hint
and the description are `dense` — and `tests/menus.test.ts` asserts the measurement rather than the
prose, so the day a re-seed makes the pairing legal the gate says so.

### The safe triangle, and why it is not a hover delay

A delay does not solve diagonal travel: a pointer moving from *Paste Options* toward its submenu
crosses *Paste Special* on the way, and a delay only postpones losing it. What is needed is
**intent** — while the pointer is inside the triangle from where it left the parent row to the
submenu's near edge, a sibling it happens to be over is not the item it means. `travellingToward`
in `src/overlay/floating.ts` is that triangle, pure, so a pointer path can be written down in a
Node test; `<mjx-menu>` arms it as a *grace* that is released the moment the pointer leaves the
triangle and re-armed on every move that is still inside it.

Two facts about the geometry that are easy to get wrong and are asserted:

* **The apex is inside the parent row, not on its edge.** The pointer that is about to cross a
  sibling is still inside the parent menu when it starts moving. An apex taken at the row's outer
  edge makes the triangle four pixels wide and the tolerance protects nothing.
* **The timeout is for a pointer that has stopped**, not for one taking its time. A fixed deadline
  turns a slow diagonal into a closed submenu 300 milliseconds late.

**The keyboard never waits for any of it.** `ArrowRight` opens the submenu and moves into it in the
same task, asserted with `performance.now()` against the hover delay.

### One menu has one kind of unavailable

`control-states.ts` distinguishes `disabled` (native, out of the tab order, inert by the platform)
from `unavailable` (`aria-disabled`, still focusable, carrying its reason). **A menu item is only
ever the second**, and it is a behavioural requirement rather than a taste: the ARIA menu pattern
keeps unavailable items in the arrow-key sequence, so a person can find out that a command exists
and is currently unavailable. `menuItemStateNames` therefore has no `disabled` member, and the
browser gate arrows *onto* an unavailable row and then requires both Enter and a click to be refused.

### The floating-layer primitive, handed forward to U09

`src/overlay/floating.ts` is deliberately menu-free — **U09 extends it rather than writing a second
placement.** It holds `placeFloating` (flip, shift, constrain), `clippingBoundary`,
`clippingAncestor`, `applyPlacement`, `pinFloating` (what a sheet is), `travellingToward` and the
`@property` registrations that make a token-derived length readable in pixels.

Four things in it were learned the hard way and are worth reading before extending it:

* **The boundary is the nearest declared viewport**, not the window: the viewport intersected with
  every flat-tree ancestor that clips *and* either is a real fixed containing block or is a
  container-query container. `.frame` is both; `.menu` is neither, which is what lets a submenu open
  outward while a menu still flips at the edge of the simulated screen. **A `parentNode` walk finds
  neither** — the frame is in a shadow root and the story content is slotted into it.
* **Chromium does not make a `container-type` element a containing block for fixed descendants**,
  whatever CSS Containment's layout-containment paragraph reads like. A sheet pinned with
  `inset: auto 0 0 0` inside the harness frame spans the *window*. That is why `pinFloating` exists.
* **The coordinates a placement writes must be physical.** `placeFloating` returns an `x` measured
  from the left of the screen whatever direction the text runs, because `physicalSide()` has already
  resolved the logical preference. Writing it into `inset-inline-start` means *the right edge* under
  RTL, and `applyPlacement`'s correction diverges rather than converging on a mirrored axis — which
  is how an Arabic submenu ended up at x = −1052.
* **A floating box is measured twice, one frame apart.** Anything that settles late — an icon with
  no box while the menu was `display: none`, a gutter column applied in the same task — changes the
  size the placement was computed for.

### What else a later child will trip over here

* **`focus({ preventScroll: true })`, and the menu scrolls itself.** The browser's scroll-into-view
  walks every ancestor scroll container, and a menu's ancestors include the parent menu it opened
  from. Left alone it scrolls the *parent* to reveal the fixed submenu, dragging every row of it 230
  pixels sideways while the submenu stays put — and the anchor the submenu was placed against then
  reads as somewhere it never was.
* **A `focusout` with a null `relatedTarget` must be re-checked a microtask later.** It is the
  commonest movement in a menu system: a submenu closing hides the focused row, focus falls to the
  document, and the submenu *then* puts it back. Acting on it synchronously closes the whole menu one
  Escape early.
* **The role goes on the host for an item and on an inner box for a menu.** A role-less
  `<mjx-menu-item>` would let an accessibility checker walking the menu's owned children descend
  *past* it and find the nested `<mjx-menu slot="submenu">` as a child of the outer menu — a `menu`
  owned by a `menu`, which is not an allowed child. The cost is that the painted box cannot carry
  `aria-disabled` or `aria-checked` (a `<div>` wearing one is a real `aria-allowed-attr` violation),
  so the host's ARIA state is mirrored onto it as `data-*` in one place.
* **The type role goes on the box the state table paints, never on the label inside it.** Every type
  role declares `font-weight`, so a role class on `.label` beats the weight the row inherits from its
  state and a checked item silently loses its bold — MJXOFF-182's accident reached by a new route.
* **The mark gutter is the menu's decision, not the row's.** Custom properties inherit through the
  flat tree, so `.menu` publishes `--mjx-menu-mark-column` and every slotted row reads it. A row that
  sized its own gutter would leave a menu whose third item is the only checkable one misaligned.
* **A backtick inside a CSS template literal ends the template.** Two builds were lost to a comment
  that quoted a CSS keyword the usual way.

## The gallery (MJXOFF-185)

`src/gallery/` holds `gallery` — **1,773 of Office's published controls**, the second-largest
archetype after `button` and the least like one. Styles, themes, shapes, transitions, table styles,
WordArt and chart layouts are all galleries. **Open `Galleries/Gallery → Live Preview` first, with a
pointer.**

```html
<mjx-gallery label="Styles" value="normal">
  <mjx-gallery-item value="normal" label="Normal" category="Built-In">
    <span style="font-size:1em;font-weight:500">AaBbCc</span>
  </mjx-gallery-item>
  <mjx-gallery-item value="title" label="Title" category="Built-In">…</mjx-gallery-item>
  <mjx-gallery-item value="caption" label="Caption" unavailable
                    explanation="This document has no caption style.">…</mjx-gallery-item>
  <button slot="footer">Save Selection as a New Style…</button>
</mjx-gallery>
```

### `<mjx-gallery-item>` is a descriptor, and it is deliberately not a `<slot>`

An item **captures its children into an inert `DocumentFragment`** on connect and renders nothing
itself; the gallery clones that fragment into the cells it decides to build. Two hard requirements
fall out of that and neither is satisfiable with a `<slot>`:

* **A slotted node can be in exactly one place.** The strip and the flyout are two geometries of
  *one* item set and they are on screen together. Slotting would mean authoring the items twice,
  which is exactly how two surfaces come to disagree about which one is selected.
* **A slotted node is rendered.** *"Hundreds of entries must not build hundreds of nodes"* is not
  satisfiable while every item's markup is live in the light DOM; hiding is not virtualising.

> ⚠ **The consequence, and it is the one thing about this component that its API does not tell
> you: an item's art is cloned into a *shadow root*, so a document stylesheet does not reach it.**
> The swatch specimens were first written with `class="swatch"` and a rule in the story's own
> `<style>`, and rendered as eight captions with no colour in them — while every gate that counted
> cells or read a caption stayed green. Custom properties **do** cross the boundary, so the answer
> is inline style with `var(--theme-…)` values. `tests/browser/gallery.spec.ts` now asserts that a
> swatch actually paints.

### Live preview: the protocol is the component's, the preview is not

`src/gallery/preview-session.ts` is a **pure state machine**, because *"leaving restored precisely
the prior state"* is an invariant over sequences and a browser test can only drive one sequence at a
time. The invariant it holds:

> **Every `preview` is followed by exactly one `cancel` or exactly one `commit`, never both and
> never neither, and at most one preview is outstanding at any moment.**

`tests/gallery.test.ts` drives 400 random sequences through it and compares a simulated listener
against the machine after every operation. `tests/browser/gallery.spec.ts` then proves the component
*obeys* it through real pointers, real keys and real touches.

**`restore` is in the protocol, not in the listener.** Every event carries the value that was
committed when the preview began, so a listener never has to keep an undo stack — and the specific
bug that makes impossible is *restoring to the value the page opened with rather than to the one
that was last committed*, which is invisible in every screenshot. There is a gate for exactly that,
and it fires when `restore` is changed to the initial value.

| Path | Timing | Why |
|---|---|---|
| pointer | **coalesced** by `pointerPreviewSettleDelay` | a pointer crossing eight cells to reach the ninth has expressed no intent about the eight |
| keyboard | **immediate** | an arrow key is a deliberate act per item — `<mjx-menu>`'s *"the keyboard never waits"*, and the outstanding-preview invariant is what protects the renderer instead |
| touch | **press and hold previews, a tap commits** | there is no hover on a phone; the affordance is chosen and argued in `touchPreviewAffordance` and marked `GUESS:` there, because Office's own mobile apps have no live preview to be parity with |

The coalescing assertion is run **beside a run with the coalescing removed** (`preview-delay="0"`,
which exists for that reason), because MJXOFF-184's lesson is that a timing assertion which passes
with the timing taken away is measuring nothing.

### Virtualisation, asserted on a node count

Each surface plans its rows (`galleryRowPlan`), takes a window (`galleryWindow`) and builds only
that; the sizer is as tall as **every** row so the scrollbar tells the truth. The fixture is **four
hundred** items, because a twenty-item fixture proves nothing, and the number the gate compares
against comes from the plan (`cellsInWindow`) rather than from the renderer.

The window is then **extended to contain the active row**, which is what keeps the roving tab stop
reachable: without it, scrolling the active cell out of view leaves a listbox with no `tabindex="0"`
in it.

### Two measurements, and only one of them is a layout decision

**How many columns there are is CSS's decision** — `repeat(auto-fill, minmax(…, 1fr))` — and the
presentation is read back out of `--mjx-gallery-presentation`, exactly as `<mjx-ribbon-group>` and
`<mjx-menu>` do. What the component *measures* is what CSS decided, by reading the used
`grid-template-columns` off a zero-height probe row, because two-dimensional keyboard navigation
cannot be done without a column count and no amount of CSS will tell `ArrowDown` what it means.
Those are different questions.

### ARIA: a gallery is a listbox, not a menu

Each surface's scroller is `role="listbox"`, each cell is `role="option"` with `aria-selected`, and
each category in the flyout is a `role="group"`. The structural divs a virtual list needs — the
sizer, the layer, the row — carry `role="presentation"` so an accessibility checker walking the
listbox's owned children does not find a `<div>` where an `option` belongs. **While the flyout is
open the strip is `inert`**, so the same forty options are not offered twice.

### Four things a later child will trip over here

* **Custom elements upgrade in tree order**, so a gallery's `connectedCallback` runs while its
  `<mjx-gallery-item>` children are still plain `HTMLElement`s — no `descriptor`, no captured art.
  The first build produced cells with captions and empty art boxes, and the later `slotchange` did
  not fix them because the rebuild was skipped for having the same window and the same count.
  `customElements.upgrade` removes the race; `#revision` makes *the data changed* part of the
  rebuild signature.
* **A null `document.activeElement` is the component rebuilding under itself, not a person
  leaving.** A virtualised surface replaces its own cells, and in the frame between removing the
  focused cell and focusing its replacement, focus is on the body. Treating that as *focus left*
  closed the flyout in the task it opened in. `<mjx-menu>`'s deferred-focusout note, arriving by a
  route a menu does not have.
* **`document.activeElement` is the *host* while focus is inside a shadow root**, so a `contains`
  against it is false for every cell this component owns. Following focus down is the difference
  between a rebuild that keeps the keyboard where it was and one that drops it on every scroll.
* **Watch the resize always, not only while the flyout is open.** Three boxes: the element itself
  (its width is what CSS turns into a column count), its parent (a ribbon group changing
  presentation republishes `--mjx-group-presentation`), and the clipping ancestor (a flyout has to
  be re-placed). The first version started this on expand, and the degradation story reported `full`
  at every width — correctly, because nothing had asked it again.

### Two gates that were green under a real break, and what fixed them

Both are worth reading before writing a gate for anything in this catalogue.

1. **A forward eight-cell traversal ended on the *unavailable* item**, which never previews. So
   *"one preview, not eight"* was satisfied by **zero**, and would have stayed green for a component
   that never previewed on hover at all. The traversal now runs backwards and asserts
   `previews.length > 0` beside the ceiling.
2. **The group-degradation gate read `--mjx-gallery-strip-rows` back and compared it against
   `galleryStripRowsFor()`** — a generated stylesheet against the table it was generated from. They
   cannot disagree, and setting every row count to two left the gate green. It now also **counts the
   rows that actually fit in the viewport**, which is geometry the custom property does not decide.
   That is `<mjx-ribbon-group>`'s own rule, re-learned.

### `GUESS:` where this diverges from Office

Three, marked at their sites: the **touch affordance** (Office has none to copy), the **rail's
accessible names** (*"Scroll Styles forward one row"* rather than Office's bare gallery name), and
the **flyout's seven-column target**, which is a shape rather than a measurement.

### One number, three surfaces

`phoneShellAtOrBelow` moved into `src/harness/presets.ts`, on the instruction MJXOFF-184 left in
`menu-model.ts` (*"when a third surface needs it, hoist it — do not add a second definition"*). The
ribbon's tab picker, the menu's sheet and the gallery's sheet are all aliases of it, and
`tests/gallery.test.ts` asserts they still are.

## The inputs (MJXOFF-186)

Seven controls — `<mjx-label>`, `<mjx-checkbox>`, `<mjx-dropdown>`, `<mjx-combo-box>`,
`<mjx-measure-input>`, `<mjx-slider>` and `<mjx-segmented-control>` — plus two descriptor elements,
`<mjx-option>` and `<mjx-segment>`. A ribbon is commands; a task pane, a dialog and an inspector are
**fields**, and this is what they are made of.

### A field is a surface you type into, not a button you press

The field state table does **not** borrow the control table's paint, and that is the one decision
the whole of `input-model.ts` is arranged around. It is a measurement rather than a taste:
**`--theme-text-secondary` is 4.32 : 1 on `--theme-border-subtle`**, and `controlStateSpecs.hover`
fills with exactly that. A combo box's placeholder, a measure input's unit suffix and a slider's
tick labels are all secondary text, so a field wearing the button's hover would have illegible
secondary text *precisely while a person was pointing at it* — legible in every screenshot,
illegible in use.

So a field's **fill never changes**: hover, editing and invalid move the *edge*, and every piece of
secondary text sits on `--theme-surface` (5.23 : 1 light, 6.61 : 1 dark) in all seven states.
`tests/inputs.test.ts` asserts the construction rather than the outcome — one assertion that the
seven fills are one fill, and one that the borrowed hover fill would have failed.

The *machinery* is still shared: `controlStateDeclarations`, `paintValue` and `disabledOpacity` all
come from `control-states.ts`, and `composeStatePaint`/`resolvePaintFingerprint` are asserted
**equal** to `effectiveStatePaint`/`resolvedStateFingerprint` over all ten control states. A
near-duplicate that is checked against the thing it nearly duplicates is not a duplicate.

The checkbox's box and the listbox's option **do** borrow, wholesale, exactly as a menu row does —
and each says why for the one or two paints it had to declare itself. A segmented control borrows
*everything*: it is painted by `controlStatesCss('.segment')` through the same `data-pressed`
attribute `<mjx-toggle-button>` writes, and the fingerprints are asserted identical rather than
similar.

### The indicator rule: what says a state is on, without reading its text

WCAG 2.2 §1.4.11 wants a state indicator at 3 : 1, and the mistake to avoid is assuming the
indicator is always a colour. `tests/inputs.test.ts` attributes every state in every table to the
strongest of five mechanisms — `ring`, `border`, `fill`, `weight` (a font weight or border-style
change, which is not a colour at all) or `declared` — and **writes the whole map out**. A state
sliding from `ring` to `declared` is then a diff somebody has to write down, rather than a threshold
that quietly still passes. Beside it sits the anti-vacuity assertion the map exists for: at least
one state per table must be carried by a *measured* colour, so a table that declared its way out of
everything fails.

Two measurements came out of writing it and both are asserted so they cannot rot:

* **`--theme-accent` on `--theme-border-subtle` is 2.81 : 1 in light**, so the slider's filled track
  is `--theme-accent-pressed` (4.41 : 1). The boundary between the filled and unfilled halves is the
  entire visual output of a slider.
* **`--theme-secondary-accent` on `--theme-surface` is 2.07 : 1 in light**, so the invalid field's
  honey edge cannot carry that state on its own. `fieldStates.invalid` declares a `nonColourCue` —
  `aria-invalid`, a warning glyph and the parse failure written out beneath — and the browser gate
  asserts all three are *drawn*. A declared cue that is not rendered fails louder than no
  declaration.

⚠ **A finding, in passing, about a table this child did not own.** `controlStateSpecs.on` borders
with `--theme-accent-border` on an `--theme-accent-surface` fill — **1.20 : 1 in light** — and fills
1.14 : 1 against white; `onHover`'s inset ring is `--theme-accent` on the same fill, **2.98 : 1**.
None of those is an indicator. What actually distinguishes a pressed toggle, a checked box and a
chosen option is the **bold label** the same row declares, which is legitimate — but a reader of
that table would assume the edge was doing it. U03's pairwise gate is green and cannot see any of
it. This child measured it, asserted it (`the borrowed 'on' paint is carried by its weight and not
by its edge`) and did **not** change `control-states.ts`: a two-value edit to another child's
committed model, without its gates in front of me, is the monkey-patch this project refuses by name.

### One tab stop each, by two different mechanisms

U05's rule — *the number of tab stops decides trap-versus-disclosure* — holds for all seven, and
`inputFocusPattern` records which mechanism produces the one stop:

| Pattern | Controls | How |
|---|---|---|
| `activeDescendant` | dropdown, combo box | focus never leaves the field; `aria-activedescendant` names the option |
| `roving` | segmented control | exactly one segment holds `tabindex="0"`; the arrows move it |
| `single` | label, checkbox, measure input, slider | one focusable element and nothing to move between |

The browser gate counts the stops with real `Tab` presses rather than believing the table.

### A dropdown is a `listbox`, and the keyboard follows from saying so

Not a menu. A menu is a set of *commands* whose items take focus (`role="menuitem"`, roving); a
listbox is a set of *values* one of which is chosen (`role="option"`, `aria-activedescendant`). A
dropdown built out of `<mjx-menu>` would announce *"menu, Cambria, menu item"* where a person needs
*"combo box, Cambria, 4 of 18"*.

**`Home` and `End` are the row that matters**, and it is the one the two list controls invert: in a
select-only dropdown they are first and last; in an editable combo box they are **caret keys** and
the list must not steal them. One flag in `ListboxKeyContext` decides both, and both directions are
asserted.

Type-ahead **opens the list rather than changing the value**, which is a deliberate divergence from
the platform `<select>`: a native one fires a change per letter, and each of those would be an undo
entry in an editor.

### The combo box's invariant, asserted after nine paths

> **The value the field shows must be the value the control reports.**

`value` is the only state, the text is `displayTextFor(options, value, allowCustom)` and nothing
else, and the only moment they may differ is while a person is *actively typing* — which is what
`typing` marks. The browser gate runs nine ways of finishing and asserts, per path, either that the
two agree and `typing` is false, or that the path is the one that deliberately ends mid-edit. **The
count of each is asserted too**, because a `finishes` field allowed to drift would turn every
assertion into "the text is whatever it is".

Escape has two meanings and the first needs a recording: with the list open it restores **the text
as it was when the list opened** (a half-typed `Cam`, not the font that was there before); with the
list closed it goes back to the value.

A string the list does not carry has exactly three answers and no fourth: a case-insensitive label
match, the string itself under `allow-custom`, or a **revert that says so** — an `mjx-input-invalid`
event carrying what was refused. Reverting is unavoidable there; reverting *silently* is not.

### The measure input never silently reverts

Points are canonical and every other unit is a factor, so switching the display unit and switching
it back is an identity rather than a rounding. **Both decimal separators are accepted, always** —
the grammar has no thousands separator, so a comma can only mean one thing, and refusing it would
punish a numeric keypad for nothing; the `decimal` attribute governs how a value is *written*.

What it does with a string it cannot read is the whole component: the text stays, nothing is
committed, the field says so three ways at once, an event carries the failure and the fragment that
defeated it, **it stays invalid on blur**, and Escape is the only way out.

### The slider is the identity-value trap in geometry

A slider at its minimum has its thumb at the start of the track whether the arithmetic is right, is
zero, or was never done. So the gate measures the thumb at **five values including both ends**,
compares each against `sliderFraction()` computed in Node — never against the custom property the
component itself wrote — and then asserts the five positions are *distinct*.

`snapToStep`'s stops are the step boundaries **plus the maximum**: a range of 0…10 in threes has
boundaries at 0, 3, 6 and 9, and `End` must still mean ten. Both directions are asserted (9.4 snaps
down, 9.6 snaps up), because "the maximum is reachable" is satisfied by a function that snaps
*everything* to the maximum.

### Four defects the gates found, that review would not have

1. **The measure input silently reverted.** `#finish()` clears `#typing` before it renders, so the
   render overwrote `banana` with `12 pt` — the exact behaviour the component exists not to have,
   with the state set, the event fired and the glyph showing. A field displaying the old value and a
   field that committed the old value are the same picture.
2. **Every option row flex-shrank to 21.25px.** The list is a flex column with a `max-block-size`,
   so rows left at the default `flex-shrink: 1` ignored their `block-size` — and the virtualiser's
   divide-by-row-height was arithmetic over a number nothing on screen had. Found by *a heading is
   exactly one row tall*, not by looking at it.
3. **`aria-activedescendant` named a row that was not in the DOM** after `PageDown`. The window is a
   function of a measured scroll offset and a measurement can be a frame behind. The fix is not a
   better measurement: `#render` now expands the window to include the cursor's row unconditionally.
4. **`aria-setsize` was on the listbox as well as on the options.** It is a *position within a set*
   attribute, which a container cannot be; axe caught it on four stories at once.

### `GUESS:` where this diverges from Office

Three, marked at their sites: **type-ahead opens the list instead of changing the value** (stated
above, and deliberate); the **unit precisions** a value is written back with; and the tri-state
`mixed → true` transition, which is `nextPressed`'s existing `GUESS:` inherited rather than a new
one.

### One widening of a shared helper

`applyAvailability` took an `HTMLButtonElement`, because MJXOFF-182's four archetypes all wrap one.
The inputs do not: a combo box's field is an `<input>` and a slider's track is a focusable
`<div role="slider">`, and neither has the platform's `disabled`. It now takes an `HTMLElement`,
sets the native property where there is one and announces with `aria-disabled` where there is not.
For every caller that existed before, the behaviour is byte-identical — asserted, over a real
`<button>`, rather than left as a claim.

## The surfaces (MJXOFF-188)

`src/surfaces/` holds **six surfaces across three elements** — a modeless dialog, a modal, the sheet
the modal becomes at a phone's width, a popover, a flyout and the task pane. They are six rows of
one table, `surfaceKinds`, and the rows differ in the four things that genuinely differ: *who holds
the keyboard*, *how it can be dismissed*, *where it is put* and *what it sits on*. **Open
`Surfaces/Dialog → Returned To Its Invoker` first, with a keyboard.**

```html
<mjx-dialog label="Save changes" modal open>
  <p>Save your changes to Quarterly Report.docx?</p>
  <div slot="footer"><button>Don’t Save</button><button>Save</button></div>
</mjx-dialog>

<mjx-popover label="Line spacing" kind="flyout">
  <button slot="anchor">Line spacing…</button>
  <input /> <input /> <button>Apply</button>
</mjx-popover>

<mjx-task-pane label="Format Shape" open dock="inlineEnd" document-color="…the page’s colour">
  <mjx-checkbox label="Lock aspect ratio"></mjx-checkbox>
</mjx-task-pane>
```

### A modal that traps focus is easy; a modal that returns it is where the defect lives

Escape, the close button, the scrim and a programmatic close are four paths, and there is **one**
`close()` — in `SurfaceSession` — because four paths with four returns is four chances to have three
of them right. The invoker is captured **once**, at the moment the surface opens, from the deep
active element; it is never re-read on close, when focus is already inside the surface. Every close
event carries `returnedFocus`, and the browser gate asserts the component's report **and** measures
where the keyboard actually landed, per path. A component that said it had returned focus and had
not is exactly the defect the test exists for.

### `inert` and `aria-hidden` are two mechanisms and one without the other is a hole

`inert` removes an element from the tab order and from hit testing; `aria-hidden` removes it from
the accessibility tree and does nothing at all to the tab order. `holdBackgroundInert` writes both,
and the suite asserts both **independently**: real `Tab` presses never leave the dialog, *and* every
background link is inside something carrying `aria-hidden="true"`. Two further things about it:

* **The hold is taken on the host, never on the box that is drawn.** The scrim is a sibling of the
  surface inside the component's own shadow root, and marking it `inert` makes the one click that
  dismisses a modal do nothing — a one-word difference that silently costs a dismissal path.
* **`held` is reported and the gate asserts it is greater than zero**, beside the ceiling it is
  compared against. U07's second defect, in this child's costume: *"nothing outside the dialog is
  tabbable"* is trivially satisfied by a hold that marked nothing.

The walk is the **flat tree**, upward, marking every sibling of every ancestor. `flatTreeParent` is
exported from `overlay/floating.ts` for this rather than copied, because the chain from a dialog to
`<html>` crosses two shadow boundaries and a `parentNode` walk would leave the whole document
tabbable.

### U05's rule needed a third answer, and it is in U05's table

*The number of tab stops decides* is right for every surface that has **taken the keyboard away from
the page**. A modeless dialog and a task pane have not: the document behind them is still live,
still editable, and still the reason the surface is open. They have many stops and must **not** trap,
because Tab leaving them strands nobody — and a task pane has no Escape, no scrim and no invoker to
be trapped back to.

So `focusManagementPatterns` in `src/menus/menu-model.ts` grew a third member, `shared`, purely
additively — U05's two are untouched and its assertions about them still hold. What decides is the
pair *(stop count, is the background still reachable)*, and the two-member table was right for every
surface that existed when it was written.

| Row | Stops | Background | Pattern |
|---|---|---|---|
| popover | one, roving | live | `roving` — Tab leaves, so leaving closes it |
| flyout | many | live | `trap` — Tab is the navigation, so it cannot be the exit |
| modal, sheet | many | inert | `trap` |
| dialog (modeless), task pane | many | live | `shared` — Tab walks out into the page |

`<mjx-popover>` **counts its own stops when it opens** and warns on the console when the count
disagrees with the declared row, exactly as `<mjx-ribbon-group>` reports a group with four essential
commands. The gate counts them with real `Tab` presses and compares the two, because a walk agreeing
with itself is not evidence.

### The scrim, the modal's edge and the sheet's handle are measured, never declared

MJXOFF-269 is the standing record of what a gate that compares code to a model is worth when the
*model* is what is wrong, and U08's remedy is the one applied here: state the **rule** and the
**candidates**, and let the gate sweep and assert the outcome. Nothing in `surface-model.ts` writes
a ratio down.

* **The scrim is the scheme's darkest token**, derived by `darkestThemeMember` rather than named.
  Which token that is differs by scheme, and it has to: an ink scrim in the dark scheme would
  *lighten* the application rather than darken it.
* **A scrim composites anything at all into a two-colour band.** Alpha compositing is monotone per
  channel and luminance is monotone in every channel, so the composite over any colour lies between
  the composite over black and the composite over white — which means a component can compute its
  edge in constant time. That is an argument, so `tests/surfaces.test.ts` runs the closed form
  against the **whole 4,352-colour sweep** and requires them to agree to a hundredth.
* **The edge is the quietest candidate that clears 3 : 1 against that whole band**, and every
  *fixed* alternative is measured beside it and shown to fail in at least one scheme. That is the
  assertion that says the first one could have failed.
* **The handle is chosen by the other half of the same rule**, because the sheet's fill *is* ours
  and maximising there gives a black bar across the top of a dialog. WCAG's floor is a floor, not a
  target.
* **The task pane's edge sits against the user's document**, whose page may be any colour, so it is
  chosen per page from a colour the shell hands in — the same question `chooseSwatchIndicator` asks
  about a swatch, with the same two candidates.

> ⚠ **The two rules are near-duplicates and the check found a real difference.** The first version
> of that test asserted they pick the same member and **failed on the grey line**: on a mid-grey
> page both candidates clear the floor, U08's rule takes the *stronger* and this one takes the
> *quieter sufficient* one. What is asserted instead is the relationship that actually holds — they agree
> exactly wherever nothing is sufficient, and this rule reports `sufficient` exactly when U08's
> clears the floor — with an anti-vacuity assertion that the two do diverge somewhere.

**A measured finding about a table this child does not own, recorded and not changed:**
`--theme-border` on `--theme-surface-raised` — the hairline `surfaceLevels.overlay` already draws
around every dialog, menu and flyout in the catalogue — **does not clear 3 : 1 in either scheme.**
That is legitimate for a decorative edge and it is why the modal's boundary is carried by a
separate, measured outline instead. *(The figure is deliberately not written here: MJXOFF-271's
re-seed moved it while this section was being written, and a number in a document is a number that
stops being true. The gate measures it.)* U07 recorded the same shape about `controlStateSpecs.on` and
deliberately did not edit another child's committed model; this does the same. (`tests/surfaces.test.ts`
asserts the measurement, so the day a re-seed makes the pairing legal the gate says so.)

### The scheme-keyed properties, and the weakest thing in this child

Which member carries the modal's edge is a **light-ish token in the dark scheme and a dark one in
the light scheme**, and no single `--theme-*` name means both. `tokens.css` publishes the *values* for both schemes
unconditionally and the *choice* only through three selectors, and a custom property cannot be
selected by another custom property — so `surfaceSchemeCss` restates those three selectors on
`:root`, and `.storybook/preview.ts` puts it on the document beside `galleryDocumentCss` and
`inputDocumentCss`. Restating them is unavoidable; restating them **unchecked** is not, so
`tests/surfaces.test.ts` reads `tokens/tokens.css` and asserts the three strings still occur in it.

**A shell that calls `defineSurfaces()` and forgets `surfaceDocumentCss` gets a modal with no scrim
at all.**

### The floating primitive proved general, and what had to be added

`overlay/floating.ts` was written by U05 with the instruction that U09 extends it rather than writing
a second placement, and it held. Five things were added, and each is a *third case of something the
file already did* rather than a new idea:

| Added | Why |
|---|---|
| `centreFloating` | a dialog is placed against nothing, so flip-shift-constrain has no anchor |
| `coverFloating` | a scrim is a box the size of the boundary |
| `pinFloating(…, edge, direction)` | a dock is a pin on the inline axis; the three-argument call is byte-identical and a gate says so |
| `syncTopLayer` | the popover dance existed **four times** before it existed once |
| `flatTreeParent`, exported | the background hold needs the same walk the boundary needed |

The fourth is the one worth reading. `<mjx-menu>`, `<mjx-gallery>`, the inputs' `ListSurface` and the
pickers' `SwatchSurface` each carried their own eight-line copy of *"set `popover="manual"`, then
show or hide it"*, and a fifth, sixth and seventh were about to be added. All four now call one
function. `manual` and never `auto` is the load-bearing half: an `auto` popover light-dismisses, and
light dismissal closes the **ancestor** when a descendant opens — which is a submenu inside a menu,
a gallery flyout inside a collapsed ribbon group, and a popover inside a dialog.

### The scrim is a second popover, not a `::backdrop`

`::backdrop` covers the **window**, and the window is not this catalogue's unit of responsiveness. A
modal opened inside a phone-sized frame would dim the whole Storybook page around it and report a
modality the container never had. Both the scrim and the box are `popover="manual"`, and both are
placed against `clippingBoundary()` — the same rectangle the sheet is pinned to and a menu flips
against. The gate measures the scrim against the frame **and** asserts the frame is genuinely
narrower than the window, so the comparison is not satisfied by a frame that happens to be one.

### Stacking, which is the invisible one

`SurfaceStack` holds the **ownership** tree — which surface opened which — because the browser's top
layer already orders what is painted and re-deriving that would be a second opinion about something
the platform has decided. A surface's parent is discovered from the flat tree rather than declared,
so a popover opened from a dialog stacks without either component knowing the other exists. The
invariant is one sentence and is driven over **400 random sequences** in Node, because *closing takes
exactly the descendants* is a claim about all sequences and a browser test can drive one:

> Closing a surface closes exactly its own descendants and nothing else, and the topmost surface is
> always the most recently opened one that is still open.

Descendants close **innermost first**, and that is not cosmetic: an outer surface's close restores
focus, and doing that before its children have let go puts the keyboard somewhere a child is about
to take back.

### A task pane is the one surface that is not dismissible

`surfaceKinds.taskPane.dismissals` is **empty**, `persistentSurfaceKinds` is asserted to be exactly
`['taskPane']`, and the browser gate presses Escape at it and requires it to still be open — an
assertion that is the *opposite* of the one every other surface gets. **A gate that swept all six
together would have inverted this one and stayed green.** It is also in flow rather than in the top
layer (asserted: the pane's box and the document's box do not overlap at all), it is
`role="complementary"` and never `dialog`, and its splitter is ARIA's window-splitter pattern —
`role="separator"`, focusable, `aria-valuenow` as a percentage, arrow keys mirrored by dock **and**
by writing direction.

At a phone width it takes the whole frame and hides its splitter. It is deliberately **not** a sheet:
a sheet is dismissible and this is not, and a third pinned surface saying the same thing is the
duplication this child exists to prevent.

### Two defects the gates found, and two gates the tree found wrong

1. **`deepActiveElement` started at the scope's own root**, which is `null` whenever focus is on a
   node *slotted* into that root — and every control inside a dialog is slotted. `wrapTab` therefore
   never recognised the last stop and a modal's Tab walked straight out of it, with every other
   assertion in the suite green. Caught by pressing Tab twice as many times as there were stops and
   counting distinct landings.
2. **The task pane's phone-width block was emitted above the base rules.** The container condition
   changes nothing about specificity — both splitter selectors score (0,1,0) — so source order was
   the whole arbitration, and the pane reported `full` at a phone width *and still drew its
   splitter*, with the presentation assertion green. MJXOFF-183's accident arriving by the one route
   its own fix does not cover, because here it is the *base* rule that would need the `:where()`.
   `tests/surfaces.test.ts` now asserts the order **and** that the two selectors are of equal
   specificity, because U04 is the record that order arbitrates nothing between rules that are not.

And two of this child's own gates were wrong before they were right, both in the way the ticket
warned about — *"if a gate of yours would break when a colour changes, it is the wrong gate"*:

* **"the two schemes pick different scrim members"** is a fact about the palette, not about the
  rule. Replaced by the property that actually matters — *nothing in a scheme is darker than that
  scheme's scrim* — which no re-seed can break for the wrong reason.
* **"no single member could have served both schemes"** failed on its first run, and the failure was
  the useful part: ten members darken in both, so a fixed choice would have worked. The derivation's
  value is that it is a **guarantee** rather than a coincidence, and MJXOFF-271's re-seed proved the
  point mid-write — under the previous palette an ink scrim genuinely did lighten the dark scheme,
  and under the current one it does not.

A third, in the same family: **"a scrim at half opacity leaves some scheme insufficient"** was
replaced by the monotonicity it was reaching for — a thinner scrim composites into a wider band, so
it can never improve a candidate anywhere, and it strictly worsens at least one. True of every
palette, and it still says the opacity is load-bearing.

### `GUESS:` where this diverges from Office

Two, marked at their sites: **a modal can be dismissed by clicking its scrim** (Office's dialogs are
OS windows and cannot be, but a web modal that traps the keyboard and refuses every pointer gesture
is the shape people report as a hung page — and `dismissals` makes it a one-line decision per row);
and **the popover/flyout split is decided by tab-stop count rather than by Fluent's own taxonomy**,
which is this catalogue's rule applied consistently rather than Office's naming reproduced.

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

  ⚠ **MJXOFF-185 found the other side of that race and closed it.** A single idle check answers
  *"is a run in flight right now"*, and the addon's run may not have **started** yet — so one poll
  sees an idle axe, the sweep starts its own, and the addon's begins underneath it. It showed up on
  a story heavy enough to delay the addon (seven galleries, fourteen listboxes, fourteen shadow
  roots), passing on its own and failing inside the full suite, which is the signature of a race
  rather than of a defect. `waitForAxeIdle` now requires the idle state to **survive a frame**, and
  `a11y.spec.ts` calls it once more immediately before `analyze()`. Neither is a retry: what is
  being waited for is still *the addon has finished*, and the second look is what makes the first
  one mean it.
* **`public/plates/` is a fixture, not a render.** The real plates come from
  `cargo run -p mjx-render-oracle -- gallery <dir>`, which builds `mjx-paint` and links the
  platform's graphics stack; making the catalogue depend on that would put a Vulkan toolchain
  between a TypeScript contributor and a green run. Regenerate the fixture with
  `node scripts/make-sample-plate.mjs`. `<mjx-plate-gallery src="…">` loads a real directory
  unchanged.
* **The visual baselines were generated by the code under test** and no person has approved them.
  They lock the current appearance against accidental change; they do not assert it is right.
* **A `<style>` string is a template literal, so a backtick inside a CSS comment ends it.** Two of
  MJXOFF-186's edits did exactly that and produced a wall of parse errors a hundred lines from the
  actual mistake. Write CSS-comment prose without backticks.
* **A flex column with a `max-block-size` shrinks its children.** Anything inside a scrolling popup
  whose height is part of an arithmetic contract needs `flex: 0 0 auto` as well as a `block-size`.
* **A backtick inside a CSS comment ends the template literal — for the third time.** MJXOFF-188
  lost a build to a paragraph explaining a specificity accident, written with the usual backticks
  around selector names. Write CSS-comment prose in plain words.
* **`document.activeElement` on a *shadow root* is `null` for anything slotted into it.** A focus
  trap that asked the surface's own root where the keyboard was therefore saw nothing, and let Tab
  walk out of a modal while every other assertion stayed green. Start at the document and descend.
* **A `@container` block changes no specificity**, so a presentation block emitted above the base
  rules it overrides simply loses. Emit presentation blocks last, and assert the order *and* the
  equal specificity that makes order matter.
* **`color-mix()` computes to `color(srgb …)` in Chromium**, not to `rgba(…)`. A gate that compared
  a computed colour as a string fails on a page painting exactly the right thing.
* **Nothing in `src/surfaces/` may declare a colour, and its own gate is stronger than the lint's**:
  a grep for a hex over the whole file, comments included. The two sRGB-cube extremes the scrim
  arithmetic needs are built from their channels instead.
* **A tab-stop helper whose failure mode is "found nothing" makes every ceiling assertion pass.**
  MJXOFF-186's first version broke its walk on a landing at `<body>` and reported *zero* stops for a
  pane that has two, which read as a passing filter rather than as a broken helper.
