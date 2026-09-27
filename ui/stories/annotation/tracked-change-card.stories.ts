import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { trackedChangeKindNames, trackedChangeKinds } from '../../src/annotation/annotation-model.ts';
import {
  annotationKeyboard,
  annotationScreenReader,
  annotationTokenDependencies,
  caption,
  cardStates,
  note,
  stage,
} from './specimens.ts';

/**
 * `<mjx-tracked-change-card>` — an insertion, a deletion, a formatting change or a move.
 *
 * **Open *The Four Kinds* and turn the theme to dark.** Each kind carries an icon, a verb and a
 * treatment of its excerpt, and the **colour on the card is the author's, not the kind's**. That is
 * deliberate: Word can tell an insertion from a deletion by colour alone in the body text because
 * the underline and the strike-through are there too, and a card has neither by default. Four cards
 * distinguished by hue would be four identical cards to the reader the author palette exists for.
 *
 * `document.*.tracked-change-insert` and `-delete` are the tokens the *canvas* paints revisions
 * with. They are two colours for four kinds and are tagged fill-only, and they are not what a card
 * is banded with.
 */

const conventions = storyConventions({
  statesMatrix: cardStates,
  tokenDependencies: annotationTokenDependencies,
  keyboard: annotationKeyboard,
  screenReader: annotationScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Annotation/Tracked Change Card',
  parameters: {
    docs: {
      description: {
        component:
          'One tracked revision in the margin: the kind, the words it touched, the author’s ' +
          'colour and an accept/reject pair.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const excerpts: Readonly<Record<string, string>> = {
  insertion: ', which had not yet been built,',
  deletion: 'the Analytical Engine',
  formatting: 'Notes on the Engine',
  move: 'the second paragraph',
};

const descriptions: Readonly<Record<string, string>> = {
  insertion: 'Added a qualifying clause after the subject.',
  deletion: 'Removed a repeated clause.',
  formatting: 'Made the heading a Heading 2.',
  move: 'Moved a paragraph above the figure.',
};

export const TheFourKinds: Story = {
  name: 'The Four Kinds',
  render: () =>
    stage(
      note(
        'Four revisions, four authors, four colours. Accept and Reject emit a verdict and change ' +
          'nothing in a document: acting on a revision against a real file is loop 2, and a card ' +
          'that pretended otherwise would be a card whose Accept button silently did nothing.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;max-inline-size:64rem">
          ${trackedChangeKindNames.map(
            (kind, index) => html`
              <mjx-tracked-change-card
                style="inline-size:18rem"
                change-id=${`k${String(index)}`}
                kind=${kind}
                author=${['Ada Lovelace', 'Charles Babbage', 'Grace Hopper', 'Alan Turing'][index] ?? ''}
                time="11:0${String(index)}"
                author-slot=${String(index)}
                anchor-top=${String(200 + index * 60)}
                excerpt=${excerpts[kind] ?? ''}
                >${descriptions[kind] ?? ''}</mjx-tracked-change-card
              >
            `,
          )}
        </div>
      `,
      caption(
        trackedChangeKindNames.map(
          (kind) =>
            `${kind} — ${trackedChangeKinds[kind].element}, icon “${trackedChangeKinds[kind].icon}”, ` +
            `announced as “${trackedChangeKinds[kind].verb}”.`,
        ),
      ),
    ),
};

export const SelectedAndResting: Story = {
  name: 'Selected And Resting',
  render: () =>
    stage(
      note(
        'The selected card is the one a reader is on, and in a pane it is the one that holds its ' +
          'anchor exactly while the rest of the column yields around it.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;max-inline-size:48rem">
          <mjx-tracked-change-card
            style="inline-size:20rem"
            change-id="sel-1"
            kind="deletion"
            author="Grace Hopper"
            time="11:04"
            author-slot="1"
            anchor-top="240"
            excerpt="the Analytical Engine"
            selected
            >Selected.</mjx-tracked-change-card
          >
          <mjx-tracked-change-card
            style="inline-size:20rem"
            change-id="sel-2"
            kind="insertion"
            author="Margaret Hamilton"
            time="11:06"
            author-slot="7"
            anchor-top="300"
            excerpt="a qualifying clause"
            >Resting.</mjx-tracked-change-card
          >
        </div>
      `,
    ),
};
