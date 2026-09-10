/**
 * MJXOFF-193's stylesheets, and the **eight scheme-keyed author colours** that can only live on the
 * document.
 *
 * Split out of `annotation-model.ts` for the reason `navigator-sheets.ts` gives: the model is what
 * the two tiers read to decide whether the components are right, and a file that is half packing
 * arithmetic and half CSS invites a gate to compare a stylesheet against the table it was generated
 * from.
 *
 * ⚠ **`annotationDocumentCss` must reach the document.** An author colour slot is *two* tokens —
 * see `author-colour.ts` for the measurement that forced it — so a component cannot write
 * var(--color-ink) and be done: which token spells a slot is a different answer in the two schemes,
 * and the only selectors that can say that are the three on :root. This is the arrangement
 * surface-model.ts and formula-sheets.ts already use, for the same reason and in the same three
 * rules. A shell that forgets gets a review pane whose author bands are all the border colour.
 *
 * ⚠ **A backtick inside a CSS comment ends the template literal.** It has bitten five children in
 * this catalogue, twice in a paragraph explaining a colour choice. Every comment below is written
 * in plain words.
 */

import {
  controlStatesCss,
  themeVariable,
} from '../controls/control-states.ts';
import { accessibleHitTargetMinimum, densityProperties, spacingMultiple } from '../foundations/density.ts';
import { motionRoleClass } from '../foundations/motion.ts';
import { radiusVariable } from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { containerName } from '../harness/presets.ts';
import { customProperties } from '../../tokens/tokens.ts';
import {
  authorColourProperty,
  authorColourSlots,
} from './author-colour.ts';
import {
  annotationTags,
  reviewPresentationProperty,
  reviewSheetAtOrBelow,
} from './annotation-model.ts';

/** The scheme selectors, spelled exactly as surface-model.ts spells them. One contract, one form. */
const schemeSelectors = {
  light: ':root',
  darkPreferred: ':root:not([data-theme="light"])',
  darkExplicit: ':root[data-theme="dark"]',
} as const;

function slotDeclarations(scheme: 'light' | 'dark', indent: string): string {
  return authorColourSlots
    .map((slot, index) => {
      const property = customProperties[scheme === 'light' ? slot.light : slot.dark];
      return `${indent}${authorColourProperty(index)}: var(${property ?? '--theme-border'});`;
    })
    .join('\n');
}

/**
 * The eight author colours, per scheme, on the document.
 *
 * Three rules and not two: the middle one is what makes a host that has not chosen a theme follow
 * the operating system, and the not-data-theme-light guard is what stops it overriding a host that
 * explicitly chose light.
 */
export const annotationDocumentCss = `
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

/* A card written as markup must not flash into the page in the moment between parsing and
 * upgrading: an un-upgraded custom element is an unknown inline element, so a review pane's worth
 * of author names would lay out as a paragraph of run-together text. */
:where(${annotationTags.commentCard}:not(:defined),
       ${annotationTags.commentThread}:not(:defined),
       ${annotationTags.trackedChangeCard}:not(:defined),
       ${annotationTags.reviewPane}:not(:defined)) {
  display: none;
}
`;

/** The last line of every sheet here. See navigator-sheets.ts for why it is last and not first. */
const hiddenLast = `
  :host([hidden]) { display: none !important; }
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
 * The card's own chrome: the author band, the dot, the head, the body and the action row.
 *
 * The band is the strongest scanning cue in the margin and it is the reason the author colour is a
 * filled area rather than a text colour. It runs the full block size of the card on the leading
 * edge, and the dot repeats it beside the author's name so a card clipped by the viewport still
 * says who wrote it.
 */
const cardShellCss = `
  :host {
    display: block;
    box-sizing: border-box;
    min-inline-size: 0;
  }

  .card {
    position: relative;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(${densityProperties.step});
    padding: var(${densityProperties.gutter});
    padding-inline-start: calc(var(${densityProperties.gutter}) + ${spacingMultiple(2)});
    background: ${themeVariable('surface')};
    border: 1px solid ${themeVariable('borderSubtle')};
    border-radius: ${radiusVariable('card')};
    color: ${themeVariable('textPrimary')};
    overflow-wrap: anywhere;
  }

  /* The author band. A pseudo-element rather than a child so the colour cannot be reordered into
   * the middle of the card by a slot, and so nothing in the accessibility tree carries it: the
   * author name beside it is the accessible carrier of the same fact. */
  .card::before {
    content: '';
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    inline-size: ${spacingMultiple(2)};
    border-start-start-radius: ${radiusVariable('card')};
    border-end-start-radius: ${radiusVariable('card')};
    background: var(--mjx-annotation-author-colour, ${themeVariable('border')});
  }

  .card[data-selected='true'] {
    background: ${themeVariable('surfaceRaised')};
    border-color: ${themeVariable('accentBorder')};
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
    min-inline-size: 0;
  }

  /* The dot carries no text, deliberately. Only six colours in this palette reach body-text
   * contrast on the light surface and two of them are the same green, so eight author colours that
   * could legibly carry initials do not exist to be chosen. The initials are drawn beside it in the
   * ordinary text colour instead, which is also what the document actually stores. */
  .dot {
    flex: 0 0 auto;
    inline-size: 1em;
    block-size: 1em;
    border-radius: 50%;
    border: 1px solid ${themeVariable('border')};
    background: var(--mjx-annotation-author-colour, ${themeVariable('border')});
  }

  .author {
    flex: 1 1 auto;
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: ${themeVariable('textPrimary')};
  }

  .initials,
  .time {
    flex: 0 0 auto;
    color: ${themeVariable('textSecondary')};
  }

  .badge {
    flex: 0 0 auto;
    padding-inline: ${spacingMultiple(2)};
    padding-block: 0;
    border-radius: ${radiusVariable('chip')};
    border: 1px solid ${themeVariable('borderSubtle')};
    background: ${themeVariable('background')};
    color: ${themeVariable('textSecondary')};
  }

  .body {
    margin: 0;
    color: ${themeVariable('textPrimary')};
  }

  /* Resolved is a quieter card and NOT a dimmed one. An opacity on a resolved comment is a real
   * contrast failure, because a resolved comment is still expected to be read; only a disabled
   * control is exempt from that rule. So the difference is a fill, a badge and a secondary body
   * colour, all of which are measured. */
  .card[data-resolved='true'] {
    background: ${themeVariable('background')};
  }

  .card[data-resolved='true'] .body {
    color: ${themeVariable('textSecondary')};
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(${densityProperties.step});
  }

  .action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(${densityProperties.step});
    min-inline-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    padding-inline: var(${densityProperties.step});
    border-radius: ${radiusVariable('control')};
    font: inherit;
    color: inherit;
    cursor: pointer;
  }

${controlStatesCss('.action')}
`;

/** `<mjx-comment-card>`. The shell, plus the two models' own presentations. */
export const commentCardCss = `
${cardShellCss}

  /* The two comment models must be told apart at a glance, because they behave differently: a
   * legacy comment has nowhere to put a reply and no resolved flag in its model at all. So the
   * legacy card is a note -- a squarer corner, a dashed leading edge on the band and no action row
   * beyond deleting it -- and the threaded card is a conversation. */
  .card[data-model='legacy'] {
    border-radius: ${radiusVariable('control')};
    border-style: dashed;
  }

  .card[data-model='legacy']::before {
    border-start-start-radius: ${radiusVariable('control')};
    border-end-start-radius: ${radiusVariable('control')};
  }

${liveRegionCss}
${hiddenLast}
`;

/** `<mjx-comment-thread>`. A card, its replies, the count and the composer. */
export const commentThreadCss = `
${cardShellCss}

  .replies {
    display: flex;
    flex-direction: column;
    gap: var(${densityProperties.step});
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .reply {
    display: flex;
    flex-direction: column;
    gap: 0;
    padding-inline-start: var(${densityProperties.gutter});
    border-inline-start: 1px solid ${themeVariable('borderSubtle')};
  }

  .more {
    align-self: flex-start;
    background: none;
    border: 0;
    padding: 0;
    color: ${themeVariable('textSecondary')};
    text-decoration: underline;
    cursor: pointer;
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
  }

  .composer {
    display: flex;
    flex-direction: column;
    gap: var(${densityProperties.step});
  }

  .composer textarea {
    box-sizing: border-box;
    inline-size: 100%;
    min-block-size: calc(var(${densityProperties.hitTarget}) * 2);
    resize: vertical;
    font: inherit;
    color: ${themeVariable('textPrimary')};
    background: ${themeVariable('background')};
    border: 1px solid ${themeVariable('border')};
    border-radius: ${radiusVariable('control')};
    padding: var(${densityProperties.step});
  }

  /* The mention affordance. A name a person can type is a name the document already carries, so
   * this is a datalist over the roster rather than a directory lookup: there is no presence layer
   * in this platform and there is not going to be one. */
  .mentions {
    display: flex;
    flex-wrap: wrap;
    gap: var(${densityProperties.step});
  }

  .mention {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    min-block-size: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    padding-inline: var(${densityProperties.step});
    border: 1px solid ${themeVariable('borderSubtle')};
    border-radius: ${radiusVariable('chip')};
    background: ${themeVariable('background')};
    color: ${themeVariable('textPrimary')};
    font: inherit;
    cursor: pointer;
  }

${liveRegionCss}
${hiddenLast}
`;

/** `<mjx-tracked-change-card>`. The shell, the four kinds and the two verdicts. */
export const trackedChangeCardCss = `
${cardShellCss}

  .description {
    margin: 0;
    color: ${themeVariable('textPrimary')};
  }

  /* The four kinds differ in the icon and in the word, never in colour alone: a card told apart
   * only by hue is a card a colour-blind reader cannot tell apart, which is the same defect the
   * author palette exists to avoid, one level up. */
  .kind {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    color: ${themeVariable('textSecondary')};
  }

  .excerpt {
    margin: 0;
    padding: var(${densityProperties.step});
    background: ${themeVariable('background')};
    border-radius: ${radiusVariable('chip')};
    color: ${themeVariable('textPrimary')};
  }

  .card[data-kind='deletion'] .excerpt {
    text-decoration: line-through;
  }

  .card[data-kind='insertion'] .excerpt {
    text-decoration: underline;
  }

${liveRegionCss}
${hiddenLast}
`;

/**
 * `<mjx-review-pane>`. The margin column, its packed cards, and the sheet it becomes on a phone.
 *
 * The presentation is written into a custom property so both the component and the gate read a
 * decision the *cascade* made, exactly as the ribbon's group presentation is. And as U11's finding
 * requires, the gate cross-checks it against three facts the property does not control: the
 * column's own position, a card's position, and whether the sheet handle is displayed.
 */
export const reviewPaneCss = `
  :host {
    display: block;
    box-sizing: border-box;
    block-size: 100%;
    min-block-size: 0;
    ${reviewPresentationProperty}: margin;
  }

  .pane {
    box-sizing: border-box;
    block-size: 100%;
    min-block-size: 0;
    display: flex;
    flex-direction: column;
    background: ${themeVariable('background')};
    color: ${themeVariable('textPrimary')};
  }

  .title {
    flex: 0 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(${densityProperties.step});
    padding: var(${densityProperties.gutter});
    border-block-end: 1px solid ${themeVariable('borderSubtle')};
  }

  .handle {
    display: none;
  }

  /* The column. A feed, and the scroll container: role and overflow on one element so the thing a
   * reader pages through is the thing that scrolls. */
  .column {
    position: relative;
    flex: 1 1 auto;
    min-block-size: 0;
    overflow-y: auto;
    overflow-x: hidden;
    padding-inline: var(${densityProperties.gutter});
  }

  /* The sizer is as long as EVERY card rather than as long as the built ones, which is what stops
   * the scrollbar lying about how much review there is. */
  .sizer {
    position: relative;
    inline-size: 100%;
  }

  /* A card that jumps to a new packed offset the instant a selection changes reads as the column
   * redrawing itself. The motion role is documentObject: attached to something in the user's
   * document, so it decelerates and never overshoots -- a card that sprang past its anchor and came
   * back would be telling a reader their comment moved. The role class supplies the duration and
   * the easing; the property is this component's to name. */
  .slot {
    position: absolute;
    inset-inline: 0;
    transition-property: inset-block-start;
  }

  .empty {
    padding: var(${densityProperties.gutter});
    color: ${themeVariable('textSecondary')};
  }

  /* THE SHEET. A margin column has no room on a phone, so below the shell threshold the pane stops
   * being a column beside the document and becomes a sheet listing the annotations: full width,
   * anchored to the block end, with a handle, and with the cards in a plain flow. Packing is
   * meaningless here and is not merely disabled -- there is no margin for a card to sit beside. */
  @container ${containerName} (max-width: ${String(reviewSheetAtOrBelow)}px) {
    :host {
      ${reviewPresentationProperty}: sheet;
    }

    .pane {
      background: ${themeVariable('surfaceRaised')};
      border-start-start-radius: ${radiusVariable('phone')};
      border-start-end-radius: ${radiusVariable('phone')};
      border: 1px solid ${themeVariable('border')};
      box-shadow: var(--shadow-lift);
    }

    .handle {
      display: block;
      flex: 0 0 auto;
      inline-size: calc(var(${densityProperties.hitTarget}) * 2);
      block-size: ${spacingMultiple(1)};
      margin: var(${densityProperties.step}) auto;
      border-radius: ${radiusVariable('chip')};
      background: ${themeVariable('border')};
    }

    .sizer {
      display: flex;
      flex-direction: column;
      gap: var(${densityProperties.step});
      padding-block: var(${densityProperties.step});
      block-size: auto !important;
    }

    .slot {
      position: static;
      inset-block-start: auto !important;
    }
  }

${liveRegionCss}
${hiddenLast}
`;

/** The type roles each part of a card uses, so a gate reads the choice from the table. */
export const annotationTypeRoles = {
  author: typeRoleClass('label'),
  meta: typeRoleClass('dense'),
  body: typeRoleClass('body'),
  paneTitle: typeRoleClass('paneTitle'),
  action: typeRoleClass('control'),
} as const;

/**
 * The motion role a card uses when it moves to a new packed position.
 *
 * ⚠ **Read by the component, not only exported.** `<mjx-review-pane>` puts this class on every card
 * it places, and `.slot` above names the property it transitions. A motion role that is exported and
 * never applied is the shape of thing CLAUDE.md calls worse than none: it looks like a decision and
 * changes nothing.
 */
export const annotationMotionClass = motionRoleClass('documentObject');
