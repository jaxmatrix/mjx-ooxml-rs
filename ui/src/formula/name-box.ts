/**
 * `<mjx-name-box>` — the address display that is **also** a combo box and **also** a navigation
 * input.
 *
 * ```html
 * <mjx-name-box label="Name box" address="B7"></mjx-name-box>
 * <script>
 *   document.querySelector('mjx-name-box').names = [
 *     { name: 'Revenue', definition: 'Summary!$B$1', kind: 'name' },
 *     { name: 'SalesTable', definition: 'Sheet1!$A$1:$F$120', kind: 'table' },
 *   ];
 * </script>
 * ```
 *
 * ## It **is** U07's combo box, with one method replaced
 *
 * MJXOFF-192 says so — *"the name box is one with a custom commit behaviour"* — and `MjxComboBox`
 * was built to be subclassed exactly here. Its `resolveTyped` doc names this case in advance:
 *
 * > A subclass overrides it when its field has a **grammar** rather than a list […] `allow-custom`
 * > alone would have committed the typed string verbatim, so the same [address] typed three ways
 * > would have been three different values, and the field would have shown a spelling the control
 * > did not report.
 *
 * So this file is small on purpose. The filtering, the listbox, `aria-activedescendant`, the eight
 * ways of finishing, Escape restoring the text as it was when the list opened, the tab-stop count,
 * the invalid-value event — all of it is inherited and none of it is re-implemented. What is added
 * is three things: a grammar (`navigationFor`), a **navigation request** on every commit, and a
 * spoken problem when the grammar refuses.
 *
 * ## A defined name beats an address that is spelled like one
 *
 * `navigationFor` looks a name up **before** it tries the grammar, and the order is the standing
 * rule about whose document this is: a workbook whose author defined `Q1` means that range. A name
 * box that sent them to cell Q1 instead would be this library imposing its own reading on somebody
 * else's file.
 *
 * ## What it does not do
 *
 * It does not move a selection — there is no grid in this loop. It **requests** one, with the
 * parsed range and its ordered bounds attached, so the thing that owns the grid has nothing left to
 * parse.
 */

import { MjxComboBox, defineComboBox } from '../inputs/combo-box.ts';
import type { OptionDescriptor } from '../inputs/input-model.ts';
import {
  addressProblemMessages,
  formulaEvents,
  navigationFor,
  normaliseTypedAddress,
  parseCellRange,
  selectionAnnouncement,
  type DefinedNameEntry,
  type NavigationRequest,
} from './formula-model.ts';
import { nameBoxCss } from './formula-sheets.ts';

/** What the two kinds of entry are called in the list, so a gate reads the names rather than guessing. */
export const definedNameCategories = {
  name: 'Defined names',
  table: 'Tables',
} as const;

export class MjxNameBox extends MjxComboBox {
  static override readonly observedAttributes: readonly string[] = [
    ...MjxComboBox.observedAttributes,
    'address',
  ];

  #names: readonly DefinedNameEntry[] = [];
  #lastRequest: NavigationRequest | undefined;

  /**
   * The names and tables the list offers.
   *
   * A property rather than markup, for the reason every navigator gives: these come from a
   * workbook, not from a template.
   */
  get names(): readonly DefinedNameEntry[] {
    return this.#names;
  }

  set names(next: readonly DefinedNameEntry[]) {
    this.#names = [...next];
    this.options = this.#names.map(nameOption);
    this.render();
  }

  /** The address showing when nothing is being typed. */
  get address(): string {
    return this.getAttribute('address') ?? '';
  }

  set address(next: string) {
    this.setAttribute('address', next);
  }

  /** The last thing this box asked for, so a host that mounted late can ask. */
  get lastRequest(): NavigationRequest | undefined {
    return this.#lastRequest;
  }

  /**
   * ⚠ **A grammar, not a list.** Three answers and no fourth, exactly as the base class's contract
   * requires: a defined name's own spelling, a canonical address, or `undefined` — which is a
   * refusal the base class turns into a revert *and* an `mjx-input-invalid` event.
   */
  protected override resolveTyped(text: string): string | undefined {
    const request = navigationFor(text, this.#names);
    switch (request.kind) {
      case 'name':
        return request.entry.name;
      case 'address':
        return request.text;
      case 'problem':
        return undefined;
    }
  }

  /**
   * What the field shows.
   *
   * ⚠ The base class's `displayText` reads the option list, and an **address** is not in it. Left
   * alone, typing `B7` would have committed `B7` and then displayed the empty string, breaking the
   * one invariant U07's combo box exists to keep — *the value the field shows must be the value the
   * control reports*.
   */
  override displayText(): string {
    const value = this.value;
    if (value === '') return this.address;
    const entry = this.#names.find((candidate) => candidate.name === value);
    return entry?.name ?? value;
  }

  /** A name box lists the whole workbook whatever is typed, and filters only what it can. */
  protected override commitValue(next: string): void {
    super.commitValue(next);
    this.#navigate(next);
  }

  #navigate(typed: string): void {
    const request = navigationFor(typed, this.#names);
    this.#lastRequest = request;
    if (request.kind === 'problem') {
      this.#say(addressProblemMessages[request.problem]);
      return;
    }
    if (request.kind === 'address') {
      const range = parseCellRange(normaliseTypedAddress(request.text).split('!').pop() ?? '');
      this.#say(range.ok ? selectionAnnouncement(range.value) : request.text);
    } else {
      this.#say(`${request.entry.name}, ${request.entry.definition}`);
    }
    this.dispatchEvent(
      new CustomEvent(formulaEvents.navigate, { bubbles: true, composed: true, detail: request }),
    );
  }

  #say(text: string): void {
    const root = this.shadowRoot;
    if (root === null) return;
    let live = root.querySelector('.name-box-live');
    if (!(live instanceof HTMLElement)) {
      live = document.createElement('div');
      live.className = 'name-box-live visually-hidden';
      live.setAttribute('role', 'status');
      live.setAttribute('aria-live', 'polite');
      root.append(live);
    }
    live.textContent = text;
  }

  protected override rendered(field: HTMLElement, entry: HTMLElement): void {
    super.rendered(field, entry);
    const root = this.shadowRoot;
    if (root !== null && root.querySelector('.name-box-sheet') === null) {
      const style = document.createElement('style');
      style.className = 'name-box-sheet';
      style.textContent = nameBoxCss;
      root.append(style);
    }
  }
}

/** A defined name as a row in U07's listbox: its own name, what it points at, and its group. */
function nameOption(entry: DefinedNameEntry): OptionDescriptor {
  return {
    value: entry.name,
    label: entry.name,
    description: entry.definition,
    category: definedNameCategories[entry.kind],
  };
}

/** Register the element. Idempotent. */
export function defineNameBox(): void {
  defineComboBox();
  if (customElements.get('mjx-name-box') === undefined) {
    customElements.define('mjx-name-box', MjxNameBox);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-name-box': MjxNameBox;
  }
}
