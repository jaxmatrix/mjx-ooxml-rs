import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  focusIndicatorMinimumContrast,
  focusRingDefaults,
} from '../../src/foundations/focus.ts';
import { surfaceLevelNames, surfaceLevels } from '../../src/foundations/surfaces.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';

/**
 * One focus ring, on every surface, in both themes.
 *
 * MJXOFF-181's sharpest warning is about this page:
 *
 * > **Focus is invisible in a static story.** A focus ring nobody focuses is untested — drive it
 * > with real keyboard interaction, and assert it is visible against **both** themes, since a ring
 * > that passes on white can vanish on the dark backdrop.
 *
 * So the assertions are not here. `tests/browser/foundations.spec.ts` presses `Tab` — a real key
 * press, through the browser's own input path, so `:focus-visible` makes its own judgement rather
 * than being asserted into existence — walks every rung of the elevation ladder in **both**
 * schemes, and for each one reads the computed outline colour and measures its contrast against
 * the surface behind it. `tests/foundations.test.ts` does the same arithmetic ahead of time from
 * the generated tokens, so a palette re-seed that made the ring invisible on one rung fails a gate
 * that names the rung.
 *
 * What this page is for is the other half: **look at it, and Tab through it.** Click a button and
 * the ring does not appear; Tab to it and it does. That is `:focus-visible`, and it is the
 * behaviour the whole treatment is built on.
 */

const conventions = storyConventions({
  statesMatrix: [
    {
      name: 'resting',
      description: 'No ring. Nothing is focused.',
    },
    {
      name: 'focus-visible (keyboard)',
      description: `Tab to a button: a ${focusRingDefaults.width} ring in --theme-accent-pressed, offset by the same amount.`,
    },
    {
      name: 'focus, not visible (pointer)',
      description:
        'Click a button: it is focused and there is no ring. The treatment is keyboard-only by default, and this is the state that proves it.',
    },
    ...surfaceLevelNames.map((level) => ({
      name: `on ${level}`,
      description: `${surfaceLevels[level].use} The ring must clear ${String(focusIndicatorMinimumContrast)} : 1 against this rung in both schemes.`,
    })),
  ],
  tokenDependencies: [
    'theme.light.accentPressed',
    'theme.dark.accentPressed',
    'theme.light.accentSurface',
    'theme.light.secondarySurface',
    'theme.light.surface',
    'theme.light.surfaceRaised',
    'theme.light.background',
    'spacing',
    'radius.control',
  ],
  keyboard: [
    { keys: 'Tab', does: 'Moves to the next button and shows the ring.' },
    { keys: 'Shift + Tab', does: 'Moves back, and shows the ring.' },
    {
      keys: 'Click',
      does: 'Focuses the button and shows no ring. Not a keyboard behaviour, and listed because its absence is the thing being promised.',
    },
  ],
  screenReader:
    'Each button announces its rung — "base, button", "sunken, button", and so on. The ring is ' +
    'a visual affordance and announces nothing; what a screen-reader user needs is the focus ' +
    'order, which is the source order here.',
});

const meta: Meta = {
  title: 'Foundations/Focus',
  parameters: {
    docs: {
      description: {
        component:
          'Tab through this page rather than reading it. Then switch the Theme toolbar to Dark ' +
          'and Tab through it again — a ring that reads on white can vanish on a dark backdrop, ' +
          'and both are gated.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * A real `<button>` on every rung of the ladder.
 *
 * Native buttons, not `tabindex` on a `<div>`: `:focus-visible` is a *user-agent* judgement about
 * how the focus was reached, and its heuristic differs between the two. A probe built on the easy
 * case would prove the ring on the case that was never in doubt.
 */
export const EveryRungOfTheLadder: Story = {
  render: () => html`
    <div style="padding:var(--mjx-density-gutter)">
      <p class=${typeRoleClass('body')} style="margin:0 0 var(--mjx-density-gutter)">
        Press <kbd>Tab</kbd>. Then click one, and watch the ring not appear.
      </p>
      <mjx-focus-probe></mjx-focus-probe>
    </div>
  `,
};

/**
 * The harness's own chrome, focused.
 *
 * The container's preset buttons and its width slider used to carry a focus rule of their own —
 * two `2px` literals written before this foundation existed. They now adopt the shared treatment
 * like everything else, which makes the harness a *consumer* of the gate rather than an exception
 * to it, and is why `tests/browser/foundations.spec.ts` tabs into it deliberately.
 */
export const TheHarnessUsesItToo: Story = {
  render: () => html`
    <div class=${typeRoleClass('body')} style="padding:var(--mjx-density-gutter)">
      Press <kbd>Tab</kbd> from here: the first three stops are the container's own preset buttons,
      then its width slider. They wear the same ring as everything below them, from the same
      declaration.
    </div>
  `,
};
