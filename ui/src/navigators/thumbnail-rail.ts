/**
 * `<mjx-thumbnail-rail>` — PowerPoint's slide sorter.
 *
 * ```html
 * <mjx-thumbnail-rail label="Slides"></mjx-thumbnail-rail>
 * <script>
 *   document.querySelector('mjx-thumbnail-rail').slides = [
 *     { id: 's1', label: 'Title', section: 'Opening', thumbnail: '/plates/1.png' },
 *     { id: 's2', label: 'Agenda', section: 'Opening', hidden: true },
 *     …four thousand nine hundred and ninety-eight more…
 *   ];
 * </script>
 * ```
 *
 * ## `role="listbox"` with `aria-multiselectable`, and deliberately not a tree
 *
 * Sections group slides, and a group is not a parent: a slide has no children, nothing expands, and
 * offering `ArrowRight` as *expand* over a control with nothing to expand is exactly the *"wrong
 * pattern is worse than none"* the ticket warns about. So a section is a heading **row**, marked
 * `role="presentation"`, and each slide's own accessible name carries its section —
 * `railSlideName`, which puts the position, the label, the section and *hidden* into one string.
 *
 * ⚠ **A `role="group"` per section was the first design and it is wrong under virtualisation.** A
 * group announces *how many* it owns, and a window holds some of a section's slides and not others,
 * so a group element would either claim ownership of rows that are not in the DOM or report a count
 * that changes as the reader scrolls. Folding the section into each name is what a screen-reader
 * user actually needs and it is true at every scroll position. Marked `GUESS:` in the story, because
 * PowerPoint has no accessible-name convention to be parity with.
 *
 * ## The placeholder is a state
 *
 * R10's plate generator produces thumbnails asynchronously, so a rail that blocked on rendering
 * would be unusable. A slide with no `thumbnail` yet draws a placeholder, carries `aria-busy`, and
 * **stays selectable, scrollable and reorderable throughout** — which is the part a story has to
 * exercise deliberately, because a fixture whose images are already in the cache never reaches it.
 *
 * ## Reorder, and why a block
 *
 * `Alt + ArrowUp` / `Alt + ArrowDown` move the **whole selection** as a block, and dragging calls
 * the same function. A per-slide swap would quietly turn a non-contiguous selection into a
 * contiguous one on the first press — the selection changes shape and nobody who meant *move my
 * slides down* would describe that as what they asked for.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  applyRailSelection,
  dragThreshold,
  emptyRailSelection,
  hiddenMark,
  moveSelectionBy,
  navigatorAriaPatterns,
  navigatorEvents,
  navigatorPresentationProperty,
  navigatorTags,
  navigatorTypeRoles,
  nextTreeIndex,
  railRowOfSlide,
  railRowPlan,
  railSlideName,
  railThumbnailState,
  reorderRefusals,
  treeKeyAction,
  typeAheadIndex,
  type NavigatorPresentation,
  type KeyModifiers,
  type RailSelection,
  type RailSlide,
} from './navigator-model.ts';
import type { GalleryRow } from '../gallery/gallery-model.ts';
import { thumbnailRailCss } from './navigator-sheets.ts';
import { VirtualScroller, type ScrollerParts } from './virtual-scroller.ts';
import { rowIndexIn } from './virtual-list.ts';

/** The sheet, composed once. */
export const thumbnailRailSheet = thumbnailRailCss;

/** The ARIA pattern this element implements. */
export const thumbnailRailPattern = navigatorAriaPatterns.thumbnailRail;

export class MjxThumbnailRail extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'sheet'];

  #root: ShadowRoot | undefined;
  #viewport: HTMLElement | undefined;
  #live: HTMLElement | undefined;
  #scroller: VirtualScroller | undefined;
  #slides: readonly RailSlide[] = [];
  #plan: GalleryRow[] = [];
  #selection: RailSelection = emptyRailSelection;
  #typeAhead = '';
  #typeAheadAt = 0;
  #instance = `mjx-rail-${String(Math.random()).slice(2, 9)}`;
  #dragFrom: { index: number; x: number; y: number } | undefined;
  #dragging = false;
  #dropAt: number | undefined;

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

  /** What the rail is called. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The deck. A property, for the reason `<mjx-virtual-list>` states. */
  get slides(): readonly RailSlide[] {
    return this.#slides;
  }

  set slides(next: readonly RailSlide[]) {
    this.#slides = [...next];
    this.#sync();
  }

  /** The chosen slides, in the deck's own order. */
  get selected(): readonly string[] {
    return this.#selection.selected;
  }

  /** Where the keyboard is, as a slide index. */
  get cursorIndex(): number {
    return this.#selection.cursor;
  }

  /** The plan, in U06's row shape, so a gate can compare against `railRowPlan()`. */
  get rowPlan(): readonly GalleryRow[] {
    return this.#plan;
  }

  /** How many rows are actually in the DOM. */
  get builtRowCount(): number {
    return this.#scroller?.builtRowCount ?? 0;
  }

  /** How many the window says there should be. */
  get expectedRowCount(): number {
    return this.#scroller?.expectedRowCount ?? 0;
  }

  /** The window currently built. */
  get builtWindow(): { readonly firstRow: number; readonly lastRow: number } {
    return this.#scroller?.builtWindow ?? { firstRow: 0, lastRow: 0 };
  }

  /** How many rows have been measured rather than guessed. */
  get measuredRowCount(): number {
    return this.#scroller?.measuredRowCount ?? 0;
  }

  /** Where the rail is scrolled to. */
  get offset(): number {
    return this.#scroller?.offset ?? 0;
  }

  /** What the offset would have been had the last change kept it. The positive control. */
  get naiveOffset(): number {
    return this.#scroller?.naiveOffset ?? 0;
  }

  /** How many slides are still waiting for a plate. */
  get pendingCount(): number {
    return this.#slides.filter((slide) => railThumbnailState(slide) === 'pending').length;
  }

  /** The last thing announced. */
  get announcement(): string {
    return this.#live?.textContent ?? '';
  }

  /**
   * Which presentation CSS has put it in — **read back, never decided here.**
   *
   * The catalogue's mechanism, and the reason it is read rather than computed: a component that
   * decided its own presentation and then reported it would be grading its own homework, and the
   * gate compares this against `navigatorPresentationAt()` instead.
   */
  get presentation(): NavigatorPresentation {
    const declared = getComputedStyle(this).getPropertyValue(navigatorPresentationProperty).trim();
    return declared === 'sheet' ? 'sheet' : 'pane';
  }

  /** Scroll so that a slide is on screen. */
  scrollToIndex(index: number): void {
    this.#scroller?.scrollToRow(railRowOfSlide(this.#plan, index));
  }

  /** Put the keyboard on a slide and choose it, replacing whatever was chosen. */
  setCursor(index: number, intent = { toggle: false, extend: false }): void {
    if (this.#slides.length === 0) return;
    this.#selection = applyRailSelection(this.#slides, this.#selection, index, intent);
    this.scrollToIndex(this.#selection.cursor);
    this.#render();
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.selectionChanged, {
        bubbles: true,
        composed: true,
        detail: { ids: this.#selection.selected },
      }),
    );
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, thumbnailRailSheet);

    const viewport = document.createElement('div');
    viewport.className = 'viewport';
    viewport.setAttribute('role', thumbnailRailPattern.container);
    viewport.setAttribute('aria-multiselectable', 'true');
    viewport.tabIndex = 0;
    viewport.setAttribute('part', 'viewport');

    const leading = document.createElement('div');
    leading.className = 'spacer';
    leading.setAttribute('role', 'presentation');
    const trailing = document.createElement('div');
    trailing.className = 'spacer';
    trailing.setAttribute('role', 'presentation');
    viewport.append(leading, trailing);

    const live = document.createElement('div');
    live.className = 'live';
    live.setAttribute('role', 'status');
    live.setAttribute('aria-live', 'polite');

    root.append(viewport, live);
    this.#viewport = viewport;
    this.#live = live;

    const parts: ScrollerParts = { viewport, leading, trailing };
    this.#scroller = new VirtualScroller(parts, {
      keys: () => this.#planKeys(),
      buildRow: (index) => this.#buildRow(index),
      cursorRow: () => railRowOfSlide(this.#plan, this.#selection.cursor),
    });

    viewport.addEventListener('keydown', this.#onKeyDown);
    viewport.addEventListener('pointerdown', this.#onPointerDown);
    viewport.addEventListener('pointermove', this.#onPointerMove);
    viewport.addEventListener('pointerup', this.#onPointerUp);
    viewport.addEventListener('pointercancel', this.#onPointerUp);
  }

  /**
   * One key per **plan** row, because the extent table is over plan rows and a heading is a row.
   *
   * A heading's key names its section, so a section that is still there after an insertion keeps
   * its measured height and the anchor over it still resolves.
   */
  #planKeys(): string[] {
    return this.#plan.map((row) =>
      row.kind === 'heading' ? `section:${row.category}` : (this.#slides[row.start]?.id ?? ''),
    );
  }

  #sync(): void {
    const viewport = this.#viewport;
    if (viewport === undefined) return;
    viewport.setAttribute('aria-label', this.label);
    this.#plan = railRowPlan(this.#slides);
    this.#scroller?.setKeys(this.#planKeys());
    this.#render();
  }

  #optionId(slideIndex: number): string {
    return `${this.#instance}-slide-${String(slideIndex)}`;
  }

  #buildRow(planRow: number): HTMLElement | undefined {
    const entry = this.#plan[planRow];
    if (entry === undefined) return undefined;
    if (entry.kind === 'heading') {
      const heading = document.createElement('div');
      heading.className = `section ${navigatorTypeRoles.section}`;
      heading.setAttribute('role', 'presentation');
      heading.dataset['section'] = entry.category;
      heading.textContent = entry.category;
      return heading;
    }

    const index = entry.start;
    const slide = this.#slides[index];
    if (slide === undefined) return undefined;

    const element = document.createElement('div');
    element.className = `slide ${navigatorTypeRoles.row}`;
    element.id = this.#optionId(index);
    element.setAttribute('role', thumbnailRailPattern.item);
    element.setAttribute('part', 'slide');
    element.setAttribute('aria-posinset', String(index + 1));
    element.setAttribute('aria-setsize', String(this.#slides.length));
    element.setAttribute('aria-label', railSlideName(slide, index + 1, this.#slides.length));
    const chosen = this.#selection.selected.includes(slide.id);
    element.setAttribute('aria-selected', String(chosen));
    element.dataset['selected'] = String(chosen);
    element.dataset['index'] = String(index);
    element.dataset['id'] = slide.id;
    if (index === this.#selection.cursor) element.dataset['cursor'] = 'true';
    if (slide.hidden === true) element.dataset['hidden'] = 'true';
    if (this.#dragging && chosen) element.dataset['dragging'] = 'true';
    if (this.#dropAt === index) element.dataset['drop'] = 'before';

    const number = document.createElement('span');
    number.className = `number ${navigatorTypeRoles.caption}`;
    number.setAttribute('aria-hidden', 'true');
    number.textContent = String(index + 1);
    element.append(number);

    const frame = document.createElement('div');
    frame.className = 'plate';
    const state = railThumbnailState(slide);
    frame.dataset['state'] = state;
    if (state === 'pending') {
      // ⚠ aria-busy on the frame and not on the option: the option's name is complete and correct
      // the moment the rail is built, and marking the whole option busy would suppress it.
      frame.setAttribute('aria-busy', 'true');
      const placeholder = document.createElement('div');
      placeholder.className = 'placeholder';
      placeholder.setAttribute('aria-hidden', 'true');
      frame.append(placeholder);
    } else {
      const image = document.createElement('img');
      image.src = slide.thumbnail ?? '';
      image.alt = '';
      image.loading = 'lazy';
      image.decoding = 'async';
      frame.append(image);
    }
    element.append(frame);

    const caption = document.createElement('div');
    caption.className = 'caption';
    const title = document.createElement('span');
    title.textContent = slide.label;
    caption.append(title);
    if (slide.hidden === true) {
      const marks = document.createElement('span');
      marks.className = `marks ${navigatorTypeRoles.caption}`;
      marks.setAttribute('aria-hidden', 'true');
      marks.textContent = `${hiddenMark} Hidden`;
      caption.append(marks);
    }
    element.append(caption);
    return element;
  }

  #render(): void {
    const viewport = this.#viewport;
    if (viewport === undefined) return;
    this.#scroller?.render();
    viewport.setAttribute('aria-activedescendant', this.#optionId(this.#selection.cursor));
  }

  #announce(message: string): void {
    const live = this.#live;
    if (live === undefined) return;
    live.textContent = message;
  }

  /** Move the selection as a block, and say what happened. */
  #reorder(delta: number): void {
    const chosen = this.#selection.selected;
    if (chosen.length === 0) return;
    const positions = this.#slides
      .map((slide, index) => (chosen.includes(slide.id) ? index : -1))
      .filter((index) => index >= 0);
    const first = positions[0] ?? 0;
    const last = positions[positions.length - 1] ?? 0;
    if (delta < 0 && first === 0) {
      this.#announce(reorderRefusals.atStart);
      return;
    }
    if (delta > 0 && last === this.#slides.length - 1) {
      this.#announce(reorderRefusals.atEnd);
      return;
    }
    this.#slides = moveSelectionBy(this.#slides, chosen, delta);
    this.#plan = railRowPlan(this.#slides);
    const cursorId = chosen[0] ?? '';
    const landed = this.#slides.findIndex((slide) => slide.id === cursorId);
    this.#selection = {
      selected: chosen,
      anchor: landed >= 0 ? landed : this.#selection.anchor,
      cursor: landed >= 0 ? landed : this.#selection.cursor,
    };
    this.#scroller?.setKeys(this.#planKeys());
    this.scrollToIndex(this.#selection.cursor);
    this.#render();
    this.#announce(
      chosen.length === 1
        ? `Slide moved to position ${String(this.#selection.cursor + 1)}.`
        : `${String(chosen.length)} slides moved to position ${String(this.#selection.cursor + 1)}.`,
    );
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.reordered, {
        bubbles: true,
        composed: true,
        detail: { order: this.#slides.map((slide) => slide.id), ids: chosen },
      }),
    );
  }

  #onKeyDown = (event: KeyboardEvent): void => {
    const modifiers: KeyModifiers = {
      altKey: event.altKey,
      ctrlKey: event.ctrlKey,
      shiftKey: event.shiftKey,
      metaKey: event.metaKey,
    };
    if (event.altKey && (event.key === 'ArrowUp' || event.key === 'ArrowDown')) {
      event.preventDefault();
      this.#reorder(event.key === 'ArrowUp' ? -1 : 1);
      return;
    }
    if (event.key === ' ' && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      this.setCursor(this.#selection.cursor, { toggle: true, extend: false });
      return;
    }
    const movement = treeKeyAction(event.key, { ...modifiers, shiftKey: false });
    if (
      movement === 'next' ||
      movement === 'previous' ||
      movement === 'first' ||
      movement === 'last'
    ) {
      event.preventDefault();
      const next = nextTreeIndex(movement, this.#selection.cursor, this.#slides.length);
      this.setCursor(next, { toggle: false, extend: event.shiftKey });
      return;
    }
    if (event.key === 'Enter') {
      event.preventDefault();
      const slide = this.#slides[this.#selection.cursor];
      if (slide === undefined) return;
      this.dispatchEvent(
        new CustomEvent(navigatorEvents.activated, {
          bubbles: true,
          composed: true,
          detail: { id: slide.id },
        }),
      );
      return;
    }
    if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
      this.#onType(event.key);
    }
  };

  #onType(character: string): void {
    const now = Date.now();
    this.#typeAhead = now - this.#typeAheadAt > 1000 ? character : this.#typeAhead + character;
    this.#typeAheadAt = now;
    const found = typeAheadIndex(
      this.#slides.map((slide) => slide.label),
      this.#typeAhead,
      this.#selection.cursor - 1,
    );
    if (found !== undefined) this.setCursor(found);
  }

  #onPointerDown = (event: PointerEvent): void => {
    const index = rowIndexIn(event.composedPath());
    if (index === undefined) return;
    this.#viewport?.focus();
    this.setCursor(index, {
      toggle: event.ctrlKey || event.metaKey,
      extend: event.shiftKey,
    });
    this.#dragFrom = { index, x: event.clientX, y: event.clientY };
    this.#viewport?.setPointerCapture(event.pointerId);
  };

  #onPointerMove = (event: PointerEvent): void => {
    const from = this.#dragFrom;
    if (from === undefined) return;
    if (!this.#dragging && Math.hypot(event.clientX - from.x, event.clientY - from.y) < dragThreshold) {
      return;
    }
    this.#dragging = true;
    this.#dropAt = this.#slideUnder(event.clientY);
    this.#render();
  };

  #onPointerUp = (event: PointerEvent): void => {
    const from = this.#dragFrom;
    this.#dragFrom = undefined;
    if (this.#viewport?.hasPointerCapture(event.pointerId) === true) {
      this.#viewport.releasePointerCapture(event.pointerId);
    }
    const target = this.#dropAt;
    const wasDragging = this.#dragging;
    this.#dragging = false;
    this.#dropAt = undefined;
    if (!wasDragging || from === undefined || target === undefined) {
      this.#render();
      return;
    }
    // The same block move the keyboard uses, one step at a time. One function, two ways in.
    const steps = target - from.index;
    for (let step = 0; step < Math.abs(steps); step += 1) this.#reorder(Math.sign(steps));
    this.#render();
  };

  #slideUnder(clientY: number): number | undefined {
    const window_ = this.#scroller?.builtWindow;
    if (window_ === undefined) return undefined;
    for (let row = window_.firstRow; row < window_.lastRow; row += 1) {
      const element = this.#scroller?.elementForRow(row);
      if (element === undefined) continue;
      const index = element.dataset['index'];
      if (index === undefined) continue;
      const box = element.getBoundingClientRect();
      if (clientY >= box.top && clientY <= box.bottom) return Number(index);
    }
    return undefined;
  }
}

/** Register the element. Idempotent. */
export function defineThumbnailRail(): void {
  if (customElements.get(navigatorTags.thumbnailRail) === undefined) {
    customElements.define(navigatorTags.thumbnailRail, MjxThumbnailRail);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-thumbnail-rail': MjxThumbnailRail;
  }
}
