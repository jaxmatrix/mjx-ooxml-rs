import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components-vite';

import { storyConventions } from '../../src/story/conventions.ts';
import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  canvasReservedGestures,
  gestureNames,
  gestureRegionNames,
  gestureRegions,
  gesturesSuppressedBy,
  reservedGestureException,
  resolveGesture,
} from '../../src/mobile/gesture-map.ts';
import {
  largePhoneLandscapeViewport,
  largePhoneViewport,
  mobileFormFactorNames,
  mobileFormFactors,
  shortViewportAtOrBelow,
  smallTabletAtOrBelow,
  smallTabletViewport,
  thumbReachBlockFraction,
  thumbReachThreshold,
} from '../../src/mobile/mobile-model.ts';
import { phoneShellAtOrBelow } from '../../src/harness/presets.ts';
import { caption, mobileTokenDependencies, note } from './specimens.ts';

/**
 * **The gesture conventions and the reachability rules — written for the person doing the audit.**
 *
 * This is a page rather than a component, and it is the deliverable the ticket asks for in words:
 * *"the gesture conventions and reachability rules are documented for the audit."* Everything on it
 * is generated from `src/mobile/gesture-map.ts` and `src/mobile/mobile-model.ts`, so the page cannot
 * say something the components do not do.
 *
 * **The one rule to take away from it:** the canvas wins any ambiguity, because the document is the
 * product. That is not a policy written beside the code — it is `resolveGesture`'s **default
 * branch**, which is the only shape in which it cannot be forgotten.
 */

const conventions = storyConventions({
  statesMatrix: [
    { name: 'the map', description: 'Seven gestures against seven regions, and who owns each.' },
    { name: 'the reserved two', description: 'Pinch and two-finger pan, and the one exception.' },
    { name: 'reachability', description: 'The band, at three viewport shapes.' },
    { name: 'the form factors', description: 'Which shape produces which presentation, and why.' },
  ],
  tokenDependencies: mobileTokenDependencies,
  keyboard: [
    {
      keys: 'not focusable',
      does: 'A documentation page. Every gesture on it has a keyboard equivalent on the component that offers it, and those are listed in that component’s own conventions.',
    },
  ],
  screenReader:
    'Three tables, each with a caption and a header row. Nothing here is interactive, so nothing ' +
    'announces a state.',
});

const meta: Meta = {
  // ⚠ A string literal — CSF is indexed statically and refuses a computed title.
  title: 'Mobile/Gestures And Reach',
  parameters: {
    docs: {
      description: {
        component:
          'What long-press does, what swipe does, where each is claimed, and how a conflict with ' +
          'the canvas’s own pan and pinch is avoided. Generated from the gesture map the ' +
          'components read.',
      },
    },
    mjx: conventions,
  },
};

export default meta;

type Story = StoryObj;

const cell = 'vertical-align:top;padding:var(--mjx-density-step);border-block-end:1px solid var(--theme-border-subtle)';
const head =
  'text-align:start;padding:var(--mjx-density-step);border-block-end:1px solid var(--theme-border)';
const table =
  'border-collapse:collapse;margin:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:90ch';

/** The map itself: who owns what, where. */
export const TheGestureMap: Story = {
  render: () =>
    html`
      ${note(
        'A region owns a gesture only if it says so. Everything a region has not claimed goes to ' +
          'the canvas — including tap, double-tap and long-press, which touch-action cannot ' +
          'express and which no region therefore claims. A claim is made by writing touch-action, ' +
          'not by adding a listener, which is why the two columns on the right must agree.',
      )}
      <table class=${typeRoleClass('body')} style=${table}>
        <caption class=${typeRoleClass('label')} style="text-align:start;padding:var(--mjx-density-step)">
          The seven regions
        </caption>
        <thead>
          <tr>
            ${['Region', 'Owner', 'touch-action', 'Claims', 'What it does', 'What it leaves'].map(
              (heading) => html`<th style=${head}>${heading}</th>`,
            )}
          </tr>
        </thead>
        <tbody>
          ${gestureRegionNames.map((region) => {
            const spec = gestureRegions[region];
            return html`
              <tr>
                <td style=${cell}><code>${region}</code></td>
                <td style=${cell}>${spec.owner}</td>
                <td style=${cell}><code>${spec.touchAction}</code></td>
                <td style=${cell}>
                  ${spec.claims.length === 0 ? 'nothing' : spec.claims.join(', ')}
                </td>
                <td style=${cell}>${spec.does}</td>
                <td style=${cell}>${spec.leaves}</td>
              </tr>
            `;
          })}
        </tbody>
      </table>
      ${caption([
        'Every row’s Claims column is derived from its touch-action, not written beside it: ' +
          gestureRegionNames
            .map((region) => `${region} → ${gesturesSuppressedBy(gestureRegions[region].touchAction).join('/') || 'nothing'}`)
            .join('; ') +
          '.',
        'A region that claimed one gesture and wrote a touch-action taking three would be a ' +
          'region that had quietly taken two more from the document. There is a test for it.',
      ])}
    `,
};

/** The ambiguity rule, applied to every pair. */
export const AmbiguityResolvesToTheCanvas: Story = {
  render: () =>
    html`
      ${note(
        'Seven gestures against seven regions: forty-nine answers, and forty-three of them are ' +
          '"canvas". That is the shape the rule is supposed to have. The document is the product, ' +
          'so the chrome takes only what it can name and justify, and everything else falls ' +
          'through to the canvas by default rather than by permission.',
      )}
      <table class=${typeRoleClass('body')} style=${table}>
        <caption class=${typeRoleClass('label')} style="text-align:start;padding:var(--mjx-density-step)">
          resolveGesture(gesture, region)
        </caption>
        <thead>
          <tr>
            <th style=${head}>Gesture</th>
            ${gestureRegionNames.map((region) => html`<th style=${head}>${region}</th>`)}
          </tr>
        </thead>
        <tbody>
          ${gestureNames.map(
            (gesture) => html`
              <tr>
                <td style=${cell}><code>${gesture}</code></td>
                ${gestureRegionNames.map(
                  (region) => html`<td style=${cell}>${resolveGesture(gesture, region)}</td>`,
                )}
              </tr>
            `,
          )}
        </tbody>
      </table>
      ${caption([
        `The canvas reserves ${canvasReservedGestures.join(' and ')} everywhere.`,
        `Exactly one region is allowed to take them — ${reservedGestureException} — and the ` +
          'argument is that a grab handle is a strip a few pixels tall that nobody pinches. A ' +
          'second region acquiring touch-action: none is a failing test with a name in it.',
        `Answers that are the canvas's: ${String(
          gestureNames.flatMap((gesture) =>
            gestureRegionNames.filter((region) => resolveGesture(gesture, region) === 'canvas'),
          ).length,
        )} of ${String(gestureNames.length * gestureRegionNames.length)}.`,
      ])}
    `,
};

/** Reachability at three viewport shapes. */
export const ReachabilityAndFormFactors: Story = {
  render: () =>
    html`
      ${note(
        'Reach is a layout constraint, not a preference. A primary action must lie wholly inside ' +
          'the band below the threshold, and the band is measured from the bottom of the ' +
          'viewport rather than of any container — a thumb does not know what a container query ' +
          'is.',
      )}
      <table class=${typeRoleClass('body')} style=${table}>
        <caption class=${typeRoleClass('label')} style="text-align:start;padding:var(--mjx-density-step)">
          The three shapes the bars are judged at
        </caption>
        <thead>
          <tr>
            ${['Viewport', 'Inline', 'Block', 'Reach threshold', 'Band'].map(
              (heading) => html`<th style=${head}>${heading}</th>`,
            )}
          </tr>
        </thead>
        <tbody>
          ${(
            [
              ['large phone, portrait', largePhoneViewport],
              ['large phone, landscape', largePhoneLandscapeViewport],
              ['small tablet, portrait', smallTabletViewport],
            ] as const
          ).map(
            ([name, shape]) => html`
              <tr>
                <td style=${cell}>${name}</td>
                <td style=${cell}>${String(shape.inline)}</td>
                <td style=${cell}>${String(shape.block)}</td>
                <td style=${cell}>${thumbReachThreshold(shape).toFixed(0)} px</td>
                <td style=${cell}>
                  ${thumbReachThreshold(shape).toFixed(0)}–${String(shape.block)} px
                </td>
              </tr>
            `,
          )}
        </tbody>
      </table>
      <table class=${typeRoleClass('body')} style=${table}>
        <caption class=${typeRoleClass('label')} style="text-align:start;padding:var(--mjx-density-step)">
          The four form factors
        </caption>
        <thead>
          <tr>
            ${['Form factor', 'Visible slots', 'Spans', 'What it is', 'Use'].map(
              (heading) => html`<th style=${head}>${heading}</th>`,
            )}
          </tr>
        </thead>
        <tbody>
          ${mobileFormFactorNames.map((factor) => {
            const spec = mobileFormFactors[factor];
            return html`
              <tr>
                <td style=${cell}><code>${factor}</code></td>
                <td style=${cell}>${String(spec.visibleSlots)}</td>
                <td style=${cell}>${spec.spans ? 'yes' : 'no'}</td>
                <td style=${cell}>${spec.description}</td>
                <td style=${cell}>${spec.use}</td>
              </tr>
            `;
          })}
        </tbody>
      </table>
      ${caption([
        `Thumb reach is the bottom ${(thumbReachBlockFraction * 100).toFixed(0)} % of the block axis.`,
        `Width is a CONTAINER query: at or below ${String(phoneShellAtOrBelow)} px a shell is a ` +
          `phone, at or below ${String(smallTabletAtOrBelow)} px it is a small tablet.`,
        `Shortness is a MEDIA query, because a viewport's block size is not a property of any ` +
          `container: at or below ${String(shortViewportAtOrBelow)} px the viewport is short, and ` +
          'a wide short viewport is a landscape phone rather than a tablet.',
        'That distinction is the whole reason this table exists. A landscape phone is 844 px wide ' +
          'and by width alone it is a tablet — which would lay eight commands across a screen ' +
          '390 px tall and put them all out of reach.',
      ])}
    `,
};
