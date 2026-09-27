import { html, type TemplateResult } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { densityModeNames, densityModes } from '../../src/foundations/density.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import { statesFor, tokenDependenciesFor } from './matrix.ts';

/**
 * All four archetypes, in both density modes — **the story the hit-target gate reads.**
 *
 * MJXOFF-182: *"Touch variants at the accessible minimum target size, which is where the compact
 * ribbon on a phone either works or does not."* A ribbon is the densest chrome in the product and
 * compact is where a target quietly falls under the floor, so this story is not a demonstration —
 * it is the fixture `tests/browser/controls.spec.ts` measures, in both modes and again at the phone
 * container preset, on computed dimensions and on the real painted box.
 *
 * The floor is WCAG 2.2's *Target Size (Minimum)*, 24 CSS pixels, and it is the foundations'
 * `.mjx-hit-target` that applies it — not a rule written here. That is the second thing this story
 * proves: every one of the four wears the class, so a component that stopped installing the
 * foundations on its shadow root loses the floor and fails here rather than shipping a ribbon
 * nobody with a trackpad can use.
 */

const conventions = storyConventions({
  statesMatrix: statesFor('button'),
  tokenDependencies: tokenDependenciesFor('toggleButton'),
  keyboard: [
    {
      keys: 'Tab',
      does: 'Reaches every control in both modes. Density changes the spacing between controls, never whether one can be reached.',
    },
  ],
  screenReader:
    'Nothing of its own — density is a visual mode. Each control announces exactly what it ' +
    'announces in the other mode, which is the property that makes it safe to change.',
});

const meta: Meta = {
  title: 'Controls/Density',
  parameters: {
    docs: {
      description: {
        component:
          'Comfortable and compact, side by side. Compact reduces the space between things and ' +
          'reduces the target only as far as the accessible floor — never past it.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const cluster = (): TemplateResult => html`
  <div style="display:flex;flex-wrap:wrap;align-items:center;gap:var(--mjx-density-step)">
    <mjx-button label="Paste" icon="folder-open"></mjx-button>
    <mjx-toggle-button
      label="Bold"
      icon="text-bold"
      size="icon"
      pressed="true"
    ></mjx-toggle-button>
    <mjx-split-button
      label="Undo"
      icon="arrow-undo"
      menu-label="Undo history"
    ></mjx-split-button>
    <mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>
  </div>
`;

/**
 * Both modes, each carrying all four archetypes.
 *
 * `data-density` cascades into a subtree — unlike the colour scheme, which `tokens.css` keys off
 * `:root` — so a compact inspector inside a comfortable shell is one attribute, and this story is
 * two `<div>`s rather than two pages.
 */
export const BothDensities: Story = {
  render: () => html`
    <div
      style="display:grid;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
    >
      ${densityModeNames.map(
        (mode) => html`
          <div data-density=${mode} data-density-probe=${mode}>
            <p
              class=${typeRoleClass('label')}
              style="margin:0 0 var(--mjx-density-step);color:var(--theme-text-secondary)"
            >
              ${mode} — ${densityModes[mode].use}
            </p>
            ${cluster()}
          </div>
        `,
      )}
    </div>
  `,
};
