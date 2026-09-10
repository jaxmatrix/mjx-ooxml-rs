import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  caption,
  furnitureTokenDependencies,
  ladderCaption,
  note,
  stage,
  statusKeyboard,
  statusScreenReader,
  statusStates,
} from './specimens.ts';

/**
 * `<mjx-status-bar>` — three regions, a declared drop order, and two live regions.
 *
 * **Open *The Priority Ladder* first and resize the container**, not the window. Watch the readings
 * leave the bar in the order the ladder declares, and then open the disclosure they went into: they
 * are the *same elements*, moved, not a second copy rendered into a menu. That is MJXOFF-183's rule
 * applied a second time, and it is why nothing here can be lost.
 *
 * **Then open *What Is Announced, And What Is Not*.** Press the button that advances the page
 * number and listen: nothing. It is the reading a person watches while they scroll, and a screen
 * reader that interrupted them on every scroll event would make the document unusable. Then press
 * the one that fails a save, and listen to that.
 */

const conventions = storyConventions({
  statesMatrix: statusStates(),
  tokenDependencies: furnitureTokenDependencies,
  keyboard: statusKeyboard,
  screenReader: statusScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Furniture/Status Bar',
  parameters: {
    docs: {
      description: {
        component:
          'The readings under the document: where you are, how big it is, what it is doing. It ' +
          'degrades by moving segments into an overflow rather than by dropping them, and it is ' +
          'silent by default because the reading a person watches is the one that changes most.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The full bar, as a shell would build it. */
function bar() {
  return html`
    <mjx-status-bar id="bar" label="Document status">
      <mjx-status-segment
        id="page"
        label="Page"
        value="4 of 20"
        priority="essential"
        region="start"
      ></mjx-status-segment>
      <mjx-status-segment
        id="words"
        label="Words"
        value="3,182"
        priority="supplementary"
        region="start"
      ></mjx-status-segment>
      <mjx-status-segment
        id="selection"
        label="Selected"
        value="128 words"
        priority="ancillary"
        region="start"
      ></mjx-status-segment>
      <mjx-status-segment
        id="saved"
        label="Saved"
        value="Just now"
        priority="standard"
        region="centre"
        announce="polite"
      ></mjx-status-segment>
      <mjx-status-segment
        id="language"
        label="Language"
        value="English (UK)"
        priority="ancillary"
        region="end"
      ></mjx-status-segment>
      <mjx-status-segment
        id="view"
        label="View"
        value="Print layout"
        priority="standard"
        region="end"
      ></mjx-status-segment>
      <mjx-status-segment
        id="zoom"
        label="Zoom"
        value="100%"
        priority="essential"
        region="end"
      ></mjx-status-segment>
    </mjx-status-bar>
  `;
}

/** The ladder, which is the whole responsive design. */
export const ThePriorityLadder: Story = {
  name: 'The Priority Ladder',
  render: () =>
    stage(
      note(
        'Resize the container. Segments leave the bar in the order their priority declares, and ' +
          'the disclosure that appears holds the same elements — open it and the readings are ' +
          'still there, with the same accessible names. A segment is never rendered twice, which ' +
          'is what makes “nothing is lost” structural rather than remembered.',
      ),
      bar(),
      caption(ladderCaption()),
    ),
};

/**
 * The live regions, and the reading that goes into neither.
 *
 * The two buttons are the specimen: one moves a routine reading, one reports something that
 * genuinely went wrong. A person auditing this with a screen reader should hear exactly one of them.
 */
export const WhatIsAnnounced: Story = {
  name: 'What Is Announced, And What Is Not',
  render: () => {
    const advance = (event: Event) => {
      const root = (event.target as HTMLElement).getRootNode() as Document | ShadowRoot;
      const segment = root.querySelector('#page');
      const current = Number.parseInt(segment?.getAttribute('value')?.split(' ')[0] ?? '4', 10);
      segment?.setAttribute('value', `${String((current % 20) + 1)} of 20`);
    };
    const fail = (event: Event) => {
      const root = (event.target as HTMLElement).getRootNode() as Document | ShadowRoot;
      root.querySelector('#saved')?.setAttribute('announce', 'assertive');
      root.querySelector('#saved')?.setAttribute('value', 'Could not save — the file is read-only');
    };
    return stage(
      note(
        'Press “Scroll a page” with a screen reader running. Nothing is announced, and that is ' +
          'the assertion: the page number changes constantly and is the reason the bar exists. ' +
          'Then press “Fail a save”, which is a thing a person has to act on, and listen to the ' +
          'alert region carry it.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter)">
          <mjx-button label="Scroll a page" size="small" @click=${advance}></mjx-button>
          <mjx-button label="Fail a save" size="small" @click=${fail}></mjx-button>
        </div>
      `,
      bar(),
    );
  },
};

/** Compact density, at the hit-target floor. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`
      <div data-density="compact">
        ${stage(
          note(
            'A status bar is the surface compact density exists for: it is a strip of readings ' +
              'across the bottom of a window that has better things to do with its height. The ' +
              'overflow disclosure is the thing to measure — it still clears 24 CSS pixels.',
          ),
          bar(),
        )}
      </div>
    `,
};
