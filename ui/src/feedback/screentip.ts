/**
 * `<mjx-screentip>` — **the enhanced tip, and the delay that is the whole of it.**
 *
 * ```html
 * <mjx-screentip heading="Bold" description="Make the selected text bold."
 *                shortcut="Ctrl + B">
 *   <mjx-button slot="trigger" label="Bold" icon="text-bold" size="icon"></mjx-button>
 * </mjx-screentip>
 * ```
 *
 * ## A delay is where a vacuous gate lives, so the delay is the contract
 *
 * MJXOFF-189 names it exactly: *"'It appeared' is satisfied by one that appears instantly."* So
 * three things are asserted rather than one — that the tip is **not** there before the delay, that
 * it **is** there after it, and that the delay the component actually resolved **equals the token
 * arithmetic**. The third is what stops a broken `@property` registration turning the first two
 * into a tautology: an unregistered `<time>` resolves to text, `resolveDurationMilliseconds`
 * returns `undefined` for it, and a component that fell back to zero would appear instantly and
 * pass *"it appeared"* every time.
 *
 * The fallback here is therefore the **generated token value**, never zero.
 *
 * ## The pointer waits and the keyboard does not
 *
 * A pointer crosses a toolbar on its way somewhere; a keyboard arrives on a control because a
 * person put it there. Those are different intentions and they get different answers: hovering
 * waits `--mjx-screentip-appear-delay`, focusing shows immediately. The warm-up in
 * `screentipWarmth` then removes the wait for the *next* trigger within
 * `--mjx-screentip-warm-window`, which is what makes a row of icon commands readable rather than a
 * row of things that will not tell you what they are.
 *
 * ## The description is announced whether or not the tip is drawn
 *
 * ⚠ **And that is why it lives in the light DOM.** An IDREF resolves within its own tree, so
 * `aria-describedby` on a *slotted* trigger cannot name an element inside this component's shadow
 * root — the trigger is in the document's tree and the tip is in a descendant tree, which is the
 * one direction ARIA element reflection does not reach either. A tip whose text existed only in the
 * shadow root would therefore be perfectly visible and completely unannounced, with every visual
 * assertion green.
 *
 * So the component writes one visually-hidden `<span>` **as its own light-DOM child**, styled by
 * `feedbackDocumentCss`, and points the trigger's `aria-describedby` at it. It is present the whole
 * time, not only while the tip is showing, which is also what WAI's tooltip pattern asks for: a
 * screen-reader user gets the description on focus regardless of a delay that exists for eyes. The
 * drawn tip is `aria-hidden`, so nothing is announced twice.
 *
 * ## Escape hides it and moves nothing
 *
 * WAI-ARIA APG, and MJXOFF-189 asks for it by name. A tip that returned focus somewhere on Escape
 * would be moving a keyboard that never left the trigger. `tests/browser/feedback.spec.ts` records
 * the focused element before and after and requires them to be the same one.
 *
 * ## It stays while the pointer travels into it
 *
 * WCAG 2.2 §1.4.13. Leaving the trigger schedules the hide `--mjx-screentip-leave-grace` later;
 * entering the tip cancels it. Without that a person using magnification cannot reach a tip that is
 * wider than what they can see.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import { resolveDurationMilliseconds } from '../foundations/motion.ts';
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
} from '../overlay/floating.ts';
import {
  feedbackEvents,
  feedbackMotionClasses,
  feedbackTags,
  feedbackTimingMilliseconds,
  feedbackTimingProperties,
  feedbackTypeRoles,
  screentipClearanceOrder,
  screentipCss,
  screentipDescriptionClass,
  screentipWarmth,
  type ScreentipHideReason,
  type ScreentipSchedule,
} from './feedback-model.ts';

let nextScreentipSerial = 0;

export class MjxScreentip extends HTMLElement {
  static readonly observedAttributes: readonly string[] = [
    'heading',
    'description',
    'shortcut',
    'open',
  ];

  #root: ShadowRoot | undefined;
  #tip: HTMLElement | undefined;
  #headingElement: HTMLElement | undefined;
  #descriptionElement: HTMLElement | undefined;
  #shortcutElement: HTMLElement | undefined;
  #triggerSlot: HTMLSlotElement | undefined;
  /** The light-DOM span an assistive technology actually reads. See the module note. */
  #announced: HTMLElement | undefined;
  #appearTimer: ReturnType<typeof setTimeout> | undefined;
  #leaveTimer: ReturnType<typeof setTimeout> | undefined;
  #showing = false;
  #waited = 0;
  #clearance: ClearPlacement | undefined;
  #observer: ResizeObserver | undefined;
  #watching = false;
  #describedId = '';

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.#ensureAnnouncedElement();
    this.render();
    this.ownerDocument.addEventListener('keydown', this.#onDocumentKeyDown, true);
  }

  disconnectedCallback(): void {
    this.#cancelTimers();
    this.#stopWatching();
    this.ownerDocument.removeEventListener('keydown', this.#onDocumentKeyDown, true);
    this.#announced?.remove();
    this.#announced = undefined;
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The tip's first line. Not `title`: that attribute is the platform's own tooltip. */
  get heading(): string {
    return this.getAttribute('heading') ?? '';
  }

  /** The sentence beneath it. Its presence is what makes a screentip *enhanced*. */
  get description(): string {
    return this.getAttribute('description') ?? '';
  }

  /** The keyboard shortcut, shown last and announced with the description. */
  get shortcut(): string {
    return this.getAttribute('shortcut') ?? '';
  }

  /** Whether the tip is drawn right now. Reflected, so a story may force it open. */
  get open(): boolean {
    return this.hasAttribute('open');
  }

  set open(value: boolean) {
    if (value) this.setAttribute('open', '');
    else this.removeAttribute('open');
  }

  /** How long the last appearance actually waited, in milliseconds. */
  get waited(): number {
    return this.#waited;
  }

  /** Where the tip ended up, and whether it cleared its trigger. */
  get clearance(): ClearPlacement | undefined {
    return this.#clearance;
  }

  /** The control in the `trigger` slot. */
  get trigger(): HTMLElement | undefined {
    const assigned = this.#triggerSlot?.assignedElements({ flatten: true }) ?? [];
    const first = assigned[0];
    return first instanceof HTMLElement ? first : undefined;
  }

  /**
   * The two spans, resolved from the cascade, with the **generated token value** as the floor.
   *
   * Never zero: see the module note on why a zero delay would make this component's whole contract
   * unfalsifiable.
   */
  get schedule(): ScreentipSchedule {
    const box = this.#tip ?? this;
    const appear =
      resolveDurationMilliseconds(box, feedbackTimingProperties.screentipAppear) ??
      feedbackTimingMilliseconds('screentipAppear');
    const warm =
      resolveDurationMilliseconds(box, feedbackTimingProperties.screentipWarm) ??
      feedbackTimingMilliseconds('screentipWarm');
    return { appear, warm };
  }

  /** How long the tip lingers after the pointer leaves the trigger. WCAG 2.2 §1.4.13. */
  get leaveGrace(): number {
    const box = this.#tip ?? this;
    return (
      resolveDurationMilliseconds(box, feedbackTimingProperties.screentipLeaveGrace) ??
      feedbackTimingMilliseconds('screentipLeaveGrace')
    );
  }

  /** Show it now, with no wait at all. What a keyboard focus does. */
  show(): void {
    this.#cancelTimers();
    this.#waited = 0;
    this.open = true;
  }

  /** Show it after whatever the schedule says. What a pointer does. */
  showAfterDelay(): void {
    if (this.open) return;
    this.#cancelTimers();
    const delay = screentipWarmth.delayFor(Date.now(), this.schedule);
    if (delay <= 0) {
      this.#waited = 0;
      this.open = true;
      return;
    }
    this.#appearTimer = setTimeout(() => {
      this.#appearTimer = undefined;
      this.#waited = delay;
      this.open = true;
    }, delay);
  }

  /** Hide it. `escape` and `blur` move no focus; nothing here ever does. */
  hide(reason: ScreentipHideReason = 'programmatic'): boolean {
    this.#cancelTimers();
    if (!this.open) return false;
    this.open = false;
    screentipWarmth.noteHidden(Date.now());
    this.dispatchEvent(
      new CustomEvent(feedbackEvents.screentipHide, {
        bubbles: true,
        composed: true,
        detail: { title: this.heading, reason },
      }),
    );
    return true;
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, screentipCss);

    nextScreentipSerial += 1;
    this.#describedId = `mjx-screentip-${String(nextScreentipSerial)}`;

    const slot = document.createElement('slot');
    slot.name = 'trigger';
    slot.addEventListener('slotchange', this.#onSlotChange);
    this.#triggerSlot = slot;

    const tip = document.createElement('div');
    tip.className = `tip anchored ${feedbackMotionClasses.tip}`;
    tip.setAttribute('part', 'tip');
    // Announced through the light-DOM description instead, so nothing is read twice. It carries no
    // focusable content, which is what keeps `aria-hidden-focus` satisfied.
    tip.setAttribute('aria-hidden', 'true');
    tip.addEventListener('pointerenter', this.#onTipEnter);
    tip.addEventListener('pointerleave', this.#onPointerLeave);
    this.#tip = tip;

    const heading = document.createElement('p');
    heading.className = `tip-title ${feedbackTypeRoles.tipTitle}`;
    heading.setAttribute('part', 'heading');
    this.#headingElement = heading;

    const description = document.createElement('p');
    description.className = `tip-description ${feedbackTypeRoles.tipBody}`;
    description.setAttribute('part', 'description');
    this.#descriptionElement = description;

    const shortcut = document.createElement('p');
    shortcut.className = `tip-shortcut ${feedbackTypeRoles.tipBody}`;
    shortcut.setAttribute('part', 'shortcut');
    this.#shortcutElement = shortcut;

    tip.append(heading, description, shortcut);
    root.append(slot, tip);

    /*
     * ⚠ **`pointerenter`/`pointerleave` on the host, and no capture.** They do not bubble, and
     * that is exactly why they are the right pair: like `mouseenter`, each is dispatched
     * *separately* to every element being entered or left, with `target` set to that element — so
     * a listener here answers *"the pointer is somewhere inside this component"* and never fires
     * again as the pointer moves between the trigger and the tip. `pointerover` would fire on
     * every internal move and would need the same question re-derived from `relatedTarget`.
     *
     * It also gives §1.4.13 for free in the common case: the tip is a *DOM* descendant of this
     * host even while it is in the top layer, so travelling from the trigger into the tip leaves
     * nothing. The grace timer below is what covers the gap between them, where the pointer is
     * over neither.
     */
    this.addEventListener('pointerenter', this.#onPointerEnter);
    this.addEventListener('pointerleave', this.#onPointerLeave);
    this.addEventListener('focusin', this.#onFocusIn);
    this.addEventListener('focusout', this.#onFocusOut);
    this.addEventListener('click', this.#onClick, true);
  }

  /**
   * The light-DOM span the trigger's `aria-describedby` names.
   *
   * Created as this element's own child rather than appended to the page, so removing the component
   * removes its description with it and a story that renders twice does not leave one behind.
   *
   * ⚠ **It is deliberately assigned to no slot, and that is what hides it.** This component's
   * shadow root has one *named* slot and no default, so an unslotted light child generates no box
   * at all — and a node **directly referenced** by `aria-describedby` still contributes its text to
   * the accessible description even when it is not rendered, which is the whole reason this
   * arrangement works. The `.mjx-screentip-description` rule in `feedbackDocumentCss` is the belt to
   * that brace: it keeps the span invisible if a later shadow tree ever grows a default slot.
   */
  #ensureAnnouncedElement(): void {
    if (this.#announced !== undefined && this.#announced.isConnected) return;
    const announced = this.ownerDocument.createElement('span');
    announced.className = screentipDescriptionClass;
    announced.id = this.#describedId;
    this.append(announced);
    this.#announced = announced;
  }

  #onSlotChange = (): void => {
    this.#syncTrigger();
  };

  #syncTrigger(): void {
    const trigger = this.trigger;
    if (trigger === undefined) return;
    trigger.setAttribute('aria-describedby', this.#describedId);
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const tip = this.#tip;
    if (tip === undefined) return;

    const heading = this.heading;
    const description = this.description;
    const shortcut = this.shortcut;

    if (this.#headingElement !== undefined) this.#headingElement.textContent = heading;
    if (this.#descriptionElement !== undefined) {
      this.#descriptionElement.textContent = description;
      this.#descriptionElement.hidden = description === '';
    }
    if (this.#shortcutElement !== undefined) {
      this.#shortcutElement.textContent = shortcut;
      this.#shortcutElement.hidden = shortcut === '';
    }

    this.#ensureAnnouncedElement();
    if (this.#announced !== undefined) {
      this.#announced.textContent = [heading, description, shortcut]
        .filter((part) => part !== '')
        .join('. ');
    }
    this.#syncTrigger();

    const open = this.open;
    tip.setAttribute('data-open', open ? 'true' : 'false');
    syncTopLayer(tip, { inTopLayer: true, open, connected: this.isConnected });

    if (open) {
      if (!this.#showing) {
        this.#showing = true;
        this.dispatchEvent(
          new CustomEvent(feedbackEvents.screentipShow, {
            bubbles: true,
            composed: true,
            detail: { title: heading, waited: this.#waited },
          }),
        );
      }
      this.#enter();
      this.#place();
      this.#startWatching();
    } else {
      this.#showing = false;
      clearPlacement(tip);
      delete tip.dataset['entering'];
      delete tip.dataset['entered'];
      this.#clearance = undefined;
      this.#stopWatching();
    }
  }

  #enter(): void {
    const tip = this.#tip;
    if (tip === undefined) return;
    if (tip.dataset['entered'] === 'true') return;
    tip.dataset['entered'] = 'true';
    tip.dataset['entering'] = '';
    requestAnimationFrame(() => {
      delete tip.dataset['entering'];
    });
  }

  #place(): void {
    const tip = this.#tip;
    const trigger = this.trigger;
    if (tip === undefined || trigger === undefined) return;
    clearPlacement(tip);
    const gap = resolveLength(tip, floatingProperties.gap);
    const inset = resolveLength(tip, floatingProperties.boundaryInset);
    const box = tip.getBoundingClientRect();
    const clearance = placeClearOfAnchor(
      {
        anchor: rectOf(trigger),
        floating: { width: box.width, height: box.height },
        boundary: clippingBoundary(tip, inset),
        align: 'start',
        direction: this.#direction(),
        gap,
      },
      screentipClearanceOrder,
    );
    applyPlacement(tip, clearance.placement);
    this.#clearance = clearance;
  }

  #direction(): Direction {
    const view = this.ownerDocument.defaultView;
    if (view === null) return 'ltr';
    return view.getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  // ── the ways it comes and goes ─────────────────────────────────────────────

  #onPointerEnter = (): void => {
    if (this.#leaveTimer !== undefined) {
      clearTimeout(this.#leaveTimer);
      this.#leaveTimer = undefined;
    }
    this.showAfterDelay();
  };

  #onTipEnter = (): void => {
    if (this.#leaveTimer === undefined) return;
    clearTimeout(this.#leaveTimer);
    this.#leaveTimer = undefined;
  };

  /**
   * Leaving schedules the hide rather than performing it. WCAG 2.2 §1.4.13, and the reason the
   * grace exists at all: the pointer has to be able to cross the gap into the tip.
   */
  #onPointerLeave = (): void => {
    if (this.#appearTimer !== undefined) {
      clearTimeout(this.#appearTimer);
      this.#appearTimer = undefined;
    }
    if (!this.open) return;
    if (this.#leaveTimer !== undefined) clearTimeout(this.#leaveTimer);
    this.#leaveTimer = setTimeout(() => {
      this.#leaveTimer = undefined;
      this.hide('leave');
    }, this.leaveGrace);
  };

  /** A keyboard arrives on purpose, so it waits for nothing. */
  #onFocusIn = (): void => {
    this.show();
  };

  #onFocusOut = (event: FocusEvent): void => {
    const next = event.relatedTarget;
    if (next instanceof Node && this.contains(next)) return;
    this.hide('blur');
  };

  /** Activating a command replaces the tip with whatever the command did. */
  #onClick = (): void => {
    this.hide('activate');
  };

  /**
   * Escape, from the document, and it moves nothing.
   *
   * On the document rather than on this element because a tip shown by hovering has the keyboard
   * somewhere else entirely, and a listener on the host would never see the key. Capturing, and
   * `stopPropagation` only when a tip was actually showing, so a screentip inside a dialog does not
   * eat the Escape that closes it.
   */
  #onDocumentKeyDown = (event: KeyboardEvent): void => {
    if (event.key !== 'Escape') return;
    if (!this.open) return;
    if (this.hide('escape')) {
      event.preventDefault();
      event.stopPropagation();
    }
  };

  #cancelTimers(): void {
    if (this.#appearTimer !== undefined) {
      clearTimeout(this.#appearTimer);
      this.#appearTimer = undefined;
    }
    if (this.#leaveTimer !== undefined) {
      clearTimeout(this.#leaveTimer);
      this.#leaveTimer = undefined;
    }
  }

  #startWatching(): void {
    if (this.#watching) return;
    this.#watching = true;
    const tip = this.#tip;
    if (tip === undefined) return;
    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver(this.#onResize);
      observer.observe(tip);
      const ancestor = clippingAncestor(tip);
      if (ancestor !== undefined) observer.observe(ancestor);
      const trigger = this.trigger;
      if (trigger !== undefined) observer.observe(trigger);
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
export function defineScreentip(): void {
  if (customElements.get(feedbackTags.screentip) === undefined) {
    customElements.define(feedbackTags.screentip, MjxScreentip);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-screentip': MjxScreentip;
  }
}
