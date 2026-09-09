/**
 * The control state vocabulary — **one table, read by the stylesheet and by both gates.**
 *
 * MJXOFF-182 names the trap this file is built around, and it is worth quoting rather than
 * paraphrasing:
 *
 * > A states matrix is exactly the kind of thing that is *listed* in a story and not actually
 * > *distinct* in the CSS. **Two states that render identically pass every "the state exists"
 * > check**, and pressed-versus-hover is where it happens. So the gate is a **pairwise distinctness
 * > assertion** over rendered output: every pair of states in the matrix must differ visibly, in
 * > both themes.
 *
 * A pairwise assertion is only worth as much as the thing it reads. If the stylesheet said one
 * thing and the gate's expectations were typed out again beside it, the gate would be measuring a
 * second opinion — the failure `mjx-paint` refuses by declining to compare a painter with itself.
 * So the paint of every state lives **here**, once:
 *
 * * `controlStatesCss()` turns this table into the rules four components adopt, and
 * * `tests/controls.test.ts` resolves the same table through the generated token values and fails,
 *   naming the pair, if any two states of a component's matrix would compute alike **in either
 *   scheme** — before a browser is involved, and therefore also on the day the palette is
 *   re-seeded, and
 * * `tests/browser/controls.spec.ts` measures what the browser actually computed and requires the
 *   same distinctness there, which is what catches a rule that is right in this file and lost in
 *   the cascade.
 *
 * ## Every colour is a scheme member, never a `var()` string
 *
 * A state names `accentSurface`, not `'var(--theme-accent-surface)'`. The variable is derived by
 * `themeVariable()` from the same `customPropertyCase` the token resolver uses, so the stylesheet
 * and the Node gate cannot disagree about which token a state paints with — and the Node gate can
 * look the value up in the generated table for both schemes, which a `var()` string would make
 * impossible without a CSS parser.
 *
 * ## The cascade is source order, deliberately, and every selector has zero specificity
 *
 * Every rule below is wrapped in `:where()`, including its pseudo-classes. That is the *opposite*
 * of an accident: MJXOFF-181 discovered that `:where(…):focus:not(:focus-visible)` scores (0,2,0)
 * while `:where(…):focus-visible` scores (0,1,0), so a rule it believed was ordered was in fact
 * out-specified, and breaking the wrong one changed nothing. Here **all nine rules score (0,0,0)**
 * and `controlStateCascade` — an array, and therefore a thing a test can read — is the only thing
 * that decides which wins. `tests/browser/controls.spec.ts` drives a pressed toggle under the
 * pointer and requires the `onHover` paint, which is an assertion about this order and fails if a
 * later child inserts a rule in the wrong place.
 *
 * ## `data-state` is an audit affordance, and it is the *same rule* as the real thing
 *
 * `hover` and `active` cannot be produced in a static story, and a catalogue whose states matrix
 * cannot show them is a catalogue that documents six states and audits four. So each interaction
 * state matches **two ways in one selector list**: the real pseudo-class, and a `data-state`
 * attribute the component writes when a story asks for it. They share one declaration block, so a
 * forced state cannot drift into a mock of the real one — there is nothing for it to drift from.
 * The browser gate still drives a real pointer over a control and compares, because a structural
 * guarantee that nobody has watched hold is a guarantee about a file rather than about a browser.
 *
 * ## Node-importable
 *
 * Data and strings only. No DOM class is defined here, for the reason `src/harness/presets.ts`
 * states: the browser tier's specs run in Node.
 */

import { tokens, type ColorScheme } from '../../tokens/tokens.ts';
import { customPropertyCase, type ThemeMember } from '../tokens/resolver.ts';
import { spacingMultiple } from '../foundations/density.ts';

/** A paint: a scheme-relative colour, or the absence of one. */
export type Paint = ThemeMember | 'transparent';

/** `accentSurface` → `var(--theme-accent-surface)`. The one place a theme variable is spelled. */
export function themeVariable(member: ThemeMember): string {
  return `var(--theme-${customPropertyCase(member)})`;
}

/** `accentSurface` → `var(--theme-accent-surface)`; `transparent` → itself. */
export function paintValue(paint: Paint): string {
  return paint === 'transparent' ? 'transparent' : themeVariable(paint);
}

/**
 * The ten states a control can be in.
 *
 * Six belong to every control; `on`, `onHover`, `mixed` and `mixedHover` belong to the toggle.
 * `componentStateMatrix` below says which component claims which, and the stories are required to
 * declare exactly that set.
 */
export const controlStateNames = [
  'rest',
  'hover',
  'active',
  'focus',
  'disabled',
  'unavailable',
  'on',
  'onHover',
  'mixed',
  'mixedHover',
] as const;

/** One of the ten. */
export type ControlState = (typeof controlStateNames)[number];

/** Everything a state paints, and the conditions under which it paints it. */
export interface ControlStateSpec {
  /** What puts the control into it, and what the auditor should look for. */
  readonly description: string;
  /** The fill. */
  readonly background?: Paint;
  /** The border colour. */
  readonly borderColor?: Paint;
  /** `dashed` is `unavailable`'s alone: a border that says *there is a reason*. */
  readonly borderStyle?: 'solid' | 'dashed';
  /** The text and icon colour — an icon is filled with `currentColor`, so this tints both. */
  readonly text?: ThemeMember;
  /** `bold` marks a state the control is *holding* rather than passing through. */
  readonly weight?: 'medium' | 'bold';
  /** Only `disabled` dims. See `disabledOpacity`. */
  readonly opacity?: number;
  /**
   * An inset ring, in a theme colour.
   *
   * This is how hover reads on a state that already carries a fill: a pressed toggle cannot get
   * *more* tinted without becoming the pressed-and-held state, so its hover thickens the edge
   * instead. It is also what makes `on` and `onHover` — and `mixed` and `mixedHover` — pairwise
   * distinct without inventing a token the palette does not have.
   */
  readonly insetRing?: ThemeMember;
  /**
   * True for `focus` alone: the ring comes from `src/foundations/focus.ts`, so this state writes
   * no paint of its own and exists here only so the matrix and the gates can name it.
   */
  readonly ring?: boolean;
  /**
   * The selectors that produce it, with `%s` standing for the control's own selector.
   *
   * Empty for `focus`, which the foundations own.
   */
  readonly matches: readonly string[];
}

/**
 * How far a hard-disabled control is dimmed.
 *
 * **The one number in this file, and it is here because the token table has no opacity group.**
 * It is deliberately not a colour: dimming by opacity keeps a disabled toggle recognisably
 * *pressed*, which a flat grey fill would throw away — Office's own disabled ribbon does the same.
 * If the re-seed ever brings an opacity token, this constant is what it replaces.
 *
 * The contrast question it raises is answered by construction rather than by hope: only the state
 * that sets the native `disabled` attribute dims, and axe's `color-contrast` rule does not apply
 * to a disabled control. The *explained* unavailable state — the one a person can still reach and
 * still read — is at full opacity for exactly that reason.
 */
export const disabledOpacity = 0.5;

/** The state a control is in when nothing else applies, named once so a test cannot mistype it. */
export const restingState: ControlState = 'rest';

const notDisabled = ':not(:disabled):not([aria-disabled="true"])';

export const controlStateSpecs: Readonly<Record<ControlState, ControlStateSpec>> = {
  rest: {
    description: 'Resting. The chrome shows nothing but the label and the icon.',
    background: 'transparent',
    borderColor: 'transparent',
    borderStyle: 'solid',
    text: 'textPrimary',
    weight: 'medium',
    opacity: 1,
    matches: ['%s'],
  },
  hover: {
    description: 'The pointer is over it. A neutral tint — the accent tint means *pressed*.',
    background: 'borderSubtle',
    borderColor: 'borderSubtle',
    text: 'textPrimary',
    weight: 'medium',
    matches: ['%s[data-state="hover"]', `%s:hover${notDisabled}`],
  },
  active: {
    description: 'Held down. One rung darker than hover, in the same neutral family.',
    background: 'border',
    borderColor: 'border',
    text: 'textPrimary',
    weight: 'medium',
    matches: ['%s[data-state="active"]', `%s:active${notDisabled}`],
  },
  focus: {
    description:
      'Reached by keyboard. The ring is the foundations’ single focus treatment; this state ' +
      'paints nothing of its own, which is the point.',
    ring: true,
    matches: [],
  },
  disabled: {
    description:
      'Unavailable and not explained: out of the tab order entirely, dimmed, and inert to pointer ' +
      'and keyboard alike.',
    text: 'textSecondary',
    opacity: disabledOpacity,
    matches: ['%s:disabled'],
  },
  unavailable: {
    description:
      'Unavailable **and explained**. Still focusable, still announced, still carries the reason ' +
      'it cannot be used — and still cannot be activated. The dashed edge says there is a ' +
      'reason to go and read.',
    borderColor: 'borderSubtle',
    borderStyle: 'dashed',
    text: 'textSecondary',
    matches: ['%s[aria-disabled="true"]'],
  },
  on: {
    description: 'Pressed. The accent tint and a bold label — a state the control is holding.',
    background: 'accentSurface',
    borderColor: 'accentBorder',
    text: 'textPrimary',
    weight: 'bold',
    matches: ['%s[data-pressed="true"]'],
  },
  onHover: {
    description:
      'Pressed, with the pointer over it. The edge thickens rather than the fill changing, so ' +
      'pressed never has to compete with hover for the same tint.',
    background: 'accentSurface',
    borderColor: 'accent',
    text: 'textPrimary',
    weight: 'bold',
    insetRing: 'accent',
    matches: [
      '%s[data-pressed="true"][data-state="hover"]',
      `%s[data-pressed="true"]:hover${notDisabled}`,
    ],
  },
  mixed: {
    description:
      'Indeterminate: the selection disagrees with itself — some of it is bold and some is ' +
      'not. The honey half of the palette, because *this state is a disagreement*, and because it ' +
      'must not read as a weaker version of pressed.',
    background: 'secondarySurface',
    borderColor: 'secondaryAccent',
    text: 'textPrimary',
    weight: 'bold',
    matches: ['%s[data-pressed="mixed"]'],
  },
  mixedHover: {
    description: 'Indeterminate, with the pointer over it. The same edge treatment as onHover.',
    background: 'secondarySurface',
    borderColor: 'secondaryAccent',
    text: 'textPrimary',
    weight: 'bold',
    insetRing: 'secondaryAccent',
    matches: [
      '%s[data-pressed="mixed"][data-state="hover"]',
      `%s[data-pressed="mixed"]:hover${notDisabled}`,
    ],
  },
};

/**
 * What the cascade actually produces for one state — **the model both gates read.**
 *
 * `controlStateSpecs` records what a state *declares*; the rules are partial, so what a control
 * ends up painted with is `rest` overridden by the state's own fields. Writing that composition
 * down here rather than in a test is what lets the browser gate assert **correspondence** as well
 * as distinctness: it measures the computed background and border of every cell and requires them
 * to equal what this function says they should be. A model that agreed with itself and disagreed
 * with the browser is the failure that makes a pairwise assertion worthless.
 *
 * `box-shadow` is not composed, because `declarations()` emits it for every state — `none` unless
 * the state carries an inset ring — so it never inherits.
 */
export interface EffectiveStatePaint {
  readonly background: Paint;
  readonly borderColor: Paint;
  readonly borderStyle: 'solid' | 'dashed';
  readonly text: ThemeMember;
  readonly weight: 'medium' | 'bold';
  readonly opacity: number;
  readonly insetRing: ThemeMember | undefined;
  /** Whether the foundations' focus ring is showing. `focus` is the only state that says yes. */
  readonly ring: boolean;
}

/** The composition above, for one state. */
export function effectiveStatePaint(state: ControlState): EffectiveStatePaint {
  const resting = controlStateSpecs.rest;
  const spec = controlStateSpecs[state];
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

/**
 * A state's paint, resolved through the generated token table for one scheme.
 *
 * This is the string the pairwise gate compares. It is derived rather than written down, so the
 * palette re-seed moves the *values* and leaves the gate alone — which is the whole point of
 * naming scheme members instead of `var()` strings.
 */
export function resolvedStateFingerprint(state: ControlState, scheme: ColorScheme): string {
  const paint = effectiveStatePaint(state);
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
 * The cascade, in the order the rules are emitted — **and therefore in the order they win**, since
 * every one of them scores (0,0,0).
 *
 * `focus` is absent because it writes no paint. `unavailable` and `disabled` come last so that an
 * unavailable *pressed* toggle still looks pressed and also looks unavailable, which is the honest
 * rendering of both facts.
 */
export const controlStateCascade: readonly ControlState[] = [
  'rest',
  'hover',
  'active',
  'on',
  'onHover',
  'mixed',
  'mixedHover',
  'unavailable',
  'disabled',
];

/** The states a story may ask a control to pretend to be in. Nothing else is forcible. */
export const forcibleStates = ['hover', 'active'] as const;

/** One of the two forcible states. */
export type ForcibleState = (typeof forcibleStates)[number];

/** Whether a value names a forcible state. */
export function isForcibleState(value: unknown): value is ForcibleState {
  return (forcibleStates as readonly string[]).includes(String(value));
}

/** The four archetypes, and the states each one's matrix must show. */
export const componentStateMatrix = {
  button: ['rest', 'hover', 'active', 'focus', 'disabled', 'unavailable'],
  toggleButton: [
    'rest',
    'hover',
    'active',
    'focus',
    'disabled',
    'unavailable',
    'on',
    'onHover',
    'mixed',
    'mixedHover',
  ],
  splitButton: ['rest', 'hover', 'active', 'focus', 'disabled', 'unavailable'],
  dialogLauncher: ['rest', 'hover', 'active', 'focus', 'disabled', 'unavailable'],
} as const satisfies Readonly<Record<string, readonly ControlState[]>>;

/** One of the four archetypes. */
export type ControlArchetype = keyof typeof componentStateMatrix;

/** The four, in the order the catalogue shows them. */
export const controlArchetypes = Object.keys(componentStateMatrix) as readonly ControlArchetype[];

/**
 * The contract between the catalogue and the pairwise gate.
 *
 * The gate opens a story by *title and name* and finds its cells by *attribute*, so all three are
 * data here rather than strings typed twice. A divergence cannot pass quietly: the gate asserts it
 * found exactly `componentStateMatrix[archetype].length` cells, so a matrix story that stopped
 * marking them fails with the count rather than by finding nothing and reporting nothing.
 */
export const stateCellAttribute = 'data-state-cell';

/** The story every archetype publishes its whole states matrix under. */
export const statesMatrixStoryName = 'The States Matrix';

/** Where each archetype's stories live in the catalogue. */
export const archetypeStoryTitle: Readonly<Record<ControlArchetype, string>> = {
  button: 'Controls/Button',
  toggleButton: 'Controls/Toggle Button',
  splitButton: 'Controls/Split Button',
  dialogLauncher: 'Controls/Dialog Launcher',
};

/** The tag each archetype is registered under. */
export const archetypeTags: Readonly<Record<ControlArchetype, string>> = {
  button: 'mjx-button',
  toggleButton: 'mjx-toggle-button',
  splitButton: 'mjx-split-button',
  dialogLauncher: 'mjx-dialog-launcher',
};

// ── size ─────────────────────────────────────────────────────────────────────

/** The three shapes Office's ribbon actually uses. */
export const controlSizeNames = ['large', 'small', 'icon'] as const;

/** One of the three. */
export type ControlSize = (typeof controlSizeNames)[number];

/** Everything a size fixes. */
export interface ControlSizeSpec {
  /** Which of the subset's drawings is used. Fluent draws each size separately. */
  readonly iconSize: 16 | 20 | 24;
  /** `column` puts the icon above the label; `row` puts it beside. */
  readonly orientation: 'column' | 'row';
  /** Whether the label is drawn. When it is not, it is still *announced* — never removed. */
  readonly labelVisible: boolean;
  /** How many lines the label may wrap to before it is clamped. */
  readonly labelLines: number;
  readonly use: string;
}

export const controlSizes: Readonly<Record<ControlSize, ControlSizeSpec>> = {
  large: {
    iconSize: 24,
    orientation: 'column',
    labelVisible: true,
    labelLines: 2,
    use: 'The tall ribbon button: icon above a label that may wrap to two lines. Office’s primary command shape.',
  },
  small: {
    iconSize: 20,
    orientation: 'row',
    labelVisible: true,
    labelLines: 1,
    use: 'The short ribbon button: icon beside a single-line label. What a group’s secondary commands use.',
  },
  icon: {
    iconSize: 20,
    orientation: 'row',
    labelVisible: false,
    labelLines: 1,
    use: 'Icon only. The label is still the accessible name — it is drawn off-screen, never dropped.',
  },
};

/** The size a control is when it does not say. */
export const defaultControlSize: ControlSize = 'small';

/** Whether a value names a size. */
export function isControlSize(value: unknown): value is ControlSize {
  return (controlSizeNames as readonly string[]).includes(String(value));
}

/**
 * The inline size a large button is laid out between, in spacing units.
 *
 * A two-line label is only a two-line label if the box has a width to wrap inside; a `large`
 * button with no bound would grow to fit its label on one line and the wrapping this size exists
 * for would never happen. Both bounds are multiples of `--spacing`, so they follow a re-seed.
 */
export const largeControlWidthUnits = { min: 14, max: 20 } as const;

// ── the split button's two regions ───────────────────────────────────────────

/**
 * The glyph the arrow region draws, and the drawing it uses.
 *
 * Here rather than in `split-button.ts` for the reason `src/harness/presets.ts` states: a module
 * that defines a custom element cannot be imported from Node, and the unit tier runs in Node.
 */
export const splitMenuIcon = { name: 'chevron-down', size: 16 } as const;

/**
 * `Undo` → `More Undo options`.
 *
 * `GUESS:` this is the shape Office uses (*"More Paste options"*) and it is **not** checked
 * against Office. A caller who knows the real name gives it in `menu-label`; what this guarantees
 * is only that the arrow never ends up sharing the primary's name, which would be two commands
 * with one name and is the silent half of a split button's accessibility.
 */
export function derivedMenuLabel(label: string): string {
  return `More ${label} options`;
}

// ── the toggle's three positions ─────────────────────────────────────────────

/** What `aria-pressed` may say. `mixed` is a selection that disagrees with itself. */
export const pressedValues = ['false', 'true', 'mixed'] as const;

/** One of the three. */
export type PressedValue = (typeof pressedValues)[number];

/** Whether a value names one of the three positions. */
export function isPressedValue(value: unknown): value is PressedValue {
  return (pressedValues as readonly string[]).includes(String(value));
}

/**
 * What one activation does to a toggle.
 *
 * `false` → `true` and `true` → `false` are not in question. **`mixed` → `true` is a GUESS:** it
 * is what Word does when a mixed-bold selection is bolded — the whole selection becomes bold
 * rather than losing its formatting — but nothing here has been checked against a screenshot of
 * Office, and this project does not call an unverified match parity.
 */
export function nextPressed(current: PressedValue): PressedValue {
  return current === 'true' ? 'false' : 'true';
}

// ── events ───────────────────────────────────────────────────────────────────

/**
 * The events the four archetypes emit.
 *
 * Wiring one to a command is loop 2; what this child owes is that the *right* event fires, exactly
 * once, and that no event at all fires from a control that cannot be used. All three bubble and
 * are `composed`, so a host listens on its own root rather than on each control's shadow tree.
 */
export const controlEvents = {
  /** A button, a dialog launcher, or a split button's primary region was activated. */
  activate: 'mjx-activate',
  /** A toggle moved. `detail.pressed` is the new position. */
  change: 'mjx-change',
  /**
   * A split button's menu was asked for — by its arrow region, or by Arrow Down on its primary.
   *
   * It is a *request* and not an opening, because U05 owns menus. The component sets
   * `aria-expanded` from its `expanded` attribute and never from this event, so a host that has
   * not built a menu yet does not end up announcing one that is not there.
   */
  menuRequest: 'mjx-menu-request',
} as const;

// ── the stylesheet ───────────────────────────────────────────────────────────

/** The declarations one state contributes, or `''` when it contributes none. */
function declarations(spec: ControlStateSpec): string {
  const lines: string[] = [];
  if (spec.background !== undefined) lines.push(`  background: ${paintValue(spec.background)};`);
  if (spec.borderColor !== undefined) {
    lines.push(`  border-color: ${paintValue(spec.borderColor)};`);
  }
  if (spec.borderStyle !== undefined) lines.push(`  border-style: ${spec.borderStyle};`);
  if (spec.text !== undefined) lines.push(`  color: ${themeVariable(spec.text)};`);
  if (spec.weight !== undefined) {
    lines.push(`  font-weight: var(--font-weight-${spec.weight === 'bold' ? 'bold' : 'medium'});`);
  }
  if (spec.opacity !== undefined) lines.push(`  opacity: ${String(spec.opacity)};`);
  lines.push(
    spec.insetRing === undefined
      ? '  box-shadow: none;'
      : `  box-shadow: inset 0 0 0 1px ${themeVariable(spec.insetRing)};`,
  );
  return lines.join('\n');
}

/**
 * The state rules for one control selector.
 *
 * Called once per component with `.control`; a split button calls it once and gets both of its
 * regions, because both carry that class and each therefore hovers on its own.
 */
export function controlStatesCss(selector: string): string {
  return controlStateCascade
    .map((state) => {
      const spec = controlStateSpecs[state];
      const where = spec.matches.map((match) => match.replaceAll('%s', selector)).join(', ');
      return `:where(${where}) {\n${declarations(spec)}\n}`;
    })
    .join('\n');
}

/**
 * The layout every archetype shares: the box, the hit-target floor and the motion.
 *
 * Most of what a control looks like is *not* here. `.mjx-type-control`, `.mjx-hit-target`,
 * `.mjx-motion-surface-settle` and the focus ring are all classes a component **wears**, and they
 * only do anything because `installFoundations(shadowRoot)` ran. That is deliberate and it is
 * asserted: `tests/browser/controls.spec.ts` reads the hit-target floor, the type role and the
 * focus ring off a control's inner button, and all three disappear if the installation is skipped.
 *
 * ⚠ **This rule paints nothing** — no background, no border colour, no text colour, no weight, no
 * opacity, no shadow. Every one of those comes from `controlStatesCss()`, whose selectors score
 * (0,0,0). A `.control { background: transparent }` here would score (0,1,0) and would therefore
 * **out-specify every state rule in the table**, leaving a control that renders its resting paint
 * in all ten states while every "the state exists" check passes. That is MJXOFF-181's specificity
 * accident wearing this child's costume, and the separation above is what prevents it: this rule
 * owns the box, the state table owns the paint, and neither declares one of the other's
 * properties.
 */
export const controlBaseCss = `
  :host {
    display: inline-flex;
    vertical-align: middle;
  }
  :host([hidden]) { display: none; }

  .control {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--mjx-density-step);
    box-sizing: border-box;
    margin: 0;
    padding-inline: var(--mjx-density-gutter);
    padding-block: 0;
    border-width: 1px;
    border-radius: var(--radius-control);
    text-align: center;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .control:disabled { cursor: default; }
  .control[aria-disabled='true'] { cursor: help; }

  .label {
    display: -webkit-box;
    -webkit-box-orient: vertical;
    overflow: hidden;
    overflow-wrap: anywhere;
  }

  /* Announced, never drawn. An icon-only control keeps its name; it does not lose it. */
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
 * **The levers a container may pull on a control it does not own** (MJXOFF-183).
 *
 * A ribbon group has to make its commands smaller when it is short of width, and there are exactly
 * three ways it could: reach into the control's shadow root (impossible, and wrong), rewrite the
 * control's `size` attribute from a resize handler (JavaScript-measured layout, which MJXOFF-183
 * rules out by name), or have the control **publish** the handful of properties a container is
 * allowed to influence and read them itself. This is the third.
 *
 * Every one is a custom property whose fallback is *exactly what the size already says*, so a
 * control with nothing set above it computes byte-for-byte what it computed before these existed —
 * which is why MJXOFF-182's suites did not move when they were added. A container that wants a
 * `large` command to lay itself out like a `small` one sets `--mjx-control-orientation: row`, and
 * the control's own rules do the rest. Custom properties inherit through the flat tree, so this is
 * the one channel that reaches a slotted component's shadow root at all.
 *
 * ⚠ **The icon is deliberately not a lever.** Fluent draws each size separately and
 * `src/icons/manifest.ts` says so: choosing a drawing is not a scale factor, and a container query
 * cannot change an attribute. A reduced `large` command therefore keeps its 24px drawing beside a
 * one-line label, which is a real shape rather than a scaled-down one.
 */
export const controlLevers = {
  /** `column` or `row` — what a `large` command's icon and label do relative to each other. */
  orientation: '--mjx-control-orientation',
  /** How many lines the label may wrap to. */
  labelLines: '--mjx-control-label-lines',
  /** The `large` shape's width floor. Never set it below `--mjx-hit-target`. */
  minInline: '--mjx-control-min-inline',
  /** The `large` shape's width ceiling, or `none`. */
  maxInline: '--mjx-control-max-inline',
} as const;

/**
 * The size rules, generated from `controlSizes` so a size cannot be styled but undeclared.
 *
 * These are **not** wrapped in `:where()`, and the reason is the same specificity argument stated
 * above read the other way round: they refine the box (`flex-direction`, `padding-block`,
 * `line-clamp`) that `.control` already declares at (0,1,0), so they must out-specify it. They
 * declare no paint at all, so they cannot compete with the state table however they score.
 */
export const controlSizeCss = controlSizeNames
  .map((size) => {
    const spec = controlSizes[size];
    const width =
      size === 'large'
        ? `  min-inline-size: var(${controlLevers.minInline}, ${spacingMultiple(largeControlWidthUnits.min)});\n` +
          `  max-inline-size: var(${controlLevers.maxInline}, ${spacingMultiple(largeControlWidthUnits.max)});\n` +
          `  padding-block: var(--mjx-density-step);\n`
        : '';
    return (
      `.control[data-size='${size}'] {\n` +
      `  flex-direction: var(${controlLevers.orientation}, ${spec.orientation});\n` +
      width +
      `}\n` +
      `.control[data-size='${size}'] .label {\n` +
      `  -webkit-line-clamp: var(${controlLevers.labelLines}, ${String(spec.labelLines)});\n` +
      `  line-clamp: var(${controlLevers.labelLines}, ${String(spec.labelLines)});\n` +
      `}`
    );
  })
  .join('\n');
