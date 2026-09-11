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
 * **Home** is the only authored tab; the rest are placeholders at the census's own priorities.
 * See `Ribbons/Word` for why that is the whole of unit 0 and why the placeholders say so on their
 * face.
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
          'ribbon. Home is authored; the rest are placeholders carrying the census’s priorities.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The catalogue's own bindings. See `Ribbons/Word` on why these are not the shell's. */
const bindings: ControlOverrides = {
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
    <mjx-button label="Arrange" icon="slide-layout"></mjx-button>
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

/** Unit 1. */
export const File: Story = { render: () => ribbon('file') };

/**
 * **Home, and the one tab that is real.** Five groups, migrated out of `Shell/PowerPoint`
 * unchanged. The census declares a sixth in-scope group — `GroupSlides` — which unit 2 authors.
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
