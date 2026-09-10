import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  note,
  sliderKeyboard,
  sliderScreenReader,
  sliderTokenDependencies,
  stack,
} from './specimens.ts';

/**
 * `<mjx-slider>` — the identity-value trap, in geometry.
 *
 * **Open *At Several Values* rather than looking at one slider.** A slider at its minimum has its
 * thumb at the start of the track whether the arithmetic is right, is zero, or was never done — so
 * a single specimen proves nothing, and the story that proves something shows the same slider at
 * both ends and three places in between.
 *
 * Then take *A Maximum Off The Boundary*, whose top is deliberately not on a step, and press End.
 * It lands on ten exactly, because End means the top.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'rest', description: 'A track, a filled half, and a thumb where the value is.' },
    {
      name: 'focus',
      description: 'The track is the focusable element and wears the foundations’ single ring.',
    },
    {
      name: 'dragging',
      description: 'A pointer is down. Preview events fire throughout; one change fires at the end.',
    },
    {
      name: 'unavailable',
      description: 'Explained and refused, and still reachable so that the reason can be read.',
    },
    { name: 'disabled', description: 'Out of the tab order, and dimmed.' },
  ],
  tokenDependencies: sliderTokenDependencies(),
  keyboard: sliderKeyboard,
  screenReader: sliderScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Inputs/Slider',
  parameters: {
    docs: {
      description: {
        component:
          'The filled track is --theme-accent-pressed and not --theme-accent, and that is a ' +
          'measurement rather than a preference: accent on border-subtle is 2.81 : 1, so the ' +
          'boundary between the filled and unfilled halves — which is the entire visual output of ' +
          'a slider — would be below the non-text minimum.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** **The story the geometry gate reads.** */
export const AtSeveralValues: Story = {
  name: 'At Several Values',
  render: () => html`
    ${note(
      'One range, five values, both ends included. The gate measures each thumb’s centre against ' +
        'the fraction computed in Node from the min, max and value — never against the custom ' +
        'property the component itself wrote, which would be the component grading its own ' +
        'homework.',
    )}
    ${stack(
      ...[0, 25, 50, 75, 100].map(
        (value) => html`
          <mjx-slider
            data-at=${String(value)}
            label=${`Opacity ${String(value)}`}
            min="0"
            max="100"
            step="5"
            suffix="%"
            value=${String(value)}
          ></mjx-slider>
        `,
      ),
    )}
  `,
};

/** A zoom control, with ticks and a suffix. */
export const WithTicks: Story = {
  name: 'With Ticks',
  render: () => html`
    ${note(
      'The tick labels are secondary text on the page’s own surface — 5.23 : 1 — and are told ' +
        'apart from the value by size rather than by colour. They are never drawn on a fill, which ' +
        'is exactly the trap the field table was reshaped to avoid.',
    )}
    ${stack(
      html`<mjx-slider
        id="zoom"
        label="Zoom"
        min="10"
        max="400"
        step="10"
        value="100"
        suffix="%"
        ticks="10,100,200,300,400"
      ></mjx-slider>`,
    )}
  `,
};

/** A fractional step, which is where a naive implementation reports 0.30000000000000004. */
export const AFractionalStep: Story = {
  name: 'A Fractional Step',
  render: () => html`
    ${note(
      'Line spacing from 1 to 3 in twentieths. Arrow up from 1.15 and read the announcement: 1.2, ' +
        'not 1.2000000000000002. Rounding to the number of places the step has is the only ' +
        'rounding guaranteed not to lose something the step could express.',
    )}
    ${stack(
      html`<mjx-slider
        id="spacing"
        label="Line spacing"
        min="1"
        max="3"
        step="0.05"
        value="1.15"
        suffix="×"
      ></mjx-slider>`,
    )}
  `,
};

/** A maximum that is not on a step boundary, which is where End goes wrong. */
export const AMaximumOffTheBoundary: Story = {
  name: 'A Maximum Off The Boundary',
  render: () => html`
    ${note(
      'Zero to ten in threes: the boundaries are 0, 3, 6 and 9, and ten is not one of them. Press ' +
        'End — it lands on ten. Arrow up from nine — it lands on ten as well, because a step past ' +
        'the top is clamped to the top. A snap that clamped first and rounded second would make ' +
        'the maximum unreachable on a slider whose label says the top is ten.',
    )}
    ${stack(
      html`<mjx-slider
        id="off-boundary"
        label="Columns"
        min="0"
        max="10"
        step="3"
        value="0"
      ></mjx-slider>`,
    )}
  `,
};

/** Right to left, where the inline arrows mirror and the block arrows do not. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () => html`
    ${note(
      'The track runs the other way, so Arrow **Left** increases. Arrow Up still increases, ' +
        'because up is up in every writing system — which is the row of the key map that gets ' +
        'forgotten.',
    )}
    <div dir="rtl">
      ${stack(
        html`<mjx-slider
          id="rtl-slider"
          label="تكبير"
          min="0"
          max="100"
          step="10"
          value="30"
          suffix="%"
        ></mjx-slider>`,
      )}
    </div>
  `,
};

/** The two kinds of unavailability. */
export const UnavailableAndDisabled: Story = {
  name: 'Unavailable And Disabled',
  render: () => html`
    ${note(
      'The first is still reachable by Tab and carries its reason; the second is out of the tab ' +
        'order. A slider’s track is a focusable div with no native `disabled`, so the shared ' +
        'availability helper announces the fact with aria-disabled and the component takes the ' +
        'track out of the tab order itself.',
    )}
    ${stack(
      html`<mjx-slider
        id="unavailable-slider"
        label="Zoom"
        value="100"
        min="10"
        max="400"
        unavailable
        explanation="The view is set to fit the page."
      ></mjx-slider>`,
      html`<mjx-slider label="Zoom" value="100" min="10" max="400" disabled></mjx-slider>`,
    )}
  `,
};
