import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import { containerPresets } from '../../src/harness/presets.ts';
import { sliderFraction } from '../../src/inputs/input-model.ts';
import {
  fitToPagePercent,
  fitToWidthPercent,
  furnitureStoryTitles,
  scrollMarkSpecs,
  statusPresentationAt,
  statusPriorityNames,
  zoomBounds,
  zoomStepFrom,
} from '../../src/furniture/furniture-model.ts';
import {
  ScrollbarModel,
  markFraction,
  thumbFraction,
  thumbStart,
  type ScrollMark,
} from '../../src/furniture/scroll-model.ts';

/**
 * MJXOFF-190's furniture, driven in a real browser.
 *
 * **Furniture is where *"it looks fine"* hides best**, because it is chrome nobody stares at — so
 * every assertion below is a position, a size or a state, as a number, and none of them is a
 * screenshot.
 *
 * Four things here cannot be proved anywhere else:
 *
 * * **the thumb does not jump when the extent changes mid-drag.** The pointer is really held down
 *   through Playwright's own input path while the extent is revised underneath it, because the
 *   whole failure mode is temporal and a synthetic event sequence proves only that a handler exists;
 * * **the status bar's ladder is CSS's decision**, read back out of the cascade and compared against
 *   `statusPresentationAt()` — a component that decided its own presentation and then reported it
 *   would be grading its own homework;
 * * **what is announced**, which lives entirely in the accessibility tree and is invisible to
 *   every other instrument;
 * * **the hit-target floor**, which is a promise about pixels and can only be measured.
 *
 * ## Two positive controls
 *
 * The mid-drag assertion computes, beside the real answer, **what the naive recomputation would have
 * produced** and asserts that the two differ by more than the tolerance. Without that, a stability
 * assertion is a tolerance nobody has tested. The same shape appears in the marks test, which
 * asserts the marks are at *different* positions before asserting each is at the right one.
 */

const statusBar = furnitureStoryTitles.statusBar;
const zoomControl = furnitureStoryTitles.zoomControl;
const scrollbar = furnitureStoryTitles.scrollbar;
const splitter = furnitureStoryTitles.splitter;

const ladderStory = { title: statusBar, name: 'The Priority Ladder' } as const;
const announceStory = { title: statusBar, name: 'What Is Announced, And What Is Not' } as const;
const compactBarStory = { title: statusBar, name: 'In Compact Density' } as const;

const restingZoomStory = { title: zoomControl, name: 'At One Hundred Per Cent' } as const;
const fitsStory = { title: zoomControl, name: 'The Two Fits' } as const;
const boundsStory = { title: zoomControl, name: 'At The Bounds' } as const;
const refusesStory = { title: zoomControl, name: 'What It Refuses, And What It Clamps' } as const;
const compactZoomStory = { title: zoomControl, name: 'In Compact Density' } as const;

const correctedStory = { title: scrollbar, name: 'An Estimate Being Corrected' } as const;
const ratiosStory = { title: scrollbar, name: 'Four Content Ratios' } as const;
const channelStory = { title: scrollbar, name: 'The Channel Of Marks' } as const;
const compactBarsStory = { title: scrollbar, name: 'In Compact Density' } as const;

const navigatorStory = { title: splitter, name: 'A Navigator Beside A Document' } as const;
const minimumsStory = { title: splitter, name: 'A Minimum Per Side' } as const;
const rememberedStory = { title: splitter, name: 'Remembered Across A Remount' } as const;
const compactSplitStory = { title: splitter, name: 'In Compact Density' } as const;

async function open(
  page: Page,
  story: { readonly title: string; readonly name: string },
  options: { containerPreset?: string } = {},
): Promise<void> {
  const entry = builtStories().find(
    (candidate) => candidate.title === story.title && candidate.name === story.name,
  );
  expect(entry, `${story.title} · ${story.name} is missing from the catalogue`).toBeDefined();
  if (entry === undefined) return;
  await openStory(page, entry.id, options);
  await page.waitForFunction(() => customElements.get('mjx-status-bar') !== undefined);
  await page.waitForFunction(() => customElements.get('mjx-zoom-control') !== undefined);
  await page.waitForFunction(() => customElements.get('mjx-scrollbar') !== undefined);
  await page.waitForFunction(() => customElements.get('mjx-splitter') !== undefined);
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/** Resize the harness's frame in place, which is what makes a node-identity assertion possible. */
async function resizeFrame(page: Page, width: number): Promise<void> {
  await page.evaluate((wanted) => {
    const frame = document.querySelector('mjx-resizable-container') as HTMLElement & {
      width: number;
    };
    frame.width = wanted;
  }, width);
  await settle(page);
  await settle(page);
}

// ── the status bar ───────────────────────────────────────────────────────────

test.describe('the status bar drops segments in a declared order', () => {
  test('the cascade decides, and the component agrees with the model at three widths', async ({
    page,
  }) => {
    await open(page, ladderStory);
    for (const width of [containerPresets.desktop, containerPresets.tablet, containerPresets.phone]) {
      await resizeFrame(page, width);
      const readback = await page.evaluate((priorities) => {
        const bar = document.querySelector('mjx-status-bar') as never as {
          presentationOf: (priority: string) => string;
        };
        return priorities.map((priority) => bar.presentationOf(priority));
      }, [...statusPriorityNames]);
      const expected = statusPriorityNames.map((priority) =>
        statusPresentationAt(priority, width),
      );
      expect(readback, `at ${String(width)}px`).toEqual(expected);
    }
  });

  test('a demoted segment is the same element, moved — never a second copy', async ({ page }) => {
    await open(page, ladderStory);
    // Stamp every segment, at the widest width, with a token nothing else writes. A copy rendered
    // into the overflow would not carry it, and a *count* would not notice the difference.
    await resizeFrame(page, containerPresets.desktop);
    const stamped = await page.evaluate(() => {
      const segments = [...document.querySelectorAll('mjx-status-segment')];
      segments.forEach((segment, index) => {
        (segment as HTMLElement & { mjxStamp?: number }).mjxStamp = index;
      });
      return segments.length;
    });
    expect(stamped).toBeGreaterThan(0);

    await resizeFrame(page, containerPresets.phone);
    const after = await page.evaluate(() => {
      const bar = document.querySelector('mjx-status-bar') as never as {
        segments: (HTMLElement & { mjxStamp?: number; label: string })[];
        shown: { label: string }[];
        overflowed: { label: string }[];
      };
      return {
        total: bar.segments.length,
        stamps: bar.segments.map((segment) => segment.mjxStamp),
        shown: bar.shown.map((segment) => segment.label),
        overflow: bar.overflowed.map((segment) => segment.label),
        // Nothing rendered twice: one element per label in the whole subtree.
        labels: [...document.querySelectorAll('mjx-status-segment')].map((segment) =>
          segment.getAttribute('label'),
        ),
      };
    });

    expect(after.total).toBe(stamped);
    expect(after.stamps).toEqual([...Array(stamped).keys()]);
    expect(after.shown.length + after.overflow.length).toBe(stamped);
    expect(new Set(after.labels).size).toBe(stamped);
    // A phone has room for two of them, which is the ticket's own sentence as a number.
    expect(after.shown).toEqual(['Page', 'Zoom']);
  });

  test('the overflow disclosure appears only when it holds something, and says how many', async ({
    page,
  }) => {
    await open(page, ladderStory);
    await resizeFrame(page, containerPresets.desktop);
    const trigger = page.locator('mjx-status-bar >> internal:control=enter-frame >> nth=0');
    // The disclosure lives in the shadow root, so it is read through the element rather than a
    // selector that would have to know the shadow tree's shape.
    const wide = await page.evaluate(() => {
      const bar = document.querySelector('mjx-status-bar') as HTMLElement;
      const button = bar.shadowRoot?.querySelector('.overflow-trigger') as HTMLElement | null;
      return { hidden: button?.hidden ?? null, label: button?.getAttribute('aria-label') ?? null };
    });
    expect(wide.hidden).toBe(true);
    expect(trigger).toBeDefined();

    await resizeFrame(page, containerPresets.phone);
    const narrow = await page.evaluate(() => {
      const bar = document.querySelector('mjx-status-bar') as HTMLElement & {
        overflowed: unknown[];
      };
      const button = bar.shadowRoot?.querySelector('.overflow-trigger') as HTMLElement | null;
      const panel = bar.shadowRoot?.querySelector('.overflow-panel') as HTMLElement | null;
      return {
        hidden: button?.hidden ?? null,
        label: button?.getAttribute('aria-label') ?? null,
        expanded: button?.getAttribute('aria-expanded') ?? null,
        panelHidden: panel?.hidden ?? null,
        count: bar.overflowed.length,
      };
    });
    expect(narrow.hidden).toBe(false);
    expect(narrow.label).toBe(`${String(narrow.count)} more status items`);
    expect(narrow.expanded).toBe('false');
    expect(narrow.panelHidden).toBe(true);

    // Opening it puts the demoted readings on screen, which is what makes "not lost" visible as
    // well as structural.
    const opened = await page.evaluate(async () => {
      const bar = document.querySelector('mjx-status-bar') as HTMLElement & {
        overflowed: HTMLElement[];
      };
      const button = bar.shadowRoot?.querySelector('.overflow-trigger') as HTMLElement | null;
      button?.click();
      await new Promise((done) => requestAnimationFrame(done));
      const panel = bar.shadowRoot?.querySelector('.overflow-panel') as HTMLElement | null;
      return {
        expanded: button?.getAttribute('aria-expanded') ?? null,
        panelHidden: panel?.hidden ?? null,
        visible: bar.overflowed.filter((segment) => segment.getBoundingClientRect().height > 0)
          .length,
        count: bar.overflowed.length,
      };
    });
    expect(opened.expanded).toBe('true');
    expect(opened.panelHidden).toBe(false);
    expect(opened.visible).toBe(opened.count);
    expect(opened.count).toBeGreaterThan(0);
  });

  /**
   * ⚠ **The failure a status bar has is announcing too much**, and this is the only instrument that
   * can see it. The page number is the reading a person watches while they scroll; a screen reader
   * that interrupted them on every change would make the document unusable.
   */
  test('a routine reading is announced into neither region, and a real failure into the alert', async ({
    page,
  }) => {
    await open(page, announceStory);
    const regions = async (): Promise<{ polite: string; alert: string }> =>
      page.evaluate(() => {
        const bar = document.querySelector('mjx-status-bar') as HTMLElement;
        return {
          polite: bar.shadowRoot?.querySelector('[role="status"]')?.textContent ?? '',
          alert: bar.shadowRoot?.querySelector('[role="alert"]')?.textContent ?? '',
        };
      });

    expect(await regions()).toEqual({ polite: '', alert: '' });

    await page.getByRole('button', { name: 'Scroll a page' }).click();
    await settle(page);
    const afterScroll = await regions();
    const pageValue = await page.evaluate(
      () => document.querySelector('#page')?.getAttribute('value') ?? '',
    );
    // The reading really did change — without this the silence below is about nothing.
    expect(pageValue).toBe('5 of 20');
    expect(afterScroll.alert).toBe('');
    expect(afterScroll.polite).toBe('');

    await page.getByRole('button', { name: 'Fail a save' }).click();
    await settle(page);
    const afterFailure = await regions();
    expect(afterFailure.alert).toContain('Could not save');
    expect(afterFailure.polite).toBe('');
  });

  test('the overflow disclosure clears the hit-target floor in compact density', async ({
    page,
  }) => {
    await open(page, compactBarStory, { containerPreset: 'phone' });
    const box = await page.evaluate(() => {
      const bar = document.querySelector('mjx-status-bar') as HTMLElement;
      const button = bar.shadowRoot?.querySelector('.overflow-trigger') as HTMLElement | null;
      const rect = button?.getBoundingClientRect();
      return { width: rect?.width ?? 0, height: rect?.height ?? 0, hidden: button?.hidden ?? true };
    });
    expect(box.hidden).toBe(false);
    expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    expect(box.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });
});

// ── the zoom control ─────────────────────────────────────────────────────────

test.describe('the zoom control is arithmetic with correct answers', () => {
  /** The percentage the control reports, and what its slider and readout say. */
  async function zoomState(page: Page, id = '#zoom'): Promise<{
    percent: number;
    readout: string;
    slider: number;
    thumbFraction: number;
  }> {
    return page.evaluate((selector) => {
      const control = document.querySelector(selector) as never as {
        percent: number;
        readoutElement: HTMLInputElement;
        sliderElement: HTMLElement & { thumbElement: HTMLElement; trackElement: HTMLElement };
      };
      const slider = control.sliderElement;
      const rail = slider.shadowRoot?.querySelector('.rail') as HTMLElement;
      const thumb = slider.thumbElement;
      const railBox = rail.getBoundingClientRect();
      const thumbBox = thumb.getBoundingClientRect();
      return {
        percent: control.percent,
        readout: control.readoutElement.value,
        slider: Number(slider.getAttribute('value')),
        thumbFraction:
          railBox.width === 0 ? -1 : (thumbBox.left + thumbBox.width / 2 - railBox.left) / railBox.width,
      };
    }, id);
  }

  test('a typed percentage and the slider agree, measured against the model', async ({ page }) => {
    await open(page, restingZoomStory);
    const readout = page.locator('#zoom').locator('xpath=.');
    await page.evaluate(() => {
      const control = document.querySelector('#zoom') as never as {
        readoutElement: HTMLInputElement;
      };
      control.readoutElement.focus();
      control.readoutElement.value = '150';
      control.readoutElement.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await page.keyboard.press('Enter');
    await settle(page);
    expect(readout).toBeDefined();

    const typed = await zoomState(page);
    expect(typed.percent).toBe(150);
    expect(typed.readout).toBe('150%');
    expect(typed.slider).toBe(150);
    // The thumb is compared against `sliderFraction()` computed here, never against the custom
    // property the slider itself wrote — U07's rule, because a component that reported its own
    // arithmetic back would be grading its own homework.
    expect(typed.thumbFraction).toBeCloseTo(
      sliderFraction(150, zoomBounds.min, zoomBounds.max),
      2,
    );

    // …and the other way round: drive the slider, read the field.
    await page.evaluate(() => {
      const control = document.querySelector('#zoom') as never as {
        sliderElement: HTMLElement & { trackElement: HTMLElement };
      };
      control.sliderElement.trackElement.focus();
    });
    await page.keyboard.press('ArrowRight');
    await settle(page);
    const nudged = await zoomState(page);
    expect(nudged.percent).toBe(151);
    expect(nudged.readout).toBe('151%');
  });

  test('the stepping commands move between stops, and disable themselves at the bounds', async ({
    page,
  }) => {
    await open(page, restingZoomStory);
    await page.getByRole('button', { name: 'Zoom in' }).click();
    await settle(page);
    expect((await zoomState(page)).percent).toBe(zoomStepFrom(100, 1));

    await page.getByRole('button', { name: 'Zoom out' }).click();
    await settle(page);
    expect((await zoomState(page)).percent).toBe(100);

    await open(page, boundsStory);
    const bounded = await page.evaluate(() =>
      ['#at-minimum', '#at-maximum'].map((selector) => {
        const control = document.querySelector(selector) as HTMLElement & { percent: number };
        const out = control.shadowRoot?.querySelector('mjx-button[label="Zoom out"]');
        const zoomIn = control.shadowRoot?.querySelector('mjx-button[label="Zoom in"]');
        return {
          percent: control.percent,
          outDisabled: out?.hasAttribute('disabled') ?? null,
          inDisabled: zoomIn?.hasAttribute('disabled') ?? null,
        };
      }),
    );
    expect(bounded[0]).toEqual({
      percent: zoomBounds.min,
      outDisabled: true,
      inDisabled: false,
    });
    expect(bounded[1]).toEqual({
      percent: zoomBounds.max,
      outDisabled: false,
      inDisabled: true,
    });
  });

  test('the two fits produce the numbers the model computes', async ({ page }) => {
    await open(page, fitsStory);
    const viewport = { width: 1000, height: 700 };

    await page.getByRole('button', { name: 'Fit width' }).click();
    await settle(page);
    expect((await zoomState(page)).percent).toBe(fitToWidthPercent(viewport));

    await page.getByRole('button', { name: 'Fit page' }).click();
    await settle(page);
    expect((await zoomState(page)).percent).toBe(fitToPagePercent(viewport));

    // A fit already applied is a limit rather than a button that does nothing.
    const disabled = await page.evaluate(() => {
      const control = document.querySelector('#zoom') as HTMLElement;
      const command = control.shadowRoot?.querySelector('mjx-button[data-fit="page"]');
      return command?.hasAttribute('disabled') ?? null;
    });
    expect(disabled).toBe(true);
  });

  /** ⚠ U07's rule, inherited whole: what it cannot read, it refuses — and it never reverts. */
  test('an unparseable percentage is refused three ways, and Escape is the way out', async ({
    page,
  }) => {
    await open(page, refusesStory);
    await page.evaluate(() => {
      const control = document.querySelector('#zoom') as never as {
        readoutElement: HTMLInputElement;
      };
      control.readoutElement.focus();
      control.readoutElement.value = 'banana';
      control.readoutElement.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await page.keyboard.press('Enter');
    await settle(page);

    const refused = await page.evaluate(() => {
      const control = document.querySelector('#zoom') as HTMLElement & { percent: number };
      const entry = control.shadowRoot?.querySelector('.entry') as HTMLInputElement;
      const glyph = control.shadowRoot?.querySelector('.trailing') as HTMLElement;
      const message = control.shadowRoot?.querySelector('.message') as HTMLElement;
      return {
        text: entry.value,
        invalid: entry.getAttribute('aria-invalid'),
        describedBy: entry.getAttribute('aria-describedby'),
        glyphHidden: glyph.hidden,
        glyphDrawn: glyph.getBoundingClientRect().height > 0,
        message: message.textContent ?? '',
        messageDrawn: message.getBoundingClientRect().height > 0,
        percent: control.percent,
        edge: getComputedStyle(
          control.shadowRoot?.querySelector('.field') as HTMLElement,
        ).borderColor,
      };
    });

    // The text stays. A field displaying the old value and a field that committed the old value
    // are the same picture, which is the defect MJXOFF-186 shipped once and caught here.
    expect(refused.text).toBe('banana');
    expect(refused.percent).toBe(100);
    // Three ways at once, because the honey edge is 2.07 : 1 in light and carries nothing alone.
    expect(refused.invalid).toBe('true');
    expect(refused.glyphHidden).toBe(false);
    expect(refused.glyphDrawn).toBe(true);
    expect(refused.message).toContain('banana');
    expect(refused.messageDrawn).toBe(true);
    expect(refused.describedBy).not.toBeNull();

    await page.keyboard.press('Escape');
    await settle(page);
    const escaped = await page.evaluate(() => {
      const control = document.querySelector('#zoom') as HTMLElement & { percent: number };
      const entry = control.shadowRoot?.querySelector('.entry') as HTMLInputElement;
      return {
        text: entry.value,
        invalid: entry.getAttribute('aria-invalid'),
        edge: getComputedStyle(
          control.shadowRoot?.querySelector('.field') as HTMLElement,
        ).borderColor,
      };
    });
    expect(escaped.text).toBe('100%');
    expect(escaped.invalid).toBe('false');
    // ⚠ **The edge measured, not the attribute counted.** `fieldStates.invalid` matches
    // `[data-invalid]` and there is no `data-state` spelling of it, so a component that put the
    // field into a state the table paints nothing for would pass every assertion above with the
    // honey edge missing. What is asserted is a *difference* between two measured colours rather
    // than a remembered hex — the palette re-seed is why.
    expect(refused.edge).not.toBe(escaped.edge);
  });

  test('out of range is clamped rather than refused, in both directions', async ({ page }) => {
    await open(page, refusesStory);
    for (const [typed, wanted] of [
      ['700', zoomBounds.max],
      ['1', zoomBounds.min],
    ] as const) {
      await page.evaluate((text) => {
        const control = document.querySelector('#zoom') as never as {
          readoutElement: HTMLInputElement;
        };
        control.readoutElement.focus();
        control.readoutElement.value = text;
        control.readoutElement.dispatchEvent(new Event('input', { bubbles: true }));
      }, typed);
      await page.keyboard.press('Enter');
      await settle(page);
      const state = await zoomState(page);
      expect(state.percent, `typing ${typed}`).toBe(wanted);
      expect(state.readout).toBe(`${String(wanted)}%`);
    }
  });

  test('the stepping commands clear the hit-target floor in compact density', async ({ page }) => {
    await open(page, compactZoomStory);
    const boxes = await page.evaluate(() => {
      const control = document.querySelector('#zoom') as HTMLElement;
      return ['Zoom out', 'Zoom in'].map((label) => {
        const command = control.shadowRoot?.querySelector(`mjx-button[label="${label}"]`);
        const button = command?.shadowRoot?.querySelector('button') as HTMLElement | null;
        const rect = button?.getBoundingClientRect();
        return { label, width: rect?.width ?? 0, height: rect?.height ?? 0 };
      });
    });
    for (const box of boxes) {
      expect(box.width, box.label).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
      expect(box.height, box.label).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    }
  });
});

// ── the scrollbar ────────────────────────────────────────────────────────────

test.describe('the scrollbar over an extent that is only an estimate', () => {
  test('the thumb is the visible share of the document, at four ratios and all distinct', async ({
    page,
  }) => {
    await open(page, ratiosStory);
    const measured = await page.evaluate(() =>
      ['fits', 'barely', 'five', 'huge'].map((id) => {
        const bar = document.querySelector(`#${id}`) as never as {
          trackElement: HTMLElement;
          thumbElement: HTMLElement;
          tabIndex: number;
        };
        const track = bar.trackElement.getBoundingClientRect();
        const thumb = bar.thumbElement.getBoundingClientRect();
        const style = getComputedStyle(bar.thumbElement);
        return {
          id,
          fraction: track.height === 0 ? -1 : thumb.height / track.height,
          visible: style.visibility !== 'hidden',
          tabIndex: bar.tabIndex,
        };
      }),
    );

    // Computed here from the same extents the story declares — never read back from the component.
    const expected = [
      thumbFraction(800, 1 * 800),
      thumbFraction(800, 9 * 100),
      thumbFraction(800, 5 * 800),
      thumbFraction(800, 500 * 800),
    ];
    measured.forEach((row, index) => {
      expect(row.fraction, row.id).toBeCloseTo(expected[index] ?? -1, 2);
    });

    // ⚠ The identity-value trap: content that fits is not a scrollbar. A full-length thumb that
    // cannot move looks exactly like a working one, so it is hidden and out of the tab order.
    expect(measured[0]?.visible).toBe(false);
    expect(measured[0]?.tabIndex).toBe(-1);
    for (const row of measured.slice(1)) {
      expect(row.visible, row.id).toBe(true);
      expect(row.tabIndex, row.id).toBe(0);
    }
    // The three that scroll are three different sizes, which is what says the arithmetic ran.
    const scrolling = measured.slice(1).map((row) => Number(row.fraction.toFixed(3)));
    expect(new Set(scrolling).size).toBe(scrolling.length);
  });

  /**
   * ⚠ **THE TRAP.** The pointer is really held down through the browser's own input path while the
   * extent is revised underneath it. A synthetic event sequence would prove a handler exists and
   * nothing about whether the platform would ever call it, and a story with a fixed content height
   * would prove nothing at all.
   */
  test('the thumb does not jump when the extent changes mid-drag', async ({ page }) => {
    // ⚠ A tablet-width frame, deliberately. At the desktop preset the harness frame is 1440 wide
    // inside a 1280 viewport and scrolls horizontally, so the scrollbar sits at x = 1400 — off
    // screen, where Playwright's real mouse cannot reach it and every pointer event is delivered
    // to nothing. The first version of this test passed its way to a silent zero.
    await open(page, correctedStory, { containerPreset: 'tablet' });

    const geometry = await page.evaluate(() => {
      const bar = document.querySelector('#corrected') as never as {
        trackElement: HTMLElement;
        thumbElement: HTMLElement;
        viewport: number;
        model: { totalHeight: number };
      };
      const track = bar.trackElement.getBoundingClientRect();
      const thumb = bar.thumbElement.getBoundingClientRect();
      return {
        track: { top: track.top, left: track.left, height: track.height, width: track.width },
        thumb: { top: thumb.top, height: thumb.height },
        viewport: bar.viewport,
        total: bar.model.totalHeight,
      };
    });
    expect(geometry.total).toBe(20_000);
    expect(geometry.track.height).toBeGreaterThan(100);

    const centreX = geometry.track.left + geometry.track.width / 2;
    // Grip the thumb at its own centre, so the point that must not move is the thumb's centre.
    await page.mouse.move(centreX, geometry.thumb.top + geometry.thumb.height / 2);
    await page.mouse.down();
    // …and drag it to the middle of the track.
    await page.mouse.move(centreX, geometry.track.top + geometry.track.height / 2, { steps: 8 });
    await settle(page);

    const before = await page.evaluate(() => {
      const bar = document.querySelector('#corrected') as never as {
        trackElement: HTMLElement;
        thumbElement: HTMLElement;
        offset: number;
        model: { totalHeight: number; measuredPages: number };
      };
      const track = bar.trackElement.getBoundingClientRect();
      const thumb = bar.thumbElement.getBoundingClientRect();
      return {
        centre: thumb.top + thumb.height / 2 - track.top,
        height: thumb.height,
        offset: bar.offset,
        total: bar.model.totalHeight,
        measured: bar.model.measuredPages,
      };
    });
    expect(before.measured).toBe(0);
    expect(before.offset).toBeGreaterThan(0);

    // The extent is revised while the pointer is still down — exactly what R13 does as pages are
    // laid out one at a time.
    await page.evaluate(() => {
      const bar = document.querySelector('#corrected') as never as {
        recordMeasuredHeight: (page: number, height: number) => void;
      };
      for (let index = 0; index < 10; index += 1) bar.recordMeasuredHeight(index, 1400);
    });
    await settle(page);

    const after = await page.evaluate(() => {
      const bar = document.querySelector('#corrected') as never as {
        trackElement: HTMLElement;
        thumbElement: HTMLElement;
        offset: number;
        model: { totalHeight: number; measuredPages: number };
      };
      const track = bar.trackElement.getBoundingClientRect();
      const thumb = bar.thumbElement.getBoundingClientRect();
      return {
        centre: thumb.top + thumb.height / 2 - track.top,
        height: thumb.height,
        offset: bar.offset,
        total: bar.model.totalHeight,
        measured: bar.model.measuredPages,
      };
    });
    await page.mouse.up();

    // The correction actually happened — R13's own anti-vacuity figure, and without these three
    // the assertion below is green for a component that ignores every correction.
    expect(after.measured).toBe(10);
    expect(after.total).toBe(24_000);
    expect(after.height).toBeLessThan(before.height - 1);
    expect(after.offset).toBeGreaterThan(before.offset);

    // THE ASSERTION: the point of the thumb under the finger has not moved.
    expect(after.centre).toBeCloseTo(before.centre, 0);

    // ⚠ THE POSITIVE CONTROL. What the naive recomputation — keep the offset, recompute the thumb
    // from it — would have put on screen, computed here from the model. If the two agreed, the
    // assertion above would be a tolerance nobody had tested.
    const naiveCentre =
      thumbStart(before.offset, geometry.viewport, after.total) * geometry.track.height +
      (thumbFraction(geometry.viewport, after.total) * geometry.track.height) / 2;
    expect(Math.abs(naiveCentre - after.centre)).toBeGreaterThan(10);
  });

  test('the marks sit at their proportional positions, in their own lanes', async ({ page }) => {
    await open(page, channelStory);
    const marks: ScrollMark[] = [
      { kind: 'search', page: 1, within: 0.25, label: 'pipeline' },
      { kind: 'search', page: 6, within: 0.5, label: 'pipeline' },
      { kind: 'search', page: 17, within: 0.1, label: 'pipeline' },
      { kind: 'comment', page: 4, within: 0, label: 'Ask legal' },
      { kind: 'comment', page: 12, within: 0.8, label: 'Reword' },
      { kind: 'change', page: 9, within: 0.4, label: 'Inserted' },
    ];
    const model = ScrollbarModel.fromExtent({
      pages: 20,
      pageHeight: 1000,
      precision: 'estimated',
    });

    const measured = await page.evaluate(() => {
      const bar = document.querySelector('#marked') as never as { trackElement: HTMLElement };
      const host = bar as never as HTMLElement;
      const track = host.shadowRoot?.querySelector('.track') as HTMLElement;
      const box = track.getBoundingClientRect();
      return [...(host.shadowRoot?.querySelectorAll('.mark') ?? [])].map((mark) => {
        const rect = mark.getBoundingClientRect();
        return {
          kind: (mark as HTMLElement).dataset['kind'] ?? '',
          top: rect.top - box.top,
          left: rect.left - box.left,
          width: rect.width,
          trackHeight: box.height,
          trackWidth: box.width,
        };
      });
    });

    expect(measured.length).toBe(marks.length);
    // Distinct first: six marks all at zero would satisfy every "is it where the model says"
    // assertion below if the model also said zero.
    expect(new Set(measured.map((mark) => Math.round(mark.top))).size).toBeGreaterThan(4);

    measured.forEach((mark, index) => {
      const wanted = markFraction(model, marks[index] as ScrollMark);
      expect(mark.top, `${mark.kind} ${String(index)}`).toBeCloseTo(
        wanted * mark.trackHeight,
        0,
      );
      // Its own lane, which is the non-colour half of telling three kinds apart.
      const lane = scrollMarkSpecs[(marks[index] as ScrollMark).kind].lane;
      expect(mark.left, `${mark.kind} lane`).toBeCloseTo((mark.trackWidth * lane) / 3, 0);
    });

    // The three kinds occupy three different columns.
    const lanes = new Set(measured.map((mark) => Math.round(mark.left)));
    expect(lanes.size).toBe(3);
  });

  test('the keyboard moves it, and Home and End are the ends of the document', async ({ page }) => {
    await open(page, correctedStory);
    await page.evaluate(() => {
      (document.querySelector('#corrected') as HTMLElement).focus();
    });
    const read = async (): Promise<{ offset: number; valuenow: string | null; text: string | null }> =>
      page.evaluate(() => {
        const bar = document.querySelector('#corrected') as HTMLElement & { offset: number };
        return {
          offset: bar.offset,
          valuenow: bar.getAttribute('aria-valuenow'),
          text: bar.getAttribute('aria-valuetext'),
        };
      });

    expect((await read()).offset).toBe(0);
    await page.keyboard.press('End');
    await settle(page);
    const end = await read();
    // 20 pages of 1000, less the 4000 the viewport already sees.
    expect(end.offset).toBe(16_000);
    expect(end.valuenow).toBe('100');
    expect(end.text).toBe('Page 17 of 20 (estimated)');

    await page.keyboard.press('Home');
    await settle(page);
    expect((await read()).offset).toBe(0);

    await page.keyboard.press('PageDown');
    await settle(page);
    expect((await read()).offset).toBe(4000);

    await page.keyboard.press('ArrowDown');
    await settle(page);
    expect((await read()).offset).toBe(4400);
  });

  test('the bar clears the hit-target floor in compact density', async ({ page }) => {
    await open(page, compactBarsStory);
    const width = await page.evaluate(
      () => (document.querySelector('#compact') as HTMLElement).getBoundingClientRect().width,
    );
    expect(width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });
});

// ── the splitter ─────────────────────────────────────────────────────────────

test.describe('the splitter, which a keyboard can move', () => {
  async function splitState(
    page: Page,
    id: string,
  ): Promise<{ valuenow: number; valuetext: string; fraction: number; navigator: number }> {
    return page.evaluate((selector) => {
      const element = document.querySelector(selector) as HTMLElement & {
        handleElement: HTMLElement;
        fraction: number;
      };
      const handle = element.handleElement;
      const region = document.querySelector(`${selector}-navigator`);
      return {
        valuenow: Number(handle.getAttribute('aria-valuenow')),
        valuetext: handle.getAttribute('aria-valuetext') ?? '',
        fraction: element.fraction,
        navigator: region?.getBoundingClientRect().width ?? -1,
      };
    }, id);
  }

  test('arrows, Home and End all move it, and the region follows', async ({ page }) => {
    await open(page, navigatorStory);
    await page.keyboard.press('Tab');
    // The frame's own stage takes the first stop; walk on until the separator has the keyboard.
    for (let attempt = 0; attempt < 8; attempt += 1) {
      const onSeparator = await page.evaluate(() => {
        let node: Element | null = document.activeElement;
        while (node?.shadowRoot?.activeElement != null) node = node.shadowRoot.activeElement;
        return node?.getAttribute('role') === 'separator';
      });
      if (onSeparator) break;
      await page.keyboard.press('Tab');
    }
    const focused = await page.evaluate(() => {
      let node: Element | null = document.activeElement;
      while (node?.shadowRoot?.activeElement != null) node = node.shadowRoot.activeElement;
      return node?.getAttribute('role') ?? '';
    });
    expect(focused).toBe('separator');

    const start = await splitState(page, '#navigator');
    expect(start.valuenow).toBe(30);

    await page.keyboard.press('ArrowRight');
    await settle(page);
    const grown = await splitState(page, '#navigator');
    // The leading region is at the start of the line, so the arrow pointing away from it grows it.
    expect(grown.valuenow).toBe(32);
    expect(grown.navigator).toBeGreaterThan(start.navigator);

    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('ArrowLeft');
    await settle(page);
    expect((await splitState(page, '#navigator')).valuenow).toBe(28);

    await page.keyboard.press('Home');
    await settle(page);
    expect((await splitState(page, '#navigator')).valuenow).toBe(15);

    await page.keyboard.press('End');
    await settle(page);
    // `min-end` is 0.3, so the widest the leading region may be is seventy per cent.
    expect((await splitState(page, '#navigator')).valuenow).toBe(70);
  });

  test('each side keeps its own minimum', async ({ page }) => {
    await open(page, minimumsStory);
    await page.evaluate(() => {
      (document.querySelector('#bounded') as HTMLElement).focus();
    });
    await page.keyboard.press('End');
    await settle(page);
    expect((await splitState(page, '#bounded')).valuenow).toBe(50);
    await page.keyboard.press('Home');
    await settle(page);
    expect((await splitState(page, '#bounded')).valuenow).toBe(20);
  });

  test('a double-click collapses, and a second one restores where it was — not a default', async ({
    page,
  }) => {
    await open(page, navigatorStory);
    await page.evaluate(() => {
      (document.querySelector('#navigator') as HTMLElement).focus();
    });
    // Move it somewhere that is provably not the declared fraction, so a "restore" that reset to
    // the default would be caught.
    for (let press = 0; press < 6; press += 1) await page.keyboard.press('ArrowRight');
    await settle(page);
    const moved = await splitState(page, '#navigator');
    expect(moved.valuenow).toBe(42);

    await page.locator('#navigator').dblclick();
    await settle(page);
    const collapsed = await splitState(page, '#navigator');
    expect(collapsed.fraction).toBe(0);
    expect(collapsed.valuenow).toBe(0);
    expect(collapsed.valuetext).toBe('Collapsed');
    expect(collapsed.navigator).toBeLessThan(2);

    await page.locator('#navigator').dblclick();
    await settle(page);
    const restored = await splitState(page, '#navigator');
    expect(restored.valuenow).toBe(moved.valuenow);
    expect(restored.navigator).toBeCloseTo(moved.navigator, 0);
  });

  test('Enter collapses and restores it too, because a pointer gesture is not an interface', async ({
    page,
  }) => {
    await open(page, navigatorStory);
    await page.evaluate(() => {
      (document.querySelector('#navigator') as HTMLElement).focus();
    });
    await page.keyboard.press('Enter');
    await settle(page);
    expect((await splitState(page, '#navigator')).valuetext).toBe('Collapsed');
    await page.keyboard.press('Enter');
    await settle(page);
    expect((await splitState(page, '#navigator')).valuenow).toBe(30);
  });

  test('the position survives a remount', async ({ page }) => {
    await open(page, rememberedStory);
    await page.evaluate(() => {
      const view = window as unknown as { localStorage: Storage };
      view.localStorage.clear();
      (document.querySelector('#remembered') as HTMLElement).focus();
    });
    for (let press = 0; press < 4; press += 1) await page.keyboard.press('ArrowRight');
    await settle(page);
    const moved = await splitState(page, '#remembered');
    expect(moved.valuenow).toBe(38);

    await page.getByRole('button', { name: 'Remount the splitter' }).click();
    await settle(page);
    await settle(page);

    const remounted = await page.evaluate(() => {
      const element = document.querySelector('#remembered') as HTMLElement & {
        handleElement: HTMLElement;
      };
      return {
        declared: element.getAttribute('fraction'),
        valuenow: Number(element.handleElement.getAttribute('aria-valuenow')),
      };
    });
    // The replacement carries no `fraction` at all, so what it came back at came back from storage.
    expect(remounted.declared).toBeNull();
    expect(remounted.valuenow).toBe(moved.valuenow);
  });

  test('the handle clears the hit-target floor in compact density', async ({ page }) => {
    await open(page, compactSplitStory);
    const box = await page.evaluate(() => {
      const element = document.querySelector('#compact') as HTMLElement & {
        handleElement: HTMLElement;
      };
      const rect = element.handleElement.getBoundingClientRect();
      return { width: rect.width, height: rect.height };
    });
    expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    expect(box.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });
});
