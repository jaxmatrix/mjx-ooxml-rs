import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MiniCommand } from '../../src/feedback/feedback-model.ts';
import type { ReviewAnnotation } from '../../src/annotation/review-pane.ts';
import type { TreeNode } from '../../src/navigators/navigator-model.ts';
import { smallOutline, smallOutlineExpanded } from '../navigators/specimens.ts';
import { lineSpacingOptions } from '../inputs/specimens.ts';
import { documentThemePalette, machineFonts, recentColors, standardColors } from '../pickers/specimens.ts';
import { styleGalleryItems } from '../gallery/specimens.ts';
import { wordPhoneCommands } from '../mobile/specimens.ts';
import {
  shellKeyboard,
  shellScreenReader,
  shellStatesMatrix,
  shellTokenDependencies,
} from './shell-model.ts';
import {
  canvasArea,
  canvasRow,
  contextRegionStyle,
  documentColumn,
  documentPlaceholder,
  field,
  navigatorPane,
  openDeclaredSurface,
  openSheetOnCommand,
  paneSplitter,
  paneStack,
  phoneBody,
  phoneRails,
  selectionRun,
  shellFrame,
  statusBar,
  surface,
  ribbonColourFieldStyle,
  ribbonFieldStyle,
  ribbonGalleryStyle,
  ribbonNarrowFieldStyle,
  workspaceStyle,
  zoom,
} from './shell-parts.ts';
import { wordContextualSets, wordTabs } from '../ribbons/word.ts';
import { designLayoutMenus, styleSetGalleryItems } from '../ribbons/design-layout-menus.ts';
import { drawMenus } from '../ribbons/draw-menus.ts';
import { insertMenus } from '../ribbons/insert-menus.ts';
import { mailingsAnimationsDataMenus } from '../ribbons/mailings-animations-data-menus.ts';
import { referencesTransitionsFormulasMenus } from '../ribbons/references-transitions-formulas-menus.ts';
import { reviewMenus } from '../ribbons/review-menus.ts';
import { viewMenus } from '../ribbons/view-menus.ts';
import {
  citationStyles,
  copyCounts,
  displayForReviewModes,
  mergeRecordNumbers,
  printerList,
} from '../ribbons/ribbon-parts.ts';

/**
 * **Word, assembled** — the ribbon, the navigation pane, the page, the review margin and the status
 * bar, at desktop, tablet and phone.
 *
 * Word is the shell with **two** panes flanking the document, and that is the composition question
 * this story exists to put: a navigation pane on one side and a review margin on the other leave the
 * page whatever is between them, and at tablet width there may not be enough of it. What to look at:
 *
 * 1. **The desktop.** Whether the page reads as a page rather than as a column between two panels.
 * 2. **The margin.** Cards packed to their anchors beside a document, in both schemes — the author
 *    colours were searched for against the palette, and this is the first time they are seen next to
 *    a ribbon and a status bar rather than on their own.
 * 3. **The tablet.** Both panes are still docked. Decide whether that is right.
 * 4. **The scroll marks.** Search hits, comments and tracked changes along the scrollbar, beside a
 *    review margin that says the same thing a different way.
 *
 * **Nothing here dispatches a command.** Press the Paragraph group's dialog launcher and a modal
 * arrives over the assembled shell; nothing in the document moves, because command dispatch and
 * document binding are loop 2.
 */

const conventions = storyConventions({
  statesMatrix: shellStatesMatrix,
  tokenDependencies: shellTokenDependencies,
  keyboard: shellKeyboard,
  screenReader: shellScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Shell/Word',
  parameters: {
    docs: {
      description: {
        component:
          'The whole application assembled from the catalogue’s own components: ribbon, navigation ' +
          'pane, page, review margin and status bar on a desktop; a command rail and a sheet on a ' +
          'phone. Cosmetic only — every command is inert.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

// ── the fixtures ─────────────────────────────────────────────────────────────

/** The document's outline, with a few more headings than the navigators' small specimen. */
const outline: readonly TreeNode[] = [
  ...smallOutline,
  {
    id: 'findings',
    label: 'Findings',
    children: [
      { id: 'findings-fidelity', label: 'Fidelity' },
      { id: 'findings-performance', label: 'Performance' },
      { id: 'findings-open', label: 'Open questions' },
    ],
  },
];

/**
 * The review margin's contents: **all three card kinds at once**, which is the point of putting them
 * in a shell.
 *
 * A legacy comment, a threaded conversation, a resolved one and three tracked changes — so the
 * author colours have four authors to tell apart and the packer has anchors close enough together to
 * have to move something.
 */
const annotations: readonly ReviewAnnotation[] = [
  {
    id: 'a1',
    kind: 'comment',
    model: 'threaded',
    author: 'Ada Lovelace',
    time: '09:12',
    text: 'Define “fidelity” here.',
    anchorTop: 8,
    replies: [{ id: 'a1r1', author: 'Grace Hopper', time: '09:40', text: 'Byte identity.' }],
  },
  {
    id: 'a2',
    kind: 'comment',
    model: 'legacy',
    author: 'Charles Babbage',
    time: '10:02',
    text: 'Predates the corpus.',
    anchorTop: 70,
  },
  {
    id: 'a3',
    kind: 'trackedChange',
    changeKind: 'insertion',
    author: 'Grace Hopper',
    time: '09:41',
    text: 'Inserted a definition.',
    excerpt: 'byte identity',
    anchorTop: 132,
  },
  {
    id: 'a4',
    kind: 'trackedChange',
    changeKind: 'deletion',
    author: 'Ada Lovelace',
    time: '10:15',
    text: 'Removed a clause.',
    excerpt: 'and structural identity',
    anchorTop: 152,
  },
  {
    id: 'a5',
    kind: 'comment',
    model: 'threaded',
    author: 'Katherine Johnson',
    time: '11:30',
    text: 'Agreed — closing.',
    anchorTop: 240,
    resolved: true,
    replies: [],
  },
  {
    id: 'a6',
    kind: 'trackedChange',
    changeKind: 'formatting',
    author: 'Katherine Johnson',
    time: '11:44',
    text: 'Set to Heading 2.',
    excerpt: 'The corpus',
    anchorTop: 320,
  },
];

/** What the mini toolbar offers over selected text. */
const textCommands: readonly MiniCommand[] = [
  { command: 'bold', label: 'Bold', icon: 'text-bold', kind: 'toggle', pressed: true },
  { command: 'italic', label: 'Italic', icon: 'text-italic', kind: 'toggle' },
  { command: 'underline', label: 'Underline', icon: 'text-underline', kind: 'toggle' },
  { command: 'align-left', label: 'Align left', icon: 'text-align-left', separatorBefore: true },
  { command: 'comment', label: 'New comment', icon: 'comment', separatorBefore: true },
];

// ── the ribbon ──────────────────────────────────────────────────────────

/**
 * **Word's ribbon, from `stories/ribbons/word.ts`** — the same functions `Ribbons/Word` audits.
 *
 * The tabs used to be written here, and moving them out is the whole of the ribbon programme's
 * unit 0. What stays is what genuinely belongs to an *application*: this machine's font list, this
 * document's palette, the id of the menu the paste button opens, the contents of the styles
 * gallery. A ribbon module cannot know any of those, so it names the commands and the shell binds
 * them — keyed by the stable command ids `dev/ribbons/census.ts` declares, so neither side has to
 * know how the other spelled its markup.
 *
 * `wordTabs()` leaves out the `appearance: 'view'` tabs — Outlining, Print Preview, Background
 * Removal — because Office shows them only inside the view they name, and a shell that carried
 * them in its default strip would be showing a ribbon that does not exist. That is also why
 * **Outlining's and Print Preview's commands are bound in `Ribbons/Word` and not here**
 * (Background Removal binds none), and
 * `printPreviewMenus` is not rendered here: a binding for a tab this strip never draws would be a
 * binding to nothing, and `tests/ribbons.test.ts` refuses a shell that opens a view tab's menu.
 */
function ribbon(): TemplateResult {
  return surface(
    'ribbon',
    'flex:0 0 auto;min-inline-size:0',
    html`
      <mjx-ribbon label="Word" selected="home" @mjx-activate=${openDeclaredSurface}>
        ${wordTabs({
          controls: {
            // The File tab's Print group. *Which printers* is this machine's business and a ribbon
            // module has no way to know it, so the census declares Printer and Copies as commands
            // with no icon and the host binds a real control over each. The lists are
            // `stories/ribbons/ribbon-parts.ts`'s, shared with the catalogue, because four copies
            // of one list is four places for one of them to drift.
            'word.file.print.printer': html`<mjx-dropdown
              id="word-printer"
              label="Printer"
              value="pdf"
              style=${ribbonFieldStyle}
            >
              ${printerList.map(
                (printer) =>
                  html`<mjx-option value=${printer.value} label=${printer.label}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'word.file.print.copies': html`<mjx-combo-box
              id="word-copies"
              label="Copies"
              value="1"
              allow-custom
              style=${ribbonNarrowFieldStyle}
            >
              ${copyCounts.map(
                (count) => html`<mjx-option value=${count} label=${count}></mjx-option>`,
              )}
            </mjx-combo-box>`,
            'word.home.clipboard.paste': html`<mjx-split-button
              label="Paste"
              icon="clipboard-paste"
              size="large"
              menu-label="Paste options"
              data-opens="word-paste-menu"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.home.font.name': html`<mjx-font-picker
              id="word-font"
              style=${ribbonFieldStyle}
              label="Font"
              value="Cambria"
              .fonts=${machineFonts}
            ></mjx-font-picker>`,
            'word.home.font.size': html`<mjx-dropdown
              id="word-size"
              label="Font size"
              value="11"
              style=${ribbonNarrowFieldStyle}
            >
              ${['9', '10', '11', '12', '14', '18'].map(
                (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'word.home.font.colour': html`<mjx-color-picker
              id="word-colour"
              style=${ribbonColourFieldStyle}
              label="Font colour"
              show-automatic
              .themePalette=${documentThemePalette}
              .standardColors=${standardColors}
              .recentColors=${recentColors}
            ></mjx-color-picker>`,
            'word.home.styles.gallery': html`<mjx-gallery
              id="word-styles"
              label="Styles"
              value="normal"
              style=${ribbonGalleryStyle}
            >
              ${styleGalleryItems()}
            </mjx-gallery>`,
            // Insert (unit 3). Office draws each of these as a dropdown or a split button, so each opens
            // its menu from `stories/ribbons/insert-menus.ts`: a dropdown is one `<mjx-button>` whose press
            // opens the menu, a split button opens it from its arrow. `data-opens` is
            // `commandSurfaceId('shell', <this key>)`, and `tests/ribbons.test.ts` requires exactly that.
            'word.insert.pages.cover-page': html`<mjx-button
              label="Cover Page"
              size="small"
              data-opens="shell-word-insert-pages-cover-page"
            ></mjx-button>`,
            'word.insert.tables.table': html`<mjx-button
              label="Table"
              icon="table"
              size="large"
              data-opens="shell-word-insert-tables-table"
            ></mjx-button>`,
            'word.insert.illustrations.pictures': html`<mjx-button
              label="Pictures"
              icon="image"
              size="large"
              data-opens="shell-word-insert-illustrations-pictures"
            ></mjx-button>`,
            'word.insert.illustrations.shapes': html`<mjx-button
              label="Shapes"
              icon="shapes"
              size="large"
              data-opens="shell-word-insert-illustrations-shapes"
            ></mjx-button>`,
            'word.insert.illustrations.3d-models': html`<mjx-button
              label="3D Models"
              icon="cube"
              size="large"
              data-opens="shell-word-insert-illustrations-3d-models"
            ></mjx-button>`,
            'word.insert.illustrations.screenshot': html`<mjx-button
              label="Screenshot"
              icon="screenshot"
              size="small"
              data-opens="shell-word-insert-illustrations-screenshot"
            ></mjx-button>`,
            'word.insert.links.link': html`<mjx-split-button
              label="Link"
              icon="link"
              size="small"
              data-opens="shell-word-insert-links-link"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.insert.header-footer.header': html`<mjx-button
              label="Header"
              icon="document-header"
              size="small"
              data-opens="shell-word-insert-header-footer-header"
            ></mjx-button>`,
            'word.insert.header-footer.footer': html`<mjx-button
              label="Footer"
              icon="document-footer"
              size="small"
              data-opens="shell-word-insert-header-footer-footer"
            ></mjx-button>`,
            'word.insert.header-footer.page-number': html`<mjx-button
              label="Page Number"
              icon="document-page-number"
              size="small"
              data-opens="shell-word-insert-header-footer-page-number"
            ></mjx-button>`,
            'word.insert.text.text-box': html`<mjx-button
              label="Text Box"
              icon="textbox"
              size="large"
              data-opens="shell-word-insert-text-text-box"
            ></mjx-button>`,
            'word.insert.text.quick-parts': html`<mjx-button
              label="Quick Parts"
              size="small"
              data-opens="shell-word-insert-text-quick-parts"
            ></mjx-button>`,
            'word.insert.text.wordart': html`<mjx-button
              label="WordArt"
              icon="text-effects"
              size="small"
              data-opens="shell-word-insert-text-wordart"
            ></mjx-button>`,
            'word.insert.text.drop-cap': html`<mjx-button
              label="Drop Cap"
              size="small"
              data-opens="shell-word-insert-text-drop-cap"
            ></mjx-button>`,
            'word.insert.text.signature-line': html`<mjx-split-button
              label="Signature Line"
              icon="signature"
              size="small"
              data-opens="shell-word-insert-text-signature-line"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.insert.text.object': html`<mjx-split-button
              label="Object"
              size="small"
              data-opens="shell-word-insert-text-object"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.insert.symbols.equation': html`<mjx-split-button
              label="Equation"
              icon="math-formula"
              size="large"
              data-opens="shell-word-insert-symbols-equation"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.insert.symbols.symbol': html`<mjx-button
              label="Symbol"
              size="small"
              data-opens="shell-word-insert-symbols-symbol"
            ></mjx-button>`,
            // Draw (unit 4). Five dropdowns and a split Eraser, each opening its menu from
            // `stories/ribbons/draw-menus.ts`. `data-opens` is `commandSurfaceId('shell', <this key>)`.
            'word.draw.drawing-tools.add-pen': html`<mjx-button
              label="Add Pen"
              size="small"
              data-opens="shell-word-draw-drawing-tools-add-pen"
            ></mjx-button>`,
            'word.draw.pens.pens': html`<mjx-button
              label="Pens"
              icon="inking-tool"
              size="large"
              data-opens="shell-word-draw-pens-pens"
            ></mjx-button>`,
            'word.draw.pens.colour': html`<mjx-button
              label="Colour"
              icon="color-line"
              size="small"
              data-opens="shell-word-draw-pens-colour"
            ></mjx-button>`,
            'word.draw.pens.thickness': html`<mjx-button
              label="Thickness"
              icon="line-thickness"
              size="small"
              data-opens="shell-word-draw-pens-thickness"
            ></mjx-button>`,
            'word.draw.write.eraser': html`<mjx-split-button
              toggle
              exclusive="word.draw.write.tools"
              label="Eraser"
              icon="eraser"
              size="large"
              data-opens="shell-word-draw-write-eraser"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.draw.input-mode.touch-mouse-mode': html`<mjx-button
              label="Touch/Mouse Mode"
              size="small"
              data-opens="shell-word-draw-input-mode-touch-mouse-mode"
            ></mjx-button>`,
            // Design and Layout (unit 5). Dropdowns and split buttons open their menus from
            // `stories/ribbons/design-layout-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            // The Style Set is an in-ribbon gallery, Page Colour a colour picker, and Indent and Spacing are measures:
            // `value` is always points, so an 8 pt Spacing After is 8 and a 0 cm indent is 0.
            'word.design.style-set.themes': html`<mjx-button
              label="Themes"
              size="small"
              data-opens="shell-word-design-style-set-themes"
            ></mjx-button>`,
            'word.design.style-set.style-set': html`<mjx-gallery
              id="word-style-set"
              label="Style Set"
              value="this-document"
              style=${ribbonGalleryStyle}
            >
              ${styleSetGalleryItems()}
            </mjx-gallery>`,
            'word.design.style-set.colours': html`<mjx-button
              label="Colours"
              icon="color"
              size="large"
              data-opens="shell-word-design-style-set-colours"
            ></mjx-button>`,
            'word.design.style-set.fonts': html`<mjx-button
              label="Fonts"
              icon="text-font"
              size="large"
              data-opens="shell-word-design-style-set-fonts"
            ></mjx-button>`,
            'word.design.style-set.paragraph-spacing': html`<mjx-button
              label="Paragraph Spacing"
              icon="text-line-spacing"
              size="small"
              data-opens="shell-word-design-style-set-paragraph-spacing"
            ></mjx-button>`,
            'word.design.style-set.effects': html`<mjx-button
              label="Effects"
              icon="square-shadow"
              size="small"
              data-opens="shell-word-design-style-set-effects"
            ></mjx-button>`,
            'word.design.page-background.watermark': html`<mjx-button
              label="Watermark"
              size="small"
              data-opens="shell-word-design-page-background-watermark"
            ></mjx-button>`,
            'word.design.page-background.page-colour': html`<mjx-color-picker
              id="word-page-colour"
              style=${ribbonColourFieldStyle}
              label="Page Colour"
              show-no-fill
              .themePalette=${documentThemePalette}
              .standardColors=${standardColors}
              .recentColors=${recentColors}
            ></mjx-color-picker>`,
            'word.layout.page-setup.margins': html`<mjx-button
              label="Margins"
              icon="document-margins"
              size="large"
              data-opens="shell-word-layout-page-setup-margins"
            ></mjx-button>`,
            'word.layout.page-setup.orientation': html`<mjx-button
              label="Orientation"
              icon="orientation"
              size="large"
              data-opens="shell-word-layout-page-setup-orientation"
            ></mjx-button>`,
            'word.layout.page-setup.size': html`<mjx-button
              label="Size"
              size="small"
              data-opens="shell-word-layout-page-setup-size"
            ></mjx-button>`,
            'word.layout.page-setup.columns': html`<mjx-button
              label="Columns"
              icon="text-column-two"
              size="large"
              data-opens="shell-word-layout-page-setup-columns"
            ></mjx-button>`,
            'word.layout.page-setup.breaks': html`<mjx-button
              label="Breaks"
              icon="document-page-break"
              size="small"
              data-opens="shell-word-layout-page-setup-breaks"
            ></mjx-button>`,
            'word.layout.page-setup.line-numbers': html`<mjx-button
              label="Line Numbers"
              size="small"
              data-opens="shell-word-layout-page-setup-line-numbers"
            ></mjx-button>`,
            'word.layout.page-setup.hyphenation': html`<mjx-button
              label="Hyphenation"
              size="small"
              data-opens="shell-word-layout-page-setup-hyphenation"
            ></mjx-button>`,
            'word.layout.paragraph.indent-left': html`<mjx-measure-input
              id="word-indent-left"
              label="Indent Left"
              value="0"
              unit="cm"
              step="0.25"
              style=${ribbonNarrowFieldStyle}
            ></mjx-measure-input>`,
            'word.layout.paragraph.indent-right': html`<mjx-measure-input
              id="word-indent-right"
              label="Indent Right"
              value="0"
              unit="cm"
              step="0.25"
              style=${ribbonNarrowFieldStyle}
            ></mjx-measure-input>`,
            'word.layout.paragraph.spacing-before': html`<mjx-measure-input
              id="word-spacing-before"
              label="Spacing Before"
              value="0"
              unit="pt"
              step="6"
              min="0"
              style=${ribbonNarrowFieldStyle}
            ></mjx-measure-input>`,
            'word.layout.paragraph.spacing-after': html`<mjx-measure-input
              id="word-spacing-after"
              label="Spacing After"
              value="8"
              unit="pt"
              step="6"
              min="0"
              style=${ribbonNarrowFieldStyle}
            ></mjx-measure-input>`,
            'word.layout.arrange.position': html`<mjx-button
              label="Position"
              size="small"
              data-opens="shell-word-layout-arrange-position"
            ></mjx-button>`,
            'word.layout.arrange.wrap-text': html`<mjx-button
              label="Wrap Text"
              icon="text-position-square"
              size="large"
              data-opens="shell-word-layout-arrange-wrap-text"
            ></mjx-button>`,
            'word.layout.arrange.bring-forward': html`<mjx-split-button
              label="Bring Forward"
              icon="position-forward"
              size="small"
              data-opens="shell-word-layout-arrange-bring-forward"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.layout.arrange.send-backward': html`<mjx-split-button
              label="Send Backward"
              icon="position-backward"
              size="small"
              data-opens="shell-word-layout-arrange-send-backward"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.layout.arrange.align': html`<mjx-button
              label="Align"
              icon="align-left"
              size="small"
              data-opens="shell-word-layout-arrange-align"
            ></mjx-button>`,
            'word.layout.arrange.group': html`<mjx-button
              label="Group"
              icon="group"
              size="small"
              data-opens="shell-word-layout-arrange-group"
            ></mjx-button>`,
            'word.layout.arrange.rotate': html`<mjx-button
              label="Rotate"
              icon="rotate-right"
              size="small"
              data-opens="shell-word-layout-arrange-rotate"
            ></mjx-button>`,
            // References (unit 6). Dropdowns and Next Footnote's split button open their menus from
            // `stories/ribbons/references-transitions-formulas-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            // Style is a dropdown field over `ribbon-parts.ts`'s list. Insert Footnote is the generic button: Office draws no arrow.
            'word.references.table-of-contents.table-of-contents': html`<mjx-button
              label="Table of Contents"
              icon="document-bullet-list"
              size="small"
              data-opens="shell-word-references-table-of-contents-table-of-contents"
            ></mjx-button>`,
            'word.references.table-of-contents.add-text': html`<mjx-button
              label="Add Text"
              size="small"
              data-opens="shell-word-references-table-of-contents-add-text"
            ></mjx-button>`,
            'word.references.footnotes.next-footnote': html`<mjx-split-button
              label="Next Footnote"
              size="small"
              data-opens="shell-word-references-footnotes-next-footnote"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.references.citations-bibliography.insert-citation': html`<mjx-button
              label="Insert Citation"
              icon="text-quote"
              size="large"
              data-opens="shell-word-references-citations-bibliography-insert-citation"
            ></mjx-button>`,
            'word.references.citations-bibliography.style': html`<mjx-dropdown
              id="word-citation-style"
              label="Style"
              value="apa"
              style=${ribbonNarrowFieldStyle}
            >
              ${citationStyles.map(
                (style) => html`<mjx-option value=${style.value} label=${style.label}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'word.references.citations-bibliography.bibliography': html`<mjx-button
              label="Bibliography"
              size="small"
              data-opens="shell-word-references-citations-bibliography-bibliography"
            ></mjx-button>`,
            // Mailings (unit 7). Dropdowns and Insert Merge Field's split button open their menus from
            // `stories/ribbons/mailings-animations-data-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            // Go to Record is a combo box over `ribbon-parts.ts`'s list: a record number is not a measure.
            'word.mailings.start-mail-merge.start-mail-merge': html`<mjx-button
              label="Start Mail Merge"
              icon="mail-multiple"
              size="small"
              data-opens="shell-word-mailings-start-mail-merge-start-mail-merge"
            ></mjx-button>`,
            'word.mailings.start-mail-merge.select-recipients': html`<mjx-button
              label="Select Recipients"
              icon="people-list"
              size="small"
              data-opens="shell-word-mailings-start-mail-merge-select-recipients"
            ></mjx-button>`,
            'word.mailings.write-insert-fields.insert-merge-field': html`<mjx-split-button
              label="Insert Merge Field"
              size="small"
              data-opens="shell-word-mailings-write-insert-fields-insert-merge-field"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.mailings.write-insert-fields.rules': html`<mjx-button
              label="Rules"
              size="small"
              data-opens="shell-word-mailings-write-insert-fields-rules"
            ></mjx-button>`,
            'word.mailings.preview-results.go-to-record': html`<mjx-combo-box
              id="word-go-to-record"
              label="Go to Record"
              value="1"
              allow-custom
              style=${ribbonNarrowFieldStyle}
            >
              ${mergeRecordNumbers.map((record) => html`<mjx-option value=${record} label=${record}></mjx-option>`)}
            </mjx-combo-box>`,
            'word.mailings.finish.finish-merge': html`<mjx-button
              label="Finish & Merge"
              size="small"
              data-opens="shell-word-mailings-finish-finish-merge"
            ></mjx-button>`,
            // Review (unit 8, Word alone). Split buttons and dropdowns open their menus from
            // `stories/ribbons/review-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            // Display for Review is a dropdown field over `ribbon-parts.ts`'s list, starting on Simple Markup.
            'word.review.accessibility.check-accessibility': html`<mjx-split-button
              label="Check Accessibility"
              icon="accessibility-checkmark"
              size="small"
              data-opens="shell-word-review-accessibility-check-accessibility"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.language.translate': html`<mjx-button
              label="Translate"
              icon="translate"
              size="large"
              data-opens="shell-word-review-language-translate"
            ></mjx-button>`,
            'word.review.language.language': html`<mjx-button
              label="Language"
              icon="local-language"
              size="large"
              data-opens="shell-word-review-language-language"
            ></mjx-button>`,
            'word.review.comments.delete': html`<mjx-split-button
              label="Delete"
              icon="comment-dismiss"
              size="large"
              data-opens="shell-word-review-comments-delete"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.comments.show-comments': html`<mjx-split-button
              toggle
              pressed="true"
              label="Show Comments"
              icon="comment-multiple"
              size="small"
              data-opens="shell-word-review-comments-show-comments"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.tracking.track-changes': html`<mjx-split-button
              toggle
              label="Track Changes"
              icon="document-edit"
              size="large"
              data-opens="shell-word-review-tracking-track-changes"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.tracking.display-for-review': html`<mjx-dropdown
              id="word-display-for-review"
              label="Display for Review"
              value="simple-markup"
              style=${ribbonFieldStyle}
            >
              ${displayForReviewModes.map((mode) => html`<mjx-option value=${mode.value} label=${mode.label}></mjx-option>`)}
            </mjx-dropdown>`,
            'word.review.tracking.show-markup': html`<mjx-button
              label="Show Markup"
              size="small"
              data-opens="shell-word-review-tracking-show-markup"
            ></mjx-button>`,
            'word.review.tracking.reviewing-pane': html`<mjx-split-button
              label="Reviewing Pane"
              icon="panel-left-text"
              size="small"
              data-opens="shell-word-review-tracking-reviewing-pane"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.changes.accept': html`<mjx-split-button
              label="Accept"
              icon="document-checkmark"
              size="large"
              data-opens="shell-word-review-changes-accept"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.changes.reject': html`<mjx-split-button
              label="Reject"
              icon="document-dismiss"
              size="small"
              data-opens="shell-word-review-changes-reject"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.compare.compare': html`<mjx-button
              label="Compare"
              size="small"
              data-opens="shell-word-review-compare-compare"
            ></mjx-button>`,
            'word.review.protect.block-authors': html`<mjx-split-button
              label="Block Authors"
              icon="person-lock"
              size="large"
              data-opens="shell-word-review-protect-block-authors"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'word.review.ink.hide-ink': html`<mjx-split-button
              toggle
              label="Hide Ink"
              size="small"
              data-opens="shell-word-review-ink-hide-ink"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            // View (Word alone). Show's three are checkboxes, Navigation Pane ticked because this
            // shell draws the pane open. Switch Windows opens its menu from
            // `stories/ribbons/view-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            'word.view.show.ruler': html`<mjx-checkbox id="word-view-ruler" label="Ruler"></mjx-checkbox>`,
            'word.view.show.gridlines': html`<mjx-checkbox id="word-view-gridlines" label="Gridlines"></mjx-checkbox>`,
            'word.view.show.navigation-pane': html`<mjx-checkbox id="word-view-navigation-pane" label="Navigation Pane" checked="true"></mjx-checkbox>`,
            'word.view.window.switch-windows': html`<mjx-button
              label="Switch Windows"
              icon="window-multiple"
              size="large"
              data-opens="shell-word-view-window-switch-windows"
            ></mjx-button>`,
          },
        })}
        ${wordContextualSets()}
      </mjx-ribbon>
    `,
  );
}

// ── the workspace ────────────────────────────────────────────────────────────

const treeStyle = 'flex:1 1 auto;min-block-size:0;inline-size:100%;border:1px solid var(--theme-border)';

const marginStyle =
  'flex:0 0 auto;box-sizing:border-box;inline-size:22rem;max-inline-size:40%;min-inline-size:0;' +
  'padding:var(--mjx-density-gutter);padding-inline-start:0';

/** The page, its context menu, a selection with a mini toolbar, and the marked scrollbar. */
function pageArea(): TemplateResult {
  return canvasRow(
    canvasArea(
      'word-canvas',
      html`
      <mjx-context-menu id="word-context" style=${contextRegionStyle}>
        ${documentPlaceholder(
          'Page 4 of 20 — “Method”',
          html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
            ${selectionRun('word-selection', 'A selected sentence')} has a mini toolbar above it.
          </p>`,
        )}
        <mjx-menu slot="menu" label="Text" floating>
          <mjx-menu-item label="Cut" icon="cut" shortcut="Ctrl+X"></mjx-menu-item>
          <mjx-menu-item label="Copy" icon="copy" shortcut="Ctrl+C"></mjx-menu-item>
          <mjx-menu-item label="New Comment" icon="comment"></mjx-menu-item>
          <mjx-menu-separator></mjx-menu-separator>
          <mjx-menu-item label="Styles">
            <mjx-menu slot="submenu" label="Styles">
              <mjx-menu-item kind="radio" label="Normal" checked></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Heading 1"></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Quote"></mjx-menu-item>
            </mjx-menu>
          </mjx-menu-item>
          <mjx-menu-separator></mjx-menu-separator>
          <mjx-menu-section label="Review">
            <mjx-menu-item kind="checkbox" label="Track Changes" checked></mjx-menu-item>
            <mjx-menu-item kind="checkbox" label="Show Markup" checked></mjx-menu-item>
          </mjx-menu-section>
        </mjx-menu>
      </mjx-context-menu>
    `,
      html`<mjx-mini-toolbar
        id="word-mini"
        label="Formatting"
        for="word-selection"
        open
        .commands=${textCommands}
      ></mjx-mini-toolbar>`,
    ),
    html`<mjx-scrollbar
      id="word-scroll"
      label="Document"
      controls="word-canvas"
      pages="20"
      page-height="1100"
      viewport="700"
    >
      <mjx-scroll-mark kind="search" page="1" within="0.25" label="fidelity"></mjx-scroll-mark>
      <mjx-scroll-mark kind="search" page="6" within="0.5" label="fidelity"></mjx-scroll-mark>
      <mjx-scroll-mark kind="comment" page="4" label="Ask legal"></mjx-scroll-mark>
      <mjx-scroll-mark kind="comment" page="12" within="0.8" label="Reword"></mjx-scroll-mark>
      <mjx-scroll-mark kind="change" page="9" within="0.4" label="Inserted"></mjx-scroll-mark>
      <mjx-scroll-mark kind="change" page="17" within="0.1" label="Deleted"></mjx-scroll-mark>
    </mjx-scrollbar>`,
  );
}

/** The bar across the foot, and the readings a Word document actually carries. */
function foot(): TemplateResult {
  return statusBar(
    'word-status',
    'Document status',
    [
      { id: 'word-page', label: 'Page', value: '4 of 20', priority: 'essential' },
      { id: 'word-words', label: 'Words', value: '3,182', priority: 'standard' },
      { id: 'word-language', label: 'Language', value: 'English (UK)', priority: 'supplementary' },
      { id: 'word-track', label: 'Track Changes', value: 'On', priority: 'standard' },
      {
        id: 'word-section',
        label: 'Section',
        value: '2',
        priority: 'ancillary',
        region: 'centre',
      },
    ],
    zoom('word-zoom', { width: 794, height: 1123 }),
  );
}

// ── the desktop and tablet shells ────────────────────────────────────────────

/** Everything above the phone. Desktop and tablet are the same assembly at two widths. */
function wideShell(size: 'desktop' | 'tablet'): TemplateResult {
  const fraction = size === 'desktop' ? '0.18' : '0.22';
  return shellFrame(
    'word',
    size,
    ribbon(),
    surface(
      'workspace',
      workspaceStyle,
      html`
        ${navigatorPane(
          'word-nav-pane',
          fraction,
          html`<mjx-tree
            id="word-outline"
            label="Navigation"
            value="method-corpus"
            style=${treeStyle}
            .nodes=${outline}
            .expanded=${[...smallOutlineExpanded, 'findings']}
          ></mjx-tree>`,
        )}
        ${paneSplitter('word-split', 'word-nav-pane', fraction)}
        ${documentColumn(pageArea())}
        <div data-mjx-shell-surface="review-margin" style=${marginStyle}>
          <mjx-review-pane
            id="word-review"
            label="Revisions"
            side="inlineStart"
            selected="a3"
            style="display:block;block-size:100%;border:1px solid var(--theme-border);
                   border-radius:var(--radius-control)"
            .annotations=${annotations}
          ></mjx-review-pane>
        </div>
      `,
    ),
    foot(),
    html`
      <mjx-menu id="word-paste-menu" label="Paste options" floating>
        <mjx-menu-section label="Paste">
          <mjx-menu-item kind="radio" label="Keep Source Formatting" checked></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Merge Formatting"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Keep Text Only"></mjx-menu-item>
        </mjx-menu-section>
        <mjx-menu-separator></mjx-menu-separator>
        <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
        <mjx-menu-item label="Set Default Paste"></mjx-menu-item>
      </mjx-menu>
      ${insertMenus('word', 'shell')} ${drawMenus('word', 'shell')}
      ${designLayoutMenus('word', 'shell')} ${referencesTransitionsFormulasMenus('word', 'shell')}
      ${mailingsAnimationsDataMenus('word', 'shell')} ${reviewMenus('word', 'shell')}
      ${viewMenus('word', 'shell')}
      <mjx-dialog id="word-paragraph" label="Paragraph" modal>
        ${paneStack(
          field(
            'word-indent',
            'Indentation before text',
            html`<mjx-measure-input
              id="word-indent"
              value="0"
              unit="cm"
              step="0.25"
            ></mjx-measure-input>`,
          ),
          field(
            'word-after',
            'Space after',
            html`<mjx-measure-input id="word-after" value="8" unit="pt" step="1"></mjx-measure-input>`,
          ),
          field(
            'word-spacing',
            'Line spacing',
            html`<mjx-dropdown id="word-spacing" label="Line spacing" value="1.15">
              ${lineSpacingOptions.map(
                (option) => html`<mjx-option
                  value=${option.value}
                  label=${option.label}
                  description=${option.description ?? ''}
                  ?unavailable=${option.unavailable === true}
                  explanation=${option.explanation ?? ''}
                ></mjx-option>`,
              )}
            </mjx-dropdown>`,
          ),
          html`<mjx-checkbox
            id="word-keep"
            label="Keep with next"
            checked="mixed"
          ></mjx-checkbox>`,
        )}
      </mjx-dialog>
    `,
  );
}

// ── the stories ──────────────────────────────────────────────────────────────

/** The whole application at 1440, with both panes docked. */
export const Desktop: Story = {
  globals: { containerPreset: 'desktop' },
  render: () => wideShell('desktop'),
};

/**
 * **The middle size, and Word's version of it is the hardest of the three.**
 *
 * A navigation pane on one side and a review margin on the other, at 834, leave the page a column.
 * Whether that is acceptable — or whether one of the two should become an overlay here — is a
 * decision for MJXOFF-195, and it is stated in `ui/README.md` rather than taken silently.
 */
export const Tablet: Story = {
  globals: { containerPreset: 'tablet' },
  render: () => wideShell('tablet'),
};

/**
 * **The phone.** No ribbon, no panes: a command rail, a contextual action bar for the selection, and
 * a sheet. Press **Styles** on the rail and the sheet arrives at its half detent.
 *
 * The review pane is here as well, and in its **other** presentation: below the phone-shell width it
 * is a list in flow rather than a packed margin, so the same component that shows two cards beside a
 * desktop page shows every annotation here. Word Mobile's comments view is exactly that, and it is
 * the only place in these nine shells where all three card kinds are on screen at once.
 */
export const Mobile: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    shellFrame(
      'word',
      'mobile',
      phoneBody(
        canvasArea(
          'word-phone-canvas',
          documentPlaceholder(
            'Page 4 of 20 — “Method”',
            html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
              ${selectionRun('word-phone-selection', 'twelve selected words')} is what the bar below
              is about.
            </p>`,
          ),
        ),
        html`<mjx-review-pane
          id="word-phone-review"
          label="Revisions"
          side="inlineStart"
          selected="a2"
          style="display:block;flex:0 0 auto;block-size:16rem;
                 border:1px solid var(--theme-border);border-radius:var(--radius-control)"
          .annotations=${annotations}
        ></mjx-review-pane>`,
      ),
      phoneRails(
        html`<mjx-contextual-action-bar
          id="word-phone-selection-bar"
          selection="text"
          selection-label="12 words"
        ></mjx-contextual-action-bar>`,
        html`<mjx-command-bar
          id="word-phone-commands"
          label="Home"
          .commands=${wordPhoneCommands}
          @mjx-mobile-command=${openSheetOnCommand('styles', 'word-phone-sheet')}
        ></mjx-command-bar>`,
      ),
      html`<mjx-dialog id="word-phone-sheet" label="Styles" modal detent="half">
        <mjx-gallery id="word-phone-styles" label="Styles" value="normal">
          ${styleGalleryItems()}
        </mjx-gallery>
      </mjx-dialog>`,
    ),
};
