/**
 * **Excel's ribbon** — every tab, in Office's order, from one source.
 *
 * The shape and the reasoning are `stories/ribbons/word.ts`'s. Two things are Excel's own:
 *
 * 1. **The File tab has no Print group**, because the census has no backstage `TabPrint` row for
 *    Excel and does have a `Publish2Tab` the other two lack. That is a gap in the dump rather than
 *    a fact about Excel, and `dev/ribbons/census.ts` records it where a reader will meet it.
 * 2. **Home carries eight in-scope groups and this renders six.** `GroupCells` and
 *    `GroupHomePowerOptions` are declared in the census and are not on the shell's Home today, so
 *    authoring them is unit 2's work rather than something this file invents.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { excelRibbonTabs, ribbonTab } from '../../dev/ribbons/census.ts';
import { stubTab } from '../shell/shell-parts.ts';
import {
  censusGroup,
  placeholderTab,
  tab,
  tabsFor,
  type TabOptions,
} from './ribbon-parts.ts';

const entry = (id: string) => ribbonTab('excel', id);

// ── the authored tabs ────────────────────────────────────────────────────────

/** Home: Clipboard, Font, Alignment, Number, Styles, Editing — in Office's order. */
export function excelHomeTab(options: TabOptions = {}): TemplateResult {
  const home = entry('home');
  const controls = options.controls ?? {};
  return tab(
    home.id,
    home.label,
    censusGroup(home, 'GroupClipboard', { launcher: 'Clipboard settings' }, controls),
    censusGroup(home, 'GroupFont', { launcher: 'Format cells: font' }, controls),
    censusGroup(home, 'GroupAlignmentExcel', { launcher: 'Format cells: alignment' }, controls),
    censusGroup(home, 'GroupNumber', { launcher: 'Format cells: number' }, controls),
    censusGroup(home, 'GroupStyles', {}, controls),
    censusGroup(home, 'GroupEditingExcel', {}, controls),
  );
}

// ── the tabs their own units author ──────────────────────────────────────────

export function excelFileTab(): TemplateResult {
  return placeholderTab(entry('file'));
}

export function excelInsertTab(): TemplateResult {
  return placeholderTab(entry('insert'));
}

export function excelDrawTab(): TemplateResult {
  return placeholderTab(entry('draw'));
}

export function excelPageLayoutTab(): TemplateResult {
  return placeholderTab(entry('page-layout'));
}

export function excelFormulasTab(): TemplateResult {
  return placeholderTab(entry('formulas'));
}

export function excelDataTab(): TemplateResult {
  return placeholderTab(entry('data'));
}

export function excelReviewTab(): TemplateResult {
  return placeholderTab(entry('review'));
}

export function excelViewTab(): TemplateResult {
  return placeholderTab(entry('view'));
}

export function excelPrintPreviewTab(): TemplateResult {
  return placeholderTab(entry('print-preview'));
}

export function excelBackgroundRemovalTab(): TemplateResult {
  return placeholderTab(entry('background-removal'));
}

// ── the whole ribbon ─────────────────────────────────────────────────────────

/** Which function builds which tab. Keyed by the census's own kebab ids. */
const builders: Readonly<Record<string, (options: TabOptions) => TemplateResult>> = {
  file: excelFileTab,
  home: excelHomeTab,
  insert: excelInsertTab,
  draw: excelDrawTab,
  'page-layout': excelPageLayoutTab,
  formulas: excelFormulasTab,
  data: excelDataTab,
  review: excelReviewTab,
  view: excelViewTab,
  'print-preview': excelPrintPreviewTab,
  'background-removal': excelBackgroundRemovalTab,
};

/** Every tab, in Office's order. See `wordTabs` on why `includeViewTabs` is a parameter. */
export function excelTabs(
  options: TabOptions & { readonly includeViewTabs?: boolean } = {},
): TemplateResult[] {
  return tabsFor(
    excelRibbonTabs,
    (declared) => {
      const build = builders[declared.id];
      if (build === undefined) {
        throw new Error(`stories/ribbons/excel.ts has no builder for the '${declared.id}' tab`);
      }
      return build(options);
    },
    options,
  );
}

/** The contextual tab sets the shell declares today. Unit 11's work — see `wordContextualSets`. */
export function excelContextualSets(): TemplateResult {
  return html`
    <mjx-contextual-tab-set label="Table Tools">
      ${stubTab('table-design', 'Design', 'Table Styles', 'table')}
    </mjx-contextual-tab-set>
  `;
}
