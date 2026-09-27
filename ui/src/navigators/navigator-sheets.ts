/**
 * The four navigators' stylesheets.
 *
 * Split out of `navigator-model.ts` for one reason and it is not size: **the model is what the two
 * tiers read to decide whether the components are right**, and a file that is half ARIA tables and
 * half CSS invites a gate to compare a stylesheet against the table it was generated from — which
 * is U06's second green-under-a-real-break defect, stated as a filing rule.
 *
 * Nothing here declares a colour, a radius, a spacing or a duration. `mjx/no-literal-design-values`
 * refuses it, and every value below is a token, a `calc()` over one, or a relative unit.
 *
 * ⚠ **A backtick inside a CSS comment ends the template literal.** It has bitten four children in
 * this catalogue. Every comment below is written in plain words for that reason.
 */

import {
  controlStatesCss,
  disabledOpacity,
  themeVariable,
} from '../controls/control-states.ts';
import {
  accessibleHitTargetMinimum,
  densityProperties,
} from '../foundations/density.ts';
import { focusRingProperties } from '../foundations/focus.ts';
import { radiusVariable, surfaceLevels } from '../foundations/surfaces.ts';
import { galleryCellStatesCss } from '../gallery/gallery-model.ts';
import {
  navigatorPhoneSheetAtOrBelow,
  navigatorPresentationProperty,
} from './navigator-model.ts';
import { containerName } from '../harness/presets.ts';

/**
 * The rules every navigator's scroller shares: the box, the two spacers, the row paint and the
 * keyboard cursor.
 *
 * ⚠ **The row paint is `galleryCellStatesCss` and not a fifth state table.** A row of a listbox,
 * a treeitem and a slide in a sorter are the same seven states over the same shared control paint —
 * resting, hovered, held, selected, selected-and-hovered, unavailable — and U06's table is already
 * measured against the generated tokens by its own pairwise and correspondence gates. A second
 * table would be a second answer to *what does selected look like*, which is how a catalogue
 * acquires two slightly different highlights.
 */
function scrollerCss(rowSelector: string): string {
  return `
  .viewport {
    position: relative;
    overflow: auto;
    overscroll-behavior: contain;
    block-size: 100%;
    min-block-size: 0;
    background: ${surfaceLevels.raised.background};
    color: ${themeVariable('textPrimary')};
  }

  /* The two spacers stand in for every row the window did not build, so the scrollbar is as long
   * as the whole collection rather than as long as what is on screen. They carry
   * role=presentation, because a div where an option belongs is a real accessibility violation and
   * U06 found that one with axe rather than by reading. */
  .spacer {
    flex: 0 0 auto;
    inline-size: 100%;
  }

  ${rowSelector} {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
    inline-size: 100%;
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.step});
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    border-radius: ${radiusVariable('control')};
    cursor: default;
    user-select: none;
  }

${galleryCellStatesCss(rowSelector)}

  /* The keyboard cursor, and it is deliberately not the focus ring. Focus stays on the container
   * because these are aria-activedescendant lists, so no row is ever focus-visible and the
   * foundations treatment can never reach one; without a rule of its own an arrow key would move an
   * announcement nobody can see. The width, the colour and the offset are the ring's own custom
   * properties, so a host that retunes the ring retunes this with it. The offset is negative
   * because an outline drawn outside a row inside a scroll container is clipped at the edges. */
  .viewport:focus-visible ${rowSelector}[data-cursor='true'] {
    outline: var(${focusRingProperties.width}) solid var(${focusRingProperties.color});
    outline-offset: calc(var(${focusRingProperties.offset}) * -1);
  }

  /* Drag feedback. The row being carried dims and the gap it would land in draws a line. Neither is
   * announced: the keyboard path announces its own outcome, and saying it again for a pointer user
   * who can see the line is noise. */
  ${rowSelector}[data-dragging='true'] { opacity: ${String(disabledOpacity)}; }

  ${rowSelector}[data-drop='before'] {
    box-shadow: inset 0 var(${focusRingProperties.width}) 0 0 var(${focusRingProperties.color});
  }

  ${rowSelector}[data-drop='after'] {
    box-shadow: inset 0 calc(var(${focusRingProperties.width}) * -1) 0 0 var(${focusRingProperties.color});
  }
`;
}

/** The visually-hidden region a refused or completed reorder is announced through. */
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
 * The last line of every sheet here, and it is load-bearing.
 *
 * MJXOFF-189: the user agent's own hidden rule lives in the user-agent origin, so any author rule
 * at all beats it. Every display declaration above is an author rule, so without this line an
 * element with the attribute set stays on screen with a correct accessibility tree and a passing
 * attribute assertion. Restated last, at the same specificity as the class rules, because source
 * order is the only thing that decides a tie between equals.
 */
const hiddenLast = `
  [hidden] { display: none !important; }
`;

/**
 * At a phone width the tree and the rail take the whole frame.
 *
 * The width is the same number the ribbon's tab picker, the menu's sheet and the gallery's sheet
 * use, and it is an alias rather than a fifth definition.
 *
 * ⚠ **In flow, and deliberately not `position: fixed`.** U05 measured the thing that makes the
 * obvious spelling wrong: **Chromium does not make a `container-type` element a containing block for
 * fixed descendants**, whatever CSS Containment's layout-containment paragraph reads like — so a
 * pane pinned with `inset: 0` inside the harness frame spans the *window* rather than the phone.
 * A navigator that simply takes all the room its frame has is the same presentation with none of
 * that, and the browser gate can measure it against the frame's own box.
 *
 * ⚠ **And the rule is on the host rather than on an attribute.** An earlier version keyed it off
 * `[sheet]`, which nothing set and no story exercised — a presentation that is declared and never
 * reached is worse than one that does not exist, because it reads as covered.
 */
const phoneSheetCss = `
  @container ${containerName} (width <= ${String(navigatorPhoneSheetAtOrBelow)}px) {
    :where(:host) {
      ${navigatorPresentationProperty}: sheet;
      inline-size: 100%;
      max-inline-size: none;
      border-radius: 0;
    }
  }
`;

/**
 * The resting presentation, and it is `:where()`-wrapped for the reason MJXOFF-183 found four times.
 *
 * ⚠ **A `@container` block changes no specificity.** A base rule written as a bare `:host` scores
 * (0,1,0) and the block above scores (0,0,0), so the base would win at *every* width and the
 * component would report `pane` on a phone with every other assertion green. Both are inside
 * `:where()`, so source order is the whole arbitration — and the phone block is emitted **after**
 * this one. `tests/navigators.test.ts` asserts the order *and* the equal specificity that makes
 * order matter, because U04 is the record that order arbitrates nothing between rules that are not.
 */
const paneBaseCss = `
  :where(:host) {
    ${navigatorPresentationProperty}: pane;
    border-radius: ${radiusVariable('card')};
  }
`;

/** `<mjx-virtual-list>`. The shared primitive with nothing added to it. */
export const virtualListCss = `
  :host {
    display: block;
    box-sizing: border-box;
    block-size: 100%;
    min-block-size: 0;
  }

  :host([hidden]) { display: none; }

${scrollerCss('.row')}

  .label {
    flex: 1 1 auto;
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .detail {
    flex: 0 0 auto;
    color: ${themeVariable('textSecondary')};
  }

${liveRegionCss}
${hiddenLast}
`;

/** `<mjx-tree>`. The list, plus a depth, a twisty and the indentation that shows both. */
export const treeCss = `
  :host {
    display: block;
    box-sizing: border-box;
    block-size: 100%;
    min-block-size: 0;
  }

  :host([hidden]) { display: none; }

${paneBaseCss}
${scrollerCss('.row')}

  /* The indentation is a padding derived from the row's own level, published as a custom property
   * per row. A margin would have moved the row's box, and the row's box is what the selection
   * paints -- an indented child would then draw a shorter highlight than its parent, which reads as
   * a rendering fault rather than as a hierarchy. */
  .row {
    padding-inline-start: calc(
      var(${densityProperties.gutter}) + var(${densityProperties.gutter}) * var(--mjx-tree-level, 0)
    );
  }

  .twisty {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: 1.25em;
    block-size: 1.25em;
    border: 0;
    padding: 0;
    background: none;
    color: inherit;
  }

  /* A leaf keeps the box and loses the glyph, so every label in a branch starts at the same inline
   * position. Visibility rather than display: that is the difference between a column that lines up
   * and one that shifts by a glyph on every expand. */
  .twisty[data-leaf='true'] { visibility: hidden; }

  .label {
    flex: 1 1 auto;
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

${phoneSheetCss}
${liveRegionCss}
${hiddenLast}
`;

/** `<mjx-thumbnail-rail>`. A number, a picture that may not have arrived yet, and a caption. */
export const thumbnailRailCss = `
  :host {
    display: block;
    box-sizing: border-box;
    block-size: 100%;
    min-block-size: 0;
  }

  :host([hidden]) { display: none; }

${paneBaseCss}
${scrollerCss('.slide')}

  .slide {
    align-items: flex-start;
    gap: var(${densityProperties.gutter});
  }

  .number {
    flex: 0 0 auto;
    min-inline-size: 2ch;
    text-align: end;
    color: ${themeVariable('textSecondary')};
  }

  .plate {
    flex: 0 0 auto;
    position: relative;
    inline-size: 40%;
    aspect-ratio: 16 / 9;
    overflow: hidden;
    border: 1px solid ${themeVariable('border')};
    border-radius: ${radiusVariable('chip')};
    background: ${themeVariable('background')};
  }

  .plate img {
    inline-size: 100%;
    block-size: 100%;
    object-fit: contain;
    display: block;
  }

  /* The placeholder, and it is a STATE rather than an absence. R10's plate generator produces a
   * thumbnail asynchronously, so a rail that blocked on rendering would be unusable and a rail that
   * drew nothing at all would be indistinguishable from one that had failed. The band is a gradient
   * over two token colours -- no third colour is introduced -- and it stops moving under a reduced
   * motion preference, where the band alone still says waiting. */
  .placeholder {
    position: absolute;
    inset: 0;
    background: linear-gradient(
      100deg,
      ${themeVariable('background')} 40%,
      ${themeVariable('borderSubtle')} 50%,
      ${themeVariable('background')} 60%
    );
    background-size: 300% 100%;
    animation: mjx-rail-pending calc(var(--duration-transition) * 8) linear infinite;
  }

  @keyframes mjx-rail-pending {
    from { background-position: 150% 0; }
    to { background-position: -150% 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .placeholder { animation: none; }
  }

  .caption {
    flex: 1 1 auto;
    min-inline-size: 0;
    display: flex;
    flex-direction: column;
    gap: calc(var(${densityProperties.step}) / 2);
    overflow-wrap: anywhere;
  }

  .marks {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    color: ${themeVariable('textSecondary')};
  }

  .section {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    inline-size: 100%;
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.step});
    color: ${themeVariable('textSecondary')};
    background: ${themeVariable('background')};
  }

${phoneSheetCss}
${liveRegionCss}
${hiddenLast}
`;

/** `<mjx-sheet-tab-bar>`. Excel's tabs: overflow, colour, rename in place, the new-sheet control. */
export const sheetTabBarCss = `
  :host {
    display: block;
    box-sizing: border-box;
    background: ${surfaceLevels.raised.background};
    border-block-start: ${surfaceLevels.raised.border};
    color: ${themeVariable('textPrimary')};
  }

  :host([hidden]) { display: none; }

  .bar {
    display: flex;
    align-items: stretch;
    gap: var(${densityProperties.step});
    padding-inline: var(${densityProperties.step});
    min-inline-size: 0;
  }

  /* The scroll affordances sit OUTSIDE the tablist, and that is a requirement rather than a layout
   * preference: a tablist's own children must be tabs, and a button among them is a real
   * aria-required-children violation. They are not tabs in any other sense either -- pressing one
   * scrolls the strip and changes nothing about which sheet is showing. */
  .affordances,
  .trailing {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    gap: calc(var(${densityProperties.step}) / 2);
  }

  .affordance,
  .add {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-inline-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    padding: 0;
    border: 1px solid transparent;
    border-radius: ${radiusVariable('control')};
    background: none;
    color: inherit;
  }

  .strip {
    flex: 1 1 auto;
    display: flex;
    align-items: stretch;
    gap: var(${densityProperties.step});
    min-inline-size: 0;
    overflow-x: auto;
    overflow-y: hidden;
    scrollbar-width: none;
  }

${controlStatesCss('.tab')}

  .tab {
    position: relative;
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    padding-inline: var(${densityProperties.gutter});
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    border-width: 1px;
    border-style: solid;
    border-start-start-radius: ${radiusVariable('control')};
    border-start-end-radius: ${radiusVariable('control')};
    white-space: nowrap;
  }

  /* The sheet's colour is drawn as an EDGE and never as a fill. It belongs to the user's workbook
   * and may be anything at all, so a label drawn on it would have a contrast nobody has checked. As
   * a bar along the tab's own edge the user's choice stays visible and the label stays on a surface
   * whose contrast is the palette's and is measured. */
  .tab-colour {
    position: absolute;
    inset-inline: 0;
    inset-block-end: 0;
    block-size: calc(var(--spacing) / 2);
    background: var(--mjx-sheet-colour, transparent);
  }

  /* A hidden sheet is told apart by its EDGE and by a mark, and never by dimming. The obvious
   * spelling was an opacity on the label, and the accessibility sweep measured what it costs:
   * 2.79 to 1, against a floor of 4.5. A native disabled control is exempt from that rule because
   * the platform dims it; a hidden sheet is not disabled at all -- it is reachable, selectable and
   * showable -- so nothing exempts it and nothing should. Both cues here are full contrast. */
  .tab[data-hidden='true'] {
    border-style: dashed;
  }

  .tab .mark {
    flex: 0 0 auto;
  }

  .rename {
    box-sizing: border-box;
    min-inline-size: 8ch;
    font: inherit;
    color: ${themeVariable('textPrimary')};
    background: ${themeVariable('surface')};
    border: 1px solid ${themeVariable('accent')};
    border-radius: ${radiusVariable('chip')};
    padding-inline: var(${densityProperties.step});
  }

  .problem {
    flex: 0 0 auto;
    align-self: center;
    color: ${themeVariable('textPrimary')};
    background: ${themeVariable('surface')};
    border: 1px solid ${themeVariable('secondaryAccent')};
    border-radius: ${radiusVariable('chip')};
    padding-inline: var(${densityProperties.step});
  }

${liveRegionCss}
${hiddenLast}
`;
