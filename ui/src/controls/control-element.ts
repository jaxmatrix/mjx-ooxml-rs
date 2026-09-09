/**
 * The machinery the four archetypes share — and the one ordering decision that makes the state
 * table work at all.
 *
 * ## Why a component's own sheet is *adopted*, and adopted second
 *
 * A shadow root's final style sheet list is its own `<style>` elements **followed by** its
 * `adoptedStyleSheets`. So a component that put its rules in a `<style>` element would find the
 * foundations sheet — adopted by `installFoundations` — winning every tie at equal specificity,
 * and `.mjx-type-control`'s `font-weight: var(--font-weight-medium)` would silently defeat the
 * `on` state's bold.
 *
 * ⚠ **And the pairwise gate would not notice.** MJXOFF-182 reversed this order on purpose to find
 * out: the four held states lose their bold, every fill still differs, and
 * `every pair of states differs` **stays green**. What fails is the *correspondence* assertion —
 * `what it computed is what the model says`, on `font-weight is 500, model says bold` — and the
 * order assertion in `the foundations reach every control's shadow root`. That is worth knowing
 * before trusting a distinctness gate: it proves no two states are the *same*, not that any of
 * them is *right*, and a design system needs both claims from different instruments.
 *
 * `installControlStyles` therefore installs the foundations and then adopts the component's sheet
 * **after** them, so the order is stated in one function rather than inferred at four call sites.
 * A component wears the foundations and then dresses over them, which is the relationship the two
 * are supposed to have.
 *
 * One `CSSStyleSheet` is constructed per distinct stylesheet text and shared by every instance, so
 * a ribbon with two hundred buttons parses four sheets rather than two hundred `<style>` elements.
 *
 * ## The `<style>` fallback
 *
 * Appended rather than prepended, for the same ordering reason, and it exists for the one
 * environment this catalogue genuinely meets that may lack constructable stylesheets — a test
 * runner's DOM emulation — and not as a hedge.
 */

import { installFoundations } from '../foundations/stylesheet.ts';
import {
  controlEvents,
  isForcibleState,
  type ForcibleState,
} from './control-states.ts';

/** One parsed sheet per stylesheet text, shared across every instance of every component. */
const sheets = new Map<string, CSSStyleSheet>();

/**
 * Give a control's shadow root the foundations **and then** its own rules, in that order.
 *
 * Every one of the four archetypes calls exactly this. A component that called
 * `installFoundations` itself and adopted a sheet by hand would be a fifth opinion about cascade
 * order, which is how a design system acquires four slightly different buttons.
 */
export function installControlStyles(root: ShadowRoot, css: string): void {
  installFoundations(root);

  const supportsAdoption =
    typeof CSSStyleSheet !== 'undefined' &&
    'replaceSync' in CSSStyleSheet.prototype &&
    Array.isArray(root.adoptedStyleSheets);

  if (supportsAdoption) {
    let sheet = sheets.get(css);
    if (sheet === undefined) {
      sheet = new CSSStyleSheet();
      sheet.replaceSync(css);
      sheets.set(css, sheet);
    }
    if (!root.adoptedStyleSheets.includes(sheet)) {
      root.adoptedStyleSheets = [...root.adoptedStyleSheets, sheet];
    }
    return;
  }

  const style = (root.ownerDocument ?? document).createElement('style');
  style.dataset['mjxControl'] = '';
  style.textContent = css;
  root.append(style);
}

/** The classes every inner control wears. Four of the five come from the foundations sheet. */
export const controlClassName =
  'control mjx-type-control mjx-hit-target mjx-motion-surface-settle';

/**
 * Fire one of the three control events from the host.
 *
 * `composed` so it crosses the shadow boundary, `bubbles` so a ribbon listens once at its root.
 * The host is the target rather than the inner button, because the inner button is an
 * implementation detail and a listener that had to know about it would be coupled to the shape of
 * a shadow tree it cannot see.
 */
export function emitControlEvent(host: HTMLElement, type: string, detail?: unknown): void {
  host.dispatchEvent(
    new CustomEvent(type, {
      bubbles: true,
      composed: true,
      ...(detail === undefined ? {} : { detail }),
    }),
  );
}

/** Whether a host declares itself unavailable-but-explained. */
export function isUnavailable(host: HTMLElement): boolean {
  return host.hasAttribute('unavailable');
}

/** Whether a host declares itself hard-disabled. */
export function isHardDisabled(host: HTMLElement): boolean {
  return host.hasAttribute('disabled');
}

/**
 * The state a story asked the control to pretend to be in, or `undefined`.
 *
 * Only `hover` and `active` are forcible, and both are purely presentational — a forced state
 * changes no ARIA, fires no event, and blocks nothing. See `control-states.ts` for why the
 * affordance exists at all and why it cannot become a mock of the real interaction.
 */
export function forcedState(host: HTMLElement): ForcibleState | undefined {
  const declared = host.getAttribute('force-state');
  return isForcibleState(declared) ? (declared as ForcibleState) : undefined;
}

/**
 * Apply availability to one inner button, and return whether it may be activated.
 *
 * The two kinds of unavailability are genuinely different and this is the only place that says so:
 *
 * * **`disabled`** sets the native attribute. The platform then removes it from the tab order and
 *   suppresses `click` for pointer *and* keyboard, which is a stronger guarantee than any handler
 *   this project could write — and axe stops asking it to meet text-contrast, which is what lets
 *   it dim.
 * * **`unavailable`** sets `aria-disabled`. The control stays focusable, stays announced, and
 *   **keeps its explanation**, because Office uses this state far more than a plain disabled one
 *   and a command a person cannot reach is a command whose reason they can never read. Activation
 *   is refused in JavaScript instead of by the platform, which is the price of staying reachable.
 */
export function applyAvailability(
  host: HTMLElement,
  button: HTMLButtonElement,
  explanationElement: HTMLElement,
): void {
  const disabled = isHardDisabled(host);
  const unavailable = !disabled && isUnavailable(host);
  const explanation = host.getAttribute('explanation') ?? '';

  button.disabled = disabled;

  if (unavailable) button.setAttribute('aria-disabled', 'true');
  else button.removeAttribute('aria-disabled');

  if (unavailable && explanation.trim() !== '') {
    explanationElement.textContent = explanation;
    button.setAttribute('aria-describedby', explanationElement.id);
    // The pointer's half of the same answer. A tooltip component is U06's; `title` is the
    // platform's own and it is what a mouse user gets until then.
    button.title = explanation;
  } else {
    explanationElement.textContent = '';
    button.removeAttribute('aria-describedby');
    button.removeAttribute('title');
  }
}

/** Whether an activation attempt on this host must be refused. */
export function refusesActivation(host: HTMLElement): boolean {
  return isHardDisabled(host) || isUnavailable(host);
}

/**
 * The visually-hidden element a control's explanation lives in.
 *
 * `id` is scoped to the shadow root, so every control may use the same one and `aria-describedby`
 * still resolves to the right element — an IDREF is resolved within its own tree.
 */
export function createExplanationElement(id: string): HTMLElement {
  const element = document.createElement('span');
  element.className = 'visually-hidden';
  element.id = id;
  return element;
}

/** The id above, named once. */
export const explanationElementId = 'explanation';

/** Re-exported so a component imports one module rather than two for the common case. */
export { controlEvents };
