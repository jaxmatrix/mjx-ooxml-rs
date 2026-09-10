/**
 * `<mjx-review-pane>` — the margin column, and **the only component in this catalogue whose layout
 * is a function of geometry the canvas owns.**
 *
 * ```html
 * <mjx-review-pane label="Comments"></mjx-review-pane>
 * <script>
 *   document.querySelector('mjx-review-pane').annotations = [
 *     { id: 'c1', kind: 'comment', model: 'threaded', author: 'Ada Lovelace',
 *       time: '10:15', text: 'Should this be the 1843 figure?', anchorTop: 120 },
 *     …two hundred more…
 *   ];
 * </script>
 * ```
 *
 * ## What it is, and what it is not
 *
 * It **is** U09's task pane: docked, persistent, not dismissible, with a title and a resizable edge.
 * What it does not inherit is the pane's *content* arrangement, because a review pane's content is
 * not a stack — see `annotation-model.ts` for the packing problem and its answer. So this element
 * sits **inside** a `<mjx-task-pane>` rather than subclassing it: a component that inherited the
 * pane and replaced its layout would have inherited a splitter, a dock and an edge indicator it
 * then had to work around, and `Annotation/Review Pane → In A Task Pane` is the story that shows
 * the composition actually working.
 *
 * ## Three things it does that a list does not
 *
 * * **It packs.** Every card gets the position nearest its anchor that leaves no overlap, and the
 *   selected card gets its preferred position exactly while the others yield. The arithmetic is in
 *   the model and is asserted against an independent reference.
 * * **It virtualises, through the catalogue's one virtualisation.** A document with hundreds of
 *   annotations builds the window and no more. The route is `reviewWindow`, which is
 *   `windowForOffset` over an `ExtentTable` built from the packed layout — see that function for
 *   why the route through `VirtualScroller` is the wrong one rather than the second one.
 * * **It emits the connector contract** — every annotation, not only the built ones, because the
 *   canvas draws a line to a card that is scrolled out of view as readily as to one on screen.
 *
 * ## The sheet
 *
 * A margin column has no room on a phone. Below `reviewSheetAtOrBelow` the pane becomes a sheet
 * listing the annotations in flow, and packing stops — not *disabled*, but meaningless, because
 * there is no margin for a card to sit beside. The switch is a container query, so it is the
 * component's own width that decides and never the window's.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import { rowsInWindow } from '../foundations/virtual-list.ts';
import {
  annotationEvents,
  annotationTags,
  marginCardGapUnits,
  packMarginCards,
  reviewAriaPattern,
  reviewPresentationProperty,
  reviewWindow,
  type AnnotationAnchorReport,
  type CommentModel,
  type ConnectorSide,
  type MarginCard,
  type PackedCard,
  type ReviewPresentation,
  type TrackedChangeKind,
} from './annotation-model.ts';
import { annotationMotionClass, annotationTypeRoles, reviewPaneCss } from './annotation-sheets.ts';
import { assignAuthorColours, authorColourFor } from './author-colour.ts';
import { MjxAnnotationCard } from './comment-card.ts';
import type { CommentReply } from './comment-thread.ts';

/** One annotation, in the shape a shell already has it. */
export interface ReviewAnnotation {
  readonly id: string;
  readonly kind: 'comment' | 'trackedChange';
  /** For a comment: which of Word's two models. Ignored for a tracked change. */
  readonly model?: CommentModel;
  /** For a tracked change: which revision. Ignored for a comment. */
  readonly changeKind?: TrackedChangeKind;
  readonly author: string;
  readonly time: string;
  readonly text: string;
  /** The words a tracked change touched. */
  readonly excerpt?: string;
  /** Where the annotation's anchor sits, in the column's own coordinates. */
  readonly anchorTop: number;
  readonly resolved?: boolean;
  /** A threaded comment's replies. */
  readonly replies?: readonly CommentReply[];
}

/**
 * The height a card is assumed to be until one has been measured.
 *
 * A guess, and it only ever costs one extra layout pass: the pane measures whatever it built and
 * re-packs when a measurement disagrees with the estimate. `VirtualScroller`'s
 * `defaultEstimatedRowExtent` is the same idea at a different scale, and for the same reason —
 * a virtualiser must be able to answer *how long is this list* before it has built any of it.
 */
export const estimatedCardExtent = 96;

/** How many cards the keyboard's page keys move by, when the viewport does not say. */
const fallbackPageStep = 1;

export class MjxReviewPane extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'selected', 'side'];

  #root: ShadowRoot | undefined;
  #column: HTMLElement | undefined;
  #sizer: HTMLElement | undefined;
  #metric: HTMLElement | undefined;
  #title: HTMLElement | undefined;
  #count: HTMLElement | undefined;
  #empty: HTMLElement | undefined;
  #live: HTMLElement | undefined;
  #annotations: readonly ReviewAnnotation[] = [];
  #measured = new Map<string, number>();
  #placed: readonly PackedCard[] = [];
  #built = new Map<string, HTMLElement>();
  #cursor = 0;
  #stepPixels = 8;
  #rendering = false;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  disconnectedCallback(): void {
    this.#column?.removeEventListener('scroll', this.#onScroll);
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** What the column is called. Announced as the feed's own name. */
  get label(): string {
    return this.getAttribute('label') ?? 'Comments';
  }

  /** Which side of the column the document is on, which the connector contract carries. */
  get side(): ConnectorSide {
    return this.getAttribute('side') === 'inlineEnd' ? 'inlineEnd' : 'inlineStart';
  }

  /** The annotations. **A property** — see `<mjx-virtual-list>` for the argument, which holds here. */
  get annotations(): readonly ReviewAnnotation[] {
    return this.#annotations;
  }

  set annotations(next: readonly ReviewAnnotation[]) {
    this.#annotations = [...next];
    this.#cursor = Math.min(this.#cursor, Math.max(0, this.#annotations.length - 1));
    this.render();
  }

  /** The selected annotation's id, or the empty string. The card that holds its anchor exactly. */
  get selectedId(): string {
    return this.getAttribute('selected') ?? '';
  }

  set selectedId(next: string) {
    if (next === '') this.removeAttribute('selected');
    else this.setAttribute('selected', next);
  }

  /** Where the packer put every card, in anchor order. What a gate reads. */
  get placedCards(): readonly PackedCard[] {
    return this.#placed;
  }

  /** **The connector contract**, for every annotation — built or not. */
  get anchors(): readonly AnnotationAnchorReport[] {
    const roster = assignAuthorColours(this.#annotations.map((entry) => entry.author));
    const byId = new Map(this.#annotations.map((entry) => [entry.id, entry]));
    return this.#placed.map((card) => {
      const entry = byId.get(card.id);
      const model = entry?.kind === 'comment' ? (entry.model ?? 'threaded') : undefined;
      return {
        id: card.id,
        kind: entry?.kind ?? 'comment',
        ...(model === undefined ? {} : { model }),
        anchorTop: card.anchorTop,
        cardTop: card.top,
        cardExtent: card.extent,
        connectorTop: card.top + Math.min(card.extent, 3 * this.#stepPixels),
        side: this.side,
        authorSlot: authorColourFor(entry?.author ?? '', roster),
        selected: card.id === this.selectedId,
      } satisfies AnnotationAnchorReport;
    });
  }

  /** How many cards are actually in the DOM — the number the virtualisation gate compares. */
  get builtCardCount(): number {
    return this.#built.size;
  }

  /** How many the window says there should be, computed by the foundations rather than here. */
  get expectedCardCount(): number {
    const column = this.#column;
    if (column === undefined) return 0;
    if (this.presentation === 'sheet') return this.#placed.length;
    return rowsInWindow(reviewWindow(this.#placed, column.scrollTop, column.clientHeight));
  }

  /** Which presentation the cascade chose. Read from the property the stylesheet wrote. */
  get presentation(): ReviewPresentation {
    const value = getComputedStyle(this).getPropertyValue(reviewPresentationProperty).trim();
    return value === 'sheet' ? 'sheet' : 'margin';
  }

  /** Where the keyboard is. */
  get cursorIndex(): number {
    return this.#cursor;
  }

  /** Put the reader on an annotation, scroll it into view and select it. */
  selectIndex(index: number): void {
    const count = this.#placed.length;
    if (count === 0) return;
    this.#cursor = Math.min(Math.max(index, 0), count - 1);
    const card = this.#placed[this.#cursor];
    if (card === undefined) return;
    this.selectedId = card.id;
    this.render();
    this.#revealCursor();
    this.#built.get(card.id)?.focus();
    this.dispatchEvent(
      new CustomEvent(annotationEvents.select, {
        detail: { id: card.id, index: this.#cursor, count },
        bubbles: true,
        composed: true,
      }),
    );
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, reviewPaneCss);

    const pane = document.createElement('div');
    pane.className = 'pane';

    const handle = document.createElement('div');
    handle.className = 'handle';
    handle.setAttribute('part', 'handle');
    handle.setAttribute('aria-hidden', 'true');

    const title = document.createElement('div');
    title.className = 'title';
    const name = document.createElement('span');
    name.className = annotationTypeRoles.paneTitle;
    const count = document.createElement('span');
    count.className = annotationTypeRoles.meta;
    title.append(name, count);

    const column = document.createElement('div');
    column.className = 'column';
    column.setAttribute('part', 'column');
    // The feed IS the scroll container. Two elements would have meant a reader paging through one
    // thing and a scrollbar moving another.
    column.setAttribute('role', reviewAriaPattern.container);
    column.tabIndex = 0;
    column.addEventListener('scroll', this.#onScroll);
    column.addEventListener('keydown', this.#onKeyDown);
    column.addEventListener('focusin', this.#onFocusIn);

    const sizer = document.createElement('div');
    sizer.className = 'sizer';

    const empty = document.createElement('p');
    empty.className = `empty ${annotationTypeRoles.body}`;
    empty.textContent = 'No comments or tracked changes in this document.';

    // ⚠ A probe, not a registered property. The gap and the connector inset are expressed in
    // density steps, and `--mjx-density-step` is a calc over `--spacing` that
    // `getComputedStyle` hands back un-substituted. Reading a real length off a laid-out box is
    // the answer that needs nothing registered and cannot resolve to a plausible wrong number.
    const metric = document.createElement('div');
    metric.className = 'metric';
    metric.setAttribute('aria-hidden', 'true');
    metric.style.setProperty('position', 'absolute');
    metric.style.setProperty('visibility', 'hidden');
    metric.style.setProperty('inline-size', '0');
    metric.style.setProperty('block-size', 'var(--mjx-density-step)');

    column.append(sizer, metric);

    const live = document.createElement('span');
    live.className = 'live';
    live.setAttribute('aria-live', 'polite');

    pane.append(handle, title, column, empty, live);
    root.append(pane);

    this.#column = column;
    this.#sizer = sizer;
    this.#metric = metric;
    this.#title = name;
    this.#count = count;
    this.#empty = empty;
    this.#live = live;
  }

  /**
   * Pack, window, build, place, report — in that order, and re-entrant-safe.
   *
   * The one subtlety is the measurement pass. A card's height is not known until it is in the DOM,
   * and the packing depends on it, so the first pass places cards on the estimate and the second
   * places them on the measurement. Re-packing only when a measurement actually *changed* is what
   * stops that being a loop.
   */
  render(): void {
    if (this.#rendering) return;
    this.#rendering = true;
    try {
      this.#renderOnce();
      if (this.#absorbMeasurements()) this.#renderOnce();
    } finally {
      this.#rendering = false;
    }
    this.#emitAnchors();
  }

  #renderOnce(): void {
    const column = this.#column;
    const sizer = this.#sizer;
    if (column === undefined || sizer === undefined) return;

    if (this.#title !== undefined) this.#title.textContent = this.label;
    // The feed's own accessible name. Without it a reader is told "feed" and nothing else, and the
    // title beside it is a heading the feed does not reference — an IDREF does not cross a shadow
    // boundary in either direction, so the name has to be written on the element.
    column.setAttribute('aria-label', this.label);
    const total = this.#annotations.length;
    if (this.#count !== undefined) {
      this.#count.textContent = total === 1 ? '1 item' : `${String(total)} items`;
    }
    if (this.#empty !== undefined) this.#empty.hidden = total > 0;

    this.#stepPixels = Math.max(1, this.#metric?.getBoundingClientRect().height ?? this.#stepPixels);

    const cards: MarginCard[] = this.#annotations.map((entry) => ({
      id: entry.id,
      anchorTop: entry.anchorTop,
      extent: this.#measured.get(entry.id) ?? estimatedCardExtent,
    }));
    const gap = marginCardGapUnits * this.#stepPixels;
    this.#placed = packMarginCards(cards, {
      gap,
      columnTop: 0,
      ...(this.selectedId === '' ? {} : { selected: this.selectedId }),
    });

    const last = this.#placed[this.#placed.length - 1];
    sizer.style.blockSize = `${String(last === undefined ? 0 : last.top + last.extent)}px`;

    // ⚠ In the sheet presentation the cards are in flow and the packed offsets mean nothing, so
    // every card is built. A sheet with a virtualised flow layout would be a list whose scroll
    // height was written by an arithmetic that no longer described it.
    const sheet = this.presentation === 'sheet';
    const window_ = sheet
      ? { firstRow: 0, lastRow: this.#placed.length }
      : reviewWindow(this.#placed, column.scrollTop, column.clientHeight);

    column.setAttribute(reviewAriaPattern.busy, 'true');
    const wanted = new Set<string>();
    for (let index = window_.firstRow; index < window_.lastRow; index += 1) {
      const card = this.#placed[index];
      if (card === undefined) continue;
      wanted.add(card.id);
      const element = this.#ensureCard(card.id);
      if (element === undefined) continue;
      element.setAttribute('aria-posinset', String(index + 1));
      element.setAttribute('aria-setsize', String(this.#placed.length));
      element.style.insetBlockStart = sheet ? '' : `${String(card.top)}px`;
      if (element instanceof MjxAnnotationCard) {
        element.stepPixels = this.#stepPixels;
        element.placement = { top: card.top, extent: card.extent };
        element.selected = card.id === this.selectedId;
      }
    }
    // ⚠ **Where the keyboard goes when its card is recycled.** A scroll can take the focused card
    // out of the window, and removing a focused element sends focus to the document body — at
    // which point the feed's own key handling is silently over, because the events no longer reach
    // the column. It looks exactly like a keyboard that stopped working for no reason. So focus
    // returns to the feed, which is focusable for this among other reasons.
    const focused = this.#root?.activeElement;
    for (const [id, element] of [...this.#built]) {
      if (wanted.has(id)) continue;
      if (focused !== null && focused !== undefined && (element === focused || element.contains(focused))) {
        column.focus();
      }
      element.remove();
      this.#built.delete(id);
    }
    column.removeAttribute(reviewAriaPattern.busy);
  }

  #ensureCard(id: string): HTMLElement | undefined {
    const existing = this.#built.get(id);
    if (existing !== undefined) return existing;
    const entry = this.#annotations.find((candidate) => candidate.id === id);
    if (entry === undefined) return undefined;
    const roster = assignAuthorColours(this.#annotations.map((each) => each.author));
    const slot = authorColourFor(entry.author, roster);
    const element = this.#createCard(entry, slot, roster);
    element.classList.add('slot', annotationMotionClass);
    this.#sizer?.append(element);
    this.#built.set(id, element);
    return element;
  }

  #createCard(
    entry: ReviewAnnotation,
    slot: number,
    roster: ReadonlyMap<string, number>,
  ): HTMLElement {
    const isThread =
      entry.kind === 'comment' && (entry.model ?? 'threaded') === 'threaded' && entry.replies !== undefined;
    const tag =
      entry.kind === 'trackedChange'
        ? annotationTags.trackedChangeCard
        : isThread
          ? annotationTags.commentThread
          : annotationTags.commentCard;
    const element = document.createElement(tag);
    element.setAttribute(entry.kind === 'trackedChange' ? 'change-id' : 'comment-id', entry.id);
    element.setAttribute('author', entry.author);
    element.setAttribute('time', entry.time);
    element.setAttribute('anchor-top', String(entry.anchorTop));
    element.setAttribute('author-slot', String(slot));
    element.setAttribute('side', this.side);
    element.textContent = entry.text;
    if (entry.kind === 'trackedChange') {
      element.setAttribute('kind', entry.changeKind ?? 'insertion');
      if (entry.excerpt !== undefined) element.setAttribute('excerpt', entry.excerpt);
    } else {
      element.setAttribute('model', entry.model ?? 'threaded');
      if (entry.resolved === true) element.setAttribute('resolved', '');
      if (isThread && 'replies' in element) {
        const thread = element as HTMLElement & {
          replies: readonly CommentReply[];
          roster: readonly string[];
        };
        thread.replies = (entry.replies ?? []).map((reply) => ({
          ...reply,
          authorSlot: authorColourFor(reply.author, roster),
        }));
        thread.roster = [...new Set(this.#annotations.map((each) => each.author))];
      }
    }
    return element;
  }

  /** Take every built card's real height. Returns whether any of them disagreed with the model. */
  #absorbMeasurements(): boolean {
    let changed = false;
    for (const [id, element] of this.#built) {
      const height = element.getBoundingClientRect().height;
      if (height <= 0) continue;
      const known = this.#measured.get(id);
      if (known !== undefined && Math.abs(known - height) < 0.5) continue;
      this.#measured.set(id, height);
      changed = true;
    }
    return changed;
  }

  #emitAnchors(): void {
    if (!this.isConnected) return;
    this.dispatchEvent(
      new CustomEvent<{ anchors: readonly AnnotationAnchorReport[] }>(annotationEvents.anchors, {
        detail: { anchors: this.anchors },
        bubbles: true,
        composed: true,
      }),
    );
  }

  #revealCursor(): void {
    const column = this.#column;
    const card = this.#placed[this.#cursor];
    if (column === undefined || card === undefined) return;
    if (this.presentation === 'sheet') {
      this.#built.get(card.id)?.scrollIntoView({ block: 'nearest' });
      return;
    }
    const viewport = column.clientHeight;
    if (card.top < column.scrollTop) column.scrollTop = card.top;
    else if (card.top + card.extent > column.scrollTop + viewport) {
      column.scrollTop = card.top + card.extent - viewport;
    }
  }

  #pageStep(): number {
    const column = this.#column;
    if (column === undefined) return fallbackPageStep;
    const viewport = column.clientHeight;
    let step = 0;
    let covered = 0;
    for (let index = this.#cursor; index < this.#placed.length && covered < viewport; index += 1) {
      covered += this.#placed[index]?.extent ?? estimatedCardExtent;
      step += 1;
    }
    return Math.max(fallbackPageStep, step);
  }

  #onScroll = (): void => {
    if (this.#rendering) return;
    this.render();
  };

  #onFocusIn = (event: FocusEvent): void => {
    const target = event.target;
    if (!(target instanceof HTMLElement)) return;
    const card = target.closest('.slot');
    if (!(card instanceof HTMLElement)) return;
    const index = this.#placed.findIndex((placed) => this.#built.get(placed.id) === card);
    if (index >= 0) this.#cursor = index;
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    const count = this.#placed.length;
    if (count === 0) return;
    // The WAI-ARIA feed keyboard, plus the two aliases a reader arriving from Office expects.
    if (event.key === 'PageDown' || event.key === 'ArrowDown') {
      event.preventDefault();
      this.selectIndex(this.#cursor + (event.key === 'PageDown' ? this.#pageStep() : 1));
      return;
    }
    if (event.key === 'PageUp' || event.key === 'ArrowUp') {
      event.preventDefault();
      this.selectIndex(this.#cursor - (event.key === 'PageUp' ? this.#pageStep() : 1));
      return;
    }
    if (event.key === 'Home' && event.ctrlKey) {
      event.preventDefault();
      this.selectIndex(0);
      return;
    }
    if (event.key === 'End' && event.ctrlKey) {
      event.preventDefault();
      this.selectIndex(count - 1);
    }
  };

  /** Say something once, politely. Used by the stories rather than by the component itself. */
  announce(message: string): void {
    if (this.#live !== undefined) this.#live.textContent = message;
  }
}

/** Register the element. Idempotent. */
export function defineReviewPane(): void {
  if (customElements.get(annotationTags.reviewPane) === undefined) {
    customElements.define(annotationTags.reviewPane, MjxReviewPane);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-review-pane': MjxReviewPane;
  }
}
