import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import type { MjxThumbnailRail } from '../../src/navigators/index.ts';
import type { RailSlide } from '../../src/navigators/navigator-model.ts';
import {
  caption,
  deckWithPlates,
  everyPlate,
  largeDeck,
  largeNavigatorCount,
  navigatorTokenDependencies,
  note,
  paneStyle,
  railKeyboard,
  railScreenReader,
  railStates,
  stage,
} from './specimens.ts';

/**
 * `<mjx-thumbnail-rail>` — PowerPoint's slide sorter.
 *
 * **Open *A Plate That Has Not Arrived* first.** Half the slides are still being rendered, and the
 * rail is fully usable while they are: it scrolls, it selects, and it reorders. That is the whole
 * requirement — R10's plate generator is asynchronous, and a rail that blocked on rendering would be
 * unusable at exactly the moment a deck is opened.
 *
 * `GUESS:` **a section is a heading row and not a `role="group"`.** A group announces how many it
 * owns, and a window holds some of a section's slides and not others — so a group element under
 * virtualisation would either claim rows that are not in the DOM or report a count that changes as
 * the reader scrolls. The section is folded into each slide's accessible name instead, which is true
 * at every scroll position. PowerPoint has no accessible-name convention to be parity with.
 */

const conventions = storyConventions({
  statesMatrix: railStates,
  tokenDependencies: navigatorTokenDependencies,
  keyboard: railKeyboard,
  screenReader: railScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Navigators/Thumbnail Rail',
  parameters: {
    docs: {
      description: {
        component:
          'A slide sorter: numbered thumbnails, sections, hidden slides, multi-select, keyboard ' +
          'and drag reorder, and a placeholder state while a plate is still being rendered.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

/** Fill a rail once it has upgraded. */
function fill(id: string, slides: readonly RailSlide[], selected?: number): void {
  queueMicrotask(() => {
    const rail = document.getElementById(id);
    if (!(rail instanceof HTMLElement)) return;
    customElements.upgrade(rail);
    const typed = rail as MjxThumbnailRail;
    typed.slides = slides;
    if (selected !== undefined) typed.setCursor(selected);
  });
}

/**
 * ⚠ **The placeholder state, exercised with deliberately delayed plates.**
 *
 * Three of the eight open with no thumbnail at all, and the button below delivers them after a
 * visible pause — which is what R10's generator does. Select a pending slide, arrow through it, move
 * it with `Alt + Arrow Down`: all of it works while the picture is missing.
 */
export const APlateThatHasNotArrived: Story = {
  name: 'A Plate That Has Not Arrived',
  render: () => {
    fill('pending-rail', deckWithPlates(), 0);
    const deliver = (): void => {
      const rail = document.getElementById('pending-rail');
      if (!(rail instanceof HTMLElement)) return;
      const typed = rail as MjxThumbnailRail;
      const plates = everyPlate(typed.slides.length);
      window.setTimeout(() => {
        typed.slides = typed.slides.map((slide, index) => {
          const plate = plates[index];
          return plate === undefined ? slide : { ...slide, thumbnail: plate };
        });
        const readout = document.getElementById('rail-pending-readout');
        if (readout !== null) readout.textContent = `${String(typed.pendingCount)} still pending`;
      }, 900);
      const readout = document.getElementById('rail-pending-readout');
      if (readout !== null) readout.textContent = 'rendering…';
    };
    return stage(
      note(
        'Three of these eight slides have no plate yet. They draw a placeholder band, carry ' +
          'aria-busy on the frame rather than on the option — so the slide is still announced with ' +
          'its full name — and they remain selectable, scrollable and reorderable. Press the button ' +
          'and the plates arrive after a pause, exactly as the renderer delivers them.',
      ),
      html`
        <button type="button" @click=${deliver} style="align-self:flex-start">
          Deliver the plates
        </button>
        <mjx-thumbnail-rail
          id="pending-rail"
          label="Slides"
          style=${paneStyle}
        ></mjx-thumbnail-rail>
        <output id="rail-pending-readout" class="mjx-type-dense" style="color:var(--theme-text-secondary)"
          >three pending</output
        >
      `,
      caption([
        'The band stops moving under a reduced-motion preference, where the band alone still says waiting.',
        'Slide 6 is hidden: marked with a glyph and named as hidden. Deliberately NOT dimmed — the accessibility sweep measured a dimmed label at 2.79 : 1 against a floor of 4.5, and a hidden slide is not a disabled control.',
      ]),
    );
  },
};

/** ⚠ Five thousand slides, so the node-count assertion is about something. */
export const FiveThousandSlides: Story = {
  name: 'Five Thousand Slides',
  render: () => {
    fill('big-rail', largeDeck(), 0);
    return stage(
      note(
        `${String(largeNavigatorCount)} slides in twelve-slide sections, none of them yet rendered. ` +
          'The DOM holds a screenful. A section heading is a shorter row than a slide, so the rows ' +
          'are of two different heights before any caption wraps — which is why the window is found ' +
          'by a binary search over measured extents rather than by dividing.',
      ),
      html`<mjx-thumbnail-rail
        id="big-rail"
        label="Slides"
        style=${paneStyle}
      ></mjx-thumbnail-rail>`,
      caption([
        'Ctrl + click adds a slide to the selection; Shift + click takes the range; Ctrl + Space toggles the cursor’s slide.',
        'Alt + Arrow Down moves the whole selection as a block — never one slide at a time, which would quietly make a scattered selection contiguous.',
      ]),
    );
  },
};

/**
 * **The trap, in the rail's terms: a slide inserted above the visible range.**
 *
 * This is the case the ticket names by name. Scroll until a numbered slide is where you can see it,
 * then press the button. Twenty slides are inserted at the top. What you were looking at does not
 * move — and its *number* changes, which is the honest outcome: it is the twenty-first slide now.
 */
export const SlidesInsertedAboveTheViewport: Story = {
  name: 'Slides Inserted Above The Viewport',
  render: () => {
    fill('insert-rail', largeDeck(600), 0);
    const insert = (): void => {
      const rail = document.getElementById('insert-rail');
      if (!(rail instanceof HTMLElement)) return;
      const typed = rail as MjxThumbnailRail;
      const stamp = String(Date.now());
      const fresh: RailSlide[] = Array.from({ length: 20 }, (_unused, index) => ({
        id: `new-${stamp}-${String(index)}`,
        label: `Inserted ${String(index + 1)}`,
        section: 'Inserted',
      }));
      typed.slides = [...fresh, ...typed.slides];
      const readout = document.getElementById('insert-readout');
      if (readout !== null) {
        readout.textContent =
          `offset taken ${typed.offset.toFixed(0)} · the naive answer would have kept ` +
          `${typed.naiveOffset.toFixed(0)} · rows measured ${String(typed.measuredRowCount)}`;
      }
    };
    return stage(
      note(
        'Scroll down, note where a slide sits on the screen, then insert twenty above it. It stays ' +
          'where it is. The two numbers below are the assertion: the offset the rail took, and the ' +
          'offset it would have kept had it held the scroll position instead of an anchor.',
      ),
      html`
        <button type="button" @click=${insert} style="align-self:flex-start">
          Insert twenty slides at the top
        </button>
        <mjx-thumbnail-rail
          id="insert-rail"
          label="Slides"
          style=${paneStyle}
        ></mjx-thumbnail-rail>
        <output id="insert-readout" class="mjx-type-dense" style="color:var(--theme-text-secondary)"
          >press the button</output
        >
      `,
      caption([
        'The slide numbers renumber, because they are positions and the positions really did change. What must not move is the picture on the screen.',
      ]),
    );
  },
};

/** ⚠ **The hit-target floor holds in compact density.** */
export const InCompactDensity: Story = {
  name: 'In Compact Density',
  render: () => {
    fill('compact-rail', deckWithPlates(), 0);
    return html`<div data-density="compact" style="display:flex;flex-direction:column;min-block-size:0">
      ${stage(
        note(
          'Compact density. The thumbnails shrink with the gutter and the rows still clear the floor.',
        ),
        html`<mjx-thumbnail-rail
          id="compact-rail"
          label="Slides"
          style=${paneStyle}
        ></mjx-thumbnail-rail>`,
      )}
    </div>`;
  },
};
