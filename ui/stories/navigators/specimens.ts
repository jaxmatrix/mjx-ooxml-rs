/**
 * The navigator specimens — and the **five-thousand-item fixtures**, which are the point.
 *
 * MJXOFF-191's trap, in the ticket's own words:
 *
 * > **Virtualisation is invisible on a short list.** A twenty-item story passes under any
 * > implementation, so the fixtures must be thousands of items and the assertion must be on *node
 * > count*, not appearance.
 *
 * So `largeOutline`, `largeDeck` and `largeList` below are built from `largeNavigatorCount`, which
 * is the number the gates read too. A story that shipped twenty rows would be a story under which
 * every implementation in the world is correct.
 *
 * The second half of the ticket's trap needs a story to *do* something rather than to *be*
 * something — *"scroll-position stability when items change above the viewport … never appears in a
 * static story"* — so each of the three carries a button that inserts above the visible range while
 * the reader is looking at a named row, and reports both the offset it took and the offset it would
 * have taken had it simply kept the old one.
 *
 * Not a story file: `stories/**` is globbed for `*.stories.ts`, so this is never indexed.
 */

import { html, type TemplateResult } from 'lit';

import { typeRoleClass } from '../../src/foundations/typography.ts';
import {
  navigatorAriaPatterns,
  navigatorOrder,
  sheetNameForbidden,
  sheetNameMaximum,
  type RailSlide,
  type SheetTab,
  type TreeNode,
} from '../../src/navigators/navigator-model.ts';
import type { VirtualItem } from '../../src/navigators/virtual-list.ts';
import type { StoryState } from '../../src/story/conventions.ts';
import type { TokenPath } from '../../src/tokens/resolver.ts';

/**
 * How many items every large fixture holds.
 *
 * ⚠ **Read by the stories and by both test tiers.** U06 used four hundred and said why; this is an
 * order of magnitude more, because three of these four are collections a real document genuinely
 * produces at this size — a hundred-page report's outline, a conference deck, a workbook's audit
 * log — and because the assertion that matters is *the DOM holds a screenful*, which only means
 * something when the collection is far larger than a screenful.
 */
export const largeNavigatorCount = 5000;

/** A note above a specimen, so an auditor knows what they are looking at. */
export function note(text: string): TemplateResult {
  return html`
    <p
      class=${typeRoleClass('body')}
      style="margin:0;padding:calc(var(--mjx-density-gutter) * 2);color:var(--theme-text-primary);max-inline-size:64ch"
    >
      ${text}
    </p>
  `;
}

/** A caption under a specimen, in the measured-figure style the catalogue uses. */
export function caption(lines: readonly string[]): TemplateResult {
  return html`
    <ul
      class=${typeRoleClass('dense')}
      style="margin:0;padding:var(--mjx-density-gutter) calc(var(--mjx-density-gutter) * 3);
             color:var(--theme-text-secondary)"
    >
      ${lines.map((line) => html`<li>${line}</li>`)}
    </ul>
  `;
}

/** A fixed-height stage, so a scrolling navigator has a viewport smaller than its contents. */
export function stage(...content: TemplateResult[]): TemplateResult {
  return html`
    <div
      style="block-size:38rem;display:flex;flex-direction:column;min-block-size:0;
             gap:var(--mjx-density-gutter);padding:calc(var(--mjx-density-gutter) * 2);
             background:var(--theme-background)"
    >
      ${content}
    </div>
  `;
}

/**
 * How wide a navigation pane is.
 *
 * ⚠ **Not decoration, and the browser gate is the reason it is written down.** A navigator is a
 * *pane*, and at a full desktop width nothing in it ever wraps — so a story that let it span the
 * frame would have rows that are all exactly one line tall, the virtualiser's binary search would
 * be indistinguishable from the division it replaced, and *"the rows are genuinely of different
 * heights"* would be a claim nothing could check. The first version of these stories did exactly
 * that and the gate reported one distinct row height out of a five-thousand-heading outline.
 */
export const navigatorPaneInline = '22rem';

/**
 * The style a navigator wears inside a stage: a pane's width, and all the height there is.
 *
 * ⚠ **No corner here, deliberately.** The radius is the component's, because it is what *changes*
 * between the two presentations — a navigator that is the whole phone has no outside corner to
 * round. An inline radius written by a shell would beat the phone rule and the presentation would be
 * declared and never reached.
 *
 * The **width** is the shell's, and that is the honest division: the component publishes which
 * presentation it is in and a shell lays out accordingly, exactly as a ribbon group publishes its
 * own. Wiring a real shell to it is loop 2.
 */
export const paneStyle =
  `flex:1 1 auto;min-block-size:0;inline-size:${navigatorPaneInline};max-inline-size:100%;` +
  'border:1px solid var(--theme-border)';

/** The token dependencies every navigator story declares. One list, four components. */
export const navigatorTokenDependencies: readonly TokenPath[] = [
  'theme.light.background',
  'theme.light.surface',
  'theme.light.surfaceRaised',
  'theme.light.border',
  'theme.light.borderSubtle',
  'theme.light.textPrimary',
  'theme.light.textSecondary',
  'theme.light.accent',
  'theme.light.accentSurface',
  'theme.light.accentPressed',
  'theme.light.secondaryAccent',
  'theme.dark.background',
  'theme.dark.surfaceRaised',
  'theme.dark.textPrimary',
  'theme.dark.textSecondary',
  'theme.dark.accentPressed',
  'radius.chip',
  'radius.control',
  'spacing',
  'duration.transition',
  'text.xs',
  'text.sm',
  'font.sans',
];

// ── the fixtures ─────────────────────────────────────────────────────────────

/** The section a slide at `index` belongs to. Twelve slides per section, which is a real deck. */
function sectionOf(index: number): string {
  return `Section ${String(Math.floor(index / 12) + 1)}`;
}

/**
 * A five-thousand-node outline, three levels deep.
 *
 * The labels are deliberately of different lengths, and every seventh is long enough to wrap in a
 * pane-width navigator.
 * **That is not decoration**: it is what makes the rows genuinely variable-height, which is the
 * precondition U07 had to hold by hand and which this child's virtualiser removes by measuring.
 * A fixture of uniform one-line rows would exercise the binary search and the division identically.
 */
export function largeOutline(count = largeNavigatorCount): TreeNode[] {
  const roots: TreeNode[] = [];
  let made = 0;
  let chapter = 0;
  while (made < count) {
    chapter += 1;
    const children: TreeNode[] = [];
    for (let section = 1; section <= 6 && made + children.length + 1 < count; section += 1) {
      const leaves: TreeNode[] = [];
      for (let leaf = 1; leaf <= 3; leaf += 1) {
        const ordinal = made + children.length + leaves.length + 2;
        leaves.push({
          id: `n${String(ordinal)}`,
          label:
            ordinal % 7 === 0
              ? `${String(chapter)}.${String(section)}.${String(leaf)} A heading long enough to wrap onto a second line in a narrow navigation pane, which is what makes these rows genuinely different heights`
              : `${String(chapter)}.${String(section)}.${String(leaf)} Detail`,
        });
      }
      children.push({
        id: `s${String(chapter)}-${String(section)}`,
        label: `${String(chapter)}.${String(section)} Section`,
        children: leaves,
      });
    }
    roots.push({ id: `c${String(chapter)}`, label: `${String(chapter)}. Chapter`, children });
    made += 1 + children.length + children.reduce((sum, child) => sum + (child.children?.length ?? 0), 0);
  }
  return roots;
}

/** A small outline, for the stories that are about behaviour rather than about scale. */
export const smallOutline: readonly TreeNode[] = [
  {
    id: 'intro',
    label: 'Introduction',
    children: [
      { id: 'intro-scope', label: 'Scope' },
      { id: 'intro-terms', label: 'Terms and definitions' },
    ],
  },
  {
    id: 'method',
    label: 'Method',
    children: [
      {
        id: 'method-corpus',
        label: 'The corpus',
        children: [
          { id: 'method-corpus-a', label: 'Provenance' },
          { id: 'method-corpus-b', label: 'Exclusions' },
        ],
      },
      { id: 'method-oracle', label: 'The oracle' },
    ],
  },
  { id: 'results', label: 'Results' },
  { id: 'appendix', label: 'Appendix' },
];

/** Which branches the small outline opens with. */
export const smallOutlineExpanded: readonly string[] = ['intro', 'method', 'method-corpus'];

/**
 * A five-thousand-slide deck, sectioned, one in nine hidden, and **every thumbnail absent**.
 *
 * The plates are absent on purpose: R10's generator is asynchronous, so *waiting for a picture* is
 * the state a rail is in most of the time it is being built, and a fixture whose images were
 * already there would never reach it. `deckWithPlates` is the other half.
 */
export function largeDeck(count = largeNavigatorCount): RailSlide[] {
  return Array.from({ length: count }, (_unused, index) => ({
    id: `slide-${String(index + 1)}`,
    label:
      index % 17 === 0
        ? `A slide whose title is long enough to wrap in the rail's caption column`
        : `Slide ${String(index + 1)}`,
    section: sectionOf(index),
    ...(index % 9 === 4 ? { hidden: true } : {}),
  }));
}

/** A short deck whose plates have arrived, so the two states can be seen side by side. */
export function deckWithPlates(): RailSlide[] {
  return Array.from({ length: 8 }, (_unused, index) => ({
    id: `plated-${String(index + 1)}`,
    label: `Slide ${String(index + 1)}`,
    section: index < 4 ? 'Opening' : 'Findings',
    ...(index === 5 ? { hidden: true } : {}),
    ...(index % 3 === 2 ? {} : { thumbnail: samplePlate(index) }),
  }));
}

/**
 * Every id in a large outline, so a story can open the whole thing.
 *
 * ⚠ **Opening only the chapters is not enough, and the browser gate is why.** The wrapping labels
 * in `largeOutline` are on the *leaves*, so a tree with only its chapters and sections open shows
 * nothing but short rows — and *"the rows are genuinely of different heights"* reported **one
 * distinct height out of five thousand headings**. A fixture that cannot reach its own hard case is
 * a fixture that makes every assertion over it weaker than it looks.
 */
export function everyBranch(nodes: readonly TreeNode[]): string[] {
  const ids: string[] = [];
  const walk = (node: TreeNode): void => {
    if ((node.children ?? []).length === 0) return;
    ids.push(node.id);
    for (const child of node.children ?? []) walk(child);
  };
  for (const node of nodes) walk(node);
  return ids;
}

/** A plate for every slide, which is what *the renderer finished* looks like. */
export function everyPlate(count: number): string[] {
  return Array.from({ length: count }, (_unused, index) => samplePlate(index % 8));
}

/**
 * A plate, drawn as a data URI.
 *
 * ⚠ **Not a fetched image, and that is a decision rather than a convenience.** `public/plates/` is a
 * committed fixture and the real plates come from `cargo run -p mjx-render-oracle -- gallery`, which
 * links the platform's graphics stack; a story that reached for a file over HTTP would also make the
 * placeholder state depend on network timing, which is the one thing the placeholder story must
 * control. An inline SVG arrives synchronously and the *absence* of one is what the pending rows
 * demonstrate.
 */
function samplePlate(index: number): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 160 90">` +
    `<rect width="160" height="90" fill="#f6f3ec"/>` +
    `<rect x="12" y="16" width="${String(60 + index * 8)}" height="10" fill="#2e9e63"/>` +
    `<rect x="12" y="36" width="120" height="6" fill="#c9c2b4"/>` +
    `<rect x="12" y="48" width="100" height="6" fill="#c9c2b4"/>` +
    `</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

/** A five-thousand-row list — a comment thread, a change log, a search result set. */
export function largeList(count = largeNavigatorCount): VirtualItem[] {
  return Array.from({ length: count }, (_unused, index) => ({
    id: `row-${String(index + 1)}`,
    label:
      index % 23 === 0
        ? `A comment long enough to wrap onto a second line, which is what makes the rows in this list different heights from one another`
        : `Comment ${String(index + 1)}`,
    detail: index % 2 === 0 ? 'Ada' : 'Grace',
    ...(index === 3 ? { unavailable: 'This comment was deleted by its author.' } : {}),
  }));
}

/** A workbook's sheets, with two colours, one hidden sheet and enough tabs to overflow. */
export function workbookTabs(): SheetTab[] {
  return [
    { id: 'summary', label: 'Summary', colour: 'var(--color-green)' },
    { id: 'q1', label: 'Q1 Actuals' },
    { id: 'q2', label: 'Q2 Actuals' },
    { id: 'q3', label: 'Q3 Actuals' },
    { id: 'q4', label: 'Q4 Actuals', colour: 'var(--color-honey)' },
    { id: 'assumptions', label: 'Assumptions' },
    { id: 'workings', label: 'Workings', hidden: true },
    { id: 'lookup', label: 'Lookup Tables' },
    { id: 'audit', label: 'Audit Trail' },
    { id: 'notes', label: 'Notes' },
  ];
}

// ── the states matrices ──────────────────────────────────────────────────────

/** Every navigator's ARIA row, drawn from the table rather than typed. */
export function patternStates(): StoryState[] {
  return navigatorOrder.map((name) => ({
    name: `${name}: ${navigatorAriaPatterns[name].container}`,
    description: navigatorAriaPatterns[name].why,
  }));
}

/** What an auditor must be able to see on the shared list. */
export const virtualListStates: readonly StoryState[] = [
  { name: 'resting', description: 'Five thousand rows, and a screenful of elements in the DOM.' },
  { name: 'selected', description: 'One row chosen. The accent tint survives every scroll, because the selection is a value on the component rather than a class on a row.' },
  { name: 'cursor', description: 'The keyboard is on a row and the container has focus, so the row carries the ring. Focus never leaves the container: this is the aria-activedescendant pattern.' },
  { name: 'unavailable', description: 'Reachable by arrow key, announced, never chosen — a deleted comment whose reason a person must be able to read.' },
  { name: 'items inserted above', description: 'A thousand rows arrive above the visible range and the row being read does not move.' },
];

/** What an auditor must be able to see on the tree. */
export const treeStates: readonly StoryState[] = [
  { name: 'collapsed branch', description: 'A heading with children and none of them showing. aria-expanded is false.' },
  { name: 'expanded branch', description: 'The same heading, open. Its children are indented by one gutter per level.' },
  { name: 'leaf', description: 'No children at all, and therefore NO aria-expanded — announcing false over a heading with nothing under it would offer a branch that does not exist.' },
  { name: 'selected', description: 'The chosen heading.' },
  { name: 'cursor', description: 'The keyboard row, with the tree focused.' },
  { name: 'reordered', description: 'Alt + Arrow Up moves a heading among its siblings AND carries its subtree with it. Announced, with the number carried.' },
  { name: 'refused', description: 'A heading already at the top of its branch. The refusal is announced rather than being a key that does nothing.' },
];

/** What an auditor must be able to see on the rail. */
export const railStates: readonly StoryState[] = [
  { name: 'pending', description: 'The plate has not arrived. A placeholder band, aria-busy on the frame, and the row is still selectable, scrollable and reorderable.' },
  { name: 'ready', description: 'The plate arrived and replaced the placeholder.' },
  { name: 'hidden', description: 'A hidden slide — marked with a glyph and named as hidden. NOT dimmed: an opacity says nothing to a screen reader and, measured, drops the label to 2.79 : 1 against a floor of 4.5.' },
  { name: 'selected', description: 'One slide chosen.' },
  { name: 'multi-selected', description: 'Several chosen with Ctrl and Shift, and they move as a block.' },
  { name: 'section break', description: 'A section heading between two slides, which is a row of a different height from the ones around it.' },
  { name: 'reordered', description: 'Alt + Arrow Down moves the whole selection down one and announces where it landed.' },
];

/** What an auditor must be able to see on the tab bar. */
export const tabBarStates: readonly StoryState[] = [
  { name: 'selected', description: 'The showing sheet.' },
  { name: 'coloured', description: 'A per-sheet colour, drawn as a bar along the tab’s edge and never as a fill behind the label.' },
  { name: 'hidden', description: 'A hidden sheet — a dashed edge, a glyph and the word hidden in its name. Not dimmed, for the reason the rail states.' },
  { name: 'overflowing', description: 'More tabs than the strip can show, with the four scroll affordances — which are deliberately NOT tabs.' },
  { name: 'renaming', description: 'The label became a field. Enter commits, Escape cancels.' },
  { name: 'rename refused', description: `A name Excel would not take — empty, over ${String(sheetNameMaximum)} characters, containing one of ${sheetNameForbidden.join(' ')}, or already used. The text stays and nothing is committed.` },
];

// ── the keyboard and screen-reader declarations ──────────────────────────────

/** The list's keyboard. */
export const virtualListKeyboard = [
  { keys: 'Tab', does: 'Focuses the list itself. One tab stop, however many rows there are.' },
  { keys: 'Arrow Up / Arrow Down', does: 'Moves the cursor one row. Clamped at both ends rather than wrapping.' },
  { keys: 'Home / End', does: 'First and last row.' },
  { keys: 'Enter or Space', does: 'Chooses the row the cursor is on.' },
  { keys: 'a letter', does: 'Type-ahead: the next row whose label starts with what was typed, wrapping once.' },
];

/** The tree's keyboard — the full ARIA tree model plus the four reorder keys. */
export const treeKeyboard = [
  { keys: 'Tab', does: 'Focuses the tree. One tab stop.' },
  { keys: 'Arrow Up / Arrow Down', does: 'Moves the cursor through the visible rows.' },
  { keys: 'Arrow Right', does: 'Opens a closed branch; on an open one, moves to its first child; on a leaf, nothing. Mirrored under right-to-left.' },
  { keys: 'Arrow Left', does: 'Closes an open branch; on a closed one or a leaf, moves to its parent. Mirrored under right-to-left.' },
  { keys: 'Home / End', does: 'First and last visible row.' },
  { keys: '*', does: 'Opens every sibling of the current row, and nothing deeper.' },
  { keys: 'Enter or Space', does: 'Chooses the heading.' },
  { keys: 'Alt + Arrow Up / Alt + Arrow Down', does: 'Moves the heading among its siblings, carrying its subtree. Announced.' },
  { keys: 'Alt + Arrow Right / Alt + Arrow Left', does: 'Indents or outdents it, carrying its subtree. Mirrored under right-to-left.' },
  { keys: 'a letter', does: 'Type-ahead over the visible rows.' },
];

/** The rail's keyboard. */
export const railKeyboard = [
  { keys: 'Tab', does: 'Focuses the rail. One tab stop, however many slides there are.' },
  { keys: 'Arrow Up / Arrow Down', does: 'Moves the cursor and chooses that slide.' },
  { keys: 'Shift + Arrow Up / Down', does: 'Extends the selection from the anchor.' },
  { keys: 'Ctrl + Space', does: 'Adds or removes the cursor’s slide without disturbing the rest.' },
  { keys: 'Home / End', does: 'First and last slide.' },
  { keys: 'Alt + Arrow Up / Alt + Arrow Down', does: 'Moves the whole selection one place, as a block. Announced.' },
  { keys: 'Enter', does: 'Goes to the slide.' },
  { keys: 'a letter', does: 'Type-ahead over the slide titles.' },
];

/** The tab bar's keyboard. */
export const tabBarKeyboard = [
  { keys: 'Tab', does: 'Focuses the showing tab. One stop in the strip: the tabs hold a roving stop.' },
  { keys: 'Arrow Left / Arrow Right', does: 'Moves to the next or previous sheet and shows it. Mirrored under right-to-left.' },
  { keys: 'Home / End', does: 'First and last sheet.' },
  { keys: 'F2', does: 'Renames the sheet in place.' },
  { keys: 'Enter', does: 'While renaming: commits, unless the name is one Excel would refuse.' },
  { keys: 'Escape', does: 'While renaming: cancels, and the old name is kept.' },
  { keys: 'Alt + Arrow Left / Alt + Arrow Right', does: 'Moves the sheet one place. Announced.' },
];

/** What each announces. */
export const virtualListScreenReader =
  'Announces the list’s name, then the row: its label, whether it is selected, and its position — ' +
  '“3 of 5,000”. The position comes from aria-posinset and aria-setsize on the row itself, which is ' +
  'a virtualised list’s only honest source for it: the number of rows in the DOM is not the number ' +
  'of items.';

export const treeScreenReader =
  'Announces “tree”, then the row: its label, its level, its position among its own siblings, and ' +
  'whether it is expanded — and a leaf is announced without an expanded state at all. A reorder is ' +
  'announced through a polite live region, including how many rows moved with it and including a ' +
  'refusal such as “Already at the top level”.';

export const railScreenReader =
  'Announces “listbox, multi-selectable”, then the slide: “Slide 4 of 5,000, Agenda, Opening, ' +
  'hidden”. A slide still waiting for its plate is announced identically — the picture is decoration ' +
  'and aria-busy is on the frame rather than on the option, so a rail that is still rendering is not ' +
  'a rail that announces nothing.';

export const tabBarScreenReader =
  'Announces “tab list”, then the tab: its name, whether it is selected, and “hidden” where the ' +
  'sheet is. The four scroll affordances are announced as buttons with their own names — “Scroll to ' +
  'the first sheet” — because they are not tabs and a tab list may not own them.';
