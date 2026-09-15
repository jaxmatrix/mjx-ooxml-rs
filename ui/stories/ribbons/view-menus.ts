/**
 * **The menus the View tab opens**, written once and rendered by both hosts.
 *
 * **Word's and PowerPoint's View tabs**, each authored under the one-tab-one-application rule, so Excel's
 * View menus are not here yet and `viewMenus` renders nothing for it. The pattern is
 * `stories/ribbons/review-menus.ts`'s, for its reasons. A binding lives in its host. The menu it opens is
 * written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `viewMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below spells its
 * command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## One menu per application, because the tab opens almost nothing
 *
 * A View tab changes how a document is looked at, and nearly every command on it is a toggle, a checkbox or
 * a single press. **Switch Windows is the only command with a popup**, in Word and in PowerPoint alike;
 * Zoom opens a dialog, which is not a menu, and PowerPoint's Show launcher opens Grid and Guides.
 *
 * ## Complete, not shallow
 *
 * The rule since the rejected Transitions gallery: a popup carries **every entry Office's popup has**.
 * Switch Windows lists **every open window of its application**, numbered, with the active one checked, and
 * nothing else. The catalogue has one document open per application, so each lists one window. The list's
 * shape is identical in both applications, so it is written once (`switchWindowsEntries`) and handed the
 * document's name.
 *
 * ⚠ `GUESS:` **the windows' names.** Office writes the document's name, and a document name is the
 * document's data rather than Office's vocabulary. The names here are the documents this catalogue already
 * calls its own in Insert's *Recent Items*: *Method notes* for Word and *Where the time went* for
 * PowerPoint, rather than somebody's real file.
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

/**
 * Switch Windows: every open window, numbered as Office numbers them, the active one checked. The same list
 * in Word and PowerPoint; only the document it names differs.
 */
function switchWindowsEntries(documentName: string): TemplateResult[] {
  return [choice(`1 ${documentName}`, true)];
}

// ── Word's View ──────────────────────────────────────────────────────────────

/** One menu. Ruler, Gridlines and Navigation Pane are checkboxes the hosts bind. */
function wordViewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.view.window.switch-windows', 'Switch Windows', ...switchWindowsEntries('Method notes'))}
  `;
}

// ── PowerPoint's View ────────────────────────────────────────────────────────

/** One menu. Ruler, Gridlines and Guides are checkboxes the hosts bind. */
function powerpointViewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.view.window.switch-windows', 'Switch Windows', ...switchWindowsEntries('Where the time went'))}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/**
 * Word and PowerPoint, until Excel's View tab has its own unit. A missing application is not a menu set
 * found to be empty; it is one nobody has written yet.
 */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordViewMenus,
  powerpoint: powerpointViewMenus,
};

/**
 * Every menu one application's View tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `reviewMenus` is.
 */
export function viewMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return menusByApplication[application]?.(host) ?? nothing;
}
