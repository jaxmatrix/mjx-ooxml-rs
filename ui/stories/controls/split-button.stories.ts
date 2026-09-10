import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { statesMatrixStoryName } from '../../src/controls/control-states.ts';
import { specimen, specimenRow, statesFor, statesMatrix, tokenDependenciesFor } from './matrix.ts';

/**
 * `<mjx-split-button>` — 440 of Office's published controls, and **the archetype whose failures are
 * silent**.
 *
 * Two controls in one box, and the three ways of getting it wrong all look right in a screenshot:
 * a single button with an arrow drawn inside it (the menu is unreachable by keyboard), a `:hover`
 * on the host (both halves light whichever one you are over, so the control never tells you what
 * you are about to press), and one accessible name for two commands.
 *
 * All three are asserted in `tests/browser/controls.spec.ts` — by tabbing to each region, by
 * hovering one region and reading the *other*'s computed paint, and by pressing at a pointer
 * position inside each half. The fourth assertion is the one that protects a person's work:
 * **Arrow Down opens the menu and cannot fire the action.** Somebody who meant to see the paste
 * options and instead pasted has an edit to undo.
 */

const conventions = storyConventions({
  statesMatrix: statesFor('splitButton'),
  tokenDependencies: tokenDependenciesFor('splitButton'),
  keyboard: [
    { keys: 'Tab', does: 'Focuses the primary action.' },
    { keys: 'Tab (again)', does: 'Focuses the menu arrow. Two regions, two tab stops.' },
    { keys: 'Enter / Space (primary)', does: 'Fires the action — `mjx-activate`, and no menu.' },
    { keys: 'Enter / Space (arrow)', does: 'Asks for the menu — `mjx-menu-request`, and no action.' },
    {
      keys: 'Arrow Down',
      does: 'Asks for the menu from either region, and never fires the action. Alt is optional, because both spellings are in common use.',
    },
  ],
  screenReader:
    'Two buttons. The first announces the command’s name and “button”; the second announces its ' +
    'own name — “More Undo options” unless the caller gave one — then “button”, “has pop-up ' +
    'menu”, and “collapsed” or “expanded”. `aria-expanded` follows the host’s attribute and ' +
    'never the event, so a menu that has not been built is never announced as open.',
});

const meta: Meta = {
  // A string literal, because CSF is indexed statically. See the note in `button.stories.ts`.
  title: 'Controls/Split Button',
  parameters: {
    docs: {
      description: {
        component:
          'A primary action and a menu arrow with two independent hit regions, a divider that ' +
          'makes the split legible, and two accessible names.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const undo = { label: 'Undo', icon: 'arrow-undo', menu: 'Undo history' } as const;

/**
 * Every state, on the **primary** region.
 *
 * The arrow's own states are the same table read on the same class, which is what makes the two
 * regions independent; `The Two Regions` below shows them one at a time.
 */
export const TheStatesMatrix: Story = {
  name: statesMatrixStoryName,
  render: () =>
    statesMatrix([
      {
        state: 'rest',
        control: html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
        ></mjx-split-button>`,
      },
      {
        state: 'hover',
        control: html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
          force-state="hover"
        ></mjx-split-button>`,
      },
      {
        state: 'active',
        control: html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
          force-state="active"
        ></mjx-split-button>`,
      },
      {
        state: 'focus',
        control: html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
        ></mjx-split-button>`,
      },
      {
        state: 'disabled',
        control: html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
          disabled
        ></mjx-split-button>`,
      },
      {
        state: 'unavailable',
        control: html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
          unavailable
          explanation="There is nothing to undo in this document yet."
        ></mjx-split-button>`,
      },
    ]),
};

/**
 * One region lit at a time — the story that shows the failure a host `:hover` produces.
 *
 * In the second specimen the primary is hovered and the arrow is at rest; in the third it is the
 * other way round. If a change ever makes both light together, this is where a person sees it and
 * `the two regions are independently hoverable` is where the build fails.
 */
export const TheTwoRegions: Story = {
  render: () =>
    specimenRow([
      specimen(
        'both at rest',
        html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
        ></mjx-split-button>`,
      ),
      specimen(
        'primary hovered',
        html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
          force-state="hover"
        ></mjx-split-button>`,
      ),
      specimen(
        'arrow hovered',
        html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
          force-menu-state="hover"
        ></mjx-split-button>`,
      ),
      specimen(
        'menu open (host-declared)',
        html`<mjx-split-button
          label=${undo.label}
          icon=${undo.icon}
          menu-label=${undo.menu}
          expanded
        ></mjx-split-button>`,
      ),
    ]),
};

/**
 * The three shapes, so the seam can be judged where Office actually puts it.
 *
 * The arrow stays a fixed mark whatever shape the primary takes — a `large` arrow that put its
 * chevron above a hidden label would be a second, wrong reading of the size variant.
 */
export const TheSizeVariants: Story = {
  render: () =>
    specimenRow([
      specimen(
        'large',
        html`<mjx-split-button
          label="Paste"
          icon="folder-open"
          menu-label="Paste options"
          size="large"
        ></mjx-split-button>`,
      ),
      specimen(
        'small',
        html`<mjx-split-button
          label="Paste"
          icon="folder-open"
          menu-label="Paste options"
          size="small"
        ></mjx-split-button>`,
      ),
      specimen(
        'icon',
        html`<mjx-split-button
          label="Paste"
          icon="folder-open"
          menu-label="Paste options"
          size="icon"
        ></mjx-split-button>`,
      ),
    ]),
};
