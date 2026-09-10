import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  caption,
  furnitureTokenDependencies,
  note,
  scrollbarContrast,
  scrollbarKeyboard,
  scrollbarScreenReader,
  scrollbarStates,
  stage,
  thumbCaption,
} from './specimens.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import type { MjxScrollbar } from '../../src/furniture/index.ts';

/**
 * `<mjx-scrollbar>` — the identity-value trap in its purest form.
 *
 * **Open *An Estimate Being Corrected* first, and use it with a pointer.** Grab the thumb, keep
 * holding it, and press *Lay out ten pages* with the other hand: the document gets 40 % longer, the
 * thumb gets shorter, and **the point of it under your finger does not move**. That single
 * behaviour is what distinguishes this component from a generic scrollbar, and it is invisible in
 * every story with a fixed content height — which is why the ticket names it as the trap.
 *
 * **Then open *Four Content Ratios*.** A scrollbar over content that fits exercises nothing at all,
 * so the four here run from *exactly fits* to *five hundred pages*, and the caption states the
 * thumb size each produces — computed by the same function the gates assert against.
 */

const conventions = storyConventions({
  statesMatrix: scrollbarStates,
  tokenDependencies: furnitureTokenDependencies,
  keyboard: scrollbarKeyboard,
  screenReader: scrollbarScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Furniture/Scrollbar',
  parameters: {
    docs: {
      description: {
        component:
          'A scrollbar over a viewport it does not own, whose extent is an estimate that gets ' +
          'corrected while a person is holding it. It carries a channel of search hits, comments ' +
          'and tracked changes, which is the reason it cannot be the platform’s own.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** A stand-in for the page canvas, so `aria-controls` names something real. */
function canvas(id: string) {
  return html`
    <div
      id=${id}
      class=${typeRoleClass('body')}
      tabindex="0"
      style="flex:1 1 auto;min-inline-size:0;block-size:100%;overflow:auto;
             padding:var(--mjx-density-gutter);border-radius:var(--radius-card);
             background:var(--theme-surface);color:var(--theme-text-primary)"
    >
      The document canvas. R08 to R13 draw it; this component only says how much of it there is.
    </div>
  `;
}

/** A scrollbar beside a canvas, at a stated height. */
function beside(id: string, attributes: Record<string, string | number>) {
  return html`
    <div style="display:flex;gap:var(--mjx-density-step);block-size:18rem">
      ${canvas(`${id}-canvas`)}
      <mjx-scrollbar
        id=${id}
        label="Document"
        controls=${`${id}-canvas`}
        pages=${String(attributes['pages'] ?? 20)}
        page-height=${String(attributes['page-height'] ?? 1000)}
        viewport=${String(attributes['viewport'] ?? 4000)}
      ></mjx-scrollbar>
    </div>
  `;
}

/**
 * The trap, made operable by hand.
 *
 * The button calls exactly what R13's layout pass calls — `recordMeasuredHeight` on one page at a
 * time — so the specimen and the gate exercise the same path.
 */
export const AnEstimateBeingCorrected: Story = {
  name: 'An Estimate Being Corrected',
  render: () => {
    const correct = (event: Event) => {
      const root = (event.target as HTMLElement).getRootNode() as Document | ShadowRoot;
      const bar = root.querySelector('#corrected') as MjxScrollbar | null;
      if (bar === null) return;
      for (let page = 0; page < 10; page += 1) bar.recordMeasuredHeight(page, 1400);
    };
    const reset = (event: Event) => {
      const root = (event.target as HTMLElement).getRootNode() as Document | ShadowRoot;
      const bar = root.querySelector('#corrected') as MjxScrollbar | null;
      if (bar === null) return;
      // Re-declaring the extent is a new document, which is what rebuilds the model.
      bar.setAttribute('pages', '20');
    };
    return stage(
      note(
        'Take hold of the thumb and keep holding it. Press “Lay out ten pages” — the first ten ' +
          'turn out to be forty per cent taller than the estimate, so the document is longer and ' +
          'the thumb is smaller. The point of the thumb under your finger stays under your ' +
          'finger, because while a drag is in flight the pointer is the state and the offset is ' +
          'derived from it, not the other way round.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter)">
          <mjx-button label="Lay out ten pages" size="small" @click=${correct}></mjx-button>
          <mjx-button label="Back to the estimate" size="small" @click=${reset}></mjx-button>
        </div>
      `,
      beside('corrected', { pages: 20, 'page-height': 1000, viewport: 4000 }),
    );
  },
};

/** Four ratios, because one proves nothing. */
export const FourContentRatios: Story = {
  name: 'Four Content Ratios',
  render: () =>
    stage(
      note(
        'Left to right: content that exactly fits, content that barely overflows, a five-viewport ' +
          'document, and a five-hundred-page one. The first has no thumb at all and is out of the ' +
          'tab order — a full-length thumb that cannot move looks exactly like a working ' +
          'scrollbar, which is the identity-value trap in one sentence.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);block-size:18rem">
          ${['fits', 'barely', 'five', 'huge'].map(
            (which, index) => html`
              <div style="display:flex;gap:var(--mjx-density-step);flex:1 1 0;min-inline-size:0">
                ${canvas(`${which}-canvas`)}
                <mjx-scrollbar
                  id=${which}
                  label=${`Document ${String(index + 1)}`}
                  controls=${`${which}-canvas`}
                  pages=${['1', '9', '5', '500'][index] ?? '1'}
                  page-height=${['800', '100', '800', '800'][index] ?? '800'}
                  viewport="800"
                ></mjx-scrollbar>
              </div>
            `,
          )}
        </div>
      `,
      caption(thumbCaption()),
    ),
};

/** The channel, which is what makes a long document navigable. */
export const TheChannel: Story = {
  name: 'The Channel Of Marks',
  render: () =>
    stage(
      note(
        'Search hits, comment threads and tracked changes, in three lanes across the width of the ' +
          'channel. The lanes are not decoration: this palette has two colour families, so three ' +
          'kinds told apart by hue alone would be two a person can distinguish and a third they ' +
          'cannot — and two marks at the same offset would draw on top of each other.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-step);block-size:18rem">
          ${canvas('marked-canvas')}
          <mjx-scrollbar
            id="marked"
            label="Document"
            controls="marked-canvas"
            pages="20"
            page-height="1000"
            viewport="4000"
          >
            <mjx-scroll-mark kind="search" page="1" within="0.25" label="pipeline"></mjx-scroll-mark>
            <mjx-scroll-mark kind="search" page="6" within="0.5" label="pipeline"></mjx-scroll-mark>
            <mjx-scroll-mark kind="search" page="17" within="0.1" label="pipeline"></mjx-scroll-mark>
            <mjx-scroll-mark kind="comment" page="4" label="Ask legal"></mjx-scroll-mark>
            <mjx-scroll-mark kind="comment" page="12" within="0.8" label="Reword"></mjx-scroll-mark>
            <mjx-scroll-mark kind="change" page="9" within="0.4" label="Inserted"></mjx-scroll-mark>
          </mjx-scrollbar>
        </div>
      `,
      caption(scrollbarContrast()),
    ),
};

/** Horizontal, which is the same arithmetic in the other axis. */
export const Horizontal: Story = {
  name: 'Along The Other Axis',
  render: () =>
    stage(
      note(
        'A wide spreadsheet. Nothing about the model changes — only which of the two measurements ' +
          'the pointer is read from, which is one branch rather than a second component.',
      ),
      html`
        <div style="display:flex;flex-direction:column;gap:var(--mjx-density-step);block-size:14rem">
          ${canvas('wide-canvas')}
          <mjx-scrollbar
            id="wide"
            label="Columns"
            controls="wide-canvas"
            orientation="horizontal"
            pages="40"
            page-height="500"
            viewport="2000"
          ></mjx-scrollbar>
        </div>
      `,
    ),
};

/** Compact density, at the hit-target floor. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`
      <div data-density="compact">
        ${stage(
          note('The bar itself is the hit target, and it still clears 24 CSS pixels.'),
          beside('compact', { pages: 20, 'page-height': 1000, viewport: 4000 }),
        )}
      </div>
    `,
};
