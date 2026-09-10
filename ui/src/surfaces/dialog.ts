/**
 * `<mjx-dialog>` — three of the six surfaces: the modeless dialog, the modal, and the sheet the
 * modal becomes at a phone's width.
 *
 * ```html
 * <mjx-dialog label="Save changes" modal open>
 *   <p>Save your changes to Report.docx before closing?</p>
 *   <div slot="footer">
 *     <mjx-button label="Don’t Save"></mjx-button>
 *     <mjx-button label="Save"></mjx-button>
 *   </div>
 * </mjx-dialog>
 * ```
 *
 * ## What is worth reading here, and what is only plumbing
 *
 * **Where focus goes on every close path.** MJXOFF-188: *"A modal that traps focus is easy; a
 * modal that returns it is where the defect lives."* Escape, the close button, the scrim and a
 * programmatic close are four paths and there is **one** `close()`, in `SurfaceSession`, because
 * four paths with four returns is four chances to have three of them right. The close event
 * carries `returnedFocus`, so the gate asserts the return rather than assuming it.
 *
 * **`inert` *and* `aria-hidden`, and the walk that applies them.** See `overlay/modality.ts`. The
 * hold is taken on **this element**, never on the box that is drawn, so the scrim stays clickable;
 * that is a one-word difference that silently costs a dismissal path.
 *
 * **A sheet is a dialog at a phone's width and shares its implementation.** The presentation is
 * read back out of `--mjx-surface-presentation` — never computed here — and the pin is
 * `pinFloating`, which is the same call `<mjx-menu>` and `<mjx-gallery>` make. There is one sheet
 * in this catalogue wearing three names.
 *
 * ## The scrim is a second popover, not a `::backdrop`
 *
 * Both the scrim and the box are `popover="manual"` and both are placed against
 * `clippingBoundary()`. `::backdrop` covers the *window*, and the window is not this catalogue's
 * unit of responsiveness: a modal opened inside a phone-sized frame would dim the whole page
 * around it and report a modality the container never had. See `coverFloating`.
 *
 * ## `GUESS:` where this diverges from Office
 *
 * Office's modal dialogs are OS windows and cannot be dismissed by clicking beside them at all.
 * This one can, because a web modal that traps the keyboard and refuses every pointer gesture is
 * the shape people report as a hung page — and because `dismissals` makes it a per-kind decision
 * rather than a global one, a shell that wants Office's behaviour removes one entry from a table.
 */

import { defineIcon } from '../icons/icon.ts';
import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import {
  centreFloating,
  clearPlacement,
  clippingAncestor,
  clippingBoundary,
  coverFloating,
  floatingCss,
  installFloatingProperties,
  pinFloating,
  resolveLength,
  syncTopLayer,
  floatingProperties,
  type Placement,
} from '../overlay/floating.ts';
import { wrapTab } from '../overlay/modality.ts';
import { SurfaceSession, type SurfaceSessionHost } from './surface-session.ts';
import {
  closeButtonLabel,
  dialogCss,
  dialogPresentationCss,
  sheetBoundaryFraction,
  surfaceCloseIcon,
  surfaceEvents,
  surfaceKinds,
  surfaceMotionClass,
  surfacePresentationProperty,
  surfaceTags,
  surfaceTypeRoles,
  type SurfaceCloseReason,
  type SurfaceKind,
} from './surface-model.ts';

/**
 * How much of its boundary a centred dialog may take.
 *
 * Not a width in pixels: the boundary is the simulated screen, and a dialog that was 480 wide
 * would be wider than a phone and a postage stamp on a desktop. The block fraction is the one that
 * matters — a dialog taller than this scrolls its body rather than running off the screen.
 */
export const dialogBoundaryFractions = { inline: 0.72, block: 0.86 } as const;

// The `@property` registrations travel with the sheet they are in, and this component reads a
// registered length back in pixels — so `floatingCss` goes on this shadow root as well as on the
// document, exactly as `<mjx-menu>` does it.
const styles = `${floatingCss}\n${dialogCss}\n${dialogPresentationCss}`;

let nextTitleSerial = 0;

export class MjxDialog extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'open', 'modal'];

  #root: ShadowRoot | undefined;
  #scrim: HTMLElement | undefined;
  #surface: HTMLElement | undefined;
  #title: HTMLElement | undefined;
  #closeButton: HTMLButtonElement | undefined;
  #session: SurfaceSession | undefined;
  #placement: Placement | undefined;
  #observer: ResizeObserver | undefined;
  #watching = false;
  #titleId = '';

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    installFloatingProperties(this.ownerDocument);
    this.render();
  }

  disconnectedCallback(): void {
    this.#stopWatching();
    this.#session?.dispose();
    this.#session = undefined;
  }

  attributeChangedCallback(): void {
    if (this.#root !== undefined) this.render();
  }

  /** The dialog's accessible name. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  set label(value: string) {
    this.setAttribute('label', value);
  }

  /** Whether it is showing. */
  get open(): boolean {
    return this.hasAttribute('open');
  }

  set open(value: boolean) {
    if (value) this.setAttribute('open', '');
    else this.removeAttribute('open');
  }

  /** Whether it takes the keyboard away from the page. */
  get modal(): boolean {
    return this.hasAttribute('modal');
  }

  set modal(value: boolean) {
    if (value) this.setAttribute('modal', '');
    else this.removeAttribute('modal');
  }

  /**
   * **Which presentation CSS put this dialog in** — read back, never computed here.
   *
   * `dialogPresentationAt()` in the model says what it should be and the browser gate compares the
   * two. A component that decided its own presentation and then reported it would be grading its
   * own homework, which is `<mjx-ribbon-group>`'s rule and this inherits it.
   */
  get presentation(): 'dialog' | 'sheet' {
    const surface = this.#surface;
    if (surface === undefined) return 'dialog';
    const value = getComputedStyle(surface).getPropertyValue(surfacePresentationProperty).trim();
    return value === 'sheet' ? 'sheet' : 'dialog';
  }

  /** Which of the six rows this dialog currently is. */
  get kind(): SurfaceKind {
    if (this.presentation === 'sheet' && this.modal) return 'sheet';
    return this.modal ? 'modal' : 'dialog';
  }

  /** Where it ended up, so a gate can compare against the model's arithmetic. */
  get placement(): Placement | undefined {
    return this.#placement;
  }

  /** The element focus will go back to. */
  get invoker(): HTMLElement | undefined {
    return this.#session?.invoker;
  }

  /** How many background elements the modal hold marked. Zero is a defect and a gate says so. */
  get backgroundHeld(): number {
    return this.#session?.backgroundHeld ?? 0;
  }

  /** Open it, optionally naming what it was opened from. */
  show(invoker?: HTMLElement): void {
    this.#pendingInvoker = invoker;
    this.open = true;
  }

  /** Close it. Refused when the kind does not allow the reason. */
  close(reason: SurfaceCloseReason = 'programmatic'): boolean {
    const closed = this.#session?.close(reason) ?? false;
    if (closed) this.open = false;
    return closed;
  }

  #pendingInvoker: HTMLElement | undefined;

  // ── building ───────────────────────────────────────────────────────────────

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, styles);
    defineIcon();

    nextTitleSerial += 1;
    this.#titleId = `mjx-dialog-title-${String(nextTitleSerial)}`;

    const scrim = document.createElement('div');
    scrim.className = 'scrim';
    scrim.setAttribute('part', 'scrim');
    // Not a button and not focusable: a scrim is a *region a click lands in*, and giving it a role
    // would put a nameless control in the accessibility tree between a person and the dialog. The
    // dismissal it offers is duplicated by Escape and by the close button, both of which are
    // reachable, which is what makes an unreachable one legitimate rather than a shortcut.
    scrim.setAttribute('aria-hidden', 'true');
    scrim.addEventListener('pointerdown', this.#onScrimDown);
    this.#scrim = scrim;

    const surface = document.createElement('div');
    surface.className = `surface ${surfaceMotionClass('modal')}`;
    surface.setAttribute('part', 'surface');
    surface.setAttribute('role', 'dialog');
    surface.setAttribute('aria-labelledby', this.#titleId);
    // A surface with no tab stop at all still has to receive focus, or a screen reader stays where
    // it was and reads nothing.
    surface.tabIndex = -1;
    this.#surface = surface;

    const handle = document.createElement('div');
    handle.className = 'handle';
    handle.setAttribute('part', 'handle');
    handle.setAttribute('aria-hidden', 'true');

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

    const footer = document.createElement('div');
    footer.className = 'footer';
    footer.setAttribute('part', 'footer');
    const footerSlot = document.createElement('slot');
    footerSlot.name = 'footer';
    footer.append(footerSlot);

    surface.append(handle, header, body, footer);
    root.append(scrim, surface);

    this.#session = new SurfaceSession(this.#sessionHost());
    this.addEventListener('keydown', this.#onKeyDown);
  }

  #sessionHost(): SurfaceSessionHost {
    return {
      element: this,
      surfaceBox: () => this.#surface,
      currentKind: () => this.kind,
      reflectOpen: (open) => {
        // The attribute is the single source of truth for whether it is showing, and the session
        // is what decides. Writing it here rather than in `close()` is what makes a programmatic
        // close from an *outer* surface's cascade reach the DOM as well as the model.
        if (open) this.setAttribute('open', '');
        else this.removeAttribute('open');
        this.render();
      },
    };
  }

  // ── rendering ──────────────────────────────────────────────────────────────

  render(): void {
    const surface = this.#surface;
    const scrim = this.#scrim;
    const session = this.#session;
    if (surface === undefined || scrim === undefined || session === undefined) return;

    if (this.#title !== undefined) this.#title.textContent = this.label;
    if (this.#closeButton !== undefined) {
      this.#closeButton.setAttribute('aria-label', closeButtonLabel(this.label));
    }

    const wantsOpen = this.open;
    if (wantsOpen && !session.open) {
      session.begin(this.#pendingInvoker);
      this.#pendingInvoker = undefined;
    } else if (!wantsOpen && session.open) {
      // The attribute was removed from outside. That is a `programmatic` close, and it is the one
      // close path a person cannot take, so it is always allowed.
      session.close('programmatic');
      return;
    }

    const kind = this.kind;
    const spec = surfaceKinds[kind];
    surface.dataset['presentation'] = this.presentation;
    surface.dataset['kind'] = kind;
    surface.dataset['modal'] = String(this.modal);
    surface.setAttribute('data-open', wantsOpen ? 'true' : 'false');
    surface.setAttribute('aria-modal', spec.modal ? 'true' : 'false');
    surface.className = `surface ${surfaceMotionClass(kind)}`;

    scrim.setAttribute('data-open', wantsOpen && spec.scrim ? 'true' : 'false');
    scrim.className = `scrim ${surfaceMotionClass(kind)}`;

    syncTopLayer(scrim, {
      inTopLayer: true,
      open: wantsOpen && spec.scrim,
      connected: this.isConnected,
    });
    syncTopLayer(surface, { inTopLayer: true, open: wantsOpen, connected: this.isConnected });

    if (wantsOpen) {
      this.#enter();
      this.#place();
      this.#startWatching();
    } else {
      clearPlacement(surface);
      clearPlacement(scrim);
      delete surface.dataset['entering'];
      delete scrim.dataset['entering'];
      delete surface.dataset['entered'];
      this.#placement = undefined;
      this.#stopWatching();
    }
  }

  /**
   * One frame of entry, removed on the next.
   *
   * ⚠ The class is added *before* the box is measured and the flag is dropped *after*, because a
   * box that is still translated when it is measured is a box whose placement is computed for a
   * position it is about to leave. The same order `<mjx-menu>` uses.
   */
  #enter(): void {
    const surface = this.#surface;
    const scrim = this.#scrim;
    if (surface === undefined || scrim === undefined) return;
    if (surface.dataset['entered'] === 'true') return;
    surface.dataset['entered'] = 'true';
    surface.dataset['entering'] = '';
    scrim.dataset['entering'] = '';
    requestAnimationFrame(() => {
      delete surface.dataset['entering'];
      delete scrim.dataset['entering'];
    });
    // Focus lands after the placement, on the next frame, so a screen reader is not reading a box
    // that is still being put where it goes.
    requestAnimationFrame(() => {
      if (this.open) this.#session?.takeFocus();
    });
  }

  /**
   * Put it where it goes.
   *
   * A sheet is **pinned** and a dialog is **centred**, and neither is `placeFloating` — there is no
   * anchor. Both come from `overlay/floating.ts` rather than from arithmetic here, which is the
   * whole of MJXOFF-188's instruction about that file.
   */
  #place(): void {
    const surface = this.#surface;
    const scrim = this.#scrim;
    if (surface === undefined || scrim === undefined) return;

    const inset = resolveLength(surface, floatingProperties.boundaryInset);
    const outer = clippingBoundary(surface, 0);
    coverFloating(scrim, outer);

    clearPlacement(surface);
    if (this.presentation === 'sheet') {
      // Inset zero: a sheet sits *on* the edge it is pinned to, which is the whole of what makes it
      // a sheet rather than a very wide dialog.
      this.#placement = pinFloating(surface, outer, sheetBoundaryFraction);
      return;
    }
    this.#placement = centreFloating(surface, clippingBoundary(surface, inset), dialogBoundaryFractions);
  }

  // ── the ways it closes ─────────────────────────────────────────────────────

  #onKeyDown = (event: KeyboardEvent): void => {
    if (!this.open) return;
    if (event.key === 'Escape') {
      if (this.close('escape')) {
        // ⚠ Stop, so an outer surface does not also close. Escape bubbles from whatever has focus,
        // and a popover inside a dialog is a DOM descendant of it — the innermost surface is the
        // one the person means, and it is the one this event reaches first.
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

  #onCloseClick = (): void => {
    this.close('closeButton');
  };

  #onScrimDown = (event: PointerEvent): void => {
    event.preventDefault();
    this.close('scrim');
  };

  // ── keeping it where it goes ───────────────────────────────────────────────

  /**
   * Watch **three** boxes, and start on open rather than on connect.
   *
   * The element itself (its content decides a centred dialog's size), the clipping ancestor (the
   * harness frame, whose width is what turns a dialog into a sheet) and the window. U06's fourth
   * finding is the reason the frame is watched at all: driving the container's width must move the
   * surface, and it must do so *without* a window resize listener, because the window is not what
   * changed.
   */
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
    if (!this.open) return;
    const surface = this.#surface;
    if (surface !== undefined) {
      // The presentation may have crossed the threshold, which changes the corner treatment and
      // the pin. Re-render rather than re-place, so the two stay one decision.
      surface.dataset['presentation'] = this.presentation;
      surface.dataset['kind'] = this.kind;
    }
    this.#place();
  };
}

/** Register the element. Idempotent. */
export function defineDialog(): void {
  if (customElements.get(surfaceTags.dialog) === undefined) {
    customElements.define(surfaceTags.dialog, MjxDialog);
  }
}

/** Re-exported so a story or a gate can name the events without a second import. */
export { surfaceEvents };

declare global {
  interface HTMLElementTagNameMap {
    'mjx-dialog': MjxDialog;
  }
}
