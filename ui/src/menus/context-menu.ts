/**
 * `<mjx-context-menu>` — a region that owns a menu, opened the three ways a context menu is opened.
 *
 * ```html
 * <mjx-context-menu>
 *   <div class="canvas">…the thing being acted on…</div>
 *   <mjx-menu slot="menu" label="Slide">
 *     <mjx-menu-item label="Cut" shortcut="Ctrl+X"></mjx-menu-item>
 *   </mjx-menu>
 * </mjx-context-menu>
 * ```
 *
 * ## Why this is the largest surface in the product
 *
 * `docs/client-platform/OFFICE_FEATURE_INVENTORY.md`'s *"None (Context Menu)"* bucket is the
 * biggest single one in every application — 1,155 controls in Word, 1,104 in Excel, 1,660 in
 * PowerPoint. More of Office is reached by right-clicking than by any ribbon tab, so **a weak
 * context menu weakens more of the product than a weak ribbon does**, and the three ways of opening
 * one are three separate features rather than three spellings of the same one:
 *
 * * **The pointer.** `contextmenu`, positioned at the pointer, which is a zero-sized anchor and
 *   needs no special case in `placeFloating`.
 * * **The keyboard.** The `ContextMenu` key, and `Shift + F10` for the many keyboards that do not
 *   have one. Positioned at the **focused element**, not at wherever the pointer happens to be
 *   resting — a keyboard user's context is their focus.
 * * **Touch.** A long press. Not a `contextmenu` event at all on every platform, and cancelled by a
 *   drag, because the same gesture that opens a context menu also starts a scroll and the
 *   difference between them is entirely how far the finger moved.
 *
 * ## The keyboard path is the one that double-fires
 *
 * Pressing the `ContextMenu` key produces a `keydown` **and then** a `contextmenu` event, so a
 * component that handled both opens twice: once at the focused element and once at whatever
 * coordinates the browser synthesised. The keydown is the one worth handling — it is the one that
 * knows what is focused — so the `contextmenu` that follows is ignored for one task.
 *
 * ## Focus goes back where the person left it
 *
 * The element that was focused when the menu opened is remembered and restored on close, which for
 * a right-click is usually the canvas and for the keyboard is the thing being acted on. That is
 * `<mjx-menu>`'s `returnFocusTo`, and it is why this component does not need a focus policy of its
 * own.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { contextMenuCss, longPressDelay, longPressMoveTolerance, menuTags } from './menu-model.ts';
import { MjxMenu } from './menu.ts';

type Timer = ReturnType<typeof setTimeout>;

export class MjxContextMenu extends HTMLElement {
  #root: ShadowRoot | undefined;
  #region: HTMLElement | undefined;
  #longPressTimer: Timer | undefined;
  #longPressOrigin: { x: number; y: number } | undefined;
  #openedByKeyboard = false;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
  }

  disconnectedCallback(): void {
    this.#cancelLongPress();
  }

  /** The menu this region owns, or `undefined` when nobody gave it one. */
  get menu(): MjxMenu | undefined {
    for (const child of this.children) {
      if (child instanceof MjxMenu && child.getAttribute('slot') === 'menu') return child;
    }
    return undefined;
  }

  /** Whether the menu is showing. */
  get open(): boolean {
    return this.menu?.open === true;
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, contextMenuCss);

    const region = document.createElement('div');
    region.className = 'region';
    region.setAttribute('part', 'region');
    region.append(document.createElement('slot'));

    const menuSlot = document.createElement('slot');
    menuSlot.name = 'menu';

    root.append(region, menuSlot);
    this.#region = region;

    this.addEventListener('contextmenu', this.#onContextMenu);
    this.addEventListener('keydown', this.#onKeyDown);
    this.addEventListener('pointerdown', this.#onPointerDown);
    this.addEventListener('pointermove', this.#onPointerMove);
    this.addEventListener('pointerup', this.#cancelLongPress);
    this.addEventListener('pointercancel', this.#cancelLongPress);
  }

  /** Open at a point in viewport coordinates. */
  openAtPoint(point: { x: number; y: number }): void {
    const menu = this.menu;
    if (menu === undefined) return;
    const previous = this.#focusedElement();
    if (menu.open) menu.close('outside', { restoreFocus: false });
    menu.openAt(point, previous === undefined ? {} : { returnFocusTo: previous });
  }

  #focusedElement(): HTMLElement | undefined {
    let active: Element | null = this.ownerDocument.activeElement;
    while (active?.shadowRoot?.activeElement != null) active = active.shadowRoot.activeElement;
    if (!(active instanceof HTMLElement)) return undefined;
    if (active === this.ownerDocument.body) return undefined;
    return active;
  }

  #onContextMenu = (event: MouseEvent): void => {
    // The keyboard already opened this one. See the module note: the `ContextMenu` key fires both.
    if (this.#openedByKeyboard) {
      event.preventDefault();
      return;
    }
    if (this.menu === undefined) return;
    event.preventDefault();
    this.openAtPoint({ x: event.clientX, y: event.clientY });
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    const asked = event.key === 'ContextMenu' || (event.key === 'F10' && event.shiftKey);
    if (!asked) return;
    const menu = this.menu;
    if (menu === undefined) return;
    // A key press inside the open menu belongs to the menu.
    if (event.composedPath().includes(menu)) return;
    event.preventDefault();
    const target = this.#focusedElement() ?? this.#region ?? this;
    this.#openedByKeyboard = true;
    setTimeout(() => {
      this.#openedByKeyboard = false;
    }, 0);
    menu.openFrom(target, { focus: true });
  };

  #onPointerDown = (event: PointerEvent): void => {
    if (event.pointerType !== 'touch') return;
    const menu = this.menu;
    if (menu === undefined) return;
    if (event.composedPath().includes(menu)) return;
    this.#longPressOrigin = { x: event.clientX, y: event.clientY };
    this.#longPressTimer = setTimeout(() => {
      const origin = this.#longPressOrigin;
      this.#cancelLongPress();
      if (origin !== undefined) this.openAtPoint(origin);
    }, longPressDelay);
  };

  #onPointerMove = (event: PointerEvent): void => {
    const origin = this.#longPressOrigin;
    if (origin === undefined) return;
    const moved = Math.hypot(event.clientX - origin.x, event.clientY - origin.y);
    // Past the tolerance the gesture is a scroll, and a scroll that opened a menu would make the
    // content unscrollable on a touch device.
    if (moved > longPressMoveTolerance) this.#cancelLongPress();
  };

  #cancelLongPress = (): void => {
    if (this.#longPressTimer !== undefined) clearTimeout(this.#longPressTimer);
    this.#longPressTimer = undefined;
    this.#longPressOrigin = undefined;
  };
}

/** Register the element. Idempotent. */
export function defineContextMenu(): void {
  if (customElements.get(menuTags.contextMenu) === undefined) {
    customElements.define(menuTags.contextMenu, MjxContextMenu);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-context-menu': MjxContextMenu;
  }
}
