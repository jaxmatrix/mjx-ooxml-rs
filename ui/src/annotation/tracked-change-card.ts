/**
 * `<mjx-tracked-change-card>` — one revision in the margin: an insertion, a deletion, a formatting
 * change or a move, with its author's colour and an accept/reject pair.
 *
 * ```html
 * <mjx-tracked-change-card
 *   change-id="r7" kind="deletion" author="Charles Babbage"
 *   time="11:04" anchor-top="240" excerpt="the Analytical Engine">
 *   Deleted three words from the second paragraph.
 * </mjx-tracked-change-card>
 * ```
 *
 * ## The four kinds differ in more than colour, on purpose
 *
 * Word tells insertions from deletions by colour alone in the body text, and it can afford to
 * because the underline and the strike-through are there too. A **card** has neither by default, so
 * a card distinguished by hue would be four identical cards to the reader this child's author
 * palette exists for. So each kind carries an icon, a verb and a treatment of its excerpt — an
 * insertion is underlined, a deletion struck through — and the colour is the *author's*, not the
 * kind's, exactly as it is in the document.
 *
 * That the author colour and the change colour are different facts is worth saying plainly: the
 * palette has `document.*.tracked-change-insert` and `-delete` for what the canvas paints in the
 * text, and those are two colours for four kinds and are tagged fill-only. They are not what a card
 * is banded with.
 *
 * ## Accepting is not implemented against a document, and says so
 *
 * The ticket puts *accepting or rejecting a change against a real document* in loop 2. The card
 * emits a verdict and removes nothing; the event is the whole contract.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  annotationEvents,
  annotationIconSize,
  annotationTags,
  trackedChangeKindNames,
  trackedChangeKinds,
  type TrackedChangeKind,
  type TrackedChangeVerdict,
} from './annotation-model.ts';
import { annotationTypeRoles, trackedChangeCardCss } from './annotation-sheets.ts';
import { MjxAnnotationCard, actionButton } from './comment-card.ts';

function isKind(value: string | null): value is TrackedChangeKind {
  return value !== null && (trackedChangeKindNames as readonly string[]).includes(value);
}

export class MjxTrackedChangeCard extends MjxAnnotationCard {
  static readonly observedAttributes: readonly string[] = [
    'change-id',
    'kind',
    'author',
    'initials',
    'time',
    'excerpt',
    'selected',
    'anchor-top',
    'author-slot',
    'side',
  ];

  #root: ShadowRoot | undefined;
  #card: HTMLElement | undefined;
  #author: HTMLElement | undefined;
  #time: HTMLElement | undefined;
  #kindLine: HTMLElement | undefined;
  #kindIcon: HTMLElement | undefined;
  #kindWord: HTMLElement | undefined;
  #excerpt: HTMLElement | undefined;
  #actions: HTMLElement | undefined;
  #live: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** Which revision this is. An unrecognised value is an insertion, which is the commonest. */
  get kind(): TrackedChangeKind {
    const declared = this.getAttribute('kind');
    return isKind(declared) ? declared : 'insertion';
  }

  /** The words the revision touched, shown so a reviewer need not look away from the margin. */
  get excerpt(): string {
    return this.getAttribute('excerpt') ?? '';
  }

  protected override annotationKind(): 'trackedChange' {
    return 'trackedChange';
  }

  /** What a screen reader is told. The verb first, because that is what a reviewer is deciding. */
  get announcement(): string {
    const spec = trackedChangeKinds[this.kind];
    const who = this.author === '' ? 'an unnamed author' : this.author;
    const when = this.time === '' ? '' : `, ${this.time}`;
    return `${spec.verb} by ${who}${when}`;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, trackedChangeCardCss);
    defineIcon();

    const card = document.createElement('div');
    card.className = 'card';
    card.setAttribute('part', 'card');

    const head = document.createElement('div');
    head.className = 'head';
    const dot = document.createElement('span');
    dot.className = 'dot';
    dot.setAttribute('aria-hidden', 'true');
    const author = document.createElement('span');
    author.className = `author ${annotationTypeRoles.author}`;
    const time = document.createElement('time');
    time.className = `time ${annotationTypeRoles.meta}`;
    head.append(dot, author, time);

    const kindLine = document.createElement('p');
    kindLine.className = `kind ${annotationTypeRoles.meta}`;
    const kindIcon = document.createElement('mjx-icon');
    kindIcon.setAttribute('size', String(annotationIconSize));
    kindIcon.setAttribute('aria-hidden', 'true');
    const kindWord = document.createElement('span');
    kindLine.append(kindIcon, kindWord);

    const excerpt = document.createElement('p');
    excerpt.className = `excerpt ${annotationTypeRoles.body}`;

    const description = document.createElement('p');
    description.className = `description ${annotationTypeRoles.body}`;
    description.append(document.createElement('slot'));

    const actions = document.createElement('div');
    actions.className = 'actions';
    actions.addEventListener('click', this.#onAction);

    const live = document.createElement('span');
    live.className = 'live';
    live.setAttribute('aria-live', 'polite');

    card.append(head, kindLine, excerpt, description, actions, live);
    root.append(card);

    this.#card = card;
    this.#author = author;
    this.#time = time;
    this.#kindLine = kindLine;
    this.#kindIcon = kindIcon;
    this.#kindWord = kindWord;
    this.#excerpt = excerpt;
    this.#actions = actions;
    this.#live = live;
  }

  render(): void {
    const card = this.#card;
    if (card === undefined) return;
    const spec = trackedChangeKinds[this.kind];
    card.dataset['kind'] = spec.presentation;
    card.dataset['selected'] = String(this.selected);
    if (this.#author !== undefined) this.#author.textContent = this.author;
    if (this.#time !== undefined) {
      this.#time.textContent = this.time;
      this.#time.setAttribute('datetime', this.time);
      this.#time.hidden = this.time === '';
    }
    this.#kindIcon?.setAttribute('name', spec.icon);
    if (this.#kindWord !== undefined) this.#kindWord.textContent = spec.verb;
    if (this.#kindLine !== undefined) this.#kindLine.dataset['element'] = spec.element;
    if (this.#excerpt !== undefined) {
      this.#excerpt.textContent = this.excerpt;
      this.#excerpt.hidden = this.excerpt === '';
    }
    if (this.#actions !== undefined && this.#actions.childElementCount === 0) {
      this.#actions.append(
        actionButton('Accept', 'checkmark', 'accept'),
        actionButton('Reject', 'dismiss', 'reject'),
      );
    }
    this.applyAuthorColour();
    this.applyArticleRole(this.announcement);
    this.reportAnchor();
  }

  #onAction = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const button = target.closest('button[data-action]');
    if (!(button instanceof HTMLButtonElement)) return;
    const action = button.dataset['action'];
    if (action !== 'accept' && action !== 'reject') return;
    const verdict: TrackedChangeVerdict = action;
    if (this.#live !== undefined) {
      this.#live.textContent = verdict === 'accept' ? 'Change accepted.' : 'Change rejected.';
    }
    this.dispatchEvent(
      new CustomEvent(annotationEvents.verdict, {
        detail: { id: this.annotationId, verdict },
        bubbles: true,
        composed: true,
      }),
    );
  };
}

/** Register the element. Idempotent. */
export function defineTrackedChangeCard(): void {
  if (customElements.get(annotationTags.trackedChangeCard) === undefined) {
    customElements.define(annotationTags.trackedChangeCard, MjxTrackedChangeCard);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-tracked-change-card': MjxTrackedChangeCard;
  }
}
