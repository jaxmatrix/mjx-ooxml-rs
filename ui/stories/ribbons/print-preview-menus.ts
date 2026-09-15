/**
 * **The menus the Print Preview tab opens**, written once for every host that draws the tab.
 *
 * **Word's Print Preview tab**, authored under the one-tab-one-application rule. The pattern is
 * `stories/ribbons/view-menus.ts`'s, for its reasons. A binding lives in its host. The menu it opens is
 * written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `printPreviewMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below
 * spells its command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## Shaped for three applications, authored for one
 *
 * All three applications carry a Print Preview tab in the census, and each is its own unit. So the file is
 * keyed by application exactly as `view-menus.ts` is: **PowerPoint's and Excel's units add one function each
 * to `menusByApplication`** and reach for the shared entries below, rather than writing a second file. Until
 * they do, `printPreviewMenus` renders nothing for them, which is the absence of a menu set rather than a
 * claim that one was found empty.
 *
 * ## The entries are Layout's, not copies
 *
 * Margins, Orientation and Size on Print Preview are the same commands as on Word's Layout tab, and Office
 * opens the same lists. So the entries are **imported** from `stories/ribbons/design-layout-menus.ts`
 * (`marginEntries`, `orientationEntries`, `sizeEntries`), and only the menus' ids are this tab's own, because
 * a menu's id is derived from the command that opens it. Excel's Page Layout already calls the same three
 * with `'excel'`, which is the reuse an Excel Print Preview unit would inherit.
 *
 * **This unit completed Layout's Margins and Size lists**, so Layout's tab draws the same whole lists: Margins
 * adds *Office 2003 Default* (and *Last Custom Setting* when present), and Size is Word's standard paper list.
 * ⚠ Office's real Size list comes from the selected printer; see `sizeEntries`.
 *
 * ## Who renders these
 *
 * Print Preview is `appearance: 'view'`: Office shows it only inside Print Preview, so `tabsFor` leaves it
 * out of a strip unless `includeViewTabs` is asked for, and only the `Ribbons/*` hosts ask. **So
 * `stories/ribbons/word.stories.ts` is the one host that binds and renders these.** `Shell/Word` never draws
 * the tab, and `tests/ribbons.test.ts` refuses a shell that opens one of these menus.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no page changes, because
 * command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { marginEntries, orientationEntries, sizeEntries } from './design-layout-menus.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── Word's Print Preview ─────────────────────────────────────────────────────

/** Three menus, Page Setup's. Show Ruler and Magnifier are checkboxes the host binds. */
function wordPrintPreviewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.print-preview.page-setup.margins', 'Margins', ...marginEntries('word'))}
    ${commandMenu(host, 'word.print-preview.page-setup.orientation', 'Orientation', ...orientationEntries())}
    ${commandMenu(host, 'word.print-preview.page-setup.size', 'Size', ...sizeEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/** Every application's Print Preview menus, as each application's unit authors them. */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordPrintPreviewMenus,
};

/**
 * Every menu one application's Print Preview tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `viewMenus` is, by a host that draws
 * view tabs.
 */
export function printPreviewMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return menusByApplication[application]?.(host) ?? nothing;
}
