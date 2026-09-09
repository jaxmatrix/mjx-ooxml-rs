/**
 * `<mjx-menu>` — the ARIA menu pattern, the submenu that a pointer can actually reach, and the
 * three presentations a menu takes.
 *
 * ```html
 * <mjx-menu label="Edit">
 *   <mjx-menu-item label="Cut" icon="delete" shortcut="Ctrl+X"></mjx-menu-item>
 *   <mjx-menu-separator></mjx-menu-separator>
 *   <mjx-menu-item label="Paste Options">
 *     <mjx-menu slot="submenu" label="Paste Options">…</mjx-menu>
 *   </mjx-menu-item>
 * </mjx-menu>
 * ```
 *
 * ## A menu that renders is not a menu that behaves
 *
 * MJXOFF-184 is explicit that everything worth having here is **temporal or positional**, and that
 * both are invisible to a static story:
 *
 * > A menu that opens instantly on hover, never flips at an edge, and drops focus on close will
 * > look perfect in a screenshot and be unpleasant to use. **So the gates are interaction tests.**
 *
 * So the parts of this file that matter are the parts a picture cannot show: the key map (which is
 * `menuKeyAction`, in the model, so a Node test can read it), the hover-intent timers, the
 * safe-triangle grace, the placement, and where focus goes on each of the ways a menu can close.
 *
 * ## Trap or disclosure: a menu is a disclosure, and the rule says why
 *
 * `focusManagementPatterns` in the model states it in full. In one line: **the number of tab stops
 * decides.** A menu holds one roving tab stop and moves with arrow keys, so `Tab` is free to mean
 * *leave* — which is exactly what the ARIA menu pattern requires — and a surface whose `Tab` means
 * *leave* closes when focus leaves it. MJXOFF-183's tab picker is the same shape and made the same
 * choice; its collapsed ribbon group traps because its commands are separately tabbable buttons and
 * `Tab` is the only way between them. **The asymmetry stands, and it is not about popups.**
 *
 * ## The safe triangle, and why it is not a hover delay
 *
 * A hover delay does not solve diagonal travel: a pointer moving from *Paste Options* down and to
 * the right toward its submenu passes over *Paste Special* on the way, and a delay only postpones
 * losing the submenu. What is needed is *intent*: while the pointer is inside the triangle whose
 * apex is where it left the parent item and whose base is the submenu's near edge, a sibling it
 * happens to be over is not the item it means.
 *
 * That is `travellingToward` in `src/overlay/floating.ts` — pure, so `tests/menus.test.ts` can put
 * a pointer path through it — and this file arms it as a **grace**: the switch to a sibling is
 * suspended while the pointer is inside the triangle, released the moment it leaves, and released
 * anyway after `submenuHoverCloseDelay` so a pointer that stops half way still ends up somewhere.
 * `tests/browser/menus.spec.ts` drives a real diagonal across a real sibling, and proves the gate
 * can fail by taking the tolerance away.
 *
 * **The keyboard never waits for any of it.** `ArrowRight` opens the submenu and moves into it in
 * the same task, with no timer involved; that is asserted separately, because a keyboard path that
 * inherited a 300ms hover delay is the classic way this goes wrong.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  applyPlacement,
  clearPlacement,
  clippingAncestor,
  clippingBoundary,
  floatingCss,
  floatingProperties,
  installFloatingProperties,
  physicalSide,
  pinFloating,
  placeFloating,
  rectOf,
  resolveLength,
  travellingToward,
  isAlign,
  isLogicalSide,
  type Align,
  type Direction,
  type LogicalSide,
  type Placement,
  type Rect,
} from '../overlay/floating.ts';
import {
  checkableMenuItemKinds,
  defaultMenuAlign,
  defaultMenuSide,
  floatingAttribute,
  menuBoxProperties,
  menuEvents,
  menuGutterColumn,
  menuKeyAction,
  menuMotionClass,
  menuPresentationProperty,
  menuPresentationOrder,
  menuPresentations,
  menuSheet,
  menuTags,
  sheetBoundaryFraction,
  nextTypeaheadIndex,
  openAttribute,
  submenuAlign,
  submenuHoverCloseDelay,
  submenuHoverOpenDelay,
  submenuSide,
  submenuTravelGrace,
  typeaheadResetDelay,
  type MenuCloseReason,
  type MenuPresentation,
} from './menu-model.ts';
import type { MjxMenuItem } from './menu-item.ts';

type Timer = ReturnType<typeof setTimeout>;

interface Point {
  readonly x: number;
  readonly y: number;
}

interface Grace {
  readonly apex: Point;
  candidate: MjxMenuItem;
  timer: Timer;
}

export class MjxMenu extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'placement', 'align', 'open'];

  #root: ShadowRoot | undefined;
  #menu: HTMLElement | undefined;
  #activeIndex = 0;
  #invoker: HTMLElement | undefined;
  #anchor: Rect | undefined;
  /** Whether the anchor came from an element, and may therefore be re-read when things move. */
  #anchorFollowsInvoker = false;
  #placement: Placement | undefined;
  #natural: { width: number; height: number } | undefined;
  #openSubmenuItem: MjxMenuItem | undefined;
  #hoverOpenTimer: Timer | undefined;
  #hoverCloseTimer: Timer | undefined;
  #typeaheadTimer: Timer | undefined;
  #typeahead = '';
  #pointer: Point = { x: 0, y: 0 };
  #exitPoint: Point | undefined;
  #grace: Grace | undefined;
  #watching = false;
  #dismissing = false;
  #observer: ResizeObserver | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    // The rows are in the light DOM and wear the foundations' focus ring, and `@property`
    // registration is document-scoped — so both sheets go on the document as well as on this root.
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.render();
  }

  disconnectedCallback(): void {
    this.#stopWatching();
    this.#clearHoverTimers();
    this.#clearGrace();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The menu's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** Whether a popup menu is showing. Meaningless on an inline one. */
  get open(): boolean {
    return this.hasAttribute('open');
  }

  /** The item that owns this menu, when it is a submenu. */
  get parentItem(): MjxMenuItem | undefined {
    const parent = this.parentElement;
    if (parent !== null && parent.localName === menuTags.item) return parent as MjxMenuItem;
    return undefined;
  }

  /** Whether this menu is a submenu, and therefore has a level for Escape to close. */
  get isSubmenu(): boolean {
    return this.parentItem !== undefined;
  }

  /** Whether this menu is acting as a popup rather than sitting in flow. */
  get floating(): boolean {
    return this.isSubmenu || this.hasAttribute('floating');
  }

  /**
   * **Which presentation CSS put this menu in** — read back, never computed here.
   *
   * `src/menus/menu-model.ts`'s `menuPresentationAt()` says what it *should* be, and the browser
   * gate compares the two. A component that decided its own presentation and then reported it
   * would be grading its own homework, which is the mistake `<mjx-ribbon-group>` was written to
   * avoid and this inherits.
   */
  get presentation(): MenuPresentation {
    const menu = this.#menu;
    if (menu === undefined) return 'inline';
    const value = getComputedStyle(menu).getPropertyValue(menuPresentationProperty).trim();
    return value === 'floating' || value === 'sheet' ? value : 'inline';
  }

  /** The writing direction this menu resolves its logical sides against. */
  get direction(): Direction {
    return getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  /** The side it prefers to open on. A submenu opens toward the end of the line. */
  get side(): LogicalSide {
    const declared = this.getAttribute('placement');
    if (isLogicalSide(declared)) return declared;
    return this.isSubmenu ? submenuSide : defaultMenuSide;
  }

  /** How it lines up with its anchor on the cross axis. */
  get align(): Align {
    const declared = this.getAttribute('align');
    if (isAlign(declared)) return declared;
    return this.isSubmenu ? submenuAlign : defaultMenuAlign;
  }

  /** The placement the last open produced, for a gate to compare against `placeFloating`. */
  get placement(): Placement | undefined {
    return this.#placement;
  }

  /**
   * The anchor the last open was placed against, and the rectangle it had to stay inside.
   *
   * Both are exposed because the browser gate re-runs `placeFloating()` **in Node** over exactly
   * these inputs and requires the same answer. A gate that could only see the result would be
   * asserting that the menu is somewhere plausible; with the inputs it asserts that the component
   * and the model agree, which is the only comparison that can catch a placement that is wrong in
   * both places at once.
   */
  get anchorRect(): Rect | undefined {
    return this.#anchor;
  }

  get boundaryRect(): Rect | undefined {
    const menu = this.#menu;
    if (menu === undefined) return undefined;
    return clippingBoundary(menu, resolveLength(menu, floatingProperties.boundaryInset));
  }

  /** The natural size the last placement was computed for. */
  get naturalSize(): { readonly width: number; readonly height: number } | undefined {
    return this.#natural;
  }

  /**
   * Every item this menu owns, in document order, walking **through** a section and **not** into a
   * submenu.
   *
   * Matched by tag name rather than `instanceof`: a menu's `connectedCallback` can run before its
   * children have been upgraded, and an `instanceof` that ran too early would report a menu with no
   * items — which reads exactly like an empty menu and is why the count is asserted by the gate.
   */
  get items(): MjxMenuItem[] {
    const found: MjxMenuItem[] = [];
    const walk = (node: Element): void => {
      for (const child of node.children) {
        if (child.localName === menuTags.item) {
          found.push(child as MjxMenuItem);
          continue;
        }
        if (child.localName === menuTags.section) walk(child);
      }
    };
    walk(this);
    return found;
  }

  /** Which item holds the roving tab stop. */
  get activeIndex(): number {
    return this.#activeIndex;
  }

  /** The item the keyboard is on, or `undefined`. */
  get activeItem(): MjxMenuItem | undefined {
    return this.items[this.#activeIndex];
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, [floatingCss, menuSheet].join('\n'));

    const menu = document.createElement('div');
    menu.className = 'menu';
    menu.setAttribute('part', 'menu');
    menu.setAttribute('role', 'menu');
    menu.setAttribute('aria-orientation', 'vertical');

    const slot = document.createElement('slot');
    slot.addEventListener('slotchange', this.#onSlotChange);
    menu.append(slot);
    root.append(menu);
    this.#menu = menu;

    this.addEventListener('keydown', this.#onKeyDown);
    this.addEventListener('click', this.#onClick);
    this.addEventListener('pointerover', this.#onPointerOver);
    this.addEventListener('pointermove', this.#onPointerMove);
    this.addEventListener('focusout', this.#onFocusOut);
  }

  /** Re-render from the attributes. */
  render(): void {
    const menu = this.#menu;
    if (menu === undefined) return;

    menu.setAttribute('aria-label', this.label);
    const floating = this.floating;
    if (floating) menu.setAttribute(floatingAttribute, '');
    else menu.removeAttribute(floatingAttribute);
    menu.setAttribute(openAttribute, floating && this.open ? 'true' : 'false');
    this.#syncTopLayer(menu, floating);

    this.#syncGutters();
    this.#syncTabStops();
  }

  /**
   * Put a floating menu in the **top layer**, and take an inline one out of it.
   *
   * `menu-model.ts` records the measurement this exists for: a submenu is a descendant of the menu
   * that opened it, that menu clips its own list, and a clipped submenu has a correct rectangle,
   * no pixels and no pointer. The top layer is the platform's answer, and `manual` is the right
   * kind: dismissal, nesting and focus are this component's own, and an `auto` popover would close
   * a parent the moment its child opened.
   *
   * Feature-detected rather than assumed. Where the API is missing the menu still opens, still
   * places itself and still announces itself — it is only a submenu inside a *scrolling* menu that
   * degrades, and degrading is better than throwing.
   */
  #syncTopLayer(menu: HTMLElement, floating: boolean): void {
    const supported = typeof menu.showPopover === 'function';
    if (!supported) return;
    if (!floating) {
      if (menu.hasAttribute('popover')) {
        if (menu.matches(':popover-open')) menu.hidePopover();
        menu.removeAttribute('popover');
      }
      return;
    }
    if (menu.getAttribute('popover') !== 'manual') menu.setAttribute('popover', 'manual');
    if (!this.isConnected) return;
    const showing = menu.matches(':popover-open');
    if (this.open && !showing) menu.showPopover();
    else if (!this.open && showing) menu.hidePopover();
  }

  // ── the gutters ────────────────────────────────────────────────────────────

  /**
   * Reserve the mark and icon columns, once, for the whole menu.
   *
   * A row cannot decide its own gutter: if each row sized its own, the labels of a menu whose
   * third item is the only checkable one would not line up, which is the single most visible way a
   * hand-built menu looks hand-built. Custom properties inherit through the flat tree, so a value
   * set here on `.menu` reaches every slotted row's shadow root — the same channel MJXOFF-183's
   * control levers use, and the only one that crosses a shadow boundary at all.
   */
  #syncGutters(): void {
    const menu = this.#menu;
    if (menu === undefined) return;
    const items = this.items;
    const checkable = items.some((item) =>
      (checkableMenuItemKinds as readonly string[]).includes(item.getAttribute('kind') ?? ''),
    );
    const iconed = items.some((item) => (item.getAttribute('icon') ?? '') !== '');
    menu.style.setProperty(menuBoxProperties.markColumn, checkable ? menuGutterColumn : '0px');
    menu.style.setProperty(menuBoxProperties.iconColumn, iconed ? menuGutterColumn : '0px');
  }

  #onSlotChange = (): void => {
    this.#syncGutters();
    this.#syncTabStops();
  };

  // ── the roving tab stop ────────────────────────────────────────────────────

  #syncTabStops(): void {
    const items = this.items;
    if (items.length === 0) return;
    if (this.#activeIndex >= items.length) this.#activeIndex = 0;
    for (const [index, item] of items.entries()) {
      item.tabIndex = index === this.#activeIndex ? 0 : -1;
    }
  }

  /**
   * Move the keyboard onto an item.
   *
   * **The tab stop is set before the focus is moved**, and the order is load-bearing: the
   * foundations' focus ring matches `[tabindex]:not([tabindex="-1"])`, so focusing an item that
   * still held `-1` would put the keyboard somewhere with no ring on it.
   */
  focusItem(index: number): void {
    const items = this.items;
    if (items.length === 0) return;
    const wrapped = ((index % items.length) + items.length) % items.length;
    this.#activeIndex = wrapped;
    this.#syncTabStops();
    const item = items[wrapped];
    if (item === undefined) return;
    // ⚠ `preventScroll`, and then the menu scrolls *itself*. The browser's own scroll-into-view
    // walks every ancestor scroll container, and a menu's ancestors include the parent menu it was
    // opened from — which is a scroll container and whose scrollport a `position: fixed` submenu is
    // outside of. Left to itself the browser therefore scrolls the *parent* menu to reveal the
    // submenu, moving every row of it while the fixed submenu stays where it was put. Revealing the
    // row inside this menu's own box is the whole of what was wanted, and it touches nothing else.
    item.focus({ preventScroll: true });
    this.#revealItem(item);
  }

  #revealItem(item: HTMLElement): void {
    const menu = this.#menu;
    if (menu === undefined || menu.scrollHeight <= menu.clientHeight) return;
    const row = item.getBoundingClientRect();
    const box = menu.getBoundingClientRect();
    if (row.top < box.top) menu.scrollTop -= box.top - row.top;
    else if (row.bottom > box.bottom) menu.scrollTop += row.bottom - box.bottom;
  }

  // ── the keyboard ───────────────────────────────────────────────────────────

  #onKeyDown = (event: KeyboardEvent): void => {
    const item = this.#ownItemIn(event.composedPath());
    if (item === undefined) return;

    const action = menuKeyAction(event.key, {
      direction: this.direction,
      hasSubmenu: item.hasSubmenu,
      isSubmenu: this.isSubmenu,
      typing: this.#typeahead !== '',
    });
    if (action === undefined) return;

    const items = this.items;
    const index = items.indexOf(item);

    switch (action) {
      case 'next':
        this.focusItem(index + 1);
        break;
      case 'previous':
        this.focusItem(index - 1);
        break;
      case 'first':
        this.focusItem(0);
        break;
      case 'last':
        this.focusItem(items.length - 1);
        break;
      case 'activate':
        this.activate(item);
        break;
      case 'openSubmenu':
        // No timer, no intent, no delay. See the module note: a keyboard that inherited the hover
        // delay is the defect this is separated to prevent.
        this.#openSubmenu(item, 'keyboard', { focus: true });
        break;
      case 'closeSubmenu':
        this.close('escape');
        break;
      case 'close':
        this.close('escape');
        break;
      case 'leave':
        this.#closeChain('tab');
        // No preventDefault: the browser's own sequential navigation continues from the invoker,
        // which is where a person expects `Tab` out of a menu to leave them.
        return;
      case 'typeahead':
        this.#type(event.key);
        break;
    }

    event.preventDefault();
    // The innermost menu that acted is the only one that should. Without this, ArrowLeft in a
    // submenu would close the submenu *and* be seen by its parent.
    event.stopPropagation();
  };

  #type(character: string): void {
    this.#typeahead += character;
    if (this.#typeaheadTimer !== undefined) clearTimeout(this.#typeaheadTimer);
    this.#typeaheadTimer = setTimeout(() => {
      this.#typeahead = '';
      this.#typeaheadTimer = undefined;
    }, typeaheadResetDelay);

    const items = this.items;
    const found = nextTypeaheadIndex(
      items.map((item) => item.label),
      this.#typeahead,
      this.#activeIndex,
    );
    if (found >= 0) this.focusItem(found);
  }

  // ── activation ─────────────────────────────────────────────────────────────

  /**
   * Activate an item — or refuse to.
   *
   * An unavailable item is refused **silently and completely**: no event, no state change, no
   * close. It is still focused and still announced, which is the whole point of the state; what it
   * is not, is a command that half happened.
   */
  activate(item: MjxMenuItem): void {
    if (item.unavailable) return;
    if (item.hasSubmenu) {
      this.#openSubmenu(item, 'keyboard', { focus: true });
      return;
    }

    const kind = item.kind;
    if (kind === 'checkbox') item.checked = !item.checked;
    if (kind === 'radio') {
      // The group is the enclosing section when there is one, and the menu otherwise — which is the
      // difference `<mjx-menu-section>` exists to make.
      const scope = item.closest(menuTags.section) ?? this;
      for (const sibling of this.items) {
        if (sibling.getAttribute('kind') !== 'radio') continue;
        if ((sibling.closest(menuTags.section) ?? this) !== scope) continue;
        sibling.checked = sibling === item;
      }
    }

    this.dispatchEvent(
      new CustomEvent(menuEvents.activate, {
        bubbles: true,
        composed: true,
        detail: { value: item.value, label: item.label, kind, checked: item.checked },
      }),
    );
    this.#closeChain('activate');
  }

  #onClick = (event: MouseEvent): void => {
    const item = this.#ownItemIn(event.composedPath());
    if (item === undefined) return;
    event.preventDefault();
    event.stopPropagation();
    this.activate(item);
  };

  // ── the pointer ────────────────────────────────────────────────────────────

  #onPointerMove = (event: PointerEvent): void => {
    const point = { x: event.clientX, y: event.clientY };
    this.#pointer = point;

    const grace = this.#grace;
    if (grace !== undefined) {
      if (point.x === grace.apex.x && point.y === grace.apex.y) return;
      if (this.#travelling(point, grace.apex)) {
        // Still on its way, so the clock starts again. The timeout is for a pointer that has
        // **stopped** inside the triangle, not for one that is taking its time crossing it — a
        // fixed deadline turns a slow diagonal into a closed submenu, which is the defect the
        // tolerance exists to prevent, arriving 300 milliseconds late.
        clearTimeout(grace.timer);
        grace.timer = setTimeout(() => this.#releaseGrace(), submenuHoverCloseDelay);
        return;
      }
      this.#releaseGrace();
      return;
    }

    const item = this.#ownItemIn(event.composedPath());
    // The last position the pointer held on the item whose submenu is open **is** the point it
    // left from, and it is the apex the triangle has to be drawn from. Sampling "the previous
    // pointermove" instead gives two adjacent pixels and a triangle with no direction in it.
    if (item !== undefined && item === this.#openSubmenuItem) this.#exitPoint = point;
  };

  #onPointerOver = (event: PointerEvent): void => {
    const path = event.composedPath();
    const openItem = this.#openSubmenuItem;
    const submenu = openItem?.submenu;

    // The pointer arrived in the open submenu: there is nothing left to protect, **and there is a
    // close timer to cancel**. Without the second half a submenu closes 300ms after the pointer
    // reaches it, whenever the pointer was over a sibling on the way — which is the same defect the
    // safe triangle prevents, arriving by the other door.
    if (submenu !== undefined && path.includes(submenu)) {
      this.#clearGrace();
      this.#clearHoverTimers();
      return;
    }

    const item = this.#ownItemIn(path);
    if (item === undefined) return;

    const grace = this.#grace;
    if (grace !== undefined) {
      grace.candidate = item;
      return;
    }

    if (openItem !== undefined && openItem !== item && this.#submenuRect() !== undefined) {
      this.#armGrace(item);
      return;
    }

    this.#highlight(item);
  };

  #armGrace(candidate: MjxMenuItem): void {
    const apex = this.#exitPoint ?? this.#pointer;
    const timer = setTimeout(() => this.#releaseGrace(), submenuHoverCloseDelay);
    this.#grace = { apex, candidate, timer };
  }

  #releaseGrace(): void {
    const grace = this.#grace;
    this.#clearGrace();
    if (grace !== undefined) this.#highlight(grace.candidate);
  }

  #clearGrace(): void {
    if (this.#grace !== undefined) clearTimeout(this.#grace.timer);
    this.#grace = undefined;
  }

  #submenuRect(): Rect | undefined {
    const submenu = this.#openSubmenuItem?.submenu;
    const box = submenu?.shadowRoot?.querySelector('.menu');
    if (!(box instanceof HTMLElement)) return undefined;
    const rect = rectOf(box);
    return rect.width > 0 && rect.height > 0 ? rect : undefined;
  }

  #travelling(point: Point, apex: Point): boolean {
    const rect = this.#submenuRect();
    if (rect === undefined) return false;
    const submenu = this.#openSubmenuItem?.submenu;
    const side =
      submenu instanceof MjxMenu
        ? (submenu.placement?.side ?? physicalSide(submenu.side, submenu.direction))
        : physicalSide(submenuSide, this.direction);
    return travellingToward(point, apex, rect, side, submenuTravelGrace);
  }

  /** Move the keyboard onto an item the pointer is over, and arm the hover intent. */
  #highlight(item: MjxMenuItem): void {
    const index = this.items.indexOf(item);
    if (index < 0) return;
    if (index !== this.#activeIndex || this.ownerDocument.activeElement !== item) {
      this.focusItem(index);
    }

    this.#clearHoverTimers();
    const open = this.#openSubmenuItem;
    if (open !== undefined && open !== item) {
      this.#hoverCloseTimer = setTimeout(() => {
        this.#closeSubmenu();
      }, submenuHoverCloseDelay);
    }
    if (item.hasSubmenu && open !== item) {
      this.#hoverOpenTimer = setTimeout(() => {
        this.#openSubmenu(item, 'pointer', { focus: false });
      }, submenuHoverOpenDelay);
    }
  }

  #clearHoverTimers(): void {
    if (this.#hoverOpenTimer !== undefined) clearTimeout(this.#hoverOpenTimer);
    if (this.#hoverCloseTimer !== undefined) clearTimeout(this.#hoverCloseTimer);
    this.#hoverOpenTimer = undefined;
    this.#hoverCloseTimer = undefined;
  }

  // ── submenus ───────────────────────────────────────────────────────────────

  #openSubmenu(item: MjxMenuItem, by: 'keyboard' | 'pointer', options: { focus: boolean }): void {
    const submenu = item.submenu;
    if (!(submenu instanceof MjxMenu)) return;
    if (this.#openSubmenuItem !== undefined && this.#openSubmenuItem !== item) this.#closeSubmenu();
    this.#clearHoverTimers();
    this.#openSubmenuItem = item;
    item.submenuOpen = true;
    submenu.openFrom(item, { focus: options.focus });
    this.dispatchEvent(
      new CustomEvent(menuEvents.submenuToggle, {
        bubbles: true,
        composed: true,
        detail: { open: true, by, label: item.label },
      }),
    );
  }

  #closeSubmenu(): void {
    const item = this.#openSubmenuItem;
    if (item === undefined) return;
    this.#openSubmenuItem = undefined;
    item.submenuOpen = false;
    const submenu = item.submenu;
    if (submenu instanceof MjxMenu) submenu.close('blur', { restoreFocus: false });
    this.dispatchEvent(
      new CustomEvent(menuEvents.submenuToggle, {
        bubbles: true,
        composed: true,
        detail: { open: false, label: item.label },
      }),
    );
  }

  // ── opening and closing ────────────────────────────────────────────────────

  /** Open beside an element, and remember it as the thing focus goes back to. */
  openFrom(invoker: HTMLElement, options: { focus?: boolean } = {}): void {
    this.#invoker = invoker;
    this.#anchor = rectOf(invoker);
    this.#anchorFollowsInvoker = true;
    this.#show(options.focus !== false);
  }

  /**
   * Open at a point — a right-click, or a long press.
   *
   * The anchor is a zero-sized rectangle at the pointer, which is exactly what a context menu's
   * anchor is, and it means `placeFloating` needs no second code path for it.
   */
  openAt(point: Point, options: { focus?: boolean; returnFocusTo?: HTMLElement } = {}): void {
    this.#invoker = options.returnFocusTo;
    this.#anchor = { x: point.x, y: point.y, width: 0, height: 0 };
    // ⚠ The invoker is where focus goes back to and is **not** the anchor. A right-click on a
    // canvas remembers the canvas so Escape lands there, and anchors on the pointer — and a
    // reposition that re-read the invoker would quietly replace the point with the canvas's whole
    // box the first time the container resized.
    this.#anchorFollowsInvoker = false;
    this.#show(options.focus !== false);
  }

  #show(focus: boolean): void {
    const menu = this.#menu;
    if (menu === undefined) return;
    if (!this.hasAttribute('floating') && !this.isSubmenu) this.setAttribute('floating', '');
    this.setAttribute('open', '');
    this.render();

    // Placement first, entry second: the entry translates the box, and a rect measured while it is
    // translated is a rect that places the menu one animation away from where it belongs.
    this.#place();

    const presentation = this.presentation;
    for (const name of menuPresentationOrder) menu.classList.remove(menuMotionClass(name));
    menu.classList.add(menuMotionClass(presentation));
    menu.dataset['entering'] = '';
    requestAnimationFrame(() => {
      delete menu.dataset['entering'];
      // A second pass, once. The first placement measures a box the browser has only just been
      // asked to lay out, and anything that settles a frame late — an icon that had no box while
      // the menu was `display: none`, a gutter column applied in the same task — changes the size
      // the placement was computed for. Re-placing after the entry frame costs one reflow and
      // removes a whole class of *nearly* right positions. It runs after the entering class is
      // gone, because that class translates the box and a rect read through it is a rect of where
      // the menu is arriving from rather than where it is.
      this.#place();
    });

    this.#startWatching();
    this.dispatchEvent(
      new CustomEvent(menuEvents.open, {
        bubbles: true,
        composed: true,
        detail: { presentation, label: this.label },
      }),
    );

    if (focus) this.focusItem(0);
  }

  /**
   * Close, and put focus back where it came from.
   *
   * `menuCloseReasons` and `focusRestoringCloseReasons` say which reasons restore. The three that
   * do are the three MJXOFF-184 names — *"asserted for Escape, for selection and for
   * outside-click"* — and `blur` deliberately does not, because focus has already gone somewhere a
   * person chose and yanking it back is worse than not restoring it at all.
   */
  close(reason: MenuCloseReason, options: { restoreFocus?: boolean } = {}): void {
    if (!this.open) return;
    this.#closeSubmenu();
    this.#clearHoverTimers();
    this.#clearGrace();
    this.removeAttribute('open');
    this.#stopWatching();
    const menu = this.#menu;
    if (menu !== undefined) {
      clearPlacement(menu);
      delete menu.dataset['entering'];
    }
    this.#placement = undefined;
    this.render();

    // A submenu that closed itself has to say so, or its owner keeps believing it is showing.
    this.ownerMenu?.releaseSubmenu();

    this.dispatchEvent(
      new CustomEvent(menuEvents.close, {
        bubbles: true,
        composed: true,
        detail: { reason },
      }),
    );

    const restore = options.restoreFocus ?? reason !== 'blur';
    if (restore) {
      const target = this.#invoker ?? this.parentItem;
      target?.focus();
    }
  }

  /** The menu one level up, when this is a submenu. */
  get ownerMenu(): MjxMenu | undefined {
    const owner = this.parentItem?.closest(menuTags.menu);
    return owner instanceof MjxMenu ? owner : undefined;
  }

  /**
   * Told by a submenu that it closed itself.
   *
   * Escape in a submenu closes **one level**, and the level it closed is the one this menu is
   * holding open. Without this the parent would still believe the submenu was showing: its item
   * would keep the highlighted paint, `aria-expanded` would stay `true`, and the next hover would
   * try to close a submenu that had already gone.
   */
  releaseSubmenu(): void {
    const item = this.#openSubmenuItem;
    if (item === undefined) return;
    this.#openSubmenuItem = undefined;
    item.submenuOpen = false;
  }

  /** Close this menu and every menu above it — what `Tab` and an outside click do. */
  #closeChain(reason: MenuCloseReason): void {
    const chain: MjxMenu[] = [this];
    for (let above = this.ownerMenu; above !== undefined; above = above.ownerMenu) {
      chain.push(above);
    }
    // Deepest first, so a parent's `#closeSubmenu` never fights a child that is already gone.
    for (const level of chain) level.close(reason, { restoreFocus: false });
    const target = chain[chain.length - 1]?.invokerElement;
    if (target === undefined) return;

    if (reason !== 'outside') {
      target.focus();
      return;
    }

    /*
     * An outside click is the one close whose focus restore has to wait.
     *
     * `pointerdown` runs **before** the browser's own focus handling, so focusing the invoker here
     * would be undone a moment later by `mousedown` moving focus to whatever was clicked. And
     * `preventDefault()` on the pointer event is not the answer — it suppresses the compatibility
     * mouse events and then the click never lands at all.
     *
     * So the restore is queued behind the browser's own focus change. It is **unconditional**,
     * which is a decision rather than an oversight: MJXOFF-184 requires focus to return to the
     * invoker *"for Escape, for selection and for outside-click"*, and a conditional restore would
     * be silently wrong in this catalogue anyway — `<mjx-resizable-container>`'s stage carries
     * `tabindex="0"` because a scroll container a keyboard cannot reach fails axe, so "the click
     * landed on nothing focusable" is never true here.
     */
    setTimeout(() => {
      target.focus();
    }, 0);
  }

  /** The element focus goes back to. Exposed so a gate can name it rather than infer it. */
  get invokerElement(): HTMLElement | undefined {
    return this.#invoker;
  }

  // ── placement ──────────────────────────────────────────────────────────────

  /**
   * Put the box where the geometry says, or leave it entirely to CSS.
   *
   * A **sheet is not placed**. It is pinned to the block-end of whatever clips it by `inset`, at
   * full width, and running the placement arithmetic over it would write inline coordinates that
   * beat the stylesheet — inline styles win — and produce a sheet floating in the middle of the
   * screen. So the presentation is read back first and the coordinates are *cleared* when it says
   * `sheet`, which is also what makes a container resize across the boundary work in both
   * directions.
   */
  #place(): void {
    const menu = this.#menu;
    const anchor = this.#anchor;
    if (menu === undefined || anchor === undefined) return;

    const presentation = this.presentation;
    if (presentation === 'inline') {
      clearPlacement(menu);
      this.#placement = undefined;
      this.#natural = undefined;
      return;
    }

    // Measured with no cap on it, so the natural size is what is being placed.
    clearPlacement(menu);
    if (!menuPresentations[presentation].anchored) {
      // A sheet: pinned to one edge of the boundary at its full width. The inset is deliberately
      // zero — a sheet sits *on* the edge it is pinned to, which is the whole of what makes it a
      // sheet rather than a very wide menu.
      const box = clippingBoundary(menu, 0);
      this.#natural = { width: box.width, height: menu.getBoundingClientRect().height };
      this.#placement = pinFloating(menu, box, sheetBoundaryFraction);
      return;
    }
    const natural = menu.getBoundingClientRect();
    this.#natural = { width: natural.width, height: natural.height };
    const inset = resolveLength(menu, floatingProperties.boundaryInset);
    const gap = resolveLength(menu, floatingProperties.gap);
    const placement = placeFloating({
      anchor,
      floating: { width: natural.width, height: natural.height },
      boundary: clippingBoundary(menu, inset),
      side: this.side,
      align: this.align,
      direction: this.direction,
      gap,
    });
    applyPlacement(menu, placement);
    this.#placement = placement;
  }

  /** Re-run the placement. Called when the thing that clips the menu changes size. */
  reposition(): void {
    if (!this.open) return;
    const anchor = this.#invoker;
    if (this.#anchorFollowsInvoker && anchor !== undefined) this.#anchor = rectOf(anchor);
    this.#place();
  }

  // ── dismissal ──────────────────────────────────────────────────────────────

  #startWatching(): void {
    if (this.#watching) return;
    this.#watching = true;
    const document_ = this.ownerDocument;
    document_.addEventListener('pointerdown', this.#onDocumentPointerDown, true);
    document_.addEventListener('scroll', this.#onDocumentScroll, true);
    const observed = clippingAncestor(this.#menu ?? this) ?? document_.documentElement;
    if (typeof ResizeObserver !== 'undefined') {
      this.#observer = new ResizeObserver(() => {
        this.reposition();
      });
      this.#observer.observe(observed);
    }
  }

  #stopWatching(): void {
    if (!this.#watching) return;
    this.#watching = false;
    const document_ = this.ownerDocument;
    document_.removeEventListener('pointerdown', this.#onDocumentPointerDown, true);
    document_.removeEventListener('scroll', this.#onDocumentScroll, true);
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  #onDocumentPointerDown = (event: Event): void => {
    if (this.isSubmenu) return;
    const path = event.composedPath();
    if (path.includes(this)) return;
    const invoker = this.#invoker;
    if (invoker !== undefined && path.includes(invoker)) return;
    this.#dismissing = true;
    this.#closeChain('outside');
    queueMicrotask(() => {
      this.#dismissing = false;
    });
  };

  #onDocumentScroll = (event: Event): void => {
    // A menu that scrolled its own list is not a page that scrolled underneath it.
    if (event.composedPath().includes(this)) return;
    if (this.isSubmenu) return;
    this.#closeChain('blur');
  };

  /**
   * The disclosure half of the focus model.
   *
   * A menu holds one tab stop, so it closes when focus leaves it — which is what makes `Tab` able
   * to mean *leave* rather than *cycle*. A submenu's focus staying inside its parent is automatic:
   * a submenu is a DOM descendant of the item that owns it.
   */
  #onFocusOut = (event: FocusEvent): void => {
    if (!this.open || this.#dismissing || this.isSubmenu) return;
    const next = event.relatedTarget;
    if (next instanceof Node && next !== this && this.contains(next)) return;
    if (next instanceof Node && this.#invoker !== undefined && this.#invoker.contains(next)) return;

    /*
     * ⚠ Deferred, and re-checked against where focus actually went.
     *
     * `relatedTarget` is `null` for the commonest movement inside a menu system: a submenu closing
     * hides the row that held focus, focus falls to the document, and the submenu *then* puts it
     * back on the row that opened it. Acting on that `null` synchronously closes the whole menu
     * one Escape early — MJXOFF-184 watched Arrow Left in an Arabic submenu take the entire menu
     * down with it. A microtask later the restore has happened and the question answers itself.
     */
    queueMicrotask(() => {
      if (!this.open || this.#dismissing) return;
      const active = deepActiveElement(this.ownerDocument);
      if (active === null) return;
      if (this.contains(active)) return;
      if (this.#invoker?.contains(active) === true) return;
      this.close('blur');
    });
  };

  // ── internals ──────────────────────────────────────────────────────────────

  /**
   * The item in an event's path that belongs to **this** menu, or `undefined`.
   *
   * The `undefined` for a foreign item is what keeps a submenu's events from being handled twice:
   * an event from a submenu row reaches that row first, and this menu sees a row it does not own
   * and declines rather than falling through to the parent item that happens to be further up the
   * same path.
   */
  #ownItemIn(path: readonly EventTarget[]): MjxMenuItem | undefined {
    const own = this.items;
    for (const node of path) {
      if (node === this) break;
      if (node instanceof HTMLElement && node.localName === menuTags.item) {
        const item = node as MjxMenuItem;
        return own.includes(item) ? item : undefined;
      }
    }
    return undefined;
  }
}

/** The focused element, followed through every shadow root it is hiding in. */
function deepActiveElement(document_: Document): Element | null {
  let element: Element | null = document_.activeElement;
  while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
  if (element === document_.body || element === document_.documentElement) return null;
  return element;
}

/** Register the element. Idempotent. */
export function defineMenu(): void {
  defineIcon();
  if (customElements.get('mjx-menu') === undefined) customElements.define('mjx-menu', MjxMenu);
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-menu': MjxMenu;
  }
}
