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
 * **Home** carries the commands migrated out of `stories/shell/word.stories.ts` — unchanged, in the
 * order the shell rendered them, so the assembled shell looks exactly as it did before this file
 * existed. Every other tab is `placeholderTab`: one group carrying the tab's name, at the priority
 * the census declares, holding one honest button. Units 2 onward replace them one tab at a time,
 * and each of those is a small diff against a file that already has the right shape.
 *
 * Word's Home has **six** in-scope groups in the census and this renders five: `GroupEditor` is
 * declared in `dev/ribbons/census.ts` and is not on the shell's Home today, so authoring it is
 * unit 2's work rather than something this file invents.
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
 * Home: Clipboard, Font, Paragraph, Styles, Editing.
 *
 * The group *order* is Office's and is this module's decision — the census has no column for it.
 * The labels, priorities and commands are the census's, so a group cannot quietly acquire a
 * different priority here from the one three gates read.
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
  );
}

// ── the tabs their own units author ──────────────────────────────────────────

export function wordInsertTab(): TemplateResult {
  return placeholderTab(entry('insert'));
}

export function wordDrawTab(): TemplateResult {
  return placeholderTab(entry('draw'));
}

export function wordDesignTab(): TemplateResult {
  return placeholderTab(entry('design'));
}

export function wordLayoutTab(): TemplateResult {
  return placeholderTab(entry('layout'));
}

export function wordReferencesTab(): TemplateResult {
  return placeholderTab(entry('references'));
}

export function wordMailingsTab(): TemplateResult {
  return placeholderTab(entry('mailings'));
}

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
