/**
 * **The menus, fields and gallery the Picture Format tab opens**, written once for all three applications: Word's,
 * PowerPoint's and Excel's Picture Format.
 *
 * The pattern is `stories/ribbons/table-tools-menus.ts`'s, for its reasons. A binding lives in its host. The menu it
 * opens is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `pictureToolsMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below spells its
 * command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## Shaped for three Picture Format tabs
 *
 * Office's Picture Format is nearly the same tab in all three applications, so **every list here is a function of
 * nothing or of the application, and only the menus function is per application**:
 *
 * - **Adjust**: `pictureCorrectionsEntries`, `pictureColourEntries`, `artisticEffectEntries`,
 *   `pictureTransparencyEntries`, `changePictureEntries` and `resetPictureEntries`. `GUESS:` that PowerPoint's and
 *   Excel's lists are Word's; PowerPoint's and Excel's call them unchanged.
 * - **Picture Styles**: `pictureStyles`, the twenty-eight styles, and `pictureStyleGalleryItems(palette)`; then
 *   `pictureEffectsEntries` and `pictureLayoutEntries`, which PowerPoint's Convert to SmartArt opens under its own
 *   name and Excel's Picture Layout under Word's. Picture Border is a colour picker over the document's palette, which
 *   a host owns, with `stories/ribbons/colour-picker-entries.ts`' `outlineEntries` beneath it; PowerPoint's host passes
 *   the Eyedropper, and Word's and Excel's do not.
 * - **Size**: `cropEntries`, over `cropShapeEntries` and `aspectRatioEntries`, and `wordPictureMeasures`,
 *   `powerpointPictureMeasures` and `excelPictureMeasures`, the height and width each host starts its two fields on.
 * - **Arrange is not here**: its commands are the census's `arrangeCommands` and its lists are
 *   `stories/ribbons/design-layout-menus.ts`' Arrange lists, which take the application. The menus themselves are
 *   rendered here, because their ids are Picture Format's.
 * - **The effect presets are not here either**: Shadow, Reflection, Glow, Bevel and 3-D Rotation are
 *   `stories/ribbons/wordart-styles-menus.ts`' lists, which Text Effects and Table Design's Effects already show.
 *   Preset and Soft Edges, which only a picture or a shape carries, are written here.
 * - **Two lists here also serve Shape Format**, through `stories/ribbons/drawing-tools-menus.ts`: `pictureEffectsEntries`
 *   is Shape Effects' menu, and `cropShapes` is Change Shape's list and the body of the whole Shapes gallery.
 *
 * `GUESS:` every label, order and preset below, from memory of Microsoft 365. Where a label differs from Office's
 * spelling the census's wins (*Colour*, *Greyscale*, *Centre*, *Recolour*).
 *
 * ## The pictures are the document's colours, not the chrome's
 *
 * A picture style is a frame, a shape and an effect around a photograph, and its white or black frame is the
 * document's Background 1 or Text 1. So each thumbnail is drawn in the document's `ThemeColorPalette`, through
 * `stories/ribbons/palette-art.ts`, as static markup for the reason the table and WordArt pictures are:
 * `<mjx-gallery-item>` clones its children into the gallery's shadow root.
 *
 * **Nothing here dispatches a command.** A menu opens, an entry can be chosen, a gallery previews and commits; no
 * picture changes, because command dispatch is loop 2.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';
import { html as staticHtml, unsafeStatic } from 'lit/static-html.js';

import type { RibbonApplication, RibbonSurfaceHost } from '../../dev/ribbons/census.ts';
import type { ThemeColorPalette, ThemeColorSlot } from '../../src/pickers/picker-model.ts';
import {
  alignEntries,
  bringForwardEntries,
  groupEntries,
  positionEntries,
  rotateEntries,
  sendBackwardEntries,
  wrapTextEntries,
} from './design-layout-menus.ts';
import { paletteSlotColour, spacingStep } from './palette-art.ts';
import { commandMenu } from './ribbon-parts.ts';
import {
  accentNames,
  bevelPresetEntries,
  glowPresetEntries,
  reflectionPresetEntries,
  rotationPresetEntries,
  shadowPresetEntries,
  submenu,
} from './wordart-styles-menus.ts';

// ── the entries ──────────────────────────────────────────────────────────────

/** One entry. */
function item(label: string): TemplateResult {
  return html`<mjx-menu-item label=${label}></mjx-menu-item>`;
}

/** One entry of a set whose current member is checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/** A labelled run of entries. */
function section(label: string, ...entries: TemplateResult[]): TemplateResult {
  return html`<mjx-menu-section label=${label}>${entries}</mjx-menu-section>`;
}

const separator = (): TemplateResult => html`<mjx-menu-separator></mjx-menu-separator>`;

/** `-40` → `-40%`, `20` → `+20%`, `0` → `0%`, as Office's correction names write a change. */
function signedPercent(value: number): string {
  return value > 0 ? `+${String(value)}%` : `${String(value)}%`;
}

// ── Adjust ───────────────────────────────────────────────────────────────────

/**
 * **Corrections' gallery, as a menu**: *Sharpen/Soften*, five steps from Soften 50% to Sharpen 50%, and
 * *Brightness/Contrast*, a five-by-five grid read across its rows (contrast down, brightness across, each −40% to
 * +40%), then Picture Corrections Options…. **The unchanged picture is checked in both sections**: a picture that has
 * been given no correction is at 0%.
 *
 * Office draws both sections as grids of the picture itself; a picture of *this* picture is document content a
 * ribbon has no copy of, so each entry is its name.
 */
export function pictureCorrectionsEntries(): TemplateResult[] {
  const steps = [-40, -20, 0, 20, 40];
  return [
    section(
      'Sharpen/Soften',
      choice('Soften: 50%'),
      choice('Soften: 25%'),
      choice('Sharpen: 0%', true),
      choice('Sharpen: 25%'),
      choice('Sharpen: 50%'),
    ),
    section(
      'Brightness/Contrast',
      ...steps.flatMap((contrast) =>
        steps.map((brightness) =>
          choice(
            `Brightness: ${signedPercent(brightness)} Contrast: ${signedPercent(contrast)}`,
            brightness === 0 && contrast === 0,
          ),
        ),
      ),
    ),
    separator(),
    item('Picture Corrections Options…'),
  ];
}

/**
 * **Colour's gallery, as a menu**: *Colour Saturation*'s seven steps, *Colour Tone*'s seven temperatures, and
 * *Recolour*'s twenty-one (No Recolour, Greyscale, Sepia, Washout and three black-and-white thresholds, then Text 2
 * and the six accents dark, then light), then More Variations, Set Transparent Colour and Picture Colour Options….
 * **An unchanged picture is checked in each**: Saturation 100%, Temperature 6500 K, No Recolour.
 *
 * `GUESS:` that More Variations is an entry: in Office it opens a colour grid, which a menu cannot hold, as Glow's More
 * Glow Colours records. The variations are named from the Office theme, as WordArt's are.
 */
export function pictureColourEntries(): TemplateResult[] {
  const saturations = ['0%', '33%', '66%', '100%', '200%', '300%', '400%'];
  const temperatures = ['4700 K', '5300 K', '5900 K', '6500 K', '7200 K', '8800 K', '11200 K'];
  const tints = [{ name: 'Blue-Grey, Text colour 2' }, ...accentNames];
  return [
    section('Colour Saturation', ...saturations.map((value) => choice(`Saturation: ${value}`, value === '100%'))),
    section('Colour Tone', ...temperatures.map((value) => choice(`Temperature: ${value}`, value === '6500 K'))),
    section(
      'Recolour',
      choice('No Recolour', true),
      choice('Greyscale'),
      choice('Sepia'),
      choice('Washout'),
      choice('Black and White: 25%'),
      choice('Black and White: 50%'),
      choice('Black and White: 75%'),
      ...tints.map((tint) => choice(`${tint.name} Dark`)),
      ...tints.map((tint) => choice(`${tint.name} Light`)),
    ),
    separator(),
    item('More Variations'),
    item('Set Transparent Colour'),
    item('Picture Colour Options…'),
  ];
}

/**
 * **The twenty-three artistic effects**, None first, in Office's order: four rows of five and a row of three.
 * Office's own names, in the census's spelling (*Greyscale*, *Watercolour*).
 */
export const artisticEffects: readonly string[] = [
  'None',
  'Marker',
  'Pencil Greyscale',
  'Pencil Sketch',
  'Line Drawing',
  'Chalk Sketch',
  'Paint Strokes',
  'Paint Brush',
  'Glow Diffused',
  'Blur',
  'Light Screen',
  'Watercolour Sponge',
  'Film Grain',
  'Mosaic Bubbles',
  'Glass',
  'Cement',
  'Texturizer',
  'Crisscross Etching',
  'Pastels Smooth',
  'Plastic Wrap',
  'Cutout',
  'Photocopy',
  'Glow Edges',
];

/** **Artistic Effects' gallery, as a menu**: the twenty-three, None checked, then Artistic Effects Options…. */
export function artisticEffectEntries(): TemplateResult[] {
  return [
    section('Artistic Effects', ...artisticEffects.map((effect) => choice(effect, effect === 'None'))),
    separator(),
    item('Artistic Effects Options…'),
  ];
}

/** **Transparency's gallery, as a menu**: seven steps from 0% (checked) to 95%, then Picture Transparency Options…. */
export function pictureTransparencyEntries(): TemplateResult[] {
  const steps = ['0%', '15%', '30%', '50%', '65%', '80%', '95%'];
  return [
    section('Transparency', ...steps.map((step) => choice(`Transparency: ${step}`, step === '0%'))),
    separator(),
    item('Picture Transparency Options…'),
  ];
}

/** **Change Picture's menu**: the five places a replacement comes from. */
export function changePictureEntries(): TemplateResult[] {
  return [
    item('From a File…'),
    item('From Stock Images…'),
    item('From Online Sources…'),
    item('From Icons…'),
    item('From Clipboard'),
  ];
}

/** **Reset Picture's arrow**: the face's command, and the one that also discards the size. */
export function resetPictureEntries(): TemplateResult[] {
  return [item('Reset Picture'), item('Reset Picture & Size')];
}

// ── Picture Styles: the menus ────────────────────────────────────────────────

/** **Preset's list**: No Presets, the twelve 3-D presets, then 3-D Options…. Only a picture or a shape has it. */
function presetEffectEntries(): TemplateResult[] {
  return [
    item('No Presets'),
    section('Presets', ...Array.from({ length: 12 }, (_, index) => item(`Preset ${String(index + 1)}`))),
    separator(),
    item('3-D Options…'),
  ];
}

/** **Soft Edges' list**: No Soft Edges, six widths in points, then Soft Edges Options…. Text has none. */
export function softEdgeEntries(): TemplateResult[] {
  const widths = ['1 Point', '2.5 Point', '5 Point', '10 Point', '25 Point', '50 Point'];
  return [
    item('No Soft Edges'),
    section('Soft Edge Variations', ...widths.map((width) => item(width))),
    separator(),
    item('Soft Edges Options…'),
  ];
}

/**
 * **Picture Effects' menu**: Preset, Shadow, Reflection, Glow, Soft Edges, Bevel and 3-D Rotation, each a submenu
 * with Office's whole list, in Office's order. Shadow, Reflection, Glow, Bevel and 3-D Rotation are the lists Text
 * Effects shows. `GUESS:` that a picture's lists are text's.
 */
export function pictureEffectsEntries(): TemplateResult[] {
  return [
    submenu('Preset', ...presetEffectEntries()),
    submenu('Shadow', ...shadowPresetEntries()),
    submenu('Reflection', ...reflectionPresetEntries()),
    submenu('Glow', ...glowPresetEntries()),
    submenu('Soft Edges', ...softEdgeEntries()),
    submenu('Bevel', ...bevelPresetEntries('3-D Options…')),
    submenu('3-D Rotation', ...rotationPresetEntries()),
  ];
}

/**
 * **Picture Layout's thirty-one SmartArt picture layouts**, as a menu of names in the gallery's order. Office draws
 * each as a diagram; `GUESS:` the names, the order and that the gallery holds exactly these.
 */
export const pictureLayouts: readonly string[] = [
  'Picture Caption List',
  'Bending Picture Accent List',
  'Horizontal Picture List',
  'Titled Picture Accent List',
  'Picture Accent List',
  'Snapshot Picture List',
  'Picture Accent Blocks',
  'Bending Picture Caption',
  'Bending Picture Semi-Transparent Text',
  'Bending Picture Blocks',
  'Bending Picture Caption List',
  'Titled Picture Blocks',
  'Picture Grid',
  'Accented Picture',
  'Circular Picture Callout',
  'Picture Strips',
  'Framed Text Picture',
  'Picture Frame',
  'Title Picture Lineup',
  'Picture Lineup',
  'Alternating Picture Blocks',
  'Alternating Picture Circles',
  'Ascending Picture Accent Process',
  'Picture Accent Process',
  'Continuous Picture List',
  'Vertical Picture List',
  'Vertical Picture Accent List',
  'Hexagon Cluster',
  'Spiral Picture',
  'Captioned Pictures',
  'Circle Picture Hierarchy',
];

/** **Picture Layout's gallery, as a menu**: the thirty-one layouts. Nothing is checked: a picture wears no layout. */
export function pictureLayoutEntries(): TemplateResult[] {
  return [section('Picture Layouts', ...pictureLayouts.map((layout) => item(layout)))];
}

// ── Size: Crop ───────────────────────────────────────────────────────────────

/**
 * **The shapes a picture can be cropped to**, by section in the Shapes gallery's order and by Office's names:
 * Rectangles, Basic Shapes, Block Arrows, Equation Shapes, Flowchart, Stars and Banners, and Callouts. **No Lines**,
 * because a line encloses nothing, and no Text Box. Exported, because Insert's Shapes gallery is the same list with
 * Lines in front and today shows a handful of it.
 *
 * `GUESS:` every name, each section's order, and that Crop to Shape leaves out exactly Lines and the Text Box.
 */
export const cropShapes: readonly { readonly section: string; readonly shapes: readonly string[] }[] = [
  {
    section: 'Rectangles',
    shapes: [
      'Rectangle',
      'Rectangle: Rounded Corners',
      'Rectangle: Single Corner Snipped',
      'Rectangle: Top Corners Snipped',
      'Rectangle: Diagonal Corners Snipped',
      'Rectangle: Top Corners One Rounded and One Snipped',
      'Rectangle: Single Corner Rounded',
      'Rectangle: Top Corners Rounded',
      'Rectangle: Diagonal Corners Rounded',
    ],
  },
  {
    section: 'Basic Shapes',
    shapes: [
      'Oval',
      'Isosceles Triangle',
      'Right Triangle',
      'Parallelogram',
      'Trapezoid',
      'Diamond',
      'Regular Pentagon',
      'Hexagon',
      'Heptagon',
      'Octagon',
      'Decagon',
      'Dodecagon',
      'Partial Circle',
      'Chord',
      'Teardrop',
      'Frame',
      'Half Frame',
      'L-Shape',
      'Diagonal Stripe',
      'Cross',
      'Plaque',
      'Can',
      'Cube',
      'Rectangle: Beveled',
      'Circle: Hollow',
      '"No" Symbol',
      'Block Arc',
      'Rectangle: Folded Corner',
      'Smiley Face',
      'Heart',
      'Lightning Bolt',
      'Sun',
      'Moon',
      'Cloud',
      'Arc',
      'Double Bracket',
      'Double Brace',
      'Left Bracket',
      'Right Bracket',
      'Left Brace',
      'Right Brace',
    ],
  },
  {
    section: 'Block Arrows',
    shapes: [
      'Arrow: Right',
      'Arrow: Left',
      'Arrow: Up',
      'Arrow: Down',
      'Arrow: Left-Right',
      'Arrow: Up-Down',
      'Arrow: Quad',
      'Arrow: Left-Right-Up',
      'Arrow: Bent',
      'Arrow: U-Turn',
      'Arrow: Left-Up',
      'Arrow: Bent-Up',
      'Arrow: Curved Right',
      'Arrow: Curved Left',
      'Arrow: Curved Up',
      'Arrow: Curved Down',
      'Arrow: Striped Right',
      'Arrow: Notched Right',
      'Arrow: Pentagon',
      'Arrow: Chevron',
      'Callout: Right Arrow',
      'Callout: Down Arrow',
      'Callout: Left Arrow',
      'Callout: Up Arrow',
      'Callout: Left-Right Arrow',
      'Callout: Quad Arrow',
      'Arrow: Circular',
    ],
  },
  {
    section: 'Equation Shapes',
    shapes: ['Plus Sign', 'Minus Sign', 'Multiplication Sign', 'Division Sign', 'Equals', 'Not Equal'],
  },
  {
    section: 'Flowchart',
    shapes: [
      'Process',
      'Alternate Process',
      'Decision',
      'Data',
      'Predefined Process',
      'Internal Storage',
      'Document',
      'Multidocument',
      'Terminator',
      'Preparation',
      'Manual Input',
      'Manual Operation',
      'Connector',
      'Off-page Connector',
      'Card',
      'Punched Tape',
      'Summing Junction',
      'Or',
      'Collate',
      'Sort',
      'Extract',
      'Merge',
      'Stored Data',
      'Delay',
      'Sequential Access Storage',
      'Magnetic Disk',
      'Direct Access Storage',
      'Display',
    ].map((shape) => `Flowchart: ${shape}`),
  },
  {
    section: 'Stars and Banners',
    shapes: [
      'Explosion: 8 Points',
      'Explosion: 14 Points',
      'Star: 4 Points',
      'Star: 5 Points',
      'Star: 6 Points',
      'Star: 7 Points',
      'Star: 8 Points',
      'Star: 10 Points',
      'Star: 12 Points',
      'Star: 16 Points',
      'Star: 24 Points',
      'Star: 32 Points',
      'Ribbon: Curved and Tilted Up',
      'Ribbon: Curved and Tilted Down',
      'Ribbon: Tilted Up',
      'Ribbon: Tilted Down',
      'Scroll: Vertical',
      'Scroll: Horizontal',
      'Wave',
      'Double Wave',
    ],
  },
  {
    section: 'Callouts',
    shapes: [
      'Speech Bubble: Rectangle',
      'Speech Bubble: Rectangle with Corners Rounded',
      'Speech Bubble: Oval',
      'Thought Bubble: Cloud',
      'Callout: Line',
      'Callout: Bent Line',
      'Callout: Double Bent Line',
      'Callout: Line with Accent Bar',
      'Callout: Bent Line with Accent Bar',
      'Callout: Double Bent Line with Accent Bar',
      'Callout: Line with No Border',
      'Callout: Bent Line with No Border',
      'Callout: Double Bent Line with No Border',
      'Callout: Line with Border and Accent Bar',
      'Callout: Bent Line with Border and Accent Bar',
      'Callout: Double Bent Line with Border and Accent Bar',
    ],
  },
];

/** **Crop to Shape ▸**: every section of `cropShapes`. */
export function cropShapeEntries(): TemplateResult[] {
  return cropShapes.map((group) => section(group.section, ...group.shapes.map((shape) => item(shape))));
}

/** **Aspect Ratio ▸**: Square, then Office's four portrait and six landscape ratios. */
export function aspectRatioEntries(): TemplateResult[] {
  return [
    section('Square', item('1:1')),
    section('Portrait', ...['2:3', '3:4', '3:5', '4:5'].map((ratio) => item(ratio))),
    section('Landscape', ...['3:2', '4:3', '5:3', '5:4', '16:9', '16:10'].map((ratio) => item(ratio))),
  ];
}

/** **Crop's arrow**: Crop, Crop to Shape ▸, Aspect Ratio ▸, Fill and Fit. */
export function cropEntries(): TemplateResult[] {
  return [
    item('Crop'),
    submenu('Crop to Shape', ...cropShapeEntries()),
    submenu('Aspect Ratio', ...aspectRatioEntries()),
    separator(),
    item('Fill'),
    item('Fit'),
  ];
}

/**
 * **The height and width a Word host starts Size's two fields on**, in centimetres: a 4:3 photograph Word has inserted
 * and scaled to 11.43 cm across. Written once, so both fields describe one picture. `GUESS:` both numbers and the
 * 0.01 cm step.
 */
export const wordPictureMeasures = { height: '8.57', width: '11.43', step: '0.01' } as const;

/**
 * **The height and width a PowerPoint host starts Size's two fields on**, in centimetres: a 4:3 photograph PowerPoint
 * has scaled to the height of a 16:9 slide, 19.05 cm, so 25.4 cm across. **PowerPoint's own**, because a slide is not a
 * column of text: Word's picture is scaled to the page's measure. `GUESS:` both numbers and the 0.01 cm step.
 */
export const powerpointPictureMeasures = { height: '19.05', width: '25.4', step: '0.01' } as const;

/**
 * **The height and width an Excel host starts Size's two fields on**, in centimetres: a 640 × 480 photograph at 96 dpi,
 * which Excel inserts at its own size, so 9.53 cm by 12.7 cm. **Excel's own**, because a worksheet scales a picture to
 * nothing: Word's is scaled to a column and PowerPoint's to a slide's height. `GUESS:` the photograph, both numbers and
 * the 0.01 cm step.
 */
export const excelPictureMeasures = { height: '9.53', width: '12.7', step: '0.01' } as const;

// ── Picture Styles: the gallery ──────────────────────────────────────────────

/** What a picture style's thumbnail shows. A description of a look, never a render of the style's `a:spPr`. */
export interface PictureStyleLook {
  /** The frame: none, a thin or thick band, a double line, a wide matte, a band with a line inside it, or metal. */
  readonly frame: 'none' | 'thin' | 'thick' | 'double' | 'matte' | 'compound' | 'metal';
  /** The document colour a white or black frame is drawn in. */
  readonly frameSlot?: 'background1' | 'text1';
  /** The outline the picture is cut to. */
  readonly shape: 'rectangle' | 'rounded' | 'oval' | 'snip-diagonal' | 'round-diagonal';
  /** The one effect beyond the frame. */
  readonly effect:
    | 'none'
    | 'shadow'
    | 'centre-shadow'
    | 'reflection'
    | 'soft-edge'
    | 'bevel'
    | 'perspective-left'
    | 'perspective-right'
    | 'relaxed'
    | 'rotated';
}

/** One picture style: its value, Office's name for it, and its look. */
export interface PictureStyleSpec {
  readonly value: string;
  readonly label: string;
  readonly look: PictureStyleLook;
}

/**
 * **One picture style's thumbnail**: a landscape photograph (a sky in Accent 1 over land in Accent 6, with a sun in
 * Accent 4) framed, cut and given its effect, on the document's paper, as static markup.
 *
 * ⚠ **Static for the reason every gallery picture is**, and **every colour is a checked hex colour or a token**,
 * through `paletteSlotColour`. The thumbnail's height matches the WordArt and table style pictures'.
 */
export function pictureStylePicture(look: PictureStyleLook, palette: ThemeColorPalette): TemplateResult {
  const step = spacingStep;
  const colour = (slot: ThemeColorSlot): string => paletteSlotColour(palette, slot);
  const paper = colour('background1');
  const ink = colour('text1');
  const sky = `color-mix(in srgb, ${colour('accent1')} 45%, ${paper})`;
  const land = colour('accent6');
  const sun = colour('accent4');
  const frameColour =
    look.frame === 'metal'
      ? `color-mix(in srgb, ${colour('accent3')} 70%, ${paper})`
      : colour(look.frameSlot ?? 'background1');
  const frames: Record<PictureStyleLook['frame'], string> = {
    none: '',
    thin: `border:${step(0.25)} solid ${frameColour}`,
    thick: `border:${step(0.625)} solid ${frameColour}`,
    double: `border:${step(0.5)} double ${frameColour}`,
    matte: `border:${step(0.75)} solid ${frameColour}`,
    compound: `border:${step(0.5)} solid ${frameColour};outline:1px solid ${paper};outline-offset:calc(${step(0.25)} * -1)`,
    metal: `border:${step(0.5)} ridge ${frameColour}`,
  };
  const shapes: Record<PictureStyleLook['shape'], string> = {
    rectangle: '',
    rounded: `border-radius:${step(0.75)}`,
    oval: 'border-radius:50%',
    'snip-diagonal': 'clip-path:polygon(18% 0, 100% 0, 100% 82%, 82% 100%, 0 100%, 0 18%)',
    'round-diagonal': `border-radius:${step(1)} 0 ${step(1)} 0`,
  };
  const shade = `color-mix(in srgb, ${ink} 45%, transparent)`;
  const effects: Record<PictureStyleLook['effect'], string> = {
    none: '',
    shadow: `box-shadow:${step(0.25)} ${step(0.25)} ${step(0.375)} ${shade}`,
    'centre-shadow': `box-shadow:0 0 ${step(0.5)} ${shade}`,
    reflection: `-webkit-box-reflect:below 1px linear-gradient(transparent 55%, color-mix(in srgb, ${ink} 35%, transparent))`,
    // A mask reads alpha alone, so the opaque stop is the document's ink rather than a literal colour.
    'soft-edge': `mask-image:radial-gradient(closest-side, ${ink} 70%, transparent)`,
    bevel: `box-shadow:inset ${step(0.125)} ${step(0.125)} 0 color-mix(in srgb, ${paper} 70%, transparent), inset calc(${step(0.125)} * -1) calc(${step(0.125)} * -1) 0 ${shade}`,
    'perspective-left': 'transform:perspective(6em) rotateY(18deg)',
    'perspective-right': 'transform:perspective(6em) rotateY(-18deg)',
    relaxed: 'transform:perspective(6em) rotateX(20deg)',
    rotated: 'transform:rotate(-6deg)',
  };
  const photo =
    `background:radial-gradient(circle at 72% 30%, ${sun} 0 12%, transparent 13%),` +
    `linear-gradient(to bottom, ${sky} 0 58%, ${land} 58% 100%)`;
  const markup =
    `<span style="display:grid;place-items:center;inline-size:100%;block-size:${step(6.25)};background:${paper};overflow:hidden">` +
    `<span style="display:block;box-sizing:border-box;inline-size:62%;block-size:${step(4)};${photo};` +
    `${frames[look.frame]};${shapes[look.shape]};${effects[look.effect]}"></span></span>`;
  return staticHtml`${unsafeStatic(markup)}`;
}

/** `Simple Frame, White` → `simple-frame-white`. */
function pictureStyleValue(label: string): string {
  return label.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

/** One style, its value derived from its name. */
function pictureStyle(label: string, look: PictureStyleLook): PictureStyleSpec {
  return { value: pictureStyleValue(label), label, look };
}

/**
 * **The twenty-eight picture styles**, four rows of seven in Office's order, by Office's names; *Centre* in the
 * census's spelling. Shared by all three applications' Quick Styles, which carry the same twenty-eight.
 *
 * `GUESS:` the order and every look; Office's thumbnails are the selected picture wearing each style, and these are a
 * description of the style around a stand-in photograph.
 */
export const pictureStyles: readonly PictureStyleSpec[] = [
  pictureStyle('Simple Frame, White', { frame: 'thin', frameSlot: 'background1', shape: 'rectangle', effect: 'shadow' }),
  pictureStyle('Beveled Matte, White', { frame: 'matte', frameSlot: 'background1', shape: 'rectangle', effect: 'bevel' }),
  pictureStyle('Metal Frame', { frame: 'metal', shape: 'rectangle', effect: 'none' }),
  pictureStyle('Drop Shadow Rectangle', { frame: 'none', shape: 'rectangle', effect: 'shadow' }),
  pictureStyle('Reflected Rounded Rectangle', { frame: 'none', shape: 'rounded', effect: 'reflection' }),
  pictureStyle('Soft Edge Rectangle', { frame: 'none', shape: 'rectangle', effect: 'soft-edge' }),
  pictureStyle('Double Frame, Black', { frame: 'double', frameSlot: 'text1', shape: 'rectangle', effect: 'none' }),
  pictureStyle('Thick Matte, Black', { frame: 'matte', frameSlot: 'text1', shape: 'rectangle', effect: 'none' }),
  pictureStyle('Simple Frame, Black', { frame: 'thin', frameSlot: 'text1', shape: 'rectangle', effect: 'none' }),
  pictureStyle('Beveled Oval, Black', { frame: 'thick', frameSlot: 'text1', shape: 'oval', effect: 'bevel' }),
  pictureStyle('Compound Frame, Black', { frame: 'compound', frameSlot: 'text1', shape: 'rectangle', effect: 'none' }),
  pictureStyle('Moderate Frame, Black', { frame: 'thick', frameSlot: 'text1', shape: 'rectangle', effect: 'shadow' }),
  pictureStyle('Centre Shadow Rectangle', { frame: 'none', shape: 'rectangle', effect: 'centre-shadow' }),
  pictureStyle('Rounded Diagonal Corner, White', { frame: 'thick', frameSlot: 'background1', shape: 'round-diagonal', effect: 'shadow' }),
  pictureStyle('Snip Diagonal Corner, White', { frame: 'thick', frameSlot: 'background1', shape: 'snip-diagonal', effect: 'none' }),
  pictureStyle('Moderate Frame, White', { frame: 'thick', frameSlot: 'background1', shape: 'rectangle', effect: 'shadow' }),
  pictureStyle('Rotated, White', { frame: 'thick', frameSlot: 'background1', shape: 'rectangle', effect: 'rotated' }),
  pictureStyle('Perspective Shadow, White', { frame: 'thin', frameSlot: 'background1', shape: 'rectangle', effect: 'perspective-right' }),
  pictureStyle('Relaxed Perspective, White', { frame: 'thick', frameSlot: 'background1', shape: 'rectangle', effect: 'relaxed' }),
  pictureStyle('Soft Edge Oval', { frame: 'none', shape: 'oval', effect: 'soft-edge' }),
  pictureStyle('Bevel Rectangle', { frame: 'none', shape: 'rectangle', effect: 'bevel' }),
  pictureStyle('Bevel Perspective', { frame: 'none', shape: 'rectangle', effect: 'perspective-left' }),
  pictureStyle('Reflected Perspective Right', { frame: 'none', shape: 'rectangle', effect: 'perspective-right' }),
  pictureStyle('Bevel Perspective Left, White', { frame: 'thick', frameSlot: 'background1', shape: 'rectangle', effect: 'perspective-left' }),
  pictureStyle('Reflected Bevel, White', { frame: 'thin', frameSlot: 'background1', shape: 'rounded', effect: 'reflection' }),
  pictureStyle('Reflected Bevel, Black', { frame: 'thin', frameSlot: 'text1', shape: 'rounded', effect: 'reflection' }),
  pictureStyle('Metal Rounded Rectangle', { frame: 'metal', shape: 'rounded', effect: 'none' }),
  pictureStyle('Metal Oval', { frame: 'metal', shape: 'oval', effect: 'none' }),
];

/**
 * **The Quick Styles gallery's items**, one `<mjx-gallery-item>` per style, drawn in the document's palette. One
 * section, because Office draws one, and no footer. A host sets no starting value: an inserted picture wears no style.
 */
export function pictureStyleGalleryItems(palette: ThemeColorPalette): TemplateResult[] {
  return pictureStyles.map(
    (style) => html`<mjx-gallery-item value=${style.value} label=${style.label} category="Picture Styles"
      >${pictureStylePicture(style.look, palette)}</mjx-gallery-item
    >`,
  );
}

// ── what a host renders ──────────────────────────────────────────────────────

/**
 * Word's sixteen menus. **Adjust's six**: Corrections, Colour, Artistic Effects and Transparency, galleries drawn as
 * menus; Change Picture; Reset Picture's arrow. **Picture Styles' two**: Picture Effects and Picture Layout. **Arrange's
 * seven**, over `stories/ribbons/design-layout-menus.ts`' Word lists. **Size's one**: Crop's arrow. Every other Picture
 * Format command is a field, a picker, a gallery, a toggle or a plain button.
 */
function wordPictureToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.picture-format.adjust.corrections', 'Corrections', ...pictureCorrectionsEntries())}
    ${commandMenu(host, 'word.picture-format.adjust.colour', 'Colour', ...pictureColourEntries())}
    ${commandMenu(host, 'word.picture-format.adjust.artistic-effects', 'Artistic Effects', ...artisticEffectEntries())}
    ${commandMenu(host, 'word.picture-format.adjust.transparency', 'Transparency', ...pictureTransparencyEntries())}
    ${commandMenu(host, 'word.picture-format.adjust.change-picture', 'Change Picture', ...changePictureEntries())}
    ${commandMenu(host, 'word.picture-format.adjust.reset-picture', 'Reset Picture', ...resetPictureEntries())}
    ${commandMenu(host, 'word.picture-format.picture-styles.picture-effects', 'Picture Effects', ...pictureEffectsEntries())}
    ${commandMenu(host, 'word.picture-format.picture-styles.picture-layout', 'Picture Layout', ...pictureLayoutEntries())}
    ${commandMenu(host, 'word.picture-format.arrange.position', 'Position', ...positionEntries())}
    ${commandMenu(host, 'word.picture-format.arrange.wrap-text', 'Wrap Text', ...wrapTextEntries())}
    ${commandMenu(host, 'word.picture-format.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('word'))}
    ${commandMenu(host, 'word.picture-format.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('word'))}
    ${commandMenu(host, 'word.picture-format.arrange.align', 'Align', ...alignEntries('word'))}
    ${commandMenu(host, 'word.picture-format.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'word.picture-format.arrange.rotate', 'Rotate', ...rotateEntries())}
    ${commandMenu(host, 'word.picture-format.size.crop', 'Crop', ...cropEntries())}
  `;
}

/**
 * PowerPoint's fourteen menus. **Adjust's six**, Word's lists. **Picture Styles' two**: Picture Effects, and **Convert
 * to SmartArt**, PowerPoint's name for Picture Layout, over the same thirty-one layouts. **Arrange's five**, over
 * `stories/ribbons/design-layout-menus.ts`' PowerPoint lists: Bring Forward and Send Backward with no text layer, Align
 * to the slide or to the selected objects, then Group and Rotate. **No Position or Wrap Text**: a picture on a slide
 * sits among no text. **Size's one**: Crop's arrow.
 */
function powerpointPictureToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.picture-format.adjust.corrections', 'Corrections', ...pictureCorrectionsEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.adjust.colour', 'Colour', ...pictureColourEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.adjust.artistic-effects', 'Artistic Effects', ...artisticEffectEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.adjust.transparency', 'Transparency', ...pictureTransparencyEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.adjust.change-picture', 'Change Picture', ...changePictureEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.adjust.reset-picture', 'Reset Picture', ...resetPictureEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.picture-styles.picture-effects', 'Picture Effects', ...pictureEffectsEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.picture-styles.convert-to-smartart', 'Convert to SmartArt', ...pictureLayoutEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.picture-format.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.picture-format.arrange.align', 'Align', ...alignEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.picture-format.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.arrange.rotate', 'Rotate', ...rotateEntries())}
    ${commandMenu(host, 'powerpoint.picture-format.size.crop', 'Crop', ...cropEntries())}
  `;
}

/**
 * Excel's fourteen menus. **Adjust's six**, Word's lists. **Picture Styles' two**: Picture Effects and Picture Layout,
 * Word's names and lists. **Arrange's five**, over `stories/ribbons/design-layout-menus.ts`' Excel lists: Bring Forward
 * and Send Backward with no text layer, Align ending on Snap to Grid, Snap to Shape and View Gridlines, then Group and
 * Rotate. **No Position or Wrap Text**: cells do not wrap around a picture. **Size's one**: Crop's arrow.
 */
function excelPictureToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'excel.picture-format.adjust.corrections', 'Corrections', ...pictureCorrectionsEntries())}
    ${commandMenu(host, 'excel.picture-format.adjust.colour', 'Colour', ...pictureColourEntries())}
    ${commandMenu(host, 'excel.picture-format.adjust.artistic-effects', 'Artistic Effects', ...artisticEffectEntries())}
    ${commandMenu(host, 'excel.picture-format.adjust.transparency', 'Transparency', ...pictureTransparencyEntries())}
    ${commandMenu(host, 'excel.picture-format.adjust.change-picture', 'Change Picture', ...changePictureEntries())}
    ${commandMenu(host, 'excel.picture-format.adjust.reset-picture', 'Reset Picture', ...resetPictureEntries())}
    ${commandMenu(host, 'excel.picture-format.picture-styles.picture-effects', 'Picture Effects', ...pictureEffectsEntries())}
    ${commandMenu(host, 'excel.picture-format.picture-styles.picture-layout', 'Picture Layout', ...pictureLayoutEntries())}
    ${commandMenu(host, 'excel.picture-format.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('excel'))}
    ${commandMenu(host, 'excel.picture-format.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('excel'))}
    ${commandMenu(host, 'excel.picture-format.arrange.align', 'Align', ...alignEntries('excel'))}
    ${commandMenu(host, 'excel.picture-format.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'excel.picture-format.arrange.rotate', 'Rotate', ...rotateEntries())}
    ${commandMenu(host, 'excel.picture-format.size.crop', 'Crop', ...cropEntries())}
  `;
}

/**
 * Every menu one application's Picture Format tab opens, with ids for one host's page. **All three are authored.**
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, by every host that draws Picture Tools **and** binds its
 * commands: `Ribbons/Word`, `Ribbons/PowerPoint`, `Shell/PowerPoint`, whose deck's selection is a picture, and
 * `Ribbons/Excel`. `Shell/Word` and `Shell/Excel` draw Table Tools alone, so they render none of these, and
 * `tests/ribbons.test.ts` requires Word's and Excel's menus of their `Ribbons/*` host alone and PowerPoint's of both
 * PowerPoint hosts.
 */
export function pictureToolsMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  if (application === 'word') return wordPictureToolsMenus(host);
  if (application === 'powerpoint') return powerpointPictureToolsMenus(host);
  return excelPictureToolsMenus(host);
}
