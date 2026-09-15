/**
 * **The gallery and the effect menus WordArt Styles opens**, written once for every tab that carries the group:
 * PowerPoint's Table Design and Shape Format today (Shape Format's Text Effects menu is rendered by
 * `stories/ribbons/drawing-tools-menus.ts`), and Word's and Excel's Shape Format and every Chart Format when their
 * units land. The commands are `wordArtStylesCommands(application, tab)` in `dev/ribbons/census.ts`.
 *
 * ## What is here, and what is not
 *
 * - **`wordArtStyleGalleryItems(palette)` and `wordArtStyleGalleryFooter()`**: the twenty WordArt styles, each drawn
 *   as a letter in the document's palette, and Clear WordArt.
 * - **`wordArtTextEffectsEntries()`**: Text Effects' six submenus, Shadow, Reflection, Glow, Bevel, 3-D Rotation and
 *   Transform, each with Office's whole preset list.
 * - **The preset lists themselves**, `shadowPresetEntries`, `reflectionPresetEntries` and `bevelPresetEntries`, which
 *   Table Design's Effects menu reuses for its Shadow, Reflection and Cell Bevel submenus, and, with
 *   `glowPresetEntries` and `rotationPresetEntries`, Picture Format's Picture Effects menu
 *   (`stories/ribbons/picture-tools-menus.ts`).
 * - **No menu.** A menu's id comes from its command id through `commandMenu`, and `tests/ribbons.test.ts` reads that
 *   id only where it is spelt literally, so each tab's own menus function (`tableToolsMenus` for Table Design) writes
 *   `commandMenu(host, '<its id>', …)` and fills it from here. A shared function that built the menu from a computed id
 *   would be a menu the gate cannot see.
 * - **No colour list.** Text Fill and Text Outline are colour pickers over the document's palette, which a host owns.
 *   The entries beneath their palettes are `stories/ribbons/colour-picker-entries.ts`'s `fillEntries` and
 *   `outlineEntries`.
 *
 * `GUESS:` every name below, from memory of Microsoft 365. The WordArt style names are the Office theme's, as
 * Insert's WordArt menu names them; Office names each from the document's own theme colours. Where a label differs
 * from Office's spelling the census's wins (*colour*, *Centre*).
 *
 * **Nothing here dispatches a command.** A menu opens, an entry can be chosen, a gallery previews and commits; no
 * text changes, because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';
import { html as staticHtml, unsafeStatic } from 'lit/static-html.js';

import type { ThemeColorPalette, ThemeColorSlot } from '../../src/pickers/picker-model.ts';
import { paletteSlotColour as slotColour, spacingStep as step } from './palette-art.ts';

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

/** An entry that opens a submenu of its own, as every Effects and Text Effects entry does in Office. */
export function submenu(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-item label=${label}>
    <mjx-menu slot="submenu" label=${label}>${entries}</mjx-menu>
  </mjx-menu-item>`;
}

// ── the preset lists Office repeats ──────────────────────────────────────────

/**
 * **Shadow's list**: No Shadow, then the nine outer offsets, the nine inner shadows and the five perspective shadows,
 * each a grid read across its rows, then Shadow Options…. Text Effects and Table Design's Effects both show it.
 */
export function shadowPresetEntries(): TemplateResult[] {
  const outer = ['Bottom Right', 'Bottom', 'Bottom Left', 'Right', 'Centre', 'Left', 'Top Right', 'Top', 'Top Left'];
  const inner = ['Top Left', 'Top', 'Top Right', 'Left', 'Centre', 'Right', 'Bottom Left', 'Bottom', 'Bottom Right'];
  const perspective = ['Upper Left', 'Upper Right', 'Below', 'Lower Left', 'Lower Right'];
  return [
    item('No Shadow'),
    section('Outer', ...outer.map((where) => item(`Offset: ${where}`))),
    section('Inner', ...inner.map((where) => item(`Inside: ${where}`))),
    section('Perspective', ...perspective.map((where) => item(`Perspective: ${where}`))),
    separator(),
    item('Shadow Options…'),
  ];
}

/**
 * **Reflection's list**: No Reflection, then nine variations, three sizes of reflection at three offsets, then
 * Reflection Options…. Text Effects and Table Design's Effects both show it.
 */
export function reflectionPresetEntries(): TemplateResult[] {
  const offsets = ['Touching', '4 pt offset', '8 pt offset'];
  const sizes = ['Tight', 'Half', 'Full'];
  return [
    item('No Reflection'),
    section(
      'Reflection Variations',
      ...offsets.flatMap((offset) => sizes.map((size) => item(`${size} Reflection: ${offset}`))),
    ),
    separator(),
    item('Reflection Options…'),
  ];
}

/**
 * **Bevel's list**: No Bevel, then the twelve bevels (`ST_BevelPresetType`'s twelve, by Office's names), then the
 * options entry when the menu has one. Text Effects' Bevel ends in 3-D Options…; Table Design's Cell Bevel, `GUESS:`,
 * ends in none.
 */
export function bevelPresetEntries(options?: string): TemplateResult[] {
  const bevels = [
    'Circle',
    'Relaxed Inset',
    'Cross',
    'Cool Slant',
    'Angle',
    'Soft Round',
    'Convex',
    'Slope',
    'Divot',
    'Riblet',
    'Hard Edge',
    'Art Deco',
  ];
  return [
    item('No Bevel'),
    section('Bevel', ...bevels.map((bevel) => item(bevel))),
    ...(options === undefined ? [] : [separator(), item(options)]),
  ];
}

/**
 * The six accents under the Office theme's names, as the WordArt and Glow names spell them. Exported for Picture
 * Format's Recolour list, which names its variations the same way.
 */
export const accentNames: readonly { readonly slot: ThemeColorSlot; readonly name: string }[] = [
  { slot: 'accent1', name: 'Blue, Accent colour 1' },
  { slot: 'accent2', name: 'Orange, Accent colour 2' },
  { slot: 'accent3', name: 'Grey, Accent colour 3' },
  { slot: 'accent4', name: 'Gold, Accent colour 4' },
  { slot: 'accent5', name: 'Blue, Accent colour 5' },
  { slot: 'accent6', name: 'Green, Accent colour 6' },
];

/**
 * **Glow's list**: No Glow, then twenty-four variations, four sizes down and the six accents across, then More Glow
 * Colours and Glow Options…. `GUESS:` that More Glow Colours is drawn as an entry: in Office it opens a colour grid,
 * which a menu cannot hold.
 */
export function glowPresetEntries(): TemplateResult[] {
  const sizes = ['5 point', '8 point', '11 point', '18 point'];
  return [
    item('No Glow'),
    section(
      'Glow Variations',
      ...sizes.flatMap((size) => accentNames.map((accent) => item(`Glow: ${size}; ${accent.name}`))),
    ),
    separator(),
    item('More Glow Colours'),
    item('Glow Options…'),
  ];
}

/**
 * **3-D Rotation's list**: No Rotation, then the parallel, perspective and oblique presets, then its options. Exported,
 * with Glow's, for Picture Format's Picture Effects menu.
 */
export function rotationPresetEntries(): TemplateResult[] {
  const parallel = [
    'Isometric: Left Down',
    'Isometric: Right Up',
    'Isometric: Top Up',
    'Isometric: Bottom Down',
    'Off Axis 1: Left',
    'Off Axis 1: Right',
    'Off Axis 1: Top',
    'Off Axis 2: Left',
    'Off Axis 2: Right',
    'Off Axis 2: Top',
  ];
  const perspective = [
    'Perspective: Front',
    'Perspective: Left',
    'Perspective: Right',
    'Perspective: Below',
    'Perspective: Above',
    'Perspective: Contrasting Left',
    'Perspective: Contrasting Right',
    'Perspective: Heroic Extreme Left',
    'Perspective: Heroic Extreme Right',
    'Perspective: Relaxed',
    'Perspective: Relaxed Moderately',
  ];
  const oblique = ['Oblique: Top Left', 'Oblique: Top Right', 'Oblique: Bottom Left', 'Oblique: Bottom Right'];
  return [
    item('No Rotation'),
    section('Parallel', ...parallel.map((preset) => item(preset))),
    section('Perspective', ...perspective.map((preset) => item(preset))),
    section('Oblique', ...oblique.map((preset) => item(preset))),
    separator(),
    item('3-D Rotation Options…'),
  ];
}

/** **Transform's list**: No Transform, then Follow Path's four and Warp's thirty-two, by Office's names. */
function transformPresetEntries(): TemplateResult[] {
  const followPath = ['Arch', 'Arch: Down', 'Circle', 'Button'];
  const warp = [
    'Square',
    'Stop',
    'Triangle: Up',
    'Triangle: Down',
    'Chevron: Up',
    'Chevron: Down',
    'Ring: Inside',
    'Ring: Outside',
    'Curve: Up',
    'Curve: Down',
    'Can: Up',
    'Can: Down',
    'Wave: Down',
    'Wave: Up',
    'Double Wave: Down-Up',
    'Double Wave: Up-Down',
    'Inflate',
    'Deflate',
    'Inflate: Bottom',
    'Deflate: Bottom',
    'Inflate: Top',
    'Deflate: Top',
    'Deflate-Inflate',
    'Deflate-Inflate-Deflate',
    'Fade: Right',
    'Fade: Left',
    'Fade: Up',
    'Fade: Down',
    'Slant: Up',
    'Slant: Down',
    'Cascade: Up',
    'Cascade: Down',
  ];
  return [
    item('No Transform'),
    section('Follow Path', ...followPath.map((preset) => item(preset))),
    section('Warp', ...warp.map((preset) => item(preset))),
  ];
}

/**
 * **Text Effects' menu**: Shadow, Reflection, Glow, Bevel, 3-D Rotation and Transform, each a submenu with Office's
 * whole list, in Office's order. Nothing is checked: the selected text's current effect is not a state this catalogue
 * holds.
 */
export function wordArtTextEffectsEntries(): TemplateResult[] {
  return [
    submenu('Shadow', ...shadowPresetEntries()),
    submenu('Reflection', ...reflectionPresetEntries()),
    submenu('Glow', ...glowPresetEntries()),
    submenu('Bevel', ...bevelPresetEntries('3-D Options…')),
    submenu('3-D Rotation', ...rotationPresetEntries()),
    submenu('Transform', ...transformPresetEntries()),
  ];
}

// ── the gallery: twenty WordArt styles, drawn in the document's colours ──────

/** What a WordArt style's picture shows: a letter's fill, its outline and its one effect. Never a render of `a:rPr`. */
export interface WordArtLook {
  /** A flat fill, a fill that darkens downwards, or a diagonal hatch of the fill over the paper. */
  readonly fill: 'solid' | 'gradient' | 'pattern';
  /** The document colour the fill is drawn in. */
  readonly fillSlot: ThemeColorSlot;
  /** The document colour the letter's outline is drawn in, or none. */
  readonly outlineSlot?: ThemeColorSlot;
  /** The one effect the style's name ends in. */
  readonly effect: 'none' | 'shadow' | 'hard-shadow' | 'glow' | 'reflection' | 'soft-bevel' | 'sharp-bevel';
  /** The document colour the effect is drawn in. Text 1 when the name gives none. */
  readonly effectSlot?: ThemeColorSlot;
}

/** One WordArt style: its value, Office's name for it, and its look. */
export interface WordArtStyleSpec {
  readonly value: string;
  readonly label: string;
  readonly look: WordArtLook;
}

/**
 * **One WordArt style's picture**: a bold letter *A* on the document's paper, filled, outlined and given its effect in
 * the document's colours, as static markup.
 *
 * ⚠ **Static for the reason the table pictures are**: `<mjx-gallery-item>` clones its children into the gallery's
 * shadow root. **Every colour is a checked hex colour or a token**, through `hexColour`, so a palette value cannot
 * inject markup. The picture's height matches a table style picture's, so the two galleries sit at one row height.
 */
export function wordArtStylePicture(look: WordArtLook, palette: ThemeColorPalette): TemplateResult {
  const paper = slotColour(palette, 'background1');
  const text = slotColour(palette, 'text1');
  const fill = slotColour(palette, look.fillSlot);
  const effect = slotColour(palette, look.effectSlot ?? 'text1');
  const clip = '-webkit-background-clip:text;background-clip:text;color:transparent';
  const fills: Record<WordArtLook['fill'], string> = {
    solid: `color:${fill}`,
    gradient: `background:linear-gradient(to bottom, color-mix(in srgb, ${fill} 40%, ${paper}), ${fill});${clip}`,
    pattern: `background:repeating-linear-gradient(135deg, ${fill} 0 ${step(0.25)}, color-mix(in srgb, ${fill} 30%, ${paper}) 0 ${step(0.5)});${clip}`,
  };
  const effects: Record<WordArtLook['effect'], string> = {
    none: '',
    shadow: `text-shadow:${step(0.25)} ${step(0.25)} ${step(0.25)} color-mix(in srgb, ${effect} 45%, transparent)`,
    'hard-shadow': `text-shadow:${step(0.375)} ${step(0.375)} 0 ${effect}`,
    glow: `text-shadow:0 0 ${step(0.5)} ${effect}, 0 0 ${step(0.25)} ${effect}`,
    reflection: `-webkit-box-reflect:below 0 linear-gradient(transparent 45%, color-mix(in srgb, ${text} 40%, transparent))`,
    'soft-bevel': `text-shadow:${step(-0.125)} ${step(-0.125)} ${step(0.125)} color-mix(in srgb, ${paper} 80%, transparent), ${step(0.125)} ${step(0.125)} ${step(0.25)} color-mix(in srgb, ${text} 45%, transparent)`,
    'sharp-bevel': `text-shadow:${step(-0.125)} ${step(-0.125)} 0 color-mix(in srgb, ${paper} 80%, transparent), ${step(0.125)} ${step(0.125)} 0 color-mix(in srgb, ${text} 55%, transparent)`,
  };
  const outline =
    look.outlineSlot === undefined ? '' : `-webkit-text-stroke:${step(0.125)} ${slotColour(palette, look.outlineSlot)}`;
  const markup =
    `<span style="display:grid;place-items:center;inline-size:100%;block-size:${step(6.25)};background:${paper};overflow:hidden">` +
    `<span style="font-weight:700;font-size:${step(4.5)};line-height:1;${fills[look.fill]};${outline};${effects[look.effect]}">A</span></span>`;
  return staticHtml`${unsafeStatic(markup)}`;
}

/** `Fill: Blue, Accent colour 1; Shadow` → `fill-blue-accent-colour-1-shadow`. */
function styleValue(label: string): string {
  return label.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/** One style, its value derived from its name. */
function style(label: string, look: WordArtLook): WordArtStyleSpec {
  return { value: styleValue(label), label, look };
}

/**
 * **The twenty WordArt styles**, four rows of five in Office's order, by the names the Office theme gives them.
 *
 * `GUESS:` every name, the order, and each look; Office's own thumbnails are drawn from each style's `a:rPr`, and these
 * are a description of a look rather than a render of one.
 */
export const wordArtStyles: readonly WordArtStyleSpec[] = [
  style('Fill: Black, Text colour 1; Shadow', { fill: 'solid', fillSlot: 'text1', effect: 'shadow' }),
  style('Fill: Blue, Accent colour 1; Shadow', { fill: 'solid', fillSlot: 'accent1', effect: 'shadow' }),
  style('Fill: Orange, Accent colour 2; Outline: Orange, Accent colour 2', { fill: 'solid', fillSlot: 'accent2', outlineSlot: 'accent2', effect: 'none' }),
  style('Fill: White; Outline: Orange, Accent colour 2; Glow: Orange, Accent colour 2', { fill: 'solid', fillSlot: 'background1', outlineSlot: 'accent2', effect: 'glow', effectSlot: 'accent2' }),
  style('Fill: Gold, Accent colour 4; Soft Bevel', { fill: 'solid', fillSlot: 'accent4', effect: 'soft-bevel' }),
  style('Gradient Fill: Grey', { fill: 'gradient', fillSlot: 'accent3', effect: 'none' }),
  style('Gradient Fill: Blue, Accent colour 5; Reflection', { fill: 'gradient', fillSlot: 'accent5', effect: 'reflection' }),
  style('Fill: White; Outline: Blue, Accent colour 1; Glow: Blue, Accent colour 1', { fill: 'solid', fillSlot: 'background1', outlineSlot: 'accent1', effect: 'glow', effectSlot: 'accent1' }),
  style('Fill: White; Outline: Blue, Accent colour 5; Shadow', { fill: 'solid', fillSlot: 'background1', outlineSlot: 'accent5', effect: 'shadow' }),
  style('Fill: Black, Text colour 1; Outline: White, Background colour 1; Shadow: Blue, Accent colour 5', { fill: 'solid', fillSlot: 'text1', outlineSlot: 'background1', effect: 'shadow', effectSlot: 'accent5' }),
  style('Fill: White; Outline: Blue, Accent colour 1; Shadow', { fill: 'solid', fillSlot: 'background1', outlineSlot: 'accent1', effect: 'shadow' }),
  style('Fill: Grey, Accent colour 3; Sharp Bevel', { fill: 'solid', fillSlot: 'accent3', effect: 'sharp-bevel' }),
  style('Fill: Black, Text colour 1; Outline: White, Background colour 1; Hard Shadow: Blue, Accent colour 5', { fill: 'solid', fillSlot: 'text1', outlineSlot: 'background1', effect: 'hard-shadow', effectSlot: 'accent5' }),
  style('Fill: Black, Text colour 1; Outline: White, Background colour 1; Hard Shadow: Orange, Accent colour 2', { fill: 'solid', fillSlot: 'text1', outlineSlot: 'background1', effect: 'hard-shadow', effectSlot: 'accent2' }),
  style('Fill: Blue, Accent colour 1; Outline: White, Background colour 1; Hard Shadow: Blue, Accent colour 1', { fill: 'solid', fillSlot: 'accent1', outlineSlot: 'background1', effect: 'hard-shadow', effectSlot: 'accent1' }),
  style('Fill: White; Outline: Grey, Accent colour 3; Hard Shadow: Grey, Accent colour 3', { fill: 'solid', fillSlot: 'background1', outlineSlot: 'accent3', effect: 'hard-shadow', effectSlot: 'accent3' }),
  style('Pattern Fill: White; Dark Upward Diagonal; Shadow', { fill: 'pattern', fillSlot: 'text1', effect: 'shadow' }),
  style('Pattern Fill: Blue, Accent colour 1, 50%; Hard Shadow: Blue, Accent colour 1', { fill: 'pattern', fillSlot: 'accent1', effect: 'hard-shadow', effectSlot: 'accent1' }),
  style('Gradient Fill: Gold, Accent colour 4; Outline: Gold, Accent colour 4', { fill: 'gradient', fillSlot: 'accent4', outlineSlot: 'accent4', effect: 'none' }),
  style('Fill: Blue, Accent colour 5; Outline: White, Background colour 1; Glow: Blue, Accent colour 5', { fill: 'solid', fillSlot: 'accent5', outlineSlot: 'background1', effect: 'glow', effectSlot: 'accent5' }),
];

/**
 * **The Quick Styles gallery's items**, one `<mjx-gallery-item>` per style, drawn in the document's palette. One
 * section, because Office draws one. A host sets no starting value: selected text wears no WordArt style.
 */
export function wordArtStyleGalleryItems(palette: ThemeColorPalette): TemplateResult[] {
  return wordArtStyles.map(
    (wordArt) => html`<mjx-gallery-item value=${wordArt.value} label=${wordArt.label} category="WordArt Styles"
      >${wordArtStylePicture(wordArt.look, palette)}</mjx-gallery-item
    >`,
  );
}

/** **The command under the expanded Quick Styles gallery**: Clear WordArt. `slot="footer"`. `GUESS:` that it is one. */
export function wordArtStyleGalleryFooter(): TemplateResult[] {
  return [html`<mjx-button slot="footer" label="Clear WordArt" size="small"></mjx-button>`];
}
