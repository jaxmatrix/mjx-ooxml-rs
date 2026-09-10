import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  feedbackKeyboard,
  feedbackScreenReader,
  feedbackTokenDependencies,
  note,
  stage,
} from './specimens.ts';

/**
 * `<mjx-empty-state>` — what a region says when there is nothing in it.
 *
 * **Press the button.** That is the whole audit, and it is not a joke: an empty state renders
 * perfectly with no data by definition, so every visual check passes on one whose action does
 * nothing. *The Action Does The Thing* wires it to the real list, and the browser gate presses it
 * and requires the empty state to be gone and the list to have items in it.
 *
 * The heading is the **one** place in this platform Young Serif is allowed —
 * `foundations/typography.ts` §4 says display-only, empty states and onboarding, never chrome and
 * never document content. The illustration is silent, the heading is real text, and the box
 * announces itself politely when it appears, because an empty state is what a region *becomes* and
 * a person who cannot see it is otherwise told nothing at all.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'nothing yet', description: 'The first state: nothing has been created. The action creates the first one.' },
    { name: 'nothing found', description: 'A filter matched nothing. The action clears the filter — a different verb for a different emptiness.' },
    { name: 'no action', description: 'Some emptiness has nothing to offer. The button is absent rather than present and inert.' },
    { name: 'with a secondary way out', description: 'A slotted link beside the primary action, because that one belongs to the application.' },
    { name: 'filled', description: 'What the action produced. The state a gate has to reach for the button to have been proved.' },
  ],
  tokenDependencies: feedbackTokenDependencies,
  keyboard: feedbackKeyboard,
  screenReader: feedbackScreenReader,
});

const meta: Meta = {
  title: 'Feedback/Empty State',
  parameters: {
    docs: {
      description: {
        component:
          'A polite announcement with a silent illustration, a serif heading and an action that ' +
          'actually does something — which is the only part of it a screenshot cannot check.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const comments = [
  'Marta: this paragraph repeats the one above it.',
  'Ade: can we cite the 2024 figures here?',
  'Marta: agreed — I will pull them in.',
];

/**
 * The action, wired to the real thing.
 *
 * Not a spy and not a counter: the button fills the list, and the gate asserts the list filled. A
 * story whose action only recorded that it had been pressed would prove the event fires and nothing
 * about whether pressing it achieves anything.
 */
function fillTheList(event: Event): void {
  const scope = event.currentTarget as HTMLElement | null;
  if (scope === null) return;
  const list = scope.querySelector('#comment-list');
  const empty = scope.querySelector('#comments-empty');
  if (list === null || empty === null) return;
  list.replaceChildren(
    ...comments.map((text) => {
      const item = document.createElement('li');
      item.className = 'mjx-type-body';
      item.textContent = text;
      return item;
    }),
  );
  (empty as HTMLElement).hidden = true;
}

export const TheActionDoesTheThing: Story = {
  name: 'The Action Does The Thing',
  render: () =>
    html`<div @mjx-empty-state-action=${fillTheList}>
      ${stage(
        note(
          'Press “Add a comment”. The list below fills and the empty state goes. That is the ' +
            'assertion this component exists for: a decorative button is the commonest defect in ' +
            'an empty state, because the state is usually built while the thing it offers is still ' +
            'a stub.',
        ),
        html`
          <div
            style="border:1px solid var(--theme-border-subtle);border-radius:var(--radius-card);
                   background:var(--theme-surface);min-block-size:16rem;display:grid;align-content:center"
          >
            <mjx-empty-state
              id="comments-empty"
              heading="No comments yet"
              description="Comments you and your reviewers add will appear here, next to the text they are about."
              icon="comment"
              action-label="Add a comment"
              action-command="comment.add"
            ></mjx-empty-state>
            <ul
              id="comment-list"
              style="margin:0;padding-inline-start:calc(var(--spacing) * 6);display:grid;
                     gap:var(--mjx-density-step);color:var(--theme-text-primary)"
            ></ul>
          </div>
        `,
      )}
    </div>`,
};

/** A different emptiness, with a different verb — and a secondary way out beside it. */
export const NothingFound: Story = {
  name: 'Nothing Found',
  render: () =>
    stage(
      note(
        'Nothing *matched* is not the same as nothing *exists*, and the action says so: the way ' +
          'out of an over-narrow filter is to widen it, never to create something. The secondary ' +
          'route is slotted, because that one belongs to the application rather than to this ' +
          'component.',
      ),
      html`
        <mjx-empty-state
          id="search-empty"
          heading="Nothing matched “tumbleweed”"
          description="No slide in this deck contains that word. Try a shorter phrase, or search every open file."
          icon="search"
          action-label="Clear the search"
          action-command="search.clear"
        >
          <a
            slot="secondary"
            href="#search-everywhere"
            class="mjx-type-control mjx-hit-target"
            style="display:inline-flex;align-items:center;padding-inline:var(--mjx-density-gutter);
                   color:var(--theme-accent-pressed)"
            >Search every open file</a
          >
        </mjx-empty-state>
      `,
    ),
};

/** Some emptiness has nothing to offer, and says so by having no button at all. */
export const NothingToOffer: Story = {
  name: 'Nothing To Offer',
  render: () =>
    stage(
      note(
        'No action-label, so there is no button — rather than a button that is present and inert. ' +
          'An empty state whose only affordance is disabled has told a person twice that they ' +
          'cannot do anything.',
      ),
      html`
        <mjx-empty-state
          id="readonly-empty"
          heading="This deck has no notes"
          description="Speaker notes were removed when this file was published as read-only."
        ></mjx-empty-state>
      `,
    ),
};
