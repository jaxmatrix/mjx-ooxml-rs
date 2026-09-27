import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { authorColourSlots } from '../../src/annotation/author-colour.ts';
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
 * `<mjx-comment-card>` — one remark in the margin, in **either** of Word's two comment models.
 *
 * **Open *Both Comment Models* first.** The two cards are not one card with a flag: a legacy
 * `w:comment` has no `commentsExtended` entry, so it has no parent, no children and no `done` flag,
 * and there is nowhere for a reply or a resolution to be stored. So the legacy card **has no reply
 * button and no resolve button at all** — not disabled ones, because a disabled control says *not
 * now* and the truth here is *not ever, in this file format*.
 *
 * The difference is visible (a squarer, dashed card) *and* audible (a note, versus a conversation).
 * One without the other would be a distinction only some readers get.
 */

const conventions = storyConventions({
  statesMatrix: cardStates,
  tokenDependencies: annotationTokenDependencies,
  keyboard: annotationKeyboard,
  screenReader: annotationScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Annotation/Comment Card',
  parameters: {
    docs: {
      description: {
        component:
          'A comment in the margin: author, time, body, resolved state and the actions its own ' +
          'model can actually honour.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

export const BothCommentModels: Story = {
  name: 'Both Comment Models',
  render: () =>
    stage(
      note(
        'The same remark, in the two models Word actually carries. The threaded card offers ' +
          'Reply, Resolve and Delete; the legacy card offers Delete alone, because its part of ' +
          'the package has nowhere to put the other two. Press Resolve on the threaded one and ' +
          'watch the card quieten without dimming — a resolved comment is still meant to be read.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;max-inline-size:48rem">
          <mjx-comment-card
            style="inline-size:20rem"
            comment-id="threaded-1"
            author="Ada Lovelace"
            initials="AL"
            time="10:15"
            model="threaded"
            author-slot="0"
            anchor-top="120"
            >Is this the 1843 figure or the 1842 one? The caption says otherwise.</mjx-comment-card
          >
          <mjx-comment-card
            style="inline-size:20rem"
            comment-id="legacy-1"
            author="Charles Babbage"
            initials="CB"
            time="10:18"
            model="legacy"
            author-slot="2"
            anchor-top="132"
            >A legacy note. There is no reply and no resolved flag in this model.</mjx-comment-card
          >
        </div>
      `,
      caption([
        'threaded — w:commentsExtended: a parent, children and a done flag.',
        'legacy — w:comment alone: one remark, no thread, no resolution.',
        'The band down the leading edge is the author’s colour; the name beside it carries the ' +
          'same fact for a reader who cannot use the colour.',
      ]),
    ),
};

export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () =>
    stage(
      note(
        'Every state an auditor has to be able to see, side by side. Resolved is a fill and a ' +
          'badge and never an opacity: only a disabled control is exempt from the contrast rule, ' +
          'and a resolved comment is not disabled.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;max-inline-size:64rem">
          <mjx-comment-card
            style="inline-size:18rem"
            comment-id="s1"
            author="Grace Hopper"
            time="10:20"
            model="threaded"
            author-slot="1"
            >Resting.</mjx-comment-card
          >
          <mjx-comment-card
            style="inline-size:18rem"
            comment-id="s2"
            author="Alan Turing"
            time="10:24"
            model="threaded"
            author-slot="3"
            selected
            >Selected. In a pane this card holds its anchor exactly.</mjx-comment-card
          >
          <mjx-comment-card
            style="inline-size:18rem"
            comment-id="s3"
            author="Barbara Liskov"
            time="10:29"
            model="threaded"
            author-slot="5"
            resolved
            >Resolved, and still legible.</mjx-comment-card
          >
          <mjx-comment-card
            style="inline-size:18rem"
            comment-id="s4"
            author="Edsger Dijkstra"
            time="10:33"
            model="legacy"
            author-slot="6"
            >Legacy, resting.</mjx-comment-card
          >
        </div>
      `,
    ),
};

export const TheAuthorRing: Story = {
  name: 'The Author Ring',
  render: () =>
    stage(
      note(
        'The eight author colours, one card each. They were searched for rather than chosen: the ' +
          'smallest CIEDE2000 distance between any two of them, under normal vision and all three ' +
          'common colour-vision deficiencies at once, is what the search maximised. Eight hues ' +
          'evenly spaced around the wheel — the obvious answer — score better than these to a ' +
          'trichromat and merge to a single colour under deuteranopia.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;max-inline-size:64rem">
          ${authorColourSlots.map(
            (slot, index) => html`
              <mjx-comment-card
                style="inline-size:15rem"
                comment-id=${`ring-${String(index)}`}
                author=${slot.name}
                time="—"
                model="threaded"
                author-slot=${String(index)}
                >Slot ${String(index)}: ${slot.light} in light, ${slot.dark} in dark.</mjx-comment-card
              >
            `,
          )}
        </div>
      `,
      caption([
        'Switch the theme: every slot changes token and keeps its family.',
        'The colour is a band and a dot, never text — only six colours in this palette reach ' +
          'body-text contrast on the light surface, and two of them are the same green.',
      ]),
    ),
};
