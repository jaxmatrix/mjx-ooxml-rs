import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MiniCommand } from '../../src/feedback/feedback-model.ts';
import type { VirtualItem } from '../../src/navigators/virtual-list.ts';
import { workbookTabs } from '../navigators/specimens.ts';
import { multiReferenceFormula, workbookNames } from '../formula/specimens.ts';
import { documentThemePalette, machineFonts, recentColors, standardColors } from '../pickers/specimens.ts';
import { largeGalleryItems } from '../gallery/specimens.ts';
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
  openDeclaredSurface,
  openSheetOnCommand,
  paneHeading,
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
import { excelContextualSets, excelTabs } from '../ribbons/excel.ts';
import { designLayoutMenus } from '../ribbons/design-layout-menus.ts';
import { drawMenus } from '../ribbons/draw-menus.ts';
import { insertMenus } from '../ribbons/insert-menus.ts';
import { dataTypeGalleryItems, mailingsAnimationsDataMenus } from '../ribbons/mailings-animations-data-menus.ts';
import { referencesTransitionsFormulasMenus } from '../ribbons/references-transitions-formulas-menus.ts';
import { reviewMenus } from '../ribbons/review-menus.ts';
import {
  excelTableName,
  excelTableStyleGalleryFooter,
  excelTableStyleGalleryItems,
  tableToolsMenus,
} from '../ribbons/table-tools-menus.ts';
import { excelSheetViews, viewMenus } from '../ribbons/view-menus.ts';
import { fitPageCounts, scalePercentages } from '../ribbons/ribbon-parts.ts';

/**
 * **Excel, assembled** — the ribbon, the name box and formula bar, the grid, a task pane, the sheet
 * tab bar and the status bar, at desktop, tablet and phone.
 *
 * Excel is the shell with the most **horizontal bands**: five of them stacked above and below the
 * grid, each one a component with its own idea of how much block space it deserves. That is the
 * composition question here — whether the grid still reads as the thing the window is for.
 *
 * What to look at:
 *
 * 1. **The bands.** Ribbon, formula row, grid, sheet tabs, status bar. Their heights were each
 *    settled in a story of their own and never against one another.
 * 2. **The formula bar.** Type `=SU` into it: the completion list opens over the grid. Move the
 *    caret through `${'=SUM(A1:A9)+B2-Sheet2!C3*$A$1'}` and watch the references take their colours —
 *    which is the first time those four colours are seen against a ribbon rather than on a page of
 *    their own.
 * 3. **The sheet tab bar at tablet.** Ten sheets in 834 pixels: the strip overflows and its four
 *    scroll affordances appear, beside a status bar that is demoting readings at the same time.
 *
 * **Nothing here dispatches a command**, and no cell holds a value: the grid is an honestly labelled
 * placeholder, because the renderer is Rust and is not wired into Storybook.
 */

const conventions = storyConventions({
  statesMatrix: shellStatesMatrix,
  tokenDependencies: shellTokenDependencies,
  keyboard: shellKeyboard,
  screenReader: shellScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Shell/Excel',
  parameters: {
    docs: {
      description: {
        component:
          'The whole application assembled from the catalogue’s own components: ribbon, name box ' +
          'and formula bar, grid, task pane, sheet tab bar and status bar on a desktop; a formula ' +
          'bar and a command rail on a phone. Cosmetic only — every command is inert.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

// ── the fixtures ─────────────────────────────────────────────────────────────

/** The workbook's defined names, as the Name Manager list holds them. */
const definedNameRows: readonly VirtualItem[] = workbookNames.map((entry) => ({
  id: entry.name,
  label: entry.name,
  detail: entry.definition,
}));

/** What the mini toolbar offers over a selected range. */
const cellCommands: readonly MiniCommand[] = [
  { command: 'bold', label: 'Bold', icon: 'text-bold', kind: 'toggle' },
  { command: 'italic', label: 'Italic', icon: 'text-italic', kind: 'toggle' },
  { command: 'align-center', label: 'Centre', icon: 'text-align-center', separatorBefore: true },
  { command: 'comment', label: 'New note', icon: 'comment', separatorBefore: true },
  {
    command: 'delete',
    label: 'Delete',
    icon: 'delete',
    unavailable: true,
    explanation: 'The selection includes a cell in a protected range.',
  },
];

// ── the ribbon ──────────────────────────────────────────────────────────

/**
 * **Excel's ribbon, from `stories/ribbons/excel.ts`** — the same functions `Ribbons/Excel` audits.
 *
 * The tabs used to be written here; moving them out is the ribbon programme's unit 0. What stays is
 * what belongs to an *application*: this machine's font list, this workbook's palette, the number
 * formats this locale offers, the id of the menu the paste button opens, the cell-style gallery's
 * contents. They are bound by the stable command ids `dev/ribbons/census.ts` declares.
 *
 * `excelTabs()` leaves out Print Preview and Background Removal, which Office shows only inside the
 * view they name. That is also why **Print Preview's Show Margins checkbox is bound in `Ribbons/Excel` and not
 * here**, and `printPreviewMenus` is not rendered here: a binding for a tab this strip never draws would be a
 * binding to nothing.
 */
function ribbon(): TemplateResult {
  return surface(
    'ribbon',
    'flex:0 0 auto;min-inline-size:0',
    html`
      <mjx-ribbon label="Excel" selected="home" @mjx-activate=${openDeclaredSurface}>
        ${excelTabs({
          controls: {
            'excel.home.clipboard.paste': html`<mjx-split-button
              label="Paste"
              icon="clipboard-paste"
              size="large"
              menu-label="Paste options"
              data-opens="xl-paste-menu"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.home.font.name': html`<mjx-font-picker
              id="xl-font"
              style=${ribbonFieldStyle}
              label="Font"
              value="Aptos"
              .fonts=${machineFonts}
            ></mjx-font-picker>`,
            'excel.home.font.size': html`<mjx-dropdown
              id="xl-size"
              label="Font size"
              value="11"
              style=${ribbonNarrowFieldStyle}
            >
              ${['9', '10', '11', '12', '14', '18'].map(
                (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'excel.home.font.fill': html`<mjx-color-picker
              id="xl-fill"
              style=${ribbonColourFieldStyle}
              label="Fill colour"
              show-no-fill
              .themePalette=${documentThemePalette}
              .standardColors=${standardColors}
              .recentColors=${recentColors}
            ></mjx-color-picker>`,
            'excel.home.number.format': html`<mjx-dropdown
              id="xl-number"
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
              ].map(
                (format) =>
                  html`<mjx-option value=${format.value} label=${format.label}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'excel.home.styles.gallery': html`<mjx-gallery
              id="xl-cell-styles"
              label="Cell styles"
              value="office-2"
              style=${ribbonGalleryStyle}
            >
              ${largeGalleryItems().slice(0, 18)}
            </mjx-gallery>`,
            // Insert (unit 3). Office draws each of these as a dropdown or a split button, so each opens
            // its menu from `stories/ribbons/insert-menus.ts`: a dropdown is one `<mjx-button>` whose press
            // opens the menu, a split button opens it from its arrow. `data-opens` is
            // `commandSurfaceId('shell', <this key>)`, and `tests/ribbons.test.ts` requires exactly that.
            'excel.insert.tables.pivottable': html`<mjx-split-button
              label="PivotTable"
              size="small"
              data-opens="shell-excel-insert-tables-pivottable"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.insert.illustrations.pictures': html`<mjx-button
              label="Pictures"
              icon="image"
              size="large"
              data-opens="shell-excel-insert-illustrations-pictures"
            ></mjx-button>`,
            'excel.insert.illustrations.shapes': html`<mjx-button
              label="Shapes"
              icon="shapes"
              size="large"
              data-opens="shell-excel-insert-illustrations-shapes"
            ></mjx-button>`,
            'excel.insert.illustrations.3d-models': html`<mjx-button
              label="3D Models"
              icon="cube"
              size="small"
              data-opens="shell-excel-insert-illustrations-3d-models"
            ></mjx-button>`,
            'excel.insert.illustrations.screenshot': html`<mjx-button
              label="Screenshot"
              icon="screenshot"
              size="small"
              data-opens="shell-excel-insert-illustrations-screenshot"
            ></mjx-button>`,
            'excel.insert.charts.column-bar': html`<mjx-button
              label="Insert Column or Bar Chart"
              icon="data-bar-vertical"
              size="icon"
              data-opens="shell-excel-insert-charts-column-bar"
            ></mjx-button>`,
            'excel.insert.charts.hierarchy': html`<mjx-button
              label="Insert Hierarchy Chart"
              icon="data-treemap"
              size="icon"
              data-opens="shell-excel-insert-charts-hierarchy"
            ></mjx-button>`,
            'excel.insert.charts.waterfall': html`<mjx-button
              label="Insert Waterfall, Funnel, Stock, Surface or Radar Chart"
              icon="data-waterfall"
              size="icon"
              data-opens="shell-excel-insert-charts-waterfall"
            ></mjx-button>`,
            'excel.insert.charts.line-area': html`<mjx-button
              label="Insert Line or Area Chart"
              icon="data-line"
              size="icon"
              data-opens="shell-excel-insert-charts-line-area"
            ></mjx-button>`,
            'excel.insert.charts.statistic': html`<mjx-button
              label="Insert Statistic Chart"
              icon="data-histogram"
              size="icon"
              data-opens="shell-excel-insert-charts-statistic"
            ></mjx-button>`,
            'excel.insert.charts.combo': html`<mjx-button
              label="Insert Combo Chart"
              size="small"
              data-opens="shell-excel-insert-charts-combo"
            ></mjx-button>`,
            'excel.insert.charts.pie-doughnut': html`<mjx-button
              label="Insert Pie or Doughnut Chart"
              icon="data-pie"
              size="icon"
              data-opens="shell-excel-insert-charts-pie-doughnut"
            ></mjx-button>`,
            'excel.insert.charts.scatter-bubble': html`<mjx-button
              label="Insert Scatter (X, Y) or Bubble Chart"
              icon="data-scatter"
              size="icon"
              data-opens="shell-excel-insert-charts-scatter-bubble"
            ></mjx-button>`,
            'excel.insert.charts.maps': html`<mjx-button
              label="Maps"
              icon="map"
              size="large"
              data-opens="shell-excel-insert-charts-maps"
            ></mjx-button>`,
            'excel.insert.charts.pivotchart': html`<mjx-split-button
              label="PivotChart"
              size="small"
              data-opens="shell-excel-insert-charts-pivotchart"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.insert.links.link': html`<mjx-split-button
              label="Link"
              icon="link"
              size="large"
              data-opens="shell-excel-insert-links-link"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.insert.text.wordart': html`<mjx-button
              label="WordArt"
              icon="text-effects"
              size="large"
              data-opens="shell-excel-insert-text-wordart"
            ></mjx-button>`,
            'excel.insert.text.signature-line': html`<mjx-split-button
              label="Signature Line"
              icon="signature"
              size="small"
              data-opens="shell-excel-insert-text-signature-line"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.insert.symbols.equation': html`<mjx-split-button
              label="Equation"
              icon="math-formula"
              size="large"
              data-opens="shell-excel-insert-symbols-equation"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            // Draw (unit 4). Five dropdowns, each opening its menu from `stories/ribbons/draw-menus.ts`;
            // Excel's Eraser is the plain toggle its census declares, so nothing is bound over it.
            'excel.draw.drawing-tools.add-pen': html`<mjx-button
              label="Add Pen"
              size="small"
              data-opens="shell-excel-draw-drawing-tools-add-pen"
            ></mjx-button>`,
            'excel.draw.pens.pens': html`<mjx-button
              label="Pens"
              icon="inking-tool"
              size="large"
              data-opens="shell-excel-draw-pens-pens"
            ></mjx-button>`,
            'excel.draw.pens.colour': html`<mjx-button
              label="Colour"
              icon="color-line"
              size="small"
              data-opens="shell-excel-draw-pens-colour"
            ></mjx-button>`,
            'excel.draw.pens.thickness': html`<mjx-button
              label="Thickness"
              icon="line-thickness"
              size="small"
              data-opens="shell-excel-draw-pens-thickness"
            ></mjx-button>`,
            'excel.draw.input-mode.touch-mouse-mode': html`<mjx-button
              label="Touch/Mouse Mode"
              size="small"
              data-opens="shell-excel-draw-input-mode-touch-mouse-mode"
            ></mjx-button>`,
            // Page Layout (unit 5). Dropdowns and split buttons open their menus from
            // `stories/ribbons/design-layout-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            // Scale to Fit is three fields over `ribbon-parts.ts`'s lists, and Sheet Options four checkboxes.
            'excel.page-layout.themes.themes': html`<mjx-button
              label="Themes"
              size="small"
              data-opens="shell-excel-page-layout-themes-themes"
            ></mjx-button>`,
            'excel.page-layout.themes.colours': html`<mjx-button
              label="Colours"
              icon="color"
              size="small"
              data-opens="shell-excel-page-layout-themes-colours"
            ></mjx-button>`,
            'excel.page-layout.themes.fonts': html`<mjx-button
              label="Fonts"
              icon="text-font"
              size="small"
              data-opens="shell-excel-page-layout-themes-fonts"
            ></mjx-button>`,
            'excel.page-layout.themes.effects': html`<mjx-button
              label="Effects"
              icon="square-shadow"
              size="small"
              data-opens="shell-excel-page-layout-themes-effects"
            ></mjx-button>`,
            'excel.page-layout.page-setup.margins': html`<mjx-button
              label="Margins"
              icon="document-margins"
              size="large"
              data-opens="shell-excel-page-layout-page-setup-margins"
            ></mjx-button>`,
            'excel.page-layout.page-setup.orientation': html`<mjx-button
              label="Orientation"
              icon="orientation"
              size="large"
              data-opens="shell-excel-page-layout-page-setup-orientation"
            ></mjx-button>`,
            'excel.page-layout.page-setup.size': html`<mjx-button
              label="Size"
              size="small"
              data-opens="shell-excel-page-layout-page-setup-size"
            ></mjx-button>`,
            'excel.page-layout.page-setup.print-area': html`<mjx-button
              label="Print Area"
              size="small"
              data-opens="shell-excel-page-layout-page-setup-print-area"
            ></mjx-button>`,
            'excel.page-layout.page-setup.breaks': html`<mjx-button
              label="Breaks"
              icon="document-page-break"
              size="small"
              data-opens="shell-excel-page-layout-page-setup-breaks"
            ></mjx-button>`,
            'excel.page-layout.scale-to-fit.width': html`<mjx-dropdown
              id="xl-fit-width"
              label="Width"
              value="automatic"
              style=${ribbonColourFieldStyle}
            >
              ${fitPageCounts.map(
                (count) => html`<mjx-option value=${count.value} label=${count.label}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'excel.page-layout.scale-to-fit.height': html`<mjx-dropdown
              id="xl-fit-height"
              label="Height"
              value="automatic"
              style=${ribbonColourFieldStyle}
            >
              ${fitPageCounts.map(
                (count) => html`<mjx-option value=${count.value} label=${count.label}></mjx-option>`,
              )}
            </mjx-dropdown>`,
            'excel.page-layout.scale-to-fit.scale': html`<mjx-combo-box
              id="xl-fit-scale"
              label="Scale"
              value="100%"
              allow-custom
              style=${ribbonNarrowFieldStyle}
            >
              ${scalePercentages.map((scale) => html`<mjx-option value=${scale} label=${scale}></mjx-option>`)}
            </mjx-combo-box>`,
            'excel.page-layout.sheet-options.view-gridlines': html`<mjx-checkbox id="xl-view-gridlines" label="View Gridlines" checked="true"></mjx-checkbox>`,
            'excel.page-layout.sheet-options.print-gridlines': html`<mjx-checkbox id="xl-print-gridlines" label="Print Gridlines"></mjx-checkbox>`,
            'excel.page-layout.sheet-options.view-headings': html`<mjx-checkbox id="xl-view-headings" label="View Headings" checked="true"></mjx-checkbox>`,
            'excel.page-layout.sheet-options.print-headings': html`<mjx-checkbox id="xl-print-headings" label="Print Headings"></mjx-checkbox>`,
            'excel.page-layout.arrange.bring-forward': html`<mjx-split-button
              label="Bring Forward"
              icon="position-forward"
              size="large"
              data-opens="shell-excel-page-layout-arrange-bring-forward"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.page-layout.arrange.send-backward': html`<mjx-split-button
              label="Send Backward"
              icon="position-backward"
              size="large"
              data-opens="shell-excel-page-layout-arrange-send-backward"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.page-layout.arrange.align': html`<mjx-button
              label="Align"
              icon="align-left"
              size="small"
              data-opens="shell-excel-page-layout-arrange-align"
            ></mjx-button>`,
            'excel.page-layout.arrange.group': html`<mjx-button
              label="Group"
              icon="group"
              size="small"
              data-opens="shell-excel-page-layout-arrange-group"
            ></mjx-button>`,
            'excel.page-layout.arrange.rotate': html`<mjx-button
              label="Rotate"
              icon="rotate-right"
              size="small"
              data-opens="shell-excel-page-layout-arrange-rotate"
            ></mjx-button>`,
            // Formulas (unit 6). Split buttons and dropdowns open their menus from
            // `stories/ribbons/references-transitions-formulas-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            'excel.formulas.function-library.autosum': html`<mjx-split-button
              label="AutoSum"
              icon="autosum"
              size="large"
              data-opens="shell-excel-formulas-function-library-autosum"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.formulas.function-library.recently-used': html`<mjx-button
              label="Recently Used"
              icon="book-star"
              size="large"
              data-opens="shell-excel-formulas-function-library-recently-used"
            ></mjx-button>`,
            'excel.formulas.function-library.financial': html`<mjx-button
              label="Financial"
              icon="book-coins"
              size="large"
              data-opens="shell-excel-formulas-function-library-financial"
            ></mjx-button>`,
            'excel.formulas.function-library.logical': html`<mjx-button
              label="Logical"
              icon="book-question-mark"
              size="large"
              data-opens="shell-excel-formulas-function-library-logical"
            ></mjx-button>`,
            'excel.formulas.function-library.text': html`<mjx-button
              label="Text"
              icon="book-letter"
              size="large"
              data-opens="shell-excel-formulas-function-library-text"
            ></mjx-button>`,
            'excel.formulas.function-library.date-time': html`<mjx-button
              label="Date & Time"
              icon="book-clock"
              size="small"
              data-opens="shell-excel-formulas-function-library-date-time"
            ></mjx-button>`,
            'excel.formulas.function-library.lookup-reference': html`<mjx-button
              label="Lookup & Reference"
              icon="book-search"
              size="small"
              data-opens="shell-excel-formulas-function-library-lookup-reference"
            ></mjx-button>`,
            'excel.formulas.function-library.math-trig': html`<mjx-button
              label="Math & Trig"
              icon="book-theta"
              size="small"
              data-opens="shell-excel-formulas-function-library-math-trig"
            ></mjx-button>`,
            'excel.formulas.function-library.more-functions': html`<mjx-button
              label="More Functions"
              icon="book"
              size="large"
              data-opens="shell-excel-formulas-function-library-more-functions"
            ></mjx-button>`,
            'excel.formulas.named-cells.define-name': html`<mjx-split-button
              label="Define Name"
              icon="tag-add"
              size="small"
              data-opens="shell-excel-formulas-named-cells-define-name"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.formulas.named-cells.use-in-formula': html`<mjx-button
              label="Use in Formula"
              size="small"
              data-opens="shell-excel-formulas-named-cells-use-in-formula"
            ></mjx-button>`,
            'excel.formulas.formula-auditing.remove-arrows': html`<mjx-split-button
              label="Remove Arrows"
              size="small"
              data-opens="shell-excel-formulas-formula-auditing-remove-arrows"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.formulas.formula-auditing.error-checking': html`<mjx-split-button
              label="Error Checking"
              icon="checkmark-circle-warning"
              size="small"
              data-opens="shell-excel-formulas-formula-auditing-error-checking"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.formulas.calculation.calculation-options': html`<mjx-button
              label="Calculation Options"
              icon="calculator"
              size="small"
              data-opens="shell-excel-formulas-calculation-calculation-options"
            ></mjx-button>`,
            // Data (unit 7). Split buttons and dropdowns open their menus from
            // `stories/ribbons/mailings-animations-data-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            // Data Types is an in-ribbon gallery with nothing selected, because a new cell has no data type.
            'excel.data.get-external-data.from-other-sources': html`<mjx-button
              label="From Other Sources"
              icon="database"
              size="small"
              data-opens="shell-excel-data-get-external-data-from-other-sources"
            ></mjx-button>`,
            'excel.data.queries-connections.refresh-all': html`<mjx-split-button
              label="Refresh All"
              icon="arrow-clockwise"
              size="large"
              data-opens="shell-excel-data-queries-connections-refresh-all"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.data.data-types.data-types': html`<mjx-gallery id="xl-data-types" label="Data Types" style=${ribbonGalleryStyle}>
              ${dataTypeGalleryItems()}
            </mjx-gallery>`,
            'excel.data.data-tools.data-validation': html`<mjx-split-button
              label="Data Validation"
              icon="table-simple-checkmark"
              size="small"
              data-opens="shell-excel-data-data-tools-data-validation"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.data.forecast.what-if-analysis': html`<mjx-button
              label="What-If Analysis"
              size="small"
              data-opens="shell-excel-data-forecast-what-if-analysis"
            ></mjx-button>`,
            'excel.data.outline.group': html`<mjx-split-button
              label="Group"
              icon="group-list"
              size="large"
              data-opens="shell-excel-data-outline-group"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.data.outline.ungroup': html`<mjx-split-button
              label="Ungroup"
              size="small"
              data-opens="shell-excel-data-outline-ungroup"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            // Excel's Review. The split buttons and the two dropdowns open their menus from
            // `stories/ribbons/review-menus.ts`, and `data-opens` is `commandSurfaceId('shell', <this key>)`.
            // Hide Ink is a split button whose face is a toggle, starting unpressed.
            'excel.review.accessibility.check-accessibility': html`<mjx-split-button
              label="Check Accessibility"
              icon="accessibility-checkmark"
              size="small"
              data-opens="shell-excel-review-accessibility-check-accessibility"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.review.notes.notes': html`<mjx-button
              label="Notes"
              icon="note"
              size="large"
              data-opens="shell-excel-review-notes-notes"
            ></mjx-button>`,
            'excel.review.changes.track-changes': html`<mjx-button
              label="Track Changes"
              icon="document-edit"
              size="small"
              data-opens="shell-excel-review-changes-track-changes"
            ></mjx-button>`,
            'excel.review.ink.hide-ink': html`<mjx-split-button
              toggle
              label="Hide Ink"
              size="small"
              data-opens="shell-excel-review-ink-hide-ink"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            // Excel's View. The Sheet View dropdown is a field over `excelSheetViews`, Show's four are checkboxes,
            // all ticked as the census declares, and Freeze Panes and Switch Windows open their menus from
            // `stories/ribbons/view-menus.ts`, with `data-opens` `commandSurfaceId('shell', <this key>)`. The Workbook
            // Views exclusive set is the generic toggles.
            'excel.view.sheet-view.sheet-view': html`<mjx-dropdown
              id="xl-view-sheet-view"
              label="Sheet View"
              value="default"
              style=${ribbonNarrowFieldStyle}
            >
              ${excelSheetViews.map((view) => html`<mjx-option value=${view.value} label=${view.label}></mjx-option>`)}
            </mjx-dropdown>`,
            'excel.view.show.ruler': html`<mjx-checkbox id="xl-view-ruler" label="Ruler" checked="true"></mjx-checkbox>`,
            'excel.view.show.gridlines': html`<mjx-checkbox id="xl-view-show-gridlines" label="Gridlines" checked="true"></mjx-checkbox>`,
            'excel.view.show.formula-bar': html`<mjx-checkbox id="xl-view-formula-bar" label="Formula Bar" checked="true"></mjx-checkbox>`,
            'excel.view.show.headings': html`<mjx-checkbox id="xl-view-show-headings" label="Headings" checked="true"></mjx-checkbox>`,
            'excel.view.window.freeze-panes': html`<mjx-button
              label="Freeze Panes"
              icon="table-freeze-column-and-row"
              size="large"
              data-opens="shell-excel-view-window-freeze-panes"
            ></mjx-button>`,
            'excel.view.window.switch-windows': html`<mjx-button
              label="Switch Windows"
              icon="window-multiple"
              size="large"
              data-opens="shell-excel-view-window-switch-windows"
            ></mjx-button>`,
          },
        })}
        ${excelContextualSets({
          sets: ['table-tools'],
          controls: {
            // Table Design (a contextual tab). This workbook's selection is in a table, so the shell draws Table Tools
            // and binds the same eleven commands `Ribbons/Excel` binds, under its own ids. The field's name, the
            // gallery and both menus are `stories/ribbons/table-tools-menus.ts`'s; the gallery's pictures read this
            // workbook's palette. The other eight commands are the generic button.
            'excel.table-design.properties.table-name': html`<mjx-combo-box
              id="xl-table-design-table-name"
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
              data-opens="shell-excel-table-design-external-table-data-export"
            ></mjx-button>`,
            'excel.table-design.external-table-data.refresh': html`<mjx-split-button
              label="Refresh"
              icon="arrow-clockwise"
              size="large"
              menu-label="Refresh"
              data-opens="shell-excel-table-design-external-table-data-refresh"
              @mjx-menu-request=${openDeclaredSurface}
            ></mjx-split-button>`,
            'excel.table-design.table-style-options.header-row': html`<mjx-checkbox id="xl-table-design-header-row" label="Header Row" checked="true"></mjx-checkbox>`,
            'excel.table-design.table-style-options.total-row': html`<mjx-checkbox id="xl-table-design-total-row" label="Total Row"></mjx-checkbox>`,
            'excel.table-design.table-style-options.banded-rows': html`<mjx-checkbox id="xl-table-design-banded-rows" label="Banded Rows" checked="true"></mjx-checkbox>`,
            'excel.table-design.table-style-options.first-column': html`<mjx-checkbox id="xl-table-design-first-column" label="First Column"></mjx-checkbox>`,
            'excel.table-design.table-style-options.last-column': html`<mjx-checkbox id="xl-table-design-last-column" label="Last Column"></mjx-checkbox>`,
            'excel.table-design.table-style-options.banded-columns': html`<mjx-checkbox id="xl-table-design-banded-columns" label="Banded Columns"></mjx-checkbox>`,
            'excel.table-design.table-style-options.filter-button': html`<mjx-checkbox id="xl-table-design-filter-button" label="Filter Button" checked="true"></mjx-checkbox>`,
            'excel.table-design.table-styles.gallery': html`<mjx-gallery
              id="xl-table-styles"
              label="Table Styles"
              value="TableStyleMedium2"
              style=${ribbonGalleryStyle}
            >
              ${excelTableStyleGalleryItems(documentThemePalette)} ${excelTableStyleGalleryFooter()}
            </mjx-gallery>`,
          },
        })}
      </mjx-ribbon>
    `,
  );
}

// ── the bands ────────────────────────────────────────────────────────────────

/**
 * The name box and the formula bar — **one band, and the name box goes inside the bar.**
 *
 * ⚠ This is a *check before you consume* finding, and it was got wrong first. `<mjx-formula-bar>`
 * builds a `name-box` **slot** of its own, before its divider and its three affordances, because
 * Excel's name box has always been part of the same band. An assembly that set the two side by side
 * — which is what this shell did until it was measured — produced a bar with an empty 96 px slot in
 * it and a name box beside it, at 101 px tall instead of 58. Nothing failed; it just looked wrong in
 * a way nobody would have attributed to the markup.
 */
function formulaRow(id: string, box: string, value: string): TemplateResult {
  return surface(
    'formula-row',
    'flex:0 0 auto;min-inline-size:0;padding:var(--mjx-density-step) var(--mjx-density-gutter)',
    html`
      <mjx-formula-bar id=${id} label="Formula bar" value=${value} style="min-inline-size:0">
        <mjx-name-box
          id=${box}
          slot="name-box"
          label="Name box"
          address="B2"
          .names=${workbookNames}
        ></mjx-name-box>
      </mjx-formula-bar>
    `,
  );
}

/** The grid, its context menu, the selection a mini toolbar hangs off, and the scrollbar. */
function gridArea(): TemplateResult {
  return canvasRow(
    canvasArea(
      'xl-grid',
      html`
      <mjx-context-menu id="xl-context" style=${contextRegionStyle}>
        ${documentPlaceholder(
          'Summary — B2:D9 selected',
          html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
            ${selectionRun('xl-selection', 'B2:D9')} is what the mini toolbar above is about.
          </p>`,
        )}
        <mjx-menu slot="menu" label="Cells" floating>
          <mjx-menu-item label="Cut" icon="cut" shortcut="Ctrl+X"></mjx-menu-item>
          <mjx-menu-item label="Copy" icon="copy" shortcut="Ctrl+C"></mjx-menu-item>
          <mjx-menu-item label="Paste Options">
            <mjx-menu slot="submenu" label="Paste Options">
              <mjx-menu-item kind="radio" label="Values" checked></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Formulas"></mjx-menu-item>
              <mjx-menu-item kind="radio" label="Formatting"></mjx-menu-item>
            </mjx-menu>
          </mjx-menu-item>
          <mjx-menu-separator></mjx-menu-separator>
          <mjx-menu-section label="Cells">
            <mjx-menu-item label="Insert…" icon="add"></mjx-menu-item>
            <mjx-menu-item label="Delete…" icon="delete"></mjx-menu-item>
            <mjx-menu-item
              label="Clear Contents"
              unavailable
              explanation="The selection includes a cell in a protected range."
            ></mjx-menu-item>
          </mjx-menu-section>
        </mjx-menu>
      </mjx-context-menu>
    `,
      html`<mjx-mini-toolbar
        id="xl-mini"
        label="Formatting"
        for="xl-selection"
        open
        .commands=${cellCommands}
      ></mjx-mini-toolbar>`,
    ),
    html`<mjx-scrollbar
      id="xl-scroll"
      label="Worksheet"
      controls="xl-grid"
      pages="30"
      page-height="700"
      viewport="620"
    ></mjx-scrollbar>`,
  );
}

/** The Queries pane: a real empty state, which is what a new workbook actually shows. */
function taskPane(fraction: string): TemplateResult {
  return html`
    <mjx-task-pane
      id="xl-pane"
      label="Queries &amp; Connections"
      open
      dock="inlineEnd"
      fraction=${fraction}
    >
      ${paneStack(
        html`<mjx-empty-state
          id="xl-queries-empty"
          heading="No queries yet"
          description="Queries you add from the Data tab appear here, with when each one last refreshed."
          icon="table"
          action-label="Get data"
          action-command="data.getData"
        ></mjx-empty-state>`,
        paneHeading('Defined names'),
        html`<mjx-virtual-list
          id="xl-names"
          label="Defined names in this workbook"
          value="Revenue"
          style="block-size:11rem;border:1px solid var(--theme-border-subtle);
                 border-radius:var(--radius-control)"
          .items=${definedNameRows}
        ></mjx-virtual-list>`,
        field(
          'xl-width',
          'Row height',
          html`<mjx-measure-input
            id="xl-width"
            value="15"
            unit="pt"
            step="0.75"
            min="0"
          ></mjx-measure-input>`,
        ),
        html`<mjx-checkbox id="xl-gridlines" label="Show gridlines" checked="true"></mjx-checkbox>`,
      )}
    </mjx-task-pane>
  `;
}

/** The sheet tab bar, with ten sheets, two colours and one hidden. */
function sheetTabs(): TemplateResult {
  return surface(
    'sheet-tabs',
    'flex:0 0 auto;min-inline-size:0;padding:0 var(--mjx-density-gutter)',
    html`<mjx-sheet-tab-bar
      id="xl-sheets"
      label="Sheets"
      value="summary"
      .tabs=${workbookTabs()}
    ></mjx-sheet-tab-bar>`,
  );
}

/** The bar across the foot, and the readings a workbook actually carries. */
function foot(): TemplateResult {
  return statusBar(
    'xl-status',
    'Workbook status',
    [
      { id: 'xl-mode', label: 'Mode', value: 'Ready', priority: 'essential' },
      { id: 'xl-average', label: 'Average', value: '18,204', priority: 'standard' },
      { id: 'xl-count', label: 'Count', value: '24', priority: 'standard' },
      { id: 'xl-sum', label: 'Sum', value: '436,896', priority: 'supplementary' },
      {
        id: 'xl-calc',
        label: 'Calculation',
        value: 'Automatic',
        priority: 'ancillary',
        region: 'centre',
      },
    ],
    zoom('xl-zoom', { width: 1100, height: 850 }),
  );
}

// ── the desktop and tablet shells ────────────────────────────────────────────

/**
 * Everything above the phone: five stacked bands and a pane.
 *
 * The pane's share grows at tablet for the reason PowerPoint's does — a docked `<mjx-task-pane>`
 * will not lay out below about 280 px, so a fraction sized for a desktop overflows an 834 px
 * workspace by 96. See `powerpoint.stories.ts` for the decision and `ui/README.md` for the question
 * it leaves open.
 */
function wideShell(size: 'desktop' | 'tablet'): TemplateResult {
  const paneFraction = size === 'desktop' ? '0.22' : '0.4';
  return shellFrame(
    'excel',
    size,
    ribbon(),
    formulaRow('xl-formula', 'xl-name-box', multiReferenceFormula),
    surface(
      'workspace',
      workspaceStyle,
      html`${documentColumn(gridArea())} ${taskPane(paneFraction)}`,
    ),
    sheetTabs(),
    foot(),
    html`
      <mjx-menu id="xl-paste-menu" label="Paste options" floating>
        <mjx-menu-section label="Paste">
          <mjx-menu-item kind="radio" label="All" checked></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Values"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Formats"></mjx-menu-item>
          <mjx-menu-item kind="radio" label="Transpose"></mjx-menu-item>
        </mjx-menu-section>
        <mjx-menu-separator></mjx-menu-separator>
        <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
      </mjx-menu>
      ${insertMenus('excel', 'shell')} ${drawMenus('excel', 'shell')}
      ${designLayoutMenus('excel', 'shell')} ${referencesTransitionsFormulasMenus('excel', 'shell')}
      ${mailingsAnimationsDataMenus('excel', 'shell')} ${reviewMenus('excel', 'shell')}
      ${viewMenus('excel', 'shell')} ${tableToolsMenus('excel', 'shell')}
      <mjx-dialog id="xl-format-cells" label="Format Cells" modal>
        ${paneStack(
          // A measure input is for a *measure*, so the field here is an indent rather than a count
          // of decimal places: `value` is in points and a spin box over 0…30 places would be a
          // different component. Naming that is cheaper than misusing this one.
          field(
            'xl-indent',
            'Indent',
            html`<mjx-measure-input
              id="xl-indent"
              value="0"
              unit="pt"
              step="3"
              min="0"
              max="216"
            ></mjx-measure-input>`,
          ),
          html`<mjx-checkbox
            id="xl-thousands"
            label="Use 1000 separator"
            checked="true"
          ></mjx-checkbox>`,
          html`<mjx-popover id="xl-negatives" label="Negative numbers" kind="flyout">
            <mjx-button slot="anchor" label="Negative numbers…" icon="settings"></mjx-button>
            <mjx-checkbox id="xl-negative-red" label="Show in red"></mjx-checkbox>
            <mjx-checkbox id="xl-negative-brackets" label="Show in brackets"></mjx-checkbox>
          </mjx-popover>`,
        )}
      </mjx-dialog>
    `,
  );
}

// ── the stories ──────────────────────────────────────────────────────────────

/** The whole application at 1440: five bands and a pane around one grid. */
export const Desktop: Story = {
  globals: { containerPreset: 'desktop' },
  render: () => wideShell('desktop'),
};

/**
 * **The middle size.** The sheet tab strip overflows at 834 and grows its four scroll affordances
 * while the status bar is demoting readings — two components deciding to change shape at once, which
 * is a thing only an assembly can show.
 */
export const Tablet: Story = {
  globals: { containerPreset: 'tablet' },
  render: () => wideShell('tablet'),
};

/**
 * **The phone**, and Excel is the one application whose phone shell keeps a desktop band: the
 * formula bar. A spreadsheet without one is not a spreadsheet, so the rail below and the bar above
 * have to share a screen 390 pixels wide.
 */
export const Mobile: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    shellFrame(
      'excel',
      'mobile',
      formulaRow('xl-phone-formula', 'xl-phone-name-box', '=SUM(B2:B9)'),
      phoneBody(canvasArea('xl-phone-grid', documentPlaceholder('Summary — B2:D9 selected'))),
      surface(
        'sheet-tabs',
        'flex:0 0 auto;min-inline-size:0;padding:0 var(--mjx-density-step)',
        html`<mjx-sheet-tab-bar
          id="xl-phone-sheets"
          label="Sheets"
          value="summary"
          .tabs=${workbookTabs()}
        ></mjx-sheet-tab-bar>`,
      ),
      phoneRails(
        html`<mjx-contextual-action-bar
          id="xl-phone-selection"
          selection="cells"
          selection-label="B2:D9"
        ></mjx-contextual-action-bar>`,
        html`<mjx-command-bar
          id="xl-phone-commands"
          label="Home"
          .commands=${wordPhoneCommands}
          @mjx-mobile-command=${openSheetOnCommand('styles', 'xl-phone-sheet')}
        ></mjx-command-bar>`,
      ),
      html`<mjx-dialog id="xl-phone-sheet" label="Cell styles" modal detent="half">
        <mjx-gallery id="xl-phone-styles" label="Cell styles" value="office-2">
          ${largeGalleryItems().slice(0, 12)}
        </mjx-gallery>
      </mjx-dialog>`,
    ),
};
