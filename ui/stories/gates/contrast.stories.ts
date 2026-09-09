import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { expectsViolations, storyConventions } from '../../src/story/conventions.ts';
import { contrastExemplars, formatRatio, measurePalette } from '../../dev/contrast.ts';
import { tokens } from '../../tokens/tokens.ts';

/**
 * The contrast gate, demonstrated.
 *
 * `DESIGN_TOKENS.md` §2.2 states the rule and says what it is:
 *
 * > **This is no longer advice — it is a build failure.**
 *
 * These stories are what makes that true on the TypeScript side. Two of them are *supposed* to be
 * rejected, and `tests/browser/a11y.spec.ts` asserts that they are: a sweep that only checked that
 * good stories pass would be satisfied by an a11y addon with its rules switched off.
 */

const exemplars = contrastExemplars();
const background = exemplars.background;

const conventions = storyConventions({
  statesMatrix: [
    {
      name: 'machinery rejected',
      description:
        'A colour that is not a token at all, guaranteed to fail. Proves the checker runs even if ' +
        'every token in the palette were legal.',
    },
    {
      name: 'token rejected',
      description: `${exemplars.rejected.path} as body text on ${background} — ${formatRatio(exemplars.rejected.ratio)}.`,
    },
    {
      name: 'token accepted',
      description: `${exemplars.accepted.path} as body text on ${background} — ${formatRatio(exemplars.accepted.ratio)}.`,
    },
    {
      name: 'ledger',
      description: 'Every colour token measured against the same background, pass or fail stated.',
    },
  ],
  tokenDependencies: ['theme.light.surface', 'theme.light.textPrimary', 'theme.light.border'],
  keyboard: [{ keys: 'Tab', does: 'Nothing: these probes are static text and take no focus.' }],
  screenReader:
    'Each probe is a paragraph. A screen reader announces the text; insufficient contrast is a ' +
    'visual defect, which is why this is checked by axe rather than by listening.',
});

const meta: Meta = {
  title: 'Gates/Contrast',
  parameters: {
    docs: {
      description: {
        component:
          'The a11y contrast rule, proved able to reject. The exemplars are chosen from the ' +
          'generated palette at build time, so the gate survives a palette change.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * A deliberately bad colour that is **not a token**.
 *
 * The machinery proof, palette-independent by construction: whatever the design tokens become,
 * this grey is still about 3 : 1 on white and axe must still reject it. If this story ever stops
 * producing a violation, the contrast rule has been switched off.
 */
export const MachineryCanReject: Story = {
  parameters: expectsViolations(
    ['color-contrast'],
    'The machinery proof. A gate that has never been watched failing is not known to work, and ' +
      'this colour is not a token, so no palette change can quietly make it pass.',
  ),
  render: () => html`
    <mjx-contrast-probe
      color="#949494"
      background="#ffffff"
      label="This paragraph is 16px body text at roughly 3 : 1. The a11y gate must reject it."
    ></mjx-contrast-probe>
  `,
};

/** The closest miss in the generated palette, used as body text. Must be rejected. */
export const TokenRejectedAsBodyText: Story = {
  parameters: expectsViolations(
    ['color-contrast'],
    "DESIGN_TOKENS.md §2.2's rule applied to the project's own palette: this token is legal for " +
      'fills, borders, indicators and icons, and illegal as body text.',
  ),
  render: () => html`
    <mjx-contrast-probe
      data-token=${exemplars.rejected.path}
      color=${exemplars.rejected.value}
      background=${background}
      label=${`${exemplars.rejected.path} is ${exemplars.rejected.value} — ${formatRatio(exemplars.rejected.ratio)} on ${background}. Legal for fills, borders, indicators and icons. Illegal as body text, which is what this paragraph is.${exemplars.rejectedIsSynthetic ? ' (No token in the palette fails, so this is a synthetic stand-in.)' : ''}`}
    ></mjx-contrast-probe>
  `,
};

/** The narrowest pass in the generated palette, used as body text. Must be accepted. */
export const TokenAcceptedAsBodyText: Story = {
  render: () => html`
    <mjx-contrast-probe
      data-token=${exemplars.accepted.path}
      color=${exemplars.accepted.value}
      background=${background}
      label=${`${exemplars.accepted.path} is ${exemplars.accepted.value} — ${formatRatio(exemplars.accepted.ratio)} on ${background}. This clears the body-text minimum, and this paragraph is body text.`}
    ></mjx-contrast-probe>
  `,
};

/**
 * The whole palette, measured.
 *
 * Documentation rather than a gate, and its labels are deliberately *not* painted in the colour
 * they describe: a ledger that demonstrated every failure would fail its own a11y sweep, and the
 * two stories above are where the demonstration belongs.
 */
export const PaletteLedger: Story = {
  render: () => {
    const rows = measurePalette(background);
    return html`
      <div
        style=${`background:${background};padding:24px;border-radius:16px;font:400 15px/1.5 system-ui,sans-serif;color:${tokens.theme.light.textPrimary}`}
      >
        <p style="margin:0 0 16px">
          Measured against ${background}. Body text needs 4.5 : 1; fills, borders and indicators
          need 3 : 1.
        </p>
        ${rows.map(
          (row) => html`
            <div style="display:flex;gap:16px;align-items:center;margin-block:8px">
              <span
                style=${`inline-size:48px;block-size:28px;border-radius:8px;background:${row.value};border:1px solid ${tokens.theme.light.border}`}
              ></span>
              <span style="min-inline-size:16ch">${row.path}</span>
              <span style="min-inline-size:9ch">${formatRatio(row.ratio)}</span>
              <span>${row.legalAsBodyText ? 'legal as body text' : 'fills, borders and icons only'}</span>
            </div>
          `,
        )}
      </div>
    `;
  },
};
