import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MiniCommand } from '../../src/feedback/feedback-model.ts';
import {
  documentBody,
  feedbackKeyboard,
  feedbackScreenReader,
  feedbackTokenDependencies,
  note,
  selectionRun,
  stage,
} from './specimens.ts';

/**
 * `<mjx-mini-toolbar>` — the commands that appear beside a selection.
 *
 * **Open *A Selection With No Room* second.** The first three stories show a toolbar finding a
 * clear side above, below and beside the selection. The fourth is a selection that fills its whole
 * room, where there is no clear side at all — and the toolbar says so, on the host, as
 * `data-covering="true"`. That story exists so the promise *“it never covers the selection”* has a
 * failure branch a gate can watch happen; a promise nothing can falsify is a promise nothing is
 * testing.
 *
 * Then **Tab into one and use the arrows.** It is a toolbar, so it holds one tab stop and the
 * arrows do the moving — and Escape gives the keyboard back only if the keyboard was ever inside.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'clear above', description: 'The usual case: room above the selection, so the toolbar goes there.' },
    { name: 'flipped below', description: 'A selection at the top of its room. The same arithmetic, the other side.' },
    { name: 'beside', description: 'A selection too tall for either block side. The inline fallback.' },
    { name: 'covering', description: 'A selection that fills its room. No clear side exists; the host says so with data-covering.' },
    { name: 'toggle on', description: 'A command whose aria-pressed is true, drawn with the on paint from the shared state table.' },
    { name: 'unavailable', description: 'aria-disabled with an explanation: still focusable, still announced, refuses activation.' },
  ],
  tokenDependencies: feedbackTokenDependencies,
  keyboard: feedbackKeyboard,
  screenReader: feedbackScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Feedback/Mini Toolbar',
  parameters: {
    docs: {
      description: {
        component:
          'A toolbar that appears beside a selection and never on top of it, placed by the same ' +
          'flip-shift-constrain arithmetic every other floating box in this catalogue uses — asked ' +
          'once per side until one of them clears the selection.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const formattingCommands: readonly MiniCommand[] = [
  { command: 'bold', label: 'Bold', icon: 'text-bold', kind: 'toggle', pressed: true },
  { command: 'italic', label: 'Italic', icon: 'text-italic', kind: 'toggle' },
  { command: 'underline', label: 'Underline', icon: 'text-underline', kind: 'toggle' },
  { command: 'align-left', label: 'Align left', icon: 'text-align-left', separatorBefore: true },
  { command: 'align-center', label: 'Centre', icon: 'text-align-center' },
  { command: 'comment', label: 'New comment', icon: 'comment', separatorBefore: true },
  {
    command: 'delete',
    label: 'Delete',
    icon: 'delete',
    unavailable: true,
    explanation: 'The selection is inside a locked content control.',
  },
];

/** A live readout of the last command, so “the commands work” is visible as well as asserted. */
function readout(): ReturnType<typeof html> {
  return html`
    <output
      id="last-command"
      class="mjx-type-dense"
      style="color:var(--theme-text-secondary)"
      aria-live="polite"
      >none yet</output
    >
  `;
}

function record(event: Event): void {
  const detail = (event as CustomEvent<{ command: string; pressed?: boolean }>).detail;
  const target = (event.currentTarget as HTMLElement | null)?.querySelector('#last-command');
  if (target === null || target === undefined) return;
  target.textContent =
    detail.pressed === undefined ? detail.command : `${detail.command}: ${String(detail.pressed)}`;
}

/** Room above, which is where a mini toolbar belongs. */
export const AboveTheSelection: Story = {
  name: 'Above The Selection',
  render: () =>
    html`<div @mjx-mini-command=${record}>
      ${stage(
        note(
          'The selection is in the third paragraph and the toolbar is above it, one gap away. ' +
            'Press a command and the readout below changes — a toolbar whose buttons fire nothing ' +
            'is the commonest thing to ship here.',
        ),
        documentBody(2),
        html`<div
          id="editor"
          tabindex="0"
          class="mjx-type-body"
          style="margin:0;color:var(--theme-text-primary);border-radius:var(--radius-control);
                 padding:var(--mjx-density-step)"
        >
          A run of text with ${selectionRun('selection-clear', 'this phrase selected')} inside it.
          This surface is focusable, so Tabbing from it into the toolbar and pressing Escape has
          somewhere to give the keyboard back to.
        </div>`,
        documentBody(2),
        readout(),
        html`<mjx-mini-toolbar
          id="toolbar-clear"
          label="Formatting"
          for="selection-clear"
          open
          .commands=${formattingCommands}
        ></mjx-mini-toolbar>`,
      )}
    </div>`,
};

/** No room above, so the same arithmetic takes the other side. */
export const FlippedBelow: Story = {
  name: 'Flipped Below The Selection',
  render: () =>
    html`<div @mjx-mini-command=${record}>
      ${stage(
        html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
          ${selectionRun('selection-top', 'A selection at the very top of its room')} has nowhere to
          put a toolbar above it.
        </p>`,
        note(
          'So the toolbar is below it — the same flip a menu makes when its button is near the ' +
            'bottom of the screen, from the same function.',
        ),
        documentBody(3),
        readout(),
        html`<mjx-mini-toolbar
          id="toolbar-flipped"
          label="Formatting"
          for="selection-top"
          open
          .commands=${formattingCommands}
        ></mjx-mini-toolbar>`,
      )}
    </div>`,
};

/**
 * The failure branch, on the page.
 *
 * A selection that fills its room leaves no clear side, and the honest answer is a *reported*
 * failure rather than a silently-least-bad placement. This story is what lets a gate watch that
 * report happen.
 */
export const NoRoomAtAll: Story = {
  name: 'A Selection With No Room',
  render: () =>
    html`<div @mjx-mini-command=${record}>
      <div
        style="block-size:14rem;overflow:hidden;container-type:inline-size;
               padding:var(--mjx-density-step);background:var(--theme-background)"
      >
        <mark
          id="selection-everywhere"
          class="mjx-type-body"
          style="display:block;block-size:100%;background:var(--document-light-selection-fill);
                 color:var(--theme-text-primary);border-radius:var(--radius-chip);
                 padding:var(--mjx-density-step)"
        >
          This selection fills every pixel of its room, so no side of it is clear. The toolbar
          reports that on its host as data-covering="true" rather than pretending it found somewhere
          to go.
        </mark>
        <mjx-mini-toolbar
          id="toolbar-covering"
          label="Formatting"
          for="selection-everywhere"
          open
          .commands=${formattingCommands}
        ></mjx-mini-toolbar>
      </div>
    </div>`,
};

/** Right to left, where the inline axis mirrors and the block axis does not. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () =>
    html`<div dir="rtl" @mjx-mini-command=${record}>
      ${stage(
        note(
          'The toolbar is still above the selection — the block axis does not mirror — and the ' +
            'arrow keys have swapped, because the inline axis does.',
        ),
        html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
          نص عربي مع ${selectionRun('selection-rtl', 'هذه العبارة محددة')} بداخله.
        </p>`,
        documentBody(2),
        readout(),
        html`<mjx-mini-toolbar
          id="toolbar-rtl"
          label="تنسيق"
          for="selection-rtl"
          open
          .commands=${formattingCommands}
        ></mjx-mini-toolbar>`,
      )}
    </div>`,
};

/** The hit-target floor holds in compact, which is the density a mini toolbar is usually in. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`<div data-density="compact" @mjx-mini-command=${record}>
      ${stage(
        note(
          'Compact reduces the space between the commands and reduces the target only as far as ' +
            'the accessible floor. Measure one: it is never smaller than 24 CSS pixels.',
        ),
        html`<p class="mjx-type-body" style="margin:0;color:var(--theme-text-primary)">
          A run of text with ${selectionRun('selection-compact', 'this phrase selected')} inside it.
        </p>`,
        documentBody(2),
        readout(),
        html`<mjx-mini-toolbar
          id="toolbar-compact"
          label="Formatting"
          for="selection-compact"
          open
          .commands=${formattingCommands}
        ></mjx-mini-toolbar>`,
      )}
    </div>`,
};
