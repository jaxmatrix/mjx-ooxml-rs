/**
 * `<mjx-status-bar>` — three regions, a declared drop order, and two live regions.
 *
 * ```html
 * <mjx-status-bar label="Document status">
 *   <mjx-status-segment label="Page" value="4 of 20" priority="essential"></mjx-status-segment>
 *   <mjx-status-segment label="Words" value="3,182" priority="supplementary"></mjx-status-segment>
 *   <mjx-status-segment label="English (UK)" value="" priority="ancillary" region="end">
 *   </mjx-status-segment>
 * </mjx-status-bar>
 * ```
 *
 * ## Nothing is lost, and that is structural rather than remembered
 *
 * MJXOFF-183's rule, applied to a second container: *"the obvious way to build a collapsing group
 * is to render its commands twice, once in the strip and once in a menu. That is also the way to
 * lose one."* So a segment is **one element that moves**. When the bar narrows past a priority's
 * width the bar reassigns that segment's `slot` from its region to the overflow panel — the same
 * DOM node, with the same identity, the same event listeners and the same accessible name — and
 * `tests/browser/furniture.spec.ts` asserts the identity across three widths rather than counting
 * labels.
 *
 * ## The decision is CSS's; the observer only says *look again*
 *
 * The ladder is eight generated `@container` blocks writing `--mjx-status-presentation` onto one
 * hidden probe per priority, and the component reads those back. That is MJXOFF-183's mechanism
 * exactly, and it matters here for the same reason: **the component never decides a width.** What
 * the `ResizeObserver` below contributes is a *notification* that the cascade may have moved —
 * CSS cannot reassign a slot, so something has to ask. A gate that compared the component against
 * `statusPresentationAt()` would be grading the implementation's own homework if the implementation
 * were also the thing deciding, and it is not.
 *
 * ## Two live regions, and the routine change goes into neither
 *
 * A page number that changes as a person scrolls is the reason the bar exists and it changes
 * constantly. `announce` therefore defaults to `off`, `polite` lands in the `status` region and
 * `assertive` in the `alert` region, and the two regions are separate elements that exist from the
 * first render — MJXOFF-189's rule, because a live region's politeness is settled when it enters
 * the accessibility tree and rewriting the attribute is a change assistive technology may not have
 * noticed.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  furnitureEvents,
  furnitureTags,
  statusBarCss,
  statusAnnouncementRoles,
  statusOverflowLabel,
  statusPresentationProperty,
  statusPriorityNames,
  statusRegionNames,
  type StatusAnnouncement,
  type StatusPresentation,
  type StatusPriority,
} from './furniture-model.ts';
import { MjxStatusSegment } from './status-segment.ts';

/** The sheet, composed once. */
export const statusBarSheet = statusBarCss;

/** The glyph the overflow disclosure draws. Up, because the panel opens above the bar. */
export const statusOverflowIcon = { name: 'chevron-up', size: 16 } as const;

/** The slot a demoted segment is moved into. */
export const overflowSlotName = 'overflow';

let nextStatusBarSerial = 0;

export class MjxStatusBar extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label'];

  #root: ShadowRoot | undefined;
  #probes = new Map<StatusPriority, HTMLElement>();
  #trigger: HTMLButtonElement | undefined;
  #panel: HTMLElement | undefined;
  #polite: HTMLElement | undefined;
  #alert: HTMLElement | undefined;
  #resize: ResizeObserver | undefined;
  #children: MutationObserver | undefined;
  #panelId = '';
  #open = false;
  /** The last arrangement written, so a re-read that changes nothing writes nothing. */
  #arrangement = '';

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.#observe();
    this.render();
  }

  disconnectedCallback(): void {
    this.#resize?.disconnect();
    this.#resize = undefined;
    this.#children?.disconnect();
    this.#children = undefined;
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The bar's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? 'Status';
  }

  /** Every segment in the bar, in document order. */
  get segments(): MjxStatusSegment[] {
    return [...this.children].filter(
      (child): child is MjxStatusSegment => child instanceof MjxStatusSegment,
    );
  }

  /** The segments currently on the bar. */
  get shown(): MjxStatusSegment[] {
    return this.segments.filter((segment) => segment.getAttribute('slot') !== overflowSlotName);
  }

  /** The segments currently behind the overflow disclosure. Never *lost* — always these. */
  get overflowed(): MjxStatusSegment[] {
    return this.segments.filter((segment) => segment.getAttribute('slot') === overflowSlotName);
  }

  /** Whether the overflow panel is open. */
  get open(): boolean {
    return this.#open;
  }

  /**
   * **Which presentation CSS put this priority in** — read back, never computed here.
   *
   * The gate compares this against `statusPresentationAt()`. A component that decided its own
   * presentation and then reported it would be grading its own homework.
   */
  presentationOf(priority: StatusPriority): StatusPresentation {
    const probe = this.#probes.get(priority);
    const view = this.ownerDocument.defaultView;
    if (probe === undefined || view === null) return 'shown';
    const value = view.getComputedStyle(probe).getPropertyValue(statusPresentationProperty).trim();
    return value === 'overflow' ? 'overflow' : 'shown';
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, statusBarSheet);
    defineIcon();

    nextStatusBarSerial += 1;
    this.#panelId = `mjx-status-overflow-${String(nextStatusBarSerial)}`;

    const bar = document.createElement('div');
    bar.className = 'bar';
    bar.setAttribute('part', 'bar');
    // ⚠ `group`, not `status`. The bar as a whole is a set of readings a person may look at; the
    // *announcing* is done by the two live regions below, and giving the container a live role as
    // well would announce every segment every time any one of them moved.
    bar.setAttribute('role', 'group');
    bar.setAttribute('aria-label', this.label);

    for (const region of statusRegionNames) {
      const box = document.createElement('div');
      box.className = 'region';
      box.dataset['region'] = region;
      box.setAttribute('part', `region-${region}`);
      const slot = document.createElement('slot');
      slot.name = region;
      box.append(slot);
      bar.append(box);
    }

    const trigger = document.createElement('button');
    trigger.type = 'button';
    trigger.className = 'overflow-trigger mjx-hit-target mjx-focus-ring';
    trigger.setAttribute('part', 'overflow-trigger');
    trigger.setAttribute('aria-expanded', 'false');
    trigger.setAttribute('aria-controls', this.#panelId);
    trigger.addEventListener('click', this.#onTrigger);
    const glyph = document.createElement('mjx-icon');
    glyph.setAttribute('name', statusOverflowIcon.name);
    glyph.setAttribute('size', String(statusOverflowIcon.size));
    trigger.append(glyph);
    bar.append(trigger);
    this.#trigger = trigger;

    const panel = document.createElement('div');
    panel.className = 'overflow-panel';
    panel.id = this.#panelId;
    panel.setAttribute('part', 'overflow-panel');
    panel.setAttribute('role', 'group');
    const overflowSlot = document.createElement('slot');
    overflowSlot.name = overflowSlotName;
    panel.append(overflowSlot);
    panel.hidden = true;
    this.#panel = panel;

    // One probe per priority. They draw nothing; they exist so a `@container` decision can be read
    // by JavaScript with no measurement anywhere.
    const probes = document.createElement('div');
    probes.setAttribute('aria-hidden', 'true');
    for (const priority of statusPriorityNames) {
      const probe = document.createElement('i');
      probe.className = 'probe';
      probe.dataset['priority'] = priority;
      probes.append(probe);
      this.#probes.set(priority, probe);
    }

    const polite = document.createElement('div');
    polite.className = 'live';
    polite.setAttribute('role', 'status');
    polite.setAttribute('part', 'polite');

    const alert = document.createElement('div');
    alert.className = 'live';
    alert.setAttribute('role', 'alert');
    alert.setAttribute('part', 'alert');

    this.#polite = polite;
    this.#alert = alert;

    root.append(bar, panel, probes, polite, alert);
    this.addEventListener(furnitureEvents.statusChange, this.#onSegmentChange as EventListener);
  }

  #observe(): void {
    const view = this.ownerDocument.defaultView;
    if (view !== null && this.#resize === undefined && 'ResizeObserver' in view) {
      // Not a measurement: the callback reads the *cascade*, never this element's box. What a
      // resize contributes is the news that the cascade may have moved, which CSS has no way of
      // telling JavaScript on its own.
      this.#resize = new view.ResizeObserver(() => {
        this.#arrange();
      });
      this.#resize.observe(this);
    }
    if (this.#children === undefined && typeof MutationObserver !== 'undefined') {
      this.#children = new MutationObserver(() => {
        this.#arrange();
      });
      // `slot` is deliberately not in the filter: this observer's own writes are `slot` writes, and
      // watching for them would be a loop.
      this.#children.observe(this, {
        childList: true,
        attributes: true,
        attributeFilter: ['priority', 'region'],
      });
    }
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  /** Re-render from the attributes and re-read the cascade. */
  render(): void {
    const root = this.#root;
    if (root === undefined) return;
    const bar = root.querySelector('.bar');
    bar?.setAttribute('aria-label', this.label);
    this.#arrange();
  }

  /**
   * Put every segment where CSS says it goes.
   *
   * Idempotent, and cheap when nothing moved: the arrangement is fingerprinted and an unchanged one
   * writes no attribute at all, which is what keeps the mutation observer from seeing its own work.
   */
  #arrange(): void {
    const trigger = this.#trigger;
    const panel = this.#panel;
    if (trigger === undefined || panel === undefined) return;

    const wanted = new Map<MjxStatusSegment, string>();
    for (const segment of this.segments) {
      const presentation = this.presentationOf(segment.priority);
      wanted.set(segment, presentation === 'overflow' ? overflowSlotName : segment.region);
    }

    const fingerprint = [...wanted.values()].join('|');
    if (fingerprint === this.#arrangement) return;
    this.#arrangement = fingerprint;

    for (const [segment, slot] of wanted) {
      if (segment.getAttribute('slot') !== slot) segment.setAttribute('slot', slot);
    }

    const hidden = [...wanted.values()].filter((slot) => slot === overflowSlotName).length;
    trigger.hidden = hidden === 0;
    trigger.setAttribute('aria-label', statusOverflowLabel(hidden));
    if (hidden === 0 && this.#open) this.#setOpen(false);
    panel.hidden = !this.#open || hidden === 0;

    this.dispatchEvent(
      new CustomEvent(furnitureEvents.statusOverflow, {
        bubbles: true,
        composed: true,
        detail: {
          shown: this.shown.map((segment) => segment.label),
          overflow: this.overflowed.map((segment) => segment.label),
        },
      }),
    );
  }

  #onTrigger = (): void => {
    this.#setOpen(!this.#open);
  };

  #setOpen(open: boolean): void {
    this.#open = open;
    this.#trigger?.setAttribute('aria-expanded', String(open));
    const panel = this.#panel;
    if (panel !== undefined) panel.hidden = !open || this.overflowed.length === 0;
  }

  #onSegmentChange = (event: CustomEvent<{ value: string; announce: StatusAnnouncement }>): void => {
    const segment = event.target;
    if (!(segment instanceof MjxStatusSegment)) return;
    const role = statusAnnouncementRoles[segment.announce];
    if (role === undefined) return;
    const region = role === 'alert' ? this.#alert : this.#polite;
    if (region === undefined) return;
    region.textContent = segment.announcement;
  };
}

/** Register the element. Idempotent. */
export function defineStatusBar(): void {
  if (customElements.get(furnitureTags.statusBar) === undefined) {
    customElements.define(furnitureTags.statusBar, MjxStatusBar);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-status-bar': MjxStatusBar;
  }
}
