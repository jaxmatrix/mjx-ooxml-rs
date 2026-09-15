/**
 * **Word's ribbon** — every tab, in Office's order, from one source.
 *
 * `Ribbons/Word` renders these tab by tab for audit; `Shell/Word` renders the same functions inside
 * the assembled application. There is no second copy, which is the point: a ribbon that had been
 * authored twice would look right in the catalogue and drift in the shell, and nobody would notice
 * until a reviewer opened both on the same afternoon.
 *
 * ## What is real and what is a placeholder, today
 *
 * **File** is the ribbon programme's unit 1: seven groups — Info, Open, Save, Print, Share, Export,
 * Help — built from the census's *backstage* rows, because decision 1 of the approved plan makes
 * File an ordinary tab rather than a separate screen.
 *
 * **Home** is unit 2: all **six** of the groups the census marks in scope — Clipboard, Font,
 * Paragraph, Styles, Editing and Editor — carrying every command Office's Home tab shows. The
 * migrated set this file held after unit 0 is gone; what replaced it is roughly four times as
 * many commands, in Office's own order, mostly drawn icon-only because that is how Office draws
 * them.
 *
 * **Insert** is unit 3: all nine in-scope groups, twenty-eight commands, and the first tab where most
 * of the face opens something — eighteen of those commands are dropdowns or split buttons that a
 * host binds by id, over the menus `stories/ribbons/insert-menus.ts` writes once for both hosts.
 *
 * **Draw** is unit 4: all eleven in-scope groups and seventeen commands, the first tab made mostly of
 * *state* (the tool in hand, the ruler on the page), and the one where the census declares two
 * generations of Office's ink tools side by side.
 *
 * **Design** and **Layout** are unit 5: two groups and three, twenty-nine commands, and the tabs about the
 * whole document rather than a selection. The Style Set is an in-ribbon gallery, Indent and Spacing are
 * measure fields, and every other command but three opens a menu from
 * `stories/ribbons/design-layout-menus.ts`.
 *
 * **References** is unit 6: seven groups and twenty-two commands, the tab of generated content — the
 * tables, notes and bibliography Word writes for you — and the commands that keep it current.
 *
 * **Mailings** is unit 7: five groups and twenty-one commands, the mail merge pipeline from choosing a
 * document to finishing it, with a record navigator whose two carets survive a collapse.
 *
 * Every other tab is `placeholderTab`: one group carrying the tab's name, at the priority the census
 * declares, holding one honest button. Units 8 onward replace them one tab at a time, and each of
 * those is a small diff against a file that already has the right shape.
 *
 * ## The three view tabs
 *
 * Outlining, Print Preview and Background Removal are `appearance: 'view'` — Office shows them only
 * inside the view they name — so `wordTabs()` leaves them out unless asked. The catalogue still
 * gives each one a story, because a tab nobody can look at cannot be audited.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { ribbonTab, wordRibbonTabs } from '../../dev/ribbons/census.ts';
import { stubTab } from '../shell/shell-parts.ts';
import {
  censusGroup,
  placeholderTab,
  tab,
  tabsFor,
  type TabOptions,
} from './ribbon-parts.ts';

const entry = (id: string) => ribbonTab('word', id);

// ── the authored tabs ────────────────────────────────────────────────────────

/**
 * File: Info, Open, Save, Print, Share, Export, Help — in the order Office lists them down the left
 * of its backstage screen.
 *
 * Decision 1 of the approved plan makes this an **ordinary ribbon tab**, so the groups are the
 * backstage *destinations* and `censusGroup` is handed a census tab id (`TabRecent`, `TabPublish`)
 * rather than a group id. `dev/ribbons/census.ts`'s `RibbonTabSource` discriminant is what lets the
 * gate assert a real equality for both shapes.
 *
 * **Only Print has a dialog launcher**, and it is the one group here where Office genuinely has a
 * further surface to open: Page Setup. Info, Open, Save, Share, Export and Help are pages rather
 * than property sheets, and a launcher on one of them would promise a dialog that does not exist.
 */
export function wordFileTab(options: TabOptions = {}): TemplateResult {
  const file = entry('file');
  const controls = options.controls ?? {};
  return tab(
    file.id,
    file.label,
    censusGroup(file, 'TabInfo', {}, controls),
    censusGroup(file, 'TabRecent', {}, controls),
    censusGroup(file, 'TabSave', {}, controls),
    censusGroup(file, 'TabPrint', { launcher: 'Page setup' }, controls),
    censusGroup(file, 'TabShare', {}, controls),
    censusGroup(file, 'TabPublish', {}, controls),
    censusGroup(file, 'TabHelp', {}, controls),
  );
}

/**
 * Home: Clipboard, Font, Paragraph, Styles, Editing, Editor — the ribbon programme's unit 2.
 *
 * The group *order* is Office's and is this module's decision — the census has no column for it.
 * The labels, priorities and commands are the census's, so a group cannot quietly acquire a
 * different priority here from the one three gates read.
 *
 * **Editor is the sixth group and unit 2 is where it arrives.** The census has carried it since
 * unit 0 and the shells never had it, so a Word ribbon in this catalogue could not open the
 * proofing pane at all.
 *
 * **Four dialog launchers and no fifth.** Clipboard, Font and Paragraph each open a real Office
 * dialog, Styles opens the Styles pane, and Editor and Editing open neither — the first is a pane
 * the button itself opens, the second is a set of three commands with no property sheet behind
 * them. A launcher on either would promise a surface that does not exist, which is the argument
 * `wordFileTab` makes about Print.
 */
export function wordHomeTab(options: TabOptions = {}): TemplateResult {
  const home = entry('home');
  const controls = options.controls ?? {};
  return tab(
    home.id,
    home.label,
    censusGroup(home, 'GroupClipboard', { launcher: 'Clipboard settings' }, controls),
    censusGroup(home, 'GroupFont', { launcher: 'Font settings' }, controls),
    censusGroup(home, 'GroupParagraph', { launcher: 'Paragraph settings' }, controls),
    censusGroup(home, 'GroupStyles', { launcher: 'Styles pane' }, controls),
    censusGroup(home, 'GroupEditing', {}, controls),
    censusGroup(home, 'GroupEditor', {}, controls),
  );
}

/**
 * Insert: Pages, Tables, Illustrations, Media, Links, Comments, Header & Footer, Text, Symbols — the
 * ribbon programme's unit 3, in Office's order.
 *
 * **The order is Office's and the census's at once**, which is not true of PowerPoint's or Excel's:
 * the declaration was already written in Word's order. What Office draws that is not here is the
 * two groups the census marks out of scope — Add-ins, between Illustrations and Media, and Barcode.
 *
 * **Eighteen of the tab's twenty-eight commands are bound by the host**, because Office draws them as a
 * dropdown or a split button and `RibbonCommand` is a button or a toggle: Cover Page, Table, Pictures,
 * Shapes, 3D Models, Screenshot, Link, Header, Footer, Page Number, Text Box, Quick Parts, WordArt,
 * Drop Cap, Signature Line, Object, Equation and Symbol. Each opens a menu from
 * `stories/ribbons/insert-menus.ts`. A host that binds nothing still gets every command drawn in
 * place, as a button that opens nothing.
 *
 * **No dialog launchers.** Office puts none on Word's Insert tab: every group is a set of things to
 * insert, and none has a property sheet behind it.
 *
 * **No group keeps a survivor** — see `dev/ribbons/census.ts` for each group's reason.
 */
export function wordInsertTab(options: TabOptions = {}): TemplateResult {
  const insert = entry('insert');
  const controls = options.controls ?? {};
  return tab(
    insert.id,
    insert.label,
    censusGroup(insert, 'GroupInsertPages', {}, controls),
    censusGroup(insert, 'GroupInsertTables', {}, controls),
    censusGroup(insert, 'GroupInsertIllustrations', {}, controls),
    censusGroup(insert, 'GroupMedia', {}, controls),
    censusGroup(insert, 'GroupInsertLinks', {}, controls),
    censusGroup(insert, 'GroupInsertComments', {}, controls),
    censusGroup(insert, 'GroupHeaderFooter', {}, controls),
    censusGroup(insert, 'GroupInsertText', {}, controls),
    censusGroup(insert, 'GroupInsertSymbols', {}, controls),
  );
}

/**
 * Draw: Drawing Tools, Pens, Write, Stencils, Editing, Drawing Canvas, Input Mode, Draw with Touch,
 * Replay, Help, Close — the ribbon programme's unit 4, and the largest of the three Draw tabs.
 *
 * ⚠ **The census's Draw tab is two generations of Office on one tab.** Write, Pens and Close are
 * Office 2013's *Ink Tools | Pens* groups, and the rest are Microsoft 365's Draw tab. No Office build
 * draws them together, so `GUESS:` **the order is the declaration's** — see `dev/ribbons/census.ts`,
 * which also records that **Editing is `GroupEditingExcel`**, Excel's Home id, on Word's tab.
 *
 * **Six of the tab's seventeen commands are bound by the host**: Add Pen, Pens, Colour, Thickness and
 * Touch/Mouse Mode are dropdowns, and Eraser is a split button with the eraser sizes behind its arrow.
 * Each opens a menu from `stories/ribbons/draw-menus.ts`.
 *
 * **No dialog launchers**: Office puts none on its Draw tab. **One survivor**, Select Objects, in
 * Write.
 */
export function wordDrawTab(options: TabOptions = {}): TemplateResult {
  const draw = entry('draw');
  const controls = options.controls ?? {};
  return tab(
    draw.id,
    draw.label,
    censusGroup(draw, 'GroupDrawingTools', {}, controls),
    censusGroup(draw, 'GroupPens2', {}, controls),
    censusGroup(draw, 'GroupWrite', {}, controls),
    censusGroup(draw, 'GroupStencils', {}, controls),
    censusGroup(draw, 'GroupEditingExcel', {}, controls),
    censusGroup(draw, 'GroupInsertDrawingCanvas', {}, controls),
    censusGroup(draw, 'GroupInputMode', {}, controls),
    censusGroup(draw, 'GroupDrawWithTouch', {}, controls),
    censusGroup(draw, 'InkReplay', {}, controls),
    censusGroup(draw, 'GroupPenAndInkHelp', {}, controls),
    censusGroup(draw, 'GroupInkClose', {}, controls),
  );
}

/**
 * Design: Style Set (Office's *Document Formatting*) and Page Background — the ribbon programme's unit 5,
 * in Office's order.
 *
 * ⚠ **The first group's label is the census's, not Office's.** `GroupStyleSet` draws *Style Set*, where
 * Office writes *Document Formatting*; `dev/ribbons/census.ts` records it beside Insert's *Slicers*.
 *
 * **Eight of the tab's ten commands are bound by the host**: Themes, Colours, Fonts, Paragraph Spacing,
 * Effects and Watermark are dropdowns over `stories/ribbons/design-layout-menus.ts`, the Style Set is an
 * in-ribbon `<mjx-gallery>`, and Page Colour is a colour picker. Set as Default and Page Borders open a
 * dialog in Office and are the generic buttons.
 *
 * **No dialog launchers**: Office puts none on Word's Design tab. **No survivor.**
 */
export function wordDesignTab(options: TabOptions = {}): TemplateResult {
  const design = entry('design');
  const controls = options.controls ?? {};
  return tab(
    design.id,
    design.label,
    censusGroup(design, 'GroupStyleSet', {}, controls),
    censusGroup(design, 'GroupPageBackground', {}, controls),
  );
}

/**
 * Layout: Page Setup, Paragraph, Arrange — the ribbon programme's unit 5, in Office's order.
 *
 * **Two dialog launchers**, because Office has two: Page Setup opens the Page Setup dialog, and
 * Paragraph opens the Paragraph dialog, the one Home's launcher opens. Arrange has none.
 *
 * **Paragraph is four measure fields**, Indent Left and Right in centimetres and Spacing Before and
 * After in points, bound by each host as `<mjx-measure-input>`. **Arrange is declared once for Word and
 * Excel** (`arrangeCommands` in `dev/ribbons/census.ts`); Word's leads with Position and Wrap Text.
 *
 * **Eighteen of the tab's nineteen commands are bound by the host**: the seven Page Setup dropdowns,
 * the four fields, and seven of Arrange's eight, Bring Forward and Send Backward as split buttons.
 * Selection Pane is the generic toggle. **No survivor.**
 */
export function wordLayoutTab(options: TabOptions = {}): TemplateResult {
  const layout = entry('layout');
  const controls = options.controls ?? {};
  return tab(
    layout.id,
    layout.label,
    censusGroup(layout, 'GroupPageLayoutSetup', { launcher: 'Page setup' }, controls),
    censusGroup(layout, 'GroupParagraphLayout', { launcher: 'Paragraph settings' }, controls),
    censusGroup(layout, 'GroupArrange', {}, controls),
  );
}

/**
 * References: Table of Contents, Footnotes, Citations & Bibliography, Captions, Index, Table of
 * Authorities, Acronyms — the ribbon programme's unit 6, in Office's order.
 *
 * What Office draws that is not here is **Research**, between Footnotes and Citations & Bibliography,
 * which the census marks out of scope. `GUESS:` **Acronyms is last**, where the declaration puts it.
 *
 * **One dialog launcher, on Footnotes**, because Office has one there: it opens Footnote and Endnote.
 *
 * **Six of the tab's twenty-two commands are bound by the host**: Table of Contents, Add Text, Insert
 * Citation and Bibliography are dropdowns and Next Footnote a split button, over
 * `stories/ribbons/references-transitions-formulas-menus.ts`, and Style is a dropdown field. **Insert
 * Footnote is the generic button**, because Office draws it with no arrow. **No survivor.**
 */
export function wordReferencesTab(options: TabOptions = {}): TemplateResult {
  const references = entry('references');
  const controls = options.controls ?? {};
  return tab(
    references.id,
    references.label,
    censusGroup(references, 'GroupTableOfContents', {}, controls),
    censusGroup(references, 'GroupFootnotes', { launcher: 'Footnote and endnote settings' }, controls),
    censusGroup(references, 'GroupCitationsAndBibliography', {}, controls),
    censusGroup(references, 'GroupCaptions', {}, controls),
    censusGroup(references, 'GroupIndex', {}, controls),
    censusGroup(references, 'GroupTableOfAuthorities', {}, controls),
    censusGroup(references, 'GroupAcronyms', {}, controls),
  );
}

/**
 * Mailings: Create, Start Mail Merge, Write & Insert Fields, Preview Results, Finish — the ribbon
 * programme's unit 7, in Office's order, which is also the census's.
 *
 * **Six of the tab's twenty-one commands are bound by the host**: Start Mail Merge, Select Recipients,
 * Rules and Finish & Merge are dropdowns and Insert Merge Field a split button, over
 * `stories/ribbons/mailings-animations-data-menus.ts`, and Go to Record is a combo box. Highlight Merge
 * Fields and Preview Results are the generic toggle.
 *
 * **No dialog launchers**: Office puts none on its Mailings tab. **Two survivors**, Previous Record and
 * Next Record, and `dev/ribbons/census.ts` gives the reason.
 */
export function wordMailingsTab(options: TabOptions = {}): TemplateResult {
  const mailings = entry('mailings');
  const controls = options.controls ?? {};
  return tab(
    mailings.id,
    mailings.label,
    censusGroup(mailings, 'GroupEnvelopeLabelCreate', {}, controls),
    censusGroup(mailings, 'GroupMailMergeStart', {}, controls),
    censusGroup(mailings, 'GroupMailMergeWriteInsertFields', {}, controls),
    censusGroup(mailings, 'GroupMailMergePreviewResults', {}, controls),
    censusGroup(mailings, 'GroupMailMergeFinish', {}, controls),
  );
}

// ── the tabs their own units author ──────────────────────────────────────────

export function wordReviewTab(): TemplateResult {
  return placeholderTab(entry('review'));
}

export function wordViewTab(): TemplateResult {
  return placeholderTab(entry('view'));
}

export function wordOutliningTab(): TemplateResult {
  return placeholderTab(entry('outlining'));
}

export function wordPrintPreviewTab(): TemplateResult {
  return placeholderTab(entry('print-preview'));
}

export function wordBackgroundRemovalTab(): TemplateResult {
  return placeholderTab(entry('background-removal'));
}

// ── the whole ribbon ─────────────────────────────────────────────────────────

/** Which function builds which tab. Keyed by the census's own kebab ids. */
const builders: Readonly<Record<string, (options: TabOptions) => TemplateResult>> = {
  file: wordFileTab,
  home: wordHomeTab,
  insert: wordInsertTab,
  draw: wordDrawTab,
  design: wordDesignTab,
  layout: wordLayoutTab,
  references: wordReferencesTab,
  mailings: wordMailingsTab,
  review: wordReviewTab,
  view: wordViewTab,
  outlining: wordOutliningTab,
  'print-preview': wordPrintPreviewTab,
  'background-removal': wordBackgroundRemovalTab,
};

/**
 * Every tab, in Office's order.
 *
 * `includeViewTabs` is a parameter rather than a second list, so there is one ordering and one
 * place a tab can be forgotten. The shells ask for the default; the catalogue asks for everything.
 */
export function wordTabs(
  options: TabOptions & { readonly includeViewTabs?: boolean } = {},
): TemplateResult[] {
  return tabsFor(
    wordRibbonTabs,
    (declared) => {
      const build = builders[declared.id];
      if (build === undefined) {
        throw new Error(`stories/ribbons/word.ts has no builder for the '${declared.id}' tab`);
      }
      return build(options);
    },
    options,
  );
}

/**
 * The contextual tab sets the shell declares today.
 *
 * ⚠ Still `stubTab`, deliberately. Contextual sets are `TabSet*` rows in the census rather than
 * core tabs, they are **unit 11** of the ribbon programme, and the whole point of unit 0 is that
 * the nine shells look exactly as they did. Replacing these with placeholders built from a census
 * entry that does not exist yet would be inventing the thing unit 11 is for.
 */
export function wordContextualSets(): TemplateResult {
  return html`
    <mjx-contextual-tab-set label="Table Tools">
      ${stubTab('table-design', 'Design', 'Table Styles', 'table')}
      ${stubTab('table-layout', 'Layout', 'Merge Cells', 'add')}
    </mjx-contextual-tab-set>
  `;
}
