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
  openDeclaredSurface,
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
import { drawMenus } from './draw-menus.ts';
import { excelContextualSets, excelTabs } from './excel.ts';
import { insertMenus } from './insert-menus.ts';

/**
 * **Excel's ribbon, tab by tab** — the same functions `Shell/Excel` composes.
 *
 * ⚠ **Excel's File tab has no Print group**, and that is the census rather than an omission: the
 * committed command surface carries a backstage `TabPrint` row for Word and PowerPoint and none for
 * Excel, and carries a `Publish2Tab` the other two lack. Excel obviously has a File → Print page,
 * so this is a gap in the dump — but the census is the checked source and inventing the row would
 * be the drift the transcription exists to prevent. `dev/ribbons/census.ts` is where it is recorded.
 *
 * **File, Home, Insert and Draw** are authored; the rest are placeholders at the census's own priorities. See
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
          'File, Home, Insert and Draw are authored; the rest are placeholders carrying the census’s ' +
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

/** Unit 5. */
export const PageLayout: Story = { render: () => ribbon('page-layout') };

/** Unit 6. */
export const Formulas: Story = { render: () => ribbon('formulas') };

/** Unit 7. */
export const Data: Story = { render: () => ribbon('data') };

/** Unit 8. */
export const Review: Story = { render: () => ribbon('review') };

/** Unit 9. */
export const View: Story = { render: () => ribbon('view') };

/** Unit 10, and a view tab — see `dev/ribbons/census.ts`. */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/** A view tab: Office shows it only while a picture's background is being removed. */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };
