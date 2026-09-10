/**
 * `<mjx-label>` — a field's name, and the one honest way to attach it across a shadow boundary.
 *
 * ```html
 * <mjx-label for="size" required hint="Between 1 and 1638 points.">Font size</mjx-label>
 * <mjx-measure-input id="size" value="12 pt"></mjx-measure-input>
 * ```
 *
 * ## Why this does not use `<label for>`, and why that is not a shortcut
 *
 * `for` and `aria-labelledby` are **IDREF** attributes, and an IDREF resolves inside one tree. The
 * control this label names keeps its focusable element inside its own shadow root, so a `for` here
 * would point at a custom element the browser does not consider labelable, and an
 * `aria-labelledby` here would name an id the control's shadow tree cannot see. Both would produce
 * a label that *looks* attached in the markup and announces nothing — the worst available outcome,
 * because it passes review.
 *
 * `ElementInternals.ariaLabelledByElements` is the standards answer and is not yet available
 * everywhere this catalogue must run, so what happens instead is stated rather than implied: this
 * element **pushes its text into the control's `label` attribute**, and every input in this child
 * puts that text on its inner focusable as the accessible name. One string, in the place the
 * platform already computes names from.
 *
 * Two consequences follow, and both are asserted:
 *
 * * **The push is a default, never an override.** A control that already declares a `label` keeps
 *   it. That is this project's standing rule — author a default only where nothing exists — and it
 *   is what stops a decorative caption silently renaming a command.
 * * **Clicking the label focuses the control**, because that is the behaviour a `<label>` would
 *   have given and losing it would be a real regression. It is implemented rather than inherited,
 *   which is why it is in the keyboard/screen-reader declaration.
 *
 * A label with no `for` is just a caption — the common case in a properties inspector, where the
 * name sits above a row of controls and belongs to none of them.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { inputTypeClass, labelCss } from './input-model.ts';

/** The attribute a control reads its accessible name from. One name, one place. */
export const labelTargetAttribute = 'label';

/** What a screen reader hears after the name of a required field. Announced, never only drawn. */
export const requiredAnnouncement = 'required';

/** The glyph a required field is marked with. Drawn; `requiredAnnouncement` is what is read. */
export const requiredMark = '*';

export class MjxLabel extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['for', 'required', 'hint'];

  #root: ShadowRoot | undefined;
  #label: HTMLElement | undefined;
  #text: HTMLElement | undefined;
  #required: HTMLElement | undefined;
  #hint: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The id of the control this names, or `undefined` for a plain caption. */
  get htmlFor(): string | undefined {
    const declared = this.getAttribute('for');
    return declared === null || declared === '' ? undefined : declared;
  }

  /** Whether the field must be filled in. */
  get required(): boolean {
    return this.hasAttribute('required');
  }

  /** A line under the label. Secondary text, on the surface — never inside a field. */
  get hint(): string {
    return this.getAttribute('hint') ?? '';
  }

  /**
   * The control this label names, or `undefined`.
   *
   * Resolved through the **root node**, so a label and its control inside a shadow tree find each
   * other and a label in the document finds a control in the document. `getElementById` on the
   * document would work for one of those and silently fail for the other.
   */
  get control(): HTMLElement | undefined {
    const id = this.htmlFor;
    if (id === undefined) return undefined;
    const root = this.getRootNode();
    const found =
      root instanceof Document || root instanceof ShadowRoot
        ? root.getElementById(id)
        : this.ownerDocument.getElementById(id);
    return found ?? undefined;
  }

  /** The text this label carries, with the required mark and the hint left out. */
  get text(): string {
    return (this.textContent ?? '').trim();
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, labelCss);

    const label = document.createElement('span');
    label.className = 'label';
    label.setAttribute('part', 'label');
    label.addEventListener('click', this.#onClick);

    const text = document.createElement('span');
    text.append(document.createElement('slot'));

    const required = document.createElement('span');
    required.className = 'required';
    required.setAttribute('aria-hidden', 'true');
    required.textContent = requiredMark;

    const announced = document.createElement('span');
    announced.className = 'visually-hidden';
    announced.textContent = ` ${requiredAnnouncement}`;

    const hint = document.createElement('span');
    hint.className = 'hint';
    hint.setAttribute('part', 'hint');

    label.append(text, required, announced);
    root.append(label, hint);

    this.#label = label;
    this.#text = text;
    this.#required = required;
    this.#hint = hint;
    // The announced word lives beside the mark and is shown or hidden with it.
    required.dataset['pair'] = '';
    announced.dataset['pair'] = '';
  }

  #onClick = (): void => {
    const control = this.control;
    if (control === undefined) return;
    // `focus()` on a component in this child is overridden to reach the inner focusable, exactly
    // as `<mjx-button>` does — so this is one call and not a walk through somebody's shadow root.
    control.focus();
  };

  /** Re-render, and push the name onto the control if it has none. */
  render(): void {
    const label = this.#label;
    const required = this.#required;
    const hint = this.#hint;
    if (label === undefined || required === undefined || hint === undefined) return;

    label.className = `label ${inputTypeClass('fieldLabel')}`;
    const isRequired = this.required;
    for (const element of label.querySelectorAll('[data-pair]')) {
      (element as HTMLElement).hidden = !isRequired;
    }

    hint.textContent = this.hint;
    hint.className = `hint ${inputTypeClass('hint')}`;
    hint.hidden = this.hint === '';

    this.#pushName();
  }

  /**
   * Give the control its name — **only if it does not already have one.**
   *
   * Deferred to a microtask on the first attempt, because a label written before its control in
   * the markup runs `connectedCallback` before the control exists. One retry is enough: both
   * elements are parsed in the same task, so by the time the microtask queue drains the control is
   * in the tree whichever order they were written in.
   */
  #pushName(): void {
    const apply = (): void => {
      const control = this.control;
      if (control === undefined) return;
      const existing = control.getAttribute(labelTargetAttribute);
      if (existing !== null && existing.trim() !== '') return;
      const text = this.text;
      if (text === '') return;
      control.setAttribute(labelTargetAttribute, text);
    };
    apply();
    queueMicrotask(apply);
  }

  /** The text element, for a subclass and for a gate. */
  protected get textElement(): HTMLElement | undefined {
    return this.#text;
  }
}

/** Register the element. Idempotent. */
export function defineLabel(): void {
  if (customElements.get('mjx-label') === undefined) customElements.define('mjx-label', MjxLabel);
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-label': MjxLabel;
  }
}
