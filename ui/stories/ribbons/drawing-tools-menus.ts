/**
 * **The menus, gallery and starting measures the Shape Format tab opens**, written once for all three applications:
 * PowerPoint's and Word's Shape Format today, and Excel's when its unit lands. Insert's Shapes menu reads the same
 * shape gallery (`insertShapesEntries`), so the gallery Office repeats on two tabs is written once.
 *
 * **Word's differs from PowerPoint's only where Office's Word does**, and those pieces are written here beside the
 * shared ones: `drawTextBoxEntries()`, Draw Text Box's arrow, where PowerPoint has a plain Text Box and Merge Shapes;
 * the **Text** group's `wordTextDirectionEntries()` and `alignTextEntries()`; `wordShapeMeasures`; and Word's
 * Position and Wrap Text menus, rendered from `design-layout-menus.ts`.
 *
 * The pattern is `stories/ribbons/picture-tools-menus.ts`'s, for its reasons. A binding lives in its host. The menu it
 * opens is written here, with its id from `commandSurfaceId(host, commandId)` through `commandMenu`. A host renders
 * `drawingToolsMenus(application, host)` once beside its ribbon. Every `commandMenu(host, '…'` call below spells its
 * command id literally, so `tests/ribbons.test.ts` can read it.
 *
 * ## Shaped for three Shape Format tabs
 *
 * Office's Shape Format is nearly the same tab in all three applications, so **every list here is a function of
 * nothing or of the application, and only the menus function is per application**:
 *
 * - **Insert Shapes**: `insertShapesEntries(application)`, the whole shape gallery (Lines, Rectangles, Basic Shapes,
 *   Block Arrows, Equation Shapes, Flowchart, Stars and Banners, Callouts, and PowerPoint's Action Buttons; Word's
 *   New Drawing Canvas under them); `editShapeEntries(application)`, whose Change Shape submenu is
 *   `changeShapeEntries(application)`; `mergeShapesEntries()`, PowerPoint's Merge Shapes.
 * - **Shape Styles**: `shapeStyles`, the forty-nine styles, and `shapeStyleGalleryItems(palette)`;
 *   `otherThemeFillEntries()`, the menu the gallery's footer opens; `shapeEffectsEntries()`. Shape Fill and Shape
 *   Outline are colour pickers over the document's palette, which a host owns, with
 *   `stories/ribbons/colour-picker-entries.ts`' `fillEntries(shapeFillEntryOptions(application))` and
 *   `outlineEntries(shapeOutlineEntryOptions(application))` beneath them.
 * - **WordArt Styles is not here**: its commands are the census's `wordArtStylesCommands` and its gallery and Text
 *   Effects lists are `stories/ribbons/wordart-styles-menus.ts`'. The Text Effects menu is rendered here, because its
 *   id is Shape Format's.
 * - **Arrange is not here either**: its commands are `arrangeCommands` and its lists are
 *   `stories/ribbons/design-layout-menus.ts`'. The menus are rendered here, under Shape Format's ids.
 * - **Size**: `powerpointShapeMeasures` and `wordShapeMeasures`, the height and width each host starts its two fields
 *   on.
 *
 * `GUESS:` every label, order and preset below, from memory of Microsoft 365. Where a label differs from Office's
 * spelling the census's wins (*Coloured*, *Centre*).
 *
 * ## The pictures are the document's colours, not the chrome's
 *
 * A shape style is a fill, an outline and an effect named after one of the document's theme colours, so each
 * thumbnail is drawn in the document's `ThemeColorPalette`, through `stories/ribbons/palette-art.ts`, as static markup
 * for the reason the table, WordArt and picture thumbnails are: `<mjx-gallery-item>` clones its children into the
 * gallery's shadow root.
 *
 * **Nothing here dispatches a command.** A menu opens, an entry can be chosen, a gallery previews and commits; no
 * shape changes, because command dispatch is loop 2.
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
import { paletteSlotColour, spacingStep } from './palette-art.ts';
import { cropShapes, pictureEffectsEntries } from './picture-tools-menus.ts';
import { commandMenu } from './ribbon-parts.ts';
import { accentNames, submenu, wordArtTextEffectsEntries } from './wordart-styles-menus.ts';

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

/** `Coloured Fill - Blue, Accent 1` → `coloured-fill-blue-accent-1`. */
function slug(label: string): string {
  return label.toLowerCase().replaceAll(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

// ── Insert Shapes: the shape gallery ─────────────────────────────────────────

/** One section of the shape gallery, by Office's heading, and its shapes by Office's names. */
export interface ShapeGallerySection {
  readonly section: string;
  readonly shapes: readonly string[];
}

/**
 * **Lines' twelve**, in the gallery's order: the straight lines, the six connectors, then the three drawn by hand.
 * Only the Shapes gallery has them; a closed shape cannot become a line, so Change Shape leaves them out.
 */
export const lineShapes: readonly string[] = [
  'Line',
  'Line Arrow',
  'Line Arrow: Double',
  'Connector: Elbow',
  'Connector: Elbow Arrow',
  'Connector: Elbow Double-Arrow',
  'Connector: Curved',
  'Connector: Curved Arrow',
  'Connector: Curved Double-Arrow',
  'Curve',
  'Freeform: Shape',
  'Freeform: Scribble',
];

/** **The two text boxes** Office puts at the head of Basic Shapes in the Shapes gallery, and nowhere else. */
export const textBoxShapes: readonly string[] = ['Text Box', 'Vertical Text Box'];

/**
 * **PowerPoint's twelve action buttons**, the gallery's last section: a shape that carries a click action. Only a deck
 * plays, so Word's and Excel's galleries have no such section.
 */
export const actionButtonShapes: readonly string[] = [
  'Go Back or Previous',
  'Go Forward or Next',
  'Go to Beginning',
  'Go to End',
  'Go Home',
  'Get Information',
  'Go Back',
  'Video',
  'Document',
  'Sound',
  'Help',
  'Blank',
].map((action) => `Action Button: ${action}`);

/**
 * **The shape gallery's sections, for one application and one purpose.**
 *
 * - **`insert`** (the Shapes gallery, on Insert and on Shape Format): Lines, then `picture-tools-menus.ts`'
 *   `cropShapes` with the two text boxes at the head of Basic Shapes, then Action Buttons in PowerPoint.
 * - **`change`** (Edit Shape ▸ Change Shape): `cropShapes` as they stand, then Action Buttons in PowerPoint: no Lines,
 *   because a closed shape does not become a line, and no text box, because a text box is a rectangle with no fill.
 *
 * **No *Recently Used Shapes* section.** Office leads the Shapes gallery with the shapes this machine drew last; a
 * catalogue that invented some would claim to know what somebody drew, for the reason Insert's Screenshot lists no
 * windows. `GUESS:` every name, each section's order, and what each purpose leaves out.
 */
export function shapeGallerySections(
  application: RibbonApplication,
  purpose: 'insert' | 'change',
): readonly ShapeGallerySection[] {
  const shared = cropShapes.map((group) =>
    purpose === 'insert' && group.section === 'Basic Shapes'
      ? { section: group.section, shapes: [...textBoxShapes, ...group.shapes] }
      : group,
  );
  return [
    ...(purpose === 'insert' ? [{ section: 'Lines', shapes: lineShapes }] : []),
    ...shared,
    ...(application === 'powerpoint' ? [{ section: 'Action Buttons', shapes: actionButtonShapes }] : []),
  ];
}

/** One run of menu sections, one entry per shape. */
function shapeSectionEntries(sections: readonly ShapeGallerySection[]): TemplateResult[] {
  return sections.map((group) => section(group.section, ...group.shapes.map((shape) => item(shape))));
}

/**
 * **The Shapes gallery, as a menu**: every section `shapeGallerySections(application, 'insert')` gives, and Word's
 * **New Drawing Canvas** under them. Office draws each shape as its outline; a menu of names is the catalogue's shape
 * for a gallery of pictures, as Picture Format's Corrections is. Insert's Shapes and Shape Format's Shapes both open
 * it.
 */
export function insertShapesEntries(application: RibbonApplication): TemplateResult[] {
  return [
    ...shapeSectionEntries(shapeGallerySections(application, 'insert')),
    ...(application === 'word' ? [separator(), item('New Drawing Canvas')] : []),
  ];
}

/** **Change Shape ▸**: every section `shapeGallerySections(application, 'change')` gives. */
export function changeShapeEntries(application: RibbonApplication): TemplateResult[] {
  return shapeSectionEntries(shapeGallerySections(application, 'change'));
}

/**
 * **Edit Shape's menu**: Change Shape ▸, Edit Points, then Reroute Connectors, **unavailable**, because it acts on a
 * connector and the selection is a closed shape. The same three in all three applications. `GUESS:` that Reroute
 * Connectors is shown unavailable rather than hidden.
 */
export function editShapeEntries(application: RibbonApplication): TemplateResult[] {
  return [
    submenu('Change Shape', ...changeShapeEntries(application)),
    item('Edit Points'),
    html`<mjx-menu-item
      label="Reroute Connectors"
      unavailable
      explanation="Select a connector to reroute it."
    ></mjx-menu-item>`,
  ];
}

/**
 * **Merge Shapes' menu**: Office's five operations on two or more selected shapes, in Office's order. Nothing is
 * checked: a merge is a verb, not a state.
 */
export function mergeShapesEntries(): TemplateResult[] {
  return [item('Union'), item('Combine'), item('Fragment'), item('Intersect'), item('Subtract')];
}

/**
 * **Draw Text Box's arrow, Word's**: *Draw Text Box* and *Draw Vertical Text Box*, the two text boxes the Shapes
 * gallery leads Basic Shapes with, under the names Word's split button gives them. Nothing is checked: each arms a
 * drawing gesture. PowerPoint's Text Box is a plain button and has no such menu. `GUESS:` both labels.
 */
export function drawTextBoxEntries(): TemplateResult[] {
  return [item('Draw Text Box'), item('Draw Vertical Text Box')];
}

// ── Text: Word's alone ───────────────────────────────────────────────────────

/** One of a set, radio-checked. */
function choice(label: string, checked = false): TemplateResult {
  return html`<mjx-menu-item kind="radio" label=${label} ?checked=${checked}></mjx-menu-item>`;
}

/**
 * **Text Direction's menu, Word's, for a text box or a shape's text**: *Horizontal* (checked, an inserted text box's
 * direction), *Rotate all text 90°*, *Rotate all text 270°*, then *Text Direction Options…*. **No *Stacked***, which
 * PowerPoint's list (`table-tools-menus.ts`' `powerpointTextDirectionEntries`) carries and Word's text boxes do not, so
 * that list is not reused. `GUESS:` every label, that Stacked is absent, and the dialog entry's name.
 */
export function wordTextDirectionEntries(): TemplateResult[] {
  return [
    choice('Horizontal', true),
    choice('Rotate all text 90°'),
    choice('Rotate all text 270°'),
    separator(),
    item('Text Direction Options…'),
  ];
}

/**
 * **Align Text's menu**: *Top* (checked, where an inserted text box anchors its text), *Middle* and *Bottom*, one set.
 * `GUESS:` that Top is where it starts, and that Word's menu holds these three and no options entry.
 */
export function alignTextEntries(): TemplateResult[] {
  return [choice('Top', true), choice('Middle'), choice('Bottom')];
}

// ── Shape Styles: the menus and the picker entries ───────────────────────────

/**
 * **Shape Effects' menu**: Preset, Shadow, Reflection, Glow, Soft Edges, Bevel and 3-D Rotation, each a submenu with
 * Office's whole list. **It is Picture Effects' menu**, because Office offers a shape exactly the seven lists it offers a
 * picture, so it calls `pictureEffectsEntries` rather than restating seven submenus. `GUESS:` that the lists are the
 * same.
 */
export function shapeEffectsEntries(): TemplateResult[] {
  return pictureEffectsEntries();
}

/**
 * **What sits beneath Shape Fill's palette**: More Fill Colours…, then Eyedropper in PowerPoint, then Picture…,
 * Gradient ▸ and Texture ▸. `GUESS:` that Word's and Excel's carry no Eyedropper, as their Picture Border carries
 * none.
 */
export function shapeFillEntryOptions(application: RibbonApplication): FillEntryOptions {
  return {
    moreColours: 'More Fill Colours…',
    eyedropper: application === 'powerpoint',
    picture: true,
    gradient: true,
    texture: true,
  };
}

/**
 * **What sits beneath Shape Outline's palette**: More Outline Colours…, then Eyedropper in PowerPoint, then Weight ▸,
 * Sketched ▸, Dashes ▸ and **Arrows ▸**, which only a shape's outline has: text has no ends. `GUESS:` as Shape Fill's.
 */
export function shapeOutlineEntryOptions(application: RibbonApplication): OutlineEntryOptions {
  return {
    moreColours: 'More Outline Colours…',
    eyedropper: application === 'powerpoint',
    weight: true,
    sketched: true,
    dashes: true,
    arrows: true,
  };
}

/**
 * **Other Theme Fills ▸, the menu under the Theme Styles gallery**: the theme's twelve background fills, *Style 1* to
 * *Style 12*, the same twelve Design's Background Styles lists. Nothing is checked: a shape wears a style's fill, not a
 * background style. `GUESS:` that the submenu holds these twelve and nothing else.
 */
export function otherThemeFillEntries(): TemplateResult[] {
  return [section('Other Theme Fills', ...Array.from({ length: 12 }, (_, index) => item(`Style ${String(index + 1)}`)))];
}

// ── Shape Styles: the gallery ────────────────────────────────────────────────

/** A shape style's family: one row of Office's gallery. */
export type ShapeStyleFamily = 'outline' | 'fill' | 'light-outline' | 'subtle' | 'moderate' | 'intense' | 'transparent';

/** One shape style: its value, Office's name for it, its family, the document colour it is named for, its section. */
export interface ShapeStyleSpec {
  readonly value: string;
  readonly label: string;
  readonly family: ShapeStyleFamily;
  readonly slot: ThemeColorSlot;
  readonly category: 'Theme Styles' | 'Presets';
}

/**
 * **One shape style's thumbnail**: a rectangle holding *Abc*, filled, outlined and given its family's effect in the
 * document's colours, on the document's paper, as static markup.
 *
 * - **Coloured Outline**: Background 1 inside a line of the colour, text in Text 1.
 * - **Coloured Fill**: the colour, a darker line of it, text in Background 1.
 * - **Light 1 Outline, Coloured Fill**: the colour inside a Background 1 line, text in Background 1.
 * - **Subtle Effect**: a light tint of the colour inside a line of it, text in Text 1.
 * - **Moderate Effect**: the colour lightening towards the top, no line, a soft shadow.
 * - **Intense Effect**: the colour darkening towards the bottom, a Background 1 line, a bevel and a deeper shadow.
 * - **Transparent, Coloured Outline**: nothing inside a line of the colour, the paper showing through as a
 *   chequerboard so it is told from Coloured Outline's Background 1.
 *
 * ⚠ **Static for the reason every gallery picture is**, and **every colour is a checked hex colour or a token**,
 * through `paletteSlotColour`. The thumbnail's height matches the WordArt, table and picture style pictures'. `GUESS:`
 * every look: Office's thumbnails are drawn from each style's `a:style` references into the theme's format scheme,
 * and these are a description of a look rather than a render of one.
 */
export function shapeStylePicture(family: ShapeStyleFamily, slot: ThemeColorSlot, palette: ThemeColorPalette): TemplateResult {
  const step = spacingStep;
  const colour = paletteSlotColour(palette, slot);
  const paper = paletteSlotColour(palette, 'background1');
  const ink = paletteSlotColour(palette, 'text1');
  const line = step(0.125);
  const shade = `color-mix(in srgb, ${ink} 40%, transparent)`;
  const looks: Record<ShapeStyleFamily, string> = {
    outline: `background:${paper};border:${line} solid ${colour};color:${ink}`,
    fill: `background:${colour};border:${line} solid color-mix(in srgb, ${colour} 70%, ${ink});color:${paper}`,
    'light-outline': `background:${colour};border:${line} solid ${paper};outline:1px solid color-mix(in srgb, ${colour} 60%, ${paper});color:${paper}`,
    subtle: `background:color-mix(in srgb, ${colour} 20%, ${paper});border:${line} solid ${colour};color:${ink}`,
    moderate:
      `background:linear-gradient(to bottom, color-mix(in srgb, ${colour} 65%, ${paper}), ${colour});` +
      `box-shadow:0 ${step(0.125)} ${step(0.25)} ${shade};color:${paper}`,
    intense:
      `background:linear-gradient(to bottom, ${colour}, color-mix(in srgb, ${colour} 70%, ${ink}));border:${line} solid ${paper};` +
      `box-shadow:inset 0 ${step(0.125)} 0 color-mix(in srgb, ${paper} 45%, transparent), 0 ${step(0.25)} ${step(0.375)} ${shade};color:${paper}`,
    transparent:
      `background:repeating-conic-gradient(color-mix(in srgb, ${ink} 12%, transparent) 0 25%, transparent 0 50%) 0 0 / ${step(0.5)} ${step(0.5)};` +
      `border:${line} solid ${colour};color:${ink}`,
  };
  const markup =
    `<span style="display:grid;place-items:center;inline-size:100%;block-size:${step(6.25)};background:${paper};overflow:hidden">` +
    `<span style="display:grid;place-items:center;box-sizing:border-box;inline-size:70%;block-size:${step(3.5)};` +
    `font-size:${step(1.5)};line-height:1;${looks[family]}">Abc</span></span>`;
  return staticHtml`${unsafeStatic(markup)}`;
}

/** The seven colours a shape style is named for: Dark 1, then the six accents, under the Office theme's names. */
const shapeStyleColours: readonly { readonly slot: ThemeColorSlot; readonly name: string }[] = [
  { slot: 'text1', name: 'Black, Dark 1' },
  ...accentNames.map((accent) => ({ slot: accent.slot, name: accent.name.replace('Accent colour', 'Accent') })),
];

/** The six Theme Styles rows, top to bottom, by the words Office's names start with. */
const themeStyleFamilies: readonly { readonly family: ShapeStyleFamily; readonly name: string }[] = [
  { family: 'outline', name: 'Coloured Outline' },
  { family: 'fill', name: 'Coloured Fill' },
  { family: 'light-outline', name: 'Light 1 Outline, Coloured Fill' },
  { family: 'subtle', name: 'Subtle Effect' },
  { family: 'moderate', name: 'Moderate Effect' },
  { family: 'intense', name: 'Intense Effect' },
];

/** One style, its value derived from its name. */
function shapeStyle(
  family: ShapeStyleFamily,
  familyName: string,
  colour: { readonly slot: ThemeColorSlot; readonly name: string },
  category: ShapeStyleSpec['category'],
): ShapeStyleSpec {
  const label = `${familyName} - ${colour.name}`;
  return { value: slug(label), label, family, slot: colour.slot, category };
}

/**
 * **PowerPoint's forty-nine shape styles**, the whole Theme Styles gallery: *Theme Styles*, six rows of seven (Coloured
 * Outline, Coloured Fill, Light 1 Outline Coloured Fill, Subtle Effect, Moderate Effect and Intense Effect, each for
 * Dark 1 and the six accents), then *Presets*, one row of seven (Transparent, Coloured Outline). Shared by all three
 * applications' Shape Styles, which Office draws from the same theme.
 *
 * `GUESS:` the names (*Coloured* in the census's spelling, the colours under the Office theme's names, as WordArt's
 * are), the order, and that Presets holds exactly one row.
 */
export const shapeStyles: readonly ShapeStyleSpec[] = [
  ...themeStyleFamilies.flatMap((row) =>
    shapeStyleColours.map((colour) => shapeStyle(row.family, row.name, colour, 'Theme Styles')),
  ),
  ...shapeStyleColours.map((colour) => shapeStyle('transparent', 'Transparent, Coloured Outline', colour, 'Presets')),
];

/**
 * **The Theme Styles gallery's items**, one `<mjx-gallery-item>` per style, drawn in the document's palette, in two
 * sections. A host sets no starting value: Office's own inserted shape wears no named style from this gallery, only
 * the theme's default. The footer, *Other Theme Fills*, is written by each host, because it opens
 * `otherThemeFillEntries()` through a `data-opens` the gate reads in the host's own source.
 */
export function shapeStyleGalleryItems(palette: ThemeColorPalette): TemplateResult[] {
  return shapeStyles.map(
    (style) => html`<mjx-gallery-item value=${style.value} label=${style.label} category=${style.category}
      >${shapeStylePicture(style.family, style.slot, palette)}</mjx-gallery-item
    >`,
  );
}

// ── Size ─────────────────────────────────────────────────────────────────────

/**
 * **The height and width a PowerPoint host starts Size's two fields on**, in centimetres: a shape PowerPoint inserts
 * with one click rather than a drag, one inch square. `GUESS:` both numbers and the 0.01 cm step.
 */
export const powerpointShapeMeasures = { height: '2.54', width: '2.54', step: '0.01' } as const;

/**
 * **The height and width a Word host starts Size's two fields on**, in centimetres: a shape Word inserts with one click,
 * one inch square, as PowerPoint's. Its own constant because Word's measures are Word's, as `wordPictureMeasures` is
 * beside PowerPoint's. `GUESS:` both numbers and the 0.01 cm step.
 */
export const wordShapeMeasures = { height: '2.54', width: '2.54', step: '0.01' } as const;

// ── what a host renders ──────────────────────────────────────────────────────

/**
 * PowerPoint's eleven menus. **Insert Shapes' three**: Shapes (the whole gallery, with Action Buttons), Edit Shape
 * (Change Shape ▸, Edit Points, Reroute Connectors) and Merge Shapes. **Shape Styles' two**: Other Theme Fills, which
 * the Theme Styles gallery's footer opens and whose id is therefore the gallery's, and Shape Effects. **WordArt Styles'
 * one**: Text Effects, `stories/ribbons/wordart-styles-menus.ts`' list. **Arrange's five**, over
 * `stories/ribbons/design-layout-menus.ts`' PowerPoint lists, as Picture Format's. Every other Shape Format command is
 * a field, a picker, a gallery, a toggle or a plain button.
 */
function powerpointDrawingToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'powerpoint.shape-format.insert-shapes.shapes', 'Shapes', ...insertShapesEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.shape-format.insert-shapes.edit-shape', 'Edit Shape', ...editShapeEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.shape-format.insert-shapes.merge-shapes', 'Merge Shapes', ...mergeShapesEntries())}
    ${commandMenu(host, 'powerpoint.shape-format.shape-styles.theme-styles', 'Other Theme Fills', ...otherThemeFillEntries())}
    ${commandMenu(host, 'powerpoint.shape-format.shape-styles.shape-effects', 'Shape Effects', ...shapeEffectsEntries())}
    ${commandMenu(host, 'powerpoint.shape-format.wordart-styles.text-effects', 'Text Effects', ...wordArtTextEffectsEntries())}
    ${commandMenu(host, 'powerpoint.shape-format.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.shape-format.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.shape-format.arrange.align', 'Align', ...alignEntries('powerpoint'))}
    ${commandMenu(host, 'powerpoint.shape-format.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'powerpoint.shape-format.arrange.rotate', 'Rotate', ...rotateEntries())}
  `;
}

/**
 * Word's fifteen menus. **Insert Shapes' three**: Shapes (the whole gallery, with New Drawing Canvas and no Action
 * Buttons), Edit Shape, and **Draw Text Box's arrow**, where PowerPoint has Merge Shapes. **Shape Styles' two** and
 * **WordArt Styles' one**, as PowerPoint's. **Text's two**: Text Direction and Align Text. **Arrange's seven**, over
 * `stories/ribbons/design-layout-menus.ts`' Word lists, as Word's Picture Format: Position and Wrap Text, then the five
 * PowerPoint's has. Every other Shape Format command is a field, a picker, a gallery, a toggle or a plain button.
 */
function wordDrawingToolsMenus(host: RibbonSurfaceHost): TemplateResult {
  return html`
    ${commandMenu(host, 'word.shape-format.insert-shapes.shapes', 'Shapes', ...insertShapesEntries('word'))}
    ${commandMenu(host, 'word.shape-format.insert-shapes.edit-shape', 'Edit Shape', ...editShapeEntries('word'))}
    ${commandMenu(host, 'word.shape-format.insert-shapes.draw-text-box', 'Draw Text Box', ...drawTextBoxEntries())}
    ${commandMenu(host, 'word.shape-format.shape-styles.theme-styles', 'Other Theme Fills', ...otherThemeFillEntries())}
    ${commandMenu(host, 'word.shape-format.shape-styles.shape-effects', 'Shape Effects', ...shapeEffectsEntries())}
    ${commandMenu(host, 'word.shape-format.wordart-styles.text-effects', 'Text Effects', ...wordArtTextEffectsEntries())}
    ${commandMenu(host, 'word.shape-format.text.text-direction', 'Text Direction', ...wordTextDirectionEntries())}
    ${commandMenu(host, 'word.shape-format.text.align-text', 'Align Text', ...alignTextEntries())}
    ${commandMenu(host, 'word.shape-format.arrange.position', 'Position', ...positionEntries())}
    ${commandMenu(host, 'word.shape-format.arrange.wrap-text', 'Wrap Text', ...wrapTextEntries())}
    ${commandMenu(host, 'word.shape-format.arrange.bring-forward', 'Bring Forward', ...bringForwardEntries('word'))}
    ${commandMenu(host, 'word.shape-format.arrange.send-backward', 'Send Backward', ...sendBackwardEntries('word'))}
    ${commandMenu(host, 'word.shape-format.arrange.align', 'Align', ...alignEntries('word'))}
    ${commandMenu(host, 'word.shape-format.arrange.group', 'Group', ...groupEntries())}
    ${commandMenu(host, 'word.shape-format.arrange.rotate', 'Rotate', ...rotateEntries())}
  `;
}

/**
 * Every menu one application's Shape Format tab opens, with ids for one host's page. **PowerPoint's and Word's are
 * authored**; Excel's renders nothing until its unit, exactly as `pictureToolsMenus` rendered nothing before it. That
 * unit adds a branch here and calls the lists above with its application.
 *
 * Rendered once beside `<mjx-ribbon>`, floating and closed, by every host that draws Drawing Tools **and** binds its
 * commands: `Ribbons/PowerPoint` and `Ribbons/Word`. `Shell/PowerPoint` draws Picture Tools and `Shell/Word` Table
 * Tools, so neither renders these, and `tests/ribbons.test.ts` requires each application's menus of its `Ribbons/*`
 * host alone.
 */
export function drawingToolsMenus(application: RibbonApplication, host: RibbonSurfaceHost): TemplateResult {
  if (application === 'powerpoint') return powerpointDrawingToolsMenus(host);
  if (application === 'word') return wordDrawingToolsMenus(host);
  return html``;
}
