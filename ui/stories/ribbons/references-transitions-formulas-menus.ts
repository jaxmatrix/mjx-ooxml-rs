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
 * ## One thing here is not a menu, and one menu follows a selection
 *
 * **The Transition to This Slide gallery's items.** PowerPoint's hosts bind `<mjx-gallery>` and fill it
 * from `transitionGalleryItems()`: every transition Office shows, fifty with None, under Office's three
 * headings and in Office's order. An item's art must be static markup with no bindings inside it
 * (`<mjx-gallery-item>` captures its children, as `stories/gallery/specimens.ts` records), so each
 * picture is one of five fixed templates, chosen by index.
 *
 * **Effect Options follows the transition**, as it does in Office: Fade offers Smoothly and Through
 * Black, Push four directions, Split four orientations. Each transition carries its own entries in
 * `transitions` below, and `followTransitionEffectOptions(host)` is the listener both hosts put on the
 * gallery's `mjx-gallery-commit`. The menu holds one `<mjx-menu-section>` whose children are this file's
 * own lit render root, seeded with the starting transition's entries through `ref` and re-rendered on
 * each commit — so the host's template never manages them and a re-render of the story cannot fight the
 * listener. A transition Office gives no options (Flash, Curtains, Prestige and the rest) leaves the menu
 * empty and marks the Effect Options button `disabled`, which is what Office draws. `GUESS:` the entries
 * for many transitions, transcribed from memory of PowerPoint 2016 to 365 and not checked against a
 * build; see `transitions`.
 *
 * ## Shallow, and real
 *
 * Decision 3 of the approved plan, as on Insert: a handful of Office's own entries under each command,
 * by Office's own names. Office's function categories list every function they hold, alphabetically,
 * then *Insert Function…*; each menu here lists ten or fewer and keeps the last entry. **More Functions'
 * six submenus are flattened** to six entries, as Insert flattened Page Number's. The current choice is
 * checked where a menu has one: body text in Add Text, a transition's first option, Automatic calculation.
 *
 * **Use in Formula lists the workbook's names from `stories/formula/specimens.ts`**, the list the name
 * box already shows, so the formula bar and the ribbon name the same workbook.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no document changes,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, render, type TemplateResult } from 'lit';
import { ref } from 'lit/directives/ref.js';

import { commandSurfaceId, type RibbonApplication, type RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
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

/**
 * A transition's picture: a slide frame, and what the transition does to it.
 *
 * Five static templates, chosen by index, because an item's art may carry no bindings. Every colour is
 * a token and every size is relative to the cell. They are pictograms of the motion, not recordings of
 * it, which is what Office's are too.
 *
 * 0. an empty frame (None, Random);
 * 1. a smaller slide inside the frame (a transition that changes the slide in place: Fade, Morph, Zoom);
 * 2. the frame half covered (a transition that moves across: Push, Wipe, Cover, Page Curl, Pan);
 * 3. the frame covered from both edges (a transition that opens or turns: Split, Doors, Cube, Box);
 * 4. the frame in bars (a transition that breaks the slide up: Cut, Random Bars, Blinds, Shred).
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

/** The four edges, in the order Office's Push and Pan list them. */
const fromFourEdges: readonly string[] = ['From Bottom', 'From Left', 'From Right', 'From Top'];

/** The four edges, in the order Office's Cube, Box, Rotate and Orbit list them. */
const fromFourSides: readonly string[] = ['From Right', 'From Top', 'From Left', 'From Bottom'];

/** The eight edges and corners, in the order Office's Wipe, Uncover and Cover list them. */
const fromEightDirections: readonly string[] = [
  ...fromFourSides,
  'From Top-Right',
  'From Bottom-Right',
  'From Top-Left',
  'From Bottom-Left',
];

/** Left and right, in the order most of Exciting lists them. */
const rightOrLeft: readonly string[] = ['Right', 'Left'];

/** The slide entering from one side or the other, as Gallery, Ferris Wheel, Conveyor and Window list it. */
const fromRightOrLeft: readonly string[] = ['From Right', 'From Left'];

/**
 * **PowerPoint's transitions, every one Office shows, by Office's names, under Office's three headings
 * and in Office's order**, each with the Effect Options Office offers for it. The first option is the
 * one a new transition starts on, and an empty list is a transition whose Effect Options Office draws
 * unavailable.
 *
 * `GUESS:` two things, both from memory of PowerPoint 2016 to 365 rather than a build this project can
 * cite. **Strips** is placed last in Subtle, and **the options of the Exciting transitions** — Glitter's
 * eight and Shred's four above all — are Office's names as remembered. Fade, Push, Wipe, Split, Reveal,
 * Random Bars, Shape, Uncover, Cover, Clock, Zoom and Fly Through are the ones this file is surest of.
 */
const transitions: readonly {
  readonly value: string;
  readonly label: string;
  readonly category: string;
  readonly picture: number;
  readonly options: readonly string[];
}[] = [
  { value: 'none', label: 'None', category: 'Subtle', picture: 0, options: [] },
  { value: 'morph', label: 'Morph', category: 'Subtle', picture: 1, options: ['Objects', 'Words', 'Characters'] },
  { value: 'fade', label: 'Fade', category: 'Subtle', picture: 1, options: ['Smoothly', 'Through Black'] },
  { value: 'push', label: 'Push', category: 'Subtle', picture: 2, options: fromFourEdges },
  { value: 'wipe', label: 'Wipe', category: 'Subtle', picture: 2, options: fromEightDirections },
  { value: 'split', label: 'Split', category: 'Subtle', picture: 3, options: ['Vertical Out', 'Vertical In', 'Horizontal Out', 'Horizontal In'] },
  {
    value: 'reveal',
    label: 'Reveal',
    category: 'Subtle',
    picture: 2,
    options: ['Smoothly From Right', 'Smoothly From Left', 'Through Black From Right', 'Through Black From Left'],
  },
  { value: 'cut', label: 'Cut', category: 'Subtle', picture: 4, options: ['Cut', 'Through Black'] },
  { value: 'random-bars', label: 'Random Bars', category: 'Subtle', picture: 4, options: ['Vertical', 'Horizontal'] },
  { value: 'shape', label: 'Shape', category: 'Subtle', picture: 1, options: ['Circle', 'Diamond', 'Plus', 'In', 'Out'] },
  { value: 'uncover', label: 'Uncover', category: 'Subtle', picture: 2, options: fromEightDirections },
  { value: 'cover', label: 'Cover', category: 'Subtle', picture: 2, options: fromEightDirections },
  { value: 'flash', label: 'Flash', category: 'Subtle', picture: 1, options: [] },
  { value: 'strips', label: 'Strips', category: 'Subtle', picture: 4, options: ['Right-Down', 'Left-Down', 'Right-Up', 'Left-Up'] },
  { value: 'fall-over', label: 'Fall Over', category: 'Exciting', picture: 2, options: ['Left', 'Right'] },
  { value: 'drape', label: 'Drape', category: 'Exciting', picture: 3, options: ['Left', 'Right'] },
  { value: 'curtains', label: 'Curtains', category: 'Exciting', picture: 3, options: [] },
  { value: 'wind', label: 'Wind', category: 'Exciting', picture: 2, options: rightOrLeft },
  { value: 'prestige', label: 'Prestige', category: 'Exciting', picture: 1, options: [] },
  { value: 'fracture', label: 'Fracture', category: 'Exciting', picture: 4, options: [] },
  { value: 'crush', label: 'Crush', category: 'Exciting', picture: 1, options: [] },
  { value: 'peel-off', label: 'Peel Off', category: 'Exciting', picture: 2, options: ['Left', 'Right'] },
  { value: 'page-curl', label: 'Page Curl', category: 'Exciting', picture: 2, options: ['Double Left', 'Double Right', 'Single Left', 'Single Right'] },
  { value: 'airplane', label: 'Airplane', category: 'Exciting', picture: 2, options: rightOrLeft },
  { value: 'origami', label: 'Origami', category: 'Exciting', picture: 1, options: rightOrLeft },
  { value: 'dissolve', label: 'Dissolve', category: 'Exciting', picture: 4, options: [] },
  { value: 'checkerboard', label: 'Checkerboard', category: 'Exciting', picture: 4, options: ['From Left', 'From Top'] },
  { value: 'blinds', label: 'Blinds', category: 'Exciting', picture: 4, options: ['Vertical', 'Horizontal'] },
  { value: 'clock', label: 'Clock', category: 'Exciting', picture: 1, options: ['Clockwise', 'Counterclockwise', 'Wedge'] },
  {
    value: 'ripple',
    label: 'Ripple',
    category: 'Exciting',
    picture: 1,
    options: ['Centre', 'From Top-Left', 'From Top-Right', 'From Bottom-Left', 'From Bottom-Right'],
  },
  { value: 'honeycomb', label: 'Honeycomb', category: 'Exciting', picture: 4, options: [] },
  {
    value: 'glitter',
    label: 'Glitter',
    category: 'Exciting',
    picture: 4,
    options: [
      'Hexagons from Left',
      'Hexagons from Top',
      'Hexagons from Right',
      'Hexagons from Bottom',
      'Diamonds from Left',
      'Diamonds from Top',
      'Diamonds from Right',
      'Diamonds from Bottom',
    ],
  },
  { value: 'vortex', label: 'Vortex', category: 'Exciting', picture: 1, options: ['From Left', 'From Top', 'From Right', 'From Bottom'] },
  { value: 'shred', label: 'Shred', category: 'Exciting', picture: 4, options: ['Strips In', 'Strips Out', 'Particles In', 'Particles Out'] },
  { value: 'switch', label: 'Switch', category: 'Exciting', picture: 2, options: rightOrLeft },
  { value: 'flip', label: 'Flip', category: 'Exciting', picture: 2, options: rightOrLeft },
  { value: 'gallery', label: 'Gallery', category: 'Exciting', picture: 2, options: fromRightOrLeft },
  { value: 'cube', label: 'Cube', category: 'Exciting', picture: 3, options: fromFourSides },
  { value: 'doors', label: 'Doors', category: 'Exciting', picture: 3, options: ['Vertical', 'Horizontal'] },
  { value: 'box', label: 'Box', category: 'Exciting', picture: 3, options: fromFourSides },
  { value: 'comb', label: 'Comb', category: 'Exciting', picture: 4, options: ['Horizontal', 'Vertical'] },
  { value: 'zoom', label: 'Zoom', category: 'Exciting', picture: 1, options: ['In', 'Out', 'Zoom and Rotate'] },
  { value: 'random', label: 'Random', category: 'Exciting', picture: 0, options: [] },
  { value: 'pan', label: 'Pan', category: 'Dynamic Content', picture: 2, options: fromFourEdges },
  { value: 'ferris-wheel', label: 'Ferris Wheel', category: 'Dynamic Content', picture: 4, options: fromRightOrLeft },
  { value: 'conveyor', label: 'Conveyor', category: 'Dynamic Content', picture: 2, options: fromRightOrLeft },
  { value: 'rotate', label: 'Rotate', category: 'Dynamic Content', picture: 3, options: fromFourSides },
  { value: 'window', label: 'Window', category: 'Dynamic Content', picture: 3, options: fromRightOrLeft },
  { value: 'orbit', label: 'Orbit', category: 'Dynamic Content', picture: 1, options: fromFourSides },
  { value: 'fly-through', label: 'Fly Through', category: 'Dynamic Content', picture: 1, options: ['In', 'In with Bounce', 'Out', 'Out with Bounce'] },
];

/** The transition PowerPoint's hosts start the gallery on. Fade, because a new deck's None offers no options. */
export const startingTransition = 'fade';

/** The Transition to This Slide gallery's items, for PowerPoint's Transitions tab. A host starts on `startingTransition`. */
export function transitionGalleryItems(): TemplateResult[] {
  return transitions.map(
    (transition) => html`<mjx-gallery-item value=${transition.value} label=${transition.label} category=${transition.category}
      >${transitionPictures[transition.picture] ?? transitionPictures[0]}</mjx-gallery-item
    >`,
  );
}

/** One transition's Effect Options, its first option checked. Empty for a transition Office gives none. */
function effectOptionsEntries(value: string): TemplateResult[] {
  const options = transitions.find((transition) => transition.value === value)?.options ?? [];
  return options.map((option, index) => choice(option, index === 0));
}

/** The command whose menu follows the gallery. */
const effectOptionsCommand = 'powerpoint.transitions.transition-styles.effect-options';

/**
 * **Show one transition's options in Effect Options, on one host's page**, and mark the button
 * unavailable when there are none.
 *
 * The section is this file's own lit render root (see this file's header), so `render` here never
 * touches nodes a host template owns. The button is found by the `data-opens` its host binds, which
 * `tests/ribbons.test.ts` already requires to be this command's surface id.
 */
function showEffectOptions(host: RibbonSurfaceHost, section: HTMLElement, value: string): void {
  const entries = effectOptionsEntries(value);
  render(html`${entries}`, section);
  const trigger = document.querySelector(`[data-opens="${commandSurfaceId(host, effectOptionsCommand)}"]`);
  trigger?.toggleAttribute('disabled', entries.length === 0);
}

/** The section a host's Effect Options menu holds, once it is in the page. */
function effectOptionsSection(host: RibbonSurfaceHost): HTMLElement | null {
  return document.querySelector(`#${commandSurfaceId(host, effectOptionsCommand)} > mjx-menu-section`);
}

/** One listener and one seed per host, so lit sees the same function on every render and binds it once. */
const followers = new Map<RibbonSurfaceHost, (event: Event) => void>();
const seeds = new Map<RibbonSurfaceHost, (element: Element | undefined) => void>();

/**
 * **The listener a host puts on the Transition to This Slide gallery's `mjx-gallery-commit`**, so Effect
 * Options offers the committed transition's entries.
 *
 * Commit rather than preview: a preview is a question, and Office's Effect Options changes only once a
 * transition is applied.
 */
export function followTransitionEffectOptions(host: RibbonSurfaceHost): (event: Event) => void {
  let follower = followers.get(host);
  if (follower === undefined) {
    follower = (event: Event): void => {
      const value = (event as CustomEvent<{ readonly value?: string }>).detail.value;
      const section = effectOptionsSection(host);
      if (value === undefined || section === null) return;
      showEffectOptions(host, section, value);
    };
    followers.set(host, follower);
  }
  return follower;
}

/** Fill the section with the starting transition's entries the first time it is rendered. */
function seedEffectOptions(host: RibbonSurfaceHost): (element: Element | undefined) => void {
  let seed = seeds.get(host);
  if (seed === undefined) {
    seed = (element: Element | undefined): void => {
      if (element instanceof HTMLElement) showEffectOptions(host, element, startingTransition);
    };
    seeds.set(host, seed);
  }
  return seed;
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

/**
 * One menu, whose entries follow the gallery: see `followTransitionEffectOptions`. The gallery, Sound,
 * Duration and the advance time are bound by the hosts.
 */
function powerpointTransitionsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(
      host,
      'powerpoint.transitions.transition-styles.effect-options',
      'Effect Options',
      html`<mjx-menu-section ${ref(seedEffectOptions(host))}></mjx-menu-section>`,
    )}
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
