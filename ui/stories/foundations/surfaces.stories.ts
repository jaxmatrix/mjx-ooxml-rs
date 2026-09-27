import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  radiusSteps,
  surfaceLevelNames,
  surfaceLevels,
  type RadiusStep,
  type SurfaceLevel,
} from '../../src/foundations/surfaces.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';

/**
 * The elevation ladder and the radius scale.
 *
 * MJXOFF-181: *"Surfaces are where the ink-tinted shadow of `DESIGN_TOKENS.md` §4 lives. A shadow
 * that renders as flat black is wrong in a way a snapshot will happily lock in."*
 *
 * So the gate is not a snapshot. `tests/browser/foundations.spec.ts` reads the computed
 * `box-shadow` off the raised rungs, parses the colour out of it, and asserts two things a picture
 * cannot: that it **equals the generated `shadow.lift` token** channel for channel, and that it is
 * **chromatic** — its three channels differ, which a neutral black can never be, however the
 * palette is re-seeded.
 *
 * The radii are checked the same way, against the generated `radius.*` values, because §4 names
 * them as *"the most recognisable part of the source's character"* and a drift back towards
 * Office's 2–4px corners is exactly the change that would go unremarked in review.
 */

const conventions = storyConventions({
  statesMatrix: surfaceLevelNames.map((level) => ({
    name: level,
    description: surfaceLevels[level].use,
  })),
  tokenDependencies: [
    'theme.light.background',
    'theme.light.surface',
    'theme.light.surfaceRaised',
    'theme.light.border',
    'theme.light.borderSubtle',
    'theme.light.accentSurface',
    'theme.light.accentBorder',
    'theme.light.secondarySurface',
    'shadow.lift',
    'radius.chip',
    'radius.control',
    'radius.card',
    'radius.panel',
    'radius.frame',
    'radius.phone',
  ],
  keyboard: [
    {
      keys: 'Tab',
      does: 'Nothing. A surface is a container; whatever it contains keeps its own tab order and its own focus ring.',
    },
  ],
  screenReader:
    'Nothing. `<mjx-surface>` has no implicit role and adds none — a card that announced itself ' +
    'as a region would put a landmark in every panel of the application.',
});

const meta: Meta = {
  title: 'Foundations/Surfaces',
  parameters: {
    docs: {
      description: {
        component:
          'Seven rungs and six radii, all of them var() over generated tokens. Switch the Theme ' +
          'toolbar: the ink tint of the shadow is the thing to watch.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * The ladder, lowest rung first.
 *
 * `base` and `sunken` share a fill and are told apart by their border; `raised` changes the fill;
 * `floating` is the first rung that leaves the plane. That progression — fill, then border, then
 * shadow — is what keeps a chrome from looking like a pile of drop shadows.
 */
export const TheElevationLadder: Story = {
  render: () => html`
    <div
      style="display:grid;grid-template-columns:repeat(auto-fit,minmax(14rem,1fr));gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
    >
      ${surfaceLevelNames.map(
        (level: SurfaceLevel) => html`
          <mjx-surface data-level=${level} level=${level}>
            <p class=${typeRoleClass('label')} style="margin:0 0 var(--mjx-density-step)">${level}</p>
            <p class=${typeRoleClass('dense')} style="margin:0;color:var(--theme-text-secondary)">
              ${surfaceLevels[level].use}
            </p>
          </mjx-surface>
        `,
      )}
    </div>
  `,
};

/**
 * The radius scale, on one rung, so the only thing changing is the corner.
 *
 * Chip 8, control 10, card 16, panel 20, frame 22, phone 36. §4: *keep that softness rather than
 * reverting to the 2–4px corners Office uses.*
 */
export const TheRadiusScale: Story = {
  render: () => html`
    <div
      style="display:flex;flex-wrap:wrap;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2)"
    >
      ${radiusSteps.map(
        (step: RadiusStep) => html`
          <mjx-surface
            data-radius=${step}
            level="floating"
            radius=${step}
            style="inline-size:9rem;block-size:6rem;display:flex;align-items:center;justify-content:center"
          >
            <span class=${typeRoleClass('dense')}>${step}</span>
          </mjx-surface>
        `,
      )}
    </div>
  `,
};

/**
 * The ink tint, isolated.
 *
 * Two identical surfaces, one carrying `--shadow-lift` and one carrying nothing, on the light
 * backdrop where the tint is visible at all. The shadow is `#223b331c` — the ink green at 11%
 * alpha — and against a neutral black at the same alpha the difference is small, obvious once seen,
 * and completely invisible in a description. Which is why the assertion is arithmetic and this
 * story is only the demonstration.
 */
export const TheShadowIsInkTinted: Story = {
  render: () => html`
    <div
      style="display:flex;gap:calc(var(--mjx-density-gutter) * 3);padding:calc(var(--mjx-density-gutter) * 4);background:var(--theme-background)"
    >
      <mjx-surface
        data-shadow="lift"
        level="floating"
        style="inline-size:12rem;block-size:8rem;display:flex;align-items:center;justify-content:center"
      >
        <span class=${typeRoleClass('dense')}>--shadow-lift</span>
      </mjx-surface>
      <mjx-surface
        data-shadow="none"
        level="raised"
        style="inline-size:12rem;block-size:8rem;display:flex;align-items:center;justify-content:center"
      >
        <span class=${typeRoleClass('dense')}>no shadow</span>
      </mjx-surface>
    </div>
  `,
};

/**
 * The accent rung, which is where a focus ring goes wrong.
 *
 * MJXOFF-181 names it: *"including the accent surfaces, where a green ring on a green tint is the
 * predictable failure."* The rung is here so the focus stories and
 * `tests/foundations.test.ts` have something real to measure against, and so an auditor can see
 * the two accent tints beside the neutral ones.
 */
export const TheAccentRungs: Story = {
  render: () => html`
    <div
      style="display:flex;gap:calc(var(--mjx-density-gutter) * 2);padding:calc(var(--mjx-density-gutter) * 2);flex-wrap:wrap"
    >
      ${(['accent', 'secondary', 'raised'] as const).map(
        (level) => html`
          <mjx-surface
            data-level=${level}
            level=${level}
            style="inline-size:14rem;display:flex;flex-direction:column;gap:var(--mjx-density-step)"
          >
            <span class=${typeRoleClass('label')}>${level}</span>
            <span class=${typeRoleClass('dense')} style="color:var(--theme-text-secondary)"
              >${surfaceLevels[level].use}</span
            >
          </mjx-surface>
        `,
      )}
    </div>
  `,
};
