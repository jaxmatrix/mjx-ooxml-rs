/**
 * The four navigators' vocabulary — **three ARIA patterns, one virtualiser, and every reorder
 * written down as a function.**
 *
 * MJXOFF-191 is *how a user moves through a document that does not fit on a screen*: Word's
 * navigation pane, PowerPoint's slide sorter, Excel's sheet tabs, and the windowed list all three
 * are built on. They share one problem — **they must stay responsive over collections large enough
 * that rendering them whole is not an option** — and they differ in the one thing that must not be
 * shared, which is what they *are* to an assistive technology.
 *
 * ## Three patterns, and using the wrong one is worse than using none
 *
 * The ticket says so and it is not a style note. A `tree` announces *level 2, 3 of 7, expanded*; a
 * `listbox` announces *selected, 12 of 4000*; a `tablist` announces *tab, 3 of 5, selected*. A slide
 * sorter built out of `role="tree"` would announce a depth nobody can act on and would take
 * `ArrowRight` to mean *expand* in a control that has nothing to expand. [`navigatorAriaPatterns`]
 * is the table, and both tiers read it rather than the component's own markup: the browser gate
 * asserts each container's computed role **against this table**, so a component that quietly grew a
 * different role fails with the two role names in the message.
 *
 * ## One virtualiser
 *
 * `src/foundations/virtual-list.ts`, which is U06's `galleryWindow` lifted down a level and given a
 * binary search so the rows need not all be one row tall. See that file for what moved and why.
 * What lives here is the *plan* each navigator's rows take — and the rail's plan is deliberately
 * U06's `GalleryRow` shape, following U08's precedent: a plan whose *shape* is shared delegates all
 * the arithmetic, so only the planning is new.
 *
 * ## Every reorder is a pure function
 *
 * Drag-to-reorder must have a keyboard equivalent — *"reordering slides by keyboard is not
 * optional"* — and the way that goes wrong is two implementations: a pointer path that splices an
 * array and a keyboard path that swaps two entries, agreeing on the easy cases and disagreeing at
 * the ends. So the pointer and the keyboard call **the same** function here, and the interesting
 * cases (moving a tree heading moves its subtree; a multi-selection moves as a block; nothing moves
 * off either end) are asserted in Node where every ordering can be swept.
 *
 * ## Node-importable
 *
 * Data and pure functions. No DOM, no CSS. The sheets live beside their components.
 */

import type { GalleryRow } from '../gallery/gallery-model.ts';
import { typeRoleClass } from '../foundations/typography.ts';
import { phoneShellAtOrBelow } from '../harness/presets.ts';
import type { Direction } from '../overlay/floating.ts';

/**
 * The type roles the four navigators use, named once.
 *
 * ⚠ **A role class goes on the box the state table paints, never on the label inside it.** U05's
 * finding, and it costs a selected row its bold every time it is forgotten: every type role
 * declares a `font-weight`, so a role class on an inner label beats the weight the row inherits
 * from its state. The one exception below is `caption`, which is a *second* line inside a rail row
 * and is deliberately quieter than the row it sits in.
 */
export const navigatorTypeRoles = {
  row: typeRoleClass('control'),
  caption: typeRoleClass('dense'),
  section: typeRoleClass('label'),
  tab: typeRoleClass('control'),
} as const;

/** The four tags this child registers. */
export const navigatorTags = {
  virtualList: 'mjx-virtual-list',
  tree: 'mjx-tree',
  thumbnailRail: 'mjx-thumbnail-rail',
  sheetTabBar: 'mjx-sheet-tab-bar',
} as const;

/** One of the four. */
export type NavigatorName = keyof typeof navigatorTags;

/**
 * The story titles, so the browser tier looks a story up by name rather than by id.
 *
 * ⚠ Duplicated as string literals in the story files themselves, and that is forced: **CSF is
 * indexed statically** and refuses a computed `title`. The gate is what keeps the two honest — it
 * fails with the title in the message when a story cannot be found.
 */
export const navigatorStoryTitles = {
  virtualList: 'Navigators/Virtual List',
  tree: 'Navigators/Tree',
  thumbnailRail: 'Navigators/Thumbnail Rail',
  sheetTabBar: 'Navigators/Sheet Tab Bar',
} as const;

/** The four, in the order the catalogue presents them. */
export const navigatorOrder: readonly NavigatorName[] = [
  'virtualList',
  'tree',
  'thumbnailRail',
  'sheetTabBar',
];

/** What a navigator is, to an assistive technology. */
export interface AriaPattern {
  /** The role on the scrolling container. */
  readonly container: string;
  /** The role on each row. */
  readonly item: string;
  /** Whether more than one row may be chosen at once. */
  readonly multiSelectable: boolean;
  /** Why this pattern and not one of the other two. */
  readonly why: string;
}

/**
 * **Three different patterns**, and the reason each one is the one.
 *
 * Read by `tests/navigators.test.ts` (which asserts the three are genuinely three) and by
 * `tests/browser/navigators.spec.ts` (which asks the browser for each container's computed role and
 * compares it against this). A gate that only read the component's own attribute would be asking
 * the implementation to grade its own homework — U04's rule, applied to ARIA.
 */
export const navigatorAriaPatterns: Readonly<Record<NavigatorName, AriaPattern>> = {
  virtualList: {
    container: 'listbox',
    item: 'option',
    multiSelectable: false,
    why:
      'A set of values, one of which is chosen. Not a menu: a menu is a set of commands, and its ' +
      'items take focus. U07 states the distinction and this is the same control it states it for.',
  },
  tree: {
    container: 'tree',
    item: 'treeitem',
    multiSelectable: false,
    why:
      'A hierarchy whose depth is meaningful and whose branches open and close. The level, the ' +
      'position within its own branch and the expanded state are all things a reader needs said, ' +
      'and `listbox` has nowhere to say any of them.',
  },
  thumbnailRail: {
    container: 'listbox',
    item: 'option',
    multiSelectable: true,
    why:
      'A flat set of slides, several of which can be acted on at once. Not a tree: sections group ' +
      'slides but a slide has no children, and offering `ArrowRight` as *expand* over a control ' +
      'with nothing to expand is worse than offering nothing.',
  },
  sheetTabBar: {
    container: 'tablist',
    item: 'tab',
    multiSelectable: false,
    why:
      'Exactly one sheet is showing and choosing a tab changes what the workspace displays. That ' +
      'is what `tablist` means, and it is the one pattern here whose items are not in a list at all.',
  },
};

/**
 * The custom property a navigator publishes its presentation through.
 *
 * The catalogue's established mechanism — a ribbon group, a status bar and a task pane all do this —
 * and it is the answer to *who decides the layout*. **CSS decides**, the component reads its own
 * decision back, and a gate compares that against [`navigatorPresentationAt`] rather than against
 * the stylesheet the rule was generated from.
 */
export const navigatorPresentationProperty = '--mjx-navigator-presentation';

/** The two presentations a navigator has. */
export const navigatorPresentationNames = ['pane', 'sheet'] as const;

/** One of the two. */
export type NavigatorPresentation = (typeof navigatorPresentationNames)[number];

/**
 * Which presentation a navigator is in, at a container of this width.
 *
 * The model half of the same question the CSS answers, so the two can be compared. A pane on a
 * desktop; the whole frame on a phone, where a navigator IS the screen.
 *
 * ⚠ **The tree and the rail only.** `<mjx-virtual-list>` publishes no presentation at all and
 * `<mjx-sheet-tab-bar>` publishes none either — the tab bar *stays* a tab bar on a phone and grows
 * its targets through density instead. A property published by a component that has one
 * presentation would read as a decision nobody took.
 */
export function navigatorPresentationAt(containerWidth: number): NavigatorPresentation {
  return containerWidth <= navigatorPhoneSheetAtOrBelow ? 'sheet' : 'pane';
}

/**
 * Where the tree and the rail become full-height sheets.
 *
 * ⚠ **An alias, never a second number.** U05 left the instruction in `menu-model.ts` — *"when a
 * third surface needs it, hoist it — do not add a second definition"* — U06 hoisted it into
 * `src/harness/presets.ts`, and this is the fifth and sixth surface to read it.
 * `tests/navigators.test.ts` asserts the identity.
 */
export const navigatorPhoneSheetAtOrBelow = phoneShellAtOrBelow;

/**
 * How far a pointer must travel before a press becomes a drag rather than a click.
 *
 * Shared, because all three draggable navigators must agree: a rail that started dragging at two
 * pixels and a tree that waited for six would feel like two different controls, and the one that
 * started early would swallow clicks.
 */
export const dragThreshold = 4;

/**
 * The glyph a hidden slide and a hidden sheet both wear.
 *
 * ⚠ **A mark and an edge, and never a dimming.** The obvious spelling of *hidden* is an opacity,
 * and the accessibility sweep measured what that costs on a sheet tab: **2.79 : 1 against a floor
 * of 4.5**. A *native disabled* control is exempt from axe's contrast rule because the platform
 * dims it and nobody is expected to read it; a hidden sheet or slide is not disabled at all — it is
 * reachable, selectable and showable — so nothing exempts it and nothing should. The state is
 * carried by this glyph, by a dashed edge, and by the word *hidden* in the accessible name, all
 * three at full contrast. U09's rule about tones, arriving through a different door.
 */
export const hiddenMark = '⊘';

// ── the tree ─────────────────────────────────────────────────────────────────

/** One heading in an outline. Children are the headings beneath it. */
export interface TreeNode {
  readonly id: string;
  readonly label: string;
  readonly children?: readonly TreeNode[];
}

/** One *visible* row of a tree: a node, plus everything ARIA needs said about it. */
export interface TreeRow {
  readonly id: string;
  readonly label: string;
  /** ARIA's `aria-level`, counted from **one**. */
  readonly level: number;
  /** ARIA's `aria-posinset`, counted from one, among its own siblings. */
  readonly posInSet: number;
  /** ARIA's `aria-setsize`: how many siblings it has, itself included. */
  readonly setSize: number;
  /** Whether it has children at all. A leaf carries no `aria-expanded`. */
  readonly expandable: boolean;
  /** Whether its children are showing. Meaningless, and `false`, for a leaf. */
  readonly expanded: boolean;
  /** Its parent's id, or `undefined` at the top level. */
  readonly parentId: string | undefined;
}

/**
 * The rows a tree shows, given which nodes are open.
 *
 * ⚠ **`aria-setsize` and `aria-posinset` are over the node's own siblings, not over the flattened
 * list.** They are the answer to *"3 of 7"* within a branch, and a virtualised tree has to carry
 * them explicitly anyway, because the number of rows in the DOM is not the number of nodes — U07's
 * finding, which is the same one in a different container.
 */
export function flattenTree(
  nodes: readonly TreeNode[],
  expanded: ReadonlySet<string>,
): TreeRow[] {
  const rows: TreeRow[] = [];
  const walk = (
    siblings: readonly TreeNode[],
    level: number,
    parentId: string | undefined,
  ): void => {
    for (const [position, node] of siblings.entries()) {
      const children = node.children ?? [];
      const isExpanded = children.length > 0 && expanded.has(node.id);
      rows.push({
        id: node.id,
        label: node.label,
        level,
        posInSet: position + 1,
        setSize: siblings.length,
        expandable: children.length > 0,
        expanded: isExpanded,
        parentId,
      });
      if (isExpanded) walk(children, level + 1, node.id);
    }
  };
  walk(nodes, 1, undefined);
  return rows;
}

/** Every id in a node's subtree, the node itself first. The thing a reorder must carry with it. */
export function subtreeIds(nodes: readonly TreeNode[], id: string): string[] {
  const found = findNode(nodes, id);
  if (found === undefined) return [];
  const collected: string[] = [];
  const walk = (node: TreeNode): void => {
    collected.push(node.id);
    for (const child of node.children ?? []) walk(child);
  };
  walk(found);
  return collected;
}

/** One node, anywhere in the tree. */
export function findNode(nodes: readonly TreeNode[], id: string): TreeNode | undefined {
  for (const node of nodes) {
    if (node.id === id) return node;
    const inside = findNode(node.children ?? [], id);
    if (inside !== undefined) return inside;
  }
  return undefined;
}

/** How many nodes there are altogether, however deep. */
export function countNodes(nodes: readonly TreeNode[]): number {
  let total = 0;
  for (const node of nodes) total += 1 + countNodes(node.children ?? []);
  return total;
}

/** The siblings a node belongs to, and where in them it sits. */
function locate(
  nodes: readonly TreeNode[],
  id: string,
): { siblings: readonly TreeNode[]; index: number; parentId: string | undefined } | undefined {
  const at = nodes.findIndex((node) => node.id === id);
  if (at >= 0) return { siblings: nodes, index: at, parentId: undefined };
  for (const node of nodes) {
    const inside = locate(node.children ?? [], id);
    if (inside === undefined) continue;
    return { ...inside, parentId: inside.parentId ?? node.id };
  }
  return undefined;
}

/** Rebuild a tree with one branch's children replaced. `undefined` names the top level. */
function withChildren(
  nodes: readonly TreeNode[],
  parentId: string | undefined,
  next: readonly TreeNode[],
): TreeNode[] {
  if (parentId === undefined) return [...next];
  return nodes.map((node) => {
    if (node.id === parentId) return { ...node, children: [...next] };
    const children = node.children;
    if (children === undefined) return node;
    return { ...node, children: withChildren(children, parentId, next) };
  });
}

/**
 * Move a node up or down among its own siblings, **carrying its subtree.**
 *
 * ⚠ **This is the assertion the ticket names.** *"Moving a tree heading moves its subtree"* is the
 * behaviour a drag implementation gets right by accident (it drags a DOM node) and a keyboard
 * implementation gets wrong by accident (it swaps two entries in a flattened list, and the children
 * stay behind under whatever heading is now above them). Both paths call this, so there is one
 * answer and it is the one that is swept.
 *
 * A node at the end of its branch does not move. It deliberately does **not** hop into the next
 * branch: a keyboard reorder that silently changed a heading's parent would be an outline edit
 * nobody asked for, and `outdentNode`/`indentNode` are how depth is changed on purpose.
 */
export function moveAmongSiblings(
  nodes: readonly TreeNode[],
  id: string,
  delta: number,
): TreeNode[] {
  const at = locate(nodes, id);
  if (at === undefined) return [...nodes];
  const target = at.index + Math.sign(delta);
  if (target < 0 || target >= at.siblings.length) return [...nodes];
  const next = [...at.siblings];
  const [moving] = next.splice(at.index, 1);
  if (moving === undefined) return [...nodes];
  next.splice(target, 0, moving);
  return withChildren(nodes, at.parentId, next);
}

/**
 * Make a node the last child of the sibling above it. Word's `Tab` in the outline view.
 *
 * The first node of a branch cannot be indented — there is nothing above it to become a child of —
 * and that is a refusal rather than a no-op with a different shape: the caller announces it.
 */
export function indentNode(nodes: readonly TreeNode[], id: string): TreeNode[] {
  const at = locate(nodes, id);
  if (at === undefined || at.index === 0) return [...nodes];
  const siblings = [...at.siblings];
  const [moving] = siblings.splice(at.index, 1);
  const newParent = siblings[at.index - 1];
  if (moving === undefined || newParent === undefined) return [...nodes];
  siblings[at.index - 1] = {
    ...newParent,
    children: [...(newParent.children ?? []), moving],
  };
  return withChildren(nodes, at.parentId, siblings);
}

/**
 * Make a node the next sibling of its own parent. Word's `Shift + Tab`.
 *
 * ⚠ **The node's own children come with it, and its *following* siblings do not.** Word promotes
 * the trailing siblings into the promoted heading; this does not, and the divergence is marked
 * `GUESS:` at the story rather than guessed at silently — a promotion that reparented text a person
 * had not selected is the kind of edit that is noticed three saves later.
 */
export function outdentNode(nodes: readonly TreeNode[], id: string): TreeNode[] {
  const at = locate(nodes, id);
  if (at === undefined || at.parentId === undefined) return [...nodes];
  const parentAt = locate(nodes, at.parentId);
  if (parentAt === undefined) return [...nodes];
  const siblings = [...at.siblings];
  const [moving] = siblings.splice(at.index, 1);
  if (moving === undefined) return [...nodes];
  const withoutIt = withChildren(nodes, at.parentId, siblings);
  const uncles = locate(withoutIt, at.parentId);
  if (uncles === undefined) return [...nodes];
  const next = [...uncles.siblings];
  next.splice(uncles.index + 1, 0, moving);
  return withChildren(withoutIt, uncles.parentId, next);
}

/** What a key means in a tree. */
export const treeActionNames = [
  'next',
  'previous',
  'first',
  'last',
  'expand',
  'collapse',
  'activate',
  'expandAll',
  'moveUp',
  'moveDown',
  'indent',
  'outdent',
] as const;

/** One of them. */
export type TreeAction = (typeof treeActionNames)[number];

/** What the keyboard handler is told about a press. */
export interface KeyModifiers {
  readonly altKey: boolean;
  readonly ctrlKey: boolean;
  readonly shiftKey: boolean;
  readonly metaKey: boolean;
}

/** Nothing held down. */
export const noModifiers: KeyModifiers = {
  altKey: false,
  ctrlKey: false,
  shiftKey: false,
  metaKey: false,
};

/**
 * The full ARIA tree key map, plus the four reorder keys.
 *
 * ⚠ **`Alt` is what separates *navigate* from *reorder*, and the arrows mirror under RTL.** The
 * horizontal pair means *expand* and *collapse* in a left-to-right tree and the opposite in a
 * right-to-left one, exactly as `foundations/splitter.ts` mirrors its growing arrow — a reader of
 * Arabic pressing the arrow that points at the branch expects the branch to open.
 *
 * `undefined` means *not ours*, and it matters: a key this returns nothing for is left to the
 * platform, which is what keeps `Tab` meaning `Tab`.
 */
export function treeKeyAction(
  key: string,
  modifiers: KeyModifiers,
  direction: Direction = 'ltr',
): TreeAction | undefined {
  if (modifiers.ctrlKey || modifiers.metaKey) return undefined;
  const towardsChildren = direction === 'rtl' ? 'ArrowLeft' : 'ArrowRight';
  const towardsParent = direction === 'rtl' ? 'ArrowRight' : 'ArrowLeft';
  if (modifiers.altKey) {
    if (key === 'ArrowUp') return 'moveUp';
    if (key === 'ArrowDown') return 'moveDown';
    if (key === towardsChildren) return 'indent';
    if (key === towardsParent) return 'outdent';
    return undefined;
  }
  if (modifiers.shiftKey && key !== 'Tab') return undefined;
  switch (key) {
    case 'ArrowDown':
      return 'next';
    case 'ArrowUp':
      return 'previous';
    case 'Home':
      return 'first';
    case 'End':
      return 'last';
    case towardsChildren:
      return 'expand';
    case towardsParent:
      return 'collapse';
    case 'Enter':
    case ' ':
      return 'activate';
    case '*':
      return 'expandAll';
    default:
      return undefined;
  }
}

/** Which of the four actions changes the outline rather than moving through it. */
export const treeReorderActions: readonly TreeAction[] = [
  'moveUp',
  'moveDown',
  'indent',
  'outdent',
];

/** Whether an action edits the outline. */
export function isTreeReorder(action: TreeAction): boolean {
  return treeReorderActions.includes(action);
}

/**
 * Where a movement key lands, over the *visible* rows.
 *
 * Clamped rather than wrapping, which is U06's `nextGalleryIndex` rule for the same reason: a list
 * that wraps takes a reader who is holding `ArrowDown` from the end back to the beginning, and they
 * do not notice until they have read the first three rows twice.
 */
export function nextTreeIndex(action: TreeAction, current: number, rowCount: number): number {
  if (rowCount <= 0) return -1;
  const at = Math.min(Math.max(current, 0), rowCount - 1);
  switch (action) {
    case 'next':
      return Math.min(rowCount - 1, at + 1);
    case 'previous':
      return Math.max(0, at - 1);
    case 'first':
      return 0;
    case 'last':
      return rowCount - 1;
    default:
      return at;
  }
}

/**
 * What `ArrowRight` does, which depends on the row it is pressed on.
 *
 * The ARIA tree pattern gives one key three meanings and they are all needed: on a closed branch it
 * opens it, on an open branch it moves to the first child, and on a leaf it does nothing at all.
 * `collapse` is the mirror, and its third meaning — *go to my parent* — is the one people miss.
 */
export function treeExpandOutcome(
  row: TreeRow | undefined,
  action: 'expand' | 'collapse',
): 'open' | 'close' | 'toFirstChild' | 'toParent' | 'nothing' {
  if (row === undefined) return 'nothing';
  if (action === 'expand') {
    if (!row.expandable) return 'nothing';
    return row.expanded ? 'toFirstChild' : 'open';
  }
  if (row.expandable && row.expanded) return 'close';
  return row.parentId === undefined ? 'nothing' : 'toParent';
}

// ── type-ahead, which all four share ─────────────────────────────────────────

/** How long a type-ahead buffer survives with nothing typed into it. */
export const typeAheadWindowMultiple = 6;

/**
 * The row a type-ahead buffer names, searched from **after** the cursor and wrapping once.
 *
 * Wrapping is right here and wrong for the arrow keys, and the difference is the reader's intent: an
 * arrow key means *one further in this direction*, and a typed letter means *the next thing called
 * that*, of which there may be none below and several above.
 *
 * A repeated single letter is the platform's own behaviour — press `s` four times and land on the
 * fourth thing beginning with `s` — and it falls out of searching from after the cursor rather than
 * needing a case of its own.
 */
export function typeAheadIndex(
  labels: readonly string[],
  buffer: string,
  from: number,
): number | undefined {
  const wanted = buffer.trim().toLowerCase();
  if (wanted === '' || labels.length === 0) return undefined;
  for (let step = 1; step <= labels.length; step += 1) {
    const at = (from + step + labels.length * labels.length) % labels.length;
    if ((labels[at] ?? '').toLowerCase().startsWith(wanted)) return at;
  }
  return undefined;
}

// ── the thumbnail rail ───────────────────────────────────────────────────────

/** One slide in the sorter. */
export interface RailSlide {
  readonly id: string;
  /** What it is called. Announced, and drawn under the thumbnail. */
  readonly label: string;
  /** Which section it belongs to. Slides with none sit before the first heading. */
  readonly section?: string;
  /** PowerPoint's *hidden slide* — still in the deck, skipped in the show. */
  readonly hidden?: boolean;
  /** Where the plate is, once R10's generator has produced one. */
  readonly thumbnail?: string;
}

/**
 * The rail's row plan, in **U06's own row shape**.
 *
 * ⚠ **This is U08's precedent and not a second planner.** `swatchRowPlan` is the record of when a
 * plan's *shape* genuinely differs from a gallery's: it produces `GalleryRow`s and delegates every
 * piece of arithmetic, so only the planning is new. A rail is one column, so each slide is its own
 * `cells` row of width one, and a section contributes a `heading` row before its slides.
 *
 * The rows are therefore **not all the same height** — a section heading is a line of text and a
 * slide is a thumbnail with a caption under it — which is exactly the precondition U07 had to make
 * true by hand and which `foundations/virtual-list.ts` removes by measuring instead of dividing.
 */
export function railRowPlan(slides: readonly RailSlide[]): GalleryRow[] {
  const rows: GalleryRow[] = [];
  let currentSection: string | undefined;
  let started = false;
  for (const [index, slide] of slides.entries()) {
    const section = slide.section;
    if (!started || section !== currentSection) {
      if (section !== undefined && section !== '') rows.push({ kind: 'heading', category: section });
      currentSection = section;
      started = true;
    }
    rows.push({ kind: 'cells', start: index, end: index + 1 });
  }
  return rows;
}

/** Which plan row holds a slide, so a scroll can be computed without reading the DOM. */
export function railRowOfSlide(rows: readonly GalleryRow[], slideIndex: number): number {
  for (const [row, entry] of rows.entries()) {
    if (entry.kind === 'cells' && slideIndex >= entry.start && slideIndex < entry.end) return row;
  }
  return 0;
}

/** A multi-selection, and the anchor a `Shift` range is measured from. */
export interface RailSelection {
  /** The chosen ids, in the deck's own order. */
  readonly selected: readonly string[];
  /** Where a range extension starts from. */
  readonly anchor: number;
  /** Where the keyboard is. */
  readonly cursor: number;
}

/** Nothing chosen. */
export const emptyRailSelection: RailSelection = { selected: [], anchor: 0, cursor: 0 };

/** How a click or a key asked for a selection to change. */
export interface SelectionIntent {
  /** `Ctrl` — add or remove this one, keep the rest. */
  readonly toggle: boolean;
  /** `Shift` — everything from the anchor to here. */
  readonly extend: boolean;
}

/**
 * The three ways a multi-selection changes, written once so a click and a key cannot disagree.
 *
 * ⚠ **`toggle` is checked before `extend`.** `Ctrl + Shift + click` is a real gesture in a slide
 * sorter and both browsers and Office resolve it as *add the range*; what must not happen is the
 * plain reading, in which the range replaces the existing selection and the `Ctrl` is silently
 * dropped. The order here is the whole of that decision.
 */
export function applyRailSelection(
  slides: readonly RailSlide[],
  current: RailSelection,
  index: number,
  intent: SelectionIntent,
): RailSelection {
  if (slides.length === 0) return emptyRailSelection;
  const at = Math.min(Math.max(index, 0), slides.length - 1);
  const idAt = (position: number): string => slides[position]?.id ?? '';
  const inOrder = (ids: Iterable<string>): string[] => {
    const wanted = new Set(ids);
    return slides.filter((slide) => wanted.has(slide.id)).map((slide) => slide.id);
  };

  if (intent.extend) {
    const from = Math.min(current.anchor, at);
    const to = Math.max(current.anchor, at);
    const range: string[] = [];
    for (let position = from; position <= to; position += 1) range.push(idAt(position));
    const combined = intent.toggle ? [...current.selected, ...range] : range;
    return { selected: inOrder(combined), anchor: current.anchor, cursor: at };
  }

  if (intent.toggle) {
    const id = idAt(at);
    const already = current.selected.includes(id);
    const next = already
      ? current.selected.filter((other) => other !== id)
      : [...current.selected, id];
    return { selected: inOrder(next), anchor: at, cursor: at };
  }

  return { selected: [idAt(at)], anchor: at, cursor: at };
}

/**
 * Move a selection through the deck as a **block**.
 *
 * The keyboard equivalent of dragging, and the one the ticket calls not optional. A block rather
 * than a per-slide swap, because a non-contiguous selection moved one slide at a time collapses into
 * a contiguous one after the first press — the selection quietly changes shape, and nobody who did
 * it on purpose would describe that as *moving my slides down*.
 *
 * Nothing moves off either end, and a selection already against the end is returned unchanged rather
 * than partially moved.
 */
export function moveSelectionBy(
  slides: readonly RailSlide[],
  selectedIds: readonly string[],
  delta: number,
): RailSlide[] {
  const step = Math.sign(delta);
  if (step === 0 || selectedIds.length === 0) return [...slides];
  const chosen = new Set(selectedIds);
  const positions = slides
    .map((slide, index) => (chosen.has(slide.id) ? index : -1))
    .filter((index) => index >= 0);
  if (positions.length === 0 || positions.length === slides.length) return [...slides];
  const first = positions[0] ?? 0;
  const last = positions[positions.length - 1] ?? 0;
  if (step < 0 && first === 0) return [...slides];
  if (step > 0 && last === slides.length - 1) return [...slides];

  const moving = slides.filter((slide) => chosen.has(slide.id));
  const remaining = slides.filter((slide) => !chosen.has(slide.id));
  const target = Math.min(remaining.length, Math.max(0, first + step));
  return [...remaining.slice(0, target), ...moving, ...remaining.slice(target)];
}

/** How a slide is announced, so the two things a picture carries are also said. */
export function railSlideName(slide: RailSlide, position: number, total: number): string {
  const parts = [`Slide ${String(position)} of ${String(total)}`, slide.label];
  if (slide.section !== undefined && slide.section !== '') parts.push(slide.section);
  if (slide.hidden === true) parts.push('hidden');
  return parts.join(', ');
}

/**
 * Whether a rail row is still waiting for its picture.
 *
 * ⚠ **The state the ticket singles out, and the reason it is a state rather than an absence.** R10's
 * plate generator produces thumbnails *asynchronously*, so a rail that blocked on rendering would be
 * unusable — and a rail that drew nothing would be indistinguishable from a rail that had failed.
 * A slide with no `thumbnail` at all is a slide whose plate has not arrived; the row draws a
 * placeholder, carries `aria-busy`, and stays selectable and scrollable throughout.
 */
export function railThumbnailState(slide: RailSlide): 'pending' | 'ready' {
  return slide.thumbnail === undefined || slide.thumbnail === '' ? 'pending' : 'ready';
}

// ── the sheet tab bar ────────────────────────────────────────────────────────

/** One sheet's tab. */
export interface SheetTab {
  readonly id: string;
  readonly label: string;
  /** Excel's per-tab colour, as a CSS colour the shell supplies. Never a token: it is the user's. */
  readonly colour?: string;
  /** A hidden sheet — in the workbook, not in the tab strip's normal reading. */
  readonly hidden?: boolean;
}

/**
 * ⚠ **A sheet's colour is an edge, never a fill.**
 *
 * Excel paints the whole inactive tab in the sheet's colour, and this catalogue cannot: the colour
 * belongs to *the user's workbook* and may be anything at all, so text drawn on it has no contrast
 * anybody has checked. Drawing it as a bar along the tab's edge keeps the user's own choice visible
 * and keeps the label on `--theme-surface`, where its contrast is the palette's and is measured.
 *
 * That is this project's standing rule — defer to the user's document, and never take a decision
 * that can break someone's flow — applied to a colour rather than to a theme. `GUESS:` at the
 * divergence from Office is recorded in the story. `tests/browser/navigators.spec.ts` measures the
 * tab's own background and requires it to be the surface token and **not** the sheet colour.
 */
export const sheetColourIsAnEdgeNotAFill = true;

/** The four scroll affordances a tab strip grows when its tabs overflow. */
export const tabScrollActionNames = ['first', 'previous', 'next', 'last'] as const;

/** One of them. */
export type TabScrollAction = (typeof tabScrollActionNames)[number];

/** Which tab an affordance brings into view. Clamped; the ends are not wrapped. */
export function tabScrollTarget(action: TabScrollAction, current: number, count: number): number {
  if (count <= 0) return -1;
  const at = Math.min(Math.max(current, 0), count - 1);
  switch (action) {
    case 'first':
      return 0;
    case 'last':
      return count - 1;
    case 'previous':
      return Math.max(0, at - 1);
    case 'next':
      return Math.min(count - 1, at + 1);
  }
}

/** How many characters Excel allows in a sheet name. */
export const sheetNameMaximum = 31;

/** The characters Excel refuses in a sheet name. */
export const sheetNameForbidden = ['\\', '/', '?', '*', '[', ']', ':'] as const;

/**
 * Why a proposed sheet name cannot be used, or `undefined` if it can.
 *
 * A refusal has to carry a reason a person can act on, which is U07's rule for the measure input:
 * *what it does with a string it cannot read is the whole component.* The rename field keeps the
 * text, says what is wrong, and commits nothing.
 */
export function sheetNameProblem(
  name: string,
  existing: readonly string[],
): string | undefined {
  const trimmed = name.trim();
  if (trimmed === '') return 'A sheet name cannot be empty.';
  if (trimmed.length > sheetNameMaximum) {
    return `A sheet name is at most ${String(sheetNameMaximum)} characters.`;
  }
  const offender = sheetNameForbidden.find((character) => trimmed.includes(character));
  if (offender !== undefined) {
    return `A sheet name cannot contain ${offender}.`;
  }
  const clash = existing.some((other) => other.toLowerCase() === trimmed.toLowerCase());
  if (clash) return `There is already a sheet called ${trimmed}.`;
  return undefined;
}

/** What a key press does to a rename in progress. */
export type RenameOutcome =
  | { readonly kind: 'commit'; readonly label: string }
  | { readonly kind: 'cancel' }
  | { readonly kind: 'continue' };

/**
 * Enter commits, Escape cancels, everything else keeps typing.
 *
 * ⚠ **Escape is asserted separately from Enter and it is the half that is usually wrong.** A rename
 * field that commits on Escape has destroyed the old name with a keystroke people press to mean
 * *stop* — and the two look identical in a screenshot taken afterwards, because both end with the
 * field closed.
 */
export function renameKeyOutcome(key: string, draft: string): RenameOutcome {
  if (key === 'Escape') return { kind: 'cancel' };
  if (key === 'Enter') return { kind: 'commit', label: draft };
  return { kind: 'continue' };
}

// ── the events every navigator emits ─────────────────────────────────────────

/**
 * The events these four emit. **Binding any of them to a real document is loop 2**; what ships now
 * is the protocol, so a shell can be written against it before a document exists.
 */
export const navigatorEvents = {
  /** The chosen row or rows changed. `detail.ids` is every one of them. */
  selectionChanged: 'mjx-navigator-select',
  /** A branch opened or closed. `detail.id`, `detail.expanded`. */
  expandedChanged: 'mjx-navigator-expand',
  /** The order changed. `detail.order` is every id, in the new order. */
  reordered: 'mjx-navigator-reorder',
  /** A row was activated — double-clicked, or Enter. `detail.id`. */
  activated: 'mjx-navigator-activate',
  /** A sheet was renamed. `detail.id`, `detail.label`. */
  renamed: 'mjx-navigator-rename',
  /** The new-sheet control was pressed. */
  added: 'mjx-navigator-add',
} as const;

/** A reorder the component refused, and why. Announced rather than swallowed. */
export const reorderRefusals = {
  atEnd: 'Already at the end.',
  atStart: 'Already at the start.',
  cannotIndent: 'Nothing above it to move it under.',
  cannotOutdent: 'Already at the top level.',
} as const;
