/**
 * The scrollbar's half of R13's viewport contract: where every page sits, and why the thumb does
 * not jump.
 *
 * ## This is a mirror, and the thing it mirrors has landed
 *
 * MJXOFF-168 (R13) shipped `crates/mjx-view/src/scroll.rs`, and MJXOFF-190's ticket says the
 * scrollbar *"must express what R13 provides"* — an estimated extent that is corrected as real
 * page heights arrive, **without the thumb jumping under the user's grip**. That file states the
 * problem exactly, and this one states the same design in the language the chrome is written in:
 *
 * > A [`ScrollAnchor`] is *a page and how far into it*, not a document offset. It is what a reader
 * > actually means by "where I am": page 47, a third of the way down. A raw offset means the same
 * > thing only while the pages before it keep the heights they were guessed at.
 *
 * The names here are the Rust names in this language's spelling — `totalHeight`, `offsetOf`,
 * `anchorAt`, `offsetOfAnchor`, `recordMeasuredHeight`, `recordExactPageCount`, `extendToAtLeast`,
 * `reEstimate`, `measuredPages` — so a reader holding both files open can check them against each
 * other line by line. **It is deliberately not a translation of the arithmetic by feel**: the
 * prefix sums are rebuilt rather than patched, for the reason R13 gives, and `anchorAt` binary
 * searches rather than walking, for the reason R13 gives.
 *
 * ## The trap, and the two halves of it
 *
 * A correction moves the offsets of every page after it. There are therefore **two** invariants,
 * and they are held by two different mechanisms because they are true at different moments:
 *
 * * **Not dragging**, the *anchor* is the state and the offset is derived, so the page under the
 *   reader does not move when a page above it turns out to be taller than the guess. That is
 *   [`ScrollbarModel`] below and it is R13's design exactly.
 * * **Dragging**, the *pointer* is the state and the offset is derived, so the point of the thumb
 *   under the finger does not move when the extent is revised mid-drag. That is
 *   [`offsetUnderGrip`] and it is this file's own half: R13 never sees a pointer.
 *
 * Both are asserted in `tests/furniture.test.ts` against numbers, and again in
 * `tests/browser/furniture.spec.ts` against a measured thumb — beside a recomputation of what the
 * **naive** answer would have been, so that a green assertion is known to be an assertion about
 * something.
 *
 * ## Node-importable
 *
 * Data and pure functions. No DOM, no tokens, no CSS.
 */

import {
  ExtentTable,
  extentPrecisionNames,
  type ExtentPrecision,
} from '../foundations/extent-table.ts';

/**
 * Whether a figure came from laying something out, or from an estimate. R13's `ExtentPrecision`.
 *
 * ⚠ **Re-exported rather than declared.** MJXOFF-191 lifted the prefix-sum table this file was
 * built around into `src/foundations/extent-table.ts`, so a virtual list of five thousand slides
 * and a scrollbar over five hundred pages share one implementation of *rows of different heights,
 * some measured and some guessed*. The names here are unchanged and every caller still compiles.
 */
export { extentPrecisionNames };
export type { ExtentPrecision };

/** Where a reader is, in terms that survive a correction. R13's `ScrollAnchor`. */
export interface ScrollAnchor {
  /** Which page the top of the viewport is inside, counted from zero. */
  readonly page: number;
  /** How far down that page, from its top edge. */
  readonly within: number;
}

/** The very top of the document. */
export const scrollOrigin: ScrollAnchor = { page: 0, within: 0 };

/** One page's height, and whether it is known or guessed. R13's `PageMetric`. */
export interface PageMetric {
  readonly height: number;
  readonly precision: ExtentPrecision;
}

/** A box model's guess at how long a document is. R13's `Extent`, in the two fields we need. */
export interface Extent {
  /** How many pages. Never below one: a document with no pages still has a scrollbar. */
  readonly pages: number;
  /** How tall each of them is guessed to be. */
  readonly pageHeight: number;
  /** Whether the **count** is known, as opposed to each page's height. */
  readonly precision: ExtentPrecision;
}

/**
 * Where every page sits, and how much document there is.
 *
 * The state is the page table; every offset is derived from it. Nothing here knows what a pixel
 * is: heights are in whatever unit the caller supplies, exactly as R13 works in `Emu`.
 *
 * ## What is left of it after MJXOFF-191's lift
 *
 * The arithmetic — the lazily rebuilt prefix sums, the binary search from an offset to an anchor,
 * `offsetOfAnchor` back again, and *measured heights survive a re-estimate* — is
 * [`ExtentTable`], in the foundations. What stayed here is what a **scrollbar** has and a list does
 * not: the word *page*, the floor of one page, and a precision for the page **count** as opposed to
 * each page's height.
 *
 * `tests/navigators.test.ts` sweeps random operation sequences through this and through a bare
 * `ExtentTable` and requires them to agree, which is MJXOFF-190's own rule for a lift: *if a binding
 * ever drifts, the two callers disagree about what the same operation does, which is exactly the
 * defect two copies would have had.*
 */
export class ScrollbarModel {
  readonly #table: ExtentTable;
  #countPrecision: ExtentPrecision;

  private constructor(table: ExtentTable, precision: ExtentPrecision) {
    this.#table = table;
    this.#countPrecision = precision;
  }

  /** A model drawn from an estimate. Every page starts at the guessed height, marked estimated. */
  static fromExtent(extent: Extent): ScrollbarModel {
    const count = Math.max(1, Math.floor(extent.pages));
    return new ScrollbarModel(
      new ExtentTable(count, Math.max(0, extent.pageHeight)),
      extent.precision,
    );
  }

  /** How many pages the model believes there are. */
  get pageCount(): number {
    return this.#table.count;
  }

  /** Whether the page **count** is known rather than guessed. */
  get countPrecision(): ExtentPrecision {
    return this.#countPrecision;
  }

  /**
   * How many pages have had their height measured rather than guessed.
   *
   * ⚠ **The figure that says how much of the scrollbar is real, and the reason it is exposed.**
   * R13: *"a stability assertion alone is green for a model that never corrects anything."* A
   * gate that asserted only that the thumb did not move would pass over a model that ignored every
   * correction, so every such assertion in this catalogue reads this beside it.
   */
  get measuredPages(): number {
    return this.#table.measuredCount;
  }

  /** One page's metric, or `undefined` past the end. */
  metric(page: number): PageMetric | undefined {
    const row = this.#table.metric(page);
    if (row === undefined) return undefined;
    return { height: row.extent, precision: row.precision };
  }

  /** How tall the whole document is. */
  get totalHeight(): number {
    return this.#table.total;
  }

  /**
   * Where the top of `page` is, measured from the top of the document.
   *
   * Past the last page this answers the document's own height, so a caller asking about a page
   * that has just been removed is placed at the end rather than at zero.
   */
  offsetOf(page: number): number {
    return this.#table.offsetOf(page);
  }

  /**
   * The anchor a document offset names.
   *
   * The **one** conversion from an offset into an anchor, done once — when the reader lets go of
   * the scrollbar, or when the shell restores a saved position. Doing it every frame would put the
   * offset back in charge and undo the whole design.
   */
  anchorAt(offset: number): ScrollAnchor {
    const anchor = this.#table.anchorAt(offset);
    return { page: anchor.index, within: anchor.within };
  }

  /** The document offset an anchor names. */
  offsetOfAnchor(anchor: ScrollAnchor): number {
    return this.#table.offsetOfAnchor({ index: anchor.page, within: anchor.within });
  }

  /**
   * Replaces `page`'s guessed height with the one it turned out to have.
   *
   * Idempotent, and cheap when the height has not changed: a correction that agrees with the guess
   * still marks the page exact but leaves the prefix sums alone.
   */
  recordMeasuredHeight(page: number, height: number): void {
    this.#table.recordMeasured(page, height);
  }

  /** The document turned out to have exactly this many pages. */
  recordExactPageCount(pages: number): void {
    this.#table.setCount(Math.max(1, Math.floor(pages)));
    this.#countPrecision = 'exact';
  }

  /** The document turned out to be at least this long. Never shrinks. */
  extendToAtLeast(pages: number): void {
    this.#table.extendToAtLeast(Math.max(1, Math.floor(pages)));
  }

  /**
   * The document changed shape, and this is the box model's fresh guess at how long it is now.
   *
   * **Measured heights are kept, deliberately** — R13's reasoning, unchanged: a page whose
   * fragments were suppressed will be laid out again and will report its height again; a page that
   * was not suppressed did not move. Throwing them all away would send the scrollbar back to the
   * estimate for a one-character edit.
   */
  reEstimate(extent: Extent): void {
    this.#table.setCount(Math.max(1, Math.floor(extent.pages)));
    this.#countPrecision = extent.precision;
  }
}

// ── the geometry, which is what a person actually sees ───────────────────────

/**
 * The smallest share of the track a thumb may take.
 *
 * ⚠ **Not a taste.** A thumb sized honestly at `viewport / total` over a five-hundred-page document
 * is under a pixel tall, which is a control nobody can grab — and the identity-value trap in its
 * purest form, because such a thumb also *looks* fine in a screenshot of a short document. The
 * floor is expressed as a fraction rather than in pixels so that it is a property of the model
 * rather than of a rendering, and `tests/furniture.test.ts` asserts the ratio it starts biting at.
 */
export const minimumThumbFraction = 0.08;

/** How much document there is that the viewport cannot already see. */
export function scrollableExtent(viewport: number, total: number): number {
  return Math.max(0, total - viewport);
}

/**
 * The thumb's size, as a fraction of the track.
 *
 * The one number a scrollbar exists to communicate: *how much of this document am I looking at.*
 * A gate that measures it at one content-to-viewport ratio measures nothing, which is why the
 * suites sweep several — including one where the content barely overflows and one where it
 * massively does.
 */
export function thumbFraction(viewport: number, total: number): number {
  if (!(total > 0) || !(viewport > 0)) return 1;
  if (viewport >= total) return 1;
  return Math.min(1, Math.max(minimumThumbFraction, viewport / total));
}

/** Whether there is anything to scroll at all. A scrollbar over content that fits is not one. */
export function scrollable(viewport: number, total: number): boolean {
  return scrollableExtent(viewport, total) > 0;
}

/** Where the thumb's leading edge sits, as a fraction of the track. */
export function thumbStart(offset: number, viewport: number, total: number): number {
  const travel = 1 - thumbFraction(viewport, total);
  const scrollableBy = scrollableExtent(viewport, total);
  if (scrollableBy <= 0 || travel <= 0) return 0;
  const along = Math.min(1, Math.max(0, offset / scrollableBy));
  return along * travel;
}

/** The offset a thumb at this position along the track names. The inverse of `thumbStart`. */
export function offsetAtThumbStart(start: number, viewport: number, total: number): number {
  const travel = 1 - thumbFraction(viewport, total);
  const scrollableBy = scrollableExtent(viewport, total);
  if (scrollableBy <= 0 || travel <= 0) return 0;
  const along = Math.min(1, Math.max(0, start / travel));
  return along * scrollableBy;
}

/**
 * **The trap, answered.** The offset that keeps the point of the thumb under the finger under the
 * finger.
 *
 * `pointer` is where along the track the pointer is, as a fraction; `grip` is where within the
 * thumb it landed when the drag began, also as a fraction. The thumb's *size* may change while the
 * extent is being revised — that is the whole point of a scrollbar over an estimate — so what must
 * be held constant is not the thumb's leading edge but the point the finger is on:
 *
 * ```
 * thumbStart + grip × thumbFraction === pointer
 * ```
 *
 * Solving that for the offset, rather than keeping the offset and recomputing the thumb from it,
 * is the entire difference between this component and a generic scrollbar. Keeping the offset is
 * the **naive** recomputation the ticket asks to be proved failable, and both suites compute it
 * beside the real answer to show that it differs.
 */
export function offsetUnderGrip(
  pointer: number,
  grip: number,
  viewport: number,
  total: number,
): number {
  const size = thumbFraction(viewport, total);
  const travel = 1 - size;
  const wantedStart = pointer - grip * size;
  const clamped = Math.min(travel, Math.max(0, wantedStart));
  return offsetAtThumbStart(clamped, viewport, total);
}

/** Where the finger is on the track, given a thumb of this size at this position. */
export function gripPoint(start: number, grip: number, size: number): number {
  return start + grip * size;
}

// ── the marks, which are what makes a long document navigable ────────────────

/** What a mark on the track can be about. Three, because Office's channel carries three. */
export const scrollMarkKindNames = ['search', 'comment', 'change'] as const;

/** One of the three. */
export type ScrollMarkKind = (typeof scrollMarkKindNames)[number];

/** Whether a value names one of the three. */
export function isScrollMarkKind(value: unknown): value is ScrollMarkKind {
  return (scrollMarkKindNames as readonly string[]).includes(String(value));
}

/** One thing worth going to, somewhere in the document. */
export interface ScrollMark {
  readonly kind: ScrollMarkKind;
  /** Which page it is on, counted from zero. */
  readonly page: number;
  /** How far down that page, as a fraction of the page's height. */
  readonly within: number;
  /** What it is, announced. */
  readonly label: string;
}

/**
 * Where a mark sits on the track, as a fraction of the whole track.
 *
 * ⚠ **Over the whole track, not over the thumb's travel.** A mark answers *where in the document*,
 * and a reader compares it against the thumb's own extent to see whether it is on screen — which
 * only works if both are measured against the same track. Mapping a mark through the thumb's
 * travel instead would put the last mark in a long document a thumb's height short of the end.
 */
export function markFraction(model: ScrollbarModel, mark: ScrollMark): number {
  const total = model.totalHeight;
  if (!(total > 0)) return 0;
  const metric = model.metric(mark.page);
  const height = metric?.height ?? 0;
  const offset = model.offsetOf(mark.page) + height * Math.min(1, Math.max(0, mark.within));
  return Math.min(1, Math.max(0, offset / total));
}

/** How a mark is announced, so a screen reader is told what the channel holds. */
export function marksSummary(marks: readonly ScrollMark[]): string {
  if (marks.length === 0) return 'No marks';
  const counts = scrollMarkKindNames
    .map((kind) => ({ kind, count: marks.filter((mark) => mark.kind === kind).length }))
    .filter((entry) => entry.count > 0)
    .map((entry) => `${String(entry.count)} ${scrollMarkNouns[entry.kind](entry.count)}`);
  return counts.join(', ');
}

/** The noun each kind is counted in. Singular and plural, because "1 search results" is a defect. */
export const scrollMarkNouns: Readonly<Record<ScrollMarkKind, (count: number) => string>> = {
  search: (count) => (count === 1 ? 'search result' : 'search results'),
  comment: (count) => (count === 1 ? 'comment' : 'comments'),
  change: (count) => (count === 1 ? 'tracked change' : 'tracked changes'),
};
