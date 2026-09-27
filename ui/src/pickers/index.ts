/**
 * MJXOFF-187's two pickers, and the one call that registers them.
 *
 * They are together because they are the same shape of problem: a field whose popup is not a list
 * of strings, and whose whole value is in what the popup *says* about each entry. A colour picker
 * that painted a selection ring in a fixed colour would be invisible on one swatch in the palette;
 * a font picker that listed families without their substitution status would let a person choose a
 * face and silently get another. Both failures look perfect in a screenshot.
 *
 * Everything here reads one model, `picker-model.ts`, and both controls are `<mjx-combo-box>`
 * underneath — the field, the ARIA combobox contract, the single tab stop, the dismissal model and
 * the placement are U07's and are not re-implemented. What each one adds is exactly its own
 * difference: a **grid** instead of a list, and a **decorated row** instead of a plain one.
 */

import { defineColorPicker } from './color-picker.ts';
import { defineFontPicker } from './font-picker.ts';

export { MjxColorPicker, colorPickerSheet, colorSectionNames, defineColorPicker } from './color-picker.ts';
export {
  MjxFontPicker,
  defineFontPicker,
  fontPickerSheet,
  substitutionMessageId,
} from './font-picker.ts';
export { SwatchSurface, isSwatchDescriptor } from './swatch-surface.ts';
export type { SwatchDescriptor, SwatchSurfaceHost } from './swatch-surface.ts';
export * from './picker-model.ts';

/** Register both. Idempotent, and what `.storybook/preview.ts` calls. */
export function definePickers(): void {
  defineColorPicker();
  defineFontPicker();
}
