import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { firstColorToken } from '../../dev/probes.ts';
import { customProperties } from '../../tokens/tokens.ts';

/**
 * The resolution order, demonstrated.
 *
 * `DESIGN_TOKENS.md` §3: *explicit host configuration → CSS custom properties read off the host
 * element → built-in defaults*, and *"that resolution order is what satisfies the original brief:
 * if tokens are set, adopt them."*
 *
 * The probe reports both the resolved value and **which leg answered**, and
 * `tests/browser/token-resolution.spec.ts` asserts the order by setting a custom property on the
 * host, watching the probe adopt it, removing it, and watching the generated default return. The
 * origin is the half that makes the assertion meaningful: a probe that only reported the value
 * would pass whenever the answer happened to be right.
 */

const token = firstColorToken();
const property = customProperties[token];

const conventions = storyConventions({
  statesMatrix: [
    { name: 'default', description: 'Nothing set: the generated default answers.' },
    {
      name: 'host',
      description: 'A CSS custom property on the host element answers, and the default is ignored.',
    },
    {
      name: 'explicit',
      description:
        'Programmatic configuration answers, over the top of a host property that is also set.',
    },
  ],
  tokenDependencies: [token, 'theme.light.textPrimary', 'theme.light.border'],
  keyboard: [{ keys: 'Tab', does: 'Nothing: the probe is a swatch and a caption.' }],
  screenReader:
    'The caption reads the token path, its resolved value and the leg that answered, in that ' +
    'order, as ordinary text.',
});

const meta: Meta = {
  title: 'Gates/Token resolution',
  parameters: {
    docs: {
      description: {
        component:
          `The probe resolves \`${token}\` (\`${String(property)}\`) and reports which leg of the ` +
          'resolution order answered. Set that custom property on the probe element and the ' +
          'origin changes from `default` to `host`.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** No override anywhere: the generated default answers. */
export const GeneratedDefault: Story = {
  render: () => html`<mjx-token-probe token=${token}></mjx-token-probe>`,
};

/**
 * The same probe with the custom property set on the host element.
 *
 * The value is a sentinel rather than another token, deliberately: the assertion is that the
 * *host's* value won, and a second token could coincide with the default.
 */
export const HostPropertyWins: Story = {
  render: () => html`
    <mjx-token-probe token=${token} style=${`${String(property)}: #ff00ff`}></mjx-token-probe>
  `,
};

/**
 * Explicit configuration, over the top of a host property that is also set.
 *
 * The host property here is not decoration: with it absent, an explicit value that won would look
 * exactly like a default that happened to match, and the *order* would be unproved.
 */
export const ExplicitWins: Story = {
  render: () => html`
    <mjx-token-probe
      token=${token}
      explicit="#00ffff"
      style=${`${String(property)}: #ff00ff`}
    ></mjx-token-probe>
  `,
};

/** All three legs at once, which is how the order is visible rather than inferred. */
export const AllThreeLegs: Story = {
  render: () => html`
    <div style="display:flex;flex-direction:column;gap:16px">
      <mjx-token-probe token=${token}></mjx-token-probe>
      <mjx-token-probe token=${token} style=${`${String(property)}: #ff00ff`}></mjx-token-probe>
      <mjx-token-probe
        token=${token}
        explicit="#00ffff"
        style=${`${String(property)}: #ff00ff`}
      ></mjx-token-probe>
    </div>
  `,
};
