/**
 * The mobile vocabulary — **one table, read by the stylesheets, by the two bars and by both
 * gates.**
 *
 * MJXOFF-194 owns the parts of the shell that exist only on a phone, and — more to the point — it
 * is where the phone is judged as a whole rather than component by component. Fourteen children
 * each added a touch presentation to their own components; this one says what the rules were,
 * writes them down as data, and then measures the catalogue against them.
 *
 * ## The four form factors, and why a container query is not enough on its own
 *
 * Every responsive decision in this catalogue so far has been a container query, and that is
 * right: a component must answer to the box it is in and not to the device. But a landscape phone
 * is 844 CSS pixels wide and 390 tall, so by width alone it is a tablet — and a bottom bar laid
 * out as if there were 844 px of room and 900 px of height puts its commands out of thumb reach on
 * the one form factor where reach is tightest. The missing fact is the viewport's shape, which is
 * not a property of any container and cannot be one.
 *
 * So the rule this child adds is narrow and stated: width is a container query, shortness is a
 * media query. A max-height media query is not device sniffing — it reads the viewport's own block
 * size, which is exactly what landscape means. Nothing here branches on a user-agent string, a
 * touch capability or a platform name, and formFactorFor is a pure function of two numbers that
 * the browser gate compares against what the cascade actually decided.
 *
 * ## Node-importable
 *
 * Data, strings and pure functions. No HTMLElement is touched here, so the Playwright specs —
 * which run in Node — can import the same table the components read.
 */

import {
  essentialCommandLimit,
  groupPriorityNames,
  type GroupPriority,
} from '../ribbon/ribbon-model.ts';
import { containerName, phoneShellAtOrBelow } from '../harness/presets.ts';

/** The two elements this child registers. */
export const mobileTags = {
  commandBar: 'mjx-command-bar',
  contextualActionBar: 'mjx-contextual-action-bar',
} as const;

/**
 * The events both bars publish.
 *
 * ⚠ **Here rather than beside the class, and the browser tier is why.** `src/harness/presets.ts`
 * states the rule this catalogue was founded on — *a module that defines a custom element cannot be
 * imported from Node*, because `class extends HTMLElement` is evaluated at module scope and the
 * Playwright specs run in Node. These constants started life in `mobile-bar-element.ts`; the first
 * spec that named one took the whole browser tier down with
 * `ReferenceError: HTMLElement is not defined`, before a single test ran. **Constants a test needs
 * live beside the component, not inside it.**
 */
export const mobileEvents = {
  /** A command was activated. `detail: { id }`. */
  command: 'mjx-mobile-command',
  /** The overflow opened or closed. `detail: { open }`. */
  overflowToggled: 'mjx-mobile-overflow',
  /** The form factor the cascade decided changed. `detail: { formFactor }`. */
  formFactorChanged: 'mjx-mobile-form-factor',
} as const;

/** The overflow control's icon and its name. Node-importable for the same reason. */
export const overflowControl = { icon: 'more-horizontal', label: 'More commands' } as const;

// ── the form factors ─────────────────────────────────────────────────────────

/** The four shapes the shell answers to. */
export const mobileFormFactorNames = [
  'phonePortrait',
  'phoneLandscape',
  'smallTablet',
  'desktop',
] as const;

/** One of the four. */
export type MobileFormFactor = (typeof mobileFormFactorNames)[number];

/**
 * The viewport block size at or below which a viewport is **short**.
 *
 * Every phone in landscape is at or under this: an iPhone 15 Pro Max is 430, a Pixel 8 Pro is 412,
 * and the tallest phone in landscape is still comfortably below 500. The smallest tablet in
 * landscape — an iPad mini at 744 — is comfortably above it. The number is therefore a real
 * discriminator rather than a taste, and it is the only place in this catalogue that reads the
 * viewport rather than a container.
 */
export const shortViewportAtOrBelow = 500;

/**
 * The container width at or below which the shell is at most a small tablet.
 *
 * The harness's tablet preset is 834, so it lands inside this band and a story opened at that
 * preset shows the small-tablet presentation rather than the desktop one. Above it the shell is a
 * desktop and this child's two bars are not what a person should be looking at.
 */
export const smallTabletAtOrBelow = 900;

/**
 * The widest a **short** viewport may be and still be a phone on its side.
 *
 * ⚠ **It is deliberately wider than `smallTabletAtOrBelow`, and the first draft of this file was
 * wrong for exactly that reason.** The largest phone in landscape — an iPhone 15 Pro Max — is 932
 * CSS pixels wide, which is *past* the small-tablet threshold; bounding the landscape branch by
 * that threshold sent the biggest phone on the market to the desktop presentation, which is the one
 * presentation it must never get. The unit test that found it is still there and is named for the
 * device.
 *
 * The ceiling still has to exist, though, or a 1440 x 420 browser window — somebody with a very
 * short desktop — would acquire a phone bar. A thousand is comfortably above every phone and
 * comfortably below any window a person would call a desktop.
 */
export const landscapePhoneInlineAtOrBelow = 1000;

/** The viewport, as the two numbers the decision is made from. */
export interface ViewportShape {
  /** The **container's** inline size — not the window's. */
  readonly inline: number;
  /** The **viewport's** block size, which no container can report. */
  readonly block: number;
}

/**
 * Which form factor a shell of this shape is in.
 *
 * Written as the same cascade the stylesheet emits — desktop, then small tablet, then phone
 * portrait, then phone landscape last — rather than as a chain of comparisons in the opposite
 * order, because a model that agreed with itself and disagreed with the browser is the failure
 * `<mjx-ribbon-group>` documents at length.
 */
export function formFactorFor(shape: ViewportShape): MobileFormFactor {
  let factor: MobileFormFactor = 'desktop';
  if (shape.inline <= smallTabletAtOrBelow) factor = 'smallTablet';
  if (shape.inline <= phoneShellAtOrBelow) factor = 'phonePortrait';
  if (
    shape.block <= shortViewportAtOrBelow &&
    shape.inline > phoneShellAtOrBelow &&
    shape.inline <= landscapePhoneInlineAtOrBelow
  ) {
    factor = 'phoneLandscape';
  }
  return factor;
}

/** What a form factor is, in the words the audit reads. */
export interface FormFactorSpec {
  readonly description: string;
  /** How many commands the bar keeps in its visible run before the rest are ordered behind them. */
  readonly visibleSlots: number;
  /** Whether the bar spans the container or floats clear of its edges. */
  readonly spans: boolean;
  readonly use: string;
}

/**
 * The four, with the one number that changes between them.
 *
 * ⚠ **Landscape gets more slots and less height.** That is not a contradiction: a landscape phone
 * has 844 px of inline room and 390 px of block room, so the scarce axis has swapped. The bar grows
 * sideways and must not grow downward, which is what `commandBarRowUnits` below says.
 */
export const mobileFormFactors: Readonly<Record<MobileFormFactor, FormFactorSpec>> = {
  phonePortrait: {
    description: 'A phone held upright. One scrollable row of commands pinned to the block end.',
    visibleSlots: 4,
    spans: true,
    use: 'The default phone presentation, and the one the reachability rule is written for.',
  },
  phoneLandscape: {
    description:
      'A phone turned sideways. The same bar, wider and shorter: more commands fit across, and ' +
      'there is far less room down the block axis for it to take.',
    visibleSlots: 6,
    spans: true,
    use: 'The form factor everyone forgets, and the one where thumb reach is tightest.',
  },
  smallTablet: {
    description:
      'A small tablet, or a phone-width pane inside a larger window. The bar floats clear of the ' +
      'container edges rather than spanning them, because a full-bleed bar on an 834 px surface ' +
      'is a very long way for one thumb to travel.',
    visibleSlots: 8,
    spans: false,
    use: 'The harness tablet preset, and a split view on a large phone.',
  },
  desktop: {
    description:
      'Not a mobile form factor at all. The bar still renders — a tablet with a mouse is a real ' +
      'configuration — but the ribbon is what a person should be looking at at this width.',
    visibleSlots: 8,
    spans: false,
    use: 'Anything wider than a small tablet.',
  },
};

/**
 * The block extent of a bar row, in spacing units, per form factor.
 *
 * A landscape phone has 390 px of block room and a keyboard may take half of it, so the bar is one
 * hit target and its padding, and nothing else. Portrait can afford the gutter.
 */
export const commandBarRowUnits: Readonly<Record<MobileFormFactor, number>> = {
  phonePortrait: 3,
  phoneLandscape: 1,
  smallTablet: 3,
  desktop: 3,
};

/** The custom property the cascade writes its decision into, and both the bars and the gate read. */
export const mobileFormFactorProperty = '--mjx-mobile-form-factor';

/** The two companions written beside it, so a rule can use the decision without re-querying. */
export const mobileRowUnitsProperty = '--mjx-mobile-row-units';
export const mobileSpansProperty = '--mjx-mobile-spans';

// ── reachability ─────────────────────────────────────────────────────────────

/**
 * The fraction of the viewport's block axis a thumb reaches comfortably, measured from the bottom.
 *
 * **A layout constraint, not a preference**, which is the ticket's own wording. The figure is the
 * conservative end of the published reach studies for a one-handed grip on a large phone: the
 * bottom half of the screen is comfortable, the next tenth is a stretch, and the top 45 % needs a
 * second hand or a grip shift. Anything this catalogue calls a primary action must lie wholly
 * inside the band, and `withinThumbReach` is what says so.
 *
 * `GUESS:` the number is ours. No reach study was run for this project and none is claimed.
 */
export const thumbReachBlockFraction = 0.55;

/**
 * The largest phone the reachability rule is judged on, in CSS pixels.
 *
 * A rule that held on a 390 px-tall phone and failed on a 932 px one would be a rule about small
 * phones. The gate opens the bars at this viewport, which is an iPhone 15 Pro Max in portrait.
 */
export const largePhoneViewport: ViewportShape = { inline: 430, block: 932 };

/** The same device turned sideways, which is where the block axis gets tight. */
export const largePhoneLandscapeViewport: ViewportShape = { inline: 932, block: 430 };

/** A small tablet in portrait — the third presentation, and the one usually forgotten. */
export const smallTabletViewport: ViewportShape = { inline: 834, block: 1112 };

/** A box, in the viewport's own coordinates. */
export interface ReachBox {
  readonly top: number;
  readonly bottom: number;
}

/** The block coordinate at or below which a target is within reach. */
export function thumbReachThreshold(viewport: ViewportShape): number {
  return viewport.block * (1 - thumbReachBlockFraction);
}

/**
 * Whether a target lies wholly inside the reach band.
 *
 * **Wholly**, not partly: half a button inside the band is a button a person aims at and misses,
 * and the half they can reach is the half nearest the edge of the target.
 */
export function withinThumbReach(box: ReachBox, viewport: ViewportShape): boolean {
  return box.top >= thumbReachThreshold(viewport) && box.bottom <= viewport.block;
}

// ── safe-area insets ─────────────────────────────────────────────────────────

/**
 * The four insets, as registered custom properties fed from the environment.
 *
 * ⚠ **The indirection is the point.** A component cannot be tested against the safe-area
 * environment variables in a headless browser: Chromium reports zero on every side and there is no
 * flag that makes it report anything else. So the shell publishes the four environment values into
 * four registered properties once, on the document, and every component reads the property. A
 * notched device feeds them through the environment; the gate feeds them by setting the property,
 * which is the same value arriving down the same channel. Nothing is stubbed and nothing is
 * branched on.
 *
 * Registered as a length, so `resolveLength` returns a number rather than the substituted text —
 * MJXOFF-189's finding, applied rather than rediscovered.
 */
export const safeAreaProperties = {
  blockStart: '--mjx-safe-area-inset-block-start',
  blockEnd: '--mjx-safe-area-inset-block-end',
  inlineStart: '--mjx-safe-area-inset-inline-start',
  inlineEnd: '--mjx-safe-area-inset-inline-end',
} as const;

/** Which environment keyword feeds each. Physical, because the environment has no logical spelling. */
export const safeAreaSources: Readonly<Record<keyof typeof safeAreaProperties, string>> = {
  blockStart: 'safe-area-inset-top',
  blockEnd: 'safe-area-inset-bottom',
  inlineStart: 'safe-area-inset-left',
  inlineEnd: 'safe-area-inset-right',
};

// ── the command bar's contents ───────────────────────────────────────────────

/**
 * A command as the bar receives it.
 *
 * `priority` and `essential` are **U04's**, imported from `ribbon-model.ts` rather than restated:
 * the ticket says read the demotion rules and apply them, do not invent a second priority scheme,
 * and a second four-valued enumeration in this file would have been exactly that. `hasPopup` is how
 * a command declares it fails demotion rule 1.
 */
export interface MobileCommand {
  readonly id: string;
  readonly label: string;
  readonly icon: string;
  /** The ribbon group it came from. The per-group ceiling is counted over this. */
  readonly group: string;
  /** U04's ladder. */
  readonly priority: GroupPriority;
  /** U04's essential slot, as a boolean. */
  readonly essential: boolean;
  /** True for anything that opens a menu, a gallery or a dialog. Demotion rule 1 refuses it. */
  readonly hasPopup: boolean;
}

/** Where a command ended up. */
export interface CommandBarPartition {
  /** The run a person sees without scrolling or opening anything. */
  readonly visible: readonly MobileCommand[];
  /** Everything else — ordered behind the visible run in the **same** rail, never removed. */
  readonly overflow: readonly MobileCommand[];
}

function priorityRank(priority: GroupPriority): number {
  const rank = groupPriorityNames.indexOf(priority);
  return rank === -1 ? groupPriorityNames.length : rank;
}

/**
 * **U04's demotion order, applied to a flat row.**
 *
 * The ribbon's ladder decides which group gives way first; a phone bar has no groups on screen, so
 * the same ladder decides which group's commands are nearest the thumb. Within a priority the
 * commands a group marked essential come first, because that is precisely U04's judgement about
 * which of a group's commands survive a collapse — and within that, declaration order, which is the
 * author's own.
 *
 * The sort is **stable** and the key is a triple, so the result is fully determined by the input.
 * `tests/mobile.test.ts` recomputes it with an independent selection sort and requires the two to
 * agree.
 */
export function commandBarOrder(commands: readonly MobileCommand[]): readonly MobileCommand[] {
  return [...commands]
    .map((command, index) => ({ command, index }))
    .sort((left, right) => {
      const byPriority = priorityRank(left.command.priority) - priorityRank(right.command.priority);
      if (byPriority !== 0) return byPriority;
      const byEssential = Number(right.command.essential) - Number(left.command.essential);
      if (byEssential !== 0) return byEssential;
      return left.index - right.index;
    })
    .map((entry) => entry.command);
}

/**
 * Split the ordered row into what is seen and what is one gesture away.
 *
 * Three of U04's four rules bind here and the fourth is structural:
 *
 * 1. **Immediate and reversible.** A command carrying `hasPopup` is never in the visible run,
 *    whatever its priority — a popup opening from a bar pinned to the bottom edge of a phone is the
 *    popup-inside-a-popup U04 refuses. It keeps its place in the order and is reached by scrolling
 *    or by opening the overflow.
 * 2. **Recognisable with no label.** A judgement, stated as one. What is checkable is that every
 *    command has an icon and a name, and `commandBarUnnameable` reports the ones that do not.
 * 3. **At most three per group.** `essentialCommandLimit`, counted over `group`.
 * 4. **Nothing is ever removed.** Structural: visible and overflow partition the input, and the bar
 *    renders both into **one** rail, so the DOM nodes are the same at every form factor.
 */
export function commandBarPartition(
  commands: readonly MobileCommand[],
  visibleSlots: number,
): CommandBarPartition {
  const ordered = commandBarOrder(commands);
  const visible: MobileCommand[] = [];
  const overflow: MobileCommand[] = [];
  const perGroup = new Map<string, number>();

  for (const command of ordered) {
    if (visible.length >= visibleSlots || command.hasPopup) {
      overflow.push(command);
      continue;
    }
    const taken = perGroup.get(command.group) ?? 0;
    if (taken >= essentialCommandLimit) {
      overflow.push(command);
      continue;
    }
    perGroup.set(command.group, taken + 1);
    visible.push(command);
  }

  return { visible, overflow };
}

/**
 * The partition a bar would have if it simply took the first commands as declared.
 *
 * **Shipped beside the real one on purpose**, exactly as `stackMarginCards` is shipped beside
 * `packMarginCards`. A gate that only asserted the real answer satisfies its own rules would be
 * satisfied by an implementation that ignored the ladder entirely on an input where declaration
 * order happens to match it. This is the answer that ignores it, and `tests/mobile.test.ts`
 * requires it to be **worse** on the catalogue's own fixture — and requires the fixture to be one
 * where the two can differ at all.
 */
export function naiveCommandBarPartition(
  commands: readonly MobileCommand[],
  visibleSlots: number,
): CommandBarPartition {
  return {
    visible: commands.slice(0, visibleSlots),
    overflow: commands.slice(visibleSlots),
  };
}

/**
 * How badly a partition disagrees with the ladder: the number of pairs in the visible run whose
 * priorities are the wrong way round, plus one for every rule the run breaks outright.
 *
 * Zero is the ladder's own answer. It is a measurement rather than a verdict so a test can say that
 * one partition is worse than another rather than merely that it is not equal.
 */
export function demotionCost(partition: CommandBarPartition): number {
  let cost = 0;
  const visible = partition.visible;
  for (let i = 0; i < visible.length; i += 1) {
    for (let j = i + 1; j < visible.length; j += 1) {
      const left = visible[i];
      const right = visible[j];
      if (left === undefined || right === undefined) continue;
      if (priorityRank(left.priority) > priorityRank(right.priority)) {
        cost += 1;
      } else if (
        priorityRank(left.priority) === priorityRank(right.priority) &&
        !left.essential &&
        right.essential
      ) {
        cost += 1;
      }
    }
  }
  const perGroup = new Map<string, number>();
  for (const command of visible) {
    if (command.hasPopup) cost += 1;
    const taken = (perGroup.get(command.group) ?? 0) + 1;
    perGroup.set(command.group, taken);
    if (taken > essentialCommandLimit) cost += 1;
  }
  return cost;
}

/** Commands the bar cannot draw without a label: no icon, or no accessible name. */
export function commandBarUnnameable(commands: readonly MobileCommand[]): readonly MobileCommand[] {
  return commands.filter((command) => command.icon.trim() === '' || command.label.trim() === '');
}

// ── the contextual action bar ────────────────────────────────────────────────

/** What the canvas says is selected. */
export const selectionKindNames = ['none', 'text', 'object', 'cells'] as const;

/** One of the four. */
export type SelectionKind = (typeof selectionKindNames)[number];

/**
 * The actions every selection offers, whatever it is.
 *
 * Declared once and spliced into each kind, so a new selection kind cannot forget them and the
 * catalogue cannot acquire two spellings of *copy*.
 */
export const sharedSelectionCommands: readonly MobileCommand[] = [
  {
    id: 'cut',
    label: 'Cut',
    icon: 'cut',
    group: 'Clipboard',
    priority: 'primary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'copy',
    label: 'Copy',
    icon: 'copy',
    group: 'Clipboard',
    priority: 'primary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'paste',
    label: 'Paste',
    icon: 'clipboard-paste',
    group: 'Clipboard',
    priority: 'primary',
    essential: true,
    hasPopup: false,
  },
  {
    id: 'delete',
    label: 'Delete',
    icon: 'delete',
    group: 'Edit',
    priority: 'standard',
    essential: true,
    hasPopup: false,
  },
];

/** The selection-specific half, before the shared four are spliced in. */
export const selectionSpecificCommands: Readonly<Record<SelectionKind, readonly MobileCommand[]>> = {
  none: [],
  text: [
    {
      id: 'bold',
      label: 'Bold',
      icon: 'text-bold',
      group: 'Font',
      priority: 'primary',
      essential: true,
      hasPopup: false,
    },
    {
      id: 'italic',
      label: 'Italic',
      icon: 'text-italic',
      group: 'Font',
      priority: 'primary',
      essential: true,
      hasPopup: false,
    },
    {
      id: 'underline',
      label: 'Underline',
      icon: 'text-underline',
      group: 'Font',
      priority: 'primary',
      essential: false,
      hasPopup: false,
    },
    {
      id: 'align',
      label: 'Alignment',
      icon: 'text-align-left',
      group: 'Paragraph',
      priority: 'standard',
      essential: false,
      hasPopup: true,
    },
    {
      id: 'comment',
      label: 'New comment',
      icon: 'comment',
      group: 'Review',
      priority: 'secondary',
      essential: false,
      hasPopup: false,
    },
  ],
  object: [
    {
      id: 'fill',
      label: 'Shape fill',
      icon: 'slide-layout',
      group: 'Shape',
      priority: 'primary',
      essential: false,
      hasPopup: true,
    },
    {
      id: 'bring-forward',
      label: 'Bring forward',
      icon: 'chevron-up',
      group: 'Arrange',
      priority: 'standard',
      essential: true,
      hasPopup: false,
    },
    {
      id: 'send-backward',
      label: 'Send backward',
      icon: 'chevron-down',
      group: 'Arrange',
      priority: 'standard',
      essential: true,
      hasPopup: false,
    },
    {
      id: 'alt-text',
      label: 'Alternative text',
      icon: 'info',
      group: 'Accessibility',
      priority: 'secondary',
      essential: false,
      hasPopup: true,
    },
  ],
  cells: [
    {
      id: 'insert-cells',
      label: 'Insert',
      icon: 'add',
      group: 'Cells',
      priority: 'primary',
      essential: true,
      hasPopup: false,
    },
    {
      id: 'table-style',
      label: 'Table styles',
      icon: 'table',
      group: 'Table',
      priority: 'standard',
      essential: false,
      hasPopup: true,
    },
    {
      id: 'align-cells',
      label: 'Align centre',
      icon: 'text-align-center',
      group: 'Alignment',
      priority: 'standard',
      essential: true,
      hasPopup: false,
    },
  ],
};

/**
 * Everything a selection of this kind offers, shared four first.
 *
 * `none` returns nothing at all, which is the state in which the contextual bar must not be on
 * screen — the command bar is. That is a behaviour rather than an emptiness, and the component
 * asserts it by hiding itself.
 */
export function contextualActions(kind: SelectionKind): readonly MobileCommand[] {
  if (kind === 'none') return [];
  return [...sharedSelectionCommands, ...selectionSpecificCommands[kind]];
}

// ── the presentation stylesheet's own conditions ─────────────────────────────

/**
 * The four presentation blocks, generated from the table above.
 *
 * Emitted in the order the model evaluates: desktop, small tablet, phone portrait, phone landscape.
 * Every selector is inside a `:where()` so all four score (0,0,0) and **source order is the only
 * thing deciding** — MJXOFF-183's specificity accident, avoided by construction rather than
 * rediscovered.
 */
export function mobilePresentationCss(selector: string): string {
  const declare = (factor: MobileFormFactor, indent: string): string =>
    `${indent}${mobileFormFactorProperty}: ${factor};\n` +
    `${indent}${mobileRowUnitsProperty}: ${String(commandBarRowUnits[factor])};\n` +
    `${indent}${mobileSpansProperty}: ${mobileFormFactors[factor].spans ? '1' : '0'};`;

  const where = `:where(${selector})`;
  return [
    `${where} {\n${declare('desktop', '  ')}\n}`,
    `@container ${containerName} (width <= ${String(smallTabletAtOrBelow)}px) {\n` +
      `  ${where} {\n${declare('smallTablet', '    ')}\n  }\n}`,
    `@container ${containerName} (width <= ${String(phoneShellAtOrBelow)}px) {\n` +
      `  ${where} {\n${declare('phonePortrait', '    ')}\n  }\n}`,
    `@media (max-height: ${String(shortViewportAtOrBelow)}px) {\n` +
      `  @container ${containerName} (${String(phoneShellAtOrBelow)}px < width <= ${String(landscapePhoneInlineAtOrBelow)}px) {\n` +
      `    ${where} {\n${declare('phoneLandscape', '      ')}\n    }\n  }\n}`,
  ].join('\n\n');
}

/** Whether a value names a form factor. */
export function isMobileFormFactor(value: unknown): value is MobileFormFactor {
  return (mobileFormFactorNames as readonly string[]).includes(String(value));
}

/** Whether a value names a selection kind. */
export function isSelectionKind(value: unknown): value is SelectionKind {
  return (selectionKindNames as readonly string[]).includes(String(value));
}
