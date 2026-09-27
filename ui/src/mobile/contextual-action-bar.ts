/**
 * `<mjx-contextual-action-bar>` — **what replaces the command bar when something is selected.**
 *
 * Format, cut, copy, paste, delete, and the actions that particular selection offers. It is the
 * same bar as `<mjx-command-bar>` — the same rail, the same overflow, the same roving tab stop, the
 * same form-factor blocks — with two differences and no third:
 *
 * * its commands come from `contextualActions(selection)` rather than from a property, so a shell
 *   sets one attribute and the bar is correct; and
 * * it carries a **selection readout**, because a row of verbs with no object is the one thing a
 *   person cannot act on. On a portrait phone the readout is hidden and the announcement carries
 *   it, which is a room decision rather than a decision that the fact does not matter.
 *
 * ```html
 * <mjx-contextual-action-bar selection="text" selection-label="12 words"></mjx-contextual-action-bar>
 * ```
 *
 * ## `selection="none"` is not an empty bar
 *
 * It is **no** bar. A contextual bar with nothing in it would occupy the block-end of a phone with
 * a row of nothing while the command bar it replaced was not on screen; so with no selection the
 * host hides itself, which is what `data-empty` on the host does, and the command bar is what a
 * person sees. There is a gate for it.
 */

import { MjxMobileBar } from './mobile-bar-element.ts';
import { contextualActionBarCss, mobileTypeRoles } from './mobile-sheets.ts';
import {
  contextualActions,
  isSelectionKind,
  mobileTags,
  type MobileCommand,
  type SelectionKind,
} from './mobile-model.ts';

export class MjxContextualActionBar extends MjxMobileBar {
  static override readonly observedAttributes: readonly string[] = [
    'label',
    'open-overflow',
    'selection',
    'selection-label',
  ];

  #readout: HTMLElement | undefined;

  protected styles(): string {
    return contextualActionBarCss;
  }

  protected barCommands(): readonly MobileCommand[] {
    return contextualActions(this.selection);
  }

  protected defaultLabel(): string {
    return 'Selection actions';
  }

  protected override decorate(bar: HTMLElement): void {
    const readout = document.createElement('span');
    readout.className = `selection ${mobileTypeRoles.selection}`;
    readout.setAttribute('part', 'selection');
    this.#readout = readout;
    bar.append(readout);
  }

  /** What the canvas says is selected. */
  get selection(): SelectionKind {
    const value = this.getAttribute('selection');
    return isSelectionKind(value) ? value : 'none';
  }

  set selection(value: SelectionKind) {
    this.setAttribute('selection', value);
  }

  /** What to call it — *12 words*, *a picture*, *B2:D9*. The shell knows; this element does not. */
  get selectionLabel(): string {
    return this.getAttribute('selection-label') ?? '';
  }

  set selectionLabel(value: string) {
    this.setAttribute('selection-label', value);
  }

  override render(): void {
    super.render();
    const readout = this.#readout;
    if (readout === undefined) return;
    const label = this.selectionLabel;
    readout.textContent = label;
    // The bar's own accessible name carries the selection even when the readout is not drawn, so a
    // screen reader on a portrait phone is told what the verbs act on. A fact that is only on
    // screen is a fact only some readers get.
    const bar = this.shadowRoot?.querySelector('.bar');
    if (bar !== null && bar !== undefined && label !== '') {
      bar.setAttribute('aria-label', `${this.label}: ${label}`);
    }
  }
}

/** Register the element. Idempotent. */
export function defineContextualActionBar(): void {
  if (customElements.get(mobileTags.contextualActionBar) === undefined) {
    customElements.define(mobileTags.contextualActionBar, MjxContextualActionBar);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-contextual-action-bar': MjxContextualActionBar;
  }
}
