/**
 * `<mjx-comment-card>` — one remark in the margin, in **either** of Word's two comment models.
 *
 * ```html
 * <mjx-comment-card
 *   comment-id="c1" author="Ada Lovelace" initials="AL"
 *   time="2026-03-04T10:15:00Z" model="threaded" anchor-top="120">
 *   Should this figure be the 1843 one?
 * </mjx-comment-card>
 * ```
 *
 * ## The two models are not two skins
 *
 * `annotation-model.ts` states what they are; what this file does with it is refuse to draw an
 * affordance the model cannot honour. A legacy comment has no `w:commentsExtended` entry, so it has
 * no parent, no children and no `done` flag — there is nowhere for a reply to go and nothing for
 * *resolved* to be stored in. So a legacy card **has no reply button and no resolve button**, and
 * the difference is visible (a squarer, dashed card) *and* audible (it is announced as a note, and a
 * threaded one as a conversation). A card that showed the buttons and disabled them would have been
 * worse: a disabled control says *not now*, and the honest answer here is *not ever, in this file
 * format*.
 *
 * ## The connector contract
 *
 * The card reports where its anchor is — [`anchorReport`] and the `mjx-annotation-anchor` event —
 * so R11's in-canvas connector line (inventory entry 54) has nothing left to measure. **The line
 * itself is canvas and is not this child's**; what is this child's is making sure the canvas never
 * has to read the chrome's layout to draw it.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  annotationEvents,
  annotationIconSize,
  annotationTags,
  commentModels,
  connectorInsetUnits,
  reviewAriaPattern,
  type AnnotationAnchorReport,
  type CommentModel,
  type ConnectorSide,
} from './annotation-model.ts';
import { annotationTypeRoles, commentCardCss } from './annotation-sheets.ts';
import { authorColourProperty, authorColourSlots, authorInitials } from './author-colour.ts';

/** Where the pane placed the card, in the column's own coordinates. */
export interface CardPlacement {
  readonly top: number;
  readonly extent: number;
}

/** The four density steps the connector inset is expressed in, resolved against the real box. */
function connectorOffset(extent: number, step: number): number {
  // Never past the card's own end: a one-line note is shorter than the inset on a comfortable
  // density, and a line landing below the card would point at nothing.
  return Math.min(extent, connectorInsetUnits * step);
}

/** The shared parts every annotation card element carries. */
export interface CardParts {
  readonly card: HTMLElement;
  readonly dot: HTMLElement;
  readonly author: HTMLElement;
  readonly live: HTMLElement;
}

/**
 * The base every annotation card shares: the author band, the anchor report, the placement the pane
 * writes back, and the selected state.
 *
 * A base class rather than a mixin because all three cards are custom elements and a custom element
 * must extend `HTMLElement` anyway — and because what is shared is *state* (the placement, the
 * slot, the anchor) rather than behaviour that could be a free function.
 */
export abstract class MjxAnnotationCard extends HTMLElement {
  #placement: CardPlacement | undefined;
  #stepPixels = 4;

  /** The annotation's identity, which the connector contract and the feed are keyed on. */
  get annotationId(): string {
    return this.getAttribute('comment-id') ?? this.getAttribute('change-id') ?? '';
  }

  /** Who wrote it, exactly as the document carries it. No directory is consulted, ever. */
  get author(): string {
    return this.getAttribute('author') ?? '';
  }

  /** The initials the document stored, or the ones the name implies. */
  get initials(): string {
    const declared = this.getAttribute('initials');
    if (declared !== null && declared.trim() !== '') return declared.trim();
    return authorInitials(this.author);
  }

  /** The timestamp, as the document carries it. */
  get time(): string {
    return this.getAttribute('time') ?? '';
  }

  /** Which author-colour slot this card is drawn in. */
  get authorSlot(): number {
    const declared = Number.parseInt(this.getAttribute('author-slot') ?? '', 10);
    if (!Number.isFinite(declared)) return 0;
    return ((declared % authorColourSlots.length) + authorColourSlots.length) % authorColourSlots.length;
  }

  /** Where the anchor sits in the column's coordinates. The number the canvas owns. */
  get anchorTop(): number {
    const declared = Number.parseFloat(this.getAttribute('anchor-top') ?? '');
    return Number.isFinite(declared) ? declared : 0;
  }

  /** Which side of the column the document is on. */
  get side(): ConnectorSide {
    return this.getAttribute('side') === 'inlineEnd' ? 'inlineEnd' : 'inlineStart';
  }

  /** Whether this is the card the reader is on. */
  get selected(): boolean {
    return this.hasAttribute('selected');
  }

  set selected(value: boolean) {
    if (value) this.setAttribute('selected', '');
    else this.removeAttribute('selected');
  }

  /**
   * Where the pane put it. Written by `<mjx-review-pane>` after packing, and `undefined` for a card
   * standing on its own — in which case the report falls back to the card's own measured box.
   */
  get placement(): CardPlacement | undefined {
    return this.#placement;
  }

  set placement(value: CardPlacement | undefined) {
    this.#placement = value;
    this.reportAnchor();
  }

  /** The density step in CSS pixels, which the pane measures once and hands down. */
  get stepPixels(): number {
    return this.#stepPixels;
  }

  set stepPixels(value: number) {
    if (Number.isFinite(value) && value > 0) this.#stepPixels = value;
    this.reportAnchor();
  }

  /** **The connector contract, for one card.** Pure: it measures, it does not draw. */
  anchorReport(): AnnotationAnchorReport {
    const placement = this.#placement;
    const top = placement?.top ?? this.offsetTop;
    const extent = placement?.extent ?? this.offsetHeight;
    return {
      id: this.annotationId,
      kind: this.annotationKind(),
      ...this.modelDetail(),
      anchorTop: this.anchorTop,
      cardTop: top,
      cardExtent: extent,
      connectorTop: top + connectorOffset(extent, this.#stepPixels),
      side: this.side,
      authorSlot: this.authorSlot,
      selected: this.selected,
    };
  }

  /** Tell whoever is listening where the anchor is now. */
  reportAnchor(): void {
    if (!this.isConnected) return;
    this.dispatchEvent(
      new CustomEvent<AnnotationAnchorReport>(annotationEvents.anchor, {
        detail: this.anchorReport(),
        bubbles: true,
        composed: true,
      }),
    );
  }

  /** Which kind of annotation this element is. */
  protected abstract annotationKind(): 'comment' | 'trackedChange';

  /** The comment model, for a comment; nothing, for a tracked change. */
  protected modelDetail(): { readonly model?: CommentModel } {
    return {};
  }

  /** Paint the author band from the slot. One custom property, read by the sheet. */
  protected applyAuthorColour(): void {
    this.style.setProperty(
      '--mjx-annotation-author-colour',
      `var(${authorColourProperty(this.authorSlot)})`,
    );
  }

  /** The host's ARIA: an article in a feed, named rather than labelled by an IDREF. */
  protected applyArticleRole(name: string): void {
    this.setAttribute('role', reviewAriaPattern.item);
    this.setAttribute('aria-label', name);
    if (!this.hasAttribute('tabindex')) this.tabIndex = 0;
  }
}

/** An action button, built once so all three cards spell one the same way. */
export function actionButton(label: string, icon: string, action: string): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = `action mjx-hit-target ${annotationTypeRoles.action}`;
  button.dataset['action'] = action;
  const glyph = document.createElement('mjx-icon');
  glyph.setAttribute('name', icon);
  glyph.setAttribute('size', String(annotationIconSize));
  button.append(glyph, document.createTextNode(label));
  return button;
}

export class MjxCommentCard extends MjxAnnotationCard {
  static readonly observedAttributes: readonly string[] = [
    'comment-id',
    'author',
    'initials',
    'time',
    'model',
    'resolved',
    'selected',
    'anchor-top',
    'author-slot',
    'side',
  ];

  #root: ShadowRoot | undefined;
  #parts: CardParts | undefined;
  #time: HTMLElement | undefined;
  #badge: HTMLElement | undefined;
  #initials: HTMLElement | undefined;
  #actions: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** Which of the two models this comment is. Threaded unless the document says otherwise. */
  get model(): CommentModel {
    return this.getAttribute('model') === 'legacy' ? 'legacy' : 'threaded';
  }

  /** Whether the thread is closed. Meaningless for a legacy comment, which has no such flag. */
  get resolved(): boolean {
    return commentModels[this.model].resolvable && this.hasAttribute('resolved');
  }

  protected override annotationKind(): 'comment' {
    return 'comment';
  }

  protected override modelDetail(): { readonly model: CommentModel } {
    return { model: this.model };
  }

  /** What a screen reader is told this card is. The audible half of *distinguishable*. */
  get announcement(): string {
    const spec = commentModels[this.model];
    const who = this.author === '' ? 'an unnamed author' : this.author;
    const when = this.time === '' ? '' : `, ${this.time}`;
    const state = this.resolved ? ', resolved' : '';
    return `${spec.announcement} by ${who}${when}${state}`;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, commentCardCss);
    defineIcon();

    const card = document.createElement('div');
    card.className = 'card';
    card.setAttribute('part', 'card');

    const head = document.createElement('div');
    head.className = 'head';

    const dot = document.createElement('span');
    dot.className = 'dot';
    dot.setAttribute('part', 'dot');
    // The name beside it carries the same fact, so the swatch is decoration in the tree.
    dot.setAttribute('aria-hidden', 'true');

    const author = document.createElement('span');
    author.className = `author ${annotationTypeRoles.author}`;
    author.setAttribute('part', 'author');

    const initials = document.createElement('span');
    initials.className = `initials ${annotationTypeRoles.meta}`;

    const time = document.createElement('time');
    time.className = `time ${annotationTypeRoles.meta}`;

    const badge = document.createElement('span');
    badge.className = `badge ${annotationTypeRoles.meta}`;
    badge.setAttribute('part', 'badge');

    head.append(dot, author, initials, time, badge);

    const body = document.createElement('p');
    body.className = `body ${annotationTypeRoles.body}`;
    body.append(document.createElement('slot'));

    const actions = document.createElement('div');
    actions.className = 'actions';
    actions.addEventListener('click', this.#onAction);

    const live = document.createElement('span');
    live.className = 'live';
    live.setAttribute('aria-live', 'polite');

    card.append(head, body, actions, live);
    root.append(card);

    this.#parts = { card, dot, author, live };
    this.#time = time;
    this.#badge = badge;
    this.#initials = initials;
    this.#actions = actions;
  }

  /** Rebuild what changed. Small enough that a diff would cost more than it saved. */
  render(): void {
    const parts = this.#parts;
    if (parts === undefined) return;
    const spec = commentModels[this.model];

    parts.card.dataset['model'] = spec.presentation;
    parts.card.dataset['resolved'] = String(this.resolved);
    parts.card.dataset['selected'] = String(this.selected);
    parts.author.textContent = this.author;
    if (this.#initials !== undefined) {
      const initials = this.initials;
      this.#initials.textContent = initials;
      this.#initials.hidden = initials === '';
    }
    if (this.#time !== undefined) {
      this.#time.textContent = this.time;
      this.#time.setAttribute('datetime', this.time);
      this.#time.hidden = this.time === '';
    }
    if (this.#badge !== undefined) {
      this.#badge.textContent = this.resolved ? 'Resolved' : spec.announcement;
    }

    this.#renderActions(spec.repliable, spec.resolvable);
    this.applyAuthorColour();
    this.applyArticleRole(this.announcement);
    this.reportAnchor();
  }

  #renderActions(repliable: boolean, resolvable: boolean): void {
    const actions = this.#actions;
    if (actions === undefined) return;
    actions.replaceChildren();
    // ⚠ Built from the MODEL's own capabilities, not from a list here. A legacy comment gets no
    // reply and no resolve because its file format has nowhere to put either.
    if (repliable) actions.append(actionButton('Reply', 'comment', 'reply'));
    if (resolvable) {
      actions.append(
        actionButton(this.resolved ? 'Reopen' : 'Resolve', 'checkmark', 'resolve'),
      );
    }
    actions.append(actionButton('Delete', 'delete', 'delete'));
  }

  #onAction = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const button = target.closest('button[data-action]');
    if (!(button instanceof HTMLButtonElement)) return;
    const action = button.dataset['action'];
    const id = this.annotationId;
    if (action === 'reply') {
      this.dispatchEvent(
        new CustomEvent(annotationEvents.reply, { detail: { id, text: '' }, bubbles: true, composed: true }),
      );
      return;
    }
    if (action === 'resolve') {
      const next = !this.resolved;
      if (next) this.setAttribute('resolved', '');
      else this.removeAttribute('resolved');
      this.#announce(next ? 'Resolved.' : 'Reopened.');
      this.dispatchEvent(
        new CustomEvent(annotationEvents.resolve, {
          detail: { id, resolved: next },
          bubbles: true,
          composed: true,
        }),
      );
      return;
    }
    if (action === 'delete') {
      this.dispatchEvent(
        new CustomEvent(annotationEvents.delete, { detail: { id }, bubbles: true, composed: true }),
      );
    }
  };

  #announce(message: string): void {
    const live = this.#parts?.live;
    if (live !== undefined) live.textContent = message;
  }
}

/** Register the element. Idempotent. */
export function defineCommentCard(): void {
  if (customElements.get(annotationTags.commentCard) === undefined) {
    customElements.define(annotationTags.commentCard, MjxCommentCard);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-comment-card': MjxCommentCard;
  }
}
