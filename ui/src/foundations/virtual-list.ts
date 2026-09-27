/**
 * The catalogue's **one** virtualisation, and the arithmetic four navigators share.
 *
 * ## What was lifted, and from where
 *
 * MJXOFF-185 (U06) wrote `galleryWindow` — *given a row count, a first visible row and how many
 * rows fit, which rows do I build* — and MJXOFF-186 and MJXOFF-187 already reuse it, so the gallery,
 * a font list and a swatch grid are one virtualiser with three plan shapes. MJXOFF-191's ticket says
 * so in as many words: *"two virtualisation implementations in one catalogue is exactly the
 * duplication the stack exists to prevent."*
 *
 * So `virtualWindow` below **is** `galleryWindow`, moved down here, and `galleryWindow` is now a
 * one-line binding over it with the same signature and the same defaults. Nothing changed for the
 * gallery, the list surface or the swatch grid, and `tests/navigators.test.ts` asserts that over a
 * sweep rather than leaving it as a claim — U11's `foundations/splitter.ts` pattern.
 *
 * ## The precondition U07 found, made explicit and then removed
 *
 * `galleryWindow` takes a *first visible row* and a *row count*, and every caller computed the
 * first from `scrollTop / rowHeight`. That division carries an unstated precondition — **every row
 * in the plan is exactly one row tall** — which a gallery's headings never had to honour and which
 * cost MJXOFF-186 two of its four real defects to make true for a font list.
 *
 * A tree with a heading, a rail with a caption and a section break, and a tab bar cannot honour it
 * at all. So this file keeps the arithmetic and replaces the division with a **binary search over
 * an [`ExtentTable`]**: `windowForOffset` answers the same question for rows of different heights,
 * and `virtualWindow` is what it delegates the clamping to. The precondition is now a property of
 * *one* entry point rather than an invisible property of every caller.
 *
 * ## The trap this file exists to answer
 *
 * > **Scroll-position stability when items change above the viewport is the defect users actually
 * > hit** — a slide inserted above the visible range must not move what they are looking at — and
 * > it never appears in a static story.
 *
 * The answer is R13's, one level up: **hold an anchor, derive the offset.** But a scrollbar's
 * anchor is `{ page, within }` and an *index* is exactly the wrong state here, because inserting a
 * row above index 40 makes index 40 a different row. So the anchor a list holds is
 * [`KeyedAnchor`] — `{ key, within }` — and [`offsetOfAnchor`] resolves the key against the *current*
 * ordering. Insert above the viewport, ask again, write the answer to `scrollTop`, and the row the
 * reader was looking at is where it was.
 *
 * The naive alternative — keep the offset — is what `tests/navigators.test.ts` and
 * `tests/browser/navigators.spec.ts` both compute **beside** the real answer and assert differs
 * from it, because U11's rule is that without a positive control a stability assertion is a
 * tolerance nobody tested.
 *
 * ## Node-importable
 *
 * Numbers only. No DOM, no tokens, no CSS.
 */

import type { ExtentTable } from './extent-table.ts';

/**
 * How many rows outside the visible window are built anyway.
 *
 * One, at each end. U06's number and U06's reasoning: zero means a row is created during the scroll
 * that reveals it, which is a blank band on every flick; more than one buys nothing and costs
 * exactly what virtualisation was for.
 */
export const defaultOverscanRows = 1;

/** The slice of rows a surface builds. */
export interface VirtualWindow {
  /** The first row built, inclusive. */
  readonly firstRow: number;
  /** The last row built, exclusive. */
  readonly lastRow: number;
}

/**
 * Which rows to build. **Pure, so a node-count gate has a number to compare against.**
 *
 * U06's `galleryWindow`, unchanged: `firstVisibleRow` and `visibleRows` come from a measurement,
 * everything after that is arithmetic, and keeping the arithmetic here is what lets both tiers
 * assert the clamping at both ends without a browser.
 */
export function virtualWindow(
  rowCount: number,
  firstVisibleRow: number,
  visibleRows: number,
  overscan = defaultOverscanRows,
): VirtualWindow {
  if (rowCount <= 0) return { firstRow: 0, lastRow: 0 };
  const first = Math.max(0, Math.min(firstVisibleRow, rowCount - 1) - overscan);
  const last = Math.min(
    rowCount,
    Math.max(firstVisibleRow, 0) + Math.max(visibleRows, 1) + overscan,
  );
  return { firstRow: first, lastRow: Math.max(last, first) };
}

/** How many rows a window holds. The number a node-count assertion compares against. */
export function rowsInWindow(window: VirtualWindow): number {
  return Math.max(0, window.lastRow - window.firstRow);
}

/**
 * Which rows to build, when the rows are not all the same height.
 *
 * The division `galleryWindow`'s callers do becomes a binary search, and the *count* of visible rows
 * becomes a walk forward from that row until the viewport is covered — which is the only honest
 * answer when three rows might be a heading, a thumbnail and a caption.
 *
 * The walk is bounded by the viewport rather than by the table: it stops the moment the accumulated
 * extent covers the viewport, so a five-thousand-row table costs the same as a fifty-row one.
 */
export function windowForOffset(
  table: ExtentTable,
  offset: number,
  viewportExtent: number,
  overscan = defaultOverscanRows,
): VirtualWindow {
  const count = table.count;
  if (count <= 0) return { firstRow: 0, lastRow: 0 };
  const first = table.anchorAt(Math.max(0, offset)).index;
  // ⚠ At least one row is visible even in a viewport of zero. A component measured before it is
  // laid out reports a client height of nothing, and a window of nothing there is a list that
  // renders empty for ever — U06's *"a ceiling satisfied by zero"* in its most literal form.
  let covered = -(Math.max(0, offset) - table.offsetOf(first));
  let visible = 0;
  let row = first;
  while (row < count && (visible === 0 || covered < Math.max(0, viewportExtent))) {
    covered += table.extentOf(row);
    visible += 1;
    row += 1;
  }
  return virtualWindow(count, first, visible, overscan);
}

/** The spacers that stand in for the rows a window did not build. */
export interface WindowSpacers {
  /** How much extent sits before the first built row. */
  readonly leading: number;
  /** How much sits after the last one. */
  readonly trailing: number;
}

/**
 * The two spacer extents, so the scroll container is as long as **every** row rather than as long
 * as the built ones.
 *
 * A sizer that measured only what was built is the classic virtualisation defect: the scrollbar
 * lies about how much document there is, and the further a reader scrolls the more it lies.
 */
export function windowSpacers(table: ExtentTable, window: VirtualWindow): WindowSpacers {
  const leading = table.offsetOf(window.firstRow);
  const trailing = Math.max(0, table.total - table.offsetOf(window.lastRow));
  return { leading, trailing };
}

/**
 * Where to scroll so that `row` is on screen, given where the list is scrolled now.
 *
 * Scroll-to-index, and it deliberately does the *least* it can: a row already fully visible does not
 * move the list at all. A version that always centred would make every arrow key jump the viewport,
 * which is the behaviour people describe as *"it keeps losing my place"*.
 */
export function offsetToReveal(
  table: ExtentTable,
  row: number,
  viewportExtent: number,
  currentOffset: number,
): number {
  const count = table.count;
  if (count <= 0) return 0;
  const at = Math.min(count - 1, Math.max(0, Math.floor(row)));
  const leading = table.offsetOf(at);
  const trailing = leading + table.extentOf(at);
  const viewport = Math.max(0, viewportExtent);
  const furthest = Math.max(0, table.total - viewport);
  if (leading < currentOffset) return Math.min(furthest, leading);
  if (trailing > currentOffset + viewport) return Math.min(furthest, trailing - viewport);
  return Math.min(furthest, Math.max(0, currentOffset));
}

/**
 * Where a reader is, in terms that survive an insertion above them.
 *
 * ⚠ **A key and not an index.** `ExtentAnchor` — R13's, and the right state for a scrollbar over
 * pages that never get reordered — names a row by *position*, and a position is exactly what an
 * insertion above it changes. A key is what a reader actually means by *where I am*: **this slide**,
 * a third of the way down, whatever number it now carries.
 */
export interface KeyedAnchor {
  /** Which row, by identity. */
  readonly key: string;
  /** How far into it, from its leading edge. */
  readonly within: number;
}

/**
 * The anchor an offset names, given the current ordering.
 *
 * `undefined` when the list is empty — a state with no anchor is a real state, and inventing one
 * would be a key naming a row that does not exist.
 */
export function anchorAtOffset(
  table: ExtentTable,
  keys: readonly string[],
  offset: number,
): KeyedAnchor | undefined {
  if (keys.length === 0 || table.count === 0) return undefined;
  const anchor = table.anchorAt(offset);
  const key = keys[Math.min(anchor.index, keys.length - 1)];
  if (key === undefined) return undefined;
  return { key, within: anchor.within };
}

/**
 * The offset an anchor names **now**, after whatever happened to the rows above it.
 *
 * `undefined` when the anchored row is gone — the caller then decides, and the honest answers are
 * *stay where you are* (the row was deleted under a reader who was not looking at it) or *go to the
 * top*. Returning zero here would silently pick the second for both, which is a list that jumps to
 * the beginning whenever anything is deleted anywhere.
 */
export function offsetOfAnchor(
  table: ExtentTable,
  keys: readonly string[],
  anchor: KeyedAnchor,
): number | undefined {
  const index = keys.indexOf(anchor.key);
  if (index < 0) return undefined;
  return table.offsetOf(index) + anchor.within;
}

/**
 * The offset that keeps the anchored row where it is, clamped to what the list can actually scroll.
 *
 * The clamp matters and is easy to leave out: near the end of a list, removing rows above the reader
 * can make the wanted offset larger than the list is long, and a `scrollTop` write past the end is
 * silently clamped by the browser to a *different* number — so the model and the DOM would disagree
 * about where the list is, which is the state every subsequent window is computed from.
 */
export function stableOffsetFor(
  table: ExtentTable,
  keys: readonly string[],
  anchor: KeyedAnchor | undefined,
  viewportExtent: number,
  fallback: number,
): number {
  const furthest = Math.max(0, table.total - Math.max(0, viewportExtent));
  const wanted = anchor === undefined ? undefined : offsetOfAnchor(table, keys, anchor);
  const chosen = wanted ?? fallback;
  return Math.min(furthest, Math.max(0, chosen));
}
