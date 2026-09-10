/**
 * Where every row of a long list sits, when the rows are not all the same height.
 *
 * ## This is a lift, not a new idea
 *
 * MJXOFF-190 wrote `ScrollbarModel` in `src/furniture/scroll-model.ts`: a table of per-page heights,
 * each marked *estimated* or *exact*, lazily rebuilt prefix sums, a binary search from an offset to
 * an anchor, and `offsetOfAnchor` back again. That file is itself a mirror of R13's
 * `crates/mjx-view/src/scroll.rs`, and its reasoning — *rebuild the prefix sums rather than patch
 * them; binary search rather than walk; keep measured heights across a re-estimate* — is repeated
 * here rather than restated, because it is the same reasoning.
 *
 * MJXOFF-191 needed exactly that arithmetic for a **virtual list with variable item heights**, and
 * the ticket is explicit that a second implementation is the thing to avoid. So the arithmetic moved
 * **down** into the foundations, parameterised by nothing but *count* and *estimated extent*, and
 * `ScrollbarModel` is now a wrapper that adds the two things a scrollbar has and a list does not:
 * the word *page*, and a precision for the page **count** as opposed to each page's height.
 * `tests/navigators.test.ts` asserts the equivalence over a sweep of operation sequences rather
 * than leaving it as a claim — U11's own pattern, from `foundations/splitter.ts`.
 *
 * ## What a list needs that a scrollbar did not
 *
 * Two operations, and they are the reason this is a lift rather than a rename:
 *
 * * **`insertAt` / `removeAt`.** A scrollbar's document grows and shrinks *at the end*
 *   (`extendToAtLeast`, `recordExactPageCount`), so `ScrollbarModel` never had to move a row. A
 *   navigator's does not: a slide is inserted in the middle, a heading is dragged above another,
 *   a sheet is deleted. **The defect users actually hit lives in exactly that operation** — a slide
 *   inserted above the visible range must not move what they are looking at — and it cannot be
 *   expressed at all in a model that can only append.
 * * **A keyed anchor.** `ScrollAnchor` is `{ page, within }`, and an *index* is the wrong state to
 *   hold across an insertion above it: index 40 is a different row after a row is inserted at 3.
 *   `src/foundations/virtual-list.ts` holds `{ key, within }` instead and resolves the key through
 *   this table. The index-keyed anchor here is still right for a scrollbar, whose pages do not get
 *   reordered, so both live and neither is a copy of the other.
 *
 * ## Node-importable
 *
 * Numbers only. No DOM, no tokens, no CSS.
 */

/** Whether a figure came from laying something out, or from an estimate. R13's `ExtentPrecision`. */
export const extentPrecisionNames = ['estimated', 'exact'] as const;

/** One of the two. */
export type ExtentPrecision = (typeof extentPrecisionNames)[number];

/** One row's extent along the scrolling axis, and whether it is known or guessed. */
export interface ExtentMetric {
  readonly extent: number;
  readonly precision: ExtentPrecision;
}

/** A position expressed as *which row, and how far into it* — the form that survives a correction. */
export interface ExtentAnchor {
  /** Which row, counted from zero. */
  readonly index: number;
  /** How far into that row, from its leading edge. */
  readonly within: number;
}

/** The very beginning. */
export const extentOrigin: ExtentAnchor = { index: 0, within: 0 };

/**
 * A table of row extents, and every offset derived from it.
 *
 * Nothing here knows what a pixel is: extents are in whatever unit the caller supplies, exactly as
 * R13 works in `Emu` and `ScrollbarModel` works in whatever the shell hands it.
 */
export class ExtentTable {
  #rows: ExtentMetric[];
  #estimate: number;
  /**
   * Prefix sums, rebuilt lazily: `offsets[n]` is the leading edge of row `n`, and the last entry is
   * the whole table's extent. **Rebuilt rather than patched**, because a patch that missed one row
   * is a list that is wrong by exactly the amount nobody notices until they scroll to the end.
   */
  #offsets: number[] = [];
  #stale = true;

  /**
   * A table of `count` rows, each guessed at `estimate` until something measures it.
   *
   * `count` is floored at zero and `estimate` at zero: a list with no rows is a legitimate state
   * (an empty outline, a deck whose slides have not loaded) and must not be forced to have one.
   * That is the one deliberate divergence from `ScrollbarModel.fromExtent`, which floors at **one**
   * because a document with no pages still has a scrollbar — and the wrapper keeps doing that.
   */
  constructor(count: number, estimate: number) {
    const rows = Math.max(0, Math.floor(count));
    this.#estimate = Math.max(0, estimate);
    this.#rows = Array.from({ length: rows }, () => ({
      extent: this.#estimate,
      precision: 'estimated' as const,
    }));
  }

  /** How many rows the table believes there are. */
  get count(): number {
    return this.#rows.length;
  }

  /** What an unmeasured row is guessed at. */
  get estimate(): number {
    return this.#estimate;
  }

  /**
   * How many rows have been measured rather than guessed.
   *
   * ⚠ **The anti-vacuity figure, and the reason it is public.** R13: *"a stability assertion alone
   * is green for a model that never corrects anything."* A gate asserting only that a row did not
   * move would pass over a table that ignored every correction, so every such assertion in this
   * catalogue reads this beside it.
   */
  get measuredCount(): number {
    return this.#rows.filter((row) => row.precision === 'exact').length;
  }

  /** How long the whole table is. */
  get total(): number {
    this.#rebuild();
    return this.#offsets[this.#offsets.length - 1] ?? 0;
  }

  /** One row's metric, or `undefined` past the end. */
  metric(index: number): ExtentMetric | undefined {
    return this.#rows[Math.floor(index)];
  }

  /** One row's extent, or the estimate past the end. */
  extentOf(index: number): number {
    return this.#rows[Math.floor(index)]?.extent ?? this.#estimate;
  }

  /**
   * Where the leading edge of `index` sits, measured from the beginning.
   *
   * Past the last row this answers the table's own extent, so a caller asking about a row that has
   * just been removed is placed at the end rather than at zero.
   */
  offsetOf(index: number): number {
    this.#rebuild();
    const at = Math.max(0, Math.floor(index));
    return this.#offsets[at] ?? this.total;
  }

  /**
   * The anchor an offset names.
   *
   * A binary search over the prefix sums rather than a walk: a five-thousand-slide deck is a real
   * deck, a hundred-thousand-row outline is a real spreadsheet, and this runs on the scroll path.
   */
  anchorAt(offset: number): ExtentAnchor {
    this.#rebuild();
    if (!Number.isFinite(offset) || offset <= 0 || this.#rows.length === 0) return extentOrigin;
    const leadingEdges = this.#offsets.slice(0, this.#rows.length);
    let low = 0;
    let high = leadingEdges.length;
    while (low < high) {
      const middle = (low + high) >> 1;
      if ((leadingEdges[middle] ?? 0) <= offset) low = middle + 1;
      else high = middle;
    }
    const index = Math.max(0, low - 1);
    return { index, within: offset - (leadingEdges[index] ?? 0) };
  }

  /** The offset an anchor names. */
  offsetOfAnchor(anchor: ExtentAnchor): number {
    return this.offsetOf(anchor.index) + anchor.within;
  }

  /**
   * Replaces a row's guessed extent with the one it turned out to have.
   *
   * Idempotent, and cheap when the extent has not changed: a correction that agrees with the guess
   * still marks the row exact but leaves the prefix sums alone.
   */
  recordMeasured(index: number, extent: number): void {
    const at = Math.floor(index);
    const row = this.#rows[at];
    if (row === undefined) return;
    const changed = row.extent !== extent;
    this.#rows[at] = { extent, precision: 'exact' };
    if (changed) this.#stale = true;
  }

  /**
   * The table turned out to have this many rows. Truncates, or appends estimated rows.
   *
   * **Measured extents are kept**, which is R13's reasoning unchanged: a row that was not removed
   * did not change height, and throwing every measurement away would send the list back to its
   * estimate for a one-row edit.
   */
  setCount(count: number): void {
    const wanted = Math.max(0, Math.floor(count));
    if (this.#rows.length === wanted) return;
    if (this.#rows.length > wanted) this.#rows = this.#rows.slice(0, wanted);
    else {
      while (this.#rows.length < wanted) {
        this.#rows.push({ extent: this.#estimate, precision: 'estimated' });
      }
    }
    this.#stale = true;
  }

  /** The table turned out to be at least this long. Never shrinks. */
  extendToAtLeast(count: number): void {
    const wanted = Math.max(0, Math.floor(count));
    if (this.#rows.length < wanted) this.setCount(wanted);
  }

  /** A fresh guess at what an unmeasured row is worth. Measured rows keep their measurements. */
  reEstimate(estimate: number): void {
    const next = Math.max(0, estimate);
    if (next === this.#estimate) return;
    this.#estimate = next;
    let changed = false;
    this.#rows = this.#rows.map((row) => {
      if (row.precision === 'exact') return row;
      changed = true;
      return { extent: next, precision: 'estimated' as const };
    });
    if (changed) this.#stale = true;
  }

  /**
   * `count` new rows appear at `index`, each at the current estimate.
   *
   * ⚠ **This is the operation a scrollbar never needed and a navigator cannot do without.** Every
   * row from `index` onward moves, which is precisely the moment a list that held a raw offset
   * shows a reader something they were not looking at.
   */
  insertAt(index: number, count: number): void {
    const many = Math.max(0, Math.floor(count));
    if (many === 0) return;
    const at = Math.min(this.#rows.length, Math.max(0, Math.floor(index)));
    const fresh: ExtentMetric[] = Array.from({ length: many }, () => ({
      extent: this.#estimate,
      precision: 'estimated' as const,
    }));
    this.#rows.splice(at, 0, ...fresh);
    this.#stale = true;
  }

  /** `count` rows disappear from `index`. */
  removeAt(index: number, count: number): void {
    const many = Math.max(0, Math.floor(count));
    if (many === 0) return;
    const at = Math.min(this.#rows.length, Math.max(0, Math.floor(index)));
    if (at >= this.#rows.length) return;
    this.#rows.splice(at, many);
    this.#stale = true;
  }

  #rebuild(): void {
    if (!this.#stale) return;
    const offsets: number[] = [];
    let running = 0;
    for (const row of this.#rows) {
      offsets.push(running);
      running += row.extent;
    }
    offsets.push(running);
    this.#offsets = offsets;
    this.#stale = false;
  }
}
