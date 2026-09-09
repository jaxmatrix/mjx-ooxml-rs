/**
 * `<mjx-combo-box>` — a text field **and** a listbox, which is the control this child was written
 * to get right.
 *
 * ```html
 * <mjx-combo-box label="Font" value="cambria" filter="contains" allow-custom>
 *   <mjx-option value="cambria" label="Cambria"></mjx-option>
 * </mjx-combo-box>
 * ```
 *
 * ## The failure this component exists to avoid
 *
 * MJXOFF-186 names it, and the reason it is worth a component's whole design is that it is silent:
 *
 * > A combo box is an editable text field *and* a listbox, and the failure is silent: typing
 * > filters, arrows move through the filtered set, Enter commits, Escape reverts to the text
 * > before opening — **and the value the field shows must be the value the control reports.**
 * > Assert those are the same thing after every path.
 *
 * The invariant is kept by construction rather than by care: `value` is the only state, the
 * displayed text is `displayTextFor(options, value, allowCustom)` and nothing else, and the *only*
 * time the two are allowed to differ is while a person is actively typing — which is exactly what
 * `#typing` marks and what every path below ends by clearing. `tests/browser/inputs.spec.ts`
 * asserts `entry.value === displayText()` after nine different ways of finishing, and the model's
 * `displayTextFor` is what both sides read.
 *
 * ## What each way of finishing does
 *
 * | Path | Value | Text |
 * |---|---|---|
 * | `Enter` with the keyboard on an option | that option | that option's label |
 * | `Enter` with nothing active | whatever the typed text resolves to | the resolution |
 * | `Escape` while the list is open | unchanged | **the text as it was when the list opened** |
 * | `Escape` while the list is closed | unchanged | the committed value's label |
 * | `Tab`, blur, an outside click | whatever the typed text resolves to | the resolution |
 * | a press on an option | that option | that option's label |
 *
 * "Whatever the typed text resolves to" is one function, `#resolveTyped`, and it has exactly three
 * answers: an option whose label matches, case-insensitively and after trimming; the text itself,
 * when `allow-custom` is set; or **nothing**, in which case the field goes back to the committed
 * value *and says so* with an `mjx-input-invalid` event carrying what was typed. That last case is
 * a revert, and it is deliberately not a silent one — a host that wants to tell a person why can,
 * and a host that does not still gets a field whose text and value agree.
 *
 * ## Escape's two meanings, and why the first one needs a recording
 *
 * *"Reverts to the text before opening"* is not the same as *"reverts to the value"*: a person who
 * had typed `Cam`, opened the list by arrowing, and then pressed Escape expects `Cam` back, not
 * the font they had before. So `opening()` records the text and `closed('escape')` restores it.
 * Recording the *value* instead would be the same code with a different variable and would be
 * wrong in exactly one case, which is the case the requirement names.
 */

import { defineIcon } from '../icons/icon.ts';
import {
  MjxListField,
  defineListFieldDependencies,
  listFieldCss,
} from './list-field.ts';
import {
  filterOptions,
  inputEvents,
  isFilterMode,
  normaliseForMatch,
  type FilterMode,
  type ListCloseReason,
  type ListboxAction,
  type OptionDescriptor,
} from './input-model.ts';

export { listFieldCss as comboBoxCss };

/** How a combo box filters when it does not say. */
export const defaultFilterMode: FilterMode = 'contains';

export class MjxComboBox extends MjxListField {
  static override readonly observedAttributes: readonly string[] = [
    ...MjxListField.observedAttributes,
    'filter',
    'allow-custom',
  ];

  #input: HTMLInputElement | undefined;
  /** True while the person's text is theirs and not ours to overwrite. */
  #typing = false;
  /** The text the field held when the list opened, which is what Escape restores. */
  #textOnOpen = '';

  /** A combo box is typed into. This is the flag that inverts Home and End. */
  protected override get editable(): boolean {
    return true;
  }

  /** `startsWith` or `contains`. */
  get filterMode(): FilterMode {
    const declared = this.getAttribute('filter');
    return isFilterMode(declared) ? (declared as FilterMode) : defaultFilterMode;
  }

  /** The text the field is showing right now. May differ from the value only while typing. */
  get text(): string {
    return this.#input?.value ?? '';
  }

  /** Whether the person is in the middle of typing something not yet resolved. */
  get typing(): boolean {
    return this.#typing;
  }

  /** The options the list is showing — the filter applied to the whole set. */
  override get visibleOptions(): readonly OptionDescriptor[] {
    if (!this.#typing) return this.options;
    return filterOptions(this.options, this.text, this.filterMode);
  }

  protected override createEntry(): HTMLInputElement {
    const input = document.createElement('input');
    input.type = 'text';
    // `off` on all three: a browser's own autofill panel over a combo box's own list is two lists
    // for one field, and the browser's covers ours.
    input.autocomplete = 'off';
    input.spellcheck = false;
    input.setAttribute('autocapitalize', 'off');
    input.addEventListener('input', this.#onInput);
    this.#input = input;
    return input;
  }

  protected override entryText(): string {
    return this.#input?.value ?? '';
  }

  protected override setEntryText(text: string): void {
    if (this.#input === undefined) return;
    this.#input.value = text;
  }

  /**
   * ⚠ The one override of `syncText`, and it adds a condition rather than a computation.
   *
   * While a person is typing, their text is theirs. Every path that finishes clears `#typing`
   * first and then calls this, so the field always ends up agreeing with the value.
   */
  protected override syncText(): void {
    if (this.#typing) return;
    super.syncText();
  }

  protected override rendered(_field: HTMLElement, entry: HTMLElement): void {
    if (entry instanceof HTMLInputElement) entry.placeholder = this.placeholder;
  }

  /** A press on the field puts the caret where it landed; only the chevron opens the list. */
  protected override pointerOpensList(event: PointerEvent): boolean {
    const disclosure = this.disclosureElement;
    if (disclosure === undefined) return false;
    return event.composedPath().includes(disclosure);
  }

  protected override opening(): void {
    this.#textOnOpen = this.text;
  }

  protected override closed(reason: ListCloseReason): void {
    if (reason !== 'escape') return;
    // Escape while open: the text as it was when the list opened, and the typing state with it —
    // a person whose `Cam` came back is still typing `Cam`.
    this.#typing = this.#textOnOpen !== this.displayText();
    this.setEntryText(this.#textOnOpen);
  }

  protected override leaving(): void {
    this.#finishFromText();
  }

  protected override initialActiveIndex(): number {
    const options = this.visibleOptions;
    if (options.length === 0) return -1;
    if (this.#typing) return 0;
    const chosen = options.findIndex((option) => option.value === this.value);
    return chosen >= 0 ? chosen : 0;
  }

  protected override handleAction(action: ListboxAction, event: KeyboardEvent): 'handled' | 'passed' {
    switch (action) {
      case 'commit': {
        if (this.open && this.commitActive()) {
          this.#typing = false;
          this.syncText();
          this.closeList('commit');
          return 'handled';
        }
        this.#finishFromText();
        if (this.open) this.closeList('commit');
        return 'handled';
      }
      case 'revert':
        // Escape with the list already closed: back to the committed value.
        this.#typing = false;
        this.syncText();
        return 'handled';
      default:
        return super.handleAction(action, event);
    }
  }

  #onInput = (): void => {
    this.#typing = true;
    const options = this.visibleOptions;
    const surface = this.surface;
    if (!this.open) this.openList();
    if (surface !== undefined) {
      surface.setOptions(options);
      // The first match becomes the cursor, or nothing does when there are none. Filtering is not
      // a value being *tried*, so no preview is emitted here — a preview per keystroke would make
      // a live-preview host repaint the document on every letter.
      surface.setActive(options.length > 0 ? 0 : -1);
    }
    this.render();
  };

  /**
   * What the typed text resolves to, and what to do about it.
   *
   * Three answers and no fourth: a matching option, the text itself under `allow-custom`, or a
   * refusal that reverts *and reports*. `normaliseForMatch` is the model's own comparison, so
   * "cambria", " Cambria " and "CAMBRIA" are one answer.
   */
  #finishFromText(): void {
    const typed = this.text;
    if (!this.#typing) {
      this.syncText();
      return;
    }
    this.#typing = false;

    const wanted = normaliseForMatch(typed);
    if (wanted === '' && !this.allowCustom) {
      // An emptied field means *no choice*, which is a value a combo box may hold.
      this.commitValue('');
      this.syncText();
      return;
    }

    const match = this.options.find(
      (option) => normaliseForMatch(option.label) === wanted && option.unavailable !== true,
    );
    if (match !== undefined) {
      this.commitValue(match.value);
      this.syncText();
      return;
    }

    if (this.allowCustom) {
      this.commitValue(typed.trim());
      this.syncText();
      return;
    }

    // No option, and custom values are not allowed. The field goes back to what it reports — and
    // says what it refused, so a host may explain it. Reverting *silently* is the behaviour this
    // child was told not to have.
    this.syncText();
    this.dispatchEvent(
      new CustomEvent(inputEvents.invalid, {
        bubbles: true,
        composed: true,
        detail: { text: typed, failure: 'noSuchOption', offending: typed },
      }),
    );
  }

  /** Choose an option with the pointer: the same finish as Enter on an active option. */
  override chose(option: OptionDescriptor, index: number): void {
    this.#typing = false;
    super.chose(option, index);
    this.syncText();
  }
}

/** Register the element. Idempotent. */
export function defineComboBox(): void {
  defineIcon();
  defineListFieldDependencies();
  if (customElements.get('mjx-combo-box') === undefined) {
    customElements.define('mjx-combo-box', MjxComboBox);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-combo-box': MjxComboBox;
  }
}
