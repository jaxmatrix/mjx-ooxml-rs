/**
 * `<mjx-popover>` — the two **anchored** surfaces: the popover and the flyout.
 *
 * ```html
 * <mjx-popover label="Line spacing" kind="flyout">
 *   <mjx-button slot="anchor" label="Line spacing"></mjx-button>
 *   <mjx-measure-input label="Before"></mjx-measure-input>
 *   <mjx-measure-input label="After"></mjx-measure-input>
 * </mjx-popover>
 * ```
 *
 * ## One element, two rows, and the difference is the tab stops
 *
 * A popover and a flyout are the same geometry: a box hung off an anchor, flipped and shifted by
 * `placeFloating`, dismissed by Escape or by a click outside. They differ in exactly the thing U05
 * says decides everything about focus — **how many tab stops are inside** — and therefore in
 * whether `Tab` means *leave* or *move within*:
 *
 * | Row | Stops | `Tab` | Closes on |
 * |---|---|---|---|
 * | `popover` | one, roving | leaves, so leaving closes it | Escape, a click outside, focus leaving |
 * | `flyout` | many, independent | moves within — so it cannot also be the exit | Escape, a click outside |
 *
 * ⚠ **The row is declared and the count is measured, and the gate compares them.** `kind` is an
 * attribute a caller writes, and a caller who put six controls in a `popover` would get a surface
 * that closes the moment a person Tabs to the second one. So the component counts its own tab
 * stops when it opens and **warns on the console** when the count disagrees with the row — the
 * same shape as `<mjx-ribbon-group>` reporting a group with four essential commands — and
 * `tests/browser/surfaces.spec.ts` counts them with real `Tab` presses and asserts the agreement
 * rather than believing either.
 *
 * ## Neither of them is modal, and that is why neither has a scrim
 *
 * `surfaceKinds` says so and this component never asks the question again. A scrim over a
 * non-modal surface is the commonest way a disclosure comes to look like a decision.
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
  isAlign,
  isLogicalSide,
  placeFloating,
  rectOf,
  resolveLength,
  syncTopLayer,
  type Align,
  type Direction,
  type LogicalSide,
  type Placement,
} from '../overlay/floating.ts';
import { tabStopsWithin, wrapTab } from '../overlay/modality.ts';
import { SurfaceSession, type SurfaceSessionHost } from './surface-session.ts';
import {
  closeButtonLabel,
  defaultPopoverAlign,
  defaultPopoverSide,
  popoverCss,
  surfaceCloseIcon,
  surfaceKinds,
  surfaceMotionClass,
  surfaceTags,
  surfaceTypeRoles,
  type SurfaceCloseReason,
  type SurfaceKind,
} from './surface-model.ts';

/** The two rows this element renders. */
export type PopoverKind = Extract<SurfaceKind, 'popover' | 'flyout'>;

/** What a row says about its own tab stops. `one` and `many`, from U05's table. */
export const popoverExpectedStops: Readonly<Record<PopoverKind, 'one' | 'many'>> = {
  popover: 'one',
  flyout: 'many',
};

const styles = `${floatingCss}\n${popoverCss}`;

let nextPopoverSerial = 0;

export class MjxPopover extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'open',
    'kind',
    'placement',
    'align',
  ];

  #root: ShadowRoot | undefined;
  #surface: HTMLElement | undefined;
  #anchorSlot: HTMLSlotElement | undefined;
  #title: HTMLElement | undefined;
  #closeButton: HTMLButtonElement | undefined;
  #session: SurfaceSession | undefined;
  #placement: Placement | undefined;
  #observer: ResizeObserver | undefined;
  #watching = false;
  #titleId = '';
  /** What the last open actually counted, so a gate need not re-derive it. */
  #countedStops = 0;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.render();
    this.ownerDocument.addEventListener('pointerdown', this.#onDocumentDown, true);
  }

  disconnectedCallback(): void {
    this.#stopWatching();
    this.ownerDocument.removeEventListener('pointerdown', this.#onDocumentDown, true);
    this.#session?.dispose();
    this.#session = undefined;
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The surface's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** Which of the two rows. */
  get kind(): PopoverKind {
    return this.getAttribute('kind') === 'flyout' ? 'flyout' : 'popover';
  }

  get open(): boolean {
    return this.hasAttribute('open');
  }

  set open(value: boolean) {
    if (value) this.setAttribute('open', '');
    else this.removeAttribute('open');
  }

  /** The side it prefers, logically. */
  get side(): LogicalSide {
    const declared = this.getAttribute('placement');
    return isLogicalSide(declared) ? declared : defaultPopoverSide;
  }

  /** How it lines up on the cross axis. */
  get align(): Align {
    const declared = this.getAttribute('align');
    return isAlign(declared) ? declared : defaultPopoverAlign;
  }

  /** Where it ended up. */
  get placement(): Placement | undefined {
    return this.#placement;
  }

  /** How many tab stops the last open found inside it. */
  get countedTabStops(): number {
    return this.#countedStops;
  }

  /** The element focus will go back to. */
  get invoker(): HTMLElement | undefined {
    return this.#session?.invoker;
  }

  /** The button, or whatever else, in the `anchor` slot. */
  get anchor(): HTMLElement | undefined {
    const assigned = this.#anchorSlot?.assignedElements({ flatten: true }) ?? [];
    const first = assigned[0];
    return first instanceof HTMLElement ? first : undefined;
  }

  show(): void {
    this.open = true;
  }

  close(reason: SurfaceCloseReason = 'programmatic'): boolean {
    const closed = this.#session?.close(reason) ?? false;
    if (closed) this.open = false;
    return closed;
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, styles);
    defineIcon();

    nextPopoverSerial += 1;
    this.#titleId = `mjx-popover-title-${String(nextPopoverSerial)}`;

    const anchorSlot = document.createElement('slot');
    anchorSlot.name = 'anchor';
    anchorSlot.addEventListener('click', this.#onAnchorClick);
    anchorSlot.addEventListener('slotchange', this.#onSlotChange);
    this.#anchorSlot = anchorSlot;

    const surface = document.createElement('div');
    surface.className = 'surface';
    surface.setAttribute('part', 'surface');
    surface.setAttribute('role', 'dialog');
    surface.setAttribute('aria-labelledby', this.#titleId);
    surface.tabIndex = -1;
    this.#surface = surface;

    const header = document.createElement('div');
    header.className = 'header';
    header.setAttribute('part', 'header');

    const title = document.createElement('h2');
    title.className = `title ${surfaceTypeRoles.title}`;
    title.id = this.#titleId;
    title.setAttribute('part', 'title');
    this.#title = title;

    const close = document.createElement('button');
    close.type = 'button';
    close.className = 'close mjx-hit-target mjx-motion-surface-settle';
    close.setAttribute('part', 'close');
    close.addEventListener('click', this.#onCloseClick);
    const glyph = document.createElement('mjx-icon');
    glyph.setAttribute('name', surfaceCloseIcon.name);
    glyph.setAttribute('size', String(surfaceCloseIcon.size));
    close.append(glyph);
    this.#closeButton = close;

    header.append(title, close);

    const body = document.createElement('div');
    body.className = `body ${surfaceTypeRoles.body}`;
    body.setAttribute('part', 'body');
    body.append(document.createElement('slot'));

    surface.append(header, body);
    root.append(anchorSlot, surface);

    this.#session = new SurfaceSession(this.#sessionHost());
    this.addEventListener('keydown', this.#onKeyDown);
    this.addEventListener('focusout', this.#onFocusOut);
  }

  #sessionHost(): SurfaceSessionHost {
    return {
      element: this,
      surfaceBox: () => this.#surface,
      currentKind: () => this.kind,
      reflectOpen: (open) => {
        if (open) this.setAttribute('open', '');
        else this.removeAttribute('open');
        this.render();
      },
    };
  }

  #onSlotChange = (): void => {
    this.#syncAnchorState();
  };

  /**
   * Tell the anchor what it opens, **only when the anchor can legally be told.**
   *
   * ⚠ `aria-haspopup` and `aria-expanded` on an element with no role are a real
   * `aria-allowed-attr` violation, and a custom element host has no role. U05 recorded the same
   * shape from the other end — *"the painted box cannot carry `aria-disabled`, so the host's ARIA
   * state is mirrored onto it as `data-*`"* — and the answer here is the mirror image: a native
   * button or an element that has declared a role gets the attributes, and anything else gets
   * `data-mjx-expanded` and a note in the catalogue telling an author to use a button.
   */
  #syncAnchorState(): void {
    const anchor = this.anchor;
    if (anchor === undefined) return;
    const expanded = this.open ? 'true' : 'false';
    if (anchor.matches('button, [role]')) {
      anchor.setAttribute('aria-haspopup', 'dialog');
      anchor.setAttribute('aria-expanded', expanded);
      anchor.removeAttribute('data-mjx-expanded');
      return;
    }
    anchor.dataset['mjxExpanded'] = expanded;
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const surface = this.#surface;
    const session = this.#session;
    if (surface === undefined || session === undefined) return;

    if (this.#title !== undefined) this.#title.textContent = this.label;
    if (this.#closeButton !== undefined) {
      this.#closeButton.setAttribute('aria-label', closeButtonLabel(this.label));
    }

    const wantsOpen = this.open;
    if (wantsOpen && !session.open) session.begin(this.anchor);
    else if (!wantsOpen && session.open) {
      session.close('programmatic');
      return;
    }

    const kind = this.kind;
    surface.dataset['kind'] = kind;
    surface.setAttribute('data-open', wantsOpen ? 'true' : 'false');
    surface.setAttribute('aria-modal', 'false');
    surface.className = `surface ${surfaceMotionClass(kind)}`;
    this.#syncAnchorState();

    syncTopLayer(surface, { inTopLayer: true, open: wantsOpen, connected: this.isConnected });

    if (wantsOpen) {
      this.#enter();
      this.#place();
      this.#startWatching();
    } else {
      clearPlacement(surface);
      delete surface.dataset['entering'];
      delete surface.dataset['entered'];
      this.#placement = undefined;
      this.#stopWatching();
    }
  }

  #enter(): void {
    const surface = this.#surface;
    if (surface === undefined) return;
    if (surface.dataset['entered'] === 'true') return;
    surface.dataset['entered'] = 'true';
    surface.dataset['entering'] = '';
    requestAnimationFrame(() => {
      delete surface.dataset['entering'];
    });
    requestAnimationFrame(() => {
      if (!this.open) return;
      this.#session?.takeFocus();
      this.#auditTabStops();
    });
  }

  /**
   * Count the stops, and say so when the count disagrees with the row.
   *
   * ⚠ **A warning and not a throw**, for the reason `<mjx-ribbon-group>` gives about its
   * three-command ceiling: the surface still works, the author still needs to know, and a
   * component that refused to render would take a whole application down over a taxonomy
   * disagreement. The *gate* is the browser suite, which counts with real keys.
   */
  #auditTabStops(): void {
    const surface = this.#surface;
    if (surface === undefined) return;
    const stops = tabStopsWithin(surface);
    this.#countedStops = stops.length;
    // The close button is always a stop of the surface's own, so a `popover` row is allowed one
    // stop of content beside it.
    const contentStops = stops.filter((stop) => stop !== this.#closeButton).length;
    const expected = popoverExpectedStops[this.kind];
    if (expected === 'one' && contentStops > 1) {
      console.warn(
        `<${surfaceTags.popover} kind="popover" label="${this.label}"> has ${String(contentStops)} ` +
          'tab stops. A surface with more than one uses Tab to move within itself, so Tab cannot ' +
          'also be the way out of it — declare kind="flyout", which traps and does not close on ' +
          'blur. See focusManagementPatterns in src/menus/menu-model.ts.',
      );
    }
    if (expected === 'many' && contentStops <= 1) {
      console.warn(
        `<${surfaceTags.popover} kind="flyout" label="${this.label}"> has ${String(contentStops)} ` +
          'tab stops. A surface with one roving stop should be a disclosure rather than a trap — ' +
          'declare kind="popover".',
      );
    }
  }

  #place(): void {
    const surface = this.#surface;
    const anchor = this.anchor;
    if (surface === undefined || anchor === undefined) return;
    clearPlacement(surface);
    const gap = resolveLength(surface, floatingProperties.gap);
    const inset = resolveLength(surface, floatingProperties.boundaryInset);
    const box = surface.getBoundingClientRect();
    const placement = placeFloating({
      anchor: rectOf(anchor),
      floating: { width: box.width, height: box.height },
      boundary: clippingBoundary(surface, inset),
      side: this.side,
      align: this.align,
      direction: this.#direction(),
      gap,
    });
    applyPlacement(surface, placement);
    this.#placement = placement;
  }

  #direction(): Direction {
    const view = this.ownerDocument.defaultView;
    if (view === null) return 'ltr';
    return view.getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  // ── the ways it closes ─────────────────────────────────────────────────────

  #onAnchorClick = (): void => {
    if (this.open) this.close('closeButton');
    else this.show();
  };

  /** Which reason a click on the anchor closes with. Named so a gate can assert the toggle. */
  static readonly anchorCloseReason: SurfaceCloseReason = 'closeButton';

  #onCloseClick = (): void => {
    this.close('closeButton');
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    if (!this.open) return;
    if (event.key === 'Escape') {
      if (this.close('escape')) {
        event.preventDefault();
        event.stopPropagation();
      }
      return;
    }
    if (surfaceKinds[this.kind].focus === 'trap') {
      const surface = this.#surface;
      if (surface !== undefined && wrapTab(event, surface)) event.stopPropagation();
    }
  };

  /**
   * A disclosure closes when focus leaves it; a trap does not, because focus cannot.
   *
   * ⚠ **A null `relatedTarget` is re-checked a microtask later.** It is the commonest movement in
   * an overlay system — a box hides the focused element, focus falls to the document, and the
   * component *then* puts it back — and acting on it synchronously closes a surface one keystroke
   * early. `<mjx-menu>` records the same, and it arrives here by the same route.
   */
  #onFocusOut = (event: FocusEvent): void => {
    if (!this.open) return;
    if (surfaceKinds[this.kind].focus === 'trap') return;
    const next = event.relatedTarget;
    if (next instanceof Node && this.contains(next)) return;
    queueMicrotask(() => {
      if (!this.open) return;
      const active = this.ownerDocument.activeElement;
      if (active !== null && this.contains(active)) return;
      this.close('blur');
    });
  };

  #onDocumentDown = (event: Event): void => {
    if (!this.open) return;
    const path = event.composedPath();
    if (path.includes(this)) return;
    this.close('outside');
  };

  // ── keeping it where it goes ───────────────────────────────────────────────

  #startWatching(): void {
    if (this.#watching) return;
    this.#watching = true;
    const surface = this.#surface;
    if (surface === undefined) return;
    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver(this.#onResize);
      observer.observe(surface);
      const ancestor = clippingAncestor(surface);
      if (ancestor !== undefined) observer.observe(ancestor);
      const anchor = this.anchor;
      if (anchor !== undefined) observer.observe(anchor);
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
export function definePopover(): void {
  if (customElements.get(surfaceTags.popover) === undefined) {
    customElements.define(surfaceTags.popover, MjxPopover);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-popover': MjxPopover;
  }
}
