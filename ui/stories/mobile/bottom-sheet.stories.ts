import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  sheetDetentNames,
  sheetDetents,
  sheetDismissBelowFraction,
  type SheetDetent,
} from '../../src/mobile/sheet-detents.ts';
import { sheetBoundaryFraction } from '../../src/surfaces/surface-model.ts';
import { caption, detentCaption, longList, mobileTokenDependencies, note } from './specimens.ts';

/**
 * The bottom sheet — **MJXOFF-188's sheet, completed rather than replaced.**
 *
 * There is still exactly one sheet in this catalogue and it is still `<mjx-dialog>` in its sheet
 * presentation. What MJXOFF-194 added is the three things U09 left it without: detents with a snap,
 * a grab handle that actually drags, and — the one that matters — **a nested scroller that does not
 * fight the sheet.**
 *
 * **Open *A Long List Inside A Sheet* first, at the phone preset**, and flick the list. The sheet
 * does not move. Then drag from the very top of the list, with the list already at its top: now
 * the sheet moves. That is the whole rule, and it is `sheetDragClaim` in
 * `src/mobile/sheet-detents.ts` — four lines, one pure function, no DOM in it.
 *
 * **Then drag the handle down slowly and let go**, and watch it snap to the nearest detent; then
 * flick it down fast from `full` and watch it go past `half` to `peek`, because a release is
 * projected forward before it is rounded.
 */

const conventions = storyConventions({
  statesMatrix: [
    ...sheetDetentNames.map((detent) => ({
      name: detent,
      description: `${(sheetDetents[detent].fraction * 100).toFixed(0)} % of the boundary. ${sheetDetents[detent].use}`,
    })),
    { name: 'dragging', description: 'Under a finger: no transition, translated one-to-one.' },
    { name: 'content scrolled', description: 'The list is scrolled; the sheet ignores the drag.' },
    { name: 'notched device', description: 'The block-end padding carries the safe-area inset.' },
  ],
  tokenDependencies: mobileTokenDependencies,
  keyboard: [
    { keys: 'Escape', does: 'Dismisses the sheet, and returns focus to whatever opened it.' },
    { keys: 'Tab', does: 'Cycles inside the sheet — a modal traps the keyboard.' },
    {
      keys: 'not focusable',
      does: 'The grab handle. It is a pointer affordance with no keyboard equivalent, because the keyboard equivalent of a detent is the sheet’s own scroll, which Tab and the arrow keys already reach.',
    },
  ],
  screenReader:
    'Announces "Paragraph, dialog" and reads the sheet’s content. The grab handle is hidden from ' +
    'the accessibility tree: it is a picture of an affordance, and the affordance itself has no ' +
    'meaning to a reader who cannot drag it.',
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Mobile/Bottom Sheet',
  parameters: {
    docs: {
      description: {
        component:
          'Detents with a snap, a grab handle, a backdrop scrim, safe-area insets — and a nested ' +
          'scroller that does not fight the sheet drag, which is the classic mobile defect and ' +
          'the reason this exists.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

function sheet(detent: SheetDetent, rows: number, id: string) {
  return html`
    <div style="min-block-size:30rem;position:relative;background:var(--theme-background)">
      <mjx-dialog id=${id} label="Paragraph" modal open detent=${detent}>
        ${longList(rows)}
        <button slot="footer" type="button" style="min-block-size:var(--mjx-hit-target)">
          Apply
        </button>
      </mjx-dialog>
    </div>
  `;
}

/** The one that matters. */
export const ALongListInsideASheet: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'The sheet is at its half detent and the list inside it is forty rows long. Flick the ' +
          'list: it scrolls, and the sheet stays exactly where it is. Scroll the list back to the ' +
          'top and drag down again: now the sheet moves, because there is nothing left for the ' +
          'list to scroll and the gesture can only mean one thing.',
      )}
      ${sheet('half', 40, 'nested')}
      ${caption([
        'Rule 1: the handle and the header are always the sheet’s.',
        'Rule 2: a scrolled scroller keeps the drag, in either direction. This is the one whose ' +
          'absence dismisses a sheet when a person flicks a list.',
        'Rule 3: at the top of the scroller, a downward drag hands off to the sheet.',
        'Rule 4: everything else is the content’s, including an upward drag at the top — which is ' +
          'an over-scroll, and overscroll-behavior: contain is what stops it reaching the page.',
      ])}
    `,
};

/** Three heights, and the third is not a new number. */
export const TheThreeDetents: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'Drag the handle and let go. A slow release snaps to the nearest detent; a fast flick is ' +
          'projected forward first, so a flick downward from full reaches peek rather than ' +
          'stopping at half. Below the dismissal threshold it goes away.',
      )}
      ${sheet('full', 60, 'detents')}
      ${caption([
        ...detentCaption(),
        `full is an ALIAS of MJXOFF-188's sheetBoundaryFraction (${sheetBoundaryFraction.toFixed(2)}), ` +
          'not a fourth number — U09 decided how much of a boundary a sheet may cover and this ' +
          'child does not get a vote on it.',
        `Below ${(sheetDismissBelowFraction * 100).toFixed(0)} % of the boundary a release ` +
          'dismisses rather than snapping back, and the threshold is deliberately well under peek: ' +
          'losing a sheet loses what was in it, and snapping back costs one more drag.',
      ])}
    `,
};

/** The shallow one, so the document behind stays legible. */
export const AtPeek: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'The shallowest detent. Most of the document is still visible behind the scrim, which is ' +
          'what a peek is for: a person is choosing something *about* what they can see.',
      )}
      ${sheet('peek', 20, 'peek')}
      ${caption(detentCaption())}
    `,
};

/** The same sheet, on a device with a notch. */
export const OnANotchedDevice: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'The four safe-area insets are registered custom properties fed from the environment. A ' +
          'notched device writes them through env(); this story writes the same numbers directly ' +
          'onto the root, which is the same value arriving down the same channel rather than a ' +
          'stub — and the only way to exercise it at all, because a headless browser reports zero ' +
          'on every side and no flag changes that.',
      )}
      <div
        style="--mjx-safe-area-inset-block-end:34px;--mjx-safe-area-inset-block-start:59px;
               --mjx-safe-area-inset-inline-start:0px;--mjx-safe-area-inset-inline-end:0px"
      >
        ${sheet('half', 24, 'notched')}
      </div>
      ${caption([
        'The sheet’s own block-end padding is the inset, so its last row clears the home indicator.',
        'A command bar under the same insets adds it to its gutter rather than replacing it, so ' +
          'the space above the commands does not change when a device has a notch and a device ' +
          'without one does not gain a gap.',
      ])}
    `,
};
