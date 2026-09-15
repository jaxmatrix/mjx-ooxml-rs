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

`@fluentui/svg-icons` ships **20,679** files and 12.7 MB; the catalogue ships **82 glyphs and 26 kB
of path data** — 71 until MJXOFF-185, which added the two the gallery's affordance rail needs, and 73
until MJXOFF-194, which added cut, copy, paste and the overflow ellipsis. (The first three are
Office's three most-used commands and this catalogue had no drawing for any of them, which is a
thing worth noticing: they are unreachable on a phone bar without one.) The
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
<mjx-split-button toggle pressed="true" label="Show Comments" icon="comment-multiple"></mjx-split-button>
<mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>
```

**A split button can hold a state.** `toggle` turns its primary region into a pressed toggle, for
Office's *state with a menu* (Track Changes, Show Comments, Hide Ink, Eraser). The primary carries
`aria-pressed` and `data-pressed` and is painted by the toggle button's rows of the state table, with the
`filled` icon when the subset has one; activating it writes `pressed` and then emits `mjx-change` with
`detail.pressed`, as `<mjx-toggle-button>` does, and no `mjx-activate`. The arrow stays a menu button,
and Arrow Down never moves the state. Without `toggle`, `pressed` is ignored and nothing is announced.
`Controls/Split Button → The Toggle Mode` shows it in isolation.

### One of a set

Some toggles hold one at a time in Office: Word's five views, its two page movements, the Draw tab's ink
tools, and Background Removal's two marking pencils. `exclusive="<set>"` makes a toggle, or a split button's
toggle face, one of such a set:

```html
<mjx-toggle-button exclusive="word.view.document-views" label="Print Layout" pressed></mjx-toggle-button>
<mjx-toggle-button exclusive="word.view.document-views" label="Web Layout"></mjx-toggle-button>
<mjx-split-button toggle exclusive="word.draw.write.tools" label="Eraser"></mjx-split-button>
<mjx-toggle-button exclusive="word.background-removal.refine" exclusive-allows-none label="Mark Areas to Keep"></mjx-toggle-button>
```

- **Pressing a member releases every other member that holds**, looked up in its nearest `<mjx-ribbon-tab>`,
  else its `<mjx-ribbon>`, else its root node. So three ribbons on one docs page stay independent, and a
  collapsed group's survivor and its popup are one set.
- **Pressing the member that holds keeps it**, and reports nothing, so one member always holds.
- **Unless the set may hold none**: with `exclusive-allows-none` on every member, pressing the member that
  holds releases it, and the set may start empty. Background Removal's pencils are the one such set, because
  its *no tool* is the ordinary pointer, which is not a command on the tab; the Draw tab's is Select Objects.
  `GUESS:` that Office releases a pencil on a second press.
- **Attributes move first, then each member that moved emits `mjx-change`**: the released ones with
  `pressed: 'false'`, then the pressed one.
- **They stay toggle buttons**, with `aria-pressed`, not radios. A radio group is one tab stop, and a
  ribbon's set spans a split button and a collapsed group's two slots.

The census declares the set (`RibbonCommand.exclusive`), `renderCommand` writes it onto the generic toggle,
and a host binding writes it by hand. `tests/ribbons.test.ts` holds each set to exactly one member pressed
(at most one where it may hold none, declared by every member alike) and each binding to the census. `src/controls/exclusive-set.ts` records the alternatives rejected.

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
      <mjx-button label="Clear All Formatting" icon="clear-formatting" size="icon"></mjx-button>
      <mjx-toggle-button slot="essential" label="Bold" icon="text-bold" size="icon"></mjx-toggle-button>
      <mjx-toggle-button label="Strikethrough" icon="text-strikethrough" size="icon"></mjx-toggle-button>
      <mjx-dialog-launcher slot="dialog-launcher" label="Font settings"></mjx-dialog-launcher>
    </mjx-ribbon-group>
  </mjx-ribbon-tab>
  <mjx-contextual-tab-set label="Table Tools">
    <mjx-ribbon-tab tab-id="table-design" label="Design">…</mjx-ribbon-tab>
  </mjx-contextual-tab-set>
</mjx-ribbon>
```

### Two slots, one order — which is why nothing can be lost and nothing moves

The obvious way to build a collapsing group is to render its commands twice, once in the strip and
once in a menu. That is also the way to lose one. So a group renders its commands **once**, and the
three presentations change what its `.panel` *is*: part of the strip in `full` and `reduced`, an
overlay anchored to a single button in `collapsed`. *Reachable in all three* is therefore structural
rather than remembered — the same DOM nodes at every width.

Children are written **in the order Office draws them**, and `slot="essential"` marks a survivor
where it stands. The group's shadow root uses *manual* slot assignment: in `full` and `reduced` every
command is in the panel's one ordered slot and the survivor row is empty; in `collapsed` the
survivors are moved into the row beside the trigger and the rest stay in the popup. A command is
moved between slots, never rebuilt.

### Where a survivor draws (unit 2b of the ribbon programme)

For three units the survivor row was built `trigger, essential, panel`, so an essential command drew
**first** at every width — Bold, Italic and Underline ahead of the font name and size, the reverse
of Word. `d01cf93` made it `trigger, panel, essential` and was reverted, because *last* is as wrong:
Word's Font draws its survivors in the middle. **Where a survivor sits is a property of the group**,
so the declared order is the answer and the component keeps it. `survivorPlacement` in
`src/ribbon/ribbon-model.ts` records the decision and the four designs refused with it — CSS `order`
over one grid (focus follows the flat tree, not `order`, and `reading-flow` is Chromium's alone),
splitting only while the popup is open, a second copy beside the trigger, and observing the ribbon
rather than a probe.

The slots have to follow CSS's decision, and CSS cannot say when it changed. So one shared
`ResizeObserver` watches a zero-height probe inside each group whose width is `100cqi` — the query
container's own width and nothing else. It fires exactly when a container condition can have
changed, and when a group in a hidden tab becomes rendered; its callback ignores the sizes, reads
every group's presentation, then re-slots them all. The probe rather than the group is observed
because re-slotting resizes the group inside the callback, which is a ResizeObserver loop error.

**Essential is declared, never inferred.** `toggle()` used to put every toggle in the survivor slot,
which capped a group at three state commands and left Justify, Subscript and Excel's vertical
alignments unable to draw pressed. A state is a toggle; a survivor is `essential: true` in
`dev/ribbons/census.ts`, and `toggle()` requires every caller to say which.

Five gates hold it: `tests/ribbons.test.ts` runs `placeGroupCommands` over every authored group and
watches a survivors-first and a survivors-last rule fail on the census's own data;
`tests/browser/ribbon.spec.ts` reads the real slots and geometry expanded and collapsed, and walks
Tab through Word's Paragraph group in both; the same file **selects the File tab by clicking the
picker at 390px** and requires every group to arrive collapsed with exactly its declared survivors —
the path every later tab's audit takes — and **widens an open collapsed Font group to 1440px** and
requires every command back in order and the popup, and its focus trap, gone; and it asserts its
fixture is **discriminating** — Font and Paragraph declare a command on each side of their survivors.

Besides the MutationObserver and the probe, a change to `priority` or `simplified` re-slots too. And
a group whose presentation stops being a popup **closes itself without moving focus**: left `open`
at a desktop width it would keep its dismissal listener and trap Tab inside a group that has no
popup.

| Presentation | What changes | Reached when |
|---|---|---|
| `full` | large commands stay large; the group's name sits under them | above the priority's reduce width |
| `reduced` | compact density; a large command lays out sideways and clamps to one line; two rows | at or below it |
| `collapsed` | one button carrying the name, the essential commands beside it, the rest in a popup | at or below the priority's collapse width |

### A bound command opens a menu that exists (unit 3 of the ribbon programme)

The Insert tab is the first where most of a group's face opens something: fifty-one of its ninety-one
commands across the three applications are a dropdown or a split button in Office. `RibbonCommand`
stays a button or a toggle, so each is declared in `dev/ribbons/census.ts` with the label, icon and
size Office draws it with — keeping the glyph inside `tests/ribbons.test.ts`'s icon gate — and both
hosts bind a real control over it: `<mjx-split-button>` where Office draws two hit regions, and
`<mjx-button>` where it draws one, whose press opens the menu through `openDeclaredSurface`.

The menus are written **once**, in `stories/ribbons/insert-menus.ts`, and a host renders
`insertMenus(application, host)` beside its ribbon. The binding and the menu therefore live in
different files, and a `data-opens` that names nothing is a button that looks right and opens
nothing. So the id is **derived** — `commandSurfaceId(host, commandId)` — and
`tests/ribbons.test.ts` reads both sides: every `data-opens` in the six hosts must resolve, a binding
may only open its own command's menu, every declared menu must be opened by both of its application's
hosts, and a host that binds menus must render them. Each refusal is watched firing on a hand-made
source.

**Excel's eight chart-family lists are exported from `stories/ribbons/insert-menus.ts`**
(`columnBarChartEntries()` to `scatterBubbleChartEntries()`), unchanged, because Chart Design's Change Chart Type opens
the same eight families. See *Word's Chart Design*.

**Shapes is the one Insert menu that is no longer a sample.** PowerPoint's Shape Format opens the same gallery twice
(Shapes, and Edit Shape ▸ Change Shape), so the whole gallery is written once in
`stories/ribbons/drawing-tools-menus.ts` and all three Insert Shapes menus call `insertShapesEntries(application)`:
every section and shape, PowerPoint's Action Buttons and Word's New Drawing Canvas. See *PowerPoint's Shape Format*;
Word's and Excel's Shape Format open their own application's copy of the same gallery.

⚠ **A one-region dropdown does not announce its menu.** `<mjx-button>` observes no `aria-haspopup`
or `aria-expanded`, so a screen reader hears *Table, button* where Office says *Table, menu button,
collapsed*. The menu itself is a real `<mjx-menu>` — roving focus, Escape restores focus to the
button — but the announcement before the press is a component gap, not something a binding can fix.

### The Draw tab is two generations of Office, and it has one survivor (unit 4 of the ribbon programme)

The census's `TabDrawInk` declares Office 2013's *Ink Tools* groups (Write, Pens, Close) next to
Microsoft 365's Draw groups (Drawing Tools, Stencils, Drawing Canvas, Replay, Help, Draw with Touch).
No Office build draws both sets together, and the census wins, so the tab draws both. Each command is
drawn once, so Drawing Tools holds only Add Pen. `dev/ribbons/census.ts` records the reading as
`GUESS:`, along with Word's `GroupEditingExcel` and the unnamed Input Mode group. The menus are
written once, in `stories/ribbons/draw-menus.ts`, under the same gate as Insert's.

**Select Objects is the tab's only survivor.** Pen, Highlighter, Lasso Select and Eraser arm a
gesture, which is the reason unit 3 refused Text Box. Ruler and Draw with Touch pass the rules, but
each is its group's only command.

**The five tools are one exclusive set**, as in Office, which holds one tool at a time. Pressing Pen
releases Select Objects, and pressing the tool that holds keeps it. Word's and PowerPoint's Eraser is a
split button whose face is one of the five. See *One of a set* under the ribbon archetypes.

### Design and Layout: galleries, dropdowns and fields (unit 5 of the ribbon programme)

Four tabs describe the whole document: Word's Design and Layout, PowerPoint's Design and Excel's Page
Layout. Almost every command opens something, and each host binds one of three shapes over it:

- **An in-ribbon `<mjx-gallery>`** where Office draws a strip in the group: Word's Style Set, and
  PowerPoint's Themes and Variants.
- **A dropdown** over a menu written once in `stories/ribbons/design-layout-menus.ts`: Margins,
  Orientation, Colours, Watermark and the rest. Bring Forward and Send Backward are split buttons.
- **A field**: Word's Indent and Spacing are `<mjx-measure-input>`, Excel's Width and Height are
  `<mjx-dropdown>`, Excel's Scale is an `<mjx-combo-box>`, Word's Page Colour is `<mjx-color-picker>`,
  and Excel's Sheet Options are `<mjx-checkbox>`.

`dev/ribbons/census.ts` records two design decisions. **Arrange is declared once for Word and Excel**,
by `arrangeCommands`, which PowerPoint's Table Layout (for a table: no Group or Rotate), Word's, PowerPoint's and Excel's Picture Format and every Shape Format now call too. **Themes, Colours, Fonts and Effects open the same menus** in Word, in Excel and
at the foot of PowerPoint's Variants gallery. No command on the four tabs survives a collapse.

⚠ **Two shapes are `GUESS:`.**

- **Word's and Excel's Themes are dropdowns.** Office draws a button whose popup is a gallery, and
  `<mjx-gallery>` has no button presentation.
- **Variants' Colours, Fonts, Effects and Background Styles are gallery footer buttons.** Each opens a
  menu from inside the open flyout, and nobody has watched that happen. Those four menus have literal
  ids in the PowerPoint hosts, because a footer button is not a census command. The gate accepts a
  literal id, but it does not require both hosts to open it.

### References, Transitions and Formulas (unit 6 of the ribbon programme)

Three tabs: Word's References, PowerPoint's Transitions and Excel's Formulas. The shapes are unit 5's.

- **An in-ribbon `<mjx-gallery>`**: Transitions' *Transition to This Slide*, starting on Fade.
- **A dropdown or split button** over a menu written once in
  `stories/ribbons/references-transitions-formulas-menus.ts`: Table of Contents, Add Text, Insert
  Citation, Bibliography, Effect Options, the eight function categories, Use in Formula and Calculation
  Options are dropdowns. Next Footnote, AutoSum, Define Name, Remove Arrows and Error Checking are split
  buttons.
- **A field**: Word's citation Style and PowerPoint's Sound are `<mjx-dropdown>`, Duration and Advance
  Slide After are `<mjx-combo-box>`, and On Mouse Click and After are `<mjx-checkbox>`. Their lists are
  in `ribbon-parts.ts`.

Fluent ships Excel's own function library books (`book-star`, `book-coins` and the rest), so the
Function Library draws Office's pictures. No command on the three tabs survives a collapse, and
`dev/ribbons/census.ts` gives each group's reason.

⚠ **Three readings are `GUESS:`.**

- **Transitions' groups.** The census labels `GroupTransitionStyles` *Transition Styles* and
  `GroupTransitionToThisSlide` *Timing*, although the second id names Office's gallery group. The labels
  win: Transition Styles holds the gallery and Effect Options, and Timing draws six commands where the
  census counts two.
- **Duration is a combo box.** `<mjx-measure-input>` carries lengths, and a duration is seconds.
- **Insert Footnote is a plain button**, as Office draws it, where the unit's brief expected a dropdown.

### Mailings, Animations and Data (unit 7 of the ribbon programme)

Three tabs: Word's Mailings, PowerPoint's Animations and Excel's Data. The shapes are unit 6's.

- **An in-ribbon `<mjx-gallery>`**: Animations' *Animation Styles*, starting on Fly In with every effect
  Office's gallery shows and its *More … Effects* footer, and Data's *Data Types*, whose pictures are
  `<mjx-icon>` — the first gallery art in the catalogue that is an icon rather than inline shapes.
- **A dropdown or split button** over a menu written once in
  `stories/ribbons/mailings-animations-data-menus.ts`: Start Mail Merge, Select Recipients, Rules, Finish &
  Merge, Effect Options, Add Animation, Trigger, From Other Sources and What-If Analysis are dropdowns.
  Insert Merge Field, Preview, Refresh All, Data Validation, Group and Ungroup are split buttons.
- **A field**: Mailings' Go to Record and Animations' Duration and Delay are `<mjx-combo-box>`, and Start
  is `<mjx-dropdown>`. Their lists are in `ribbon-parts.ts`; Duration shares `durationSeconds` with
  Transitions.

**Survivors**: Previous Record and Next Record on Mailings, Sort A to Z and Sort Z to A on Data, none on
Animations. Filter does not survive, because its funnel is already Insert's Slicer. `dev/ribbons/census.ts`
gives each group's reason.

⚠ **Four readings are `GUESS:`.**

- **Data's generations.** The census declares Office 2016's Get External Data and Connections beside
  Microsoft 365's Queries & Connections and its Workbook Links variant, and marks Power Query's Get &
  Transform Data out of scope. So there is no Get Data, the legacy Get External Data face is drawn, and
  Refresh All, Properties and the rest are each drawn once across the three connection groups.
- **Insert Merge Field and Preview are split buttons**, as Office draws them, where the brief expected
  dropdowns.
- **Preview Results starts pressed**, so the record navigator has something to audit.
- **Animations' Effect Options is Fly In's** and does not follow the gallery.

**Transitions was completed in this unit.** The gallery holds every transition Office shows (None and
forty-nine more), Sound holds Office's whole list, Timing carries the *Advance Slide* caption as
`<mjx-label>`, and **Effect Options follows the gallery**: `followTransitionEffectOptions(host)` listens
for `mjx-gallery-commit` and re-renders one `<mjx-menu-section>` that is its own lit render root, seeded
through `ref`, and marks the button `disabled` for a transition Office gives no options. ⚠ Office stacks
Timing in two columns; `<mjx-ribbon-group>` draws every command in one row at full width, so the order
is Office's and the columns are not.

### Word's Review (unit 8 of the ribbon programme)

**One tab of one application**: the unit was narrowed to Word's Review. PowerPoint's and Excel's followed in
the next two sections. Nine groups and twenty-three commands, in Office's order: Proofing,
Accessibility, Language, Comments, Tracking, Changes, Compare, Protect, Ink.

- **A dropdown or split button** over a menu written once in `stories/ribbons/review-menus.ts`: Translate,
  Language, Show Markup and Compare are dropdowns; Check Accessibility, Delete, Show Comments, Track
  Changes, Reviewing Pane, Accept, Reject, Block Authors and Hide Ink are split buttons.
- **A field**: Display for Review is `<mjx-dropdown>` over `ribbon-parts.ts`'s `displayForReviewModes`.
- **A toggle**: Restrict Editing, pressed while its pane is open.
- **A split button whose face is a toggle** (`<mjx-split-button toggle>`): Track Changes (starts
  unpressed), Show Comments (starts pressed) and Hide Ink. The face draws pressed; the arrow opens the menu.

**Every menu carries Office's whole list**, not a sample, since the user rejected a sampled Transitions
gallery. Submenus (Show Markup's *Balloons* and *Specific People*, Compare's *Show Source Documents*) are
flattened into labelled sections, as before.

**Survivors**: Previous Comment and Next Comment. Previous Change and Next Change pass rule 1 and have no
honest glyph, because Fluent's page-with-an-arrow means upload and download.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Show Comments starts pressed** and Track Changes and Hide Ink start unpressed: a new document's
  defaults, not a build this project can cite.
- **Ink is drawn last**, where Microsoft 365 draws it; the census declares it fifth.
- **Office's face says *Previous* and *Next* twice.** The labels here are Office's tooltips (Previous
  Comment, Next Change and the rest), because a label is also the accessible name.
- **Show Comments' Contextual/List arrow, Hide Ink's shape, Block Authors' arrow and Check Accessibility's
  entries** are from memory of Microsoft 365 and Word 2010, not from a build this project can cite.

### PowerPoint's Review

**One tab of one application again**, authored after Word's and to its pattern. Seven groups and nineteen
commands, drawn in Office's order: Proofing, Accessibility, Language, Comments, Compare, Activity, Ink.
Nothing is declared once for both applications, because every group's face differs.

- **A dropdown or split button** over `stories/ribbons/review-menus.ts`: Language is a dropdown; Check
  Accessibility, Delete, Accept and Reject are split buttons. Language reuses Word's entry list, the only
  list the two tabs genuinely share.
- **A split button whose face is a toggle**: Show Comments and Hide Ink, both starting unpressed.
- **A toggle**: Reviewing Pane.
- **No field, no gallery, no dialog launcher.**

**Survivors**: Previous Comment and Next Comment, small where Office draws them large, because every
survivor in this catalogue is small.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Activity is drawn as Show Changes alone.** The census counts two controls and names none.
- **Ink is drawn last, after Activity**; the census declares it fifth, before Compare.
- **Office greys Accept, Reject, the change navigators, Reviewing Pane and End Review** until a comparison
  is under way. They are drawn available so their menus can be audited.
- **Translate is a plain button**, and Reviewing Pane's `panel-right` glyph, Show Comments' and Hide Ink's
  arrows and starting positions, Delete's wording and Check Accessibility's entries are from memory of
  Microsoft 365 and PowerPoint 2013, not from a build this project can cite.
- **Compare, Previous Change, Next Change, End Review, Show Changes and Hide Ink carry no icon.**

### Excel's Review

**The third Review tab, one application again**, to Word's pattern. Eleven groups and twenty-three commands,
drawn in Office's order where Office has one: Proofing, Performance, Accessibility, Language, Threaded
Comments, Comments, Notes, Protect, Changes, Ink, Debug. Nothing is declared once with Word or PowerPoint;
Translate is the only face that matches, and one shared button is not worth a function of the application.

- **A dropdown or split button** over `stories/ribbons/review-menus.ts`: Notes and Track Changes are
  dropdowns; Check Accessibility is a split button.
- **A split button whose face is a toggle**: Hide Ink, starting unpressed.
- **A toggle**: Show Comments, Show/Hide Comment, Show All Comments and Protect Workbook.
- **No field, no gallery, no dialog launcher.**

**Survivors**: Previous Comment and Next Comment, in Threaded Comments.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Comments is three generations of Office**, as Data's Connections is. Threaded Comments is Microsoft 365's
  face; Comments is Office 2016's, drawn as Show/Hide Comment and Show All Comments; Notes is Microsoft 365's
  dropdown. Those two toggles and Notes' Show/Hide Note and Show All Notes entries are the same two commands
  under two generations' names.
- **Changes is Office 2016's legacy group**: Share Workbook, Protect and Share Workbook and Track Changes,
  which Microsoft 365 hides behind ribbon customisation. Protect Sheet, Protect Workbook and Allow Edit Ranges
  are drawn once, in Protect. Microsoft 365's Show Changes has no census row here.
- **Debug is one button carrying the group's label**, as Home's Power Options is. The census counts one
  control and names none.
- **Performance is drawn second and Ink tenth**; the census declares them tenth and seventh. Insights and
  Lineage are out of scope.
- **Office greys Unshare Workbook and, on a protected sheet, Allow Edit Ranges.** Both are drawn available.
- **Check Performance's speedometer, Protect Sheet's grid with a padlock, Protect Workbook as a toggle, and
  the Check Accessibility, Notes and Hide Ink entry lists** are from memory of Microsoft 365, not from a
  build this project can cite.
- **Show/Hide Comment, Show All Comments, Allow Edit Ranges, Unshare Workbook, Share Workbook, Protect and
  Share Workbook, Hide Ink and Debug carry no icon.**

### Word's View

**One tab of one application**, after the three Review tabs; PowerPoint's View and Excel's View followed. Seven groups and twenty-five commands, drawn in Office's order: Document Views, Modes, Page
Movement, Show, Zoom, Window, Night Mode. The tab changes how a document is looked at and never the
document, so almost nothing on it opens anything.

- **Toggles**: the five views (Print Layout pressed), Focus, Vertical (pressed) and Side to Side, View Side by
  Side, Synchronous Scrolling and Switch Modes. **The five views are one exclusive set, and Vertical and Side
  to Side are another**, so each holds exactly one member, as in Office.
- **Checkboxes a host binds**: Ruler, Gridlines and Navigation Pane (ticked, because the shell draws the pane).
- **One dropdown**: Switch Windows, over `stories/ribbons/view-menus.ts`, which lists every open window.
- **No field, no gallery, no dialog launcher.**

**Survivors**: 100%, One Page and Page Width, in Zoom. Each sets the zoom in one press and changes no
document, which is the Mailings record-navigator standard.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Page Movement is drawn third**, where Microsoft 365 draws it; the census declares it sixth. Night Mode is
  last because nothing says where Office puts it. Macros and SharePoint are out of scope.
- **Three group labels are the census's, not Microsoft 365's**: Document Views is Office's *Views*, Modes is
  *Immersive*, and Night Mode is *Dark Mode*, whose Switch Modes Office shows only under the Black theme.
- **Split is a plain button**, because Office relabels it Remove Split. Office greys Synchronous Scrolling
  and Reset Window Position until View Side by Side is on; both are drawn available.
- **Switch Windows lists one window, *Method notes***, the catalogue's own document, not a real file name.
- **Read Mode's book, Print Layout's page, Web Layout's globe, Outline's stepped bars, Draft's lines with a
  pencil, Focus's corners, the three zoom glyphs, Split, View Side by Side, Switch Windows and Switch Modes**
  are judged from Fluent's drawings, not from a build this project can cite. Outline does not use the nearer
  `text-bullet-list-tree`, because that is Multilevel List's.
- **Vertical, Side to Side, Ruler, Gridlines, Navigation Pane, Zoom, Multiple Pages, Arrange All, Synchronous
  Scrolling and Reset Window Position carry no icon.**
- `GUESS:` **pressing the member of a set that holds does nothing.** Office's Pen, pressed again, opens its
  options.

### PowerPoint's View

**The second View tab, one application again**, to Word's pattern. Seven groups and twenty-four commands, in
the census's order, which is Office's for the six groups Office draws: Presentation Views, Master Views,
Show, Zoom, Colour/Greyscale, Window, View Direction. Nothing is declared once with Word's: Zoom and Window
share census ids and not faces. The glyphs of the four commands the two tabs share are shared, and so is
the Switch Windows list's shape.

- **Three exclusive sets**: the five presentation views (Normal pressed); Colour (pressed), Greyscale and
  Black and White; Left-to-Right (pressed) and Right-to-Left.
- **A plain toggle**: Notes, unpressed, because the shell's status bar reads *Notes: Hidden*.
- **Checkboxes a host binds**: Ruler (ticked, as the shell's slide context menu shows it), Gridlines, Guides.
- **One dropdown**: Switch Windows, over `stories/ribbons/view-menus.ts`.
- **Buttons**: the three masters, Zoom, Fit to Window, New Window, Arrange All, Cascade and Move Split.
- **One dialog launcher**, *Grid Settings*, on Show. **No field, no gallery, no split button**: nothing here is
  a state with a menu behind it.

**Survivor**: Fit to Window, in Zoom. One press, no deck changed, a glyph no other command has.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **View Direction is the census's alone**: three controls, none described. It is drawn as Left-to-Right and
  Right-to-Left, last, as the group Office adds for right-to-left editing. All of that is `GUESS:`. Macros is
  out of scope.
- **The census's spelling is drawn**: Colour/Greyscale, Colour and Greyscale, where Office writes Color and
  Grayscale.
- **Greyscale, Black and White and the three masters open view tabs in Office**; here the colour modes are one
  set that Colour releases, and the masters are plain buttons. The tabs they open are their own stories; see
  *PowerPoint's Black and White* and *PowerPoint's Greyscale*.
- **Outline View, Notes Master and Notes are small where Office draws them large**, because Fluent draws their
  glyphs at 20 alone.
- **Office greys Arrange All, Cascade and Move Split** in some states; all are drawn available.
- **Every command carries a glyph but the three checkboxes**, and every glyph is judged from Fluent's
  drawings. Zoom's `zoom-in` disagrees with Word's Zoom, which carries none.

### Excel's View

**The third View tab, one application again**, to Word's and PowerPoint's pattern. Seven groups and twenty-eight
commands, drawn in Office's order where Office has one: Sheet View, Workbook Views, Show, Zoom, Window, Night
Mode, Debug. Nothing is declared once with Word's or PowerPoint's: Zoom, Window, Show and Night Mode share
census ids and not faces. Seven glyphs are shared, and so is the Switch Windows list's shape.

- **One exclusive set**: Normal (pressed), Page Break Preview and Page Layout. Custom Views is a button.
- **Toggles**: Split, View Side by Side, Synchronous Scrolling and Switch Modes, all unpressed.
- **Checkboxes a host binds**: Ruler, Gridlines, Formula Bar and Headings, all ticked.
- **One field a host binds**: the Sheet View dropdown, reading *Default*.
- **Two dropdowns** over `stories/ribbons/view-menus.ts`: Freeze Panes (Freeze Panes, Freeze Top Row, Freeze
  First Column, each with a glyph and a description) and Switch Windows (*1 Findings*).
- **Buttons**: Keep, Exit, New, Options, Custom Views, Zoom, 100%, Zoom to Selection, New Window, Arrange All,
  Hide, Unhide, Reset Window Position and Debug.
- **No gallery, no split button, no dialog launcher.**

**Survivors**: 100% and Zoom to Selection, in Zoom. One press each, no workbook changed. Split and Synchronous
Scrolling pass rule 1 and fail rule 2 on their glyphs.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Sheet View is drawn first**, where Microsoft 365 draws it; the census declares it fifth. Night Mode and
  Debug are last because nothing says where Office puts them. Macros is out of scope.
- **Night Mode is Word's reading**, one Switch Modes toggle, and **Debug is one labelled button**, as on
  Excel's Review. The census names both groups and describes neither.
- **Office greys** most of Sheet View outside OneDrive and SharePoint, Ruler outside Page Layout view, Unhide
  with nothing hidden, and Synchronous Scrolling and Reset Window Position until View Side by Side is on. All
  are drawn available.
- **Gridlines and Headings repeat Page Layout's View Gridlines and View Headings**, the same two options on two
  faces, because the census declares both groups.
- **Freeze Panes' first entry is *Freeze Panes***, because nothing is frozen; Office relabels it *Unfreeze
  Panes* while panes are. The three descriptions are from memory of Microsoft 365.
- **Custom Views is small where Office draws it large**, because Fluent draws `window-bullet-list` at 20 alone.
  Page Break Preview is large with a three-word label, which may not wrap cleanly.
- **Every glyph is judged from Fluent's drawings.** Synchronous Scrolling's up-and-down arrow and Reset Window
  Position's two columns disagree with Word's, which carry none.
- **The Sheet View dropdown, the four Show checkboxes and Debug carry no icon**: a field draws its value, a
  checkbox its tick box, and Debug has no described behaviour for a glyph to show.

### PowerPoint's Slide Show

**One tab of one application, and the only application with the tab**, after the three View tabs. Four groups
and fourteen commands, drawn in Office's order: Start Slide Show, Rehearse, Set Up, Monitors. The tab plays the
deck and says how it is played.

- **Buttons**: From Beginning, From Current Slide, Rehearse with Coach, Set Up Slide Show and Rehearse Timings.
- **A toggle**: Hide Slide, unpressed.
- **Two dropdowns and a split button** over `stories/ribbons/slide-show-menus.ts`: Present Online and Custom
  Slide Show; Record, whose arrow holds From Current Slide…, From Beginning… and a *Clear* section of four.
- **One field a host binds**: Monitor, reading *Automatic*, over `slideShowMonitors`.
- **Checkboxes a host binds**: Play Narrations, Use Timings, Show Media Controls and Use Presenter View, all
  ticked.
- **No exclusive set, no split toggle, no gallery, no dialog launcher.**

**Survivor**: Hide Slide, in Set Up. One press, one undo, a dashed slide no other command draws. From Beginning
and From Current Slide take over the screen, which no undo takes back.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Rehearse is drawn second**, where Microsoft 365 draws Rehearse with Coach; the census declares it after Set
  Up, counts one control and names none, so the command is `GUESS:` too.
- **Captions & Subtitles is out of scope**: the census's `GroupLiveSubtitles` row is `in_scope = 0`.
- **The three menus are from memory** of PowerPoint 2016 and Microsoft 365: Present Online's Office Presentation
  Service and Skype for Business, Custom Slide Show's Custom Shows… alone (the deck has saved no custom show),
  and Record's wording. Monitor's *Primary Monitor* stands for a display name, which is the machine's data.
- **Office greys Rehearse with Coach offline and Show Media Controls in a deck with no media.** Both are drawn
  available.
- **Every glyph is judged from Fluent's drawings.** Present Online shares File's `presenter`.
- **Monitor and the four checkboxes carry no icon**: a field draws its value, a checkbox its tick box.

### PowerPoint's Recording

**One tab of one application, and the only application with the tab**, after Slide Show. Ten groups and fifteen
commands, drawn in the census's declared order: Record, Recording, Content, Camera, Auto-play Media, Edit, Save,
Export, Preview, Help. The tab records a deck with its narration and camera, then saves or exports it.

⚠ **The census declares two generations of the tab.** Five group ids end in `TabRecord` (Recording, Edit,
Export, Preview, Help) and read as the newer recorder's Record tab. The other five (Record, Content, Camera,
Auto-play Media, Save) read as the Recording tab Microsoft 365 has shipped since 2017. Office never draws both,
and the tab draws all ten, which is Draw's answer. `GUESS:` the reading.

- **Buttons**: From Beginning, From Current Slide, Screen Recording, Save as Show, Export to Video, Preview, Help.
- **Two split buttons** over `stories/ribbons/recording-menus.ts`: Record (From Current Slide…, From Beginning…)
  and Cameo.
- **Six dropdowns**: Screenshot, Video, Audio, Clear Recording, Reset to Cameo, Export.
- **Insert's code, reused rather than rewritten**: Camera is `cameraCommands('recording')`, and the Screenshot,
  Cameo, Video and Audio menus call `insert-menus.ts`'s `screenshotEntries`, `cameoEntries`,
  `powerpointVideoEntries` and `powerpointAudioEntries`, which Insert now calls too.
- **No toggle, no exclusive set, no split toggle, no gallery, no field, no checkbox, no dialog launcher.**

**Survivors: none.** Every command takes over the screen, opens a menu, a dialog or a page, or is its group's only
command.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Record is drawn twice in effect**: the Record group's two buttons and the Recording group's Record split. Each
  is one generation's shape of one verb, and dropping either would empty a declared group.
- **The counts are larger than the faces, and nothing is padded.** Record counts 8, which is exactly PowerPoint
  2016's Record Slide Show split and its Clear submenu, and draws two. Auto-play Media counts 5 and draws two.
  Edit's 6 is exactly two menus of a face and two entries.
- **Every command in Recording, Edit, Export, Preview and Help is `GUESS:`**, from Microsoft's support wording
  for the record window: Clear Recording and Reset to Cameo, each *on Current Slide* and *on All Slides*;
  Export's Export Video and Customize Export.
- **Content drops Apps and Quizzes**, Office Mix's door, which Microsoft retired in 2018.
- **Two labels are the census's**: *Recording* (Office: Record) and *Auto-play Media* (Office: Auto-Play Media).
- **Every glyph is judged from Fluent's drawings.** From Beginning and From Current Slide share Slide Show's
  glyphs; Record shares Slide Show's Record. `delete`, `arrow-reset` and `arrow-export` gain 24s;
  `save-arrow-right`, `video-clip` and `play-circle` are new. Every command carries an icon.

### Word's Outlining

**One tab of one application, and the first view tab authored**, after PowerPoint's Recording. Three groups and
twenty-one commands, drawn in Office's order, which is also the census's: Outlining Tools, Master Document,
Close. The tab restructures a document by its headings: levels, order, and the subdocuments a master document
is saved as.

⚠ **A view tab renders in the catalogue alone.** Office shows Outlining only inside Outline view, and `tabsFor`
leaves every `appearance: 'view'` tab out of a strip unless `includeViewTabs` is asked for. Only `Ribbons/Word`
asks, so its four bindings are written in `stories/ribbons/word.stories.ts` and nowhere else; `Shell/Word` never
draws the tab.

- **Two fields a host binds**: Outline Level (*Body Text*) and Show Level (*All Levels*), each a dropdown over
  `stories/ribbons/outlining-menus.ts`, which lists Level 1 to Level 9 and then Body Text or All Levels.
- **Two checkboxes a host binds**: Show Text Formatting (ticked) and Show First Line Only.
- **Toggles**: Show Document (pressed), Collapse Subdocuments and Lock Document.
- **Buttons**: Promote to Heading 1, Promote, Demote, Demote to Body Text, Move Up, Move Down, Expand and
  Collapse (all icon-only), Create, Insert, Unlink, Merge, Split and Close Outline View.
- **No menu, no gallery, no split button, no exclusive set, no dialog launcher.** `outlining-menus.ts` holds
  the two fields' lists and declares no menu, because Office's tab opens none.

**Survivors**: Promote and Demote, in Outlining Tools. One press moves a paragraph one level, and one undo takes
it back. Promote to Heading 1, Demote to Body Text, Move Down and Lock Document fail rule 2 on their glyphs, and
Move Up is not kept alone without its pair. Expand and Collapse pass, and are not kept because four survivors
would exceed the ceiling of three.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **The first group's label is the census's, *Outlining Tools***, where Office writes *Outline Tools*.
- **The Outline Level field sits between Promote and Demote**, where Office draws it; the brief listed it first.
  Its list puts Body Text last, as the ribbon's box is remembered, where Word's Paragraph dialog puts it first.
- **Master Document is drawn whole, with Show Document pressed.** Office shows Create, Insert, Unlink, Merge,
  Split and Lock Document only while Show Document is pressed. Office greys five of the eight in a document
  with no subdocuments; all are drawn available.
- **Collapse Subdocuments is a toggle, as the brief lists it**, and Office may relabel it *Expand Subdocuments*
  instead of drawing it pressed, which is why Word View's Split is a plain button. It is small where Office
  draws it large, because *Subdocuments* does not wrap. *Close Outline View* is large and may not wrap cleanly.
- **Every glyph is judged from Fluent's drawings.** Move Up and Move Down share Animations' Move Earlier and
  Move Later; Expand and Collapse share Excel's Show Detail and Hide Detail. The four level arrows, the Master
  Document glyphs and Close's cross in a square are new.
- **The two fields and the two checkboxes carry no icon**: a field draws its value, a checkbox its tick box.

### Word's Print Preview

**One tab of one application, and the second view tab authored**, after Word's Outlining. Four groups and
seventeen commands, drawn in Office's order, which is also the census's: Print, Page Setup, Zoom, Preview. It
is Word's classic Print Preview (Word 2007 and 2010, and Microsoft 365's *Print Preview Edit Mode*): the document
as it will print, with the page setup and zoom a person needs to judge it.

⚠ **A view tab renders in the catalogue alone, and this is the first with menus.** `tabsFor` leaves every
`appearance: 'view'` tab out of a strip unless `includeViewTabs` is asked for, and only `Ribbons/Word` asks, so
the tab's five bindings and `printPreviewMenus('word', 'ribbons')` are in `stories/ribbons/word.stories.ts` alone.
**The surface gate required every declared menu of both hosts**, which for a view tab would have demanded a
binding to nothing in `Shell/Word`. `tests/ribbons.test.ts` now asks a view tab's menu of the host that draws
view tabs alone (`hostDrawsViewTabs`, held to each host's `includeViewTabs` in its source), and refuses a shell
that opens one.

- **Three dropdowns a host binds**: Margins, Orientation and Size, over `stories/ribbons/print-preview-menus.ts`,
  which opens **Layout's own lists**: `marginEntries`, `orientationEntries` and `sizeEntries` are now exported
  from `stories/ribbons/design-layout-menus.ts` rather than copied. The file is keyed by application, as
  `view-menus.ts` is, so PowerPoint's and Excel's Print Preview units each add one function to it.
- **Two checkboxes a host binds**: Show Ruler and Magnifier (ticked).
- **Buttons**: Print, Options, Zoom, 100%, One Page, Two Pages, Page Width, Shrink One Page, Next Page, Previous
  Page and Close Print Preview.
- **One dialog launcher**, on Page Setup, as on Layout. No gallery, no split button, no exclusive set, no toggle
  button.

**Survivors**: 100%, One Page and Page Width in Zoom, as on View; Next Page and Previous Page in Preview, the
Mailings record-navigator standard. Two Pages has no glyph. Shrink One Page fails rule 2, because arrows pressed
together read as *minimise*. Close Print Preview leaves the view.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Magnifier is a ticked checkbox, where the brief listed a toggle.** Office stacks Show Ruler and Magnifier
  as two ticks above Shrink One Page. This also avoids a second magnifier beside Zoom's.
- **The census counts more than Office draws, and nothing is padded**: Page Setup is 6 and draws 3 and a
  launcher, and Zoom is 6 and draws 5. Zoom shares `GroupZoom`, View's census id, and is found inside this tab.
- **Layout's Margins and Size lists are completed, and Layout's tab changes with them.** Margins adds *Office
  2003 Default*, and *Last Custom Setting* at the top once custom margins exist (never in the catalogue's new
  document). Size is Word's standard paper list, Letter to Envelope Monarch, A4 checked. ⚠ Office's real Size
  list comes from the printer; `GUESS:` the order.
- **Zoom draws `zoom-in`**, as PowerPoint's and Excel's View do for the same dialog, where Word's View draws none.
- **Every glyph is judged from Fluent's drawings.** `settings` gains a 24 for Options. `arrow-minimize-vertical`,
  `document-arrow-down` and `document-arrow-up` are new. Next Page and Previous Page may read as *download* and
  *upload*.
- **Carrying no glyph**: Size (Fluent draws no page size, Layout's reason), Two Pages (every two-page picture is
  already Read Mode, Arrange All, Columns or Handout Master), and the two checkboxes.

### Word's Background Removal

**One tab of one application, and the third view tab authored**, after Word's Print Preview. Two groups and
four commands, in Office's order, which is also the census's: Refine and Close. Office shows the tab only while
a picture's background is being removed, so it renders in `Ribbons/Word` alone, and it binds nothing.

- **Written once, as functions of the application**: `backgroundRemovalRefineCommands` and
  `backgroundRemovalCloseCommands` in `dev/ribbons/census.ts`. PowerPoint's and Excel's census rows are the
  same two groups with the same counts, so their units add `commands:` to two rows and a tab function.
- **Two large toggles in one exclusive set that may hold none**: Mark Areas to Keep and Mark Areas to Remove,
  neither pressed. This unit added `exclusive-allows-none`; see *One of a set*.
- **Two large buttons**: Discard All Changes and Keep Changes.
- No menu, no gallery, no split button, no field, no checkbox, no dialog launcher.

**No survivors.** Both pencils arm a gesture, which fails rule 1 as on Draw. Discard All Changes is
irreversible, and Keep Changes leaves the view.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Refine counts 3 and draws 2.** Delete Mark, which Office 2010 to 2016 drew for their straight marks, is
  left out because Microsoft 365 draws free-form strokes and no Delete Mark. Nothing is padded. `GUESS:`.
- **The pencils start with neither pressed and release on a second press.** `GUESS:` both, from Office's
  other arm-a-gesture commands.
- **Close's order is the brief's**, Discard All Changes then Keep Changes. `GUESS:` that it is Office's.
- **Four circles.** Mark Areas to Keep and Remove draw `add-circle` and `subtract-circle`, the plus and minus
  of Office's pencils, since Fluent draws no pencil with either. Discard All Changes and Keep Changes draw
  `dismiss-circle` and `checkmark-circle`. The pairs share an outline, which is the weakest visual choice on
  the tab. Every glyph is `GUESS:`.
- **Every command carries a glyph.**

### PowerPoint's Background Removal

**One tab of one application, and PowerPoint's first view tab authored**, after Word's Background Removal. It is
Word's tab under PowerPoint's ids: the census row calls `backgroundRemovalRefineCommands('powerpoint')` and
`backgroundRemovalCloseCommands('powerpoint')`, and `powerpointBackgroundRemovalTab` renders the two groups. It
renders in `Ribbons/PowerPoint` alone and binds nothing. No glyph, binding or menu was added.

- **Its pencils are their own set**, `powerpoint.background-removal.refine`, and may hold none, as Word's do.
  `tests/ribbons.test.ts` lists both sets.
- **No PowerPoint-specific disagreement.** The census's PowerPoint row is Word's to the field (group ids,
  labels, priorities, counts 3 and 2), and Office draws the same four commands. Everything under *Word's
  Background Removal* above, `GUESS:` items included, applies unchanged and is not restated.

### Excel's Background Removal

**One tab of one application, and Excel's first view tab authored**, after PowerPoint's Background Removal. It
is Word's tab under Excel's ids: the census row calls `backgroundRemovalRefineCommands('excel')` and
`backgroundRemovalCloseCommands('excel')`, and `excelBackgroundRemovalTab` renders the two groups. It renders in
`Ribbons/Excel` alone and binds nothing. No glyph, binding or menu was added.

- **Its pencils are their own set**, `excel.background-removal.refine`, and may hold none, as Word's and
  PowerPoint's do. `tests/ribbons.test.ts` lists all three sets.
- **No Excel-specific disagreement.** The census's Excel row is Word's to the field (group ids, labels,
  priorities, counts 3 and 2), and Office draws the same four commands. Everything under *Word's Background
  Removal* above, `GUESS:` items included, applies unchanged and is not restated.

### PowerPoint's Print Preview

**One tab of one application, and PowerPoint's second view tab authored**, after Excel's Background Removal.
Four groups and ten commands, in Office's order, which is also the census's: Print, Page Setup, Zoom, Preview. It
is PowerPoint 2007's Print Preview, the last PowerPoint with the tab: the deck as it will print, in whichever
printout shape is chosen. **Nothing is shared with Word's tab but four group ids and some glyphs.**

It renders in `Ribbons/PowerPoint` alone, as every view tab does: the four bindings and
`printPreviewMenus('powerpoint', 'ribbons')` are in `stories/ribbons/powerpoint.stories.ts`, and
`Shell/PowerPoint` draws neither.

- **Two fields a host binds**: Print What (Slides, Handouts at 1, 2, 3, 4, 6 and 9 slides per page, Notes Pages,
  Outline View; starts on Slides) and Colour/Greyscale (Colour, Greyscale, Pure Black and White; starts on
  Colour). Their lists are `powerpointPrintWhat` and `powerpointPrintColourModes` in
  `stories/ribbons/print-preview-menus.ts`.
- **Two dropdowns a host binds**: Options, over PowerPoint's printing options, and Orientation, over **Layout's
  `orientationEntries()` unchanged**. Margins and Size are not reused, because PowerPoint's tab has neither.
- **Buttons**: Print, Zoom, Fit to Window, Next Page, Previous Page and Close Print Preview. No dialog launcher,
  toggle, checkbox, exclusive set, gallery or split button.

**Survivors**: Fit to Window in Zoom, as on View; Next Page and Previous Page in Preview, as on Word's Print
Preview. Print, Options and Zoom open a dialog or a menu, and the fields are refused by the gate.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Page Setup counts 5 and draws 3.** `GUESS:` the census counts each field's caption as a control. Print (2),
  Zoom (2) and Preview (3) draw exactly their counts.
- **Colour/Greyscale is a Page Setup field, as the brief places it.** PowerPoint 2007 is remembered with it as a
  submenu of Options, so Options' menu leaves that submenu out rather than carry one setting twice. `GUESS:`.
- **The census's spelling wins**: *Colour/Greyscale*, *Colour*, *Greyscale*, where Office and the brief write
  *Color/Grayscale*.
- **Options is a menu, where Word's is a button**: Header and Footer…, Scale to Fit Paper, Frame Slides, Print
  Comments and Ink Markup, a *Print Order* section (Horizontal checked, Vertical), Print Hidden Slides. `GUESS:`
  every entry and tick.
- **Orientation is small, under Print What**, and is available where Office greys it while Print What is Slides.
  `GUESS:` the size, and that PowerPoint's two entries are Word's.
- **No dialog launcher on Page Setup**, where Word's has one. `GUESS:`.
- **Every face command but the two fields carries a glyph, and no glyph is new**: `print`, `settings`,
  `orientation`, `zoom-in`, `page-fit`, `document-arrow-down`, `document-arrow-up`, `dismiss-square`. Each is
  the glyph this subset already draws for the same command.

### Excel's Print Preview

**One tab of one application, and Excel's second view tab authored**, after PowerPoint's Print Preview. Three
groups and seven commands. It is Excel 2007's Print Preview, the last Excel with the tab: the sheet as it will
print, a page at a time. **Nothing is declared once with Word's or PowerPoint's.** Two group ids are shared, and
Zoom is `GroupPrintPreviewZoom` rather than View's `GroupZoom`.

⚠ **Drawn in Office's order, Print, Zoom, Preview; the census declares Print, Preview, Zoom.** The census row
keeps its own order, because it is a transcription. `excelPrintPreviewTab` follows Office, as Excel's Review and
View do. The ladder collapses by priority, not position, so Zoom (`ancillary`) still gives way first.

It renders in `Ribbons/Excel` alone, as every view tab does. The one binding and `printPreviewMenus('excel',
'ribbons')` are in `stories/ribbons/excel.stories.ts`, and `Shell/Excel` draws neither.

- **One checkbox a host binds**: Show Margins, unticked.
- **Buttons**: Print, Page Setup, Zoom, Next Page, Previous Page and Close Print Preview.
- **No menu.** `excelPrintPreviewMenus` is present and renders an empty set, because Excel's tab opens none. No
  field, dialog launcher, toggle button, exclusive set, gallery or split button.

**Survivors**: Next Page and Previous Page in Preview, as on Word's and PowerPoint's tabs.
- Print and Page Setup open dialogs.
- Zoom is its group's only command, so a survivor would leave the collapsed popup empty.
- Show Margins is a checkbox, which the gate refuses.
- Close Print Preview leaves the view.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Show Margins is a checkbox, where the brief lists a toggle**, as Word's Magnifier is. Excel 2007 draws it as
  a check box under Next Page and Previous Page. `GUESS:` the shape and the unticked start.
- **Zoom is a plain large button**, which switches the preview between the whole page and a magnified page. It is
  not View's Zoom dialog. `GUESS:` that Office does not draw it pressed while magnified.
- **Page Setup draws `settings`, a cog**, the glyph Word's and PowerPoint's Options draw in the same slot.
  `GUESS:`. Fluent draws no page with a cog, and every page glyph in the subset is already another command.
  Excel's tab has no Options for the cog to be confused with.
- **The counts agree**: Print 2, Preview 4, Zoom 1, each drawn exactly.
- **Every command but the checkbox carries a glyph, and no glyph is new**: `print`, `settings`, `zoom-in`,
  `document-arrow-down`, `document-arrow-up`, `dismiss-square`.

### PowerPoint's Slide Master

**One tab of one application, and PowerPoint's third view tab authored**, after Excel's Print Preview. Six groups
and eighteen commands, in Office's order, which is also the census's: Edit Master, Master Layout, Edit Theme,
Background, Size, Close. It is the tab Slide Master view shows: the master and its layouts, the placeholders a
layout carries, and the theme and background every slide inherits.

**Three groups are declared once for every master view.** Edit Theme, Background and Close are the same rows, with
the same counts, on `TabSlideMaster`, `TabHandoutMaster` and `TabNotesMaster`, so the census declares them as
`masterEditThemeCommands`, `masterBackgroundCommands` and `masterCloseCommands`, functions of the master view.
`stories/ribbons/slide-master-menus.ts` shares their lists the same way (`editThemeMenuEntries`,
`backgroundMenuEntries`); a menu's id must be spelt literally, so the Handout Master and Notes Master units each add
one function to its `menusByTab` and write no list.

It renders in `Ribbons/PowerPoint` alone, as every view tab does: the ten bindings and
`masterViewMenus('powerpoint', 'ribbons')` are in `stories/ribbons/powerpoint.stories.ts`, and `Shell/PowerPoint`
draws neither.

- **One split button a host binds**: Insert Placeholder, whose arrow opens Office's ten placeholders.
- **Six dropdowns a host binds**: Themes, Colours, Fonts, Effects, Background Styles and Slide Size, **every list
  reused from `stories/ribbons/design-layout-menus.ts`**. Themes is the Design gallery's `themes` as a menu
  (`powerpointThemeEntries`).
- **Three checkboxes a host binds**: Title and Footers, ticked; Hide Background Graphics, unticked.
- **One toggle**: Preserve, unpressed.
- **Buttons**: Insert Slide Master, Insert Layout, Delete, Rename, Master Layout and Close Master View.
- **One dialog launcher**, *Format Background*, on Background.

**No survivors.** Insert Slide Master's and Insert Layout's glyphs read as New Slide and Layout without a label;
Rename, Master Layout and every menu open something; Delete removes; un-pressing Preserve on an unused master asks to
delete it; the checkboxes are refused by the gate; Size and Close hold one command each.

**Shared lists changed.** Office's Colours, Fonts, Effects and Background Styles carry more than Design did, so
`themeColourEntries` (24 sets), `themeFontEntries` (20 pairs), `themeEffectEntries` (15) and
`backgroundStyleEntries` (Style 1 to 12) are now whole, and the Design gallery's `themes` holds Office's 31. **Word's
Design, Excel's Page Layout and PowerPoint's Design tab draw the same lists.** `slideSizeEntries` is exported.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Colours, Fonts and Effects are in Edit Theme**, as the brief and PowerPoint 2010 place them. Microsoft 365 draws
  them in Background. The census's Edit Theme count of 4 fits the brief. `GUESS:` the 365 placement.
- **Master Layout counts 15 and draws 4.** `GUESS:` Insert Placeholder's two halves and ten entries are twelve.
- **Background counts 11 and draws 2 and a launcher.** `GUESS:` the census reads 365's group with Colours, Fonts and
  Effects and some menu footers; no reading reaches 11 exactly.
- **Size counts 2 and draws 1.** `GUESS:` Slide Size's face and arrow. Edit Master (5), Edit Theme (4) and Close
  (1) draw their counts.
- **Every entry of every theme list**, their order, and the font pairs are `GUESS:`, from memory of Microsoft 365.
- **Preserve starts unpressed**, and Title and Footers ticked. `GUESS:` both.
- **Office greys** Delete, Rename, Title, Footers, Hide Background Graphics and Reset Slide Background in some
  selections. All are drawn available, because `disabled` is loop 2's.
- **Insert Placeholder's entries carry no glyph**, where Office draws one each: Fluent has no vertical content
  placeholder, and nine glyphs with a gap would read as broken.
- **Glyphs**, every one `GUESS:`. Six are new: `slide-text-title-add` (Insert Slide Master), `rename`, `pin`
  (Preserve, filled while pressed), `slide-text-title-checkmark` (Master Layout), `slide-content` (Insert
  Placeholder) and `style-guide` (Themes, the first Themes to carry a glyph). Reused: `slide-layout` (Insert Layout,
  now also at 24), `delete`, `color`, `text-font`, `square-shadow`, `color-background`, `slide-size`,
  `dismiss-square`. Title, Footers and Hide Background Graphics are checkboxes and carry none.

### PowerPoint's Slide Master Home

**One tab of one application, and PowerPoint's fourth view tab authored**, after Slide Master. Six groups and
forty-four commands, in Office's order, which is also the census's: Clipboard, Master Slides, Font, Paragraph,
Drawing, Editing. It is `TabSlideMasterHome`, the Home tab Slide Master view shows beside Slide Master. Office labels
it *Home*; the ordinary Home is never on screen with it, and `Ribbons/PowerPoint` is the one place the two collide.

**Five groups are Home's, written once.** Clipboard, Font, Paragraph, Drawing and Editing carry Home's ids and
Home's counts, so the census declares them as `powerpointHomeClipboardCommands`, `powerpointHomeFontCommands`,
`powerpointHomeParagraphCommands`, `powerpointHomeDrawingCommands` and `powerpointHomeEditingCommands`, functions of
the Home tab, as `fileOpenCommands` is a function of the application. Home calls them with `'home'` and draws what it
drew before; this tab calls them with `'slide-master-home'`, so every id is distinct. **Home's host markup is shared
the same way**: `homeBinding` in `stories/ribbons/powerpoint.stories.ts` writes Paste, Font, Font size, Font colour,
Shape styles and Arrange once. Both tabs' bindings are still keyed by literal ids, and each tab passes its own DOM
ids. Home passes the ids it always had.

It renders in `Ribbons/PowerPoint` alone, as every view tab does: the eight bindings and the two menus
`masterViewMenus('powerpoint', 'ribbons')` renders are in that story file and `stories/ribbons/slide-master-menus.ts`,
and `Shell/PowerPoint` draws neither.

**Master Slides** is Office's master-view counterpart to Home's Slides:

- **Buttons**: Insert Slide Master and Insert Layout, large, Slide Master's commands with Slide Master's glyphs;
  Reset, small, Home's.
- **Two dropdowns a host binds**: Layout, over the Office Theme's eleven layouts, and Section, over Add Section,
  Rename Section, Remove Section, Remove All Sections, Collapse All and Expand All.

**Survivors are Home's**: Bold, Italic and Underline in Font; Align Left, Centre and Align Right in Paragraph.
**Master Slides keeps none**: Layout and Section open menus, the two insert glyphs read as New Slide and Layout
without a label, and Reset's loop fails rule 2, as on Home.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Master Slides counts 9 and draws 5.** `GUESS:` the census counts the five commands and Section's four entries
  that change sections, and not Collapse All and Expand All. The other five groups' counts agree with Home's.
- **Layout draws `layout-row-two-split-top`, new**, not Home's `slide-layout`, because Insert Layout wears
  `slide-layout` in the same group. `GUESS:` the glyph.
- **Layout and Section are dropdowns here and buttons on Home.** Home is unchanged by this unit.
- **Layout lists eleven layouts; Insert's New Slide lists seven**, so neither list is reused. `GUESS:` the eleven,
  their order, and that none is ticked.
- **Office greys Section and Reset in master view.** `GUESS:`. Both are drawn available, because `disabled` is
  loop 2's.
- **The brief names Shapes as a host control; Home binds no Shapes**, so this tab binds Home's six and Shapes stays
  the generic button.
- **Home's four dialog launchers are kept.** `GUESS:` that master view keeps them.
- **One glyph is new**, `layout-row-two-split-top`. Reused: `slide-text-title-add`, `slide-layout`, `arrow-reset`,
  `slide-multiple`, and every Home glyph in the five shared groups.

### PowerPoint's Handout Master

**One tab of one application, and PowerPoint's fifth view tab authored**, after Slide Master Home. Five groups and
fourteen commands, in Office's order, which is also the census's: Page Setup, Placeholders, Edit Theme, Background,
Close. It is `TabHandoutMaster`, the tab Handout Master view shows: one printed handout page, the slide frames on it,
and the header, date, footer and page number around them.

**Three groups are Slide Master's, called and not rewritten.** Edit Theme, Background and Close are
`masterEditThemeCommands('handout-master')`, `masterBackgroundCommands('handout-master')` and
`masterCloseCommands('handout-master')`, and their menus are `editThemeMenuEntries` and `backgroundMenuEntries` under
this tab's ids, in `handoutMasterMenus`, the function this unit added to `menusByTab` in
`stories/ribbons/slide-master-menus.ts`. No theme list is written twice. **Page Setup and Placeholders are this tab's
own.**

It renders in `Ribbons/PowerPoint` alone, as every view tab does: the thirteen bindings and
`masterViewMenus('powerpoint', 'ribbons')` are in `stories/ribbons/powerpoint.stories.ts`, and `Shell/PowerPoint`
draws neither.

- **Eight dropdowns a host binds**: Handout Orientation (Layout's `orientationEntries()`), Slide Size (Design's
  `slideSizeEntries()`), Slides Per Page (Office's seven handout layouts, this tab's own list), and Slide Master's
  Themes, Colours, Fonts, Effects and Background Styles.
- **Five checkboxes a host binds**: Header, Date, Footer and Page Number, ticked; Hide Background Graphics, unticked.
- **One button**: Close Master View.
- **One dialog launcher**, *Format Background*, on Background.

**No survivors.** Page Setup's three and Edit Theme's four open menus; Placeholders' four are checkboxes, which the
gate refuses; Background holds a menu and a checkbox; Close leaves the view.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Page Setup counts 11 and draws 3.** `GUESS:` the census counts the eleven choices the three menus offer (Portrait
  and Landscape; Standard and Widescreen; 1, 2, 3, 4, 6, 9 Slides and Outline), not the faces or *Custom Slide
  Size…*. It is the one reading found that reaches 11 exactly. Placeholders (4), Edit Theme (4) and Close (1) draw
  their counts; Background (11) is Slide Master's disagreement, unchanged.
- **Microsoft 365's Page Setup is drawn, not PowerPoint 2010's**, which had Page Setup, Handout Orientation, Slide
  Orientation and Slides Per Page. `GUESS:` both.
- **Slides Per Page starts on 6 Slides**, and its labels and order are `GUESS:`. Print Preview's Print What names the
  same layouts as *Handouts (n Slides Per Page)*, so neither list is reused.
- **All four placeholders start ticked**, and Handout Orientation on Portrait. `GUESS:` both.
- **Office greys Themes on Handout Master.** `GUESS:`. It is drawn available, because `disabled` is loop 2's.
- **The Format Background launcher is kept.** `GUESS:` that Handout Master draws it, as Slide Master does.
- **No glyph is new.** Slides Per Page draws `layout-cell-four`, Excel's Arrange All glyph, a page divided into four
  frames (`GUESS:`; never on one ribbon with Excel's). Handout Orientation draws `orientation` and Slide Size
  `slide-size`; the shared groups draw Slide Master's `style-guide`, `color`, `text-font`, `square-shadow`,
  `color-background` and `dismiss-square`. The five checkboxes carry none.

### PowerPoint's Notes Master

**One tab of one application, and PowerPoint's sixth view tab authored**, after Handout Master. Five groups and
fifteen commands, in Office's order, which is also the census's: Page Setup, Placeholders, Edit Theme, Background,
Close. It is `TabNotesMaster`, the tab Notes Master view shows: one printed notes page, the slide image at its top,
the notes body under it, and the header, date, footer and page number around them.

**Three groups are Slide Master's, called and not rewritten**, as on Handout Master: Edit Theme, Background and Close
are `masterEditThemeCommands('notes-master')`, `masterBackgroundCommands('notes-master')` and
`masterCloseCommands('notes-master')`, and their menus are `editThemeMenuEntries` and `backgroundMenuEntries` under
this tab's ids, in `notesMasterMenus`, the function this unit added to `menusByTab` in
`stories/ribbons/slide-master-menus.ts`. **Page Setup and Placeholders are this tab's own, and it writes no list**:
both Page Setup menus reuse lists already written.

It renders in `Ribbons/PowerPoint` alone, as every view tab does: the fourteen bindings and
`masterViewMenus('powerpoint', 'ribbons')` are in `stories/ribbons/powerpoint.stories.ts`, and `Shell/PowerPoint`
draws neither.

- **Seven dropdowns a host binds**: Notes Page Orientation (Layout's `orientationEntries()`), Slide Size (Design's
  `slideSizeEntries()`), and Slide Master's Themes, Colours, Fonts, Effects and Background Styles.
- **Seven checkboxes a host binds**: Header, Slide Image, Footer, Date, Body and Page Number, ticked; Hide Background
  Graphics, unticked.
- **One button**: Close Master View.
- **One dialog launcher**, *Format Background*, on Background.

**No survivors.** Page Setup's two and Edit Theme's four open menus; Placeholders' six are checkboxes, which the gate
refuses; Background holds a menu and a checkbox; Close leaves the view.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Page Setup counts 3 and draws 2.** `GUESS:` the census counts Slide Size's face and arrow as two, as Slide
  Master's Size does, plus Notes Page Orientation. PowerPoint 2010's group (Page Setup, Notes Page Orientation, Slide
  Orientation) also reaches 3. **Handout Master's reading, the choices the menus offer, gives 4 here**, so the
  census's two Page Setup counts are not read one way; that is recorded, not reconciled. Placeholders (6), Edit Theme
  (4) and Close (1) draw their counts; Background (11) is Slide Master's disagreement, unchanged.
- **Background is `primary` and Page Setup `standard`**, the census's priorities, kept. They are the reverse of
  Handout Master's, so as the ribbon narrows here Background collapses after Page Setup, Placeholders and Edit Theme,
  where on Handout Master it collapses before Page Setup.
- **Microsoft 365's Page Setup is drawn, not PowerPoint 2010's.** `GUESS:` both.
- **All six placeholders start ticked**, in Office's two columns of three read down each column, and Notes Page
  Orientation starts on Portrait. `GUESS:` all three.
- **Office greys Themes on Notes Master.** `GUESS:`. It is drawn available, because `disabled` is loop 2's.
- **The Format Background launcher is kept.** `GUESS:` that Notes Master draws it, as Slide Master does.
- **No glyph is new.** Notes Page Orientation draws `orientation` and Slide Size `slide-size`; the shared groups draw
  Slide Master's `style-guide`, `color`, `text-font`, `square-shadow`, `color-background` and `dismiss-square`. The
  seven checkboxes carry none.

### PowerPoint's Black and White

**One tab of one application, and PowerPoint's seventh view tab authored**, after Notes Master. Two groups and eleven
commands, in Office's order, which is also the census's: Colour Mode and Close. It is `TabBlackAndWhite`, the tab
Office shows while the deck is previewed as a black-and-white printer would print it. The tab chooses how the
**selected object** is drawn in that preview, and changes no slide's colours.

**Both groups are written once, as functions of the colour-mode tab**: `colourModeSettingCommands` and
`colourModeCloseCommands` in `dev/ribbons/census.ts`. The census's Greyscale row (`TabGrayscale`) is the same two
groups with the same labels, priorities and counts, so its unit added `commands:` to two rows and a tab function; see
*PowerPoint's Greyscale*.

It renders in `Ribbons/PowerPoint` alone, as every view tab does, and **binds nothing**: every command is the generic
toggle or button, so there is no binding and no menu to write.

- **Ten toggles in one exclusive set**, `powerpoint.black-and-white.colour-mode`, Automatic pressed: Automatic,
  Greyscale, Light Greyscale, Inverse Greyscale, Grey with White Fill, Black with Greyscale Fill, Black with White
  Fill, Black, White and Don't Show. `tests/ribbons.test.ts` lists the set and its start.
- **One button**: Back To Colour View.
- No menu, no gallery, no field, no checkbox, no split button, no dialog launcher.

**Why toggles and not a gallery.** Office draws a row of labelled buttons, with no scroll arrows, expander or popup
grid. The census counts 10, one per setting, where a gallery would be one control. The choice has one current value
that another press replaces, which is what `exclusive` holds. And a gallery's items preview a result, which here
would be colour swatches the subset cannot draw.

**No survivors.** Colour Mode is an exclusive set, which rule 1 refuses, as on View. Five of its members carry no
glyph besides. Back To Colour View leaves the view.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **The group is labelled *Colour Mode***, the census's label, where Office writes *Change Selected Object*.
  Office's *Grayscale*, *Gray* and *Back To Color View* are spelt Greyscale, Grey and Back To Colour View. The
  counts agree: 10 and 1.
- **Automatic starts pressed, and the set always holds one.** Office reflects the selection: it greys the ten with
  nothing selected and highlights none for a mixed selection. `GUESS:` Automatic and the greying. A second press
  keeps the setting (`GUESS:`), so the set does not allow none.
- **This tab's set and Greyscale's are separate.** `GUESS:` that Office keeps the two settings apart. The ten map
  to DrawingML's `ST_BlackWhiteMode` without `clr` (`GUESS:`, from memory), one `bwMode` per shape, so a
  document-backed host might show one choice on both tabs.
- **Five settings are small where Office draws all ten large**, because they carry no glyph: Automatic, Grey with
  White Fill, Black with Greyscale Fill, Black with White Fill, Black and White. A glyph is one tint from a token, so
  it has no grey, and its ink turns light in the dark theme. A pressed toggle draws Fluent's *filled* drawing, which
  on a fill-named command changes the meaning. Office draws coloured swatches, which the subset refuses. Automatic is
  a rule, not a look, and Fluent's *auto* glyphs are a camera flash and a translation.
- **Greyscale, Light Greyscale, Inverse Greyscale, Don't Show and Back To Colour View are large, unmeasured.**
  `GUESS:` each label wraps into two lines, by comparison with Word's Close Outline View.
- **One glyph is new**, `brightness-high` (Light Greyscale, a sun). Reused: `color-off` (View's Greyscale, gaining a
  24), `dark-theme` (Word's and Excel's Switch Modes, for Inverse Greyscale), `eye-off` (Excel's Hide, gaining a 24
  and the filled drawing, for Don't Show), and `dismiss-square` (every view tab's close). Every glyph is `GUESS:`.

### PowerPoint's Greyscale

**One tab of one application, and PowerPoint's eighth and last view tab authored**, after Black and White. Two groups
and eleven commands, in Office's order, which is also the census's: Colour Mode and Close. It is `TabGrayscale`, the
tab Office shows while the deck is previewed in greyscale. The tab chooses how the **selected object** is drawn in
that preview, and changes no slide's colours. **It was the last core or view placeholder**: every in-scope core and
view tab of the three applications is now authored. The contextual tabs follow; see *The contextual tab sets*.

**It writes nothing of its own.** Its two census rows call `colourModeSettingCommands('greyscale')` and
`colourModeCloseCommands('greyscale')`, and `powerpointGreyscaleTab` places them. Every label, size, glyph, priority
and count is Black and White's, and so is every point under *PowerPoint's Black and White*'s ⚠ list. It renders in
`Ribbons/PowerPoint` alone and **binds nothing**.

- **Ten toggles in their own exclusive set**, `powerpoint.greyscale.colour-mode`, Automatic pressed, apart from
  `powerpoint.black-and-white.colour-mode`. `tests/ribbons.test.ts` lists the set and its start.
- **One button**: Back To Colour View.
- No menu, no gallery, no field, no checkbox, no split button, no dialog launcher, no survivor. No new glyph.

⚠ **Greyscale-specific differences from Office: none known.** Office draws the same *Change Selected Object* group,
with the same ten settings in the same order, and the same *Back To Color View*. Two things might differ, and neither
is drawn:

- **The start.** `GUESS:` that Office starts a new shape on Automatic in greyscale as it does in black and white.
- **The shared setting.** If Office keeps one `bwMode` per shape, a setting pressed here would show on Black and White
  too. The two sets are independent here.

### The contextual tab sets (the four common sets)

**Declared, then authored one tab of one application at a time; Word's Table Design is the first, Word's Table
Layout the second, PowerPoint's Table Design the third, PowerPoint's Table Layout the fourth, Excel's Table Design
the fifth, Word's Picture Format the sixth, PowerPoint's Picture Format the seventh, Excel's Picture Format the
eighth, PowerPoint's Shape Format the ninth, Word's Shape Format the tenth, Excel's Shape Format the eleventh,
Word's Chart Design the twelfth, PowerPoint's Chart Design the thirteenth, Excel's Chart Design the fourteenth, and
Word's Chart Format the fifteenth.** A
contextual tab
is one Office shows only while something is selected, under a coloured
band naming its set, and the census writes it as a row whose `tab_set` is a `TabSet*` id. Until this unit the three
modules drew their sets with a hand-written one-button stub, with no census entry behind it. Now each set is a
`RibbonContextualSetEntry` in `dev/ribbons/census.ts`: a kebab id, the census set id, an English band label and its
tabs. Each tab is an ordinary `RibbonTabEntry` with `appearance: 'contextual'` and
`source: { kind: 'contextual', tabSet, tab }`, and **its groups are transcribed with no commands**, so each tab can be
authored one tab of one application at a time, exactly as the view tabs were.

**Only the four common sets are built; the user decided that on 2026-09-15.**

| Set | Word | PowerPoint | Excel |
|---|---|---|---|
| Table Tools | Table Design (3 groups), Layout (7) | Table Design (4), Layout (7) | Table Design (5), from `TabSetTableToolsExcel` |
| Picture Tools | Picture Format (6) | Picture Format (6) | Picture Format (6) |
| Drawing Tools | Shape Format (7) | Shape Format (6) | Shape Format (6) |
| Chart Tools | Chart Design (4), Format (7) | Chart Design (4), Format (7) | Chart Design (5), Format (7) |

Every other in-scope set is recorded in `unbuiltContextualSets` with that reason: thirteen per application, among
them SmartArt, Equation, Ink, 3D Model, Graphics (SVG), Header & Footer, Audio and Video, PivotTable and PivotChart.
`tests/ribbons.test.ts` holds the model three ways:

- **Group identity.** Each declared contextual tab's groups and counts must equal the census's in-scope rows for its
  set *and* tab, through the same `groupIdentityFindings` every core tab goes through.
- **Coverage.** Every in-scope `TabSet*` tab must be accounted for exactly once: declared, recorded in its built
  set's `unbuiltTabs`, or recorded in an unbuilt set. A set dropped from the record fails, and so do a tab recorded
  twice and a record of a tab the census does not carry.
- **The decision itself.** The built sets and their tab labels are pinned to the table above.

Each refusal is watched firing on a doctored entry, including the right tab id in the wrong set. The rubric, the
placeholder priority and the id checks now sweep contextual tabs too.

**Rendering.** `contextualSetsFor` in `stories/ribbons/ribbon-parts.ts` draws each set as the
`<mjx-contextual-tab-set>` it always was, label from the census. `<app>ContextualSets(options)` takes an optional
`sets` list and a host's `controls`, and each contextual tab goes through a `contextualBuilders` entry. Every entry
was `placeholderTab` when the sets were declared; **Word's Table Design is the first to be replaced**, see *Word's
Table Design* below, **Word's Table Layout the second**, see *Word's Table Layout*, **PowerPoint's Table Design
the third**, see *PowerPoint's Table Design*, **PowerPoint's Table Layout the fourth**, see *PowerPoint's Table
Layout*, **Excel's Table Design the fifth**, see *Excel's Table Design*, **Word's Picture Format the sixth**, see
*Word's Picture Format*, **PowerPoint's Picture Format the seventh**, see *PowerPoint's Picture Format*, the first
contextual tab `Shell/PowerPoint` shows authored, **Excel's Picture Format the eighth**, see *Excel's Picture
Format*, **PowerPoint's Shape Format the ninth**, see *PowerPoint's Shape Format*, the first of Drawing Tools,
**Word's Shape Format the tenth**, see *Word's Shape Format*, **Excel's Shape Format the eleventh**, see *Excel's
Shape Format*, **Word's Chart Design the twelfth**, see *Word's Chart Design*, the first of Chart Tools,
**PowerPoint's Chart Design the thirteenth**, see *PowerPoint's Chart Design*, **Excel's Chart Design the
fourteenth**, see *Excel's Chart Design*, the last Chart Design of the three, and **Word's Chart Format the
fifteenth**, see *Word's Chart Format*, the first Chart Format and the last of Word's contextual tabs.

- **`Ribbons/*` draws all four sets**, so each contextual tab has its own story (`TableDesign`, `TableLayout`,
  `PictureFormat`, `ShapeFormat`, `ChartDesign`, `ChartFormat`).
- **`Shell/*` names the one set its document's selection shows**: Table Tools in Word and Excel, Picture Tools in
  PowerPoint, which are the sets the stubs drew.

`stubTab` is gone. **A per-tab unit** declares the tab's commands in the census, replaces its one `contextualBuilders`
line with a `<app><Tab>Tab` function, and binds and documents it as every earlier tab unit did.

⚠ **Four readings, recorded in `dev/ribbons/census.ts`'s header:**

- **Chart Tools is three generations in the census.** `TabChartToolsDesignNew` and `TabChartToolsFormatNew`
  (Office 2013+) are built. `TabChartToolsDesign`, `TabChartToolsFormat` and `TabChartToolsLayout` (Office
  2007–2010) are recorded in the set's `unbuiltTabs` with their own reason.
- **The group labels are Microsoft 365's**, because the inventory names none of these groups and several ids read
  wrongly on their own. `GroupTableLayout` is Word's Table Style Options (`GUESS:`), `GroupPictureTools` is Adjust
  and `GroupTextStylesTable` is WordArt Styles. `GroupImagePlay` is the one label derived from its id, *Image Play*.
- **Two contextual tabs are labelled *Layout* and *Format***, as Office labels them. The accessible name carries the
  set (*Layout, Table Tools*), which is what tells them apart from the core Layout tab.
- **The rubric binds small counts.** Word's and PowerPoint's Chart Styles and Excel's Chart Data count 2, so they are
  `secondary`.

⚠ **The menu gate knows which contextual sets each host draws.** Until PowerPoint's Table Design, `surfaceFindings`
treated a contextual command like an `always` one and required its menu of both hosts; Word's two Table Tools tabs
never met the difference, because both Word hosts draw Table Tools. PowerPoint's shell draws Picture Tools, so
**PowerPoint's Table Design taught the gate**, as Word's Print Preview did for view tabs:

- **`hostContextualSets`** in `tests/ribbons.test.ts` says which sets each host draws per application: every built
  set for `Ribbons/*`, Table Tools for `Shell/Word` and `Shell/Excel`, Picture Tools for `Shell/PowerPoint`.
- **`hostDrawsTabOf(host, application, command)`** combines it with `hostDrawsViewTabs`. A contextual command's menu
  is required only of the hosts that draw its set, and a host that opens the menu of a set it never draws is refused
  as a binding to nothing.
- **The table is held to the hosts' source**: `contextualSetsDrawnIn` reads each `<app>ContextualSets(…)` call's
  literal `sets: [...]`, or *every* for a call with none, and must equal the table. Every set it names must be built.
- **Self-tests** watch it read the calls, require a Table Design menu of `Ribbons/PowerPoint` and not of
  `Shell/PowerPoint`, and refuse a PowerPoint shell that opens one. A further check requires at least one declared
  contextual menu that a shell does not draw, so the exemption is known to exempt something.

### Word's Table Design

**One tab of one application, and the first contextual tab authored.** Three groups and fourteen commands, in
Office's order, which is also the census's: Table Style Options, Table Styles, Borders. It is `TabTableToolsDesign` in
`TabSetTableTools`, under the *Table Tools* band while the insertion point is in a table: which parts of the table its
style sets apart, which style it wears, and the pen its borders are drawn with.

**The census's groups, read.** `GroupTableLayout` (13) is Table Style Options, `GroupTableStylesWord` (6) Table
Styles, `GroupTableBorders` (13) Borders. The ids, labels and priorities are the contextual unit's, unchanged.

**It renders in both Word hosts**, because both draw Table Tools: `Ribbons/Word` for its `TableDesign` story and
`Shell/Word` because its document's selection is in a table. Each binds thirteen commands and renders
`tableToolsMenus('word', host)`.

- **Six checkboxes**: Header Row, Total Row and Banded Rows down the first column; First Column, Last Column and
  Banded Columns down the second. Header Row, First Column and Banded Rows are ticked, Word's `w:tblLook` for an
  inserted table (`04A0`).
- **One gallery**, Table Styles: Word's 105 built-in styles by Word's own names, under *Plain Tables* (7), *Grid
  Tables* (49) and *List Tables* (49), starting on Table Grid, with Modify Table Style…, Clear and New Table Style…
  under it.
- **Two colour pickers**: Shading (*No Colour*) and Pen Colour (*Automatic*), over the document's palette.
- **Two fields**: Line Style (No Border and twenty-four styles, each value its `ST_Border` token, on Single) and Line
  Weight (¼ pt to 6 pt, on ½ pt).
- **One dropdown**, Border Styles, the first command of Borders as Microsoft 365 draws it: *Theme Borders*,
  twenty-one borders, then Border Sampler. Table Styles holds only the gallery and Shading.
- **One split button**, Borders: Office's sixteen entries, View Gridlines ticked.
- **One toggle**, Border Painter, unpressed, and **one dialog launcher**, *Borders and Shading*, on Borders.

**Written once, in `stories/ribbons/table-tools-menus.ts`, for three Table Design tabs.** What all three applications
share is shared, and Word's alone says so in its name:

- `tableStylePicture` and `tableStyleGalleryItems`: a style is a `TableStyleSpec` (name, category, the document
  colour it is tinted by, and a `TableStylePicture` look), and the picture is a small table drawn in **the document's
  palette**, which the host passes. PowerPoint's and Excel's units write their own style lists and reuse both.
- `tableLineWeights`: the nine weights, in points, which PowerPoint's Pen Weight also offers.
- `wordBorderLineStyles`, Word's Border Styles menu and Word's Borders menu are Word's.

⚠ **A picture is static markup built with `lit/static-html.js`.** `<mjx-gallery-item>` captures its children into a
fragment the gallery clones, so its art must carry no lit binding, and 105 pictures cannot be 105 hand-written
templates. Each is one markup string made static with `unsafeStatic`. **Only a colour that passes `hexColour` reaches
that string**; anything else becomes a token, so a palette value cannot inject markup. The file holds no colour
literal, so `tests/design-values.test.ts`' list of files that carry one is unchanged.

**No survivors.** The checkboxes are refused by the gate; the gallery, Shading, Border Styles, Line Style, Line Weight,
Pen Colour and Borders all open something. Border Painter passes rule 1 and fails rule 2, because unlabelled its brush
is Format Painter's.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **The counts.** Table Style Options counts 13 and draws 6; Table Styles counts 6 and draws 2; Borders counts 13 and
  draws 6 and a launcher. `GUESS:` each reading, in `dev/ribbons/census.ts`. Nothing is padded.
- **Shading and Pen Colour are the catalogue's colour picker**, where Office draws a bucket split button and a pen
  dropdown. Both carry **More Colours…** beneath the palette. `GUESS:` that Word's carry nothing else.
- **Line Style and Line Weight carry names**, where Office draws pictures of lines. `GUESS:` the style names.
- **Every gallery picture**, the footer's order, the border styles' three weights, the Borders menu's separators, the
  checkboxes' columns and View Gridlines ticked are `GUESS:`.
- **The census's spelling wins** (*Pen Colour*), except in the style names, which keep Word's *Colorful*: a style name
  is data in the document.
- **Office presses Border Painter** when a border style, line style, weight or colour is chosen. Nothing dispatches
  here, so it does not.
- **Glyphs**, all `GUESS:`. One is new: `line-style` (Border Styles). Reused: `border-all` (Borders, now at 24) and
  `paint-brush` (Border Painter, now at 24 and filled). The six checkboxes, the gallery, both colour pickers and both
  fields carry none.

### Word's Table Layout

**One tab of one application, and the second contextual tab authored.** Seven groups and thirty-three commands, in
Office's order, which is also the census's: Table, Draw, Rows & Columns, Merge, Cell Size, Alignment, Data. It is
`TabTableToolsLayout` in `TabSetTableTools`, beside Table Design under the *Table Tools* band: a table's structure,
its cells' sizes and alignment, and its data. Its label is Office's *Layout*; the accessible name *Layout, Table
Tools* tells it from the core Layout tab.

**The census's groups, read.** `GroupTable` (7) Table, `GroupTableDraw` (2) Draw, `GroupTableRowsAndColumns` (10)
Rows & Columns, `GroupTableMerge` (3) Merge, `GroupTableCellSize` (9) Cell Size, `GroupTableAlignment` (21)
Alignment, `GroupTableData` (4) Data. Every id names its group plainly, and the ids, labels and priorities are the
contextual unit's, unchanged.

**It renders in both Word hosts**, for Table Design's reason. Each binds five commands and renders the same
`tableToolsMenus('word', host)`, which now carries Table Layout's three menus too.

- **Table**: Select (a dropdown: Select Cell, Column, Row, Table), View Gridlines (a toggle, pressed) and Properties.
- **Draw**: Draw Table and Eraser, large toggles in **one exclusive set that may hold none**,
  `word.table-layout.draw.tools`, neither pressed.
- **Rows & Columns**: Delete (a large dropdown: Delete Cells…, Columns, Rows, Table), Insert Above large, then
  Insert Below, Insert Left and Insert Right; the *Insert Cells* launcher.
- **Merge**: Merge Cells, Split Cells and Split Table.
- **Cell Size**: AutoFit (a large dropdown: AutoFit Contents, AutoFit Window, Fixed Column Width), Height and Width
  (measure fields, 0.5 cm and 3.18 cm), Distribute Rows and Distribute Columns; the *Table Properties* launcher.
- **Alignment**: the nine cell alignments, icon-only toggles in **one exclusive set of exactly one**,
  `word.table-layout.alignment.cell-alignment`, Align Top Left pressed and declared down each column; then Text
  Direction and Cell Margins, large.
- **Data**: Sort large, then Repeat Header Rows (a toggle), Convert to Text and Formula.

**Written for PowerPoint's Table Layout to reuse**, in `stories/ribbons/table-tools-menus.ts`:
`tableSelectEntries(application)` and `tableDeleteEntries(application)`. Word's lists start with a cell, and
PowerPoint's (`GUESS:`) have none. AutoFit's list is Word's alone. The two exclusive sets are added to
`tests/ribbons.test.ts`' named lists of sets, starting states and sets that may hold none.

**Eleven survivors in five groups**, each passing all four demotion rules on the shape Office draws:

- **Rows & Columns**: Insert Above, Insert Below and Insert Right. Insert Left is the ceiling's cost.
- **Merge**: Merge Cells and Split Table. Split Cells opens a dialog.
- **Cell Size**: Distribute Rows and Distribute Columns.
- **Alignment**: the top row, Align Top Left, Top Centre and Top Right. Unlike a view set, one undo takes a press
  back.
- **Data**: Repeat Header Rows.
- **Table and Draw keep none.** Select opens a menu and Properties a dialog, and View Gridlines' glyph reads as Inside
  Borders (rule 2). Draw Table and Eraser arm a gesture.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **The counts.** Table counts 7 and draws 3; Rows & Columns counts 10 and draws 5 and a launcher; Cell Size counts 9
  and draws 5 and a launcher; Alignment counts 21 and draws 11. `GUESS:` each reading, in `dev/ribbons/census.ts`.
  Nothing is padded.
- **Both sets**: that Draw Table and Eraser release each other and start empty, that Align Top Left starts pressed,
  and the column order of the nine.
- **Which survivors**: which three of the four inserts, and which three of the nine alignments.
- **The census's spelling wins**: *Align Top Centre*, *Align Centre* and the rest, where Office writes *Center*.
- **Text Direction is a plain button**, Word's shape; PowerPoint's opens a list. **Height and Width's starting sizes**
  and step, **View Gridlines pressed**, AutoFit ticking none, PowerPoint's Select and Delete lists, and the two
  launchers' names are `GUESS:`.
- **Split Cells, Properties, Cell Margins, Sort, Convert to Text, Formula and both launchers open dialogs in Office**
  and open nothing here: no dialog is wired on the Table Tools tabs.
- **Glyphs**, all `GUESS:`. Twenty-four are new: `table-cursor`, `border-inside`, `table-edit`, the four
  `table-stack-*`, `table-cells-split`, `table-split`, `arrow-autofit-content`, the two `align-space-evenly-*`, the
  nine `textbox-align-*`, `padding-left` and `table-arrow-repeat-all`, `table-switch`. Reused: `table-settings`,
  `eraser`, `table-dismiss` (now at 24), `table-cells-merge`, `text-direction-rotate-90-right` (now at 24),
  `arrow-sort` and `math-formula`. **Cell Margins' `padding-left` is the weakest.** Height and Width are fields and
  carry none.

### PowerPoint's Table Design

**One tab of one application, and the third contextual tab authored**, PowerPoint's first. Four groups and nineteen
commands, in Office's order, which is also the census's: Table Style Options, Table Styles, WordArt Styles, Draw
Borders. It is `TabTableToolsDesign` in `TabSetTableTools`, under the *Table Tools* band while a table on a slide is
selected: which parts of the table its style sets apart, which style and cell effects it wears, how its text is
dressed, and the pen Draw Table draws with.

**The census's groups, read.** `GroupTableStyleOptionsPowerPoint` (6) Table Style Options,
`GroupTableStylesPowerPoint` (33) Table Styles, `GroupTextStylesTable` (33) WordArt Styles, `GroupDrawBorders` (6)
Draw Borders. The ids, labels and priorities are the contextual unit's, unchanged.

**It renders in `Ribbons/PowerPoint` alone.** `Shell/PowerPoint` draws Picture Tools, so it binds none of the tab and
renders none of its menus; see *The menu gate knows which contextual sets each host draws* above. `Ribbons/PowerPoint`
binds seventeen commands and renders `tableToolsMenus('powerpoint', 'ribbons')`.

- **Table Style Options**: six checkboxes, two columns of three read down each column. **Header Row and Banded Rows
  are ticked**, PowerPoint's `<a:tblPr firstRow="1" bandRow="1">` for an inserted table.
- **Table Styles**: the gallery in-ribbon, PowerPoint's 74 built-in styles by their own names under *Best Match for
  Document* (14), *Light* (21), *Medium* (28) and *Dark* (11), starting on Medium Style 2 - Accent 1, with Clear Table
  under it. Then Shading (a colour picker, *No Fill*, with entries), **Borders** (a small split button: twelve entries, No Border
  and All Borders first) and **Effects** (a small dropdown: Cell Bevel, Shadow and Reflection, each a submenu with
  Office's whole preset list).
- **WordArt Styles**: Quick Styles (a gallery of twenty WordArt styles, each a letter drawn in the document's
  palette, with Clear WordArt under it), Text Fill and Text Outline (colour pickers) and **Text Effects** (a small
  dropdown: Shadow, Reflection, Glow, Bevel, 3-D Rotation and Transform, each a submenu). Launcher: *Format Text
  Effects*.
- **Draw Borders**: Pen Style (No Border and PowerPoint's eight dashes, each value its `ST_PresetLineDashVal` token,
  on Solid), Pen Weight (¼ pt to 6 pt, on 1 pt) and Pen Colour (a colour picker, on Text 1) in a column, then **Draw
  Table and Eraser**, large toggles in **one exclusive set that may hold none**,
  `powerpoint.table-design.draw-borders.tools`, neither pressed. Launcher: *Format Shape*.

**Reused, and added.** `stories/ribbons/table-tools-menus.ts` gives the tab Word's table picture, gallery item builder
and nine line weights, and now carries PowerPoint's own lists beside Word's: `powerpointTableStyles`,
`powerpointPenStyles`, the Borders and Effects menus and the gallery footer. **WordArt Styles is written once for the
tabs that repeat it**: `wordArtStylesCommands(application, tab)` in `dev/ribbons/census.ts`, which Shape Format and
Chart Format will call in all three applications, and `stories/ribbons/wordart-styles-menus.ts`, which holds the
WordArt gallery, its pictures and every effect preset list. Table Design's Effects menu takes its Shadow, Reflection
and Cell Bevel lists from there. The menu itself is always written by the tab's own menus function, because the gate
reads a menu's command id only where it is spelt literally. `hexColour`, the one gate between a palette value and a
static picture, moved to `stories/ribbons/palette-art.ts`, so both galleries' pictures pass through one copy of it.
The new exclusive set is added to `tests/ribbons.test.ts`' named lists.

**No survivors.** Six checkboxes; a gallery, a colour grid, a split button and a menu; a gallery, two colour grids and
a menu; three lists and two gestures.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Shading, Text Fill, Text Outline and Pen Colour are the catalogue's colour picker**, where Office draws small
  buttons with a colour bar. Office's entries beneath each grid are the picker's slotted entries (see *The entries
  beneath a colour picker's palette* below): More Colours and Eyedropper on all four, Picture, Gradient, Texture and
  Table Background on Shading, Picture, Gradient and Texture on Text Fill, Weight, Sketched and Dashes on Text
  Outline. `GUESS:` which entries each carries and every label. **Table Background's own colour grid is not drawn**,
  only its commands: a menu holds commands, not swatches.
- **The counts.** Table Styles counts 33 and draws 4; WordArt Styles counts 33 and draws 4 and a launcher. No reading
  reaches either. Draw Borders counts 6 and draws 5 and a launcher, the reading that makes 6. Nothing is padded.
- **Best Match for Document holds the fourteen *No Style* and *Themed* styles** and Light begins at Light Style 1.
  Office may repeat styles under Best Match; a gallery listing one value twice would select two cells, so none is
  repeated. `GUESS:` that reading, every table and WordArt picture, the twenty WordArt names (the Office theme's, as
  Insert's WordArt menu names them) and both footers.
- **Every effect preset name**, that a table's Shadow list is text's, that Cell Bevel has no options entry, and that
  More Glow Colours is an entry rather than a colour grid, are `GUESS:`.
- **The checkboxes' columns** (the brief lists them across the rows), **the starting states**, both launchers and the
  Borders menu's order are `GUESS:`.
- **The census's spelling wins**: *Pen Colour*, and *colour* and *Centre* in menu and WordArt names.
- **Office presses Draw Table** when a pen style, weight or colour is chosen. Nothing dispatches here, so it does not.
- **Glyphs**, all reused and all `GUESS:`: `border-all` (Borders), `square-shadow` (Effects), `text-effects` (Text
  Effects), `table-edit` (Draw Table) and `eraser` (Eraser). No glyph is new, so the subset is unchanged. The
  checkboxes, both galleries, the four colour pickers and both fields carry none.

### PowerPoint's Table Layout

**One tab of one application, and the fourth contextual tab authored**, PowerPoint's second. Seven groups and
twenty-eight commands, in Office's order, which is also the census's: Table, Rows & Columns, Merge, Cell Size,
Alignment, Table Size, Arrange. It is `TabTableToolsLayout` in `TabSetTableTools`, beside Table Design under the
*Table Tools* band while a table on a slide is selected. A slide table is a shape on a canvas, so it has Table Size and
Arrange where Word's has Draw and Data.

**The census's groups, read.** `GroupTable` (5) Table, `GroupTableRowsAndColumns` (8) Rows & Columns, `GroupMerge` (2)
Merge, `GroupTableCellSize` (4) Cell Size, `GroupAlignment` (12) Alignment, `GroupTableSize` (3) Table Size,
`GroupArrange` (46) Arrange. The ids, labels and priorities are the contextual unit's, unchanged.

**It renders in `Ribbons/PowerPoint` alone**, for Table Design's reason: `Shell/PowerPoint` draws Picture Tools.
`Ribbons/PowerPoint` binds twelve commands, and `tableToolsMenus('powerpoint', 'ribbons')` now renders seven more
menus.

- **Table**: Select (a large dropdown: Select Table, Column, Row) and View Gridlines (a large toggle, pressed).
- **Rows & Columns**: Delete (a large dropdown: Delete Columns, Rows, Table), Insert Above large, then Insert Below,
  Insert Left and Insert Right.
- **Merge**: Merge Cells and Split Cells, large.
- **Cell Size**: Height and Width (measure fields, 1.02 cm and 5.84 cm), Distribute Rows and Distribute Columns.
- **Alignment**: Align Left, Align Centre and Align Right in **one exclusive set of exactly one**,
  `powerpoint.table-layout.alignment.horizontal`, Align Left pressed; Align Top, Centre Vertically and Align Bottom in
  another, `powerpoint.table-layout.alignment.vertical`, Align Top pressed; all six icon-only. Then **Text Direction**
  (a large dropdown: Horizontal, Rotate all text 90°, Rotate all text 270°, Stacked, More Options…) and **Cell
  Margins** (a large dropdown: Normal, None, Narrow, Wide, each with its measures as a second line, and Custom
  Margins…).
- **Table Size**: Height and Width (measure fields, 2.04 cm and 29.21 cm) and Lock Aspect Ratio (a checkbox).
- **Arrange**: Bring Forward and Send Backward (small split buttons), Selection Pane (a toggle) and Align (a dropdown:
  six alignments, two distributions, Align to Slide ticked and Align Selected Objects).

**Reused, and generalised.** `tableSelectEntries('powerpoint')` and `tableDeleteEntries('powerpoint')`, written by
Word's Table Layout for this unit, are called as they stand; the census's counts for Table (5) and Rows & Columns (8)
are met by their three-entry lists. **`arrangeCommands` in `dev/ribbons/census.ts` now takes the application, the tab
and the object** (`arrangeCommands('powerpoint', 'table-layout', 'table')`), so Picture, Shape and Chart Format can
call it: Word and Excel pass their tab and are unchanged, PowerPoint draws the layer commands small, and a table has no
Group or Rotate. `bringForwardEntries`, `sendBackwardEntries` and `alignEntries` in
`stories/ribbons/design-layout-menus.ts` are exported and take PowerPoint, whose Align aligns to the slide or to the
selected objects. Text Direction's and Cell Margins' lists and the four starting measures
(`powerpointTableLayoutMeasures`) are new in `stories/ribbons/table-tools-menus.ts`. The two exclusive sets are added
to `tests/ribbons.test.ts`' named lists and starting states.

**Nine survivors in four groups**, each passing all four demotion rules on the shape Office draws:

- **Rows & Columns**: Insert Above, Insert Below and Insert Right, Word's three. Insert Left is the ceiling's cost.
- **Merge**: Merge Cells. Split Cells opens a dialog.
- **Cell Size**: Distribute Rows and Distribute Columns.
- **Alignment**: Align Left, Align Centre and Align Right. The vertical three pass and give way to the ceiling.
- **Table, Table Size and Arrange keep none.** Select opens a menu and View Gridlines' glyph reads as Inside Borders;
  two fields and a checkbox; two split buttons, a menu and a pane.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **The counts.** Table, Rows & Columns, Merge, Cell Size and Table Size are met by a reading. **Alignment counts 12 and
  draws 8**: Cell Margins' four presets make 12, and so would Text Direction's four directions. **Arrange counts 46 and
  draws 4**, and no reading reaches it. Nothing is padded.
- **Both sets' starts** (Align Left, Align Top), and that the six, declared horizontal then vertical, read as Office's
  two rows of three.
- **Every entry name and measure** in Text Direction, Cell Margins and PowerPoint's Align; that Cell Margins ticks its
  current preset; **the four starting measures** and the 0.01 cm step; View Gridlines pressed; Lock Aspect Ratio
  unticked.
- **Sizes**: Select, View Gridlines, Merge Cells and Split Cells large, where Word's are small; Arrange small. **No
  group has a dialog launcher.**
- **Which survivors**: which three inserts, and the horizontal rather than the vertical alignments.
- **The census's spelling wins**: *Align Centre* and *Centre Vertically*, where Office writes *Center*.
- **The cell and table measures are one state in Office**; nothing dispatches, so they do not follow each other.
- **Split Cells, More Options… and Custom Margins… open dialogs in Office**, and Selection Pane a pane; they open
  nothing here.
- **Glyphs**, all reused and all `GUESS:`. `table-cursor`, `border-inside`, `table-cells-merge` and `table-cells-split`
  gain a 24px drawing, so the subset grows by five files. Alignment reuses Home's `text-align-left`, `-center` and
  `-right` and Excel's `align-top`, `align-center-vertical` and `align-bottom`. **Cell Margins' `padding-left` is still
  the weakest.** The four fields, the checkbox and Selection Pane carry none; Selection Pane because Fluent draws no
  selection pane.

### Excel's Table Design

**One tab of one application, and the fifth contextual tab authored**, Excel's first. Five groups and nineteen
commands, in Office's order, which is also the census's: Properties, Tools, External Table Data, Table Style Options,
Table Styles. It is `TabTableToolsDesignExcel` in Excel's own `TabSetTableToolsExcel`, under the *Table Tools* band
while the active cell is in a worksheet table. The set has no Layout tab, because a worksheet table's rows and columns
are the sheet's, and no borders or pens, because its lines are its cells'.

**The census's groups, read.** `GroupTableProperties` (3) Properties, `GroupTableTools` (4) Tools,
`GroupTableExternalData` (13) External Table Data, `GroupTableStyleOptions` (7) Table Style Options,
`GroupTableStylesExcel` (3) Table Styles. The ids, labels and priorities are the contextual unit's, unchanged.

**It renders in both Excel hosts**, because both draw Table Tools. Each binds eleven commands and renders
`tableToolsMenus('excel', host)`, which until this unit rendered nothing for Excel.

- **Properties**: Table Name (a combo box on *Table1*) over Resize Table.
- **Tools**: Summarize with PivotTable, Remove Duplicates and Convert to Range small, then Insert Slicer large.
- **External Table Data**: **Export** (a large dropdown: Export Table to SharePoint List…, Export Table to Visio Pivot
  Diagram…) and **Refresh** (a large split button: Refresh, Refresh All, Refresh Status, Cancel Refresh, Connection
  Properties…), then Properties, Open in Browser and Unlink small.
- **Table Style Options**: seven checkboxes in three columns, **Header Row, Banded Rows and Filter Button ticked**, the
  table Format as Table inserts (`headerRowCount="1"`, an `<autoFilter>`, `showRowStripes="1"`).
- **Table Styles**: the gallery in-ribbon, **None and Excel's 60 built-in styles** under *Light* (22), *Medium* (28) and
  *Dark* (11), starting on Table Style Medium 2, with New Table Style… and Clear under it. Each value is the style's
  wire name (`TableStyleMedium2`), each label Office's display name.

**Reused, and added.** `stories/ribbons/table-tools-menus.ts` gives the tab its table picture and gallery item builder,
and now carries Excel's lists beside Word's and PowerPoint's: `excelTableStyles`, `excelTableName`, the gallery footer,
and the Export and Refresh menus. `tableToolsMenus` now always returns menus. Where a command is one the workbook
already draws, its glyph is reused: Insert's Slicer (`filter`), Data's Refresh All (`arrow-clockwise`), Recording's
Export (`arrow-export`), Outlining's Unlink (`link-dismiss`) and Word's Table Layout Properties (`table-settings`).
Remove Duplicates carries no glyph, as Data's does.

**No survivors.** Table Name is a field and Resize Table opens a dialog. Summarize with PivotTable, Remove Duplicates
and Insert Slicer open dialogs, and Convert to Range asks first. Export is a menu, Refresh a split button, Properties
a dialog, Open in Browser leaves the workbook and Unlink cannot be undone. Seven checkboxes, and a gallery. **No
dialog launcher.**

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Table Name is a combo box with a list of one**, where Office draws a plain text box. The catalogue has no plain
  text field, so its arrow opens a list holding the name itself. **The weakest part of the tab.**
- **The gallery draws 61, not the brief's 60**: Office's Light section opens with None. `GUESS:` None's place, every
  picture, the footer's order, and that the label drops the colour word Microsoft 365 prefixes to each tooltip.
- **The counts.** Properties (3), Tools (4) and Table Style Options (7) are met. **External Table Data counts 13 and
  draws 5**; Export's two entries and Refresh's face, arrow and five entries make 13. **Table Styles counts 3 and draws
  1**; the gallery and its two footer commands make 3. `GUESS:` both readings. Nothing is padded.
- **Export's second entry** (Visio, shown where Visio is installed), **Refresh's order**, the checkboxes' columns and
  starts, and *Table1* are `GUESS:`.
- **Properties, Open in Browser, Unlink and Refresh are drawn available.** Office greys them for a table with no
  external source; nothing here tracks one.
- **Glyphs**, all `GUESS:`. Four are new: `resize-table`, `pivot` (Summarize with PivotTable), `convert-range` and
  `globe-arrow-forward` (Open in Browser), so the subset grows by four files. **`pivot` is the weakest**, and it and
  `table-settings` disagree with Insert's PivotTable and Data's Properties, which carry no glyph; those units are left
  as they are. The field, the seven checkboxes and the gallery carry none.

### Word's Picture Format

**One tab of one application, and the sixth contextual tab authored**, Word's third and the first of Picture Tools.
Six groups and twenty-five commands, in Office's order, which is also the census's: Adjust, Picture Styles,
Accessibility, Arrange, Size, Image Play. It is `TabPictureToolsFormat` in `TabSetPictureTools`, under the *Picture
Tools* band while a picture is selected: how the picture is corrected, framed, described, placed, cropped and sized.

**The census's groups, read.** `GroupPictureTools` (29) Adjust, `GroupPictureStyles` (28) Picture Styles,
`GroupAltText` (1) Accessibility, `GroupArrangeWith3DEditor` (65) Arrange, `GroupPictureSize` (20) Size,
`GroupImagePlay` (1) Image Play. The ids, labels and priorities are the contextual unit's, unchanged.

**It renders in `Ribbons/Word` alone.** `Shell/Word` draws Table Tools, so it binds none of the tab and renders none of
its menus; the menu gate's `hostContextualSets` already says so, and needed no change. `Ribbons/Word` binds twenty
commands and renders `pictureToolsMenus('word', 'ribbons')`, sixteen menus.

- **Adjust**: Remove Background (a large button), **Corrections**, **Colour**, **Artistic Effects** and
  **Transparency** (large dropdowns, each Office's whole gallery as a menu of names with the unchanged state checked),
  then Compress Pictures (a button), **Change Picture** (a dropdown of five sources) and **Reset Picture** (a split
  button: Reset Picture, Reset Picture & Size), small.
- **Picture Styles**: Quick Styles (an in-ribbon gallery of Word's twenty-eight picture styles, each a stand-in
  photograph framed and given its effect in the document's palette), Picture Border (a colour picker, *No Outline*,
  with More Outline Colours…, Weight, Sketched and Dashes beneath it), **Picture Effects** (Preset, Shadow, Reflection,
  Glow, Soft Edges, Bevel, 3-D Rotation, each a submenu) and **Picture Layout** (thirty-one SmartArt picture layouts).
  Launcher: *Format Picture*.
- **Accessibility**: Alt Text, a large toggle.
- **Arrange**: Layout's eight, from `arrangeCommands('word', 'picture-format')`, opening Layout's own seven lists.
- **Size**: **Crop**, a large **split toggle** whose arrow opens Crop, Crop to Shape (147 shapes in seven sections),
  Aspect Ratio (eleven) and Fill and Fit; then Height and Width (measure fields, 8.57 cm and 11.43 cm). Launcher:
  *Layout*.
- **Image Play**: Play Animation, a large toggle, pressed.

**Written once, and reused.**

- **`sizeCommands(application, tab, object)`** in `dev/ribbons/census.ts`: Crop for a picture, then Height and Width,
  for Shape Format's and Chart Format's Size in all three applications to call with `object: 'drawing'`.
- **`stories/ribbons/picture-tools-menus.ts`**, new: every Adjust list, the picture styles and their thumbnail
  builder, Preset and Soft Edges, Picture Layout, Crop and its two submenus, the starting measures, and
  `pictureToolsMenus(application, host)`, whose PowerPoint and Excel branches render nothing until their units.
- **Reused**: `arrangeCommands` as it stands; `design-layout-menus.ts`' Position, Wrap Text, Group and Rotate lists,
  now exported beside Bring Forward, Send Backward and Align; `wordart-styles-menus.ts`' Shadow, Reflection and Bevel
  lists, with Glow, 3-D Rotation and the accent names now exported; `colour-picker-entries.ts`' `outlineEntries`.
- **Moved**: `paletteSlotColour` and `spacingStep`, which the WordArt gallery kept privately, are in `palette-art.ts`,
  so the two galleries drawn in the document's colours read one copy.
- **The gate**: Crop joins the split-toggle rule's named list in `tests/ribbons.test.ts`.

**No survivors.** Four galleries, a menu, a split button, a view tab and a dialog; a gallery, a colour grid and two
menus; a pane; Arrange's menus and pane; a split button and two fields; and Image Play's one command, which would
leave its collapsed popup empty.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Image Play is the census's**, and the brief lists five groups. **Play Animation** — its label, glyph, size, start
  and that it is the group's command — is `GUESS:`. Office shows it only for a moving picture.
- **Remove Background's glyph, `video-background-effect`, is the weakest on the tab.**
- **Five galleries are menus of names**: Corrections, Colour, Artistic Effects, Transparency and Picture Layout, where
  Office draws the picture (or a diagram) wearing each preset. Every name, step and order, and More Variations as an
  entry rather than a colour grid, are `GUESS:`.
- **The counts.** Accessibility, Image Play and (by one reading: Crop's parts, its entries and eleven ratios, the two
  fields and the launcher) Size are met. Picture Styles' 28 is the gallery alone. **Adjust counts 29 and draws 8** and
  **Arrange 65 and draws 8**; no reading reaches either. Nothing is padded.
- **Quick Styles' thumbnails** are a stand-in photograph; every look and the order are `GUESS:`.
- **Picture Effects**: that a picture's Shadow, Reflection, Glow, Bevel and 3-D Rotation lists are text's, and every
  Preset and Soft Edges name. **Crop**: that its face draws pressed, that Crop to Shape leaves out Lines and the Text
  Box, and every shape's name. **Change Picture's** sources, **Picture Border** carrying no Eyedropper, both launchers,
  the sizes and the two starting measures.
- **The census's spelling wins**: *Colour*, *Recolour*, *Greyscale*, *Watercolour*, *Centre*.
- **Position stays small with no glyph**, where Microsoft 365 draws it large on this tab.
- **Remove Background opens the Background Removal tab in Office**, Compress Pictures and both launchers open dialogs,
  and Alt Text and Selection Pane panes; none of them opens anything here.
- **Glyphs**, all `GUESS:`. Ten are new — `video-background-effect`, `photo-filter`, `transparency-square`,
  `arrow-minimize`, `image-arrow-forward`, `image-arrow-counterclockwise`, `image-shadow`, `image-alt-text`, `crop` and
  `play` — and `brightness-high`, `color` and `diagram` are reused. The gallery, the colour picker, the two fields,
  Position and Selection Pane carry none.

### PowerPoint's Picture Format

**One tab of one application, and the seventh contextual tab authored**, PowerPoint's third and the second of Picture
Tools. Six groups and twenty-three commands, in Office's order, which is also the census's: Adjust, Picture Styles,
Accessibility, Arrange, Size, Image Play. It is `TabPictureToolsFormat` in `TabSetPictureTools`, under the *Picture
Tools* band while a picture on a slide is selected. **It is Word's tab through Word's functions**, and differs only
where Office does.

**The census's groups, read as Word's.** `GroupPictureTools` (31) Adjust, `GroupPictureStyles` (30) Picture Styles,
`GroupAltText` (1) Accessibility, `GroupArrangeWith3DEditor` (24) Arrange, `GroupPictureSize` (20) Size,
`GroupImagePlay` (1) Image Play. The ids, labels and priorities are the contextual unit's, unchanged.

**It renders in both PowerPoint hosts**, because `Shell/PowerPoint` draws Picture Tools: it is the first contextual tab
that shell shows authored rather than as a placeholder. Each host binds eighteen commands and renders
`pictureToolsMenus('powerpoint', host)`, fourteen menus. The menu gate's `hostContextualSets` already named Picture
Tools for the PowerPoint shell, and needed no change.

- **Adjust**: Word's eight, in Word's shapes and sizes, opening Word's lists.
- **Picture Styles**: Quick Styles (Word's twenty-eight, in the deck's palette); Picture Border (a colour picker,
  *No Outline*, with More Outline Colours…, **Eyedropper**, Weight, Sketched and Dashes beneath it); Picture Effects;
  **Convert to SmartArt**, PowerPoint's name for Picture Layout, over the same thirty-one layouts. Launcher: *Format
  Picture*.
- **Accessibility**: Alt Text, a large toggle.
- **Arrange**: `arrangeCommands('powerpoint', 'picture-format')`, exactly Office's six: Bring Forward and Send Backward
  (small split buttons, no text layers), Selection Pane, Align (Align to Slide ticked), Group and Rotate. **No Position
  or Wrap Text.**
- **Size**: Crop (a large split toggle, Word's list), then Height and Width (19.05 cm and 25.4 cm). Launcher: **Size
  and Position**.
- **Image Play**: Play Animation, a large toggle, pressed.

**Reused, and PowerPoint's own.**

- **Reused as they stand**: `arrangeCommands`, `sizeCommands`, every Adjust list, `pictureStyleGalleryItems` and its
  thumbnail builder, `pictureEffectsEntries`, `pictureLayoutEntries`, `cropEntries`, `outlineEntries`, and
  `design-layout-menus.ts`' PowerPoint Arrange lists. **No icon is new.**
- **PowerPoint's own, because Office's entries differ**: the Convert to SmartArt command and menu; the Eyedropper under
  Picture Border (`eyedropper: true`); `powerpointPictureMeasures`, a slide's starting size, beside
  `wordPictureMeasures`; the Size and Position launcher; `pictureToolsMenus`' PowerPoint branch, which leaves out
  Position and Wrap Text.
- **The gate**: PowerPoint's Crop joins the split-toggle rule's named list in `tests/ribbons.test.ts`, twice.

**No survivors**, for Word's reasons group by group; `dev/ribbons/census.ts` gives each.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Image Play is the census's**, and the brief lists five groups. Play Animation is `GUESS:`, as on Word's.
- **Convert to SmartArt** is the brief's label for this button; `GUESS:` that Microsoft 365's PowerPoint uses it rather
  than Picture Layout.
- **The counts.** Accessibility, Image Play and (by Word's reading) Size are met. **Picture Styles counts 30, Adjust 31
  and Arrange 24**, against 4, 8 and 6 drawn; no reading reaches them with certainty. Nothing is padded.
- **Every list is Word's** (`GUESS:` that PowerPoint's are the same), with the census's spelling (*Colour*,
  *Recolour*, *Greyscale*, *Centre*); five galleries are menus of names; the gallery is a stand-in photograph.
- **Height and Width's starting measures**, both launchers' labels, and Align to Slide ticked.
- **Remove Background opens the Background Removal tab in Office**, Compress Pictures and both launchers open dialogs
  or the Format Picture pane, and Alt Text and Selection Pane panes; none of them opens anything here.
- **Glyphs**, all reused and all `GUESS:`; Remove Background's `video-background-effect` is still the weakest. The
  gallery, the colour picker, the two fields and Selection Pane carry none.

### Excel's Picture Format

**One tab of one application, and the eighth contextual tab authored**, Excel's second and the last of Picture Tools.
Six groups and twenty-three commands, in Office's order, which is also the census's: Adjust, Picture Styles,
Accessibility, Arrange, Size, Image Play. It is `TabPictureToolsFormat` in `TabSetPictureTools`, under the *Picture
Tools* band while a picture over a worksheet is selected. **It is Word's tab through Word's functions**, and differs
only where Office does.

**The census's groups, read as Word's.** `GroupPictureTools` (30) Adjust, `GroupPictureStyles` (28) Picture Styles,
`GroupAltText` (1) Accessibility, `GroupArrangeWith3DEditor` (47) Arrange, `GroupPictureSize` (20) Size,
`GroupImagePlay` (1) Image Play. Office's five named groups map one to one onto the first five; Image Play is the
census's. The ids, labels and priorities are the contextual unit's, unchanged.

**It renders in `Ribbons/Excel` alone.** `Shell/Excel` draws Table Tools, so it binds none of the tab and renders none
of its menus; the menu gate's `hostContextualSets` already says so, and needed no change. `Ribbons/Excel` binds
eighteen commands and renders `pictureToolsMenus('excel', 'ribbons')`, fourteen menus.

- **Adjust**: Word's eight, in Word's shapes and sizes, opening Word's lists.
- **Picture Styles**: Quick Styles (Word's twenty-eight, in the workbook's palette); Picture Border (a colour picker,
  *No Outline*, with More Outline Colours…, Weight, Sketched and Dashes beneath it, **no Eyedropper**, as Word's);
  Picture Effects; **Picture Layout**, Word's name, over the same thirty-one layouts. Launcher: *Format Picture*.
- **Accessibility**: Alt Text, a large toggle.
- **Arrange**: `arrangeCommands('excel', 'picture-format')`, exactly Office's six and Page Layout's shape: **Bring
  Forward and Send Backward large** split buttons (no text layers), then Selection Pane, Align (**Snap to Grid, Snap to
  Shape, View Gridlines ticked**), Group and Rotate small. **No Position or Wrap Text.**
- **Size**: Crop (a large split toggle, Word's list), then Height and Width (9.53 cm and 12.7 cm). Launcher: **Size and
  Properties**.
- **Image Play**: Play Animation, a large toggle, pressed.

**Reused, and Excel's own.**

- **Reused as they stand**: `arrangeCommands`, `sizeCommands`, every Adjust list, `pictureStyleGalleryItems` and its
  thumbnail builder, `pictureEffectsEntries`, `pictureLayoutEntries`, `cropEntries`, `outlineEntries` with Word's
  options, and `design-layout-menus.ts`' Excel Arrange lists. **No icon is new.**
- **Excel's own, because Office's entries differ**: `excelPictureMeasures`, a picture at its own size, beside Word's
  and PowerPoint's; the Size and Properties launcher; `pictureToolsMenus`' Excel branch, which leaves out Position and
  Wrap Text. The large layer commands and the snapping Align were already Excel's in `arrangeCommands` and
  `alignEntries`, and needed nothing new.
- **The gate**: Excel's Crop joins the split-toggle rule's named list in `tests/ribbons.test.ts`, once.

**No survivors**, for Word's reasons group by group; `dev/ribbons/census.ts` gives each.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Image Play is the census's**, and the brief lists five groups. Play Animation is `GUESS:`, as on Word's.
- **Picture Border carries no Eyedropper.** `GUESS:`: recent Microsoft 365 builds may carry one in Excel. The census's
  Picture Styles count, 28, equals Word's and is two short of PowerPoint's, which is indirect support. **The weakest
  call on the tab.**
- **The counts.** Accessibility, Image Play and (by Word's reading) Size are met, and Picture Styles' 28 is Word's
  gallery reading again. **Adjust counts 30 and Arrange 47**, against 8 and 6 drawn; no reading reaches either (47 is
  Page Layout's Arrange count). Nothing is padded.
- **Every list is Word's** (`GUESS:` that Excel's are the same), with the census's spelling (*Colour*, *Recolour*,
  *Greyscale*, *Centre*); five galleries are menus of names; the gallery is a stand-in photograph.
- **Height and Width's starting measures**, both launchers' labels, and View Gridlines ticked under Align.
- **Remove Background opens the Background Removal tab in Office**, Compress Pictures and both launchers open dialogs
  or the Format Picture pane, and Alt Text and Selection Pane panes; none of them opens anything here.
- **Glyphs**, all reused and all `GUESS:`; Remove Background's `video-background-effect` is still the weakest. The
  gallery, the colour picker, the two fields and Selection Pane carry none.

### PowerPoint's Shape Format

**One tab of one application, and the ninth contextual tab authored**, PowerPoint's fourth and the first of Drawing
Tools. Six groups and twenty-one commands, in Office's order, which is also the census's: Insert Shapes, Shape Styles,
WordArt Styles, Accessibility, Arrange, Size. It is `TabDrawingToolsFormat` in `TabSetDrawingTools`, under the
*Drawing Tools* band while a shape, a text box or a WordArt on a slide is selected.

**The census's groups, read.** `GroupShapes` (13) Insert Shapes, `GroupShapeStyles` (40) Shape Styles,
`GroupWordArtStyles` (33) WordArt Styles, `GroupAltText` (1) Accessibility, `GroupArrangeWith3DEditor` (24) Arrange,
`GroupSize` (3) Size. Office's six groups map one to one onto them. The ids, labels and priorities are the contextual
unit's, unchanged.

**It renders in `Ribbons/PowerPoint` alone.** `Shell/PowerPoint` draws Picture Tools, so it binds none of the tab and
renders none of its menus; the menu gate's `hostContextualSets` already says so, and needed no change.
`Ribbons/PowerPoint` binds eighteen commands and renders `drawingToolsMenus('powerpoint', 'ribbons')`, eleven menus.

- **Insert Shapes**: **Shapes** (a large dropdown over the whole shape gallery, Action Buttons included), then **Edit
  Shape** (Change Shape ▸, Edit Points, Reroute Connectors unavailable), **Text Box** (a plain button) and **Merge
  Shapes** (Union, Combine, Fragment, Intersect, Subtract), small.
- **Shape Styles**: **Theme Styles** (a gallery of forty-nine shape styles, six Theme Styles rows and one Presets row,
  each an *Abc* box in the deck's palette, with **Other Theme Fills** in its footer opening Style 1 to Style 12);
  **Shape Fill** (a colour picker on Accent 1, *No Fill*, with More Fill Colours…, Eyedropper, Picture…, Gradient and
  Texture beneath); **Shape Outline** (on Accent 1, Darker 50%, *No Outline*, with More Outline Colours…, Eyedropper,
  Weight, Sketched, Dashes and **Arrows**); **Shape Effects** (Picture Effects' seven submenus). Launcher: *Format
  Shape*.
- **WordArt Styles**: Table Design's group under this tab's ids; Text Fill starts on Background 1. Launcher: *Format Text
  Effects*.
- **Accessibility**: Alt Text, a large toggle.
- **Arrange**: `arrangeCommands('powerpoint', 'shape-format')`, Picture Format's six, Align to Slide ticked.
- **Size**: Height and Width (2.54 cm each), no Crop. Launcher: *Size and Position*.

**Written once, reused, and new shared code.**

- **New, in `stories/ribbons/drawing-tools-menus.ts`, shaped for Word's and Excel's Shape Format**: the shape gallery
  (`shapeGallerySections`, `insertShapesEntries`, `changeShapeEntries`, `editShapeEntries`), `mergeShapesEntries`,
  `shapeStyles` with `shapeStylePicture` and `shapeStyleGalleryItems`, `otherThemeFillEntries`, `shapeEffectsEntries`,
  `shapeFillEntryOptions` and `shapeOutlineEntryOptions` (Eyedropper for PowerPoint only), `powerpointShapeMeasures`,
  and `drawingToolsMenus`, whose Word and Excel branches render nothing yet.
- **New, in `dev/ribbons/census.ts`**: `insertShapesCommands(application, tab)` (Merge Shapes for PowerPoint only) and
  `shapeStylesCommands(application, tab)`, for Word's and Excel's Shape Format and Chart Format.
- **Reused as they stand**: `wordArtStylesCommands` and every list in `wordart-styles-menus.ts`; `arrangeCommands` and
  `design-layout-menus.ts`' PowerPoint Arrange lists; `sizeCommands(…, 'drawing')`; `fillEntries` and
  `outlineEntries`; `pictureEffectsEntries` (Shape Effects) and `cropShapes` (Change Shape, and the body of the Shapes
  gallery) from `picture-tools-menus.ts`; `accentNames`.
- **Changed outside the tab**: **Insert's three Shapes menus now open the whole gallery** through
  `insertShapesEntries`, rather than nine sampled shapes. Their commands, ids and bindings are unchanged.
- **Two new glyphs**: `bezier-curve-square` (Edit Shape) and `shape-union` (Merge Shapes).

**No survivors.** A gallery, two menus and a drawing gesture; a gallery, two colour grids and a menu, twice; a pane; two
split buttons, three menus and a pane; two fields.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Shapes is a large dropdown over names**, where Office draws an in-ribbon gallery of outlines.
- **Theme Styles**: the label (Office's collapsed button says *Quick Styles*), all forty-nine names and looks, that
  Presets is one row, and Other Theme Fills' twelve. Other Theme Fills' menu id is the gallery command's.
- **Insert's Shapes menus changed**, against Decision 3's *menus stay shallow*, so that one Office gallery is written
  once; the brief said to reuse Insert's list, which was a sample. No *Recently Used Shapes* section.
- **Merge Shapes is PowerPoint's alone** (the census counts PowerPoint's Insert Shapes 13 and Word's and Excel's 12).
- **Change Shape leaves out Lines and the text boxes**, and Reroute Connectors is drawn unavailable.
- **Text Box has no Horizontal/Vertical arrow.**
- **Shape Fill's and Shape Outline's entries and starts**, Text Fill on Background 1, that Shape Effects' lists are a
  picture's, Height and Width's 2.54 cm, all three launchers' labels, and Align to Slide ticked.
- **The counts.** Accessibility and Size are met. Insert Shapes counts 13 and draws 4, Shape Styles 40 and 4, WordArt
  Styles 33 and 4, Arrange 24 and 6; no reading reaches any of them. Nothing is padded.
- **Glyphs**, all `GUESS:`. **Alt Text's `image-alt-text`, a picture with a label on a shape's tab, is the weakest.**
  The two galleries, the four colour pickers, the two fields and Selection Pane carry none.

### Word's Shape Format

**One tab of one application, and the tenth contextual tab authored**, Word's fourth and Word's first of Drawing
Tools. Seven groups and twenty-five commands, in Office's order, which is also the census's: Insert Shapes, Shape
Styles, WordArt Styles, Text, Accessibility, Arrange, Size. It is `TabDrawingToolsFormat` in `TabSetDrawingTools`,
under the *Drawing Tools* band while a shape, a text box or a WordArt is selected. **It is PowerPoint's Shape Format
wherever Office's Word is**, and every difference below is one Office's Word makes.

**The census's groups, read.** `GroupShapes` (12) Insert Shapes, `GroupShapeStyles` (37) Shape Styles,
`GroupWordArtStyles` (30) WordArt Styles, `GroupTextbox` (5) Text, `GroupAltText` (1) Accessibility,
`GroupArrangeWith3DEditor` (65) Arrange, `GroupSize` (3) Size. Office's seven groups map one to one onto them. The ids,
labels and priorities are the contextual unit's, unchanged.

**It renders in `Ribbons/Word` alone.** `Shell/Word` draws Table Tools, so it binds none of the tab and renders none
of its menus; the menu gate's `hostContextualSets` already says so, and needed no change. `Ribbons/Word` binds
twenty-two commands and renders `drawingToolsMenus('word', 'ribbons')`, fifteen menus.

- **Insert Shapes**: **Shapes** (large, the whole gallery with New Drawing Canvas and no Action Buttons), **Edit Shape**
  (Change Shape ▸, Edit Points, Reroute Connectors unavailable) and **Draw Text Box**, a small split button whose arrow
  opens Draw Text Box and Draw Vertical Text Box. No Merge Shapes.
- **Shape Styles**: PowerPoint's Theme Styles gallery with Other Theme Fills; **Shape Fill** (Accent 1, *No Fill*, More
  Fill Colours…, Picture…, Gradient, Texture) and **Shape Outline** (Accent 1, Darker 50%, *No Outline*, More Outline
  Colours…, Weight, Sketched, Dashes, Arrows), **no Eyedropper**; **Shape Effects**. Launcher: *Format Shape*.
- **WordArt Styles**: Quick Styles; **Text Fill** on Background 1 with More Fill Colours… and Gradient; **Text Outline**
  with More Outline Colours…, Weight and Dashes; Text Effects. Launcher: *Format Text Effects*.
- **Text**, Word's alone: **Text Direction** (Horizontal ticked, Rotate all text 90°, Rotate all text 270°, Text
  Direction Options…; no Stacked), **Align Text** (Top ticked, Middle, Bottom) and **Create Link**, a plain button.
- **Accessibility**: Alt Text, a large toggle.
- **Arrange**: `arrangeCommands('word', 'shape-format')`, Picture Format's eight, Align to Margin ticked.
- **Size**: Height and Width (2.54 cm each). Launcher: *Layout*.

**Reused, and Word's own.**

- **Reused as they stand**: `insertShapesCommands`, `shapeStylesCommands`, `wordArtStylesCommands`, `arrangeCommands`
  and `sizeCommands(…, 'drawing')` in the census; in `stories/ribbons/drawing-tools-menus.ts` the shape gallery, Edit
  Shape, the shape styles and their pictures, Other Theme Fills, `shapeEffectsEntries`, `shapeFillEntryOptions('word')`
  and `shapeOutlineEntryOptions('word')`; `wordart-styles-menus.ts`' gallery and Text Effects lists; `fillEntries` and
  `outlineEntries`; `design-layout-menus.ts`' seven Word Arrange lists; Picture Format's Alt Text. **No new glyph.**
- **Word's own**: `insertShapesCommands` now gives Word **Draw Text Box** (`draw-text-box`) where the others keep Text
  Box; `wordShapeFormatText` in the census; `drawTextBoxEntries`, `wordTextDirectionEntries`, `alignTextEntries`,
  `wordShapeMeasures` and the `word` branch of `drawingToolsMenus` in `drawing-tools-menus.ts`; the shorter Text Fill
  and Text Outline entries, written in the binding.

**No survivors.** A gallery, a menu and a split button; a gallery, two colour grids and a menu, twice; two menus and a
link gesture; a pane; six menus or split buttons and a pane; two fields.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **The Text group**: every label in both menus, that Text Direction has no Stacked, both starts, and that Create Link
  is one button reading *Break Link* on a linked box (only the unlinked state is drawn).
- **Draw Text Box**: the label, the split shape and both entries; that Word has no Merge Shapes.
- **Text Fill and Text Outline's shorter entries**, no Eyedropper anywhere, all four pickers' starts, Height and
  Width's 2.54 cm, and the three launchers (*Layout* on Size).
- **The counts.** Accessibility and Size are met. Insert Shapes counts 12 and draws 3; one reading reaches 12 (the
  gallery's four parts, Edit Shape and its three entries, Draw Text Box's face, arrow and two entries). Text counts 5 and
  draws 3, and no reading reaches 5. Shape Styles 37 and 4, WordArt Styles 30 and 4, Arrange 65 and 8. Nothing is
  padded.
- **Glyphs**, all reused and all `GUESS:`. **Create Link's `link`, a chain that reads *hyperlink* first, is the
  weakest.** The two galleries, the four colour pickers, the two fields, Position and Selection Pane carry none.
- PowerPoint's Shape Format's readings of Shapes, Theme Styles, Other Theme Fills, Edit Shape and Shape Effects hold
  here unchanged.

### Excel's Shape Format

**One tab of one application, and the eleventh contextual tab authored**, Excel's third and the last of the three
Shape Format tabs. Six groups and twenty commands, in Office's order, which is also the census's: Insert Shapes, Shape
Styles, WordArt Styles, Accessibility, Arrange, Size. It is `TabDrawingToolsFormat` in `TabSetDrawingTools`, under the
*Drawing Tools* band while a shape, a text box or a WordArt over a worksheet is selected. **It is PowerPoint's Shape
Format wherever Office's Excel is**, and every difference below is one Office's Excel makes.

**The census's groups, read.** `GroupShapes` (12) Insert Shapes, `GroupShapeStyles` (37) Shape Styles,
`GroupWordArtStyles` (30) WordArt Styles, `GroupAltText` (1) Accessibility, `GroupArrangeWith3DEditor` (47) Arrange,
`GroupSize` (3) Size. The brief's six groups map one to one onto them. The ids, labels and priorities are the
contextual unit's, unchanged.

**It renders in `Ribbons/Excel` alone.** `Shell/Excel` draws Table Tools, so it binds none of the tab and renders none
of its menus; the menu gate's `hostContextualSets` already says so, and needed no change. `Ribbons/Excel` binds
eighteen commands and renders `drawingToolsMenus('excel', 'ribbons')`, eleven menus.

- **Insert Shapes**: **Shapes** (large, the whole gallery with no Action Buttons and no New Drawing Canvas), **Edit
  Shape** (Change Shape ▸, Edit Points, Reroute Connectors unavailable) and **Text Box**, a small split button whose
  arrow opens Draw Horizontal Text Box and Vertical Text Box, as Excel's Insert tab does. No Merge Shapes.
- **Shape Styles**: PowerPoint's Theme Styles gallery with Other Theme Fills; **Shape Fill** (Accent 1, *No Fill*, More
  Fill Colours…, Picture…, Gradient, Texture) and **Shape Outline** (Accent 1, Darker 50%, *No Outline*, More Outline
  Colours…, Weight, Sketched, Dashes, Arrows), **no Eyedropper**; **Shape Effects**. Launcher: *Format Shape*.
- **WordArt Styles**: Quick Styles; **Text Fill** on Background 1 with More Fill Colours…, Picture…, Gradient and
  Texture; **Text Outline** with More Outline Colours…, Weight, Sketched and Dashes; Text Effects. Launcher: *Format
  Text Effects*.
- **Accessibility**: Alt Text, a large toggle.
- **Arrange**: `arrangeCommands('excel', 'shape-format')`, Picture Format's six: **Bring Forward and Send Backward
  large**, then Selection Pane, Align (**Snap to Grid, Snap to Shape, View Gridlines ticked**), Group and Rotate small.
- **Size**: Height and Width (2.54 cm each). Launcher: **Size and Properties**.

**Reused, and Excel's own.**

- **Reused**: `insertShapesCommands` (which now names Excel's Text Box in its own branch), `shapeStylesCommands`,
  `wordArtStylesCommands`, `arrangeCommands` and `sizeCommands(…, 'drawing')` in the census; in `stories/ribbons/drawing-tools-menus.ts` the
  shape gallery, Edit Shape, the shape styles and their pictures, Other Theme Fills, `shapeEffectsEntries`,
  `shapeFillEntryOptions('excel')` and `shapeOutlineEntryOptions('excel')`; `wordart-styles-menus.ts`' gallery and Text
  Effects lists; `fillEntries` and `outlineEntries`; `design-layout-menus.ts`' five Excel Arrange lists; Picture
  Format's Alt Text. **No new glyph.**
- **Excel's own**: `excelShapeFormatAccessibility` in the census (a list only because an id carries its application);
  `excelTextBoxEntries` (Word's `drawTextBoxEntries` carries different labels), `excelShapeMeasures` and the `excel`
  branch of `drawingToolsMenus` in `drawing-tools-menus.ts`; Text Fill's and Text
  Outline's entries, PowerPoint's less the Eyedropper, written in the binding; the *Size and Properties* launcher.

**No survivors.** A gallery, a menu and a split button; a gallery, two colour grids and a menu, twice; a pane; two
split buttons, three menus and a pane; two fields.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Text Fill and Text Outline carry PowerPoint's entries less the Eyedropper**, on the reading that a shape's text in a
  workbook is DrawingML text, as a slide's is; Sketched is the least certain entry.
- **No Eyedropper anywhere**; that Excel has no Merge Shapes; all four pickers' starts; Height and Width's 2.54 cm; the
  three launchers (*Size and Properties* on Size); View Gridlines ticked under Align.
- **The counts.** Accessibility, Size and Insert Shapes are met: Insert Shapes draws 3, and the gallery's four parts,
  Edit Shape and its three entries, and Text Box's face, arrow and two entries make 12. Shape Styles 37 and 4, WordArt
  Styles 30 and 4, Arrange 47 and 6. Nothing is padded.
- **Glyphs**, all reused and all `GUESS:`. **Alt Text's `image-alt-text`, a picture with a label on a shape's tab, is
  the weakest.** The two galleries, the four colour pickers, the two fields and Selection Pane carry none.
- PowerPoint's Shape Format's readings of Shapes, Theme Styles, Other Theme Fills, Edit Shape and Shape Effects hold
  here unchanged.

### Word's Chart Design

**One tab of one application, and the twelfth contextual tab authored**, Word's fifth and the first of Chart Tools.
Four groups and nine commands, in Office's order, which is also the census's: Chart Layouts, Chart Styles, Data, Type.
It is `TabChartToolsDesignNew` in `TabSetChartTools`, under the *Chart Tools* band while a chart in the document is
selected. **Every group is written once as a function of the application and the tab**, for PowerPoint's and Excel's
Chart Design to call.

**The census's groups, read.** `GroupChartLayouts` (23) Chart Layouts, `GroupChartStyles` (2) Chart Styles,
`GroupChartData` (6) Data, `GroupChartType` (1) Type. The brief's four groups map one to one onto them. The ids,
labels and priorities are the contextual unit's, unchanged. No group has a dialog launcher.

**It renders in `Ribbons/Word` alone.** `Shell/Word` draws Table Tools, so it binds none of the tab and renders none
of its menus; the menu gate's `hostContextualSets` already says so, and needed no change. `Ribbons/Word` binds six
commands and renders `chartToolsMenus('word', 'ribbons')`, five menus.

- **Chart Layouts**: **Add Chart Element** (large, eleven submenus with every entry: Axes, Axis Titles, Chart Title,
  Data Labels, Data Table, Error Bars, Gridlines, Legend, Lines, Trendline, Up/Down Bars, each ending on its *More …
  Options…*) and **Quick Layout** (large, Layout 1 to Layout 11).
- **Chart Styles**: **Change Colours** (large, *Colourful* Palettes 1–4 and *Monochromatic* Palettes 1–13, one radio
  set) and the **Chart Styles** gallery in-ribbon, Style 1 to Style 16 in the document's palette, starting on Style 1.
- **Data**: **Switch Row/Column**, **Select Data** and **Refresh Data**, plain large buttons, and **Edit Data**, a large
  split button whose arrow opens Edit Data and Edit Data in Excel.
- **Type**: **Change Chart Type**, a large dropdown over Excel's Insert → Charts families.

**Written once, and reused.**

- **In the census**: `chartLayoutsCommands`, `chartStylesCommands`, `chartDataCommands` (which gives Excel Switch
  Row/Column and Select Data alone, as its census count of 2 says) and `chartTypeCommands`.
- **In `stories/ribbons/chart-tools-menus.ts`**, every list a function of nothing: `chartElements` and
  `addChartElementEntries`, `quickLayoutEntries`, `changeColoursEntries`, `chartStyles` and `chartStyleGalleryItems`,
  `editDataEntries`, `changeChartTypeEntries`; `chartToolsMenus(application, host)` had a `word` branch and rendered
  nothing for the other two until their units (both have since landed; see *PowerPoint's Chart Design* and *Excel's
  Chart Design*).
- **Reused**: Excel's Insert → Charts lists, now exported from `insert-menus.ts`; `wordart-styles-menus.ts`' `submenu`;
  `palette-art.ts`' `paletteSlotColour` and `spacingStep`. **One new glyph**, `data-bar-vertical-add`; `table-switch`
  and `chart-multiple` gain a 24.

**No survivors.** Two menus; a menu and a gallery; a swap with no self-evident glyph, a dialog, a split button and a
refresh no undo takes back; a menu.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Change Chart Type opens a menu of families**, where Office opens a dialog, and has **no Map family**.
- **Lines and Up/Down Bars open**, where Office greys both on a column chart, so their entries can be judged.
- **Every Add Chart Element entry, start and options entry**; that Axes, Axis Titles and Gridlines are checkboxes and
  Trendline plain.
- **Quick Layout's eleven** and **Change Colours' 4 and 13**, both drawn as names where Office draws pictures.
- **The Chart Styles gallery's sixteen pictures**, a description of each look rather than a render of Office's.
- **Switch Row/Column and Refresh Data are drawn available**, where Office greys them until the data sheet is open or
  the chart is linked.
- **The counts.** Chart Styles and Type are met. Chart Layouts counts 23 and draws 2; Add Chart Element, its eleven
  submenus and Quick Layout's eleven layouts make 23. Data counts 6 and draws 4; the three buttons, Edit Data's split
  and its two entries make 6. Nothing is padded.
- **Spelling**: *Change Colours*, *Colourful*, *Centred Overlay*, where the brief and Office's US build write *Colors*.
- **Glyphs**, all `GUESS:`. **Quick Layout's `layout-cell-four`, four tiled regions that read *arrange windows* first,
  is the weakest.** The gallery carries none.

### PowerPoint's Chart Design

**One tab of one application, and the thirteenth contextual tab authored**, PowerPoint's fifth and its first of Chart
Tools. Four groups and nine commands, in Office's order, which is also the census's: Chart Layouts, Chart Styles, Data,
Type. It is `TabChartToolsDesignNew` in `TabSetChartTools`, under the *Chart Tools* band while a chart on a slide is
selected. **It is Word's Chart Design under PowerPoint's ids**, because Office's two tabs do not differ.

**The census's groups, read.** `GroupChartLayouts` (23) Chart Layouts, `GroupChartStyles` (2) Chart Styles,
`GroupChartData` (6) Data, `GroupChartType` (1) Type: every id, label, count and priority Word's own. The brief's four
groups map one to one onto them. No group has a dialog launcher.

**It renders in `Ribbons/PowerPoint` alone.** `Shell/PowerPoint` draws Picture Tools, so it binds none of the tab and
renders none of its menus; the menu gate's `hostContextualSets` already says so, and needed no change.
`Ribbons/PowerPoint` binds six commands and renders `chartToolsMenus('powerpoint', 'ribbons')`, five menus.

- **Chart Layouts**: **Add Chart Element** (large, eleven submenus) and **Quick Layout** (large, Layout 1 to 11).
- **Chart Styles**: **Change Colours** (large) and the **Chart Styles** gallery in-ribbon, Style 1 to Style 16.
- **Data**: **Switch Row/Column**, **Select Data** and **Refresh Data**, plain large buttons, and **Edit Data**, a large
  split button whose arrow opens Edit Data and Edit Data in Excel.
- **Type**: **Change Chart Type**, a large dropdown over Excel's Insert → Charts families.

**Reused, and PowerPoint's own.**

- **Reused, all of it**: `chartLayoutsCommands`, `chartStylesCommands`, `chartDataCommands` and `chartTypeCommands`
  with `'powerpoint'` in the census; in `stories/ribbons/chart-tools-menus.ts`, `addChartElementEntries`,
  `quickLayoutEntries`, `changeColoursEntries`, `chartStyleGalleryItems`, `editDataEntries` and
  `changeChartTypeEntries`. **No new glyph.**
- **PowerPoint's own**: the `powerpoint` branch of `chartToolsMenus` (five `commandMenu` calls, because an id carries its
  application), `powerpointChartDesignTab`, and the six bindings. **No list is PowerPoint's own**: the Data group and
  Edit Data's entries, the likeliest places for Office to differ, were checked and read as Word's.

**No survivors**, by Word's readings: two menus; a menu and a gallery; a swap with no self-evident glyph, a dialog, a
split button and a refresh no undo takes back; a menu.

⚠ **What is not Office's shape, or is `GUESS:`.** Word's Chart Design's list holds here unchanged. Beyond it:

- **Data is Word's four**, which `chartDataCommands` guessed before this unit; the census's 6, Word's count, supports it.
- **Edit Data's two entries are Word's**; the data sheet opens over the slide.
- **The chart PowerPoint inserts starts as Word's does** (title Above Chart, legend Bottom, both axes, major horizontal
  gridlines), so Add Chart Element's starts are Word's.
- **The gallery's pictures read the catalogue's one specimen theme**, so they match `Ribbons/Word`'s exactly; that is
  not a claim that a deck and a document share a theme.
- **Glyphs**, all Word's and all `GUESS:`; Quick Layout's `layout-cell-four` is again the weakest.

### Excel's Chart Design

**One tab of one application, and the fourteenth contextual tab authored**, Excel's fourth and its first of Chart
Tools, and the last Chart Design of the three. Five groups and eight commands, in Office's order, which is also the
census's: Chart Layouts, Chart Styles, Data, Type, Location. It is `TabChartToolsDesignNew` in `TabSetChartTools`,
under the *Chart Tools* band while a chart on a worksheet or a chart sheet is selected. **It is Word's Chart Design
wherever Office's Excel is**, and the two differences below are both Office's Excel's.

**The census's groups, read.** `GroupChartLayouts` (23, `primary`) Chart Layouts, `GroupChartStyles` (3, `primary`)
Chart Styles, `GroupChartData` (2, `secondary`) Data, `GroupChartType` (1, `secondary`) Type, and
**`GroupChartLocation` (1, `ancillary`) Location**, the fifth group, which is **Move Chart**: the brief's reading holds.
The ids, labels, counts and priorities are the contextual unit's, unchanged. No group has a dialog launcher.

**It renders in `Ribbons/Excel` alone.** `Shell/Excel` draws Table Tools, so it binds none of the tab and renders none
of its menus; the menu gate's `hostContextualSets` already says so, and needed no change. `Ribbons/Excel` binds five
commands and renders `chartToolsMenus('excel', 'ribbons')`, four menus.

- **Chart Layouts**: **Add Chart Element** (large, eleven submenus) and **Quick Layout** (large, Layout 1 to 11).
- **Chart Styles**: **Change Colours** (large) and the **Chart Styles** gallery in-ribbon, Style 1 to Style 16.
- **Data**: **Switch Row/Column** and **Select Data**, plain large buttons. No Edit Data, no Refresh Data.
- **Type**: **Change Chart Type**, a large dropdown over this ribbon's own Insert → Charts families.
- **Location**: **Move Chart**, a plain large button; Office's opens the Move Chart dialog.

**Reused, and Excel's own.**

- **Reused**: `chartLayoutsCommands`, `chartStylesCommands`, `chartDataCommands` (whose `excel` branch Word's unit wrote)
  and `chartTypeCommands` with `'excel'` in the census; in `stories/ribbons/chart-tools-menus.ts`,
  `addChartElementEntries`, `quickLayoutEntries`, `changeColoursEntries`, `chartStyleGalleryItems` and
  `changeChartTypeEntries`. `arrow-move`, PowerPoint's Move Split, for Move Chart.
- **Excel's own, each where Office's Excel differs**: `excelChartDesignLocation` in the census (the Location group, which
  neither other application has); the `excel` branch of `chartToolsMenus`, four menus, because there is no Edit Data;
  `excelChartDesignTab`; the five bindings. **No new list and no new glyph**; `arrow-move` gains a 24.

**No survivors**: two menus; a menu and a gallery; a swap with no self-evident glyph and a dialog; a menu; a dialog.

⚠ **What is not Office's shape, or is `GUESS:`.** Word's Chart Design's list holds here, less Edit Data and Refresh
Data. Beyond it:

- **Chart Styles is `primary` and Data `secondary`**, where Word's and PowerPoint's are `secondary` and `standard`: the
  census counts Chart Styles 3 and Data 2 here. The census wins.
- **Chart Styles counts 3 and draws 2.** `GUESS:` that the third is the gallery's collapsed *Quick Styles* form. Data,
  Type and Location are met; Chart Layouts is Word's 23 and 2. Nothing is padded.
- **Change Chart Type has no Map family**, though Excel's dialog lists Map; Insert's Maps reaches it.
- **The chart Excel inserts starts as Word's does**, so Add Chart Element's starts are Word's.
- **Switch Row/Column is drawn available**, which is Office's Excel's shape too (Word greys it until the data sheet is
  open).
- **Glyphs**, all `GUESS:`. **Move Chart's `arrow-move`, four arrows that say *move* and not *chart*, is the weakest**;
  Fluent's one chart with an arrow, `data-bar-vertical-arrow-down`, reads *download*. The gallery carries none.

### Word's Chart Format

**One tab of one application, and the fifteenth contextual tab authored**, Word's sixth and last, and the first Chart
Format. Seven groups and twenty-five commands, in Office's order, which is also the census's: Current Selection,
Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange, Size. It is `TabChartToolsFormatNew` in
`TabSetChartTools`, under the *Chart Tools* band while a chart in the document is selected. **It is Word's Shape
Format wherever a chart behaves as a shape**, with a chart's own Current Selection in front.

**The census's groups, read.** `GroupChartCurrentSelection` (3) Current Selection, `GroupShapesChart` (2) Insert Shapes,
`GroupChartShapeStyles` (35) Shape Styles, `GroupWordArtStyles` (30) WordArt Styles, `GroupAltText` (1) Accessibility,
`GroupArrange` (65) Arrange, `GroupSize` (3) Size. The brief's seven groups map one to one onto them. The ids, labels
and priorities are the contextual unit's, unchanged.

**It renders in `Ribbons/Word` alone.** `Shell/Word` draws Table Tools, so it binds none of the tab and renders none
of its menus; the menu gate's `hostContextualSets` already says so, and needed no change. `Ribbons/Word` binds
twenty-one commands and renders the tab's twelve menus through `chartToolsMenus('word', 'ribbons')`.

- **Current Selection**: **Chart Elements**, a field over the ten parts of the chart Word inserts, starting on Chart
  Area; **Format Selection** and **Reset to Match Style**, plain small buttons.
- **Insert Shapes**: **Shapes** (large, the shape gallery with no New Drawing Canvas and no Action Buttons) and
  **Change Shape** (small, Change Shape's list, drawn available).
- **Shape Styles**: Shape Format's Theme Styles gallery with Other Theme Fills; **Shape Fill** (Background 1, *No Fill*,
  More Fill Colours…, Picture…, Gradient, Texture) and **Shape Outline** (Text 1, Lighter 80%, *No Outline*, More
  Outline Colours…, Weight, Dashes), **no Eyedropper, Sketched or Arrows**; **Shape Effects**. Launcher: *Format Shape*.
- **WordArt Styles**: Quick Styles; **Text Fill** on Text 1, Lighter 40% with More Fill Colours… and Gradient; **Text
  Outline** with More Outline Colours…, Weight and Dashes; Text Effects. Launcher: *Format Text Effects*.
- **Accessibility**: Alt Text, a large toggle.
- **Arrange**: `arrangeCommands('word', 'chart-format')`, Word's Picture Format's eight, Align to Margin ticked.
- **Size**: Height 8.89 cm and Width 15.24 cm. Launcher: *Layout*.

**Reused, and new shared code.**

- **Reused as they stand**: `shapeStylesCommands`, `wordArtStylesCommands`, `arrangeCommands` and
  `sizeCommands(…, 'drawing')` in the census; in `stories/ribbons/drawing-tools-menus.ts` the shape styles and their
  pictures, Other Theme Fills, `shapeEffectsEntries`, `shapeFillEntryOptions`, `shapeGallerySections` and (now exported)
  `shapeSectionEntries`; `wordart-styles-menus.ts`' gallery and Text Effects; `design-layout-menus.ts`' seven Word
  Arrange lists; `fillEntries` and `outlineEntries`; every Shape Format's Alt Text glyph.
- **New, written for PowerPoint's and Excel's Chart Format**: in the census, `insertShapesCommands(…, 'chart')` (Shapes
  and Change Shape), `chartCurrentSelectionCommands` and `chartFormatAccessibilityCommands`; in
  `stories/ribbons/chart-tools-menus.ts`, `chartSelectionElements` and `chartSelectionOptions`,
  `chartShapeGallerySections`, `chartShapesEntries` and `chartChangeShapeEntries`, `chartOutlineEntryOptions`, and
  `chartTextFillEntryOptions` and `chartTextOutlineEntryOptions` (with each application's branch).
- **Word's own**: `wordChartMeasures`, `wordChartFormatMenus` in `chartToolsMenus`' `word` branch,
  `wordChartFormatTab`, and the bindings. **One new glyph**, `data-bar-vertical-edit`.

**No survivors.** A field, a pane and a reset whose glyph is PowerPoint's Reset; a gallery and a menu; a gallery, two
colour grids and a menu, twice; a pane; six menus or split buttons and a pane; two fields.

⚠ **What is not Office's shape, or is `GUESS:`.**

- **Chart Elements' order is Office's alphabetical one**, the three series before the vertical axis, where the brief
  lists each series last; every label, and that no data point is listed.
- **Shape Outline drops Sketched and Arrows**, on the census's Shape Styles 35 against Shape Format's 37 (38 against 40
  in PowerPoint).
- **WordArt Styles is read as each application's Shape Format text**, on the census's equal counts (30 and 30 in Word
  and Excel, 33 and 33 in PowerPoint), so Text Effects keeps Transform, which Office may grey on chart text.
- **All four pickers' starts are the nearest swatches** to Office's chart grey (the five-step grid has no Lighter 85%
  or 35%).
- **Shapes leaves out New Drawing Canvas and Action Buttons; Change Shape is drawn available.**
- **Height 8.89 cm and Width 15.24 cm**, and the three launchers (*Layout* on Size).
- **The counts.** Current Selection, Insert Shapes, Accessibility and Size are met. Shape Styles 35 and 4, WordArt Styles
  30 and 4, Arrange 65 and 8. Nothing is padded.
- **Glyphs**, all `GUESS:`. **Reset to Match Style's `arrow-reset`, which says *reset* and not *to the style*, is the
  weakest**; Change Shape draws Edit Shape's `bezier-curve-square`. Chart Elements, the two galleries, the four colour
  pickers, the two fields, Position and Selection Pane carry none.

### The entries beneath a colour picker's palette

**Office's colour grids carry commands under the swatches, and `<mjx-color-picker>` now draws them.** A host slots
one `<mjx-menu slot="entries">` inside the picker; the picker draws it beneath the palette, in the same popup, and
sets `embedded` on it so its rows sit flush on the palette rather than as a card inside a card. It is the
catalogue's own menu, so submenus, checkable rows, unavailable rows and `mjx-menu-activate` all work as on any menu.
Choosing an entry closes the picker and returns focus to the field. `no-fill-label` names the picker's *none* chip
(*No Fill*, *No Outline*, *No Colour*); the value stays `none`.

- **The keyboard crosses at one seam.** Arrow Down on the palette's last drawn row moves focus onto the first entry,
  and the grid drops its cursor. Arrow Up on the first entry returns to the swatch it left. Both rules are pure
  functions in `src/pickers/picker-model.ts` (`paletteMovesIntoEntries`, `entriesReturnToPalette`,
  `paletteReturnIndex`), tested in `tests/pickers.test.ts`. Escape on an entry closes the picker; Tab leaves.
- **The popup is no longer the listbox.** A menu is not an allowed child of a listbox, so the swatches are a
  `role="listbox"` inside the popup, and `PopupSurface.controlledElement` names it for the field's `aria-controls`.
- **The lists are written once**, in `stories/ribbons/colour-picker-entries.ts`: `fillEntries` and `outlineEntries`
  take options for each entry, over `lineWeightEntries` (`tableLineWeights`), `lineDashEntries` (`presetLineDashes`,
  which Pen Style now also reads), `lineSketchEntries`, `lineArrowEntries`, `gradientEntries`, `textureEntries` and
  `tableBackgroundEntries`. Shape Format's Shape Fill and Shape Outline call the same two functions, with options from
  `stories/ribbons/drawing-tools-menus.ts`' `shapeFillEntryOptions` and `shapeOutlineEntryOptions`.
- **Bound today**: Word's Table Design Shading and Pen Colour (both hosts), PowerPoint's Table Design Shading, Text
  Fill, Text Outline and Pen Colour, Word's Picture Format Picture Border (`Ribbons/Word`), PowerPoint's Picture
  Format Picture Border, with an Eyedropper (both hosts), Excel's Picture Format Picture Border, without one
  (`Ribbons/Excel`), PowerPoint's Shape Format Shape Fill, Shape Outline (the first to carry **Arrows ▸**), Text
  Fill and Text Outline (`Ribbons/PowerPoint`), and Word's Shape Format's four, with no Eyedropper, Text Fill carrying
  Gradient alone and Text Outline Weight and Dashes (`Ribbons/Word`), and Excel's Shape Format's four, with no
  Eyedropper and PowerPoint's entries otherwise (`Ribbons/Excel`), and Word's Chart Format's four, Shape Outline with
  no Sketched or Arrows and the text pickers Word's Shape Format's (`Ribbons/Word`). `GUESS:` which entries each carries, every label and
  preset, and that Word's carry More Colours… alone.
- ⚠ **Table Background's colour grid is not drawn.** Its submenu lists its four commands; a menu holds commands, not
  swatches.

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

### An empty column is not a feed

`role="feed"` requires owned `article` children, so a document with nothing to review would publish
an invalid role — axe says so by name (`aria-required-children`) and a reader would be told there is
a list and then find nothing in it. With no annotations the column is `hidden`, carries no role, no
name and no tab stop, and the message beside it is what is announced. There is a gate for it.

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

## Mobile forms (MJXOFF-194)

`src/mobile/` holds the two components that exist **only** on a phone — `<mjx-command-bar>` and
`<mjx-contextual-action-bar>` — the completion of U09's sheet, and the one thing in this catalogue
that is not a component at all: **a sweep over every component in it.** Open
*Mobile/Gestures And Reach* first; it is written for a person doing the audit and everything on it
is generated from the tables the components read.

```html
<mjx-command-bar label="Home"></mjx-command-bar>
<mjx-contextual-action-bar selection="text" selection-label="12 words"></mjx-contextual-action-bar>
<mjx-dialog label="Paragraph" modal open detent="half">…</mjx-dialog>
```

### Width is a container query; **shortness is a media query**

Every responsive decision in this catalogue before this one was a container query, and that is
right. But **a landscape phone is 844 px wide, so by width alone it is a tablet** — eight commands
laid across a screen 390 px tall, all of them out of thumb reach. The missing fact is the viewport's
*shape*, which is not a property of any container and cannot be one.

So `mobilePresentationCss` emits three `@container` blocks and one `@media (max-height: …)` block
nested inside a fourth, in the order `formFactorFor()` evaluates, every selector inside `:where()` so
source order is the only thing deciding. That is not device sniffing — nothing branches on a
user-agent string, a touch capability or a platform name — and `tests/browser/mobile.spec.ts` proves
the media half is doing work by opening the **same container width twice** with only the viewport's
block size different and requiring the two answers to differ.

⚠ **`landscapePhoneInlineAtOrBelow` is 1000 and `smallTabletAtOrBelow` is 900, and the first draft
was wrong about exactly that.** An iPhone 15 Pro Max in landscape is **932** px wide — *past* the
small-tablet threshold — so bounding the landscape branch by that threshold sent the biggest phone on
the market to the desktop presentation. The test that found it is named for the device.

### The command bar is U04's ladder, and it is **one rail**

`commandBarOrder` sorts by `groupPriorities`' rank, then by U04's `essential` flag, then by
declaration order. Nothing here is a second priority scheme; `GroupPriority` and
`essentialCommandLimit` are imported from `ribbon-model.ts`.

The overflow is MJXOFF-183's rule applied a second time: **every command is built once, into one
rail**, and the overflow control changes what that rail *is* — a horizontal scroller pinned to the
bar, or a wrapped grid floating above it. The DOM nodes are identical in both, so *nothing is lost*
is structural rather than remembered, and the browser gate asserts it by comparing the two lists of
`data-command` values.

Three gates, and the second is the one that matters:

| Gate | Says |
|---|---|
| `the order equals an independently computed one` | a selection sort over an explicit key, with nothing in common with the comparator |
| `the ladder's answer costs nothing, and taking the first N costs more` | `naiveCommandBarPartition` is **shipped beside** the real one and is measured against it |
| `the fixture is DISCRIMINATING` | declaration order is asserted **not** to equal ladder order, or the comparison above is vacuous |

`wordPhoneCommands` is Word's Home tab declared in *ribbon* order — Clipboard first, and Clipboard is
`secondary` — so the two genuinely disagree. Font contributes four essential commands so the
per-group ceiling of three has something to refuse, and four commands carry `hasPopup` so demotion
rule 1 does. **Paste is one of the four, and is not essential** — in this fixture and in the
contextual bar's shared four alike — because it is a split button in Office, which is the judgement
the ribbon census makes for every Clipboard group.

### The sheet was **completed**, not replaced

A sheet is still `<mjx-dialog>` in its sheet presentation, still pinned with `pinFloating`. What U09
left it without was the one number: `pinFloating` was handed `sheetBoundaryFraction`, and a sheet
with one size has no detents. A detent **is** that number, and `full` is an *alias* of it, asserted.

`sheetDragClaim` is the whole of the classic mobile defect, as four rules and a pure function:

1. the handle and the header are always the sheet's;
2. **a scrolled scroller keeps the drag, in either direction** — the rule whose absence dismisses a
   sheet when a person flicks a long list;
3. at the top of the scroller a downward drag hands off to the sheet;
4. everything else is the content's, including an upward drag at the top.

The claim is re-evaluated **on the first move, not on the press**: at `pointerdown` a gesture has no
direction yet, and a rule that guessed one would give the sheet every press landing in a list at the
top of its scroll.

⚠ **The snap is not animated, deliberately.** A detent change re-runs `pinFloating`, and
`applyPlacement` measures the box and corrects it — so a transition on `top` would be measured in
mid-flight, which is U04's finding about reading a box during a transition. The *drag* is one-to-one
with the finger (`translate`, with `transition-property: none` under `data-dragging`); only the
release is instant.

### Safe-area insets: the indirection is the point

A component cannot be tested against `env(safe-area-inset-*)` in a headless browser — Chromium
reports zero on every side and no flag changes it. So `mobileDocumentCss` publishes the four `env()`
values into four **registered** `<length>` properties on the document, and every component reads the
property. A notched device feeds them through `env()`; a gate feeds them by setting the property.
Same value, same channel, nothing stubbed.

⚠ **What the registration buys, and the test that thought it knew.** The first version of the gate
asserted the property computes to something matching `/^\d+px$/`, on MJXOFF-189's finding that an
unregistered custom property hands back its *substituted text*. That finding is true and the test was
vacuous, because the substituted text here **is** `0px` — deleting every `@property` block left it
green, and a mutation is what said so. What registration actually buys is **type checking**: a host
that sets a nonsense value gets the declared initial value and the padding keeps its gutter, whereas
unregistered the nonsense reaches the `calc()`, the declaration is invalid at computed-value time,
and `padding-block-end` collapses to zero — a bar sitting on the home indicator because somebody
typed a unit wrong. That is what the gate asserts now, and the mutation fails it.

### The touch-target audit — and **both** axes

MJXOFF-193 flagged that this catalogue's 24-pixel floor was asserted on the block axis only. This
child owns the sweep, so the decision is made and stated: **both axes are asserted.** A 200 × 12
target and a 12 × 200 one are equally unhittable, and every control here is laid out in a row, so the
block axis is the one a stray padding fixes by accident.

What makes that affordable rather than a wall of exemptions is that the rule asserted is **WCAG 2.2
SC 2.5.8's actual rule**: at least 24 × 24, *or* a 24-pixel circle centred on the target touching no
other target's circle. The spacing exception is what legitimately passes a scrollbar thumb, a
splitter and a slider track — and it passes them for the right reason. **There is no exemption list.**

The sweep runs over the built story index, at phone width, in both themes, and fails in four
independent ways:

| Failure | Where |
|---|---|
| a component `src/` defines that the registry does not know | `tests/mobile.test.ts`, before a browser starts |
| a registered component no story renders | `every component was accounted for` |
| a component declared target-free that grew a target | the same test |
| a target under the floor | every per-story test |

⚠ **Two ownership rules, and both were found by watching the sweep fail.** *A registered element
owns itself* — `<mjx-menu-item>`, `<mjx-splitter>` and `<mjx-scrollbar>` put their role and their
tab stop on their own **host**, so an upward-only walk attributed them to nobody and the accounting
reported seven components as never showing a target. And *a shadow root confers ownership, light-DOM
ancestry does not* — `<mjx-screentip>` wraps the control it describes, so an ancestor walk reported
the screentip as having grown four buttons that belonged to the story.

⚠ **`container` is a fifth audit kind and a weaker claim than `targets`.** A menu is full of things
to press and builds none of them: every one is an `<mjx-menu-item>` that owns itself. So
`<mjx-menu>`, `<mjx-context-menu>` and `<mjx-task-pane>` are asserted to add **nothing of their own**
— the same anti-rot property `presentational` has — and are not asserted to contain anything, because
what they contain is audited elsewhere or belongs to the host application. That is stated rather than
glossed: it is the softest assertion in the sweep.

⚠ **The spacing exception means "shrink one target" is not automatically a failure**, and that is
the rule being right rather than the gate being weak. Shrinking every control to 12 px *tall* left
the sweep green, correctly: a 12 px-tall button with 40 px between rows satisfies SC 2.5.8. Shrinking
it on **both** axes, so neighbours' centres fall inside 24 px, fires immediately and names the
element, the size and the distance. Anyone re-proving this gate should shrink and **crowd**, not just
shrink — a break that changes nothing proves nothing, which is MJXOFF-183's own finding.

⚠ **The aggregate is on disk, and that is not paranoia.** Playwright tears a worker down after a
failing test and starts a fresh one, so a module-level total resets at every failure. The first
version accumulated in memory, and on the run that had sixteen findings it reported forty components
as never rendered. The far worse half of that bug is the other direction: a run with **no** failures
never restarts a worker, so the in-memory aggregate looks complete and the accounting passes — the
gate would have been correct exactly when it was not needed.

### Four defects the sweep found, three of them in other children's components

Every one is invisible in a screenshot, which is the whole argument for the sweep.

* **`mjx-dropdown`, `mjx-combo-box`, `mjx-measure-input` and both pickers had a 22 px target inside a
  40 px field.** `.field` wears `.mjx-hit-target` and `align-items: center`, so the focusable
  `.entry` sat 22 px tall with nine dead pixels above and below it — a control that looks full
  height and is a third unpressable. Fixed with `align-self: stretch` on `.entry` in `inputBaseCss`;
  the text does not move.
* **`mjx-formula-bar`'s editor was sixteen pixels wide at 390 px.** The name slot has a 96 px floor
  and the affordances are fixed, so the formula — the thing the bar is *for* — got what was left.
  Nothing in U13's own suite could have found it: every assertion there is caret-relative, and a
  caret in a 16 px box is still in the right place. Fixed with a container block at
  `formulaBarStackAtOrBelow` (the fourth alias of `phoneShellAtOrBelow`) that wraps the editor onto
  its own full-width row, which is what Excel does on a narrow window.
* **`mjx-toast-region` and `mjx-toast` were classified the wrong way round** — the region builds the
  card and the toast is a descriptor, exactly as `<mjx-option>` is.
* **Five `touch-action: none` declarations already existed in four crates and none was written
  down.** The region table describes the mobile shell, so a browser gate over the mobile shell's own
  stories checks the mobile shell — the ticket's trap, one level up. `reservedGestureSites` now names
  all five with their justification (all five are drag affordances, which is the same argument the
  sheet handle makes), and a **catalogue-wide** unit scan requires the set to match exactly.

### Gestures: ambiguity resolves to the canvas, as a default branch

`resolveGesture(gesture, region)` returns `'chrome'` only where a region explicitly claims the
gesture; **everything else falls through to the canvas.** That is the rule as the function's default
branch rather than as a policy written beside it, which is the only shape in which it cannot be
forgotten. Forty-three of the forty-nine pairs are the canvas's.

A claim is made by writing `touch-action`, not by adding a listener, so each region declares both and
`gestureInconsistencies()` requires the claims to **equal** what the value actually suppresses. A
region that claimed one gesture and wrote a value taking three would have quietly taken two more from
the document.

⚠ **`tap`, `doubleTap` and `longPress` are claimed by nobody, deliberately** — and **nothing here
implements a long press.** `touch-action` cannot express those three, so a claim on one would be a
claim the table could not enforce; and an earlier draft of this map said a long press on a command
reveals its screentip, which nothing did. A convention described in a table with no code behind it is
the shape CLAUDE.md calls worse than none, so the rows say what the components actually do and the
screentip-on-long-press is named as loop 2's, where the bar is wired to a document.

⚠ **The `canvas` row is a contract, not an observation.** There is no canvas in this catalogue, and
Playwright has no multi-touch — so *pinch reaches the document* is asserted through the
`touch-action` values every chrome region computes and the browser's own documented behaviour, never
by driving two fingers. That is the strongest claim available without the renderer, and it is a
weaker claim than it looks.

### Reachability

`thumbReachBlockFraction` is 0.55, measured from the **viewport's** bottom rather than any
container's — a thumb does not know what a container query is. A primary action must lie **wholly**
inside the band: half a button inside it is a button a person aims at and misses, and the half they
can reach is the half nearest the edge. `GUESS:` the number is ours; no reach study was run.

### `GUESS:` where this diverges from Office

Four, marked at their sites: the thumb-reach fraction, the detent fractions and the flick projection
span (120 ms), the per-form-factor slot counts, and the shortness threshold. None is checked against
any platform's own physics and none of it is parity.

## The assembly (MJXOFF-274)

**Nine stories — PowerPoint, Word and Excel, each at desktop, tablet and phone — built from the real
custom elements and from nothing else.** They live under `Shell/` and the sidebar is sorted to put
that folder **first**, because the assembly is what a reviewer opens Storybook to look at.

```sh
cd ui && npm run storybook          # http://localhost:6006 → Shell/PowerPoint, Shell/Word, Shell/Excel
```

Fifteen children audited components in isolation, and every one of them can be right while the
composition is wrong. What only appears here: a spacing that reads generous around one button and
loose across five ribbon groups; two surfaces that each clear their contrast floor and sit badly
beside one another; a component that takes half the window when nothing else is competing for it.

### Live, inert, and where the line is

Every component's **own** interaction works — menus open, the ribbon collapses, galleries preview,
panes resize, the formula bar's autocomplete opens, the sheet snaps between detents, the phone rails
overflow. **No command does anything to a document**: command dispatch, document binding, the
`ShellBridge` and the real Rust canvas are all loop 2. The document surface says so on its face,
because a reviewer must not be left wondering why pressing Bold changes nothing.

### The gates, and what each is for

| Gate | Where | What it refuses |
|---|---|---|
| **no mock-up** | `tests/shell.test.ts` | a raw `<button>`, `<input>`, `<dialog>`…; a `<div>` wearing a `role`; an interactive `aria-*` state; an invented `mjx-` element |
| **coverage, forwards** | `tests/browser/shell.spec.ts` | a catalogued component that appears in none of the nine |
| **coverage, backwards** | the same | a component named in `absentFromShells` that is in fact present — a reason that has become a lie |
| **the right width** | the same | a story whose frame did not come up at the preset its own `globals` declare |
| **no horizontal overflow** | the same | a box that is not a declared scroller and holds more than it is wide |
| **no clipped surface** | the same | a named surface under 24 px on an axis, or outside the shell's own box |
| **nothing hidden and drawn** | the same | an element carrying `hidden` that still has a box |
| **both schemes** | the same | all of the above, at 9 × 2 |

`stories/shell/shell-model.ts` holds the rule and the declarations; `shell-parts.ts` holds the layout
and **the only wiring in the assembly**, which is one function that opens the surface a launcher
names. Everything else a component does for itself.

Two components are **deliberately absent from all nine**, with reasons in `absentFromShells`:
`<mjx-resizable-container>` (the harness frame the story renders *inside*) and `<mjx-plate-gallery>`
(a developer surface for the render oracle, not chrome the product ships).

### The cosmetic problems the assembly revealed

**This is the actual output of MJXOFF-274.** Nine of these were left as they are, on purpose:
MJXOFF-195 is the user in front of the browser, and a composition problem is usually a decision about
a token or a component rather than a bug to be quietly patched.

**Fixed, because each was unambiguously a defect:**

1. **`[hidden]` was not restated in three sheets, so four elements were permanently drawn.** A
   measure input's invalid warning glyph *and* its message, a font picker's substitution warning, a
   label's hint and a slider's empty tick rail, all with the attribute set, a correct accessibility
   tree and four component suites passing. `ui/README.md` has recorded the trap since MJXOFF-189;
   `input-model.ts` and `picker-model.ts` predate it. Invisible in a catalogue — a small triangle in
   each of four fields on a page of fields reads as part of the design — and obvious the moment one
   field sits in a task pane. Fixed with one line per sheet and locked by the *nothing hidden and
   drawn* gate. Note that `picker-model.ts` had a **single-selector** version of the line covering
   only the message: somebody met this once and patched the symptom.
2. **The name box belongs inside the formula bar.** `<mjx-formula-bar>` builds a `name-box` slot of
   its own; setting the two side by side left an empty 96 px slot in the bar and made the band 101 px
   tall instead of 58.
3. **`<mjx-measure-input>`'s `value` is always in points**, and a centimetre field written as `12.7`
   reads `0.45 cm`. A number that is wrong and perfectly plausible.
4. **A gallery in a ribbon does not clip.** Given a height it draws its second row *below the
   ribbon*, over the navigation pane. The shell now sets `overflow: hidden`; whether the component
   should is a question for the audit (see 5).

**Left for MJXOFF-195, because each is a judgement:**

1. **The ribbon takes 45–60 % of the window.** 390 px of 832 on a desktop PowerPoint, 522 px on a
   tablet Word. Office's Home tab is about 140 px. Two causes, both design decisions:
   * **`.body { flex-wrap: wrap }` — a ribbon that runs out of room wraps to a second row instead of
     demoting a group.** The collapse ladder is driven by the *ribbon's* `@container` width, not by
     how much room is left on the row, so five groups that are 1,935 px wide all stay expanded at
     1,440 and wrap. Office never wraps. This is the single largest cosmetic finding.
   * **A ribbon group lays its commands out in one horizontal row with full labels.** Office stacks
     up to three small commands in a column; Clipboard is 460 px here and about 120 px there.
2. **A gallery in a ribbon asks for 266 px** — half the ribbon, and roughly four times what Word's
   Styles gallery occupies. The cell size comes from the item art, so the question is whether the
   in-ribbon presentation should cap it.
3. **`<mjx-dropdown>` will not lay out below 112 px** and silently overflows a narrower host. Every
   short field in these ribbons is 7 rem for that reason.
4. **`<mjx-task-pane>` will not lay out below about 280–336 px.** At 834 a quarter-width pane
   overflows the workspace by 136 px, so the tablet shells give the pane a *larger share of the
   smaller screen* and take it out of the navigator. The alternative — the pane becoming an
   **overlay** at tablet — is a presentation the component does not have, and it is probably the
   right answer.
5. **Word at tablet is the hardest case in the catalogue.** A navigation pane on one side and a
   review margin on the other leave the page 241 px. One of the two should probably go.
6. **A review margin shows two cards** beside a desktop page, because the ribbon has taken half the
   height and a card is about 150 px. The connector dots between the page and the column read as
   stray marks when the cards they belong to are off-screen.
7. **The formula bar's mode chip renders below the bar**, unattached to anything, and reads as a
   floating status pill in the corner of the grid.
8. **`<mjx-empty-state>`'s heading uses the display face**, which is right on a page and
   disproportionate in a 280 px task pane.
9. **A phone shell shows the contextual action bar and the command bar stacked.** In Office one
   replaces the other; both are shown here so a reviewer can see both, and the two rails of icons
   read as one confusing double row.

Two smaller ones, recorded rather than argued: the **thumbnail rail wraps a title to one character
per line below about 150 px** (the tablet fraction is 0.19 for that reason), and a **paused toast
stack pins itself to the viewport corner**, so it is declared on the PowerPoint *desktop* shell only
— at tablet it covered the task pane and at phone it would cover the command rail.

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
* **`role="feed"` with no `article` children is an axe violation**, not merely an empty list —
  `aria-required-children`. A virtualised feed hits this the moment its data is empty, which is the
  one state a story is most likely to have and a gate least likely to cover. Drop the role rather
  than shipping an empty one.
* **Playwright starts a fresh worker after a failing test**, so any module-level total in a spec file
  resets at each failure. A sweep that accumulates across tests must write to disk, or its aggregate
  is complete exactly on the runs where nothing was wrong — which is the worst possible property for
  an assertion about coverage. MJXOFF-194 hit both directions of this in one afternoon.
* **A scroll container with no focusable content inside it fails `scrollable-region-focusable`**, and
  `<mjx-dialog>`'s body has been `overflow: auto` since MJXOFF-188. Nobody met it until a sheet
  listed paragraph styles — an `<ol>` with no tab stop in it. The fix is conditional on the body
  *actually* overflowing, because declaring the stop unconditionally would change the tab order of
  every dialog in the catalogue.
* **`env(safe-area-inset-*)` is always zero in headless Chromium and there is no flag.** Publish it
  into a registered custom property on the document and have components read the property; that is a
  channel a gate can drive without stubbing anything.
* **A `@container` query alone cannot find a landscape phone.** It is 844 px wide, which by width is
  a tablet. Shortness is a property of the viewport and needs `@media (max-height: …)` — which is not
  device sniffing, and is the only fact in this catalogue that is not a container's.
* **A story's own readout can fail the a11y sweep before the component does.** MJXOFF-193's packing
  readout has a `max-block-size` and `overflow: auto`, and at three hundred annotations it became a
  scroll container a keyboard could not reach: `scrollable-region-focusable`, reported against the
  story, in a sweep everyone reads as being about the component. The harness stage carries
  `tabindex="0"` for exactly this reason; anything scrollable a story writes needs it too.
* **A story declares its own container size with `globals: { containerPreset }` in its CSF.** It is
  Storybook's supported mechanism and it beats both the toolbar's last setting and a nested
  `<mjx-resizable-container>` — which would give the story a second harness chrome inside the first.
  A test that then *passes* the preset in the URL is testing its own URL: open the story without one
  and assert the frame's measured width instead.
* **`<mjx-formula-bar>` has a `name-box` slot, and the name box goes in it.** So do several other
  components: check for a slot before laying two elements out side by side. Assembling them as
  siblings looks almost right, and the "almost" is a 96 px empty slot and a band twice as tall as it
  should be.
* **`<mjx-measure-input>`'s `value` is in *points*, always.** The `unit` attribute is a display
  choice. `value="12.7" unit="cm"` is 0.45 cm, which is wrong and entirely plausible.
* **A component's shadow root can be a `display: block` region with an automatic height**, and
  nothing slotted into one can fill it. `<mjx-context-menu>` is the example: wrapping a flex-sized
  canvas area in it made the area take its *content's* height and run 218 px past the foot of the
  shell. Wrap the page, not the pane.
* **A floating surface inside a `container-type` element still escapes it.** Chromium does not make
  such an element a containing block for fixed descendants — `overlay/floating.ts` measured that and
  says so — so a menu or a dialog can be placed anywhere in the tree. But an `<mjx-menu>` that has
  not been given the `floating` attribute is **in flow and visible**, and in a shell that means a
  full-width menu sitting in the layout taking space from everything below it. A menu a shell holds
  for later needs `floating` from the start.
* **`[hidden]` is still not restated everywhere**, three children after MJXOFF-189 wrote the warning
  down. MJXOFF-274 found four elements permanently drawn with the attribute set, in sheets written
  before the discovery. When you add a sheet, restate it last; when you touch an old one, check.
  `tests/browser/shell.spec.ts` now sweeps the whole assembly for it.
* **A composition problem is usually a decision, not a bug.** The nine shells are for a person to
  look at, and the temptation when one looks wrong is to reach into the component and change it.
  Fix what is unambiguously broken; put the rest in a list with enough detail to be decided in a
  sentence. The list from this child is under *The assembly* above.
