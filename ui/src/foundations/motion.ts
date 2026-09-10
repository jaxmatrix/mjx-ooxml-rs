/**
 * The motion vocabulary — the easing tokens with their intended use **recorded and checked**.
 *
 * `DESIGN_TOKENS.md` §4:
 *
 * > **`--ease-spring` overshoots** (`cubic-bezier(.34, 1.56, .64, 1)`). Good for a panel or sheet
 * > entering; wrong for anything attached to a document object, where overshoot reads as
 * > imprecision. Use `--ease-ink` for selection, handles and canvas motion.
 *
 * That paragraph has been true and unenforced since MJXOFF-156. Here it becomes a test. Every role
 * below declares whether it is **attached to a document object**, and `tests/foundations.test.ts`
 * fails if any role that is uses the overshooting easing. A later child adding a selection-handle
 * animation with `--ease-spring` therefore fails a gate rather than shipping motion that reads as
 * imprecision — which is the only way a note in a document survives fifteen children.
 *
 * `tests/browser/foundations.spec.ts` then reads the *computed* `transition-timing-function` back
 * out of a browser and compares it to the generated token's own cubic-bezier, so a role that names
 * the right token in this file and resolves to the wrong curve in the cascade is caught as well.
 *
 * ## The one duration token, and how a *delay* is expressed in it (MJXOFF-189)
 *
 * The generated table declares exactly one duration, `--duration-transition`. U10 needs four more
 * spans of time — a screentip's appearance delay, its warm-up window, a toast's dwell and an
 * indeterminate bar's cycle — and the honest options were the two `typography.ts` faced over its
 * two type sizes: invent tokens, or derive.
 *
 * **Deriving wins here for a second reason on top of that one.** A delay written as `600ms` in a
 * component is a literal the lint would refuse, and a delay written as `calc(var(--duration-transition) * 4)`
 * is a *ratio* — so a host that slows the platform down for a person who needs longer to read
 * slows the screentip down with it, and a re-seed of the token moves all five together.
 * `durationMultiple` is the spelling; `resolveDurationMilliseconds` is how a component gets a
 * **number** back out of the cascade so a `setTimeout` can use it.
 *
 * ## Node-importable
 *
 * Data and strings, plus one function that reads a computed style. The DOM is touched **inside a
 * function body, never at module scope** — the rule `overlay/floating.ts` states for the same
 * arrangement — so `tests/foundations.test.ts` still imports this file from Node.
 */

/** The roles, in the order the catalogue shows them. */
export const motionRoleNames = [
  'panelEnter',
  'sheetEnter',
  'surfaceSettle',
  'selection',
  'documentObject',
] as const;

/** One of the five roles. */
export type MotionRole = (typeof motionRoleNames)[number];

/** Everything a role fixes, plus the one fact the gate reads. */
export interface MotionRoleSpec {
  /** The easing's dotted token path, so a gate can look it up in the generated table. */
  readonly easingToken: 'ease.ink' | 'ease.outSoft' | 'ease.spring';
  /** The `var()` a stylesheet writes. */
  readonly easing: string;
  /** The duration's `var()`. One duration token exists; a role that needs another states a ratio. */
  readonly duration: string;
  /**
   * Whether the thing being animated is attached to an object in the user's document.
   *
   * **This is the field the gate reads.** True means overshoot is forbidden: a selection handle
   * that springs past its anchor and comes back has told the user their object moved.
   */
  readonly attachedToDocumentObject: boolean;
  readonly use: string;
}

export const motionRoles: Readonly<Record<MotionRole, MotionRoleSpec>> = {
  panelEnter: {
    easingToken: 'ease.spring',
    easing: 'var(--ease-spring)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: false,
    use: 'A task pane or an inspector docking. Overshoot reads as the panel arriving with weight.',
  },
  sheetEnter: {
    easingToken: 'ease.spring',
    easing: 'var(--ease-spring)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: false,
    use: 'A sheet or a dialog entering over the content.',
  },
  surfaceSettle: {
    easingToken: 'ease.outSoft',
    easing: 'var(--ease-out-soft)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: false,
    use: 'Hover, press and disclosure on chrome: a decelerating move with no overshoot at all.',
  },
  selection: {
    easingToken: 'ease.ink',
    easing: 'var(--ease-ink)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: true,
    use: 'A selection rectangle, a handle, an alignment guide. §4 names these exactly.',
  },
  documentObject: {
    easingToken: 'ease.ink',
    easing: 'var(--ease-ink)',
    duration: 'var(--duration-transition)',
    attachedToDocumentObject: true,
    use: 'A shape, a row or a column moving on the canvas. Overshoot here reads as imprecision.',
  },
};

/** The easing token no document-attached role may use, named once so the gate cannot mistype it. */
export const overshootingEasingToken = 'ease.spring';

/** `panelEnter` → `.mjx-motion-panel-enter`. */
export function motionRoleClass(role: MotionRole): string {
  return `mjx-motion-${role.replace(/[A-Z]/g, (character) => `-${character.toLowerCase()}`)}`;
}

/** The one duration token, as its dotted path, so nothing spells the variable twice. */
export const transitionDurationToken = 'duration.transition';

/** The `var()` every derived span of time is a multiple of. */
export const transitionDurationVariable = 'var(--duration-transition)';

/**
 * `4` → `calc(var(--duration-transition) * 4)`. One multiple is the variable on its own.
 *
 * The mirror of `spacingMultiple` in `density.ts`, and written the same way for the same reason: a
 * span of time expressed as a ratio of the platform's one duration follows a re-seed and follows a
 * host that retunes it, and a span of time expressed as a number does neither.
 */
export function durationMultiple(units: number): string {
  return units === 1 ? transitionDurationVariable : `calc(${transitionDurationVariable} * ${String(units)})`;
}

/**
 * One **registered** `<time>` custom property, in milliseconds, or `undefined` when it is not one.
 *
 * The exact counterpart of `resolveLength` in `overlay/floating.ts`, and it exists for the same
 * measured reason: `getComputedStyle().getPropertyValue()` on an *unregistered* custom property
 * returns the substituted text — `calc(150ms * 4)` — and `Number.parseFloat` on that returns `150`,
 * which is a delay four times too short and looks like nothing at all in a diff. Registering the
 * property with `syntax: '<time>'` is what makes the browser evaluate the `calc()` and hand back a
 * single time.
 *
 * ⚠ **`undefined`, never a stand-in number.** U07's lesson is that a helper whose failure mode is a
 * plausible answer makes every assertion above it pass; a caller here is required to say what it
 * wants to happen when the registration is missing, and the callers in `src/feedback/` fall back to
 * the *generated token value* rather than to zero — because a zero delay would turn the screentip's
 * whole contract into a tautology.
 *
 * Chromium serialises a registered `<time>` in seconds and other engines may not, so both units
 * are read rather than one assumed.
 */
export function resolveDurationMilliseconds(
  element: Element,
  property: string,
): number | undefined {
  const raw = getComputedStyle(element).getPropertyValue(property).trim();
  const match = /^(-?\d+(?:\.\d+)?)(ms|s)$/.exec(raw);
  if (match === null) return undefined;
  const value = Number.parseFloat(match[1] ?? '');
  if (!Number.isFinite(value)) return undefined;
  return match[2] === 's' ? value * 1000 : value;
}

export const motionCss = `
${motionRoleNames
  .map((role) => {
    const spec = motionRoles[role];
    return `:where(.${motionRoleClass(role)}) {
  transition-duration: ${spec.duration};
  transition-timing-function: ${spec.easing};
}`;
  })
  .join('\n')}

/* A person who has asked their operating system to stop moving things has asked this platform too.
 * Stated here, once, rather than in each of fifteen components. */
@media (prefers-reduced-motion: reduce) {
  :where(${motionRoleNames.map((role) => `.${motionRoleClass(role)}`).join(', ')}) {
    transition-duration: 0s;
    animation-duration: 0s;
  }
}
`;
