/**
 * **The menus the master view tabs open**: PowerPoint's Slide Master today, and Handout Master and Notes Master
 * when their units land, written once for the host that draws them.
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

// ── what a host renders ──────────────────────────────────────────────────────

/** Every master view's menus, as each unit authors them. Handout Master and Notes Master add theirs here. */
const menusByTab: readonly ((host: RibbonSurfaceHost) => TemplateResult)[] = [slideMasterMenus];

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
