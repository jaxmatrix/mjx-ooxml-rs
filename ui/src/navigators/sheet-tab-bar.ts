/**
 * `<mjx-sheet-tab-bar>` — Excel's sheet tabs.
 *
 * ```html
 * <mjx-sheet-tab-bar label="Sheets" value="s1"></mjx-sheet-tab-bar>
 * <script>
 *   document.querySelector('mjx-sheet-tab-bar').tabs = [
 *     { id: 's1', label: 'Summary', colour: '#2e9e63' },
 *     { id: 's2', label: 'Q1', hidden: true },
 *   ];
 * </script>
 * ```
 *
 * ## `role="tablist"`, and the four buttons that are deliberately not tabs
 *
 * A tablist's own children must be tabs — a button among them is a real `aria-required-children`
 * violation — so the scroll affordances and the new-sheet control sit **outside** the tablist, in
 * the bar. They are not tabs in any other sense either: pressing one scrolls the strip or adds a
 * sheet, and neither changes which sheet is showing.
 *
 * The tabs themselves hold a **roving tab stop** rather than `aria-activedescendant`, which is the
 * one place this child differs from the other three and the reason is that they are not virtualised:
 * a workbook has tens of sheets, not thousands, they are real `<button>`s in the DOM at all times,
 * and a roving stop over real buttons is the pattern with the fewest moving parts.
 *
 * ## Rename in place, and Escape is the half that is usually wrong
 *
 * `F2` or a double-click turns the tab's label into an `<input>`. `Enter` commits, **`Escape`
 * cancels**, and blurring commits — the platform's own convention for an in-place edit. A rename
 * field that committed on Escape has destroyed the old name with a keystroke people press to mean
 * *stop*, and the two are the same picture afterwards. A name Excel would refuse keeps the text,
 * says what is wrong, and commits nothing — U07's rule for the measure input, applied to a string.
 *
 * ## The colour belongs to the user's workbook
 *
 * See `sheetColourIsAnEdgeNotAFill`. It is drawn as a bar along the tab's edge, never as a fill
 * behind a label, because a colour this catalogue did not choose has a contrast nobody has checked.
 */

import { installControlStyles } from '../controls/control-element.ts';
import { installFoundations } from '../foundations/stylesheet.ts';
import type { Direction } from '../overlay/floating.ts';
import {
  navigatorAriaPatterns,
  navigatorEvents,
  hiddenMark,
  navigatorTags,
  navigatorTypeRoles,
  renameKeyOutcome,
  reorderRefusals,
  sheetNameProblem,
  tabScrollActionNames,
  tabScrollTarget,
  type SheetTab,
  type TabScrollAction,
} from './navigator-model.ts';
import { sheetTabBarCss } from './navigator-sheets.ts';

/** The sheet, composed once. */
export const sheetTabBarSheet = sheetTabBarCss;

/** The ARIA pattern this element implements. */
export const sheetTabBarPattern = navigatorAriaPatterns.sheetTabBar;

/** What each affordance is called, so a gate reads the names rather than guessing them. */
export const tabScrollLabels: Readonly<Record<TabScrollAction, string>> = {
  first: 'Scroll to the first sheet',
  previous: 'Scroll one sheet back',
  next: 'Scroll one sheet forward',
  last: 'Scroll to the last sheet',
};

/** The glyph each affordance draws. Text rather than an icon: four arrows are not four commands. */
const affordanceGlyphs: Readonly<Record<TabScrollAction, string>> = {
  first: '⏮',
  previous: '◀',
  next: '▶',
  last: '⏭',
};

export class MjxSheetTabBar extends HTMLElement {
  static readonly observedAttributes: readonly string[] = ['label', 'value'];

  #root: ShadowRoot | undefined;
  #strip: HTMLElement | undefined;
  #problem: HTMLElement | undefined;
  #live: HTMLElement | undefined;
  #tabs: readonly SheetTab[] = [];
  #cursor = 0;
  #renaming: string | undefined;
  #renameInput: HTMLInputElement | undefined;
  #instance = `mjx-tabs-${String(Math.random()).slice(2, 9)}`;
  #dragFrom: number | undefined;

  connectedCallback(): void {
    if (this.#root === undefined) this.#build();
    installFoundations(this.ownerDocument);
    this.#render();
  }

  attributeChangedCallback(): void {
    if (this.#root === undefined) return;
    this.#render();
  }

  /** What the strip is called. */
  get label(): string {
    return this.getAttribute('label') ?? '';
  }

  /** The sheets. */
  get tabs(): readonly SheetTab[] {
    return this.#tabs;
  }

  set tabs(next: readonly SheetTab[]) {
    this.#tabs = [...next];
    this.#cursor = Math.min(this.#cursor, Math.max(0, this.#tabs.length - 1));
    this.#render();
  }

  /** The showing sheet's id. */
  get value(): string {
    return this.getAttribute('value') ?? this.#tabs[0]?.id ?? '';
  }

  set value(next: string) {
    this.setAttribute('value', next);
  }

  /** Where the roving tab stop is. */
  get cursorIndex(): number {
    return this.#cursor;
  }

  /** Which sheet is being renamed, if any. */
  get renaming(): string | undefined {
    return this.#renaming;
  }

  /** Whether the strip has more tabs than it can show. */
  get overflowing(): boolean {
    const strip = this.#strip;
    if (strip === undefined) return false;
    return strip.scrollWidth - strip.clientWidth > 1;
  }

  /** The refusal currently shown, if the rename field is holding a name Excel would not take. */
  get renameProblem(): string {
    return this.#problem?.textContent ?? '';
  }

  /** The last thing announced. */
  get announcement(): string {
    return this.#live?.textContent ?? '';
  }

  /** Which way the text runs. */
  get direction(): Direction {
    return getComputedStyle(this).direction === 'rtl' ? 'rtl' : 'ltr';
  }

  /** Start an in-place rename. */
  beginRename(id: string): void {
    this.#renaming = id;
    this.#render();
    this.#renameInput?.select();
  }

  /** Stop renaming without committing. */
  cancelRename(): void {
    this.#renaming = undefined;
    this.#render();
    this.#focusTab(this.#cursor);
  }

  /** Choose a sheet. */
  select(index: number): void {
    const tab = this.#tabs[index];
    if (tab === undefined) return;
    this.#cursor = index;
    this.value = tab.id;
    this.#render();
    this.#focusTab(index);
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.selectionChanged, {
        bubbles: true,
        composed: true,
        detail: { ids: [tab.id] },
      }),
    );
  }

  /** Move a sheet one place. The keyboard equivalent of dragging it. */
  moveBy(delta: number): void {
    const from = this.#cursor;
    const to = from + Math.sign(delta);
    if (to < 0) {
      this.#announce(reorderRefusals.atStart);
      return;
    }
    if (to >= this.#tabs.length) {
      this.#announce(reorderRefusals.atEnd);
      return;
    }
    const next = [...this.#tabs];
    const [moving] = next.splice(from, 1);
    if (moving === undefined) return;
    next.splice(to, 0, moving);
    this.#tabs = next;
    this.#cursor = to;
    this.#render();
    this.#focusTab(to);
    this.#announce(`${moving.label} moved to position ${String(to + 1)}.`);
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.reordered, {
        bubbles: true,
        composed: true,
        detail: { order: next.map((tab) => tab.id) },
      }),
    );
  }

  #build(): void {
    const root = this.attachShadow({ mode: 'open' });
    this.#root = root;
    installControlStyles(root, sheetTabBarSheet);

    const bar = document.createElement('div');
    bar.className = 'bar';

    const affordances = document.createElement('div');
    affordances.className = 'affordances';
    for (const action of tabScrollActionNames) {
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'affordance';
      button.dataset['affordance'] = action;
      button.setAttribute('aria-label', tabScrollLabels[action]);
      button.textContent = affordanceGlyphs[action];
      button.addEventListener('click', () => {
        this.#scrollBy(action);
      });
      affordances.append(button);
    }

    const strip = document.createElement('div');
    strip.className = 'strip';
    strip.setAttribute('role', sheetTabBarPattern.container);
    strip.setAttribute('part', 'strip');

    const problem = document.createElement('span');
    problem.className = `problem ${navigatorTypeRoles.caption}`;
    problem.setAttribute('role', 'status');
    problem.hidden = true;

    const trailing = document.createElement('div');
    trailing.className = 'trailing';
    const add = document.createElement('button');
    add.type = 'button';
    add.className = 'add';
    add.setAttribute('aria-label', 'New sheet');
    add.textContent = '+';
    add.addEventListener('click', () => {
      this.dispatchEvent(
        new CustomEvent(navigatorEvents.added, { bubbles: true, composed: true, detail: {} }),
      );
    });
    trailing.append(add);

    const live = document.createElement('div');
    live.className = 'live';
    live.setAttribute('role', 'status');
    live.setAttribute('aria-live', 'polite');

    bar.append(affordances, strip, problem, trailing);
    root.append(bar, live);
    this.#strip = strip;
    this.#problem = problem;
    this.#live = live;

    strip.addEventListener('keydown', this.#onKeyDown);
    strip.addEventListener('dblclick', this.#onDoubleClick);
    strip.addEventListener('pointerdown', this.#onPointerDown);
    strip.addEventListener('pointerup', this.#onPointerUp);
  }

  #tabId(index: number): string {
    return `${this.#instance}-tab-${String(index)}`;
  }

  #render(): void {
    const strip = this.#strip;
    if (strip === undefined) return;
    strip.setAttribute('aria-label', this.label);

    /*
     * ⚠ A tab already on screen is KEPT, never rebuilt. MJXOFF-189's rule, and here it is worse
     * than cosmetic in two ways: rebuilding the strip destroys the button the keyboard is on, and
     * it destroys the rename field mid-edit — which would be indistinguishable from a rename that
     * silently reverted.
     */
    const existing = new Map<string, HTMLElement>();
    for (const child of strip.children) {
      if (child instanceof HTMLElement && child.dataset['id'] !== undefined) {
        existing.set(child.dataset['id'], child);
      }
    }

    const order: HTMLElement[] = [];
    for (const [index, tab] of this.#tabs.entries()) {
      const element = existing.get(tab.id) ?? this.#buildTab(tab);
      this.#paintTab(element, tab, index);
      order.push(element);
    }
    strip.replaceChildren(...order);

    const problem = this.#problem;
    if (problem !== undefined && this.#renaming === undefined) {
      problem.textContent = '';
      problem.hidden = true;
    }
  }

  #buildTab(tab: SheetTab): HTMLElement {
    const element = document.createElement('button');
    element.type = 'button';
    element.className = `tab ${navigatorTypeRoles.tab}`;
    element.setAttribute('role', sheetTabBarPattern.item);
    element.setAttribute('part', 'tab');
    element.dataset['id'] = tab.id;

    const mark = document.createElement('span');
    mark.className = 'mark';
    mark.setAttribute('aria-hidden', 'true');
    mark.textContent = hiddenMark;
    mark.hidden = true;
    element.append(mark);

    const label = document.createElement('span');
    label.className = 'label';
    element.append(label);

    const colour = document.createElement('span');
    colour.className = 'tab-colour';
    colour.setAttribute('aria-hidden', 'true');
    element.append(colour);

    element.addEventListener('click', () => {
      const at = this.#tabs.findIndex((candidate) => candidate.id === tab.id);
      if (at >= 0 && this.#renaming === undefined) this.select(at);
    });
    return element;
  }

  #paintTab(element: HTMLElement, tab: SheetTab, index: number): void {
    element.id = this.#tabId(index);
    const chosen = tab.id === this.value;
    element.setAttribute('aria-selected', String(chosen));
    element.dataset['pressed'] = String(chosen);
    element.dataset['index'] = String(index);
    element.dataset['hidden'] = String(tab.hidden === true);
    const mark = element.querySelector('.mark');
    if (mark instanceof HTMLElement) mark.hidden = tab.hidden !== true;
    element.tabIndex = index === this.#cursor ? 0 : -1;
    /*
     * The hidden state is in the accessible name and not only in the drawing, which is this
     * catalogue's standing rule about a picture: an opacity is invisible to a screen reader and to
     * anyone who cannot tell two greys apart.
     */
    element.setAttribute(
      'aria-label',
      tab.hidden === true ? `${tab.label}, hidden` : tab.label,
    );
    if (tab.colour !== undefined && tab.colour !== '') {
      element.style.setProperty('--mjx-sheet-colour', tab.colour);
    } else {
      element.style.removeProperty('--mjx-sheet-colour');
    }

    const label = element.querySelector('.label');
    if (!(label instanceof HTMLElement)) return;

    if (this.#renaming === tab.id) {
      const existing = label.querySelector('input');
      if (existing instanceof HTMLInputElement) return;
      label.textContent = '';
      const input = document.createElement('input');
      input.className = 'rename';
      input.type = 'text';
      input.value = tab.label;
      input.setAttribute('aria-label', `Rename ${tab.label}`);
      input.addEventListener('keydown', this.#onRenameKey);
      input.addEventListener('blur', this.#onRenameBlur);
      label.append(input);
      this.#renameInput = input;
      queueMicrotask(() => {
        input.focus();
        input.select();
      });
      return;
    }

    if (this.#renameInput?.isConnected !== true) this.#renameInput = undefined;
    if (label.firstElementChild !== null) label.replaceChildren();
    label.textContent = tab.label;
  }

  /**
   * Put the keyboard on a tab and bring it into view.
   *
   * The **one** place a tab is focused, so the reveal above cannot be right in one path and wrong in
   * another — which is precisely how the page came to scroll sideways for the arrow keys while the
   * affordances behaved.
   */
  #focusTab(index: number): void {
    const strip = this.#strip;
    const element = strip?.children[index];
    if (strip === undefined || !(element instanceof HTMLElement)) return;
    element.focus({ preventScroll: true });
    const leading = element.offsetLeft;
    const trailing = leading + element.offsetWidth;
    if (leading < strip.scrollLeft) strip.scrollLeft = leading;
    else if (trailing > strip.scrollLeft + strip.clientWidth) {
      strip.scrollLeft = trailing - strip.clientWidth;
    }
  }

  #announce(message: string): void {
    const live = this.#live;
    if (live === undefined) return;
    live.textContent = message;
  }

  #scrollBy(action: TabScrollAction): void {
    const target = tabScrollTarget(action, this.#cursor, this.#tabs.length);
    if (target < 0) return;
    this.#cursor = target;
    this.#render();
    /*
     * ⚠ **`preventScroll`, and the strip is scrolled by arithmetic rather than by
     * `scrollIntoView`** — see `#focusTab`, which is where that is done and why.
     */
    this.#focusTab(target);
  }

  #commitRename(draft: string): void {
    const id = this.#renaming;
    if (id === undefined) return;
    const others = this.#tabs.filter((tab) => tab.id !== id).map((tab) => tab.label);
    const problem = sheetNameProblem(draft, others);
    const notice = this.#problem;
    if (problem !== undefined) {
      /*
       * ⚠ The text stays, nothing is committed, and the reason is shown. U07's measure input:
       * *what it does with a string it cannot read is the whole component*. A field that reverted
       * silently and a field that committed the old value are the same picture afterwards.
       */
      if (notice !== undefined) {
        notice.textContent = problem;
        notice.hidden = false;
      }
      this.#renameInput?.focus();
      return;
    }
    const label = draft.trim();
    this.#tabs = this.#tabs.map((tab) => (tab.id === id ? { ...tab, label } : tab));
    this.#renaming = undefined;
    if (notice !== undefined) {
      notice.textContent = '';
      notice.hidden = true;
    }
    this.#render();
    this.#focusTab(this.#cursor);
    this.dispatchEvent(
      new CustomEvent(navigatorEvents.renamed, {
        bubbles: true,
        composed: true,
        detail: { id, label },
      }),
    );
  }

  #onRenameKey = (event: KeyboardEvent): void => {
    const input = event.currentTarget;
    if (!(input instanceof HTMLInputElement)) return;
    const outcome = renameKeyOutcome(event.key, input.value);
    if (outcome.kind === 'continue') return;
    event.preventDefault();
    event.stopPropagation();
    if (outcome.kind === 'cancel') {
      this.cancelRename();
      return;
    }
    this.#commitRename(outcome.label);
  };

  #onRenameBlur = (event: FocusEvent): void => {
    const input = event.currentTarget;
    if (!(input instanceof HTMLInputElement)) return;
    if (this.#renaming === undefined) return;
    // A blur onto the refusal notice is not a person leaving the field.
    this.#commitRename(input.value);
  };

  #onDoubleClick = (event: MouseEvent): void => {
    const index = tabIndexIn(event.composedPath());
    if (index === undefined) return;
    const tab = this.#tabs[index];
    if (tab === undefined) return;
    this.#cursor = index;
    this.beginRename(tab.id);
  };

  #onPointerDown = (event: PointerEvent): void => {
    const index = tabIndexIn(event.composedPath());
    if (index === undefined) return;
    this.#dragFrom = index;
  };

  #onPointerUp = (event: PointerEvent): void => {
    const from = this.#dragFrom;
    this.#dragFrom = undefined;
    if (from === undefined) return;
    const to = tabIndexIn(event.composedPath());
    if (to === undefined || to === from) return;
    // The same one-step move the keyboard makes, repeated. One function, two ways in.
    this.#cursor = from;
    for (let step = 0; step < Math.abs(to - from); step += 1) this.moveBy(Math.sign(to - from));
  };

  #onKeyDown = (event: KeyboardEvent): void => {
    if (this.#renaming !== undefined) return;
    const forward = this.direction === 'rtl' ? 'ArrowLeft' : 'ArrowRight';
    const back = this.direction === 'rtl' ? 'ArrowRight' : 'ArrowLeft';

    if (event.altKey && (event.key === forward || event.key === back)) {
      event.preventDefault();
      this.moveBy(event.key === forward ? 1 : -1);
      return;
    }
    if (event.key === 'F2') {
      event.preventDefault();
      const tab = this.#tabs[this.#cursor];
      if (tab !== undefined) this.beginRename(tab.id);
      return;
    }
    switch (event.key) {
      case forward:
        event.preventDefault();
        this.select(tabScrollTarget('next', this.#cursor, this.#tabs.length));
        return;
      case back:
        event.preventDefault();
        this.select(tabScrollTarget('previous', this.#cursor, this.#tabs.length));
        return;
      case 'Home':
        event.preventDefault();
        this.select(0);
        return;
      case 'End':
        event.preventDefault();
        this.select(this.#tabs.length - 1);
        return;
      default:
        return;
    }
  };
}

/** The tab index a pointer landed on, from the event's composed path. */
function tabIndexIn(path: readonly EventTarget[]): number | undefined {
  for (const node of path) {
    if (!(node instanceof HTMLElement)) continue;
    const index = node.dataset['index'];
    if (index !== undefined) return Number(index);
  }
  return undefined;
}

/** Register the element. Idempotent. */
export function defineSheetTabBar(): void {
  if (customElements.get(navigatorTags.sheetTabBar) === undefined) {
    customElements.define(navigatorTags.sheetTabBar, MjxSheetTabBar);
  }
}

declare global {
  interface HTMLElementTagNameMap {
    'mjx-sheet-tab-bar': MjxSheetTabBar;
  }
}
