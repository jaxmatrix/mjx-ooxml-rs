import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  fontsNeedingWarning,
  substitutionNote,
  type FontDescriptor,
  type MjxFontPicker,
} from '../../src/pickers/index.ts';
import {
  fontPickerKeyboard,
  fontPickerScreenReader,
  fontPickerTokenDependencies,
  fontStatesFor,
  installedFamily,
  machineFonts,
  manyFonts,
  missingFamily,
  note,
  stack,
  stage,
  substitutedFamily,
} from './specimens.ts';

/**
 * `<mjx-font-picker>` — a font box that says which of these fonts this machine does not have.
 *
 * **Open *What The Warning Is For* and compare the two fields.** Both say a family name. One of
 * them is a family this machine does not have, is being drawn with something else, and says so —
 * in the field, in the row, and to a screen reader. Take that sentence away and the two fields are
 * indistinguishable, which is precisely the failure: a person picks Cambria, gets Caladea, and
 * nothing anywhere said so.
 *
 * **Then open *Four Hundred Rows Of Faces* and scroll.** Each name is drawn in its own face, which
 * is U06's virtualisation precondition violated by construction — a face decides its own ascender.
 * Every row is nonetheless exactly one row tall, because the height comes from the row and not
 * from the glyphs inside it, and the gate measures every built row rather than trusting this
 * paragraph.
 */

const conventions = storyConventions({
  statesMatrix: fontStatesFor(),
  tokenDependencies: fontPickerTokenDependencies(),
  keyboard: fontPickerKeyboard,
  screenReader: fontPickerScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Pickers/Font Picker',
  parameters: {
    docs: {
      description: {
        component:
          'Every installed family, each drawn in its own face, with the ones this machine ' +
          'substitutes marked as substituted — in the row, in the field, and in the ' +
          'announcement. A substituted family stays choosable, because refusing it would ' +
          'silently rewrite a document that legitimately asks for it.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Hand a picker its families, and optionally open it. */
function furnish(
  select: string,
  fonts: readonly FontDescriptor[],
  options: { value?: string; open?: boolean; active?: number } = {},
): void {
  requestAnimationFrame(() => {
    const found = document.querySelector(select);
    if (found === null) return;
    const picker = found as MjxFontPicker;
    picker.fonts = fonts;
    if (options.value !== undefined) picker.value = options.value;
    if (options.open === true) {
      picker.openList();
      if (options.active !== undefined) picker.surface?.setActive(options.active);
    }
  });
}

/** **The states matrix**, and the whole reason the control exists. */
export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () => {
    furnish('#state-installed', machineFonts, { value: installedFamily });
    furnish('#state-substituted', machineFonts, { value: substitutedFamily });
    furnish('#state-missing', machineFonts, { value: missingFamily });
    furnish('#state-unavailable', machineFonts, { value: installedFamily });
    return html`
      ${note(
        'Four fields holding four families. The first is installed and carries nothing. The ' +
          'second is substituted with a metric-compatible face — the layout does not move, and ' +
          'the sentence says so. The third is substituted with one whose metrics differ, so lines ' +
          'may rewrap, and the sentence says that instead. The fourth is not available at all. ' +
          'Every one of those sentences is associated with the field by aria-describedby, so it ' +
          'is announced rather than merely drawn.',
      )}
      ${stack(
        html`<mjx-font-picker id="state-installed" label="Font — installed"></mjx-font-picker>`,
        html`<mjx-font-picker id="state-substituted" label="Font — substituted, metrics match"></mjx-font-picker>`,
        html`<mjx-font-picker id="state-missing" label="Font — missing"></mjx-font-picker>`,
        html`<mjx-font-picker
          id="state-unavailable"
          label="Font — the run inherits it"
          unavailable
          explanation="The run takes its font from its character style."
        ></mjx-font-picker>`,
      )}
    `;
  },
};

/** The comparison the whole control is for: a warning, and its absence. */
export const WhatTheWarningIsFor: Story = {
  name: 'What The Warning Is For',
  render: () => {
    furnish('#warning-present', machineFonts, { value: substitutedFamily });
    furnish('#warning-absent', machineFonts, { value: installedFamily });
    const substituted = machineFonts.find((font) => font.family === substitutedFamily);
    return html`
      ${note(
        'Two fields. Cover the sentence under the first one and they are the same control saying ' +
          'the same thing — which is what a font picker without a substitution warning is. The ' +
          `sentence is: “${substituted === undefined ? '' : (substitutionNote(substituted) ?? '')}” ` +
          `and ${String(fontsNeedingWarning(machineFonts).length)} of the ` +
          `${String(machineFonts.length)} families on this machine carry one.`,
      )}
      ${stack(
        html`<mjx-font-picker id="warning-present" label="Font"></mjx-font-picker>`,
        html`<mjx-font-picker id="warning-absent" label="Font"></mjx-font-picker>`,
      )}
    `;
  },
};

/** The open list: names in their own faces, marks on the rows that need them. */
export const EveryRowSaysWhatItIs: Story = {
  name: 'Every Row Says What It Is',
  render: () => {
    furnish('#open-list', machineFonts, { value: substitutedFamily, open: true, active: 5 });
    return html`
      ${note(
        'Each family name is drawn in the face it will actually be drawn in — which for a ' +
          'substituted family is the stand-in, not the family. Previewing the family would have ' +
          'drawn the browser’s silent fallback and made the row look installed, so the one place ' +
          'a preview could have lied is the one place it must not. The rows that are not ' +
          'installed carry a glyph and a word, at the dense size: told apart by size, never by ' +
          'colour, because an option’s fill changes under the keyboard cursor and grey on that ' +
          'fill is 4.32 : 1.',
      )}
      <div style=${stage('30rem')}>
        <mjx-font-picker id="open-list" label="Font" style="inline-size:22rem"></mjx-font-picker>
      </div>
    `;
  },
};

/**
 * **The story the one-row-tall gate reads.**
 *
 * Sixty families in two sections, of which eight are ever visible, every name in a different face.
 */
export const FourHundredRowsOfFaces: Story = {
  name: 'Four Hundred Rows Of Faces',
  render: () => {
    furnish('#long-faces', manyFonts, { value: 'Cambria', open: true });
    return html`
      ${note(
        'Sixty families in two sections. The rows outside the window are heights rather than ' +
          'elements — U06’s row planner at one column, unchanged — and every one of them is ' +
          'exactly one row tall despite each name being drawn in its own face. The probes below ' +
          'are the same five faces outside the list, and they are visibly different heights: that ' +
          'is what makes the gate’s “all rows are one height” a measurement rather than a ' +
          'tautology about a list of one face.',
      )}
      <div style=${stage('30rem')}>
        <mjx-font-picker id="long-faces" label="Font" style="inline-size:22rem"></mjx-font-picker>
      </div>
      <div
        data-face-probes
        style="display:flex;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2);align-items:flex-start"
      >
        ${['system-ui', 'serif', 'monospace', 'cursive', 'fantasy'].map(
          (family) => html`
            <span
              data-face-probe=${family}
              class="mjx-type-control"
              style=${`font-family:${family};line-height:normal;font-size:var(--text-lg);color:var(--theme-text-primary)`}
              >Hxg${family}</span
            >
          `,
        )}
      </div>
    `;
  },
};

/** A family the list does not carry, typed — the round trip a font box must still keep. */
export const AFamilyTheListDoesNotCarry: Story = {
  name: 'A Family The List Does Not Carry',
  render: () => {
    furnish('#custom-family', machineFonts, { value: installedFamily });
    return html`
      ${note(
        'A document may legitimately name a family this machine has never heard of, and a person ' +
          'may legitimately type one. With allow-custom the field keeps it and reports it; ' +
          'without, the field reverts to what it reports and says so with an event rather than ' +
          'reverting silently. Either way the text the field shows is the value the control ' +
          'reports, after every one of the nine ways of finishing.',
      )}
      ${stack(
        html`<mjx-font-picker id="custom-family" label="Font" allow-custom></mjx-font-picker>`,
      )}
    `;
  },
};

/** In compact density, where the rows are shorter and the floor still holds. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    furnish('#compact-faces', machineFonts, { value: substitutedFamily, open: true });
    return html`
      ${note(
        'A compact inspector’s font box. The rows follow the density mode down and stop at the 24 ' +
          'CSS pixel floor, and they are all still exactly one height — including the section ' +
          'headings, which is the part that is easy to get wrong and impossible to see.',
      )}
      <div data-density="compact" style=${stage('26rem')}>
        <mjx-font-picker id="compact-faces" label="Font" style="inline-size:20rem"></mjx-font-picker>
      </div>
    `;
  },
};
