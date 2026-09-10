import { expect, test, type Locator, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import { containerPresets } from '../../src/harness/presets.ts';
import { groupPresentationAt } from '../../src/ribbon/ribbon-model.ts';
import {
  cellsInWindow,
  galleryCellPaint,
  galleryCellStateNames,
  galleryParts,
  galleryPresentationAt,
  galleryRowPlan,
  gallerySheetAtOrBelow,
  galleryStateCellAttribute,
  galleryStatesMatrixStoryName,
  galleryStoryTitles,
  galleryStripRowsFor,
  galleryTouchPreviewDelay,
  nextGalleryIndex,
  pointerPreviewSettleDelay,
  type GalleryCellState,
} from '../../src/gallery/gallery-model.ts';
import { placeFloating } from '../../src/overlay/floating.ts';
import {
  galleryTestIds,
  largeGalleryItemCount,
  styleValues,
  unavailableStyleValue,
} from '../../stories/gallery/specimens.ts';

/**
 * The gallery, measured and driven in a browser.
 *
 * MJXOFF-185 names the trap, and it decides the shape of almost everything below:
 *
 * > **Live preview is the whole point of a gallery and is exactly the part a static story cannot
 * > show.** A gallery with the events unwired looks identical in every screenshot and passes any
 * > appearance snapshot. **So the gate is on the event protocol.**
 *
 * And the brief that commissioned this child sharpened it into four rules this file obeys:
 *
 * 1. The gate is not *"hovering changed something"* — it is **"leaving restored precisely the prior
 *    state"**, on the same measurements, after several items and by keyboard as well as pointer.
 * 2. **A preview that never commits and a preview that always commits both look right in a still**,
 *    so Enter and Escape are asserted to do *different* things.
 * 3. **A twenty-item fixture proves nothing** — virtualisation is asserted on a node count over four
 *    hundred items, against a number the plan produced rather than one the renderer did.
 * 4. The strip and the flyout are different geometries of one item set, so the selection is asserted
 *    to survive the transition **both ways**.
 *
 * And MJXOFF-184's lesson about vacuity is obeyed twice: the coalescing assertion is run **beside a
 * run with the coalescing removed**, and the restoration assertions sample the middle of the
 * traversal rather than only its end.
 */

const gallery = galleryStoryTitles.gallery;

const matrixStory = { title: gallery, name: galleryStatesMatrixStoryName } as const;
const previewStory = { title: gallery, name: 'Live Preview' } as const;
const largeStory = { title: gallery, name: 'A Large Gallery' } as const;
const kindsStory = { title: gallery, name: 'Three Kinds Of Item' } as const;
const groupStory = { title: gallery, name: 'Degrading With Its Group' } as const;
const rtlStory = { title: gallery, name: 'Under Right To Left' } as const;
const phoneStory = { title: gallery, name: 'On A Phone' } as const;

const schemes: readonly ColorScheme[] = ['light', 'dark'];
const transitionMilliseconds = Number.parseFloat(tokens.duration.transition);

async function open(
  page: Page,
  story: { readonly title: string; readonly name: string },
  options: { theme?: ColorScheme; containerPreset?: string } = {},
): Promise<void> {
  const entry = builtStories().find(
    (candidate) => candidate.title === story.title && candidate.name === story.name,
  );
  expect(entry, `${story.title} · ${story.name} is missing from the catalogue`).toBeDefined();
  if (entry === undefined) return;
  await openStory(page, entry.id, options);
  await page.waitForFunction(() => customElements.get('mjx-gallery') !== undefined);
  await settle(page);
}

async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
}

/** Two frames is a layout change; a paint change needs the transition the token declares. */
async function settlePaint(page: Page): Promise<void> {
  await settle(page);
  await page.waitForTimeout(transitionMilliseconds * 2 + 100);
}

/** `#2e9e63` → `rgb(46, 158, 99)`; `transparent` → what a browser reports for it. */
function expectedColor(member: string | undefined, scheme: ColorScheme): string {
  if (member === undefined || member === 'transparent') return 'rgba(0, 0, 0, 0)';
  const hex = tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
  const number = Number.parseInt(hex.slice(1, 7), 16);
  return `rgb(${String((number >> 16) & 0xff)}, ${String((number >> 8) & 0xff)}, ${String(number & 0xff)})`;
}

/** A cell of the strip, by item index. */
function stripCell(page: Page, index: number): Locator {
  return page.locator(`#${galleryTestIds.gallery}`).locator(`#strip-cell-${String(index)}`);
}

/**
 * Everything the preview target computes, as one comparable object.
 *
 * **Seven properties, and every one of them is moved by at least one of the eight styles.** *"It
 * went back"* is only worth asserting if what it went back from was a real, measurable difference:
 * a fingerprint of one property would be satisfied by seven styles that happen to share it.
 */
interface TargetFingerprint {
  applied: string | null;
  fontSize: string;
  fontWeight: string;
  fontStyle: string;
  fontFamily: string;
  letterSpacing: string;
  textDecoration: string;
}

async function readTarget(page: Page): Promise<TargetFingerprint> {
  return page.evaluate((id) => {
    const target = document.querySelector(`#${id}`);
    if (target === null) throw new Error(`the preview target #${id} is missing`);
    const style = getComputedStyle(target);
    return {
      applied: target.getAttribute('data-applied'),
      fontSize: style.fontSize,
      fontWeight: style.fontWeight,
      fontStyle: style.fontStyle,
      fontFamily: style.fontFamily,
      letterSpacing: style.letterSpacing,
      textDecoration: style.textDecorationLine,
    };
  }, galleryTestIds.target);
}

/** The protocol log the story records, one event per line. */
async function protocolLog(page: Page): Promise<string[]> {
  return page.evaluate((id) => {
    const log = document.querySelector(`#${id}`);
    return (log?.textContent ?? '').split('\n').filter((line) => line !== '');
  }, galleryTestIds.log);
}

async function clearLog(page: Page): Promise<void> {
  await page.evaluate((id) => {
    const log = document.querySelector(`#${id}`);
    if (log !== null) log.textContent = '';
  }, galleryTestIds.log);
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Live preview: the protocol, and restoration that is exact
// ─────────────────────────────────────────────────────────────────────────────

test.describe('live preview restores exactly what was there', () => {
  test('hovering several items in sequence changes the document at every step, and leaving puts it back', async ({
    page,
  }) => {
    await open(page, previewStory);
    const before = await readTarget(page);

    // ⚠ **Sampled at every step, not only at the end.** MJXOFF-184's vacuity lesson: a test that
    // only asserts the end state passes whether or not anything ever happened in the middle, and
    // "leaving restored it" is trivially true of a gallery that never previewed at all.
    const samples: unknown[] = [];
    for (const index of [1, 2, 3, 4, 5]) {
      await stripCell(page, index).hover();
      await page.waitForTimeout(pointerPreviewSettleDelay + 60);
      samples.push(await readTarget(page));
    }

    // Every sample differs from the resting state **and** from its neighbour, so the traversal
    // genuinely moved the document five times.
    for (const [position, sample] of samples.entries()) {
      expect(sample, `sample ${String(position)} did not change the document`).not.toEqual(before);
    }
    expect(new Set(samples.map((sample) => JSON.stringify(sample))).size).toBe(samples.length);

    // Leave the gallery entirely.
    await page.locator(`#${galleryTestIds.outside}`).hover();
    await settlePaint(page);
    const after = await readTarget(page);
    expect(after, 'leaving must restore precisely the prior state').toEqual(before);
  });

  test('the keyboard previews too, and Escape reverts exactly — sampled at every arrow', async ({
    page,
  }) => {
    await open(page, previewStory);
    const before = await readTarget(page);

    await stripCell(page, 0).focus();
    const samples: unknown[] = [];
    for (let step = 0; step < 5; step += 1) {
      await page.keyboard.press('ArrowRight');
      await settle(page);
      samples.push(await readTarget(page));
    }
    for (const [position, sample] of samples.entries()) {
      expect(sample, `arrow ${String(position)} emitted no preview`).not.toEqual(before);
    }
    expect(new Set(samples.map((sample) => JSON.stringify(sample))).size).toBe(samples.length);

    await page.keyboard.press('Escape');
    await settlePaint(page);
    expect(await readTarget(page), 'Escape must revert exactly').toEqual(before);
  });

  test('Enter commits and Escape reverts, and the two are distinguishable', async ({ page }) => {
    // ⚠ The brief: *"a preview that never commits and a preview that always commits both look right
    // in a still."* So both halves are asserted, against the same starting point.
    await open(page, previewStory);
    const resting = await readTarget(page);

    await stripCell(page, 0).focus();
    await page.keyboard.press('ArrowRight');
    await settle(page);
    const previewed = await readTarget(page);
    expect(previewed).not.toEqual(resting);

    await page.keyboard.press('Enter');
    await settlePaint(page);
    const committed = await readTarget(page);
    expect(committed, 'Enter must keep what was previewed').toEqual(previewed);
    expect(committed).not.toEqual(resting);

    // Now the other half, from the committed state: preview something else and Escape.
    await stripCell(page, 4).focus();
    await settle(page);
    expect(await readTarget(page)).not.toEqual(committed);
    await page.keyboard.press('Escape');
    await settlePaint(page);
    expect(
      await readTarget(page),
      'Escape must return to the committed value, not to the value the page opened with',
    ).toEqual(committed);
  });

  test('after a commit, leaving restores the committed value and not the initial one', async ({
    page,
  }) => {
    // **The failure `restore` is in the protocol to make impossible.** A listener that remembered
    // "what it was when the page loaded" would put the original back here, and no screenshot could
    // tell the difference.
    await open(page, previewStory);
    const initial = await readTarget(page);

    await stripCell(page, 1).click();
    await settlePaint(page);
    const committed = await readTarget(page);
    expect(committed).not.toEqual(initial);

    for (const index of [3, 5, 2]) {
      await stripCell(page, index).hover();
      await page.waitForTimeout(pointerPreviewSettleDelay + 60);
      expect(await readTarget(page)).not.toEqual(committed);
    }
    await page.locator(`#${galleryTestIds.outside}`).hover();
    await settlePaint(page);
    const restored = await readTarget(page);
    expect(restored).toEqual(committed);
    expect(restored).not.toEqual(initial);
  });

  test('the event sequence is exactly preview / cancel / commit, and never two previews at once', async ({
    page,
  }) => {
    await open(page, previewStory);
    await clearLog(page);

    await stripCell(page, 1).hover();
    await page.waitForTimeout(pointerPreviewSettleDelay + 60);
    await stripCell(page, 3).hover();
    await page.waitForTimeout(pointerPreviewSettleDelay + 60);
    await stripCell(page, 3).click();
    await settle(page);
    await page.locator(`#${galleryTestIds.outside}`).hover();
    await settle(page);

    const log = await protocolLog(page);
    const kinds = log.map((line) => line.split(':')[0]);
    expect(kinds.filter((kind) => kind === 'commit')).toHaveLength(1);

    // Walk the sequence and require the invariant the protocol promises.
    let outstanding = 0;
    for (const line of log) {
      const kind = line.split(':')[0];
      if (kind === 'preview') outstanding += 1;
      else outstanding -= 1;
      expect(outstanding, `two previews were live at once: ${log.join(' / ')}`).toBeLessThanOrEqual(
        1,
      );
      expect(outstanding, `something was cancelled twice: ${log.join(' / ')}`).toBeGreaterThanOrEqual(
        0,
      );
    }
    expect(outstanding, 'a preview was left open').toBe(0);

    // The `restore` field carries the value that was committed when the preview began.
    const firstPreview = log.find((line) => line.startsWith('preview:'));
    expect(firstPreview?.split(':')[3]).toBe(styleValues[0]);
  });

  test('an unavailable item never previews, because a preview it cannot keep is a broken promise', async ({
    page,
  }) => {
    await open(page, previewStory);
    const before = await readTarget(page);
    const index = styleValues.indexOf(unavailableStyleValue);
    expect(index).toBeGreaterThan(0);

    await clearLog(page);
    await stripCell(page, index).hover();
    await page.waitForTimeout(pointerPreviewSettleDelay + 120);
    expect(await protocolLog(page)).toEqual([]);
    expect(await readTarget(page)).toEqual(before);

    // And it refuses to commit, while staying reachable by the keyboard.
    //
    // ⚠ `force`, because Playwright reads `aria-disabled="true"` as *not enabled* and will wait for
    // sixty seconds rather than click it. That refusal is the harness being right about what the
    // markup says — which is itself worth knowing — and it is exactly why the click has to be
    // forced here: the thing under test is whether **the component** refuses, not whether the test
    // runner does.
    await stripCell(page, index).click({ force: true });
    await settle(page);
    expect(await protocolLog(page)).toEqual([]);
    await stripCell(page, index).focus();
    await page.keyboard.press('Enter');
    await settle(page);
    expect(await protocolLog(page)).toEqual([]);
    await expect(stripCell(page, index)).toBeFocused();
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 2. Coalescing — and the run that proves the assertion is measuring it
// ─────────────────────────────────────────────────────────────────────────────

test.describe('rapid traversal is coalesced', () => {
  /**
   * Sweep the pointer across a run of cells as fast as Playwright will move it.
   *
   * ⚠ It runs **backwards, from the last cell to the second**, and that is not arbitrary: the
   * fixture's last item is the unavailable one, which never previews at all. A forward traversal
   * therefore ends on a cell that emits nothing, and *"one preview, not eight"* would be satisfied
   * by zero — which is the vacuous pass this whole file is written to refuse.
   */
  const traversal = [7, 6, 5, 4, 3, 2, 1];

  async function traverse(page: Page): Promise<void> {
    for (const index of traversal) {
      const box = await stripCell(page, index).boundingBox();
      expect(box).not.toBeNull();
      if (box === null) continue;
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    }
  }

  test('crossing seven cells quickly emits one preview, not seven', async ({ page }) => {
    await open(page, previewStory);
    await clearLog(page);
    await traverse(page);
    await page.waitForTimeout(pointerPreviewSettleDelay + 120);

    const log = await protocolLog(page);
    const previews = log.filter((line) => line.startsWith('preview:'));
    expect(
      previews.length,
      `expected the traversal to coalesce: ${log.join(' / ')}`,
    ).toBeLessThanOrEqual(2);
    // And there **is** one — a component that previewed nothing would satisfy the line above.
    expect(previews.length).toBeGreaterThan(0);
    // The one that survived is the cell the pointer stopped on, not the first it crossed.
    expect(previews[previews.length - 1]).toContain(`preview:${String(styleValues[1])}`);
  });

  test('the anti-vacuity run: with the coalescing removed the same traversal emits one each', async ({
    page,
  }) => {
    // ⚠ MJXOFF-184: *"if this passes while the other test also passes, nothing is being measured."*
    // Without this, the assertion above would be satisfied by a component that never emits a preview
    // on hover at all — which is the exact defect the whole child exists to prevent.
    await open(page, previewStory);
    await page.evaluate((id) => {
      document.querySelector(`#${id}`)?.setAttribute('preview-delay', '0');
    }, galleryTestIds.gallery);
    await clearLog(page);
    await traverse(page);
    await page.waitForTimeout(120);

    const log = await protocolLog(page);
    const previews = log.filter((line) => line.startsWith('preview:'));
    expect(
      previews.length,
      'with the settle delay at zero every crossed cell must request a preview; if this is small ' +
        'too then the coalescing assertion above is measuring silence',
    ).toBeGreaterThanOrEqual(5);
  });

  test('however many requests are emitted, one preview is outstanding at most', async ({ page }) => {
    // The invariant that protects a renderer on the *keyboard* path, where there is no settle at
    // all: ten arrow presses emit ten requests and nine cancels.
    await open(page, previewStory);
    await clearLog(page);
    await stripCell(page, 0).focus();
    for (let step = 0; step < 6; step += 1) {
      await page.keyboard.press('ArrowRight');
    }
    await settle(page);

    const log = await protocolLog(page);
    expect(log.filter((line) => line.startsWith('preview:')).length).toBeGreaterThanOrEqual(5);
    let outstanding = 0;
    for (const line of log) {
      outstanding += line.startsWith('preview:') ? 1 : -1;
      expect(outstanding).toBeLessThanOrEqual(1);
    }
    expect(
      await page.evaluate((id) => {
        const element = document.querySelector(`#${id}`) as unknown as { outstandingPreviews: number };
        return element.outstandingPreviews;
      }, galleryTestIds.gallery),
    ).toBe(1);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 3. Virtualisation, asserted on a node count
// ─────────────────────────────────────────────────────────────────────────────

test.describe('a four-hundred-item gallery does not build four hundred cells', () => {
  test('the item descriptors render nothing at all, and the cells are a window', async ({
    page,
  }) => {
    await open(page, largeStory);
    const measured = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        builtCellsIn(name: string): number;
        expectedCellsIn(name: string): number;
        columnsIn(name: string): number;
        shadowRoot: ShadowRoot;
      };
      const items = [...document.querySelectorAll('mjx-gallery-item')];
      return {
        items: items.length,
        // ⚠ The claim being checked: an item that is not on screen has **no rendered descendants**,
        // because its art is in an inert fragment. A design that hid them instead would have four
        // hundred subtrees here.
        itemChildren: items.reduce((total, item) => total + item.childElementCount, 0),
        itemBoxes: items.reduce((total, item) => total + item.getClientRects().length, 0),
        built: element.builtCellsIn('strip'),
        expected: element.expectedCellsIn('strip'),
        columns: element.columnsIn('strip'),
        // The strip's cells alone: the expanded surface is in the same shadow tree and is planned
        // whether or not it is showing, so a query across both is a count of two windows.
        cellsInDom: element.shadowRoot.querySelectorAll(
          '.surface[data-surface="strip"] .cell',
        ).length,
        totalElements: document.querySelectorAll('*').length,
        scrollHeight: (element.shadowRoot.querySelector('.viewport') as HTMLElement).scrollHeight,
        clientHeight: (element.shadowRoot.querySelector('.viewport') as HTMLElement).clientHeight,
      };
    }, galleryTestIds.large);

    expect(measured.items).toBe(largeGalleryItemCount);
    expect(measured.itemChildren, 'an item must render nothing of its own').toBe(0);
    expect(measured.itemBoxes, 'an item must occupy no layout box').toBe(0);

    // The number to compare against comes from the *plan*, not from the renderer — the component
    // reports both and they must agree.
    expect(measured.built).toBe(measured.expected);
    expect(measured.built).toBeGreaterThan(0);
    expect(measured.built).toBeLessThan(largeGalleryItemCount / 4);
    expect(measured.cellsInDom).toBe(measured.built);

    // A whole page of four hundred built cells would be well over two thousand elements.
    expect(measured.totalElements).toBeLessThan(1200);

    // And the scrollbar is honest: the sizer is as tall as every row, built or not.
    const rows = Math.ceil(largeGalleryItemCount / measured.columns);
    expect(measured.scrollHeight).toBeGreaterThan(measured.clientHeight * (rows / 4));
  });

  test('scrolling builds the rows it reveals and drops the ones it leaves', async ({ page }) => {
    await open(page, largeStory);
    const first = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      return [...element.shadowRoot.querySelectorAll('.cell')].map((cell) =>
        cell.getAttribute('data-index'),
      );
    }, galleryTestIds.large);

    await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      const viewport = element.shadowRoot.querySelector('.viewport') as HTMLElement;
      viewport.scrollTop = viewport.scrollHeight / 2;
    }, galleryTestIds.large);
    await settle(page);

    const later = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        shadowRoot: ShadowRoot;
        builtCellsIn(name: string): number;
        expectedCellsIn(name: string): number;
      };
      return {
        indices: [...element.shadowRoot.querySelectorAll('.cell')].map((cell) =>
          cell.getAttribute('data-index'),
        ),
        built: element.builtCellsIn('strip'),
        expected: element.expectedCellsIn('strip'),
      };
    }, galleryTestIds.large);

    // The window moved: it is a different set of items, not more of them.
    expect(later.indices).not.toEqual(first);
    // The active row is kept in the window, so the two sets overlap by at most that one row.
    expect(later.indices.length).toBeLessThan(largeGalleryItemCount / 4);
    expect(later.built).toBe(later.expected);
  });

  test('the flyout of a large gallery is virtualised too, and its plan has sections in it', async ({
    page,
  }) => {
    await open(page, largeStory);
    await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      (element.shadowRoot.querySelector('[part="expand"]') as HTMLElement).click();
    }, galleryTestIds.large);
    await settlePaint(page);

    const measured = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        builtCellsIn(name: string): number;
        expectedCellsIn(name: string): number;
        planIn(name: string): { kind: string }[];
        presentation: string;
        shadowRoot: ShadowRoot;
      };
      return {
        presentation: element.presentation,
        built: element.builtCellsIn('expanded'),
        expected: element.expectedCellsIn('expanded'),
        headings: element.planIn('expanded').filter((row) => row.kind === 'heading').length,
        groups: element.shadowRoot.querySelectorAll(
          '.surface[data-surface="expanded"] [role="group"]',
        ).length,
      };
    }, galleryTestIds.large);

    expect(measured.presentation).toBe('flyout');
    expect(measured.built).toBe(measured.expected);
    expect(measured.built).toBeLessThan(largeGalleryItemCount / 4);
    // Four categories in the fixture, so four heading rows in the plan — and however many of them
    // the window happens to have reached are real `role="group"` elements.
    expect(measured.headings).toBe(4);
    expect(measured.groups).toBeGreaterThan(0);
  });

  test('the plan the component built is the plan the model says', async ({ page }) => {
    await open(page, largeStory);
    const reported = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        planIn(name: string): unknown[];
        columnsIn(name: string): number;
        windowIn(name: string): { firstRow: number; lastRow: number };
      };
      return {
        plan: element.planIn('strip'),
        columns: element.columnsIn('strip'),
        window: element.windowIn('strip'),
      };
    }, galleryTestIds.large);

    const categories = Array.from({ length: largeGalleryItemCount }, () => 'x');
    const expectedPlan = galleryRowPlan(categories, reported.columns, false);
    expect(reported.plan).toEqual(expectedPlan);
    expect(cellsInWindow(expectedPlan, reported.window)).toBeGreaterThan(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 4. Two geometries, one selection
// ─────────────────────────────────────────────────────────────────────────────

test.describe('the strip and the flyout never disagree about which item is selected', () => {
  async function expand(page: Page, id: string): Promise<void> {
    await page.evaluate((selector) => {
      const element = document.querySelector(`#${selector}`) as unknown as { shadowRoot: ShadowRoot };
      (element.shadowRoot.querySelector('[part="expand"]') as HTMLElement).click();
    }, id);
    await settlePaint(page);
  }

  async function selectionState(page: Page, id: string): Promise<unknown> {
    return page.evaluate((selector) => {
      const element = document.querySelector(`#${selector}`) as unknown as {
        value: string;
        activeIndex: number;
        selectedIndex: number;
        shadowRoot: ShadowRoot;
      };
      const marked = (surface: string): (string | null)[] =>
        [
          ...element.shadowRoot.querySelectorAll(
            `.surface[data-surface="${surface}"] .cell[aria-selected="true"]`,
          ),
        ].map((cell) => cell.getAttribute('data-value'));
      const tabStops = (surface: string): (string | null)[] =>
        [
          ...element.shadowRoot.querySelectorAll(
            `.surface[data-surface="${surface}"] .cell[tabindex="0"]`,
          ),
        ].map((cell) => cell.getAttribute('data-index'));
      return {
        value: element.value,
        activeIndex: element.activeIndex,
        selectedIndex: element.selectedIndex,
        stripSelected: marked('strip'),
        expandedSelected: marked('expanded'),
        stripStops: tabStops('strip'),
        expandedStops: tabStops('expanded'),
      };
    }, id);
  }

  test('a selection made in the strip is the selection the flyout shows, and back again', async ({
    page,
  }) => {
    await open(page, previewStory);
    await stripCell(page, 5).click();
    await settlePaint(page);

    const inStrip = (await selectionState(page, galleryTestIds.gallery)) as {
      value: string;
      stripSelected: string[];
      expandedSelected: string[];
      stripStops: string[];
      expandedStops: string[];
      activeIndex: number;
    };
    expect(inStrip.value).toBe(styleValues[5]);
    expect(inStrip.stripSelected).toEqual([styleValues[5]]);
    expect(inStrip.expandedSelected).toEqual([styleValues[5]]);
    // Exactly one roving tab stop per surface, and they are the same item.
    expect(inStrip.stripStops).toHaveLength(1);
    expect(inStrip.expandedStops).toHaveLength(1);
    expect(inStrip.stripStops).toEqual(inStrip.expandedStops);

    await expand(page, galleryTestIds.gallery);
    const opened = (await selectionState(page, galleryTestIds.gallery)) as typeof inStrip;
    expect(opened.value).toBe(styleValues[5]);
    expect(opened.expandedSelected).toEqual([styleValues[5]]);
    expect(opened.activeIndex).toBe(inStrip.activeIndex);
    // And the flyout put the keyboard on it.
    await expect(
      page.locator(`#${galleryTestIds.gallery}`).locator(`#expanded-cell-${String(5)}`),
    ).toBeFocused();

    // Now choose something else *in the flyout* and require the strip to agree on the way back.
    await page.locator(`#${galleryTestIds.gallery}`).locator('#expanded-cell-2').click();
    await settlePaint(page);
    const collapsed = (await selectionState(page, galleryTestIds.gallery)) as typeof inStrip;
    expect(collapsed.value).toBe(styleValues[2]);
    expect(collapsed.stripSelected).toEqual([styleValues[2]]);
    expect(collapsed.stripStops).toEqual(collapsed.expandedStops);
    await expect(page.locator(`#${galleryTestIds.gallery}`)).not.toHaveAttribute('expanded', '');
  });

  test('while the flyout is open the strip is inert, so the same options are not offered twice', async ({
    page,
  }) => {
    await open(page, previewStory);
    await expand(page, galleryTestIds.gallery);
    const inert = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      const strip = element.shadowRoot.querySelector('.surface[data-surface="strip"]');
      return {
        strip: (strip as HTMLElement).inert,
        listboxes: element.shadowRoot.querySelectorAll('[role="listbox"]').length,
      };
    }, galleryTestIds.gallery);
    expect(inert.strip).toBe(true);
    expect(inert.listboxes).toBe(2);
  });

  test('an item scrolled far out of the strip’s window is still reachable and still selectable', async ({
    page,
  }) => {
    // The transition in the direction that is easy to get wrong: a value chosen in the flyout that
    // the strip has never built.
    await open(page, largeStory);
    await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { value: string };
      element.value = 'theme-250';
    }, galleryTestIds.large);
    await settlePaint(page);

    const state = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        value: string;
        selectedIndex: number;
        shadowRoot: ShadowRoot;
      };
      return {
        value: element.value,
        selectedIndex: element.selectedIndex,
        // It is *not* built, because it is 250 items down — and that is correct, not a failure.
        builtSelected: element.shadowRoot.querySelectorAll('.cell[aria-selected="true"]').length,
        // Nothing is left wearing a stale mark either, which is what `render()` re-syncs.
        marked: [...element.shadowRoot.querySelectorAll('.cell')].filter(
          (cell) => cell.getAttribute('aria-selected') === 'true',
        ).length,
      };
    }, galleryTestIds.large);
    expect(state.value).toBe('theme-250');
    expect(state.selectedIndex).toBe(250);
    expect(state.builtSelected).toBe(0);
    expect(state.marked, 'a cell that is no longer selected must not keep its mark').toBe(0);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 5. Two-dimensional keyboard navigation
// ─────────────────────────────────────────────────────────────────────────────

test.describe('the keyboard walks the grid in two dimensions', () => {
  async function activeIndex(page: Page): Promise<number> {
    return page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { activeIndex: number };
      return element.activeIndex;
    }, galleryTestIds.gallery);
  }

  async function columns(page: Page): Promise<number> {
    return page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        columnsIn(name: string): number;
      };
      return element.columnsIn('strip');
    }, galleryTestIds.gallery);
  }

  test('every key lands where nextGalleryIndex says, over the fixture’s ragged last row', async ({
    page,
  }) => {
    await open(page, previewStory);
    const width = await columns(page);
    expect(width).toBeGreaterThan(1);
    // The fixture is eight items; a ragged tail needs the column count not to divide it.
    const grid = { count: styleValues.length, columns: width, rowsPerPage: 1 };

    const script: readonly { key: string; action: Parameters<typeof nextGalleryIndex>[0] }[] = [
      { key: 'ArrowRight', action: 'next' },
      { key: 'ArrowRight', action: 'next' },
      { key: 'ArrowDown', action: 'rowDown' },
      { key: 'ArrowDown', action: 'rowDown' },
      { key: 'ArrowUp', action: 'rowUp' },
      { key: 'ArrowLeft', action: 'previous' },
      { key: 'End', action: 'last' },
      { key: 'ArrowRight', action: 'next' },
      { key: 'ArrowDown', action: 'rowDown' },
      { key: 'Home', action: 'first' },
      { key: 'ArrowUp', action: 'rowUp' },
      { key: 'PageDown', action: 'pageDown' },
      { key: 'PageUp', action: 'pageUp' },
    ];

    await stripCell(page, 0).focus();
    let expected = 0;
    for (const step of script) {
      const rowsPerPage = await page.evaluate((id) => {
        const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
        const viewport = element.shadowRoot.querySelector(
          '.surface[data-surface="strip"] .viewport',
        ) as HTMLElement;
        const row = element.shadowRoot.querySelector('.row[data-kind="cells"]') as HTMLElement;
        return Math.max(1, Math.round(viewport.clientHeight / Math.max(1, row.offsetHeight)));
      }, galleryTestIds.gallery);
      expected = nextGalleryIndex(step.action, expected, { ...grid, rowsPerPage });
      await page.keyboard.press(step.key);
      await settle(page);
      expect(await activeIndex(page), `${step.key} → ${step.action}`).toBe(expected);
      // And the cell the model names is the cell that actually holds focus.
      await expect(stripCell(page, expected)).toBeFocused();
    }
  });

  test('there is exactly one tab stop, and Tab leaves the gallery rather than cycling in it', async ({
    page,
  }) => {
    await open(page, previewStory);
    await stripCell(page, 2).focus();
    await settle(page);
    const stops = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      return element.shadowRoot.querySelectorAll(
        '.surface[data-surface="strip"] .cell[tabindex="0"]',
      ).length;
    }, galleryTestIds.gallery);
    expect(stops).toBe(1);

    await page.keyboard.press('Tab');
    await settle(page);
    const stillInside = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`);
      return element?.shadowRoot?.activeElement?.classList.contains('cell') === true;
    }, galleryTestIds.gallery);
    expect(stillInside).toBe(false);
  });

  test('under RTL the inline arrows mirror', async ({ page }) => {
    await open(page, rtlStory);
    const read = async (): Promise<number> =>
      page.evaluate(() => {
        const element = document.querySelector('#rtl-gallery') as unknown as { activeIndex: number };
        return element.activeIndex;
      });
    await page
      .locator('#rtl-gallery')
      .locator('#strip-cell-2')
      .focus();
    await page.keyboard.press('ArrowLeft');
    await settle(page);
    expect(await read(), 'Arrow Left is *next* when the line runs right to left').toBe(3);
    await page.keyboard.press('ArrowRight');
    await settle(page);
    expect(await read()).toBe(2);
  });

  test('Alt + Arrow Down opens the flyout from the keyboard', async ({ page }) => {
    await open(page, previewStory);
    await stripCell(page, 0).focus();
    await page.keyboard.press('Alt+ArrowDown');
    await settlePaint(page);
    await expect(page.locator(`#${galleryTestIds.gallery}`)).toHaveAttribute('expanded', '');
    await page.keyboard.press('Escape');
    await settlePaint(page);
    await expect(page.locator(`#${galleryTestIds.gallery}`)).not.toHaveAttribute('expanded', '');
    // Escape returns focus to the button that opened it.
    const onExpand = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`);
      return element?.shadowRoot?.activeElement?.getAttribute('part') === 'expand';
    }, galleryTestIds.gallery);
    expect(onExpand).toBe(true);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 6. The states matrix: correspondence, then distinctness
// ─────────────────────────────────────────────────────────────────────────────

/** What one cell computed, as one comparable object. */
interface CellPaint {
  background: string;
  borderColor: string;
  borderStyle: string;
  color: string;
  weight: string;
  opacity: string;
  shadow: string;
  outlineWidth: string;
  /**
   * ⚠ The weight of the **caption**, not only of the cell it sits in.
   *
   * Reading it off the cell alone leaves the hole MJXOFF-182's accident drives through for the
   * fifth time: a type-role class on the caption declares its own `font-weight`, so the cell can
   * report the state's bold while the words a person actually reads render medium.
   */
  captionWeight: string;
}

/** Every states-matrix cell, keyed by the state it is showing. */
async function readCells(page: Page): Promise<Record<string, CellPaint>> {
  return page.evaluate((attribute) => {
    const read = (element: Element): CellPaint => {
      const style = getComputedStyle(element);
      const caption = element.querySelector('.caption');
      return {
        background: style.backgroundColor,
        borderColor: style.borderTopColor,
        borderStyle: style.borderTopStyle,
        color: style.color,
        weight: style.fontWeight,
        opacity: style.opacity,
        shadow: style.boxShadow,
        outlineWidth: style.outlineWidth,
        captionWeight: caption === null ? '' : getComputedStyle(caption).fontWeight,
      };
    };
    const found: Record<string, CellPaint> = {};
    for (const holder of document.querySelectorAll(`[${attribute}]`)) {
      const state = holder.getAttribute(attribute) ?? '';
      // The **strip's** cell. Both surfaces are always planned, so `.cell` alone would sometimes be
      // the flyout's copy of the same item — a box that is `display: none` and computes nothing.
      const cell = holder
        .querySelector('mjx-gallery')
        ?.shadowRoot?.querySelector('.surface[data-surface="strip"] .cell');
      if (cell !== null && cell !== undefined) found[state] = read(cell);
    }
    return found;
  }, galleryStateCellAttribute);
}

test.describe('the states matrix, on what the browser computed', () => {
  for (const scheme of schemes) {
    test(`${scheme}: what it computed is what the model says`, async ({ page }) => {
      await open(page, matrixStory, { theme: scheme });
      await settlePaint(page);
      const cells = await readCells(page);

      const measurable = galleryCellStateNames.filter((state) => state !== 'focus');
      expect(Object.keys(cells).sort()).toEqual([...galleryCellStateNames].sort());

      for (const state of measurable) {
        const paint = galleryCellPaint(state as GalleryCellState);
        const measured = cells[state];
        expect(measured, state).toBeDefined();
        if (measured === undefined) continue;
        expect(measured.background, `${state} background`).toBe(
          expectedColor(paint.background, scheme),
        );
        expect(measured.borderColor, `${state} border colour`).toBe(
          expectedColor(paint.borderColor, scheme),
        );
        expect(measured.borderStyle, `${state} border style`).toBe(paint.borderStyle);
        expect(measured.color, `${state} text`).toBe(expectedColor(paint.text, scheme));
        expect(
          Number(measured.weight),
          `${state} weight is ${measured.weight}, model says ${paint.weight}`,
        ).toBe(Number(tokens.fontWeight[paint.weight]));
        expect(measured.captionWeight, `${state} caption weight`).toBe(measured.weight);
        expect(Number(measured.opacity), `${state} opacity`).toBeCloseTo(paint.opacity, 2);
      }
    });

    test(`${scheme}: every pair of states differs`, async ({ page }) => {
      await open(page, matrixStory, { theme: scheme });
      await settlePaint(page);
      const cells = await readCells(page);
      const measurable = galleryCellStateNames.filter((state) => state !== 'focus');
      const fingerprints = new Map<string, string>();
      for (const state of measurable) {
        const measured = cells[state];
        expect(measured, state).toBeDefined();
        fingerprints.set(
          state,
          JSON.stringify({
            background: measured?.background,
            borderColor: measured?.borderColor,
            borderStyle: measured?.borderStyle,
            color: measured?.color,
            weight: measured?.weight,
            opacity: measured?.opacity,
            shadow: measured?.shadow,
          }),
        );
      }
      for (const [first, firstPrint] of fingerprints) {
        for (const [second, secondPrint] of fingerprints) {
          if (first >= second) continue;
          expect(firstPrint, `${first} and ${second} render identically`).not.toBe(secondPrint);
        }
      }
    });
  }

  test('a forced state computes exactly what a real pointer produces', async ({ page }) => {
    // The affordance and the real thing share one declaration block, so there is nothing for a
    // forced state to drift from — and this drives a real pointer anyway, because a structural
    // guarantee nobody has watched hold is a guarantee about a file.
    await open(page, matrixStory);
    await settlePaint(page);
    const forced = await readCells(page);
    // ⚠ The **cell**, not the gallery. A gallery's box includes its affordance rail, so hovering the
    // host puts the pointer wherever the centre of the whole component happens to be — which was a
    // button, and the cell stayed at rest while the assertion said the forced state was wrong.
    await page
      .locator(`[${galleryStateCellAttribute}="rest"] mjx-gallery`)
      .locator('#strip-cell-0')
      .hover();
    await settlePaint(page);
    const hovered = await page.evaluate((attribute) => {
      const holder = document.querySelector(`[${attribute}="rest"]`);
      const cell = holder
        ?.querySelector('mjx-gallery')
        ?.shadowRoot?.querySelector('.surface[data-surface="strip"] .cell');
      if (cell === null || cell === undefined) return null;
      const style = getComputedStyle(cell);
      return { background: style.backgroundColor, borderColor: style.borderTopColor };
    }, galleryStateCellAttribute);
    expect(hovered).not.toBeNull();
    expect(hovered?.background).toBe(forced['hover']?.background);
    expect(hovered?.borderColor).toBe(forced['hover']?.borderColor);
  });

  test('the keyboard highlight is the foundations’ ring and nothing of the gallery’s own', async ({
    page,
  }) => {
    await open(page, matrixStory);
    await page
      .locator(`[${galleryStateCellAttribute}="focus"] mjx-gallery`)
      .locator('#strip-cell-0')
      .focus();
    await settlePaint(page);
    const measured = await page.evaluate((attribute) => {
      const holder = document.querySelector(`[${String(attribute)}="focus"]`);
      const cell = holder
        ?.querySelector('mjx-gallery')
        ?.shadowRoot?.querySelector('.surface[data-surface="strip"] .cell');
      if (cell === null || cell === undefined) return null;
      const style = getComputedStyle(cell);
      return { outlineWidth: style.outlineWidth, background: style.backgroundColor };
    }, galleryStateCellAttribute);
    expect(measured).not.toBeNull();
    expect(Number.parseFloat(measured?.outlineWidth ?? '0')).toBeGreaterThan(0);
    // `focus` paints no fill of its own — a second highlight would be a second focus treatment.
    expect(measured?.background).toBe(expectedColor('transparent', 'light'));
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 7. The item is a slot
// ─────────────────────────────────────────────────────────────────────────────

test.describe('three genuinely different item kinds render', () => {
  test('a text miniature, a shape and a swatch all reach a cell, and all paint', async ({
    page,
  }) => {
    await open(page, kindsStory);
    await settlePaint(page);
    const measured = await page.evaluate(() => {
      const read = (kind: string): Record<string, unknown> => {
        const holder = document.querySelector(`[data-item-kind="${kind}"]`);
        const cell = holder
          ?.querySelector('mjx-gallery')
          ?.shadowRoot?.querySelector('.surface[data-surface="strip"] .cell');
        const art = cell?.querySelector('.art');
        const child = art?.firstElementChild ?? null;
        const box = child?.getBoundingClientRect();
        return {
          tag: child?.tagName ?? null,
          // ⚠ The art is cloned into a **shadow root**, so a document class does not reach it. This
          // is the assertion that catches the failure that produced eight captions and no colour.
          painted:
            child === null
              ? null
              : `${getComputedStyle(child).backgroundColor}|${getComputedStyle(child).fontSize}`,
          width: box?.width ?? 0,
          height: box?.height ?? 0,
          // The cell's own name comes from `label`, never from the art.
          name: cell?.getAttribute('aria-label') ?? null,
          artHidden: art?.getAttribute('aria-hidden'),
        };
      };
      return {
        textMiniature: read('textMiniature'),
        shape: read('shape'),
        swatch: read('swatch'),
      };
    });

    expect(measured.textMiniature['tag']).toBe('SPAN');
    expect(measured.shape['tag']).toBe('svg');
    expect(measured.swatch['tag']).toBe('SPAN');

    for (const kind of ['textMiniature', 'shape', 'swatch'] as const) {
      const item = measured[kind];
      expect(Number(item['width']), `${kind} art has no width`).toBeGreaterThan(0);
      expect(Number(item['height']), `${kind} art has no height`).toBeGreaterThan(0);
      expect(item['artHidden'], `${kind} art must not be announced`).toBe('true');
      expect(String(item['name'] ?? ''), `${kind} cell has no name`).not.toBe('');
    }

    // A swatch is a block of colour, so it must actually have one — the specific failure the
    // shadow-root note in `gallery-item.ts` records.
    expect(measured.swatch['painted']).not.toContain('rgba(0, 0, 0, 0)');
    // And three kinds that all rendered the same thing would prove nothing about the slot.
    const shapes = new Set([
      measured.textMiniature['tag'],
      measured.shape['tag'],
      measured.swatch['painted'],
    ]);
    expect(shapes.size).toBe(3);
  });

  test('the cell’s accessible name is the item’s label, not the words drawn inside it', async ({
    page,
  }) => {
    await open(page, previewStory);
    const names = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      return [...element.shadowRoot.querySelectorAll('.surface[data-surface="strip"] .cell')].map(
        (cell) => cell.getAttribute('aria-label'),
      );
    }, galleryTestIds.gallery);
    expect(names[0]).toBe('Normal');
    expect(names.every((name) => (name ?? '').includes('AaBbCc'))).toBe(false);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// 8. Presentation, placement, degradation and the touch sheet
// ─────────────────────────────────────────────────────────────────────────────

test.describe('the three presentations, and the placement primitive behind two of them', () => {
  test('the flyout goes where placeFloating says, from the component’s own recorded inputs', async ({
    page,
  }) => {
    await open(page, previewStory);
    await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      (element.shadowRoot.querySelector('[part="expand"]') as HTMLElement).click();
    }, galleryTestIds.gallery);
    await settlePaint(page);

    const measured = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        presentation: string;
        placement: Record<string, number | string | boolean>;
        anchorRect: { x: number; y: number; width: number; height: number };
        boundaryRect: { x: number; y: number; width: number; height: number };
        shadowRoot: ShadowRoot;
      };
      const box = element.shadowRoot.querySelector(
        '.surface[data-surface="expanded"]',
      ) as HTMLElement;
      const rect = box.getBoundingClientRect();
      const style = getComputedStyle(box);
      return {
        presentation: element.presentation,
        placement: element.placement,
        anchor: element.anchorRect,
        boundary: element.boundaryRect,
        rect: { x: rect.x, y: rect.y, width: rect.width, height: rect.height },
        gap: Number.parseFloat(style.getPropertyValue('--mjx-floating-gap')),
        popover: box.getAttribute('popover'),
        open: box.matches(':popover-open'),
      };
    }, galleryTestIds.gallery);

    expect(measured.presentation).toBe(
      galleryPresentationAt(containerPresets.desktop, true),
    );
    // The top layer, and `manual` — an `auto` popover would close a collapsed ribbon group the
    // moment its gallery opened.
    expect(measured.popover).toBe('manual');
    expect(measured.open).toBe(true);
    expect(measured.gap).toBeGreaterThan(0);

    // Re-run the model **in Node** over the component's own inputs and require the same answer.
    const recomputed = placeFloating({
      anchor: measured.anchor,
      floating: { width: measured.rect.width, height: measured.rect.height },
      boundary: measured.boundary,
      side: 'blockEnd',
      align: 'start',
      direction: 'ltr',
      gap: measured.gap,
    });
    expect(Math.round(Number(measured.placement['x']))).toBe(Math.round(recomputed.x));
    expect(measured.placement['side']).toBe(recomputed.side);

    // And it is inside what clips it, which is the frame rather than the window.
    expect(measured.rect.x).toBeGreaterThanOrEqual(measured.boundary.x - 1);
    expect(measured.rect.x + measured.rect.width).toBeLessThanOrEqual(
      measured.boundary.x + measured.boundary.width + 1,
    );
  });

  test('at a phone width the flyout is a sheet: pinned to the bottom edge, at full width', async ({
    page,
  }) => {
    await open(page, phoneStory, { containerPreset: 'phone' });
    expect(containerPresets.phone).toBeLessThanOrEqual(gallerySheetAtOrBelow);
    await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      (element.shadowRoot.querySelector('[part="expand"]') as HTMLElement).click();
    }, galleryTestIds.gallery);
    await settlePaint(page);

    const measured = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as {
        presentation: string;
        shadowRoot: ShadowRoot;
      };
      const box = element.shadowRoot.querySelector(
        '.surface[data-surface="expanded"]',
      ) as HTMLElement;
      const frame = document
        .querySelector('mjx-resizable-container')
        ?.shadowRoot?.querySelector('.frame') as HTMLElement;
      const boxRect = box.getBoundingClientRect();
      const frameRect = frame.getBoundingClientRect();
      const style = getComputedStyle(box);
      return {
        presentation: element.presentation,
        box: { x: boxRect.x, width: boxRect.width, bottom: boxRect.bottom },
        frame: { x: frameRect.x, width: frameRect.width, bottom: frameRect.bottom },
        radiusEnd: style.borderEndStartRadius,
        radiusStart: style.borderStartStartRadius,
      };
    }, galleryTestIds.gallery);

    expect(measured.presentation).toBe('sheet');
    expect(measured.box.width).toBeCloseTo(measured.frame.width, 0);
    expect(measured.box.x).toBeCloseTo(measured.frame.x, 0);
    expect(measured.box.bottom).toBeCloseTo(measured.frame.bottom, 0);
    // A sheet squares the edge it is pinned to and rounds the one it does not.
    expect(Number.parseFloat(measured.radiusEnd)).toBe(0);
    expect(Number.parseFloat(measured.radiusStart)).toBeGreaterThan(0);
  });

  test('press and hold previews on touch, and a tap commits — and the two are different', async ({
    page,
  }) => {
    await open(page, phoneStory, { containerPreset: 'phone' });
    const before = await readTarget(page);
    const box = await stripCell(page, 3).boundingBox();
    expect(box).not.toBeNull();
    if (box === null) return;
    const x = box.x + box.width / 2;
    const y = box.y + box.height / 2;

    await clearLog(page);
    // A hold: down, wait past the long-press delay, up.
    const touch = { pointerType: 'touch' as const };
    await page.evaluate(
      ([selector, cx, cy]) => {
        const element = document.querySelector(String(selector));
        const target = element?.shadowRoot?.querySelector('#strip-cell-3') as HTMLElement;
        const options = {
          bubbles: true,
          composed: true,
          pointerType: 'touch',
          clientX: Number(cx),
          clientY: Number(cy),
          pointerId: 1,
        };
        target.dispatchEvent(new PointerEvent('pointerdown', options));
      },
      [`#${galleryTestIds.gallery}`, x, y] as const,
    );
    await page.waitForTimeout(galleryTouchPreviewDelay + 120);
    const held = await readTarget(page);
    expect(held, 'a held cell must preview').not.toEqual(before);
    expect((await protocolLog(page)).some((line) => line.includes(':touch:'))).toBe(true);

    await page.evaluate(
      ([selector, cx, cy]) => {
        const element = document.querySelector(String(selector));
        const target = element?.shadowRoot?.querySelector('#strip-cell-3') as HTMLElement;
        target.dispatchEvent(
          new PointerEvent('pointerup', {
            bubbles: true,
            composed: true,
            pointerType: 'touch',
            clientX: Number(cx),
            clientY: Number(cy),
            pointerId: 1,
          }),
        );
      },
      [`#${galleryTestIds.gallery}`, x, y] as const,
    );
    await settlePaint(page);
    expect(await readTarget(page), 'releasing a hold must restore exactly').toEqual(before);
    expect(touch.pointerType).toBe('touch');

    // A tap — down and up inside the hold delay — commits instead.
    await clearLog(page);
    await page.evaluate(
      ([selector, cx, cy]) => {
        const element = document.querySelector(String(selector));
        const target = element?.shadowRoot?.querySelector('#strip-cell-3') as HTMLElement;
        const options = {
          bubbles: true,
          composed: true,
          pointerType: 'touch',
          clientX: Number(cx),
          clientY: Number(cy),
          pointerId: 2,
        };
        target.dispatchEvent(new PointerEvent('pointerdown', options));
        target.dispatchEvent(new PointerEvent('pointerup', options));
      },
      [`#${galleryTestIds.gallery}`, x, y] as const,
    );
    await settlePaint(page);
    const log = await protocolLog(page);
    expect(log.filter((line) => line.startsWith('commit:'))).toHaveLength(1);
    expect(await readTarget(page)).not.toEqual(before);
  });

  test('a gallery degrades with its ribbon group, and it is measured as well as read back', async ({
    page,
  }) => {
    await open(page, groupStory);
    /**
     * ⚠ **Two assertions, and the second is the one that can catch a defect.**
     *
     * Reading `--mjx-gallery-strip-rows` back and comparing it against `galleryStripRowsFor()` is
     * comparing a generated stylesheet against the table it was generated from: they cannot
     * disagree, and this child *proved* they cannot by setting every row count to two and watching
     * the assertion stay green. So the row count is also **counted in the layout** — how many rows
     * of cells actually fit inside the viewport without scrolling — which is geometry the custom
     * property does not decide. That is `<mjx-ribbon-group>`'s rule (*"a gate that read only that
     * would be asking the implementation to grade its own homework"*) applied here.
     */
    for (const width of [1440, 900, 400]) {
      await page.evaluate((value) => {
        document.querySelector('mjx-resizable-container')?.setAttribute('width', String(value));
      }, width);
      await settlePaint(page);
      const measured = await page.evaluate(() => {
        const element = document.querySelector('#grouped-gallery') as unknown as {
          groupPresentation: string;
          shadowRoot: ShadowRoot;
        };
        const strip = element.shadowRoot.querySelector(
          '.surface[data-surface="strip"]',
        ) as HTMLElement;
        const viewport = strip.querySelector('.viewport') as HTMLElement;
        const port = viewport.getBoundingClientRect();
        const rows = new Set<number>();
        for (const cell of strip.querySelectorAll('.cell')) {
          const box = cell.getBoundingClientRect();
          if (box.top >= port.top - 1 && box.bottom <= port.bottom + 1) {
            rows.add(Math.round(box.top));
          }
        }
        const ribbon = document.querySelector('mjx-ribbon') as HTMLElement;
        return {
          group: element.groupPresentation,
          declared: Number.parseInt(
            getComputedStyle(strip).getPropertyValue('--mjx-gallery-strip-rows'),
            10,
          ),
          visibleRows: rows.size,
          cells: strip.querySelectorAll('.cell').length,
          ribbonWidth: ribbon.getBoundingClientRect().width,
        };
      });
      const expectedGroup = groupPresentationAt('primary', measured.ribbonWidth);
      expect(measured.group, `at ${String(width)}px`).toBe(expectedGroup);
      expect(measured.declared, `at ${String(width)}px`).toBe(galleryStripRowsFor(expectedGroup));
      // The fixture has more items than fit on one row at any of these widths, so a strip that
      // showed the wrong number of rows shows it here in pixels.
      expect(measured.cells).toBeGreaterThan(measured.visibleRows);
      expect(measured.visibleRows, `at ${String(width)}px, in the layout`).toBe(
        galleryStripRowsFor(expectedGroup),
      );
    }
  });

  test('every part the component publishes is a part it actually renders, and no others', async ({
    page,
  }) => {
    await open(page, previewStory);
    await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      (element.shadowRoot.querySelector('[part="expand"]') as HTMLElement).click();
    }, galleryTestIds.gallery);
    await settlePaint(page);

    const rendered = await page.evaluate((id) => {
      const element = document.querySelector(`#${id}`) as unknown as { shadowRoot: ShadowRoot };
      const found = new Set<string>();
      for (const node of element.shadowRoot.querySelectorAll('[part]')) {
        for (const part of (node.getAttribute('part') ?? '').split(/\s+/)) {
          if (part !== '') found.add(part);
        }
      }
      return [...found].sort();
    }, galleryTestIds.gallery);

    // **Set equality, in both directions.** A declared part that renders nowhere is a promise to a
    // shell that will never be kept; a rendered part nobody declared is a surface nobody documented.
    expect(rendered).toEqual([...galleryParts].sort());
  });

  test('every cell clears the accessible hit-target floor, in compact density', async ({ page }) => {
    await open(page, groupStory);
    await page.evaluate(() => {
      document.querySelector('mjx-resizable-container')?.setAttribute('width', '900');
    });
    await settlePaint(page);
    const heights = await page.evaluate(() => {
      const element = document.querySelector('#grouped-gallery') as unknown as {
        shadowRoot: ShadowRoot;
      };
      // ⚠ The **strip's** cells. The expanded surface is in the shadow tree too and is
      // `display: none` until it opens, so a query across both measures a set of zero-height boxes
      // and reports a floor violation that is really a closed popup.
      return [
        ...element.shadowRoot.querySelectorAll('.surface[data-surface="strip"] .cell'),
      ].map((cell) => cell.getBoundingClientRect().height);
    });
    expect(heights.length).toBeGreaterThan(0);
    for (const height of heights) expect(height).toBeGreaterThanOrEqual(accessibleHitTargetMinimum);
  });
});
