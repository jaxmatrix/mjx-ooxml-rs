import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  boxStatesFor,
  boxStatesMatrix,
  boxTokenDependencies,
  checkboxKeyboard,
  checkboxScreenReader,
  note,
  stack,
} from './specimens.ts';

/**
 * `<mjx-checkbox>` — and the third state, which is the one that gets lost.
 *
 * **Open *The Three Positions* and press Space on each.** The indeterminate one becomes checked
 * rather than unchecked, because a half-bold selection that a person bolds becomes bold. Then
 * inspect the middle one: `aria-checked="mixed"` is an *attribute value*, and a component that
 * reached for a class here would paint a third state and announce two.
 */

const conventions = storyConventions({
  statesMatrix: boxStatesFor(),
  tokenDependencies: boxTokenDependencies(),
  keyboard: checkboxKeyboard,
  screenReader: checkboxScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal, and it must stay one — Storybook indexes CSF statically and refuses a
  // computed title. `tests/browser/inputs.spec.ts` looks every story up by this string.
  title: 'Inputs/Checkbox',
  parameters: {
    docs: {
      description: {
        component:
          'A tri-state checkbox: a real <button role="checkbox"> so that all three positions are ' +
          'one attribute, which a native input cannot do — `indeterminate` is a DOM property with ' +
          'no attribute and survives no round trip through markup.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** **The story every gate that measures the box's paint opens.** */
export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () => html`
    ${note(
      'The four positions of the square. `hover` is forced so a static page can show it; `focus` ' +
        'is never forced — press Tab and watch the ring arrive through the browser’s own judgement ' +
        'about how focus got there.',
    )}
    ${boxStatesMatrix()}
  `,
};

/** The three positions, side by side, with the middle one the point of the story. */
export const TheThreePositions: Story = {
  name: 'The Three Positions',
  render: () => html`
    ${note(
      'Unchecked, checked, and indeterminate. Press Space on the third: it becomes checked, never ' +
        'unchecked. A selection that disagrees with itself, bolded, becomes bold.',
    )}
    ${stack(
      html`<mjx-checkbox label="Unchecked"></mjx-checkbox>`,
      html`<mjx-checkbox label="Checked" checked="true"></mjx-checkbox>`,
      html`<mjx-checkbox label="Indeterminate" checked="mixed"></mjx-checkbox>`,
    )}
  `,
};

/** The two kinds of unavailability, which are not the same kind. */
export const UnavailableAndDisabled: Story = {
  name: 'Unavailable And Disabled',
  render: () => html`
    ${note(
      'The first is Office’s: greyed, still reachable by Tab, still announced, and carrying the ' +
        'reason it cannot be used. The second is the platform’s: out of the tab order entirely. ' +
        'Tab through them and count how many stops there are — there is one.',
    )}
    ${stack(
      html`<mjx-checkbox
        label="Track changes"
        unavailable
        explanation="The document is not shared, so there is nobody to track changes for."
      ></mjx-checkbox>`,
      html`<mjx-checkbox label="Track changes" checked="true" disabled></mjx-checkbox>`,
    )}
  `,
};

/** A properties inspector's worth of them, in compact density. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => html`
    ${note(
      'Compact is what a properties inspector uses. The square shrinks with the density step and ' +
        'the row still clears the 24-pixel accessible hit-target floor, which is asserted rather ' +
        'than eyeballed.',
    )}
    <div data-density="compact">
      ${stack(
        html`<mjx-checkbox label="Ruler" checked="true"></mjx-checkbox>`,
        html`<mjx-checkbox label="Gridlines"></mjx-checkbox>`,
        html`<mjx-checkbox label="Navigation pane" checked="mixed"></mjx-checkbox>`,
      )}
    </div>
  `,
};
