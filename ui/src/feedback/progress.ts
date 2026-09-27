/**
 * `<mjx-progress>` — **two kinds of bar, and the one that must not be mistaken for a stalled one.**
 *
 * ```html
 * <mjx-progress label="Uploading" value="3" max="7"></mjx-progress>
 * <mjx-progress label="Contacting the server" indeterminate></mjx-progress>
 * ```
 *
 * ## Two traps, and MJXOFF-189 names both
 *
 * > **Progress has two kinds and one is the identity-value trap.** A determinate bar tested at a
 * > single value exercises no arithmetic; assert several values including both ends. An
 * > indeterminate one must be distinguishable from a stalled determinate one.
 *
 * The first is answered in the model: `progressFraction` has two clamps, a non-unit maximum, a zero
 * maximum and two non-finite cases in it, and *any single value* exercises at most one of them.
 * `tests/feedback.test.ts` drives all of them and `tests/browser/feedback.spec.ts` measures the
 * **painted** indicator at five values, comparing each against the track it sits in rather than
 * against the previous one — because a bar that grew is not the same claim as a bar that is
 * proportional.
 *
 * The second is answered three ways at once, and it needs three because the third is switched off
 * for some people:
 *
 * | Instrument | Determinate | Indeterminate |
 * |---|---|---|
 * | the accessibility tree | `aria-valuenow` is present, `aria-valuetext` is a percentage | **no** `aria-valuenow`, and the text says *working* |
 * | geometry | the indicator's width **is** the value | a fixed span of the track, which is no value |
 * | time | still, at a fixed value | its position differs between two samples |
 *
 * ⚠ **The third is deliberately absent under a reduced-motion preference**, which is exactly why
 * the other two exist. A person who has asked their operating system to stop moving things gets a
 * bar that does not move, and if motion were the only difference they would be looking at something
 * indistinguishable from a task that had stopped. The gate emulates the preference and asserts the
 * first two still hold.
 *
 * ## `role="progressbar"`, and its value is the author's own scale
 *
 * `aria-valuemax` is the `max` the author declared, not a normalised 100 — so *“3 of 7 files”* is
 * announceable, and the arithmetic is exercised at a maximum that is not 1. `aria-valuetext` is the
 * percentage, which is what a person actually wants to hear.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  feedbackBoxProperties,
  feedbackMotionClasses,
  feedbackTags,
  feedbackTypeRoles,
  formatProgressPercent,
  progressCss,
  progressFraction,
  progressValueText,
  type ProgressKind,
} from './feedback-model.ts';

/** The maximum a bar takes when nobody declares one: a fraction of the whole. */
const defaultMax = 1;

export class MjxProgress extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'value',
    'max',
    'indeterminate',
    'readout',
  ];

  #root: ShadowRoot | undefined;
  #labelElement: HTMLElement | undefined;
  #readoutElement: HTMLElement | undefined;
  #track: HTMLElement | undefined;
  #indicator: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The bar's accessible name. A progress bar with no name is a moving rectangle. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** How much is done, in the author's own units. */
  get value(): number {
    const declared = Number.parseFloat(this.getAttribute('value') ?? '');
    return Number.isFinite(declared) ? declared : 0;
  }

  set value(next: number) {
    this.setAttribute('value', String(next));
  }

  /** What *all of it* is, in the same units. */
  get max(): number {
    const declared = Number.parseFloat(this.getAttribute('max') ?? '');
    return Number.isFinite(declared) ? declared : defaultMax;
  }

  set max(next: number) {
    this.setAttribute('max', String(next));
  }

  /** Which of the two kinds this is. */
  get kind(): ProgressKind {
    return this.hasAttribute('indeterminate') ? 'indeterminate' : 'determinate';
  }

  get indeterminate(): boolean {
    return this.hasAttribute('indeterminate');
  }

  set indeterminate(value: boolean) {
    if (value) this.setAttribute('indeterminate', '');
    else this.removeAttribute('indeterminate');
  }

  /** Whether the percentage is drawn beside the label. Announced either way. */
  get readout(): boolean {
    return this.hasAttribute('readout');
  }

  /** How far along, as a fraction of the whole. `0` for an indeterminate bar: nobody knows. */
  get fraction(): number {
    if (this.kind === 'indeterminate') return 0;
    return progressFraction(this.value, this.max);
  }

  /** The `role="progressbar"` element, so a gate names it rather than guessing at a selector. */
  get bar(): HTMLElement | undefined {
    return this.#track;
  }

  /** The painted part. */
  get indicator(): HTMLElement | undefined {
    return this.#indicator;
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, progressCss);

    const field = document.createElement('div');
    field.className = 'field';
    field.setAttribute('part', 'field');

    const heading = document.createElement('div');
    heading.className = 'heading';
    heading.setAttribute('part', 'heading');

    const label = document.createElement('span');
    label.className = `label ${feedbackTypeRoles.progressLabel}`;
    label.setAttribute('part', 'label');
    this.#labelElement = label;

    const readout = document.createElement('span');
    readout.className = `readout ${feedbackTypeRoles.progressReadout}`;
    readout.setAttribute('part', 'readout');
    // Drawn only. The same number reaches an assistive technology through `aria-valuetext`, and a
    // readout that was also announced would say the percentage twice.
    readout.setAttribute('aria-hidden', 'true');
    this.#readoutElement = readout;

    heading.append(label, readout);

    const track = document.createElement('div');
    track.className = 'track';
    track.setAttribute('part', 'track');
    track.setAttribute('role', 'progressbar');
    this.#track = track;

    const indicator = document.createElement('div');
    indicator.className = `indicator ${feedbackMotionClasses.progress}`;
    indicator.setAttribute('part', 'indicator');
    this.#indicator = indicator;

    track.append(indicator);
    field.append(heading, track);
    root.append(field);
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const track = this.#track;
    const indicator = this.#indicator;
    if (track === undefined || indicator === undefined) return;

    const kind = this.kind;
    const label = this.label;
    const fraction = this.fraction;

    if (this.#labelElement !== undefined) this.#labelElement.textContent = label;
    if (this.#readoutElement !== undefined) {
      this.#readoutElement.textContent = kind === 'determinate' ? formatProgressPercent(fraction) : '';
      this.#readoutElement.hidden = !this.readout || kind !== 'determinate';
    }

    track.dataset['kind'] = kind;
    track.setAttribute('aria-label', label);
    track.setAttribute('aria-valuetext', progressValueText(kind, fraction, label));

    if (kind === 'determinate') {
      const max = this.max;
      track.setAttribute('aria-valuemin', '0');
      track.setAttribute('aria-valuemax', String(max));
      /*
       * The *clamped* value, so a bar declared past its maximum announces what it draws. Announcing
       * one number and painting another is the defect a readout is supposed to prevent.
       *
       * ⚠ **Clamped, and not `fraction * max`.** The obvious spelling recovers the value from the
       * fraction, and floating-point division does not always give it back: one of forty-nine
       * returns to `0.9999999999999999`, which an assistive technology reads out in full. It
       * happened to be exact for every value in the catalogue's own stories, which is precisely how
       * a defect like this survives a suite.
       */
      track.setAttribute('aria-valuenow', String(Math.min(Math.max(this.value, 0), max)));
      indicator.style.setProperty(feedbackBoxProperties.progressFill, formatProgressPercent(fraction));
    } else {
      /*
       * ⚠ **Removed, not set to something.** An indeterminate `role="progressbar"` is defined by
       * the *absence* of `aria-valuenow`; a bar that reported `0` would announce a task that has
       * made no progress, which is a different and false claim. This is the half of the
       * indeterminate distinction that survives a reduced-motion preference.
       */
      track.removeAttribute('aria-valuenow');
      track.removeAttribute('aria-valuemin');
      track.removeAttribute('aria-valuemax');
      indicator.style.removeProperty(feedbackBoxProperties.progressFill);
    }
  }
}

/** Register the element. Idempotent. */
export function defineProgress(): void {
  if (customElements.get(feedbackTags.progress) === undefined) {
    customElements.define(feedbackTags.progress, MjxProgress);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-progress': MjxProgress;
  }
}
