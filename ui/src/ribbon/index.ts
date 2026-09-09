/**
 * The ribbon's four elements, and the one call that registers them.
 *
 * `group` (1,193), `tab` (166) and `tabSet` (52) are the structure the 10,518 controls MJXOFF-182
 * built actually live in — and the reason this child is the hardest responsive problem in the
 * catalogue rather than four more custom elements. Word's `TabHome` alone is 152 controls across
 * thirteen groups in the committed census, and all of it has to degrade to a phone without losing
 * a command.
 *
 * Menus are U05, galleries are U06, and deciding which real command goes in which group is loop 2.
 * What this child ships is the mechanism and one realistic worst case.
 */

import { defineContextualTabSet } from './contextual-tab-set.ts';
import { defineRibbon } from './ribbon.ts';
import { defineRibbonGroup } from './ribbon-group.ts';
import { defineRibbonTab } from './ribbon-tab.ts';

export { MjxContextualTabSet, defineContextualTabSet } from './contextual-tab-set.ts';
export { MjxRibbon, defineRibbon } from './ribbon.ts';
export { MjxRibbonGroup, defineRibbonGroup } from './ribbon-group.ts';
export { ribbonCss, ribbonGroupCss } from './ribbon-model.ts';
export { MjxRibbonTab, defineRibbonTab } from './ribbon-tab.ts';

/** Register all four. Idempotent, and what `.storybook/preview.ts` calls. */
export function defineRibbonElements(): void {
  defineRibbonGroup();
  defineRibbonTab();
  defineContextualTabSet();
  defineRibbon();
}
