/**
 * The two sheets MJXOFF-192 ships, and the one that has to be on the **document**.
 *
 * ## Why the reference colours are declared on `:root` and nowhere else
 *
 * A colour slot is *two* tokens — see `referenceColourSlots` for the measurement that forced it —
 * so the component cannot simply write `var(--color-green-deep)` and be done. Which token a slot
 * resolves to is a different answer in the two schemes, and the only selectors that can say that
 * are `:root`, `:root:not([data-theme="light"])` inside a `prefers-color-scheme` query, and
 * `:root[data-theme="dark"]`. `:host-context()` would have kept it inside the component and is not
 * implemented in Firefox, so it is not an option for a shipped component.
 *
 * This is the arrangement `surfaceSchemeCss` already uses for the scrim, the modal edge and the
 * sheet handle, and it is stated in `surface-model.ts` in as many words: *"custom properties
 * inherit through the flat tree and `:root` is a document selector, so this is the only place these
 * can be declared."* `formulaDocumentCss` carries it, `.storybook/preview.ts` installs it, and a
 * host that forgets gets a formula bar whose references are all `currentColor` rather than a
 * catalogue that silently draws the wrong scheme's colour.
 *
 * ## The alignment invariant, and why it is a list of properties
 *
 * The editor is a `<textarea>` with transparent text over a `<div>` that draws the same text in
 * colour. That is the only arrangement in which the caret, the selection, undo and IME are the
 * platform's own — see `formula-bar.ts` for the alternative that was rejected — and it holds
 * exactly as long as **the two boxes lay text out identically**. So the properties they must share
 * are named once, in [`alignedTextProperties`], written into both rules from that list, and
 * asserted equal by `tests/browser/formula.spec.ts` through `getComputedStyle`. A drift in any one
 * of them is a caret that sits beside its own glyph, which is the classic defect of this technique
 * and is invisible until a line is long enough to wrap.
 */

import { densityProperties, accessibleHitTargetMinimum, spacingMultiple } from '../foundations/density.ts';
import { motionRoleClass } from '../foundations/motion.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { floatingProperties } from '../overlay/floating.ts';
import { customProperties } from '../../tokens/tokens.ts';
import { referenceColourSlots, referenceSlotProperty } from './formula-model.ts';

/** The scheme selectors, spelled exactly as `surface-model.ts` spells them. One contract, one form. */
const schemeSelectors = {
  light: ':root',
  darkPreferred: ':root:not([data-theme="light"])',
  darkExplicit: ':root[data-theme="dark"]',
} as const;

function slotDeclarations(scheme: 'light' | 'dark', indent: string): string {
  return referenceColourSlots
    .map((slot, index) => {
      const property = customProperties[scheme === 'light' ? slot.light : slot.dark];
      return `${indent}${referenceSlotProperty(index)}: var(${property ?? '--theme-text-primary'});`;
    })
    .join('\n');
}

/**
 * The four slot colours, per scheme, on the document.
 *
 * Three rules and not two: the middle one is what makes a host that has *not* chosen a theme follow
 * the operating system, and the `:not([data-theme="light"])` guard is what stops it overriding a
 * host that explicitly chose light. `tokens.css`'s own scheme layer has exactly this shape.
 */
export const formulaSchemeCss = `
${schemeSelectors.light} {
${slotDeclarations('light', '  ')}
}

@media (prefers-color-scheme: dark) {
  ${schemeSelectors.darkPreferred} {
${slotDeclarations('dark', '    ')}
  }
}

${schemeSelectors.darkExplicit} {
${slotDeclarations('dark', '  ')}
}
`;

/**
 * Everything the two layers must agree about, or the caret drifts from its glyph.
 *
 * ⚠ **Read by the stylesheet AND by the gate.** A list that only the CSS knew about would be a list
 * nobody could check, and a gate with its own copy would be two lists that drift.
 */
export const alignedTextProperties: readonly string[] = [
  'font-family',
  'font-size',
  'font-weight',
  'font-style',
  'letter-spacing',
  'line-height',
  'tab-size',
  'text-indent',
  'white-space',
  'overflow-wrap',
  'word-spacing',
  'padding-top',
  'padding-right',
  'padding-bottom',
  'padding-left',
  'border-top-width',
  'border-right-width',
  'border-bottom-width',
  'border-left-width',
];

/** The declarations both layers carry. Written once, interpolated into both rules. */
const alignedText = `
    box-sizing: border-box;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    font-weight: inherit;
    font-style: normal;
    line-height: var(--leading-snug);
    letter-spacing: normal;
    word-spacing: normal;
    tab-size: 4;
    text-indent: 0;
    white-space: pre-wrap;
    overflow-wrap: break-word;
    margin: 0;
    border: 0 solid transparent;
    padding-block: var(${densityProperties.step});
    padding-inline: var(${densityProperties.step});
`;

/** `<mjx-formula-bar>`'s sheet. */
export const formulaBarCss = `
  :host {
    display: block;
    container-type: inline-size;
  }

  .bar {
    display: flex;
    align-items: stretch;
    gap: var(${densityProperties.step});
    box-sizing: border-box;
    background: var(--theme-surface);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-control);
    padding: var(${densityProperties.step});
    color: var(--theme-text-primary);
  }

  .name-slot {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    min-inline-size: ${spacingMultiple(24)};
  }

  .divider {
    flex: 0 0 auto;
    inline-size: 1px;
    background: var(--theme-border-subtle);
  }

  .affordances {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
  }

  .affordance {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 1px solid transparent;
    border-radius: var(--radius-chip);
    color: var(--theme-text-primary);
    cursor: pointer;
    font: inherit;
    padding: 0;
    transition-property: background, border-color, color;
    transition-duration: var(--duration-transition);
    transition-timing-function: var(--ease-out-soft);
  }

  .affordance:hover:not([disabled]) {
    background: var(--theme-accent-surface);
  }

  .affordance[disabled] {
    color: var(--theme-text-secondary);
    cursor: default;
    opacity: 0.55;
  }

  .affordance.insert {
    font-family: var(--font-serif);
    font-style: italic;
    font-weight: var(--font-weight-semibold);
  }

  .editor {
    position: relative;
    flex: 1 1 auto;
    min-inline-size: 0;
  }

  .backdrop {
    position: absolute;
    inset: 0;
    overflow: hidden;
    pointer-events: none;
    user-select: none;
    color: var(--theme-text-primary);
${alignedText}
  }

  .entry {
    position: relative;
    display: block;
    inline-size: 100%;
    background: transparent;
    /*
     * ⚠ Transparent, with a visible caret. The colour a person SEES is the backdrop's, one layer
     * down; this element exists for the caret, the selection, undo and the IME, all of which are
     * the platform's own and none of which a re-implementation gets right.
     */
    color: transparent;
    caret-color: var(--theme-text-primary);
    resize: none;
    overflow: auto;
${alignedText}
  }

  .entry::selection {
    background: var(--theme-accent-surface);
  }

  .entry:focus-visible {
    outline: none;
  }

  .bar:focus-within {
    border-color: var(--theme-accent-border);
  }

  .token-reference { color: var(--mjx-formula-reference-0); }
  .token-reference[data-slot='1'] { color: var(${referenceSlotProperty(1)}); }
  .token-reference[data-slot='2'] { color: var(${referenceSlotProperty(2)}); }
  .token-reference[data-slot='3'] { color: var(${referenceSlotProperty(3)}); }

  .token-functionName {
    font-weight: var(--font-weight-semibold);
  }

  .token-string,
  .token-number,
  .token-errorValue {
    color: var(--theme-text-secondary);
  }

  /*
   * A matched pair is a FILL rather than a colour: every text colour in this component has to clear
   * 4.5 : 1 against the surface, and the palette has four such colours, all four of which the
   * reference ring has already spent.
   */
  .bracket-match {
    background: var(--theme-accent-surface);
    border-radius: var(--radius-chip);
  }

  /*
   * And an unmatched one is a DECORATION rather than a colour, for the same reason plus one more:
   * --theme-secondary-accent is 3.44 : 1 on the light surface, so it may underline text and may
   * not be text. (No backtick in this comment: one inside a CSS comment ends the template literal
   * it is in, which surface-model.ts records having cost it a build. It has now cost two.)
   */
  .bracket-unmatched {
    text-decoration: underline wavy;
    text-decoration-color: var(--theme-secondary-accent);
    text-underline-offset: 0.2em;
  }

  .handle {
    flex: 0 0 auto;
    align-self: stretch;
    display: flex;
    align-items: center;
    justify-content: center;
    background: transparent;
    border: 0;
    border-radius: var(--radius-chip);
    color: var(--theme-text-secondary);
    cursor: ns-resize;
    padding: 0;
    touch-action: none;
  }

  .handle:hover {
    background: var(--theme-accent-surface);
  }

  .grip {
    inline-size: 60%;
    block-size: 1px;
    background: currentColor;
    box-shadow: 0 0.2em currentColor;
  }

  .mode {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    margin-block-start: var(${densityProperties.step});
    padding-inline: var(${densityProperties.step});
    border-radius: var(--radius-chip);
    background: var(--theme-surface-raised);
    color: var(--theme-text-primary);
    border: 1px solid var(--theme-border-subtle);
  }

  .mode[data-mode='point'],
  .mode[data-mode='enter'],
  .mode[data-mode='edit'] {
    background: var(--theme-accent-surface);
    border-color: var(--theme-accent-border);
  }

  .mode-dot {
    inline-size: 0.5em;
    block-size: 0.5em;
    border-radius: 50%;
    background: var(--theme-text-secondary);
  }

  .mode[data-mode='enter'] .mode-dot,
  .mode[data-mode='edit'] .mode-dot,
  .mode[data-mode='point'] .mode-dot {
    background: var(--theme-accent-pressed);
  }

  /*
   * Four modes, four shapes as well as four fills: a state told only in colour is a state a person
   * who cannot separate two hues cannot read. Ready is a hollow ring, Enter a filled dot, Edit a
   * bar and Point a diamond.
   */
  .mode[data-mode='ready'] .mode-dot {
    background: transparent;
    border: 1px solid var(--theme-text-secondary);
  }

  .mode[data-mode='edit'] .mode-dot {
    border-radius: 0;
    block-size: 0.25em;
  }

  .mode[data-mode='point'] .mode-dot {
    border-radius: 0;
    rotate: 45deg;
  }

  .tooltip {
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    z-index: 1;
    max-inline-size: var(${floatingProperties.maxInlineSize}, none);
    box-sizing: border-box;
    margin: 0;
    padding: var(${densityProperties.step}) var(${densityProperties.gutter});
    background: var(--theme-surface-raised);
    color: var(--theme-text-primary);
    border: 1px solid var(--theme-border);
    border-radius: var(--radius-chip);
    box-shadow: var(--shadow-lift);
    font-family: var(--font-mono);
    pointer-events: none;
  }

  .tooltip[hidden] {
    display: none;
  }

  .tooltip .emphasis {
    font-weight: var(--font-weight-bold);
    text-decoration: underline;
    text-underline-offset: 0.2em;
  }

  .tooltip .summary {
    display: block;
    font-family: var(--font-sans);
    color: var(--theme-text-secondary);
  }

  .live,
  .visually-hidden {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    margin: -1px;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }
`;

/** `<mjx-name-box>`'s own additions on top of U07's list-field sheet. */
export const nameBoxCss = `
  :host {
    display: inline-block;
    min-inline-size: ${spacingMultiple(24)};
  }

  .field {
    font-family: var(--font-mono);
  }

  .problem {
    display: block;
    color: var(--theme-text-primary);
  }

  .problem[hidden] {
    display: none;
  }
`;

/**
 * The rules a **document** carries.
 *
 * Two jobs, the same two `surfaceDocumentCss` has: the scheme-keyed properties, which can only live
 * on `:root`; and a size for the elements before they upgrade, so a bar that has not been defined
 * yet does not flash a bare `<textarea>` and an unstyled row of buttons into the layout.
 */
export const formulaDocumentCss = `
${formulaSchemeCss}

:where(mjx-formula-bar:not(:defined)) {
  display: block;
  min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
}

:where(mjx-name-box:not(:defined)) {
  display: inline-block;
  min-inline-size: ${spacingMultiple(24)};
}
`;

/** The type roles each part of the bar wears, named once so a gate reads them rather than guessing. */
export const formulaTypeRoles = {
  editor: typeRoleClass('control'),
  mode: typeRoleClass('dense'),
  tooltip: typeRoleClass('dense'),
} as const;

/** The motion role the tooltip and the autocomplete enter with. */
export const formulaMotionClass = motionRoleClass('panelEnter');
