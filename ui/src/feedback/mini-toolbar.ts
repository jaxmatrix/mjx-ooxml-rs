/**
 * `<mjx-mini-toolbar>` — **the commands that appear beside a selection, and never on top of it.**
 *
 * ```html
 * <p id="run">…the selected run…</p>
 * <mjx-mini-toolbar label="Formatting" for="run" open></mjx-mini-toolbar>
 * ```
 * ```ts
 * toolbar.commands = [
 *   { command: 'bold', label: 'Bold', icon: 'text-bold', kind: 'toggle', pressed: true },
 *   { command: 'italic', label: 'Italic', icon: 'text-italic', kind: 'toggle' },
 * ];
 * ```
 *
 * ## The whole contract is spatial, and it is not what `placeFloating` gives you
 *
 * MJXOFF-189 states it: *"A mini toolbar appears near a selection and must not cover it."* That is
 * one sentence and two separate things, and the second is the one a placement primitive does not
 * hand you for free. `placeFloating` puts a box beside its anchor with a gap and then **clamps** it
 * inside the boundary — so a toolbar that fits neither above nor below its selection is pushed back
 * *onto* the selection rather than off the frame. For a menu hanging off a button that is right.
 * For a toolbar whose entire purpose is to act on the thing underneath it, it is the defect.
 *
 * So this uses `placeClearOfAnchor`, which asks the same primitive once per side in
 * `miniToolbarClearanceOrder` and takes the first placement that overlaps the selection by
 * **nothing at all**. Above first, because that is where Office puts it and because a toolbar below
 * a selection sits over the next line a person is about to read; then below; then the two inline
 * sides, which are what a tall selection needs.
 *
 * ⚠ **And when there is no clear side, it says so.** A selection that fills its room leaves nowhere
 * to go, and the honest answer is a reported failure rather than a silently-least-bad placement: the
 * host gets `data-covering="true"`, `clearance.clear` is `false`, and the catalogue ships
 * *A Selection With No Room* precisely so a gate can watch that happen. **A promise whose failure
 * branch no story can reach is a promise no gate is testing** — which is the lesson every child
 * since U06 has contributed a variation of.
 *
 * ## It is a toolbar, so it holds one tab stop
 *
 * WAI's toolbar pattern, and U05's rule arrives at the same place from the other direction: a
 * roving `tabindex` means `Tab` enters the group once and leaves it once, so `Tab` is still free to
 * mean *leave* and the arrows do the moving. The buttons are this component's own, in its own
 * shadow root, which is what makes the roving possible at all — a slotted `<mjx-button>` keeps its
 * inner native button in the sequential order whatever `tabindex` the host carries, so a toolbar
 * built out of slotted controls would have had one tab stop per command and no way to fix it.
 *
 * Their **paint** is not this component's own: `controlStatesCss('.command')` is the same ten-state
 * table the ribbon's controls wear. A mini toolbar with its own hover colour is how a design system
 * acquires two hover colours.
 *
 * ## Escape leaves the caret alone
 *
 * A mini toolbar hangs off a *selection*, and a person who presses Escape without having Tabbed
 * into it has not moved the keyboard anywhere — so putting focus somewhere would be this component
 * inventing a movement. Focus is returned only when it is actually inside the toolbar, which is the
 * same rule `<mjx-screentip>` follows and for the same reason.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  applyPlacement,
  clearPlacement,
  clippingAncestor,
  clippingBoundary,
  floatingProperties,
  installFloatingProperties,
  placeClearOfAnchor,
  rectOf,
  resolveLength,
  syncTopLayer,
  type ClearPlacement,
  type Direction,
  type Rect,
} from '../overlay/floating.ts';
import { deepActiveElement, returnFocusTo } from '../overlay/modality.ts';
import {
  coveringAttribute,
  feedbackEvents,
  feedbackMotionClasses,
  feedbackTags,
  feedbackTypeRoles,
  miniCommandIconSize,
  miniToolbarClearanceOrder,
  miniToolbarCss,
  type MiniCommand,
} from './feedback-model.ts';

let nextToolbarSerial = 0;

export class MjxMiniToolbar extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'open', 'for'];

  #root: ShadowRoot | undefined;
  #toolbar: HTMLElement | undefined;
  #explanations: HTMLElement | undefined;
  #buttons: HTMLButtonElement[] = [];
  #commands: readonly MiniCommand[] = [];
  #pressed = new Map<string, boolean>();
  #activeIndex = 0;
  #selection: Rect | undefined;
  #clearance: ClearPlacement | undefined;
  #observer: ResizeObserver | undefined;
  #watching = false;
  #invoker: HTMLElement | undefined;
  #serial = 0;
  /** The data the buttons were last built from, so an unrelated attribute change rebuilds nothing. */
  #renderedSignature = '';

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.render();
  }

  disconnectedCallback(): void {
    this.#stopWatching();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The toolbar's accessible name. A toolbar with no name is a row of unexplained icons. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  get open(): boolean {
    return this.hasAttribute('open');
  }

  set open(value: boolean) {
    if (value) this.setAttribute('open', '');
    else this.removeAttribute('open');
  }

  /** The commands, as data. Setting them re-seeds the toggles' state. */
  get commands(): readonly MiniCommand[] {
    return this.#commands;
  }

  set commands(next: readonly MiniCommand[]) {
    this.#commands = [...next];
    this.#pressed = new Map(
      this.#commands
        .filter((entry) => (entry.kind ?? 'command') === 'toggle')
        .map((entry) => [entry.command, entry.pressed === true]),
    );
    if (this.#activeIndex >= this.#commands.length) this.#activeIndex = 0;
    this.render();
  }

  /** Whether a toggle is on. `undefined` for a command that is not a toggle. */
  pressedState(command: string): boolean | undefined {
    return this.#pressed.get(command);
  }

  /**
   * The selection this toolbar serves, in viewport coordinates.
   *
   * A property first, because a real editor has a `Range` and not an element; the `for` attribute
   * is the catalogue's way of saying the same thing about a box that is on the page.
   */
  get selection(): Rect | undefined {
    if (this.#selection !== undefined) return this.#selection;
    const id = this.getAttribute('for');
    if (id === null || id === '') return undefined;
    const element = this.ownerDocument.getElementById(id);
    return element === null ? undefined : rectOf(element);
  }

  set selection(next: Rect | undefined) {
    this.#selection = next;
    this.render();
  }

  /** Where it ended up, and whether it cleared the selection. */
  get clearance(): ClearPlacement | undefined {
    return this.#clearance;
  }

  /** Whether the toolbar is, right now, drawn over any part of the selection it serves. */
  get covering(): boolean {
    return this.#clearance !== undefined && !this.#clearance.clear;
  }

  /** Which command holds the roving tab stop. */
  get activeIndex(): number {
    return this.#activeIndex;
  }

  /** The buttons, so a gate can count what a person can reach rather than what was declared. */
  get commandButtons(): readonly HTMLButtonElement[] {
    return this.#buttons;
  }

  override focus(options?: FocusOptions): void {
    const button = this.#buttons[this.#activeIndex];
    if (button === undefined) super.focus(options);
    else button.focus(options);
  }

  /** Open it, remembering where the keyboard was so Escape can put it back. */
  show(): void {
    this.#invoker = this.#currentFocus();
    this.open = true;
  }

  /** Close it. Focus goes back only if it is currently inside — see the module note. */
  close(): void {
    if (!this.open) return;
    const inside = this.#focusIsInside();
    this.open = false;
    if (inside) returnFocusTo(this.#invoker);
    this.#invoker = undefined;
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, miniToolbarCss);
    defineIcon();

    nextToolbarSerial += 1;
    this.#serial = nextToolbarSerial;

    const toolbar = document.createElement('div');
    toolbar.className = `toolbar anchored ${feedbackMotionClasses.toolbar}`;
    toolbar.setAttribute('part', 'toolbar');
    toolbar.setAttribute('role', 'toolbar');
    toolbar.addEventListener('keydown', this.#onKeyDown);
    this.#toolbar = toolbar;

    // One container for every command's explanation. An IDREF resolves inside its own tree, so a
    // shadow root may reuse short ids without colliding with the page or with another instance.
    const explanations = document.createElement('div');
    explanations.className = 'visually-hidden';
    this.#explanations = explanations;

    root.append(toolbar, explanations);
    this.addEventListener('focusin', this.#onFocusIn);
  }

  /**
   * Where the keyboard was, if it was anywhere a return could mean something.
   *
   * `<body>` is excluded deliberately: it is what `document.activeElement` reports when nothing is
   * focused, it is not focusable, and *"returned focus to the body"* is a claim that would read as
   * a success in a gate and as a lost keyboard to a person.
   */
  #currentFocus(): HTMLElement | undefined {
    const active = deepActiveElement(this.ownerDocument);
    if (!(active instanceof HTMLElement)) return undefined;
    if (active === this || active === this.ownerDocument.body) return undefined;
    return active;
  }

  /**
   * ⚠ **The invoker is captured when focus first *enters*, not when the toolbar opens.**
   *
   * A mini toolbar appears because of a selection, not because of a button — so at the moment it
   * opens the keyboard is wherever the person was typing, and very often nowhere at all. Capturing
   * then would record `<body>` for every toolbar the catalogue renders with `open` in its markup,
   * and Escape would then hand the keyboard to nothing.
   *
   * Capturing on the way in is the honest rule and it matches what Escape is for: a person who
   * never Tabbed into the toolbar has not moved, so there is nothing to give back; a person who did
   * gets back exactly where they came from. `event.relatedTarget` is where they came from, and it
   * needs no retargeting because it is in the document's own tree.
   */
  #onFocusIn = (event: FocusEvent): void => {
    if (this.#invoker !== undefined) return;
    const from = event.relatedTarget;
    if (!(from instanceof HTMLElement)) return;
    if (from === this || this.contains(from)) return;
    if (from === this.ownerDocument.body) return;
    this.#invoker = from;
  };

  #focusIsInside(): boolean {
    const active = deepActiveElement(this.ownerDocument);
    return active instanceof HTMLElement && this.#buttons.includes(active as HTMLButtonElement);
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const toolbar = this.#toolbar;
    if (toolbar === undefined) return;

    toolbar.setAttribute('aria-label', this.label);
    const open = this.open;
    toolbar.setAttribute('data-open', open ? 'true' : 'false');

    this.#renderCommands();
    syncTopLayer(toolbar, { inTopLayer: true, open, connected: this.isConnected });

    if (open) {
      this.#enter();
      this.#place();
      this.#startWatching();
    } else {
      clearPlacement(toolbar);
      delete toolbar.dataset['entering'];
      delete toolbar.dataset['entered'];
      this.#clearance = undefined;
      this.removeAttribute(coveringAttribute);
      this.#stopWatching();
    }
  }

  /**
   * Rebuild the row, and **only when the row has actually changed.**
   *
   * `render()` runs on every attribute change, and rebuilding the buttons destroys the one the
   * keyboard is on — a toolbar that lost focus because its `label` was retitled would be a defect
   * no counting assertion could see. The signature is the data the buttons are built from, so a
   * change to any of it rebuilds and nothing else does.
   */
  #renderCommands(): void {
    const toolbar = this.#toolbar;
    const explanations = this.#explanations;
    if (toolbar === undefined || explanations === undefined) return;

    const signature = JSON.stringify(this.#commands);
    if (signature === this.#renderedSignature && this.#buttons.length > 0) return;
    this.#renderedSignature = signature;

    toolbar.replaceChildren();
    explanations.replaceChildren();
    this.#buttons = [];

    for (const [index, entry] of this.#commands.entries()) {
      if (entry.separatorBefore === true && index > 0) {
        const rule = document.createElement('div');
        rule.className = 'separator';
        rule.setAttribute('role', 'separator');
        rule.setAttribute('aria-orientation', 'vertical');
        toolbar.append(rule);
      }
      toolbar.append(this.#commandButton(entry, index, explanations));
    }

    this.#syncTabStops();
  }

  #commandButton(entry: MiniCommand, index: number, explanations: HTMLElement): HTMLButtonElement {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = `command mjx-hit-target ${feedbackTypeRoles.command} ${feedbackMotionClasses.toolbar}`;
    button.dataset['command'] = entry.command;
    button.setAttribute('aria-label', entry.label);

    const kind = entry.kind ?? 'command';
    if (kind === 'toggle') {
      button.setAttribute('aria-pressed', this.#pressed.get(entry.command) === true ? 'true' : 'false');
    }

    /*
     * The two kinds of unavailable, applied to a **data row** rather than to a host.
     *
     * `applyAvailability` in `controls/control-element.ts` answers exactly this question, and it
     * answers it by reading a *host element's* attributes — which a command that is a plain object
     * does not have. Rather than fabricate a host to read from, the same two decisions are made
     * here explicitly: `disabled` is the platform's, `unavailable` stays focusable and keeps its
     * explanation. The doctrine is shared; only the place the facts come from differs.
     */
    if (entry.unavailable === true) {
      button.setAttribute('aria-disabled', 'true');
      const explanation = entry.explanation ?? '';
      if (explanation !== '') {
        const note = document.createElement('span');
        note.id = `mjx-mini-${String(this.#serial)}-${String(index)}`;
        note.textContent = explanation;
        explanations.append(note);
        button.setAttribute('aria-describedby', note.id);
        button.title = explanation;
      }
    }

    if (entry.icon !== undefined && entry.icon !== '') {
      const glyph = document.createElement('mjx-icon');
      glyph.setAttribute('name', entry.icon);
      glyph.setAttribute('size', String(miniCommandIconSize));
      button.append(glyph);
    } else {
      button.textContent = entry.label;
    }

    button.addEventListener('click', () => {
      this.#activate(entry, index);
    });
    this.#buttons.push(button);
    return button;
  }

  #activate(entry: MiniCommand, index: number): void {
    if (entry.unavailable === true) return;
    this.#activeIndex = index;
    this.#syncTabStops();
    let pressed: boolean | undefined;
    if ((entry.kind ?? 'command') === 'toggle') {
      pressed = this.#pressed.get(entry.command) !== true;
      this.#pressed.set(entry.command, pressed);
      this.#buttons[index]?.setAttribute('aria-pressed', pressed ? 'true' : 'false');
    }
    this.dispatchEvent(
      new CustomEvent(feedbackEvents.command, {
        bubbles: true,
        composed: true,
        detail: { command: entry.command, pressed },
      }),
    );
  }

  /**
   * The roving tab stop.
   *
   * **Set before focus moves**, exactly as `<mjx-menu>` records: the foundations' focus ring matches
   * `[tabindex]:not([tabindex="-1"])`, so focusing a button that still held `-1` would put the
   * keyboard somewhere with no ring on it.
   */
  #syncTabStops(): void {
    if (this.#buttons.length === 0) return;
    if (this.#activeIndex >= this.#buttons.length) this.#activeIndex = 0;
    for (const [index, button] of this.#buttons.entries()) {
      button.tabIndex = index === this.#activeIndex ? 0 : -1;
    }
  }

  #focusCommand(index: number): void {
    if (this.#buttons.length === 0) return;
    const count = this.#buttons.length;
    this.#activeIndex = ((index % count) + count) % count;
    this.#syncTabStops();
    this.#buttons[this.#activeIndex]?.focus({ preventScroll: true });
  }

  #onKeyDown = (event: KeyboardEvent): void => {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      this.close();
      return;
    }
    const rtl = this.#direction() === 'rtl';
    let next: number | undefined;
    switch (event.key) {
      case 'ArrowRight':
        next = this.#activeIndex + (rtl ? -1 : 1);
        break;
      case 'ArrowLeft':
        next = this.#activeIndex + (rtl ? 1 : -1);
        break;
      case 'Home':
        next = 0;
        break;
      case 'End':
        next = this.#buttons.length - 1;
        break;
      default:
        return;
    }
    event.preventDefault();
    this.#focusCommand(next);
  };

  // ── placement ──────────────────────────────────────────────────────────────

  #enter(): void {
    const toolbar = this.#toolbar;
    if (toolbar === undefined) return;
    if (toolbar.dataset['entered'] === 'true') return;
    toolbar.dataset['entered'] = 'true';
    toolbar.dataset['entering'] = '';
    requestAnimationFrame(() => {
      delete toolbar.dataset['entering'];
    });
  }

  #place(): void {
    const toolbar = this.#toolbar;
    const selection = this.selection;
    if (toolbar === undefined || selection === undefined) return;
    clearPlacement(toolbar);
    const gap = resolveLength(toolbar, floatingProperties.gap);
    const inset = resolveLength(toolbar, floatingProperties.boundaryInset);
    const box = toolbar.getBoundingClientRect();
    const clearance = placeClearOfAnchor(
      {
        anchor: selection,
        floating: { width: box.width, height: box.height },
        boundary: clippingBoundary(toolbar, inset),
        align: 'center',
        direction: this.#direction(),
        gap,
      },
      miniToolbarClearanceOrder,
    );
    applyPlacement(toolbar, clearance.placement);
    this.#clearance = clearance;
    if (clearance.clear) this.removeAttribute(coveringAttribute);
    else this.setAttribute(coveringAttribute, 'true');
  }

  #direction(): Direction {
    const view = this.ownerDocument.defaultView;
    if (view === null) return 'ltr';
    return view.getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  #startWatching(): void {
    if (this.#watching) return;
    this.#watching = true;
    const toolbar = this.#toolbar;
    if (toolbar === undefined) return;
    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver(this.#onResize);
      observer.observe(toolbar);
      const ancestor = clippingAncestor(toolbar);
      if (ancestor !== undefined) observer.observe(ancestor);
      const id = this.getAttribute('for');
      const anchor = id === null || id === '' ? null : this.ownerDocument.getElementById(id);
      if (anchor !== null) observer.observe(anchor);
      this.#observer = observer;
    }
    this.ownerDocument.defaultView?.addEventListener('resize', this.#onResize, { passive: true });
  }

  #stopWatching(): void {
    if (!this.#watching) return;
    this.#watching = false;
    this.#observer?.disconnect();
    this.#observer = undefined;
    this.ownerDocument.defaultView?.removeEventListener('resize', this.#onResize);
  }

  #onResize = (): void => {
    if (this.open) this.#place();
  };
}

/** Register the element. Idempotent. */
export function defineMiniToolbar(): void {
  if (customElements.get(feedbackTags.miniToolbar) === undefined) {
    customElements.define(feedbackTags.miniToolbar, MjxMiniToolbar);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-mini-toolbar': MjxMiniToolbar;
  }
}
