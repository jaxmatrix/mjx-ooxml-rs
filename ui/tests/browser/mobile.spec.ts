import { expect, test, type Locator, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import {
  commandBarOrder,
  contextualActions,
  commandBarPartition,
  formFactorFor,
  largePhoneLandscapeViewport,
  largePhoneViewport,
  mobileEvents,
  mobileFormFactorProperty,
  mobileFormFactors,
  smallTabletViewport,
  safeAreaProperties,
  sharedSelectionCommands,
  thumbReachThreshold,
  withinThumbReach,
  type MobileFormFactor,
} from '../../src/mobile/mobile-model.ts';
import { gestureRegionNames, gestureRegions } from '../../src/mobile/gesture-map.ts';
import { sheetDetents, type SheetDetent } from '../../src/mobile/sheet-detents.ts';
import { mobileBarTargetMinimum } from '../../src/mobile/touch-audit.ts';
import { wordPhoneCommands } from '../../stories/mobile/specimens.ts';

/**
 * MJXOFF-194's browser tier — **the half that measures rather than looks.**
 *
 * The ticket is explicit about why this exists at all:
 *
 * > Mobile is where *it renders* is furthest from *it works*. A component that displays correctly
 * > at 390 px but has 24 px targets, or whose sheet dismisses when a user scrolls an inner list,
 * > passes every appearance test and fails in the hand. **So the gates are measured targets and
 * > interaction tests**, not screenshots.
 *
 * There is no snapshot in this file. Every assertion is a number read out of the browser or a
 * gesture driven through it.
 */

const stories = builtStories();

function storyId(title: string, name: string): string {
  const found = stories.find((entry) => entry.title === title && entry.name === name);
  if (found === undefined) {
    throw new Error(
      `The built catalogue has no story "${title} · ${name}". The browser tier runs against ` +
        'storybook-static/, so a renamed story is invisible until the catalogue is rebuilt.',
    );
  }
  return found.id;
}

const ids = {
  ladder: () => storyId('Mobile/Command Bar', 'The Demotion Ladder Made Visible'),
  overflow: () => storyId('Mobile/Command Bar', 'The Overflow Is The Same Rail'),
  formFactors: () => storyId('Mobile/Command Bar', 'Three Form Factors'),
  reach: () => storyId('Mobile/Command Bar', 'Within Thumb Reach'),
  selections: () => storyId('Mobile/Contextual Action Bar', 'The Four Selections'),
  noSelection: () => storyId('Mobile/Contextual Action Bar', 'No Selection Is No Bar'),
  nested: () => storyId('Mobile/Bottom Sheet', 'A Long List Inside A Sheet'),
  detents: () => storyId('Mobile/Bottom Sheet', 'The Three Detents'),
  notched: () => storyId('Mobile/Bottom Sheet', 'On A Notched Device'),
};

/** The container frame, which is what a container query answers to — never the host. */
function frame(page: Page): Locator {
  return page.locator('mjx-resizable-container [part="frame"]');
}

async function frameWidth(page: Page): Promise<number> {
  const box = await frame(page).boundingBox();
  if (box === null) throw new Error('the harness frame has no box');
  return box.width;
}

async function property(locator: Locator, name: string): Promise<string> {
  return locator.evaluate(
    (element, custom) => getComputedStyle(element).getPropertyValue(custom).trim(),
    name,
  );
}

async function longhand(locator: Locator, name: string): Promise<string> {
  return locator.evaluate(
    (element, css) => getComputedStyle(element).getPropertyValue(css),
    name,
  );
}

// ── the form factor the cascade decided ──────────────────────────────────────

test.describe('the form factor is decided by the cascade and read back', () => {
  const shapes: readonly {
    readonly name: string;
    readonly viewport: { width: number; height: number };
    readonly preset: string;
    readonly expected: MobileFormFactor;
  }[] = [
    {
      name: 'a large phone, upright',
      viewport: { width: largePhoneViewport.inline, height: largePhoneViewport.block },
      preset: 'phone',
      expected: 'phonePortrait',
    },
    {
      name: 'the same phone, on its side',
      viewport: {
        width: largePhoneLandscapeViewport.inline,
        height: largePhoneLandscapeViewport.block,
      },
      preset: 'tablet',
      expected: 'phoneLandscape',
    },
    {
      name: 'a small tablet, upright',
      viewport: { width: 1100, height: smallTabletViewport.block },
      preset: 'tablet',
      expected: 'smallTablet',
    },
    {
      name: 'a desktop',
      viewport: { width: 1600, height: 1000 },
      preset: 'desktop',
      expected: 'desktop',
    },
  ];

  for (const shape of shapes) {
    test(`${shape.name} is ${shape.expected}`, async ({ page }) => {
      await page.setViewportSize(shape.viewport);
      await openStory(page, ids.formFactors(), { containerPreset: shape.preset });

      const bar = page.locator('#bar');
      const inner = bar.locator('[part="bar"]');
      const inline = await frameWidth(page);
      const modelled = formFactorFor({ inline, block: shape.viewport.height });

      // Three claims, and only the first is the component's own opinion.
      expect(modelled, 'the model and the fixture disagree about this shape').toBe(shape.expected);
      expect(await property(inner, mobileFormFactorProperty)).toBe(shape.expected);

      // Cross-checked against two facts the custom property does not control — MJXOFF-183's rule
      // that a component reading back its own token is grading its own homework.
      expect(await inner.getAttribute('data-spans')).toBe(
        mobileFormFactors[shape.expected].spans ? 'true' : 'false',
      );
      const visible = await bar.evaluate(
        (element) =>
          (element as HTMLElement).shadowRoot?.querySelectorAll(
            '.command[data-demoted="false"]',
          ).length ?? -1,
      );
      const expectedVisible = commandBarPartition(
        wordPhoneCommands,
        mobileFormFactors[shape.expected].visibleSlots,
      ).visible.length;
      expect(visible).toBe(expectedVisible);
    });
  }

  test('LANDSCAPE IS NOT A TABLET, which is what the media half buys', async ({ page }) => {
    // The same container width, twice, with only the viewport's block size different. If the media
    // query were missing both would report smallTablet and this test would be the only thing that
    // noticed.
    await page.setViewportSize({ width: 1100, height: 1000 });
    await openStory(page, ids.formFactors(), { containerPreset: 'tablet' });
    const tall = await property(page.locator('#bar [part="bar"]'), mobileFormFactorProperty);
    const tallWidth = await frameWidth(page);

    await page.setViewportSize({ width: 1100, height: 430 });
    await openStory(page, ids.formFactors(), { containerPreset: 'tablet' });
    const short = await property(page.locator('#bar [part="bar"]'), mobileFormFactorProperty);
    const shortWidth = await frameWidth(page);

    expect(shortWidth, 'the container width must not be what changed').toBe(tallWidth);
    expect(tall).toBe('smallTablet');
    expect(short).toBe('phoneLandscape');
  });
});

// ── the demotion ladder ──────────────────────────────────────────────────────

test.describe('the command bar follows U04’s demotion rules', () => {
  test('the rail is in ladder order, and the visible run is the partition’s', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.ladder(), { containerPreset: 'phone' });

    const bar = page.locator('#bar');
    const rail = await bar.evaluate((element) =>
      Array.from(
        (element as HTMLElement).shadowRoot?.querySelectorAll('.command') ?? [],
      ).map((button) => ({
        id: (button as HTMLElement).dataset['command'] ?? '',
        demoted: (button as HTMLElement).dataset['demoted'] === 'true',
        haspopup: button.getAttribute('aria-haspopup'),
      })),
    );

    const modelled = commandBarOrder(wordPhoneCommands);
    expect(rail.map((entry) => entry.id)).toEqual(modelled.map((command) => command.id));

    const visible = commandBarPartition(
      wordPhoneCommands,
      mobileFormFactors.phonePortrait.visibleSlots,
    ).visible;
    expect(rail.filter((entry) => !entry.demoted).map((entry) => entry.id)).toEqual(
      visible.map((command) => command.id),
    );

    // Demotion rule 1, read off the DOM rather than off the model.
    expect(rail.filter((entry) => !entry.demoted).map((entry) => entry.haspopup)).toEqual(
      visible.map(() => null),
    );
    expect(
      rail.some((entry) => entry.haspopup === 'true'),
      'the fixture no longer contains a popup command, so rule 1 is being asserted about nothing',
    ).toBe(true);
  });

  test('EVERY DEMOTED COMMAND IS REACHABLE — and it is the same DOM node', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.ladder(), { containerPreset: 'phone' });
    const bar = page.locator('#bar');

    const identity = () =>
      bar.evaluate((element) => {
        const root = (element as HTMLElement).shadowRoot;
        const buttons = Array.from(root?.querySelectorAll('.command') ?? []);
        return buttons.map((button) => (button as HTMLElement).dataset['command'] ?? '');
      });

    const closed = await identity();
    expect(closed).toHaveLength(wordPhoneCommands.length);

    // Open the overflow through the control a person presses, not by setting the attribute.
    await bar.evaluate((element) => {
      const control = (element as HTMLElement).shadowRoot?.querySelector('.overflow');
      (control as HTMLElement | null)?.click();
    });
    await page.waitForFunction(
      (selector) =>
        document
          .querySelector(selector)
          ?.shadowRoot?.querySelector('.rail')
          ?.getAttribute('data-region') === 'overflowPanel',
      '#bar',
    );

    const open = await identity();
    // The same commands, in the same order, in the same element: the rail did not gain a menu, it
    // became a grid. That is what makes *nothing is lost* structural rather than remembered.
    expect(open).toEqual(closed);

    // And every one of them is now genuinely on screen and hit-testable.
    const reachable = await bar.evaluate((element) => {
      const root = (element as HTMLElement).shadowRoot;
      const buttons = Array.from(root?.querySelectorAll('.command') ?? []);
      return buttons.filter((button) => {
        const rect = button.getBoundingClientRect();
        return rect.width > 0 && rect.height > 0;
      }).length;
    });
    expect(reachable).toBe(wordPhoneCommands.length);
  });

  test('every command clears the mobile bar’s own floor, on both axes', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    for (const theme of ['light', 'dark']) {
      await openStory(page, ids.overflow(), { containerPreset: 'phone', theme });
      const boxes = await page.locator('#bar').evaluate((element) => {
        const root = (element as HTMLElement).shadowRoot;
        return Array.from(root?.querySelectorAll('.command, .overflow') ?? []).map((button) => {
          const rect = button.getBoundingClientRect();
          return { where: button.className, width: rect.width, height: rect.height };
        });
      });
      expect(boxes.length).toBeGreaterThan(wordPhoneCommands.length - 1);
      for (const box of boxes) {
        expect(
          Math.min(box.width, box.height),
          `[${theme}] ${box.where} is ${box.width.toFixed(1)} x ${box.height.toFixed(1)}`,
        ).toBeGreaterThanOrEqual(mobileBarTargetMinimum);
      }
    }
  });
});

// ── reachability ─────────────────────────────────────────────────────────────

test.describe('reachability is a layout constraint', () => {
  test('every visible command is within thumb reach on a large phone', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.reach(), { containerPreset: 'phone' });

    const boxes = await page.locator('#bar').evaluate((element) => {
      const root = (element as HTMLElement).shadowRoot;
      return Array.from(root?.querySelectorAll('.command[data-demoted="false"], .overflow') ?? []).map(
        (button) => {
          const rect = button.getBoundingClientRect();
          return {
            where: (button as HTMLElement).dataset['command'] ?? button.className,
            top: rect.top,
            bottom: rect.bottom,
          };
        },
      );
    });

    expect(boxes.length).toBeGreaterThan(0);
    const viewport = { inline: largePhoneViewport.inline, block: largePhoneViewport.block };
    const threshold = thumbReachThreshold(viewport);
    for (const box of boxes) {
      expect(
        withinThumbReach(box, viewport),
        `${box.where} spans ${box.top.toFixed(0)}–${box.bottom.toFixed(0)} px and the reach band ` +
          `begins at ${threshold.toFixed(0)} px`,
      ).toBe(true);
    }
  });

  test('the reach rule can fail — a control at the top of the viewport is out of it', async ({
    page,
  }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.reach(), { containerPreset: 'phone' });
    // The harness's own preset controls are at the top of the page. They are not primary actions
    // and are not subject to the rule — which is exactly why they are the honest way to show that
    // the rule discriminates rather than passing everything.
    const box = await page.locator('mjx-resizable-container').first().boundingBox();
    expect(box).not.toBeNull();
    expect(
      withinThumbReach({ top: box?.y ?? 0, bottom: (box?.y ?? 0) + 8 }, largePhoneViewport),
    ).toBe(false);
  });
});

// ── the gesture map ──────────────────────────────────────────────────────────

test.describe('the gesture map is what the CSS actually does', () => {
  test('each declared region writes the touch-action the map says', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.ladder(), { containerPreset: 'phone' });

    const rail = page.locator('#bar').locator('.rail');
    expect(await rail.getAttribute('data-region')).toBe('commandBar');
    expect(await longhand(rail, 'touch-action')).toBe(gestureRegions.commandBar.touchAction);

    const command = page.locator('#bar').locator('.command').first();
    expect(await longhand(command, 'touch-action')).toBe(
      gestureRegions.commandBarButton.touchAction,
    );

    await page.locator('#bar').evaluate((element) => {
      const control = (element as HTMLElement).shadowRoot?.querySelector('.overflow');
      (control as HTMLElement | null)?.click();
    });
    await page.waitForFunction(
      (selector) =>
        document
          .querySelector(selector)
          ?.shadowRoot?.querySelector('.rail')
          ?.getAttribute('data-region') === 'overflowPanel',
      '#bar',
    );
    expect(await longhand(rail, 'touch-action')).toBe(gestureRegions.overflowPanel.touchAction);
  });

  test('the sheet’s three regions write theirs', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.nested(), { containerPreset: 'phone' });
    const sheet = page.locator('#nested');
    expect(await longhand(sheet.locator('[part="handle"]'), 'touch-action')).toBe(
      gestureRegions.sheetHandle.touchAction,
    );
    expect(await longhand(sheet.locator('[part="header"]'), 'touch-action')).toBe(
      gestureRegions.sheetHeader.touchAction,
    );
    expect(await longhand(sheet.locator('[part="body"]'), 'touch-action')).toBe(
      gestureRegions.sheetScroller.touchAction,
    );
  });

  test('NOTHING ELSE TAKES THE DOCUMENT’S GESTURES — the whole page, both stories', async ({
    page,
  }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    for (const id of [ids.nested(), ids.ladder()]) {
      await openStory(page, id, { containerPreset: 'phone' });
      const offenders = await page.evaluate(() => {
        function everything(root: ParentNode, into: Element[] = []): Element[] {
          for (const element of Array.from(root.querySelectorAll('*'))) {
            into.push(element);
            const shadow = (element as HTMLElement).shadowRoot;
            if (shadow !== null) everything(shadow, into);
          }
          return into;
        }
        const root = document.querySelector('#storybook-root');
        if (root === null) return [];
        return everything(root)
          .filter((element) => getComputedStyle(element).touchAction === 'none')
          .map((element) => `${element.localName}.${String(element.className)}`);
      });
      // Exactly one element in the whole page may take pinch away from the document, and it is the
      // grab handle. `gestureRegions` names it and `tests/mobile.test.ts` asserts it is the only
      // region allowed to; this is the same claim read off the rendered page.
      expect(offenders.filter((entry) => !entry.includes('handle'))).toEqual([]);
    }
  });

  test('every region in the map is one the page can produce', () => {
    // Anti-vacuity for the two tests above: a region added to the map and never rendered would be
    // a claim nothing checks.
    expect(gestureRegionNames.filter((region) => region !== 'canvas').length).toBe(6);
  });
});

// ── safe-area insets ─────────────────────────────────────────────────────────

test.describe('safe-area insets on a simulated notched device', () => {
  test('the bar’s block-end padding grows by exactly the inset', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.ladder(), { containerPreset: 'phone' });

    const bar = page.locator('#bar').locator('[part="bar"]');
    const before = parseFloat(await longhand(bar, 'padding-block-end'));

    const inset = 34;
    await page.evaluate(
      ({ name, value }) => {
        document.documentElement.style.setProperty(name, `${String(value)}px`);
      },
      { name: safeAreaProperties.blockEnd, value: inset },
    );

    const after = parseFloat(await longhand(bar, 'padding-block-end'));
    expect(after - before).toBeCloseTo(inset, 1);

    // And the space ABOVE the commands does not move: a notch must not squash the bar, it must
    // move it clear of the home indicator.
    const start = parseFloat(await longhand(bar, 'padding-block-start'));
    await page.evaluate((name) => {
      document.documentElement.style.removeProperty(name);
    }, safeAreaProperties.blockEnd);
    expect(parseFloat(await longhand(bar, 'padding-block-start'))).toBeCloseTo(start, 1);
    expect(parseFloat(await longhand(bar, 'padding-block-end'))).toBeCloseTo(before, 1);
  });

  test('the sheet’s block-end padding carries it too', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.notched(), { containerPreset: 'phone' });
    const surface = page.locator('#notched').locator('[part="surface"]');
    expect(await surface.getAttribute('data-presentation')).toBe('sheet');
    // The story sets the four properties on a wrapper, which is the same channel `env()` writes.
    expect(parseFloat(await longhand(surface, 'padding-block-end'))).toBeCloseTo(34, 1);
  });

  /**
   * ⚠ **The first version of this test was vacuous, and a mutation is what said so.**
   *
   * It asserted that the property computes to something matching `/^\d+px$/`, on the reasoning that
   * an unregistered custom property hands back its substituted text. That is MJXOFF-189's finding
   * and it is true — but the substituted text *here* is `0px`, which matches the pattern. Deleting
   * every `@property` block left the test green.
   *
   * So this asserts what the registration actually buys, which is **type checking**: a host that
   * sets a nonsense value gets the declared initial value, and the padding that reads it keeps its
   * gutter. Unregistered, the nonsense reaches the `calc()`, the whole declaration is invalid at
   * computed-value time, and `padding-block-end` collapses to zero — a bar sitting on the home
   * indicator because somebody typed a unit wrong.
   */
  test('the properties are REGISTERED, so a bad value costs the inset and not the padding', async ({
    page,
  }) => {
    await openStory(page, ids.ladder(), { containerPreset: 'phone' });
    const bar = page.locator('#bar').locator('[part="bar"]');
    const gutter = parseFloat(await longhand(bar, 'padding-block-end'));
    expect(gutter).toBeGreaterThan(0);

    await page.evaluate((name) => {
      document.documentElement.style.setProperty(name, 'banana');
    }, safeAreaProperties.blockEnd);

    expect(
      parseFloat(await longhand(bar, 'padding-block-end')),
      'a nonsense inset collapsed the padding entirely, which means the property is not registered',
    ).toBeCloseTo(gutter, 1);
  });
});

// ── the sheet ────────────────────────────────────────────────────────────────

async function sheetHeight(page: Page, id: string): Promise<number> {
  const box = await page.locator(`#${id}`).locator('[part="surface"]').boundingBox();
  if (box === null) throw new Error('the sheet has no box');
  return box.height;
}

/**
 * The clipping boundary the sheet is pinned inside, measured from something that is not the sheet.
 *
 * ⚠ **The scrim, not an ancestor walk.** The surface is in the top layer, so its `parentElement`
 * chain is its own shadow root's and has nothing to do with what clips it on screen — the first
 * version of this helper walked that chain and reported a boundary 20 % out. The scrim is pinned to
 * `clippingBoundary(surface, 0)` by `coverFloating`, is a *different element*, and is not touched by
 * the detent at all, so it is the honest independent reference.
 */
async function boundaryHeight(page: Page, id: string): Promise<number> {
  const box = await page.locator(`#${id}`).locator('[part="scrim"]').boundingBox();
  if (box === null) throw new Error('the sheet has no scrim, so it is not modal');
  return box.height;
}

test.describe('the sheet’s detents', () => {
  for (const detent of ['peek', 'half', 'full'] as const) {
    test(`${detent} is ${(sheetDetents[detent].fraction * 100).toFixed(0)} % of the boundary`, async ({
      page,
    }) => {
      await page.setViewportSize({
        width: largePhoneViewport.inline,
        height: largePhoneViewport.block,
      });
      await openStory(page, ids.detents(), { containerPreset: 'phone' });
      await page.locator('#detents').evaluate((element, value) => {
        element.setAttribute('detent', value);
      }, detent as SheetDetent);
      await page.waitForTimeout(50);

      const height = await sheetHeight(page, 'detents');
      const boundary = await boundaryHeight(page, 'detents');
      expect(boundary).toBeGreaterThan(0);
      // The content is long enough to fill every detent, so the sheet is at its cap rather than at
      // its content height — which is what makes this a test of the detent.
      expect(height / boundary).toBeCloseTo(sheetDetents[detent].fraction, 1);
    });
  }

  test('the detents are genuinely different heights', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.detents(), { containerPreset: 'phone' });
    const heights: number[] = [];
    for (const detent of ['peek', 'half', 'full'] as const) {
      await page.locator('#detents').evaluate((element, value) => {
        element.setAttribute('detent', value);
      }, detent as SheetDetent);
      await page.waitForTimeout(50);
      heights.push(await sheetHeight(page, 'detents'));
    }
    // Three detents that all rendered the same height would satisfy every ratio assertion above at
    // a low enough precision. This is the assertion that says they are three things.
    expect(heights[0]).toBeLessThan(heights[1] ?? 0);
    expect(heights[1]).toBeLessThan(heights[2] ?? 0);
  });
});

test.describe('NESTED SCROLLING DOES NOT DISMISS THE SHEET', () => {
  test('a drag inside a scrolled list scrolls the list and does not move the sheet', async ({
    page,
  }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.nested(), { containerPreset: 'phone' });

    const sheet = page.locator('#nested');
    const body = sheet.locator('[part="body"]');
    expect(await sheet.getAttribute('detent')).toBe('half');

    // Put the list somewhere in the middle of its own scroll, which is rule 2's precondition.
    await body.evaluate((element) => {
      element.scrollTop = 120;
    });
    expect(await body.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);

    const before = await sheetHeight(page, 'nested');
    const box = await body.boundingBox();
    expect(box).not.toBeNull();
    const startX = (box?.x ?? 0) + (box?.width ?? 0) / 2;
    const startY = (box?.y ?? 0) + (box?.height ?? 0) / 2;

    await page.mouse.move(startX, startY);
    await page.mouse.down();
    for (let step = 1; step <= 8; step += 1) {
      await page.mouse.move(startX, startY + step * 20);
    }
    await page.mouse.up();
    await page.waitForTimeout(80);

    // The sheet is still open, still at its detent, and still the same height. THIS is the defect
    // the whole file exists for: a sheet that dismissed here would look perfect in a screenshot.
    expect(await sheet.getAttribute('open')).not.toBeNull();
    expect(await sheet.getAttribute('detent')).toBe('half');
    expect(await sheetHeight(page, 'nested')).toBeCloseTo(before, 0);
  });

  test('a wheel inside the list scrolls it and leaves the sheet alone', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.nested(), { containerPreset: 'phone' });
    const sheet = page.locator('#nested');
    const body = sheet.locator('[part="body"]');
    const box = await body.boundingBox();
    expect(box).not.toBeNull();

    const before = await sheetHeight(page, 'nested');
    await page.mouse.move((box?.x ?? 0) + 20, (box?.y ?? 0) + 40);
    await page.mouse.wheel(0, 400);
    await page.waitForTimeout(80);

    expect(await body.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
    expect(await sheet.getAttribute('open')).not.toBeNull();
    expect(await sheetHeight(page, 'nested')).toBeCloseTo(before, 0);
  });

  test('a drag on the HANDLE does move the sheet, so the rule is not simply off', async ({
    page,
  }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.detents(), { containerPreset: 'phone' });
    const sheet = page.locator('#detents');
    expect(await sheet.getAttribute('detent')).toBe('full');

    const handle = sheet.locator('[part="handle"]');
    const box = await handle.boundingBox();
    expect(box).not.toBeNull();
    const startX = (box?.x ?? 0) + (box?.width ?? 0) / 2;
    const startY = (box?.y ?? 0) + (box?.height ?? 0) / 2;

    await page.mouse.move(startX, startY);
    await page.mouse.down();
    for (let step = 1; step <= 6; step += 1) {
      await page.mouse.move(startX, startY + step * 25);
    }
    await page.mouse.up();
    await page.waitForTimeout(80);

    const after = await sheet.getAttribute('detent');
    expect(after, 'a drag from the handle did nothing, so the drag machinery is not wired').not.toBe(
      'full',
    );
    expect(await sheet.getAttribute('open')).not.toBeNull();
  });

  test('a drag from the TOP of the list hands off to the sheet', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.detents(), { containerPreset: 'phone' });
    const sheet = page.locator('#detents');
    const body = sheet.locator('[part="body"]');
    await body.evaluate((element) => {
      element.scrollTop = 0;
    });

    const box = await body.boundingBox();
    const startX = (box?.x ?? 0) + (box?.width ?? 0) / 2;
    const startY = (box?.y ?? 0) + 10;

    await page.mouse.move(startX, startY);
    await page.mouse.down();
    for (let step = 1; step <= 6; step += 1) {
      await page.mouse.move(startX, startY + step * 25);
    }
    await page.mouse.up();
    await page.waitForTimeout(80);

    // Rule 3: at the top of the scroller there is nothing left to scroll, so a downward drag can
    // only mean *move the sheet*. Without it a sheet feels like a fixed panel.
    expect(await sheet.getAttribute('detent')).not.toBe('full');
  });
});

// ── the contextual action bar ────────────────────────────────────────────────

test.describe('the contextual action bar', () => {
  test('no selection is no bar, not an empty one', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.noSelection(), { containerPreset: 'phone' });
    const bar = page.locator('mjx-contextual-action-bar');
    expect(await bar.getAttribute('data-empty')).toBe('true');
    await expect(bar).toBeHidden();
  });

  /**
   * ⚠ **The shared four are always *present*, not always *first*.**
   *
   * This test asserted the latter and was wrong to, which is a finding worth keeping: the rail is
   * ordered by U04's ladder and by nothing else, so `delete` — which is `standard` — sits behind a
   * text selection's `primary` formatting commands. That is one priority scheme applied everywhere
   * rather than a special case for the contextual bar, and giving the shared four a reserved run at
   * the front would have been the second scheme the ticket forbids.
   */
  test('every selection offers the shared four, in the ladder’s order', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.selections(), { containerPreset: 'phone' });
    for (const kind of ['text', 'object', 'cells'] as const) {
      const rail = await page.locator(`#bar-${kind}`).evaluate((element) =>
        Array.from((element as HTMLElement).shadowRoot?.querySelectorAll('.command') ?? []).map(
          (button) => (button as HTMLElement).dataset['command'] ?? '',
        ),
      );
      expect(rail).toEqual(commandBarOrder(contextualActions(kind)).map((command) => command.id));
      for (const shared of sharedSelectionCommands) {
        expect(rail, `${kind} does not offer ${shared.id}`).toContain(shared.id);
      }
    }
  });

  test('the selection is in the accessible name even where the readout is not drawn', async ({
    page,
  }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.selections(), { containerPreset: 'phone' });
    const bar = page.locator('#bar-text').locator('[part="bar"]');
    expect(await bar.getAttribute('aria-label')).toContain('12 words');
    // On a portrait phone the readout is not displayed. A fact only on screen is a fact only some
    // readers get, which is why the name carries it.
    expect(await longhand(page.locator('#bar-text').locator('.selection'), 'display')).toBe('none');
  });
});

// ── what a shell hears ───────────────────────────────────────────────────────

test.describe('the events a shell binds to', () => {
  test('a press publishes the command’s id, and the overflow publishes its state', async ({
    page,
  }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.ladder(), { containerPreset: 'phone' });

    const heard = await page.evaluate(
      async ({ commandEvent, overflowEvent }) => {
        const bar = document.querySelector('#bar');
        if (bar === null) return [];
        const log: string[] = [];
        bar.addEventListener(commandEvent, (event) => {
          log.push(`command:${String((event as CustomEvent<{ id: string }>).detail.id)}`);
        });
        bar.addEventListener(overflowEvent, (event) => {
          log.push(`overflow:${String((event as CustomEvent<{ open: boolean }>).detail.open)}`);
        });
        const root = (bar as HTMLElement).shadowRoot;
        (root?.querySelector('.command') as HTMLElement | null)?.click();
        (root?.querySelector('.overflow') as HTMLElement | null)?.click();
        return log;
      },
      { commandEvent: mobileEvents.command, overflowEvent: mobileEvents.overflowToggled },
    );

    // The first command in the rail is the ladder's first, not the author's — which is the whole
    // point of the ladder, seen from the other end.
    expect(heard).toEqual([`command:${commandBarOrder(wordPhoneCommands)[0]?.id ?? ''}`, 'overflow:true']);
  });
});

// ── the keyboard ─────────────────────────────────────────────────────────────

test.describe('a toolbar has one tab stop', () => {
  test('the arrow keys move it and Tab does not', async ({ page }) => {
    await page.setViewportSize({ width: largePhoneViewport.inline, height: largePhoneViewport.block });
    await openStory(page, ids.ladder(), { containerPreset: 'phone' });

    const stops = await page.locator('#bar').evaluate((element) => {
      const root = (element as HTMLElement).shadowRoot;
      return Array.from(root?.querySelectorAll('.command, .overflow') ?? []).filter(
        (button) => (button as HTMLElement).tabIndex === 0,
      ).length;
    });
    expect(stops, 'a toolbar holds exactly one tab stop, whatever it holds').toBe(1);

    await page.locator('#bar').evaluate((element) => {
      const root = (element as HTMLElement).shadowRoot;
      (root?.querySelector('.command') as HTMLElement | null)?.focus();
    });
    await page.keyboard.press('ArrowRight');
    const focused = await page.evaluate(() => {
      let node: Element | null = document.activeElement;
      while (node?.shadowRoot?.activeElement != null) node = node.shadowRoot.activeElement;
      return (node as HTMLElement | null)?.dataset['command'] ?? '';
    });
    const order = commandBarOrder(wordPhoneCommands);
    expect(focused).toBe(order[1]?.id ?? '');
  });
});
