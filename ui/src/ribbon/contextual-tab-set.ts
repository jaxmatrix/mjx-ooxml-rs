/**
 * `<mjx-contextual-tab-set>` — the titled group of tabs that appears because something is selected.
 *
 * ```html
 * <mjx-contextual-tab-set label="Table Tools">
 *   <mjx-ribbon-tab tab-id="table-design" label="Design">…</mjx-ribbon-tab>
 *   <mjx-ribbon-tab tab-id="table-layout" label="Layout">…</mjx-ribbon-tab>
 * </mjx-contextual-tab-set>
 * ```
 *
 * ## Why this is a wrapper and not a flag on a tab
 *
 * MJXOFF-183: *"Twenty-one distinct tab sets exist in the published surface […] several with two or
 * three tabs each, so this must be a general mechanism rather than one hard-coded case."* (The
 * committed census disagrees with that number — see `dev/word-tab-home.ts` — but the requirement is
 * the same either way, and larger.)
 *
 * A *set* is the unit Office actually publishes: `TabSetTableTools` is the thing that appears, and
 * `Design` and `Layout` are what it contains. Marking each tab `contextual="Table Tools"` would
 * spell the set's name once per tab and let two tabs of one set disagree about it; it would also
 * lose the ordering, which is what puts a set's tabs beside each other under one band. So the set
 * is the element, the tabs are its children, and `<mjx-ribbon>` reads the structure.
 *
 * ## `display: contents`, deliberately
 *
 * This element draws nothing. Its *tabs* are drawn by the ribbon's tab strip (which is where a
 * band over two tab buttons has to be drawn from, since the buttons are the ribbon's), and its
 * *panels* are its `<mjx-ribbon-tab>` children, which lay themselves out exactly as an unwrapped
 * tab does. Generating a box here would put a wrapper between the ribbon's body and a panel and
 * change nothing else.
 */

const styles = `
  :host {
    display: contents;
  }
`;

export class MjxContextualTabSet extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label'];

  #root: ShadowRoot | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
  }

  /** The set's title — *Table Tools*, *Picture Tools*, *Chart Tools*. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    const sheet = document.createElement('style');
    sheet.textContent = styles;
    root.append(sheet, document.createElement('slot'));
  }
}

/** Register the element. Idempotent. */
export function defineContextualTabSet(): void {
  if (customElements.get('mjx-contextual-tab-set') === undefined) {
    customElements.define('mjx-contextual-tab-set', MjxContextualTabSet);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-contextual-tab-set': MjxContextualTabSet;
  }
}
