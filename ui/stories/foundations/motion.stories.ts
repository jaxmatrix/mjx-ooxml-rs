import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { motionRoleNames, motionRoles } from '../../src/foundations/motion.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';

/**
 * The motion vocabulary — five roles over three easing tokens.
 *
 * `DESIGN_TOKENS.md` §4:
 *
 * > **`--ease-spring` overshoots.** Good for a panel or sheet entering; wrong for anything attached
 * > to a document object, where overshoot reads as imprecision. Use `--ease-ink` for selection,
 * > handles and canvas motion.
 *
 * That sentence has been true and unenforced since the tokens were generated. `motion.ts` records
 * for every role whether it is attached to a document object, and `tests/foundations.test.ts`
 * fails if any role that is uses the overshooting easing — so a later child animating a selection
 * handle with `--ease-spring` fails a gate rather than shipping motion that reads as imprecision.
 *
 * ## Watch the ends
 *
 * The boxes below all travel the same distance in the same time. The difference is entirely in the
 * last fifth: `panelEnter` and `sheetEnter` go past their stop and come back; `selection` and
 * `documentObject` arrive and stay. On a panel that reads as weight. On a selection handle it reads
 * as *the object moved and then moved again*, which is the failure §4 is describing.
 */

const conventions = storyConventions({
  statesMatrix: motionRoleNames.map((role) => ({
    name: role,
    description: `${motionRoles[role].easing} · ${motionRoles[role].attachedToDocumentObject ? 'attached to a document object — overshoot forbidden' : 'chrome'} · ${motionRoles[role].use}`,
  })),
  tokenDependencies: [
    'ease.ink',
    'ease.outSoft',
    'ease.spring',
    'duration.transition',
    'theme.light.accent',
    'theme.light.background',
    'radius.chip',
  ],
  keyboard: [
    {
      keys: 'Tab',
      does: 'Nothing. The boxes are decoration; nothing on this page is interactive, and that is deliberate — a motion demonstration that needed a button would be a button story.',
    },
  ],
  screenReader:
    'Nothing is announced. Everything moving here is presentational, and a person who has asked ' +
    'their system to reduce motion sees it stationary: the reduced-motion rule is in the ' +
    'foundations stylesheet, stated once for everything carrying a motion class.',
});

const meta: Meta = {
  title: 'Foundations/Motion',
  parameters: {
    docs: {
      description: {
        component:
          'Five roles, three easings, and the one rule that is enforced by a test: nothing ' +
          'attached to a document object may overshoot.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** All five roles running side by side. Watch where each one stops. */
export const TheFiveRoles: Story = {
  render: () => html`
    <div style="padding:var(--mjx-density-gutter)">
      <mjx-motion-probe></mjx-motion-probe>
    </div>
  `,
};

/**
 * The vocabulary, written down — which easing, whether it may overshoot, and what it is for.
 *
 * A table rather than a paragraph, because the thing a later child needs is *which one do I use
 * here*, and the answer is a lookup.
 */
export const TheVocabulary: Story = {
  render: () => html`
    <div
      class=${typeRoleClass('dense')}
      style="padding:var(--mjx-density-gutter);display:flex;flex-direction:column;gap:var(--mjx-density-step)"
    >
      ${motionRoleNames.map(
        (role) => html`
          <div
            data-motion-row=${role}
            style="display:grid;grid-template-columns:minmax(9ch,auto) minmax(18ch,auto) 1fr;gap:var(--mjx-density-gutter);align-items:baseline;padding:var(--mjx-density-step);background:var(--theme-surface);border-radius:var(--radius-chip)"
          >
            <strong>${role}</strong>
            <code style="font-family:var(--font-mono)">${motionRoles[role].easingToken}</code>
            <span style="color:var(--theme-text-secondary)">
              ${motionRoles[role].attachedToDocumentObject
                ? 'attached to a document object — overshoot forbidden. '
                : 'chrome. '}${motionRoles[role].use}
            </span>
          </div>
        `,
      )}
    </div>
  `,
};
