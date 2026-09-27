import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  controlSizeNames,
  controlSizes,
  statesMatrixStoryName,
} from '../../src/controls/control-states.ts';
import { specimen, specimenRow, statesFor, statesMatrix, tokenDependenciesFor } from './matrix.ts';

/**
 * `<mjx-button>` — 8,687 of Office's 15,346 published controls, which is why it is the first
 * component this catalogue grows.
 *
 * The story worth opening first is **The States Matrix**. Every cell in it is a real control in a
 * real state — the two that cannot happen in a static page, `hover` and `active`, are produced by
 * the same CSS rule the pointer produces, not by a copy of it — and
 * `tests/browser/controls.spec.ts` requires every pair of them to compute differently **in both
 * schemes**. A matrix that lists six states and renders four is the failure MJXOFF-182 is built
 * around, and it is the one a screenshot cannot see.
 */

const conventions = storyConventions({
  statesMatrix: statesFor('button'),
  tokenDependencies: tokenDependenciesFor('button'),
  keyboard: [
    { keys: 'Tab', does: 'Focuses the button and shows the foundations’ focus ring.' },
    { keys: 'Enter', does: 'Activates it: one `mjx-activate` event, and exactly one.' },
    { keys: 'Space', does: 'The same. Both keys, because a button is operated with both.' },
    {
      keys: 'Tab (disabled)',
      does: 'Skips it entirely. A hard-disabled button is out of the tab order, which is the platform’s own behaviour and not something re-implemented here.',
    },
    {
      keys: 'Tab (unavailable)',
      does: 'Reaches it. An unavailable-but-explained command stays focusable so its explanation can be read; Enter and Space still do nothing.',
    },
  ],
  screenReader:
    'Announces the label, then “button”. Unavailable: the label, “button”, “dimmed”, then the ' +
    'explanation — which is the whole reason that state exists rather than a plain disabled one. ' +
    'At size="icon" the label is off-screen and still announced.',
});

const meta: Meta = {
  // ⚠ A **string literal**, and it must stay one. Storybook indexes CSF statically and refuses a
  // computed title — *"CSF: unexpected dynamic title"* — so `archetypeStoryTitle.button` cannot be
  // written here even though it holds exactly this string. This is the same static-indexing
  // constraint that made MJXOFF-180's `defineStoryMeta` wrapper impossible, met in a second place.
  // The two are kept honest by the gate rather than by the type system:
  // `tests/browser/controls.spec.ts` looks every story up by title and fails with *"is missing
  // from the catalogue"* the moment they diverge.
  title: 'Controls/Button',
  parameters: {
    docs: {
      description: {
        component:
          'The ribbon command button, in the three shapes Office uses and the six states it can ' +
          'be in. Switch the Theme toolbar: the pairwise gate checks both schemes, and so should ' +
          'a person.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * Every state, side by side. **This is the story the pairwise gate reads.**
 *
 * `hover` and `active` are forced with `force-state`, which is an audit affordance and not a mock:
 * the forced selector and the real pseudo-class share one declaration block in
 * `control-states.ts`, and the gate additionally drives a real pointer over a control and requires
 * the two to compute identically.
 */
export const TheStatesMatrix: Story = {
  name: statesMatrixStoryName,
  render: () =>
    statesMatrix([
      { state: 'rest', control: html`<mjx-button label="Bold" icon="text-bold"></mjx-button>` },
      {
        state: 'hover',
        control: html`<mjx-button
          label="Bold"
          icon="text-bold"
          force-state="hover"
        ></mjx-button>`,
      },
      {
        state: 'active',
        control: html`<mjx-button
          label="Bold"
          icon="text-bold"
          force-state="active"
        ></mjx-button>`,
      },
      { state: 'focus', control: html`<mjx-button label="Bold" icon="text-bold"></mjx-button>` },
      {
        state: 'disabled',
        control: html`<mjx-button label="Bold" icon="text-bold" disabled></mjx-button>`,
      },
      {
        state: 'unavailable',
        control: html`<mjx-button
          label="Bold"
          icon="text-bold"
          unavailable
          explanation="Bold is unavailable because the selection is inside a heading style."
        ></mjx-button>`,
      },
    ]),
};

/**
 * The three shapes.
 *
 * `large` puts the icon above the label and lets it wrap to two lines; `small` puts it beside;
 * `icon` draws no label and **still announces one** — the span is moved off-screen rather than
 * removed, which is the difference between an icon toolbar a screen-reader user can use and one
 * they cannot.
 */
export const TheSizeVariants: Story = {
  render: () =>
    specimenRow(
      controlSizeNames.map((size) =>
        specimen(
          `${size} · ${String(controlSizes[size].iconSize)}px glyph`,
          html`<mjx-button label="Paste" icon="folder-open" size=${size}></mjx-button>`,
        ),
      ),
    ),
};

/**
 * A long command name in a `large` button, which is where Office's two-line label lives.
 *
 * The clamp is asserted rather than eyeballed: `tests/browser/controls.spec.ts` measures the
 * label's height against its own line height and requires exactly two lines, and requires the
 * button not to have grown wider than the size's bound — a label that "wraps" by making the button
 * three words wide has not wrapped.
 */
export const TwoLineLabels: Story = {
  render: () =>
    specimenRow([
      specimen(
        'one word',
        html`<mjx-button label="Undo" icon="arrow-undo" size="large"></mjx-button>`,
      ),
      specimen(
        'two lines',
        html`<mjx-button
          label="Conditional Formatting"
          icon="text-bold"
          size="large"
        ></mjx-button>`,
      ),
      specimen(
        'clamped',
        html`<mjx-button
          label="Insert Function Reference Argument"
          icon="document"
          size="large"
        ></mjx-button>`,
      ),
    ]),
};

/**
 * The state Office uses far more than a plain disabled one: **unavailable, and explained.**
 *
 * It is still in the tab order, still announced, and still carries the reason — and it still
 * cannot be activated, by pointer or by either key. A command a person cannot reach is a command
 * whose reason they can never read.
 */
export const UnavailableAndExplained: Story = {
  render: () =>
    specimenRow([
      specimen(
        'disabled',
        html`<mjx-button label="Paste" icon="folder-open" disabled></mjx-button>`,
      ),
      specimen(
        'unavailable',
        html`<mjx-button
          label="Paste"
          icon="folder-open"
          unavailable
          explanation="There is nothing on the clipboard."
        ></mjx-button>`,
      ),
    ]),
};
