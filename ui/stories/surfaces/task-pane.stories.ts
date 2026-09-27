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
  worstPageColor,
} from './specimens.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import { tokens } from '../../tokens/tokens.ts';

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

/**
 * The ordinary page: white paper, read out of the document palette rather than written.
 *
 * `document.*.page` is `#ffffff` in **both** schemes on purpose — DESIGN_TOKENS.md §2.3 says an
 * editor has two palettes and the document keeps true white whichever one the chrome is wearing —
 * so one member serves every specimen here. It has to be a resolved colour rather than a `var()`
 * because `document-color` is a value the pane *measures*, not one it paints.
 */
const paperPage = tokens.document.light.page;

/**
 * A document object: something on the page that a person can tab to.
 *
 * Painted entirely in tokens against a token — the platform's own surface, its own border and its
 * own text colour — so that the one unmeasured colour in this story (the page) is only ever next to
 * a *non-text* edge. See the comment in `workspace` for the a11y failure that made this a chip
 * rather than a paragraph.
 */
const pageObjectStyle =
  'display:inline-block;padding:var(--mjx-density-step);border-radius:var(--radius-control);' +
  'border:1px solid var(--theme-border);background:var(--theme-surface);' +
  'color:var(--theme-text-primary)';

const fieldStyle =
  'padding:var(--mjx-density-step);border-radius:var(--radius-control);' +
  'border:1px solid var(--theme-border);background:var(--theme-surface);' +
  'color:var(--theme-text-primary)';

/**
 * The workspace: a document, and a pane beside it.
 *
 * ⚠ **Nothing on the page is painted in a theme text colour, and MJXOFF-279 is why.**
 *
 * The page is a colour the platform does not own, so a token drawn *directly on it* is a pairing
 * nobody has measured. This story used to put a body paragraph and a bare `<a>` in
 * `--theme-text-primary` on a hard-coded mid-grey, at 3.05 : 1 and 2.38 : 1, and the a11y sweep
 * failed on both. The pane's *edge* is allowed to sit against the page, because an edge is non-text
 * and the rule that picks it is measured — that is what `dockEdgeReport` reports. Text is not, and
 * 4.5 : 1 against an arbitrary colour is a promise a two-candidate palette cannot make.
 *
 * So the page carries an **object** rather than a run: a link painted as a platform chip, on the
 * platform's own surface. The claim this specimen exists to make — *the document stays in the tab
 * order and is never made inert* — is still checkable with a keyboard, and every contrast question
 * on the page is a token against a token. It is also closer to what a document is: a page is paper,
 * and what sits on it is content rather than chrome.
 */
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
               border-radius:var(--radius-card);background:${page}"
      >
        <a href="#page-body" style=${pageObjectStyle}>The document</a>
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
      ${workspace(paperPage, 'inlineEnd')}
    `,
};

/** A page colour the platform did not choose. */
export const AgainstAColouredPage: Story = {
  name: 'Against A Coloured Page',
  render: () =>
    html`
      ${note(
        `The page is ${worstPageColor} — not a colour anybody picked, but the one the rule is ` +
          'weakest against over the whole sweep, computed here so the specimen and the caption ' +
          'below it can never drift apart. The pane measures it and picks whichever end of the ' +
          'palette reads on it.',
      )}
      ${workspace(worstPageColor, 'inlineEnd')} ${dockEdgeReport()}
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
      ${workspace(paperPage, 'inlineStart')}
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
        ${workspace(paperPage, 'inlineEnd')}
      </div>
    `,
};
