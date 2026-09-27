import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MjxVirtualList, VirtualItem } from '../../src/navigators/index.ts';
import {
  caption,
  largeList,
  largeNavigatorCount,
  navigatorTokenDependencies,
  note,
  paneStyle,
  stage,
  virtualListKeyboard,
  virtualListScreenReader,
  virtualListStates,
} from './specimens.ts';

/**
 * `<mjx-virtual-list>` — the windowed list the other three navigators are built on.
 *
 * **Open *Five Thousand Rows, A Screenful Of Elements* first, and then open the inspector.** The
 * thing to look at is not the list: it is the DOM. Five thousand items are declared and a few dozen
 * elements exist, and the number of elements does not change however far you scroll.
 *
 * Then open *A Thousand Rows Arrive Above You*, which is the one a screenshot cannot show at all.
 */

const conventions = storyConventions({
  statesMatrix: virtualListStates,
  tokenDependencies: navigatorTokenDependencies,
  keyboard: virtualListKeyboard,
  screenReader: virtualListScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Navigators/Virtual List',
  parameters: {
    docs: {
      description: {
        component:
          'A windowed listbox over a collection too large to render: variable row heights, ' +
          'scroll-to-index, and a scroll position that survives items changing above the viewport.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Fill a list once it has upgraded. Items are a property, never markup. */
function fill(id: string, items: readonly VirtualItem[], value?: string): void {
  queueMicrotask(() => {
    const list = document.getElementById(id);
    if (!(list instanceof HTMLElement)) return;
    customElements.upgrade(list);
    const typed = list as MjxVirtualList;
    typed.items = items;
    if (value !== undefined) typed.value = value;
  });
}

/**
 * ⚠ **A twenty-item story passes under any implementation, so this one has five thousand.**
 *
 * The assertion the ticket asks for is on *node count* and not on appearance, and the number it is
 * compared against comes from the window the foundations computed rather than from the renderer.
 */
export const FiveThousandRowsAScreenfulOfElements: Story = {
  name: 'Five Thousand Rows, A Screenful Of Elements',
  render: () => {
    fill('big-list', largeList(), 'row-2');
    return stage(
      note(
        `This list holds ${String(largeNavigatorCount)} rows and the DOM holds a screenful. Scroll it: ` +
          'the count does not grow. Every row that is not built is a height in an extent table ' +
          'rather than an element, and the scrollbar is as long as all five thousand because the ' +
          'two spacers stand in for the ones that were not built.',
      ),
      html`<mjx-virtual-list id="big-list" label="Comments" style=${paneStyle}></mjx-virtual-list>`,
      caption([
        'One tab stop, whatever the row count. Focus stays on the container and the row is named by aria-activedescendant.',
        'Row 4 is unavailable and explained: reachable by arrow key, announced, never chosen.',
        'Every twenty-third row wraps onto two lines, so the rows are genuinely of different heights.',
      ]),
    );
  },
};

/**
 * **The trap the ticket names, and it never appears in a still.**
 *
 * Scroll until *Comment 2,500* is somewhere you can see, note where it is on screen, then press the
 * button. A thousand rows arrive **above** it. The row does not move.
 *
 * The caption prints two numbers: the offset the list actually took, and the offset it would have
 * taken had it simply kept the old one. If the second ever equals the first, this story is proving
 * nothing — which is why it is printed rather than assumed.
 */
export const AThousandRowsArriveAboveYou: Story = {
  name: 'A Thousand Rows Arrive Above You',
  render: () => {
    fill('stable-list', largeList(), 'row-2');
    const insert = (): void => {
      const list = document.getElementById('stable-list');
      if (!(list instanceof HTMLElement)) return;
      const typed = list as MjxVirtualList;
      const fresh: VirtualItem[] = Array.from({ length: 1000 }, (_unused, index) => ({
        id: `fresh-${String(Date.now())}-${String(index)}`,
        label: `Inserted ${String(index + 1)}`,
        detail: 'New',
      }));
      typed.items = [...fresh, ...typed.items];
      const readout = document.getElementById('stable-readout');
      if (readout !== null) {
        readout.textContent =
          `offset taken ${typed.offset.toFixed(0)} · offset the naive answer would have kept ` +
          `${typed.naiveOffset.toFixed(0)} · rows measured ${String(typed.measuredRowCount)}`;
      }
    };
    return stage(
      note(
        'Scroll down, remember where a row is on the screen, then press the button. A thousand rows ' +
          'arrive above the visible range. What you were looking at stays exactly where it was, ' +
          'because the list holds an ANCHOR — this row, this far into it — and derives the offset ' +
          'from it, rather than holding the offset.',
      ),
      html`
        <button type="button" @click=${insert} style="align-self:flex-start">
          Insert a thousand rows above
        </button>
        <mjx-virtual-list id="stable-list" label="Comments" style=${paneStyle}></mjx-virtual-list>
        <output id="stable-readout" class="mjx-type-dense" style="color:var(--theme-text-secondary)"
          >press the button</output
        >
      `,
      caption([
        'The two numbers are the whole assertion: the offset taken, and the offset the naive answer — keep the scroll position — would have kept.',
        'If they were ever equal this story would be proving nothing, which is why both are printed.',
      ]),
    );
  },
};

/**
 * ⚠ **The hit-target floor holds in compact density.** The floor is a promise about pixels on a
 * screen, so it is measured rather than declared: the browser gate reads the height of a real row
 * here and requires it to clear the accessible minimum.
 */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    fill('compact-list', largeList(200), 'row-2');
    return html`<div data-density="compact" style="display:flex;flex-direction:column;min-block-size:0">
      ${stage(
        note('Compact density. The rows are tighter and the target floor still holds.'),
        html`<mjx-virtual-list
          id="compact-list"
          label="Comments"
          style=${paneStyle}
        ></mjx-virtual-list>`,
      )}
    </div>`;
  },
};
