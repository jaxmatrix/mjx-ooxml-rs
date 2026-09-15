import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { largeGalleryItems } from '../gallery/specimens.ts';
import {
  documentThemePalette,
  machineFonts,
  recentColors,
  standardColors,
} from '../pickers/specimens.ts';
import {
  fitPageCounts,
  openDeclaredSurface,
  ribbonColourFieldStyle,
  ribbonFieldStyle,
  ribbonGalleryStyle,
  ribbonKeyboard,
  ribbonNarrowFieldStyle,
  ribbonScreenReader,
  ribbonStatesMatrix,
  ribbonTokenDependencies,
  scalePercentages,
  type ControlOverrides,
} from './ribbon-parts.ts';
import { chartStyleGalleryItems, chartToolsMenus } from './chart-tools-menus.ts';
import { colourPickerEntries, fillEntries, outlineEntries } from './colour-picker-entries.ts';
import { designLayoutMenus } from './design-layout-menus.ts';
import { drawMenus } from './draw-menus.ts';
import {
  drawingToolsMenus,
  excelShapeMeasures,
  shapeFillEntryOptions,
  shapeOutlineEntryOptions,
  shapeStyleGalleryItems,
} from './drawing-tools-menus.ts';
import { excelContextualSets, excelTabs } from './excel.ts';
import { insertMenus } from './insert-menus.ts';
import { dataTypeGalleryItems, mailingsAnimationsDataMenus } from './mailings-animations-data-menus.ts';
import { excelPictureMeasures, pictureStyleGalleryItems, pictureToolsMenus } from './picture-tools-menus.ts';
import { printPreviewMenus } from './print-preview-menus.ts';
import { referencesTransitionsFormulasMenus } from './references-transitions-formulas-menus.ts';
import { reviewMenus } from './review-menus.ts';
import {
  excelTableName,
  excelTableStyleGalleryFooter,
  excelTableStyleGalleryItems,
  tableToolsMenus,
} from './table-tools-menus.ts';
import { excelSheetViews, viewMenus } from './view-menus.ts';
import { wordArtStyleGalleryFooter, wordArtStyleGalleryItems } from './wordart-styles-menus.ts';

/**
 * **Excel's ribbon, tab by tab** — the same functions `Shell/Excel` composes.
 *
 * ⚠ **Excel's File tab has no Print group**, and that is the census rather than an omission: the
 * committed command surface carries a backstage `TabPrint` row for Word and PowerPoint and none for
 * Excel, and carries a `Publish2Tab` the other two lack. Excel obviously has a File → Print page,
 * so this is a gap in the dump — but the census is the checked source and inventing the row would
 * be the drift the transcription exists to prevent. `dev/ribbons/census.ts` is where it is recorded.
 *
 * **Every core and view tab is authored**: File, Home, Insert, Draw, Page Layout, Formulas, Data, Review, View,
 * Print Preview and Background Removal. **Of the five contextual tabs, Table Design, Picture Format, Shape Format and
 * Chart Design are authored**, Excel's first four; **the last is a placeholder** at the census's own priorities: Chart
 * Tools' Format. Every story draws all four contextual sets so each can be reached; `Shell/Excel` draws Table Tools
 * alone, and binds Table Design as this file does, which is why Picture Format's, Shape Format's and Chart Design's
 * bindings and menus are written here and nowhere else. See
 * `Ribbons/Word → File` for what to look at on a File tab — the three are one tab with three sets
 * of differences rather than three tabs.
 */

const conventions = storyConventions({
  statesMatrix: ribbonStatesMatrix,
  tokenDependencies: ribbonTokenDependencies,
  keyboard: ribbonKeyboard,
  screenReader: ribbonScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Ribbons/Excel',
  parameters: {
    docs: {
      description: {
        component:
          'Excel’s ten core tabs, its File tab and its five contextual tabs, each shown selected inside the whole ' +
          'ribbon. Every core and view tab is authored: File, Home, Insert, Draw, Page Layout, Formulas, Data, ' +
          'Review, View, Print Preview and Background Removal. Of the contextual tabs of the four common sets, Table ' +
          'Design, Picture Format, Shape Format and Chart Design are authored; Chart Tools’ Format is a placeholder ' +
          'carrying the census’s priorities.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The catalogue's own bindings. See `Ribbons/Word` on why these are not the shell's. */
const bindings: ControlOverrides = {
  'excel.home.clipboard.paste': html`<mjx-split-button
    label="Paste"
    icon="clipboard-paste"
    size="large"
    menu-label="Paste options"
    data-opens="ribbons-xl-paste"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.home.font.name': html`<mjx-font-picker
    id="ribbons-xl-font"
    style=${ribbonFieldStyle}
    label="Font"
    value="Aptos"
    .fonts=${machineFonts}
  ></mjx-font-picker>`,
  'excel.home.font.size': html`<mjx-dropdown
    id="ribbons-xl-size"
    label="Font size"
    value="11"
    style=${ribbonNarrowFieldStyle}
  >
    ${['9', '10', '11', '12', '14', '18'].map(
      (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'excel.home.font.fill': html`<mjx-color-picker
    id="ribbons-xl-fill"
    style=${ribbonColourFieldStyle}
    label="Fill colour"
    show-no-fill
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'excel.home.number.format': html`<mjx-dropdown
    id="ribbons-xl-number"
    label="Number format"
    value="general"
    style=${ribbonColourFieldStyle}
  >
    ${[
      { value: 'general', label: 'General' },
      { value: 'number', label: 'Number' },
      { value: 'currency', label: 'Currency' },
      { value: 'accounting', label: 'Accounting' },
      { value: 'percentage', label: 'Percentage' },
      { value: 'date', label: 'Short Date' },
    ].map((format) => html`<mjx-option value=${format.value} label=${format.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'excel.home.styles.gallery': html`<mjx-gallery
    id="ribbons-xl-cell-styles"
    label="Cell styles"
    value="office-2"
    style=${ribbonGalleryStyle}
  >
    ${largeGalleryItems().slice(0, 18)}
  </mjx-gallery>`,
  // Insert (unit 3). Office draws each of these as a dropdown or a split button, so each opens
  // its menu from `stories/ribbons/insert-menus.ts`: a dropdown is one `<mjx-button>` whose press
  // opens the menu, a split button opens it from its arrow. `data-opens` is
  // `commandSurfaceId('ribbons', <this key>)`, and `tests/ribbons.test.ts` requires exactly that.
  'excel.insert.tables.pivottable': html`<mjx-split-button
    label="PivotTable"
    size="small"
    data-opens="ribbons-excel-insert-tables-pivottable"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.insert.illustrations.pictures': html`<mjx-button
    label="Pictures"
    icon="image"
    size="large"
    data-opens="ribbons-excel-insert-illustrations-pictures"
  ></mjx-button>`,
  'excel.insert.illustrations.shapes': html`<mjx-button
    label="Shapes"
    icon="shapes"
    size="large"
    data-opens="ribbons-excel-insert-illustrations-shapes"
  ></mjx-button>`,
  'excel.insert.illustrations.3d-models': html`<mjx-button
    label="3D Models"
    icon="cube"
    size="small"
    data-opens="ribbons-excel-insert-illustrations-3d-models"
  ></mjx-button>`,
  'excel.insert.illustrations.screenshot': html`<mjx-button
    label="Screenshot"
    icon="screenshot"
    size="small"
    data-opens="ribbons-excel-insert-illustrations-screenshot"
  ></mjx-button>`,
  'excel.insert.charts.column-bar': html`<mjx-button
    label="Insert Column or Bar Chart"
    icon="data-bar-vertical"
    size="icon"
    data-opens="ribbons-excel-insert-charts-column-bar"
  ></mjx-button>`,
  'excel.insert.charts.hierarchy': html`<mjx-button
    label="Insert Hierarchy Chart"
    icon="data-treemap"
    size="icon"
    data-opens="ribbons-excel-insert-charts-hierarchy"
  ></mjx-button>`,
  'excel.insert.charts.waterfall': html`<mjx-button
    label="Insert Waterfall, Funnel, Stock, Surface or Radar Chart"
    icon="data-waterfall"
    size="icon"
    data-opens="ribbons-excel-insert-charts-waterfall"
  ></mjx-button>`,
  'excel.insert.charts.line-area': html`<mjx-button
    label="Insert Line or Area Chart"
    icon="data-line"
    size="icon"
    data-opens="ribbons-excel-insert-charts-line-area"
  ></mjx-button>`,
  'excel.insert.charts.statistic': html`<mjx-button
    label="Insert Statistic Chart"
    icon="data-histogram"
    size="icon"
    data-opens="ribbons-excel-insert-charts-statistic"
  ></mjx-button>`,
  'excel.insert.charts.combo': html`<mjx-button
    label="Insert Combo Chart"
    size="small"
    data-opens="ribbons-excel-insert-charts-combo"
  ></mjx-button>`,
  'excel.insert.charts.pie-doughnut': html`<mjx-button
    label="Insert Pie or Doughnut Chart"
    icon="data-pie"
    size="icon"
    data-opens="ribbons-excel-insert-charts-pie-doughnut"
  ></mjx-button>`,
  'excel.insert.charts.scatter-bubble': html`<mjx-button
    label="Insert Scatter (X, Y) or Bubble Chart"
    icon="data-scatter"
    size="icon"
    data-opens="ribbons-excel-insert-charts-scatter-bubble"
  ></mjx-button>`,
  'excel.insert.charts.maps': html`<mjx-button
    label="Maps"
    icon="map"
    size="large"
    data-opens="ribbons-excel-insert-charts-maps"
  ></mjx-button>`,
  'excel.insert.charts.pivotchart': html`<mjx-split-button
    label="PivotChart"
    size="small"
    data-opens="ribbons-excel-insert-charts-pivotchart"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.insert.links.link': html`<mjx-split-button
    label="Link"
    icon="link"
    size="large"
    data-opens="ribbons-excel-insert-links-link"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.insert.text.wordart': html`<mjx-button
    label="WordArt"
    icon="text-effects"
    size="large"
    data-opens="ribbons-excel-insert-text-wordart"
  ></mjx-button>`,
  'excel.insert.text.signature-line': html`<mjx-split-button
    label="Signature Line"
    icon="signature"
    size="small"
    data-opens="ribbons-excel-insert-text-signature-line"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.insert.symbols.equation': html`<mjx-split-button
    label="Equation"
    icon="math-formula"
    size="large"
    data-opens="ribbons-excel-insert-symbols-equation"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  // Draw (unit 4): Word's bindings less Eraser, which Excel draws as the plain toggle its census
  // count says it is. Each opens its menu from `stories/ribbons/draw-menus.ts`.
  'excel.draw.drawing-tools.add-pen': html`<mjx-button
    label="Add Pen"
    size="small"
    data-opens="ribbons-excel-draw-drawing-tools-add-pen"
  ></mjx-button>`,
  'excel.draw.pens.pens': html`<mjx-button
    label="Pens"
    icon="inking-tool"
    size="large"
    data-opens="ribbons-excel-draw-pens-pens"
  ></mjx-button>`,
  'excel.draw.pens.colour': html`<mjx-button
    label="Colour"
    icon="color-line"
    size="small"
    data-opens="ribbons-excel-draw-pens-colour"
  ></mjx-button>`,
  'excel.draw.pens.thickness': html`<mjx-button
    label="Thickness"
    icon="line-thickness"
    size="small"
    data-opens="ribbons-excel-draw-pens-thickness"
  ></mjx-button>`,
  'excel.draw.input-mode.touch-mouse-mode': html`<mjx-button
    label="Touch/Mouse Mode"
    size="small"
    data-opens="ribbons-excel-draw-input-mode-touch-mouse-mode"
  ></mjx-button>`,
  // Page Layout (unit 5). Dropdowns and split buttons open their menus from
  // `stories/ribbons/design-layout-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // Scale to Fit is three fields over `ribbon-parts.ts`'s lists, and Sheet Options four checkboxes.
  'excel.page-layout.themes.themes': html`<mjx-button
    label="Themes"
    size="small"
    data-opens="ribbons-excel-page-layout-themes-themes"
  ></mjx-button>`,
  'excel.page-layout.themes.colours': html`<mjx-button
    label="Colours"
    icon="color"
    size="small"
    data-opens="ribbons-excel-page-layout-themes-colours"
  ></mjx-button>`,
  'excel.page-layout.themes.fonts': html`<mjx-button
    label="Fonts"
    icon="text-font"
    size="small"
    data-opens="ribbons-excel-page-layout-themes-fonts"
  ></mjx-button>`,
  'excel.page-layout.themes.effects': html`<mjx-button
    label="Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-excel-page-layout-themes-effects"
  ></mjx-button>`,
  'excel.page-layout.page-setup.margins': html`<mjx-button
    label="Margins"
    icon="document-margins"
    size="large"
    data-opens="ribbons-excel-page-layout-page-setup-margins"
  ></mjx-button>`,
  'excel.page-layout.page-setup.orientation': html`<mjx-button
    label="Orientation"
    icon="orientation"
    size="large"
    data-opens="ribbons-excel-page-layout-page-setup-orientation"
  ></mjx-button>`,
  'excel.page-layout.page-setup.size': html`<mjx-button
    label="Size"
    size="small"
    data-opens="ribbons-excel-page-layout-page-setup-size"
  ></mjx-button>`,
  'excel.page-layout.page-setup.print-area': html`<mjx-button
    label="Print Area"
    size="small"
    data-opens="ribbons-excel-page-layout-page-setup-print-area"
  ></mjx-button>`,
  'excel.page-layout.page-setup.breaks': html`<mjx-button
    label="Breaks"
    icon="document-page-break"
    size="small"
    data-opens="ribbons-excel-page-layout-page-setup-breaks"
  ></mjx-button>`,
  'excel.page-layout.scale-to-fit.width': html`<mjx-dropdown
    id="ribbons-xl-fit-width"
    label="Width"
    value="automatic"
    style=${ribbonColourFieldStyle}
  >
    ${fitPageCounts.map(
      (count) => html`<mjx-option value=${count.value} label=${count.label}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'excel.page-layout.scale-to-fit.height': html`<mjx-dropdown
    id="ribbons-xl-fit-height"
    label="Height"
    value="automatic"
    style=${ribbonColourFieldStyle}
  >
    ${fitPageCounts.map(
      (count) => html`<mjx-option value=${count.value} label=${count.label}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'excel.page-layout.scale-to-fit.scale': html`<mjx-combo-box
    id="ribbons-xl-fit-scale"
    label="Scale"
    value="100%"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${scalePercentages.map((scale) => html`<mjx-option value=${scale} label=${scale}></mjx-option>`)}
  </mjx-combo-box>`,
  'excel.page-layout.sheet-options.view-gridlines': html`<mjx-checkbox id="ribbons-xl-view-gridlines" label="View Gridlines" checked="true"></mjx-checkbox>`,
  'excel.page-layout.sheet-options.print-gridlines': html`<mjx-checkbox id="ribbons-xl-print-gridlines" label="Print Gridlines"></mjx-checkbox>`,
  'excel.page-layout.sheet-options.view-headings': html`<mjx-checkbox id="ribbons-xl-view-headings" label="View Headings" checked="true"></mjx-checkbox>`,
  'excel.page-layout.sheet-options.print-headings': html`<mjx-checkbox id="ribbons-xl-print-headings" label="Print Headings"></mjx-checkbox>`,
  'excel.page-layout.arrange.bring-forward': html`<mjx-split-button
    label="Bring Forward"
    icon="position-forward"
    size="large"
    data-opens="ribbons-excel-page-layout-arrange-bring-forward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.page-layout.arrange.send-backward': html`<mjx-split-button
    label="Send Backward"
    icon="position-backward"
    size="large"
    data-opens="ribbons-excel-page-layout-arrange-send-backward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.page-layout.arrange.align': html`<mjx-button
    label="Align"
    icon="align-left"
    size="small"
    data-opens="ribbons-excel-page-layout-arrange-align"
  ></mjx-button>`,
  'excel.page-layout.arrange.group': html`<mjx-button
    label="Group"
    icon="group"
    size="small"
    data-opens="ribbons-excel-page-layout-arrange-group"
  ></mjx-button>`,
  'excel.page-layout.arrange.rotate': html`<mjx-button
    label="Rotate"
    icon="rotate-right"
    size="small"
    data-opens="ribbons-excel-page-layout-arrange-rotate"
  ></mjx-button>`,
  // Formulas (unit 6). Split buttons and dropdowns open their menus from
  // `stories/ribbons/references-transitions-formulas-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  'excel.formulas.function-library.autosum': html`<mjx-split-button
    label="AutoSum"
    icon="autosum"
    size="large"
    data-opens="ribbons-excel-formulas-function-library-autosum"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.formulas.function-library.recently-used': html`<mjx-button
    label="Recently Used"
    icon="book-star"
    size="large"
    data-opens="ribbons-excel-formulas-function-library-recently-used"
  ></mjx-button>`,
  'excel.formulas.function-library.financial': html`<mjx-button
    label="Financial"
    icon="book-coins"
    size="large"
    data-opens="ribbons-excel-formulas-function-library-financial"
  ></mjx-button>`,
  'excel.formulas.function-library.logical': html`<mjx-button
    label="Logical"
    icon="book-question-mark"
    size="large"
    data-opens="ribbons-excel-formulas-function-library-logical"
  ></mjx-button>`,
  'excel.formulas.function-library.text': html`<mjx-button
    label="Text"
    icon="book-letter"
    size="large"
    data-opens="ribbons-excel-formulas-function-library-text"
  ></mjx-button>`,
  'excel.formulas.function-library.date-time': html`<mjx-button
    label="Date & Time"
    icon="book-clock"
    size="small"
    data-opens="ribbons-excel-formulas-function-library-date-time"
  ></mjx-button>`,
  'excel.formulas.function-library.lookup-reference': html`<mjx-button
    label="Lookup & Reference"
    icon="book-search"
    size="small"
    data-opens="ribbons-excel-formulas-function-library-lookup-reference"
  ></mjx-button>`,
  'excel.formulas.function-library.math-trig': html`<mjx-button
    label="Math & Trig"
    icon="book-theta"
    size="small"
    data-opens="ribbons-excel-formulas-function-library-math-trig"
  ></mjx-button>`,
  'excel.formulas.function-library.more-functions': html`<mjx-button
    label="More Functions"
    icon="book"
    size="large"
    data-opens="ribbons-excel-formulas-function-library-more-functions"
  ></mjx-button>`,
  'excel.formulas.named-cells.define-name': html`<mjx-split-button
    label="Define Name"
    icon="tag-add"
    size="small"
    data-opens="ribbons-excel-formulas-named-cells-define-name"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.formulas.named-cells.use-in-formula': html`<mjx-button
    label="Use in Formula"
    size="small"
    data-opens="ribbons-excel-formulas-named-cells-use-in-formula"
  ></mjx-button>`,
  'excel.formulas.formula-auditing.remove-arrows': html`<mjx-split-button
    label="Remove Arrows"
    size="small"
    data-opens="ribbons-excel-formulas-formula-auditing-remove-arrows"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.formulas.formula-auditing.error-checking': html`<mjx-split-button
    label="Error Checking"
    icon="checkmark-circle-warning"
    size="small"
    data-opens="ribbons-excel-formulas-formula-auditing-error-checking"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.formulas.calculation.calculation-options': html`<mjx-button
    label="Calculation Options"
    icon="calculator"
    size="small"
    data-opens="ribbons-excel-formulas-calculation-calculation-options"
  ></mjx-button>`,
  // Data (unit 7). Split buttons and dropdowns open their menus from
  // `stories/ribbons/mailings-animations-data-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // Data Types is an in-ribbon gallery with nothing selected, because a new cell has no data type.
  'excel.data.get-external-data.from-other-sources': html`<mjx-button
    label="From Other Sources"
    icon="database"
    size="small"
    data-opens="ribbons-excel-data-get-external-data-from-other-sources"
  ></mjx-button>`,
  'excel.data.queries-connections.refresh-all': html`<mjx-split-button
    label="Refresh All"
    icon="arrow-clockwise"
    size="large"
    data-opens="ribbons-excel-data-queries-connections-refresh-all"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.data.data-types.data-types': html`<mjx-gallery id="ribbons-xl-data-types" label="Data Types" style=${ribbonGalleryStyle}>
    ${dataTypeGalleryItems()}
  </mjx-gallery>`,
  'excel.data.data-tools.data-validation': html`<mjx-split-button
    label="Data Validation"
    icon="table-simple-checkmark"
    size="small"
    data-opens="ribbons-excel-data-data-tools-data-validation"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.data.forecast.what-if-analysis': html`<mjx-button
    label="What-If Analysis"
    size="small"
    data-opens="ribbons-excel-data-forecast-what-if-analysis"
  ></mjx-button>`,
  'excel.data.outline.group': html`<mjx-split-button
    label="Group"
    icon="group-list"
    size="large"
    data-opens="ribbons-excel-data-outline-group"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.data.outline.ungroup': html`<mjx-split-button
    label="Ungroup"
    size="small"
    data-opens="ribbons-excel-data-outline-ungroup"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  // Excel's Review. The split buttons and the two dropdowns open their menus from
  // `stories/ribbons/review-menus.ts`, and `data-opens` is `commandSurfaceId('ribbons', <this key>)`.
  // Hide Ink is a split button whose face is a toggle, starting unpressed.
  'excel.review.accessibility.check-accessibility': html`<mjx-split-button
    label="Check Accessibility"
    icon="accessibility-checkmark"
    size="small"
    data-opens="ribbons-excel-review-accessibility-check-accessibility"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.review.notes.notes': html`<mjx-button
    label="Notes"
    icon="note"
    size="large"
    data-opens="ribbons-excel-review-notes-notes"
  ></mjx-button>`,
  'excel.review.changes.track-changes': html`<mjx-button
    label="Track Changes"
    icon="document-edit"
    size="small"
    data-opens="ribbons-excel-review-changes-track-changes"
  ></mjx-button>`,
  'excel.review.ink.hide-ink': html`<mjx-split-button
    toggle
    label="Hide Ink"
    size="small"
    data-opens="ribbons-excel-review-ink-hide-ink"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  // Excel's View. The Sheet View dropdown is a field over `excelSheetViews`, Show's four are checkboxes,
  // all ticked as the census declares, and Freeze Panes and Switch Windows open their menus from
  // `stories/ribbons/view-menus.ts`, with `data-opens` `commandSurfaceId('ribbons', <this key>)`. The Workbook
  // Views exclusive set is the generic toggles.
  'excel.view.sheet-view.sheet-view': html`<mjx-dropdown
    id="ribbons-xl-view-sheet-view"
    label="Sheet View"
    value="default"
    style=${ribbonNarrowFieldStyle}
  >
    ${excelSheetViews.map((view) => html`<mjx-option value=${view.value} label=${view.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'excel.view.show.ruler': html`<mjx-checkbox id="ribbons-xl-view-ruler" label="Ruler" checked="true"></mjx-checkbox>`,
  'excel.view.show.gridlines': html`<mjx-checkbox id="ribbons-xl-view-show-gridlines" label="Gridlines" checked="true"></mjx-checkbox>`,
  'excel.view.show.formula-bar': html`<mjx-checkbox id="ribbons-xl-view-formula-bar" label="Formula Bar" checked="true"></mjx-checkbox>`,
  'excel.view.show.headings': html`<mjx-checkbox id="ribbons-xl-view-show-headings" label="Headings" checked="true"></mjx-checkbox>`,
  'excel.view.window.freeze-panes': html`<mjx-button
    label="Freeze Panes"
    icon="table-freeze-column-and-row"
    size="large"
    data-opens="ribbons-excel-view-window-freeze-panes"
  ></mjx-button>`,
  'excel.view.window.switch-windows': html`<mjx-button
    label="Switch Windows"
    icon="window-multiple"
    size="large"
    data-opens="ribbons-excel-view-window-switch-windows"
  ></mjx-button>`,
  // Print Preview (a view tab). `Shell/Excel` never draws a view tab, so this binding is written here and
  // nowhere else. Show Margins is a checkbox, unticked, as the census declares; the tab opens no menu.
  'excel.print-preview.preview.show-margins': html`<mjx-checkbox id="ribbons-xl-print-preview-show-margins" label="Show Margins"></mjx-checkbox>`,
  // Table Design (a contextual tab, in Excel's own Table Tools). Both Excel hosts draw the set, so `Shell/Excel` binds
  // the same eleven commands under its own ids. Table Name is a combo box over `excelTableName`, the seven checkboxes
  // start as Format as Table leaves a new table, the gallery is filled from `stories/ribbons/table-tools-menus.ts` in
  // the document's palette, and Export and Refresh open that file's menus. The other eight are the generic button.
  'excel.table-design.properties.table-name': html`<mjx-combo-box
    id="ribbons-xl-table-design-table-name"
    label="Table Name"
    value=${excelTableName}
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    <mjx-option value=${excelTableName} label=${excelTableName}></mjx-option>
  </mjx-combo-box>`,
  'excel.table-design.external-table-data.export': html`<mjx-button
    label="Export"
    icon="arrow-export"
    size="large"
    data-opens="ribbons-excel-table-design-external-table-data-export"
  ></mjx-button>`,
  'excel.table-design.external-table-data.refresh': html`<mjx-split-button
    label="Refresh"
    icon="arrow-clockwise"
    size="large"
    menu-label="Refresh"
    data-opens="ribbons-excel-table-design-external-table-data-refresh"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.table-design.table-style-options.header-row': html`<mjx-checkbox id="ribbons-xl-table-design-header-row" label="Header Row" checked="true"></mjx-checkbox>`,
  'excel.table-design.table-style-options.total-row': html`<mjx-checkbox id="ribbons-xl-table-design-total-row" label="Total Row"></mjx-checkbox>`,
  'excel.table-design.table-style-options.banded-rows': html`<mjx-checkbox id="ribbons-xl-table-design-banded-rows" label="Banded Rows" checked="true"></mjx-checkbox>`,
  'excel.table-design.table-style-options.first-column': html`<mjx-checkbox id="ribbons-xl-table-design-first-column" label="First Column"></mjx-checkbox>`,
  'excel.table-design.table-style-options.last-column': html`<mjx-checkbox id="ribbons-xl-table-design-last-column" label="Last Column"></mjx-checkbox>`,
  'excel.table-design.table-style-options.banded-columns': html`<mjx-checkbox id="ribbons-xl-table-design-banded-columns" label="Banded Columns"></mjx-checkbox>`,
  'excel.table-design.table-style-options.filter-button': html`<mjx-checkbox id="ribbons-xl-table-design-filter-button" label="Filter Button" checked="true"></mjx-checkbox>`,
  'excel.table-design.table-styles.gallery': html`<mjx-gallery
    id="ribbons-xl-table-styles"
    label="Table Styles"
    value="TableStyleMedium2"
    style=${ribbonGalleryStyle}
  >
    ${excelTableStyleGalleryItems(documentThemePalette)} ${excelTableStyleGalleryFooter()}
  </mjx-gallery>`,
  // Picture Format (a contextual tab, in Picture Tools). `Shell/Excel` draws Table Tools alone, so this binding is
  // written here and nowhere else. Every menu, the gallery's styles and the two starting measures are
  // `stories/ribbons/picture-tools-menus.ts`'s; Arrange's menus are Arrange's Excel lists under this tab's ids. Remove
  // Background, Compress Pictures, Alt Text, Selection Pane and Play Animation are the generic button or toggle and are
  // not bound.
  'excel.picture-format.adjust.corrections': html`<mjx-button
    label="Corrections"
    icon="brightness-high"
    size="large"
    data-opens="ribbons-excel-picture-format-adjust-corrections"
  ></mjx-button>`,
  'excel.picture-format.adjust.colour': html`<mjx-button
    label="Colour"
    icon="color"
    size="large"
    data-opens="ribbons-excel-picture-format-adjust-colour"
  ></mjx-button>`,
  'excel.picture-format.adjust.artistic-effects': html`<mjx-button
    label="Artistic Effects"
    icon="photo-filter"
    size="large"
    data-opens="ribbons-excel-picture-format-adjust-artistic-effects"
  ></mjx-button>`,
  'excel.picture-format.adjust.transparency': html`<mjx-button
    label="Transparency"
    icon="transparency-square"
    size="large"
    data-opens="ribbons-excel-picture-format-adjust-transparency"
  ></mjx-button>`,
  'excel.picture-format.adjust.change-picture': html`<mjx-button
    label="Change Picture"
    icon="image-arrow-forward"
    size="small"
    data-opens="ribbons-excel-picture-format-adjust-change-picture"
  ></mjx-button>`,
  'excel.picture-format.adjust.reset-picture': html`<mjx-split-button
    label="Reset Picture"
    icon="image-arrow-counterclockwise"
    size="small"
    menu-label="Reset Picture"
    data-opens="ribbons-excel-picture-format-adjust-reset-picture"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.picture-format.picture-styles.quick-styles': html`<mjx-gallery
    id="ribbons-xl-picture-format-quick-styles"
    label="Quick Styles"
    style=${ribbonGalleryStyle}
  >
    ${pictureStyleGalleryItems(documentThemePalette)}
  </mjx-gallery>`,
  'excel.picture-format.picture-styles.picture-border': html`<mjx-color-picker
    id="ribbons-xl-picture-format-picture-border"
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
  'excel.picture-format.picture-styles.picture-effects': html`<mjx-button
    label="Picture Effects"
    icon="image-shadow"
    size="small"
    data-opens="ribbons-excel-picture-format-picture-styles-picture-effects"
  ></mjx-button>`,
  'excel.picture-format.picture-styles.picture-layout': html`<mjx-button
    label="Picture Layout"
    icon="diagram"
    size="small"
    data-opens="ribbons-excel-picture-format-picture-styles-picture-layout"
  ></mjx-button>`,
  'excel.picture-format.arrange.bring-forward': html`<mjx-split-button
    label="Bring Forward"
    icon="position-forward"
    size="large"
    data-opens="ribbons-excel-picture-format-arrange-bring-forward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.picture-format.arrange.send-backward': html`<mjx-split-button
    label="Send Backward"
    icon="position-backward"
    size="large"
    data-opens="ribbons-excel-picture-format-arrange-send-backward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.picture-format.arrange.align': html`<mjx-button
    label="Align"
    icon="align-left"
    size="small"
    data-opens="ribbons-excel-picture-format-arrange-align"
  ></mjx-button>`,
  'excel.picture-format.arrange.group': html`<mjx-button
    label="Group"
    icon="group"
    size="small"
    data-opens="ribbons-excel-picture-format-arrange-group"
  ></mjx-button>`,
  'excel.picture-format.arrange.rotate': html`<mjx-button
    label="Rotate"
    icon="rotate-right"
    size="small"
    data-opens="ribbons-excel-picture-format-arrange-rotate"
  ></mjx-button>`,
  'excel.picture-format.size.crop': html`<mjx-split-button
    toggle
    label="Crop"
    icon="crop"
    size="large"
    menu-label="Crop"
    data-opens="ribbons-excel-picture-format-size-crop"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.picture-format.size.height': html`<mjx-measure-input
    id="ribbons-xl-picture-format-height"
    label="Height"
    value=${excelPictureMeasures.height}
    unit="cm"
    step=${excelPictureMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'excel.picture-format.size.width': html`<mjx-measure-input
    id="ribbons-xl-picture-format-width"
    label="Width"
    value=${excelPictureMeasures.width}
    unit="cm"
    step=${excelPictureMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  // Shape Format (a contextual tab, in Drawing Tools). `Shell/Excel` draws Table Tools alone, so these eighteen bindings
  // and `drawingToolsMenus('excel', …)` are written here and nowhere else. Every menu, the Theme Styles gallery, the
  // entry options and the starting measures are `stories/ribbons/drawing-tools-menus.ts`'s; the WordArt gallery is
  // `stories/ribbons/wordart-styles-menus.ts`'; the four pickers and both galleries' pictures read this workbook's
  // palette. Other Theme Fills, under the Theme Styles gallery, opens the menu declared for the gallery's own command.
  // Alt Text and Selection Pane are the generic toggle; neither is bound.
  'excel.shape-format.insert-shapes.shapes': html`<mjx-button
    label="Shapes"
    icon="shapes"
    size="large"
    data-opens="ribbons-excel-shape-format-insert-shapes-shapes"
  ></mjx-button>`,
  'excel.shape-format.insert-shapes.edit-shape': html`<mjx-button
    label="Edit Shape"
    icon="bezier-curve-square"
    size="small"
    data-opens="ribbons-excel-shape-format-insert-shapes-edit-shape"
  ></mjx-button>`,
  'excel.shape-format.insert-shapes.text-box': html`<mjx-split-button
    label="Text Box"
    icon="textbox"
    size="small"
    menu-label="Text Box"
    data-opens="ribbons-excel-shape-format-insert-shapes-text-box"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.shape-format.shape-styles.theme-styles': html`<mjx-gallery
    id="ribbons-xl-shape-format-theme-styles"
    label="Theme Styles"
    style=${ribbonGalleryStyle}
  >
    ${shapeStyleGalleryItems(documentThemePalette)}
    <mjx-button
      slot="footer"
      label="Other Theme Fills"
      size="small"
      data-opens="ribbons-excel-shape-format-shape-styles-theme-styles"
    ></mjx-button>
  </mjx-gallery>`,
  'excel.shape-format.shape-styles.shape-fill': html`<mjx-color-picker
    id="ribbons-xl-shape-format-shape-fill"
    style=${ribbonColourFieldStyle}
    label="Shape Fill"
    value="theme:accent1"
    show-no-fill
    no-fill-label="No Fill"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Shape Fill', fillEntries(shapeFillEntryOptions('excel')))}
  </mjx-color-picker>`,
  'excel.shape-format.shape-styles.shape-outline': html`<mjx-color-picker
    id="ribbons-xl-shape-format-shape-outline"
    style=${ribbonColourFieldStyle}
    label="Shape Outline"
    value="theme:accent1/darker50"
    show-no-fill
    no-fill-label="No Outline"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries('Shape Outline', outlineEntries(shapeOutlineEntryOptions('excel')))}
  </mjx-color-picker>`,
  'excel.shape-format.shape-styles.shape-effects': html`<mjx-button
    label="Shape Effects"
    icon="square-shadow"
    size="small"
    data-opens="ribbons-excel-shape-format-shape-styles-shape-effects"
  ></mjx-button>`,
  'excel.shape-format.wordart-styles.quick-styles': html`<mjx-gallery
    id="ribbons-xl-shape-format-quick-styles"
    label="Quick Styles"
    style=${ribbonGalleryStyle}
  >
    ${wordArtStyleGalleryItems(documentThemePalette)} ${wordArtStyleGalleryFooter()}
  </mjx-gallery>`,
  // Excel's Text Fill and Text Outline are PowerPoint's less the Eyedropper: a shape's text in a workbook is DrawingML
  // text, as a slide's is, so it keeps Picture…, Texture ▸ and Sketched ▸, which Word's shorter pair drops. See the
  // census's Excel's Shape Format disagreement 5.
  'excel.shape-format.wordart-styles.text-fill': html`<mjx-color-picker
    id="ribbons-xl-shape-format-text-fill"
    style=${ribbonColourFieldStyle}
    label="Text Fill"
    value="theme:background1"
    show-no-fill
    no-fill-label="No Fill"
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  >
    ${colourPickerEntries(
      'Text Fill',
      fillEntries({ moreColours: 'More Fill Colours…', picture: true, gradient: true, texture: true }),
    )}
  </mjx-color-picker>`,
  'excel.shape-format.wordart-styles.text-outline': html`<mjx-color-picker
    id="ribbons-xl-shape-format-text-outline"
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
      outlineEntries({ moreColours: 'More Outline Colours…', weight: true, sketched: true, dashes: true }),
    )}
  </mjx-color-picker>`,
  'excel.shape-format.wordart-styles.text-effects': html`<mjx-button
    label="Text Effects"
    icon="text-effects"
    size="small"
    data-opens="ribbons-excel-shape-format-wordart-styles-text-effects"
  ></mjx-button>`,
  'excel.shape-format.arrange.bring-forward': html`<mjx-split-button
    label="Bring Forward"
    icon="position-forward"
    size="large"
    data-opens="ribbons-excel-shape-format-arrange-bring-forward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.shape-format.arrange.send-backward': html`<mjx-split-button
    label="Send Backward"
    icon="position-backward"
    size="large"
    data-opens="ribbons-excel-shape-format-arrange-send-backward"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.shape-format.arrange.align': html`<mjx-button
    label="Align"
    icon="align-left"
    size="small"
    data-opens="ribbons-excel-shape-format-arrange-align"
  ></mjx-button>`,
  'excel.shape-format.arrange.group': html`<mjx-button
    label="Group"
    icon="group"
    size="small"
    data-opens="ribbons-excel-shape-format-arrange-group"
  ></mjx-button>`,
  'excel.shape-format.arrange.rotate': html`<mjx-button
    label="Rotate"
    icon="rotate-right"
    size="small"
    data-opens="ribbons-excel-shape-format-arrange-rotate"
  ></mjx-button>`,
  'excel.shape-format.size.height': html`<mjx-measure-input
    id="ribbons-xl-shape-format-height"
    label="Height"
    value=${excelShapeMeasures.height}
    unit="cm"
    step=${excelShapeMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  'excel.shape-format.size.width': html`<mjx-measure-input
    id="ribbons-xl-shape-format-width"
    label="Width"
    value=${excelShapeMeasures.width}
    unit="cm"
    step=${excelShapeMeasures.step}
    min="0"
    style=${ribbonNarrowFieldStyle}
  ></mjx-measure-input>`,
  // Chart Design (a contextual tab, in Chart Tools). `Shell/Excel` draws Table Tools alone, so these five bindings and
  // `chartToolsMenus('excel', …)` are written here and nowhere else. They are `Ribbons/Word`'s six less Edit Data, under
  // Excel's ids: every menu and the gallery's pictures are `stories/ribbons/chart-tools-menus.ts`'s, and the pictures
  // read this workbook's palette. Switch Row/Column, Select Data and Move Chart are the generic large button; none is
  // bound.
  'excel.chart-design.chart-layouts.add-chart-element': html`<mjx-button
    label="Add Chart Element"
    icon="data-bar-vertical-add"
    size="large"
    data-opens="ribbons-excel-chart-design-chart-layouts-add-chart-element"
  ></mjx-button>`,
  'excel.chart-design.chart-layouts.quick-layout': html`<mjx-button
    label="Quick Layout"
    icon="layout-cell-four"
    size="large"
    data-opens="ribbons-excel-chart-design-chart-layouts-quick-layout"
  ></mjx-button>`,
  'excel.chart-design.chart-styles.change-colours': html`<mjx-button
    label="Change Colours"
    icon="color"
    size="large"
    data-opens="ribbons-excel-chart-design-chart-styles-change-colours"
  ></mjx-button>`,
  'excel.chart-design.chart-styles.style-gallery': html`<mjx-gallery
    id="ribbons-xl-chart-design-chart-styles"
    label="Chart Styles"
    value="style-1"
    style=${ribbonGalleryStyle}
  >
    ${chartStyleGalleryItems(documentThemePalette)}
  </mjx-gallery>`,
  'excel.chart-design.type.change-chart-type': html`<mjx-button
    label="Change Chart Type"
    icon="chart-multiple"
    size="large"
    data-opens="ribbons-excel-chart-design-type-change-chart-type"
  ></mjx-button>`,
};

/**
 * The whole ribbon with one tab selected, and the paste menu the Home tab opens.
 *
 * ⚠ **No `<mjx-resizable-container>` of its own** — see `Ribbons/Word` for why a second one would
 * put a duplicate width control above every tab in this section.
 */
function ribbon(selected: string): TemplateResult {
  return html`
    <mjx-ribbon label="Excel" selected=${selected} @mjx-activate=${openDeclaredSurface}>
      ${excelTabs({ controls: bindings, includeViewTabs: true })} ${excelContextualSets({ controls: bindings })}
    </mjx-ribbon>
    <mjx-menu id="ribbons-xl-paste" label="Paste options" floating>
      <mjx-menu-section label="Paste">
        <mjx-menu-item kind="radio" label="All" checked></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Values"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Formats"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Transpose"></mjx-menu-item>
      </mjx-menu-section>
      <mjx-menu-separator></mjx-menu-separator>
      <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
    </mjx-menu>

    ${insertMenus('excel', 'ribbons')} ${drawMenus('excel', 'ribbons')}
    ${designLayoutMenus('excel', 'ribbons')} ${referencesTransitionsFormulasMenus('excel', 'ribbons')}
    ${mailingsAnimationsDataMenus('excel', 'ribbons')} ${reviewMenus('excel', 'ribbons')}
    ${viewMenus('excel', 'ribbons')} ${printPreviewMenus('excel', 'ribbons')}
    ${tableToolsMenus('excel', 'ribbons')} ${pictureToolsMenus('excel', 'ribbons')}
    ${drawingToolsMenus('excel', 'ribbons')} ${chartToolsMenus('excel', 'ribbons')}
  `;
}

/**
 * **File** — seven groups, and the only one of the three that has no Print and does have a Publish.
 *
 * Both differences are the census's rather than this catalogue's, and this file's header is where
 * they are recorded. What they look like here:
 *
 * 1. **No Print group at all.** The strip goes Info, Open, Save, Share, Export, Publish, Help, and
 *    the Print group the other two collapse second simply is not there to collapse.
 * 2. **Publish is Excel's Power BI page**, three controls, which is the one group on any of the
 *    three File tabs where what Office shows and what the census counts agree exactly. Its headline
 *    carries no icon: Fluent draws no Power BI mark, and every generic candidate already means one
 *    of the two commands underneath it.
 * 3. **Info carries a fifth command**, Workbook Statistics, which Word and PowerPoint have no
 *    equivalent of.
 *
 * See `Ribbons/Word → File` for the collapse order and for the rest of the reasoning.
 */
export const File: Story = { render: () => ribbon('file') };

/**
 * **Home** — all eight in-scope groups, and the ribbon programme's unit 2. See `Ribbons/Word →
 * Home` for the two rules that shape every Home tab. What is Excel's own:
 *
 * 1. **Cells arrives with this unit** — Insert, Delete, Format — declared since unit 0 and rendered
 *    by nothing, which meant this catalogue's Excel could not insert a row.
 * 2. **Power Options arrives too, and it is the one group in this catalogue whose content is
 *    honestly unknown.** The census gives an id, a count of one and an in-scope flag, and names no
 *    control; every Office command it might be is a guess. So the command takes the group's own
 *    label and no icon, and `dev/ribbons/census.ts` says so at length — including that the census
 *    marks Excel's *Analyze Data* out of scope two rows away, which is the best evidence available
 *    that this is not that.
 * 3. **Alignment is eleven commands and seven of them draw pressed.** Top, Middle and Bottom Align,
 *    Wrap Text, and Left, Centre and Right are all toggles — Bottom and Centre pressed, because an
 *    unformatted cell is bottom-aligned. Until unit 2b only three could be, which is where the
 *    ceiling described in `Ribbons/Word → Home` was sharpest. Left, Centre and Right are the
 *    survivors, and they draw on the second row where Office draws them, not ahead of Top Align.
 *    Clipboard, Number, Styles, Cells and Editing keep no survivor: every candidate opens a menu —
 *    AutoSum included, which is a split button in Office.
 * 4. **Underline arrived with this unit.** The migrated shell set had Bold and Italic alone, which
 *    made Excel the one application whose Font group could not underline a cell.
 * 5. **Three commands in Number carry no icon** — Comma Style, Increase Decimal, Decrease Decimal.
 *    The last two were drawn with a plus and a minus, which is what *insert* and *delete* mean
 *    everywhere else on this tab; Office draws `.00` with an arrow and Fluent draws no such thing.
 */
export const Home: Story = { render: () => ribbon('home') };

/**
 * **Insert** — ten groups and thirty-five commands, the largest of the three, and the ribbon
 * programme's unit 3. See `Ribbons/Word → Insert` for the two rules that shape every Insert tab. What
 * is Excel's own:
 *
 * 1. **Charts is eleven commands, and eight of them are glyphs alone** — the one place on any Insert
 *    tab where Office draws no names. Each chart family's accessible name is Office's tooltip,
 *    *Insert Column or Bar Chart* and the rest, and each opens a menu of real chart types under
 *    Office's own section headings. **Insert Combo Chart is the one labelled command in the row**:
 *    Fluent draws columns and lines, never both in one picture. Charts also carries the only dialog
 *    launcher on any application's Insert tab, because Excel has one there.
 * 2. **PivotTable, the headline of the tab, carries no icon.** Fluent draws no pivot, and the nearest
 *    pictures say *refresh* and *swap*. It is a labelled split button beside Recommended PivotTables,
 *    which carries no glyph either, and a large Table.
 * 3. **The group Office calls Filters is labelled Slicers**, which is the census's `GroupSlicerInsert`.
 *    Its commands are Office's: Slicer and Timeline.
 * 4. **Cell Controls is new in Office and drawn last** (`GUESS:`), holding Checkbox — the one command
 *    on the tab that passes demotion rules 1 and 2, and still keeps nothing, because it is its group's
 *    only command and a survivor would leave the collapsed popup empty.
 * 5. **Nineteen commands open a menu**, from PivotTable's data sources to PivotChart's two shapes;
 *    **seven carry no icon** — PivotTable, Recommended PivotTables, Insert Combo Chart, PivotChart,
 *    Win/Loss, Object and Symbol.
 *
 * Tables and Charts are the primary groups, so they are the last two standing.
 */
export const Insert: Story = { render: () => ribbon('insert') };

/**
 * **Draw**: eight groups and the ribbon programme's unit 4. See `Ribbons/Word → Draw` for what shapes
 * every Draw tab: two generations of Office's ink tools on one tab, one survivor (Select Objects),
 * and five tools of which one holds at a time. What is Excel's own:
 *
 * 1. **No Stencils group**, because Excel's census declares none. There is no Ruler, and the strip
 *    goes from Write to Input Mode.
 * 2. **Eraser is a plain toggle, not a split button.** Excel's census counts five controls in Write,
 *    exactly its five tools, so nothing is behind an arrow. Press it and it draws pressed, releasing the
 *    tool that held; nothing opens. Compare `Ribbons/PowerPoint → Draw`, where the same command has an
 *    arrow.
 * 3. **Five commands open something**: Add Pen, Pens, Colour, Thickness and Touch/Mouse Mode.
 */
export const Draw: Story = { render: () => ribbon('draw') };

/**
 * **Page Layout**: five groups and twenty-four commands, and Excel's part of the ribbon programme's unit
 * 5. Themes, Page Setup, Scale to Fit, Sheet Options and Arrange. What to look at:
 *
 * 1. **Scale to Fit is three fields.** Width and Height are dropdowns (Automatic, 1 page …), and Scale
 *    is a combo box: pick 75% or type 80%. A percentage is not a length, so it is not a measure input.
 * 2. **Sheet Options is four checkboxes**: View Gridlines and View Headings ticked, Print Gridlines and
 *    Print Headings not, as in a new workbook. Office draws them under *Gridlines* and *Headings*
 *    headings; here each checkbox carries the full name.
 * 3. **Arrange is Word's Layout Arrange without Position and Wrap Text**, from the same declaration.
 *    Bring Forward and Send Backward are large split buttons at its head, as Excel draws them. Selection
 *    Pane is a toggle with no icon.
 * 4. **Themes, Colours, Fonts and Effects open the same menus as Word's Design tab.** Themes carries no
 *    icon, so it is small. In Page Setup, only Margins and Orientation are large; Size, Print Area,
 *    Background and Print Titles have no glyph, and Breaks sits in their column.
 * 5. **Three dialog launchers**, on Page Setup, Scale to Fit and Sheet Options, all opening Page Setup
 *    as Office's do. Nothing survives a collapse, and Page Setup is the primary group.
 */
export const PageLayout: Story = { render: () => ribbon('page-layout') };

/**
 * **Formulas**: the function library and the tools that check it, and Excel's part of the ribbon
 * programme's unit 6. Four groups: Function Library, Named Cells, Formula Auditing and Calculation. What
 * to look at:
 *
 * 1. **The function categories are Excel's own books.** Recently Used, Financial, Logical, Text, Date &
 *    Time, Lookup & Reference and Math & Trig each draw a book with their mark on it, and More Functions
 *    the plain book. Check that each glyph reads at 20 pixels. Date & Time, Lookup & Reference and Math &
 *    Trig are small: three tokens do not fit a large button.
 * 2. **Nine commands in Function Library open something.** AutoSum is a split button (Sum, Average, Count
 *    Numbers, Max, Min). Each category opens ten of its functions, then *Insert Function… Shift+F3*.
 *    More Functions opens Office's six further categories. Insert Function is the plain *fx* button.
 * 3. **The second group is labelled Named Cells, and Office calls it Defined Names.** The census wins.
 *    Define Name is a split button. Use in Formula lists the names the name box shows (Revenue,
 *    CostOfSales, Headcount, Q1, PrintArea), then Paste Names.
 * 4. **Formula Auditing has two split buttons and a toggle.** Remove Arrows and Error Checking open their
 *    menus from the arrow. Press Show Formulas and it draws pressed. Watch Window carries no icon, so it
 *    is small where Office draws it large.
 * 5. **Calculation Options opens Automatic, checked, and the two other modes.** No dialog launchers.
 *    Nothing survives a collapse, and Function Library is the primary group. Office's Python groups are
 *    out of scope in the census and are not drawn.
 */
export const Formulas: Story = { render: () => ribbon('formulas') };

/**
 * **Data**: where a workbook meets the world outside it, and Excel's part of the ribbon programme's unit 7.
 * Nine groups: Get External Data, Queries & Connections, Workbook Links, Connections, Data Types, Sort &
 * Filter, Data Tools, Forecast and Outline. What to look at:
 *
 * 1. ⚠ **There is no Get Data.** The census marks Microsoft 365's Get & Transform Data (Power Query) out
 *    of scope, and its in-scope Get External Data is Office 2016's legacy group: From Access, From Web,
 *    From Text, From Other Sources (a dropdown of the legacy wizards) and Existing Connections.
 * 2. ⚠ **Three groups are one Office group in three generations**, each command drawn once. Queries &
 *    Connections holds Refresh All (a large split button), the Queries & Connections toggle and
 *    Properties; Workbook Links holds its toggle; Connections holds Connections and Edit Links. `GUESS:`
 *    the reading.
 * 3. **Sort A to Z and Sort Z to A are the tab's survivors.** Drag narrow until Sort & Filter collapses:
 *    the two sort glyphs stay beside its trigger. **Filter is a large toggle**: press it and it draws
 *    pressed. It does not survive, because its funnel is also Insert's Slicer.
 * 4. **Data Types is an in-ribbon gallery** of Stocks, Currencies and Geography, each drawn with an icon
 *    (`building-bank`, `money`, `map`), nothing selected. `GUESS:` that an icon renders inside a gallery
 *    cell; a blank cell is the finding.
 * 5. **Data Validation, Group and Ungroup are split buttons**, and What-If Analysis is a dropdown
 *    (Scenario Manager, Goal Seek, Data Table). Outline has the tab's one dialog launcher. Text to
 *    Columns, Remove Duplicates, Consolidate, Manage Data Model, Ungroup and Subtotal carry no icon and are
 *    labelled.
 */
export const Data: Story = { render: () => ribbon('data') };

/**
 * **Review**: the tab where a workbook is read by somebody else, authored after Word's and PowerPoint's
 * under the same one-tab-one-application rule. Eleven groups: Proofing, Performance, Accessibility,
 * Language, Threaded Comments, Comments, Notes, Protect, Changes, Ink and Debug. What to look at, least
 * certain first:
 *
 * 1. ⚠ **Debug is one button labelled Debug, and all of it is `GUESS:`.** The census names the group and
 *    counts one control, nothing more. It is a plain labelled button that opens nothing here.
 * 2. ⚠ **Threaded Comments, Comments and Notes are one Office group in three generations.** Threaded
 *    Comments is Microsoft 365's five. Comments is Office 2016's, drawn as the two toggles no other group
 *    has on its face: Show/Hide Comment and Show All Comments. Press either and it draws pressed. Notes is
 *    one large dropdown: New Note (Shift+F2), Previous Note, Next Note, Show/Hide Note and Show All Notes
 *    (both unticked), then Convert to Comments. `GUESS:` the reading.
 * 3. ⚠ **Changes is Office 2016's legacy group**: Share Workbook, Protect and Share Workbook (both labels
 *    alone), and Track Changes, a small dropdown of Highlight Changes… and Accept/Reject Changes.
 *    Microsoft 365 hides all three unless the ribbon is customised.
 * 4. ⚠ **Group order.** Performance is second, after Proofing, and Ink is after Changes, where
 *    Microsoft 365 draws them; the census declares Performance tenth and Ink seventh. Insights (Smart
 *    Lookup) and Lineage are out of scope in the census and are not drawn.
 * 5. **Protect**: Protect Sheet (large, a grid with a padlock), Protect Workbook (a large toggle, pressed
 *    once pressed, File's padlock glyph), then Allow Edit Ranges and Unshare Workbook, both labels alone.
 *    Office greys Unshare Workbook in a workbook that is not a legacy shared one; it is available here.
 * 6. **Previous Comment and Next Comment are the tab's only survivors.** Drag narrow until Threaded
 *    Comments collapses: the two comment arrows stay beside its trigger. Show Comments is a large plain
 *    toggle with no arrow.
 * 7. **The other two menus.** Check Accessibility's arrow: Check Accessibility, Alt Text, then Options:
 *    Accessibility. Hide Ink is a split button whose face is a toggle: Hide Ink, then Delete All Ink on
 *    Sheet. Check Performance is a small speedometer. Spelling and Translate are large plain buttons. No
 *    dialog launchers.
 */
export const Review: Story = { render: () => ribbon('review') };

/**
 * **View**: how a workbook is looked at, never the workbook itself. Authored after Word's and PowerPoint's
 * View, one tab of one application. Seven groups: Sheet View, Workbook Views, Show, Zoom, Window, Night Mode
 * and Debug. What to look at, least certain first:
 *
 * 1. ⚠ **Page Break Preview is large with a three-word label.** It should wrap to *Page Break* over
 *    *Preview* without an ellipsis, beside Normal (pressed, a grid) and Page Layout (a printed page).
 *    `GUESS:` that it fits.
 * 2. ⚠ **Night Mode and Debug are `GUESS:` from end to end.** Night Mode is Word's reading, one large Switch
 *    Modes toggle; Debug is one small labelled button with no icon, as on Excel's Review. Both are drawn
 *    last.
 * 3. ⚠ **Sheet View is drawn first**, where Microsoft 365 draws it; the census declares it fifth. Its
 *    dropdown reads *Default* and lists Default alone. Keep, Exit, New and Options are small buttons with
 *    a disk, an exit arrow, a plus and a cog, all available though Office greys most of them here.
 * 4. ⚠ **100% and Zoom to Selection are the tab's only survivors.** Drag narrow until Zoom collapses: both
 *    stay beside the trigger, and Zoom opens from it. `GUESS:` that *1:1* and the magnifier in fit corners
 *    read with no label.
 * 5. **Workbook Views is one exclusive set.** Press Page Layout and Normal releases; press Page Layout again
 *    and it stays. Custom Views is a small button (a window with a list), not in the set.
 * 6. **Window's two menus.** Freeze Panes (large, a grid with a held row and column): Freeze Panes, Freeze
 *    Top Row, Freeze First Column, each with its glyph and a one-line description. Switch Windows: one
 *    window, *1 Findings*, checked. Split, View Side by Side and Synchronous Scrolling fill while pressed;
 *    Hide (eye struck through), Unhide (eye) and Reset Window Position (two columns) are plain buttons.
 * 7. **Show is four checkboxes, all ticked**: Ruler, Gridlines, Formula Bar, Headings. No dialog launchers.
 */
export const View: Story = { render: () => ribbon('view') };

/**
 * **Print Preview**: the sheet as it will print, a page at a time, and a view tab Office shows only inside Print
 * Preview. Authored after PowerPoint's Print Preview, one tab of one application, and Excel's second view tab.
 * Three groups: Print, Zoom and Preview. What to look at, least certain first:
 *
 * 1. ⚠ **The groups read Print, Zoom, Preview**, Office's order; the census declares Zoom last. Drag narrow and
 *    Zoom still collapses first, because it is `ancillary`.
 * 2. ⚠ **Page Setup draws a cog**, large beside Print, where Word's and PowerPoint's tabs draw Options. `GUESS:`
 *    that it reads as the page's settings; Fluent draws no page with a cog.
 * 3. ⚠ **Show Margins is an unticked checkbox, not a toggle button.** It sits under Next Page and Previous Page, in
 *    one column. Tick it: it ticks, and nothing else changes. `GUESS:` the shape and the start; the brief listed a
 *    toggle.
 * 4. ⚠ **Zoom is a large magnifier alone in its group**, a plain button: pressing it does not stay pressed and
 *    opens nothing. `GUESS:` that Office does not draw it pressed while magnified.
 * 5. **Two survivors.** Drag narrow until Preview collapses: a page with an arrow down and a page with an arrow up
 *    stay beside the trigger, and Show Margins and Close Print Preview open from it. Collapse Zoom: its trigger
 *    stands alone and Zoom opens from it.
 * 6. **Close Print Preview is large with a three-word label**, under the cross in a square. It should wrap without
 *    an ellipsis. **No dialog launcher** on any group, and **no button opens a menu**.
 * 7. **Not in `Shell/Excel`**: the shell's strip has no Print Preview tab.
 */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/**
 * **Background Removal**: the two pencils that correct Office's guess at a picture's background, and the two
 * ways out. A view tab Office shows only while a picture's background is being removed. Authored after Word's
 * and PowerPoint's, one tab of one application, from the same census functions under Excel's ids. Two groups:
 * Refine and Close. What to look at, least certain first:
 *
 * 1. ⚠ **The pencils hold at most one, and start with neither.** Press Mark Areas to Keep: it fills. Press
 *    Mark Areas to Remove: it fills and Keep releases. Press Remove again: it releases, and neither is
 *    pressed. `GUESS:` the release on a second press and the empty start; View's Workbook Views set, by
 *    contrast, keeps one pressed.
 * 2. **The set is Excel's own.** Inspect a pencil: its `exclusive` attribute is
 *    `excel.background-removal.refine`, not Word's or PowerPoint's.
 * 3. ⚠ **Refine has two commands, not three.** Delete Mark, which Office 2010 to 2016 drew, is absent because
 *    Microsoft 365 no longer draws it. `GUESS:`.
 * 4. ⚠ **Four circles**: a plus and a minus for the pencils, a cross and a tick for Discard All Changes and
 *    Keep Changes. `GUESS:` every glyph, as on Word's.
 * 5. **All four are large, and the long labels wrap.** *Mark Areas to Remove* and *Discard All Changes* should
 *    wrap to two lines without an ellipsis.
 * 6. **No survivors.** Drag narrow until both groups collapse: each popup trigger stands alone and opens its
 *    commands in order. The pencils' set still holds one at most when pressed inside the popup.
 * 7. **Not in `Shell/Excel`**: the shell's strip has no Background Removal tab. No menus, no dialog
 *    launchers.
 */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };

/**
 * **Table Design**: a worksheet table's name and range, what it turns into, where its data comes from, which parts its
 * style sets apart, and the style it wears. Table Tools' one tab in Excel, and Excel's first contextual tab authored;
 * Office shows it only while the active cell is in a table. ⚠ **Excel's Table Tools is its own census set**,
 * `TabSetTableToolsExcel`, with **no Layout tab**: a worksheet table's rows and columns are the sheet's. Five groups:
 * Properties, Tools, External Table Data, Table Style Options and Table Styles. See `Ribbons/Word → Table Design` for
 * the band. What to look at, least certain first:
 *
 * 1. ⚠ **Table Name is a combo box**, where Office draws a plain text box. It reads *Table1*; type a new name and press
 *    Enter, and it keeps the text. **Its arrow opens a list of one, Table1**, because the catalogue has no plain text
 *    field. Judge whether that list is acceptable; it is the weakest part of the tab's shape.
 * 2. ⚠ **The gallery's pictures and sections.** Expand Table Styles: *Light* (22), *Medium* (28) and *Dark* (11), one
 *    family to a row of seven, **Table Style Medium 2 selected**. **Light opens with None**, the table with no style,
 *    so its rows start one cell later than Medium's; the brief listed 60 and this draws 61. Dark ends with Dark 8 to
 *    11, the three accent pairs drawn in their first accent. Under the list: New Table Style… and Clear. Hover a cell:
 *    its name is *Table Style Light 9*, the name the file carries, without the colour word Microsoft 365 adds. `GUESS:`
 *    None's place and every picture.
 * 3. ⚠ **Glyphs to judge**, all `GUESS:`. New: Resize Table's table in corner marks, **Summarize with PivotTable's
 *    turned blocks (`pivot`, the weakest; Insert's PivotTable carries no glyph)**, Convert to Range's table turning to
 *    lines, and Open in Browser's globe with an arrow. Reused: Insert Slicer is Insert's funnel, Export Recording's
 *    arrow leaving a box, Refresh Data's refresh arrow, Properties Word's table with a cog, Unlink Outlining's struck
 *    link. **Remove Duplicates has none**, as on Data.
 * 4. **Seven checkboxes in three columns**: Header Row, Total Row and Banded Rows; First Column, Last Column and Banded
 *    Columns; Filter Button alone. **Header Row, Banded Rows and Filter Button are ticked**, the table Format as Table
 *    inserts. `GUESS:` the columns and the start.
 * 5. **Export is a large dropdown**: Export Table to SharePoint List… and Export Table to Visio Pivot Diagram….
 *    **Refresh is a large split button**: the face does nothing here, and the arrow lists Refresh (Alt+F5), Refresh
 *    All (Ctrl+Alt+F5), Refresh Status, Cancel Refresh, then Connection Properties….
 * 6. **Properties, Open in Browser and Unlink are available**, small in a column. Office greys them, and Refresh, for
 *    a table with no external source; nothing here tracks a source.
 * 7. **Tools**: Summarize with PivotTable, Remove Duplicates and Convert to Range small in a column, then Insert Slicer
 *    large. Properties: Table Name over Resize Table.
 * 8. **No dialog launcher and no survivor anywhere.** Drag narrow: External Table Data is `secondary` and gives way
 *    first, then Properties and Tools (`standard`), and Table Style Options and Table Styles (`primary`) last. Each
 *    collapses to a trigger with nothing beside it.
 * 9. **Also in `Shell/Excel`**, which draws Table Tools: select Table Design there and every list, menu and starting
 *    state above is the same, under the shell's own ids.
 */
export const TableDesign: Story = { render: () => ribbon('table-design') };

/**
 * **Picture Format**: how a picture floating over a worksheet is corrected, framed, described, placed among the sheet's
 * objects, cropped and sized. Picture Tools' one tab, and Excel's second contextual tab authored; Office shows it only
 * while a picture is selected. Six groups: Adjust, Picture Styles, Accessibility, Arrange, Size and Image Play. **It is
 * Word's tab through Word's functions**, so see `Ribbons/Word → Picture Format` for every list; what to look at here is
 * where Excel's differs, and what is still least certain, least certain first:
 *
 * 1. ⚠ **Image Play is the census's, not the brief's.** Play Animation, a large toggle, starts pressed. `GUESS:` all of
 *    it, as on Word's and PowerPoint's.
 * 2. ⚠ **Picture Border has no Eyedropper**: open it, and beneath the palette read More Outline Colours…, Weight ▸,
 *    Sketched ▸ and Dashes ▸, exactly Word's. *No Outline* is the chip, starting on none. PowerPoint's has an
 *    Eyedropper; `GUESS:` that Excel's has none.
 * 3. ⚠ **Arrange has six commands, and Bring Forward and Send Backward are large** at the head of the group, as on Page
 *    Layout, where Word and PowerPoint draw them small. Their arrows list Bring Forward and Bring to Front, and Send
 *    Backward and Send to Back, **without Word's text layers**. **Align** opens the six alignments and two
 *    distributions, then **Snap to Grid, Snap to Shape and View Gridlines, the last ticked**. Group opens Group, Regroup
 *    and Ungroup; Rotate its four turns and flips and More Rotation Options…. Selection Pane is a small toggle with no
 *    glyph. **No Position or Wrap Text.**
 * 4. ⚠ **Height and Width start on 9.53 cm and 12.7 cm**, a 640 × 480 photograph at its own size, stepping by 0.01.
 *    `GUESS:` both numbers. They do not follow each other, because nothing dispatches.
 * 5. **Two launchers**: *Format Picture* at Picture Styles' corner, and **Size and Properties** at Size's, where Word's
 *    says Layout and PowerPoint's Size and Position. `GUESS:` both labels.
 * 6. **Picture Layout, as Word says**, third in Picture Styles' small column, with the diagram glyph, opening the
 *    thirty-one picture layouts. PowerPoint calls the same button Convert to SmartArt.
 * 7. **Adjust is Word's**: Remove Background, Corrections, Colour, Artistic Effects and Transparency large; Compress
 *    Pictures, Change Picture and Reset Picture small. Press Corrections, Colour, Artistic Effects and Transparency:
 *    each is Word's whole list with the unchanged state checked. Remove Background is a plain button, and in Office
 *    opens the Background Removal tab, which is authored (`BackgroundRemoval`).
 * 8. **The Quick Styles gallery is Word's twenty-eight**, drawn in this workbook's palette. Nothing is selected.
 *    `GUESS:` every look.
 * 9. **Crop is a split toggle.** Press its face: it fills; again, it releases. Its arrow opens Crop, Crop to Shape (147
 *    shapes), Aspect Ratio and Fill and Fit.
 * 10. **Picture Effects** opens Preset, Shadow, Reflection, Glow, Soft Edges, Bevel and 3-D Rotation.
 * 11. **Alt Text is a large toggle**, unpressed. No pane opens, nor does Selection Pane's, Compress Pictures' dialog or
 *     either launcher's.
 * 12. **Glyphs**: every one is Word's Picture Format's or Arrange's, reused; **Remove Background's is still the
 *     weakest**. The gallery, Picture Border, Height, Width and Selection Pane carry none.
 * 13. **No survivor anywhere.** Drag narrow: Image Play (`ancillary`) gives way first, Accessibility (`secondary`) next,
 *     then Arrange and Size, and Adjust and Picture Styles last; each collapses to a trigger with nothing beside it.
 * 14. **Not in `Shell/Excel`**, which draws Table Tools: the shell's strip has no Picture Tools band.
 */
export const PictureFormat: Story = { render: () => ribbon('picture-format') };

/**
 * **Shape Format**: which shape a shape over the worksheet is, how it is filled, outlined and given effects, how its
 * text is dressed, how it is described, where it sits among the sheet's objects, and its size. Drawing Tools' one tab,
 * and Excel's third contextual tab authored; Office shows it only while a shape, a text box or a WordArt is selected.
 * Six groups: Insert Shapes, Shape Styles, WordArt Styles, Accessibility, Arrange and Size. **It is PowerPoint's Shape
 * Format wherever Office's Excel is**, so `Ribbons/PowerPoint`'s `ShapeFormat` story covers the Theme Styles pictures,
 * Other Theme Fills, the Shapes list and Edit Shape; and Arrange is this file's `PictureFormat`'s. What to look at here,
 * least certain first:
 *
 * 1. ⚠ **Text Fill and Text Outline are PowerPoint's less the Eyedropper**, not Word's shorter pair. Text Fill starts on
 *    Background 1 (the white text of an inserted shape), with *No Fill*, More Fill Colours…, Picture…, Gradient ▸ and
 *    Texture ▸ beneath the palette; Text Outline starts on none, with *No Outline*, More Outline Colours…, Weight ▸,
 *    Sketched ▸ and Dashes ▸. `GUESS:` all of it, Sketched most.
 * 2. ⚠ **No Eyedropper anywhere.** Shape Fill starts on Accent 1 with More Fill Colours…, Picture…, Gradient ▸ and
 *    Texture ▸; Shape Outline on Accent 1, Darker 50%, with More Outline Colours…, Weight ▸, Sketched ▸, Dashes ▸ and
 *    Arrows ▸. `GUESS:` both starts, and that recent builds have not brought the Eyedropper to Excel.
 * 3. ⚠ **Arrange is Picture Format's six, not Word's eight**: **Bring Forward and Send Backward large** split buttons at
 *    the head, their arrows without Word's text layers; Selection Pane small with no glyph; **Align ending on Snap to
 *    Grid, Snap to Shape and View Gridlines, the last ticked**; Group and Rotate. No Position or Wrap Text.
 * 4. **Text Box is a small split button**, beside Edit Shape, as Word's Draw Text Box is. Press its face: nothing opens,
 *    because it arms a drawing gesture. Press its arrow: *Draw Horizontal Text Box* and *Vertical Text Box*, the two
 *    Excel's Insert tab offers. **No Merge Shapes** beside it (`GUESS:` that Excel still lacks it).
 * 5. **Shapes** opens the whole gallery with **no Action Buttons and no New Drawing Canvas**, the list `Insert → Shapes`
 *    opens. **Edit Shape**'s Change Shape has no Action Buttons either, and Reroute Connectors is unavailable.
 * 6. **Height and Width start on 2.54 cm**, stepping by 0.01. `GUESS:` both. They do not follow each other.
 * 7. **Three launchers**: *Format Shape* at Shape Styles' corner, *Format Text Effects* at WordArt Styles', and **Size
 *    and Properties** at Size's, as on Picture Format, where PowerPoint's says *Size and Position* and Word's *Layout*.
 *    `GUESS:` all three.
 * 8. **Theme Styles and Quick Styles** are PowerPoint's galleries in this workbook's palette, Other Theme Fills in the
 *    first one's footer; **Shape Effects** opens Picture Effects' seven submenus; **Text Effects** WordArt's six.
 * 9. **Alt Text** is a large toggle, unpressed; its glyph, a picture with a label, is **the weakest on the tab**. No pane
 *    opens, nor does Selection Pane's or any launcher's.
 * 10. **Glyphs**: eleven, every one reused (Shapes, Edit Shape, Text Box, Shape Effects, Text Effects, Alt Text and
 *     Arrange's five). The two galleries, the four colour pickers, Height, Width and Selection Pane carry none.
 * 11. **No survivor anywhere.** Drag narrow: Accessibility (`secondary`) gives way first, then Insert Shapes, WordArt
 *     Styles, Arrange and Size (`standard`), and Shape Styles (`primary`) last; each collapses to a trigger with nothing
 *     beside it.
 * 12. **Not in `Shell/Excel`**, which draws Table Tools: there is no Drawing Tools band there and none of these menus is
 *     on that page.
 */
export const ShapeFormat: Story = { render: () => ribbon('shape-format') };

/**
 * **Chart Design**: which elements a chart on the worksheet carries and how they are laid out, which colours and style
 * it wears, which cells it plots, what kind of chart it is, and where in the workbook it lives. Chart Tools' first tab,
 * and Excel's fourth contextual tab authored; Office shows it only while a chart is selected. Five groups: Chart
 * Layouts, Chart Styles, Data, Type and **Location**, every command large. **It is `Ribbons/Word`'s Chart Design where
 * Office's Excel is**, through the same functions, so judge the three side by side: Chart Layouts, Chart Styles and
 * Type must match Word's and PowerPoint's exactly, and Data and Location are the only places Excel's may differ. It is
 * the census's `TabChartToolsDesignNew`. What to look at here, least certain first:
 *
 * 1. ⚠ **Move Chart's glyph is the weakest on the tab**: four arrows, which say *move* and not *chart* or *sheet*. Office
 *    draws a chart with an arrow leaving it. Pressing it opens nothing, where Office opens the Move Chart dialog (*New
 *    sheet* or *Object in*). It is **Location**, Excel's own group, holding nothing else. `GUESS:` the glyph.
 * 2. ⚠ **Data is two commands, not four**: Switch Row/Column and Select Data, large, and **no Edit Data or Refresh
 *    Data**, because a workbook's chart reads its own cells. There is no split button on this tab. Judge the two glyphs
 *    together: a table with a turn arrow, a table with a pointer.
 * 3. ⚠ **Change Chart Type opens a menu, where Office opens a dialog.** Its eight submenus are exactly the lists this
 *    ribbon's own Insert tab opens for each family, each ending on *More … Charts…*; compare them with `Insert`. **No
 *    Map family**, though Excel's dialog lists one (Insert's Maps reaches it). `GUESS:` both.
 * 4. ⚠ **Add Chart Element's starts are Word's**, read as the Clustered Column Excel inserts: **Chart Title** (*Above
 *    Chart*), **Legend** (*Bottom*), **Gridlines** (*Primary Major Horizontal* ticked), **Axes** (both ticked), **Axis
 *    Titles** (neither), **Data Labels**, **Data Table**, **Error Bars**, **Lines** and **Up/Down Bars** (each on *None*)
 *    and **Trendline** (plain entries). Each ends on its *More … Options…*. **Lines and Up/Down Bars open**, where Office
 *    greys both on a column chart. `GUESS:` that Excel's inserted chart starts as Word's.
 * 5. ⚠ **The Chart Styles gallery**, *Style 1* to *Style 16*, starting on Style 1, in the catalogue's one specimen theme,
 *    so the pictures match Word's and PowerPoint's exactly. Each describes a look rather than rendering Office's.
 *    `GUESS:` sixteen, and every look.
 * 6. **Quick Layout**: four tiled regions, reading *arrange windows* first; its menu is *Layout 1* to *Layout 11*, names
 *    where Office draws thumbnails.
 * 7. **Change Colours** opens *Colourful* (Palettes 1–4) and *Monochromatic* (Palettes 1–13), one set, on *Colourful
 *    Palette 1*. `GUESS:` both counts.
 * 8. **The census's priorities differ from Word's**, and the collapse shows it. Drag narrow: **Location** (`ancillary`)
 *    gives way first, then Data and Type (`secondary`, where Word's Data is `standard`), and **Chart Layouts and Chart
 *    Styles** (`primary`, where Word's Chart Styles is `secondary`) last; each collapses to a trigger with nothing beside
 *    it. **No survivor anywhere.**
 * 9. **The spelling is the catalogue's**: *Change Colours*, *Colourful*, *Centred Overlay*, *Centre*.
 * 10. **No dialog launcher** on any group.
 * 11. **Not in `Shell/Excel`**, which draws Table Tools: there is no Chart Tools band there and none of these menus is on
 *     that page.
 */
export const ChartDesign: Story = { render: () => ribbon('chart-design') };

/**
 * **Format** — Chart Tools' second tab, a placeholder. Seven groups: Current Selection, Insert Shapes, Shape Styles,
 * WordArt Styles, Accessibility, Arrange and Size.
 */
export const ChartFormat: Story = { render: () => ribbon('chart-format') };
