/**
 * The surface vocabulary — **one table, read by the stylesheets, by the three components and by
 * both gates.**
 *
 * Six surfaces, three elements. A dialog, a modal, a sheet, a popover, a flyout and a task pane are
 * not six components: they are six rows of `surfaceKinds`, and the rows differ in the four things
 * that actually differ — *who holds the keyboard*, *how it can be dismissed*, *where it is put*,
 * and *what it sits on*. `<mjx-dialog>` renders three of the rows, `<mjx-popover>` two and
 * `<mjx-task-pane>` one.
 *
 * ## The three decisions this file exists to make once
 *
 * ### 1. A sheet is a dialog at a phone's width, and there is one implementation of it
 *
 * MJXOFF-188 states the failure it is guarding against: *"A sheet and a dialog are the same surface
 * at two sizes — U05 and U06 both build sheets at the phone width. Three implementations of a sheet
 * would be the failure this child exists to prevent."* So `<mjx-dialog>` reads its presentation
 * back out of `--mjx-surface-presentation`, exactly as `<mjx-ribbon-group>`, `<mjx-menu>` and
 * `<mjx-gallery>` do, and pins with `pinFloating` — the same call the menu's sheet and the
 * gallery's sheet make. There is no fourth pinning arithmetic and no second threshold:
 * `phoneShellAtOrBelow` is the number, hoisted into `src/harness/presets.ts` by U06 for this exact
 * reason.
 *
 * ### 2. The scrim, the modal's edge and the sheet's handle are **measured**, never declared
 *
 * MJXOFF-269 is the standing record of what happens when a gate compares code to a model and the
 * *model* is what is wrong. U08's remedy is the one applied here: state the **rule** and the
 * **candidates**, and let the gate sweep and assert the outcome. Three sites, one rule:
 *
 * | Site | What it must be told apart from | Who controls that colour |
 * |---|---|---|
 * | the modal's edge | the scrim, composited over *any* application content | nobody |
 * | the task pane's edge | the document page beside it | the person editing the document |
 * | the sheet's grab handle | the sheet's own fill | this catalogue |
 *
 * The first two are indicators over a surface we do not control, and the third is not — which is
 * why the handle is chosen by a *different* half of the same rule. See `chooseIndicator`.
 *
 * ### 3. A task pane is the one surface that is not dismissible
 *
 * MJXOFF-188 again: *"a gate that treats it like the others will assert the wrong thing."* So
 * `dismissals` is a per-kind list rather than a boolean, the task pane's is **empty**, and
 * `tests/browser/surfaces.spec.ts` proves the emptiness by pressing Escape at it and requiring it
 * to still be open — an assertion that is the *opposite* of the one every other surface gets, and
 * that a shared "every surface closes on Escape" sweep would have silently inverted.
 *
 * ## Node-importable
 *
 * Data, arithmetic and strings. No custom element is defined here, so `tests/surfaces.test.ts` can
 * sweep the indicator rule in Node and `tests/browser/surfaces.spec.ts` can import the model it is
 * comparing a browser against.
 */

import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import {
  contrastRatioOrWorst,
  formatHexColor,
  formatRatio,
  nonTextMinimum,
  parseHexColor,
  relativeLuminance,
} from '../tokens/contrast.ts';
import { customPropertyCase, type ThemeMember } from '../tokens/resolver.ts';
import {
  accessibleHitTargetMinimum,
  densityProperties,
  spacingMultiple,
} from '../foundations/density.ts';
import { motionRoleClass, type MotionRole } from '../foundations/motion.ts';
import {
  radiusVariable,
  surfaceBackgroundMember,
  surfaceLevels,
  type RadiusStep,
  type SurfaceLevel,
} from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { containerName, phoneShellAtOrBelow } from '../harness/presets.ts';
import { focusManagementPatterns } from '../menus/menu-model.ts';
import { floatingProperties, type Align, type LogicalSide } from '../overlay/floating.ts';

// ── the three elements ───────────────────────────────────────────────────────

/** The custom element names, so a gate never spells one itself. */
export const surfaceTags = {
  dialog: 'mjx-dialog',
  popover: 'mjx-popover',
  taskPane: 'mjx-task-pane',
} as const;

/** The three story titles, as string literals — CSF refuses a computed one. */
export const surfaceStoryTitles = {
  dialog: 'Surfaces/Dialog',
  popover: 'Surfaces/Popover',
  taskPane: 'Surfaces/Task Pane',
} as const;

// ── the six surfaces ─────────────────────────────────────────────────────────

/** The six, in the order the catalogue lists them. */
export const surfaceKindNames = [
  'dialog',
  'modal',
  'sheet',
  'popover',
  'flyout',
  'taskPane',
] as const;

/** One of the six. */
export type SurfaceKind = (typeof surfaceKindNames)[number];

/** How a surface is put where it is. */
export type SurfacePlacementKind = 'centred' | 'anchored' | 'pinned' | 'docked';

/** Every way a surface can be closed. `programmatic` is the application changing its mind. */
export const surfaceCloseReasons = [
  'escape',
  'closeButton',
  'scrim',
  'outside',
  'blur',
  'programmatic',
] as const;

/** One of the six. */
export type SurfaceCloseReason = (typeof surfaceCloseReasons)[number];

/** Which of U05's focus patterns a surface follows. Its names come from `menu-model.ts`. */
export type SurfaceFocusPattern = keyof typeof focusManagementPatterns;

/** Everything a kind fixes. */
export interface SurfaceKindSpec {
  /** The element that renders it. */
  readonly tag: (typeof surfaceTags)[keyof typeof surfaceTags];
  /**
   * Whether the surface takes the keyboard away from the page.
   *
   * **This is the field that decides `focus`**, together with the stop count — see the note on
   * `focusManagementPatterns.shared`. A modal holds the background inert; nothing else does.
   */
  readonly modal: boolean;
  /** How it is put where it is. */
  readonly placement: SurfacePlacementKind;
  /** The rung of the elevation ladder it paints with. */
  readonly level: SurfaceLevel;
  /** The corner treatment. A sheet squares off the edge it is pinned to. */
  readonly radius: RadiusStep;
  /** The motion vocabulary it wears. */
  readonly motionRole: MotionRole;
  /** U05's pattern, applied. */
  readonly focus: SurfaceFocusPattern;
  /** Every way a person may close it. **Empty is a real answer** — see the task pane. */
  readonly dismissals: readonly SurfaceCloseReason[];
  /** Whether opening it dims what is behind. */
  readonly scrim: boolean;
  readonly use: string;
}

/**
 * The six.
 *
 * ⚠ **Read the `focus` column against the `modal` column, not on its own.** U05's rule is *the
 * number of tab stops decides*, and every surface here except a popover has many — yet only three
 * of them trap. The missing half of the rule is whether the background is still reachable, which
 * MJXOFF-188 added to `focusManagementPatterns` as `shared`: a modeless dialog and a task pane have
 * many stops and do **not** trap, because Tab leaving them strands nobody. A trap is what a surface
 * owes a person whose way back it has taken away.
 */
export const surfaceKinds: Readonly<Record<SurfaceKind, SurfaceKindSpec>> = {
  dialog: {
    tag: surfaceTags.dialog,
    modal: false,
    placement: 'centred',
    level: 'overlay',
    radius: 'panel',
    motionRole: 'sheetEnter',
    focus: 'shared',
    dismissals: ['escape', 'closeButton', 'programmatic'],
    scrim: false,
    use: 'Find and Replace, a floating inspector: the document behind it is still being edited.',
  },
  modal: {
    tag: surfaceTags.dialog,
    modal: true,
    placement: 'centred',
    level: 'overlay',
    radius: 'panel',
    motionRole: 'sheetEnter',
    focus: 'trap',
    dismissals: ['escape', 'closeButton', 'scrim', 'programmatic'],
    scrim: true,
    use: 'A choice that must be made before anything else happens: Save changes, Paste Special.',
  },
  sheet: {
    tag: surfaceTags.dialog,
    modal: true,
    placement: 'pinned',
    level: 'overlay',
    // A sheet is pinned to an edge, and rounding away from an edge there is nothing behind is how a
    // sheet ends up looking like a floating card that slipped. The block-end corners are squared by
    // the stylesheet; this is the radius the other two corners take.
    radius: 'panel',
    motionRole: 'sheetEnter',
    focus: 'trap',
    dismissals: ['escape', 'closeButton', 'scrim', 'programmatic'],
    scrim: true,
    use: 'The same dialog at a phone width. Reached with a thumb, so it lives at the bottom.',
  },
  popover: {
    tag: surfaceTags.popover,
    modal: false,
    placement: 'anchored',
    level: 'overlay',
    radius: 'card',
    motionRole: 'surfaceSettle',
    focus: 'roving',
    dismissals: ['escape', 'closeButton', 'outside', 'blur', 'programmatic'],
    scrim: false,
    use: 'A definition, a hint, a one-control disclosure hung off the thing it explains.',
  },
  flyout: {
    tag: surfaceTags.popover,
    modal: false,
    placement: 'anchored',
    level: 'overlay',
    radius: 'panel',
    motionRole: 'panelEnter',
    focus: 'trap',
    dismissals: ['escape', 'closeButton', 'outside', 'programmatic'],
    scrim: false,
    use: 'A panel of independent controls hung off a button — MJXOFF-183’s collapsed ribbon group.',
  },
  taskPane: {
    tag: surfaceTags.taskPane,
    modal: false,
    placement: 'docked',
    level: 'floating',
    radius: 'panel',
    motionRole: 'panelEnter',
    focus: 'shared',
    // ⚠ **Empty, and it is the whole point of this row.** A task pane is docked, resizable and
    // persistent: Escape does nothing to it, there is no scrim to click, and losing focus is what
    // happens every time a person types in the document. The application closes it, and nothing
    // else does.
    dismissals: [],
    scrim: false,
    use: 'Format Shape, Styles, Navigation: docked beside the document and still there tomorrow.',
  },
};

/** Whether a value names one of the six. */
export function isSurfaceKind(value: unknown): value is SurfaceKind {
  return (surfaceKindNames as readonly string[]).includes(String(value));
}

/** Whether a kind may be closed for a given reason. The one call a dismissal path makes. */
export function dismissible(kind: SurfaceKind, reason: SurfaceCloseReason): boolean {
  return surfaceKinds[kind].dismissals.includes(reason);
}

/** The kinds a person can never dismiss. A list, because a gate asserting emptiness needs one. */
export const persistentSurfaceKinds: readonly SurfaceKind[] = surfaceKindNames.filter(
  (kind) => surfaceKinds[kind].dismissals.length === 0,
);

// ── the presentations a dialog takes ─────────────────────────────────────────

/**
 * The width at or below which a dialog becomes a sheet.
 *
 * An alias of `phoneShellAtOrBelow`, which is the fourth surface to need it. U06 hoisted it on
 * U05's instruction — *"when a third surface needs it, hoist it; do not add a second definition"* —
 * and `tests/surfaces.test.ts` asserts this is still an alias rather than a number that drifted.
 */
export const dialogSheetAtOrBelow = phoneShellAtOrBelow;

/**
 * Which presentation a dialog is in, from the container width and whether it is modal.
 *
 * **This is the second, independent statement of a rule CSS also states**, exactly as
 * `<mjx-ribbon-group>`, `<mjx-menu>` and `<mjx-gallery>` do it: the component reads
 * `--mjx-surface-presentation` back out of the cascade and the gate compares the two. A component
 * that decided its own presentation and then reported it would be grading its own homework.
 */
export function dialogPresentationAt(containerWidth: number, modal: boolean): SurfaceKind {
  if (modal && containerWidth <= dialogSheetAtOrBelow) return 'sheet';
  return modal ? 'modal' : 'dialog';
}

/** The custom property every presentation block writes, and the component reads back. */
export const surfacePresentationProperty = '--mjx-surface-presentation';

/** How much of the boundary a sheet may cover before it scrolls. */
export const sheetBoundaryFraction = 0.7;

/** The fraction of the boundary a task pane takes when nobody has resized it. */
export const taskPaneDefaultFraction = 0.32;

/** The narrowest and widest a task pane may be dragged, as fractions of its boundary. */
export const taskPaneFractionBounds = { min: 0.2, max: 0.6 } as const;

/** One press of an arrow key on the splitter, as a fraction of the boundary. */
export const taskPaneResizeStep = 0.02;

/**
 * How a pointer drag is turned into a fraction. Pure, so a Node test can drive a whole drag. */
export function fractionFromDrag(
  pointerInline: number,
  boundary: { readonly start: number; readonly size: number },
  side: 'left' | 'right',
): number {
  if (boundary.size <= 0) return taskPaneDefaultFraction;
  const fromStart = (pointerInline - boundary.start) / boundary.size;
  const raw = side === 'right' ? 1 - fromStart : fromStart;
  return clampFraction(raw);
}

/** Keep a fraction inside the bounds a pane may take. */
export function clampFraction(value: number): number {
  if (!Number.isFinite(value)) return taskPaneDefaultFraction;
  return Math.min(Math.max(value, taskPaneFractionBounds.min), taskPaneFractionBounds.max);
}

/**
 * What one key does to the pane's share of the screen.
 *
 * ⚠ **The arrow that grows the pane depends on which edge it is docked to**, and on the writing
 * direction, for the same reason `<mjx-menu>`'s submenu arrows mirror: a pane docked at the end of
 * the line is grown by the arrow that points toward the start of it, and *which* arrow that is
 * changes in Arabic. Returning the fraction rather than the delta is what lets `Home` and `End`
 * share the function.
 */
export function fractionFromKey(
  key: string,
  current: number,
  side: 'left' | 'right',
): number | undefined {
  const grow = side === 'right' ? 'ArrowLeft' : 'ArrowRight';
  const shrink = side === 'right' ? 'ArrowRight' : 'ArrowLeft';
  switch (key) {
    case grow:
      return clampFraction(current + taskPaneResizeStep);
    case shrink:
      return clampFraction(current - taskPaneResizeStep);
    case 'Home':
      return taskPaneFractionBounds.min;
    case 'End':
      return taskPaneFractionBounds.max;
    default:
      return undefined;
  }
}

/** Which edge a task pane docks to. Logical, so RTL is `physicalSide()` and not a second path. */
export const taskPaneDockEdges: readonly LogicalSide[] = ['inlineEnd', 'inlineStart'];

/** Where a popover opens when nobody says otherwise. */
export const defaultPopoverSide: LogicalSide = 'blockEnd';
export const defaultPopoverAlign: Align = 'start';

// ── the surface stack ────────────────────────────────────────────────────────

/**
 * **Stacking is the invisible one**, and this is the model that makes it assertable.
 *
 * MJXOFF-188 names it: *"A popover opened from a dialog, a menu opened from a task pane — assert
 * the order is right and that closing the inner one does not close the outer."* Both halves are
 * properties of a *sequence* of opens and closes rather than of any one of them, which is exactly
 * the shape U06's `PreviewSession` was written for, and for the same reason: a browser test can
 * drive one sequence, and an invariant over all sequences needs a machine a Node test can hammer.
 *
 * The invariant, stated once:
 *
 * > **Closing a surface closes exactly its own descendants and nothing else, and the topmost
 * > surface is always the most recently opened one that is still open.**
 *
 * The stack is deliberately *not* the top layer. The browser's top layer already orders what is
 * painted, and re-deriving that here would be a second opinion about something the platform has
 * already decided. What this holds is the **ownership tree** — which surface opened which — because
 * that is what decides where Escape goes and what a close takes with it, and no browser knows it.
 */
export interface SurfaceStackEntry {
  readonly id: string;
  readonly kind: SurfaceKind;
  /** The id of the surface this one was opened from, when it was opened from one. */
  readonly parent: string | undefined;
}

export class SurfaceStack {
  #entries: SurfaceStackEntry[] = [];

  /** Every open surface, oldest first. */
  get entries(): readonly SurfaceStackEntry[] {
    return this.#entries;
  }

  /** How many are open. */
  get depth(): number {
    return this.#entries.length;
  }

  /** The most recently opened surface that is still open. */
  get topmost(): SurfaceStackEntry | undefined {
    return this.#entries[this.#entries.length - 1];
  }

  /** Whether an id is open. */
  has(id: string): boolean {
    return this.#entries.some((entry) => entry.id === id);
  }

  /**
   * Open one.
   *
   * Re-opening an id that is already open **moves** it to the top rather than adding a second
   * entry: a component that re-rendered while open would otherwise push a duplicate, and the
   * duplicate would survive its own close.
   */
  push(entry: SurfaceStackEntry): void {
    this.#entries = this.#entries.filter((existing) => existing.id !== entry.id);
    this.#entries.push(entry);
  }

  /**
   * Close one, and report **everything that closed** — it and its descendants, innermost first.
   *
   * Innermost first is the order a caller must fire close events in, and it is not cosmetic: an
   * outer surface's close handler may restore focus, and doing that before its children have let go
   * of theirs puts focus somewhere a child is about to take it back from.
   */
  remove(id: string): SurfaceStackEntry[] {
    if (!this.has(id)) return [];
    const closing = new Set<string>([id]);
    // One forward pass is enough: an entry's parent is always earlier in the array, because it was
    // open before this one was pushed.
    for (const entry of this.#entries) {
      if (entry.parent !== undefined && closing.has(entry.parent)) closing.add(entry.id);
    }
    const removed = this.#entries.filter((entry) => closing.has(entry.id)).reverse();
    this.#entries = this.#entries.filter((entry) => !closing.has(entry.id));
    return removed;
  }

  /** Everything, innermost first. What a shell calls when a document closes under its surfaces. */
  clear(): SurfaceStackEntry[] {
    const removed = [...this.#entries].reverse();
    this.#entries = [];
    return removed;
  }
}

/**
 * The one stack the three components share.
 *
 * A module-level singleton, because there is one keyboard and one top layer per document, and two
 * stacks would each be right about half of it.
 */
export const openSurfaces = new SurfaceStack();

// ── the events ───────────────────────────────────────────────────────────────

/** The events all three elements emit. One vocabulary; the `kind` in the detail says which. */
export const surfaceEvents = {
  /** A surface opened. `detail.kind` says which of the six. */
  open: 'mjx-surface-open',
  /** A surface closed. `detail.reason` is one of `surfaceCloseReasons`. */
  close: 'mjx-surface-close',
  /** A task pane was resized. `detail.fraction` is its share of the boundary. */
  resize: 'mjx-surface-resize',
} as const;

// ── the indicator rule, which is measured rather than declared ───────────────

/**
 * How much of the scrim's own colour reaches the eye, as a percentage.
 *
 * ⚠ **Not a taste, and not a number copied from another design system.** It is the smallest value
 * with margin at which `chooseIndicator` can find a sufficient edge in *both* schemes, and the
 * sweep in `tests/surfaces.test.ts` is what says so: at 50 per cent nothing clears 3 : 1 in either
 * scheme, at 55 it clears by four hundredths in light, and this is the first step above that with
 * room in it. Lowering it is a legitimate design change; lowering it without re-running the sweep
 * is how a modal stops having a perceivable boundary in one scheme only.
 */
export const scrimOpacityPercent = 65;

/** The same, as the fraction the compositing arithmetic uses. */
export const scrimAlpha = scrimOpacityPercent / 100;

/**
 * The scheme's darkest colour, **derived rather than chosen**.
 *
 * A scrim says *what is behind this is unavailable*, and the way it says it is by darkening. Which
 * token is the scheme's darkest is not the same answer in both schemes — it is the text colour in
 * light and the application background in dark, because the two palettes are inversions of each
 * other — and writing either name down would be a value this file was told not to write. So the
 * rule is *the darkest member*, and a re-seed that moves the palette moves the scrim with it.
 *
 * ⚠ An ink scrim in the dark scheme would **lighten** a dark application rather than darkening it,
 * which is the specific defect this derivation exists to make impossible.
 */
export function darkestThemeMember(scheme: ColorScheme): ThemeMember {
  const palette = tokens.theme[scheme];
  let best: { member: ThemeMember; luminance: number } | undefined;
  for (const [member, value] of Object.entries(palette)) {
    const luminance = relativeLuminance(String(value));
    if (luminance === undefined) continue;
    if (best === undefined || luminance < best.luminance) {
      best = { member: member as ThemeMember, luminance };
    }
  }
  // Unreachable while the generated palette has a colour in it; the fallback exists because a type
  // checker cannot know that and a non-null assertion would be a lie about why.
  return best?.member ?? 'textPrimary';
}

/** What the scrim is painted in, for one scheme. */
export function scrimColor(scheme: ColorScheme): string {
  return tokens.theme[scheme][darkestThemeMember(scheme) as keyof (typeof tokens.theme)['light']];
}

/**
 * One colour composited over another at an alpha, in sRGB, the way a browser does it.
 *
 * Straight alpha over an opaque backdrop, per channel, in the gamma-encoded space — which is what
 * `color-mix(in srgb, C <p>%, transparent)` produces and what the compositor then blends. Doing it
 * in a linear space would give a different, prettier answer that no browser will ever paint, and a
 * gate measuring against it would disagree with the screen.
 */
export function compositeOver(backdrop: string, front: string, alpha: number): string {
  const under = parseHexColor(backdrop);
  const over = parseHexColor(front);
  if (under === undefined || over === undefined) return backdrop;
  return formatHexColor({
    red: under.red * (1 - alpha) + over.red * alpha,
    green: under.green * (1 - alpha) + over.green * alpha,
    blue: under.blue * (1 - alpha) + over.blue * alpha,
  });
}

/**
 * The two ends of what a scrim can composite to, over **any** application content.
 *
 * A composite channel is `(1 - alpha) x backdrop + alpha x scrim`, which is increasing in the
 * backdrop channel; relative luminance is increasing in every channel; so the luminance of the
 * composite over an arbitrary colour lies between the luminance of the composite over black and
 * the composite over white, and every value in between is reached by some grey. **The band is
 * therefore exactly two colours**, and a component may compute its edge in constant time instead of
 * sweeping four thousand of them at paint time.
 *
 * ⚠ That reasoning is an argument, and an argument is not a gate. `tests/surfaces.test.ts` runs
 * the closed form against the whole 4,352-colour sweep and requires them to agree to a hundredth,
 * so the day the arithmetic stops being monotone the gate says so rather than the screen.
 */
export interface ScrimBand {
  readonly darkest: string;
  readonly lightest: string;
}

/**
 * The two ends of the sRGB cube, **built rather than typed.**
 *
 * `mjx/no-literal-design-values` refuses a hex literal anywhere in a component and it is right to:
 * a hex in `src/` is invisible while it happens to match a token. These two are not colours a
 * component paints with — they are the extreme *arguments* the band arithmetic is evaluated at —
 * and constructing them from their channels says so at the site instead of arguing with a rule that
 * cannot tell the difference. U08's pickers ship no colours at all for the same reason, and its
 * grep for a hex under `src/pickers/` is the stronger form of this rule; `tests/surfaces.test.ts`
 * runs the same grep over `src/surfaces/`.
 */
const cubeExtremes = {
  black: formatHexColor({ red: 0, green: 0, blue: 0 }),
  white: formatHexColor({ red: 255, green: 255, blue: 255 }),
} as const;

export function scrimBand(scheme: ColorScheme, alpha = scrimAlpha): ScrimBand {
  const scrim = scrimColor(scheme);
  return {
    darkest: compositeOver(cubeExtremes.black, scrim, alpha),
    lightest: compositeOver(cubeExtremes.white, scrim, alpha),
  };
}

/**
 * The worst contrast a colour has against anything the band can produce.
 *
 * A colour whose luminance falls *inside* the band is matched exactly by some backdrop, so its
 * worst case is `1` — the ratio a pair of identical colours has, and the honest answer for an edge
 * that some application content will make invisible. Outside the band, the worst case is at
 * whichever end is nearer.
 */
export function worstAgainstBand(colour: string, band: ScrimBand): number {
  const own = relativeLuminance(colour);
  const low = relativeLuminance(band.darkest);
  const high = relativeLuminance(band.lightest);
  if (own === undefined || low === undefined || high === undefined) return 1;
  if (own >= Math.min(low, high) && own <= Math.max(low, high)) return 1;
  return Math.min(
    contrastRatioOrWorst(colour, band.darkest),
    contrastRatioOrWorst(colour, band.lightest),
  );
}

/**
 * The candidates a modal's edge may be drawn in, **in order of increasing visual weight**.
 *
 * Order is the rule, not a preference list: see `chooseIndicator`. A subtle border is what a dialog
 * wants; a near-white hairline around a dark dialog is what a dark scheme's scrim forces, and it is
 * chosen only because nothing quieter clears the floor.
 */
export const modalEdgeCandidates: readonly ThemeMember[] = [
  'borderSubtle',
  'border',
  'textSecondary',
  'textPrimary',
  'surface',
];

/** The candidates a sheet's grab handle may be drawn in, in the same order. */
export const surfaceHandleCandidates: readonly ThemeMember[] = [
  'borderSubtle',
  'border',
  'textSecondary',
  'textPrimary',
];

/**
 * The candidates a task pane's edge against the **document** may be drawn in.
 *
 * Two, and both are the ends of the scheme's own range — the same two, for the same reason, that
 * `chooseSwatchIndicatorAmong` offers a selection ring: *"for any colour at all, one of the two
 * extremes of a palette is far from it,"* which is what makes the worst case bounded and a third
 * candidate no help. The pane's edge and the picker's ring are the **same question** asked about
 * two different surfaces, and `tests/surfaces.test.ts` asserts the two rules agree colour for
 * colour over the whole sweep rather than letting a near-duplicate go unchecked.
 */
export const dockEdgeCandidates: readonly ThemeMember[] = ['textPrimary', 'surface'];

/** A chosen indicator: which member, how badly it reads, and whether it was good enough. */
export interface ChosenIndicator {
  readonly member: ThemeMember;
  readonly ratio: number;
  /**
   * Whether the chosen member actually clears the floor.
   *
   * ⚠ **The gates assert this is always true, and that is the assertion that can fail.** A rule
   * that silently returned its least-bad option would satisfy every "an indicator was chosen"
   * check while drawing an invisible line, which is MJXOFF-269's defect in a new costume.
   */
  readonly sufficient: boolean;
}

/**
 * **The rule.** The quietest candidate that clears the floor; failing that, the loudest one there
 * is, flagged as insufficient.
 *
 * Two halves, and which half applies depends on whether we control what the indicator sits on:
 *
 * * **Over a surface we do not control** — the scrim's composite, a document page — the candidate
 *   list is the two ends of the palette and *quietest first* is the same as *whichever works*,
 *   because at most one of two opposite extremes can ever fail.
 * * **Over a surface we do control** — a sheet's own fill — quietest-first is doing real work: the
 *   strongest candidate always wins a maximum, and a grab handle drawn in the primary text colour
 *   at 12 : 1 is a black bar across the top of a dialog. WCAG's floor is a floor, not a target.
 *
 * @param candidates in order of increasing visual weight.
 * @param colorOf what a member resolves to *here* — an argument rather than a lookup, because a
 *   host that re-themed the platform by declaring `--theme-text-primary` has changed the answer.
 * @param worstAgainst the worst ratio a colour achieves against whatever it must be told apart
 *   from. One function, so a single colour and a whole band are the same shape of question.
 */
export function chooseIndicator(
  candidates: readonly ThemeMember[],
  colorOf: (member: ThemeMember) => string,
  worstAgainst: (colour: string) => number,
  floor = nonTextMinimum,
): ChosenIndicator {
  let loudest: ChosenIndicator | undefined;
  for (const member of candidates) {
    const ratio = worstAgainst(colorOf(member));
    if (ratio >= floor) return { member, ratio, sufficient: true };
    if (loudest === undefined || ratio > loudest.ratio) {
      loudest = { member, ratio, sufficient: false };
    }
  }
  return loudest ?? { member: 'textPrimary', ratio: 1, sufficient: false };
}

/** A member's generated value for one scheme. What the sweeps measure with. */
export function themeColor(scheme: ColorScheme, member: ThemeMember): string {
  return tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
}

/** The edge a modal draws against its own scrim, for one scheme. */
export function chooseModalEdge(scheme: ColorScheme, alpha = scrimAlpha): ChosenIndicator {
  const band = scrimBand(scheme, alpha);
  return chooseIndicator(
    modalEdgeCandidates,
    (member) => themeColor(scheme, member),
    (colour) => worstAgainstBand(colour, band),
  );
}

/** The grab handle a sheet draws on its own fill, for one scheme. */
export function chooseSurfaceHandle(scheme: ColorScheme): ChosenIndicator {
  // The fill comes from `surfaceBackgroundMember`, which U02 wrote for exactly this: *"a ring's
  // contrast is measured against the thing behind the ring, and that is this. Stated once here so
  // the gate cannot disagree with the stylesheet."* A handle is a ring by another name.
  const member = surfaceBackgroundMember[surfaceKinds.sheet.level] as ThemeMember;
  const fill = themeColor(scheme, member);
  return chooseIndicator(
    surfaceHandleCandidates,
    (member) => themeColor(scheme, member),
    (colour) => contrastRatioOrWorst(colour, fill),
  );
}

/**
 * The edge a task pane draws against **the document beside it**, given what that document actually
 * is.
 *
 * The colour is an argument because a page is not ours: a person may set any page colour they like,
 * and a pane that measured against `--document-page` would choose correctly for a white page and
 * wrongly for the one on screen. `<mjx-task-pane>` reads it off its `document-color` attribute,
 * which is what a shell that knows the page fills in — the same shape as the colour picker taking
 * its palettes as data.
 */
export function chooseDockEdgeAmong(
  documentColor: string,
  colorOf: (member: ThemeMember) => string,
): ChosenIndicator {
  return chooseIndicator(dockEdgeCandidates, colorOf, (colour) =>
    contrastRatioOrWorst(colour, documentColor),
  );
}

/** The same, against the generated palette for one scheme. What a gate sweeps with. */
export function chooseDockEdge(documentColor: string, scheme: ColorScheme): ChosenIndicator {
  return chooseDockEdgeAmong(documentColor, (member) => themeColor(scheme, member));
}

/** The weakest edge over a set of document colours — the number a gate asserts. */
export interface WorstSurfaceIndicator {
  readonly against: string;
  readonly indicator: ChosenIndicator;
}

export function worstDockEdge(
  documentColors: Iterable<string>,
  scheme: ColorScheme,
): WorstSurfaceIndicator | undefined {
  let worst: WorstSurfaceIndicator | undefined;
  for (const against of documentColors) {
    const indicator = chooseDockEdge(against, scheme);
    if (worst === undefined || indicator.ratio < worst.indicator.ratio) {
      worst = { against, indicator };
    }
  }
  return worst;
}

/** `3.44 : 1 against the scrim band (borderSubtle)`, for a caption and a failure message. */
export function describeIndicator(member: ThemeMember, ratio: number, against: string): string {
  return `${formatRatio(ratio)} against ${against} (${member})`;
}

/** WCAG 1.4.11's floor, re-exported so the surfaces' gates and their stories read one number. */
export { nonTextMinimum };

/**
 * U05's focus table, re-exported.
 *
 * Re-exported rather than restated, on MJXOFF-188's own instruction, and the `shared` member this
 * child added lives there beside the other two rather than here beside its consumers — because a
 * table split across two files is two tables that will eventually disagree.
 */
export { focusManagementPatterns };

// ── the scheme-keyed properties ──────────────────────────────────────────────

/**
 * The custom properties the three components read.
 *
 * The first three are **scheme-keyed** and cannot be anything else: which member carries the modal
 * edge is `borderSubtle` in light and `textPrimary` in dark, and no single `--theme-*` name means
 * both. The remaining ones are written by a component onto its own box.
 */
export const surfaceProperties = {
  /** The composited scrim. */
  scrim: '--mjx-surface-scrim',
  /** The modal's outer edge, chosen against the scrim band. */
  edge: '--mjx-surface-edge',
  /** The sheet's grab handle, chosen against the sheet's own fill. */
  handle: '--mjx-surface-handle',
  /** Written by `<mjx-task-pane>` at paint time, from the document colour it was handed. */
  dockEdge: '--mjx-surface-dock-edge',
  /** The pane's share of its boundary, so a drag is one custom property. */
  dockFraction: '--mjx-surface-dock-fraction',
  /** The corner treatment, so one box serves a centred dialog and a pinned sheet. */
  radiusBlockStart: '--mjx-surface-radius-block-start',
  radiusBlockEnd: '--mjx-surface-radius-block-end',
  /** How far a surface travels as it enters. Zero for anything that must not travel. */
  enterTranslate: '--mjx-surface-enter-translate',
} as const;

/**
 * The three selectors `tokens.css` keys its colour-scheme layer off.
 *
 * ⚠ **This is a second statement of somebody else's rule, and it is the weakest thing in this
 * file.** The generated stylesheet decides which scheme is in force with exactly these three
 * selectors — the light default, the system preference unless the host asked for light, and an
 * explicit `data-theme` — and the scheme-keyed properties above have to agree with it or a dialog
 * will pick its light edge on a dark page.
 *
 * There is no way to avoid restating them: `tokens.css` publishes the *values* for both schemes
 * unconditionally and the *choice* only through those rules, and a custom property cannot be
 * selected by another custom property. What is possible is to make the restatement checkable, so
 * `tests/surfaces.test.ts` reads `ui/tokens/tokens.css` and asserts these three strings still
 * occur in it. A re-seed that changes the scheme layer's shape then fails a gate here instead of
 * shipping a dialog that is right in one scheme.
 */
export const schemeSelectors = {
  light: ':root',
  darkPreferred: ':root:not([data-theme="light"])',
  darkExplicit: ':root[data-theme="dark"]',
} as const;

/** `textPrimary`, `dark` → `var(--theme-dark-text-primary)`. The unconditional, scheme-named half. */
export function schemeThemeVariable(scheme: ColorScheme, member: ThemeMember): string {
  return `var(--theme-${scheme}-${customPropertyCase(member)})`;
}

function schemeDeclarations(scheme: ColorScheme, indent: string): string {
  const scrim = schemeThemeVariable(scheme, darkestThemeMember(scheme));
  const edge = schemeThemeVariable(scheme, chooseModalEdge(scheme).member);
  const handle = schemeThemeVariable(scheme, chooseSurfaceHandle(scheme).member);
  return [
    `${indent}${surfaceProperties.scrim}: color-mix(in srgb, ${scrim} ${String(scrimOpacityPercent)}%, transparent);`,
    `${indent}${surfaceProperties.edge}: ${edge};`,
    `${indent}${surfaceProperties.handle}: ${handle};`,
  ].join('\n');
}

/**
 * The scheme-keyed sheet, installed on the **document** rather than on a shadow root.
 *
 * Custom properties inherit through the flat tree and `:root` is a document selector, so this is
 * the only place these three can be declared. It is the same arrangement `galleryDocumentCss` and
 * `inputDocumentCss` use, for the same reason.
 */
export const surfaceSchemeCss = `
${schemeSelectors.light} {
${schemeDeclarations('light', '  ')}
}

@media (prefers-color-scheme: dark) {
  ${schemeSelectors.darkPreferred} {
${schemeDeclarations('dark', '    ')}
  }
}

${schemeSelectors.darkExplicit} {
${schemeDeclarations('dark', '  ')}
}
`;

// ── the stylesheets ──────────────────────────────────────────────────────────

const overlayRung = surfaceLevels.overlay;
const floatingRung = surfaceLevels.floating;

/** The header a dialog, a sheet, a flyout and a task pane all wear. Composed once. */
const headerCss = `
  .header {
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.step});
    flex: 0 0 auto;
  }

  .title {
    flex: 1 1 auto;
    margin: 0;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .body {
    /* A flex column with a max-block-size shrinks its children — U07's second defect. Anything
     * inside a scrolling popup whose height is part of an arithmetic contract needs an explicit
     * flex basis as well as a size. */
    flex: 1 1 auto;
    min-block-size: 0;
    overflow: auto;
    overscroll-behavior: contain;
    padding-inline: var(${densityProperties.gutter});
    padding-block-end: var(${densityProperties.gutter});
  }

  .footer {
    display: flex;
    justify-content: flex-end;
    gap: var(${densityProperties.step});
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.gutter});
    flex: 0 0 auto;
  }

  .footer:not(:has(*)) { display: none; }

  /* The close control. It paints nothing of its own beyond the hit floor and the shared motion
   * class: a bare glyph on the surface it sits on, so a re-seed moves it with everything else. */
  .close {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    border: 1px solid transparent;
    border-radius: var(--radius-control);
    background: transparent;
    color: var(--theme-text-secondary);
    cursor: pointer;
    transition-property: background, border-color, color;
  }

  .close:hover { background: var(--theme-border-subtle); color: var(--theme-text-primary); }
  .close:active { background: var(--theme-border); }
`;

/**
 * `<mjx-dialog>`'s own rules.
 *
 * ⚠ **The box is a top-layer popover and so is the scrim**, and they are two popovers rather than
 * one box with a pseudo-element. `::backdrop` would have been fewer lines and would cover the
 * **window**, not the simulated screen: this catalogue's whole responsive doctrine is that a
 * component answers to `<mjx-resizable-container>`'s frame, and a scrim that dimmed the whole
 * Storybook page while the phone-sized frame beside it stayed bright would be reporting a
 * modality the container never had. Pinning the scrim to `clippingBoundary()` — the same rectangle
 * the sheet is pinned to and the same one a menu flips against — is what keeps one answer.
 */
export const dialogCss = `
  :host { display: contents; }
  :host([hidden]) { display: none; }

  .scrim {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    border: none;
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    inline-size: var(${floatingProperties.inlineSize}, auto);
    block-size: var(${floatingProperties.blockSize}, auto);
    background: var(${surfaceProperties.scrim});
    opacity: 1;
    transition-property: opacity;
  }

  .scrim[data-entering] { opacity: 0; }
  .scrim[data-open='false'] { display: none; }

  .surface {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    color: inherit;
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    inline-size: var(${floatingProperties.inlineSize}, auto);
    max-inline-size: var(${floatingProperties.maxInlineSize});
    max-block-size: var(${floatingProperties.maxBlockSize});
    display: flex;
    flex-direction: column;
    overflow: clip;
    background: ${overlayRung.background};
    border: ${overlayRung.border};
    box-shadow: ${overlayRung.shadow};
    /* The measured edge. A hairline outside the rung's own border, in whichever member cleared the
     * floor against the scrim band for the scheme in force. See chooseModalEdge. */
    outline: 1px solid var(${surfaceProperties.edge});
    outline-offset: 0px;
    border-start-start-radius: var(${surfaceProperties.radiusBlockStart});
    border-start-end-radius: var(${surfaceProperties.radiusBlockStart});
    border-end-start-radius: var(${surfaceProperties.radiusBlockEnd});
    border-end-end-radius: var(${surfaceProperties.radiusBlockEnd});
    translate: 0 0;
    opacity: 1;
    transition-property: translate, opacity;
  }

  .surface[data-open='false'] { display: none; }

  .surface[data-entering] {
    translate: 0 var(${surfaceProperties.enterTranslate});
    opacity: 0;
  }

  .handle {
    flex: 0 0 auto;
    display: none;
    justify-content: center;
    padding-block-start: var(${densityProperties.step});
  }

  .handle::after {
    content: '';
    display: block;
    block-size: calc(var(--spacing) / 2);
    inline-size: calc(var(--spacing) * 10);
    border-radius: var(--radius-chip);
    background: var(${surfaceProperties.handle});
  }

  .surface[data-presentation='sheet'] .handle { display: flex; }

${headerCss}
`;

/** The presentation blocks, generated from the table so the two cannot disagree. */
export const dialogPresentationCss = `
  :where(.surface) {
    ${surfacePresentationProperty}: dialog;
    ${surfaceProperties.radiusBlockStart}: ${radiusVariable(surfaceKinds.dialog.radius)};
    ${surfaceProperties.radiusBlockEnd}: ${radiusVariable(surfaceKinds.dialog.radius)};
    ${surfaceProperties.enterTranslate}: 0px;
  }

  /* ⚠ Keyed on the modality as well as the width, and the extra condition is behaviour rather
   * than taste: a sheet is dismissible by its scrim and a modeless dialog has no scrim, so a
   * modeless dialog that became a sheet at a phone width would silently acquire a dismissal path
   * its own row does not list. A modeless dialog stays centred at every width. */
  @container ${containerName} (width <= ${String(dialogSheetAtOrBelow)}px) {
    :where(.surface[data-modal='true']) {
      ${surfacePresentationProperty}: sheet;
      ${surfaceProperties.radiusBlockStart}: ${radiusVariable(surfaceKinds.sheet.radius)};
      /* Squared, because a sheet sits on the edge it is pinned to and rounding away from an edge
       * there is nothing behind is how a sheet ends up looking like a card that slipped. */
      ${surfaceProperties.radiusBlockEnd}: 0px;
      ${surfaceProperties.enterTranslate}: ${spacingMultiple(8)};
    }
  }
`;

/** `<mjx-popover>`'s own rules. Anchored, never pinned, and never scrimmed. */
export const popoverCss = `
  :host { display: contents; }
  :host([hidden]) { display: none; }

  .surface {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    color: inherit;
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    inline-size: max-content;
    max-inline-size: var(${floatingProperties.maxInlineSize});
    max-block-size: var(${floatingProperties.maxBlockSize});
    display: flex;
    flex-direction: column;
    overflow: clip;
    background: ${overlayRung.background};
    border: ${overlayRung.border};
    box-shadow: ${overlayRung.shadow};
    /* The four longhands, not the shorthand. Not a style preference: the literal-value lint reads a
     * border-radius shorthand and requires a radius token in it, and this value is a custom
     * property whose own value is one. The longhands say the same thing and are checkable at the
     * place the token is actually named. */
    border-start-start-radius: var(${surfaceProperties.radiusBlockStart});
    border-start-end-radius: var(${surfaceProperties.radiusBlockStart});
    border-end-start-radius: var(${surfaceProperties.radiusBlockStart});
    border-end-end-radius: var(${surfaceProperties.radiusBlockStart});
    translate: 0 0;
    opacity: 1;
    transition-property: translate, opacity;
  }

  .surface[data-open='false'] { display: none; }
  .surface[data-entering] { opacity: 0; }

  :where(.surface[data-kind='popover']) {
    ${surfaceProperties.radiusBlockStart}: ${radiusVariable(surfaceKinds.popover.radius)};
  }

  :where(.surface[data-kind='flyout']) {
    ${surfaceProperties.radiusBlockStart}: ${radiusVariable(surfaceKinds.flyout.radius)};
    min-inline-size: ${spacingMultiple(48)};
  }

${headerCss}
`;

/**
 * `<mjx-task-pane>`'s own rules.
 *
 * The pane is in **flow** at a desktop width — it is a dock, not an overlay, and its whole point is
 * that the document lays out beside it rather than under it — and becomes a pinned sheet at a
 * phone width, where there is no room for two columns. That is one more consumer of
 * `phoneShellAtOrBelow` and of `pinFloating`, and no new arithmetic.
 */
export const taskPaneCss = `
  :host {
    display: block;
    box-sizing: border-box;
    ${surfacePresentationProperty}: docked;
    inline-size: calc(100% * var(${surfaceProperties.dockFraction}));
    flex: 0 0 auto;
  }

  :host([hidden]) { display: none; }
  :host(:not([open])) { display: none; }

  .pane {
    box-sizing: border-box;
    block-size: 100%;
    display: flex;
    flex-direction: column;
    overflow: clip;
    background: ${floatingRung.background};
    border: ${floatingRung.border};
    box-shadow: ${floatingRung.shadow};
    border-start-start-radius: var(${surfaceProperties.radiusBlockStart});
    border-start-end-radius: var(${surfaceProperties.radiusBlockStart});
    border-end-start-radius: var(${surfaceProperties.radiusBlockStart});
    border-end-end-radius: var(${surfaceProperties.radiusBlockStart});
    ${surfaceProperties.radiusBlockStart}: ${radiusVariable(surfaceKinds.taskPane.radius)};
  }

  /* The splitter. A separator with a value, per the ARIA window-splitter pattern, so a keyboard can
   * resize what a pointer can drag. It carries the measured edge against the document. */
  .splitter {
    flex: 0 0 auto;
    align-self: stretch;
    inline-size: max(var(${densityProperties.step}), ${String(accessibleHitTargetMinimum)}px);
    cursor: col-resize;
    background: transparent;
    border: none;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    touch-action: none;
  }

  .splitter::after {
    content: '';
    display: block;
    inline-size: 1px;
    block-size: 100%;
    background: var(${surfaceProperties.dockEdge}, var(${surfaceProperties.edge}));
  }

  .dock {
    display: flex;
    block-size: 100%;
  }

  :host([dock='inlineStart']) .dock { flex-direction: row-reverse; }

  /* At a phone width a dock is not a dock: two columns do not fit, so the pane takes the whole
   * frame and there is nothing left to resize it against. It is deliberately NOT a sheet — a sheet
   * is dismissible and a task pane is not, and building a third pinned surface to say the same
   * thing would be the duplication this child exists to avoid.
   *
   * WARNING: this block must stay AFTER the base rules, and the browser gate found out why. The
   * container condition changes nothing about specificity: the splitter selector inside it scores
   * (0,1,0) and so does the one outside it, so source order is the whole of the arbitration.
   * Written above the base rules -- which is where it read most naturally, beside the host rule it
   * also changes -- the pane reported full at a phone width AND STILL DREW ITS SPLITTER, with the
   * presentation assertion green. MJXOFF-183's accident arriving by the one route its own fix
   * (wrapping every presentation selector in a zero-specificity :where) does not cover, because
   * here it is the base rule that would need the wrapping. tests/surfaces.test.ts asserts the
   * order.
   *
   * And a second one, for the third time in this catalogue: a backtick inside a CSS comment ends
   * the template literal it is in. This paragraph was written with them and cost a build. */
  @container ${containerName} (width <= ${String(phoneShellAtOrBelow)}px) {
    :host {
      ${surfacePresentationProperty}: full;
      inline-size: 100%;
    }
    .splitter { display: none; }
  }

${headerCss}
`;

/**
 * The rule a **document** carries, so a task pane that has not upgraded yet does not flash at full
 * width and push the document off the screen for one frame.
 *
 * The same arrangement `galleryDocumentCss` and `inputDocumentCss` use, and here it also carries
 * the scheme-keyed properties, because those can only be declared on `:root`.
 */
export const surfaceDocumentCss = `
${surfaceSchemeCss}

:where(${surfaceTags.taskPane}:not(:defined)) {
  display: block;
  inline-size: calc(100% * ${String(taskPaneDefaultFraction)});
}

:where(${surfaceTags.dialog}:not(:defined), ${surfaceTags.popover}:not(:defined)) {
  display: contents;
}
`;

/** `dialog` → `.mjx-motion-sheet-enter`, read off the table rather than written twice. */
export function surfaceMotionClass(kind: SurfaceKind): string {
  return motionRoleClass(surfaceKinds[kind].motionRole);
}

/** The type role each part of a surface wears. Stated once so a gate can assert it. */
export const surfaceTypeRoles = {
  title: typeRoleClass('paneTitle'),
  body: typeRoleClass('body'),
} as const;

/** The close glyph, at the mark size the rest of the catalogue uses for marks beside a command. */
export const surfaceCloseIcon = { name: 'dismiss', size: 16 } as const;

/** The accessible name of the close control, derived from the surface's own label. */
export function closeButtonLabel(label: string): string {
  return label === '' ? 'Close' : `Close ${label}`;
}

/** The accessible name of a task pane's splitter. */
export function splitterLabel(label: string): string {
  return label === '' ? 'Resize pane' : `Resize ${label}`;
}
