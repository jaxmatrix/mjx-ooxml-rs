import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  documentBody,
  feedbackKeyboard,
  feedbackScreenReader,
  feedbackTokenDependencies,
  note,
  stage,
  timingReport,
} from './specimens.ts';

/**
 * `<mjx-screentip>` — Office's enhanced tip, and the delay that is most of its behaviour.
 *
 * **Hover *A Row Of Commands* and watch the clock rather than the tip.** The first one waits; move
 * straight to the second and it appears with no wait at all, because the warm-up window is still
 * open. Wait a couple of seconds and try again: the wait is back. That is the behaviour that makes
 * a row of icon commands readable, and it is invisible in every screenshot ever taken of a tooltip.
 *
 * **Then Tab to one.** A keyboard arrives on a control because a person put it there, so it waits
 * for nothing. **Then press Escape** — the tip goes and the keyboard does not move, which is the
 * assertion MJXOFF-189 asks for by name.
 *
 * Every delay on this page is a multiple of the platform's single duration token. The report at the
 * bottom is computed, not typed.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'waiting', description: 'A pointer is resting on the trigger and the tip is not there yet. The state a screenshot cannot show.' },
    { name: 'shown', description: 'The delay elapsed. Heading, description and shortcut, placed below the trigger.' },
    { name: 'shown immediately', description: 'Reached by keyboard, or by a pointer inside the warm-up window: the same tip with no wait.' },
    { name: 'flipped', description: 'No room below the trigger, so the tip takes the other side — the same arithmetic a menu uses.' },
    { name: 'plain', description: 'A heading and nothing else. A tip is enhanced by having a description, not by an attribute.' },
    { name: 'dismissed', description: 'Escape, or the pointer leaving. Focus does not move in either case.' },
  ],
  tokenDependencies: feedbackTokenDependencies,
  keyboard: feedbackKeyboard,
  screenReader: feedbackScreenReader,
});

const meta: Meta = {
  title: 'Feedback/Screentip',
  parameters: {
    docs: {
      description: {
        component:
          'A tip that waits for a pointer and not for a keyboard, whose text is the trigger’s ' +
          'accessible description whether or not it is drawn, and which Escape removes without ' +
          'moving the keyboard anywhere.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const triggerStyle =
  'padding-inline:var(--mjx-density-gutter);padding-block:var(--mjx-density-step);' +
  'border-radius:var(--radius-control);border:1px solid var(--theme-border);' +
  'background:var(--theme-surface);color:var(--theme-text-primary);cursor:pointer';

function trigger(id: string, label: string) {
  return html`
    <button slot="trigger" id=${id} class="mjx-type-control mjx-hit-target" style=${triggerStyle}>
      ${label}
    </button>
  `;
}

/** The one a gate hovers: it must not be there before the delay, and must be after it. */
export const WaitsForAPointer: Story = {
  name: 'Waits For A Pointer',
  render: () =>
    stage(
      note(
        'Rest the pointer on the button and count. Nothing happens for the first part of a ' +
          'second, which is the whole point: a tip that appeared instantly would appear while the ' +
          'pointer was merely crossing the toolbar on its way somewhere else.',
      ),
      html`
        <div style="display:flex;gap:calc(var(--mjx-density-gutter) * 2)">
          <mjx-screentip
            id="tip-bold"
            heading="Bold"
            description="Make the selected text bold. Applies to the whole selection, including any table cells inside it."
            shortcut="Ctrl + B"
          >
            ${trigger('tip-bold-trigger', 'Bold')}
          </mjx-screentip>

          <mjx-screentip id="tip-plain" heading="Undo">
            ${trigger('tip-plain-trigger', 'Undo')}
          </mjx-screentip>
        </div>
      `,
      documentBody(2),
      timingReport(),
    ),
};

/** Three in a row, so the warm-up has something to be warm about. */
export const ARowOfCommands: Story = {
  name: 'A Row Of Commands',
  render: () =>
    stage(
      note(
        'Hover the first, wait for its tip, then move straight along the row. The second and ' +
          'third appear immediately — a delay applied afresh to every button would turn this row ' +
          'into a row of things that will not tell you what they are.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-step)">
          <mjx-screentip id="row-one" heading="Save" description="Write the file to disk." shortcut="Ctrl + S">
            ${trigger('row-one-trigger', 'Save')}
          </mjx-screentip>
          <mjx-screentip id="row-two" heading="Open" description="Choose another file." shortcut="Ctrl + O">
            ${trigger('row-two-trigger', 'Open')}
          </mjx-screentip>
          <mjx-screentip id="row-three" heading="Find" description="Search this document." shortcut="Ctrl + F">
            ${trigger('row-three-trigger', 'Find')}
          </mjx-screentip>
        </div>
      `,
      documentBody(3),
    ),
};

/** Forced open, so the drawn tip can be audited at leisure and in both schemes. */
export const TheEnhancedTip: Story = {
  name: 'The Enhanced Tip',
  render: () =>
    stage(
      note(
        'Held open so the box itself can be read: a heading in the label role, a description and ' +
          'a shortcut in the dense role, on the overlay rung. Nothing here is announced twice — ' +
          'the drawn box is aria-hidden and the same words reach a screen reader as the trigger’s ' +
          'accessible description.',
      ),
      html`
        <mjx-screentip
          id="tip-open"
          open
          heading="Format Painter"
          description="Copy formatting from one place and apply it to another. Double-click to keep the brush loaded."
          shortcut="Ctrl + Shift + C"
        >
          ${trigger('tip-open-trigger', 'Format Painter')}
        </mjx-screentip>
      `,
      documentBody(3),
    ),
};

/** At the bottom of its room, so the placement has to take the other side. */
export const AtTheBottomOfItsRoom: Story = {
  name: 'At The Bottom Of Its Room',
  render: () =>
    html`
      <div
        style="block-size:20rem;display:flex;flex-direction:column;justify-content:flex-end;
               padding:calc(var(--mjx-density-gutter) * 2);background:var(--theme-background)"
      >
        ${note(
          'There is no room below the button, so the tip goes above it — and it still does not ' +
            'cover the button, which is a stronger promise than merely flipping.',
        )}
        <mjx-screentip
          id="tip-flipped"
          open
          heading="Line spacing"
          description="Set the space between lines of the selected paragraphs."
        >
          ${trigger('tip-flipped-trigger', 'Line spacing')}
        </mjx-screentip>
      </div>
    `,
};
