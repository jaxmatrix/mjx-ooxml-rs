/**
 * **The menus and the gallery the Chart Design tab opens, and the lists Chart Format adds to Shape Format's**, written
 * once for all three applications: Word's, PowerPoint's and Excel's Chart Design and Chart Format all call them. Change
 * Chart Type reads Excel's Insert → Charts
 * lists (`stories/ribbons/insert-menus.ts`' `columnBarChartEntries()` and the seven beside it), so a chart family Office
 * offers on two tabs is written once.
 *
 * The pattern is `stories/ribbons/drawing-tools-menus.ts`', for its reasons. A binding lives in its host. The menu it
 * opens is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `chartToolsMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below spells its
 * command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## Shaped for three Chart Design tabs
 *
 * Office's Chart Design is nearly the same tab in all three applications, so **every list here is a function of
 * nothing, and only the menus function is per application**:
 *
 * - **Chart Layouts**: `addChartElementEntries()`, Add Chart Element's eleven submenus with Office's entries under each
 *   (`chartElements`, the data); `quickLayoutEntries()`, Layout 1 to Layout 11.
 * - **Chart Styles**: `changeColoursEntries()`, the Colourful and Monochromatic palettes; `chartStyles`, Style 1 to
 *   Style 16, and `chartStyleGalleryItems(palette)`, their pictures in the document's colours.
 * - **Data**: `editDataEntries()`, Edit Data's arrow. Switch Row/Column, Select Data and Refresh Data open nothing a
 *   menu can hold (a swap, a dialog, a refresh), so they have no list. Excel's Data group has neither Edit Data nor
 *   Refresh Data (a workbook's chart reads its own cells), and calls nothing here. Excel's Location group, Move Chart,
 *   opens a dialog and has no list either.
 * - **Type**: `changeChartTypeEntries()`, the eight chart families as submenus, each Insert's own list.
 *
 * ## Shaped for three Chart Format tabs
 *
 * **Chart Format is Shape Format's tab with a chart's Current Selection in front**, so it reads
 * `stories/ribbons/drawing-tools-menus.ts`' lists wherever a chart behaves as a shape (the shape styles and Other Theme
 * Fills, Shape Effects, Shape Fill's entries), `wordart-styles-menus.ts`' gallery and Text Effects, and
 * `design-layout-menus.ts`' Arrange lists. **Only what a chart changes is written here**, each a function of nothing or
 * of the application:
 *
 * - **Current Selection**: `chartSelectionElements`, the parts of the chart Office inserts, and
 *   `chartSelectionOptions()`, the Chart Elements field's options.
 * - **Insert Shapes**: `chartShapeGallerySections(application, purpose)`, the shape gallery less what a chart cannot
 *   hold, over it `chartShapesEntries(application)` and `chartChangeShapeEntries(application)`.
 * - **Shape Styles**: `chartOutlineEntryOptions(application)`, Shape Outline's entries less Sketched and Arrows.
 * - **WordArt Styles**: `chartTextFillEntryOptions(application)` and `chartTextOutlineEntryOptions(application)`, each
 *   application's own Shape Format text entries.
 * - **Size**: `wordChartMeasures` and `powerpointChartMeasures`, the chart each application inserts.
 *
 * `GUESS:` every label, order, check and preset below, from memory of Microsoft 365. Where a label differs from Office's
 * US spelling the census's wins (*Colours*, *Colourful*, *Centred*), as it does across the catalogue.
 *
 * ## The pictures are the document's colours, not the chrome's
 *
 * A chart style is a set of fills, outlines, gridlines and a ground drawn in the document's theme, so each thumbnail is
 * drawn in the document's `ThemeColorPalette`, through `stories/ribbons/palette-art.ts`, as static markup, for the
 * reason every gallery picture is: `<mjx-gallery-item>` clones its children into the gallery's shadow root.
 *
 * **Nothing here dispatches a command.** A menu opens, an entry can be chosen, a gallery previews and commits; no chart
 * changes, because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';
import { html as staticHtml, unsafeStatic } from 'lit/static-html.js';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import type { ThemeColorPalette, ThemeColorSlot } from '../../src/pickers/picker-model.ts';
import type { FillEntryOptions, OutlineEntryOptions } from './colour-picker-entries.ts';
import {
  alignEntries,
  bringForwardEntries,
  groupEntries,
  positionEntries,
  rotateEntries,
  sendBackwardEntries,
  wrapTextEntries,
} from './design-layout-menus.ts';
import {
  otherThemeFillEntries,
  shapeEffectsEntries,
  shapeGallerySections,
  shapeOutlineEntryOptions,
  shapeSectionEntries,
  type ShapeGallerySection,
} from './drawing-tools-menus.ts';
import {
  columnBarChartEntries,
  comboChartEntries,
  hierarchyChartEntries,
  lineAreaChartEntries,
  pieDoughnutChartEntries,
  scatterBubbleChartEntries,
  statisticChartEntries,
  waterfallChartEntries,
} from './insert-menus.ts';
import { paletteSlotColour, spacingStep } from './palette-art.ts';
import { commandMenu } from './ribbon-parts.ts';
import { submenu, wordArtTextEffectsEntries } from './wordart-styles-menus.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string): TemplateResult {
  return html`<mjx-menu-item label=${label}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

/** One of a set, radio-checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** One element that is on or off independently of its neighbours. */
function check(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="checkbox" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** `Style 16` → `style-16`. */
function slug(label: string): string {
  return label.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

// ── Chart Layouts: Add Chart Element ─────────────────────────────────────────

/**
 * How one of Add Chart Element's submenus behaves: **`one`**, a set of which the chart holds exactly one (a title is
 * above the chart, overlaid, or absent), drawn as radio entries; **`each`**, elements the chart carries independently
 * (either axis, any gridline), drawn as checkboxes; **`none`**, verbs with no state, drawn as plain entries.
 */
export type ChartElementKind = 'one' | 'each' | 'none';

/** One Add Chart Element submenu: its label, its entries in Office's order, which start on, and its options entry. */
export interface ChartElementSpec {
  readonly label: string;
  readonly kind: ChartElementKind;
  readonly entries: readonly string[];
  /** The entries a chart Word has just inserted carries. */
  readonly on: readonly string[];
  /** The last entry, below a separator, which opens the Format pane at that element. */
  readonly options: string;
}

/**
 * **Add Chart Element's eleven submenus**, in Office's order, as Word's Chart Design shows them for the chart Word
 * inserts, a Clustered Column, and PowerPoint's for the same Clustered Column PowerPoint inserts: every entry and the
 * options entry under a separator, and PowerPoint's and Excel's for the same Clustered Column each inserts. `GUESS:`
 * that PowerPoint's and Excel's inserted charts start exactly as Word's does.
 *
 * - **Axes** and **Axis Titles**: Primary Horizontal and Primary Vertical, each independent. A new chart has both axes
 *   and no axis title.
 * - **Chart Title**: None, Above Chart, Centred Overlay. A new chart's title is Above Chart.
 * - **Data Labels**: None, Centre, Inside End, Inside Base, Outside End, Data Callout, the positions a column takes. A
 *   new chart has none.
 * - **Data Table**: None, With Legend Keys, No Legend Keys. None.
 * - **Error Bars**: None, Standard Error, Percentage, Standard Deviation. None.
 * - **Gridlines**: Primary Major Horizontal, Primary Major Vertical, Primary Minor Horizontal, Primary Minor Vertical,
 *   each independent. A new chart has Primary Major Horizontal.
 * - **Legend**: None, Right, Top, Left, Bottom. A new chart's legend is at the Bottom.
 * - **Lines**: None, Drop Lines, High-Low Lines. None.
 * - **Trendline**: None, Linear, Exponential, Linear Forecast, Moving Average. None; a verb per series, so plain.
 * - **Up/Down Bars**: None, Up/Down Bars. None.
 *
 * ⚠ **Lines and Up/Down Bars are drawn available**, where Office greys both on a column chart (they belong to line and
 * stock charts): a greyed submenu cannot be opened, and a list nobody can open cannot be audited. `GUESS:` every label,
 * every order, every start, each options entry's name, and which submenus are radio sets.
 */
export const chartElements: readonly ChartElementSpec[] = [
  {
    label: 'Axes',
    kind: 'each',
    entries: ['Primary Horizontal', 'Primary Vertical'],
    on: ['Primary Horizontal', 'Primary Vertical'],
    options: 'More Axis Options…',
  },
  {
    label: 'Axis Titles',
    kind: 'each',
    entries: ['Primary Horizontal', 'Primary Vertical'],
    on: [],
    options: 'More Axis Title Options…',
  },
  {
    label: 'Chart Title',
    kind: 'one',
    entries: ['None', 'Above Chart', 'Centred Overlay'],
    on: ['Above Chart'],
    options: 'More Title Options…',
  },
  {
    label: 'Data Labels',
    kind: 'one',
    entries: ['None', 'Centre', 'Inside End', 'Inside Base', 'Outside End', 'Data Callout'],
    on: ['None'],
    options: 'More Data Label Options…',
  },
  {
    label: 'Data Table',
    kind: 'one',
    entries: ['None', 'With Legend Keys', 'No Legend Keys'],
    on: ['None'],
    options: 'More Data Table Options…',
  },
  {
    label: 'Error Bars',
    kind: 'one',
    entries: ['None', 'Standard Error', 'Percentage', 'Standard Deviation'],
    on: ['None'],
    options: 'More Error Bars Options…',
  },
  {
    label: 'Gridlines',
    kind: 'each',
    entries: ['Primary Major Horizontal', 'Primary Major Vertical', 'Primary Minor Horizontal', 'Primary Minor Vertical'],
    on: ['Primary Major Horizontal'],
    options: 'More Gridline Options…',
  },
  {
    label: 'Legend',
    kind: 'one',
    entries: ['None', 'Right', 'Top', 'Left', 'Bottom'],
    on: ['Bottom'],
    options: 'More Legend Options…',
  },
  {
    label: 'Lines',
    kind: 'one',
    entries: ['None', 'Drop Lines', 'High-Low Lines'],
    on: ['None'],
    options: 'More Lines Options…',
  },
  {
    label: 'Trendline',
    kind: 'none',
    entries: ['None', 'Linear', 'Exponential', 'Linear Forecast', 'Moving Average'],
    on: [],
    options: 'More Trendline Options…',
  },
  {
    label: 'Up/Down Bars',
    kind: 'one',
    entries: ['None', 'Up/Down Bars'],
    on: ['None'],
    options: 'More Up/Down Bars Options…',
  },
];

/** One Add Chart Element submenu's entries, drawn by its kind. */
function chartElementEntries(element: ChartElementSpec): TemplateResult[] {
  const draw = (label: string): TemplateResult => {
    const on = element.on.includes(label);
    if (element.kind === 'one') return choice(label, on);
    if (element.kind === 'each') return check(label, on);
    return item(label);
  };
  return [...element.entries.map(draw), separator(), item(element.options)];
}

/** **Add Chart Element's menu**: every submenu `chartElements` declares, in order. */
export function addChartElementEntries(): TemplateResult[] {
  return chartElements.map((element) => submenu(element.label, ...chartElementEntries(element)));
}

// ── Chart Layouts: Quick Layout ──────────────────────────────────────────────

/**
 * **Quick Layout's gallery, as a menu**: *Layout 1* to *Layout 11*, the eleven arrangements of title, legend, axis
 * titles, gridlines and data table Office offers a column chart. Office draws each as a thumbnail; a menu of names is the
 * catalogue's shape for a gallery of pictures that opens from a button, as Picture Format's Corrections is. Nothing is
 * checked: a layout is applied, not worn. `GUESS:` that a column chart is offered eleven.
 */
export function quickLayoutEntries(): TemplateResult[] {
  return Array.from({ length: 11 }, (_, index) => item(`Layout ${String(index + 1)}`));
}

// ── Chart Styles: Change Colours ─────────────────────────────────────────────

/** **The Colourful palettes**, each series in a different theme accent, in Office's order. */
export const colourfulPalettes: readonly string[] = Array.from(
  { length: 4 },
  (_, index) => `Colourful Palette ${String(index + 1)}`,
);

/** **The Monochromatic palettes**, every series in shades of one colour, in Office's order. */
export const monochromaticPalettes: readonly string[] = Array.from(
  { length: 13 },
  (_, index) => `Monochromatic Palette ${String(index + 1)}`,
);

/**
 * **Change Colours' menu**: *Colourful*, four palettes, then *Monochromatic*, thirteen, one set across both, with
 * *Colourful Palette 1* checked, the palette a new chart wears (its series in Accent 1, 2, 3…). Office draws each palette
 * as a strip of swatches beside its name; a menu holds the names. `GUESS:` the counts (4 and 13), every name, and the
 * start.
 */
export function changeColoursEntries(): TemplateResult[] {
  return [
    section('Colourful', ...colourfulPalettes.map((palette, index) => choice(palette, index === 0))),
    section('Monochromatic', ...monochromaticPalettes.map((palette) => choice(palette))),
  ];
}

// ── Chart Styles: the gallery ────────────────────────────────────────────────

/** What a chart style's plot stands on: the page, a tint of Accent 1, or the document's dark text colour. */
export type ChartStyleGround = 'paper' | 'tint' | 'dark';

/** How a chart style fills its columns. */
export type ChartStyleBars = 'solid' | 'outline' | 'pattern' | 'gradient' | 'pale';

/** One chart style: its value, Office's name, and the look its picture describes. */
export interface ChartStyleSpec {
  readonly value: string;
  readonly label: string;
  readonly ground: ChartStyleGround;
  readonly bars: ChartStyleBars;
  readonly gridlines: boolean;
  /** Narrow columns stand far apart; wide ones nearly touch. */
  readonly gap: 'wide' | 'narrow';
}

/** One style, its value derived from its name. */
function chartStyle(
  index: number,
  ground: ChartStyleGround,
  bars: ChartStyleBars,
  gridlines: boolean,
  gap: ChartStyleSpec['gap'],
): ChartStyleSpec {
  const label = `Style ${String(index)}`;
  return { value: slug(label), label, ground, bars, gridlines, gap };
}

/**
 * **The Chart Styles gallery's sixteen styles**, *Style 1* to *Style 16*, as a column chart is offered them. Style 1 is
 * the look a new chart wears. `GUESS:` that there are sixteen, and every look: Office's thumbnails are renders of each
 * style's `c:chartStyle` part in the theme, and these are a description of a look (a ground, a fill, gridlines or not,
 * the gap between columns) rather than a render of one.
 */
export const chartStyles: readonly ChartStyleSpec[] = [
  chartStyle(1, 'paper', 'solid', true, 'wide'),
  chartStyle(2, 'paper', 'solid', false, 'narrow'),
  chartStyle(3, 'paper', 'pattern', true, 'wide'),
  chartStyle(4, 'paper', 'solid', true, 'narrow'),
  chartStyle(5, 'tint', 'solid', true, 'wide'),
  chartStyle(6, 'paper', 'outline', true, 'wide'),
  chartStyle(7, 'paper', 'pale', true, 'wide'),
  chartStyle(8, 'dark', 'solid', false, 'wide'),
  chartStyle(9, 'paper', 'gradient', true, 'wide'),
  chartStyle(10, 'paper', 'solid', false, 'wide'),
  chartStyle(11, 'tint', 'outline', true, 'narrow'),
  chartStyle(12, 'dark', 'gradient', true, 'narrow'),
  chartStyle(13, 'paper', 'pattern', false, 'narrow'),
  chartStyle(14, 'dark', 'pale', false, 'wide'),
  chartStyle(15, 'tint', 'gradient', false, 'wide'),
  chartStyle(16, 'dark', 'outline', true, 'wide'),
];

/** The three series a thumbnail draws, in Colourful Palette 1's order, and each column's height as a share of the plot. */
const chartStyleSeries: readonly { readonly slot: ThemeColorSlot; readonly height: number }[] = [
  { slot: 'accent1', height: 45 },
  { slot: 'accent2', height: 80 },
  { slot: 'accent3', height: 60 },
];

/**
 * **One chart style's thumbnail**: three columns in Accent 1, 2 and 3 on a plot, with or without horizontal gridlines,
 * in the document's colours, as static markup.
 *
 * - **Grounds**: Background 1; Background 1 tinted with Accent 1; Text 1, on which the gridlines turn light.
 * - **Columns**: solid; an outline of the colour around a faint fill; a diagonal hatch of the colour; the colour darkening
 *   towards the base; a pale tint of the colour.
 *
 * ⚠ **Static for the reason every gallery picture is**, and **every colour is a checked hex colour or a token**,
 * through `paletteSlotColour`. The thumbnail's height matches the shape, WordArt, table and picture style pictures'.
 * `GUESS:` every look; see `chartStyles`.
 */
export function chartStylePicture(style: ChartStyleSpec, palette: ThemeColorPalette): TemplateResult {
  const step = spacingStep;
  const paper = paletteSlotColour(palette, 'background1');
  const ink = paletteSlotColour(palette, 'text1');
  const grounds: Record<ChartStyleGround, string> = {
    paper,
    tint: `color-mix(in srgb, ${paletteSlotColour(palette, 'accent1')} 12%, ${paper})`,
    dark: ink,
  };
  const ground = grounds[style.ground];
  const rule = style.ground === 'dark' ? `color-mix(in srgb, ${paper} 30%, transparent)` : `color-mix(in srgb, ${ink} 18%, transparent)`;
  const gridlines = style.gridlines
    ? `background-image:repeating-linear-gradient(to top, ${rule} 0 1px, transparent 1px ${step(1)});`
    : '';
  const column = (slot: ThemeColorSlot, height: number): string => {
    const colour = paletteSlotColour(palette, slot);
    const looks: Record<ChartStyleBars, string> = {
      solid: `background:${colour}`,
      outline: `background:color-mix(in srgb, ${colour} 25%, ${ground});border:${step(0.125)} solid ${colour};box-sizing:border-box`,
      pattern:
        `background:repeating-linear-gradient(45deg, ${colour} 0 ${step(0.25)}, ` +
        `color-mix(in srgb, ${colour} 35%, ${ground}) ${step(0.25)} ${step(0.5)})`,
      gradient: `background:linear-gradient(to bottom, ${colour}, color-mix(in srgb, ${colour} 60%, ${ink}))`,
      pale: `background:color-mix(in srgb, ${colour} 55%, ${ground})`,
    };
    return `<span style="flex:1;block-size:${String(height)}%;${looks[style.bars]}"></span>`;
  };
  const gap = style.gap === 'wide' ? step(0.25) : step(1);
  const markup =
    `<span style="display:grid;place-items:center;inline-size:100%;block-size:${step(6.25)};background:${ground};overflow:hidden">` +
    `<span style="display:flex;align-items:flex-end;gap:${gap};box-sizing:border-box;inline-size:70%;block-size:${step(4)};` +
    `padding-inline:${step(0.5)};border-block-end:1px solid ${rule};${gridlines}">` +
    chartStyleSeries.map((series) => column(series.slot, series.height)).join('') +
    `</span></span>`;
  return staticHtml`${unsafeStatic(markup)}`;
}

/**
 * **The Chart Styles gallery's items**, one `<mjx-gallery-item>` per style, drawn in the document's palette. A host
 * starts the gallery on `style-1`, the style a new chart wears.
 */
export function chartStyleGalleryItems(palette: ThemeColorPalette): TemplateResult[] {
  return chartStyles.map(
    (style) => html`<mjx-gallery-item value=${style.value} label=${style.label}
      >${chartStylePicture(style, palette)}</mjx-gallery-item
    >`,
  );
}

// ── Data: Edit Data ──────────────────────────────────────────────────────────

/**
 * **Edit Data's arrow**: *Edit Data*, the small data sheet Word opens over the document and PowerPoint over the slide,
 * and *Edit Data in Excel*, the whole workbook in Excel. Nothing is checked: each opens a window. PowerPoint's is the
 * same two, and calls this function; Excel has no Edit Data, and does not.
 * `GUESS:` both labels.
 */
export function editDataEntries(): TemplateResult[] {
  return [item('Edit Data'), item('Edit Data in Excel')];
}

// ── Type: Change Chart Type ──────────────────────────────────────────────────

/**
 * **Change Chart Type's menu**: the eight chart families of Excel's Insert → Charts group, each a submenu holding
 * exactly the list Insert's own command opens, in that group's order, under Insert's names without *Insert*. Each
 * family ends on its *More … Charts…* entry, which opens the Change Chart Type dialog at that family.
 *
 * ⚠ **Office's Change Chart Type opens a dialog directly**; the catalogue opens a menu of the dialog's families, because
 * a dialog is not something this catalogue draws and the families are the choice the dialog offers. **No Map family**:
 * all three applications' dialogs list Map, which Excel reaches from Insert's Maps, not from these eight. `GUESS:` both.
 */
export function changeChartTypeEntries(): TemplateResult[] {
  return [
    submenu('Column or Bar Chart', ...columnBarChartEntries()),
    submenu('Hierarchy Chart', ...hierarchyChartEntries()),
    submenu('Waterfall, Funnel, Stock, Surface or Radar Chart', ...waterfallChartEntries()),
    submenu('Line or Area Chart', ...lineAreaChartEntries()),
    submenu('Statistic Chart', ...statisticChartEntries()),
    submenu('Combo Chart', ...comboChartEntries()),
    submenu('Pie or Doughnut Chart', ...pieDoughnutChartEntries()),
    submenu('Scatter (X, Y) or Bubble Chart', ...scatterBubbleChartEntries()),
  ];
}

// ── Format: Current Selection ────────────────────────────────────────────────

/** One part of a chart the Chart Elements field can select: its value, and Office's name for it. */
export interface ChartSelectionElement {
  readonly value: string;
  readonly label: string;
}

/**
 * **The Chart Elements field's entries**, for the Clustered Column all three applications insert: every part that chart
 * carries (`chartElements`' starts: a title, a legend, both axes and the major horizontal gridlines, which Office names
 * after the vertical axis they mark), the chart and plot areas, and each of its three series, **in Office's alphabetical
 * order**, which puts the series before the vertical axis. A host starts the field on `chart-area`, what a click on the
 * chart's edge selects. No individual data point is listed: Office adds one only once a point is selected.
 * `GUESS:` the order, every label and the three series.
 */
export const chartSelectionElements: readonly ChartSelectionElement[] = [
  { value: 'chart-area', label: 'Chart Area' },
  { value: 'chart-title', label: 'Chart Title' },
  { value: 'horizontal-category-axis', label: 'Horizontal (Category) Axis' },
  { value: 'legend', label: 'Legend' },
  { value: 'plot-area', label: 'Plot Area' },
  { value: 'series-1', label: 'Series "Series 1"' },
  { value: 'series-2', label: 'Series "Series 2"' },
  { value: 'series-3', label: 'Series "Series 3"' },
  { value: 'vertical-value-axis', label: 'Vertical (Value) Axis' },
  { value: 'vertical-value-axis-major-gridlines', label: 'Vertical (Value) Axis Major Gridlines' },
];

/** **The Chart Elements field's options**, one `<mjx-option>` per `chartSelectionElements` entry, which a host slots. */
export function chartSelectionOptions(): TemplateResult[] {
  return chartSelectionElements.map(
    (element) => html`<mjx-option value=${element.value} label=${element.label}></mjx-option>`,
  );
}

// ── Format: Insert Shapes ────────────────────────────────────────────────────

/**
 * **The shape gallery a chart offers**: `shapeGallerySections(application, purpose)` less **Action Buttons**, which run
 * a slide show a chart does not. A chart's Shapes also has no **New Drawing Canvas** under the gallery, because a canvas
 * cannot sit inside a chart; that entry is not a section, so it is left out by `chartShapesEntries` not calling
 * `insertShapesEntries`. `GUESS:` both omissions.
 */
export function chartShapeGallerySections(
  application: RibbonApplication,
  purpose: 'insert' | 'change',
): readonly ShapeGallerySection[] {
  return shapeGallerySections(application, purpose).filter((group) => group.section !== 'Action Buttons');
}

/** **A chart's Shapes, as a menu**: every section `chartShapeGallerySections(application, 'insert')` gives. */
export function chartShapesEntries(application: RibbonApplication): TemplateResult[] {
  return shapeSectionEntries(chartShapeGallerySections(application, 'insert'));
}

/**
 * **A chart's Change Shape**: every section `chartShapeGallerySections(application, 'change')` gives, which changes a
 * shape already drawn inside the chart. Office greys the command until one is selected; a host draws it available, so
 * the list can be judged.
 */
export function chartChangeShapeEntries(application: RibbonApplication): TemplateResult[] {
  return shapeSectionEntries(chartShapeGallerySections(application, 'change'));
}

// ── Format: Shape Styles and WordArt Styles ──────────────────────────────────

/**
 * **What sits beneath a chart part's Shape Outline**: Shape Format's (`shapeOutlineEntryOptions`) **less Sketched and
 * Arrows**. A chart element's border is not a hand-drawn line and has no ends, and the census counts Shape Styles two
 * fewer on every Chart Format than on the same application's Shape Format. Shape Fill's entries are Shape Format's
 * unchanged, so a host calls `shapeFillEntryOptions`. `GUESS:` both omissions.
 */
export function chartOutlineEntryOptions(application: RibbonApplication): OutlineEntryOptions {
  return { ...shapeOutlineEntryOptions(application), sketched: false, arrows: false };
}

/**
 * **What sits beneath a chart's Text Fill**: the application's own Shape Format Text Fill entries, because the census
 * counts WordArt Styles the same on the two tabs in each application (Word and Excel 30, PowerPoint 33). **Word's**:
 * More Fill Colours… and Gradient ▸. **PowerPoint's**: More Fill Colours…, Eyedropper, Picture…, Gradient ▸, Texture ▸.
 * **Excel's**: PowerPoint's less the Eyedropper. `GUESS:` the reading, and every entry.
 */
export function chartTextFillEntryOptions(application: RibbonApplication): FillEntryOptions {
  if (application === 'word') return { moreColours: 'More Fill Colours…', gradient: true };
  return {
    moreColours: 'More Fill Colours…',
    eyedropper: application === 'powerpoint',
    picture: true,
    gradient: true,
    texture: true,
  };
}

/**
 * **What sits beneath a chart's Text Outline**, by `chartTextFillEntryOptions`' reading. **Word's**: More Outline
 * Colours…, Weight ▸ and Dashes ▸. **PowerPoint's**: More Outline Colours…, Eyedropper, Weight ▸, Sketched ▸, Dashes ▸.
 * **Excel's**: PowerPoint's less the Eyedropper. `GUESS:` the reading, and every entry.
 */
export function chartTextOutlineEntryOptions(application: RibbonApplication): OutlineEntryOptions {
  if (application === 'word') return { moreColours: 'More Outline Colours…', weight: true, dashes: true };
  return {
    moreColours: 'More Outline Colours…',
    eyedropper: application === 'powerpoint',
    weight: true,
    sketched: true,
    dashes: true,
  };
}

// ── Format: Size ─────────────────────────────────────────────────────────────

/**
 * **The height and width a Word host starts a chart's Size fields on**, in centimetres: the chart Word inserts, 3.5
 * inches high and 6 wide (`wp:extent` 3200400 × 5486400 EMU), stepping by 0.01 cm. `GUESS:` both numbers and the step.
 */
export const wordChartMeasures = { height: '8.89', width: '15.24', step: '0.01' } as const;

/**
 * **The height and width a PowerPoint host starts a chart's Size fields on**, in centimetres: the chart PowerPoint
 * inserts on a slide with no content placeholder, `p:graphicFrame`'s `a:ext` 8128000 × 5418667 EMU, 22.58 cm wide and
 * 15.05 high, stepping by 0.01 cm. A chart inserted into a content placeholder takes the placeholder's size instead.
 * `GUESS:` both numbers, the step, and which of the two to start on.
 */
export const powerpointChartMeasures = { height: '15.05', width: '22.58', step: '0.01' } as const;

// ── what a host renders ──────────────────────────────────────────────────────

/**
 * Word's twelve Chart Format menus. **Insert Shapes' two**: Shapes and Change Shape, over a chart's gallery. **Shape
 * Styles' two**: Other Theme Fills, which the Theme Styles gallery's footer opens and whose id is therefore the
 * gallery's, and Shape Effects. **WordArt Styles' one**: Text Effects. **Arrange's seven**, over
 * `stories/ribbons/design-layout-menus.ts`' Word lists, as Word's Shape Format. The Chart Elements field is a field, and
 * every other command a picker, a gallery, a toggle or a plain button.
 */
function wordChartFormatMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.chart-format.insert-shapes.shapes', 'Shapes', ...chartShapesEntries('word'))}
    ${commandMenu(host, 'word.chart-format.insert-shapes.change-shape', 'Change Shape', ...chartChangeShapeEntries('word'))}
    ${commandMenu(host, 'word.chart-format.shape-styles.theme-styles', 'Other Theme Fills', ...otherThemeFillEntries())}
    ${commandMenu(host, 'word.chart-format.shape-styles.shape-effects', 'Shape Effects', ...shapeEffectsEntries())}
    ${commandMenu(host, 'word.chart-format.wordart-styles.text-effects', 'Text Effects', ...wordArtTextEffectsEntries())}
    ${commandMenu(host, 'word.chart-format.arrange.position', 'Position', ...positionEntries())}
    ${commandMenu(host, 'word.chart-format.arrange.wrap-text', 'Wrap Text', ...wrapTextEntries())}
    ${commandMenu(host, 'word.chart-format.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('word'))}
    ${commandMenu(host, 'word.chart-format.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('word'))}
    ${commandMenu(host, 'word.chart-format.arrange.align', 'Align', ...alignEntries('word'))}
    ${commandMenu(host, 'word.chart-format.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'word.chart-format.arrange.rotate', 'Rotate', ...rotateEntries())}
  `;
}

/**
 * Word's seventeen menus: **Chart Design's five**, then `wordChartFormatMenus`' twelve. Chart Design: **Chart Layouts'
 * two**, Add Chart Element and Quick Layout; **Chart Styles' one**, Change Colours; **Data's one**, Edit Data's arrow;
 * **Type's one**, Change Chart Type. The Chart Styles gallery is in-ribbon, and Switch Row/Column, Select Data and
 * Refresh Data are plain buttons.
 */
function wordChartToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.chart-design.chart-layouts.add-chart-element', 'Add Chart Element', ...addChartElementEntries())}
    ${commandMenu(host, 'word.chart-design.chart-layouts.quick-layout', 'Quick Layout', ...quickLayoutEntries())}
    ${commandMenu(host, 'word.chart-design.chart-styles.change-colours', 'Change Colours', ...changeColoursEntries())}
    ${commandMenu(host, 'word.chart-design.data.edit-data', 'Edit Data', ...editDataEntries())}
    ${commandMenu(host, 'word.chart-design.type.change-chart-type', 'Change Chart Type', ...changeChartTypeEntries())}
    ${wordChartFormatMenus(host)}
  `;
}

/**
 * PowerPoint's ten Chart Format menus: **Word's twelve less Position and Wrap Text**, which a slide does not have,
 * under PowerPoint's ids. **Insert Shapes' two**: Shapes and Change Shape, over a chart's gallery. **Shape Styles'
 * two**: Other Theme Fills, whose id is the Theme Styles gallery's, and Shape Effects. **WordArt Styles' one**: Text
 * Effects. **Arrange's five**, over `stories/ribbons/design-layout-menus.ts`' PowerPoint lists, as PowerPoint's Shape
 * Format (Align to Slide ticked). The Chart Elements field is a field, and every other command a picker, a gallery, a
 * toggle or a plain button.
 */
function powerpointChartFormatMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.chart-format.insert-shapes.shapes', 'Shapes', ...chartShapesEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.chart-format.insert-shapes.change-shape', 'Change Shape', ...chartChangeShapeEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.chart-format.shape-styles.theme-styles', 'Other Theme Fills', ...otherThemeFillEntries())}
    ${commandMenu(host, 'powerpoint.chart-format.shape-styles.shape-effects', 'Shape Effects', ...shapeEffectsEntries())}
    ${commandMenu(host, 'powerpoint.chart-format.wordart-styles.text-effects', 'Text Effects', ...wordArtTextEffectsEntries())}
    ${commandMenu(host, 'powerpoint.chart-format.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.chart-format.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.chart-format.arrange.align', 'Align', ...alignEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.chart-format.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'powerpoint.chart-format.arrange.rotate', 'Rotate', ...rotateEntries())}
  `;
}

/**
 * PowerPoint's fifteen menus: **Chart Design's five**, Word's five under PowerPoint's ids over the same lists, then
 * `powerpointChartFormatMenus`' ten. Office's PowerPoint Chart Design differs from Word's in no menu that tab opens (the
 * census counts every group the same), so its half writes no list of its own. It is a function only because a
 * command's id carries its application, and `tests/ribbons.test.ts` reads each id from a literal
 * `commandMenu(host, '…'` call.
 */
function powerpointChartToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.chart-design.chart-layouts.add-chart-element', 'Add Chart Element', ...addChartElementEntries())}
    ${commandMenu(host, 'powerpoint.chart-design.chart-layouts.quick-layout', 'Quick Layout', ...quickLayoutEntries())}
    ${commandMenu(host, 'powerpoint.chart-design.chart-styles.change-colours', 'Change Colours', ...changeColoursEntries())}
    ${commandMenu(host, 'powerpoint.chart-design.data.edit-data', 'Edit Data', ...editDataEntries())}
    ${commandMenu(host, 'powerpoint.chart-design.type.change-chart-type', 'Change Chart Type', ...changeChartTypeEntries())}
    ${powerpointChartFormatMenus(host)}
  `;
}

/**
 * Excel's **four** menus: Word's five less Edit Data's, under Excel's ids, over the same lists. **Excel's Data group is
 * Switch Row/Column and Select Data alone** (a workbook's chart reads its own cells, so there is no data sheet to edit),
 * and its fifth group, Location, is Move Chart, a dialog, with no menu. The Chart Styles gallery is in-ribbon.
 */
function excelChartToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.chart-design.chart-layouts.add-chart-element', 'Add Chart Element', ...addChartElementEntries())}
    ${commandMenu(host, 'excel.chart-design.chart-layouts.quick-layout', 'Quick Layout', ...quickLayoutEntries())}
    ${commandMenu(host, 'excel.chart-design.chart-styles.change-colours', 'Change Colours', ...changeColoursEntries())}
    ${commandMenu(host, 'excel.chart-design.type.change-chart-type', 'Change Chart Type', ...changeChartTypeEntries())}
  `;
}

/**
 * Every menu one application's Chart Tools tabs open, with ids for one host's page. **Every Chart Design is authored**:
 * Word's and PowerPoint's five, Excel's four. **Of the three Chart Formats, Word's and PowerPoint's are
 * authored**, twelve menus and ten; Excel's adds its own in its unit.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, by every host that draws Chart Tools **and** binds its
 * commands: `Ribbons/Word`, `Ribbons/PowerPoint` and `Ribbons/Excel`. `Shell/Word` and `Shell/Excel` draw Table Tools
 * and `Shell/PowerPoint` Picture Tools, so none renders any of these, and `tests/ribbons.test.ts` requires each
 * application's menus of its `Ribbons/*` host alone.
 */
export function chartToolsMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  if (application === 'word') return wordChartToolsMenus(host);
  if (application === 'powerpoint') return powerpointChartToolsMenus(host);
  return excelChartToolsMenus(host);
}
