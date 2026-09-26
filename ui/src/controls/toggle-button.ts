/**
 * `<mjx-toggle-button>` — a command that holds a state. 1,261 of Office's published controls.
 *
 * ```html
 * <mjx-toggle-button label="Bold" icon="text-bold" size="icon" pressed="true"></mjx-toggle-button>
 * <mjx-toggle-button label="Bold" icon="text-bold" pressed="mixed"></mjx-toggle-button>
 * ```
 *
 * ## The failure this component exists to avoid
 *
 * MJXOFF-182 names it: *"a pressed state that is **distinguishable from hover at a glance**, which
 * is the classic failure."* It happens because both states want to say *something is happening
 * here* and both reach for the same tint. The answer here is that they do not share a family at
 * all — **hover is neutral, pressed is the accent tint, and pressed-while-hovered thickens an edge
 * rather than deepening a fill.** All three are rows of `controlStateSpecs`, and no two of them may
 * compute alike in either scheme; `tests/controls.test.ts` proves that from the token table and
 * `tests/browser/controls.spec.ts` proves it from the rendering.
 *
 * ## The third position
 *
 * `pressed="mixed"` is a selection that disagrees with itself — half the run is bold and half is
 * not — and `aria-pressed="mixed"` is what says so. It is deliberately **not** a weaker version of
 * pressed: it takes the honey half of the palette, because a mixed selection is a disagreement
 * rather than a partial agreement, and because a lighter green would be exactly the pair of states
 * the pairwise gate exists to reject.
 *
 * `GUESS:` one activation takes `mixed` to `true` — the whole selection becomes bold rather than
 * losing its formatting. That is what Word appears to do and it is *not* checked against Office;
 * see `nextPressed` in `control-states.ts`.
 *
 * ## One of a set
 *
 * `exclusive="<set>"` makes this toggle one of a set of which exactly one holds — Word's document
 * views, the Draw tab's ink tools. Pressing it releases the other members in its ribbon tab, and
 * pressing it while it holds keeps it. With `exclusive-allows-none` as well, pressing it while it holds
 * releases it, so the set holds at most one — the Background Removal tab's two marking pencils. It stays a toggle button to assistive technology. The mechanism,
 * and the designs it was chosen over, are in `exclusive-set.ts`.
 *
 * ## The icon changes drawing, not colour
 *
 * Fluent draws a `filled` variant for exactly this — MJXOFF-181's manifest says so on every toggle
 * row: *"A toggle, so it needs the filled drawing for its selected state."* A pressed toggle
 * therefore asks for the filled glyph **if the subset carries one**, and falls back to the regular
 * drawing rather than rendering nothing, because `<mjx-icon>` draws nothing for a glyph it does not
 * have and a blank toolbar button is a bug nobody notices.
 */

import { defineIcon, lookupGlyph } from '../icons/icon.ts';
import type { IconSize, IconVariant } from '../icons/manifest.ts';
import { MjxButton, buttonAttributes } from './button.ts';
import {
  pressedFromAttribute,
  pressedIconVariant,
  type PressedValue,
} from './control-states.ts';
import { activateToggle, exclusiveAttribute, exclusiveSetFromAttribute } from './exclusive-set.ts';

export class MjxToggleButton extends MjxButton {
  static override readonly observedAttributes: readonly string[] = [
    ...buttonAttributes,
    'pressed',
  ];

  /** `false`, `true` or `mixed`. Absent means `false`. */
  get pressed(): PressedValue {
    // One reading, shared with `<mjx-split-button toggle>`: a bare attribute is true.
    return pressedFromAttribute(this.getAttribute('pressed'));
  }

  set pressed(value: PressedValue) {
    this.setAttribute('pressed', value);
  }

  /** The exclusive set this toggle belongs to, or `undefined`. See `exclusive-set.ts`. */
  get exclusive(): string | undefined {
    return exclusiveSetFromAttribute(this.getAttribute(exclusiveAttribute));
  }

  set exclusive(value: string | undefined) {
    if (value === undefined) this.removeAttribute(exclusiveAttribute);
    else this.setAttribute(exclusiveAttribute, value);
  }

  /**
   * Move, then report.
   *
   * The attribute is written first so `aria-pressed` and the paint have already moved by the time
   * a listener runs — a listener that read `event.target.pressed` and got the *old* value would be
   * a footgun with no upside. Inside an `exclusive` set, the other members are released first and
   * the pressed member stays pressed; `activateToggle` is where both are written.
   */
  protected override activated(_event: MouseEvent): void {
    activateToggle(this, this.pressed);
  }

  protected override rendered(button: HTMLButtonElement): void {
    const pressed = this.pressed;
    // `aria-pressed` is what makes it a toggle to a screen reader, and it is written for all three
    // positions including `false` — a toggle that only announced itself when pressed would sound
    // like an ordinary button for the whole time it is off.
    button.setAttribute('aria-pressed', pressed);
    // …and the same fact in the form the stylesheet reads. One source, two spellings, written
    // together so they cannot drift.
    button.dataset['pressed'] = pressed;
  }

  protected override iconVariant(name: string, size: IconSize): IconVariant {
    return pressedIconVariant(this.pressed, lookupGlyph(name, size, 'filled') !== undefined);
  }
}

/** Register the element. Idempotent. */
export function defineToggleButton(): void {
  defineIcon();
  if (customElements.get('mjx-toggle-button') === undefined) {
    customElements.define('mjx-toggle-button', MjxToggleButton);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-toggle-button': MjxToggleButton;
  }
}
