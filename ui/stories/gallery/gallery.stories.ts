import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import {
  galleryKeyboard,
  galleryScreenReader,
  galleryStatesFor,
  galleryStatesMatrix,
  galleryTokenDependencies,
  largeGalleryItems,
  note,
  previewStage,
  specimenCss,
  styleGalleryItems,
  threeKindsSpecimen,
} from './specimens.ts';

/**
 * `<mjx-gallery>` and `<mjx-gallery-item>` — **1,773 of Office's published controls**, the
 * second-largest archetype after `button`, and the only one whose defining behaviour a screenshot
 * cannot show at all.
 *
 * **Open *Live Preview* first, and use a pointer.** Hover a style: the paragraph below takes it.
 * Move to another: it takes that one. Move off the gallery entirely: the paragraph goes back to
 * exactly what it was, not to what it started as. Then press a style, hover others, and move
 * away — it goes back to the one you pressed. That last sentence is the whole feature, and it is
 * the one a still cannot distinguish from a gallery with the events unwired.
 */

const conventions = storyConventions({
  statesMatrix: galleryStatesFor(),
  tokenDependencies: galleryTokenDependencies(),
  keyboard: galleryKeyboard,
  screenReader: galleryScreenReader,
});

const meta: Meta = {
  // ⚠ A string literal, and it must stay one — Storybook indexes CSF statically and refuses a
  // computed title. `tests/browser/gallery.spec.ts` looks every story up by this string.
  title: 'Galleries/Gallery',
  parameters: {
    docs: {
      description: {
        component:
          'The in-ribbon strip, the expanded flyout and live preview — with the protocol asserted ' +
          'as an event sequence rather than as an appearance, virtualisation asserted on a node ' +
          'count, and two-dimensional keyboard navigation over a deliberately ragged grid.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const pad = 'padding:calc(var(--mjx-density-gutter) * 2)';

/**
 * **The story every gate that measures paint opens.**
 *
 * Seven inline strips, one cell each, one state each. The two a pointer produces are forced so a
 * static page can show them; `focus` never is.
 */
export const TheStatesMatrix: Story = {
  name: 'The States Matrix',
  render: () => html`
    ${note(
      'Seven states. The paint comes from the same table a ribbon button’s does — a gallery cell ' +
        'is not a second opinion about what “hovered” looks like — and there is deliberately no ' +
        '“previewing” state, because the cell that is previewing is the cell the pointer is over ' +
        'and an eighth state nobody could tell from hover is a state that should not exist.',
    )}
    ${galleryStatesMatrix()}
  `,
};

/**
 * **The story the live-preview gate drives, and the one to open first.**
 *
 * The stage listens for the three protocol events and applies them with `applyPreviewEvent` — the
 * shipped listener, not a copy — writing every one into a hidden log the gate reads as a sequence.
 */
export const LivePreview: Story = {
  name: 'Live Preview',
  render: () =>
    html`
      ${note(
        'Hover a style and the paragraph takes it. Move away and it comes back — to what was ' +
          'committed, not to what it started as, which is the difference a still cannot show. ' +
          'Press one to commit it, then hover others and leave: it returns to the one you pressed. ' +
          'Everything above is also true of the arrow keys, and Escape reverts.',
      )}
      ${previewStage(
        html`<mjx-gallery
          id="gallery-under-test"
          label="Styles"
          value="normal"
          style="max-inline-size:26rem"
        >
          ${styleGalleryItems()}
          <button
            slot="footer"
            type="button"
            class="mjx-type-control mjx-hit-target"
            style="border:1px solid var(--theme-border);border-radius:var(--radius-control);background:var(--theme-surface);color:var(--theme-text-primary);padding-inline:var(--mjx-density-gutter);cursor:pointer"
          >
            Save Selection as a New Style…
          </button>
        </mjx-gallery>`,
        '26rem',
      )}
    `,
};

/**
 * **The story the virtualisation gate drives.**
 *
 * Four hundred items in four sections. A gallery that built them all would be caught here by a node
 * count; a twenty-item fixture would prove nothing at all.
 */
export const ALargeGallery: Story = {
  name: 'A Large Gallery',
  render: () => html`
    ${note(
      'Four hundred themes. Scroll the strip, then press the double chevron and scroll the flyout. ' +
        'Open devtools and count the cells: there are a few dozen at any moment, and the scrollbar ' +
        'still says four hundred, because the sizer is as tall as every row whether or not the row ' +
        'was built.',
    )}
    ${specimenCss}
    <div style=${pad}>
      <mjx-gallery id="large-gallery" label="Themes" value="theme-0" style="max-inline-size:34rem">
        ${largeGalleryItems()}
      </mjx-gallery>
    </div>
  `,
};

/**
 * **The story that proves the item is a slot.**
 *
 * A formatted-text miniature, a drawn shape and a block of colour, in three galleries built by the
 * same component, which knows what none of them are.
 */
export const ThreeKindsOfItem: Story = {
  name: 'Three Kinds Of Item',
  render: () => html`
    ${note(
      'The component renders none of this. An item captures whatever markup its author wrote into ' +
        'a fragment and the gallery clones it into the cells it decides to build — which is also ' +
        'what makes an off-screen item cost nothing and what lets the strip and the flyout show ' +
        'the same item at the same time.',
    )}
    ${threeKindsSpecimen()}
  `,
};

/**
 * **The story the degradation gate drives.**
 *
 * The same gallery in three ribbon groups at three priorities. Drive the container narrower and
 * watch each one lose a row as its group reduces — the gallery never reads a width, only the
 * `--mjx-group-presentation` its group publishes.
 */
export const DegradingWithItsGroup: Story = {
  name: 'Degrading With Its Group',
  render: () => html`
    ${specimenCss}
    ${note(
      'A gallery inside a ribbon group. Resize the container: the group decides its own ' +
        'presentation from the priority ladder, publishes it as a custom property, and the ' +
        'gallery reads it and shows one row instead of two. Nothing here measures a width.',
    )}
    <mjx-ribbon label="Word" selected="home" state="expanded">
      <mjx-ribbon-tab tab-id="home" label="Home">
        <mjx-ribbon-group label="Styles" priority="primary">
          <mjx-gallery id="grouped-gallery" label="Styles" value="normal">
            ${styleGalleryItems()}
          </mjx-gallery>
        </mjx-ribbon-group>
        <mjx-ribbon-group label="Shapes" priority="ancillary">
          <mjx-gallery label="Shapes" value="normal"> ${styleGalleryItems()} </mjx-gallery>
        </mjx-ribbon-group>
      </mjx-ribbon-tab>
    </mjx-ribbon>
  `,
};

/** The same gallery with the line running the other way. */
export const UnderRightToLeft: Story = {
  name: 'Under Right To Left',
  render: () => html`
    ${specimenCss}
    ${note(
      'Exactly the same component with `dir="rtl"`. Arrow Left is *next* and Arrow Right is ' +
        '*previous*, because a gallery reads in the direction the line runs — one function decides ' +
        'both rather than a second code path. The flyout flips and shifts through the same ' +
        'placement primitive a menu uses.',
    )}
    <div dir="rtl" style=${pad}>
      <mjx-gallery id="rtl-gallery" label="الأنماط" value="normal" style="max-inline-size:26rem">
        ${styleGalleryItems()}
      </mjx-gallery>
    </div>
  `,
};

/**
 * **The story the touch gate drives.**
 *
 * At a phone-width container the flyout is a sheet, and live preview is press-and-hold.
 */
export const OnAPhone: Story = {
  name: 'On A Phone',
  render: () =>
    html`
      ${note(
        'Set the container to phone. The expanded gallery is pinned to the bottom edge at full ' +
          'width, because a floating grid aimed at with a thumb is a grid nobody hits. Live ' +
          'preview becomes press-and-hold: hold a cell to see it, release to put back exactly ' +
          'what was there, tap to apply it. That choice is argued in `touchPreviewAffordance` and ' +
          'marked as a guess, because Office’s own mobile applications have no live preview to be ' +
          'parity with.',
      )}
      ${previewStage(
        html`<mjx-gallery
          id="gallery-under-test"
          label="Styles"
          value="normal"
          style="max-inline-size:26rem"
        >
          ${styleGalleryItems()}
        </mjx-gallery>`,
        '30rem',
      )}
    `,
};
