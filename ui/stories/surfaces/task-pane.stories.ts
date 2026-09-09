import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  dockEdgeReport,
  note,
  surfaceKeyboard,
  surfaceScreenReader,
  surfaceStatesFor,
  surfaceTokenDependencies,
} from './specimens.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';

/**
 * `<mjx-task-pane>` — docked, resizable, persistent, and **the one surface a person cannot
 * dismiss.**
 *
 * **Open *Docked Beside The Document* first, with a keyboard.** Tab to the splitter and press the
 * arrow keys: the pane resizes, and the percentage it announces is the one on screen. Then press
 * Escape — nothing happens, and that is the assertion. Every other surface in this catalogue closes
 * on Escape; a gate that swept all of them together would have quietly inverted this one.
 *
 * **Then open *Against A Coloured Page*.** The line between the pane and the document is an
 * indicator over a surface this platform does not control — a person may set any page colour — so
 * it is chosen per page rather than fixed. Change the page colour and watch the line change with
 * it; the caption is the worst case over four thousand pages.
 */

const conventions = storyConventions({
  statesMatrix: surfaceStatesFor(['taskPane']),
  tokenDependencies: surfaceTokenDependencies,
  keyboard: surfaceKeyboard,
  screenReader: surfaceScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Surfaces/Task Pane',
  parameters: {
    docs: {
      description: {
        component:
          'A docked, resizable, persistent pane beside the document. No scrim, no trap, no ' +
          'Escape — and an edge measured against a page colour it does not own.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const fieldStyle =
  'padding:var(--mjx-density-step);border-radius:var(--radius-control);' +
  'border:1px solid var(--theme-border);background:var(--theme-surface);' +
  'color:var(--theme-text-primary)';

/** The workspace: a document, and a pane beside it. */
function workspace(page: string, dock: 'inlineStart' | 'inlineEnd') {
  return html`
    <div
      id="workspace"
      style="display:flex;block-size:26rem;gap:0;padding:var(--mjx-density-gutter);
             background:var(--theme-background)"
    >
      <div
        id="page"
        class=${typeRoleClass('body')}
        style="flex:1 1 auto;min-inline-size:0;overflow:auto;padding:var(--mjx-density-gutter);
               border-radius:var(--radius-card);background:${page};color:var(--theme-text-primary)"
      >
        <p style="margin:0">
          <a href="#page-body">The document</a>, which the pane is docked beside rather than drawn
          over. It stays editable, stays in the tab order, and is never made inert.
        </p>
      </div>
      <mjx-task-pane
        id="pane"
        label="Format Shape"
        open
        dock=${dock}
        document-color=${page}
      >
        <label class="mjx-type-control" style="display:grid;gap:var(--mjx-density-step)">
          Width
          <input id="pane-width" style=${fieldStyle} value="4.5 cm" />
        </label>
        <label class="mjx-type-control" style="display:grid;gap:var(--mjx-density-step)">
          Height
          <input id="pane-height" style=${fieldStyle} value="3.2 cm" />
        </label>
      </mjx-task-pane>
    </div>
  `;
}

/** The ordinary case. */
export const DockedBesideTheDocument: Story = {
  name: 'Docked Beside The Document',
  render: () =>
    html`
      ${note(
        'Tab to the splitter and use the arrow keys, Home and End. Then press Escape at the pane ' +
          'and watch nothing happen — a task pane is closed by the application and by nothing a ' +
          'person can do to it.',
      )}
      ${workspace('#ffffff', 'inlineEnd')}
    `,
};

/** A page colour the platform did not choose. */
export const AgainstAColouredPage: Story = {
  name: 'Against A Coloured Page',
  render: () =>
    html`
      ${note(
        'The page is a mid-grey, which is the worst kind of colour for a fixed edge: it is far ' +
          'from nothing. The pane measures it and picks whichever end of the palette reads on it.',
      )}
      ${workspace('#808080', 'inlineEnd')} ${dockEdgeReport()}
    `,
};

/** Docked to the other edge, which is what a right-to-left workspace does by itself. */
export const DockedAtTheStart: Story = {
  name: 'Docked At The Start Of The Line',
  render: () =>
    html`
      ${note(
        'The same pane on the other edge. The arrow key that grows it swaps with it — and swaps ' +
          'again under right-to-left, which is why the key map is resolved through physicalSide ' +
          'rather than written twice.',
      )}
      ${workspace('#ffffff', 'inlineStart')}
    `,
};

/** Compact density, at the hit-target floor. */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () =>
    html`
      <div data-density="compact">
        ${note(
          'A properties inspector is the surface compact density exists for. The splitter is the ' +
            'thing to measure: it still clears 24 CSS pixels.',
        )}
        ${workspace('#ffffff', 'inlineEnd')}
      </div>
    `,
};
