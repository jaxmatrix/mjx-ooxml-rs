/**
 * Word's `TabHome`, as the committed command surface actually records it — **the realistic worst
 * case MJXOFF-183 asks the ribbon to survive.**
 *
 * ## Why this file exists, and why it is checked against the source
 *
 * A worst case that was typed out by hand is a worst case somebody chose, and it drifts the moment
 * the census is re-derived. So the group names and control counts here are transcribed from
 * `docs/client-platform/data/command-surface.tsv`, and `tests/ribbon.test.ts` **reads that file and
 * fails if they disagree** — the same doctrine as `npm run tokens:check`. The transcription exists
 * at all because `ui/` is a Node workspace that must not grow a build step reading a document tree,
 * and because a story cannot read a TSV at render time without one.
 *
 * ## ⚠ The census disagrees with the ticket, and the census wins
 *
 * MJXOFF-183 says *"Word's `TabHome` alone is 165 controls across eight groups"*. The committed TSV
 * says **152 controls across thirteen groups**, of which 140 across six are in scope, and
 * `OFFICE_FEATURE_INVENTORY.md` §4.1 gives 165 while listing only the five principal groups. The
 * three numbers are not reconcilable from anything in this repository, so this file uses the one
 * that is *checked* — the TSV — and says so rather than quietly matching the ticket. Thirteen
 * groups is the harder case anyway: seven of them are one- and two-control groups, which is
 * exactly the shape a uniform collapse rule gets wrong.
 *
 * ## The commands
 *
 * `named` are the real Word commands, in the order Word shows them, as far as the icon subset can
 * draw them. The remainder of each group's count is filled with numbered placeholders, because
 * **deciding which real command goes in which group is loop 2** and a made-up command name would be
 * a worse lie than an obvious placeholder. What the story is for is the *layout* at 152 controls,
 * and a placeholder occupies exactly as much of it as a command does.
 *
 * ## Node-importable
 *
 * Data only, for the reason `src/harness/presets.ts` states.
 */

import type { ControlSize } from '../src/controls/control-states.ts';
import type { GroupPriority } from '../src/ribbon/ribbon-model.ts';

/** One command in the specimen. */
export interface SpecimenCommand {
  readonly label: string;
  /** A name from `src/icons/manifest.ts`, or `undefined` for a text-only command. */
  readonly icon?: string;
  readonly size?: ControlSize;
  /** A toggle rather than a one-shot verb. */
  readonly toggle?: boolean;
  /** A command that survives a collapse. See `demotionRules`. */
  readonly essential?: boolean;
}

/** One group in the specimen. */
export interface SpecimenGroup {
  /** The group's id in the census — `GroupFont`. */
  readonly id: string;
  /** What the ribbon draws. */
  readonly label: string;
  /** The census's control count for this group. The specimen renders exactly this many. */
  readonly controls: number;
  /** Whether the census marks it in scope for this project. */
  readonly inScope: boolean;
  readonly priority: GroupPriority;
  /** Whether the group has a dialog launcher. `GUESS:` not checked against Office. */
  readonly dialogLauncher: boolean;
  readonly named: readonly SpecimenCommand[];
}

/** Which application, tab set and tab the census rows below come from. */
export const wordTabHomeSource = {
  app: 'Word',
  tabSet: 'None (Core Tab)',
  tab: 'TabHome',
  file: 'docs/client-platform/data/command-surface.tsv',
} as const;

/**
 * The thirteen groups.
 *
 * The **priorities** are this child's design decision and the thing the audit is being shown: Font
 * and Paragraph are what a person opened Home for, so they give way last; Clipboard's four verbs
 * are on the keyboard anyway, so it goes early; and the seven small groups that are nobody's reason
 * for opening Home are ancillary and collapse while there is still room.
 */
export const wordTabHomeGroups: readonly SpecimenGroup[] = [
  {
    id: 'GroupClipboard',
    label: 'Clipboard',
    controls: 12,
    inScope: true,
    priority: 'secondary',
    dialogLauncher: true,
    named: [
      { label: 'Paste', icon: 'folder-open', size: 'large' },
      { label: 'Cut', icon: 'delete' },
      { label: 'Copy', icon: 'document' },
      { label: 'Format Painter', icon: 'settings' },
    ],
  },
  {
    id: 'GroupFont',
    label: 'Font',
    controls: 43,
    inScope: true,
    priority: 'primary',
    dialogLauncher: true,
    named: [
      { label: 'Bold', icon: 'text-bold', size: 'icon', toggle: true, essential: true },
      { label: 'Italic', icon: 'text-italic', size: 'icon', toggle: true, essential: true },
      { label: 'Underline', icon: 'text-underline', size: 'icon', toggle: true, essential: true },
      { label: 'Clear All Formatting', icon: 'dismiss' },
      { label: 'Grow Font', icon: 'add' },
      { label: 'Change Case', icon: 'text-align-left' },
      { label: 'Text Highlight Colour', icon: 'comment' },
    ],
  },
  {
    id: 'GroupParagraph',
    label: 'Paragraph',
    controls: 56,
    inScope: true,
    priority: 'primary',
    dialogLauncher: true,
    named: [
      { label: 'Align Left', icon: 'text-align-left', size: 'icon', toggle: true, essential: true },
      { label: 'Centre', icon: 'text-align-center', size: 'icon', toggle: true, essential: true },
      { label: 'Align Right', icon: 'text-align-right', size: 'icon', toggle: true, essential: true },
      { label: 'Bullets', icon: 'add' },
      { label: 'Sort', icon: 'chevron-down' },
      { label: 'Borders', icon: 'table' },
    ],
  },
  {
    id: 'GroupStyles',
    label: 'Styles',
    controls: 5,
    inScope: true,
    priority: 'standard',
    dialogLauncher: true,
    named: [
      { label: 'Normal' },
      { label: 'No Spacing' },
      { label: 'Heading 1' },
      { label: 'Heading 2' },
      { label: 'Title' },
    ],
  },
  {
    id: 'GroupEditing',
    label: 'Editing',
    controls: 23,
    inScope: true,
    priority: 'standard',
    dialogLauncher: false,
    named: [
      { label: 'Find', icon: 'search', size: 'icon', essential: true },
      { label: 'Replace', icon: 'checkmark' },
      { label: 'Select', icon: 'chevron-right' },
    ],
  },
  {
    id: 'GroupEditor',
    label: 'Editor',
    controls: 1,
    inScope: true,
    priority: 'secondary',
    dialogLauncher: false,
    named: [{ label: 'Editor', icon: 'checkmark', size: 'large' }],
  },
  {
    id: 'GroupVoiceTools',
    label: 'Voice',
    controls: 4,
    inScope: false,
    priority: 'ancillary',
    dialogLauncher: false,
    named: [{ label: 'Dictate', icon: 'comment', size: 'large' }],
  },
  {
    id: 'GroupAIAssistance',
    label: 'Copilot',
    controls: 2,
    inScope: false,
    priority: 'ancillary',
    dialogLauncher: false,
    named: [{ label: 'Copilot', icon: 'info', size: 'large' }],
  },
  {
    id: 'GroupOfficeExtensionsAddinFlyout',
    label: 'Add-ins',
    controls: 2,
    inScope: false,
    priority: 'ancillary',
    dialogLauncher: false,
    named: [{ label: 'Add-ins', icon: 'add' }],
  },
  {
    id: 'GroupAIDemo',
    label: 'Demo',
    controls: 1,
    inScope: false,
    priority: 'ancillary',
    dialogLauncher: false,
    named: [{ label: 'Demo', icon: 'slide-layout' }],
  },
  {
    id: 'GroupActivation',
    label: 'Activation',
    controls: 1,
    inScope: false,
    priority: 'ancillary',
    dialogLauncher: false,
    named: [{ label: 'Activate', icon: 'warning' }],
  },
  {
    id: 'GroupClassifyLabelProtect',
    label: 'Sensitivity',
    controls: 1,
    inScope: false,
    priority: 'ancillary',
    dialogLauncher: false,
    named: [{ label: 'Sensitivity', icon: 'warning' }],
  },
  {
    id: 'GroupResearch',
    label: 'Research',
    controls: 1,
    inScope: false,
    priority: 'ancillary',
    dialogLauncher: false,
    named: [{ label: 'Researcher', icon: 'search' }],
  },
];

/** Every command the specimen renders for one group: the named ones, then the placeholders. */
export function specimenCommands(group: SpecimenGroup): readonly SpecimenCommand[] {
  const filler: SpecimenCommand[] = [];
  for (let index = group.named.length; index < group.controls; index += 1) {
    filler.push({ label: `${group.label} command ${String(index + 1)}` });
  }
  return [...group.named, ...filler];
}

/** How many controls the whole specimen renders. */
export const wordTabHomeControlCount = wordTabHomeGroups.reduce(
  (total, group) => total + group.controls,
  0,
);

/** The tabs the worst-case story shows beside Home, so the tab strip has something to overflow. */
export const wordCoreTabs: readonly { readonly id: string; readonly label: string }[] = [
  { id: 'file', label: 'File' },
  { id: 'home', label: 'Home' },
  { id: 'insert', label: 'Insert' },
  { id: 'draw', label: 'Draw' },
  { id: 'design', label: 'Design' },
  { id: 'layout', label: 'Layout' },
  { id: 'references', label: 'References' },
  { id: 'mailings', label: 'Mailings' },
  { id: 'review', label: 'Review' },
  { id: 'view', label: 'View' },
];

/**
 * Three of the contextual tab sets, with their real tabs.
 *
 * Three because the ticket asks the general mechanism to be *"demonstrated with at least three of
 * the twenty-one sets"*. The committed TSV records **thirty-three** distinct `TabSet*` names, not
 * twenty-one, which is another place the ticket and the census disagree; the requirement is
 * unchanged and larger.
 */
export const contextualTabSets: readonly {
  readonly label: string;
  readonly tabs: readonly { readonly id: string; readonly label: string }[];
}[] = [
  {
    label: 'Table Tools',
    tabs: [
      { id: 'table-design', label: 'Design' },
      { id: 'table-layout', label: 'Layout' },
    ],
  },
  {
    label: 'Picture Tools',
    tabs: [{ id: 'picture-format', label: 'Format' }],
  },
  {
    label: 'Chart Tools',
    tabs: [
      { id: 'chart-design', label: 'Design' },
      { id: 'chart-format', label: 'Format' },
    ],
  },
];
