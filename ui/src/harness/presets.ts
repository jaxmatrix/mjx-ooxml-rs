/**
 * The harness's dimensions, as data and nothing else.
 *
 * Split out of `resizable-container.ts` for a reason worth stating: **a module that defines a
 * custom element cannot be imported from Node.** `class extends HTMLElement` is evaluated at module
 * scope, and the browser tier's test files run in Node — so a Playwright spec that wanted the
 * preset widths would drag a DOM class into a process that has no DOM and fail at import time,
 * before a single assertion ran.
 *
 * The rule this establishes for the fifteen further Phase U children: **constants a test needs live
 * beside the component, not inside it.**
 */

/** The three widths the catalogue is audited at, in CSS pixels. */
export const containerPresets = {
  desktop: 1440,
  tablet: 834,
  phone: 390,
} as const;

/** A preset's name. */
export type ContainerPreset = keyof typeof containerPresets;

/** The presets in the order the harness offers them: widest first. */
export const containerPresetOrder: readonly ContainerPreset[] = ['desktop', 'tablet', 'phone'];

/** The narrowest and widest widths the range control offers. */
export const containerWidthBounds = { min: 280, max: 1600 } as const;

/** The name a component's `@container` query should address. */
export const containerName = 'mjx-frame';
