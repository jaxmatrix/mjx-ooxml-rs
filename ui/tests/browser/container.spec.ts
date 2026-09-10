import { expect, test } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
// The data modules, not the element modules: a spec runs in Node, and `class extends HTMLElement`
// is evaluated at import time. See `src/harness/presets.ts` for the rule this establishes.
import { containerBreakpoints, presetBands } from '../../dev/bands.ts';
import { containerPresets } from '../../src/harness/presets.ts';

/**
 * The responsive mechanism.
 *
 * `BUILD_PLAN_LOOP_1.md` §3: *"A component that only looks right at a viewport width has not been
 * validated."* This suite is the assertion that turns that sentence into a gate, and the decisive
 * line is not that the probe changes band — it is that **`window.innerWidth` never moves while it
 * does**. A harness built on a viewport addon would satisfy every other assertion here and fail
 * that one.
 */

const story = builtStories().find(
  (entry) => entry.title === 'Gates/Container query' && entry.name === 'Responds To Its Container',
);

const twoAtOnce = builtStories().find(
  (entry) => entry.title === 'Gates/Container query' && entry.name === 'Two Widths At One Viewport',
);

test.describe('the resizable container', () => {
  test('is in the catalogue at all', () => {
    expect(story, 'Gates/Container query · Responds To Its Container is missing').toBeDefined();
  });

  test('changes the component’s band without changing the viewport', async ({ page }) => {
    test.skip(story === undefined, 'the container story is missing');
    if (story === undefined) return;
    await openStory(page, story.id, { containerPreset: 'desktop' });

    const viewportBefore = await page.evaluate(() => window.innerWidth);

    const bandAt = async (width: number): Promise<string | undefined> => {
      await page.evaluate((value) => {
        const container = document.querySelector('mjx-resizable-container');
        container?.setAttribute('width', String(value));
      }, width);
      await page.evaluate(async () => {
        await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
      });
      return page.evaluate(() => document.querySelector('mjx-container-probe')?.band);
    };

    // Each harness preset must land in a different band, or the harness would look like it worked
    // while proving nothing. The expected band is computed from the breakpoints rather than
    // written here, so a change to either table cannot leave this test asserting the old answer.
    expect(new Set(Object.values(presetBands)).size, 'two presets share a band').toBe(3);
    expect(await bandAt(containerPresets.phone)).toBe(presetBands.phone);
    expect(await bandAt(containerPresets.tablet)).toBe(presetBands.tablet);
    expect(await bandAt(containerPresets.desktop)).toBe(presetBands.desktop);

    // Either side of both breakpoints, to prove the query and not merely three coincidences.
    //
    // These assertions are exact only because the frame has no padding and no border: a container
    // query resolves against the CONTENT box, so any chrome inside the container would shift every
    // breakpoint by its own width. That was the first version's defect and this pair of
    // off-by-one assertions is what found it.
    expect(await bandAt(containerBreakpoints.medium - 1)).toBe('narrow');
    expect(await bandAt(containerBreakpoints.medium)).toBe('medium');
    expect(await bandAt(containerBreakpoints.wide - 1)).toBe('medium');
    expect(await bandAt(containerBreakpoints.wide)).toBe('wide');

    const viewportAfter = await page.evaluate(() => window.innerWidth);
    expect(
      viewportAfter,
      'the container changed the component and the viewport stayed exactly where it was. If this ' +
        'fails, the harness has become a viewport mechanism and every later component would be ' +
        'validated at a window width instead of at a container width.',
    ).toBe(viewportBefore);
  });

  test('reports exactly the width it was asked for, with no chrome inside the container', async ({
    page,
  }) => {
    test.skip(story === undefined, 'the container story is missing');
    if (story === undefined) return;
    const measure = async () =>
      page.evaluate(() => {
        const container = document.querySelector('mjx-resizable-container');
        const frame = container?.shadowRoot?.querySelector('.frame');
        if (!(frame instanceof HTMLElement)) throw new Error('the frame did not render');
        const style = getComputedStyle(frame);
        return {
          declared: container?.width ?? 0,
          content: frame.clientWidth,
          padding: `${style.paddingLeft} ${style.paddingRight}`,
          border: `${style.borderLeftWidth} ${style.borderRightWidth}`,
          viewport: window.innerWidth,
        };
      });

    // The assertion that keeps every later component's breakpoints honest. A container query
    // resolves against the container's content box, so padding, a border or a permanent scrollbar
    // gutter inside the frame would silently move every breakpoint in the catalogue.
    await openStory(page, story.id, { containerPreset: 'tablet' });
    const tablet = await measure();
    expect(tablet.padding).toBe('0px 0px');
    expect(tablet.border).toBe('0px 0px');
    expect(tablet.declared).toBe(containerPresets.tablet);
    expect(tablet.content).toBe(tablet.declared);

    // And the same at a preset **wider than the window**. This is the one that catches a frame
    // clamped with `max-inline-size: 100%`: the preset would then be a label rather than a width,
    // and the band assertions above would pass for the wrong reason because 1280 and 1440 fall in
    // the same band.
    await openStory(page, story.id, { containerPreset: 'desktop' });
    const desktop = await measure();
    expect(desktop.declared).toBe(containerPresets.desktop);
    expect(desktop.viewport).toBeLessThan(containerPresets.desktop);
    expect(desktop.content).toBe(containerPresets.desktop);
  });

  test('shows two different bands at one instant, which no viewport can do', async ({ page }) => {
    test.skip(twoAtOnce === undefined, 'the two-widths story is missing');
    if (twoAtOnce === undefined) return;
    await openStory(page, twoAtOnce.id);

    const bands = await page.evaluate(() =>
      [...document.querySelectorAll('mjx-container-probe')].map((probe) => probe.band),
    );
    expect(bands).toHaveLength(2);
    expect(bands[0]).toBe('narrow');
    expect(bands[1]).toBe('wide');
  });

  test('offers its presets to the keyboard, not only to a drag handle', async ({ page }) => {
    test.skip(story === undefined, 'the container story is missing');
    if (story === undefined) return;
    await openStory(page, story.id, { containerPreset: 'desktop' });

    const container = page.locator('mjx-resizable-container').first();
    await container.getByRole('button', { name: /phone/ }).click();
    await page.evaluate(async () => {
      await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
    });
    expect(await page.evaluate(() => document.querySelector('mjx-container-probe')?.band)).toBe(
      'narrow',
    );
    await expect(container.getByRole('button', { name: /phone/ })).toHaveAttribute(
      'aria-pressed',
      'true',
    );
    await expect(container.getByRole('slider', { name: /width/i })).toHaveValue(
      String(containerPresets.phone),
    );
  });
});
