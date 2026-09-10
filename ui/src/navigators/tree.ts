/**
 * `<mjx-tree>` — Word's navigation pane and outline view.
 *
 * ```html
 * <mjx-tree label="Navigation"></mjx-tree>
 * <script>
 *   document.querySelector('mjx-tree').nodes = [
 *     { id: 'h1', label: 'Introduction', children: [{ id: 'h1a', label: 'Scope' }] },
 *     …
 *   ];
 * </script>
 * ```
 *
 * ## `role="tree"`, and the three keys that follow from saying so
 *
 * A tree is not a list with indentation. `aria-level`, `aria-posinset` **within its own branch** and
 * `aria-expanded` are three facts a `listbox` has nowhere to put, and `ArrowRight` means three
 * different things depending on the row it is pressed on — open this branch, move into it, or
 * nothing at all. [`treeExpandOutcome`] is that decision, written down where both tiers can read it.
 *
 * ## Reorder: one function, two ways of reaching it
 *
 * `Alt + ArrowUp` / `Alt + ArrowDown` move a heading among its siblings; `Alt + ArrowRight` /
 * `Alt + ArrowLeft` change its depth, mirrored under right-to-left. Dragging calls the same
 * functions. **Moving a heading moves its subtree**, which falls out of the model operating on the
 * tree rather than on the flattened rows — a keyboard implementation that swapped two entries in the
 * visible list would leave the children behind under whichever heading ended up above them, and it
 * would look completely correct until somebody collapsed the branch.
 *
 * ⚠ **A refused reorder is announced.** *Already at the top level* is information; silence is a
 * key that appears not to work.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import type { Direction } from '../overlay/floating.ts';
import {
  countNodes,
  dragThreshold,
  flattenTree,
  indentNode,
  isTreeReorder,
  moveAmongSiblings,
  navigatorAriaPatterns,
  navigatorEvents,
  navigatorPresentationProperty,
  navigatorTags,
  navigatorTypeRoles,
  nextTreeIndex,
  outdentNode,
  reorderRefusals,
  subtreeIds,
  treeExpandOutcome,
  treeKeyAction,
  typeAheadIndex,
  type NavigatorPresentation,
  type KeyModifiers,
  type TreeAction,
  type TreeNode,
  type TreeRow,
} from './navigator-model.ts';
import { treeCss } from './navigator-sheets.ts';
import { VirtualScroller, type ScrollerParts } from './virtual-scroller.ts';
import { rowIndexIn } from './virtual-list.ts';

/** The sheet, composed once. */
export const treeSheet = treeCss;

/** The ARIA pattern this element implements. */
export const treePattern = navigatorAriaPatterns.tree;

export class MjxTree extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'value', 'sheet'];

  #root: ShadowRoot | undefined;
  #viewport: HTMLElement | undefined;
  #live: HTMLElement | undefined;
  #scroller: VirtualScroller | undefined;
  #nodes: readonly TreeNode[] = [];
  #expanded = new Set<string>();
  #rows: TreeRow[] = [];
  #cursor = 0;
  #typeAhead = '';
  #typeAheadAt = 0;
  #instance = `mjx-tree-${String(Math.random()).slice(2, 9)}`;
  #dragFrom: { index: number; x: number; y: number } | undefined;
  #dragging = false;
  #dropAt: { index: number; after: boolean } | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.#sync();
  }

  disconnectedCallback(): void {
    this.#scroller?.dispose();
  }

  attributeChangedCallback(): void {
    if (this.#root === undefined) return;
    this.#sync();
  }

  /** What the tree is called. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The outline. A property, for the reason `<mjx-virtual-list>` states. */
  get nodes(): readonly TreeNode[] {
    return this.#nodes;
  }

  set nodes(next: readonly TreeNode[]) {
    this.#nodes = next;
    this.#sync();
  }

  /** The chosen heading's id. */
  get value(): string {
    return this.getAttribute('value') ?? '';
  }

  set value(next: string) {
    this.setAttribute('value', next);
  }

  /** Which branches are open. A copy: the set is the component's. */
  get expanded(): readonly string[] {
    return [...this.#expanded];
  }

  set expanded(next: readonly string[]) {
    this.#expanded = new Set(next);
    this.#sync();
  }

  /** The visible rows, which is what the keyboard moves through. */
  get rows(): readonly TreeRow[] {
    return this.#rows;
  }

  /** How many nodes there are altogether, however deep — the number a node count is compared to. */
  get nodeCount(): number {
    return countNodes(this.#nodes);
  }

  /** Where the keyboard is. */
  get cursorIndex(): number {
    return this.#cursor;
  }

  /** How many rows are actually in the DOM. */
  get builtRowCount(): number {
    return this.#scroller?.builtRowCount ?? 0;
  }

  /** How many the window says there should be. */
  get expectedRowCount(): number {
    return this.#scroller?.expectedRowCount ?? 0;
  }

  /** The window currently built. */
  get builtWindow(): { readonly firstRow: number; readonly lastRow: number } {
    return this.#scroller?.builtWindow ?? { firstRow: 0, lastRow: 0 };
  }

  /** How many rows have been measured rather than guessed. */
  get measuredRowCount(): number {
    return this.#scroller?.measuredRowCount ?? 0;
  }

  /** Where the tree is scrolled to. */
  get offset(): number {
    return this.#scroller?.offset ?? 0;
  }

  /** What the offset would have been had the last change kept it. The positive control. */
  get naiveOffset(): number {
    return this.#scroller?.naiveOffset ?? 0;
  }

  /** The last thing announced, so a gate reads the words rather than guessing them. */
  get announcement(): string {
    return this.#live?.textContent ?? '';
  }

  /**
   * Which presentation CSS has put it in — **read back, never decided here.**
   *
   * The catalogue's mechanism, and the reason it is read rather than computed: a component that
   * decided its own presentation and then reported it would be grading its own homework, and the
   * gate compares this against `navigatorPresentationAt()` instead.
   */
  get presentation(): NavigatorPresentation {
    const declared = getComputedStyle(this).getPropertyValue(navigatorPresentationProperty).trim();
    return declared === 'sheet' ? 'sheet' : 'pane';
  }

  /** Which way the text runs, which is what mirrors the horizontal arrows. */
  get direction(): Direction {
    return getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  /** Put the keyboard on a row. */
  setCursor(index: number): void {
    if (this.#rows.length === 0) return;
    this.#cursor = Math.min(Math.max(index, 0), this.#rows.length - 1);
    this.#scroller?.scrollToRow(this.#cursor);
    this.#render();
  }

  /** Scroll so that a row is on screen. */
  scrollToIndex(index: number): void {
    this.#scroller?.scrollToRow(index);
  }

  /** Open or close a branch. */
  setExpanded(id: string, open: boolean): void {
    const was = this.#expanded.has(id);
    if (was === open) return;
    if (open) this.#expanded.add(id);
    else this.#expanded.delete(id);
    this.#sync();
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.expandedChanged, {
        bubbles: true,
        composed: true,
        detail: { id, expanded: open },
      }),
    );
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, treeSheet);

    const viewport = document.createElement('div');
    viewport.className = 'viewport';
    viewport.setAttribute('role', treePattern.container);
    viewport.tabIndex = 0;
    viewport.setAttribute('part', 'viewport');

    const leading = document.createElement('div');
    leading.className = 'spacer';
    leading.setAttribute('role', 'presentation');
    const trailing = document.createElement('div');
    trailing.className = 'spacer';
    trailing.setAttribute('role', 'presentation');
    viewport.append(leading, trailing);

    const live = document.createElement('div');
    live.className = 'live';
    live.setAttribute('role', 'status');
    live.setAttribute('aria-live', 'polite');

    root.append(viewport, live);
    this.#viewport = viewport;
    this.#live = live;

    const parts: ScrollerParts = { viewport, leading, trailing };
    this.#scroller = new VirtualScroller(parts, {
      keys: () => this.#rows.map((row) => row.id),
      buildRow: (index) => this.#buildRow(index),
      cursorRow: () => this.#cursor,
    });

    viewport.addEventListener('keydown', this.#onKeyDown);
    viewport.addEventListener('pointerdown', this.#onPointerDown);
    viewport.addEventListener('pointermove', this.#onPointerMove);
    viewport.addEventListener('pointerup', this.#onPointerUp);
    viewport.addEventListener('pointercancel', this.#onPointerUp);
  }

  #sync(): void {
    const viewport = this.#viewport;
    if (viewport === undefined) return;
    viewport.setAttribute('aria-label', this.label);
    this.#rows = flattenTree(this.#nodes, this.#expanded);
    this.#cursor = Math.min(this.#cursor, Math.max(0, this.#rows.length - 1));
    this.#scroller?.setKeys(this.#rows.map((row) => row.id));
    this.#render();
  }

  #rowId(index: number): string {
    return `${this.#instance}-row-${String(index)}`;
  }

  #buildRow(index: number): HTMLElement | undefined {
    const row = this.#rows[index];
    if (row === undefined) return undefined;
    const element = document.createElement('div');
    element.className = `row ${navigatorTypeRoles.row}`;
    element.id = this.#rowId(index);
    element.setAttribute('role', treePattern.item);
    element.setAttribute('part', 'row');
    element.setAttribute('aria-level', String(row.level));
    element.setAttribute('aria-posinset', String(row.posInSet));
    element.setAttribute('aria-setsize', String(row.setSize));
    element.setAttribute('aria-selected', String(row.id === this.value));
    // ⚠ A leaf carries no aria-expanded at all. Setting it to false would announce a branch that
    // can be opened, over a heading with nothing under it.
    if (row.expandable) element.setAttribute('aria-expanded', String(row.expanded));
    element.dataset['selected'] = String(row.id === this.value);
    element.dataset['index'] = String(index);
    element.dataset['id'] = row.id;
    if (index === this.#cursor) element.dataset['cursor'] = 'true';
    if (this.#dragging && index === this.#cursor) element.dataset['dragging'] = 'true';
    if (this.#dropAt?.index === index) {
      element.dataset['drop'] = this.#dropAt.after ? 'after' : 'before';
    }
    // The indentation is a padding computed from the level, published per row. The level is
    // one-based for ARIA and zero-based for the arithmetic, which is the only reason for the minus.
    element.style.setProperty('--mjx-tree-level', String(row.level - 1));

    const twisty = document.createElement('span');
    twisty.className = 'twisty';
    twisty.dataset['leaf'] = String(!row.expandable);
    twisty.setAttribute('aria-hidden', 'true');
    twisty.textContent = row.expanded ? '▾' : '▸';
    element.append(twisty);

    const label = document.createElement('span');
    label.className = 'label';
    label.textContent = row.label;
    element.append(label);
    return element;
  }

  #render(): void {
    const viewport = this.#viewport;
    if (viewport === undefined) return;
    this.#scroller?.render();
    viewport.setAttribute('aria-activedescendant', this.#rowId(this.#cursor));
  }

  #announce(message: string): void {
    const live = this.#live;
    if (live === undefined) return;
    live.textContent = message;
  }

  #select(index: number): void {
    const row = this.#rows[index];
    if (row === undefined) return;
    this.value = row.id;
    this.#cursor = index;
    this.#render();
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.selectionChanged, {
        bubbles: true,
        composed: true,
        detail: { ids: [row.id] },
      }),
    );
  }

  /** Apply a reorder, announce what happened, and say so to the shell. */
  #reorder(action: TreeAction): void {
    const row = this.#rows[this.#cursor];
    if (row === undefined) return;
    const before = this.#nodes;
    let next: readonly TreeNode[];
    let refusal: string | undefined;
    switch (action) {
      case 'moveUp':
        next = moveAmongSiblings(before, row.id, -1);
        if (row.posInSet === 1) refusal = reorderRefusals.atStart;
        break;
      case 'moveDown':
        next = moveAmongSiblings(before, row.id, 1);
        if (row.posInSet === row.setSize) refusal = reorderRefusals.atEnd;
        break;
      case 'indent':
        next = indentNode(before, row.id);
        if (row.posInSet === 1) refusal = reorderRefusals.cannotIndent;
        break;
      case 'outdent':
        next = outdentNode(before, row.id);
        if (row.parentId === undefined) refusal = reorderRefusals.cannotOutdent;
        break;
      default:
        return;
    }
    if (refusal !== undefined) {
      this.#announce(refusal);
      return;
    }
    const carried = subtreeIds(before, row.id).length - 1;
    this.#nodes = next;
    // An indent puts a heading under the sibling above it, and a heading nobody can see is a
    // heading that has vanished. Opening its new parent is the honest completion of the gesture.
    if (action === 'indent') {
      const parent = flattenTree(next, this.#expanded);
      const stillVisible = parent.some((candidate) => candidate.id === row.id);
      if (!stillVisible) {
        for (const node of next) collectParents(node, row.id, this.#expanded);
      }
    }
    this.#rows = flattenTree(this.#nodes, this.#expanded);
    const landed = this.#rows.findIndex((candidate) => candidate.id === row.id);
    this.#cursor = landed >= 0 ? landed : this.#cursor;
    this.#scroller?.setKeys(this.#rows.map((candidate) => candidate.id));
    this.#render();
    this.#announce(
      carried === 0
        ? `${row.label} moved.`
        : `${row.label} moved, with ${String(carried)} beneath it.`,
    );
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.reordered, {
        bubbles: true,
        composed: true,
        detail: { id: row.id, order: this.#rows.map((candidate) => candidate.id), carried },
      }),
    );
  }

  #onKeyDown = (event: KeyboardEvent): void => {
    const modifiers: KeyModifiers = {
      altKey: event.altKey,
      ctrlKey: event.ctrlKey,
      shiftKey: event.shiftKey,
      metaKey: event.metaKey,
    };
    const action = treeKeyAction(event.key, modifiers, this.direction);
    if (action === undefined) {
      if (event.key.length === 1 && !event.ctrlKey && !event.metaKey && !event.altKey) {
        this.#onType(event.key);
      }
      return;
    }
    event.preventDefault();

    if (isTreeReorder(action)) {
      this.#reorder(action);
      return;
    }

    const row = this.#rows[this.#cursor];
    switch (action) {
      case 'next':
      case 'previous':
      case 'first':
      case 'last':
        this.setCursor(nextTreeIndex(action, this.#cursor, this.#rows.length));
        return;
      case 'activate':
        this.#select(this.#cursor);
        return;
      case 'expandAll': {
        // The ARIA pattern's asterisk: open every sibling of the current row, and nothing deeper.
        const level = row?.level ?? 1;
        const parentId = row?.parentId;
        for (const candidate of this.#rows) {
          if (candidate.level === level && candidate.parentId === parentId && candidate.expandable) {
            this.#expanded.add(candidate.id);
          }
        }
        this.#sync();
        return;
      }
      case 'expand': {
        const outcome = treeExpandOutcome(row, 'expand');
        if (outcome === 'open' && row !== undefined) this.setExpanded(row.id, true);
        else if (outcome === 'toFirstChild') this.setCursor(this.#cursor + 1);
        return;
      }
      case 'collapse': {
        const outcome = treeExpandOutcome(row, 'collapse');
        if (outcome === 'close' && row !== undefined) this.setExpanded(row.id, false);
        else if (outcome === 'toParent' && row !== undefined) {
          const parent = this.#rows.findIndex((candidate) => candidate.id === row.parentId);
          if (parent >= 0) this.setCursor(parent);
        }
        return;
      }
      default:
        return;
    }
  };

  #onType(character: string): void {
    const now = Date.now();
    this.#typeAhead = now - this.#typeAheadAt > 1000 ? character : this.#typeAhead + character;
    this.#typeAheadAt = now;
    const found = typeAheadIndex(
      this.#rows.map((row) => row.label),
      this.#typeAhead,
      this.#cursor - 1,
    );
    if (found !== undefined) this.setCursor(found);
  }

  #onPointerDown = (event: PointerEvent): void => {
    const index = rowIndexIn(event.composedPath());
    if (index === undefined) return;
    this.#viewport?.focus();
    const twisty = event.composedPath().some(
      (node) => node instanceof HTMLElement && node.classList.contains('twisty'),
    );
    const row = this.#rows[index];
    if (twisty && row !== undefined && row.expandable) {
      this.setExpanded(row.id, !row.expanded);
      return;
    }
    this.#select(index);
    this.#dragFrom = { index, x: event.clientX, y: event.clientY };
    this.#viewport?.setPointerCapture(event.pointerId);
  };

  #onPointerMove = (event: PointerEvent): void => {
    const from = this.#dragFrom;
    if (from === undefined) return;
    const travelled = Math.hypot(event.clientX - from.x, event.clientY - from.y);
    if (!this.#dragging && travelled < dragThreshold) return;
    this.#dragging = true;
    const target = this.#rowUnder(event.clientY);
    this.#dropAt = target;
    this.#render();
  };

  #onPointerUp = (event: PointerEvent): void => {
    const from = this.#dragFrom;
    this.#dragFrom = undefined;
    if (this.#viewport?.hasPointerCapture(event.pointerId) === true) {
      this.#viewport.releasePointerCapture(event.pointerId);
    }
    const target = this.#dropAt;
    const wasDragging = this.#dragging;
    this.#dragging = false;
    this.#dropAt = undefined;
    if (!wasDragging || from === undefined || target === undefined) {
      this.#render();
      return;
    }
    /*
     * ⚠ A drag calls the SAME function the keyboard calls, one step at a time, rather than
     * splicing the array itself. Two implementations of *move a heading* is how a pointer path and
     * a keyboard path come to disagree at the ends of a branch — and the subtree is carried by the
     * model rather than by the fact that a DOM node happens to contain its children.
     */
    const steps = target.index - from.index;
    const direction = Math.sign(steps);
    /*
     * ⚠ The dragged heading is followed **by id**, not by the index it started at. Every step
     * rebuilds the visible rows, so `#rows[from.index]` after the first one is whichever heading
     * has since moved into that position — a two-step drag would then pick up a different node
     * half way and drop it somewhere nobody asked for.
     */
    const dragged = this.#rows[from.index]?.id;
    if (dragged === undefined) return;
    for (let step = 0; step < Math.abs(steps); step += 1) {
      const at = this.#rows.findIndex((candidate) => candidate.id === dragged);
      if (at < 0) break;
      this.#cursor = at;
      this.#reorder(direction > 0 ? 'moveDown' : 'moveUp');
    }
    this.#render();
  };

  /** Which row a pointer is over, and whether it is past that row's midpoint. */
  #rowUnder(clientY: number): { index: number; after: boolean } | undefined {
    for (let index = this.#scroller?.builtWindow.firstRow ?? 0; index < (this.#scroller?.builtWindow.lastRow ?? 0); index += 1) {
      const element = this.#scroller?.elementForRow(index);
      if (element === undefined) continue;
      const box = element.getBoundingClientRect();
      if (clientY >= box.top && clientY <= box.bottom) {
        return { index, after: clientY > box.top + box.height / 2 };
      }
    }
    return undefined;
  }
}

/** Open every ancestor of `id`, so a node that has just been indented is still visible. */
function collectParents(node: TreeNode, id: string, open: Set<string>): boolean {
  for (const child of node.children ?? []) {
    if (child.id === id || collectParents(child, id, open)) {
      open.add(node.id);
      return true;
    }
  }
  return false;
}

/** Register the element. Idempotent. */
export function defineTree(): void {
  if (customElements.get(navigatorTags.tree) === undefined) {
    customElements.define(navigatorTags.tree, MjxTree);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-tree': MjxTree;
  }
}
