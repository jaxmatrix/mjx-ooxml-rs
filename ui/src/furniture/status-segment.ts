/**
 * `<mjx-status-segment>` — one reading on the status bar.
 *
 * ```html
 * <mjx-status-segment label="Page" value="4 of 20" priority="essential" region="start">
 * </mjx-status-segment>
 * <mjx-status-segment label="Track changes" value="On" priority="standard" announce="polite">
 * </mjx-status-segment>
 * ```
 *
 * ## It declares three things, and each of them belongs to the segment rather than to the bar
 *
 * * **`priority`** — when it gives way as the bar narrows, relative to the others. A width would
 *   have been the wrong declaration for the reason MJXOFF-183 gives about ribbon groups: what a
 *   designer knows is the *ordering*, and CSS can match on an attribute's value but cannot read a
 *   number out of one and use it in a `@container` condition.
 * * **`region`** — start, centre or end. Where a reading belongs is a property of the reading.
 * * **`announce`** — and it defaults to `off`. **A status bar is a live region and the failure is
 *   announcing too much:** a page number that changes as a person scrolls is the reason the bar
 *   exists, it changes constantly, and a screen reader reading it out every time makes the document
 *   unusable. Silence is therefore the default and a segment opts *in*.
 *
 * ## The announcement is an event, not a live region of its own
 *
 * The segment says *my value changed and this is how loudly I asked to be announced*; the bar owns
 * the two live regions and decides nothing else. That is MJXOFF-189's rule — **two regions, never
 * one whose politeness is rewritten** — and putting a region inside every segment would have given
 * a bar of six readings six of them.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  defaultStatusAnnouncement,
  defaultStatusPriority,
  defaultStatusRegion,
  furnitureEvents,
  furnitureTags,
  furnitureTypeRoles,
  isStatusAnnouncement,
  isStatusPriority,
  isStatusRegion,
  statusSegmentCss,
  type StatusAnnouncement,
  type StatusPriority,
  type StatusRegion,
} from './furniture-model.ts';

/** The sheet, composed once. */
export const statusSegmentSheet = statusSegmentCss;

export class MjxStatusSegment extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'value',
    'priority',
    'region',
    'announce',
  ];

  #root: ShadowRoot | undefined;
  #name: HTMLElement | undefined;
  #value: HTMLElement | undefined;
  /**
   * Whether this element has rendered once.
   *
   * ⚠ **The first render is not a change**, and without this the bar would announce every segment
   * it holds the moment a document opened — which is the *"announcing too much"* failure arriving
   * before a person has done anything at all.
   */
  #rendered = false;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.render();
  }

  attributeChangedCallback(name: string, previous: string | null, next: string | null): void {
    if (this.#root === undefined) return;
    this.render();
    if (name === 'value' && this.#rendered && previous !== next) this.#announce();
  }

  /** What the reading is called. `Page`, `Words`, `Language`. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The reading itself. */
  get value(): string {
    return this.getAttribute('value') ?? '';
  }

  set value(next: string) {
    this.setAttribute('value', next);
  }

  /** When it gives way as the bar narrows, relative to the others. */
  get priority(): StatusPriority {
    const declared = this.getAttribute('priority');
    return isStatusPriority(declared) ? declared : defaultStatusPriority;
  }

  /** Which of the bar's three regions it belongs in. */
  get region(): StatusRegion {
    const declared = this.getAttribute('region');
    return isStatusRegion(declared) ? declared : defaultStatusRegion;
  }

  /** How loudly a *change* to it is announced. `off` unless it says otherwise. */
  get announce(): StatusAnnouncement {
    const declared = this.getAttribute('announce');
    return isStatusAnnouncement(declared) ? declared : defaultStatusAnnouncement;
  }

  /** What a screen reader would read from it, which is what the bar announces. */
  get announcement(): string {
    return this.label === '' ? this.value : `${this.label} ${this.value}`;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, statusSegmentSheet);

    const segment = document.createElement('span');
    segment.className = `segment ${furnitureTypeRoles.segment}`;
    segment.setAttribute('part', 'segment');

    const name = document.createElement('span');
    name.className = `name ${furnitureTypeRoles.segmentName}`;
    name.setAttribute('part', 'name');

    const value = document.createElement('span');
    value.className = 'value';
    value.setAttribute('part', 'value');

    segment.append(name, value);
    root.append(segment);
    this.#name = name;
    this.#value = value;
  }

  /** Re-render from the attributes. */
  render(): void {
    const name = this.#name;
    const value = this.#value;
    if (name === undefined || value === undefined) return;
    name.textContent = this.label;
    name.hidden = this.label === '';
    value.textContent = this.value;
    this.#rendered = true;
  }

  #announce(): void {
    this.dispatchEvent(
      new CustomEvent(furnitureEvents.statusChange, {
        bubbles: true,
        composed: true,
        detail: { segment: this, value: this.value, announce: this.announce },
      }),
    );
  }
}

/** Register the element. Idempotent. */
export function defineStatusSegment(): void {
  if (customElements.get(furnitureTags.statusSegment) === undefined) {
    customElements.define(furnitureTags.statusSegment, MjxStatusSegment);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-status-segment': MjxStatusSegment;
  }
}
