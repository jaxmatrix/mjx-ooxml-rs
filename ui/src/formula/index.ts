/**
 * MJXOFF-192's Excel chrome — the formula bar, the name box, and the one call that registers them.
 *
 * **The most-used control in Excel appears in no ribbon census**, because it is not a ribbon
 * control. It is also the most textually complex thing in this catalogue: a formula is a small
 * language, and the bar is its editor.
 *
 * Two things are worth knowing before reading any of it, and both are in `formula-model.ts`'s own
 * note: **every hard behaviour here is caret-relative** — which is why the model is pure functions
 * and why the gate is a caret-position table rather than a story — and **`mjx-sml` has no formula
 * tokeniser by design**, so this one duplicates nothing, while `mjx-sml`'s *address* grammar is
 * authoritative and is mirrored rather than reinvented.
 */

import { defineFormulaBar } from './formula-bar.ts';
import { defineNameBox } from './name-box.ts';

export {
  MjxFormulaBar,
  affordanceIcons,
  affordanceLabels,
  completionVisibleRows,
  defineFormulaBar,
  handleLabel,
} from './formula-bar.ts';
export { MjxNameBox, defineNameBox, definedNameCategories } from './name-box.ts';
export {
  alignedTextProperties,
  formulaBarCss,
  formulaDocumentCss,
  formulaMotionClass,
  formulaSchemeCss,
  formulaTypeRoles,
  nameBoxCss,
} from './formula-sheets.ts';
export * from './formula-model.ts';

/** Register both. Idempotent, and what `.storybook/preview.ts` calls. */
export function defineFormulaChrome(): void {
  defineNameBox();
  defineFormulaBar();
}
