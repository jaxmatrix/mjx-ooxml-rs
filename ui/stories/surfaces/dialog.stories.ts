import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  documentBody,
  note,
  scrimReport,
  stage,
  surfaceKeyboard,
  surfaceScreenReader,
  surfaceStatesFor,
  surfaceTokenDependencies,
} from './specimens.ts';

/**
 * `<mjx-dialog>` — three of the six surfaces, and the one where the defect is never the trap.
 *
 * **Open *Returned To Its Invoker* first, with a keyboard.** Tab to the button, press it, close the
 * dialog four different ways, and watch the keyboard land back on the button every time. That is
 * the assertion this component is arranged around: a modal that traps focus is easy, and a modal
 * that returns it is where the defect lives.
 *
 * **Then open *The Scrim, Measured*.** The hairline around a modal is not a fixed colour and cannot
 * be: the scrim composites whatever is behind it into a band, and a band in the middle of the
 * luminance range swallows any single edge you might pick. Which token carries it is a different
 * answer in each scheme, and the caption is the measurement rather than a claim.
 *
 * **Then switch the container to *phone*.** The same element is a sheet, pinned to the bottom edge
 * with the same call `<mjx-menu>` and `<mjx-gallery>` make.
 */

const conventions = storyConventions({
  statesMatrix: surfaceStatesFor(['dialog', 'modal', 'sheet']),
  tokenDependencies: surfaceTokenDependencies,
  keyboard: surfaceKeyboard,
  screenReader: surfaceScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Surfaces/Dialog',
  parameters: {
    docs: {
      description: {
        component:
          'A modeless dialog, a modal, and the sheet the modal becomes at a phone’s width. One ' +
          'element and one close path; the scrim, the edge and the grab handle are measured ' +
          'rather than declared.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** A button that opens the dialog beside it. The invoker every gate looks for. */
function opener(target: string, label: string) {
  return html`
    <button
      id="open-${target}"
      class="mjx-type-control mjx-hit-target"
      style="align-self:start;padding-inline:var(--mjx-density-gutter);border-radius:var(--radius-control);
             border:1px solid var(--theme-border);background:var(--theme-surface);
             color:var(--theme-text-primary);cursor:pointer"
      @click=${(event: Event) => {
        const root = (event.target as HTMLElement).getRootNode() as ParentNode;
        const dialog = root.querySelector(`#${target}`);
        if (dialog instanceof HTMLElement && 'show' in dialog) {
          (dialog as HTMLElement & { show(invoker?: HTMLElement): void }).show(
            event.currentTarget as HTMLElement,
          );
        }
      }}
    >
      ${label}
    </button>
  `;
}

/** The footer every dialog specimen wears: two real, focusable buttons. */
function footer(id: string) {
  return html`
    <div slot="footer">
      <button
        id="${id}-cancel"
        class="mjx-type-control mjx-hit-target"
        style="padding-inline:var(--mjx-density-gutter);border-radius:var(--radius-control);
               border:1px solid var(--theme-border);background:var(--theme-surface);
               color:var(--theme-text-primary);cursor:pointer"
      >
        Don’t Save
      </button>
      <button
        id="${id}-save"
        class="mjx-type-control mjx-hit-target"
        style="padding-inline:var(--mjx-density-gutter);border-radius:var(--radius-control);
               border:1px solid var(--theme-accent-border);background:var(--theme-accent-surface);
               color:var(--theme-text-primary);cursor:pointer"
      >
        Save
      </button>
    </div>
  `;
}

/** A modal, open, over a document with links in it. */
export const AModal: Story = {
  name: 'A Modal Over The Document',
  render: () =>
    stage(
      note(
        'The dialog is modal, so everything behind it is inert AND aria-hidden — two mechanisms, ' +
          'because one without the other leaves a screen reader wandering out of the dialog. Try ' +
          'to Tab to a paragraph link: there is nowhere for the keyboard to go.',
      ),
      documentBody(),
      html`
        <mjx-dialog id="modal" label="Save changes" modal open>
          <p style="margin:0">
            Save your changes to <strong>Quarterly Report.docx</strong> before closing?
          </p>
          ${footer('modal')}
        </mjx-dialog>
      `,
    ),
};

/** Every close path, and the invoker that gets the keyboard back. */
export const ReturnedToItsInvoker: Story = {
  name: 'Returned To Its Invoker',
  render: () =>
    stage(
      note(
        'Four ways out — Escape, the close button, the scrim, and the application changing its ' +
          'mind — and one close path behind all four. Open it with the keyboard and close it any ' +
          'way you like: focus comes back to the button you pressed.',
      ),
      opener('returning', 'Save changes…'),
      documentBody(3),
      html`
        <mjx-dialog id="returning" label="Save changes" modal>
          <p style="margin:0">Close this however you like and watch where the keyboard lands.</p>
          ${footer('returning')}
        </mjx-dialog>
      `,
    ),
};

/** A modeless dialog: the document behind it is still being edited. */
export const Modeless: Story = {
  name: 'Modeless — The Document Is Still Live',
  render: () =>
    stage(
      note(
        'Find and Replace, not Save changes. There is no scrim, nothing is inert, and Tab walks ' +
          'straight out of the dialog into the document — which is why it does not trap. A ' +
          'modeless dialog stays centred at every width: becoming a sheet would give it a scrim ' +
          'to dismiss by, and a dismissal path its own row does not list.',
      ),
      html`
        <mjx-dialog id="modeless" label="Find and Replace" open>
          <label class="mjx-type-control" style="display:grid;gap:var(--mjx-density-step)">
            Find what
            <input
              id="find-what"
              style="padding:var(--mjx-density-step);border-radius:var(--radius-control);
                     border:1px solid var(--theme-border);background:var(--theme-surface);
                     color:var(--theme-text-primary)"
            />
          </label>
        </mjx-dialog>
      `,
      documentBody(4),
    ),
};

/** The measurement, on the page. */
export const TheScrimMeasured: Story = {
  name: 'The Scrim, Measured',
  render: () =>
    stage(
      note(
        'Nothing below is written down anywhere. Each figure is computed by the same functions ' +
          'the gates sweep, so a palette re-seed moves the caption and the assertion together.',
      ),
      scrimReport(),
    ),
};

/** A popover opened from inside a dialog: the stacking case. */
export const APopoverInsideADialog: Story = {
  name: 'A Popover Inside A Dialog',
  render: () =>
    stage(
      note(
        'Open the popover, then press Escape once: the popover closes and the dialog stays. ' +
          'Press Escape again and the dialog closes too. Closing the dialog while the popover is ' +
          'open takes the popover with it — ownership runs one way.',
      ),
      html`
        <mjx-dialog id="stacked" label="Paragraph" modal open>
          <mjx-popover id="inner" label="Line spacing">
            <button
              slot="anchor"
              id="inner-anchor"
              class="mjx-type-control mjx-hit-target"
              style="padding-inline:var(--mjx-density-gutter);border-radius:var(--radius-control);
                     border:1px solid var(--theme-border);background:var(--theme-surface);
                     color:var(--theme-text-primary);cursor:pointer"
            >
              Line spacing…
            </button>
            <p style="margin:0" class="mjx-type-body">Exactly 1.15 lines.</p>
          </mjx-popover>
          ${footer('stacked')}
        </mjx-dialog>
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
          note(
            'Compact reduces the space between things and never the target below 24 CSS pixels. ' +
              'The close button is the one to measure.',
          ),
          html`
            <mjx-dialog id="compact" label="Paste Special" modal open>
              <p style="margin:0">Compact density, and the close control still clears the floor.</p>
              ${footer('compact')}
            </mjx-dialog>
          `,
        )}
      </div>
    `,
};
