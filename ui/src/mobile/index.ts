/**
 * MJXOFF-194's mobile forms.
 *
 * ```ts
 * import { defineMobile, mobileDocumentCss } from './src/mobile/index.ts';
 * defineMobile();
 * ```
 *
 * ⚠ **`mobileDocumentCss` must reach the document**, and it is not optional decoration. It carries
 * the four **registered** safe-area properties, and an unregistered custom property reports its
 * substituted text to `getComputedStyle` rather than a length — so a padding built on one resolves
 * to nothing and a notched phone silently loses its inset. `.storybook/preview.ts` installs it
 * beside the surfaces', the formula bar's and the annotation family's, for the same class of
 * reason.
 */

import { defineCommandBar } from './command-bar.ts';
import { defineContextualActionBar } from './contextual-action-bar.ts';

export { MjxCommandBar, defineCommandBar } from './command-bar.ts';
export { MjxContextualActionBar, defineContextualActionBar } from './contextual-action-bar.ts';
export { MjxMobileBar } from './mobile-bar-element.ts';
export {
  commandBarCss,
  contextualActionBarCss,
  mobileDocumentCss,
  mobileFormFactorAttribute,
  mobileTypeRoles,
  overflowPanelMotionClass,
  sheetCompletionCss,
} from './mobile-sheets.ts';
export * from './mobile-model.ts';
export * from './gesture-map.ts';
export * from './sheet-detents.ts';
export * from './touch-audit.ts';

/** Register both bars. Idempotent. */
export function defineMobile(): void {
  defineCommandBar();
  defineContextualActionBar();
}
