/**
 * `<mjx-zoom-control>` — arithmetic with correct answers, and a readout that never lies.
 *
 * ```html
 * <mjx-zoom-control percent="100" viewport-width="1000" viewport-height="700">
 * </mjx-zoom-control>
 * ```
 *
 * ## It is a slider and a numeric field, and it writes neither
 *
 * MJXOFF-186 shipped both. `<mjx-slider>` already owns the geometry, the keyboard, pointer capture
 * and the `preview`-throughout / `change`-once split that makes a live zoom possible without an
 * undo entry per pixel; `<mjx-button>` already owns two kinds of unavailable. **Do not write a
 * third slider** is the ticket's own instruction, and this file honours it: the whole component is
 * a slider, two commands, a readout and the arithmetic between them.
 *
 * ## The arithmetic has right answers, and the gates assert numbers
 *
 * `furniture-model.ts` holds all of it — `zoomSteps`, `clampZoom`, `zoomStepFrom`,
 * `fitToWidthPercent`, `fitToPagePercent`, `parseZoomPercent` — and `tests/furniture.test.ts`
 * checks each against figures rather than against a rendering. The four that matter:
 *
 * * the minus and plus commands move to the next **stop**, not by one per cent;
 * * a fit is **floored**, because a rounded-up fit overflows by a fraction of a pixel and produces
 *   the horizontal scrollbar the command exists to avoid;
 * * both bounds clamp, in both directions, and the commands disable themselves at them rather than
 *   silently doing nothing;
 * * **a typed percentage and the slider agree**, which is asserted by driving one and reading the
 *   other.
 *
 * ## The readout never silently reverts
 *
 * U07's rule, inherited whole: a string this cannot read leaves the text alone, commits nothing,
 * says so three ways at once — `aria-invalid`, a glyph and the reason written out — fires an event
 * carrying the fragment that defeated it, and stays invalid on blur. Escape is the way back.
 *
 * **Out of range is not a failure.** `700` is a number a person meant; it becomes 500 and the
 * component announces that it clamped, because a field that silently substituted a different number
 * would be the same defect wearing a nicer costume.
 *
 * ## `GUESS:` the slider is linear and Office's is not
 *
 * Word's zoom slider has a detent at 100 % in the middle of its travel, which means its mapping
 * from position to percentage is piecewise. Ours is linear over 10…500, so 100 % sits at about a
 * fifth of the track. The alternative was to give `<mjx-slider>` a value space that is not the
 * percentage — and then its own `aria-valuenow`, which is the number a screen reader reads, would
 * have been a track position rather than a zoom. A linear slider that announces the truth is worth
 * more than a shapelier one that does not, and the preset ticks are what make the scale legible.
 */

import { defineControls } from '../controls/index.ts';
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
import { defineSlider, type MjxSlider } from '../inputs/index.ts';
import { inputEvents } from '../inputs/input-model.ts';
import type { MeasureParseFailure } from '../inputs/measure.ts';
import {
  clampZoom,
  defaultZoomPercent,
  formatZoom,
  furnitureEvents,
  furnitureTags,
  furnitureTypeRoles,
  letterPagePixels,
  parseZoomPercent,
  zoomBounds,
  zoomControlCss,
  zoomFailureMessages,
  zoomFitNames,
  zoomFits,
  zoomStepFrom,
  zoomSteps,
  type FurnitureCause,
  type Rectangle,
  type ZoomFit,
} from './furniture-model.ts';

/** The sheet, composed once. */
export const zoomControlSheet = zoomControlCss;

/** The glyph an invalid readout draws. Filled, because a status mark reads at a glance when solid. */
export const zoomInvalidIcon = { name: 'warning', size: 16, variant: 'filled' } as const;

/** The two stepping commands, and the glyph each draws. */
export const zoomStepIcons = { out: 'subtract', in: 'add' } as const;

let nextZoomSerial = 0;

export class MjxZoomControl extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'percent',
    'viewport-width',
    'viewport-height',
    'page-width',
    'page-height',
    'disabled',
    'unavailable',
    'explanation',
  ];

  #root: ShadowRoot | undefined;
  #slider: MjxSlider | undefined;
  #out: HTMLElement | undefined;
  #in: HTMLElement | undefined;
  #field: HTMLElement | undefined;
  #entry: HTMLInputElement | undefined;
  #glyph: HTMLElement | undefined;
  #message: HTMLElement | undefined;
  #fits = new Map<ZoomFit, HTMLElement>();
  #explanationElement: HTMLElement | undefined;
  #invalid: { failure: MeasureParseFailure; offending: string } | undefined;
  /** True while the person's text is theirs. Cleared by every path that finishes. */
  #typing = false;
  readonly #id = `mjx-zoom-${String((nextZoomSerial += 1))}`;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(name: string): void {
    // A zoom set from outside is a new zoom, and it ends any editing in flight — otherwise a host
    // that fitted the page while a person was typing would be ignored.
    if (name === 'percent') {
      this.#typing = false;
      this.#invalid = undefined;
    }
    if (this.#root !== undefined) this.render();
  }

  /** The zoom, clamped on the way out, so `percent` is never a number the slider is not at. */
  get percent(): number {
    const declared = this.getAttribute('percent');
    if (declared === null || declared.trim() === '') return defaultZoomPercent;
    const parsed = Number.parseFloat(declared);
    return Number.isFinite(parsed) ? clampZoom(parsed) : defaultZoomPercent;
  }

  set percent(next: number) {
    this.setAttribute('percent', String(clampZoom(next)));
  }

  /** The room a page has to be drawn in, in CSS pixels. What the two fit commands measure against. */
  get viewport(): Rectangle {
    return {
      width: this.#number('viewport-width', 0),
      height: this.#number('viewport-height', 0),
    };
  }

  /** The page, in CSS pixels. US Letter at 96 dpi unless the shell says otherwise. */
  get page(): Rectangle {
    return {
      width: this.#number('page-width', letterPagePixels.width),
      height: this.#number('page-height', letterPagePixels.height),
    };
  }

  /** Whether the readout currently holds something it could not read. */
  get invalid(): boolean {
    return this.#invalid !== undefined;
  }

  /** The zoom either fit command would produce, without applying it. What a gate compares. */
  fit(which: ZoomFit): number {
    return zoomFits[which].compute(this.viewport, this.page);
  }

  /** The slider, so a gate measures the real thumb rather than a proxy for it. */
  get sliderElement(): MjxSlider | undefined {
    return this.#slider;
  }

  /** The readout, so a gate types into the real field. */
  get readoutElement(): HTMLInputElement | undefined {
    return this.#entry;
  }

  override focus(options?: FocusOptions): void {
    if (this.#entry === undefined) super.focus(options);
    else this.#entry.focus(options);
  }

  #number(name: string, fallback: number): number {
    const declared = Number.parseFloat(this.getAttribute(name) ?? '');
    return Number.isFinite(declared) ? declared : fallback;
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, zoomControlSheet);
    defineIcon();
    defineControls();
    defineSlider();

    const commands = document.createElement('div');
    commands.className = 'commands';

    const out = document.createElement('mjx-button');
    out.setAttribute('label', 'Zoom out');
    out.setAttribute('icon', zoomStepIcons.out);
    out.setAttribute('size', 'icon');
    out.addEventListener('click', () => {
      this.#step(-1);
    });
    this.#out = out;

    const slider = document.createElement('mjx-slider') as MjxSlider;
    slider.className = 'slider';
    slider.setAttribute('label', 'Zoom');
    slider.setAttribute('min', String(zoomBounds.min));
    slider.setAttribute('max', String(zoomBounds.max));
    slider.setAttribute('step', '1');
    slider.setAttribute('suffix', '%');
    slider.setAttribute('ticks', zoomSteps.join(','));
    slider.addEventListener(inputEvents.preview, this.#onSlider as EventListener);
    slider.addEventListener(inputEvents.change, this.#onSlider as EventListener);
    this.#slider = slider;

    const zoomIn = document.createElement('mjx-button');
    zoomIn.setAttribute('label', 'Zoom in');
    zoomIn.setAttribute('icon', zoomStepIcons.in);
    zoomIn.setAttribute('size', 'icon');
    zoomIn.addEventListener('click', () => {
      this.#step(1);
    });
    this.#in = zoomIn;

    commands.append(out, slider, zoomIn);

    const field = document.createElement('div');
    field.className = `field readout ${furnitureTypeRoles.readout}`;
    field.setAttribute('part', 'readout');

    const entry = document.createElement('input');
    entry.className = 'entry';
    entry.type = 'text';
    entry.inputMode = 'decimal';
    entry.autocomplete = 'off';
    entry.spellcheck = false;
    entry.id = `${this.#id}-entry`;
    entry.setAttribute('aria-label', 'Zoom percentage');
    entry.addEventListener('input', this.#onInput);
    entry.addEventListener('keydown', this.#onKeyDown);
    entry.addEventListener('blur', this.#onBlur);
    this.#entry = entry;

    const glyph = document.createElement('mjx-icon');
    glyph.className = 'trailing';
    glyph.setAttribute('name', zoomInvalidIcon.name);
    glyph.setAttribute('size', String(zoomInvalidIcon.size));
    glyph.setAttribute('variant', zoomInvalidIcon.variant);
    glyph.setAttribute('aria-hidden', 'true');
    glyph.hidden = true;
    this.#glyph = glyph;

    field.append(entry, glyph);
    this.#field = field;

    const message = document.createElement('p');
    message.className = 'message';
    message.id = `${this.#id}-message`;
    message.hidden = true;
    this.#message = message;

    const fits = document.createElement('div');
    fits.className = 'commands';
    for (const which of zoomFitNames) {
      const command = document.createElement('mjx-button');
      command.setAttribute('label', zoomFits[which].label);
      command.setAttribute('size', 'small');
      command.dataset['fit'] = which;
      command.addEventListener('click', () => {
        this.#apply(this.fit(which), 'command');
      });
      this.#fits.set(which, command);
      fits.append(command);
    }

    const explanation = createExplanationElement(explanationElementId);
    this.#explanationElement = explanation;

    root.append(commands, field, fits, message, explanation);
  }

  // ── the paths in ───────────────────────────────────────────────────────────

  #onSlider = (event: CustomEvent<{ value: number }>): void => {
    if (refusesActivation(this)) return;
    this.#apply(clampZoom(event.detail.value), 'pointer');
  };

  #step(direction: 1 | -1): void {
    if (refusesActivation(this)) return;
    this.#apply(zoomStepFrom(this.percent, direction), 'command');
  }

  #onInput = (): void => {
    this.#typing = true;
    this.#invalid = undefined;
    this.#paintInvalid();
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    if (event.key === 'Enter') {
      event.preventDefault();
      this.#commitTyped();
      return;
    }
    if (event.key === 'Escape') {
      event.preventDefault();
      // Escape is the way out of an invalid field, and it is the only one.
      this.#typing = false;
      this.#invalid = undefined;
      this.render();
    }
  };

  #onBlur = (): void => {
    this.#commitTyped();
  };

  /**
   * Read what is in the readout and either commit it or refuse it.
   *
   * ⚠ **The text is not touched on a refusal.** MJXOFF-186's first defect was a `#finish()` that
   * cleared its typing flag before rendering, so the render overwrote what the person had typed
   * with the old value — the exact behaviour the component exists not to have, with the state set,
   * the event fired and the glyph showing. A field displaying the old value and a field that
   * committed the old value are the same picture.
   */
  #commitTyped(): void {
    const entry = this.#entry;
    if (entry === undefined) return;
    if (!this.#typing) return;

    const parsed = parseZoomPercent(entry.value);
    if (!parsed.ok) {
      this.#invalid = { failure: parsed.error.failure, offending: parsed.error.offending };
      this.dispatchEvent(
        new CustomEvent(furnitureEvents.zoomInvalid, {
          bubbles: true,
          composed: true,
          detail: {
            text: entry.value,
            failure: parsed.error.failure,
            offending: parsed.error.offending,
          },
        }),
      );
      this.#paintInvalid();
      return;
    }

    this.#typing = false;
    this.#invalid = undefined;
    this.#apply(parsed.percent, 'typed');
  }

  #apply(percent: number, cause: FurnitureCause): void {
    const previous = this.percent;
    const next = clampZoom(percent);
    this.#typing = false;
    this.#invalid = undefined;
    if (next === previous) {
      this.render();
      return;
    }
    this.setAttribute('percent', String(next));
    this.render();
    this.dispatchEvent(
      new CustomEvent(furnitureEvents.zoom, {
        bubbles: true,
        composed: true,
        detail: { percent: next, previous, cause },
      }),
    );
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render from the attributes. */
  render(): void {
    const slider = this.#slider;
    const entry = this.#entry;
    const explanation = this.#explanationElement;
    if (slider === undefined || entry === undefined || explanation === undefined) return;

    const percent = this.percent;
    slider.setAttribute('value', String(percent));
    // ⚠ **No `aria-valuetext` here, and the a11y sweep is what said so.** It belongs to a range
    // widget; the readout is a textbox, whose value *is* its accessible text, and axe reports the
    // attribute as `aria-allowed-attr`. The percentage is announced by the slider beside it, which
    // is the range widget and builds its own value text from its suffix.
    // ⚠ Only when the text is not the person's. This is the line MJXOFF-186 got wrong once.
    if (!this.#typing) entry.value = formatZoom(percent);

    const disabled = isHardDisabled(this);
    const unavailable = !disabled && isUnavailable(this);
    if (disabled || unavailable) this.dataset['disabled'] = '';
    else delete this.dataset['disabled'];

    // A command at its bound is *disabled*, not silent: a button that stopped responding reads as
    // a broken control rather than as a limit.
    this.#setAvailable(this.#out, percent > zoomBounds.min && !disabled && !unavailable);
    this.#setAvailable(this.#in, percent < zoomBounds.max && !disabled && !unavailable);
    for (const [which, command] of this.#fits) {
      const wanted = this.fit(which);
      this.#setAvailable(command, wanted !== percent && !disabled && !unavailable);
    }

    this.#paintInvalid();
    applyAvailability(this, entry, explanation);
  }

  #setAvailable(element: HTMLElement | undefined, available: boolean): void {
    if (element === undefined) return;
    if (available) element.removeAttribute('disabled');
    else element.setAttribute('disabled', '');
  }

  /**
   * Say the field is invalid, three ways at once.
   *
   * The honey edge that `fieldStates.invalid` paints is **2.07 : 1 in light** and cannot carry the
   * state on its own, which is why `aria-invalid`, the glyph and the written reason are all here
   * and all asserted by the browser gate. A declared cue that is not rendered fails louder than no
   * declaration.
   */
  #paintInvalid(): void {
    const field = this.#field;
    const entry = this.#entry;
    const glyph = this.#glyph;
    const message = this.#message;
    if (field === undefined || entry === undefined || glyph === undefined || message === undefined) {
      return;
    }
    const invalid = this.#invalid;
    // ⚠ `data-invalid`, and not `data-state="invalid"`. `fieldStates.invalid` matches
    // `%s[data-invalid]` — there is no `data-state` spelling of it — so the first version of this
    // line put the field into a state the table paints *nothing* for, and the honey edge never
    // appeared. Every one of the three non-colour cues was already correct, which is exactly why
    // the browser gate now measures the edge as well as counting the cues.
    if (invalid === undefined) delete field.dataset['invalid'];
    else field.dataset['invalid'] = '';
    entry.setAttribute('aria-invalid', invalid === undefined ? 'false' : 'true');
    glyph.hidden = invalid === undefined;
    if (invalid === undefined) {
      message.hidden = true;
      message.textContent = '';
      entry.removeAttribute('aria-describedby');
      return;
    }
    message.hidden = false;
    message.textContent = zoomFailureMessages[invalid.failure].replace(
      '{offending}',
      invalid.offending,
    );
    entry.setAttribute('aria-describedby', message.id);
  }
}

/** Register the element. Idempotent. */
export function defineZoomControl(): void {
  if (customElements.get(furnitureTags.zoomControl) === undefined) {
    customElements.define(furnitureTags.zoomControl, MjxZoomControl);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-zoom-control': MjxZoomControl;
  }
}
