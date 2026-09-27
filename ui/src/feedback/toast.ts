/**
 * `<mjx-toast-region>` and `<mjx-toast>` — **a queue, and the politeness that is its whole
 * accessibility contract.**
 *
 * ```html
 * <mjx-toast-region label="Notifications">
 *   <mjx-toast tone="success" message="Saved to OneDrive."></mjx-toast>
 * </mjx-toast-region>
 * ```
 * ```ts
 * region.show({ tone: 'error', message: 'Could not reach the server.' });
 * ```
 *
 * ## Two live regions, never one whose politeness is rewritten
 *
 * MJXOFF-189 names the assertion that matters: *"an assertive announcement is not made for a polite
 * toast."* The way that goes wrong is not a missing `aria-live` — it is **one** region whose
 * `aria-live` is swapped per message. A live region's politeness is settled when it enters the
 * accessibility tree, and rewriting the attribute afterwards produces, depending on the screen
 * reader, either an announcement nobody hears or a polite message that interrupts what a person was
 * reading. Both are silent failures on screen.
 *
 * So the shadow root holds **two** visually-hidden regions that never change — `role="status"` with
 * `aria-live="polite"`, and `role="alert"` with `aria-live="assertive"` — and a toast's text is
 * written into exactly the one its tone names. `tests/browser/feedback.spec.ts` asserts the message
 * is in one and that the other is **empty**, which is the contract in one assertion and is the
 * thing a single-region implementation cannot satisfy.
 *
 * The visible stack is separate and is not a live region at all. That is deliberate: it holds the
 * dismiss and action buttons, so it has to stay reachable, and a card that was both drawn and
 * announced would be announced twice.
 *
 * ## The queue is pure and the region only wires a clock to it
 *
 * Everything that could go wrong with a stack of timers — a second toast cancelling the first one's
 * timer, an error being pushed off by three “Saved” messages, the order reversing — is a property
 * of a *sequence of instants*, and `ToastQueue` in the model is swept over one in Node. This
 * element owns a single `setTimeout` armed at `queue.nextExpiry()` and re-armed after every change,
 * which is the one thing Node cannot prove and the browser can.
 *
 * ⚠ **One timeout for the whole queue, and not one per toast.** Not an optimisation: a timer per
 * entry is precisely the shape in which *"showing a second toast restarted the first one's clock"*
 * hides, because the bug is then a missing `clearTimeout` rather than an arithmetic error, and the
 * arithmetic is what a test can see. Here every deadline is a number on an entry and the timer is
 * derived from them, so there is nothing to forget to cancel.
 *
 * ## `<mjx-toast>` is a descriptor, and it goes through the same queue
 *
 * A markup path matters for the catalogue — an auditor needs a stack that is simply *there* — and
 * two rendering paths would be two chances to disagree. So a declared toast is **pushed** exactly
 * as a programmatic one is, and `paused` is what stops the clock while a person is looking at it.
 * The card a story writes and the card `show()` produces are the same card.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import { resolveDurationMilliseconds } from '../foundations/motion.ts';
import {
  clearPlacement,
  clippingAncestor,
  clippingBoundary,
  floatingProperties,
  installFloatingProperties,
  pinFloating,
  resolveLength,
  syncTopLayer,
  type Direction,
} from '../overlay/floating.ts';
import {
  ToastQueue,
  feedbackEvents,
  feedbackMotionClasses,
  feedbackTags,
  feedbackTimingMilliseconds,
  feedbackTimingProperties,
  feedbackTypeRoles,
  isToastTone,
  politenessRoles,
  toastCss,
  toastDismissIcon,
  toastDismissLabel,
  toastPolitenessNames,
  toastTones,
  type ToastEntry,
  type ToastPoliteness,
  type ToastRequest,
  type ToastTone,
} from './feedback-model.ts';

/** How much of its boundary the region may take. All of it: the cards size themselves. */
const regionFraction = 1;

/** The default tone a declared toast takes when it does not name one. */
const defaultTone: ToastTone = 'info';

export class MjxToastRegion extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'paused'];

  #root: ShadowRoot | undefined;
  #region: HTMLElement | undefined;
  #slot: HTMLSlotElement | undefined;
  #announcers = new Map<ToastPoliteness, HTMLElement>();
  #queue = new ToastQueue();
  #timer: ReturnType<typeof setTimeout> | undefined;
  #observer: ResizeObserver | undefined;
  #watching = false;
  #declaredSeen = new WeakSet<Element>();
  /** The card on screen for each entry, so a push does not rebuild the ones already there. */
  #cards = new Map<string, HTMLElement>();

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.#readDeclared();
    this.render();
  }

  disconnectedCallback(): void {
    this.#disarm();
    this.#stopWatching();
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The stack's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /**
   * Whether the clock is stopped.
   *
   * What a catalogue story sets so an auditor can look at a stack without it dissolving underneath
   * them — and what an application sets while a pointer is over the stack, which is WCAG 2.2
   * §2.2.1 for anything that disappears on a timer.
   */
  get paused(): boolean {
    return this.hasAttribute('paused');
  }

  set paused(value: boolean) {
    if (value) this.setAttribute('paused', '');
    else this.removeAttribute('paused');
  }

  /** What is on screen, oldest first. */
  get entries(): readonly ToastEntry[] {
    return this.#queue.entries;
  }

  /** The two live regions, so a gate can read what each was told rather than what it hopes. */
  announcer(politeness: ToastPoliteness): HTMLElement | undefined {
    return this.#announcers.get(politeness);
  }

  /** How long a tone dwells here, in milliseconds, or `undefined` when it never leaves. */
  dwellFor(tone: ToastTone): number | undefined {
    const dwell = toastTones[tone].dwell;
    if (dwell === 'persistent') return undefined;
    const box = this.#region ?? this;
    return (
      resolveDurationMilliseconds(box, feedbackTimingProperties[dwell]) ??
      feedbackTimingMilliseconds(dwell)
    );
  }

  /** Push one. Returns the entry, so a caller can dismiss it later by id. */
  show(request: ToastRequest): ToastEntry {
    const now = Date.now();
    const { entry, retired } = this.#queue.push(request, now, (tone) => this.dwellFor(tone));
    this.#announce(entry);
    for (const departure of retired) this.#emitDismiss(departure.entry, 'retired');
    this.dispatchEvent(
      new CustomEvent(feedbackEvents.toastShow, {
        bubbles: true,
        composed: true,
        detail: { id: entry.id, tone: entry.tone, politeness: toastTones[entry.tone].politeness },
      }),
    );
    this.render();
    return entry;
  }

  /** Remove one by id. */
  dismiss(id: string): boolean {
    const entry = this.#queue.dismiss(id);
    if (entry === undefined) return false;
    this.#emitDismiss(entry, 'dismissed');
    this.render();
    return true;
  }

  /** Remove everything. */
  clear(): void {
    for (const departure of this.#queue.clear()) this.#emitDismiss(departure.entry, 'cleared');
    this.render();
  }

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, toastCss);
    defineIcon();

    const region = document.createElement('div');
    region.className = 'region';
    region.setAttribute('part', 'region');
    // A plain labelled group: the *stack* is a box, and the announcement is the two regions below.
    // A `role="log"` here would announce every card a second time, in the wrong politeness for
    // half of them, and `role="presentation"` may not carry the name the stack needs.
    region.setAttribute('role', 'group');
    this.#region = region;

    root.append(region);

    /*
     * The two announcers, built once and never re-roled.
     *
     * They live outside `.region` on purpose: `.region` is a popover, and a popover that is not
     * showing is `display: none` — which takes a live region out of the accessibility tree
     * entirely. An announcer inside it would work exactly until the stack emptied once.
     */
    for (const politeness of toastPolitenessNames) {
      const announcer = document.createElement('div');
      announcer.className = 'visually-hidden';
      announcer.setAttribute('part', `announcer-${politeness}`);
      announcer.setAttribute('role', politenessRoles[politeness]);
      announcer.setAttribute('aria-live', politeness);
      announcer.setAttribute('aria-atomic', 'true');
      root.append(announcer);
      this.#announcers.set(politeness, announcer);
    }

    const slot = document.createElement('slot');
    slot.hidden = true;
    slot.addEventListener('slotchange', this.#onSlotChange);
    root.append(slot);
    this.#slot = slot;
  }

  #onSlotChange = (): void => {
    this.#readDeclared();
    this.render();
  };

  /**
   * Push every declared `<mjx-toast>` that has not been pushed already.
   *
   * Matched by `localName` rather than `instanceof`, for the reason `inputs/descriptors.ts` states:
   * custom elements upgrade in tree order, so a region's `connectedCallback` can run before its
   * toasts have upgraded and an `instanceof` that ran too early would report an empty stack.
   */
  #readDeclared(): void {
    const assigned = this.#slot?.assignedElements({ flatten: true }) ?? [...this.children];
    for (const child of assigned) {
      if (child.localName !== feedbackTags.toast) continue;
      if (this.#declaredSeen.has(child)) continue;
      this.#declaredSeen.add(child);
      const declared = child.getAttribute('tone');
      const tone = isToastTone(declared) ? declared : defaultTone;
      const message = child.getAttribute('message') ?? '';
      const actionLabel = child.getAttribute('action-label');
      const actionCommand = child.getAttribute('action-command');
      this.show({
        tone,
        message,
        ...(actionLabel === null || actionLabel === ''
          ? {}
          : { action: { label: actionLabel, command: actionCommand ?? actionLabel } }),
      });
    }
  }

  #announce(entry: ToastEntry): void {
    const politeness = toastTones[entry.tone].politeness;
    const announcer = this.#announcers.get(politeness);
    if (announcer === undefined) return;
    // Cleared first: an `aria-atomic` region whose text is replaced by an equal string announces
    // nothing at all, and two identical toasts in a row is exactly the case an application has.
    announcer.textContent = '';
    announcer.textContent = entry.message;
  }

  #emitDismiss(entry: ToastEntry, reason: string): void {
    this.dispatchEvent(
      new CustomEvent(feedbackEvents.toastDismiss, {
        bubbles: true,
        composed: true,
        detail: { id: entry.id, tone: entry.tone, reason },
      }),
    );
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const region = this.#region;
    if (region === undefined) return;

    region.setAttribute('aria-label', this.label);
    const entries = this.#queue.entries;
    const open = entries.length > 0;
    region.setAttribute('data-open', open ? 'true' : 'false');

    /*
     * ⚠ **A card that is already on screen is kept, never rebuilt.**
     *
     * The obvious `replaceChildren(...entries.map(card))` is one line shorter and wrong in a way no
     * assertion in either tier would catch: every card carries an entry animation, so rebuilding
     * the list on each push makes *every* toast fade in again whenever a new one arrives. The
     * counts, the order, the announcements and the timers would all still be right, and the stack
     * would flicker on every message.
     */
    for (const [id, card] of this.#cards) {
      if (!entries.some((entry) => entry.id === id)) {
        card.remove();
        this.#cards.delete(id);
      }
    }
    region.replaceChildren(
      ...entries.map((entry) => {
        const existing = this.#cards.get(entry.id);
        if (existing !== undefined) return existing;
        const card = this.#card(entry);
        this.#cards.set(entry.id, card);
        return card;
      }),
    );

    syncTopLayer(region, { inTopLayer: true, open, connected: this.isConnected });

    if (open) {
      this.#place();
      this.#startWatching();
    } else {
      clearPlacement(region);
      this.#stopWatching();
    }
    this.#arm();
  }

  #card(entry: ToastEntry): HTMLElement {
    const spec = toastTones[entry.tone];
    const card = document.createElement('div');
    card.className = `toast ${feedbackMotionClasses.toast}`;
    card.setAttribute('part', 'toast');
    // The tone's edge is painted by a rule in `toastCss` keyed off this attribute, not written here.
    // A colour set from JavaScript is a colour no cascade gate can read back.
    card.dataset['tone'] = entry.tone;
    card.dataset['toastId'] = entry.id;
    card.dataset['entering'] = '';
    requestAnimationFrame(() => {
      delete card.dataset['entering'];
    });

    const glyph = document.createElement('span');
    glyph.className = 'glyph';
    const icon = document.createElement('mjx-icon');
    icon.setAttribute('name', spec.icon.name);
    icon.setAttribute('size', String(spec.icon.size));
    icon.setAttribute('variant', spec.icon.variant);
    glyph.append(icon);

    const message = document.createElement('p');
    message.className = `message ${feedbackTypeRoles.toastMessage}`;
    message.textContent = entry.message;

    const actions = document.createElement('div');
    actions.className = 'actions';

    if (entry.action !== undefined) {
      const action = document.createElement('button');
      action.type = 'button';
      action.className = `toast-action mjx-hit-target ${feedbackTypeRoles.toastAction} ${feedbackMotionClasses.toast}`;
      action.textContent = entry.action.label;
      const command = entry.action.command;
      action.addEventListener('click', () => {
        this.dispatchEvent(
          new CustomEvent(feedbackEvents.command, {
            bubbles: true,
            composed: true,
            detail: { command, toast: entry.id },
          }),
        );
        this.dismiss(entry.id);
      });
      actions.append(action);
    }

    const dismiss = document.createElement('button');
    dismiss.type = 'button';
    dismiss.className = `toast-dismiss mjx-hit-target ${feedbackMotionClasses.toast}`;
    dismiss.setAttribute('aria-label', toastDismissLabel(entry.message));
    const dismissGlyph = document.createElement('mjx-icon');
    dismissGlyph.setAttribute('name', toastDismissIcon.name);
    dismissGlyph.setAttribute('size', String(toastDismissIcon.size));
    dismiss.append(dismissGlyph);
    dismiss.addEventListener('click', () => {
      this.dismiss(entry.id);
    });
    actions.append(dismiss);

    card.append(glyph, message, actions);
    return card;
  }

  #place(): void {
    const region = this.#region;
    if (region === undefined) return;
    clearPlacement(region);
    const inset = resolveLength(region, floatingProperties.boundaryInset);
    pinFloating(region, clippingBoundary(region, inset), regionFraction, 'blockEnd', this.#direction());
  }

  #direction(): Direction {
    const view = this.ownerDocument.defaultView;
    if (view === null) return 'ltr';
    return view.getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  // ── the clock ──────────────────────────────────────────────────────────────

  /** Arm one timeout at the soonest deadline in the queue. See the module note. */
  #arm(): void {
    this.#disarm();
    if (this.paused) return;
    const next = this.#queue.nextExpiry();
    if (next === undefined) return;
    const wait = Math.max(next - Date.now(), 0);
    this.#timer = setTimeout(() => {
      this.#timer = undefined;
      const gone = this.#queue.expire(Date.now());
      for (const departure of gone) this.#emitDismiss(departure.entry, departure.reason);
      this.render();
    }, wait);
  }

  #disarm(): void {
    if (this.#timer === undefined) return;
    clearTimeout(this.#timer);
    this.#timer = undefined;
  }

  #startWatching(): void {
    if (this.#watching) return;
    this.#watching = true;
    const region = this.#region;
    if (region === undefined) return;
    if (typeof ResizeObserver !== 'undefined') {
      const observer = new ResizeObserver(this.#onResize);
      observer.observe(region);
      const ancestor = clippingAncestor(region);
      if (ancestor !== undefined) observer.observe(ancestor);
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
    if (this.#queue.size > 0) this.#place();
  };
}

/**
 * `<mjx-toast>` — a declared toast. **Data written as markup, never content.**
 *
 * Read for its attributes by the region that holds it and never rendered, exactly as `<mjx-option>`
 * is. `role="none"` as well as `display: none`, because a descriptor a stylesheet failed to reach
 * must still not be a node in the accessibility tree.
 */
export class MjxToast extends HTMLElement {
  connectedCallback(): void {
    this.setAttribute('role', 'none');
    this.hidden = true;
  }
}

/** Register both. Idempotent. */
export function defineToasts(): void {
  if (customElements.get(feedbackTags.toastRegion) === undefined) {
    customElements.define(feedbackTags.toastRegion, MjxToastRegion);
  }
  if (customElements.get(feedbackTags.toast) === undefined) {
    customElements.define(feedbackTags.toast, MjxToast);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-toast-region': MjxToastRegion;
    'mjx-toast': MjxToast;
  }
}
