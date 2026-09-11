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
 * **File and Home** are authored; the rest are placeholders at the census's own priorities. See
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
          'ribbon. File and Home are authored; the rest are placeholders carrying the census’s ' +
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
    slot="essential"
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
 * and only three commands per group can draw pressed. What is PowerPoint's own:
 *
 * 1. **Slides arrives with this unit** — New Slide, Layout, Reset, Section — declared in the census
 *    since unit 0 and rendered by nothing, which meant this catalogue's PowerPoint had no way to
 *    add a slide. New Slide is the only `size="large"` command unit 2 adds anywhere.
 * 2. **Drawing is the census's largest Home group at 63 controls and draws six commands.** Sixty
 *    three is the shapes gallery's whole catalogue plus three effect menus and Arrange's fourteen
 *    entries; the face is Shapes, Arrange, the style gallery and the three shape formats. Shapes is
 *    the group's survivor, because a collapsed Drawing group has room for one verb.
 * 3. **Arrange is drawn with `layer` now, not `slide-layout`** — which was the *Layout* command's
 *    icon, one group to the left, on a button that means something else entirely.
 * 4. **Text Shadow carries no icon**, so it is the one labelled button in a row of glyphs. Fluent
 *    draws no shadowed letter, and both candidates already name other commands in this subset.
 */
export const Home: Story = { render: () => ribbon('home') };

/** Unit 3. */
export const Insert: Story = { render: () => ribbon('insert') };

/** Unit 4. */
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
