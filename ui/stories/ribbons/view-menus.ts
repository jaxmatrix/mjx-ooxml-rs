/**
 * **The menus the View tab opens**, written once and rendered by both hosts.
 *
 * **Word's View tab, alone**, under the one-tab-one-application rule, so PowerPoint's and Excel's View
 * menus are not here yet and `viewMenus` renders nothing for either. The pattern is
 * `stories/ribbons/review-menus.ts`'s, for its reasons. A binding lives in its host. The menu it opens is
 * written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `viewMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below spells its
 * command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## One menu, because the tab opens almost nothing
 *
 * Word's View tab changes how a document is looked at, and nearly every command on it is a toggle, a
 * checkbox or a single press. **Switch Windows is the only command with a popup**; Zoom opens a dialog,
 * which is not a menu.
 *
 * ## Complete, not shallow
 *
 * The rule since the rejected Transitions gallery: a popup carries **every entry Office's popup has**.
 * Switch Windows lists **every open Word window**, numbered, with the active one checked, and nothing
 * else. The catalogue has one document open, so it lists one window.
 *
 * ⚠ `GUESS:` **the window's name.** Office writes the document's name, and a document name is the
 * document's data rather than Office's vocabulary. The name here is *Method notes*, the document this
 * catalogue already calls its own in Insert's *Recent Items*, rather than somebody's real file.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no window changes,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry of a set whose current member is checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

// ── Word's View ──────────────────────────────────────────────────────────────

/** Switch Windows: every open window, numbered as Office numbers them, the active one checked. */
function switchWindowsEntries(): TemplateResult[] {
  return [choice('1 Method notes', true)];
}

/** One menu. Ruler, Gridlines and Navigation Pane are checkboxes the hosts bind. */
function wordViewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.view.window.switch-windows', 'Switch Windows', ...switchWindowsEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/**
 * Word alone, until PowerPoint's and Excel's View tabs have their own unit. A missing application is not
 * a menu set found to be empty; it is one nobody has written yet.
 */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordViewMenus,
};

/**
 * Every menu one application's View tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `reviewMenus` is.
 */
export function viewMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return menusByApplication[application]?.(host) ?? nothing;
}
