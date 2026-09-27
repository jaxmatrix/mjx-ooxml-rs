/**
 * `<mjx-font-picker>` — Office's font box, and **the substitution warning is the whole reason it
 * is a component rather than a combo box full of strings.**
 *
 * ```ts
 * picker.fonts = [
 *   { family: 'Calibri', category: 'Theme fonts', availability: 'installed' },
 *   { family: 'Cambria', category: 'All fonts', availability: 'substituted',
 *     substitutedBy: 'Caladea', metricCompatible: true },
 *   { family: 'Bookshelf Symbol 7', category: 'All fonts', availability: 'missing' },
 * ];
 * ```
 *
 * ## What it exists to prevent
 *
 * `mjx-text` carries a metric-compatible substitution table and a per-document substitution
 * manifest, and both of those facts are invisible in a list of family names. A person picks
 * *Cambria*, the machine has no Cambria, something else is drawn, and — if the stand-in has
 * different metrics — every line after the first rewraps. Nothing on screen said so. A picker that
 * lists fonts without saying which of them are substituted is exactly that failure, so:
 *
 * * every row that is not installed carries a **word and a glyph**, in the row itself;
 * * the **field** carries the same glyph and a sentence under it while the chosen family is one of
 *   them, associated with `aria-describedby` so it is announced rather than merely drawn;
 * * a substituted family is **still choosable**, because a document may legitimately keep asking
 *   for a face this machine does not have and refusing would silently rewrite it.
 *
 * ## The note is told apart by size, never by colour
 *
 * `--theme-text-secondary` is 4.32 : 1 on `--theme-border-subtle`, which is what an option row
 * fills with under the keyboard cursor — so a grey note would be legible in every screenshot and
 * illegible exactly while a person pointed at it. U05 settled this for a menu's hint and U07 for
 * an option's second line; the note here is primary text at the dense size and a glyph.
 *
 * ## Each name is drawn in its own face, and every row is still one row tall
 *
 * That is U06's virtualisation precondition, violated by construction — a face decides its own
 * ascender and descender, so a row sized by its content is a different height per row and the
 * virtualiser's divide-by-row-height becomes arithmetic over a number nothing on screen has. The
 * row's height stays `--mjx-option-block-size`; `.font-name` is `block-size: 100%` with
 * `overflow: hidden` and `line-height: 1`, so a face changes the glyphs inside the box and never
 * the box. `tests/browser/pickers.spec.ts` measures every built row of a list whose faces have
 * deliberately different metrics and requires one height, and requires a heading to be exactly one
 * of them.
 *
 * ## And a substituted family is previewed in what it will *actually* be drawn in
 *
 * `fontPreviewStack` uses the stand-in rather than the family, because a browser asked for a face
 * it does not have falls back silently — so previewing the family would draw the fallback and make
 * the row look installed. The one place a preview could have lied is the one place it must not.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { MjxComboBox } from '../inputs/combo-box.ts';
import { defineListFieldDependencies } from '../inputs/list-field.ts';
import { defineIcon } from '../icons/icon.ts';
import { inputTypeClass, type OptionDescriptor } from '../inputs/input-model.ts';
import {
  fontOption,
  fontPickerCss,
  fontPreviewStack,
  pickerTypeClass,
  substitutionGlyph,
  substitutionNote,
  substitutionWord,
  type FontDescriptor,
} from './picker-model.ts';

/** The whole sheet a font picker adopts, on top of the list field's. */
export const fontPickerSheet = fontPickerCss;

/** The id of the element the field's substitution sentence lives in, within the shadow root. */
export const substitutionMessageId = 'substitution';

export class MjxFontPicker extends MjxComboBox {
  static override readonly observedAttributes: readonly string[] = [...MjxComboBox.observedAttributes];

  #fonts: readonly FontDescriptor[] = [];
  #message: HTMLElement | undefined;
  #warning: HTMLElement | undefined;

  /**
   * The families, as the font engine reports them.
   *
   * A property and never markup: this list is *every installed family*, which is four hundred
   * rows nobody types out — U07's own rule for when a list is data rather than content.
   */
  get fonts(): readonly FontDescriptor[] {
    return this.#fonts;
  }

  set fonts(next: readonly FontDescriptor[]) {
    this.#fonts = [...next];
    // ⚠ Through the base class's own `options` setter, rather than by overriding `options` with a
    // getter of our own. Overriding half of a get/set pair leaves the other half undefined, so a
    // host that assigned `picker.options` would get a TypeError from a control that looks like it
    // has the property — and the base's filter, its `displayTextFor` and its label matching all
    // read `options`, so there would then be two lists and one of them would be empty.
    //
    // The assignment re-renders, which is why `#fonts` is written first: `decorateOption` reads it
    // while the render is in flight.
    super.options = this.#fonts.map((font) => fontOption(font));
  }

  /** The descriptor for a family, or `undefined`. */
  fontFor(family: string): FontDescriptor | undefined {
    return this.#fonts.find((font) => font.family === family);
  }

  /** The chosen family's descriptor, or `undefined`. */
  get chosenFont(): FontDescriptor | undefined {
    return this.value === '' ? undefined : this.fontFor(this.value);
  }

  /** The sentence the field is currently showing, or `undefined` when it shows none. */
  get warning(): string | undefined {
    const font = this.chosenFont;
    return font === undefined ? undefined : substitutionNote(font);
  }

  override connectedCallback(): void {
    super.connectedCallback();
    const root = this.shadowRoot;
    if (root !== null) installControlStyles(root, fontPickerSheet);
    this.render();
  }

  /**
   * The row's last look: the name in its own face, and the substitution mark.
   *
   * ⚠ Both go **inside** the row's existing grid — the label in column two and the mark in column
   * three, which `optionCss` already reserves — so the row's own `block-size` still decides its
   * height. A decorator that wrapped the row, or that appended a second line, would break the
   * virtualiser's one-row-tall precondition, and the symptom would be a list that scrolls to
   * nearly the right font.
   */
  override decorateOption(row: HTMLElement, option: OptionDescriptor, _index: number): void {
    const font = this.fontFor(option.value);
    if (font === undefined) return;

    const label = row.querySelector('.option-label');
    if (label instanceof HTMLElement) {
      label.classList.add('font-name');
      label.style.fontFamily = fontPreviewStack(font);
    }

    const word = substitutionWord(font);
    if (word === undefined) return;

    const mark = document.createElement('span');
    mark.className = `substitution ${pickerTypeClass('substitution')}`;
    const icon = document.createElement('mjx-icon');
    icon.setAttribute('name', substitutionGlyph.name);
    icon.setAttribute('size', String(substitutionGlyph.size));
    icon.setAttribute('aria-hidden', 'true');
    const text = document.createElement('span');
    text.textContent = word;
    mark.append(icon, text);
    row.append(mark);

    // The row is not focusable, so a description associated with it would never be visited on its
    // own — the sentence is announced as part of the row instead, exactly as an unavailable
    // option's explanation is.
    const note = substitutionNote(font);
    if (note !== undefined) {
      const announced = document.createElement('span');
      announced.className = 'visually-hidden';
      announced.textContent = `, ${note}`;
      row.append(announced);
    }
  }

  /**
   * The field's own half of the warning.
   *
   * A glyph inside the field and a sentence under it, wired with `aria-describedby` — a warning
   * that is drawn and not announced is a warning half the people who need it never receive.
   */
  protected override rendered(field: HTMLElement, entry: HTMLElement): void {
    super.rendered(field, entry);

    let warning = this.#warning;
    if (warning === undefined) {
      warning = document.createElement('mjx-icon');
      warning.className = 'field-warning';
      warning.setAttribute('name', substitutionGlyph.name);
      warning.setAttribute('size', String(substitutionGlyph.size));
      warning.setAttribute('aria-hidden', 'true');
      field.append(warning);
      this.#warning = warning;
    }

    let message = this.#message;
    if (message === undefined) {
      message = document.createElement('span');
      message.className = `substitution-message ${inputTypeClass('message')}`;
      message.id = substitutionMessageId;
      field.after(message);
      this.#message = message;
    }

    const note = this.warning;
    if (note === undefined) {
      warning.hidden = true;
      message.hidden = true;
      message.textContent = '';
      const described = entry.getAttribute('aria-describedby');
      if (described === substitutionMessageId) entry.removeAttribute('aria-describedby');
    } else {
      warning.hidden = false;
      message.hidden = false;
      message.textContent = note;
      // ⚠ Only when the base class has not already claimed it for an unavailability explanation.
      // Two writers of one attribute is one of them silently losing.
      if (!entry.hasAttribute('aria-describedby')) {
        entry.setAttribute('aria-describedby', substitutionMessageId);
      }
    }
  }
}

/** Register the element. Idempotent. */
export function defineFontPicker(): void {
  defineIcon();
  defineListFieldDependencies();
  if (customElements.get('mjx-font-picker') === undefined) {
    customElements.define('mjx-font-picker', MjxFontPicker);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-font-picker': MjxFontPicker;
  }
}
