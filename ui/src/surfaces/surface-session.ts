/**
 * The lifecycle a dismissible surface has — **opened from somewhere, stacked on something, and
 * closed back to where it came from.**
 *
 * `<mjx-dialog>` and `<mjx-popover>` both have it, `<mjx-task-pane>` deliberately has none of it,
 * and it is a class the two own rather than a base they extend — the same decision `ListSurface`
 * records for the same reason: *custom elements upgrade in tree order*, so anything that is itself
 * an element is sometimes built before the thing that has to fill it, while a plain object is
 * constructed by its owner when its owner is ready.
 *
 * ## What it is actually for: the two defects the ticket names
 *
 * > **A modal that traps focus is easy; a modal that returns it is where the defect lives.**
 *
 * So the invoker is captured **once**, at the moment the surface opens, from the deep active
 * element — not from an attribute a caller has to remember to set, and not re-read on close, when
 * focus is already inside the surface. Every close path funnels through one `close()`, so there is
 * exactly one place the return happens and no path can skip it.
 *
 * > **Stacking is the invisible one.**
 *
 * A surface's parent is discovered from the DOM rather than declared: the nearest flat-tree
 * ancestor that is itself an open surface. That is what makes *a popover opened from a dialog*
 * stack without either component knowing the other exists, and what lets closing the dialog take
 * the popover with it while closing the popover leaves the dialog alone. `SurfaceStack` in the
 * model holds the ordering; this holds the elements it is ordering.
 */

import {
  dismissible,
  openSurfaces,
  surfaceEvents,
  surfaceKinds,
  type SurfaceCloseReason,
  type SurfaceKind,
} from './surface-model.ts';
import { flatTreeParent } from '../overlay/floating.ts';
import {
  focusInto,
  holdBackgroundInert,
  returnFocusTo,
  deepActiveElement,
  type BackgroundHold,
} from '../overlay/modality.ts';

/** The attribute every dismissible surface's host carries while it exists. */
export const surfaceIdAttribute = 'data-mjx-surface-id';

/** What a session needs of the component that owns it. */
export interface SurfaceSessionHost {
  /** The custom element itself. Its siblings are what a modal makes inert. */
  readonly element: HTMLElement;
  /** The box that is drawn — the thing focus moves into. */
  surfaceBox(): HTMLElement | undefined;
  /** Which of the six this surface currently is. Re-read on every open, because width decides. */
  currentKind(): SurfaceKind;
  /** Put the `open` state back into the DOM. Called after the session has changed it. */
  reflectOpen(open: boolean): void;
}

let nextSurfaceSerial = 0;

/** Every open session, by id, so closing an outer surface can reach the inner ones. */
const liveSessions = new Map<string, SurfaceSession>();

export class SurfaceSession {
  readonly id: string;
  #host: SurfaceSessionHost;
  #open = false;
  #invoker: HTMLElement | undefined;
  #hold: BackgroundHold | undefined;
  #kind: SurfaceKind = 'dialog';

  constructor(host: SurfaceSessionHost) {
    this.#host = host;
    nextSurfaceSerial += 1;
    this.id = `mjx-surface-${String(nextSurfaceSerial)}`;
    host.element.setAttribute(surfaceIdAttribute, this.id);
  }

  get open(): boolean {
    return this.#open;
  }

  /** The kind this surface was when it last opened. */
  get kind(): SurfaceKind {
    return this.#kind;
  }

  /** How many elements the background hold marked. Zero while closed, or while not modal. */
  get backgroundHeld(): number {
    return this.#hold?.held ?? 0;
  }

  /** The element focus will be returned to. Exposed so a gate can name it. */
  get invoker(): HTMLElement | undefined {
    return this.#invoker;
  }

  /**
   * Open, remembering where the keyboard was.
   *
   * `invoker` may be supplied by a component that owns its own trigger — a popover's anchor
   * button, which is inside its shadow root and is therefore not what `deepActiveElement` reports
   * when the surface is opened programmatically from a story.
   */
  begin(invoker?: HTMLElement): void {
    if (this.#open) return;
    this.#open = true;
    this.#kind = this.#host.currentKind();
    const active = deepActiveElement(this.#host.element.ownerDocument);
    this.#invoker =
      invoker ?? (active instanceof HTMLElement && active !== this.#host.element ? active : undefined);
    liveSessions.set(this.id, this);
    openSurfaces.push({ id: this.id, kind: this.#kind, parent: this.#parentSurfaceId() });
    if (surfaceKinds[this.#kind].modal) {
      // ⚠ The **host**, not the box that is drawn. The walk marks every flat-tree sibling of every
      // ancestor, and the scrim is a sibling of the surface inside this component's own shadow
      // root — marking it `inert` would make the one click that dismisses a modal do nothing.
      this.#hold = holdBackgroundInert(this.#host.element);
    }
    this.#host.reflectOpen(true);
    this.#host.element.dispatchEvent(
      new CustomEvent(surfaceEvents.open, {
        bubbles: true,
        composed: true,
        detail: { kind: this.#kind, id: this.id },
      }),
    );
  }

  /** Move the keyboard into the surface. Separate from `begin`, because placement happens between. */
  takeFocus(): HTMLElement | undefined {
    const box = this.#host.surfaceBox();
    if (box === undefined) return undefined;
    return focusInto(box);
  }

  /**
   * Close, if this reason is one this kind allows.
   *
   * Returns whether anything happened, so a caller can tell *refused* from *already closed* —
   * which is the difference a task pane's gate is asserting and a shared sweep would flatten.
   */
  close(reason: SurfaceCloseReason): boolean {
    if (!this.#open) return false;
    if (!dismissible(this.#kind, reason)) return false;
    this.#finish(reason);
    return true;
  }

  /** Close whatever the reason, for a component that is being removed from the document. */
  dispose(): void {
    if (this.#open) this.#finish('programmatic');
    liveSessions.delete(this.id);
    this.#host.element.removeAttribute(surfaceIdAttribute);
  }

  #finish(reason: SurfaceCloseReason): void {
    // Innermost first: an outer surface's close restores focus, and doing that before its children
    // have let go of theirs puts the keyboard somewhere a child is about to take back.
    for (const entry of openSurfaces.remove(this.id)) {
      if (entry.id === this.id) continue;
      const inner = liveSessions.get(entry.id);
      if (inner !== undefined) inner.#finish('programmatic');
    }
    this.#open = false;
    this.#hold?.release();
    this.#hold = undefined;
    this.#host.reflectOpen(false);
    const returned = returnFocusTo(this.#invoker);
    const invoker = this.#invoker;
    this.#invoker = undefined;
    liveSessions.delete(this.id);
    this.#host.element.dispatchEvent(
      new CustomEvent(surfaceEvents.close, {
        bubbles: true,
        composed: true,
        detail: { kind: this.#kind, id: this.id, reason, returnedFocus: returned, invoker },
      }),
    );
  }

  /**
   * The nearest open surface this one is inside, by the flat tree.
   *
   * ⚠ **The flat tree, not `parentNode`.** A popover opened from inside a dialog's *slotted*
   * content reaches that dialog only through the slot it was assigned to, and a `parentNode` walk
   * would report no parent at all — which is exactly the case the stacking gate drives, and it
   * would have passed by reporting two independent surfaces instead of a nested pair.
   */
  #parentSurfaceId(): string | undefined {
    let node: Node | null = flatTreeParent(this.#host.element);
    while (node !== null) {
      if (node instanceof HTMLElement) {
        const id = node.getAttribute(surfaceIdAttribute);
        if (id !== null && id !== this.id && openSurfaces.has(id)) return id;
      }
      node = flatTreeParent(node);
    }
    return undefined;
  }
}
