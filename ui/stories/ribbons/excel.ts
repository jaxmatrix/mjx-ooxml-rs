/**
 * **Excel's ribbon** — every tab, in Office's order, from one source.
 *
 * The shape and the reasoning are `stories/ribbons/word.ts`'s. Two things are Excel's own:
 *
 * 1. **The File tab has no Print group**, because the census has no backstage `TabPrint` row for
 *    Excel and does have a `Publish2Tab` the other two lack. That is a gap in the dump rather than
 *    a fact about Excel, and `dev/ribbons/census.ts` records it where a reader will meet it. The
 *    tab itself is authored — the ribbon programme's unit 1 — so the difference is now visible
 *    rather than described.
 * 2. **Home carries eight in-scope groups and unit 2 renders all eight.** `GroupCells` and
 *    `GroupHomePowerOptions` had been declared in the census since unit 0 and rendered by nothing;
 *    they arrive with the rest of Home. The second of them is the one group on this tab whose
 *    single control the census names only by the group's own id, and `dev/ribbons/census.ts`
 *    records that rather than inventing an Office command to fill it.
 * 3. **Insert is unit 3, and the largest of the three**: ten groups and thirty-five commands, because
 *    Charts alone draws eleven — Recommended Charts, eight chart families as glyphs, Maps and
 *    PivotChart. Nineteen commands are dropdowns or split buttons a host binds by id, and Charts is
 *    the only group on any application's Insert tab with a dialog launcher.
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

/**
 * File: Info, Open, Save, Share, Export, Publish, Help — and **no Print group**.
 *
 * The shape and the reasoning are `stories/ribbons/word.ts`'s. Excel's two differences are both the
 * census's rather than this module's, and both are recorded in `dev/ribbons/census.ts` rather than
 * smoothed over here:
 *
 * 1. **There is no backstage `TabPrint` row for Excel**, so there is no Print group to author.
 *    Excel obviously has a File → Print page, so this is a gap in the census dump — and inventing a
 *    row would be exactly the drift the transcription exists to prevent.
 * 2. **There is a `Publish2Tab` the other two lack**, which is Excel's Publish to Power BI page. Its
 *    three controls are the one place on this whole tab where what Office shows and what the census
 *    counts agree exactly.
 *
 * Info carries a fifth command the other two applications have no equivalent of: Workbook
 * Statistics.
 */
export function excelFileTab(options: TabOptions = {}): TemplateResult {
  const file = entry('file');
  const controls = options.controls ?? {};
  return tab(
    file.id,
    file.label,
    censusGroup(file, 'TabInfo', {}, controls),
    censusGroup(file, 'TabRecent', {}, controls),
    censusGroup(file, 'TabSave', {}, controls),
    censusGroup(file, 'TabShare', {}, controls),
    censusGroup(file, 'TabPublish', {}, controls),
    censusGroup(file, 'Publish2Tab', {}, controls),
    censusGroup(file, 'TabHelp', {}, controls),
  );
}

/**
 * Home: Clipboard, Font, Alignment, Number, Styles, Cells, Editing, Power Options — the ribbon
 * programme's unit 2, and **all eight** of the groups the census marks in scope.
 *
 * Two of them arrive here. `GroupCells` has been declared since unit 0 and rendered by nothing,
 * which meant an Excel ribbon in this catalogue could not insert a row. `GroupHomePowerOptions` is
 * the one group in this whole tab whose contents the census does not describe at all — see
 * `dev/ribbons/census.ts`, which records what is known and refuses to guess the rest.
 *
 * **Three launchers, and Styles, Cells, Editing and Power Options have none.** Office's Format
 * Cells dialog has a Font tab, an Alignment tab and a Number tab, and the three launchers open
 * exactly those; the other four groups open menus rather than property sheets.
 */
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
    censusGroup(home, 'GroupCells', {}, controls),
    censusGroup(home, 'GroupEditingExcel', {}, controls),
    censusGroup(home, 'GroupHomePowerOptions', {}, controls),
  );
}

/**
 * Insert: Tables, Illustrations, Charts, Sparklines, Filters, Links, Comments, Text, Symbols, Cell
 * Controls — the ribbon programme's unit 3.
 *
 * `GUESS:` **Cell Controls is drawn last**, where the declaration puts it; Office added the group
 * recently and this project cannot cite a build that fixes its position. The other nine are Office's
 * order. Office's Add-ins and Tours (3D Map) groups are out of scope in the census and are not drawn.
 * The group Office labels **Filters** is `GroupSlicerInsert` and draws the census's *Slicers*; see
 * `dev/ribbons/census.ts` on why the label is not changed here.
 *
 * **Nineteen of the tab's thirty-five commands are bound by the host**: PivotTable, Pictures, Shapes,
 * 3D Models, Screenshot, the eight chart families, Maps, PivotChart, Link, WordArt, Signature Line
 * and Equation. Each opens a menu from `stories/ribbons/insert-menus.ts`.
 *
 * **One dialog launcher, on Charts**, because Office has one there: it opens the Insert Chart dialog
 * on its All Charts page, which is the whole gallery the eight glyphs are the front of. No other
 * group on the tab has a property sheet behind it.
 *
 * **No group keeps a survivor** — including Cell Controls, whose Checkbox passes rules 1 and 2 and is
 * the group's only command.
 */
export function excelInsertTab(options: TabOptions = {}): TemplateResult {
  const insert = entry('insert');
  const controls = options.controls ?? {};
  return tab(
    insert.id,
    insert.label,
    censusGroup(insert, 'GroupInsertTablesExcel', {}, controls),
    censusGroup(insert, 'GroupInsertIllustrations', {}, controls),
    censusGroup(insert, 'GroupInsertChartsExcel', { launcher: 'See all charts' }, controls),
    censusGroup(insert, 'GroupSparklinesInsert', {}, controls),
    censusGroup(insert, 'GroupSlicerInsert', {}, controls),
    censusGroup(insert, 'GroupInsertLinks', {}, controls),
    censusGroup(insert, 'GroupInsertComments', {}, controls),
    censusGroup(insert, 'GroupInsertText', {}, controls),
    censusGroup(insert, 'GroupInsertSymbols', {}, controls),
    censusGroup(insert, 'GroupCellControls', {}, controls),
  );
}

// ── the tabs their own units author ──────────────────────────────────────────

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
