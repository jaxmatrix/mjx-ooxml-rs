/**
 * MJXOFF-194's stylesheets, and the four safe-area properties that can only live on the document.
 *
 * Split out of `mobile-model.ts` for the reason `navigator-sheets.ts` gives: the model is what the
 * two tiers read to decide whether the components are right, and a file that is half arithmetic and
 * half CSS invites a gate to compare a stylesheet against the table it was generated from.
 *
 * ⚠ **`mobileDocumentCss` must reach the document.** It carries the four registered safe-area
 * properties, and a registration is not optional decoration: an unregistered custom property
 * reports its *substituted text* to `getComputedStyle`, so a padding built on one resolves to
 * nothing and a notched phone silently loses its inset. This is the same arrangement
 * `surface-model.ts`, `formula-sheets.ts` and `annotation-sheets.ts` already use, and it is
 * installed in `.storybook/preview.ts` beside them.
 *
 * ⚠ **A backtick inside a CSS comment ends the template literal.** It has bitten six children in
 * this catalogue. Every comment below is written in plain words for that reason.
 */

import { controlStatesCss, themeVariable } from '../controls/control-states.ts';
import {
  accessibleHitTargetMinimum,
  densityProperties,
  spacingMultiple,
} from '../foundations/density.ts';
import { motionRoleClass } from '../foundations/motion.ts';
import { radiusVariable } from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { gestureRegions } from './gesture-map.ts';
import {
  mobilePresentationCss,
  mobileTags,
  safeAreaProperties,
  safeAreaSources,
} from './mobile-model.ts';
import { mobileBarTargetMinimum } from './touch-audit.ts';

/**
 * The four safe-area insets, registered and fed from the environment.
 *
 * Registered as a length with a zero initial value, so a browser with no notch resolves them to
 * zero rather than to nothing — the difference matters, because a `calc()` over an unresolvable
 * custom property is itself unresolvable and takes the whole declaration with it.
 */
export const mobileDocumentCss = `
${(Object.keys(safeAreaProperties) as (keyof typeof safeAreaProperties)[])
  .map(
    (side) => `@property ${safeAreaProperties[side]} {
  syntax: '<length>';
  inherits: true;
  initial-value: 0px;
}`,
  )
  .join('\n')}

/* The environment feeds the properties, and every component reads the property. A notched device
 * writes these; a gate writes the same values by setting the property directly, which is the same
 * number arriving down the same channel rather than a stub. */
:where(:root, .mjx-foundations) {
${(Object.keys(safeAreaProperties) as (keyof typeof safeAreaProperties)[])
  .map((side) => `  ${safeAreaProperties[side]}: env(${safeAreaSources[side]}, 0px);`)
  .join('\n')}
}

/* A bar written as markup must not flash into the page in the moment between parsing and
 * upgrading: an un-upgraded custom element is an unknown inline element, so a bar's worth of
 * commands would lay out as a paragraph of run-together words across the document. */
:where(${mobileTags.commandBar}:not(:defined), ${mobileTags.contextualActionBar}:not(:defined)) {
  display: none;
}
`;

/**
 * The last line of every sheet here. See navigator-sheets.ts for why it is last and not first.
 *
 * ⚠ **Both selectors, and the inner one is the load-bearing half.** The user agent's own hidden
 * rule lives in the user-agent origin, where any author rule at all beats it — and the command rule
 * below writes `display: inline-flex` on a class the overflow control wears. Without this line an
 * overflow control with `hidden` set is on screen, in the accessibility tree, and correct by every
 * attribute assertion. MJXOFF-189 found this with a visibility check where an attribute check would
 * have passed; this is the same trap, met knowingly.
 */
const hiddenLast = `
  :host([hidden]) { display: none !important; }
  .command[hidden],
  .overflow[hidden] { display: none !important; }
`;

/** A polite live region that is present but not seen. */
const liveRegionCss = `
  .live {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    margin: 0;
    padding: 0;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
    border: 0;
  }
`;

/**
 * The bar both mobile components are.
 *
 * ## One rail, two presentations — which is why nothing can be lost
 *
 * MJXOFF-183's rule, applied a second time. The obvious way to build an overflow is to render the
 * visible commands in the bar and the rest into a menu; that is also the way to lose one, and it is
 * the way that makes *reachable* a thing the implementation has to remember. So every command is
 * built **once**, into one rail, and the overflow control changes what that rail *is*: a horizontal
 * scroller pinned to the bar, or a wrapped grid floating above it. The DOM nodes are the same in
 * both, so nothing can be demoted out of existence.
 *
 * ## The two touch-action values, and where they come from
 *
 * They are read from `gestureRegions` rather than written here, so the CSS cannot disagree with the
 * gesture map the audit reads. A closed rail is the region called commandBar and takes the inline
 * axis; an open one is the region called overflowPanel and takes the block axis. The component sets
 * the data-region attribute, and the browser gate compares the computed value against the map.
 */
const barCss = `
  :host {
    display: block;
    box-sizing: border-box;
  }

  .bar {
    position: relative;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
    background: ${themeVariable('surface')};
    color: ${themeVariable('textPrimary')};
    border-block-start: 1px solid ${themeVariable('borderSubtle')};
    /* No padding of its own: the commands are already at their hit-target size, so the only space
     * left is the safe-area inset, which a notched phone still needs to clear the home indicator. */
    padding-inline: var(${safeAreaProperties.inlineStart});
    padding-block-start: 0;
    padding-block-end: var(${safeAreaProperties.blockEnd});
  }

  /* A bar that spans its container has square corners and a full-bleed edge; one that floats clear
   * of it is a card. Which of the two is a form-factor decision and is read from the cascade. */
  .bar[data-spans='false'] {
    margin-inline: var(${densityProperties.gutter});
    margin-block-end: var(${densityProperties.gutter});
    border: 1px solid ${themeVariable('border')};
    border-radius: ${radiusVariable('panel')};
    box-shadow: var(--shadow-lift);
  }

  /* THE RAIL. One element, two presentations. */
  .rail {
    flex: 1 1 auto;
    min-inline-size: 0;
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
    overflow-x: auto;
    overflow-y: hidden;
    overscroll-behavior: contain;
    scrollbar-width: none;
    touch-action: ${gestureRegions.commandBar.touchAction};
  }

  .rail::-webkit-scrollbar { display: none; }

  .rail[data-region='overflowPanel'] {
    position: absolute;
    inset-inline: 0;
    inset-block-end: 100%;
    z-index: 1;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(${spacingMultiple(18)}, 1fr));
    align-content: start;
    max-block-size: ${spacingMultiple(60)};
    overflow-x: hidden;
    overflow-y: auto;
    touch-action: ${gestureRegions.overflowPanel.touchAction};
    padding: var(${densityProperties.gutter});
    margin-block-end: var(${densityProperties.step});
    background: ${themeVariable('surfaceRaised')};
    border: 1px solid ${themeVariable('border')};
    border-radius: ${radiusVariable('panel')};
    box-shadow: var(--shadow-lift);
  }

  /* THE COMMAND. The floor here is higher than the catalogue's: a bar that exists only on a phone
   * has no excuse for the WCAG minimum, so it takes the density system's comfortable target and
   * never less than the mobile floor.
   *
   * ⚠ This rule paints NOTHING, for the reason controlBaseCss states at length: it scores (0,1,0)
   * and every rule the state table emits scores (0,0,0), so one background or one font shorthand
   * here would out-specify the whole table and leave a command rendering its resting paint in
   * every state. The font is named by its longhands for exactly that reason -- the shorthand sets
   * font-weight, which the table owns. */
  .command,
  .overflow {
    flex: 0 0 auto;
    box-sizing: border-box;
    display: inline-flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 0;
    margin: 0;
    min-inline-size: max(var(${densityProperties.hitTarget}), ${String(mobileBarTargetMinimum)}px);
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(mobileBarTargetMinimum)}px);
    padding-inline: var(${densityProperties.step});
    padding-block: 0;
    border-width: 1px;
    border-radius: ${radiusVariable('control')};
    font-family: inherit;
    font-size: inherit;
    text-align: center;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    touch-action: ${gestureRegions.commandBarButton.touchAction};
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  /* In the grid presentation a command carries its label as well as its icon, because a grid has
   * the room and because a person who opened the overflow is looking for something by name. */
  .rail[data-region='overflowPanel'] .command {
    flex-direction: row;
    justify-content: flex-start;
    gap: var(${densityProperties.step});
    inline-size: 100%;
    padding-inline: var(${densityProperties.step});
  }

  .name {
    display: none;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .rail[data-region='overflowPanel'] .name { display: block; }

  .overflow {
    position: relative;
    z-index: 2;
  }

${controlStatesCss('.command')}
${controlStatesCss('.overflow')}

  /* Nothing to show is not an empty bar, it is no bar. The contextual action bar with no selection
   * is the case, and it is a behaviour rather than an emptiness. */
  :host([data-empty='true']) { display: none; }

${liveRegionCss}
${hiddenLast}
`;

/** `<mjx-command-bar>` — the bar, plus the form-factor blocks. */
export const commandBarCss = `
${barCss}

${mobilePresentationCss('.bar')}
`;

/** `<mjx-contextual-action-bar>` — the same bar, with the selection band above it. */
export const contextualActionBarCss = `
${barCss}

  /* The selection readout. A contextual bar that did not say what it was acting on would be a row
   * of verbs with no object, which is the one thing a person needs to know before pressing one. */
  .selection {
    flex: 0 0 auto;
    align-self: center;
    padding-inline: var(${densityProperties.step});
    color: ${themeVariable('textSecondary')};
    white-space: nowrap;
  }

  .bar[data-form-factor='phonePortrait'] .selection {
    display: none;
  }

${mobilePresentationCss('.bar')}
`;

/**
 * The type roles each part of a bar uses, so a gate reads the choice from the table.
 *
 * A command's own label is `control`; the overflow grid's names are `control` too, because they are
 * the same words at the same size; the selection readout is `dense`, because it is a reading rather
 * than a command.
 */
export const mobileTypeRoles = {
  command: typeRoleClass('control'),
  selection: typeRoleClass('dense'),
} as const;

/**
 * The motion role a bar's overflow panel uses.
 *
 * `panelEnter`, not `sheetEnter`: the panel is chrome opening from chrome, and the sheet role's
 * longer, softer entrance is the one a surface arriving from off-screen wants. **Read by the
 * component, not merely exported** — a motion role that is exported and never applied is the shape
 * of thing CLAUDE.md calls worse than none.
 */
export const overflowPanelMotionClass = motionRoleClass('panelEnter');

/**
 * The sheet's own additions, which complete MJXOFF-188's rather than replacing it.
 *
 * Three declarations and a comment each, appended to `dialogCss` by `<mjx-dialog>` itself:
 *
 * * the safe-area inset on a sheet's block end, so the content clears the home indicator;
 * * the two grab regions' touch-action values, read from the gesture map;
 * * the body's own scroller, which is what a nested list scrolls inside.
 */
export const sheetCompletionCss = `
  /* A sheet sits ON the edge it is pinned to, so its own block-end padding is the only thing
   * between its last row and the device's home indicator. */
  .surface[data-presentation='sheet'] {
    padding-block-end: var(${safeAreaProperties.blockEnd});
  }

  .surface[data-presentation='sheet'] .handle {
    touch-action: ${gestureRegions.sheetHandle.touchAction};
    cursor: grab;
    /* The grab area, not the drawing. The bar a person sees is a few pixels tall; the region a
     * thumb has to hit is a whole target, and the two are deliberately different sizes. */
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    align-items: center;
  }

  .surface[data-presentation='sheet'] .header {
    touch-action: ${gestureRegions.sheetHeader.touchAction};
  }

  /* THE NESTED SCROLLER, and only one declaration of it is this child's.
   *
   * MJXOFF-188 already made the body a scroller with overscroll-behavior: contain -- which is the
   * CSS half of the nested-scroll rule and stops an over-scroll inside the sheet reaching the page
   * behind it. What was missing is the touch-action, which is what says the block axis is the
   * scroller's and the inline axis and both multi-touch gestures are still the document's.
   * sheetDragClaim is the rest, and it is what decides whether a downward drag scrolls this or
   * moves the sheet. */
  .surface[data-presentation='sheet'] .body {
    touch-action: ${gestureRegions.sheetScroller.touchAction};
  }

  /* A drag is direct manipulation and must not be interpolated: a sheet that lagged the finger by a
   * transition duration would feel broken however short the duration was. */
  .surface[data-dragging='true'] {
    transition-property: none;
  }
`;

/** The attribute a bar reflects its form factor through, for a story and for the gate. */
export const mobileFormFactorAttribute = 'data-form-factor';
