/**
 * `<mjx-checkbox>` — the control with **three** states, and the third is the one that gets lost.
 *
 * ```html
 * <mjx-checkbox label="Ruler" checked="true"></mjx-checkbox>
 * <mjx-checkbox label="Bold" checked="mixed"></mjx-checkbox>
 * <mjx-checkbox label="Track changes" unavailable
 *               explanation="The document is not shared."></mjx-checkbox>
 * ```
 *
 * ## `mixed` is an attribute value, not a class
 *
 * MJXOFF-186 names the trap exactly: *"a checkbox has three states. If every story shows two,
 * `indeterminate` has never rendered — and `aria-checked="mixed"` is a different attribute value,
 * not a class."* So the third position is `aria-checked="mixed"` written by the component, the
 * states matrix has a `mixed` cell, and `tests/browser/inputs.spec.ts` reads the attribute rather
 * than a data hook — a component that painted a mixed box and announced `false` would look right
 * and be unusable, which is the failure worth spending a gate on.
 *
 * The mixed position is **arrived at, never chosen**. It is what a selection looks like when half
 * of it agrees, so it comes from the model; one activation takes it to `true`, which is
 * `nextPressed` in the control table and carries the same `GUESS:` it does — Word bolds a
 * half-bold selection rather than un-bolding it, and nothing here has been checked against Office.
 *
 * ## A `<button role="checkbox">`, not an `<input type="checkbox">`
 *
 * A native checkbox cannot carry the third position as an attribute at all: `indeterminate` is a
 * DOM property with no attribute, so it survives no round trip through markup, and the platform
 * still reports the element as checked or unchecked underneath it. `role="checkbox"` on a real
 * `<button>` keeps Space, the disabled semantics and the focus behaviour the platform already does
 * correctly, and makes all three positions one attribute a test can read and a person can write.
 *
 * ## The row hovers; the box does not
 *
 * See `boxStates` in the model. One hover treatment for one hover.
 */

import { defineIcon } from '../icons/icon.ts';
import {
  applyAvailability,
  controlClassName,
  createExplanationElement,
  emitControlEvent,
  explanationElementId,
  forcedState,
  installControlStyles,
  refusesActivation,
} from '../controls/control-element.ts';
import {
  controlBaseCss,
  controlStatesCss,
  isPressedValue,
  nextPressed,
  type PressedValue,
} from '../controls/control-states.ts';
import { boxGlyphs, checkboxSheet, inputEvents, inputTypeClass } from './input-model.ts';

/** The whole sheet, composed once and shared by every instance. */
export const checkboxCssSheet = [
  controlBaseCss,
  controlStatesCss('.control'),
  checkboxSheet,
].join('\n');

export class MjxCheckbox extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'checked',
    'disabled',
    'unavailable',
    'explanation',
    'force-state',
  ];

  #root: ShadowRoot | undefined;
  #button: HTMLButtonElement | undefined;
  #box: HTMLElement | undefined;
  #glyph: HTMLElement | undefined;
  #labelElement: HTMLElement | undefined;
  #explanationElement: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The control's name, and its visible text. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /**
   * `false`, `true` or `mixed`. Absent means `false`; a bare `checked` means `true`.
   *
   * The same three-valued accessor `<mjx-toggle-button>` has, reading the same validator, because
   * a checkbox and a toggle button are one state machine wearing two shapes.
   */
  get checked(): PressedValue {
    const declared = this.getAttribute('checked');
    if (declared === '') return 'true';
    return isPressedValue(declared) ? (declared as PressedValue) : 'false';
  }

  set checked(value: PressedValue) {
    this.setAttribute('checked', value);
  }

  /** The reported value: the position, as the one shape every input in this child reports. */
  get value(): PressedValue {
    return this.checked;
  }

  /** Focus the control this element *is*, not the host, which carries no tabindex. */
  override focus(options?: FocusOptions): void {
    if (this.#button === undefined) super.focus(options);
    else this.#button.focus(options);
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, checkboxCssSheet);

    const button = document.createElement('button');
    button.type = 'button';
    button.className = controlClassName;
    button.setAttribute('part', 'control');
    button.setAttribute('role', 'checkbox');
    button.addEventListener('click', this.#onClick);

    const box = document.createElement('span');
    box.className = 'box';
    box.setAttribute('part', 'box');
    box.setAttribute('aria-hidden', 'true');

    const glyph = document.createElement('mjx-icon');
    box.append(glyph);

    const label = document.createElement('span');
    label.className = 'label';

    const explanation = createExplanationElement(explanationElementId);

    button.append(box, label);
    root.append(button, explanation);

    this.#button = button;
    this.#box = box;
    this.#glyph = glyph;
    this.#labelElement = label;
    this.#explanationElement = explanation;
  }

  #onClick = (event: MouseEvent): void => {
    if (refusesActivation(this)) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    const previous = this.checked;
    const next = nextPressed(previous);
    // Move, then report — the same ordering `<mjx-toggle-button>` states: a listener that read
    // `event.target.checked` and got the old value would be a footgun with no upside.
    this.setAttribute('checked', next);
    emitControlEvent(this, inputEvents.change, { value: next, previous });
  };

  /** Re-render from the attributes. */
  render(): void {
    const button = this.#button;
    const box = this.#box;
    const glyph = this.#glyph;
    const labelElement = this.#labelElement;
    const explanation = this.#explanationElement;
    if (
      button === undefined ||
      box === undefined ||
      glyph === undefined ||
      labelElement === undefined ||
      explanation === undefined
    ) {
      return;
    }

    const checked = this.checked;
    // The announcement and the paint, written together from one value so they cannot drift.
    // `aria-checked` carries all three positions — `mixed` is *this attribute's value*, and a
    // component that reached for a class here would announce a two-state control.
    button.setAttribute('aria-checked', checked);
    button.dataset['pressed'] = checked;

    const drawing = checked === 'true' ? boxGlyphs.checked : checked === 'mixed' ? boxGlyphs.mixed : undefined;
    if (drawing === undefined) {
      glyph.removeAttribute('name');
      glyph.hidden = true;
    } else {
      glyph.hidden = false;
      glyph.setAttribute('name', drawing.name);
      glyph.setAttribute('size', String(drawing.size));
      glyph.setAttribute('variant', 'regular');
    }

    labelElement.textContent = this.label;
    labelElement.className = `label ${inputTypeClass('fieldValue')}`;

    const forced = forcedState(this);
    if (forced === undefined) delete button.dataset['state'];
    else button.dataset['state'] = forced;

    applyAvailability(this, button, explanation);
  }
}

/** Register the element. Idempotent. */
export function defineCheckbox(): void {
  defineIcon();
  if (customElements.get('mjx-checkbox') === undefined) {
    customElements.define('mjx-checkbox', MjxCheckbox);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-checkbox': MjxCheckbox;
  }
}
