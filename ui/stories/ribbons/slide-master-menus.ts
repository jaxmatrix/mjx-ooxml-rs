/**
 * **The menus the master view tabs open**: PowerPoint's Slide Master, the Home tab beside it, Handout Master and Notes
 * Master, written once for the host that draws them.
 *
 * The pattern is `stories/ribbons/print-preview-menus.ts`'s, for its reasons. A binding lives in its host. The
 * menu it opens is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host
 * renders `masterViewMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below
 * spells its command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## Shaped for three master views
 *
 * `dev/ribbons/census.ts` declares **Edit Theme, Background and Close once, as functions of the master view**,
 * because `TabSlideMaster`, `TabHandoutMaster` and `TabNotesMaster` carry the same three groups. The menus follow
 * the same seam, with one constraint the gate imposes: a menu's command id is spelt literally, so a menu cannot
 * be a function of the tab. So what is shared is **the entries**, one function per group: `editThemeMenuEntries`
 * and `backgroundMenuEntries` below hand each menu of those groups its list, and each master view writes its own
 * `commandMenu` calls under its own ids. Close opens nothing. **The Handout Master and Notes Master units each add
 * one function to `menusByTab`** that calls the two, and write no list.
 *
 * ## The entries are Design's, not copies
 *
 * Themes, Colours, Fonts, Effects, Background Styles and Slide Size are the same commands as on PowerPoint's
 * Design tab, and Office opens the same lists. So every list is **imported** from
 * `stories/ribbons/design-layout-menus.ts`: `powerpointThemeEntries` (the Themes gallery's `themes`, as a menu),
 * `themeColourEntries`, `themeFontEntries`, `themeEffectEntries`, `backgroundStyleEntries` and `slideSizeEntries`.
 * **This unit completed four of them to Office's whole lists**, so Word's Design, Excel's Page Layout and
 * PowerPoint's Variants footer draw the same lists; see that file.
 *
 * **Insert Placeholder's list is Slide Master's own**, because no other tab opens it.
 *
 * ## PowerPoint's Slide Master Home
 *
 * The Home tab Slide Master view shows opens two menus of its own, both from its Master Slides group: **Layout**, the
 * Office Theme's eleven layouts, and **Section**, Office's six section commands. Neither list is written anywhere
 * else: Insert's New Slide lists seven layouts rather than eleven, and Home draws its Layout and Section as buttons.
 * Every other binding on that tab is Home's and opens Home's paste menu, which the host renders already.
 *
 * ## PowerPoint's Handout Master
 *
 * **The first unit to take the shared seam**: `handoutMasterMenus` writes Edit Theme's four and Background Styles'
 * `commandMenu` calls under its own ids over `editThemeMenuEntries` and `backgroundMenuEntries`, and writes no theme
 * list. Its Page Setup opens three menus: **Handout Orientation** over Layout's `orientationEntries()`, **Slide
 * Size** over Design's `slideSizeEntries()`, and **Slides Per Page**, Office's seven handout layouts, which is
 * Handout Master's own list because no other tab opens it. Its Placeholders are checkboxes and open nothing.
 *
 * ## PowerPoint's Notes Master
 *
 * **The second unit to take the shared seam, and the first to write no list at all**: `notesMasterMenus` writes Edit
 * Theme's four and Background Styles' `commandMenu` calls under its own ids over `editThemeMenuEntries` and
 * `backgroundMenuEntries`, and its Page Setup opens two menus over lists already written, **Notes Page Orientation**
 * over Layout's `orientationEntries()` and **Slide Size** over Design's `slideSizeEntries()`. Its six Placeholders
 * are checkboxes and open nothing.
 *
 * ## Who renders these
 *
 * Every master view is `appearance: 'view'`: Office shows it only inside its view, so `tabsFor` leaves it out of a
 * strip unless `includeViewTabs` is asked for, and only the `Ribbons/*` hosts ask. **So
 * `stories/ribbons/powerpoint.stories.ts` is the host that binds and renders these.** No shell draws the tab, and
 * `tests/ribbons.test.ts` refuses a shell that opens one of these menus.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no deck changes, because
 * command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import {
  backgroundStyleEntries,
  orientationEntries,
  powerpointThemeEntries,
  slideSizeEntries,
  themeColourEntries,
  themeEffectEntries,
  themeFontEntries,
} from './design-layout-menus.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the shared groups: Edit Theme and Background, on every master view ──────

/** Edit Theme's four lists, one per command, for any master view. */
export const editThemeMenuEntries = {
  themes: powerpointThemeEntries,
  colours: themeColourEntries,
  fonts: themeFontEntries,
  effects: themeEffectEntries,
} as const;

/**
 * Background's one list, for any master view. Hide Background Graphics is a checkbox a host binds, and the
 * group's launcher opens the Format Background pane rather than a menu.
 */
export const backgroundMenuEntries = {
  backgroundStyles: backgroundStyleEntries,
} as const;

// ── PowerPoint's Slide Master ────────────────────────────────────────────────

/**
 * Insert Placeholder's arrow: **Office's ten placeholders**, in Office's order. The split button's face inserts
 * the first, Content.
 *
 * No entry carries a glyph. Office draws one beside each, and Fluent has no vertical content placeholder to draw
 * beside *Content (Vertical)*, so a list with nine glyphs and a gap would be the one entry that looks broken.
 */
function insertPlaceholderEntries(): TemplateResult[] {
  return [
    'Content',
    'Content (Vertical)',
    'Text',
    'Text (Vertical)',
    'Picture',
    'Chart',
    'Table',
    'SmartArt',
    'Media',
    'Online Image',
  ].map((label) => html`<mjx-menu-item label=${label}></mjx-menu-item>`);
}

/**
 * Seven menus: Insert Placeholder's, Edit Theme's four, Background Styles and Slide Size. Title, Footers and Hide
 * Background Graphics are checkboxes the host binds; Master Layout and Rename open dialogs.
 */
function slideMasterMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.slide-master.master-layout.insert-placeholder', 'Insert Placeholder', ...insertPlaceholderEntries())}
    ${commandMenu(host, 'powerpoint.slide-master.edit-theme.themes', 'Themes', ...editThemeMenuEntries.themes())}
    ${commandMenu(host, 'powerpoint.slide-master.edit-theme.colours', 'Colours', ...editThemeMenuEntries.colours())}
    ${commandMenu(host, 'powerpoint.slide-master.edit-theme.fonts', 'Fonts', ...editThemeMenuEntries.fonts())}
    ${commandMenu(host, 'powerpoint.slide-master.edit-theme.effects', 'Effects', ...editThemeMenuEntries.effects())}
    ${commandMenu(host, 'powerpoint.slide-master.background.background-styles', 'Background Styles', ...backgroundMenuEntries.backgroundStyles())}
    ${commandMenu(host, 'powerpoint.slide-master.size.slide-size', 'Slide Size', ...slideSizeEntries())}
  `;
}

// ── PowerPoint's Slide Master Home ───────────────────────────────────────────

/**
 * Layout's list: **the Office Theme's eleven layouts**, in the order Office's Layout gallery draws them, under the
 * theme's name. No entry is ticked: in master view the selection is a master or a layout rather than a slide, so
 * there is no current layout to tick. `GUESS:` the order, and the unticked list.
 */
function officeThemeLayoutEntries(): TemplateResult {
  return html`<mjx-menu-section label="Office Theme">
    ${[
      'Title Slide',
      'Title and Content',
      'Section Header',
      'Two Content',
      'Comparison',
      'Title Only',
      'Blank',
      'Content with Caption',
      'Picture with Caption',
      'Title and Vertical Text',
      'Vertical Title and Text',
    ].map((label) => html`<mjx-menu-item label=${label}></mjx-menu-item>`)}
  </mjx-menu-section>`;
}

/**
 * Section's list: **Office's six section commands**, the four that change the deck's sections and, after a
 * separator, the two that fold the thumbnail pane. `GUESS:` that the separator is Office's.
 */
function sectionEntries(): TemplateResult[] {
  return [
    html`<mjx-menu-item label="Add Section"></mjx-menu-item>`,
    html`<mjx-menu-item label="Rename Section"></mjx-menu-item>`,
    html`<mjx-menu-item label="Remove Section"></mjx-menu-item>`,
    html`<mjx-menu-item label="Remove All Sections"></mjx-menu-item>`,
    html`<mjx-menu-separator></mjx-menu-separator>`,
    html`<mjx-menu-item label="Collapse All"></mjx-menu-item>`,
    html`<mjx-menu-item label="Expand All"></mjx-menu-item>`,
  ];
}

/** Two menus, both from Master Slides. Insert Slide Master, Insert Layout and Reset open nothing. */
function slideMasterHomeMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.slide-master-home.master-slides.layout', 'Layout', officeThemeLayoutEntries())}
    ${commandMenu(host, 'powerpoint.slide-master-home.master-slides.section', 'Section', ...sectionEntries())}
  `;
}

// ── PowerPoint's Handout Master ──────────────────────────────────────────────

/**
 * Slides Per Page's list: **Office's seven handout layouts**, in Office's order, **6 Slides** checked as a new deck's
 * handout master. PowerPoint's Print Preview names the same layouts in Print What as *Handouts (n Slides Per Page)*
 * and *Outline View*, a field's voice rather than a menu's, so neither list is the other. `GUESS:` the labels, the
 * order, the start, and that Office draws no separator before Outline.
 */
function slidesPerPageEntries(): TemplateResult[] {
  return ['1 Slide', '2 Slides', '3 Slides', '4 Slides', '6 Slides', '9 Slides', 'Outline'].map(
    (label) => html`<mjx-menu-item kind="radio" label=${label} ?checked=${label === '6 Slides'}></mjx-menu-item>`,
  );
}

/**
 * Eight menus: Page Setup's three, Edit Theme's four and Background Styles. Header, Date, Footer, Page Number and
 * Hide Background Graphics are checkboxes the host binds; Close Master View opens nothing.
 */
function handoutMasterMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.handout-master.page-setup.handout-orientation', 'Handout Orientation', ...orientationEntries())}
    ${commandMenu(host, 'powerpoint.handout-master.page-setup.slide-size', 'Slide Size', ...slideSizeEntries())}
    ${commandMenu(host, 'powerpoint.handout-master.page-setup.slides-per-page', 'Slides Per Page', ...slidesPerPageEntries())}
    ${commandMenu(host, 'powerpoint.handout-master.edit-theme.themes', 'Themes', ...editThemeMenuEntries.themes())}
    ${commandMenu(host, 'powerpoint.handout-master.edit-theme.colours', 'Colours', ...editThemeMenuEntries.colours())}
    ${commandMenu(host, 'powerpoint.handout-master.edit-theme.fonts', 'Fonts', ...editThemeMenuEntries.fonts())}
    ${commandMenu(host, 'powerpoint.handout-master.edit-theme.effects', 'Effects', ...editThemeMenuEntries.effects())}
    ${commandMenu(host, 'powerpoint.handout-master.background.background-styles', 'Background Styles', ...backgroundMenuEntries.backgroundStyles())}
  `;
}

// ── PowerPoint's Notes Master ────────────────────────────────────────────────

/**
 * Seven menus: Page Setup's two, Edit Theme's four and Background Styles. Header, Slide Image, Footer, Date, Body, Page
 * Number and Hide Background Graphics are checkboxes the host binds; Close Master View opens nothing.
 */
function notesMasterMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.notes-master.page-setup.notes-page-orientation', 'Notes Page Orientation', ...orientationEntries())}
    ${commandMenu(host, 'powerpoint.notes-master.page-setup.slide-size', 'Slide Size', ...slideSizeEntries())}
    ${commandMenu(host, 'powerpoint.notes-master.edit-theme.themes', 'Themes', ...editThemeMenuEntries.themes())}
    ${commandMenu(host, 'powerpoint.notes-master.edit-theme.colours', 'Colours', ...editThemeMenuEntries.colours())}
    ${commandMenu(host, 'powerpoint.notes-master.edit-theme.fonts', 'Fonts', ...editThemeMenuEntries.fonts())}
    ${commandMenu(host, 'powerpoint.notes-master.edit-theme.effects', 'Effects', ...editThemeMenuEntries.effects())}
    ${commandMenu(host, 'powerpoint.notes-master.background.background-styles', 'Background Styles', ...backgroundMenuEntries.backgroundStyles())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

/**
 * Every master view's menus, as each unit authors them, one function per master view tab. Slide Master Home is not a
 * master view of its own, and shares this file because it is shown inside Slide Master view.
 */
const menusByTab: readonly ((host: RibbonSurfaceHost) => TemplateResult)[] = [
  slideMasterMenus,
  slideMasterHomeMenus,
  handoutMasterMenus,
  notesMasterMenus,
];

/**
 * Every menu one application's master view tabs open, with ids for one host's page. Only PowerPoint has master
 * views, so Word and Excel render nothing.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `printPreviewMenus` is, by a host that
 * draws view tabs.
 */
export function masterViewMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult | typeof nothing {
  return application === 'powerpoint' ? html`${menusByTab.map((menus) => menus(host))}` : nothing;
}
