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

/**
 * The container width at or below which this platform stops being a desktop.
 *
 * **Hoisted here by MJXOFF-185, on the instruction MJXOFF-184 left in `menu-model.ts`:**
 *
 * > Two numbers that must agree and are written twice are two numbers that will not agree. When a
 * > third surface needs it, hoist it to `src/harness/presets.ts` — do not add a second definition.
 *
 * There are now three. The ribbon's tab strip becomes a picker here (`tabStripPickerAtOrBelow`), a
 * floating menu becomes a sheet here (`menuSheetAtOrBelow`), and a gallery's expanded flyout
 * becomes a sheet here (`gallerySheetAtOrBelow`). All three are aliases of this constant, and
 * `tests/gallery.test.ts` asserts that they still are — because an alias that quietly became a
 * literal is exactly the drift the hoist was asked for.
 *
 * It lives beside the container presets because it is the same kind of fact as `containerPresets`:
 * a statement about the shell the catalogue is audited in, not about any one component. It sits
 * between `phone` (390) and `tablet` (834), which is what makes both presets land on the side of
 * the threshold a person would expect.
 */
export const phoneShellAtOrBelow = 560;
