/**
 * `<mjx-menu-separator>` and `<mjx-menu-section>` — the two things in a menu that are not commands.
 *
 * ```html
 * <mjx-menu label="Edit">
 *   <mjx-menu-item label="Cut" shortcut="Ctrl+X"></mjx-menu-item>
 *   <mjx-menu-separator></mjx-menu-separator>
 *   <mjx-menu-section label="Paste Options">
 *     <mjx-menu-item kind="radio" label="Keep Source Formatting" checked></mjx-menu-item>
 *     <mjx-menu-item kind="radio" label="Merge Formatting"></mjx-menu-item>
 *   </mjx-menu-section>
 * </mjx-menu>
 * ```
 *
 * ## Two ways to divide a menu, and they are not interchangeable
 *
 * A **separator** is a `role="separator"`: it says *these two runs of commands are unrelated* and
 * announces nothing. A **section** is a `role="group"` with a name: it says *these commands are
 * one choice* and a screen reader announces *"Paste Options, group"* on entering it. Office uses
 * both, and using a separator where a section belongs is how a radio group loses the only thing
 * that says the four items are alternatives.
 *
 * The visible heading is `aria-hidden`, because the group already carries the same words in its
 * `aria-label` and a reader that read both would say *"Paste Options"* twice on entering every
 * section. That is the same decision `<mjx-contextual-tab-set>` makes about its coloured band.
 *
 * Neither element is focusable and neither is in the arrow-key sequence — `<mjx-menu>`'s `items`
 * getter walks *through* a section and past a separator, so a section is a naming device rather
 * than a level of navigation.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { menuSectionCss, menuSeparatorCss, menuTypeClass } from './menu-model.ts';

export class MjxMenuSeparator extends HTMLElement {
  #root: ShadowRoot | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) {
      const root = this.attachShadow({ mode: 'open' });
      this.#root = root;
      installControlStyles(root, menuSeparatorCss);
      const rule = document.createElement('div');
      rule.className = 'separator';
      rule.setAttribute('part', 'separator');
      root.append(rule);
    }
    this.setAttribute('role', 'separator');
    // A separator that is not focusable needs no `aria-valuenow`, and saying which way it runs is
    // the one thing a menu's separator has to be explicit about.
    this.setAttribute('aria-orientation', 'horizontal');
  }
}

export class MjxMenuSection extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label'];

  #root: ShadowRoot | undefined;
  #heading: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The section's name — announced once, and drawn once. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, menuSectionCss);

    const heading = document.createElement('p');
    heading.className = `heading ${menuTypeClass('sectionHeading')}`;
    heading.setAttribute('part', 'heading');
    heading.setAttribute('aria-hidden', 'true');

    root.append(heading, document.createElement('slot'));
    this.#heading = heading;
  }

  /** Re-render from the attributes. */
  render(): void {
    this.setAttribute('role', 'group');
    const label = this.label;
    this.setAttribute('aria-label', label);
    if (this.#heading !== undefined) {
      this.#heading.textContent = label;
      this.#heading.hidden = label === '';
    }
  }
}

/** Register both. Idempotent. */
export function defineMenuStructure(): void {
  if (customElements.get('mjx-menu-separator') === undefined) {
    customElements.define('mjx-menu-separator', MjxMenuSeparator);
  }
  if (customElements.get('mjx-menu-section') === undefined) {
    customElements.define('mjx-menu-section', MjxMenuSection);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-menu-separator': MjxMenuSeparator;
    'mjx-menu-section': MjxMenuSection;
  }
}
