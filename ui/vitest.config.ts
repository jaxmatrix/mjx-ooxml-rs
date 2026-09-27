import { defineConfig } from 'vitest/config';

/**
 * The unit tier.
 *
 * Deliberately **node**, with no DOM emulation. Everything in this child that depends on layout,
 * the cascade or computed styles — the token resolver's host leg, the theme, the container query,
 * the contrast gate — is proved in `tests/browser/` against a real browser, because a DOM
 * emulation resolves neither `var()` nor `@container` and would let all four of those pass while
 * being broken. What is left for this tier is the logic that has no rendering in it: the manifest
 * parser, the contrast arithmetic, the conventions validator, and the lint rule.
 *
 * One worker: this machine runs Rust builds beside the catalogue and a test tier that competes for
 * memory with a compiler is a test tier that fails for reasons that have nothing to do with the
 * code.
 */
export default defineConfig({
  test: {
    environment: 'node',
    include: ['tests/**/*.test.ts'],
    fileParallelism: false,
    maxWorkers: 1,
    reporters: ['default'],
  },
});
