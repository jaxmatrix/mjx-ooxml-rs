/**
 * The container probe's breakpoints, as data.
 *
 * Separated from `probes.ts` for the same reason `src/harness/presets.ts` is separated from
 * `resizable-container.ts`: the browser tier's specs run in Node, and a module that evaluates
 * `class extends HTMLElement` at import time cannot be loaded there. Constants a test needs live
 * beside the thing that uses them, not inside it.
 *
 * The values sit deliberately *between* the harness presets, so that each preset lands in a
 * different band. A probe whose breakpoints coincided with the presets would report the same band
 * for two of them and the container test would look like it worked while proving nothing.
 */

import { containerPresets } from '../src/harness/presets.ts';

/** The bands the container probe reports, widest first. */
export const containerBands = ['wide', 'medium', 'narrow'] as const;

/** One of the three bands. */
export type ContainerBand = (typeof containerBands)[number];

/** The breakpoints, in the container's inline size. */
export const containerBreakpoints = { medium: 700, wide: 1100 } as const;

/**
 * Each preset's band, computed rather than written down — so the two tables cannot drift apart.
 */
export function bandFor(width: number): ContainerBand {
  if (width >= containerBreakpoints.wide) return 'wide';
  if (width >= containerBreakpoints.medium) return 'medium';
  return 'narrow';
}

/** The three presets and the band each one lands in. */
export const presetBands: Readonly<Record<keyof typeof containerPresets, ContainerBand>> = {
  desktop: bandFor(containerPresets.desktop),
  tablet: bandFor(containerPresets.tablet),
  phone: bandFor(containerPresets.phone),
};
