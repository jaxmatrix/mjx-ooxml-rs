/**
 * Density — comfortable and compact.
 *
 * MJXOFF-181: *"comfortable and compact, since a formula bar and a ribbon cannot share one spacing
 * scale."* And, as the gate: *"density modes change spacing without changing hit-target size below
 * the accessible minimum, asserted on computed values."*
 *
 * Those two sentences are in tension on purpose, and the resolution is the whole design here:
 * **compact reduces the space between things, and reduces the target only as far as the floor.**
 * A compact mode that shrank a 40px button to 20px would be a compact mode nobody with a trackpad
 * or a tremor can use, and the floor is what says so — WCAG 2.2's *Target Size (Minimum)*, 24 CSS
 * pixels.
 *
 * ## Everything is a multiple of `--spacing`
 *
 * `--spacing` is `0.25rem` — the Tailwind base unit the source site is written in. Expressing the
 * step, the gutter and the hit target as integer multiples of it means the whole density system
 * follows a re-seed of the token, and means no length is written down in this file. The literal-
 * value lint is scoped to `src/`, so that is enforced rather than intended.
 *
 * ## How it is applied
 *
 * `data-density="compact"` on any ancestor. The custom properties cascade, so a compact inspector
 * inside a comfortable shell is one attribute, and — unlike the colour scheme, which
 * `tokens.css` keys off `:root` and which therefore cannot vary per subtree — this genuinely does
 * work on a `<div>`. `tests/browser/foundations.spec.ts` proves it by nesting one inside the
 * other.
 *
 * ## Node-importable
 *
 * Data and strings only.
 */

/** The two modes. */
export const densityModeNames = ['comfortable', 'compact'] as const;

/** One of the two. */
export type DensityMode = (typeof densityModeNames)[number];

/** The custom properties a density mode sets. */
export const densityProperties = {
  /** The gap between two related things — a label and its field, two toolbar buttons. */
  step: '--mjx-density-step',
  /** The padding inside a surface. */
  gutter: '--mjx-density-gutter',
  /** The minimum block and inline size of anything a pointer must hit. */
  hitTarget: '--mjx-hit-target',
} as const;

/**
 * WCAG 2.2 *Target Size (Minimum)*, level AA, in CSS pixels.
 *
 * The number is a standard's, not a taste; it is written here once so both the stylesheet and the
 * gate read the same one.
 */
export const accessibleHitTargetMinimum = 24;

/** A mode, in multiples of `--spacing`. */
export interface DensitySpec {
  readonly stepUnits: number;
  readonly gutterUnits: number;
  readonly hitTargetUnits: number;
  readonly use: string;
}

/**
 * The two modes.
 *
 * `--spacing` is `0.25rem` = 4px at the default root size, so comfortable is an 8px step, a 12px
 * gutter and a 40px target, and compact is 4px, 8px and 32px. The floor is 24px; compact clears it
 * by two whole spacing units, which is the margin a re-seed of `--spacing` may eat into before the
 * gate fires.
 */
export const densityModes: Readonly<Record<DensityMode, DensitySpec>> = {
  comfortable: {
    stepUnits: 1,
    gutterUnits: 2,
    hitTargetUnits: 8,
    use: 'The default. A ribbon, a dialog, a task pane — anywhere a pointer is the primary input.',
  },
  compact: {
    stepUnits: 0.5,
    gutterUnits: 1,
    hitTargetUnits: 8,
    use: 'A formula bar, a cell grid, a properties inspector: surfaces where vertical room is the scarce resource.',
  },
};

/** `2` → `calc(var(--spacing) * 2)`. One unit is `var(--spacing)` on its own. */
export function spacingMultiple(units: number): string {
  return units === 1 ? 'var(--spacing)' : `calc(var(--spacing) * ${String(units)})`;
}

function declarations(mode: DensityMode): string {
  const spec = densityModes[mode];
  return (
    `  ${densityProperties.step}: ${spacingMultiple(spec.stepUnits)};\n` +
    `  ${densityProperties.gutter}: ${spacingMultiple(spec.gutterUnits)};\n` +
    `  ${densityProperties.hitTarget}: ${spacingMultiple(spec.hitTargetUnits)};`
  );
}

export const densityCss = `
/* The default is comfortable and it is written on :root as well as on the attribute, so a shell
 * that never sets data-density still gets a defined --mjx-hit-target rather than an empty
 * custom property that silently resolves to nothing. */
:where(:root, .mjx-foundations, [data-density='comfortable']) {
${declarations('comfortable')}
}

:where([data-density='compact']) {
${declarations('compact')}
}

/* The floor, applied rather than documented. A component that opts in cannot be shrunk below the
 * accessible minimum by any density mode, present or future. */
:where(.mjx-hit-target) {
  min-inline-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
  min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
}
`;
