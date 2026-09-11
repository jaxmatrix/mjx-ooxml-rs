/**
 * The menu vocabulary — **one table, read by the stylesheet, by the components and by both gates.**
 *
 * `menu` is 649 of Office's published controls and `contextMenu` another 302, and the inventory's
 * *"None (Context Menu)"* bucket is the largest single one in every application: 1,155 in Word,
 * 1,104 in Excel, 1,660 in PowerPoint. A weak context menu weakens more of the product than a weak
 * ribbon does.
 *
 * ## Nothing here is a second opinion about paint
 *
 * A menu row's `rest`, `hover`, `active`, checked and unavailable states are **the same states a
 * ribbon button has**, so they are the same table: `menuItemStates` names a `ControlState` and the
 * declarations come from `controlStateDeclarations`, which is `src/controls/control-states.ts`'s
 * own emitter. What this file supplies is the *selectors*, because those genuinely differ — a menu
 * row is a `<div>`, so `:disabled` can never match it, and (see below) a menu has no hard-disabled
 * state to match anyway.
 *
 * The consequence is that the browser gate asserts **correspondence** — every cell's computed
 * paint equals `effectiveStatePaint(state)` — with the identical model function U03's gate uses,
 * rather than asserting that the states merely differ. MJXOFF-182 is quoted at the top of
 * `control-states.ts` and it is the reason: *"a distinctness gate proves no two states are the
 * same; it does not prove any of them is right."*
 *
 * ## A menu has one kind of unavailable, and it is the reachable one
 *
 * `control-states.ts` distinguishes `disabled` (the native attribute: out of the tab order, inert
 * by the platform's own doing) from `unavailable` (`aria-disabled`: still focusable, still
 * announced, still carrying its reason). **A menu item is only ever the second**, and that is a
 * behavioural requirement rather than a preference:
 *
 * > A disabled item must not be activatable by pointer *or* key, and **must still be reachable by
 * > arrow keys** — unlike a disabled button in a toolbar.
 *
 * The ARIA menu pattern says a menu's disabled items stay in the arrow-key sequence, so a person
 * can find out that the command exists and is currently unavailable. An item that removed itself
 * from that sequence would be a command whose absence looks like a bug. So `menuItemStateNames`
 * has no `disabled` member, `tests/menus.test.ts` asserts it, and the browser gate arrows onto an
 * unavailable item and requires the arrow to land there and the activation to be refused.
 *
 * ## Every state selector is inside `:where()`, and this is the fourth time that matters
 *
 * MJXOFF-181 lost a focus treatment to `:where(…):focus:not(:focus-visible)` out-specifying
 * `:where(…):focus-visible`. MJXOFF-182 lost four bold states to a stylesheet adopted in the wrong
 * order. MJXOFF-183 **shipped** a `:host([simplified]) .group[data-priority='x']` at (0,4,0)
 * against container rules at (0,2,0), so a simplified ribbon never collapsed a group — and the
 * test asserting *emission order* stayed green throughout, because emission order only arbitrates
 * between rules of equal specificity.
 *
 * Menus are the densest state surface in the catalogue — `:hover`, `[data-unavailable]`,
 * `[data-checked]`, `[data-submenu-open]`, a forced state and a keyboard highlight all on one row —
 * so this is exactly where it would bite next. Every selector this file emits is wrapped, and
 * `tests/menus.test.ts` asserts the wrapping as well as the order.
 *
 * ## Node-importable
 *
 * Data, geometry and strings. No custom element is defined here, so the Playwright specs can
 * import the model they are comparing a browser against.
 */

import {
  controlStateDeclarations,
  controlStateSpecs,
  effectiveStatePaint,
  resolvedStateFingerprint,
  type ControlState,
  type EffectiveStatePaint,
} from '../controls/control-states.ts';
import { densityProperties, spacingMultiple } from '../foundations/density.ts';
import { motionRoleClass, type MotionRole } from '../foundations/motion.ts';
import { radiusVariable, surfaceLevels, type RadiusStep } from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { containerName } from '../harness/presets.ts';
import { tabStripPickerAtOrBelow } from '../ribbon/ribbon-model.ts';
import { floatingProperties, type Align, type Direction, type LogicalSide } from '../overlay/floating.ts';
import type { ColorScheme } from '../../tokens/tokens.ts';

// ── the three elements, and what each one is ─────────────────────────────────

/** The custom element names, so a gate never spells one itself. */
export const menuTags = {
  menu: 'mjx-menu',
  item: 'mjx-menu-item',
  separator: 'mjx-menu-separator',
  section: 'mjx-menu-section',
  contextMenu: 'mjx-context-menu',
} as const;

/** The three kinds of menu item Office uses, and the ARIA role each one carries. */
export const menuItemKinds = ['command', 'checkbox', 'radio'] as const;

/** One of the three. */
export type MenuItemKind = (typeof menuItemKinds)[number];

/** `checkbox` → `menuitemcheckbox`. The mapping ARIA fixes; nothing here is a choice. */
export const menuItemRoles: Readonly<Record<MenuItemKind, string>> = {
  command: 'menuitem',
  checkbox: 'menuitemcheckbox',
  radio: 'menuitemradio',
};

/** Whether a value names one of the three kinds. */
export function isMenuItemKind(value: unknown): value is MenuItemKind {
  return (menuItemKinds as readonly string[]).includes(String(value));
}

/** The two kinds that carry `aria-checked`. */
export const checkableMenuItemKinds: readonly MenuItemKind[] = ['checkbox', 'radio'];

// ── the item's anatomy ───────────────────────────────────────────────────────

/**
 * The parts a menu row is made of, in the inline order Office draws them.
 *
 * Exposed as CSS `part`s so a host can restyle one without reaching into a shadow root, and named
 * here so the browser gate can assert that every one of them is present when it should be and
 * absent when it should not — an anatomy that is *listed* and not *rendered* is exactly the kind
 * of thing a screenshot of one specimen hides.
 */
export const menuItemParts = [
  'item',
  'mark',
  'icon',
  'label',
  'description',
  'shortcut',
  'arrow',
] as const;

/** One of the seven. */
export type MenuItemPart = (typeof menuItemParts)[number];

/**
 * The glyph size a menu row's marks are drawn at.
 *
 * 16 rather than 20: a check mark and a submenu chevron are *marks* beside a command, drawn one
 * rung below the command's own icon — the same reasoning `control-states.ts` gives for the dialog
 * launcher's corner mark. Fluent draws each size separately, so this chooses a drawing rather than
 * scaling one.
 */
export const menuGlyphSize = 16;

/** The submenu chevron. `chevron-right` mirrors under RTL by the same rule text does. */
export const submenuArrowIcon = { name: 'chevron-right', size: menuGlyphSize } as const;

/** A checked `menuitemcheckbox`. */
export const checkedMarkIcon = { name: 'checkmark', size: menuGlyphSize } as const;

/** The chosen member of a `menuitemradio` group. */
export const radioMarkIcon = { name: 'radio-button', size: menuGlyphSize } as const;

/** The mark a kind draws when it is checked, or `undefined` for a plain command. */
export function markIconFor(kind: MenuItemKind): { name: string; size: number } | undefined {
  if (kind === 'checkbox') return checkedMarkIcon;
  if (kind === 'radio') return radioMarkIcon;
  return undefined;
}

/**
 * **The secondary lines are told apart by size, not by colour — and the palette decided that.**
 *
 * Office greys a shortcut hint and a description. This catalogue cannot, and the number is worth
 * writing down because it will change when the palette is re-seeded: `--theme-text-secondary`
 * (`#5c7168`) measures **4.32 : 1** against `--theme-border-subtle` (`#efe9dc`), which is the fill
 * the `hover` state paints across the whole row. That is below WCAG's 4.5 : 1 for normal text —
 * so a greyed shortcut would become illegible *exactly when the pointer is on it*, which is the
 * worst possible place to lose contrast, and axe would fail the story that showed it.
 *
 * So every line in the row inherits the row's own text colour and the hierarchy comes from the
 * type scale: the label is `control`, the description and the shortcut are `dense`. The section
 * heading is the one piece of secondary text in a menu, and it is legal because a heading is never
 * hovered — `tests/menus.test.ts` measures both facts rather than trusting this paragraph.
 */
export const menuSecondaryTextIsSizeNotColour = true;

/**
 * The type role each part of the row wears.
 *
 * ⚠ **`row` is on the box the state table paints, and there is no role on the label.** Every type
 * role declares `font-weight`, so a role class on `.label` would beat the weight the row inherits
 * from its state and a checked item would silently lose its bold — MJXOFF-182's adoption-order
 * accident, reached by a different route. The description and the hint *do* carry a role, and they
 * therefore stay at `dense`'s medium weight even on a checked row: the label carries the emphasis,
 * the hint does not, which is what Office draws anyway.
 */
export const menuItemTypeRoles = {
  row: 'control',
  description: 'dense',
  shortcut: 'dense',
  sectionHeading: 'label',
} as const;

// ── the state table ──────────────────────────────────────────────────────────

/**
 * The states a menu row can be in.
 *
 * Seven, not ten. `disabled` is absent for the reason in the module note — a menu's unavailable
 * items stay arrow-reachable — and `mixed` is absent because a menu item is one command's state
 * rather than a selection's summary: an indeterminate menu item has no meaning Office uses.
 */
export const menuItemStateNames = [
  'rest',
  'hover',
  'active',
  'focus',
  'unavailable',
  'checked',
  'checkedHover',
] as const;

/** One of the seven. */
export type MenuItemState = (typeof menuItemStateNames)[number];

/** What a menu state is, in terms of the shared control table, and what produces it. */
export interface MenuItemStateSpec {
  /** The control state whose paint this is. **The paint is never restated here.** */
  readonly controlState: ControlState;
  /** What puts a row into it, and what the auditor should look for. */
  readonly description: string;
  /** The selectors that produce it, with `%s` standing for the row's own selector. */
  readonly matches: readonly string[];
}

/** A row that cannot be used never lights up, however the pointer is moved over it. */
const available = ':not([data-unavailable])';

export const menuItemStates: Readonly<Record<MenuItemState, MenuItemStateSpec>> = {
  rest: {
    controlState: 'rest',
    description: 'Resting. The row shows its mark gutter, its icon, its label and its hint.',
    matches: ['%s'],
  },
  hover: {
    controlState: 'hover',
    description:
      'The pointer is over it — or its submenu is open, which is the same *highlighted* reading ' +
      'and deliberately not a state of its own.',
    matches: [
      '%s[data-state="hover"]',
      `%s:hover${available}`,
      // An item holding its submenu open is the highlighted item. Office paints it exactly as it
      // paints a hovered one, and inventing an eighth state for it would be a state no auditor
      // could tell from hover in a screenshot — which is the definition of a state that should not
      // exist.
      '%s[data-submenu-open]',
    ],
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
      'else** — a second fill for "the keyboard is here" would be a second focus treatment, which ' +
      'is the thing MJXOFF-181 wrote one ring to prevent.',
    matches: [],
  },
  unavailable: {
    controlState: 'unavailable',
    description:
      'Unavailable and explained. **Still reachable by arrow key**, still announced, still ' +
      'carrying its reason — and still refused. The dashed edge says there is a reason to read.',
    matches: ['%s[data-unavailable]'],
  },
  checked: {
    controlState: 'on',
    description:
      'A checkbox that is on, or the chosen member of a radio group. The mark in the gutter is ' +
      'the announcement; the accent tint is what makes it visible without reading.',
    matches: ['%s[data-checked="true"]'],
  },
  checkedHover: {
    controlState: 'onHover',
    description: 'Checked, with the pointer over it. The edge thickens; the fill does not change.',
    matches: [
      '%s[data-checked="true"][data-state="hover"]',
      `%s[data-checked="true"]:hover${available}`,
    ],
  },
};

/**
 * The cascade, in the order the rules are emitted — **and therefore in the order they win**, since
 * every one of them scores (0,0,0).
 *
 * `focus` is absent because it paints nothing. `unavailable` is last so that an unavailable
 * *checked* item still looks checked and also looks unavailable, which is the honest rendering of
 * both facts and is the same decision `controlStateCascade` makes.
 */
export const menuItemStateCascade: readonly MenuItemState[] = [
  'rest',
  'hover',
  'active',
  'checked',
  'checkedHover',
  'unavailable',
];

/** The composed paint of one menu state — **the shared model, not a copy of it.** */
export function menuItemPaint(state: MenuItemState): EffectiveStatePaint {
  return effectiveStatePaint(menuItemStates[state].controlState);
}

/** A menu state's paint resolved through the generated tokens, for the pairwise gate. */
export function resolvedMenuItemFingerprint(state: MenuItemState, scheme: ColorScheme): string {
  return resolvedStateFingerprint(menuItemStates[state].controlState, scheme);
}

/** The state rules for one row selector. Every selector wrapped; source order is the cascade. */
export function menuItemStatesCss(selector: string): string {
  return menuItemStateCascade
    .map((state) => {
      const spec = menuItemStates[state];
      const where = spec.matches.map((match) => match.replaceAll('%s', selector)).join(', ');
      return `:where(${where}) {\n${controlStateDeclarations(controlStateSpecs[spec.controlState])}\n}`;
    })
    .join('\n');
}

// ── the two presentations, and the one that is not a style ───────────────────

/**
 * How a menu is presented. **`sheet` is a behavioural difference, not a skin.**
 *
 * MJXOFF-184: *"on a phone a context menu is a sheet, not a floating list, which is a real
 * behavioural difference and not merely a style."* A floating list is aimed at with a pointer and
 * belongs beside the thing it acts on; a sheet is reached with a thumb and belongs at the bottom of
 * the screen, at full width, with targets a thumb can hit. So the two differ in where they are, how
 * wide they are, which motion role they wear, and — the part that makes it behaviour rather than
 * paint — **whether they are placed by geometry at all**.
 *
 * `inline` is the third, and it is not a popup: a menu with no invoker, rendered in flow. The
 * states matrix is built out of them, which is what lets `Tab` walk from one specimen to the next
 * and reach the `focus` cell through the browser's own input path.
 */
export const menuPresentationOrder = ['inline', 'floating', 'sheet'] as const;

/** One of the three. */
export type MenuPresentation = (typeof menuPresentationOrder)[number];

/** Everything a presentation fixes. */
export interface MenuPresentationSpec {
  readonly description: string;
  /** `static` for the one that is in flow, `fixed` for the two that are not. */
  readonly position: 'static' | 'fixed';
  /**
   * Whether the box is placed **against its anchor**.
   *
   * A sheet is not: it is pinned to one edge of the boundary at that boundary's full width, which
   * is a different question from *which side of this button*. Both are computed in JavaScript —
   * see `pinFloating` for the measurement that ruled out doing the sheet with four zeroes in a
   * stylesheet — and the distinction is what the component branches on.
   */
  readonly anchored: boolean;
  /** The motion vocabulary it wears. `sheetEnter` is the one §4 names for a sheet. */
  readonly motionRole: MotionRole;
  /** The corner treatment. A sheet squares off the edge it is pinned to. */
  readonly radius: RadiusStep;
  readonly use: string;
}

export const menuPresentations: Readonly<Record<MenuPresentation, MenuPresentationSpec>> = {
  inline: {
    description: 'In flow, with no invoker. What a states-matrix cell and a docked list are.',
    position: 'static',
    anchored: false,
    motionRole: 'surfaceSettle',
    radius: 'card',
    use: 'The catalogue’s specimens, and any menu a shell chooses to dock rather than float.',
  },
  floating: {
    description: 'Beside its invoker, flipped and shifted to stay inside what clips it.',
    position: 'fixed',
    anchored: true,
    motionRole: 'panelEnter',
    radius: 'panel',
    use: 'Every pointer-driven menu: a split button’s list, a submenu, a context menu on a desktop.',
  },
  sheet: {
    description:
      'Pinned to the bottom of what clips it, at full width, with the bottom corners squared ' +
      'because there is nothing below them to round away from.',
    position: 'fixed',
    anchored: false,
    motionRole: 'sheetEnter',
    radius: 'panel',
    use: 'A context menu on a phone. A floating list aimed at with a thumb is a list nobody hits.',
  },
};

/** The custom property every presentation block writes, and the component reads back. */
export const menuPresentationProperty = '--mjx-menu-presentation';

/**
 * The width at or below which a floating menu becomes a sheet.
 *
 * **Read from `mjx-ribbon`'s own constant rather than restated**, because it is the same fact: the
 * width at which this platform stops being a desktop is the width at which the ribbon's tab strip
 * becomes a picker *and* the width at which a floating list becomes a sheet. Two numbers that must
 * agree and are written twice are two numbers that will not agree. When a third surface needs it,
 * hoist it to `src/harness/presets.ts` — do not add a second definition.
 */
export const menuSheetAtOrBelow = tabStripPickerAtOrBelow;

/**
 * Which presentation a menu is in, from the container width and whether it has an invoker.
 *
 * **The model the browser gate compares against.** The component never calls this — it reads
 * `--mjx-menu-presentation` back out of the cascade, exactly as `<mjx-ribbon-group>` does — so this
 * function and the stylesheet are two independent statements of the same rule, which is what makes
 * comparing them worth anything.
 */
export function menuPresentationAt(containerWidth: number, floating: boolean): MenuPresentation {
  if (!floating) return 'inline';
  return containerWidth <= menuSheetAtOrBelow ? 'sheet' : 'floating';
}

/** The attribute the component mirrors onto its own box when it is acting as a popup. */
export const floatingAttribute = 'data-floating';

/** The attribute that says a popup is showing. */
export const openAttribute = 'data-open';

const overlay = surfaceLevels.overlay;

/** The custom properties a presentation writes. Layout only; the box reads every one of them. */
export const menuBoxProperties = {
  position: '--mjx-menu-position',
  /**
   * ⚠ **Physical, and deliberately not logical.**
   *
   * Everything about a placement is viewport-physical: `placeFloating` returns an `x` measured from
   * the left edge of the screen whatever direction the text runs, because `physicalSide()` has
   * already resolved the caller's *logical* preference. Writing that number into
   * `inset-inline-start` would mean **the right edge** under RTL, and the box would be placed its
   * own x-coordinate away from the wrong side of the screen — 1096px from the right instead of from
   * the left, which is how MJXOFF-184 first found an Arabic submenu at x = −1052. `applyPlacement`'s
   * measure-and-correct step cannot rescue it either: the correction assumes the coordinate and the
   * offset run the same way, and on a mirrored axis it diverges instead of converging.
   */
  insetTop: '--mjx-menu-inset-top',
  insetBottom: '--mjx-menu-inset-bottom',
  insetLeft: '--mjx-menu-inset-left',
  insetRight: '--mjx-menu-inset-right',
  inlineSize: '--mjx-menu-inline-size',
  minInlineSize: '--mjx-menu-min-inline-size',
  maxInlineSize: '--mjx-menu-max-inline-size',
  maxBlockSize: '--mjx-menu-max-block-size',
  /** `clip` for a floating menu, which never scrolls; `auto` for a sheet, which is capped. */
  blockOverflow: '--mjx-menu-block-overflow',
  radiusBlockStart: '--mjx-menu-radius-block-start',
  radiusBlockEnd: '--mjx-menu-radius-block-end',
  enterTranslate: '--mjx-menu-enter-translate',
  /** Reserved for the check/radio gutter, or `0px` when no item in the menu is checkable. */
  markColumn: '--mjx-menu-mark-column',
  /** Reserved for the command icon, or `0px` when no item in the menu has one. */
  iconColumn: '--mjx-menu-icon-column',
} as const;

/**
 * How wide a floating menu is at its narrowest.
 *
 * The same reasoning as the collapsed ribbon group's popup: a menu that shrank to the width of its
 * longest label would be a sliver a person has to aim at. In spacing units, so a re-seed moves it.
 */
export const menuMinimumInlineUnits = 44;

/**
 * The 16px glyph box, in spacing units.
 *
 * `--spacing` is `0.25rem`, so four of them is the glyph's own box at the default root size — and
 * unlike `16px` it follows a re-seed. `tests/menus.test.ts` asserts the two agree today, which is
 * the assertion that fires if `--spacing` ever changes and this needs a different multiple.
 */
export const menuGlyphColumnUnits = 4;

/** One column of the row's gutter: the glyph box plus one density step of air. */
const glyphColumn = `calc(${spacingMultiple(menuGlyphColumnUnits)} + var(${densityProperties.step}))`;

/**
 * How much of the boundary a sheet may cover before it scrolls.
 *
 * A fraction rather than a percentage, because a sheet is pinned in JavaScript and a CSS percentage
 * would resolve against the containing block instead of the boundary — which, measured, is the
 * window rather than the frame. `pinFloating` multiplies it out.
 */
export const sheetBoundaryFraction = 0.75;

function presentationDeclarations(presentation: MenuPresentation, indent: string): string {
  const spec = menuPresentations[presentation];
  const sheet = presentation === 'sheet';
  const floating = presentation === 'floating';
  const popup = sheet || floating;
  const lines = [
    `${indent}${menuPresentationProperty}: ${presentation};`,
    `${indent}${menuBoxProperties.position}: ${spec.position};`,
    // Both popups read the same two coordinates. A sheet is *pinned* rather than placed, but it is
    // pinned to the same boundary the floating one flips against, and reading one pair of
    // properties is what makes a container resize across the threshold move the menu from one
    // presentation to the other rather than from one to something else.
    `${indent}${menuBoxProperties.insetTop}: ${popup ? `var(${floatingProperties.y})` : 'auto'};`,
    `${indent}${menuBoxProperties.insetBottom}: auto;`,
    `${indent}${menuBoxProperties.insetLeft}: ${popup ? `var(${floatingProperties.x})` : 'auto'};`,
    `${indent}${menuBoxProperties.insetRight}: auto;`,
    `${indent}${menuBoxProperties.inlineSize}: ${sheet ? `var(${floatingProperties.inlineSize}, auto)` : 'max-content'};`,
    `${indent}${menuBoxProperties.minInlineSize}: ${sheet ? 'auto' : spacingMultiple(menuMinimumInlineUnits)};`,
    // ⚠ The floating fallback is `none`, not `100%`, and the difference is measurable. A percentage
    // resolves against the containing block, and a fixed box's containing block is whatever
    // ancestor happens to establish one — here, the parent menu it opened from. A submenu would
    // therefore be capped at its *parent's* width while it was being measured, and the natural size
    // the placement was computed for would be a size the menu never had.
    `${indent}${menuBoxProperties.maxInlineSize}: ${
      popup ? `var(${floatingProperties.maxInlineSize}, none)` : '100%'
    };`,
    `${indent}${menuBoxProperties.maxBlockSize}: ${
      sheet ? `var(${floatingProperties.maxBlockSize}, none)` : 'none'
    };`,
    `${indent}${menuBoxProperties.blockOverflow}: ${sheet ? 'auto' : 'clip'};`,
    `${indent}${menuBoxProperties.radiusBlockStart}: ${radiusVariable(spec.radius)};`,
    // A sheet's bottom corners are square: it is pinned to an edge, and rounding away from an edge
    // there is nothing behind is how a sheet ends up looking like a floating card that slipped.
    `${indent}${menuBoxProperties.radiusBlockEnd}: ${sheet ? '0px' : radiusVariable(spec.radius)};`,
    `${indent}${menuBoxProperties.enterTranslate}: ${sheet ? spacingMultiple(8) : '0px'};`,
  ];
  return lines.join('\n');
}

/**
 * The presentation rules: one base block, one floating block, one `@container` sheet block.
 *
 * ⚠ **The order is the cascade**, and it is only the cascade because every selector is inside
 * `:where()`. The floating block refines the base one and the sheet block refines the floating one,
 * and all three score (0,0,0), so a rule appearing later is a rule that wins — which is what
 * `tests/menus.test.ts` asserts, together with the wrapping, because MJXOFF-183 proved the
 * ordering assertion alone can be true and useless.
 *
 * The container is `mjx-frame` — the harness's own, the same one `<mjx-ribbon>` addresses. A menu
 * is `position: fixed` and a container query still resolves against its **flat-tree** ancestors, so
 * a menu inside a narrow frame becomes a sheet however the box happens to be positioned.
 */
export function menuPresentationCss(): string {
  return [
    `:where(.menu) {\n${presentationDeclarations('inline', '  ')}\n}`,
    `:where(.menu[${floatingAttribute}]) {\n${presentationDeclarations('floating', '  ')}\n}`,
    `@container ${containerName} (width <= ${String(menuSheetAtOrBelow)}px) {\n` +
      `  :where(.menu[${floatingAttribute}]) {\n${presentationDeclarations('sheet', '    ')}\n  }\n}`,
  ].join('\n');
}

// ── the keyboard model ───────────────────────────────────────────────────────

/**
 * Everything a key press can ask a menu to do.
 *
 * **This is a model the component reads, not a description of what it happens to do.** The whole
 * ARIA menu pattern is here as data, so `tests/menus.test.ts` can assert the mapping — including
 * the RTL mirror and the one case that is genuinely conditional — in a Node test that runs in
 * milliseconds, and `tests/browser/menus.spec.ts` can then assert that the component *obeys* it
 * through real key presses. A keyboard model that only exists inside a `switch` is a keyboard model
 * nobody can compare a browser against.
 */
export const menuActions = [
  'next',
  'previous',
  'first',
  'last',
  'activate',
  'openSubmenu',
  'closeSubmenu',
  'close',
  'leave',
  'typeahead',
] as const;

/** One of the ten. */
export type MenuAction = (typeof menuActions)[number];

/** What the menu knows about itself when a key arrives. */
export interface MenuKeyContext {
  readonly direction: Direction;
  /** Whether the item the keyboard is on opens a submenu. */
  readonly hasSubmenu: boolean;
  /** Whether this menu is itself a submenu, and therefore has a level to close. */
  readonly isSubmenu: boolean;
  /** Whether a type-ahead buffer is currently collecting, which is what Space means then. */
  readonly typing: boolean;
}

/**
 * The ARIA menu pattern, as a function.
 *
 * Two things in here are the ones that get written wrong:
 *
 * * **`Tab` leaves.** A menu is not a focus trap. See `focusManagementPatterns`.
 * * **The inline arrows mirror under RTL.** `ArrowRight` opens a submenu in English and *closes*
 *   one in Arabic, because a submenu opens toward the end of the line in both. A component that
 *   hard-coded `ArrowRight` would be unusable in half the writing systems Office ships in.
 */
export function menuKeyAction(key: string, context: MenuKeyContext): MenuAction | undefined {
  const forward = context.direction === 'rtl' ? 'ArrowLeft' : 'ArrowRight';
  const back = context.direction === 'rtl' ? 'ArrowRight' : 'ArrowLeft';

  switch (key) {
    case 'ArrowDown':
      return 'next';
    case 'ArrowUp':
      return 'previous';
    case 'Home':
    case 'PageUp':
      return 'first';
    case 'End':
    case 'PageDown':
      return 'last';
    case 'Enter':
      return 'activate';
    case 'Escape':
      return 'close';
    case 'Tab':
      return 'leave';
    default:
      break;
  }

  // Space activates — unless a type-ahead is in flight, when it is a space in a label.
  if (key === ' ') return context.typing ? 'typeahead' : 'activate';
  if (key === forward) return context.hasSubmenu ? 'openSubmenu' : undefined;
  if (key === back) return context.isSubmenu ? 'closeSubmenu' : undefined;
  // A single printable character. `key.length === 1` is the platform's own test for one.
  if (key.length === 1) return 'typeahead';
  return undefined;
}

/**
 * How long a type-ahead buffer collects before it starts again, in milliseconds.
 *
 * **Not a design token, and deliberately not derived from one.** `--duration-transition` is how
 * long a colour takes to change; this is how long a person takes to type the second letter of a
 * command's name, and tying the two together would mean a designer who sped up the platform's
 * animation had shortened everybody's typing window. The figure is the one every menu
 * implementation has used since the Windows 95 shell.
 */
export const typeaheadResetDelay = 1000;

/**
 * The next item a type-ahead buffer selects, or `-1` when it selects none.
 *
 * The one subtlety is the repeated character: pressing `p` three times cycles through the items
 * beginning with `p` rather than looking for an item called *"ppp"*, which is what a person means
 * and is the behaviour every desktop menu has. A buffer of two *different* characters is a prefix.
 *
 * Unavailable items are **not** skipped, for the same reason the arrow keys do not skip them: a
 * person typing *"pas"* to find *Paste Special* must land on it and be told it is unavailable, not
 * silently land on something else.
 */
export function nextTypeaheadIndex(
  labels: readonly string[],
  buffer: string,
  from: number,
): number {
  if (buffer === '' || labels.length === 0) return -1;
  const needle = buffer.toLowerCase();
  const repeated = [...needle].every((character) => character === needle[0]);
  const prefix = repeated ? (needle[0] ?? '') : needle;
  // A repeated buffer cycles, so it starts *after* where the keyboard already is; a growing prefix
  // must be able to stay where it is, or typing `p`, `a` would walk off the item it just found.
  const offset = repeated ? 1 : 0;
  for (let step = 0; step < labels.length; step += 1) {
    const index = (from + offset + step + labels.length) % labels.length;
    const label = labels[index];
    if (label !== undefined && label.trim().toLowerCase().startsWith(prefix)) return index;
  }
  return -1;
}

/**
 * `Ctrl+V` → `Control+V`, which is the spelling `aria-keyshortcuts` is defined in terms of.
 *
 * The visible hint is what Office draws; the ARIA value is what a screen reader reads out and what
 * a shortcut-listing tool collects. They are different strings for the same shortcut, and deriving
 * one from the other is what stops an author having to write both — a `keyshortcuts` attribute
 * overrides the derivation for the cases it cannot know about.
 */
export function ariaKeyShortcuts(display: string): string {
  const names: Readonly<Record<string, string>> = {
    ctrl: 'Control',
    control: 'Control',
    cmd: 'Meta',
    command: 'Meta',
    win: 'Meta',
    '⌘': 'Meta',
    alt: 'Alt',
    option: 'Alt',
    '⌥': 'Alt',
    shift: 'Shift',
    '⇧': 'Shift',
  };
  return display
    .split('+')
    .map((part) => part.trim())
    .filter((part) => part !== '')
    .map((part) => names[part.toLowerCase()] ?? part)
    .join('+');
}

// ── pointer intent ───────────────────────────────────────────────────────────

/**
 * How long a pointer must rest on an item before its submenu opens, in milliseconds.
 *
 * A platform interaction constant, like `accessibleHitTargetMinimum` and unlike a colour: Windows'
 * own `SPI_GETMENUSHOWDELAY` has defaulted to 400ms since NT, and every menu that has ever felt
 * *snappy* has been somewhere below it. It is expressed in milliseconds in JavaScript rather than
 * as a CSS duration precisely because it is not a design token — a re-seed of the palette must not
 * change how long a person has to hold still.
 */
export const submenuHoverOpenDelay = 300;

/**
 * How long an open submenu survives the pointer leaving its parent item, in milliseconds.
 *
 * The safe triangle below handles a *direct* diagonal; this handles the pause in the middle of one.
 */
export const submenuHoverCloseDelay = 300;

/**
 * How far past a submenu's near edge the safe triangle reaches, in CSS pixels.
 *
 * A pointer aimed at a submenu's first or last row overshoots the edge slightly, so a triangle
 * drawn to the exact corners closes the submenu for the two rows people aim at most.
 */
export const submenuTravelGrace = 12;

/**
 * How long a touch must be held before a context menu opens, in milliseconds.
 *
 * Android's `ViewConfiguration.getLongPressTimeout()` is 500ms and iOS' recogniser is 500ms; a
 * platform that disagreed with both would feel broken on both.
 */
export const longPressDelay = 500;

/** How far a touch may drift and still be a long press, in CSS pixels. Beyond it, it is a scroll. */
export const longPressMoveTolerance = 10;

// ── focus management: the question U04 left open ─────────────────────────────

/**
 * **Trap or disclosure — and the property that decides it.**
 *
 * MJXOFF-183 shipped two popups with two different answers and flagged that an accessibility audit
 * should confirm the asymmetry. This is that answer, and it is a rule rather than a preference:
 *
 * > **The number of tab stops decides.** A surface with **one** tab stop moves with arrow keys, so
 * > `Tab` is free to mean *leave*, and the surface closes when focus leaves it. A surface with
 * > **many** tab stops uses `Tab` as its internal navigation, so `Tab` cannot also mean *leave* —
 * > and a surface a person can Tab out of but not Tab back into has lost them, which is why it
 * > must trap.
 *
 * Under that rule the ARIA menu pattern is *not* a trap and never was: it is explicit that `Tab`
 * moves focus out of a menu and closes it. So `<mjx-menu>` is `roving`, and so is MJXOFF-183's tab
 * picker, which holds one roving tab stop — **the two agree.** The collapsed ribbon group is the
 * one that traps, and it traps because it is a *container of controls* rather than a menu: its
 * commands are real buttons, each separately tabbable, so `Tab` is the only way between them.
 *
 * The asymmetry therefore stands, and it is not an asymmetry between *popups* — it is the same
 * rule applied to two surfaces with different contents.
 */
export const focusManagementPatterns = {
  roving: {
    tabStops: 'one',
    dismissal: 'closes when focus leaves it',
    movement: 'arrow keys, Home, End and type-ahead',
    surfaces: 'a menu, a submenu, a context menu, MJXOFF-183’s ribbon tab picker',
  },
  trap: {
    tabStops: 'many',
    dismissal: 'Escape, or a click outside; focus cannot leave by Tab',
    movement: 'Tab, because the stops are independent controls',
    surfaces:
      'MJXOFF-183’s collapsed ribbon group, whose commands are separately focusable buttons, and ' +
      'MJXOFF-188’s modal dialog, modal sheet and flyout',
  },
  /**
   * **Added by MJXOFF-188, and it is the third answer the two-way rule could not give.**
   *
   * The rule above decides between *trap* and *disclosure* for a surface that has taken the
   * keyboard away from the page. A docked task pane and a modeless dialog have not: the document
   * behind them is still live, still editable and still the reason the surface is open. They have
   * many tab stops, so the rule as written would make them traps — and a trap a person cannot
   * escape from, because a task pane has no Escape, no scrim and no invoker to return to.
   *
   * The resolution is that the rule's premise does not hold: `Tab` is only forced to mean *stay*
   * when leaving would strand the person, and leaving a surface whose background is still live
   * strands nobody. So its stops join the page's own sequence, `Tab` walks out of the far side
   * exactly as it walks out of a `<form>`, and nothing is trapped and nothing is dismissed.
   *
   * **What decides is therefore not the stop count alone but the pair** *(stop count, is the
   * background still reachable)* — and the two-member table was right for every surface that
   * existed when it was written, because all of them had taken the keyboard.
   *
   * ⚠ It is added here rather than restated in `src/surfaces/surface-model.ts` on MJXOFF-188's own
   * instruction — *"extend it rather than restating it"* — and it is purely additive: `roving` and
   * `trap` are untouched, so `tests/menus.test.ts`'s two assertions about them still hold.
   */
  shared: {
    tabStops: 'many',
    dismissal: 'none by the person leaving it; the application decides when the surface goes',
    movement: 'Tab, in the page’s own sequence — the surface shares the keyboard with what is behind it',
    surfaces: 'MJXOFF-188’s task pane and its modeless dialog',
  },
} as const;

/** Which pattern a menu uses. Asserted against the component's actual behaviour by the browser gate. */
export const menuFocusPattern: keyof typeof focusManagementPatterns = 'roving';

// ── events ───────────────────────────────────────────────────────────────────

/** The events a menu emits. Wiring one to a command is loop 2. */
export const menuEvents = {
  /** A menu opened. `detail.presentation` says which shape it took. */
  open: 'mjx-menu-open',
  /** A menu closed. `detail.reason` is one of `menuCloseReasons`. */
  close: 'mjx-menu-close',
  /** An item was activated. `detail` carries `value`, `kind` and `checked`. */
  activate: 'mjx-menu-activate',
  /** A submenu opened or closed. `detail.open` says which, `detail.by` says what did it. */
  submenuToggle: 'mjx-menu-submenu-toggle',
} as const;

/** Why a menu closed — the three the focus-restoration gate distinguishes. */
export const menuCloseReasons = ['escape', 'activate', 'outside', 'blur', 'tab'] as const;

/** One of the five. */
export type MenuCloseReason = (typeof menuCloseReasons)[number];

/** The reasons that must return focus to the invoker. `blur` and `tab` deliberately do not. */
export const focusRestoringCloseReasons: readonly MenuCloseReason[] = [
  'escape',
  'activate',
  'outside',
];

// ── the catalogue contract the gates read ────────────────────────────────────

/** The attribute a states-matrix cell carries. Spelled in the story too — see `matrix.ts`. */
export const menuStateCellAttribute = 'data-menu-state-cell';

/** The story every gate that measures paint opens. */
export const menuStatesMatrixStoryName = 'The States Matrix';

/** The two story titles, as string literals — CSF refuses a computed one. */
export const menuStoryTitles = {
  menu: 'Menus/Menu',
  contextMenu: 'Menus/Context Menu',
} as const;

/** The default side and alignment a menu opens on when nobody says otherwise. */
export const defaultMenuSide: LogicalSide = 'blockEnd';
export const defaultMenuAlign: Align = 'start';
/** A submenu opens toward the end of the line, whichever way the line runs. */
export const submenuSide: LogicalSide = 'inlineEnd';
export const submenuAlign: Align = 'start';

// ── the stylesheets ──────────────────────────────────────────────────────────

/**
 * `<mjx-menu>`'s own rules.
 *
 * The box reads every custom property the presentation blocks write and declares no paint that a
 * presentation could disagree with. The surface is `surfaceLevels.overlay` read out of the
 * elevation ladder — *"a menu, a dialog, a popover: anything drawn over content it must be readable
 * against"* is a rung that already exists, and a second hand-made card is how a design system
 * acquires two menus that do not match.
 */
export const menuCss = `
  :host {
    display: block;
  }
  :host([hidden]) { display: none; }

  /*
   * ⚠ A floating menu is a **top-layer popover**, and it is not a nicety.
   *
   * A submenu is a DOM descendant of the row that opened it, which is a descendant of the parent
   * menu's box — and that box scrolls its own list, so it clips. MJXOFF-184 measured what that
   * costs: the submenu had the right rectangle, was painted nowhere, and elementsFromPoint at its
   * own coordinates returned the parent menu. A pointer could not reach it and a *screenshot could
   * not show it*, which is the exact shape of defect this catalogue's gates exist to catch. Fixed
   * positioning does not help — the clip is applied to the subtree whether or not the containing
   * block escapes it.
   *
   * The top layer is what the platform provides for this, so the box carries a manual popover when
   * it is floating: manual rather than auto, because dismissal, nesting and focus are this
   * component's own and an auto popover would close a parent when a child opened. What follows
   * undoes the UA popover stylesheet — an inset of zero, an automatic margin, a border, a padding
   * and a CanvasText colour — every one of which would otherwise beat what a menu looks like.
   */
  .menu {
    box-sizing: border-box;
    margin: 0;
    padding-inline: 0;
    color: inherit;
    block-size: auto;
    position: var(${menuBoxProperties.position});
    top: var(${menuBoxProperties.insetTop});
    bottom: var(${menuBoxProperties.insetBottom});
    left: var(${menuBoxProperties.insetLeft});
    right: var(${menuBoxProperties.insetRight});
    inline-size: var(${menuBoxProperties.inlineSize});
    min-inline-size: var(${menuBoxProperties.minInlineSize});
    max-inline-size: var(${menuBoxProperties.maxInlineSize});
    max-block-size: var(${menuBoxProperties.maxBlockSize});
    overflow-y: var(${menuBoxProperties.blockOverflow});
    display: flex;
    flex-direction: column;
    /* Deliberately clip rather than hidden, and it is the difference between a working submenu
     * and a menu that jumps sideways when one opens. An overflow of hidden still makes a box a
     * *scroll container*, so when focus moves into a submenu — which is fixed-positioned beside
     * this menu and therefore outside its scrollport — the browser scrolls this menu to "reveal"
     * it, dragging every row of the parent menu 230 pixels to the left while the fixed submenu
     * stays put. The anchor the submenu was placed against then reads as somewhere it never was.
     * Clip is the value that says *do not scroll, ever*, which is what a menu means. */
    overflow-x: clip;
    overscroll-behavior: contain;
    padding-block: 0;
    background: ${overlay.background};
    border: ${overlay.border};
    box-shadow: ${overlay.shadow};
    border-start-start-radius: var(${menuBoxProperties.radiusBlockStart});
    border-start-end-radius: var(${menuBoxProperties.radiusBlockStart});
    border-end-start-radius: var(${menuBoxProperties.radiusBlockEnd});
    border-end-end-radius: var(${menuBoxProperties.radiusBlockEnd});
    z-index: 1;
    translate: 0 0;
    transition-property: translate, opacity;
  }

  /* A popup that is not open is not drawn. An inline menu has no closed state at all, which is why
   * the selector names the floating attribute rather than only the open one. */
  .menu[${floatingAttribute}][${openAttribute}='false'] { display: none; }

  /* An accordion submenu: expanded inside its parent's list, so it is rows rather than a card. */
  .menu[data-accordion][${openAttribute}='false'] { display: none; }
  .menu[data-accordion] {
    inline-size: 100%;
    background: none;
    border: 0;
    box-shadow: none;
    border-radius: 0;
    padding-inline-start: var(${densityProperties.gutter});
  }

  /* One frame of entry, removed on the next. Only the sheet travels: a floating list appears where
   * it was aimed, and a list that slid into place would be a list a pointer arrives before. */
  .menu[data-entering] {
    translate: 0 var(${menuBoxProperties.enterTranslate});
    opacity: 0;
  }
`;

/**
 * `<mjx-menu-item>`'s own rules.
 *
 * ⚠ **`.item` paints nothing.** No background, no border colour or style, no text colour, no
 * weight, no opacity, no shadow — every one of those comes from `menuItemStatesCss()`, whose
 * selectors score (0,0,0). A single `background:` here would score (0,1,0) and out-specify the
 * entire state table, leaving a row that renders its resting paint in all seven states while every
 * "the state exists" check passed. That is MJXOFF-181's specificity accident wearing this child's
 * costume, and `tests/menus.test.ts` asserts the separation rather than trusting this comment.
 */
export const menuItemCss = `
  :host {
    display: block;
  }
  :host([hidden]) { display: none; }

  .item {
    display: grid;
    grid-template-columns:
      var(${menuBoxProperties.markColumn}, 0px)
      var(${menuBoxProperties.iconColumn}, 0px)
      1fr auto auto;
    align-items: center;
    box-sizing: border-box;
    inline-size: 100%;
    margin: 0;
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.step});
    border-width: 1px;
    border-radius: var(--radius-control);
    text-align: start;
    cursor: pointer;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .item[data-unavailable] { cursor: help; }

  .mark,
  .icon {
    display: inline-flex;
    align-items: center;
    justify-content: flex-start;
  }

  .lines {
    display: grid;
    min-inline-size: 0;
    padding-inline-end: var(${densityProperties.gutter});
  }

  .label,
  .description {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* The hint and the description are told apart from the label by *size*. See
   * menuSecondaryTextIsSizeNotColour for the measurement that decided that. */
  .shortcut {
    justify-self: end;
    white-space: nowrap;
    font-variant-numeric: tabular-nums;
  }

  .arrow {
    display: inline-flex;
    align-items: center;
    padding-inline-start: var(${densityProperties.step});
  }

  /* At sheet widths a submenu expands inside the list, so its arrow points at what it opened
   * rather than off the side of a screen the submenu never goes to. */
  @container ${containerName} (width <= ${String(menuSheetAtOrBelow)}px) {
    .item[data-submenu-open] .arrow { rotate: 90deg; }
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

/** `<mjx-menu-separator>` — a rule, and the one place a hairline is not a design value. */
export const menuSeparatorCss = `
  :host { display: block; }
  .separator {
    block-size: 1px;
    margin-block: var(${densityProperties.step});
    margin-inline: var(${densityProperties.gutter});
    background: var(--theme-border-subtle);
  }
`;

/**
 * `<mjx-menu-section>` — a named group of items.
 *
 * The heading is `aria-hidden` because the group already carries the same words in its
 * `aria-label`, and a screen reader that read both would announce *"Paste Options, Paste Options"*
 * on entering every section.
 */
export const menuSectionCss = `
  :host { display: block; }
  .heading {
    margin: 0;
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.step});
    color: var(--theme-text-secondary);
  }
`;

/** `<mjx-context-menu>` — a region that owns a menu. It draws nothing of its own. */
export const contextMenuCss = `
  :host { display: block; }
  :host([hidden]) { display: none; }
  .region {
    display: block;
    /* A long press must not also select the text under the thumb. */
    -webkit-touch-callout: none;
  }
  .region[data-touch] { touch-action: manipulation; user-select: none; }
`;

/** The class a presentation's motion role is applied with, read back from the model. */
export function menuMotionClass(presentation: MenuPresentation): string {
  return motionRoleClass(menuPresentations[presentation].motionRole);
}

/** The type-role class one line of the row wears. */
export function menuTypeClass(line: keyof typeof menuItemTypeRoles): string {
  return typeRoleClass(menuItemTypeRoles[line]);
}

/** The whole sheet a `<mjx-menu>` adopts. */
export const menuSheet = [menuCss, menuPresentationCss()].join('\n');

/** The whole sheet a `<mjx-menu-item>` adopts. */
export const menuItemSheet = [menuItemCss, menuItemStatesCss('.item')].join('\n');

/** The gutter columns a menu publishes to the rows inside it. */
export const menuGutterColumn = glyphColumn;
