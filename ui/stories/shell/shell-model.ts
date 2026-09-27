/**
 * **The assembly's own model** — what the nine shells are, what may not appear inside one, and what
 * is deliberately absent from all of them.
 *
 * MJXOFF-274 states the trap this file exists against:
 *
 * > **A mock-up is not an assembly.** A shell rebuilt out of plain markup that *looks* like the
 * > product proves nothing about the components and will drift from them the moment either changes
 * > — and it is the easy thing to do under time pressure, because a real assembly surfaces awkward
 * > problems a mock-up hides.
 *
 * So the gate is compositional. `findAdHocSubstitutes` below is the rule, `tests/shell.test.ts`
 * runs it over the assembly's own sources before a browser starts, and
 * `tests/browser/shell.spec.ts` runs the *other* direction over the rendered nine.
 *
 * ## Node-importable
 *
 * Data and pure functions, for the reason `src/harness/presets.ts` states: a module that touches
 * the DOM cannot be imported by a Playwright spec or a vitest file, and both tiers read this.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import type { ContainerPreset } from '../../src/harness/presets.ts';
import { catalogueComponents } from '../../src/mobile/touch-audit.ts';
import type { KeyboardBehaviour, StoryState } from '../../src/story/conventions.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

// ── what the nine are ────────────────────────────────────────────────────────

/** The three applications, in the order the sidebar shows them. */
export const shellApplicationNames = ['powerpoint', 'word', 'excel'] as const;

/** One of the three. */
export type ShellApplication = (typeof shellApplicationNames)[number];

/**
 * The three sizes, and **the middle one is the one that breaks.**
 *
 * MJXOFF-274: *"Desktop and phone are both designed for; the tablet width is where a ribbon is
 * neither full nor collapsed and a task pane must decide whether to dock or overlay."*
 */
export const shellSizeNames = ['desktop', 'tablet', 'mobile'] as const;

/** One of the three. */
export type ShellSize = (typeof shellSizeNames)[number];

/**
 * Which harness preset each size is.
 *
 * The names differ by one word on purpose: a *shell* size is a design decision about which chrome
 * the application shows, and a *container preset* is a width. They happen to agree today, and a
 * later child that adds a fourth preset must not silently acquire a fourth shell.
 */
export const shellSizePreset: Readonly<Record<ShellSize, ContainerPreset>> = {
  desktop: 'desktop',
  tablet: 'tablet',
  mobile: 'phone',
};

/** The Storybook title each application's stories live under. One folder, three entries. */
export const shellStoryTitle: Readonly<Record<ShellApplication, string>> = {
  powerpoint: 'Shell/PowerPoint',
  word: 'Shell/Word',
  excel: 'Shell/Excel',
};

/**
 * The id the built catalogue gives one shell story.
 *
 * ⚠ Derived from the title and the export name the way Storybook derives it, and then **checked**:
 * `tests/browser/shell.spec.ts` looks every one of these up in `storybook-static/index.json` and
 * fails if it is not there. A hand-written id that had drifted would make the whole browser gate
 * skip silently, which is the *found nothing* failure this programme keeps meeting.
 */
export function shellStoryId(application: ShellApplication, size: ShellSize): string {
  return `shell-${application}--${size}`;
}

/** All nine, in reading order. */
export const shellStories: readonly {
  readonly application: ShellApplication;
  readonly size: ShellSize;
  readonly id: string;
}[] = shellApplicationNames.flatMap((application) =>
  shellSizeNames.map((size) => ({ application, size, id: shellStoryId(application, size) })),
);

// ── how the gates find things ────────────────────────────────────────────────

/**
 * The attribute every shell's outermost element carries, naming which application it is.
 *
 * The gates scan **inside** it and never outside, which is what makes the harness frame around the
 * story — `<mjx-resizable-container>`, a catalogued component — legitimately absent from the
 * assembly rather than accidentally present in all nine.
 */
export const shellRootAttribute = 'data-mjx-shell';

/**
 * The attribute a *named surface* carries — the ribbon band, the workspace, the navigator pane, the
 * status bar, the phone's command rail.
 *
 * `tests/browser/shell.spec.ts` requires each of these to be laid out inside the shell with a
 * non-trivial box. That is the *no clipped surface* half of MJXOFF-274's fifth condition, and it is
 * the half a horizontal-overflow check cannot see: a status bar pushed off the bottom of a
 * fixed-height shell overflows nothing.
 */
export const shellSurfaceAttribute = 'data-mjx-shell-surface';

/**
 * The smallest box a named surface may have and still be said to be *there*, in CSS pixels.
 *
 * A surface under the accessible hit-target floor on either axis has effectively been squeezed out
 * of the layout, whatever the DOM says.
 */
export const shellSurfaceMinimumExtent = 24;

// ── the no-mock-up rule ──────────────────────────────────────────────────────

/**
 * The raw HTML tags a shell may not open, and the component each of them would be standing in for.
 *
 * ⚠ **This is the whole of MJXOFF-274's third condition and it is deliberately blunt.** Every one
 * of these has a catalogued counterpart that has already been audited in isolation, so writing the
 * raw element in a shell is precisely the drift the ticket names: it looks right today, it is not
 * the thing the audit passed, and it stops matching the moment either changes.
 */
export const adHocSubstituteTags: Readonly<Record<string, string>> = {
  button: '<mjx-button>, <mjx-toggle-button>, <mjx-split-button> or <mjx-menu-item>',
  input: '<mjx-measure-input>, <mjx-checkbox>, <mjx-slider> or <mjx-combo-box>',
  select: '<mjx-dropdown>',
  textarea: '<mjx-formula-bar>',
  option: '<mjx-option>',
  optgroup: '<mjx-menu-section>',
  dialog: '<mjx-dialog>',
  menu: '<mjx-menu>',
  details: '<mjx-popover>',
  summary: '<mjx-popover>',
  progress: '<mjx-progress>',
  meter: '<mjx-progress>',
  fieldset: '<mjx-ribbon-group>',
  legend: '<mjx-label>',
  label: '<mjx-label>',
  hr: '<mjx-menu-separator>',
};

/**
 * ARIA attributes a shell may not write.
 *
 * A role or an interactive state written by the assembly is a control the assembly built, whatever
 * element it is on — so this catches the substitute that is a `<div>` rather than a `<button>`,
 * which is the form the temptation actually takes. `aria-label` is **not** here: naming a region is
 * the host's job and always has been.
 */
export const adHocAriaAttributes: readonly string[] = [
  'role',
  'aria-haspopup',
  'aria-expanded',
  'aria-pressed',
  'aria-checked',
  'aria-selected',
  'aria-controls',
  'aria-activedescendant',
  'aria-current',
  'aria-valuenow',
];

/** One thing the rule found, in a message a person can act on. */
export interface AdHocFinding {
  /** `tag` or `aria`. */
  readonly kind: 'tag' | 'aria';
  /** The tag or attribute name. */
  readonly name: string;
  /** 1-based, so the message reads like a compiler's. */
  readonly line: number;
  readonly advice: string;
}

/**
 * Strip comments and doc blocks, so an *explanation* of the rule is never mistaken for a breach.
 *
 * The same reasoning as `eslint-rules/design-values.js`: this very file names the raw elements it
 * forbids a dozen times, and a scanner that could not tell prose from markup would be a scanner
 * nobody could describe in words.
 *
 * ⚠ **Newlines survive.** A block comment is replaced by the newlines it spanned rather than by a
 * space, because this gate reports a **line number** and a comment-collapsing pass would point a
 * reader at the wrong one — which is worse than reporting none, since the line it names would look
 * innocent and the reader would conclude the gate was broken.
 */
function withoutComments(source: string): string {
  return source
    .replaceAll(/\/\*[\s\S]*?\*\//g, (comment) => '\n'.repeat(comment.split('\n').length - 1))
    .replaceAll(/(^|[^:\w])\/\/[^\n]*/gm, '$1 ');
}

/**
 * **The rule.** Every ad-hoc substitute in one source file.
 *
 * Pure, so `tests/shell.test.ts` can prove it able to fail against hand-written specimens as well
 * as running it over the real assembly — a gate nobody has watched reject anything is a gate that
 * might be matching nothing, which is this catalogue's oldest lesson.
 */
export function findAdHocSubstitutes(source: string): AdHocFinding[] {
  const cleaned = withoutComments(source);
  const findings: AdHocFinding[] = [];
  const lineOf = (index: number): number => cleaned.slice(0, index).split('\n').length;

  for (const [tag, advice] of Object.entries(adHocSubstituteTags)) {
    // An opening tag: `<button`, `<button>`, `<button ` — and never `<mjx-button`, because the
    // boundary before the name is required.
    const pattern = new RegExp(`<${tag}(?=[\\s/>])`, 'g');
    for (const match of cleaned.matchAll(pattern)) {
      findings.push({
        kind: 'tag',
        name: tag,
        line: lineOf(match.index),
        advice: `a shell composes ${advice} rather than opening a raw <${tag}>`,
      });
    }
  }

  for (const attribute of adHocAriaAttributes) {
    const pattern = new RegExp(`(?<![\\w-])${attribute}\\s*=`, 'g');
    for (const match of cleaned.matchAll(pattern)) {
      findings.push({
        kind: 'aria',
        name: attribute,
        line: lineOf(match.index),
        advice:
          `a shell never writes ${attribute}: the component that owns the behaviour owns the ` +
          'accessibility tree, and an assembly that wrote one has built a control of its own',
      });
    }
  }

  return findings.sort((left, right) => left.line - right.line);
}

/** How the gate says what it found. */
export function describeAdHocFinding(file: string, finding: AdHocFinding): string {
  const what = finding.kind === 'tag' ? `<${finding.name}>` : finding.name;
  return `${file}:${String(finding.line)} — ${what}: ${finding.advice}.`;
}

// ── coverage, in both directions ─────────────────────────────────────────────

/**
 * **The components deliberately absent from every shell, and why each is.**
 *
 * MJXOFF-274's fourth condition: *"every component the catalogue ships appears in at least one
 * shell, or is named as deliberately absent with a reason. A component that appears in no assembly
 * has never been seen in context."*
 *
 * ⚠ **Both halves are gated.** `tests/browser/shell.spec.ts` requires every catalogued tag that is
 * *not* named here to appear in the rendered DOM of at least one of the nine, and requires every
 * tag that *is* named here to appear in none of them — because a reason attached to a component
 * that is actually present is a reason that has quietly become a lie, and it is the half a
 * one-directional check cannot see.
 */
export const absentFromShells: Readonly<Record<string, string>> = {
  'mjx-resizable-container':
    'The audit frame. It is the harness the story renders *inside*, not chrome the application ' +
    'ships — the shells are scanned within their own root for exactly this reason, so a frame ' +
    'that surrounds all nine is absent from all nine.',
  'mjx-plate-gallery':
    'A developer surface: the loader for the render oracle’s plate manifest. It reviews the ' +
    'output of `cargo run -p mjx-render-oracle -- gallery` and never reaches a person using the ' +
    'product, so an assembly that showed it would be showing something the product does not have.',
};

/** Every catalogued tag that must therefore be found in the rendered assembly. */
export const tagsRequiredInShells: readonly string[] = catalogueComponents
  .map((component) => component.tag)
  .filter((tag) => !(tag in absentFromShells));

/**
 * The smallest number of distinct catalogued components the nine shells must render between them.
 *
 * An anti-vacuity floor, and the reason it is written rather than derived: the gate above already
 * requires *every* required tag, so this number can only fire when the scan itself has broken —
 * which is precisely the failure that would otherwise report a green run over an empty set.
 */
export const minimumDistinctComponents = 40;

// ── what every shell story declares ──────────────────────────────────────────

/**
 * **The states matrix a shell publishes**, and it is not a component's.
 *
 * A component's matrix lists the states *it* can be in. An assembly's lists what a reviewer must be
 * able to reach, because MJXOFF-274's whole argument is that the composition is only judgeable when
 * every component's own interaction is live: *"a menu that cannot open cannot be judged, and a
 * hover state nobody can reach cannot be assessed for cosmetics."*
 */
export const shellStatesMatrix: readonly StoryState[] = [
  {
    name: 'at rest',
    description:
      'The whole application, nothing open. This is the state the cosmetic audit is mostly ' +
      'about: spacing that reads generous around one button, seen across thirteen ribbon groups.',
  },
  {
    name: 'a menu open',
    description:
      'Press the split button’s arrow, or right-click the document surface. The menu is the ' +
      'catalogue’s own, opening over the assembled chrome — which is the only way its elevation ' +
      'can be judged against what is behind it.',
  },
  {
    name: 'a dialog open',
    description:
      'Press a ribbon group’s dialog launcher. A modal, with its scrim over the assembled shell.',
  },
  {
    name: 'a pane resized',
    description:
      'Drag either divider, or Tab to one and use the arrow keys. Both the navigation pane and ' +
      'the task pane are resizable, and where they stop is a decision this assembly makes.',
  },
  {
    name: 'the ribbon narrowed',
    description:
      'Take the container from desktop to tablet to phone. Groups collapse in priority order, the ' +
      'tab strip becomes a picker, and no command is ever removed.',
  },
  {
    name: 'a gallery previewing',
    description: 'Hover a style or a theme. The preview session is the component’s own.',
  },
  {
    name: 'commands inert',
    description:
      'Every control lights up and nothing happens to a document. Command dispatch, document ' +
      'binding, the ShellBridge and the real canvas are all loop 2, and the placeholder says so.',
  },
];

/** What a keyboard does in an assembly: the components’, plus the order they are reached in. */
export const shellKeyboard: readonly KeyboardBehaviour[] = [
  {
    keys: 'Tab',
    does:
      'Walks the shell in reading order — the harness stage first, then the ribbon’s one tab ' +
      'stop, then the navigator, the document, the pane and the status bar. Every component ' +
      'contributes the stops its own story declares and no more.',
  },
  {
    keys: 'Arrow Left / Arrow Right',
    does: 'Moves along the ribbon’s tab strip, selecting as it goes.',
  },
  { keys: 'Escape', does: 'Closes whichever surface is topmost and returns the keyboard to it.' },
  {
    keys: 'Shift + F10 or the Context Menu key',
    does:
      'Opens the document surface’s context menu where the keyboard is, rather than where a ' +
      'pointer was last left.',
  },
  {
    keys: 'Arrow keys on a divider',
    does: 'Resizes the pane it controls; Enter collapses it and restores it to where it was.',
  },
];

/** What a screen reader meets in an assembly. */
export const shellScreenReader =
  'Nothing here announces anything of its own: every name, role and live region in the shell ' +
  'belongs to the component that owns the behaviour, which is what the no-mock-up gate enforces. ' +
  'What an assembly adds is order — the ribbon before the document, the document before the pane, ' +
  'the status bar last — and the one thing it deliberately does not add is a landmark. See ' +
  'ui/README.md on why the assembly declares none and what that costs.';

/** The tokens a change would move across every shell. Checked against the generated table. */
export const shellTokenDependencies: readonly TokenPath[] = [
  'theme.light.background',
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'theme.light.accent',
  'theme.light.accentSurface',
  'theme.light.accentBorder',
  'theme.light.accentPressed',
  'theme.dark.background',
  'theme.dark.surface',
  'theme.dark.surfaceRaised',
  'theme.dark.textPrimary',
  'theme.dark.textSecondary',
  'radius.card',
  'radius.panel',
  'radius.control',
  'radius.chip',
  'spacing',
  'duration.transition',
  'text.xs',
  'text.sm',
  'font.sans',
  'font.mono',
];
