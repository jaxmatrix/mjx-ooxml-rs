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
import { designLayoutMenus } from './design-layout-menus.ts';
import { drawMenus } from './draw-menus.ts';
import { excelContextualSets, excelTabs } from './excel.ts';
import { insertMenus } from './insert-menus.ts';
import { dataTypeGalleryItems, mailingsAnimationsDataMenus } from './mailings-animations-data-menus.ts';
import { referencesTransitionsFormulasMenus } from './references-transitions-formulas-menus.ts';

/**
 * **Excel's ribbon, tab by tab** — the same functions `Shell/Excel` composes.
 *
 * ⚠ **Excel's File tab has no Print group**, and that is the census rather than an omission: the
 * committed command surface carries a backstage `TabPrint` row for Word and PowerPoint and none for
 * Excel, and carries a `Publish2Tab` the other two lack. Excel obviously has a File → Print page,
 * so this is a gap in the dump — but the census is the checked source and inventing the row would
 * be the drift the transcription exists to prevent. `dev/ribbons/census.ts` is where it is recorded.
 *
 * **File, Home, Insert, Draw, Page Layout, Formulas and Data** are authored; the rest are placeholders at the census's own priorities. See
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
          'Excel’s ten core tabs and its File tab, each shown selected inside the whole ribbon. ' +
          'File, Home, Insert, Draw, Page Layout, Formulas and Data are authored; the rest are placeholders carrying the census’s ' +
          'priorities.',
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
      ${excelTabs({ controls: bindings, includeViewTabs: true })} ${excelContextualSets()}
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
    ${mailingsAnimationsDataMenus('excel', 'ribbons')}
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
 * and tools that draw pressed but do not yet release each other. What is Excel's own:
 *
 * 1. **No Stencils group**, because Excel's census declares none. There is no Ruler, and the strip
 *    goes from Write to Input Mode.
 * 2. **Eraser is a plain toggle, not a split button.** Excel's census counts five controls in Write,
 *    exactly its five tools, so nothing is behind an arrow. Press it and it draws pressed; nothing
 *    opens. Compare `Ribbons/PowerPoint → Draw`, where the same command has an arrow.
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

/** Unit 8. */
export const Review: Story = { render: () => ribbon('review') };

/** Unit 9. */
export const View: Story = { render: () => ribbon('view') };

/** Unit 10, and a view tab — see `dev/ribbons/census.ts`. */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/** A view tab: Office shows it only while a picture's background is being removed. */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };
