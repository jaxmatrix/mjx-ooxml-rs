import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  fieldStatesFor,
  fieldStatesMatrix,
  fieldTokenDependencies,
  measureKeyboard,
  measureScreenReader,
  note,
  stack,
} from './specimens.ts';

/**
 * `<mjx-measure-input>` — a number with a unit on it, and a stated answer for a string it cannot
 * read.
 *
 * **Open *What It Does With Nonsense* and type `banana`, then press Tab.** The text stays, the
 * value does not move, the field says so three ways at once, and an event carries what was
 * refused. Then press Escape: back to the value, and out of the invalid state. That is the whole
 * of the component's contract, and the only reason it needs a story of its own is that the
 * tempting alternative — quietly putting the old number back — looks identical in a screenshot.
 */

const conventions = storyConventions({
  statesMatrix: fieldStatesFor(),
  tokenDependencies: fieldTokenDependencies(),
  keyboard: measureKeyboard,
  screenReader: measureScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Inputs/Measure Input',
  parameters: {
    docs: {
      description: {
        component:
          'Points are the canonical unit and every other is a factor, so switching the display ' +
          'unit never moves the stored value. Both decimal separators are accepted, always — the ' +
          'grammar has no thousands separator, so a comma can only mean one thing — and the ' +
          '`decimal` attribute decides how a value is written back.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** **The story every gate that measures a field's paint opens.** */
export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () => html`
    ${note(
      'The seven states of a field, on the control this catalogue’s field table was written for. ' +
        '`focus` is never forced. `invalid` is forced here so a static page can show it; the ' +
        'story next door produces it for real.',
    )}
    ${fieldStatesMatrix()}
  `,
};

/** The same quantity, in six units. */
export const TheSameQuantitySixWays: Story = {
  name: 'The Same Quantity Six Ways',
  render: () => html`
    ${note(
      'Every field holds 72 points. They are the same number, converted for display and never for ' +
        'storage — which is what makes switching a field’s unit and switching it back an identity ' +
        'rather than a rounding.',
    )}
    ${stack(
      html`<mjx-measure-input label="Points" value="72" unit="pt"></mjx-measure-input>`,
      html`<mjx-measure-input label="Inches" value="72" unit="in"></mjx-measure-input>`,
      html`<mjx-measure-input label="Centimetres" value="72" unit="cm"></mjx-measure-input>`,
      html`<mjx-measure-input label="Millimetres" value="72" unit="mm"></mjx-measure-input>`,
      html`<mjx-measure-input label="CSS pixels" value="72" unit="px"></mjx-measure-input>`,
      html`<mjx-measure-input label="Picas" value="72" unit="pc"></mjx-measure-input>`,
    )}
  `,
};

/** **The story the invalid-path gate reads.** */
export const WhatItDoesWithNonsense: Story = {
  name: 'What It Does With Nonsense',
  render: () => html`
    ${note(
      'Type `banana` into the first and Tab away. Type `12 furlongs` into the second. Each says ' +
        'which of the three ways it failed and what defeated it — and neither touches the text or ' +
        'the value. Escape is the way out.',
    )}
    ${stack(
      html`<mjx-measure-input id="nonsense" label="Left indent" value="12"></mjx-measure-input>`,
      html`<mjx-measure-input id="wrong-unit" label="Right indent" value="12"></mjx-measure-input>`,
    )}
  `,
};

/** A locale that writes its decimals with a comma. */
export const WithADecimalComma: Story = {
  name: 'With A Decimal Comma',
  render: () => html`
    ${note(
      'The second field writes `2,5 cm`. Type `2.5 cm` into it anyway: it is accepted, because ' +
        'this grammar has no thousands separator and a full stop can only mean the same thing. ' +
        'The attribute governs the writing, which is the half that is genuinely a locale question.',
    )}
    ${stack(
      html`<mjx-measure-input label="Top margin (dot)" value="70.866" unit="cm"></mjx-measure-input>`,
      html`<mjx-measure-input
        id="comma"
        label="Top margin (comma)"
        value="70.866"
        unit="cm"
        decimal="comma"
      ></mjx-measure-input>`,
    )}
  `,
};

/** A field with a range, and the arrow keys that step inside it. */
export const SteppingInsideARange: Story = {
  name: 'Stepping Inside A Range',
  render: () => html`
    ${note(
      'Arrow Up in the centimetres field moves by a quarter of a centimetre, not by 1/28th of one: ' +
        'a step is in the unit the field is showing. Both fields stop at their ends, and a number ' +
        'typed past an end is clamped rather than refused — a range is a promise about what the ' +
        'field accepts, and a person who typed 2000 pt meant *as much as it takes*.',
    )}
    ${stack(
      html`<mjx-measure-input
        id="ranged-pt"
        label="Font size"
        value="12"
        unit="pt"
        step="1"
        min="1"
        max="1638"
      ></mjx-measure-input>`,
      html`<mjx-measure-input
        id="ranged-cm"
        label="Top margin"
        value="72"
        unit="cm"
        step="0.25"
        min="0"
        max="1584"
      ></mjx-measure-input>`,
    )}
  `,
};
