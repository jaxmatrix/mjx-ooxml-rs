/**
 * `<mjx-dialog-launcher>` — the corner mark that opens a ribbon group's full dialog.
 *
 * ```html
 * <mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>
 * ```
 *
 * 130 of Office's published controls, and the one most easily got wrong, because it is visually
 * trivial and therefore invites being built as a glyph with a click handler. MJXOFF-182:
 *
 * > Trivial visually, and it needs a **real accessible name** because *"dialog launcher"* tells a
 * > screen-reader user nothing about which dialog.
 *
 * ## The name is required, and nothing is invented when it is missing
 *
 * `label` is the accessible name, drawn off-screen because the control is a glyph. There is no
 * default, no fallback to the group's heading, and no name derived from the icon: every one of
 * those produces a *plausible* announcement that is wrong, and a wrong name is worse than a
 * missing one because a screen-reader user acts on it. A launcher with no label renders a nameless
 * button, reports it on the console, and **fails axe's `button-name` rule** — which the catalogue
 * proves is live in `Gates/Control Naming`, on a deliberately nameless launcher, in the same way
 * MJXOFF-180 proved the contrast rule can reject.
 *
 * ## The glyph, and the target it sits in
 *
 * Fluent's `arrow-down-right` at 16, which is the mark Office draws. It is one rung below the icon
 * size a command button uses, because this is a *mark* rather than a command's icon.
 *
 * `GUESS:` **the target is larger than Office's.** Office's launcher is a few pixels of arrow in
 * the corner of a group; this one carries `.mjx-hit-target`, so it is at least WCAG 2.2's 24 CSS
 * pixels in both density modes and 40 in comfortable. That is a deliberate divergence rather than
 * an approximation of Office, and it is asserted in `tests/browser/controls.spec.ts` rather than
 * hoped for: a control that cannot be hit is not a control, and this project's rule is that the
 * option which cannot break someone already working is the one to take.
 *
 * ## `aria-haspopup="dialog"`
 *
 * Announced as *"opens dialog"*, which is the fact a person needs before they press it. The
 * component does **not** claim `aria-expanded`: a dialog is not a disclosure, and U06 owns the
 * dialog itself.
 */

import { MjxButton } from './button.ts';
import { defineIcon } from '../icons/icon.ts';
import type { IconSize } from '../icons/manifest.ts';
import type { ControlSize } from './control-states.ts';

/** The Fluent glyph Office draws in the corner of a ribbon group. */
export const dialogLauncherIcon = 'arrow-down-right';

/** The drawing it is rendered at — one rung below a command's icon, because it is a mark. */
export const dialogLauncherIconSize: IconSize = 16;

export class MjxDialogLauncher extends MjxButton {
  protected override fixedIcon(): string {
    return dialogLauncherIcon;
  }

  protected override fixedSize(): ControlSize {
    // Icon-only, always: the label is the accessible name and is never drawn.
    return 'icon';
  }

  protected override iconRenderSize(_size: IconSize): IconSize {
    return dialogLauncherIconSize;
  }

  protected override rendered(button: HTMLButtonElement): void {
    button.setAttribute('aria-haspopup', 'dialog');
    button.dataset['launcher'] = '';
  }
}

/** Register the element. Idempotent. */
export function defineDialogLauncher(): void {
  defineIcon();
  if (customElements.get('mjx-dialog-launcher') === undefined) {
    customElements.define('mjx-dialog-launcher', MjxDialogLauncher);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-dialog-launcher': MjxDialogLauncher;
  }
}
