/**
 * `<mjx-slider>` — the identity-value trap, in geometry.
 *
 * ```html
 * <mjx-slider label="Zoom" min="10" max="400" value="100" step="10" ticks="10,100,200,400">
 * </mjx-slider>
 * <mjx-slider label="Line spacing" min="1" max="3" value="1.15" step="0.05"></mjx-slider>
 * ```
 *
 * ## Why this component's gates are arithmetic
 *
 * MJXOFF-186 names the trap: *"a slider tested at one value exercises no arithmetic."* A slider at
 * its minimum has a thumb at the start of the track whether the fraction is computed correctly, is
 * computed as zero, or is not computed at all — so a gate that opens one story and looks at one
 * thumb proves nothing whatever. The four assertions that do prove something are:
 *
 * * the thumb's measured position matches `sliderFraction()` **at several values including both
 *   ends**, computed in Node from the min, max and value rather than read back from the custom
 *   property this component itself wrote;
 * * a keyboard step moves the value by exactly `step`;
 * * `Home` and `End` land on the bounds **exactly**, including when the maximum is not on a step
 *   boundary;
 * * a value that is not on a step boundary is snapped the way `snapToStep()` says.
 *
 * All four are in `tests/inputs.test.ts` as numbers and in `tests/browser/inputs.spec.ts` as
 * measurements, and the browser one compares against the model rather than against itself.
 *
 * ## `role="slider"` on a focusable element, and the three values it must carry
 *
 * `aria-valuenow`, `aria-valuemin` and `aria-valuemax` are the whole announcement, and
 * `aria-valuetext` is what turns `100` into `100 %`. A slider that carried only `valuenow` would
 * be announced as a bare number with no sense of where in its range it is.
 *
 * ## The rail is dragged, not just clicked
 *
 * Pointer capture, so a drag that leaves the track keeps working — a slider you lose the moment
 * your hand strays off the rail is the commonest way this control is got wrong. `preview` fires
 * throughout the drag and `change` fires once at the end, which is what makes a live zoom possible
 * without an undo entry per pixel.
 */

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
  applySliderAction,
  decimalPlaces,
  defaultPageStep,
  inputBoxProperties,
  inputEvents,
  inputTypeClass,
  isSliderOrientation,
  roundTo,
  sliderCss,
  sliderFraction,
  sliderKeyAction,
  snapToStep,
  valueAtFraction,
  type SliderOrientation,
  type SliderRange,
} from './input-model.ts';

/** The sheet, composed once. */
export const sliderSheet = sliderCss;

/** What a slider's range is when it says nothing. A percentage, because most of them are. */
export const sliderDefaults = { min: 0, max: 100, step: 1 } as const;

export class MjxSlider extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'value',
    'min',
    'max',
    'step',
    'page-step',
    'ticks',
    'tick-labels',
    'suffix',
    'orientation',
    'disabled',
    'unavailable',
    'explanation',
  ];

  #root: ShadowRoot | undefined;
  #slider: HTMLElement | undefined;
  #track: HTMLElement | undefined;
  #rail: HTMLElement | undefined;
  #thumb: HTMLElement | undefined;
  #ticks: HTMLElement | undefined;
  #tickLabels: HTMLElement | undefined;
  #explanationElement: HTMLElement | undefined;
  #dragging = false;
  #valueAtDragStart: number | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  #number(name: string, fallback: number): number {
    const declared = Number.parseFloat(this.getAttribute(name) ?? '');
    return Number.isFinite(declared) ? declared : fallback;
  }

  /** The range and the granularity, in one object, because every function here takes all three. */
  get range(): SliderRange {
    const min = this.#number('min', sliderDefaults.min);
    const max = this.#number('max', sliderDefaults.max);
    const step = this.#number('step', sliderDefaults.step);
    return { min, max: Math.max(max, min), step: step > 0 ? step : sliderDefaults.step };
  }

  /**
   * The value, **snapped and clamped on the way out**.
   *
   * So an attribute a host wrote off a boundary reports as the value the slider is actually at,
   * and `value` is never a number the thumb is not on. That is the invariant that makes the
   * browser gate's comparison meaningful: it measures a position and compares it against
   * `sliderFraction(value, …)`, and if `value` could be a number the thumb was not at, the
   * comparison would be measuring the component against a fiction.
   */
  get value(): number {
    const range = this.range;
    const declared = this.getAttribute('value');
    if (declared === null || declared.trim() === '') return range.min;
    const parsed = Number.parseFloat(declared);
    if (!Number.isFinite(parsed)) return range.min;
    return snapToStep(parsed, range);
  }

  set value(next: number) {
    this.setAttribute('value', String(next));
  }

  /** How far Page Up and Page Down move. A tenth of the range unless the author says. */
  get pageStep(): number {
    const declared = Number.parseFloat(this.getAttribute('page-step') ?? '');
    if (Number.isFinite(declared) && declared > 0) return declared;
    return defaultPageStep(this.range);
  }

  /** `horizontal` or `vertical`. */
  get orientation(): SliderOrientation {
    const declared = this.getAttribute('orientation');
    return isSliderOrientation(declared) ? (declared as SliderOrientation) : 'horizontal';
  }

  /** What follows the number when it is announced. `%`, `pt`, `×`. */
  get suffix(): string {
    return this.getAttribute('suffix') ?? '';
  }

  /** The values a tick is drawn at. Empty when the slider draws none. */
  get ticks(): number[] {
    const declared = this.getAttribute('ticks');
    if (declared === null || declared.trim() === '') return [];
    return declared
      .split(',')
      .map((piece) => Number.parseFloat(piece.trim()))
      .filter((number) => Number.isFinite(number));
  }

  /** Which ticks carry a number. Every tick when the attribute is absent. */
  get labelledTicks(): number[] {
    const declared = this.getAttribute('tick-labels');
    if (declared === null) return this.ticks;
    if (declared.trim() === '') return [];
    return declared
      .split(',')
      .map((piece) => Number.parseFloat(piece.trim()))
      .filter((number) => Number.isFinite(number));
  }

  /** Where the thumb is, as a fraction of the track. What the browser gate compares against. */
  get fraction(): number {
    const range = this.range;
    return sliderFraction(this.value, range.min, range.max);
  }

  /** How the value is announced. */
  get valueText(): string {
    const places = decimalPlaces(this.range.step);
    const number = roundTo(this.value, places).toFixed(places);
    const trimmed = places > 0 ? number.replace(/\.?0+$/, '') : number;
    return this.suffix === '' ? trimmed : `${trimmed} ${this.suffix}`;
  }

  /** The writing direction, which decides whether the inline arrows are mirrored. */
  get direction(): 'ltr' | 'rtl' {
    return getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  /** The track, which is the focusable element and the thing a gate measures against. */
  get trackElement(): HTMLElement | undefined {
    return this.#track;
  }

  /** The thumb, which is the thing a gate measures. */
  get thumbElement(): HTMLElement | undefined {
    return this.#thumb;
  }

  override focus(options?: FocusOptions): void {
    if (this.#track === undefined) super.focus(options);
    else this.#track.focus(options);
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, sliderSheet);

    const slider = document.createElement('div');
    slider.className = 'slider';
    slider.setAttribute('part', 'slider');

    const track = document.createElement('div');
    track.className = 'track mjx-hit-target mjx-focus-ring';
    track.setAttribute('part', 'track');
    track.setAttribute('role', 'slider');
    track.tabIndex = 0;
    track.addEventListener('keydown', this.#onKeyDown);
    track.addEventListener('pointerdown', this.#onPointerDown);
    track.addEventListener('pointermove', this.#onPointerMove);
    track.addEventListener('pointerup', this.#onPointerUp);
    track.addEventListener('pointercancel', this.#onPointerUp);

    const rail = document.createElement('div');
    rail.className = 'rail';
    rail.setAttribute('part', 'rail');

    const fill = document.createElement('div');
    fill.className = `fill ${'mjx-motion-surface-settle'}`;
    fill.setAttribute('part', 'fill');

    const thumb = document.createElement('div');
    thumb.className = 'thumb mjx-motion-surface-settle';
    thumb.setAttribute('part', 'thumb');

    const ticks = document.createElement('div');
    ticks.className = 'ticks';
    ticks.setAttribute('aria-hidden', 'true');

    const tickLabels = document.createElement('div');
    tickLabels.className = `tick-labels ${inputTypeClass('tickLabel')}`;
    tickLabels.setAttribute('aria-hidden', 'true');

    const explanation = createExplanationElement(explanationElementId);

    rail.append(fill, thumb);
    track.append(rail);
    slider.append(track, ticks, tickLabels);
    root.append(slider, explanation);

    this.#slider = slider;
    this.#track = track;
    this.#rail = rail;
    this.#thumb = thumb;
    this.#ticks = ticks;
    this.#tickLabels = tickLabels;
    this.#explanationElement = explanation;
  }

  // ── the keyboard ───────────────────────────────────────────────────────────

  #onKeyDown = (event: KeyboardEvent): void => {
    if (refusesActivation(this)) return;
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    const action = sliderKeyAction(event.key, this.direction, this.orientation);
    if (action === undefined) return;
    event.preventDefault();
    const next = applySliderAction(action, this.value, this.range, this.pageStep);
    this.#commit(next);
  };

  // ── the pointer ────────────────────────────────────────────────────────────

  #onPointerDown = (event: PointerEvent): void => {
    if (refusesActivation(this)) return;
    const track = this.#track;
    if (track === undefined) return;
    event.preventDefault();
    track.focus();
    this.#dragging = true;
    this.#valueAtDragStart = this.value;
    // Capture, so a drag that leaves the track keeps working. Without it a slider stops responding
    // the moment a hand strays off a four-pixel rail, which reads as the control being broken.
    track.setPointerCapture(event.pointerId);
    this.#moveTo(event);
  };

  #onPointerMove = (event: PointerEvent): void => {
    if (!this.#dragging) return;
    this.#moveTo(event);
  };

  #onPointerUp = (event: PointerEvent): void => {
    if (!this.#dragging) return;
    this.#dragging = false;
    this.#track?.releasePointerCapture(event.pointerId);
    const started = this.#valueAtDragStart;
    this.#valueAtDragStart = undefined;
    // One change for one drag. `preview` fired throughout; this is the event an undo entry is made
    // from, and a drag across a hundred pixels must not be a hundred of them.
    if (started !== undefined && started !== this.value) this.#report(started, this.value);
  };

  #moveTo(event: PointerEvent): void {
    const rail = this.#rail;
    if (rail === undefined) return;
    const box = rail.getBoundingClientRect();
    if (box.width <= 0) return;
    const along = (event.clientX - box.left) / box.width;
    const fraction = this.direction === 'rtl' ? 1 - along : along;
    const next = valueAtFraction(fraction, this.range);
    if (next === this.value) return;
    this.setAttribute('value', String(next));
    this.render();
    this.dispatchEvent(
      new CustomEvent(inputEvents.preview, { bubbles: true, composed: true, detail: { value: next } }),
    );
  }

  #commit(next: number): void {
    const previous = this.value;
    if (next === previous) return;
    this.setAttribute('value', String(next));
    this.render();
    this.#report(previous, next);
  }

  #report(previous: number, value: number): void {
    this.dispatchEvent(
      new CustomEvent(inputEvents.change, {
        bubbles: true,
        composed: true,
        detail: { value, previous },
      }),
    );
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render from the attributes. */
  render(): void {
    const slider = this.#slider;
    const track = this.#track;
    const ticks = this.#ticks;
    const tickLabels = this.#tickLabels;
    const explanation = this.#explanationElement;
    if (
      slider === undefined ||
      track === undefined ||
      ticks === undefined ||
      tickLabels === undefined ||
      explanation === undefined
    ) {
      return;
    }

    const range = this.range;
    const value = this.value;

    track.setAttribute('aria-label', this.label);
    track.setAttribute('aria-orientation', this.orientation);
    track.setAttribute('aria-valuemin', String(range.min));
    track.setAttribute('aria-valuemax', String(range.max));
    track.setAttribute('aria-valuenow', String(value));
    track.setAttribute('aria-valuetext', this.valueText);

    // One number, written once, and both the fill and the thumb read it. A component that
    // positioned the two separately would have a slider whose fill and thumb could disagree.
    slider.style.setProperty(inputBoxProperties.sliderFraction, String(this.fraction));
    // Physical, and therefore mirrored by hand — the same trap `menuBoxProperties` records.
    this.dataset['direction'] = this.direction;

    const disabled = isHardDisabled(this);
    const unavailable = !disabled && isUnavailable(this);
    if (disabled || unavailable) this.dataset['disabled'] = '';
    else delete this.dataset['disabled'];
    if (disabled) track.tabIndex = -1;
    else track.tabIndex = 0;

    this.#renderTicks(ticks, tickLabels, range);
    applyAvailability(this, track, explanation);
  }

  #renderTicks(ticks: HTMLElement, labels: HTMLElement, range: SliderRange): void {
    const wanted = this.ticks;
    const labelled = new Set(this.labelledTicks);
    ticks.replaceChildren();
    labels.replaceChildren();
    ticks.hidden = wanted.length === 0;
    labels.hidden = labelled.size === 0;
    for (const at of wanted) {
      const fraction = sliderFraction(at, range.min, range.max);
      const mark = document.createElement('span');
      mark.className = 'tick';
      mark.style.insetInlineStart = `${String(fraction * 100)}%`;
      ticks.append(mark);

      if (!labelled.has(at)) continue;
      const label = document.createElement('span');
      label.className = 'tick-label';
      label.style.insetInlineStart = `${String(fraction * 100)}%`;
      label.textContent = String(at);
      labels.append(label);
    }
  }
}

/** Register the element. Idempotent. */
export function defineSlider(): void {
  if (customElements.get('mjx-slider') === undefined) {
    customElements.define('mjx-slider', MjxSlider);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-slider': MjxSlider;
  }
}
