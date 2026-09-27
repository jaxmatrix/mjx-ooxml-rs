/**
 * The foundations stylesheet, and the one function that installs it.
 *
 * Fourteen component children follow this one, and every one of them needs typography, elevation,
 * focus, motion and density. A design system in which each component pastes its own copy of those
 * rules is a design system with fourteen slightly different focus rings — the failure MJXOFF-181
 * exists to prevent. So the rules are composed once here, and a component asks for them.
 *
 * ## Why an installer rather than an import
 *
 * The components are **Shadow DOM custom elements**, and a shadow root does not inherit the
 * document's stylesheets. It does inherit *inherited properties* — `color`, `font-family`, and
 * every custom property — which is why `tokens.css` works across the boundary without help and why
 * a component may write `var(--theme-surface)` inside its shadow root with nothing installed. What
 * does **not** cross is a rule: `.mjx-type-dense` in the document says nothing about a
 * `.mjx-type-dense` inside a shadow root, and neither does the `:focus-visible` rule that gives the
 * platform its single focus ring.
 *
 * `installFoundations(root)` is therefore called with the *shadow root* by a component and with the
 * *document* by the shell. One `CSSStyleSheet` is constructed once and adopted by every root, so
 * fifteen components share one parsed stylesheet rather than fifteen copies of the same text.
 *
 * `adoptedStyleSheets` has been in every current browser for years; the `<style>` fallback below
 * exists for the one environment this catalogue genuinely meets that may lack it — a test runner's
 * DOM emulation — and not as a hedge.
 */

import { typographyCss } from './typography.ts';
import { surfacesCss } from './surfaces.ts';
import { focusCss } from './focus.ts';
import { motionCss } from './motion.ts';
import { densityCss } from './density.ts';

/**
 * The whole sheet, in cascade order.
 *
 * Density before focus is deliberate: focus's ring width is a `calc()` over `--spacing`, not over
 * a density property, so the order does not matter for correctness — but reading it as
 * *what things are* (typography, surfaces), then *how big* (density), then *how they respond*
 * (focus, motion) is the order a later child will expect to find a rule in.
 */
export const foundationsCss = [typographyCss, surfacesCss, densityCss, focusCss, motionCss].join(
  '\n',
);

/** The single parsed copy, built on first use. */
let sheet: CSSStyleSheet | undefined;

/** Roots that already carry it, so a component constructed a thousand times adopts it once. */
const installed = new WeakSet<Document | ShadowRoot>();

/**
 * Give a document or a shadow root the foundations rules. Idempotent, and cheap after the first
 * call.
 */
export function installFoundations(root: Document | ShadowRoot): void {
  if (installed.has(root)) return;
  installed.add(root);

  const supportsAdoption =
    typeof CSSStyleSheet !== 'undefined' &&
    'replaceSync' in CSSStyleSheet.prototype &&
    Array.isArray(root.adoptedStyleSheets);

  if (supportsAdoption) {
    if (sheet === undefined) {
      sheet = new CSSStyleSheet();
      sheet.replaceSync(foundationsCss);
    }
    if (!root.adoptedStyleSheets.includes(sheet)) {
      root.adoptedStyleSheets = [...root.adoptedStyleSheets, sheet];
    }
    return;
  }

  const owner = root instanceof Document ? root : (root.ownerDocument as Document);
  const element = owner.createElement('style');
  element.dataset['mjxFoundations'] = '';
  element.textContent = foundationsCss;
  const target: ParentNode = root instanceof Document ? root.head : root;
  target.prepend(element);
}
