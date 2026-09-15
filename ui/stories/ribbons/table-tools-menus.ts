/**
 * **The menus, fields and gallery the Table Tools tabs open**, written once for both hosts: Word's and PowerPoint's
 * Table Design and Table Layout, and Excel's Table Design.
 *
 * The pattern is `stories/ribbons/slide-master-menus.ts`'s, for its reasons. A binding lives in its host. The menu it
 * opens is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `tableToolsMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below spells its
 * command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## Three things here are not menus
 *
 * 1. **The Table Styles galleries.** Both Word hosts bind `<mjx-gallery>` and fill it from
 *    `wordTableStyleGalleryItems(palette)` and `wordTableStyleGalleryFooter()`; `Ribbons/PowerPoint` fills its own from
 *    `powerpointTableStyleGalleryItems(palette)` and `powerpointTableStyleGalleryFooter()`; both Excel hosts fill theirs
 *    from `excelTableStyleGalleryItems(palette)` and `excelTableStyleGalleryFooter()`.
 * 2. **The field lists**, `wordBorderLineStyles` (Line Style), `powerpointPenStyles` (Pen Style) and
 *    `tableLineWeights` (Line Weight and Pen Weight), which each host maps onto `<mjx-option>`s, as `ribbon-parts.ts`'s
 *    shared field lists are, and `excelTableName`, the one name Excel's Table Name field starts on and lists.
 * 3. **No colour list.** Shading and Pen Colour are colour pickers over the document's palette, which a host owns.
 *    The entries beneath their palettes (More Colours…, Gradient ▸, Weight ▸) are
 *    `stories/ribbons/colour-picker-entries.ts`'s, which reads `tableLineWeights` and `presetLineDashes` from here.
 *
 * **PowerPoint's WordArt Styles group is not here**: its gallery and effect lists are
 * `stories/ribbons/wordart-styles-menus.ts`'s, because Shape Format and Chart Format carry the same group. Its Text
 * Effects menu is rendered here, beside Table Design's other menus, because its id is Table Design's.
 *
 * ## Shaped for three Table Design tabs
 *
 * Only what is **the same in all three applications** is written as shared, and everything Word's alone says so in
 * its name. `GUESS:` each of these readings of Office, from memory of Microsoft 365:
 *
 * - **`tableStylePicture` and `tableStyleGalleryItems` are shared.** Every Table Styles gallery is a list of named
 *   styles in categories, each drawn as a small table in the document's colours. Word's categories are Plain, Grid
 *   and List Tables; PowerPoint's are Best Match for Document, Light, Medium and Dark; Excel's are Light, Medium and
 *   Dark. So each application's unit writes its own style list as `TableStyleSpec`s and reuses the picture and the
 *   item builder, which is where the work is.
 * - **`tableLineWeights` is shared.** PowerPoint's Pen Weight offers the same nine weights from ¼ pt to 6 pt, and its
 *   values are points here for that reason, rather than Word's eighths of a point (`w:sz`). **PowerPoint's Table Design
 *   now calls it.**
 * - **The line styles, the Borders menu and the Border Styles menu are Word's.** PowerPoint's Pen Style is a shorter
 *   list of dashes, its Borders menu lists No Border and All Borders first and has no Horizontal Line, Draw Table or
 *   View Gridlines, and it has no Border Styles. Excel's Table Design has no borders at all. **PowerPoint's are written
 *   below as its own**: `powerpointPenStyles`, `powerpointBorderEntries`, its 74 styles and its Effects menu.
 * - **Excel's Table Design calls the picture and the item builder**, and writes its own 61 styles and its Export and
 *   Refresh menus below. It has no line weights, pens or borders: a worksheet table's lines are its cells'.
 * - **`tableSelectEntries` and `tableDeleteEntries` are shared by the two Table Layout tabs**, and take the
 *   application: Word's lists start with a cell (Select Cell, Delete Cells…) and PowerPoint's cannot select or delete
 *   one cell. **PowerPoint's Table Layout now calls both.** **AutoFit's list is Word's**: PowerPoint's Table Layout
 *   has no AutoFit. **Text Direction's and Cell Margins' lists are PowerPoint's**: Word's Text Direction is a plain
 *   button and its Cell Margins opens a dialog. Excel has no Table Layout.
 * - **PowerPoint's Arrange menus are not written here**: Bring Forward's, Send Backward's and Align's entries are
 *   `stories/ribbons/design-layout-menus.ts`' Arrange lists, which take the application. The menus themselves are
 *   rendered here, because their ids are Table Layout's.
 *
 * ## The pictures are the document's colours, not the chrome's
 *
 * A table style is drawn in the theme of the document it is applied to: Grid Table 4 - Accent 2 is the document's
 * second accent, whatever that document's theme is. So the pictures take the document's `ThemeColorPalette` as an
 * argument, from the host that owns it, and **carry no colour of this file's own**. The chrome's tokens appear only
 * where the document has not said: `--document-page` for a palette with no Background 1, `--theme-text-primary` for
 * one with no Text 1 or no accent. That is the colour picker's rule applied to a picture.
 *
 * ⚠ **A picture carries no lit binding, and that is why it is built with `lit/static-html.js`.** `<mjx-gallery-item>`
 * captures its children into a fragment and the gallery clones that fragment into its shadow root, as
 * `stories/gallery/specimens.ts` records, so the art must be static markup. 105 styles cannot be 105 hand-written
 * templates, so each picture is one markup string with no parts, made static by `unsafeStatic`. **Every colour that
 * reaches the string is a hex colour checked against `hexColour`**, and anything else becomes a token, so a palette
 * value cannot inject markup.
 *
 * **Nothing here dispatches a command.** A menu opens, an entry can be chosen, a gallery previews and commits; no
 * table changes, because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, nothing, type TemplateResult } from 'lit';
import { html as staticHtml, unsafeStatic } from 'lit/static-html.js';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import type { ThemeColorPalette, ThemeColorSlot } from '../../src/pickers/picker-model.ts';
import { alignEntries, bringForwardEntries, sendBackwardEntries } from './design-layout-menus.ts';
import { hexColour } from './palette-art.ts';
import { commandMenu } from './ribbon-parts.ts';
import {
  bevelPresetEntries,
  reflectionPresetEntries,
  shadowPresetEntries,
  submenu,
  wordArtTextEffectsEntries,
} from './wordart-styles-menus.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry, with the keyboard shortcut Office prints beside it where it prints one. */
function item(label: string, shortcut?: string): TemplateResult {
  return html`<mjx-menu-item label=${label} shortcut=${shortcut ?? nothing}></mjx-menu-item>`;
}

/** One entry that is on or off by itself. */
function option(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="checkbox" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

// ── the fields: Line Style and Line Weight ───────────────────────────────────

/**
 * **Word's Line Style list**: No Border, then the twenty-four border styles Word's Borders and Shading dialog offers,
 * in that dialog's order. A host starts on `single`.
 *
 * Each `value` is the style's `ST_Border` wire token (`w:val` on a border), so a later unit that dispatches one needs
 * no second table. Office draws each entry as a picture of the line with no name; the labels are the names a screen
 * reader needs, and `GUESS:` every one of them and the order.
 */
export const wordBorderLineStyles: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'none', label: 'No Border' },
  { value: 'single', label: 'Single' },
  { value: 'dotted', label: 'Dotted' },
  { value: 'dashSmallGap', label: 'Dashed (small gap)' },
  { value: 'dashed', label: 'Dashed (large gap)' },
  { value: 'dotDash', label: 'Dot dash' },
  { value: 'dotDotDash', label: 'Dot dot dash' },
  { value: 'double', label: 'Double' },
  { value: 'triple', label: 'Triple' },
  { value: 'thinThickSmallGap', label: 'Thin-thick (small gap)' },
  { value: 'thickThinSmallGap', label: 'Thick-thin (small gap)' },
  { value: 'thinThickThinSmallGap', label: 'Thin-thick-thin (small gap)' },
  { value: 'thinThickMediumGap', label: 'Thin-thick (medium gap)' },
  { value: 'thickThinMediumGap', label: 'Thick-thin (medium gap)' },
  { value: 'thinThickThinMediumGap', label: 'Thin-thick-thin (medium gap)' },
  { value: 'thinThickLargeGap', label: 'Thin-thick (large gap)' },
  { value: 'thickThinLargeGap', label: 'Thick-thin (large gap)' },
  { value: 'thinThickThinLargeGap', label: 'Thin-thick-thin (large gap)' },
  { value: 'wave', label: 'Wave' },
  { value: 'doubleWave', label: 'Double wave' },
  { value: 'dashDotStroked', label: 'Dash dot stroked' },
  { value: 'threeDEmboss', label: '3-D emboss' },
  { value: 'threeDEngrave', label: '3-D engrave' },
  { value: 'outset', label: 'Outset' },
  { value: 'inset', label: 'Inset' },
];

/**
 * **Line Weight's nine weights**, ¼ pt to 6 pt, as Office writes them. A Word host starts on `0.5`, a new table's
 * ½ pt border. Shared: PowerPoint's Pen Weight offers the same nine (`GUESS:`), so each `value` is in points rather
 * than Word's eighths of a point.
 */
export const tableLineWeights: readonly { readonly value: string; readonly label: string }[] = [
  { value: '0.25', label: '¼ pt' },
  { value: '0.5', label: '½ pt' },
  { value: '0.75', label: '¾ pt' },
  { value: '1', label: '1 pt' },
  { value: '1.5', label: '1 ½ pt' },
  { value: '2.25', label: '2 ¼ pt' },
  { value: '3', label: '3 pt' },
  { value: '4.5', label: '4 ½ pt' },
  { value: '6', label: '6 pt' },
];

// ── the menus: Border Styles and Borders, Word's ─────────────────────────────

/**
 * **Border Styles' list, Word's**: *Theme Borders*, twenty-one borders in three weights across Text 1 and the six
 * accents, then *Border Sampler*, which picks a border up from the table as Format Painter picks up formatting.
 *
 * Office adds a *Recently Used Borders* row once a border has been applied, and none has. Nothing is checked: a
 * border style loads the pen rather than being a state the table is in. `GUESS:` the three weights, the order, and
 * that each entry's name is its tooltip.
 */
function wordBorderStyleEntries(): TemplateResult[] {
  const lines = ['Single solid line, ½ pt', 'Double solid lines, ½ pt', 'Single solid line, 1 ½ pt'];
  const colours = ['Text 1', 'Accent 1', 'Accent 2', 'Accent 3', 'Accent 4', 'Accent 5', 'Accent 6'];
  return [
    section('Theme Borders', ...lines.flatMap((line) => colours.map((colour) => item(`${line}, ${colour}`)))),
    separator(),
    item('Border Sampler'),
  ];
}

/**
 * **Borders' arrow, Word's**: the sixteen entries Office lists under Table Design's Borders, which are Home's
 * Paragraph Borders list in a table. The four edges; none, all, outside and inside; the inside and diagonal lines;
 * Horizontal Line; then Draw Table, View Gridlines and the dialog.
 *
 * **View Gridlines is a state**, ticked (`GUESS:` that a new install shows gridlines). No entry carries a glyph:
 * Fluent draws eight of the sixteen borders and none of the inside or diagonal lines, and a list with gaps would read
 * as broken, the reason Slide Master's Insert Placeholder gives. `GUESS:` the separators.
 *
 * Home's own Borders command does not open this list yet; it is a button whose unit predates the list. Its binding
 * is the one line that would change.
 */
function wordBorderEntries(): TemplateResult[] {
  return [
    item('Bottom Border'),
    item('Top Border'),
    item('Left Border'),
    item('Right Border'),
    separator(),
    item('No Border'),
    item('All Borders'),
    item('Outside Borders'),
    item('Inside Borders'),
    separator(),
    item('Inside Horizontal Border'),
    item('Inside Vertical Border'),
    item('Diagonal Down Border'),
    item('Diagonal Up Border'),
    separator(),
    item('Horizontal Line'),
    separator(),
    item('Draw Table'),
    option('View Gridlines', true),
    item('Borders and Shading…'),
  ];
}

// ── the gallery: one picture and one item builder, for every Table Styles ────

/** What a table style's picture shows. A description of a look, never a render of the style's `w:tblStylePr`. */
export interface TableStylePicture {
  /** The header row: a solid band in the tint, a bold row with a rule under it, bold alone, or italic with a rule. */
  readonly header: 'filled' | 'ruled' | 'bold' | 'italic';
  /** The cell lines drawn: every line, the lines between rows, the outer edge, or none. */
  readonly lines: 'grid' | 'rows' | 'outline' | 'none';
  /** How strong the lines are: the tint itself, or the tint mixed half into the paper. */
  readonly lineStrength: 'full' | 'light';
  /** Whether alternate body rows are shaded. */
  readonly banded: boolean;
  /** Whether the first column is set apart: filled like the header, or in its weight. */
  readonly firstColumn: 'filled' | 'emphasised' | 'plain';
  /** Every body cell shaded in the tint, as a *Dark* style is. */
  readonly bodyFilled: boolean;
  /** Text in the tint, as a *Colorful* style is, rather than in Text 1. */
  readonly tintedText: boolean;
}

/** One style in a gallery: its name, its section, which document colour tints it, and its look. */
export interface TableStyleSpec {
  readonly value: string;
  readonly label: string;
  readonly category: string;
  /** The document colour the style is drawn in. `text1` is a style with no accent. */
  readonly tint: ThemeColorSlot;
  readonly picture: TableStylePicture;
}

/** The colours one picture is drawn in, every one either a checked hex colour or a token. */
interface PictureColours {
  readonly paper: string;
  readonly text: string;
  readonly tint: string;
  readonly soft: string;
  readonly medium: string;
}

function pictureColours(palette: ThemeColorPalette, slot: ThemeColorSlot): PictureColours {
  const paper = hexColour(palette.background1) ?? 'var(--document-page)';
  const text = hexColour(palette.text1) ?? 'var(--theme-text-primary)';
  const tint = hexColour(palette[slot]) ?? text;
  return {
    paper,
    text,
    tint,
    soft: `color-mix(in srgb, ${tint} 22%, ${paper})`,
    medium: `color-mix(in srgb, ${tint} 55%, ${paper})`,
  };
}

/** How many columns and body rows a picture draws. Five by four reads as a table at a gallery cell's size. */
const pictureColumns = 5;
const pictureBodyRows = 4;

/**
 * **One table style's picture**: a five-column table with a header row and four body rows, drawn in the document's
 * colours, as static markup.
 *
 * Each cell is a shaded box holding a short bar standing for its text, and the bar's weight and colour carry the
 * header's and first column's emphasis. Every size is relative to the gallery cell and every colour comes from
 * `pictureColours`, so the markup string holds no literal value of its own.
 */
export function tableStylePicture(look: TableStylePicture, palette: ThemeColorPalette, slot: ThemeColorSlot): TemplateResult {
  const colour = pictureColours(palette, slot);
  const line = look.lineStrength === 'full' ? colour.tint : colour.medium;
  const cells: string[] = [];
  for (let row = 0; row <= pictureBodyRows; row += 1) {
    for (let column = 0; column < pictureColumns; column += 1) {
      const header = row === 0;
      const firstColumn = column === 0 && !header;
      const bandedRow = !header && look.banded && row % 2 === 1;
      let fill = look.bodyFilled && !header ? colour.soft : 'transparent';
      if (bandedRow) fill = look.bodyFilled ? colour.medium : colour.soft;
      if ((header && look.header === 'filled') || (firstColumn && look.firstColumn === 'filled')) fill = colour.tint;
      const onTint = (header && look.header === 'filled') || (firstColumn && look.firstColumn === 'filled');
      const ink = onTint ? colour.paper : look.tintedText ? colour.tint : colour.text;
      const strong = (header && look.header !== 'italic') || (firstColumn && look.firstColumn !== 'plain');
      const borders: string[] = [];
      if (look.lines === 'grid') {
        borders.push(`border-block-end:1px solid ${line}`);
        if (column < pictureColumns - 1) borders.push(`border-inline-end:1px solid ${line}`);
      } else if (look.lines === 'rows' && row < pictureBodyRows) {
        borders.push(`border-block-end:1px solid ${line}`);
      }
      if (header && (look.header === 'ruled' || look.header === 'italic')) {
        borders.push(`border-block-end:calc(var(--spacing) * 0.5) solid ${colour.tint}`);
      }
      cells.push(
        `<span style="display:grid;place-items:center;block-size:calc(var(--spacing) * 1.25);background:${fill};${borders.join(';')}">` +
          `<span style="inline-size:${strong ? '70%' : '55%'};block-size:${strong ? 'calc(var(--spacing) * 0.5)' : '1px'};background:${ink}"></span></span>`,
      );
    }
  }
  const outline = look.lines === 'outline' || look.lines === 'grid' ? `border:1px solid ${line};` : '';
  const markup =
    `<span style="display:grid;grid-template-columns:repeat(${String(pictureColumns)},1fr);inline-size:100%;` +
    `background:${colour.paper};${outline}box-sizing:border-box">${cells.join('')}</span>`;
  return staticHtml`${unsafeStatic(markup)}`;
}

/**
 * **A Table Styles gallery's items**, one `<mjx-gallery-item>` per style, drawn in the document's palette. Shared by
 * every application's Table Design: each unit passes its own `TableStyleSpec` list.
 */
export function tableStyleGalleryItems(styles: readonly TableStyleSpec[], palette: ThemeColorPalette): TemplateResult[] {
  return styles.map(
    (style) => html`<mjx-gallery-item value=${style.value} label=${style.label} category=${style.category}
      >${tableStylePicture(style.picture, palette, style.tint)}</mjx-gallery-item
    >`,
  );
}

// ── Word's table styles ──────────────────────────────────────────────────────

/** A look, with every field a style family does not change left at its plainest. */
function look(overrides: Partial<TableStylePicture>): TableStylePicture {
  return {
    header: 'bold',
    lines: 'none',
    lineStrength: 'light',
    banded: false,
    firstColumn: 'plain',
    bodyFilled: false,
    tintedText: false,
    ...overrides,
  };
}

/**
 * **Word's Plain Tables**, which have no accent. `GUESS:` each look, from memory of the gallery's thumbnails.
 */
const wordPlainTables: readonly { readonly label: string; readonly picture: TableStylePicture }[] = [
  { label: 'Table Grid', picture: look({ header: 'bold', lines: 'grid', lineStrength: 'full' }) },
  { label: 'Table Grid Light', picture: look({ header: 'bold', lines: 'grid' }) },
  { label: 'Plain Table 1', picture: look({ lines: 'grid', banded: true, firstColumn: 'emphasised' }) },
  { label: 'Plain Table 2', picture: look({ header: 'ruled', lines: 'rows', firstColumn: 'emphasised' }) },
  { label: 'Plain Table 3', picture: look({ header: 'ruled', banded: true, firstColumn: 'emphasised' }) },
  { label: 'Plain Table 4', picture: look({ banded: true, firstColumn: 'emphasised' }) },
  { label: 'Plain Table 5', picture: look({ header: 'italic', banded: true, firstColumn: 'emphasised' }) },
];

/**
 * **Word's seven Grid Table families and seven List Table families**, each drawn once without an accent and once per
 * accent. `GUESS:` each look; the names are Word's built-in style names.
 */
const wordGridFamilies: readonly { readonly label: string; readonly picture: TableStylePicture }[] = [
  { label: 'Grid Table 1 Light', picture: look({ header: 'ruled', lines: 'grid', firstColumn: 'emphasised' }) },
  { label: 'Grid Table 2', picture: look({ header: 'ruled', lines: 'rows', banded: true, firstColumn: 'emphasised' }) },
  { label: 'Grid Table 3', picture: look({ header: 'bold', lines: 'grid', banded: true, firstColumn: 'emphasised' }) },
  { label: 'Grid Table 4', picture: look({ header: 'filled', lines: 'grid', banded: true, firstColumn: 'emphasised' }) },
  { label: 'Grid Table 5 Dark', picture: look({ header: 'filled', lines: 'grid', banded: true, firstColumn: 'filled', bodyFilled: true }) },
  { label: 'Grid Table 6 Colorful', picture: look({ header: 'ruled', lines: 'grid', banded: true, firstColumn: 'emphasised', tintedText: true }) },
  { label: 'Grid Table 7 Colorful', picture: look({ header: 'italic', lines: 'grid', banded: true, firstColumn: 'emphasised', tintedText: true }) },
];

const wordListFamilies: readonly { readonly label: string; readonly picture: TableStylePicture }[] = [
  { label: 'List Table 1 Light', picture: look({ header: 'ruled', banded: true, firstColumn: 'emphasised' }) },
  { label: 'List Table 2', picture: look({ header: 'bold', lines: 'rows', banded: true, firstColumn: 'emphasised' }) },
  { label: 'List Table 3', picture: look({ header: 'filled', lines: 'outline', lineStrength: 'full', firstColumn: 'emphasised' }) },
  { label: 'List Table 4', picture: look({ header: 'filled', lines: 'rows', banded: true, firstColumn: 'emphasised' }) },
  { label: 'List Table 5 Dark', picture: look({ header: 'ruled', firstColumn: 'filled', bodyFilled: true, banded: true }) },
  { label: 'List Table 6 Colorful', picture: look({ header: 'ruled', banded: true, firstColumn: 'emphasised', tintedText: true }) },
  { label: 'List Table 7 Colorful', picture: look({ header: 'italic', banded: true, firstColumn: 'emphasised', tintedText: true }) },
];

/** The six accents, as Word's style names spell them and as the palette's slots name them. */
const accents: readonly { readonly suffix: string; readonly slot: ThemeColorSlot }[] = [
  { suffix: 'Accent 1', slot: 'accent1' },
  { suffix: 'Accent 2', slot: 'accent2' },
  { suffix: 'Accent 3', slot: 'accent3' },
  { suffix: 'Accent 4', slot: 'accent4' },
  { suffix: 'Accent 5', slot: 'accent5' },
  { suffix: 'Accent 6', slot: 'accent6' },
];

/** `Grid Table 4 - Accent 1` → `grid-table-4-accent-1`. */
function styleValue(label: string): string {
  return label.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/** One family, without an accent and then in each accent, in Office's order: one gallery row per family. */
function familyStyles(
  category: string,
  families: readonly { readonly label: string; readonly picture: TableStylePicture }[],
): TableStyleSpec[] {
  return families.flatMap((family) => [
    { value: styleValue(family.label), label: family.label, category, tint: 'text1' as const, picture: family.picture },
    ...accents.map((accent) => {
      const label = `${family.label} - ${accent.suffix}`;
      return { value: styleValue(label), label, category, tint: accent.slot, picture: family.picture };
    }),
  ]);
}

/**
 * **Word's whole built-in Table Styles gallery**: 105 styles. *Plain Tables* (7), then *Grid Tables* and *List
 * Tables*, seven families each drawn without an accent and in the six accents (49 each). The order is Office's, one
 * family to a row of seven.
 *
 * ⚠ **The names keep Word's own spelling**, *Colorful* included, where the census writes *Colour*: a table style's
 * name is data written into the document (`w:style/w:name`), and the gallery must name the style a file Word wrote
 * actually carries. `GUESS:` that Word lists no *Custom* section for a document with no custom table style.
 */
export const wordTableStyles: readonly TableStyleSpec[] = [
  ...wordPlainTables.map((style) => ({
    value: styleValue(style.label),
    label: style.label,
    category: 'Plain Tables',
    tint: 'text1' as const,
    picture: style.picture,
  })),
  ...familyStyles('Grid Tables', wordGridFamilies),
  ...familyStyles('List Tables', wordListFamilies),
];

/** Word's Table Styles gallery items, in the document's palette. A host starts the gallery on `table-grid`. */
export function wordTableStyleGalleryItems(palette: ThemeColorPalette): TemplateResult[] {
  return tableStyleGalleryItems(wordTableStyles, palette);
}

/**
 * **The three commands under Word's expanded Table Styles gallery**: Modify Table Style…, Clear and New Table
 * Style…, in Office's order. `slot="footer"`, the gallery's own seam. `GUESS:` the order.
 */
export function wordTableStyleGalleryFooter(): TemplateResult[] {
  return ['Modify Table Style…', 'Clear', 'New Table Style…'].map(
    (label) => html`<mjx-button slot="footer" label=${label} size="small"></mjx-button>`,
  );
}

// ── the menus: Select, Delete and AutoFit, for Table Layout ──────────────────

/** The two applications with a Table Layout tab. Excel's Table Tools set has Table Design alone. */
export type TableLayoutApplication = Extract<RibbonApplication, 'word' | 'powerpoint'>;

/**
 * **Select's list**, shared by Word's and PowerPoint's Table Layout. Word's is Select Cell, Select Column, Select Row
 * and Select Table, in Office's order. PowerPoint selects no single cell from the ribbon, so its list is Select
 * Table, Select Column and Select Row.
 *
 * Nothing is checked: a selection is where the insertion point is, not a state the table keeps. No entry carries a
 * glyph, as none of Table Design's menus does. **PowerPoint's Table Layout calls it**, and the census's count for its
 * Table group (5: Select, three entries, View Gridlines) is met by a three-entry list. `GUESS:` PowerPoint's order.
 */
export function tableSelectEntries(application: TableLayoutApplication): TemplateResult[] {
  return application === 'word'
    ? [item('Select Cell'), item('Select Column'), item('Select Row'), item('Select Table')]
    : [item('Select Table'), item('Select Column'), item('Select Row')];
}

/**
 * **Delete's list**, shared by Word's and PowerPoint's Table Layout. Word's is Delete Cells…, which opens the Delete
 * Cells dialog to choose how the others shift, then Delete Columns, Delete Rows and Delete Table. PowerPoint's table
 * cannot lose one cell, so its list is Delete Columns, Delete Rows and Delete Table.
 *
 * **PowerPoint's Table Layout calls it**, and the census's count for its Rows & Columns group (8: Delete, three
 * entries, four inserts) is met by a three-entry list. `GUESS:` PowerPoint's order.
 */
export function tableDeleteEntries(application: TableLayoutApplication): TemplateResult[] {
  const shared = [item('Delete Columns'), item('Delete Rows'), item('Delete Table')];
  return application === 'word' ? [item('Delete Cells…'), ...shared] : shared;
}

/**
 * **AutoFit's list, Word's**: AutoFit Contents, AutoFit Window and Fixed Column Width, in Office's order. PowerPoint
 * has no AutoFit.
 *
 * **Nothing is checked**, although a table is in exactly one of the three: Office draws the list as three commands
 * rather than a radio set. `GUESS:` that it ticks none.
 */
function wordAutoFitEntries(): TemplateResult[] {
  return [item('AutoFit Contents'), item('AutoFit Window'), item('Fixed Column Width')];
}

// ── PowerPoint's Table Design: Pen Style, Borders, Effects and the 74 styles ──

/**
 * **PowerPoint's Pen Style list**: No Border, then PowerPoint's eight dashes, in Office's order. A host starts on
 * `solid`.
 *
 * Each `value` is the dash's `ST_PresetLineDashVal` token (`a:prstDash`), so a later unit that dispatches one needs no
 * second table, **except No Border**, which is not a dash but a line with no fill, and is `none`. Office draws each
 * entry as a picture of the line; the labels are PowerPoint's dash names. `GUESS:` that the list is exactly these,
 * in this order.
 */
export const powerpointPenStyles: readonly { readonly value: string; readonly label: string }[] = [
  { value: 'none', label: 'No Border' },
  ...presetLineDashesList(),
];

/**
 * **PowerPoint's eight dashes**, in Office's order, each `value` its `ST_PresetLineDashVal` token. Shared: Pen Style
 * lists them after No Border, and a colour picker's **Dashes ▸** entry lists them alone
 * (`stories/ribbons/colour-picker-entries.ts`).
 */
export const presetLineDashes: readonly { readonly value: string; readonly label: string }[] = presetLineDashesList();

/** A function rather than a constant read by `powerpointPenStyles`, because that constant is declared first. */
function presetLineDashesList(): { readonly value: string; readonly label: string }[] {
  return [
    { value: 'solid', label: 'Solid' },
    { value: 'sysDot', label: 'Round Dot' },
    { value: 'sysDash', label: 'Square Dot' },
    { value: 'dash', label: 'Dash' },
    { value: 'dashDot', label: 'Dash Dot' },
    { value: 'lgDash', label: 'Long Dash' },
    { value: 'lgDashDot', label: 'Long Dash Dot' },
    { value: 'lgDashDotDot', label: 'Long Dash Dot Dot' },
  ];
}

/**
 * **Borders' arrow, PowerPoint's**: twelve entries, No Border and All Borders first, as the header records. No
 * Horizontal Line, Draw Table, View Gridlines or dialog: Draw Table is on the tab itself, and a slide has no page
 * gridlines for a table. No entry carries a glyph, for Word's reason. `GUESS:` the order and that there is no
 * separator.
 */
function powerpointBorderEntries(): TemplateResult[] {
  return [
    'No Border',
    'All Borders',
    'Outside Borders',
    'Inside Borders',
    'Top Border',
    'Bottom Border',
    'Left Border',
    'Right Border',
    'Inside Horizontal Border',
    'Inside Vertical Border',
    'Diagonal Down Border',
    'Diagonal Up Border',
  ].map((label) => item(label));
}

/**
 * **Effects' menu, PowerPoint's**: Cell Bevel, Shadow and Reflection, each a submenu. Shadow and Reflection are the
 * lists Text Effects shows, from `stories/ribbons/wordart-styles-menus.ts`; Cell Bevel is Bevel's twelve with No Bevel
 * and no options entry. `GUESS:` that a table's Shadow list is text's, and that Cell Bevel has no options entry.
 */
function powerpointTableEffectsEntries(): TemplateResult[] {
  return [
    submenu('Cell Bevel', ...bevelPresetEntries()),
    submenu('Shadow', ...shadowPresetEntries()),
    submenu('Reflection', ...reflectionPresetEntries()),
  ];
}

/** One PowerPoint style in one section. */
function powerpointStyle(category: string, label: string, tint: ThemeColorSlot, picture: TableStylePicture): TableStyleSpec {
  return { value: styleValue(label), label, category, tint, picture };
}

/** A row of seven: one style with no accent (its own name), then one family in each accent. */
function powerpointRow(
  category: string,
  lead: { readonly label: string; readonly picture: TableStylePicture },
  family: { readonly label: string; readonly picture: TableStylePicture },
): TableStyleSpec[] {
  return [
    powerpointStyle(category, lead.label, 'text1', lead.picture),
    ...accents.map((accent) =>
      powerpointStyle(category, `${family.label} - ${accent.suffix}`, accent.slot, family.picture),
    ),
  ];
}

/** A family drawn without an accent under its own name, then in the six accents. */
const powerpointFamily = (category: string, family: { readonly label: string; readonly picture: TableStylePicture }) =>
  powerpointRow(category, family, family);

/**
 * **PowerPoint's four families of Light styles and Medium styles, and Dark Style 1.** `GUESS:` each look, from memory
 * of the gallery's thumbnails; the names are PowerPoint's built-in style names.
 */
const powerpointLight = {
  one: { label: 'Light Style 1', picture: look({ header: 'ruled', banded: true }) },
  two: { label: 'Light Style 2', picture: look({ header: 'filled', lines: 'outline', lineStrength: 'full' }) },
  three: { label: 'Light Style 3', picture: look({ header: 'ruled', lines: 'grid', banded: true, tintedText: true }) },
} as const;

const powerpointMedium = {
  one: { label: 'Medium Style 1', picture: look({ header: 'filled', lines: 'rows', lineStrength: 'full', banded: true }) },
  two: { label: 'Medium Style 2', picture: look({ header: 'filled', lines: 'grid', bodyFilled: true, banded: true }) },
  three: { label: 'Medium Style 3', picture: look({ header: 'filled', lines: 'outline', lineStrength: 'full', banded: true }) },
  four: { label: 'Medium Style 4', picture: look({ header: 'ruled', lines: 'grid', lineStrength: 'full', bodyFilled: true, banded: true }) },
} as const;

const powerpointDarkOne = {
  label: 'Dark Style 1',
  picture: look({ header: 'filled', bodyFilled: true, banded: true, firstColumn: 'filled' }),
};

const powerpointDarkTwo = look({ header: 'filled', bodyFilled: true, banded: true });

/**
 * **PowerPoint's whole built-in Table Styles gallery**: 74 styles, in the brief's four sections.
 *
 * - *Best Match for Document* (14): No Style, No Grid and the six *Themed Style 1*; No Style, Table Grid and the six
 *   *Themed Style 2*. These are the styles drawn from the document's theme alone.
 * - *Light* (21): Light Style 1, 2 and 3, each without an accent and in the six accents.
 * - *Medium* (28): Medium Style 1 to 4, the same way.
 * - *Dark* (11): Dark Style 1 the same way, then Dark Style 2 and its three accent pairs.
 *
 * ⚠ **`GUESS:` the Best Match reading.** Office may repeat styles under Best Match for Document; a gallery that listed
 * one value twice would select two cells for one style, so no style is listed twice and the sections add to 74. The
 * order inside each section is Office's, one family to a row of seven. **A Dark Style 2 pair is drawn in its first
 * accent**, because a `TableStylePicture` carries one tint.
 */
export const powerpointTableStyles: readonly TableStyleSpec[] = [
  ...powerpointRow(
    'Best Match for Document',
    { label: 'No Style, No Grid', picture: look({}) },
    { label: 'Themed Style 1', picture: look({ header: 'filled', lines: 'grid', lineStrength: 'full', banded: true }) },
  ),
  ...powerpointRow(
    'Best Match for Document',
    { label: 'No Style, Table Grid', picture: look({ lines: 'grid', lineStrength: 'full' }) },
    { label: 'Themed Style 2', picture: look({ header: 'filled', lines: 'outline', lineStrength: 'full', bodyFilled: true }) },
  ),
  ...powerpointFamily('Light', powerpointLight.one),
  ...powerpointFamily('Light', powerpointLight.two),
  ...powerpointFamily('Light', powerpointLight.three),
  ...powerpointFamily('Medium', powerpointMedium.one),
  ...powerpointFamily('Medium', powerpointMedium.two),
  ...powerpointFamily('Medium', powerpointMedium.three),
  ...powerpointFamily('Medium', powerpointMedium.four),
  ...powerpointFamily('Dark', powerpointDarkOne),
  powerpointStyle('Dark', 'Dark Style 2', 'text1', powerpointDarkTwo),
  powerpointStyle('Dark', 'Dark Style 2 - Accent 1/Accent 2', 'accent1', powerpointDarkTwo),
  powerpointStyle('Dark', 'Dark Style 2 - Accent 3/Accent 4', 'accent3', powerpointDarkTwo),
  powerpointStyle('Dark', 'Dark Style 2 - Accent 5/Accent 6', 'accent5', powerpointDarkTwo),
];

/**
 * PowerPoint's Table Styles gallery items, in the document's palette. A host starts the gallery on
 * `medium-style-2-accent-1`, the style PowerPoint inserts a table in.
 */
export function powerpointTableStyleGalleryItems(palette: ThemeColorPalette): TemplateResult[] {
  return tableStyleGalleryItems(powerpointTableStyles, palette);
}

/** **The command under PowerPoint's expanded Table Styles gallery**: Clear Table. `slot="footer"`. `GUESS:`. */
export function powerpointTableStyleGalleryFooter(): TemplateResult[] {
  return [html`<mjx-button slot="footer" label="Clear Table" size="small"></mjx-button>`];
}

// ── PowerPoint's Table Layout: Text Direction, Cell Margins and the four measures ──

/** One entry of a set whose current member is checked, with an optional second line. */
function choice(label: string, checked = false, description?: string): TemplateResult {
  return html`<mjx-menu-item
    kind="radio"
    label=${label}
    description=${description ?? nothing}
    ?checked=${checked}
  ></mjx-menu-item>`;
}

/**
 * **Text Direction's list, PowerPoint's**: Horizontal, Rotate all text 90°, Rotate all text 270° and Stacked, one of
 * which the selected cells are in, then More Options…, which opens the Format Shape pane's text box options in
 * Office and nothing here. **Horizontal is checked**: an inserted table's cells write no `vert`. Each direction is its
 * `ST_TextVerticalType` token's meaning (`horz`, `vert`, `vert270`, `wordArtVert`).
 *
 * `GUESS:` the four names, the order, the start and the options entry's name.
 */
export function powerpointTextDirectionEntries(): TemplateResult[] {
  return [
    choice('Horizontal', true),
    choice('Rotate all text 90°'),
    choice('Rotate all text 270°'),
    choice('Stacked'),
    separator(),
    item('More Options…'),
  ];
}

/**
 * **Cell Margins' list, PowerPoint's**: Normal, None, Narrow and Wide, each with its four margins as its second line,
 * then Custom Margins…, which opens the Cell Text Layout dialog in Office and nothing here. **Normal is checked**: an
 * inserted table's cells write no `marL`, `marR`, `marT` or `marB`, and PowerPoint's defaults are 0.25 cm left and
 * right and 0.13 cm top and bottom (91440 and 45720 EMU).
 *
 * Office draws the four presets as pictures with their measures beside them; the measures are the description here.
 * `GUESS:` the four presets' measures, the order, and that the list ticks the current preset.
 */
export function powerpointCellMarginEntries(): TemplateResult[] {
  return [
    choice('Normal', true, 'Top: 0.13 cm, Bottom: 0.13 cm, Left: 0.25 cm, Right: 0.25 cm'),
    choice('None', false, 'Top: 0 cm, Bottom: 0 cm, Left: 0 cm, Right: 0 cm'),
    choice('Narrow', false, 'Top: 0.13 cm, Bottom: 0.13 cm, Left: 0.13 cm, Right: 0.13 cm'),
    choice('Wide', false, 'Top: 0.38 cm, Bottom: 0.38 cm, Left: 0.38 cm, Right: 0.38 cm'),
    separator(),
    item('Custom Margins…'),
  ];
}

/**
 * **The four measures a PowerPoint host starts Table Layout's fields on**, in centimetres: a five-column, two-row
 * table inserted into the Office theme's widescreen content placeholder, 29.21 cm wide, each row 1.02 cm high (an
 * 18 pt line and the default top and bottom margins). The cell pair shows the selected cell, the table pair the whole
 * table. Written once, so the two pairs cannot disagree about the table they describe.
 *
 * `GUESS:` every number, and the 0.01 cm step.
 */
export const powerpointTableLayoutMeasures = {
  cellHeight: '1.02',
  cellWidth: '5.84',
  tableHeight: '2.04',
  tableWidth: '29.21',
  step: '0.01',
} as const;

// ── Excel's Table Design: the 61 styles, Table Name, Export and Refresh ─────

/**
 * **The name Excel's Table Name field starts on, and the one option it lists**: *Table1*, the name Excel gives a
 * workbook's first table (`<table name="Table1" displayName="Table1">`). Written once so both hosts' fields agree.
 * `GUESS:` that the field lists the name itself, since Office's plain text box lists nothing; see the census.
 */
export const excelTableName = 'Table1';

/** One Excel style: its display name, its wire name (`<tableStyleInfo name>`) as the value, and its look. */
function excelStyle(
  category: 'Light' | 'Medium' | 'Dark',
  number: number,
  tint: ThemeColorSlot,
  picture: TableStylePicture,
): TableStyleSpec {
  return {
    value: `TableStyle${category}${String(number)}`,
    label: `Table Style ${category} ${String(number)}`,
    category,
    tint,
    picture,
  };
}

/** One family of seven, numbered from `first`: no accent, then the six accents, one gallery row. */
function excelFamily(category: 'Light' | 'Medium' | 'Dark', first: number, picture: TableStylePicture): TableStyleSpec[] {
  return [
    excelStyle(category, first, 'text1', picture),
    ...accents.map((accent, index) => excelStyle(category, first + index + 1, accent.slot, picture)),
  ];
}

/**
 * **Excel's families of seven**, and the one family of four pairs. `GUESS:` each look, from memory of the gallery's
 * thumbnails.
 *
 * - Light 1–7 rule the header and shade alternate rows; Light 8–14 fill the header and outline the table; Light 15–21
 *   draw every line.
 * - Medium 1–7 fill the header over lines between rows; Medium 8–14 shade every cell; Medium 15–21 fill the header,
 *   rule the rows and set the first column apart; Medium 22–28 shade every cell and draw every line under a plain
 *   header.
 * - Dark 1–7 shade every cell and fill the header and first column; Dark 8–11 shade every cell under a filled header.
 */
const excelLooks = {
  light1: look({ header: 'ruled', banded: true }),
  light8: look({ header: 'filled', lines: 'outline', lineStrength: 'full' }),
  light15: look({ header: 'ruled', lines: 'grid', lineStrength: 'full', banded: true }),
  medium1: look({ header: 'filled', lines: 'rows', banded: true }),
  medium8: look({ header: 'filled', bodyFilled: true, banded: true }),
  medium15: look({ header: 'filled', lines: 'rows', lineStrength: 'full', banded: true, firstColumn: 'emphasised' }),
  medium22: look({ lines: 'grid', bodyFilled: true, banded: true }),
  dark1: look({ header: 'filled', bodyFilled: true, banded: true, firstColumn: 'filled' }),
  dark8: look({ header: 'filled', bodyFilled: true, banded: true, lines: 'outline', lineStrength: 'full' }),
} as const;

/**
 * **Excel's whole built-in Table Styles gallery**: None and 60 styles, 61 cells.
 *
 * - *Light* (22): **None**, the table with no style, then Table Style Light 1 to 21, three families of seven.
 * - *Medium* (28): Table Style Medium 1 to 28, four families of seven.
 * - *Dark* (11): Table Style Dark 1 to 7, one family of seven, then Dark 8 (no accent) and Dark 9, 10 and 11, the
 *   pairs Accent 1/2, 3/4 and 5/6, each drawn in its first accent because a `TableStylePicture` carries one tint.
 *
 * Each `value` is the style's wire name, `TableStyleMedium2`, so a later unit that dispatches one needs no second
 * table; None's is `none`, because a table with no style writes no `<tableStyleInfo name>`. Each label is Office's
 * display name. ⚠ **The brief lists 60**: None is Office's first Light cell, kept here. `GUESS:` None's place, every
 * picture and the order inside each section. A host starts the gallery on `TableStyleMedium2`, Format as Table's
 * default.
 */
export const excelTableStyles: readonly TableStyleSpec[] = [
  { value: 'none', label: 'None', category: 'Light', tint: 'text1', picture: look({}) },
  ...excelFamily('Light', 1, excelLooks.light1),
  ...excelFamily('Light', 8, excelLooks.light8),
  ...excelFamily('Light', 15, excelLooks.light15),
  ...excelFamily('Medium', 1, excelLooks.medium1),
  ...excelFamily('Medium', 8, excelLooks.medium8),
  ...excelFamily('Medium', 15, excelLooks.medium15),
  ...excelFamily('Medium', 22, excelLooks.medium22),
  ...excelFamily('Dark', 1, excelLooks.dark1),
  excelStyle('Dark', 8, 'text1', excelLooks.dark8),
  excelStyle('Dark', 9, 'accent1', excelLooks.dark8),
  excelStyle('Dark', 10, 'accent3', excelLooks.dark8),
  excelStyle('Dark', 11, 'accent5', excelLooks.dark8),
];

/** Excel's Table Styles gallery items, in the document's palette. A host starts the gallery on `TableStyleMedium2`. */
export function excelTableStyleGalleryItems(palette: ThemeColorPalette): TemplateResult[] {
  return tableStyleGalleryItems(excelTableStyles, palette);
}

/**
 * **The two commands under Excel's expanded Table Styles gallery**: New Table Style… and Clear, in Office's order.
 * `slot="footer"`. `GUESS:` the order. Home's Format as Table offers New PivotTable Style… in Clear's place; a table
 * that already exists can be cleared, and a PivotTable style is not this tab's.
 */
export function excelTableStyleGalleryFooter(): TemplateResult[] {
  return ['New Table Style…', 'Clear'].map(
    (label) => html`<mjx-button slot="footer" label=${label} size="small"></mjx-button>`,
  );
}

/**
 * **Export's list, Excel's**: Export Table to SharePoint List…, which opens the Export Table to SharePoint List wizard,
 * and Export Table to Visio Pivot Diagram…, which Office lists where Visio is installed. `GUESS:` that both are listed,
 * and the second's name.
 */
function excelTableExportEntries(): TemplateResult[] {
  return [item('Export Table to SharePoint List…'), item('Export Table to Visio Pivot Diagram…')];
}

/**
 * **Refresh's arrow, Excel's Table Design**: Refresh and Refresh All with Data's shortcuts, Refresh Status, Cancel
 * Refresh, then Connection Properties…. Data's Refresh All lists the same five with Refresh All first, because
 * there Refresh All is the face; here the face refreshes this table. `GUESS:` the order and the separator.
 */
function excelTableRefreshEntries(): TemplateResult[] {
  return [
    item('Refresh', 'Alt+F5'),
    item('Refresh All', 'Ctrl+Alt+F5'),
    item('Refresh Status'),
    item('Cancel Refresh'),
    separator(),
    item('Connection Properties…'),
  ];
}

// ── what a host renders ──────────────────────────────────────────────────────

/**
 * Word's five menus: Border Styles and Borders on Table Design; Select, Delete and AutoFit on Table Layout. Every other
 * Table Tools command is a field, a picker, a box, a toggle or a plain button.
 */
function wordTableToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.table-design.borders.border-styles', 'Border Styles', ...wordBorderStyleEntries())}
    ${commandMenu(host, 'word.table-design.borders.borders', 'Borders', ...wordBorderEntries())}
    ${commandMenu(host, 'word.table-layout.table.select', 'Select', ...tableSelectEntries('word'))}
    ${commandMenu(host, 'word.table-layout.rows-and-columns.delete', 'Delete', ...tableDeleteEntries('word'))}
    ${commandMenu(host, 'word.table-layout.cell-size.autofit', 'AutoFit', ...wordAutoFitEntries())}
  `;
}

/**
 * PowerPoint's ten menus. **Table Design's three**: Borders and Effects in Table Styles, and Text Effects in WordArt
 * Styles, whose entries are `stories/ribbons/wordart-styles-menus.ts`'s. **Table Layout's seven**: Select and Delete,
 * the lists shared with Word; Text Direction and Cell Margins, PowerPoint's own; and Arrange's Bring Forward, Send
 * Backward and Align, whose entries are `stories/ribbons/design-layout-menus.ts`' Arrange lists. Every other Table Tools
 * command is a field, a picker, a gallery, a box, a toggle or a plain button.
 */
function powerpointTableToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.table-design.table-styles.borders', 'Borders', ...powerpointBorderEntries())}
    ${commandMenu(host, 'powerpoint.table-design.table-styles.effects', 'Effects', ...powerpointTableEffectsEntries())}
    ${commandMenu(host, 'powerpoint.table-design.wordart-styles.text-effects', 'Text Effects', ...wordArtTextEffectsEntries())}
    ${commandMenu(host, 'powerpoint.table-layout.table.select', 'Select', ...tableSelectEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.table-layout.rows-and-columns.delete', 'Delete', ...tableDeleteEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.table-layout.alignment.text-direction', 'Text Direction', ...powerpointTextDirectionEntries())}
    ${commandMenu(host, 'powerpoint.table-layout.alignment.cell-margins', 'Cell Margins', ...powerpointCellMarginEntries())}
    ${commandMenu(host, 'powerpoint.table-layout.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.table-layout.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.table-layout.arrange.align', 'Align', ...alignEntries('powerpoint'))}
  `;
}

/**
 * Excel's two menus, both on Table Design's External Table Data: Export and Refresh. Every other Table Design command
 * is a field, a checkbox, a gallery or a plain button.
 */
function excelTableToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.table-design.external-table-data.export', 'Export', ...excelTableExportEntries())}
    ${commandMenu(host, 'excel.table-design.external-table-data.refresh', 'Refresh', ...excelTableRefreshEntries())}
  `;
}

/**
 * Every menu one application's Table Tools tabs open, with ids for one host's page. Word's and PowerPoint's Table
 * Design and Table Layout and Excel's Table Design are authored.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `masterViewMenus` is, by every host that draws
 * the Table Tools set: both Word hosts, both Excel hosts, and `Ribbons/PowerPoint`. **`Shell/PowerPoint` does not**: it
 * draws Picture Tools, so it renders none of these, and `tests/ribbons.test.ts` requires PowerPoint's menus of
 * `Ribbons/PowerPoint` alone.
 */
export function tableToolsMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  if (application === 'word') return wordTableToolsMenus(host);
  if (application === 'powerpoint') return powerpointTableToolsMenus(host);
  return excelTableToolsMenus(host);
}
