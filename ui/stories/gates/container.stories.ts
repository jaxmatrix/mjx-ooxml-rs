import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { containerBreakpoints } from '../../dev/probes.ts';
import { containerPresets } from '../../src/harness/resizable-container.ts';

/**
 * The responsive mechanism, demonstrated.
 *
 * `BUILD_PLAN_LOOP_1.md` §3: *"A component that only looks right at a viewport width has not been
 * validated."* The probe below reports which `@container` band it is in and contains no `@media`
 * rule at all, so the only thing that can move it is the container's width.
 *
 * `tests/browser/container.spec.ts` drives all three presets and asserts `window.innerWidth` never
 * changes. That assertion, rather than the picture, is what distinguishes this harness from a
 * viewport toolbar.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'narrow', description: `Container below ${String(containerBreakpoints.medium)}px.` },
    {
      name: 'medium',
      description: `Container between ${String(containerBreakpoints.medium)}px and ${String(containerBreakpoints.wide)}px.`,
    },
    { name: 'wide', description: `Container at or above ${String(containerBreakpoints.wide)}px.` },
  ],
  tokenDependencies: ['theme.light.textPrimary', 'radius.card', 'spacing'],
  keyboard: [
    {
      keys: 'Tab, then Arrow Left / Arrow Right',
      does:
        "Reaches the harness's preset buttons and its width slider. Resizing by keyboard is the " +
        'reason the harness is not a drag handle alone.',
    },
  ],
  screenReader:
    'The harness announces "Container width presets" as a group, each preset button by name and ' +
    'pressed state, and the width slider by its label and value. The probe itself is plain text.',
});

const meta: Meta = {
  title: 'Gates/Container query',
  parameters: {
    docs: {
      description: {
        component:
          `The probe reports \`narrow\` below ${String(containerBreakpoints.medium)}px, \`medium\` ` +
          `below ${String(containerBreakpoints.wide)}px and \`wide\` at or above it. The harness ` +
          `presets are ${Object.entries(containerPresets)
            .map(([name, width]) => `${name} ${String(width)}px`)
            .join(', ')}, so each preset lands in a different band.`,
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Whatever band the current container preset puts it in. */
export const RespondsToItsContainer: Story = {
  render: () => html`<mjx-container-probe></mjx-container-probe>`,
};

/**
 * Two probes in one story, in containers of different widths, at one viewport.
 *
 * This is the shape of the argument in a single picture: the same component reports two different
 * bands at the same instant, which no viewport mechanism can produce.
 */
export const TwoWidthsAtOneViewport: Story = {
  render: () => html`
    <div style="display:flex;gap:16px;align-items:flex-start;flex-wrap:wrap">
      <mjx-resizable-container width="380" style="flex:0 0 auto">
        <mjx-container-probe></mjx-container-probe>
      </mjx-resizable-container>
      <mjx-resizable-container width="1200" style="flex:0 0 auto">
        <mjx-container-probe></mjx-container-probe>
      </mjx-resizable-container>
    </div>
  `,
};
