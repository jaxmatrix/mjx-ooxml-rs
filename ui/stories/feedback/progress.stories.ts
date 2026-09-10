import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  indeterminateSpanFraction,
  progressKindNames,
} from '../../src/feedback/feedback-model.ts';
import {
  feedbackKeyboard,
  feedbackScreenReader,
  feedbackTokenDependencies,
  note,
  stage,
  timingReport,
} from './specimens.ts';

/**
 * `<mjx-progress>` — two kinds of bar, and the one that must not be mistaken for a stalled one.
 *
 * **Open *Told Apart From A Stalled Bar* first.** The two bars on it are the same width, on
 * purpose: the determinate one is parked at exactly the fraction of the track the indeterminate
 * one's indicator spans. A screenshot of that page cannot tell you which is which, and neither can
 * a gate that only measured a width. What tells them apart is that one of them **moves**, and — for
 * a person who has asked their operating system to stop moving things, where neither does — that
 * only one of them carries a value in the accessibility tree.
 *
 * **Then read *The Ladder*.** Five values including both ends, because a bar tested at one value
 * exercises no arithmetic at all.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'determinate 0%', description: 'Nothing done. The indicator has no width, and the value is still announced.' },
    { name: 'determinate, partway', description: 'The indicator’s width is the value’s fraction of the track. Three of these on The Ladder.' },
    { name: 'determinate 100%', description: 'The indicator fills the track. The other end of the arithmetic.' },
    { name: 'a non-unit maximum', description: '3 of 7 files: aria-valuemax is the author’s own scale, so the announcement is countable.' },
    { name: 'indeterminate', description: 'A fixed span that travels. No aria-valuenow at all, because a number nobody knows is a number nobody should be told.' },
    { name: 'indeterminate, reduced motion', description: 'It stops. Which is why the missing value, and not the movement, is what carries the meaning.' },
  ],
  tokenDependencies: feedbackTokenDependencies,
  keyboard: feedbackKeyboard,
  screenReader: feedbackScreenReader,
});

const meta: Meta = {
  title: 'Feedback/Progress',
  parameters: {
    docs: {
      description: {
        component:
          'A determinate bar whose indicator is its value, and an indeterminate one that is told ' +
          'apart from a stalled bar three ways at once — because the most obvious of the three is ' +
          'switched off for anyone who asked for less motion.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Five values, including both ends. A single value would exercise none of the arithmetic. */
const ladder = [0, 0.25, 0.5, 0.75, 1];

export const TheLadder: Story = {
  name: 'The Ladder',
  render: () =>
    stage(
      note(
        'Five bars at five values. Measure any of them against its own track rather than against ' +
          'the one above it: a bar that grew is a weaker claim than a bar that is proportional, ' +
          'and only the second one is what a person reads a percentage off.',
      ),
      html`
        <div style="display:grid;gap:var(--mjx-density-gutter);max-inline-size:32rem">
          ${ladder.map(
            (value) => html`
              <mjx-progress
                id=${`ladder-${String(Math.round(value * 100))}`}
                label=${`Uploading at ${String(Math.round(value * 100))} per cent`}
                value=${String(value)}
                readout
              ></mjx-progress>
            `,
          )}
        </div>
      `,
      timingReport(),
    ),
};

/** A maximum that is not one, so the arithmetic has something to divide. */
export const CountableUnits: Story = {
  name: 'Countable Units',
  render: () =>
    stage(
      note(
        'Three of seven files. aria-valuemax is the author’s own scale rather than a normalised ' +
          'hundred, so an assistive technology can say how many — and the drawn readout is still a ' +
          'percentage, because that is what an eye reads off a bar.',
      ),
      html`
        <div style="display:grid;gap:var(--mjx-density-gutter);max-inline-size:32rem">
          <mjx-progress id="files" label="Uploading files" value="3" max="7" readout></mjx-progress>
          <mjx-progress id="pages" label="Rendering pages" value="128" max="512" readout></mjx-progress>
          <!--
            One of forty-nine, and it is here because it is the value that catches a defect the
            other two cannot: recovering the value from the fraction returns 0.9999999999999999 for
            it, which an assistive technology reads out in full. Every other value in this
            catalogue happens to divide exactly.
          -->
          <mjx-progress id="shapes" label="Placing shapes" value="1" max="49" readout></mjx-progress>
        </div>
      `,
    ),
};

/**
 * The story this component exists for.
 *
 * The determinate bar is parked at exactly the fraction the indeterminate indicator spans, so the
 * two are the same width and a width comparison proves nothing about either.
 */
export const ToldApartFromAStalledBar: Story = {
  name: 'Told Apart From A Stalled Bar',
  render: () =>
    stage(
      note(
        'These two indicators are the same width, deliberately. One of them is a task that has ' +
          'stopped at 45 per cent and one is a task whose length nobody knows. In this window the ' +
          'difference you can see is the movement; with a reduced-motion preference there is no ' +
          'movement to see, and the only difference left is that one of them has a value and the ' +
          'other has none.',
      ),
      html`
        <div style="display:grid;gap:var(--mjx-density-gutter);max-inline-size:32rem">
          <mjx-progress
            id="stalled"
            label="Stalled at the same width"
            value=${String(indeterminateSpanFraction)}
            readout
          ></mjx-progress>
          <mjx-progress id="working" label="Contacting the server" indeterminate></mjx-progress>
        </div>
      `,
      html`
        <p class="mjx-type-dense" style="margin:0;color:var(--theme-text-secondary)">
          The two kinds are ${progressKindNames.join(' and ')}, and the indeterminate indicator
          spans ${String(Math.round(indeterminateSpanFraction * 100))} per cent of its track — a
          fraction chosen so that it is neither a full bar nor a round number anyone would read as
          one.
        </p>
      `,
    ),
};

/** Compact, where a progress bar most often lives: a status bar or an inspector row. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`<div data-density="compact">
      ${stage(
        note(
          'The track is a multiple of the spacing unit, so it thins with the density rather than ' +
            'staying a number somebody typed.',
        ),
        html`
          <div style="display:grid;gap:var(--mjx-density-step);max-inline-size:24rem">
            <mjx-progress id="compact-determinate" label="Saving" value="0.6" readout></mjx-progress>
            <mjx-progress id="compact-indeterminate" label="Contacting the server" indeterminate></mjx-progress>
          </div>
        `,
      )}
    </div>`,
};
