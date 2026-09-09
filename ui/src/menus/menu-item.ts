/**
 * `<mjx-menu-item>` — one row of a menu, in the anatomy Office uses.
 *
 * ```html
 * <mjx-menu-item label="Paste" icon="folder-open" shortcut="Ctrl+V"></mjx-menu-item>
 * <mjx-menu-item kind="checkbox" label="Ruler" checked></mjx-menu-item>
 * <mjx-menu-item kind="radio" label="Portrait" checked value="portrait"></mjx-menu-item>
 * <mjx-menu-item label="Paste Special" unavailable
 *                explanation="The clipboard holds no formatted content."></mjx-menu-item>
 * <mjx-menu-item label="Paste Options">
 *   <mjx-menu slot="submenu" label="Paste Options">…</mjx-menu>
 * </mjx-menu-item>
 * ```
 *
 * ## The role is on the host, and that is load-bearing rather than tidy
 *
 * A custom element that put `role="menuitem"` on a `<div>` inside its shadow root would leave a
 * role-less host between the menu and the item, and an accessibility checker walking a `role="menu"`
 * element's *owned* children descends through role-less elements. It would therefore descend
 * **past** this element and find the nested `<mjx-menu slot="submenu">` as a child of the outer
 * menu — a `menu` owned by a `menu`, which is not an allowed child and is a violation of a rule
 * nobody meant to break. Putting the role on the host stops the walk exactly where the ARIA tree
 * should stop it, and the submenu becomes what it is: a descendant of the item that opens it.
 *
 * The cost is that `.item` — the box that is actually painted — cannot carry `aria-disabled` or
 * `aria-checked`, because those are not global attributes and a `<div>` wearing one is a genuine
 * `aria-allowed-attr` violation. So the host's ARIA state is mirrored onto `.item` as `data-*`,
 * and `src/menus/menu-model.ts`'s selectors read the mirrors. **The mirror is one-way and written
 * in one place**, which is what stops the two drifting.
 *
 * ## A menu item is never hard-disabled
 *
 * `unavailable` sets `aria-disabled` and nothing else, and the row stays in the arrow-key sequence.
 * `menu-model.ts` explains why at length; the short version is that a person must be able to find
 * out that a command exists and is currently unavailable, and an item that took itself out of the
 * sequence would be a command whose absence looks like a bug. Activation is refused in JavaScript,
 * which is the price of staying reachable — the same trade `<mjx-button>` makes.
 *
 * ## Naming
 *
 * The accessible name is set explicitly rather than derived from the row's own text, because the
 * row's text is *"Paste  Ctrl+V  Keep source formatting"* and that is three things, not a name. The
 * shortcut goes where ARIA puts a shortcut — `aria-keyshortcuts` — and the description is appended
 * to the name, which is a **`GUESS:`** about how Office announces its two-line items and is marked
 * as one at `accessibleName` below.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  ariaKeyShortcuts,
  isMenuItemKind,
  markIconFor,
  menuGlyphSize,
  menuItemRoles,
  menuItemSheet,
  menuTypeClass,
  submenuArrowIcon,
  type MenuItemKind,
} from './menu-model.ts';
import { isForcibleState } from '../controls/control-states.ts';

/** The id the explanation lives under, resolved inside this shadow root. */
const explanationId = 'explanation';

export class MjxMenuItem extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'icon',
    'kind',
    'checked',
    'unavailable',
    'explanation',
    'shortcut',
    'keyshortcuts',
    'description',
    'value',
    'force-state',
  ];

  #root: ShadowRoot | undefined;
  #item: HTMLElement | undefined;
  #mark: HTMLElement | undefined;
  #iconCell: HTMLElement | undefined;
  #labelElement: HTMLElement | undefined;
  #descriptionElement: HTMLElement | undefined;
  #shortcutElement: HTMLElement | undefined;
  #arrow: HTMLElement | undefined;
  #explanation: HTMLElement | undefined;
  #hasSubmenu = false;
  #submenuOpen = false;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    // The focus ring lives in the foundations and the host is in the light DOM, so the sheet has
    // to be on the document as well as on the shadow root. `ui/README.md` states the rule; this is
    // the first component that needs both halves of it.
    installFoundations(this.ownerDocument);
    if (!this.hasAttribute('tabindex')) this.tabIndex = -1;
    this.render();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The command's name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** `command`, `checkbox` or `radio`. */
  get kind(): MenuItemKind {
    const declared = this.getAttribute('kind');
    return isMenuItemKind(declared) ? declared : 'command';
  }

  /** Whether a checkable item is on. Meaningless on a plain command, and not announced there. */
  get checked(): boolean {
    return this.hasAttribute('checked');
  }

  set checked(value: boolean) {
    if (value) this.setAttribute('checked', '');
    else this.removeAttribute('checked');
  }

  /** Whether the item is unavailable-but-explained. A menu has no other kind. */
  get unavailable(): boolean {
    return this.hasAttribute('unavailable');
  }

  /** The value a radio group reports, falling back to the label. */
  get value(): string {
    return this.getAttribute('value') ?? this.label;
  }

  /** The visible shortcut hint. */
  get shortcut(): string {
    return this.getAttribute('shortcut') ?? '';
  }

  /** The second line, when there is one. */
  get description(): string {
    return this.getAttribute('description') ?? '';
  }

  /** Whether this item owns a submenu. Reconciled from the slot, never declared twice. */
  get hasSubmenu(): boolean {
    return this.#hasSubmenu;
  }

  /** The submenu, or `undefined`. */
  get submenu(): HTMLElement | undefined {
    for (const child of this.children) {
      if (child instanceof HTMLElement && child.getAttribute('slot') === 'submenu') return child;
    }
    return undefined;
  }

  /** Whether the submenu is showing. Set by the menu that owns the level. */
  get submenuOpen(): boolean {
    return this.#submenuOpen;
  }

  set submenuOpen(value: boolean) {
    this.#submenuOpen = value;
    this.render();
  }

  /**
   * The accessible name.
   *
   * `GUESS:` a described item announces *"Keep Source Formatting, keeps the original fonts and
   * colours"*. Office's own two-line paste items do announce both, but the exact separator is not
   * something this project has measured, and a caller who knows better sets `aria-label` itself —
   * which this method deliberately does not overwrite.
   */
  get accessibleName(): string {
    const description = this.description.trim();
    return description === '' ? this.label : `${this.label}, ${description}`;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, menuItemSheet);

    const item = document.createElement('div');
    // ⚠ The type role goes on **the element the state table paints**, never on the label inside
    // it. `.mjx-type-control` declares `font-weight`, and a role class on `.label` would beat the
    // row's inherited weight — so a checked item would lose its bold while every "the state
    // exists" check passed. That is MJXOFF-182's adoption-order accident wearing a fourth costume,
    // and `tests/browser/menus.spec.ts` reads the weight off `.item` for exactly that reason.
    item.className = `item ${menuTypeClass('row')} mjx-hit-target mjx-motion-surface-settle`;
    item.setAttribute('part', 'item');

    const mark = document.createElement('span');
    mark.className = 'mark';
    mark.setAttribute('part', 'mark');
    mark.setAttribute('aria-hidden', 'true');

    const iconCell = document.createElement('span');
    iconCell.className = 'icon';
    iconCell.setAttribute('part', 'icon');
    iconCell.setAttribute('aria-hidden', 'true');

    const lines = document.createElement('span');
    lines.className = 'lines';

    const label = document.createElement('span');
    label.className = 'label';
    label.setAttribute('part', 'label');

    const description = document.createElement('span');
    description.className = `description ${menuTypeClass('description')}`;
    description.setAttribute('part', 'description');

    lines.append(label, description);

    const shortcut = document.createElement('span');
    shortcut.className = `shortcut ${menuTypeClass('shortcut')}`;
    shortcut.setAttribute('part', 'shortcut');
    shortcut.setAttribute('aria-hidden', 'true');

    const arrow = document.createElement('span');
    arrow.className = 'arrow';
    arrow.setAttribute('part', 'arrow');
    arrow.setAttribute('aria-hidden', 'true');

    item.append(mark, iconCell, lines, shortcut, arrow);

    const explanation = document.createElement('span');
    explanation.className = 'visually-hidden';
    explanation.id = explanationId;

    const submenuSlot = document.createElement('slot');
    submenuSlot.name = 'submenu';
    submenuSlot.addEventListener('slotchange', this.#onSubmenuSlotChange);

    root.append(item, explanation, submenuSlot);

    this.#item = item;
    this.#mark = mark;
    this.#iconCell = iconCell;
    this.#labelElement = label;
    this.#descriptionElement = description;
    this.#shortcutElement = shortcut;
    this.#arrow = arrow;
    this.#explanation = explanation;
    this.#hasSubmenu = this.submenu !== undefined;
  }

  #onSubmenuSlotChange = (): void => {
    this.#hasSubmenu = this.submenu !== undefined;
    this.render();
  };

  /** Re-render from the attributes. Safe at any time; a no-op before the first build. */
  render(): void {
    const item = this.#item;
    const mark = this.#mark;
    const iconCell = this.#iconCell;
    const labelElement = this.#labelElement;
    const descriptionElement = this.#descriptionElement;
    const shortcutElement = this.#shortcutElement;
    const arrow = this.#arrow;
    const explanation = this.#explanation;
    if (
      item === undefined ||
      mark === undefined ||
      iconCell === undefined ||
      labelElement === undefined ||
      descriptionElement === undefined ||
      shortcutElement === undefined ||
      arrow === undefined ||
      explanation === undefined
    ) {
      return;
    }

    const kind = this.kind;
    this.setAttribute('role', menuItemRoles[kind]);

    // ── the ARIA state, on the host ──
    if (kind === 'command') this.removeAttribute('aria-checked');
    else this.setAttribute('aria-checked', this.checked ? 'true' : 'false');

    if (this.unavailable) this.setAttribute('aria-disabled', 'true');
    else this.removeAttribute('aria-disabled');

    if (this.#hasSubmenu) {
      this.setAttribute('aria-haspopup', 'menu');
      this.setAttribute('aria-expanded', this.#submenuOpen ? 'true' : 'false');
    } else {
      this.removeAttribute('aria-haspopup');
      this.removeAttribute('aria-expanded');
    }

    const shortcut = this.shortcut.trim();
    const declaredKeys = this.getAttribute('keyshortcuts');
    if (declaredKeys !== null && declaredKeys.trim() !== '') {
      this.setAttribute('aria-keyshortcuts', declaredKeys);
    } else if (shortcut !== '') {
      this.setAttribute('aria-keyshortcuts', ariaKeyShortcuts(shortcut));
    } else {
      this.removeAttribute('aria-keyshortcuts');
    }

    this.setAttribute('aria-label', this.accessibleName);

    // ── the presentation mirrors, on the box ──
    if (kind === 'command') delete item.dataset['checked'];
    else item.dataset['checked'] = this.checked ? 'true' : 'false';

    if (this.unavailable) item.dataset['unavailable'] = '';
    else delete item.dataset['unavailable'];

    if (this.#submenuOpen) item.dataset['submenuOpen'] = '';
    else delete item.dataset['submenuOpen'];

    const forced = this.getAttribute('force-state');
    if (isForcibleState(forced)) item.dataset['state'] = forced;
    else delete item.dataset['state'];

    // ── the anatomy ──
    const markIcon = markIconFor(kind);
    mark.replaceChildren();
    if (markIcon !== undefined && this.checked) {
      const glyph = document.createElement('mjx-icon');
      glyph.setAttribute('name', markIcon.name);
      glyph.setAttribute('size', String(markIcon.size));
      mark.append(glyph);
    }

    const iconName = this.getAttribute('icon');
    iconCell.replaceChildren();
    if (iconName !== null && iconName !== '') {
      const glyph = document.createElement('mjx-icon');
      glyph.setAttribute('name', iconName);
      glyph.setAttribute('size', String(menuGlyphSize));
      glyph.setAttribute('variant', 'regular');
      iconCell.append(glyph);
    }

    labelElement.textContent = this.label;
    const description = this.description;
    descriptionElement.textContent = description;
    descriptionElement.hidden = description === '';
    shortcutElement.textContent = shortcut;

    arrow.replaceChildren();
    if (this.#hasSubmenu) {
      const glyph = document.createElement('mjx-icon');
      glyph.setAttribute('name', submenuArrowIcon.name);
      glyph.setAttribute('size', String(submenuArrowIcon.size));
      arrow.append(glyph);
    }

    const reason = this.getAttribute('explanation') ?? '';
    if (this.unavailable && reason.trim() !== '') {
      explanation.textContent = reason;
      // The IDREF resolves inside this shadow root, and the host is the element that carries the
      // role — so the description is attached to `.item`'s tree and referenced from the host by the
      // one mechanism that crosses the boundary: the name computation already walks the shadow
      // tree, so the reason is appended to the label instead of pointing at an id the host cannot
      // see. `title` is the pointer's half of the same answer, exactly as `<mjx-button>` does it.
      this.setAttribute('aria-label', `${this.accessibleName}, ${reason}`);
      this.title = reason;
    } else {
      explanation.textContent = '';
      this.removeAttribute('title');
    }
  }
}

/** Register the element. Idempotent. */
export function defineMenuItem(): void {
  defineIcon();
  if (customElements.get('mjx-menu-item') === undefined) {
    customElements.define('mjx-menu-item', MjxMenuItem);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-menu-item': MjxMenuItem;
  }
}
