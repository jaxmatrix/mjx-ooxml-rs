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
 * 4. **Draw is unit 4**: eight groups and fourteen commands, declared by the same functions as the
 *    other two. Excel's differences are its census's: no Stencils group, and an Eraser with nothing
 *    behind an arrow.
 * 5. **Page Layout is unit 5**: five groups and twenty-four commands. Arrange is declared by the same
 *    function as Word's Layout, and Scale to Fit and Sheet Options are fields and checkboxes rather than
 *    buttons.
 * 6. **Formulas is unit 6**: four groups and twenty-four commands. Fluent draws Excel's own function
 *    library books, and fourteen commands open a menu.
 * 7. **Data is unit 7**: nine groups and thirty-three commands, three of the groups one Office group in
 *    three generations, and Sort A to Z and Sort Z to A the tab's survivors.
 * 8. **Review** followed Word's and PowerPoint's, one tab of one application: eleven groups and
 *    twenty-three commands. Comments is three generations of Office, as Data's Connections is, and every
 *    menu on the tab carries Office's whole list.
 * 9. **View** followed Word's and PowerPoint's View, one tab of one application: seven groups and
 *    twenty-eight commands, the tab that changes how a workbook is looked at and never the workbook. Workbook
 *    Views is its one exclusive set, Freeze Panes and Switch Windows its two menus.
 * 10. **Background Removal** followed Word's and PowerPoint's, Excel's first view tab authored: Word's two groups
 *     and four commands under Excel's ids, from the census's shared functions. It binds nothing and opens no
 *     menu.
 * 11. **Print Preview** followed PowerPoint's, Excel's second view tab authored: three groups and seven
 *     commands, the sheet as it will print. It opens no menu, binds one checkbox (Show Margins) in
 *     `Ribbons/Excel` alone, and draws its groups in Office's order rather than the census's.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { excelRibbonTabs, ribbonTab } from '../../dev/ribbons/census.ts';
import { stubTab } from '../shell/shell-parts.ts';
import {
  censusGroup,
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

/**
 * Draw: Drawing Tools, Pens, Write, Input Mode, Draw with Touch, Replay, Help, Close — the ribbon
 * programme's unit 4.
 *
 * PowerPoint's Draw tab without Stencils, which Excel's census does not declare. Every group is
 * declared by the same function as Word's and PowerPoint's; see `wordDrawTab` and
 * `dev/ribbons/census.ts`, including `GUESS:` **the order is the declaration's**.
 *
 * **Eraser is the one command drawn differently from the other two applications.** Excel's census
 * counts **five** controls in Write, exactly its five tools, so Eraser has no sizes behind an arrow.
 * It is the plain toggle the census declares, and Excel's hosts bind nothing over it.
 *
 * **Five of the tab's fourteen commands are bound by the host**: Add Pen, Pens, Colour, Thickness and
 * Touch/Mouse Mode, over the menus in `stories/ribbons/draw-menus.ts`. **No dialog launchers**, and
 * **one survivor**, Select Objects.
 */
export function excelDrawTab(options: TabOptions = {}): TemplateResult {
  const draw = entry('draw');
  const controls = options.controls ?? {};
  return tab(
    draw.id,
    draw.label,
    censusGroup(draw, 'GroupDrawingTools', {}, controls),
    censusGroup(draw, 'GroupPens2', {}, controls),
    censusGroup(draw, 'GroupWrite', {}, controls),
    censusGroup(draw, 'GroupInputMode', {}, controls),
    censusGroup(draw, 'GroupDrawWithTouch', {}, controls),
    censusGroup(draw, 'InkReplay', {}, controls),
    censusGroup(draw, 'GroupPenAndInkHelp', {}, controls),
    censusGroup(draw, 'GroupInkClose', {}, controls),
  );
}

/**
 * Page Layout: Themes, Page Setup, Scale to Fit, Sheet Options, Arrange — the ribbon programme's unit 5,
 * in Office's order.
 *
 * **Three dialog launchers, all onto one dialog**, as Office has them: Page Setup, Scale to Fit and Sheet
 * Options each open Page Setup, on its Page, Page and Sheet pages. Themes and Arrange have none.
 *
 * **Scale to Fit is three fields** (Width and Height dropdowns, a Scale combo box) and **Sheet Options is
 * four checkboxes**, each bound by the host. **Arrange** is Word's Layout Arrange without Position and
 * Wrap Text, from the same declaration, with Bring Forward and Send Backward large at its head.
 *
 * **Twenty-one of the tab's twenty-four commands are bound by the host**: the four theme dropdowns,
 * five Page Setup dropdowns, the three fields, the four checkboxes, and five of Arrange's six over
 * `stories/ribbons/design-layout-menus.ts`. Background, Print Titles and Selection Pane are the generic
 * buttons and toggle. **No survivor.**
 */
export function excelPageLayoutTab(options: TabOptions = {}): TemplateResult {
  const pageLayout = entry('page-layout');
  const controls = options.controls ?? {};
  return tab(
    pageLayout.id,
    pageLayout.label,
    censusGroup(pageLayout, 'GroupThemesExcel', {}, controls),
    censusGroup(pageLayout, 'GroupPageSetup', { launcher: 'Page setup' }, controls),
    censusGroup(pageLayout, 'GroupPageLayoutScaleToFit', { launcher: 'Page setup: scaling' }, controls),
    censusGroup(pageLayout, 'GroupPageLayoutSheetOptions', { launcher: 'Page setup: sheet' }, controls),
    censusGroup(pageLayout, 'GroupArrange', {}, controls),
  );
}

/**
 * Formulas: Function Library, Named Cells (Office's *Defined Names*), Formula Auditing, Calculation — the
 * ribbon programme's unit 6, in Office's order.
 *
 * Office's four Python groups are out of scope in the census and are not drawn. The second group draws
 * the census's label; `dev/ribbons/census.ts` records Office's.
 *
 * **Fourteen of the tab's twenty-four commands are bound by the host**: AutoSum, Define Name, Remove
 * Arrows and Error Checking are split buttons, and the eight function categories, Use in Formula and
 * Calculation Options are dropdowns, over `stories/ribbons/references-transitions-formulas-menus.ts`.
 * Show Formulas is the generic toggle.
 *
 * **No dialog launchers**: Office puts none on its Formulas tab. **No survivor.**
 */
export function excelFormulasTab(options: TabOptions = {}): TemplateResult {
  const formulas = entry('formulas');
  const controls = options.controls ?? {};
  return tab(
    formulas.id,
    formulas.label,
    censusGroup(formulas, 'GroupFunctionLibrary', {}, controls),
    censusGroup(formulas, 'GroupNamedCells', {}, controls),
    censusGroup(formulas, 'GroupFormulaAuditing', {}, controls),
    censusGroup(formulas, 'GroupCalculation', {}, controls),
  );
}

/**
 * Data: Get External Data, Queries & Connections, Workbook Links, Connections, Data Types, Sort & Filter,
 * Data Tools, Forecast, Outline — the ribbon programme's unit 7, in the census's order, which follows
 * Office's.
 *
 * ⚠ **Four of the nine groups are generations of Office rather than groups one build draws side by
 * side**: Get External Data is Office 2016's, and Queries & Connections, Workbook Links and Connections
 * are one group three times over. `dev/ribbons/census.ts` records how each command is drawn once, and why
 * Microsoft 365's Get Data is not here.
 *
 * **Eight of the tab's thirty-three commands are bound by the host**: Refresh All, Data Validation, Group
 * and Ungroup are split buttons, From Other Sources and What-If Analysis are dropdowns, over
 * `stories/ribbons/mailings-animations-data-menus.ts`, and Data Types is an in-ribbon gallery. Queries &
 * Connections, Workbook Links and Filter are the generic toggle.
 *
 * **One dialog launcher, on Outline**, because Office has one there: it opens the outline Settings.
 * **Two survivors**, Sort A to Z and Sort Z to A.
 */
export function excelDataTab(options: TabOptions = {}): TemplateResult {
  const data = entry('data');
  const controls = options.controls ?? {};
  return tab(
    data.id,
    data.label,
    censusGroup(data, 'GroupGetExternalData', {}, controls),
    censusGroup(data, 'GroupDataQueriesAndConnections', {}, controls),
    censusGroup(data, 'GroupDataQueriesAndConnectionsWorkbookLinks', {}, controls),
    censusGroup(data, 'GroupConnections', {}, controls),
    censusGroup(data, 'GroupLinkedEntityConvert', {}, controls),
    censusGroup(data, 'GroupSortFilter', {}, controls),
    censusGroup(data, 'GroupDataTools', {}, controls),
    censusGroup(data, 'GroupForecast', {}, controls),
    censusGroup(data, 'GroupOutline', { launcher: 'Outline settings' }, controls),
  );
}

/**
 * Review: Proofing, Performance, Accessibility, Language, Threaded Comments, Comments, Notes, Protect,
 * Changes, Ink, Debug, in **Office's** order where Office has one.
 *
 * ⚠ **Performance is second here and tenth in the census's declaration**, where Microsoft 365 draws Check
 * Performance beside Proofing; **Ink is drawn after Protect and Changes**, where Microsoft 365 draws it
 * last. Debug stays last, where the declaration puts it, because nothing says where Office does. `GUESS:`
 * all three positions, and `dev/ribbons/census.ts` records the disagreement. Office's Insights (Smart
 * Lookup) and Lineage are out of scope in the census and are not drawn.
 *
 * ⚠ **Threaded Comments, Comments and Notes are one Office group in three generations**, and each face
 * command is drawn once; `dev/ribbons/census.ts` gives the reading.
 *
 * **Four of the tab's twenty-three commands are bound by the host**: Check Accessibility is a split
 * button, Hide Ink is a split button whose face is a toggle, and Notes and Track Changes are dropdowns.
 * All four open menus from `stories/ribbons/review-menus.ts`. Show Comments, Show/Hide Comment, Show All
 * Comments and Protect Workbook are the generic toggle.
 *
 * **No dialog launchers**, because Office puts none here. **Two survivors**, Previous Comment and Next
 * Comment, and `dev/ribbons/census.ts` gives the reason.
 */
export function excelReviewTab(options: TabOptions = {}): TemplateResult {
  const review = entry('review');
  const controls = options.controls ?? {};
  return tab(
    review.id,
    review.label,
    censusGroup(review, 'GroupProofing', {}, controls),
    censusGroup(review, 'GroupPerformance', {}, controls),
    censusGroup(review, 'GroupAccessibility', {}, controls),
    censusGroup(review, 'GroupLanguage', {}, controls),
    censusGroup(review, 'GroupThreadedComments', {}, controls),
    censusGroup(review, 'GroupComments', {}, controls),
    censusGroup(review, 'GroupCommentsLegacy', {}, controls),
    censusGroup(review, 'GroupProtectExcel', {}, controls),
    censusGroup(review, 'GroupChangesExcel', {}, controls),
    censusGroup(review, 'GroupInk', {}, controls),
    censusGroup(review, 'GroupDebug', {}, controls),
  );
}

/**
 * View: Sheet View, Workbook Views, Show, Zoom, Window, Night Mode, Debug, in **Office's** order where Office
 * has one.
 *
 * ⚠ **Sheet View is first here and fifth in the census's declaration**, where Microsoft 365 draws it, at the
 * tab's left edge. Night Mode and Debug stay last, where the declaration puts them, because nothing says
 * where Office does. `GUESS:` both positions, and `dev/ribbons/census.ts` records the disagreement. Office's
 * **Macros** is out of scope in the census and is not drawn.
 *
 * **Seven of the tab's twenty-eight commands are bound by the host**: the Sheet View dropdown is a field;
 * Ruler, Gridlines, Formula Bar and Headings are checkboxes; Freeze Panes and Switch Windows are dropdowns
 * over `stories/ribbons/view-menus.ts`. Everything else is the generic toggle or button, including the
 * Workbook Views exclusive set.
 *
 * **No dialog launchers**, as in Office. **Two survivors**, 100% and Zoom to Selection, in Zoom.
 */
export function excelViewTab(options: TabOptions = {}): TemplateResult {
  const view = entry('view');
  const controls = options.controls ?? {};
  return tab(
    view.id,
    view.label,
    censusGroup(view, 'GroupNamedSheetView', {}, controls),
    censusGroup(view, 'GroupWorkbookViews', {}, controls),
    censusGroup(view, 'GroupViewShowHide', {}, controls),
    censusGroup(view, 'GroupZoom', {}, controls),
    censusGroup(view, 'GroupWindow', {}, controls),
    censusGroup(view, 'GroupNightMode', {}, controls),
    censusGroup(view, 'GroupViewDebug', {}, controls),
  );
}

/**
 * Background Removal: Refine, Close — the first Excel view tab authored, in **Office's** order, which is also
 * the census's.
 *
 * **Word's tab with Excel's ids**: the census row calls `backgroundRemovalRefineCommands('excel')` and
 * `backgroundRemovalCloseCommands('excel')`, so the shape and every disagreement are `wordBackgroundRemovalTab`'s,
 * recorded once in `dev/ribbons/census.ts`. Excel adds none.
 *
 * ⚠ **A view tab: Office shows it only while a picture's background is being removed**, so `excelTabs()`
 * leaves it out unless `includeViewTabs` is asked for.
 *
 * **Nothing is bound by the host**: two generic toggles in one exclusive set that may hold none, and two generic
 * buttons. **No menus, no dialog launchers, no survivors.**
 */
export function excelBackgroundRemovalTab(options: TabOptions = {}): TemplateResult {
  const backgroundRemoval = entry('background-removal');
  const controls = options.controls ?? {};
  return tab(
    backgroundRemoval.id,
    backgroundRemoval.label,
    censusGroup(backgroundRemoval, 'GroupBackgroundRemovalMode', {}, controls),
    censusGroup(backgroundRemoval, 'GroupBackgroundRemovalClose', {}, controls),
  );
}

/**
 * Print Preview: Print, Zoom, Preview — Excel's second view tab authored, in **Office's** order, where the census
 * declares Print, Preview, Zoom. `dev/ribbons/census.ts` records the disagreement.
 *
 * ⚠ **A view tab: Office shows it only inside Print Preview**, so `excelTabs()` leaves it out unless
 * `includeViewTabs` is asked for. Only `Ribbons/Excel` asks, which is why the tab's binding is written there and
 * nowhere else.
 *
 * **One of the tab's seven commands is bound by the host**: Show Margins, a checkbox. Everything else is the
 * generic button. **No menu**: `printPreviewMenus('excel', …)` renders an empty set.
 *
 * **No dialog launcher.** **Two survivors**, Next Page and Previous Page in Preview, as on Word's and PowerPoint's
 * Print Preview.
 */
export function excelPrintPreviewTab(options: TabOptions = {}): TemplateResult {
  const printPreview = entry('print-preview');
  const controls = options.controls ?? {};
  return tab(
    printPreview.id,
    printPreview.label,
    censusGroup(printPreview, 'GroupPrintPreviewPrint', {}, controls),
    censusGroup(printPreview, 'GroupPrintPreviewZoom', {}, controls),
    censusGroup(printPreview, 'GroupPrintPreviewPreview', {}, controls),
  );
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
