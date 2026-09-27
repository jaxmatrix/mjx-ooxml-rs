/**
 * The windowed scroller the four navigators share — **one implementation, wired to the foundations.**
 *
 * ## Not a custom element
 *
 * A class the components own, not a fifth registered tag, and the reason is U06's third defect:
 * *custom elements upgrade in tree order*, so a scroller that was its own element would sometimes be
 * built before the component that has to fill it. A plain class is constructed by its owner, when
 * its owner is ready, and there is no upgrade order to get wrong. U07's `ListSurface` is the same
 * decision for the same reason.
 *
 * ## What it holds, and why each piece is where it is
 *
 * | Piece | Where | Why |
 * |---|---|---|
 * | which rows to build | `foundations/virtual-list.ts` | U06's arithmetic, lifted; a gate needs a number the renderer did not produce |
 * | how tall each row is | `foundations/extent-table.ts` | U11's prefix sums, lifted; rows here are **not** all one row tall |
 * | what a row looks like | the component | a treeitem, a slide and an option are three different drawings of one window |
 *
 * ## The two invariants, and they are true at different moments
 *
 * This is MJXOFF-190's structure applied to a list rather than to a document, and its table is the
 * clearest statement of why one rule is not enough:
 *
 * * **The rows changed** — an insertion, a removal, a reorder. The **anchor** is the state and the
 *   offset is derived, so a slide inserted above the viewport does not move what the reader is
 *   looking at. The anchor is a **key**, never an index, because an index is precisely what an
 *   insertion above it changes.
 * * **A row turned out to be a different height than it was guessed at.** The same rule, arriving by
 *   the other road: the correction moves every offset after it, so the offset is re-derived from the
 *   anchor rather than kept.
 *
 * Both are asserted against numbers in `tests/navigators.test.ts` and against a measured on-screen
 * position in `tests/browser/navigators.spec.ts` — **beside the naive answer**, which is `keep the
 * scroll offset`, so that a green assertion is known to be an assertion about something. That is
 * U11's rule and it is the difference between a stability gate and a tolerance nobody tested.
 */

import { ExtentTable } from '../foundations/extent-table.ts';
import {
  anchorAtOffset,
  defaultOverscanRows,
  offsetToReveal,
  rowsInWindow,
  stableOffsetFor,
  windowForOffset,
  windowSpacers,
  type KeyedAnchor,
  type VirtualWindow,
} from '../foundations/virtual-list.ts';

/** What a component must tell the scroller about its own rows. */
export interface VirtualRowSource {
  /** One stable key per row, in order. The identity an anchor is held by. */
  keys(): readonly string[];
  /** Build the element for row `index`, or `undefined` to build nothing there. */
  buildRow(index: number): HTMLElement | undefined;
  /**
   * The row the keyboard is on, or `-1`.
   *
   * ⚠ **It is always built, whatever the scroll offset says.** U07's third defect: the window is a
   * function of a *measured* offset, a measurement can be a frame behind, and the cursor then names
   * a row that was never built — at which point `aria-activedescendant` silently names nothing and
   * the announcement stops with the list looking perfectly fine.
   *
   * ⚠ **And it is built as ONE DETACHED ROW rather than by widening the window** — see
   * [`VirtualScroller.render`], where the measurement that forced that is written down.
   */
  cursorRow(): number;
}

/** One built row, so the scroller can find its element again without a query per keystroke. */
interface BuiltRow {
  readonly index: number;
  readonly key: string;
  readonly element: HTMLElement;
}

/** The elements a component hands over. */
export interface ScrollerParts {
  /** The scrolling box. Carries the container role. */
  readonly viewport: HTMLElement;
  /** The spacer standing in for every row before the window. */
  readonly leading: HTMLElement;
  /** The spacer standing in for every row after it. */
  readonly trailing: HTMLElement;
}

/** How tall a row is guessed at before anything measures one, in CSS pixels. */
export const defaultEstimatedRowExtent = 28;

export class VirtualScroller {
  readonly #parts: ScrollerParts;
  readonly #source: VirtualRowSource;
  readonly #estimate: number;
  /**
   * Every extent that has ever been measured, **by key**.
   *
   * ⚠ Keyed rather than indexed, and that is the whole reason an insertion is cheap: the rows below
   * the inserted one keep the heights they were measured at instead of reverting to the estimate,
   * so the list does not visibly re-settle every time something is added above it.
   */
  readonly #measured = new Map<string, number>();
  #keys: readonly string[] = [];
  #table: ExtentTable;
  #anchor: KeyedAnchor | undefined;
  #built: BuiltRow[] = [];
  /** The cursor's row when it is outside the window: built alone, at its own offset. */
  #detached: BuiltRow | undefined;
  #window: VirtualWindow = { firstRow: 0, lastRow: 0 };
  /** The offset the last `setKeys` would have kept, had it kept the offset. The positive control. */
  #naiveOffset = 0;
  /** Suppresses the scroll handler for a scroll this class caused. */
  #internal = false;
  #settling = false;

  constructor(parts: ScrollerParts, source: VirtualRowSource, estimate = defaultEstimatedRowExtent) {
    this.#parts = parts;
    this.#source = source;
    this.#estimate = Math.max(1, estimate);
    this.#table = new ExtentTable(0, this.#estimate);
    parts.viewport.addEventListener('scroll', this.#onScroll, { passive: true });
  }

  /** Stop listening. Called from `disconnectedCallback`. */
  dispose(): void {
    this.#parts.viewport.removeEventListener('scroll', this.#onScroll);
  }

  /** The extent table, so a gate reads the same numbers the code used. */
  get table(): ExtentTable {
    return this.#table;
  }

  /** The window currently built. */
  get builtWindow(): VirtualWindow {
    return this.#window;
  }

  /**
   * How many rows should exist: the window, plus the detached cursor row when there is one.
   *
   * The number a node-count gate compares against, and it comes from the foundations' own window
   * rather than from the renderer — U06's rule, and the only kind of number that assertion may use.
   */
  get expectedRowCount(): number {
    return rowsInWindow(this.#window) + (this.#detached === undefined ? 0 : 1);
  }

  /** Whether the cursor's row is currently built outside the window. */
  get cursorIsDetached(): boolean {
    return this.#detached !== undefined;
  }

  /** How many rows actually exist. The number the virtualisation gate compares. */
  get builtRowCount(): number {
    return this.#built.length;
  }

  /** How many rows have been measured rather than guessed — the anti-vacuity figure. */
  get measuredRowCount(): number {
    return this.#table.measuredCount;
  }

  /** Where the list is scrolled to. */
  get offset(): number {
    return this.#parts.viewport.scrollTop;
  }

  /** The anchor being held. */
  get anchor(): KeyedAnchor | undefined {
    return this.#anchor;
  }

  /**
   * What the offset would have been had the last row change simply kept it.
   *
   * ⚠ **The positive control, and it is exposed on purpose.** *"The row did not move"* is trivially
   * true of a list that never scrolls, of a list whose rows are all the same height, and of a change
   * that happened below the viewport. Both suites assert that this differs from the offset actually
   * taken, so a stability assertion is known to be about something.
   */
  get naiveOffset(): number {
    return this.#naiveOffset;
  }

  /** The element built for a row, if that row is in the window. */
  elementForRow(index: number): HTMLElement | undefined {
    return this.#built.find((row) => row.index === index)?.element;
  }

  /**
   * The rows changed. Rebuild the extent table around the new ordering and **hold the anchor.**
   *
   * The measured extents are carried across by key, so an insertion costs one estimate — the new
   * row's — rather than sending every row below it back to the guess.
   */
  setKeys(next: readonly string[]): void {
    const anchor = this.#anchor;
    const viewport = this.#parts.viewport;
    this.#naiveOffset = viewport.scrollTop;
    this.#keys = [...next];

    const table = new ExtentTable(this.#keys.length, this.#estimate);
    for (const [index, key] of this.#keys.entries()) {
      const measured = this.#measured.get(key);
      if (measured !== undefined) table.recordMeasured(index, measured);
    }
    this.#table = table;

    /*
     * Three steps, and the order is load-bearing in both directions.
     *
     * The first render gives the scroll container its new scrollHeight — a scrollTop write past the
     * current one is silently clamped by the browser to a *different* number, and the model and the
     * DOM would then disagree about where the list is, which is the state every subsequent window is
     * computed from.
     *
     * ⚠ **And the wanted offset is computed AFTER that render, not before it.** Rendering measures
     * the rows it built, so an offset computed first is an answer to extents the render has since
     * corrected. `tests/browser/navigators.spec.ts` measured what that costs: the watched row moved
     * **265 pixels** — far too little to look like a defect and far too much to be right — with the
     * anchor, the keys and the arithmetic all correct, because the answer was written one step
     * before the numbers it was an answer to.
     *
     * The third step makes the result visible in the same task, so a test does not have to wait a
     * frame to see the window the offset implies.
     */
    this.render();
    const wanted = stableOffsetFor(
      table,
      this.#keys,
      anchor,
      viewport.clientHeight,
      this.#naiveOffset,
    );
    this.#scrollTo(wanted);
    this.render();
  }

  /**
   * Rebuild the window from the current offset.
   *
   * ## ⚠ The cursor row is built **detached**, and the first version widened the window instead
   *
   * U07's answer to *the cursor must always be in the DOM* was `firstRow = min(firstRow, cursor)`,
   * and it is right for a font list: there the cursor moves **because** the list scrolled, so the
   * two are never far apart. Here they are: a pointer scrolls a five-thousand-row list with the
   * keyboard cursor still on row 0, and widening the window to reach it builds **every row in
   * between**.
   *
   * That is not a theory. `tests/browser/navigators.spec.ts` scrolled the list to its midpoint and
   * measured **2,504 rows in the DOM** — a virtualised list that had quietly stopped virtualising,
   * with the node count on the resting story still perfect and every other assertion in the suite
   * green. It is exactly the shape U06 warns about: a ceiling that is satisfied until the one
   * interaction nobody wrote a story for.
   *
   * So the cursor's row is built as a single absolutely-positioned element at its true offset when
   * it falls outside the window. The invariant is unchanged — the row `aria-activedescendant` names
   * is in the DOM, wherever the reader has scrolled to — and the cost is one element rather than
   * however many rows lie between the cursor and the viewport.
   */
  render(): void {
    const viewport = this.#parts.viewport;
    const table = this.#table;
    const measured = viewport.clientHeight;

    /*
     * ⚠ A viewport of zero is not *nothing is visible*, it is *nobody has laid this out yet* — a
     * component measured before its first frame, or one inside a closed sheet. U06's lesson in its
     * most literal form: a ceiling satisfied by zero. Falling back to a screenful of estimated rows
     * builds something, and the measurement one frame later corrects it.
     */
    const viewportExtent = measured > 0 ? measured : this.#estimate * 12;

    const window_ = windowForOffset(
      table,
      viewport.scrollTop,
      viewportExtent,
      defaultOverscanRows,
    );
    this.#window = window_;

    const spacers = windowSpacers(table, window_);
    this.#parts.leading.style.blockSize = `${String(spacers.leading)}px`;
    this.#parts.trailing.style.blockSize = `${String(spacers.trailing)}px`;

    for (const row of this.#built) row.element.remove();
    this.#built = [];

    const built: BuiltRow[] = [];
    const fragment = document.createDocumentFragment();
    for (let index = window_.firstRow; index < window_.lastRow; index += 1) {
      const element = this.#source.buildRow(index);
      if (element === undefined) continue;
      built.push({ index, key: this.#keys[index] ?? String(index), element });
      fragment.append(element);
    }
    this.#parts.trailing.before(fragment);

    const cursor = this.#source.cursorRow();
    this.#detached = undefined;
    if (cursor >= 0 && cursor < table.count && (cursor < window_.firstRow || cursor >= window_.lastRow)) {
      const element = this.#source.buildRow(cursor);
      if (element !== undefined) {
        element.dataset['detached'] = 'true';
        element.style.position = 'absolute';
        element.style.insetInlineStart = '0';
        element.style.insetBlockStart = `${String(table.offsetOf(cursor))}px`;
        viewport.append(element);
        const row = { index: cursor, key: this.#keys[cursor] ?? String(cursor), element };
        built.push(row);
        this.#detached = row;
      }
    }

    this.#built = built;

    this.#measure();
  }

  /**
   * Read back what the rows turned out to be, and hold the anchor across the correction.
   *
   * One pass, and bounded: the second pass would measure the elements the first pass built, so it
   * would find the values it has just recorded. What it must not do is leave the reader somewhere
   * else — a correction to a row *above* the anchor moves every offset after it, which is the same
   * defect an insertion causes and takes the same answer.
   */
  #measure(): void {
    if (this.#settling) return;
    let moved = false;
    for (const row of this.#built) {
      const extent = row.element.getBoundingClientRect().height;
      if (!(extent > 0)) continue;
      if (Math.abs(this.#table.extentOf(row.index) - extent) > 0.5) moved = true;
      this.#measured.set(row.key, extent);
      this.#table.recordMeasured(row.index, extent);
    }
    if (!moved) return;

    this.#settling = true;
    const spacers = windowSpacers(this.#table, this.#window);
    this.#parts.leading.style.blockSize = `${String(spacers.leading)}px`;
    this.#parts.trailing.style.blockSize = `${String(spacers.trailing)}px`;
    const anchor = this.#anchor;
    if (anchor !== undefined) {
      const wanted = stableOffsetFor(
        this.#table,
        this.#keys,
        anchor,
        this.#parts.viewport.clientHeight,
        this.#parts.viewport.scrollTop,
      );
      this.#scrollTo(wanted);
    }
    this.#settling = false;
  }

  /** Scroll so that a row is on screen, doing the least that achieves it. */
  scrollToRow(index: number): void {
    const viewport = this.#parts.viewport;
    const wanted = offsetToReveal(
      this.#table,
      index,
      viewport.clientHeight,
      viewport.scrollTop,
    );
    this.#scrollTo(wanted);
    this.render();
  }

  /** Take the anchor from where the list is now. Called after a scroll settles. */
  captureAnchor(): void {
    this.#anchor = anchorAtOffset(this.#table, this.#keys, this.#parts.viewport.scrollTop);
  }

  #scrollTo(offset: number): void {
    const viewport = this.#parts.viewport;
    if (Math.abs(viewport.scrollTop - offset) < 0.5) {
      this.captureAnchor();
      return;
    }
    this.#internal = true;
    viewport.scrollTop = offset;
    this.#internal = false;
    this.captureAnchor();
  }

  #onScroll = (): void => {
    if (this.#internal) return;
    this.captureAnchor();
    this.render();
  };
}
