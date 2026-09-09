import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { definePlateGallery } from '../../src/plates/gallery.ts';
import { supportedManifestVersion } from '../../src/plates/manifest.ts';

definePlateGallery();

/**
 * The plate loader, against R10's manifest shape.
 *
 * R10 (MJXOFF-165) has landed and `cargo run -p mjx-render-oracle -- gallery <dir>` writes real
 * plates. The manifest the gallery below loads is **a committed fixture**, not one of them, and
 * that is a deliberate limitation rather than an oversight: the real generator builds `mjx-paint`,
 * which links the platform's graphics stack, and putting a Vulkan toolchain between a TypeScript
 * contributor and a green catalogue would be the wrong trade. The fixture matches
 * `crates/mjx-render-oracle/src/plate.rs` field for field and
 * `tests/plate-manifest.test.ts` parses the *real* generator's documented shape.
 *
 * Point `src` at a directory produced by the oracle and this element loads it unchanged.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'loaded', description: 'A manifest with one plate, and its provenance notice.' },
    {
      name: 'missing',
      description: 'No manifest at the given directory: the reason is shown, not swallowed.',
    },
  ],
  tokenDependencies: [
    'theme.light.surface',
    'theme.light.border',
    'theme.light.textPrimary',
    'theme.light.textSecondary',
    'theme.light.secondaryAccent',
    'theme.light.secondarySurface',
    'radius.card',
    'radius.chip',
    'radius.control',
    'text.sm',
    'text.xs',
  ],
  keyboard: [
    {
      keys: 'Tab',
      does:
        'Nothing today: the gallery is figures and text. When a plate becomes clickable it will ' +
        'need a focusable control, which is a change to this row as much as to the element.',
    },
  ],
  screenReader:
    "Each plate is a figure. Its image announces the plate's description as alternative text, and " +
    'the caption and the fact list follow it, so the provenance is read out rather than seen only.',
});

const meta: Meta = {
  title: 'Harness/Plate gallery',
  parameters: {
    docs: {
      description: {
        component:
          `Loads \`plates.json\` (schema version ${String(supportedManifestVersion)}) from a ` +
          'directory and shows each plate with the provenance that limits what it may claim. An ' +
          'unrecognised version is refused rather than guessed at.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** The committed fixture. */
export const Loaded: Story = {
  render: () => html`<mjx-plate-gallery src="/plates"></mjx-plate-gallery>`,
};

/**
 * A directory with no manifest in it.
 *
 * A gallery that failed silently would show an empty page, which reads as "the renderer produced
 * nothing" — a much worse message than the true one.
 */
export const ManifestMissing: Story = {
  render: () => html`<mjx-plate-gallery src="/plates-that-do-not-exist"></mjx-plate-gallery>`,
};
