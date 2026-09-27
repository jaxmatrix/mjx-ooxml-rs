/**
 * **The menus the View tab opens**, written once and rendered by both hosts.
 *
 * **Word's, PowerPoint's and Excel's View tabs**, each authored under the one-tab-one-application rule. The
 * pattern is
 * `stories/ribbons/review-menus.ts`'s, for its reasons. A binding lives in its host. The menu it opens is
 * written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `viewMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below spells its
 * command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## A menu or two per application, because the tab opens almost nothing
 *
 * A View tab changes how a document is looked at, and nearly every command on it is a toggle, a checkbox or
 * a single press. **Switch Windows is the only command with a popup** in Word and in PowerPoint; Zoom opens
 * a dialog, which is not a menu, and PowerPoint's Show launcher opens Grid and Guides. **Excel adds Freeze
 * Panes**, and its Sheet View dropdown is a field whose options are listed here (`excelSheetViews`) so both
 * hosts bind the same list.
 *
 * ## Complete, not shallow
 *
 * The rule since the rejected Transitions gallery: a popup carries **every entry Office's popup has**.
 * Switch Windows lists **every open window of its application**, numbered, with the active one checked, and
 * nothing else. The catalogue has one document open per application, so each lists one window. The list's
 * shape is identical in all three applications, so it is written once (`switchWindowsEntries`) and handed
 * the document's name. **Freeze Panes** carries Office's three entries, each with its glyph and Office's
 * one-line description; **Sheet View** lists Default, the only view a workbook with no saved sheet views has.
 *
 * ⚠ `GUESS:` **the windows' names.** Office writes the document's name, and a document name is the
 * document's data rather than Office's vocabulary. The names here are the documents this catalogue already
 * calls its own in Insert's *Recent Items*: *Method notes* for Word, *Where the time went* for PowerPoint
 * and *Findings* for Excel, rather than somebody's real file.
 *
 * ⚠ `GUESS:` **Freeze Panes' descriptions**, from memory of Microsoft 365, and that the first entry reads
 * *Freeze Panes* because the catalogue's sheet has no panes frozen; Office relabels it *Unfreeze Panes*
 * while they are.
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
 * in all three applications; only the document it names differs.
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

// ── Excel's View ─────────────────────────────────────────────────────────────

/** One entry that does one thing, drawn with its glyph and Office's one-line description. */
function described(label: string, icon: string, description: string): TemplateResult {
  return html`<mjx-menu-item label=${label} icon=${icon} description=${description}></mjx-menu-item>`;
}

/**
 * Freeze Panes: Office's three, in Office's order. See this file's header on the first entry's label and the
 * descriptions.
 */
function freezePanesEntries(): TemplateResult[] {
  return [
    described(
      'Freeze Panes',
      'table-freeze-column-and-row',
      'Keep rows and columns visible while the rest of the worksheet scrolls (based on current selection).',
    ),
    described('Freeze Top Row', 'table-freeze-row', 'Keep the top row visible while scrolling through the rest of the worksheet.'),
    described(
      'Freeze First Column',
      'table-freeze-column',
      'Keep the first column visible while scrolling through the rest of the worksheet.',
    ),
  ];
}

/**
 * **The Sheet View dropdown's options**, which each Excel host renders inside its `<mjx-dropdown>`: Default,
 * and every sheet view the workbook has saved. The catalogue's workbook has saved none.
 */
export const excelSheetViews: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'default', label: 'Default' },
];

/**
 * Two menus. The Sheet View dropdown is a field over `excelSheetViews`, and Ruler, Gridlines, Formula Bar and
 * Headings are checkboxes the hosts bind.
 */
function excelViewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.view.window.freeze-panes', 'Freeze Panes', ...freezePanesEntries())}
    ${commandMenu(host, 'excel.view.window.switch-windows', 'Switch Windows', ...switchWindowsEntries('Findings'))}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/** Every application's View menus. */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordViewMenus,
  powerpoint: powerpointViewMenus,
  excel: excelViewMenus,
};

/**
 * Every menu one application's View tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `reviewMenus` is.
 */
export function viewMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return menusByApplication[application]?.(host) ?? nothing;
}
