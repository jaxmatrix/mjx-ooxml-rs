/**
 * `<mjx-split-button>` — a primary action and a menu arrow, in one control. 440 of Office's
 * published controls, and the one whose failures are silent.
 *
 * ```html
 * <mjx-split-button label="Undo" icon="arrow-undo" size="large"
 *                   menu-label="Undo history"></mjx-split-button>
 * ```
 *
 * ## Two controls, and the three ways that goes wrong
 *
 * MJXOFF-182: *"a primary action and a menu arrow with **two independent hit regions**, each
 * separately focusable and separately hoverable."* Each of those three is a separate failure and
 * each is silent — the control looks right in every screenshot:
 *
 * * **Not separately focusable.** One `<button>` with an arrow drawn inside it is a control whose
 *   menu a keyboard cannot reach at all. Two real buttons is the answer; sequential focus
 *   navigation descends into a shadow tree, so `Tab` reaches both with no `tabindex` anywhere.
 * * **Not separately hoverable.** A `:hover` written on the host — the obvious way to build this —
 *   lights *both* regions whichever one the pointer is over, so the control never tells you which
 *   half you are about to press. Every state rule here matches `.control`, which both regions
 *   carry, so each hovers on its own and neither can light the other.
 * * **Not separately labelled.** *"Undo"* twice is two commands with one name. The arrow's own name
 *   comes from `menu-label`, and is derived when absent rather than left blank.
 *
 * All three are asserted in `tests/browser/controls.spec.ts` — by keyboard, by pointer position,
 * and by reading the two accessible names.
 *
 * ## Arrow Down opens the menu and does not fire the action
 *
 * This is the keyboard half of *two controls in one*, and getting it wrong destroys work: a person
 * who meant to see the paste options and instead pasted has an edit to undo. `ArrowDown` — with or
 * without `Alt`, since both are in common use — requests the menu from either region and cannot
 * produce an activation, because it never produces a `click` and `mjx-activate` is only ever
 * emitted from one.
 *
 * ## The menu is requested, not opened
 *
 * U05 owns menus. `aria-expanded` follows this component's `expanded` attribute and never the
 * event, so a host that has not built a menu yet does not announce one that is not there.
 *
 * `GUESS:` the derived arrow name is *"More <label> options"*, which is the shape Office uses
 * (*"More Paste options"*). It is not checked against Office, and a caller who knows the real name
 * gives it in `menu-label`.
 */

import { defineIcon } from '../icons/icon.ts';
import {
  applyAvailability,
  controlClassName,
  controlEvents,
  createExplanationElement,
  emitControlEvent,
  explanationElementId,
  forcedState,
  installControlStyles,
  refusesActivation,
} from './control-element.ts';
import {
  controlBaseCss,
  controlSizeCss,
  controlSizes,
  controlStatesCss,
  defaultControlSize,
  derivedMenuLabel,
  isControlSize,
  isForcibleState,
  splitMenuIcon,
  type ControlSize,
} from './control-states.ts';

/**
 * The split's own rules: the seam between the two regions, and the divider that makes it legible.
 *
 * The radius surgery is why the seam reads as one control rather than as two buttons that happen
 * to touch — and the divider is why it reads as *split* rather than as one wide button. Both are
 * `0` and `1px`, the two lengths that are not design decisions.
 */
export const splitButtonCss = `
  .split {
    display: inline-flex;
    align-items: stretch;
  }
  .control[data-region='primary'] {
    border-start-end-radius: 0;
    border-end-end-radius: 0;
  }
  .control[data-region='menu'] {
    border-start-start-radius: 0;
    border-end-start-radius: 0;
    padding-inline: var(--mjx-density-step);
  }
  .divider {
    flex: none;
    align-self: stretch;
    inline-size: 1px;
    margin-block: var(--mjx-density-step);
    background: var(--theme-border);
  }
`;

/** The whole sheet, composed once and shared by every instance. */
export const splitButtonSheet = [
  controlBaseCss,
  controlSizeCss,
  controlStatesCss('.control'),
  splitButtonCss,
].join('\n');

export class MjxSplitButton extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'icon',
    'size',
    'disabled',
    'unavailable',
    'explanation',
    'force-state',
    'force-menu-state',
    'menu-label',
    'expanded',
  ];

  #root: ShadowRoot | undefined;
  #primary: HTMLButtonElement | undefined;
  #menu: HTMLButtonElement | undefined;
  #labelElement: HTMLElement | undefined;
  #menuLabelElement: HTMLElement | undefined;
  #iconElement: HTMLElement | undefined;
  #explanationElement: HTMLElement | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The primary action's name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The arrow's own name. Derived from the label when it is not given. */
  get menuLabel(): string {
    const declared = this.getAttribute('menu-label');
    if (declared !== null && declared.trim() !== '') return declared;
    return derivedMenuLabel(this.label);
  }

  /** The Fluent icon's name, or `undefined`. */
  get icon(): string | undefined {
    const declared = this.getAttribute('icon');
    return declared === null || declared === '' ? undefined : declared;
  }

  /** `large`, `small` or `icon`. */
  get size(): ControlSize {
    const declared = this.getAttribute('size');
    return isControlSize(declared) ? (declared as ControlSize) : defaultControlSize;
  }

  /** Whether the host says a menu is open. Never set by this component — see the module note. */
  get expanded(): boolean {
    return this.hasAttribute('expanded');
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, splitButtonSheet);

    const container = document.createElement('div');
    container.className = 'split';
    container.setAttribute('part', 'split');

    const primary = document.createElement('button');
    primary.type = 'button';
    primary.className = controlClassName;
    primary.dataset['region'] = 'primary';
    primary.setAttribute('part', 'primary');
    primary.addEventListener('click', this.#onPrimaryClick);
    primary.addEventListener('keydown', this.#onKeyDown);

    const icon = document.createElement('mjx-icon');
    const label = document.createElement('span');
    primary.append(icon, label);

    const divider = document.createElement('span');
    divider.className = 'divider';
    divider.setAttribute('part', 'divider');
    // Decorative: the seam is announced by there being two buttons, not by a third element.
    divider.setAttribute('aria-hidden', 'true');

    const menu = document.createElement('button');
    menu.type = 'button';
    menu.className = controlClassName;
    menu.dataset['region'] = 'menu';
    menu.setAttribute('part', 'menu');
    menu.setAttribute('aria-haspopup', 'menu');
    menu.addEventListener('click', this.#onMenuClick);
    menu.addEventListener('keydown', this.#onKeyDown);

    const menuIcon = document.createElement('mjx-icon');
    menuIcon.setAttribute('name', splitMenuIcon.name);
    menuIcon.setAttribute('size', String(splitMenuIcon.size));
    const menuLabel = document.createElement('span');
    menuLabel.className = 'visually-hidden';
    menu.append(menuIcon, menuLabel);

    const explanation = createExplanationElement(explanationElementId);

    container.append(primary, divider, menu);
    root.append(container, explanation);

    this.#primary = primary;
    this.#menu = menu;
    this.#iconElement = icon;
    this.#labelElement = label;
    this.#menuLabelElement = menuLabel;
    this.#explanationElement = explanation;
  }

  #onPrimaryClick = (event: MouseEvent): void => {
    if (refusesActivation(this)) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    emitControlEvent(this, controlEvents.activate, { region: 'primary' });
  };

  #onMenuClick = (event: MouseEvent): void => {
    if (refusesActivation(this)) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }
    // Not `activate`. The whole point of the arrow is that it is not the action.
    emitControlEvent(this, controlEvents.menuRequest, { region: 'menu' });
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== 'ArrowDown') return;
    event.preventDefault();
    if (refusesActivation(this)) return;
    const region = (event.currentTarget as HTMLElement | null)?.dataset['region'] ?? 'primary';
    emitControlEvent(this, controlEvents.menuRequest, { region });
  };

  /** Re-render from the attributes. */
  render(): void {
    const primary = this.#primary;
    const menu = this.#menu;
    const labelElement = this.#labelElement;
    const menuLabelElement = this.#menuLabelElement;
    const iconElement = this.#iconElement;
    const explanationElement = this.#explanationElement;
    if (
      primary === undefined ||
      menu === undefined ||
      labelElement === undefined ||
      menuLabelElement === undefined ||
      iconElement === undefined ||
      explanationElement === undefined
    ) {
      return;
    }

    const size = this.size;
    const spec = controlSizes[size];
    primary.dataset['size'] = size;
    // The arrow is a fixed mark whatever shape the primary takes: an arrow that grew to `large`
    // and put its chevron above a hidden label would be a second, wrong reading of the size.
    menu.dataset['size'] = 'icon';

    const icon = this.icon;
    if (icon === undefined) {
      iconElement.remove();
    } else {
      iconElement.setAttribute('name', icon);
      iconElement.setAttribute('size', String(spec.iconSize));
      iconElement.setAttribute('variant', 'regular');
      if (iconElement.parentNode === null) primary.prepend(iconElement);
    }

    labelElement.textContent = this.label;
    labelElement.className = spec.labelVisible ? 'label' : 'visually-hidden';
    menuLabelElement.textContent = this.menuLabel;

    const forcedPrimary = forcedState(this);
    if (forcedPrimary === undefined) delete primary.dataset['state'];
    else primary.dataset['state'] = forcedPrimary;

    const declaredMenuState = this.getAttribute('force-menu-state');
    if (isForcibleState(declaredMenuState)) menu.dataset['state'] = declaredMenuState;
    else delete menu.dataset['state'];

    menu.setAttribute('aria-expanded', this.expanded ? 'true' : 'false');

    applyAvailability(this, primary, explanationElement);
    applyAvailability(this, menu, explanationElement);
  }
}

/** Register the element. Idempotent. */
export function defineSplitButton(): void {
  defineIcon();
  if (customElements.get('mjx-split-button') === undefined) {
    customElements.define('mjx-split-button', MjxSplitButton);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-split-button': MjxSplitButton;
  }
}
