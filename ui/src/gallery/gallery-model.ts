/**
 * The gallery vocabulary — **one table, read by the stylesheet, by the component and by the
 * gates.**
 *
 * `gallery` is **1,773** of Office's published controls: the second-largest archetype after
 * `button`, and the one that is least like a button. Styles, themes, shapes, transitions, table
 * styles, WordArt and chart layouts are all galleries, and MJXOFF-185 says the thing worth quoting:
 *
 * > **Nothing else in the catalogue behaves like it**, and treating it as *"a grid of buttons"* is
 * > how it goes wrong.
 *
 * Three things make it different, and each one is a section below.
 *
 * ## 1. The item is a *slot*, not an icon
 *
 * A style item is a miniature of formatted text; a shape item is a shape; a colour item is a
 * swatch. So `<mjx-gallery-item>` is a **descriptor**, not a rendered element: it captures whatever
 * markup its author wrote into an inert `DocumentFragment` and renders nothing at all. The gallery
 * clones that fragment into the cells it decides to build.
 *
 * That indirection is not an aesthetic choice, and two hard requirements fall out of it for free:
 *
 * * **Virtualisation becomes real rather than nominal.** An item outside the window has *no*
 *   rendered descendants, because its content is in a fragment and a fragment has no layout boxes.
 *   `tests/browser/gallery.spec.ts` counts nodes, per the ticket, rather than looking at a picture.
 * * **The same item set can be in two geometries at once.** The in-ribbon strip scrolls by row and
 *   the flyout is a categorised grid, and they are on screen together the moment the flyout opens.
 *   Light-DOM children rendered through a `<slot>` can only be in one place, so a slotted design
 *   would have needed the items *duplicated* — which is exactly how the two surfaces come to
 *   disagree about which item is selected. Here they cannot: the selection is one value on one
 *   component, and both surfaces are drawings of it.
 *
 * ## 2. Live preview is a *temporary* change that must be *exactly* undone
 *
 * This is the whole point of a gallery and it is the part a still cannot show. MJXOFF-185:
 *
 * > **The component owns the *protocol*, not the preview**: a hover emits a preview-request,
 * > leaving emits a cancel, selecting commits.
 *
 * The protocol lives in `preview-session.ts` as a pure state machine, so the invariant that makes
 * restoration exact is a thing a Node test can prove over arbitrary input rather than a thing a
 * browser test can observe once. The invariant is:
 *
 * > **Every `preview` is followed by exactly one `cancel` or exactly one `commit`, never both and
 * > never neither, and at most one preview is outstanding at any moment.**
 *
 * And the cancel event carries `restore` — the value that was committed when the preview began —
 * so *the host does not have to remember anything*. A protocol whose correctness depended on the
 * listener keeping its own undo stack would be a protocol that is right in this file and wrong in
 * every application.
 *
 * ## 3. The keyboard is not a second-class preview path, and it is not the pointer's path either
 *
 * MJXOFF-185 requires *"preview-request on hover **and** on keyboard focus"* — *"without that,
 * live preview is a mouse-only feature"* — and separately requires rapid traversal to be coalesced.
 * Those two pull in opposite directions, and the resolution is that **they are different acts**:
 *
 * * A pointer crossing eight cells to reach the ninth has expressed no intent about the eight. So
 *   the pointer path **coalesces**: entering a cell schedules a request, and entering another cell
 *   before it fires replaces it. Ten cells crossed quickly produce one request.
 * * An arrow key is a deliberate act *per item*, so the keyboard path emits **immediately** — the
 *   same rule MJXOFF-184 applied to submenus (*"the keyboard never waits for any of it"*). What
 *   protects the renderer there is not a delay but the outstanding-preview invariant: ten arrow
 *   presses emit ten requests and nine cancels, so there is never more than one preview live.
 *
 * Both are asserted, and the pointer one is asserted **against a run with the coalescing removed**,
 * because MJXOFF-184's lesson is that a timing assertion which passes with the timing taken away is
 * measuring nothing.
 *
 * ## Nothing here is a second opinion about paint
 *
 * A cell's `rest`, `hover`, `active`, selected and unavailable states are the same states a ribbon
 * button has, so they are the same table: `galleryCellStates` names a `ControlState` and the
 * declarations come from `controlStateDeclarations`. What this file supplies is the *selectors*,
 * because a cell is a `<div>` and `:disabled` can never match it — the same split
 * `src/menus/menu-model.ts` makes, for the same reason.
 *
 * ## Node-importable
 *
 * Data, strings and pure functions. No custom element is defined here.
 */

import {
  controlStateDeclarations,
  controlStateSpecs,
  controlStatesCss,
  effectiveStatePaint,
  resolvedStateFingerprint,
  type ControlState,
  type EffectiveStatePaint,
} from '../controls/control-states.ts';
import { densityProperties, spacingMultiple } from '../foundations/density.ts';
import { radiusVariable, surfaceLevels } from '../foundations/surfaces.ts';
import { motionRoleClass, type MotionRole } from '../foundations/motion.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { containerName, phoneShellAtOrBelow } from '../harness/presets.ts';
import {
  groupPresentationOrder,
  groupPresentationProperty,
  type GroupPresentation,
} from '../ribbon/ribbon-model.ts';
import { floatingProperties } from '../overlay/floating.ts';
import {
  defaultOverscanRows,
  rowsInWindow,
  virtualWindow,
  type VirtualWindow,
} from '../foundations/virtual-list.ts';
import type { Align, LogicalSide } from '../overlay/floating.ts';
import type { ColorScheme } from '../../tokens/tokens.ts';

// ── the elements ─────────────────────────────────────────────────────────────

/** The two tag names, so nothing spells one by hand. */
export const galleryTags = {
  gallery: 'mjx-gallery',
  item: 'mjx-gallery-item',
} as const;

/**
 * The three item kinds the catalogue proves, and the reason there are exactly three.
 *
 * MJXOFF-185: *"Three genuinely different item kinds render — a formatted-text miniature, a shape
 * and a colour swatch — proving the item is a slot rather than an icon."* They are listed here as
 * **data** so the story, the documentation and the gate all name the same three; the component
 * knows nothing about them, which is the point being proved.
 */
export const galleryItemKindNames = ['textMiniature', 'shape', 'swatch'] as const;

/** One of the three. */
export type GalleryItemKind = (typeof galleryItemKindNames)[number];

/** What each kind is, for the audit. */
export const galleryItemKinds: Readonly<Record<GalleryItemKind, { readonly description: string }>> =
  {
    textMiniature: {
      description:
        'A miniature of formatted text — the Styles gallery. The item is a paragraph rendered at ' +
        'the size and weight the style would apply, so what is being previewed is legible in the ' +
        'item itself.',
    },
    shape: {
      description:
        'A drawn shape — the Shapes and SmartArt galleries. Inline SVG, so it scales with the ' +
        'cell and takes its colour from the cell’s own `currentColor`.',
    },
    swatch: {
      description:
        'A block of colour — the theme and colour-set galleries. The one kind with no text in it ' +
        'at all, which is why the cell carries its name rather than deriving one from its content.',
    },
  };

// ── the three presentations ──────────────────────────────────────────────────

/**
 * How a gallery is presented. **Two of the three are behavioural differences, not skins.**
 *
 * * `strip` — the in-ribbon window: a few rows, scrolled a row at a time, with an expand
 *   affordance. It is the presentation the ribbon has room for.
 * * `flyout` — everything, in a floating grid wider than the ribbon, with categorised sections and
 *   a footer for the *"More…"* and *"Save selection as…"* commands Office puts there.
 * * `sheet` — the flyout on a phone. Pinned to the bottom edge at the boundary's full width,
 *   because a floating grid aimed at with a thumb is a grid nobody hits. Exactly the same
 *   substitution `<mjx-menu>` makes at exactly the same width, and for the same reason.
 *
 * The order is the cascade: every selector the emitter below writes is inside `:where()`, so the
 * later block refines the earlier one and nothing else decides. `tests/gallery.test.ts` asserts
 * both the order and the wrapping, because MJXOFF-183 proved the order assertion alone can be true
 * and useless.
 */
export const galleryPresentationOrder = ['strip', 'flyout', 'sheet'] as const;

/** One of the three. */
export type GalleryPresentation = (typeof galleryPresentationOrder)[number];

/** Everything a presentation fixes, and every field is something a gate can measure. */
export interface GalleryPresentationSpec {
  readonly description: string;
  /** `static` for the one that is in flow, `fixed` for the two that are not. */
  readonly position: 'static' | 'fixed';
  /** Whether the box is placed **against its anchor**. A sheet is pinned instead. */
  readonly anchored: boolean;
  /** Whether categorised sections and the footer are shown. The strip is linear and has neither. */
  readonly sectioned: boolean;
  /** The motion vocabulary it wears. */
  readonly motionRole: MotionRole;
  /** The corner treatment. A sheet squares off the edge it is pinned to. */
  readonly radius: 'card' | 'panel';
  readonly use: string;
}

export const galleryPresentations: Readonly<
  Record<GalleryPresentation, GalleryPresentationSpec>
> = {
  strip: {
    description:
      'The in-ribbon window: as many columns as the width affords, one or two rows, scrolled a ' +
      'row at a time, with scroll-back, scroll-forward and expand beside it.',
    position: 'static',
    anchored: false,
    sectioned: false,
    motionRole: 'surfaceSettle',
    radius: 'card',
    use: 'A gallery living in a ribbon group, at any width the group itself survives.',
  },
  flyout: {
    description:
      'Everything the gallery has, in a grid wider than the ribbon, grouped into its categories, ' +
      'with a footer for the commands Office puts under one.',
    position: 'fixed',
    anchored: true,
    sectioned: true,
    motionRole: 'panelEnter',
    radius: 'panel',
    use: 'The expanded gallery on a desktop or a tablet.',
  },
  sheet: {
    description:
      'The same expanded grid, pinned to the bottom of what clips it at full width, with the ' +
      'bottom corners squared because there is nothing below them to round away from.',
    position: 'fixed',
    anchored: false,
    sectioned: true,
    motionRole: 'sheetEnter',
    radius: 'panel',
    use: 'The expanded gallery on a phone, where a floating grid is a grid a thumb cannot aim at.',
  },
};

/** The custom property every presentation block writes, and the component and the gate read. */
export const galleryPresentationProperty = '--mjx-gallery-presentation';

/**
 * The width at or below which an expanded gallery becomes a sheet.
 *
 * **Read from `src/harness/presets.ts` rather than restated.** MJXOFF-184 left the instruction in
 * `menu-model.ts` — *"when a third surface needs it, hoist it"* — and this is the third surface.
 * `tests/gallery.test.ts` asserts that the ribbon's picker threshold, the menu's sheet threshold
 * and this one are still the same number.
 */
export const gallerySheetAtOrBelow = phoneShellAtOrBelow;

/**
 * Which presentation a gallery is in, from the container width and whether it is expanded.
 *
 * **The model the browser gate compares against.** The component never calls it — it reads
 * `--mjx-gallery-presentation` back out of the cascade, exactly as `<mjx-ribbon-group>` and
 * `<mjx-menu>` do — so this function and the stylesheet are two independent statements of one rule.
 */
export function galleryPresentationAt(
  containerWidth: number,
  expanded: boolean,
): GalleryPresentation {
  if (!expanded) return 'strip';
  return containerWidth <= gallerySheetAtOrBelow ? 'sheet' : 'flyout';
}

/** The attribute the component mirrors onto its flyout box when it is showing. */
export const galleryOpenAttribute = 'data-open';

/** The attribute a surface carries so its rules and the gate can name it. */
export const gallerySurfaceAttribute = 'data-surface';

/** The two surfaces one gallery draws. Both are geometries of one item set. */
export const gallerySurfaceNames = ['strip', 'expanded'] as const;

/** One of the two. */
export type GallerySurface = (typeof gallerySurfaceNames)[number];

// ── degrading with the group ─────────────────────────────────────────────────

/**
 * How many rows the strip shows, per the presentation its **ribbon group** is in.
 *
 * MJXOFF-185: *"a gallery must degrade with its group."* `<mjx-ribbon-group>` publishes
 * `--mjx-group-presentation` on the box that contains its slot, and a custom property inherits
 * through the flat tree, so a slotted gallery can read the group's own decision without measuring
 * anything and without the group knowing a gallery exists.
 *
 * One row below `full`, and the reason is that the thing a reduced group is short of is *vertical*
 * room — that is what reducing means — and nothing is lost by it, because the whole gallery is one
 * press away in the flyout. A gallery that kept two rows in a collapsed group would be a gallery
 * that pushed the commands beside it off the popup.
 */
export const galleryStripRows: Readonly<Record<GroupPresentation, number>> = {
  full: 1,
  reduced: 1,
  collapsed: 1,
};

/** The strip's row count for a group presentation. The model the browser gate compares against. */
export function galleryStripRowsFor(presentation: GroupPresentation): number {
  return galleryStripRows[presentation];
}

/** The attribute the component mirrors the group's presentation onto. */
export const galleryGroupPresentationAttribute = 'data-group-presentation';

// ── the cell state table ─────────────────────────────────────────────────────

/**
 * The states a cell can be in.
 *
 * Seven, and **none of them is `previewing`.** A cell that is previewing is the cell the pointer is
 * over or the keyboard is on, and both of those already have a state; an eighth state for it would
 * be a state no auditor could tell from `hover` in a screenshot, which is the definition of a state
 * that should not exist. `<mjx-menu>` made the same call about the row holding a submenu open, and
 * this is the same call for the same reason.
 *
 * `disabled` is absent: like a menu row, an unavailable gallery item stays reachable and carries
 * its reason. A style a person cannot apply is a style whose reason they must be able to read.
 */
export const galleryCellStateNames = [
  'rest',
  'hover',
  'active',
  'focus',
  'selected',
  'selectedHover',
  'unavailable',
] as const;

/** One of the seven. */
export type GalleryCellState = (typeof galleryCellStateNames)[number];

/** What a cell state is, in terms of the shared control table, and what produces it. */
export interface GalleryCellStateSpec {
  /** The control state whose paint this is. **The paint is never restated here.** */
  readonly controlState: ControlState;
  /** What puts a cell into it, and what the auditor should look for. */
  readonly description: string;
  /** The selectors that produce it, with `%s` standing for the cell's own selector. */
  readonly matches: readonly string[];
}

/** A cell that cannot be chosen never lights up, however the pointer is moved over it. */
const available = ':not([data-unavailable])';

export const galleryCellStates: Readonly<Record<GalleryCellState, GalleryCellStateSpec>> = {
  rest: {
    controlState: 'rest',
    description: 'Resting. The cell shows its art and its name and nothing else.',
    matches: ['%s'],
  },
  hover: {
    controlState: 'hover',
    description:
      'The pointer is over it — which is also the cell whose preview is in flight, deliberately ' +
      'drawn as one state rather than two.',
    matches: ['%s[data-state="hover"]', `%s:hover${available}`],
  },
  active: {
    controlState: 'active',
    description: 'Held down. One rung darker than hover, in the same neutral family.',
    matches: ['%s[data-state="active"]', `%s:active${available}`],
  },
  focus: {
    controlState: 'focus',
    description:
      'Reached by arrow key. **The keyboard highlight is the foundations’ focus ring and nothing ' +
      'else** — and on a gallery that matters more than anywhere else in the catalogue, because ' +
      'the cell’s own fill is what the item being previewed looks like.',
    matches: [],
  },
  selected: {
    controlState: 'on',
    description:
      'The style that is currently applied. The accent tint says *this one*, and it survives the ' +
      'strip becoming a flyout and back, because the selection is a value on the component rather ' +
      'than a class on a cell.',
    matches: ['%s[data-selected="true"]'],
  },
  selectedHover: {
    controlState: 'onHover',
    description:
      'Selected, with the pointer over it. The edge thickens; the fill does not change, so ' +
      '*selected* never has to compete with *hovered* for the same tint.',
    matches: [
      '%s[data-selected="true"][data-state="hover"]',
      `%s[data-selected="true"]:hover${available}`,
    ],
  },
  unavailable: {
    controlState: 'unavailable',
    description:
      'Unavailable and explained. **Still reachable by arrow key**, still announced, still ' +
      'carrying its reason — and it emits no preview at all, because previewing a style that ' +
      'cannot be applied is a promise the commit will break.',
    matches: ['%s[data-unavailable]'],
  },
};

/**
 * The cascade, in the order the rules are emitted — **and therefore in the order they win**, since
 * every one of them scores (0,0,0).
 *
 * `focus` is absent because it paints nothing. `unavailable` is last so that an unavailable
 * *selected* cell still looks selected and also looks unavailable, which is the honest rendering of
 * both facts and the same decision `controlStateCascade` and `menuItemStateCascade` make.
 */
export const galleryCellStateCascade: readonly GalleryCellState[] = [
  'rest',
  'hover',
  'active',
  'selected',
  'selectedHover',
  'unavailable',
];

/** The composed paint of one cell state — **the shared model, not a copy of it.** */
export function galleryCellPaint(state: GalleryCellState): EffectiveStatePaint {
  return effectiveStatePaint(galleryCellStates[state].controlState);
}

/** A cell state's paint resolved through the generated tokens, for the pairwise gate. */
export function resolvedGalleryCellFingerprint(
  state: GalleryCellState,
  scheme: ColorScheme,
): string {
  return resolvedStateFingerprint(galleryCellStates[state].controlState, scheme);
}

/** The state rules for one cell selector. Every selector wrapped; source order is the cascade. */
export function galleryCellStatesCss(selector: string): string {
  return galleryCellStateCascade
    .map((state) => {
      const spec = galleryCellStates[state];
      const where = spec.matches.map((match) => match.replaceAll('%s', selector)).join(', ');
      return `:where(${where}) {\n${controlStateDeclarations(controlStateSpecs[spec.controlState])}\n}`;
    })
    .join('\n');
}

// ── the two-dimensional keyboard model ───────────────────────────────────────

/**
 * Everything a key press can ask a gallery to do.
 *
 * **A model the component reads, not a description of what it happens to do.** The whole point is
 * that `tests/gallery.test.ts` can put every key through it — including the RTL mirror and every
 * edge of a wrapped grid with a ragged last row — in a Node test that runs in milliseconds, and
 * `tests/browser/gallery.spec.ts` can then assert that the component *obeys* it through real key
 * presses. A keyboard model that only exists inside a `switch` is a model nobody can compare a
 * browser against.
 */
export const galleryActions = [
  'next',
  'previous',
  'rowDown',
  'rowUp',
  'first',
  'last',
  'pageDown',
  'pageUp',
  'commit',
  'cancel',
  'expand',
] as const;

/** One of the eleven. */
export type GalleryAction = (typeof galleryActions)[number];

/** What the gallery knows about itself when a key arrives. */
export interface GalleryKeyContext {
  readonly direction: 'ltr' | 'rtl';
  /** Whether the expanded surface is showing. Decides what `Escape` and `Alt+ArrowDown` mean. */
  readonly expanded: boolean;
  /** Whether `Alt` was held. Office's own *open the list* modifier. */
  readonly altKey?: boolean;
}

/**
 * The gallery keyboard pattern, as a function.
 *
 * Three things in here are the ones that get written wrong:
 *
 * * **The inline arrows mirror under RTL.** A gallery reads in the direction the line runs, so
 *   `ArrowRight` is *next* in English and *previous* in Arabic. A component that hard-coded it
 *   would walk backwards through half the writing systems Office ships in.
 * * **`Escape` cancels a preview whether or not anything is open.** It is the *revert* key, and a
 *   gallery whose Escape only closed a flyout would leave a preview applied on the strip.
 * * **`Alt + ArrowDown` expands** — Office's own gesture for opening a list from its closed
 *   presentation, and it is what makes the flyout reachable without hitting a 24-pixel button.
 */
export function galleryKeyAction(
  key: string,
  context: GalleryKeyContext,
): GalleryAction | undefined {
  const forward = context.direction === 'rtl' ? 'ArrowLeft' : 'ArrowRight';
  const back = context.direction === 'rtl' ? 'ArrowRight' : 'ArrowLeft';

  if (context.altKey === true) {
    // The one chord. Anything else with Alt held belongs to the shell, not to a listbox.
    return key === 'ArrowDown' && !context.expanded ? 'expand' : undefined;
  }

  switch (key) {
    case 'ArrowDown':
      return 'rowDown';
    case 'ArrowUp':
      return 'rowUp';
    case 'Home':
      return 'first';
    case 'End':
      return 'last';
    case 'PageDown':
      return 'pageDown';
    case 'PageUp':
      return 'pageUp';
    case 'Enter':
    case ' ':
      return 'commit';
    case 'Escape':
      return 'cancel';
    default:
      break;
  }

  if (key === forward) return 'next';
  if (key === back) return 'previous';
  return undefined;
}

/** The actions that move the keyboard rather than doing something to the gallery. */
export const galleryMovementActions: readonly GalleryAction[] = [
  'next',
  'previous',
  'rowDown',
  'rowUp',
  'first',
  'last',
  'pageDown',
  'pageUp',
];

/** Whether an action moves the roving stop. */
export function isGalleryMovement(action: GalleryAction): boolean {
  return galleryMovementActions.includes(action);
}

/** The geometry a movement resolves against. */
export interface GalleryGrid {
  /** How many items there are in total. */
  readonly count: number;
  /** How many cells fit on a row **right now**. Never zero. */
  readonly columns: number;
  /** How many rows a page is — what `PageUp` and `PageDown` move by. Never zero. */
  readonly rowsPerPage: number;
}

/**
 * Where a movement lands. **Pure arithmetic, and the whole of the two-dimensional navigation.**
 *
 * Two decisions are made here and both are the kind that a component gets subtly wrong:
 *
 * * **The inline arrows are linear and the block arrows are not.** `ArrowRight` at the end of a row
 *   goes to the first cell of the next row, because that is the reading order of a wrapped grid and
 *   a person walking a gallery with one key expects to reach the end of it. `ArrowDown` from the
 *   last full row **clamps to the last item** rather than falling off a ragged tail into nothing —
 *   the case that is always missing, because the fixture that would show it has an exact multiple
 *   of the column count in it.
 * * **Nothing wraps around the ends.** `ArrowRight` on the last item stays there. A menu wraps
 *   because a menu is a short list a person is cycling; a four-hundred-item gallery that jumped
 *   from the last theme to the first would be a gallery that lost somebody's place.
 */
export function nextGalleryIndex(
  action: GalleryAction,
  index: number,
  grid: GalleryGrid,
): number {
  const { count } = grid;
  if (count === 0) return -1;
  const columns = Math.max(1, grid.columns);
  const rowsPerPage = Math.max(1, grid.rowsPerPage);
  const last = count - 1;
  const clamp = (value: number): number => Math.min(Math.max(value, 0), last);
  const current = clamp(index);

  switch (action) {
    case 'next':
      return clamp(current + 1);
    case 'previous':
      return clamp(current - 1);
    case 'rowDown':
      // The clamp is the ragged-tail case: the cell below the third column of the penultimate row
      // may not exist, and the answer a person means is *the last item*, not *nowhere*.
      return clamp(current + columns);
    case 'rowUp':
      // Deliberately **not** clamped to 0 from the first row: an ArrowUp that jumped to the first
      // item from the middle of the first row would be a movement nobody asked for. Staying put is
      // what a grid does at its own edge.
      return current - columns < 0 ? current : current - columns;
    case 'first':
      return 0;
    case 'last':
      return last;
    case 'pageDown':
      return clamp(current + columns * rowsPerPage);
    case 'pageUp':
      return clamp(current - columns * rowsPerPage);
    default:
      return current;
  }
}

// ── virtualisation ───────────────────────────────────────────────────────────

/**
 * How many rows outside the visible window are built anyway.
 *
 * One, at each end. Zero means a row is created during the scroll that reveals it, which is a blank
 * band on every flick; more than one buys nothing and costs exactly what virtualisation was for.
 */
export const galleryOverscanRows = defaultOverscanRows;

/** One row of a surface's plan: a section heading, or a run of cells. */
export type GalleryRow =
  | { readonly kind: 'heading'; readonly category: string }
  | { readonly kind: 'cells'; readonly start: number; readonly end: number };

/**
 * The row plan for a surface — **the one place the two geometries are actually decided.**
 *
 * The strip is linear: `ceil(count / columns)` rows of cells and nothing else. The expanded surface
 * is categorised: each category contributes a heading row and then its own runs of cells, which is
 * what makes a category *a break in the grid* rather than a label floating over a continuous one.
 *
 * It is a plan of **rows** rather than of items, so it is `O(count / columns)` long and a
 * four-hundred-item gallery plans about fifty entries. That is the number virtualisation actually
 * cares about: the expensive thing is a DOM node, not an integer.
 *
 * `categories` is the item list's categories **in first-appearance order**, so a gallery's author
 * decides the order of its sections by the order they wrote the items in — and an item with no
 * category joins the section named by `uncategorised`.
 */
export function galleryRowPlan(
  categories: readonly string[],
  columns: number,
  sectioned: boolean,
): GalleryRow[] {
  const width = Math.max(1, columns);
  const rows: GalleryRow[] = [];
  if (categories.length === 0) return rows;

  if (!sectioned) {
    for (let start = 0; start < categories.length; start += width) {
      rows.push({ kind: 'cells', start, end: Math.min(start + width, categories.length) });
    }
    return rows;
  }

  let index = 0;
  while (index < categories.length) {
    const category = categories[index] ?? '';
    let end = index;
    while (end < categories.length && categories[end] === category) end += 1;
    rows.push({ kind: 'heading', category });
    for (let start = index; start < end; start += width) {
      rows.push({ kind: 'cells', start, end: Math.min(start + width, end) });
    }
    index = end;
  }
  return rows;
}

/**
 * The slice of rows a surface builds, given where it is scrolled to.
 *
 * MJXOFF-191 lifted this into `src/foundations/virtual-list.ts` as `VirtualWindow`; the name here
 * is an alias so no caller in this crate had to change.
 */
export type GalleryWindow = VirtualWindow;

/**
 * Which rows to build. **Pure, so the node-count gate has a number to compare against.**
 *
 * `firstVisibleRow` and `visibleRows` come from a measurement; everything after that is arithmetic,
 * and keeping it out of the renderer is what lets `tests/gallery.test.ts` assert the clamping at
 * both ends without a browser.
 *
 * ⚠ **This is now a one-line binding over `virtualWindow`, and the body moved rather than being
 * copied.** MJXOFF-191 needed the same arithmetic for four navigators over rows that are *not* all
 * one row tall, and the ticket is explicit that a second virtualisation is the thing to avoid — so
 * the arithmetic went **down** into the foundations, exactly as MJXOFF-190 moved the splitter's.
 * The signature, the defaults and the behaviour are unchanged, and `tests/navigators.test.ts`
 * asserts that over a sweep rather than leaving it as a claim.
 */
export function galleryWindow(
  rowCount: number,
  firstVisibleRow: number,
  visibleRows: number,
  overscan = galleryOverscanRows,
): GalleryWindow {
  return virtualWindow(rowCount, firstVisibleRow, visibleRows, overscan);
}

/** How many rows a window holds — the foundations' own count, re-exported for a gate to read. */
export { rowsInWindow };

/**
 * The number of cells a window builds, for the gate to compare a node count against.
 *
 * Derived from the plan rather than estimated, because *"virtualisation is asserted on node count"*
 * is only worth anything if the number it is asserted against comes from somewhere other than the
 * code that built the nodes.
 */
export function cellsInWindow(rows: readonly GalleryRow[], window: GalleryWindow): number {
  let total = 0;
  for (let index = window.firstRow; index < window.lastRow; index += 1) {
    const row = rows[index];
    if (row !== undefined && row.kind === 'cells') total += row.end - row.start;
  }
  return total;
}

/** The category an item with none of its own joins. */
export const uncategorisedSectionLabel = 'Uncategorised';

// ── events ───────────────────────────────────────────────────────────────────

/**
 * The events a gallery emits. **Wiring them to the renderer is loop 2; the protocol is this
 * child's.**
 *
 * MJXOFF-185 puts it exactly that way — *"the catalogue proves the events are emitted correctly"* —
 * and `preview-session.ts` is what makes *correctly* a provable word rather than a hopeful one.
 */
export const galleryEvents = {
  /**
   * Show me what this would look like. `detail` carries `value`, `label`, `by` and `restore`.
   *
   * `restore` is the committed value at the moment the preview began, so a listener that has lost
   * track can always get back — see `preview-session.ts` for why that is in the protocol rather
   * than in the listener.
   */
  preview: 'mjx-gallery-preview',
  /** Stop showing it and put back exactly what was there. `detail.restore` says what that is. */
  previewCancel: 'mjx-gallery-preview-cancel',
  /** Apply it for real. `detail` carries `value`, `label` and `by`. */
  commit: 'mjx-gallery-commit',
  /** The expanded surface opened or closed. `detail.expanded` says which, `detail.by` says what. */
  expand: 'mjx-gallery-expand',
} as const;

/** What produced a preview. Three, because the touch affordance is genuinely a third thing. */
export const previewSourceNames = ['pointer', 'keyboard', 'touch'] as const;

/** One of the three. */
export type PreviewSource = (typeof previewSourceNames)[number];

/**
 * How long a pointer must settle on a cell before its preview is requested, in milliseconds.
 *
 * **Not a design token, and deliberately not derived from one** — the same argument
 * `submenuHoverOpenDelay` makes. `--duration-transition` is how long a colour takes to change; this
 * is how long a person's hand takes to stop, and tying the two together would mean a designer who
 * sped up the platform's animation had made live preview flicker.
 *
 * It is shorter than the submenu delay on purpose. Opening a submenu moves the whole surface and
 * costs a person their place if it is wrong; a preview costs nothing and is undone by moving on, so
 * it can afford to be eager. What it may not be is *instant*, because instant is what turns a
 * pointer crossing a row into ten renders of a document.
 */
export const pointerPreviewSettleDelay = 120;

/**
 * The attribute that overrides the settle delay, in milliseconds.
 *
 * It exists for two reasons and the second is the one that matters. A shell wiring a genuinely
 * expensive renderer may want longer. And **the gate needs a run with the coalescing taken away**:
 * MJXOFF-184's lesson is that a timing assertion which still passes with the timing removed is
 * measuring nothing, so `tests/browser/gallery.spec.ts` drives the same ten-cell traversal at
 * `preview-delay="0"` and requires ten requests, beside the run that requires one.
 */
export const previewDelayAttribute = 'preview-delay';

/**
 * How long a touch must be held on a cell before it previews, in milliseconds.
 *
 * **The same number `<mjx-menu>`'s long press uses, read from there rather than restated**, because
 * it is the same fact about a hand: Android's `ViewConfiguration.getLongPressTimeout()` is 500ms
 * and iOS' recogniser is 500ms.
 */
export { longPressDelay as galleryTouchPreviewDelay } from '../menus/menu-model.ts';

/**
 * **The touch affordance for live preview, chosen here, and stated for the audit.**
 *
 * MJXOFF-185 makes the choice this child's work: *"Live preview needs a different affordance on
 * touch, and choosing it is part of this child's design work."* A hover-driven preview has no
 * meaning on a phone — there is no hover — so the question is what gesture means *show me* as
 * distinct from *do it*.
 *
 * The answer taken here is **press and hold previews; a tap commits.**
 *
 * * It is the only gesture on a touch screen that already means *look before you leap*: it is what
 *   a long press does on every platform's link, and a person who holds a cell and sees the document
 *   change has learnt the whole feature in one gesture with nothing to read.
 * * It costs no chrome. The alternatives are a preview *mode* toggle (a mode is a thing to get
 *   stuck in, and Office's own mobile apps have no such toggle) or a two-step tap-to-preview-then-
 *   confirm, which makes every single-item choice two taps and is the interaction people abandon.
 * * It composes with the sheet rather than fighting it: the sheet covers the bottom of the screen,
 *   the document is above it, and a held cell previews into the part that is still visible.
 *
 * ⚠ `GUESS:` **Office does not do this.** Office's mobile applications have no live preview at all,
 * so there is nothing to be parity with; this is a design decision made because the ticket asked for
 * one, and it is marked so an audit can overrule it rather than inherit it silently. What is *not*
 * a guess is the requirement it satisfies — that live preview must not be a feature only a mouse
 * can reach — and the same requirement is what puts the keyboard on the same protocol.
 */
export const touchPreviewAffordance = {
  gesture: 'press and hold',
  previews: 'while the cell is held',
  cancels: 'on release, restoring exactly what was there',
  commits: 'a tap — a press released before the hold delay',
  isGuess: true,
} as const;

// ── the catalogue contract the gates read ────────────────────────────────────

/** The attribute a states-matrix cell carries. Spelled in the story too — see `specimens.ts`. */
export const galleryStateCellAttribute = 'data-gallery-state-cell';

/**
 * The attribute a story sets to make every cell of a gallery pretend to be in one state.
 *
 * `hover` and `active` cannot be produced in a static story, and a catalogue whose states matrix
 * cannot show them is a catalogue that documents seven states and audits five. The forced selector
 * and the real pseudo-class **share one declaration block** in `galleryCellStates`, so a forced
 * state cannot drift into a mock of the real one — there is nothing for it to drift from — and the
 * browser gate still drives a real pointer over a cell and requires the two to compute identically.
 *
 * It is on the *gallery* rather than on the item, because an item is a descriptor and has no cell
 * of its own: the cell belongs to whichever surface built it, and there may be two of them.
 */
export const galleryForcedStateAttribute = 'force-cell-state';

/** The story every gate that measures paint opens. */
export const galleryStatesMatrixStoryName = 'The States Matrix';

/** The story titles, as string literals — CSF refuses a computed one. */
export const galleryStoryTitles = {
  gallery: 'Galleries/Gallery',
} as const;

/** The default side and alignment the expanded flyout opens on. */
export const galleryFlyoutSide: LogicalSide = 'blockEnd';
export const galleryFlyoutAlign: Align = 'start';

/**
 * The parts a gallery publishes, so a shell can style what it is allowed to.
 *
 * **Checked, not documented.** `tests/browser/gallery.spec.ts` opens the flyout and requires the
 * set of `part` attributes actually in the shadow tree to equal this list *exactly* — so a part
 * that is declared and never rendered fails, and so does one that is rendered and never declared.
 * That is the set-equality shape `tests/browser/icons.spec.ts` uses on the icon subset, and it is
 * the shape that catches a list going stale in **both** directions. It caught one immediately:
 * `grid` was named here and on nothing.
 */
export const galleryParts = [
  'strip',
  'viewport',
  'grid',
  'rail',
  'scroll-back',
  'scroll-forward',
  'expand',
  'flyout',
  'section',
  'heading',
  'footer',
  'cell',
  'art',
  'caption',
] as const;

// ── geometry ─────────────────────────────────────────────────────────────────

/**
 * The cell's shape, in multiples of `--spacing`, so a re-seed moves it and no length is written
 * down here.
 *
 * `minInline` is a floor rather than a width: the grid is `auto-fill`, so the columns grow to share
 * whatever is left over and the count follows the width **in CSS**, with no resize handler deciding
 * it. That is MJXOFF-183's rule (*"no measuring in a resize handler"*) honoured for the part that
 * is a layout question. The column *count* is still measured, once per resize, because
 * two-dimensional keyboard navigation cannot be done without knowing it — and those are different
 * questions, one about how it looks and one about what a key means.
 */
export const galleryCellUnits = { minInline: 20, artBlock: 11 } as const;

/** How much of the boundary an expanded gallery may cover as a sheet before it scrolls. */
export const gallerySheetBoundaryFraction = 0.7;

/** How wide the flyout is, at least, in spacing units. Wider than the ribbon, which is the point. */
export const galleryFlyoutMinimumInlineUnits = 88;

/**
 * How many columns the flyout aims for.
 *
 * **A number of columns rather than a width**, because *"wider than the ribbon"* is a statement
 * about how many items a person can see at once and not about pixels. The grid is still
 * `auto-fill`, so this is a preferred size the boundary may cut down — a flyout that insisted on
 * seven columns inside a narrow frame would be a flyout hanging off the side of it.
 *
 * Measured, and worth writing down: **`inline-size: max-content` does not work here.** A grid of
 * `repeat(auto-fill, minmax(X, 1fr))` has no intrinsic maximum — one row of `1fr` tracks is as wide
 * as you let it be — so `max-content` resolved to the *minimum* and the flyout came out narrower
 * than the strip it expanded from, which is the one thing it must never be.
 */
export const galleryFlyoutColumns = 7;

/** The custom properties a presentation writes. Layout only; the box reads every one. */
export const galleryBoxProperties = {
  position: '--mjx-gallery-position',
  insetTop: '--mjx-gallery-inset-top',
  insetLeft: '--mjx-gallery-inset-left',
  inlineSize: '--mjx-gallery-inline-size',
  minInlineSize: '--mjx-gallery-min-inline-size',
  maxInlineSize: '--mjx-gallery-max-inline-size',
  maxBlockSize: '--mjx-gallery-max-block-size',
  radiusBlockStart: '--mjx-gallery-radius-block-start',
  radiusBlockEnd: '--mjx-gallery-radius-block-end',
  enterTranslate: '--mjx-gallery-enter-translate',
  sectionDisplay: '--mjx-gallery-section-display',
  footerDisplay: '--mjx-gallery-footer-display',
  /** How many rows the strip shows. Written by the group-presentation rules. */
  stripRows: '--mjx-gallery-strip-rows',
  /** The measured row pitch, written by the component so the sizer and the layer agree. */
  rowPitch: '--mjx-gallery-row-pitch',
  /** The virtual sizer's height: every row, whether or not it is built. */
  scrollBlockSize: '--mjx-gallery-scroll-block-size',
  /** How far the built layer is pushed down inside the sizer. */
  layerOffset: '--mjx-gallery-layer-offset',
  cellMinInline: '--mjx-gallery-cell-min-inline',
  cellArtBlock: '--mjx-gallery-cell-art-block',
} as const;

function presentationDeclarations(presentation: GalleryPresentation, indent: string): string {
  const spec = galleryPresentations[presentation];
  const sheet = presentation === 'sheet';
  const popup = spec.position === 'fixed';
  return [
    `${indent}${galleryPresentationProperty}: ${presentation};`,
    `${indent}${galleryBoxProperties.position}: ${spec.position};`,
    `${indent}${galleryBoxProperties.insetTop}: ${popup ? `var(${floatingProperties.y})` : 'auto'};`,
    `${indent}${galleryBoxProperties.insetLeft}: ${popup ? `var(${floatingProperties.x})` : 'auto'};`,
    `${indent}${galleryBoxProperties.inlineSize}: ${
      sheet
        ? `var(${floatingProperties.inlineSize}, auto)`
        : popup
          ? `calc(var(${galleryBoxProperties.cellMinInline}) * ${String(galleryFlyoutColumns)} + ` +
            `var(${densityProperties.step}) * ${String(galleryFlyoutColumns + 1)})`
          : '100%'
    };`,
    `${indent}${galleryBoxProperties.minInlineSize}: ${
      sheet ? 'auto' : popup ? spacingMultiple(galleryFlyoutMinimumInlineUnits) : '0'
    };`,
    // ⚠ `none`, not `100%`, for the same measured reason `menu-model.ts` records: a percentage
    // resolves against the containing block, a fixed box's containing block is whatever ancestor
    // happens to establish one, and the natural size the placement was computed for would then be
    // a size the flyout never had.
    `${indent}${galleryBoxProperties.maxInlineSize}: ${
      popup ? `var(${floatingProperties.maxInlineSize}, none)` : '100%'
    };`,
    `${indent}${galleryBoxProperties.maxBlockSize}: ${
      popup ? `var(${floatingProperties.maxBlockSize}, none)` : 'none'
    };`,
    `${indent}${galleryBoxProperties.radiusBlockStart}: ${radiusVariable(spec.radius)};`,
    `${indent}${galleryBoxProperties.radiusBlockEnd}: ${sheet ? '0px' : radiusVariable(spec.radius)};`,
    `${indent}${galleryBoxProperties.enterTranslate}: ${sheet ? spacingMultiple(8) : '0px'};`,
    `${indent}${galleryBoxProperties.sectionDisplay}: ${spec.sectioned ? 'block' : 'contents'};`,
    `${indent}${galleryBoxProperties.footerDisplay}: ${spec.sectioned ? 'flex' : 'none'};`,
  ].join('\n');
}

/**
 * The presentation rules: one base block, one flyout block, one `@container` sheet block.
 *
 * ⚠ **The order is the cascade**, and it is only the cascade because every selector is inside
 * `:where()`. All three score (0,0,0), so a rule appearing later is a rule that wins.
 * `tests/gallery.test.ts` asserts the wrapping as well as the order, because MJXOFF-183 proved the
 * ordering assertion alone can be true and useless — a (0,4,0) selector beat a (0,2,0) ladder and
 * a simplified ribbon never collapsed a group, with the order test green throughout.
 *
 * The container is `mjx-frame`, the harness's own — the same one `<mjx-menu>` addresses, because it
 * is the same question: *is this shell a phone*.
 */
export function galleryPresentationCss(): string {
  return [
    `:where(.surface) {\n${presentationDeclarations('strip', '  ')}\n}`,
    `:where(.surface[${gallerySurfaceAttribute}='expanded']) {\n${presentationDeclarations('flyout', '  ')}\n}`,
    `@container ${containerName} (width <= ${String(gallerySheetAtOrBelow)}px) {\n` +
      `  :where(.surface[${gallerySurfaceAttribute}='expanded']) {\n${presentationDeclarations('sheet', '    ')}\n  }\n}`,
  ].join('\n');
}

/**
 * The strip's row count, per the presentation of the ribbon group it is slotted into.
 *
 * Emitted from `galleryStripRows` so the table and the stylesheet cannot disagree, and matched on
 * an attribute the component mirrors from `--mjx-group-presentation` rather than on the custom
 * property itself: a style container query would work in Chromium and this catalogue's rule is that
 * a decision a gate must read is a decision written where `getComputedStyle` can see it.
 */
export function galleryGroupDegradationCss(): string {
  return groupPresentationOrder
    .map(
      (presentation) =>
        `:where(.surface[${galleryGroupPresentationAttribute}='${presentation}']) {\n` +
        `  ${galleryBoxProperties.stripRows}: ${String(galleryStripRows[presentation])};\n}`,
    )
    .join('\n');
}

// ── the stylesheets ──────────────────────────────────────────────────────────

const overlay = surfaceLevels.overlay;

/**
 * `<mjx-gallery>`'s own rules.
 *
 * ⚠ **`.cell` paints nothing.** No background, no border colour or style, no text colour, no
 * weight, no opacity, no shadow — every one of those comes from `galleryCellStatesCss()`, whose
 * selectors score (0,0,0). A single `background:` here would score (0,1,0) and out-specify the
 * entire state table, leaving a cell that renders its resting paint in all seven states while every
 * *"the state exists"* check passed. That is MJXOFF-181's specificity accident wearing this child's
 * costume, and `tests/gallery.test.ts` asserts the separation rather than trusting this comment.
 *
 * ⚠ **The expanded surface is a top-layer popover, and it is not a nicety.** MJXOFF-184 measured
 * what happens without one: a floating box inside a scrolling ancestor has the right rectangle, is
 * painted nowhere, and `elementsFromPoint` at its own coordinates returns the ancestor. A gallery is
 * worse off than a menu here, because its flyout opens from a *ribbon group*, whose collapsed panel
 * is itself an overlay. `manual` rather than `auto`, because dismissal and focus are this
 * component's own and an `auto` popover closes a parent the moment its child opens. What follows
 * undoes the UA popover stylesheet — an inset of zero, an automatic margin, a border, a padding and
 * a `CanvasText` colour, every one of which beats what a gallery is supposed to look like.
 */
export const galleryCss = `
  :host {
    display: block;
    ${galleryBoxProperties.cellMinInline}: ${spacingMultiple(galleryCellUnits.minInline)};
    ${galleryBoxProperties.cellArtBlock}: ${spacingMultiple(galleryCellUnits.artBlock)};
  }
  :host([hidden]) { display: none; }

  /* An item is a descriptor and draws nothing. Stated on the host's own document sheet as well,
   * so an item that has not upgraded yet does not flash its content into the page. */
  ::slotted(${galleryTags.item}) { display: none; }

  .surface {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    color: inherit;
    block-size: auto;
    position: var(${galleryBoxProperties.position});
    top: var(${galleryBoxProperties.insetTop});
    left: var(${galleryBoxProperties.insetLeft});
    inline-size: var(${galleryBoxProperties.inlineSize});
    min-inline-size: var(${galleryBoxProperties.minInlineSize});
    max-inline-size: var(${galleryBoxProperties.maxInlineSize});
    max-block-size: var(${galleryBoxProperties.maxBlockSize});
    display: flex;
    flex-direction: column;
    translate: 0 0;
    transition-property: translate, opacity;
  }

  .surface[${gallerySurfaceAttribute}='strip'] {
    flex-direction: row;
    align-items: stretch;
    gap: var(${densityProperties.step});
  }

  .surface[${gallerySurfaceAttribute}='expanded'] {
    overflow: hidden;
    border: ${overlay.border};
    background: ${overlay.background};
    box-shadow: ${overlay.shadow};
    border-start-start-radius: var(${galleryBoxProperties.radiusBlockStart});
    border-start-end-radius: var(${galleryBoxProperties.radiusBlockStart});
    border-end-start-radius: var(${galleryBoxProperties.radiusBlockEnd});
    border-end-end-radius: var(${galleryBoxProperties.radiusBlockEnd});
    z-index: 1;
  }

  /* A popup that is not open is not drawn. The strip has no closed state at all, which is why the
   * selector names the surface rather than only the open attribute. */
  .surface[${gallerySurfaceAttribute}='expanded'][${galleryOpenAttribute}='false'] { display: none; }

  /* One frame of entry, removed on the next. Only the sheet travels. */
  .surface[data-entering] {
    translate: 0 var(${galleryBoxProperties.enterTranslate});
    opacity: 0;
  }

  /* ── the scroller, the sizer and the built layer ── */

  .viewport {
    position: relative;
    flex: 1 1 auto;
    min-inline-size: 0;
    overflow-y: auto;
    overflow-x: clip;
    overscroll-behavior: contain;
    padding: 0;
    scrollbar-width: none;
  }

  .viewport::-webkit-scrollbar { display: none; }

  .surface[${gallerySurfaceAttribute}='strip'] .viewport {
    /* Exactly as many rows as the group we are in can afford, and not one pixel more: the strip is
     * a window and the scrollbar is what says there is more behind it. */
    block-size: calc(
      var(${galleryBoxProperties.rowPitch}, var(${galleryBoxProperties.cellArtBlock})) *
        var(${galleryBoxProperties.stripRows}, 2)
    );
    scroll-snap-type: y mandatory;
  }

  /* The sizer is every row the plan has, built or not. It is what makes the scrollbar honest —
   * a virtualised list whose scroller is only as tall as what it built is a list that says it is
   * three rows long. */
  .sizer {
    position: relative;
    block-size: var(${galleryBoxProperties.scrollBlockSize}, auto);
    min-block-size: 100%;
  }

  .layer {
    position: absolute;
    inset-inline: 0;
    top: var(${galleryBoxProperties.layerOffset}, 0px);
    display: flex;
    flex-direction: column;
    gap: var(${densityProperties.step});
  }

  .row {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(${galleryBoxProperties.cellMinInline}), 1fr));
    gap: var(${densityProperties.step});
    scroll-snap-align: start;
  }

  .row[data-kind='heading'] {
    display: var(${galleryBoxProperties.sectionDisplay});
  }

  .heading {
    margin: 0;
    padding-inline: var(${densityProperties.step});
    padding-block: var(${densityProperties.step});
    color: var(--theme-text-secondary);
  }

  /* ── the affordance rail ── */

  .rail {
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: var(${densityProperties.step});
    padding-block: 0;
    block-size: calc(
      var(${galleryBoxProperties.rowPitch}, var(${galleryBoxProperties.cellArtBlock})) *
        var(${galleryBoxProperties.stripRows}, 1)
    );
  }

  .affordance {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    border-width: 1px;
    border-radius: var(--radius-control);
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    flex: 1 1 0;
    min-block-size: 0;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .affordance:disabled { cursor: default; }

  /* ── the footer ── */

  .footer {
    display: var(${galleryBoxProperties.footerDisplay});
    flex-direction: column;
    align-items: stretch;
    gap: var(${densityProperties.step});
    padding: var(${densityProperties.step});
    border-block-start: 1px solid var(--theme-border-subtle);
  }

  /* ── one cell ── */

  .cell {
    display: grid;
    grid-template-rows: 1fr auto;
    justify-items: stretch;
    align-items: stretch;
    box-sizing: border-box;
    margin: 0;
    padding: var(${densityProperties.step});
    padding-block-start: 0;
    border-width: 1px;
    border-radius: var(--radius-control);
    text-align: center;
    cursor: pointer;
    overflow: hidden;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .cell[data-unavailable] { cursor: help; }

  .art {
    display: flex;
    align-items: center;
    justify-content: center;
    block-size: var(${galleryBoxProperties.cellArtBlock});
    overflow: hidden;
    pointer-events: none;
  }

  .art > * { max-inline-size: 100%; }

  .caption {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    pointer-events: none;
  }

  .visually-hidden {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    margin: 0;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }
`;

/** The class a presentation's motion role is applied with, read back from the model. */
export function galleryMotionClass(presentation: GalleryPresentation): string {
  return motionRoleClass(galleryPresentations[presentation].motionRole);
}

/**
 * The type role each line of a gallery wears.
 *
 * ⚠ **A caption is `dense`, and that is a contrast decision rather than a size decision.**
 * ⚠ **And the role goes on `.cell`, never on `.caption`.** Every type role declares `font-weight`,
 * so a role class on the caption would beat the weight the cell inherits from its state and a
 * *selected* cell would silently lose its bold — MJXOFF-182's adoption-order accident reached by a
 * fifth route, and the one `<mjx-menu-item>` records hitting by a fourth. The caption carries no
 * class at all and inherits everything, which is what makes the state table the only thing deciding.
 *
 * MJXOFF-184 measured `--theme-text-secondary` at **4.32 : 1** against `--theme-border-subtle`,
 * which is the fill `hover` paints across a whole cell — so a grey caption would become illegible
 * *exactly when the pointer is on it*, which on a gallery is exactly when it is being read. The
 * caption therefore stays at the primary text colour the state table gives it and is told apart
 * from a section heading by **size and weight**, never by colour. `tests/gallery.test.ts` asserts
 * the measurement rather than the prose, so the day a re-seed makes the grey pairing legal the gate
 * says so.
 */
export const galleryTypeRoles = {
  cell: 'dense',
  heading: 'label',
  footer: 'control',
} as const;

/** The type-role class one line of a gallery wears. */
export function galleryTypeClass(line: keyof typeof galleryTypeRoles): string {
  return typeRoleClass(galleryTypeRoles[line]);
}

/** True while the secondary-text pairing is illegible on the hover fill. See `galleryTypeRoles`. */
export const galleryCaptionIsSizeNotColour = true;

/** The whole sheet a `<mjx-gallery>` adopts. */
export const gallerySheet = [
  galleryCss,
  galleryPresentationCss(),
  galleryGroupDegradationCss(),
  galleryCellStatesCss('.cell'),
  // ⚠ The rail's buttons take the **button** state table, not the cell one, and the difference is
  // not cosmetic: `.affordance` is a real `<button>`, so `:disabled` matches it — and a scroll-back
  // that is disabled at the top of the list has to *look* disabled or it is a control a person
  // presses and presses. `galleryCellStates` has no hard-disabled member at all, for the reason a
  // menu row has none, and a cell is not a button.
  controlStatesCss('.affordance'),
].join('\n');

/** The rule the *document* needs: an un-upgraded descriptor must not flash its content. */
export const galleryDocumentCss = `
${galleryTags.item} { display: none; }
`;

/** Re-exported so a gate imports one module. */
export { groupPresentationProperty };
