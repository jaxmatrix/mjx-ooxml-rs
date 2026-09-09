/**
 * `<mjx-segmented-control>` — one choice out of two to five, all of them visible.
 *
 * ```html
 * <mjx-segmented-control label="Alignment" value="left">
 *   <mjx-segment value="left" label="Left" icon="text-align-left"></mjx-segment>
 *   <mjx-segment value="center" label="Centre" icon="text-align-center"></mjx-segment>
 * </mjx-segmented-control>
 * ```
 *
 * ## A radio group, and therefore a roving tab stop
 *
 * This is the one control in this child that does **not** use `aria-activedescendant`, and the
 * reason is the pattern rather than convenience: a radio group's members are real, individually
 * focusable controls, and ARIA's radio-group pattern moves focus between them. It still has
 * **one** tab stop — exactly one segment carries `tabindex="0"` — so U05's rule is kept and `Tab`
 * still means *leave*. `inputFocusPattern` in the model records which of the two mechanisms each
 * control uses, and the browser gate counts the stops rather than believing the table.
 *
 * The arrow keys **select as they move**, which is what a radio group does and is deliberately
 * unlike the two list controls: there is no popup to commit from, so a selection a person has
 * arrowed onto is a selection they have made.
 *
 * ## Zero new paint
 *
 * A segment is a toggle button and is painted by `controlStatesCss` verbatim, through the same
 * `data-pressed` attribute `<mjx-toggle-button>` writes. `tests/inputs.test.ts` asserts the
 * fingerprints are **identical** to the toggle's, which is a stronger claim than "they look
 * similar" and is the one that catches a segmented control quietly acquiring its own green.
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
  inputEvents,
  inputTags,
  inputTypeClass,
  segmentedSheet,
} from './input-model.ts';
import { optionDescriptorsIn, optionsChangedEvent } from './descriptors.ts';
import type { OptionDescriptor } from './input-model.ts';

/** The sheet, composed once in the model so a Node test can read it. */
export const segmentedControlCss = segmentedSheet;

/** The glyph size a segment draws at. The small ribbon shape's, because that is what this is. */
export const segmentIconSize = 16;

export class MjxSegmentedControl extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'value',
    'disabled',
    'unavailable',
    'explanation',
  ];

  #root: ShadowRoot | undefined;
  #group: HTMLElement | undefined;
  #explanationElement: HTMLElement | undefined;
  #declared: readonly OptionDescriptor[] | undefined;
  #light: readonly OptionDescriptor[] = [];
  #buttons: HTMLButtonElement[] = [];
  #activeIndex = 0;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.#readLight();
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The group's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /** The chosen segment's value. */
  get value(): string {
    return this.getAttribute('value') ?? '';
  }

  set value(next: string) {
    this.setAttribute('value', next);
  }

  /** The segments, as data. The property wins over the descriptors, as everywhere in this child. */
  get segments(): readonly OptionDescriptor[] {
    return this.#declared ?? this.#light;
  }

  set segments(next: readonly OptionDescriptor[]) {
    this.#declared = [...next];
    this.render();
  }

  /** Which segment holds the roving tab stop. */
  get activeIndex(): number {
    return this.#activeIndex;
  }

  override focus(options?: FocusOptions): void {
    const button = this.#buttons[this.#activeIndex];
    if (button === undefined) super.focus(options);
    else button.focus(options);
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, segmentedControlCss);

    const group = document.createElement('div');
    group.className = 'group';
    group.setAttribute('part', 'group');
    group.setAttribute('role', 'radiogroup');
    group.addEventListener('keydown', this.#onKeyDown);

    const explanation = createExplanationElement(explanationElementId);
    root.append(group, explanation);

    const slot = document.createElement('slot');
    slot.hidden = true;
    slot.addEventListener('slotchange', this.#onSlotChange);
    root.append(slot);
    this.addEventListener(optionsChangedEvent, this.#onSlotChange);

    this.#group = group;
    this.#explanationElement = explanation;
  }

  #onSlotChange = (): void => {
    this.#readLight();
    this.render();
  };

  #readLight(): void {
    this.#light = optionDescriptorsIn(this, inputTags.segment);
  }

  /**
   * The arrow keys move **and choose**, wrapping at both ends.
   *
   * A radio group whose arrows only moved focus would need a second key to choose, which is the
   * shape of a listbox and not of this. Both axes are bound, because a segmented control is
   * horizontal and a person's fingers do not always know that.
   */
  #onKeyDown = (event: KeyboardEvent): void => {
    if (refusesActivation(this)) return;
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const segments = this.segments;
    if (segments.length === 0) return;
    const rtl = getComputedStyle(this).direction === 'rtl';

    let next: number | undefined;
    switch (event.key) {
      case 'ArrowRight':
        next = this.#activeIndex + (rtl ? -1 : 1);
        break;
      case 'ArrowLeft':
        next = this.#activeIndex + (rtl ? 1 : -1);
        break;
      case 'ArrowDown':
        next = this.#activeIndex + 1;
        break;
      case 'ArrowUp':
        next = this.#activeIndex - 1;
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = segments.length - 1;
        break;
      default:
        return;
    }
    event.preventDefault();
    const count = segments.length;
    const wrapped = ((next % count) + count) % count;
    this.#choose(wrapped, { focus: true });
  };

  #choose(index: number, options: { focus: boolean }): void {
    const segment = this.segments[index];
    if (segment === undefined) return;
    if (segment.unavailable === true) {
      // Reachable and refused, exactly as an unavailable menu row is: the tab stop moves there so
      // it can be read, and nothing is chosen.
      this.#activeIndex = index;
      this.render();
      if (options.focus) this.#buttons[index]?.focus();
      return;
    }
    this.#activeIndex = index;
    const previous = this.value;
    if (segment.value !== previous) {
      this.setAttribute('value', segment.value);
      this.render();
      this.dispatchEvent(
        new CustomEvent(inputEvents.change, {
          bubbles: true,
          composed: true,
          detail: { value: segment.value, previous },
        }),
      );
    } else {
      this.render();
    }
    if (options.focus) this.#buttons[index]?.focus();
  }

  /** Re-render from the attributes. */
  render(): void {
    const group = this.#group;
    const explanation = this.#explanationElement;
    if (group === undefined || explanation === undefined) return;

    group.setAttribute('aria-label', this.label);
    const segments = this.segments;
    const chosen = segments.findIndex((segment) => segment.value === this.value);
    if (chosen >= 0) this.#activeIndex = chosen;
    else if (this.#activeIndex >= segments.length) this.#activeIndex = 0;

    const disabled = isHardDisabled(this);
    const unavailable = !disabled && isUnavailable(this);

    // Rebuilt rather than reconciled: a segmented control has two to five members and the cost of
    // getting a reconciliation subtly wrong is a stale `aria-checked`, which is worse than the
    // handful of nodes it saves.
    group.replaceChildren();
    this.#buttons = [];
    for (const [index, segment] of segments.entries()) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = `segment mjx-hit-target mjx-motion-surface-settle ${inputTypeClass('fieldValue')}`;
      button.setAttribute('part', 'segment');
      button.setAttribute('role', 'radio');
      const isChosen = segment.value === this.value;
      button.setAttribute('aria-checked', String(isChosen));
      // The same attribute `<mjx-toggle-button>` writes, so the same rules paint it. A segmented
      // control with its own `.segment[data-selected]` would be a second green.
      button.dataset['pressed'] = String(isChosen);
      button.tabIndex = index === this.#activeIndex ? 0 : -1;
      button.disabled = disabled;
      if (segment.unavailable === true || unavailable) {
        button.setAttribute('aria-disabled', 'true');
        if (segment.explanation !== undefined && segment.explanation !== '') {
          button.title = segment.explanation;
        }
      }
      button.addEventListener('click', () => {
        if (disabled || unavailable) return;
        this.#choose(index, { focus: true });
      });

      if (segment.description !== undefined && segment.description !== '') {
        button.title = segment.description;
      }

      const text = document.createElement('span');
      text.textContent = segment.label;
      button.append(text);
      group.append(button);
      this.#buttons.push(button);
    }

    applyAvailability(this, group, explanation);
  }
}

/** Register the element. Idempotent. */
export function defineSegmentedControl(): void {
  defineIcon();
  if (customElements.get('mjx-segmented-control') === undefined) {
    customElements.define('mjx-segmented-control', MjxSegmentedControl);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-segmented-control': MjxSegmentedControl;
  }
}
