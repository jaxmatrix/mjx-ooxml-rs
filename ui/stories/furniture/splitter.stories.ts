import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  furnitureTokenDependencies,
  note,
  splitterKeyboard,
  splitterScreenReader,
  splitterStates,
  stage,
} from './specimens.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';

/**
 * `<mjx-splitter>` — a divider between two regions, and a *lift* rather than a third implementation.
 *
 * **Open *A Navigator Beside A Document* and use it with a keyboard first.** Tab to the divider and
 * press the arrow keys, `Home` and `End`; then press `Enter` and watch the navigator collapse, and
 * press it again and watch it come back **to where you had it** rather than to a default nobody
 * chose. A resize that only works by drag excludes keyboard users from a core layout control, which
 * is the whole reason ARIA has a window-splitter pattern at all.
 *
 * The arithmetic under it is `src/foundations/splitter.ts`, shared with `<mjx-task-pane>`'s dock
 * splitter — the same clamp, the same drag, and the same key map whose growing arrow depends on
 * which side the region is on and mirrors under right-to-left.
 */

const conventions = storyConventions({
  statesMatrix: splitterStates,
  tokenDependencies: furnitureTokenDependencies,
  keyboard: splitterKeyboard,
  screenReader: splitterScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Furniture/Splitter',
  parameters: {
    docs: {
      description: {
        component:
          'A draggable divider with keyboard resize, a minimum per side, a collapse that restores ' +
          'rather than resets, and a position that survives a remount.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/**
 * ⚠ Two things here are the browser gate's findings rather than taste.
 *
 * `box-sizing: border-box`, because under the default `content-box` a padded region set to
 * `inline-size: 0` is still its own padding wide. And then the padding itself has to go, because
 * **border-box cannot shrink a box below its own padding either** — CSS floors the used width
 * there. Both left *collapsed* showing a two-dozen-pixel stripe of surface with every announced
 * number already correct, which is why `--mjx-split-collapsed` exists: a region needs a signal it
 * can zero its own padding from. Only a measurement of the region could see any of it.
 */
const panelStyle =
  'box-sizing:border-box;min-inline-size:0;overflow:hidden;' +
  'padding:calc(var(--mjx-density-gutter) * (1 - var(--mjx-split-collapsed, 0)));' +
  'border-radius:var(--radius-card);background:var(--theme-surface);' +
  'color:var(--theme-text-primary)';

/** Two regions with a splitter between them, laid out from the splitter's own custom property. */
function workspace(id: string, extra: Record<string, string> = {}) {
  return html`
    <div
      id=${`${id}-boundary`}
      style="display:flex;block-size:18rem;gap:0;align-items:stretch"
    >
      <div
        id=${`${id}-navigator`}
        class=${typeRoleClass('body')}
        style=${`flex:0 0 auto;inline-size:calc(100% * var(--mjx-split-fraction, 0.3));${panelStyle}`}
      >
        The navigator. Its width is the splitter’s fraction, written as one custom property.
      </div>
      <mjx-splitter
        id=${id}
        label="the navigator"
        controls=${`${id}-navigator`}
        fraction=${extra['fraction'] ?? '0.3'}
        min-start=${extra['min-start'] ?? '0.15'}
        min-end=${extra['min-end'] ?? '0.3'}
        storage-key=${extra['storage-key'] ?? ''}
      ></mjx-splitter>
      <div
        id=${`${id}-document`}
        class=${typeRoleClass('body')}
        style=${`flex:1 1 auto;${panelStyle}`}
      >
        The document. It takes whatever is left, which is why only one of the two regions needs a
        width at all.
      </div>
    </div>
  `;
}

/** The ordinary case. */
export const ANavigatorBesideADocument: Story = {
  name: 'A Navigator Beside A Document',
  render: () =>
    stage(
      note(
        'Tab to the divider and use the arrows, Home and End. Then press Enter twice: the ' +
          'navigator collapses and comes back to where you left it. A double-click does the same ' +
          'thing with a pointer.',
      ),
      workspace('navigator'),
    ),
};

/** The minimums, which are per side and not one number. */
export const AMinimumPerSide: Story = {
  name: 'A Minimum Per Side',
  render: () =>
    stage(
      note(
        'This one keeps at least a fifth of the boundary for the navigator and at least half of ' +
          'it for the document, so the two limits are different numbers — which is why they are ' +
          'declared separately rather than as one symmetrical bound. Press End: the divider stops ' +
          'at fifty per cent, not at the edge.',
      ),
      workspace('bounded', { fraction: '0.3', 'min-start': '0.2', 'min-end': '0.5' }),
    ),
};

/** The position, remembered. */
export const RememberedAcrossARemount: Story = {
  name: 'Remembered Across A Remount',
  render: () => {
    const remount = (event: Event) => {
      const root = (event.target as HTMLElement).getRootNode() as Document | ShadowRoot;
      const splitter = root.querySelector('#remembered');
      const parent = splitter?.parentElement;
      if (splitter === null || splitter === undefined || parent === null || parent === undefined) {
        return;
      }
      const next = splitter.cloneNode(true) as Element;
      // The clone deliberately carries no `fraction`: what comes back has to come back from
      // storage, or the specimen would prove nothing.
      next.removeAttribute('fraction');
      splitter.replaceWith(next);
    };
    return stage(
      note(
        'Move the divider, then press “Remount the splitter”. The element is replaced by a fresh ' +
          'one carrying no fraction at all, and it comes back where you put it — from ' +
          'localStorage, keyed by storage-key. Every read and write is wrapped, because storage ' +
          '*throws* rather than returning nothing in a private window, and a shell that would not ' +
          'open because of a preference is worse than a splitter at its default.',
      ),
      html`<mjx-button label="Remount the splitter" size="small" @click=${remount}></mjx-button>`,
      workspace('remembered', { 'storage-key': 'catalogue/navigator' }),
    );
  },
};

/** Compact density, at the hit-target floor. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`
      <div data-density="compact">
        ${stage(
          note(
            'The divider is a one-pixel line inside a hit target that still clears 24 CSS pixels. ' +
              'A one-pixel divider a person has to hit exactly is a divider only a mouse in good ' +
              'hands can move.',
          ),
          workspace('compact'),
        )}
      </div>
    `,
};
