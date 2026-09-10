/**
 * MJXOFF-190's model: the chrome that frames the document surface, as data and pure functions.
 *
 * Four components read this file — `<mjx-status-bar>`, `<mjx-zoom-control>`, `<mjx-scrollbar>` and
 * `<mjx-splitter>` — and so do both gate tiers. Nothing here has a DOM in it, for the reason
 * `src/harness/presets.ts` states: the browser tier's specs run in Node, and a module that defines
 * a custom element cannot be imported from one.
 *
 * ## Why furniture is where *"it looks fine"* hides best
 *
 * Every component here is chrome nobody stares at, and every one of them is arithmetic with
 * correct answers:
 *
 * * **a scrollbar is the identity-value trap in its purest form** — a scrollbar over content that
 *   fits exercises nothing at all, and a thumb over *known* content behaves perfectly under any
 *   implementation. The whole of it is in `scroll-model.ts` and the assertions are ratios;
 * * **a zoom control has right answers** — the steps, the fit computations, the bounds, and the
 *   agreement between a typed percentage and the slider;
 * * **a status bar is a live region, and the failure is announcing too much** — a page number that
 *   changes as a person scrolls must not interrupt them;
 * * **a splitter that only drags is broken**, which is why the arithmetic it shares with
 *   `<mjx-task-pane>` was lifted into `src/foundations/splitter.ts` rather than written twice.
 *
 * So every figure in this file is asserted as a number somewhere, and nothing here is a screenshot.
 */

import { paintValue, themeVariable, type Paint } from '../controls/control-states.ts';
import {
  accessibleHitTargetMinimum,
  densityProperties,
  spacingMultiple,
} from '../foundations/density.ts';
import { motionRoleClass } from '../foundations/motion.ts';
import { radiusVariable, surfaceLevels } from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { splitterDefaultBounds, splitterStep } from '../foundations/splitter.ts';
import { inputBaseCss, fieldStatesCss } from '../inputs/input-model.ts';
import { readTypedNumber, type MeasureParseError } from '../inputs/measure.ts';
import type { measureFailureMessages } from '../inputs/measure.ts';
import { contrastRatioOrWorst, formatRatio, nonTextMinimum } from '../tokens/contrast.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import type { ThemeMember } from '../tokens/resolver.ts';
import { scrollMarkKindNames, type ScrollMarkKind } from './scroll-model.ts';

// ── the elements ─────────────────────────────────────────────────────────────

/** The custom element names, so a gate never spells one itself. */
export const furnitureTags = {
  statusBar: 'mjx-status-bar',
  statusSegment: 'mjx-status-segment',
  zoomControl: 'mjx-zoom-control',
  scrollbar: 'mjx-scrollbar',
  scrollMark: 'mjx-scroll-mark',
  splitter: 'mjx-splitter',
} as const;

/** The story titles, named once so a spec cannot mistype one. */
export const furnitureStoryTitles = {
  statusBar: 'Furniture/Status Bar',
  zoomControl: 'Furniture/Zoom Control',
  scrollbar: 'Furniture/Scrollbar',
  splitter: 'Furniture/Splitter',
} as const;

/** The events this child's four components emit. One vocabulary. */
export const furnitureEvents = {
  /** The zoom changed. `detail: { percent, previous, cause }`. */
  zoom: 'mjx-zoom-change',
  /** A typed zoom could not be read. `detail: { text, failure, offending }`. */
  zoomInvalid: 'mjx-zoom-invalid',
  /** The document was scrolled from the scrollbar. `detail: { offset, anchor, cause }`. */
  scroll: 'mjx-scroll',
  /** A splitter moved. `detail: { fraction, collapsed, cause }`. */
  split: 'mjx-split',
  /** The status bar's segments were re-arranged. `detail: { shown, overflow }`. */
  statusOverflow: 'mjx-status-overflow',
  /** A status segment's value changed. `detail: { segment, value, announce }`. */
  statusChange: 'mjx-status-change',
} as const;

/** How a change was caused, so a host can tell a drag from a key from a programmatic set. */
export const furnitureCauses = ['pointer', 'keyboard', 'typed', 'command', 'host'] as const;

/** One of the five. */
export type FurnitureCause = (typeof furnitureCauses)[number];

// ── the status bar ───────────────────────────────────────────────────────────

/** The three regions of a status bar, in reading order. */
export const statusRegionNames = ['start', 'centre', 'end'] as const;

/** One of the three. */
export type StatusRegion = (typeof statusRegionNames)[number];

/** Whether a value names one of the three. */
export function isStatusRegion(value: unknown): value is StatusRegion {
  return (statusRegionNames as readonly string[]).includes(String(value));
}

/** The region a segment goes in when it says nothing. */
export const defaultStatusRegion: StatusRegion = 'start';

/**
 * The name every status-bar-scoped `@container` query addresses.
 *
 * `<mjx-status-bar>` establishes it **on its own host element**, exactly as `<mjx-ribbon>` does,
 * and deliberately not `mjx-frame`: a component that queried the harness would be untestable
 * outside the harness and would take the harness's width when the real shell's bar was narrower.
 */
export const statusContainerName = 'mjx-status-bar';

/**
 * The custom property the status bar's container rules write, and the component and the gate read.
 *
 * ⚠ **This is how a CSS decision is legible to JavaScript without measuring anything**, and it is
 * MJXOFF-183's mechanism reused rather than a second one. The alternative — deciding a width in a
 * resize handler — would make the gate a measurement of a measurement.
 */
export const statusPresentationProperty = '--mjx-status-presentation';

/** The four priorities a segment may declare, first to give way last. */
export const statusPriorityNames = ['ancillary', 'supplementary', 'standard', 'essential'] as const;

/** One of the four. */
export type StatusPriority = (typeof statusPriorityNames)[number];

/** Whether a value names one of the four. */
export function isStatusPriority(value: unknown): value is StatusPriority {
  return (statusPriorityNames as readonly string[]).includes(String(value));
}

/** The priority a segment is when it does not say. */
export const defaultStatusPriority: StatusPriority = 'standard';

/** What a segment is promising when it declares a priority. */
export interface StatusPrioritySpec {
  /**
   * At or below this container width the segment leaves the bar for the overflow.
   *
   * `undefined` means *never*: a segment at the top of the ladder is on screen at every width, and
   * a phone has room for exactly those.
   */
  readonly dropAtOrBelow?: number;
  readonly use: string;
}

/**
 * The ladder, in container pixels.
 *
 * ⚠ **These are the one class of length the token system has no name for**, and MJXOFF-183 already
 * explains why in `ribbon-model.ts`: a breakpoint is a relationship between a layout and a
 * container, not a design value, and `--spacing` cannot appear in a `@container` condition because
 * a query is evaluated before custom properties on the queried element are known. They are data
 * *here*, once, and every rule that uses them is generated.
 *
 * The widths are chosen so the harness's three presets land informatively: at `desktop` (1440)
 * everything is on the bar; at `tablet` (834) the bottom two priorities have gone; at `phone` (390)
 * only `essential` is left, which is *"a phone has room for two of them"* stated as a number.
 *
 * `GUESS:` Office's status bar drops items as its window narrows and the *ordering* here is that
 * behaviour; no width is checked against Office and none of this is parity.
 */
export const statusPriorities: Readonly<Record<StatusPriority, StatusPrioritySpec>> = {
  ancillary: {
    dropAtOrBelow: 1100,
    use: 'A reading a person consults rather than watches — the language, the section number. First to go, and it goes while there is still plenty of room.',
  },
  supplementary: {
    dropAtOrBelow: 834,
    use: 'Something the document is doing that is not urgent: the word count, the selection summary.',
  },
  standard: {
    dropAtOrBelow: 560,
    use: 'The default. On the bar at a tablet width and gone on a phone.',
  },
  essential: {
    use: 'Where you are and how big it is: the page position and the zoom. Never dropped, at any width, because a document with neither is a document a person is lost in.',
  },
};

/** What a presentation the bar can put a segment in. */
export const statusPresentationNames = ['shown', 'overflow'] as const;

/** One of the two. */
export type StatusPresentation = (typeof statusPresentationNames)[number];

/**
 * **The model both gates read**: where a segment of this priority is, at this container width.
 *
 * Written as the same cascade the stylesheet emits — `shown`, then `overflow` if the condition
 * holds — rather than as a chain of comparisons in the opposite order, because a model that agreed
 * with itself and disagreed with the browser is the failure MJXOFF-182 spent a child discovering.
 */
export function statusPresentationAt(
  priority: StatusPriority,
  containerWidth: number,
): StatusPresentation {
  const spec = statusPriorities[priority];
  let presentation: StatusPresentation = 'shown';
  if (spec.dropAtOrBelow !== undefined && containerWidth <= spec.dropAtOrBelow) {
    presentation = 'overflow';
  }
  return presentation;
}

/**
 * How loudly a segment's *change* is announced. Three, and the default is silence.
 *
 * ⚠ **A status bar is a live region, and the failure is announcing too much.** A page number that
 * changes as a person scrolls is the canonical example: it is the reason the bar exists, it changes
 * constantly, and a screen reader reading it out on every change makes the document unusable. So
 * `off` is the default and a segment must *opt in* to being announced at all.
 */
export const statusAnnouncementNames = ['off', 'polite', 'assertive'] as const;

/** One of the three. */
export type StatusAnnouncement = (typeof statusAnnouncementNames)[number];

/** Whether a value names one of the three. */
export function isStatusAnnouncement(value: unknown): value is StatusAnnouncement {
  return (statusAnnouncementNames as readonly string[]).includes(String(value));
}

/** What a segment announces when it does not say. Nothing. */
export const defaultStatusAnnouncement: StatusAnnouncement = 'off';

/**
 * The role each politeness gets.
 *
 * ⚠ **Two regions, never one whose `aria-live` is swapped**, which is MJXOFF-189's rule and its
 * reasoning: a live region's politeness is settled when it enters the accessibility tree, and
 * rewriting the attribute on a live region is a change assistive technology may or may not have
 * noticed. `off` names no region at all — the text is on screen and readable, and nothing is
 * pushed at anybody.
 */
export const statusAnnouncementRoles: Readonly<
  Record<StatusAnnouncement, 'status' | 'alert' | undefined>
> = {
  off: undefined,
  polite: 'status',
  assertive: 'alert',
};

/** The label the overflow disclosure carries, which says how many segments are behind it. */
export function statusOverflowLabel(count: number): string {
  if (count === 0) return 'No hidden status items';
  return count === 1 ? '1 more status item' : `${String(count)} more status items`;
}

// ── the zoom control ─────────────────────────────────────────────────────────

/**
 * The range a document may be zoomed to, as whole percentages.
 *
 * `GUESS:` Word's own slider runs 10 % to 500 % and this matches it; nothing here is checked
 * against Office.
 */
export const zoomBounds = { min: 10, max: 500 } as const;

/** Where a document opens. */
export const defaultZoomPercent = 100;

/**
 * The stops the minus and plus commands move between, and the ticks the slider draws.
 *
 * A zoom control's buttons do **not** step by one per cent: nobody wants to press a button
 * ninety times to halve a page. They move to the next stop, which is what makes the control usable
 * with a keyboard alone.
 */
export const zoomSteps: readonly number[] = [10, 25, 50, 75, 100, 125, 150, 200, 300, 400, 500];

/** Bring a percentage inside the supported range, as a whole number. */
export function clampZoom(percent: number): number {
  if (!Number.isFinite(percent)) return defaultZoomPercent;
  return Math.min(zoomBounds.max, Math.max(zoomBounds.min, Math.round(percent)));
}

/**
 * The next stop beyond `current`, in `direction`.
 *
 * Strictly beyond, so a value already sitting on a stop moves off it, and a value *between* two
 * stops moves to the nearer side. At the ends it answers the bound rather than `undefined`: a
 * button that stopped responding would read as a broken control rather than as a limit, and the
 * component disables it instead — which is a thing a person can see.
 */
export function zoomStepFrom(current: number, direction: 1 | -1): number {
  const from = clampZoom(current);
  if (direction === 1) {
    const next = zoomSteps.find((stop) => stop > from);
    return clampZoom(next ?? zoomBounds.max);
  }
  const below = zoomSteps.filter((stop) => stop < from);
  return clampZoom(below[below.length - 1] ?? zoomBounds.min);
}

/** A rectangle, in CSS pixels. Both a viewport and a page are one. */
export interface Rectangle {
  readonly width: number;
  readonly height: number;
}

/**
 * US Letter at 96 dpi, which is what a page is in the catalogue's specimens.
 *
 * 8.5 × 11 inches written as arithmetic rather than as `816 × 1056`, so a reader can check it
 * without a calculator — the same rule `measure.ts` follows for its unit factors.
 */
export const letterPagePixels: Rectangle = { width: 8.5 * 96, height: 11 * 96 };

/** How much room is left around a page inside the viewport, in CSS pixels, on every side. */
export const zoomFitGutter = 24;

/**
 * The zoom at which the page is as wide as the viewport allows.
 *
 * ⚠ **Floored, not rounded.** A rounded-up fit overflows the viewport by a fraction of a pixel and
 * produces a horizontal scrollbar on a command whose entire promise is that there will not be one.
 * `tests/furniture.test.ts` asserts both the arithmetic and the direction of the rounding.
 */
export function fitToWidthPercent(
  viewport: Rectangle,
  page: Rectangle = letterPagePixels,
  gutter: number = zoomFitGutter,
): number {
  if (!(page.width > 0)) return defaultZoomPercent;
  const available = viewport.width - gutter * 2;
  return clampZoom(Math.floor((available / page.width) * 100));
}

/** The zoom at which the whole page fits: the smaller of the two fits, so neither axis overflows. */
export function fitToPagePercent(
  viewport: Rectangle,
  page: Rectangle = letterPagePixels,
  gutter: number = zoomFitGutter,
): number {
  if (!(page.width > 0) || !(page.height > 0)) return defaultZoomPercent;
  const byWidth = Math.floor(((viewport.width - gutter * 2) / page.width) * 100);
  const byHeight = Math.floor(((viewport.height - gutter * 2) / page.height) * 100);
  return clampZoom(Math.min(byWidth, byHeight));
}

/** The two commands a zoom control offers beside its slider. */
export const zoomFitNames = ['width', 'page'] as const;

/** One of the two. */
export type ZoomFit = (typeof zoomFitNames)[number];

/** What each fit is called and what it computes. */
export const zoomFits: Readonly<
  Record<ZoomFit, { readonly label: string; readonly compute: (viewport: Rectangle, page?: Rectangle) => number }>
> = {
  width: { label: 'Fit width', compute: (viewport, page) => fitToWidthPercent(viewport, page) },
  page: { label: 'Fit page', compute: (viewport, page) => fitToPagePercent(viewport, page) },
};

/** How a zoom is written into its readout. Whole per cent, and never a trailing zero. */
export function formatZoom(percent: number): string {
  return `${String(clampZoom(percent))}%`;
}

/** What a typed zoom that could not be read carries with it. */
export type ZoomParseError = MeasureParseError;

/** What `parseZoomPercent` returns. */
export type ZoomParse =
  | { readonly ok: true; readonly percent: number; readonly clamped: boolean }
  | { readonly ok: false; readonly error: ZoomParseError };

/**
 * Read a percentage out of what a person typed.
 *
 * **It never guesses, and it never silently reverts** — U07's rule for `<mjx-measure-input>`,
 * which the zoom readout inherits wholesale: a string this cannot read comes back as an error
 * carrying the fragment that defeated it, the component keeps the text, marks the field
 * `aria-invalid` and commits nothing.
 *
 * Out of range is **not** a failure. The ticket asks for a readout that *"clamps to the supported
 * range"*, and `700` is a number a person meant: it becomes 500 and `clamped` says so, which is
 * what lets the component announce that it did something other than what was asked.
 *
 * The number itself is read by `readTypedNumber`, shared with the measure input, so `1.2.3`, a
 * bare `-` and a thousands separator are refused here exactly as they are there.
 */
export function parseZoomPercent(text: string): ZoomParse {
  const read = readTypedNumber(text);
  if (!read.ok) return { ok: false, error: read.error };
  const { magnitude, tail } = read.value;
  // A trailing `%` is the only suffix a percentage may carry, and it is optional: a person typing
  // into a field that already shows `%` should not have to type it again.
  if (tail !== '' && tail !== '%') {
    return { ok: false, error: { failure: 'unknownUnit', offending: tail } };
  }
  const percent = clampZoom(magnitude);
  return { ok: true, percent, clamped: percent !== Math.round(magnitude) };
}

/** What a person is told, per failure. `{offending}` is substituted by the component. */
export const zoomFailureMessages: Readonly<Record<keyof typeof measureFailureMessages, string>> = {
  empty: 'Type a zoom, such as 100%.',
  notANumber: '“{offending}” is not a number. Type a whole percentage, such as 125.',
  unknownUnit: '“{offending}” is not part of a percentage. Type a number, optionally followed by %.',
};

// ── the scrollbar's paints ───────────────────────────────────────────────────

/**
 * What the scrollbar is painted in, and every one of them is measured.
 *
 * A scrollbar is entirely non-text: WCAG 2.2 §1.4.11 asks for 3 : 1, and
 * `tests/furniture.test.ts` sweeps every pair below in **both** schemes and asserts the worst.
 * The figures are not written here — the sweep computes them, on the rule the palette re-seed
 * taught this catalogue: *assert measured properties, not remembered numbers.*
 */
export const scrollbarPaints = {
  /** The channel the thumb runs in. */
  track: 'background' as ThemeMember,
  /** The thumb. */
  thumb: 'textSecondary' as ThemeMember,
} as const;

/**
 * The three mark kinds, and how each is told from the others.
 *
 * ⚠ **Never by colour alone**, which is MJXOFF-189's rule and the reason a `lane` is here: this
 * palette has two colour families, and three kinds distinguished only by hue would be two kinds a
 * person can tell apart and a third they cannot. Each kind therefore occupies its own lane across
 * the width of the channel as well as its own colour, so the channel reads as three columns even
 * in greyscale — and, incidentally, two marks at the same offset no longer draw on top of each
 * other.
 */
export interface ScrollMarkSpec {
  readonly paint: Paint;
  /** Which third of the channel's width it occupies, counted from the leading edge. */
  readonly lane: 0 | 1 | 2;
  readonly use: string;
}

/** The three. */
export const scrollMarkSpecs: Readonly<Record<ScrollMarkKind, ScrollMarkSpec>> = {
  search: {
    paint: 'accentPressed',
    lane: 0,
    use: 'A hit for what is in the find box. The channel is the only thing that makes “237 results” navigable.',
  },
  comment: {
    paint: 'secondaryAccent',
    lane: 1,
    use: 'A comment thread. The second colour family, so it is not a weaker search hit.',
  },
  change: {
    paint: 'textPrimary',
    lane: 2,
    use: 'A tracked change. Deliberately the ink colour rather than a third hue: a two-family palette does not have a third, and inventing one here would be a literal.',
  },
};

/** How many lanes the channel has. Read from the specs rather than typed, so a fourth kind fits. */
export const scrollMarkLanes = 3;

/** A colour a scrollbar part resolves to, in one scheme. */
export function furnitureColor(scheme: ColorScheme, member: ThemeMember): string {
  return tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
}

/** Every pair a scrollbar puts on screen, so a sweep names them rather than remembering them. */
export function scrollbarContrastPairs(): readonly {
  readonly what: string;
  readonly foreground: ThemeMember;
  readonly background: ThemeMember;
}[] {
  const track = scrollbarPaints.track;
  const pairs: { what: string; foreground: ThemeMember; background: ThemeMember }[] = [
    { what: 'the thumb on its track', foreground: scrollbarPaints.thumb, background: track },
  ];
  for (const kind of scrollMarkKindNames) {
    const paint = scrollMarkSpecs[kind].paint;
    if (paint === 'transparent') continue;
    pairs.push({ what: `the ${kind} mark on its track`, foreground: paint, background: track });
  }
  return pairs;
}

/** The worst ratio any scrollbar pair reaches in a scheme, and which pair it was. */
export function worstScrollbarContrast(scheme: ColorScheme): {
  readonly ratio: number;
  readonly what: string;
} {
  let worst = { ratio: Number.POSITIVE_INFINITY, what: '' };
  for (const pair of scrollbarContrastPairs()) {
    const ratio = contrastRatioOrWorst(
      furnitureColor(scheme, pair.foreground),
      furnitureColor(scheme, pair.background),
    );
    if (ratio < worst.ratio) worst = { ratio, what: pair.what };
  }
  return worst;
}

/** The caption a specimen prints, so the catalogue and the gate can never drift apart. */
export function scrollbarContrastCaption(scheme: ColorScheme): string {
  const worst = worstScrollbarContrast(scheme);
  return `${formatRatio(worst.ratio)} : 1 — ${worst.what}, in ${scheme}`;
}

/** Re-exported so a gate can name the floor it is comparing against. */
export { nonTextMinimum };

// ── the splitter's own defaults ──────────────────────────────────────────────

/** What `<mjx-splitter>` allows when it says nothing. Re-stated from the shared primitive. */
export { splitterDefaultBounds, splitterStep };

/**
 * Where a collapsed splitter puts the region it collapsed.
 *
 * Zero, and that is the whole reason `collapsed` is a separate state rather than a fraction: the
 * bounds forbid zero, deliberately — a region a drag could shrink to nothing is a region a person
 * loses by accident. A collapse is a *command*, it is reversible by the same command, and the
 * fraction it collapsed from is remembered so that reversing it is a restore rather than a reset to
 * a default nobody chose.
 */
export const collapsedFraction = 0;

/** The keys that collapse and restore, beside the double-click that does the same thing. */
export const splitterCollapseKeys: readonly string[] = ['Enter', ' '];

/** How a splitter announces what it is at. */
export function splitterValueText(fraction: number, collapsed: boolean): string {
  if (collapsed) return 'Collapsed';
  return `${String(Math.round(fraction * 100))} per cent`;
}

/** Where a splitter's position is remembered, per key. */
export function splitterStorageKey(key: string): string {
  return `mjx-splitter:${key}`;
}

// ── the boxes ────────────────────────────────────────────────────────────────

/** The custom properties this child's boxes read. Layout only; no paint goes through one. */
export const furnitureBoxProperties = {
  /** The thumb's size, as a fraction of the track. */
  thumbSize: '--mjx-scrollbar-thumb-size',
  /** Where the thumb's leading edge is, as a fraction of the track. */
  thumbStart: '--mjx-scrollbar-thumb-start',
  /** How thick the scrollbar is. */
  scrollbarThickness: '--mjx-scrollbar-thickness',
  /** A mark's position along the channel, as a fraction. */
  markAt: '--mjx-scroll-mark-at',
  /** Which lane a mark is in, as an index. */
  markLane: '--mjx-scroll-mark-lane',
  /** The leading region's share of the splitter's boundary. */
  splitFraction: '--mjx-split-fraction',
  /**
   * `1` while the leading region is collapsed, `0` otherwise.
   *
   * ⚠ **Not redundant with a fraction of zero, and the browser gate is what proved it.** A padded
   * region set to `inline-size: 0` under `box-sizing: border-box` **cannot** shrink below its own
   * padding — CSS floors the used width there — so a collapsed pane left a two-dozen-pixel stripe
   * of surface on screen with every announced number already correct. A region needs a signal it
   * can zero its *own* padding from, and this is it.
   */
  splitCollapsed: '--mjx-split-collapsed',
} as const;

/** How thick a scrollbar is, in spacing units, before the hit-target floor is applied. */
export const scrollbarThicknessUnits = 3;

/** How tall a mark is, in spacing units. */
export const scrollMarkBlockUnits = 1;

// ── the stylesheets ──────────────────────────────────────────────────────────

const raised = surfaceLevels.raised;
const overlay = surfaceLevels.overlay;

/**
 * `<mjx-status-bar>`'s rules, and the generated ladder.
 *
 * ⚠ **The container blocks are emitted LAST**, and MJXOFF-188 is why: a `@container` block changes
 * no specificity at all, so a presentation rule written above the base rule it overrides simply
 * loses on source order at equal specificity. `tests/furniture.test.ts` asserts the order *and*
 * the equal specificity that makes the order matter, because the order assertion alone was true
 * and useless for MJXOFF-183.
 */
export const statusBarCss = [
  `
  :host {
    display: block;
    box-sizing: border-box;
    container-type: inline-size;
    container-name: ${statusContainerName};
    background: ${raised.background};
    border-block-start: ${raised.border};
    color: ${themeVariable('textPrimary')};
  }

  :host([hidden]) { display: none; }

  .bar {
    display: flex;
    align-items: center;
    gap: var(${densityProperties.gutter});
    padding-inline: var(${densityProperties.gutter});
    min-block-size: max(var(${densityProperties.step}) * 4, ${String(accessibleHitTargetMinimum)}px);
  }

  .region {
    display: flex;
    align-items: center;
    gap: var(${densityProperties.gutter});
    min-inline-size: 0;
  }

  .region[data-region='start'] { flex: 1 1 auto; justify-content: flex-start; }
  .region[data-region='centre'] { flex: 0 1 auto; justify-content: center; }
  .region[data-region='end'] { flex: 1 1 auto; justify-content: flex-end; }

  ::slotted(${furnitureTags.statusSegment}) {
    min-inline-size: 0;
  }

  /* The disclosure that holds what the bar could not fit. Drawn only when it holds something. */
  .overflow-trigger {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(${densityProperties.step});
    min-inline-size: max(var(${densityProperties.step}) * 3, ${String(accessibleHitTargetMinimum)}px);
    min-block-size: max(var(${densityProperties.step}) * 3, ${String(accessibleHitTargetMinimum)}px);
    padding: 0;
    border-width: 1px;
    border-radius: ${radiusVariable('control')};
  }

  .overflow-panel {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(${densityProperties.gutter});
    padding: var(${densityProperties.gutter});
    background: ${overlay.background};
    border-block-start: ${overlay.border};
  }

  /* The probes. One per priority, matched by the generated blocks below, read back by the
   * component and by the gate. They are how a CSS decision reaches JavaScript without a
   * measurement, and they draw nothing whatever. */
  .probe {
    position: absolute;
    inline-size: 0;
    block-size: 0;
    overflow: hidden;
    pointer-events: none;
  }

  /* ⚠ The presentation declaration is at (0,0,0) and the layout above is not, ON PURPOSE.
   * A container query changes no specificity, so the generated blocks below score exactly what
   * their selector scores -- and a base rule written as a bare class would be (0,1,0) and would
   * beat every one of them at every width, with the probe reporting shown forever and the gate
   * comparing the component against a model it silently never followed. That is MJXOFF-183's
   * specificity accident, and the answer is the one MJXOFF-183 arrived at: put every rule that
   * decides a presentation inside :where(), so the arbitration is source order and nothing else. */
  :where(.probe) {
    ${statusPresentationProperty}: shown;
  }

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

  /* ⚠ Restated LAST, and MJXOFF-189 found out why: the user agent's own [hidden] rule is in the
   * user-agent origin, so any author rule at all beats it. Every display above is an author rule,
   * so without this line an element with the attribute set stays on screen with a correct
   * accessibility tree and a passing attribute assertion. */
  [hidden] { display: none !important; }
`,
  ...statusPriorityNames.flatMap((priority) => {
    const width = statusPriorities[priority].dropAtOrBelow;
    if (width === undefined) return [];
    return [
      `@container ${statusContainerName} (width <= ${String(width)}px) {\n` +
        `  :where(.probe[data-priority='${priority}']) {\n` +
        `    ${statusPresentationProperty}: overflow;\n  }\n}`,
    ];
  }),
].join('\n');

/** `<mjx-status-segment>`'s rules. A segment is a label, a value and nothing else. */
export const statusSegmentCss = `
  :host {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    min-inline-size: 0;
    color: ${themeVariable('textPrimary')};
  }

  :host([hidden]) { display: none; }

  .segment {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
    min-inline-size: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .name {
    color: ${themeVariable('textSecondary')};
  }

  [hidden] { display: none !important; }
`;

/**
 * `<mjx-zoom-control>`'s rules.
 *
 * It composes `<mjx-slider>` and two `<mjx-button>`s rather than drawing any of them, so almost
 * nothing here is paint: the readout is the only box this component owns, and it borrows the field
 * state table wholesale for exactly the reason `<mjx-measure-input>` does.
 */
export const zoomControlCss = [
  inputBaseCss,
  fieldStatesCss('.field'),
  `
  :host {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
  }

  :host([hidden]) { display: none; }

  .slider {
    inline-size: ${spacingMultiple(24)};
    flex: 0 1 auto;
  }

  .readout {
    inline-size: ${spacingMultiple(14)};
    min-inline-size: ${spacingMultiple(14)};
    flex: 0 0 auto;
  }

  .readout .entry {
    text-align: end;
  }

  .commands {
    display: inline-flex;
    align-items: center;
    gap: var(${densityProperties.step});
  }

  .message {
    display: block;
    margin-block-start: var(${densityProperties.step});
    color: ${themeVariable('textPrimary')};
  }

  [hidden] { display: none !important; }
`,
].join('\n');

/**
 * `<mjx-scrollbar>`'s rules.
 *
 * Two numbers reach CSS and nothing else does: the thumb's size and the thumb's start, both as
 * fractions of the track. **A painter that computed either of them from a different quantity than
 * the model did would be a second interpretation**, so the component writes exactly what
 * `scroll-model.ts` returned and the stylesheet does the multiplication.
 */
export const scrollbarCss = `
  :host {
    display: block;
    box-sizing: border-box;
    ${furnitureBoxProperties.scrollbarThickness}: max(
      calc(var(${densityProperties.step}) * ${String(scrollbarThicknessUnits)}),
      ${String(accessibleHitTargetMinimum)}px
    );
    inline-size: var(${furnitureBoxProperties.scrollbarThickness});
    block-size: 100%;
    /* ⚠ Not shrinkable. A flex item's default is flex-shrink: 1, so a scrollbar beside a canvas
     * that wants the room is squeezed below its own thickness -- and the hit-target floor, which
     * is a promise about pixels on screen, quietly stops being true. U07 found the same default in
     * the other axis, where option rows ignored their height inside a flex column. */
    flex: 0 0 auto;
    background: ${themeVariable(scrollbarPaints.track)};
    touch-action: none;
  }

  :host([hidden]) { display: none; }

  :host([orientation='horizontal']) {
    inline-size: 100%;
    block-size: var(${furnitureBoxProperties.scrollbarThickness});
  }

  /* Content that fits needs no scrollbar, and one drawn over it is the identity-value trap made
   * visible: a full-length thumb that cannot move looks exactly like a working scrollbar. */
  :host([data-scrollable='false']) .thumb { visibility: hidden; }

  .track {
    position: relative;
    inline-size: 100%;
    block-size: 100%;
    overflow: hidden;
  }

  .thumb {
    position: absolute;
    inset-inline: 0;
    inset-block-start: calc(100% * var(${furnitureBoxProperties.thumbStart}, 0));
    block-size: calc(100% * var(${furnitureBoxProperties.thumbSize}, 1));
    background: ${themeVariable(scrollbarPaints.thumb)};
    border-radius: ${radiusVariable('chip')};
  }

  :host([orientation='horizontal']) .thumb {
    inset-block: 0;
    inset-inline-start: calc(100% * var(${furnitureBoxProperties.thumbStart}, 0));
    inline-size: calc(100% * var(${furnitureBoxProperties.thumbSize}, 1));
    block-size: auto;
  }

  .marks {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .mark {
    position: absolute;
    inset-block-start: calc(100% * var(${furnitureBoxProperties.markAt}, 0));
    inline-size: calc(100% / ${String(scrollMarkLanes)});
    inset-inline-start: calc(
      100% * var(${furnitureBoxProperties.markLane}, 0) / ${String(scrollMarkLanes)}
    );
    block-size: calc(var(${densityProperties.step}) * ${String(scrollMarkBlockUnits)});
    border-radius: ${radiusVariable('chip')};
  }

  :host([orientation='horizontal']) .mark {
    inset-block-start: calc(
      100% * var(${furnitureBoxProperties.markLane}, 0) / ${String(scrollMarkLanes)}
    );
    block-size: calc(100% / ${String(scrollMarkLanes)});
    inset-inline-start: calc(100% * var(${furnitureBoxProperties.markAt}, 0));
    inline-size: calc(var(${densityProperties.step}) * ${String(scrollMarkBlockUnits)});
  }

${scrollMarkKindNames
  .map(
    (kind) =>
      `  .mark[data-kind='${kind}'] { background: ${paintValue(scrollMarkSpecs[kind].paint)}; }`,
  )
  .join('\n')}

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

  [hidden] { display: none !important; }
`;

/**
 * `<mjx-splitter>`'s rules.
 *
 * The handle is the whole hit target and the *line* is drawn inside it — a one-pixel divider a
 * person has to hit exactly is a divider only a mouse in good hands can move, and the floor is the
 * same 24 CSS pixels every other control in this catalogue holds in `compact`.
 */
export const splitterCss = `
  :host {
    display: flex;
    align-items: center;
    justify-content: center;
    flex: 0 0 auto;
    align-self: stretch;
    inline-size: max(var(${densityProperties.step}), ${String(accessibleHitTargetMinimum)}px);
    cursor: col-resize;
    touch-action: none;
    background: transparent;
  }

  :host([hidden]) { display: none; }

  :host([orientation='horizontal']) {
    inline-size: auto;
    block-size: max(var(${densityProperties.step}), ${String(accessibleHitTargetMinimum)}px);
    cursor: row-resize;
  }

  .line {
    inline-size: 1px;
    block-size: 100%;
    background: ${themeVariable('border')};
    /* A named property rather than the default of all: a transition on all would ease the line's
     * geometry as well, and a divider that eases its own position lags the finger dragging it. */
    transition-property: background;
  }

  :host([orientation='horizontal']) .line {
    inline-size: 100%;
    block-size: 1px;
  }

  :host([data-collapsed]) .line {
    background: ${themeVariable('accent')};
  }

  [hidden] { display: none !important; }
`;

/** The class the moving parts wear, so a splitter and a thumb settle rather than snap. */
export const furnitureMotionClass = motionRoleClass('surfaceSettle');

/** The type roles this child's parts use. */
export const furnitureTypeRoles = {
  segment: typeRoleClass('dense'),
  segmentName: typeRoleClass('dense'),
  readout: typeRoleClass('control'),
} as const;

/**
 * The one rule that must be on the **document**.
 *
 * `<mjx-status-segment>` and `<mjx-scroll-mark>` carry data written as markup. Between the moment
 * the parser creates them and the moment they upgrade they are unknown elements with attributes,
 * and an unknown element is `display: inline` — so without this a page flashes a row of segment
 * labels and a list of mark descriptions into the layout before any component exists.
 * `inputDocumentCss` and `galleryDocumentCss` make the same arrangement for the same reason.
 */
export const furnitureDocumentCss = `
${furnitureTags.scrollMark} {
  display: none !important;
}
`;
