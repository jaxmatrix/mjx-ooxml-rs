/**
 * MJXOFF-184's menus, registered together.
 *
 * The same reasoning `src/controls/index.ts` and `src/ribbon/index.ts` give: a component that is
 * only defined by the story that happened to import it is a component whose absence looks like a
 * rendering bug in a *different* story. A menu is worse than most, because an unregistered
 * `<mjx-menu-item>` is an inert element with no role — and a menu of them is a menu that renders
 * and announces nothing.
 */

export { MjxMenu, defineMenu } from './menu.ts';
export { MjxMenuItem, defineMenuItem } from './menu-item.ts';
export { ariaKeyShortcuts } from './menu-model.ts';
export { MjxMenuSection, MjxMenuSeparator, defineMenuStructure } from './menu-structure.ts';
export { MjxContextMenu, defineContextMenu } from './context-menu.ts';

import { defineMenu } from './menu.ts';
import { defineMenuItem } from './menu-item.ts';
import { defineMenuStructure } from './menu-structure.ts';
import { defineContextMenu } from './context-menu.ts';

/** Register every menu element. Idempotent, because a story file and a test may both ask. */
export function defineMenus(): void {
  defineMenuItem();
  defineMenuStructure();
  defineMenu();
  defineContextMenu();
}
