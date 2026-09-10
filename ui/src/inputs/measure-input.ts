/**
 * `<mjx-measure-input>` — a number with a unit on it, and a stated answer for what happens when it
 * cannot be read.
 *
 * ```html
 * <mjx-measure-input label="Left indent" value="12" unit="pt" min="0" max="1584">
 * </mjx-measure-input>
 * <mjx-measure-input label="Top margin" value="72" unit="cm" decimal="comma"></mjx-measure-input>
 * ```
 *
 * ## What it does with something it cannot read — the whole point of the component
 *
 * MJXOFF-186: *"say what you do with an unparseable string rather than silently reverting."* This
 * is what it does, and every clause is asserted:
 *
 * * **The text stays.** What a person typed is theirs. It is not replaced, not trimmed, not
 *   corrected.
 * * **Nothing is committed.** `value` does not move, so no undo entry is made and no document is
 *   changed by a typo.
 * * **The field says so, three ways at once.** `aria-invalid="true"`, a warning glyph inside the
 *   box, and the reason written beneath it in primary text — because the honey border that also
 *   appears is **2.07 : 1 in light** and cannot carry the state on its own. `fieldStates.invalid`
 *   declares exactly that as its `nonColourCue` and the browser gate asserts all three are there.
 * * **An `mjx-input-invalid` event fires**, carrying the text, which of the three ways it failed,
 *   and the fragment that defeated it.
 * * **It stays invalid on blur.** Leaving a field does not fix what is in it, and a field that
 *   quietly reverted the moment focus left would lose a person's work while they were looking at
 *   something else.
 * * **Escape is the way out**, and it is the only one: back to the committed value, and the
 *   invalid state clears.
 *
 * ## The arithmetic is in `measure.ts` and is asserted as numbers
 *
 * Parsing, converting, rounding and formatting are pure functions with correct answers, tested in
 * Node against numbers rather than against a rendering. This file is the part that has a DOM in it.
 *
 * ## Stepping happens in the displayed unit
 *
 * Arrow Up in a centimetres field moves by a centimetre-sized step, not by a point. `stepMeasure`
 * says why: a step in points would move a centimetres field by 1/28th of a centimetre and show a
 * number no field should ever show.
 */

import { defineIcon } from '../icons/icon.ts';
import {
  applyAvailability,
  createExplanationElement,
  explanationElementId,
  installControlStyles,
  isHardDisabled,
  isUnavailable,
  refusesActivation,
} from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  fieldSheet,
  forcedFieldStateAttribute,
  inputEvents,
  inputTypeClass,
  isForcibleFieldState,
} from './input-model.ts';
import {
  clampMeasure,
  defaultMeasureUnit,
  formatMeasure,
  isDecimalSeparator,
  isMeasureUnit,
  measureFailureMessages,
  measureUnits,
  parseMeasure,
  pointsIn,
  stepMeasure,
  type DecimalSeparator,
  type MeasureParseFailure,
  type MeasureRange,
  type MeasureUnit,
} from './measure.ts';

/** The sheet, composed once. */
export const measureInputCss = fieldSheet;

/** The glyph an invalid field draws. Filled, because a status mark reads at a glance when solid. */
export const invalidIcon = { name: 'warning', size: 16, variant: 'filled' } as const;

/** How many steps a field takes when nothing says. One of whatever unit it is displaying. */
export const defaultMeasureStep = 1;

export class MjxMeasureInput extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'value',
    'unit',
    'decimal',
    'step',
    'min',
    'max',
    'placeholder',
    'disabled',
    'unavailable',
    'explanation',
    forcedFieldStateAttribute,
  ];

  #root: ShadowRoot | undefined;
  #field: HTMLElement | undefined;
  #input: HTMLInputElement | undefined;
  #glyph: HTMLElement | undefined;
  #message: HTMLElement | undefined;
  #explanationElement: HTMLElement | undefined;
  #invalid: { failure: MeasureParseFailure; offending: string } | undefined;
  /** True while the person's text is theirs. Cleared by every path that finishes. */
  #typing = false;
  static #sequence = 0;
  readonly #id = `mjx-measure-${String((MjxMeasureInput.#sequence += 1))}`;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(name: string): void {
    // A change to `value` from outside is a new value, and it ends any editing in flight —
    // otherwise a host that set the value while a person was typing would be ignored.
    if (name === 'value') {
      this.#typing = false;
      this.#invalid = undefined;
    }
    if (this.#root !== undefined) this.render();
  }

  /** The accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /**
   * The committed quantity, **in points**, or `undefined` when the field holds no value.
   *
   * The attribute is a bare number of points rather than a formatted string, so a host reads and
   * writes one canonical unit and the display unit is a separate, purely presentational choice.
   */
  get value(): number | undefined {
    const declared = this.getAttribute('value');
    if (declared === null || declared.trim() === '') return undefined;
    const parsed = Number.parseFloat(declared);
    return Number.isFinite(parsed) ? parsed : undefined;
  }

  set value(points: number | undefined) {
    if (points === undefined) this.removeAttribute('value');
    else this.setAttribute('value', String(points));
  }

  /** The unit the value is shown in. */
  get unit(): MeasureUnit {
    const declared = this.getAttribute('unit');
    return isMeasureUnit(declared) ? (declared as MeasureUnit) : defaultMeasureUnit;
  }

  /** How a value is written back. Parsing accepts both separators — see `measure.ts`. */
  get decimalSeparator(): DecimalSeparator {
    const declared = this.getAttribute('decimal');
    return isDecimalSeparator(declared) ? (declared as DecimalSeparator) : 'dot';
  }

  /** How far one arrow key moves, in the displayed unit. */
  get step(): number {
    const declared = Number.parseFloat(this.getAttribute('step') ?? '');
    return Number.isFinite(declared) && declared > 0 ? declared : defaultMeasureStep;
  }

  /** The range, in points. Either end may be absent. */
  get range(): MeasureRange {
    const read = (name: string): number | undefined => {
      const declared = Number.parseFloat(this.getAttribute(name) ?? '');
      return Number.isFinite(declared) ? declared : undefined;
    };
    const minimum = read('min');
    const maximum = read('max');
    return {
      ...(minimum === undefined ? {} : { minimumPoints: minimum }),
      ...(maximum === undefined ? {} : { maximumPoints: maximum }),
    };
  }

  /** Shown when the field holds no value. */
  get placeholder(): string {
    return this.getAttribute('placeholder') ?? '';
  }

  /** What the field is showing right now. */
  get text(): string {
    return this.#input?.value ?? '';
  }

  /** Whether the field is holding text it could not read. */
  get invalid(): boolean {
    return this.#invalid !== undefined;
  }

  /** Which way it failed, or `undefined`. */
  get invalidFailure(): MeasureParseFailure | undefined {
    return this.#invalid?.failure;
  }

  /** The text the committed value is displayed as. The component and the gate both read this. */
  displayText(): string {
    const points = this.value;
    if (points === undefined) return '';
    return formatMeasure(points, this.unit, this.decimalSeparator);
  }

  /** The field box, for a gate. */
  get fieldElement(): HTMLElement | undefined {
    return this.#field;
  }

  override focus(options?: FocusOptions): void {
    if (this.#input === undefined) super.focus(options);
    else this.#input.focus(options);
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, measureInputCss);

    const field = document.createElement('div');
    field.className = 'field mjx-hit-target mjx-motion-surface-settle';
    field.setAttribute('part', 'field');

    const input = document.createElement('input');
    input.type = 'text';
    // `decimal` rather than `numeric`: a soft keyboard must offer the separator, and `numeric`
    // does not. Not `type="number"`, which would refuse the unit and the comma outright.
    input.inputMode = 'decimal';
    input.autocomplete = 'off';
    input.spellcheck = false;
    input.className = `entry ${inputTypeClass('fieldValue')}`;
    input.setAttribute('part', 'entry');
    input.id = `${this.#id}-entry`;
    input.addEventListener('input', this.#onInput);
    input.addEventListener('keydown', this.#onKeyDown);
    input.addEventListener('blur', this.#onBlur);

    const glyph = document.createElement('mjx-icon');
    glyph.setAttribute('name', invalidIcon.name);
    glyph.setAttribute('size', String(invalidIcon.size));
    glyph.setAttribute('variant', invalidIcon.variant);
    glyph.setAttribute('aria-hidden', 'true');
    glyph.classList.add('trailing');
    glyph.setAttribute('part', 'invalid-mark');
    glyph.hidden = true;

    const message = document.createElement('span');
    message.className = `message ${inputTypeClass('message')}`;
    message.setAttribute('part', 'message');
    message.id = `${this.#id}-message`;
    message.hidden = true;

    const explanation = createExplanationElement(explanationElementId);

    field.append(input, glyph);
    root.append(field, message, explanation);

    this.#field = field;
    this.#input = input;
    this.#glyph = glyph;
    this.#message = message;
    this.#explanationElement = explanation;
  }

  #onInput = (): void => {
    this.#typing = true;
    // Typing is not committing, and it is not failing either: a half-typed `1` on the way to
    // `1.5 cm` is not an error, so the invalid state is cleared here and decided on finishing.
    if (this.#invalid !== undefined) {
      this.#invalid = undefined;
      this.render();
    }
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    if (refusesActivation(this)) return;
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    switch (event.key) {
      case 'Enter':
        this.#finish();
        event.preventDefault();
        return;
      case 'Escape':
        this.revert();
        event.preventDefault();
        return;
      case 'ArrowUp':
        this.#nudge(1);
        event.preventDefault();
        return;
      case 'ArrowDown':
        this.#nudge(-1);
        event.preventDefault();
        return;
      default:
        return;
    }
  };

  #onBlur = (): void => {
    this.#finish();
  };

  /**
   * Back to the committed value, and out of the invalid state. **The only way out of it.**
   *
   * Public, because a host that wants a Cancel button needs it and reaching into a shadow root for
   * it would be the coupling this catalogue's `part` and event surface exist to avoid.
   */
  revert(): void {
    this.#typing = false;
    this.#invalid = undefined;
    this.render();
  }

  /**
   * Read what is in the field, and commit it — or refuse, loudly.
   *
   * An arrow key on an unreadable field steps from the *committed* value, which is the only number
   * there is; that is why `#nudge` finishes first and then steps.
   */
  #finish(): void {
    if (!this.#typing) return;
    const typed = this.text;
    this.#typing = false;

    if (typed.trim() === '' && this.value === undefined) {
      this.#invalid = undefined;
      this.render();
      return;
    }

    const parse = parseMeasure(typed, this.unit);
    if (!parse.ok) {
      // ⚠ The text is **not** touched, and `value` is **not** touched. See the module note.
      this.#invalid = { failure: parse.error.failure, offending: parse.error.offending };
      this.render();
      this.dispatchEvent(
        new CustomEvent(inputEvents.invalid, {
          bubbles: true,
          composed: true,
          detail: { text: typed, failure: parse.error.failure, offending: parse.error.offending },
        }),
      );
      return;
    }

    this.#invalid = undefined;
    // Clamping is not refusing: a range is the field's own promise about what it accepts, and a
    // person who typed 2000 pt into a field that stops at 1584 meant *as much as it takes*.
    this.#commit(clampMeasure(parse.measure.points, this.range));
  }

  #nudge(direction: 1 | -1): void {
    this.#finish();
    const from = this.value;
    if (from === undefined) {
      // Nothing to step from: the first arrow key lands on the range's own start, or on zero.
      this.#commit(clampMeasure(this.range.minimumPoints ?? 0, this.range));
      return;
    }
    this.#commit(stepMeasure(from, this.unit, this.step, direction, this.range));
  }

  #commit(points: number): void {
    const previous = this.value;
    if (previous !== undefined && Math.abs(previous - points) < Number.EPSILON) {
      this.render();
      return;
    }
    this.setAttribute('value', String(points));
    this.render();
    this.dispatchEvent(
      new CustomEvent(inputEvents.change, {
        bubbles: true,
        composed: true,
        detail: {
          value: points,
          previous,
          unit: this.unit,
          magnitude: pointsIn(points, this.unit),
        },
      }),
    );
  }

  /** Re-render from the attributes. */
  render(): void {
    const field = this.#field;
    const input = this.#input;
    const glyph = this.#glyph;
    const message = this.#message;
    const explanation = this.#explanationElement;
    if (
      field === undefined ||
      input === undefined ||
      glyph === undefined ||
      message === undefined ||
      explanation === undefined
    ) {
      return;
    }

    const invalid = this.#invalid;
    input.setAttribute('aria-label', this.label);
    input.placeholder = this.placeholder;
    /*
     * ⚠ **`#invalid` as well as `#typing`, and the second half is a defect this child shipped for
     * about an hour.** `#finish()` clears `#typing` before it renders, so a render that only
     * checked that flag overwrote `banana` with `12 pt` — which is precisely the silent revert the
     * whole component exists not to do. It looked completely correct: the state was set, the event
     * fired, the glyph appeared, and the text a person typed was gone.
     *
     * Nothing but a gate would have caught it, because a field showing the old value and a field
     * that committed the old value are the same picture. `tests/browser/inputs.spec.ts` asserts
     * the text is still `banana` after Tab and still `banana` after clicking away.
     */
    if (!this.#typing && invalid === undefined) input.value = this.displayText();

    input.setAttribute('aria-invalid', String(invalid !== undefined));
    glyph.hidden = invalid === undefined;
    message.hidden = invalid === undefined;
    if (invalid === undefined) {
      message.textContent = '';
      input.removeAttribute('aria-errormessage');
      delete field.dataset['invalid'];
    } else {
      message.textContent = measureFailureMessages[invalid.failure]
        .replace('{offending}', invalid.offending)
        .replace('{units}', Object.keys(measureUnits).join(', '));
      // `aria-describedby` as well as `aria-errormessage`: the second is the correct relationship
      // and the first is the one every screen reader actually announces today.
      input.setAttribute('aria-errormessage', message.id);
      input.setAttribute('aria-describedby', message.id);
      field.dataset['invalid'] = '';
    }

    const disabled = isHardDisabled(this);
    const unavailable = !disabled && isUnavailable(this);
    if (disabled) field.dataset['disabled'] = '';
    else delete field.dataset['disabled'];
    if (unavailable) field.dataset['unavailable'] = '';
    else delete field.dataset['unavailable'];

    const forced = this.getAttribute(forcedFieldStateAttribute);
    delete field.dataset['state'];
    delete field.dataset['editing'];
    if (invalid === undefined) delete field.dataset['invalid'];
    if (isForcibleFieldState(forced)) {
      if (forced === 'hover') field.dataset['state'] = 'hover';
      else field.dataset[forced] = '';
    }

    applyAvailability(this, input, explanation);
  }
}

/** Register the element. Idempotent. */
export function defineMeasureInput(): void {
  defineIcon();
  if (customElements.get('mjx-measure-input') === undefined) {
    customElements.define('mjx-measure-input', MjxMeasureInput);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-measure-input': MjxMeasureInput;
  }
}
