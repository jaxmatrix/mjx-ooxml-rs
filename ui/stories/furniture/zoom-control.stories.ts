import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  caption,
  furnitureTokenDependencies,
  note,
  stage,
  zoomCaption,
  zoomKeyboard,
  zoomScreenReader,
  zoomStates,
} from './specimens.ts';
import { zoomBounds } from '../../src/furniture/furniture-model.ts';

/**
 * `<mjx-zoom-control>` — a slider, a numeric field, and arithmetic with correct answers.
 *
 * **Open *The Two Fits* first.** The viewport is stated on the specimen and so is the page, so both
 * commands have a number a reader can check by hand — and the caption computes them from the same
 * functions the gates assert, so the page and the test cannot drift apart.
 *
 * **Then type into the readout.** Type `banana` and press Enter: the text stays, nothing is
 * committed, and the field says so three ways at once. Type `700`: it becomes 500, because out of
 * range is not a failure — it is a number a person meant.
 */

const conventions = storyConventions({
  statesMatrix: zoomStates,
  tokenDependencies: furnitureTokenDependencies,
  keyboard: zoomKeyboard,
  screenReader: zoomScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Furniture/Zoom Control',
  parameters: {
    docs: {
      description: {
        component:
          'A slider with preset stops, a percentage readout that is also an input, and fit to ' +
          'width and fit to page. It writes no slider and no field of its own: MJXOFF-186 shipped ' +
          'both, and a third would have been a third answer to “what does Home do”.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const viewport = { width: 1000, height: 700 };

/** The ordinary case. */
export const Resting: Story = {
  name: 'At One Hundred Per Cent',
  render: () =>
    stage(
      note(
        'The stepping commands move between stops rather than by one per cent: nobody wants to ' +
          'press a button ninety times to halve a page. The slider moves by one, which is what ' +
          'the arrow keys are for.',
      ),
      html`
        <mjx-zoom-control
          id="zoom"
          percent="100"
          viewport-width=${viewport.width}
          viewport-height=${viewport.height}
        ></mjx-zoom-control>
      `,
      caption(zoomCaption(viewport)),
    ),
};

/** The two fits, with the numbers on the page. */
export const TheTwoFits: Story = {
  name: 'The Two Fits',
  render: () =>
    stage(
      note(
        'Press Fit width and then Fit page. The command that has already been applied disables ' +
          'itself, which is a limit a person can see rather than a button that stopped ' +
          'responding. Both computations are floored.',
      ),
      html`
        <mjx-zoom-control
          id="zoom"
          percent="100"
          viewport-width=${viewport.width}
          viewport-height=${viewport.height}
        ></mjx-zoom-control>
      `,
      caption(zoomCaption(viewport)),
    ),
};

/** The bounds, where a command becomes a limit. */
export const AtTheBounds: Story = {
  name: 'At The Bounds',
  render: () =>
    stage(
      note(
        `The range is ${String(zoomBounds.min)} % to ${String(zoomBounds.max)} %. At either end ` +
          'the command that would leave it is disabled rather than silent, and Home and End on ' +
          'the slider land on the bounds exactly.',
      ),
      html`
        <div style="display:flex;flex-direction:column;gap:var(--mjx-density-gutter)">
          <mjx-zoom-control
            id="at-minimum"
            percent=${zoomBounds.min}
            viewport-width=${viewport.width}
            viewport-height=${viewport.height}
          ></mjx-zoom-control>
          <mjx-zoom-control
            id="at-maximum"
            percent=${zoomBounds.max}
            viewport-width=${viewport.width}
            viewport-height=${viewport.height}
          ></mjx-zoom-control>
        </div>
      `,
    ),
};

/** What it does with a string it cannot read, which is the whole point of the readout. */
export const WhatItRefuses: Story = {
  name: 'What It Refuses, And What It Clamps',
  render: () =>
    stage(
      note(
        'Type banana and press Enter: the text stays, nothing is committed, aria-invalid is set, ' +
          'a warning glyph appears and the reason is written out beneath. Escape is the way back ' +
          'and it is the only one. Then type 700 — that is not a failure, it is 500, and the ' +
          'field says so by simply being at 500.',
      ),
      html`
        <mjx-zoom-control
          id="zoom"
          percent="100"
          viewport-width=${viewport.width}
          viewport-height=${viewport.height}
        ></mjx-zoom-control>
      `,
    ),
};

/** Compact density, at the hit-target floor. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`
      <div data-density="compact">
        ${stage(
          note('The two stepping commands are the things to measure: they still clear 24 CSS pixels.'),
          html`
            <mjx-zoom-control
              id="zoom"
              percent="100"
              viewport-width=${viewport.width}
              viewport-height=${viewport.height}
            ></mjx-zoom-control>
          `,
        )}
      </div>
    `,
};
