/**
 * The input model: what the seven controls of MJXOFF-186 are painted with, what their keys mean,
 * and the arithmetic a slider and a listbox are made of.
 *
 * **Node-importable.** Data, strings and pure functions — no DOM, no custom elements. That is what
 * lets `tests/inputs.test.ts` assert the geometry, the key maps and the contrast of every state
 * without a browser, and `tests/browser/inputs.spec.ts` then assert that the components *obey* it.
 *
 * ## Why a field does not reuse the control state table's paint
 *
 * MJXOFF-184's menu row does reuse it, wholesale, and says so. A field cannot, and the reason is a
 * measured one rather than a taste: **`--theme-text-secondary` is 4.32 : 1 on
 * `--theme-border-subtle`**, and `controlStateSpecs.hover` fills with exactly that. A combo box's
 * placeholder, a measure input's unit suffix and a slider's tick labels are all secondary text, so
 * a field that took the button's hover fill would have illegible secondary text *precisely while a
 * person was pointing at it* — legible in every screenshot, illegible in use.
 *
 * So a field is a **surface you type into**, not a button you press: its fill never changes, and
 * its hover, editing and invalid states move the **edge**. Every piece of secondary text inside a
 * field therefore sits on `--theme-surface` in all seven states, which is 5.23 : 1 in light and
 * 6.61 : 1 in dark. That is a design decision taken to make a contrast rule true by construction
 * rather than to make a gate pass, and `tests/inputs.test.ts` asserts the construction.
 *
 * The *machinery* is still shared and not copied: `controlStateDeclarations`, `paintValue`,
 * `themeVariable` and `disabledOpacity` all come from `control-states.ts`, and
 * `composeStatePaint`/`resolvePaintFingerprint` below are asserted **equal** to
 * `effectiveStatePaint`/`resolvedStateFingerprint` over all ten control states. A near-duplicate
 * that is checked against the thing it nearly duplicates is not a duplicate.
 *
 * ## The indicator rule, which is a gate rather than a paragraph
 *
 * WCAG 2.2 §1.4.11 wants a state indicator at 3 : 1 against what it is drawn on. Every border and
 * inset ring in every table below is checked against that, in both schemes, by
 * `tests/inputs.test.ts` — and a state that genuinely cannot clear it must declare a
 * `nonColourCue`, which the browser gate then asserts is actually rendered. Two states use that
 * escape hatch and both say why. It is not a waiver: a cue that is declared and not drawn fails.
 */

import {
  controlStateDeclarations,
  controlStateSpecs,
  controlStatesCss,
  disabledOpacity,
  paintValue,
  themeVariable,
  type ControlState,
  type ControlStateSpec,
  type EffectiveStatePaint,
  type Paint,
} from '../controls/control-states.ts';
import {
  accessibleHitTargetMinimum,
  densityProperties,
  spacingMultiple,
} from '../foundations/density.ts';
import { radiusVariable, surfaceLevels } from '../foundations/surfaces.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { motionRoleClass } from '../foundations/motion.ts';
import { floatingProperties } from '../overlay/floating.ts';
import { galleryRowPlan, galleryWindow, cellsInWindow } from '../gallery/gallery-model.ts';
import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { customPropertyCase, type ThemeMember } from '../tokens/resolver.ts';

/**
 * U06's virtualisation, re-exported rather than re-implemented.
 *
 * A dropdown of every installed font is the same problem a gallery of every table style is, and
 * `galleryRowPlan(items, 1, false)` is a one-column plan — one row per option — which is exactly a
 * list. Writing a second windowing function would have been a second set of off-by-one errors to
 * find, and the gate that compares a node count against `cellsInWindow` only means anything
 * because the number comes from somewhere other than the code that built the nodes.
 */
export { galleryRowPlan, galleryWindow, cellsInWindow };

/** The nine elements this child registers. */
export const inputTags = {
  label: 'mjx-label',
  checkbox: 'mjx-checkbox',
  dropdown: 'mjx-dropdown',
  comboBox: 'mjx-combo-box',
  option: 'mjx-option',
  measureInput: 'mjx-measure-input',
  slider: 'mjx-slider',
  segmentedControl: 'mjx-segmented-control',
  segment: 'mjx-segment',
} as const;

/**
 * The events the seven emit. **One change event, with one shape.**
 *
 * A host that has to listen for `mjx-checkbox-change`, `mjx-slider-change` and
 * `mjx-dropdown-change` separately is a host that will forget one. Every control here reports
 * `{ value, previous }` and the type of `value` is the control's own — a `PressedValue` for the
 * checkbox, a number of points for the measure input, a number for the slider, a string for the
 * three list controls.
 */
export const inputEvents = {
  /** The control now reports a new value. `detail: { value, previous }`. */
  change: 'mjx-input-change',
  /**
   * A value is being *tried* — a slider mid-drag, an arrow key through a dropdown's list.
   *
   * Uncommitted, and it may be followed by a `change` or by nothing at all. This is the event a
   * live preview listens to; `change` is the one an undo entry is made from.
   */
  preview: 'mjx-input-preview',
  /** A popup list opened or closed. `detail: { open }`. */
  toggle: 'mjx-input-toggle',
  /**
   * What a person typed could not be read. `detail: { text, failure, offending }`.
   *
   * Nothing was committed **and nothing was reverted** — see `<mjx-measure-input>`.
   */
  invalid: 'mjx-input-invalid',
} as const;

// ── the shared composition machinery ─────────────────────────────────────────

/**
 * What a partial spec actually paints, once the resting row underneath it is composed in.
 *
 * `effectiveStatePaint` is this function over the control table; it takes a `ControlState` key, so
 * a table with different rows cannot call it. `tests/inputs.test.ts` asserts the two agree for all
 * ten control states, which is what makes this a shared composition rather than a second one.
 */
export function composeStatePaint(
  spec: ControlStateSpec,
  resting: ControlStateSpec,
): EffectiveStatePaint {
  return {
    background: spec.background ?? resting.background ?? 'transparent',
    borderColor: spec.borderColor ?? resting.borderColor ?? 'transparent',
    borderStyle: spec.borderStyle ?? resting.borderStyle ?? 'solid',
    text: spec.text ?? resting.text ?? 'textPrimary',
    weight: spec.weight ?? resting.weight ?? 'medium',
    opacity: spec.opacity ?? resting.opacity ?? 1,
    insetRing: spec.insetRing,
    ring: spec.ring === true,
  };
}

/** A composed paint, resolved through the generated tokens. The string a pairwise gate compares. */
export function resolvePaintFingerprint(paint: EffectiveStatePaint, scheme: ColorScheme): string {
  const colour = (value: Paint | undefined): string =>
    value === undefined || value === 'transparent' ? 'transparent' : tokens.theme[scheme][value];
  return [
    `background:${colour(paint.background)}`,
    `border:${colour(paint.borderColor)} ${paint.borderStyle}`,
    `color:${colour(paint.text)}`,
    `weight:${paint.weight}`,
    `opacity:${String(paint.opacity)}`,
    `ring-inset:${colour(paint.insetRing)}`,
    `focus-ring:${String(paint.ring)}`,
  ].join(' | ');
}

/**
 * One row of a derived state table.
 *
 * `paint` is either the **name of a row in the shared control table** — in which case nothing is
 * restated and a change there arrives here — or a spec this table owns, which must then carry a
 * `because` saying what the shared table could not do. The gate asserts every locally-declared
 * paint has one.
 */
export interface DerivedStateSpec {
  readonly paint: ControlState | ControlStateSpec;
  /** Why this table declares its own paint. Required exactly when `paint` is a spec. */
  readonly because?: string;
  /** What puts the control into it, and what the auditor should look for. */
  readonly description: string;
  /** The selectors that produce it, with `%s` standing for the element's own selector. */
  readonly matches: readonly string[];
  /**
   * What says this state is on **without using colour**.
   *
   * Required of any state whose colour indicator cannot clear 3 : 1 against the fill it is drawn
   * on. Two states in this file use it, and the browser gate asserts each one is actually
   * rendered — a declared cue that is not drawn fails louder than no declaration at all.
   */
  readonly nonColourCue?: string;
}

/** The spec a row's `paint` resolves to. */
export function paintSpecOf(entry: DerivedStateSpec): ControlStateSpec {
  return typeof entry.paint === 'string' ? controlStateSpecs[entry.paint] : entry.paint;
}

/** Emit the CSS for one derived table: `:where(selectors) { declarations }`, in cascade order. */
export function derivedStatesCss(
  table: Readonly<Record<string, DerivedStateSpec>>,
  cascade: readonly string[],
  selector: string,
): string {
  return cascade
    .map((state) => {
      const entry = table[state];
      if (entry === undefined) return '';
      const where = entry.matches.map((match) => match.replaceAll('%s', selector)).join(', ');
      return `:where(${where}) {\n${controlStateDeclarations(paintSpecOf(entry))}\n}`;
    })
    .filter((block) => block !== '')
    .join('\n');
}

// ── the field ────────────────────────────────────────────────────────────────

/** The seven states a text-entry surface can be in. */
export const fieldStateNames = [
  'rest',
  'hover',
  'editing',
  'invalid',
  'unavailable',
  'disabled',
  'focus',
] as const;

/** One of the seven. */
export type FieldState = (typeof fieldStateNames)[number];

/** A field that cannot be used never lights up, however the pointer is moved over it. */
const usableField = ':not([data-unavailable]):not([data-disabled])';

/**
 * The field table.
 *
 * ⚠ **`rest` declares a fill and every other row leaves it alone.** That is the whole of the
 * decision in this module's note: the fill is `--theme-surface` in all seven states, so secondary
 * text inside a field is on 5.23 : 1 (light) or 6.61 : 1 (dark) whatever the pointer is doing.
 */
export const fieldStates: Readonly<Record<FieldState, DerivedStateSpec>> = {
  rest: {
    paint: {
      description: '',
      background: 'surface',
      borderColor: 'border',
      borderStyle: 'solid',
      text: 'textPrimary',
      weight: 'medium',
      opacity: 1,
      matches: [],
    },
    because:
      'A control rests transparent; a field rests filled. A text box you cannot see the edge of ' +
      'is a text box nobody knows they may type in, and the fill is what keeps its placeholder ' +
      'legible under the pointer.',
    description: 'Resting. A filled box with a neutral edge, showing its value or its placeholder.',
    matches: ['%s'],
  },
  hover: {
    paint: {
      description: '',
      borderColor: 'textSecondary',
      matches: [],
    },
    because:
      'The control table’s hover fills with --theme-border-subtle, and secondary text is 4.32 : 1 ' +
      'on that. A field darkens its edge instead, and its fill never moves.',
    description: 'The pointer is over it. The edge darkens; the fill does not change.',
    matches: ['%s[data-state="hover"]', `%s:hover${usableField}`],
  },
  editing: {
    paint: {
      description: '',
      borderColor: 'accent',
      insetRing: 'accent',
      matches: [],
    },
    because:
      'The accent edge means *the keyboard is in here*, which no row of the control table means. ' +
      'It is a state of a surface rather than of a command.',
    description:
      'The keyboard is inside it, or its list is open. An accent edge and an accent inset ring — ' +
      'the ring is what makes the edge read as two pixels rather than one at a glance.',
    matches: ['%s[data-editing]', `%s:focus-within${usableField}`],
  },
  invalid: {
    paint: {
      description: '',
      borderColor: 'secondaryAccent',
      matches: [],
    },
    because:
      'The honey half of the palette is the disagreement colour, exactly as it is for a mixed ' +
      'toggle — but a field is not a toggle and none of the toggle’s rows carry a fill a field ' +
      'may take.',
    // ⚠ 2.07 : 1 in light. Measured, not assumed: `--theme-secondary-accent` on
    // `--theme-surface` cannot carry this state on its own, and pretending otherwise would be a
    // colour-only error indication.
    nonColourCue:
      'aria-invalid="true", a warning glyph inside the field, and the parse failure written out ' +
      'beneath it in primary text.',
    description:
      'What was typed could not be read. **Nothing was committed and nothing was reverted** — the ' +
      'text a person typed is still theirs.',
    matches: ['%s[data-invalid]'],
  },
  unavailable: {
    paint: 'unavailable',
    description:
      'Unavailable and explained. Still focusable, still announced, still carrying its reason.',
    nonColourCue: 'a dashed edge, aria-disabled="true", and the explanation it is described by.',
    matches: ['%s[data-unavailable]'],
  },
  disabled: {
    paint: 'disabled',
    description: 'Unavailable and not explained: out of the tab order entirely, and dimmed.',
    nonColourCue: 'removal from the tab order — the platform’s own disabled semantics.',
    matches: ['%s[data-disabled]'],
  },
  focus: {
    paint: {
      description: '',
      ring: true,
      matches: [],
    },
    because:
      'The foundations own the ring. This row paints nothing and exists so the matrix and the ' +
      'gates can name the state, exactly as `controlStateSpecs.focus` does.',
    description: 'Reached by keyboard. The foundations’ single focus treatment, and nothing else.',
    matches: [],
  },
};

/**
 * The cascade, in emission order — **and therefore in winning order**, every rule being (0,0,0).
 *
 * `invalid` after `editing` so a field being typed into while invalid still reads invalid, and the
 * two unavailable rows last for the reason `controlStateCascade` gives.
 */
export const fieldStateCascade: readonly FieldState[] = [
  'rest',
  'hover',
  'editing',
  'invalid',
  'unavailable',
  'disabled',
];

/** The composed paint of one field state. */
export function fieldPaint(state: FieldState): EffectiveStatePaint {
  return composeStatePaint(paintSpecOf(fieldStates[state]), paintSpecOf(fieldStates.rest));
}

/** A field state's paint through the generated tokens, for the pairwise gate. */
export function resolvedFieldFingerprint(state: FieldState, scheme: ColorScheme): string {
  return resolvePaintFingerprint(fieldPaint(state), scheme);
}

/** The state rules for one field selector. */
export function fieldStatesCss(selector: string): string {
  return derivedStatesCss(fieldStates, fieldStateCascade, selector);
}

// ── the checkbox's box ───────────────────────────────────────────────────────

/** The four states the little square takes. */
export const boxStateNames = ['rest', 'hover', 'checked', 'mixed'] as const;

/** One of the four. */
export type BoxState = (typeof boxStateNames)[number];

const usableRow = ':not(:disabled):not([aria-disabled="true"])';

/**
 * The box table.
 *
 * **There is no `checkedHover`**, and its absence is the decision. The *row* hovers — it is a real
 * `<button>` painted by `controlStatesCss` like every other command — and a box that also changed
 * under the pointer would be two hover treatments for one hover. Office's checkbox does the same
 * thing: the row lights and the box holds still.
 */
export const boxStates: Readonly<Record<BoxState, DerivedStateSpec>> = {
  rest: {
    paint: {
      description: '',
      background: 'surface',
      borderColor: 'border',
      borderStyle: 'solid',
      text: 'textPrimary',
      weight: 'medium',
      opacity: 1,
      matches: [],
    },
    because:
      'The control table rests transparent, and an unchecked checkbox that shows nothing is a ' +
      'checkbox nobody can find. This is the one paint the box owns.',
    description: 'Unchecked. An empty square with a neutral edge.',
    matches: ['%s'],
  },
  hover: {
    paint: {
      description: '',
      background: 'borderSubtle',
      borderColor: 'textSecondary',
      matches: [],
    },
    because:
      'The control table’s hover borders with the same colour it fills with, which on a square ' +
      'this small erases the edge exactly when the pointer is asking where it is.',
    description: 'The pointer is over the row. The square fills faintly and keeps its edge.',
    matches: [`.control:hover${usableRow} %s`, '.control[data-state="hover"] %s'],
  },
  checked: {
    paint: 'on',
    description:
      'On. The accent tint of a pressed toggle, because that is what this is, with a check mark ' +
      'in primary text — 10.58 : 1 in light, 12.12 : 1 in dark.',
    matches: ['.control[data-pressed="true"] %s'],
  },
  mixed: {
    paint: 'mixed',
    description:
      'Indeterminate: the selection disagrees with itself. The honey half of the palette and a ' +
      'dash rather than a check, because a dash is not a faint tick.',
    matches: ['.control[data-pressed="mixed"] %s'],
  },
};

/** The cascade, in emission order. */
export const boxStateCascade: readonly BoxState[] = ['rest', 'hover', 'checked', 'mixed'];

/** The composed paint of one box state. */
export function boxPaint(state: BoxState): EffectiveStatePaint {
  return composeStatePaint(paintSpecOf(boxStates[state]), paintSpecOf(boxStates.rest));
}

/** A box state's paint through the generated tokens. */
export function resolvedBoxFingerprint(state: BoxState, scheme: ColorScheme): string {
  return resolvePaintFingerprint(boxPaint(state), scheme);
}

/** The state rules for the box selector. */
export function boxStatesCss(selector: string): string {
  return derivedStatesCss(boxStates, boxStateCascade, selector);
}

/** The glyph each position draws. `rest` draws nothing at all. */
export const boxGlyphs: Readonly<Record<BoxState, { name: string; size: 16 } | undefined>> = {
  rest: undefined,
  hover: undefined,
  checked: { name: 'checkmark', size: 16 },
  // A dash rather than a smaller tick: `mixed` must not read as a weaker `checked`, which is the
  // same argument `controlStateSpecs.mixed` makes about the fill.
  mixed: { name: 'subtract', size: 16 },
};

// ── a listbox option ─────────────────────────────────────────────────────────

/**
 * The five states an option takes.
 *
 * ⚠ **There is no `focus`.** Focus never reaches an option: it stays on the combo box element and
 * `aria-activedescendant` says which option the keyboard is on — which is the ARIA listbox pattern
 * and is what gives the whole control **one** tab stop. So the keyboard cursor cannot be the
 * foundations’ focus ring, and `active` below is what draws it instead. A component that had left
 * this to `:focus-visible` would have a keyboard cursor that never appeared.
 */
export const optionStateNames = [
  'rest',
  'active',
  'selected',
  'selectedActive',
  'unavailable',
] as const;

/** One of the five. */
export type OptionState = (typeof optionStateNames)[number];

export const optionStates: Readonly<Record<OptionState, DerivedStateSpec>> = {
  rest: {
    paint: 'rest',
    description: 'An option in the list, neither chosen nor under the keyboard.',
    matches: ['%s'],
  },
  active: {
    paint: {
      description: '',
      background: 'borderSubtle',
      borderColor: 'border',
      insetRing: 'accentPressed',
      matches: [],
    },
    because:
      'This is the keyboard cursor and there is no focus ring to be it — focus is on the combo ' +
      'box, not on the option. The control table’s hover would be a 1.21 : 1 fill difference and ' +
      'nothing else, which is not an indicator. The accentPressed ring is 4.41 : 1 in light and ' +
      '3.97 : 1 in dark on the fill it is drawn on.',
    description:
      'The keyboard is on it — what aria-activedescendant points at. Also what the pointer hovers.',
    matches: ['%s[data-active]', `%s:hover${usableRow}`],
  },
  selected: {
    paint: 'on',
    description: 'The chosen option. The accent tint and a bold label, exactly as a pressed toggle.',
    matches: ['%s[aria-selected="true"]'],
  },
  selectedActive: {
    paint: {
      description: '',
      background: 'accentSurface',
      borderColor: 'accent',
      text: 'textPrimary',
      weight: 'bold',
      insetRing: 'accentPressed',
      matches: [],
    },
    because:
      'The control table’s onHover rings with --theme-accent, which is 2.98 : 1 on ' +
      '--theme-accent-surface in light. accentPressed is 4.68 : 1 on the same fill. Chosen *and* ' +
      'under the keyboard is the one combination where an option carries two indicators at once, ' +
      'so it is the one where the weaker ring would actually be missed.',
    description: 'The chosen option, with the keyboard on it. The ring the cursor always draws.',
    matches: ['%s[aria-selected="true"][data-active]', `%s[aria-selected="true"]:hover${usableRow}`],
  },
  unavailable: {
    paint: 'unavailable',
    description:
      'An option that exists and cannot be chosen — a font the document has but the printer does ' +
      'not. Still reached by the arrow keys, still announced, still refused.',
    nonColourCue: 'a dashed edge, aria-disabled="true", and the reason it cannot be chosen.',
    matches: ['%s[aria-disabled="true"]'],
  },
};

/** The cascade, in emission order. `unavailable` last, so an unavailable choice still looks chosen. */
export const optionStateCascade: readonly OptionState[] = [
  'rest',
  'active',
  'selected',
  'selectedActive',
  'unavailable',
];

/** The composed paint of one option state. */
export function optionPaint(state: OptionState): EffectiveStatePaint {
  return composeStatePaint(paintSpecOf(optionStates[state]), paintSpecOf(optionStates.rest));
}

/** An option state's paint through the generated tokens. */
export function resolvedOptionFingerprint(state: OptionState, scheme: ColorScheme): string {
  return resolvePaintFingerprint(optionPaint(state), scheme);
}

/** The state rules for one option selector. */
export function optionStatesCss(selector: string): string {
  return derivedStatesCss(optionStates, optionStateCascade, selector);
}

/**
 * **An option's second line is told apart by size, not by colour.**
 *
 * U05 established it for a menu's hint and the reason applies here twice over: an option's fill
 * *does* change under the keyboard cursor, so any colour reserved for "less important" would have
 * to clear 4.5 : 1 against two different fills. The second line is therefore primary text at the
 * `dense` size. `tests/browser/inputs.spec.ts` asserts no part of an option computes
 * `--theme-text-secondary`, which is the assertion that fires if somebody helpfully greys it.
 */
export const optionSecondaryTextIsSizeNotColour = true;

// ── the slider's parts ───────────────────────────────────────────────────────

/**
 * What a slider is painted with, and the measurement behind every choice.
 *
 * The filled track is `accentPressed` rather than `accent`, and that is not a preference: **accent
 * on borderSubtle is 2.81 : 1 in light**, so the boundary between the filled and unfilled halves
 * of the track — which is the entire visual output of a slider — would be below the non-text
 * minimum. accentPressed on borderSubtle is 4.41 : 1 in light and 3.97 : 1 in dark.
 *
 * `tests/inputs.test.ts` asserts all four ratios rather than trusting this paragraph, and asserts
 * the rejected pairing fails, so the comment cannot quietly become false.
 */
export const sliderPaints = {
  /** The unfilled track. */
  rail: 'borderSubtle',
  /** The filled track, from the minimum to the thumb. */
  fill: 'accentPressed',
  /** The thumb's fill — a light knob, so the filled track reads *through* it rather than into it. */
  thumb: 'surface',
  /** The thumb's edge. */
  thumbBorder: 'accentPressed',
  /** A tick mark on the rail. */
  tick: 'textSecondary',
  /** A tick's label. On the page's own surface, never on a fill — see the module note. */
  tickLabel: 'textSecondary',
} as const satisfies Readonly<Record<string, ThemeMember>>;

/** The surface a slider's tick labels are read against. The gate measures against exactly this. */
export const sliderLabelBackground: ThemeMember = 'surface';

/** Which slider paints are *indicators* and must clear 3 : 1, and against what. */
export const sliderIndicatorPairs: readonly (readonly [ThemeMember, ThemeMember])[] = [
  [sliderPaints.fill, sliderPaints.rail],
  [sliderPaints.thumbBorder, sliderPaints.thumb],
  [sliderPaints.tick, sliderLabelBackground],
];

// ── slider arithmetic ────────────────────────────────────────────────────────

/** How a slider is laid out. Vertical exists because a zoom control is one. */
export const sliderOrientations = ['horizontal', 'vertical'] as const;

/** One of the two. */
export type SliderOrientation = (typeof sliderOrientations)[number];

/** Whether a value names one of the two. */
export function isSliderOrientation(value: unknown): value is SliderOrientation {
  return (sliderOrientations as readonly string[]).includes(String(value));
}

/**
 * How many decimal places a number is written with, from its own decimal representation.
 *
 * The whole of the float problem in one place: `0 + 3 * 0.1` is `0.30000000000000004`, and a
 * slider that reported that has told a person their line spacing is thirty quadrillionths off.
 * Rounding to the number of places the *step* has is the only rounding that is guaranteed not to
 * lose information the step could express.
 */
export function decimalPlaces(value: number): number {
  if (!Number.isFinite(value)) return 0;
  const text = String(value);
  const exponent = text.indexOf('e-');
  if (exponent >= 0) {
    const digits = text.slice(0, exponent).split('.')[1] ?? '';
    return digits.length + Number.parseInt(text.slice(exponent + 2), 10);
  }
  return (text.split('.')[1] ?? '').length;
}

/** Round to a fixed number of decimal places, without going through exponential notation. */
export function roundTo(value: number, places: number): number {
  const scale = 10 ** Math.min(Math.max(places, 0), 15);
  const sign = value < 0 ? -1 : 1;
  return (sign * Math.round(Math.abs(value) * scale)) / scale;
}

/** A slider's range and granularity. */
export interface SliderRange {
  readonly min: number;
  readonly max: number;
  readonly step: number;
}

/**
 * Bring a value onto a stop, and inside the range.
 *
 * ⚠ **The stops are the step boundaries *plus the maximum*, and that is the decision.** Boundaries
 * are counted from `min`, so a range of 0…10 with a step of 3 has boundaries at 0, 3, 6 and 9 —
 * and 10 is not one of them. Snapping to the nearest boundary and then clamping would make the
 * maximum **unreachable**: `End` would land on 9 on a slider whose own label says the top is ten,
 * and dragging the thumb to the very end of the track would leave it a step short.
 *
 * So the maximum is a stop of its own, and a value nearer to it than to any boundary snaps to it.
 * `tests/inputs.test.ts` asserts both directions over a range whose maximum deliberately is not on
 * a boundary — that 9.4 snaps down to 9 and 9.6 snaps up to 10 — because "the maximum is
 * reachable" is satisfied by a function that snaps *everything* to the maximum, and only the pair
 * of assertions rules that out.
 */
export function snapToStep(value: number, range: SliderRange): number {
  const { min, max, step } = range;
  if (!Number.isFinite(value)) return min;
  if (!(step > 0)) return Math.min(Math.max(value, min), max);
  const places = Math.max(decimalPlaces(step), decimalPlaces(min));
  const snapped = roundTo(min + Math.round((value - min) / step) * step, places);
  const candidate = Math.min(Math.max(snapped, min), max);
  if (candidate !== max && Math.abs(value - max) < Math.abs(value - candidate)) return max;
  return candidate;
}

/**
 * Where the thumb sits, as a fraction of the track. `0` at the minimum, `1` at the maximum.
 *
 * A degenerate range returns `0` rather than dividing by zero — a slider whose minimum equals its
 * maximum has one position and it is the start of the track.
 */
export function sliderFraction(value: number, min: number, max: number): number {
  if (!(max > min)) return 0;
  const clamped = Math.min(Math.max(value, min), max);
  return (clamped - min) / (max - min);
}

/** The reverse: the value a fraction of the track means, snapped. What a click on the rail does. */
export function valueAtFraction(fraction: number, range: SliderRange): number {
  const clamped = Math.min(Math.max(fraction, 0), 1);
  return snapToStep(range.min + clamped * (range.max - range.min), range);
}

/**
 * How far Page Up and Page Down move, when nothing says.
 *
 * A tenth of the range, rounded onto a step boundary, and never smaller than one step — otherwise
 * a slider with a coarse step would have a page key that did nothing, which is worse than not
 * binding the key.
 */
export function defaultPageStep(range: SliderRange): number {
  const span = range.max - range.min;
  if (!(span > 0) || !(range.step > 0)) return Math.max(range.step, 0);
  const tenth = span / 10;
  const steps = Math.max(1, Math.round(tenth / range.step));
  return roundTo(steps * range.step, decimalPlaces(range.step));
}

/** Everything a key press can ask a slider to do. */
export const sliderActions = [
  'increment',
  'decrement',
  'incrementPage',
  'decrementPage',
  'minimum',
  'maximum',
] as const;

/** One of the six. */
export type SliderAction = (typeof sliderActions)[number];

/**
 * The key map. **Data, so a Node test can assert the RTL mirror without a browser.**
 *
 * ARIA's slider pattern mirrors the *inline* arrows under RTL and never mirrors the block ones: on
 * a right-to-left horizontal slider the track runs the other way, so Arrow Left is the direction
 * of increase, while Arrow Up is up in every writing system. A vertical slider therefore does not
 * mirror at all, which is the case that gets forgotten.
 */
export function sliderKeyAction(
  key: string,
  direction: 'ltr' | 'rtl',
  orientation: SliderOrientation,
): SliderAction | undefined {
  const mirrored = orientation === 'horizontal' && direction === 'rtl';
  switch (key) {
    case 'ArrowRight':
      return mirrored ? 'decrement' : 'increment';
    case 'ArrowLeft':
      return mirrored ? 'increment' : 'decrement';
    case 'ArrowUp':
      return 'increment';
    case 'ArrowDown':
      return 'decrement';
    case 'PageUp':
      return 'incrementPage';
    case 'PageDown':
      return 'decrementPage';
    case 'Home':
      return 'minimum';
    case 'End':
      return 'maximum';
    default:
      return undefined;
  }
}

/** Apply one action to a value. Pure, so the browser gate has an expected number to compare with. */
export function applySliderAction(
  action: SliderAction,
  value: number,
  range: SliderRange,
  pageStep: number,
): number {
  switch (action) {
    case 'increment':
      return snapToStep(value + range.step, range);
    case 'decrement':
      return snapToStep(value - range.step, range);
    case 'incrementPage':
      return snapToStep(value + pageStep, range);
    case 'decrementPage':
      return snapToStep(value - pageStep, range);
    case 'minimum':
      return range.min;
    case 'maximum':
      // Not snapped: `End` means *the top*, and the top is the maximum whether or not it happens
      // to sit on a step boundary. See `snapToStep`.
      return range.max;
  }
}

// ── the list ─────────────────────────────────────────────────────────────────

/** One option, as data. The three list controls read this shape and never an element's identity. */
export interface OptionDescriptor {
  readonly value: string;
  readonly label: string;
  /** A second line — a preview, a size, a note. Primary text at the dense size, never grey. */
  readonly description?: string;
  /** Exists and cannot be chosen. Still arrow-reachable, still announced. */
  readonly unavailable?: boolean;
  /** Why it cannot be chosen. Announced, and shown in the row. */
  readonly explanation?: string;
  /** The section it belongs to. Options with none sit above the first heading. */
  readonly category?: string;
}

/** How a combo box decides which options a query keeps. */
export const filterModes = ['startsWith', 'contains'] as const;

/** One of the two. */
export type FilterMode = (typeof filterModes)[number];

/** Whether a value names one of the two. */
export function isFilterMode(value: unknown): value is FilterMode {
  return (filterModes as readonly string[]).includes(String(value));
}

/** Lower-case, trimmed, and with runs of whitespace collapsed. */
export function normaliseForMatch(text: string): string {
  return text.trim().toLowerCase().replace(/\s+/g, ' ');
}

/**
 * The options a query keeps, **in the original order**.
 *
 * An empty query keeps everything, which is what makes "clear the field and every font comes back"
 * true rather than a special case in the component.
 */
export function filterOptions(
  options: readonly OptionDescriptor[],
  query: string,
  mode: FilterMode,
): OptionDescriptor[] {
  const wanted = normaliseForMatch(query);
  if (wanted === '') return [...options];
  return options.filter((option) => {
    const label = normaliseForMatch(option.label);
    return mode === 'contains' ? label.includes(wanted) : label.startsWith(wanted);
  });
}

/** The option a value names, or `undefined`. */
export function optionByValue(
  options: readonly OptionDescriptor[],
  value: string,
): OptionDescriptor | undefined {
  return options.find((option) => option.value === value);
}

/**
 * The text a value is displayed as.
 *
 * **This is the function that keeps MJXOFF-186's headline trap shut.** The value the field shows
 * must be the value the control reports, so exactly one function turns one into the other and both
 * the component and the gate call it. A field that rendered its text one way and reported its
 * value another would be right in every screenshot and wrong in every use.
 */
export function displayTextFor(
  options: readonly OptionDescriptor[],
  value: string,
  allowCustom: boolean,
): string {
  const found = optionByValue(options, value);
  if (found !== undefined) return found.label;
  return allowCustom ? value : '';
}

/** Everything a key press can ask a list control to do. */
export const listboxActions = [
  'open',
  'close',
  'commit',
  'revert',
  'next',
  'previous',
  'first',
  'last',
  'pageNext',
  'pagePrevious',
  'typeahead',
  'leave',
] as const;

/** One of the twelve. */
export type ListboxAction = (typeof listboxActions)[number];

/** What the key map needs to know about the control the key arrived at. */
export interface ListboxKeyContext {
  /** Whether the list is showing. */
  readonly open: boolean;
  /**
   * Whether the field is a text box a person types into.
   *
   * ⚠ **This is the flag that decides Home and End**, and it is the difference that gets missed. In
   * a select-only dropdown they jump to the first and last option; in an editable combo box they
   * are *caret keys* and belong to the text box, so the list must not steal them. A combo box that
   * jumped to the last font when a person pressed End to get to the end of what they had typed is
   * the exact shape of that bug.
   */
  readonly editable: boolean;
}

/**
 * The key map. Data, so `tests/inputs.test.ts` can assert every cell of it in milliseconds.
 *
 * `Tab` returns `leave` and the component does **not** call `preventDefault` on it: the browser's
 * own sequential navigation continues from the field, which is where a person expects `Tab` out of
 * a combo box to leave them. That, and the single tab stop, is what makes this a disclosure rather
 * than a trap — see `inputFocusPatterns`.
 */
export function listboxKeyAction(
  key: string,
  context: ListboxKeyContext,
): ListboxAction | undefined {
  const { open, editable } = context;
  switch (key) {
    case 'ArrowDown':
      return open ? 'next' : 'open';
    case 'ArrowUp':
      return open ? 'previous' : 'open';
    case 'PageDown':
      return open ? 'pageNext' : undefined;
    case 'PageUp':
      return open ? 'pagePrevious' : undefined;
    case 'Home':
      return editable ? undefined : 'first';
    case 'End':
      return editable ? undefined : 'last';
    case 'Enter':
      return 'commit';
    case 'Escape':
      return open ? 'close' : 'revert';
    case 'Tab':
      return 'leave';
    case ' ':
      // A space in a text box is a space. In a select-only dropdown it is what opens the list,
      // which is the native <select> behaviour and the one a person's fingers already know.
      return editable ? undefined : open ? 'commit' : 'open';
    default:
      if (editable) return undefined;
      return key.length === 1 && key !== ' ' ? 'typeahead' : undefined;
  }
}

/** How far Page Up and Page Down move through a list. Rows, not pixels. */
export const listboxPageRows = 10;

/** How many options a list shows before it scrolls, in rows. */
export const listboxVisibleRows = 8;

/** Where the next index lands, with wrapping at both ends. */
export function nextOptionIndex(action: ListboxAction, current: number, count: number): number {
  if (count <= 0) return -1;
  const wrap = (index: number): number => ((index % count) + count) % count;
  switch (action) {
    case 'next':
      return wrap(current + 1);
    case 'previous':
      return wrap(current - 1);
    case 'first':
      return 0;
    case 'last':
      return count - 1;
    case 'pageNext':
      return Math.min(count - 1, current + listboxPageRows);
    case 'pagePrevious':
      return Math.max(0, current - listboxPageRows);
    default:
      return current;
  }
}

/**
 * The first option whose label starts with what has been typed, from just after `from`.
 *
 * The same contract as `nextTypeaheadIndex` in the menu model and deliberately a separate
 * function, because a listbox's type-ahead searches the **filtered** list and a menu's searches
 * all of it. Returns `-1` when nothing matches, so a mistyped letter leaves the cursor alone
 * rather than sending it to the top.
 */
export function typeaheadIndex(
  labels: readonly string[],
  typed: string,
  from: number,
): number {
  const wanted = normaliseForMatch(typed);
  if (wanted === '' || labels.length === 0) return -1;
  for (let offset = 1; offset <= labels.length; offset += 1) {
    const index = (from + offset) % labels.length;
    if (normaliseForMatch(labels[index] ?? '').startsWith(wanted)) return index;
  }
  return -1;
}

/** How long a type-ahead run stays open. The menu's number, because it is the same finger. */
export { typeaheadResetDelay } from '../menus/menu-model.ts';

// ── the focus doctrine ───────────────────────────────────────────────────────

/**
 * **The number of tab stops decides trap-versus-disclosure**, and U05 wrote the rule.
 *
 * Every control in this child holds exactly **one** tab stop, so `Tab` is free to mean *leave* in
 * all seven, and every surface that opens closes when focus leaves it. Two mechanisms produce that
 * one stop and the difference is worth naming, because the ARIA patterns are different:
 *
 * * **`activeDescendant`** — focus never moves off the field; `aria-activedescendant` names the
 *   option the keyboard is on. The two list controls use it, which is the ARIA combobox pattern.
 * * **`roving`** — focus moves between children and exactly one of them has `tabindex="0"`. The
 *   segmented control uses it, which is the ARIA radio-group pattern.
 * * **`single`** — the control is one focusable element and there is nothing to move between.
 *
 * `tests/browser/inputs.spec.ts` counts the tab stops of every control by pressing `Tab` through
 * a real browser and asserts this table, which is the only way the claim means anything.
 */
export const inputFocusPatterns = {
  activeDescendant:
    'Focus stays on the field. aria-activedescendant names the option the keyboard is on, and the ' +
    'list is a disclosure: Tab leaves, and leaving closes it.',
  roving:
    'Exactly one child holds tabindex="0" and the arrows move it. Tab enters the group once and ' +
    'leaves it once.',
  single: 'One focusable element and nothing to move between.',
} as const;

/** One of the three. */
export type InputFocusPattern = keyof typeof inputFocusPatterns;

/** Which pattern each control uses, and therefore how many tab stops it has. Which is one. */
export const inputFocusPattern: Readonly<Record<keyof typeof inputTags, InputFocusPattern>> = {
  label: 'single',
  checkbox: 'single',
  dropdown: 'activeDescendant',
  comboBox: 'activeDescendant',
  option: 'single',
  measureInput: 'single',
  slider: 'single',
  segmentedControl: 'roving',
  segment: 'single',
};

/** The tab stops a control contributes. One, in every case, and the gate counts them. */
export const tabStopsPerControl = 1;

/** Why a list control closes. `blur` does not restore focus; the others do. */
export const listCloseReasons = ['escape', 'commit', 'outside', 'blur', 'tab'] as const;

/** One of the five. */
export type ListCloseReason = (typeof listCloseReasons)[number];

/** The reasons that put focus back on the field. */
export const focusRestoringListCloseReasons: readonly ListCloseReason[] = [
  'escape',
  'commit',
  'outside',
];

// ── the boxes ────────────────────────────────────────────────────────────────

/** The custom properties this child's boxes read. Layout only; no paint goes through one. */
export const inputBoxProperties = {
  /** The inline size a field is laid out at. A container sets it; the field never measures. */
  fieldInlineSize: '--mjx-field-inline-size',
  /** The list's own width, so it can be wider than the field that opened it. */
  listInlineSize: '--mjx-list-inline-size',
  /** The list's height cap, written by the placement. */
  listMaxBlockSize: '--mjx-list-max-block-size',
  /** The height of one option row, which the virtualiser needs and CSS supplies. */
  optionBlockSize: '--mjx-option-block-size',
  /** How far along its track a slider's thumb is, as a number between 0 and 1. */
  sliderFraction: '--mjx-slider-fraction',
  /** The thumb's diameter. */
  sliderThumbSize: '--mjx-slider-thumb-size',
  /** The track's thickness. */
  sliderTrackSize: '--mjx-slider-track-size',
} as const;

/** The narrowest a field is laid out, in spacing units. Below this a value is unreadable. */
export const fieldMinimumInlineUnits = 28;

/** The narrowest a popup list is, in spacing units. It is never narrower than the field. */
export const listMinimumInlineUnits = 40;

/** The checkbox's square and the slider's thumb, in spacing units. */
export const boxSideUnits = 4;
export const sliderThumbUnits = 4;
export const sliderTrackUnits = 1;

/** How much of the boundary a list may cover before it scrolls. The menu's fraction. */
export { sheetBoundaryFraction as listBoundaryFraction } from '../menus/menu-model.ts';

// ── the stylesheets ──────────────────────────────────────────────────────────

const overlay = surfaceLevels.overlay;

/**
 * The shared box rules.
 *
 * ⚠ **Nothing here paints.** No background, no border colour or style, no text colour, no weight,
 * no opacity, no shadow — every one of those comes from a `:where()` state rule at (0,0,0), and a
 * single `background:` in this block would score (0,1,0) and out-specify the whole table. That is
 * MJXOFF-181's specificity accident, which MJXOFF-183 actually shipped, and the separation below
 * is what prevents it. `tests/inputs.test.ts` asserts it rather than trusting this comment.
 */
export const inputBaseCss = `
  :host {
    display: inline-block;
    vertical-align: middle;
  }
  :host([hidden]) { display: none; }

  .field {
    display: flex;
    align-items: center;
    gap: var(${densityProperties.step});
    box-sizing: border-box;
    inline-size: var(${inputBoxProperties.fieldInlineSize}, 100%);
    min-inline-size: ${spacingMultiple(fieldMinimumInlineUnits)};
    padding-inline: var(${densityProperties.gutter});
    padding-block: 0;
    border-width: 1px;
    border-radius: ${radiusVariable('control')};
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  /*
   * ⚠ align-self: stretch, and it is a HIT TARGET fix rather than a layout preference
   * (MJXOFF-194). The field wears .mjx-hit-target, so the BOX is 40px tall in comfortable density
   * -- but the entry is the focusable element and the thing a press actually lands on, and with the
   * field's align-items: center it was 22px tall with nine dead pixels above and below it inside a
   * control that looked like a full-height field. The catalogue-wide touch sweep found it at phone
   * width on the dropdown, the combo box, the measure input and both pickers at once, which is
   * exactly the class of defect it exists for: correct in every screenshot, and a third of the
   * control unpressable in the hand. Stretching the entry makes the field's whole height the
   * target and changes nothing about where the text sits.
   */
  .entry {
    flex: 1 1 auto;
    align-self: stretch;
    min-inline-size: 0;
    margin: 0;
    padding: 0;
    border: 0;
    background: none;
    color: inherit;
    font: inherit;
    text-overflow: ellipsis;
    appearance: none;
    -webkit-appearance: none;
  }

  /* The one place a placeholder is styled, and the reason the field never changes fill. */
  .entry::placeholder {
    color: ${themeVariable('textSecondary')};
    opacity: 1;
  }

  .entry:focus { outline: none; }

  .affix {
    flex: 0 0 auto;
    color: ${themeVariable('textSecondary')};
    white-space: nowrap;
  }

  .trailing {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    color: inherit;
  }

  .message {
    display: block;
    margin-block-start: var(${densityProperties.step});
    color: ${themeVariable('textPrimary')};
  }

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

  /* ⚠ Restated LAST, and MJXOFF-274 found out what it costs to leave out.
   *
   * The user agent's own hidden rule lives in the user-agent origin, so any author rule at all
   * beats it — and the trailing and message rules above are both author rules. Without this line a
   * measure input drew its invalid warning glyph and its message box with the attribute set, the
   * accessibility tree correct and every attribute assertion passing: a state indicator that is
   * always on, which is the same as no indicator at all. MJXOFF-189 documented the trap and
   * MJXOFF-190 and MJXOFF-191 restated the line; this sheet predates all three, and the defect only
   * became visible when a field sat inside an assembled task pane instead of in a matrix of four.
   *
   * Important rather than source order alone, exactly as navigator-sheets.ts does: this sheet is
   * joined with others after it. */
  [hidden] { display: none !important; }
`;

/** `<mjx-label>`'s rules. */
export const labelCss = `
  :host { display: block; }
  :host([hidden]) { display: none; }

  .label {
    display: flex;
    align-items: baseline;
    gap: var(${densityProperties.step});
    margin-block-end: var(${densityProperties.step});
    color: ${themeVariable('textPrimary')};
    cursor: default;
  }

  :host([for]) .label { cursor: pointer; }

  /*
   * The required marker is a glyph *and* a word, never a bare asterisk.
   *
   * An asterisk alone is a convention a screen reader reads as "star" and a person who has not met
   * the convention reads as nothing. The visible mark is the asterisk; the announced text is the
   * word, off-screen beside it.
   */
  .required {
    color: ${themeVariable('secondaryAccent')};
  }

  .hint {
    display: block;
    color: ${themeVariable('textSecondary')};
  }

  /* Restated last, for the reason the base sheet gives at length: a label hides its required mark
   * and its hint with the attribute, and the hint rule above is an author rule that beat it. */
  [hidden] { display: none !important; }
`;

/** The popup list's rules. The overlay rung, exactly as a menu — one card, not two. */
export const listCss = `
  .list {
    box-sizing: border-box;
    margin: 0;
    padding-block: var(${densityProperties.step});
    padding-inline: 0;
    position: fixed;
    inset: auto;
    top: var(${floatingProperties.y});
    left: var(${floatingProperties.x});
    inline-size: var(${inputBoxProperties.listInlineSize}, max-content);
    min-inline-size: ${spacingMultiple(listMinimumInlineUnits)};
    max-inline-size: var(${floatingProperties.maxInlineSize}, none);
    ${inputBoxProperties.optionBlockSize}: max(var(${densityProperties.hitTarget}), ${String(accessibleHitTargetMinimum)}px);
    max-block-size: min(
      var(${inputBoxProperties.listMaxBlockSize}, 100vh),
      var(${floatingProperties.maxBlockSize}, 100vh)
    );
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    overflow-x: clip;
    overscroll-behavior: contain;
    background: ${overlay.background};
    border: ${overlay.border};
    box-shadow: ${overlay.shadow};
    border-radius: ${radiusVariable(overlay.radius)};
    color: ${themeVariable('textPrimary')};
    z-index: 1;
  }

  .list[data-open='false'] { display: none; }

  /* The spacers a virtualised list is made of: the rows above and below the window, as height. */
  .spacer { flex: 0 0 auto; }

  .section {
    /* One row tall, and not shrinkable — see .option above. The virtualiser turns a scroll offset
     * into a row index by dividing, so a heading of its own height would put every row below it at
     * the wrong index, and a list would scroll to *nearly* the right option for ever. */
    flex: 0 0 auto;
    box-sizing: border-box;
    block-size: var(--mjx-option-block-size);
    display: block;
    align-content: center;
    padding-inline: var(${densityProperties.gutter});
    padding-block: var(${densityProperties.step});
    color: ${themeVariable('textSecondary')};
  }
`;

/**
 * One option row.
 *
 * ⚠ Paints nothing, for the reason `inputBaseCss` states. The `min-block-size` is here because the
 * virtualiser needs every row to be the same height and CSS is the only thing that can promise it.
 */
export const optionCss = `
  .option {
    /*
     * ⚠ **flex: 0 0 auto, and it is load-bearing.** The list is a flex column with a
     * max-block-size, so a row left at the default flex-shrink of 1 is squeezed to its content
     * height the moment the list is taller than its cap — and the block-size below is quietly
     * ignored. Measured: rows came out at 21.25 pixels instead of 40, a heading was a different
     * height again, and the virtualiser's divide-by-row-height was arithmetic over a number
     * nothing on screen had. tests/browser/inputs.spec.ts asserts every row is the same height
     * and that a heading is exactly one of them, which is what found it.
     */
    flex: 0 0 auto;
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    column-gap: var(${densityProperties.step});
    box-sizing: border-box;
    inline-size: 100%;
    /*
     * Every row is the **same height**, and that is a requirement rather than a look: the
     * virtualiser turns a scroll offset into a row index by dividing by this, and a list of rows
     * that each sized to their own content would make that division a guess. It is the density
     * mode's hit target, so a compact list is denser and still clears the 24px floor.
     */
    block-size: var(${inputBoxProperties.optionBlockSize});
    padding-inline: var(${densityProperties.gutter});
    padding-block: 0;
    border-width: 1px;
    border-radius: ${radiusVariable('chip')};
    cursor: pointer;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .option[aria-disabled='true'] { cursor: help; }

  .option-label {
    grid-column: 2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Primary text at the dense size. Told apart by size, never by colour — see the model note. */
  .option-description {
    grid-column: 2;
    color: inherit;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .option-mark {
    grid-column: 1;
    grid-row: 1 / -1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: ${spacingMultiple(boxSideUnits)};
  }

  .option-trailing {
    grid-column: 3;
    grid-row: 1 / -1;
    display: inline-flex;
    align-items: center;
  }
`;

/** `<mjx-checkbox>`'s own rules, over `controlBaseCss`. */
export const checkboxCss = `
  .control {
    justify-content: flex-start;
    inline-size: 100%;
    text-align: start;
  }

  .box {
    flex: 0 0 auto;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
    inline-size: ${spacingMultiple(boxSideUnits)};
    block-size: ${spacingMultiple(boxSideUnits)};
    border-width: 1px;
    border-radius: ${radiusVariable('chip')};
    transition-property: background, border-color, color, box-shadow, opacity;
  }
`;

/** `<mjx-slider>`'s rules. The track, the fill, the thumb and the ticks. */
export const sliderCss = `
  :host {
    display: block;
    ${inputBoxProperties.sliderThumbSize}: ${spacingMultiple(sliderThumbUnits)};
    ${inputBoxProperties.sliderTrackSize}: ${spacingMultiple(sliderTrackUnits)};
  }
  :host([hidden]) { display: none; }

  .slider {
    display: grid;
    gap: var(${densityProperties.step});
  }

  .track {
    position: relative;
    display: flex;
    align-items: center;
    box-sizing: border-box;
    min-block-size: max(var(${densityProperties.hitTarget}), var(${inputBoxProperties.sliderThumbSize}));
    padding-inline: calc(var(${inputBoxProperties.sliderThumbSize}) / 2);
    cursor: pointer;
    touch-action: none;
  }

  .track:focus { outline: none; }

  .rail {
    position: relative;
    inline-size: 100%;
    block-size: var(${inputBoxProperties.sliderTrackSize});
    border-radius: ${radiusVariable('chip')};
    background: ${themeVariable(sliderPaints.rail)};
  }

  .fill {
    position: absolute;
    inset-block: 0;
    inset-inline-start: 0;
    inline-size: calc(var(${inputBoxProperties.sliderFraction}, 0) * 100%);
    border-radius: inherit;
    background: ${themeVariable(sliderPaints.fill)};
  }

  .thumb {
    position: absolute;
    inset-block-start: 50%;
    inset-inline-start: calc(var(${inputBoxProperties.sliderFraction}, 0) * 100%);
    translate: -50% -50%;
    box-sizing: border-box;
    inline-size: var(${inputBoxProperties.sliderThumbSize});
    block-size: var(${inputBoxProperties.sliderThumbSize});
    border: 1px solid ${themeVariable(sliderPaints.thumbBorder)};
    border-radius: 50%;
    background: ${themeVariable(sliderPaints.thumb)};
    transition-property: translate, scale;
  }

  /*
   * RTL: the fill grows from the end of the line and the thumb travels the other way.
   * inset-inline-start does that on its own; translate is PHYSICAL and does not, so it is
   * mirrored here — the same trap menuBoxProperties records for a placement's coordinates.
   *
   * The direction comes from an attribute the component writes rather than from :dir(), because
   * :host(:dir(rtl)) is not matched by every engine this catalogue is meant to survive and a
   * silently unmatched selector would put the thumb half a thumb's width out with nothing failing.
   */
  :host([data-direction='rtl']) .thumb { translate: 50% -50%; }

  .ticks {
    position: relative;
    display: block;
    block-size: var(${inputBoxProperties.sliderTrackSize});
  }

  .tick {
    position: absolute;
    inline-size: 1px;
    block-size: 100%;
    background: ${themeVariable(sliderPaints.tick)};
  }

  .tick-labels {
    position: relative;
    display: block;
    color: ${themeVariable(sliderPaints.tickLabel)};
  }

  .tick-label {
    position: absolute;
    translate: -50% 0;
    white-space: nowrap;
  }

  :host([data-disabled]) .slider { opacity: ${String(disabledOpacity)}; }

  /* Restated last, for the reason the base sheet gives: a slider with no ticks hides its tick rail
   * and tick labels with the attribute, and both are author display rules that beat it. */
  [hidden] { display: none !important; }
`;

/** `<mjx-segmented-control>`'s rules. Every paint comes from the shared control table. */
export const segmentedCss = `
  :host { display: inline-block; }
  :host([hidden]) { display: none; }

  .group {
    display: inline-flex;
    gap: var(${densityProperties.step});
    box-sizing: border-box;
    padding: var(${densityProperties.step});
    border-radius: ${radiusVariable('control')};
    background: ${themeVariable('background')};
  }

  .segment {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(${densityProperties.step});
    box-sizing: border-box;
    margin: 0;
    padding-inline: var(${densityProperties.gutter});
    padding-block: 0;
    border-width: 1px;
    border-radius: ${radiusVariable('chip')};
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .segment:disabled { cursor: default; }
  .segment[aria-disabled='true'] { cursor: help; }
`;

/** The class an option row wears for its motion role. */
export const inputMotionClass = motionRoleClass('surfaceSettle');

/** The class a popup list wears when it enters. */
export const listMotionClass = motionRoleClass('panelEnter');

/** The type roles this child's parts use. */
export const inputTypeRoles = {
  fieldValue: 'control',
  optionLabel: 'control',
  optionDescription: 'dense',
  fieldLabel: 'label',
  hint: 'dense',
  message: 'dense',
  tickLabel: 'dense',
  sectionHeading: 'label',
} as const;

/** `optionLabel` → `mjx-type-control`. */
export function inputTypeClass(part: keyof typeof inputTypeRoles): string {
  return typeRoleClass(inputTypeRoles[part]);
}

/** The whole sheet a field-shaped component adopts. */
export const fieldSheet = [inputBaseCss, fieldStatesCss('.field')].join('\n');

/** The whole sheet a list-bearing component adopts, on top of `fieldSheet`. */
export const listSheet = [listCss, optionCss, optionStatesCss('.option')].join('\n');

/** The whole sheet `<mjx-checkbox>` adopts, on top of the shared control base. */
export const checkboxSheet = [checkboxCss, boxStatesCss('.box')].join('\n');

/**
 * The whole sheet `<mjx-segmented-control>` adopts — **and every paint in it is the shared
 * table's**, through the same `data-pressed` attribute a toggle button writes.
 *
 * Composed here rather than in the component because a module that defines a custom element cannot
 * be imported from Node, and this is the string `tests/inputs.test.ts` asserts contains the shared
 * rules verbatim.
 */
export const segmentedSheet = [segmentedCss, controlStatesCss('.segment')].join('\n');

// ── the catalogue contract ───────────────────────────────────────────────────

/** Where each control's stories live. Spelled again as literals in the story files — CSF is static. */
export const inputStoryTitles = {
  label: 'Inputs/Label',
  checkbox: 'Inputs/Checkbox',
  dropdown: 'Inputs/Dropdown',
  comboBox: 'Inputs/Combo Box',
  measureInput: 'Inputs/Measure Input',
  slider: 'Inputs/Slider',
  segmentedControl: 'Inputs/Segmented Control',
} as const;

/** The story every table publishes its whole states matrix under. */
export const inputStatesMatrixStoryName = 'The States Matrix';

/**
 * The attribute a matrix cell is found by. One per table, so a gate cannot sweep the wrong one.
 *
 * ⚠ **There is deliberately no `optionStateCellAttribute`.** An option lives inside a popup's
 * shadow root and cannot be wrapped by a story, so a hook here would be a hook nothing draws — and
 * a declared marker that is never rendered is U06's sixth defect in miniature. The option states
 * are asserted on **live rows instead**: all five are reachable by choosing one option, putting the
 * keyboard on another, and marking a third unavailable, and the gate computes the expected state
 * from each row's own `aria-selected`, `data-active` and `aria-disabled`. That is a stronger
 * assertion than a forced matrix, because the rows it measures are rows a person can produce.
 */
export const fieldStateCellAttribute = 'data-field-state-cell';
export const boxStateCellAttribute = 'data-box-state-cell';

/** The state a live option row is in, from the three facts the row itself carries. */
export function optionStateOf(
  row: { readonly selected: boolean; readonly active: boolean; readonly unavailable: boolean },
): OptionState {
  if (row.unavailable) return 'unavailable';
  if (row.selected && row.active) return 'selectedActive';
  if (row.selected) return 'selected';
  if (row.active) return 'active';
  return 'rest';
}

/** The attribute a story writes to force a state a pointer would otherwise have to produce. */
export const forcedFieldStateAttribute = 'force-field-state';

/** The states a story may force. Nothing else is forcible. */
export const forcibleFieldStates = ['hover', 'editing', 'invalid'] as const;

/** One of the three. */
export type ForcibleFieldState = (typeof forcibleFieldStates)[number];

/** Whether a value names one of the three. */
export function isForcibleFieldState(value: unknown): value is ForcibleFieldState {
  return (forcibleFieldStates as readonly string[]).includes(String(value));
}

/** `accentSurface` → `--theme-accent-surface`. Re-exported so a gate spells it once. */
export { customPropertyCase, paintValue };
