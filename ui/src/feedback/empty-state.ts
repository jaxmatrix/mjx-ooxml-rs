/**
 * `<mjx-empty-state>` — **what a region says when there is nothing in it, and why that is the
 * easiest thing in this child to ship wrong.**
 *
 * ```html
 * <mjx-empty-state heading="No comments yet"
 *                  description="Comments you and your reviewers add will appear here."
 *                  action-label="Add a comment" action-command="comment.add">
 * </mjx-empty-state>
 * ```
 *
 * MJXOFF-189 puts it exactly: *"An empty state is the easiest thing to ship wrong, because it
 * renders fine with no data by definition."* Every visual check passes — there is a picture, there
 * is a heading, there is a button — and the two things that actually matter are invisible:
 *
 * * **the action has to work.** A decorative button is the commonest defect in an empty state,
 *   because the state is usually built while the thing the button does is still a stub. So the
 *   catalogue's story wires the action to the *real* thing — it fills the list — and
 *   `tests/browser/feedback.spec.ts` clicks it and asserts the empty state is gone and the list has
 *   items in it. Nothing weaker would notice a button that fires nothing.
 * * **it has to be announced.** An empty state is what a region *becomes* when a filter, a search
 *   or a delete emptied it, so its appearance is an update — and a person who cannot see it is
 *   otherwise told nothing at all. `role="status"` with `aria-live="polite"` is the answer, the art
 *   is `aria-hidden` because a decorative glyph announced by name is worse than one announced by
 *   nothing, and the heading is real text rather than a picture of text.
 *
 * ## The one place the serif is allowed
 *
 * `foundations/typography.ts` §4: *Young Serif is display-only. It belongs in empty states and
 * onboarding, never in the chrome or in document content.* This is that place. The heading wears
 * `.mjx-type-display` and `tests/feedback.test.ts` asserts that this component names that role and
 * that no other component in `src/` does — which turns a sentence in a design document into a fact
 * about the tree.
 *
 * ## The action is a real button this component owns
 *
 * Not a slot, for the reason `<mjx-mini-toolbar>` gives about its commands: a slotted control is a
 * control this component cannot put in a known state, and *“the action works”* is the assertion the
 * whole component exists to support. A second, optional action is a slot, because *that* one is
 * genuinely the application's — a link to help, a different route out.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  emptyStateCss,
  emptyStateIcon,
  emptyStateRole,
  feedbackEvents,
  feedbackMotionClasses,
  feedbackTags,
  feedbackTypeRoles,
} from './feedback-model.ts';

let nextEmptyStateSerial = 0;

export class MjxEmptyState extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'heading',
    'description',
    'action-label',
    'action-command',
    'icon',
  ];

  #root: ShadowRoot | undefined;
  #box: HTMLElement | undefined;
  #art: HTMLElement | undefined;
  #headingElement: HTMLElement | undefined;
  #descriptionElement: HTMLElement | undefined;
  #action: HTMLButtonElement | undefined;
  #glyph: HTMLElement | undefined;
  #headingId = '';

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The line a person reads first. Real text, never a picture of text. */
  get heading(): string {
    return this.getAttribute('heading') ?? '';
  }

  /** What to do about it. An empty state that only says *nothing here* has not helped. */
  get description(): string {
    return this.getAttribute('description') ?? '';
  }

  /** The primary action's label. Absent means there is no action, which is a real answer. */
  get actionLabel(): string {
    return this.getAttribute('action-label') ?? '';
  }

  /** What the action emits. Defaults to its label, so a story need not say the same thing twice. */
  get actionCommand(): string {
    const declared = this.getAttribute('action-command');
    return declared === null || declared === '' ? this.actionLabel : declared;
  }

  /** The glyph, from the committed subset. */
  get icon(): string {
    return this.getAttribute('icon') ?? emptyStateIcon.name;
  }

  /** The action, so a gate presses the thing a person presses. */
  get actionButton(): HTMLButtonElement | undefined {
    return this.#action;
  }

  /** The announced box, so a gate reads the role off the element rather than off this file. */
  get statusRegion(): HTMLElement | undefined {
    return this.#box;
  }

  override focus(options?: FocusOptions): void {
    if (this.#action === undefined) super.focus(options);
    else this.#action.focus(options);
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, emptyStateCss);
    defineIcon();

    nextEmptyStateSerial += 1;
    this.#headingId = `mjx-empty-heading-${String(nextEmptyStateSerial)}`;

    const box = document.createElement('div');
    box.className = 'empty';
    box.setAttribute('part', 'empty');
    box.setAttribute('role', emptyStateRole);
    box.setAttribute('aria-live', 'polite');
    box.setAttribute('aria-labelledby', this.#headingId);
    this.#box = box;

    const art = document.createElement('span');
    art.className = 'art';
    art.setAttribute('part', 'art');
    // Decorative. A glyph announced by its Fluent name is noise, and a glyph announced as an image
    // with no name is worse.
    art.setAttribute('aria-hidden', 'true');
    this.#art = art;

    const heading = document.createElement('h2');
    heading.className = `heading ${feedbackTypeRoles.emptyHeading}`;
    heading.setAttribute('part', 'heading');
    heading.id = this.#headingId;
    this.#headingElement = heading;

    const description = document.createElement('p');
    description.className = `description ${feedbackTypeRoles.emptyBody}`;
    description.setAttribute('part', 'description');
    this.#descriptionElement = description;

    const actions = document.createElement('div');
    actions.className = 'actions';
    actions.setAttribute('part', 'actions');

    const action = document.createElement('button');
    action.type = 'button';
    action.className = `action mjx-hit-target ${feedbackTypeRoles.emptyAction} ${feedbackMotionClasses.toolbar}`;
    action.setAttribute('part', 'action');
    action.addEventListener('click', this.#onAction);
    this.#action = action;

    // The secondary way out belongs to the application, so it is a slot.
    const extra = document.createElement('slot');
    extra.name = 'secondary';

    actions.append(action, extra);
    box.append(art, heading, description, actions);
    root.append(box);
  }

  #onAction = (): void => {
    this.dispatchEvent(
      new CustomEvent(feedbackEvents.emptyStateAction, {
        bubbles: true,
        composed: true,
        detail: { command: this.actionCommand },
      }),
    );
  };

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const art = this.#art;
    if (art === undefined) return;

    // The glyph's attributes are updated rather than the element replaced: `<mjx-icon>` upgrades and
    // resolves its drawing on connection, so rebuilding it on every attribute change would make the
    // art flicker whenever an unrelated one moved.
    let glyph = this.#glyph;
    if (glyph === undefined) {
      glyph = document.createElement('mjx-icon');
      art.append(glyph);
      this.#glyph = glyph;
    }
    glyph.setAttribute('name', this.icon);
    glyph.setAttribute('size', String(emptyStateIcon.size));
    glyph.setAttribute('variant', emptyStateIcon.variant);

    if (this.#headingElement !== undefined) this.#headingElement.textContent = this.heading;
    if (this.#descriptionElement !== undefined) {
      this.#descriptionElement.textContent = this.description;
      this.#descriptionElement.hidden = this.description === '';
    }
    if (this.#action !== undefined) {
      this.#action.textContent = this.actionLabel;
      this.#action.hidden = this.actionLabel === '';
    }
  }
}

/** Register the element. Idempotent. */
export function defineEmptyState(): void {
  if (customElements.get(feedbackTags.emptyState) === undefined) {
    customElements.define(feedbackTags.emptyState, MjxEmptyState);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-empty-state': MjxEmptyState;
  }
}
