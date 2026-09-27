/**
 * **Modality** — making the rest of a page genuinely unreachable, counting the tab stops inside a
 * surface, and putting focus back where it came from.
 *
 * `overlay/floating.ts` answers *where a surface goes*. This answers *what happens to everything
 * else while it is there*, and the two are deliberately separate files: a docked task pane needs
 * none of this, a popover needs only the focus return, and a modal dialog needs all of it.
 *
 * ## `inert` and `aria-hidden` are two mechanisms, and one without the other is a defect
 *
 * MJXOFF-188 names the trap: *"`inert` and `aria-hidden` on the background are different
 * mechanisms, and getting one without the other leaves a screen reader wandering out of the
 * dialog."* They are not synonyms and neither implies the other in every engine:
 *
 * * `inert` removes an element from the **tab order** and from **hit testing**, and modern engines
 *   also drop it from the accessibility tree — but `inert` is a comparatively recent addition and
 *   an engine that has the attribute may still expose the subtree to an assistive technology
 *   walking the DOM rather than the focus order.
 * * `aria-hidden="true"` removes it from the **accessibility tree** and does nothing at all to the
 *   tab order, which is why an `aria-hidden` element containing a focusable one is itself an axe
 *   violation.
 *
 * So `holdBackgroundInert` writes **both**, and `tests/browser/surfaces.spec.ts` asserts both
 * independently: real `Tab` presses never land outside the surface, *and* nothing outside it has a
 * reachable accessible name.
 *
 * ## The walk is the flat tree, upward, marking siblings
 *
 * The background is not "every element that is not the dialog" — the dialog's own ancestors must
 * stay reachable, or marking them would hide the dialog with them. It is *every flat-tree sibling
 * of every flat-tree ancestor*, which is the standard construction and the only one that leaves
 * exactly the surface reachable.
 *
 * ⚠ **A `parentNode` walk finds the wrong ancestors.** The surfaces in this catalogue are slotted
 * into `<mjx-resizable-container>`'s `.frame`, which is inside a shadow root, so the chain from a
 * dialog to `<html>` crosses two shadow boundaries. `flatTreeParent` — exported from
 * `floating.ts` for exactly this, rather than copied here — is what makes the walk reach the
 * document at all.
 *
 * ## Restoring is exact, or it is not restoring
 *
 * Every attribute this file writes is recorded with **what was there before**, including the case
 * where an element was *already* inert because an outer surface had marked it. Releasing an inner
 * modal must not un-hide the background an outer one is still holding, and the way to be sure of
 * that is to put back what was found rather than to remove what was written.
 *
 * ## Node-importable? No — and deliberately
 *
 * This file is DOM. The *decisions* it implements live in `src/surfaces/surface-model.ts`, which is
 * Node-importable, and this file holds only the mechanics. No custom element is defined here, so a
 * Playwright spec may still import the constants.
 */

import { flatTreeParent } from './floating.ts';
import { focusableSelector } from '../foundations/focus.ts';

/**
 * Elements a hold walks straight past.
 *
 * `<head>` is a flat-tree sibling of `<body>`, so a walk that ran to the document would mark it
 * inert and `aria-hidden` — which changes nothing a person can perceive and makes the `held` count
 * a number nobody can reason about. Anything here draws nothing and focuses nothing, so leaving it
 * alone costs no guarantee.
 */
const unpaintedElements = new Set([
  'head',
  'script',
  'style',
  'link',
  'meta',
  'title',
  'template',
  'base',
  'noscript',
]);

/** The attributes a background hold writes. Named once so a gate cannot mistype one. */
export const backgroundHoldAttributes = { inert: 'inert', hidden: 'aria-hidden' } as const;

/** One element, and what it looked like before a hold marked it. */
interface HeldElement {
  readonly element: HTMLElement;
  readonly hadInert: boolean;
  readonly ariaHidden: string | null;
}

/** A hold on the background. Calling `release` puts every attribute back exactly as it was. */
export interface BackgroundHold {
  /** How many elements were marked. **Zero is a failure, not a quiet success** — see below. */
  readonly held: number;
  release(): void;
}

/**
 * Make everything outside `surface` unreachable, and return the undo.
 *
 * ⚠ **`held` is reported, and every caller's gate asserts it is greater than zero.** U07's second
 * defect is the reason: *"a helper whose failure mode is 'found nothing' makes every ceiling
 * assertion pass."* A walk that landed on the wrong root, or one that ran before the surface was
 * connected, would mark nothing at all — and *"nothing outside the dialog is tabbable"* is
 * trivially satisfied by a hold that marked nothing, on a page where everything still is.
 */
export function holdBackgroundInert(surface: Element): BackgroundHold {
  const held: HeldElement[] = [];
  let node: Node | null = surface;
  let parent: Node | null = flatTreeParent(node);

  while (parent !== null) {
    for (const child of flatTreeChildren(parent)) {
      if (child === node) continue;
      if (!(child instanceof HTMLElement)) continue;
      if (unpaintedElements.has(child.localName)) continue;
      // A `<slot>` on the path is not a box of its own; marking it would hide the assigned nodes,
      // which include the surface. Its *siblings* are marked by this same loop.
      if (child.localName === 'slot' && child.contains(surface)) continue;
      held.push({
        element: child,
        hadInert: child.hasAttribute(backgroundHoldAttributes.inert),
        ariaHidden: child.getAttribute(backgroundHoldAttributes.hidden),
      });
      child.setAttribute(backgroundHoldAttributes.inert, '');
      child.setAttribute(backgroundHoldAttributes.hidden, 'true');
    }
    node = parent;
    parent = flatTreeParent(node);
  }

  let released = false;
  return {
    held: held.length,
    release(): void {
      if (released) return;
      released = true;
      // Backwards, so an element marked twice by two nested holds is restored innermost-first and
      // ends up carrying the outer hold's mark rather than none.
      for (let index = held.length - 1; index >= 0; index -= 1) {
        const entry = held[index];
        if (entry === undefined) continue;
        if (!entry.hadInert) entry.element.removeAttribute(backgroundHoldAttributes.inert);
        if (entry.ariaHidden === null) {
          entry.element.removeAttribute(backgroundHoldAttributes.hidden);
        } else {
          entry.element.setAttribute(backgroundHoldAttributes.hidden, entry.ariaHidden);
        }
      }
    },
  };
}

/**
 * The flat-tree children of a node: a shadow host's shadow root's children, and a `<slot>`'s
 * assigned nodes.
 *
 * The mirror of `flatTreeParent`, and it has to be, or a hold would mark a shadow host's *light*
 * children — which are the nodes the surface was slotted from — instead of the boxes actually on
 * screen beside it.
 */
function flatTreeChildren(node: Node): Node[] {
  if (node instanceof Element && node.shadowRoot !== null) {
    return [...node.shadowRoot.childNodes];
  }
  if (node instanceof HTMLSlotElement) {
    const assigned = node.assignedNodes({ flatten: true });
    if (assigned.length > 0) return assigned;
  }
  return [...node.childNodes];
}

/**
 * Every tab stop inside a root, in tab order, descending through shadow roots and slots.
 *
 * ⚠ **This is the helper U07 warned about, written the way that warning asks for.** Its first
 * version broke its walk on a landing at `<body>` and reported *zero* stops for a pane that has
 * two, *"which read as a passing filter rather than as a broken helper."* Two things follow:
 *
 * * it never swallows a failure — a root with no stops returns an empty array and every caller is
 *   required to say whether that is expected;
 * * the browser gate does **not** trust it. `tests/browser/surfaces.spec.ts` counts stops by
 *   pressing real `Tab` keys, and compares the two counts. A walk and a keyboard agreeing is
 *   evidence; a walk agreeing with itself is not.
 */
export function tabStopsWithin(root: ParentNode): HTMLElement[] {
  const found: HTMLElement[] = [];
  const visit = (node: Node): void => {
    for (const child of flatTreeChildren(node)) {
      if (!(child instanceof HTMLElement)) continue;
      if (child.hasAttribute('inert') || child.hasAttribute('disabled')) continue;
      if (child.getAttribute('aria-hidden') === 'true') continue;
      if (child.matches(focusableSelector) && isTabbable(child)) found.push(child);
      visit(child);
    }
  };
  visit(root as unknown as Node);
  return found;
}

/** Whether an element is in the *sequential* order rather than merely focusable. */
function isTabbable(element: HTMLElement): boolean {
  if (element.tabIndex < 0) return false;
  // `.mjx-focus-ring` is the foundations' opt-in for a box that is not itself focusable; it is in
  // `focusableSelector` so the ring reaches it, and it is not a tab stop unless it also carries a
  // tabindex — which the branch above has already decided.
  if (element.classList.contains('mjx-focus-ring') && !element.hasAttribute('tabindex')) {
    return false;
  }
  return true;
}

/**
 * Wrap `Tab` inside a container. Returns `true` when the event was handled.
 *
 * A trap is only ever installed where U05's rule says one belongs — **many tab stops** — and the
 * reason it must be written at all, rather than left to `inert`, is that `inert` on the background
 * stops focus reaching the *page* and does nothing about focus reaching the **browser's own
 * chrome**. A person who Tabs past the last control of a modal lands in the address bar, and the
 * way back is not discoverable. Wrapping is what closes that.
 */
export function wrapTab(event: KeyboardEvent, container: ParentNode): boolean {
  if (event.key !== 'Tab') return false;
  const stops = tabStopsWithin(container);
  if (stops.length === 0) return false;
  const first = stops[0];
  const last = stops[stops.length - 1];
  if (first === undefined || last === undefined) return false;
  const active = deepActiveElement(container);
  if (event.shiftKey && (active === first || active === null)) {
    event.preventDefault();
    last.focus();
    return true;
  }
  if (!event.shiftKey && active === last) {
    event.preventDefault();
    first.focus();
    return true;
  }
  return false;
}

/**
 * The focused element, followed down through every shadow root it is hiding in.
 *
 * `document.activeElement` is the **host** while focus is inside a shadow root — U06's third
 * finding — so a `contains` against it is false for every control a component owns.
 */
export function deepActiveElement(scope: ParentNode): Element | null {
  // ⚠ **From the document down, never from the scope's own root.** The first version of this
  // started at `scope.getRootNode().activeElement`, which is `null` whenever focus is on a node
  // that was *slotted* into that root — and every control inside a dialog is slotted. `wrapTab`
  // then saw no active element, never recognised the last stop, and a modal's Tab walked straight
  // out of it while every other assertion in the suite stayed green. The browser gate caught it by
  // pressing Tab twice as many times as there were stops and counting distinct landings.
  const owner = scope instanceof Document ? scope : ((scope as Node).ownerDocument ?? document);
  let element: Element | null = owner.activeElement;
  while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
  return element;
}

/**
 * Put focus back on the element a surface was opened from.
 *
 * ⚠ **`preventScroll`, and it is not a nicety.** `<mjx-menu>` records the same lesson from the
 * other direction: the browser's scroll-into-view walks every ancestor scroll container, so
 * restoring focus to a button inside a scrolled task pane scrolls that pane — and a person who
 * closed a dialog finds the pane behind it in a different place than they left it.
 *
 * Returns whether the invoker actually took focus. A caller that ignored this could report a
 * successful return for an invoker that had been removed from the document while the surface was
 * open, which is the commonest way focus quietly ends up on `<body>`.
 */
export function returnFocusTo(invoker: HTMLElement | undefined): boolean {
  if (invoker === undefined || !invoker.isConnected) return false;
  invoker.focus({ preventScroll: true });
  const landed = deepActiveElement(invoker.ownerDocument);
  return landed === invoker || invoker.contains(landed);
}

/**
 * Move focus into a surface: its first tab stop, or the surface itself.
 *
 * A surface with no tab stop at all — a message with one line of text and no buttons — still has
 * to receive focus, or a screen reader stays where it was and reads nothing. `tabindex="-1"` on
 * the box is what makes that possible, and it is the component's job to have set one.
 */
export function focusInto(surface: HTMLElement): HTMLElement | undefined {
  const stops = tabStopsWithin(surface);
  const target = stops[0] ?? surface;
  target.focus({ preventScroll: true });
  return stops[0];
}
