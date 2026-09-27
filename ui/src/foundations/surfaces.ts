/**
 * The elevation ladder and the radius scale.
 *
 * `DESIGN_TOKENS.md` §4:
 *
 * > **Radii are large** (`10px` controls, `16px` cards, `20px` panels). Toolbars and inspectors
 * > should keep that softness rather than reverting to the 2–4px corners Office uses; it is the
 * > most recognisable part of the source's character.
 *
 * > **Shadows are ink-tinted**, never neutral black. `--shadow-lift` is `#223b331c`.
 *
 * The second is where MJXOFF-181's trap lives: *"a shadow that renders as flat black is wrong in a
 * way a snapshot will happily lock in."* So the browser gate does not compare a picture. It reads
 * the computed `box-shadow` off a raised surface, parses the colour out of it, and asserts two
 * separate things — that it **equals the generated `shadow.lift` token** channel for channel, and
 * that it is **chromatic** (its three channels are not equal), which is the property a neutral
 * black cannot have however the palette is re-seeded.
 *
 * ## Seven levels, one shadow token
 *
 * The token table declares exactly one shadow. Rather than invent a second — the same reasoning as
 * `typography.ts`: an invented token would be overwritten by the re-seed or become a second source
 * of truth — `overlay` states `--shadow-lift` twice. Two identical shadows composite to a deeper
 * one of the same hue, so the ink tint is preserved by construction and the ladder still has a
 * step above `floating`. That is a composition of the token, not a replacement for it.
 *
 * ## Node-importable
 *
 * Data and strings only.
 */

/** The ladder, lowest first. */
export const surfaceLevelNames = [
  'base',
  'sunken',
  'raised',
  'floating',
  'overlay',
  'accent',
  'secondary',
] as const;

/** One rung. */
export type SurfaceLevel = (typeof surfaceLevelNames)[number];

/** Everything a level fixes. Every value is a `var()` over a generated token. */
export interface SurfaceLevelSpec {
  readonly background: string;
  readonly border: string;
  /** `none`, or a composition of `--shadow-lift`. */
  readonly shadow: string;
  /** The default radius, which the `radius` attribute overrides. */
  readonly radius: RadiusStep;
  readonly use: string;
}

/** The radius scale, smallest first. Exactly the five the tokens declare, plus the phone bezel. */
export const radiusSteps = ['chip', 'control', 'card', 'panel', 'frame', 'phone'] as const;

/** One step of the radius scale. */
export type RadiusStep = (typeof radiusSteps)[number];

/** `control` → `var(--radius-control)`. */
export function radiusVariable(step: RadiusStep): string {
  return `var(--radius-${step})`;
}

/** The ladder. */
export const surfaceLevels: Readonly<Record<SurfaceLevel, SurfaceLevelSpec>> = {
  base: {
    background: 'var(--theme-background)',
    border: 'none',
    shadow: 'none',
    radius: 'frame',
    use: 'The application ground. Nothing sits behind it.',
  },
  sunken: {
    background: 'var(--theme-background)',
    border: '1px solid var(--theme-border-subtle)',
    shadow: 'none',
    radius: 'card',
    use: 'A well inside a panel: a list, a preview area, a code block.',
  },
  raised: {
    background: 'var(--theme-surface)',
    border: '1px solid var(--theme-border-subtle)',
    shadow: 'none',
    radius: 'card',
    use: 'A card or a toolbar on the ground. Told apart from the ground by fill, not by shadow.',
  },
  floating: {
    background: 'var(--theme-surface-raised)',
    border: '1px solid var(--theme-border)',
    shadow: 'var(--shadow-lift)',
    radius: 'panel',
    use: 'A task pane, a docked inspector, a sheet. The first rung that leaves the plane.',
  },
  overlay: {
    background: 'var(--theme-surface-raised)',
    border: '1px solid var(--theme-border)',
    // The one shadow token, stated twice: two identical ink-tinted shadows composite to a deeper
    // one of the same hue. A second hand-written shadow would have been a literal, and the ticket
    // is explicit that a literal is a defect.
    shadow: 'var(--shadow-lift), var(--shadow-lift)',
    radius: 'panel',
    use: 'A menu, a dialog, a popover — anything drawn over content it must be readable against.',
  },
  accent: {
    background: 'var(--theme-accent-surface)',
    border: '1px solid var(--theme-accent-border)',
    shadow: 'none',
    radius: 'control',
    use: 'A selected row, a chosen tab, an active chip. MJXOFF-181 names this the predictable focus failure: a green ring on a green tint.',
  },
  secondary: {
    background: 'var(--theme-secondary-surface)',
    border: '1px solid var(--theme-border-subtle)',
    shadow: 'none',
    radius: 'control',
    use: 'A warning or an unsaved-state banner: the honey half of the palette.',
  },
};

/**
 * The token whose *computed* colour each level shows behind its content.
 *
 * The focus gate needs it: a ring's contrast is measured against the thing behind the ring, and
 * that is this. Stated once here so the gate cannot disagree with the stylesheet.
 */
export const surfaceBackgroundMember: Readonly<Record<SurfaceLevel, string>> = {
  base: 'background',
  sunken: 'background',
  raised: 'surface',
  floating: 'surfaceRaised',
  overlay: 'surfaceRaised',
  accent: 'accentSurface',
  secondary: 'secondarySurface',
};

/** The stylesheet: one class per level, one per radius step. */
export const surfacesCss = `
${surfaceLevelNames
  .map((level) => {
    const spec = surfaceLevels[level];
    return `:where(.mjx-surface-${level}) {
  background: ${spec.background};
  border: ${spec.border};
  box-shadow: ${spec.shadow};
  border-radius: ${radiusVariable(spec.radius)};
}`;
  })
  .join('\n')}
${radiusSteps.map((step) => `:where(.mjx-radius-${step}) { border-radius: ${radiusVariable(step)}; }`).join('\n')}
`;
