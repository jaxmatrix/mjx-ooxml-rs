/**
 * **The menus the Print Preview tab opens**, written once for every host that draws the tab.
 *
 * **Word's Print Preview tab**, authored under the one-tab-one-application rule. The pattern is
 * `stories/ribbons/view-menus.ts`'s, for its reasons. A binding lives in its host. The menu it opens is
 * written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `printPreviewMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below
 * spells its command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## Shaped for three applications, authored for all three
 *
 * All three applications carry a Print Preview tab in the census, and each is its own unit. So the file is
 * keyed by application exactly as `view-menus.ts` is: **each unit adds one function to `menusByApplication`**
 * and reaches for the shared entries, rather than writing a second file. **PowerPoint's unit** added
 * `powerpointPrintPreviewMenus`, which reuses `orientationEntries()` alone (PowerPoint has no Margins or Size),
 * and the option lists of its two fields, `powerpointPrintWhat` and `powerpointPrintColourModes`, which
 * `Ribbons/PowerPoint` renders inside its own `<mjx-dropdown>`s. **Excel's unit** added
 * `excelPrintPreviewMenus`, which is **authored and empty**: Excel's tab opens no menu, so its entry is present
 * and renders nothing, which is the claim that the set was found empty.
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
 * `stories/ribbons/word.stories.ts`, `stories/ribbons/powerpoint.stories.ts` and
 * `stories/ribbons/excel.stories.ts` are the hosts that bind and render these**, Excel's an empty set. No shell
 * draws the tab, and `tests/ribbons.test.ts` refuses a shell that opens one of these menus.
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

// ── PowerPoint's Print Preview ───────────────────────────────────────────────

/** One option of a field: the value a host starts it on, and what the list draws. */
interface FieldOption {
  readonly value: string;
  readonly label: string;
}

/**
 * **The Print What field's options**: PowerPoint's nine printout shapes, in Office's order. A host starts it on
 * `slides`, a new deck's.
 */
export const powerpointPrintWhat: readonly FieldOption[] = [
  { value: 'slides', label: 'Slides' },
  { value: 'handouts-1', label: 'Handouts (1 Slide Per Page)' },
  { value: 'handouts-2', label: 'Handouts (2 Slides Per Page)' },
  { value: 'handouts-3', label: 'Handouts (3 Slides Per Page)' },
  { value: 'handouts-4', label: 'Handouts (4 Slides Per Page)' },
  { value: 'handouts-6', label: 'Handouts (6 Slides Per Page)' },
  { value: 'handouts-9', label: 'Handouts (9 Slides Per Page)' },
  { value: 'notes-pages', label: 'Notes Pages' },
  { value: 'outline-view', label: 'Outline View' },
];

/**
 * **The Colour/Greyscale field's options**, in the census's spelling rather than Office's *Color* and
 * *Grayscale*. A host starts it on `colour`. `GUESS:` the start: Office starts it on what the printer can do.
 */
export const powerpointPrintColourModes: readonly FieldOption[] = [
  { value: 'colour', label: 'Colour' },
  { value: 'greyscale', label: 'Greyscale' },
  { value: 'pure-black-and-white', label: 'Pure Black and White' },
];

/**
 * Options: PowerPoint 2007's printing options, with *Print Order* flattened into a section. **No
 * *Color/Grayscale* submenu**, because this catalogue draws that setting as Page Setup's field; see the census.
 * `GUESS:` the entries, their order and every tick.
 */
function powerpointPrintOptionsEntries(): TemplateResult[] {
  return [
    html`<mjx-menu-item label="Header and Footer…"></mjx-menu-item>`,
    html`<mjx-menu-separator></mjx-menu-separator>`,
    html`<mjx-menu-item kind="checkbox" label="Scale to Fit Paper"></mjx-menu-item>`,
    html`<mjx-menu-item kind="checkbox" label="Frame Slides"></mjx-menu-item>`,
    html`<mjx-menu-item kind="checkbox" label="Print Comments and Ink Markup"></mjx-menu-item>`,
    html`<mjx-menu-separator></mjx-menu-separator>`,
    html`<mjx-menu-section label="Print Order">
      <mjx-menu-item kind="radio" label="Horizontal" checked></mjx-menu-item>
      <mjx-menu-item kind="radio" label="Vertical"></mjx-menu-item>
    </mjx-menu-section>`,
    html`<mjx-menu-separator></mjx-menu-separator>`,
    html`<mjx-menu-item kind="checkbox" label="Print Hidden Slides"></mjx-menu-item>`,
  ];
}

/**
 * Two menus: Options, PowerPoint's own, and Orientation, **Layout's list unchanged**, because PowerPoint's two
 * entries and their start are Word's. Print What and Colour/Greyscale are fields over the lists above.
 */
function powerpointPrintPreviewMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.print-preview.print.options', 'Options', ...powerpointPrintOptionsEntries())}
    ${commandMenu(host, 'powerpoint.print-preview.page-setup.orientation', 'Orientation', ...orientationEntries())}
  `;
}

// ── Excel's Print Preview ────────────────────────────────────────────────────

/**
 * **No menus, and that is a finding rather than a gap.** Excel 2007's tab opens none: Print and Page Setup open
 * dialogs, Zoom switches the preview's magnification, Next Page and Previous Page move it, Show Margins is a
 * checkbox `Ribbons/Excel` binds, and Close Print Preview leaves the view. So Excel's entry is present, and renders
 * an empty set, where an absent entry would have said the tab was still unauthored.
 *
 * **No shared list is reused**, because no Excel command opens one: Excel's tab has no Margins, Orientation or
 * Size, which Page Layout carries instead.
 */
function excelPrintPreviewMenus(): TemplateResult {
  return html``;
}

// ── what a host renders ──────────────────────────────────────────────────────

/** Every application's Print Preview menus, as each application's unit authors them. */
const menusByApplication: Partial<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordPrintPreviewMenus,
  powerpoint: powerpointPrintPreviewMenus,
  excel: excelPrintPreviewMenus,
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
