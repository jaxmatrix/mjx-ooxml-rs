import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';

/**
 * The document surface, in both schemes.
 *
 * `DESIGN_TOKENS.md` §2.3 states the property this story exists to make visible:
 *
 * > **The page stays true white (`#ffffff`) in both themes.** A document is white paper, and a
 * > warm-tinted or dark page changes what the author sees relative to what they will print or
 * > send. What changes with the theme is the *canvas backdrop* behind the page, the page shadow,
 * > and the in-canvas UI.
 *
 * `tests/browser/theme.spec.ts` asserts it on computed values in both schemes, against the
 * generated tokens rather than against a written-down hex — so the assertion survives a palette
 * change and would still catch a theme layer that started tinting the page.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'light', description: 'Warm off-white backdrop, true-white page, ink-tinted shadow.' },
    { name: 'dark', description: 'Dark backdrop, the same true-white page, a deeper shadow.' },
    {
      name: 'system',
      description:
        'No data-theme attribute: prefers-color-scheme decides. Reachable from the Theme toolbar.',
    },
  ],
  tokenDependencies: [
    'document.light.backdrop',
    'document.light.page',
    'document.light.pageBorder',
    'document.light.pageShadow',
    'document.dark.backdrop',
    'document.dark.page',
    'document.dark.pageBorder',
    'document.dark.pageShadow',
  ],
  keyboard: [
    { keys: 'Tab', does: 'Nothing: the surface is not interactive until the canvas lands in R11.' },
  ],
  screenReader:
    'Nothing is announced. The document surface is a drawing; the content on it will come from ' +
    'the renderer and carries its own accessibility tree.',
});

const meta: Meta = {
  title: 'Foundations/Document surface',
  parameters: {
    docs: {
      description: {
        component:
          'Switch the Theme toolbar between Light and Dark. The backdrop changes; the page does ' +
          'not. That is the whole point of the two-palette split.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The page on its backdrop, in whichever scheme the toolbar has chosen. */
export const PageOnBackdrop: Story = {
  render: () => html`<mjx-document-surface-probe></mjx-document-surface-probe>`,
};

/**
 * Both schemes at once, side by side.
 *
 * Each half **names** its scheme rather than wearing a `data-theme` attribute, because
 * `tokens.css`'s scheme layer is `:root`-scoped: a `data-theme` on a `<div>` changes nothing, and
 * a story that pretended otherwise would show two identical panels and look like a passing test.
 * The scheme-specific properties (`--document-light-*`, `--document-dark-*`) are emitted
 * unconditionally, so naming one is the supported way to show both at once.
 *
 * This is also the visual-regression plate: colour and geometry with no text in it, which is what
 * makes a cross-machine pixel comparison meaningful rather than a font-rasterisation flake.
 */
export const BothSchemes: Story = {
  render: () => html`
    <div style="display:flex;gap:16px;flex-wrap:wrap">
      <mjx-document-surface-probe
        scheme="light"
        style="flex:1 1 280px;min-inline-size:0"
      ></mjx-document-surface-probe>
      <mjx-document-surface-probe
        scheme="dark"
        style="flex:1 1 280px;min-inline-size:0"
      ></mjx-document-surface-probe>
    </div>
  `,
};
