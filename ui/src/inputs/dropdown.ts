/**
 * `<mjx-dropdown>` — choose one of a fixed set. Office's *Font Colour*, *Line Spacing*, *Style*.
 *
 * ```html
 * <mjx-dropdown label="Line spacing" value="1.15">
 *   <mjx-option value="1.0" label="Single"></mjx-option>
 *   <mjx-option value="1.15" label="1.15"></mjx-option>
 * </mjx-dropdown>
 * ```
 *
 * ## A listbox, and the keyboard contract that follows from saying so
 *
 * `list-surface.ts` states why this is not a menu. What follows here is the half a person feels:
 *
 * | Key | What it does | Why |
 * |---|---|---|
 * | `Arrow Down` / `Arrow Up` | opens the list, then moves through it | the native `<select>` contract |
 * | `Home` / `End` | first and last option | **not** caret keys: this field is not a text box |
 * | `Enter` / `Space` | commits the option the keyboard is on | |
 * | `Escape` | closes without committing | a listbox reverts; a menu merely closes |
 * | a printable character | opens the list and starts a type-ahead | |
 * | `Tab` | leaves, and the value does not change | one tab stop, so `Tab` may mean leave |
 *
 * `Home` and `End` are the row of that table that matters, because it is the one the combo box
 * inverts — see `ListboxKeyContext.editable` in the model, which is the single flag both read.
 *
 * ## Type-ahead opens the list rather than changing the value
 *
 * A native `<select>` changes its value on a keystroke while closed, which fires a change event
 * per letter — three of them for "Cam" — and each one is an undo entry in an editor. So a printable
 * key opens the list and moves the cursor, and the value moves when the person says so. That is a
 * deliberate divergence from the platform control and it is the one this catalogue makes on
 * purpose.
 */

import {
  MjxListField,
  defineListFieldDependencies,
  listFieldCss,
} from './list-field.ts';
import { inputTypeClass } from './input-model.ts';

export { listFieldCss as dropdownCss };

export class MjxDropdown extends MjxListField {
  static override readonly observedAttributes: readonly string[] = [
    ...MjxListField.observedAttributes,
  ];

  #text: HTMLElement | undefined;

  /** A dropdown is chosen from, never typed into. */
  protected override get editable(): boolean {
    return false;
  }

  /**
   * A real `<button>`, wearing `role="combobox"`.
   *
   * The platform already gives a button its focus behaviour, its disabled semantics and its
   * activation on Space — and a `<div tabindex="0">` would be a re-implementation of three
   * behaviours to avoid one element, which is the argument `<mjx-button>` makes and this inherits.
   * The role override is what turns *press this* into *choose from this*, and it is what makes the
   * announcement "combo box, collapsed" rather than "button".
   */
  protected override createEntry(): HTMLElement {
    const button = document.createElement('button');
    button.type = 'button';
    const text = document.createElement('span');
    text.className = `value ${inputTypeClass('fieldValue')}`;
    button.append(text);
    this.#text = text;
    return button;
  }

  protected override entryText(): string {
    return this.#text?.textContent ?? '';
  }

  protected override setEntryText(text: string): void {
    const element = this.#text;
    if (element === undefined) return;
    const shown = text === '' ? this.placeholder : text;
    element.textContent = shown;
    // The placeholder is a *rendering* of "no value", not a value. It is marked so the field's
    // own gate can tell the two apart, and so a stylesheet could grey it without greying a value.
    if (text === '' && this.placeholder !== '') element.dataset['placeholder'] = '';
    else delete element.dataset['placeholder'];
  }
}

/** Register the element. Idempotent. */
export function defineDropdown(): void {
  defineListFieldDependencies();
  if (customElements.get('mjx-dropdown') === undefined) {
    customElements.define('mjx-dropdown', MjxDropdown);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-dropdown': MjxDropdown;
  }
}
