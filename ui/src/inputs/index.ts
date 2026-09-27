/**
 * MJXOFF-186's seven inputs, and the one call that registers them.
 *
 * A ribbon is commands; a task pane, a dialog and an inspector are **fields**, and this is what
 * they are made of. The seven were chosen because between them they cover every shape Office's
 * property surfaces use: a name for a field, a three-state box, a fixed choice, a choice you may
 * also type, a number with a unit, a continuous value, and a small exclusive set shown all at once.
 *
 * Everything here reads one model, `input-model.ts`, and the two things worth knowing before
 * reading any of it are in that file's note: **a field never changes fill** (because a placeholder
 * on `--theme-border-subtle` is 4.32 : 1), and **every control holds exactly one tab stop** (so
 * `Tab` may mean leave, which is what makes an open list a disclosure rather than a trap).
 */

import { defineCheckbox } from './checkbox.ts';
import { defineComboBox } from './combo-box.ts';
import { defineDescriptors } from './descriptors.ts';
import { defineDropdown } from './dropdown.ts';
import { defineLabel } from './label.ts';
import { defineMeasureInput } from './measure-input.ts';
import { defineSegmentedControl } from './segmented-control.ts';
import { defineSlider } from './slider.ts';

export { MjxLabel, defineLabel, labelTargetAttribute, requiredAnnouncement, requiredMark } from './label.ts';
export { MjxCheckbox, defineCheckbox, checkboxCssSheet } from './checkbox.ts';
export { MjxDropdown, defineDropdown, dropdownCss } from './dropdown.ts';
export { MjxComboBox, defineComboBox, comboBoxCss, defaultFilterMode } from './combo-box.ts';
export {
  MjxMeasureInput,
  defaultMeasureStep,
  defineMeasureInput,
  invalidIcon,
  measureInputCss,
} from './measure-input.ts';
export { MjxSlider, defineSlider, sliderDefaults, sliderSheet } from './slider.ts';
export {
  MjxSegmentedControl,
  defineSegmentedControl,
  segmentIconSize,
  segmentedControlCss,
} from './segmented-control.ts';
export { MjxOption, MjxSegment, defineDescriptors, optionDescriptorsIn, optionsChangedEvent } from './descriptors.ts';
export { MjxListField, listDisclosureIcon, listFieldCss } from './list-field.ts';
export { ListSurface, listAlign, listSide } from './list-surface.ts';
export type { ListSurfaceHost } from './list-surface.ts';
export * from './input-model.ts';
export * from './measure.ts';

/**
 * The one rule that must be on the **document**.
 *
 * `<mjx-option>` and `<mjx-segment>` are data written as markup. Between the moment the parser
 * creates them and the moment they upgrade they are unknown elements with attributes, and an
 * unknown element is `display: inline` — so without this a page would flash a row of option
 * labels into the layout before any component existed. `galleryDocumentCss` makes the same
 * arrangement for the same reason.
 */
export const inputDocumentCss = `
mjx-option,
mjx-segment {
  display: none !important;
}
`;

/** Register all nine. Idempotent, and what `.storybook/preview.ts` calls. */
export function defineInputs(): void {
  defineDescriptors();
  defineLabel();
  defineCheckbox();
  defineDropdown();
  defineComboBox();
  defineMeasureInput();
  defineSlider();
  defineSegmentedControl();
}
