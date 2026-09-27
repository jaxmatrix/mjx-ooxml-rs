import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  documentBody,
  note,
  stage,
  surfaceKeyboard,
  surfaceScreenReader,
  surfaceStatesFor,
  surfaceTokenDependencies,
} from './specimens.ts';

/**
 * `<mjx-popover>` — the two anchored surfaces, and the one question that separates them.
 *
 * **Open *One Stop Or Many* first, with a keyboard.** The popover holds a single control: Tab out
 * of it and it closes, because a surface whose `Tab` means *leave* has to close when focus leaves.
 * The flyout beside it holds four: Tab wraps inside it, because `Tab` is now the navigation and
 * cannot also be the exit. That is U05's rule, and neither behaviour is a preference.
 *
 * **Then open *A Popover That Disagrees With Itself*.** It declares `kind="popover"` and holds four
 * controls, which is a real defect — a person Tabbing to the second one would lose the surface. The
 * component says so on the console rather than silently doing the wrong thing, and the browser gate
 * counts the stops with real key presses.
 */

const conventions = storyConventions({
  statesMatrix: surfaceStatesFor(['popover', 'flyout']),
  tokenDependencies: surfaceTokenDependencies,
  keyboard: surfaceKeyboard,
  screenReader: surfaceScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Surfaces/Popover',
  parameters: {
    docs: {
      description: {
        component:
          'A popover and a flyout: the same anchored geometry, told apart by how many tab stops ' +
          'are inside, which is what decides whether Tab means leave or move within.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const buttonStyle =
  'padding-inline:var(--mjx-density-gutter);border-radius:var(--radius-control);' +
  'border:1px solid var(--theme-border);background:var(--theme-surface);' +
  'color:var(--theme-text-primary);cursor:pointer';

const fieldStyle =
  'padding:var(--mjx-density-step);border-radius:var(--radius-control);' +
  'border:1px solid var(--theme-border);background:var(--theme-surface);' +
  'color:var(--theme-text-primary)';

function anchor(id: string, label: string) {
  return html`
    <button slot="anchor" id="${id}" class="mjx-type-control mjx-hit-target" style=${buttonStyle}>
      ${label}
    </button>
  `;
}

/** One control in one, four in the other. */
export const OneStopOrMany: Story = {
  name: 'One Stop Or Many',
  render: () =>
    stage(
      note(
        'The left one is a disclosure and closes when focus leaves it. The right one is a trap ' +
          'and does not, because its four controls need Tab for themselves. Both close on Escape ' +
          'and on a click outside, and both put the keyboard back on their own button.',
      ),
      html`
        <div style="display:flex;gap:calc(var(--mjx-density-gutter) * 2);align-items:start">
          <mjx-popover id="disclosure" label="Word count">
            ${anchor('disclosure-anchor', 'Word count…')}
            <button id="recount" class="mjx-type-control mjx-hit-target" style=${buttonStyle}>
              Recount
            </button>
          </mjx-popover>

          <mjx-popover id="trapping" label="Line spacing" kind="flyout">
            ${anchor('trapping-anchor', 'Line spacing…')}
            <label class="mjx-type-control" style="display:grid;gap:var(--mjx-density-step)">
              Before
              <input id="spacing-before" style=${fieldStyle} value="0 pt" />
            </label>
            <label class="mjx-type-control" style="display:grid;gap:var(--mjx-density-step)">
              After
              <input id="spacing-after" style=${fieldStyle} value="8 pt" />
            </label>
            <button id="spacing-apply" class="mjx-type-control mjx-hit-target" style=${buttonStyle}>
              Apply
            </button>
          </mjx-popover>
        </div>
      `,
      documentBody(3),
    ),
};

/** The declared row and the counted stops disagreeing. */
export const DisagreesWithItself: Story = {
  name: 'A Popover That Disagrees With Itself',
  render: () =>
    stage(
      note(
        'This one declares kind="popover" and holds three controls. Tab twice inside it and it ' +
          'closes under you, which is exactly the defect. Open the console: the component names ' +
          'the disagreement and says which attribute to change.',
      ),
      html`
        <mjx-popover id="disagreeing" label="Too many stops" open>
          ${anchor('disagreeing-anchor', 'Three controls in a popover')}
          <input id="wrong-one" aria-label="First value" style=${fieldStyle} value="one" />
          <input id="wrong-two" aria-label="Second value" style=${fieldStyle} value="two" />
          <button id="wrong-three" class="mjx-type-control mjx-hit-target" style=${buttonStyle}>
            three
          </button>
        </mjx-popover>
      `,
    ),
};

/** At the bottom of its room, so the placement has to flip. */
export const AtTheBottomOfItsRoom: Story = {
  name: 'At The Bottom Of Its Room',
  render: () =>
    html`
      <div
        style="block-size:22rem;display:flex;flex-direction:column;justify-content:flex-end;
               padding:calc(var(--mjx-density-gutter) * 2);background:var(--theme-background)"
      >
        ${note(
          'There is no room below the button, so the surface flips above it — the same ' +
            'flip-shift-constrain arithmetic a menu uses, from the same function.',
        )}
        <mjx-popover id="flipping" label="Word count" open>
          ${anchor('flipping-anchor', 'Word count…')}
          <p style="margin:0" class="mjx-type-body">1,284 words.</p>
          <button id="flipping-recount" class="mjx-type-control mjx-hit-target" style=${buttonStyle}>
            Recount
          </button>
        </mjx-popover>
      </div>
    `,
};

/** Right to left, where the inline axis mirrors and the block axis does not. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () =>
    html`
      <div dir="rtl">
        ${stage(
          note(
            'The surface still opens below its anchor — the block axis does not mirror — and its ' +
              'start alignment is now the anchor’s right edge.',
          ),
          html`
            <mjx-popover id="rtl" label="عدد الكلمات" open align="start">
              ${anchor('rtl-anchor', 'عدد الكلمات…')}
              <p style="margin:0" class="mjx-type-body">١٬٢٨٤ كلمة.</p>
            </mjx-popover>
          `,
        )}
      </div>
    `,
};
