/**
 * **The entries beneath a colour picker's palette**, written once for every colour command the ribbon binds: Word's
 * Table Design Shading and Pen Colour, PowerPoint's Table Design Shading, Text Fill, Text Outline and Pen Colour, Word's,
 * PowerPoint's and Excel's Picture Format Picture Border (PowerPoint's alone with an Eyedropper), and PowerPoint's Shape
 * Format Shape Fill and Shape Outline (Arrows ▸ on the outline), Text Fill and Text Outline, and Word's Shape Format's
 * four, with no Eyedropper and a shorter Text Fill and Text Outline, and Excel's Shape Format's four, with no Eyedropper
 * and PowerPoint's Text Fill and Text Outline otherwise.
 *
 * `<mjx-color-picker>` draws one slotted `<mjx-menu slot="entries">` beneath its swatches; its module note says why it
 * is a real menu and how the keyboard crosses into it. A binding writes the menu inside its picker:
 *
 * ```ts
 * html`<mjx-color-picker label="Text Outline" show-no-fill no-fill-label="No Outline" …>
 *   ${colourPickerEntries('Text Outline', outlineEntries({ moreColours: 'More Outline Colours…', eyedropper: true, … }))}
 * </mjx-color-picker>`
 * ```
 *
 * ## What is here
 *
 * - **`fillEntries(options)`**: More Colours…, Eyedropper, Picture…, Gradient ▸, Texture ▸ and Table Background ▸,
 *   each included by option, in Office's order.
 * - **`outlineEntries(options)`**: More Colours…, Eyedropper, Weight ▸, Sketched ▸, Dashes ▸ and Arrows ▸, the same way.
 * - **The submenus' lists**: `lineWeightEntries` over `tableLineWeights`, `lineDashEntries` over `presetLineDashes`
 *   (both from `table-tools-menus.ts`, where Line Weight, Pen Weight and Pen Style already read them), and
 *   `lineSketchEntries`, `lineArrowEntries`, `gradientEntries`, `textureEntries` and `tableBackgroundEntries` over this
 *   file's `lineSketchStyles`, `lineArrowStyles`, `gradientDirections` and `textures`.
 *
 * ## What is not an entry
 *
 * **No Fill, No Outline and No Colour.** Office lists them among the entries, but choosing one sets the colour to
 * *none*, which is a value the picker already commits from its own chip. A host names that chip with `no-fill-label`
 * rather than writing a second way to reach one value.
 *
 * `GUESS:` every label, order and preset below, from memory of Microsoft 365: which colour commands carry which
 * entries (each binding says what it passes), the *More … Colours…* wording, the nine gradient directions in each
 * variation, the eleven arrow styles and their names (Office draws pictures and names them *Arrow Style 1* to *11*),
 * the four sketched styles, and that Sketched has no *More Lines…*. The twenty-four textures are Office's own names.
 * Where a label differs from Office's spelling the census's wins (*Colours*, *Centre*).
 *
 * **Nothing here dispatches a command.** Each checkable row carries a `value` a later unit can dispatch without a
 * second table: a weight in points, an `ST_PresetLineDashVal` token, an `ask:lineSketchStyleProps` type, a pair of
 * `ST_LineEndType` tokens. Choosing any entry fires `mjx-menu-activate` and closes the picker.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { presetLineDashes, tableLineWeights } from './table-tools-menus.ts';
import { submenu } from './wordart-styles-menus.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string): TemplateResult {
  return html`<mjx-menu-item label=${label}></mjx-menu-item>`;
}

/** One entry of a set whose current member is checked. */
function choice(label: string, value: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} value=${value} ?checked=${checked}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

/** `Linear Diagonal - Top Left to Bottom Right` → `linear-diagonal-top-left-to-bottom-right`. */
function slug(label: string): string {
  return label.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/**
 * **The menu a picker slots beneath its palette**, holding `entries`. `label` names the menu for a screen reader;
 * a binding passes its command's own label.
 */
export function colourPickerEntries(label: string, entries: readonly TemplateResult[]): TemplateResult {
  return html`<mjx-menu slot="entries" label=${label}>${entries}</mjx-menu>`;
}

// ── the lists ────────────────────────────────────────────────────────────────

/**
 * **Sketched's four styles**: a straight line, then Office's three hand-drawn ones. Each `value` is the `ask:type` of
 * an `ask:lineSketchStyleProps` extension, which is where Office writes a sketched line.
 */
export const lineSketchStyles: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'lineSketchNone', label: 'Straight' },
  { value: 'lineSketchCurved', label: 'Curved' },
  { value: 'lineSketchFreehand', label: 'Freehand' },
  { value: 'lineSketchScribble', label: 'Scribble' },
];

/**
 * **Arrows' eleven styles**, a line's two ends. Each `value` is `<head>/<tail>`, the `ST_LineEndType` of `a:headEnd`
 * and of `a:tailEnd`; the labels describe the picture Office draws, rather than its *Arrow Style N*.
 */
export const lineArrowStyles: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'none/none', label: 'No Arrows' },
  { value: 'none/triangle', label: 'Arrow at End' },
  { value: 'triangle/none', label: 'Arrow at Start' },
  { value: 'triangle/triangle', label: 'Arrows at Both Ends' },
  { value: 'none/arrow', label: 'Open Arrow at End' },
  { value: 'arrow/none', label: 'Open Arrow at Start' },
  { value: 'arrow/arrow', label: 'Open Arrows at Both Ends' },
  { value: 'none/stealth', label: 'Stealth Arrow at End' },
  { value: 'oval/oval', label: 'Round Ends' },
  { value: 'diamond/diamond', label: 'Diamond Ends' },
  { value: 'oval/triangle', label: 'Round Start, Arrow at End' },
];

/** **A gradient's nine directions**, a grid read across its rows. Gradient lists them twice, light and dark. */
export const gradientDirections: readonly string[] = [
  'Linear Diagonal - Top Left to Bottom Right',
  'Linear Down',
  'Linear Diagonal - Top Right to Bottom Left',
  'Linear Right',
  'From Centre',
  'Linear Left',
  'Linear Diagonal - Bottom Left to Top Right',
  'Linear Up',
  'Linear Diagonal - Bottom Right to Top Left',
];

/** **Texture's twenty-four textures**, Office's own, a grid read across its rows. */
export const textures: readonly string[] = [
  'Papyrus',
  'Canvas',
  'Denim',
  'Woven Mat',
  'Water Droplets',
  'Paper Bag',
  'Fish Fossil',
  'Sand',
  'Green Marble',
  'White Marble',
  'Brown Marble',
  'Granite',
  'Newsprint',
  'Recycled Paper',
  'Parchment',
  'Stationery',
  'Blue Tissue Paper',
  'Pink Tissue Paper',
  'Purple Mesh',
  'Bouquet',
  'Cork',
  'Walnut',
  'Oak',
  'Medium Wood',
];

// ── the submenus ─────────────────────────────────────────────────────────────

/** **Weight ▸**: the nine weights, ¼ pt to 6 pt, then More Lines…. Nothing is checked until a weight is chosen. */
export function lineWeightEntries(current?: string): TemplateResult[] {
  return [
    ...tableLineWeights.map((weight) => choice(weight.label, weight.value, weight.value === current)),
    separator(),
    item('More Lines…'),
  ];
}

/** **Dashes ▸**: PowerPoint's eight dashes, then More Lines…. */
export function lineDashEntries(current?: string): TemplateResult[] {
  return [
    ...presetLineDashes.map((dash) => choice(dash.label, dash.value, dash.value === current)),
    separator(),
    item('More Lines…'),
  ];
}

/** **Sketched ▸**: the four styles, and no options entry. */
export function lineSketchEntries(current?: string): TemplateResult[] {
  return lineSketchStyles.map((style) => choice(style.label, style.value, style.value === current));
}

/** **Arrows ▸**: the eleven styles, then More Arrows…. */
export function lineArrowEntries(current?: string): TemplateResult[] {
  return [
    ...lineArrowStyles.map((arrows) => choice(arrows.label, arrows.value, arrows.value === current)),
    separator(),
    item('More Arrows…'),
  ];
}

/** **Gradient ▸**: No Gradient, the nine directions light and then dark, then More Gradients…. */
export function gradientEntries(): TemplateResult[] {
  const variation = (tone: 'Light' | 'Dark'): TemplateResult =>
    section(
      `${tone} Variations`,
      ...gradientDirections.map((direction) => html`<mjx-menu-item
          label=${direction}
          value=${`${tone.toLowerCase()}-${slug(direction)}`}
        ></mjx-menu-item>`),
    );
  return [item('No Gradient'), variation('Light'), variation('Dark'), separator(), item('More Gradients…')];
}

/** **Texture ▸**: the twenty-four textures, then More Textures…. */
export function textureEntries(): TemplateResult[] {
  return [
    section('Textures', ...textures.map((texture) => html`<mjx-menu-item label=${texture} value=${slug(texture)}></mjx-menu-item>`)),
    separator(),
    item('More Textures…'),
  ];
}

/**
 * **Table Background ▸**, PowerPoint's: the fill behind a whole table rather than its cells. No Fill, More Fill
 * Colours…, Eyedropper and Picture….
 *
 * ⚠ **Office also draws a theme and standard colour grid in this submenu, and a menu cannot hold one.** A second
 * `<mjx-color-picker>` inside a submenu would be a combo box inside a menu row, and the grid here is a copy of the one
 * directly above it. `GUESS:` that the four commands are the rest of it.
 */
export function tableBackgroundEntries(): TemplateResult[] {
  return [item('No Fill'), item('More Fill Colours…'), item('Eyedropper'), item('Picture…')];
}

// ── the two families ─────────────────────────────────────────────────────────

/** Which fill entries a colour command carries. `moreColours` is its own wording; every other entry is opt-in. */
export interface FillEntryOptions {
  /** *More Colours…* (Word) or *More Fill Colours…* (a PowerPoint fill). */
  readonly moreColours: string;
  readonly eyedropper?: boolean;
  readonly picture?: boolean;
  readonly gradient?: boolean;
  readonly texture?: boolean;
  /** PowerPoint's table Shading alone. */
  readonly tableBackground?: boolean;
}

/**
 * **A fill's entries**: Shading, Text Fill, Shape Fill. In Office's order: More Colours…, Eyedropper, Picture…,
 * Gradient ▸, Texture ▸, Table Background ▸.
 */
export function fillEntries(options: FillEntryOptions): TemplateResult[] {
  return [
    item(options.moreColours),
    ...(options.eyedropper === true ? [item('Eyedropper')] : []),
    ...(options.picture === true ? [item('Picture…')] : []),
    ...(options.gradient === true ? [submenu('Gradient', ...gradientEntries())] : []),
    ...(options.texture === true ? [submenu('Texture', ...textureEntries())] : []),
    ...(options.tableBackground === true ? [submenu('Table Background', ...tableBackgroundEntries())] : []),
  ];
}

/** Which outline entries a colour command carries. */
export interface OutlineEntryOptions {
  /** *More Colours…* (a pen) or *More Outline Colours…* (an outline). */
  readonly moreColours: string;
  readonly eyedropper?: boolean;
  readonly weight?: boolean;
  readonly sketched?: boolean;
  readonly dashes?: boolean;
  /** A shape's outline alone: text has no ends. */
  readonly arrows?: boolean;
}

/**
 * **An outline's entries**: Text Outline, Shape Outline, and a pen. In Office's order: More Colours…, Eyedropper,
 * Weight ▸, Sketched ▸, Dashes ▸, Arrows ▸.
 */
export function outlineEntries(options: OutlineEntryOptions): TemplateResult[] {
  return [
    item(options.moreColours),
    ...(options.eyedropper === true ? [item('Eyedropper')] : []),
    ...(options.weight === true ? [submenu('Weight', ...lineWeightEntries())] : []),
    ...(options.sketched === true ? [submenu('Sketched', ...lineSketchEntries())] : []),
    ...(options.dashes === true ? [submenu('Dashes', ...lineDashEntries())] : []),
    ...(options.arrows === true ? [submenu('Arrows', ...lineArrowEntries())] : []),
  ];
}
