/**
 * **The menus and gallery items the Design and Layout tabs open**, written once and rendered by both
 * hosts.
 *
 * The ribbon programme's unit 5: Word's Design and Layout, PowerPoint's Design, Excel's Page Layout. The
 * pattern is `stories/ribbons/insert-menus.ts`'s, for its reasons. A binding lives in its host. The menu
 * it opens is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`.
 * A host renders `designLayoutMenus(application, host)` once beside its ribbon. Every
 * `commandMenu(host, '…'` call below spells its command id literally, so `tests/ribbons.test.ts` can
 * read it.
 *
 * ## Two things here are not menus
 *
 * 1. **Gallery items.** Word's Style Set and PowerPoint's Themes and Variants are in-ribbon galleries,
 *    so the hosts bind `<mjx-gallery>` and fill it from `styleSetGalleryItems()`, `themeGalleryItems()`
 *    and `variantGalleryItems()`. An item's art must be static markup with no bindings inside it
 *    (`<mjx-gallery-item>` captures its children, as `stories/gallery/specimens.ts` records), so each
 *    picture is one of a few fixed templates, chosen by index.
 * 2. **The entries of the four menus at the foot of PowerPoint's Variants gallery**: Colours, Fonts,
 *    Effects and Background Styles. They are not census commands, so there is no `commandMenu` for
 *    them. Each PowerPoint host writes the four `<mjx-menu>` wrappers with literal ids beside its paste
 *    menu, and fills them from the exported entries functions below. The entries are still written
 *    once.
 *
 * ## Shared entries
 *
 * Themes, Colours, Fonts and Effects open the same lists in Word's Design, Excel's Page Layout and
 * PowerPoint's Variants footer. Margins, Orientation, Size and Breaks are close in Word and Excel, not
 * identical: Word's Margins has Moderate and Mirrored, and Excel's Breaks inserts a page break at the
 * cell rather than offering section breaks. So each is one function, taking the application where the
 * two differ. Arrange's five menus are one function each, for the same reason `arrangeCommands` is.
 *
 * ## Shallow, and real
 *
 * Decision 3 of the approved plan, as on Insert: a handful of Office's own entries under each command,
 * by Office's own names, and one entry that reaches the rest (*Custom Margins…*, *More Paper Sizes…*).
 * The current choice is checked, at Office's defaults for a new document in a British English build:
 * Office Theme, Normal margins, Portrait, A4, one column.
 *
 * **Nothing here dispatches a command.** A menu opens and an entry can be chosen; no document changes,
 * because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import { commandMenu } from './ribbon-parts.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string): TemplateResult {
  return html`<mjx-menu-item label=${label}></mjx-menu-item>`;
}

/** One entry of a set whose current member is checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
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

// ── the theme menus: Word's Design, Excel's Page Layout, PowerPoint's Variants footer ─────────────

/**
 * Themes, in Word and Excel: a few of Office's built-in themes, then the three commands under them.
 *
 * *Reset to Theme from Template* is Word's alone, because a workbook has no attached template to reset
 * to.
 */
function themeEntries(application: 'word' | 'excel'): TemplateResult[] {
  return [
    section(
      'Office',
      choice('Office Theme', true),
      choice('Facet'),
      choice('Integral'),
      choice('Ion'),
      choice('Retrospect'),
      choice('Slice'),
    ),
    separator(),
    ...(application === 'word' ? [item('Reset to Theme from Template')] : []),
    item('Browse for Themes…'),
    item('Save Current Theme…'),
  ];
}

/** Colours: Office's theme colour sets, by their own names, and the dialog. */
export function themeColourEntries(): TemplateResult[] {
  return [
    section(
      'Office',
      choice('Office', true),
      choice('Office 2007 – 2010'),
      choice('Greyscale'),
      choice('Blue Warm'),
      choice('Blue'),
      choice('Green'),
      choice('Orange'),
      choice('Red'),
      choice('Violet'),
    ),
    separator(),
    item('Customise Colours…'),
  ];
}

/** Fonts: Office's theme font pairs, heading font then body font, and the dialog. */
export function themeFontEntries(): TemplateResult[] {
  return [
    section(
      'Office',
      choice('Office: Aptos Display, Aptos', true),
      choice('Office 2007 – 2010: Cambria, Calibri'),
      choice('Calibri: Calibri Light, Calibri'),
      choice('Arial'),
      choice('Corbel'),
      choice('Candara'),
      choice('Franklin Gothic: Franklin Gothic Medium, Franklin Gothic Book'),
    ),
    separator(),
    item('Customise Fonts…'),
  ];
}

/** Effects: Office's theme effect sets, by their own names. Office puts nothing under them. */
export function themeEffectEntries(): TemplateResult[] {
  return [
    section(
      'Office',
      choice('Office', true),
      choice('Office 2007 – 2010'),
      choice('Subtle Solids'),
      choice('Banded Edge'),
      choice('Smokey Glass'),
      choice('Glow Edge'),
      choice('Frosted Glass'),
    ),
  ];
}

/** Background Styles, at the foot of PowerPoint's Variants: the first styles, the pane, and the reset. */
export function backgroundStyleEntries(): TemplateResult[] {
  return [
    section('Background Styles', choice('Style 1', true), choice('Style 2'), choice('Style 3'), choice('Style 4')),
    separator(),
    item('Format Background…'),
    item('Reset Slide Background'),
  ];
}

// ── Word's Design ─────────────────────────────────────────────────────────────

/** Paragraph Spacing: the style set's own spacing, the built-in presets, and the dialog. */
function paragraphSpacingEntries(): TemplateResult[] {
  return [
    section('Style Set', choice('Default', true)),
    section(
      'Built-In',
      choice('No Paragraph Space'),
      choice('Compact'),
      choice('Tight'),
      choice('Open'),
      choice('Relaxed'),
      choice('Double'),
    ),
    separator(),
    item('Custom Paragraph Spacing…'),
  ];
}

/** Watermark: a few of Office's built-in watermarks under its own headings, then the three commands. */
function watermarkEntries(): TemplateResult[] {
  return [
    section('Confidential', item('CONFIDENTIAL 1'), item('DO NOT COPY 1')),
    section('Disclaimers', item('DRAFT 1'), item('SAMPLE 1')),
    section('Urgent', item('ASAP 1'), item('URGENT 1')),
    separator(),
    item('Custom Watermark…'),
    item('Remove Watermark'),
    item('Save Selection to Watermark Gallery…'),
  ];
}

// ── page setup: Word's Layout and Excel's Page Layout ───────────────────────

/** Margins: Office's presets by name (Word has five, Excel three), and the dialog. */
function marginEntries(application: 'word' | 'excel'): TemplateResult[] {
  const presets =
    application === 'word'
      ? [choice('Normal', true), choice('Narrow'), choice('Moderate'), choice('Wide'), choice('Mirrored')]
      : [choice('Normal', true), choice('Wide'), choice('Narrow')];
  return [...presets, separator(), item('Custom Margins…')];
}

/** Orientation: the two, and portrait is a new document's. */
function orientationEntries(): TemplateResult[] {
  return [choice('Portrait', true), choice('Landscape')];
}

/** Size: a few paper sizes, A4 checked as a British build's default, and the dialog. */
function sizeEntries(): TemplateResult[] {
  return [
    choice('A4', true),
    choice('A5'),
    choice('Letter'),
    choice('Legal'),
    choice('Executive'),
    separator(),
    item('More Paper Sizes…'),
  ];
}

/** Columns, in Word: Office's five presets and the dialog. */
function columnsEntries(): TemplateResult[] {
  return [
    choice('One', true),
    choice('Two'),
    choice('Three'),
    choice('Left'),
    choice('Right'),
    separator(),
    item('More Columns…'),
  ];
}

/** Breaks, in Word: page breaks and section breaks, under Office's own two headings. */
function wordBreaksEntries(): TemplateResult[] {
  return [
    section('Page Breaks', item('Page'), item('Column'), item('Text Wrapping')),
    section('Section Breaks', item('Next Page'), item('Continuous'), item('Even Page'), item('Odd Page')),
  ];
}

/** Breaks, in Excel: a page break at the active cell, and the two ways to take them out. */
function excelBreaksEntries(): TemplateResult[] {
  return [item('Insert Page Break'), item('Remove Page Break'), item('Reset All Page Breaks')];
}

/** Line Numbers, in Word: the four numbering modes, the per-paragraph exception, and the dialog. */
function lineNumbersEntries(): TemplateResult[] {
  return [
    choice('None', true),
    choice('Continuous'),
    choice('Restart Each Page'),
    choice('Restart Each Section'),
    option('Suppress for Current Paragraph'),
    separator(),
    item('Line Numbering Options…'),
  ];
}

/** Hyphenation, in Word: off, automatic, by hand, and the dialog. */
function hyphenationEntries(): TemplateResult[] {
  return [choice('None', true), choice('Automatic'), item('Manual'), separator(), item('Hyphenation Options…')];
}

/** Print Area, in Excel. */
function printAreaEntries(): TemplateResult[] {
  return [item('Set Print Area'), item('Clear Print Area')];
}

// ── Arrange: Word's Layout and Excel's Page Layout ──────────────────────────

/** Position, in Word: in line, three of Office's nine wrapped positions, and the dialog. */
function positionEntries(): TemplateResult[] {
  return [
    section('In Line with Text', item('In Line with Text')),
    section(
      'With Text Wrapping',
      item('Position in Top Left with Square Text Wrapping'),
      item('Position in Middle Centre with Square Text Wrapping'),
      item('Position in Bottom Right with Square Text Wrapping'),
    ),
    separator(),
    item('More Layout Options…'),
  ];
}

/** Wrap Text, in Word: the seven wraps, then the three commands under them. */
function wrapTextEntries(): TemplateResult[] {
  return [
    choice('In Line with Text', true),
    choice('Square'),
    choice('Tight'),
    choice('Through'),
    choice('Top and Bottom'),
    choice('Behind Text'),
    choice('In Front of Text'),
    separator(),
    item('Edit Wrap Points'),
    item('More Layout Options…'),
    item('Set as Default Layout'),
  ];
}

/** Bring Forward's arrow. Word adds the layer a picture can have that a sheet's shapes cannot: text. */
function bringForwardEntries(application: 'word' | 'excel'): TemplateResult[] {
  return [
    item('Bring Forward'),
    item('Bring to Front'),
    ...(application === 'word' ? [item('Bring in Front of Text')] : []),
  ];
}

/** Send Backward's arrow, mirroring Bring Forward's. */
function sendBackwardEntries(application: 'word' | 'excel'): TemplateResult[] {
  return [
    item('Send Backward'),
    item('Send to Back'),
    ...(application === 'word' ? [item('Send Behind Text')] : []),
  ];
}

/**
 * Align: the six alignments and the two distributions, then what each application aligns against.
 *
 * Word aligns to the page or the margin; Excel snaps to the grid or to other shapes. Both offer the
 * gridlines.
 */
function alignEntries(application: 'word' | 'excel'): TemplateResult[] {
  const against =
    application === 'word'
      ? [choice('Align to Page'), choice('Align to Margin', true), separator(), option('Use Alignment Guides', true), option('View Gridlines')]
      : [option('Snap to Grid'), option('Snap to Shape'), option('View Gridlines', true)];
  return [
    section(
      'Align',
      item('Align Left'),
      item('Align Centre'),
      item('Align Right'),
      item('Align Top'),
      item('Align Middle'),
      item('Align Bottom'),
    ),
    section('Distribute', item('Distribute Horizontally'), item('Distribute Vertically')),
    separator(),
    ...against,
  ];
}

/** Group: the three commands, in Office's order. */
function groupEntries(): TemplateResult[] {
  return [item('Group'), item('Regroup'), item('Ungroup')];
}

/** Rotate: the two quarter turns, the two flips, and the dialog. */
function rotateEntries(): TemplateResult[] {
  return [
    item('Rotate Right 90°'),
    item('Rotate Left 90°'),
    item('Flip Vertical'),
    item('Flip Horizontal'),
    separator(),
    item('More Rotation Options…'),
  ];
}

// ── PowerPoint's Customise ───────────────────────────────────────────────────

/** Slide Size: the two built-in shapes, widescreen being a new deck's, and the dialog. */
function slideSizeEntries(): TemplateResult[] {
  return [choice('Standard (4:3)'), choice('Widescreen (16:9)', true), separator(), item('Custom Slide Size…')];
}

// ── the gallery items ────────────────────────────────────────────────────────

/**
 * A style set's picture: a title, a heading and body lines, drawn in that set's weight.
 *
 * Static templates, chosen by index, because an item's art may carry no bindings. Every colour is a
 * token, and every size is relative to the cell, so the pictures move with the theme and the density.
 */
const styleSetPictures: readonly TemplateResult[] = [
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);text-align:start;font-size:0.6em;line-height:1.1">
    <span style="font-size:1.6em;font-weight:300">Title</span>
    <span style="font-weight:600;color:var(--theme-accent)">Heading 1</span>
    <span style="color:var(--theme-text-secondary)">Body text runs on.</span>
  </span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);text-align:start;font-size:0.6em;line-height:1.1">
    <span style="font-size:1.6em;font-weight:700">Title</span>
    <span style="font-weight:700">Heading 1</span>
    <span style="color:var(--theme-text-secondary)">Body text runs on.</span>
  </span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);text-align:start;font-size:0.6em;line-height:1.1">
    <span style="font-size:1.5em;font-family:var(--font-serif);font-style:italic">Title</span>
    <span style="font-family:var(--font-serif);font-weight:600">Heading 1</span>
    <span style="color:var(--theme-text-secondary)">Body text runs on.</span>
  </span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);text-align:center;font-size:0.6em;line-height:1.1">
    <span style="font-size:1.5em;font-weight:600">Title</span>
    <span style="font-weight:600">Heading 1</span>
    <span style="color:var(--theme-text-secondary)">Body text runs on.</span>
  </span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);text-align:start;font-size:0.6em;line-height:1.1">
    <span style="font-size:1.5em;font-weight:600;border-block-end:1px solid var(--theme-accent)">Title</span>
    <span style="font-weight:600;border-block-end:1px solid var(--theme-border)">Heading 1</span>
    <span style="color:var(--theme-text-secondary)">Body text runs on.</span>
  </span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);text-align:start;font-size:0.6em;line-height:1.1">
    <span style="font-size:1.5em;font-weight:600;background:var(--theme-accent-surface)">Title</span>
    <span style="font-weight:600;background:var(--theme-accent-surface)">Heading 1</span>
    <span style="color:var(--theme-text-secondary)">Body text runs on.</span>
  </span>`,
];

/** Word's style sets, by Office's names. The document's own set is first, under Office's heading. */
const styleSets: readonly { readonly value: string; readonly label: string; readonly category: string; readonly picture: number }[] = [
  { value: 'this-document', label: 'This Document', category: 'This Document', picture: 0 },
  { value: 'basic-elegant', label: 'Basic (Elegant)', category: 'Built-In', picture: 2 },
  { value: 'basic-simple', label: 'Basic (Simple)', category: 'Built-In', picture: 1 },
  { value: 'casual', label: 'Casual', category: 'Built-In', picture: 3 },
  { value: 'lines-distinctive', label: 'Lines (Distinctive)', category: 'Built-In', picture: 4 },
  { value: 'lines-simple', label: 'Lines (Simple)', category: 'Built-In', picture: 4 },
  { value: 'minimalist', label: 'Minimalist', category: 'Built-In', picture: 0 },
  { value: 'shaded', label: 'Shaded', category: 'Built-In', picture: 5 },
  { value: 'word-2013', label: 'Word 2013', category: 'Built-In', picture: 1 },
];

/** The Style Set gallery's items, for Word's Design tab. The value a host starts on is `this-document`. */
export function styleSetGalleryItems(): TemplateResult[] {
  return styleSets.map(
    (set) => html`<mjx-gallery-item value=${set.value} label=${set.label} category=${set.category}
      >${styleSetPictures[set.picture] ?? styleSetPictures[0]}</mjx-gallery-item
    >`,
  );
}

/**
 * A theme's picture: its first four accent colours in a row under a letter.
 *
 * Every colour is a palette token, so six themes share six arrangements of one palette. That is the
 * honest limit of a catalogue with no document: it has one palette to draw with, and a theme is a
 * palette.
 */
const themePictures: readonly TemplateResult[] = [
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);inline-size:100%;font-size:0.75em;font-weight:600">Aa
    <span style="display:grid;grid-template-columns:repeat(4,1fr);gap:1px;block-size:calc(var(--spacing) * 1.5)">
      <span style="background:var(--theme-accent)"></span><span style="background:var(--theme-secondary-accent)"></span><span style="background:var(--theme-accent-border)"></span><span style="background:var(--theme-text-secondary)"></span>
    </span></span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);inline-size:100%;font-size:0.75em;font-weight:300;font-family:var(--font-serif)">Aa
    <span style="display:grid;grid-template-columns:repeat(4,1fr);gap:1px;block-size:calc(var(--spacing) * 1.5)">
      <span style="background:var(--theme-secondary-accent)"></span><span style="background:var(--theme-accent)"></span><span style="background:var(--theme-border)"></span><span style="background:var(--theme-accent-border)"></span>
    </span></span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);inline-size:100%;font-size:0.75em;font-weight:700;text-transform:uppercase">Aa
    <span style="display:grid;grid-template-columns:repeat(4,1fr);gap:1px;block-size:calc(var(--spacing) * 1.5)">
      <span style="background:var(--theme-accent-border)"></span><span style="background:var(--theme-text-secondary)"></span><span style="background:var(--theme-accent)"></span><span style="background:var(--theme-secondary-accent)"></span>
    </span></span>`,
  html`<span style="display:grid;gap:calc(var(--spacing) * 0.5);inline-size:100%;font-size:0.75em;font-style:italic">Aa
    <span style="display:grid;grid-template-columns:repeat(4,1fr);gap:1px;block-size:calc(var(--spacing) * 1.5)">
      <span style="background:var(--theme-text-secondary)"></span><span style="background:var(--theme-accent-border)"></span><span style="background:var(--theme-secondary-accent)"></span><span style="background:var(--theme-accent)"></span>
    </span></span>`,
];

/** PowerPoint's themes, by Office's names. */
const themes: readonly { readonly value: string; readonly label: string; readonly picture: number }[] = [
  { value: 'office-theme', label: 'Office Theme', picture: 0 },
  { value: 'facet', label: 'Facet', picture: 1 },
  { value: 'integral', label: 'Integral', picture: 2 },
  { value: 'ion', label: 'Ion', picture: 3 },
  { value: 'ion-boardroom', label: 'Ion Boardroom', picture: 1 },
  { value: 'organic', label: 'Organic', picture: 2 },
  { value: 'retrospect', label: 'Retrospect', picture: 0 },
  { value: 'slice', label: 'Slice', picture: 3 },
  { value: 'wisp', label: 'Wisp', picture: 2 },
];

/** The Themes gallery's items, for PowerPoint's Design tab. A host starts on `office-theme`. */
export function themeGalleryItems(): TemplateResult[] {
  return themes.map(
    (theme, index) => html`<mjx-gallery-item
      value=${theme.value}
      label=${theme.label}
      category=${index === 0 ? 'This Presentation' : 'Office'}
      >${themePictures[theme.picture] ?? themePictures[0]}</mjx-gallery-item
    >`,
  );
}

/**
 * The Variants gallery's items: four variants of the current theme.
 *
 * `GUESS:` **the labels.** Office's tooltip on a variant is the theme's name, four times, which is four
 * identical accessible names in one listbox. *Office Theme, Variant 2* keeps the theme's name and tells
 * the four apart.
 */
export function variantGalleryItems(): TemplateResult[] {
  return [0, 1, 2, 3].map(
    (index) => html`<mjx-gallery-item
      value=${`variant-${String(index + 1)}`}
      label=${`Office Theme, Variant ${String(index + 1)}`}
      >${themePictures[index] ?? themePictures[0]}</mjx-gallery-item
    >`,
  );
}

// ── Word ─────────────────────────────────────────────────────────────────────

function wordDesignLayoutMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.design.style-set.themes', 'Themes', ...themeEntries('word'))}
    ${commandMenu(host, 'word.design.style-set.colours', 'Colours', ...themeColourEntries())}
    ${commandMenu(host, 'word.design.style-set.fonts', 'Fonts', ...themeFontEntries())}
    ${commandMenu(host, 'word.design.style-set.paragraph-spacing', 'Paragraph Spacing', ...paragraphSpacingEntries())}
    ${commandMenu(host, 'word.design.style-set.effects', 'Effects', ...themeEffectEntries())}
    ${commandMenu(host, 'word.design.page-background.watermark', 'Watermark', ...watermarkEntries())}
    ${commandMenu(host, 'word.layout.page-setup.margins', 'Margins', ...marginEntries('word'))}
    ${commandMenu(host, 'word.layout.page-setup.orientation', 'Orientation', ...orientationEntries())}
    ${commandMenu(host, 'word.layout.page-setup.size', 'Size', ...sizeEntries())}
    ${commandMenu(host, 'word.layout.page-setup.columns', 'Columns', ...columnsEntries())}
    ${commandMenu(host, 'word.layout.page-setup.breaks', 'Breaks', ...wordBreaksEntries())}
    ${commandMenu(host, 'word.layout.page-setup.line-numbers', 'Line Numbers', ...lineNumbersEntries())}
    ${commandMenu(host, 'word.layout.page-setup.hyphenation', 'Hyphenation', ...hyphenationEntries())}
    ${commandMenu(host, 'word.layout.arrange.position', 'Position', ...positionEntries())}
    ${commandMenu(host, 'word.layout.arrange.wrap-text', 'Wrap Text', ...wrapTextEntries())}
    ${commandMenu(host, 'word.layout.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('word'))}
    ${commandMenu(host, 'word.layout.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('word'))}
    ${commandMenu(host, 'word.layout.arrange.align', 'Align', ...alignEntries('word'))}
    ${commandMenu(host, 'word.layout.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'word.layout.arrange.rotate', 'Rotate', ...rotateEntries())}
  `;
}

// ── PowerPoint ───────────────────────────────────────────────────────────────

/**
 * One menu. The Themes and Variants galleries are bound by the hosts, and Variants' four footer menus
 * are written by each PowerPoint host from the exported entries above.
 */
function powerpointDesignLayoutMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.design.customise.slide-size', 'Slide Size', ...slideSizeEntries())}
  `;
}

// ── Excel ────────────────────────────────────────────────────────────────────

function excelDesignLayoutMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.page-layout.themes.themes', 'Themes', ...themeEntries('excel'))}
    ${commandMenu(host, 'excel.page-layout.themes.colours', 'Colours', ...themeColourEntries())}
    ${commandMenu(host, 'excel.page-layout.themes.fonts', 'Fonts', ...themeFontEntries())}
    ${commandMenu(host, 'excel.page-layout.themes.effects', 'Effects', ...themeEffectEntries())}
    ${commandMenu(host, 'excel.page-layout.page-setup.margins', 'Margins', ...marginEntries('excel'))}
    ${commandMenu(host, 'excel.page-layout.page-setup.orientation', 'Orientation', ...orientationEntries())}
    ${commandMenu(host, 'excel.page-layout.page-setup.size', 'Size', ...sizeEntries())}
    ${commandMenu(host, 'excel.page-layout.page-setup.print-area', 'Print Area', ...printAreaEntries())}
    ${commandMenu(host, 'excel.page-layout.page-setup.breaks', 'Breaks', ...excelBreaksEntries())}
    ${commandMenu(host, 'excel.page-layout.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('excel'))}
    ${commandMenu(host, 'excel.page-layout.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('excel'))}
    ${commandMenu(host, 'excel.page-layout.arrange.align', 'Align', ...alignEntries('excel'))}
    ${commandMenu(host, 'excel.page-layout.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'excel.page-layout.arrange.rotate', 'Rotate', ...rotateEntries())}
  `;
}

// ── what a host renders ──────────────────────────────────────────────────────

const menusByApplication: Readonly<Record<RibbonApplication, (host: RibbonSurfaceHost) => TemplateResult>> = {
  word: wordDesignLayoutMenus,
  powerpoint: powerpointDesignLayoutMenus,
  excel: excelDesignLayoutMenus,
};

/**
 * Every menu one application's Design or Layout tab opens, with ids for one host's page.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, exactly as `insertMenus` is.
 */
export function designLayoutMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  return menusByApplication[application](host);
}
