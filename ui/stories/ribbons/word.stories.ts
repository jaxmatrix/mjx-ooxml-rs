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
import { designLayoutMenus, styleSetGalleryItems } from './design-layout-menus.ts';
import { drawMenus } from './draw-menus.ts';
import { insertMenus } from './insert-menus.ts';
import { mailingsAnimationsDataMenus } from './mailings-animations-data-menus.ts';
import { outlineLevels, showLevels } from './outlining-menus.ts';
import { printPreviewMenus } from './print-preview-menus.ts';
import { referencesTransitionsFormulasMenus } from './references-transitions-formulas-menus.ts';
import { reviewMenus } from './review-menus.ts';
import { viewMenus } from './view-menus.ts';
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
 * **File, Home, Insert, Draw, Design, Layout, References, Mailings, Review, View, Outlining and Print Preview** are real. Every other tab is a placeholder — one group carrying the tab's name,
 * at the priority `dev/ribbons/census.ts` declares for it, holding one button that says so. That is
 * unit 0 of the ribbon programme: the scaffold, with the census transcribed, the ladder already
 * right and every tab present, so each later unit is a small diff rather than a new file.
 *
 * The placeholder button says *Not yet authored* rather than naming a plausible command, for the
 * reason `dev/word-tab-home.ts` gives about its own filler: a made-up command name is a worse lie
 * than an obvious placeholder, and a placeholder occupies exactly as much of the layout as a
 * command does.
 *
 * **Nothing here dispatches a command.** The paste button's menu opens, the Insert, Draw, Design,
 * Layout, References, Mailings, Review, View and Print Preview tabs' menus open, the pickers open, the gallery previews — and no document changes, because command
 * dispatch is loop 2.
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
          'File, Home, Insert, Draw, Design, Layout, References, Mailings, Review, View, Outlining and Print Preview are authored; the rest are placeholders carrying the census’s own ' +
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

    ${insertMenus('word', 'ribbons')} ${drawMenus('word', 'ribbons')}
    ${designLayoutMenus('word', 'ribbons')} ${referencesTransitionsFormulasMenus('word', 'ribbons')}
    ${mailingsAnimationsDataMenus('word', 'ribbons')} ${reviewMenus('word', 'ribbons')}
    ${viewMenus('word', 'ribbons')} ${printPreviewMenus('word', 'ribbons')}
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

/** A view tab: Office shows it only while a picture's background is being removed. */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };
