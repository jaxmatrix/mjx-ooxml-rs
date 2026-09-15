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
  citationStyles,
  copyCounts,
  displayForReviewModes,
  mergeRecordNumbers,
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
import { colourPickerEntries, fillEntries, outlineEntries } from './colour-picker-entries.ts';
import { designLayoutMenus, styleSetGalleryItems } from './design-layout-menus.ts';
import { drawMenus } from './draw-menus.ts';
import { insertMenus } from './insert-menus.ts';
import { mailingsAnimationsDataMenus } from './mailings-animations-data-menus.ts';
import { outlineLevels, showLevels } from './outlining-menus.ts';
import { printPreviewMenus } from './print-preview-menus.ts';
import { referencesTransitionsFormulasMenus } from './references-transitions-formulas-menus.ts';
import { reviewMenus } from './review-menus.ts';
import {
  tableLineWeights,
  tableToolsMenus,
  wordBorderLineStyles,
  wordTableStyleGalleryFooter,
  wordTableStyleGalleryItems,
} from './table-tools-menus.ts';
import { pictureStyleGalleryItems, pictureToolsMenus, wordPictureMeasures } from './picture-tools-menus.ts';
import {
  drawingToolsMenus,
  shapeFillEntryOptions,
  shapeOutlineEntryOptions,
  shapeStyleGalleryItems,
  wordShapeMeasures,
} from './drawing-tools-menus.ts';
import {
  chartOutlineEntryOptions,
  chartSelectionOptions,
  chartStyleGalleryItems,
  chartTextFillEntryOptions,
  chartTextOutlineEntryOptions,
  chartToolsMenus,
  wordChartMeasures,
} from './chart-tools-menus.ts';
import { viewMenus } from './view-menus.ts';
import { wordArtStyleGalleryFooter, wordArtStyleGalleryItems } from './wordart-styles-menus.ts';
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
 * **Every core and view tab is authored**: File, Home, Insert, Draw, Design, Layout, References, Mailings, Review,
 * View, Outlining, Print Preview and Background Removal. **All six contextual tabs are authored**: Table Design, Table
 * Tools' Layout, Picture Format, Shape Format, Chart Design and Chart Tools' Format. Each was a placeholder until its
 * unit — one group carrying the tab's name, at the priority `dev/ribbons/census.ts` declares for it, holding one button
 * that says so. That is the shape unit 0 gave every core tab: the census transcribed, the ladder already right and every
 * tab present, so each later unit is a small diff rather than a new file.
 *
 * **Every story draws all four contextual sets**, as every story draws the view tabs, so a contextual tab can be
 * reached from any story; Office shows one set at a time, and `Shell/Word` draws Table Tools alone.
 *
 * The placeholder button says *Not yet authored* rather than naming a plausible command, for the
 * reason `dev/word-tab-home.ts` gives about its own filler: a made-up command name is a worse lie
 * than an obvious placeholder, and a placeholder occupies exactly as much of the layout as a
 * command does.
 *
 * **Nothing here dispatches a command.** The paste button's menu opens, the Insert, Draw, Design, Layout, References,
 * Mailings, Review, View, Print Preview, Table Design, Table Layout, Picture Format, Shape Format, Chart Design and Chart
 * Format tabs' menus open,
 * the pickers open, the
 * galleries preview — and no document changes, because command dispatch is loop 2.
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
          'Word’s twelve core tabs, its File tab and its six contextual tabs, each shown selected inside the whole ' +
          'ribbon. Every core and view tab is authored: File, Home, Insert, Draw, Design, Layout, References, ' +
          'Mailings, Review, View, Outlining, Print Preview and Background Removal. Of the contextual tabs of the ' +
          'four common sets, all six are authored: Table Design, Layout, Picture Format, Shape Format, Chart Design ' +
          'and Chart Tools’ Format.',
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
  // Draw (unit 4). Office draws five of these as a dropdown and Eraser as a split button, so each
  // opens its menu from `stories/ribbons/draw-menus.ts`. `data-opens` is
  // `commandSurfaceId('ribbons', <this key>)`, and `tests/ribbons.test.ts` requires exactly that.
  'word.draw.drawing-tools.add-pen': html`<mjx-button
    label="Add Pen"
    size="small"
    data-opens="ribbons-word-draw-drawing-tools-add-pen"
  ></mjx-button>`,
  'word.draw.pens.pens': html`<mjx-button
    label="Pens"
    icon="inking-tool"
    size="large"
    data-opens="ribbons-word-draw-pens-pens"
  ></mjx-button>`,
  'word.draw.pens.colour': html`<mjx-button
    label="Colour"
    icon="color-line"
    size="small"
    data-opens="ribbons-word-draw-pens-colour"
  ></mjx-button>`,
  'word.draw.pens.thickness': html`<mjx-button
    label="Thickness"
    icon="line-thickness"
    size="small"
    data-opens="ribbons-word-draw-pens-thickness"
  ></mjx-button>`,
  'word.draw.write.eraser': html`<mjx-split-button
    toggle
    exclusive="word.draw.write.tools"
    label="Eraser"
    icon="eraser"
    size="large"
    data-opens="ribbons-word-draw-write-eraser"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.draw.input-mode.touch-mouse-mode': html`<mjx-button
    label="Touch/Mouse Mode"
    size="small"
    data-opens="ribbons-word-draw-input-mode-touch-mouse-mode"
  ></mjx-button>`,
  // Design and Layout (unit 5). Dropdowns and split buttons open their menus from
  // `stories/ribbons/design-layout-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // The Style Set is an in-ribbon gallery, Page Colour a colour picker, and Indent and Spacing are measures:
  // `value` is always points, so an 8 pt Spacing After is 8 and a 0 cm indent is 0.
  'word.design.style-set.themes': html`<mjx-button
    label="Themes"
    size="small"
    data-opens="ribbons-word-design-style-set-themes"
  ></mjx-button>`,
  'word.design.style-set.style-set': html`<mjx-gallery
    id="ribbons-word-style-set"
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
    data-opens="ribbons-word-design-style-set-colours"
  ></mjx-button>`,
  'word.design.style-set.fonts': html`<mjx-button
    label="Fonts"
    icon="text-font"
    size="large"
    data-opens="ribbons-word-design-style-set-fonts"
  ></mjx-button>`,
  'word.design.style-set.paragraph-spacing': html`<mjx-button
    label="Paragraph Spacing"
    icon="text-line-spacing"
    size="small"
    data-opens="ribbons-word-design-style-set-paragraph-spacing"
  ></mjx-button>`,
  'word.design.style-set.effects': html`<mjx-button
    label="Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-word-design-style-set-effects"
  ></mjx-button>`,
  'word.design.page-background.watermark': html`<mjx-button
    label="Watermark"
    size="small"
    data-opens="ribbons-word-design-page-background-watermark"
  ></mjx-button>`,
  'word.design.page-background.page-colour': html`<mjx-color-picker
    id="ribbons-word-page-colour"
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
    data-opens="ribbons-word-layout-page-setup-margins"
  ></mjx-button>`,
  'word.layout.page-setup.orientation': html`<mjx-button
    label="Orientation"
    icon="orientation"
    size="large"
    data-opens="ribbons-word-layout-page-setup-orientation"
  ></mjx-button>`,
  'word.layout.page-setup.size': html`<mjx-button
    label="Size"
    size="small"
    data-opens="ribbons-word-layout-page-setup-size"
  ></mjx-button>`,
  'word.layout.page-setup.columns': html`<mjx-button
    label="Columns"
    icon="text-column-two"
    size="large"
    data-opens="ribbons-word-layout-page-setup-columns"
  ></mjx-button>`,
  'word.layout.page-setup.breaks': html`<mjx-button
    label="Breaks"
    icon="document-page-break"
    size="small"
    data-opens="ribbons-word-layout-page-setup-breaks"
  ></mjx-button>`,
  'word.layout.page-setup.line-numbers': html`<mjx-button
    label="Line Numbers"
    size="small"
    data-opens="ribbons-word-layout-page-setup-line-numbers"
  ></mjx-button>`,
  'word.layout.page-setup.hyphenation': html`<mjx-button
    label="Hyphenation"
    size="small"
    data-opens="ribbons-word-layout-page-setup-hyphenation"
  ></mjx-button>`,
  'word.layout.paragraph.indent-left': html`<mjx-measure-input
    id="ribbons-word-indent-left"
    label="Indent Left"
    value="0"
    unit="cm"
    step="0.25"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'word.layout.paragraph.indent-right': html`<mjx-measure-input
    id="ribbons-word-indent-right"
    label="Indent Right"
    value="0"
    unit="cm"
    step="0.25"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'word.layout.paragraph.spacing-before': html`<mjx-measure-input
    id="ribbons-word-spacing-before"
    label="Spacing Before"
    value="0"
    unit="pt"
    step="6"
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'word.layout.paragraph.spacing-after': html`<mjx-measure-input
    id="ribbons-word-spacing-after"
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
    data-opens="ribbons-word-layout-arrange-position"
  ></mjx-button>`,
  'word.layout.arrange.wrap-text': html`<mjx-button
    label="Wrap Text"
    icon="text-position-square"
    size="large"
    data-opens="ribbons-word-layout-arrange-wrap-text"
  ></mjx-button>`,
  'word.layout.arrange.bring-forward': html`<mjx-split-button
    label="Bring Forward"
    icon="position-forward"
    size="small"
    data-opens="ribbons-word-layout-arrange-bring-forward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.layout.arrange.send-backward': html`<mjx-split-button
    label="Send Backward"
    icon="position-backward"
    size="small"
    data-opens="ribbons-word-layout-arrange-send-backward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.layout.arrange.align': html`<mjx-button
    label="Align"
    icon="align-left"
    size="small"
    data-opens="ribbons-word-layout-arrange-align"
  ></mjx-button>`,
  'word.layout.arrange.group': html`<mjx-button
    label="Group"
    icon="group"
    size="small"
    data-opens="ribbons-word-layout-arrange-group"
  ></mjx-button>`,
  'word.layout.arrange.rotate': html`<mjx-button
    label="Rotate"
    icon="rotate-right"
    size="small"
    data-opens="ribbons-word-layout-arrange-rotate"
  ></mjx-button>`,
  // References (unit 6). Dropdowns and Next Footnote's split button open their menus from
  // `stories/ribbons/references-transitions-formulas-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // Style is a dropdown field over `ribbon-parts.ts`'s list. Insert Footnote is the generic button: Office draws no arrow.
  'word.references.table-of-contents.table-of-contents': html`<mjx-button
    label="Table of Contents"
    icon="document-bullet-list"
    size="small"
    data-opens="ribbons-word-references-table-of-contents-table-of-contents"
  ></mjx-button>`,
  'word.references.table-of-contents.add-text': html`<mjx-button
    label="Add Text"
    size="small"
    data-opens="ribbons-word-references-table-of-contents-add-text"
  ></mjx-button>`,
  'word.references.footnotes.next-footnote': html`<mjx-split-button
    label="Next Footnote"
    size="small"
    data-opens="ribbons-word-references-footnotes-next-footnote"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.references.citations-bibliography.insert-citation': html`<mjx-button
    label="Insert Citation"
    icon="text-quote"
    size="large"
    data-opens="ribbons-word-references-citations-bibliography-insert-citation"
  ></mjx-button>`,
  'word.references.citations-bibliography.style': html`<mjx-dropdown
    id="ribbons-word-citation-style"
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
    data-opens="ribbons-word-references-citations-bibliography-bibliography"
  ></mjx-button>`,
  // Mailings (unit 7). Dropdowns and Insert Merge Field's split button open their menus from
  // `stories/ribbons/mailings-animations-data-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // Go to Record is a combo box over `ribbon-parts.ts`'s list: a record number is not a measure.
  'word.mailings.start-mail-merge.start-mail-merge': html`<mjx-button
    label="Start Mail Merge"
    icon="mail-multiple"
    size="small"
    data-opens="ribbons-word-mailings-start-mail-merge-start-mail-merge"
  ></mjx-button>`,
  'word.mailings.start-mail-merge.select-recipients': html`<mjx-button
    label="Select Recipients"
    icon="people-list"
    size="small"
    data-opens="ribbons-word-mailings-start-mail-merge-select-recipients"
  ></mjx-button>`,
  'word.mailings.write-insert-fields.insert-merge-field': html`<mjx-split-button
    label="Insert Merge Field"
    size="small"
    data-opens="ribbons-word-mailings-write-insert-fields-insert-merge-field"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.mailings.write-insert-fields.rules': html`<mjx-button
    label="Rules"
    size="small"
    data-opens="ribbons-word-mailings-write-insert-fields-rules"
  ></mjx-button>`,
  'word.mailings.preview-results.go-to-record': html`<mjx-combo-box
    id="ribbons-word-go-to-record"
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
    data-opens="ribbons-word-mailings-finish-finish-merge"
  ></mjx-button>`,
  // Review (unit 8, Word alone). Split buttons and dropdowns open their menus from
  // `stories/ribbons/review-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // Display for Review is a dropdown field over `ribbon-parts.ts`'s list, starting on Simple Markup.
  'word.review.accessibility.check-accessibility': html`<mjx-split-button
    label="Check Accessibility"
    icon="accessibility-checkmark"
    size="small"
    data-opens="ribbons-word-review-accessibility-check-accessibility"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.language.translate': html`<mjx-button
    label="Translate"
    icon="translate"
    size="large"
    data-opens="ribbons-word-review-language-translate"
  ></mjx-button>`,
  'word.review.language.language': html`<mjx-button
    label="Language"
    icon="local-language"
    size="large"
    data-opens="ribbons-word-review-language-language"
  ></mjx-button>`,
  'word.review.comments.delete': html`<mjx-split-button
    label="Delete"
    icon="comment-dismiss"
    size="large"
    data-opens="ribbons-word-review-comments-delete"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.comments.show-comments': html`<mjx-split-button
    toggle
    pressed="true"
    label="Show Comments"
    icon="comment-multiple"
    size="small"
    data-opens="ribbons-word-review-comments-show-comments"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.tracking.track-changes': html`<mjx-split-button
    toggle
    label="Track Changes"
    icon="document-edit"
    size="large"
    data-opens="ribbons-word-review-tracking-track-changes"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.tracking.display-for-review': html`<mjx-dropdown
    id="ribbons-word-display-for-review"
    label="Display for Review"
    value="simple-markup"
    style=${ribbonFieldStyle}
  >
    ${displayForReviewModes.map((mode) => html`<mjx-option value=${mode.value} label=${mode.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'word.review.tracking.show-markup': html`<mjx-button
    label="Show Markup"
    size="small"
    data-opens="ribbons-word-review-tracking-show-markup"
  ></mjx-button>`,
  'word.review.tracking.reviewing-pane': html`<mjx-split-button
    label="Reviewing Pane"
    icon="panel-left-text"
    size="small"
    data-opens="ribbons-word-review-tracking-reviewing-pane"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.changes.accept': html`<mjx-split-button
    label="Accept"
    icon="document-checkmark"
    size="large"
    data-opens="ribbons-word-review-changes-accept"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.changes.reject': html`<mjx-split-button
    label="Reject"
    icon="document-dismiss"
    size="small"
    data-opens="ribbons-word-review-changes-reject"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.compare.compare': html`<mjx-button
    label="Compare"
    size="small"
    data-opens="ribbons-word-review-compare-compare"
  ></mjx-button>`,
  'word.review.protect.block-authors': html`<mjx-split-button
    label="Block Authors"
    icon="person-lock"
    size="large"
    data-opens="ribbons-word-review-protect-block-authors"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.review.ink.hide-ink': html`<mjx-split-button
    toggle
    label="Hide Ink"
    size="small"
    data-opens="ribbons-word-review-ink-hide-ink"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  // View (Word alone). Show's three are checkboxes, Navigation Pane ticked as the census declares.
  // Switch Windows opens its menu from `stories/ribbons/view-menus.ts`, and `data-opens` is
  // `commandSurfaceId('ribbons', <this key>)`.
  'word.view.show.ruler': html`<mjx-checkbox id="ribbons-word-view-ruler" label="Ruler"></mjx-checkbox>`,
  'word.view.show.gridlines': html`<mjx-checkbox id="ribbons-word-view-gridlines" label="Gridlines"></mjx-checkbox>`,
  'word.view.show.navigation-pane': html`<mjx-checkbox id="ribbons-word-view-navigation-pane" label="Navigation Pane" checked="true"></mjx-checkbox>`,
  'word.view.window.switch-windows': html`<mjx-button
    label="Switch Windows"
    icon="window-multiple"
    size="large"
    data-opens="ribbons-word-view-window-switch-windows"
  ></mjx-button>`,
  // Outlining (Word alone, and a view tab). `Shell/Word` never draws a view tab, so these four bindings are
  // written here and nowhere else. Outline Level and Show Level are dropdown fields over
  // `stories/ribbons/outlining-menus.ts`; Show Text Formatting (ticked, as the census declares) and Show First
  // Line Only are checkboxes. The tab opens no menu.
  'word.outlining.outlining-tools.outline-level': html`<mjx-dropdown
    id="ribbons-word-outlining-outline-level"
    label="Outline Level"
    value="body-text"
    style=${ribbonNarrowFieldStyle}
  >
    ${outlineLevels.map((level) => html`<mjx-option value=${level.value} label=${level.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'word.outlining.outlining-tools.show-level': html`<mjx-dropdown
    id="ribbons-word-outlining-show-level"
    label="Show Level"
    value="all-levels"
    style=${ribbonNarrowFieldStyle}
  >
    ${showLevels.map((level) => html`<mjx-option value=${level.value} label=${level.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'word.outlining.outlining-tools.show-text-formatting': html`<mjx-checkbox id="ribbons-word-outlining-show-text-formatting" label="Show Text Formatting" checked="true"></mjx-checkbox>`,
  'word.outlining.outlining-tools.show-first-line-only': html`<mjx-checkbox id="ribbons-word-outlining-show-first-line-only" label="Show First Line Only"></mjx-checkbox>`,
  // Print Preview (Word alone here, and a view tab). `Shell/Word` never draws a view tab, so these five
  // bindings and the three menus they open are written here and nowhere else. Margins, Orientation and Size
  // open Layout's own lists through `stories/ribbons/print-preview-menus.ts`; Show Ruler and Magnifier
  // (ticked, as the census declares) are checkboxes.
  'word.print-preview.page-setup.margins': html`<mjx-button
    label="Margins"
    icon="document-margins"
    size="large"
    data-opens="ribbons-word-print-preview-page-setup-margins"
  ></mjx-button>`,
  'word.print-preview.page-setup.orientation': html`<mjx-button
    label="Orientation"
    icon="orientation"
    size="large"
    data-opens="ribbons-word-print-preview-page-setup-orientation"
  ></mjx-button>`,
  'word.print-preview.page-setup.size': html`<mjx-button
    label="Size"
    size="small"
    data-opens="ribbons-word-print-preview-page-setup-size"
  ></mjx-button>`,
  'word.print-preview.preview.show-ruler': html`<mjx-checkbox id="ribbons-word-print-preview-show-ruler" label="Show Ruler"></mjx-checkbox>`,
  'word.print-preview.preview.magnifier': html`<mjx-checkbox id="ribbons-word-print-preview-magnifier" label="Magnifier" checked="true"></mjx-checkbox>`,
  // Table Design (a contextual tab, in Table Tools). Both Word hosts draw Table Tools, so `Shell/Word` binds the same
  // thirteen commands under its own ids. Table Style Options' six are checkboxes; the gallery and the two fields are
  // filled from `stories/ribbons/table-tools-menus.ts`, the pickers from the document's palette; Border Styles and
  // Borders open their menus from that file. Border Painter is the generic toggle and is not bound.
  'word.table-design.table-style-options.header-row': html`<mjx-checkbox id="ribbons-word-table-design-header-row" label="Header Row" checked="true"></mjx-checkbox>`,
  'word.table-design.table-style-options.total-row': html`<mjx-checkbox id="ribbons-word-table-design-total-row" label="Total Row"></mjx-checkbox>`,
  'word.table-design.table-style-options.banded-rows': html`<mjx-checkbox id="ribbons-word-table-design-banded-rows" label="Banded Rows" checked="true"></mjx-checkbox>`,
  'word.table-design.table-style-options.first-column': html`<mjx-checkbox id="ribbons-word-table-design-first-column" label="First Column" checked="true"></mjx-checkbox>`,
  'word.table-design.table-style-options.last-column': html`<mjx-checkbox id="ribbons-word-table-design-last-column" label="Last Column"></mjx-checkbox>`,
  'word.table-design.table-style-options.banded-columns': html`<mjx-checkbox id="ribbons-word-table-design-banded-columns" label="Banded Columns"></mjx-checkbox>`,
  'word.table-design.table-styles.gallery': html`<mjx-gallery
    id="ribbons-word-table-styles"
    label="Table Styles"
    value="table-grid"
    style=${ribbonGalleryStyle}
  >
    ${wordTableStyleGalleryItems(documentThemePalette)} ${wordTableStyleGalleryFooter()}
  </mjx-gallery>`,
  'word.table-design.table-styles.shading': html`<mjx-color-picker
    id="ribbons-word-table-design-shading"
    style=${ribbonColourFieldStyle}
    label="Shading"
    show-no-fill
    no-fill-label="No Colour"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Shading', fillEntries({ moreColours: 'More Colours…' }))}
  </mjx-color-picker>`,
  'word.table-design.borders.border-styles': html`<mjx-button
    label="Border Styles"
    icon="line-style"
    size="large"
    data-opens="ribbons-word-table-design-borders-border-styles"
  ></mjx-button>`,
  'word.table-design.borders.line-style': html`<mjx-dropdown
    id="ribbons-word-table-design-line-style"
    label="Line Style"
    value="single"
    style=${ribbonFieldStyle}
  >
    ${wordBorderLineStyles.map((style) => html`<mjx-option value=${style.value} label=${style.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'word.table-design.borders.line-weight': html`<mjx-dropdown
    id="ribbons-word-table-design-line-weight"
    label="Line Weight"
    value="0.5"
    style=${ribbonNarrowFieldStyle}
  >
    ${tableLineWeights.map((weight) => html`<mjx-option value=${weight.value} label=${weight.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'word.table-design.borders.pen-colour': html`<mjx-color-picker
    id="ribbons-word-table-design-pen-colour"
    style=${ribbonColourFieldStyle}
    label="Pen Colour"
    show-automatic
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Pen Colour', outlineEntries({ moreColours: 'More Colours…' }))}
  </mjx-color-picker>`,
  'word.table-design.borders.borders': html`<mjx-split-button
    label="Borders"
    icon="border-all"
    size="large"
    menu-label="Borders"
    data-opens="ribbons-word-table-design-borders-borders"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  // Table Layout (a contextual tab, in Table Tools). Both Word hosts draw Table Tools, so `Shell/Word` binds the same
  // five commands under its own ids. Select, Delete and AutoFit open their menus from
  // `stories/ribbons/table-tools-menus.ts`; Height and Width are measures in centimetres. Every other command, both
  // exclusive sets included, is the generic toggle or button and is not bound.
  'word.table-layout.table.select': html`<mjx-button
    label="Select"
    icon="table-cursor"
    size="small"
    data-opens="ribbons-word-table-layout-table-select"
  ></mjx-button>`,
  'word.table-layout.rows-and-columns.delete': html`<mjx-button
    label="Delete"
    icon="table-dismiss"
    size="large"
    data-opens="ribbons-word-table-layout-rows-and-columns-delete"
  ></mjx-button>`,
  'word.table-layout.cell-size.autofit': html`<mjx-button
    label="AutoFit"
    icon="arrow-autofit-content"
    size="large"
    data-opens="ribbons-word-table-layout-cell-size-autofit"
  ></mjx-button>`,
  'word.table-layout.cell-size.height': html`<mjx-measure-input
    id="ribbons-word-table-layout-height"
    label="Height"
    value="0.5"
    unit="cm"
    step="0.1"
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'word.table-layout.cell-size.width': html`<mjx-measure-input
    id="ribbons-word-table-layout-width"
    label="Width"
    value="3.18"
    unit="cm"
    step="0.1"
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  // Picture Format (a contextual tab, in Picture Tools). `Shell/Word` draws Table Tools alone, so these twenty bindings
  // and the sixteen menus they open are written here and nowhere else. Every menu, the gallery's styles and the two
  // starting measures are `stories/ribbons/picture-tools-menus.ts`'s; Arrange's menus are Layout's own lists under this
  // tab's ids. Remove Background, Compress Pictures, Alt Text, Selection Pane and Play Animation are the generic button
  // or toggle and are not bound.
  'word.picture-format.adjust.corrections': html`<mjx-button
    label="Corrections"
    icon="brightness-high"
    size="large"
    data-opens="ribbons-word-picture-format-adjust-corrections"
  ></mjx-button>`,
  'word.picture-format.adjust.colour': html`<mjx-button
    label="Colour"
    icon="color"
    size="large"
    data-opens="ribbons-word-picture-format-adjust-colour"
  ></mjx-button>`,
  'word.picture-format.adjust.artistic-effects': html`<mjx-button
    label="Artistic Effects"
    icon="photo-filter"
    size="large"
    data-opens="ribbons-word-picture-format-adjust-artistic-effects"
  ></mjx-button>`,
  'word.picture-format.adjust.transparency': html`<mjx-button
    label="Transparency"
    icon="transparency-square"
    size="large"
    data-opens="ribbons-word-picture-format-adjust-transparency"
  ></mjx-button>`,
  'word.picture-format.adjust.change-picture': html`<mjx-button
    label="Change Picture"
    icon="image-arrow-forward"
    size="small"
    data-opens="ribbons-word-picture-format-adjust-change-picture"
  ></mjx-button>`,
  'word.picture-format.adjust.reset-picture': html`<mjx-split-button
    label="Reset Picture"
    icon="image-arrow-counterclockwise"
    size="small"
    menu-label="Reset Picture"
    data-opens="ribbons-word-picture-format-adjust-reset-picture"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.picture-format.picture-styles.quick-styles': html`<mjx-gallery
    id="ribbons-word-picture-format-quick-styles"
    label="Quick Styles"
    style=${ribbonGalleryStyle}
  >
    ${pictureStyleGalleryItems(documentThemePalette)}
  </mjx-gallery>`,
  'word.picture-format.picture-styles.picture-border': html`<mjx-color-picker
    id="ribbons-word-picture-format-picture-border"
    style=${ribbonColourFieldStyle}
    label="Picture Border"
    show-no-fill
    no-fill-label="No Outline"
    value="none"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries(
      'Picture Border',
      outlineEntries({ moreColours: 'More Outline Colours…', weight: true, sketched: true, dashes: true }),
    )}
  </mjx-color-picker>`,
  'word.picture-format.picture-styles.picture-effects': html`<mjx-button
    label="Picture Effects"
    icon="image-shadow"
    size="small"
    data-opens="ribbons-word-picture-format-picture-styles-picture-effects"
  ></mjx-button>`,
  'word.picture-format.picture-styles.picture-layout': html`<mjx-button
    label="Picture Layout"
    icon="diagram"
    size="small"
    data-opens="ribbons-word-picture-format-picture-styles-picture-layout"
  ></mjx-button>`,
  'word.picture-format.arrange.position': html`<mjx-button
    label="Position"
    size="small"
    data-opens="ribbons-word-picture-format-arrange-position"
  ></mjx-button>`,
  'word.picture-format.arrange.wrap-text': html`<mjx-button
    label="Wrap Text"
    icon="text-position-square"
    size="large"
    data-opens="ribbons-word-picture-format-arrange-wrap-text"
  ></mjx-button>`,
  'word.picture-format.arrange.bring-forward': html`<mjx-split-button
    label="Bring Forward"
    icon="position-forward"
    size="small"
    data-opens="ribbons-word-picture-format-arrange-bring-forward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.picture-format.arrange.send-backward': html`<mjx-split-button
    label="Send Backward"
    icon="position-backward"
    size="small"
    data-opens="ribbons-word-picture-format-arrange-send-backward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.picture-format.arrange.align': html`<mjx-button
    label="Align"
    icon="align-left"
    size="small"
    data-opens="ribbons-word-picture-format-arrange-align"
  ></mjx-button>`,
  'word.picture-format.arrange.group': html`<mjx-button
    label="Group"
    icon="group"
    size="small"
    data-opens="ribbons-word-picture-format-arrange-group"
  ></mjx-button>`,
  'word.picture-format.arrange.rotate': html`<mjx-button
    label="Rotate"
    icon="rotate-right"
    size="small"
    data-opens="ribbons-word-picture-format-arrange-rotate"
  ></mjx-button>`,
  'word.picture-format.size.crop': html`<mjx-split-button
    toggle
    label="Crop"
    icon="crop"
    size="large"
    menu-label="Crop"
    data-opens="ribbons-word-picture-format-size-crop"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.picture-format.size.height': html`<mjx-measure-input
    id="ribbons-word-picture-format-height"
    label="Height"
    value=${wordPictureMeasures.height}
    unit="cm"
    step=${wordPictureMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'word.picture-format.size.width': html`<mjx-measure-input
    id="ribbons-word-picture-format-width"
    label="Width"
    value=${wordPictureMeasures.width}
    unit="cm"
    step=${wordPictureMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  // Shape Format (a contextual tab, in Drawing Tools). `Shell/Word` draws Table Tools alone, so these twenty-two
  // bindings and `drawingToolsMenus('word', …)` are written here and nowhere else. Every menu, the Theme Styles
  // gallery, the entry options and the starting measures are `stories/ribbons/drawing-tools-menus.ts`'s; the WordArt
  // gallery is `stories/ribbons/wordart-styles-menus.ts`'; the four pickers and both galleries' pictures read this
  // document's palette. Other Theme Fills, under the Theme Styles gallery, opens the menu declared for the gallery's own
  // command. Create Link is the generic button, and Alt Text and Selection Pane the generic toggle; none is bound.
  'word.shape-format.insert-shapes.shapes': html`<mjx-button
    label="Shapes"
    icon="shapes"
    size="large"
    data-opens="ribbons-word-shape-format-insert-shapes-shapes"
  ></mjx-button>`,
  'word.shape-format.insert-shapes.edit-shape': html`<mjx-button
    label="Edit Shape"
    icon="bezier-curve-square"
    size="small"
    data-opens="ribbons-word-shape-format-insert-shapes-edit-shape"
  ></mjx-button>`,
  'word.shape-format.insert-shapes.draw-text-box': html`<mjx-split-button
    label="Draw Text Box"
    icon="textbox"
    size="small"
    menu-label="Draw Text Box"
    data-opens="ribbons-word-shape-format-insert-shapes-draw-text-box"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.shape-format.shape-styles.theme-styles': html`<mjx-gallery
    id="ribbons-word-shape-format-theme-styles"
    label="Theme Styles"
    style=${ribbonGalleryStyle}
  >
    ${shapeStyleGalleryItems(documentThemePalette)}
    <mjx-button
      slot="footer"
      label="Other Theme Fills"
      size="small"
      data-opens="ribbons-word-shape-format-shape-styles-theme-styles"
    ></mjx-button>
  </mjx-gallery>`,
  'word.shape-format.shape-styles.shape-fill': html`<mjx-color-picker
    id="ribbons-word-shape-format-shape-fill"
    style=${ribbonColourFieldStyle}
    label="Shape Fill"
    value="theme:accent1"
    show-no-fill
    no-fill-label="No Fill"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Shape Fill', fillEntries(shapeFillEntryOptions('word')))}
  </mjx-color-picker>`,
  'word.shape-format.shape-styles.shape-outline': html`<mjx-color-picker
    id="ribbons-word-shape-format-shape-outline"
    style=${ribbonColourFieldStyle}
    label="Shape Outline"
    value="theme:accent1/darker50"
    show-no-fill
    no-fill-label="No Outline"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Shape Outline', outlineEntries(shapeOutlineEntryOptions('word')))}
  </mjx-color-picker>`,
  'word.shape-format.shape-styles.shape-effects': html`<mjx-button
    label="Shape Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-word-shape-format-shape-styles-shape-effects"
  ></mjx-button>`,
  'word.shape-format.wordart-styles.quick-styles': html`<mjx-gallery
    id="ribbons-word-shape-format-quick-styles"
    label="Quick Styles"
    style=${ribbonGalleryStyle}
  >
    ${wordArtStyleGalleryItems(documentThemePalette)} ${wordArtStyleGalleryFooter()}
  </mjx-gallery>`,
  // Word's Text Fill and Text Outline carry fewer entries than PowerPoint's: no Eyedropper, no Picture… or Texture ▸
  // under the fill, no Sketched ▸ under the outline. See the census's Word's Shape Format disagreement 7.
  'word.shape-format.wordart-styles.text-fill': html`<mjx-color-picker
    id="ribbons-word-shape-format-text-fill"
    style=${ribbonColourFieldStyle}
    label="Text Fill"
    value="theme:background1"
    show-no-fill
    no-fill-label="No Fill"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Text Fill', fillEntries({ moreColours: 'More Fill Colours…', gradient: true }))}
  </mjx-color-picker>`,
  'word.shape-format.wordart-styles.text-outline': html`<mjx-color-picker
    id="ribbons-word-shape-format-text-outline"
    style=${ribbonColourFieldStyle}
    label="Text Outline"
    show-no-fill
    no-fill-label="No Outline"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries(
      'Text Outline',
      outlineEntries({ moreColours: 'More Outline Colours…', weight: true, dashes: true }),
    )}
  </mjx-color-picker>`,
  'word.shape-format.wordart-styles.text-effects': html`<mjx-button
    label="Text Effects"
    icon="text-effects"
    size="small"
    data-opens="ribbons-word-shape-format-wordart-styles-text-effects"
  ></mjx-button>`,
  'word.shape-format.text.text-direction': html`<mjx-button
    label="Text Direction"
    icon="text-direction-rotate-90-right"
    size="small"
    data-opens="ribbons-word-shape-format-text-text-direction"
  ></mjx-button>`,
  'word.shape-format.text.align-text': html`<mjx-button
    label="Align Text"
    icon="align-center-vertical"
    size="small"
    data-opens="ribbons-word-shape-format-text-align-text"
  ></mjx-button>`,
  'word.shape-format.arrange.position': html`<mjx-button
    label="Position"
    size="small"
    data-opens="ribbons-word-shape-format-arrange-position"
  ></mjx-button>`,
  'word.shape-format.arrange.wrap-text': html`<mjx-button
    label="Wrap Text"
    icon="text-position-square"
    size="large"
    data-opens="ribbons-word-shape-format-arrange-wrap-text"
  ></mjx-button>`,
  'word.shape-format.arrange.bring-forward': html`<mjx-split-button
    label="Bring Forward"
    icon="position-forward"
    size="small"
    data-opens="ribbons-word-shape-format-arrange-bring-forward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.shape-format.arrange.send-backward': html`<mjx-split-button
    label="Send Backward"
    icon="position-backward"
    size="small"
    data-opens="ribbons-word-shape-format-arrange-send-backward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.shape-format.arrange.align': html`<mjx-button
    label="Align"
    icon="align-left"
    size="small"
    data-opens="ribbons-word-shape-format-arrange-align"
  ></mjx-button>`,
  'word.shape-format.arrange.group': html`<mjx-button
    label="Group"
    icon="group"
    size="small"
    data-opens="ribbons-word-shape-format-arrange-group"
  ></mjx-button>`,
  'word.shape-format.arrange.rotate': html`<mjx-button
    label="Rotate"
    icon="rotate-right"
    size="small"
    data-opens="ribbons-word-shape-format-arrange-rotate"
  ></mjx-button>`,
  'word.shape-format.size.height': html`<mjx-measure-input
    id="ribbons-word-shape-format-height"
    label="Height"
    value=${wordShapeMeasures.height}
    unit="cm"
    step=${wordShapeMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'word.shape-format.size.width': html`<mjx-measure-input
    id="ribbons-word-shape-format-width"
    label="Width"
    value=${wordShapeMeasures.width}
    unit="cm"
    step=${wordShapeMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  // Chart Design (a contextual tab, in Chart Tools). `Shell/Word` draws Table Tools alone, so these six bindings and
  // `chartToolsMenus('word', …)` are written here and nowhere else. Every menu and the gallery's pictures are
  // `stories/ribbons/chart-tools-menus.ts`'s; the pictures read this document's palette. Switch Row/Column, Select Data
  // and Refresh Data are the generic large button; none is bound.
  'word.chart-design.chart-layouts.add-chart-element': html`<mjx-button
    label="Add Chart Element"
    icon="data-bar-vertical-add"
    size="large"
    data-opens="ribbons-word-chart-design-chart-layouts-add-chart-element"
  ></mjx-button>`,
  'word.chart-design.chart-layouts.quick-layout': html`<mjx-button
    label="Quick Layout"
    icon="layout-cell-four"
    size="large"
    data-opens="ribbons-word-chart-design-chart-layouts-quick-layout"
  ></mjx-button>`,
  'word.chart-design.chart-styles.change-colours': html`<mjx-button
    label="Change Colours"
    icon="color"
    size="large"
    data-opens="ribbons-word-chart-design-chart-styles-change-colours"
  ></mjx-button>`,
  'word.chart-design.chart-styles.style-gallery': html`<mjx-gallery
    id="ribbons-word-chart-design-chart-styles"
    label="Chart Styles"
    value="style-1"
    style=${ribbonGalleryStyle}
  >
    ${chartStyleGalleryItems(documentThemePalette)}
  </mjx-gallery>`,
  'word.chart-design.data.edit-data': html`<mjx-split-button
    label="Edit Data"
    icon="table-edit"
    size="large"
    menu-label="Edit Data"
    data-opens="ribbons-word-chart-design-data-edit-data"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.chart-design.type.change-chart-type': html`<mjx-button
    label="Change Chart Type"
    icon="chart-multiple"
    size="large"
    data-opens="ribbons-word-chart-design-type-change-chart-type"
  ></mjx-button>`,
  // Chart Format (a contextual tab, in Chart Tools). `Shell/Word` draws Table Tools alone, so these twenty-one bindings
  // and the Chart Format half of `chartToolsMenus('word', …)` are written here and nowhere else. The Chart Elements
  // field's options, a chart's Shapes and Change Shape, the entry options that differ from Shape Format's and the
  // starting measures are `stories/ribbons/chart-tools-menus.ts`'s; the Theme Styles gallery, Other Theme Fills, Shape
  // Effects and Shape Fill's entries `stories/ribbons/drawing-tools-menus.ts`'; the WordArt gallery
  // `stories/ribbons/wordart-styles-menus.ts`'; the four pickers and both galleries' pictures read this document's
  // palette. Format Selection and Reset to Match Style are the generic button, and Alt Text and Selection Pane the
  // generic toggle; none is bound.
  'word.chart-format.current-selection.chart-elements': html`<mjx-dropdown
    id="ribbons-word-chart-format-chart-elements"
    label="Chart Elements"
    value="chart-area"
    style=${ribbonFieldStyle}
  >
    ${chartSelectionOptions()}
  </mjx-dropdown>`,
  'word.chart-format.insert-shapes.shapes': html`<mjx-button
    label="Shapes"
    icon="shapes"
    size="large"
    data-opens="ribbons-word-chart-format-insert-shapes-shapes"
  ></mjx-button>`,
  'word.chart-format.insert-shapes.change-shape': html`<mjx-button
    label="Change Shape"
    icon="bezier-curve-square"
    size="small"
    data-opens="ribbons-word-chart-format-insert-shapes-change-shape"
  ></mjx-button>`,
  'word.chart-format.shape-styles.theme-styles': html`<mjx-gallery
    id="ribbons-word-chart-format-theme-styles"
    label="Theme Styles"
    style=${ribbonGalleryStyle}
  >
    ${shapeStyleGalleryItems(documentThemePalette)}
    <mjx-button
      slot="footer"
      label="Other Theme Fills"
      size="small"
      data-opens="ribbons-word-chart-format-shape-styles-theme-styles"
    ></mjx-button>
  </mjx-gallery>`,
  'word.chart-format.shape-styles.shape-fill': html`<mjx-color-picker
    id="ribbons-word-chart-format-shape-fill"
    style=${ribbonColourFieldStyle}
    label="Shape Fill"
    value="theme:background1"
    show-no-fill
    no-fill-label="No Fill"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Shape Fill', fillEntries(shapeFillEntryOptions('word')))}
  </mjx-color-picker>`,
  'word.chart-format.shape-styles.shape-outline': html`<mjx-color-picker
    id="ribbons-word-chart-format-shape-outline"
    style=${ribbonColourFieldStyle}
    label="Shape Outline"
    value="theme:text1/lighter80"
    show-no-fill
    no-fill-label="No Outline"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Shape Outline', outlineEntries(chartOutlineEntryOptions('word')))}
  </mjx-color-picker>`,
  'word.chart-format.shape-styles.shape-effects': html`<mjx-button
    label="Shape Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-word-chart-format-shape-styles-shape-effects"
  ></mjx-button>`,
  'word.chart-format.wordart-styles.quick-styles': html`<mjx-gallery
    id="ribbons-word-chart-format-quick-styles"
    label="Quick Styles"
    style=${ribbonGalleryStyle}
  >
    ${wordArtStyleGalleryItems(documentThemePalette)} ${wordArtStyleGalleryFooter()}
  </mjx-gallery>`,
  'word.chart-format.wordart-styles.text-fill': html`<mjx-color-picker
    id="ribbons-word-chart-format-text-fill"
    style=${ribbonColourFieldStyle}
    label="Text Fill"
    value="theme:text1/lighter40"
    show-no-fill
    no-fill-label="No Fill"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Text Fill', fillEntries(chartTextFillEntryOptions('word')))}
  </mjx-color-picker>`,
  'word.chart-format.wordart-styles.text-outline': html`<mjx-color-picker
    id="ribbons-word-chart-format-text-outline"
    style=${ribbonColourFieldStyle}
    label="Text Outline"
    show-no-fill
    no-fill-label="No Outline"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Text Outline', outlineEntries(chartTextOutlineEntryOptions('word')))}
  </mjx-color-picker>`,
  'word.chart-format.wordart-styles.text-effects': html`<mjx-button
    label="Text Effects"
    icon="text-effects"
    size="small"
    data-opens="ribbons-word-chart-format-wordart-styles-text-effects"
  ></mjx-button>`,
  'word.chart-format.arrange.position': html`<mjx-button
    label="Position"
    size="small"
    data-opens="ribbons-word-chart-format-arrange-position"
  ></mjx-button>`,
  'word.chart-format.arrange.wrap-text': html`<mjx-button
    label="Wrap Text"
    icon="text-position-square"
    size="large"
    data-opens="ribbons-word-chart-format-arrange-wrap-text"
  ></mjx-button>`,
  'word.chart-format.arrange.bring-forward': html`<mjx-split-button
    label="Bring Forward"
    icon="position-forward"
    size="small"
    data-opens="ribbons-word-chart-format-arrange-bring-forward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.chart-format.arrange.send-backward': html`<mjx-split-button
    label="Send Backward"
    icon="position-backward"
    size="small"
    data-opens="ribbons-word-chart-format-arrange-send-backward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'word.chart-format.arrange.align': html`<mjx-button
    label="Align"
    icon="align-left"
    size="small"
    data-opens="ribbons-word-chart-format-arrange-align"
  ></mjx-button>`,
  'word.chart-format.arrange.group': html`<mjx-button
    label="Group"
    icon="group"
    size="small"
    data-opens="ribbons-word-chart-format-arrange-group"
  ></mjx-button>`,
  'word.chart-format.arrange.rotate': html`<mjx-button
    label="Rotate"
    icon="rotate-right"
    size="small"
    data-opens="ribbons-word-chart-format-arrange-rotate"
  ></mjx-button>`,
  'word.chart-format.size.height': html`<mjx-measure-input
    id="ribbons-word-chart-format-height"
    label="Height"
    value=${wordChartMeasures.height}
    unit="cm"
    step=${wordChartMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'word.chart-format.size.width': html`<mjx-measure-input
    id="ribbons-word-chart-format-width"
    label="Width"
    value=${wordChartMeasures.width}
    unit="cm"
    step=${wordChartMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
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
      ${wordTabs({ controls: bindings, includeViewTabs: true })} ${wordContextualSets({ controls: bindings })}
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

    ${insertMenus('word', 'ribbons')} ${drawMenus('word', 'ribbons')}
    ${designLayoutMenus('word', 'ribbons')} ${referencesTransitionsFormulasMenus('word', 'ribbons')}
    ${mailingsAnimationsDataMenus('word', 'ribbons')} ${reviewMenus('word', 'ribbons')}
    ${viewMenus('word', 'ribbons')} ${printPreviewMenus('word', 'ribbons')}
    ${tableToolsMenus('word', 'ribbons')} ${pictureToolsMenus('word', 'ribbons')}
    ${drawingToolsMenus('word', 'ribbons')} ${chartToolsMenus('word', 'ribbons')}
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

/**
 * **Draw**: the tab of the pen, and the ribbon programme's unit 4. Eleven groups: Drawing Tools, Pens,
 * Write, Stencils, Editing, Drawing Canvas, Input Mode, Draw with Touch, Replay, Help and Close.
 * What to look at:
 *
 * 1. **This is two generations of Office on one tab, because the census says so.** Write, Pens and
 *    Close are Office 2013's *Ink Tools* groups: Select Objects, Lasso Select, Pen, Highlighter and
 *    Eraser; then the pen styles, Colour and Thickness; then Stop Inking. The rest are Microsoft 365's
 *    Draw tab. Each command is drawn once, so Drawing Tools holds only **Add Pen**. The group order
 *    is `GUESS:` the declaration's, because no Office build draws the union. `dev/ribbons/census.ts`
 *    has the whole reading, including **Editing**: that is `GroupEditingExcel`, an Excel id on Word's
 *    tab, and it holds Ink Editor.
 * 2. **One command on the tab survives a collapse: Select Objects.** Drag the container in and Write
 *    collapses to its trigger with the arrow pointer beside it; every other group goes to its trigger
 *    alone. Pen, Highlighter, Lasso Select and Eraser arm a gesture rather than doing one thing, which
 *    is unit 3's reason for refusing Text Box. Ruler and Draw with Touch pass the rules and are each
 *    their group's only command. Pens and Write are the tab's primary groups, so they are the last two
 *    standing.
 * 3. ⚠ **The five tools hold one at a time, and Select Objects starts pressed.** Press Pen and Select
 *    Objects releases; press Pen again and it stays pressed. Eraser's face is one of the five, so pressing
 *    it releases Pen, and its arrow still only opens the eraser sizes. Collapse Write and press a tool in
 *    its popup: Select Objects, beside the trigger, releases too. `GUESS:` that pressing the tool that
 *    holds does nothing, where Office's Pen opens its options.
 * 4. **Six commands open something.** Press Add Pen, Pens, Colour, Thickness or Touch/Mouse Mode and
 *    its menu opens; press Eraser's arrow and the eraser sizes open. Eraser's face is a toggle, so pressing
 *    it draws pressed without opening anything. The other commands are the plain
 *    toggles and buttons Office draws.
 * 5. **Three commands carry no icon**: Add Pen, Touch/Mouse Mode and Drawing Canvas. Fluent draws no
 *    pen with a plus, no mouse-and-touch switch and no canvas, and a wrong glyph is worse than a label.
 */
export const Draw: Story = { render: () => ribbon('draw') };

/**
 * **Design**: the look of the whole document, and half of the ribbon programme's unit 5. Two groups:
 * Style Set and Page Background. What to look at:
 *
 * 1. **The first group is labelled Style Set, and Office calls it Document Formatting.** That is the
 *    census's `GroupStyleSet`, and the census wins; `dev/ribbons/census.ts` records it. Its commands
 *    are Office's, in Office's order: Themes, the Style Set gallery, Colours, Fonts, Paragraph Spacing,
 *    Effects, Set as Default.
 * 2. **The Style Set is a real in-ribbon gallery.** It holds nine of Office's style sets by name, from
 *    *This Document* to *Word 2013*. Arrow through it, open its flyout, and the two sections are *This
 *    Document* and *Built-In*. Each picture is a title, a heading and a line of body text in that set's
 *    weight, drawn from the catalogue's one palette.
 * 3. ⚠ **Themes is a dropdown, not a gallery.** Office draws it as a large button whose popup is a grid
 *    of themes, and `<mjx-gallery>` has no button presentation. The popup is a menu of named themes, and
 *    `dev/ribbons/census.ts` marks the reading `GUESS:`. Themes also carries no icon, so it is small
 *    where Office draws it large.
 * 4. **Six commands open a menu.** Press Themes, Colours, Fonts, Paragraph Spacing, Effects or
 *    Watermark and its list opens, with the current choice checked. **Page Colour is a colour picker**
 *    with *No Colour* in it, the component Home's Font Colour already is. Set as Default and Page
 *    Borders open a dialog in Office, so they are plain buttons.
 * 5. **Nothing on this tab survives a collapse**, and no group has a dialog launcher, because Office puts
 *    none there. Style Set is the primary group, so it is the last standing.
 */
export const Design: Story = { render: () => ribbon('design') };

/**
 * **Layout**: the page and where things sit on it, and the other half of unit 5. Word calls this tab
 * Layout; the census calls it `TabPageLayoutWord`. Three groups: Page Setup, Paragraph and Arrange. What
 * to look at:
 *
 * 1. **Paragraph is four fields, not four buttons.** Indent Left and Indent Right are measure inputs in
 *    centimetres, and Spacing Before and Spacing After are in points, starting at 0 and 8. Type
 *    `1.5 cm` into Indent Left and it commits; type `abc` and the field says it cannot read it and keeps
 *    your text. Arrow Up steps by a quarter centimetre, and by six points in Spacing.
 * 2. **Page Setup is seven dropdowns.** Margins, Orientation and Columns are large, and Size is small
 *    between them because Fluent draws no page size. Breaks, Line Numbers and Hyphenation are a column.
 *    Each opens Office's own list, with the current choice checked: Normal, Portrait, A4, One.
 * 3. **Arrange is declared once for Word and Excel.** Position and Wrap Text lead. Bring Forward and Send
 *    Backward are split buttons: the face moves one layer, and the arrow offers Bring to Front and Bring
 *    in Front of Text. Selection Pane is a toggle with no icon, drawn pressed while the pane is open.
 * 4. **Two dialog launchers**, on Page Setup and Paragraph, as Office has them. Arrange has none.
 * 5. **Nothing survives a collapse.** Page Setup is the primary group, so it is the last standing.
 */
export const Layout: Story = { render: () => ribbon('layout') };

/**
 * **References**: the tab of generated content, and Word's part of the ribbon programme's unit 6. Seven
 * groups: Table of Contents, Footnotes, Citations & Bibliography, Captions, Index, Table of Authorities
 * and Acronyms. What to look at:
 *
 * 1. ⚠ **Insert Footnote is a plain large button, and Next Footnote is the split button.** Office draws
 *    no arrow on Insert Footnote. Press Next Footnote's arrow and Previous Footnote, Next Endnote and
 *    Previous Endnote open; press its face and nothing opens. Footnotes has the tab's one dialog
 *    launcher, as in Office.
 * 2. **Four dropdowns and a field.** Table of Contents opens the three built-in tables, Add Text the
 *    levels with *Do Not Show in Table of Contents* checked, Insert Citation the two ways to add a
 *    source, and Bibliography the three built-in bibliographies. **Style is a dropdown field** starting
 *    on APA; pick MLA and the field shows it.
 * 3. **Update Table appears three times**, in Table of Contents, Captions and Table of Authorities, with
 *    Update Index beside them, all four on one refresh glyph. That shared glyph is why none of them
 *    survives a collapse.
 * 4. **Table of Contents is small, although Office draws it large**: three tokens do not fit a large
 *    button. Twelve commands carry no icon and are labelled, from Add Text to Acronyms, and the Style
 *    field carries none either.
 * 5. **Office's Research group is not here**: the census marks it out of scope. `GUESS:` Acronyms is
 *    last. Nothing survives a collapse, and Table of Contents and Footnotes are the primary groups.
 */
export const References: Story = { render: () => ribbon('references') };

/**
 * **Mailings**: the mail merge pipeline, and Word's part of the ribbon programme's unit 7. Five groups:
 * Create, Start Mail Merge, Write & Insert Fields, Preview Results and Finish. What to look at:
 *
 * 1. **Preview Results is pressed, and the record navigator beside it is icon-only**: First Record,
 *    Previous Record, the record number, Next Record and Last Record. The number is a combo box on 1:
 *    pick 3, or type 12. `GUESS:` pressed rather than a new merge's off, so the navigator has something
 *    to audit.
 * 2. **Previous Record and Next Record are the tab's only survivors.** Drag the container narrow until
 *    Preview Results collapses: the two carets stay beside its trigger, and everything else opens from it.
 * 3. **Five menus.** Start Mail Merge opens the document types with Normal Word Document checked, Select
 *    Recipients the three sources, Rules Word's nine rules, and Finish & Merge the three ways to finish.
 *    **Insert Merge Field is a split button**: its arrow lists the fields, and its face opens nothing
 *    here. The brief expected a dropdown; Office draws a split button.
 * 4. **Glyphs to judge**: Envelopes' envelope (large), Address Block's contact card and Greeting Line's
 *    waving hand (both large), and Start Mail Merge, Select Recipients and Edit Recipient List
 *    (`mail-multiple`, `people-list`, `people-edit`). All but the envelope and the hand are `GUESS:`.
 *    Labels, Highlight Merge Fields, Insert Merge Field, Rules, Match Fields, Update Labels and Finish &
 *    Merge carry no icon and are labelled.
 * 5. **Highlight Merge Fields is a toggle**: press it and it draws pressed. No dialog launchers, as in
 *    Office. Start Mail Merge and Write & Insert Fields are the primary groups.
 */
export const Mailings: Story = { render: () => ribbon('mailings') };

/**
 * **Review**: the tab where a document is read by somebody else, and the ribbon programme's unit 8 — the
 * first unit narrowed to one tab of one application. Nine groups: Proofing, Accessibility, Language,
 * Comments, Tracking, Changes, Compare, Protect and Ink. What to look at:
 *
 * 1. ⚠ **Ink is last**, after Protect, where Microsoft 365 draws it; the census declares it fifth.
 *    `GUESS:` the position. Speech (Read Aloud) is not here: the census marks it out of scope.
 * 2. **Every menu is Office's whole list.** Track Changes' arrow: For Everyone (checked), Just Mine, Lock
 *    Tracking. Show Markup: Comments, Ink, Insertions and Deletions and Formatting (all ticked), then
 *    *Balloons* (Show Only Comments and Formatting in Balloons checked) and *Specific People* (All
 *    Reviewers). Accept and Reject: five entries each. Compare: Compare, Combine, then *Show Source
 *    Documents* (Show Both checked). Display for Review is a dropdown on Simple Markup, over its four modes.
 * 3. **Track Changes, Show Comments and Hide Ink are split buttons whose face is a toggle**, as Office
 *    draws a state with a menu. Show Comments starts pressed; Track Changes and Hide Ink start unpressed.
 *    Press a face and it draws pressed or releases; press its arrow and the menu opens and the state does
 *    not move. **Restrict Editing is the plain toggle**: press it and it draws pressed. `GUESS:` Show
 *    Comments' Contextual/List arrow and Hide Ink's shape; the brief's *Show Ink* is Office's *Hide Ink*.
 * 4. **Previous Comment and Next Comment are the tab's only survivors.** Drag narrow until Comments
 *    collapses: the two comment arrows stay beside its trigger. Previous Change and Next Change carry no
 *    icon, so they give way with their group.
 * 5. **Labels, not glyphs**: Show Markup, Compare, Previous Change, Next Change and Hide Ink carry no icon.
 *    Spelling & Grammar and Check Accessibility are small where Office draws them large, because their
 *    labels do not fit a large button. Tracking has the tab's one dialog launcher.
 */
export const Review: Story = { render: () => ribbon('review') };

/**
 * **View**: how the document is looked at, never the document itself. Authored after the three Review tabs,
 * one tab of one application. Seven groups: Document Views, Modes, Page Movement, Show, Zoom, Window and
 * Night Mode. What to look at, least certain first:
 *
 * 1. ⚠ **The three zoom survivors.** Drag narrow until Zoom collapses: 100% (large, *1:1*), One Page (a page
 *    in fit corners) and Page Width (a width between two stops) stay beside the trigger, and Zoom and
 *    Multiple Pages open from it. `GUESS:` that each glyph reads with no label, and 100% keeps its large size
 *    in the survivor row.
 * 2. ⚠ **One view at a time, and one page movement.** Print Layout starts pressed. Press Web Layout and
 *    Print Layout releases; press Web Layout again and it stays pressed. Vertical (pressed) and Side to Side
 *    do the same, and neither set touches the other. Tab to a view and press Space: the same, by keyboard.
 * 3. ⚠ **Group labels and order.** Document Views, Modes and Night Mode are the census's labels, where
 *    Microsoft 365 says Views, Immersive and Dark Mode. Page Movement is third, where Microsoft 365 draws
 *    it; the census declares it sixth. Night Mode is last, `GUESS:`. Macros and SharePoint are out of scope.
 * 4. **Glyphs to judge**: Read Mode's open book, Print Layout's page and Web Layout's globe (all large
 *    toggles, filled while pressed), Outline's stepped bars and Draft's lines with a pencil (small, filled
 *    while pressed), Focus's four corners, Immersive Reader's own mark, New Window, Split's window cut
 *    across, View Side by Side's two panes and Switch Modes' half-dark circle. All but Immersive Reader and
 *    New Window are `GUESS:`.
 * 5. **Switch Windows is the tab's only menu**: one window, *1 Method notes*, checked. `GUESS:` the name,
 *    which is the catalogue's own document rather than a real file. Split is a plain button, because Office
 *    relabels it Remove Split.
 * 6. **Show is three checkboxes**: Ruler and Gridlines unticked, Navigation Pane ticked. Tick one and it
 *    ticks.
 * 7. **Labels, not glyphs**: Vertical, Side to Side, Zoom, Multiple Pages, Arrange All, Synchronous
 *    Scrolling and Reset Window Position carry no icon and are small. No dialog launchers.
 */
export const View: Story = { render: () => ribbon('view') };

/**
 * **Outlining**: a document restructured by its headings, and a view tab Office shows only in Outline view.
 * Authored after PowerPoint's Recording, one tab of one application, and the first view tab authored. Three
 * groups: Outlining Tools, Master Document and Close. What to look at, least certain first:
 *
 * 1. ⚠ **Promote and Demote are the tab's only survivors.** Drag narrow until Outlining Tools collapses: the
 *    left and right arrows stay beside the trigger, and the other ten open from it in order. `GUESS:` that a
 *    bare left arrow reads as *promote* rather than *back*.
 * 2. ⚠ **Collapse Subdocuments is a small toggle** with the brief's label. `GUESS:` Office relabels it
 *    *Expand Subdocuments* rather than drawing it pressed, and draws it large; *Subdocuments* does not wrap.
 *    Press it and it fills; press again and it releases.
 * 3. ⚠ **Close Outline View is large with a three-word label.** It should wrap to two lines without an
 *    ellipsis, under a cross in a square. The group label reads *Outlining Tools*, the census's, where Office
 *    writes *Outline Tools*.
 * 4. ⚠ **The level row.** Promote to Heading 1 (arrow to a wall, left), Promote (left), the Outline Level
 *    field on *Body Text*, Demote (right), Demote to Body Text (arrow to a wall, right), then Move Up, Move
 *    Down, Expand (plus in a box) and Collapse (minus in a box). Open Outline Level: Level 1 to Level 9, then
 *    Body Text. `GUESS:` the list's order and the field's place between the arrows.
 * 5. **Master Document is drawn whole, Show Document pressed.** Office hides Create, Insert, Unlink, Merge,
 *    Split and Lock Document until Show Document is pressed; releasing it here hides nothing. Lock Document
 *    fills while pressed. Glyphs to judge, all `GUESS:`: stacked pages, lines drawn together, a page with a
 *    plus, a page with an arrow, a struck link, two paths joining, one path dividing, a padlock.
 * 6. **Show Level** reads *All Levels* and lists Level 1 to Level 9, then All Levels. **Show Text Formatting**
 *    is ticked and **Show First Line Only** is not.
 * 7. **Not in `Shell/Word`**: switch to the shell and the strip has no Outlining tab, as in Office outside
 *    Outline view. No menus, no dialog launchers.
 */
export const Outlining: Story = { render: () => ribbon('outlining') };

/**
 * **Print Preview**: the document as it will print, and a view tab Office shows only inside Print Preview.
 * Authored after Outlining, one tab of one application, and the second view tab authored. Four groups: Print,
 * Page Setup, Zoom and Preview. What to look at, least certain first:
 *
 * 1. ⚠ **Magnifier is a ticked checkbox, not a toggle button.** It sits under Show Ruler (unticked), above
 *    Shrink One Page, as Office stacks them. `GUESS:` the shape and the tick, from memory of Word 2007 and
 *    2010; the brief listed a toggle.
 * 2. ⚠ **Next Page and Previous Page survive.** Drag narrow until Preview collapses: a page with an arrow
 *    down and a page with an arrow up stay beside the trigger, and the other four open from it in order.
 *    `GUESS:` that the glyphs read as pages rather than *download* and *upload*.
 * 3. ⚠ **Margins, Orientation and Size open Layout's lists, now whole.** Margins: Normal (checked), Narrow,
 *    Moderate, Wide, Mirrored, Office 2003 Default, then Custom Margins… (no Last Custom Setting, because a new
 *    document has none). Orientation: Portrait (checked), Landscape. Size: Letter, Legal, Executive, A3, A4
 *    (checked), A5, B4 (JIS), B5 (JIS), Tabloid, Statement, five envelopes, then More Paper Sizes…; it may
 *    need to scroll. Layout's tab opens the same lists. `GUESS:` the Size order.
 * 4. ⚠ **Zoom draws a magnifier, and Two Pages draws nothing.** Zoom and 100% are large; One Page, Two Pages
 *    and Page Width stack beside them. Collapse Zoom and 100%, One Page and Page Width stay, as on View.
 * 5. **Close Print Preview is large with a three-word label**, under Close Outline View's cross in a square.
 *    It should wrap to two lines without an ellipsis.
 * 6. **Print and Options are large**, a printer and a cog. **Page Setup has a dialog launcher**; Print, Zoom
 *    and Preview have none. **Size is small** between two large neighbours, having no glyph.
 * 7. **Not in `Shell/Word`**: switch to the shell and the strip has no Print Preview tab, and no Print Preview
 *    menu is on the page.
 */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/**
 * **Background Removal**: the two pencils that correct Office's guess at a picture's background, and the two
 * ways out. A view tab Office shows only while a background is being removed. Authored after Print Preview,
 * one tab of one application, and the third view tab authored. Two groups: Refine and Close. What to look
 * at, least certain first:
 *
 * 1. ⚠ **The pencils hold at most one, and start with neither.** Press Mark Areas to Keep: it fills. Press
 *    Mark Areas to Remove: it fills and Keep releases. Press Remove again: it releases, and neither is
 *    pressed. `GUESS:` the release on a second press and the empty start; the Draw tab's tools, by contrast,
 *    keep one pressed.
 * 2. ⚠ **Refine has two commands, not three.** Delete Mark, which Office 2010 to 2016 drew, is absent because
 *    Microsoft 365 no longer draws it. `GUESS:`.
 * 3. ⚠ **Four circles**: a plus and a minus for the pencils, a cross and a tick for Discard All Changes and
 *    Keep Changes. Judge whether the plus and the tick read as different kinds of command. `GUESS:` every
 *    glyph.
 * 4. **All four are large, and the long labels wrap.** *Mark Areas to Remove* and *Discard All Changes* should
 *    wrap to two lines without an ellipsis.
 * 5. **No survivors.** Drag narrow until both groups collapse: each popup trigger stands alone and opens its
 *    commands in order. The pencils' set still holds one at most when pressed inside the popup.
 * 6. **Not in `Shell/Word`**: switch to the shell and the strip has no Background Removal tab. No menus, no
 *    dialog launchers.
 */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };

/**
 * **Table Design**: the style a table wears and the pen its borders are drawn with. Table Tools' first tab, and the
 * first contextual tab authored; Office shows it only while the insertion point is in a table. Three groups: Table
 * Style Options, Table Styles and Borders. What to look at, least certain first. **Items 1 and 2 hold on every
 * contextual story**, placeholders included:
 *
 * 1. **The band.** The strip draws *Table Tools* over this tab and Layout, in the contextual tone, after every core
 *    tab; the tab's accessible name is *Table Design, Table Tools*.
 * 2. **The collapse order is the census's.** A placeholder's one group is drawn at the strongest priority its tab
 *    declares. Here, authored: Table Style Options is `standard` and gives way first, and Table Styles and Borders
 *    are `primary` and give way last.
 * 3. **Border Styles is Borders' first command**, large, left of Line Style, as Microsoft 365 draws it; Table Styles
 *    holds only the gallery and Shading. Press it: *Theme Borders*, twenty-one entries from *Single solid line, ½ pt,
 *    Text 1* to *Single solid line, 1 ½ pt, Accent 6*, then Border Sampler.
 * 4. ⚠ **The gallery's pictures.** Expand Table Styles: *Plain Tables* (7), then *Grid Tables* and *List Tables* (49
 *    each), one family to a row of seven, Table Grid selected. Every picture is drawn in the document's palette, so
 *    the six accent columns should be the six accents Shading's theme row shows. Judge whether Grid Table 4, Grid
 *    Table 5 Dark and Grid Table 6 Colorful read as different styles. Under the list: Modify Table Style…, Clear and
 *    New Table Style…. `GUESS:` every picture.
 * 5. ⚠ **Shading and Pen Colour are colour fields**, where Office draws a paint bucket split button and a small pen
 *    dropdown. Open each: the document's theme colours, standard colours and recent colours, with *No Colour* on
 *    Shading and *Automatic* on Pen Colour, and **More Colours…** beneath the palette on both. Arrow Down from the
 *    last row of recent colours moves onto it; Arrow Up returns. `GUESS:` that Word's two carry no Eyedropper,
 *    Picture, Gradient or Texture, which PowerPoint's do.
 * 6. **Six checkboxes, two columns of three**: Header Row, Total Row and Banded Rows down the first; First Column, Last
 *    Column and Banded Columns down the second. Header Row, Banded Rows and First Column are ticked, the look Word
 *    writes on an inserted table. `GUESS:` the start.
 * 7. **Line Style and Line Weight are fields with names.** Line Style lists No Border and twenty-four styles, Single
 *    selected; Line Weight lists ¼ pt to 6 pt, ½ pt selected. Office draws pictures of lines instead of names.
 * 8. **Borders is a split button.** The face does nothing here; the arrow opens sixteen entries: the four edges; No,
 *    All, Outside and Inside Borders; the inside and diagonal lines; Horizontal Line; then Draw Table, View Gridlines
 *    (ticked) and Borders and Shading….
 * 9. **Border Painter is a toggle**, unpressed. Press it: the brush fills. Press again: it releases.
 * 10. **The launcher at Borders' corner is *Borders and Shading*.** No other group has one.
 * 11. **Glyphs to judge**, all `GUESS:`: Border Styles' three dashed lines, the one new glyph; Borders' grid, Home's
 *     Borders glyph, now large; Border Painter's brush, Format Painter's, now large and filled when pressed.
 * 12. **No survivor anywhere.** Drag narrow: each group collapses to a trigger with nothing beside it, and every
 *     command opens from its popup.
 * 13. **Also in `Shell/Word`**, which draws Table Tools: select Table Design there and every list, menu and starting
 *     state above is the same, under the shell's own ids.
 */
export const TableDesign: Story = { render: () => ribbon('table-design') };

/**
 * **Layout**: a table's structure, its cells' sizes and alignment, and its data. Table Tools' second tab, and the
 * second contextual tab authored; Office shows it only while the insertion point is in a table. Seven groups: Table,
 * Draw, Rows & Columns, Merge, Cell Size, Alignment and Data. Its label is Office's *Layout*, the same as the core
 * Layout tab's; the band and the accessible name *Layout, Table Tools* tell them apart. What to look at, least certain
 * first:
 *
 * 1. ⚠ **Cell Margins' glyph**, `padding-left`: an edge, a dashed guide and an arrow between them, for the room
 *    between a cell's border and its text. The weakest glyph on the tab; judge whether it reads as margins at all.
 * 2. ⚠ **The nine alignments are one set that holds one.** Align Top Left starts pressed. Press Align Centre: it
 *    fills and Top Left releases. Press Align Centre again: nothing changes. They are declared down each column (Top
 *    Left, Centre Left, Bottom Left, then the centre column, then the right); check they draw as Office's grid of
 *    three by three, top row across the top. `GUESS:` the start and the column order.
 * 3. ⚠ **Draw Table and Eraser are one set that may hold none.** Neither starts pressed. Press Draw Table, then
 *    Eraser: Draw Table releases. Press Eraser again: it releases and neither holds. Eraser is a plain toggle here,
 *    with no sizes behind it, unlike Draw's split Eraser.
 * 4. **Eleven survivors, in five groups.** Drag narrow until each group collapses:
 *    - Rows & Columns keeps Insert Above, Insert Below and Insert Right, in that order, with Delete and Insert Left in
 *      its popup. Insert Above keeps its large size beside the trigger.
 *    - Merge keeps Merge Cells and Split Table, with Split Cells in the popup.
 *    - Cell Size keeps Distribute Rows and Distribute Columns, with AutoFit, Height and Width in the popup.
 *    - Alignment keeps Align Top Left, Align Top Centre and Align Top Right, with the other six, Text Direction and
 *      Cell Margins in the popup.
 *    - Data keeps Repeat Header Rows, with Sort, Convert to Text and Formula in the popup.
 *    - Table and Draw keep nothing.
 *
 *    `GUESS:` which three inserts and which three alignments.
 * 5. **The collapse order is the census's.** Draw is `ancillary` and goes first, Table `secondary` next, then Merge,
 *    Cell Size and Data, and Rows & Columns and Alignment last.
 * 6. **Three dropdowns.** Select opens Select Cell, Select Column, Select Row and Select Table. Delete opens Delete
 *    Cells…, Delete Columns, Delete Rows and Delete Table. AutoFit opens AutoFit Contents, AutoFit Window and Fixed
 *    Column Width, none ticked.
 * 7. **Height and Width are measure fields**, 0.5 cm and 3.18 cm, stepping by 0.1. `GUESS:` both numbers.
 * 8. **View Gridlines starts pressed**, as Table Design's Borders menu ticks it. The two do not follow each other
 *    here, because nothing dispatches. **Repeat Header Rows starts unpressed**; press it and it fills.
 * 9. **Two launchers**: *Insert Cells* at Rows & Columns' corner and *Table Properties* at Cell Size's. No other group
 *    has one.
 * 10. **The census's spelling**: *Align Top Centre*, *Align Centre* and the rest, where Office writes *Center*. Hover
 *     an alignment to read its name.
 * 11. **Glyphs to judge**, all `GUESS:`. Select's table with a pointer; View Gridlines' dashed box; Properties' cog;
 *     Draw Table's pencil; Delete's cross, now large; the four inserts' tables with a new line on one side; Merge
 *     Cells, Split Cells and Split Table; AutoFit's arrows; the two Distribute commands' three equal bars; the nine
 *     boxes with two lines; Text Direction's turned *A*, now large; Sort's arrows; Repeat Header Rows' table with a
 *     loop; Convert to Text's table with a turn arrow; Formula's *fx*. Height and Width carry none.
 * 12. **Split Cells, Properties, Text Direction, Cell Margins, Sort, Convert to Text and Formula are plain buttons**:
 *     pressing one does nothing, because no dialog is wired and nothing dispatches.
 * 13. **Also in `Shell/Word`**, which draws Table Tools: select Layout under the *Table Tools* band there and every
 *     list, field, set and survivor above is the same, under the shell's own ids.
 */
export const TableLayout: Story = { render: () => ribbon('table-layout') };

/**
 * **Picture Format**: how a picture's light, colour and transparency are corrected, the frame and effects it wears, its
 * alternative text, where it sits, how it is cropped and how big it is. Picture Tools' one tab, and the sixth contextual
 * tab authored; Office shows it only while a picture is selected. Six groups: Adjust, Picture Styles, Accessibility,
 * Arrange, Size and Image Play. See `Table Design` for the band. What to look at, least certain first:
 *
 * 1. ⚠ **Image Play is the census's, not the brief's.** Its one command is **Play Animation**, a large toggle that
 *    starts pressed, standing for the play and pause of a moving picture. `GUESS:` the command, its label, its glyph and
 *    that it starts playing. Office shows the group only for an animated picture.
 * 2. ⚠ **Glyphs to judge**, all `GUESS:`. **Remove Background's subject before a hatched background is the weakest.**
 *    Then Artistic Effects' two lenses, Transparency's chequerboard, Compress Pictures' four inward arrows, Change
 *    Picture's picture with a forward arrow, Reset Picture's picture with a turn back, Picture Effects' picture with a
 *    shadow, Alt Text's picture with a label, Crop's marks and Play Animation's play mark, all new; Corrections' sun
 *    (Greyscale's), Colour's palette (Design's Colours) and Picture Layout's diagram (Convert to SmartArt's), reused.
 * 3. ⚠ **Corrections, Colour, Artistic Effects and Transparency are large dropdowns of names**, where Office draws
 *    grids of the picture wearing each preset. Press each:
 *    - Corrections: *Sharpen/Soften* (five, Sharpen: 0% checked) and *Brightness/Contrast* (twenty-five, contrast down
 *      and brightness across, Brightness: 0% Contrast: 0% checked), then Picture Corrections Options….
 *    - Colour: *Colour Saturation* (seven, 100% checked), *Colour Tone* (seven, 6500 K checked), *Recolour* (twenty-one,
 *      No Recolour checked), then More Variations, Set Transparent Colour and Picture Colour Options….
 *    - Artistic Effects: twenty-three, None checked, then Artistic Effects Options….
 *    - Transparency: seven, 0% checked, then Picture Transparency Options….
 *
 *    Arrow through a section: each is one set, so choosing a step moves the tick within its section alone.
 * 4. ⚠ **The Quick Styles gallery's thumbnails.** Twenty-eight picture styles by Office's names, from Simple Frame,
 *    White to Metal Oval, each a stand-in landscape (sky Accent 1, land Accent 6, sun Accent 4) framed, cut and given
 *    its effect in the document's palette. Judge whether Metal Frame, Beveled Matte and the two Perspective styles read
 *    as different styles. Nothing is selected, and there is no footer. `GUESS:` every look.
 * 5. **Crop is a split toggle.** Press its face: it fills. Press again: it releases. Its arrow opens Crop, **Crop to
 *    Shape** (seven sections, 147 shapes), **Aspect Ratio** (Square, Portrait, Landscape) and Fill and Fit. `GUESS:`
 *    that Crop draws pressed, and every shape's name.
 * 6. **Picture Effects** opens seven submenus: Preset, Shadow, Reflection, Glow, Soft Edges, Bevel and 3-D Rotation,
 *    the middle five Text Effects' lists. **Picture Layout** opens thirty-one SmartArt picture layouts.
 * 7. **Picture Border is a colour field** with *No Outline*, starting on none, and More Outline Colours…, Weight ▸,
 *    Sketched ▸ and Dashes ▸ beneath the palette. No Eyedropper, `GUESS:`.
 * 8. **Adjust's small column**: Compress Pictures (a plain button), Change Picture (a dropdown: From a File…, From Stock
 *    Images…, From Online Sources…, From Icons…, From Clipboard) and **Reset Picture** (a split button: Reset Picture,
 *    Reset Picture & Size). **Remove Background** is a plain button: in Office it opens the Background Removal tab.
 * 9. **Arrange is Layout's**: Position and Wrap Text, Bring Forward and Send Backward split, Selection Pane, Align,
 *    Group and Rotate, opening the same lists as on Layout under this tab's ids. Position stays small with no glyph.
 * 10. **Height and Width are measure fields**, 8.57 cm and 11.43 cm, stepping by 0.01. `GUESS:` both numbers.
 * 11. **Alt Text is a large toggle**, unpressed; pressed, it fills. No pane opens.
 * 12. **Two launchers**: *Format Picture* at Picture Styles' corner and *Layout* at Size's. No other group has one.
 * 13. **No survivor anywhere.** Drag narrow: Image Play (`ancillary`) gives way first, Accessibility (`secondary`) next,
 *     then Arrange and Size, and Adjust and Picture Styles last; each collapses to a trigger with nothing beside it.
 * 14. **Not in `Shell/Word`**, which draws Table Tools: the shell's strip has no Picture Tools band.
 */
export const PictureFormat: Story = { render: () => ribbon('picture-format') };

/**
 * **Shape Format**: which shape a shape in the document is, how it is filled, outlined and given effects, how its text
 * is dressed and laid inside it, how it is described, where it sits among the text, and its size. Drawing Tools' one
 * tab, and Word's fourth contextual tab authored; Office shows it only while a shape, a text box or a WordArt is
 * selected. Seven groups: Insert Shapes, Shape Styles, WordArt Styles, Text, Accessibility, Arrange and Size. **It is
 * PowerPoint's Shape Format wherever Office's Word is**, so `Ribbons/PowerPoint`'s `ShapeFormat` story covers the Theme
 * Styles pictures, Other Theme Fills, the Shapes list and Edit Shape; and Arrange is `PictureFormat`'s. What to look at
 * here, least certain first:
 *
 * 1. ⚠ **The Text group, Word's alone.** **Text Direction** opens *Horizontal* (checked), *Rotate all text 90°*, *Rotate
 *    all text 270°* and *Text Direction Options…*: **no Stacked**, which PowerPoint's has. **Align Text** opens *Top*
 *    (checked), *Middle* and *Bottom*, one set: choosing one moves the tick. **Create Link** is a plain small button;
 *    in Office the same button reads *Break Link* on a linked text box, and only the unlinked state is drawn. `GUESS:`
 *    every label and both starts.
 * 2. ⚠ **Create Link's chain is the weakest glyph on the tab**: it says *hyperlink* before it says *flow this text into
 *    the next box*. Judge it beside Text Direction's rotated letters and Align Text's centred bar, both reused.
 * 3. ⚠ **Draw Text Box is a split button**, where PowerPoint's Text Box is a plain one. Press its face: nothing opens,
 *    because it arms a drawing gesture. Press its arrow: *Draw Text Box* and *Draw Vertical Text Box*. `GUESS:` the
 *    label, the shape and both entries. **No Merge Shapes** beside it.
 * 4. ⚠ **Text Fill and Text Outline carry fewer entries than PowerPoint's.** Text Fill starts on Background 1 (the white
 *    text of an inserted shape), with *No Fill*, More Fill Colours… and Gradient ▸ beneath the palette; Text Outline
 *    starts on none, with *No Outline*, More Outline Colours…, Weight ▸ and Dashes ▸. `GUESS:` all of it.
 * 5. **Shape Fill and Shape Outline have no Eyedropper.** Shape Fill starts on Accent 1 with More Fill Colours…,
 *    Picture…, Gradient ▸ and Texture ▸; Shape Outline on Accent 1, Darker 50%, with More Outline Colours…, Weight ▸,
 *    Sketched ▸, Dashes ▸ and Arrows ▸. `GUESS:` both starts.
 * 6. **Shapes** opens the whole gallery with **no Action Buttons** and **New Drawing Canvas** under it, the list
 *    `Insert → Shapes` opens. **Edit Shape**'s Change Shape has no Action Buttons either.
 * 7. **Arrange is Picture Format's eight**: Position small with no glyph, Wrap Text large, Bring Forward and Send
 *    Backward split buttons with *in Front of Text* and *Behind Text*, Selection Pane with no glyph, Align with **Align
 *    to Margin ticked**, Group and Rotate.
 * 8. **Theme Styles and Quick Styles** are PowerPoint's galleries in this document's palette, Other Theme Fills in the
 *    first one's footer; **Shape Effects** opens Picture Effects' seven submenus.
 * 9. **Height and Width start on 2.54 cm**, stepping by 0.01. `GUESS:` both.
 * 10. **Three launchers**: *Format Shape* at Shape Styles' corner, *Format Text Effects* at WordArt Styles', and
 *     **Layout** at Size's, where PowerPoint's says *Size and Position*. `GUESS:` all three.
 * 11. **Alt Text** is a large toggle, unpressed.
 * 12. **No survivor anywhere.** Drag narrow: Accessibility (`secondary`) gives way first, then Insert Shapes, WordArt
 *     Styles, Text, Arrange and Size (`standard`), and Shape Styles (`primary`) last; each collapses to a trigger with
 *     nothing beside it.
 * 13. **Not in `Shell/Word`**, which draws Table Tools: there is no Drawing Tools band there and none of these menus is
 *     on that page.
 */
export const ShapeFormat: Story = { render: () => ribbon('shape-format') };

/**
 * **Chart Design**: which elements a chart in the document carries and how they are laid out, which colours and style it
 * wears, where its data comes from, and what kind of chart it is. Chart Tools' first tab, and Word's fifth contextual
 * tab authored; Office shows it only while a chart is selected. Four groups: Chart Layouts, Chart Styles, Data and Type,
 * every command large. It is the census's `TabChartToolsDesignNew`; the three older chart tabs the census also carries
 * are recorded as unbuilt. What to look at here, least certain first:
 *
 * 1. ⚠ **Change Chart Type opens a menu, where Office opens a dialog.** Its eight submenus are Excel's Insert → Charts
 *    families, each holding exactly the list `Ribbons/Excel`'s Insert tab opens for it and ending on *More … Charts…*.
 *    **No Map family.** `GUESS:` both.
 * 2. ⚠ **Add Chart Element's eleven submenus**, for the Clustered Column Word inserts: **Chart Title** (*Above Chart*
 *    checked), **Legend** (*Bottom*), **Gridlines** (*Primary Major Horizontal* ticked, a checkbox each), **Axes** (both
 *    ticked), **Axis Titles** (neither), **Data Labels**, **Data Table**, **Error Bars**, **Lines** and **Up/Down Bars**
 *    (each on *None*), and **Trendline** (plain entries, a verb per series); each ends on its *More … Options…* under a
 *    separator. **Lines and Up/Down Bars open**, where Office greys both on a column chart, so their entries can be
 *    judged. Choose a radio entry: the tick moves within its submenu. `GUESS:` every entry and start.
 * 3. ⚠ **The Chart Styles gallery**, *Style 1* to *Style 16* in this document's palette, starting on Style 1: three
 *    columns in Accent 1, 2 and 3 on a paper, tinted or dark ground, solid, outlined, hatched, shaded or pale, with or
 *    without gridlines. Each picture describes a look rather than rendering Office's. `GUESS:` sixteen, and every look.
 * 4. ⚠ **Quick Layout's glyph is the weakest on the tab**: four tiled regions, which reads *arrange windows* before
 *    *arrange a chart's title, plot and legend*. Its menu is *Layout 1* to *Layout 11*, names where Office draws
 *    thumbnails.
 * 5. **Change Colours** opens *Colourful* (Palettes 1–4) and *Monochromatic* (Palettes 1–13), one set across both, on
 *    *Colourful Palette 1*. Office draws a strip of swatches per palette; the menu holds names. `GUESS:` both counts.
 * 6. **Edit Data is a split button**: its face opens the data sheet, and its arrow *Edit Data* and *Edit Data in Excel*.
 * 7. **Switch Row/Column, Select Data and Refresh Data** are plain large buttons, drawn available; in Office the first is
 *    greyed until the data sheet is open and the last until the chart is linked. Judge their glyphs together: a table
 *    with a turn arrow, a table with a pointer, and the refresh arrow.
 * 8. **The spelling is the catalogue's**: *Change Colours*, *Colourful*, *Centred Overlay*, *Centre*.
 * 9. **No dialog launcher** on any group, as Microsoft 365 draws none here.
 * 10. **No survivor anywhere.** Drag narrow: Chart Styles and Type (`secondary`) give way first, then Data (`standard`),
 *     and Chart Layouts (`primary`) last; each collapses to a trigger with nothing beside it.
 * 11. **Not in `Shell/Word`**, which draws Table Tools: there is no Chart Tools band there and none of these menus is on
 *     that page.
 */
export const ChartDesign: Story = { render: () => ribbon('chart-design') };

/**
 * **Chart Format**: which part of a chart in the document is selected and how to format or reset it, a shape drawn over
 * the chart, how that part is filled, outlined and given effects, how its text is dressed, how the chart is described,
 * where it sits among the text, and its size. Chart Tools' second tab, and Word's sixth contextual tab authored; Office
 * shows it only while a chart is selected, and its accessible name, *Format, Chart Tools*, is what tells it from Shape
 * Format. Seven groups: Current Selection, Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange and
 * Size. **It is Word's Shape Format wherever a chart behaves as a shape**, so `ShapeFormat` covers the Theme Styles
 * pictures, Other Theme Fills, Shape Effects and Text Effects; and Arrange is `PictureFormat`'s. What to look at here,
 * least certain first:
 *
 * 1. ⚠ **Chart Elements**, the field at the head of Current Selection, starts on *Chart Area* and lists the ten parts of
 *    the Clustered Column Word inserts **alphabetically, as Office does**: Chart Area, Chart Title, Horizontal (Category)
 *    Axis, Legend, Plot Area, Series "Series 1" to "Series 3", Vertical (Value) Axis, Vertical (Value) Axis Major
 *    Gridlines. The series come before the vertical axis, where the brief lists them last. Check the field is wide
 *    enough for the longest name. `GUESS:` the order and every label.
 * 2. ⚠ **Reset to Match Style's glyph is the weakest on the tab**: PowerPoint's Reset loop, which says *reset* without
 *    *to the chart's style*. Judge it beside **Format Selection**'s new column chart with a pencil. Both are plain small
 *    buttons, drawn available.
 * 3. ⚠ **Shape Outline has no Sketched ▸ and no Arrows ▸**, where Shape Format's has both, and starts on *Text 1, Lighter
 *    80%*, the nearest swatch to a new chart's light grey border. **Shape Fill** starts on *Background 1* (white) with
 *    More Fill Colours…, Picture…, Gradient ▸ and Texture ▸. `GUESS:` both starts and both omissions.
 * 4. ⚠ **Text Fill and Text Outline are Word's Shape Format's**: Text Fill starts on *Text 1, Lighter 40%* (a chart's
 *    grey text) with More Fill Colours… and Gradient ▸; Text Outline on none, with More Outline Colours…, Weight ▸ and
 *    Dashes ▸. **Text Effects keeps Transform**, which Office may grey on chart text. `GUESS:` all of it.
 * 5. **Shapes** opens the whole shape gallery with **no New Drawing Canvas and no Action Buttons**; **Change Shape**, small,
 *    opens Change Shape's list, **drawn available** where Office greys it until a shape inside the chart is selected. It
 *    draws Edit Shape's glyph. No Draw Text Box, Edit Points or Merge Shapes.
 * 6. **Height 8.89 cm and Width 15.24 cm**, the 3.5 × 6 inch chart Word inserts, stepping by 0.01. `GUESS:` both.
 * 7. **Arrange is Word's Shape Format's eight**: Position small with no glyph, Wrap Text large, Bring Forward and Send
 *    Backward split buttons, Selection Pane with no glyph, Align with **Align to Margin ticked**, Group and Rotate.
 * 8. **Three launchers**: *Format Shape* at Shape Styles' corner, *Format Text Effects* at WordArt Styles', **Layout** at
 *    Size's. `GUESS:` all three; none on Current Selection.
 * 9. **Alt Text** is a large toggle, unpressed.
 * 10. **No survivor anywhere.** Drag narrow: Insert Shapes (`ancillary`) gives way first, then Accessibility
 *     (`secondary`), then Current Selection, WordArt Styles, Arrange and Size (`standard`), and Shape Styles (`primary`)
 *     last; each collapses to a trigger with nothing beside it.
 * 11. **Not in `Shell/Word`**, which draws Table Tools: there is no Chart Tools band there and none of these menus is on
 *     that page.
 */
export const ChartFormat: Story = { render: () => ribbon('chart-format') };
