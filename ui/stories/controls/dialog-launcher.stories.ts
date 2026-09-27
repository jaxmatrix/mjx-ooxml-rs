import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { statesMatrixStoryName } from '../../src/controls/control-states.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import { specimen, specimenRow, statesFor, statesMatrix, tokenDependenciesFor } from './matrix.ts';

/**
 * `<mjx-dialog-launcher>` — 130 of Office's published controls, and the easiest of the four to
 * ship broken, because it is visually trivial.
 *
 * MJXOFF-182: *it needs a real accessible name, because “dialog launcher” tells a screen-reader
 * user nothing about which dialog.* So `label` is required, nothing is derived from the glyph, and
 * a launcher without one renders a nameless button that **axe rejects** — proved on a deliberately
 * nameless one in `Gates/Control Naming`, in the same shape MJXOFF-180 used to prove the contrast
 * rule can reject.
 *
 * `GUESS:` **the target is deliberately larger than Office's.** Office draws a few pixels of arrow
 * in the corner of a group; this carries the same hit-target floor as every other control, so it
 * is at least 24 CSS pixels in compact and 40 in comfortable. That is a divergence chosen on
 * purpose — a control that cannot be hit is not a control — and it is asserted rather than hoped
 * for.
 */

const conventions = storyConventions({
  statesMatrix: statesFor('dialogLauncher'),
  tokenDependencies: tokenDependenciesFor('dialogLauncher'),
  keyboard: [
    { keys: 'Tab', does: 'Focuses the launcher, which is a tab stop like any other button.' },
    { keys: 'Enter', does: 'Activates it — `mjx-activate`. Opening the dialog is U06’s.' },
    { keys: 'Space', does: 'The same.' },
  ],
  screenReader:
    'Announces the label — “Font settings”, never “dialog launcher” — then “button”, then “opens ' +
    'dialog” from aria-haspopup. There is no aria-expanded: a dialog is not a disclosure.',
});

const meta: Meta = {
  // A string literal, because CSF is indexed statically. See the note in `button.stories.ts`.
  title: 'Controls/Dialog Launcher',
  parameters: {
    docs: {
      description: {
        component:
          'The corner mark that opens a ribbon group’s full dialog. Fluent’s arrow-down-right at ' +
          '16, in a target that meets the accessible minimum.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Every state. **The story the pairwise gate reads.** */
export const TheStatesMatrix: Story = {
  name: statesMatrixStoryName,
  render: () =>
    statesMatrix([
      {
        state: 'rest',
        control: html`<mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>`,
      },
      {
        state: 'hover',
        control: html`<mjx-dialog-launcher
          label="Font settings"
          force-state="hover"
        ></mjx-dialog-launcher>`,
      },
      {
        state: 'active',
        control: html`<mjx-dialog-launcher
          label="Font settings"
          force-state="active"
        ></mjx-dialog-launcher>`,
      },
      {
        state: 'focus',
        control: html`<mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>`,
      },
      {
        state: 'disabled',
        control: html`<mjx-dialog-launcher label="Font settings" disabled></mjx-dialog-launcher>`,
      },
      {
        state: 'unavailable',
        control: html`<mjx-dialog-launcher
          label="Font settings"
          unavailable
          explanation="Font settings are managed by the theme this presentation uses."
        ></mjx-dialog-launcher>`,
      },
    ]),
};

/**
 * Where it actually lives: the bottom-right corner of a ribbon group.
 *
 * The name is the *dialog's*, not the affordance's — “Font settings”, “Paragraph settings” — which
 * is the entire point of the component.
 */
export const InAGroupHeader: Story = {
  render: () =>
    specimenRow([
      specimen(
        'a ribbon group',
        html`
          <mjx-surface level="raised" radius="card">
            <div style="display:grid;gap:var(--mjx-density-step);min-inline-size:12rem">
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
                ></mjx-toggle-button>
              </div>
              <div
                style="display:flex;align-items:center;justify-content:space-between;gap:var(--mjx-density-step)"
              >
                <span
                  class=${typeRoleClass('label')}
                  style="color:var(--theme-text-secondary)"
                  >Font</span
                >
                <mjx-dialog-launcher label="Font settings"></mjx-dialog-launcher>
              </div>
            </div>
          </mjx-surface>
        `,
      ),
    ]),
};
