/**
 * One focus-visible treatment, defined once.
 *
 * MJXOFF-181: *"one focus-visible treatment, defined once, used by every interactive component.
 * Keyboard-only by default, and visible against every surface in both themes."* The reason it is a
 * foundations child's job rather than a button child's is stated in the same ticket: building it
 * inside the first component that needs it *"is how a design system acquires four slightly
 * different button focus rings."*
 *
 * ## The three decisions
 *
 * **Keyboard-only is `:focus-visible` and an explicit denial.** The rules below style
 * `:focus-visible` and additionally clear the ring on `:focus:not(:focus-visible)`, because a
 * component may have set an outline of its own and the point of a single treatment is that there
 * is exactly one. `tests/browser/foundations.spec.ts` drives both halves with a real device: it
 * presses `Tab` and asserts the ring appears, and it clicks and asserts it does not.
 *
 * ⚠ **The denial is not belt-and-braces; it is what makes the treatment keyboard-only at all, and
 * the reason is a specificity accident worth knowing.** `:where()` contributes nothing, but the
 * pseudo-classes *outside* it do — so `:where(…):focus:not(:focus-visible)` scores (0,2,0) and
 * `:where(…):focus-visible` scores (0,1,0). The denial therefore **out-specifies** the ring rather
 * than merely preceding it. MJXOFF-181 found this by breaking the ring rule to `:focus` on purpose
 * and watching the pointer test keep passing: the denial was holding the whole promise on its own.
 * A later child that "simplifies" these two rules into one will not be caught by source order.
 *
 * **The colour is `--theme-accent-pressed`, and that is checked rather than asserted.** It is the
 * scheme-relative pressed accent, so it changes with the theme without this file naming a scheme.
 * Whether it is *visible* is a contrast question with a number attached — WCAG 2.2's non-text
 * contrast minimum of 3 : 1 against the adjacent surface — and `tests/foundations.test.ts`
 * measures it against **every rung of the elevation ladder in both schemes**, including the accent
 * rung, which MJXOFF-181 names as the predictable failure: *a green ring on a green tint*. If a
 * palette re-seed ever breaks one of those pairs the gate names the rung and the scheme. That is
 * the correct outcome: an invisible focus ring is a defect whether or not anyone chose it.
 *
 * **The geometry is spacing-derived, not written down.** `--spacing` is `0.25rem`, so a half-step
 * is the 2px ring and offset that reads at every size. A literal `2px` here would be exactly the
 * defect the literal-value lint exists to catch, and the lint is scoped to `src/`, so this file is
 * subject to it.
 *
 * ## What a component has to do
 *
 * Nothing, if it uses a native focusable. `installFoundations(shadowRoot)` brings the rules with
 * it and the `:where(button, input, …)` selector below picks them up. A custom focusable — a
 * `[tabindex]` on a `<div>` — is covered by the same selector. A component that wants the ring on
 * something that is not itself focusable adds `.mjx-focus-ring` and a `:focus-within`.
 *
 * ## Node-importable
 *
 * Data and strings only.
 */

/** WCAG 2.2's non-text contrast minimum, which a focus indicator must clear against its surface. */
export const focusIndicatorMinimumContrast = 3;

/** The custom properties the treatment reads, so a host or a component may retune one. */
export const focusRingProperties = {
  color: '--mjx-focus-ring-color',
  width: '--mjx-focus-ring-width',
  offset: '--mjx-focus-ring-offset',
} as const;

/**
 * The defaults, in tokens.
 *
 * `color` is the token path the gate measures; `width` and `offset` are `calc()` over `--spacing`
 * so the ring scales with the spacing unit rather than with a number typed here.
 */
export const focusRingDefaults = {
  colorTokenMember: 'accentPressed',
  color: 'var(--theme-accent-pressed)',
  width: 'calc(var(--spacing) / 2)',
  offset: 'calc(var(--spacing) / 2)',
} as const;

/** The selector every native and custom focusable is matched by. Stated once. */
export const focusableSelector =
  'a[href], area[href], button, input, select, summary, textarea, ' +
  '[tabindex]:not([tabindex="-1"]), .mjx-focus-ring';

export const focusCss = `
:where(:root, .mjx-foundations) {
  ${focusRingProperties.color}: ${focusRingDefaults.color};
  ${focusRingProperties.width}: ${focusRingDefaults.width};
  ${focusRingProperties.offset}: ${focusRingDefaults.offset};
}

/* Keyboard-only. A pointer press focuses without :focus-visible, and this is the half that says
 * so out loud — a component that had set an outline of its own would otherwise still show one,
 * and then there would be two treatments again. */
:where(${focusableSelector}):focus:not(:focus-visible) {
  outline: none;
}

:where(${focusableSelector}):focus-visible {
  outline: var(${focusRingProperties.width}) solid var(${focusRingProperties.color});
  outline-offset: var(${focusRingProperties.offset});
}
`;
