import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { statesMatrixStoryName } from '../../src/controls/control-states.ts';
import { specimen, specimenRow, statesFor, statesMatrix, tokenDependenciesFor } from './matrix.ts';

/**
 * `<mjx-toggle-button>` — 1,261 of Office's published controls, and the archetype with the classic
 * failure in it.
 *
 * MJXOFF-182: *"a pressed state that is **distinguishable from hover at a glance**."* Both states
 * want to say *something is happening here*, both reach for the same tint, and a matrix that lists
 * them separately renders them alike. The answer here is that they do not share a colour family at
 * all — hover is neutral, pressed is the accent tint, and pressed-while-hovered thickens an edge
 * instead of deepening a fill — and the ten cells below are checked pairwise, in both schemes, on
 * what the browser computed.
 *
 * The third position is the one that matters in a real editor: `mixed`, for a selection that
 * disagrees with itself. It takes the honey half of the palette rather than a paler green,
 * because *a mixed selection is a disagreement, not a weaker agreement* — and because a paler
 * green is precisely the pair of states the gate exists to reject.
 */

const conventions = storyConventions({
  statesMatrix: statesFor('toggleButton'),
  tokenDependencies: tokenDependenciesFor('toggleButton'),
  keyboard: [
    { keys: 'Tab', does: 'Focuses the toggle.' },
    {
      keys: 'Enter',
      does: 'Moves it and reports: `aria-pressed` changes before the `mjx-change` event is emitted, so a listener never reads the old value.',
    },
    { keys: 'Space', does: 'The same.' },
    {
      keys: 'Enter / Space (mixed)',
      does: 'Takes a mixed selection to pressed — GUESS: what Word appears to do, and not checked against Office.',
    },
    {
      keys: 'Tab (unavailable)',
      does: 'Reaches it, announces the explanation, and refuses to move.',
    },
  ],
  screenReader:
    'Announces the label, “toggle button”, then “pressed”, “not pressed” or “partially ' +
    'pressed” — `aria-pressed` is written for all three positions, including false, so the ' +
    'control never sounds like an ordinary button while it is off.',
});

const meta: Meta = {
  // A string literal, because CSF is indexed statically. See the note in `button.stories.ts`.
  title: 'Controls/Toggle Button',
  parameters: {
    docs: {
      description: {
        component:
          'A command that holds a state, in ten of them. Pressed versus hover is the pair to ' +
          'look at, and mixed versus pressed is the one an editor actually needs.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const bold = { label: 'Bold', icon: 'text-bold' } as const;

/** Every state, side by side. **The story the pairwise gate reads.** */
export const TheStatesMatrix: Story = {
  name: statesMatrixStoryName,
  render: () =>
    statesMatrix([
      {
        state: 'rest',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
        ></mjx-toggle-button>`,
      },
      {
        state: 'hover',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          force-state="hover"
        ></mjx-toggle-button>`,
      },
      {
        state: 'active',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          force-state="active"
        ></mjx-toggle-button>`,
      },
      {
        state: 'focus',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
        ></mjx-toggle-button>`,
      },
      {
        state: 'disabled',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          disabled
        ></mjx-toggle-button>`,
      },
      {
        state: 'unavailable',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          unavailable
          explanation="Bold is fixed by the heading style this paragraph uses."
        ></mjx-toggle-button>`,
      },
      {
        state: 'on',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          pressed="true"
        ></mjx-toggle-button>`,
      },
      {
        state: 'onHover',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          pressed="true"
          force-state="hover"
        ></mjx-toggle-button>`,
      },
      {
        state: 'mixed',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          pressed="mixed"
        ></mjx-toggle-button>`,
      },
      {
        state: 'mixedHover',
        control: html`<mjx-toggle-button
          label=${bold.label}
          icon=${bold.icon}
          pressed="mixed"
          force-state="hover"
        ></mjx-toggle-button>`,
      },
    ]),
};

/**
 * The three positions together, which is the comparison a person actually has to make.
 *
 * Off, on and mixed at the same size, in the same row: if `mixed` reads as a dimmer `on` rather
 * than as *a different answer*, this is where it shows.
 */
export const TheThreePositions: Story = {
  render: () =>
    specimenRow([
      specimen(
        'off — nothing in the selection is bold',
        html`<mjx-toggle-button label="Bold" icon="text-bold"></mjx-toggle-button>`,
      ),
      specimen(
        'on — all of it is',
        html`<mjx-toggle-button label="Bold" icon="text-bold" pressed="true"></mjx-toggle-button>`,
      ),
      specimen(
        'mixed — the selection disagrees',
        html`<mjx-toggle-button label="Bold" icon="text-bold" pressed="mixed"></mjx-toggle-button>`,
      ),
    ]),
};

/**
 * A formatting cluster, icon-only, the way a ribbon actually shows it.
 *
 * The pressed member draws Fluent's **filled** glyph rather than a tinted regular one — which is
 * what the icon manifest's toggle rows were requested for — so the state is legible even where a
 * fill is not.
 */
export const AFormattingCluster: Story = {
  render: () =>
    specimenRow([
      html`
        <div style="display:flex;gap:var(--mjx-density-step)">
          <mjx-toggle-button
            label="Bold"
            icon="text-bold"
            size="icon"
            pressed="true"
          ></mjx-toggle-button>
          <mjx-toggle-button label="Italic" icon="text-italic" size="icon"></mjx-toggle-button>
          <mjx-toggle-button
            label="Underline"
            icon="text-underline"
            size="icon"
            pressed="mixed"
          ></mjx-toggle-button>
        </div>
      `,
    ]),
};
