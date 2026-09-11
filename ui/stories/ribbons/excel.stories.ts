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
import { excelContextualSets, excelTabs } from './excel.ts';

/**
 * **Excel's ribbon, tab by tab** — the same functions `Shell/Excel` composes.
 *
 * ⚠ **Excel's File tab has no Print group**, and that is the census rather than an omission: the
 * committed command surface carries a backstage `TabPrint` row for Word and PowerPoint and none for
 * Excel, and carries a `Publish2Tab` the other two lack. Excel obviously has a File → Print page,
 * so this is a gap in the dump — but the census is the checked source and inventing the row would
 * be the drift the transcription exists to prevent. `dev/ribbons/census.ts` is where it is recorded.
 *
 * **File and Home** are authored; the rest are placeholders at the census's own priorities. See
 * `Ribbons/Word → File` for what to look at on a File tab — the three are one tab with three sets
 * of differences rather than three tabs.
 */

const conventions = storyConventions({
  statesMatrix: ribbonStatesMatrix,
  tokenDependencies: ribbonTokenDependencies,
  keyboard: ribbonKeyboard,
  screenReader: ribbonScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Ribbons/Excel',
  parameters: {
    docs: {
      description: {
        component:
          'Excel’s ten core tabs and its File tab, each shown selected inside the whole ribbon. ' +
          'File and Home are authored; the rest are placeholders carrying the census’s priorities.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The catalogue's own bindings. See `Ribbons/Word` on why these are not the shell's. */
const bindings: ControlOverrides = {
  'excel.home.clipboard.paste': html`<mjx-split-button
    slot="essential"
    label="Paste"
    icon="clipboard-paste"
    size="large"
    menu-label="Paste options"
    data-opens="ribbons-xl-paste"
    @mjx-menu-request=${openDeclaredSurface}
  ></mjx-split-button>`,
  'excel.home.font.name': html`<mjx-font-picker
    id="ribbons-xl-font"
    style=${ribbonFieldStyle}
    label="Font"
    value="Aptos"
    .fonts=${machineFonts}
  ></mjx-font-picker>`,
  'excel.home.font.size': html`<mjx-dropdown
    id="ribbons-xl-size"
    label="Font size"
    value="11"
    style=${ribbonNarrowFieldStyle}
  >
    ${['9', '10', '11', '12', '14', '18'].map(
      (size) => html`<mjx-option value=${size} label=${size}></mjx-option>`,
    )}
  </mjx-dropdown>`,
  'excel.home.font.fill': html`<mjx-color-picker
    id="ribbons-xl-fill"
    style=${ribbonColourFieldStyle}
    label="Fill colour"
    show-no-fill
    .themePalette=${documentThemePalette}
    .standardColors=${standardColors}
    .recentColors=${recentColors}
  ></mjx-color-picker>`,
  'excel.home.number.format': html`<mjx-dropdown
    id="ribbons-xl-number"
    label="Number format"
    value="general"
    style=${ribbonColourFieldStyle}
  >
    ${[
      { value: 'general', label: 'General' },
      { value: 'number', label: 'Number' },
      { value: 'currency', label: 'Currency' },
      { value: 'accounting', label: 'Accounting' },
      { value: 'percentage', label: 'Percentage' },
      { value: 'date', label: 'Short Date' },
    ].map((format) => html`<mjx-option value=${format.value} label=${format.label}></mjx-option>`)}
  </mjx-dropdown>`,
  'excel.home.styles.gallery': html`<mjx-gallery
    id="ribbons-xl-cell-styles"
    label="Cell styles"
    value="office-2"
    style=${ribbonGalleryStyle}
  >
    ${largeGalleryItems().slice(0, 18)}
  </mjx-gallery>`,
};

/**
 * The whole ribbon with one tab selected, and the paste menu the Home tab opens.
 *
 * ⚠ **No `<mjx-resizable-container>` of its own** — see `Ribbons/Word` for why a second one would
 * put a duplicate width control above every tab in this section.
 */
function ribbon(selected: string): TemplateResult {
  return html`
    <mjx-ribbon label="Excel" selected=${selected} @mjx-activate=${openDeclaredSurface}>
      ${excelTabs({ controls: bindings, includeViewTabs: true })} ${excelContextualSets()}
    </mjx-ribbon>
    <mjx-menu id="ribbons-xl-paste" label="Paste options" floating>
      <mjx-menu-section label="Paste">
        <mjx-menu-item kind="radio" label="All" checked></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Values"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Formats"></mjx-menu-item>
        <mjx-menu-item kind="radio" label="Transpose"></mjx-menu-item>
      </mjx-menu-section>
      <mjx-menu-separator></mjx-menu-separator>
      <mjx-menu-item label="Paste Special…" shortcut="Ctrl+Alt+V"></mjx-menu-item>
    </mjx-menu>
  `;
}

/**
 * **File** — seven groups, and the only one of the three that has no Print and does have a Publish.
 *
 * Both differences are the census's rather than this catalogue's, and this file's header is where
 * they are recorded. What they look like here:
 *
 * 1. **No Print group at all.** The strip goes Info, Open, Save, Share, Export, Publish, Help, and
 *    the Print group the other two collapse second simply is not there to collapse.
 * 2. **Publish is Excel's Power BI page**, three controls, which is the one group on any of the
 *    three File tabs where what Office shows and what the census counts agree exactly. Its headline
 *    carries no icon: Fluent draws no Power BI mark, and every generic candidate already means one
 *    of the two commands underneath it.
 * 3. **Info carries a fifth command**, Workbook Statistics, which Word and PowerPoint have no
 *    equivalent of.
 *
 * See `Ribbons/Word → File` for the collapse order and for the rest of the reasoning.
 */
export const File: Story = { render: () => ribbon('file') };

/**
 * **Home, and the one tab that is real.** Six groups, migrated out of `Shell/Excel` unchanged. The
 * census declares eight in scope; `GroupCells` and `GroupHomePowerOptions` are unit 2's work.
 */
export const Home: Story = { render: () => ribbon('home') };

/** Unit 3. */
export const Insert: Story = { render: () => ribbon('insert') };

/** Unit 4. */
export const Draw: Story = { render: () => ribbon('draw') };

/** Unit 5. */
export const PageLayout: Story = { render: () => ribbon('page-layout') };

/** Unit 6. */
export const Formulas: Story = { render: () => ribbon('formulas') };

/** Unit 7. */
export const Data: Story = { render: () => ribbon('data') };

/** Unit 8. */
export const Review: Story = { render: () => ribbon('review') };

/** Unit 9. */
export const View: Story = { render: () => ribbon('view') };

/** Unit 10, and a view tab — see `dev/ribbons/census.ts`. */
export const PrintPreview: Story = { render: () => ribbon('print-preview') };

/** A view tab: Office shows it only while a picture's background is being removed. */
export const BackgroundRemoval: Story = { render: () => ribbon('background-removal') };
