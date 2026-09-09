/**
 * The ribbon's structural vocabulary — **one table, read by the stylesheet and by both gates.**
 *
 * MJXOFF-183 names the trap this file is built around, and MJXOFF-182 named the sharper one that
 * generalises to it. Both are worth quoting rather than paraphrasing:
 *
 * > *"The ribbon is responsive"* is satisfied by a layout that merely wraps or scrolls
 * > horizontally, which looks plausible in a screenshot and is unusable in practice. **The gate
 * > must assert the three group presentations are actually reached at declared container widths**,
 * > and that a collapsed group's commands are still *reachable*.
 *
 * > **A distinctness gate proves no two states are the same; it does not prove any of them is
 * > right.**
 *
 * The second is why nothing here is expressed as *"narrow differs from wide"*. Every presentation
 * is a **named row in a table**, the stylesheet is generated from that table, and both gates
 * compute the expected row from `groupPresentationAt()` and compare. A rule that put a group in
 * the wrong presentation would satisfy any assertion that merely said the layout changed.
 *
 * ## Why a priority *ladder* and not a per-group pixel width
 *
 * The ticket is explicit that *"Office's group-collapse ordering is a per-group priority, not a
 * uniform rule, so groups must be able to declare their own"*. It is equally explicit that a
 * resize handler is the wrong instrument: *"no JavaScript-measured layout where a container query
 * would do — measurement in a resize handler is how a ribbon becomes janky."*
 *
 * Those two together fix the design. CSS cannot read a number out of an attribute and use it in a
 * `@container` condition, so *per-group arbitrary widths* would require JavaScript to measure and
 * assign — the thing that is ruled out. What CSS **can** do is match on the attribute's *value*, so
 * the declaration a group makes is a **priority**, and the ladder below turns four priorities into
 * eight generated `@container` blocks. A group says *when it gives way relative to the others*,
 * which is what Office's ordering actually is, and the mechanism stays pure CSS.
 *
 * `Ribbon/Group presentations · Three Presentations At One Width` is the story that shows why the
 * ladder is not a uniform rule: at 1000px three groups sit in three different presentations
 * simultaneously, which a single global breakpoint cannot produce.
 *
 * ## How the browser is asked which presentation it chose
 *
 * A container query is CSS's decision, and both the component's own behaviour and the gate need to
 * *read it back* without measuring anything. So every presentation block sets a custom property —
 * `--mjx-group-presentation: reduced` — and `getComputedStyle(...).getPropertyValue()` is the
 * answer. The component reads it to know whether its panel is currently a popup; the gate reads it
 * to compare against `groupPresentationAt()`.
 *
 * ⚠ A gate that read *only* that property would be asking the implementation to grade its own
 * homework: a stylesheet that set the token and changed no layout would pass. So
 * `tests/browser/ribbon.spec.ts` cross-checks each presentation against three facts it does not
 * control — the collapse trigger's `display`, the panel's `position`, and the computed
 * `--mjx-density-step` inside the group — all four of which must agree with `groupPresentations`
 * below.
 *
 * ## Node-importable
 *
 * Data and strings only. No DOM class is defined here, for the reason `src/harness/presets.ts`
 * states: the browser tier's specs run in Node.
 */

import { customPropertyCase, type ThemeMember } from '../tokens/resolver.ts';
import { controlStatesCss } from '../controls/control-states.ts';
import {
  densityModes,
  densityProperties,
  spacingMultiple,
  type DensityMode,
} from '../foundations/density.ts';
import { radiusVariable, surfaceLevels } from '../foundations/surfaces.ts';
import { controlLevers } from '../controls/control-states.ts';
import { phoneShellAtOrBelow } from '../harness/presets.ts';

/** `accentSurface` → `var(--theme-accent-surface)`. Re-stated from the control table's spelling. */
export function themeVariable(member: ThemeMember): string {
  return `var(--theme-${customPropertyCase(member)})`;
}

// ── the container ────────────────────────────────────────────────────────────

/**
 * The name every ribbon-scoped `@container` query addresses.
 *
 * `<mjx-ribbon>` establishes it **on its own host element**, which is a light-DOM ancestor of every
 * group. A group's rules live inside the group's *shadow* root and query it from there, which
 * works because container lookup walks the flat tree and therefore crosses a shadow boundary —
 * verified in Chromium before this file was written, because the whole design rests on it.
 *
 * It is deliberately not `mjx-frame` (the harness's own container). A component that queried the
 * harness would be untestable outside the harness and would silently take the harness's width when
 * the real application's ribbon was narrower than its window.
 */
export const ribbonContainerName = 'mjx-ribbon';

/**
 * The custom property a group's container rules write, and the component and the gate read.
 *
 * This is how a CSS decision is legible to JavaScript **without measuring anything**. The
 * alternative — `getBoundingClientRect()` in a resize handler — is the design the ticket rules
 * out, and it would also make the gate a measurement of a measurement.
 */
export const groupPresentationProperty = '--mjx-group-presentation';

/** The same idea for the tab strip. */
export const tabStripPresentationProperty = '--mjx-tab-strip-presentation';

// ── the three group presentations ────────────────────────────────────────────

/**
 * The three presentations, **widest first — and this array is the cascade.**
 *
 * Every generated rule scores identically, so source order is the only thing deciding which wins
 * at a width where two conditions match. That is MJXOFF-182's rule (*"the cascade is source
 * order"*) applied here, and `tests/ribbon.test.ts` reads this array rather than trusting it.
 */
export const groupPresentationOrder = ['full', 'reduced', 'collapsed'] as const;

/** One of the three. */
export type GroupPresentation = (typeof groupPresentationOrder)[number];

/**
 * Everything a presentation fixes — **and every field is something a gate can measure.**
 *
 * A presentation that existed only as a name would be exactly the defect the ticket describes: an
 * implementation that cannot collapse at all lays a ribbon out identically at one width.
 */
export interface GroupPresentationSpec {
  /** What the auditor is looking at. */
  readonly description: string;
  /** Which density the group's subtree runs at. `compact` is still above the 24px hit floor. */
  readonly density: DensityMode;
  /** Whether the single collapse button is drawn — and therefore whether it is focusable. */
  readonly triggerVisible: boolean;
  /** Whether the group's label sits under its commands. The trigger carries the name instead. */
  readonly footerVisible: boolean;
  /** Whether the panel is an overlay anchored to the trigger rather than part of the strip. */
  readonly panelIsPopup: boolean;
  /** How many rows the commands wrap into. */
  readonly commandRows: number;
  /**
   * What a `large` control is asked to become, through the levers `controlSizeCss` publishes.
   *
   * `undefined` leaves the control's own `size` attribute in charge, which is what `full` does. A
   * group **never restyles a control**; it sets a documented custom property the control's own
   * rules read, which is the difference between arranging and reaching in.
   */
  readonly largeControlOrientation: 'row' | undefined;
  readonly use: string;
}

export const groupPresentations: Readonly<Record<GroupPresentation, GroupPresentationSpec>> = {
  full: {
    description:
      'Everything the group has, at the size its author asked for: large commands stay large, ' +
      'the group’s name sits under them, and the dialog launcher sits beside the name.',
    density: 'comfortable',
    triggerVisible: false,
    footerVisible: true,
    panelIsPopup: false,
    commandRows: 1,
    largeControlOrientation: undefined,
    use: 'A desktop ribbon with room to spare.',
  },
  reduced: {
    description:
      'The same commands, closer together. Compact density, a large command turns its icon and ' +
      'label side by side and clamps to one line, and the row wraps into two.',
    density: 'compact',
    triggerVisible: false,
    footerVisible: true,
    panelIsPopup: false,
    commandRows: 2,
    largeControlOrientation: 'row',
    use: 'A tablet, a split editor, a docked pane — anywhere the ribbon is narrow but the group still fits.',
  },
  collapsed: {
    description:
      'One button carrying the group’s name, and the whole group behind it. The commands marked ' +
      'essential stay beside the button; everything else is one press away and nothing is lost.',
    density: 'compact',
    triggerVisible: true,
    footerVisible: false,
    panelIsPopup: true,
    commandRows: 3,
    largeControlOrientation: 'row',
    use: 'A phone, and any width at which the group would otherwise push its neighbours off the strip.',
  },
};

// ── the priority ladder ──────────────────────────────────────────────────────

/** The four priorities a group may declare, first to give way last. */
export const groupPriorityNames = ['primary', 'standard', 'secondary', 'ancillary'] as const;

/** One of the four. */
export type GroupPriority = (typeof groupPriorityNames)[number];

/** What a group is promising when it declares a priority. */
export interface GroupPrioritySpec {
  /** At or below this container width the group is at most `reduced`. */
  readonly reduceAtOrBelow: number;
  /** At or below this container width the group is `collapsed`. */
  readonly collapseAtOrBelow: number;
  readonly use: string;
}

/**
 * The ladder, in container pixels.
 *
 * ⚠ **These are the one class of length the token system has no name for**, and that is not an
 * oversight in either direction: a breakpoint is a relationship between a layout and a container,
 * not a design value, and `--spacing` cannot appear in a `@container` condition because a query
 * condition is evaluated before custom properties on the queried element are known. They are
 * therefore data *here*, once, and every rule that uses them is generated — which is why
 * `mjx/no-literal-design-values` sees no `900px` anywhere in this crate's CSS strings.
 *
 * The numbers are chosen so that the harness's three presets land informatively: at `desktop`
 * (1440) three of the four are `full` and `ancillary` is already `reduced`; at `tablet` (834) three
 * are `reduced` and `ancillary` is `collapsed`; at `phone` (390) everything is `collapsed`. And at
 * 1000 — a width no preset offers, which is why the story sets it explicitly — three different
 * presentations are on screen at once.
 *
 * `GUESS:` the ladder's *shape* is Office's (a group declares when it gives way, not at what
 * width), but no number here is checked against Office and none of this is parity.
 */
export const groupPriorities: Readonly<Record<GroupPriority, GroupPrioritySpec>> = {
  primary: {
    reduceAtOrBelow: 900,
    collapseAtOrBelow: 420,
    use: 'The group the tab exists for — Word’s Font on Home. It is the last thing to give way, and on a phone it is the only group still showing its essential commands.',
  },
  standard: {
    reduceAtOrBelow: 1100,
    collapseAtOrBelow: 600,
    use: 'The default. A group a person reaches for often but not constantly.',
  },
  secondary: {
    reduceAtOrBelow: 1300,
    collapseAtOrBelow: 800,
    use: 'A group whose commands are also reachable elsewhere — Clipboard, whose four verbs are on the keyboard anyway.',
  },
  ancillary: {
    reduceAtOrBelow: 1600,
    collapseAtOrBelow: 1000,
    use: 'A group that is rarely the reason anyone opened the tab. It is never full below a wide desktop, and it collapses while there is still plenty of room.',
  },
};

/** The priority a group is when it does not say. */
export const defaultGroupPriority: GroupPriority = 'standard';

/** Whether a value names a priority. */
export function isGroupPriority(value: unknown): value is GroupPriority {
  return (groupPriorityNames as readonly string[]).includes(String(value));
}

/**
 * **The model both gates read**: which presentation a group of this priority is in, at this
 * container width.
 *
 * It is written as the same cascade the stylesheet emits — `full`, then `reduced` if the condition
 * holds, then `collapsed` if that condition holds — rather than as a chain of comparisons in the
 * opposite order, because a model that agreed with itself and disagreed with the browser is the
 * failure MJXOFF-182 spent a child discovering.
 *
 * `simplified` is Office's single-row ribbon. It never *prevents* a collapse; it only refuses the
 * full presentation, which is why it is applied between the two container conditions.
 */
export function groupPresentationAt(
  priority: GroupPriority,
  containerWidth: number,
  options: { readonly simplified?: boolean } = {},
): GroupPresentation {
  const ladder = groupPriorities[priority];
  let presentation: GroupPresentation = 'full';
  if (options.simplified === true) presentation = 'reduced';
  if (containerWidth <= ladder.reduceAtOrBelow) presentation = 'reduced';
  if (containerWidth <= ladder.collapseAtOrBelow) presentation = 'collapsed';
  return presentation;
}

// ── the tab strip's two presentations ────────────────────────────────────────

/**
 * What happens when the tab strip itself does not fit — **the phone case**, and the one the ticket
 * says a naive implementation answers by scrolling.
 *
 * It is two presentations rather than a scroll for a reason that is worth stating: a horizontally
 * scrolling tab strip *loses* tabs in the only sense that matters, because nothing on screen says
 * they exist. The picker keeps every tab in one list, announces how many there are, and costs one
 * press.
 */
export const tabStripPresentationOrder = ['strip', 'picker'] as const;

/** One of the two. */
export type TabStripPresentation = (typeof tabStripPresentationOrder)[number];

/** What each one is. */
export const tabStripPresentations: Readonly<
  Record<TabStripPresentation, { readonly description: string }>
> = {
  strip: { description: 'Every tab side by side, in a horizontal tablist.' },
  picker: {
    description:
      'One button naming the selected tab. Pressing it opens the whole tablist as a vertical ' +
      'popup — the same tablist, the same buttons, the same roving tabindex.',
  },
};

/**
 * At or below this container width the strip becomes a picker.
 *
 * **The number itself lives in `src/harness/presets.ts`** (`phoneShellAtOrBelow`) since
 * MJXOFF-185, because three surfaces now turn on it — this one, a floating menu becoming a sheet,
 * and a gallery's expanded flyout becoming a sheet — and MJXOFF-184's note said what to do when a
 * third arrived. This name stays, because *"the strip becomes a picker"* is what this file is
 * talking about; what it no longer does is write the number down a second time.
 */
export const tabStripPickerAtOrBelow = phoneShellAtOrBelow;

/** The model the gate reads. */
export function tabStripPresentationAt(containerWidth: number): TabStripPresentation {
  return containerWidth <= tabStripPickerAtOrBelow ? 'picker' : 'strip';
}

// ── the ribbon's three states ────────────────────────────────────────────────

/** Expanded, collapsed to its tabs, or out of the way entirely. */
export const ribbonStateNames = ['expanded', 'tabs', 'hidden'] as const;

/** One of the three. */
export type RibbonState = (typeof ribbonStateNames)[number];

/** What each state shows. Every one of these is measurable, which is what the gate reads. */
export const ribbonStates: Readonly<
  Record<
    RibbonState,
    {
      readonly description: string;
      readonly stripVisible: boolean;
      readonly bodyVisible: boolean;
      /** Whether the *only* thing on screen is the button that brings the ribbon back. */
      readonly restoreVisible: boolean;
    }
  >
> = {
  expanded: {
    description: 'Tabs and the selected tab’s groups. The ordinary state.',
    stripVisible: true,
    bodyVisible: true,
    restoreVisible: false,
  },
  tabs: {
    description:
      'Tabs alone. Office’s collapsed ribbon: the commands are one press away and the document ' +
      'has the room back.',
    stripVisible: true,
    bodyVisible: false,
    restoreVisible: false,
  },
  hidden: {
    description:
      'Nothing but the button that brings it back. Not `display: none` on the whole element — a ' +
      'ribbon a keyboard cannot restore is a ribbon a keyboard has lost.',
    stripVisible: false,
    bodyVisible: false,
    restoreVisible: true,
  },
};

/** The state a ribbon is in when it does not say. */
export const defaultRibbonState: RibbonState = 'expanded';

/** Whether a value names a state. */
export function isRibbonState(value: unknown): value is RibbonState {
  return (ribbonStateNames as readonly string[]).includes(String(value));
}

// ── command demotion ─────────────────────────────────────────────────────────

/** The slot a command declares itself essential by being in. */
export const essentialSlotName = 'essential';

/**
 * How many commands a group may keep visible through a collapse.
 *
 * The number is part of the rule rather than a taste: a survivor row longer than this is a ribbon
 * again, and the collapse bought nothing. `tests/browser/ribbon.spec.ts` asserts it over the
 * worst-case story, and `<mjx-ribbon-group>` reports a violation on the console the way
 * `<mjx-button>` reports a nameless button.
 */
export const essentialCommandLimit = 3;

/**
 * **The command demotion rules — the design decision the audit needs to see.**
 *
 * MJXOFF-183: *"Command demotion rules, documented: which controls survive into a phone toolbar
 * and by what criterion. This is a design decision the audit needs to see, not an implementation
 * detail."*
 *
 * The criterion is three tests, and a command must pass all three. Two of them are checkable by a
 * gate and the third is a judgement, which is stated as a judgement rather than dressed up as a
 * measurement.
 *
 * The rule that makes the whole thing safe is the one at the end: **demotion never removes a
 * command.** A collapsed group holds every one of its commands in the same DOM nodes it held them
 * in when it was full — the panel becomes a popup, it is not rebuilt — so *"reachable in all three
 * presentations"* is structural rather than something the implementation has to remember.
 */
export const demotionRules: readonly {
  readonly rule: string;
  readonly because: string;
  readonly checkedBy: string;
}[] = [
  {
    rule: 'It is immediate and reversible: one press does one thing, and one undo takes it back.',
    because:
      'A command that opens a menu, a gallery or a dialog puts a popup inside a popup, which is ' +
      'where a phone UI dies. Those stay in the group and cost one press to reach.',
    checkedBy:
      'The gate reads every essential command’s shadow root and fails on an aria-haspopup, and ' +
      'fails on a split button, whose whole shape is a menu.',
  },
  {
    rule: 'It is recognisable with no label, at icon size.',
    because:
      'The survivor row has no room for text. A command whose icon does not say what it is has ' +
      'been demoted into a mystery, which is worse than one more press.',
    checkedBy:
      'A judgement, and stated as one. What the gate can say is that the command has an icon and ' +
      'an accessible name at all, which it does.',
  },
  {
    rule: `At most ${String(essentialCommandLimit)} per group.`,
    because:
      'A longer survivor row is a ribbon again. The collapse exists to buy width, and a rule with ' +
      'no ceiling gives it all back.',
    checkedBy: 'essentialCommandLimit, asserted by the browser gate and reported on the console.',
  },
  {
    rule: 'Nothing is ever removed.',
    because:
      'A control that disappears at narrow width has not degraded, it has been lost. This is the ' +
      'assertion the ticket says is the one that matters.',
    checkedBy:
      'Structural: the panel that becomes a popup is the same element holding the same slot, so ' +
      'the commands are the same DOM nodes at every width. The gate counts them at all three.',
  },
];

// ── the tab tones ────────────────────────────────────────────────────────────

/**
 * The two tones a tab can carry, and every colour in them is a scheme member.
 *
 * A core tab reuses the control state table's `on` paint for *selected*, because a selected tab and
 * a pressed toggle are the same claim and a design system whose two disagree has lost the argument
 * it exists to settle. A contextual tab is the one place MJXOFF-183 adds paint of its own, and it
 * uses the palette's honey half — the same family `mixed` uses, for the same reason: *this is not
 * the ordinary thing*.
 *
 * `GUESS:` Office gives each tool set its own hue. This palette has two families, so every
 * contextual set shares one and is told apart by its **title**, which is the part a screen reader
 * gets either way. A re-seed that brings more hues is where per-set colour would come from.
 */
export interface TabToneSpec {
  readonly selectedBackground: ThemeMember;
  readonly selectedBorder: ThemeMember;
  /** The band drawn over a contextual set's tabs. `undefined` for a core tab, which has none. */
  readonly bandBackground: ThemeMember | undefined;
  readonly bandText: ThemeMember | undefined;
  readonly use: string;
}

export const tabToneNames = ['core', 'contextual'] as const;

/** One of the two. */
export type TabTone = (typeof tabToneNames)[number];

export const tabTones: Readonly<Record<TabTone, TabToneSpec>> = {
  core: {
    selectedBackground: 'accentSurface',
    selectedBorder: 'accentBorder',
    bandBackground: undefined,
    bandText: undefined,
    use: 'Home, Insert, Review — the tabs that are always there.',
  },
  contextual: {
    selectedBackground: 'secondarySurface',
    selectedBorder: 'secondaryAccent',
    bandBackground: 'secondarySurface',
    bandText: 'textPrimary',
    use: 'Table Tools, Picture Tools, Chart Tools — a set that appears because something is selected.',
  },
};

// ── the attributes the gates address ─────────────────────────────────────────

/** The attribute a tab button carries its tab's id in. */
export const tabIdAttribute = 'data-tab';

/** The attribute a tab button carries its tone in. */
export const tabToneAttribute = 'data-tone';

/** The events the ribbon emits. Wiring one to a command is loop 2. */
export const ribbonEvents = {
  /** The selected tab changed. `detail.tabId` is the new one. */
  tabChange: 'mjx-ribbon-tab-change',
  /** A collapsed group opened or closed. `detail.open` says which. */
  groupToggle: 'mjx-ribbon-group-toggle',
  /** The ribbon's own state changed. `detail.state` is the new one. */
  stateChange: 'mjx-ribbon-state-change',
} as const;

// ── the stylesheet ───────────────────────────────────────────────────────────

/** The density custom properties for one mode, composed from the density foundation's own table. */
function densityDeclarations(mode: DensityMode, indent: string): string {
  const spec = densityModes[mode];
  return [
    `${indent}${densityProperties.step}: ${spacingMultiple(spec.stepUnits)};`,
    `${indent}${densityProperties.gutter}: ${spacingMultiple(spec.gutterUnits)};`,
    `${indent}${densityProperties.hitTarget}: ${spacingMultiple(spec.hitTargetUnits)};`,
  ].join('\n');
}

/**
 * How wide the collapsed group's popup is at its narrowest, in spacing units.
 *
 * A popup that shrank to the width of its widest command would be a sliver holding one button, and
 * a person would have to aim at it. Expressed in `--spacing` like every other length here, so a
 * re-seed moves it.
 */
export const collapsedPanelMinimumUnits = 40;

/**
 * The declarations one presentation contributes.
 *
 * Everything a presentation changes is a **custom property**, and that is the whole mechanism
 * rather than a stylistic choice: custom properties inherit through the flat tree, so a value set
 * here on the group's own box reaches the slotted controls' shadow roots — which is the only legal
 * way for a group to make a command smaller without reaching into a component it does not own.
 *
 * The popup's paint is `surfaceLevels.overlay` read out of the elevation ladder rather than a card
 * assembled here, for the reason `surfaces.ts` gives: *"a menu, a dialog, a popover — anything
 * drawn over content it must be readable against"* is a rung that already exists, and a second
 * hand-made one is how a design system acquires two slightly different menus.
 */
function presentationDeclarations(presentation: GroupPresentation, indent: string): string {
  const spec = groupPresentations[presentation];
  const overlay = surfaceLevels.overlay;
  const popup = spec.panelIsPopup;
  const lines = [
    `${indent}${groupPresentationProperty}: ${presentation};`,
    densityDeclarations(spec.density, indent),
    `${indent}--mjx-group-trigger-display: ${spec.triggerVisible ? 'inline-flex' : 'none'};`,
    `${indent}--mjx-group-footer-display: ${spec.footerVisible ? 'flex' : 'none'};`,
    `${indent}--mjx-group-panel-position: ${popup ? 'absolute' : 'static'};`,
    `${indent}--mjx-group-panel-label-display: ${popup ? 'block' : 'none'};`,
    // A collapsed group's panel is not drawn until it is asked for; an expanded one's is always
    // drawn, and `open` means nothing to it.
    `${indent}--mjx-group-panel-closed-display: ${popup ? 'none' : 'flex'};`,
    `${indent}--mjx-group-panel-background: ${popup ? overlay.background : 'transparent'};`,
    `${indent}--mjx-group-panel-border: ${popup ? overlay.border : 'none'};`,
    `${indent}--mjx-group-panel-shadow: ${popup ? overlay.shadow : 'none'};`,
    `${indent}--mjx-group-panel-radius: ${popup ? radiusVariable(overlay.radius) : '0'};`,
    `${indent}--mjx-group-panel-padding: ${popup ? `var(${densityProperties.gutter})` : '0'};`,
    `${indent}--mjx-group-panel-min-inline: ${popup ? spacingMultiple(collapsedPanelMinimumUnits) : 'auto'};`,
    // Column flow over a fixed number of rows is how Office fills a group: down, then across.
    // The whole value is substituted rather than the count alone, because a custom property inside
    // `repeat()` is a grammar this does not need to depend on.
    `${indent}--mjx-group-command-template: repeat(${String(spec.commandRows)}, auto);`,
  ];
  if (spec.largeControlOrientation !== undefined) {
    lines.push(
      `${indent}${controlLevers.orientation}: ${spec.largeControlOrientation};`,
      `${indent}${controlLevers.labelLines}: 1;`,
      // The floor, not a number: `--mjx-hit-target` is itself a multiple of `--spacing` and the
      // density foundation clamps it to the accessible minimum.
      `${indent}${controlLevers.minInline}: var(${densityProperties.hitTarget});`,
      `${indent}${controlLevers.maxInline}: none;`,
    );
  }
  return lines.join('\n');
}

/**
 * One presentation rule's selector — **every one of them wrapped in `:where()`, and that is
 * load-bearing rather than a house style.**
 *
 * MJXOFF-183 shipped this file once with the simplified rule written as
 * `:host([simplified]) .group[data-priority='x']`, which scores (0,4,0) against the container
 * rules' (0,2,0). It therefore won at *every* width, and a simplified ribbon never collapsed a
 * group however narrow it got. Source order had nothing to do with it, and the unit test that
 * asserted the emission order stayed green throughout — because emission order only decides between
 * rules of equal specificity, and these were not.
 *
 * That is MJXOFF-181's specificity accident (`:where(…):focus:not(:focus-visible)` out-specifying
 * `:where(…):focus-visible`) in a third costume, and it was caught by the *correspondence* gate —
 * the one that compares the browser against `groupPresentationAt()` — rather than by anything that
 * merely checked the layout responded. `:where()` puts all four families at (0,0,0), which makes
 * `groupPresentationOrder` genuinely the only thing deciding.
 *
 * Simplified is matched on a `data-simplified` attribute the component mirrors onto `.group`,
 * rather than on `:host([simplified])`, so that no selector here needs `:host` inside `:where()`
 * and every one of them has the same shape.
 */
function groupSelector(
  priority: GroupPriority,
  options: { readonly simplified?: boolean } = {},
): string {
  const simplified = options.simplified === true ? '[data-simplified]' : '';
  return `:where(.group[data-priority='${priority}']${simplified})`;
}

/**
 * The group's presentation rules: one base block, one simplified block, and eight `@container`
 * blocks generated from the ladder.
 *
 * ⚠ **The order is the cascade**, and it is only the cascade because `groupSelector` puts every
 * rule at (0,0,0). `reduced` is emitted for every priority before `collapsed` is emitted for any,
 * which is what makes a group at a width below both conditions end up collapsed rather than
 * reduced. `tests/ribbon.test.ts` reads the emitted text and asserts both the ordering **and** that
 * every selector is wrapped, because the ordering assertion on its own was green while the bug
 * above was live.
 */
export function groupPresentationCss(): string {
  const blocks: string[] = [];

  for (const priority of groupPriorityNames) {
    blocks.push(`${groupSelector(priority)} {\n${presentationDeclarations('full', '  ')}\n}`);
  }

  // Simplified is Office's single-row ribbon. It refuses `full` and never prevents `collapsed`,
  // which is why it sits between the two container conditions rather than after both.
  for (const priority of groupPriorityNames) {
    blocks.push(
      `${groupSelector(priority, { simplified: true })} {\n` +
        `${presentationDeclarations('reduced', '  ')}\n}`,
    );
  }

  for (const priority of groupPriorityNames) {
    const width = groupPriorities[priority].reduceAtOrBelow;
    blocks.push(
      `@container ${ribbonContainerName} (width <= ${String(width)}px) {\n` +
        `  ${groupSelector(priority)} {\n${presentationDeclarations('reduced', '    ')}\n  }\n}`,
    );
  }

  for (const priority of groupPriorityNames) {
    const width = groupPriorities[priority].collapseAtOrBelow;
    blocks.push(
      `@container ${ribbonContainerName} (width <= ${String(width)}px) {\n` +
        `  ${groupSelector(priority)} {\n${presentationDeclarations('collapsed', '    ')}\n  }\n}`,
    );
  }

  return blocks.join('\n');
}

/**
 * The tab strip's two presentations, generated the same way and for the same reasons.
 *
 * The picker's popup is `surfaceLevels.overlay` again — the same rung the collapsed group's panel
 * uses, because they are the same kind of thing and a second hand-made card is how a design system
 * ends up with two menus that do not match.
 */
export function tabStripPresentationCss(): string {
  const overlay = surfaceLevels.overlay;
  return [
    `.strip {`,
    `  ${tabStripPresentationProperty}: strip;`,
    `  --mjx-picker-display: none;`,
    `  --mjx-tabs-position: static;`,
    `  --mjx-tabs-direction: row;`,
    `  --mjx-tabs-closed-display: flex;`,
    `  --mjx-tabs-background: transparent;`,
    `  --mjx-tabs-border: none;`,
    `  --mjx-tabs-shadow: none;`,
    `  --mjx-tabs-radius: 0;`,
    `  --mjx-tabs-padding: 0;`,
    `  --mjx-tabs-min-inline: auto;`,
    `}`,
    `@container ${ribbonContainerName} (width <= ${String(tabStripPickerAtOrBelow)}px) {`,
    `  .strip {`,
    `    ${tabStripPresentationProperty}: picker;`,
    `    --mjx-picker-display: inline-flex;`,
    `    --mjx-tabs-position: absolute;`,
    `    --mjx-tabs-direction: column;`,
    `    --mjx-tabs-closed-display: none;`,
    `    --mjx-tabs-background: ${overlay.background};`,
    `    --mjx-tabs-border: ${overlay.border};`,
    `    --mjx-tabs-shadow: ${overlay.shadow};`,
    `    --mjx-tabs-radius: ${radiusVariable(overlay.radius)};`,
    `    --mjx-tabs-padding: var(${densityProperties.gutter});`,
    `    --mjx-tabs-min-inline: ${spacingMultiple(collapsedPanelMinimumUnits)};`,
    `  }`,
    `}`,
  ].join('\n');
}

/**
 * The contextual tone rules — **the one place this child adds paint**, and it is generated from
 * `tabTones` so the gate can assert correspondence rather than distinctness.
 */
export function tabToneCss(): string {
  const contextual = tabTones.contextual;
  const core = tabTones.core;
  return [
    `.tab[${tabToneAttribute}='core'][data-pressed='true'] {`,
    `  background: ${themeVariable(core.selectedBackground)};`,
    `  border-color: ${themeVariable(core.selectedBorder)};`,
    `}`,
    `.tab[${tabToneAttribute}='contextual'][data-pressed='true'] {`,
    `  background: ${themeVariable(contextual.selectedBackground)};`,
    `  border-color: ${themeVariable(contextual.selectedBorder)};`,
    `}`,
    `.tab-set-title {`,
    contextual.bandBackground === undefined
      ? ''
      : `  background: ${themeVariable(contextual.bandBackground)};`,
    contextual.bandText === undefined ? '' : `  color: ${themeVariable(contextual.bandText)};`,
    `  border-block-end: 1px solid ${themeVariable(contextual.selectedBorder)};`,
    `}`,
  ]
    .filter((line) => line !== '')
    .join('\n');
}

/**
 * The group's own rules.
 *
 * ⚠ `.trigger` declares **no paint** — no background, no border colour, no text colour, no weight,
 * no opacity, no shadow — for exactly the reason `controlBaseCss` gives: `controlStatesCss` emits
 * every state at (0,0,0), so a single `background:` in a (0,1,0) rule here would out-specify the
 * whole state table and leave a button that renders its resting paint in all six states while every
 * "the state exists" check passed. `tests/ribbon.test.ts` asserts the base rule declares none of
 * the seven properties the table owns, the same way `tests/controls.test.ts` does for the four
 * archetypes.
 */
export const ribbonGroupCss = [
  `
  :host {
    display: inline-flex;
    vertical-align: top;
  }
  :host([hidden]) { display: none; }

  .group {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: var(--mjx-density-step);
    padding-inline: var(--mjx-density-gutter);
    padding-block: var(--mjx-density-step);
    border-inline-end: 1px solid var(--theme-border-subtle);
  }

  .row {
    display: flex;
    align-items: flex-start;
    gap: var(--mjx-density-step);
  }

  /* Paint comes from controlStatesCss('.trigger'). This rule owns the box and nothing else. */
  .trigger {
    display: var(--mjx-group-trigger-display, none);
    align-items: center;
    justify-content: center;
    gap: var(--mjx-density-step);
    box-sizing: border-box;
    margin: 0;
    padding-inline: var(--mjx-density-gutter);
    padding-block: 0;
    border-width: 1px;
    border-radius: var(--radius-control);
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    white-space: nowrap;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .essential {
    display: flex;
    align-items: center;
    gap: var(--mjx-density-step);
  }

  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--mjx-density-step);
    position: var(--mjx-group-panel-position, static);
    inset-block-start: 100%;
    inset-inline-start: 0;
    z-index: 1;
    box-sizing: border-box;
    min-inline-size: var(--mjx-group-panel-min-inline, auto);
    padding: var(--mjx-group-panel-padding, 0);
    background: var(--mjx-group-panel-background, transparent);
    border: var(--mjx-group-panel-border, none);
    /* The fallback is the ladder's own panel rung, not a square corner: a group used outside
     * a ribbon still gets a real radius. Every presentation sets the property, so the
     * fallback is what a group with no container at all falls back to. */
    border-radius: var(--mjx-group-panel-radius, var(--radius-panel));
    box-shadow: var(--mjx-group-panel-shadow, none);
  }

  /* Openness is an attribute; whether openness *means* anything is the presentation's decision.
   * In full and reduced the closed display is flex, so an 'open' attribute changes nothing there. */
  :host(:not([open])) .panel {
    display: var(--mjx-group-panel-closed-display, flex);
  }

  .commands {
    display: grid;
    grid-auto-flow: column;
    grid-template-rows: var(--mjx-group-command-template, repeat(1, auto));
    gap: var(--mjx-density-step);
    align-items: start;
    justify-items: start;
  }

  .panel-label {
    display: var(--mjx-group-panel-label-display, none);
    margin: 0;
    color: var(--theme-text-secondary);
  }

  .footer {
    display: var(--mjx-group-footer-display, flex);
    align-items: center;
    justify-content: center;
    gap: var(--mjx-density-step);
  }

  .label {
    color: var(--theme-text-secondary);
    white-space: nowrap;
  }

`,
  controlStatesCss('.trigger'),
  groupPresentationCss(),
].join('\n');

/**
 * The ribbon's own rules.
 *
 * ⚠ Neither `.tab` nor `.chrome-button` declares paint. Both take it from `controlStatesCss`, whose
 * rules score (0,0,0), so a `background:` in either box rule would out-specify the whole state
 * table — the specificity accident MJXOFF-181 found and MJXOFF-182 wrote down. The **one** place
 * this child adds paint of its own is `tabToneCss()`, which is generated from `tabTones` and
 * emitted last so a selected contextual tab reads as contextual rather than as pressed.
 */
export const ribbonCss = [
  `
  :host {
    display: block;
    /* ⚠ No padding and no border, ever. See the module note: this is the query container and a
     * container query resolves against its content box. */
    container-type: inline-size;
    container-name: ${ribbonContainerName};
    font-family: var(--font-sans);
    color: var(--theme-text-primary);
  }
  :host([hidden]) { display: none; }

  .ribbon {
    display: flex;
    flex-direction: column;
    background: var(--theme-surface);
    border-block-end: 1px solid var(--theme-border-subtle);
  }

  .strip {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--mjx-density-step);
    padding-inline: var(--mjx-density-gutter);
    padding-block: var(--mjx-density-step);
  }
  :host([state='hidden']) .strip { display: none; }

  .tabs {
    display: flex;
    flex-direction: var(--mjx-tabs-direction, row);
    align-items: stretch;
    gap: var(--mjx-density-step);
    position: var(--mjx-tabs-position, static);
    inset-block-start: 100%;
    inset-inline-start: 0;
    z-index: 2;
    box-sizing: border-box;
    min-inline-size: var(--mjx-tabs-min-inline, auto);
    padding: var(--mjx-tabs-padding, 0);
    background: var(--mjx-tabs-background, transparent);
    border: var(--mjx-tabs-border, none);
    border-radius: var(--mjx-tabs-radius, var(--radius-panel));
    box-shadow: var(--mjx-tabs-shadow, none);
  }
  :host(:not([picker-open])) .tabs {
    display: var(--mjx-tabs-closed-display, flex);
  }

  .tab-set {
    display: flex;
    flex-direction: column;
    gap: 0;
  }
  .tab-set-row {
    display: flex;
    flex-direction: var(--mjx-tabs-direction, row);
    gap: var(--mjx-density-step);
  }
  .tab-set-title {
    margin: 0;
    padding-inline: var(--mjx-density-gutter);
    padding-block: 0;
    border-start-start-radius: var(--radius-chip);
    border-start-end-radius: var(--radius-chip);
    text-align: center;
    white-space: nowrap;
  }

  /* Box only. Paint is controlStatesCss('.tab') plus tabToneCss(). */
  .tab, .chrome-button {
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
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
    white-space: nowrap;
    transition-property: background, border-color, color, box-shadow, opacity;
  }

  .picker { display: var(--mjx-picker-display, none); }
  .ribbon-toggle { margin-inline-start: auto; }

  .body {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-start;
    padding-inline: var(--mjx-density-gutter);
    padding-block: var(--mjx-density-step);
  }
  :host([state='tabs']) .body,
  :host([state='hidden']) .body { display: none; }

  .restore { display: none; margin: var(--mjx-density-gutter); }
  :host([state='hidden']) .restore { display: inline-flex; }

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
`,
  controlStatesCss('.tab'),
  controlStatesCss('.chrome-button'),
  tabStripPresentationCss(),
  tabToneCss(),
].join('\n');
