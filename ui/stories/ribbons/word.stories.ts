import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { styleGalleryItems } from '../gallery/specimens.ts';
import {
  documentThemePalette,
  machineFonts,
  recentColors,
  standardColors,
} from '../pickers/specimens.ts';
import {
  copyCounts,
  openDeclaredSurface,
  printerList,
  ribbonColourFieldStyle,
  ribbonFieldStyle,
  ribbonGalleryStyle,
  ribbonKeyboard,
  ribbonNarrowFieldStyle,
  ribbonScreenReader,
  ribbonStatesMatrix,
  ribbonTokenDependencies,
  type ControlOverrides,
} from './ribbon-parts.ts';
import { insertMenus } from './insert-menus.ts';
import { wordContextualSets, wordTabs } from './word.ts';

/**
 * **Word's ribbon, tab by tab** — the same functions `Shell/Word` composes, shown one tab at a
 * time so each can be audited on its own.
 *
 * Every story below renders the **whole** ribbon and selects one tab, so the tab strip is real and
 * switching tabs works: what a reviewer is looking at is a tab in its ribbon rather than a tab
 * extracted from one. Drag the container narrower and the priority ladder demotes and collapses the
 * groups; take it below 600px and the strip itself becomes a picker.
 *
 * ## What is authored and what is not
 *
 * **File, Home and Insert** are real. Every other tab is a placeholder — one group carrying the tab's name,
 * at the priority `dev/ribbons/census.ts` declares for it, holding one button that says so. That is
 * unit 0 of the ribbon programme: the scaffold, with the census transcribed, the ladder already
 * right and every tab present, so each later unit is a small diff rather than a new file.
 *
 * The placeholder button says *Not yet authored* rather than naming a plausible command, for the
 * reason `dev/word-tab-home.ts` gives about its own filler: a made-up command name is a worse lie
 * than an obvious placeholder, and a placeholder occupies exactly as much of the layout as a
 * command does.
 *
 * **Nothing here dispatches a command.** The paste button's menu opens, the Insert tab's menus open,
 * the pickers open, the gallery previews — and no document changes, because command dispatch is loop 2.
 */

const conventions = storyConventions({
  statesMatrix: ribbonStatesMatrix,
  tokenDependencies: ribbonTokenDependencies,
  keyboard: ribbonKeyboard,
  screenReader: ribbonScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Ribbons/Word',
  parameters: {
    docs: {
      description: {
        component:
          'Word’s twelve core tabs and its File tab, each shown selected inside the whole ribbon. ' +
          'File, Home and Insert are authored; the rest are placeholders carrying the census’s own ' +
          'priorities.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * The catalogue's own bindings for the commands the census cannot describe.
 *
 * Deliberately **not** the shell's. A shell binds this machine's font list, this document's palette
 * and the id of the menu its paste button opens, because those are application state; a catalogue
 * binds whatever shows the control at its most legible. The two agree on the command ids and on
 * nothing else, which is exactly the seam `stories/ribbons/ribbon-parts.ts` exists to draw.
 */
const bindings: ControlOverrides = {
  'word.file.print.printer': html`<mjx-dropdown
    id="ribbons-word-printer"
    label="Printer"
    value="pdf"
    style=${ribbonFieldStyle}
  >
    ${printerList.map(
      (printer) => html`<mjx-option value=${printer.value} label=${printer.label}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'word.file.print.copies': html`<mjx-combo-box
    id="ribbons-word-copies"
    label="Copies"
    value="1"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${copyCounts.map((count) => html`<mjx-option value=${count} label=${count}></mjx-option>`)}
  </mjx-combo-box>`,
  'word.home.clipboard.paste': html`<mjx-split-button
    label="Paste"
    icon="clipboard-paste"
    size="large"
    menu-label="Paste options"
    data-opens="ribbons-word-paste"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.home.font.name': html`<mjx-font-picker
    id="ribbons-word-font"
    style=${ribbonFieldStyle}
    label="Font"
    value="Cambria"
    .fonts=${machineFonts}
  ></mjx-font-picker>`,
  'word.home.font.size': html`<mjx-dropdown
    id="ribbons-word-size"
    label="Font size"
    value="11"
    style=${ribbonNarrowFieldStyle}
  >
    ${['9', '10', '11', '12', '14', '18'].map(
      (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'word.home.font.colour': html`<mjx-color-picker
    id="ribbons-word-colour"
    style=${ribbonColourFieldStyle}
    label="Font colour"
    show-automatic
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'word.home.styles.gallery': html`<mjx-gallery
    id="ribbons-word-styles"
    label="Styles"
    value="normal"
    style=${ribbonGalleryStyle}
  >
    ${styleGalleryItems()}
  </mjx-gallery>`,
  // Insert (unit 3). Office draws each of these as a dropdown or a split button, so each opens
  // its menu from `stories/ribbons/insert-menus.ts`: a dropdown is one `<mjx-button>` whose press
  // opens the menu, a split button opens it from its arrow. `data-opens` is
  // `commandSurfaceId('ribbons', <this key>)`, and `tests/ribbons.test.ts` requires exactly that.
  'word.insert.pages.cover-page': html`<mjx-button
    label="Cover Page"
    size="small"
    data-opens="ribbons-word-insert-pages-cover-page"
  ></mjx-button>`,
  'word.insert.tables.table': html`<mjx-button
    label="Table"
    icon="table"
    size="large"
    data-opens="ribbons-word-insert-tables-table"
  ></mjx-button>`,
  'word.insert.illustrations.pictures': html`<mjx-button
    label="Pictures"
    icon="image"
    size="large"
    data-opens="ribbons-word-insert-illustrations-pictures"
  ></mjx-button>`,
  'word.insert.illustrations.shapes': html`<mjx-button
    label="Shapes"
    icon="shapes"
    size="large"
    data-opens="ribbons-word-insert-illustrations-shapes"
  ></mjx-button>`,
  'word.insert.illustrations.3d-models': html`<mjx-button
    label="3D Models"
    icon="cube"
    size="large"
    data-opens="ribbons-word-insert-illustrations-3d-models"
  ></mjx-button>`,
  'word.insert.illustrations.screenshot': html`<mjx-button
    label="Screenshot"
    icon="screenshot"
    size="small"
    data-opens="ribbons-word-insert-illustrations-screenshot"
  ></mjx-button>`,
  'word.insert.links.link': html`<mjx-split-button
    label="Link"
    icon="link"
    size="small"
    data-opens="ribbons-word-insert-links-link"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.insert.header-footer.header': html`<mjx-button
    label="Header"
    icon="document-header"
    size="small"
    data-opens="ribbons-word-insert-header-footer-header"
  ></mjx-button>`,
  'word.insert.header-footer.footer': html`<mjx-button
    label="Footer"
    icon="document-footer"
    size="small"
    data-opens="ribbons-word-insert-header-footer-footer"
  ></mjx-button>`,
  'word.insert.header-footer.page-number': html`<mjx-button
    label="Page Number"
    icon="document-page-number"
    size="small"
    data-opens="ribbons-word-insert-header-footer-page-number"
  ></mjx-button>`,
  'word.insert.text.text-box': html`<mjx-button
    label="Text Box"
    icon="textbox"
    size="large"
    data-opens="ribbons-word-insert-text-text-box"
  ></mjx-button>`,
  'word.insert.text.quick-parts': html`<mjx-button
    label="Quick Parts"
    size="small"
    data-opens="ribbons-word-insert-text-quick-parts"
  ></mjx-button>`,
  'word.insert.text.wordart': html`<mjx-button
    label="WordArt"
    icon="text-effects"
    size="small"
    data-opens="ribbons-word-insert-text-wordart"
  ></mjx-button>`,
  'word.insert.text.drop-cap': html`<mjx-button
    label="Drop Cap"
    size="small"
    data-opens="ribbons-word-insert-text-drop-cap"
  ></mjx-button>`,
  'word.insert.text.signature-line': html`<mjx-split-button
    label="Signature Line"
    icon="signature"
    size="small"
    data-opens="ribbons-word-insert-text-signature-line"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.insert.text.object': html`<mjx-split-button
    label="Object"
    size="small"
    data-opens="ribbons-word-insert-text-object"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.insert.symbols.equation': html`<mjx-split-button
    label="Equation"
    icon="math-formula"
    size="large"
    data-opens="ribbons-word-insert-symbols-equation"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.insert.symbols.symbol': html`<mjx-button
    label="Symbol"
    size="small"
    data-opens="ribbons-word-insert-symbols-symbol"
  ></mjx-button>`,
};

/**
 * The whole ribbon with one tab selected, and the paste menu the Home tab opens.
 *
 * ⚠ **No `<mjx-resizable-container>` of its own, deliberately.** `.storybook/preview.ts`'s
 * `withContainer` already renders every story inside one, so a second would put a duplicate preset
 * bar above every tab in this section — forty-three of them across the three applications — and
 * would give an auditor two width controls that mean the same thing. The pinned stories in
 * `Ribbon/Ribbon` wrap because they fix a width the toolbar cannot reach; nothing here does.
 * Drag the harness's own handle and the ladder demotes exactly as it would in a shell.
 */
function ribbon(selected: string): TemplateResult {
  return html`
    <mjx-ribbon label="Word" selected=${selected} @mjx-activate=${openDeclaredSurface}>
      ${wordTabs({ controls: bindings, includeViewTabs: true })} ${wordContextualSets()}
    </mjx-ribbon>
    <mjx-menu id="ribbons-word-paste" label="Paste options" floating>
      <mjx-menu-section label="Paste">
        <mjx-menu-item kind="radio" label="Keep Source Formatting" checked></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Merge Formatting"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Keep Text Only"></mjx-menu-item>
      </mjx-menu-section>
      <mjx-menu-separator></mjx-menu-separator>
      <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
    </mjx-menu>

    ${insertMenus('word', 'ribbons')}
  `;
}

/**
 * **File** — the backstage destinations as an ordinary ribbon tab, which is decision 1 of the
 * approved plan and the ribbon programme's unit 1.
 *
 * Seven groups: Info, Open, Save, Print, Share, Export, Help, in the order Office lists them down
 * the left of its backstage screen. What to look at:
 *
 * 1. **The two primary groups are Open and Save**, which is the census's own reading of what a File
 *    tab is for, and it is what decides the collapse order. Drag the container in and Help goes
 *    first, then Print, then Info, Share and Export; Open and Save give way last.
 * 2. **AutoSave is the only command on the tab that survives a collapse.** Collapsed, Save keeps it
 *    beside the trigger and every other group is its trigger alone. Unit 1 gave each group a
 *    survivor; six of the seven open something or cannot be taken back — Browse is a file dialog,
 *    Protect a menu, Print a page that has left the printer — and demotion rule 1 refuses all of
 *    them. Expanded, every command draws in the order Office lists it, survivors included.
 * 3. **Properties has no icon**, deliberately, and neither do three commands on PowerPoint's and
 *    Excel's File tabs. Fluent draws nothing honest for them, and `<mjx-icon>`'s own rule is that a
 *    wrong icon is worse than a missing one because a person acts on it.
 * 4. **AutoSave is the only toggle on the whole tab**, and it is drawn pressed, because that is
 *    what Office ships for a cloud document.
 *
 * Printer and Copies are bound here rather than declared in the census: *which printer* is this
 * machine's business and no ribbon data can know it. See `stories/ribbons/ribbon-parts.ts`.
 */
export const File: Story = { render: () => ribbon('file') };

/**
 * **Home** — the tab a person spends their day on, and the ribbon programme's unit 2. Six groups:
 * Clipboard, Font, Paragraph, Styles, Editing and Editor, carrying every command Office's Home tab
 * shows. What to look at:
 *
 * 1. **Most of this tab is unlabelled, and that is Office's layout rather than a shortcut.** Font
 *    draws two fields and eleven glyphs; Paragraph draws fourteen glyphs and nothing else. A
 *    `size="icon"` control keeps its label as the accessible name — it is drawn off-screen, never
 *    dropped — so every one of them is still reachable by a screen reader and by a tooltip.
 * 2. **Every state command draws pressed, and only three per group survive a collapse.** All six
 *    character formats are toggles, and so are Show/Hide ¶ and all four alignments — Strikethrough,
 *    Subscript, Superscript, Justify and Show/Hide could not be before unit 2b, because a toggle
 *    was essential by construction and a group may keep three. *Survives a collapse* is declared
 *    now: Bold and Italic, and Left, Centre and Right. Underline draws pressed and does not survive,
 *    because in Word it is a split button. **And the survivors draw where Office draws them** — Bold
 *    is the seventh command in Font, not the first. `dev/ribbons/census.ts` has the
 *    table for all three applications.
 * 3. **Editor is new here.** The census has declared Word's one-command Editor group since unit 0
 *    and nothing rendered it, so this catalogue's Word could not open the proofing pane.
 * 4. **Three icons changed meaning rather than appearing.** Format Painter was a cog, Bullets was a
 *    plus, Numbering was a minus, Borders was a table and Select was a tick — five stand-ins from
 *    the migrated shell set, each replaced by the glyph Office actually draws.
 *
 * Drag the container in and Font and Paragraph are the last two groups standing, down to their
 * survivors beside the trigger — Bold and Italic, and the three alignments; Editor and Editing give
 * way first, and keep none — Find is a split button in Office, and a survivor may not open anything.
 */
export const Home: Story = { render: () => ribbon('home') };

/**
 * **Insert** — the tab of things to put on the page, and the ribbon programme's unit 3. Nine groups:
 * Pages, Tables, Illustrations, Media, Links, Comments, Header & Footer, Text and Symbols, in
 * Office's order. What to look at:
 *
 * 1. **Eighteen of the twenty-eight commands open something, and each one really does.** Press
 *    Table, Shapes, Header or Text Box and its menu opens under it; press the arrow on Link,
 *    Signature Line, Object or Equation and the split button's menu opens. Each menu carries a
 *    handful of real Office entries — the built-in headers by name, then Edit Header and Remove
 *    Header — rather than the whole gallery, which is decision 3 of the approved plan. The other ten
 *    open a dialog or a card in Office (Icons, SmartArt, Chart, Online Videos, Bookmark,
 *    Cross-reference, Comment, Date & Time) or insert at once (Blank Page, Page Break), and are drawn
 *    as the plain buttons they are.
 * 2. **Nothing on this tab survives a collapse.** Drag the container in and every group goes to its
 *    trigger alone. That is the demotion rules working rather than a gap: nearly every command opens
 *    a surface, Page Break is on the keyboard, and Blank Page's glyph is New Document's.
 *    `dev/ribbons/census.ts` gives each group's reason.
 * 3. **Large where Office draws large, a column where Office draws a column.** Table, Pictures,
 *    Shapes, Icons, 3D Models, Online Videos, Comment, Text Box and Equation are large; Pages, Links,
 *    Header & Footer and most of Text are columns of labelled commands. The sizes of Links and Header
 *    & Footer are marked `GUESS:` beside their groups, because Office has drawn them both ways.
 * 4. **Six commands carry no icon** — Cover Page, Cross-reference, Quick Parts, Drop Cap, Object and
 *    Symbol — because Fluent draws nothing honest for them, and a wrong glyph is worse than a label.
 *
 * Tables and Illustrations are the tab's primary groups, so they are the last two standing; Media
 * and Comments give way first.
 */
export const Insert: Story = { render: () => ribbon('insert') };

/** Unit 4. */
export const Draw: Story = { render: () => ribbon('draw') };

/** Unit 5. */
export const Design: Story = { render: () => ribbon('design') };

/** Unit 5. Word calls its page-setup tab Layout; the census calls it `TabPageLayoutWord`. */
export const Layout: Story = { render: () => ribbon('layout') };

/** Unit 6. */
export const References: Story = { render: () => ribbon('references') };

/** Unit 7. */
export const Mailings: Story = { render: () => ribbon('mailings') };

/** Unit 8. */
export const Review: Story = { render: () => ribbon('review') };

/** Unit 9. */
export const View: Story = { render: () => ribbon('view') };

/** Unit 10, and a tab Office shows only in Outline view — see `dev/ribbons/census.ts`. */
export const Outlining: Story = { render: () => ribbon('outlining') };

/** Unit 10, and a view tab. */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/** A view tab: Office shows it only while a picture's background is being removed. */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };
