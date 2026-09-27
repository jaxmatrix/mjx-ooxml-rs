/**
 * `<mjx-ribbon-tab>` — one tab's worth of groups, and the panel they live in.
 *
 * ```html
 * <mjx-ribbon-tab tab-id="home" label="Home">
 *   <mjx-ribbon-group label="Clipboard" priority="secondary">…</mjx-ribbon-group>
 *   <mjx-ribbon-group label="Font" priority="primary">…</mjx-ribbon-group>
 * </mjx-ribbon-tab>
 * ```
 *
 * ## A declaration, not a button
 *
 * The thing a person clicks is **not** this element. `<mjx-ribbon>` builds the `role="tab"` buttons
 * itself, in its own shadow root, because the ARIA tabs pattern needs them to be siblings inside
 * one `role="tablist"` with one roving tabindex between them — and a tablist whose tabs each lived
 * in a different shadow root is a roving tabindex spread across as many roots as there are tabs.
 * This element is the *panel*: it declares the tab's identity and holds its groups.
 *
 * ## `tab-id`, and not `id`
 *
 * Two reasons, and the second is the one that will bite a later child. An `id` on a light-DOM
 * element is a document-wide name and a catalogue that renders three ribbons would collide.
 * More importantly, **an IDREF does not cross a shadow boundary**: the tab button lives in the
 * ribbon's shadow root and this panel lives in the light DOM, so `aria-controls` pointing here
 * would resolve to nothing whichever spelling was used. The association is therefore made with
 * `aria-label` on the panel — the tab's own name, which is what a screen reader would have read
 * out of an `aria-labelledby` anyway — and the reason is written here rather than discovered again.
 */

import { installFoundations } from '../foundations/stylesheet.ts';

const styles = `
  :host {
    display: block;
    /* The panel below scrolls, and it can only do that if this flex item is allowed to be narrower
     * than its own content. A flex item's min-inline-size is auto, which is the content size, so
     * without this the tab is as wide as every group and the ribbon overflows the screen. */
    flex: 1 1 auto;
    min-inline-size: 0;
  }
  /* The unselected tabs are not merely invisible — display: none takes them out of the
   * accessibility tree and out of the tab order, which is what makes the ribbon have one panel
   * rather than five overlapping ones a keyboard walks through. */
  :host(:not([selected])) {
    display: none;
  }
  .panel {
    display: flex;
    flex-wrap: nowrap;
    align-items: stretch;
    gap: var(--mjx-density-step);
    overflow-x: auto;
    overflow-y: clip;
  }
  /* The groups are slotted, so the slot itself would be the one flex item and they would lay out
   * as inline content inside it, wrapping whatever the container says. display:contents makes each
   * group a flex item of the panel, which is what nowrap is about to be asked about. */
  .panel > slot { display: contents; }
`;

export class MjxRibbonTab extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'tab-id', 'selected'];

  #root: ShadowRoot | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The tab's name, as it is drawn on the tab button and announced. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /** The identity `<mjx-ribbon>` selects by. Falls back to the label so a tab always has one. */
  get tabId(): string {
    const declared = this.getAttribute('tab-id');
    return declared !== null && declared !== '' ? declared : this.label;
  }

  /** Whether this is the shown tab. Written by `<mjx-ribbon>`; never set by hand. */
  get selected(): boolean {
    return this.hasAttribute('selected');
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installFoundations(root);
    const sheet = document.createElement('style');
    sheet.textContent = styles;
    const panel = document.createElement('div');
    panel.className = 'panel';
    panel.setAttribute('part', 'panel');
    panel.append(document.createElement('slot'));
    root.append(sheet, panel);
  }

  render(): void {
    if (this.#root === undefined) return;
    this.setAttribute('role', 'tabpanel');
    this.setAttribute('aria-label', this.label);
  }
}

/** Register the element. Idempotent. */
export function defineRibbonTab(): void {
  if (customElements.get('mjx-ribbon-tab') === undefined) {
    customElements.define('mjx-ribbon-tab', MjxRibbonTab);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-ribbon-tab': MjxRibbonTab;
  }
}
