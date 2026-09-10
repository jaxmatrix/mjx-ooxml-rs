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
    foundations/     the type scale, elevation ladder, focus ring, motion vocabulary, density, the
                     splitter arithmetic and the ONE virtualisation (extent-table.ts,
                     virtual-list.ts)
    controls/        the ribbon archetypes: button, toggle, split button, dialog launcher
    furniture/       the chrome around the document: status bar, zoom control, scrollbar, splitter
    navigators/      the four ways through a long document: virtual list, tree, thumbnail rail,
                     sheet tab bar — all four on one windowed scroller
    formula/         Excel's formula bar and name box, and the caret-relative model under them:
                     the tokeniser, the reference-colouring contract and the argument tooltip
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

## Selection and feedback (MJXOFF-189)

`src/feedback/` holds **five components across six elements** — the mini toolbar that appears beside
a selection, the enhanced screentip, the toast queue and its declarative descriptor, the two kinds of
progress bar, and the empty state. They are one child because they share one property, and it is the
property everything below is arranged around:

> **Every one of them is invisible in a still.**

A screenshot of a screentip cannot say whether it waited. A screenshot of a toast cannot say whether
it was announced, or to whom. A screenshot of a progress bar cannot tell a task that is working from
a task that has stopped. So this family has almost no visual gates and a great many temporal and
accessibility-tree ones.

```html
<mjx-mini-toolbar label="Formatting" for="selected-run" open></mjx-mini-toolbar>

<mjx-screentip heading="Bold" description="Make the selected text bold." shortcut="Ctrl + B">
  <button slot="trigger">Bold</button>
</mjx-screentip>

<mjx-toast-region label="Notifications">
  <mjx-toast tone="error" message="Could not reach the server."></mjx-toast>
</mjx-toast-region>

<mjx-progress label="Uploading files" value="3" max="7" readout></mjx-progress>
<mjx-progress label="Contacting the server" indeterminate></mjx-progress>

<mjx-empty-state heading="No comments yet" description="…"
                 action-label="Add a comment" action-command="comment.add"></mjx-empty-state>
```

### A span of time is a multiple of the one duration token

The generated table declares exactly one duration. This child needs five more — a screentip's appear
delay, its warm-up window, its leave grace, a toast's dwell and an indeterminate bar's cycle — and
every one of them is `calc(var(--duration-transition) * n)` rather than a number. That is the same
answer `typography.ts` gave to a two-size type scale, plus one reason of its own: a delay written as
a **ratio** follows a host that slows the platform down for someone who needs longer to read, and a
delay written as a number does not.

Getting a number back out needs the property to be **registered**. `getComputedStyle` on an
*unregistered* custom property returns the substituted text `calc(150ms * 4)`, and `parseFloat` on
that returns `150` — a delay four times too short that looks like nothing in a diff. So
`feedbackTimingCss` registers all six with `syntax: '<time>'`, and `resolveDurationMilliseconds` in
`foundations/motion.ts` checks the unit rather than trusting `parseFloat` and returns **`undefined`**
rather than a plausible number when it cannot read one. Every caller falls back to the *generated
token value* and never to zero, because a zero delay would turn this child's central contract into a
claim that cannot fail.

### `placeClearOfAnchor`: the thing `placeFloating` does not give you

`overlay/floating.ts` grew one function, and the reason is worth stating because it looks at first
like the primitive already covered it. `placeFloating` puts a box beside its anchor with a gap and
then **clamps it inside the boundary** — so a box that fits on neither side of its anchor is pushed
back *onto* the anchor rather than off the frame. For a menu hanging off a button that is the right
answer; a menu covering its own button is a menu you can still read. For a **mini toolbar**, whose
entire purpose is to act on the thing underneath it, and for a **screentip**, which describes the
control it would be drawn over, it is the defect itself.

So `placeClearOfAnchor` asks the primitive once per side, in the caller's order, and returns the
first placement that overlaps the anchor by **nothing at all**. No second placement arithmetic: every
candidate is a `placeFloating` call and the overlap is `intersectRects`.

⚠ **And it reports failure rather than hiding it.** A selection that fills its room has no clear side
and no arithmetic can invent one, so the least-overlapping placement comes back with `clear: false`
and the area beside it. `<mjx-mini-toolbar>` writes that on its host as `data-covering="true"`, and
`Feedback/Mini Toolbar → A Selection With No Room` exists **so a gate can watch it happen**. A
function that silently returned its least-bad answer would satisfy every *"the toolbar never covers
the selection"* assertion by making that assertion unfalsifiable.

### A tone is never carried by colour alone, and the palette is why

This brand has one alarm colour and no red. Adding one would override the palette of whoever opens
the editor, which is the project's standing rule about deferring to the user's own document applied
to its chrome. So a toast's tone is carried by **four** signals — the glyph, its variant, the
politeness and the dwell — and the colour is the weakest of them.

That is measured rather than asserted, and the measurement is the argument: `theme.light.accent` and
`theme.light.secondaryAccent` differ in hue and are within **1.03 : 1** of each other in *luminance*.
A contrast ratio sees luminance and nothing else, so those two colours are **identical to a contrast
gate** — and to a reader with a colour deficiency they may be close to identical too.
`tests/feedback.test.ts` asserts the separation is *below* the non-text floor, which is the honest
way round: it records the fact the design is built on, and it will fail loudly if a re-seed ever
makes colour a usable signal.

`warning` and `error` therefore share that one colour deliberately, and are told apart by the drawing
(regular against filled), by the politeness, and by the fact that an error does not dismiss itself.

### Two live regions, never one whose politeness is rewritten

A live region's politeness is settled when it enters the accessibility tree. Rewriting `aria-live` on
one region per message produces — depending on the screen reader — either an announcement nobody
hears or a polite message that interrupts, and both are invisible on screen. So `<mjx-toast-region>`
builds **two** visually-hidden regions that never change, `role="status"`/`polite` and
`role="alert"`/`assertive`, and routes by tone.

The gate is a single sample of both: the polite region **has** the message and the assertive one is
**empty**. Both halves are needed — *"the assertive region is empty"* on its own is satisfied by a
component that announces nothing at all.

The two regions live **outside** the popover that draws the stack, because a popover that is not
showing is `display: none` and a live region inside one would work exactly until the stack emptied
once.

### The queue is pure and the region wires a clock to it

Everything that can go wrong with a stack of timers — a second toast cancelling the first one's
timer, an error being pushed off by three confirmations, the order reversing — is a property of a
*sequence of instants*. `ToastQueue` takes `now` as an argument and is swept in Node over a dozen of
them; the browser proves only the one thing Node cannot, that real time reaches it.

⚠ **One `setTimeout` for the whole queue, armed at `nextExpiry()`, and not one per toast.** Not an
optimisation: a timer per entry is the shape in which *"showing a second toast restarted the first
one's clock"* hides, because the bug is then a missing `clearTimeout` rather than an arithmetic
error — and the arithmetic is the part a test can see.

⚠ **A card already on screen is kept, never rebuilt**, and the same rule holds for the mini
toolbar's buttons and the empty state's glyph. The obvious `replaceChildren(...entries.map(card))`
is one line shorter and wrong in a way neither tier would catch: every card carries an entry
animation, so rebuilding the list on each push makes *every* toast fade in again whenever a new one
arrives — with the counts, the order, the announcements and the timers all still right. The mini
toolbar's version of it is worse than cosmetic: rebuilding the row destroys the button the keyboard
is on, so a toolbar retitled while a person was using it would silently lose focus.

The ceiling retires **the oldest entry that leaves on its own** before it touches a persistent one.
An error a person has not read must not be shunted off the screen by three “Saved” messages, and a
ceiling that simply took the head of the queue would pass a size assertion and lose the only message
that mattered.

### An indeterminate bar is told from a stalled one three ways, because one of the three is optional

| Instrument | Determinate | Indeterminate |
|---|---|---|
| the accessibility tree | `aria-valuenow`, and a percentage in `aria-valuetext` | **no** `aria-valuenow` at all; the text says *working* |
| geometry | the indicator's width **is** the value | a fixed span of the track, which is no value |
| time | still | its position differs between two samples |

`Feedback/Progress → Told Apart From A Stalled Bar` parks a determinate bar at *exactly* the fraction
the indeterminate indicator spans, so the two are the same width and a width comparison decides
nothing about either. The browser gate then samples both indicators at two instants a quarter of a
cycle apart — and runs the whole thing again under an emulated `prefers-reduced-motion`, where
**neither moves** and only the accessibility tree is left. That is why the missing `aria-valuenow` is
removed rather than set to `0`: a bar reporting zero would announce a task that has made no progress,
which is a different and false claim.

The determinate half is the identity-value trap. `progressFraction` has six branches — two clamps, a
non-unit maximum, a zero maximum and two non-finite cases — and *any single value visits at most one
of them*, so the unit tier drives a table of twelve and the browser measures five painted bars,
each against **its own track** rather than against the bar above it.

### A screentip's description is announced whether or not the tip is drawn

⚠ **And that is why the description lives in the light DOM.** An IDREF resolves within its own tree,
so `aria-describedby` on a *slotted* trigger cannot name an element inside the shadow root of the
component that slotted it — the trigger is in the document's tree and the tip is in a descendant
tree, which is the one direction ARIA element reflection does not reach either. A tip whose text
existed only in the shadow root would be perfectly visible and **completely unannounced**, with every
visual assertion green.

So the component writes one `<span>` as its own light-DOM child, assigned to no slot — which is what
hides it, because a node **directly referenced** by `aria-describedby` still contributes its text
even when it is not rendered — and points the trigger at it. It is there the whole time, which is
also what WAI's tooltip pattern asks for: the delay exists for eyes and costs a screen-reader user
nothing. The drawn tip is `aria-hidden`, so nothing is announced twice.

The pointer waits and the keyboard does not: a pointer crosses a toolbar on its way somewhere, and a
keyboard arrives on a control because a person put it there. The warm-up window then removes the wait
for the *next* trigger, which is what makes a row of icon commands readable rather than a row of
things that will not tell you what they are. Escape hides the tip and **moves no focus at all**.

### An empty state is the easiest thing to ship wrong

It renders perfectly with no data by definition, so every visual check passes on one whose action
does nothing. `Feedback/Empty State → The Action Does The Thing` wires the button to the real list
rather than to a counter, and the gate presses it and counts what appeared. A story whose action only
recorded that it had been pressed would prove the event fires and nothing about whether pressing it
achieves anything.

It is `role="status"` with `aria-live="polite"`, and that is a decision rather than a default: an
empty state is what a region *becomes* when a filter, a search or a delete emptied it, so its
appearance is an update and a person who cannot see it is otherwise told nothing. The art is
`aria-hidden`, the heading is real text, and the heading is the **one** place in this platform the
serif is allowed — `typography.ts` §4, enforced by a sweep of `src/` that allows exactly two files to
name `typeRoleClass('display')`.

### Four defects the gates found

1. **`hidden` did not hide.** `<mjx-empty-state>` hides its action when there is no action to offer.
   The attribute was set, the accessibility tree was right, and the button was on screen — because
   the UA's `[hidden] { display: none }` lives in the *user-agent* origin and **any author rule at
   all beats it**, so `.action { display: inline-flex }` had silently switched `hidden` off for every
   element wearing that class. An assertion on the attribute would have passed; only asking the
   browser whether the thing is *visible* caught it. Every sheet now restates `[hidden]` at the same
   (0,1,0) the class rules score and **last**, which is the only thing that decides a tie at equal
   specificity — and `tests/feedback.test.ts` asserts the position, because this is U09's `@container`
   defect wearing a different costume.
2. **`ClearPlacement.tried` reported the wrong number on failure.** It named the position of the
   least-bad candidate rather than the whole order, which made a genuine four-sided failure
   indistinguishable from a search that stopped early — the one thing the field exists to tell apart.
3. **Escape had nothing to give the keyboard back to.** The mini toolbar captured its invoker when it
   *opened*, and a toolbar rendered with `open` in its markup opens while focus is on `<body>`. It now
   captures on the way **in**, from `focusin`'s `relatedTarget`, which is both simpler and the honest
   rule: a person who never Tabbed into the toolbar has not moved, so there is nothing to return.
4. **`aria-valuenow` announced `0.9999999999999999`.** `<mjx-progress>` recovered the value from the
   fraction it had just computed — `fraction * max`, which is the obvious spelling — and
   floating-point division does not always give it back. One of forty-nine is such a pair, and an
   assistive technology reads that number out in full. Every value in the catalogue's own stories
   divided exactly, which is precisely how it would have survived the suite; the story now carries a
   `value="1" max="49"` bar for no other reason.

### `GUESS:` where this diverges from Office

Three, marked at their sites. **The mini toolbar's commands are data rather than slotted controls** —
Office's is a fixed set and ours is a property, because a toolbar built out of slotted `<mjx-button>`s
cannot hold a roving tab stop: sequential focus descends into a shadow tree whatever `tabindex` the
host carries, so every command would have been a tab stop with no way to fix it. **`warning` and
`error` share one colour**, because this palette has one alarm colour and inventing a red would
override the branding of whoever opens the file. And **an error toast is persistent while the other
three dwell**, which is Fluent's behaviour rather than a rule anyone has written down about Office.

## Document furniture (MJXOFF-190)

`src/furniture/` holds the chrome that frames the document surface — `<mjx-status-bar>` with
`<mjx-status-segment>`, `<mjx-zoom-control>`, `<mjx-scrollbar>` with `<mjx-scroll-mark>`, and
`<mjx-splitter>`. Open `Furniture/Scrollbar → An Estimate Being Corrected` first, **with a pointer**.

```html
<mjx-status-bar label="Document status">
  <mjx-status-segment label="Page" value="4 of 20" priority="essential"></mjx-status-segment>
  <mjx-status-segment label="Words" value="3,182" priority="supplementary"></mjx-status-segment>
</mjx-status-bar>

<mjx-zoom-control percent="100" viewport-width="1000" viewport-height="700"></mjx-zoom-control>

<mjx-scrollbar label="Document" controls="canvas" pages="20" page-height="1000" viewport="4000">
  <mjx-scroll-mark kind="search" page="3" within="0.2" label="pipeline"></mjx-scroll-mark>
</mjx-scrollbar>

<mjx-splitter label="the navigator" controls="navigator"
              min-start="0.15" min-end="0.3" storage-key="word/navigator"></mjx-splitter>
```

### The splitter is a *lift*, not a third implementation

MJXOFF-188 built an ARIA window splitter inside `<mjx-task-pane>` and put its arithmetic in
`surface-model.ts` under task-pane names. This child needed the same keyboard contract with
different bounds, so the arithmetic moved to **`src/foundations/splitter.ts`** parameterised by its
bounds, and `surface-model.ts`'s `clampFraction`, `fractionFromDrag` and `fractionFromKey` are now
**one-line bindings** over it — same signatures, same behaviour, no caller changed.
`tests/furniture.test.ts` asserts the equivalence over a sweep of 200 fractions, 172 drag positions
and every key in both directions, rather than leaving it as a claim: if a binding ever drifts, the
task pane and the splitter disagree about what an arrow key does, which is exactly the defect two
copies would have had.

### The scrollbar is R13's requirement seen from the front

`crates/mjx-view/src/scroll.rs` (MJXOFF-168) gives a document a scrollbar the instant it opens by
*estimating* how long it is, then corrects page heights one at a time as they are laid out.
`src/furniture/scroll-model.ts` is the same design in this language, with the same names —
`totalHeight`, `offsetOf`, `anchorAt`, `offsetOfAnchor`, `recordMeasuredHeight`, `measuredPages` —
so a reader holding both files open can check them line by line.

**There are two invariants and they are true at different moments:**

| When | What is the state | What is derived | Why |
|---|---|---|---|
| not dragging | the **anchor** — a page and how far into it | the offset | a correction above the reader must not move the page under them |
| dragging | the **pointer** | the offset | the point of the thumb under the finger must not move while the thumb changes size |

`offsetUnderGrip` is the second one and it is three lines. The naive alternative — keep the offset,
recompute the thumb from it — is what **both** suites compute *beside* the real answer and assert
differs from it. Without that positive control a stability assertion is a tolerance nobody tested.

The browser gate holds a real mouse button down through Playwright's own input path while the extent
is revised underneath it, because the failure mode is temporal: a story with a fixed content height
proves nothing at all, and a synthetic event sequence proves a handler exists rather than that the
platform would ever call it.

### A status bar is a live region, and the failure is announcing too much

A page number that changes as a person scrolls is the reason the bar exists and it changes
constantly. So `announce` **defaults to `off`** and a segment opts *in*; `polite` lands in the
`status` region and `assertive` in the `alert` region, and the two regions are separate elements
from the first render — MJXOFF-189's rule, because a live region's politeness is settled when it
enters the accessibility tree. The gate advances the page number, asserts the value really changed,
and asserts **both** regions are still empty.

### Nothing is dropped — a demoted segment is the same element, moved

MJXOFF-183's rule applied to a second container. The ladder is generated `@container` blocks writing
`--mjx-status-presentation` onto one hidden probe per priority; the component reads those back and
reassigns the segment's `slot` from its region to the overflow panel. **The same DOM node**, with
the same identity, listeners and accessible name. The gate stamps every segment at the desktop
width and reads the stamps back at the phone width, because a *count* cannot tell a moved element
from a re-rendered copy.

The `ResizeObserver` in there decides nothing: CSS cannot reassign a slot, so something has to ask
the cascade again. The presentation itself is still CSS's, which is what makes the comparison
against `statusPresentationAt()` meaningful.

### Eight defects, and only six of them were the gates

1. **The `@container` blocks would have lost.** The base rule was a bare `.probe` at (0,1,0) against
   the generated blocks' `:where(…)` at (0,0,0), so the probe would have reported `shown` at every
   width for ever — with the component and the model silently disagreeing and every other assertion
   green. It is MJXOFF-183's specificity accident for the **fourth** time. Caught while writing the
   unit gate, which now asserts the `:where()` wrapping *and* the emission order.
2. **The splitter's fraction reached nothing.** A custom property inherits *down*, and the two
   regions a splitter sits between are its **siblings** — so a fraction written only onto the host
   is read by nobody, and a story sizing its navigator from `var(--mjx-split-fraction, 0.3)` keeps
   the fallback for ever. Every attribute and every announced number was already correct; the
   assertion that caught it was *the region got wider*. The fraction is now written on the
   **boundary** as well, which is the honest place for it.
3. **`box-sizing: border-box` cannot shrink a box below its own padding.** A collapsed region at
   `inline-size: 0` still drew a 24-pixel stripe of surface, twice — once for `content-box` and once
   for the padding floor. Hence `--mjx-split-collapsed`, so a region can zero its own padding.
4. **The scrollbar was squeezed to 12 pixels.** A flex item's default is `flex-shrink: 1`, so a
   scrollbar beside a canvas that wanted the room lost half its thickness — and the hit-target
   floor, which is a promise about pixels on screen, quietly stopped being true. U07 found the same
   default in the other axis.
5. **`.mjx-hit-target` made the splitter's handle wider than its host.** That class sets a *minimum*
   of 40 CSS pixels in comfortable density; on a 24-pixel divider it overflowed. The floor belongs
   on the host, where `<mjx-task-pane>`'s splitter puts it.
6. **The first mid-drag test passed its way to a silent zero.** At the desktop preset the harness
   frame is 1440 wide inside a 1280 viewport and scrolls horizontally, so the scrollbar sat at
   x = 1400 — off screen, where a real mouse cannot reach it and every pointer event was delivered
   to nothing. `before.offset` was `0` and the drag had never happened.
7. **A backtick inside a CSS comment ended the template literal, for the fourth time.** *"Named,
   never `all`"* in a `transition-property` comment, and a wall of parse errors a hundred lines
   away. Write CSS-comment prose in plain words.
8. **The zoom readout's invalid edge was never painted.** `fieldStates.invalid` matches
   `%s[data-invalid]` and there is **no `data-state` spelling of it**, so a field put into
   `data-state="invalid"` is in a state the table paints nothing for. All three non-colour cues
   were correct and the gate was green, because the gate counted cues and did not measure the edge.
   It now measures it — as a *difference* between two computed colours rather than a remembered hex,
   which is what the palette re-seed taught. Found by reading U07's table rather than by a gate,
   which is the honest way to say it.

### Two things that are deliberately *not* animated

The scrollbar thumb wears **no motion class at all**, which is the only component in the catalogue
that does not. `transition-property` defaults to `all`, so a settle class would ease the thumb's
position and size — the thumb would lag the finger dragging it, and the mid-drag gate would be
measuring an interpolated box. A gate that can be satisfied by an animation halfway through is not
a gate. The splitter's *line* does wear one, and names the property it transitions.

### One widening of a shared helper

`measure.ts` grew `readTypedNumber`, and `parseMeasure` now calls it. A zoom readout takes a
percentage rather than a length — there is no `%` among the six units and a percentage has no value
in points — but what it *must* share is how a number is read: both decimal separators, no thousands
separator, and `-`, `.` and `1.2.3` each landing in the invalid path with the text preserved. A
second regex would have been a second answer to *"is `1.2.3` a number"*.

### `GUESS:` where this diverges from Office

Three, marked at their sites. **The zoom slider is linear** and Word's has a detent at 100 % in the
middle of its travel; the alternative was to give `<mjx-slider>` a value space that is not the
percentage, and then its `aria-valuenow` — the number a screen reader reads — would have been a
track position rather than a zoom. **A second double-click restores rather than resets**, which is
this catalogue's reading of Office's collapse. And the **status ladder's widths** are ours: the
*shape* is Office's behaviour (items leave as the window narrows) but no number is checked against
it.

## The navigators (MJXOFF-191)

`src/navigators/` holds the four ways a person moves through a document that does not fit on a
screen — `<mjx-virtual-list>`, `<mjx-tree>`, `<mjx-thumbnail-rail>` and `<mjx-sheet-tab-bar>`. Open
`Navigators/Virtual List → A Thousand Rows Arrive Above You` first, **and scroll it before you press
the button**; that story is the one a screenshot cannot show anything about.

```html
<mjx-virtual-list label="Comments"></mjx-virtual-list>
<mjx-tree label="Navigation"></mjx-tree>
<mjx-thumbnail-rail label="Slides"></mjx-thumbnail-rail>
<mjx-sheet-tab-bar label="Sheets" value="s1"></mjx-sheet-tab-bar>
```

**All four take their contents from a *property*** — `.items`, `.nodes`, `.slides`, `.tabs` — and not
from markup. U06's `<mjx-gallery-item>` descriptor is the right answer for forty styles whose art is
arbitrary; it is the wrong answer for five thousand rows, because a descriptor per item means five
thousand light-DOM elements before anything is drawn, which is the cost virtualisation exists to
avoid paid in the one place a virtualiser cannot help.

### U06's virtualisation was **lifted**, twice, and the equivalence is asserted

The ticket said to read it first and reuse or lift it. Both halves moved *down* into the foundations,
which is MJXOFF-190's own pattern from `foundations/splitter.ts`:

| What | From | To | Why it had to move |
|---|---|---|---|
| `galleryWindow` | `gallery/gallery-model.ts` | `foundations/virtual-list.ts` as `virtualWindow` | four navigators need it and nothing may reach a component for it |
| the prefix-sum table | `furniture/scroll-model.ts` | `foundations/extent-table.ts` as `ExtentTable` | a list of five thousand slides and a scrollbar over five hundred pages are the same arithmetic |

`galleryWindow` and `ScrollbarModel` are now **bindings**, with the same signatures and the same
behaviour, and `tests/navigators.test.ts` asserts both over sweeps rather than leaving it as a claim
— 1,120 calls for the first and 200 random operation sequences for the second. Nothing changed for
the gallery, the font list or the swatch grid.

**The precondition U07 found is gone rather than documented.** `galleryWindow`'s callers all computed
their first visible row by dividing a scroll offset by a row height, which silently requires *every
row to be exactly one row tall* — the thing that cost MJXOFF-186 two of its four real defects. A tree
with wrapping headings, a rail whose section breaks are shorter than its slides, and a caption under
a thumbnail cannot honour it at all, so `windowForOffset` replaces the division with a **binary
search over an `ExtentTable`** and the precondition is now a property of one entry point instead of
an invisible property of every caller.

**What a virtual list needed that a scrollbar did not** is two things, and they are why this is a
lift and not a rename: `insertAt`/`removeAt` (a scrollbar's document only ever grows at the *end*),
and a **keyed** anchor. `ScrollAnchor` is `{ page, within }`, and an index is precisely what an
insertion above it changes.

### The trap, answered: hold an anchor, derive the offset — and the anchor is a key

> A slide inserted above the visible range must not move what the reader is looking at.

R13's design one level up. `KeyedAnchor` is `{ key, within }`, `offsetOfAnchor` resolves the key
against the *current* ordering, and the measured extents are carried across an insertion **by key**
so the rows below keep the heights they were measured at. Both tiers assert it **beside the naive
answer** — `naiveOffset` is exposed on all four components for exactly that reason — because U11's
rule is that without a positive control a stability assertion is a tolerance nobody tested. The
browser gate measures the on-screen position of a *named* row before and after a thousand rows
arrive above it.

### Three ARIA patterns, and the browser is asked which

`navigatorAriaPatterns` is the table, and it says *why* for each. `tree` / `treeitem`, `listbox` /
`option` (twice, once multi-selectable), `tablist` / `tab`. The browser gate asks **Playwright's role
engine** rather than reading the component's own attribute, which is U04's rule applied to ARIA.

Three consequences worth carrying forward:

* **`aria-setsize` and `aria-posinset` go on the row and never on the container** — U07's finding, and
  in a virtualised list they are the *only* honest source for the count, because the number of rows
  in the DOM is not the number of items.
* **A leaf carries no `aria-expanded` at all.** `false` would announce a branch that does not exist.
* **A rail's section is a heading row, not a `role="group"`.** A group announces how many it owns and
  a window holds only some of a section's slides, so a group would either claim rows that are not in
  the DOM or report a count that changes as the reader scrolls. The section is folded into each
  slide's accessible name instead, which is true at every scroll position. Marked `GUESS:`.

### Reordering: one function, two ways in

Drag calls exactly what the keyboard calls, one step at a time. Two implementations of *move* is how
a pointer path and a keyboard path come to disagree at the ends of a branch — and **moving a heading
moves its subtree** falls out of the model operating on the tree rather than on the flattened rows. A
keyboard implementation that swapped two entries in the visible list would leave the children behind
under whichever heading ended up above them, and it would look correct until somebody collapsed the
branch. A rail moves its **whole selection as a block**, because a per-slide swap quietly turns a
scattered selection into a contiguous one on the first press.

**A refused reorder is announced.** *Already at the top level* is information; silence is a key that
appears not to work.

The browser gate makes the same move **twice, once by each road** — a real mouse through Playwright's
own input path, and then `Alt + Arrow` from the same starting order — and compares the two outcomes
to each other rather than to a list written in the test. That is the claim the design rests on, so it
is the claim that is checked.

### Eight defects the gates found

1. **The cursor row widened the window, and the list stopped virtualising.** U07's answer to *the
   cursor must always be in the DOM* is `firstRow = min(firstRow, cursor)`, and it is right for a
   font list, where the cursor moves *because* the list scrolled. Here a pointer scrolls five
   thousand rows with the cursor still on row 0 — and the browser gate measured **2,504 rows in the
   DOM**, with the resting story's node count still perfect and every other assertion green. The
   cursor's row is now built as **one absolutely-positioned element** at its true offset when it
   falls outside the window. Same invariant, constant cost.
2. **The stable offset was computed one step too early.** `setKeys` worked out where to scroll to
   *before* rendering — and rendering is what measures the rows, so the answer was an answer to
   extents the render then corrected. The watched row moved **265 pixels**: far too little to look
   like a defect, far too much to be right, with the anchor, the keys and the arithmetic all correct.
3. **`scrollIntoView` scrolled the page.** The browser's own reveal walks *every* ancestor scroll
   container, and at a container width wider than the viewport, pressing *Scroll to the last sheet*
   moved the whole catalogue sideways by sixteen pixels. The strip is now scrolled by arithmetic, in
   one place, so the arrow keys and the affordances cannot disagree.
4. **The fixtures could not reach their own hard case.** Two of them: the tree's long headings are on
   its *leaves*, so a story that opened only the chapters showed **one distinct row height out of
   five thousand**; and at a full desktop width nothing in a navigator wraps at all, which is why
   `paneStyle` exists and is 22rem. A fixture that cannot reach its hard case makes every assertion
   over it weaker than it looks — U10's *"a fixture set of convenient numbers cannot see this class
   of defect"*, arriving as a layout rather than as a number.
5. **A dimmed *hidden* label measured 2.79 : 1**, against a floor of 4.5, and the a11y sweep failed
   all four sheet-tab-bar stories on it. The reasoning that made it look safe is the trap: a
   **native disabled** control is exempt from axe's contrast rule *because the platform dims it and
   nobody is expected to read it*. A hidden sheet is not disabled — it is reachable, selectable and
   showable — so nothing exempts it and nothing should. The state is now a dashed edge, a glyph and
   the word in the accessible name, three cues at full contrast. *(The rail never dimmed its hidden
   slides — it marks them — so it was already right, and the sweep says so by passing. That is worth
   recording rather than claiming a second fix: the two were written a day apart and only one of them
   reached for an opacity.)*
6. **The phone gate measured the wrong box.** `<mjx-resizable-container>` fills the page and resizes
   a `.frame` inside its own shadow root, so measuring the *host* reported 1,280 at every preset —
   a phone assertion that was really comparing the viewport against a breakpoint that is not about
   the viewport. U01's own rule (*the container is the mechanism, not the viewport*) as a
   measurement mistake rather than as a design one. The rail's thumbnail box was also called
   `.frame`, which turned the corrected locator into a strict-mode violation; it is `.plate` now.
7. **A bounding box read before the press was a box that had moved.** The first drag gate read the
   row's rectangle, moved the mouse there and pressed — on the **stage**, twelve pixels below the
   tree, because the list measures the rows it built and re-derives its offset from the anchor, so
   the first correction after a render shifts every row. The failure said only that the order had
   not changed. `hover()` first, then read the box.
8. **This child broke a test in U07's suite without touching a line of it.**
   `inputs.spec.ts`'s *"the fifth state is reachable"* opens its dropdown from a
   `requestAnimationFrame` and then presses `ArrowUp` twice — which means *move the cursor* only
   while the list is open, and *change the value* when it is not. Four more elements in `preview.ts`
   and twelve more stories in the catalogue pushed that frame past the test's wait, and the failure
   reported a row count of zero and said nothing whatever about the list being shut. The precondition
   the test's own comment already assumed is now asserted. **A test that depends on an unstated
   precondition fails, eventually, for a reason that is not in its message** — which is U07's own
   one-row-tall precondition wearing a temporal costume.

### The touch presentation, and who owns the layout

At a phone width the tree and the rail publish `--mjx-navigator-presentation: sheet` and lose their
corner; the tab bar publishes nothing and stays a tab bar, growing its targets through density. The
component reports the presentation and **a shell lays out accordingly** — the same division a ribbon
group already has, and the honest one: a navigator's width belongs to the application, not to the
navigator. Wiring a real shell to it is loop 2.

⚠ **In flow, and deliberately not `position: fixed`.** U05 measured what makes the obvious spelling
wrong — Chromium does not make a `container-type` element a containing block for fixed descendants —
so a pane pinned with `inset: 0` inside the harness frame spans the *window*.

The browser gate compares the published presentation against `navigatorPresentationAt()` at both
presets and cross-checks it against the **radius**, which is geometry the custom property does not
decide. That radius therefore lives in the component and not in the story's inline style: an inline
radius written by a shell would beat the container rule, and the presentation would be declared and
never reached.

### `GUESS:` where this diverges from Office

Four, marked at their sites. **Outdent does not promote the following siblings** (Word's does;
reparenting text a person had not selected is an edit noticed three saves later). **A sheet's colour
is an edge and never a fill** — Excel paints the whole inactive tab, and this catalogue cannot,
because the colour belongs to the user's workbook and a label drawn on it would have a contrast
nobody has checked; that is the project's standing rule about deferring to the user's document,
applied to a colour. **Hidden sheets are shown, marked and named** rather than omitted from the
strip. And **a section is a heading row rather than a `role="group"`**, above.

## The Excel chrome (MJXOFF-192)

`src/formula/` holds `<mjx-formula-bar>` and `<mjx-name-box>` — **the most-used control in Excel,
which appears in no ribbon census** because it is not a ribbon control, and the most textually
complex thing in this catalogue. Open `Excel Chrome/Formula Bar → The Argument Tooltip Tracks The
Caret` first, **and move the caret with the arrow keys**; a screenshot of that story proves nothing
at all, which is the ticket's own trap stated as a viewing instruction.

```html
<mjx-formula-bar label="Formula" value="=SUM(A1:A9)">
  <mjx-name-box slot="name-box" label="Name box" address="B7"></mjx-name-box>
</mjx-formula-bar>
```

### Everything hard here is caret-relative, so the gate is a table and not a story

`tests/formula.test.ts` drives **fourteen caret offsets** over one formula with nested calls and two
quoted strings containing commas:

```
=IF(COUNTIF(A1:A9,">5")>0,TEXT(B2,"#,##0"),"none, really")
```

and asserts the active argument at each. The half that matters is the other one:
`naiveActiveArgument` is **shipped beside** the real implementation as a positive control — a
character-by-character comma count — and the suite asserts that the two **disagree** at exactly the
two offsets a quoted comma moves, and **agree** everywhere else. Without that, the table is fourteen
assertions that a correct implementation satisfies and so does any other. `naiveBracketReport` is the
same arrangement for bracket matching.

### What `mjx-sml` owns, and what it deliberately does not

`crates/mjx-sml/src/formula/mod.rs` says it in its own words: *"There is no expression tree, **no
tokeniser** and no dependency graph."* That is settled scope (`PLAN.md`, MJXOFF-21), not a gap — so
this child's tokeniser duplicates nothing. `crates/mjx-sml/src/address.rs` is the opposite case: it
owns a complete address grammar, and `formula-model.ts` **mirrors** it — the same two grid limits
(restated as literals in the suite, so a drift is a failing line), the same four range shapes, the
same anchoring vocabulary, the same *ordering happens in `normalizedBounds` and nowhere else* rule,
and one problem name per `AddressError` variant.

The **one** deliberate difference is the UI's rather than the grammar's: `address.rs` refuses `"a1"`,
because folding case in a *file reader* would accept bytes Excel would not have written. A person
typing into a name box expects `a1` to work, so `normaliseTypedAddress` up-cases before the grammar
sees the text and the grammar stays as strict as the Rust one. Both directions are asserted.

### The reference-colouring contract

`referenceHighlights()` reports every reference in source order with its offsets, its ordered
`GridBounds`, its sheet and its **colour slot** — and the same value rides the
`mjx-formula-references` event. That is the whole point: the in-canvas grid highlight is loop 2's,
and a grid that had to re-tokenise the formula to find the ranges would be a second tokeniser whose
first defect would be disagreeing about a comma inside a string.

⚠ **A slot is two tokens, not one, and four slots rather than Excel's seven.** This palette is warm
and small: a colour that clears 4.5 : 1 on the light surface is usually the one that fails on the
dark one (`color.clay` is 5.23 : 1 and 2.27 : 1). So a slot is an *identity* — first reference,
second — and each scheme spells it with the token that is legible there, declared on `:root` by
`formulaDocumentCss` because that is the only selector that can say so. Four is what the palette can
spell in both schemes at once; a fifth distinct range wraps to the first slot and the contract says
so. Two references naming the same cells share a slot however they were spelled, because anchoring
changes what a *copy* does and not which cells are named.

### Two layers, and the invariant that keeps the caret on its glyph

The editor is a `<textarea>` with **transparent** glyphs over a `<div>` that draws the same text in
colour. The `contenteditable` alternative was rejected because the caret, the selection, undo, redo
and IME composition are then all re-implementations — and above all because a `contenteditable`
caret inside a shadow root needs `ShadowRoot.getSelection()`, **which is not a standard**, and every
caret-relative behaviour in this child would have rested on it.

The cost is an alignment invariant, and it is a list rather than a hope: `alignedTextProperties`
names the nineteen properties both boxes must agree about, both CSS rules are written from that one
list, and `tests/browser/formula.spec.ts` compares them through `getComputedStyle` — with an
anti-vacuity check, because two properties that both resolved to nothing would compare equal.

### `GUESS:` where this diverges from Office

Three, marked at their sites. **Point mode is shown whenever the caret sits where an arrow key
*would* name a range** — Excel enters it when an arrow key or a click actually names one, and this
bar has no grid to point at, so it reports the thing a person needs to know. **The reference ring is
four colours**, above. And **the function catalogue is a starter set of twenty-three with our own
summaries**, exposed as a `functions` property a host replaces wholesale: transcribing five hundred
of somebody else's reference entries into a component library would be a large, stale copy.

**No formula is ever evaluated.** There is no calculation engine in this loop, and
`tests/formula.test.ts` asserts the *shape* of a signature rather than describing it, so a `compute`
or an `evaluate` appearing on one is a failing test.

## Annotation (MJXOFF-193)

`src/annotation/` holds the four components review is made of — `<mjx-comment-card>`,
`<mjx-comment-thread>`, `<mjx-tracked-change-card>` and `<mjx-review-pane>` — and the one component
in this catalogue **whose layout is a function of geometry the canvas owns.** Open
`Annotation/Review Pane → Anchors A Few Lines Apart` first, and click the cards.

```html
<mjx-review-pane label="Comments" side="inlineStart"></mjx-review-pane>
<script>
  document.querySelector('mjx-review-pane').annotations = [
    { id: 'c1', kind: 'comment', model: 'threaded', author: 'Ada Lovelace',
      time: '10:15', text: 'Should this be the 1843 figure?', anchorTop: 300,
      replies: [{ id: 'r1', author: 'Charles Babbage', time: '10:22', text: 'It should.' }] },
  ];
</script>
```

### The margin packing problem, and why a stack is not it

Cards want to sit beside their anchors; several anchors can be within a few lines of each other; and
cards must not overlap. That is **one-dimensional packing with preferred positions**, and the
ticket's trap is that it is invisible on the fixture anybody would write:

> The margin packing problem does not appear with well-spaced anchors — and a simple stacked list
> looks identical and correct there.

Sort by anchor, subtract each card's cumulative stack offset, and *no overlap* becomes *z is
non-decreasing*. Minimising the squared distance from the preferred positions under that constraint
is **isotonic regression**, which `packMarginCards` solves exactly with pool-adjacent-violators in
one pass — no iteration, no tolerance. Two things fall out of the transform rather than being coded:
the column's bounds are a **uniform box in z** even though they are not in y, so the bounded answer
is the unbounded one clamped; and pinning the selected card **splits the problem in two**, so *the
selected card takes its preferred position and the others yield around it* is a property of the
decomposition rather than a special case.

Three gates hold it, and the second is the one that matters:

| Gate | Says |
|---|---|
| `leaves no overlap` | what a stack also satisfies, which is why it is not enough |
| `places every card as near its anchor as packing allows` | the layout **equals an independent optimum** computed by enumerating every block structure and taking the cheapest feasible one — an algorithm with nothing in common with PAVA |
| `a simple stack is worse` | `stackMarginCards` is **shipped beside** the real one and costs more on the tight fixture, and cannot hold the pin |

And the fixture is asserted to be tight (`the tight fixture really is tight`), because a later child
who spreads those anchors out would silently turn the whole suite into a test of nothing.

⚠ **The column's own top wins over the pin, and that is not a rounding case.** Seven cards are about
seven hundred pixels tall and the last anchor is 388 into the document, so holding *it* at its anchor
would need six cards above the column's beginning. The packer moves it and says so; it does not
overlap. There is a test named for that.

### Author colours: searched for, not chosen

Eight hues evenly spaced around the wheel — the answer everyone writes — are **9.15 apart in
CIEDE2000 to a normal-vision reader and 0.00 apart under simulated deuteranopia**: the first two
simulate to the same byte. So `authorColourSlots` was searched over the whole brand palette,
maximising the smallest distance across normal vision and all three dichromacies at once, and the
gate re-runs that measurement rather than trusting the table. The light set clears the floor at 6.17
and the dark set at 7.16; the naive rotation is asserted to **fail** the same gate.

`dev/colour-vision.ts` is the instrument: Viénot, Brettel & Mollon's projection in **linear** sRGB
(projecting gamma-encoded channels is the commonest way to get a plausible wrong answer), and a full
CIEDE2000 checked against **nine rows of Sharma, Wu & Dalal's published test data**.

Three consequences worth knowing before touching any of it:

* **A slot is two tokens**, exactly as MJXOFF-192's reference colours are. `color.ink` is 11.88 : 1
  on the light surface and **1.00 : 1** on the dark one, because the dark surface *is* `color.ink`.
* **An author colour is never a text colour.** The palette contains six colours that reach 4.5 : 1 on
  the light surface and two of them are the same green, so eight author-coloured *names* do not exist
  to be chosen. The colour is a band and a dot — a delineated area — and the name beside it is
  ordinary `text-primary`. `authorColourVisibilityMinimum` is 2 : 1 for that reason, and it is a
  measurement: **no eight-colour subset of this palette reaches 2.5 : 1** on the light surface.
* **Assignment is by first appearance, never by hash.** With eight slots and eight authors a hash
  collides with probability 1 − 8!/8⁸ = 99.76%. `dev/colour-vision.ts` ships `naiveHashSlot` and the
  gate measures the collision rate over it. The cost is stated rather than hidden: a colour is stable
  *for a document*, not across documents, and nothing short of a presence layer can do better.

### A feed, not a list

The column is `role="feed"` and every card — and every reply inside a thread — is `role="article"`
with `aria-posinset` and `aria-setsize`. A `list` would have announced *"list, 9 items"* on a
document with 300 comments, because that is how many are in the DOM; the feed says *"4 of 300"*. The
keyboard is the feed pattern's (Page Up/Down, Control + Home/End) plus arrow-key aliases for readers
arriving from Office.

⚠ **Focus returns to the feed when a card is recycled.** A scroll can take the focused card out of
the window, and removing a focused element sends focus to `<body>` — at which point the feed's key
handling is silently over, because events no longer reach the column. It looks exactly like a
keyboard that stopped working for no reason.

### Both comment models, and the affordances one of them cannot have

A legacy `w:comment` has no `commentsExtended` entry: no parent, no children, no `done` flag. So the
legacy card has **no reply button and no resolve button at all** — not disabled ones, because a
disabled control says *not now* and the truth is *not ever, in this file format*. The difference is
visible (squarer, dashed) **and** audible (a *note*, versus a *conversation*), and the gate asserts
both, because a visual difference nobody can hear is a difference only some readers get.

### The connector contract

Each card reports its anchor (`anchorReport()`, `mjx-annotation-anchor`) and the pane aggregates
(`mjx-annotation-anchors`) for **every** annotation, built or not — the canvas draws a line to a card
that has scrolled out of view as readily as to one on screen. **The line itself is inventory entry 54
and is canvas, not chrome.** `Annotation/Review Pane → The Connector Contract` shows the payload.

### Virtualisation, and the route that was not taken

`reviewWindow` is `windowForOffset` over an `ExtentTable` built from the packed layout, with each
card's extent taken to include the gap beneath it. It deliberately does **not** go through
`<mjx-virtual-list>`'s `VirtualScroller`: that class owns a DOM contract — two spacers and a run of
contiguous rows — which packed cards cannot honour, since a packed card sits at an absolute offset
with a gap of its own. U13 found the right route can be transitive; this is the mirror case, where
the transitive route is the wrong one and the foundation is the right one.

### A card that moves is animated, and cannot overshoot

`annotationMotionClass` is the `documentObject` role and the pane puts it on every card it places;
`.slot` names the property (`inset-block-start`) the role's duration and easing then apply to. A card
that sprang past its new position and came back would be telling a reader their comment moved. The
consequence for the browser tier is stated in its own `settle()`: a box read two frames after a
selection is a box in mid-flight, so the gate waits the transition out rather than switching it off.

### The sheet

Below `reviewSheetAtOrBelow` (an alias of `phoneShellAtOrBelow`, asserted) the pane becomes a sheet
listing annotations in flow. Packing does not merely switch off — it stops meaning anything, because
there is no margin for a card to sit beside — and **every card is built**, since a virtualised flow
layout would be a list whose scroll height was written by an arithmetic that no longer described it.
The gate reads the presentation property *and* three facts it does not control: a card's `position`,
the handle's `display`, and the built-card count.

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
* **The UA's `[hidden] { display: none }` loses to any author rule at all**, because it is in the
  user-agent origin. A component that writes `display: inline-flex` on a class has switched `hidden`
  off for every element wearing it — with the attribute set, the accessibility tree correct and the
  element on screen. Restate `[hidden]` last in the sheet, and assert the position; MJXOFF-189 found
  this with a `toBeHidden()` where an attribute assertion would have passed.
* **`getComputedStyle` on an unregistered custom property returns the substituted text.**
  `calc(150ms * 4)` parses as `150`, and `0.25rem` parses as `0.25`. Register the property
  (`@property … syntax: '<time>'` or `'<length>'`) and check the unit rather than trusting
  `parseFloat`; `resolveDurationMilliseconds` and `resolveLength` both return a refusal rather than a
  plausible number.
* **Sequential focus descends into a shadow tree whatever `tabindex` the host carries.** A container
  that needs a roving tab stop over its children must own those children — a group of slotted
  `<mjx-button>`s has one tab stop per command and no way to fix it from outside.
* **A node directly referenced by `aria-describedby` contributes its text even when it is not
  rendered**, which is what makes an unslotted light-DOM `<span>` the way to describe a *slotted*
  trigger. An IDREF does not cross a shadow boundary in either direction, so a description that lived
  in the component's own shadow root would be visible and completely unannounced.
* **A *disabled* control is exempt from axe's contrast rule; a merely dimmed one is not.** The
  platform dims a disabled control and nobody is expected to read it. Anything else — unavailable,
  hidden, out of scope — is still expected to be read, so an opacity on its label is a real
  contrast failure. MJXOFF-191 measured 2.79 : 1 on a hidden sheet's name.
* **The container's `.frame` is the container query's container; the host is not.** Measuring
  `mjx-resizable-container` itself reports the page width at every preset. Measure `[part="frame"]`.
* **A story that opens something from a `requestAnimationFrame` is a precondition, not a fact.** A
  later child that makes the page a little heavier moves that frame past whatever the test waited,
  and the failure is reported in terms of whatever the test measured afterwards. Assert the state
  the presses require.
* **A bounding box read before a press is a box that may have moved**, and the failure is reported
  as whatever the press landed on doing nothing. Any component that measures its own children and
  corrects its layout — every virtualised one — shifts by a pixel or two after its first render.
  `hover()` first, then read the box.
* **A tab-stop helper whose failure mode is "found nothing" makes every ceiling assertion pass.**
  MJXOFF-186's first version broke its walk on a landing at `<body>` and reported *zero* stops for a
  pane that has two, which read as a passing filter rather than as a broken helper.
* **`<textarea role="combobox">` fails axe's `aria-allowed-role`, and `aria-expanded` on a textbox
  fails `aria-allowed-attr`.** Both were confirmed against axe-core *before* MJXOFF-192's
  autocomplete was written, rather than after the sweep went red. What a textarea *may* carry is
  `aria-controls` and **`aria-activedescendant`**, which is enough for the active option to be
  announced; that the list opened at all then has to be said in words, through a live region.
* **axe reports the contrast of a transparent foreground over a positioned sibling as `incomplete`,
  not as a violation.** So a two-layer editor passes the sweep and its colours are *not measured by
  it*. MJXOFF-192 measures them in the unit tier instead, against both schemes' surfaces — which is
  the stronger check anyway, because the sweep only ever runs in light.
* **A backtick inside a CSS comment ends the template literal it is in.** `surface-model.ts` records
  this having cost it a build; it has now cost two, in the same way, in a paragraph explaining a
  colour choice.
* **A container's own scroll height decides how much of a cluster you can see, so a DOM-level
  overlap assertion on a margin column is an assertion about two cards, not seven.** MJXOFF-193's
  browser gate scrolls the whole column, requires every snapshot to be clean **and** requires the
  sweep to have seen every card. A single snapshot passed while showing two of seven.
* **`getComputedStyle` on `--mjx-density-step` hands back `calc(var(--spacing) * 2)`**, not a length,
  and `resolveLength` correctly refuses it. Registering another module's custom property to fix that
  would be reaching into it; MJXOFF-193 measures a zero-width, hidden probe element whose
  `block-size` is that variable instead, which needs nothing registered and cannot resolve to a
  plausible wrong number.
* **The browser tier runs against `storybook-static/`, so a source fix is invisible until you
  rebuild.** MJXOFF-193 spent a cycle debugging an `aria-label` that was already in the source and
  not yet in the bundle, and the failure reads as a component defect rather than as a stale build.
