import { expect, test, type Locator, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import {
  feedbackStoryTitles,
  feedbackTimingMilliseconds,
  indeterminateSpanFraction,
  toastStackCeiling,
} from '../../src/feedback/feedback-model.ts';

/**
 * The feedback family, driven in a real browser.
 *
 * **Every component in MJXOFF-189 is invisible in a still**, and this suite is arranged entirely
 * around that. What is asserted here is what a screenshot cannot hold:
 *
 * * **that a delay is a delay.** *"It appeared"* is satisfied by a tip that appears instantly, so
 *   the screentip is sampled **before** its delay as well as after it, and the delay the component
 *   resolved is compared against the token arithmetic — because a broken `@property` registration
 *   would make the other two assertions unfalsifiable rather than false;
 * * **that a toast is announced to the right ear.** The assertive region is asserted **empty** for a
 *   polite toast, which is the one claim a single-region implementation cannot satisfy;
 * * **that a stack of timers is a stack of timers.** Two toasts pushed half a dwell apart leave half
 *   a dwell apart;
 * * **that an indeterminate bar is not a stalled one.** Its indicator is sampled at two instants,
 *   beside a determinate bar parked at *exactly the same width*, and then the whole thing is done
 *   again under a reduced-motion preference where neither of them moves and only the accessibility
 *   tree is left to tell them apart;
 * * **that an empty state's button does something.** It is pressed, and the list it was supposed to
 *   fill is counted.
 *
 * ## The lesson this suite is written under
 *
 * Every ceiling has an anti-vacuity assertion beside it, and every *"never"* has a story where it
 * happens. The mini toolbar's *"it never covers the selection"* is checked on three stories that
 * clear it **and** on one that cannot, so the gate is watching a property rather than an
 * arrangement.
 */

const miniToolbar = feedbackStoryTitles.miniToolbar;
const screentip = feedbackStoryTitles.screentip;
const toast = feedbackStoryTitles.toast;
const progress = feedbackStoryTitles.progress;
const emptyState = feedbackStoryTitles.emptyState;

const clearStory = { title: miniToolbar, name: 'Above The Selection' } as const;
const flippedStory = { title: miniToolbar, name: 'Flipped Below The Selection' } as const;
const coveringStory = { title: miniToolbar, name: 'A Selection With No Room' } as const;
const compactToolbarStory = { title: miniToolbar, name: 'In Compact Density' } as const;

const waitingStory = { title: screentip, name: 'Waits For A Pointer' } as const;
const rowStory = { title: screentip, name: 'A Row Of Commands' } as const;
const flippedTipStory = { title: screentip, name: 'At The Bottom Of Its Room' } as const;

const tonesStory = { title: toast, name: 'The Four Tones' } as const;
const queueStory = { title: toast, name: 'A Queue Over Time' } as const;
const compactToastStory = { title: toast, name: 'In Compact Density' } as const;

const ladderStory = { title: progress, name: 'The Ladder' } as const;
const countableStory = { title: progress, name: 'Countable Units' } as const;
const stalledStory = { title: progress, name: 'Told Apart From A Stalled Bar' } as const;

const actionStory = { title: emptyState, name: 'The Action Does The Thing' } as const;
const nothingToOfferStory = { title: emptyState, name: 'Nothing To Offer' } as const;

/** The dwell the queue story overrides its region to, so the audit does not take a minute. */
const storyDwell = 900;

async function open(
  page: Page,
  story: { readonly title: string; readonly name: string },
  options: { theme?: string; containerPreset?: string } = {},
): Promise<void> {
  const entry = builtStories().find(
    (candidate) => candidate.title === story.title && candidate.name === story.name,
  );
  expect(entry, `${story.title} · ${story.name} is missing from the catalogue`).toBeDefined();
  if (entry === undefined) return;
  await openStory(page, entry.id, options);
  for (const tag of [
    'mjx-mini-toolbar',
    'mjx-screentip',
    'mjx-toast-region',
    'mjx-progress',
    'mjx-empty-state',
  ]) {
    await page.waitForFunction((name) => customElements.get(name) !== undefined, tag);
  }
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

interface Box {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
}

/** A rectangle, or a failure. Never a stand-in: an absent box makes every geometry check pass. */
async function boxOf(locator: Locator, what: string): Promise<Box> {
  const box = await locator.boundingBox();
  expect(box, `${what} has no box at all, so nothing below is measuring anything`).not.toBeNull();
  if (box === null) throw new Error(`${what} has no box`);
  expect(box.width, `${what} has no width`).toBeGreaterThan(0);
  expect(box.height, `${what} has no height`).toBeGreaterThan(0);
  return box;
}

function overlapArea(a: Box, b: Box): number {
  const x = Math.max(a.x, b.x);
  const y = Math.max(a.y, b.y);
  const right = Math.min(a.x + a.width, b.x + b.width);
  const bottom = Math.min(a.y + a.height, b.y + b.height);
  return Math.max(right - x, 0) * Math.max(bottom - y, 0);
}

/** The active element, followed through every shadow root it is hiding in. */
async function deepActive(page: Page): Promise<string> {
  return page.evaluate(() => {
    let element: Element | null = document.activeElement;
    while (element?.shadowRoot?.activeElement != null) element = element.shadowRoot.activeElement;
    if (element === null) return '';
    return element.id !== ''
      ? `#${element.id}`
      : `${element.localName}:${element.getAttribute('aria-label') ?? element.textContent?.trim() ?? ''}`;
  });
}

// ── the mini toolbar ─────────────────────────────────────────────────────────

test.describe('the mini toolbar never covers the selection it serves', () => {
  test('above it, when there is room above', async ({ page }) => {
    await open(page, clearStory);
    const selection = await boxOf(page.locator('#selection-clear'), 'the selection');
    const toolbarBox = await boxOf(
      page.locator('#toolbar-clear').locator('css=div.toolbar'),
      'the toolbar',
    );
    expect(overlapArea(toolbarBox, selection)).toBe(0);
    // Above, and near it: a toolbar the other side of the page clears the selection too.
    expect(toolbarBox.y + toolbarBox.height).toBeLessThanOrEqual(selection.y + 1);
    expect(selection.y - (toolbarBox.y + toolbarBox.height)).toBeLessThan(selection.height * 4);
    await expect(page.locator('#toolbar-clear')).not.toHaveAttribute('data-covering', 'true');
  });

  test('below it, when the selection is at the top of its room', async ({ page }) => {
    await open(page, flippedStory);
    const selection = await boxOf(page.locator('#selection-top'), 'the selection');
    const toolbarBox = await boxOf(
      page.locator('#toolbar-flipped').locator('css=div.toolbar'),
      'the toolbar',
    );
    expect(overlapArea(toolbarBox, selection)).toBe(0);
    expect(toolbarBox.y).toBeGreaterThanOrEqual(selection.y + selection.height - 1);
    await expect(page.locator('#toolbar-flipped')).not.toHaveAttribute('data-covering', 'true');
  });

  /**
   * ⚠ **The failure branch, watched happening.**
   *
   * The two assertions above are satisfied by a component that could never report a covering
   * placement at all. This is the story where no clear side exists, and the requirement here is the
   * *opposite* one: the flag is set, and the rectangles really do overlap. Without it, *"it never
   * covers the selection"* would be a claim about three arrangements rather than about a property.
   */
  test('and says so, on the host, when the selection leaves nowhere to go', async ({ page }) => {
    await open(page, coveringStory);
    await expect(page.locator('#toolbar-covering')).toHaveAttribute('data-covering', 'true');
    const selection = await boxOf(page.locator('#selection-everywhere'), 'the selection');
    const toolbarBox = await boxOf(
      page.locator('#toolbar-covering').locator('css=div.toolbar'),
      'the toolbar',
    );
    expect(
      overlapArea(toolbarBox, selection),
      'the host claims it is covering the selection and the rectangles say it is not',
    ).toBeGreaterThan(0);
  });
});

test.describe('the mini toolbar is a toolbar', () => {
  test('announces itself as one, with a name and a command per row', async ({ page }) => {
    await open(page, clearStory);
    const toolbarBox = page.locator('#toolbar-clear').locator('css=div.toolbar');
    await expect(toolbarBox).toHaveAttribute('role', 'toolbar');
    await expect(toolbarBox).toHaveAttribute('aria-label', 'Formatting');
    const commands = toolbarBox.locator('css=button.command');
    await expect(commands).toHaveCount(7);
    await expect(commands.nth(0)).toHaveAttribute('aria-pressed', 'true');
    await expect(commands.nth(1)).toHaveAttribute('aria-pressed', 'false');
    // The unavailable one stays reachable and keeps its explanation, which is the whole difference
    // between `unavailable` and `disabled` in this catalogue.
    await expect(commands.nth(6)).toHaveAttribute('aria-disabled', 'true');
    await expect(commands.nth(6)).not.toHaveAttribute('disabled', '');
  });

  /**
   * One tab stop, proved by pressing real keys.
   *
   * The walk in `overlay/modality.ts` is never trusted on its own — U07's rule — so the count here
   * comes from the keyboard: Tab lands on exactly one command, and the *next* Tab has left the
   * toolbar entirely.
   */
  test('holds one tab stop, and the arrows do the moving', async ({ page }) => {
    await open(page, clearStory);
    await page.locator('#editor').focus();
    expect(await deepActive(page)).toBe('#editor');

    let landed = '';
    for (let press = 0; press < 6; press += 1) {
      await page.keyboard.press('Tab');
      landed = await deepActive(page);
      if (landed.startsWith('button:')) break;
    }
    expect(landed, 'Tab never reached the toolbar from the editor').toBe('button:Bold');

    await page.keyboard.press('ArrowRight');
    expect(await deepActive(page)).toBe('button:Italic');
    await page.keyboard.press('End');
    expect(await deepActive(page)).toBe('button:Delete');
    await page.keyboard.press('Home');
    expect(await deepActive(page)).toBe('button:Bold');

    // …and Tab leaves rather than moving to the second command.
    await page.keyboard.press('Tab');
    expect(await deepActive(page)).not.toBe('button:Italic');
  });

  test('Escape gives the keyboard back to where it came from', async ({ page }) => {
    await open(page, clearStory);
    await page.locator('#editor').focus();
    for (let press = 0; press < 6; press += 1) {
      await page.keyboard.press('Tab');
      if ((await deepActive(page)).startsWith('button:')) break;
    }
    expect(await deepActive(page)).toBe('button:Bold');
    await page.keyboard.press('Escape');
    await settle(page);
    await expect(page.locator('#toolbar-clear')).not.toHaveAttribute('open', '');
    expect(await deepActive(page)).toBe('#editor');
  });

  /**
   * ⚠ **The commands actually do something**, which is the assertion a states matrix cannot make.
   * The readout in the story is written by the story's own listener, so what is checked is that the
   * event crossed the shadow boundary and carried the right command — not that a handler ran.
   */
  test('a command fires, and a toggle reports which way it went', async ({ page }) => {
    await open(page, clearStory);
    const commands = page.locator('#toolbar-clear').locator('css=button.command');
    await expect(page.locator('#last-command')).toHaveText('none yet');

    await commands.nth(0).click();
    await expect(page.locator('#last-command')).toHaveText('bold: false');
    await expect(commands.nth(0)).toHaveAttribute('aria-pressed', 'false');

    await commands.nth(0).click();
    await expect(page.locator('#last-command')).toHaveText('bold: true');
    await expect(commands.nth(0)).toHaveAttribute('aria-pressed', 'true');

    // A one-shot command carries no pressed state at all.
    await commands.nth(4).click();
    await expect(page.locator('#last-command')).toHaveText('align-center');

    /*
     * …and an unavailable one refuses.
     *
     * ⚠ **`force: true`, and the reason is itself an assertion.** Playwright's own actionability
     * check refuses to click an element carrying `aria-disabled="true"` — *"element is not
     * enabled"* — which is a second, independent instrument saying the announcement is right. So
     * the plain click is asserted to be *refused by the harness*, and then the component's own
     * refusal is proved by forcing past it: without the force, this test would pass on a component
     * that fires happily and would only ever have been measuring Playwright.
     */
    await expect(commands.nth(6)).toBeDisabled();
    await commands.nth(6).click({ force: true });
    await expect(page.locator('#last-command')).toHaveText('align-center');
  });

  test('the hit-target floor holds in compact density', async ({ page }) => {
    await open(page, compactToolbarStory);
    const commands = page.locator('#toolbar-compact').locator('css=button.command');
    const count = await commands.count();
    expect(count, 'no commands to measure').toBeGreaterThan(0);
    for (let index = 0; index < count; index += 1) {
      const box = await boxOf(commands.nth(index), `command ${String(index)}`);
      expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
      expect(box.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    }
  });
});

// ── the screentip ────────────────────────────────────────────────────────────

test.describe('a screentip waits', () => {
  /**
   * ⚠ **The assertion that keeps the other two honest.**
   *
   * If the `@property` registration were missing, `resolveDurationMilliseconds` would report that it
   * cannot read a time, every component would fall back to its generated default, and *"it appeared
   * after the delay"* would still pass while a host's override reached nothing. So the number the
   * component resolved is compared against the token arithmetic by name.
   */
  test('resolves its delay from the cascade, and it is the token arithmetic', async ({ page }) => {
    await open(page, waitingStory);
    const resolved = await page.evaluate(() => {
      const tip = document.querySelector('#tip-bold') as
        | (HTMLElement & { schedule: { appear: number; warm: number } })
        | null;
      return tip === null ? null : tip.schedule;
    });
    expect(resolved).not.toBeNull();
    expect(resolved?.appear).toBe(feedbackTimingMilliseconds('screentipAppear'));
    expect(resolved?.warm).toBe(feedbackTimingMilliseconds('screentipWarm'));
    expect(resolved?.appear).toBeGreaterThan(0);
  });

  /** Sampled in the middle of the delay, not only at the end of it. */
  test('is not there before the delay, and is there after it', async ({ page }) => {
    await open(page, waitingStory);
    const appear = feedbackTimingMilliseconds('screentipAppear');
    const tip = page.locator('#tip-bold').locator('css=div.tip');
    await expect(tip).toBeHidden();

    const trigger = await boxOf(page.locator('#tip-bold-trigger'), 'the trigger');
    await page.mouse.move(trigger.x + trigger.width / 2, trigger.y + trigger.height / 2);

    // Half way through. A tip that appeared instantly is caught exactly here and nowhere else.
    await page.waitForTimeout(appear * 0.4);
    await expect(
      tip,
      'the tip was drawn before its delay had elapsed, which is a delay that is not one',
    ).toBeHidden();

    await page.waitForTimeout(appear * 0.6 + 200);
    await expect(tip).toBeVisible();
    await expect(tip).toContainText('Bold');
    await expect(tip).toContainText('Ctrl + B');
  });

  /**
   * The warm-up: the *second* trigger in a row does not wait.
   *
   * Sampled well inside the delay, so a component that had simply forgotten to warm up would still
   * be waiting when this assertion runs.
   */
  test('the next tip in a row appears with no wait at all', async ({ page }) => {
    await open(page, rowStory);
    const appear = feedbackTimingMilliseconds('screentipAppear');
    const first = await boxOf(page.locator('#row-one-trigger'), 'the first trigger');
    await page.mouse.move(first.x + first.width / 2, first.y + first.height / 2);
    await page.waitForTimeout(appear + 200);
    await expect(page.locator('#row-one').locator('css=div.tip')).toBeVisible();

    const second = await boxOf(page.locator('#row-two-trigger'), 'the second trigger');
    await page.mouse.move(second.x + second.width / 2, second.y + second.height / 2);
    await page.waitForTimeout(appear * 0.3);
    await expect(
      page.locator('#row-two').locator('css=div.tip'),
      'the second tip in a row waited the full delay, so the warm-up window is doing nothing',
    ).toBeVisible();
  });

  test('Escape removes it and moves no focus at all', async ({ page }) => {
    await open(page, waitingStory);
    const appear = feedbackTimingMilliseconds('screentipAppear');
    await page.locator('#tip-plain-trigger').focus();
    const before = await deepActive(page);
    expect(before).toBe('#tip-plain-trigger');

    const trigger = await boxOf(page.locator('#tip-bold-trigger'), 'the trigger');
    await page.mouse.move(trigger.x + trigger.width / 2, trigger.y + trigger.height / 2);
    await page.waitForTimeout(appear + 200);
    const tip = page.locator('#tip-bold').locator('css=div.tip');
    await expect(tip).toBeVisible();

    await page.keyboard.press('Escape');
    await settle(page);
    await expect(tip).toBeHidden();
    expect(
      await deepActive(page),
      'Escape on a screentip moved the keyboard, which it never may: the tip was shown by a pointer',
    ).toBe(before);
  });

  /**
   * ⚠ **The description is announced whether or not the tip is drawn**, which is what makes the
   * delay a thing for eyes rather than an accessibility gate. Read at load time, before any pointer
   * has been near it.
   */
  test('the trigger carries the tip’s text as its accessible description, from the start', async ({
    page,
  }) => {
    await open(page, waitingStory);
    const described = await page.evaluate(() => {
      const trigger = document.querySelector('#tip-bold-trigger');
      const id = trigger?.getAttribute('aria-describedby') ?? '';
      const target = id === '' ? null : document.getElementById(id);
      return { id, text: target?.textContent ?? '', drawn: target?.isConnected ?? false };
    });
    expect(described.id).not.toBe('');
    expect(described.drawn).toBe(true);
    expect(described.text).toContain('Bold');
    expect(described.text).toContain('Make the selected text bold');
    // …and the drawn box is not announced a second time.
    await expect(page.locator('#tip-bold').locator('css=div.tip')).toHaveAttribute(
      'aria-hidden',
      'true',
    );
  });

  test('flips above its trigger, and still does not cover it', async ({ page }) => {
    await open(page, flippedTipStory);
    const trigger = await boxOf(page.locator('#tip-flipped-trigger'), 'the trigger');
    const tip = await boxOf(page.locator('#tip-flipped').locator('css=div.tip'), 'the tip');
    expect(overlapArea(tip, trigger)).toBe(0);
    expect(tip.y + tip.height).toBeLessThanOrEqual(trigger.y + 1);
  });
});

// ── the toast ────────────────────────────────────────────────────────────────

/** Read both announcers at once, so *"one has it and the other does not"* is a single sample. */
async function announcements(page: Page, regionId: string): Promise<{ polite: string; assertive: string }> {
  return page.evaluate((id) => {
    const region = document.querySelector(`#${id}`);
    const root = region?.shadowRoot ?? null;
    const read = (politeness: string): string =>
      root?.querySelector(`[aria-live="${politeness}"]`)?.textContent ?? '';
    return { polite: read('polite'), assertive: read('assertive') };
  }, regionId);
}

test.describe('a toast is announced to exactly one ear', () => {
  test('the two regions exist, never change their politeness, and carry the two roles', async ({
    page,
  }) => {
    await open(page, tonesStory);
    const regions = await page.evaluate(() => {
      const root = document.querySelector('#tones')?.shadowRoot ?? null;
      return [...(root?.querySelectorAll('[aria-live]') ?? [])].map((element) => ({
        live: element.getAttribute('aria-live') ?? '',
        role: element.getAttribute('role') ?? '',
      }));
    });
    expect(regions).toHaveLength(2);
    expect(regions).toEqual(
      expect.arrayContaining([
        { live: 'polite', role: 'status' },
        { live: 'assertive', role: 'alert' },
      ]),
    );
  });

  /**
   * ⚠ **The assertion MJXOFF-189 calls the whole accessibility contract of a toast.**
   *
   * Both halves are needed and only one of them is obvious: the polite region must have the message
   * (otherwise nothing was announced at all, and *"the assertive region is empty"* is satisfied by a
   * component that announces nothing), and the assertive one must be empty.
   */
  test('a polite toast makes no assertive announcement, and an error does', async ({ page }) => {
    await open(page, queueStory);
    await page.locator('#push-info').click();
    await settle(page);

    let heard = await announcements(page, 'queue');
    expect(heard.polite, 'nothing was announced politely, so the emptiness below proves nothing')
      .toContain('Link copied');
    expect(
      heard.assertive,
      'a polite toast reached the assertive region, which interrupts whatever a person was reading',
    ).toBe('');

    await page.locator('#push-error').click();
    await settle(page);
    heard = await announcements(page, 'queue');
    expect(heard.assertive).toContain('Could not reach the server');
    // …and the polite region still holds its own message rather than having been repurposed.
    expect(heard.polite).toContain('Link copied');
  });
});

test.describe('the toast queue, driven by real time', () => {
  test('stacks in insertion order, oldest furthest from the corner', async ({ page }) => {
    await open(page, tonesStory);
    const cards = page.locator('#tones').locator('css=div.toast');
    await expect(cards).toHaveCount(toastStackCeiling);
    const tones = await cards.evaluateAll((nodes) =>
      nodes.map((node) => (node as HTMLElement).dataset['tone'] ?? ''),
    );
    // The info was pushed first and the ceiling retired it; the rest kept their order.
    expect(tones).toEqual(['success', 'warning', 'error']);

    let previousBottom = -Infinity;
    for (let index = 0; index < toastStackCeiling; index += 1) {
      const box = await boxOf(cards.nth(index), `card ${String(index)}`);
      expect(box.y, 'the stack is not in DOM order on screen').toBeGreaterThanOrEqual(previousBottom);
      previousBottom = box.y;
    }
  });

  /**
   * ⚠ **The one thing the Node sweep cannot prove: that real time reaches the queue.**
   *
   * Two toasts half a dwell apart leave half a dwell apart. A shared timer would drop both together
   * or neither, and a per-toast timer with a missing cancellation would restart the first one's
   * clock — neither of which a test that only waited for the end could see.
   */
  test('a second toast does not restart the first one’s clock', async ({ page }) => {
    await open(page, queueStory);
    const cards = page.locator('#queue').locator('css=div.toast');

    await page.locator('#push-info').click();
    await page.waitForTimeout(storyDwell / 2);
    await page.locator('#push-success').click();
    await expect(cards).toHaveCount(2);

    // Just past the first one's deadline and well before the second's.
    await page.waitForTimeout(storyDwell / 2 + 250);
    await expect(cards).toHaveCount(1);
    await expect(cards.first()).toHaveAttribute('data-tone', 'success');

    await page.waitForTimeout(storyDwell / 2 + 250);
    await expect(cards).toHaveCount(0);
  });

  test('an error stays while everything around it leaves', async ({ page }) => {
    await open(page, queueStory);
    const cards = page.locator('#queue').locator('css=div.toast');
    await page.locator('#push-error').click();
    await page.locator('#push-info').click();
    await expect(cards).toHaveCount(2);
    await page.waitForTimeout(storyDwell + 500);
    await expect(cards).toHaveCount(1);
    await expect(cards.first()).toHaveAttribute('data-tone', 'error');

    // …and the ceiling does not push it out either.
    for (const id of ['#push-success', '#push-success', '#push-success']) {
      await page.locator(id).click();
    }
    await expect(cards).toHaveCount(toastStackCeiling);
    await expect(cards.first()).toHaveAttribute('data-tone', 'error');
  });

  test('the ceiling holds, and it is a positive number of real cards', async ({ page }) => {
    await open(page, queueStory);
    const cards = page.locator('#queue').locator('css=div.toast');
    for (let index = 0; index < toastStackCeiling + 2; index += 1) {
      await page.locator('#push-info').click();
    }
    expect(toastStackCeiling).toBeGreaterThan(0);
    await expect(cards).toHaveCount(toastStackCeiling);
    await page.locator('#clear-toasts').click();
    await expect(cards).toHaveCount(0);
  });

  test('the dismissal is named after the message it dismisses, and works', async ({ page }) => {
    await open(page, tonesStory);
    const cards = page.locator('#tones').locator('css=div.toast');
    const before = await cards.count();
    expect(before).toBeGreaterThan(0);
    const dismiss = cards.first().locator('css=button.toast-dismiss');
    await expect(dismiss).toHaveAttribute('aria-label', /Dismiss: Saved to OneDrive/);
    await dismiss.click();
    await expect(cards).toHaveCount(before - 1);
  });

  test('every tone paints a different edge from the card it sits on', async ({ page }) => {
    await open(page, tonesStory);
    const edges = await page
      .locator('#tones')
      .locator('css=div.toast')
      .evaluateAll((nodes) =>
        nodes.map((node) => {
          const style = getComputedStyle(node as HTMLElement);
          return {
            tone: (node as HTMLElement).dataset['tone'] ?? '',
            edge: style.borderInlineStartColor,
            fill: style.backgroundColor,
          };
        }),
      );
    expect(edges.length).toBeGreaterThan(0);
    for (const row of edges) {
      expect(row.edge, `${row.tone} has no edge colour`).not.toBe('');
      expect(row.edge, `${row.tone} paints its edge in its own fill`).not.toBe(row.fill);
    }
  });

  test('the dismissal clears the hit-target floor in compact density', async ({ page }) => {
    await open(page, compactToastStory);
    const dismissals = page.locator('#compact-toasts').locator('css=button.toast-dismiss');
    const count = await dismissals.count();
    expect(count).toBeGreaterThan(0);
    for (let index = 0; index < count; index += 1) {
      const box = await boxOf(dismissals.nth(index), `dismissal ${String(index)}`);
      expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
      expect(box.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
    }
  });
});

// ── progress ─────────────────────────────────────────────────────────────────

/** The indicator's width as a fraction of its own track. Measured, never derived from the value. */
async function fillFraction(page: Page, id: string): Promise<number> {
  const track = await boxOf(page.locator(`#${id}`).locator('css=div.track'), `${id}'s track`);
  const width = await page
    .locator(`#${id}`)
    .locator('css=div.indicator')
    .evaluate((node) => node.getBoundingClientRect().width);
  return width / track.width;
}

test.describe('a determinate bar is its value', () => {
  /**
   * ⚠ **Five values including both ends**, each compared against **its own track** rather than
   * against the bar above it. A ladder that only asserted each bar was wider than the last would
   * pass for a bar that grew by a pixel a step.
   */
  test('the painted indicator is the value’s fraction of the track, at every step', async ({
    page,
  }) => {
    await open(page, ladderStory);
    // Past the transition, so what is measured is where the indicator settled rather than where it
    // happened to be on its way there.
    await page.waitForTimeout(600);
    for (const [percent, expected] of [
      [0, 0],
      [25, 0.25],
      [50, 0.5],
      [75, 0.75],
      [100, 1],
    ] as const) {
      const measured = await fillFraction(page, `ladder-${String(percent)}`);
      expect(measured, `the bar at ${String(percent)} per cent`).toBeCloseTo(expected, 2);
    }
  });

  test('announces its own scale rather than a normalised hundred', async ({ page }) => {
    await open(page, countableStory);
    const files = page.locator('#files').locator('css=div.track');
    await expect(files).toHaveAttribute('role', 'progressbar');
    await expect(files).toHaveAttribute('aria-valuemin', '0');
    await expect(files).toHaveAttribute('aria-valuemax', '7');
    await expect(files).toHaveAttribute('aria-valuenow', '3');
    await expect(files).toHaveAttribute('aria-valuetext', 'Uploading files: 43%');
    await expect(page.locator('#files').locator('css=span.readout')).toHaveText('43%');
    expect(await fillFraction(page, 'files')).toBeCloseTo(3 / 7, 2);

    /*
     * ⚠ **One of forty-nine, and it is the assertion that found a defect.**
     *
     * The obvious way to announce the value is to recover it from the fraction — `fraction * max` —
     * and floating-point division does not always give it back: this pair returns
     * `0.9999999999999999`, which an assistive technology reads out in full. Every other value in
     * the catalogue divides exactly, which is exactly how it would have survived the suite.
     */
    const shapes = page.locator('#shapes').locator('css=div.track');
    await expect(shapes).toHaveAttribute('aria-valuenow', '1');
    await expect(shapes).toHaveAttribute('aria-valuemax', '49');
    await expect(shapes).toHaveAttribute('aria-valuetext', 'Placing shapes: 2%');
  });
});

test.describe('an indeterminate bar is not a stalled one', () => {
  /** Two samples a fraction of a cycle apart. The instrument a still cannot be. */
  async function indicatorOffset(page: Page, id: string): Promise<number> {
    return page
      .locator(`#${id}`)
      .locator('css=div.indicator')
      .evaluate((node) => node.getBoundingClientRect().x);
  }

  test('the two are the same width, so a width comparison decides nothing', async ({ page }) => {
    await open(page, stalledStory);
    await page.waitForTimeout(600);
    const stalled = await fillFraction(page, 'stalled');
    const working = await fillFraction(page, 'working');
    expect(stalled).toBeCloseTo(indeterminateSpanFraction, 2);
    expect(working).toBeCloseTo(indeterminateSpanFraction, 2);
  });

  test('one of them carries a value and the other carries none', async ({ page }) => {
    await open(page, stalledStory);
    const stalled = page.locator('#stalled').locator('css=div.track');
    const working = page.locator('#working').locator('css=div.track');
    await expect(stalled).toHaveAttribute('aria-valuenow', /.+/);
    await expect(working).not.toHaveAttribute('aria-valuenow', /.*/);
    await expect(working).toHaveAttribute('aria-valuetext', 'Contacting the server: working');
    await expect(stalled).toHaveAttribute('aria-valuetext', /45%$/);
  });

  test('and one of them moves', async ({ page }) => {
    await open(page, stalledStory);
    const cycle = feedbackTimingMilliseconds('progressCycle');
    const workingFirst = await indicatorOffset(page, 'working');
    const stalledFirst = await indicatorOffset(page, 'stalled');
    await page.waitForTimeout(cycle * 0.25);
    const workingSecond = await indicatorOffset(page, 'working');
    const stalledSecond = await indicatorOffset(page, 'stalled');

    expect(
      Math.abs(workingSecond - workingFirst),
      'the indeterminate indicator did not move between two samples a quarter of a cycle apart',
    ).toBeGreaterThan(1);
    expect(
      Math.abs(stalledSecond - stalledFirst),
      'the determinate bar moved, which means the sample interval is measuring a transition',
    ).toBeLessThanOrEqual(1);
  });

  /**
   * ⚠ **And this is why the movement cannot be the only difference.**
   *
   * With a reduced-motion preference nothing moves, so a person who asked for less motion would be
   * looking at two identical rectangles if the accessibility tree did not carry the distinction.
   * Emulated rather than argued.
   */
  test('under a reduced-motion preference neither moves, and the tree still tells them apart', async ({
    page,
  }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await open(page, stalledStory);
    const cycle = feedbackTimingMilliseconds('progressCycle');

    const first = await indicatorOffset(page, 'working');
    await page.waitForTimeout(cycle * 0.5);
    const second = await indicatorOffset(page, 'working');
    expect(
      Math.abs(second - first),
      'the indeterminate bar is still sweeping for someone who asked their system to stop moving things',
    ).toBeLessThanOrEqual(1);

    await expect(page.locator('#working').locator('css=div.track')).not.toHaveAttribute(
      'aria-valuenow',
      /.*/,
    );
    await expect(page.locator('#stalled').locator('css=div.track')).toHaveAttribute(
      'aria-valuenow',
      /.+/,
    );
    // …and the geometry says the same thing a second way: the whole track, which is no fraction.
    expect(await fillFraction(page, 'working')).toBeCloseTo(1, 2);
  });
});

// ── the empty state ──────────────────────────────────────────────────────────

test.describe('the empty state', () => {
  test('announces itself, is named by its heading, and its picture is silent', async ({ page }) => {
    await open(page, actionStory);
    const box = page.locator('#comments-empty').locator('css=div.empty');
    await expect(box).toHaveAttribute('role', 'status');
    await expect(box).toHaveAttribute('aria-live', 'polite');
    const named = await page.evaluate(() => {
      const root = document.querySelector('#comments-empty')?.shadowRoot ?? null;
      const region = root?.querySelector('div.empty') ?? null;
      const id = region?.getAttribute('aria-labelledby') ?? '';
      return {
        heading: root?.getElementById(id)?.textContent ?? '',
        art: root?.querySelector('span.art')?.getAttribute('aria-hidden') ?? '',
      };
    });
    expect(named.heading).toBe('No comments yet');
    expect(named.art).toBe('true');
  });

  /**
   * ⚠ **The whole point of the component, and the one thing a picture of it cannot show.**
   *
   * The story's action is wired to the real list rather than to a counter, so what is asserted is
   * that pressing the button *achieves* something — not that an event fired.
   */
  test('the action does the thing', async ({ page }) => {
    await open(page, actionStory);
    await expect(page.locator('#comment-list').locator('css=li')).toHaveCount(0);
    await expect(page.locator('#comments-empty')).toBeVisible();

    await page.locator('#comments-empty').locator('css=button.action').click();
    await expect(page.locator('#comment-list').locator('css=li')).toHaveCount(3);
    await expect(page.locator('#comments-empty')).toBeHidden();
  });

  /**
   * ⚠ **This assertion found a real defect, and it is worth writing down where it happened.**
   *
   * The button carried `hidden`, the accessibility tree agreed, and it was on screen — because the
   * UA's `[hidden] { display: none }` lives in the user-agent origin and **any** author rule beats
   * it, so `.action { display: inline-flex }` had silently switched `hidden` off. An assertion on
   * the *attribute* would have passed; only asking the browser whether the thing is visible caught
   * it. See `hiddenLastCss` in the model for the fix and for the ordering it depends on.
   */
  test('an emptiness with nothing to offer has no button rather than a dead one', async ({
    page,
  }) => {
    await open(page, nothingToOfferStory);
    await expect(
      page.locator('#readonly-empty').locator('css=button.action'),
    ).toBeHidden();
    await expect(page.locator('#readonly-empty').locator('css=h2.heading')).toHaveText(
      'This deck has no notes',
    );
  });

  /**
   * The one place the serif is allowed, checked against the cascade rather than against the class
   * name — a heading that named the display role and resolved to the sans would pass a class check
   * and be wrong on screen.
   */
  test('its heading is the display face, and the description is not', async ({ page }) => {
    await open(page, actionStory);
    const faces = await page.evaluate(() => {
      const root = document.querySelector('#comments-empty')?.shadowRoot ?? null;
      const read = (selector: string): string => {
        const element = root?.querySelector(selector);
        return element === null || element === undefined
          ? ''
          : getComputedStyle(element).fontFamily;
      };
      return {
        heading: read('h2.heading'),
        description: read('p.description'),
        serif: getComputedStyle(document.documentElement).getPropertyValue('--font-serif').trim(),
        sans: getComputedStyle(document.documentElement).getPropertyValue('--font-sans').trim(),
      };
    });
    expect(faces.serif).not.toBe('');
    expect(faces.heading).toBe(faces.serif);
    expect(faces.description).toBe(faces.sans);
    expect(faces.heading).not.toBe(faces.description);
  });
});
