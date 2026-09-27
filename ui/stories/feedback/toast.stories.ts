import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MjxToastRegion } from '../../src/feedback/toast.ts';
import type { ToastTone } from '../../src/feedback/feedback-model.ts';
import {
  documentBody,
  feedbackKeyboard,
  feedbackScreenReader,
  feedbackTokenDependencies,
  note,
  stage,
  toneReport,
  toneStates,
} from './specimens.ts';

/**
 * `<mjx-toast-region>` — a queue, and the politeness that is its whole accessibility contract.
 *
 * **Open *A Queue Over Time* and press the buttons quickly.** Three toasts stack, the fourth
 * retires the oldest one that leaves on its own, and each keeps its *own* clock — the second does
 * not restart the first. Then press *Show an error*: it is announced assertively, and it never
 * leaves until somebody dismisses it. Push three more after it and it is still there, because a
 * message a person has not read must not be shunted off the screen by three confirmations.
 *
 * **The four tones are told apart by four things and the colour is the weakest of them.** This
 * palette has one alarm colour and no red; the report at the bottom measures how little the two
 * accents differ in the one dimension a contrast gate can see, which is the argument for the icon,
 * the politeness and the dwell doing the work.
 */

const conventions = storyConventions({
  statesMatrix: toneStates(),
  tokenDependencies: feedbackTokenDependencies,
  keyboard: feedbackKeyboard,
  screenReader: feedbackScreenReader,
});

const meta: Meta = {
  title: 'Feedback/Toast',
  parameters: {
    docs: {
      description: {
        component:
          'A stack of transient messages with a ceiling, a per-entry clock, and two live regions ' +
          'that never change their politeness — because a live region whose aria-live is rewritten ' +
          'is the classic way to ship an announcement nobody hears.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const buttonStyle =
  'padding-inline:var(--mjx-density-gutter);padding-block:var(--mjx-density-step);' +
  'border-radius:var(--radius-control);border:1px solid var(--theme-border);' +
  'background:var(--theme-surface);color:var(--theme-text-primary);cursor:pointer';

function pushTo(regionId: string, tone: ToastTone, message: string, action?: string) {
  return (event: Event): void => {
    const root = (event.currentTarget as HTMLElement | null)?.getRootNode();
    const scope = root instanceof Document || root instanceof ShadowRoot ? root : document;
    const region = scope.querySelector(`#${regionId}`) as MjxToastRegion | null;
    if (region === null) return;
    region.show({
      tone,
      message,
      ...(action === undefined ? {} : { action: { label: action, command: `${tone}.${action}` } }),
    });
  };
}

function clearOf(regionId: string) {
  return (event: Event): void => {
    const root = (event.currentTarget as HTMLElement | null)?.getRootNode();
    const scope = root instanceof Document || root instanceof ShadowRoot ? root : document;
    const region = scope.querySelector(`#${regionId}`) as MjxToastRegion | null;
    region?.clear();
  };
}

function control(id: string, label: string, handler: (event: Event) => void) {
  return html`
    <button id=${id} class="mjx-type-control mjx-hit-target" style=${buttonStyle} @click=${handler}>
      ${label}
    </button>
  `;
}

/**
 * The four tones, held still.
 *
 * `paused` stops the clock so an auditor can look at the stack without it dissolving underneath
 * them — the same attribute an application sets while a pointer is over the stack, which is WCAG
 * 2.2 §2.2.1 for anything that disappears on a timer.
 */
export const TheFourTones: Story = {
  name: 'The Four Tones',
  render: () =>
    stage(
      note(
        'Paused, so the stack stays. Four tones, and the ceiling is three — so the oldest one ' +
          'that leaves on its own has already been retired here, which is why the error is still ' +
          'on screen and the first information message is not.',
      ),
      documentBody(3),
      toneReport(),
      html`
        <mjx-toast-region id="tones" label="Notifications" paused>
          <mjx-toast tone="info" message="Link copied to the clipboard."></mjx-toast>
          <mjx-toast tone="success" message="Saved to OneDrive." action-label="Open" action-command="file.open"></mjx-toast>
          <mjx-toast tone="warning" message="Two fonts in this deck are not installed. They were substituted."></mjx-toast>
          <mjx-toast tone="error" message="Could not reach the server. Your changes are still here."></mjx-toast>
        </mjx-toast-region>
      `,
    ),
};

/**
 * The clock, driven for real.
 *
 * The dwell is overridden on this story's own container so the audit does not take a minute — the
 * arithmetic is swept in Node at a dozen instants, and what a browser is here to prove is only that
 * real time reaches it.
 */
export const AQueueOverTime: Story = {
  name: 'A Queue Over Time',
  render: () =>
    html`
      <div style="--mjx-toast-dwell:900ms;--mjx-toast-dwell-long:1400ms">
        ${stage(
          note(
            'Press two of these a moment apart. Each toast keeps its own deadline, so the first ' +
              'one still leaves first — a stack that shared one timer would drop both together or ' +
              'neither. The error never leaves at all.',
          ),
          html`
            <div style="display:flex;gap:var(--mjx-density-step);flex-wrap:wrap">
              ${control('push-info', 'Show information', pushTo('queue', 'info', 'Link copied to the clipboard.'))}
              ${control('push-success', 'Show success', pushTo('queue', 'success', 'Saved to OneDrive.', 'Open'))}
              ${control('push-warning', 'Show a warning', pushTo('queue', 'warning', 'Two fonts were substituted.'))}
              ${control('push-error', 'Show an error', pushTo('queue', 'error', 'Could not reach the server.'))}
              ${control('clear-toasts', 'Clear', clearOf('queue'))}
            </div>
          `,
          documentBody(4),
          html`<mjx-toast-region id="queue" label="Notifications"></mjx-toast-region>`,
        )}
      </div>
    `,
};

/** Compact, where the stack is narrower and the dismissal still clears the accessible floor. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`<div data-density="compact">
      ${stage(
        note(
          'Compact reduces the padding inside a card and the gap between them. The dismissal is ' +
            'still never smaller than 24 CSS pixels, which is the floor the density system holds.',
        ),
        documentBody(3),
        html`
          <mjx-toast-region id="compact-toasts" label="Notifications" paused>
            <mjx-toast tone="success" message="Saved to OneDrive."></mjx-toast>
            <mjx-toast tone="warning" message="Two fonts were substituted."></mjx-toast>
          </mjx-toast-region>
        `,
      )}
    </div>`,
};
