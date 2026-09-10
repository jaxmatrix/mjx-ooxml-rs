/**
 * `<mjx-virtual-list>` — the windowed list the other three navigators are built on.
 *
 * ```html
 * <mjx-virtual-list label="Comments"></mjx-virtual-list>
 * <script>
 *   document.querySelector('mjx-virtual-list').items = [
 *     { id: 'c1', label: 'Check this figure', detail: 'Ada' },
 *     …four thousand nine hundred and ninety-nine more…
 *   ];
 * </script>
 * ```
 *
 * ## Items are data, and a five-thousand-row list is why
 *
 * U06's `<mjx-gallery-item>` is a *descriptor element*, which is the right answer for forty styles
 * whose art is arbitrary markup. It is the wrong answer here and the reason is arithmetic: a
 * descriptor per item means five thousand light-DOM elements before a single row is drawn, which is
 * the cost virtualisation exists to avoid, paid in the one place a virtualiser cannot help. So the
 * items are a **property** — U09's decision about the mini toolbar's commands, for a different but
 * equally forced reason.
 *
 * The consequence is worth stating because it is a real trade: a list cannot be written declaratively
 * in HTML, and a shell must set `.items`. That is what `Navigators/Virtual List` shows.
 *
 * ## What is deliberately not here
 *
 * **`<mjx-tree>` and `<mjx-thumbnail-rail>` do not use this element at all.** They use the same
 * [`VirtualScroller`] and the same foundations, with their own ARIA. That is on purpose: a tree
 * nested inside a listbox would announce `option` where `treeitem` belongs, and *using the wrong
 * ARIA pattern is worse than using none*. The primitive that is shared is the **scroller**, and the
 * element is one of four things built on it rather than the parent of the other three.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  navigatorAriaPatterns,
  navigatorEvents,
  navigatorTags,
  navigatorTypeRoles,
  nextTreeIndex,
  treeKeyAction,
  typeAheadIndex,
  type KeyModifiers,
} from './navigator-model.ts';
import { virtualListCss } from './navigator-sheets.ts';
import { VirtualScroller, type ScrollerParts } from './virtual-scroller.ts';

/** One row of a plain list. */
export interface VirtualItem {
  readonly id: string;
  readonly label: string;
  /** A quieter second column — an author, a count, a date. */
  readonly detail?: string;
  /** Unavailable but explained. Still reachable by arrow key, never chosen. */
  readonly unavailable?: string;
}

/** The sheet, composed once. */
export const virtualListSheet = virtualListCss;

/** The ARIA pattern this element implements, so a gate reads it from the table and not the markup. */
export const virtualListPattern = navigatorAriaPatterns.virtualList;

export class MjxVirtualList extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'value'];

  #root: ShadowRoot | undefined;
  #viewport: HTMLElement | undefined;
  #scroller: VirtualScroller | undefined;
  #items: readonly VirtualItem[] = [];
  #cursor = 0;
  #typeAhead = '';
  #typeAheadAt = 0;
  #instance = `mjx-vl-${String(Math.random()).slice(2, 9)}`;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.#sync();
  }

  disconnectedCallback(): void {
    this.#scroller?.dispose();
  }

  attributeChangedCallback(): void {
    if (this.#root === undefined) return;
    this.#sync();
  }

  /** What the list is called. Announced as the listbox's own name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The rows. **A property, not markup** — see the note at the top of this file. */
  get items(): readonly VirtualItem[] {
    return this.#items;
  }

  set items(next: readonly VirtualItem[]) {
    this.#items = [...next];
    this.#cursor = Math.min(this.#cursor, Math.max(0, this.#items.length - 1));
    this.#sync();
  }

  /** The chosen row's id, or the empty string. */
  get value(): string {
    return this.getAttribute('value') ?? '';
  }

  set value(next: string) {
    this.setAttribute('value', next);
  }

  /** Where the keyboard is. */
  get cursorIndex(): number {
    return this.#cursor;
  }

  /** How many rows are actually in the DOM — the number the virtualisation gate compares. */
  get builtRowCount(): number {
    return this.#scroller?.builtRowCount ?? 0;
  }

  /** How many the window says there should be, computed by the foundations rather than here. */
  get expectedRowCount(): number {
    return this.#scroller?.expectedRowCount ?? 0;
  }

  /** The window currently built. */
  get builtWindow(): { readonly firstRow: number; readonly lastRow: number } {
    return this.#scroller?.builtWindow ?? { firstRow: 0, lastRow: 0 };
  }

  /** How many rows have been measured rather than guessed. The anti-vacuity figure. */
  get measuredRowCount(): number {
    return this.#scroller?.measuredRowCount ?? 0;
  }

  /** Where the list is scrolled to. */
  get offset(): number {
    return this.#scroller?.offset ?? 0;
  }

  /** What the offset would have been had the last change simply kept it. The positive control. */
  get naiveOffset(): number {
    return this.#scroller?.naiveOffset ?? 0;
  }

  /** Scroll so that a row is on screen, doing the least that achieves it. */
  scrollToIndex(index: number): void {
    this.#scroller?.scrollToRow(index);
  }

  /** Put the keyboard on a row and make sure that row is built and on screen. */
  setCursor(index: number): void {
    if (this.#items.length === 0) return;
    this.#cursor = Math.min(Math.max(index, 0), this.#items.length - 1);
    this.#scroller?.scrollToRow(this.#cursor);
    this.#render();
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, virtualListSheet);

    const viewport = document.createElement('div');
    viewport.className = 'viewport';
    viewport.setAttribute('role', virtualListPattern.container);
    viewport.tabIndex = 0;
    viewport.setAttribute('part', 'viewport');

    const leading = document.createElement('div');
    leading.className = 'spacer';
    leading.setAttribute('role', 'presentation');

    const trailing = document.createElement('div');
    trailing.className = 'spacer';
    trailing.setAttribute('role', 'presentation');

    viewport.append(leading, trailing);

    root.append(viewport);
    this.#viewport = viewport;

    const parts: ScrollerParts = { viewport, leading, trailing };
    this.#scroller = new VirtualScroller(parts, {
      keys: () => this.#items.map((item) => item.id),
      buildRow: (index) => this.#buildRow(index),
      cursorRow: () => this.#cursor,
    });

    viewport.addEventListener('keydown', this.#onKeyDown);
    viewport.addEventListener('pointerdown', this.#onPointerDown);
  }

  #sync(): void {
    const viewport = this.#viewport;
    if (viewport === undefined) return;
    viewport.setAttribute('aria-label', this.label);
    this.#scroller?.setKeys(this.#items.map((item) => item.id));
    this.#render();
  }

  #optionId(index: number): string {
    return `${this.#instance}-option-${String(index)}`;
  }

  #buildRow(index: number): HTMLElement | undefined {
    const item = this.#items[index];
    if (item === undefined) return undefined;
    const row = document.createElement('div');
    row.className = `row ${navigatorTypeRoles.row}`;
    row.id = this.#optionId(index);
    row.setAttribute('role', virtualListPattern.item);
    row.setAttribute('part', 'row');
    /*
     * ⚠ Both of these go on the OPTION and never on the listbox. They are position-within-a-set
     * attributes, which a container cannot be — axe's aria-allowed-attr rule caught exactly that in
     * U07 — and they are a virtualised list's only honest source for the count, because the number
     * of rows in the DOM is not the number of items.
     */
    row.setAttribute('aria-posinset', String(index + 1));
    row.setAttribute('aria-setsize', String(this.#items.length));
    row.setAttribute('aria-selected', String(item.id === this.value));
    row.dataset['selected'] = String(item.id === this.value);
    row.dataset['index'] = String(index);
    if (index === this.#cursor) row.dataset['cursor'] = 'true';
    if (item.unavailable !== undefined) {
      row.dataset['unavailable'] = '';
      row.setAttribute('aria-disabled', 'true');
      row.title = item.unavailable;
    }

    const label = document.createElement('span');
    label.className = 'label';
    label.textContent = item.label;
    row.append(label);

    if (item.detail !== undefined && item.detail !== '') {
      const detail = document.createElement('span');
      detail.className = `detail ${navigatorTypeRoles.caption}`;
      detail.textContent = item.detail;
      row.append(detail);
    }
    return row;
  }

  #render(): void {
    const viewport = this.#viewport;
    if (viewport === undefined) return;
    this.#scroller?.render();
    viewport.setAttribute('aria-activedescendant', this.#optionId(this.#cursor));
  }

  #choose(index: number): void {
    const item = this.#items[index];
    if (item === undefined || item.unavailable !== undefined) return;
    this.value = item.id;
    this.#cursor = index;
    this.#render();
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.selectionChanged, {
        bubbles: true,
        composed: true,
        detail: { ids: [item.id] },
      }),
    );
  }

  #onPointerDown = (event: PointerEvent): void => {
    const index = rowIndexIn(event.composedPath());
    if (index === undefined) return;
    this.#viewport?.focus();
    this.#choose(index);
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    const modifiers: KeyModifiers = {
      altKey: event.altKey,
      ctrlKey: event.ctrlKey,
      shiftKey: event.shiftKey,
      metaKey: event.metaKey,
    };
    const action = treeKeyAction(event.key, modifiers);
    if (action === 'next' || action === 'previous' || action === 'first' || action === 'last') {
      event.preventDefault();
      this.setCursor(nextTreeIndex(action, this.#cursor, this.#items.length));
      return;
    }
    if (action === 'activate') {
      event.preventDefault();
      this.#choose(this.#cursor);
      return;
    }
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      this.#onType(event.key);
    }
  };

  /** Type-ahead, over the labels, shared with the tree and the rail. */
  #onType(character: string): void {
    const now = Date.now();
    this.#typeAhead = now - this.#typeAheadAt > 1000 ? character : this.#typeAhead + character;
    this.#typeAheadAt = now;
    const found = typeAheadIndex(
      this.#items.map((item) => item.label),
      this.#typeAhead,
      this.#cursor - 1,
    );
    if (found !== undefined) this.setCursor(found);
  }

  /*
   * ⚠ **No live region here, on purpose.** A listbox's selection is announced by the platform from
   * aria-selected, so a region of its own would say everything twice. The tree and the rail have
   * one because they announce something the accessibility tree cannot carry — that a reorder
   * happened, how much moved with it, and that one was refused.
   */
}

/** The row index a pointer landed on, from the event's composed path. */
export function rowIndexIn(path: readonly EventTarget[]): number | undefined {
  for (const node of path) {
    if (!(node instanceof HTMLElement)) continue;
    const index = node.dataset['index'];
    if (index !== undefined) return Number(index);
  }
  return undefined;
}

/** Register the element. Idempotent. */
export function defineVirtualList(): void {
  if (customElements.get(navigatorTags.virtualList) === undefined) {
    customElements.define(navigatorTags.virtualList, MjxVirtualList);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-virtual-list': MjxVirtualList;
  }
}
