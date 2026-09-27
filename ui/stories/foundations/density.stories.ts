import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  accessibleHitTargetMinimum,
  densityModeNames,
  densityModes,
} from '../../src/foundations/density.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';

/**
 * Density — comfortable and compact.
 *
 * MJXOFF-181: *"comfortable and compact, since a formula bar and a ribbon cannot share one spacing
 * scale"*, gated as *"density modes change spacing without changing hit-target size below the
 * accessible minimum, asserted on computed values."*
 *
 * Those pull in opposite directions on purpose, and the resolution is the design: **compact
 * reduces the space between things, and reduces the target only as far as the floor.** The floor is
 * WCAG 2.2's *Target Size (Minimum)*, and it is applied in the stylesheet with a `max()` rather
 * than documented — so no future density mode can go under it whatever multiplier it chooses.
 *
 * `tests/browser/foundations.spec.ts` reads both, on computed values: the gap between rows **must**
 * differ between the two modes, and the hit target **must** stay at or above the floor in both. A
 * compact mode that changed nothing passes the second assertion and fails the first, which is what
 * makes the pair non-vacuous.
 */

const conventions = storyConventions({
  statesMatrix: densityModeNames.map((mode) => ({
    name: mode,
    description: `${densityModes[mode].use} Step ${String(densityModes[mode].stepUnits)}×--spacing, gutter ${String(densityModes[mode].gutterUnits)}×, hit target ${String(densityModes[mode].hitTargetUnits)}×.`,
  })),
  tokenDependencies: [
    'spacing',
    'text.xs',
    'leading.tight',
    'theme.light.surface',
    'theme.light.background',
    'radius.chip',
    'radius.card',
  ],
  keyboard: [
    {
      keys: 'Tab',
      does: 'Nothing here. Density changes the size of a target, not whether it is one — the rows on this page are rows, not controls.',
    },
  ],
  screenReader:
    'Nothing of its own. Density is entirely visual: the same content is announced identically ' +
    'in both modes, which is the property that makes it safe to switch.',
});

const meta: Meta = {
  title: 'Foundations/Density',
  parameters: {
    docs: {
      description: {
        component:
          'Comfortable and compact, side by side, so the difference is a comparison rather than ' +
          'a memory. The hit target never goes below WCAG 2.2’s minimum of ' +
          `${String(accessibleHitTargetMinimum)} CSS pixels in either, and the floor is applied ` +
          'with a max() in the stylesheet rather than left to the multipliers.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The two modes side by side. Everything about them except the spacing is identical. */
export const BothModes: Story = {
  render: () => html`
    <div
      style="display:grid;grid-template-columns:repeat(auto-fit,minmax(14rem,1fr));gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
    >
      ${densityModeNames.map(
        (mode) => html`
          <div style="display:flex;flex-direction:column;gap:var(--mjx-density-step)">
            <span class=${typeRoleClass('label')}>${mode}</span>
            <mjx-density-probe data-mode=${mode} density=${mode}></mjx-density-probe>
            <span class=${typeRoleClass('dense')} style="color:var(--theme-text-secondary)"
              >${densityModes[mode].use}</span
            >
          </div>
        `,
      )}
    </div>
  `,
};

/**
 * A compact inspector inside a comfortable shell.
 *
 * This is the difference from the colour scheme worth knowing before assuming both work the same
 * way. `tokens.css` keys its scheme layer off `:root`, so `data-theme="dark"` on a `<div>` does
 * **nothing** and a docked dark task pane inside a light shell is not possible today. Density is
 * an ordinary custom property on an ordinary attribute selector, so it *does* cascade into a
 * subtree — and this story is what proves it, with the browser gate reading the computed values on
 * both sides of the boundary.
 */
export const NestedInsideAComfortableShell: Story = {
  render: () => html`
    <mjx-surface
      data-shell="comfortable"
      level="raised"
      density="comfortable"
      style="margin:calc(var(--mjx-density-gutter) * 2);display:flex;flex-direction:column;gap:var(--mjx-density-gutter)"
    >
      <span class=${typeRoleClass('label')}>comfortable shell</span>
      <mjx-density-probe data-mode="inherited"></mjx-density-probe>
      <mjx-surface data-pane="compact" level="sunken" density="compact">
        <span class=${typeRoleClass('label')}>compact inspector</span>
        <mjx-density-probe data-mode="nested"></mjx-density-probe>
      </mjx-surface>
    </mjx-surface>
  `,
};
