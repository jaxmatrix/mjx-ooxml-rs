/**
 * The type scale — six roles, every one of them expressed in tokens.
 *
 * `DESIGN_TOKENS.md` §4 makes two corrections that this file exists to *enforce* rather than
 * repeat:
 *
 * > **Nunito Sans is a rounded humanist face** and reads slightly wider than a typical UI font.
 * > Dense surfaces — the formula bar, the cell grid, a properties inspector — should use
 * > `--text-xs` (`0.75rem`) with `--leading-tight`, not the site's relaxed `1.7`.
 *
 * > **Young Serif is display-only.** It belongs in empty states and onboarding, never in the
 * > chrome or in document content.
 *
 * Both are asserted. `tests/foundations.test.ts` fails if the dense role stops being
 * `text.xs × leading.tight`, and fails if any role other than `display` names the serif;
 * `tests/browser/foundations.spec.ts` reads the *computed* values back out of a browser, so a
 * scale that is right in this file and wrong in the cascade is caught too.
 *
 * ## Why the scale is only two token sizes wide, and what is done about it
 *
 * The generated token table declares exactly two sizes: `--text-xs` and `--text-sm`. That is the
 * source site's own scale, and it is genuinely enough for chrome — a toolbar, a menu and an
 * inspector all live between 12 and 14 pixels. It is *not* enough for a pane title or an empty
 * state, and the honest options were to invent two more tokens or to derive them.
 *
 * **Deriving won**, for the reason the whole child is written around: `tokens.json` is about to be
 * re-seeded from another product of the user's, that file belongs to the Rust track, and a token
 * invented here would either be overwritten or become a second source of truth. A ratio applied to
 * `--text-sm` follows the re-seed automatically, so the scale stays proportional to whatever the
 * body size becomes. The ratios are named constants below rather than magic numbers in a CSS
 * string, and the browser gate asserts the resulting sizes are all distinct and in order — which
 * is what makes the scale a scale rather than six names for one size.
 *
 * ## Node-importable
 *
 * Data and strings only. `tests/foundations.test.ts` imports this from Node.
 */

/** The roles, widest use first. Every component picks one; nothing invents a seventh. */
export const typeRoleNames = [
  'dense',
  'control',
  'body',
  'label',
  'paneTitle',
  'display',
] as const;

/** One of the six roles. */
export type TypeRole = (typeof typeRoleNames)[number];

/**
 * The two ratios that extend the scale past the two sizes the tokens declare.
 *
 * A ratio, unlike a length, survives the palette re-seed: whatever `--text-sm` becomes, a pane
 * title stays proportionally larger than a menu item.
 */
export const typeScaleRatios = { paneTitle: 1.15, display: 2 } as const;

/** Everything a role fixes. Every value is a `var()` or a `calc()` over one. */
export interface TypeRoleSpec {
  /** `font-family`. */
  readonly family: string;
  /** `font-size`. */
  readonly size: string;
  /** `line-height`. */
  readonly leading: string;
  /** `font-weight`. */
  readonly weight: string;
  /** `letter-spacing`. */
  readonly tracking: string;
  /** Where it is used, and where it must not be. */
  readonly use: string;
}

/**
 * The scale.
 *
 * `dense` is the one §4 legislates and the one an Office chrome uses most: a cell, a formula bar,
 * an inspector row. `display` is the only serif, and the only role a chrome may never use.
 */
export const typeRoles: Readonly<Record<TypeRole, TypeRoleSpec>> = {
  dense: {
    family: 'var(--font-sans)',
    size: 'var(--text-xs)',
    leading: 'var(--leading-tight)',
    weight: 'var(--font-weight-medium)',
    tracking: 'normal',
    use: 'The formula bar, the cell grid, an inspector row — DESIGN_TOKENS.md §4 names these exactly.',
  },
  control: {
    family: 'var(--font-sans)',
    size: 'var(--text-sm)',
    leading: 'var(--leading-snug)',
    weight: 'var(--font-weight-medium)',
    tracking: 'normal',
    use: 'Buttons, menu items, tabs, field labels: one line of text inside a control.',
  },
  body: {
    family: 'var(--font-sans)',
    size: 'var(--text-sm)',
    leading: 'var(--leading-body)',
    weight: 'var(--font-weight-medium)',
    tracking: 'normal',
    use: 'Prose inside a panel — help text, a dialog explanation. The one place the source site’s relaxed 1.7 measure is right.',
  },
  label: {
    family: 'var(--font-sans)',
    size: 'var(--text-xs)',
    leading: 'var(--leading-tight)',
    weight: 'var(--font-weight-bold)',
    tracking: 'var(--tracking-tight)',
    use: 'A group header inside a panel or a ribbon tab. Same size as `dense`, and told apart by weight.',
  },
  paneTitle: {
    family: 'var(--font-sans)',
    size: `calc(var(--text-sm) * ${String(typeScaleRatios.paneTitle)})`,
    leading: 'var(--leading-snug)',
    weight: 'var(--font-weight-semibold)',
    tracking: 'var(--tracking-tight)',
    use: 'The title of a task pane, a dialog or a sheet.',
  },
  display: {
    family: 'var(--font-serif)',
    size: `calc(var(--text-sm) * ${String(typeScaleRatios.display)})`,
    leading: 'var(--leading-tight)',
    weight: 'var(--font-weight-extrabold)',
    tracking: 'var(--tracking-tight)',
    use: 'Empty states and onboarding, and nothing else. §4: Young Serif is display-only — never chrome, never document content.',
  },
};

/** The class a role is applied with. `dense` → `.mjx-type-dense`. */
export function typeRoleClass(role: TypeRole): string {
  return `mjx-type-${role.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`)}`;
}

/**
 * The stylesheet.
 *
 * `:where()` keeps the specificity at zero so a component can override one property of a role
 * without an `!important` arms race — the role is a starting point, not a cage.
 */
export const typographyCss = `
:where(.mjx-typography) {
  font-family: var(--font-sans);
  font-size: var(--text-sm);
  line-height: var(--leading-snug);
  color: var(--theme-text-primary);
}
${typeRoleNames
  .map((role) => {
    const spec = typeRoles[role];
    return `:where(.${typeRoleClass(role)}) {
  font-family: ${spec.family};
  font-size: ${spec.size};
  line-height: ${spec.leading};
  font-weight: ${spec.weight};
  letter-spacing: ${spec.tracking};
}`;
  })
  .join('\n')}
`;
