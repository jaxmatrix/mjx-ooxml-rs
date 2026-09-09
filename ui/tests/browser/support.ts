import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { Page } from '@playwright/test';

/**
 * Shared machinery for the browser tier.
 *
 * The story list is read from `storybook-static/index.json` — the build's own index — rather than
 * from a list written here. That is the difference between a sweep that covers the catalogue and a
 * sweep that covers whatever somebody remembered to add, and fifteen further children are about to
 * add stories to it.
 */

const staticRoot = resolve(import.meta.dirname, '../../storybook-static');

interface IndexEntry {
  readonly id: string;
  readonly title: string;
  readonly name: string;
  readonly type: string;
}

/** Every story in the built catalogue. Throws if the catalogue has not been built. */
export function builtStories(): IndexEntry[] {
  let raw: string;
  try {
    raw = readFileSync(resolve(staticRoot, 'index.json'), 'utf8');
  } catch {
    throw new Error(
      'storybook-static/index.json is missing. Run `npm run build-storybook` first — the browser ' +
        'tier deliberately runs against the built catalogue, not the dev server.',
    );
  }
  const parsed = JSON.parse(raw) as { entries?: Record<string, IndexEntry> };
  const entries = Object.values(parsed.entries ?? {}).filter((entry) => entry.type === 'story');
  if (entries.length === 0) {
    throw new Error('the built catalogue contains no stories, which cannot be right.');
  }
  return entries;
}

/** The attributes `.storybook/preview.ts` publishes on the root element. */
export const expectationAttribute = 'data-mjx-expect-violations';
export const conventionsAttribute = 'data-mjx-conventions';

export interface OpenOptions {
  /** `light`, `dark` or `system`. */
  readonly theme?: string;
  /** `desktop`, `tablet` or `phone`. */
  readonly containerPreset?: string;
}

/**
 * Open one story in the preview iframe and wait until it has rendered.
 *
 * Globals are passed in the URL, which is Storybook's own supported mechanism, so a test changes
 * the theme exactly the way the toolbar does rather than by reaching into the preview.
 */
export async function openStory(page: Page, id: string, options: OpenOptions = {}): Promise<void> {
  const globals = Object.entries(options)
    .filter(([, value]) => value !== undefined)
    .map(([key, value]) => `${key}:${String(value)}`)
    .join(';');
  const query = `id=${encodeURIComponent(id)}&viewMode=story${globals === '' ? '' : `&globals=${encodeURIComponent(globals)}`}`;
  await page.goto(`/iframe.html?${query}`, { waitUntil: 'load' });
  await page.waitForSelector('#storybook-root > *', { state: 'attached', timeout: 20_000 });
  // Custom elements upgrade asynchronously; a computed style read before the upgrade would see the
  // element's pre-render box and report an answer that has nothing to do with the component.
  await page.waitForFunction(() => customElements.get('mjx-resizable-container') !== undefined);
  await page.evaluate(async () => {
    await new Promise((done) => requestAnimationFrame(() => requestAnimationFrame(done)));
  });
  await waitForAxeIdle(page);
}

/**
 * Wait until the accessibility addon's own axe run has finished.
 *
 * ⚠ There are **two** axe runs per story and only one of them is the gate. `preview.ts` sets
 * `a11y: { test: 'error' }`, so the addon runs axe in the preview as each story renders; the sweep
 * then runs axe again through `AxeBuilder`. axe-core refuses to run twice at once — *"Axe is
 * already running"* — and which of the two wins is a matter of how long the story took to paint.
 *
 * MJXOFF-181 hit it as a single red story out of seventy-six, on a run where nothing about that
 * story had changed. Left alone it is the worst kind of gate failure: intermittent, unrelated to
 * the change, and therefore the thing that teaches everyone to re-run the suite instead of reading
 * it. Waiting for the addon to finish removes the race rather than retrying past it.
 */
export async function waitForAxeIdle(page: Page): Promise<void> {
  try {
    await page.waitForFunction(
      () => {
        const axe = (globalThis as { axe?: { _running?: boolean } }).axe;
        return axe === undefined || axe._running !== true;
      },
      undefined,
      { timeout: 15_000 },
    );
  } catch {
    // A story whose addon run never finishes is a finding in its own right, but it is not this
    // helper's to report: the sweep below will still run axe and will still say what it found.
  }
}

/** The rules a component catalogue cannot meaningfully answer, and why each is off. */
export const disabledAxeRules: Readonly<Record<string, string>> = {
  // Every story renders alone in a preview iframe with no landmarks, because a component is not a
  // page. Landmark structure belongs to the application shell, which is loop 2.
  region: 'a component in an isolated preview has no page landmarks, and should not invent any',
};
