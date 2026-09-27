/**
 * The bottom sheet's detents, its snap arithmetic, and **the nested-scroll rule.**
 *
 * MJXOFF-188 built the sheet and MJXOFF-194 completes it. Nothing here replaces it: a sheet is
 * still `<mjx-dialog>` in its sheet presentation, still pinned with `pinFloating`, still the one
 * implementation U09 insisted on. What was missing is that `pinFloating` was handed **one** number
 * — `sheetBoundaryFraction` — and a sheet with one size is a sheet with no detents. A detent is
 * that number, and `full` is an **alias** of it, asserted, in the same doctrine that keeps
 * `phoneShellAtOrBelow` from acquiring four copies.
 *
 * ## The classic defect, and why it is a pure function here
 *
 * > A list inside a sheet must scroll without dismissing it.
 *
 * That is the failure this file exists to prevent, and it is entirely a question of **who claims a
 * downward drag**: the sheet, or the thing inside it. Getting it wrong is invisible in a screenshot
 * and obvious in the hand, which is precisely the class of defect a pure function plus an
 * exhaustive table can nail down. `sheetDragClaim` is that function, and it has no DOM in it.
 *
 * ## Node-importable
 *
 * Data and pure functions only.
 */

import { sheetBoundaryFraction } from '../surfaces/surface-model.ts';

/** The three heights a sheet rests at, shallowest first. */
export const sheetDetentNames = ['peek', 'half', 'full'] as const;

/** One of the three. */
export type SheetDetent = (typeof sheetDetentNames)[number];

/** What a detent is for. */
export interface SheetDetentSpec {
  /** The fraction of the clipping boundary's block size the sheet occupies. */
  readonly fraction: number;
  readonly use: string;
}

/**
 * The three detents.
 *
 * ⚠ **`full` is `sheetBoundaryFraction`, not a fourth number.** U09 decided how much of the
 * boundary a sheet may cover before it scrolls; that decision is unchanged and this file does not
 * get a vote on it. `tests/mobile.test.ts` asserts the identity, because an alias that quietly
 * became a literal is exactly the drift the hoist doctrine exists to catch.
 */
export const sheetDetents: Readonly<Record<SheetDetent, SheetDetentSpec>> = {
  peek: {
    fraction: 0.3,
    use: 'Enough to read the sheet’s title and its first row, with most of the document still visible behind it.',
  },
  half: {
    fraction: 0.55,
    use: 'The resting detent. A list is worth scrolling and the document is still legible above it.',
  },
  full: {
    fraction: sheetBoundaryFraction,
    use: 'As much of the boundary as U09 allows a sheet to cover. Beyond this the sheet scrolls rather than grows.',
  },
};

/** Where a sheet opens when nothing says otherwise. */
export const defaultSheetDetent: SheetDetent = 'half';

/**
 * Below this fraction a release dismisses the sheet rather than snapping to `peek`.
 *
 * It is deliberately well under `peek`: a sheet that vanished whenever a person undershot the
 * shallowest detent would be a sheet that dismisses itself, and the cost of the two mistakes is not
 * symmetric. Losing a sheet loses whatever was in it; snapping back to `peek` costs one more drag.
 */
export const sheetDismissBelowFraction = 0.18;

/**
 * How far ahead a release is projected, in milliseconds.
 *
 * A flick is a short, fast gesture that ends well short of where the person meant it to go, so
 * snapping to whatever fraction the finger happened to lift at makes a fast flick feel like it was
 * ignored. Projecting the velocity forward by a fixed span is the standard answer and the span is
 * the whole of the tuning.
 *
 * `GUESS:` 120 ms is ours; it is not measured against any platform's own physics.
 */
export const detentProjectionMilliseconds = 120;

/** Where the sheet is going, as fractions of the boundary. */
export interface DetentRelease {
  /** The fraction the sheet is at when the finger lifts. */
  readonly fraction: number;
  /**
   * How fast it is moving, in fractions of the boundary per millisecond. **Positive is opening**
   * — growing — which is the opposite sign to a downward drag in screen coordinates, and the
   * component is where that conversion happens.
   */
  readonly velocity: number;
}

/** The result of a release: a detent to snap to, or a dismissal. */
export type DetentOutcome = { readonly kind: 'snap'; readonly detent: SheetDetent } | { readonly kind: 'dismiss' };

/** Where a release is projected to end up, before it is rounded to a detent. */
export function projectFraction(release: DetentRelease): number {
  return release.fraction + release.velocity * detentProjectionMilliseconds;
}

/**
 * Snap a release to a detent, or dismiss.
 *
 * The projection comes first and the dismissal test is applied to the **projected** fraction, not
 * to the released one: a slow drag that stops just above the dismissal threshold should stay, and a
 * fast flick downward from halfway should go, and only the projection tells those two apart.
 */
export function snapDetent(release: DetentRelease): DetentOutcome {
  const projected = projectFraction(release);
  if (projected < sheetDismissBelowFraction) return { kind: 'dismiss' };

  let best: SheetDetent = sheetDetentNames[0];
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const name of sheetDetentNames) {
    const distance = Math.abs(sheetDetents[name].fraction - projected);
    if (distance < bestDistance) {
      bestDistance = distance;
      best = name;
    }
  }
  return { kind: 'snap', detent: best };
}

// ── the nested-scroll rule ───────────────────────────────────────────────────

/** Everything the decision is made from. No element, no event: two booleans and two numbers. */
export interface SheetDragInput {
  /** The gesture started on the grab handle. */
  readonly onHandle: boolean;
  /** The gesture started on the header — the title row, which is a grab area too. */
  readonly onHeader: boolean;
  /** How far the scroller inside the sheet is scrolled, in pixels. */
  readonly scrollTop: number;
  /** How far the gesture has moved down the block axis. **Positive is downward.** */
  readonly deltaBlock: number;
}

/** Who owns this drag. */
export type SheetDragOwner = 'sheet' | 'content';

/**
 * **Who gets the drag** — the whole of the classic mobile defect, in four rules.
 *
 * 1. **The handle and the header are always the sheet's.** They are the grab areas; that is what
 *    they are for, and a sheet whose handle did not drag it would have no affordance at all.
 * 2. **A scrolled scroller keeps the drag.** If the content inside the sheet is scrolled at all,
 *    the gesture belongs to it — in either direction. This is the rule whose absence dismisses a
 *    sheet when a person flicks a long list, and it is the one worth stating loudest.
 * 3. **At the top of the scroller, a downward drag hands off to the sheet.** This is the one
 *    hand-off, and it is what makes a sheet feel like a sheet rather than like a fixed panel: at
 *    the top of the list there is nothing left to scroll, so the gesture means *move the sheet*.
 * 4. **Everything else is the content's**, including an upward drag at the top of a scroller,
 *    which is an over-scroll and belongs to the scroller's own rubber-banding.
 *
 * `overscroll-behavior: contain` on the scroller is what stops rule 4's over-scroll reaching the
 * page behind, and the browser gate reads it — it is the CSS half of the same decision.
 */
export function sheetDragClaim(input: SheetDragInput): SheetDragOwner {
  if (input.onHandle || input.onHeader) return 'sheet';
  if (input.scrollTop > 0) return 'content';
  if (input.deltaBlock > 0) return 'sheet';
  return 'content';
}

/** The attribute the sheet reflects its detent through, so a story and a gate can both read it. */
export const sheetDetentAttribute = 'detent';

/** Whether a value names a detent. */
export function isSheetDetent(value: unknown): value is SheetDetent {
  return (sheetDetentNames as readonly string[]).includes(String(value));
}

/** The fraction for a detent, which is the number `pinFloating` is handed. */
export function detentFraction(detent: SheetDetent): number {
  return sheetDetents[detent].fraction;
}

/**
 * The detent nearest a fraction, with no projection: what an in-flight drag is currently *at*.
 *
 * Used for the readout a story shows and for nothing that decides anything, which is why it is
 * separate from `snapDetent` rather than a parameterisation of it.
 */
export function detentNearest(fraction: number): SheetDetent {
  const outcome = snapDetent({ fraction, velocity: 0 });
  return outcome.kind === 'snap' ? outcome.detent : sheetDetentNames[0];
}
