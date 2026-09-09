/**
 * The popup list a dropdown and a combo box share — **a `listbox`, and never a `menu`.**
 *
 * MJXOFF-186 states the distinction and it is not pedantry: a menu is a set of *commands* whose
 * items take focus and whose pattern is `role="menu"`/`role="menuitem"` with a roving tab stop; a
 * listbox is a set of *values* one of which is chosen, whose options never take focus at all and
 * whose pattern is `role="listbox"`/`role="option"` with `aria-activedescendant`. A dropdown built
 * out of `<mjx-menu>` would announce "menu, Cambria, menu item" where a person needs "combo box,
 * Cambria, 4 of 18", and would give the wrong keys: a menu closes on Escape and a listbox reverts.
 *
 * U05's rule decides the rest, and this obeys it: **the number of tab stops decides
 * trap-versus-disclosure.** Focus stays on the field, so the whole control is one tab stop, so
 * `Tab` is free to mean *leave*, so leaving closes the list. `inputFocusPatterns` in the model says
 * so and `tests/browser/inputs.spec.ts` counts the stops with real `Tab` presses.
 *
 * ## Not a custom element
 *
 * This is a class the two components own, not a ninth registered tag, and the reason is U06's
 * third defect: *custom elements upgrade in tree order*, so a popup that was its own element would
 * sometimes be built before the component that had to fill it. A plain class is constructed by its
 * owner, when its owner is ready, and there is no upgrade order to get wrong.
 *
 * ## Virtualisation
 *
 * A font list is four hundred rows and only eight of them are ever visible, so the rows outside the
 * window are **heights rather than elements**. The plan comes from U06's `galleryRowPlan` at one
 * column and the window from its `galleryWindow`, so the node-count gate compares against
 * `cellsInWindow` — a number produced by something other than the code that built the nodes, which
 * is the only kind of number that assertion may use.
 *
 * ⚠ **Every row in the plan is exactly one row tall, headings included.** The virtualiser turns a
 * scroll offset into a row index by dividing, so a heading that sized to its own content would put
 * every row below it at the wrong index — and the symptom would be a list that scrolls to
 * *nearly* the right option, which nobody reports as a bug.
 */

import {
  applyPlacement,
  clearPlacement,
  clippingAncestor,
  clippingBoundary,
  floatingProperties,
  placeFloating,
  rectOf,
  resolveLength,
  syncTopLayer,
  type Align,
  type Direction,
  type LogicalSide,
  type Placement,
  type Rect,
} from '../overlay/floating.ts';
import { accessibleHitTargetMinimum } from '../foundations/density.ts';
import {
  cellsInWindow,
  galleryRowPlan,
  galleryWindow,
  inputBoxProperties,
  inputTypeClass,
  listMotionClass,
  listboxVisibleRows,
  type OptionDescriptor,
} from './input-model.ts';

/** Where a list opens relative to its field, and how it lines up. */
export const listSide: LogicalSide = 'blockEnd';
export const listAlign: Align = 'start';

/** What the surface tells its owner. The owner turns these into the model's events. */
export interface ListSurfaceHost {
  /** The option the pointer chose. */
  chose(option: OptionDescriptor, index: number): void;
  /** The pointer moved the keyboard cursor. */
  movedActive(index: number): void;
  /**
   * A last look at a row the surface has just built.
   *
   * Added by MJXOFF-187 so a font picker can draw each family's name **in its own face** and hang
   * its substitution mark on the row, without a second list surface existing. The alternative was
   * a fork, and a fork is two places where the virtualiser's one-row-tall precondition has to stay
   * true.
   *
   * ⚠ **A decorator may not change the row's height.** The virtualiser turns a scroll offset into
   * a row index by dividing, so a decoration that grew a row would put every row below it at the
   * wrong index — the failure U07 found and made structural, and the one a list of faces walks
   * straight back into. `.option`'s explicit `block-size` is what holds it; the decorator adds
   * content *inside* that box and never around it.
   */
  decorateOption?(row: HTMLElement, option: OptionDescriptor, index: number): void;
}

/**
 * What a **field** needs of the thing that pops up under it.
 *
 * `MjxListField` drives a listbox and a colour grid through exactly this, which is what lets the
 * two share one field, one ARIA combobox contract, one dismissal model and one placement — while
 * differing in the only thing they genuinely differ in, which is what a row looks like.
 *
 * It is deliberately the *smallest* set of members the field actually calls. A wider interface
 * would have made `ListSurface`'s virtualisation accessors part of the contract, and a colour grid
 * of eighty swatches does not virtualise and should not be made to pretend it does.
 */
export interface PopupSurface {
  readonly element: HTMLElement;
  readonly open: boolean;
  readonly options: readonly OptionDescriptor[];
  readonly activeIndex: number;
  readonly activeOption: OptionDescriptor | undefined;
  readonly activeDescendantId: string | undefined;
  setOptions(options: readonly OptionDescriptor[]): void;
  setSelected(value: string | undefined): void;
  setActive(index: number): void;
  show(anchor: HTMLElement, direction: Direction): void;
  place(anchor: HTMLElement, direction: Direction): void;
  hide(): void;
  dispose(): void;
}

/** One built row, so the surface can find its element again without a query per keystroke. */
interface BuiltRow {
  readonly index: number;
  readonly element: HTMLElement;
}

export class ListSurface implements PopupSurface {
  readonly #list: HTMLElement;
  readonly #top: HTMLElement;
  readonly #bottom: HTMLElement;
  readonly #host: ListSurfaceHost;
  readonly #idPrefix: string;

  #options: readonly OptionDescriptor[] = [];
  #sectioned = false;
  #selected: string | undefined;
  #active = -1;
  #built: BuiltRow[] = [];
  #rowHeight = accessibleHitTargetMinimum;
  #open = false;
  #placement: Placement | undefined;
  #anchor: Rect | undefined;
  #natural: { width: number; height: number } | undefined;
  #observer: ResizeObserver | undefined;
  #windowStart = 0;
  #windowEnd = 0;

  constructor(host: ListSurfaceHost, idPrefix: string) {
    this.#host = host;
    this.#idPrefix = idPrefix;

    const list = document.createElement('div');
    list.className = `list ${listMotionClass}`;
    list.setAttribute('part', 'list');
    list.setAttribute('role', 'listbox');
    list.dataset['open'] = 'false';
    list.addEventListener('scroll', this.#onScroll, { passive: true });
    list.addEventListener('pointerdown', this.#onPointerDown);
    list.addEventListener('pointerover', this.#onPointerOver);

    const top = document.createElement('div');
    top.className = 'spacer';
    top.setAttribute('aria-hidden', 'true');
    const bottom = document.createElement('div');
    bottom.className = 'spacer';
    bottom.setAttribute('aria-hidden', 'true');

    list.append(top, bottom);
    this.#list = list;
    this.#top = top;
    this.#bottom = bottom;
  }

  /** The listbox element, to be appended to the owner's shadow root. */
  get element(): HTMLElement {
    return this.#list;
  }

  /** Whether the list is showing. */
  get open(): boolean {
    return this.#open;
  }

  /** The options currently in the list — already filtered, if the owner filters. */
  get options(): readonly OptionDescriptor[] {
    return this.#options;
  }

  /** Which option the keyboard is on, or `-1`. */
  get activeIndex(): number {
    return this.#active;
  }

  /** The option the keyboard is on, or `undefined`. */
  get activeOption(): OptionDescriptor | undefined {
    return this.#active < 0 ? undefined : this.#options[this.#active];
  }

  /** The id `aria-activedescendant` should carry, or `undefined` when nothing is active. */
  get activeDescendantId(): string | undefined {
    if (this.#active < 0 || this.#active >= this.#options.length) return undefined;
    return this.#optionId(this.#active);
  }

  /** The placement the last open produced, for a gate to re-run `placeFloating` against. */
  get placement(): Placement | undefined {
    return this.#placement;
  }

  /** The anchor the last placement was computed against. */
  get anchorRect(): Rect | undefined {
    return this.#anchor;
  }

  /** The natural size the last placement was computed for. */
  get naturalSize(): { readonly width: number; readonly height: number } | undefined {
    return this.#natural;
  }

  /** The boundary the last placement had to stay inside. */
  get boundaryRect(): Rect {
    return clippingBoundary(this.#list, resolveLength(this.#list, floatingProperties.boundaryInset));
  }

  /** The rows built right now — the number the virtualisation gate compares. */
  get builtRowCount(): number {
    return this.#built.length;
  }

  /** The window the plan is currently showing, so a gate reads the same numbers the code used. */
  get builtWindow(): { readonly firstRow: number; readonly lastRow: number } {
    return { firstRow: this.#windowStart, lastRow: this.#windowEnd };
  }

  /** The row plan, from U06's planner at one column. */
  get rowPlan(): ReturnType<typeof galleryRowPlan> {
    return galleryRowPlan(this.#categories(), 1, this.#sectioned);
  }

  /** How many cells the current window should hold, per the shared planner. */
  get expectedCellCount(): number {
    return cellsInWindow(this.rowPlan, {
      firstRow: this.#windowStart,
      lastRow: this.#windowEnd,
    });
  }

  #categories(): string[] {
    return this.#options.map((option) => option.category ?? '');
  }

  #optionId(index: number): string {
    return `${this.#idPrefix}-option-${String(index)}`;
  }

  /** Replace the options. Keeps the active index inside the new list, or drops it. */
  setOptions(options: readonly OptionDescriptor[]): void {
    this.#options = options;
    this.#sectioned = options.some((option) => (option.category ?? '') !== '');
    if (this.#active >= options.length) this.#active = options.length - 1;
    this.#render();
  }

  /** Set which option is chosen. `undefined` chooses none. */
  setSelected(value: string | undefined): void {
    this.#selected = value;
    this.#render();
  }

  /**
   * Move the keyboard cursor, scroll the row into view, and rebuild the window around it.
   *
   * The scroll happens **before** the rebuild, because the window is a function of the scroll
   * offset: rebuilding first and scrolling second would build one window and show another, and the
   * row `aria-activedescendant` names would be a row that is not in the DOM. That is the specific
   * way virtualisation and `aria-activedescendant` break each other, and it is silent — the
   * announcement simply stops.
   */
  setActive(index: number): void {
    const clamped = index < 0 ? -1 : Math.min(index, this.#options.length - 1);
    this.#active = clamped;
    if (clamped >= 0) this.#scrollRowIntoView(this.#rowOf(clamped));
    this.#render();
  }

  /** The plan row that holds an option, so a scroll can be computed without a DOM read. */
  #rowOf(optionIndex: number): number {
    const rows = this.rowPlan;
    for (const [row, entry] of rows.entries()) {
      if (entry.kind === 'cells' && optionIndex >= entry.start && optionIndex < entry.end) {
        return row;
      }
    }
    return 0;
  }

  #scrollRowIntoView(row: number): void {
    const height = this.#rowHeight;
    const top = row * height;
    const view = this.#list.clientHeight;
    if (view <= 0) return;
    if (top < this.#list.scrollTop) this.#list.scrollTop = top;
    else if (top + height > this.#list.scrollTop + view) {
      this.#list.scrollTop = top + height - view;
    }
  }

  #onScroll = (): void => {
    this.#render();
  };

  #onPointerOver = (event: PointerEvent): void => {
    const row = this.#rowIn(event.composedPath());
    if (row === undefined || row.index === this.#active) return;
    this.#active = row.index;
    this.#render();
    this.#host.movedActive(row.index);
  };

  #onPointerDown = (event: PointerEvent): void => {
    const row = this.#rowIn(event.composedPath());
    if (row === undefined) return;
    // ⚠ `pointerdown` and not `click`, and `preventDefault` with it. A click on an option would
    // otherwise move focus off the field first, the field's own blur handler would close the list,
    // and the click would land on nothing. Choosing on the press and refusing the focus change is
    // what makes a mouse choice and a keyboard choice the same code path.
    event.preventDefault();
    const option = this.#options[row.index];
    if (option === undefined) return;
    if (option.unavailable === true) return;
    this.#host.chose(option, row.index);
  };

  #rowIn(path: readonly EventTarget[]): BuiltRow | undefined {
    for (const node of path) {
      if (node === this.#list) return undefined;
      const found = this.#built.find((row) => row.element === node);
      if (found !== undefined) return found;
    }
    return undefined;
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  #render(): void {
    const rows = this.rowPlan;
    const height = this.#rowHeight;
    const view = this.#list.clientHeight;
    const visibleRows = view > 0 ? Math.ceil(view / height) : listboxVisibleRows;
    const firstVisible = view > 0 ? Math.floor(this.#list.scrollTop / height) : 0;
    const window_ = galleryWindow(rows.length, firstVisible, visibleRows);

    /*
     * ⚠ **The row `aria-activedescendant` names is always built, whatever the scroll offset says.**
     *
     * The window is a function of a *measured* scroll offset, and a measurement can be a frame
     * behind the thing that caused it — a programmatic `scrollTop` write, a cap that was computed
     * before the rows had their real height, a `PageDown` that jumps ten rows at once. When it is,
     * the cursor points at a row that was never created, and `aria-activedescendant` silently
     * names nothing: the announcement stops and the list looks fine.
     *
     * `tests/browser/inputs.spec.ts` found exactly that on `PageDown` through a thirty-seven-row
     * list. The fix is not a better measurement — it is to stop the invariant depending on one.
     */
    let firstRow = window_.firstRow;
    let lastRow = window_.lastRow;
    if (this.#active >= 0) {
      const cursor = this.#rowOf(this.#active);
      firstRow = Math.min(firstRow, cursor);
      lastRow = Math.max(lastRow, cursor + 1);
    }
    this.#windowStart = firstRow;
    this.#windowEnd = lastRow;

    this.#top.style.blockSize = `${String(firstRow * height)}px`;
    this.#bottom.style.blockSize = `${String((rows.length - lastRow) * height)}px`;

    for (const row of this.#built) row.element.remove();
    this.#built = [];

    const built: BuiltRow[] = [];
    const fragment = document.createDocumentFragment();
    for (let row = firstRow; row < lastRow; row += 1) {
      const entry = rows[row];
      if (entry === undefined) continue;
      if (entry.kind === 'heading') {
        fragment.append(this.#buildHeading(entry.category));
        continue;
      }
      for (let index = entry.start; index < entry.end; index += 1) {
        const option = this.#options[index];
        if (option === undefined) continue;
        const element = this.#buildOption(option, index);
        built.push({ index, element });
        fragment.append(element);
      }
    }
    this.#bottom.before(fragment);
    this.#built = built;

    /*
     * ⚠ **`aria-setsize` goes on the options and never on the listbox**, and this line used to put
     * it on both. It is a *position within a set* attribute: it says "this is one of 37", which a
     * container cannot be. axe's `aria-allowed-attr` rule caught it on four stories at once, which
     * is the argument for running the sweep over the built catalogue rather than over the stories
     * somebody remembered to check.
     *
     * The count a screen reader announces still comes from the rows, which carry `aria-posinset`
     * and `aria-setsize` themselves — and it is a virtualised list's *only* honest source for it,
     * because the number of rows in the DOM is not the number of options.
     */

    // The row height is *measured*, not assumed — a compact list is shorter than a comfortable one
    // and the density mode is a class on an ancestor this surface cannot see. One read per render,
    // from a row that already exists.
    const sample = built[0]?.element;
    if (sample !== undefined) {
      const measured = sample.getBoundingClientRect().height;
      if (measured > 0 && Math.abs(measured - this.#rowHeight) > 0.5) {
        this.#rowHeight = measured;
        // One re-run, with the true height. Bounded because the second pass measures the same
        // element the first pass built, so the value it finds is the value it just used.
        this.#top.style.blockSize = `${String(firstRow * measured)}px`;
        this.#bottom.style.blockSize = `${String((rows.length - lastRow) * measured)}px`;
      }
    }
  }

  #buildHeading(category: string): HTMLElement {
    const heading = document.createElement('div');
    heading.className = `section ${inputTypeClass('sectionHeading')}`;
    heading.setAttribute('role', 'presentation');
    // The height comes from `.section` in the sheet, not from an inline style: two places that
    // both set it is two places that can disagree, and the virtualiser divides by only one of them.
    heading.textContent = category;
    return heading;
  }

  #buildOption(option: OptionDescriptor, index: number): HTMLElement {
    const row = document.createElement('div');
    row.className = `option ${inputTypeClass('optionLabel')}`;
    row.setAttribute('part', 'option');
    row.setAttribute('role', 'option');
    row.id = this.#optionId(index);
    row.setAttribute('aria-selected', String(option.value === this.#selected));
    row.setAttribute('aria-posinset', String(index + 1));
    row.setAttribute('aria-setsize', String(this.#options.length));
    if (index === this.#active) row.dataset['active'] = '';
    if (option.unavailable === true) {
      row.setAttribute('aria-disabled', 'true');
      if (option.explanation !== undefined && option.explanation !== '') {
        // Announced as part of the row rather than through `aria-describedby`: an option is not
        // focusable, so a description associated with it is never visited on its own.
        const reason = document.createElement('span');
        reason.className = 'visually-hidden';
        reason.textContent = `, ${option.explanation}`;
        row.append(reason);
      }
    }

    const label = document.createElement('span');
    label.className = 'option-label';
    label.textContent = option.label;
    row.append(label);

    if (option.description !== undefined && option.description !== '') {
      const description = document.createElement('span');
      // ⚠ Primary text at the dense size. Never `--theme-text-secondary`: an option's fill changes
      // under the keyboard cursor, and grey on that fill is 4.32 : 1. Size, not colour.
      description.className = `option-description ${inputTypeClass('optionDescription')}`;
      description.textContent = option.description;
      row.append(description);
    }

    // The owner's last look at the row. See `ListSurfaceHost.decorateOption` for the one rule it
    // has to keep, which is that the row is still exactly one row tall afterwards.
    this.#host.decorateOption?.(row, option, index);

    return row;
  }

  // ── opening, placing and closing ───────────────────────────────────────────

  /** Show the list beside a field, and place it. */
  show(anchor: HTMLElement, direction: Direction): void {
    if (this.#open) {
      this.place(anchor, direction);
      return;
    }
    this.#open = true;
    this.#list.dataset['open'] = 'true';
    this.#syncTopLayer();
    this.#render();
    this.place(anchor, direction);
    // A second pass a frame later, for the reason `<mjx-menu>` gives: the first placement measures
    // a box the browser has only just been asked to lay out, and a row whose glyph settled a frame
    // late changes the size the placement was computed for.
    requestAnimationFrame(() => {
      if (this.#open) this.place(anchor, direction);
    });
    if (typeof ResizeObserver !== 'undefined') {
      this.#observer = new ResizeObserver(() => {
        if (this.#open) this.place(anchor, direction);
      });
      this.#observer.observe(clippingAncestor(this.#list) ?? this.#list.ownerDocument.documentElement);
    }
  }

  /** Re-run the placement against the field. */
  place(anchor: HTMLElement, direction: Direction): void {
    const list = this.#list;
    clearPlacement(list);
    list.style.removeProperty(inputBoxProperties.listInlineSize);
    list.style.removeProperty(inputBoxProperties.listMaxBlockSize);

    const anchorRect = rectOf(anchor);
    this.#anchor = anchorRect;

    // The list is never narrower than the field it belongs to. A dropdown whose list was the width
    // of its longest label would jump width every time the filter changed, which reads as the list
    // moving rather than as the list filtering.
    const natural = list.getBoundingClientRect();
    const width = Math.max(anchorRect.width, natural.width);
    list.style.setProperty(inputBoxProperties.listInlineSize, `${String(Math.round(width))}px`);

    // The row cap: eight rows and their padding, so a long list is a scrollable eight rather than a
    // column down the whole screen.
    const cap = this.#rowHeight * listboxVisibleRows;
    list.style.setProperty(inputBoxProperties.listMaxBlockSize, `${String(Math.round(cap))}px`);

    const measured = list.getBoundingClientRect();
    this.#natural = { width, height: measured.height };

    const inset = resolveLength(list, floatingProperties.boundaryInset);
    const gap = resolveLength(list, floatingProperties.gap);
    const placement = placeFloating({
      anchor: anchorRect,
      floating: { width, height: measured.height },
      boundary: clippingBoundary(list, inset),
      side: listSide,
      align: listAlign,
      direction,
      gap,
    });
    applyPlacement(list, placement);
    // `applyPlacement` writes the caps from the placement; the row cap is this function's own and
    // must survive it, so it is written last. `.list`'s `max-block-size` is the `min()` of both.
    list.style.setProperty(inputBoxProperties.listMaxBlockSize, `${String(Math.round(cap))}px`);
    this.#placement = placement;
    this.#render();
  }

  /** Hide the list. */
  hide(): void {
    if (!this.#open) return;
    this.#open = false;
    this.#list.dataset['open'] = 'false';
    this.#syncTopLayer();
    clearPlacement(this.#list);
    this.#placement = undefined;
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  /** Release everything the surface is holding. Called from the owner's `disconnectedCallback`. */
  dispose(): void {
    this.hide();
  }

  /**
   * Put the list in the **top layer** while it is open.
   *
   * A field inside a scrolling task pane clips its own popup exactly as a menu inside a scrolling
   * menu does — MJXOFF-184 measured the failure: a correct rectangle, no pixels and no pointer.
   * `manual` rather than `auto`, because dismissal and focus are the owner component's own and an
   * auto popover would close on a light dismiss the owner had not decided about yet.
   */
  #syncTopLayer(): void {
    // ⚠ The dance moved into `overlay/floating.ts` in MJXOFF-188, which was its fifth consumer.
    // The conditions are unchanged: always `manual`, always in the top layer while it exists.
    syncTopLayer(this.#list, {
      inTopLayer: true,
      open: this.#open,
      connected: this.#list.isConnected,
    });
  }
}
