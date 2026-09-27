import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  contextualActions,
  selectionKindNames,
  sharedSelectionCommands,
  type SelectionKind,
} from '../../src/mobile/mobile-model.ts';
import {
  barStates,
  caption,
  documentArea,
  mobileTokenDependencies,
  note,
  phoneStage,
} from './specimens.ts';

/**
 * `<mjx-contextual-action-bar>` — what replaces the command bar the moment something is selected.
 *
 * **Open *The Four Selections* first** and read the four bars against each other. Cut, copy, paste
 * and delete are in all three non-empty ones, because they are declared once and spliced in — a
 * catalogue that spelled *copy* twice would be a catalogue with two copies of it. What changes is
 * everything else.
 *
 * ⚠ **They are always *present*, not always *first*.** The rail is ordered by U04's ladder and by
 * nothing else, so on a text selection `delete` — which is `standard` — sits behind the `primary`
 * formatting commands. Giving the shared four a reserved run at the front would have been the second
 * priority scheme the ticket forbids.
 *
 * **Then open *No Selection Is No Bar*.** There is nothing there, and that is the assertion: a
 * contextual bar with an empty rail would take the block end of a phone away from the command bar
 * it replaced while offering nothing in exchange.
 */

const conventions = storyConventions({
  statesMatrix: [
    ...barStates(),
    { name: 'text selected', description: 'The shared four, then bold, italic, underline.' },
    { name: 'object selected', description: 'The shared four, then arrange and alt text.' },
    { name: 'cells selected', description: 'The shared four, then insert, style and alignment.' },
    { name: 'nothing selected', description: 'No bar at all — the host hides itself.' },
  ],
  tokenDependencies: mobileTokenDependencies,
  keyboard: [
    { keys: 'Tab', does: 'Enters the bar at its one tab stop.' },
    { keys: 'ArrowRight / ArrowLeft', does: 'Moves the stop along the rail.' },
    { keys: 'Home / End', does: 'Jumps to the first or last reachable control.' },
    { keys: 'Enter / Space', does: 'Activates the focused action.' },
    { keys: 'Escape', does: 'Closes the overflow grid.' },
  ],
  screenReader:
    'Announces "Selection actions: 12 words, toolbar" — the selection is in the bar’s own ' +
    'accessible name, so a reader on a portrait phone is told what the verbs act on even though ' +
    'the readout beside them is not drawn at that width.',
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Mobile/Contextual Action Bar',
  parameters: {
    docs: {
      description: {
        component:
          'Format, cut, copy, paste, delete and whatever this particular selection offers. The ' +
          'same rail, overflow and roving tab stop as the command bar, because it is the same ' +
          'implementation with a different source of commands.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const selectionLabels: Readonly<Record<SelectionKind, string>> = {
  none: '',
  text: '12 words',
  object: 'a picture',
  cells: 'B2:D9',
};

function actionBar(kind: SelectionKind) {
  return html`
    <mjx-contextual-action-bar
      id=${`bar-${kind}`}
      selection=${kind}
      selection-label=${selectionLabels[kind]}
    ></mjx-contextual-action-bar>
  `;
}

/** All four, one under the other. */
export const TheFourSelections: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'Four selections, four bars. Every one of them offers cut, copy, paste and delete; the ' +
          'whole rail is then laid out in the ladder’s order rather than in the order it was ' +
          'declared, which is why delete sits behind bold and italic on a text selection.',
      )}
      <div style="display:flex;flex-direction:column;gap:var(--mjx-density-gutter)">
        ${selectionKindNames.map((kind) => actionBar(kind))}
      </div>
      ${caption([
        `The shared four are ${sharedSelectionCommands.map((command) => command.label).join(', ')}.`,
        ...selectionKindNames
          .filter((kind) => kind !== 'none')
          .map(
            (kind) =>
              `${kind}: ${String(contextualActions(kind).length)} actions — ` +
              `${contextualActions(kind)
                .map((command) => command.label)
                .join(', ')}.`,
          ),
        'none: 0 actions, and therefore no bar.',
      ])}
    `,
};

/** The state that is a behaviour rather than an emptiness. */
export const NoSelectionIsNoBar: Story = {
  globals: { containerPreset: 'phone' },
  render: () =>
    html`
      ${note(
        'There is a contextual action bar in the markup below and it is not on screen. With ' +
          'nothing selected the host hides itself, so the block end of the phone belongs to the ' +
          'command bar — which is the thing a person actually needs when they have selected ' +
          'nothing.',
      )}
      ${phoneStage(documentArea('Nothing is selected.'), actionBar('none'))}
      ${caption([
        'The element is present, has no commands, and carries data-empty="true" on its host.',
        'That is not an empty toolbar: an empty toolbar is a row of nothing a person can still ' +
          'tab into.',
      ])}
    `,
};

/** A tablet with a mouse is a real configuration. */
export const OnASmallTablet: Story = {
  globals: { containerPreset: 'tablet' },
  render: () =>
    html`
      ${note(
        'Eight visible slots and a bar that floats clear of the container’s edges. The selection ' +
          'readout is drawn at this width, because there is room for it; on a portrait phone it ' +
          'is announced instead.',
      )}
      ${phoneStage(documentArea('A picture is selected.'), actionBar('object'))}
      ${caption([
        `object: ${contextualActions('object')
          .map((command) => command.label)
          .join(', ')}.`,
      ])}
    `,
};
