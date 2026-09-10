import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { tokens } from '../../tokens/tokens.ts';
import {
  manyAnnotations,
  tightlyClusteredAnchors,
  wellSpacedAnchors,
} from '../../dev/annotation-fixtures.ts';
import {
  annotationKeyboard,
  annotationScreenReader,
  annotationTokenDependencies,
  caption,
  fillAnnotations,
  marginColumn,
  note,
  paneStates,
  readout,
  report,
  stage,
} from './specimens.ts';

/**
 * `<mjx-review-pane>` — the margin column, and **the packing problem this child exists for.**
 *
 * **Open *Anchors A Few Lines Apart* first, and click the cards one at a time.** Seven annotations
 * whose anchors span about four lines of text want about seven hundred pixels of column between
 * them; nothing can put every card beside its own anchor. What the packer does is place the crowd so
 * that the total distance from their anchors is the smallest any no-overlap arrangement can achieve
 * — which means pulling cards **up** as well as pushing them down — and give the *selected* card its
 * preferred position exactly while the others yield around it. The readout says what it cost, and
 * what a simple downward stack would have cost instead.
 *
 * **Then open *Well Spaced Anchors*.** That is the fixture a person would have written, and on it
 * the packer and the stack produce **identical** output. That is the trap, and it is why the first
 * fixture has to be tight.
 *
 * The connector contract is the other readout: every annotation, with the point on the card where
 * R11's in-canvas line should land. **The line itself is canvas and is not this component's.**
 */

const conventions = storyConventions({
  statesMatrix: paneStates,
  tokenDependencies: annotationTokenDependencies,
  keyboard: annotationKeyboard,
  screenReader: annotationScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Annotation/Review Pane',
  parameters: {
    docs: {
      description: {
        component:
          'The margin column: one-dimensional packing with preferred positions, a virtualised ' +
          'feed, a connector contract, and a sheet where a phone has no margin.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

export const AnchorsAFewLinesApart: Story = {
  name: 'Anchors A Few Lines Apart',
  render: () => {
    fillAnnotations('pane-tight', tightlyClusteredAnchors);
    report('pane-tight', 'tight-readout', 'packing');
    return stage(
      note(
        'Seven annotations inside about four lines of text. Click a card, or focus the column and ' +
          'press Arrow Down: the selected card takes its anchor exactly and the others re-settle ' +
          'around it. The readout compares the packing’s total cost with what a simple downward ' +
          'stack would have cost on the same input.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;align-items:flex-start">
          ${marginColumn(
            html`<mjx-review-pane id="pane-tight" label="Comments" side="inlineStart"></mjx-review-pane>`,
          )}
          <div style="flex:1 1 22rem;min-inline-size:20rem">${readout('tight-readout')}</div>
        </div>
      `,
      caption([
        'A card pulled UP is what a stack can never do, and it is most of the difference.',
        'The selected card’s displacement is zero unless the column’s own top forbids it — with ' +
          'seven cards above it, the last anchor cannot be reached, and the packer says so by ' +
          'moving it rather than by overlapping.',
      ]),
    );
  },
};

export const WellSpacedAnchors: Story = {
  name: 'Well Spaced Anchors',
  render: () => {
    fillAnnotations('pane-spaced', wellSpacedAnchors);
    report('pane-spaced', 'spaced-readout', 'packing');
    return stage(
      note(
        'The fixture a hand would naturally have written: six annotations, well apart. Here the ' +
          'packer and a simple stack are identical — every card sits exactly at its anchor and the ' +
          'two costs in the readout are the same number. This story exists to make that visible, ' +
          'because a gate built on this fixture would have passed a stack.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;align-items:flex-start">
          ${marginColumn(
            html`<mjx-review-pane id="pane-spaced" label="Comments"></mjx-review-pane>`,
          )}
          <div style="flex:1 1 22rem;min-inline-size:20rem">${readout('spaced-readout')}</div>
        </div>
      `,
    );
  },
};

export const TheConnectorContract: Story = {
  name: 'The Connector Contract',
  render: () => {
    fillAnnotations('pane-connector', tightlyClusteredAnchors);
    report('pane-connector', 'connector-readout', 'connector');
    return stage(
      note(
        'What R11 receives. One entry per annotation — built or not, because the canvas draws a ' +
          'line to a card that has scrolled out of view as readily as to one on screen — saying ' +
          'where the anchor is, where the card’s leading edge ended up, which point on it the line ' +
          'should meet, which side of the column the document is on, and which author colour to ' +
          'draw it in. A canvas that had to measure the chrome for that would be reading a layout ' +
          'it does not own.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;align-items:flex-start">
          ${marginColumn(
            html`<mjx-review-pane id="pane-connector" label="Review" side="inlineEnd"></mjx-review-pane>`,
          )}
          <div style="flex:1 1 22rem;min-inline-size:20rem">${readout('connector-readout')}</div>
        </div>
      `,
    );
  },
};

export const ThreeHundredAnnotations: Story = {
  name: 'Three Hundred Annotations',
  render: () => {
    fillAnnotations('pane-many', manyAnnotations(300));
    report('pane-many', 'many-readout', 'packing');
    return stage(
      note(
        'Three hundred annotations, every fifth one within a line of the one before it. Scroll ' +
          'the column: about a dozen cards exist in the DOM at any moment, and the feed still ' +
          'announces each of them as “n of 300”. The scrollbar is as long as every card rather ' +
          'than as long as the built ones, which is what stops it lying the further you scroll.',
      ),
      html`
        <div style="display:flex;gap:var(--mjx-density-gutter);flex-wrap:wrap;align-items:flex-start">
          ${marginColumn(
            html`<mjx-review-pane id="pane-many" label="Comments"></mjx-review-pane>`,
          )}
          <div style="flex:1 1 22rem;min-inline-size:20rem">${readout('many-readout')}</div>
        </div>
      `,
    );
  },
};

export const InATaskPane: Story = {
  name: 'In A Task Pane',
  render: () => {
    fillAnnotations('pane-docked', tightlyClusteredAnchors);
    return stage(
      note(
        'The review pane IS U09’s task pane: docked, persistent and not dismissible. It composes ' +
          'rather than inherits — a component that had subclassed the pane would have inherited a ' +
          'splitter, a dock and an edge indicator it then had to work around — so the pane brings ' +
          'the chrome and this brings the layout. Drag the splitter and the packing re-runs at the ' +
          'new width.',
      ),
      html`
        <div
          style="display:flex;block-size:30rem;border:1px solid var(--theme-border);
                 border-radius:var(--radius-panel);overflow:hidden"
        >
          <div style="flex:1 1 auto;background:var(--document-page);min-inline-size:0"></div>
          <mjx-task-pane
            label="Comments"
            open
            dock="inlineEnd"
            document-color=${tokens.document.light.page}
          >
            <mjx-review-pane
              id="pane-docked"
              label="Comments"
              side="inlineStart"
              style="block-size:100%"
            ></mjx-review-pane>
          </mjx-task-pane>
        </div>
      `,
    );
  },
};

export const NothingToReview: Story = {
  name: 'Nothing To Review',
  render: () => {
    fillAnnotations('pane-empty', []);
    return stage(
      note('A document with no comments and no tracked changes. An empty column, and it says so.'),
      html`
        ${marginColumn(html`<mjx-review-pane id="pane-empty" label="Comments"></mjx-review-pane>`)}
      `,
    );
  },
};

export const OnAPhone: Story = {
  name: 'On A Phone',
  render: () => {
    fillAnnotations('pane-sheet', tightlyClusteredAnchors);
    return stage(
      note(
        'Choose the phone preset in the toolbar, or drag the container narrow. A margin column has ' +
          'no room on a phone, so the pane becomes a sheet listing the annotations in flow — and ' +
          'packing stops, because there is no margin for a card to sit beside. The switch is a ' +
          'container query on the component’s own width, never the window’s.',
      ),
      html`
        <div style="block-size:32rem;display:flex;flex-direction:column;justify-content:flex-end">
          <mjx-review-pane id="pane-sheet" label="Comments"></mjx-review-pane>
        </div>
      `,
      caption([
        'Below 560px of container the presentation property reads “sheet”.',
        'In the sheet the cards are in flow, so every one is built: a virtualised flow layout ' +
          'would be a list whose scroll height was written by an arithmetic no longer describing it.',
      ]),
    );
  },
};
