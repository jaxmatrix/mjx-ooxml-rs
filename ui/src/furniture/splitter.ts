/**
 * `<mjx-splitter>` — a draggable divider between two regions, and a *lift* rather than a third copy.
 *
 * ```html
 * <div class="workspace">
 *   <div id="navigator">…</div>
 *   <mjx-splitter label="the navigator" controls="navigator" fraction="0.3"
 *                 min-start="0.15" min-end="0.3" storage-key="word/navigator"></mjx-splitter>
 *   <div id="document">…</div>
 * </div>
 * ```
 *
 * ## What this is, relative to `<mjx-task-pane>`
 *
 * MJXOFF-188 already built an ARIA window splitter inside the task pane, and MJXOFF-190's brief is
 * explicit: *"your pane splitter is either that component or a shared primitive lifted from it —
 * not a third implementation."* It is the second. The arithmetic — clamping, the drag, and the key
 * map whose growing arrow depends on which side the region is on and on the writing direction —
 * moved into `src/foundations/splitter.ts` parameterised by its bounds, and
 * `src/surfaces/surface-model.ts`'s three exports are now one-line bindings over it. Neither
 * component has its own copy, and `tests/furniture.test.ts` asserts the equivalence over a sweep
 * rather than leaving it as a claim.
 *
 * What this component adds is what a general splitter needs and a dock does not:
 *
 * * **a minimum per side**, declared separately, because the two regions are different things;
 * * **a collapse**, by double-click and by Enter or Space, which **restores rather than resets**:
 *   the fraction it collapsed from is remembered, so reversing it puts the region back where the
 *   person had it rather than at a default nobody chose;
 * * **a persisted position**, keyed by `storage-key`, so a splitter is where it was left across a
 *   remount.
 *
 * ## A splitter that only drags is broken
 *
 * `role="separator"` with `aria-valuenow`, focusable, and arrows, `Home` and `End` that move it —
 * ARIA's window-splitter pattern, which is the one thing here a keyboard user needs and a pointer
 * user never sees. A drag handle only a pointer can move is a resizable layout only some people can
 * resize, and `tests/browser/furniture.spec.ts` drives every one of those keys through the
 * browser's own input path.
 *
 * ⚠ **The role is on the HOST, and `aria-controls` is why** — the same finding `<mjx-scrollbar>`
 * records, and the a11y sweep caught this one in the act. An IDREF does not cross a shadow
 * boundary in either direction, and a splitter's `aria-controls` names *a region the component does
 * not own*; written onto a box inside the shadow root it resolves to nothing, which axe reports as
 * `aria-valid-attr-value` and a DOM inspector reports as perfectly fine. `<mjx-task-pane>`'s
 * splitter keeps its role inside its shadow root only because the pane it controls is in there
 * with it.
 *
 * ## `GUESS:` what a double-click does
 *
 * Office collapses a pane on a double-click of its divider and restores it on the next. The
 * *restore-rather-than-reset* half is this catalogue's reading of that behaviour and is not checked
 * against Office; it is stated here and in the story so that a reader can disagree with it on
 * purpose.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  clampToBounds,
  dragFraction,
  keyFraction,
  splitterDefaultFraction,
  splitterLabel,
  splitterValueNow,
  type SplitterBounds,
} from '../foundations/splitter.ts';
import { physicalSide, type Direction, type LogicalSide } from '../overlay/floating.ts';
import {
  collapsedFraction,
  furnitureBoxProperties,
  furnitureEvents,
  furnitureMotionClass,
  furnitureTags,
  splitterCollapseKeys,
  splitterCss,
  splitterDefaultBounds,
  splitterStep,
  splitterStorageKey,
  splitterValueText,
} from './furniture-model.ts';

/** The sheet, composed once. */
export const splitterSheet = splitterCss;

export class MjxSplitter extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'label',
    'controls',
    'orientation',
    'fraction',
    'min-start',
    'min-end',
    'collapsed',
    'storage-key',
  ];

  #root: ShadowRoot | undefined;
  #handle: HTMLElement | undefined;
  #fraction = splitterDefaultFraction;
  /** Where it was before it collapsed, so reversing a collapse is a restore and not a reset. */
  #restore = splitterDefaultFraction;
  #dragging: number | undefined;
  #loaded = false;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.#load();
    this.render();
  }

  attributeChangedCallback(name: string): void {
    if (name === 'fraction') {
      const declared = Number.parseFloat(this.getAttribute('fraction') ?? '');
      if (Number.isFinite(declared)) {
        this.#fraction = clampToBounds(declared, this.bounds, splitterDefaultFraction);
        this.#restore = this.#fraction;
      }
    }
    if (this.#root !== undefined) this.render();
  }

  /** What the splitter resizes, in words. The accessible name is derived from it. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The id of the region whose size it changes. */
  get controls(): string {
    return this.getAttribute('controls') ?? '';
  }

  /** `vertical` — a vertical line between two columns — or `horizontal`. ARIA's own sense. */
  get orientation(): 'vertical' | 'horizontal' {
    return this.getAttribute('orientation') === 'horizontal' ? 'horizontal' : 'vertical';
  }

  /**
   * The bounds, assembled from the **minimum per side**.
   *
   * `min-start` is what the leading region may never go below and `min-end` is the same promise for
   * the trailing one, which is why the maximum is `1 - min-end` rather than a second number an
   * author has to keep consistent with the first.
   */
  get bounds(): SplitterBounds {
    const min = this.#number('min-start', splitterDefaultBounds.min);
    const end = this.#number('min-end', 1 - splitterDefaultBounds.max);
    const max = 1 - end;
    return max > min ? { min, max } : splitterDefaultBounds;
  }

  /** The leading region's share of the boundary. Zero while collapsed. */
  get fraction(): number {
    return this.collapsed ? collapsedFraction : this.#fraction;
  }

  set fraction(value: number) {
    this.#setFraction(clampToBounds(value, this.bounds, splitterDefaultFraction), 'host');
  }

  /** Whether the leading region is collapsed away. */
  get collapsed(): boolean {
    return this.hasAttribute('collapsed');
  }

  set collapsed(value: boolean) {
    if (value === this.collapsed) return;
    this.#setCollapsed(value, 'host');
  }

  /** Where a collapse would restore to. */
  get restoreFraction(): number {
    return this.#restore;
  }

  /** Where the position is remembered, or the empty string when it is not remembered at all. */
  get storageKey(): string {
    return this.getAttribute('storage-key') ?? '';
  }

  /**
   * The separator, which **is this element**.
   *
   * Kept as a named accessor rather than removed, because a gate measuring *the thing a person
   * aims at* should not have to know whether that is the host or a box inside it.
   */
  get handleElement(): HTMLElement | undefined {
    return this.#handle;
  }

  #number(name: string, fallback: number): number {
    const declared = Number.parseFloat(this.getAttribute(name) ?? '');
    return Number.isFinite(declared) ? declared : fallback;
  }

  /**
   * Publish the fraction where the regions can actually read it.
   *
   * ⚠ **On the parent, not only on the host, and the browser gate is what said so.** A custom
   * property inherits *down*, and the two regions a splitter sits between are its **siblings** —
   * so a fraction written only onto this element reaches nothing at all, and a story sizing its
   * navigator from `var(--mjx-split-fraction, 0.3)` silently keeps the fallback for ever. The
   * assertion that caught it was *the region got wider*, not *the value changed*: every
   * attribute and every announced number was already right.
   *
   * `<mjx-task-pane>` gets away with writing its fraction on itself because the pane **is** the
   * resized region. A splitter is not, and the boundary is the only element both regions inherit
   * from — which is also the honest place for it, because the fraction is a fact about the
   * boundary rather than about the divider.
   */
  #writeFraction(): void {
    const fraction = String(this.fraction);
    const collapsed = this.collapsed ? '1' : '0';
    for (const target of [this as HTMLElement, this.parentElement]) {
      target?.style.setProperty(furnitureBoxProperties.splitFraction, fraction);
      target?.style.setProperty(furnitureBoxProperties.splitCollapsed, collapsed);
    }
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, splitterSheet);

    // The separator is this element. See the module note: `aria-controls` names a region in the
    // light DOM, and an IDREF written inside the shadow root would resolve to nothing.
    //
    // ⚠ No `mjx-hit-target` class anywhere, and the reason was measured rather than reasoned
    // about: that class sets a *minimum* of 40 CSS pixels in comfortable density, which made an
    // inner handle wider than the host it filled and left a forty-pixel gap between two panes.
    // `splitterCss` states the floor on `:host`, exactly as `<mjx-task-pane>`'s splitter does.
    this.setAttribute('role', 'separator');
    this.tabIndex = 0;
    this.addEventListener('keydown', this.#onKeyDown);
    this.addEventListener('pointerdown', this.#onPointerDown);
    this.addEventListener('pointermove', this.#onPointerMove);
    this.addEventListener('pointerup', this.#onPointerUp);
    this.addEventListener('pointercancel', this.#onPointerUp);
    this.addEventListener('dblclick', this.#onDoubleClick);

    const line = document.createElement('span');
    // The one moving part here that *should* ease: the line's colour changes when the region
    // collapses, and a settle is what makes that read as a state change rather than a repaint.
    line.className = `line ${furnitureMotionClass}`;
    line.setAttribute('part', 'line');

    root.append(line);
    this.#handle = this;
  }

  // ── the position, remembered ───────────────────────────────────────────────

  /**
   * Read a remembered position, once, on the first connection.
   *
   * ⚠ **Every read and write is wrapped**, because `localStorage` *throws* rather than returning
   * nothing in a private window, in a sandboxed frame and under a browser set to block site data —
   * and a splitter that threw on connect would take the whole shell down with it.
   */
  #load(): void {
    if (this.#loaded) return;
    this.#loaded = true;
    const key = this.storageKey;
    if (key === '') return;
    try {
      const stored = this.ownerDocument.defaultView?.localStorage.getItem(splitterStorageKey(key));
      if (stored === null || stored === undefined) return;
      const parsed = JSON.parse(stored) as { fraction?: number; collapsed?: boolean };
      if (typeof parsed.fraction === 'number') {
        this.#fraction = clampToBounds(parsed.fraction, this.bounds, splitterDefaultFraction);
        this.#restore = this.#fraction;
      }
      if (parsed.collapsed === true) this.setAttribute('collapsed', '');
      else this.removeAttribute('collapsed');
    } catch {
      // A position nobody can read is a position nobody had. The default is right, and refusing to
      // render would be a shell that will not open because of a preference.
    }
  }

  #save(): void {
    const key = this.storageKey;
    if (key === '') return;
    try {
      this.ownerDocument.defaultView?.localStorage.setItem(
        splitterStorageKey(key),
        JSON.stringify({ fraction: this.#fraction, collapsed: this.collapsed }),
      );
    } catch {
      // See `#load`: storage is a convenience and never a requirement.
    }
  }

  // ── moving it ──────────────────────────────────────────────────────────────

  #physicalSide(): 'left' | 'right' {
    const view = this.ownerDocument.defaultView;
    const direction: Direction =
      view !== null && view.getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
    // The leading region is at the start of the line, so the splitter grows it with the arrow that
    // points toward the end — which is `physicalSide('inlineStart')` and mirrors by itself.
    const side: LogicalSide = 'inlineStart';
    return physicalSide(side, direction) === 'left' ? 'left' : 'right';
  }

  #setFraction(value: number, cause: 'pointer' | 'keyboard' | 'host'): void {
    const wanted = clampToBounds(value, this.bounds, splitterDefaultFraction);
    if (wanted === this.#fraction && !this.collapsed) return;
    this.#fraction = wanted;
    this.#restore = wanted;
    if (this.collapsed) this.removeAttribute('collapsed');
    this.render();
    this.#save();
    this.#report(cause);
  }

  #setCollapsed(collapsed: boolean, cause: 'pointer' | 'keyboard' | 'host'): void {
    if (collapsed) {
      this.#restore = this.#fraction;
      this.setAttribute('collapsed', '');
    } else {
      this.#fraction = clampToBounds(this.#restore, this.bounds, splitterDefaultFraction);
      this.removeAttribute('collapsed');
    }
    this.render();
    this.#save();
    this.#report(cause);
  }

  #report(cause: 'pointer' | 'keyboard' | 'host'): void {
    this.dispatchEvent(
      new CustomEvent(furnitureEvents.split, {
        bubbles: true,
        composed: true,
        detail: { fraction: this.fraction, collapsed: this.collapsed, cause },
      }),
    );
  }

  #onKeyDown = (event: KeyboardEvent): void => {
    if (splitterCollapseKeys.includes(event.key)) {
      event.preventDefault();
      this.#setCollapsed(!this.collapsed, 'keyboard');
      return;
    }
    const next = keyFraction(
      event.key,
      this.#fraction,
      this.#physicalSide(),
      this.bounds,
      splitterStep,
      splitterDefaultFraction,
    );
    if (next === undefined) return;
    event.preventDefault();
    this.#setFraction(next, 'keyboard');
  };

  #onDoubleClick = (event: MouseEvent): void => {
    event.preventDefault();
    this.#setCollapsed(!this.collapsed, 'pointer');
  };

  #onPointerDown = (event: PointerEvent): void => {
    // ⚠ A second click of a double-click must not start a drag, or the collapse would be undone by
    // the drag that carried it.
    if (event.detail > 1) return;
    event.preventDefault();
    this.setPointerCapture(event.pointerId);
    this.#dragging = event.pointerId;
  };

  #onPointerMove = (event: PointerEvent): void => {
    if (this.#dragging !== event.pointerId) return;
    const parent = this.parentElement;
    if (parent === null) return;
    const box = parent.getBoundingClientRect();
    const horizontal = this.orientation === 'horizontal';
    const along = horizontal ? event.clientY : event.clientX;
    const boundary = horizontal
      ? { start: box.top, size: box.height }
      : { start: box.left, size: box.width };
    const side = horizontal ? 'left' : this.#physicalSide();
    this.#setFraction(
      dragFraction(along, boundary, side, this.bounds, splitterDefaultFraction),
      'pointer',
    );
  };

  #onPointerUp = (event: PointerEvent): void => {
    if (this.#dragging !== event.pointerId) return;
    this.releasePointerCapture(event.pointerId);
    this.#dragging = undefined;
  };

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render from the attributes. */
  render(): void {
    const handle = this.#handle;
    if (handle === undefined) return;
    const bounds = this.bounds;

    this.#writeFraction();
    if (this.collapsed) this.dataset['collapsed'] = '';
    else delete this.dataset['collapsed'];

    handle.setAttribute('aria-label', splitterLabel(this.label));
    handle.setAttribute('aria-orientation', this.orientation);
    handle.setAttribute('aria-valuemin', String(splitterValueNow(bounds.min)));
    handle.setAttribute('aria-valuemax', String(splitterValueNow(bounds.max)));
    handle.setAttribute('aria-valuenow', String(splitterValueNow(this.fraction)));
    handle.setAttribute('aria-valuetext', splitterValueText(this.fraction, this.collapsed));
    if (this.controls === '') handle.removeAttribute('aria-controls');
    else handle.setAttribute('aria-controls', this.controls);
  }
}

/** Register the element. Idempotent. */
export function defineSplitter(): void {
  if (customElements.get(furnitureTags.splitter) === undefined) {
    customElements.define(furnitureTags.splitter, MjxSplitter);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-splitter': MjxSplitter;
  }
}
