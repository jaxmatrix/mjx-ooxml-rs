/**
 * The feedback vocabulary — **one table, read by five components, both gates and every caption.**
 *
 * MJXOFF-189 builds the five things a person sees *while* something is happening: a mini toolbar
 * beside a selection, a screentip under a pointer, a toast that arrives and leaves, a progress bar,
 * and the empty state that stands in for content that is not there. They have one property in
 * common and it is the reason they are one child:
 *
 * > **Every one of them is invisible in a still.**
 *
 * A screenshot of a screentip cannot say whether it waited; a screenshot of a toast cannot say
 * whether it was announced, or to whom; a screenshot of a progress bar cannot tell a bar that is
 * working from a bar that has stopped. So almost nothing here is a picture and almost everything is
 * either **arithmetic over time** or **a fact about the accessibility tree**, and both are testable
 * without a camera.
 *
 * ## The four decisions this file exists to make once
 *
 * ### 1. A span of time is a multiple of the one duration token
 *
 * The generated table declares exactly one duration. Five more spans are needed — a screentip's
 * delay, its warm-up window, its leave grace, a toast's dwell and an indeterminate bar's cycle —
 * and every one of
 * them is `calc(var(--duration-transition) * n)` rather than a number, for the reason
 * `foundations/motion.ts` now records: a delay written as a ratio follows a host that slows the
 * platform down for someone who needs longer to read, and a delay written as a number does not.
 *
 * ### 2. **A tone is never carried by colour alone**, and that is measured rather than asserted
 *
 * This palette has one alarm colour. It has no red, and inventing one would override the branding
 * of whoever opens the editor — the project's standing rule about deferring to the user's own
 * document, applied to its chrome. So a toast's tone is carried by **four** signals, of which the
 * colour is the weakest, and `tests/feedback.test.ts` measures why: `theme.light.accent` and
 * `theme.light.secondaryAccent` differ in hue and are within a hair of each other in *luminance*.
 * Two tones drawn only in those two colours would be hard to tell apart for a person with a colour
 * deficiency and **identical to a contrast gate** — so `toastSignals` lists what actually
 * distinguishes them and the gate asserts every pair differs in something that is not a colour.
 *
 * ### 3. A queue is a queue, and every entry carries its own clock
 *
 * `ToastQueue` is pure and takes `now` as an argument, so the whole of *"a second toast does not
 * cancel the first one's timer"* is a Node test over a sequence of instants rather than a browser
 * test that waits. The browser then proves only the thing Node cannot: that real time reaches it.
 *
 * ### 4. An indeterminate bar is not a stalled determinate one, by three instruments
 *
 * MJXOFF-189 names the trap. A determinate bar at one value exercises no arithmetic, and an
 * indeterminate bar that merely *renders* is satisfied by a bar that has stopped. So the two are
 * told apart three ways at once — the accessibility tree (`aria-valuenow` is absent), the geometry
 * (the indicator spans a fixed fraction of the track rather than the value's fraction) and time
 * (its position changes between two samples) — and the gate asserts all three, because under a
 * reduced-motion preference the third is deliberately switched off and the first two are all that
 * is left.
 *
 * ## Node-importable
 *
 * Data, arithmetic and strings. No custom element is defined here, so `tests/feedback.test.ts` can
 * sweep the tone table in Node and `tests/browser/feedback.spec.ts` can import the model it is
 * comparing a browser against.
 */

import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { contrastRatioOrWorst, formatRatio, nonTextMinimum } from '../tokens/contrast.ts';
import type { ThemeMember } from '../tokens/resolver.ts';
import {
  accessibleHitTargetMinimum,
  densityProperties,
  spacingMultiple,
} from '../foundations/density.ts';
import { durationMultiple, motionRoleClass } from '../foundations/motion.ts';
import { radiusVariable, surfaceLevels, type SurfaceLevel } from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { controlStatesCss, themeVariable } from '../controls/control-states.ts';
import { floatingProperties, type LogicalSide } from '../overlay/floating.ts';
import type { IconSize, IconVariant } from '../icons/manifest.ts';

// ── the five elements ────────────────────────────────────────────────────────

/** The custom element names, so a gate never spells one itself. */
export const feedbackTags = {
  miniToolbar: 'mjx-mini-toolbar',
  screentip: 'mjx-screentip',
  toastRegion: 'mjx-toast-region',
  toast: 'mjx-toast',
  progress: 'mjx-progress',
  emptyState: 'mjx-empty-state',
} as const;

/** The five story titles, as string literals — CSF refuses a computed one. */
export const feedbackStoryTitles = {
  miniToolbar: 'Feedback/Mini Toolbar',
  screentip: 'Feedback/Screentip',
  toast: 'Feedback/Toast',
  progress: 'Feedback/Progress',
  emptyState: 'Feedback/Empty State',
} as const;

/** Every event this child emits. One vocabulary; the detail says which component spoke. */
export const feedbackEvents = {
  /** A mini-toolbar command was activated. `detail: { command, pressed }`. */
  command: 'mjx-mini-command',
  /** A screentip became visible, after its delay. `detail: { title, waited }`. */
  screentipShow: 'mjx-screentip-show',
  /** A screentip went away. `detail: { title, reason }`. */
  screentipHide: 'mjx-screentip-hide',
  /** A toast entered the region. `detail: { id, tone, politeness }`. */
  toastShow: 'mjx-toast-show',
  /** A toast left it. `detail: { id, tone, reason }`. */
  toastDismiss: 'mjx-toast-dismiss',
  /** An empty state's action was activated. `detail: { command }`. */
  emptyStateAction: 'mjx-empty-state-action',
} as const;

// ── time, expressed as multiples of the one duration token ───────────────────

/**
 * The registered `<time>` properties this child reads.
 *
 * Registered, because an *unregistered* custom property hands `getComputedStyle` back the
 * substituted text — `calc(150ms * 4)` — and `Number.parseFloat` on that returns the transition's
 * own number, which is a delay four times too short and looks like nothing in a diff. See
 * `resolveDurationMilliseconds` in `foundations/motion.ts`.
 */
export const feedbackTimingProperties = {
  screentipAppear: '--mjx-screentip-appear-delay',
  screentipWarm: '--mjx-screentip-warm-window',
  screentipLeaveGrace: '--mjx-screentip-leave-grace',
  toastDwell: '--mjx-toast-dwell',
  toastDwellLong: '--mjx-toast-dwell-long',
  progressCycle: '--mjx-progress-cycle',
} as const;

/** One of the six spans. */
export type FeedbackTiming = keyof typeof feedbackTimingProperties;

/** The six, in the order the catalogue lists them. */
export const feedbackTimingNames = Object.keys(
  feedbackTimingProperties,
) as readonly FeedbackTiming[];

/**
 * How many transition durations each span is.
 *
 * ⚠ **These are ratios, not durations**, and that is the whole point — see the module note. The
 * numbers themselves answer to behaviour rather than taste: a screentip that appears in much under
 * half a second appears while a pointer is merely crossing the toolbar, and one that takes much
 * over a second reads as broken; a toast that leaves in under about five seconds leaves before it
 * has been read.
 *
 * `screentipLeaveGrace` is **WCAG 2.2's 1.4.13 (Content on Hover or Focus)** rather than a
 * flourish: content that appears on hover must stay while the pointer travels into it, or a person
 * using magnification cannot read a tip that is wider than their viewport. It is the shortest of
 * the spans, because the only thing it has to cover is the gap between a trigger and its tip.
 */
export const feedbackTimingMultiples: Readonly<Record<FeedbackTiming, number>> = {
  screentipAppear: 4,
  screentipWarm: 6,
  screentipLeaveGrace: 2,
  toastDwell: 40,
  toastDwellLong: 60,
  progressCycle: 10,
};

/**
 * A span in milliseconds, computed from the **generated** token value.
 *
 * This is the floor a component falls back to when the cascade cannot answer — never zero, because
 * a zero screentip delay turns *"it does not appear before the delay"* into a claim that cannot
 * fail, which is the exact shape of vacuity this child is written against.
 */
export function feedbackTimingMilliseconds(span: FeedbackTiming): number {
  const declared = Number.parseFloat(tokens.duration.transition);
  const base = Number.isFinite(declared) ? declared : 0;
  return base * feedbackTimingMultiples[span];
}

/** `screentipAppear` → `calc(var(--duration-transition) * 4)`. */
export function feedbackTimingValue(span: FeedbackTiming): string {
  return durationMultiple(feedbackTimingMultiples[span]);
}

/** The registrations and the defaults. Adopted on a shadow root *and* on the document. */
export const feedbackTimingCss = `
${feedbackTimingNames
  .map(
    (span) => `@property ${feedbackTimingProperties[span]} {
  syntax: '<time>';
  inherits: true;
  initial-value: 0s;
}`,
  )
  .join('\n')}

:where(:root, .mjx-foundations, .mjx-feedback) {
${feedbackTimingNames
  .map((span) => `  ${feedbackTimingProperties[span]}: ${feedbackTimingValue(span)};`)
  .join('\n')}
}
`;

// ── the screentip's delay ────────────────────────────────────────────────────

/** The two spans a screentip's schedule is made of, in milliseconds. */
export interface ScreentipSchedule {
  /** How long a pointer must rest on a trigger before the tip appears. */
  readonly appear: number;
  /** How long after a tip goes away the *next* one appears with no wait at all. */
  readonly warm: number;
}

/** Why a screentip went away. `escape` is the one that must not move focus. */
export const screentipHideReasons = [
  'leave',
  'escape',
  'blur',
  'activate',
  'programmatic',
] as const;

/** One of the five. */
export type ScreentipHideReason = (typeof screentipHideReasons)[number];

/**
 * How long *this* screentip must wait, given when the last one went away.
 *
 * **The warm-up is not decoration.** A person reading a toolbar moves along it, and a delay applied
 * afresh to every button turns a row of commands into a row of things that will not tell you what
 * they are. Office has had this behaviour for as long as it has had screentips, and it is what
 * makes the delay bearable rather than merely correct.
 *
 * The boundary is **inclusive**: a tip that went away exactly `warm` ago is still warm. Written
 * down because a boundary a test does not know about is a boundary the test samples past.
 *
 * A `now` earlier than `lastHiddenAt` — a clock that went backwards — waits the full delay rather
 * than treating a negative age as warm, which is the answer that cannot surprise anybody.
 *
 * @param now the current instant.
 * @param lastHiddenAt when the previous tip went away, or `undefined` if none has.
 */
export function screentipDelayFor(
  now: number,
  lastHiddenAt: number | undefined,
  schedule: ScreentipSchedule,
): number {
  if (lastHiddenAt === undefined) return schedule.appear;
  const since = now - lastHiddenAt;
  if (since < 0) return schedule.appear;
  return since <= schedule.warm ? 0 : schedule.appear;
}

/**
 * The shared warmth, because it is a property of the person's pointer and not of one component.
 *
 * A class rather than two module-level variables, so `tests/feedback.test.ts` can drive a sequence
 * against a fresh one instead of against whatever the previous test left behind.
 */
export class ScreentipWarmth {
  #lastHiddenAt: number | undefined;

  /** When the last tip went away, or `undefined`. */
  get lastHiddenAt(): number | undefined {
    return this.#lastHiddenAt;
  }

  /** Record that a tip has gone away. */
  noteHidden(now: number): void {
    this.#lastHiddenAt = now;
  }

  /** Forget everything — what a test does between sequences, and nothing else. */
  reset(): void {
    this.#lastHiddenAt = undefined;
  }

  /** The delay the next tip must wait, given this warmth. */
  delayFor(now: number, schedule: ScreentipSchedule): number {
    return screentipDelayFor(now, this.#lastHiddenAt, schedule);
  }
}

/** The one every `<mjx-screentip>` shares. */
export const screentipWarmth = new ScreentipWarmth();

// ── the toast tones ──────────────────────────────────────────────────────────

/** The four tones, in the order the catalogue lists them. */
export const toastToneNames = ['info', 'success', 'warning', 'error'] as const;

/** One of the four. */
export type ToastTone = (typeof toastToneNames)[number];

/** Whether a value names one of the four. */
export function isToastTone(value: unknown): value is ToastTone {
  return (toastToneNames as readonly string[]).includes(String(value));
}

/** The two live-region politenesses, and there are exactly two. */
export const toastPolitenessNames = ['polite', 'assertive'] as const;

/** One of the two. */
export type ToastPoliteness = (typeof toastPolitenessNames)[number];

/**
 * The role each politeness gets.
 *
 * ⚠ **Two regions, never one whose `aria-live` is swapped.** A live region's politeness is settled
 * when the region enters the accessibility tree, and rewriting the attribute on a live region is
 * the classic way to ship an announcement nobody hears — or, worse, a polite message that
 * interrupts. So `<mjx-toast-region>` builds one of each and routes by tone, and
 * `tests/browser/feedback.spec.ts` asserts that a polite toast leaves the assertive region
 * **empty**, which is the whole accessibility contract of a toast in one assertion.
 */
export const politenessRoles: Readonly<Record<ToastPoliteness, 'status' | 'alert'>> = {
  polite: 'status',
  assertive: 'alert',
};

/** How long a tone stays. `persistent` means *until a person dismisses it*. */
export type ToastDwell = 'persistent' | Extract<FeedbackTiming, 'toastDwell' | 'toastDwellLong'>;

/** Everything a tone fixes. */
export interface ToastToneSpec {
  /** Which live region it is announced into. */
  readonly politeness: ToastPoliteness;
  /** The glyph. Its **name and variant together** are one of the four non-colour signals. */
  readonly icon: { readonly name: string; readonly size: IconSize; readonly variant: IconVariant };
  /** The theme member the leading edge is drawn in. The *weakest* of the four signals. */
  readonly edge: ThemeMember;
  /** How long it stays. */
  readonly dwell: ToastDwell;
  readonly use: string;
}

/**
 * The four tones.
 *
 * **`warning` and `error` share the palette's one alarm colour on purpose.** There is no red in
 * this brand, and adding one would impose our values on a document whose owner chose the palette.
 * The two are told apart by the drawing (regular against filled — the icon manifest says the filled
 * one *"is the drawing a warning actually wants"*), by the politeness, and by the fact that an
 * error does not dismiss itself. Three differences, none of them a hue.
 */
export const toastTones: Readonly<Record<ToastTone, ToastToneSpec>> = {
  info: {
    politeness: 'polite',
    icon: { name: 'info', size: 16, variant: 'regular' },
    edge: 'textSecondary',
    dwell: 'toastDwell',
    use: 'Something happened that a person may want to know and need not act on.',
  },
  success: {
    politeness: 'polite',
    icon: { name: 'checkmark', size: 16, variant: 'regular' },
    edge: 'accent',
    dwell: 'toastDwell',
    use: 'A thing the person asked for finished.',
  },
  warning: {
    politeness: 'polite',
    icon: { name: 'warning', size: 16, variant: 'regular' },
    edge: 'secondaryAccent',
    dwell: 'toastDwellLong',
    use: 'The result is not what was asked for and the document is still fine. A longer dwell, because it has to be read.',
  },
  error: {
    politeness: 'assertive',
    icon: { name: 'warning', size: 16, variant: 'filled' },
    edge: 'secondaryAccent',
    dwell: 'persistent',
    use: 'Something failed and the person has to decide. Announced assertively, and it never leaves on its own.',
  },
};

/**
 * The signals that tell one tone from another, **not one of which is a colour**.
 *
 * The gate asserts every pair of tones differs in at least one of these. That is WCAG 1.4.1 stated
 * as a test rather than as an intention, and it is necessary rather than ceremonial — see
 * `toneColourSeparation`, which measures how little this palette's two accents differ in the one
 * dimension a contrast ratio can see.
 */
export function toastSignals(tone: ToastTone): readonly string[] {
  const spec = toastTones[tone];
  return [
    `icon:${spec.icon.name}`,
    `variant:${spec.icon.variant}`,
    `politeness:${spec.politeness}`,
    `dwell:${String(spec.dwell)}`,
  ];
}

/** A member's generated value for one scheme. What the sweeps measure with. */
export function themeColor(scheme: ColorScheme, member: ThemeMember): string {
  return tokens.theme[scheme][member as keyof (typeof tokens.theme)['light']];
}

/** What a toast is drawn on: the overlay rung, named once. */
export const toastSurfaceLevel: SurfaceLevel = 'overlay';

/** The theme member that rung fills with, so the edge is measured against the real thing. */
export const toastSurfaceMember: ThemeMember = 'surfaceRaised';

/** How well a tone's edge reads against the toast's own fill, in one scheme. */
export function toastEdgeContrast(tone: ToastTone, scheme: ColorScheme): number {
  return contrastRatioOrWorst(
    themeColor(scheme, toastTones[tone].edge),
    themeColor(scheme, toastSurfaceMember),
  );
}

/**
 * How far apart two tones' edges are **in luminance**, which is all a contrast ratio can see.
 *
 * ⚠ **This is the measurement that justifies `toastSignals`.** A ratio near `1` means a contrast
 * gate would call the two colours identical however different they look to a reader with typical
 * colour vision — and a reader without it may genuinely not tell them apart. The story caption
 * quotes this number, and the gate asserts the non-colour signals exist *because* of it.
 */
export function toneColourSeparation(a: ToastTone, b: ToastTone, scheme: ColorScheme): number {
  return contrastRatioOrWorst(
    themeColor(scheme, toastTones[a].edge),
    themeColor(scheme, toastTones[b].edge),
  );
}

/** WCAG 1.4.11's floor and the ratio formatter, re-exported so gates and captions read one number. */
export { nonTextMinimum, formatRatio };

// ── the queue ────────────────────────────────────────────────────────────────

/**
 * How many toasts may be on screen at once.
 *
 * Three, because a stack deep enough to cover the document has become a dialog. The gate asserts
 * the ceiling **and** that it kept a positive number **and** that the entries it kept are the right
 * ones — a ceiling satisfied by an empty stack is U06's defect in a new costume.
 */
export const toastStackCeiling = 3;

/** Why a toast left. */
export const toastDismissReasons = ['expired', 'dismissed', 'retired', 'cleared'] as const;

/** One of the four. */
export type ToastDismissReason = (typeof toastDismissReasons)[number];

/** What a caller asks for. */
export interface ToastRequest {
  readonly tone: ToastTone;
  readonly message: string;
  /** An optional single action, e.g. “Undo”. */
  readonly action?: { readonly label: string; readonly command: string };
}

/** A toast in the queue. */
export interface ToastEntry extends ToastRequest {
  readonly id: string;
  readonly shownAt: number;
  /** When it leaves, or `undefined` for a persistent one. */
  readonly expiresAt: number | undefined;
}

/** One entry and why it left, so a caller can treat a retirement differently from an expiry. */
export interface ToastDeparture {
  readonly entry: ToastEntry;
  readonly reason: ToastDismissReason;
}

let nextToastSerial = 0;

/**
 * The toast queue — **pure, and time is an argument.**
 *
 * Everything MJXOFF-189 asks about a queue is a property of a sequence of instants: that order is
 * kept, that a second toast does not cancel the first one's timer, that the ceiling retires the
 * right entry. A queue that owned a `setTimeout` could only be tested by waiting, and a test that
 * waits is a test that samples the end. This one is swept in Node at a dozen instants, and the
 * browser proves only the one thing Node cannot — that real time reaches it.
 */
export class ToastQueue {
  #entries: ToastEntry[] = [];

  /** The entries on screen, **oldest first**, which is also the order they are rendered in. */
  get entries(): readonly ToastEntry[] {
    return this.#entries;
  }

  /** How many are on screen. */
  get size(): number {
    return this.#entries.length;
  }

  /**
   * Add one, and report anything the ceiling pushed out.
   *
   * @param dwellFor how long a tone stays, in milliseconds, or `undefined` for a persistent one.
   *   An argument rather than a lookup, because the dwell is a custom property a host may retune
   *   and this class may not read a stylesheet.
   */
  push(
    request: ToastRequest,
    now: number,
    dwellFor: (tone: ToastTone) => number | undefined,
  ): { readonly entry: ToastEntry; readonly retired: readonly ToastDeparture[] } {
    nextToastSerial += 1;
    const dwell = dwellFor(request.tone);
    const entry: ToastEntry = {
      ...request,
      id: `mjx-toast-${String(nextToastSerial)}`,
      shownAt: now,
      expiresAt: dwell === undefined ? undefined : now + dwell,
    };
    this.#entries.push(entry);
    const retired: ToastDeparture[] = [];
    while (this.#entries.length > toastStackCeiling) {
      const victim = this.#chooseRetirement();
      if (victim === undefined) break;
      this.#entries.splice(this.#entries.indexOf(victim), 1);
      retired.push({ entry: victim, reason: 'retired' });
    }
    return { entry, retired };
  }

  /**
   * Which entry the ceiling pushes out.
   *
   * **The oldest one that leaves on its own**, and only if every entry is persistent, the oldest
   * outright. An error that has not been read must not be shunted off the screen by three “Saved”
   * messages, and a ceiling that simply took the head of the queue would do exactly that.
   */
  #chooseRetirement(): ToastEntry | undefined {
    return this.#entries.find((entry) => entry.expiresAt !== undefined) ?? this.#entries[0];
  }

  /** Remove everything whose time is up. Returns what left, oldest first. */
  expire(now: number): readonly ToastDeparture[] {
    const gone: ToastDeparture[] = [];
    this.#entries = this.#entries.filter((entry) => {
      if (entry.expiresAt !== undefined && entry.expiresAt <= now) {
        gone.push({ entry, reason: 'expired' });
        return false;
      }
      return true;
    });
    return gone;
  }

  /** Remove one by id. Returns it, or `undefined` if it was not there. */
  dismiss(id: string): ToastEntry | undefined {
    const index = this.#entries.findIndex((entry) => entry.id === id);
    if (index < 0) return undefined;
    const [entry] = this.#entries.splice(index, 1);
    return entry;
  }

  /** Remove everything. */
  clear(): readonly ToastDeparture[] {
    const gone = this.#entries.map((entry) => ({ entry, reason: 'cleared' as const }));
    this.#entries = [];
    return gone;
  }

  /** The next instant anything expires, or `undefined` when nothing does. */
  nextExpiry(): number | undefined {
    let soonest: number | undefined;
    for (const entry of this.#entries) {
      if (entry.expiresAt === undefined) continue;
      if (soonest === undefined || entry.expiresAt < soonest) soonest = entry.expiresAt;
    }
    return soonest;
  }
}

// ── progress ─────────────────────────────────────────────────────────────────

/** The two kinds. */
export const progressKindNames = ['determinate', 'indeterminate'] as const;

/** One of the two. */
export type ProgressKind = (typeof progressKindNames)[number];

/**
 * A value and a maximum, as a fraction of the whole, clamped into `[0, 1]`.
 *
 * ⚠ **The identity-value trap lives here.** A bar tested only at `1` — or only at `0.5` — exercises
 * none of this: the two clamps, the non-unit maximum, the zero maximum and the two non-finite cases
 * are every line of it, and `tests/feedback.test.ts` drives all of them. A `max` of zero is not a
 * complete task, it is a task with nothing in it, so it reports nothing done rather than everything.
 */
export function progressFraction(value: number, max: number): number {
  if (!Number.isFinite(value) || !Number.isFinite(max)) return 0;
  if (max <= 0) return 0;
  if (value <= 0) return 0;
  if (value >= max) return 1;
  return value / max;
}

/** `0.42` → `42%`. Rounded: a bar is not a readout and a readout is not a bar. */
export function formatProgressPercent(fraction: number): string {
  const clamped = Number.isFinite(fraction) ? Math.min(Math.max(fraction, 0), 1) : 0;
  return `${String(Math.round(clamped * 100))}%`;
}

/**
 * What an assistive technology says.
 *
 * An indeterminate bar has **no percentage**, and stating one would be a lie about a quantity
 * nobody knows. It says what is happening instead, which is the only true thing there is to say.
 */
export function progressValueText(kind: ProgressKind, fraction: number, label: string): string {
  if (kind === 'indeterminate') return `${label}: working`;
  return `${label}: ${formatProgressPercent(fraction)}`;
}

/**
 * How much of the track an **indeterminate** indicator covers.
 *
 * Fixed, and deliberately not a round fraction a determinate bar would be read at: a bar covering
 * the whole track is a *completed* bar, and one covering a third is a bar stalled at 33 per cent.
 * It is a `%` in the stylesheet, which is a layout relationship rather than a measurement, so no
 * length is written down.
 */
export const indeterminateSpanFraction = 0.45;

/** How far the indeterminate indicator travels, as a multiple of the track, in each direction. */
export const indeterminateTravel = { from: -0.6, to: 1.6 } as const;

// ── the mini toolbar ─────────────────────────────────────────────────────────

/** What a mini toolbar shows. Data, never markup — the descriptor doctrine, as a property. */
export interface MiniCommand {
  /** What is emitted when it is activated. */
  readonly command: string;
  /** Its accessible name. Always present: a mini toolbar is icons, so the name is all there is. */
  readonly label: string;
  /** A glyph from the committed subset. */
  readonly icon?: string;
  /** A one-shot command, or a two-state toggle. */
  readonly kind?: 'command' | 'toggle';
  /** For a toggle: whether it is on. */
  readonly pressed?: boolean;
  /** Unavailable-but-explained, exactly as the ribbon's controls mean it. */
  readonly unavailable?: boolean;
  readonly explanation?: string;
  /** A rule drawn before this command. A mini toolbar groups; it does not run on. */
  readonly separatorBefore?: boolean;
}

/**
 * The sides a mini toolbar looks for room in, most-wanted first.
 *
 * **Above the selection first**, because that is where Office puts it and because a toolbar below a
 * selection sits over the next line a person is about to read. The inline sides are the fallback
 * for a selection that is tall, and they are what make *"it never covers the selection"* a promise
 * rather than a hope.
 */
export const miniToolbarClearanceOrder: readonly LogicalSide[] = [
  'blockStart',
  'blockEnd',
  'inlineEnd',
  'inlineStart',
];

/** The sides a screentip looks for room in. Below the trigger first: a tip is a caption. */
export const screentipClearanceOrder: readonly LogicalSide[] = [
  'blockEnd',
  'blockStart',
  'inlineEnd',
  'inlineStart',
];

/**
 * The attribute a mini toolbar writes when no side cleared the selection.
 *
 * It exists so the promise is **falsifiable**. `placeClearOfAnchor` reports failure rather than
 * hiding it, this is where that report lands in the DOM, and
 * `stories/feedback/mini-toolbar.stories.ts` ships a selection that fills its room so a gate can
 * watch the attribute become `true`. A flag no story can turn on is a flag no gate is testing.
 */
export const coveringAttribute = 'data-covering';

// ── the boxes ────────────────────────────────────────────────────────────────

/** The custom properties this child's boxes read. Layout only; no paint goes through one. */
export const feedbackBoxProperties = {
  /** How wide a toast may be. */
  toastInlineSize: '--mjx-toast-inline-size',
  /** How thick a progress track is. */
  progressTrack: '--mjx-progress-track-size',
  /** The determinate indicator's width, written by the component as a percentage. */
  progressFill: '--mjx-progress-fill',
} as const;

/** The two lengths above, in spacing units, so a re-seed of `--spacing` moves both. */
export const feedbackBoxUnits = { toastInlineSize: 80, progressTrack: 2 } as const;

const overlayRung = surfaceLevels.overlay;

/**
 * **The platform's own `hidden` has to win, and it does not win by itself.**
 *
 * ⚠ Measured rather than assumed, and it cost a browser gate to find. The UA stylesheet's
 * `[hidden] { display: none }` lives in the *user-agent* origin, so **any author rule at all**
 * beats it — and a component that writes `display: inline-flex` on a class has therefore silently
 * disabled `hidden` for every element wearing it. `<mjx-empty-state>` hides its action when there
 * is no action to offer; the attribute was set, the accessibility tree was right, and the button
 * was on screen. Every assertion about the attribute passed.
 *
 * So each sheet restates it, at the same (0,1,0) the class rules score, **and last** — which is the
 * only thing that decides a tie at equal specificity. `tests/feedback.test.ts` asserts the position
 * rather than trusting it, for exactly the reason MJXOFF-188 records about a `@container` block
 * emitted too early.
 */
const hiddenLastCss = `
  [hidden] { display: none; }
`;

/** Announced, never drawn. The same declarations every other child's components use. */
const visuallyHiddenCss = `
  .visually-hidden {
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
 * The box an anchored surface is drawn in, positioned by `overlay/floating.ts`.
 *
 * ⚠ **`position: fixed` with coordinates from custom properties**, never `inset` zeroes, and the
 * reason is the one `pinFloating` records: a fixed box's percentages resolve against its containing
 * block, Chromium does not make a `container-type` element one, and this catalogue's harness frame
 * is exactly that — so a box positioned by CSS alone escapes the frame it is being audited inside.
 *
 * `top`/`left` rather than the logical properties, matching `popoverCss`: the coordinates written
 * by `applyPlacement` are physical and viewport-relative, and writing them into logical properties
 * would mirror them a second time under right-to-left.
 */
const anchoredBoxCss = `
  .anchored {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    color: inherit;
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    inline-size: max-content;
    max-block-size: var(${floatingProperties.maxBlockSize});
    overflow: clip;
  }

  .anchored[data-open='false'] { display: none; }
`;

/**
 * `<mjx-mini-toolbar>`'s rules.
 *
 * The commands are real buttons in this component's own shadow root, and their paint comes from
 * `controlStatesCss('.command')` — the same ten-state table the ribbon's controls wear. A second
 * hover colour for a mini toolbar is how a design system acquires two hover colours.
 */
export const miniToolbarCss = `
${feedbackTimingCss}
${anchoredBoxCss}
${visuallyHiddenCss}

  :host { display: contents; }
  :host([hidden]) { display: none; }

  .toolbar {
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
    padding: var(${densityProperties.step});
    max-inline-size: var(${floatingProperties.maxInlineSize});
    background: ${overlayRung.background};
    border: ${overlayRung.border};
    border-radius: ${radiusVariable(overlayRung.radius)};
    box-shadow: ${overlayRung.shadow};
    opacity: 1;
    transition-property: opacity;
  }

  .toolbar[data-entering] { opacity: 0; }

  /* Paint comes from controlStatesCss('.command'). This rule owns the box and nothing else, for
   * the specificity reason control-states.ts states at length: a background declared here would
   * out-specify the whole state table and win in all ten states. */
  .command {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    margin: 0;
    padding: 0;
    border-width: 1px;
    border-radius: ${radiusVariable('control')};
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .command:disabled { cursor: default; }
  .command[aria-disabled='true'] { cursor: help; }

  .separator {
    inline-size: 1px;
    align-self: stretch;
    background: var(--theme-border);
  }

${controlStatesCss('.command')}
${hiddenLastCss}
`;

/**
 * `<mjx-screentip>`'s rules.
 *
 * A tip is a *caption*, so it wears the label and dense type roles rather than the control role,
 * and it is never wider than a readable measure — a screentip that ran the width of a desktop would
 * be a paragraph nobody reads. The cap is a `min()` of the measure and the room the boundary left,
 * so the two constraints compose instead of one silently defeating the other.
 */
export const screentipCss = `
${feedbackTimingCss}
${anchoredBoxCss}
${visuallyHiddenCss}

  :host { display: contents; }
  :host([hidden]) { display: none; }

  .tip {
    display: grid;
    gap: calc(var(${densityProperties.step}) / 2);
    padding: var(${densityProperties.gutter});
    max-inline-size: min(36ch, var(${floatingProperties.maxInlineSize}));
    background: ${overlayRung.background};
    border: ${overlayRung.border};
    border-radius: ${radiusVariable('card')};
    box-shadow: ${overlayRung.shadow};
    color: var(--theme-text-primary);
    opacity: 1;
    transition-property: opacity;
  }

  .tip[data-entering] { opacity: 0; }

  .tip-title,
  .tip-description,
  .tip-shortcut {
    margin: 0;
  }

  .tip-description,
  .tip-shortcut {
    color: var(--theme-text-secondary);
  }
${hiddenLastCss}
`;

/** `<mjx-toast-region>` and `<mjx-toast>`. */
export const toastCss = `
${feedbackTimingCss}
${visuallyHiddenCss}

  :host { display: contents; }
  :host([hidden]) { display: none; }

  .region {
    box-sizing: border-box;
    margin: 0;
    padding: var(${densityProperties.gutter});
    color: inherit;
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    inline-size: var(${floatingProperties.inlineSize});
    max-block-size: var(${floatingProperties.maxBlockSize});
    display: flex;
    flex-direction: column;
    align-items: end;
    justify-content: end;
    gap: var(${densityProperties.step});
    background: none;
    border: none;
    overflow: visible;
    pointer-events: none;
  }

  .region[data-open='false'] { display: none; }

  .region > * { pointer-events: auto; }

  .toast {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: start;
    gap: var(${densityProperties.step});
    box-sizing: border-box;
    inline-size: min(var(${feedbackBoxProperties.toastInlineSize}), 100%);
    padding: var(${densityProperties.gutter});
    background: ${overlayRung.background};
    border: ${overlayRung.border};
    border-inline-start-width: calc(var(--spacing) / 2);
    border-inline-start-style: solid;
    border-radius: ${radiusVariable('card')};
    box-shadow: ${overlayRung.shadow};
    color: var(--theme-text-primary);
    opacity: 1;
    translate: none;
    transition-property: opacity, translate;
  }

  .toast[data-entering] {
    opacity: 0;
    translate: 0 calc(var(--spacing) * 2);
  }

  /* The tone's edge, generated from the table so the stylesheet and the gate cannot disagree about
   * which member a tone paints with — and so the colour is *declared* rather than written onto an
   * element by JavaScript, which is what lets a browser gate read it back off the cascade. It is
   * the weakest of a tone's four signals; see toastSignals. */
${toastToneNames
  .map(
    (tone) =>
      `  .toast[data-tone='${tone}'] { border-inline-start-color: ${themeVariable(
        toastTones[tone].edge,
      )}; }`,
  )
  .join('\n')}

  .glyph { display: inline-flex; align-items: center; }

  .message { margin: 0; overflow-wrap: anywhere; }

  .actions {
    display: inline-flex;
    align-items: start;
    gap: var(${densityProperties.step});
  }

  .toast-action,
  .toast-dismiss {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    margin: 0;
    padding-inline: var(${densityProperties.step});
    border-width: 1px;
    border-radius: ${radiusVariable('control')};
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

${controlStatesCss('.toast-action')}
${controlStatesCss('.toast-dismiss')}
${hiddenLastCss}
`;

/**
 * `<mjx-progress>`.
 *
 * ⚠ **The reduced-motion block is last, and that is load-bearing.** MJXOFF-188 lost a whole
 * behaviour to a `@container` block emitted above the rules it overrode: *a media or container
 * block changes no specificity, so a block emitted first simply loses at equal specificity.* The
 * indeterminate rule below scores (0,2,1) and out-specifies the foundations' own
 * `:where(.mjx-motion-*)` reduced-motion rule at (0,0,0) — so if this block were anywhere but last,
 * an indeterminate bar would keep sweeping for a person who has asked their operating system to
 * stop moving things, and nothing but a person would notice.
 */
export const progressCss = `
${feedbackTimingCss}
${visuallyHiddenCss}

  :host { display: block; }
  :host([hidden]) { display: none; }

  .field {
    display: grid;
    gap: calc(var(${densityProperties.step}) / 2);
  }

  .heading {
    display: flex;
    justify-content: space-between;
    gap: var(${densityProperties.step});
    color: var(--theme-text-primary);
  }

  .readout {
    color: var(--theme-text-secondary);
    font-variant-numeric: tabular-nums;
  }

  .track {
    position: relative;
    block-size: var(${feedbackBoxProperties.progressTrack});
    overflow: hidden;
    background: var(--theme-border-subtle);
    border-radius: ${radiusVariable('chip')};
  }

  .indicator {
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    background: var(--theme-accent);
    border-radius: ${radiusVariable('chip')};
    transition-property: inline-size;
  }

  /* Determinate: the indicator's width IS the value, and nothing else writes that property. */
  .track[data-kind='determinate'] > .indicator {
    inline-size: var(${feedbackBoxProperties.progressFill}, 0%);
  }

  /* Indeterminate: a fixed span that travels, so it can never be read as a value. Its position is
   * what a sample-two-instants gate looks at. */
  .track[data-kind='indeterminate'] > .indicator {
    inline-size: ${String(indeterminateSpanFraction * 100)}%;
    animation-name: mjx-progress-sweep;
    animation-duration: var(${feedbackTimingProperties.progressCycle});
    animation-timing-function: linear;
    animation-iteration-count: infinite;
  }

  @keyframes mjx-progress-sweep {
    from { translate: ${String(indeterminateTravel.from * 100)}% 0; }
    to   { translate: ${String(indeterminateTravel.to * 100)}% 0; }
  }

  @media (prefers-reduced-motion: reduce) {
    .track[data-kind='indeterminate'] > .indicator {
      animation-name: none;
      inline-size: 100%;
      translate: none;
    }
  }
${hiddenLastCss}
`;

/**
 * `<mjx-empty-state>`.
 *
 * The heading is the **one** place in this platform the serif is allowed — `typography.ts` §4:
 * *Young Serif is display-only. It belongs in empty states and onboarding, never in the chrome or
 * in document content.* This is that place, and `tests/feedback.test.ts` asserts that this
 * component names the `display` role and that no other component in the catalogue does.
 */
export const emptyStateCss = `
${visuallyHiddenCss}

  :host { display: block; }
  :host([hidden]) { display: none; }

  .empty {
    display: grid;
    justify-items: center;
    text-align: center;
    gap: var(${densityProperties.gutter});
    padding: calc(var(${densityProperties.gutter}) * 2);
    color: var(--theme-text-primary);
  }

  .art {
    display: inline-flex;
    color: var(--theme-text-secondary);
  }

  .heading { margin: 0; }

  .description {
    margin: 0;
    max-inline-size: 48ch;
    color: var(--theme-text-secondary);
  }

  .actions {
    display: inline-flex;
    gap: var(${densityProperties.step});
    flex-wrap: wrap;
    justify-content: center;
  }

  .action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(${densityProperties.step});
    box-sizing: border-box;
    margin: 0;
    padding-inline: var(${densityProperties.gutter});
    border-width: 1px;
    border-radius: ${radiusVariable('control')};
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

${controlStatesCss('.action')}
${hiddenLastCss}
`;

/**
 * The rules that must reach the **document**.
 *
 * Three of them, and none is cosmetic:
 *
 * * **the timing registrations**, because `@property` is document-scoped wherever the sheet
 *   carrying it is applied and an unregistered `<time>` resolves to text a component cannot read;
 * * **the two box lengths**, so a shell that never sets them still has defined values;
 * * **the screentip's accessible description**, which lives in the *light* DOM and must be
 *   announced without being drawn. It is there rather than in the shadow root because an IDREF does
 *   not cross a shadow boundary: `aria-describedby` on a slotted trigger cannot name an element
 *   inside the component that slotted it. See `<mjx-screentip>` for the whole argument.
 */
export const feedbackDocumentCss = `
${feedbackTimingCss}

:where(.mjx-screentip-description) {
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

:where(:root, .mjx-foundations, .mjx-feedback) {
  ${feedbackBoxProperties.toastInlineSize}: ${spacingMultiple(feedbackBoxUnits.toastInlineSize)};
  ${feedbackBoxProperties.progressTrack}: ${spacingMultiple(feedbackBoxUnits.progressTrack)};
}
`;

/** The class the screentip's light-DOM description carries. Named once so a gate cannot mistype it. */
export const screentipDescriptionClass = 'mjx-screentip-description';

/** The type role each part of this child's boxes wears. Stated once so a gate can assert it. */
export const feedbackTypeRoles = {
  command: typeRoleClass('control'),
  tipTitle: typeRoleClass('label'),
  tipBody: typeRoleClass('dense'),
  toastMessage: typeRoleClass('body'),
  toastAction: typeRoleClass('control'),
  progressLabel: typeRoleClass('control'),
  progressReadout: typeRoleClass('dense'),
  emptyHeading: typeRoleClass('display'),
  emptyBody: typeRoleClass('body'),
  emptyAction: typeRoleClass('control'),
} as const;

/** The motion class each moving thing wears. Chrome settles; nothing here overshoots. */
export const feedbackMotionClasses = {
  toolbar: motionRoleClass('surfaceSettle'),
  tip: motionRoleClass('surfaceSettle'),
  toast: motionRoleClass('surfaceSettle'),
  progress: motionRoleClass('surfaceSettle'),
} as const;

/** The dismissal glyph a toast wears, at the mark size the rest of the catalogue uses. */
export const toastDismissIcon = { name: 'dismiss', size: 16, variant: 'regular' } as const;

/** The empty state's art. The one icon carried at 48, which is what makes this shape possible. */
export const emptyStateIcon = { name: 'document', size: 48, variant: 'regular' } as const;

/** The glyph a mini-toolbar command wears when it does not name one. */
export const miniCommandIconSize = 16;

/**
 * The role an empty state carries.
 *
 * `status`, and it is a decision rather than a default: an empty state is what a region becomes when
 * a filter, a search or a delete emptied it, so its appearance **is** an update and a person who
 * cannot see it has to be told. The alternative — a silent `<div>` with a picture in it — is the
 * failure MJXOFF-189 names: *"the easiest thing to ship wrong, because it renders fine with no data
 * by definition."*
 */
export const emptyStateRole = 'status';

/** The accessible name of a toast's dismissal, derived from the message it is dismissing. */
export function toastDismissLabel(message: string): string {
  return `Dismiss: ${message}`;
}

/** The floor a hit target may not go below, re-exported so this child's gates read one number. */
export { accessibleHitTargetMinimum };
