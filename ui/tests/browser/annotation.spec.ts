import { expect, test, type Page } from '@playwright/test';

import { builtStories, openStory } from './support.ts';
import { accessibleHitTargetMinimum } from '../../src/foundations/density.ts';
import {
  annotationStoryTitles,
  annotationTags,
  reviewAriaPattern,
  reviewPresentationProperty,
} from '../../src/annotation/annotation-model.ts';
import { authorColourProperty, authorColourSlots } from '../../src/annotation/author-colour.ts';
import { tightlyClusteredAnchors } from '../../dev/annotation-fixtures.ts';

/**
 * MJXOFF-193's annotation family, driven in a real browser.
 *
 * Three things live only here, because none of them can be proved in Node:
 *
 * * **The packing survives the cascade.** `tests/annotation.test.ts` proves the arithmetic is the
 *   optimum; this proves the component actually laid the cards out where the arithmetic said, by
 *   measuring the boxes the browser produced. A packer whose positions were overridden by a
 *   stylesheet would pass the first gate and fail this one.
 * * **The presentation the container query chose**, cross-checked against three facts the custom
 *   property does not control — U11's rule, because a gate that read only the property would be
 *   asking the implementation to grade its own homework.
 * * **The hit-target floor**, which is a promise about pixels.
 */

const tight = { title: annotationStoryTitles.reviewPane, name: 'Anchors A Few Lines Apart' } as const;
const spaced = { title: annotationStoryTitles.reviewPane, name: 'Well Spaced Anchors' } as const;
const many = { title: annotationStoryTitles.reviewPane, name: 'Three Hundred Annotations' } as const;
const phone = { title: annotationStoryTitles.reviewPane, name: 'On A Phone' } as const;
const models = { title: annotationStoryTitles.commentCard, name: 'Both Comment Models' } as const;
const thread = { title: annotationStoryTitles.commentThread, name: 'Collapsed And Expanded' } as const;
const kinds = { title: annotationStoryTitles.trackedChangeCard, name: 'The Four Kinds' } as const;

async function open(
  page: Page,
  story: { readonly title: string; readonly name: string },
  options: { containerPreset?: string; theme?: string } = {},
): Promise<void> {
  const entry = builtStories().find(
    (candidate) => candidate.title === story.title && candidate.name === story.name,
  );
  expect(entry, `${story.title} · ${story.name} is missing from the catalogue`).toBeDefined();
  if (entry === undefined) return;
  await openStory(page, entry.id, options);
  for (const tag of Object.values(annotationTags)) {
    await page.waitForFunction((name) => customElements.get(name) !== undefined, tag);
  }
  await settle(page);
}

/**
 * Two frames, and then the packing transition.
 *
 * ⚠ A card moving to a new packed offset is animated — the `documentObject` motion role, whose
 * duration is `--duration-transition` — so a box read two frames after a selection is a box in
 * mid-flight, and every packing assertion below would be measuring an intermediate position. This
 * waits the transition out rather than disabling it, because what the gate is about is where the
 * cards *end up* and the animation is part of the component under test.
 */
async function settle(page: Page): Promise<void> {
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
  await page.waitForTimeout(transitionSettleMilliseconds);
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(done));
  });
}

/** `--duration-transition` is 150ms; this is that plus room for the frame it starts on. */
const transitionSettleMilliseconds = 240;

/** Where every card ended up, as the browser laid it out. */
interface CardBox {
  readonly id: string;
  readonly top: number;
  readonly height: number;
}

async function cardBoxes(page: Page, paneId: string): Promise<CardBox[]> {
  return page.evaluate((id) => {
    const pane = document.getElementById(id);
    if (pane === null) return [];
    const column = pane.shadowRoot?.querySelector('[part="column"]');
    if (column === null || column === undefined) return [];
    const origin = column.getBoundingClientRect().top - column.scrollTop;
    return [...column.querySelectorAll('.slot')].map((card) => {
      const box = card.getBoundingClientRect();
      return {
        id: card.getAttribute('comment-id') ?? card.getAttribute('change-id') ?? '',
        top: box.top - origin,
        height: box.height,
      };
    });
  }, paneId);
}

test.describe('the margin packing reaches the DOM', () => {
  test('no two built cards overlap, on anchors a few lines apart', async ({ page }) => {
    await open(page, tight);
    // ⚠ Swept, not sampled. A margin column shows three or four of a tight cluster at a time, so a
    // single snapshot would be an overlap assertion over a pair — which is what the packer gets
    // right first and everywhere. Scrolling the whole column and requiring every snapshot to be
    // clean *and* the sweep to have seen every card is the assertion that is actually about seven.
    const seen = new Set<string>();
    for (const offset of [0, 150, 300, 450, 600, 750, 900, 1200, 1600]) {
      await page.evaluate((top) => {
        const column = document.getElementById('pane-tight')?.shadowRoot?.querySelector('[part="column"]');
        if (column instanceof HTMLElement) column.scrollTop = top;
      }, offset);
      await settle(page);
      const boxes = (await cardBoxes(page, 'pane-tight')).sort((a, b) => a.top - b.top);
      expect(boxes.length, `nothing built at ${String(offset)}`).toBeGreaterThan(0);
      for (const box of boxes) seen.add(box.id);
      for (let index = 1; index < boxes.length; index += 1) {
        const above = boxes[index - 1];
        const below = boxes[index];
        if (above === undefined || below === undefined) continue;
        expect(
          below.top,
          `${below.id} overlaps ${above.id} at scroll ${String(offset)}`,
        ).toBeGreaterThanOrEqual(above.top + above.height - 0.5);
      }
    }
    expect(seen.size).toBe(tightlyClusteredAnchors.length);
  });

  test('the component agrees with its own model about where it put them', async ({ page }) => {
    await open(page, tight);
    const model = await page.evaluate(() => {
      const pane = document.getElementById('pane-tight');
      const placed = (pane as (HTMLElement & { placedCards?: readonly { id: string; top: number }[] }) | null)
        ?.placedCards;
      return (placed ?? []).map((card) => ({ id: card.id, top: card.top }));
    });
    const boxes = await cardBoxes(page, 'pane-tight');
    expect(model.length).toBeGreaterThan(0);
    for (const box of boxes) {
      const placed = model.find((card) => card.id === box.id);
      expect(placed, `${box.id} was laid out but never packed`).toBeDefined();
      // A stylesheet that overrode the packed offset would fail exactly here, and nowhere else.
      expect(Math.abs((placed?.top ?? 0) - box.top)).toBeLessThan(2);
    }
  });

  test('the selected card takes its anchor and the others move', async ({ page }) => {
    await open(page, tight);
    const before = await cardBoxes(page, 'pane-tight');
    const selected = await page.evaluate(() => {
      const pane = document.getElementById('pane-tight') as
        | (HTMLElement & { selectIndex: (index: number) => void; placedCards: readonly { id: string; anchorTop: number; top: number }[] })
        | null;
      if (pane === null) return undefined;
      pane.selectIndex(0);
      const card = pane.placedCards.find((entry) => entry.id === pane.getAttribute('selected'));
      return card === undefined ? undefined : { id: card.id, top: card.top, anchorTop: card.anchorTop };
    });
    await settle(page);
    expect(selected).toBeDefined();
    expect(Math.abs((selected?.top ?? 1) - (selected?.anchorTop ?? 0))).toBeLessThan(0.5);
    const after = await cardBoxes(page, 'pane-tight');
    const moved = after.filter((box) => {
      const was = before.find((entry) => entry.id === box.id);
      return was !== undefined && Math.abs(was.top - box.top) > 1;
    });
    expect(moved.length, 'nothing yielded around the selection').toBeGreaterThan(0);
  });

  test('every card sits exactly at its anchor when the anchors are well spaced', async ({ page }) => {
    // The trap, in the browser: this is the fixture on which a stack is indistinguishable, and it
    // is here so a reader can see that the tight one above is doing the work.
    await open(page, spaced);
    const model = await page.evaluate(() => {
      const pane = document.getElementById('pane-spaced');
      const placed = (pane as (HTMLElement & { placedCards?: readonly { top: number; anchorTop: number }[] }) | null)
        ?.placedCards;
      return (placed ?? []).map((card) => card.top - card.anchorTop);
    });
    expect(model.length).toBeGreaterThan(0);
    for (const displacement of model) expect(Math.abs(displacement)).toBeLessThan(0.5);
  });
});

test.describe('the connector contract', () => {
  test('names every annotation, with a point on the card for the line to meet', async ({ page }) => {
    await open(page, tight);
    const anchors = await page.evaluate(() => {
      const pane = document.getElementById('pane-tight') as
        | (HTMLElement & {
            anchors: readonly {
              id: string;
              anchorTop: number;
              cardTop: number;
              cardExtent: number;
              connectorTop: number;
              side: string;
              authorSlot: number;
              kind: string;
            }[];
          })
        | null;
      return pane === null ? [] : [...pane.anchors];
    });
    expect(anchors).toHaveLength(tightlyClusteredAnchors.length);
    for (const anchor of anchors) {
      expect(anchor.id).not.toBe('');
      expect(anchor.side).toBe('inlineStart');
      expect(anchor.authorSlot).toBeGreaterThanOrEqual(0);
      expect(anchor.authorSlot).toBeLessThan(authorColourSlots.length);
      // The line lands ON the card, never on its corner and never past its end.
      expect(anchor.connectorTop).toBeGreaterThanOrEqual(anchor.cardTop);
      expect(anchor.connectorTop).toBeLessThanOrEqual(anchor.cardTop + anchor.cardExtent + 0.5);
    }
    // Every annotation is reported, including the ones the virtualiser did not build.
    const built = await cardBoxes(page, 'pane-tight');
    expect(anchors.length).toBeGreaterThanOrEqual(built.length);
  });

  test('a card reports its own anchor, standing on its own', async ({ page }) => {
    await open(page, models);
    const report = await page.evaluate(() => {
      const card = document.querySelector('mjx-comment-card[comment-id="threaded-1"]') as
        | (HTMLElement & { anchorReport: () => { id: string; anchorTop: number; model?: string } })
        | null;
      return card === null ? undefined : card.anchorReport();
    });
    expect(report?.id).toBe('threaded-1');
    expect(report?.anchorTop).toBe(120);
    expect(report?.model).toBe('threaded');
  });

  test('every annotation reports its anchor for a document of three hundred', async ({ page }) => {
    await open(page, many);
    const counts = await page.evaluate(() => {
      const pane = document.getElementById('pane-many') as
        | (HTMLElement & { anchors: readonly unknown[]; builtCardCount: number })
        | null;
      return pane === null ? undefined : { anchors: pane.anchors.length, built: pane.builtCardCount };
    });
    expect(counts?.anchors).toBe(300);
    expect(counts?.built ?? 0).toBeGreaterThan(0);
  });
});

test.describe('virtualisation', () => {
  test('three hundred annotations build a windowful of nodes, not three hundred', async ({ page }) => {
    await open(page, many);
    const report = await page.evaluate(() => {
      const pane = document.getElementById('pane-many') as
        | (HTMLElement & { builtCardCount: number; expectedCardCount: number })
        | null;
      const column = pane?.shadowRoot?.querySelector('[part="column"]') ?? null;
      return {
        built: pane?.builtCardCount ?? 0,
        expected: pane?.expectedCardCount ?? 0,
        nodes: column === null ? 0 : column.querySelectorAll('.slot').length,
        sizer: column === null ? 0 : (column.querySelector('.sizer') as HTMLElement | null)?.getBoundingClientRect().height ?? 0,
      };
    });
    // Both ends: a ceiling satisfied by zero is no ceiling.
    expect(report.nodes).toBeGreaterThan(0);
    expect(report.nodes).toBeLessThan(30);
    expect(report.nodes).toBe(report.built);
    expect(report.built).toBe(report.expected);
    // The scrollbar is as long as every card, not as long as the built ones.
    expect(report.sizer).toBeGreaterThan(300 * 40);
  });

  test('scrolling changes which cards exist without changing how many', async ({ page }) => {
    await open(page, many);
    const first = await cardBoxes(page, 'pane-many');
    await page.evaluate(() => {
      const column = document.getElementById('pane-many')?.shadowRoot?.querySelector('[part="column"]');
      if (column instanceof HTMLElement) column.scrollTop = 8000;
    });
    await settle(page);
    const second = await cardBoxes(page, 'pane-many');
    expect(second.length).toBeGreaterThan(0);
    expect(second.length).toBeLessThan(30);
    const overlap = second.filter((card) => first.some((entry) => entry.id === card.id));
    expect(overlap.length, 'the window did not move').toBe(0);
  });
});

test.describe('ARIA: a conversation, not a flat list', () => {
  test('the column is a feed whose articles carry their position in the whole set', async ({ page }) => {
    await open(page, many);
    const feed = page.locator(`${annotationTags.reviewPane}#pane-many`).first();
    // Asked of the browser's own role engine rather than read off our attribute.
    const column = page.getByRole('feed', { name: 'Comments' });
    await expect(column).toHaveCount(1);
    expect(await feed.evaluate((element) => element.shadowRoot?.querySelector('[part="column"]')?.getAttribute('role'))).toBe(
      reviewAriaPattern.container,
    );
    const positions = await page.evaluate(() => {
      const pane = document.getElementById('pane-many');
      const column_ = pane?.shadowRoot?.querySelector('[part="column"]');
      return [...(column_?.querySelectorAll('.slot') ?? [])].map((card) => ({
        role: card.getAttribute('role'),
        position: card.getAttribute('aria-posinset'),
        size: card.getAttribute('aria-setsize'),
        name: card.getAttribute('aria-label'),
      }));
    });
    expect(positions.length).toBeGreaterThan(0);
    for (const entry of positions) {
      expect(entry.role).toBe(reviewAriaPattern.item);
      expect(entry.size).toBe('300');
      expect(Number(entry.position)).toBeGreaterThan(0);
      expect(entry.name ?? '').not.toBe('');
    }
  });

  test('the two comment models are announced differently', async ({ page }) => {
    await open(page, models);
    const names = await page.evaluate(() =>
      [...document.querySelectorAll('mjx-comment-card')].map((card) => card.getAttribute('aria-label') ?? ''),
    );
    expect(names.some((name) => name.startsWith('conversation by'))).toBe(true);
    expect(names.some((name) => name.startsWith('note by'))).toBe(true);
  });

  test('a legacy comment offers no reply and no resolve, at all', async ({ page }) => {
    await open(page, models);
    const actions = await page.evaluate(() => {
      const read = (selector: string): string[] => {
        const card = document.querySelector(selector);
        return [...(card?.shadowRoot?.querySelectorAll('button[data-action]') ?? [])].map(
          (button) => (button as HTMLElement).dataset['action'] ?? '',
        );
      };
      return {
        threaded: read('mjx-comment-card[comment-id="threaded-1"]'),
        legacy: read('mjx-comment-card[comment-id="legacy-1"]'),
      };
    });
    expect(actions.threaded).toEqual(['reply', 'resolve', 'delete']);
    expect(actions.legacy).toEqual(['delete']);
  });

  test('a reply is an article of its own inside the conversation', async ({ page }) => {
    await open(page, thread);
    const replies = await page.evaluate(() => {
      const thread_ = document.getElementById('thread-expanded');
      return [...(thread_?.shadowRoot?.querySelectorAll('.reply') ?? [])].map((reply) => ({
        role: reply.getAttribute('role'),
        name: reply.getAttribute('aria-label') ?? '',
      }));
    });
    expect(replies.length).toBe(3);
    for (const reply of replies) {
      expect(reply.role).toBe('article');
      expect(reply.name.startsWith('Reply by ')).toBe(true);
    }
  });

  test('the fold says how many replies it is hiding, and can be opened from the keyboard', async ({ page }) => {
    await open(page, thread);
    const collapsed = page.locator('#thread-collapsed');
    const more = collapsed.locator('.more');
    await expect(more).toHaveText(/Show 2 earlier replies/);
    await more.focus();
    await page.keyboard.press('Enter');
    await settle(page);
    await expect(more).toHaveText(/Show fewer replies/);
    expect(await more.getAttribute('aria-expanded')).toBe('true');
  });

  test('the feed is navigable by the keys the feed pattern specifies', async ({ page }) => {
    await open(page, tight);
    await page.locator('#pane-tight').evaluate((pane) => {
      const column = pane.shadowRoot?.querySelector('[part="column"]');
      if (column instanceof HTMLElement) column.focus();
    });
    const selectionAfter = async (key: string): Promise<string> => {
      await page.keyboard.press(key);
      await settle(page);
      return (await page.locator('#pane-tight').getAttribute('selected')) ?? '';
    };
    const first = await selectionAfter('ArrowDown');
    expect(first).not.toBe('');
    const second = await selectionAfter('ArrowDown');
    expect(second).not.toBe(first);
    const back = await selectionAfter('ArrowUp');
    expect(back).toBe(first);
    await page.keyboard.press('Control+End');
    await settle(page);
    const last = (await page.locator('#pane-tight').getAttribute('selected')) ?? '';
    expect(last).toBe('r3');
    await page.keyboard.press('Control+Home');
    await settle(page);
    expect(await page.locator('#pane-tight').getAttribute('selected')).toBe('c1');
  });
});

test.describe('an empty column', () => {
  test('is not a feed at all, because an empty feed is invalid', async ({ page }) => {
    const empty = { title: annotationStoryTitles.reviewPane, name: 'Nothing To Review' } as const;
    await open(page, empty);
    const state = await page.evaluate(() => {
      const pane = document.getElementById('pane-empty');
      const column = pane?.shadowRoot?.querySelector('[part="column"]') ?? null;
      const message = pane?.shadowRoot?.querySelector('.empty') ?? null;
      return {
        role: column === null ? 'gone' : column.getAttribute('role'),
        hidden: column === null ? true : column.hasAttribute('hidden'),
        display: column === null ? 'none' : getComputedStyle(column).display,
        tabindex: column === null ? null : column.getAttribute('tabindex'),
        message: message === null ? '' : (message as HTMLElement).textContent ?? '',
      };
    });
    // ⚠ `role="feed"` requires owned `article` children; a feed with none is an announcement that
    // there is a list, followed by nothing in it. axe names it `aria-required-children`.
    expect(state.role).toBeNull();
    expect(state.tabindex).toBeNull();
    expect(state.hidden).toBe(true);
    expect(state.display).toBe('none');
    expect(state.message).toContain('No comments');
  });
});

test.describe('author colours reach the paint', () => {
  test('the eight slots are declared on the document and differ from each other', async ({ page }) => {
    await open(page, models);
    const colours = await page.evaluate((count) => {
      const style = getComputedStyle(document.documentElement);
      return Array.from({ length: count }, (_, index) =>
        style.getPropertyValue(`--mjx-author-colour-${String(index)}`).trim(),
      );
    }, authorColourSlots.length);
    expect(colours.filter((colour) => colour !== '')).toHaveLength(authorColourSlots.length);
    expect(new Set(colours).size).toBe(authorColourSlots.length);
  });

  test('the slot property is what a card paints its band with', async ({ page }) => {
    await open(page, models);
    const painted = await page.evaluate((property) => {
      const card = document.querySelector('mjx-comment-card[comment-id="threaded-1"]');
      const own = (card as HTMLElement | null)?.style.getPropertyValue('--mjx-annotation-author-colour') ?? '';
      const band = card?.shadowRoot?.querySelector('[part="card"]');
      const before = band === null || band === undefined ? '' : getComputedStyle(band, '::before').backgroundColor;
      return { own, before, expected: property };
    }, authorColourProperty(0));
    expect(painted.own).toContain(painted.expected);
    expect(painted.before).not.toBe('rgba(0, 0, 0, 0)');
  });

  test('the slots change token when the scheme changes', async ({ page }) => {
    await open(page, models, { theme: 'light' });
    const light = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--mjx-author-colour-6').trim(),
    );
    await open(page, models, { theme: 'dark' });
    const dark = await page.evaluate(() =>
      getComputedStyle(document.documentElement).getPropertyValue('--mjx-author-colour-6').trim(),
    );
    // Slot 6 is `color.ink` in light and `color.card` in dark: the same identity, spelled by the
    // token that is visible in each. One token per slot would have made this author invisible in
    // whichever scheme nobody checked.
    expect(light).not.toBe(dark);
  });
});

test.describe('the touch presentation', () => {
  test('is a sheet in the phone preset, and a margin column on a desktop', async ({ page }) => {
    await open(page, phone, { containerPreset: 'desktop' });
    const onDesktop = await page.evaluate((property) => {
      const pane = document.getElementById('pane-sheet');
      if (pane === null) return undefined;
      const column = pane.shadowRoot?.querySelector('[part="column"]') ?? null;
      const card = column?.querySelector('.slot') ?? null;
      const handle = pane.shadowRoot?.querySelector('[part="handle"]') ?? null;
      return {
        presentation: getComputedStyle(pane).getPropertyValue(property).trim(),
        cardPosition: card === null ? '' : getComputedStyle(card).position,
        handleDisplay: handle === null ? '' : getComputedStyle(handle).display,
      };
    }, reviewPresentationProperty);
    expect(onDesktop?.presentation).toBe('margin');
    expect(onDesktop?.cardPosition).toBe('absolute');
    expect(onDesktop?.handleDisplay).toBe('none');

    await open(page, phone, { containerPreset: 'phone' });
    const onPhone = await page.evaluate((property) => {
      const pane = document.getElementById('pane-sheet');
      if (pane === null) return undefined;
      const column = pane.shadowRoot?.querySelector('[part="column"]') ?? null;
      const card = column?.querySelector('.slot') ?? null;
      const handle = pane.shadowRoot?.querySelector('[part="handle"]') ?? null;
      return {
        presentation: getComputedStyle(pane).getPropertyValue(property).trim(),
        cardPosition: card === null ? '' : getComputedStyle(card).position,
        handleDisplay: handle === null ? '' : getComputedStyle(handle).display,
        built: (pane as HTMLElement & { builtCardCount: number }).builtCardCount,
      };
    }, reviewPresentationProperty);
    // ⚠ Three facts the property does not control, beside the property itself. A gate that read
    // only the token would be asking the implementation to grade its own homework.
    expect(onPhone?.presentation).toBe('sheet');
    expect(onPhone?.cardPosition).toBe('static');
    expect(onPhone?.handleDisplay).toBe('block');
    expect(onPhone?.built).toBe(tightlyClusteredAnchors.length);
  });

  test('the container decides, and the window never moves', async ({ page }) => {
    await open(page, phone, { containerPreset: 'phone' });
    const width = await page.evaluate(() => window.innerWidth);
    await open(page, phone, { containerPreset: 'desktop' });
    expect(await page.evaluate(() => window.innerWidth)).toBe(width);
  });
});

test.describe('the hit-target floor', () => {
  test('every affordance on a card clears it', async ({ page }) => {
    for (const story of [models, thread, kinds]) {
      await open(page, story);
      const boxes = await page.evaluate(() => {
        const found: { where: string; width: number; height: number }[] = [];
        for (const tag of ['mjx-comment-card', 'mjx-comment-thread', 'mjx-tracked-change-card']) {
          for (const host of document.querySelectorAll(tag)) {
            for (const control of host.shadowRoot?.querySelectorAll('button') ?? []) {
              const box = control.getBoundingClientRect();
              if (box.width === 0 && box.height === 0) continue;
              found.push({
                where: `${tag} ${(control as HTMLElement).dataset['action'] ?? control.className}`,
                width: box.width,
                height: box.height,
              });
            }
          }
        }
        return found;
      });
      expect(boxes.length, `${story.name} has no affordances to measure`).toBeGreaterThan(0);
      for (const box of boxes) {
        expect(box.height, box.where).toBeGreaterThanOrEqual(accessibleHitTargetMinimum - 0.5);
      }
    }
  });
});
