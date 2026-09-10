/**
 * `<mjx-command-bar>` — **the phone's ribbon.**
 *
 * One scrollable row of the highest-priority commands, an overflow control, and contents that
 * change with the selection context. It is where MJXOFF-183's command-demotion rules stop being a
 * table in a document and become something a person presses — and where they are proved to have
 * produced a usable answer rather than an arbitrary one.
 *
 * ```html
 * <mjx-command-bar label="Home"></mjx-command-bar>
 * <script>
 *   document.querySelector('mjx-command-bar').commands = [
 *     { id: 'bold', label: 'Bold', icon: 'text-bold', group: 'Font',
 *       priority: 'primary', essential: true, hasPopup: false },
 *   ];
 * </script>
 * ```
 *
 * ## What it does not do
 *
 * It does not decide its own order. `commandBarOrder` does, out of U04's ladder, and the browser
 * gate compares the rail's DOM order against that function rather than against a list written in a
 * test. A bar that sorted its own commands would be the second priority scheme the ticket forbids.
 */

import { MjxMobileBar } from './mobile-bar-element.ts';
import { commandBarCss } from './mobile-sheets.ts';
import { mobileTags, type MobileCommand } from './mobile-model.ts';

export class MjxCommandBar extends MjxMobileBar {
  #commands: readonly MobileCommand[] = [];

  protected styles(): string {
    return commandBarCss;
  }

  protected barCommands(): readonly MobileCommand[] {
    return this.#commands;
  }

  protected defaultLabel(): string {
    return 'Commands';
  }

  /**
   * The commands the bar holds, in the author's own order.
   *
   * A **property** rather than markup, for the reason MJXOFF-191 gives about all four navigators:
   * a command carries a priority, an essential flag and a popup flag, and expressing four fields
   * per command as attributes on a light-DOM element would make the shell write a descriptor
   * element it never reads back.
   */
  get commands(): readonly MobileCommand[] {
    return this.#commands;
  }

  set commands(value: readonly MobileCommand[]) {
    this.#commands = [...value];
    this.render();
  }
}

/** Register the element. Idempotent. */
export function defineCommandBar(): void {
  if (customElements.get(mobileTags.commandBar) === undefined) {
    customElements.define(mobileTags.commandBar, MjxCommandBar);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-command-bar': MjxCommandBar;
  }
}
