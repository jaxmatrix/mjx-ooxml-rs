import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { expectsViolations, storyConventions } from '../../src/story/conventions.ts';
import { statesFor, tokenDependenciesFor } from '../controls/matrix.ts';

/**
 * **The naming gate, proved able to reject.**
 *
 * MJXOFF-180 established the shape and the reason: *"a sweep that only asserts that stories pass
 * is satisfied by an accessibility checker with its rules switched off."* Zero violations is what a
 * clean catalogue looks like and it is also what a disabled rule looks like, and nothing in a green
 * run tells them apart.
 *
 * MJXOFF-182 requires that the dialog launcher have *"a real accessible name, not a glyph"*, and
 * the component honours that by **refusing to invent one** — no fallback to the group's heading, no
 * name derived from the Fluent glyph, because each of those produces a plausible announcement that
 * is wrong, and a wrong name is worse than a missing one. What makes that guarantee worth
 * something is this story: a launcher with no `label`, which must violate axe's `button-name` rule.
 * If it ever stops violating, either the component started fabricating a name or the rule stopped
 * running — and both are findings.
 *
 * ⚠ **This is the only place in the catalogue where a shipped component is deliberately misused.**
 * `expectsViolations` in a component's own story file would be an admission; here it is a fixture,
 * and it lives under `Gates/` beside MJXOFF-180's contrast probes for exactly that reason. The
 * console also carries the component's own complaint, which is the other half of the same answer.
 */

const conventions = storyConventions({
  statesMatrix: statesFor('dialogLauncher'),
  tokenDependencies: tokenDependenciesFor('dialogLauncher'),
  keyboard: [
    {
      keys: 'Tab',
      does: 'Reaches the button — which is the problem. It is focusable, activatable and nameless: a screen reader announces “button” and nothing else.',
    },
  ],
  screenReader:
    'Announces “button”, and nothing more. That is the defect, and it is what axe reports as ' +
    'button-name.',
});

const meta: Meta = {
  title: 'Gates/Control Naming',
  parameters: {
    docs: {
      description: {
        component:
          'A deliberately nameless dialog launcher. It exists so the accessibility sweep can be ' +
          'watched rejecting something, which is the only way to know it is switched on.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * A launcher with no `label`. **Must** violate `button-name`.
 *
 * Beside it, the same component with a name, so the story shows the fix as well as the failure.
 */
export const LauncherWithNoName: Story = {
  parameters: expectsViolations(
    ['button-name'],
    'A dialog launcher with no label renders a nameless button on purpose. The component refuses ' +
      'to invent a name from its glyph, so the only thing standing between that and a shipped ' +
      'defect is the accessibility sweep — and a sweep nobody has watched reject anything might ' +
      'be matching nothing.',
  ),
  render: () => html`
    <div
      style="display:flex;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
    >
      <mjx-dialog-launcher></mjx-dialog-launcher>
      <mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>
    </div>
  `,
};
