import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  annotationKeyboard,
  annotationScreenReader,
  annotationTokenDependencies,
  caption,
  cardStates,
  fillThread,
  note,
  stage,
} from './specimens.ts';

/**
 * `<mjx-comment-thread>` — a comment with its replies, **announced as a conversation.**
 *
 * **Open *Collapsed And Expanded* with a screen reader on.** The thread is an `article`, each reply
 * is an `article` of its own inside it, and the whole column is a `feed`. The alternative — a list
 * of list items — would have announced *"list, 9 items"* on a document with two hundred comments,
 * because only nine of them exist in the DOM at once, and would have made a reply a paragraph
 * rather than something a named person wrote at a named time.
 *
 * Collapsed shows the **latest** reply and says how many are hidden. Not the first: what a reviewer
 * needs from a fold is where the conversation got to.
 */

const conventions = storyConventions({
  statesMatrix: cardStates,
  tokenDependencies: annotationTokenDependencies,
  keyboard: annotationKeyboard,
  screenReader: annotationScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Annotation/Comment Thread',
  parameters: {
    docs: {
      description: {
        component:
          'A threaded comment: the root remark, its replies, a reply composer and the mention ' +
          'affordances the document’s own author list allows.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const replies = [
  { id: 'r1', author: 'Charles Babbage', time: '10:22', text: 'The 1843 one — Menabrea’s.' },
  { id: 'r2', author: 'Grace Hopper', time: '10:31', text: 'Caption fixed in the next revision.' },
  { id: 'r3', author: 'Alan Turing', time: '10:45', text: 'Then this paragraph can go.' },
];

const roster = ['Ada Lovelace', 'Charles Babbage', 'Grace Hopper', 'Alan Turing'];

export const CollapsedAndExpanded: Story = {
  name: 'Collapsed And Expanded',
  render: () => {
    fillThread('thread-collapsed', replies, roster);
    fillThread('thread-expanded', replies, roster);
    return stage(
      note(
        'The same thread, folded and open. Press “Show 2 earlier replies” and the fold announces ' +
          'the change politely rather than silently growing the card. Tab into the composer, use a ' +
          'mention chip, and press Reply: the chips offer the people who have already written in ' +
          'this document and nobody else — there is no directory, no presence and no avatar ' +
          'service in this platform, by design.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;max-inline-size:52rem">
          <mjx-comment-thread
            id="thread-collapsed"
            style="inline-size:22rem"
            comment-id="t1"
            author="Ada Lovelace"
            time="10:15"
            author-slot="0"
            anchor-top="120"
            >Is this the 1843 figure or the 1842 one?</mjx-comment-thread
          >
          <mjx-comment-thread
            id="thread-expanded"
            style="inline-size:22rem"
            comment-id="t2"
            author="Ada Lovelace"
            time="10:15"
            author-slot="0"
            anchor-top="120"
            expanded
            >Is this the 1843 figure or the 1842 one?</mjx-comment-thread
          >
        </div>
      `,
      caption([
        'Collapsed shows the LATEST reply — where the conversation got to, not where it started.',
        'Each reply is an article with its own author and time, inside the conversation article.',
      ]),
    );
  },
};

export const AResolvedConversation: Story = {
  name: 'A Resolved Conversation',
  render: () => {
    fillThread('thread-resolved', replies.slice(0, 1), roster);
    return stage(
      note(
        'Resolved, and still readable. The card changes fill and badge; nothing is dimmed. A ' +
          'resolved comment is exactly the case the catalogue’s contrast rule is about — only a ' +
          'disabled control may be dimmed, because only a disabled control is one nobody is ' +
          'expected to read.',
      ),
      html`
        <mjx-comment-thread
          id="thread-resolved"
          style="inline-size:22rem"
          comment-id="t3"
          author="Barbara Liskov"
          time="10:29"
          author-slot="5"
          anchor-top="240"
          resolved
          expanded
          >Kept for the record.</mjx-comment-thread
        >
      `,
    );
  },
};
