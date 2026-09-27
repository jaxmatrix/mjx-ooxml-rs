/**
 * The base both mobile bars are — **one implementation, two elements.**
 *
 * `<mjx-command-bar>` and `<mjx-contextual-action-bar>` are the same object with two contents:
 * a rail of commands, an overflow control that changes what the rail is, a roving tab stop and a
 * form factor read back out of the cascade. Writing that twice would be writing two slightly
 * different overflow behaviours, which is the failure `installControlStyles` and `surfaceSession`
 * were both built to avoid one level down.
 *
 * ## The one thing this class does not decide
 *
 * **Which commands it holds.** A command bar's contents come from a `commands` property the shell
 * sets; a contextual action bar's come from `contextualActions(selection)`. That is the only
 * difference between the two elements, and it is a single overridden method.
 *
 * ## Roving tab stop, and why it is possible here
 *
 * MJXOFF-186's finding is that a container cannot give a roving tab stop to children it does not
 * own — sequential focus descends into a shadow tree whatever `tabindex` the host carries. These
 * bars **build** their commands in their own shadow root rather than slotting them, so the roving
 * stop is theirs to hold, and a toolbar with one tab stop is what the ARIA pattern asks for.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { defineIcon } from '../icons/icon.ts';
import {
  commandBarOrder,
  commandBarPartition,
  isMobileFormFactor,
  mobileFormFactorProperty,
  mobileEvents,
  mobileFormFactors,
  mobileSpansProperty,
  overflowControl,
  type CommandBarPartition,
  type MobileCommand,
  type MobileFormFactor,
} from './mobile-model.ts';
import { mobileFormFactorAttribute, mobileTypeRoles, overflowPanelMotionClass } from './mobile-sheets.ts';

let nextRailSerial = 0;

/** The behaviour both bars share. */
export abstract class MjxMobileBar extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'open-overflow'];

  #root: ShadowRoot | undefined;
  #bar: HTMLElement | undefined;
  #rail: HTMLElement | undefined;
  #overflow: HTMLButtonElement | undefined;
  #live: HTMLElement | undefined;
  #buttons: HTMLButtonElement[] = [];
  #focused = 0;
  #observer: ResizeObserver | undefined;
  #railId = '';
  #lastFormFactor: MobileFormFactor | undefined;

  /** The stylesheet this element wears. */
  protected abstract styles(): string;

  /** What the rail holds. */
  protected abstract barCommands(): readonly MobileCommand[];

  /** The bar's accessible name, when the host has not given one. */
  protected abstract defaultLabel(): string;

  /** Anything the subclass wants between the rail and the overflow control. */
  protected decorate(_bar: HTMLElement): void {
    // Nothing, by default.
  }

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    this.render();
    this.#startWatching();
  }

  disconnectedCallback(): void {
    this.#observer?.disconnect();
    this.#observer = undefined;
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The bar's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? this.defaultLabel();
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /** Whether the rail is presented as the overflow grid. */
  get openOverflow(): boolean {
    return this.hasAttribute('open-overflow');
  }

  set openOverflow(value: boolean) {
    if (value) this.setAttribute('open-overflow', '');
    else this.removeAttribute('open-overflow');
  }

  /**
   * **Which form factor the cascade put this bar in** — read back, never computed here.
   *
   * `formFactorFor()` in the model says what it should be and the browser gate compares the two. A
   * component that decided its own presentation and then reported it would be grading its own
   * homework, which is `<mjx-ribbon-group>`'s rule and this inherits it.
   */
  get formFactor(): MobileFormFactor {
    const bar = this.#bar;
    if (bar === undefined) return 'desktop';
    const value = getComputedStyle(bar).getPropertyValue(mobileFormFactorProperty).trim();
    return isMobileFormFactor(value) ? value : 'desktop';
  }

  /** How the commands split at the current form factor. */
  get partition(): CommandBarPartition {
    return commandBarPartition(this.barCommands(), mobileFormFactors[this.formFactor].visibleSlots);
  }

  /** Every command button, in rail order. The gate counts these. */
  get commandButtons(): readonly HTMLButtonElement[] {
    return this.#buttons.filter((button) => button.classList.contains('command'));
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, this.styles());
    defineIcon();

    nextRailSerial += 1;
    this.#railId = `mjx-mobile-rail-${String(nextRailSerial)}`;

    const bar = document.createElement('div');
    bar.className = 'bar';
    bar.setAttribute('part', 'bar');
    bar.setAttribute('role', 'toolbar');
    bar.setAttribute('aria-orientation', 'horizontal');
    bar.addEventListener('keydown', this.#onKeyDown);
    this.#bar = bar;

    const rail = document.createElement('div');
    rail.className = `rail ${overflowPanelMotionClass}`;
    rail.setAttribute('part', 'rail');
    rail.id = this.#railId;
    this.#rail = rail;

    const overflow = document.createElement('button');
    overflow.type = 'button';
    overflow.className = 'overflow';
    overflow.setAttribute('part', 'overflow');
    overflow.setAttribute('aria-controls', this.#railId);
    overflow.setAttribute('aria-label', overflowControl.label);
    const glyph = document.createElement('mjx-icon');
    glyph.setAttribute('name', overflowControl.icon);
    overflow.append(glyph);
    overflow.addEventListener('click', this.#onOverflowClick);
    this.#overflow = overflow;

    const live = document.createElement('div');
    live.className = 'live';
    live.setAttribute('aria-live', 'polite');
    this.#live = live;

    bar.append(rail);
    this.decorate(bar);
    bar.append(overflow, live);
    root.append(bar);
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /**
   * Re-entrancy guard.
   *
   * `render()` rebuilds the rail, which changes the bar's size, which the `ResizeObserver` reports,
   * which calls `render()`. The second pass computes the same thing and settles — but the browser
   * logs a *ResizeObserver loop* error while it does, and a console error inside a Storybook
   * preview is indistinguishable from a component fault when the a11y sweep reports it.
   */
  #rendering = false;

  render(): void {
    const bar = this.#bar;
    const rail = this.#rail;
    const overflow = this.#overflow;
    if (bar === undefined || rail === undefined || overflow === undefined) return;
    if (this.#rendering) return;
    this.#rendering = true;
    try {
      this.#renderOnce(bar, rail, overflow);
    } finally {
      this.#rendering = false;
    }
  }

  #renderOnce(bar: HTMLElement, rail: HTMLElement, overflow: HTMLButtonElement): void {
    bar.setAttribute('aria-label', this.label);

    const factor = this.formFactor;
    bar.setAttribute(mobileFormFactorAttribute, factor);
    bar.dataset['spans'] = getComputedStyle(bar).getPropertyValue(mobileSpansProperty).trim() === '1'
      ? 'true'
      : 'false';

    const commands = this.barCommands();
    this.dataset['empty'] = commands.length === 0 ? 'true' : 'false';

    const { visible } = commandBarPartition(commands, mobileFormFactors[factor].visibleSlots);
    const demoted = new Set(commands.map((command) => command.id));
    for (const command of visible) demoted.delete(command.id);

    // ONE rail, holding every command in ladder order. Nothing is removed and nothing is rendered
    // twice: the visible run is simply the first commands in the rail, and the demoted ones are
    // reached by scrolling the rail or by opening it into the grid.
    rail.textContent = '';
    this.#buttons = [];
    for (const command of commandBarOrder(commands)) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'command';
      button.dataset['command'] = command.id;
      button.dataset['group'] = command.group;
      button.dataset['priority'] = command.priority;
      button.dataset['demoted'] = demoted.has(command.id) ? 'true' : 'false';
      button.setAttribute('aria-label', command.label);
      if (command.hasPopup) button.setAttribute('aria-haspopup', 'true');
      const glyph = document.createElement('mjx-icon');
      glyph.setAttribute('name', command.icon);
      const name = document.createElement('span');
      name.className = `name ${mobileTypeRoles.command}`;
      name.textContent = command.label;
      name.setAttribute('aria-hidden', 'true');
      button.append(glyph, name);
      button.addEventListener('click', () => {
        this.dispatchEvent(
          new CustomEvent(mobileEvents.command, {
            detail: { id: command.id },
            bubbles: true,
            composed: true,
          }),
        );
      });
      rail.append(button);
      this.#buttons.push(button);
    }

    const open = this.openOverflow;
    rail.dataset['region'] = open ? 'overflowPanel' : 'commandBar';
    overflow.setAttribute('aria-expanded', open ? 'true' : 'false');
    overflow.hidden = demoted.size === 0 && !open;
    this.#buttons.push(overflow);

    this.#applyRoving();

    if (this.#lastFormFactor !== factor) {
      this.#lastFormFactor = factor;
      this.dispatchEvent(
        new CustomEvent(mobileEvents.formFactorChanged, {
          detail: { formFactor: factor },
          bubbles: true,
          composed: true,
        }),
      );
    }
  }

  // ── the roving tab stop ────────────────────────────────────────────────────

  #reachable(): HTMLButtonElement[] {
    return this.#buttons.filter((button) => !button.hidden);
  }

  #applyRoving(): void {
    const reachable = this.#reachable();
    if (reachable.length === 0) return;
    if (this.#focused >= reachable.length) this.#focused = reachable.length - 1;
    reachable.forEach((button, index) => {
      button.tabIndex = index === this.#focused ? 0 : -1;
    });
  }

  #moveFocus(delta: number): void {
    const reachable = this.#reachable();
    if (reachable.length === 0) return;
    const next = (this.#focused + delta + reachable.length) % reachable.length;
    this.#focused = next;
    this.#applyRoving();
    reachable[next]?.focus();
  }

  #onKeyDown = (event: KeyboardEvent): void => {
    switch (event.key) {
      case 'ArrowRight':
        this.#moveFocus(1);
        break;
      case 'ArrowLeft':
        this.#moveFocus(-1);
        break;
      case 'Home':
        this.#focused = 0;
        this.#applyRoving();
        this.#reachable()[0]?.focus();
        break;
      case 'End': {
        const reachable = this.#reachable();
        this.#focused = reachable.length - 1;
        this.#applyRoving();
        reachable[reachable.length - 1]?.focus();
        break;
      }
      case 'Escape':
        if (!this.openOverflow) return;
        this.openOverflow = false;
        this.#overflow?.focus();
        break;
      default:
        return;
    }
    event.preventDefault();
    event.stopPropagation();
  };

  #onOverflowClick = (): void => {
    const open = !this.openOverflow;
    this.openOverflow = open;
    if (this.#live !== undefined) {
      this.#live.textContent = open ? 'More commands shown.' : 'More commands hidden.';
    }
    this.dispatchEvent(
      new CustomEvent(mobileEvents.overflowToggled, {
        detail: { open },
        bubbles: true,
        composed: true,
      }),
    );
  };

  // ── keeping the form factor current ────────────────────────────────────────

  /**
   * Watch the clipping ancestor as well as the element.
   *
   * The container's width is what a container query answers to, and it is not the element's own —
   * MJXOFF-188's fourth finding, applied. The window is watched too, because the **media** half of
   * the form-factor decision reads the viewport's block size, which no `ResizeObserver` reports.
   */
  #startWatching(): void {
    if (typeof ResizeObserver !== 'undefined' && this.#observer === undefined) {
      const observer = new ResizeObserver(() => {
        this.render();
      });
      observer.observe(this);
      const parent = this.parentElement;
      if (parent !== null) observer.observe(parent);
      this.#observer = observer;
    }
    this.ownerDocument.defaultView?.addEventListener('resize', this.#onViewportResize, {
      passive: true,
    });
  }

  #onViewportResize = (): void => {
    this.render();
  };
}
