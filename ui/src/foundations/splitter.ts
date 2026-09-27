/**
 * The window-splitter primitive: the arithmetic behind an ARIA `separator` that has a value.
 *
 * ## Why this file exists, and where it came from
 *
 * MJXOFF-188 built a resizable dock splitter inside `<mjx-task-pane>` and put its arithmetic in
 * `src/surfaces/surface-model.ts` under task-pane names — `clampFraction`, `fractionFromDrag`,
 * `fractionFromKey`, all bound to `taskPaneFractionBounds`. MJXOFF-190 needs the same behaviour for
 * a standalone `<mjx-splitter>` between two regions of the shell, with *different* bounds and a
 * collapse the task pane does not have.
 *
 * A second copy would have been two implementations of one keyboard contract, and the first defect
 * would have been an arrow key that grew the pane in one component and shrank it in the other. So
 * the arithmetic was **lifted here and parameterised by its bounds**, and `surface-model.ts` now
 * re-states its three exports as one-line bindings over these functions with the task pane's own
 * bounds. Behaviour for every existing caller is byte-identical — `tests/surfaces.test.ts` is the
 * assertion, unchanged — and `tests/furniture.test.ts` asserts the equivalence directly rather than
 * leaving it as a claim.
 *
 * ## Node-importable
 *
 * Data and pure functions. No DOM, no tokens, no CSS — for the reason `src/harness/presets.ts`
 * states: the browser tier's specs run in Node, and a module that defines a custom element cannot
 * be imported from one.
 */

/** The narrowest and widest share of the boundary a splitter may be dragged to. */
export interface SplitterBounds {
  readonly min: number;
  readonly max: number;
}

/** Where a splitter sits when nobody has moved it, and what an unreadable value falls back to. */
export const splitterDefaultFraction = 0.32;

/** One press of an arrow key, as a fraction of the boundary. */
export const splitterStep = 0.02;

/** What `<mjx-splitter>` allows when it says nothing: a sixth of the boundary at either end. */
export const splitterDefaultBounds: SplitterBounds = { min: 0.15, max: 0.85 };

/**
 * Keep a fraction inside its bounds.
 *
 * A value that is not a number falls back rather than propagating: a `NaN` fraction would be
 * written into a custom property, `calc()` would fail to parse it, and the pane would take its
 * declared width in CSS while reporting a number nobody can read in `aria-valuenow`.
 */
export function clampToBounds(value: number, bounds: SplitterBounds, fallback: number): number {
  if (!Number.isFinite(value)) return fallback;
  return Math.min(Math.max(value, bounds.min), bounds.max);
}

/** The boundary a drag is measured against: where it starts, and how long it is. */
export interface SplitterBoundary {
  readonly start: number;
  readonly size: number;
}

/**
 * How a pointer position on the boundary becomes a fraction.
 *
 * Pure, so a Node test can drive a whole drag without a browser. `side` is the **physical** side
 * the resized region occupies, which is what makes right-to-left one call to `physicalSide` rather
 * than a second code path.
 */
export function dragFraction(
  pointerInline: number,
  boundary: SplitterBoundary,
  side: 'left' | 'right',
  bounds: SplitterBounds,
  fallback: number,
): number {
  if (boundary.size <= 0) return fallback;
  const fromStart = (pointerInline - boundary.start) / boundary.size;
  const raw = side === 'right' ? 1 - fromStart : fromStart;
  return clampToBounds(raw, bounds, fallback);
}

/**
 * What one key does to the resized region's share of the boundary.
 *
 * ⚠ **The arrow that grows the region depends on which side it is on**, and on the writing
 * direction, for the same reason `<mjx-menu>`'s submenu arrows mirror: a region at the end of the
 * line is grown by the arrow that points toward the start of it, and *which* arrow that is changes
 * in Arabic. Returning the fraction rather than the delta is what lets `Home` and `End` share the
 * function.
 *
 * `undefined` means *this key is not ours* — the caller must not `preventDefault` a key it did not
 * consume, or a splitter would eat `Tab`.
 */
export function keyFraction(
  key: string,
  current: number,
  side: 'left' | 'right',
  bounds: SplitterBounds,
  step: number,
  fallback: number,
): number | undefined {
  const grow = side === 'right' ? 'ArrowLeft' : 'ArrowRight';
  const shrink = side === 'right' ? 'ArrowRight' : 'ArrowLeft';
  switch (key) {
    case grow:
      return clampToBounds(current + step, bounds, fallback);
    case shrink:
      return clampToBounds(current - step, bounds, fallback);
    case 'Home':
      return bounds.min;
    case 'End':
      return bounds.max;
    default:
      return undefined;
  }
}

/**
 * The accessible name of a splitter, derived from what it resizes.
 *
 * A separator announced as *"separator"* tells a person there is a control and nothing about what
 * moving it does.
 */
export function splitterLabel(label: string): string {
  return label === '' ? 'Resize pane' : `Resize ${label}`;
}

/** What `aria-valuenow`, `aria-valuemin` and `aria-valuemax` carry: whole percentages. */
export function splitterValueNow(fraction: number): number {
  return Math.round(fraction * 100);
}
