/**
 * `<mjx-comment-thread>` — a comment with its replies, and the surface that has to be **announced
 * as a conversation**.
 *
 * ```html
 * <mjx-comment-thread comment-id="t1" author="Ada Lovelace" time="10:15" anchor-top="120">
 *   Should this figure be the 1843 one?
 * </mjx-comment-thread>
 * <script>
 *   document.querySelector('mjx-comment-thread').replies = [
 *     { id: 'r1', author: 'Charles Babbage', time: '10:22', text: 'It should.' },
 *   ];
 * </script>
 * ```
 *
 * ## Replies are a property, and the reason is not virtualisation
 *
 * U12 made items a property because five thousand descriptor elements is the cost virtualisation
 * exists to avoid. A thread is never five thousand replies. The reason here is different and worth
 * stating: a reply carries an **author**, whose colour comes from the pane's roster, and a **time**,
 * and a thread written as markup would either repeat those as attributes on nested elements — a
 * second card element nobody asked for — or lose them. A property is the shape the data already
 * has.
 *
 * The root comment's own text stays a slot, so a thread is still readable in view-source and a
 * story can write one in HTML.
 *
 * ## Collapsed is the default, and it is not a styling state
 *
 * A margin column of expanded threads is a column of one thread. Collapsed shows the root and the
 * **most recent** reply — not the first, because the thing a reviewer needs is where the
 * conversation got to — and says how many are hidden. `aria-expanded` on the disclosure and a live
 * announcement of the count are what make that legible to a reader who cannot see the fold.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  annotationEvents,
  annotationIconSize,
  annotationTags,
  collapsedReplyCount,
  commentModels,
  reviewAriaPattern,
} from './annotation-model.ts';
import { annotationTypeRoles, commentThreadCss } from './annotation-sheets.ts';
import { MjxAnnotationCard, actionButton } from './comment-card.ts';

/** One reply in a thread. */
export interface CommentReply {
  readonly id: string;
  readonly author: string;
  readonly time: string;
  readonly text: string;
  /** Which author-colour slot the reply's author holds, from the pane's roster. */
  readonly authorSlot?: number;
}

export class MjxCommentThread extends MjxAnnotationCard {
  static readonly observedAttributes: readonly string[] = [
    'comment-id',
    'author',
    'initials',
    'time',
    'resolved',
    'selected',
    'expanded',
    'anchor-top',
    'author-slot',
    'side',
  ];

  #root: ShadowRoot | undefined;
  #card: HTMLElement | undefined;
  #author: HTMLElement | undefined;
  #time: HTMLElement | undefined;
  #badge: HTMLElement | undefined;
  #replyList: HTMLElement | undefined;
  #more: HTMLButtonElement | undefined;
  #composer: HTMLTextAreaElement | undefined;
  #mentions: HTMLElement | undefined;
  #actions: HTMLElement | undefined;
  #live: HTMLElement | undefined;
  #replies: readonly CommentReply[] = [];
  #roster: readonly string[] = [];

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The replies, newest last. A property — see the note at the top of this file. */
  get replies(): readonly CommentReply[] {
    return this.#replies;
  }

  set replies(next: readonly CommentReply[]) {
    this.#replies = [...next];
    this.render();
  }

  /** The names a mention affordance may offer: **the document's own authors and nobody else.** */
  get roster(): readonly string[] {
    return this.#roster;
  }

  set roster(next: readonly string[]) {
    this.#roster = [...next];
    this.render();
  }

  /** Whether every reply is shown. */
  get expanded(): boolean {
    return this.hasAttribute('expanded');
  }

  set expanded(value: boolean) {
    if (value) this.setAttribute('expanded', '');
    else this.removeAttribute('expanded');
  }

  /** Whether the conversation is closed. A thread is the threaded model, so it always may be. */
  get resolved(): boolean {
    return this.hasAttribute('resolved');
  }

  /** How many replies the fold is hiding. The number the disclosure says out loud. */
  get hiddenReplyCount(): number {
    if (this.expanded) return 0;
    return Math.max(0, this.#replies.length - collapsedReplyCount);
  }

  protected override annotationKind(): 'comment' {
    return 'comment';
  }

  protected override modelDetail(): { readonly model: 'threaded' } {
    return { model: 'threaded' };
  }

  /** What the whole thread is announced as. */
  get announcement(): string {
    const who = this.author === '' ? 'an unnamed author' : this.author;
    const count = this.#replies.length;
    const replies = count === 0 ? 'no replies' : count === 1 ? '1 reply' : `${String(count)} replies`;
    const state = this.resolved ? ', resolved' : '';
    return `${commentModels.threaded.announcement} started by ${who}, ${replies}${state}`;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, commentThreadCss);
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
    const badge = document.createElement('span');
    badge.className = `badge ${annotationTypeRoles.meta}`;
    head.append(dot, author, time, badge);

    const body = document.createElement('p');
    body.className = `body ${annotationTypeRoles.body}`;
    body.append(document.createElement('slot'));

    const replyList = document.createElement('div');
    replyList.className = 'replies';

    const more = document.createElement('button');
    more.type = 'button';
    more.className = `more ${annotationTypeRoles.meta}`;
    more.addEventListener('click', this.#onToggle);

    const composer = document.createElement('div');
    composer.className = 'composer';
    const field = document.createElement('textarea');
    field.rows = 2;
    field.setAttribute('aria-label', 'Write a reply');
    field.placeholder = 'Reply…';
    const mentions = document.createElement('div');
    mentions.className = 'mentions';
    mentions.addEventListener('click', this.#onMention);
    composer.append(field, mentions);

    const actions = document.createElement('div');
    actions.className = 'actions';
    actions.addEventListener('click', this.#onAction);

    const live = document.createElement('span');
    live.className = 'live';
    live.setAttribute('aria-live', 'polite');

    card.append(head, body, replyList, more, composer, actions, live);
    root.append(card);

    this.#card = card;
    this.#author = author;
    this.#time = time;
    this.#badge = badge;
    this.#replyList = replyList;
    this.#more = more;
    this.#composer = field;
    this.#mentions = mentions;
    this.#actions = actions;
    this.#live = live;
  }

  render(): void {
    const card = this.#card;
    if (card === undefined) return;
    card.dataset['model'] = commentModels.threaded.presentation;
    card.dataset['resolved'] = String(this.resolved);
    card.dataset['selected'] = String(this.selected);
    if (this.#author !== undefined) this.#author.textContent = this.author;
    if (this.#time !== undefined) {
      this.#time.textContent = this.time;
      this.#time.setAttribute('datetime', this.time);
      this.#time.hidden = this.time === '';
    }
    if (this.#badge !== undefined) {
      this.#badge.textContent = this.resolved ? 'Resolved' : commentModels.threaded.announcement;
    }

    this.#renderReplies();
    this.#renderMentions();
    this.#renderActions();
    this.applyAuthorColour();
    this.applyArticleRole(this.announcement);
    this.reportAnchor();
  }

  #renderReplies(): void {
    const list = this.#replyList;
    const more = this.#more;
    if (list === undefined || more === undefined) return;
    // Collapsed shows the LATEST reply, not the first: what a reviewer needs to know is where the
    // conversation got to, and a thread whose fold showed its oldest answer would hide the one
    // thing they opened the margin for.
    const shown = this.expanded ? this.#replies : this.#replies.slice(-collapsedReplyCount);
    list.replaceChildren();
    for (const reply of shown) {
      list.append(this.#buildReply(reply));
    }
    const hidden = this.hiddenReplyCount;
    more.hidden = hidden === 0 && !this.expanded;
    more.setAttribute('aria-expanded', String(this.expanded));
    more.textContent = this.expanded
      ? 'Show fewer replies'
      : hidden === 1
        ? 'Show 1 earlier reply'
        : `Show ${String(hidden)} earlier replies`;
  }

  #buildReply(reply: CommentReply): HTMLElement {
    const item = document.createElement('div');
    item.className = 'reply';
    // A reply is an authored thing, so it is an article of its own inside the conversation. A
    // paragraph here is what the ticket means by "a flat list of paragraphs".
    item.setAttribute('role', reviewAriaPattern.item);
    item.setAttribute('aria-label', `Reply by ${reply.author}${reply.time === '' ? '' : `, ${reply.time}`}`);
    const head = document.createElement('div');
    head.className = 'head';
    const author = document.createElement('span');
    author.className = `author ${annotationTypeRoles.author}`;
    author.textContent = reply.author;
    const time = document.createElement('time');
    time.className = `time ${annotationTypeRoles.meta}`;
    time.textContent = reply.time;
    time.setAttribute('datetime', reply.time);
    head.append(author, time);
    const text = document.createElement('p');
    text.className = `body ${annotationTypeRoles.body}`;
    text.textContent = reply.text;
    item.append(head, text);
    return item;
  }

  #renderMentions(): void {
    const mentions = this.#mentions;
    if (mentions === undefined) return;
    mentions.replaceChildren();
    // ⚠ **The roster and nothing else.** MJXOFF-193 is explicit that there is no author identity
    // beyond what the document carries — no avatar service, no presence, no directory — so a
    // mention offers the people who have already written in this file and refuses to invent one.
    for (const name of this.#roster) {
      if (name.trim() === '' || name === this.author) continue;
      const chip = document.createElement('button');
      chip.type = 'button';
      chip.className = `mention mjx-hit-target ${annotationTypeRoles.meta}`;
      chip.dataset['mention'] = name;
      chip.textContent = `@${name}`;
      mentions.append(chip);
    }
  }

  #renderActions(): void {
    const actions = this.#actions;
    if (actions === undefined) return;
    actions.replaceChildren(
      actionButton('Reply', 'comment', 'reply'),
      actionButton(this.resolved ? 'Reopen' : 'Resolve', 'checkmark', 'resolve'),
      actionButton('Delete', 'delete', 'delete'),
    );
    const glyph = document.createElement('mjx-icon');
    glyph.setAttribute('name', this.expanded ? 'chevron-up' : 'chevron-down');
    glyph.setAttribute('size', String(annotationIconSize));
    glyph.setAttribute('aria-hidden', 'true');
    actions.append(glyph);
  }

  #onToggle = (): void => {
    this.expanded = !this.expanded;
    this.#announce(
      this.expanded
        ? `Showing all ${String(this.#replies.length)} replies.`
        : `Showing the latest reply. ${String(this.hiddenReplyCount)} hidden.`,
    );
    this.dispatchEvent(
      new CustomEvent(annotationEvents.expand, {
        detail: { id: this.annotationId, expanded: this.expanded },
        bubbles: true,
        composed: true,
      }),
    );
  };

  #onMention = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const chip = target.closest('button[data-mention]');
    if (!(chip instanceof HTMLButtonElement)) return;
    const name = chip.dataset['mention'] ?? '';
    const field = this.#composer;
    if (field === undefined) return;
    const prefix = field.value === '' || field.value.endsWith(' ') ? '' : ' ';
    field.value = `${field.value}${prefix}@${name} `;
    field.focus();
    field.setSelectionRange(field.value.length, field.value.length);
  };

  #onAction = (event: Event): void => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    const button = target.closest('button[data-action]');
    if (!(button instanceof HTMLButtonElement)) return;
    const id = this.annotationId;
    const action = button.dataset['action'];
    if (action === 'reply') {
      const field = this.#composer;
      const text = field?.value.trim() ?? '';
      if (field !== undefined) field.value = '';
      this.#announce(text === '' ? 'Write a reply first.' : 'Reply added.');
      this.dispatchEvent(
        new CustomEvent(annotationEvents.reply, { detail: { id, text }, bubbles: true, composed: true }),
      );
      return;
    }
    if (action === 'resolve') {
      const next = !this.resolved;
      if (next) this.setAttribute('resolved', '');
      else this.removeAttribute('resolved');
      this.#announce(next ? 'Conversation resolved.' : 'Conversation reopened.');
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
    if (this.#live !== undefined) this.#live.textContent = message;
  }
}

/** Register the element. Idempotent. */
export function defineCommentThread(): void {
  if (customElements.get(annotationTags.commentThread) === undefined) {
    customElements.define(annotationTags.commentThread, MjxCommentThread);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-comment-thread': MjxCommentThread;
  }
}
