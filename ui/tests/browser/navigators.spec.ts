import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import {
  navigatorAriaPatterns,
  navigatorPhoneSheetAtOrBelow,
  navigatorPresentationAt,
  navigatorStoryTitles,
  sheetNameMaximum,
} from '../../src/navigators/navigator-model.ts';
import { largeNavigatorCount } from '../../stories/navigators/specimens.ts';

/**
 * MJXOFF-191's navigators, driven in a real browser.
 *
 * The ticket names two traps and both of them are here, because neither can be proved anywhere else:
 *
 * * **Virtualisation is invisible on a short list.** Every count below is taken over a
 *   five-thousand-item fixture, on the **number of nodes**, and every ceiling has an anti-vacuity
 *   assertion beside it. U06's ceiling was once satisfied by *zero*, and would have stayed green for
 *   a component that rendered nothing at all.
 * * **Scroll-position stability when items change above the viewport never appears in a still.** So
 *   the assertion is made on the **on-screen position of a named row**, measured before and after a
 *   thousand rows are inserted above it — and **beside the naive answer**, which the component
 *   reports for exactly this purpose. Without that positive control, *the row did not move* is
 *   trivially true of a list that never scrolls.
 *
 * Two more things live only here: **which ARIA pattern the browser actually computed** (asked of
 * Playwright's role engine rather than read off the component's own attribute — U04's rule applied
 * to ARIA) and **the hit-target floor**, which is a promise about pixels.
 */

const listStory = { title: navigatorStoryTitles.virtualList, name: 'Five Thousand Rows, A Screenful Of Elements' } as const;
const stableStory = { title: navigatorStoryTitles.virtualList, name: 'A Thousand Rows Arrive Above You' } as const;
const compactListStory = { title: navigatorStoryTitles.virtualList, name: 'In Compact Density' } as const;

const treeStory = { title: navigatorStoryTitles.tree, name: 'A Heading Moves With Its Subtree' } as const;
const bigTreeStory = { title: navigatorStoryTitles.tree, name: 'Five Thousand Headings' } as const;
const compactTreeStory = { title: navigatorStoryTitles.tree, name: 'In Compact Density' } as const;

const pendingStory = { title: navigatorStoryTitles.thumbnailRail, name: 'A Plate That Has Not Arrived' } as const;
const bigRailStory = { title: navigatorStoryTitles.thumbnailRail, name: 'Five Thousand Slides' } as const;
const insertStory = { title: navigatorStoryTitles.thumbnailRail, name: 'Slides Inserted Above The Viewport' } as const;

const renameStory = { title: navigatorStoryTitles.sheetTabBar, name: 'Rename In Place, And Escape Cancels' } as const;
const overflowStory = { title: navigatorStoryTitles.sheetTabBar, name: 'When The Tabs Overflow' } as const;
const compactTabStory = { title: navigatorStoryTitles.sheetTabBar, name: 'In Compact Density' } as const;

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
  for (const tag of ['mjx-virtual-list', 'mjx-tree', 'mjx-thumbnail-rail', 'mjx-sheet-tab-bar']) {
    await page.waitForFunction((name) => customElements.get(name) !== undefined, tag);
  }
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/** What a navigator reports about its own window, so the gate compares two numbers. */
interface WindowReport {
  readonly built: number;
  readonly expected: number;
  readonly measured: number;
  readonly offset: number;
  readonly naive: number;
}

/** Scroll a tree so that a named row is inside its window, so a DOM assertion is not a lottery. */
async function scrollToRow(page: Page, host: string, id: string): Promise<void> {
  await page.locator(host).evaluate(
    (element, wanted) => {
      const tree = element as HTMLElement & {
        rows: { id: string }[];
        scrollToIndex: (index: number) => void;
      };
      const at = tree.rows.findIndex((row) => row.id === wanted);
      if (at >= 0) tree.scrollToIndex(at);
    },
    id,
  );
  await settle(page);
}

async function reportOf(page: Page, selector: string): Promise<WindowReport> {
  return page.evaluate((query) => {
    const element = document.querySelector(query) as HTMLElement & {
      builtRowCount: number;
      expectedRowCount: number;
      measuredRowCount: number;
      offset: number;
      naiveOffset: number;
    };
    return {
      built: element.builtRowCount,
      expected: element.expectedRowCount,
      measured: element.measuredRowCount,
      offset: element.offset,
      naive: element.naiveOffset,
    };
  }, selector);
}

// ── the three ARIA patterns, as the browser computed them ────────────────────

test.describe('three ARIA patterns, and the browser is asked which', () => {
  test('the list is a listbox whose rows are options, with the set size on the OPTION', async ({
    page,
  }) => {
    await open(page, listStory);
    const listbox = page.getByRole(navigatorAriaPatterns.virtualList.container as 'listbox', {
      name: 'Comments',
    });
    await expect(listbox).toHaveCount(1);

    const options = page.locator('#big-list .row');
    const built = await options.count();
    expect(built, 'no rows at all makes every count below vacuous').toBeGreaterThan(0);
    // ⚠ aria-setsize is on the OPTION and never on the listbox — it is a position-within-a-set
    // attribute, which a container cannot be, and it is a virtualised list's only honest source for
    // the count because the number of rows in the DOM is not the number of items.
    await expect(options.first()).toHaveAttribute('aria-setsize', String(largeNavigatorCount));
    await expect(listbox).not.toHaveAttribute('aria-setsize', /.*/);
  });

  test('the tree is a tree whose rows are treeitems, and a leaf carries no expanded state', async ({
    page,
  }) => {
    await open(page, treeStory);
    await expect(page.getByRole('tree', { name: 'Navigation' })).toHaveCount(1);
    const items = page.locator('#behaviour-tree .row');
    expect(await items.count()).toBeGreaterThan(0);

    const branch = page.locator('#behaviour-tree .row[data-id="intro"]');
    await expect(branch).toHaveAttribute('aria-expanded', 'true');
    await expect(branch).toHaveAttribute('aria-level', '1');

    // A leaf must not announce a branch that does not exist. Scrolled to deliberately: a
    // virtualised tree's DOM is a window, and asserting on a row that happens to be in it today is
    // an assertion that starts failing when a font gets taller.
    await scrollToRow(page, '#behaviour-tree', 'results');
    const leaf = page.locator('#behaviour-tree .row[data-id="results"]');
    await expect(leaf).toHaveCount(1);
    await expect(leaf).not.toHaveAttribute('aria-expanded', /.*/);

    // aria-posinset is over the node's own siblings, not over the flattened list.
    const nested = page.locator('#behaviour-tree .row[data-id="intro-terms"]');
    await expect(nested).toHaveAttribute('aria-level', '2');
    await expect(nested).toHaveAttribute('aria-posinset', '2');
    await expect(nested).toHaveAttribute('aria-setsize', '2');
  });

  test('the rail is a multi-selectable listbox, never a tree', async ({ page }) => {
    await open(page, bigRailStory);
    const listbox = page.getByRole('listbox', { name: 'Slides' });
    await expect(listbox).toHaveCount(1);
    await expect(listbox).toHaveAttribute('aria-multiselectable', 'true');
    // The wrong pattern is worse than none: a slide has nothing to expand.
    await expect(page.getByRole('tree')).toHaveCount(0);
    await expect(page.locator('#big-rail .slide').first()).toHaveAttribute('aria-posinset', '1');
  });

  test('the tab bar is a tablist, and the affordances are deliberately not inside it', async ({
    page,
  }) => {
    await open(page, overflowStory);
    const tablist = page.getByRole('tablist', { name: 'Sheets' });
    await expect(tablist).toHaveCount(1);
    // ⚠ A tablist's own children must be tabs. A button among them is a real aria-required-children
    // violation, and the a11y sweep would have found it — but it would have found it as one story
    // failing rather than as a statement about where the affordances live.
    const insideTheStrip = await page.locator('#overflow-bar .strip > *:not([role="tab"])').count();
    expect(insideTheStrip).toBe(0);
    await expect(page.getByRole('button', { name: 'Scroll to the first sheet' })).toHaveCount(1);
    await expect(page.getByRole('button', { name: 'New sheet' })).toHaveCount(1);
  });
});

// ── the trap (a): virtualisation, asserted on node count ─────────────────────

test.describe('virtualisation, asserted on node count over five thousand items', () => {
  test('a five-thousand-row list builds a screenful, and the number comes from the window', async ({
    page,
  }) => {
    await open(page, listStory);
    const before = await reportOf(page, '#big-list');

    expect(before.built, 'a list that built nothing satisfies every ceiling').toBeGreaterThan(0);
    expect(before.built).toBeLessThan(largeNavigatorCount / 50);
    // The number compared against comes from the foundations' own window rather than from the
    // renderer — U06's rule, and the only kind of number this assertion may use.
    expect(before.built).toBe(before.expected);

    // Scroll a long way and require the count to stay a screenful. A component that appended rows
    // as it went would pass the first assertion and fail this one.
    await page.evaluate(() => {
      const list = document.querySelector('#big-list');
      const viewport = list?.shadowRoot?.querySelector('.viewport');
      if (viewport instanceof HTMLElement) viewport.scrollTop = viewport.scrollHeight / 2;
    });
    await settle(page);
    const after = await reportOf(page, '#big-list');
    expect(after.built).toBeGreaterThan(0);
    expect(after.built).toBeLessThan(largeNavigatorCount / 50);
    expect(after.built).toBe(after.expected);
    // Anti-vacuity of a different kind: the scroll must actually have moved the window.
    expect(after.offset).toBeGreaterThan(before.offset);
  });

  test('a five-thousand-heading tree builds a screenful', async ({ page }) => {
    await open(page, bigTreeStory);
    const report = await reportOf(page, '#big-tree');
    const rows = await page.locator('#big-tree .row').count();
    expect(rows).toBeGreaterThan(0);
    expect(rows).toBe(report.built);
    expect(rows).toBe(report.expected);
    expect(rows).toBeLessThan(largeNavigatorCount / 50);
    // ⚠ And the rows are genuinely of different heights, which is the precondition U07 had to hold
    // by hand. If they were all the same, dividing would work and the binary search would be
    // unexercised — so a fixture of uniform rows would make this whole file weaker.
    const heights = new Set<number>();
    for (const fraction of [0, 0.25, 0.5]) {
      await page.evaluate((where) => {
        const viewport = document
          .querySelector('#big-tree')
          ?.shadowRoot?.querySelector('.viewport') as HTMLElement | null;
        if (viewport !== null) viewport.scrollTop = viewport.scrollHeight * where;
      }, fraction);
      await settle(page);
      const sample = await page
        .locator('#big-tree .row')
        .evaluateAll((rows_) => rows_.map((row) => Math.round(row.getBoundingClientRect().height)));
      for (const height of sample) heights.add(height);
    }
    // Sampled across the outline rather than at the top, and that is the honest shape: whether the
    // first screenful happens to contain a wrapping heading is an accident of the fixture, and an
    // assertion that depends on one is an assertion that starts failing when a font changes.
    expect(heights.size).toBeGreaterThan(1);
  });

  test('a five-thousand-slide rail builds a screenful, and the scrollbar tells the truth', async ({
    page,
  }) => {
    await open(page, bigRailStory);
    const report = await reportOf(page, '#big-rail');
    expect(report.built).toBeGreaterThan(0);
    expect(report.built).toBe(report.expected);
    expect(report.built).toBeLessThan(largeNavigatorCount / 50);

    // The sizer is as tall as EVERY row, built or not: a scroll container whose height came only
    // from what was built is a scrollbar that lies further the further a reader scrolls.
    const geometry = await page.evaluate(() => {
      const viewport = document
        .querySelector('#big-rail')
        ?.shadowRoot?.querySelector('.viewport') as HTMLElement | null;
      return { scrollHeight: viewport?.scrollHeight ?? 0, clientHeight: viewport?.clientHeight ?? 0 };
    });
    expect(geometry.clientHeight).toBeGreaterThan(0);
    expect(geometry.scrollHeight).toBeGreaterThan(geometry.clientHeight * 50);
  });
});

// ── the trap (b): stability when rows change above the viewport ──────────────

test.describe('a row does not move when rows are inserted above it', () => {
  test('the list, measured on the on-screen position of a named row', async ({ page }) => {
    await open(page, stableStory);

    // Scroll to somewhere in the middle and pick a row that is genuinely on screen.
    await page.evaluate(() => {
      const viewport = document
        .querySelector('#stable-list')
        ?.shadowRoot?.querySelector('.viewport') as HTMLElement | null;
      if (viewport !== null) viewport.scrollTop = 40_000;
    });
    await settle(page);

    const watched = page.locator('#stable-list .row').nth(2);
    const label = await watched.locator('.label').textContent();
    expect(label, 'no row to watch makes the whole assertion vacuous').toBeTruthy();
    const before = await watched.boundingBox();
    expect(before).not.toBeNull();

    await page.getByRole('button', { name: 'Insert a thousand rows above' }).click();
    await settle(page);
    await settle(page);

    const again = page.locator('#stable-list .row').filter({ hasText: label ?? '' }).first();
    const after = await again.boundingBox();
    expect(after, 'the watched row left the DOM, so nothing was measured').not.toBeNull();
    expect(Math.abs((after?.y ?? 0) - (before?.y ?? 0))).toBeLessThan(2);

    /*
     * ⚠ **The positive control, and without it this test is a tolerance nobody has tested.** The
     * component reports what the offset would have been had it simply kept the scroll position. If
     * the two are ever equal, everything above is satisfied by a component that does nothing at all.
     */
    const report = await reportOf(page, '#stable-list');
    expect(report.naive).toBeGreaterThan(0);
    expect(Math.abs(report.offset - report.naive)).toBeGreaterThan(100);
    // R13's own anti-vacuity: a stability assertion alone is green for a model that never corrects.
    expect(report.measured).toBeGreaterThan(0);
  });

  test('the rail, with twenty slides inserted at the top', async ({ page }) => {
    await open(page, insertStory);
    await page.evaluate(() => {
      const viewport = document
        .querySelector('#insert-rail')
        ?.shadowRoot?.querySelector('.viewport') as HTMLElement | null;
      if (viewport !== null) viewport.scrollTop = 3000;
    });
    await settle(page);

    const watched = page.locator('#insert-rail .slide').nth(2);
    const id = await watched.getAttribute('data-id');
    expect(id).toBeTruthy();
    const before = await watched.boundingBox();
    expect(before).not.toBeNull();

    await page.getByRole('button', { name: 'Insert twenty slides at the top' }).click();
    await settle(page);
    await settle(page);

    const again = page.locator(`#insert-rail .slide[data-id="${id ?? ''}"]`);
    await expect(again).toHaveCount(1);
    const after = await again.boundingBox();
    expect(Math.abs((after?.y ?? 0) - (before?.y ?? 0))).toBeLessThan(2);

    const report = await reportOf(page, '#insert-rail');
    expect(Math.abs(report.offset - report.naive)).toBeGreaterThan(100);

    // The slide's NUMBER changed, which is the honest outcome: it is twenty slides later now. What
    // must not move is the picture on the screen.
    await expect(again).toHaveAttribute('aria-posinset', /^2[0-9]|^[3-9][0-9]/);
  });
});

// ── keyboard reordering, which is not optional ───────────────────────────────

test.describe('reordering by keyboard', () => {
  test('a tree heading moves among its siblings and takes its subtree with it', async ({ page }) => {
    await open(page, treeStory);
    const tree = page.locator('#behaviour-tree .viewport');
    await tree.focus();

    // Down to Method, which has two children and one grandchild showing.
    /*
     * ⚠ The **visible** rows, not the built ones. The two are the same only while everything fits,
     * and a virtualised tree's DOM is a window — so an identity assertion taken from the DOM would
     * silently become an assertion about scrolling. The DOM's own correctness is asserted in the
     * ARIA tests above; this one is about the outline.
     */
    const order = async (): Promise<string[]> =>
      page.locator('#behaviour-tree').evaluate((element) =>
        (element as HTMLElement & { rows: { id: string }[] }).rows.map((row) => row.id),
      );
    const before = await order();
    const methodAt = before.indexOf('method');
    expect(methodAt).toBeGreaterThan(0);
    for (let step = 0; step < methodAt; step += 1) await page.keyboard.press('ArrowDown');

    await page.keyboard.press('Alt+ArrowUp');
    await settle(page);
    const after = await order();

    // Method is now above Introduction — and its children came with it, in their own order.
    expect(after.indexOf('method')).toBeLessThan(after.indexOf('intro'));
    expect(after.slice(after.indexOf('method'), after.indexOf('method') + 5)).toEqual([
      'method',
      'method-corpus',
      'method-corpus-a',
      'method-corpus-b',
      'method-oracle',
    ]);
    // ⚠ Nothing was lost. A flattened-list swap would have left the children behind under whichever
    // heading ended up above them, and it would look correct until somebody collapsed the branch.
    expect(after.length).toBe(before.length);
    expect([...after].sort()).toEqual([...before].sort());

    const announced = await page.locator('#behaviour-tree').evaluate(
      (element) => (element as HTMLElement & { announcement: string }).announcement,
    );
    expect(announced).toContain('4');
  });

  test('and a heading already at the top of its branch says so rather than doing nothing', async ({
    page,
  }) => {
    await open(page, treeStory);
    await page.locator('#behaviour-tree .viewport').focus();
    await page.keyboard.press('Alt+ArrowUp');
    await settle(page);
    const announced = await page.locator('#behaviour-tree').evaluate(
      (element) => (element as HTMLElement & { announcement: string }).announcement,
    );
    expect(announced).toContain('start');
  });

  test('indent and outdent change a heading’s depth, carrying its subtree', async ({ page }) => {
    await open(page, treeStory);
    await page.locator('#behaviour-tree .viewport').focus();
    // Onto Results, which is a top-level leaf after Method.
    const levelOf = async (id: string): Promise<string | null> => {
      await scrollToRow(page, '#behaviour-tree', id);
      return page.locator(`#behaviour-tree .row[data-id="${id}"]`).getAttribute('aria-level');
    };
    expect(await levelOf('results')).toBe('1');

    const ids = await page.locator('#behaviour-tree').evaluate((element) =>
      (element as HTMLElement & { rows: { id: string }[] }).rows.map((row) => row.id),
    );
    for (let step = 0; step < ids.indexOf('results'); step += 1) {
      await page.keyboard.press('ArrowDown');
    }
    await page.keyboard.press('Alt+ArrowRight');
    await settle(page);
    expect(await levelOf('results')).toBe('2');

    await page.keyboard.press('Alt+ArrowLeft');
    await settle(page);
    expect(await levelOf('results')).toBe('1');
  });

  test('a rail moves the whole selection as a block, and announces where it landed', async ({
    page,
  }) => {
    await open(page, pendingStory);
    await page.locator('#pending-rail .viewport').focus();
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Shift+ArrowDown');

    const order = async (): Promise<string[]> =>
      page.locator('#pending-rail .slide').evaluateAll((rows) =>
        rows.map((row) => (row as HTMLElement).dataset['id'] ?? ''),
      );
    const before = await order();
    const selected = await page.locator('#pending-rail .slide[aria-selected="true"]').count();
    expect(selected, 'nothing selected makes the move vacuous').toBe(2);

    await page.keyboard.press('Alt+ArrowDown');
    await settle(page);
    const after = await order();
    expect(after).not.toEqual(before);
    expect([...after].sort()).toEqual([...before].sort());
    const announced = await page.locator('#pending-rail').evaluate(
      (element) => (element as HTMLElement & { announcement: string }).announcement,
    );
    expect(announced).toContain('2 slides moved');
  });

  test('a sheet moves one place by keyboard, and refuses at the end', async ({ page }) => {
    await open(page, renameStory);
    await page.getByRole('tab', { name: 'Summary' }).focus();
    await page.keyboard.press('Alt+ArrowRight');
    await settle(page);
    // ⚠ The LABEL's text, not the tab's: a hidden sheet's tab also carries a mark glyph, and a
    // reorder assertion that read the whole tab would be an assertion about the marking too.
    const labels = await page
      .locator('#rename-bar .tab .label')
      .evaluateAll((parts) => parts.map((part) => part.textContent?.trim() ?? ''));
    expect(labels[1]).toBe('Summary');

    await page.keyboard.press('Alt+ArrowLeft');
    await settle(page);
    await page.keyboard.press('Alt+ArrowLeft');
    await settle(page);
    const announced = await page.locator('#rename-bar').evaluate(
      (element) => (element as HTMLElement & { announcement: string }).announcement,
    );
    expect(announced).toContain('start');
  });
});

// ── reordering by pointer, which must land where the keyboard lands ──────────

test.describe('reordering by drag', () => {
  test('a real mouse drag moves a heading exactly where Alt + Arrow does', async ({ page }) => {
    /*
     * ⚠ **A real pointer through Playwright's own input path**, not a synthetic event sequence.
     * MJXOFF-190's rule: a dispatched `pointermove` proves a handler exists; it does not prove the
     * platform would ever call it, and a drag is precisely a thing the platform decides — a capture,
     * a threshold, a click that is not a drag.
     *
     * The claim being checked is the one the design rests on: **the drag calls the same function
     * the keyboard calls**, so the two must land in the same place. So the same move is made twice,
     * once by each road, and the two orders are compared to each other rather than to a list
     * written here.
     */
    const order = async (): Promise<string[]> =>
      page.locator('#behaviour-tree').evaluate((element) =>
        (element as HTMLElement & { rows: { id: string }[] }).rows.map((row) => row.id),
      );

    await open(page, treeStory);
    const before = await order();

    const from = page.locator('#behaviour-tree .row[data-id="results"]');
    const onto = page.locator('#behaviour-tree .row[data-id="method"]');

    /*
     * ⚠ **`hover()` and then a box, in that order.** A box read first and pressed at afterwards is
     * a box that may have moved: this list measures the rows it built and re-derives its offset
     * from the anchor, so the first correction after a story renders shifts every row by a pixel or
     * two. The first version of this test read the box, moved to it, and pressed on the *stage*
     * twelve pixels below the tree — and the failure said only that the order had not changed.
     */
    await from.hover();
    const start = await from.boundingBox();
    expect(start, 'the dragged row is not on screen, so nothing was dragged').not.toBeNull();
    if (start === null) return;

    await page.mouse.down();
    // Past the threshold first, in its own move: a single jump to the target would also be a drag,
    // but it would not exercise the press-then-travel path a person actually produces.
    await page.mouse.move(start.x + start.width / 2, start.y + start.height / 2 - 12);
    const target = await onto.boundingBox();
    expect(target, 'the drop target is not on screen').not.toBeNull();
    if (target === null) return;
    await page.mouse.move(target.x + target.width / 2, target.y + target.height / 2);
    await page.mouse.up();
    await settle(page);

    const dragged = await order();
    expect(dragged, 'the drag moved nothing at all').not.toEqual(before);
    expect([...dragged].sort()).toEqual([...before].sort());

    // Now the same move by keyboard, from the original order, and the two must agree.
    await open(page, treeStory);
    const tree = page.locator('#behaviour-tree .viewport');
    await tree.focus();
    const startingRows = await order();
    for (let step = 0; step < startingRows.indexOf('results'); step += 1) {
      await page.keyboard.press('ArrowDown');
    }
    const steps = startingRows.indexOf('results') - startingRows.indexOf('method');
    for (let step = 0; step < steps; step += 1) await page.keyboard.press('Alt+ArrowUp');
    await settle(page);

    expect(await order()).toEqual(dragged);
  });
});

// ── the thumbnail placeholder ────────────────────────────────────────────────

test.describe('a plate that has not arrived', () => {
  test('draws a placeholder, is announced fully, and stays usable', async ({ page }) => {
    await open(page, pendingStory);

    const pending = page.locator('#pending-rail .plate[data-state="pending"]');
    const ready = page.locator('#pending-rail .plate[data-state="ready"]');
    const waiting = await pending.count();
    expect(waiting, 'no pending rows makes this whole test vacuous').toBeGreaterThan(0);
    expect(await ready.count(), 'no ready rows means the two states are not contrasted').toBeGreaterThan(0);

    await expect(pending.first()).toHaveAttribute('aria-busy', 'true');
    await expect(pending.first().locator('.placeholder')).toBeVisible();

    // ⚠ The option itself is NOT busy: its name is complete and correct the moment the rail is
    // built, and marking the whole option busy would suppress it.
    const option = page.locator('#pending-rail .slide').first();
    await expect(option).not.toHaveAttribute('aria-busy', /.*/);
    const name = await option.getAttribute('aria-label');
    expect(name).toContain('Slide 1 of 8');

    // And it is still selectable and scrollable while it waits.
    await option.click();
    await expect(option).toHaveAttribute('aria-selected', 'true');

    await page.getByRole('button', { name: 'Deliver the plates' }).click();
    await expect(page.locator('#pending-rail .plate[data-state="pending"]')).toHaveCount(0, {
      timeout: 5000,
    });
    expect(await page.locator('#pending-rail .plate img').count()).toBeGreaterThan(0);
  });

  test('a hidden slide says so in its name, not only in its opacity', async ({ page }) => {
    await open(page, pendingStory);
    // Scrolled to: the hidden slide is the sixth of eight and a rail's DOM is a window.
    await page.locator('#pending-rail').evaluate((element) => {
      (element as HTMLElement & { scrollToIndex: (index: number) => void }).scrollToIndex(7);
    });
    await settle(page);
    const hidden = page.locator('#pending-rail .slide[data-hidden="true"]').first();
    await expect(hidden).toHaveCount(1);
    expect(await hidden.getAttribute('aria-label')).toContain('hidden');
  });
});

// ── the sheet tab bar ────────────────────────────────────────────────────────

test.describe('the sheet tabs', () => {
  test('rename commits on Enter and CANCELS on Escape', async ({ page }) => {
    await open(page, renameStory);
    const tab = page.getByRole('tab', { name: 'Q1 Actuals' });
    await tab.dblclick();
    const field = page.locator('#rename-bar .rename');
    await expect(field).toBeFocused();
    await field.fill('Something Else');
    await page.keyboard.press('Escape');
    await settle(page);

    // ⚠ The half that is usually wrong. A field that committed on Escape has destroyed a name with
    // the key people press to mean *stop*, and afterwards the two are the same picture.
    await expect(page.getByRole('tab', { name: 'Q1 Actuals' })).toHaveCount(1);
    await expect(page.getByRole('tab', { name: 'Something Else' })).toHaveCount(0);

    await tab.dblclick();
    await page.locator('#rename-bar .rename').fill('Q1 Restated');
    await page.keyboard.press('Enter');
    await settle(page);
    await expect(page.getByRole('tab', { name: 'Q1 Restated' })).toHaveCount(1);
    await expect(page.locator('#rename-readout')).toHaveText(/Q1 Restated/);
  });

  test('a name Excel would refuse keeps the text and commits nothing', async ({ page }) => {
    await open(page, renameStory);
    await page.getByRole('tab', { name: 'Q2 Actuals' }).dblclick();
    const field = page.locator('#rename-bar .rename');
    await field.fill('Q2/Q3');
    await page.keyboard.press('Enter');
    await settle(page);

    await expect(field).toHaveValue('Q2/Q3');
    await expect(page.locator('#rename-bar .problem')).toBeVisible();
    await expect(page.getByRole('tab', { name: 'Q2 Actuals' })).toHaveCount(1);

    await field.fill('x'.repeat(sheetNameMaximum + 1));
    await page.keyboard.press('Enter');
    await settle(page);
    await expect(page.locator('#rename-bar .problem')).toContainText(String(sheetNameMaximum));
  });

  test('a sheet colour is an edge and the label never sits on it', async ({ page }) => {
    await open(page, renameStory);
    const coloured = page.locator('#rename-bar .tab[data-index="0"]');
    const bar = coloured.locator('.tab-colour');
    const paint = await bar.evaluate((element) => getComputedStyle(element).backgroundColor);
    const behind = await coloured.evaluate((element) => getComputedStyle(element).backgroundColor);
    expect(paint).not.toBe('rgba(0, 0, 0, 0)');
    // ⚠ The tab's own fill is the palette's, never the workbook's. A label drawn on a colour this
    // catalogue did not choose would have a contrast nobody has checked.
    expect(behind).not.toBe(paint);
    // And it is genuinely drawn rather than a zero-height box nobody can see.
    const box = await bar.boundingBox();
    expect(box?.height ?? 0).toBeGreaterThan(0);

    // An uncoloured tab draws nothing at all, which is what makes the coloured one a signal.
    const plain = page.locator('#rename-bar .tab[data-index="1"] .tab-colour');
    expect(await plain.evaluate((element) => getComputedStyle(element).backgroundColor)).toBe(
      'rgba(0, 0, 0, 0)',
    );
  });

  test('the strip really overflows, and an affordance brings a far tab into view', async ({
    page,
  }) => {
    await open(page, overflowStory);
    const overflowing = await page.locator('#overflow-bar').evaluate(
      (element) => (element as HTMLElement & { overflowing: boolean }).overflowing,
    );
    expect(overflowing, 'the strip fits, so there is no overflow to test').toBe(true);

    const pageScrollBefore = await page.evaluate(() => window.scrollX);
    const before = await page
      .locator('#overflow-bar .strip')
      .evaluate((element) => element.scrollLeft);
    await page.getByRole('button', { name: 'Scroll to the last sheet' }).click();
    await settle(page);
    const after = await page
      .locator('#overflow-bar .strip')
      .evaluate((element) => element.scrollLeft);
    expect(after).toBeGreaterThan(before);

    /*
     * And the page around it did not move: focus({ preventScroll }) plus a scrollIntoView of our
     * own, because the browser's own walk reaches every ancestor scroll container.
     *
     * ⚠ Compared against where the page already was, not against zero. At the desktop preset the
     * harness frame is wider than the viewport and the page is ALREADY scrolled sideways — U11's
     * sixth defect, met from the other side: an assertion of zero here would have been an assertion
     * about the harness rather than about the tab strip.
     */
    expect(await page.evaluate(() => window.scrollX)).toBe(pageScrollBefore);
  });
});

// ── the touch presentation ───────────────────────────────────────────────────

test.describe('at a phone width the tree and the rail take the whole frame', () => {
  for (const [name, host] of [
    ['the tree', '#big-tree'],
    ['the rail', '#big-rail'],
  ] as const) {
    test(`${name} reports what the cascade decided, and the box agrees`, async ({ page }) => {
      const story = host === '#big-tree' ? bigTreeStory : bigRailStory;

      await open(page, story, { containerPreset: 'desktop' });
      const desktopWidth = await frameWidth(page);
      expect(await presentationOf(page, host)).toBe(navigatorPresentationAt(desktopWidth));
      expect(navigatorPresentationAt(desktopWidth)).toBe('pane');

      await open(page, story, { containerPreset: 'phone' });
      const phone = await frameWidth(page);
      expect(phone).toBeLessThanOrEqual(navigatorPhoneSheetAtOrBelow);
      expect(await presentationOf(page, host)).toBe(navigatorPresentationAt(phone));

      /*
       * ⚠ **Cross-checked against a fact the custom property does not decide.** U06's second
       * green-under-a-real-break defect was a gate that read a generated stylesheet back and
       * compared it against the table it was generated from — they cannot disagree. The radius is
       * geometry: at a phone width the pane loses its corner, because it is the whole frame.
       */
      const radius = await page
        .locator(host)
        .evaluate((element) => getComputedStyle(element).borderTopLeftRadius);
      expect(Number.parseFloat(radius)).toBe(0);
    });
  }
});

/**
 * The width of the **frame**, which is the container query's container.
 *
 * ⚠ Not the host element. `<mjx-resizable-container>` fills the page and resizes a `[part=frame]` box inside
 * its own shadow root — so measuring the host reported 1,280 at every preset, and a phone assertion
 * taken from it would have compared the *viewport* against a breakpoint that is not about the
 * viewport at all. U01's own rule — *the container is the mechanism, not the viewport* — as a
 * measurement mistake rather than as a design one.
 */
async function frameWidth(page: Page): Promise<number> {
  return page
    .locator('mjx-resizable-container [part="frame"]')
    .evaluate((frame) => frame.getBoundingClientRect().width);
}

async function presentationOf(page: Page, host: string): Promise<string> {
  return page.locator(host).evaluate(
    (element) => (element as HTMLElement & { presentation: string }).presentation,
  );
}

// ── the hit-target floor, which is a promise about pixels ────────────────────

test.describe('the hit-target floor holds in compact density', () => {
  const cases = [
    { story: compactListStory, selector: '#compact-list .row' },
    { story: compactTreeStory, selector: '#compact-tree .row' },
    { story: compactTabStory, selector: '#compact-bar .tab' },
    { story: compactTabStory, selector: '#compact-bar .affordance' },
    { story: compactTabStory, selector: '#compact-bar .add' },
  ] as const;

  for (const { story, selector } of cases) {
    test(`${selector} clears ${String(accessibleHitTargetMinimum)} CSS pixels`, async ({ page }) => {
      await open(page, story);
      const boxes = await page
        .locator(selector)
        .evaluateAll((elements) =>
          elements.map((element) => {
            const box = element.getBoundingClientRect();
            return { width: box.width, height: box.height };
          }),
        );
      expect(boxes.length, 'nothing matched, so nothing was measured').toBeGreaterThan(0);
      for (const box of boxes) {
        expect(box.height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
        expect(box.width).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
      }
    });
  }
});

// ── the cursor is visible, which the aria-activedescendant pattern requires ──

test('the keyboard cursor is drawn, because focus never reaches a row', async ({ page }) => {
  await open(page, listStory);
  const viewport = page.locator('#big-list .viewport');
  await viewport.focus();
  await page.keyboard.press('ArrowDown');
  await settle(page);

  const cursor = page.locator('#big-list .row[data-cursor="true"]');
  await expect(cursor).toHaveCount(1);
  const outline = await cursor.evaluate((element) => getComputedStyle(element).outlineWidth);
  // ⚠ No row is ever :focus-visible here — focus stays on the container — so the foundations' own
  // treatment can never reach one. Without a rule of its own an arrow key would move an
  // announcement nobody can see.
  expect(Number.parseFloat(outline)).toBeGreaterThan(0);

  // And the row the cursor names is really in the DOM: `aria-activedescendant` naming nothing is
  // silent, and the list looks perfectly fine while it happens.
  const named = await viewport.getAttribute('aria-activedescendant');
  expect(named).toBeTruthy();
  await expect(page.locator(`#big-list [id="${named ?? ''}"]`)).toHaveCount(1);
});

test('the cursor stays named after a jump the measurement is a frame behind', async ({ page }) => {
  await open(page, bigTreeStory);
  const viewport = page.locator('#big-tree .viewport');
  await viewport.focus();
  await page.keyboard.press('End');
  await settle(page);
  const named = await viewport.getAttribute('aria-activedescendant');
  await expect(page.locator(`#big-tree [id="${named ?? ''}"]`)).toHaveCount(1);
  await page.keyboard.press('Home');
  await settle(page);
  const first = await viewport.getAttribute('aria-activedescendant');
  await expect(page.locator(`#big-tree [id="${first ?? ''}"]`)).toHaveCount(1);
});
