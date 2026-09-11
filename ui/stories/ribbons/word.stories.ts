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
 * **File and Home** are real. Every other tab is a placeholder — one group carrying the tab's name,
 * at the priority `dev/ribbons/census.ts` declares for it, holding one button that says so. That is
 * unit 0 of the ribbon programme: the scaffold, with the census transcribed, the ladder already
 * right and every tab present, so each later unit is a small diff rather than a new file.
 *
 * The placeholder button says *Not yet authored* rather than naming a plausible command, for the
 * reason `dev/word-tab-home.ts` gives about its own filler: a made-up command name is a worse lie
 * than an obvious placeholder, and a placeholder occupies exactly as much of the layout as a
 * command does.
 *
 * **Nothing here dispatches a command.** The paste button's menu opens, the pickers open, the
 * gallery previews — and no document changes, because command dispatch is loop 2.
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
          'File and Home are authored; the rest are placeholders carrying the census’s own ' +
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
    slot="essential"
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
 *    first, then Print, then Info, Share and Export; Open and Save are still there at a phone's
 *    width, each down to its one essential command — *Browse* and *Save*.
 * 2. **Browse survives the Open group's collapse and Recent does not.** Recent is the page's
 *    headline and the group's `large` button; a collapsed group has room for a verb rather than for
 *    a list, and the verb is *go and find one*.
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
 * 2. **Only three commands per group can draw pressed.** Bold, Italic and Underline are toggles and
 *    Strikethrough, Subscript and Superscript are not; Left, Centre and Right are toggles and
 *    Justify is not. `essentialCommandLimit` is 3 and `shell-parts.ts`'s `toggle()` always claims
 *    an essential slot, so a fourth state command in one group is impossible today. The commands
 *    are all there and all work; what the fourth cannot do is show you the paragraph is justified.
 *    `dev/ribbons/census.ts` names all four groups this bites.
 * 3. **Editor is new here.** The census has declared Word's one-command Editor group since unit 0
 *    and nothing rendered it, so this catalogue's Word could not open the proofing pane.
 * 4. **Three icons changed meaning rather than appearing.** Format Painter was a cog, Bullets was a
 *    plus, Numbering was a minus, Borders was a table and Select was a tick — five stand-ins from
 *    the migrated shell set, each replaced by the glyph Office actually draws.
 *
 * Drag the container in and Font and Paragraph are the last two groups standing, each down to its
 * three toggles; Editor and Editing give way first.
 */
export const Home: Story = { render: () => ribbon('home') };

/** Unit 3. */
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
