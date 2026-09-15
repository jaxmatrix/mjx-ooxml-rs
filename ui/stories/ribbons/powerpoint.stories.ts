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
  copyCounts,
  openDeclaredSurface,
  printerList,
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
import { insertMenus } from './insert-menus.ts';
import { powerpointContextualSets, powerpointTabs } from './powerpoint.ts';

/**
 * **PowerPoint's ribbon, tab by tab** — the same functions `Shell/PowerPoint` composes.
 *
 * Nineteen stories, because the census marks eighteen in-scope core tabs and decision 1 of the
 * approved plan makes File a nineteenth. **Eight of the eighteen are view tabs** — the two colour
 * modes, the four masters, Print Preview and Background Removal — which Office shows only inside
 * the view they name. The shell leaves them out and the catalogue does not, because a tab nobody
 * can look at cannot be audited.
 *
 * ⚠ **Two stories are called Home.** `Home` is the ordinary one; `SlideMasterHome` is Office's own
 * Home tab as it appears in Slide Master view, and they are never on screen together there. Here
 * they are, because every story renders every tab, and the duplicate label in the strip is the
 * catalogue's artefact rather than a transcription slip.
 *
 * **File, Home, Insert and Draw** are authored; the rest are placeholders at the census's own priorities. See
 * `Ribbons/Word` for why a placeholder says so on its face — and for what to look at on a File tab,
 * since the three are one tab with three sets of differences rather than three tabs.
 */

const conventions = storyConventions({
  statesMatrix: ribbonStatesMatrix,
  tokenDependencies: ribbonTokenDependencies,
  keyboard: ribbonKeyboard,
  screenReader: ribbonScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Ribbons/PowerPoint',
  parameters: {
    docs: {
      description: {
        component:
          'PowerPoint’s eighteen core tabs and its File tab, each shown selected inside the whole ' +
          'ribbon. File, Home, Insert and Draw are authored; the rest are placeholders carrying the ' +
          'census’s priorities.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The catalogue's own bindings. See `Ribbons/Word` on why these are not the shell's. */
const bindings: ControlOverrides = {
  'powerpoint.file.print.printer': html`<mjx-dropdown
    id="ribbons-powerpoint-printer"
    label="Printer"
    value="pdf"
    style=${ribbonFieldStyle}
  >
    ${printerList.map(
      (printer) => html`<mjx-option value=${printer.value} label=${printer.label}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'powerpoint.file.print.copies': html`<mjx-combo-box
    id="ribbons-powerpoint-copies"
    label="Copies"
    value="1"
    allow-custom
    style=${ribbonNarrowFieldStyle}
  >
    ${copyCounts.map((count) => html`<mjx-option value=${count} label=${count}></mjx-option>`)}
  </mjx-combo-box>`,
  'powerpoint.home.clipboard.paste': html`<mjx-split-button
    label="Paste"
    icon="clipboard-paste"
    size="large"
    menu-label="Paste options"
    data-opens="ribbons-ppt-paste"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.home.font.name': html`<mjx-font-picker
    id="ribbons-ppt-font"
    style=${ribbonFieldStyle}
    label="Font"
    value="Aptos"
    .fonts=${machineFonts}
  ></mjx-font-picker>`,
  'powerpoint.home.font.size': html`<mjx-dropdown
    id="ribbons-ppt-size"
    label="Font size"
    value="18"
    style=${ribbonNarrowFieldStyle}
  >
    ${['12', '14', '18', '24', '32', '44'].map(
      (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'powerpoint.home.font.colour': html`<mjx-color-picker
    id="ribbons-ppt-colour"
    style=${ribbonColourFieldStyle}
    label="Font colour"
    show-automatic
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'powerpoint.home.drawing.styles': html`<mjx-gallery
    id="ribbons-ppt-shape-styles"
    label="Shape styles"
    value="office-1"
    style=${ribbonGalleryStyle}
  >
    ${largeGalleryItems().slice(0, 24)}
  </mjx-gallery>`,
  'powerpoint.home.drawing.arrange': html`<mjx-screentip
    heading="Arrange"
    description="Change how the selected shapes overlap one another, and how they line up."
    shortcut="Alt + J D A"
  >
    <mjx-button label="Arrange" icon="layer"></mjx-button>
  </mjx-screentip>`,
  // Insert (unit 3). Office draws each of these as a dropdown or a split button, so each opens
  // its menu from `stories/ribbons/insert-menus.ts`: a dropdown is one `<mjx-button>` whose press
  // opens the menu, a split button opens it from its arrow. `data-opens` is
  // `commandSurfaceId('ribbons', <this key>)`, and `tests/ribbons.test.ts` requires exactly that.
  'powerpoint.insert.slides.new-slide': html`<mjx-split-button
    label="New Slide"
    icon="slide-add"
    size="large"
    data-opens="ribbons-powerpoint-insert-slides-new-slide"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.tables.table': html`<mjx-button
    label="Table"
    icon="table"
    size="large"
    data-opens="ribbons-powerpoint-insert-tables-table"
  ></mjx-button>`,
  'powerpoint.insert.images.pictures': html`<mjx-button
    label="Pictures"
    icon="image"
    size="large"
    data-opens="ribbons-powerpoint-insert-images-pictures"
  ></mjx-button>`,
  'powerpoint.insert.images.screenshot': html`<mjx-button
    label="Screenshot"
    icon="screenshot"
    size="large"
    data-opens="ribbons-powerpoint-insert-images-screenshot"
  ></mjx-button>`,
  'powerpoint.insert.images.photo-album': html`<mjx-split-button
    label="Photo Album"
    icon="image-multiple"
    size="large"
    data-opens="ribbons-powerpoint-insert-images-photo-album"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.illustrations.shapes': html`<mjx-button
    label="Shapes"
    icon="shapes"
    size="large"
    data-opens="ribbons-powerpoint-insert-illustrations-shapes"
  ></mjx-button>`,
  'powerpoint.insert.illustrations.3d-models': html`<mjx-button
    label="3D Models"
    icon="cube"
    size="large"
    data-opens="ribbons-powerpoint-insert-illustrations-3d-models"
  ></mjx-button>`,
  'powerpoint.insert.camera.cameo': html`<mjx-split-button
    label="Cameo"
    icon="camera"
    size="large"
    data-opens="ribbons-powerpoint-insert-camera-cameo"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.links.zoom': html`<mjx-button
    label="Zoom"
    size="small"
    data-opens="ribbons-powerpoint-insert-links-zoom"
  ></mjx-button>`,
  'powerpoint.insert.links.link': html`<mjx-split-button
    label="Link"
    icon="link"
    size="large"
    data-opens="ribbons-powerpoint-insert-links-link"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.text.wordart': html`<mjx-button
    label="WordArt"
    icon="text-effects"
    size="large"
    data-opens="ribbons-powerpoint-insert-text-wordart"
  ></mjx-button>`,
  'powerpoint.insert.symbols.equation': html`<mjx-split-button
    label="Equation"
    icon="math-formula"
    size="large"
    data-opens="ribbons-powerpoint-insert-symbols-equation"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.insert.media-clips.video': html`<mjx-button
    label="Video"
    icon="video"
    size="large"
    data-opens="ribbons-powerpoint-insert-media-clips-video"
  ></mjx-button>`,
  'powerpoint.insert.media-clips.audio': html`<mjx-button
    label="Audio"
    icon="speaker-2"
    size="large"
    data-opens="ribbons-powerpoint-insert-media-clips-audio"
  ></mjx-button>`,
  // Draw (unit 4): Word's six bindings, over `stories/ribbons/draw-menus.ts`. See `Ribbons/Word`.
  'powerpoint.draw.drawing-tools.add-pen': html`<mjx-button
    label="Add Pen"
    size="small"
    data-opens="ribbons-powerpoint-draw-drawing-tools-add-pen"
  ></mjx-button>`,
  'powerpoint.draw.pens.pens': html`<mjx-button
    label="Pens"
    icon="inking-tool"
    size="large"
    data-opens="ribbons-powerpoint-draw-pens-pens"
  ></mjx-button>`,
  'powerpoint.draw.pens.colour': html`<mjx-button
    label="Colour"
    icon="color-line"
    size="small"
    data-opens="ribbons-powerpoint-draw-pens-colour"
  ></mjx-button>`,
  'powerpoint.draw.pens.thickness': html`<mjx-button
    label="Thickness"
    icon="line-thickness"
    size="small"
    data-opens="ribbons-powerpoint-draw-pens-thickness"
  ></mjx-button>`,
  'powerpoint.draw.write.eraser': html`<mjx-split-button
    label="Eraser"
    icon="eraser"
    size="large"
    data-opens="ribbons-powerpoint-draw-write-eraser"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'powerpoint.draw.input-mode.touch-mouse-mode': html`<mjx-button
    label="Touch/Mouse Mode"
    size="small"
    data-opens="ribbons-powerpoint-draw-input-mode-touch-mouse-mode"
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
    <mjx-ribbon label="PowerPoint" selected=${selected} @mjx-activate=${openDeclaredSurface}>
      ${powerpointTabs({ controls: bindings, includeViewTabs: true })}
      ${powerpointContextualSets()}
    </mjx-ribbon>
    <mjx-menu id="ribbons-ppt-paste" label="Paste options" floating>
      <mjx-menu-section label="Paste">
        <mjx-menu-item kind="radio" label="Use Destination Theme" checked></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Keep Source Formatting"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Picture"></mjx-menu-item>
      </mjx-menu-section>
      <mjx-menu-separator></mjx-menu-separator>
      <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
    </mjx-menu>

    ${insertMenus('powerpoint', 'ribbons')} ${drawMenus('powerpoint', 'ribbons')}
  `;
}

/**
 * **File** — the same seven groups as Word's, and the one tab where PowerPoint's File page visibly
 * differs from the other two.
 *
 * **Export** carries *Create a Video*, *Package Presentation for CD* and *Create Handouts*: a deck
 * is the only document that can be played and the only one with a second shape to be printed in.
 * **Share** carries *Publish Slides*, which sends slides to a library one at a time rather than the
 * deck as a file.
 *
 * Two of those five carry **no icon** — Package Presentation for CD and Create Handouts — because
 * Fluent draws no CD at 20px and a handout is not a landscape page. That is the rule rather than an
 * oversight: a wrong icon is worse than a missing one, because a person acts on it.
 *
 * See `Ribbons/Word → File` for the collapse order, which is the same here.
 */
export const File: Story = { render: () => ribbon('file') };

/**
 * **Home** — six groups, and the ribbon programme's unit 2. See `Ribbons/Word → Home` for the two
 * rules that shape every Home tab: most of it is drawn icon-only because Office draws it that way,
 * and every state draws pressed while only a declared three per group survive a collapse. What is
 * PowerPoint's own:
 *
 * 1. **Slides arrives with this unit** — New Slide, Layout, Reset, Section — declared in the census
 *    since unit 0 and rendered by nothing, which meant this catalogue's PowerPoint had no way to
 *    add a slide. New Slide is the only `size="large"` command unit 2 adds anywhere.
 * 2. **Drawing is the census's largest Home group at 63 controls and draws six commands.** Sixty
 *    three is the shapes gallery's whole catalogue plus three effect menus and Arrange's fourteen
 *    entries; the face is Shapes, Arrange, the style gallery and the three shape formats. **It keeps
 *    no survivor**: all six open a gallery or a menu, and unit 2's survivor, Shapes, put a gallery
 *    inside the collapsed popup. Slides and Editing keep none for the same reason.
 * 3. **Arrange is drawn with `layer` now, not `slide-layout`** — which was the *Layout* command's
 *    icon, one group to the left, on a button that means something else entirely.
 * 4. **Text Shadow carries no icon**, so it is the one labelled toggle in a row of glyphs. Fluent
 *    draws no shadowed letter, and both candidates already name other commands in this subset. It
 *    draws pressed on shadowed text, as Strikethrough and Justify now do too, and it can never be a
 *    survivor: a survivor has no room for a label.
 */
export const Home: Story = { render: () => ribbon('home') };

/**
 * **Insert** — eleven groups, the most of any application's Insert tab, and the ribbon programme's
 * unit 3. See `Ribbons/Word → Insert` for the two rules that shape every Insert tab: a command that
 * opens something opens a real menu, and no group keeps a survivor because nearly every command
 * opens a surface. What is PowerPoint's own:
 *
 * 1. **A deck splits Word's Illustrations in two.** Pictures, Screenshot and Photo Album are the
 *    Images group, and Shapes, Icons, 3D Models, SmartArt and Chart are Illustrations — all large,
 *    where Word draws three of its seven small. Camera (Cameo) and Media (Video, Audio, Screen
 *    Recording) exist in no other application.
 * 2. **Fourteen of the twenty-eight commands open a menu.** New Slide's arrow is the layout list, as
 *    it is on Home; Zoom opens Summary, Section and Slide Zoom. **Text Box does not**: in PowerPoint
 *    it arms a drawing gesture rather than opening a gallery, so it is the plain button Office draws
 *    — and the gesture is also why it is not a survivor. **Symbol does not either**, although Word's
 *    does: PowerPoint's opens the Symbol dialog directly.
 * 3. **Two group labels are the census's and not Office's.** Office calls the media group *Media*;
 *    the census's `GroupInsertMediaClips` gives *Media Clips*. And **Content** is one control the
 *    census names without describing — it carries its own label and no icon, exactly as Excel's Power
 *    Options does on Home, and `dev/ribbons/census.ts` records what is and is not known.
 * 4. **Camera's and Content's positions are `GUESS:`** — each is drawn where the declaration puts it.
 * 5. **Five commands carry no icon**: Reuse Slides, Zoom, Object, Symbol and Content. Zoom is the one
 *    to look at: Fluent's magnifier is the status bar's view zoom, a different command.
 */
export const Insert: Story = { render: () => ribbon('insert') };

/**
 * **Draw**: nine groups and the ribbon programme's unit 4. See `Ribbons/Word → Draw` for what shapes
 * every Draw tab: two generations of Office's ink tools on one tab, one survivor (Select Objects),
 * and tools that draw pressed but do not yet release each other. What is PowerPoint's own:
 *
 * 1. **Word's tab without two groups.** Editing (Ink Editor) and Drawing Canvas are Word's alone, so
 *    the strip goes straight from Stencils to Input Mode.
 * 2. **Every command is declared by the same function as Word's.** The census counts seven controls
 *    in Pens where Word counts six, and the same three commands are drawn. Nothing is padded to the
 *    count.
 * 3. **The same six commands open something**, and Eraser is the same split button.
 */
export const Draw: Story = { render: () => ribbon('draw') };

/** Unit 5. */
export const Design: Story = { render: () => ribbon('design') };

/** Unit 6. */
export const Transitions: Story = { render: () => ribbon('transitions') };

/** Unit 7. */
export const Animations: Story = { render: () => ribbon('animations') };

/** Unit 10. */
export const SlideShow: Story = { render: () => ribbon('slide-show') };

/** Unit 10. */
export const Recording: Story = { render: () => ribbon('recording') };

/** Unit 8. */
export const Review: Story = { render: () => ribbon('review') };

/** Unit 9. */
export const View: Story = { render: () => ribbon('view') };

/** Unit 10, and a view tab: Office shows it only in Slide Master view. */
export const SlideMaster: Story = { render: () => ribbon('slide-master') };

/** Unit 10, and the *second* tab called Home — see this file's header. */
export const SlideMasterHome: Story = { render: () => ribbon('slide-master-home') };

/** Unit 10, and a view tab. */
export const HandoutMaster: Story = { render: () => ribbon('handout-master') };

/** Unit 10, and a view tab. */
export const NotesMaster: Story = { render: () => ribbon('notes-master') };

/** A view tab: Office shows it only while a deck is being previewed in black and white. */
export const BlackAndWhite: Story = { render: () => ribbon('black-and-white') };

/** A view tab, and the census's `TabGrayscale` under this catalogue's spelling. */
export const Greyscale: Story = { render: () => ribbon('greyscale') };

/** Unit 10, and a view tab. */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/** A view tab: Office shows it only while a picture's background is being removed. */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };
