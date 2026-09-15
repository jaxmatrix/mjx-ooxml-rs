/**
 * **The menus and gallery items the Mailings, Animations and Data tabs open**, written once and rendered
 * by both hosts.
 *
 * The ribbon programme's unit 7: Word's Mailings, PowerPoint's Animations, Excel's Data. The pattern is
 * `stories/ribbons/insert-menus.ts`'s, for its reasons. A binding lives in its host. The menu it opens
 * is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host
 * renders `mailingsAnimationsDataMenus(application, host)` once beside its ribbon. Every
 * `commandMenu(host, '…'` call below spells its command id literally, so `tests/ribbons.test.ts` can
 * read it.
 *
 * ## Two things here are not menus
 *
 * **The Animation Styles gallery's items** and **the Data Types gallery's items**. The hosts bind
 * `<mjx-gallery>` and fill it from `animationGalleryItems()` and `dataTypeGalleryItems()`. An item's art
 * must be static markup with no bindings inside it (`<mjx-gallery-item>` captures its children, as
 * `stories/gallery/specimens.ts` records), so each animation picture is one of five fixed templates,
 * chosen by index, and each data type picture is one fixed `<mjx-icon>`.
 *
 * **The animation effects are written once.** The gallery and Add Animation's menu list the same
 * effects under the same headings, from `animationEffects` — every effect Office's gallery shows, fifty-two
 * with None — and the gallery's footer and the menu's foot name the same five dialogs, so the two cannot
 * name different sets. **Effect Options is Fly In's**, the effect the hosts start the gallery on; unlike
 * Transitions' it does not follow the selection, and `stories/ribbons/powerpoint.stories.ts` says so.
 *
 * ## Shallow, and real
 *
 * Decision 3 of the approved plan, as on Insert: a handful of Office's own entries under each command,
 * by Office's own names. **Submenus are flattened** — Add Animation's *More … Effects*, From Other
 * Sources' wizards — as Insert flattened Page Number's. The current choice is checked where a menu has
 * one: Normal Word Document, AutoPreview, Fly In's From Bottom and As One Object.
 *
 * **Insert Merge Field lists the columns of Word's own new recipient list** (Title, First_Name and the
 * rest), which is what a person who chose *Type a New List* sees. **Trigger lists the slide's shapes by
 * PowerPoint's default names** (Title 1, Content Placeholder 2), which name no one's deck.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no document changes,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
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

/** One entry that is a setting on its own, checked or not. */
function setting(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="checkbox" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

// ── Word's Mailings ──────────────────────────────────────────────────────────

/** Start Mail Merge: the document types, a new document's checked, then the wizard. */
function startMailMergeEntries(): TemplateResult[] {
  return [
    choice('Letters'),
    choice('E-mail Messages'),
    choice('Envelopes…'),
    choice('Labels…'),
    choice('Directory'),
    choice('Normal Word Document', true),
    separator(),
    item('Step-by-Step Mail Merge Wizard…'),
  ];
}

/** Select Recipients: the three sources. */
function selectRecipientsEntries(): TemplateResult[] {
  return [item('Type a New List…'), item('Use an Existing List…'), item('Choose from Outlook Contacts…')];
}

/** Insert Merge Field's arrow: the columns of Word's own new recipient list. */
function insertMergeFieldEntries(): TemplateResult[] {
  return [
    item('Title'),
    item('First_Name'),
    item('Last_Name'),
    item('Company_Name'),
    item('Address_Line_1'),
    item('Address_Line_2'),
    item('City'),
    item('State'),
    item('ZIP_Code'),
    item('Country_or_Region'),
  ];
}

/** Rules: Word's nine merge rules, in Office's order. */
function rulesEntries(): TemplateResult[] {
  return [
    item('Ask…'),
    item('Fill-in…'),
    item('If…Then…Else…'),
    item('Merge Record #'),
    item('Merge Sequence #'),
    item('Next Record'),
    item('Next Record If…'),
    item('Set Bookmark…'),
    item('Skip Record If…'),
  ];
}

/** Finish & Merge: the three ways to finish. */
function finishMergeEntries(): TemplateResult[] {
  return [item('Edit Individual Documents…'), item('Print Documents…'), item('Send Email Messages…')];
}

// ── PowerPoint's Animations ──────────────────────────────────────────────────

/** Preview's arrow: the command, and the setting that previews an effect as it is applied. */
function previewEntries(): TemplateResult[] {
  return [item('Preview'), setting('AutoPreview', true)];
}

/** Effect Options, for Fly In, which the hosts start the gallery on. */
function effectOptionsEntries(): TemplateResult[] {
  return [
    section(
      'Direction',
      choice('From Bottom', true),
      choice('From Bottom-Left'),
      choice('From Left'),
      choice('From Top-Left'),
      choice('From Top'),
      choice('From Top-Right'),
      choice('From Right'),
      choice('From Bottom-Right'),
    ),
    section('Sequence', choice('As One Object', true), choice('All at Once'), choice('By Paragraph')),
  ];
}

/**
 * An effect's picture: a star for an effect on the object, a path for a motion path, an empty frame for
 * None.
 *
 * Five static templates, chosen by index, because an item's art may carry no bindings. Every colour is
 * a token and every size is relative to the cell. Office colours its stars by category (green, yellow,
 * red); the catalogue has no status tokens for that, so the category is carried by the heading and by
 * the star's fill: solid for Entrance, on a soft ground for Emphasis, outlined for Exit. `GUESS:` that
 * reading.
 *
 * 0. an empty frame (None);
 * 1. a solid star (Entrance);
 * 2. a star on a soft ground (Emphasis);
 * 3. an outlined star (Exit);
 * 4. a line with a dot at its start (Motion Paths).
 */
const effectPictures: readonly TemplateResult[] = [
  html`<span style="display:block;inline-size:100%;aspect-ratio:1;border:1px solid var(--theme-border)"></span>`,
  html`<span style="display:grid;place-items:center;inline-size:100%;aspect-ratio:1">
    <span style="display:block;inline-size:70%;aspect-ratio:1;background:var(--theme-accent);clip-path:polygon(50% 0%,61% 35%,98% 35%,68% 57%,79% 91%,50% 70%,21% 91%,32% 57%,2% 35%,39% 35%)"></span>
  </span>`,
  html`<span style="display:grid;place-items:center;inline-size:100%;aspect-ratio:1;background:var(--theme-accent-soft)">
    <span style="display:block;inline-size:70%;aspect-ratio:1;background:var(--theme-accent-pressed);clip-path:polygon(50% 0%,61% 35%,98% 35%,68% 57%,79% 91%,50% 70%,21% 91%,32% 57%,2% 35%,39% 35%)"></span>
  </span>`,
  html`<span style="display:grid;place-items:center;inline-size:100%;aspect-ratio:1">
    <span style="display:grid;place-items:center;inline-size:70%;aspect-ratio:1;background:var(--theme-accent);clip-path:polygon(50% 0%,61% 35%,98% 35%,68% 57%,79% 91%,50% 70%,21% 91%,32% 57%,2% 35%,39% 35%)">
      <span style="display:block;inline-size:60%;aspect-ratio:1;background:var(--theme-accent-surface);clip-path:polygon(50% 0%,61% 35%,98% 35%,68% 57%,79% 91%,50% 70%,21% 91%,32% 57%,2% 35%,39% 35%)"></span>
    </span>
  </span>`,
  html`<span style="display:grid;grid-template-columns:auto 1fr;align-items:center;inline-size:100%;aspect-ratio:1">
    <span style="display:block;inline-size:0.4em;aspect-ratio:1;border-radius:50%;background:var(--theme-accent)"></span>
    <span style="display:block;block-size:0;border-block-start:1px dashed var(--theme-accent)"></span>
  </span>`,
];

/**
 * **PowerPoint's effects, every one Office's Animation Styles gallery shows**, by Office's names, under
 * Office's five headings and in Office's order. `value` is unique although a label is not: Fade, Split,
 * Wipe, Shape, Wheel, Random Bars, Zoom, Swivel and Bounce are both an entrance and an exit.
 *
 * Office greys the text-only emphasis effects (Font Colour to Wave) for a shape with no text; they are
 * listed, because Office lists them. `GUESS:` the British spellings (*Colour Pulse*, *Object Colour*),
 * which this catalogue uses throughout, and Office's order within Emphasis.
 */
const animationEffects: readonly {
  readonly value: string;
  readonly label: string;
  readonly category: 'None' | 'Entrance' | 'Emphasis' | 'Exit' | 'Motion Paths';
  readonly picture: number;
}[] = [
  { value: 'none', label: 'None', category: 'None', picture: 0 },
  { value: 'appear', label: 'Appear', category: 'Entrance', picture: 1 },
  { value: 'fade', label: 'Fade', category: 'Entrance', picture: 1 },
  { value: 'fly-in', label: 'Fly In', category: 'Entrance', picture: 1 },
  { value: 'float-in', label: 'Float In', category: 'Entrance', picture: 1 },
  { value: 'split', label: 'Split', category: 'Entrance', picture: 1 },
  { value: 'wipe', label: 'Wipe', category: 'Entrance', picture: 1 },
  { value: 'shape', label: 'Shape', category: 'Entrance', picture: 1 },
  { value: 'wheel', label: 'Wheel', category: 'Entrance', picture: 1 },
  { value: 'random-bars', label: 'Random Bars', category: 'Entrance', picture: 1 },
  { value: 'grow-turn', label: 'Grow & Turn', category: 'Entrance', picture: 1 },
  { value: 'zoom', label: 'Zoom', category: 'Entrance', picture: 1 },
  { value: 'swivel', label: 'Swivel', category: 'Entrance', picture: 1 },
  { value: 'bounce', label: 'Bounce', category: 'Entrance', picture: 1 },
  { value: 'pulse', label: 'Pulse', category: 'Emphasis', picture: 2 },
  { value: 'colour-pulse', label: 'Colour Pulse', category: 'Emphasis', picture: 2 },
  { value: 'teeter', label: 'Teeter', category: 'Emphasis', picture: 2 },
  { value: 'spin', label: 'Spin', category: 'Emphasis', picture: 2 },
  { value: 'grow-shrink', label: 'Grow/Shrink', category: 'Emphasis', picture: 2 },
  { value: 'desaturate', label: 'Desaturate', category: 'Emphasis', picture: 2 },
  { value: 'darken', label: 'Darken', category: 'Emphasis', picture: 2 },
  { value: 'lighten', label: 'Lighten', category: 'Emphasis', picture: 2 },
  { value: 'transparency', label: 'Transparency', category: 'Emphasis', picture: 2 },
  { value: 'object-colour', label: 'Object Colour', category: 'Emphasis', picture: 2 },
  { value: 'complementary-colour', label: 'Complementary Colour', category: 'Emphasis', picture: 2 },
  { value: 'line-colour', label: 'Line Colour', category: 'Emphasis', picture: 2 },
  { value: 'fill-colour', label: 'Fill Colour', category: 'Emphasis', picture: 2 },
  { value: 'brush-colour', label: 'Brush Colour', category: 'Emphasis', picture: 2 },
  { value: 'font-colour', label: 'Font Colour', category: 'Emphasis', picture: 2 },
  { value: 'underline', label: 'Underline', category: 'Emphasis', picture: 2 },
  { value: 'bold-flash', label: 'Bold Flash', category: 'Emphasis', picture: 2 },
  { value: 'bold-reveal', label: 'Bold Reveal', category: 'Emphasis', picture: 2 },
  { value: 'wave', label: 'Wave', category: 'Emphasis', picture: 2 },
  { value: 'disappear', label: 'Disappear', category: 'Exit', picture: 3 },
  { value: 'fade-exit', label: 'Fade', category: 'Exit', picture: 3 },
  { value: 'fly-out', label: 'Fly Out', category: 'Exit', picture: 3 },
  { value: 'float-out', label: 'Float Out', category: 'Exit', picture: 3 },
  { value: 'split-exit', label: 'Split', category: 'Exit', picture: 3 },
  { value: 'wipe-exit', label: 'Wipe', category: 'Exit', picture: 3 },
  { value: 'shape-exit', label: 'Shape', category: 'Exit', picture: 3 },
  { value: 'wheel-exit', label: 'Wheel', category: 'Exit', picture: 3 },
  { value: 'random-bars-exit', label: 'Random Bars', category: 'Exit', picture: 3 },
  { value: 'shrink-turn', label: 'Shrink & Turn', category: 'Exit', picture: 3 },
  { value: 'zoom-exit', label: 'Zoom', category: 'Exit', picture: 3 },
  { value: 'swivel-exit', label: 'Swivel', category: 'Exit', picture: 3 },
  { value: 'bounce-exit', label: 'Bounce', category: 'Exit', picture: 3 },
  { value: 'lines', label: 'Lines', category: 'Motion Paths', picture: 4 },
  { value: 'arcs', label: 'Arcs', category: 'Motion Paths', picture: 4 },
  { value: 'turns', label: 'Turns', category: 'Motion Paths', picture: 4 },
  { value: 'shapes', label: 'Shapes', category: 'Motion Paths', picture: 4 },
  { value: 'loops', label: 'Loops', category: 'Motion Paths', picture: 4 },
  { value: 'custom-path', label: 'Custom Path', category: 'Motion Paths', picture: 4 },
];

/** The effect PowerPoint's hosts start the gallery on. Fly In, because a new shape's None offers no options. */
export const startingAnimation = 'fly-in';

/** The Animation Styles gallery's items, for PowerPoint's Animations tab. A host starts on `startingAnimation`. */
export function animationGalleryItems(): TemplateResult[] {
  return animationEffects.map(
    (effect) => html`<mjx-gallery-item value=${effect.value} label=${effect.label} category=${effect.category}
      >${effectPictures[effect.picture] ?? effectPictures[0]}</mjx-gallery-item
    >`,
  );
}

/**
 * **The Animation Styles gallery's footer**: Office's four *More … Effects* dialogs and OLE Action Verbs,
 * which Office greys unless an embedded object is selected. Written once, because both hosts slot it into
 * the gallery and Add Animation lists the same four. The dialogs are loop 2's, so the buttons open nothing.
 */
export function animationGalleryFooter(): TemplateResult {
  return html`
    <mjx-button slot="footer" label="More Entrance Effects…"></mjx-button>
    <mjx-button slot="footer" label="More Emphasis Effects…"></mjx-button>
    <mjx-button slot="footer" label="More Exit Effects…"></mjx-button>
    <mjx-button slot="footer" label="More Motion Paths…"></mjx-button>
    <mjx-button slot="footer" label="OLE Action Verbs…" disabled></mjx-button>
  `;
}

/** The effects of one heading, as menu entries. */
function effectEntries(category: 'Entrance' | 'Emphasis' | 'Exit' | 'Motion Paths'): TemplateResult {
  return section(
    category,
    ...animationEffects.filter((effect) => effect.category === category).map((effect) => item(effect.label)),
  );
}

/** Add Animation: the gallery's effects under the gallery's headings, then the gallery footer's five entries. */
function addAnimationEntries(): TemplateResult[] {
  return [
    effectEntries('Entrance'),
    effectEntries('Emphasis'),
    effectEntries('Exit'),
    effectEntries('Motion Paths'),
    separator(),
    item('More Entrance Effects…'),
    item('More Emphasis Effects…'),
    item('More Exit Effects…'),
    item('More Motion Paths…'),
    item('OLE Action Verbs…'),
  ];
}

/** Trigger: On Click of one of the slide's shapes, by PowerPoint's default names. On Bookmark is flattened. */
function triggerEntries(): TemplateResult[] {
  return [
    section('On Click of', item('Title 1'), item('Content Placeholder 2'), item('Picture 3')),
    separator(),
    item('On Bookmark'),
  ];
}

// ── Excel's Data ─────────────────────────────────────────────────────────────

/** From Other Sources: Office 2016's legacy wizards, in Office's order. */
function fromOtherSourcesEntries(): TemplateResult[] {
  return [
    item('From SQL Server'),
    item('From Analysis Services'),
    item('From OData Data Feed'),
    item('From XML Data Import'),
    item('From Data Connection Wizard'),
    item('From Microsoft Query'),
  ];
}

/** Refresh All's arrow. */
function refreshAllEntries(): TemplateResult[] {
  return [
    item('Refresh All', 'Ctrl+Alt+F5'),
    item('Refresh', 'Alt+F5'),
    item('Refresh Status'),
    item('Cancel Refresh'),
    separator(),
    item('Connection Properties…'),
  ];
}

/** Data Validation's arrow. */
function dataValidationEntries(): TemplateResult[] {
  return [item('Data Validation…'), item('Circle Invalid Data'), item('Clear Validation Circles')];
}

/** What-If Analysis: Office's three tools. */
function whatIfAnalysisEntries(): TemplateResult[] {
  return [item('Scenario Manager…'), item('Goal Seek…'), item('Data Table…')];
}

/** Group's arrow. */
function groupEntries(): TemplateResult[] {
  return [item('Group…', 'Shift+Alt+Right'), item('Auto Outline')];
}

/** Ungroup's arrow. */
function ungroupEntries(): TemplateResult[] {
  return [item('Ungroup…', 'Shift+Alt+Left'), item('Clear Outline')];
}

/** Excel's linked data types, each with Office's glyph. `GUESS:` an `<mjx-icon>` renders inside a cell. */
const dataTypes: readonly { readonly value: string; readonly label: string; readonly art: TemplateResult }[] = [
  { value: 'stocks', label: 'Stocks', art: html`<mjx-icon name="building-bank" size="24"></mjx-icon>` },
  { value: 'currencies', label: 'Currencies', art: html`<mjx-icon name="money" size="24"></mjx-icon>` },
  { value: 'geography', label: 'Geography', art: html`<mjx-icon name="map" size="24"></mjx-icon>` },
];

/** The Data Types gallery's items, for Excel's Data tab. Nothing is selected: a new cell has no data type. */
export function dataTypeGalleryItems(): TemplateResult[] {
  return dataTypes.map(
    (type) => html`<mjx-gallery-item value=${type.value} label=${type.label}
      ><span style="display:grid;place-items:center;inline-size:100%;aspect-ratio:1">${type.art}</span></mjx-gallery-item
    >`,
  );
}

// ── Word ─────────────────────────────────────────────────────────────────────

/** Five menus. Go to Record is a field the hosts bind. */
function wordMailingsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.mailings.start-mail-merge.start-mail-merge', 'Start Mail Merge', ...startMailMergeEntries())}
    ${commandMenu(host, 'word.mailings.start-mail-merge.select-recipients', 'Select Recipients', ...selectRecipientsEntries())}
    ${commandMenu(host, 'word.mailings.write-insert-fields.insert-merge-field', 'Insert Merge Field', ...insertMergeFieldEntries())}
    ${commandMenu(host, 'word.mailings.write-insert-fields.rules', 'Rules', ...rulesEntries())}
    ${commandMenu(host, 'word.mailings.finish.finish-merge', 'Finish & Merge', ...finishMergeEntries())}
  `;
}

// ── PowerPoint ───────────────────────────────────────────────────────────────

/** Four menus. The gallery, Start, Duration and Delay are bound by the hosts. */
function powerpointAnimationsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.animations.preview.preview', 'Preview', ...previewEntries())}
    ${commandMenu(host, 'powerpoint.animations.animations.effect-options', 'Effect Options', ...effectOptionsEntries())}
    ${commandMenu(host, 'powerpoint.animations.custom-animation.add-animation', 'Add Animation', ...addAnimationEntries())}
    ${commandMenu(host, 'powerpoint.animations.custom-animation.trigger', 'Trigger', ...triggerEntries())}
  `;
}

// ── Excel ────────────────────────────────────────────────────────────────────

/** Six menus. The Data Types gallery is bound by the hosts. */
function excelDataMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.data.get-external-data.from-other-sources', 'From Other Sources', ...fromOtherSourcesEntries())}
    ${commandMenu(host, 'excel.data.queries-connections.refresh-all', 'Refresh All', ...refreshAllEntries())}
    ${commandMenu(host, 'excel.data.data-tools.data-validation', 'Data Validation', ...dataValidationEntries())}
    ${commandMenu(host, 'excel.data.forecast.what-if-analysis', 'What-If Analysis', ...whatIfAnalysisEntries())}
    ${commandMenu(host, 'excel.data.outline.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'excel.data.outline.ungroup', 'Ungroup', ...ungroupEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

const menusByApplication: Readonly<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordMailingsMenus,
  powerpoint: powerpointAnimationsMenus,
  excel: excelDataMenus,
};

/**
 * Every menu one application's unit-7 tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `insertMenus` is.
 */
export function mailingsAnimationsDataMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  return menusByApplication[application](host);
}
