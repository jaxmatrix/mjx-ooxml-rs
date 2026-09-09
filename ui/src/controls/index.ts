/**
 * The four ribbon archetypes of MJXOFF-182, and the one call that registers them.
 *
 * `button`, `toggleButton`, `splitButton` and `button (dialogBoxLauncher)` are **10,518 of the
 * 15,346 controls Office publishes** — 8,687, 1,261, 440 and 130 respectively. That is why they
 * come first, and why they are built against one state table rather than four: a design system
 * whose most-used control disagrees with its second-most-used about what *pressed* looks like has
 * already lost the argument it exists to settle.
 *
 * Ribbon structure is U04, menus are U05, and wiring any of these to a command is loop 2. A split
 * button here *asks* for a menu; it does not know what one is.
 */

import { defineButton } from './button.ts';
import { defineDialogLauncher } from './dialog-launcher.ts';
import { defineSplitButton } from './split-button.ts';
import { defineToggleButton } from './toggle-button.ts';

export { MjxButton, buttonAttributes, buttonCss, defineButton } from './button.ts';
export { MjxToggleButton, defineToggleButton } from './toggle-button.ts';
export { MjxSplitButton, defineSplitButton, splitButtonCss, splitButtonSheet } from './split-button.ts';
export {
  MjxDialogLauncher,
  defineDialogLauncher,
  dialogLauncherIcon,
  dialogLauncherIconSize,
} from './dialog-launcher.ts';

/** Register all four. Idempotent, and what `.storybook/preview.ts` calls. */
export function defineControls(): void {
  defineButton();
  defineToggleButton();
  defineSplitButton();
  defineDialogLauncher();
}
