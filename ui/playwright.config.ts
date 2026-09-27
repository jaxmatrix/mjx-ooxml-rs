import { defineConfig, devices } from '@playwright/test';

/**
 * The browser tier — the one that can actually prove anything about this child.
 *
 * Four of the five gates MJXOFF-180 asks for are unprovable without a rendering engine: a contrast
 * rule needs composited colours, a theme assertion needs the cascade, a container query needs
 * layout, and a token override needs `getComputedStyle` to resolve `var()`. A DOM emulation
 * answers none of those and would let all four pass while broken, which is precisely the trap the
 * ticket describes.
 *
 * ## Against the built catalogue, not the dev server
 *
 * `build-storybook` *"fails on errors a dev server tolerates"*, so the tier that has to be
 * trustworthy runs against the same artefact CI publishes. `webServer` builds nothing — it serves
 * `storybook-static/`, which `npm run check` has already produced.
 *
 * ## One worker, on purpose
 *
 * This repository's Rust workspace is often compiling in the same tree. A browser tier that fanned
 * out across cores would compete with a compiler for memory, and *a resource failure reads exactly
 * like a code failure* — an OOM-killed browser reports a closed page, which looks like a bug in the
 * page.
 */
const port = 6007;

export default defineConfig({
  testDir: 'tests/browser',
  fullyParallel: false,
  workers: 1,
  forbidOnly: process.env['CI'] !== undefined,
  retries: 0,
  reporter: process.env['CI'] !== undefined ? [['github'], ['list']] : [['list']],
  timeout: 60_000,
  expect: {
    timeout: 10_000,
    toHaveScreenshot: {
      // Colour-and-geometry plates only — see `tests/browser/visual.spec.ts` for why no snapshot
      // in this suite contains text. The tolerance covers a stray antialiased edge pixel, not a
      // changed colour: a token change moves thousands of pixels, not two.
      maxDiffPixelRatio: 0.002,
      threshold: 0.1,
    },
  },
  use: {
    baseURL: `http://127.0.0.1:${String(port)}`,
    trace: 'retain-on-failure',
    screenshot: 'only-on-failure',
    viewport: { width: 1280, height: 900 },
    deviceScaleFactor: 1,
  },
  projects: [
    {
      name: 'chromium',
      // The viewport is restated after the device spread on purpose: the device preset carries one
      // of its own, and `tests/browser/container.spec.ts` asserts that the viewport does not move,
      // which needs the starting width to be a stated number rather than a preset's default.
      use: { ...devices['Desktop Chrome'], viewport: { width: 1280, height: 900 }, deviceScaleFactor: 1 },
    },
  ],
  webServer: {
    command: `node scripts/serve-static.mjs storybook-static ${String(port)}`,
    url: `http://127.0.0.1:${String(port)}/index.json`,
    reuseExistingServer: false,
    timeout: 30_000,
    stdout: 'ignore',
    stderr: 'pipe',
  },
});
