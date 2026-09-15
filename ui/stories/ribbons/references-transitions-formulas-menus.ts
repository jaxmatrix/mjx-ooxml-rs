/**
 * **The menus and gallery items the References, Transitions and Formulas tabs open**, written once and
 * rendered by both hosts.
 *
 * The ribbon programme's unit 6: Word's References, PowerPoint's Transitions, Excel's Formulas. The
 * pattern is `stories/ribbons/insert-menus.ts`'s, for its reasons. A binding lives in its host. The menu
 * it opens is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`.
 * A host renders `referencesTransitionsFormulasMenus(application, host)` once beside its ribbon. Every
 * `commandMenu(host, '…'` call below spells its command id literally, so `tests/ribbons.test.ts` can
 * read it.
 *
 * ## One thing here is not a menu
 *
 * **The Transition to This Slide gallery's items.** PowerPoint's hosts bind `<mjx-gallery>` and fill it
 * from `transitionGalleryItems()`. An item's art must be static markup with no bindings inside it
 * (`<mjx-gallery-item>` captures its children, as `stories/gallery/specimens.ts` records), so each
 * picture is one of five fixed templates, chosen by index.
 *
 * ## Shallow, and real
 *
 * Decision 3 of the approved plan, as on Insert: a handful of Office's own entries under each command,
 * by Office's own names. Office's function categories list every function they hold, alphabetically,
 * then *Insert Function…*; each menu here lists ten or fewer and keeps the last entry. **More Functions'
 * six submenus are flattened** to six entries, as Insert flattened Page Number's. The current choice is
 * checked where a menu has one: body text in Add Text, Fade's Smoothly, Automatic calculation.
 *
 * **Use in Formula lists the workbook's names from `stories/formula/specimens.ts`**, the list the name
 * box already shows, so the formula bar and the ribbon name the same workbook.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no document changes,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { workbookNames } from '../formula/specimens.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string, shortcut?: string): TemplateResult {
  return html`<mjx-menu-item label=${label} shortcut=${shortcut ?? ''}></mjx-menu-item>`;
}

/** One entry of a set whose current member is checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

// ── Word's References ────────────────────────────────────────────────────────

/** Table of Contents: the three built-in tables, then the three commands under them. */
function tableOfContentsEntries(): TemplateResult[] {
  return [
    section('Built-In', item('Automatic Table 1'), item('Automatic Table 2'), item('Manual Table')),
    separator(),
    item('Custom Table of Contents…'),
    item('Remove Table of Contents'),
    item('Save Selection to Table of Contents Gallery…'),
  ];
}

/** Add Text: the level the paragraph takes in the table. Body text is not shown, which is checked. */
function addTextEntries(): TemplateResult[] {
  return [
    choice('Do Not Show in Table of Contents', true),
    choice('Level 1'),
    choice('Level 2'),
    choice('Level 3'),
  ];
}

/** Next Footnote's arrow: the four directions through the notes. */
function nextFootnoteEntries(): TemplateResult[] {
  return [item('Next Footnote'), item('Previous Footnote'), item('Next Endnote'), item('Previous Endnote')];
}

/**
 * Insert Citation: the two ways to add a source.
 *
 * Office lists the document's existing sources above them. A new document has none, and inventing an
 * author would be a claim about somebody's bibliography.
 */
function insertCitationEntries(): TemplateResult[] {
  return [item('Add New Source…'), item('Add New Placeholder…')];
}

/** Bibliography: the three built-in bibliographies, then the two commands under them. */
function bibliographyEntries(): TemplateResult[] {
  return [
    section('Built-In', item('Bibliography'), item('References'), item('Works Cited')),
    separator(),
    item('Insert Bibliography'),
    item('Save Selection to Bibliography Gallery…'),
  ];
}

// ── PowerPoint's Transitions ─────────────────────────────────────────────────

/** Effect Options, for Fade, which the hosts start the gallery on. */
function effectOptionsEntries(): TemplateResult[] {
  return [section('Effect Options', choice('Smoothly', true), choice('Through Black'))];
}

/**
 * A transition's picture: a slide frame, and what the transition does to it.
 *
 * Five static templates, chosen by index, because an item's art may carry no bindings. Every colour is
 * a token and every size is relative to the cell. They are pictograms of the motion, not recordings of
 * it, which is what Office's are too.
 *
 * 0. an empty frame (None);
 * 1. a smaller slide inside the frame (Fade, Morph, Vortex);
 * 2. the frame half covered (Push, Wipe, Reveal, Page Curl, Pan);
 * 3. the frame covered from both edges (Split, Curtains, Cube);
 * 4. the frame in bars (Cut, Ferris Wheel).
 */
const transitionPictures: readonly TemplateResult[] = [
  html`<span style="display:block;inline-size:100%;aspect-ratio:4/3;border:1px solid var(--theme-border)"></span>`,
  html`<span style="display:grid;place-items:center;inline-size:100%;aspect-ratio:4/3;border:1px solid var(--theme-border)">
    <span style="display:block;inline-size:60%;block-size:60%;background:var(--theme-accent-surface);border:1px solid var(--theme-accent)"></span>
  </span>`,
  html`<span style="display:grid;grid-template-columns:1fr 1fr;inline-size:100%;aspect-ratio:4/3;border:1px solid var(--theme-border)">
    <span style="background:var(--theme-accent)"></span><span></span>
  </span>`,
  html`<span style="display:grid;grid-template-columns:1fr 2fr 1fr;inline-size:100%;aspect-ratio:4/3;border:1px solid var(--theme-border)">
    <span style="background:var(--theme-accent)"></span><span></span><span style="background:var(--theme-accent)"></span>
  </span>`,
  html`<span style="display:grid;grid-template-rows:repeat(4,1fr);inline-size:100%;aspect-ratio:4/3;border:1px solid var(--theme-border)">
    <span style="background:var(--theme-accent)"></span><span></span><span style="background:var(--theme-accent)"></span><span></span>
  </span>`,
];

/** PowerPoint's transitions, by Office's names, under Office's three headings. */
const transitions: readonly { readonly value: string; readonly label: string; readonly category: string; readonly picture: number }[] = [
  { value: 'none', label: 'None', category: 'Subtle', picture: 0 },
  { value: 'morph', label: 'Morph', category: 'Subtle', picture: 1 },
  { value: 'fade', label: 'Fade', category: 'Subtle', picture: 1 },
  { value: 'push', label: 'Push', category: 'Subtle', picture: 2 },
  { value: 'wipe', label: 'Wipe', category: 'Subtle', picture: 2 },
  { value: 'split', label: 'Split', category: 'Subtle', picture: 3 },
  { value: 'reveal', label: 'Reveal', category: 'Subtle', picture: 2 },
  { value: 'cut', label: 'Cut', category: 'Subtle', picture: 4 },
  { value: 'curtains', label: 'Curtains', category: 'Exciting', picture: 3 },
  { value: 'page-curl', label: 'Page Curl', category: 'Exciting', picture: 2 },
  { value: 'vortex', label: 'Vortex', category: 'Exciting', picture: 1 },
  { value: 'cube', label: 'Cube', category: 'Exciting', picture: 3 },
  { value: 'pan', label: 'Pan', category: 'Dynamic Content', picture: 2 },
  { value: 'ferris-wheel', label: 'Ferris Wheel', category: 'Dynamic Content', picture: 4 },
];

/** The Transition to This Slide gallery's items, for PowerPoint's Transitions tab. A host starts on `fade`. */
export function transitionGalleryItems(): TemplateResult[] {
  return transitions.map(
    (transition) => html`<mjx-gallery-item value=${transition.value} label=${transition.label} category=${transition.category}
      >${transitionPictures[transition.picture] ?? transitionPictures[0]}</mjx-gallery-item
    >`,
  );
}

// ── Excel's Formulas ─────────────────────────────────────────────────────────

/** A function category: some of its functions, then Insert Function, which every category ends with. */
function functionEntries(...names: string[]): TemplateResult[] {
  return [...names.map((name) => item(name)), separator(), item('Insert Function…', 'Shift+F3')];
}

/** AutoSum's arrow: the five quick functions, and the dialog. */
function autoSumEntries(): TemplateResult[] {
  return [item('Sum'), item('Average'), item('Count Numbers'), item('Max'), item('Min'), separator(), item('More Functions…')];
}

/** More Functions: Office's six further categories, each a submenu in Office, flattened here. */
function moreFunctionsEntries(): TemplateResult[] {
  return [item('Statistical'), item('Engineering'), item('Cube'), item('Information'), item('Compatibility'), item('Web')];
}

/** Define Name's arrow. */
function defineNameEntries(): TemplateResult[] {
  return [item('Define Name…'), item('Apply Names…')];
}

/** Use in Formula: the workbook's defined names, then the dialog that pastes them all. */
function useInFormulaEntries(): TemplateResult[] {
  return [
    ...workbookNames.filter((entry) => entry.kind === 'name').map((entry) => item(entry.name)),
    separator(),
    item('Paste Names…', 'F3'),
  ];
}

/** Remove Arrows' arrow. */
function removeArrowsEntries(): TemplateResult[] {
  return [item('Remove Arrows'), item('Remove Precedent Arrows'), item('Remove Dependent Arrows')];
}

/** Error Checking's arrow. Circular References is a submenu in Office, flattened here. */
function errorCheckingEntries(): TemplateResult[] {
  return [item('Error Checking…'), item('Trace Error'), item('Circular References')];
}

/** Calculation Options: the three modes, Automatic a new workbook's. */
function calculationOptionsEntries(): TemplateResult[] {
  return [choice('Automatic', true), choice('Automatic Except for Data Tables'), choice('Manual')];
}

// ── Word ─────────────────────────────────────────────────────────────────────

function wordReferencesMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.references.table-of-contents.table-of-contents', 'Table of Contents', ...tableOfContentsEntries())}
    ${commandMenu(host, 'word.references.table-of-contents.add-text', 'Add Text', ...addTextEntries())}
    ${commandMenu(host, 'word.references.footnotes.next-footnote', 'Next Footnote', ...nextFootnoteEntries())}
    ${commandMenu(host, 'word.references.citations-bibliography.insert-citation', 'Insert Citation', ...insertCitationEntries())}
    ${commandMenu(host, 'word.references.citations-bibliography.bibliography', 'Bibliography', ...bibliographyEntries())}
  `;
}

// ── PowerPoint ───────────────────────────────────────────────────────────────

/** One menu. The gallery, Sound, Duration and the advance time are bound by the hosts. */
function powerpointTransitionsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.transitions.transition-styles.effect-options', 'Effect Options', ...effectOptionsEntries())}
  `;
}

// ── Excel ────────────────────────────────────────────────────────────────────

function excelFormulasMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.formulas.function-library.autosum', 'AutoSum', ...autoSumEntries())}
    ${commandMenu(host, 'excel.formulas.function-library.recently-used', 'Recently Used', ...functionEntries('SUM', 'AVERAGE', 'IF', 'HYPERLINK', 'COUNT', 'MAX', 'SIN', 'SUMIF', 'PMT', 'STDEV'))}
    ${commandMenu(host, 'excel.formulas.function-library.financial', 'Financial', ...functionEntries('ACCRINT', 'DB', 'FV', 'IPMT', 'IRR', 'NPER', 'NPV', 'PMT', 'PV', 'RATE'))}
    ${commandMenu(host, 'excel.formulas.function-library.logical', 'Logical', ...functionEntries('AND', 'FALSE', 'IF', 'IFERROR', 'IFS', 'NOT', 'OR', 'SWITCH', 'TRUE', 'XOR'))}
    ${commandMenu(host, 'excel.formulas.function-library.text', 'Text', ...functionEntries('CONCAT', 'FIND', 'LEFT', 'LEN', 'MID', 'RIGHT', 'SUBSTITUTE', 'TEXT', 'TEXTJOIN', 'TRIM'))}
    ${commandMenu(host, 'excel.formulas.function-library.date-time', 'Date & Time', ...functionEntries('DATE', 'DAY', 'EDATE', 'EOMONTH', 'MONTH', 'NETWORKDAYS', 'NOW', 'TODAY', 'WEEKDAY', 'YEAR'))}
    ${commandMenu(host, 'excel.formulas.function-library.lookup-reference', 'Lookup & Reference', ...functionEntries('CHOOSE', 'FILTER', 'INDEX', 'INDIRECT', 'MATCH', 'OFFSET', 'SORT', 'UNIQUE', 'VLOOKUP', 'XLOOKUP'))}
    ${commandMenu(host, 'excel.formulas.function-library.math-trig', 'Math & Trig', ...functionEntries('ABS', 'INT', 'MOD', 'PRODUCT', 'RAND', 'ROUND', 'SQRT', 'SUM', 'SUMIF', 'SUMPRODUCT'))}
    ${commandMenu(host, 'excel.formulas.function-library.more-functions', 'More Functions', ...moreFunctionsEntries())}
    ${commandMenu(host, 'excel.formulas.named-cells.define-name', 'Define Name', ...defineNameEntries())}
    ${commandMenu(host, 'excel.formulas.named-cells.use-in-formula', 'Use in Formula', ...useInFormulaEntries())}
    ${commandMenu(host, 'excel.formulas.formula-auditing.remove-arrows', 'Remove Arrows', ...removeArrowsEntries())}
    ${commandMenu(host, 'excel.formulas.formula-auditing.error-checking', 'Error Checking', ...errorCheckingEntries())}
    ${commandMenu(host, 'excel.formulas.calculation.calculation-options', 'Calculation Options', ...calculationOptionsEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

const menusByApplication: Readonly<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordReferencesMenus,
  powerpoint: powerpointTransitionsMenus,
  excel: excelFormulasMenus,
};

/**
 * Every menu one application's unit-6 tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `insertMenus` is.
 */
export function referencesTransitionsFormulasMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  return menusByApplication[application](host);
}
