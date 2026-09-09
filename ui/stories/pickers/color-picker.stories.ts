import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  chooseSwatchIndicator,
  describeWorstIndicator,
  indicatorSweepColors,
  swatchStateNames,
  swatchStates,
  worstSwatchIndicator,
  type MjxColorPicker,
} from '../../src/pickers/index.ts';
import {
  colorPickerKeyboard,
  colorPickerScreenReader,
  colorPickerTokenDependencies,
  documentThemePalette,
  grid,
  note,
  offGridColor,
  partialThemePalette,
  recentColors,
  stack,
  stage,
  standardColors,
  swatchStatesFor,
} from './specimens.ts';

/**
 * `<mjx-color-picker>` — and the two questions it exists to answer correctly.
 *
 * **Open *A Document's Own Theme* and look at the top block.** Those are not this platform's
 * colours: they are a made-up document's, handed in as data, and the picker ships none of its own.
 * Choose one and read the value under the grid — it says `theme:accent1/lighter40`, a **slot**,
 * not the blue it currently happens to be. That is the difference between a colour that follows a
 * customer's brand and one that overrides it, and it is invisible on the day it is chosen.
 *
 * **Then open *Every Indicator, Measured*.** The selected-swatch ring is not a fixed colour and
 * cannot be: any fixed edge is invisible on a swatch of that colour, and which swatch that is
 * depends entirely on the palette. It is chosen per swatch, and the caption is the measured worst
 * case over four thousand of them.
 */

const conventions = storyConventions({
  statesMatrix: swatchStatesFor(),
  tokenDependencies: colorPickerTokenDependencies(),
  keyboard: colorPickerKeyboard,
  screenReader: colorPickerScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Pickers/Colour Picker',
  parameters: {
    docs: {
      description: {
        component:
          'A colour, chosen from the document’s own theme or typed as any spelling of a literal. ' +
          'A theme choice reports as a slot and never as a colour; a selection indicator is ' +
          'measured against the swatch it sits on rather than fixed.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Give a picker its palettes, and optionally open it. */
function furnish(
  select: string,
  options: {
    theme?: typeof documentThemePalette;
    standard?: readonly string[];
    recent?: readonly string[];
    value?: string;
    open?: boolean;
    active?: number;
  } = {},
): void {
  requestAnimationFrame(() => {
    const found = document.querySelector(select);
    if (found === null) return;
    const picker = found as MjxColorPicker;
    if (options.theme !== undefined) picker.themePalette = options.theme;
    if (options.standard !== undefined) picker.standardColors = options.standard;
    if (options.recent !== undefined) picker.recentColors = options.recent;
    if (options.value !== undefined) picker.value = options.value;
    if (options.open === true) {
      picker.openList();
      // Through the surface, which is what a key press moves — not by writing an attribute, which
      // would be the story staging a picture of a state rather than producing one.
      if (options.active !== undefined) picker.surface?.setActive(options.active);
    }
  });
}

/** **The states matrix.** All five swatch states at once, outside any popup. */
export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () => html`
    ${note(
      'The five states a swatch takes, and the two different rings they are made of. The cursor ' +
        'ring is outside the square, on the popup’s own surface — a token against a token. The ' +
        'selection ring is inside the square, on the colour a person chose, so it is measured ' +
        'against that colour rather than fixed. One ring doing both jobs would have had to be a ' +
        'fixed colour and would have vanished on the swatch of that colour.',
    )}
    ${grid(
      ...swatchStateNames.map(
        (state) => html`
          <div
            data-swatch-state-cell=${state}
            title=${swatchStates[state].description}
            style="display:grid;gap:var(--mjx-density-step)"
          >
            <p class="mjx-type-label" style="margin:0;color:var(--theme-text-secondary)">${state}</p>
            <p class="mjx-type-dense" style="margin:0;color:var(--theme-text-primary)">
              ${swatchStates[state].description}
            </p>
            <p class="mjx-type-dense" style="margin:0;color:var(--theme-text-primary)">
              ${swatchStates[state].nonColourCue === undefined
                ? 'Told apart by colour, at 3 : 1 or better.'
                : `Non-colour cue: ${swatchStates[state].nonColourCue}`}
            </p>
          </div>
        `,
      ),
    )}
    ${note(
      'The live states are asserted on the grid itself rather than here: all five are producible ' +
        'by choosing one swatch, putting the keyboard on another and handing the picker a theme ' +
        'the document does not fully define — which is what *A Theme With Holes* does.',
    )}
  `,
};

/** The theme row is the **document's**, and the value it reports is a slot. */
export const ADocumentsOwnTheme: Story = {
  name: 'A Document’s Own Theme',
  render: () => {
    furnish('#document-theme', {
      theme: documentThemePalette,
      standard: standardColors,
      recent: recentColors,
      value: 'theme:accent1/lighter40',
      open: true,
    });
    return html`
      ${note(
        'Sixty theme swatches, ten standard and three recent — none of them shipped by the ' +
          'component. The theme block is variant-major, so every column is one slot and every row ' +
          'is one step of the luminance ladder, which is what makes Arrow Down mean “the same ' +
          'slot, one step darker”. The field reads “Accent 1, Lighter 40%”, because that is what ' +
          'the control reports; it does not read a hex value, and it must not.',
      )}
      <div style=${stage('34rem')}>
        <mjx-color-picker
          id="document-theme"
          label="Font colour"
          show-automatic
          show-no-fill
          automatic="#0f1729"
          style="inline-size:18rem"
        ></mjx-color-picker>
      </div>
    `;
  },
};

/** A theme that does not define every slot — the refused state, produced rather than forced. */
export const AThemeWithHoles: Story = {
  name: 'A Theme With Holes',
  render: () => {
    furnish('#partial-theme', {
      theme: partialThemePalette,
      standard: standardColors,
      value: 'theme:accent1',
      open: true,
      active: 12,
    });
    return html`
      ${note(
        'This document defines six of the ten gallery slots. The four it does not are drawn with ' +
          'a dashed edge, announce as disabled, and carry the reason — they are still reachable ' +
          'with the arrow keys, because an entry a person cannot reach is an entry whose reason ' +
          'they can never read. The picker does not fill the holes with its own accent, which ' +
          'would have looked entirely correct and would have put our colours into their deck.',
      )}
      <div style=${stage('34rem')}>
        <mjx-color-picker
          id="partial-theme"
          label="Shape fill"
          show-no-fill
          style="inline-size:18rem"
        ></mjx-color-picker>
      </div>
    `;
  },
};

/** No theme at all: the theme section is **absent**, not invented. */
export const NoThemeSupplied: Story = {
  name: 'No Theme Supplied',
  render: () => {
    furnish('#no-theme', { standard: standardColors, open: true });
    return html`
      ${note(
        'A picker that has not been told the document’s theme draws no theme section. The ' +
          'alternative — falling back to this platform’s accents — is the failure mode the ' +
          'project’s standing rule names by name: author a default only where nothing exists, ' +
          'and a document always has a theme.',
      )}
      <div style=${stage('22rem')}>
        <mjx-color-picker id="no-theme" label="Border colour" style="inline-size:18rem"></mjx-color-picker>
      </div>
    `;
  },
};

/** A colour that is on no grid, typed into the field. */
export const AColourThatIsOnNoGrid: Story = {
  name: 'A Colour That Is On No Grid',
  render: () => {
    furnish('#off-grid', { theme: documentThemePalette, standard: standardColors, value: offGridColor });
    return html`
      ${note(
        `The field is a text box as well as a grid. Type ${offGridColor}, or rgb(18, 52, 87), or ` +
          'hsl(214, 65.7%, 20.6%), or the words “theme:accent2/darker25” — every one of them ' +
          'commits the same canonical value, and the field then shows exactly what the control ' +
          'reports. A picker whose only input was its own palette could not return a colour that ' +
          'is not in its own palette, which is the requirement this control was written for.',
      )}
      ${stack(
        html`<mjx-color-picker
          id="off-grid"
          label="Font colour"
          show-automatic
          automatic="#0f1729"
        ></mjx-color-picker>`,
        html`<mjx-color-picker label="Highlight" placeholder="Choose a colour…" show-no-fill></mjx-color-picker>`,
        html`<mjx-color-picker
          label="Font colour"
          value="theme:accent1"
          unavailable
          explanation="The run takes its colour from its character style."
        ></mjx-color-picker>`,
      )}
    `;
  },
};

/**
 * **The measurement, as a caption.**
 *
 * The numbers are computed here, from the same functions the component and the gate call — not
 * transcribed. A caption that had been typed out would be a claim about the palette of the day it
 * was typed.
 */
export const EveryIndicatorMeasured: Story = {
  name: 'Every Indicator, Measured',
  render: () => {
    const colours = indicatorSweepColors();
    const light = worstSwatchIndicator(colours, 'light');
    const dark = worstSwatchIndicator(colours, 'dark');
    const shown = ['#ffffff', '#000000', '#808080', '#2f5fb0', '#ffff00', '#123457'];
    return html`
      ${note(
        `A selection indicator on an arbitrary colour cannot be a fixed colour: any fixed edge is ` +
          `invisible on a swatch of that colour. It is chosen per swatch — whichever of ` +
          `--theme-text-primary and --theme-surface reads better on it — and over ` +
          `${String(colours.length)} colours (every grey, and a lattice of the sRGB cube) the ` +
          `worst case is ${light === undefined ? 'unmeasured' : describeWorstIndicator(light)} in ` +
          `light and ${dark === undefined ? 'unmeasured' : describeWorstIndicator(dark)} in dark. ` +
          `WCAG 1.4.11 asks for 3 : 1. Every fixed alternative bottoms out at 1.00 : 1, which is ` +
          `what the gate proves rather than assumes.`,
      )}
      <div
        style="display:flex;flex-wrap:wrap;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
      >
        ${shown.map((colour) => {
          const indicator = chooseSwatchIndicator(colour, 'light');
          return html`
            <div style="display:grid;gap:var(--mjx-density-step);justify-items:center">
              <span
                style=${`inline-size:3rem;block-size:3rem;border-radius:var(--radius-chip);background:${colour};box-shadow:inset 0 0 0 calc(var(--spacing) * 0.5) var(--theme-${indicator.member === 'textPrimary' ? 'text-primary' : 'surface'})`}
              ></span>
              <span class="mjx-type-dense" style="color:var(--theme-text-primary)">${colour}</span>
              <span class="mjx-type-dense" style="color:var(--theme-text-primary)"
                >${indicator.member} · ${indicator.ratio.toFixed(2)} : 1</span
              >
            </div>
          `;
        })}
      </div>
    `;
  },
};

/** Near the bottom of a short container, so the palette has to flip. */
export const AtTheBottomOfItsRoom: Story = {
  name: 'At The Bottom Of Its Room',
  render: () => {
    furnish('#flipping-palette', { theme: documentThemePalette, standard: standardColors, open: true });
    return html`
      ${note(
        'The field is at the foot of what clips it, so the palette opens upward. The placement is ' +
          '`placeFloating` from the overlay module — the same arithmetic a menu, a gallery flyout ' +
          'and a listbox use — and the gate re-runs it in Node over the component’s own recorded ' +
          'inputs and requires the same answer.',
      )}
      <div style=${`${stage('20rem')};display:flex;align-items:flex-end`}>
        <mjx-color-picker
          id="flipping-palette"
          label="Shape fill"
          style="inline-size:18rem"
        ></mjx-color-picker>
      </div>
    `;
  },
};

/** Under right-to-left, where the inline arrows mirror and the block arrows do not. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () => {
    furnish('#rtl-palette', {
      theme: documentThemePalette,
      standard: standardColors,
      value: 'theme:accent3',
      open: true,
      active: 4,
    });
    return html`
      ${note(
        'Arrow Right moves to the *previous* cell, because the inline axis mirrors; Arrow Down ' +
          'still moves down, because the block axis does not. The slider model records the same ' +
          'trap and the same fix, and the gate asserts the two directions disagree about one key ' +
          'rather than asserting that the cursor ends up somewhere plausible.',
      )}
      <div dir="rtl" style=${stage('34rem')}>
        <mjx-color-picker id="rtl-palette" label="لون الخط" style="inline-size:18rem"></mjx-color-picker>
      </div>
    `;
  },
};

/** In compact density, where the cell shrinks and the floor holds. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    furnish('#compact-palette', { theme: documentThemePalette, standard: standardColors, open: true });
    return html`
      ${note(
        'A compact inspector’s picker. The swatch cell follows the density mode down and stops at ' +
          'the 24 CSS pixel floor — WCAG 2.2’s Target Size (Minimum) — which is the one thing a ' +
          'density mode in this catalogue is never allowed to cross. Below about ten of those the ' +
          'ten-wide theme block scrolls sideways rather than narrowing its cells.',
      )}
      <div data-density="compact" style=${stage('30rem')}>
        <mjx-color-picker
          id="compact-palette"
          label="Font colour"
          style="inline-size:16rem"
        ></mjx-color-picker>
      </div>
    `;
  },
};
