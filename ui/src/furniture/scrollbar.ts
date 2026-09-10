/**
 * `<mjx-scrollbar>` — a scrollbar over a viewport it does not own, and the thumb that does not jump.
 *
 * ```html
 * <mjx-scrollbar label="Document" controls="page-canvas"
 *                pages="20" page-height="1000" viewport="4000">
 *   <mjx-scroll-mark kind="search" page="3" within="0.2" label="pipeline"></mjx-scroll-mark>
 *   <mjx-scroll-mark kind="comment" page="11" label="Ask legal"></mjx-scroll-mark>
 * </mjx-scrollbar>
 * ```
 *
 * ## The defining requirement is invisible in any story with a fixed content height
 *
 * MJXOFF-190 states it as the trap: *"a thumb over known content behaves perfectly under any
 * implementation; the failure only appears when the extent changes while the user is dragging."*
 * That is R13's requirement seen from the front — `crates/mjx-view/src/scroll.rs` gives a scrollbar
 * the instant a document opens by *estimating* how long it is, and then corrects page heights one
 * at a time as they are laid out.
 *
 * There are therefore **two** invariants here, true at different moments and held by different
 * mechanisms:
 *
 * * **not dragging**, the *anchor* is the state — a page and how far into it — and the offset is
 *   derived from the model, so a correction to a page above the reader does not move the page under
 *   them;
 * * **dragging**, the *pointer* is the state and the offset is derived from it, so the point of the
 *   thumb under the finger stays under the finger even while the thumb's own size is changing.
 *
 * `#applyPointer` is the second one and it is three lines. The naive alternative — keep the offset,
 * recompute the thumb from it — is what both gates compute *beside* the real answer, so that a
 * green assertion is known to be an assertion about something rather than a tolerance nobody tested.
 *
 * ## Not the platform's scrollbar, and that is a requirement rather than a preference
 *
 * The ticket: *"it must not depend on native overlay-scrollbar behaviour, which differs across
 * platforms and cannot carry scroll marks."* The marks are the reason — a channel of search hits,
 * comment threads and tracked changes is what makes a three-hundred-page document navigable, and
 * no platform scrollbar has anywhere to put one.
 *
 * ## `role="scrollbar"` on the HOST, and the IDREF that put it there
 *
 * `aria-valuenow`, `aria-valuemin` and `aria-valuemax` as a percentage of the scrollable extent,
 * `aria-valuetext` as the page a reader is on, and `aria-controls` naming the region that actually
 * scrolls. A scrollbar announced as a bare number tells nobody where they are in a document.
 *
 * ⚠ **The role is on the host element rather than on a box inside the shadow root, and
 * `aria-controls` is the whole reason.** MJXOFF-183's finding in a new shape: an IDREF does not
 * cross a shadow boundary in either direction. A scrollbar's `aria-controls` must name *the region
 * that scrolls*, and that region belongs to the shell rather than to this component — so an
 * `aria-controls` written onto a shadow child would resolve to nothing, in exactly the way that
 * looks correct in a DOM inspector. `<mjx-task-pane>`'s splitter can keep its role inside its
 * shadow root precisely because the thing it controls is in there with it; this one cannot.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  furnitureBoxProperties,
  furnitureEvents,
  furnitureTags,
  scrollMarkSpecs,
  scrollbarCss,
} from './furniture-model.ts';
import {
  ScrollbarModel,
  isScrollMarkKind,
  markFraction,
  marksSummary,
  offsetUnderGrip,
  scrollOrigin,
  scrollable,
  scrollableExtent,
  thumbFraction,
  thumbStart,
  type ExtentPrecision,
  type ScrollAnchor,
  type ScrollMark,
  type ScrollMarkKind,
} from './scroll-model.ts';

/** The sheet, composed once. */
export const scrollbarSheet = scrollbarCss;

/** What a mark descriptor fires when one of its attributes changes. */
export const marksChangedEvent = 'mjx-scroll-marks-changed';

/**
 * `<mjx-scroll-mark>` — data written as markup, never content.
 *
 * Read for its attributes and never cloned, on the rule `<mjx-option>` established: **custom
 * elements upgrade in tree order**, so a scrollbar's `connectedCallback` can run before its marks
 * have upgraded, and an `instanceof` that ran too early would report a channel with nothing in it —
 * which reads exactly like a document with no search hits.
 */
export class MjxScrollMark extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['kind', 'page', 'within', 'label'];

  connectedCallback(): void {
    // `role="none"` as well as `display: none`: a descriptor a stylesheet failed to reach must
    // still not be a node in the accessibility tree.
    this.setAttribute('role', 'none');
    this.#announce();
  }

  attributeChangedCallback(): void {
    this.#announce();
  }

  #announce(): void {
    this.dispatchEvent(new CustomEvent(marksChangedEvent, { bubbles: true, composed: true }));
  }
}

/** Every mark directly inside a host, in document order. Matched by `localName`, per the note. */
export function scrollMarksIn(host: Element): ScrollMark[] {
  const found: ScrollMark[] = [];
  for (const child of host.children) {
    if (child.localName !== furnitureTags.scrollMark) continue;
    const kind = child.getAttribute('kind');
    if (!isScrollMarkKind(kind)) continue;
    const page = Number.parseInt(child.getAttribute('page') ?? '', 10);
    const within = Number.parseFloat(child.getAttribute('within') ?? '');
    found.push({
      kind: kind as ScrollMarkKind,
      page: Number.isFinite(page) ? Math.max(0, page) : 0,
      within: Number.isFinite(within) ? Math.min(1, Math.max(0, within)) : 0,
      label: child.getAttribute('label') ?? '',
    });
  }
  return found;
}

/** How far one arrow press moves the viewport, as a fraction of it. */
export const scrollLineFraction = 0.1;

let nextScrollbarSerial = 0;

export class MjxScrollbar extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'controls',
    'orientation',
    'viewport',
    'pages',
    'page-height',
    'precision',
  ];

  #root: ShadowRoot | undefined;
  #track: HTMLElement | undefined;
  #thumb: HTMLElement | undefined;
  #marks: HTMLElement | undefined;
  #live: HTMLElement | undefined;
  #model: ScrollbarModel | undefined;
  #anchor: ScrollAnchor = scrollOrigin;
  #dragging: number | undefined;
  /** Where within the thumb the drag began, as a fraction of the thumb. */
  #grip = 0;
  /** Where along the track the pointer last was, as a fraction. The state while dragging. */
  #pointer = 0;
  readonly #id = `mjx-scrollbar-${String((nextScrollbarSerial += 1))}`;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(name: string): void {
    // A change to the extent's declaration is a new document, not a correction to this one.
    if (name === 'pages' || name === 'page-height' || name === 'precision') this.#model = undefined;
    if (this.#root !== undefined) this.render();
  }

  /** The scrollbar's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The id of the region that actually scrolls. */
  get controls(): string {
    return this.getAttribute('controls') ?? '';
  }

  /** `vertical` or `horizontal`. */
  get orientation(): 'vertical' | 'horizontal' {
    return this.getAttribute('orientation') === 'horizontal' ? 'horizontal' : 'vertical';
  }

  /** How much of the document the viewport can see, in the model's own units. */
  get viewport(): number {
    const declared = Number.parseFloat(this.getAttribute('viewport') ?? '');
    return Number.isFinite(declared) && declared > 0 ? declared : 0;
  }

  /**
   * The model. Built from the declared extent on first use, and **corrected in place** after that.
   *
   * Corrected rather than rebuilt is the whole design: a rebuild would throw away every measured
   * page height, which is exactly what R13 refuses to do for a one-character edit.
   */
  get model(): ScrollbarModel {
    if (this.#model === undefined) {
      const pages = Number.parseInt(this.getAttribute('pages') ?? '', 10);
      const pageHeight = Number.parseFloat(this.getAttribute('page-height') ?? '');
      const precision = this.getAttribute('precision');
      this.#model = ScrollbarModel.fromExtent({
        pages: Number.isFinite(pages) ? pages : 1,
        pageHeight: Number.isFinite(pageHeight) ? pageHeight : 0,
        precision: (precision === 'exact' ? 'exact' : 'estimated') satisfies ExtentPrecision,
      });
    }
    return this.#model;
  }

  /** Where the reader is, in terms that survive a correction. */
  get anchor(): ScrollAnchor {
    return this.#anchor;
  }

  set anchor(next: ScrollAnchor) {
    this.#anchor = next;
    this.render();
  }

  /** Where the reader is, as a document offset. Derived; never stored. */
  get offset(): number {
    return this.model.offsetOfAnchor(this.#anchor);
  }

  set offset(next: number) {
    this.#anchor = this.model.anchorAt(this.#clampOffset(next));
    this.render();
  }

  /** The thumb's size, as a fraction of the track. What a gate measures against. */
  get thumbSize(): number {
    return thumbFraction(this.viewport, this.model.totalHeight);
  }

  /** The thumb's leading edge, as a fraction of the track. */
  get thumbAt(): number {
    return thumbStart(this.offset, this.viewport, this.model.totalHeight);
  }

  /** Whether there is anything to scroll. A scrollbar over content that fits is not one. */
  get scrollable(): boolean {
    return scrollable(this.viewport, this.model.totalHeight);
  }

  /** The marks in the channel, read from the descriptors. */
  get marks(): ScrollMark[] {
    return scrollMarksIn(this);
  }

  /** The track, so a gate measures the real box. */
  get trackElement(): HTMLElement | undefined {
    return this.#track;
  }

  /** The thumb, so a gate measures the real box. */
  get thumbElement(): HTMLElement | undefined {
    return this.#thumb;
  }

  /**
   * A page turned out to be this tall.
   *
   * **The path the trap lives on.** If a drag is in flight the offset is re-derived from where the
   * pointer is, so the point under the finger does not move; otherwise the anchor is already the
   * state and there is nothing to do but re-render.
   */
  recordMeasuredHeight(page: number, height: number): void {
    this.model.recordMeasuredHeight(page, height);
    if (this.#dragging !== undefined) this.#applyPointer();
    else this.render();
  }

  /** The document turned out to have exactly this many pages. */
  recordExactPageCount(pages: number): void {
    this.model.recordExactPageCount(pages);
    if (this.#dragging !== undefined) this.#applyPointer();
    else this.render();
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, scrollbarSheet);

    // The role, the value and the keyboard all live on the host — see the module note on the IDREF.
    this.setAttribute('role', 'scrollbar');
    this.tabIndex = 0;
    this.addEventListener('keydown', this.#onKeyDown);
    this.addEventListener('pointerdown', this.#onPointerDown);
    this.addEventListener('pointermove', this.#onPointerMove);
    this.addEventListener('pointerup', this.#onPointerUp);
    this.addEventListener('pointercancel', this.#onPointerUp);

    const track = document.createElement('div');
    track.className = 'track';
    track.setAttribute('part', 'track');
    track.id = `${this.#id}-track`;

    const marks = document.createElement('div');
    marks.className = 'marks';
    marks.setAttribute('part', 'marks');
    marks.setAttribute('aria-hidden', 'true');

    const thumb = document.createElement('div');
    // ⚠ **No motion class, deliberately.** Every other moving part in this catalogue wears one; a
    // scrollbar thumb must not. `transition-property` defaults to `all`, so a settle class here
    // would ease the thumb's position and size — which means the thumb would lag the finger
    // dragging it, and, worse, would make the mid-drag stability gate measure an interpolated box.
    // A gate that can be satisfied by an animation halfway through is not a gate.
    thumb.className = 'thumb';
    thumb.setAttribute('part', 'thumb');

    const live = document.createElement('div');
    live.className = 'live';
    live.setAttribute('role', 'status');

    track.append(marks, thumb);
    root.append(track, live);
    this.#track = track;
    this.#thumb = thumb;
    this.#marks = marks;
    this.#live = live;

    this.addEventListener(marksChangedEvent, this.#onMarksChanged);
  }

  #onMarksChanged = (): void => {
    this.render();
  };

  // ── the pointer ────────────────────────────────────────────────────────────

  /** Where a pointer event is along the track, as a fraction. */
  #along(event: PointerEvent): number {
    const track = this.#track;
    if (track === undefined) return 0;
    const box = track.getBoundingClientRect();
    if (this.orientation === 'horizontal') {
      if (box.width <= 0) return 0;
      return Math.min(1, Math.max(0, (event.clientX - box.left) / box.width));
    }
    if (box.height <= 0) return 0;
    return Math.min(1, Math.max(0, (event.clientY - box.top) / box.height));
  }

  #onPointerDown = (event: PointerEvent): void => {
    const track = this.#track;
    if (track === undefined || !this.scrollable) return;
    event.preventDefault();
    this.focus();
    const along = this.#along(event);
    const size = this.thumbSize;
    const start = this.thumbAt;
    if (along >= start && along <= start + size && size > 0) {
      // On the thumb: remember where within it the finger landed. That fraction is the whole of
      // what must be held constant while the extent is being revised.
      this.#grip = (along - start) / size;
    } else {
      // Off the thumb: jump so the thumb centres under the finger, and drag from there.
      this.#grip = 0.5;
    }
    this.#pointer = along;
    this.#dragging = event.pointerId;
    this.setPointerCapture(event.pointerId);
    this.#applyPointer('pointer');
  };

  #onPointerMove = (event: PointerEvent): void => {
    if (this.#dragging !== event.pointerId) return;
    this.#pointer = this.#along(event);
    this.#applyPointer('pointer');
  };

  #onPointerUp = (event: PointerEvent): void => {
    if (this.#dragging !== event.pointerId) return;
    this.releasePointerCapture(event.pointerId);
    this.#dragging = undefined;
  };

  /**
   * **The trap, answered in three lines.**
   *
   * The offset is *solved for* from where the pointer is, rather than kept and used to recompute
   * the thumb. Keeping it is the naive answer, and it is what makes the thumb slide out from under
   * a finger the instant a page turns out to be taller than the guess.
   */
  #applyPointer(cause: 'pointer' | 'host' = 'host'): void {
    const total = this.model.totalHeight;
    const offset = offsetUnderGrip(this.#pointer, this.#grip, this.viewport, total);
    this.#anchor = this.model.anchorAt(offset);
    this.render();
    this.#report(cause);
  }

  // ── the keyboard ───────────────────────────────────────────────────────────

  #onKeyDown = (event: KeyboardEvent): void => {
    if (!this.scrollable) return;
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const viewport = this.viewport;
    const back = this.orientation === 'horizontal' ? 'ArrowLeft' : 'ArrowUp';
    const forward = this.orientation === 'horizontal' ? 'ArrowRight' : 'ArrowDown';
    const extent = scrollableExtent(viewport, this.model.totalHeight);
    let next: number | undefined;
    switch (event.key) {
      case back:
        next = this.offset - viewport * scrollLineFraction;
        break;
      case forward:
        next = this.offset + viewport * scrollLineFraction;
        break;
      case 'PageUp':
        next = this.offset - viewport;
        break;
      case 'PageDown':
        next = this.offset + viewport;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = extent;
        break;
      default:
        return;
    }
    event.preventDefault();
    const clamped = this.#clampOffset(next);
    if (clamped === this.offset) return;
    this.#anchor = this.model.anchorAt(clamped);
    this.render();
    this.#report('keyboard');
  };

  #clampOffset(offset: number): number {
    const extent = scrollableExtent(this.viewport, this.model.totalHeight);
    if (!Number.isFinite(offset)) return 0;
    return Math.min(extent, Math.max(0, offset));
  }

  #report(cause: 'pointer' | 'keyboard' | 'host'): void {
    this.dispatchEvent(
      new CustomEvent(furnitureEvents.scroll, {
        bubbles: true,
        composed: true,
        detail: { offset: this.offset, anchor: this.#anchor, cause },
      }),
    );
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render from the model. */
  render(): void {
    const track = this.#track;
    const thumb = this.#thumb;
    const marks = this.#marks;
    const live = this.#live;
    if (track === undefined || thumb === undefined || marks === undefined || live === undefined) {
      return;
    }

    const model = this.model;
    const total = model.totalHeight;
    const size = thumbFraction(this.viewport, total);
    const start = thumbStart(this.offset, this.viewport, total);
    const extent = scrollableExtent(this.viewport, total);

    // Two numbers reach CSS and nothing else does. A painter that computed either of them from a
    // different quantity than the model did would be a second interpretation of the same document.
    this.style.setProperty(furnitureBoxProperties.thumbSize, String(size));
    this.style.setProperty(furnitureBoxProperties.thumbStart, String(start));
    this.dataset['scrollable'] = String(extent > 0);

    this.setAttribute('aria-label', this.label);
    this.setAttribute('aria-orientation', this.orientation);
    this.setAttribute('aria-valuemin', '0');
    this.setAttribute('aria-valuemax', '100');
    this.setAttribute(
      'aria-valuenow',
      String(extent > 0 ? Math.round((this.offset / extent) * 100) : 0),
    );
    this.setAttribute('aria-valuetext', this.valueText);
    if (this.controls === '') this.removeAttribute('aria-controls');
    else this.setAttribute('aria-controls', this.controls);
    // A scrollbar over content that fits is not a scrollbar, and leaving it in the tab order gives
    // a keyboard user a stop that does nothing at all.
    this.tabIndex = this.scrollable ? 0 : -1;

    this.#renderMarks(marks, model);
    // ⚠ Written only when it changes. This runs on every frame of a drag, and re-setting a live
    // region's text to the string it already holds is a second announcement in some assistive
    // technology — which is the same "announcing too much" failure the status bar is arranged
    // around, arriving through a different door.
    const summary = marksSummary(this.marks);
    if (live.textContent !== summary) live.textContent = summary;
  }

  /** What a screen reader is told about where the reader is. A page, not a percentage. */
  get valueText(): string {
    const pages = this.model.pageCount;
    const page = this.#anchor.page + 1;
    const precision = this.model.countPrecision === 'exact' ? '' : ' (estimated)';
    return `Page ${String(page)} of ${String(pages)}${precision}`;
  }

  #renderMarks(host: HTMLElement, model: ScrollbarModel): void {
    const wanted = this.marks;
    host.replaceChildren();
    for (const mark of wanted) {
      const element = document.createElement('span');
      element.className = 'mark';
      element.dataset['kind'] = mark.kind;
      element.style.setProperty(
        furnitureBoxProperties.markAt,
        String(markFraction(model, mark)),
      );
      element.style.setProperty(
        furnitureBoxProperties.markLane,
        String(scrollMarkSpecs[mark.kind].lane),
      );
      host.append(element);
    }
  }
}

/** Register both elements. Idempotent. */
export function defineScrollbar(): void {
  if (customElements.get(furnitureTags.scrollMark) === undefined) {
    customElements.define(furnitureTags.scrollMark, MjxScrollMark);
  }
  if (customElements.get(furnitureTags.scrollbar) === undefined) {
    customElements.define(furnitureTags.scrollbar, MjxScrollbar);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-scrollbar': MjxScrollbar;
    'mjx-scroll-mark': MjxScrollMark;
  }
}
